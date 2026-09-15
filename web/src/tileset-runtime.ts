// SPDX-License-Identifier: MPL-2.0

import { Assets, Rectangle, Texture } from "pixi.js";

import type { EditableVisualDto } from "./protocol";
import { defaultVisuals, uniqueEffect, visualOverride, type VisualPreferences } from "./visual-preferences.ts";
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
  #glyphAtlas: GlyphAtlas;
  #visuals = defaultVisuals();
  #knownGlyphs: Record<string, string> = {};
  #visualKey = "";
  #renderGlyphs: Readonly<Record<string, string>>;
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
    this.#renderGlyphs = contentGlyphs;
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

  setVisuals(preferences: VisualPreferences, catalog: readonly EditableVisualDto[]): boolean {
    preferences = { ...preferences, overrides: Object.fromEntries(catalog.map(v => [v.id, visualOverride(preferences, v)])) };
    const knownGlyphs = Object.fromEntries(catalog.map(v => [v.id, v.glyph]));
    const signature = JSON.stringify([preferences, knownGlyphs]);
    if (signature === this.#visualKey) return false;
    const atlas = new GlyphAtlas([
      ...Object.values(this.#contentGlyphs), ...Object.values(knownGlyphs),
      ...Object.values(this.manifest.mappings).flatMap(v => v.glyph ? [v.glyph] : []),
      ...catalog.flatMap(v => preferences.overrides[v.id]?.glyph ? [preferences.overrides[v.id]!.glyph!] : []),
    ], this.manifest.tileWidth, this.manifest.tileHeight, this.manifest.fallback.glyph);
    const previous = this.#glyphAtlas;
    this.#glyphAtlas = atlas; this.#visuals = preferences; this.#knownGlyphs = knownGlyphs;
    this.#renderGlyphs = { ...this.#contentGlyphs, ...knownGlyphs };
    this.#visualKey = signature; this.#visualCache.clear(); previous.destroy();
    return true;
  }

  visualBase(id: string): { glyph: string; foreground: string; background?: string } {
    const mapping = this.manifest.mappings[id];
    return { glyph: mapping?.glyph ?? this.#knownGlyphs[id] ?? this.#contentGlyphs[id] ?? this.manifest.fallback.glyph,
      foreground: mapping?.foreground ?? this.manifest.fallback.foreground,
      background: mapping ? mapping.background : this.manifest.fallback.background };
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

  resolveGlyph(glyph: string): RuntimeTileVisual {
    return {
      source: "glyph",
      texture: this.#glyphAtlas.texture(glyph),
      tint: 0xffffff,
      usedFallback: false,
    };
  }

  uniqueEffect(id: string, unique: boolean) {
    return uniqueEffect(this.#visuals, id, unique && Object.hasOwn(this.#knownGlyphs, id));
  }

  #resolveUncached(semanticId: string): RuntimeTileVisual {
    const mapping = this.manifest.mappings[semanticId];
    const standaloneImage = mapping?.image
      ? this.#standaloneImages.get(mapping.image)
      : undefined;
    const visual = resolveTilesetVisual(
      this.manifest,
      semanticId,
      this.#renderGlyphs,
      mapping?.image ? standaloneImage !== undefined : this.#imageAtlas !== undefined,
      this.#visuals, Object.hasOwn(this.#knownGlyphs, semanticId),
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
