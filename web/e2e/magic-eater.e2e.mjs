// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { Localization } from "../src/localization.ts";
import { selectCreationBuild, selectCreationRace } from "./character-creation.e2e.mjs";
import { connectKeyboard } from "./character-creation-layout.e2e.mjs";

// Written in step 6; execute after bindings, compilation and checks in step 7.
export async function runMagicEaterUiScenario(driver, directory, profile) {
  await mkdir(directory, { recursive: true });
  const keyboard = await connectKeyboard(profile);
  const sources = Object.fromEntries(await Promise.all(["zh-CN", "en-US"].map(async locale => [locale,
    await Promise.all(["ui", "content", "game"].map(file => readFile(new URL(`../../locales/${locale}/${file}.ftl`, import.meta.url), "utf8"))),
  ])));
  const localization = new Localization("zh-CN", sources);
  const report = { preparation: "Normal human level-1 birth and birth-wand absorption first. Then explicitly prepare level 25, a lit quiet floor, thirty generated/absorbed devices and one floor replacement. Full slots and high level are prepared, not naturally acquired. Commands, cancellation, export/load and continuation use the regular UI.", locales: [] };
  const click = selector => driver.execute('document.querySelector(arguments[0]).click(); return true;', [selector]);
  const focus = selector => driver.execute('document.querySelector(arguments[0]).focus(); return true;', [selector]);
  const text = selector => driver.execute('return document.querySelector(arguments[0]).textContent;', [selector]);
  const ready = () => driver.waitFor('return document.querySelector("#connection-status").classList.contains("ready")', "idle UI");
  async function invoke(command, args) {
    await driver.execute(`window.__meResult=null;window.__meDone=false;window.__meError=null;
      window.__TAURI_INTERNALS__.invoke(arguments[0],arguments[1]).then(value=>{window.__meResult=value;window.__meDone=true;},error=>window.__meError=String(error));return true;`, [command, args]);
    await driver.waitFor('return window.__meDone || window.__meError', command, 30_000);
    assert.equal(await driver.execute('return window.__meError'), null);
    return driver.execute('return window.__meResult');
  }
  const snapshot = () => invoke("inspect_game_e2e");
  async function changed(hash, label) {
    await driver.waitFor('return document.querySelector("#hash-value").title !== arguments[0]', label, 20_000, [hash]);
    await ready(); return snapshot();
  }
  async function viewport(width, height, zoom = 1) {
    await invoke("plugin:webview|set_webview_zoom", { label: "main", value: zoom });
    const rect = await driver.command("GET", "/window/rect"), current = await driver.execute('return {width:innerWidth,height:innerHeight}');
    await driver.command("POST", "/window/rect", { width: Math.round(rect.width + (width-current.width)*zoom), height: Math.round(rect.height+(height-current.height)*zoom) });
    await driver.waitFor('return Math.abs(innerWidth-arguments[0])<=1 && Math.abs(innerHeight-arguments[1])<=1', "viewport", 10_000, [width, height]);
  }
  async function capture(name, selector) {
    const layout = await driver.execute(`const n=document.querySelector(arguments[0]),r=n.getBoundingClientRect();return {visible:n.checkVisibility(),overflow:n.scrollWidth>n.clientWidth+1,left:r.left,right:r.right,top:r.top,bottom:r.bottom,width:innerWidth,height:innerHeight};`, [selector]);
    assert.equal(layout.visible, true); assert.equal(layout.overflow, false);
    assert.ok(layout.left>=-1 && layout.right<=layout.width+1 && layout.top>=-1 && layout.bottom<=layout.height+1, JSON.stringify(layout));
    await driver.execute('window.__mePainted=false;requestAnimationFrame(()=>requestAnimationFrame(()=>window.__mePainted=true));return true;');
    await driver.waitFor('return window.__mePainted', "paint");
    await writeFile(path.join(directory, `${name}.png`), await driver.screenshot(), "base64");
  }
  async function chooseTalent() {
    const state = await snapshot();
    if (!state.player.pendingRaceMutationChoice) return state;
    await click("#player-ui-character-open"); await click("#character-tab-other");
    await focus(".mutation-choice-candidate"); await keyboard.key("Enter");
    const next = await changed(state.stateHash, "human talent"); await keyboard.key("Escape"); return next;
  }
  async function open() {
    if (await driver.execute('return document.querySelector("#magic-eater-dialog").open')) return;
    await click("#player-ui-ability-open"); await click("#magic-eater-open");
    await driver.waitFor('return document.querySelector("#magic-eater-dialog").open', "device menu");
  }
  async function selectSource(id) {
    await click("#magic-eater-absorb");
    await driver.waitFor('return !!document.querySelector(".item-target-dialog[open]:not(#magic-eater-dialog) select")', "absorption item selector");
    await driver.execute('const s=document.querySelector(".item-target-dialog[open]:not(#magic-eater-dialog) select");s.value=arguments[0];s.dispatchEvent(new Event("change"));return true;', [id]);
    await focus('.item-target-dialog[open]:not(#magic-eater-dialog) button[type="submit"]'); await keyboard.key("Enter");
  }
  async function exportSave(name) {
    await driver.execute(`window.__meExport=null;window.__meHooks=[URL.createObjectURL,URL.revokeObjectURL,HTMLAnchorElement.prototype.click];
      URL.createObjectURL=blob=>{window.__meExport={blob};return "blob:magic-eater-e2e";};URL.revokeObjectURL=()=>{};
      HTMLAnchorElement.prototype.click=function(){window.__meExport.name=this.download;};return true;`);
    await click("#magic-eater-save");
    await driver.waitFor('return window.__meExport?.name?.endsWith(".rfbsave")', "normal export");
    await driver.execute(`[URL.createObjectURL,URL.revokeObjectURL,HTMLAnchorElement.prototype.click]=window.__meHooks;
      window.__meExportIndex=null;window.__meExport.blob.arrayBuffer().then(buffer=>{window.__meSaves??=[];window.__meExportIndex=window.__meSaves.push(Array.from(new Uint8Array(buffer)))-1;});return true;`);
    await driver.waitFor('return window.__meExportIndex!==null', "export bytes"); await ready();
    const index = await driver.execute('return window.__meExportIndex');
    await writeFile(path.join(directory, `${name}.rfbsave`), Buffer.from(await driver.execute('return window.__meSaves[arguments[0]]', [index])));
    return index;
  }
  async function load(index, hash) {
    await driver.execute(`window.__meLoaded=false;const original=window.fetch,url=window.__TAURI_INTERNALS__.convertFileSrc("load_game","ipc");
      window.fetch=(input,options)=>{if(input!==url)return original(input,options);window.fetch=original;return original(input,options).then(value=>{setTimeout(()=>window.__meLoaded=true,0);return value;});};
      const transfer=new DataTransfer();transfer.items.add(new File([new Uint8Array(window.__meSaves[arguments[0]])],"magic-eater.rfbsave"));
      const input=document.querySelector("#load-input");input.files=transfer.files;input.dispatchEvent(new Event("change",{bubbles:true}));return true;`, [index]);
    await driver.waitFor('return window.__meLoaded && document.querySelector("#hash-value").title===arguments[0]', "normal import", 30_000, [hash]);
    await ready(); return snapshot();
  }
  const slotButton = (slot, action) => `#magic-eater-slots [data-slot="${slot}"] [data-action="${action}"]`;
  async function inscribe(slot, inscription) {
    const before = await snapshot(); await click(slotButton(slot, "inscribe"));
    await driver.execute('document.querySelector(".item-target-dialog[open]:not(#magic-eater-dialog) input").value=arguments[0];return true;', [inscription]);
    await focus('.item-target-dialog[open]:not(#magic-eater-dialog) button[type="submit"]'); await keyboard.key("Enter");
    return changed(before.stateHash, "inscribe");
  }
  async function useRod() {
    await open(); await focus("#magic-eater-slots"); await keyboard.key("R");
    const before = await snapshot(); await focus(slotButton(0, "use")); await keyboard.key("Enter");
    const after = await changed(before.stateHash, "body rod use");
    return { after, events: await driver.execute('return window.__meUpdate.events') };
  }
  try {
    await invoke("plugin:window|set_min_size", { label: "main", value: null });
    for (const locale of ["zh-CN", "en-US"]) {
      localization.setLocale(locale);
      await driver.execute('localStorage.setItem("rfb.locale",arguments[0]);localStorage.setItem("rfb.input-preset","numpad");return true;', [locale]); await keyboard.reload();
      await driver.waitFor('return document.documentElement.dataset.appMode==="title"&&!document.querySelector("#session-new-game").disabled', "localized title", 60_000);
      await driver.execute(`const original=window.fetch,endpoint=window.__TAURI_INTERNALS__.convertFileSrc("dispatch_game_command","ipc");
        window.fetch=(url,options)=>url!==endpoint?original(url,options):original(url,options).then(async response=>{window.__meUpdate=await response.clone().json();return response;});return true;`);
      await viewport(1280, 720); await click("#session-new-game");
      await driver.execute('document.querySelector("#session-character-name").value="Magic-Eater UI";document.querySelector("#session-seed").value="925";return true;');
      await selectCreationRace(driver, "demo.race.rfb-human"); await selectCreationBuild(driver, "demo.build.magic-eater");
      assert.equal(await text('[data-career-group="device"]'), localization.format("session-career-category-device"));
      for (const [width, height, zoom] of [[390, 844, 1], [640, 360, 2]]) {
        await viewport(width, height, zoom); await capture(`${locale}-creation-${width}-${zoom}`, "#session-new-game-view");
      }
      await viewport(1280, 720); await focus("#session-start-game"); await keyboard.key("Enter");
      await driver.waitFor('return document.documentElement.dataset.appMode==="playing"&&document.querySelector("#connection-status").classList.contains("ready")', "normal birth", 60_000);
      const born = await chooseTalent(); assert.equal(born.player.progress.level, 1);
      assert.equal(born.player.build.buildId, "demo.build.magic-eater");
      assert.equal(born.player.magicEater.slots.length, 30); assert.equal(born.player.magicEater.slots.every(slot => !slot.item), true);
      const wand = born.inventory.find(item => item.kindId === "demo.item.magic-missile-wand"); assert.ok(wand);
      await open(); await selectSource(wand.id); let current = await changed(born.stateHash, "birth absorption selection");
      assert.equal(current.turn, born.turn);
      await focus(slotButton(0, "select")); await keyboard.key("Enter"); current = await changed(current.stateHash, "birth absorption committed");
      assert.equal(current.inventory.some(item => item.id === wand.id), false);
      assert.equal(current.player.magicEater.slots[0].item.id, wand.id);
      const bornAbsorbed = current.stateHash;
      await click(slotButton(0, "use")); await driver.waitFor('return document.querySelector("#map-host").dataset.targetingAction==="absorbed-device"', "body aiming");
      await keyboard.key("Escape");
      await driver.waitFor('return window.__meUpdate.events.some(event=>event.kind==="skill.device-success"||event.kind==="skill.device-failure")', "cancelled use still checks the device");
      await ready();
      const cancelled = await snapshot(), cancellationEvents = await driver.execute('return window.__meUpdate.events');
      assert.equal(cancelled.player.magicEater.slots[0].item.charges.current, current.player.magicEater.slots[0].item.charges.current);
      assert.equal(cancelled.turn, current.turn + (cancellationEvents.some(event => event.kind === "skill.device-failure") ? 1 : 0), "natural failure spends time; successful check and cancellation refunds it");

      const prepared = await invoke("prepare_magic_eater_e2e");
      const bytes = await invoke("save_game", { savedAt: "2026-09-12T12:00:00Z" });
      const preparedIndex = await driver.execute('window.__meSaves??=[];return window.__meSaves.push(arguments[0])-1;', [bytes]);
      current = await load(preparedIndex, prepared.stateHash); current = await chooseTalent();
      assert.equal(current.player.magicEater.slots.filter(slot => slot.item).length, 30);
      await open(); current = await inscribe(0, "@mq retained");
      await selectSource("e2e.magic-eater.replacement"); current = await changed(current.stateHash, "floor absorption selection");
      const pendingSlotSave = await exportSave(`${locale}-pending-slot`), pendingSlotHash = current.stateHash;
      await keyboard.key("Escape"); current = await changed(current.stateHash, "cancel slot selection");
      current = await load(pendingSlotSave, pendingSlotHash);
      assert.equal(await driver.execute('return document.querySelector("#magic-eater-dialog").open'), true);
      await click(slotButton(0, "select")); current = await changed(current.stateHash, "occupied slot prompt");
      const pendingSave = await exportSave(`${locale}-pending-replacement`), pendingHash = current.stateHash;
      for (const [width, height, zoom] of [[390, 844, 1], [640, 360, 2]]) {
        await viewport(width, height, zoom); await capture(`${locale}-replacement-${width}-${zoom}`, "#magic-eater-dialog");
      }
      await keyboard.key("Escape"); await changed(pendingHash, "cancel replacement");
      current = await load(pendingSave, pendingHash);
      assert.equal(await driver.execute('return document.activeElement.id'), "magic-eater-close");
      await click("#magic-eater-inherit");
      await driver.execute(`const original=window.fetch,url=window.__TAURI_INTERNALS__.convertFileSrc("dispatch_game_command","ipc");window.__meHeld=0;window.__meRelease=null;
        window.fetch=(input,options)=>input!==url?original(input,options):new Promise((resolve,reject)=>{window.__meHeld++;window.__meRelease=()=>{window.fetch=original;original(input,options).then(resolve,reject);};});return true;`);
      await click("#magic-eater-confirm"); await driver.waitFor('return !!window.__meRelease', "held replacement");
      assert.equal(await driver.execute('return [...document.querySelectorAll("#magic-eater-dialog button,#magic-eater-dialog select,#magic-eater-dialog input")].every(node=>node.disabled)'), true);
      await keyboard.key("Escape"); await click("#magic-eater-confirm"); assert.equal(await driver.execute('return window.__meHeld'), 1);
      await driver.execute('window.__meRelease();return true;'); current = await changed(pendingHash, "confirmed replacement");
      assert.equal(current.player.magicEater.slots[0].item.id, "e2e.magic-eater.replacement");
      assert.equal(current.player.magicEater.slots[0].item.inscription, "@mq retained");
      const beforeSwap = current; await focus("#magic-eater-slots"); await keyboard.key("X"); await keyboard.key("a"); await keyboard.key("b");
      current = await changed(beforeSwap.stateHash, "keyboard exchange");
      assert.equal(current.turn, beforeSwap.turn); assert.equal(current.player.magicEater.slots[1].item.id, "e2e.magic-eater.replacement");
      assert.equal(current.player.magicEater.slots[1].useLabel, "q");
      await click(slotButton(1, "inspect")); await capture(`${locale}-absorbed-detail`, "#inventory-detail-dialog"); await keyboard.key("Escape");
      await click("#magic-eater-close"); await click("#player-ui-settings-open");
      for (const selector of ["#travel-auto-detect", "#travel-auto-map", "#travel-disturb-detect"]) { const hash = (await snapshot()).stateHash; await click(selector); current = await changed(hash, "saved travel option"); }
      assert.deepEqual(current.travelOptions, { autoDetectTraps: true, autoMapArea: true, disturbTrapDetect: false });
      await capture(`${locale}-travel-options`, "#player-ui-settings-dialog"); await keyboard.key("Escape");
      await open();
      for (const key of ["W", "S", "R"]) {
        await focus("#magic-eater-slots"); await keyboard.key(key);
        assert.equal(await driver.execute('return document.querySelectorAll("#magic-eater-slots > li").length'), 10);
        await capture(`${locale}-slots-${key}`, "#magic-eater-dialog");
      }
      const saved = await snapshot(), savedIndex = await exportSave(`${locale}-absorbed`);
      const continued = await useRod(); await load(savedIndex, saved.stateHash);
      assert.deepEqual((await snapshot()).player.magicEater, saved.player.magicEater);
      assert.deepEqual((await snapshot()).travelOptions, saved.travelOptions);
      const replayed = await useRod(); assert.deepEqual(replayed, continued);
      report.locales.push({ locale, bornAbsorbed, pendingSlotHash, pendingHash, savedHash: saved.stateHash, continuedHash: continued.after.stateHash, events: continued.events });
      process.stdout.write(`Magic-Eater ${locale}: birth, slots, replacement, keys, settings and save UI passed.\n`);
    }
    assert.deepEqual(keyboard.errors, []);
    await writeFile(path.join(directory, "report.json"), `${JSON.stringify(report, null, 2)}\n`);
  } finally { keyboard.close(); }
}
