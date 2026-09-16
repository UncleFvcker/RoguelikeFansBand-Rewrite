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
import { checkConfigRecords } from "./config-records.e2e.mjs";
import { checkDiscovery } from "./discovery.e2e.mjs";

// Ordinary standalone binary and production UI/load/save commands. Preparation
// uses a normal fresh character; no preparation or WebDriver-only IPC is enabled.
const root = fileURLToPath(new URL("../../", import.meta.url));
const executable = path.join(root, "target/debug/rfb-tauri.exe");
const directory = path.join(root, "test-results/map-intelligence");
await mkdir(directory, { recursive: true });
await mkdir(path.join(root, "target/e2e"), { recursive: true });
const profile = await mkdtemp(path.join(root, "target/e2e/map-intelligence-"));
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
  await driver.execute('for(const [id,value] of [["session-seed","511"],["session-character-name","地图情报验收"]]){const input=document.getElementById(id);input.value=value;input.dispatchEvent(new Event("input",{bubbles:true}));}return true;');
  await click("#session-start-game");
  await driver.waitFor('return document.documentElement.dataset.appMode==="playing" && document.querySelector("#connection-status").classList.contains("ready")', "fresh warrior", 30000);

  if (await driver.execute('return !!document.querySelector(".mutation-choice-candidate")')) {
    const before = await hash();
    await click(".mutation-choice-candidate");
    await driver.waitFor('return document.querySelector("#hash-value").title!==arguments[0]', "birth choice", 15000, [before]);
  }
  if (await driver.execute('return document.querySelector("#player-page-dialog").open')) await click("#player-page-close");
  const initial = await hash();
  await writeFile(path.join(directory, "fresh.rfbsave"), Buffer.from(await save()));
  const selector = "#map-intelligence-dialog";
  const open = async (key, modifiers = 0) => {
    await driver.execute('document.activeElement.blur();return true;');
    await keyboard.key(key, modifiers);
    await driver.waitFor('return document.querySelector("#map-intelligence-dialog").open', "map inquiry");
  };
  const close = async () => { await keyboard.key("Escape"); assert.equal(await hash(), initial); };
  const capture = async name => writeFile(path.join(directory, name + ".png"), await keyboard.screenshot(), "base64");
  await open("M", 8);
  const rows = await driver.execute('return [...document.querySelectorAll("#map-intelligence-dialog svg text")].map(n=>n.textContent)');
  assert.ok(rows.join("").includes("@"));
  const knownCells = await driver.execute('const h=document.querySelector("#map-host");return Number(h.dataset.visibleCellCount)+Number(h.dataset.rememberedCellCount)');
  assert.ok(rows.join("").replaceAll(" ","").length <= knownCells + 1);
  await capture("overview"); await close();
  checks.push("M overview respects fog and consumes no time");
  await open("L", 8);
  const origin = await driver.execute('return document.querySelector("#map-intelligence-dialog p").textContent');
  await keyboard.key("ArrowRight");
  assert.notEqual(await driver.execute('return document.querySelector("#map-intelligence-dialog p").textContent'), origin);
  await keyboard.key("v", 2);
  assert.equal(await driver.execute('return document.querySelector("#map-intelligence-dialog p").textContent'), origin);
  await capture("locate"); await close();
  checks.push("L pans half a sector; Ctrl+V returns to the player");
  await setPreferences(driver, { cameraMode: "full-map" });
  await driver.execute('document.querySelector("#map-host").scrollTo(0,0);return true;');
  await keyboard.key("v", 2);
  assert.ok(await driver.execute('return document.querySelector("#map-host").scrollLeft>0'));
  assert.equal(await hash(), initial);
  checks.push("Ctrl+V centers the scrollable main map without changing zoom or game state");
  await open("[");
  await capture("monsters"); await close();
  checks.push("[ opens the currently perceived monster list");
  await open("/");
  await keyboard.text("D");
  assert.match(await driver.execute('return document.querySelector("#map-intelligence-dialog").textContent'), /上古龙/);
  await keyboard.key("a", 2);
  assert.equal(await driver.execute('return document.querySelector("#map-intelligence-dialog select").value'), "all");
  await keyboard.key("m", 2);
  await keyboard.text("不存在的名称");
  await capture("symbols"); await close();
  checks.push("/ symbol definition, Ctrl+A and Ctrl+M stay inside the query");
  await open("f", 2);
  assert.match(await driver.execute('return document.querySelector("#map-intelligence-dialog").textContent'), /典型的城镇/);
  await close();
  checks.push("Ctrl+F shows the Core floor feeling without spending a turn");
  await setPreferences(driver, { inputPreset: "roguelike" });
  await open("W", 8); await close();
  await open("M", 8);
  assert.equal(await driver.execute('return document.querySelector("#map-intelligence-dialog").open'), true);
  await load(await save(), initial);
  assert.equal(await driver.execute('return document.querySelector("#map-intelligence-dialog").open'), false);
  checks.push("Rogue W/M and native reload close stale inquiries");

  const guide = "#help-knowledge-dialog";
  const guideOpen = async key => {
    await driver.execute('document.activeElement.blur();return true;');
    await keyboard.key(key);
    await driver.waitFor('return document.querySelector("#help-knowledge-dialog").open', "help/knowledge opens");
  };
  await guideOpen("?");
  assert.match(await driver.execute('return document.querySelector(".guide-preset").textContent'), /Roguelike/);
  assert.equal(await driver.execute('return document.querySelectorAll("#help-knowledge-dialog section").length'), 12);
  await keyboard.key("/");
  assert.equal(await driver.execute('return document.activeElement.id'), "guide-search");
  await keyboard.text("铭刻");
  assert.ok(await driver.execute('return document.querySelectorAll("#help-knowledge-dialog section[hidden]").length>0 && !document.querySelector("#guide-topic-items").hidden'));
  await keyboard.text("p~?不会匹配");
  assert.equal(await driver.execute('return document.querySelector("#help-knowledge-dialog [role=status]").hidden'), false);
  assert.equal(await driver.execute('return document.querySelectorAll("dialog[open]").length'), 1);
  await driver.execute('const s=document.querySelector("#guide-search");s.value="";s.dispatchEvent(new Event("input",{bubbles:true}));return true;');
  await capture("help-chinese");
  // Search fields consume their first Escape to clear native search state on some platforms.
  await driver.execute('document.querySelector("#help-knowledge-dialog").focus();return true;');
  await close();
  assert.equal(await driver.execute('return document.querySelector("#help-knowledge-dialog").open'), false);
  checks.push("? opens a searchable 12-chapter preset-aware manual; text input stays local and no turn elapses");

  await guideOpen("~");
  await capture("knowledge-chinese");
  const routes = [
    ["a","artifacts"], ["o","objects"], ["e","egos"],
    ["_","autopick","#mogaminator-dialog"], ["c","materials","#player-page-dialog"],
    ["m","monsters"], ["w","wanted"], ["u","uniques"], ["k","kills"], ["p","pets"],
    ["d","dungeons"], ["q","quests","#player-page-dialog"], ["t","terrain","#map-intelligence-dialog"],
    ["@","self","#player-page-dialog"], ["W","weapon","#player-page-dialog"], ["S","shooter","#player-page-dialog"],
    ["M","mutations","#player-page-dialog"], ["v","virtues","#player-page-dialog"], ["x","extra","#player-page-dialog"],
    ["H","scores"], ["P","weapon-skills","#player-page-dialog"], ["s","spell-skills","#player-page-dialog"],
  ];
  for (const [key, entry, childSelector] of routes) {
    assert.equal(await driver.execute('return document.querySelector("#help-knowledge-dialog").open'), true, `index before ${entry}`);
    const label = await driver.execute('return document.querySelector(`[data-knowledge-entry="${arguments[0]}"]`).textContent.split(" · ").slice(1).join(" · ")', [entry]);
    if (key === "_") await click(`[data-knowledge-entry="${entry}"]`);
    else await keyboard.key(key, /^[A-Z]$/.test(key) ? 8 : 0);
    assert.equal(await driver.execute('return document.querySelector("#help-knowledge-dialog h3").textContent'), label, entry);
    if (["objects", "artifacts", "egos", "monsters", "uniques", "kills", "dungeons"].includes(entry)) {
      assert.equal(await driver.execute('return !!document.querySelector("#archive-search") && !!document.querySelector("#archive-list")'), true, entry);
      const count = await driver.execute('return document.querySelectorAll("[data-discovery-id]").length');
      if (entry === "objects") assert.ok(count > 0, "birth equipment recorded");
      await driver.execute('const s=document.querySelector("#archive-search");s.value="no-such-discovery-xyz";s.dispatchEvent(new Event("input"));return true;');
      assert.equal(await driver.execute('return document.querySelectorAll("[data-discovery-id]").length'), 0);
      await driver.execute('const s=document.querySelector("#archive-search");s.value="";s.dispatchEvent(new Event("input"));return true;');
      assert.equal(await driver.execute('return document.querySelectorAll("[data-discovery-id]").length'), count);
    }
    if (childSelector) {
      await driver.execute('document.querySelector("#guide-open-view").focus();return true;');
      await keyboard.key("Enter");
      assert.equal(await driver.execute('return document.querySelector(arguments[0]).open', [childSelector]), true, entry);
      if (["monsters", "uniques"].includes(entry)) assert.equal(await driver.execute('return document.querySelector("#map-intelligence-dialog select").value'), entry === "uniques" ? "unique" : "all");
      if (["weapon", "shooter"].includes(entry)) assert.equal(await driver.execute('return document.querySelector("#character-detail-tab-offense").getAttribute("aria-selected")'), "true");
      if (childSelector === "#mogaminator-dialog") await click("#mogaminator-close");
      else await keyboard.key("Escape");
      assert.equal(await driver.execute('return document.querySelector(arguments[0]).open', [childSelector]), false, entry);
      assert.equal(await driver.execute('return document.querySelector("#help-knowledge-dialog").open'), true, entry);
    } else assert.equal(await driver.execute('return !!document.querySelector("#guide-open-view")'), false, entry);
    await keyboard.key("Escape");
    assert.equal(await driver.execute('return document.querySelectorAll("[data-knowledge-entry]").length'), routes.length);
    assert.equal(await driver.execute('return document.querySelector("#help-knowledge-dialog").open'), true, `returned from ${entry}`);
    assert.equal(await hash(), initial, entry);
  }
  await keyboard.key("Escape");
  assert.equal(await driver.execute('return document.querySelector("#help-knowledge-dialog").open'), false);
  checks.push("All 22 source knowledge entries support case-sensitive selection, scope, existing views and return without changing state");
  await click('[data-guide="knowledge"]');
  await load(await save(), initial);
  assert.equal(await driver.execute('return document.querySelector("#help-knowledge-dialog").open'), false);
  checks.push("Knowledge toolbar entry opens and native load closes stale knowledge");
  await checkConfigRecords({ keyboard, driver, click, hash, save, load, capture, checks, directory });
  await checkDiscovery({ root, keyboard, driver, click, hash, save, load, capture, checks, directory });
  await setPreferences(driver, { locale: "en-US" });
  await driver.waitFor('return document.querySelector("#hash-value").title!==arguments[0]', "Core locale applied", 15000, [initial]);
  await open("M", 8); await keyboard.viewport(390,844);
  assert.equal(await driver.execute('const d=document.querySelector("#map-intelligence-dialog");return d.scrollWidth<=d.clientWidth && d.getBoundingClientRect().width<=innerWidth'), true);
  await capture("english-narrow");
  await keyboard.key("Escape");
  for (const key of ["?", "~"]) {
    await guideOpen(key);
    assert.equal(await driver.execute('const d=document.querySelector("#help-knowledge-dialog");return d.scrollWidth<=d.clientWidth && d.getBoundingClientRect().width<=innerWidth'), true);
    assert.doesNotMatch(await driver.execute('return document.querySelector("#help-knowledge-dialog").textContent'), /\[guide-|[\u4e00-\u9fff]/);
    await capture(key === "?" ? "help-english-narrow" : "knowledge-english-narrow");
    await keyboard.key("Escape");
  }
  checks.push("English help and knowledge fit 390px and have no missing translations");
  await driver.execute('document.activeElement.blur();return true;');
  await keyboard.key("@");
  await driver.waitFor('return document.querySelector("#config-records-dialog").open', "English configuration opens");
  assert.equal(await driver.execute('const d=document.querySelector("#config-records-dialog");return d.open && d.scrollWidth<=d.clientWidth && d.getBoundingClientRect().width<=innerWidth'), true);
  assert.doesNotMatch(await driver.execute('return document.querySelector("#config-records-dialog").textContent'), /\[cfg-|[\u4e00-\u9fff]/);
  await capture("config-english-narrow"); await keyboard.key("Escape");
  checks.push("English configuration and macro editor fits a 390px window");
  assert.deepEqual(keyboard.errors, []);
  await writeFile(path.join(directory, "report.json"), JSON.stringify({executable, sha256:createHash("sha256").update(await readFile(executable)).digest("hex"), preparation:"Normal fresh human Warrior for base map/help/configuration checks. Discovery checks additionally load an explicitly prepared character: Warrens depth 3, known uniques and a credited death, research, then identified artifacts/egos removed from the current inventory. See discovery-scenario.json; no natural acquisition claim.", checks, errors:keyboard.errors},null,2)+"\n");
  await Promise.all(["failure.json","failure.png"].map(name=>rm(path.join(directory,name),{force:true})));
  process.stdout.write("Ordinary standalone: "+checks.length+" map/help/configuration checks passed.\n");
} catch (error) {
  await writeFile(path.join(directory,"failure.json"),JSON.stringify({error:String(error.stack??error),checks},null,2));
  if(keyboard) await writeFile(path.join(directory,"failure.png"),await keyboard.screenshot(),"base64");
  throw error;
} finally {
  keyboard?.close();
  if(child.exitCode===null && child.signalCode===null) child.kill();
  await writeFile(path.join(directory,"standalone.log"),logs.join(""));
}
