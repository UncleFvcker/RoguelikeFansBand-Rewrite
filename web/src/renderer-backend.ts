// SPDX-License-Identifier: MPL-2.0

import type { EditableVisualDto, Position } from "./protocol";
import type { VisualPreferences } from "./visual-preferences";
import type { CameraTransform } from "./camera";
import type { VisibilityState } from "./protocol";
import type { TilesetWarning } from "./tileset-runtime";
import type { PlayerMeleeAttack } from "./player-motion";
import type { ProjectileFlight } from "./projectile-motion";

export type CellVisibility = VisibilityState;

export interface CellLight {
  color: number;
  intensity: number;
}

export interface RenderCell {
  index: number;
  x: number;
  y: number;
  terrainId: string;
  itemKindId?: string;
  actorKindId?: string;
  actorId?: string;
  actorPlayer?: boolean;
  actorGlyph?: string;
  actorUnique?: boolean;
  highlightPet?: boolean;
  visibility: CellVisibility;
  light: CellLight;
}

export interface BackendInitialization {
  host: HTMLElement;
  width: number;
  height: number;
  tilesetManifestUrl: string;
  contentGlyphs: Readonly<Record<string, string>>;
  canvasLabel: string;
  zoom?: CameraTransform["zoom"];
  onPlayerPosition?: (position: Position) => void;
  onCameraImpulse?: (offset: Position) => void;
}

export interface TilesetChangeResult {
  id: string;
  warnings: readonly TilesetWarning[];
}

export interface RendererBackendDiagnostics {
  terrainChunkSize: number;
  terrainChunkCount: number;
  visibleChunkCount: number;
  lastRebuiltTerrainChunks: number;
  totalRebuiltTerrainChunks: number;
  activeDynamicChunkCount: number;
  pooledDynamicChunkCount: number;
  cellViewCount: number;
  dynamicDisplayObjectCount: number;
}

export interface RendererBackend {
  readonly id: string;
  getDiagnostics(): RendererBackendDiagnostics;
  initialize(options: BackendInitialization): Promise<TilesetChangeResult>;
  resize(width: number, height: number): void;
  applyCells(cells: readonly RenderCell[]): number;
  // Includes melee recovery and projectile batches; repeated commands share this display gate.
  readonly playerMoving: boolean;
  whenPlayerSettled(): Promise<void>;
  setPlayerPosition(position: Position, animate: boolean, continuous?: boolean): void;
  playPlayerMelee(attack: PlayerMeleeAttack, target?: RenderCell): void;
  playProjectiles(flights: readonly ProjectileFlight[]): void;
  setMeleeCameraShake(enabled: boolean): void;
  setCameraTransform(transform: CameraTransform): void;
  setTileset(tilesetManifestUrl: string): Promise<TilesetChangeResult>;
  setVisuals(preferences: VisualPreferences, catalog: readonly EditableVisualDto[]): boolean;
  visualBase(id: string): { glyph: string; foreground: string; background?: string };
  setCanvasLabel(label: string): void;
  capturePng(): string;
  destroy(): void;
}
