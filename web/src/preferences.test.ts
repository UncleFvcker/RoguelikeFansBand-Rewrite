// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Node's built-in TypeScript test runner.
import assert from "node:assert/strict";
import test from "node:test";
import { PreferencesClient, defaultPreferences, migratePreferences, parsePreferences, behaviorPreferences } from "./preferences.ts";
import { DEFAULT_HUD_DISPLAY, HUD_DISPLAY_FIELDS } from "./display-preferences.ts";

test("older global display preferences default HUD sections without masking invalid values", () => {
  const p = defaultPreferences();
  for (const field of HUD_DISPLAY_FIELDS) delete p.display[field];
  const loaded = parsePreferences(JSON.stringify(p));
  for (const field of HUD_DISPLAY_FIELDS) assert.equal(loaded.display[field], DEFAULT_HUD_DISPLAY[field]);
  p.display.showMessages = false;
  assert.equal(parsePreferences(JSON.stringify(p)).display.showMessages, false);
  p.display.showMessages = "false";
  assert.throws(() => parsePreferences(JSON.stringify(p)), /preferences-invalid/);
});

test("hotbar bindings round-trip globally, default only when absent and reject malformed slots", () => {
  const p = defaultPreferences();
  const behavior = behaviorPreferences(p);
  delete p.hotbar;
  assert.deepEqual(parsePreferences(JSON.stringify(p)).hotbar, Array(60).fill(null));
  p.hotbar = Array(60).fill(null);
  p.hotbar[59] = { type: "ability", id: "demo.ability.heal" };
  assert.deepEqual(parsePreferences(JSON.stringify(p)), p);
  assert.deepEqual(behaviorPreferences(p), behavior);
  for (const hotbar of [null, [], Array(59).fill(null), [{ type: "ability", id: "" }, ...Array(59).fill(null)]]) {
    assert.throws(() => parsePreferences(JSON.stringify({ ...p, hotbar })), /preferences-invalid/);
  }
});

test("display options round-trip globally and never enter behavior context", () => {
  const p = defaultPreferences(), before = structuredClone(behaviorPreferences(p));
  p.display.hpWarningPercent = 90; p.display.targetPath = true;
  assert.deepEqual(parsePreferences(JSON.stringify(p)), p);
  assert.deepEqual(behaviorPreferences(p), before);
  for (const display of [
    { ...p.display, hpWarningPercent: 95 }, { ...p.display, manaWarningPercent: -10 },
    { ...p.display, targetPath: "true" }, { ...p.display, resourceBars: false },
  ]) assert.throws(() => parsePreferences(JSON.stringify({ ...p, display })), /preferences-invalid/);
});

function memoryStorage() {
  let saved = null;
  return {
    defaults: async () => defaultPreferences(),
    load: async () => structuredClone(saved),
    save: async (preferences, expectedRevision) => {
      if ((saved?.revision ?? null) !== expectedRevision) throw new Error("preferences-stale");
      saved = { revision: (saved?.revision ?? 0) + 1, preferences: structuredClone(preferences) };
      return structuredClone(saved);
    },
  };
}

test("saved Mogaminator reload reads disk without replacing the active snapshot or draft revision", async () => {
  const storage = memoryStorage(), editor = new PreferencesClient(storage), other = new PreferencesClient(storage);
  await editor.load(); await other.load();
  const before = structuredClone(editor.snapshot);
  const next = structuredClone(other.snapshot.preferences);
  next.zoom = 2; next.mogaminator.enabled = true; next.mogaminator.enUsSource = "!items";
  await other.save(next, other.snapshot.revision);
  assert.deepEqual(await editor.savedMogaminator(), next.mogaminator);
  assert.deepEqual(editor.snapshot, before);
  await assert.rejects(editor.save(before.preferences, before.revision), /stale/);
  storage.load = async () => { throw new Error("preferences-corrupt"); };
  await assert.rejects(editor.savedMogaminator(), /corrupt/);
  assert.deepEqual(editor.snapshot, before);
});

test("migration preserves valid preferences and notes, reports invalid values, and runs only once", async () => {
  const data = new Map([["rfb.locale", "en-US"], ["rfb.zoom", "99"], ["rfb.player-notes.v1", "notes"],
    ["rfb.custom-keys.v1", JSON.stringify([{ preset: "original", trigger: "F2", action: "i" }])]]);
  const old = { getItem: key => data.get(key) ?? null, removeItem: key => data.delete(key) };
  const store = memoryStorage(), client = new PreferencesClient(store);
  await client.load(old);
  assert.equal(client.snapshot.preferences.locale, "en-US");
  assert.equal(client.snapshot.preferences.zoom, 1);
  assert.equal(client.snapshot.preferences.keyBindings[0].trigger, "F2");
  assert.equal(data.has("rfb.locale"), false);
  assert.equal(data.has("rfb.custom-keys.v1"), false);
  assert.equal(data.get("rfb.zoom"), "99");
  assert.equal(data.get("rfb.player-notes.v1"), "notes");
  assert.match(client.warnings[0], /rfb.zoom/);
  data.set("rfb.locale", "zh-CN");
  await client.load(old);
  assert.equal(client.snapshot.preferences.locale, "en-US");
});

