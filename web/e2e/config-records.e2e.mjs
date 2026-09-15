// SPDX-License-Identifier: MPL-2.0
import { setPreferences } from "./preferences.mjs";
import assert from "node:assert/strict";
import { mkdtemp, readFile } from "node:fs/promises";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";

export async function checkConfigRecords({ keyboard, driver, click, hash, save, load, capture, checks, directory }) {
  const initial = await hash(), fresh = await save();
  const selector = "#config-records-dialog";
  const turn = () => driver.execute('return parseInt(document.querySelector("#turn-value").textContent,10)');
  async function open(key) {
    await driver.execute('document.activeElement.blur();return true;'); await keyboard.key(key);
    await driver.waitFor('return document.querySelector("#config-records-dialog").open', `open ${key}`);
  }
  const button = text => driver.execute('const button=[...document.querySelectorAll("#config-records-dialog button")].find(b=>b.textContent===arguments[0]);if(!button)throw new Error(arguments[0]);button.click();return true;', [text]);
  async function action(key) {
    const before = await hash(); await keyboard.key(key);
    await driver.waitFor('return document.querySelector("#hash-value").title!==arguments[0]', `action ${key}`, 15000, [before]);
  }
  async function preset(value) {
    await setPreferences(driver, { inputPreset: value });
  }
  await preset("roguelike");
  await open("@");
  await click("#cfg-capture-key"); await driver.execute('document.querySelector("#cfg-capture-key").focus();return true;'); await keyboard.key("F2");
  await driver.execute('document.querySelector("#cfg-key-action").value="i";return true;'); await click("#cfg-key-save");
  await driver.waitFor('return document.querySelector("#config-records-dialog").getAttribute("aria-busy")==="false"', "key saved");
  assert.match(await driver.execute('return window.__TAURI_INTERNALS__.invoke("load_preferences").then(saved=>JSON.stringify(saved.preferences.keyBindings))'), /F2/);
  await capture("config-bindings"); await keyboard.key("Escape");
  await keyboard.key("F2");
  assert.equal(await driver.execute('return document.querySelector("#player-page-dialog").dataset.page'), "inventory");
  await keyboard.key("Escape"); await keyboard.key("\\"); await keyboard.key("F2");
  assert.equal(await driver.execute('return document.querySelector("#player-page-dialog").open'), false);
  await preset("original"); await keyboard.key("F2");
  assert.equal(await driver.execute('return document.querySelector("#player-page-dialog").open'), false);
  await preset("roguelike"); assert.equal(await hash(), initial);
  checks.push("Custom key persistence, preset isolation and literal bypass work in the normal executable");

  await open("Enter");
  const beforeMenu = await turn(); await click('[data-command-key="5"]');
  await driver.waitFor('return parseInt(document.querySelector("#turn-value").textContent,10)===arguments[0]', "menu stay", 15000, [beforeMenu + 1]);
  await open('"'); await keyboard.key("a");
  assert.equal(await driver.execute('return document.querySelector("#command-record-status").hidden'), false);
  await action("5");
  assert.equal(await driver.execute('return document.querySelector("#command-record-status").hidden'), true);
  await open("'"); const beforeRegister = await turn(); await keyboard.key("a");
  await driver.waitFor('return parseInt(document.querySelector("#turn-value").textContent,10)===arguments[0]', "register replay", 15000, [beforeRegister + 1]);
  checks.push("Command menu executes a real action; double/single quote record and replay its selected register");

  await open("@");
  await driver.execute('const r=document.querySelector("#cfg-register");r.value="b";r.dispatchEvent(new Event("change",{bubbles:true}));return true;');
  await button("开始录制"); await action("#"); await action("5"); await click("#command-record-status");
  await open("@");
  assert.equal(await driver.execute('return document.querySelectorAll("#config-records-dialog ol li").length'), 2);
  await driver.execute('document.querySelector("#config-records-dialog ol li button:last-child").click();return true;');
  assert.equal(await driver.execute('return document.querySelectorAll("#config-records-dialog ol li").length'), 3);
  await driver.execute('document.querySelector("#config-records-dialog ol li button").click();return true;');
  assert.equal(await driver.execute('return document.querySelectorAll("#config-records-dialog ol li").length'), 2);
  await capture("config-macro");
  const beforeMacro = await turn(); await button("回放");
  await driver.waitFor('return parseInt(document.querySelector("#turn-value").textContent,10)===arguments[0] && document.querySelector("#stop-continuous-action").hidden', "heterogeneous macro completed", 15000, [beforeMacro + 1]);
  checks.push("Macro recording retains two heterogeneous actions, supports editing and replays both sequentially");

  const downloads = await mkdtemp(path.join(directory, "exports-"));
  await keyboard.downloadsTo(downloads);
  async function exported(name) {
    for (let i = 0; i < 100; i++) {
      try { return await readFile(path.join(downloads, name)); } catch (error) { if (error.code !== "ENOENT") throw error; }
      await delay(100);
    }
    throw new Error(`Export missing: ${name}`);
  }
  async function importBindings(text) {
    await driver.execute('const transfer=new DataTransfer();transfer.items.add(new File([arguments[0]],"keys.json",{type:"application/json"}));const input=document.querySelector("#cfg-key-import");input.files=transfer.files;input.dispatchEvent(new Event("change",{bubbles:true}));return true;', [text]);
    await driver.waitFor('const expected=arguments[0];return window.__TAURI_INTERNALS__.invoke("load_preferences").then(saved=>document.querySelector("#config-records-dialog").getAttribute("aria-busy")==="false" && JSON.stringify(saved.preferences.keyBindings)===JSON.stringify(JSON.parse(expected)))', "bindings imported", 15000, [text]);
  }
  await open("@"); await button("导出全部预设映射");
  const keyFile = (await exported("rfb-keybindings.json")).toString("utf8");
  assert.equal(JSON.parse(keyFile)[0].trigger, "F2");
  await button("清除此预设的自定义映射");
  await driver.waitFor('return document.querySelector("#config-records-dialog").getAttribute("aria-busy")==="false"', "keys reset");
  await importBindings(JSON.stringify([{ preset: "roguelike", trigger: "F2", action: "Register:b" }]));
  await keyboard.key("Escape");
  const beforeBoundMacro = await turn(); await keyboard.key("F2");
  await driver.waitFor('return parseInt(document.querySelector("#turn-value").textContent,10)===arguments[0] && document.querySelector("#stop-continuous-action").hidden', "bound macro completed", 15000, [beforeBoundMacro + 1]);
  await open("@"); await importBindings(keyFile); await keyboard.key("Escape");
  checks.push("Bindings export/import round-trip restores profiles, and a custom trigger plays a recorded macro");
  await open(":"); const noteHash = await hash();
  const note = '验收笔记 <script>throw new Error("must stay text")</script>';
  await driver.execute('document.querySelector("#cfg-note-text").value=arguments[0];return true;', [note]); await button("添加笔记");
  assert.ok(await driver.execute('return document.querySelector("#config-records-dialog").textContent.includes(arguments[0])', [note]));
  assert.equal(await driver.execute('return document.querySelectorAll("#config-records-dialog script").length'), 0);
  await button("导出笔记文本"); assert.ok((await exported("rfb-notes.txt")).toString("utf8").includes(note));
  await capture("config-notes"); await keyboard.key("Escape"); assert.equal(await hash(), noteHash);
  await open(")");
  assert.ok(await driver.execute('const image=document.querySelector("#config-records-dialog img"),canvas=document.querySelector("#map-host canvas");return image.naturalWidth>0 && image.naturalWidth<canvas.width'));
  await button("导出 PNG"); assert.deepEqual([...((await exported("rfb-map.png")).subarray(0, 8))], [137,80,78,71,13,10,26,10]);
  await button("导出文本"); assert.ok((await exported("rfb-map.txt")).toString("utf8").includes("@"));
  await button("导出 HTML"); const html = (await exported("rfb-map.html")).toString("utf8");
  assert.match(html, /<!doctype html>/); assert.match(html, /data:image\/png;base64,/); assert.doesNotMatch(html, /<script/);
  await capture("config-screen"); await keyboard.key("Escape"); assert.equal(await hash(), noteHash);
  checks.push(`Notes and PNG/text/HTML player exports produce real files without spending a turn (${path.basename(downloads)})`);

  await open("@"); await load(fresh, initial);
  assert.equal(await driver.execute('return document.querySelector("#config-records-dialog").open'), false);
  await keyboard.key("F2"); assert.equal(await driver.execute('return document.querySelector("#player-page-dialog").open'), true); await keyboard.key("Escape");
  await open("'"); await keyboard.key("a");
  assert.equal(await driver.execute('return document.querySelector("#config-records-dialog").open'), true);
  await keyboard.key("Escape"); assert.equal(await hash(), initial);
  await open(":"); assert.ok(await driver.execute('return document.querySelector("#config-records-dialog").textContent.includes(arguments[0])', [note])); await keyboard.key("Escape");
  checks.push("Native reload preserves bindings/notebook, clears session registers and closes stale editors");
}
