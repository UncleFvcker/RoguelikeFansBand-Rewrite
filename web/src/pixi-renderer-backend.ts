// SPDX-License-Identifier: MPL-2.0

import { Application, Container, Graphics, Sprite } from "pixi.js";

import { MAP_CELL_SIZE, type CameraTransform } from "./camera";
import { ensureContrast } from "./render-color";
import type {
  BackendInitialization,
  RenderCell,
  RendererBackend,
  TilesetChangeResult,
} from "./renderer-backend";
import { TilesetRuntime, type RuntimeTileVisual } from "./tileset-runtime";

const DEFAULT_BACKGROUND = 0x090d12;
const GRID_COLOR = 0x18212d;

interface CellView {
  terrainBackground: Graphics;
  terrainSymbol: Sprite;
  itemBackground: Graphics;
  itemSymbol: Sprite;
  actorBackground: Graphics;
  actorSymbol: Sprite;
  visibilityMask: Graphics;
  lightColor: Graphics;
  darkness: Graphics;
}

export class PixiRendererBackend implements RendererBackend {
  readonly id = "pixi-layered-v1";
  readonly #application = new Application();
  readonly #camera = new Container();
  readonly #terrainLayer = new Container();
  readonly #objectLayer = new Container();
  readonly #actorLayer = new Container();
  readonly #visibilityLayer = new Container();
  readonly #lightingLayer = new Container();
  #cells: CellView[] = [];
  #tileset: TilesetRuntime | undefined;
  #contentGlyphs: Readonly<Record<string, string>> = {};
  #host: HTMLElement | undefined;
  #width = 0;
  #height = 0;
  #zoom: CameraTransform["zoom"] = 1;

  async initialize(options: BackendInitialization): Promise<TilesetChangeResult> {
    this.#host = options.host;
    this.#width = options.width;
    this.#height = options.height;
    this.#zoom = options.zoom ?? 1;
    this.#contentGlyphs = options.contentGlyphs;
    this.#tileset = await TilesetRuntime.load(options.tilesetManifestUrl, options.contentGlyphs);
    await this.#application.init({
      width: options.width * MAP_CELL_SIZE * this.#zoom,
      height: options.height * MAP_CELL_SIZE * this.#zoom,
      background: "#090d12",
      antialias: false,
      resolution: window.devicePixelRatio,
      autoDensity: true,
    });
    this.#application.canvas.setAttribute("aria-label", options.canvasLabel);
    options.host.replaceChildren(this.#application.canvas);
    this.#camera.scale.set(this.#zoom);
    this.#application.stage.addChild(this.#camera);
    this.#camera.addChild(
      this.#terrainLayer,
      this.#objectLayer,
      this.#actorLayer,
      this.#visibilityLayer,
      this.#lightingLayer,
    );
    this.#createCells();
    return this.#tilesetResult();
  }

  setCameraTransform(transform: CameraTransform): void {
    if (this.#zoom !== transform.zoom) {
      this.#zoom = transform.zoom;
      this.#application.renderer.resize(
        this.#width * MAP_CELL_SIZE * this.#zoom,
        this.#height * MAP_CELL_SIZE * this.#zoom,
      );
    }
    this.#camera.scale.set(this.#zoom);
    this.#camera.position.set(transform.x, transform.y);
  }

  applyCells(cells: readonly RenderCell[]): number {
    let applied = 0;
    for (const cell of cells) {
      const view = this.#cells[cell.index];
      if (!view || !this.#tileset) continue;
      this.#applyCell(view, cell, this.#tileset);
      applied += 1;
    }
    return applied;
  }

  async setTileset(tilesetManifestUrl: string): Promise<TilesetChangeResult> {
    const replacement = await TilesetRuntime.load(tilesetManifestUrl, this.#contentGlyphs);
    const previous = this.#tileset;
    this.#tileset = replacement;
    previous?.destroy();
    return this.#tilesetResult();
  }

  setCanvasLabel(label: string): void {
    if (this.#host) this.#application.canvas.setAttribute("aria-label", label);
  }

  destroy(): void {
    this.#tileset?.destroy();
    this.#tileset = undefined;
    this.#cells = [];
    this.#host = undefined;
    this.#application.destroy(true, { children: true });
  }

  #createCells(): void {
    this.#cells = new Array(this.#width * this.#height);
    for (let y = 0; y < this.#height; y += 1) {
      for (let x = 0; x < this.#width; x += 1) {
        const terrainBackground = new Graphics();
        const terrainSymbol = cellSprite(x, y);
        const itemBackground = new Graphics();
        const itemSymbol = cellSprite(x, y);
        const actorBackground = new Graphics();
        const actorSymbol = cellSprite(x, y);
        const visibilityMask = new Graphics();
        const lightColor = new Graphics();
        const darkness = new Graphics();
        this.#terrainLayer.addChild(terrainBackground, terrainSymbol);
        this.#objectLayer.addChild(itemBackground, itemSymbol);
        this.#actorLayer.addChild(actorBackground, actorSymbol);
        this.#visibilityLayer.addChild(visibilityMask);
        this.#lightingLayer.addChild(lightColor, darkness);
        this.#cells[y * this.#width + x] = {
          terrainBackground,
          terrainSymbol,
          itemBackground,
          itemSymbol,
          actorBackground,
          actorSymbol,
          visibilityMask,
          lightColor,
          darkness,
        };
      }
    }
  }

  #applyCell(view: CellView, cell: RenderCell, tileset: TilesetRuntime): void {
    const terrain = tileset.resolve(cell.terrainId);
    const item = cell.itemKindId ? tileset.resolve(cell.itemKindId) : undefined;
    const actor = cell.actorKindId ? tileset.resolve(cell.actorKindId) : undefined;
    const terrainBackground = terrain.background ?? DEFAULT_BACKGROUND;
    drawBackground(view.terrainBackground, cell, terrainBackground, true);
    applyVisual(view.terrainSymbol, terrain, terrainBackground);
    applyLayerVisual(view.itemBackground, view.itemSymbol, cell, item, terrainBackground);
    applyLayerVisual(
      view.actorBackground,
      view.actorSymbol,
      cell,
      actor,
      item?.background ?? terrainBackground,
    );
    drawVisibility(view.visibilityMask, cell);
    drawLighting(view.lightColor, view.darkness, cell);
  }

  #tilesetResult(): TilesetChangeResult {
    const tileset = this.#tileset;
    if (!tileset) throw new Error("tileset runtime is not initialized");
    return { id: tileset.manifest.id, warnings: tileset.warnings };
  }
}

