// SPDX-License-Identifier: MPL-2.0

import { Application, Container, Graphics, Text } from "pixi.js";

import type { CellDto, GameSnapshot, GameUpdate } from "./protocol";

const CELL_SIZE = 28;

interface CellView {
  background: Graphics;
  glyph: Text;
}

export class MapRenderer {
  readonly #application = new Application();
  readonly #world = new Container();
  #width = 0;
  #height = 0;
  #cells: CellView[] = [];

  async initialize(host: HTMLElement, width: number, height: number): Promise<void> {
    this.#width = width;
    this.#height = height;
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
  }

  applySnapshot(snapshot: GameSnapshot): void {
    for (const cell of snapshot.cells) this.#applyCell(cell);
  }

  applyUpdate(update: GameUpdate): void {
    for (const cell of update.changedCells) this.#applyCell(cell);
  }

  destroy(): void {
    this.#application.destroy(true, { children: true });
  }

  #createCells(): void {
    for (let y = 0; y < this.#height; y += 1) {
      for (let x = 0; x < this.#width; x += 1) {
        const background = new Graphics();
        const glyph = new Text({
          text: "",
          style: {
            fontFamily: "Consolas, 'Cascadia Mono', monospace",
            fontSize: 20,
            fill: "#d8e1ed",
          },
        });
        glyph.anchor.set(0.5);
        glyph.position.set(x * CELL_SIZE + CELL_SIZE / 2, y * CELL_SIZE + CELL_SIZE / 2);
        this.#world.addChild(background, glyph);
        this.#cells.push({ background, glyph });
      }
    }
  }

  #applyCell(cell: CellDto): void {
    const index = cell.position.y * this.#width + cell.position.x;
    const view = this.#cells[index];
    if (!view) return;

    const x = cell.position.x * CELL_SIZE;
    const y = cell.position.y * CELL_SIZE;
    const visual = appearance(cell);
    view.background
      .clear()
      .rect(x, y, CELL_SIZE, CELL_SIZE)
      .fill(visual.background)
      .rect(x, y, CELL_SIZE, CELL_SIZE)
      .stroke({ color: "#18212d", width: 1, alpha: 0.55 });
    view.glyph.text = visual.glyph;
    view.glyph.tint = visual.foreground;
  }
}

function appearance(cell: CellDto): {
  background: number;
  foreground: number;
  glyph: string;
} {
  if (cell.actorId === "demo.player") {
    return { background: 0x17324a, foreground: 0x8ed8ff, glyph: "@" };
  }
  if (cell.actorId?.startsWith("demo.monster.ember-mote")) {
    return { background: 0x47251c, foreground: 0xff9b5e, glyph: "✦" };
  }
  if (cell.terrainId === "demo.terrain.wall") {
    return { background: 0x26303d, foreground: 0x8190a3, glyph: "#" };
  }
  return { background: 0x111820, foreground: 0x334252, glyph: "·" };
}

