// SPDX-License-Identifier: MPL-2.0

import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { RACE_GROUPS, PLAYTEST_RACE_IDS, CAREER_GROUPS, PLAYTEST_BUILD_IDS } from "../src/character-creation.ts";

export async function selectCreationRace(driver, raceId) {
  const group = RACE_GROUPS.find(group => group.options.some(entry => entry.id === raceId || ("children" in entry && entry.children.some(child => child.id === raceId))));
  assert.ok(group, `Creation race missing: ${raceId}`);
  await driver.execute(`
    document.querySelector("#session-tab-race").click();
    document.querySelector('[data-race-group="' + arguments[0] + '"]').click();
    if (arguments[1].startsWith("rfb-legacy.race.draconian-")) document.querySelector('[data-race-id="draconian"]').click();
    const button = document.querySelector('[data-race-id="' + arguments[1] + '"]');
    if (!button || !button.checkVisibility()) throw new Error("Race option is not visible");
    button.focus(); button.click();
    return true;
  `, [group.id, raceId]);
}

export async function selectCreationBuild(driver, buildId) {
  const group = CAREER_GROUPS.find(group => group.options.some(entry => entry.id === buildId || ("children" in entry && entry.children.some(child => child.id === buildId))));
  assert.ok(group, `Creation build missing: ${buildId}`);
  const parent = group.options.find(entry => "children" in entry && entry.children.some(child => child.id === buildId));
  await driver.execute(`
    document.querySelector("#session-tab-career").click();
    document.querySelector('[data-career-group="' + arguments[0] + '"]').click();
    if (arguments[2]) document.querySelector('[data-career-id="' + arguments[2] + '"]').click();
    const button = document.querySelector('[data-career-id="' + arguments[1] + '"]');
    if (!button || !button.checkVisibility()) throw new Error("Career option is not visible");
    button.focus(); button.click();
    return true;
  `, [group.id, buildId, parent?.id]);
}

