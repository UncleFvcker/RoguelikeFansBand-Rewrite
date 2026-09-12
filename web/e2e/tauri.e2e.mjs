// SPDX-License-Identifier: MPL-2.0

import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import net from "node:net";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { runRendererProfile } from "./render-profile.e2e.mjs";
import { runEgoScenario } from "./ego.e2e.mjs";
import { runCharacterCreationScenario, selectCreationRace, selectCreationBuild } from "./character-creation.e2e.mjs";
import { runCreationLayoutScenario } from "./character-creation-layout.e2e.mjs";
import { runMindcrafterUiScenario } from "./mindcrafter.e2e.mjs";
import { runBerserkerUiScenario } from "./berserker.e2e.mjs";
import { runDuelistUiScenario } from "./duelist.e2e.mjs";
import { runMageUiScenario } from "./mage.e2e.mjs";
import { runRangerUiScenario } from "./ranger.e2e.mjs";
import { runPriestUiScenario } from "./priest.e2e.mjs";
import { runWarriorMageUiScenario } from "./warrior-mage.e2e.mjs";
import { runMagicEaterUiScenario } from "./magic-eater.e2e.mjs";
import { runCraftScenario } from "./craft.e2e.mjs";
import { runTownMapScenario } from "./town-maps.e2e.mjs";
import { runOneRingScenario } from "./one-ring.e2e.mjs";
import { runThingolScenario } from "./thingol.e2e.mjs";

const webDirectory = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repositoryDirectory = path.resolve(webDirectory, "..");

// The scenario pins protocol and content identity against the same sources
// of truth the build uses, so routine version bumps cannot silently strand
// this script on stale literals.
async function loadExpectedIdentity() {
  const lock = JSON.parse(
    await readFile(
      path.join(repositoryDirectory, "packs", "rfb-demo-original", "content.lock.json"),
      "utf8",
    ),
  );
  const protocolSource = await readFile(
    path.join(repositoryDirectory, "crates", "rfb-protocol", "src", "lib.rs"),
    "utf8",
  );
  const protocolMatch = protocolSource.match(/PROTOCOL_VERSION: &str = "([0-9.]+)"/);
  if (!protocolMatch) {
    throw new Error("PROTOCOL_VERSION not found in rfb-protocol/src/lib.rs");
  }
  return {
    protocolVersion: protocolMatch[1],
    contentId: lock.packId,
    contentHash: lock.contentHash,
  };
}
const executable = path.join(
  repositoryDirectory,
  "target",
  "e2e",
  "debug",
  "rfb-tauri.exe",
);
const artifactDirectory = path.join(repositoryDirectory, "test-results");
const diagnosticDirectory = path.join(artifactDirectory, "e2e-crash-diagnostics");
const desktopLogPath = path.join(artifactDirectory, "e2e-rfb-desktop.log");
const renderProfileOnly = process.argv.includes("--render-profile");
const tomteOnly = process.argv.includes("--tomte");
const tonberryOnly = process.argv.includes("--tonberry");
const entOnly = process.argv.includes("--ent");
const spectreOnly = process.argv.includes("--spectre");
const lifeForceOnly = process.argv.includes("--life-force");
const egoOnly = process.argv.includes("--ego");
const logs = [];
let child;
let client;

