// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import {
  TILESET_MANIFESTS,
  inputPresetMessageKey,
  isInputPreset,
  readLocale,
} from "./settings-panel.ts";

test("settings preserve input preset validation and localized labels", () => {
  assert.equal(isInputPreset("numpad"), true);
  assert.equal(isInputPreset("vi"), true);
  assert.equal(isInputPreset("wasd"), true);
  assert.equal(isInputPreset("arrows"), false);
  assert.equal(inputPresetMessageKey("vi"), "input-preset-vi");
});

test("settings locale persistence keeps supported values and defaults safely", () => {
  assert.equal(readLocale({ getItem: () => "en-US" }), "en-US");
  assert.equal(readLocale({ getItem: () => "invalid" }), "zh-CN");
  assert.equal(readLocale({ getItem: () => null }), "zh-CN");
});

test("image preset selects the RFB 28px manifest", () => {
  assert.equal(TILESET_MANIFESTS.ascii, "/tilesets/ascii-default/tileset.json");
  assert.equal(TILESET_MANIFESTS.image, "/tilesets/rfb-pixel-28/tileset.json");
});
