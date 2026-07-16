// SPDX-License-Identifier: MPL-2.0

import { MAP_CELL_SIZE } from "./camera";
import { PixiRendererBackend } from "./pixi-renderer-backend";
import type { RenderCell, RendererBackendDiagnostics } from "./renderer-backend";
import {
  RENDER_PROFILE_CHUNK_SIZES,
  RENDER_PROFILE_HEIGHT,
  RENDER_PROFILE_WIDTH,
  createDynamicProfileUpdates,
  createRendererProfileCells,
  createTerrainProfileUpdates,
  summarizeFrameIntervals,
  type FrameTimingSummary,
} from "./render-profile-scenario";

const PROFILE_VIEWPORT_SIZE = 420;
const PROFILE_FRAME_SAMPLES = 45;
const ASCII_TILESET = "/tilesets/ascii-default/tileset.json";
const IMAGE_TILESET = "/tilesets/image-demo/tileset.json";
const PROFILE_ENABLE_STORAGE_KEY = "rfb.renderer-profile-enabled";
const PROFILE_GLYPHS: Readonly<Record<string, string>> = {
  "profile.terrain.floor": ".",
  "profile.terrain.wall": "#",
  "profile.item.light": "*",
  "profile.actor.mote": "m",
};

export interface RendererProfileRun {
  chunkSize: number;
  initializeMs: number;
  initialSnapshotMs: number;
  cameraSweepMs: number;
  dynamicUpdateMs: number;
  terrainUpdateMs: number;
  tilesetSwitchMs: number;
  canvasPixelWidth: number;
  canvasPixelHeight: number;
  frameTiming: FrameTimingSummary;
  diagnostics: RendererBackendDiagnostics;
}

export interface RendererProfileReport {
  schemaVersion: 1;
  scenarioId: "rfb-render-profile-large-original-v1";
  rendererBackend: "pixi-layered-chunks-v2";
  width: number;
  height: number;
  cellCount: number;
  dynamicUpdateCellCount: number;
  terrainUpdateCellCount: number;
  devicePixelRatio: number;
  userAgent: string;
  runs: RendererProfileRun[];
  recommendation: "visible-chunk-dynamic-views" | "retain-preallocated-dynamic-views";
}

declare global {
  interface Window {
    __rfbRunRendererProfile?: () => Promise<RendererProfileReport>;
    __rfbRendererProfileResult?: RendererProfileReport;
    __rfbRendererProfileError?: string;
  }
}

export function installRendererProfileHook(): void {
  if (localStorage.getItem(PROFILE_ENABLE_STORAGE_KEY) !== "1") return;
  let activeRun: Promise<RendererProfileReport> | undefined;
  window.__rfbRunRendererProfile = () => {
    if (activeRun) return activeRun;
    document.documentElement.dataset.rendererProfileState = "running";
    delete window.__rfbRendererProfileResult;
    delete window.__rfbRendererProfileError;
    activeRun = runRendererProfile()
      .then((report) => {
        window.__rfbRendererProfileResult = report;
        document.documentElement.dataset.rendererProfileState = "complete";
        return report;
      })
      .catch((error: unknown) => {
        const detail = error instanceof Error ? error.message : String(error);
        window.__rfbRendererProfileError = detail;
        document.documentElement.dataset.rendererProfileState = "error";
        throw error;
      })
      .finally(() => {
        activeRun = undefined;
      });
    return activeRun;
  };
}

export async function runRendererProfile(): Promise<RendererProfileReport> {
  const initialCells = createRendererProfileCells();
  const dynamicUpdates = createDynamicProfileUpdates(initialCells);
  const terrainUpdates = createTerrainProfileUpdates(initialCells);
  const runs: RendererProfileRun[] = [];
  for (const chunkSize of RENDER_PROFILE_CHUNK_SIZES) {
    runs.push(
      await profileChunkSize(chunkSize, initialCells, dynamicUpdates, terrainUpdates),
    );
  }
  const dynamicDisplayObjectCount = Math.max(
    ...runs.map((run) => run.diagnostics.dynamicDisplayObjectCount),
  );
  return {
    schemaVersion: 1,
    scenarioId: "rfb-render-profile-large-original-v1",
    rendererBackend: "pixi-layered-chunks-v2",
    width: RENDER_PROFILE_WIDTH,
    height: RENDER_PROFILE_HEIGHT,
    cellCount: initialCells.length,
    dynamicUpdateCellCount: dynamicUpdates.length,
    terrainUpdateCellCount: terrainUpdates.length,
    devicePixelRatio: window.devicePixelRatio,
    userAgent: navigator.userAgent,
    runs,
    recommendation:
      dynamicDisplayObjectCount >= 50_000
        ? "visible-chunk-dynamic-views"
        : "retain-preallocated-dynamic-views",
  };
}

