// SPDX-License-Identifier: MPL-2.0

import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import net from "node:net";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { nextWalk } from "./berserker.e2e.mjs";
import { Localization } from "../src/localization.ts";

const webDirectory = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repositoryDirectory = path.resolve(webDirectory, "..");
const executable = path.join(repositoryDirectory, "target", "e2e", "debug", "rfb-tauri.exe");
const artifactDirectory = path.join(repositoryDirectory, "test-results");
const logs = [];
let child;
let client;
let nativeSaveName;
const localization = new Localization("zh-CN", { "en-US":[], "zh-CN":await Promise.all(
  ["ui","content","game"].map(file=>readFile(path.join(repositoryDirectory,"locales/zh-CN",`${file}.ftl`),"utf8")),
) });

async function main() {
  if (process.platform !== "win32") {
    throw new Error("Tauri desktop E2E currently requires Windows WebView2");
  }

  try {
    const port = await reservePort();
    child = spawn(executable, [], {
      cwd: repositoryDirectory,
      env: {
        ...process.env,
        WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: [
          process.env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS,
          "--disable-gpu",
        ].filter(Boolean).join(" "),
        TAURI_WEBDRIVER_PORT: String(port),
        WEBVIEW2_USER_DATA_FOLDER: path.join(repositoryDirectory, "target", "e2e", "supply-webview"),
        RFB_E2E_WORLD: "warrens",
      },
      stdio: ["ignore", "pipe", "pipe"],
      windowsHide: true,
    });
    captureOutput(child.stdout, "stdout");
    captureOutput(child.stderr, "stderr");
    await waitForServer(port, child);
    client = await WebDriverClient.create(port, child);
    await runSupplyLoop(client);
    process.stdout.write("Tauri Gate 6 supply-loop E2E passed.\n");
  } catch (error) {
    await mkdir(artifactDirectory, { recursive: true });
    if (client) {
      try {
        logs.push(`[dom] ${JSON.stringify(await client.execute(`
          return {
            homeOpen: document.querySelector("#home-dialog")?.open,
            position: document.querySelector("#position-value")?.textContent,
            shopTitle: document.querySelector("#shop-dialog")?.open ? document.querySelector("#shop-title")?.textContent : null,
            saveName: document.querySelector("#native-save-name")?.value,
            saveNameDisabled: document.querySelector("#native-save-name")?.disabled,
            saveButtonDisabled: document.querySelector("#native-save-create")?.disabled,
            saveRows: [...document.querySelectorAll(".native-save-name")].map((item) => item.textContent),
            messages: [...document.querySelectorAll("#message-list li")].slice(-5).map((item) => item.textContent),
          };
        `))}`);
        await writeFile(
          path.join(artifactDirectory, "tauri-supply-e2e.png"),
          await client.screenshot(),
          "base64",
        );
      } catch (screenshotError) {
        logs.push(`[screenshot] ${String(screenshotError)}`);
      }
    }
    await writeFile(
      path.join(artifactDirectory, "tauri-supply-e2e.log"),
      `${logs.join("\n")}\n`,
    );
    process.stderr.write(`${error instanceof Error ? error.stack : String(error)}\n`);
    process.stderr.write(`Artifacts: ${artifactDirectory}\n`);
    process.exitCode = 1;
  } finally {
    if (client && nativeSaveName) await cleanupNativeSave(client, nativeSaveName).catch(() => undefined);
    if (client) await client.close().catch(() => undefined);
    if (child && child.exitCode === null && child.signalCode === null) child.kill();
  }
}

