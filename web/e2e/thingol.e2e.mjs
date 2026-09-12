// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { selectCreationBuild, selectCreationRace } from "./character-creation.e2e.mjs";

export async function runThingolScenario(driver, directory) {
  await mkdir(directory, { recursive: true });
  const click = selector => driver.execute('document.querySelector(arguments[0]).click();return true;', [selector]);
  const hash = () => driver.execute('return document.querySelector("#hash-value").title');
  async function invoke(command, args = {}) {
    await driver.execute('window.__thingolReply=undefined;window.__thingolError=null;window.__TAURI_INTERNALS__.invoke(arguments[0],arguments[1]).then(v=>window.__thingolReply=v,e=>window.__thingolError=String(e));return true;', [command,args]);
    await driver.waitFor('return window.__thingolReply!==undefined || window.__thingolError',command,30_000);
    assert.equal(await driver.execute('return window.__thingolError'),null);
    return driver.execute('return window.__thingolReply');
  }
  const snapshot = () => invoke("inspect_game_e2e");
  const save = () => invoke("save_game", { savedAt: "2026-09-12T12:00:00Z" });
  async function load(bytes, expectedHash) {
    const encoded=Buffer.from(bytes).toString("base64");
    await driver.execute('window.__thingolUpload="";return true;');
    for(let offset=0;offset<encoded.length;offset+=524288) await driver.execute('window.__thingolUpload+=arguments[0];return true;', [encoded.slice(offset,offset+524288)]);
    await driver.execute('const files=new DataTransfer();files.items.add(new File([Uint8Array.from(atob(window.__thingolUpload),c=>c.charCodeAt(0))],"thingol.rfbsave"));const input=document.querySelector("#load-input");input.files=files.files;input.dispatchEvent(new Event("change",{bubbles:true}));return true;');
    await driver.waitFor('return document.querySelector("#connection-status").classList.contains("ready") && document.querySelector("#hash-value").title===arguments[0]',"Thingol save restored",30_000,[expectedHash]);
  }
  async function changed(before) {
    await driver.waitFor('return document.querySelector("#hash-value").title!==arguments[0] && document.querySelector("#connection-status").classList.contains("ready")',"Thingol action committed",15_000,[before]);
  }
  await driver.waitFor('return document.documentElement.dataset.appMode==="title"',"title",60_000);
  await driver.execute('window.__thingolReload=true;localStorage.setItem("rfb.locale","zh-CN");setTimeout(()=>location.reload(),50);return true;');
  await driver.waitFor('return !window.__thingolReload && document.documentElement.lang==="zh-CN" && document.documentElement.dataset.appMode==="title"',"Chinese title",60_000);
  await click("#session-new-game");
  await selectCreationRace(driver,"demo.race.rfb-human");
  await selectCreationBuild(driver,"demo.build.warrior");
  await driver.execute('for(const [id,value] of [["session-seed","511"],["session-character-name","辛葛充能验收"]]){const input=document.getElementById(id);input.value=value;input.dispatchEvent(new Event("input",{bubbles:true}));}return true;');
  await click("#session-start-game");
  await driver.waitFor('return document.documentElement.dataset.appMode==="playing" && document.querySelector("#connection-status").classList.contains("ready")',"fresh warrior",60_000);
  const input=path.join(directory,"new-game.rfbsave");
  await writeFile(input,Buffer.from(await save()));
  const preparation=await promisify(execFile)("cargo",["test","-p","rfb-core","--lib","game::tests::thingol::export_thingol_desktop_save","--","--ignored","--exact"],{cwd:path.resolve(directory,"../.."),env:{...process.env,THINGOL_DESKTOP_INPUT:input},windowsHide:true,timeout:180_000});
  await writeFile(path.join(directory,"preparation.log"),preparation.stdout+preparation.stderr);
  const bytes=await readFile(path.join(directory,"prepared.rfbsave"));
  const initialHash=await readFile(path.join(directory,"prepared.hash"),"utf8");
  await load(bytes,initialHash);
  const initial=await snapshot();
  const cloak=initial.equipment.find(item=>item.kindId==="demo.item.thingol");
  assert.equal(cloak.requiresRechargeTargets,true);
  await driver.execute('window.__thingolErrors=[];window.addEventListener("error",e=>window.__thingolErrors.push(e.message));return true;');
  await click("#player-ui-inventory-open");
  async function source() {
    await click(`[data-slot-id="${cloak.slotId}"] > button`);
    await click(".equipment-activate");
    await driver.waitFor('return !!document.querySelector(".item-target-dialog[open] select")',"recharge source");
    assert.deepEqual(await driver.execute('return [...document.querySelector(".item-target-dialog[open] select").options].map(o=>o.value)'),["donor"]);
  }
  async function target() {
    await source();
    await click('.item-target-dialog[open] button[type="submit"]');
    await driver.waitFor('return document.querySelector(".item-target-dialog[open] select")?.value==="target"',"recharge target");
    assert.deepEqual(await driver.execute('return [...document.querySelector(".item-target-dialog[open] select").options].map(o=>o.value)'),["target"]);
  }
  for(const stage of [source,target]) {
    await stage();const before=await hash();
    await writeFile(path.join(directory,`${stage.name}.png`),await driver.screenshot(),"base64");
    await click('.item-target-dialog[open] button[type="button"]');await changed(before);
    const cancelled=await snapshot();
    assert.equal(cancelled.equipment.find(item=>item.id===cloak.id).charges.current,1);
    assert.equal(cancelled.inventory.find(item=>item.id==="donor").charges.current,initial.inventory.find(item=>item.id==="donor").charges.current);
    assert.ok(!cancelled.items.find(item=>item.id==="target").canSupplyRecharge);
    await load(bytes,initialHash);
  }
  async function transfer() {
    await target();const before=await hash();
    await click('.item-target-dialog[open] button[type="submit"]');await changed(before);
    return snapshot();
  }
  const completed=await transfer();
  assert.equal(completed.equipment.find(item=>item.id===cloak.id).charges.current,0);
  assert.ok(completed.inventory.find(item=>item.id==="donor").charges.current < initial.inventory.find(item=>item.id==="donor").charges.current);
  assert.equal(completed.items.find(item=>item.id==="target").canSupplyRecharge,true);
  const cooldownBytes=await save();const cooldownHash=await hash();
  await load(bytes,initialHash);
  assert.equal((await transfer()).stateHash,completed.stateHash,"same next transfer after native load");
  await load(cooldownBytes,cooldownHash);
  assert.equal((await snapshot()).equipment.find(item=>item.id===cloak.id).usable,false);
  await click("#player-page-close");
  await writeFile(path.join(directory,"result.png"),await driver.screenshot(),"base64");
  assert.deepEqual(await driver.execute('return window.__thingolErrors'),[]);
  await writeFile(path.join(directory,"report.json"),JSON.stringify({preparation:"Fresh level-one human Warrior; chosen birth talent, cleared monsters, granted and equipped Thingol plus two real devices, target placed underfoot; selected a successful non-destroying seed. No ordinary acquisition or natural leveling claim.",checks:["two distinct source/target dialogs","cancel either stage spends time but preserves charges","equipped Thingol transfers pack device energy to floor device","cooldown survives native save/load","same next transfer and state hash after restoring native save","no frontend errors"],initialHash,completedHash:completed.stateHash,cooldownHash},null,2)+"\n");
}
