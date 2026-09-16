// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import {
  TILESET_MANIFESTS,
  inputPresetMessageKey,
  isInputPreset,
  SettingsPanel,
} from "./settings-panel.ts";
import { PreferencesClient, defaultPreferences } from "./preferences.ts";
import { ConfigRecords } from "./config-records.ts";

test("melee camera shake applies from global settings only after saving", async () => {
  const f = await fixture(); await f.panel.apply(); await f.panel.open("display");
  assert.equal(f.state.meleeCameraShake, true);
  const control = f.element("display-meleeCameraShake");
  assert.equal(control.checked, true);
  control.checked = false; control.dispatchEvent(new Event("change"));
  assert.equal(f.state.meleeCameraShake, true, "draft does not alter the renderer");
  f.click("preferences-save"); await f.idle();
  assert.equal(f.client.snapshot.preferences.display.meleeCameraShake, false);
  assert.equal(f.state.meleeCameraShake, false);
  assert.equal(f.behaviorCalls.length, 0, "camera effects never dispatch a Core command");
});

test("advanced shortcut opens the shared settings dialog and focuses the enabled command input", async () => {
  const f = await fixture(); await f.panel.open("advanced");
  assert.equal(f.element("player-ui-settings-dialog").open, true);
  assert.equal(f.element("preferences-command").disabled, false);
  assert.equal(f.element("preferences-command").focused, true);
});

test("PRF preview requires explicit supported-subset adoption then the normal global save", async () => {
  const f = await fixture(); await f.panel.open();
  const file = f.element("preferences-prf-file");
  file.files = [{ size: 100, arrayBuffer: async () => new TextEncoder().encode("Y:always_pickup\nY:graph_visuals").buffer }];
  file.dispatchEvent(new Event("change")); await f.idle();
  assert.equal(f.element("travel-always-pickup").checked, false);
  f.click("preferences-prf-accept"); await f.idle();
  assert.match(f.element("preferences-status").textContent, /prf-subset-required/);
  f.element("preferences-prf-subset").checked = true;
  f.click("preferences-prf-accept"); await f.idle();
  assert.equal(f.element("travel-always-pickup").checked, true);
  assert.equal(f.client.snapshot.preferences.travel.alwaysPickup, false);
  f.click("preferences-save"); await f.idle();
  assert.equal(f.client.snapshot.preferences.travel.alwaysPickup, true);
});

test("invalid UTF-8 and PRF context cannot apply an older preview; stale draft needs another preview", async () => {
  const f = await fixture(); await f.panel.open();
  f.change("preferences-command", "Y:always_pickup");
  f.click("preferences-prf-preview"); await f.idle();
  f.change("zoom-level", "1.5");
  f.click("preferences-prf-accept"); await f.idle();
  assert.match(f.element("preferences-status").textContent, /prf-preview-stale/);
  const file = f.element("preferences-prf-file");
  file.files = [{ size: 2, arrayBuffer: async () => new Uint8Array([0xff, 0xfe]).buffer }];
  file.dispatchEvent(new Event("change")); await f.idle();
  assert.equal(f.element("preferences-prf-accept").disabled, true);
  f.change("preferences-command", "%:other.prf"); f.click("preferences-prf-preview"); await f.idle();
  assert.equal(f.element("preferences-prf-accept").disabled, true);
  assert.equal(f.client.snapshot.preferences.travel.alwaysPickup, false);
});

test("visual edits share the global commit and never dispatch a behavior command", async () => {
  const f = await fixture(); await f.panel.apply(); f.state.mode = "playing";
  const p = structuredClone(f.client.snapshot.preferences);
  p.visuals.overrides["demo.terrain.floor"] = { glyph: "地" };
  p.visuals.theme.memoryOpacity = 0.7;
  await f.panel.commit(p, f.client.snapshot.revision);
  assert.deepEqual(f.state.visuals, p.visuals);
  assert.equal(f.behaviorCalls.length, 0);
});

