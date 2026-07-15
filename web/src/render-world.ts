// SPDX-License-Identifier: MPL-2.0

import type {
  CellDto,
  EntityDto,
  GameSnapshot,
  GameUpdate,
  ItemDto,
  PlayerDto,
  Position,
} from "./protocol";
import type { CellVisibility, RenderCell } from "./renderer-backend";

const PRESENTATION_LIGHT_RADIUS = 6;
const PRESENTATION_LIGHT_COLOR = 0xffd7a3;
const AMBIENT_LIGHT = 0.5;

export class RenderWorld {
  readonly #width: number;
  readonly #height: number;
  readonly #cells: Array<CellDto | undefined>;
  readonly #visibility: CellVisibility[];
  readonly #entityKinds = new Map<string, string>();
  #playerPosition: Position = { x: 0, y: 0 };

  constructor(width: number, height: number) {
    this.#width = width;
    this.#height = height;
    this.#cells = new Array(width * height);
    this.#visibility = new Array<CellVisibility>(width * height).fill("visible");
  }

  get playerPosition(): Position {
    return { ...this.#playerPosition };
  }

  applySnapshot(snapshot: GameSnapshot): RenderCell[] {
    this.#syncEntityKinds(snapshot.player, snapshot.entities, snapshot.items);
    this.#playerPosition = snapshot.player.position;
    this.#visibility.fill("visible");
    for (const cell of snapshot.cells) this.#storeCell(cell);
    return this.allCells();
  }

  applyUpdate(update: GameUpdate): RenderCell[] {
    const previousPlayerPosition = this.#playerPosition;
    this.#syncEntityKinds(update.player, update.entities, update.items);
    this.#playerPosition = update.player.position;
    const dirty = new Set<number>();
    for (const cell of update.changedCells) {
      const index = this.#storeCell(cell);
      if (index !== undefined) dirty.add(index);
    }
    if (!samePosition(previousPlayerPosition, this.#playerPosition)) {
      this.#addLightFootprint(dirty, previousPlayerPosition);
      this.#addLightFootprint(dirty, this.#playerPosition);
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

  #composeCell(index: number): RenderCell[] {
    const cell = this.#cells[index];
    if (!cell) return [];
    const x = index % this.#width;
    const y = Math.floor(index / this.#width);
    return [
      {
        index,
        x,
        y,
        terrainId: cell.terrainId,
        ...(cell.itemId
          ? { itemKindId: this.#entityKinds.get(cell.itemId) ?? cell.itemId }
          : {}),
        ...(cell.actorId
          ? { actorKindId: this.#entityKinds.get(cell.actorId) ?? cell.actorId }
          : {}),
        visibility: this.#visibility[index] ?? "visible",
        light: presentationLight(this.#playerPosition, { x, y }),
      },
    ];
  }

  #addLightFootprint(dirty: Set<number>, center: Position): void {
    const minimumX = Math.max(0, center.x - PRESENTATION_LIGHT_RADIUS);
    const maximumX = Math.min(this.#width - 1, center.x + PRESENTATION_LIGHT_RADIUS);
    const minimumY = Math.max(0, center.y - PRESENTATION_LIGHT_RADIUS);
    const maximumY = Math.min(this.#height - 1, center.y + PRESENTATION_LIGHT_RADIUS);
    for (let y = minimumY; y <= maximumY; y += 1) {
      for (let x = minimumX; x <= maximumX; x += 1) {
        if (Math.hypot(x - center.x, y - center.y) <= PRESENTATION_LIGHT_RADIUS) {
          dirty.add(y * this.#width + x);
        }
      }
    }
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

export function presentationLight(player: Position, cell: Position) {
  const distance = Math.hypot(cell.x - player.x, cell.y - player.y);
  const normalized = Math.max(0, 1 - distance / PRESENTATION_LIGHT_RADIUS);
  return {
    color: PRESENTATION_LIGHT_COLOR,
    intensity: AMBIENT_LIGHT + (1 - AMBIENT_LIGHT) * normalized * normalized,
  };
}

function samePosition(left: Position, right: Position): boolean {
  return left.x === right.x && left.y === right.y;
}
