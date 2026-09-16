// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Node's built-in TypeScript test runner.
import assert from "node:assert/strict";
import test from "node:test";
import { cellAppearance } from "./cell-appearance.ts";
import { DEFAULT_THEME as theme } from "./visual-preferences.ts";
import { playerStepProgress, playerStepPosition } from "./player-motion.ts";

const cell = { index: 0, x: 0, y: 0, terrainId: "floor", visibility: "visible",
  light: { color: 0xffcc88, intensity: 1 } };
const remembered = { ...cell, visibility: "remembered" };

test("manual and continuous movement share their exact progress with fog and light", () => {
  for (const continuous of [false, true]) {
    const elapsed = 25;
    const progress = playerStepProgress(elapsed, continuous);
    const position = playerStepPosition({ x: 0, y: 0 }, { x: 1, y: 0 }, elapsed, continuous);
    const fading = cellAppearance(cell, remembered, theme, progress);
    assert.equal(position.x, progress);
    assert.equal(fading.fog.alpha, theme.memoryOpacity * position.x);
    assert.equal(fading.tint.alpha, 0.5 * theme.lightTintOpacity * (1 - position.x));
    assert.equal(fading.tint.color, cell.light.color);
    const dim = cellAppearance(cell, { ...cell, light: { ...cell.light, intensity: 0 } }, theme, progress);
    assert.equal(dim.darkness.alpha, theme.darknessOpacity * position.x);
  }
});

test("reveals start at the old mask and settle exactly on the original per-cell appearance", () => {
  const hidden = { ...cell, visibility: "hidden" };
  const start = cellAppearance(hidden, cell, theme, 0);
  assert.deepEqual(start.fog, { color: 0, alpha: 1 });
  assert.equal(start.tint.alpha, 0);
  assert.equal(start.actorAlpha, 0);
  assert.equal(start.itemAlpha, 0);
  const middle = cellAppearance(hidden, cell, theme, 0.5);
  assert.equal(middle.fog.alpha, 0.5);
  assert.equal(middle.actorAlpha, 0.5);
  const end = cellAppearance(hidden, cell, theme, 1);
  assert.deepEqual(end, cellAppearance(cell, cell, theme, 1));
  assert.deepEqual(end.tint, { color: cell.light.color, alpha: 0.5 * theme.lightTintOpacity });
  assert.equal(end.actorAlpha, 1);
  // Remembered items stay visible, while new actors are revealed with the step.
  const knownItem = { ...remembered, itemKindId: "item" };
  assert.equal(cellAppearance(knownItem, { ...cell, itemKindId: "item" }, theme, 0).itemAlpha, 1);
  const memory = cellAppearance(cell, remembered, theme, 1);
  assert.deepEqual(memory.fog, { color: 0x12213a, alpha: theme.memoryOpacity });
  assert.equal(memory.tint.alpha, 0);
  assert.equal(memory.darkness.alpha, 0);
});
