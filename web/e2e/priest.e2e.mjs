// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { Localization } from "../src/localization.ts";
import { PRIEST_SECOND_REALMS } from "../src/character-creation.ts";
import { selectCreationBuild, selectCreationRace } from "./character-creation.e2e.mjs";
import { connectKeyboard } from "./character-creation-layout.e2e.mjs";
import { nextWalk } from "./berserker.e2e.mjs";
import { prepareDungeonEntry } from "./dungeon-entry.e2e.mjs";

// Reports separate normal birth/dungeon play from explicit high-level preparation.
export async function runPriestUiScenario(driver, directory, profile, playthrough = false) {
  await mkdir(directory, { recursive: true });
  const keyboard = await connectKeyboard(profile);
  const sources = Object.fromEntries(await Promise.all(["zh-CN", "en-US"].map(async locale => [locale,
    await Promise.all(["ui", "content", "game"].map(file => readFile(new URL(`../../locales/${locale}/${file}.ftl`, import.meta.url), "utf8"))),
  ])));
  const localization = new Localization("zh-CN", sources);
  const report = { mode: playthrough ? "play" : "ui", preparation: `${playthrough ? "Life/Sorcery and Death/Sorcery created as normal level-1 humans; birth equipment, generated Warrens and real melee before any XP preparation. Optional fast-entry recorded per run." : "All 24 pairs inspected in both localized menus; Life/Sorcery and Death/Sorcery start as normal level-1 humans in both locales."} Explicit XP to 5, 34/35 or 41/42${playthrough ? " and 50" : ""}, quiet lit map, full HP/MP, Craft first book and an ordinary Dagger. At 35/42/50 a 3x3 floor and source Sheep are prepared; level 50 also adds current-realm fourth books. Natural cast failures retained. No natural leveling or item acquisition claim.`, runs: [] };
  const click = selector => driver.execute('document.querySelector(arguments[0]).click(); return true;', [selector]);
  const focus = selector => driver.execute('document.querySelector(arguments[0]).focus(); return true;', [selector]);
  const text = selector => driver.execute('return document.querySelector(arguments[0]).textContent;', [selector]);
  const focused = selector => driver.execute('return document.activeElement.matches(arguments[0]);', [selector]);
  const ready = () => driver.waitFor('return document.querySelector("#connection-status").classList.contains("ready")', "idle UI");
  async function invoke(command, args) {
    await driver.execute(`window.__priestDone=false; window.__priestError=null;
      window.__TAURI_INTERNALS__.invoke(arguments[0],arguments[1]).then(value=>{window.__priestResult=value;window.__priestDone=true;},error=>window.__priestError=String(error));return true;`, [command, args]);
    await driver.waitFor('return window.__priestDone || window.__priestError', command, 30_000);
    assert.equal(await driver.execute('return window.__priestError'), null);
    return driver.execute('return window.__priestResult');
  }
  const snapshot = () => invoke("inspect_game_e2e");
  async function changed(before, label) {
    await driver.waitFor('return document.querySelector("#hash-value").title!==arguments[0]', label, 15_000, [before]);
    await ready(); return snapshot();
  }
  async function save() {
    await invoke("save_game", { savedAt: "2026-09-12T12:00:00Z" });
    return driver.execute('window.__priestSaves??=[];return window.__priestSaves.push(window.__priestResult)-1;');
  }
  async function load(index, hash) {
    await driver.execute(`window.__priestLoaded=false;const original=window.fetch,endpoint=window.__TAURI_INTERNALS__.convertFileSrc("load_game","ipc");
      window.fetch=(url,options)=>{if(url!==endpoint)return original(url,options);window.fetch=original;return original(url,options).then(response=>{setTimeout(()=>window.__priestLoaded=true,0);return response;});};
      const transfer=new DataTransfer();transfer.items.add(new File([new Uint8Array(window.__priestSaves[arguments[0]])],"priest-ui.rfbsave"));
      const input=document.querySelector("#load-input");input.files=transfer.files;input.dispatchEvent(new Event("change",{bubbles:true}));return true;`, [index]);
    await driver.waitFor('return window.__priestLoaded && document.querySelector("#hash-value").title===arguments[0] && document.querySelector("#connection-status").classList.contains("ready")', "native save import", 30_000, [hash]);
    return snapshot();
  }
  async function page() {
    await click(await driver.execute('return document.querySelector("#player-page-dialog").open') ? "#player-page-tab-ability" : "#player-ui-ability-open");
    await driver.waitFor('return document.querySelector("#ability-list").checkVisibility()', "books and powers");
  }
  async function closePage() {
    if (await driver.execute('return document.querySelector("#player-page-dialog").open')) await keyboard.key("Escape");
  }
  async function talent() {
    if ((await snapshot()).player.pendingRaceMutationChoice) {
      await click("#player-ui-character-open"); await click("#character-tab-other");
      const before = (await snapshot()).stateHash;
      await focus(".mutation-choice-candidate"); await keyboard.key("Enter");
      await changed(before, "human talent"); await closePage();
    }
  }
  async function prepare(level) {
    await closePage();
    const prepared = await invoke("prepare_spell_learning_e2e", { level });
    await load(await save(), prepared.stateHash); await talent(); await page(); return snapshot();
  }
  async function viewport(width, height, zoom = 1) {
    await invoke("plugin:webview|set_webview_zoom", { label: "main", value: zoom });
    const rect = await driver.command("GET", "/window/rect");
    const current = await driver.execute('return {width:innerWidth,height:innerHeight}');
    await driver.command("POST", "/window/rect", { width: Math.round(rect.width + (width-current.width)*zoom), height: Math.round(rect.height + (height-current.height)*zoom) });
    await driver.waitFor('return Math.abs(innerWidth-arguments[0])<=1 && Math.abs(innerHeight-arguments[1])<=1', "viewport", 10_000, [width, height]);
  }
  async function layout(selector) {
    assert.equal(await driver.execute(`const n=document.querySelector(arguments[0]),r=n.getBoundingClientRect();return n.checkVisibility()&&n.scrollWidth<=n.clientWidth+1&&r.left>=-1&&r.right<=innerWidth+1&&r.top>=-1&&r.bottom<=innerHeight+1;`, [selector]), true, selector);
  }
  async function screenshot(name) {
    await driver.execute('window.__priestPainted=false;requestAnimationFrame(()=>requestAnimationFrame(()=>window.__priestPainted=true));return true;');
    await driver.waitFor('return window.__priestPainted', "paint");
    await writeFile(path.join(directory, `${name}.png`), await driver.screenshot(), "base64");
  }
  async function hold(command) {
    await driver.execute(`const original=window.fetch,endpoint=window.__TAURI_INTERNALS__.convertFileSrc(arguments[0],"ipc");window.__priestDispatches=0;window.__priestRelease=null;
      window.fetch=(url,options)=>url!==endpoint?original(url,options):new Promise((resolve,reject)=>{window.__priestDispatches++;window.__priestRelease=()=>{window.fetch=original;original(url,options).then(resolve,reject);};});return true;`, [command]);
  }
  async function release() {
    assert.equal(await driver.execute('return window.__priestDispatches'), 1);
    await driver.execute('window.__priestRelease();return true;');
  }
  async function study(realm, busy = false, rank) {
    const before = await snapshot();
    const ability = before.player.abilities.find(a => a.bookRealmId === realm && a.canStudy && (!rank || a.bookRank === rank));
    assert.ok(ability, `random study candidate in ${realm}`);
    const selector = `.ability-book-heading[data-book-item-id="${ability.bookItemId}"] button`;
    assert.equal(await text(selector), localization.format("action-ability-study-random"));
    if (busy) await hold("dispatch_game_command");
    await focus(selector); await keyboard.key("Enter");
    if (busy) {
      await driver.waitFor('return !!window.__priestRelease', "held study");
      assert.equal(await driver.execute('return [...document.querySelectorAll("#ability-list button")].every(n=>n.matches(":disabled"))'), true);
      await keyboard.key("Enter"); await release();
    }
    const after = await changed(before.stateHash, "random study");
    const gifts = after.player.abilities.filter(a => a.learned && !before.player.abilities.find(b => b.id === a.id)?.learned);
    assert.equal(gifts.length, 1); assert.equal(gifts[0].bookRealmId, realm);
    return gifts[0];
  }
  const penalty = state => state.player.traitDetails.stats.find(stat => stat.id === "priest-blade-failure").value;
  const powerRow = id => `[data-ability-id="${id}"]`;
  async function actKey(key) {
    const before = (await snapshot()).stateHash;
    await keyboard.key(key); return changed(before, `key ${key}`);
  }
  async function meleeHit() {
    await closePage();
    const visited = new Set();
    let current = await snapshot();
    for (let step = 0; step < 180; step++) {
      assert.equal(current.player.isDead, false);
      visited.add(`${current.player.position.x},${current.player.position.y}`);
      const distance = p => Math.max(Math.abs(p.x-current.player.position.x), Math.abs(p.y-current.player.position.y));
      const target = current.entities.filter(e => e.faction === "hostile").sort((a,b) => distance(a.position)-distance(b.position))[0];
      const before = current;
      current = await actKey(nextWalk(current, visited, target?.position));
      const hpAfter = target ? current.entities.find(e => e.id === target.id)?.hp ?? 0 : null;
      if (target && hpAfter < target.hp) {
        assert.equal(current.player.isDead, false);
        return { target: target.kindId, hpBefore: target.hp, hpAfter, before: before.stateHash, after: current.stateHash, playerHp: current.player.hp };
      }
    }
    assert.fail("Priest must hit a monster through real melee input");
  }
  async function castSpell(id) {
    await page();
    const before = await snapshot(), ability = before.player.abilities.find(a => a.id === id);
    assert.equal(ability?.canCast, true, `${id}: ${ability?.unavailableReason}`);
    await focus(`${powerRow(id)} .ability-cast-action`); await keyboard.key("Enter");
    if (ability.targetSpec.modes.some(mode => mode === "position" || mode === "direction")) {
      await driver.waitFor('return document.querySelector("#map-host").dataset.targeting === "true"', "spell target");
      await keyboard.key("4"); await keyboard.key("Enter");
    }
    const after = await changed(before.stateHash, `cast ${id}`);
    const resolution = await driver.execute('return window.__priestUpdate.events.find(e=>e.outcome?.type==="ability-cast")?.outcome.resolution');
    assert.equal(resolution.abilityId, id); assert.equal(after.player.isDead, false);
    await closePage(); return { state: after, resolution };
  }
  async function successfulSpell(id, level) {
    const attempts = [];
    for (let attempt = 0; attempt < 24; attempt++) {
      if (!(await snapshot()).player.abilities.find(a => a.id === id).canCast) await prepare(level);
      const cast = await castSpell(id); attempts.push(cast.resolution);
      if (cast.resolution.succeeded) return attempts;
    }
    assert.fail(`No natural success casting ${id}`);
  }
  async function exportPlayedSave(name) {
    await closePage();
    await driver.execute(`window.__priestExport=null;window.__priestDownloadHooks=[URL.createObjectURL,URL.revokeObjectURL,HTMLAnchorElement.prototype.click];
      URL.createObjectURL=blob=>{window.__priestExport={blob};return "blob:priest-acceptance";};URL.revokeObjectURL=()=>{};
      HTMLAnchorElement.prototype.click=function(){window.__priestExport.name=this.download;};document.querySelector('.hud-menu').open=true;return true;`);
    await click("#save-button"); await driver.waitFor('return window.__priestExport?.name?.endsWith(".rfbsave")', "menu export");
    await driver.execute(`document.querySelector('.hud-menu').open=false;
      [URL.createObjectURL,URL.revokeObjectURL,HTMLAnchorElement.prototype.click]=window.__priestDownloadHooks;
      window.__priestExportIndex=null;window.__priestExport.blob.arrayBuffer().then(buffer=>{
        window.__priestSaves??=[];window.__priestExportIndex=window.__priestSaves.push(Array.from(new Uint8Array(buffer)))-1;});return true;`);
    await driver.waitFor('return window.__priestExportIndex!==null', "save bytes");
    const index = await driver.execute('return window.__priestExportIndex');
    await writeFile(path.join(directory, `${name}.rfbsave`), Buffer.from(await driver.execute('return window.__priestSaves[arguments[0]]', [index])));
    return index;
  }
  try {
    await invoke("plugin:window|set_min_size", { label: "main", value: null });
    for (const locale of playthrough ? ["zh-CN"] : ["zh-CN", "en-US"]) {
      localization.setLocale(locale);
      for (const primary of ["life", "death"]) {
        await driver.execute('localStorage.setItem("rfb.locale",arguments[0]);localStorage.setItem("rfb.input-preset","numpad");return true;', [locale]);
        await keyboard.reload();
        await driver.waitFor('return document.documentElement.dataset.appMode==="title"&&!document.querySelector("#session-new-game").disabled', "localized title", 60_000);
        await driver.execute(`const original=window.fetch,endpoint=window.__TAURI_INTERNALS__.convertFileSrc("dispatch_game_command","ipc");window.fetch=(url,options)=>url!==endpoint?original(url,options):original(url,options).then(async response=>{window.__priestUpdate=await response.clone().json();return response;});return true;`);
        await viewport(1280, 720); await click("#session-new-game");
        await driver.execute('document.querySelector("#session-character-name").value="Priest UI";document.querySelector("#session-seed").value="925";return true;');
        await selectCreationRace(driver, "demo.race.rfb-human");
        const menus = [];
        if (primary === "life" && !playthrough) {
          await selectCreationBuild(driver, "demo.build.warrior");
          const confirmed = await text("#session-creation-summary");
          await click('[data-career-group="prayer"]'); await focus('[data-career-id="priest"]'); await keyboard.key("Enter");
          assert.equal(await driver.execute('return document.querySelector("#session-start-game").disabled'), true);
          await focus('[data-career-id="priest-life"]'); await keyboard.key("Enter");
          await driver.execute('document.querySelector("#session-new-game-view").requestSubmit();return true;');
          assert.equal(await driver.execute('return document.documentElement.dataset.appMode'), "new-game");
          await keyboard.key("Escape"); assert.equal(await focused('[data-career-id="priest-life"]'), true);
          await keyboard.key("Escape"); assert.equal(await focused('[data-career-id="priest"]'), true);
          assert.equal(await text("#session-creation-summary"), confirmed);
          for (const [first, seconds] of Object.entries(PRIEST_SECOND_REALMS)) {
            for (const second of seconds) {
              const id = `demo.build.priest-${first}-${second}`;
              await selectCreationBuild(driver, id);
              const ids = await driver.execute('return [...document.querySelectorAll("#session-career-options [data-career-id]")].map(n=>n.dataset.careerId)');
              assert.deepEqual(ids, seconds.map(realm => `demo.build.priest-${first}-${realm}`));
              assert.equal(await driver.execute('return document.querySelector(arguments[0]).getAttribute("aria-pressed")', [`[data-career-id="${id}"]`]), "true");
              assert.ok((await text("#session-career-notes")).includes(localization.format("session-priest-second-realm-help")));
              menus.push(id);
            }
          }
          assert.equal(new Set(menus).size, 24);
          for (const [width, height, zoom] of [[390, 844, 1], [640, 360, 2]]) {
            await viewport(width, height, zoom); await selectCreationBuild(driver, "demo.build.priest-life-sorcery");
            await focus('[data-career-id="demo.build.priest-life-sorcery"]'); await keyboard.key("End");
            assert.equal(await focused('[data-career-id="demo.build.priest-life-armageddon"]'), true);
            await keyboard.key("Home"); await keyboard.key("Enter");
            await layout("#session-new-game-view"); await layout("#session-start-game"); await screenshot(`${locale}-creation-${width}-${zoom}`);
          }
          await viewport(1280, 720);
        }
        const build = `demo.build.priest-${primary}-sorcery`;
        await selectCreationBuild(driver, build); await hold("initialize_game");
        await focus("#session-start-game"); await keyboard.key("Enter");
        await driver.waitFor('return !!window.__priestRelease', "held creation");
        assert.equal(await driver.execute('return [...document.querySelectorAll("#session-new-game-view button,#session-new-game-view input")].every(n=>n.matches(":disabled"))'), true);
        await keyboard.key("Enter"); await keyboard.key("Escape"); await release();
        await driver.waitFor('return document.documentElement.dataset.appMode==="playing"&&document.querySelector("#connection-status").classList.contains("ready")', "normal Priest birth", 60_000);
        await talent(); await page();
        const born = await snapshot();
        assert.equal(born.player.build.buildId, build); assert.equal(born.player.progress.level, 1);
        assert.equal(born.player.abilityLearning.studyMode, "divine-random"); assert.ok(born.player.abilityLearning.capacity > 0);
        assert.equal(await text("#spell-realms-study-help"), localization.format("ability-random-study-help"));
        assert.equal(await driver.execute('return document.querySelectorAll("[data-ability-action=study]").length'), 0);
        const good = primary === "life", power = `demo.ability.priest-${good ? "bless-weapon" : "evocation"}`, level = good ? 35 : 42;
        assert.equal(born.player.abilities.find(a => a.id === power).unavailableReason, "level-too-low");
        assert.equal(born.player.abilities.some(a => a.id === `demo.ability.priest-${good ? "evocation" : "bless-weapon"}`), false);
        await screenshot(`${locale}-${primary}-level-one`);
        const played = {};
        if (playthrough) {
          await closePage(); await click("#player-ui-inventory-open");
          const torch = born.inventory.find(item => item.kindId === "demo.item.wooden-torch"); assert.ok(torch);
          await click(`[data-item-id="${torch.id}"] input[type="checkbox"]`); await click("#inventory-equip");
          await changed(born.stateHash, "birth torch"); await closePage();
          let outside = await snapshot();
          if (process.argv.includes("--fast-entry")) { played.fastEntry = await prepareDungeonEntry(driver); outside = await snapshot(); }
          else for (let step = 0; step < 120; step++) {
            const entry = outside.cells.find(c => c.terrainId === "demo.terrain.stairs-down").position;
            if (outside.player.position.x === entry.x && outside.player.position.y === entry.y) break;
            outside = await actKey(nextWalk(outside, new Set(), entry));
          }
          await click("#traverse-stairs");
          assert.equal((await changed(outside.stateHash, "generated dungeon")).floorId, "demo.floor.warrens-depth-1");
          played.naturalMelee = await meleeHit();
          await screenshot(`${primary}-natural-melee`); await exportPlayedSave(`${primary}-level-one-dungeon`);
          process.stdout.write(`Priest ${primary}: normal birth, generated dungeon, melee hit and survival passed.\n`);
        }
        await prepare(5);
        const firstGift = await study(primary, true), secondGift = await study("sorcery");
        assert.equal(firstGift.proficiencyCap, 1600); assert.equal(secondGift.proficiencyCap, 1400);
        if (playthrough) {
          played.spellCasts = [await successfulSpell(firstGift.id, 5), await successfulSpell(secondGift.id, 5)];
        }
        let current = await prepare(level - 1);
        assert.equal(current.player.abilities.find(a => a.id === power).unavailableReason, "level-too-low");
        await closePage(); await click("#player-ui-inventory-open");
        await click('[data-item-id="e2e.priest-dagger"] input[type="checkbox"]'); await click("#inventory-equip");
        current = await changed(current.stateHash, "equip unblessed blade");
        assert.equal(penalty(current), good ? 25 : 0);
        current = await prepare(level);
        const available = current.player.abilities.find(a => a.id === power);
        assert.equal(available.canCast, true); assert.equal(available.governingAttribute, "wisdom");
        assert.equal(await text("#spell-realms-weapon-note"), localization.format("ability-priest-blade-penalty", { value: good ? 25 : 0 }));
        assert.ok((await text(powerRow(power))).includes(localization.format("ability-priest-power-cost-help")));
        const attempts = [];
        if (good) {
          await focus(`${powerRow(power)} .ability-cast-action`); await keyboard.key("Enter");
          await driver.waitFor('return document.querySelector(".item-target-dialog")?.open', "blessing targets");
          await keyboard.key("Escape");
          assert.equal((await snapshot()).stateHash, current.stateHash);
        } else {
          const effect = available.effects.find(effect => effect.type === "evocation");
          assert.ok((await text(powerRow(power))).includes(localization.format("ability-evocation-summary", { damage: effect.damage, power: effect.power })));
        }
        for (let attempt = 0; attempt < 24; attempt++) {
          if (attempt) current = await prepare(level);
          await page();
          const before = current, projected = current.player.abilities.find(a => a.id === power);
          await focus(`${powerRow(power)} .ability-cast-action`); await keyboard.key("Enter");
          if (good) {
            await driver.waitFor('return document.querySelector(".item-target-dialog")?.open', "blessing target selection");
            await driver.execute('const dialog=document.querySelector(".item-target-dialog");dialog.querySelector("select").value="e2e.priest-dagger";dialog.querySelector("form").requestSubmit();return true;');
          }
          current = await changed(before.stateHash, "class power");
          const resolution = await driver.execute('return window.__priestUpdate.events.find(e=>e.outcome?.type==="ability-cast")?.outcome.resolution');
          assert.equal(resolution.abilityId, power); assert.equal(resolution.resourcePaid, projected.resourceCost); assert.equal(resolution.hpPaid, projected.hitPointCost);
          assert.equal(current.player.isDead, false); attempts.push(resolution);
          if (resolution.succeeded) break;
        }
        assert.ok(attempts.some(result => result.succeeded), "natural class power success");
        assert.equal(penalty(current), 0);
        if (playthrough) {
          if (good) {
            played.blessedMelee = await meleeHit();
            played.blessedPrayer = await successfulSpell(firstGift.id, level);
          } else {
            assert.equal(current.entities.some(entity => entity.id === "e2e.priest-power-target"), false, "Evocation kills the prepared Sheep");
            played.evocationTarget = "demo.actor.sheep";
          }
          current = await prepare(50);
          const high = current.player.abilities.filter(a => a.bookRank === 4);
          assert.equal(high.length, 16);
          assert.ok(high.some(a => a.canStudy));
          played.highBooks = { abilities: high, learned: await study(primary, false, 4) };
          current = await snapshot();
        }
        await page(); await screenshot(`${locale}-${primary}-power-result`);
        for (const [width, height, zoom] of [[390, 844, 1], [640, 360, 2]]) {
          await viewport(width, height, zoom); await focus("#spell-realms-value");
          await layout("#player-page-dialog"); await layout("#ability-panel"); await screenshot(`${locale}-${primary}-books-${width}-${zoom}`);
        }
        const beforeChange = current;
        await focus('#realm-change-books [data-realm-id="craft"]'); await keyboard.key("Enter");
        current = await changed(beforeChange.stateHash, "request realm change");
        assert.equal(await focused("#realm-change-decline"), true);
        const pending = current.stateHash, pendingSave = await save();
        await layout("#realm-change-dialog"); await screenshot(`${locale}-${primary}-change-200-percent`);
        await viewport(390, 844); await layout("#realm-change-dialog");
        await keyboard.key("Escape"); current = await changed(pending, "cancel realm change");
        assert.equal(current.player.abilityLearning.realms.secondRealmId, "sorcery");
        assert.equal(current.player.abilityLearning.remainingSlots, beforeChange.player.abilityLearning.remainingSlots);
        await load(pendingSave, pending); assert.equal(await focused("#realm-change-decline"), true);
        await hold("dispatch_game_command"); await focus("#realm-change-accept"); await keyboard.key("Enter");
        await driver.waitFor('return !!window.__priestRelease', "held confirmation");
        assert.equal(await driver.execute('return [...document.querySelectorAll("#realm-change-dialog button")].every(n=>n.matches(":disabled"))'), true);
        await keyboard.key("Escape"); assert.equal(await driver.execute('return document.querySelector("#realm-change-dialog").open'), true); await release();
        current = await changed(pending, "confirm and study new realm");
        assert.equal(current.player.abilityLearning.realms.secondRealmId, "craft");
        assert.deepEqual(current.player.abilityLearning.realms.previousRealmIds, ["sorcery"]);
        assert.equal(current.player.abilities.filter(a => a.bookRealmId === "craft" && a.learned).length, 1);
        assert.equal(current.player.abilities.find(a => a.id === firstGift.id).learned, true);
        assert.equal(current.player.abilityLearning.remainingSlots, beforeChange.player.abilityLearning.remainingSlots - 1);
        assert.equal(await focused("#spell-realms-value"), true);
        const confirmed = current.stateHash;
        await closePage(); await load(await save(), confirmed); await page();
        assert.ok((await text("#spell-realms-history")).includes(localization.format("realm-sorcery-name")));
        assert.equal(penalty(await snapshot()), 0);
        await screenshot(`${locale}-${primary}-changed-and-restored`);
        if (playthrough) {
          const saved = await snapshot(), index = await exportPlayedSave(`${primary}-level50-changed-realm-test-prepared`);
          async function continueSave() {
            const first = await castSpell(firstGift.id);
            const powerCast = good ? await castSpell(firstGift.id) : await castSpell(power);
            await page(); const gift = await study("craft"); await closePage();
            return { first: first.resolution, second: powerCast.resolution, gift, hash: (await snapshot()).stateHash };
          }
          const continued = await continueSave(); await load(index, saved.stateHash);
          const restored = await snapshot();
          assert.deepEqual(restored.player, saved.player); assert.deepEqual(restored.inventory, saved.inventory); assert.deepEqual(restored.equipment, saved.equipment);
          const replayed = await continueSave(); assert.deepEqual(replayed, continued);
          played.saveContinuation = { saved: saved.stateHash, continued, replayed };
          await screenshot(`${primary}-saved-casts-study-continued`);
        }
        report.runs.push({ locale, build, menus, birthHash: born.stateHash, gifts: [firstGift.id, secondGift.id], preparedLevels: [5, level-1, level, ...(playthrough ? [50] : [])], powerAttempts: attempts, finalHash: (await snapshot()).stateHash, played });
        process.stdout.write(`Priest ${locale} ${primary}: birth, study, power, realm change and save UI passed.\n`);
      }
    }
    assert.deepEqual(keyboard.errors, []);
    await writeFile(path.join(directory, "report.json"), `${JSON.stringify(report, null, 2)}\n`);
  } finally { keyboard.close(); }
}
