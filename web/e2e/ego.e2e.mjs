// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";

// Explicit E8.8 milestone: fresh, museum-bound saves and the normal desktop UI.
export async function runEgoScenario(driver, directory) {
  const checks = [];
  async function click(selector) {
    await driver.waitFor(`const node = document.querySelector(arguments[0]); return !!node && !node.disabled;`, selector, 10_000, [selector]);
    await driver.execute(`document.querySelector(arguments[0]).click(); return true;`, [selector]);
  }
  const hash = () => driver.execute(`return document.querySelector('#hash-value').title;`);
  const turn = () => driver.execute(`return parseInt(document.querySelector('#turn-value').textContent, 10);`);
  const after = previous => driver.waitFor(`return parseInt(document.querySelector('#turn-value').textContent, 10) > arguments[0] && document.querySelector('#connection-status').classList.contains('ready');`, "committed E8.8 action", 10_000, [previous]);
  async function key(key, code) {
    await driver.execute(`document.dispatchEvent(new KeyboardEvent('keydown', {key:arguments[0],code:arguments[1],bubbles:true})); return true;`, [key, code]);
  }
  const row = id => `#inventory-list [data-item-id="${id}"]`;
  async function select(id) {
    await driver.execute(`for (const input of document.querySelectorAll('#inventory-list input:checked')) input.click(); const input = document.querySelector(arguments[0] + ' input'); if (!input) throw new Error('Missing '+arguments[0]); input.click(); return true;`, [row(id)]);
  }
  async function openDetails(id, slot) {
    await click(slot ? `#equipment-list [data-slot-id="${slot}"] .equipment-slot-button` : `${row(id)} .inventory-item-inspect`);
    await driver.waitFor(`return document.querySelector('#inventory-detail-dialog').open;`, "item details");
  }
  const detail = () => driver.execute(`return {name:document.querySelector('#inventory-detail-body .inventory-item-name').textContent, text:document.querySelector('#inventory-detail-body').textContent, properties:[...document.querySelectorAll('#inventory-detail-body .item-property, #inventory-detail-body .item-modifier, #inventory-detail-body .inventory-bag-capacity')].map(node=>node.textContent), identified:!document.querySelector('#inventory-detail-body .item-identification'), charges:document.querySelector('#inventory-detail-body .inventory-charges')?.textContent};`);
  async function inspect(id, slot) { await openDetails(id, slot); const value = await detail(); await click('#inventory-detail-close'); return value; }
  async function identify(id) {
    for (let attempt = 0; attempt < 10; attempt++) {
      await select('e88.identify'); const before = await turn(); await click('#inventory-use');
      await driver.waitFor(`return !!document.querySelector('.item-target-dialog[open] select');`, "identify target");
      await driver.execute(`const select = document.querySelector('.item-target-dialog[open] select'); select.value = arguments[0]; if (select.value !== arguments[0]) throw new Error('Missing target'); select.dispatchEvent(new Event('change',{bubbles:true})); return true;`, [id]);
      await click('.item-target-dialog[open] button[type="submit"]'); await after(before);
      const value = await inspect(id); if (value.identified) return value;
    }
    throw new Error(`Identification never succeeded: ${id}`);
  }
  async function exportSave() {
    await driver.execute(`window.__e88Save = undefined; window.__e88Base64 = undefined; document.querySelector('.hud-menu').open = true; return true;`);
    await click('#save-button');
    await driver.waitFor(`return !!window.__e88Save;`, "save export");
    await driver.execute(`const reader = new FileReader(); reader.onload = () => { window.__e88Base64 = reader.result.split(',')[1]; }; reader.readAsDataURL(window.__e88Save); document.querySelector('.hud-menu').open = false; return true;`);
    await driver.waitFor(`return !!window.__e88Base64;`, "save bytes");
    return driver.execute(`return window.__e88Base64;`);
  }
  async function importSave(bytes, expectedHash) {
    const before = await hash();
    await driver.execute(`const transfer = new DataTransfer(); transfer.items.add(new File([Uint8Array.from(atob(arguments[0]), c=>c.charCodeAt(0))], 'e88.rfbsave')); const input = document.querySelector('#load-input'); input.files = transfer.files; input.dispatchEvent(new Event('change',{bubbles:true})); return true;`, [bytes]);
    await driver.waitFor(`const hash=document.querySelector('#hash-value').title; return (arguments[1] ? hash === arguments[1] : hash !== arguments[0]) && document.querySelector('#connection-status').classList.contains('ready');`, "bound save load", 60_000, [before, expectedHash ?? null]);
  }
  const uiState = () => driver.execute(`return {hash:document.querySelector('#hash-value').title, inventory:document.querySelector('#inventory-list').textContent, summary:document.querySelector('#inventory-count').textContent, equipment:document.querySelector('#equipment-list').textContent, vitals:document.querySelector('#character-vitals-list').textContent, traits:document.querySelector('[data-trait-section="trait-resistances"]').textContent};`);
  await driver.waitFor(`return document.documentElement.dataset.appMode === 'title';`, "title", 60_000);
  await click('#session-new-game');
  await click('#session-build-high-mage-death');
  await driver.execute(`document.querySelector('#session-seed').value='808'; document.querySelector('#session-character-name').value='E8.8验收'; return true;`);
  await click('#session-start-game');
  await driver.waitFor(`return document.documentElement.dataset.appMode === 'playing' && document.querySelector('#connection-status').classList.contains('ready');`, "new bound character", 60_000);
  await driver.execute(`window.__e88Errors=[]; window.addEventListener('error', event=>window.__e88Errors.push(event.message)); URL.createObjectURL=blob=>{window.__e88Save=blob;return 'blob:e88-save';}; URL.revokeObjectURL=()=>{}; HTMLAnchorElement.prototype.click=function(){}; return true;`);
  const input = path.join(directory, 'e88-new-game.rfbsave');
  await writeFile(input, Buffer.from(await exportSave(), 'base64'));
  const prepared = await promisify(execFile)('cargo', ['test','-p','rfb-core','export_ego_desktop_acceptance_save','--','--ignored','--nocapture'], {cwd:path.dirname(directory), env:{...process.env,E88_DESKTOP_INPUT:input}, timeout:180_000, windowsHide:true});
  await writeFile(path.join(directory, 'e88-preparation.log'), prepared.stdout + prepared.stderr);
  const expected = JSON.parse(await readFile(path.join(directory,'e88-expectations.json'),'utf8'));
  for (const entry of expected) {
    process.stdout.write(`E8.8 ${entry.case}: seed ${entry.seed}\n`);
    await importSave((await readFile(path.join(directory,`e88-${entry.case}.rfbsave`))).toString('base64'));
    const beforePickup = await turn(); await key('g','KeyG'); await after(beforePickup);
    await click('#player-ui-inventory-open');
    assert.equal((await inspect(entry.id)).identified, false, 'generated item starts unknown');
    const identified = await identify(entry.id);
    assert.ok(identified.properties.length > 0);
    if (entry.case === 'negative') assert.match(identified.text, /-\d/);
    if (entry.case === 'randart') assert.ok(identified.name.includes(entry.details.artifactName));
    if (entry.case === 'bag') assert.match(identified.text, new RegExp(`背包容量：\\s*${entry.details.bagCapacity}`));
    await select(entry.id); const beforeEquip = await turn(); await click('#inventory-equip');
    await driver.execute(`document.querySelector('.item-target-dialog[open] button[type="submit"]')?.click(); return true;`);
    await after(beforeEquip);
    await driver.waitFor(`return !document.querySelector(arguments[0]);`, "equipped", 10_000, [row(entry.id)]);
    const equipped = await inspect(entry.id, entry.details.slotId);
    assert.equal(equipped.name, identified.name);
    const vitals = await driver.execute(`return [...document.querySelectorAll('#character-vitals-list dd')].map(node=>Number(node.textContent));`);
    assert.equal(vitals.at(-2), entry.player.armorClass, 'actual character armor includes generated enchantments');
    if (entry.case === 'negative') {
      assert.ok(entry.player.speed < entry.plainSpeed);
      assert.equal(vitals.at(-1), entry.player.speed, 'negative speed affects the character');
      await openDetails(entry.id, entry.details.slotId); const before = await turn();
      await driver.execute(`const button=[...document.querySelectorAll('#inventory-detail-actions button')].find(node=>node.textContent.includes('卸下')); if(!button)throw new Error('Missing unequip'); button.click();return true;`);
      await driver.waitFor(`return document.querySelector('#message-list').textContent.includes('诅咒');`, 'curse blocks removal');
      await after(before);
      assert.equal(await driver.execute(`return !!document.querySelector(arguments[0]);`, [row(entry.id)]), false, 'cursed item remains equipped');
      checks.push({check:'negative-speed-and-cursed-removal',speed:entry.player.speed,withoutSpeedPenalty:entry.plainSpeed,penalty:entry.intrinsic.modifiers.speed});
      await driver.execute(`document.querySelector('#inventory-detail-dialog').open && document.querySelector('#inventory-detail-close').click(); return true;`);
    }
    if (entry.case === 'randart') {
      await openDetails(entry.id, entry.details.slotId); const before = await detail();
      for(let attempt=0;attempt<10;attempt++) {
        const previous=await turn(); await click(`[data-activation-item-id="${entry.id}"]`); await after(previous);
        if((await detail()).charges !== before.charges) break;
      }
      const activated=await detail(); assert.notEqual(activated.charges, before.charges, 'artifact activation spends energy');
      checks.push({check:'randart-activation',before:before.charges,after:activated.charges});
      await click('#inventory-detail-close');
    }
    if (entry.case === 'dragon') {
      assert.ok(entry.player.armorClass > entry.plainArmor, 'Ego armor bonus contributes');
      for(const element of Object.keys(entry.intrinsic.resistances)) {
        const projected=entry.player.traitDetails.resistances.find(res=>res.damageType===element);
        assert.ok(projected?.reductionPercent > 0);
        const text=await driver.execute(`return document.querySelector(arguments[0]).textContent;`, [`[data-trait="resistance-${element}"]`]);
        assert.ok(text.includes(`${projected.reductionPercent}%`), `dragon base ${element} contributes to player resistance`);
      }
      checks.push({check:'dragon-base-resistance-and-ego-armor',baseResistances:entry.intrinsic.resistances,armor:entry.player.armorClass,withoutEnchantment:entry.plainArmor});
    }
    if (entry.case === 'bag') {
      await click('#player-page-close');
      // 24 initial cargo stacks plus ten pickups fill the generated 34-slot bag.
      for(let pickup=0;pickup<10;pickup++) { const before=await turn(); await key('g','KeyG'); await after(before); }
      await click('#player-ui-inventory-open');
      const filled=await uiState(); assert.match(filled.summary,/背包槽位 34 \/ 34/);
      assert.equal(await driver.execute(`return document.querySelectorAll('#inventory-list [data-item-id]').length;`),34);
      const weight=await driver.execute(`return [...document.querySelectorAll('#inventory-list .inventory-item-weight')].reduce((total,node)=>total+Number(node.textContent.match(/[0-9.]+/)[0]),0);`);
      assert.ok(filled.summary.includes(`${(weight+entry.details.weightTenthsPound/10).toFixed(1)} /`), 'actual carried weight includes cargo and bag');
      await writeFile(path.join(directory,'e88-bag-filled.png'),await driver.screenshot(),'base64');
      await openDetails(entry.id,entry.details.slotId); const beforeRemoval=await turn();
      await driver.execute(`[...document.querySelectorAll('#inventory-detail-actions button')].find(node=>node.textContent.includes('卸下')).click(); return true;`);
      await after(beforeRemoval); assert.equal((await uiState()).equipment,filled.equipment,'full bag cannot be removed');
      await driver.execute(`document.querySelector('#inventory-detail-dialog').open && document.querySelector('#inventory-detail-close').click();return true;`);
      await click('#player-page-close'); const beforeOverflow=await turn(); await key('g','KeyG');
      await driver.waitFor(`return document.querySelector('#message-list').textContent.includes('背包');`, 'overflow rejected');
      await after(beforeOverflow); assert.equal((await uiState()).inventory,filled.inventory,'overflow pickup does not lose cargo');
      await click('#player-ui-inventory-open');
      checks.push({check:'bag-extra-slots-weight-and-overflow',bagCapacity:entry.details.bagCapacity,filled:filled.summary,cargoWeightPounds:weight,bagWeightPounds:entry.details.weightTenthsPound/10});
    }
    await openDetails(entry.id,entry.details.slotId); const savedDetails = await detail();
    await writeFile(path.join(directory,`e88-${entry.case}.png`),await driver.screenshot(),'base64');
    await click('#inventory-detail-close');
    await click('#player-page-close');
    const saved=await uiState(); const bytes=await exportSave();
    await writeFile(path.join(directory,`e88-${entry.case}-played.rfbsave`),Buffer.from(bytes,'base64'));
    const previous=await turn(); await key('5','Numpad5'); await after(previous);
    assert.notEqual(await hash(),saved.hash); await importSave(bytes,saved.hash);
    assert.deepEqual(await uiState(),saved,`${entry.case} exact save restoration`);
    await click('#player-ui-inventory-open'); assert.deepEqual(await inspect(entry.id,entry.details.slotId),savedDetails); await click('#player-page-close');
    const continued=await turn(); await key('5','Numpad5'); await after(continued);
    checks.push({check:'pickup-identify-equip-save-restore-continue',case:entry.case,seed:entry.seed,identified,equipped,saved});
    await writeFile(path.join(directory,'ego-desktop-report.json'),JSON.stringify({checks},null,2)+'\n');
  }
  assert.deepEqual(await driver.execute(`return window.__e88Errors;`),[]);
}
