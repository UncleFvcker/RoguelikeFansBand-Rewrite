// SPDX-License-Identifier: MPL-2.0

import type { Position } from "./protocol";

export const MAP_CELL_SIZE = 28;
export const PLAYER_CENTERED_VIEW_CELLS = 15;
export const ZOOM_LEVELS = [0.75, 1, 1.25, 1.5, 2] as const;
export const DEFAULT_ZOOM = 1;

export type CameraMode = "full-map" | "player-centered";
export type ZoomLevel = (typeof ZOOM_LEVELS)[number];

export interface CameraOffsetOptions {
  mode: CameraMode;
  focus: Position;
  worldWidth: number;
  worldHeight: number;
  viewportWidth: number;
  viewportHeight: number;
  cellSize?: number;
  zoom?: number;
}

export interface CameraOffset {
  x: number;
  y: number;
}

export interface CameraTransform extends CameraOffset {
  zoom: ZoomLevel;
  viewportWidth: number;
  viewportHeight: number;
  cullingEnabled: boolean;
}

export function computeCameraOffset(options: CameraOffsetOptions): CameraOffset {
  if (options.mode === "full-map") return { x: 0, y: 0 };
  const cellSize = options.cellSize ?? MAP_CELL_SIZE;
  const zoom = options.zoom ?? DEFAULT_ZOOM;
  return {
    x: axisOffset(
      options.focus.x * cellSize * zoom + (cellSize * zoom) / 2,
      options.worldWidth * zoom,
      options.viewportWidth,
    ),
    y: axisOffset(
      options.focus.y * cellSize * zoom + (cellSize * zoom) / 2,
      options.worldHeight * zoom,
      options.viewportHeight,
    ),
  };
}

export function parseZoomLevel(value: string | null): ZoomLevel {
  const parsed = Number(value);
  return isZoomLevel(parsed) ? parsed : DEFAULT_ZOOM;
}

export function isZoomLevel(value: number): value is ZoomLevel {
  return ZOOM_LEVELS.some((level) => level === value);
}

function axisOffset(focus: number, worldSize: number, viewportSize: number): number {
  if (viewportSize >= worldSize) return Math.round((viewportSize - worldSize) / 2);
  const ideal = viewportSize / 2 - focus;
  return Math.round(Math.max(viewportSize - worldSize, Math.min(0, ideal)));
}
