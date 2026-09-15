// SPDX-License-Identifier: MPL-2.0
import { setPreferences } from "./preferences.mjs";
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { setTimeout as delay } from "node:timers/promises";
import { connectKeyboard } from "./character-creation-layout.e2e.mjs";
import { selectCreationBuild, selectCreationRace } from "./character-creation.e2e.mjs";

// Ordinary standalone binary and production UI/load/save commands. Preparation
// uses a normal fresh character; no preparation or WebDriver-only IPC is enabled.
const root = fileURLToPath(new URL("../../", import.meta.url));
const executable = path.join(root, "target/debug/rfb-tauri.exe");
const directory = path.join(root, "test-results/character-ending");
await mkdir(directory, { recursive: true });
await mkdir(path.join(root, "target/e2e"), { recursive: true });
const profile = await mkdtemp(path.join(root, "target/e2e/character-ending-"));
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
    await driver.waitFor('const list=document.querySelector("#message-list");return list.children.length>arguments[0] && list.lastElementChild.textContent.includes("已恢复最新进度")', "native final checkpoint recovery rendered", 30000, [messages]);
    await readyHash(expected);
  }
  await driver.waitFor('return document.documentElement.dataset.appMode==="title"', "ordinary title", 30000);
  await setPreferences(driver, { locale: "zh-CN", inputPreset: "original" });
  await keyboard.reload();
  await driver.waitFor('return document.documentElement.lang==="zh-CN" && document.documentElement.dataset.appMode==="title" && !document.querySelector("#session-new-game").disabled', "Chinese title ready");
  const initialScores = await invoke("list_high_scores");
  await click("#title-high-scores");
  await driver.waitFor('return document.querySelector("#high-scores-dialog").open && !document.querySelector("#high-scores-dialog").textContent.includes("正在读取")', "title scores");
  await keyboard.key("Escape");
  async function createCharacter(name) {
    await click("#session-new-game");
    await selectCreationRace(driver, "demo.race.rfb-human");
    await selectCreationBuild(driver, "demo.build.warrior");
    await driver.execute('for(const [id,value] of [["session-seed","511"],["session-character-name",arguments[0]]]){const input=document.getElementById(id);input.value=value;input.dispatchEvent(new Event("input",{bubbles:true}));}return true;', [name]);
    await click("#session-start-game");
    await driver.waitFor('return document.documentElement.dataset.appMode==="playing" && document.querySelector("#connection-status").classList.contains("ready")', "fresh warrior", 30000);
    if (await driver.execute('return !!document.querySelector(".mutation-choice-candidate")')) {
      const before = await hash(); await click(".mutation-choice-candidate");
      await driver.waitFor('return document.querySelector("#hash-value").title!==arguments[0] && document.querySelector("#connection-status").classList.contains("ready")', "talent choice", 30000, [before]);
    }
    await driver.execute('document.activeElement.blur();return true;');
  }
  await createCharacter("成绩验收");
  const original = await save(), before = await hash();
  await keyboard.withDialog(false, () => keyboard.key("Q"));
  assert.equal(await hash(), before);
  await keyboard.withDialog([{ accept: true }, { accept: false }], () => keyboard.key("Q"));
  assert.equal(await hash(), before);
  await keyboard.withDialog([{ accept: true }, { accept: true, promptText: "wrong" }], () => keyboard.key("Q"));
  assert.equal(await hash(), before);
  assert.equal((await invoke("list_high_scores")).length, initialScores.length);
  checks.push({ name: "native confirmation cancel, prompt cancel and wrong token preserve character", passed: true });
  await keyboard.withDialog([{ accept: true }, { accept: true, promptText: "@" }], () => keyboard.key("Q"));
  await driver.waitFor('return document.documentElement.dataset.journeyResult==="abandoned"', "abandoned result", 30000);
  const finalHash = await hash();
  assert.ok((await driver.execute('return document.querySelector("#result-detail").textContent')).includes("你放弃了冒险"));
  const scores = await invoke("list_high_scores");
  assert.equal(scores.length, initialScores.length + 1);
  const added = scores.find(row => !initialScores.some(previous => previous.characterId === row.characterId));
  assert.equal(added.name, "成绩验收"); assert.equal(added.outcome, "abandoned");
  assert.equal(added.score, Number(await driver.execute('return document.querySelector("#result-score").textContent')));
  checks.push({ name: "Q ends role, shows final result and commits authoritative score", passed: true });
  await click("#result-high-scores");
  await driver.waitFor('return !!document.querySelector("#high-scores-dialog tbody tr[data-character-id="+JSON.stringify(arguments[0])+"]")', "final score row", 15000, [added.characterId]);
  await writeFile(path.join(directory, "high-scores.png"), Buffer.from(await keyboard.screenshot(), "base64"));
  await keyboard.viewport(390, 844);
  await writeFile(path.join(directory, "high-scores-narrow.png"), Buffer.from(await keyboard.screenshot(), "base64"));
  assert.equal(await driver.execute('return document.documentElement.scrollWidth <= innerWidth'), true);
  assert.equal(await driver.execute('const table=document.querySelector("#high-scores-dialog table");return table.scrollWidth>table.parentElement.clientWidth && getComputedStyle(table).whiteSpace==="nowrap"'), true);
  await keyboard.viewport(1280, 720); await keyboard.key("Escape");
  await load(original, finalHash);
  assert.equal((await invoke("list_high_scores")).length, scores.length);
  await keyboard.key("5"); assert.equal(await hash(), finalHash);
  checks.push({ name: "old active save recovers final checkpoint, cannot resume or duplicate score", passed: true });
  await click("#result-menu");
  await driver.waitFor('return document.documentElement.dataset.appMode==="title"', "back to title");
  await keyboard.reload();
  await driver.waitFor('return document.documentElement.dataset.appMode==="title"', "reload title");
  assert.equal((await invoke("list_high_scores")).length, scores.length);
  await createCharacter("成绩验收");
  await keyboard.key("~");
  await driver.waitFor('return document.querySelector("#help-knowledge-dialog").open', "knowledge menu");
  await keyboard.key("H");
  await driver.waitFor('return document.querySelector("#high-scores-dialog").open', "knowledge scores");
  await keyboard.key("Escape");
  await driver.execute('document.activeElement.blur();return true;');
  await keyboard.withDialog([{ accept: true }, { accept: true, promptText: "@" }], () => keyboard.key("Q"));
  await driver.waitFor('return document.documentElement.dataset.journeyResult==="abandoned"', "second result", 30000);
  const finalScores = await invoke("list_high_scores");
  assert.equal(finalScores.length, scores.length + 1);
  checks.push({ name: "scores persist through reload; same-name, same-seed new role gets a separate score; ~ H opens board", passed: true });
  assert.deepEqual(keyboard.errors, []);
  await writeFile(path.join(directory, "report.json"), JSON.stringify({ status: "passed", executable, executableSha256: createHash("sha256").update(await readFile(executable)).digest("hex"), checks,
    preparation: "Two fresh human warriors via production creation; no prepared game saves or debug IPC. Existing profile scores retained.", endedCharacterIds: finalScores.filter(row => !initialScores.some(previous => previous.characterId === row.characterId)).map(row => row.characterId) }, null, 2));
} catch (error) {
  await writeFile(path.join(directory, "failure.json"), JSON.stringify({ error: String(error.stack || error), checks, logs, errors: keyboard?.errors }, null, 2));
  if (keyboard) await writeFile(path.join(directory, "failure.png"), Buffer.from(await keyboard.screenshot(), "base64")).catch(() => {});
  throw error;
} finally {
  keyboard?.close(); child.kill();
}
