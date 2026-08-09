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

test("RFB 28px manifest exposes the expanded atlas and standalone player art", () => {
  const image = parseTilesetManifest(
    readManifest("../public/tilesets/rfb-pixel-28/tileset.json"),
  );

  assert.equal(image.id, "rfb.tileset.pixel-28");
  assert.equal(image.labelKey, "tileset-rfb-pixel-28");
  assert.equal(image.mode, "image");
  assert.equal(image.tileWidth, 28);
  assert.equal(image.tileHeight, 28);
  assert.deepEqual(image.atlas, { source: "atlas.png", columns: 8, rows: 16 });
  assert.deepEqual(image.mappings["demo.terrain.floor"]?.tile, { x: 0, y: 0 });
  assert.deepEqual(image.mappings["demo.terrain.wall"]?.tile, { x: 1, y: 0 });
  assert.deepEqual(image.mappings["demo.terrain.permanent-wall"]?.tile, { x: 2, y: 0 });
  assert.deepEqual(image.mappings["demo.terrain.magma-vein"]?.tile, { x: 3, y: 0 });
  assert.deepEqual(image.mappings["demo.terrain.quartz-vein"]?.tile, { x: 4, y: 0 });
  assert.deepEqual(image.mappings["demo.terrain.surface-grass"]?.tile, { x: 5, y: 0 });
  assert.deepEqual(image.mappings["demo.terrain.surface-path"]?.tile, { x: 6, y: 0 });
  assert.deepEqual(image.mappings["demo.terrain.surface-rock"]?.tile, { x: 7, y: 0 });
  assert.deepEqual(image.mappings["demo.terrain.outpost-wall"]?.tile, { x: 0, y: 1 });
  assert.deepEqual(image.mappings["demo.terrain.stairs-down"]?.tile, { x: 7, y: 1 });
  assert.equal(image.mappings["demo.actor.warrior-player"]?.image, "players/warrior.png");
  assert.equal(image.mappings["demo.actor.warrior-player"]?.tile, undefined);
  assert.deepEqual(image.mappings["demo.actor.newt"]?.tile, { x: 1, y: 2 });
  assert.deepEqual(image.mappings["demo.actor.wild-cat"]?.tile, { x: 7, y: 2 });
  assert.deepEqual(image.mappings["demo.terrain.created-trap"]?.tile, { x: 0, y: 3 });
  assert.deepEqual(image.mappings["core.gold.gold"]?.tile, { x: 1, y: 3 });
  assert.deepEqual(image.mappings["demo.item.ration-of-food"]?.tile, { x: 2, y: 3 });
  assert.deepEqual(image.mappings["demo.item.wooden-torch"]?.tile, { x: 3, y: 3 });
  assert.deepEqual(image.mappings["demo.item.broad-sword"]?.tile, { x: 4, y: 3 });
  assert.deepEqual(image.mappings["demo.item.arrow"]?.tile, { x: 5, y: 3 });
  assert.deepEqual(image.mappings["demo.item.healing-potion"]?.tile, { x: 6, y: 3 });
  assert.deepEqual(image.mappings["demo.item.fur-cloak"]?.tile, { x: 7, y: 3 });
  assert.deepEqual(image.mappings["demo.terrain.surface-tree"]?.tile, { x: 0, y: 4 });
  assert.deepEqual(image.mappings["demo.terrain.warren-snare"]?.tile, { x: 6, y: 4 });
  assert.deepEqual(image.mappings["demo.terrain.thieves-hideout-entry-available"]?.tile, { x: 7, y: 4 });
  assert.deepEqual(image.mappings["demo.terrain.general-store-entrance"]?.tile, { x: 0, y: 5 });
  assert.deepEqual(image.mappings["demo.terrain.bookstore-entrance"]?.tile, { x: 5, y: 5 });
  assert.deepEqual(image.mappings["demo.terrain.magic-shop-entrance"]?.tile, { x: 5, y: 5 });
  assert.deepEqual(image.mappings["demo.terrain.home-entrance"]?.tile, { x: 7, y: 5 });
  assert.deepEqual(image.mappings["demo.actor.raven"]?.tile, { x: 0, y: 6 });
  assert.deepEqual(image.mappings["demo.actor.duck"]?.tile, { x: 1, y: 6 });
  assert.deepEqual(image.mappings["demo.actor.fruit-bat"]?.tile, { x: 2, y: 6 });
  assert.deepEqual(image.mappings["demo.actor.giant-white-centipede"]?.tile, { x: 3, y: 6 });
  assert.deepEqual(image.mappings["demo.actor.jackal"]?.tile, { x: 4, y: 6 });
  assert.deepEqual(image.mappings["demo.actor.rock-lizard"]?.tile, { x: 5, y: 6 });
  assert.deepEqual(image.mappings["demo.actor.blue-yeek"]?.tile, { x: 6, y: 6 });
  assert.deepEqual(image.mappings["demo.actor.giant-green-frog"]?.tile, { x: 7, y: 6 });
  assert.deepEqual(image.mappings["demo.item.brass-lantern"]?.tile, { x: 0, y: 7 });
  assert.deepEqual(image.mappings["demo.item.flask-of-oil"]?.tile, { x: 1, y: 7 });
  for (const id of ["demo.item.broken-dagger", "demo.item.broken-sword"]) {
    assert.deepEqual(image.mappings[id]?.tile, { x: 2, y: 7 });
  }
  assert.deepEqual(image.mappings["demo.item.club"]?.tile, { x: 3, y: 7 });
  assert.deepEqual(image.mappings["demo.item.dagger"]?.tile, { x: 4, y: 7 });
  for (const id of [
    "demo.item.seeking-scroll",
    "demo.item.appraisal-scroll",
    "demo.item.summoning-scroll",
    "demo.item.cartography-scroll",
    "demo.item.clamor-scroll",
    "demo.item.homeward-scroll",
    "demo.item.trapfinding-scroll",
    "demo.item.flicker-scroll",
    "demo.item.detect-invisible-scroll",
    "demo.item.benediction-scroll",
    "demo.item.door-stair-location-scroll",
    "demo.item.confusing-touch-scroll",
    "demo.item.satisfy-hunger-scroll",
    "demo.item.darkness-scroll",
    "demo.item.trap-creation-scroll",
  ]) {
    assert.deepEqual(image.mappings[id]?.tile, { x: 5, y: 7 });
  }
  for (const id of [
    "demo.item.filthy-rag",
    "demo.item.cloak",
    "demo.item.robe",
    "demo.item.padded-armour",
    "demo.item.cord-armour",
    "demo.item.paper-armour",
  ]) {
    assert.deepEqual(image.mappings[id]?.tile, { x: 6, y: 7 });
  }
  assert.deepEqual(image.mappings["demo.item.shovel"]?.tile, { x: 7, y: 7 });
  assert.deepEqual(image.mappings["demo.actor.freesia"]?.tile, { x: 0, y: 8 });
  assert.deepEqual(image.mappings["demo.actor.metallic-green-centipede"]?.tile, { x: 1, y: 8 });
  assert.deepEqual(image.mappings["demo.actor.salamander"]?.tile, { x: 2, y: 8 });
  assert.deepEqual(image.mappings["demo.actor.metallic-blue-centipede"]?.tile, { x: 3, y: 8 });
  assert.deepEqual(image.mappings["demo.actor.metallic-red-centipede"]?.tile, { x: 4, y: 8 });
  assert.deepEqual(image.mappings["demo.actor.cave-lizard"]?.tile, { x: 5, y: 8 });
  assert.deepEqual(image.mappings["demo.actor.giant-white-rat"]?.tile, { x: 6, y: 8 });
  assert.deepEqual(image.mappings["demo.actor.large-kobold"]?.tile, { x: 7, y: 8 });
  assert.deepEqual(image.mappings["demo.actor.giant-brown-bat"]?.tile, { x: 0, y: 9 });
  assert.deepEqual(image.mappings["demo.actor.rat-thing"]?.tile, { x: 1, y: 9 });
  assert.deepEqual(image.mappings["demo.actor.night-lizard"]?.tile, { x: 2, y: 9 });
  assert.deepEqual(image.mappings["demo.actor.brown-yeek"]?.tile, { x: 3, y: 9 });
  assert.deepEqual(image.mappings["demo.actor.giant-salamander"]?.tile, { x: 4, y: 9 });
  assert.deepEqual(image.mappings["demo.actor.giant-grey-rat"]?.tile, { x: 5, y: 9 });
  assert.deepEqual(image.mappings["demo.actor.skaven"]?.tile, { x: 6, y: 9 });
  assert.deepEqual(image.mappings["demo.actor.skaven-shaman"]?.tile, { x: 7, y: 9 });
  assert.deepEqual(image.mappings["demo.item.mace"]?.tile, { x: 0, y: 10 });
  for (const id of [
    "demo.item.cutlass",
    "demo.item.rapier",
    "demo.item.small-sword",
    "demo.item.short-sword",
  ]) {
    assert.deepEqual(image.mappings[id]?.tile, { x: 1, y: 10 });
  }
  assert.deepEqual(image.mappings["demo.item.whip"]?.tile, { x: 2, y: 10 });
  assert.deepEqual(image.mappings["demo.item.small-leather-shield"]?.tile, { x: 3, y: 10 });
  for (const id of ["demo.item.hard-leather-cap", "demo.item.knit-cap", "demo.item.pointy-hat"]) {
    assert.deepEqual(image.mappings[id]?.tile, { x: 4, y: 10 });
  }
  for (const id of ["demo.item.leather-gloves", "demo.item.set-of-studded-leather-gloves"]) {
    assert.deepEqual(image.mappings[id]?.tile, { x: 5, y: 10 });
  }
  for (const id of ["demo.item.soft-leather-boots", "demo.item.pair-of-hard-leather-boots"]) {
    assert.deepEqual(image.mappings[id]?.tile, { x: 6, y: 10 });
  }
  for (const id of [
    "demo.item.soft-leather-armour",
    "demo.item.soft-studded-leather",
    "demo.item.hard-leather-armour",
  ]) {
    assert.deepEqual(image.mappings[id]?.tile, { x: 7, y: 10 });
  }
  for (const id of ["demo.item.main-gauche", "demo.item.tanto"]) {
    assert.deepEqual(image.mappings[id]?.tile, { x: 4, y: 7 });
  }
  assert.deepEqual(image.mappings["demo.item.pick"]?.tile, { x: 7, y: 7 });
  for (const id of ["demo.item.boldness-potion", "demo.item.swiftstep-tonic"]) {
    assert.deepEqual(image.mappings[id]?.tile, { x: 0, y: 11 });
  }
  for (const id of [
    "demo.item.slowness-potion",
    "demo.item.frailty-tonic",
    "demo.item.veil-draught",
  ]) {
    assert.deepEqual(image.mappings[id]?.tile, { x: 1, y: 11 });
  }
  for (const id of ["demo.item.venom-draught", "demo.item.slime-mold-juice"]) {
    assert.deepEqual(image.mappings[id]?.tile, { x: 2, y: 11 });
  }
  for (const id of [
    "demo.item.temperate-tonic",
    "demo.item.vigor-potion",
    "demo.item.valor-tonic",
  ]) {
    assert.deepEqual(image.mappings[id]?.tile, { x: 3, y: 11 });
  }
  for (const id of [
    "demo.item.sleep-potion",
    "demo.item.clumsiness-potion",
    "demo.item.tsuyoshi-special",
  ]) {
    assert.deepEqual(image.mappings[id]?.tile, { x: 4, y: 11 });
  }
  for (const id of [
    "demo.item.slime-mold",
    "demo.item.blindness-mushroom",
    "demo.item.confusion-mushroom",
    "demo.item.paranoia-mushroom",
    "demo.item.poison-mushroom",
  ]) {
    assert.deepEqual(image.mappings[id]?.tile, { x: 5, y: 11 });
  }
  assert.deepEqual(image.mappings["demo.item.corpse-remains"]?.tile, { x: 6, y: 11 });
  assert.deepEqual(image.mappings["demo.item.skeleton-remains"]?.tile, { x: 7, y: 11 });
  assert.deepEqual(image.mappings["demo.item.light-healing-potion"]?.tile, { x: 6, y: 3 });
  assert.deepEqual(image.mappings["demo.terrain.outpost-fortification"]?.tile, { x: 0, y: 1 });
  for (const [id, x] of [
    ["demo.terrain.surface-waste", 0],
    ["demo.terrain.surface-swamp", 1],
    ["demo.terrain.surface-snow", 2],
    ["demo.terrain.surface-pack-ice", 3],
    ["demo.terrain.surface-mountain", 4],
    ["demo.terrain.surface-glacier", 5],
    ["demo.terrain.surface-lava-shallow", 6],
    ["demo.terrain.surface-lava-deep", 7],
  ]) {
    assert.deepEqual(image.mappings[id]?.tile, { x, y: 12 });
  }
  for (const [id, x] of [
    ["demo.actor.filthy-street-urchin", 0],
    ["demo.actor.agent-of-black-market", 1],
    ["demo.actor.novice-rogue", 2],
    ["demo.actor.scruffy-looking-hobbit", 3],
    ["demo.actor.nibelung", 4],
    ["demo.actor.bandit", 5],
    ["demo.actor.tax-collector", 6],
    ["demo.terrain.count-entrance", 7],
  ]) {
    assert.deepEqual(image.mappings[id]?.tile, { x, y: 13 });
  }
  for (const [id, x] of [
    ["demo.actor.floating-eye", 0],
    ["demo.actor.grip-farmer-maggots-dog", 1],
    ["demo.actor.wolf-farmer-maggots-dog", 2],
    ["demo.actor.fang-farmer-maggots-dog", 3],
    ["demo.actor.blubbering-icky-thing", 4],
    ["demo.actor.cave-spider", 5],
    ["demo.actor.clear-icky-thing", 6],
    ["demo.actor.giant-black-ant", 7],
  ]) {
    assert.deepEqual(image.mappings[id]?.tile, { x, y: 14 });
  }
  for (const [id, x] of [
    ["demo.actor.goomba", 0],
    ["demo.actor.large-yellow-snake", 1],
    ["demo.actor.shrieker-mushroom-patch", 2],
    ["demo.actor.slimy-worm-mass", 3],
    ["demo.actor.white-harpy", 4],
    ["demo.actor.yellow-jelly", 5],
    ["demo.actor.yellow-mushroom-patch", 6],
    ["demo.actor.giant-white-ant", 7],
  ]) {
    assert.deepEqual(image.mappings[id]?.tile, { x, y: 15 });
  }
  for (const id of ["demo.actor.crow", "demo.actor.crow-of-durthang"]) {
    assert.deepEqual(image.mappings[id]?.tile, { x: 0, y: 6 });
  }
  for (const id of [
    "demo.actor.creeping-copper-coins",
    "demo.actor.creeping-silver-coins",
    "demo.actor.creeping-gold-coins",
    "demo.actor.creeping-mithril-coins",
  ]) {
    assert.deepEqual(image.mappings[id]?.tile, { x: 1, y: 3 });
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

  const unsafeImage = readManifest("../public/tilesets/rfb-pixel-28/tileset.json");
  unsafeImage.mappings["demo.actor.warrior-player"].image = "../player.png";
  assert.throws(() => parseTilesetManifest(unsafeImage), /safe relative path/);

  const conflictingImage = readManifest("../public/tilesets/rfb-pixel-28/tileset.json");
  conflictingImage.mappings["demo.actor.warrior-player"].tile = { x: 0, y: 2 };
  assert.throws(() => parseTilesetManifest(conflictingImage), /both tile and image/);
});