async function runSupplyLoop(driver) {
  await driver.waitFor(
    `return document.documentElement.dataset.appMode === "title"`,
    "title session shell",
    60_000,
  );
  await driver.execute(`
    localStorage.clear();
    localStorage.setItem("rfb.locale", "zh-CN");
    setTimeout(() => window.location.reload(), 250);
    return true;
  `);
  await driver.waitFor(
    `return performance.getEntriesByType("navigation")[0]?.type === "reload" && document.documentElement.dataset.appMode === "title"`,
    "deterministic title reload",
    60_000,
  );
  await driver.execute(`
    document.querySelector("#session-new-game").click();
    const seed = document.querySelector("#session-seed");
    seed.value = "42";
    seed.dispatchEvent(new Event("input", { bubbles: true }));
    document.querySelector("#session-start-game").click();
    return true;
  `);
  await driver.waitFor(
    `return document.documentElement.dataset.appMode === "playing" && document.querySelector("#map-host")?.dataset.worldId === "demo.world.middle-earth"`,
    "Warrens Warrior session",
    60_000,
  );
  assert.match(await text(driver, "#hud-location-value"), /前哨站/);

  await walkToShop(driver,"demo.shop.outpost-general-store");
  await driver.waitFor(`return document.querySelector("#shop-dialog")?.open`, "automatic shop entry");
  const shopLayout = await driver.execute(`
    const dialog = document.querySelector("#shop-dialog");
    const workspace = document.querySelector(".shop-workspace");
    return {
      title: document.querySelector("#shop-title")?.textContent,
      owner: document.querySelector("#shop-owner")?.textContent,
      dialogFits: dialog.scrollWidth <= dialog.clientWidth && dialog.scrollHeight <= dialog.clientHeight,
      dialogSize: [dialog.scrollWidth,dialog.clientWidth,dialog.scrollHeight,dialog.clientHeight],
      workspaceColumns: getComputedStyle(workspace).gridTemplateColumns.split(" ").length,
      activeTab: document.querySelector('#shop-dialog [role="tab"][aria-selected="true"]')?.id,
      stockRows: document.querySelectorAll("#shop-item-list [data-shop-item-id]").length,
      stockNames: [...document.querySelectorAll("#shop-item-list .shop-item-name")]
        .map((item) => item.textContent),
    };
  `);
  assert.equal(shopLayout.title, "杂货店");
  assert.match(shopLayout.owner, /玛拉·文/);
  assert.equal(shopLayout.dialogFits, true,JSON.stringify(shopLayout.dialogSize));
  assert.equal(shopLayout.workspaceColumns, 2);
  assert.equal(shopLayout.activeTab, "shop-buy-tab");
  await checkShopProjection(driver,shopLayout);

  await mkdir(artifactDirectory, { recursive: true });
  await writeFile(
    path.join(artifactDirectory, "tauri-supply-shop-desktop.png"),
    await driver.screenshot(),
    "base64",
  );

  await driver.setWindowRect(900, 620);
  await driver.waitFor(
    `return window.innerWidth <= 900 && getComputedStyle(document.querySelector(".shop-workspace")).gridTemplateColumns.split(" ").length === 1`,
    "narrow shop layout",
  );
  const narrowShopLayout = await driver.execute(`
    const dialog = document.querySelector("#shop-dialog");
    const rows = [...document.querySelectorAll("#shop-item-list [data-shop-item-id]")];
    return {
      viewportWidth: window.innerWidth,
      dialogFits: dialog.scrollWidth <= dialog.clientWidth,
      pageFits: document.documentElement.scrollWidth <= document.documentElement.clientWidth,
      rowsFit: rows.every((row) => row.scrollWidth <= row.clientWidth),
      workspaceColumns: getComputedStyle(document.querySelector(".shop-workspace"))
        .gridTemplateColumns.split(" ").length,
    };
  `);
  assert.ok(narrowShopLayout.viewportWidth <= 900);
  assert.equal(narrowShopLayout.dialogFits, true);
  assert.equal(narrowShopLayout.pageFits, true);
  assert.equal(narrowShopLayout.rowsFit, true);
  assert.equal(narrowShopLayout.workspaceColumns, 1);
  await writeFile(
    path.join(artifactDirectory, "tauri-supply-shop-narrow.png"),
    await driver.screenshot(),
    "base64",
  );

  await driver.setWindowRect(1280, 820);
  await driver.waitFor(
    `return window.innerWidth > 900 && getComputedStyle(document.querySelector(".shop-workspace")).gridTemplateColumns.split(" ").length === 2`,
    "restored desktop shop layout",
  );

  const goldBeforeShopping = Number(await text(driver, "#shop-gold-value"));
  const rationBefore = await inventoryQuantity(driver, "demo.item.ration-of-food");
  await selectShopItem(driver, "食物口粮");
  await setShopQuantity(driver, 1);
  await click(driver, "#shop-confirm");
  await driver.waitFor(
    `return document.querySelector("#shop-feedback")?.dataset.kind === "success" && document.querySelector("#shop-feedback")?.textContent.includes("购买")`,
    "ration purchase",
  );
  const rationAfterPurchase = await inventoryQuantity(driver, "demo.item.ration-of-food");
  assert.equal(
    rationAfterPurchase,
    rationBefore + 1,
    `inventory rows: ${JSON.stringify(await inventoryRows(driver))}`,
  );

  await selectShopItem(driver, "黄铜提灯");
  await setShopQuantity(driver, 1);
  await click(driver, "#shop-confirm");
  await driver.waitFor(
    `return document.querySelector("#shop-feedback")?.dataset.kind === "success" && document.querySelector("#inventory-list")?.textContent.includes("黄铜提灯")`,
    "lantern purchase",
  );
  await click(driver, "#shop-sell-tab");
  await driver.waitFor(
    `return document.querySelector("#shop-sell-tab")?.getAttribute("aria-selected") === "true"`,
    "sell tab",
  );
  await selectShopItem(driver, "木火把");
  await setShopQuantity(driver, 1);
  await click(driver, "#shop-confirm");
  await driver.waitFor(
    `return document.querySelector("#shop-feedback")?.dataset.kind === "success" && document.querySelector("#shop-feedback")?.textContent.includes("出售")`,
    "torch sale",
  );
  assert.ok(Number(await text(driver, "#shop-gold-value")) < goldBeforeShopping);

  await click(driver, "#shop-close");
  await walkToShop(driver,"demo.shop.outpost-armoury");
  await driver.waitFor(
    `return document.querySelector("#shop-dialog")?.open && document.querySelector("#shop-title")?.textContent === "护甲店"`,
    "Armoury shop entry",
  );
  const armouryLayout = await currentShopLayout(driver);
  assert.match(armouryLayout.owner, /冷酷的达格罗/);
  await checkShopProjection(driver,armouryLayout);
  await selectShopItem(driver, "皮手套");
  await setShopQuantity(driver, 1);
  await click(driver, "#shop-confirm");
  await driver.waitFor(
    `return document.querySelector("#shop-feedback")?.dataset.kind === "success" && document.querySelector("#inventory-list")?.textContent.includes("皮手套")`,
    "Armoury gloves purchase",
  );
  await click(driver, "#shop-close");
  await selectInventoryItem(driver, "demo.item.leather-gloves");
  await click(driver, "#inventory-equip");
  await driver.waitFor(
    `return [...document.querySelectorAll("#equipment-list li")].some((item) => item.textContent.includes("皮手套"))`,
    "equipping purchased gloves",
  );

  await walkToShop(driver,"demo.shop.outpost-weaponsmith");
  await driver.waitFor(
    `return document.querySelector("#shop-dialog")?.open && document.querySelector("#shop-title")?.textContent === "武器店"`,
    "Weaponsmith shop entry",
  );
  const weaponsmithLayout = await currentShopLayout(driver);
  assert.match(weaponsmithLayout.owner, /屠兽者阿恩达尔/);
  await checkShopProjection(driver,weaponsmithLayout);

  await click(driver, "#shop-close");
  await walkToShop(driver,"demo.shop.outpost-temple");
  await driver.waitFor(
    `return document.querySelector("#shop-dialog")?.open && document.querySelector("#shop-title")?.textContent === "圣殿"`,
    "Temple shop entry",
  );
  const templeLayout = await currentShopLayout(driver);
  assert.match(templeLayout.owner, /奥尔德伦·维尔/);
  await checkShopProjection(driver,templeLayout);
  await selectShopItem(driver, "治疗轻伤药水");
  await setShopQuantity(driver, 1);
  await click(driver, "#shop-confirm");
  await driver.waitFor(
    `return document.querySelector("#shop-feedback")?.dataset.kind === "success" && document.querySelector("#inventory-list")?.textContent.includes("治疗轻伤药水")`,
    "Temple healing purchase",
  );

  await click(driver, "#shop-close");
  await walkToShop(driver,"demo.shop.outpost-alchemist");
  await driver.waitFor(
    `return document.querySelector("#shop-dialog")?.open && document.querySelector("#shop-title")?.textContent === "炼金店"`,
    "Alchemist shop entry",
  );
  const alchemistLayout = await currentShopLayout(driver);
  assert.match(alchemistLayout.owner, /伊莉拉·莫斯/);
  await checkShopProjection(driver,alchemistLayout);
  await selectShopItem(driver, "相位之门卷轴");
  await setShopQuantity(driver, 1);
  await click(driver, "#shop-confirm");
  await driver.waitFor(
    `return document.querySelector("#shop-feedback")?.dataset.kind === "success" && document.querySelector("#inventory-list")?.textContent.includes("相位之门卷轴")`,
    "Alchemist scroll purchase",
  );

  await click(driver, "#shop-close");
  await walkToShop(driver,"demo.shop.outpost-bookstore");
  await driver.waitFor(
    `return document.querySelector("#shop-dialog")?.open && document.querySelector("#shop-title")?.textContent === "书店"`,
    "Bookstore entry",
  );
  const bookstoreLayout = await currentShopLayout(driver);
  assert.match(bookstoreLayout.owner, /贪婪的多拉夫/);
  await checkShopProjection(driver,bookstoreLayout);

  await click(driver, "#shop-close");
  await walkToShop(driver,"demo.shop.outpost-magic-shop");
  await driver.waitFor(
    `return document.querySelector("#shop-dialog")?.open && document.querySelector("#shop-title")?.textContent === "魔法店"`,
    "Magic Shop entry",
  );
  const magicShopLayout = await currentShopLayout(driver);
  assert.match(magicShopLayout.owner, /埃德林·索尔/);
  await checkShopProjection(driver,magicShopLayout);

  await click(driver, "#shop-close");
  await walkToShop(driver,"demo.shop.outpost-black-market");
  await driver.waitFor(
    `return document.querySelector("#shop-dialog")?.open && document.querySelector("#shop-title")?.textContent === "黑市"`,
    "Black Market entry",
  );
  const blackMarketLayout = await currentShopLayout(driver);
  assert.match(blackMarketLayout.owner, /公平的托皮/);
  await checkShopProjection(driver,blackMarketLayout);

  await click(driver, "#shop-close");
  await walkTo(driver,state=>state.cells.find(cell=>cell.terrainId==="demo.terrain.stairs-down").position);
  const entrance = (await snapshot(driver)).player.position;
  await driver.execute(`const mode = document.querySelector("#camera-mode"); mode.value = "full-map";
    mode.dispatchEvent(new Event("change", {bubbles:true})); return true;`);
  await driver.waitFor('return document.querySelector("#map-host").dataset.cameraMode === "full-map"',"full-map camera");
  const fullMapCamera = await driver.execute(`
    const host = document.querySelector("#map-host");
    const cellSize = 28 * Number(host.dataset.zoom);
    const playerLeft = arguments[0].x * cellSize - host.scrollLeft;
    const playerTop = arguments[0].y * cellSize - host.scrollTop;
    return {
      mode: host.dataset.cameraMode,
      scrollX: host.scrollLeft,
      playerVisible:
        playerLeft >= 0 &&
        playerTop >= 0 &&
        playerLeft + cellSize <= host.clientWidth &&
        playerTop + cellSize <= host.clientHeight,
    };
  `,[entrance]);
  assert.equal(fullMapCamera.mode, "full-map");
  assert.ok(fullMapCamera.scrollX > 0);
  assert.equal(fullMapCamera.playerVisible, true);
  await dispatchKey(driver, "Period", ">");
  await driver.waitFor(
    `return document.querySelector("#map-host")?.dataset.worldId === "demo.world.middle-earth" && document.querySelector("#hud-location-value")?.textContent.includes("兽穴") && document.querySelector("#hud-location-value")?.textContent.includes("1")`,
    "entering Warrens depth 1",
    30_000,
  );

  await driver.execute(`return window.__rfbPrepareSupplyE2e(37).then(() => true);`);
  await driver.waitFor(
    `return document.querySelector("#map-host")?.dataset.goldPileCount === "1"`,
    "deterministic Warrens gold fixture",
  );
  const rationAtDepth = await inventoryQuantity(driver, "demo.item.ration-of-food");
  await selectInventoryItem(driver, "demo.item.ration-of-food");
  await click(driver, "#inventory-use");
  await driver.waitFor(
    `return document.querySelector("#message-list")?.textContent.includes("你从食物口粮补充了")`,
    "Warrens ration consumption",
  );
  assert.equal(
    await inventoryQuantity(driver, "demo.item.ration-of-food"),
    rationAtDepth - 1,
  );

  const goldBeforePickup = Number((await text(driver, "#gold-value")).replaceAll(",", ""));
  await dispatchKey(driver, "KeyG", "g");
  await driver.waitFor(
    `return document.querySelector("#map-host")?.dataset.goldPileCount === "0" && document.querySelector("#message-list")?.textContent.includes("37 金币")`,
    "Warrens gold pickup",
  );
  assert.equal(
    Number((await text(driver, "#gold-value")).replaceAll(",", "")),
    goldBeforePickup + 37,
  );

  await dispatchKey(driver, "Comma", "<");
  await driver.waitFor(
    `return document.querySelector("#position-value")?.textContent === arguments[0] && document.querySelector("#hud-location-value")?.textContent.includes("前哨站")`,
    "returning to Outpost",
    30_000,
    [`${entrance.x}, ${entrance.y}`],
  );
  await walkToShop(driver,"demo.shop.outpost-general-store");
  await driver.waitFor(`return document.querySelector("#shop-dialog")?.open`, "return shop entry");
  await selectShopItem(driver, "食物口粮");
  await setShopQuantity(driver, 1);
  await click(driver, "#shop-confirm");
  await driver.waitFor(
    `return document.querySelector("#shop-feedback")?.dataset.kind === "success"`,
    "Outpost resupply",
  );

  await click(driver, "#shop-close");
  await walkTo(driver,state=>state.homes.find(home=>!home.museum).entrancePosition);
  const home = (await snapshot(driver)).homes.find(home=>!home.museum);
  await driver.waitFor(
    `return document.querySelector("#home-dialog")?.open && document.querySelector("#home-title")?.textContent === "家"`,
    "automatic Home entry",
  );
  assert.equal(await text(driver, "#position-value"),`${home.entrancePosition.x}, ${home.entrancePosition.y}`);
  assert.equal(
    await driver.execute(`return document.querySelector("#home-withdraw-tab")?.getAttribute("aria-selected");`),
    "true",
  );
  await click(driver, "#home-deposit-tab");
  await driver.waitFor(
    `return document.querySelector("#home-deposit-tab")?.getAttribute("aria-selected") === "true"`,
    "Home deposit tab",
  );
  const rationsBeforeDeposit = await inventoryQuantity(driver, "demo.item.ration-of-food");
  const goldBeforeHome = await text(driver, "#gold-value");
  await selectHomeItem(driver, "食物口粮");
  await setHomeQuantity(driver, 1);
  await click(driver, "#home-confirm");
  await driver.waitFor(
    `return document.querySelector("#home-feedback")?.dataset.kind === "success" && document.querySelector("#home-feedback")?.textContent.includes("存入")`,
    "Home ration deposit",
  );
  assert.equal(await inventoryQuantity(driver, "demo.item.ration-of-food"), rationsBeforeDeposit - 1);
  assert.equal(await text(driver, "#gold-value"), goldBeforeHome);
  await click(driver, "#home-withdraw-tab");
  await driver.waitFor(
    `return document.querySelectorAll("#home-item-list [data-home-item-id]").length === 1 && document.querySelector("#home-item-list")?.textContent.includes("食物口粮")`,
    "Home stored ration",
  );

  const savedHash = await driver.execute(`return document.querySelector("#hash-value")?.title;`);
  const savedGold = await text(driver, "#gold-value");
  const savedRations = await inventoryQuantity(driver, "demo.item.ration-of-food");
  nativeSaveName = `E2E 补给闭环 ${Date.now()}`;
  await click(driver, "#home-close");
  await driver.execute(`
    const input = document.querySelector("#native-save-name");
    input.value = arguments[0];
    input.dispatchEvent(new Event("input", { bubbles: true }));
    document.querySelector("#native-save-create").click();
    return true;
  `, [nativeSaveName]);
  await driver.waitFor(
    `return [...document.querySelectorAll(".native-save-item")].some((row) => row.querySelector(".native-save-name")?.textContent === arguments[0])`,
    "supply-loop native save",
    10_000,
    [nativeSaveName],
  );

  const homeStreet = (await snapshot(driver)).cells.find(cell => cell.terrainId === "demo.terrain.floor"
    && Math.max(Math.abs(cell.position.x-home.entrancePosition.x),Math.abs(cell.position.y-home.entrancePosition.y)) === 1)?.position;
  assert.ok(homeStreet,"Home has an adjacent street");
  const offset = {x:homeStreet.x-home.entrancePosition.x,y:homeStreet.y-home.entrancePosition.y};
  await walkTo(driver,state=> {
    const entry = state.homes.find(home=>!home.museum).entrancePosition;
    return {x:entry.x+offset.x,y:entry.y+offset.y};
  });
  await walkTo(driver,state=>state.homes.find(home=>!home.museum).entrancePosition);
  await driver.waitFor(`return document.querySelector("#home-dialog")?.open`, "Home re-entry before mutation");
  await selectHomeItem(driver, "食物口粮");
  await setHomeQuantity(driver, 1);
  await click(driver, "#home-confirm");
  await driver.waitFor(
    `return document.querySelector("#home-feedback")?.dataset.kind === "success" && document.querySelector("#home-feedback")?.textContent.includes("取出")`,
    "post-save Home withdrawal mutation",
  );
  assert.notEqual(await driver.execute(`return document.querySelector("#hash-value")?.title;`), savedHash);
  await driver.execute(`
    const row = [...document.querySelectorAll(".native-save-item")]
      .find((item) => item.querySelector(".native-save-name")?.textContent === arguments[0]);
    row.querySelector('[data-native-save-action="load"]').click();
    return true;
  `, [nativeSaveName]);
  await driver.waitFor(
    `return document.querySelector("#hash-value")?.title === arguments[0] && document.querySelector("#home-dialog")?.open && document.querySelector("#home-item-list")?.textContent.includes("食物口粮")`,
    "supply-loop native restore",
    30_000,
    [savedHash],
  );
  assert.equal(await text(driver, "#gold-value"), savedGold);
  assert.equal(await inventoryQuantity(driver, "demo.item.ration-of-food"), savedRations);
}

