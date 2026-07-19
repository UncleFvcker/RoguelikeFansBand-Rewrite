// SPDX-License-Identifier: MPL-2.0

import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdir, rm, writeFile } from "node:fs/promises";
import net from "node:net";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const webDirectory = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repositoryDirectory = path.resolve(webDirectory, "..");
const executable = path.join(repositoryDirectory, "target", "debug", "rfb-tauri.exe");
const artifactDirectory = path.join(repositoryDirectory, "test-results");
const diagnosticDirectory = path.join(artifactDirectory, "e2e-crash-diagnostics");
const desktopLogPath = path.join(artifactDirectory, "e2e-rfb-desktop.log");
const renderProfilePath = path.join(artifactDirectory, "render-profile.json");
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
    await rm(renderProfilePath, { force: true });
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
    await runScenario(client);
    if (process.env.RFB_E2E_CAPTURE_SCREENSHOT === "1") {
      await mkdir(artifactDirectory, { recursive: true });
      await writeFile(
        path.join(artifactDirectory, "tauri-e2e-success.png"),
        await client.screenshot(),
        "base64",
      );
    }
    process.stdout.write("Tauri desktop E2E passed.\n");
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
    if (client) await cleanupNativeTestSaves(client).catch(() => undefined);
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
  await driver.waitFor(
    `return document.querySelector("#connection-status")?.classList.contains("ready")`,
    "native core connection",
  );
  await driver.execute(`
    localStorage.clear();
    localStorage.setItem("rfb.locale", "zh-CN");
    localStorage.setItem("rfb.renderer-profile-enabled", "1");
    setTimeout(() => window.location.reload(), 0);
    return true;
  `);
  await driver.waitFor(
    `return performance.getEntriesByType("navigation")[0]?.type === "reload" && document.querySelector("#connection-status")?.classList.contains("ready")`,
    "deterministic application reload",
  );

  await driver.execute(`
    const input = document.querySelector("#input-preset");
    input.value = "numpad";
    input.dispatchEvent(new Event("change", { bubbles: true }));
    const tileset = document.querySelector("#tileset-preset");
    if (tileset.value !== "ascii") {
      tileset.value = "ascii";
      tileset.dispatchEvent(new Event("change", { bubbles: true }));
    }
    const camera = document.querySelector("#camera-mode");
    camera.value = "full-map";
    camera.dispatchEvent(new Event("change", { bubbles: true }));
    const zoom = document.querySelector("#zoom-level");
    zoom.value = "1";
    zoom.dispatchEvent(new Event("change", { bubbles: true }));
    window.__rfbE2eCanvas = document.querySelector("#map-host canvas");
    return true;
  `);
  await driver.waitFor(
    `return document.querySelector("#map-host")?.dataset.tilesetId === "rfb.tileset.ascii-default"`,
    "ASCII tileset normalization",
  );

  let state = await readState(driver);
  assert.equal(state.turn, "0");
  assert.equal(state.position, "3, 3");
  assert.equal(state.renderKind, "snapshot");
  assert.equal(state.appliedCells, "400");
  assert.equal(state.tilesetId, "rfb.tileset.ascii-default");
  assert.equal(state.rendererBackend, "pixi-layered-chunks-v3");
  assert.equal(state.rendererLayerCount, "5");
  assert.equal(state.rendererLayers, "terrain,object,actor,visibility,lighting");
  assert.equal(state.terrainMode, "chunk-render-texture-v1");
  assert.equal(state.dynamicViewMode, "visible-chunk-reuse-v1");
  assert.equal(state.terrainChunkSize, "16");
  assert.equal(state.terrainChunkCount, "4");
  assert.equal(state.visibleChunkCount, "4");
  assert.equal(state.culledChunkCount, "0");
  assert.equal(state.lastRebuiltTerrainChunks, "4");
  assert.equal(state.totalRebuiltTerrainChunks, "4");
  assert.equal(state.rendererCellViewCount, "400");
  assert.equal(state.rendererDynamicDisplayObjectCount, "2800");
  assert.equal(state.activeDynamicChunkCount, "4");
  assert.equal(state.pooledDynamicChunkCount, "0");
  assert.equal(state.visibilityMode, "rust-fov-memory-v1");
  assert.equal(state.lightingMode, "rust-content-lights-v1");
  assert.equal(state.protocolVersion, "1.18");
  assert.equal(state.visualCellCount, "400");
  assert.ok(Number(state.visibleCellCount) > 0);
  assert.equal(state.rememberedCellCount, "0");
  assert.ok(Number(state.hiddenCellCount) > 0);
  assert.equal(state.cameraMode, "full-map");
  assert.equal(state.cameraX, "0");
  assert.equal(state.cameraY, "0");
  assert.equal(state.viewportWidth, "560");
  assert.equal(state.viewportHeight, "560");
  assert.equal(state.zoom, "1");
  assert.equal(state.canvasUnchanged, true);
  assert.equal(state.contentId, "rfb.demo.original-v1");
  assert.equal(
    state.contentHash,
    "cb56a8e9dd6d7280b38fe4e388fc0f7ce08fd4a40cef2c8886907e3c662ffc96",
  );
  assert.equal(state.worldId, "demo.world.original-v1");
  assert.equal(state.contentVisualCount, "14");
  assert.equal(state.itemCount, "5");
  assert.equal(state.inventoryStackCount, "0");
  assert.equal(state.equipmentCount, "0");
  assert.equal(state.playerStatusCount, "0");
  assert.equal(state.effects, "无");
  assert.equal(state.attack, "2");
  assert.equal(state.defense, "1");
  assert.match(state.inventory, /背包是空的/);

  await dispatchKey(driver, "Numpad5", "5");
  await driver.waitFor(`return document.querySelector("#turn-value")?.textContent === "1"`, "wait command");
  state = await readState(driver);
  assert.equal(state.position, "3, 3");
  assert.equal(state.renderKind, "update");
  assert.equal(state.appliedCells, "0");
  assert.equal(state.lastRebuiltTerrainChunks, "0");
  assert.equal(state.totalRebuiltTerrainChunks, "4");
  assert.equal(state.canvasUnchanged, true);
  assert.match(state.messages, /你在寂静中停留了一回合/);

  await dispatchKey(driver, "Numpad6", "6");
  await driver.waitFor(
    `return document.querySelector("#position-value")?.textContent === "4, 3"`,
    "east movement",
  );
  state = await readState(driver);
  assert.equal(state.turn, "2");
  assert.equal(state.renderKind, "update");
  assert.equal(state.appliedCells, "99");
  assert.equal(state.lastRebuiltTerrainChunks, "0");
  assert.equal(state.canvasUnchanged, true);

  await dispatchKey(driver, "KeyG", "g");
  await driver.waitFor(
    `return document.querySelector("#turn-value")?.textContent === "3" && document.querySelector("#inventory-count")?.textContent === "1 堆"`,
    "ground item pickup",
  );
  state = await readState(driver);
  assert.equal(state.renderKind, "update");
  assert.equal(state.appliedCells, "1");
  assert.equal(state.itemCount, "4");
  assert.equal(state.inventoryStackCount, "1");
  assert.match(state.inventory, /发光碎片/);
  assert.match(state.inventory, /×5/);
  assert.match(state.messages, /你将 5 个发光碎片收入了背包/);

  const nativeSaveName = `E2E 原生存档 ${Date.now()}`;
  const nativeSaveHash = state.stateHash;
  await driver.execute(`
    const input = document.querySelector("#native-save-name");
    input.value = arguments[0];
    input.dispatchEvent(new Event("input", { bubbles: true }));
    document.querySelector("#native-save-create").click();
    return true;
  `, [nativeSaveName]);
  await driver.waitFor(
    `return [...document.querySelectorAll(".native-save-item")].some((row) => row.querySelector(".native-save-name")?.textContent === arguments[0])`,
    "native save creation",
    10_000,
    [nativeSaveName],
  );
  const nativeSlot = await driver.execute(`
    const row = [...document.querySelectorAll(".native-save-item")]
      .find((item) => item.querySelector(".native-save-name")?.textContent === arguments[0]);
    return {
      slotId: row.dataset.slotId,
      status: row.querySelector(".native-save-status")?.textContent,
      metadata: row.querySelector(".native-save-meta")?.textContent,
    };
  `, [nativeSaveName]);
  assert.match(nativeSlot.slotId, /^save-[0-9]+(?:-[0-9]+)?$/);
  assert.equal(nativeSlot.status, "可用");
  assert.match(nativeSlot.metadata, /原创实验场/);
  assert.match(nativeSlot.metadata, /回合 3/);
  assert.match((await readState(driver)).messages, /已创建原生存档/);

  await dispatchKey(driver, "Numpad2", "2");
  await driver.waitFor(
    `return document.querySelector("#position-value")?.textContent === "4, 4" && document.querySelector("#turn-value")?.textContent === "4"`,
    "movement after native save",
  );
  await click(driver, `[data-slot-id="${nativeSlot.slotId}"] [data-native-save-action="load"]`);
  await driver.waitFor(
    `return document.querySelector("#position-value")?.textContent === "4, 3" && document.querySelector("#turn-value")?.textContent === "3" && !document.querySelector('[data-slot-id="${nativeSlot.slotId}"] [data-native-save-action="load"]')?.disabled`,
    "native save restore",
  );
  state = await readState(driver);
  assert.equal(state.stateHash, nativeSaveHash);
  assert.equal(state.canvasUnchanged, true);
  assert.match(state.messages, /已载入原生存档/);

  await dispatchKey(driver, "Numpad5", "5");
  await driver.waitFor(
    `return document.querySelector("#turn-value")?.textContent === "4"`,
    "command sequence after native restore",
  );
  await click(driver, `[data-slot-id="${nativeSlot.slotId}"] [data-native-save-action="load"]`);
  await driver.waitFor(
    `return document.querySelector("#turn-value")?.textContent === "3" && !document.querySelector('[data-slot-id="${nativeSlot.slotId}"] [data-native-save-action="overwrite"]')?.disabled`,
    "second native save restore",
  );
  await click(
    driver,
    `[data-slot-id="${nativeSlot.slotId}"] [data-native-save-action="overwrite"]`,
  );
  await driver.waitFor(
    `return document.querySelector("#message-list")?.textContent.includes("已安全覆盖原生存档")`,
    "native save overwrite",
  );
  await driver.execute(`window.confirm = () => true; return true;`);
  await click(driver, `[data-slot-id="${nativeSlot.slotId}"] [data-native-save-action="delete"]`);
  await driver.waitFor(
    `return !document.querySelector('[data-slot-id="${nativeSlot.slotId}"]')`,
    "native save deletion",
  );
  state = await readState(driver);
  assert.match(state.messages, /已删除原生存档/);

  await driver.execute(`
    const downloads = [];
    window.__rfbE2eDownloads = downloads;
    URL.createObjectURL = (blob) => {
      downloads.push({ blob, fileName: "", size: blob.size });
      return "blob:rfb-e2e-" + downloads.length;
    };
    URL.revokeObjectURL = () => {};
    HTMLAnchorElement.prototype.click = function () {
      const download = downloads.at(-1);
      if (download) download.fileName = this.download;
    };
    return true;
  `);

  await click(driver, "#save-button");
  await driver.waitFor(
    `return window.__rfbE2eDownloads?.some((item) => item.fileName.endsWith(".rfbsave"))`,
    "save export",
  );
  let download = await lastDownload(driver);
  assert.equal(download.fileName, "rfb-rewrite-demo.rfbsave");
  assert.ok(download.size > 100, `save is unexpectedly small: ${download.size}`);
  assert.match((await readState(driver)).messages, /已导出带校验和的 \.rfbsave 存档/);

  await driver.execute(`
    const row = document.querySelector('[data-item-id="demo.item.luminous-shard.1"]');
    row.querySelector('input[type="checkbox"]').click();
    const quantity = document.querySelector("#inventory-drop-quantity");
    quantity.value = "2";
    quantity.dispatchEvent(new Event("input", { bubbles: true }));
    return true;
  `);
  await click(driver, "#inventory-drop");
  await driver.waitFor(
    `return document.querySelector("#turn-value")?.textContent === "4" && document.querySelector('[data-item-id="demo.item.luminous-shard.1"] .inventory-quantity')?.textContent === "×3"`,
    "partial stack drop",
  );
  state = await readState(driver);
  assert.equal(state.itemCount, "5");
  assert.equal(state.inventoryStackCount, "1");
  assert.match(state.messages, /丢下了 1 堆物品，共 2 件/);

  await dispatchKey(driver, "Numpad6", "6");
  await driver.waitFor(
    `return document.querySelector("#position-value")?.textContent === "5, 3" && document.querySelector("#turn-value")?.textContent === "5"`,
    "movement to equippable item",
  );
  await dispatchKey(driver, "KeyG", "g");
  await driver.waitFor(
    `return document.querySelector("#turn-value")?.textContent === "6" && document.querySelector("#inventory-count")?.textContent === "2 堆"`,
    "second item pickup",
  );
  state = await readState(driver);
  assert.equal(state.itemCount, "4");
  assert.equal(state.inventoryStackCount, "2");
  assert.match(state.inventory, /回声护符/);
  assert.match(state.inventory, /可装备：护符/);
  assert.match(state.inventory, /攻击 \+1/);
  assert.match(state.inventory, /防御 \+1/);
  assert.match(state.inventory, /最大生命 \+4/);

  await driver.execute(`
    for (const checkbox of document.querySelectorAll('#inventory-list input[type="checkbox"]')) {
      if (checkbox.checked) checkbox.click();
    }
    const row = document.querySelector('[data-item-id="demo.item.echo-charm.1"]');
    const checkbox = row.querySelector('input[type="checkbox"]');
    checkbox.click();
    return true;
  `);
  await click(driver, "#inventory-equip");
  await driver.waitFor(
    `return document.querySelector("#turn-value")?.textContent === "7" && document.querySelector("#map-host")?.dataset.equipmentCount === "1"`,
    "equipment action",
  );
  state = await readState(driver);
  assert.equal(state.inventoryStackCount, "1");
  assert.equal(state.equipmentCount, "1");
  assert.match(state.equipment, /回声护符/);
  assert.match(state.equipment, /攻击 \+1/);
  assert.match(state.equipment, /防御 \+1/);
  assert.match(state.equipment, /最大生命 \+4/);
  assert.match(state.health, /10 \/ 14（装备 \+4）/);
  assert.equal(state.attack, "3（装备 +1）");
  assert.equal(state.defense, "2（装备 +1）");
  assert.match(state.messages, /装备在护符槽位/);

  await click(driver, '[data-slot-id="charm"] button');
  await driver.waitFor(
    `return document.querySelector("#turn-value")?.textContent === "8" && document.querySelector("#map-host")?.dataset.equipmentCount === "0"`,
    "unequipment action",
  );
  state = await readState(driver);
  assert.equal(state.inventoryStackCount, "2");
  assert.equal(state.health, "10 / 10");
  assert.equal(state.attack, "2");
  assert.equal(state.defense, "1");
  assert.match(state.messages, /卸下了回声护符/);

  await driver.execute(`
    for (const checkbox of document.querySelectorAll('#inventory-list input[type="checkbox"]')) {
      if (!checkbox.checked) checkbox.click();
    }
    return true;
  `);
  await driver.waitFor(
    `return document.querySelector("#inventory-selection-count")?.textContent === "已选择 2 堆"`,
    "multi-item selection",
  );
  await click(driver, "#inventory-drop");
  await driver.waitFor(
    `return document.querySelector("#turn-value")?.textContent === "9" && document.querySelector("#inventory-count")?.textContent === "0 堆"`,
    "batch item drop",
  );
  state = await readState(driver);
  assert.equal(state.itemCount, "6");
  assert.equal(state.inventoryStackCount, "0");
  assert.match(state.messages, /丢下了 2 堆物品，共 4 件/);

  await driver.execute(`
    const saved = window.__rfbE2eDownloads.find((item) => item.fileName.endsWith(".rfbsave"));
    const input = document.querySelector("#load-input");
    const transfer = new DataTransfer();
    transfer.items.add(new File([saved.blob], saved.fileName, { type: "application/octet-stream" }));
    input.files = transfer.files;
    input.dispatchEvent(new Event("change", { bubbles: true }));
    return true;
  `);
  await driver.waitFor(
    `return document.querySelector("#position-value")?.textContent === "4, 3" && document.querySelector("#turn-value")?.textContent === "3" && document.querySelector("#inventory-count")?.textContent === "1 堆"`,
    "inventory action save reset",
  );
  state = await readState(driver);
  assert.equal(state.itemCount, "4");
  assert.equal(state.equipmentCount, "0");
  assert.equal(state.canvasUnchanged, true);

  const hashBeforeCameraSwitch = state.stateHash;
  const appliedCellsBeforeCameraSwitch = state.appliedCells;
  const totalAppliedCellsBeforeCameraSwitch = state.totalAppliedCells;
  await driver.execute(`
    const select = document.querySelector("#camera-mode");
    select.value = "player-centered";
    select.dispatchEvent(new Event("change", { bubbles: true }));
    return true;
  `);
  await driver.waitFor(
    `return document.querySelector("#map-host")?.dataset.cameraMode === "player-centered" && document.querySelector("#map-host")?.dataset.viewportWidth === "420"`,
    "player-centered camera mode",
  );
  state = await readState(driver);
  assert.equal(state.stateHash, hashBeforeCameraSwitch);
  assert.equal(state.appliedCells, appliedCellsBeforeCameraSwitch);
  assert.equal(state.totalAppliedCells, totalAppliedCellsBeforeCameraSwitch);
  assert.equal(state.cameraX, "0");
  assert.equal(state.cameraY, "0");
  assert.equal(state.viewportHeight, "420");
  assert.equal(state.visibleChunkCount, "1");
  assert.equal(state.culledChunkCount, "3");
  assert.equal(state.canvasUnchanged, true);

  for (const x of [5, 6, 7, 8]) {
    await dispatchKey(driver, "Numpad6", "6");
    await driver.waitFor(
      `return document.querySelector("#position-value")?.textContent === "${x}, 3"`,
      `camera-follow movement to x=${x}`,
    );
  }
  state = await readState(driver);
  assert.equal(state.cameraMode, "player-centered");
  assert.equal(state.cameraX, "-28");
  assert.equal(state.cameraY, "0");
  assert.equal(state.visibleChunkCount, "2");
  assert.equal(state.culledChunkCount, "2");
  assert.ok(Number(state.rememberedCellCount) > 0);
  assert.equal(state.canvasUnchanged, true);

  const hashBeforeZoom = state.stateHash;
  const cellsBeforeZoom = state.totalAppliedCells;
  await driver.execute(`
    const select = document.querySelector("#zoom-level");
    select.value = "1.5";
    select.dispatchEvent(new Event("change", { bubbles: true }));
    return true;
  `);
  await driver.waitFor(
    `return document.querySelector("#map-host")?.dataset.zoom === "1.5" && document.querySelector("#map-host")?.dataset.cameraX === "-147"`,
    "screen zoom",
  );
  state = await readState(driver);
  assert.equal(state.stateHash, hashBeforeZoom);
  assert.equal(state.totalAppliedCells, cellsBeforeZoom);
  assert.equal(state.zoom, "1.5");
  assert.equal(state.viewportWidth, "420");
  assert.equal(state.viewportHeight, "420");
  assert.equal(state.visibleChunkCount, "1");
  assert.equal(state.canvasUnchanged, true);
  await driver.execute(`
    const select = document.querySelector("#zoom-level");
    select.value = "1";
    select.dispatchEvent(new Event("change", { bubbles: true }));
    return true;
  `);
  await driver.waitFor(
    `return document.querySelector("#map-host")?.dataset.zoom === "1" && document.querySelector("#map-host")?.dataset.cameraX === "-28"`,
    "zoom restore",
  );

  const hashAfterCameraMovement = state.stateHash;
  const cellsAfterCameraMovement = state.totalAppliedCells;
  await driver.execute(`
    const select = document.querySelector("#camera-mode");
    select.value = "full-map";
    select.dispatchEvent(new Event("change", { bubbles: true }));
    return true;
  `);
  await driver.waitFor(
    `return document.querySelector("#map-host")?.dataset.cameraMode === "full-map" && document.querySelector("#map-host")?.dataset.cameraX === "0"`,
    "full-map camera restore",
  );
  state = await readState(driver);
  assert.equal(state.stateHash, hashAfterCameraMovement);
  assert.equal(state.totalAppliedCells, cellsAfterCameraMovement);
  assert.equal(state.visibleChunkCount, "4");
  await driver.execute(`
    const select = document.querySelector("#camera-mode");
    select.value = "player-centered";
    select.dispatchEvent(new Event("change", { bubbles: true }));
    return true;
  `);
  await driver.waitFor(
    `return document.querySelector("#map-host")?.dataset.cameraMode === "player-centered" && document.querySelector("#map-host")?.dataset.cameraX === "-28"`,
    "player-centered camera restore",
  );

  await driver.waitFor(
    `return document.querySelector("#position-value")?.textContent === "8, 3"`,
    "movement after save",
  );

  await driver.execute(`
    const saved = window.__rfbE2eDownloads.find((item) => item.fileName.endsWith(".rfbsave"));
    const input = document.querySelector("#load-input");
    const transfer = new DataTransfer();
    transfer.items.add(new File([saved.blob], saved.fileName, { type: "application/octet-stream" }));
    input.files = transfer.files;
    input.dispatchEvent(new Event("change", { bubbles: true }));
    return true;
  `);
  await driver.waitFor(
    `return document.querySelector("#position-value")?.textContent === "4, 3" && document.querySelector("#turn-value")?.textContent === "3" && document.querySelector("#inventory-count")?.textContent === "1 堆"`,
    "save restore",
  );
  state = await readState(driver);
  assert.equal(state.renderKind, "snapshot");
  assert.equal(state.appliedCells, "400");
  assert.equal(state.itemCount, "4");
  assert.equal(state.inventoryStackCount, "1");
  assert.equal(state.cameraMode, "player-centered");
  assert.equal(state.cameraX, "0");
  assert.equal(state.visibleChunkCount, "1");
  assert.match(state.inventory, /发光碎片/);
  assert.match(state.messages, /存档校验与载入成功/);

  await click(driver, "#replay-button");
  await driver.waitFor(
    `return window.__rfbE2eDownloads?.some((item) => item.fileName.endsWith(".rfbreplay"))`,
    "replay export",
  );
  download = await lastDownload(driver);
  assert.equal(download.fileName, "rfb-rewrite-diagnostic.rfbreplay");
  assert.ok(download.size > 50, `replay is unexpectedly small: ${download.size}`);
  assert.match((await readState(driver)).messages, /已导出不包含存档和本地路径的诊断回放/);

  await driver.execute(`
    window.dispatchEvent(new ErrorEvent("error", { message: "synthetic E2E crash" }));
    return true;
  `);
  await driver.waitFor(
    `return document.documentElement.dataset.crashDiagnosticReport?.endsWith(".rfbdiagnostic")`,
    "automatic frontend crash diagnostic",
  );
  state = await readState(driver);
  assert.equal(state.crashDiagnosticReason, "frontend-error");
  assert.match(state.crashDiagnosticReport, /^crash-\d+(?:-\d+)?\.rfbdiagnostic$/);
  assert.match(state.messages, /已在本机自动保存脱敏诊断报告/);

  await driver.execute(`
    const select = document.querySelector("#tileset-preset");
    select.value = "image";
    select.dispatchEvent(new Event("change", { bubbles: true }));
    return true;
  `);
  await driver.waitFor(
    `return document.querySelector("#map-host")?.dataset.tilesetId === "rfb.tileset.image-demo"`,
    "image tileset hot switch",
  );
  state = await readState(driver);
  assert.equal(state.renderKind, "tileset");
  assert.equal(state.appliedCells, "400");
  assert.equal(state.lastRebuiltTerrainChunks, "4");
  assert.equal(state.totalRebuiltTerrainChunks, "8");
  assert.equal(state.visibleChunkCount, "1");
  assert.equal(state.canvasUnchanged, true);
  assert.match(state.messages, /地图外观已载入：rfb\.tileset\.image-demo/);

  const hashBeforeLanguageSwitch = state.stateHash;
  await driver.execute(`
    const select = document.querySelector("#language-select");
    select.value = "en-US";
    select.dispatchEvent(new Event("change", { bubbles: true }));
    return true;
  `);
  await driver.waitFor(
    `return document.documentElement.lang === "en-US" && document.querySelector("#connection-status")?.textContent === "Core connected"`,
    "English locale switch",
  );
  state = await readState(driver);
  assert.equal(state.stateHash, hashBeforeLanguageSwitch);
  assert.equal(state.canvasUnchanged, true);
  assert.match(state.inventory, /luminous shard/);
  assert.match(state.messages, /You pick up luminous shard ×5/);
  assert.match(state.controls, /Numpad 1–9 moves in eight directions/);

  await driver.execute(`
    window.__rfbRunRendererProfile().catch(() => undefined);
    return true;
  `);
  await driver.waitFor(
    `return document.documentElement.dataset.rendererProfileState === "complete" || document.documentElement.dataset.rendererProfileState === "error"`,
    "large-map renderer profile",
    120_000,
  );
  const profileState = await driver.execute(`
    return {
      state: document.documentElement.dataset.rendererProfileState,
      error: window.__rfbRendererProfileError,
      report: window.__rfbRendererProfileResult,
    };
  `);
  assert.equal(profileState.state, "complete", profileState.error);
  const profile = profileState.report;
  assert.equal(profile.schemaVersion, 1);
  assert.equal(profile.scenarioId, "rfb-render-profile-large-original-v1");
  assert.equal(profile.rendererBackend, "pixi-layered-chunks-v3");
  assert.equal(profile.dynamicViewMode, "visible-chunk-reuse-v1");
  assert.equal(profile.width, 192);
  assert.equal(profile.height, 64);
  assert.equal(profile.cellCount, 12_288);
  assert.equal(profile.dynamicUpdateCellCount, 256);
  assert.equal(profile.terrainUpdateCellCount, 96);
  assert.equal(profile.estimatedFullMapDynamicDisplayObjectCount, 86_016);
  assert.equal(profile.recommendation, "retain-visible-chunk-dynamic-views");
  assert.deepEqual(profile.runs.map((run) => run.chunkSize), [8, 16, 32]);
  assert.deepEqual(
    profile.runs.map((run) => run.diagnostics.terrainChunkCount),
    [192, 48, 12],
  );
  assert.deepEqual(
    profile.runs.map((run) => run.diagnostics.activeDynamicChunkCount),
    [16, 4, 4],
  );
  assert.deepEqual(
    profile.runs.map((run) => run.diagnostics.cellViewCount),
    [1024, 1024, 4096],
  );
  assert.deepEqual(
    profile.runs.map((run) => run.diagnostics.dynamicDisplayObjectCount),
    [7168, 7168, 28_672],
  );
  for (const run of profile.runs) {
    assert.ok(run.diagnostics.visibleChunkCount > 0);
    assert.ok(run.diagnostics.visibleChunkCount < run.diagnostics.terrainChunkCount);
    assert.equal(
      run.diagnostics.activeDynamicChunkCount,
      run.diagnostics.visibleChunkCount,
    );
    assert.equal(run.diagnostics.pooledDynamicChunkCount, 0);
    assert.ok(
      run.diagnostics.dynamicDisplayObjectCount <
        profile.estimatedFullMapDynamicDisplayObjectCount,
    );
    assert.equal(
      run.diagnostics.lastRebuiltTerrainChunks,
      run.diagnostics.terrainChunkCount,
    );
    assert.ok(
      run.diagnostics.totalRebuiltTerrainChunks >=
        run.diagnostics.terrainChunkCount * 2,
    );
    assert.ok(run.canvasPixelWidth >= 192 * 28);
    assert.ok(run.canvasPixelHeight >= 64 * 28);
    assert.equal(run.frameTiming.sampleCount, 45);
    for (const timing of [
      run.initializeMs,
      run.initialCameraMs,
      run.initialSnapshotMs,
      run.cameraSweepMs,
      run.dynamicUpdateMs,
      run.terrainUpdateMs,
      run.tilesetSwitchMs,
      run.frameTiming.medianMs,
      run.frameTiming.p95Ms,
      run.frameTiming.maxMs,
    ]) {
      assert.ok(Number.isFinite(timing) && timing >= 0);
    }
  }
  await mkdir(artifactDirectory, { recursive: true });
  await writeFile(renderProfilePath, `${JSON.stringify(profile, null, 2)}\n`);
}

