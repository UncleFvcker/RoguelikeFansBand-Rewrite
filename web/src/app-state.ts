// SPDX-License-Identifier: MPL-2.0

import type {
  BodySlotDto,
  CellDto,
  CellVisualDto,
  ContentVisualDto,
  EquipmentItemDto,
  GameSnapshot,
  GameUpdate,
  InventoryItemDto,
} from "./protocol";
import type { TargetingState } from "./targeting";
import type { TerrainInteractionMode } from "./terrain-interaction";

export type ConnectionState = "starting" | "ready" | "error";
export type ApplicationMode = "title" | "starting-session" | "playing";

export type TargetingIntent =
  | { type: "look" }
  | { type: "local-travel" }
  | { type: "projectile" }
  | { type: "mutation-direction" }
  | { type: "ability-direction" }
  | { type: "ability"; abilityId: string }
  | { type: "item"; itemId: string };

export class AppState {
  busy = false;
  playerDead = false;
  campaignEnded = false;
  mode: ApplicationMode = "title";
  connection: ConnectionState = "starting";
  mapWidth = 0;
  mapHeight = 0;
  worldId: string | undefined;
  status: GameSnapshot | GameUpdate | undefined;
  inventory: InventoryItemDto[] = [];
  equipment: EquipmentItemDto[] = [];
  bodySlots: BodySlotDto[] = [];
  readonly selectedInventoryIds = new Set<string>();
  readonly cells = new Map<string, CellDto>();
  readonly cellVisibility = new Map<string, CellVisualDto["visibility"]>();
  readonly contentGlyphs = new Map<string, string>();
  dropQuantityItemId: string | undefined;
  targeting: TargetingState | undefined;
  targetingIntent: TargetingIntent | undefined;
  terrainInteractionMode: TerrainInteractionMode | undefined;

  get commandBlocked(): boolean {
    return (
      this.mode !== "playing" ||
      this.playerDead ||
      this.campaignEnded ||
      (this.status?.player.pendingMutationDirection != null ||
        this.status?.player.pendingAbilityDirection != null)
    );
  }

  get worldMap(): boolean {
    return this.status?.mapScale === "world";
  }

  setMapSize(width: number, height: number): void {
    this.mapWidth = width;
    this.mapHeight = height;
  }

  replaceCells(cells: readonly CellDto[]): void {
    this.cells.clear();
    this.updateCells(cells);
  }

  updateCells(cells: readonly CellDto[]): void {
    for (const cell of cells) {
      this.cells.set(`${cell.position.x},${cell.position.y}`, cell);
    }
  }

  cellAt(position: { readonly x: number; readonly y: number }): CellDto | undefined {
    return this.cells.get(`${position.x},${position.y}`);
  }

  replaceContentVisuals(visuals: readonly ContentVisualDto[]): void {
    this.contentGlyphs.clear();
    for (const visual of visuals) this.contentGlyphs.set(visual.id, visual.glyph);
  }

  replaceVisualCells(cells: readonly CellVisualDto[]): void {
    this.cellVisibility.clear();
    this.updateVisualCells(cells);
  }

  updateVisualCells(cells: readonly CellVisualDto[]): void {
    for (const cell of cells) {
      this.cellVisibility.set(`${cell.position.x},${cell.position.y}`, cell.visibility);
    }
  }
}