test("display draft is inert until saved and applies without a Core command", async () => {
  const f = await fixture(); await f.panel.apply(); f.state.mode = "playing";
  await f.panel.open();
  f.element("display-highlightPlayer").checked = true;
  f.change("display-hpWarningPercent", "30");
  assert.equal(f.state.display.highlightPlayer, false);
  f.click("preferences-save"); await f.idle();
  assert.equal(f.state.display.highlightPlayer, true);
  assert.equal(f.state.display.hpWarningPercent, 30);
  assert.equal(f.client.snapshot.preferences.display.hpWarningPercent, 30);
  assert.equal(f.behaviorCalls.length, 0);
});

test("HUD quick toggles persist, preserve section choices and retain the active layout on write failure", async () => {
  const f = await fixture(); await f.panel.apply(); f.state.mode = "playing";
  await f.panel.open();
  f.element("display-showNearby").checked = false;
  f.element("display-showDungeonInfo").checked = false;
  f.click("preferences-save"); await f.idle(); f.panel.close();
  for (const area of ["header", "sidebar", "footer"]) {
    f.click("hud-toggle-" + area); await f.idle();
    assert.equal(f.element("app").dataset[area + "Expanded"], "false");
    assert.equal(f.element("hud-toggle-" + area).attributes.get("aria-expanded"), "false");
    f.click("hud-toggle-" + area); await f.idle();
    assert.equal(f.element("app").dataset[area + "Expanded"], "true");
  }
  assert.equal(f.client.snapshot.preferences.display.showNearby, false);
  assert.equal(f.client.snapshot.preferences.display.showDungeonInfo, false);
  const revision = f.client.snapshot.revision;
  f.failWrite(true); f.click("hud-toggle-sidebar"); await f.idle();
  assert.equal(f.client.snapshot.revision, revision);
  assert.equal(f.element("app").dataset.sidebarExpanded, "true");
  assert.equal(f.behaviorCalls.length, 0);
});

test("Mogaminator settings entry explains unavailable state and opens the shared editor in game", async () => {
  const f = await fixture(); await f.panel.open();
  f.click("preferences-autopick");
  assert.equal(f.element("preferences-mogaminator-unavailable").hidden, false);
  f.click("preferences-mogaminator");
  assert.equal(f.mogaminatorCalls.length, 0);
  f.panel.close(); f.state.mode = "playing"; f.state.status = { mogaminator: {} };
  await f.panel.open(); f.click("preferences-autopick");
  assert.equal(f.element("preferences-mogaminator-unavailable").hidden, true);
  f.click("preferences-mogaminator");
  assert.equal(f.element("player-ui-settings-dialog").open, false);
  assert.equal(f.mogaminatorCalls.length, 1);
});

test("settings offer only the two RFB input presets", () => {
  for (const preset of ["original", "roguelike"]) {
    assert.equal(isInputPreset(preset), true);
    assert.equal(inputPresetMessageKey(preset), `input-preset-${preset}`);
  }
  for (const preset of ["numpad", "vi", "wasd", "arrows"]) assert.equal(isInputPreset(preset), false);
});

test("image preset selects the RFB 28px manifest", () => {
  assert.equal(TILESET_MANIFESTS.ascii, "/tilesets/ascii-default/tileset.json");
  assert.equal(TILESET_MANIFESTS.image, "/tilesets/rfb-pixel-28/tileset.json");
});

test("operation controls share the global draft, cancellation and behavior commit", async () => {
  const f = await fixture(); await f.panel.apply();
  f.state.mode = "playing"; await f.panel.open();
  f.element("run-stop-knownTreasure").checked = true;
  f.element("operation-autoRepeat").checked = false;
  f.change("operation-defaultTarget", "manual");
  assert.equal(f.client.snapshot.preferences.operations.defaultTarget, "old-then-nearest");
  f.panel.close(); await f.panel.open();
  assert.equal(f.element("run-stop-knownTreasure").checked, false);
  f.element("run-stop-knownTreasure").checked = true;
  f.element("operation-autoRepeat").checked = false;
  f.change("operation-defaultTarget", "old-then-nearest");
  f.click("preferences-save"); await f.idle();
  assert.equal(f.client.snapshot.preferences.operations.defaultTarget, "old-then-nearest");
  assert.equal(f.client.snapshot.preferences.operations.autoRepeat, false);
  assert.equal(f.client.snapshot.preferences.operations.runStops.knownTreasure, true);
  assert.equal(f.client.snapshot.preferences.operations.runStops.stairs, true);
  assert.equal(f.behaviorCalls.length, 1);
});

