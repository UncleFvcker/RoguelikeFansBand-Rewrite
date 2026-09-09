// SPDX-License-Identifier: MPL-2.0

import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import net from "node:net";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { runRendererProfile } from "./render-profile.e2e.mjs";

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
    child = spawn(executable, [], {
      cwd: repositoryDirectory,
      env: {
        ...process.env,
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
    } else if (tomteOnly) {
      await runTomteScenario(client);
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
      renderProfileOnly ? "Renderer profile passed.\n" : tomteOnly ? "Tomte desktop acceptance passed.\n" : "Tauri desktop E2E passed.\n",
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
      const build = document.querySelector(arguments[0]); build.checked = true;
      build.dispatchEvent(new Event("change", { bubbles: true }));
      const seed = document.querySelector("#session-seed"); seed.value = arguments[1];
      seed.dispatchEvent(new Event("input", { bubbles: true }));
      document.querySelector("#session-start-game").click(); return true;
    `, [build, seed]);
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

  await start("#session-build-warrior", "42");
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
  await start("#session-build-high-mage-death", "7");
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

async function runTomteScenario(driver) {
  const expected = await loadExpectedIdentity();
  const report = { identity: expected, checks: [] };
  const state = () => driver.execute(`return {
    hash: document.querySelector("#hash-value")?.title,
    turn: parseInt(document.querySelector("#turn-value")?.textContent, 10),
    equipment: document.querySelector("#equipment-list")?.textContent,
    race: document.querySelector("#character-race-value")?.textContent,
    errors: window.__tomteErrors ?? [],
  };`);
  const afterTurn = turn => driver.waitFor(`return parseInt(document.querySelector("#turn-value")?.textContent, 10) > arguments[0]`, "Tomte action committed", 10_000, [turn]);
  await driver.waitFor(`return document.documentElement.dataset.appMode === "title"`, "Tomte title", 60_000);
  await driver.execute(`localStorage.setItem("rfb.locale", "zh-CN"); setTimeout(() => location.reload(), 100); return true;`);
  await driver.waitFor(`return performance.getEntriesByType("navigation")[0]?.type === "reload" && document.documentElement.dataset.appMode === "title"`, "Chinese title", 60_000);
  await mkdir(artifactDirectory, { recursive: true });
  for (const build of ["warrior", "high-mage-death", "archer"]) {
    await click(driver, "#session-new-game");
    const description = await driver.execute(`
      window.__tomteErrors = [];
      window.addEventListener("error", event => window.__tomteErrors.push(event.message));
      const race = document.querySelector("#session-race"); race.value = "rfb-legacy.race.tomte";
      race.dispatchEvent(new Event("change", { bubbles: true }));
      const build = document.querySelector("#session-build-" + arguments[0]); build.checked = true;
      build.dispatchEvent(new Event("change", { bubbles: true }));
      document.querySelector("#session-seed").value = "83";
      document.querySelector("#session-character-name").value = "托姆特验收";
      const note = document.querySelector("#session-tomte-description");
      return { visible: !note.hidden, text: note.textContent, name: race.selectedOptions[0].textContent };
    `, [build]);
    assert.equal(description.name, "托姆特");
    assert.equal(description.visible, true);
    assert.ok(description.text.includes("1.0 磅") && description.text.includes("40"));
    await click(driver, "#session-start-game");
    await driver.waitFor(`return document.documentElement.dataset.appMode === "playing" && document.querySelector("#connection-status")?.classList.contains("ready")`, "Tomte creation", 60_000);
    assert.equal((await state()).race, "托姆特");
    assert.equal(await driver.execute(`return document.querySelector("#map-host").dataset.contentHash;`), expected.contentHash);
    assert.equal(await driver.execute(`return document.querySelector("#map-host").dataset.protocolVersion;`), expected.protocolVersion);
    await click(driver, "#player-ui-character-open");
    await click(driver, "#character-tab-details");
    await click(driver, "#character-detail-tab-defenses");
    await driver.waitFor(`return document.querySelector('[data-trait="tomte-headgear"]')?.textContent.includes("头饰未超重")`, "Tomte headgear projection");
    await driver.execute(`const row = document.querySelector('[data-trait="tomte-headgear"]'); row.open = true; row.scrollIntoView({ block: "center" }); return true;`);
    await writeFile(path.join(artifactDirectory, `tomte-${build}.png`), await driver.screenshot(), "base64");
    await click(driver, "#player-page-close");
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
    await click(driver, "#player-ui-inventory-open");
    const cap = await driver.execute(`const row = document.querySelector('#equipment-list [data-slot-id="head"]'); const name = row.querySelector(".equipment-slot-name").textContent; row.querySelector("button").click(); return name;`);
    assert.ok(cap.includes("针织帽"));
    await driver.waitFor(`return document.querySelector("#inventory-detail-dialog")?.open`, "cap detail");
    await click(driver, "#inventory-detail-actions .equipment-actions button:last-child");
    await driver.waitFor(`return document.querySelector('#equipment-list [data-slot-id="head"]')?.classList.contains("equipment-slot-vacant")`, "cap removed");
    await driver.execute(`
      if (document.querySelector("#inventory-detail-dialog").open) document.querySelector("#inventory-detail-close").click();
      const row = [...document.querySelectorAll("#inventory-list .inventory-item")].find(row => row.querySelector(".inventory-item-name").textContent === arguments[0]);
      row.querySelector('input[type="checkbox"]').click(); return true;
    `, [cap]);
    await click(driver, "#inventory-equip");
    await driver.waitFor(`return document.querySelector('#equipment-list [data-slot-id="head"] .equipment-slot-name')?.textContent === arguments[0]`, "cap equipped", 10_000, [cap]);
    await click(driver, "#player-page-close");
    await driver.execute(`
      window.__rfbE2eDownloads = [];
      URL.createObjectURL = blob => { window.__rfbE2eDownloads.push({ blob, size: blob.size }); return "blob:tomte-acceptance"; };
      URL.revokeObjectURL = () => {};
      HTMLAnchorElement.prototype.click = function () { window.__rfbE2eDownloads.at(-1).fileName = this.download; };
      document.querySelector(".hud-menu").open = true; return true;
    `);
    const saved = await state();
    await click(driver, "#save-button");
    await driver.waitFor(`return window.__rfbE2eDownloads.some(item => item.fileName?.endsWith(".rfbsave"))`, "Tomte save export");
    assert.ok((await lastDownload(driver)).size > 100);
    await driver.execute(`document.querySelector(".hud-menu").open = false; return true;`);
    await dispatchKey(driver, "Numpad5", "5"); await afterTurn(saved.turn);
    await driver.execute(`
      const saved = window.__rfbE2eDownloads.find(item => item.fileName?.endsWith(".rfbsave"));
      const transfer = new DataTransfer(); transfer.items.add(new File([saved.blob], saved.fileName));
      const input = document.querySelector("#load-input"); input.files = transfer.files;
      input.dispatchEvent(new Event("change", { bubbles: true })); return true;
    `);
    await driver.waitFor(`return document.querySelector("#hash-value")?.title === arguments[0]`, "Tomte exact restore", 10_000, [saved.hash]);
    assert.deepEqual(await state(), saved);
    await dispatchKey(driver, "Numpad5", "5"); await afterTurn(saved.turn);
    assert.deepEqual((await state()).errors, []);
    report.checks.push({ build, race: saved.race, saveHash: saved.hash, checks: ["create", "Chinese description", "headgear hint", "probe", "unequip/equip", "save/restore", "continue"] });
    await driver.execute(`setTimeout(() => location.reload(), 100); return true;`);
    await driver.waitFor(`return document.documentElement.dataset.appMode === "title"`, "next Tomte class", 60_000);
  }
  await writeFile(path.join(artifactDirectory, "tomte-acceptance.json"), JSON.stringify(report, null, 2) + "\n");
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
  const payload = await response.json();
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
