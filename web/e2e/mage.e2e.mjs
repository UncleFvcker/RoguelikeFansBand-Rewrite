// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { Localization } from "../src/localization.ts";
import { MAGE_REALMS } from "../src/character-creation.ts";
import { selectCreationBuild, selectCreationRace } from "./character-creation.e2e.mjs";
import { connectKeyboard } from "./character-creation-layout.e2e.mjs";
import { nextWalk } from "./berserker.e2e.mjs";
import { prepareDungeonEntry } from "./dungeon-entry.e2e.mjs";

// Mage UI and new-game acceptance; all granted XP, books and devices are reported.
export async function runMageUiScenario(driver, directory, profile, playthrough = false) {
  await mkdir(directory, { recursive: true });
  await driver.waitFor('return document.documentElement.dataset.appMode === "title"', "Mage title", 60_000);
  const keyboard = await connectKeyboard(profile);
  const sources = Object.fromEntries(await Promise.all(["zh-CN", "en-US"].map(async locale => [locale,
    await Promise.all(["ui", "content", "game"].map(file => readFile(new URL(`../../locales/${locale}/${file}.ftl`, import.meta.url), "utf8"))),
  ])));
  const localization = new Localization("zh-CN", sources);
  const report = { preparation: "Normal human level-1 creation; explicit level-20 XP, quiet lit map, Life book, then real XP drain and recovery. No natural progression or item-acquisition claim.", locales: [] };
  const click = selector => driver.execute('document.querySelector(arguments[0]).click(); return true;', [selector]);
  const focus = selector => driver.execute('document.querySelector(arguments[0]).focus(); return true;', [selector]);
  const text = selector => driver.execute('return document.querySelector(arguments[0]).textContent;', [selector]);
  const focusIs = selector => driver.execute('return document.activeElement.matches(arguments[0]);', [selector]);
  const hash = () => driver.execute('return document.querySelector("#hash-value").title;');
  const ready = () => driver.waitFor('return document.querySelector("#connection-status").classList.contains("ready")', "idle UI");
  const row = id => `[data-ability-id="${id}"]`;
  const studyButton = id => `${row(id)} [data-ability-action="study"]`;
  const realmButton = id => `#realm-change-books [data-realm-id="${id}"]`;
  async function invoke(command, args) {
    await driver.execute(`window.__mageResult = null; window.__mageDone = false; window.__mageError = null;
      window.__TAURI_INTERNALS__.invoke(arguments[0], arguments[1]).then(value => { window.__mageResult = value; window.__mageDone = true; }, error => window.__mageError = String(error)); return true;`, [command, args]);
    await driver.waitFor('return window.__mageDone || window.__mageError', command, 30_000);
    assert.equal(await driver.execute('return window.__mageError'), null);
    return driver.execute('return window.__mageResult');
  }
  const snapshot = () => invoke("inspect_game_e2e");
  async function save() {
    await invoke("save_game", { savedAt: "2026-09-11T02:00:00Z" });
    return driver.execute('window.__mageSaves ??= []; return window.__mageSaves.push(window.__mageResult) - 1;');
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
    await driver.execute('window.__magePainted = false; requestAnimationFrame(() => requestAnimationFrame(() => window.__magePainted = true)); return true;');
    await driver.waitFor('return window.__magePainted', "paint");
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
    await driver.execute(`const original = window.fetch, commandUrl = window.__TAURI_INTERNALS__.convertFileSrc(arguments[0], "ipc"); window.__mageDispatches = 0; window.__mageRelease = null;
      window.fetch = (url, options) => url !== commandUrl ? original(url, options) : new Promise((resolve, reject) => {
        window.__mageDispatches++; window.__mageRelease = () => { window.fetch = original; original(url, options).then(resolve, reject); };
      }); return true;`, [command]);
  }
  async function release() {
    assert.equal(await driver.execute('return window.__mageDispatches'), 1, "one IPC request while busy");
    await driver.execute('window.__mageRelease(); return true;');
  }
  async function load(bytes, expectedHash) {
    await driver.execute(`window.__mageLoaded = false; const original = window.fetch, commandUrl = window.__TAURI_INTERNALS__.convertFileSrc("load_game", "ipc");
      window.fetch = (url, options) => {
        if (url !== commandUrl) return original(url, options);
        window.fetch = original;
        return original(url, options).then(value => { setTimeout(() => window.__mageLoaded = true, 0); return value; });
      };
      const transfer = new DataTransfer(); transfer.items.add(new File([new Uint8Array(window.__mageSaves[arguments[0]])], "mage-ui.rfbsave"));
      const input = document.querySelector("#load-input"); input.files = transfer.files; input.dispatchEvent(new Event("change", { bubbles:true })); return true;`, [bytes]);
    await driver.waitFor('return window.__mageLoaded && document.querySelector("#hash-value").title === arguments[0]', "save loaded through UI", 30_000, [expectedHash]);
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
  async function actKey(key) {
    const before = await hash(); await keyboard.key(key);
    return changed(before, `native key ${key}`);
  }
  async function aim(position, origin) {
    await driver.waitFor('return document.querySelector("#map-host").dataset.targeting === "true"', "aiming");
    let dx = position.x - origin.x, dy = position.y - origin.y;
    while (dx || dy) {
      const sx = Math.sign(dx), sy = Math.sign(dy);
      await keyboard.key(({ "-1,-1":"7", "0,-1":"8", "1,-1":"9", "-1,0":"4", "1,0":"6", "-1,1":"1", "0,1":"2", "1,1":"3" })[`${sx},${sy}`]);
      dx -= sx; dy -= sy;
    }
    await keyboard.key("Enter");
  }
  async function cast(id, target) {
    await abilitiesPage();
    const before = await snapshot(), ability = before.player.abilities.find(a => a.id === id);
    assert.equal(ability?.canCast, true, `${id}: ${ability?.unavailableReason}`);
    await focus(`${row(id)} .ability-cast-action`); await keyboard.key("Enter");
    if (typeof target === "string") {
      await driver.waitFor('return document.querySelector(".item-target-dialog")?.open', "item target choice");
      await driver.execute('document.querySelector(".item-target-dialog select").value = arguments[0]; return true;', [target]);
      await click('.item-target-dialog button[type="submit"]');
    } else if (target) await aim(target, before.player.position);
    const after = await changed(before.stateHash, `cast ${id}`);
    assert.equal(after.player.isDead, false);
    await closePage();
    return after;
  }
  async function useItem(id, target) {
    await closePage(); await click("#player-ui-inventory-open");
    const before = await snapshot();
    assert.equal(before.inventory.find(item => item.id === id)?.usable, true, id);
    await driver.execute(`for (const input of document.querySelectorAll('#inventory-list input[type="checkbox"]:checked')) input.click();
      document.querySelector('[data-item-id="' + arguments[0] + '"] input[type="checkbox"]').click(); return true;`, [id]);
    await click("#inventory-use");
    if (target) await aim(target, before.player.position);
    const after = await changed(before.stateHash, `use ${id}`);
    assert.equal(after.player.isDead, false);
    await closePage();
    return after;
  }
  async function exportPlayedSave(name) {
    await driver.execute(`window.__mageExport = null;
      window.__mageDownloadHooks = [URL.createObjectURL, URL.revokeObjectURL, HTMLAnchorElement.prototype.click];
      URL.createObjectURL = blob => { window.__mageExport = { blob }; return "blob:mage-acceptance"; };
      URL.revokeObjectURL = () => {}; HTMLAnchorElement.prototype.click = function () { window.__mageExport.name = this.download; };
      document.querySelector('.hud-menu').open = true; return true;`);
    await click("#save-button");
    await driver.waitFor('return window.__mageExport?.name?.endsWith(".rfbsave")', "menu save export");
    await driver.execute(`document.querySelector('.hud-menu').open = false;
      [URL.createObjectURL, URL.revokeObjectURL, HTMLAnchorElement.prototype.click] = window.__mageDownloadHooks;
      window.__mageExportIndex = null; window.__mageExport.blob.arrayBuffer().then(buffer => {
        window.__mageSaves ??= []; window.__mageExportIndex = window.__mageSaves.push(Array.from(new Uint8Array(buffer))) - 1;
      }); return true;`);
    await driver.waitFor('return window.__mageExportIndex !== null', "exported bytes");
    const index = await driver.execute('return window.__mageExportIndex');
    await writeFile(path.join(directory, `${name}.rfbsave`), Buffer.from(await driver.execute('return window.__mageSaves[arguments[0]]', [index])));
    return index;
  }
  async function playNewGame(born) {
    const primary = "demo.ability.arcane-zap", secondary = "demo.ability.sorcery-detect-monsters";
    const eat = "demo.ability.mage-eat-magic", wand = "e2e.mage-wand", scroll = "e2e.mage-acquirement";
    const checks = [];
    assert.equal(born.player.build.raceId, "demo.race.rfb-human");
    assert.equal(born.player.progress.level, 1);
    await abilitiesPage(); let current = await study(primary);
    checks.push({ birth: { build:born.player.build.buildId, level:1, hash:born.stateHash, learningCapacity:born.player.abilityLearning.capacity, learned:primary, afterStudy:current.stateHash } });
    await closePage(); await click("#player-ui-inventory-open");
    const torch = current.inventory.find(item => item.kindId === "demo.item.wooden-torch");
    assert.ok(torch);
    await click(`[data-item-id="${torch.id}"] input[type="checkbox"]`);
    const beforeTorch = await hash(); await click("#inventory-equip");
    current = await changed(beforeTorch, "birth torch equipped"); await closePage();
    const beforeDevice = current;
    current = await prepare(1); await closePage();
    assert.equal(current.player.progress.level, 1);
    assert.deepEqual(current.player.position, beforeDevice.player.position);
    assert.equal(current.player.hp, beforeDevice.player.hp);
    assert.deepEqual(current.entities, beforeDevice.entities);
    checks.push({ preparation:"One generated, kind-aware Magic Missile wand; no XP, HP, map or monster preparation.", item:current.inventory.find(item => item.id === wand), before:beforeDevice.stateHash, after:current.stateHash });
    if (process.argv.includes("--fast-entry")) { checks.push({ fastEntry: await prepareDungeonEntry(driver) }); current = await snapshot(); }
    else for (let step = 0; step < 120; step++) {
      const entry = current.cells.find(cell => cell.terrainId === "demo.terrain.stairs-down").position;
      if (current.player.position.x === entry.x && current.player.position.y === entry.y) break;
      current = await actKey(nextWalk(current, new Set(), entry));
    }
    const outside = current.stateHash; await click("#traverse-stairs");
    current = await changed(outside, "natural dungeon entry");
    assert.equal(current.floorId, "demo.floor.warrens-depth-1");
    const visited = new Set();
    let spellHit, deviceHit;
    for (let step = 0; step < 180 && (!spellHit || !deviceHit || !current.player.abilities.find(a=>a.id===secondary).castCount); step++) {
      assert.equal(current.player.isDead, false);
      if (current.player.abilities.find(a=>a.id===secondary).canStudy && !current.player.abilities.find(a=>a.id===secondary).learned) {
        await abilitiesPage(); current = await study(secondary); await closePage();
        checks.push({ naturalSecondaryStudy:{level:current.player.progress.level,ability:secondary,hash:current.stateHash} });
      }
      visited.add(`${current.player.position.x},${current.player.position.y}`);
      const distance = position => Math.max(Math.abs(position.x - current.player.position.x), Math.abs(position.y - current.player.position.y));
      const target = current.entities.filter(entity => entity.faction === "hostile").sort((a,b) => distance(a.position)-distance(b.position))[0];
      if (!current.player.abilities.find(a=>a.id===secondary).castCount && current.player.abilities.find(a=>a.id===secondary).canCast && (!target || distance(target.position)>1)) {
        current = await cast(secondary); continue;
      }
      if (target && distance(target.position) <= 6) {
        const before = current;
        const useSpell = !spellHit && current.player.abilities.find(a=>a.id===primary).canCast;
        current = useSpell ? await cast(primary,target.position) : await useItem(wand,target.position);
        const remainingHp = current.entities.find(entity => entity.id === target.id)?.hp ?? 0;
        if (remainingHp < target.hp) {
          const result = { target:{id:target.id,kindId:target.kindId,hpBefore:target.hp,hpAfter:remainingHp}, before:before.stateHash, after:current.stateHash, level:current.player.progress.level, playerHp:current.player.hp };
          if (useSpell) { spellHit = result; await screenshot("natural-zap-hit"); }
          else { deviceHit = result; await screenshot("natural-wand-hit"); }
        }
      } else current = await actKey(nextWalk(current,visited,target?.position));
    }
    assert.ok(spellHit && deviceHit, "both a learned spell and an allowed device must hit natural monsters");
    assert.ok(current.player.abilities.find(a=>a.id===secondary).castCount > 0,"the naturally learned second realm spell must be cast");
    checks.push({ naturalDungeon:{floor:current.floorId,spellHit,deviceHit,detection:current.player.abilities.find(a=>a.id===secondary),alive:!current.player.isDead}, preparation:"The wand above is the only item fixture; normal dungeon generation, movement, monsters and combat RNG." });
    await exportPlayedSave("natural-start-with-test-wand");
    process.stdout.write("Mage natural start: both realms learned, detection, Zap and device hit; survived.\n");

    current = await prepare(25);
    assert.equal(current.player.progress.level,25);
    const proficiency = [];
    for (const [id,cap] of [[primary,1600],[secondary,1400]]) {
      const stages = [current.player.abilities.find(a=>a.id===id).proficiency];
      while (current.player.abilities.find(a=>a.id===id).canStudy) {
        current = await study(id); stages.push(current.player.abilities.find(a=>a.id===id).proficiency);
      }
      const ability = current.player.abilities.find(a=>a.id===id);
      assert.equal(ability.proficiency,cap); assert.equal(ability.proficiencyCap,cap);
      assert.equal(await driver.execute('return document.querySelector(arguments[0]).matches(":disabled")',[studyButton(id)]),true);
      proficiency.push({id,stages,cap,rendered:await text(row(id))});
    }
    await screenshot("level25-study-caps"); await closePage();
    // Spend real MP first; retain the real outer and internal Eat Magic failure rolls.
    for (let attempt = 0; attempt < 4; attempt++) current = await cast(secondary);
    const eatAttempts = [];
    for (let attempt = 0; attempt < 30; attempt++) {
      const before = current, device = before.inventory.find(item=>item.id===wand);
      current = await cast(eat,wand);
      const afterDevice = current.inventory.find(item=>item.id===wand);
      const mana = state => state.player.resources.find(resource=>resource.id==="demo.resource.mana").current;
      const entry = {before:before.stateHash,after:current.stateHash,mpBefore:mana(before),mpAfter:mana(current),spBefore:device.charges.current,spAfter:afterDevice?.charges.current,destroyed:!afterDevice};
      eatAttempts.push(entry);
      if (entry.mpAfter > entry.mpBefore && entry.spAfter < entry.spBefore) break;
      assert.ok(afterDevice,"prepared wand survived Eat Magic until a successful drain");
    }
    assert.ok(eatAttempts.some(entry=>entry.mpAfter>entry.mpBefore && entry.spAfter<entry.spBefore));
    await abilitiesPage(); await focus(row(eat)); await screenshot("level25-eat-magic");
    checks.push({ preparation:"Real XP grant to level 25; clear active monsters, light map and refill HP/MP; add Life first book and one Acquirement scroll. No proficiency or success/RNG override.", level:25, proficiency, eatMagic:eatAttempts });

    const beforeChange = current;
    await focus(realmButton("life")); await keyboard.key("Enter");
    current = await changed(beforeChange.stateHash,"played realm request");
    await focus("#realm-change-accept"); await keyboard.key("Enter");
    current = await changed(current.stateHash,"played realm confirmed");
    assert.equal(current.player.abilityLearning.realms.secondRealmId,"life");
    assert.deepEqual(current.player.abilityLearning.realms.previousRealmIds,["sorcery"]);
    assert.equal(current.player.abilities.find(a=>a.id===primary).proficiency,1600);
    const healing = "demo.ability.life-cure-light-wounds";
    current = await study(healing); await closePage();
    const saved = current, savedIndex = await exportPlayedSave("level25-changed-realm-test-prepared");
    async function continuePlayedSave() {
      let state = await snapshot();
      state = await cast(primary,{x:state.player.position.x+1,y:state.player.position.y});
      state = await cast(healing);
      const beforeItems = new Set(state.items.map(item=>item.id));
      state = await useItem(scroll);
      assert.equal(state.inventory.some(item=>item.id===scroll),false);
      const generated = state.items.filter(item=>!beforeItems.has(item.id));
      assert.ok(generated.length>0,"Acquirement must produce a real ground item");
      return {state,generated};
    }
    const continued = await continuePlayedSave();
    await load(savedIndex,saved.stateHash);
    const restored = await snapshot();
    assert.deepEqual(restored.player,saved.player);
    assert.deepEqual(restored.inventory,saved.inventory);
    assert.deepEqual(restored.equipment,saved.equipment);
    const replayed = await continuePlayedSave();
    assert.equal(replayed.state.stateHash,continued.state.stateHash,"same casts and generation preserve complete state and RNG after load");
    assert.deepEqual(replayed.generated,continued.generated);
    await screenshot("changed-save-casts-generation-continue");
    checks.push({ saveContinue:{saved:saved.stateHash,realms:saved.player.abilityLearning.realms,continued:continued.state.stateHash,restoredContinuation:replayed.state.stateHash,generated:continued.generated}, preparation:"Menu export and normal native import; real spell failure/effect and Acquirement rolls. Scroll is the explicitly prepared one above." });
    await writeFile(path.join(directory,"checks.json"),JSON.stringify({build:born.player.build.buildId,checks},null,2)+"\n");
    process.stdout.write("Mage level 25: source study caps, Eat Magic, realm change and saved spell/generation continuation passed.\n");
  }
  try {
    await invoke("plugin:window|set_min_size", { label: "main", value: null });
    for (const locale of playthrough ? ["zh-CN"] : ["zh-CN", "en-US"]) {
      localization.setLocale(locale);
      await driver.execute('localStorage.setItem("rfb.locale", arguments[0]); return true;', [locale]);
      await keyboard.reload();
      await driver.waitFor('return document.documentElement.dataset.appMode === "title" && !document.querySelector("#session-new-game").disabled', "localized title", 60_000);
      await viewport(1280, 720);
      await click("#session-new-game");
      await driver.execute('document.querySelector("#session-character-name").value = "Mage UI"; document.querySelector("#session-seed").value = "925"; return true;');
      const tested = [];
      for (const first of MAGE_REALMS) for (const second of MAGE_REALMS) if (first !== second) {
        const id = `demo.build.mage-${first}-${second}`;
        await selectCreationBuild(driver, id);
        const selection = await driver.execute(`return { choices:document.querySelector("#session-career-options").children.length,
          selected:document.querySelectorAll('#session-career-options [aria-pressed="true"]').length,
          disabled:document.querySelector("#session-start-game").disabled, pending:document.querySelector("#session-career-pending").hidden,
          description:document.querySelector("#session-career-description").textContent };`);
        assert.deepEqual(selection, { choices:7, selected:1, disabled:false, pending:true, description:localization.format(`build-demo-mage-${first}-${second}-description`) });
        tested.push(id);
      }
      await selectCreationBuild(driver, "demo.build.warrior");
      const confirmed = await text("#session-creation-summary");
      await focus('[data-career-group="magic"]'); await keyboard.key("Enter"); await keyboard.key("Enter");
      assert.equal(await driver.execute('return document.querySelector("#session-start-game").disabled'), true);
      await driver.execute('document.querySelector("#session-new-game-view").requestSubmit(); return true;');
      assert.equal(await driver.execute('return document.documentElement.dataset.appMode'), "new-game");
      await keyboard.key("Home"); await keyboard.key("ArrowDown"); await keyboard.key(" ");
      assert.equal(await driver.execute('return document.querySelector("#session-career-options").children.length'), 7);
      assert.equal(await driver.execute('return !!document.querySelector(\'[data-career-id="demo.build.mage-sorcery-sorcery"]\')'), false);
      assert.ok((await text("#session-career-path")).includes(localization.format("session-second-realm-label")));
      await keyboard.key("Escape");
      assert.equal(await focusIs('[data-career-id="mage-sorcery"]'), true);
      assert.equal(await driver.execute('return document.querySelector("#session-start-game").disabled'), true);
      await keyboard.key("Escape");
      assert.equal(await focusIs('[data-career-id="mage"]'), true);
      assert.equal(await text("#session-creation-summary"), confirmed);
      await keyboard.key("Enter"); await keyboard.key("Enter");
      await click("#session-tab-race");
      assert.equal(await text("#session-creation-summary"), confirmed, "changing page cancels an incomplete pair");
      await selectCreationBuild(driver, "demo.build.mage-life-death");
      await selectCreationRace(driver, "rfb-legacy.race.tonberry");
      assert.equal(await driver.execute('return document.querySelector("#session-start-game").disabled'), false);
      await selectCreationBuild(driver, "demo.build.duelist");
      assert.equal(await driver.execute('return document.querySelector("#session-start-game").disabled'), true);
      const build = playthrough ? "demo.build.mage-arcane-sorcery" : "demo.build.mage-death-sorcery";
      await selectCreationBuild(driver, build);
      assert.equal(await driver.execute('return document.querySelector("#session-start-game").disabled'), false);
      await selectCreationRace(driver, "demo.race.rfb-human");
      await selectCreationBuild(driver, "demo.build.high-mage-death");
      assert.equal(await driver.execute('return document.querySelector("#session-career-options").children.length'), 2);
      for (const [width, height, zoom] of [[1280,720,1], [390,844,1], [640,360,2]]) {
        await viewport(width, height, zoom);
        await click('[data-career-group="magic"]'); await click('[data-career-id="mage"]');
        await checkLayout("#session-new-game-view");
        await checkLayout("#session-career-path");
        await checkLayout("#session-start-game");
        await focus('[data-career-id="mage-life"]'); await keyboard.key("End");
        assert.equal(await focusIs('[data-career-id="mage-armageddon"]'), true);
        await keyboard.key("Enter"); await keyboard.key("Home"); await keyboard.key("Enter");
        assert.equal(await focusIs('[data-career-id="demo.build.mage-armageddon-life"]'), true);
        await checkLayout("#session-career-path");
        await screenshot(`${locale}-creation-${width}-${zoom}`);
      }
      await viewport(1280,720);
      await selectCreationBuild(driver, build);
      await hold("initialize_game");
      await focus("#session-start-game"); await keyboard.key("Enter");
      await driver.waitFor('return !!window.__mageRelease', "creation request held");
      assert.equal(await driver.execute('return [...document.querySelectorAll("#session-new-game-view button, #session-new-game-view input")].every(node => node.disabled)'), true);
      await keyboard.key("Escape");
      await driver.execute('document.querySelector("#session-new-game-view").requestSubmit(); return true;');
      await release();
      await driver.waitFor('return document.documentElement.dataset.appMode === "playing" && document.querySelector("#connection-status").classList.contains("ready")', "normal Mage birth", 60_000);
      let current = await chooseTalent();
      assert.equal(current.player.progress.level, 1);
      assert.equal(current.player.build.buildId, build);
      assert.equal(current.player.abilityLearning.realms.firstRealmId, playthrough ? "arcane" : "death");
      assert.equal(current.player.abilityLearning.realms.secondRealmId, "sorcery");
      if (playthrough) {
        await playNewGame(current);
        assert.deepEqual(keyboard.errors, []);
        return;
      }
      current = await prepare(20);
      assert.equal(current.player.progress.level, 20);
      assert.equal(await text("#spell-realms-study-help"), localization.format("ability-mage-study-help"));
      const primary = current.player.abilities.find(a => a.bookRealmId === "death" && a.canStudy && a.minimumLevel === 1).id;
      const secondary = current.player.abilities.find(a => a.bookRealmId === "sorcery" && a.canStudy && a.minimumLevel === 1).id;
      current = await study(primary); current = await study(secondary, " ");
      assert.equal(current.player.abilities.find(a => a.id === primary).proficiencyCap, 1600);
      assert.equal(current.player.abilities.find(a => a.id === secondary).proficiencyCap, 1400);
      assert.equal(await text(studyButton(secondary)), localization.format("action-ability-restudy"));
      const beforeRepeat = current;
      await hold("dispatch_game_command");
      await focus(studyButton(secondary)); await keyboard.key("Enter");
      await driver.waitFor('return !!window.__mageRelease', "restudy request held");
      assert.equal(await driver.execute('return [...document.querySelectorAll("#ability-list button")].every(button => button.matches(":disabled"))'), true);
      await keyboard.key("Enter"); await click(studyButton(secondary));
      await release();
      current = await changed(beforeRepeat.stateHash, "one repeat study");
      assert.equal(current.player.abilityLearning.remainingSlots, beforeRepeat.player.abilityLearning.remainingSlots - 1);
      assert.ok(current.player.abilities.find(a => a.id === secondary).proficiency > beforeRepeat.player.abilities.find(a => a.id === secondary).proficiency);
      assert.equal(await focusIs(studyButton(secondary)), true);
      const advanced = current.player.abilities.find(a => a.bookRealmId === "death" && a.canStudy && a.minimumLevel > 1).id;
      current = await study(advanced);
      current = await prepare(0);
      assert.equal(current.player.progress.level, 1);
      assert.equal(current.player.abilities.find(a => a.id === advanced).forgotten, true);
      assert.ok((await text(row(advanced))).includes(localization.format("ability-status-forgotten")));
      assert.equal(await driver.execute('return document.querySelector(arguments[0]).matches(":disabled")', [studyButton(advanced)]), true);
      await screenshot(`${locale}-forgotten`);
      current = await prepare(20);
      assert.equal(current.player.abilities.find(a => a.id === advanced).forgotten, undefined);
      assert.equal(current.player.abilities.find(a => a.id === advanced).learned, true);
      for (const [width, height, zoom] of [[390,844,1], [640,360,2]]) {
        await viewport(width,height,zoom);
        await focus("#spell-realms-value");
        await checkLayout("#player-page-dialog"); await checkLayout("#ability-panel");
        await screenshot(`${locale}-books-${width}-${zoom}`);
      }
      const beforeChange = current;
      await focus(realmButton("life")); await keyboard.key("Enter");
      current = await changed(beforeChange.stateHash, "realm change requested");
      assert.equal(await focusIs("#realm-change-decline"), true);
      assert.equal(await text("#realm-change-description"), localization.format("realm-change-description", {
        old:localization.format("realm-sorcery-name"), first:localization.format("realm-death-name"), next:localization.format("realm-life-name"),
      }));
      const pendingHash = current.stateHash, pendingSave = await save();
      await checkLayout("#realm-change-dialog");
      await screenshot(`${locale}-change-200-percent`);
      await viewport(390,844);
      await checkLayout("#realm-change-dialog"); await screenshot(`${locale}-change-390`);
      await keyboard.key("Escape");
      current = await changed(pendingHash, "Escape keeps original realm");
      assert.equal(current.player.abilityLearning.realms.secondRealmId, "sorcery");
      assert.equal(current.player.abilityLearning.remainingSlots, beforeChange.player.abilityLearning.remainingSlots);
      await load(pendingSave, pendingHash);
      assert.equal(await focusIs("#realm-change-decline"), true, "pending save restores confirmation");
      await hold("dispatch_game_command");
      await keyboard.key("Tab"); assert.equal(await focusIs("#realm-change-accept"), true);
      await keyboard.key("Enter");
      await driver.waitFor('return !!window.__mageRelease', "confirmation request held");
      assert.equal(await driver.execute('return [...document.querySelectorAll("#realm-change-dialog button")].every(button => button.disabled)'), true);
      await keyboard.key("Escape");
      assert.equal(await driver.execute('return document.querySelector("#realm-change-dialog").open'), true);
      await release();
      current = await changed(pendingHash, "realm confirmed once");
      assert.equal(current.player.abilityLearning.realms.secondRealmId, "life");
      assert.deepEqual(current.player.abilityLearning.realms.previousRealmIds, ["sorcery"]);
      assert.equal(current.player.abilities.some(a => a.bookRealmId === "sorcery"), false);
      assert.equal(current.player.abilities.find(a => a.id === primary).learned, true);
      assert.equal(current.player.abilities.filter(a => a.bookRealmId === "life").every(a => !a.learned), true);
      assert.equal(current.player.abilityLearning.remainingSlots, beforeChange.player.abilityLearning.remainingSlots);
      assert.equal(await focusIs("#spell-realms-value"), true);
      const confirmedHash = current.stateHash;
      await keyboard.key("Escape");
      assert.equal((await snapshot()).stateHash, confirmedHash, "closing spell selection does not undo confirmation");
      await load(await save(), confirmedHash);
      await abilitiesPage();
      assert.equal(await text("#spell-realms-value"), localization.format("ability-realms-value", { first:localization.format("realm-death-name"), second:localization.format("realm-life-name") }));
      assert.equal(await driver.execute('return [...document.querySelectorAll(".ability-realm-badge")].filter(node => node.textContent.includes(arguments[0])).length', [localization.format("realm-life-name")]), 4);
      await screenshot(`${locale}-changed-and-restored`);
      report.locales.push({ locale, builds:tested, birthLevel:1, preparedLevel:20, checks:["ordered realm choices", "keyboard and focus", "draft cancellation", "race/class changes", "390px and 200%", "creation busy", "both realms study", "restudy busy", "forgotten and recovered", "confirmation Escape", "pending save", "confirmation busy", "confirmed change persists without studying", "changed realm UI after load"], finalHash:confirmedHash });
      process.stdout.write(`Mage ${locale}: 56 choices, keyboard, study/change/save and responsive UI passed.\n`);
    }
    assert.deepEqual(keyboard.errors, []);
    await writeFile(path.join(directory, "report.json"), JSON.stringify(report,null,2) + "\n");
  } finally { keyboard.close(); }
}
