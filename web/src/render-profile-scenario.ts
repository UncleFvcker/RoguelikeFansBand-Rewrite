// SPDX-License-Identifier: MPL-2.0

import type { RenderCell } from "./renderer-backend";

export const RENDER_PROFILE_WIDTH = 192;
export const RENDER_PROFILE_HEIGHT = 64;
export const RENDER_PROFILE_CHUNK_SIZES = [8, 16, 32] as const;

const PROFILE_DYNAMIC_UPDATE_CELLS = 256;
const PROFILE_TERRAIN_UPDATE_CELLS = 96;

export interface FrameTimingSummary {
  sampleCount: number;
  medianMs: number;
  p95Ms: number;
  maxMs: number;
}

export function createRendererProfileCells(): RenderCell[] {
  const cells: RenderCell[] = [];
  for (let y = 0; y < RENDER_PROFILE_HEIGHT; y += 1) {
    for (let x = 0; x < RENDER_PROFILE_WIDTH; x += 1) {
      const index = y * RENDER_PROFILE_WIDTH + x;
      const border =
        x === 0 ||
        y === 0 ||
        x === RENDER_PROFILE_WIDTH - 1 ||
        y === RENDER_PROFILE_HEIGHT - 1;
      const patternedWall = !border && (x * 17 + y * 31) % 47 === 0;
      const visibilityRoll = (x * 13 + y * 29) % 10;
      const visibility =
        visibilityRoll < 6 ? "visible" : visibilityRoll < 8 ? "remembered" : "hidden";
      const itemKindId = visibility === "visible" && index % 53 === 0
        ? "profile.item.light"
        : undefined;
      const actorKindId = visibility === "visible" && index % 97 === 0
        ? "profile.actor.mote"
        : undefined;
      cells.push({
        index,
        x,
        y,
        terrainId:
          border || patternedWall ? "profile.terrain.wall" : "profile.terrain.floor",
        ...(itemKindId ? { itemKindId } : {}),
        ...(actorKindId ? { actorKindId } : {}),
        visibility,
        light: {
          color: (x + y) % 3 === 0 ? 0xffb060 : 0x80b8ff,
          intensity: ((x * 19 + y * 23) % 101) / 100,
        },
      });
    }
  }
  return cells;
}

export function createDynamicProfileUpdates(
  source: readonly RenderCell[],
): RenderCell[] {
  return selectDistributedCells(source, PROFILE_DYNAMIC_UPDATE_CELLS, 47).map((cell) => ({
    ...cell,
    ...(cell.itemKindId
      ? { itemKindId: undefined }
      : { itemKindId: "profile.item.light" }),
    light: {
      color: cell.light.color === 0xffb060 ? 0x80b8ff : 0xffb060,
      intensity: 1 - cell.light.intensity,
    },
  }));
}

export function createTerrainProfileUpdates(
  source: readonly RenderCell[],
): RenderCell[] {
  return selectDistributedCells(source, PROFILE_TERRAIN_UPDATE_CELLS, 127).map((cell) => ({
    ...cell,
    terrainId:
      cell.terrainId === "profile.terrain.wall"
        ? "profile.terrain.floor"
        : "profile.terrain.wall",
  }));
}

export function summarizeFrameIntervals(
  intervals: readonly number[],
): FrameTimingSummary {
  if (intervals.length === 0) {
    return { sampleCount: 0, medianMs: 0, p95Ms: 0, maxMs: 0 };
  }
  const sorted = [...intervals].sort((left, right) => left - right);
  return {
    sampleCount: sorted.length,
    medianMs: roundMilliseconds(percentile(sorted, 0.5)),
    p95Ms: roundMilliseconds(percentile(sorted, 0.95)),
    maxMs: roundMilliseconds(sorted.at(-1) ?? 0),
  };
}

function selectDistributedCells(
  source: readonly RenderCell[],
  count: number,
  stride: number,
): RenderCell[] {
  const selected: RenderCell[] = [];
  const seen = new Set<number>();
  let index = 0;
  while (selected.length < Math.min(count, source.length)) {
    const normalized = index % source.length;
    if (!seen.has(normalized)) {
      const cell = source[normalized];
      if (cell) selected.push(cell);
      seen.add(normalized);
    }
    index += stride;
  }
  return selected;
}

function percentile(sorted: readonly number[], fraction: number): number {
  const index = Math.min(sorted.length - 1, Math.ceil(sorted.length * fraction) - 1);
  return sorted[Math.max(0, index)] ?? 0;
}

function roundMilliseconds(value: number): number {
  return Math.round(value * 1000) / 1000;
}
