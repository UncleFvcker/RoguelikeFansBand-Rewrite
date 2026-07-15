// SPDX-License-Identifier: MPL-2.0

import type { TilesetWarning } from "./tileset-runtime";

export type CellVisibility = "visible" | "remembered" | "hidden";

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
}

export interface TilesetChangeResult {
  id: string;
  warnings: readonly TilesetWarning[];
}

export interface RendererBackend {
  readonly id: string;
  initialize(options: BackendInitialization): Promise<TilesetChangeResult>;
  applyCells(cells: readonly RenderCell[]): number;
  setCameraOffset(x: number, y: number): void;
  setTileset(tilesetManifestUrl: string): Promise<TilesetChangeResult>;
  setCanvasLabel(label: string): void;
  destroy(): void;
}
