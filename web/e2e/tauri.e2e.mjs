// SPDX-License-Identifier: MPL-2.0

import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import net from "node:net";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const webDirectory = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repositoryDirectory = path.resolve(webDirectory, "..");
const executable = path.join(repositoryDirectory, "target", "debug", "rfb-tauri.exe");
const artifactDirectory = path.join(repositoryDirectory, "test-results");
const logs = [];
let child;
let client;

async function main() {
  if (process.platform !== "win32") {
    throw new Error("Tauri desktop E2E currently requires Windows WebView2");
  }

  try {
    const port = await reservePort();
    child = spawn(executable, [], {
      cwd: repositoryDirectory,
      env: { ...process.env, TAURI_WEBDRIVER_PORT: String(port) },
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
    if (client) await client.close().catch(() => undefined);
    if (child && child.exitCode === null && child.signalCode === null) child.kill();
  }
}

async function runScenario(driver) {
  await driver.waitFor(
    `return document.querySelector("#connection-status")?.classList.contains("ready")`,
    "native core connection",
  );
  await driver.execute(`
    localStorage.clear();
    localStorage.setItem("rfb.locale", "zh-CN");
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
  assert.equal(state.canvasUnchanged, true);
  assert.equal(state.contentId, "rfb.demo.original-v1");
  assert.equal(
    state.contentHash,
    "880610557b208e7c2459ff876c4ace1cb2ef9903986cb7883a04d511ca13c025",
  );
  assert.equal(state.worldId, "demo.world.original-v1");
  assert.equal(state.contentVisualCount, "5");
  assert.equal(state.itemCount, "1");
  assert.equal(state.inventoryStackCount, "0");
  assert.match(state.inventory, /背包是空的/);

  await dispatchKey(driver, "Numpad5", "5");
  await driver.waitFor(`return document.querySelector("#turn-value")?.textContent === "1"`, "wait command");
  state = await readState(driver);
  assert.equal(state.position, "3, 3");
  assert.equal(state.renderKind, "update");
  assert.equal(state.appliedCells, "0");
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
  assert.equal(state.appliedCells, "2");
  assert.equal(state.canvasUnchanged, true);

  await dispatchKey(driver, "KeyG", "g");
  await driver.waitFor(
    `return document.querySelector("#turn-value")?.textContent === "3" && document.querySelector("#inventory-count")?.textContent === "1 堆"`,
    "ground item pickup",
  );
  state = await readState(driver);
  assert.equal(state.renderKind, "update");
  assert.equal(state.appliedCells, "1");
  assert.equal(state.itemCount, "0");
  assert.equal(state.inventoryStackCount, "1");
  assert.match(state.inventory, /发光碎片/);
  assert.match(state.inventory, /×1/);
  assert.match(state.messages, /你将 1 个发光碎片收入了背包/);

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

  await dispatchKey(driver, "Numpad6", "6");
  await driver.waitFor(
    `return document.querySelector("#position-value")?.textContent === "5, 3"`,
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
  assert.equal(state.itemCount, "0");
  assert.equal(state.inventoryStackCount, "1");
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
  assert.match(state.messages, /You pick up luminous shard ×1/);
  assert.match(state.controls, /Numpad 1–9 moves in eight directions/);
}

async function readState(driver) {
  return driver.execute(`
    const host = document.querySelector("#map-host");
    return {
      turn: document.querySelector("#turn-value")?.textContent,
      position: document.querySelector("#position-value")?.textContent,
      renderKind: host?.dataset.renderKind,
      appliedCells: host?.dataset.lastAppliedCells,
      tilesetId: host?.dataset.tilesetId,
      contentId: host?.dataset.contentId,
      contentHash: host?.dataset.contentHash,
      worldId: host?.dataset.worldId,
      contentVisualCount: host?.dataset.contentVisualCount,
      itemCount: host?.dataset.itemCount,
      inventoryStackCount: host?.dataset.inventoryStackCount,
      inventory: document.querySelector("#inventory-list")?.textContent,
      controls: document.querySelector("#controls-help")?.textContent,
      locale: document.documentElement.lang,
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

  async waitFor(script, description, timeoutMs = 10_000) {
    const deadline = Date.now() + timeoutMs;
    let lastError;
    while (Date.now() < deadline) {
      try {
        if (await this.execute(script)) return;
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
