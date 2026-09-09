// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";

// Explicit milestone only: generate ego-desktop.rfbsave with the ignored core exporter first.
export async function runEgoScenario(driver, directory) {
  const checks = [];
  async function click(selector) {
    await driver.waitFor(`const button = document.querySelector(arguments[0]); return !!button && !button.disabled;`, selector, 10_000, [selector]);
    await driver.execute(`document.querySelector(arguments[0]).click(); return true;`, [selector]);
  }
  const turn = () => driver.execute(`return parseInt(document.querySelector("#turn-value").textContent, 10);`);
  const after = previous => driver.waitFor(`return parseInt(document.querySelector("#turn-value").textContent, 10) > arguments[0] && document.querySelector("#connection-status").classList.contains("ready");`, "committed Ego action", 10_000, [previous]);
  const row = id => `#inventory-list [data-item-id="${id}"]`;
  async function select(id) {
    await driver.execute(`
      for (const input of document.querySelectorAll('#inventory-list input:checked')) input.click();
      const input = document.querySelector(arguments[0] + ' input'); if (!input) throw new Error("Missing " + arguments[0]); input.click(); return true;
    `, [row(id)]);
  }
  async function details(id) {
    await click(`${row(id)} .inventory-item-inspect`);
    await driver.waitFor(`return document.querySelector("#inventory-detail-dialog").open;`, "Ego details");
    const result = await driver.execute(`return { name: document.querySelector('#inventory-detail-body .inventory-item-name').textContent, text: document.querySelector('#inventory-detail-body').textContent, properties: [...document.querySelectorAll('#inventory-detail-body .item-property')].map(item => item.textContent), identified: !document.querySelector('#inventory-detail-body .item-identification') };`);
    await click("#inventory-detail-close");
    return result;
  }
  async function useOn(source, target) {
    await select(source);
    const before = await turn();
    await click("#inventory-use");
    await driver.waitFor(`return !!document.querySelector('.item-target-dialog[open] select');`, "item target selection");
    await driver.execute(`const select = document.querySelector('.item-target-dialog[open] select'); select.value = arguments[0]; if (select.value !== arguments[0]) throw new Error('Target missing'); select.dispatchEvent(new Event('change', {bubbles:true})); return true;`, [target]);
    await click('.item-target-dialog[open] button[type="submit"]');
    await after(before);
  }
  await driver.waitFor(`return document.documentElement.dataset.appMode === 'title';`, "title", 60_000);
  await click("#session-new-game");
  await click("#session-start-game");
  await driver.waitFor(`return document.documentElement.dataset.appMode === 'playing' && document.querySelector('#connection-status').classList.contains('ready');`, "new game before save import", 60_000);
  const fixture = (await readFile(path.join(directory, "ego-desktop.rfbsave"))).toString("base64");
  const initialHash = await driver.execute(`return document.querySelector('#hash-value').title;`);
  await driver.execute(`
    window.__egoErrors = []; window.addEventListener('error', event => window.__egoErrors.push(event.message));
    const bytes = Uint8Array.from(atob(arguments[0]), c => c.charCodeAt(0));
    const transfer = new DataTransfer(); transfer.items.add(new File([bytes], 'ego-desktop.rfbsave'));
    const input = document.querySelector('#load-input'); input.files = transfer.files;
    input.dispatchEvent(new Event('change', {bubbles:true})); return true;
  `, [fixture]);
  await driver.waitFor(`return document.querySelector('#hash-value').title !== arguments[0] && document.querySelector('#connection-status').classList.contains('ready');`, "Ego fixture load", 60_000, [initialHash]);
  for (let pickup = 0; pickup < 3; pickup++) {
    const beforePickup = await turn();
    await driver.execute(`document.dispatchEvent(new KeyboardEvent('keydown', {key:'g',code:'KeyG',bubbles:true})); return true;`);
    await after(beforePickup);
  }
  await click("#player-ui-inventory-open");
  for (const id of ["e8.light", "e8.ring", "e8.quiver"]) {
    await driver.waitFor(`return !!document.querySelector(arguments[0]);`, "picked up Ego", 10_000, [row(id)]);
    assert.equal((await details(id)).identified, false, `${id} starts unknown`);
    // Only a successful revelation scroll use satisfies the knowledge check.
    let identified;
    for (let attempt = 0; attempt < 6; attempt++) {
      await useOn("e8.identify", id);
      identified = await details(id);
      if (identified.identified) break;
    }
    assert.equal(identified.identified, true, `${id} identified with the revelation scroll`);
    assert.ok(identified.properties.length > 0);
    assert.doesNotMatch(identified.properties.join(" "), /[&~]/);
    await select(id);
    const beforeEquip = await turn();
    await click("#inventory-equip");
    await driver.execute(`document.querySelector('.item-target-dialog[open] button[type="submit"]')?.click(); return true;`);
    await after(beforeEquip);
    await driver.waitFor(`return !document.querySelector(arguments[0]);`, "Ego equipped", 10_000, [row(id)]);
    checks.push({ check: "pickup-identify-equip", id, details: identified });
  }
  await driver.execute(`const button = [...document.querySelectorAll('#equipment-list .equipment-item button')].find(button => button.textContent.includes(arguments[0])); if (!button) throw new Error('Equipped light missing'); button.click(); return true;`, [checks[0].details.name]);
  await driver.waitFor(`return !!document.querySelector('[data-activation-item-id="e8.light"]');`, "light activation");
  const chargesBefore = await driver.execute(`return document.querySelector('#inventory-detail-body .inventory-charges').textContent;`);
  for (let attempt = 0; attempt < 6; attempt++) {
    const before = await turn();
    await click('[data-activation-item-id="e8.light"]');
    await after(before);
    if (await driver.execute(`return document.querySelector('#inventory-detail-body .inventory-charges').textContent !== arguments[0];`, [chargesBefore])) break;
  }
  const chargesAfter = await driver.execute(`return document.querySelector('#inventory-detail-body .inventory-charges').textContent;`);
  assert.notEqual(chargesAfter, chargesBefore);
  checks.push({ check: "ego-activation-consumes-energy", chargesBefore, chargesAfter });
  await click("#inventory-detail-close");
  await useOn("e8.craft", "e8.dagger");
  const crafted = await details("e8.dagger");
  assert.equal(crafted.identified, true);
  assert.ok(crafted.properties.length > 0, "Craft materializes a known Ego");
  assert.equal(await driver.execute(`return !!document.querySelector(arguments[0]);`, [row("e8.craft")]), false);
  checks.push({ check: "craft-success-full-identification-and-ego", details: crafted });
  await click("#player-page-close");
  await driver.execute(`
    URL.createObjectURL = blob => { window.__egoSave = blob; return 'blob:ego-save'; }; URL.revokeObjectURL = () => {};
    HTMLAnchorElement.prototype.click = function () { window.__egoSaveName = this.download; };
    document.querySelector('.hud-menu').open = true; return true;
  `);
  const saved = await driver.execute(`return {hash:document.querySelector('#hash-value').title, equipment:document.querySelector('#equipment-list').textContent};`);
  await click("#save-button");
  await driver.waitFor(`return window.__egoSaveName?.endsWith('.rfbsave');`, "Ego save export");
  await driver.execute(`document.querySelector('.hud-menu').open = false; document.dispatchEvent(new KeyboardEvent('keydown', {key:'5',code:'Numpad5',bubbles:true})); return true;`);
  await driver.waitFor(`return document.querySelector('#hash-value').title !== arguments[0];`, "post-save action", 10_000, [saved.hash]);
  await driver.execute(`const transfer = new DataTransfer(); transfer.items.add(new File([window.__egoSave], window.__egoSaveName)); const input = document.querySelector('#load-input'); input.files = transfer.files; input.dispatchEvent(new Event('change', {bubbles:true})); return true;`);
  await driver.waitFor(`return document.querySelector('#hash-value').title === arguments[0];`, "Ego exact save restoration", 10_000, [saved.hash]);
  assert.equal(await driver.execute(`return document.querySelector('#equipment-list').textContent;`), saved.equipment);
  assert.deepEqual(await driver.execute(`return window.__egoErrors;`), []);
  checks.push({ check: "save-restores-exact-hash-and-equipment", hash: saved.hash });
  await writeFile(path.join(directory, "ego-desktop-report.json"), JSON.stringify({ checks }, null, 2) + "\n");
  await writeFile(path.join(directory, "ego-desktop.png"), await driver.screenshot(), "base64");
}
