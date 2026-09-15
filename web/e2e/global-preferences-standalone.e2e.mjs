// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdir, mkdtemp, readFile, writeFile, rename, rm, copyFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { setTimeout as delay } from "node:timers/promises";
import { connectKeyboard } from "./character-creation-layout.e2e.mjs";
import { selectCreationBuild, selectCreationRace } from "./character-creation.e2e.mjs";
import { setPreferences } from "./preferences.mjs";
import { runNativeSaves } from "./native-saves.e2e.mjs";

const root = fileURLToPath(new URL("../../", import.meta.url));
const builtExecutable = process.env.RFB_STANDALONE_EXE ?? path.join(root, "target/debug/rfb-tauri.exe");
const nativeSaves = process.argv.includes("--native-saves");
const terrainColors = process.argv.includes("--terrain-colors");
const directory = path.join(root, nativeSaves ? "test-results/native-saves" : terrainColors ? "test-results/terrain-colors" : "test-results/global-preferences");
const preferencesFile = path.join(process.env.LOCALAPPDATA, "io.github.unclefvcker.rfb-rewrite/preferences.json");
await mkdir(directory, { recursive: true });
await rm(path.join(directory, "report.json"), { force: true });
await mkdir(path.join(root, "target/e2e"), { recursive: true });
let installDirectory = await mkdtemp(path.join(root, "target/e2e/portable-game-"));
let executable = path.join(installDirectory, "rfb-tauri.exe");
await copyFile(builtExecutable, executable);
let original;
try { original = await readFile(preferencesFile); }
catch (error) { if (error.code !== "ENOENT") throw error; }
if (original) await writeFile(path.join(directory, "preferences-before.json"), original);
// Run with no other game process. Restore the exact original preference bytes in finally.
await rm(preferencesFile, { force: true });
let profile;
let child, keyboard, driver;
const checks = [], logs = [], runtimeErrors = [];
async function launch() {
  profile = await mkdtemp(path.join(root, "target/e2e/global-preferences-"));
  child = spawn(executable, ["--edge-webview-switches=--remote-debugging-port=0"], {
    cwd: root, windowsHide: true, stdio: ["ignore", "pipe", "pipe"],
    env: { ...process.env, WEBVIEW2_USER_DATA_FOLDER: profile },
  });
  let launchError;
  child.on("error", error => { launchError = error; });
  child.stdout.on("data", data => logs.push(String(data))); child.stderr.on("data", data => logs.push(String(data)));
  for (let attempt = 0; attempt < 150; attempt++) {
    if (launchError) throw launchError;
    assert.equal(child.exitCode, null);
    try {
      const port = (await readFile(path.join(profile, "EBWebView/DevToolsActivePort"), "utf8")).split("\n")[0];
      if ((await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).some(p => p.type === "page" && p.url.includes("tauri.localhost"))) break;
    } catch (error) { if (! ["ENOENT", "EBUSY"].includes(error.code) && error.cause?.code !== "ECONNREFUSED") throw error; }
    await delay(200);
  }
  keyboard = await connectKeyboard(profile);
  await keyboard.downloadsTo(directory);
  driver = {
    async execute(body, args = []) {
      try { return await keyboard.evaluate(`(function(){${body}}).apply(null,${JSON.stringify(args)})`); }
      catch (error) { throw new Error(`${body} ${JSON.stringify(args)}`, { cause: error }); }
    },
    async waitFor(body, label, timeout = 30000, args = []) {
      const start = Date.now();
      while (Date.now() - start < timeout) { if (await this.execute(body, args)) return; await delay(100); }
      throw new Error(`Timed out: ${label}`);
    },
  };
  await driver.waitFor('return document.documentElement?.dataset.appMode === "title" && document.querySelector("#session-new-game")?.disabled === false', "title ready");
  await keyboard.evaluate('window.__preferenceRejections = []; window.addEventListener("unhandledrejection", event => window.__preferenceRejections.push(String(event.reason))); true;');
}
async function stop(clean = false) {
  if (child && child.exitCode === null && child.signalCode === null) {
    const exited = new Promise(resolve => child.once("exit", resolve));
    if (clean) {
      assert.deepEqual(await driver.execute("return window.__preferenceRejections"), []);
      if (await driver.execute('return document.documentElement.dataset.appMode === "playing"')) {
        await click("#save-button");
        await driver.waitFor('return !document.querySelector("#save-button").disabled', "save before clean exit");
      }
      await driver.execute('document.querySelector("#session-exit").click(); return true;');
      let timeout;
      try { await Promise.race([exited, new Promise((_, reject) => { timeout = setTimeout(() => reject(new Error("Normal application exit timed out")), 10000); })]); }
      finally { clearTimeout(timeout); }
      assert.equal(child.exitCode, 0);
    } else { child.kill(); await exited; }
  }
  if (keyboard) runtimeErrors.push(...keyboard.errors);
  keyboard?.close(); keyboard = undefined;
}