async function readState(driver) {
  return driver.execute(`
    const host = document.querySelector("#map-host");
    return {
      turn: document.querySelector("#turn-value")?.textContent,
      position: document.querySelector("#position-value")?.textContent,
      health: document.querySelector("#hp-value")?.textContent,
      attack: document.querySelector("#attack-value")?.textContent,
      defense: document.querySelector("#defense-value")?.textContent,
      renderKind: host?.dataset.renderKind,
      appliedCells: host?.dataset.lastAppliedCells,
      tilesetId: host?.dataset.tilesetId,
      rendererBackend: host?.dataset.rendererBackend,
      rendererLayerCount: host?.dataset.rendererLayerCount,
      rendererLayers: host?.dataset.rendererLayers,
      terrainMode: host?.dataset.terrainMode,
      dynamicViewMode: host?.dataset.dynamicViewMode,
      terrainChunkSize: host?.dataset.terrainChunkSize,
      terrainChunkCount: host?.dataset.terrainChunkCount,
      visibleChunkCount: host?.dataset.visibleChunkCount,
      culledChunkCount: host?.dataset.culledChunkCount,
      lastRebuiltTerrainChunks: host?.dataset.lastRebuiltTerrainChunks,
      totalRebuiltTerrainChunks: host?.dataset.totalRebuiltTerrainChunks,
      activeDynamicChunkCount: host?.dataset.activeDynamicChunkCount,
      pooledDynamicChunkCount: host?.dataset.pooledDynamicChunkCount,
      rendererCellViewCount: host?.dataset.rendererCellViewCount,
      rendererDynamicDisplayObjectCount: host?.dataset.rendererDynamicDisplayObjectCount,
      visibilityMode: host?.dataset.visibilityMode,
      lightingMode: host?.dataset.lightingMode,
      protocolVersion: host?.dataset.protocolVersion,
      visualCellCount: host?.dataset.visualCellCount,
      visibleCellCount: host?.dataset.visibleCellCount,
      rememberedCellCount: host?.dataset.rememberedCellCount,
      hiddenCellCount: host?.dataset.hiddenCellCount,
      cameraMode: host?.dataset.cameraMode,
      cameraX: host?.dataset.cameraX,
      cameraY: host?.dataset.cameraY,
      viewportWidth: host?.dataset.viewportWidth,
      viewportHeight: host?.dataset.viewportHeight,
      zoom: host?.dataset.zoom,
      contentId: host?.dataset.contentId,
      contentHash: host?.dataset.contentHash,
      worldId: host?.dataset.worldId,
      contentVisualCount: host?.dataset.contentVisualCount,
      itemCount: host?.dataset.itemCount,
      inventoryStackCount: host?.dataset.inventoryStackCount,
      equipmentCount: host?.dataset.equipmentCount,
      playerStatusCount: host?.dataset.playerStatusCount,
      totalAppliedCells: host?.dataset.totalAppliedCells,
      inventory: document.querySelector("#inventory-list")?.textContent,
      equipment: document.querySelector("#equipment-list")?.textContent,
      effects: document.querySelector("#effects-value")?.textContent,
      controls: document.querySelector("#controls-help")?.textContent,
      locale: document.documentElement.lang,
      crashDiagnosticReport: document.documentElement.dataset.crashDiagnosticReport,
      crashDiagnosticReason: document.documentElement.dataset.crashDiagnosticReason,
      stateHash: document.querySelector("#hash-value")?.title,
      canvasUnchanged: window.__rfbE2eCanvas === host?.querySelector("canvas"),
      messages: document.querySelector("#message-list")?.textContent,
    };
  `);
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