async function currentShopLayout(driver) {
  return driver.execute(`
    return {
      title: document.querySelector("#shop-title")?.textContent,
      owner: document.querySelector("#shop-owner")?.textContent,
      stockRows: document.querySelectorAll("#shop-item-list [data-shop-item-id]").length,
      stockNames: [...document.querySelectorAll("#shop-item-list .shop-item-name")]
        .map((item) => item.textContent),
    };
  `);
}

async function snapshot(driver) {
  await driver.execute(`window.__supplySnapshot=null; window.__supplyError=null;
    window.__TAURI_INTERNALS__.invoke("inspect_game_e2e").then(value=>window.__supplySnapshot=value,error=>window.__supplyError=String(error)); return true;`);
  await driver.waitFor('return window.__supplySnapshot || window.__supplyError',"supply snapshot");
  assert.equal(await driver.execute('return window.__supplyError'),null);
  return driver.execute('return window.__supplySnapshot');
}

async function walkToShop(driver,id) {
  await walkTo(driver,state=>state.shops.find(shop=>shop.id===id).entrancePosition);
}

async function walkTo(driver,selectTarget) {
  let state = await snapshot(driver);
  for (let step=0; step<400; step++) {
    const {x,y} = state.player.position, target = selectTarget(state);
    if (x===target.x && y===target.y) return;
    const key = nextWalk(state,new Set(),target);
    const [dx,dy] = ({"1":[-1,1],"2":[0,1],"3":[1,1],"4":[-1,0],"6":[1,0],"7":[-1,-1],"8":[0,-1],"9":[1,-1]})[key];
    // A route can cross another entrance in the source town layout.
    if (await driver.execute('return document.querySelector("#shop-dialog").open')) await click(driver,"#shop-close");
    await dispatchKey(driver,`Numpad${key}`,key);
    await driver.waitFor('return document.querySelector("#hash-value").title !== arguments[0] && document.querySelector("#connection-status").classList.contains("ready")',"supply movement",10000,[state.stateHash]);
    const moved = await driver.execute('return {position:document.querySelector("#position-value").textContent,hash:document.querySelector("#hash-value").title}');
    const [nextX,nextY] = moved.position.split(", ").map(Number);
    // Ordinary town steps preserve the navigation grid. Refresh on recentering or a blocked step.
    if (nextX !== x+dx || nextY !== y+dy) state = await snapshot(driver);
    else { state.player.position={x:nextX,y:nextY}; state.stateHash=moved.hash; }
  }
  throw new Error("Supply walk did not reach its projected destination");
}

