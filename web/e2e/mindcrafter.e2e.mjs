// SPDX-License-Identifier: MPL-2.0

import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { Localization } from "../src/localization.ts";
import { connectKeyboard } from "./character-creation-layout.e2e.mjs";

export async function runMindcrafterUiScenario(driver, directory, profile) {
  await mkdir(directory, { recursive: true });
  const keyboard = await connectKeyboard(profile);
  await driver.waitFor('return document.documentElement.dataset.appMode === "title"', "Mindcrafter title", 60_000);
  await driver.execute('window.__mindReload = true; localStorage.setItem("rfb.locale", "zh-CN"); localStorage.setItem("rfb.input-preset", "numpad"); setTimeout(() => location.reload(), 50); return true;');
  await driver.waitFor('return !window.__mindReload && document.documentElement.dataset.appMode === "title"', "Chinese title", 60_000);
  const sources = Object.fromEntries(await Promise.all(["zh-CN", "en-US"].map(async locale => [locale,
    await Promise.all(["ui", "content", "game"].map(file => readFile(new URL(`../../locales/${locale}/${file}.ftl`, import.meta.url), "utf8"))),
  ])));
  const localization = new Localization("zh-CN", sources);
  const checks = [];
  const click = selector => driver.execute('document.querySelector(arguments[0]).click(); return true;', [selector]);
  const focusIs = selector => driver.execute('return document.activeElement.matches(arguments[0]);', [selector]);
  const hash = () => driver.execute('return document.querySelector("#hash-value").title;');
  const ready = () => driver.waitFor('return document.querySelector("#connection-status").classList.contains("ready")', "ready UI");
  const row = slug => `[data-ability-id="demo.ability.mindcrafter-${slug}"]`;
  async function tabTo(selector) {
    for (let step = 0; step < 50; step++) {
      if (await focusIs(selector)) return;
      await keyboard.key("Tab");
    }
    throw new Error(`Tab could not reach ${selector}`);
  }
  async function invoke(command, args) {
    await driver.execute(`window.__mindInvokeDone = false; window.__mindInvokeError = null;
      window.__TAURI_INTERNALS__.invoke(arguments[0], arguments[1]).then(() => window.__mindInvokeDone = true, error => window.__mindInvokeError = String(error)); return true;`, [command, args]);
    await driver.waitFor('return window.__mindInvokeDone || window.__mindInvokeError', command);
    assert.equal(await driver.execute('return window.__mindInvokeError'), null);
  }
  async function viewport(width, height, zoom = 1) {
    await invoke("plugin:webview|set_webview_zoom", { label: "main", value: zoom });
    const rect = await driver.command("GET", "/window/rect");
    const current = await driver.execute('return { width: innerWidth, height: innerHeight }');
    await driver.command("POST", "/window/rect", { width: rect.width + (width - current.width) * zoom, height: rect.height + (height - current.height) * zoom });
    await driver.waitFor('return innerWidth === arguments[0] && innerHeight === arguments[1]', `${width}x${height}`, 10_000, [width, height]);
    await driver.execute('window.__mindLayoutReady = false; requestAnimationFrame(() => requestAnimationFrame(() => window.__mindLayoutReady = true)); return true;');
    await driver.waitFor('return window.__mindLayoutReady', "responsive layout settled");
  }
  async function screenshot(name) {
    await writeFile(path.join(directory, `mindcrafter-${name}.png`), await driver.screenshot(), "base64");
  }
  async function abilitiesPage() {
    const open = await driver.execute('return document.querySelector("#player-page-dialog").open');
    await click(open ? "#player-page-tab-ability" : "#player-ui-ability-open");
    await driver.waitFor('return document.querySelector("#ability-list").checkVisibility()', "ability page");
  }
  async function prepareLevel(level) {
    await driver.execute(`window.__mindPrepared = null; window.__mindPrepareError = null;
      (async () => {
        const snapshot = await window.__TAURI_INTERNALS__.invoke("prepare_mindcrafter_e2e", { level: arguments[0] });
        const bytes = await window.__TAURI_INTERNALS__.invoke("save_game", { savedAt: "2026-09-10T12:00:00Z" });
        const files = new DataTransfer(); files.items.add(new File([new Uint8Array(bytes)], "mindcrafter-ui.rfbsave"));
        const input = document.querySelector("#load-input"); input.files = files.files;
        input.dispatchEvent(new Event("change", { bubbles: true }));
        window.__mindPrepared = { level: snapshot.player.progress.level, hash: snapshot.stateHash, abilities: snapshot.player.abilities };
      })().catch(error => window.__mindPrepareError = String(error)); return true;`, [level]);
    await driver.waitFor('return window.__mindPrepared || window.__mindPrepareError', "real level and save preparation", 30_000);
    assert.equal(await driver.execute('return window.__mindPrepareError'), null);
    const prepared = await driver.execute('return window.__mindPrepared');
    assert.equal(prepared.level, level);
    await driver.waitFor('return document.querySelector("#hash-value").title === arguments[0] && document.querySelector("#connection-status").classList.contains("ready")', "exact native save loaded", 30_000, [prepared.hash]);
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
      assert.equal(rendered.summary, localization.format(ability.governingAttribute ? "ability-summary-governed" : "ability-summary", {
        level: ability.minimumLevel, attribute: ability.governingAttribute?.toUpperCase().slice(0, 3) ?? "",
        cost: ability.resourceCost, baseCost: ability.baseResourceCost, failure: ability.failurePercent,
      }));
      assert.equal(rendered.actions, 1, `${ability.id}: no study/forget for automatic powers`);
      assert.equal(rendered.disabled, !ability.canCast);
      assert.ok(rendered.text.includes(localization.format(ability.targetSpec.range > 0 ? "ability-target-range-summary" : "ability-target-summary", {
        modes: ability.targetSpec.modes.map(mode => localization.format(`ability-target-${mode}`)).join(" / "),
        range: ability.targetSpec.range,
      })), `${ability.id}: authoritative target modes and range`);
      assert.doesNotMatch(rendered.text, /熟练度|Proficiency|\[[a-z][a-z-]+\]/);
      if (ability.unavailableReason) assert.ok(rendered.text.includes(localization.format(`ability-unavailable-${ability.unavailableReason}`)));
    }
    checks.push({ locale: localization.locale, level: prepared.level, abilities: actual.map(value => ({ id: value.id, name: value.name, summary: value.summary })), hash: prepared.hash });
  }
  try {
    await invoke("plugin:window|set_min_size", { label: "main", value: null });
    await viewport(1280, 720);
    await tabTo("#session-new-game"); await keyboard.key("Enter");
    await keyboard.key("a", 2); await keyboard.text("心灵旅人");
    await tabTo("#session-seed"); await keyboard.key("a", 2); await keyboard.text("924");
    await tabTo("#session-tab-overview"); await keyboard.key("ArrowRight"); await keyboard.key("ArrowRight");
    await tabTo('[data-career-group="melee"]'); await keyboard.key("End");
    assert.equal(await focusIs('[data-career-group="mind"]'), true);
    await keyboard.key("Enter"); await tabTo('[data-career-id="demo.build.mindcrafter"]'); await keyboard.key("Enter");
    assert.equal(await driver.execute('return document.querySelector("#session-start-game").disabled'), false);
    assert.equal(await driver.execute('return document.querySelectorAll("#session-career-options .session-menu-parent").length'), 0);
    assert.match(await driver.execute('return document.querySelector("#session-creation-summary").textContent'), /心灵旅人.*心灵术士/);
    await viewport(390, 844);
    await tabTo('#session-page-career [data-menu-view="details"]'); await keyboard.key("Enter");
    assert.equal(await driver.execute('const node = document.querySelector("#session-career-description"); return node.scrollWidth <= node.clientWidth'), true);
    await screenshot("creation-390x844");
    await keyboard.key("Escape");
    assert.equal(await focusIs('[data-career-id="demo.build.mindcrafter"]'), true);
    await viewport(1280, 720);
    await tabTo("#session-start-game"); await keyboard.key("Enter");
    await driver.waitFor('return document.documentElement.dataset.appMode === "playing" && document.querySelector("#connection-status").classList.contains("ready")', "new human Mindcrafter", 60_000);
    assert.equal(await driver.execute('return document.querySelector("#app").dataset.sessionBuildId'), "demo.build.mindcrafter");
    const bornHash = await hash();
    await abilitiesPage();
    await tabTo(`${row("neural-blast")} .ability-cast-action`); await keyboard.key("Enter");
    await driver.waitFor('return !document.querySelector("#target-cursor").hidden', "birth spell target");
    await keyboard.key("6"); await keyboard.key("Enter");
    await driver.waitFor('return document.querySelector("#hash-value").title !== arguments[0]', "unmodified birth spell cast", 10_000, [bornHash]);
    await ready();
    const birthCast = { hash: await hash(), messages: await driver.execute('return document.querySelector("#message-list").textContent') };
    assert.match(birthCast.messages, /你成功施放了神经爆破/);
    const birthPosition = await driver.execute('return document.querySelector("#position-value").textContent');
    await keyboard.key("2");
    await driver.waitFor('return document.querySelector("#position-value").textContent !== arguments[0]', "normal movement", 10_000, [birthPosition]);
    await ready();
    const movedPosition = await driver.execute('return document.querySelector("#position-value").textContent');
    await screenshot("birth-cast-move");
    checks.push({ bornHash, birthCast, birthPosition, movedPosition, precondition: "Normal new character, no debug preparation or forced success" });
    const initial = await prepareLevel(1);
    await checkAbilities(initial);
    assert.equal(initial.abilities.filter(ability => ability.minimumLevel <= 1).length, 1);
    await tabTo(`${row("neural-blast")} .ability-cast-action`); await keyboard.key("Enter");
    await driver.waitFor('return !document.querySelector("#target-cursor").hidden', "keyboard targeting");
    await keyboard.key("Escape");
    assert.equal(await hash(), initial.hash, "cancelled targeting keeps authoritative state");

    await prepareLevel(3);
    const lowLevelCasts = [];
    for (const slug of ["precognition", "minor-displacement"]) {
      const name = slug === "precognition" ? "预知" : "微级位移";
      let succeeded = false;
      // Keep natural casting rolls: a reported spell failure spends an action,
      // then the player may cast again; transport/assertion errors still fail immediately.
      for (let attempt = 0; attempt < 5 && !succeeded; attempt++) {
        await abilitiesPage();
        const before = await hash();
        const position = await driver.execute('return document.querySelector("#position-value").textContent');
        await tabTo(`${row(slug)} .ability-cast-action`); await keyboard.key("Enter");
        await driver.waitFor('return document.querySelector("#hash-value").title !== arguments[0]', `level-three ${slug}`, 10_000, [before]);
        await ready();
        const messages = await driver.execute('return document.querySelector("#message-list").textContent');
        succeeded = messages.includes(`你成功施放了${name}`);
        const afterPosition = await driver.execute('return document.querySelector("#position-value").textContent');
        if (succeeded && slug === "minor-displacement") assert.notEqual(afterPosition, position);
        if (!succeeded) {
          assert.ok(messages.includes(`你施放${name}失败了`));
          assert.equal(afterPosition, position);
        }
        lowLevelCasts.push({ slug, succeeded, before, after: await hash(), position, afterPosition });
      }
      assert.ok(succeeded, `${name} must actually take effect`);
    }
    await driver.execute(`window.__mindDownload = null;
      URL.createObjectURL = blob => { window.__mindDownload = { blob }; return "blob:mindcrafter-acceptance"; };
      URL.revokeObjectURL = () => {};
      HTMLAnchorElement.prototype.click = function () { window.__mindDownload.name = this.download; };
      document.querySelector('.hud-menu').open = true; return true;`);
    const savedHash = await hash(); await click("#save-button");
    await driver.waitFor('return window.__mindDownload?.name?.endsWith(".rfbsave")', "save menu exports a native save");
    await driver.execute('document.querySelector(".hud-menu").open = false; return true;');
    await keyboard.key("5");
    await driver.waitFor('return document.querySelector("#hash-value").title !== arguments[0]', "continue before restore", 10_000, [savedHash]);
    await ready(); const continuedHash = await hash();
    await driver.execute(`const saved = window.__mindDownload;
      const files = new DataTransfer(); files.items.add(new File([saved.blob], saved.name));
      const input = document.querySelector('#load-input'); input.files = files.files;
      input.dispatchEvent(new Event('change', { bubbles: true })); return true;`);
    await driver.waitFor('return document.querySelector("#hash-value").title === arguments[0]', "restore exact played character", 30_000, [savedHash]);
    await ready(); await keyboard.key("5");
    await driver.waitFor('return document.querySelector("#hash-value").title === arguments[0]', "deterministic continuation after restore", 10_000, [continuedHash]);
    checks.push({ lowLevelCasts, savedHash, continuedHash, precondition: "Real experience grant to level 3; UI detection/teleport, save menu, file restore and identical continued action" });

    for (const level of [19, 20, 24, 25, 29, 30, 44, 45]) {
      const prepared = await prepareLevel(level);
      await checkAbilities(prepared);
      if (level === 19 || level === 20) assert.equal(prepared.abilities.find(ability => ability.id.endsWith("-psychometry")).effects[0].type, level === 20 ? "identify-item" : "psychometry");
      if (level === 29 || level === 30) assert.deepEqual(prepared.abilities.find(ability => ability.id.endsWith("-domination")).targetSpec.modes, level === 30 ? ["self"] : ["direction", "position"]);
      if (level === 44 || level === 45) {
        const door = prepared.abilities.find(ability => ability.id.endsWith("-minor-displacement"));
        assert.equal(door.resourceCost, level === 45 ? 42 : 2);
        assert.deepEqual(door.targetSpec.modes, level === 45 ? ["position"] : ["self"]);
      }
    }
    // Resolve the normal Human talent prompt before continuing real commands.
    while (await driver.execute('return !!document.querySelector(".mutation-choice-candidate")')) {
      await click("#player-page-tab-character"); await click("#character-tab-other");
      const before = await hash(); await click(".mutation-choice-candidate");
      await driver.waitFor('return document.querySelector("#hash-value").title !== arguments[0]', "human talent selected", 10_000, [before]);
      await ready();
    }
    const final = await prepareLevel(45);
    await checkAbilities(final);
    await viewport(390, 844);
    const layout = await driver.execute(`const dialog = document.querySelector('#player-page-dialog');
      const list = document.querySelector('#ability-list'); return { viewport:[innerWidth,innerHeight], dialogWidth:dialog.getBoundingClientRect().width,
      overflow:list.scrollWidth > list.clientWidth, missingButtons:[...list.querySelectorAll('.ability-cast-action')].filter(button => button.getBoundingClientRect().width < 1).length };`);
    assert.equal(layout.overflow, false); assert.equal(layout.missingButtons, 0); assert.ok(layout.dialogWidth <= 390);
    await screenshot("abilities-45-390x844");
    await viewport(1280, 720);
    await screenshot("abilities-45-1280x720");
    await tabTo(`${row("minor-displacement")} .ability-cast-action`); await keyboard.key("Enter");
    await driver.waitFor('return !document.querySelector("#target-cursor").hidden', "upgraded dimension door targets a grid");
    await keyboard.key("Escape"); assert.equal(await hash(), final.hash);
    await abilitiesPage();
    const before = await hash();
    await tabTo(`${row("precognition")} .ability-cast-action`); await keyboard.key("Enter");
    await driver.waitFor('return document.querySelector("#hash-value").title !== arguments[0]', "cast after native save restoration", 10_000, [before]);
    await ready();
    assert.doesNotMatch(await driver.execute('return document.querySelector("#message-list").textContent'), /\[ability-|未知事件/);
    const afterLoadCast = await hash();
    // Spend real mana until the high-cost power becomes unavailable.
    for (let attempt = 0; attempt < 60; attempt++) {
      await abilitiesPage();
      if (await driver.execute('return document.querySelector(arguments[0]).disabled', [`${row("psycho-storm")} .ability-cast-action`])) break;
      const before = await hash(); await click(`${row("precognition")} .ability-cast-action`);
      await driver.waitFor('return document.querySelector("#hash-value").title !== arguments[0]', "mana-consuming cast", 10_000, [before]);
      await ready();
    }
    const reason = localization.format("ability-disabled-summary", { reason: localization.format("ability-unavailable-insufficient-resource") });
    assert.ok((await driver.execute('return document.querySelector(arguments[0]).textContent', [row("psycho-storm")])).includes(reason));
    await screenshot("unavailable-zh-CN");
    await driver.execute('const select = document.querySelector("#language-select"); select.value = "en-US"; select.dispatchEvent(new Event("change", { bubbles: true })); return true;');
    await ready(); localization.setLocale("en-US");
    const englishReason = localization.format("ability-disabled-summary", { reason: localization.format("ability-unavailable-insufficient-resource") });
    assert.ok((await driver.execute('return document.querySelector(arguments[0]).textContent', [row("psycho-storm")])).includes(englishReason));
    const english = await prepareLevel(45);
    await checkAbilities(english);
    await viewport(640, 360, 2);
    assert.equal(await driver.execute('const list = document.querySelector("#ability-list"); return list.scrollWidth <= list.clientWidth'), true);
    await screenshot("abilities-45-en-US-200percent");
    await keyboard.reload();
    await driver.waitFor('return document.documentElement.dataset.appMode === "title"', "playing renderer disposed on reload", 30_000);
    assert.deepEqual(keyboard.errors, [], "title and playing reloads must not throw renderer cleanup errors");
    checks.push({ layout, afterLoadCast, englishZoom: 2, insufficientMana: [reason, englishReason], keyboard: "WebView2 CDP native Tab, arrows, Enter and Escape", levelSetup: "WebDriver-only experience grants; all level snapshots loaded through native save validation" });
    await writeFile(path.join(directory, "mindcrafter-ui-acceptance.json"), JSON.stringify(checks, null, 2));
  } finally { keyboard.close(); }
}
