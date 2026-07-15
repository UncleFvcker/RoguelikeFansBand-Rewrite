// SPDX-License-Identifier: MPL-2.0

import {
  computeCameraOffset,
  MAP_CELL_SIZE,
  PLAYER_CENTERED_VIEW_CELLS,
  type CameraMode,
} from "./camera";
import { PixiRendererBackend } from "./pixi-renderer-backend";
import type { GameSnapshot, GameUpdate } from "./protocol";
import { RenderWorld } from "./render-world";
import type {
  RendererBackend,
  TilesetChangeResult,
} from "./renderer-backend";

export { type TilesetChangeResult } from "./renderer-backend";
export { type CameraMode } from "./camera";

export class MapRenderer {
  readonly #backend: RendererBackend;
  #world: RenderWorld | undefined;
  #host: HTMLElement | undefined;
  #cameraMode: CameraMode = "full-map";
  #resizeObserver: ResizeObserver | undefined;
  #width = 0;
  #height = 0;
  #totalAppliedCells = 0;

  constructor(backend: RendererBackend = new PixiRendererBackend()) {
    this.#backend = backend;
  }

  async initialize(
    host: HTMLElement,
    width: number,
    height: number,
    tilesetManifestUrl: string,
    contentGlyphs: Readonly<Record<string, string>>,
    canvasLabel: string,
    cameraMode: CameraMode = "full-map",
  ): Promise<TilesetChangeResult> {
    this.#host = host;
    this.#totalAppliedCells = 0;
    this.#width = width;
    this.#height = height;
    this.#cameraMode = cameraMode;
    this.#world = new RenderWorld(width, height);
    const result = await this.#backend.initialize({
      host,
      width,
      height,
      tilesetManifestUrl,
      contentGlyphs,
      canvasLabel,
    });
    host.dataset.rendererBackend = this.#backend.id;
    host.dataset.rendererLayerCount = "5";
    host.dataset.rendererLayers = "terrain,object,actor,visibility,lighting";
    host.dataset.visibilityMode = "all-visible";
    host.dataset.lightingMode = "presentation-player-v1";
    this.#configureViewport();
    if (typeof ResizeObserver !== "undefined") {
      this.#resizeObserver = new ResizeObserver(() => this.#updateCamera());
      this.#resizeObserver.observe(host);
    }
    this.#recordRender("tileset", 0, result.id);
    this.#updateCamera();
    return result;
  }

  setCanvasLabel(label: string): void {
    this.#backend.setCanvasLabel(label);
  }

  setCameraMode(mode: CameraMode): void {
    this.#cameraMode = mode;
    this.#configureViewport();
    this.#updateCamera();
  }

  async setTileset(tilesetManifestUrl: string): Promise<TilesetChangeResult> {
    const result = await this.#backend.setTileset(tilesetManifestUrl);
    const appliedCells = this.#backend.applyCells(this.#requireWorld().allCells());
    this.#recordRender("tileset", appliedCells, result.id);
    return result;
  }

  applySnapshot(snapshot: GameSnapshot): void {
    const cells = this.#requireWorld().applySnapshot(snapshot);
    const appliedCells = this.#backend.applyCells(cells);
    this.#recordRender("snapshot", appliedCells);
    this.#updateCamera();
  }

  applyUpdate(update: GameUpdate): void {
    const cells = this.#requireWorld().applyUpdate(update);
    const appliedCells = this.#backend.applyCells(cells);
    this.#recordRender("update", appliedCells);
    this.#updateCamera();
  }

  destroy(): void {
    this.#resizeObserver?.disconnect();
    this.#resizeObserver = undefined;
    this.#backend.destroy();
    this.#world = undefined;
    this.#host = undefined;
    this.#width = 0;
    this.#height = 0;
  }

  #configureViewport(): void {
    const host = this.#host;
    if (!host) return;
    const worldWidth = this.#width * MAP_CELL_SIZE;
    const worldHeight = this.#height * MAP_CELL_SIZE;
    const centeredWidth = Math.min(this.#width, PLAYER_CENTERED_VIEW_CELLS) * MAP_CELL_SIZE;
    const centeredHeight = Math.min(this.#height, PLAYER_CENTERED_VIEW_CELLS) * MAP_CELL_SIZE;
    host.dataset.cameraMode = this.#cameraMode;
    host.style.setProperty("--map-world-width", `${worldWidth}px`);
    host.style.setProperty("--map-world-height", `${worldHeight}px`);
    host.style.setProperty("--map-centered-width", `${centeredWidth}px`);
    host.style.setProperty("--map-centered-height", `${centeredHeight}px`);
  }

  #updateCamera(): void {
    const host = this.#host;
    const world = this.#world;
    if (!host || !world) return;
    const viewportWidth = host.clientWidth || this.#width * MAP_CELL_SIZE;
    const viewportHeight = host.clientHeight || this.#height * MAP_CELL_SIZE;
    const offset = computeCameraOffset({
      mode: this.#cameraMode,
      focus: world.playerPosition,
      worldWidth: this.#width * MAP_CELL_SIZE,
      worldHeight: this.#height * MAP_CELL_SIZE,
      viewportWidth,
      viewportHeight,
    });
    this.#backend.setCameraOffset(offset.x, offset.y);
    host.dataset.cameraX = String(offset.x);
    host.dataset.cameraY = String(offset.y);
    host.dataset.viewportWidth = String(viewportWidth);
    host.dataset.viewportHeight = String(viewportHeight);
  }

  #requireWorld(): RenderWorld {
    if (!this.#world) throw new Error("render world is not initialized");
    return this.#world;
  }

  #recordRender(
    kind: "snapshot" | "update" | "tileset",
    appliedCells: number,
    tilesetId?: string,
  ): void {
    const host = this.#host;
    if (!host) return;
    this.#totalAppliedCells += appliedCells;
    host.dataset.renderKind = kind;
    host.dataset.lastAppliedCells = String(appliedCells);
    host.dataset.totalAppliedCells = String(this.#totalAppliedCells);
    if (tilesetId) host.dataset.tilesetId = tilesetId;
  }
}
