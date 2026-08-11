// SPDX-License-Identifier: MPL-2.0

import type {
  CellDto,
  CellVisualDto,
  EntityDto,
  GoldAppearanceDto,
  GoldPileDto,
  GameSnapshot,
  GameUpdate,
  ItemDto,
  PlayerDto,
  Position,
} from "./protocol";
import type { CellLight, CellVisibility, RenderCell } from "./renderer-backend";

const DEFAULT_LIGHT: CellLight = { color: 0xffffff, intensity: 0 };
const HALLUCINATION_STATUS_ID = "rfb.status.hallucination";
const FUZZY_MONSTER_KIND_ID = "core.actor.fuzzy-monster";

export class RenderWorld {
  readonly #width: number;
  readonly #height: number;
  readonly #cells: Array<CellDto | undefined>;
  readonly #visibility: CellVisibility[];
  readonly #lights: CellLight[];
  readonly #entityKinds = new Map<string, string>();
  readonly #fuzzyEntityGlyphs = new Map<string, string>();
  #actorKindIds: string[] = [];
  #itemKindIds: string[] = [];
  #playerId = "";
  #playerPosition: Position = { x: 0, y: 0 };
  #hallucinating = false;
  #hallucinationPhase = 0;

  constructor(width: number, height: number) {
    this.#width = width;
    this.#height = height;
    this.#cells = new Array(width * height);
    this.#visibility = new Array<CellVisibility>(width * height).fill("hidden");
    this.#lights = new Array<CellLight>(width * height).fill(DEFAULT_LIGHT);
  }

