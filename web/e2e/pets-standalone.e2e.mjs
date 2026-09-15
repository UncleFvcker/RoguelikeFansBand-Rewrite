// SPDX-License-Identifier: MPL-2.0
import { setPreferences } from "./preferences.mjs";
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
const directory = path.join(root, "test-results/pets");
await mkdir(directory, { recursive: true });
await mkdir(path.join(root, "target/e2e"), { recursive: true });
const profile = await mkdtemp(path.join(root, "target/e2e/pets-"));
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
  await driver.execute('for(const [id,value] of [["session-seed","511"],["session-character-name","宠物系统验收"]]){const input=document.getElementById(id);input.value=value;input.dispatchEvent(new Event("input",{bubbles:true}));}return true;');
  await click("#session-start-game");
  await driver.waitFor('return document.documentElement.dataset.appMode==="playing" && document.querySelector("#connection-status").classList.contains("ready")', "fresh warrior", 30000);
  const input = path.join(directory, "new-game.rfbsave");
  await writeFile(input, Buffer.from(await save()));
  const preparation = await promisify(execFile)("cargo", ["test", "-p", "rfb-core", "--lib", "game::tests::pet_commands::export_pet_desktop_save", "--", "--ignored", "--exact"], {
    cwd: root, env: { ...process.env, PET_DESKTOP_INPUT: input }, windowsHide: true, timeout: 240000,
  });
  await writeFile(path.join(directory, "preparation.log"), preparation.stdout + preparation.stderr);
  const scenario = JSON.parse(await readFile(path.join(directory, "scenario.json"), "utf8"));
  await load(await readFile(path.join(directory, "prepared.rfbsave")), scenario.initialHash);
  const menu = async () => {
    await keyboard.key("p");
    await driver.waitFor('return !!document.querySelector(".pet-menu-dialog[open]")', "pet keyboard menu");
  };
  const menuButton = text => driver.execute('const button=[...document.querySelectorAll(".pet-menu-dialog button")].find(button=>button.textContent===arguments[0]);if(!button||button.disabled)throw new Error("Unavailable pet button: "+arguments[0]);button.click();return true;', [text]);
  await menu();
  await driver.execute('document.querySelector(".pet-menu-dialog").scrollTop=0;return true;');
  await writeFile(path.join(directory, "pet-menu.png"), await keyboard.screenshot(), "base64");
  await keyboard.viewport(390, 844);
  await driver.waitFor('return innerWidth===390', "narrow viewport");
  assert.equal(await driver.execute('const d=document.querySelector(".pet-menu-dialog");return d.scrollWidth<=d.clientWidth && d.getBoundingClientRect().width<=innerWidth'), true);
  await writeFile(path.join(directory, "pet-menu-narrow.png"), await keyboard.screenshot(), "base64");
  await keyboard.viewport(1280, 820);
  await keyboard.key("Escape");
  await driver.waitFor('return !document.querySelector(".pet-menu-dialog")', "menu escape");
  assert.equal(await hash(), scenario.initialHash);
  checks.push({ action: "menu escape", unchanged: true });
  for (const [index, step] of scenario.steps.entries()) {
      const command = step.command;
      if (["set-pet-name", "set-pet-option", "set-pet-target", "dismiss-pet"].includes(command.type)) {
        await menu();
        if (command.type === "set-pet-name" || command.type === "dismiss-pet") {
          await driver.execute('const select=document.querySelectorAll(".pet-menu-dialog select")[1];select.value=arguments[0];select.dispatchEvent(new Event("change",{bubbles:true}));return true;', [command.actorId]);
        }
        if (command.type === "set-pet-name") {
          await driver.execute('document.querySelector(".pet-menu-dialog input").focus();return true;');
          await keyboard.text(command.name);
          await keyboard.key("Enter");
        } else if (command.type === "set-pet-target") {
          await driver.execute('const select=document.querySelector(".pet-menu-dialog select");select.value=arguments[0];return true;', [command.actorId]);
          await menuButton("指定宠物目标");
        } else if (command.type === "dismiss-pet") {
          const before = await hash();
          await keyboard.withDialog(false, () => menuButton("解散所选宠物"));
          assert.equal(await hash(), before);
          await keyboard.withDialog(true, () => menuButton("解散所选宠物"));
        } else {
          const labels = {
            "highlight-map": `在地图上高亮宠物 (目前：${command.enabled ? "关" : "开"})`,
            "open-doors": `宠物开门 (目前：${command.enabled ? "关" : "开"})`,
            "pickup-items": `宠物拾取物品 (目前：${command.enabled ? "关" : "开"})`,
            "no-breeding": `禁止繁殖 (目前：${command.enabled ? "关" : "开"})`,
            "attack-spells": `允许施放攻击法术 (当前: ${command.enabled ? "关闭" : "开启"})`,
            "summon-spells": `允许施放召唤法术 (当前: ${command.enabled ? "关闭" : "开启"})`,
            "riding-two-hands": command.enabled ? "双手握持武器" : "单手控制骑乘宠物",
          };
          await menuButton(labels[command.option]);
        }
      } else if (command.type === "wait") await keyboard.key("5");
      else if (command.type === "move") await keyboard.key("6");
      else if (command.type === "enter-world-map" || command.type === "leave-world-map") {
        await click("#traverse-stairs");
      } else throw new Error(`Unhandled pet command ${command.type}`);
      await readyHash(step.hash);
      if (["wait", "enter-world-map", "move", "leave-world-map", "dismiss-pet"].includes(command.type)) {
        await writeFile(path.join(directory, `${index}-${command.type}.png`), await keyboard.screenshot(), "base64");
      }
      const bytes = await save();
      await load(bytes, step.hash);
      assert.equal(await hash(), step.hash);
      checks.push({ command, hash: step.hash, savedAndRestored: true });
  }
  await writeFile(path.join(directory, "result.rfbsave"), Buffer.from(await save()));
  assert.deepEqual(keyboard.errors, []);
  await writeFile(path.join(directory, "report.json"), JSON.stringify({
    executable, sha256: createHash("sha256").update(await readFile(executable)).digest("hex"),
    preparation: scenario.preparation,
    checks, errors: keyboard.errors,
  }, null, 2) + "\n");
  process.stdout.write(`Ordinary Tauri standalone: ${checks.length} pet/action and save/load checks passed.\n`);
} catch (error) {
  await writeFile(path.join(directory, "failure.json"), JSON.stringify({ error: String(error.stack ?? error), checks }, null, 2));
  if (keyboard) await writeFile(path.join(directory, "failure.png"), await keyboard.screenshot(), "base64");
  throw error;
} finally {
  keyboard?.close();
  if (child.exitCode === null && child.signalCode === null) child.kill();
  await writeFile(path.join(directory, "standalone.log"), logs.join(""));
}
