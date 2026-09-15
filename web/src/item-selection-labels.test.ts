// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.
import assert from "node:assert/strict";
import test from "node:test";
import { itemSelectionLabels, itemSelectionConfirmations } from "./item-selection-labels.ts";
import { commandShortcut, originalCommandKey } from "./command-shortcuts.ts";

test("labels scan inscriptions in order and displace then relabel conflicts", () => {
  assert.deepEqual(itemSelectionLabels(["中文 @1 @ma", "@ma @2", "@qb @mZ", "@m? @m0"], "m"), ["1", "a", "Z", "0"]);
  assert.deepEqual(itemSelectionLabels(["@3", "@3", "@ma"], "m"), ["b", "3", "a"]);
  assert.deepEqual(itemSelectionLabels(["@mb", undefined, "@ma"], "m"), ["b", "c", "a"]);
  assert.deepEqual(itemSelectionLabels(["@mA", "@ma", "@ma"], "m"), ["A", "b", "a"]);
  assert.deepEqual(itemSelectionLabels(["@ma", "@q2", "@", "@1"], "r"), ["a", "b", "c", "1"]);
  assert.deepEqual(itemSelectionLabels(["@ma", "@1"]), ["a", "1"]);
  assert.deepEqual(itemSelectionLabels(["@ma", "@1"], "m", true), ["a", "b"]);
  const crowded = itemSelectionLabels(Array.from({ length: 26 }, () => "@0"), "m");
  assert.equal(new Set(crowded).size, 26);
  assert.equal(crowded[25], "0");
  assert.equal(crowded[24], "y");
});

test("confirmation guards preserve grouped, repeated, wildcard and question-number syntax", () => {
  for (const [text, command, expected] of [
    ["保留", "q", 0], ["!sdk", "d", 1], ["!k!q", "q", 1], ["!q!*", "q", 2],
    ["!qq", "q", 2], ["!Q", "q", 0], ["!?123q!q", "q", 2], ["!q?123q", "q", 2],
    ["!?123!q", "q", 1], ["!?123", "q", 0], ["!q 中文q", "q", 1], ["!q中文q", "q", 1],
    ["!{", "{", 1], ["!*", undefined, 0], [null, "q", 0],
  ] as const) assert.equal(itemSelectionConfirmations(text, command), expected, String(text));
});

test("preset keys resolve to the same canonical inscription command", () => {
  for (const [original, rogue, expected] of [["a", "z", "a"], ["z", "a", "z"], ["t", "T", "t"], ["f", "t", "f"]]) {
    for (const [key, preset] of [[original, "original"], [rogue, "roguelike"]] as const) {
      const shortcut = commandShortcut({ key: key!, ctrlKey: false, altKey: false, metaKey: false, shiftKey: false }, preset);
      assert.ok(shortcut);
      assert.equal(originalCommandKey(shortcut), expected);
    }
  }
});
