// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { parseTilesetManifest, resolveTilesetVisual } from "./tileset-manifest.ts";

const glyphs = {
  "demo.terrain.floor": ".",
  "demo.terrain.wall": "#",
  "demo.actor.warrior-player": "@",
  "demo.actor.explorer": "@",
  "demo.actor.small-kobold": "k",
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
  assert.equal(ascii.mappings["demo.item.heavy-lance"]?.glyph, "/");
  assert.equal(image.mappings["demo.item.heavy-lance"]?.glyph, "/");
});

test("RFB 28px manifest keeps representative terrain, actor, and item mappings", () => {
  const image = parseTilesetManifest(
    readManifest("../public/tilesets/rfb-pixel-28/tileset.json"),
  );

  assert.equal(image.id, "rfb.tileset.pixel-28");
  assert.equal(image.labelKey, "tileset-rfb-pixel-28");
  assert.equal(image.mode, "image");
  assert.equal(image.tileWidth, 28);
  assert.equal(image.tileHeight, 28);
  for (const [id, tile] of [
    ["demo.terrain.floor", { x: 0, y: 0 }],
    ["demo.actor.newt", { x: 1, y: 2 }],
    ["demo.item.ration-of-food", { x: 2, y: 3 }],
  ]) {
    assert.deepEqual(image.mappings[id]?.tile, tile, id);
  }
  assert.equal(image.mappings["demo.item.heavy-lance"]?.glyph, "/");
});

test("tile coordinates must stay inside the declared atlas", () => {
  const image = readManifest("../public/tilesets/image-demo/tileset.json");
  for (const tile of [
    { x: -1, y: 0 },
    { x: image.atlas.columns, y: 0 },
    { x: 0, y: -1 },
    { x: 0, y: image.atlas.rows },
  ]) {
    image.mappings["demo.terrain.floor"].tile = tile;
    assert.throws(() => parseTilesetManifest(image), /tile [xy]/);
  }
});

test("standalone mapping images resolve independently from the main atlas", () => {
  const image = parseTilesetManifest(
    readManifest("../public/tilesets/rfb-pixel-28/tileset.json"),
  );
  const player = resolveTilesetVisual(image, "demo.actor.warrior-player", glyphs, true);
  const playerWithoutImage = resolveTilesetVisual(
    image,
    "demo.actor.warrior-player",
    glyphs,
    false,
  );

  assert.equal(player.source, "image");
  assert.equal(player.image, "players/warrior.png");
  assert.equal(player.background, undefined);
  assert.equal(player.usedFallback, false);
  assert.equal(playerWithoutImage.source, "glyph");
  assert.equal(playerWithoutImage.glyph, "@");
  assert.equal(playerWithoutImage.usedFallback, true);
});

test("missing image tiles fall back to the shared glyph path", () => {
  const image = parseTilesetManifest(readManifest("../public/tilesets/image-demo/tileset.json"));
  const floorImage = resolveTilesetVisual(image, "demo.terrain.floor", glyphs, true);
  const floorWithoutAtlas = resolveTilesetVisual(image, "demo.terrain.floor", glyphs, false);
  const monsterWithoutTile = resolveTilesetVisual(image, "demo.actor.small-kobold", glyphs, true);

  assert.equal(floorImage.source, "image");
  assert.deepEqual(floorImage.tile, { x: 0, y: 0 });
  assert.equal(floorWithoutAtlas.source, "glyph");
  assert.equal(floorWithoutAtlas.glyph, ".");
  assert.equal(floorWithoutAtlas.usedFallback, true);
  assert.equal(monsterWithoutTile.source, "glyph");
  assert.equal(monsterWithoutTile.glyph, "k");
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

  const unsafeImage = readManifest("../public/tilesets/rfb-pixel-28/tileset.json");
  unsafeImage.mappings["demo.actor.warrior-player"].image = "../player.png";
  assert.throws(() => parseTilesetManifest(unsafeImage), /safe relative path/);

  const conflictingImage = readManifest("../public/tilesets/rfb-pixel-28/tileset.json");
  conflictingImage.mappings["demo.actor.warrior-player"].tile = { x: 0, y: 2 };
  assert.throws(() => parseTilesetManifest(conflictingImage), /both tile and image/);
});