async function main() {
  if (process.platform !== "win32") {
    throw new Error("Tauri desktop E2E currently requires Windows WebView2");
  }

  try {
    await rm(diagnosticDirectory, { recursive: true, force: true });
    await rm(desktopLogPath, { force: true });
    const port = await reservePort();
    const creationLayout = process.argv.includes("--magic-eater-ui") || process.argv.includes("--zul") || process.argv.includes("--town-maps") || process.argv.includes("--warrior-mage-play") || process.argv.includes("--warrior-mage-ui") || process.argv.includes("--character-creation") || process.argv.includes("--creation-layout") || process.argv.includes("--mindcrafter") || process.argv.includes("--berserker") || process.argv.includes("--duelist-ui") || process.argv.includes("--mage-ui") || process.argv.includes("--mage-play") || process.argv.includes("--ranger-ui") || process.argv.includes("--ranger-play") || process.argv.includes("--priest-ui") || process.argv.includes("--priest-play");
    const debugProfile = path.join(repositoryDirectory, "target", "e2e", "creation-webview");
    child = spawn(executable, [], {
      cwd: repositoryDirectory,
      env: {
        ...process.env,
        ...(creationLayout ? { WEBVIEW2_USER_DATA_FOLDER: debugProfile } : {}),
        WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: [
          process.env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS,
          "--disable-gpu",
        ]
          .filter(Boolean)
          .join(" "),
        TAURI_WEBDRIVER_PORT: String(port),
        RFB_E2E_DIAGNOSTIC_ROOT: diagnosticDirectory,
        RFB_E2E_LOG_PATH: desktopLogPath,
      },
      stdio: ["ignore", "pipe", "pipe"],
      windowsHide: true,
    });
    captureOutput(child.stdout, "stdout");
    captureOutput(child.stderr, "stderr");
    child.once("exit", (code, signal) => {
      logs.push(`[process] exited code=${code ?? "null"} signal=${signal ?? "null"}`);
    });

    await waitForServer(port, child);
    client = await WebDriverClient.create(port, child);
    if (renderProfileOnly) {
      await runRendererProfile(client, artifactDirectory);
    } else if (process.argv.includes("--thingol")) {
      await runThingolScenario(client, path.join(artifactDirectory, "thingol"));
    } else if (process.argv.includes("--one-ring")) {
      await runOneRingScenario(client, path.join(artifactDirectory, "one-ring"));
    } else if (process.argv.includes("--town-maps")) {
      await runTownMapScenario(client, path.join(artifactDirectory, "town-maps"), debugProfile);
    } else if (process.argv.includes("--zul")) {
      await runTownMapScenario(client, path.join(artifactDirectory, "zul"), debugProfile, true);
    } else if (process.argv.includes("--character-creation")) {
      await runCharacterCreationScenario(client, artifactDirectory);
      await runCreationLayoutScenario(client, artifactDirectory, debugProfile);
    } else if (process.argv.includes("--creation-layout")) {
      await runCreationLayoutScenario(client, artifactDirectory, debugProfile);
    } else if (process.argv.includes("--mindcrafter")) {
      await runMindcrafterUiScenario(client, artifactDirectory, debugProfile);
    } else if (process.argv.includes("--craft")) {
      await runCraftScenario(client, path.join(artifactDirectory, "craft"));
    } else if (process.argv.includes("--berserker")) {
      await runBerserkerUiScenario(client, artifactDirectory, debugProfile);
    } else if (process.argv.includes("--duelist-ui")) {
      await runDuelistUiScenario(client, path.join(artifactDirectory, "duelist-ui"), debugProfile);
    } else if (process.argv.includes("--mage-ui")) {
      await runMageUiScenario(client, path.join(artifactDirectory, "mage-ui"), debugProfile);
    } else if (process.argv.includes("--ranger-ui")) {
      await runRangerUiScenario(client, path.join(artifactDirectory, "ranger-ui"), debugProfile);
    } else if (process.argv.includes("--magic-eater-ui")) {
      await runMagicEaterUiScenario(client, path.join(artifactDirectory, "magic-eater-ui"), debugProfile);
    } else if (process.argv.includes("--warrior-mage-play")) {
      await runWarriorMageUiScenario(client, path.join(artifactDirectory, "warrior-mage-play"), debugProfile, true);
    } else if (process.argv.includes("--warrior-mage-ui")) {
      await runWarriorMageUiScenario(client, path.join(artifactDirectory, "warrior-mage-ui"), debugProfile);
    } else if (process.argv.includes("--priest-ui")) {
      await runPriestUiScenario(client, path.join(artifactDirectory, "priest-ui"), debugProfile);
    } else if (process.argv.includes("--priest-play")) {
      await runPriestUiScenario(client, path.join(artifactDirectory, "priest-play"), debugProfile, true);
    } else if (process.argv.includes("--ranger-play")) {
      await runRangerUiScenario(client, path.join(artifactDirectory, "ranger-play"), debugProfile, true);
    } else if (process.argv.includes("--mage-play")) {
      await runMageUiScenario(client, path.join(artifactDirectory, "mage-play"), debugProfile, true);
    } else if (lifeForceOnly) {
      await runLifeForceScenario(client);
    } else if (tomteOnly || tonberryOnly || entOnly || spectreOnly) {
      await runRaceScenario(client, spectreOnly ? "spectre" : entOnly ? "ent" : tonberryOnly ? "tonberry" : "tomte");
    } else if (egoOnly) {
      await runEgoScenario(client, artifactDirectory);
    } else {
      await runScenario(client);
    }
    if (process.env.RFB_E2E_CAPTURE_SCREENSHOT === "1") {
      await mkdir(artifactDirectory, { recursive: true });
      await writeFile(
        path.join(artifactDirectory, "tauri-e2e-success.png"),
        await client.screenshot(),
        "base64",
      );
    }
    process.stdout.write(
      renderProfileOnly ? "Renderer profile passed.\n" : tomteOnly || tonberryOnly || entOnly || spectreOnly ? "Race desktop acceptance passed.\n" : "Tauri desktop E2E passed.\n",
    );
  } catch (error) {
    await mkdir(artifactDirectory, { recursive: true });
    if (client) {
      try {
        const screenshot = await client.screenshot();
        await writeFile(path.join(artifactDirectory, "tauri-e2e.png"), screenshot, "base64");
      } catch (screenshotError) {
        logs.push(`[screenshot] ${String(screenshotError)}`);
      }
    }
    await writeFile(path.join(artifactDirectory, "tauri-e2e.log"), `${logs.join("\n")}\n`);
    process.stderr.write(`${error instanceof Error ? error.stack : String(error)}\n`);
    process.stderr.write(`Artifacts: ${artifactDirectory}\n`);
    process.exitCode = 1;
  } finally {
    if (client && !renderProfileOnly) await cleanupNativeTestSaves(client).catch(() => undefined);
    if (client) await client.close().catch(() => undefined);
    if (child && child.exitCode === null && child.signalCode === null) child.kill();
  }
}

async function cleanupNativeTestSaves(driver) {
  await driver.execute(`
    window.confirm = () => true;
    for (const row of document.querySelectorAll(".native-save-item")) {
      if (row.querySelector(".native-save-name")?.textContent?.startsWith("E2E 原生存档 ")) {
        row.querySelector('[data-native-save-action="delete"]')?.click();
      }
    }
    return true;
  `);
  await delay(300);
}