test("title and game use one draft: preview is inert, cancel discards, save applies globally", async () => {
  const f = await fixture();
  await f.panel.open();
  assert.equal(f.element("preferences-mogaminator").disabled, true);
  f.change("input-preset", "roguelike");
  assert.equal(f.panel.inputPreset, "original");
  assert.equal(f.client.snapshot.revision, 1);
  assert.equal(f.panel.close(), true);
  f.state.mode = "playing";
  f.state.status = { mogaminator: {} };
  await f.panel.open();
  assert.equal(f.element("input-preset").value, "original");
  assert.equal(f.element("preferences-mogaminator").disabled, false);
  f.change("input-preset", "roguelike");
  f.click("preferences-save"); await f.idle();
  assert.equal(f.panel.inputPreset, "roguelike");
  assert.equal(f.client.snapshot.revision, 2);
  assert.equal(f.element("preferences-status").dataset.savedRevision, "2");
  assert.equal(f.applied.length, 1);
  f.panel.close(); f.state.mode = "title";
  await f.panel.open();
  assert.equal(f.element("input-preset").value, "roguelike");
});

test("failed saves keep the active configuration and draft; import/reset only preview", async () => {
  const f = await fixture(); await f.panel.open();
  f.change("zoom-level", "1.5"); f.failWrite(true);
  f.click("preferences-save"); await f.idle();
  assert.equal(f.panel.zoom, 1);
  assert.equal(f.element("zoom-level").value, "1.5");
  assert.equal(f.applied.length, 0);
  assert.match(f.element("preferences-status").textContent, /disk full/);
  assert.equal(f.element("preferences-save").disabled, false);
  f.failWrite(false); f.click("preferences-save"); await f.idle();
  const saved = structuredClone(f.client.snapshot);
  const backup = { ...defaultPreferences(), locale: "en-US", inputPreset: "original" };
  const input = f.element("preferences-import");
  input.files = [{ size: 500, text: async () => JSON.stringify(backup) }];
  input.dispatchEvent(new Event("change")); await f.idle();
  assert.equal(f.element("language-select").value, "en-US");
  assert.deepEqual(f.client.snapshot, saved);
  f.click("preferences-reset");
  assert.equal(f.element("language-select").value, "zh-CN");
  assert.equal(f.element("zoom-level").value, "1");
  assert.deepEqual(f.client.snapshot, saved);
  f.panel.close(); await f.panel.open();
  assert.equal(f.element("zoom-level").value, "1.5");
});

test("custom keys are editable without a character and consume the saved global mappings", async () => {
  const f = await fixture();
  f.state.commandBlocked = true;
  const records = new ConfigRecords({ document: f.document, state: f.state, localization: f.localization,
    preferences: f.client, preset: () => f.panel.inputPreset, message: () => assert.fail("unexpected error"),
  });
  records.open("input-config");
  const dialog = f.document.body.children.find(node => node.id === "config-records-dialog");
  assert.equal(dialog.open, true);
  assert.equal(records.hasBinding({ key: "F2" }), false);
  await f.client.save({ ...f.client.snapshot.preferences, keyBindings: [{ preset: "original", trigger: "F2", action: "i" }] }, f.client.snapshot.revision);
  assert.equal(records.hasBinding({ key: "F2" }), true);
  records.close(); records.open("command-menu");
  assert.equal(dialog.open, false);
});

