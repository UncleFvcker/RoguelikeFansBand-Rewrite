// SPDX-License-Identifier: MPL-2.0

import type {
  BodySlotDto,
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
  | { type: "projectile" }
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
  status: GameSnapshot | GameUpdate | undefined;
  inventory: InventoryItemDto[] = [];
  equipment: EquipmentItemDto[] = [];
  bodySlots: BodySlotDto[] = [];
  readonly selectedInventoryIds = new Set<string>();
  dropQuantityItemId: string | undefined;
  targeting: TargetingState | undefined;
  targetingIntent: TargetingIntent | undefined;
  terrainInteractionMode: TerrainInteractionMode | undefined;

  get commandBlocked(): boolean {
    return this.mode !== "playing" || this.playerDead || this.campaignEnded;
  }

  setMapSize(width: number, height: number): void {
    this.mapWidth = width;
    this.mapHeight = height;
  }
}