async function runScenario(driver) {
  const expected = await loadExpectedIdentity();
  const report = { identity: expected, checks: [] };
  await driver.waitFor(`return document.documentElement.dataset.appMode === "title"`, "title", 60_000);
  await driver.execute(`localStorage.clear(); localStorage.setItem("rfb.locale", "zh-CN"); setTimeout(() => location.reload(), 250); return true;`);
  await driver.waitFor(`return performance.getEntriesByType("navigation")[0]?.type === "reload" && document.documentElement.dataset.appMode === "title"`, "clean title", 60_000);

  async function start(build, seed) {
    await driver.execute(`
      window.__acceptanceErrors = [];
      window.addEventListener("error", event => window.__acceptanceErrors.push(event.message));
      document.querySelector("#session-new-game").click();

      const seed = document.querySelector("#session-seed"); seed.value = arguments[1];
      seed.dispatchEvent(new Event("input", { bubbles: true }));
      return true;
    `, [build, seed]);
    await selectCreationBuild(driver, build);
    await click(driver, "#session-start-game");
    await driver.waitFor(`return document.documentElement.dataset.appMode === "playing" && document.querySelector("#connection-status")?.classList.contains("ready")`, "new game", 60_000);
  }
  async function state() {
    return driver.execute(`return {
      hash: document.querySelector("#hash-value")?.title,
      turn: parseInt(document.querySelector("#turn-value")?.textContent, 10),
      position: document.querySelector("#position-value")?.textContent,
      equipment: document.querySelector("#equipment-list")?.textContent,
      resources: document.querySelector("#resource-list")?.textContent,
      errors: window.__acceptanceErrors ?? [],
    };`);
  }
  async function waitTurnAfter(turn) {
    await driver.waitFor(`return parseInt(document.querySelector("#turn-value")?.textContent, 10) > arguments[0]`, "committed action", 10_000, [turn]);
  }
  async function installDownloads() {
    await driver.execute(`
      const downloads = []; window.__rfbE2eDownloads = downloads;
      URL.createObjectURL = blob => { downloads.push({ blob, size: blob.size, fileName: "" }); return "blob:acceptance-" + downloads.length; };
      URL.revokeObjectURL = () => {};
      HTMLAnchorElement.prototype.click = function () { downloads.at(-1).fileName = this.download; };
      return true;
    `);
  }
  async function saveRestore() {
    await installDownloads();
    const saved = await state();
    await driver.execute(`document.querySelector(".hud-menu").open = true; return true;`);
    await click(driver, "#save-button");
    await driver.waitFor(`return window.__rfbE2eDownloads.some(item => item.fileName.endsWith(".rfbsave"))`, "save export");
    assert.ok((await lastDownload(driver)).size > 100);
    await driver.execute(`document.querySelector(".hud-menu").open = false; return true;`);
    await dispatchKey(driver, "Numpad5", "5"); await waitTurnAfter(saved.turn);
    assert.notEqual((await state()).hash, saved.hash);
    await driver.execute(`
      const saved = window.__rfbE2eDownloads.find(item => item.fileName.endsWith(".rfbsave"));
      const transfer = new DataTransfer(); transfer.items.add(new File([saved.blob], saved.fileName));
      const input = document.querySelector("#load-input"); input.files = transfer.files;
      input.dispatchEvent(new Event("change", { bubbles: true })); return true;
    `);
    await driver.waitFor(`return document.querySelector("#hash-value")?.title === arguments[0]`, "exact save restoration", 10_000, [saved.hash]);
    const restored = await state();
    assert.equal(restored.turn, saved.turn); assert.equal(restored.position, saved.position);
    assert.equal(restored.equipment, saved.equipment); assert.equal(restored.resources, saved.resources);
    await dispatchKey(driver, "Numpad5", "5"); await waitTurnAfter(saved.turn);
    report.checks.push({ check: "save-restore-and-continue", hash: saved.hash });
  }

  await start("demo.build.warrior", "42");
  const identity = await driver.execute(`return {
    build: document.querySelector("#app").dataset.sessionBuildId,
    seed: document.querySelector("#app").dataset.sessionSeed,
    protocol: document.querySelector("#map-host").dataset.protocolVersion,
    content: document.querySelector("#map-host").dataset.contentHash,
    canvas: Boolean(document.querySelector("#map-host canvas")),
  };`);
  assert.equal(identity.build, "demo.build.warrior"); assert.equal(identity.seed, "42");
  assert.equal(identity.protocol, expected.protocolVersion);
  assert.equal(identity.content, expected.contentHash); assert.equal(identity.canvas, true);
  report.checks.push({ check: "warrior-new-game", ...identity });

  const menuHash = (await state()).hash;
  for (const button of ["#player-ui-inventory-open", "#player-ui-character-open", "#player-ui-tasks-open", "#player-ui-ability-open"]) {
    await click(driver, button);
    await driver.waitFor(`return document.querySelector("#player-page-dialog")?.open`, button);
    assert.equal(await driver.execute(`const d = document.querySelector("#player-page-dialog"); return d.scrollWidth <= d.clientWidth;`), true);
    await click(driver, "#player-page-close");
  }
  await dispatchKey(driver, "KeyI", "i");
  await driver.waitFor(`return document.querySelector("#player-page-dialog")?.open`, "inventory shortcut");
  await dispatchKey(driver, "KeyI", "i");
  await driver.waitFor(`return !document.querySelector("#player-page-dialog")?.open`, "shortcut closes menu");
  await click(driver, "#player-ui-settings-open");
  await driver.waitFor(`return document.querySelector("#player-ui-settings-dialog")?.open`, "settings menu");
  await click(driver, "#player-ui-settings-close");
  assert.equal((await state()).hash, menuHash);
  report.checks.push({ check: "menus-shortcuts-no-turn" });

  await driver.execute(`window.__acceptanceCanvas = document.querySelector("#map-host canvas"); return true;`);
  for (const [selector, value, field, projected] of [
    ["#camera-mode", "full-map", "cameraMode", "full-map"],
    ["#camera-mode", "player-centered", "cameraMode", "player-centered"],
    ["#zoom-level", "1.5", "zoom", "1.5"],
    ["#zoom-level", "1", "zoom", "1"],
    ["#tileset-preset", "image", "tilesetId", "rfb.tileset.pixel-28"],
    ["#tileset-preset", "ascii", "tilesetId", "rfb.tileset.ascii-default"],
  ]) {
    await driver.execute(`const input = document.querySelector(arguments[0]); input.value = arguments[1]; input.dispatchEvent(new Event("change", { bubbles: true })); return true;`, [selector, value]);
    await driver.waitFor(`return document.querySelector("#map-host").dataset[arguments[0]] === arguments[1]`, selector, 10_000, [field, projected]);
    assert.equal((await state()).hash, menuHash);
    assert.equal(await driver.execute(`return window.__acceptanceCanvas === document.querySelector("#map-host canvas");`), true);
  }
  report.checks.push({ check: "camera-zoom-tileset-canvas-reuse" });

  await click(driver, "#player-ui-inventory-open");
  const equipment = await driver.execute(`
    const row = document.querySelector("#equipment-list .equipment-item:not(.equipment-slot-vacant)");
    const result = { slot: row.dataset.slotId, name: row.querySelector(".equipment-slot-name").textContent };
    row.querySelector("button").click(); return result;
  `);
  await driver.waitFor(`return document.querySelector("#inventory-detail-dialog")?.open`, "equipment details");
  await click(driver, "#inventory-detail-actions .equipment-actions button:last-child");
  await driver.waitFor(`return document.querySelector('#equipment-list [data-slot-id="' + arguments[0] + '"]')?.classList.contains("equipment-slot-vacant")`, "unequip", 10_000, [equipment.slot]);
  await driver.execute(`
    if (document.querySelector("#inventory-detail-dialog").open) document.querySelector("#inventory-detail-close").click();
    const row = [...document.querySelectorAll("#inventory-list .inventory-item")].find(row => row.querySelector(".inventory-item-name").textContent === arguments[0]);
    if (!row) throw new Error("Unequipped item missing from inventory");
    row.querySelector('input[type="checkbox"]').click(); return true;
  `, [equipment.name]);
  await click(driver, "#inventory-equip");
  await driver.waitFor(`const row = document.querySelector('#equipment-list [data-slot-id="' + arguments[0] + '"]'); return row && !row.classList.contains("equipment-slot-vacant");`, "re-equip", 10_000, [equipment.slot]);
  assert.equal(await driver.execute(`return document.querySelector('#equipment-list [data-slot-id="' + arguments[0] + '"] .equipment-slot-name').textContent;`, [equipment.slot]), equipment.name);
  await click(driver, "#player-page-close");
  report.checks.push({ check: "unequip-and-equip", ...equipment });
  await saveRestore();

  const nativeSaveName = `E2E 原生存档 ${Date.now()}`;
  const nativeHash = (await state()).hash;
  await driver.execute(`const input = document.querySelector("#native-save-name"); input.value = arguments[0]; input.dispatchEvent(new Event("input", { bubbles: true })); document.querySelector("#native-save-create").click(); return true;`, [nativeSaveName]);
  await driver.waitFor(`return [...document.querySelectorAll(".native-save-name")].some(row => row.textContent === arguments[0])`, "native save", 10_000, [nativeSaveName]);
  const slot = await driver.execute(`return [...document.querySelectorAll(".native-save-item")].find(row => row.querySelector(".native-save-name")?.textContent === arguments[0]).dataset.slotId;`, [nativeSaveName]);
  await driver.execute(`setTimeout(() => location.reload(), 250); return true;`);
  await driver.waitFor(`return document.documentElement.dataset.appMode === "title"`, "reload to title", 60_000);
  await click(driver, "#session-load-game");
  const loadSelector = `#session-load-list [data-slot-id="${slot}"] [data-session-load-action="load"]`;
  await driver.waitFor(`return document.querySelector(arguments[0]) && !document.querySelector(arguments[0]).disabled`, "native slot in title", 10_000, [loadSelector]);
  await click(driver, loadSelector);
  await driver.waitFor(`return document.documentElement.dataset.appMode === "playing" && document.querySelector("#hash-value")?.title === arguments[0]`, "native title load", 60_000, [nativeHash]);
  report.checks.push({ check: "native-save-title-load", hash: nativeHash });
  assert.deepEqual((await state()).errors, []);
  await cleanupNativeTestSaves(driver);

  await driver.execute(`setTimeout(() => location.reload(), 250); return true;`);
  await driver.waitFor(`return document.documentElement.dataset.appMode === "title"`, "mage title", 60_000);
  await start("demo.build.high-mage-death", "7");
  await click(driver, "#player-ui-ability-open");
  await driver.waitFor(`return document.querySelector("#player-page-dialog")?.open && document.querySelectorAll(".ability-row").length > 0`, "mage spellbook");
  const spell = await driver.execute(`const row = [...document.querySelectorAll(".ability-row")].find(row => !row.querySelector(".ability-actions button")?.disabled); if (!row) throw new Error("No learnable spell"); const name = row.querySelector(".ability-name").textContent; row.querySelector(".ability-actions button").click(); return name;`);
  await driver.waitFor(`return [...document.querySelectorAll(".ability-row")].some(row => row.querySelector(".ability-name").textContent === arguments[0] && !row.querySelector(".ability-cast-action").disabled)`, "spell learned", 10_000, [spell]);
  const beforeCast = await state();
  await driver.execute(`const row = [...document.querySelectorAll(".ability-row")].find(row => row.querySelector(".ability-name").textContent === arguments[0]); row.querySelector(".ability-cast-action").click(); return true;`, [spell]);
  await waitTurnAfter(beforeCast.turn);
  await driver.waitFor(`return Boolean(document.querySelector(".message-ability-cast-success"))`, "successful cast");
  assert.notEqual((await state()).resources, beforeCast.resources);
  assert.equal(await driver.execute(`return document.querySelector("#player-page-dialog").open;`), false);
  report.checks.push({ check: "mage-learn-and-cast", spell });
  await saveRestore();
  assert.deepEqual((await state()).errors, []);
  await click(driver, "#replay-button");
  await driver.waitFor(`return window.__rfbE2eDownloads.some(item => item.fileName.endsWith(".rfbreplay") && item.size > 50)`, "diagnostic replay export");
  report.checks.push({ check: "diagnostic-replay-export" });
  await driver.execute(`window.dispatchEvent(new ErrorEvent("error", { message: "synthetic E2E crash" })); return true;`);
  await driver.waitFor(`return document.documentElement.dataset.crashDiagnosticReport?.endsWith(".rfbdiagnostic") && document.documentElement.dataset.crashDiagnosticReason === "frontend-error"`, "automatic frontend crash diagnostic");
  report.checks.push({ check: "frontend-crash-diagnostic" });
  await mkdir(artifactDirectory, { recursive: true });
  await writeFile(path.join(artifactDirectory, "playable-acceptance.json"), JSON.stringify(report, null, 2) + "\n");
}

