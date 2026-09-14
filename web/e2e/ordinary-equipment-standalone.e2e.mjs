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
const trump = process.argv.includes("--trump");
const chaos = process.argv.includes("--chaos");
const existingRealms = process.argv.includes("--existing-realms");
const questItems = process.argv.includes("--quest-items");
const questItemsAll = process.argv.includes("--quest-items-all");
const nonQuestN1 = process.argv.includes("--non-quest-n1");
const nonQuestN3 = process.argv.includes("--non-quest-n3");
const nonQuestN2 = process.argv.includes("--non-quest-n2");
const executable = path.join(process.env.CARGO_TARGET_DIR ?? path.join(root, "target"), "debug/rfb-tauri.exe");
const directory = path.join(root, trump ? "test-results/trump" : chaos ? "test-results/chaos" : existingRealms ? "test-results/existing-realms" : nonQuestN3 ? "test-results/non-quest-n3" : nonQuestN2 ? "test-results/non-quest-n2" : nonQuestN1 ? "test-results/non-quest-n1" : questItemsAll ? "test-results/quest-items-q2-q5" : questItems ? "test-results/quest-items-q1" : "test-results/ordinary-equipment");
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
  if (existingRealms || chaos || trump) {
    const { MAGE_REALMS, PLAYTEST_BUILD_IDS } = await import("../src/character-creation.ts");
    const ids = trump ? PLAYTEST_BUILD_IDS.filter(id => id.includes("-trump")) : chaos ? PLAYTEST_BUILD_IDS.filter(id => id.includes("-chaos")) : [...MAGE_REALMS.filter(r => !["death", "craft", "chaos"].includes(r)).map(r => `demo.build.high-mage-${r}`), ...MAGE_REALMS.filter(r => !["craft", "chaos"].includes(r)).flatMap(r => [`demo.build.mage-craft-${r}`, `demo.build.mage-${r}-craft`]), ...["life", "crusade", "daemon"].map(r => `demo.build.paladin-${r}`)];
    for (const locale of ["en-US", "zh-CN"]) {
      await driver.execute('localStorage.setItem("rfb.locale",arguments[0]);return true;', [locale]);
      await keyboard.reload();
      await driver.waitFor('return document.documentElement.dataset.appMode === "title" && !document.querySelector("#session-new-game").disabled', "creation locale ready");
      await click("#session-new-game");
      for (const id of ids) {
        await selectCreationBuild(driver, id);
        assert.equal(await driver.execute('return document.querySelector(`[data-career-id="${arguments[0]}"]`).getAttribute("aria-pressed")', [id]), "true");
      }
    }
    await selectCreationRace(driver, "demo.race.rfb-human");
  }
  await selectCreationBuild(driver, trump ? "demo.build.high-mage-trump" : chaos ? "demo.build.high-mage-chaos" : nonQuestN1 || nonQuestN2 || nonQuestN3 ? "demo.build.mage-life-arcane" : "demo.build.warrior");
  await driver.execute('for(const [id,value] of [["session-seed","511"],["session-character-name","常规装备验收"]]){const input=document.getElementById(id);input.value=value;input.dispatchEvent(new Event("input",{bubbles:true}));}return true;');
  await click("#session-start-game");
  await driver.waitFor('return document.documentElement.dataset.appMode==="playing" && document.querySelector("#connection-status").classList.contains("ready")', "fresh warrior", 30000);
  const input = path.join(directory, "new-game.rfbsave");
  await writeFile(input, Buffer.from(await save()));
  const exporter = trump ? "game::tests::trump::export_trump_desktop_saves" : chaos ? "game::tests::chaos::ch5::export_chaos_desktop_saves" : existingRealms ? "game::tests::existing_realms::export_existing_realms_desktop_saves" : nonQuestN3 ? "game::tests::non_quest_artifacts::n3::export_n3_desktop_saves" : nonQuestN2 ? "game::tests::non_quest_artifacts::n2::export_n2_desktop_saves" : nonQuestN1 ? "game::tests::non_quest_artifacts::export_n1_desktop_saves" : questItemsAll ? "game::tests::quest_items::desktop::export_quest_item_desktop_saves" : questItems ? "game::tests::quest_items::export_q1_desktop_save" : "game::tests::death_scythe::export_ordinary_equipment_desktop_saves";
  const preparation = await promisify(execFile)("cargo", ["test", "-p", "rfb-core", "--lib", exporter, "--", "--ignored", "--exact"], {
    cwd: root, env: { ...process.env, ORDINARY_EQUIPMENT_INPUT: input }, windowsHide: true, timeout: 240000,
  });
  await writeFile(path.join(directory, "preparation.log"), preparation.stdout + preparation.stderr);
  const scenarios = JSON.parse(await readFile(path.join(directory, "scenarios.json"), "utf8"));
  for (const scenario of scenarios) {
    await load(await readFile(path.join(directory, `${scenario.name}.rfbsave`)), scenario.initialHash);
    for (const [index, step] of scenario.steps.entries()) {
      const command = step.command;
      if (["study-ability", "study-prayer", "cast-ability"].includes(command.type)) {
        await click("#player-ui-ability-open");
        if (command.type === "study-prayer") await click(`[data-book-item-id="${command.bookItemId}"] [data-ability-action="study-prayer"]`);
        else await click(`[data-ability-id="${command.abilityId}"] ${command.type === "study-ability" ? '[data-ability-action="study"]' : '.ability-cast-action'}`);
        if (command.target?.type === "item") {
          await driver.waitFor('return !!document.querySelector("dialog.item-target-dialog[open] select")', "spell item target");
          await driver.execute('const dialog=document.querySelector("dialog.item-target-dialog[open]");const select=dialog.querySelector("select");select.value=arguments[0];select.dispatchEvent(new Event("change",{bubbles:true}));dialog.querySelector("button[type=submit]").click();return true;', [command.target.itemId]);
        }
        if (command.target?.type === "direction") { await keyboard.key("6"); await keyboard.key("Enter"); }
        if (command.target?.type === "position") await keyboard.key("Enter");
        await readyHash(step.hash);
        await writeFile(path.join(directory, `${scenario.name}-${index}.png`), await keyboard.screenshot(), "base64");
        if (await driver.execute('return document.querySelector("#player-page-dialog").open')) await click("#player-page-close");
      } else if (command.type === "cancel-ability-direction") {
        await keyboard.key("Escape");
        await readyHash(step.hash);
      } else if (command.type === "resolve-ability-direction") {
        await keyboard.key("6"); await keyboard.key("Enter");
        await readyHash(step.hash);
      } else if (command.type === "equip") {
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
      } else if (command.type === "use-item") {
        // Keep fishing's automatic 10ms continuation from racing the explicit
        // save/load checkpoints; native commands and manual waits still run.
        if (nonQuestN2 && scenario.name === "taikobo") await keyboard.pauseTimers();
        await click("#player-ui-inventory-open");
        await click(`[data-slot-id="${step.activationSlot}"] > button`);
        await click(".equipment-activate");
        if (command.target?.type === "direction" || command.target?.type === "position") {
          await keyboard.key("6");
          await keyboard.key("Enter");
        }
        await readyHash(step.hash);
        await writeFile(path.join(directory, `${scenario.name}-${index}.png`), await keyboard.screenshot(), "base64");
        if (await driver.execute('return !document.querySelector("#player-page-close").hidden')) await click("#player-page-close");
      } else {
        if (command.type === "dig-terrain") { await keyboard.key("T", 8); await keyboard.key("8"); }
        else if (command.type === "fire") { await keyboard.key("f"); await keyboard.key("6"); await keyboard.key("Enter"); }
        else await keyboard.key(command.type === "move" ? "6" : command.type === "pick-up" ? "g" : "5");
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
    preparation: trump ? "All 27 Trump creation choices checked in both languages. Fresh formal High-Mage Trump builds prepared to level50, full resources, invulnerability, real books and local targets. UI study, position summoning, paid Lovers direction and cancellation, pet healing and weapon branding; native save/load after each action. Successful RNG seeds explicitly selected; no natural leveling or acquisition claim." : chaos ? "All 27 Chaos creation choices checked in both languages. Fresh formal High-Mage Chaos builds prepared to level50, full resources, invulnerability and local targets. Ordinary UI spell casts, real staff recharge and weapon branding, temporary centaur form and expiry, random pending direction and native save/load after every action. Success seeds and advanced books explicitly prepared; no natural leveling or acquisition claim." : existingRealms ? "All 29 existing-realm creation choices checked in both languages. Four fresh formal builds prepared to level50, source attribute potentials, full HP/MP, cleared actors and local source targets. Actual UI study/prayer, spell/class power and wait, with native save/load after each action. Successful RNG seeds explicitly selected; no natural leveling or all-spell playthrough claim." : nonQuestN3
      ? "Fresh human Mage native save, controlled level50 and source attribute potentials, local floor, invulnerability, torch and sleeping source targets. Four N3 artifacts generated through base/rarity gates at controlled depth100; normal UI pickup, equipment, melee, arrow fired from crossbow, Cupid charm, Vice activation and gold drain. Successful seeds selected through real commands. Every action saved and restored; no natural leveling/acquisition claim."
      : nonQuestN2
      ? "Fresh human Mage native save, controlled level50, local floor, invulnerability and equipped torch. Five artifacts use real base/instant candidates and rarity at controlled depth100; unknown pickup, equip, activation and wait. Ball/drain targets are prepared adjacent sleeping source actors; fishing receives adjacent water. Successful activation seeds are selected through real checks. Browser timers are paused for the fishing activation/checkpoints, so automatic continuation cannot race explicit native save/load and manual wait. Every UI action is saved and restored through normal native commands. No natural acquisition or leveling claim."
      : nonQuestN1
      ? "Fresh human Mage native save, controlled level50, local floor, invulnerability and cleared actors/items. Four artifacts generated with real base candidates and rarity at controlled depth100, initially unknown. Aglarang/Hellfire receive an adjacent source actor and a successful attack seed; Hellfire receives ordinary bolts and an equipped torch for visible UI targeting. UI pickup, equipment, melee/shooting/wait and save/load after every step. No natural acquisition or leveling claim."
      : questItemsAll
      ? "Fresh human Warrior native save; explicit level50, talent, invulnerability and local scene preparation. Eyes/Hydra/Rama artifacts are produced by actual deaths of prepared adjacent source actors with1HP and successful RNG seeds; Sting is generated after actual Telmora castle acceptance and Vault entry. Scene starts on the real reward after other actors/items are cleared. Rama receives10 normal arrows. UI performs pickup, equipment, actual activation/shooting and save/load continuation. No natural leveling or difficulty claim."
      : questItems
      ? "Fresh level-one human Warrior, chosen talent, cleared monsters/items and prepared adjacent Fang with1HP (original maxHP/rules), local floor tiles and a successful melee/drop seed. The collar is obtained by the real death reward, not granted. Core separately checks controlled complete-pool acquisition for all four actors. No natural leveling claim."
      : "Fresh level-one human Warrior, chosen talent, cleared monsters, prepared small floor area and adjacent target/magma, granted and identified seven representative bases. Scythe uses legal -255 hit enchantment, a temporary +2000 HP status and a seed selected for nonfatal backlash. Weapon and swimsuit targets start asleep. No natural acquisition or leveling claim.",
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
