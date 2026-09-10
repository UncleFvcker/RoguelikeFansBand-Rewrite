// SPDX-License-Identifier: MPL-2.0

import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

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
  const screenshot = async (name) => writeFile(path.join(artifactDirectory, `creation-${name}.png`), await driver.screenshot(), "base64");
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
  await fill("#session-race", "rfb-legacy.race.draconian-red");
  await click("#session-tab-career");
  await click("#session-build-high-mage-death");
  await checkFrame("career");
  await screenshot("zh-CN-career");
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
  await click("#session-start-game");
  await driver.waitFor(`return document.documentElement.dataset.appMode === "playing" && document.querySelector("#connection-status").classList.contains("ready")`, "created character", 60_000);
  const identity = await driver.execute(`return { name: document.querySelector("#character-name-value").textContent, race: document.querySelector("#character-race-value").textContent, career: document.querySelector("#character-class-value").textContent, build: document.querySelector("#app").dataset.sessionBuildId, seed: document.querySelector("#app").dataset.sessionSeed }`);
  assert.equal(identity.name, "面板验收");
  assert.match(identity.race, /红色/);
  assert.equal(identity.build, "demo.build.high-mage-death");
  assert.equal(identity.seed, "83");
  await writeFile(path.join(artifactDirectory, "character-creation-acceptance.json"), JSON.stringify({ measurements, identity }, null, 2));
}