async function runRaceScenario(driver, raceId) {
  const isTomte = raceId === "tomte";
  const isEnt = raceId === "ent";
  const isSpectre = raceId === "spectre";
  const raceName = isSpectre ? "幽灵" : isEnt ? "树人" : isTomte ? "托姆特" : "冬贝利";
  const slot = isTomte ? "head" : "right-hand";
  const trait = isSpectre ? "spectre-rules" : isEnt ? "ent-rules" : isTomte ? "tomte-headgear" : "tonberry-rules";
  const builds = ["warrior", "high-mage-death", "archer"];
  const expected = await loadExpectedIdentity();
  const report = { identity: expected, checks: [] };
  const state = () => driver.execute(`return {
    hash: document.querySelector("#hash-value")?.title,
    turn: parseInt(document.querySelector("#turn-value")?.textContent, 10),
    equipment: document.querySelector("#equipment-list")?.textContent,
    race: document.querySelector("#character-race-value")?.textContent,
    errors: window.__raceErrors ?? [],
  };`);
  const afterTurn = turn => driver.waitFor(`return parseInt(document.querySelector("#turn-value")?.textContent, 10) > arguments[0]`, "race action committed", 10_000, [turn]);
  await driver.waitFor(`return document.documentElement.dataset.appMode === "title"`, "race title", 60_000);
  await driver.execute(`localStorage.setItem("rfb.locale", "zh-CN"); setTimeout(() => location.reload(), 100); return true;`);
  await driver.waitFor(`return performance.getEntriesByType("navigation")[0]?.type === "reload" && document.documentElement.dataset.appMode === "title"`, "Chinese title", 60_000);
  await mkdir(artifactDirectory, { recursive: true });
  for (const build of builds) {
    await click(driver, "#session-new-game");
    await selectCreationBuild(driver, "demo.build." + build);
    await selectCreationRace(driver, "rfb-legacy.race." + raceId);
    const description = await driver.execute(`
      window.__raceErrors = [];
      window.addEventListener("error", event => window.__raceErrors.push(event.message));
      document.querySelector("#session-seed").value = "83";
      document.querySelector("#session-character-name").value = arguments[2] + "验收";
      const note = document.querySelector("#session-race-details");
      const race = document.querySelector('[data-race-id="rfb-legacy.race.' + arguments[1] + '"]');
      return { visible: note.checkVisibility(), text: note.textContent, name: document.querySelector("#session-race-detail-title").textContent, descriptionId: race.getAttribute("aria-describedby") };
    `, [build, raceId, raceName]);
    assert.equal(description.name, raceName);
    assert.equal(description.visible, true);
    assert.equal(description.descriptionId, "session-race-description");
    assert.ok(isSpectre ? description.text.includes("150") && description.text.includes("5000") && description.text.includes("恐吓怪物")
      : isEnt ? description.text.includes("4200") && description.text.includes("14999") && description.text.includes("召唤树人")
      : isTomte ? description.text.includes("1.0 磅") && description.text.includes("40")
      : description.text.includes("偏爱菜刀和宽刃刀") && description.text.includes("0.04") && description.text.includes("军刀"));
    if (isEnt || isSpectre) {
      const layout = await driver.execute(`
        const note = document.querySelector("#session-race-details");
        note.scrollIntoView({ block: "start" });
        return { bullets: note.querySelectorAll("li").length, fits: note.scrollWidth <= note.clientWidth };
      `, [raceId]);
      assert.deepEqual(layout, { bullets: 8, fits: true });
      await writeFile(path.join(artifactDirectory, `${raceId}-${build}-creation.png`), await driver.screenshot(), "base64");
    }
    await click(driver, "#session-start-game");
    await driver.waitFor(`return document.documentElement.dataset.appMode === "playing" && document.querySelector("#connection-status")?.classList.contains("ready")`, "race creation", 60_000);
    assert.equal((await state()).race, raceName);
    assert.equal(await driver.execute(`return document.querySelector("#map-host").dataset.contentHash;`), expected.contentHash);
    assert.equal(await driver.execute(`return document.querySelector("#map-host").dataset.protocolVersion;`), expected.protocolVersion);
    await click(driver, "#player-ui-character-open");
    await click(driver, "#character-tab-details");
    await click(driver, isTomte || isEnt || isSpectre ? "#character-detail-tab-defenses" : "#character-detail-tab-offense");
    await driver.waitFor(`return document.querySelector('[data-trait="' + arguments[0] + '"]')?.textContent.includes(arguments[1])`, "racial trait projection", 10_000, [trait, isSpectre ? "5000" : isEnt ? "14999" : isTomte ? "头饰未超重" : "0.04"]);
    if (!isTomte) {
      assert.equal(await driver.execute(`return document.querySelectorAll('[data-trait="' + arguments[0] + '"] .race-effects-list > li').length;`, [trait]), isEnt || isSpectre ? 8 : 6);
    }
    await driver.execute(`const row = document.querySelector('[data-trait="' + arguments[0] + '"]'); row.open = true; row.scrollIntoView({ block: "center" }); return true;`, [trait]);
    await writeFile(path.join(artifactDirectory, `${raceId}-${build}.png`), await driver.screenshot(), "base64");
    await click(driver, "#player-page-close");
    if (isTomte) {
      await click(driver, "#player-ui-ability-open");
      const beforeProbe = await state();
      await driver.execute(`
        const row = [...document.querySelectorAll(".ability-row")].find(row => row.querySelector(".ability-name")?.textContent === "探测怪物");
        const button = row?.querySelector(".ability-cast-action");
        if (!button || button.disabled) throw new Error("Tomte probe unavailable");
        button.click(); return true;
      `);
      await afterTurn(beforeProbe.turn);
      await driver.execute(`if (document.querySelector("#monster-probe-dialog").open) document.querySelector("#monster-probe-close").click(); return true;`);
      await driver.execute(`if (document.querySelector("#player-page-dialog").open) document.querySelector("#player-page-close").click(); return true;`);
    }
    await click(driver, "#player-ui-inventory-open");
    if (isSpectre) {
      const staffState = () => driver.execute(`
        const rows = [...document.querySelectorAll("#inventory-list .inventory-item")];
        const staff = rows.find(row => row.dataset.itemKindId === "demo.item.staff-of-nothing");
        return { charges: staff.querySelector(".inventory-item-status").textContent,
          nutrition: document.querySelector("#nutrition-value").textContent,
          hasRations: rows.some(row => row.dataset.itemKindId === "demo.item.ration-of-food"),
          hasTorches: rows.some(row => row.textContent.includes("火把")) };
      `);
      const before = await staffState();
      assert.equal(before.hasRations, false);
      assert.equal(before.hasTorches, true);
      await driver.execute(`document.querySelector('#inventory-list [data-item-kind-id="demo.item.staff-of-nothing"] input[type="checkbox"]').click(); return true;`);
      const turn = (await state()).turn;
      await click(driver, "#inventory-absorb");
      await driver.waitFor(`return document.querySelector(".item-target-dialog")?.open`, "absorption device choice");
      await click(driver, ".item-target-dialog .item-target-actions button:last-child");
      await afterTurn(turn);
      const after = await staffState();
      assert.match(before.charges, /21/);
      assert.match(after.charges, /20/);
      assert.match(before.nutrition, /99/);
      assert.match(after.nutrition, /149/);
      await driver.execute(`for (const input of document.querySelectorAll('#inventory-list input[type="checkbox"]:checked')) input.click(); return true;`);
      report.checks.push({ build, absorption: { before, after } });
    }
    if (isEnt) {
      const beforeWater = await driver.execute(`
        const rows = [...document.querySelectorAll("#inventory-list .inventory-item")];
        const water = rows.filter(row => row.dataset.itemKindId === "demo.item.water-potion");
        const quantity = water.reduce((sum, row) => sum + Number(row.querySelector(".inventory-quantity").textContent.match(/\\d+/)[0]), 0);
        const nutritionPercent = Number(document.querySelector("#nutrition-value").textContent.match(/\\d+/)[0]);
        water[0].querySelector('input[type="checkbox"]').click();
        return { quantity, nutritionPercent, hasRations: rows.some(row => row.dataset.itemKindId === "demo.item.ration-of-food"), hasTorches: rows.some(row => row.textContent.includes("火把")) };
      `);
      assert.ok(beforeWater.quantity >= 15 && beforeWater.quantity <= 23);
      assert.equal(beforeWater.hasRations, false);
      assert.equal(beforeWater.hasTorches, true);
      const beforeDrink = await state();
      await click(driver, "#inventory-use");
      await afterTurn(beforeDrink.turn);
      const afterWater = await driver.execute(`return {
        quantity: [...document.querySelectorAll("#inventory-list .inventory-item")].filter(row => row.dataset.itemKindId === "demo.item.water-potion").reduce((sum, row) => sum + Number(row.querySelector(".inventory-quantity").textContent.match(/\\d+/)[0]), 0),
        nutritionPercent: Number(document.querySelector("#nutrition-value").textContent.match(/\\d+/)[0]),
      };`);
      assert.equal(afterWater.quantity, beforeWater.quantity - 1);
      assert.equal(beforeWater.nutritionPercent, 99);
      assert.equal(afterWater.nutritionPercent, 141);
      await driver.execute(`for (const input of document.querySelectorAll('#inventory-list input[type="checkbox"]:checked')) input.click(); return true;`);
      report.checks.push({ build, water: { before: beforeWater, after: afterWater } });
    }
    const equipmentName = await driver.execute(`const row = document.querySelector('#equipment-list [data-slot-id="' + arguments[0] + '"]'); const name = row.querySelector(".equipment-slot-name").textContent; row.querySelector("button").click(); return name;`, [slot]);
    assert.ok(isTomte ? equipmentName.includes("针织帽") : equipmentName.length > 0);
    await driver.waitFor(`return document.querySelector("#inventory-detail-dialog")?.open`, "equipment detail");
    await click(driver, "#inventory-detail-actions .equipment-actions button:last-child");
    await driver.waitFor(`return document.querySelector('#equipment-list [data-slot-id="' + arguments[0] + '"]')?.classList.contains("equipment-slot-vacant")`, "equipment removed", 10_000, [slot]);
    await driver.execute(`
      if (document.querySelector("#inventory-detail-dialog").open) document.querySelector("#inventory-detail-close").click();
      const row = [...document.querySelectorAll("#inventory-list .inventory-item")].find(row => row.querySelector(".inventory-item-name").textContent === arguments[0]);
      row.querySelector('input[type="checkbox"]').click(); return true;
    `, [equipmentName]);
    await click(driver, "#inventory-equip");
    await driver.waitFor(`return document.querySelector('#equipment-list [data-slot-id="' + arguments[0] + '"] .equipment-slot-name')?.textContent === arguments[1]`, "equipment equipped", 10_000, [slot, equipmentName]);
    await click(driver, "#player-page-close");
    if (isSpectre) {
      const wallState = () => driver.execute(`return {
        position: document.querySelector("#position-value").textContent,
        hp: parseInt(document.querySelector("#hp-value").textContent, 10),
        densityMessage: document.body.textContent.includes("密度使你受到了"),
      };`);
      const step = async (code, key) => {
        const turn = (await state()).turn;
        await dispatchKey(driver, code, key);
        await afterTurn(turn);
        return wallState();
      };
      // Outpost's building wall is three tiles north of birth. The continuous
      // wilderness translates the town template, so use the actual birth origin.
      const [birthX, birthY] = (await wallState()).position.split(", ").map(Number);
      const northOfBirth = distance => `${birthX}, ${birthY - distance}`;
      await step("Numpad8", "8");
      const before = await step("Numpad8", "8");
      assert.equal(before.position, northOfBirth(2));
      const inside = await step("Numpad8", "8");
      assert.equal(inside.position, northOfBirth(3));
      assert.ok(inside.hp < before.hp);
      assert.equal(inside.densityMessage, true);
      const waited = await step("Numpad5", "5");
      assert.equal(waited.hp, inside.hp - 1);
      await writeFile(path.join(artifactDirectory, `${raceId}-${build}-wall.png`), await driver.screenshot(), "base64");
      const outside = await step("Numpad2", "2");
      assert.equal(outside.position, northOfBirth(2));
      assert.ok(outside.hp >= waited.hp);
      report.checks.push({ build, wall: { before, inside, waited, outside } });
    }
    await driver.execute(`
      window.__rfbE2eDownloads = [];
      URL.createObjectURL = blob => { window.__rfbE2eDownloads.push({ blob, size: blob.size }); return "blob:race-acceptance"; };
      URL.revokeObjectURL = () => {};
      HTMLAnchorElement.prototype.click = function () { window.__rfbE2eDownloads.at(-1).fileName = this.download; };
      document.querySelector(".hud-menu").open = true; return true;
    `);
    const saved = await state();
    await click(driver, "#save-button");
    await driver.waitFor(`return window.__rfbE2eDownloads.some(item => item.fileName?.endsWith(".rfbsave"))`, "race save export");
    assert.ok((await lastDownload(driver)).size > 100);
    await driver.execute(`document.querySelector(".hud-menu").open = false; return true;`);
    await dispatchKey(driver, "Numpad5", "5"); await afterTurn(saved.turn);
    await driver.execute(`
      const saved = window.__rfbE2eDownloads.find(item => item.fileName?.endsWith(".rfbsave"));
      const transfer = new DataTransfer(); transfer.items.add(new File([saved.blob], saved.fileName));
      const input = document.querySelector("#load-input"); input.files = transfer.files;
      input.dispatchEvent(new Event("change", { bubbles: true })); return true;
    `);
    await driver.waitFor(`return document.querySelector("#hash-value")?.title === arguments[0]`, "race exact restore", 10_000, [saved.hash]);
    assert.deepEqual(await state(), saved);
    await dispatchKey(driver, "Numpad5", "5"); await afterTurn(saved.turn);
    assert.deepEqual((await state()).errors, []);
    report.checks.push({ build, race: saved.race, saveHash: saved.hash, checks: ["create", "Chinese description", "racial traits", ...(isSpectre ? ["birth staff/torches/no rations", "absorb device", "enter wall", "wait for density damage", "leave wall"] : isTomte ? ["probe"] : isEnt ? ["birth water/torches/no rations", "drink water"] : []), "unequip/equip", "save/restore", "continue"] });
    process.stdout.write(`${raceId}: ${build} passed.\n`);
    if (build !== builds.at(-1)) {
      await driver.execute(`setTimeout(() => location.reload(), 100); return true;`);
      await driver.waitFor(`return document.documentElement.dataset.appMode === "title"`, "next race class", 60_000);
    }
  }
  await writeFile(path.join(artifactDirectory, `${raceId}-acceptance.json`), JSON.stringify(report, null, 2) + "\n");
}

