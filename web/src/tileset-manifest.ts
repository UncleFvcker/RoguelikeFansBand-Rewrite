// SPDX-License-Identifier: MPL-2.0

export const TILESET_SCHEMA =
  "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/tileset-v1.schema.json";

export type TilesetMode = "ascii" | "image";

export interface TileCoordinate {
  x: number;
  y: number;
}

export interface TilesetMapping {
  glyph?: string;
  foreground: string;
  background?: string;
  tile?: TileCoordinate;
}

export interface TilesetAtlas {
  source: string;
  columns: number;
  rows: number;
}

export interface TilesetFallback {
  glyph: string;
  foreground: string;
  background: string;
}

export interface TilesetManifestV1 {
  $schema: string;
  format: "rfb-tileset";
  formatVersion: 1;
  id: string;
  labelKey: string;
  mode: TilesetMode;
  tileWidth: number;
  tileHeight: number;
  atlas?: TilesetAtlas;
  mappings: Record<string, TilesetMapping>;
  fallback: TilesetFallback;
}

export interface ResolvedTilesetVisual {
  semanticId: string;
  source: "glyph" | "image";
  glyph: string;
  foreground: number;
  background?: number;
  tile?: TileCoordinate;
  usedFallback: boolean;
}

export function parseTilesetManifest(value: unknown): TilesetManifestV1 {
  const manifest = objectValue(value, "tileset manifest");
  assertKeys(
    manifest,
    [
      "$schema",
      "format",
      "formatVersion",
      "id",
      "labelKey",
      "mode",
      "tileWidth",
      "tileHeight",
      "atlas",
      "mappings",
      "fallback",
    ],
    "tileset manifest",
  );
  if (manifest.$schema !== TILESET_SCHEMA) fail("tileset manifest has an unknown $schema");
  if (manifest.format !== "rfb-tileset" || manifest.formatVersion !== 1) {
    fail("tileset manifest format/version is unsupported");
  }
  requireStableId(manifest.id, "tileset id");
  requireMessageKey(manifest.labelKey, "tileset labelKey");
  if (manifest.mode !== "ascii" && manifest.mode !== "image") {
    fail("tileset mode must be ascii or image");
  }
  requireInteger(manifest.tileWidth, 8, 128, "tileWidth");
  requireInteger(manifest.tileHeight, 8, 128, "tileHeight");

  const atlas = manifest.atlas === undefined ? undefined : parseAtlas(manifest.atlas);
  if (manifest.mode === "image" && !atlas) fail("image tileset requires an atlas");
  if (manifest.mode === "ascii" && atlas) fail("ascii tileset cannot declare an image atlas");

  const rawMappings = objectValue(manifest.mappings, "tileset mappings");
  const mappings: Record<string, TilesetMapping> = {};
  for (const [semanticId, rawMapping] of Object.entries(rawMappings)) {
    requireStableId(semanticId, "tileset mapping id");
    const mapping = parseMapping(rawMapping, manifest.mode, atlas);
    mappings[semanticId] = mapping;
  }
  if (Object.keys(mappings).length === 0) fail("tileset mappings cannot be empty");

  const fallback = parseFallback(manifest.fallback);
  return {
    $schema: TILESET_SCHEMA,
    format: "rfb-tileset",
    formatVersion: 1,
    id: manifest.id,
    labelKey: manifest.labelKey,
    mode: manifest.mode,
    tileWidth: manifest.tileWidth,
    tileHeight: manifest.tileHeight,
    ...(atlas ? { atlas } : {}),
    mappings,
    fallback,
  };
}

export function resolveTilesetVisual(
  manifest: TilesetManifestV1,
  semanticId: string,
  contentGlyphs: Readonly<Record<string, string>>,
  imageAvailable: boolean,
): ResolvedTilesetVisual {
  const mapping = manifest.mappings[semanticId];
  const tile = manifest.mode === "image" && imageAvailable ? mapping?.tile : undefined;
  const glyph = mapping?.glyph ?? contentGlyphs[semanticId] ?? manifest.fallback.glyph;
  const foreground = parseColor(mapping?.foreground ?? manifest.fallback.foreground);
  const background = mapping?.background
    ? parseColor(mapping.background)
    : mapping
      ? undefined
      : parseColor(manifest.fallback.background);
  return {
    semanticId,
    source: tile ? "image" : "glyph",
    glyph,
    foreground,
    ...(background === undefined ? {} : { background }),
    ...(tile ? { tile } : {}),
    usedFallback: !mapping || (manifest.mode === "image" && !tile),
  };
}

