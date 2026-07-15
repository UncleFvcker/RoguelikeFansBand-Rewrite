// SPDX-License-Identifier: MPL-2.0

import { Rectangle, Texture } from "pixi.js";

export class GlyphAtlas {
  readonly #atlasTexture: Texture;
  readonly #textures = new Map<string, Texture>();
  readonly #fallbackGlyph: string;

  constructor(glyphs: Iterable<string>, tileWidth: number, tileHeight: number, fallbackGlyph: string) {
    this.#fallbackGlyph = fallbackGlyph;
    const uniqueGlyphs = [...new Set([...glyphs, fallbackGlyph])].sort();
    const columns = Math.min(16, uniqueGlyphs.length);
    const rows = Math.ceil(uniqueGlyphs.length / columns);
    const resolution = Math.max(1, Math.min(window.devicePixelRatio, 2));
    const frameWidth = tileWidth * resolution;
    const frameHeight = tileHeight * resolution;
    const canvas = document.createElement("canvas");
    canvas.width = columns * frameWidth;
    canvas.height = rows * frameHeight;
    const context = canvas.getContext("2d");
    if (!context) throw new Error("无法创建 ASCII glyph atlas");

    context.clearRect(0, 0, canvas.width, canvas.height);
    context.fillStyle = "#ffffff";
    context.textAlign = "center";
    context.textBaseline = "middle";
    context.font = `${Math.floor(tileHeight * 0.72 * resolution)}px Consolas, "Cascadia Mono", monospace`;

    for (const [index, glyph] of uniqueGlyphs.entries()) {
      const column = index % columns;
      const row = Math.floor(index / columns);
      context.fillText(
        glyph,
        column * frameWidth + frameWidth / 2,
        row * frameHeight + frameHeight / 2 + resolution,
      );
    }

    this.#atlasTexture = Texture.from(canvas, true);
    this.#atlasTexture.source.scaleMode = "linear";
    for (const [index, glyph] of uniqueGlyphs.entries()) {
      const column = index % columns;
      const row = Math.floor(index / columns);
      this.#textures.set(
        glyph,
        new Texture({
          source: this.#atlasTexture.source,
          frame: new Rectangle(column * frameWidth, row * frameHeight, frameWidth, frameHeight),
        }),
      );
    }
  }

  texture(glyph: string): Texture {
    return this.#textures.get(glyph) ?? this.#textures.get(this.#fallbackGlyph) ?? Texture.EMPTY;
  }

  destroy(): void {
    for (const texture of this.#textures.values()) texture.destroy(false);
    this.#textures.clear();
    this.#atlasTexture.destroy(true);
  }
}