test("travel edits stay in the draft and only behavior changes reach the active game", async () => {
  const f = await fixture(); await f.panel.apply();
  f.state.mode = "playing"; await f.panel.open();
  f.element("travel-auto-detect").checked = true;
  f.element("travel-auto-detect").dispatchEvent(new Event("change"));
  assert.equal(f.client.snapshot.preferences.travel.autoDetectTraps, false);
  assert.equal(f.behaviorCalls.length, 0);
  f.click("preferences-save"); await f.idle();
  assert.equal(f.client.snapshot.preferences.travel.autoDetectTraps, true);
  assert.equal(f.behaviorCalls.length, 1);
  f.change("zoom-level", "1.5"); f.click("preferences-save"); await f.idle();
  assert.equal(f.behaviorCalls.length, 1, "display-only save is not a Game command");
});

async function fixture() {
  class Element extends EventTarget {
    value = ""; textContent = ""; hidden = false; disabled = false; open = false;
    dataset = {}; children = []; attributes = new Map();
    append(...nodes) { this.children.push(...nodes); }
    replaceChildren(...nodes) { this.children = nodes; this.textContent = ""; }
    setAttribute(key, value) { this.attributes.set(key, value); }
    querySelectorAll() { return []; }
    showModal() { this.open = true; }
    close() { this.open = false; }
    focus() { if (!this.disabled) this.focused = true; }
    scrollIntoView() {}
  }
  const elements = new Map();
  const element = id => { if (!elements.has(id)) elements.set(id, new Element()); return elements.get(id); };
  const document = { body: new Element(), getElementById: element, createElement: () => new Element(), defaultView: Object.assign(new EventTarget(), { confirm: () => true }) };
  let saved = { revision: 1, preferences: defaultPreferences() }, fail = false;
  const client = new PreferencesClient({ defaults: async () => defaultPreferences(), load: async () => structuredClone(saved), save: async (preferences, revision) => {
    if (fail) throw new Error("disk full");
    assert.equal(revision, saved.revision);
    saved = { revision: revision + 1, preferences: structuredClone(preferences) }; return structuredClone(saved);
  } });
  await client.load();
  const state = { mode: "title" }, applied = [], behaviorCalls = [], mogaminatorCalls = [];
  const localization = { locale: "zh-CN", setLocale(locale) { this.locale = locale; }, localizeDocument() {}, format: (key, args) => key + (args ? JSON.stringify(args) : "") };
  const panel = new SettingsPanel({ document, state, preferences: client, localization,
    dom: { languageSelect: element("language-select"), inputPresetSelect: element("input-preset"), tilesetPresetSelect: element("tileset-preset"), cameraModeSelect: element("camera-mode"), zoomSelect: element("zoom-level"), controlsHelp: element("controls-help") },
    renderer: { setMeleeCameraShake(enabled) { state.meleeCameraShake = enabled; } }, rendererReady: () => false, beforeEdit: async () => {}, openKeys() {}, openMogaminator() { mogaminatorCalls.push(true); }, download() {},
    renderTargeting() {}, renderLocaleDependentUi: () => applied.push(client.snapshot.revision), async onBehaviorChange(p) { behaviorCalls.push(p); }, announce() {},
  });
  panel.initialize(); panel.install();
  return { panel, client, state, element, document, localization, applied, behaviorCalls, mogaminatorCalls, failWrite: value => { fail = value; },
    change: (id, value) => { element(id).value = value; element(id).dispatchEvent(new Event("change")); },
    click: id => element(id).dispatchEvent(new Event("click")), idle: () => new Promise(resolve => setImmediate(resolve)),
  };
}


test("blocked PRF preview with unchanged settings does not prompt to discard", async () => {
  const f = await fixture(); await f.panel.open("advanced");
  f.document.defaultView.confirm = () => assert.fail("no preference value changed");
  f.element("preferences-command").value = "%:other.prf";
  f.click("preferences-prf-preview"); await f.idle();
  assert.equal(f.element("preferences-prf-accept").disabled, true);
  assert.equal(f.panel.close(), true);
});
