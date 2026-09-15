// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Node's built-in TypeScript runner.
import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { CommandRecording, MAX_MACRO_STEPS, keyToken, originalKey } from "./command-recording.ts";
import { parseBindings, mapText } from "./config-records.ts";
import { commandMenuEntries } from "./command-shortcuts.ts";
import { Localization } from "./localization.ts";
import { AppState } from "./app-state.ts";

test("command recording retains choices, preserves cancelled registers and bounds recordings", () => {
  const r = new CommandRecording();
  const command = { type: "use-item", itemId: "wand-1", target: { type: "position", position: { x: 2, y: 3 } } };
  r.start("a", true); r.observe(command, "Wand"); command.target.position.x = 99;
  assert.equal(r.registers.get("a")[0].command.target.position.x, 2);
  r.start("a", true); r.finish(); assert.equal(r.registers.get("a")[0].description, "Wand");
  r.start("b", false); r.observe({ type: "wait" }, "Wait");
  r.playing = true; r.observe({ type: "wait" }, "Playback"); r.playing = false;
  assert.equal(r.observe({ type: "run", direction: "east" }, "Run"), "unsupported");
  assert.equal(r.registers.get("b").length, 1);
  r.start("c", false);
  for (let i = 0; i < MAX_MACRO_STEPS - 1; i++) assert.equal(r.observe({ type: "wait" }, "Wait"), undefined);
  assert.equal(r.observe({ type: "wait" }, "Wait"), "full");
  assert.equal(r.registers.get("c").length, MAX_MACRO_STEPS);
  assert.equal(r.recording, undefined);
  r.reset(); assert.equal(r.registers.size, 0);
});

test("binding imports reject ambiguous/reserved actions and preserve preset isolation", () => {
  const bindings = [{ preset: "original", trigger: "F2", action: "i" }, { preset: "roguelike", trigger: "F2", action: "Register:a" }];
  assert.deepEqual(parseBindings(JSON.stringify(bindings)), bindings);
  for (const value of [null, {}, [...bindings, bindings[0]], [{ ...bindings[0], action: "eval" }],
    [{ ...bindings[0], trigger: "Escape" }], [{ ...bindings[0], trigger: "\\" }], [{ ...bindings[0], preset: "unknown" }]]) {
    assert.throws(() => parseBindings(JSON.stringify(value)));
  }
  assert.equal(keyToken({ key: "E", ctrlKey: true }), "Ctrl+e");
  assert.equal(keyToken({ key: "E", ctrlKey: true, shiftKey: true }), "Ctrl+Shift+e");
  assert.equal(keyToken({ key: " " }), "Space");
  assert.equal(keyToken({ key: "Escape" }), undefined);
  assert.equal(keyToken({ key: "ArrowRight", shiftKey: true }), "Shift+ArrowRight");
  assert.deepEqual(originalKey("Ctrl+e"), { key: "e", ctrlKey: true, shiftKey: false });
});

test("exported map text keeps unknown squares blank and uses projected identities", () => {
  const state = new AppState(); state.mapWidth = 3; state.mapHeight = 1;
  state.status = { player: { visualCatalog: [], position: { x: 0, y: 0 } }, items: [], entities: [{ position: { x: 1, y: 0 }, kindId: "fuzzy", glyph: "?" }] };
  state.replaceCells([0, 1, 2].map(x => ({ position: { x, y: 0 }, terrainId: "wall" })));
  state.contentGlyphs.set("wall", "#"); state.cellVisibility.set("0,0", "visible");
  assert.equal(mapText(state), "@? ");
  assert.equal(mapText(state, 1, 1), "@");
});

test("all executable menu actions have native labels in both languages", () => {
  const sources = Object.fromEntries(["en-US", "zh-CN"].map(locale => [locale,
    ["ui", "game", "content"].map(name => readFileSync(new URL(`../../locales/${locale}/${name}.ftl`, import.meta.url), "utf8"))]));
  for (const locale of ["en-US", "zh-CN"]) {
    const l = new Localization(locale, sources);
    for (const [, command] of commandMenuEntries) assert.ok(l.hasMessage(locale, `cfg-command-${command}`), command);
    assert.doesNotMatch(l.format("cfg-macro-help"), /\[cfg-/);
  }
});
