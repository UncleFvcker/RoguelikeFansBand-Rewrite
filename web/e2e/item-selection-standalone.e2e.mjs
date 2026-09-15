// SPDX-License-Identifier: MPL-2.0
import { setPreferences } from "./preferences.mjs";
import assert from "node:assert/strict";
import { execFile, spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { setTimeout as delay } from "node:timers/promises";
import { connectKeyboard } from "./character-creation-layout.e2e.mjs";
import { selectCreationBuild, selectCreationRace } from "./character-creation.e2e.mjs";

// Ordinary standalone binary and production UI/load/save commands. Preparation
// is confined to the ignored core exporter; no WebDriver-only IPC is enabled.
const root = fileURLToPath(new URL("../../", import.meta.url));
const executable = path.join(root, "target/debug/rfb-tauri.exe");
const directory = path.join(root, "test-results/item-selection");
await mkdir(directory, { recursive: true });
await mkdir(path.join(root, "target/e2e"), { recursive: true });
const profile = await mkdtemp(path.join(root, "target/e2e/item-selection-"));
const child = spawn(executable, ["--edge-webview-switches=--remote-debugging-port=0 --disable-gpu"], {
  cwd: root, windowsHide: true, stdio: ["ignore", "pipe", "pipe"],
  env: { ...process.env, WEBVIEW2_USER_DATA_FOLDER: profile },
});
const logs = [];
child.stdout.on("data", data => logs.push(String(data)));
child.stderr.on("data", data => logs.push(String(data)));
let launchError;
child.on("error", error => { launchError = error; });
let keyboard;
const checks = [];
try {
  for (let attempt = 0; attempt < 150; attempt++) {
    if (launchError) throw launchError;
    assert.equal(child.exitCode, null);
    let ready = false;
    try {
      const port = (await readFile(path.join(profile, "EBWebView/DevToolsActivePort"), "utf8")).split("\n")[0];
      ready = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).some(page => page.type === "page" && page.url.includes("tauri.localhost"));
    } catch (error) {
      if (error.code !== "ENOENT" && error.cause?.code !== "ECONNREFUSED") throw error;
    }
    if (ready) break;
    await delay(200);
  }
  keyboard = await connectKeyboard(profile);
  const driver = {
    execute: (body, args = []) => keyboard.evaluate(`(function(){${body}}).apply(null,${JSON.stringify(args)})`),
    async waitFor(body, label, timeout = 15000, args = []) {
      const start = Date.now();
      while (Date.now() - start < timeout) {
        if (await this.execute(body, args)) return;
        await delay(100);
      }
      throw new Error(`Timed out: ${label}`);
    },
  };
  const click = selector => driver.execute("document.querySelector(arguments[0]).click();return true;", [selector]);
  const hash = () => driver.execute('return document.querySelector("#hash-value").title');
  const readyHash = expected => driver.waitFor('return document.querySelector("#connection-status").classList.contains("ready") && document.querySelector("#hash-value").title===arguments[0]', "expected production hash", 30000, [expected]);
  const invoke = (command, args = {}) => keyboard.evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})`);
  const save = () => invoke("save_game", { savedAt: "2026-09-14T16:00:00Z" });
  async function load(bytes, expected) {
    const messages = await driver.execute('return document.querySelector("#message-list").children.length');
    const encoded = Buffer.from(bytes).toString("base64");
    await driver.execute('window.__ordinaryUpload="";return true;');
    for (let offset = 0; offset < encoded.length; offset += 524288) await driver.execute('window.__ordinaryUpload+=arguments[0];return true;', [encoded.slice(offset, offset + 524288)]);
    await driver.execute('const files=new DataTransfer();files.items.add(new File([Uint8Array.from(atob(window.__ordinaryUpload),c=>c.charCodeAt(0))],"ordinary.rfbsave"));const input=document.querySelector("#load-input");input.files=files.files;input.dispatchEvent(new Event("change",{bubbles:true}));return true;');
    await driver.waitFor('const list=document.querySelector("#message-list");return list.children.length>arguments[0] && list.lastElementChild.textContent.includes("存档校验与载入成功")', "native load rendered", 30000, [messages]);
    await readyHash(expected);
  }
  await driver.waitFor('return document.documentElement.dataset.appMode==="title"', "ordinary title", 30000);
  await setPreferences(driver, { locale: "zh-CN", inputPreset: "original" });
  await keyboard.reload();
  await driver.waitFor('return document.documentElement.lang==="zh-CN" && document.documentElement.dataset.appMode==="title" && !document.querySelector("#session-new-game").disabled', "Chinese title ready");
  await click("#session-new-game");
  await selectCreationRace(driver, "demo.race.rfb-human");
  await selectCreationBuild(driver, "demo.build.warrior");
  await driver.execute('for(const [id,value] of [["session-seed","511"],["session-character-name","物品选择验收"]]){const input=document.getElementById(id);input.value=value;input.dispatchEvent(new Event("input",{bubbles:true}));}return true;');
  await click("#session-start-game");
  await driver.waitFor('return document.documentElement.dataset.appMode==="playing" && document.querySelector("#connection-status").classList.contains("ready")', "fresh warrior", 30000);
  const input = path.join(directory, "new-game.rfbsave");
  await writeFile(input, Buffer.from(await save()));
  const preparation = await promisify(execFile)("cargo", ["test", "-p", "rfb-core", "--lib", "game::tests::item_selection_desktop::export_item_selection_desktop_save", "--", "--ignored", "--exact"], {
    cwd: root, env: { ...process.env, ITEM_SELECTION_INPUT: input }, windowsHide: true, timeout: 240000,
  });
  await writeFile(path.join(directory, "preparation.log"), preparation.stdout + preparation.stderr);
  const scenario = JSON.parse(await readFile(path.join(directory, "scenario.json"), "utf8"));
  await load(await readFile(path.join(directory, "prepared.rfbsave")), scenario.initialHash);

  const selector = ".item-selection-dialog[open]";
  const options = () => driver.execute('return [...document.querySelector(arguments[0]).querySelectorAll(".item-selection-row")].map(o=>({id:o.dataset.itemId,label:o.querySelector("kbd").textContent+") "+o.querySelector(".inventory-item-name").textContent,source:o.dataset.source}))', [selector]);
  const chooseId = async id => {
    const choice = (await options()).find(item => item.id === id);
    assert.ok(choice, "Visible candidate: " + id);
    await keyboard.key(choice.label[0]);
  };
  const open = async (key, prefix = []) => {
    if (await driver.execute('return document.querySelector("#player-page-dialog").open')) await click("#player-page-close");
    await driver.execute('document.activeElement.blur();return true;');
    for (const part of prefix) await keyboard.key(part);
    await keyboard.key(key, /^[A-Z]$/.test(key) ? 8 : 0);
    await driver.waitFor('return !!document.querySelector(arguments[0])', "item chooser", 15000, [selector]);
  };
  const capture = async name => writeFile(path.join(directory, name + ".png"), await keyboard.screenshot(), "base64");
  await open("I");
  assert.equal((await options()).length, 26);
  const overridden = (await options()).filter(item => item.label.startsWith("3)"));
  assert.equal(overridden.length, 1);
  assert.equal(overridden[0].id, "is.pack.01");
  await keyboard.key("PageDown");
  assert.ok((await options()).length > 0);
  await capture("pack-page-2");
  await keyboard.key("PageUp");
  await keyboard.key("@", 8);
  assert.equal((await options())[0].label[0], "a");
  const beforeInspect = await hash();
  await keyboard.key("A", 8);
  assert.equal(await driver.execute('return !document.querySelector(".item-selection-details").hidden'), true);
  assert.equal(await hash(), beforeInspect);
  await capture("inspect");
  await keyboard.viewport(390, 844);
  assert.equal(await driver.execute('const d=document.querySelector(".item-selection-dialog");return d.scrollWidth<=d.clientWidth && d.getBoundingClientRect().width<=innerWidth'), true);
  await capture("narrow");
  await keyboard.viewport(1280, 820);
  await driver.execute('document.querySelector(".item-selection-dialog").dispatchEvent(new KeyboardEvent("keydown",{key:"a",isComposing:true,bubbles:true}));return true;');
  assert.equal(await hash(), beforeInspect);
  await keyboard.key("Escape");
  assert.equal(await hash(), scenario.initialHash);
  checks.push({ action: "paging, collision, uppercase inspection, toggle, narrow layout and IME guard", unchanged: true });

  await open("u"); await chooseId("is.staff");
  await driver.waitFor('return document.querySelectorAll(".item-selection-dialog[open] .item-selection-row").length>1', "staff target");
  for (const [key, source] of [["e","equipment"],["q","quiver"],["f","floor"],["p","pack"]]) {
    await keyboard.key(key, 2);
    assert.ok((await options()).every(item => item.source === source));
  }
  await keyboard.key("\\");
  assert.equal((await options())[0].source, "floor");
  await keyboard.key("/");
  assert.equal((await options())[0].source, "pack");
  await capture("four-sources");
  await keyboard.key("Escape");
  await readyHash(scenario.steps[0].hash);
  checks.push({ action: "four sources and paid device cancellation", hash: scenario.steps[0].hash });
  await load(await save(), scenario.steps[0].hash);

  await open("q");
  await keyboard.withDialog(false, () => click(selector + ' button[type="submit"]'));
  assert.equal(await hash(), scenario.steps[0].hash);
  await keyboard.withDialog(false, () => keyboard.key("1"));
  assert.equal(await hash(), scenario.steps[0].hash);
  await keyboard.withDialog(true, () => keyboard.key("1"));
  await readyHash(scenario.steps[1].hash);
  checks.push({ action: "inscribed potion confirmation reject/accept", hash: scenario.steps[1].hash });
  await load(await save(), scenario.steps[1].hash);

  await setPreferences(driver, { inputPreset: "roguelike" });
  // Rogue u is movement: the literal prefix retains the canonical staff command.
  await open("u", ["\\"]); await chooseId("is.staff");
  await driver.waitFor('return document.querySelectorAll(".item-selection-dialog[open] .item-selection-row").length>1', "rogue staff target");
  await keyboard.key("-");
  await readyHash(scenario.steps[2].hash);
  checks.push({ action: "roguelike literal command and sole-floor target", hash: scenario.steps[2].hash });
  await load(await save(), scenario.steps[2].hash);

  await open("d", ["0", "2"]);
  let page = 0;
  while (!(await options()).some(item => item.id === "is.potion")) { assert.ok(page++ < 3); await keyboard.key("PageDown"); }
  await chooseId("is.potion");
  await readyHash(scenario.steps[3].hash);
  checks.push({ action: "count prefix resolves stack ID and quantity", hash: scenario.steps[3].hash });
  await click("#player-page-close");
  await driver.execute('document.activeElement.blur();return true;');
  await keyboard.key("X", 8);
  await readyHash(scenario.steps[4].hash);
  await keyboard.key("X", 8);
  await readyHash(scenario.steps[5].hash);
  checks.push({ action: "repeat uses resolved stack ID and quantity; moved ID cannot select another stack", hash: scenario.steps[5].hash });
  const finalSave = await save();
  await open("I");
  await load(finalSave, scenario.steps[5].hash);
  assert.equal(await driver.execute('return !!document.querySelector(".item-selection-dialog[open]")'), false);
  checks.push({ action: "native save/load closes old item selection", hash: scenario.steps[5].hash });
  if (await driver.execute('return document.querySelector("#player-page-dialog").open')) await click("#player-page-close");
  await driver.execute('document.activeElement.blur();return true;');
  for (const key of ["0", "9", "9", "9", "9"]) await keyboard.key(key);
  await keyboard.key("R", 8);
  await driver.waitFor('return !document.querySelector("#stop-continuous-action").hidden', "active counted rest");
  await keyboard.key("I", 8);
  await driver.waitFor('return document.querySelector("#stop-continuous-action").hidden && document.querySelector("#connection-status").classList.contains("ready")', "item shortcut stopped rest");
  assert.equal(await driver.execute('return !!document.querySelector(".item-selection-dialog[open]")'), false);
  const stoppedHash = await hash();
  await delay(250);
  assert.equal(await hash(), stoppedHash);
  await open("I");
  await keyboard.key("Escape");
  await click("#player-page-close");
  await load(await save(), stoppedHash);
  checks.push({ action: "item shortcut first interrupts continuous rest; second press opens selection", hash: stoppedHash });
  await setPreferences(driver, { locale: "en-US" });
  await driver.waitFor('return document.documentElement.lang==="en-US" && document.querySelector("#hash-value").title!==arguments[0]', "English locale applied by Core", 15000, [stoppedHash]);
  await open("I");
  const englishHash = await hash();
  await click('.item-selection-sources [data-source="equipment"]');
  assert.ok((await options()).every(item => item.source === "equipment"));
  assert.match(await driver.execute('return document.querySelector(".item-selection-sources").textContent'), /Equipment/);
  await keyboard.viewport(390, 844);
  assert.equal(await driver.execute('const d=document.querySelector(".item-selection-dialog");return d.scrollWidth<=d.clientWidth && d.getBoundingClientRect().width<=innerWidth'), true);
  await capture("english-narrow");
  await click(selector + ' button[type="submit"]');
  await driver.waitFor('return !document.querySelector(".item-selection-dialog[open]")', "mouse inspected equipment");
  assert.equal(await hash(), englishHash);
  checks.push({ action: "English narrow window, mouse source switch and inspection", unchanged: true });
  await capture("final");
  await writeFile(path.join(directory, "result.rfbsave"), Buffer.from(await save()));
  assert.deepEqual(keyboard.errors, []);
  await writeFile(path.join(directory, "report.json"), JSON.stringify({
    executable, sha256: createHash("sha256").update(await readFile(executable)).digest("hex"),
    preparation: scenario.preparation,
    checks, errors: keyboard.errors,
  }, null, 2) + "\n");
  await Promise.all(["failure.json", "failure.png"].map(name => rm(path.join(directory, name), { force: true })));
  process.stdout.write(`Ordinary Tauri standalone: ${checks.length} item selection and save/load checks passed.\n`);
} catch (error) {
  await writeFile(path.join(directory, "failure.json"), JSON.stringify({ error: String(error.stack ?? error), checks }, null, 2));
  if (keyboard) await writeFile(path.join(directory, "failure.png"), await keyboard.screenshot(), "base64");
  throw error;
} finally {
  keyboard?.close();
  if (child.exitCode === null && child.signalCode === null) child.kill();
  await writeFile(path.join(directory, "standalone.log"), logs.join(""));
}