async function dispatchKey(driver, code, key) {
  await driver.execute(`
    window.dispatchEvent(new KeyboardEvent("keydown", {
      code: arguments[0],
      key: arguments[1],
      bubbles: true,
    }));
    return true;
  `, [code, key]);
}

async function runLifeForceScenario(driver) {
  const expected = await loadExpectedIdentity();
  const report = { identity: expected, precondition: "Level 19, life force 1, awake original barrow-wight east of player, daylight suppressed on both tiles, simulation RNG seed 0. No forced hit, damage, saving throw or conversion target.", checks: [] };
  const state = () => driver.execute(`return {
    hash: document.querySelector("#hash-value").title,
    turn: parseInt(document.querySelector("#turn-value").textContent, 10),
    race: document.querySelector("#character-race-value").textContent,
    class: document.querySelector("#character-class-value").textContent,
    level: document.querySelector("#character-level-value").textContent,
    hp: document.querySelector("#hp-value").textContent,
    lifeForce: document.querySelector("#health-meter").title,
    position: document.querySelector("#position-value").textContent,
    nutrition: document.querySelector("#nutrition-value").textContent,
    light: document.querySelector("#light-value").textContent,
    equipment: [...document.querySelectorAll("#equipment-list .equipment-slot-name")].map(node => node.textContent),
    inventory: [...document.querySelectorAll("#inventory-list [data-item-id]")].map(node => [node.dataset.itemId, node.dataset.itemKindId]),
    abilities: [...document.querySelectorAll("#ability-list .ability-name")].map(node => node.textContent),
    alive: document.documentElement.dataset.playerState === "alive",
    errors: window.__lifeForceErrors,
  };`);
  const afterTurn = turn => driver.waitFor(`return parseInt(document.querySelector("#turn-value").textContent, 10) > arguments[0]`, "life force action", 10_000, [turn]);
  await driver.waitFor(`return document.documentElement.dataset.appMode === "title"`, "life force title", 60_000);
  await driver.execute(`localStorage.setItem("rfb.locale", "zh-CN"); localStorage.setItem("rfb.input-preset", "numpad"); setTimeout(() => location.reload(), 100); return true;`);
  await driver.waitFor(`return performance.getEntriesByType("navigation")[0]?.type === "reload" && document.documentElement.dataset.appMode === "title"`, "Chinese title", 60_000);
  await mkdir(artifactDirectory, { recursive: true });
  for (const [build, raceId, targetName] of [
    ["warrior", "demo.race.rfb-human", "吸血鬼"],
    ["high-mage-death", "rfb-legacy.race.imp", "幽灵"],
    ["archer", "demo.race.rfb-human", "吸血鬼"],
  ]) {
    await click(driver, "#session-new-game");
    await selectCreationBuild(driver, "demo.build." + build);
    await selectCreationRace(driver, raceId);
    await driver.execute(`
      window.__lifeForceErrors = [];
      window.addEventListener("error", event => window.__lifeForceErrors.push(event.message));
      document.querySelector("#session-seed").value = "83";
      document.querySelector("#session-character-name").value = "生命力验收";
      document.querySelector("#session-start-game").click(); return true;
    `, [build, raceId]);
    await driver.waitFor(`return document.documentElement.dataset.appMode === "playing" && document.querySelector("#connection-status").classList.contains("ready")`, "life force creation", 60_000);
    await driver.execute(`window.__lifeForcePrepared = false; window.__rfbPrepareLifeForceE2e(0).then(() => window.__lifeForcePrepared = true).catch(error => window.__lifeForceErrors.push(String(error))); return true;`);
    await driver.waitFor(`return window.__lifeForcePrepared`, "explicit life force precondition");
    assert.equal(await driver.execute(`return document.querySelector("#map-host").dataset.protocolVersion`), expected.protocolVersion);
    assert.equal(await driver.execute(`return document.querySelector("#map-host").dataset.contentHash`), expected.contentHash);
    const before = await state();
    assert.equal(before.lifeForce, "生命力：1 / 1000");
    assert.equal(before.level, "19");
    if (raceId.endsWith(".imp")) assert.ok(before.abilities.includes("火焰箭/火球术"));
    for (let attack = 0; attack < 5 && (await state()).race !== targetName; attack += 1) {
      const turn = (await state()).turn;
      await dispatchKey(driver, "Numpad5", "5");
      await afterTurn(turn);
    }
    const transformed = await state();
    assert.equal(transformed.race, targetName);
    assert.equal(transformed.alive, true);
    assert.equal(transformed.class, before.class);
    assert.equal(transformed.lifeForce, "生命力：1000 / 1000");
    assert.deepEqual(transformed.equipment, before.equipment);
    assert.deepEqual(transformed.inventory, before.inventory);
    if (raceId.endsWith(".imp")) {
      assert.ok(!transformed.abilities.includes("火焰箭/火球术"));
      assert.ok(transformed.abilities.includes("恐吓怪物"));
    } else {
      assert.ok(transformed.abilities.includes("吸血"));
    }
    const messages = await driver.execute(`return [...document.querySelectorAll("#message-list li")].map(node => node.textContent);`);
    const exhausted = messages.findIndex(text => text.includes("你的生命力枯竭了"));
    const changed = messages.findIndex(text => text.includes("永久转化为" + targetName));
    const restored = messages.findIndex(text => text.includes("生命力恢复至1000"));
    assert.ok(exhausted >= 0 && exhausted < changed && changed < restored, messages.join("\n"));
    assert.ok(!messages.some(text => /\[[a-z][a-z-]+\]|未知事件|unknown event/i.test(text)), messages.join("\n"));
    await click(driver, "#player-ui-character-open");
    await click(driver, "#character-tab-overview");
    assert.equal(await driver.execute(`return [...document.querySelectorAll("#character-vitals-list > div")].find(row => row.querySelector("dt").textContent === "生命力").querySelector("dd").textContent;`), "1000 / 1000");
    await writeFile(path.join(artifactDirectory, `life-force-${build}-character.png`), await driver.screenshot(), "base64");
    await click(driver, "#player-page-close");
    await click(driver, "#player-ui-ability-open");
    const powerName = targetName === "幽灵" ? "恐吓怪物" : "吸血";
    await driver.execute(`
      const row = [...document.querySelectorAll("#ability-list .ability-row")].find(row => row.querySelector(".ability-name").textContent === arguments[0]);
      if (row.querySelector(".ability-cast-action").disabled) throw new Error("New racial power unavailable");
      row.scrollIntoView({ block: "center" }); return true;
    `, [powerName]);
    await writeFile(path.join(artifactDirectory, `life-force-${build}-abilities.png`), await driver.screenshot(), "base64");
    await driver.execute(`
      const row = [...document.querySelectorAll("#ability-list .ability-row")].find(row => row.querySelector(".ability-name").textContent === arguments[0]);
      row.querySelector(".ability-cast-action").click(); return true;
    `, [powerName]);
    await driver.waitFor(`return document.querySelector("#map-host").dataset.targetingAction === "ability"`, "new racial targeting");
    await dispatchKey(driver, "Escape", "Escape");
    await driver.waitFor(`return document.querySelector("#map-host").dataset.targetingAction === "none"`, "cancel racial targeting");
    const turn = (await state()).turn;
    await dispatchKey(driver, "Numpad4", "4"); await afterTurn(turn);
    await driver.execute(`
      window.__rfbE2eDownloads = [];
      URL.createObjectURL = blob => { window.__rfbE2eDownloads.push({ blob, size: blob.size }); return "blob:life-force-acceptance"; };
      URL.revokeObjectURL = () => {};
      HTMLAnchorElement.prototype.click = function () { window.__rfbE2eDownloads.at(-1).fileName = this.download; };
      document.querySelector(".hud-menu").open = true; return true;
    `);
    const saved = await state();
    await click(driver, "#save-button");
    await driver.waitFor(`return window.__rfbE2eDownloads.some(item => item.fileName?.endsWith(".rfbsave"))`, "life force save export");
    await driver.execute(`document.querySelector(".hud-menu").open = false; return true;`);
    await dispatchKey(driver, "Numpad4", "4"); await afterTurn(saved.turn);
    const continued = await state();
    await driver.execute(`
      const saved = window.__rfbE2eDownloads.find(item => item.fileName?.endsWith(".rfbsave"));
      const transfer = new DataTransfer(); transfer.items.add(new File([saved.blob], saved.fileName));
      const input = document.querySelector("#load-input"); input.files = transfer.files;
      input.dispatchEvent(new Event("change", { bubbles: true })); return true;
    `);
    await driver.waitFor(`return document.querySelector("#hash-value").title === arguments[0]`, "life force exact restore", 10_000, [saved.hash]);
    assert.deepEqual(await state(), saved);
    await dispatchKey(driver, "Numpad4", "4"); await afterTurn(saved.turn);
    assert.deepEqual(await state(), continued);
    assert.equal(continued.alive, true);
    assert.deepEqual(continued.errors, []);
    report.checks.push({ build, before, transformed, saved, continued, messages });
    process.stdout.write(`Life force: ${build} -> ${targetName} passed.\n`);
    if (build !== "archer") {
      await driver.execute(`setTimeout(() => location.reload(), 100); return true;`);
      await driver.waitFor(`return document.documentElement.dataset.appMode === "title"`, "next life force class", 60_000);
    }
  }
  await writeFile(path.join(artifactDirectory, "life-force-acceptance.json"), JSON.stringify(report, null, 2) + "\n");
}

