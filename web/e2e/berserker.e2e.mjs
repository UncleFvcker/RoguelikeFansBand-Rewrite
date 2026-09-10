// SPDX-License-Identifier: MPL-2.0

import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { Localization } from "../src/localization.ts";
import { connectKeyboard } from "./character-creation-layout.e2e.mjs";

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
  async function tabTo(selector) {
    for (let step = 0; step < 50; step++) {
      if (await focusIs(selector)) return;
      await keyboard.key("Tab");
    }
    throw new Error(`Tab could not reach ${selector}`);
  }
  async function invoke(command, args) {
    await driver.execute(`window.__berserkerInvokeDone = false; window.__berserkerInvokeError = null;
      window.__TAURI_INTERNALS__.invoke(arguments[0], arguments[1]).then(() => window.__berserkerInvokeDone = true, error => window.__berserkerInvokeError = String(error)); return true;`, [command, args]);
    await driver.waitFor('return window.__berserkerInvokeDone || window.__berserkerInvokeError', command);
    assert.equal(await driver.execute('return window.__berserkerInvokeError'), null);
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
  async function prepareLevel(level, wounded = false) {
    await driver.execute(`window.__berserkerPrepared = null; window.__berserkerPrepareError = null;
      (async () => {
        const snapshot = await window.__TAURI_INTERNALS__.invoke("prepare_berserker_e2e", { level: arguments[0], wounded: arguments[1] });
        const bytes = await window.__TAURI_INTERNALS__.invoke("save_game", { savedAt: "2026-09-10T12:00:00Z" });
        const files = new DataTransfer(); files.items.add(new File([new Uint8Array(bytes)], "berserker-ui.rfbsave"));
        const input = document.querySelector("#load-input"); input.files = files.files;
        input.dispatchEvent(new Event("change", { bubbles: true }));
        window.__berserkerPrepared = { level: snapshot.player.progress.level, hash: snapshot.stateHash, abilities: snapshot.player.abilities, player: snapshot.player, inventory: snapshot.inventory };
      })().catch(error => window.__berserkerPrepareError = String(error)); return true;`, [level, wounded]);
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

    for (const level of [7, 8, 9, 10, 14, 15, 19, 20, 24, 25, 29, 30]) {
      const prepared = await prepareLevel(level);
      await checkAbilities(prepared);
      assert.equal(prepared.player.abilityLearning, undefined);
      assert.equal(prepared.player.resources, undefined);
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
      const potion = prepared.inventory.find(item => item.kindId === "demo.item.healing-potion");
      assert.equal(potion.usable, true);
      assert.equal(potion.useUnavailableReason, undefined);
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