async function checkShopProjection(driver,layout) {
  const shop = (await snapshot(driver)).shops.find(shop=>shop.playerAtEntrance);
  assert.ok(shop);
  assert.equal(layout.title,localization.format(shop.nameKey));
  assert.equal(layout.stockRows,shop.stock.length);
  assert.deepEqual(layout.stockNames,shop.stock.map(item=>localization.format(item.displayNameKey)));
}

async function selectHomeItem(driver, name) {
  await driver.execute(`
    const button = [...document.querySelectorAll("#home-item-list [data-home-item-id]")]
      .find((item) => item.querySelector(".shop-item-name")?.textContent === arguments[0]);
    button?.click();
    return Boolean(button);
  `, [name]);
}

async function setHomeQuantity(driver, quantity) {
  await driver.execute(`
    const input = document.querySelector("#home-quantity");
    input.value = String(arguments[0]);
    input.dispatchEvent(new Event("input", { bubbles: true }));
    return true;
  `, [quantity]);
}

async function selectShopItem(driver, name) {
  await driver.execute(`
    const button = [...document.querySelectorAll("#shop-item-list [data-shop-item-id]")]
      .find((item) => item.textContent.includes(arguments[0]) && !item.disabled);
    if (!button) throw new Error("shop item unavailable: " + arguments[0]);
    button.click();
    return true;
  `, [name]);
}

