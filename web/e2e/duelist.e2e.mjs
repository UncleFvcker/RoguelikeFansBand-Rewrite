// SPDX-License-Identifier: MPL-2.0

import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { Localization } from "../src/localization.ts";
import { connectKeyboard } from "./character-creation-layout.e2e.mjs";
import { selectCreationRace } from "./character-creation.e2e.mjs";
import { nextWalk } from "./berserker.e2e.mjs";

// UI acceptance. High levels and targets are explicitly prepared, not natural progression.
export async function runDuelistUiScenario(driver, directory, profile) {
  await mkdir(directory, { recursive: true });
  await driver.waitFor('return document.documentElement.dataset.appMode === "title"', "Duelist title", 60_000);
  const keyboard = await connectKeyboard(profile);
  const sources = Object.fromEntries(await Promise.all(["zh-CN", "en-US"].map(async locale => [locale,
    await Promise.all(["ui", "content", "game"].map(file => readFile(new URL(`../../locales/${locale}/${file}.ftl`, import.meta.url), "utf8"))),
  ])));
  const localization = new Localization("zh-CN", sources);
  const checks = [];
  const click = selector => driver.execute('document.querySelector(arguments[0]).click(); return true;', [selector]);
  const focus = selector => driver.execute('document.querySelector(arguments[0]).focus(); return true;', [selector]);
  const fill = (selector, value) => driver.execute('const el = document.querySelector(arguments[0]); el.value = arguments[1]; el.dispatchEvent(new Event("change", { bubbles: true })); return true;', [selector, value]);
  const hash = () => driver.execute('return document.querySelector("#hash-value").title');
  const ready = () => driver.waitFor('return document.querySelector("#connection-status").classList.contains("ready")', "idle controls");
  const row = slug => `[data-ability-id="demo.ability.duelist-${slug}"]`;
  async function invoke(command, args) {
    await driver.execute(`window.__duelistResult = null; window.__duelistDone = false; window.__duelistError = null;
      window.__TAURI_INTERNALS__.invoke(arguments[0], arguments[1]).then(value => { window.__duelistResult = value; window.__duelistDone = true; }, error => window.__duelistError = String(error)); return true;`, [command, args]);
    await driver.waitFor('return window.__duelistDone || window.__duelistError', command, 30_000);
    assert.equal(await driver.execute('return window.__duelistError'), null);
    return driver.execute('return window.__duelistResult');
  }
  const snapshot = () => invoke("inspect_game_e2e");
  async function changed(before, label) {
    try {
      await driver.waitFor('return document.querySelector("#hash-value").title !== arguments[0]', label, 10_000, [before]);
    } catch (error) {
      throw new Error(`${error}: ${JSON.stringify(await driver.execute('return { focus: document.activeElement.id, map: {...document.querySelector("#map-host").dataset}, messages: document.querySelector("#message-list").textContent };'))}`);
    }
    await ready();
    return snapshot();
  }
  async function viewport(width, height, zoom = 1) {
    await invoke("plugin:webview|set_webview_zoom", { label: "main", value: zoom });
    const rect = await driver.command("GET", "/window/rect");
    const current = await driver.execute('return { width: innerWidth, height: innerHeight }');
    await driver.command("POST", "/window/rect", { width: rect.width + (width - current.width) * zoom, height: rect.height + (height - current.height) * zoom });
    await driver.waitFor('return innerWidth === arguments[0] && innerHeight === arguments[1]', `${width}x${height}`, 10_000, [width, height]);
  }
  async function screenshot(name) {
    await driver.execute('window.__duelistPainted = false; requestAnimationFrame(() => requestAnimationFrame(() => window.__duelistPainted = true)); return true;');
    await driver.waitFor('return window.__duelistPainted', "paint");
    await writeFile(path.join(directory, `${name}.png`), await driver.screenshot(), "base64");
  }
  async function abilitiesPage() {
    await click(await driver.execute('return document.querySelector("#player-page-dialog").open') ? "#player-page-tab-ability" : "#player-ui-ability-open");
    await driver.waitFor('return document.querySelector("#ability-list").checkVisibility()', "abilities page");
  }
  async function prepare(level, targetDistance = 2, wounded = false) {
    let prepared = await invoke("prepare_duelist_e2e", { level, targetDistance, wounded });
    await invoke("save_game", { savedAt: "2026-09-10T13:00:00Z" });
    await driver.execute(`const transfer = new DataTransfer(); transfer.items.add(new File([new Uint8Array(window.__duelistResult)], "duelist-ui.rfbsave"));
      const input = document.querySelector("#load-input"); input.files = transfer.files; input.dispatchEvent(new Event("change", { bubbles: true })); return true;`);
    try {
      await driver.waitFor('return document.querySelector("#hash-value").title === arguments[0]', "prepared save loaded through UI", 30_000, [prepared.stateHash]);
    } catch (error) {
      throw new Error(`${error}: ${await driver.execute('return document.querySelector("#message-list").textContent')}`);
    }
    await ready();
    const talent = prepared.player.pendingRaceMutationChoice;
    if (talent) {
      const index = talent.candidates.findIndex(candidate => candidate.id === "rfb.mutation.sacred-vitality");
      assert.ok(index >= 0);
      await click("#player-ui-character-open"); await click("#character-tab-other");
      await driver.execute('const button = document.querySelectorAll(".mutation-choice-candidate")[arguments[0]]; button.focus(); return true;', [index]);
      await keyboard.key("Enter");
      prepared = await changed(prepared.stateHash, "human talent choice after XP grant");
      await keyboard.key("Escape");
    }
    return prepared;
  }
  async function mark(id) {
    const before = await snapshot();
    const target = before.entities.find(entity => entity.id === id);
    assert.ok(target, `visible ${id}`);
    await focus("#duelist-mark"); await keyboard.key("Enter");
    await driver.waitFor('return document.querySelector("#map-host").dataset.targeting === "true"', "challenge aiming");
    let dx = target.position.x - before.player.position.x, dy = target.position.y - before.player.position.y;
    while (dx || dy) {
      const sx = Math.sign(dx), sy = Math.sign(dy);
      await keyboard.key(({ "-1,-1": "7", "0,-1": "8", "1,-1": "9", "-1,0": "4", "1,0": "6", "-1,1": "1", "0,1": "2", "1,1": "3" })[`${sx},${sy}`]);
      dx -= sx; dy -= sy;
    }
    await keyboard.key("Enter");
    const after = await changed(before.stateHash, "manual challenge");
    assert.equal(after.player.duelistTargetId, id);
    return after;
  }
  async function cast(slug) {
    await abilitiesPage();
    const before = await hash();
    await focus(row(slug) + " .ability-cast-action"); await keyboard.key("Enter");
    return changed(before, slug);
  }
  async function actKey(key) {
    const before = await hash();
    await keyboard.key(key);
    return changed(before, `native key ${key}`);
  }
  async function playNaturalBirth() {
    const born = await snapshot();
    assert.equal(born.player.progress.level, 1);
    assert.equal(born.player.build.classId, "demo.class.duelist");
    assert.equal(born.player.build.raceId, "demo.race.rfb-human");
    await click("#player-ui-inventory-open");
    const torch = born.inventory.find(item => item.kindId === "demo.item.wooden-torch");
    assert.ok(torch);
    await click(`[data-item-id="${torch.id}"] input[type="checkbox"]`);
    const beforeTorch = await hash(); await click("#inventory-equip");
    await changed(beforeTorch, "birth torch equipped"); await keyboard.key("Escape");
    const entrance = born.cells.find(cell => cell.terrainId === "demo.terrain.stairs-down").position;
    let walked = await snapshot();
    for (let step = 0; step < 120; step++) {
      const entry = walked.cells.find(cell => cell.terrainId === "demo.terrain.stairs-down").position;
      if (walked.player.position.x === entry.x && walked.player.position.y === entry.y) break;
      walked = await actKey(nextWalk(walked, new Set(), entry));
    }
    const outside = await hash(); await click("#traverse-stairs");
    let combat = await changed(outside, "normal dungeon entry");
    assert.equal(combat.floorId, "demo.floor.warrens-depth-1");
    const visited = new Set();
    let challenged, hit;
    for (let step = 0; step < 120 && !hit; step++) {
      visited.add(`${combat.player.position.x},${combat.player.position.y}`);
      const distance = position => Math.max(Math.abs(position.x - combat.player.position.x), Math.abs(position.y - combat.player.position.y));
      const target = combat.entities.filter(entity => entity.faction === "hostile").sort((a, b) => distance(a.position) - distance(b.position))[0];
      if (target && !challenged) {
        combat = await mark(target.id);
        challenged = { id: target.id, kindId: target.kindId, hash: combat.stateHash, level: combat.player.progress.level };
        assert.equal(challenged.level, 1);
        await screenshot("natural-monster-marked");
        continue;
      }
      const before = combat;
      combat = await actKey(nextWalk(combat, visited, target?.position));
      assert.equal(combat.player.isDead, false);
      const messages = await driver.execute('return document.querySelector("#message-list").textContent');
      if (messages.includes("你击中了")) {
        assert.ok(challenged);
        assert.equal(before.player.progress.level, 1);
        hit = { before: before.stateHash, after: combat.stateHash, target: target?.id, hp: combat.player.hp, messages };
      }
    }
    assert.ok(hit, "normal level-one character must mark and hit a natural monster");
    await screenshot("natural-monster-melee");
    await click("#player-ui-inventory-open");
    const potion = combat.inventory.find(item => item.kindId === "demo.item.swiftstep-tonic");
    assert.ok(potion?.usable);
    await click(`[data-item-id="${potion.id}"] input[type="checkbox"]`);
    const beforePotion = await hash(); await click("#inventory-use");
    const used = await changed(beforePotion, "ordinary potion used");
    assert.equal(used.inventory.find(item => item.id === potion.id)?.quantity ?? 0, potion.quantity - 1);
    await keyboard.key("Escape"); await screenshot("natural-start-potion");
    checks.push({ naturalBirth: { hash: born.stateHash, entrance, challenged, hit, potion: potion.kindId, afterPotion: used.stateHash }, precondition: "Ordinary level-one Human; normal dungeon generation and keyboard movement, no experience, actor or item preparation." });
  }
  async function saveChargeContinue() {
    const saved = await snapshot();
    assert.equal(saved.player.progress.level, 8);
    assert.equal(saved.player.duelistTargetId, "e2e.duelist-target");
    await driver.execute(`window.__duelistExport = null;
      window.__duelistDownloadHooks = [URL.createObjectURL, URL.revokeObjectURL, HTMLAnchorElement.prototype.click];
      URL.createObjectURL = blob => { window.__duelistExport = { blob }; return "blob:duelist-acceptance"; };
      URL.revokeObjectURL = () => {};
      HTMLAnchorElement.prototype.click = function () { window.__duelistExport.name = this.download; };
      document.querySelector('.hud-menu').open = true; return true;`);
    await click("#save-button");
    await driver.waitFor('return window.__duelistExport?.name?.endsWith(".rfbsave")', "menu save export");
    await driver.execute(`document.querySelector('.hud-menu').open = false;
      [URL.createObjectURL, URL.revokeObjectURL, HTMLAnchorElement.prototype.click] = window.__duelistDownloadHooks;
      window.__duelistSaveBytes = null; window.__duelistExport.blob.arrayBuffer().then(buffer => window.__duelistSaveBytes = Array.from(new Uint8Array(buffer))); return true;`);
    await driver.waitFor('return window.__duelistSaveBytes != null', "exported bytes");
    await writeFile(path.join(directory, "level8-test-upgraded-marked.rfbsave"), Buffer.from(await driver.execute('return window.__duelistSaveBytes')));
    const continued = await cast("charge"); await keyboard.key("Escape");
    assert.notDeepEqual(continued.player.position, saved.player.position);
    assert.ok((continued.entities.find(entity => entity.id === saved.player.duelistTargetId)?.hp ?? 0) < saved.entities.find(entity => entity.id === saved.player.duelistTargetId).hp, "level-eight charge must hit the challenged actor");
    await driver.execute(`const transfer = new DataTransfer(); transfer.items.add(new File([window.__duelistExport.blob], window.__duelistExport.name));
      const input = document.querySelector('#load-input'); input.files = transfer.files; input.dispatchEvent(new Event('change', { bubbles: true })); return true;`);
    await driver.waitFor('return document.querySelector("#hash-value").title === arguments[0]', "marked save restored", 30_000, [saved.stateHash]);
    await ready();
    const restored = await snapshot();
    assert.deepEqual(restored.player, saved.player);
    assert.deepEqual(restored.inventory, saved.inventory);
    assert.deepEqual(restored.equipment, saved.equipment);
    assert.equal(await driver.execute('return document.querySelector("#duelist-status").dataset.targetId'), saved.player.duelistTargetId);
    const replayed = await cast("charge"); await keyboard.key("Escape");
    assert.equal(replayed.stateHash, continued.stateHash, "same charge preserves complete state and RNG after loading");
    await screenshot("marked-save-charge-continue");
    checks.push({ saveContinue: { saved: saved.stateHash, challenge: saved.player.duelistTargetId, continued: continued.stateHash, restoredContinuation: replayed.stateHash, hpBefore: saved.player.hp, hpAfter: replayed.player.hp, origin: saved.player.position, landing: replayed.player.position }, precondition: "Explicit experience to level 8, a lit test floor and source Sheep targets; menu export and native save loading, real charge/combat/RNG." });
  }
  async function checkAbilities(prepared) {
    await abilitiesPage();
    const actual = await driver.execute(`return [...document.querySelectorAll('#ability-list .ability-row')].map(row => ({
      id: row.dataset.abilityId, summary: row.querySelector('.ability-summary').textContent, text: row.textContent,
      disabled: row.querySelector('.ability-cast-action').disabled,
    }));`);
    const visibleAbilities = prepared.player.abilities.filter(ability => !ability.uiGroupNameKey || ability.minimumLevel <= prepared.player.progress.level);
    assert.deepEqual(actual.map(entry => entry.id).sort(), visibleAbilities.map(ability => ability.id).sort());
    for (const ability of visibleAbilities) {
      const rendered = actual.find(entry => entry.id === ability.id);
      assert.ok(rendered, ability.id);
      assert.equal(rendered.summary, localization.format(ability.governingAttribute ? "ability-summary-hp-governed" : "ability-summary-hp", {
        level: ability.minimumLevel, attribute: ability.governingAttribute?.toUpperCase().slice(0, 3) ?? "", cost: ability.hitPointCost,
        baseCost: ability.baseResourceCost, failure: ability.failurePercent,
      }));
      assert.equal(rendered.disabled, !ability.canCast);
      assert.ok(rendered.text.includes(localization.format(ability.targetSpec.range > 0 ? "ability-target-range-summary" : "ability-target-summary", {
        modes: ability.targetSpec.modes.map(mode => localization.format(`ability-target-${mode}`)).join(" / "), range: ability.targetSpec.range,
      })));
      if (ability.unavailableReason) assert.ok(rendered.text.includes(localization.format(`ability-unavailable-${ability.unavailableReason}`)));
      assert.doesNotMatch(rendered.text, /\[[a-z][a-z-]+\]/);
    }
    checks.push({ locale: localization.locale, level: prepared.player.progress.level, abilities: actual });
  }
  async function frameFits(selector) {
    const measurements = await driver.execute(`const el = document.querySelector(arguments[0]), r = el.getBoundingClientRect();
      return { width: innerWidth, height: innerHeight, x: r.x, y: r.y, right: r.right, bottom: r.bottom, scrollWidth: el.scrollWidth, clientWidth: el.clientWidth };`, [selector]);
    assert.ok(measurements.x >= -1 && measurements.y >= -1 && measurements.right <= measurements.width + 1 && measurements.bottom <= measurements.height + 1, JSON.stringify(measurements));
    assert.ok(measurements.scrollWidth <= measurements.clientWidth + 1, JSON.stringify(measurements));
    checks.push({ selector, ...measurements });
  }
  try {
    await invoke("plugin:window|set_min_size", { label: "main", value: null });
    await viewport(1280, 720);
    await click("#session-settings"); await fill("#session-settings-language", "zh-CN"); await fill("#session-settings-input", "numpad"); await click("#session-settings-back");
    await click("#session-new-game");
    await fill("#session-character-name", "决斗旅人"); await fill("#session-seed", "923");
    await click("#session-tab-career"); await click('[data-career-group="melee"]');
    await focus('[data-career-id="demo.build.warrior"]'); await keyboard.key("ArrowDown"); await keyboard.key("ArrowDown"); await keyboard.key("Enter");
    assert.equal(await driver.execute('return document.activeElement.dataset.careerId'), "demo.build.duelist");
    await selectCreationRace(driver, "rfb-legacy.race.tonberry");
    assert.equal(await driver.execute('return document.querySelector("#session-start-game").disabled'), true);
    assert.match(await driver.execute('return document.querySelector("#session-creation-summary").textContent'), /冬贝利/);
    await selectCreationRace(driver, "demo.race.rfb-human");
    assert.equal(await driver.execute('return document.querySelector("#session-start-game").disabled'), false);
    await click("#session-tab-career"); await viewport(390, 844);
    await click('#session-page-career [data-menu-view="details"]');
    await frameFits("#session-career-details"); await screenshot("creation-zh-390");
    await keyboard.key("Escape");
    await viewport(1280, 720); await focus("#session-start-game"); await keyboard.key("Enter");
    await driver.waitFor('return document.documentElement.dataset.appMode === "playing"', "Duelist created", 60_000); await ready();
    await checkAbilities(await snapshot()); await keyboard.key("Escape");
    await playNaturalBirth();
    await prepare(8, 4); await mark("e2e.duelist-target");
    await saveChargeContinue();
    await prepare(8, 8);
    let marked = await mark("e2e.duelist-target");
    const cancelHash = await hash();
    await focus("#duelist-mark"); await keyboard.key("Enter"); await keyboard.key("Escape");
    assert.equal(await hash(), cancelHash);
    assert.equal(await driver.execute('return document.querySelector("#duelist-status").dataset.targetId'), "e2e.duelist-target");
    await frameFits("#duelist-status"); await screenshot("challenge-zh");
    let pending = await cast("charge");
    assert.equal(pending.player.pendingDuelist.type, "charge");
    await viewport(390, 844); await frameFits("#duelist-choice-dialog"); await screenshot("charge-zh-390");
    await keyboard.key("Escape");
    await driver.waitFor('return !document.querySelector("#duelist-choice-dialog").open', "charge declined");
    let declined = await snapshot();
    assert.equal(declined.player.hp, marked.player.hp); assert.equal(declined.worldTick, marked.worldTick);
    assert.equal(declined.player.duelistTargetId, marked.player.duelistTargetId);
    await viewport(1280, 720);
    const beforeClear = await hash(); await focus("#duelist-clear"); await keyboard.key(" ");
    const cleared = await changed(beforeClear, "clear challenge");
    assert.equal(cleared.player.duelistTargetId ?? null, null); assert.equal(cleared.worldTick, declined.worldTick); assert.equal(cleared.player.hp, declined.player.hp);
    await prepare(24, 3);
    const beforeDisengage = await mark("e2e.duelist-target");
    const disengaged = await cast("disengage"); await keyboard.key("Escape");
    assert.equal(disengaged.player.duelistTargetId ?? null, null);
    assert.notDeepEqual(disengaged.player.position, beforeDisengage.player.position);
    assert.equal(disengaged.player.isDead, false);
    await screenshot("level24-disengage");
    checks.push({ disengage: { level: 24, before: beforeDisengage.stateHash, after: disengaged.stateHash, origin: beforeDisengage.player.position, landing: disengaged.player.position, hpBefore: beforeDisengage.player.hp, hpAfter: disengaged.player.hp, challengeCleared: true }, precondition: "Explicit level and source actor preparation; real Disengage command." });
    await prepare(35, 2);
    marked = await mark("e2e.duelist-target");
    for (let attempt = 0; attempt < 20; attempt++) {
      pending = await cast("charge");
      if (pending.player.pendingDuelist?.type === "challenge") break;
    }
    assert.equal(pending.player.pendingDuelist?.type, "challenge");
    await viewport(390, 844); await frameFits("#duelist-choice-dialog"); await screenshot("free-challenge-zh-390");
    await focus("#duelist-choice-target"); await keyboard.key("Home"); await keyboard.key("Tab");
    assert.equal(await driver.execute('return document.activeElement.id'), "duelist-choice-accept");
    await keyboard.key("Enter");
    await driver.waitFor('return !document.querySelector("#duelist-choice-dialog").open', "free challenge accepted");
    assert.equal((await snapshot()).player.duelistTargetId, "e2e.duelist-next", await driver.execute('return document.querySelector("#message-list").textContent'));
    await viewport(1280, 720);
    await fill("#language-select", "en-US"); localization.setLocale("en-US");
    await checkAbilities(await prepare(50, 8));
    await viewport(640, 480, 2); await frameFits("#player-page-dialog"); await screenshot("abilities-en-200percent");
    await keyboard.key("Escape"); await frameFits("#duelist-status");
    await viewport(1280, 720);
    await mark("e2e.duelist-target");
    pending = await cast("charge");
    assert.equal(pending.player.pendingDuelist?.type, "charge");
    await viewport(640, 480, 2); await frameFits("#duelist-choice-dialog"); await screenshot("charge-en-200percent");
    await keyboard.key("Tab", 8);
    assert.equal(await driver.execute('return document.activeElement.id'), "duelist-choice-accept");
    await keyboard.key("Enter");
    await driver.waitFor('return !document.querySelector("#duelist-choice-dialog").open', "charge accepted");
    const accepted = await snapshot();
    assert.equal(accepted.player.pendingDuelist ?? null, null);
    assert.notDeepEqual(accepted.player.position, pending.player.position);
    await viewport(1280, 720);
    await checkAbilities(await prepare(50, 8, true));
    await screenshot("insufficient-hp-en"); await keyboard.key("Escape");
    await prepare(50, 2);
    await mark("e2e.duelist-target");
    await click("#player-ui-inventory-open");
    await click('[data-item-id="e2e.duelist-shield"] input[type="checkbox"]');
    const beforeEquip = await hash(); await click("#inventory-equip");
    const equipped = await changed(beforeEquip, "shield invalidates challenge");
    assert.equal(equipped.player.duelistTargetId ?? null, null);
    assert.equal(await driver.execute('return document.querySelector("#duelist-mark").disabled'), true);
    await checkAbilities(equipped); await screenshot("equipment-unavailable-en");
    assert.deepEqual(keyboard.errors, []);
    await writeFile(path.join(directory, "checks.json"), JSON.stringify({ fixture: "Ordinary level-one birth and natural-monster play, followed by explicit levels/targets on a lit test floor. Real Rust commands, native keyboard, menu export, native save loading and WebView layout. High levels are not natural progression; optimized EXE acceptance is recorded separately.", checks }, null, 2));
    console.log(`Duelist UI passed: ${checks.length} gameplay and projection/layout records`);
  } finally { keyboard.close(); }
}
