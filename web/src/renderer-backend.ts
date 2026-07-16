// SPDX-License-Identifier: MPL-2.0

import type { CameraTransform } from "./camera";
import type { VisibilityState } from "./protocol";
import type { TilesetWarning } from "./tileset-runtime";

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
}

export interface RendererBackend {
  readonly id: string;
  getDiagnostics(): RendererBackendDiagnostics;
  initialize(options: BackendInitialization): Promise<TilesetChangeResult>;
  applyCells(cells: readonly RenderCell[]): number;
  setCameraTransform(transform: CameraTransform): void;
  setTileset(tilesetManifestUrl: string): Promise<TilesetChangeResult>;
  setCanvasLabel(label: string): void;
  destroy(): void;
}