function cellSprite(x: number, y: number): Sprite {
  const sprite = new Sprite({ roundPixels: true });
  sprite.position.set(x * MAP_CELL_SIZE, y * MAP_CELL_SIZE);
  sprite.width = MAP_CELL_SIZE;
  sprite.height = MAP_CELL_SIZE;
  return sprite;
}

function applyLayerVisual(
  background: Graphics,
  sprite: Sprite,
  cell: RenderCell,
  visual: RuntimeTileVisual | undefined,
  inheritedBackground: number,
): void {
  if (!visual) {
    background.clear();
    sprite.visible = false;
    return;
  }
  const layerBackground = visual.background ?? inheritedBackground;
  if (visual.background === undefined) background.clear();
  else drawBackground(background, cell, layerBackground, false);
  applyVisual(sprite, visual, layerBackground);
}

function applyVisual(sprite: Sprite, visual: RuntimeTileVisual, background: number): void {
  sprite.visible = true;
  sprite.texture = visual.texture;
  sprite.tint = visual.source === "glyph" ? ensureContrast(visual.tint, background) : visual.tint;
}

function drawBackground(
  graphics: Graphics,
  cell: RenderCell,
  color: number,
  includeGrid: boolean,
): void {
  const x = cell.x * MAP_CELL_SIZE;
  const y = cell.y * MAP_CELL_SIZE;
  graphics.clear().rect(x, y, MAP_CELL_SIZE, MAP_CELL_SIZE).fill(color);
  if (includeGrid) {
    graphics
      .rect(x, y, MAP_CELL_SIZE, MAP_CELL_SIZE)
      .stroke({ color: GRID_COLOR, width: 1, alpha: 0.55 });
  }
}

function drawVisibility(graphics: Graphics, cell: RenderCell): void {
  graphics.clear();
  if (cell.visibility === "visible") return;
  const color = cell.visibility === "remembered" ? 0x12213a : 0x000000;
  const alpha = cell.visibility === "remembered" ? 0.58 : 1;
  graphics
    .rect(
      cell.x * MAP_CELL_SIZE,
      cell.y * MAP_CELL_SIZE,
      MAP_CELL_SIZE,
      MAP_CELL_SIZE,
    )
    .fill({ color, alpha });
}

function drawLighting(lightColor: Graphics, darkness: Graphics, cell: RenderCell): void {
  const x = cell.x * MAP_CELL_SIZE;
  const y = cell.y * MAP_CELL_SIZE;
  const intensity = Math.max(0, Math.min(1, cell.light.intensity));
  lightColor.clear();
  darkness.clear();
  const colorAlpha = Math.max(0, intensity - 0.5) * 0.18;
  if (colorAlpha > 0) {
    lightColor
      .rect(x, y, MAP_CELL_SIZE, MAP_CELL_SIZE)
      .fill({ color: cell.light.color, alpha: colorAlpha });
  }
  const darknessAlpha = (1 - intensity) * 0.62;
  if (darknessAlpha > 0) {
    darkness
      .rect(x, y, MAP_CELL_SIZE, MAP_CELL_SIZE)
      .fill({ color: 0x000000, alpha: darknessAlpha });
  }
}
