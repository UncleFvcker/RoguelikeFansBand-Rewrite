// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { execFile, spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
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
const directory = path.join(root, "test-results/ordinary-equipment");
await mkdir(directory, { recursive: true });
await mkdir(path.join(root, "target/e2e"), { recursive: true });
const profile = await mkdtemp(path.join(root, "target/e2e/ordinary-equipment-"));
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
  const save = () => invoke("save_game", { savedAt: "2026-09-12T16:00:00Z" });
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
  await driver.execute('localStorage.setItem("rfb.locale","zh-CN");return true;');
  await keyboard.reload();
  await driver.waitFor('return document.documentElement.lang==="zh-CN" && document.documentElement.dataset.appMode==="title" && !document.querySelector("#session-new-game").disabled', "Chinese title ready");
  await click("#session-new-game");
  await selectCreationRace(driver, "demo.race.rfb-human");
  await selectCreationBuild(driver, "demo.build.warrior");
  await driver.execute('for(const [id,value] of [["session-seed","511"],["session-character-name","常规装备验收"]]){const input=document.getElementById(id);input.value=value;input.dispatchEvent(new Event("input",{bubbles:true}));}return true;');
  await click("#session-start-game");
  await driver.waitFor('return document.documentElement.dataset.appMode==="playing" && document.querySelector("#connection-status").classList.contains("ready")', "fresh warrior", 30000);
  const input = path.join(directory, "new-game.rfbsave");
  await writeFile(input, Buffer.from(await save()));
  const preparation = await promisify(execFile)("cargo", ["test", "-p", "rfb-core", "--lib", "game::tests::death_scythe::export_ordinary_equipment_desktop_saves", "--", "--ignored", "--exact"], {
    cwd: root, env: { ...process.env, ORDINARY_EQUIPMENT_INPUT: input }, windowsHide: true, timeout: 240000,
  });
  await writeFile(path.join(directory, "preparation.log"), preparation.stdout + preparation.stderr);
  const scenarios = JSON.parse(await readFile(path.join(directory, "scenarios.json"), "utf8"));
  for (const scenario of scenarios) {
    await load(await readFile(path.join(directory, `${scenario.name}.rfbsave`)), scenario.initialHash);
    for (const [index, step] of scenario.steps.entries()) {
      const command = step.command;
      if (command.type === "equip") {
        await click("#player-ui-inventory-open");
        await click(`[data-item-id="${command.itemId}"] input[type="checkbox"]`);
        await click("#inventory-equip");
        if (command.slotId) {
          await driver.waitFor('return !!document.querySelector("dialog.item-target-dialog[open] select")', "equipment slot choice");
          await driver.execute('const dialog=document.querySelector("dialog.item-target-dialog[open]");const select=dialog.querySelector("select");select.value=arguments[0];select.dispatchEvent(new Event("change",{bubbles:true}));dialog.querySelector("button[type=submit]").click();return true;', [command.slotId]);
        }
        await readyHash(step.hash);
        await writeFile(path.join(directory, `${scenario.name}-${index}.png`), await keyboard.screenshot(), "base64");
        await click("#player-page-close");
      } else {
        if (command.type === "dig-terrain") { await keyboard.key("T", 8); await keyboard.key("8"); }
        else await keyboard.key(command.type === "move" ? "6" : "5");
        await readyHash(step.hash);
      }
      const bytes = await save();
      await load(bytes, step.hash);
      assert.equal(await hash(), step.hash);
      checks.push({ scenario: scenario.name, command, hash: step.hash, savedAndRestored: true });
    }
    await writeFile(path.join(directory, `${scenario.name}-result.png`), await keyboard.screenshot(), "base64");
    await writeFile(path.join(directory, `${scenario.name}-result.rfbsave`), Buffer.from(await save()));
  }
  assert.deepEqual(keyboard.errors, []);
  await writeFile(path.join(directory, "report.json"), JSON.stringify({
    executable, sha256: createHash("sha256").update(await readFile(executable)).digest("hex"),
    preparation: "Fresh level-one human Warrior, chosen talent, cleared monsters, prepared small floor area and adjacent target/magma, granted and identified seven representative bases. Scythe uses legal -255 hit enchantment, a temporary +2000 HP status and a seed selected for nonfatal backlash. Weapon and swimsuit targets start asleep. No natural acquisition or leveling claim.",
    checks, errors: keyboard.errors,
  }, null, 2) + "\n");
  process.stdout.write(`Ordinary Tauri standalone: ${checks.length} equipment/action and save/load checks passed.\n`);
} catch (error) {
  await writeFile(path.join(directory, "failure.json"), JSON.stringify({ error: String(error.stack ?? error), checks }, null, 2));
  if (keyboard) await writeFile(path.join(directory, "failure.png"), await keyboard.screenshot(), "base64");
  throw error;
} finally {
  keyboard?.close();
  if (child.exitCode === null && child.signalCode === null) child.kill();
  await writeFile(path.join(directory, "standalone.log"), logs.join(""));
}
