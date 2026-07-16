// SPDX-License-Identifier: MPL-2.0

import type {
  CellDto,
  CellVisualDto,
  EntityDto,
  GameSnapshot,
  GameUpdate,
  ItemDto,
  PlayerDto,
  Position,
} from "./protocol";
import type { CellLight, CellVisibility, RenderCell } from "./renderer-backend";

const DEFAULT_LIGHT: CellLight = { color: 0xffffff, intensity: 0 };

export class RenderWorld {
  readonly #width: number;
  readonly #height: number;
  readonly #cells: Array<CellDto | undefined>;
  readonly #visibility: CellVisibility[];
  readonly #lights: CellLight[];
  readonly #entityKinds = new Map<string, string>();
  #playerPosition: Position = { x: 0, y: 0 };

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
    this.#syncEntityKinds(snapshot.player, snapshot.entities, snapshot.items);
    this.#playerPosition = snapshot.player.position;
    this.#visibility.fill("hidden");
    this.#lights.fill(DEFAULT_LIGHT);
    for (const cell of snapshot.cells) this.#storeCell(cell);
    for (const visual of snapshot.visualCells) this.#storeVisual(visual);
    return this.allCells();
  }

  applyUpdate(update: GameUpdate): RenderCell[] {
    this.#syncEntityKinds(update.player, update.entities, update.items);
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

  #syncEntityKinds(player: PlayerDto, entities: EntityDto[], items: ItemDto[]): void {
    this.#entityKinds.clear();
    this.#entityKinds.set(player.id, player.kindId);
    for (const entity of entities) this.#entityKinds.set(entity.id, entity.kindId);
    for (const item of items) this.#entityKinds.set(item.id, item.kindId);
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
          ? { itemKindId: this.#entityKinds.get(cell.itemId) ?? cell.itemId }
          : {}),
        ...(occupantsVisible && cell.actorId
          ? { actorKindId: this.#entityKinds.get(cell.actorId) ?? cell.actorId }
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