async function setShopQuantity(driver, quantity) {
  await driver.execute(`
    const input = document.querySelector("#shop-quantity");
    input.value = String(arguments[0]);
    input.dispatchEvent(new Event("input", { bubbles: true }));
    return true;
  `, [quantity]);
  await driver.waitFor(
    `return document.querySelector("#shop-quantity")?.value === String(arguments[0]) && !document.querySelector("#shop-confirm")?.disabled`,
    "valid shop quantity",
    10_000,
    [quantity],
  );
}

async function selectInventoryItem(driver, kindId) {
  await driver.execute(`
    for (const checkbox of document.querySelectorAll('#inventory-list input[type="checkbox"]')) {
      if (checkbox.checked) checkbox.click();
    }
    const row = [...document.querySelectorAll("#inventory-list .inventory-item")]
      .find((item) => item.dataset.itemKindId === arguments[0]);
    if (!row) throw new Error("inventory item unavailable: " + arguments[0]);
    row.querySelector('input[type="checkbox"]').click();
    return true;
  `, [kindId]);
}

async function inventoryQuantity(driver, kindId) {
  return driver.execute(`
    const row = [...document.querySelectorAll("#inventory-list .inventory-item")]
      .find((item) => item.dataset.itemKindId === arguments[0]);
    if (!row) return 0;
    const match = row.querySelector(".inventory-quantity")?.textContent.match(/([0-9]+)/);
    return match ? Number(match[1]) : 0;
  `, [kindId]);
}