const click = selector => driver.execute('document.querySelector(arguments[0]).click(); return true;', [selector]);
const edit = (selector, value, type = "change") => driver.execute('const e=document.querySelector(arguments[0]); e[typeof arguments[1] === "boolean" ? "checked" : "value"]=arguments[1]; e.dispatchEvent(new Event(arguments[2],{bubbles:true})); return true;', [selector, value, type]);
const invoke = (command, args = {}) => keyboard.evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})`);
const snapshot = () => invoke("refresh_museum");
const saved = () => invoke("load_preferences");
const capture = async name => writeFile(path.join(directory, `${name}.png`), await keyboard.screenshot(), "base64");
async function open(key = "=") {
  await driver.execute('document.activeElement?.blur(); return true;'); await keyboard.key(key);
  await driver.waitFor('return document.querySelector("#player-ui-settings-dialog").open && !document.querySelector("#preferences-save").disabled', "settings open");
}
async function saveDraft() {
  await click("#preferences-save");
  await driver.waitFor('return document.querySelector("#preferences-status").dataset.savedRevision && !document.querySelector("#preferences-save").disabled', "saved draft");
}
async function close() { await click("#player-ui-settings-close"); await driver.execute('document.activeElement?.blur(); return true;'); }
async function create(name, seed) {
  await click("#session-new-game"); await selectCreationRace(driver, "demo.race.rfb-human"); await selectCreationBuild(driver, "demo.build.warrior");
  await edit("#session-character-name", name, "input"); await edit("#session-seed", String(seed), "input");
  await click("#session-start-game");
  await driver.waitFor('return document.documentElement.dataset.appMode === "playing" && document.querySelector("#connection-status").classList.contains("ready")', "fresh character");
  if (await driver.execute('return document.querySelector("#player-page-dialog").open')) await click("#player-page-close");
}
try {
  await launch();
  await setPreferences(driver, { locale: "zh-CN", inputPreset: "original" });
  if (nativeSaves) {
    await runNativeSaves({ click, create, capture, snapshot, invoke, launch, stop, checks,
      keyboard: () => keyboard, driver: () => driver,
      installDirectory: () => installDirectory,
      setPreferences: values => setPreferences(driver, values),
      async waitExit() {
        for (let i = 0; i < 100 && child.exitCode === null; i++) await delay(100);
        assert.equal(child.exitCode, 0, "save-and-exit completes normally");
      },
      async moveInstall() {
        const destination = installDirectory + "-moved";
        const parent = path.resolve(root, "target/e2e") + path.sep;
        assert.ok(path.resolve(installDirectory).startsWith(parent) && path.resolve(destination).startsWith(parent));
        await rename(installDirectory, destination);
        installDirectory = destination; executable = path.join(destination, "rfb-tauri.exe");
      },
    });
  } else if (terrainColors) {
    await create("地形配色验收", 811);
    await keyboard.viewport(1600, 1000);
    await driver.waitFor('return document.querySelector("#map-host").dataset.mapScale === "local"', "local map");
    await capture("local-ascii");
    const local = await snapshot();
    await driver.execute('document.activeElement?.blur(); return true;');
    await click("#traverse-stairs");
    await driver.waitFor('return document.querySelector("#map-host").dataset.mapScale === "world"', "world map");
    const world = await snapshot();
    const terrainIds = [...new Set(world.cells.map(cell => cell.terrainId))];
    const mapping = await keyboard.evaluate('fetch("/tilesets/ascii-default/tileset.json").then(r=>r.json()).then(m=>m.mappings)');
    for (const id of terrainIds) assert.ok(mapping[id], id);
    assert.ok(new Set(terrainIds.map(id => mapping[id].background)).size >= 8);
    await capture("world-ascii");
    const hash = world.stateHash;
    await setPreferences(driver, { tilesetPreset: "image" });
    await driver.waitFor('return document.querySelector("#map-host").dataset.tilesetId !== "rfb.tileset.ascii-default"', "image tileset loaded");
    assert.equal((await snapshot()).stateHash, hash, "changing tileset never changes game state");
    await capture("world-image-fallback");
    await writeFile(path.join(directory, "terrain-ids.json"), JSON.stringify({ local: [...new Set(local.cells.map(cell => cell.terrainId))], world: terrainIds }, null, 2));
    checks.push("Fresh local ASCII map and normal world entry", "World terrain has distinct backgrounds and explicit shipped colors", "Image preset uses colored world glyphs without changing game state");
  } else {
  await open("!");
  assert.equal(await driver.execute('return document.activeElement.id'), "preferences-command");
  await keyboard.text("Y:always_pickup!");
  assert.equal(await driver.execute('return document.querySelectorAll("dialog[open]").length'), 1);
  await edit("#preferences-command", "Y:always_pickup", "input"); await click("#preferences-prf-preview");
  await driver.waitFor('return !document.querySelector("#preferences-prf-accept").disabled', "PRF preview");
  assert.equal((await saved()).preferences.travel.alwaysPickup, false);
  await click("#preferences-prf-accept"); await saveDraft(); await close();
  checks.push("Title settings, literal text input, PRF preview then global commit");
  await stop(true); await launch();
  assert.equal((await saved()).preferences.travel.alwaysPickup, true);
  await create("偏好验收一", 811);
  const first = await snapshot(); assert.equal(first.travelOptions.alwaysPickup, true);
  checks.push("Process restart and fresh character inherit saved global behavior");
  await keyboard.key("i"); await driver.waitFor('return document.querySelector("#player-page-dialog").open', "inventory opens");
  const inventorySize = await driver.execute('const r=document.querySelector("#player-page-dialog").getBoundingClientRect(); return {width:r.width,height:r.height};');
  await click("#player-page-close"); await open();
  assert.deepEqual(await driver.execute('return [...document.querySelector("#input-preset").options].map(o=>o.value)'), ["original", "roguelike"]);
  const settingsSize = await driver.execute('const r=document.querySelector("#player-ui-settings-dialog").getBoundingClientRect(); return {width:r.width,height:r.height};');
  assert.ok(Math.abs(settingsSize.width-inventorySize.width)<3 && Math.abs(settingsSize.height-inventorySize.height)<3, "settings share the inventory footprint");
  for (const page of ["general", "behavior", "display"]) {
    await click(`#preferences-${page}`);
    assert.deepEqual(await driver.execute('return [...document.querySelectorAll("[data-settings-panel]")].filter(e=>!e.hidden).map(e=>e.dataset.settingsPanel)'), [page]);
    await capture(`settings-${page}`);
  }
  await close(); await keyboard.key("p");
  await driver.waitFor('return document.querySelector(".pet-menu-dialog")?.open', "pet menu opens");
  const petSize = await driver.execute('const r=document.querySelector(".pet-menu-dialog").getBoundingClientRect(); return {width:r.width,height:r.height};');
  assert.ok(Math.abs(petSize.width-inventorySize.width)<3 && Math.abs(petSize.height-inventorySize.height)<3, "pets share the inventory footprint");
  assert.equal(await driver.execute('return document.querySelectorAll(".pet-menu-content > section").length'), 3);
  await capture("pets-chinese"); await click(".pet-menu-dialog > header button");
  assert.equal(await driver.execute('return document.querySelector(".support-drawer").tagName'), "SECTION");
  assert.deepEqual(await driver.execute('return [...document.querySelector("#support-panel-host").children].map(e=>e.id)'), ["dungeon-info-panel"]);
  assert.equal(await driver.execute('return document.querySelectorAll("#summon-command-panel,#campaign-panel,#native-save-panel,#task-log-entry").length'), 0);
  checks.push("Two RFB presets, inventory-sized settings/pets, category navigation and always-visible dungeon-only footer");
  await open("%");
  await edit("#preferences-visual-editor > select", "marker");
  await edit('[data-visual-field="glyph"]', "人", "input"); await saveDraft();
  const visualSaved = await saved(); assert.ok(Object.values(visualSaved.preferences.visuals.overrides).some(v => v.glyph === "人"));
  assert.equal((await snapshot()).stateHash, first.stateHash, "visual-only edit does not dispatch behavior");
  await capture("glyph-chinese"); await close(); await capture("map-unicode");
  await open("&");
  await edit('#preferences-visual-editor input[type="color"]', "#112233", "input"); await saveDraft();
  assert.equal((await saved()).preferences.visuals.palette[0], "#112233");
  assert.equal((await snapshot()).stateHash, first.stateHash, "palette-only edit does not dispatch behavior");
  await capture("colors-chinese"); await close();
  await setPreferences(driver, { locale: "en-US", tilesetPreset: "image" });
  await open("!"); await keyboard.viewport(390, 844);
  const layout = await driver.execute('const d=document.querySelector("#player-ui-settings-dialog");return {scroll:d.scrollWidth,client:d.clientWidth,width:d.getBoundingClientRect().width,viewport:innerWidth,overflow:[...d.querySelectorAll("*")].filter(e=>e.getBoundingClientRect().right>d.getBoundingClientRect().right).map(e=>({tag:e.tagName,id:e.id,width:e.getBoundingClientRect().width}))};');
  assert.ok(layout.scroll <= layout.client && layout.width <= layout.viewport, JSON.stringify(layout));
  assert.doesNotMatch(await driver.execute('return document.querySelector("#player-ui-settings-dialog").textContent'), /\[(?:preferences|display|visual|prf)-/);
  await driver.execute('document.querySelector("#preferences-command").scrollIntoView({block:"center"}); return true;');
  await capture("english-narrow"); await close(); await keyboard.key("p");
  await driver.waitFor('return document.querySelector(".pet-menu-dialog")?.open', "narrow pet menu");
  assert.equal(await driver.execute('const d=document.querySelector(".pet-menu-dialog"); return d.scrollWidth<=d.clientWidth && d.getBoundingClientRect().width<=innerWidth;'), true);
  await capture("pets-english-narrow"); await click(".pet-menu-dialog > header button");
  await keyboard.viewport(1280, 820); await open("!");
  await edit("#preferences-command", "%:other.prf", "input"); await click("#preferences-prf-preview");
  await driver.waitFor('return !document.querySelector("#preferences-prf-preview").disabled', "blocked preview settled");
  assert.equal(await driver.execute('return document.querySelector("#preferences-prf-accept").disabled'), true);
  await close();
  checks.push("Glyph/color shortcuts, Unicode save, image mode, English 390px and blocked include");
  const backup = await saved();
  await rm(path.join(directory, "rfb-preferences.json"), { force: true });
  await rm(path.join(directory, "rfb-preferences.prf"), { force: true });
  await open("!"); await click("#preferences-export"); await click("#preferences-prf-export");
  for (let attempt = 0; attempt < 100; attempt++) {
    try {
      assert.deepEqual(JSON.parse(await readFile(path.join(directory, "rfb-preferences.json"), "utf8")), backup.preferences);
      assert.match(await readFile(path.join(directory, "rfb-preferences.prf"), "utf8"), /always_pickup/);
      break;
    } catch (error) { if (error.code !== "ENOENT" || attempt === 99) throw error; await delay(100); }
  }
  await click("#preferences-behavior"); await edit("#operation-defaultTarget", "nearest-enemy");
  await keyboard.withDialog(false, () => click("#player-ui-settings-close"));
  assert.equal(await driver.execute('return document.querySelector("#player-ui-settings-dialog").open'), true);
  await keyboard.withDialog(true, () => click("#player-ui-settings-close"));
  assert.deepEqual(await saved(), backup);
  await open("!");
  await driver.execute('const transfer=new DataTransfer(); transfer.items.add(new File([arguments[0]], "rfb-preferences.json", {type:"application/json"})); const input=document.querySelector("#preferences-import"); input.files=transfer.files; input.dispatchEvent(new Event("change",{bubbles:true})); return true;', [await readFile(path.join(directory, "rfb-preferences.json"), "utf8")]);
  await driver.waitFor('return document.querySelector("#preferences-status").textContent.includes("Import ready") && !document.querySelector("#preferences-save").disabled', "JSON import preview");
  assert.deepEqual(await saved(), backup); await saveDraft(); await close();
  assert.deepEqual((await saved()).preferences, backup.preferences);
  await open();
  assert.deepEqual(await driver.execute('return [...document.querySelectorAll("[data-settings-panel]")].filter(e=>!e.hidden).map(e=>e.dataset.settingsPanel)'), ["general"], "a previous visual editor never reopens on the general page");
  await close();
  checks.push("JSON file export/import roundtrip, PRF export and explicit draft discard/cancel");
  await keyboard.key("_"); await driver.waitFor('return document.querySelector("#mogaminator-dialog").open', "Mogaminator opens");
  await edit("#mogaminator-source", "unsaved draft", "input");
  const external = await saved(); external.preferences.mogaminator.enabled = true;
  external.preferences.mogaminator.enUsSource = "!items"; external.preferences.mogaminator.zhCnSource = "!物品";
  await invoke("save_preferences", { preferences: external.preferences, expectedRevision: external.revision });
  const beforeReload = await snapshot(); await click("#mogaminator-reload");
  await driver.waitFor('return !document.querySelector("#mogaminator-reload").disabled && document.querySelector("#mogaminator-diagnostics").textContent.includes("Reloaded")', "saved rule reload");
  assert.equal(await driver.execute('return document.querySelector("#mogaminator-source").value'), "unsaved draft");
  const afterReload = await snapshot(); assert.equal(afterReload.mogaminator.source, "!items");
  assert.equal(afterReload.turn, beforeReload.turn); assert.deepEqual(afterReload.inventory, beforeReload.inventory);
  await click("#mogaminator-close"); await driver.execute('document.activeElement?.blur(); return true;');
  await keyboard.key("$"); await driver.waitFor('return document.querySelector("#message-list").lastElementChild.textContent.includes("Reloaded")', "$ reload");
  checks.push("Shared $/editor saved-rule reload preserves draft, inventory and turn");
  const complete = await saved();
  await stop(true); await launch(); await create("Preference acceptance two", 812);
  const second = await snapshot(); assert.equal(second.travelOptions.alwaysPickup, true); assert.equal(second.mogaminator.source, "!items");
  assert.deepEqual((await saved()).preferences, complete.preferences);
  checks.push("Second fresh character inherits rules and complete saved visual preferences");
  }
  assert.deepEqual([...runtimeErrors, ...keyboard.errors], []);
  await stop(true);
  await rm(path.join(directory, "failure.json"), { force: true });
  await rm(path.join(directory, "failure.png"), { force: true });
  await writeFile(path.join(directory, "report.json"), JSON.stringify({ builtExecutable, executable, checks, errors: runtimeErrors, preparation: nativeSaves ? "Two normal fresh Warriors in an isolated copy of the ordinary EXE, UI save/load, installation directory move and deliberate primary corruption for recovery. Original preferences restored." : terrainColors ? "One normal fresh human Warrior, normal world-map entry, ASCII/image rendering. No preparation commands. Original preferences restored after the run." : "Two normal fresh human Warriors, production preference UI and ordinary native storage. No WebDriver preparation commands. Original preferences restored after the run." }, null, 2));
  process.stdout.write(`Global preferences standalone: ${checks.length} grouped checks passed.\n`);
} catch (error) {
  await writeFile(path.join(directory, "failure.json"), JSON.stringify({ error: String(error.stack ?? error), checks }, null, 2));
  if (keyboard) { try { await capture("failure"); } catch (captureError) { process.stderr.write(`Failure screenshot: ${captureError}\n`); } }
  throw error;
} finally {
  await stop(); await writeFile(path.join(directory, "standalone.log"), logs.join(""));
  if (original) { await writeFile(preferencesFile + ".o8-restore", original); await rename(preferencesFile + ".o8-restore", preferencesFile); }
  else await rm(preferencesFile, { force: true });
}
