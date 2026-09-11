// SPDX-License-Identifier: MPL-2.0

import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { Localization } from "../src/localization.ts";
import { connectKeyboard } from "./character-creation-layout.e2e.mjs";
import { prepareDungeonEntry } from "./dungeon-entry.e2e.mjs";

const directions = [[1, 0, "6"], [0, 1, "2"], [-1, 0, "4"], [0, -1, "8"], [1, 1, "3"], [-1, 1, "1"], [-1, -1, "7"], [1, -1, "9"]];
const positionKey = position => `${position.x},${position.y}`;
// Navigation for the acceptance player; every step still goes through normal keyboard input.
export function nextWalk(state, visited, target) {
  const floor = new Set(state.cells.filter(cell => cell.terrainId === "demo.terrain.floor" || cell.terrainId.includes("stairs")).map(cell => positionKey(cell.position)));
  if (target) floor.add(positionKey(target)); // Explicit destinations also include projected shop/home entrances.
  const queue = [{ ...state.player.position, key: undefined }];
  const seen = new Set([positionKey(state.player.position)]);
  for (let index = 0; index < queue.length; index++) {
    const position = queue[index];
    if (position.key && (target ? positionKey(position) === positionKey(target) : !visited.has(positionKey(position)))) return position.key;
    for (const [dx, dy, key] of directions) {
      const next = { x: position.x + dx, y: position.y + dy, key: position.key ?? key };
      const id = positionKey(next);
      if (floor.has(id) && !seen.has(id)) { seen.add(id); queue.push(next); }
    }
  }
  throw new Error("No reachable acceptance destination");
}

