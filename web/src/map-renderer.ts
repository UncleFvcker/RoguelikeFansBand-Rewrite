// SPDX-License-Identifier: MPL-2.0

import { PixiRendererBackend } from "./pixi-renderer-backend";
import type { GameSnapshot, GameUpdate } from "./protocol";
import { RenderWorld } from "./render-world";
import type {
  RendererBackend,
  TilesetChangeResult,
} from "./renderer-backend";

export { type TilesetChangeResult } from "./renderer-backend";

export class MapRenderer {
  readonly #backend: RendererBackend;
  #world: RenderWorld | undefined;
  #host: HTMLElement | undefined;
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
  ): Promise<TilesetChangeResult> {
    this.#host = host;
    this.#totalAppliedCells = 0;
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
    this.#recordRender("tileset", 0, result.id);
    return result;
  }

  setCanvasLabel(label: string): void {
    this.#backend.setCanvasLabel(label);
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
  }

  applyUpdate(update: GameUpdate): void {
    const cells = this.#requireWorld().applyUpdate(update);
    const appliedCells = this.#backend.applyCells(cells);
    this.#recordRender("update", appliedCells);
  }

  destroy(): void {
    this.#backend.destroy();
    this.#world = undefined;
    this.#host = undefined;
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
