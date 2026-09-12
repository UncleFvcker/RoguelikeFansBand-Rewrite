// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { Localization } from "../src/localization.ts";
import { WARRIOR_MAGE_SECOND_REALMS } from "../src/character-creation.ts";
import { selectCreationBuild, selectCreationRace } from "./character-creation.e2e.mjs";
import { connectKeyboard } from "./character-creation-layout.e2e.mjs";
import { nextWalk } from "./berserker.e2e.mjs";
import { prepareDungeonEntry } from "./dungeon-entry.e2e.mjs";

// UI acceptance code; execution is deferred until Warrior-Mage step 7.
export async function runWarriorMageUiScenario(driver, directory, profile, playthrough = false) {
  await mkdir(directory, { recursive: true });
  await driver.waitFor('return document.documentElement.dataset.appMode === "title"', "Warrior-Mage title", 60_000);
  const keyboard = await connectKeyboard(profile);
  const sources = Object.fromEntries(await Promise.all(["zh-CN", "en-US"].map(async locale => [locale,
    await Promise.all(["ui", "content", "game"].map(file => readFile(new URL(`../../locales/${locale}/${file}.ftl`, import.meta.url), "utf8"))),
  ])));
  const localization = new Localization("zh-CN", sources);
  const report = { mode: playthrough ? "play" : "ui", preparation: "Arcane/Sorcery normal human birth. UI: all eight menus in both locales, explicit XP to 24/25. Play: normal dungeon melee and both realms, then explicit XP to 25/50 and current-realm fourth books. Preparation clears monsters, lights the map, refills HP/MP, adds Life first book and at 25/50 a Sheep on a 3x3 floor. Natural spell and conversion failures retained. No natural high-level progression or prepared-item acquisition claim.", locales: [] };
  const click = selector => driver.execute('document.querySelector(arguments[0]).click(); return true;', [selector]);
  const focus = selector => driver.execute('document.querySelector(arguments[0]).focus(); return true;', [selector]);
  const text = selector => driver.execute('return document.querySelector(arguments[0]).textContent;', [selector]);
  const focusIs = selector => driver.execute('return document.activeElement.matches(arguments[0]);', [selector]);
  const ready = () => driver.waitFor('return document.querySelector("#connection-status").classList.contains("ready")', "idle UI");
  const row = id => `[data-ability-id="${id}"]`;
  const studyButton = id => `${row(id)} [data-ability-action="study"]`;
  const realmButton = id => `#realm-change-books [data-realm-id="${id}"]`;
  async function invoke(command, args) {
    await driver.execute(`window.__wmResult = null; window.__wmDone = false; window.__wmError = null;
      window.__TAURI_INTERNALS__.invoke(arguments[0], arguments[1]).then(value => { window.__wmResult = value; window.__wmDone = true; }, error => window.__wmError = String(error)); return true;`, [command, args]);
    await driver.waitFor('return window.__wmDone || window.__wmError', command, 30_000);
    assert.equal(await driver.execute('return window.__wmError'), null);
    return driver.execute('return window.__wmResult');
  }
  const snapshot = () => invoke("inspect_game_e2e");
  async function save() {
    await invoke("save_game", { savedAt: "2026-09-12T12:00:00Z" });
    return driver.execute('window.__wmSaves ??= []; return window.__wmSaves.push(window.__wmResult) - 1;');
  }
  async function changed(before, label) {
    await driver.waitFor('return document.querySelector("#hash-value").title !== arguments[0]', label, 15_000, [before]);
    await ready();
    return snapshot();
  }
  async function viewport(width, height, zoom = 1) {
    await invoke("plugin:webview|set_webview_zoom", { label: "main", value: zoom });
    const rect = await driver.command("GET", "/window/rect");
    const current = await driver.execute('return { width: innerWidth, height: innerHeight }');
    await driver.command("POST", "/window/rect", { width: Math.round(rect.width + (width - current.width) * zoom), height: Math.round(rect.height + (height - current.height) * zoom) });
    await driver.waitFor('return Math.abs(innerWidth - arguments[0]) <= 1 && Math.abs(innerHeight - arguments[1]) <= 1', `${width}x${height} at ${zoom}`, 10_000, [width, height]);
  }
  async function screenshot(name) {
    await driver.execute('window.__wmPainted = false; requestAnimationFrame(() => requestAnimationFrame(() => window.__wmPainted = true)); return true;');
    await driver.waitFor('return window.__wmPainted', "paint");
    await writeFile(path.join(directory, `${name}.png`), await driver.screenshot(), "base64");
  }
  async function checkLayout(selector) {
    const result = await driver.execute(`const node = document.querySelector(arguments[0]); const r = node.getBoundingClientRect();
      return { visible: node.checkVisibility(), left:r.left, right:r.right, top:r.top, bottom:r.bottom, width:innerWidth, height:innerHeight, overflow:node.scrollWidth > node.clientWidth + 1 };`, [selector]);
    assert.equal(result.visible, true, selector);
    assert.equal(result.overflow, false, `${selector} must not overflow horizontally`);
    assert.ok(result.left >= -1 && result.right <= result.width + 1 && result.top >= -1 && result.bottom <= result.height + 1, `${selector}: ${JSON.stringify(result)}`);
  }
  async function hold(command) {
    await driver.execute(`const original = window.fetch, commandUrl = window.__TAURI_INTERNALS__.convertFileSrc(arguments[0], "ipc"); window.__wmDispatches = 0; window.__wmRelease = null;
      window.fetch = (url, options) => url !== commandUrl ? original(url, options) : new Promise((resolve, reject) => {
        window.__wmDispatches++; window.__wmRelease = () => { window.fetch = original; original(url, options).then(resolve, reject); };
      }); return true;`, [command]);
  }
  async function release() {
    assert.equal(await driver.execute('return window.__wmDispatches'), 1, "one IPC request while busy");
    await driver.execute('window.__wmRelease(); return true;');
  }
  async function load(bytes, expectedHash) {
    await driver.execute(`window.__wmLoaded = false; const original = window.fetch, commandUrl = window.__TAURI_INTERNALS__.convertFileSrc("load_game", "ipc");
      window.fetch = (url, options) => {
        if (url !== commandUrl) return original(url, options);
        window.fetch = original;
        return original(url, options).then(value => { setTimeout(() => window.__wmLoaded = true, 0); return value; });
      };
      const transfer = new DataTransfer(); transfer.items.add(new File([new Uint8Array(window.__wmSaves[arguments[0]])], "warrior-mage-ui.rfbsave"));
      const input = document.querySelector("#load-input"); input.files = transfer.files; input.dispatchEvent(new Event("change", { bubbles:true })); return true;`, [bytes]);
    await driver.waitFor('return window.__wmLoaded && document.querySelector("#hash-value").title === arguments[0]', "save loaded through UI", 30_000, [expectedHash]);
    await ready();
    return snapshot();
  }
  async function abilitiesPage() {
    await click(await driver.execute('return document.querySelector("#player-page-dialog").open') ? "#player-page-tab-ability" : "#player-ui-ability-open");
    await driver.waitFor('return document.querySelector("#ability-list").checkVisibility()', "spellbook page");
  }
  async function chooseTalent() {
    let current = await snapshot();
    if (current.player.pendingRaceMutationChoice) {
      await click("#player-ui-character-open"); await click("#character-tab-other");
      await focus(".mutation-choice-candidate"); await keyboard.key("Enter");
      current = await changed(current.stateHash, "source human talent");
      await keyboard.key("Escape");
    }
    return current;
  }
  async function prepare(level) {
    const prepared = await invoke("prepare_spell_learning_e2e", { level });
    await load(await save(), prepared.stateHash);
    await chooseTalent();
    await abilitiesPage();
    return snapshot();
  }
  async function study(id, key = "Enter") {
    await abilitiesPage();
    const before = await snapshot();
    assert.equal(before.player.abilities.find(ability => ability.id === id)?.canStudy, true);
    await focus(studyButton(id)); await keyboard.key(key);
    const after = await changed(before.stateHash, `study ${id}`);
    assert.equal(await focusIs(after.player.abilities.find(a => a.id === id).canStudy ? studyButton(id) : row(id)), true, "study focus survives rerender or moves to its row when disabled");
    assert.equal(after.player.abilityLearning.remainingSlots, before.player.abilityLearning.remainingSlots - 1);
    return after;
  }
  async function closePage() {
    if (await driver.execute('return document.querySelector("#player-page-dialog").open')) await keyboard.key("Escape");
  }
  const hpToMp = "demo.ability.warrior-mage-hp-to-sp", mpToHp = "demo.ability.warrior-mage-sp-to-hp";
  async function castAbility(id, target) {
    await abilitiesPage();
    const before = await snapshot();
    const ability = before.player.abilities.find(a => a.id === id);
    assert.equal(ability?.canCast, true, `${id}: ${ability?.unavailableReason}; resources=${JSON.stringify(before.player.resources)}`);
    await focus(`${row(id)} .ability-cast-action`); await keyboard.key("Enter");
    if (target) {
      await driver.waitFor('return document.querySelector("#map-host").dataset.targeting === "true"', "spell target");
      let dx = target.x - before.player.position.x, dy = target.y - before.player.position.y;
      while (dx || dy) {
        const sx = Math.sign(dx), sy = Math.sign(dy);
        await keyboard.key(({ "-1,-1":"7", "0,-1":"8", "1,-1":"9", "-1,0":"4", "1,0":"6", "-1,1":"1", "0,1":"2", "1,1":"3" })[`${sx},${sy}`]);
        dx -= sx; dy -= sy;
      }
      await keyboard.key("Enter");
    }
    const after = await changed(before.stateHash, id);
    const events = await driver.execute('return window.__wmUpdate.events');
    assert.equal(events.find(e => e.outcome?.type === "ability-cast").outcome.resolution.abilityId, id);
    assert.equal(after.player.isDead, false);
    return { after, events };
  }
  async function actKey(key) {
    const before = await snapshot(); await keyboard.key(key); return changed(before.stateHash, `native key ${key}`);
  }
  async function exportPlayedSave(name) {
    await closePage();
    await driver.execute(`window.__wmExport=null;window.__wmDownloadHooks=[URL.createObjectURL,URL.revokeObjectURL,HTMLAnchorElement.prototype.click];
      URL.createObjectURL=blob=>{window.__wmExport={blob};return "blob:warrior-mage-acceptance";};URL.revokeObjectURL=()=>{};
      HTMLAnchorElement.prototype.click=function(){window.__wmExport.name=this.download;};document.querySelector('.hud-menu').open=true;return true;`);
    await click("#save-button"); await driver.waitFor('return window.__wmExport?.name?.endsWith(".rfbsave")', "menu export");
    await driver.execute(`document.querySelector('.hud-menu').open=false;
      [URL.createObjectURL,URL.revokeObjectURL,HTMLAnchorElement.prototype.click]=window.__wmDownloadHooks;
      window.__wmExportIndex=null;window.__wmExport.blob.arrayBuffer().then(buffer=>{
        window.__wmSaves??=[];window.__wmExportIndex=window.__wmSaves.push(Array.from(new Uint8Array(buffer)))-1;});return true;`);
    await driver.waitFor('return window.__wmExportIndex!==null', "save bytes");
    const index = await driver.execute('return window.__wmExportIndex');
    await writeFile(path.join(directory, `${name}.rfbsave`), Buffer.from(await driver.execute('return window.__wmSaves[arguments[0]]', [index])));
    return index;
  }
  async function playNewGame(born) {
    const checks = [], primary = "demo.ability.arcane-zap", secondary = "demo.ability.sorcery-detect-monsters";
    let current = await study(primary);
    checks.push({ birth: { level: born.player.progress.level, hash: born.stateHash, learned: primary, inventory: born.inventory, equipment: born.equipment } });
    await closePage(); await click("#player-ui-inventory-open");
    const torch = current.inventory.find(item => item.kindId === "demo.item.wooden-torch"); assert.ok(torch);
    await click(`[data-item-id="${torch.id}"] input[type="checkbox"]`); await click("#inventory-equip");
    current = await changed(current.stateHash, "birth torch"); await closePage();
    if (process.argv.includes("--fast-entry")) { checks.push({ fastEntry: await prepareDungeonEntry(driver) }); current = await snapshot(); }
    else for (let step = 0; step < 120; step++) {
      const entry = current.cells.find(cell => cell.terrainId === "demo.terrain.stairs-down").position;
      if (current.player.position.x === entry.x && current.player.position.y === entry.y) break;
      current = await actKey(nextWalk(current, new Set(), entry));
    }
    await click("#traverse-stairs"); current = await changed(current.stateHash, "normal generated dungeon");
    assert.equal(current.floorId, "demo.floor.warrens-depth-1");
    const visited = new Set(); let meleeHit, spellHit;
    for (let step = 0; step < 220; step++) {
      assert.equal(current.player.isDead, false);
      const second = current.player.abilities.find(a => a.id === secondary);
      if (!second.learned && second.canStudy) { await abilitiesPage(); current = await study(secondary); await closePage(); }
      if (!current.player.abilities.find(a => a.id === secondary).castCount && current.player.abilities.find(a => a.id === secondary).canCast) {
        current = (await castAbility(secondary)).after; await closePage();
      }
      if (meleeHit && spellHit && current.player.abilities.find(a => a.id === secondary).castCount > 0) break;
      visited.add(`${current.player.position.x},${current.player.position.y}`);
      const distance = p => Math.max(Math.abs(p.x-current.player.position.x), Math.abs(p.y-current.player.position.y));
      const target = current.entities.filter(e => e.faction === "hostile").sort((a,b) => distance(a.position)-distance(b.position))[0];
      const before = current;
      if (target && !spellHit && distance(target.position) <= 6 && current.player.abilities.find(a => a.id === primary).canCast) {
        current = (await castAbility(primary, target.position)).after; await closePage();
        const hpAfter = current.entities.find(e => e.id === target.id)?.hp ?? 0;
        if (hpAfter < target.hp) spellHit = { target: target.kindId, hpBefore: target.hp, hpAfter, before: before.stateHash, after: current.stateHash };
      } else {
        current = await actKey(nextWalk(current, visited, target?.position));
        if (target && Math.max(Math.abs(target.position.x-before.player.position.x), Math.abs(target.position.y-before.player.position.y)) === 1) {
          const hpAfter = current.entities.find(e => e.id === target.id)?.hp ?? 0;
          if (hpAfter < target.hp) meleeHit = { target: target.kindId, hpBefore: target.hp, hpAfter, before: before.stateHash, after: current.stateHash };
        }
      }
    }
    assert.ok(meleeHit && spellHit, "birth-equipment melee and Zap must damage natural monsters");
    assert.ok(current.player.abilities.find(a => a.id === secondary).castCount > 0, "natural second-realm study and cast");
    checks.push({ naturalDungeon: { level: current.player.progress.level, meleeHit, spellHit, secondRealmCasts: current.player.abilities.find(a => a.id === secondary).castCount, hash: current.stateHash } });
    await screenshot("natural-dungeon"); await exportPlayedSave("natural-dungeon-birth-equipment");
    process.stdout.write("Warrior-Mage: normal birth, generated dungeon, melee and both realms passed.\n");
    current = await prepare(25);
    for (let i = 0; i < 4; i++) current = (await castAbility(secondary)).after;
    async function succeed(id, targetId) {
      const attempts = [];
      for (let attempt = 0; attempt < 24; attempt++) {
        const target = targetId ? current.entities.find(e => e.id === targetId) : undefined;
        if (targetId) assert.ok(target, `live spell target ${targetId}`);
        const result = await castAbility(id, target?.position); current = result.after;
        attempts.push({ events: result.events, hash: current.stateHash });
        if (result.events.some(e => e.outcome?.type === "ability-cast" && e.outcome.resolution.succeeded)) return attempts;
      }
      assert.fail(`No natural success: ${id}`);
    }
    const converted = await succeed(hpToMp);
    const conversion = converted.at(-1).events.find(e => e.outcome?.type === "resource-conversion").outcome.resolution;
    assert.ok(conversion.resourceAfter > conversion.resourceBefore); assert.ok(conversion.hpAfter < conversion.hpBefore);
    const sheep = current.entities.find(e => e.id === "e2e.warrior-mage-power-target"); assert.ok(sheep);
    const resumedSpell = await succeed(primary, sheep.id);
    assert.ok((current.entities.find(e => e.id === sheep.id)?.hp ?? 0) < sheep.hp);
    const healing = await succeed(mpToHp);
    checks.push({ preparedLevel: 25, conversion: converted, continuedSpell: resumedSpell, healing });
    await screenshot("level25-conversion-and-combat");
    current = await prepare(50);
    const high = ["demo.ability.arcane-see-invisible", "demo.ability.sorcery-probe"];
    const highCasts = [];
    for (const id of high) {
      const ability = current.player.abilities.find(a => a.id === id);
      assert.equal(ability.bookRank, 4); assert.equal(ability.canStudy, true);
      current = await study(id); highCasts.push({ id, attempts: await succeed(id) });
    }
    checks.push({ preparedLevel: 50, highBooks: highCasts });
    await abilitiesPage();
    await focus(realmButton("life")); await keyboard.key("Enter"); current = await changed(current.stateHash, "played change request");
    await focus("#realm-change-accept"); await keyboard.key("Enter"); current = await changed(current.stateHash, "played change confirmed");
    assert.equal(current.player.abilityLearning.realms.secondRealmId, "life");
    assert.deepEqual(current.player.abilityLearning.realms.previousRealmIds, ["sorcery"]);
    const heal = "demo.ability.life-cure-light-wounds"; current = await study(heal);
    const detect = "demo.ability.arcane-detect-monsters"; current = await study(detect);
    const saved = current, index = await exportPlayedSave("level50-changed-realm-test-prepared");
    async function continuePlayedSave() {
      const actions = [];
      for (const id of [hpToMp, detect, heal, mpToHp]) {
        const result = await castAbility(id); actions.push({ id, events: result.events, hash: result.after.stateHash });
      }
      return { actions, state: await snapshot() };
    }
    const continued = await continuePlayedSave(); await closePage();
    const restored = await load(index, saved.stateHash);
    assert.deepEqual(restored.player, saved.player); assert.deepEqual(restored.inventory, saved.inventory); assert.deepEqual(restored.equipment, saved.equipment);
    const replayed = await continuePlayedSave(); assert.deepEqual(replayed, continued);
    checks.push({ saveContinuation: { savedHash: saved.stateHash, actions: continued.actions, continuedHash: continued.state.stateHash, replayedHash: replayed.state.stateHash } });
    await screenshot("saved-conversion-spells-continued");
    report.locales.push({ locale: "zh-CN", build: born.player.build.buildId, checks });
    process.stdout.write("Warrior-Mage: prepared conversion/combat, high books, changed-realm export and identical saved continuation passed.\n");
  }
  try {
    await invoke("plugin:window|set_min_size", { label: "main", value: null });
    for (const locale of playthrough ? ["zh-CN"] : ["zh-CN", "en-US"]) {
      localization.setLocale(locale);
      await driver.execute('localStorage.setItem("rfb.locale",arguments[0]);localStorage.setItem("rfb.input-preset","numpad");return true;', [locale]);
      await keyboard.reload();
      await driver.waitFor('return document.documentElement.dataset.appMode==="title"&&!document.querySelector("#session-new-game").disabled', "localized title", 60_000);
      await driver.execute(`const original=window.fetch,endpoint=window.__TAURI_INTERNALS__.convertFileSrc("dispatch_game_command","ipc");
        window.fetch=(url,options)=>url!==endpoint?original(url,options):original(url,options).then(async response=>{window.__wmUpdate=await response.clone().json();return response;});return true;`);
      await viewport(1280, 720); await click("#session-new-game");
      await driver.execute('document.querySelector("#session-character-name").value="Warrior-Mage UI";document.querySelector("#session-seed").value="925";return true;');
      await selectCreationRace(driver, "demo.race.rfb-human");
      const builds = WARRIOR_MAGE_SECOND_REALMS.map(second => `demo.build.warrior-mage-arcane-${second}`);
      if (!playthrough) {
        await selectCreationBuild(driver, "demo.build.warrior");
        const confirmed = await text("#session-creation-summary");
        await click('[data-career-group="hybrid"]'); await focus('[data-career-id="warrior-mage"]'); await keyboard.key("Enter");
        assert.equal(await driver.execute('return document.querySelector("#session-start-game").disabled'), true);
        await driver.execute('document.querySelector("#session-new-game-view").requestSubmit();return true;');
        assert.equal(await driver.execute('return document.documentElement.dataset.appMode'), "new-game");
        await keyboard.key("Escape"); assert.equal(await focusIs('[data-career-id="warrior-mage"]'), true);
        assert.equal(await text("#session-creation-summary"), confirmed);
        for (const id of builds) {
          await selectCreationBuild(driver, id);
          assert.deepEqual(await driver.execute('return [...document.querySelectorAll("#session-career-options [data-career-id]")].map(n=>n.dataset.careerId)'), builds);
          assert.equal(await driver.execute('return document.querySelector(arguments[0]).getAttribute("aria-pressed")', [`[data-career-id="${id}"]`]), "true");
          assert.ok((await text("#session-career-notes")).includes(localization.format("session-warrior-mage-realms-help")));
        }
        for (const [width, height, zoom] of [[390, 844, 1], [640, 360, 2]]) {
          await viewport(width, height, zoom);
          await focus('[data-career-id="demo.build.warrior-mage-arcane-life"]'); await keyboard.key("End");
          assert.equal(await focusIs('[data-career-id="demo.build.warrior-mage-arcane-armageddon"]'), true);
          await keyboard.key("Home"); await keyboard.key("ArrowDown"); await keyboard.key(" ");
          assert.equal(await focusIs('[data-career-id="demo.build.warrior-mage-arcane-sorcery"]'), true);
          await checkLayout("#session-new-game-view"); await checkLayout("#session-start-game");
          await screenshot(`${locale}-creation-${width}-${zoom}`);
        }
        await viewport(1280, 720);
      }
      await selectCreationBuild(driver, "demo.build.warrior-mage-arcane-sorcery");
      await hold("initialize_game"); await focus("#session-start-game"); await keyboard.key("Enter");
      await driver.waitFor('return !!window.__wmRelease', "held creation");
      assert.equal(await driver.execute('return [...document.querySelectorAll("#session-new-game-view button,#session-new-game-view input")].every(n=>n.matches(":disabled"))'), true);
      await keyboard.key("Enter"); await keyboard.key("Escape"); await release();
      await driver.waitFor('return document.documentElement.dataset.appMode==="playing"&&document.querySelector("#connection-status").classList.contains("ready")', "normal Warrior-Mage birth", 60_000);
      const born = await chooseTalent(); await abilitiesPage();
      assert.equal(born.player.progress.level, 1);
      assert.equal(born.player.build.buildId, "demo.build.warrior-mage-arcane-sorcery");
      assert.equal(born.player.abilityLearning.studyMode, "chosen");
      assert.equal(born.player.abilityLearning.realms.firstRealmId, "arcane");
      assert.equal(await text("#spell-realms-study-help"), localization.format("ability-mage-study-help"));
      for (const id of [hpToMp, mpToHp]) assert.equal(born.player.abilities.find(a => a.id === id).unavailableReason, "level-too-low");
      await screenshot(`${locale}-birth`);
      if (playthrough) { await playNewGame(born); continue; }
      let current = await prepare(24);
      for (const id of [hpToMp, mpToHp]) assert.equal(current.player.abilities.find(a => a.id === id).unavailableReason, "level-too-low");
      const primary = current.player.abilities.find(a => a.bookRealmId === "arcane" && a.canStudy).id;
      const secondary = current.player.abilities.find(a => a.bookRealmId === "sorcery" && a.canStudy).id;
      current = await study(primary); current = await study(secondary, " ");
      assert.equal(current.player.abilities.find(a => a.id === primary).proficiencyCap, 1600);
      assert.equal(current.player.abilities.find(a => a.id === secondary).proficiencyCap, 1400);
      assert.equal(await text(studyButton(secondary)), localization.format("action-ability-restudy"));
      const beforeRepeat = current;
      await hold("dispatch_game_command"); await focus(studyButton(secondary)); await keyboard.key("Enter");
      await driver.waitFor('return !!window.__wmRelease', "held restudy");
      assert.equal(await driver.execute('return [...document.querySelectorAll("#ability-list button")].every(n=>n.matches(":disabled"))'), true);
      await keyboard.key("Enter"); await release(); current = await changed(beforeRepeat.stateHash, "one restudy");
      assert.equal(current.player.abilityLearning.remainingSlots, beforeRepeat.player.abilityLearning.remainingSlots - 1);
      assert.ok(current.player.abilities.find(a => a.id === secondary).proficiency > beforeRepeat.player.abilities.find(a => a.id === secondary).proficiency);
      current = await prepare(25);
      for (const id of [hpToMp, mpToHp]) {
        const ability = current.player.abilities.find(a => a.id === id);
        assert.equal(ability.canCast, true); assert.equal(ability.governingAttribute, "intelligence");
        const effect = ability.effects.find(e => e.type === (id === hpToMp ? "health-to-mana" : "mana-to-health"));
        const summary = id === hpToMp
          ? localization.format("ability-health-to-mana-summary", { cost: effect.hitPointCost, divisor: effect.manaDivisor })
          : localization.format("ability-mana-to-health-summary", { cost: effect.manaCost, healing: effect.healing });
        assert.ok((await text(row(id))).includes(summary));
        assert.ok((await text(row(id))).includes(localization.format("ability-resource-conversion-cost-help")));
      }
      const attempts = [];
      for (const id of [hpToMp, mpToHp]) {
        let succeeded = false;
        for (let attempt = 0; attempt < 24; attempt++) {
          const result = await castAbility(id); current = result.after;
          attempts.push({ id, events: result.events, hash: current.stateHash });
          const converted = result.events.find(e => e.outcome?.type === "resource-conversion");
          if (converted) { assert.equal(converted.outcome.resolution.converted, true); succeeded = true; break; }
        }
        assert.equal(succeeded, true, `natural success for ${id}`);
      }
      for (const [width, height, zoom] of [[390, 844, 1], [640, 360, 2]]) {
        await abilitiesPage();
        await viewport(width, height, zoom); await focus("#spell-realms-value");
        await checkLayout("#player-page-dialog"); await checkLayout("#ability-panel");
        await screenshot(`${locale}-abilities-${width}-${zoom}`);
      }
      const beforeChange = current;
      await focus(realmButton("life")); await keyboard.key("Enter");
      current = await changed(beforeChange.stateHash, "realm change requested");
      assert.equal(await focusIs("#realm-change-decline"), true);
      const pending = current.stateHash, pendingSave = await save();
      await checkLayout("#realm-change-dialog"); await screenshot(`${locale}-change-200-percent`);
      await viewport(390, 844); await checkLayout("#realm-change-dialog"); await screenshot(`${locale}-change-390`);
      await keyboard.key("Escape"); current = await changed(pending, "cancel change");
      assert.equal(current.player.abilityLearning.realms.secondRealmId, "sorcery");
      assert.equal(current.player.abilityLearning.remainingSlots, beforeChange.player.abilityLearning.remainingSlots);
      await load(pendingSave, pending); assert.equal(await focusIs("#realm-change-decline"), true);
      await hold("dispatch_game_command"); await keyboard.key("Tab");
      assert.equal(await focusIs("#realm-change-accept"), true); await keyboard.key("Enter");
      await driver.waitFor('return !!window.__wmRelease', "held confirmation");
      assert.equal(await driver.execute('return [...document.querySelectorAll("#realm-change-dialog button")].every(n=>n.matches(":disabled"))'), true);
      await keyboard.key("Escape"); assert.equal(await driver.execute('return document.querySelector("#realm-change-dialog").open'), true);
      await release(); current = await changed(pending, "confirm change");
      assert.equal(current.player.abilityLearning.realms.secondRealmId, "life");
      assert.deepEqual(current.player.abilityLearning.realms.previousRealmIds, ["sorcery"]);
      assert.equal(current.player.abilities.find(a => a.id === primary).learned, true);
      assert.equal(current.player.abilities.filter(a => a.bookRealmId === "life").every(a => !a.learned), true);
      assert.equal(current.player.abilityLearning.remainingSlots, beforeChange.player.abilityLearning.remainingSlots);
      await closePage(); assert.equal((await snapshot()).stateHash, current.stateHash);
      const saved = await save(), savedState = current;
      await load(saved, current.stateHash); await abilitiesPage();
      assert.equal(await text("#spell-realms-value"), localization.format("ability-realms-value", { first: localization.format("realm-arcane-name"), second: localization.format("realm-life-name") }));
      const life = current.player.abilities.find(a => a.bookRealmId === "life" && a.canStudy).id;
      async function continueSave() {
        await abilitiesPage(); const learned = await study(life);
        const cast = await castAbility(mpToHp);
        return { learned: learned.stateHash, events: cast.events, after: cast.after };
      }
      const continued = await continueSave(); await closePage();
      const restored = await load(saved, savedState.stateHash);
      assert.deepEqual(restored.player, savedState.player);
      assert.deepEqual(restored.inventory, savedState.inventory);
      const replayed = await continueSave(); assert.deepEqual(replayed, continued);
      await screenshot(`${locale}-restored-continuation`);
      report.locales.push({ locale, builds, birthHash: born.stateHash, preparedLevels: [24, 25], attempts, pendingHash: pending, savedHash: savedState.stateHash, continuedHash: continued.after.stateHash });
      process.stdout.write(`Warrior-Mage ${locale}: menu, study, conversion, realm change and save UI passed.\n`);
    }
    assert.deepEqual(keyboard.errors, []);
    await writeFile(path.join(directory, "report.json"), `${JSON.stringify(report, null, 2)}\n`);
  } finally { keyboard.close(); }
}
