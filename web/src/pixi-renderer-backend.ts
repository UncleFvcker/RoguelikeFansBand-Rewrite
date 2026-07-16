// SPDX-License-Identifier: MPL-2.0

import {
  Application,
  Container,
  Graphics,
  Rectangle,
  Sprite,
  type Texture,
} from "pixi.js";

import { MAP_CELL_SIZE, type CameraTransform } from "./camera";
import { ensureContrast } from "./render-color";
import {
  TERRAIN_CHUNK_SIZE,
  chunkIndexForCell,
  createRenderChunkLayout,
  updateTerrainChunkState,
  visibleRenderChunkIndexes,
  type RenderChunk,
  type RenderChunkLayout,
} from "./render-chunks";
import type {
  BackendInitialization,
  RenderCell,
  RendererBackend,
  RendererBackendDiagnostics,
  TilesetChangeResult,
} from "./renderer-backend";
import { TilesetRuntime, type RuntimeTileVisual } from "./tileset-runtime";

const DEFAULT_BACKGROUND = 0x090d12;
const GRID_COLOR = 0x18212d;
const DYNAMIC_DISPLAY_OBJECTS_PER_CELL = 7;

export interface PixiRendererBackendOptions {
  terrainChunkSize?: number;
}

interface CellView {
  itemBackground: Graphics;
  itemSymbol: Sprite;
  actorBackground: Graphics;
  actorSymbol: Sprite;
  visibilityMask: Graphics;
  lightColor: Graphics;
  darkness: Graphics;
}

interface ChunkView {
  descriptor: RenderChunk;
  terrainSprite: Sprite;
  terrainTexture?: Texture;
  objectLayer: Container;
  actorLayer: Container;
  visibilityLayer: Container;
  lightingLayer: Container;
}

export class PixiRendererBackend implements RendererBackend {
  readonly id = "pixi-layered-chunks-v2";
  readonly #application = new Application();
  readonly #camera = new Container();
  readonly #terrainLayer = new Container();
  readonly #objectLayer = new Container();
  readonly #actorLayer = new Container();
  readonly #visibilityLayer = new Container();
  readonly #lightingLayer = new Container();
  #layout: RenderChunkLayout | undefined;
  #chunks: ChunkView[] = [];
  #cells: CellView[] = [];
  #terrainIds: Array<string | undefined> = [];
  #tileset: TilesetRuntime | undefined;
  #contentGlyphs: Readonly<Record<string, string>> = {};
  #host: HTMLElement | undefined;
  #width = 0;
  #height = 0;
  #zoom: CameraTransform["zoom"] = 1;
  #forceTerrainRebuild = true;
  #visibleChunkCount = 0;
  #lastRebuiltTerrainChunks = 0;
  #totalRebuiltTerrainChunks = 0;
  readonly #terrainChunkSize: number;

  constructor(options: PixiRendererBackendOptions = {}) {
    const terrainChunkSize = options.terrainChunkSize ?? TERRAIN_CHUNK_SIZE;
    if (!Number.isInteger(terrainChunkSize) || terrainChunkSize <= 0) {
      throw new Error("terrain chunk size must be a positive integer");
    }
    this.#terrainChunkSize = terrainChunkSize;
  }

