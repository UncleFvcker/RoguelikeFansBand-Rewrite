// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { selectCreationBuild, selectCreationRace } from "./character-creation.e2e.mjs";

export async function runCraftScenario(driver, directory) {
  await mkdir(directory, { recursive: true });
  const click = selector => driver.execute('document.querySelector(arguments[0]).click(); return true;', [selector]);
  const ready = () => driver.waitFor('return document.querySelector("#connection-status").classList.contains("ready")', "Craft ready");
  async function invoke(command, args = {}) {
    await driver.execute(`window.__craftReply = undefined; window.__craftError = null;
      window.__TAURI_INTERNALS__.invoke(arguments[0], arguments[1]).then(result => window.__craftReply = result, error => window.__craftError = String(error)); return true;`, [command, args]);
    await driver.waitFor('return window.__craftReply !== undefined || window.__craftError', command);
    assert.equal(await driver.execute('return window.__craftError'), null);
    return driver.execute('return window.__craftReply');
  }
  async function reloadSave() {
    const snapshot = await invoke("inspect_game_e2e");
    await invoke("save_game", { savedAt: "2026-09-11T00:00:00Z" });
    await driver.execute(`const files = new DataTransfer(); files.items.add(new File([new Uint8Array(window.__craftReply)], "craft.rfbsave"));
      const input = document.querySelector("#load-input"); input.files = files.files; input.dispatchEvent(new Event("change", { bubbles: true })); return true;`);
    await ready();
    await driver.waitFor('return document.querySelector("#hash-value").title === arguments[0]', "loaded Craft hash", 15_000, [snapshot.stateHash]);
    assert.equal((await invoke("inspect_game_e2e")).stateHash, snapshot.stateHash);
    return snapshot;
  }
  await driver.waitFor('return document.documentElement.dataset.appMode === "title"', "Craft title", 60_000);
  await click("#session-settings");
  await driver.execute('const input=document.querySelector("#session-settings-language");input.value="zh-CN";input.dispatchEvent(new Event("change", {bubbles:true}));return true;');
  await click("#session-settings-back");
  await click("#session-new-game");
  await selectCreationRace(driver, "rfb-legacy.race.dwarf");
  await selectCreationBuild(driver, "demo.build.high-mage-craft");
  await driver.execute('const input=document.querySelector("#session-seed");input.value="415";input.dispatchEvent(new Event("input", {bubbles:true}));return true;');
  await driver.execute('const input=document.querySelector("#session-character-name");input.value="工艺验收";input.dispatchEvent(new Event("input", {bubbles:true}));return true;');
  await writeFile(path.join(directory, "craft-creation.png"), await driver.screenshot(), "base64");
  await click("#session-start-game");
  await driver.waitFor('return document.documentElement.dataset.appMode === "playing"', "Craft new game", 60_000); await ready();
  const born = await invoke("inspect_game_e2e");
  assert.equal(born.player.build.buildId, "demo.build.high-mage-craft");
  assert.equal(born.player.abilities.filter(a => a.source === "learned").length, 32);
  await invoke("prepare_craft_e2e");
  const prepared = await reloadSave();
  const openAbilities = async () => {
    await click(await driver.execute('return document.querySelector("#player-page-dialog").open') ? "#player-page-tab-ability" : "#player-ui-ability-open");
    await driver.waitFor('return document.querySelector("#ability-list").checkVisibility()', "Craft abilities");
  };
  await openAbilities();
  const row = slug => `[data-ability-id="demo.ability.craft-${slug}"]`;
  const choices = await driver.execute('return [...document.querySelector(arguments[0]).options].map(o => ({value:o.value,text:o.textContent}));', [`${row("elemental-brand")} .ability-element-target`]);
  assert.deepEqual(choices.map(o => o.value), ["fire", "cold", "poison", "acid", "electricity"]);
  assert.ok(choices.every(o => o.text && !o.text.includes("[")));
  await driver.execute('document.querySelector(arguments[0]).value="cold";return true;', [`${row("elemental-brand")} .ability-element-target`]);
  await click(`${row("elemental-brand")} .ability-cast-action`); await ready();
  let snapshot = await invoke("inspect_game_e2e");
  assert.ok(snapshot.player.statuses.some(s => s.kindId === "rfb.status.elemental-brand"));
  const beforeCancel = snapshot.stateHash;
  await openAbilities(); await click(`${row("enchantment")} .ability-cast-action`);
  await driver.waitFor('return document.querySelector(".item-target-dialog")?.open', "enchantment targets");
  await click('.item-target-dialog button[type="button"]');
  assert.equal((await invoke("inspect_game_e2e")).stateHash, beforeCancel);
  await click(`${row("enchantment")} .ability-cast-action`);
  await driver.execute('const dialog=document.querySelector(".item-target-dialog");dialog.querySelector("select").value="e2e.craft.dagger";dialog.querySelector("form").requestSubmit();return true;');
  await driver.waitFor('return document.querySelector("#hash-value").title !== arguments[0]', "weapon enchantment completed", 10_000, [beforeCancel]);
  await ready(); snapshot = await invoke("inspect_game_e2e");
  const dagger = snapshot.inventory.find(i => i.id === "e2e.craft.dagger");
  assert.equal(dagger.enchantments.toHit, 3); assert.equal(dagger.enchantments.toDamage, 3);
  await openAbilities(); await click(`${row("magic-armor")} .ability-cast-action`); await ready();
  snapshot = await reloadSave();
  assert.ok(snapshot.player.statuses.some(s => s.kindId === "rfb.status.magic-armor"));
  await openAbilities();
  await writeFile(path.join(directory, "craft-abilities.png"), await driver.screenshot(), "base64");
  await writeFile(path.join(directory, "craft-report.json"), JSON.stringify({ buildId: born.player.build.buildId, birthLevel: born.player.progress.level,
    fixture: "Seed 415; level 50; Intelligence raised to its existing potential, capped at 18/100; three advanced books and an identified dagger; four spells learned at Master proficiency. Loaded through the normal save UI before casting.",
    preparedHash: prepared.stateHash, finalHash: snapshot.stateHash, choices, cancelledItemSelectionPreservedHash: true,
    checks: ["normal creation entry", "32 realm spells", "five localized elemental choices", "cold brand cast", "cancelled item target", "+3 weapon enchantment", "magic armor cast", "native save and UI load"] }, null, 2));
}
