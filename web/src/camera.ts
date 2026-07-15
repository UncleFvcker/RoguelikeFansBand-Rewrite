// SPDX-License-Identifier: MPL-2.0

import type { Position } from "./protocol";

export const MAP_CELL_SIZE = 28;
export const PLAYER_CENTERED_VIEW_CELLS = 15;

export type CameraMode = "full-map" | "player-centered";

export interface CameraOffsetOptions {
  mode: CameraMode;
  focus: Position;
  worldWidth: number;
  worldHeight: number;
  viewportWidth: number;
  viewportHeight: number;
  cellSize?: number;
}

export interface CameraOffset {
  x: number;
  y: number;
}

export function computeCameraOffset(options: CameraOffsetOptions): CameraOffset {
  if (options.mode === "full-map") return { x: 0, y: 0 };
  const cellSize = options.cellSize ?? MAP_CELL_SIZE;
  return {
    x: axisOffset(
      options.focus.x * cellSize + cellSize / 2,
      options.worldWidth,
      options.viewportWidth,
    ),
    y: axisOffset(
      options.focus.y * cellSize + cellSize / 2,
      options.worldHeight,
      options.viewportHeight,
    ),
  };
}

function axisOffset(focus: number, worldSize: number, viewportSize: number): number {
  if (viewportSize >= worldSize) return Math.round((viewportSize - worldSize) / 2);
  const ideal = viewportSize / 2 - focus;
  return Math.round(Math.max(viewportSize - worldSize, Math.min(0, ideal)));
}
