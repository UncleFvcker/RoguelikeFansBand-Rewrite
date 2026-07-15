// SPDX-License-Identifier: MPL-2.0

import { Application, Container, Graphics, Sprite } from "pixi.js";

import { CONTENT_GLYPHS } from "./content-visuals";
import type { CellDto, EntityDto, GameSnapshot, GameUpdate, PlayerDto } from "./protocol";
import { TilesetRuntime } from "./tileset-runtime";

const CELL_SIZE = 28;
const DEFAULT_BACKGROUND = 0x090d12;

interface CellView {
  background: Graphics;
  symbol: Sprite;
}

export interface TilesetChangeResult {
  id: string;
  warnings: readonly string[];
}

export class MapRenderer {
  readonly #application = new Application();
  readonly #world = new Container();
  readonly #actorKinds = new Map<string, string>();
  #width = 0;
  #height = 0;
  #cells: CellView[] = [];
  #cellData: Array<CellDto | undefined> = [];
  #tileset: TilesetRuntime | undefined;
  #host: HTMLElement | undefined;
  #totalAppliedCells = 0;

  async initialize(
    host: HTMLElement,
    width: number,
    height: number,
    tilesetManifestUrl: string,
  ): Promise<TilesetChangeResult> {
    this.#host = host;
    this.#width = width;
    this.#height = height;
    this.#tileset = await TilesetRuntime.load(tilesetManifestUrl, CONTENT_GLYPHS);
    await this.#application.init({
      width: width * CELL_SIZE,
      height: height * CELL_SIZE,
      background: "#090d12",
      antialias: false,
      resolution: window.devicePixelRatio,
      autoDensity: true,
    });
    this.#application.canvas.setAttribute("aria-label", "原创测试地图画布");
    host.replaceChildren(this.#application.canvas);
    this.#application.stage.addChild(this.#world);
    this.#createCells();
    this.#recordRender("tileset", 0);
    return this.#tilesetResult();
  }

  async setTileset(tilesetManifestUrl: string): Promise<TilesetChangeResult> {
    const replacement = await TilesetRuntime.load(tilesetManifestUrl, CONTENT_GLYPHS);
    const previous = this.#tileset;
    this.#tileset = replacement;
    previous?.destroy();
    let appliedCells = 0;
    for (const cell of this.#cellData) {
      if (cell) {
        this.#applyCell(cell);
        appliedCells += 1;
      }
    }
    this.#recordRender("tileset", appliedCells);
    return this.#tilesetResult();
  }

  applySnapshot(snapshot: GameSnapshot): void {
    this.#syncActorKinds(snapshot.player, snapshot.entities);
    for (const cell of snapshot.cells) this.#storeAndApplyCell(cell);
    this.#recordRender("snapshot", snapshot.cells.length);
  }

  applyUpdate(update: GameUpdate): void {
    this.#syncActorKinds(update.player, update.entities);
    for (const cell of update.changedCells) this.#storeAndApplyCell(cell);
    this.#recordRender("update", update.changedCells.length);
  }

  destroy(): void {
    this.#tileset?.destroy();
    this.#tileset = undefined;
    this.#host = undefined;
    this.#application.destroy(true, { children: true });
  }

  #createCells(): void {
    this.#cellData = new Array(this.#width * this.#height);
    for (let y = 0; y < this.#height; y += 1) {
      for (let x = 0; x < this.#width; x += 1) {
        const background = new Graphics();
        const symbol = new Sprite({ roundPixels: true });
        symbol.position.set(x * CELL_SIZE, y * CELL_SIZE);
        this.#world.addChild(background, symbol);
        this.#cells.push({ background, symbol });
      }
    }
  }

  #syncActorKinds(player: PlayerDto, entities: EntityDto[]): void {
    this.#actorKinds.clear();
    this.#actorKinds.set(player.id, player.kindId);
    for (const entity of entities) this.#actorKinds.set(entity.id, entity.kindId);
  }

  #storeAndApplyCell(cell: CellDto): void {
    const index = cell.position.y * this.#width + cell.position.x;
    if (index < 0 || index >= this.#cellData.length) return;
    this.#cellData[index] = cell;
    this.#applyCell(cell);
  }

  #applyCell(cell: CellDto): void {
    const index = cell.position.y * this.#width + cell.position.x;
    const view = this.#cells[index];
    const tileset = this.#tileset;
    if (!view || !tileset) return;

    const terrainVisual = tileset.resolve(cell.terrainId);
    const actorSemanticId = cell.actorId
      ? (this.#actorKinds.get(cell.actorId) ?? cell.actorId)
      : undefined;
    const symbolVisual = actorSemanticId ? tileset.resolve(actorSemanticId) : terrainVisual;
    const background = symbolVisual.background ?? terrainVisual.background ?? DEFAULT_BACKGROUND;
    const x = cell.position.x * CELL_SIZE;
    const y = cell.position.y * CELL_SIZE;

    view.background
      .clear()
      .rect(x, y, CELL_SIZE, CELL_SIZE)
      .fill(background)
      .rect(x, y, CELL_SIZE, CELL_SIZE)
      .stroke({ color: "#18212d", width: 1, alpha: 0.55 });
    view.symbol.texture = symbolVisual.texture;
    view.symbol.tint = symbolVisual.tint;
    view.symbol.width = CELL_SIZE;
    view.symbol.height = CELL_SIZE;
  }

  #tilesetResult(): TilesetChangeResult {
    const tileset = this.#tileset;
    if (!tileset) throw new Error("tileset 尚未初始化");
    return { id: tileset.manifest.id, warnings: tileset.warnings };
  }

  #recordRender(kind: "snapshot" | "update" | "tileset", appliedCells: number): void {
    const host = this.#host;
    const tileset = this.#tileset;
    if (!host || !tileset) return;
    this.#totalAppliedCells += appliedCells;
    host.dataset.renderKind = kind;
    host.dataset.lastAppliedCells = String(appliedCells);
    host.dataset.totalAppliedCells = String(this.#totalAppliedCells);
    host.dataset.tilesetId = tileset.manifest.id;
  }
}
