// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { selectCreationBuild, selectCreationRace } from "./character-creation.e2e.mjs";

export async function runOneRingScenario(driver, directory) {
  await mkdir(directory, { recursive: true });
  const click = selector => driver.execute('document.querySelector(arguments[0]).click(); return true;', [selector]);
  const hash = () => driver.execute('return document.querySelector("#hash-value").title');
  async function invoke(command, args = {}) {
    await driver.execute('window.__ringReply=undefined;window.__ringError=null;window.__TAURI_INTERNALS__.invoke(arguments[0],arguments[1]).then(value=>window.__ringReply=value,error=>window.__ringError=String(error));return true;', [command,args]);
    await driver.waitFor('return window.__ringReply!==undefined || window.__ringError', command, 30_000);
    assert.equal(await driver.execute('return window.__ringError'), null);
    return driver.execute('return window.__ringReply');
  }
  const snapshot = () => invoke("inspect_game_e2e");
  const save = () => invoke("save_game", { savedAt: "2026-09-12T12:00:00Z" });
  async function load(bytes, expectedHash) {
    // The native WebDriver limits request bodies; transfer the actual save in chunks.
    const encoded=Buffer.from(bytes).toString("base64");
    await driver.execute('window.__ringUpload="";return true;');
    for(let offset=0;offset<encoded.length;offset+=524288) await driver.execute('window.__ringUpload+=arguments[0];return true;', [encoded.slice(offset,offset+524288)]);
    await driver.execute('const files=new DataTransfer();files.items.add(new File([Uint8Array.from(atob(window.__ringUpload),c=>c.charCodeAt(0))],"one-ring.rfbsave"));const input=document.querySelector("#load-input");input.files=files.files;input.dispatchEvent(new Event("change",{bubbles:true}));return true;');
    await driver.waitFor('return document.querySelector("#connection-status").classList.contains("ready") && document.querySelector("#hash-value").title===arguments[0]', "saved ring restored", 30_000, [expectedHash]);
  }
  async function changed(before) {
    await driver.waitFor('return document.querySelector("#hash-value").title!==arguments[0] && document.querySelector("#connection-status").classList.contains("ready")', "ring action committed", 15_000, [before]);
  }
  await driver.waitFor('return document.documentElement.dataset.appMode==="title"', "title", 60_000);
  await driver.execute('window.__ringReload=true;localStorage.setItem("rfb.locale","zh-CN");setTimeout(()=>location.reload(),50);return true;');
  await driver.waitFor('return !window.__ringReload && document.documentElement.lang==="zh-CN" && document.documentElement.dataset.appMode==="title"', "Chinese title", 60_000);
  await click("#session-new-game");
  await selectCreationRace(driver, "demo.race.rfb-human");
  await selectCreationBuild(driver, "demo.build.warrior");
  await driver.execute('for(const [id,value] of [["session-seed","511"],["session-character-name","C5b读取验收"]]) { const input=document.getElementById(id);input.value=value;input.dispatchEvent(new Event("input",{bubbles:true})); } return true;');
  await click("#session-start-game");
  await driver.waitFor('return document.documentElement.dataset.appMode==="playing" && document.querySelector("#connection-status").classList.contains("ready")', "fresh warrior", 60_000);
  const input = path.join(directory, "new-game.rfbsave");
  await writeFile(input, Buffer.from(await save()));
  const prepared = await promisify(execFile)("cargo", ["test","-p","rfb-core","--lib","game::tests::ring_of_power::export_c5b_desktop_save","--","--ignored","--exact"], { cwd:path.resolve(directory,"../.."), env:{...process.env,C5B_DESKTOP_INPUT:input}, windowsHide:true, timeout:180_000 });
  await writeFile(path.join(directory,"preparation.log"), prepared.stdout+prepared.stderr);
  const bytes = await readFile(path.join(directory,"prepared.rfbsave"));
  await load(bytes, await readFile(path.join(directory,"prepared.hash"),"utf8"));
  const initial = await snapshot();
  await driver.execute('window.__ringErrors=[];window.addEventListener("error",e=>window.__ringErrors.push(e.message));return true;');
  const id = initial.inventory.find(item=>item.kindId==="demo.item.one-ring").id;
  await click("#player-ui-inventory-open");
  async function openRead() {
    await click("#inventory-more");await click("#inventory-read");
    await driver.waitFor('return !!document.querySelector(".item-target-dialog[open] select")', "readable item selector");
    assert.deepEqual(await driver.execute('return [...document.querySelector(".item-target-dialog[open] select").options].map(option=>option.value)'), [id]);
  }
  await openRead();const beforeCancel=await hash();
  await writeFile(path.join(directory,"selector.png"),await driver.screenshot(),"base64");
  await click('.item-target-dialog[open] button[type="button"]');
  assert.equal(await hash(), beforeCancel);
  async function read() {
    await openRead();const before=await hash();await click('.item-target-dialog[open] button[type="submit"]');await changed(before);
    assert.equal(await driver.execute('return !!document.querySelector(".item-target-dialog[open]")'), false);
    const message=await driver.execute('return document.querySelector("#message-list").textContent');
    for(const line of ["至尊戒，驭众戒", "至尊戒，寻众戒", "至尊戒，引众戒", "禁锢众戒黑暗中"]) assert.ok(message.includes(line), line);
    assert.equal(await driver.execute('return getComputedStyle(document.querySelector(".message-item-one-ring-inscription-read span:last-child")).whiteSpace'),"pre-line","source inscription retains four visible lines");
    return snapshot();
  }
  const inventoryRead=await read();
  assert.deepEqual(inventoryRead.inventory,initial.inventory,"reading keeps the depleted, unexamined item");
  const checkpoint=await save();const checkpointHash=await hash();
  const continued=await read();await load(checkpoint,checkpointHash);
  const replayed=await read();assert.equal(replayed.stateHash,continued.stateHash);
  await driver.execute('document.querySelector("#inventory-list input[type=checkbox]").click();return true;');
  const beforeDrop=await hash();await click("#inventory-drop");await changed(beforeDrop);
  const ground=await read();assert.equal(ground.items.find(item=>item.id===id).readable,true);
  assert.equal(ground.inventory.some(item=>item.id===id),false,"floor reading does not pick up the ring");
  await click("#player-page-close");
  await driver.execute('document.querySelector("#message-list").lastElementChild.scrollIntoView({block:"end"});return true;');
  await writeFile(path.join(directory,"reading.png"),await driver.screenshot(),"base64");
  assert.deepEqual(await driver.execute('return window.__ringErrors'),[]);
  await writeFile(path.join(directory,"report.json"),JSON.stringify({preparation:"Fresh level-one human Warrior; chosen birth talent, cleared monsters, moved to a prepared floor cell and granted a source-defined One Ring with zero charges and recovery progress123. No ordinary acquisition or natural leveling claim.",checks:["cancel preserves state","depleted inventory reading prints four Chinese lines and preserves item","native save/load and next read have identical state hash","floor reading leaves item on the ground","no frontend errors"],initialHash:initial.stateHash,continuedHash:continued.stateHash,finalHash:ground.stateHash},null,2)+"\n");
}