async function click(driver, selector) {
  await driver.execute(`document.querySelector(arguments[0]).click(); return true;`, [selector]);
}

async function lastDownload(driver) {
  return driver.execute(`
    const item = window.__rfbE2eDownloads.at(-1);
    return { fileName: item.fileName, size: item.size };
  `);
}

class WebDriverClient {
  constructor(port, sessionId) {
    this.baseUrl = `http://127.0.0.1:${port}`;
    this.sessionId = sessionId;
  }

  static async create(port, app, timeoutMs = 15_000) {
    const deadline = Date.now() + timeoutMs;
    let lastError;
    while (Date.now() < deadline) {
      if (app.exitCode !== null || app.signalCode !== null) {
        throw new Error(
          `Tauri application exited before its main window was available (${app.exitCode ?? app.signalCode})`,
        );
      }
      try {
        const response = await request(port, "POST", "/session", {
          capabilities: { alwaysMatch: { "wdio:tauriServiceOptions": { windowLabel: "main" } } },
        });
        return new WebDriverClient(port, response.sessionId);
      } catch (error) {
        lastError = error;
        if (!String(error).includes("no such window")) throw error;
        await delay(100);
      }
    }
    throw new Error(`Timed out waiting for the Tauri main window: ${String(lastError)}`);
  }