  get playerPosition(): Position {
    return { ...this.#playerPosition };
  }

  get visibilityCounts(): Readonly<Record<CellVisibility, number>> {
    const counts: Record<CellVisibility, number> = { visible: 0, remembered: 0, hidden: 0 };
    for (const visibility of this.#visibility) counts[visibility] += 1;
    return counts;
  }

  applySnapshot(snapshot: GameSnapshot): RenderCell[] {
    this.#syncEntityKinds(snapshot.player, snapshot.entities, snapshot.items, snapshot.goldPiles);
    this.#syncHallucination(snapshot.player, snapshot.worldTick);
    this.#playerPosition = snapshot.player.position;
    this.#visibility.fill("hidden");
    this.#lights.fill(DEFAULT_LIGHT);
    for (const cell of snapshot.cells) this.#storeCell(cell);
    for (const visual of snapshot.visualCells) this.#storeVisual(visual);
    return this.allCells();
  }

  applyUpdate(update: GameUpdate): RenderCell[] {
    const previousHallucinating = this.#hallucinating;
    const previousPhase = this.#hallucinationPhase;
    this.#syncEntityKinds(update.player, update.entities, update.items, update.goldPiles);
    this.#syncHallucination(update.player, update.worldTick);
    this.#playerPosition = update.player.position;
    const dirty = new Set<number>();
    for (const cell of update.changedCells) {
      const index = this.#storeCell(cell);
      if (index !== undefined) dirty.add(index);
    }
    for (const visual of update.changedVisualCells) {
      const index = this.#storeVisual(visual);
      if (index !== undefined) dirty.add(index);
    }
    if (
      previousHallucinating !== this.#hallucinating ||
      (this.#hallucinating && previousPhase !== this.#hallucinationPhase)
    ) {
      return this.allCells();
    }
    return [...dirty]
      .sort((left, right) => left - right)
      .flatMap((index) => this.#composeCell(index));
  }

  applyVisibilityDelta(
    states: readonly { position: Position; visibility: CellVisibility }[],
  ): RenderCell[] {
    const dirty = new Set<number>();
    for (const state of states) {
      const index = this.#index(state.position);
      if (index === undefined || this.#visibility[index] === state.visibility) continue;
      this.#visibility[index] = state.visibility;
      dirty.add(index);
    }
    return [...dirty]
      .sort((left, right) => left - right)
      .flatMap((index) => this.#composeCell(index));
  }

  allCells(): RenderCell[] {
    const cells: RenderCell[] = [];
    for (let index = 0; index < this.#cells.length; index += 1) {
      cells.push(...this.#composeCell(index));
    }
    return cells;
  }

  #syncEntityKinds(
    player: PlayerDto,
    entities: EntityDto[],
    items: ItemDto[],
    goldPiles: GoldPileDto[],
  ): void {
    this.#entityKinds.clear();
    this.#fuzzyEntityGlyphs.clear();
    this.#playerId = player.id;
    this.#entityKinds.set(player.id, player.kindId);
    for (const entity of entities) {
      this.#entityKinds.set(entity.id, entity.kindId);
      if (entity.kindId === FUZZY_MONSTER_KIND_ID) {
        this.#fuzzyEntityGlyphs.set(entity.id, entity.glyph);
      }
    }
    for (const item of items) this.#entityKinds.set(item.id, item.kindId);
    for (const pile of goldPiles) this.#entityKinds.set(pile.id, goldVisualId(pile.appearance));
    this.#actorKindIds = [...new Set([player.kindId, ...entities.map((entity) => entity.kindId)])]
      .sort();
    this.#itemKindIds = [
      ...new Set([
        ...items.map((item) => item.kindId),
        ...goldPiles.map((pile) => goldVisualId(pile.appearance)),
      ]),
    ].sort();
  }

  #syncHallucination(player: PlayerDto, worldTick: number): void {
    this.#hallucinating = player.statuses?.some(
      (status) => status.kindId === HALLUCINATION_STATUS_ID,
    ) ?? false;
    this.#hallucinationPhase = worldTick ?? 0;
  }

  #hallucinatedKind(actualKindId: string, candidates: string[], index: number, salt: number): string {
    if (!this.#hallucinating || candidates.length < 2) return actualKindId;
    const choice = (index * 31 + this.#hallucinationPhase * 17 + salt) % candidates.length;
    return candidates[choice] ?? actualKindId;
  }

  #storeCell(cell: CellDto): number | undefined {
    const index = this.#index(cell.position);
    if (index === undefined) return undefined;
    this.#cells[index] = cell;
    return index;
  }

  #storeVisual(visual: CellVisualDto): number | undefined {
    const index = this.#index(visual.position);
    if (index === undefined) return undefined;
    this.#visibility[index] = visual.visibility;
    this.#lights[index] = {
      color: visual.light.color,
      intensity: Math.max(0, Math.min(1, visual.light.intensity / 100)),
    };
    return index;
  }

  #composeCell(index: number): RenderCell[] {
    const cell = this.#cells[index];
    if (!cell) return [];
    const x = index % this.#width;
    const y = Math.floor(index / this.#width);
    const visibility = this.#visibility[index] ?? "hidden";
    const occupantsVisible = visibility === "visible";
    return [
      {
        index,
        x,
        y,
        terrainId: cell.terrainId,
        ...(occupantsVisible && cell.itemId
          ? {
              itemKindId: this.#hallucinatedKind(
                this.#entityKinds.get(cell.itemId) ?? cell.itemId,
                this.#itemKindIds,
                index,
                7,
              ),
            }
          : {}),
        ...((occupantsVisible || (cell.actorId && this.#fuzzyEntityGlyphs.has(cell.actorId))) && cell.actorId
          ? {
              actorKindId:
                cell.actorId === this.#playerId
                  ? (this.#entityKinds.get(cell.actorId) ?? cell.actorId)
                  : this.#hallucinatedKind(
                      this.#entityKinds.get(cell.actorId) ?? cell.actorId,
                      this.#actorKindIds,
                      index,
                      13,
                    ),
              ...(!this.#hallucinating && this.#fuzzyEntityGlyphs.has(cell.actorId)
                ? { actorGlyph: this.#fuzzyEntityGlyphs.get(cell.actorId) }
                : {}),
            }
          : {}),
        visibility,
        light: this.#lights[index] ?? DEFAULT_LIGHT,
      },
    ];
  }

  #index(position: Position): number | undefined {
    if (
      position.x < 0 ||
      position.y < 0 ||
      position.x >= this.#width ||
      position.y >= this.#height
    ) {
      return undefined;
    }
    return position.y * this.#width + position.x;
  }
}

export function goldVisualId(appearance: GoldAppearanceDto): string {
  return `core.gold.${appearance}`;
}
