// SPDX-License-Identifier: MPL-2.0

import { Assets, Rectangle, Texture } from "pixi.js";

import { GlyphAtlas } from "./glyph-atlas";
import {
  parseTilesetManifest,
  resolveTilesetVisual,
  type TilesetManifestV1,
} from "./tileset-manifest";

export interface RuntimeTileVisual {
  texture: Texture;
  tint: number;
  background?: number;
  usedFallback: boolean;
}

export type TilesetWarning = "image-too-small" | "image-load-failed";

export class TilesetRuntime {
  readonly manifest: TilesetManifestV1;
  readonly warnings: readonly TilesetWarning[];
  readonly #contentGlyphs: Readonly<Record<string, string>>;
  readonly #glyphAtlas: GlyphAtlas;
  readonly #imageAtlas: Texture | undefined;
  readonly #imageFrames = new Map<string, Texture>();

  private constructor(
    manifest: TilesetManifestV1,
    contentGlyphs: Readonly<Record<string, string>>,
    glyphAtlas: GlyphAtlas,
    imageAtlas: Texture | undefined,
    warnings: TilesetWarning[],
  ) {
    this.manifest = manifest;
    this.#contentGlyphs = contentGlyphs;
    this.#glyphAtlas = glyphAtlas;
    this.#imageAtlas = imageAtlas;
    this.warnings = warnings;
  }

  static async load(
    manifestUrl: string,
    contentGlyphs: Readonly<Record<string, string>>,
  ): Promise<TilesetRuntime> {
    const response = await fetch(manifestUrl, { cache: "no-store" });
    if (!response.ok) throw new Error(`tileset manifest request failed: HTTP ${response.status}`);
    const manifest = parseTilesetManifest(await response.json());
    const glyphs = [
      ...Object.values(contentGlyphs),
      ...Object.values(manifest.mappings).flatMap((mapping) =>
        mapping.glyph === undefined ? [] : [mapping.glyph],
      ),
      manifest.fallback.glyph,
    ];
    const glyphAtlas = new GlyphAtlas(
      glyphs,
      manifest.tileWidth,
      manifest.tileHeight,
      manifest.fallback.glyph,
    );
    const warnings: TilesetWarning[] = [];
    let imageAtlas: Texture | undefined;

    if (manifest.mode === "image" && manifest.atlas) {
      const atlasUrl = new URL(
        manifest.atlas.source,
        new URL(manifestUrl, window.location.href),
      ).toString();
      try {
        const loaded = await Assets.load<Texture>(atlasUrl);
        const expectedWidth = manifest.atlas.columns * manifest.tileWidth;
        const expectedHeight = manifest.atlas.rows * manifest.tileHeight;
        if (loaded.source.width < expectedWidth || loaded.source.height < expectedHeight) {
          warnings.push("image-too-small");
        } else {
          loaded.source.scaleMode = "nearest";
          imageAtlas = loaded;
        }
      } catch {
        warnings.push("image-load-failed");
      }
    }

    return new TilesetRuntime(manifest, contentGlyphs, glyphAtlas, imageAtlas, warnings);
  }

  resolve(semanticId: string): RuntimeTileVisual {
    const visual = resolveTilesetVisual(
      this.manifest,
      semanticId,
      this.#contentGlyphs,
      this.#imageAtlas !== undefined,
    );
    if (visual.source === "image" && visual.tile && this.#imageAtlas) {
      const key = `${visual.tile.x},${visual.tile.y}`;
      let texture = this.#imageFrames.get(key);
      if (!texture) {
        texture = new Texture({
          source: this.#imageAtlas.source,
          frame: new Rectangle(
            visual.tile.x * this.manifest.tileWidth,
            visual.tile.y * this.manifest.tileHeight,
            this.manifest.tileWidth,
            this.manifest.tileHeight,
          ),
        });
        this.#imageFrames.set(key, texture);
      }
      return {
        texture,
        tint: 0xffffff,
        ...(visual.background === undefined ? {} : { background: visual.background }),
        usedFallback: visual.usedFallback,
      };
    }
    return {
      texture: this.#glyphAtlas.texture(visual.glyph),
      tint: visual.foreground,
      ...(visual.background === undefined ? {} : { background: visual.background }),
      usedFallback: visual.usedFallback,
    };
  }

  destroy(): void {
    for (const texture of this.#imageFrames.values()) texture.destroy(false);
    this.#imageFrames.clear();
    this.#glyphAtlas.destroy();
  }
}
