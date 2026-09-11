// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { Localization } from "../src/localization.ts";
import { selectCreationBuild, selectCreationRace } from "./character-creation.e2e.mjs";
import { connectKeyboard } from "./character-creation-layout.e2e.mjs";

export async function runRangerUiScenario(driver, directory, profile) {
  await mkdir(directory, { recursive: true });
  await driver.waitFor('return document.documentElement.dataset.appMode === "title"', "Ranger title", 60_000);
  const keyboard = await connectKeyboard(profile);
  const sources = Object.fromEntries(await Promise.all(["zh-CN", "en-US"].map(async locale => [locale,
    await Promise.all(["ui", "content", "game"].map(file => readFile(new URL(`../../locales/${locale}/${file}.ftl`, import.meta.url), "utf8"))),
  ])));
  const localization = new Localization("zh-CN", sources);
  const report = { preparation: "All four builds created through normal human level-1 UI in both locales. Explicit XP to levels 2, 3 and 15, quiet lit map and a third-realm book; real XP drain and recovery. No natural progression or acquisition claim.", locales: [] };
  const click = selector => driver.execute('document.querySelector(arguments[0]).click(); return true;', [selector]);
  const focus = selector => driver.execute('document.querySelector(arguments[0]).focus(); return true;', [selector]);
  const text = selector => driver.execute('return document.querySelector(arguments[0]).textContent;', [selector]);
  const focusIs = selector => driver.execute('return document.activeElement.matches(arguments[0]);', [selector]);
  const ready = () => driver.waitFor('return document.querySelector("#connection-status").classList.contains("ready")', "idle UI");
  async function invoke(command, args) {
    await driver.execute(`window.__rangerDone = false; window.__rangerError = null;
      window.__TAURI_INTERNALS__.invoke(arguments[0], arguments[1]).then(value => { window.__rangerResult = value; window.__rangerDone = true; }, error => window.__rangerError = String(error)); return true;`, [command, args]);
    await driver.waitFor('return window.__rangerDone || window.__rangerError', command, 30_000);
    assert.equal(await driver.execute('return window.__rangerError'), null);
    return driver.execute('return window.__rangerResult');
  }
  const snapshot = () => invoke("inspect_game_e2e");
  async function changed(before, label) {
    await driver.waitFor('return document.querySelector("#hash-value").title !== arguments[0]', label, 15_000, [before]);
    await ready(); return snapshot();
  }
  async function save() {
    await invoke("save_game", { savedAt: "2026-09-11T12:00:00Z" });
    return driver.execute('window.__rangerSaves ??= []; return window.__rangerSaves.push(window.__rangerResult) - 1;');
  }
  async function load(index, hash) {
    await driver.execute(`window.__rangerLoaded=false; const original=window.fetch, endpoint=window.__TAURI_INTERNALS__.convertFileSrc("load_game","ipc");
      window.fetch=(url,options)=>{
        if(url!==endpoint) return original(url,options);
        window.fetch=original; return original(url,options).then(value=>{setTimeout(()=>window.__rangerLoaded=true,0);return value;});
      };
      const transfer = new DataTransfer(); transfer.items.add(new File([new Uint8Array(window.__rangerSaves[arguments[0]])], "ranger-ui.rfbsave"));
      const input = document.querySelector("#load-input"); input.files = transfer.files; input.dispatchEvent(new Event("change", { bubbles:true })); return true;`, [index]);
    await driver.waitFor('return window.__rangerLoaded && document.querySelector("#hash-value").title === arguments[0] && document.querySelector("#connection-status").classList.contains("ready")', "native save import", 30_000, [hash]);
    return snapshot();
  }
  async function page() {
    await click(await driver.execute('return document.querySelector("#player-page-dialog").open') ? "#player-page-tab-ability" : "#player-ui-ability-open");
    await driver.waitFor('return document.querySelector("#ability-list").checkVisibility()', "book page");
  }
  async function talent() {
    const current = await snapshot();
    if (current.player.pendingRaceMutationChoice) {
      await click("#player-ui-character-open"); await click("#character-tab-other");
      await focus(".mutation-choice-candidate"); await keyboard.key("Enter");
      await changed(current.stateHash, "human talent"); await keyboard.key("Escape");
    }
  }
  async function prepare(level) {
    const prepared = await invoke("prepare_spell_learning_e2e", { level });
    await load(await save(), prepared.stateHash); await talent(); await page(); return snapshot();
  }
  async function viewport(width, height, zoom = 1) {
    await invoke("plugin:webview|set_webview_zoom", { label: "main", value: zoom });
    const rect = await driver.command("GET", "/window/rect");
    const current = await driver.execute('return { width:innerWidth, height:innerHeight }');
    await driver.command("POST", "/window/rect", { width:Math.round(rect.width+(width-current.width)*zoom), height:Math.round(rect.height+(height-current.height)*zoom) });
    await driver.waitFor('return Math.abs(innerWidth-arguments[0])<=1 && Math.abs(innerHeight-arguments[1])<=1', "viewport", 10_000, [width,height]);
  }
  async function layout(selector) {
    const result = await driver.execute(`const n = document.querySelector(arguments[0]), r=n.getBoundingClientRect();
      return n.checkVisibility() && n.scrollWidth<=n.clientWidth+1 && r.left>=-1 && r.right<=innerWidth+1 && r.top>=-1 && r.bottom<=innerHeight+1;`, [selector]);
    assert.equal(result,true,`${selector} visible without overflow`);
  }
  async function screenshot(name) {
    await driver.execute('window.__rangerPainted=false; requestAnimationFrame(()=>requestAnimationFrame(()=>window.__rangerPainted=true)); return true;');
    await driver.waitFor('return window.__rangerPainted', "paint");
    await writeFile(path.join(directory,`${name}.png`),await driver.screenshot(),"base64");
  }
  async function hold(command) {
    await driver.execute(`const original=window.fetch, endpoint=window.__TAURI_INTERNALS__.convertFileSrc(arguments[0],"ipc"); window.__rangerDispatches=0; window.__rangerRelease=null;
      window.fetch=(url,options)=>url!==endpoint ? original(url,options) : new Promise((resolve,reject)=>{
        window.__rangerDispatches++; window.__rangerRelease=()=>{window.fetch=original; original(url,options).then(resolve,reject);};
      }); return true;`,[command]);
  }
  async function release() {
    assert.equal(await driver.execute('return window.__rangerDispatches'),1);
    await driver.execute('window.__rangerRelease(); return true;');
  }
  const bookRow = id => `.ability-book-heading[data-book-item-id="${id}"]`;
  async function study(realm, busy = false) {
    const before = await snapshot();
    const candidate = before.player.abilities.find(a=>a.bookRealmId===realm && a.canStudy);
    assert.ok(candidate, `${realm} has a core-approved learning candidate`);
    const selector = `${bookRow(candidate.bookItemId)} button`;
    assert.equal(await text(selector),localization.format("action-ability-study-random"));
    if (busy) await hold("dispatch_game_command");
    await focus(selector); await keyboard.key("Enter");
    if(busy) {
      await driver.waitFor('return !!window.__rangerRelease',"held study");
      assert.equal(await driver.execute('return [...document.querySelectorAll("#ability-list button")].every(n=>n.matches(":disabled"))'),true);
      await keyboard.key("Enter"); await click(selector); await release();
    }
    const after = await changed(before.stateHash,"random study");
    const learned = after.player.abilities.filter(a=>a.learned && !before.player.abilities.find(b=>b.id===a.id)?.learned);
    assert.equal(learned.length,1); assert.equal(learned[0].bookRealmId,realm);
    assert.equal(after.player.abilityLearning.remainingSlots,before.player.abilityLearning.remainingSlots-1);
    assert.equal(await text("#ability-panel .ability-learning-summary"),localization.format("ability-learning-value",{learned:after.player.abilityLearning.learnedCount,capacity:after.player.abilityLearning.capacity,remaining:after.player.abilityLearning.remainingSlots}));
    assert.equal(await focusIs(after.player.abilities.some(a=>a.bookItemId===candidate.bookItemId && a.canStudy) ? selector : bookRow(candidate.bookItemId)),true,"random study focus survives rendering");
    return { state:after, learned:learned[0] };
  }
  try {
    await invoke("plugin:window|set_min_size",{label:"main",value:null});
    for(const locale of ["zh-CN","en-US"]) {
      localization.setLocale(locale);
      const births=[];
      for(const realm of ["death","arcane","daemon","sorcery"]) {
        await driver.execute('localStorage.setItem("rfb.locale",arguments[0]); return true;',[locale]);
        await keyboard.reload();
        await driver.waitFor('return document.documentElement.dataset.appMode==="title" && !document.querySelector("#session-new-game").disabled',"localized title",60_000);
        await viewport(1280,720); await click("#session-new-game");
        await driver.execute('document.querySelector("#session-character-name").value="Ranger UI"; document.querySelector("#session-seed").value="925"; return true;');
        await selectCreationRace(driver,"demo.race.rfb-human");
        if(realm==="sorcery") {
          await selectCreationBuild(driver,"demo.build.warrior");
          const confirmed=await text("#session-creation-summary");
          await click('[data-career-group="archery"]'); await focus('[data-career-id="ranger"]'); await keyboard.key("Enter");
          assert.equal(await driver.execute('return document.querySelector("#session-start-game").disabled'),true);
          await driver.execute('document.querySelector("#session-new-game-view").requestSubmit(); return true;');
          assert.equal(await driver.execute('return document.documentElement.dataset.appMode'),"new-game");
          await keyboard.key("Escape"); assert.equal(await focusIs('[data-career-id="ranger"]'),true);
          assert.equal(await text("#session-creation-summary"),confirmed);
          await keyboard.key("Enter"); await click("#session-tab-race");
          assert.equal(await text("#session-creation-summary"),confirmed);
          await selectCreationRace(driver,"rfb-legacy.race.tonberry");
          await selectCreationBuild(driver,"demo.build.duelist");
          assert.equal(await driver.execute('return document.querySelector("#session-start-game").disabled'),true);
          await selectCreationBuild(driver,"demo.build.ranger-nature-sorcery");
          assert.equal(await driver.execute('return document.querySelector("#session-start-game").disabled'),false);
          await selectCreationRace(driver,"demo.race.rfb-human");
          for(const [width,height,zoom] of [[390,844,1],[640,360,2]]) {
            await viewport(width,height,zoom); await selectCreationBuild(driver,"demo.build.ranger-nature-sorcery");
            await focus('[data-career-id="demo.build.ranger-nature-sorcery"]'); await keyboard.key("End");
            assert.equal(await focusIs('[data-career-id="demo.build.ranger-nature-daemon"]'),true);
            await keyboard.key(" "); await keyboard.key("Home"); await keyboard.key("Enter");
            await layout("#session-new-game-view"); await layout("#session-career-path"); await layout("#session-start-game");
            await screenshot(`${locale}-creation-${width}-${zoom}`);
          }
          await viewport(1280,720);
        }
        const build=`demo.build.ranger-nature-${realm}`;
        await selectCreationBuild(driver,build);
        assert.equal(await driver.execute('return document.querySelector("#session-career-options").children.length'),4);
        assert.ok((await text("#session-career-notes")).includes(localization.format("session-ranger-realms-help")));
        if(realm==="sorcery") await hold("initialize_game");
        await focus("#session-start-game"); await keyboard.key("Enter");
        if(realm==="sorcery") {
          await driver.waitFor('return !!window.__rangerRelease',"held creation");
          assert.equal(await driver.execute('return [...document.querySelectorAll("#session-new-game-view button,#session-new-game-view input")].every(n=>n.disabled)'),true);
          await keyboard.key("Escape"); await driver.execute('document.querySelector("#session-new-game-view").requestSubmit(); return true;'); await release();
        }
        await driver.waitFor('return document.documentElement.dataset.appMode==="playing" && document.querySelector("#connection-status").classList.contains("ready")',"Ranger birth",60_000);
        await talent(); await page();
        const born=await snapshot();
        assert.equal(born.player.build.buildId,build); assert.equal(born.player.progress.level,1);
        assert.equal(born.player.abilityLearning.realms.firstRealmId,"nature"); assert.equal(born.player.abilityLearning.realms.secondRealmId,realm);
        assert.equal(born.player.abilityLearning.studyMode,"divine-random"); assert.equal(born.player.abilityLearning.capacity,0);
        assert.equal(await text("#spell-realms-study-help"),localization.format("ability-random-study-help"));
        assert.equal(born.player.abilities.find(a=>a.id==="demo.ability.ranger-probe-monsters").canCast,false);
        assert.equal(await text(".ability-learning-start"),localization.format("ability-learning-start",{level:3}));
        await layout("#ability-panel .ability-learning-start");
        assert.equal(await text("#ability-panel .ability-learning-summary"),localization.format("ability-learning-value",{learned:0,capacity:0,remaining:0}));
        if(realm==="sorcery") await screenshot(`${locale}-level-one-books`);
        assert.equal(await driver.execute('return document.querySelectorAll("[data-ability-action=study]").length'),0);
        births.push({build,level:1,hash:born.stateHash});
      }
      let current=await prepare(2);
      assert.equal(current.player.abilityLearning.capacity,0);
      assert.equal(await text(".ability-learning-start"),localization.format("ability-learning-start",{level:3}));
      current=await prepare(3);
      assert.ok(current.player.abilityLearning.remainingSlots>0);
      assert.equal(await driver.execute('return !!document.querySelector(".ability-learning-start")'),false);
      const primary=await study("nature",true);
      assert.equal(primary.learned.proficiencyCap,1600);
      await screenshot(`${locale}-random-study-result`);
      current=await prepare(15);
      const secondary=await study("sorcery");
      assert.equal(secondary.learned.proficiencyCap,1400);
      assert.ok((await text(`[data-ability-id="${secondary.learned.id}"]`)).includes(localization.format("ability-status-learned")));
      const probe=secondary.state.player.abilities.find(a=>a.id==="demo.ability.ranger-probe-monsters");
      assert.equal(probe.canCast,true); assert.equal(probe.governingAttribute,"wisdom");
      current=await prepare(0);
      assert.equal(current.player.abilities.find(a=>a.id===primary.learned.id).forgotten,true);
      assert.ok((await text(`[data-ability-id="${primary.learned.id}"]`)).includes(localization.format("ability-status-forgotten")));
      await screenshot(`${locale}-forgotten`);
      current=await prepare(15);
      assert.equal(current.player.abilities.find(a=>a.id===primary.learned.id).learned,true);
      for(const [width,height,zoom] of [[390,844,1],[640,360,2]]) {
        await viewport(width,height,zoom); await focus("#spell-realms-value");
        await layout("#player-page-dialog"); await layout("#ability-panel"); await screenshot(`${locale}-books-${width}-${zoom}`);
      }
      const before=current;
      await focus('#realm-change-books [data-realm-id="death"]'); await keyboard.key("Enter");
      current=await changed(before.stateHash,"begin realm change");
      assert.equal(await focusIs("#realm-change-decline"),true);
      assert.equal(await text("#realm-change-description"),localization.format("realm-change-random-description",{old:localization.format("realm-sorcery-name"),first:localization.format("realm-nature-name"),next:localization.format("realm-death-name")}));
      const pending=current.stateHash, saved=await save();
      await layout("#realm-change-dialog"); await screenshot(`${locale}-change-200-percent`);
      await viewport(390,844); await layout("#realm-change-dialog"); await screenshot(`${locale}-change-390`);
      await keyboard.key("Escape"); current=await changed(pending,"cancel realm change");
      assert.equal(current.player.abilityLearning.realms.secondRealmId,"sorcery");
      assert.equal(current.player.abilityLearning.remainingSlots,before.player.abilityLearning.remainingSlots);
      await load(saved,pending); assert.equal(await focusIs("#realm-change-decline"),true);
      await hold("dispatch_game_command"); await keyboard.key("Tab"); assert.equal(await focusIs("#realm-change-accept"),true); await keyboard.key("Enter");
      await driver.waitFor('return !!window.__rangerRelease',"held confirmation");
      assert.equal(await driver.execute('return [...document.querySelectorAll("#realm-change-dialog button")].every(n=>n.disabled)'),true);
      await keyboard.key("Escape"); assert.equal(await driver.execute('return document.querySelector("#realm-change-dialog").open'),true); await release();
      current=await changed(pending,"confirm and immediately learn");
      assert.equal(current.player.abilityLearning.realms.secondRealmId,"death"); assert.deepEqual(current.player.abilityLearning.realms.previousRealmIds,["sorcery"]);
      assert.equal(current.player.abilityLearning.remainingSlots,before.player.abilityLearning.remainingSlots-1);
      assert.equal(current.player.abilities.filter(a=>a.bookRealmId==="death" && a.learned).length,1);
      assert.equal(current.player.abilities.find(a=>a.id===primary.learned.id).learned,true);
      assert.equal(current.player.abilities.some(a=>a.bookRealmId==="sorcery"),false);
      assert.equal(await focusIs("#spell-realms-value"),true);
      const confirmed=current.stateHash;
      await keyboard.key("Escape"); assert.equal((await snapshot()).stateHash,confirmed);
      await load(await save(),confirmed); await page();
      assert.ok((await text("#spell-realms-history")).includes(localization.format("realm-sorcery-name")));
      await screenshot(`${locale}-changed-and-restored`);
      report.locales.push({locale,births,randomLearning:[primary.learned.id,secondary.learned.id],preparedLevels:[2,3,15,0,15],checks:["draft/race/class switching","native keyboard focus","creation/study/confirmation busy locks","low-level hint","random study and caps","forgetting/recovery","probe eligibility","390px and 200%","Escape cancellation","pending save","immediate learning after confirmation","changed UI after load"],finalHash:confirmed});
      process.stdout.write(`Ranger ${locale}: four births, random study, realm change and responsive UI passed.\n`);
    }
    assert.deepEqual(keyboard.errors,[]);
    await writeFile(path.join(directory,"report.json"),JSON.stringify(report,null,2)+"\n");
  } finally { keyboard.close(); }
}