  async initialize(options: BackendInitialization): Promise<TilesetChangeResult> {
    this.#host = options.host;
    this.#width = options.width;
    this.#height = options.height;
    this.#zoom = options.zoom ?? 1;
    this.#contentGlyphs = options.contentGlyphs;
    this.#layout = createRenderChunkLayout(
      options.width,
      options.height,
      this.#terrainChunkSize,
    );
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
    this.#createChunksAndCells();
    return this.#tilesetResult();
  }

  getDiagnostics(): RendererBackendDiagnostics {
    return {
      terrainChunkSize: this.#terrainChunkSize,
      terrainChunkCount: this.#chunks.length,
      visibleChunkCount: this.#visibleChunkCount,
      lastRebuiltTerrainChunks: this.#lastRebuiltTerrainChunks,
      totalRebuiltTerrainChunks: this.#totalRebuiltTerrainChunks,
      cellViewCount: this.#cells.length,
      dynamicDisplayObjectCount:
        this.#cells.length * DYNAMIC_DISPLAY_OBJECTS_PER_CELL,
    };
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
    this.#applyChunkCulling(transform);
  }

  applyCells(cells: readonly RenderCell[]): number {
    const layout = this.#layout;
    if (!layout) return 0;
    const dirtyChunks = updateTerrainChunkState(
      this.#terrainIds,
      cells,
      layout.chunksAcross,
      this.#chunks.length,
      this.#forceTerrainRebuild,
      this.#terrainChunkSize,
    );

    let applied = 0;
    for (const cell of cells) {
      const view = this.#cells[cell.index];
      if (!view || !this.#tileset) continue;
      this.#applyDynamicCell(view, cell, this.#tileset);
      applied += 1;
    }

    for (const chunkIndex of [...dirtyChunks].sort((left, right) => left - right)) {
      this.#rebuildTerrainChunk(chunkIndex);
    }
    this.#forceTerrainRebuild = false;
    this.#lastRebuiltTerrainChunks = dirtyChunks.size;
    this.#totalRebuiltTerrainChunks += dirtyChunks.size;
    return applied;
  }

  async setTileset(tilesetManifestUrl: string): Promise<TilesetChangeResult> {
    const replacement = await TilesetRuntime.load(tilesetManifestUrl, this.#contentGlyphs);
    const previous = this.#tileset;
    this.#tileset = replacement;
    this.#forceTerrainRebuild = true;
    previous?.destroy();
    return this.#tilesetResult();
  }

  setCanvasLabel(label: string): void {
    if (this.#host) this.#application.canvas.setAttribute("aria-label", label);
  }

  destroy(): void {
    for (const chunk of this.#chunks) chunk.terrainTexture?.destroy(true);
    this.#tileset?.destroy();
    this.#tileset = undefined;
    this.#layout = undefined;
    this.#chunks = [];
    this.#cells = [];
    this.#terrainIds = [];
    this.#host = undefined;
    this.#application.destroy(true, { children: true });
  }

  #createChunksAndCells(): void {
    const layout = this.#layout;
    if (!layout) throw new Error("render chunk layout is not initialized");
    this.#chunks = layout.chunks.map((descriptor) => {
      const terrainSprite = new Sprite({ roundPixels: true });
      terrainSprite.position.set(
        descriptor.cellX * MAP_CELL_SIZE,
        descriptor.cellY * MAP_CELL_SIZE,
      );
      terrainSprite.width = descriptor.cellWidth * MAP_CELL_SIZE;
      terrainSprite.height = descriptor.cellHeight * MAP_CELL_SIZE;
      const objectLayer = new Container();
      const actorLayer = new Container();
      const visibilityLayer = new Container();
      const lightingLayer = new Container();
      this.#terrainLayer.addChild(terrainSprite);
      this.#objectLayer.addChild(objectLayer);
      this.#actorLayer.addChild(actorLayer);
      this.#visibilityLayer.addChild(visibilityLayer);
      this.#lightingLayer.addChild(lightingLayer);
      return {
        descriptor,
        terrainSprite,
        objectLayer,
        actorLayer,
        visibilityLayer,
        lightingLayer,
      };
    });
    this.#visibleChunkCount = this.#chunks.length;
    this.#terrainIds = new Array(this.#width * this.#height);
    this.#cells = new Array(this.#width * this.#height);
    for (let y = 0; y < this.#height; y += 1) {
      for (let x = 0; x < this.#width; x += 1) {
        const chunk = this.#chunks[
          chunkIndexForCell(x, y, layout.chunksAcross, this.#terrainChunkSize)
        ];
        if (!chunk) throw new Error("render cell resolved to a missing chunk");
        const itemBackground = new Graphics();
        const itemSymbol = cellSprite(x, y);
        const actorBackground = new Graphics();
        const actorSymbol = cellSprite(x, y);
        const visibilityMask = new Graphics();
        const lightColor = new Graphics();
        const darkness = new Graphics();
        chunk.objectLayer.addChild(itemBackground, itemSymbol);
        chunk.actorLayer.addChild(actorBackground, actorSymbol);
        chunk.visibilityLayer.addChild(visibilityMask);
        chunk.lightingLayer.addChild(lightColor, darkness);
        this.#cells[y * this.#width + x] = {
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

  #applyDynamicCell(view: CellView, cell: RenderCell, tileset: TilesetRuntime): void {
    const terrain = tileset.resolve(cell.terrainId);
    const item = cell.itemKindId ? tileset.resolve(cell.itemKindId) : undefined;
    const actor = cell.actorKindId ? tileset.resolve(cell.actorKindId) : undefined;
    const terrainBackground = terrain.background ?? DEFAULT_BACKGROUND;
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

  #rebuildTerrainChunk(chunkIndex: number): void {
    const chunk = this.#chunks[chunkIndex];
    const tileset = this.#tileset;
    if (!chunk || !tileset) return;
    const { descriptor } = chunk;
    const source = new Container();
    for (let localY = 0; localY < descriptor.cellHeight; localY += 1) {
      for (let localX = 0; localX < descriptor.cellWidth; localX += 1) {
        const worldX = descriptor.cellX + localX;
        const worldY = descriptor.cellY + localY;
        const terrainId = this.#terrainIds[worldY * this.#width + worldX];
        if (!terrainId) continue;
        const terrain = tileset.resolve(terrainId);
        const background = terrain.background ?? DEFAULT_BACKGROUND;
        const terrainBackground = new Graphics();
        drawTerrainBackground(terrainBackground, localX, localY, background);
        const terrainSymbol = cellSprite(localX, localY);
        applyVisual(terrainSymbol, terrain, background);
        source.addChild(terrainBackground, terrainSymbol);
      }
    }
    const pixelWidth = descriptor.cellWidth * MAP_CELL_SIZE;
    const pixelHeight = descriptor.cellHeight * MAP_CELL_SIZE;
    const texture = this.#application.renderer.generateTexture({
      target: source,
      frame: new Rectangle(0, 0, pixelWidth, pixelHeight),
      resolution: window.devicePixelRatio,
      antialias: false,
      textureSourceOptions: { scaleMode: "nearest" },
    });
    source.destroy({ children: true });
    chunk.terrainTexture?.destroy(true);
    chunk.terrainTexture = texture;
    chunk.terrainSprite.texture = texture;
    chunk.terrainSprite.width = pixelWidth;
    chunk.terrainSprite.height = pixelHeight;
  }

  #applyChunkCulling(transform: CameraTransform): void {
    const layout = this.#layout;
    if (!layout) return;
    const visible = visibleRenderChunkIndexes(layout.chunks, transform);
    this.#visibleChunkCount = visible.size;
    for (const chunk of this.#chunks) {
      const chunkVisible = visible.has(chunk.descriptor.index);
      chunk.terrainSprite.visible = chunkVisible;
      chunk.objectLayer.visible = chunkVisible;
      chunk.actorLayer.visible = chunkVisible;
      chunk.visibilityLayer.visible = chunkVisible;
      chunk.lightingLayer.visible = chunkVisible;
    }
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
  else drawBackground(background, cell, layerBackground);
  applyVisual(sprite, visual, layerBackground);
}

function applyVisual(sprite: Sprite, visual: RuntimeTileVisual, background: number): void {
  sprite.visible = true;
  sprite.texture = visual.texture;
  sprite.tint = visual.source === "glyph" ? ensureContrast(visual.tint, background) : visual.tint;
}

function drawTerrainBackground(
  graphics: Graphics,
  cellX: number,
  cellY: number,
  color: number,
): void {
  const x = cellX * MAP_CELL_SIZE;
  const y = cellY * MAP_CELL_SIZE;
  graphics
    .rect(x, y, MAP_CELL_SIZE, MAP_CELL_SIZE)
    .fill(color)
    .rect(x, y, MAP_CELL_SIZE, MAP_CELL_SIZE)
    .stroke({ color: GRID_COLOR, width: 1, alpha: 0.55 });
}

function drawBackground(graphics: Graphics, cell: RenderCell, color: number): void {
  graphics
    .clear()
    .rect(
      cell.x * MAP_CELL_SIZE,
      cell.y * MAP_CELL_SIZE,
      MAP_CELL_SIZE,
      MAP_CELL_SIZE,
    )
    .fill(color);
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