async function inventoryRows(driver) {
  return driver.execute(`
    return [...document.querySelectorAll("#inventory-list .inventory-item")].map((item) => ({
      itemId: item.dataset.itemId,
      kindId: item.dataset.itemKindId,
      name: item.querySelector(".inventory-item-name")?.textContent,
      quantity: item.querySelector(".inventory-quantity")?.textContent,
    }));
  `);
}

async function text(driver, selector) {
  return driver.execute(`return document.querySelector(arguments[0])?.textContent;`, [selector]);
}

async function click(driver, selector) {
  await driver.execute(`document.querySelector(arguments[0]).click(); return true;`, [selector]);
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

async function cleanupNativeSave(driver, name) {
  await driver.execute(`
    window.confirm = () => true;
    const row = [...document.querySelectorAll(".native-save-item")]
      .find((item) => item.querySelector(".native-save-name")?.textContent === arguments[0]);
    row?.querySelector('[data-native-save-action="delete"]')?.click();
    return true;
  `, [name]);
  await delay(300);
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
        throw new Error(`Tauri application exited before its main window was available (${app.exitCode ?? app.signalCode})`);
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

  async setWindowRect(width, height) {
    return this.command("POST", "/window/rect", { width, height });
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
      if (!address || typeof address === "string") {
        reject(new Error("Could not reserve a WebDriver port"));
        return;
      }
      server.close(() => resolve(address.port));
    });
  });
}

function captureOutput(stream, label) {
  stream?.setEncoding("utf8");
  stream?.on("data", (chunk) => logs.push(`[${label}] ${chunk}`));
}

function delay(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

await main();
