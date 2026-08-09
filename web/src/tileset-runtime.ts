// SPDX-License-Identifier: MPL-2.0

import { Assets, Rectangle, Texture } from "pixi.js";

import { GlyphAtlas } from "./glyph-atlas";
import {
  parseTilesetManifest,
  resolveTilesetVisual,
  type TilesetManifestV1,
} from "./tileset-manifest";

export interface RuntimeTileVisual {
  source: "glyph" | "image";
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
  readonly #standaloneImages: Map<string, Texture>;
  readonly #imageFrames = new Map<string, Texture>();
  readonly #visualCache = new Map<string, RuntimeTileVisual>();

  private constructor(
    manifest: TilesetManifestV1,
    contentGlyphs: Readonly<Record<string, string>>,
    glyphAtlas: GlyphAtlas,
    imageAtlas: Texture | undefined,
    standaloneImages: Map<string, Texture>,
    warnings: TilesetWarning[],
  ) {
    this.manifest = manifest;
    this.#contentGlyphs = contentGlyphs;
    this.#glyphAtlas = glyphAtlas;
    this.#imageAtlas = imageAtlas;
    this.#standaloneImages = standaloneImages;
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
    const standaloneImages = new Map<string, Texture>();
    const warn = (warning: TilesetWarning): void => {
      if (!warnings.includes(warning)) warnings.push(warning);
    };

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
          warn("image-too-small");
        } else {
          loaded.source.scaleMode = "nearest";
          imageAtlas = loaded;
        }
      } catch {
        warn("image-load-failed");
      }
    }

    const imageSources = new Set(
      Object.values(manifest.mappings).flatMap((mapping) =>
        mapping.image === undefined ? [] : [mapping.image],
      ),
    );
    for (const imageSource of imageSources) {
      const imageUrl = new URL(imageSource, new URL(manifestUrl, window.location.href)).toString();
      try {
        const loaded = await Assets.load<Texture>(imageUrl);
        if (
          loaded.source.width < manifest.tileWidth ||
          loaded.source.height < manifest.tileHeight
        ) {
          warn("image-too-small");
        } else {
          loaded.source.scaleMode = "nearest";
          standaloneImages.set(imageSource, loaded);
        }
      } catch {
        warn("image-load-failed");
      }
    }

    return new TilesetRuntime(
      manifest,
      contentGlyphs,
      glyphAtlas,
      imageAtlas,
      standaloneImages,
      warnings,
    );
  }

  resolve(semanticId: string): RuntimeTileVisual {
    // Resolution is pure for the lifetime of a runtime instance, so each
    // semantic id only needs the manifest walk and colour parsing once.
    const cached = this.#visualCache.get(semanticId);
    if (cached) return cached;
    const resolved = this.#resolveUncached(semanticId);
    this.#visualCache.set(semanticId, resolved);
    return resolved;
  }

  #resolveUncached(semanticId: string): RuntimeTileVisual {
    const mapping = this.manifest.mappings[semanticId];
    const standaloneImage = mapping?.image
      ? this.#standaloneImages.get(mapping.image)
      : undefined;
    const visual = resolveTilesetVisual(
      this.manifest,
      semanticId,
      this.#contentGlyphs,
      mapping?.image ? standaloneImage !== undefined : this.#imageAtlas !== undefined,
    );
    if (visual.source === "image" && visual.image && standaloneImage) {
      return {
        source: "image",
        texture: standaloneImage,
        tint: 0xffffff,
        ...(visual.background === undefined ? {} : { background: visual.background }),
        usedFallback: visual.usedFallback,
      };
    }
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
        source: "image",
        texture,
        tint: 0xffffff,
        ...(visual.background === undefined ? {} : { background: visual.background }),
        usedFallback: visual.usedFallback,
      };
    }
    return {
      source: "glyph",
      texture: this.#glyphAtlas.texture(visual.glyph),
      tint: visual.foreground,
      ...(visual.background === undefined ? {} : { background: visual.background }),
      usedFallback: visual.usedFallback,
    };
  }

  destroy(): void {
    for (const texture of this.#imageFrames.values()) texture.destroy(false);
    this.#imageFrames.clear();
    this.#standaloneImages.clear();
    this.#glyphAtlas.destroy();
  }
}