async function profileChunkSize(
  chunkSize: number,
  initialCells: readonly RenderCell[],
  dynamicUpdates: readonly RenderCell[],
  terrainUpdates: readonly RenderCell[],
): Promise<RendererProfileRun> {
  const host = createProfileHost();
  const backend = new PixiRendererBackend({ terrainChunkSize: chunkSize });
  let initialized = false;
  try {
    const initializeStart = performance.now();
    await backend.initialize({
      host,
      width: RENDER_PROFILE_WIDTH,
      height: RENDER_PROFILE_HEIGHT,
      tilesetManifestUrl: ASCII_TILESET,
      contentGlyphs: PROFILE_GLYPHS,
      canvasLabel: `RFB render profile chunk ${chunkSize}`,
    });
    initialized = true;
    const initializeMs = elapsedMilliseconds(initializeStart);

    const snapshotStart = performance.now();
    backend.applyCells(initialCells);
    const initialSnapshotMs = elapsedMilliseconds(snapshotStart);

    backend.setCameraTransform(centeredCameraTransform());
    const cameraStart = performance.now();
    for (let step = 0; step < 32; step += 1) {
      backend.setCameraTransform(sweptCameraTransform(step, 32));
    }
    const cameraSweepMs = elapsedMilliseconds(cameraStart);

    const dynamicStart = performance.now();
    backend.applyCells(dynamicUpdates);
    const dynamicUpdateMs = elapsedMilliseconds(dynamicStart);

    const terrainStart = performance.now();
    backend.applyCells(terrainUpdates);
    const terrainUpdateMs = elapsedMilliseconds(terrainStart);

    const tilesetStart = performance.now();
    await backend.setTileset(IMAGE_TILESET);
    backend.applyCells(initialCells);
    const tilesetSwitchMs = elapsedMilliseconds(tilesetStart);

    backend.setCameraTransform(centeredCameraTransform());
    const frameTiming = summarizeFrameIntervals(
      await collectAnimationFrameIntervals(PROFILE_FRAME_SAMPLES),
    );
    const canvas = host.querySelector("canvas");
    return {
      chunkSize,
      initializeMs,
      initialSnapshotMs,
      cameraSweepMs,
      dynamicUpdateMs,
      terrainUpdateMs,
      tilesetSwitchMs,
      canvasPixelWidth: canvas?.width ?? 0,
      canvasPixelHeight: canvas?.height ?? 0,
      frameTiming,
      diagnostics: backend.getDiagnostics(),
    };
  } finally {
    if (initialized) backend.destroy();
    host.remove();
  }
}

function createProfileHost(): HTMLDivElement {
  const host = document.createElement("div");
  host.dataset.rendererProfileHost = "true";
  Object.assign(host.style, {
    position: "fixed",
    inset: "0 auto auto 0",
    width: `${PROFILE_VIEWPORT_SIZE}px`,
    height: `${PROFILE_VIEWPORT_SIZE}px`,
    overflow: "hidden",
    opacity: "0.01",
    pointerEvents: "none",
    zIndex: "-1",
  });
  document.body.append(host);
  return host;
}

function centeredCameraTransform() {
  const worldWidth = RENDER_PROFILE_WIDTH * MAP_CELL_SIZE;
  const worldHeight = RENDER_PROFILE_HEIGHT * MAP_CELL_SIZE;
  return {
    x: -Math.round((worldWidth - PROFILE_VIEWPORT_SIZE) / 2),
    y: -Math.round((worldHeight - PROFILE_VIEWPORT_SIZE) / 2),
    zoom: 1 as const,
    viewportWidth: PROFILE_VIEWPORT_SIZE,
    viewportHeight: PROFILE_VIEWPORT_SIZE,
    cullingEnabled: true,
  };
}

function sweptCameraTransform(step: number, stepCount: number) {
  const worldWidth = RENDER_PROFILE_WIDTH * MAP_CELL_SIZE;
  const worldHeight = RENDER_PROFILE_HEIGHT * MAP_CELL_SIZE;
  const ratio = stepCount <= 1 ? 0 : step / (stepCount - 1);
  return {
    x: -Math.round((worldWidth - PROFILE_VIEWPORT_SIZE) * ratio),
    y: -Math.round((worldHeight - PROFILE_VIEWPORT_SIZE) * (1 - ratio)),
    zoom: 1 as const,
    viewportWidth: PROFILE_VIEWPORT_SIZE,
    viewportHeight: PROFILE_VIEWPORT_SIZE,
    cullingEnabled: true,
  };
}

function collectAnimationFrameIntervals(sampleCount: number): Promise<number[]> {
  return new Promise((resolve) => {
    const intervals: number[] = [];
    let previous: number | undefined;
    const sample = (timestamp: number) => {
      if (previous !== undefined) intervals.push(timestamp - previous);
      previous = timestamp;
      if (intervals.length >= sampleCount) resolve(intervals);
      else requestAnimationFrame(sample);
    };
    requestAnimationFrame(sample);
  });
}

function elapsedMilliseconds(start: number): number {
  return roundMilliseconds(performance.now() - start);
}

function roundMilliseconds(value: number): number {
  return Math.round(value * 1000) / 1000;
}