test("failed first write leaves legacy values and client untouched; corruption never starts migration", async () => {
  let removed = false, reads = 0;
  const old = { getItem: () => { reads++; return "en-US"; }, removeItem: () => { removed = true; } };
  const client = new PreferencesClient({ defaults: async () => defaultPreferences(), load: async () => null, save: async () => { throw new Error("disk full"); } });
  await assert.rejects(client.load(old), /disk full/);
  assert.equal(removed, false); assert.equal(client.snapshot, undefined);
  reads = 0;
  const corrupt = new PreferencesClient({ defaults: async () => defaultPreferences(), load: async () => { throw new Error("preferences-corrupt"); }, save: async () => assert.fail("must not overwrite") });
  await assert.rejects(corrupt.load(old), /corrupt/);
  assert.equal(reads, 0);
});

test("stale windows retain their draft and cannot replace a later commit", async () => {
  const store = memoryStorage(), first = new PreferencesClient(store), second = new PreferencesClient(store);
  await first.load(); await second.load();
  const draft = { ...second.snapshot.preferences, locale: "en-US" };
  await first.save({ ...first.snapshot.preferences, zoom: 1.5 }, first.snapshot.revision);
  await assert.rejects(second.save(draft, second.snapshot.revision), /stale/);
  assert.equal(second.snapshot.preferences.zoom, 1);
  assert.equal(draft.locale, "en-US");
  await second.load();
  assert.equal(second.snapshot.preferences.zoom, 1.5);
  assert.equal(second.snapshot.preferences.locale, "zh-CN");
});

test("operation options round-trip with strict enum and nested boolean validation", () => {
  const p = defaultPreferences();
  p.operations.defaultTarget = "old-then-nearest";
  p.operations.autoRepeat = false;
  p.operations.runStops.knownTreasure = true;
  assert.deepEqual(parsePreferences(JSON.stringify(p)), p);
  for (const operations of [
    { ...p.operations, defaultTarget: "nearest-pet" },
    { ...p.operations, easyOpen: 1 },
    { ...p.operations, runStops: { ...p.operations.runStops, knownTreasure: "yes" } },
    { ...p.operations, runStops: { ...p.operations.runStops, hiddenTreasure: true } },
  ]) assert.throws(() => parsePreferences(JSON.stringify({ ...p, operations })), /preferences-invalid/);
});

test("retry after a failed first write still migrates the original values", async () => {
  const store = memoryStorage(); let fail = true;
  const client = new PreferencesClient({ ...store, save: (...args) => {
    if (fail) return Promise.reject(new Error("disk full"));
    return store.save(...args);
  } });
  const old = new Map([["rfb.locale", "en-US"]]);
  await assert.rejects(client.load({ getItem: key => old.get(key) ?? null, removeItem: key => old.delete(key) }));
  fail = false;
  await client.load();
  assert.equal(client.snapshot.preferences.locale, "en-US");
  assert.equal(old.size, 0);
});

test("JSON backup round trips keys and rejects unknown fields, reserved triggers and invalid versions", () => {
  const p = { ...defaultPreferences(), keyBindings: [{ preset: "roguelike", trigger: "F3", action: "Register:a" }] };
  assert.deepEqual(parsePreferences(JSON.stringify(p)), p);
  for (const invalid of [
    { ...p, formatVersion: 99 }, { ...p, characterId: "wrong scope" }, { ...p, zoom: "1" },
    { ...p, keyBindings: [{ preset: "original", trigger: "Escape", action: "i" }] },
    { ...p, keyBindings: [p.keyBindings[0], p.keyBindings[0]] },
    { ...p, keyBindings: [{ ...p.keyBindings[0], entityId: "not a key mapping" }] },
  ]) assert.throws(() => parsePreferences(JSON.stringify(invalid)));
  assert.equal(migratePreferences({ getItem: () => null }).preferences.locale, "zh-CN");
});


test("global preferences default to Original and reject the removed development presets", () => {
  const p = defaultPreferences();
  assert.equal(p.inputPreset, "original");
  for (const inputPreset of ["numpad", "vi", "wasd"])
    assert.throws(() => parsePreferences(JSON.stringify({ ...p, inputPreset })), /preferences-invalid/);
});
