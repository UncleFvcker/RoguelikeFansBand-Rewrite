// SPDX-License-Identifier: MPL-2.0

import { MAP_CELL_SIZE, type CameraTransform } from "./camera.ts";

export const TERRAIN_CHUNK_SIZE = 16;
export const CHUNK_CULL_MARGIN_CELLS = 1;

export interface RenderChunk {
  index: number;
  column: number;
  row: number;
  cellX: number;
  cellY: number;
  cellWidth: number;
  cellHeight: number;
}

export interface RenderChunkLayout {
  chunksAcross: number;
  chunksDown: number;
  chunks: readonly RenderChunk[];
}

export interface TerrainChunkCell {
  index: number;
  x: number;
  y: number;
  terrainId: string;
}

export function createRenderChunkLayout(
  width: number,
  height: number,
  chunkSize: number = TERRAIN_CHUNK_SIZE,
): RenderChunkLayout {
  if (width <= 0 || height <= 0 || chunkSize <= 0) {
    throw new Error("render chunk dimensions must be positive");
  }
  const chunksAcross = Math.ceil(width / chunkSize);
  const chunksDown = Math.ceil(height / chunkSize);
  const chunks: RenderChunk[] = [];
  for (let row = 0; row < chunksDown; row += 1) {
    for (let column = 0; column < chunksAcross; column += 1) {
      const cellX = column * chunkSize;
      const cellY = row * chunkSize;
      chunks.push({
        index: row * chunksAcross + column,
        column,
        row,
        cellX,
        cellY,
        cellWidth: Math.min(chunkSize, width - cellX),
        cellHeight: Math.min(chunkSize, height - cellY),
      });
    }
  }
  return { chunksAcross, chunksDown, chunks };
}

export function chunkIndexForCell(
  x: number,
  y: number,
  chunksAcross: number,
  chunkSize: number = TERRAIN_CHUNK_SIZE,
): number {
  return Math.floor(y / chunkSize) * chunksAcross + Math.floor(x / chunkSize);
}

export function visibleRenderChunkIndexes(
  chunks: readonly RenderChunk[],
  transform: CameraTransform,
  cellSize: number = MAP_CELL_SIZE,
  marginCells: number = CHUNK_CULL_MARGIN_CELLS,
): Set<number> {
  if (!transform.cullingEnabled) return new Set(chunks.map((chunk) => chunk.index));
  const margin = marginCells * cellSize;
  const left = Math.max(0, -transform.x / transform.zoom - margin);
  const top = Math.max(0, -transform.y / transform.zoom - margin);
  const right = (transform.viewportWidth - transform.x) / transform.zoom + margin;
  const bottom = (transform.viewportHeight - transform.y) / transform.zoom + margin;
  const visible = new Set<number>();
  for (const chunk of chunks) {
    const chunkLeft = chunk.cellX * cellSize;
    const chunkTop = chunk.cellY * cellSize;
    const chunkRight = (chunk.cellX + chunk.cellWidth) * cellSize;
    const chunkBottom = (chunk.cellY + chunk.cellHeight) * cellSize;
    if (
      chunkLeft < right &&
      chunkRight > left &&
      chunkTop < bottom &&
      chunkBottom > top
    ) {
      visible.add(chunk.index);
    }
  }
  return visible;
}

export function updateTerrainChunkState(
  terrainIds: Array<string | undefined>,
  cells: readonly TerrainChunkCell[],
  chunksAcross: number,
  chunkCount: number,
  forceAll: boolean,
  chunkSize: number = TERRAIN_CHUNK_SIZE,
): Set<number> {
  const dirty = new Set<number>();
  if (forceAll) {
    for (let index = 0; index < chunkCount; index += 1) dirty.add(index);
  }
  for (const cell of cells) {
    if (terrainIds[cell.index] === cell.terrainId) continue;
    terrainIds[cell.index] = cell.terrainId;
    dirty.add(chunkIndexForCell(cell.x, cell.y, chunksAcross, chunkSize));
  }
  return dirty;
}