  async execute(script, args = []) {
    return this.command("POST", "/execute/sync", { script, args });
  }

  async waitFor(script, description, timeoutMs = 10_000, args = []) {
    const deadline = Date.now() + timeoutMs;
    let lastError;
    while (Date.now() < deadline) {
      try {
        if (await this.execute(script, args)) return;
      } catch (error) {
        lastError = error;
      }
      await delay(100);
    }
    throw new Error(`Timed out waiting for ${description}${lastError ? `: ${lastError}` : ""}`);
  }

  async screenshot() {
    return this.command("GET", "/screenshot");
  }

  async close() {
    await requestUrl(this.baseUrl, "DELETE", `/session/${this.sessionId}`);
  }

  async command(method, suffix, body) {
    return requestUrl(this.baseUrl, method, `/session/${this.sessionId}${suffix}`, body);
  }
}

async function request(port, method, route, body) {
  return requestUrl(`http://127.0.0.1:${port}`, method, route, body);
}

async function requestUrl(baseUrl, method, route, body) {
  const response = await fetch(`${baseUrl}${route}`, {
    method,
    headers: body === undefined ? undefined : { "content-type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const responseText = await response.text();
  let payload;
  try {
    payload = JSON.parse(responseText);
  } catch {
    throw new Error(`${method} ${route}: HTTP ${response.status}: ${responseText}`);
  }
  if (!response.ok) {
    throw new Error(`${method} ${route}: ${payload.value?.error}: ${payload.value?.message}`);
  }
  return payload.value;
}

async function waitForServer(port, app, timeoutMs = 15_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (app.exitCode !== null || app.signalCode !== null) {
      throw new Error(`Tauri application exited before WebDriver started (${app.exitCode ?? app.signalCode})`);
    }
    try {
      await request(port, "GET", "/status");
      return;
    } catch {
      await delay(100);
    }
  }
  throw new Error("Timed out waiting for embedded Tauri WebDriver server");
}

function reservePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.unref();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      server.close(() => resolve(address.port));
    });
  });
}

function captureOutput(stream, label) {
  stream.setEncoding("utf8");
  stream.on("data", (chunk) => {
    for (const line of chunk.split(/\r?\n/)) {
      if (line) logs.push(`[${label}] ${line}`);
    }
  });
}

function delay(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

await main();
