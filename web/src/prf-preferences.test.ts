// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Node's built-in TypeScript test runner; execution deferred to O8.
import assert from "node:assert/strict";
import test from "node:test";
import { defaultPreferences } from "./preferences.ts";
import { parsePrf, exportPrf } from "./prf-preferences.ts";
import { visualOverride, visualStyle } from "./visual-preferences.ts";
import { commandShortcut } from "./command-shortcuts.ts";

const known = [
  { id: "demo.actor.orc", prf: "R:40", category: "monster", glyph: "o", nameKey: "orc" },
  { id: "demo.item.sword", prf: "K:23:5", category: "item", glyph: "|", nameKey: "sword" },
  { id: "demo.item.artifact", prf: "K:23:5", category: "item", glyph: "|", nameKey: "artifact" },
];
test("combined aliases preserve the other bit, apply in source order, and invert stops", () => {
  const p = defaultPreferences(); p.operations.defaultTarget = "old-target"; p.mogaminator.autoGetMode = "wanted";
  const result = parsePrf("Y:auto_target\nX:auto_get_ammo\nY:find_ignore_stairs\nY:find_ignore_doors\nX:find_ignore_doors\nX:no_mogaminator", p, []);
  assert.equal(result.blocked, false); assert.equal(result.needsSubset, false);
  assert.equal(result.preferences.operations.defaultTarget, "old-then-nearest");
  assert.equal(result.preferences.mogaminator.autoGetMode, "wanted");
  assert.equal(result.preferences.mogaminator.enabled, true);
  assert.deepEqual(result.preferences.operations.runStops, { stairs: false, openDoors: true, knownTreasure: false });
  assert.equal(p.operations.defaultTarget, "old-target");
  assert.equal(parsePrf("X:use_old_target", result.preferences, []).preferences.operations.defaultTarget, "nearest-enemy");
});
test("syntax, conditions and includes block the whole file while unsupported settings require subset consent", () => {
  const p = defaultPreferences();
  for (const input of ["?:[EQU $SYS win]", "%:other.prf", "Yalways_pickup", "V:16:0:1:2:3", "R:40:999:65", "C:0:x"]) {
    const result = parsePrf(`Y:always_pickup\n${input}`, p, known);
    assert.equal(result.blocked, true, input); assert.deepEqual(result.preferences, p);
  }
  const subset = parsePrf("\uFEFF# comment\r\nY:always_pickup\r\n Y:auto_target\r\nY:graph_visuals\r\nF:1:1:46", p, known);
  assert.equal(subset.blocked, false); assert.equal(subset.needsSubset, true);
  assert.equal(subset.preferences.travel.alwaysPickup, true);
  assert.equal(subset.preferences.operations.defaultTarget, "manual");
  assert.equal(subset.diagnostics[0].reason, "leading-space");
});
test("R/K resolve known source identities; shared K follows later discoveries without coloring appearances", () => {
  const p = defaultPreferences();
  const result = parsePrf("R:40:4:79\nK:23:5:0:47\nV:4:0:1:2:3", p, known.slice(0, 2));
  assert.equal(result.blocked, false); assert.equal(result.needsSubset, false);
  assert.deepEqual(visualOverride(result.preferences.visuals, known[2]), { foreground: "#000000", glyph: "/" });
  assert.deepEqual(parsePrf("R:0x28:04:0x4f\nK:23:5:0:057\nV:4:0:1:2:3", p, known.slice(0, 2)).preferences, result.preferences);
  const appearance = { ...known[2], id: "core.appearance.red", prf: null };
  assert.deepEqual(visualOverride(result.preferences.visuals, appearance), {});
  const before = structuredClone(result.preferences);
  assert.deepEqual(parsePrf("R:40:0:0", before, known).preferences, before);
  const missing = parsePrf("R:999:1:65", p, known);
  assert.equal(missing.needsSubset, true); assert.equal(missing.diagnostics[0].input, "R:999:1:65");
  assert.deepEqual(missing.preferences, p);
  assert.equal(visualStyle(result.preferences.visuals, known[0].id, { glyph: "o", foreground: "#ffffff" }, true).foreground, "#010203");
});
test("single-key mappings use each original preset without recursive custom mappings or switching preset", () => {
  const p = defaultPreferences();
  p.keyBindings = [{ preset: "original", trigger: "i", action: "e" }];
  const result = parsePrf("A:i\nC:0:x\nA:k\nC:1:x\nA:z\nC:1:v", p, []);
  assert.equal(result.blocked, false); assert.equal(result.needsSubset, false);
  assert.equal(result.preferences.inputPreset, "original");
  assert.deepEqual(result.preferences.keyBindings.slice(1), [
    { preset: "original", trigger: "x", action: "i" },
    { preset: "roguelike", trigger: "x", action: "8" },
    { preset: "roguelike", trigger: "v", action: "a" },
  ]);
  for (const input of ["A:oo\nC:0:x", "A:^M\nC:0:x", "A:.\nC:0:x", "A:i\nC:0:\\e", "A:i\nP:xx"])
    assert.equal(parsePrf(input, p, []).needsSubset, true, input);
});
test("exported subset round trips and explicitly lists settings needing JSON", () => {
  const p = parsePrf("Y:rogue_like_commands\nY:auto_target\nY:use_old_target\nY:auto_get_objects\nR:40:4:79\nK:23:5:0:47\nV:4:0:1:2:3\nA:z\nC:1:x", defaultPreferences(), known).preferences;
  const exported = exportPrf(p, known), restored = parsePrf(exported.text, defaultPreferences(), known);
  assert.equal(restored.blocked, false); assert.equal(restored.needsSubset, false);
  assert.deepEqual(restored.preferences, p);
  assert.ok(exported.omitted.includes("mogaminator.zhCnSource"));
  p.visuals.overrides["demo.actor.orc"] = { glyph: "地" };
  p.keyBindings.push({ preset: "roguelike", trigger: "F2", action: "i" });
  const omitted = exportPrf(p, known).omitted;
  assert.ok(omitted.includes("visuals.overrides.demo.actor.orc"));
  assert.ok(omitted.includes("keyBindings.roguelike.F2"));
});
test("advanced import and saved-rule reload are available under all input presets", () => {
  for (const preset of ["original", "roguelike"])
    for (const [key, command] of [["!", "advanced-preferences"], ["$", "reload-pickup-rules"]]) {
      assert.equal(commandShortcut({ key, ctrlKey: false, altKey: false, metaKey: false, shiftKey: true }, preset), command);
      assert.equal(commandShortcut({ key, ctrlKey: false, altKey: true, metaKey: false, shiftKey: true }, preset), undefined);
    }
});