export async function runCharacterCreationScenario(driver, artifactDirectory) {
  await mkdir(artifactDirectory, { recursive: true });
  await driver.waitFor(`return document.documentElement.dataset.appMode === "title" && !document.querySelector("#session-new-game").disabled`, "ready title", 60_000);
  const rect = await driver.command("GET", "/window/rect");
  const viewport = await driver.execute("return { width: innerWidth, height: innerHeight }");
  await driver.command("POST", "/window/rect", { width: rect.width + 1280 - viewport.width, height: rect.height + 720 - viewport.height });
  await driver.waitFor("return innerWidth === 1280 && innerHeight === 720", "1280x720 viewport");

  const click = (selector) => driver.execute(`document.querySelector(arguments[0]).click(); return true;`, [selector]);
  const fill = (selector, value) => driver.execute(`const input = document.querySelector(arguments[0]); input.value = arguments[1]; input.dispatchEvent(new Event("input", { bubbles: true })); input.dispatchEvent(new Event("change", { bubbles: true })); return true;`, [selector, value]);
  const key = (selector, value) => driver.execute(`const node = document.querySelector(arguments[0]); node.focus(); node.dispatchEvent(new KeyboardEvent("keydown", { key: arguments[1], bubbles: true, cancelable: true })); return true;`, [selector, value]);
  const screenshot = async (name) => {
    await new Promise(resolve => setTimeout(resolve, 150)); // Let the existing focus/selection transition settle.
    await writeFile(path.join(artifactDirectory, `creation-${name}.png`), await driver.screenshot(), "base64");
  };
  const measurements = [];
  async function checkFrame(page) {
    const layout = await driver.execute(`
      const rect = selector => { const r = document.querySelector(selector).getBoundingClientRect(); return { x:r.x, y:r.y, width:r.width, height:r.height, right:r.right, bottom:r.bottom }; };
      return { viewport: [innerWidth, innerHeight], frame: rect(".session-card"), start: rect("#session-start-game"), name: rect("#session-character-name"), visiblePages: [...document.querySelectorAll("[data-creation-panel]")].filter(p => !p.hidden).map(p => p.dataset.creationPanel), scrollWidth: document.documentElement.scrollWidth, scrollHeight: document.documentElement.scrollHeight };
    `);
    assert.deepEqual(layout.visiblePages, [page]);
    assert.ok(Math.abs(layout.frame.width - 1280 * 0.84) < 2);
    assert.ok(Math.abs(layout.frame.height - 720 * 0.84) < 2);
    for (const item of [layout.start, ...(page === "overview" ? [layout.name] : [])]) {
      assert.ok(item.width > 0 && item.height > 0);
      assert.ok(item.x >= layout.frame.x && item.right <= layout.frame.right);
      assert.ok(item.y >= layout.frame.y && item.bottom <= layout.frame.bottom);
    }
    assert.equal(layout.scrollWidth, 1280);
    assert.equal(layout.scrollHeight, 720);
    measurements.push({ page, ...layout });
  }

  for (const locale of ["zh-CN", "en-US"]) {
    await click("#session-settings");
    await fill("#session-settings-language", locale);
    await click("#session-settings-back");
    await click("#session-new-game");
    await checkFrame("overview");
    await screenshot(`${locale}-overview`);
    if (locale === "en-US") {
      await selectCreationRace(driver, "rfb-legacy.race.draconian-red");
      await checkFrame("race");
      await screenshot("en-US-subrace");
      await selectCreationRace(driver, "demo.race.rfb-human");
      await selectCreationBuild(driver, "demo.build.paladin-death");
      await checkFrame("career");
      assert.match(await driver.execute('return document.querySelector("#session-career-notes").textContent'), /Currently available realm: Death/);
      await screenshot("en-US-career-realm");
      await selectCreationBuild(driver, "demo.build.warrior");
    }
    await click("#session-new-game-back");
  }
  await click("#session-settings");
  await fill("#session-settings-language", "zh-CN");
  await click("#session-settings-back");
  await click("#session-new-game");
  await fill("#session-character-name", "面板验收");
  await fill("#session-seed", "18446744073709551615");
  await key("#session-tab-overview", "ArrowRight");
  await checkFrame("race");
  assert.equal(await driver.execute("return document.activeElement.id"), "session-tab-race");
  const beforePreview = await driver.execute(`return document.querySelector("#session-creation-summary").textContent`);
  await key('[data-race-id="demo.race.rfb-human"]', "Home");
  assert.equal(await driver.execute(`return document.querySelector("#session-creation-summary").textContent`), beforePreview);
  assert.equal(await driver.execute(`return document.querySelector("#session-race-detail-title").textContent`), "安珀人");
  await click('[data-race-group="other"]');
  await click('[data-race-id="draconian"]');
  assert.equal(await driver.execute(`return document.querySelector("#session-start-game").disabled`), true);
  await key("#session-tab-race", "Escape");
  assert.equal(await driver.execute(`return document.activeElement.dataset.raceId`), "draconian");
  assert.equal(await driver.execute(`return document.querySelector("#session-start-game").disabled`), false);
  assert.equal(await driver.execute(`return document.querySelector("#session-creation-summary").textContent`), beforePreview);
  await click('[data-race-id="draconian"]');
  await click("#session-tab-overview");
  assert.equal(await driver.execute(`return document.querySelector("#session-start-game").disabled`), false);
  assert.equal(await driver.execute(`return document.querySelector("#session-creation-summary").textContent`), beforePreview);

  const visited = [];
  for (const id of PLAYTEST_RACE_IDS) {
    await selectCreationRace(driver, id);
    const selected = await driver.execute(`
      const button = document.querySelector('[data-race-id="' + arguments[0] + '"]');
      const description = document.querySelector("#session-race-description");
      return { selected: button.getAttribute("aria-pressed"), selectedCount: document.querySelectorAll('#session-race-options [aria-pressed="true"]').length, pending: document.querySelector("#session-start-game").disabled, name: document.querySelector("#session-race-detail-title").textContent, summary: document.querySelector("#session-creation-summary").textContent, description: description.textContent, notes: document.querySelector("#session-race-notes").children.length, fits: description.scrollWidth <= description.clientWidth };
    `, [id]);
    assert.equal(selected.selected, "true");
    assert.equal(selected.selectedCount, 1);
    assert.equal(selected.pending, false);
    assert.ok(selected.summary.includes(selected.name));
    assert.ok(selected.description.length > 20 && !selected.description.startsWith("["));
    assert.equal(selected.fits, true);
    const specialNotes = { "rfb-legacy.race.tomte": 1, "rfb-legacy.race.tonberry": 6, "rfb-legacy.race.ent": 8, "rfb-legacy.race.spectre": 8 };
    if (id in specialNotes) {
      assert.equal(selected.notes, specialNotes[id]);
      await screenshot(`race-${id.split(".").at(-1)}`);
    }
    visited.push(id);
  }
  assert.equal(new Set(visited).size, 46);
  await selectCreationRace(driver, "rfb-legacy.race.draconian-red");
  await checkFrame("race");
  await screenshot("zh-CN-subrace");
  await selectCreationRace(driver, "demo.race.rfb-human");
  assert.doesNotMatch(await driver.execute(`return document.querySelector("#session-creation-summary").textContent`), /龙人分支/);
  await selectCreationRace(driver, "rfb-legacy.race.draconian-red");
  await click("#session-tab-overview");
  await click('[data-creation-page="race"]:not([role="tab"])');
  assert.match(await driver.execute(`return document.querySelector("#session-race-path").textContent`), /龙人分支/);
  assert.equal(await driver.execute(`return document.querySelector('[data-race-id="rfb-legacy.race.draconian-red"]').getAttribute("aria-pressed")`), "true");
  await click("#session-tab-career");
  const beforeCareer = await driver.execute('return document.querySelector("#session-creation-summary").textContent');
  await click('[data-career-group="archery"]');
  await key('[data-career-id="demo.build.archer"]', "End");
  assert.equal(await driver.execute('return document.querySelector("#session-creation-summary").textContent'), beforeCareer);
  assert.equal(await driver.execute('return document.querySelector("#session-career-detail-title").textContent'), "狙击手");
  await driver.execute(`document.querySelector('[data-career-id="demo.build.archer"]').dispatchEvent(new PointerEvent("pointermove", { bubbles:true, movementX:1 })); return true;`);
  assert.equal(await driver.execute('return document.querySelector("#session-career-detail-title").textContent'), "弓箭手");
  assert.equal(await driver.execute('return document.querySelector("#session-creation-summary").textContent'), beforeCareer);
  for (const [group, parent] of [["magic", "high-mage"], ["hybrid", "paladin"]]) {
    await click(`[data-career-group="${group}"]`);
    await click(`[data-career-id="${parent}"]`);
    assert.equal(await driver.execute('return document.querySelector("#session-start-game").disabled'), true);
    assert.equal(await driver.execute('return document.querySelector("#session-career-options").children.length'), 1);
    await driver.execute('document.querySelector("#session-new-game-view").requestSubmit(); return true;');
    assert.equal(await driver.execute('return document.documentElement.dataset.appMode'), "new-game");
    await key("#session-tab-career", "Escape");
    assert.equal(await driver.execute('return document.activeElement.dataset.careerId'), parent);
    assert.equal(await driver.execute('return document.querySelector("#session-start-game").disabled'), false);
    assert.equal(await driver.execute('return document.querySelector("#session-creation-summary").textContent'), beforeCareer);
    await click(`[data-career-id="${parent}"]`);
    await click("#session-tab-overview");
    assert.equal(await driver.execute('return document.querySelector("#session-start-game").disabled'), false);
    await click('[data-creation-page="career"]:not([role="tab"])');
    assert.equal(await driver.execute('return document.querySelector("#session-career-path").textContent'), "职业 › 近战");
  }
  const visitedBuilds = [];
  for (const build of PLAYTEST_BUILD_IDS) {
    await selectCreationBuild(driver, build);
    const selected = await driver.execute(`return {
      selected: document.querySelector('[data-career-id="' + arguments[0] + '"]').getAttribute("aria-pressed"),
      summary: document.querySelector("#session-creation-summary").textContent,
      title: document.querySelector("#session-career-detail-title").textContent,
      description: document.querySelector("#session-career-description").textContent,
      pending: document.querySelector("#session-start-game").disabled,
      selectedCount: document.querySelectorAll('#session-career-options [aria-pressed="true"]').length,
    };`, [build]);
    assert.equal(selected.selected, "true");
    assert.equal(selected.selectedCount, 1);
    assert.equal(selected.pending, false);
    assert.ok(selected.summary.includes(selected.title));
    assert.ok(selected.description.length > 15 && !selected.description.startsWith("["));
    visitedBuilds.push(build);
  }
  await selectCreationBuild(driver, "demo.build.warrior");
  assert.doesNotMatch(await driver.execute('return document.querySelector("#session-creation-summary").textContent'), /死亡/);
  await selectCreationBuild(driver, "demo.build.high-mage-death");
  await click("#session-tab-overview");
  await click('[data-creation-page="career"]:not([role="tab"])');
  assert.equal(await driver.execute(`return document.querySelector('[data-career-id="demo.build.high-mage-death"]').getAttribute("aria-pressed")`), "true");
  await checkFrame("career");
  await screenshot("zh-CN-career");
  await key("#session-tab-career", "Escape");
  await key("#session-tab-career", "Escape");
  await checkFrame("overview");
  assert.match(await driver.execute(`return document.querySelector("#session-creation-summary").textContent`), /面板验收.*龙人分支.*红色.*高阶法师.*死亡/);
  await screenshot("zh-CN-selected");

  // Returning from another page must reveal the invalid input before focusing it.
  await fill("#session-character-name", "");
  await click("#session-tab-race");
  await click("#session-start-game");
  assert.equal(await driver.execute("return document.activeElement.id"), "session-character-name");
  await checkFrame("overview");
  await fill("#session-character-name", "面板验收");
  await fill("#session-seed", "18446744073709551616");
  await click("#session-tab-career");
  await click("#session-start-game");
  assert.equal(await driver.execute("return document.activeElement.id"), "session-seed");
  await checkFrame("overview");
  await fill("#session-seed", "83");
  await click("#session-new-game-back");
  await click("#session-new-game");
  assert.equal(await driver.execute(`return document.querySelector("#session-seed").value`), "83");
  assert.match(await driver.execute(`return document.querySelector("#session-creation-summary").textContent`), /面板验收.*龙人分支.*红色.*高阶法师.*死亡/);
  // Hold and reject the native boundary once to exercise the real shell recovery.
  await driver.execute(`
    window.__creationFetch = window.fetch;
    window.__creationUrl = window.__TAURI_INTERNALS__.convertFileSrc("initialize_game", "ipc");
    window.__creationRequests = [];
    window.fetch = (url, options) => {
      if (url !== window.__creationUrl) return window.__creationFetch(url, options);
      window.__creationRequests.push(JSON.parse(options.body));
      return new Promise(resolve => { window.__creationReject = () => resolve(new Response("creation-recovery-check", { status: 400, headers: { "Content-Type": "text/plain", "Tauri-Response": "error" } })); });
    };
    return true;
  `);
  await click("#session-start-game");
  await driver.waitFor('return typeof window.__creationReject === "function"', "pending creation");
  assert.equal(await driver.execute('return [...document.querySelectorAll("#session-shell button, #session-shell input, #session-shell select")].every(control => control.disabled)'), true);
  await driver.execute('document.querySelector("#session-new-game-view").requestSubmit(); return true;');
  assert.equal(await driver.execute('return window.__creationRequests.length'), 1);
  await driver.execute('window.__creationReject(); return true;');
  await driver.waitFor('return !document.querySelector("#session-start-game").disabled', "creation recovery");
  assert.match(await driver.execute('return document.querySelector("#session-error").textContent'), /creation-recovery-check/);
  assert.equal(await driver.execute('return document.querySelector("#session-seed").value'), "83");
  assert.match(await driver.execute('return document.querySelector("#session-creation-summary").textContent'), /面板验收.*红色.*高阶法师.*死亡/);
  await checkFrame("overview");
  await screenshot("zh-CN-creation-error");
  const failedRequest = await driver.execute('return window.__creationRequests[0]');
  assert.deepEqual({ seed: failedRequest.seed, buildId: failedRequest.buildId, raceId: failedRequest.raceId, playerName: failedRequest.playerName }, { seed: "83", buildId: "demo.build.high-mage-death", raceId: "rfb-legacy.race.draconian-red", playerName: "面板验收" });
  await driver.execute(`
    window.fetch = (url, options) => {
      if (url === window.__creationUrl) window.__creationRequests.push(JSON.parse(options.body));
      return window.__creationFetch(url, options);
    }; return true;
  `);
  await click("#session-start-game");
  await driver.waitFor(`return document.documentElement.dataset.appMode === "playing" && document.querySelector("#connection-status").classList.contains("ready")`, "created character", 60_000);
  const requests = await driver.execute('window.fetch = window.__creationFetch; return window.__creationRequests;');
  assert.equal(requests.length, 2);
  assert.deepEqual(requests.map(({ createdAt, ...request }) => request), [requests[0], requests[0]].map(({ createdAt, ...request }) => request));
  assert.equal(await driver.execute('return document.querySelector("#session-error").textContent'), "");
  const identity = await driver.execute(`return { name: document.querySelector("#character-name-value").textContent, race: document.querySelector("#character-race-value").textContent, career: document.querySelector("#character-class-value").textContent, build: document.querySelector("#app").dataset.sessionBuildId, seed: document.querySelector("#app").dataset.sessionSeed }`);
  assert.equal(identity.name, "面板验收");
  assert.match(identity.race, /红色/);
  assert.equal(identity.build, "demo.build.high-mage-death");
  assert.equal(identity.seed, "83");
  await writeFile(path.join(artifactDirectory, "character-creation-acceptance.json"), JSON.stringify({ measurements, visited, visitedBuilds, recoveryRequests: requests, identity }, null, 2));
}
