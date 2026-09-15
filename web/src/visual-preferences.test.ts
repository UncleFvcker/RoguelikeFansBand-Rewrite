// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Node's built-in TypeScript test runner; no rendering context required.
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { defaultVisuals, validVisuals, visualStyle } from "./visual-preferences.ts";
import { parseTilesetManifest, resolveTilesetVisual } from "./tileset-manifest.ts";
import { defaultPreferences, parsePreferences, behaviorPreferences } from "./preferences.ts";
import { AppState } from "./app-state.ts";
import { commandShortcut } from "./command-shortcuts.ts";

test("visual preferences round-trip globally, reject invalid input, and stay outside behavior", () => {
  const p = defaultPreferences(), behavior = structuredClone(behaviorPreferences(p));
  p.visuals.overrides["demo.item.potion"] = { glyph: "药", foreground: "#ff0000" };
  assert.deepEqual(parsePreferences(JSON.stringify(p)), p);
  assert.deepEqual(behaviorPreferences(p), behavior);
  for (const glyph of ["", "ab", "\n", "\u0085", "\u202e", "\ud800", " "]) {
    const v = defaultVisuals(); v.overrides["demo.item.potion"] = { glyph };
    assert.equal(validVisuals(v), false);
  }
  for (const override of [{ foreground: "red" }, { background: "#00000000" }, { tile: [1, 2] }, {}]) {
    const v = defaultVisuals(); v.overrides["demo.item.potion"] = override;
    assert.equal(validVisuals(v), false);
  }
  assert.equal(validVisuals({ ...p.visuals, theme: { ...p.visuals.theme, memoryOpacity: 2 } }), false);
});

test("a saved true-kind override cannot distinguish an unknown appearance", () => {
  const state = new AppState();
  state.visuals.overrides["demo.item.secret-potion"] = { glyph: "秘", foreground: "#ff0000" };
  state.status = { player: { visualCatalog: [{ id: "core.appearance.blue-potion", glyph: "!" }] } };
  assert.equal(state.visualGlyph("demo.item.secret-potion", "!"), "!");
  const cell = { style: {} };
  state.paintVisual(cell, "core.appearance.blue-potion", "!");
  assert.equal(cell.textContent, "!");
  state.visuals.overrides["core.appearance.blue-potion"] = { glyph: "瓶", foreground: "#00ff00" };
  state.paintVisual(cell, "core.appearance.blue-potion", "!");
  assert.equal(cell.textContent, "瓶");
  assert.equal(cell.style.color, "#00ff00");
});

test("ASCII overrides and palette changes leave image pixels alone and apply to missing-image fallback", () => {
  const image = parseTilesetManifest(JSON.parse(readFileSync(new URL("../public/tilesets/rfb-pixel-28/tileset.json", import.meta.url), "utf8")));
  const id = "demo.actor.warrior-player", glyphs = { [id]: "@" }, p = defaultVisuals();
  p.overrides[id] = { glyph: "勇", foreground: "#ffffff" }; p.palette[1] = "#123456";
  const original = resolveTilesetVisual(image, id, glyphs, true);
  assert.equal(original.source, "image");
  assert.deepEqual(resolveTilesetVisual(image, id, glyphs, true, p, true), original);
  const fallback = resolveTilesetVisual(image, id, glyphs, false, p, true);
  assert.equal(fallback.glyph, "勇"); assert.equal(fallback.foreground, 0x123456);
  assert.equal(resolveTilesetVisual(image, id, glyphs, false, p, false).glyph, "@");
  delete p.overrides[id];
  assert.equal(visualStyle(p, id, { glyph: "@", foreground: "#ffffff" }, true).glyph, "@");
});

test("visual shortcuts work in every preset and respect modifier isolation", () => {
  for (const preset of ["original", "roguelike"]) {
    assert.equal(commandShortcut({ key: "%" }, preset), "glyphs");
    assert.equal(commandShortcut({ key: "&" }, preset), "colors");
    assert.equal(commandShortcut({ key: "%", ctrlKey: true }, preset), undefined);
  }
});