export async function runBerserkerUiScenario(driver, directory, profile) {
  await mkdir(directory, { recursive: true });
  const keyboard = await connectKeyboard(profile);
  await driver.waitFor('return document.documentElement.dataset.appMode === "title"', "Berserker title", 60_000);
  await driver.execute('window.__berserkerReload = true; localStorage.setItem("rfb.locale", "zh-CN"); localStorage.setItem("rfb.input-preset", "numpad"); setTimeout(() => location.reload(), 50); return true;');
  await driver.waitFor('return !window.__berserkerReload && document.documentElement.dataset.appMode === "title"', "Chinese title", 60_000);
  const sources = Object.fromEntries(await Promise.all(["zh-CN", "en-US"].map(async locale => [locale,
    await Promise.all(["ui", "content", "game"].map(file => readFile(new URL(`../../locales/${locale}/${file}.ftl`, import.meta.url), "utf8"))),
  ])));
  const localization = new Localization("zh-CN", sources);
  const checks = [];
  const click = selector => driver.execute('document.querySelector(arguments[0]).click(); return true;', [selector]);
  const focusIs = selector => driver.execute('return document.activeElement.matches(arguments[0]);', [selector]);
  const hash = () => driver.execute('return document.querySelector("#hash-value").title;');
  const ready = () => driver.waitFor('return document.querySelector("#connection-status").classList.contains("ready")', "ready UI");
  const row = slug => `[data-ability-id="demo.ability.berserker-${slug}"]`;
  async function actKey(key) {
    const before = await hash();
    await keyboard.key(key);
    await driver.waitFor('return document.querySelector("#hash-value").title !== arguments[0]', `native key ${key}`, 10_000, [before]);
    await ready();
    return invoke("inspect_game_e2e");
  }
  async function tabTo(selector) {
    for (let step = 0; step < 50; step++) {
      if (await focusIs(selector)) return;
      await keyboard.key("Tab");
    }
    throw new Error(`Tab could not reach ${selector}`);
  }
  async function invoke(command, args) {
    await driver.execute(`window.__berserkerInvokeDone = false; window.__berserkerInvokeError = null;
      window.__TAURI_INTERNALS__.invoke(arguments[0], arguments[1]).then(result => { window.__berserkerInvokeResult = result; window.__berserkerInvokeDone = true; }, error => window.__berserkerInvokeError = String(error)); return true;`, [command, args]);
    await driver.waitFor('return window.__berserkerInvokeDone || window.__berserkerInvokeError', command);
    assert.equal(await driver.execute('return window.__berserkerInvokeError'), null);
    return driver.execute('return window.__berserkerInvokeResult');
  }
  async function viewport(width, height, zoom = 1) {
    await invoke("plugin:webview|set_webview_zoom", { label: "main", value: zoom });
    const rect = await driver.command("GET", "/window/rect");
    const current = await driver.execute('return { width: innerWidth, height: innerHeight }');
    await driver.command("POST", "/window/rect", { width: rect.width + (width - current.width) * zoom, height: rect.height + (height - current.height) * zoom });
    await driver.waitFor('return innerWidth === arguments[0] && innerHeight === arguments[1]', `${width}x${height}`, 10_000, [width, height]);
    await driver.execute('window.__berserkerLayoutReady = false; requestAnimationFrame(() => requestAnimationFrame(() => window.__berserkerLayoutReady = true)); return true;');
    await driver.waitFor('return window.__berserkerLayoutReady', "responsive layout settled");
  }
  async function screenshot(name) {
    await writeFile(path.join(directory, `berserker-${name}.png`), await driver.screenshot(), "base64");
  }
  async function abilitiesPage() {
    const open = await driver.execute('return document.querySelector("#player-page-dialog").open');
    await click(open ? "#player-page-tab-ability" : "#player-ui-ability-open");
    await driver.waitFor('return document.querySelector("#ability-list").checkVisibility()', "ability page");
  }
  async function prepareLevel(level, wounded = false, withTarget = false) {
    await driver.execute(`window.__berserkerPrepared = null; window.__berserkerPrepareError = null;
      (async () => {
        const snapshot = await window.__TAURI_INTERNALS__.invoke("prepare_berserker_e2e", { level: arguments[0], wounded: arguments[1], withTarget: arguments[2] });
        const bytes = await window.__TAURI_INTERNALS__.invoke("save_game", { savedAt: "2026-09-10T12:00:00Z" });
        const files = new DataTransfer(); files.items.add(new File([new Uint8Array(bytes)], "berserker-ui.rfbsave"));
        const input = document.querySelector("#load-input"); input.files = files.files;
        input.dispatchEvent(new Event("change", { bubbles: true }));
        window.__berserkerPrepared = { level: snapshot.player.progress.level, hash: snapshot.stateHash, abilities: snapshot.player.abilities, player: snapshot.player, inventory: snapshot.inventory };
      })().catch(error => window.__berserkerPrepareError = String(error)); return true;`, [level, wounded, withTarget]);
    await driver.waitFor('return window.__berserkerPrepared || window.__berserkerPrepareError', "real level and save preparation", 30_000);
    assert.equal(await driver.execute('return window.__berserkerPrepareError'), null);
    const prepared = await driver.execute('return window.__berserkerPrepared');
    assert.equal(prepared.level, level);
    try {
      await driver.waitFor('return document.querySelector("#hash-value").title === arguments[0] && document.querySelector("#connection-status").classList.contains("ready")', "exact native save loaded", 30_000, [prepared.hash]);
    } catch (error) {
      throw new Error(`${error}: ${await driver.execute('return JSON.stringify({ level: window.__berserkerPrepared.level, expected: window.__berserkerPrepared.hash, actual: document.querySelector("#hash-value").title, messages: document.querySelector("#message-list").textContent });')}`);
    }
    await abilitiesPage();
    return prepared;
  }
  async function checkAbilities(prepared) {
    const actual = await driver.execute(`return [...document.querySelectorAll('#ability-list .ability-row')].map(row => ({
      id: row.dataset.abilityId, name: row.querySelector('.ability-name').textContent,
      description: row.querySelector('.ability-description').textContent, summary: row.querySelector('.ability-summary').textContent,
      text: row.textContent, actions: row.querySelectorAll('button').length, disabled: row.querySelector('.ability-cast-action').disabled,
    }));`);
    const expected = prepared.abilities.filter(ability => !ability.uiGroupNameKey || ability.minimumLevel <= prepared.level);
    assert.deepEqual(actual.map(value => value.id).sort(), expected.map(value => value.id).sort());
    for (const ability of expected) {
      const rendered = actual.find(value => value.id === ability.id);
      assert.equal(rendered.name, localization.format(ability.nameKey));
      assert.equal(rendered.description, localization.format(ability.descriptionKey));
      assert.equal(rendered.summary, localization.format(ability.governingAttribute ? "ability-summary-hp-governed" : "ability-summary-hp", {
        level: ability.minimumLevel, attribute: ability.governingAttribute?.toUpperCase().slice(0, 3) ?? "",
        cost: ability.hitPointCost, baseCost: ability.baseResourceCost, failure: ability.failurePercent,
      }));
      assert.equal(rendered.actions, 1, `${ability.id}: no study/forget for automatic powers`);
      assert.equal(rendered.disabled, !ability.canCast);
      assert.ok(rendered.text.includes(localization.format(ability.targetSpec.range > 0 ? "ability-target-range-summary" : "ability-target-summary", {
        modes: ability.targetSpec.modes.map(mode => localization.format(`ability-target-${mode}`)).join(" / "),
        range: ability.targetSpec.range,
      })), `${ability.id}: authoritative target modes and range`);
      assert.doesNotMatch(rendered.text, /熟练度|Proficiency|\[[a-z][a-z-]+\]/);
      if (ability.detect?.category === "mind") assert.ok(rendered.text.includes(localization.format("ability-detect-mind-summary", {
        radius: ability.detect.radius, persistence: localization.format(ability.detect.persistent ? "ability-detect-persistent" : "ability-detect-transient"),
      })));
      if (ability.unavailableReason) assert.ok(rendered.text.includes(localization.format(`ability-unavailable-${ability.unavailableReason}`)));
    }
    checks.push({ locale: localization.locale, level: prepared.level, abilities: actual.map(value => ({ id: value.id, name: value.name, summary: value.summary })), hash: prepared.hash });
  }
  async function castPower(slug, cost, direction) {
    const attempts = [];
    for (let attempt = 0; attempt < 20; attempt++) {
      await abilitiesPage();
      const before = await invoke("inspect_game_e2e");
      assert.ok(before.player.hp > cost, `Insufficient safe HP for ${slug}: ${JSON.stringify(attempts)}`);
      await tabTo(row(slug) + " .ability-cast-action"); await keyboard.key("Enter");
      if (direction) {
        await driver.waitFor('return !document.querySelector("#target-cursor").hidden', "power direction");
        await keyboard.key(direction); await keyboard.key("Enter");
      }
      await driver.waitFor('return document.querySelector("#hash-value").title !== arguments[0]', `real ${slug} cast`, 10_000, [before.stateHash]);
      await ready();
      const after = await invoke("inspect_game_e2e");
      const message = await driver.execute('const row = [...document.querySelectorAll("#message-list .message-ability-cast-success, #message-list .message-ability-cast-failure")].at(-1); return { success: row.classList.contains("message-ability-cast-success"), text: row.lastElementChild.textContent };');
      assert.ok(message.text.includes(localization.format("message-ability-cast-cost-hp", { amount: cost })));
      attempts.push({ before: before.stateHash, after: after.stateHash, hpBefore: before.player.hp, hpAfter: after.player.hp, message });
      if (message.success) return { before, after, attempts };
    }
    throw new Error(`No successful ${slug} in twenty natural casting rolls: ${JSON.stringify(attempts)}`);
  }
  async function playPowers() {
    const detection = await castPower("detect-menace", 5);
    const detected = await driver.execute('return [...document.querySelectorAll("#message-list .message-ability-detect")].at(-1).lastElementChild.textContent;');
    assert.equal(detected, localization.format("message-ability-detect-mind", { ability: localization.format("ability-demo-berserker-detect-menace-name"), count: 1 }));
    const target = detection.after.entities.find(entity => entity.id === "e2e.charge-target");
    assert.ok(target);
    const origin = detection.after.player.position;
    const dx = target.position.x - origin.x, dy = target.position.y - origin.y;
    assert.equal(Math.max(Math.abs(dx), Math.abs(dy)), 1);
    const charge = await castPower("charge", 20, directions.find(([x, y]) => x === dx && y === dy)[2]);
    assert.deepEqual(charge.after.player.position, { x: origin.x + dx * 2, y: origin.y + dy * 2 });
    assert.ok((charge.after.entities.find(entity => entity.id === target.id)?.hp ?? 0) < target.hp, "charge must actually hit");
    await screenshot("detection-charge");
    const recallStart = await castPower("recall", 10);
    assert.ok(recallStart.after.player.recall.remainingTurns > 0);
    const recallCancel = await castPower("recall", 10);
    assert.equal(recallCancel.after.player.recall.remainingTurns, undefined);
    const recallAgain = await castPower("recall", 10);
    assert.ok(recallAgain.after.player.recall.remainingTurns > 0);
    await driver.execute(`window.__berserkerDownload = null;
      URL.createObjectURL = blob => { window.__berserkerDownload = { blob }; return "blob:berserker-acceptance"; };
      URL.revokeObjectURL = () => {};
      HTMLAnchorElement.prototype.click = function () { window.__berserkerDownload.name = this.download; };
      document.querySelector('.hud-menu').open = true; return true;`);
    const saved = await invoke("inspect_game_e2e"); await click("#save-button");
    await driver.waitFor('return window.__berserkerDownload?.name?.endsWith(".rfbsave")', "native save export from menu");
    await driver.execute('document.querySelector(".hud-menu").open = false; window.__berserkerSaveBytes = null; window.__berserkerDownload.blob.arrayBuffer().then(buffer => window.__berserkerSaveBytes = Array.from(new Uint8Array(buffer))); return true;');
    await driver.waitFor('return window.__berserkerSaveBytes != null', "exported save bytes");
    await writeFile(path.join(directory, "berserker-level15-test-upgraded.rfbsave"), Buffer.from(await driver.execute('return window.__berserkerSaveBytes')));
    const continued = await actKey("5");
    await driver.execute(`const saved = window.__berserkerDownload;
      const files = new DataTransfer(); files.items.add(new File([saved.blob], saved.name));
      const input = document.querySelector('#load-input'); input.files = files.files;
      input.dispatchEvent(new Event('change', { bubbles: true })); return true;`);
    await driver.waitFor('return document.querySelector("#hash-value").title === arguments[0]', "exact played save restored", 30_000, [saved.stateHash]);
    await ready();
    const restored = await invoke("inspect_game_e2e");
    assert.deepEqual(restored.player, saved.player);
    assert.deepEqual(restored.inventory, saved.inventory);
    assert.deepEqual(restored.equipment, saved.equipment);
    const replayed = await actKey("5");
    assert.equal(replayed.stateHash, continued.stateHash, "same command preserves state and RNG after loading");
    let recalled = replayed;
    for (let turn = 0; turn < 40 && recalled.floorId === saved.floorId; turn++) recalled = await actKey("5");
    assert.equal(recalled.floorId, "core.floor.wilderness");
    assert.equal(recalled.player.recall.remainingTurns, undefined);
    assert.equal(recalled.player.isDead, false);
    await screenshot("recall-save-continue");
    checks.push({ powers: { detection: detection.attempts, detected, charge: charge.attempts, chargeOrigin: origin, chargeLanding: charge.after.player.position, recallStart: recallStart.attempts, recallCancel: recallCancel.attempts, recallAgain: recallAgain.attempts, saved: saved.stateHash, continued: continued.stateHash, restoredContinuation: replayed.stateHash, recalled: recalled.stateHash, returnedFloor: recalled.floorId }, precondition: "Real XP grant to level 15 and one source-defined Stone Troll target; natural casting, combat and recall rolls, menu save, native load and deterministic continuation" });
  }
  try {
    await invoke("plugin:window|set_min_size", { label: "main", value: null });
    await viewport(1280, 720);
    await tabTo("#session-new-game"); await keyboard.key("Enter");
    await keyboard.key("a", 2); await keyboard.text("狂战旅人");
    await tabTo("#session-seed"); await keyboard.key("a", 2); await keyboard.text("923");
    await tabTo("#session-tab-overview"); await keyboard.key("ArrowRight"); await keyboard.key("ArrowRight");
    await tabTo('[data-career-group="melee"]'); await keyboard.key("Enter");
    await tabTo('[data-career-id="demo.build.warrior"]'); await keyboard.key("ArrowDown");
    assert.equal(await focusIs('[data-career-id="demo.build.berserker"]'), true);
    await keyboard.key("Enter");
    assert.equal(await driver.execute('return document.querySelector("#session-start-game").disabled'), false);
    assert.match(await driver.execute('return document.querySelector("#session-creation-summary").textContent'), /狂战旅人.*狂战士/);
    await viewport(390, 844);
    await tabTo('#session-page-career [data-menu-view="details"]'); await keyboard.key("Enter");
    assert.equal(await driver.execute('const node = document.querySelector("#session-career-description"); return node.scrollWidth <= node.clientWidth'), true);
    await screenshot("creation-390x844");
    await keyboard.key("Escape");
    assert.equal(await focusIs('[data-career-id="demo.build.berserker"]'), true);
    await viewport(1280, 720);
    await tabTo("#session-start-game"); await keyboard.key("Enter");
    await driver.waitFor('return document.documentElement.dataset.appMode === "playing" && document.querySelector("#connection-status").classList.contains("ready")', "normal human Berserker birth", 60_000);
    assert.equal(await driver.execute('return document.querySelector("#app").dataset.sessionBuildId'), "demo.build.berserker");
    await abilitiesPage();
    assert.equal(await driver.execute('return document.querySelectorAll("#ability-list .ability-row").length'), 0);
    assert.equal(await driver.execute('return document.querySelector("#ability-list .ability-empty").textContent'), localization.format("ability-unavailable"));
    assert.equal(await driver.execute('return document.querySelectorAll("#resource-list .resource-row").length'), 0);
    await screenshot("birth-no-powers");
    checks.push({ normalBirthHash: await hash(), level: 1, abilities: [], precondition: "Normal new Human, no debug preparation" });
    const born = await invoke("inspect_game_e2e");
    await keyboard.key("Escape");
    await click("#player-ui-inventory-open");
    const torch = born.inventory.find(item => item.kindId === "demo.item.wooden-torch");
    await click(`[data-item-id="${torch.id}"] input[type="checkbox"]`);
    const beforeTorch = await hash(); await click("#inventory-equip");
    await driver.waitFor('return document.querySelector("#hash-value").title !== arguments[0]', "starting torch equipped", 10_000, [beforeTorch]);
    await ready(); await click("#player-page-close");
    const entrance = born.cells.find(cell => cell.terrainId === "demo.terrain.stairs-down").position;
    let walked = await invoke("inspect_game_e2e");
    if (process.argv.includes("--fast-entry")) { checks.push({ fastEntry: await prepareDungeonEntry(driver) }); walked = await invoke("inspect_game_e2e"); }
    else for (let step = 0; step < 120; step++) {
      if (walked.player.position.x === entrance.x && walked.player.position.y === entrance.y) break;
      walked = await actKey(nextWalk(walked, new Set(), entrance));
    }
    const outsideHash = await hash(); await click("#traverse-stairs");
    await driver.waitFor('return document.querySelector("#hash-value").title !== arguments[0]', "normal dungeon entry", 10_000, [outsideHash]);
    await ready();
    const dungeon = await invoke("inspect_game_e2e");
    assert.equal(dungeon.floorId, "demo.floor.warrens-depth-1");
    assert.equal(dungeon.player.progress.level, 1);
    let combat = dungeon;
    let birthHit;
    const visited = new Set();
    for (let step = 0; step < 120 && !birthHit; step++) {
      visited.add(positionKey(combat.player.position));
      const distance = position => Math.max(Math.abs(position.x - combat.player.position.x), Math.abs(position.y - combat.player.position.y));
      const target = combat.entities.filter(entity => entity.faction === "hostile").sort((a, b) => distance(a.position) - distance(b.position))[0];
      const before = combat;
      combat = await actKey(nextWalk(combat, visited, target?.position));
      assert.equal(combat.player.isDead, false);
      const messages = await driver.execute('return document.querySelector("#message-list").textContent');
      if (messages.includes("你击中了")) {
        assert.equal(before.player.progress.level, 1, "first melee uses the normal level-one character");
        birthHit = { steps: step + 1, before: before.stateHash, after: combat.stateHash, hp: combat.player.hp, target: target?.kindId, messages };
      }
    }
    assert.ok(birthHit, "normal new character must land a real melee hit");
    await screenshot("birth-melee");
    await click("#player-ui-inventory-open");
    const potion = combat.inventory.find(item => item.kindId === "demo.item.healing-potion");
    assert.ok(potion?.usable);
    await click(`[data-item-id="${potion.id}"] input[type="checkbox"]`);
    const beforePotion = await hash(); await click("#inventory-use");
    await driver.waitFor('return document.querySelector("#hash-value").title !== arguments[0]', "normal birth potion used", 10_000, [beforePotion]);
    await ready();
    const afterPotion = await invoke("inspect_game_e2e");
    assert.equal(afterPotion.inventory.find(item => item.id === potion.id)?.quantity ?? 0, potion.quantity - 1);
    checks.push({ normalBirth: { birth: born.stateHash, walked: walked.stateHash, entrance, dungeon: dungeon.floorId, birthHit, potionBefore: beforePotion, potionAfter: afterPotion.stateHash }, precondition: "Normal new character, dungeon exploration, natural monster melee and starting potion; only --fast-entry skips the road, recorded separately when used." });

    for (const level of [7, 8, 9, 10, 14, 15, 19, 20, 24, 25, 29, 30]) {
      const prepared = await prepareLevel(level, false, level === 15);
      await checkAbilities(prepared);
      assert.equal(prepared.player.abilityLearning, undefined);
      assert.equal(prepared.player.resources, undefined);
      if (level === 15) await playPowers();
    }
    // Resolve the normal Human level-30 talent prompt before targeting.
    while (await driver.execute('return !!document.querySelector(".mutation-choice-candidate")')) {
      await click("#player-page-tab-character"); await click("#character-tab-other");
      const before = await hash(); await click(".mutation-choice-candidate");
      await driver.waitFor('return document.querySelector("#hash-value").title !== arguments[0]', "human talent selected", 10_000, [before]);
      await ready();
    }
    const final = await prepareLevel(30);
    await checkAbilities(final);
    assert.equal(final.abilities.length, 6);
    await tabTo(row("charge") + " .ability-cast-action"); await keyboard.key("Enter");
    await driver.waitFor('return !document.querySelector("#target-cursor").hidden', "charge directional target");
    await keyboard.key("Escape");
    assert.equal(await hash(), final.hash, "keyboard cancellation preserves HP, turn and RNG");
    await abilitiesPage();
    await viewport(390, 844);
    const layout = await driver.execute('const list = document.querySelector("#ability-list"); return { overflow: list.scrollWidth > list.clientWidth, dialogWidth: document.querySelector("#player-page-dialog").getBoundingClientRect().width, buttons: [...list.querySelectorAll(".ability-cast-action")].every(button => button.getBoundingClientRect().width > 0) };');
    assert.equal(layout.overflow, false); assert.equal(layout.buttons, true); assert.ok(layout.dialogWidth <= 390);
    await screenshot("abilities-30-390x844");
    await viewport(1280, 720);
    await screenshot("abilities-30-1280x720");

    async function checkItems(prepared) {
      await click("#player-page-tab-inventory");
      for (const id of ["e2e.scroll", "e2e.wand", "e2e.activation"]) {
        const item = prepared.inventory.find(item => item.id === id);
        assert.equal(item.usable, false);
        assert.equal(item.useUnavailableReason, "berserker");
        await click('[data-item-id="' + id + '"] .inventory-item-inspect');
        await driver.waitFor('return document.querySelector("#inventory-detail-dialog").open', "item details");
        assert.equal(await driver.execute('return document.querySelector("#inventory-detail-body .inventory-use-unavailable").textContent'), localization.format("item-use-unavailable-berserker"));
        await screenshot("forbidden-" + id.slice(4) + "-" + localization.locale);
        await keyboard.key("Escape");
      }
    }
    await checkItems(final);
    for (const locale of ["zh-CN", "en-US"]) {
      await driver.execute('const select = document.querySelector("#language-select"); select.value = arguments[0]; select.dispatchEvent(new Event("change", { bubbles: true })); return true;', [locale]);
      await ready(); localization.setLocale(locale);
      const wounded = await prepareLevel(30, true);
      await checkAbilities(wounded);
      assert.equal(wounded.player.hp, 1);
      assert.ok(wounded.abilities.every(ability => !ability.canCast && ability.unavailableReason === "insufficient-hit-points"));
      await screenshot("insufficient-hp-" + locale);
      await checkItems(wounded);
    }
    const english = await prepareLevel(30);
    await checkAbilities(english);
    await viewport(640, 360, 2);
    assert.equal(await driver.execute('const list = document.querySelector("#ability-list"); return list.scrollWidth <= list.clientWidth'), true);
    await screenshot("abilities-30-en-US-200percent");
    checks.push({ layout, englishZoom: 2, cancelledHash: final.hash, insufficientHp: 1, forbiddenItems: ["scroll", "wand", "activation"], keyboard: "Native WebView2 CDP Tab, arrows, Enter and Escape", levelSetup: "WebDriver-only real experience grants, explicit item and HP fixtures; native save validation on every load" });
    await writeFile(path.join(directory, "berserker-ui-acceptance.json"), JSON.stringify(checks, null, 2));
  } finally { keyboard.close(); }
}