export function parseColor(value: string): number {
  if (!/^#[0-9a-fA-F]{6}$/.test(value)) fail(`invalid RGB color ${value}`);
  return Number.parseInt(value.slice(1), 16);
}

function parseAtlas(value: unknown): TilesetAtlas {
  const atlas = objectValue(value, "tileset atlas");
  assertKeys(atlas, ["source", "columns", "rows"], "tileset atlas");
  if (typeof atlas.source !== "string" || !isSafeRelativeAssetPath(atlas.source)) {
    fail("tileset atlas source must be a safe relative path");
  }
  requireInteger(atlas.columns, 1, 1024, "atlas columns");
  requireInteger(atlas.rows, 1, 1024, "atlas rows");
  return { source: atlas.source, columns: atlas.columns, rows: atlas.rows };
}

function parseMapping(
  value: unknown,
  mode: TilesetMode,
  atlas: TilesetAtlas | undefined,
): TilesetMapping {
  const mapping = objectValue(value, "tileset mapping");
  assertKeys(mapping, ["glyph", "foreground", "background", "tile"], "tileset mapping");
  if (mapping.glyph !== undefined) requireGlyph(mapping.glyph, "mapping glyph");
  requireColor(mapping.foreground, "mapping foreground");
  if (mapping.background !== undefined) requireColor(mapping.background, "mapping background");
  const tile = mapping.tile === undefined ? undefined : parseTile(mapping.tile, atlas);
  if (mode === "ascii" && tile) fail("ascii mappings cannot contain tile coordinates");
  return {
    ...(mapping.glyph === undefined ? {} : { glyph: mapping.glyph }),
    foreground: mapping.foreground,
    ...(mapping.background === undefined ? {} : { background: mapping.background }),
    ...(tile ? { tile } : {}),
  };
}

function parseTile(value: unknown, atlas: TilesetAtlas | undefined): TileCoordinate {
  if (!atlas) fail("tile coordinates require an image atlas");
  const tile = objectValue(value, "tile coordinate");
  assertKeys(tile, ["x", "y"], "tile coordinate");
  requireInteger(tile.x, 0, atlas.columns - 1, "tile x");
  requireInteger(tile.y, 0, atlas.rows - 1, "tile y");
  return { x: tile.x, y: tile.y };
}

function parseFallback(value: unknown): TilesetFallback {
  const fallback = objectValue(value, "tileset fallback");
  assertKeys(fallback, ["glyph", "foreground", "background"], "tileset fallback");
  requireGlyph(fallback.glyph, "fallback glyph");
  requireColor(fallback.foreground, "fallback foreground");
  requireColor(fallback.background, "fallback background");
  return {
    glyph: fallback.glyph,
    foreground: fallback.foreground,
    background: fallback.background,
  };
}

function objectValue(value: unknown, label: string): Record<string, any> {
  if (!value || typeof value !== "object" || Array.isArray(value)) fail(`${label} must be an object`);
  return value as Record<string, any>;
}

function assertKeys(value: Record<string, any>, allowed: readonly string[], label: string): void {
  const allowedKeys = new Set(allowed);
  for (const key of Object.keys(value)) {
    if (!allowedKeys.has(key)) fail(`${label} contains unknown field ${key}`);
  }
}

function requireStableId(value: unknown, label: string): asserts value is string {
  if (
    typeof value !== "string" ||
    value.length === 0 ||
    value.length > 128 ||
    value.split(".").length < 3 ||
    value.split(".").some((part) => part.length === 0) ||
    !/^[a-z0-9._-]+$/.test(value)
  ) {
    fail(`${label} is invalid`);
  }
}

function requireMessageKey(value: unknown, label: string): asserts value is string {
  if (typeof value !== "string" || !/^[a-z0-9_-]{1,128}$/.test(value)) fail(`${label} is invalid`);
}

function requireGlyph(value: unknown, label: string): asserts value is string {
  if (typeof value !== "string" || [...value].length !== 1 || /[\u0000-\u001f\u007f]/.test(value)) {
    fail(`${label} must contain one visible Unicode scalar`);
  }
}

function requireColor(value: unknown, label: string): asserts value is string {
  if (typeof value !== "string" || !/^#[0-9a-fA-F]{6}$/.test(value)) fail(`${label} is invalid`);
}

function requireInteger(value: unknown, minimum: number, maximum: number, label: string): asserts value is number {
  if (!Number.isInteger(value) || (value as number) < minimum || (value as number) > maximum) {
    fail(`${label} must be an integer between ${minimum} and ${maximum}`);
  }
}

function isSafeRelativeAssetPath(value: string): boolean {
  return (
    value.length > 0 &&
    value.length <= 256 &&
    !value.includes("://") &&
    !value.startsWith("/") &&
    !value.startsWith("\\") &&
    !value.includes("?") &&
    !value.includes("#") &&
    value.split(/[\\/]/).every((part) => part.length > 0 && part !== "." && part !== "..")
  );
}

function fail(message: string): never {
  throw new Error(message);
}
