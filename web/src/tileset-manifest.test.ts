// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { parseTilesetManifest, resolveTilesetVisual } from "./tileset-manifest.ts";

const glyphs = {
  "demo.terrain.floor": ".",
  "demo.terrain.wall": "#",
  "demo.actor.explorer": "@",
  "demo.actor.ember-mote": "*",
};

function readManifest(path: string): unknown {
  return JSON.parse(readFileSync(new URL(path, import.meta.url), "utf8"));
}

test("committed ASCII and image manifests pass strict parsing", () => {
  const ascii = parseTilesetManifest(
    readManifest("../public/tilesets/ascii-default/tileset.json"),
  );
  const image = parseTilesetManifest(readManifest("../public/tilesets/image-demo/tileset.json"));

  assert.equal(ascii.mode, "ascii");
  assert.equal(image.mode, "image");
  assert.equal(image.atlas?.columns, 3);
});

test("missing image tiles fall back to the shared glyph path", () => {
  const image = parseTilesetManifest(readManifest("../public/tilesets/image-demo/tileset.json"));
  const floorImage = resolveTilesetVisual(image, "demo.terrain.floor", glyphs, true);
  const floorWithoutAtlas = resolveTilesetVisual(image, "demo.terrain.floor", glyphs, false);
  const monsterWithoutTile = resolveTilesetVisual(image, "demo.actor.ember-mote", glyphs, true);

  assert.equal(floorImage.source, "image");
  assert.deepEqual(floorImage.tile, { x: 0, y: 0 });
  assert.equal(floorWithoutAtlas.source, "glyph");
  assert.equal(floorWithoutAtlas.glyph, ".");
  assert.equal(floorWithoutAtlas.usedFallback, true);
  assert.equal(monsterWithoutTile.source, "glyph");
  assert.equal(monsterWithoutTile.glyph, "✦");
});

test("unknown semantic IDs use the visible fallback style", () => {
  const ascii = parseTilesetManifest(
    readManifest("../public/tilesets/ascii-default/tileset.json"),
  );
  const visual = resolveTilesetVisual(ascii, "demo.terrain.unknown", glyphs, false);

  assert.equal(visual.glyph, "?");
  assert.equal(visual.foreground, 0xff77aa);
  assert.equal(visual.background, 0x2b1522);
  assert.equal(visual.usedFallback, true);
});

test("unsafe atlas paths and unknown fields are rejected", () => {
  const unsafe = readManifest("../public/tilesets/image-demo/tileset.json");
  unsafe.atlas.source = "../outside.svg";
  assert.throws(() => parseTilesetManifest(unsafe), /safe relative path/);

  const unknownField = readManifest("../public/tilesets/ascii-default/tileset.json");
  unknownField.unreviewedOption = true;
  assert.throws(() => parseTilesetManifest(unknownField), /unknown field/);
});
