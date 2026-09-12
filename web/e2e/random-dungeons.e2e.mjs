// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { selectCreationBuild, selectCreationRace } from "./character-creation.e2e.mjs";

const kinds = ["forest", "volcano", "mountain", "sea"];
const phases = ["arrival", "entered", "ascended", "reentered", "recall-pending", "returned"];
const bounds = {forest:[25,50],volcano:[50,90],mountain:[40,70],sea:[55,75]};
const names = {forest:"随机森林",volcano:"随机火山",mountain:"随机山脉",sea:"随机海洋"};

export async function runRandomDungeonsScenario(context) {
  const {driver,keyboard,report,invoke,snapshot,click,changed,closeDialogs,reloadPrepared,capture,nativeSaveRoundTrip,directory} = context;
  report.fixture = "Fresh level-one Human Warrior, birth seed 42. WebDriver preparation selects an eligible source wilderness site and a deterministic source encounter draw, generates the surface once, places the player on the resulting entrance, grants one Homeward scroll and long levitation/invulnerability, clears actors/carried items and reveals/lights the map. Dungeon entry uses native stairs and the production RNG/depth/generation path. Route preparation clears actors; stairs preparation places the player on an existing up stair. This verifies generated entry/travel/save behavior, not natural encounter frequency, walking, combat difficulty or item acquisition.";
  const resume = process.argv.find(arg => arg.startsWith("--random-resume="))?.split("=")[1];
  const [resumeKind,resumePhase] = resume?.split(":") ?? [];
  if(resume) {
    assert.ok(kinds.includes(resumeKind) && phases.includes(resumePhase), "resume must be kind:phase");
    const previous=JSON.parse(await readFile(path.join(directory,"progress.json"),"utf8"));
    assert.equal(previous.contentHash,report.contentHash,"progress must belong to this content");
    assert.equal(previous.protocolVersion,report.protocolVersion);
    report.checks=[...previous.checks,...report.checks];
    report.screenshots=previous.screenshots;
  }
  for(const kind of kinds) {
    if(resume && kinds.indexOf(kind)<kinds.indexOf(resumeKind)) continue;
    let state, departure, firstInstance, start = -1;
    async function prepare(phase) {
      const before=await snapshot();
      await invoke("prepare_random_dungeon_e2e",{kind,phase});
      state=await reloadPrepared();
      report.checks.push({type:"preparation",kind,phase,beforeHash:before.stateHash,afterHash:state.stateHash,
        wildernessSeed:state.wildernessSeed,wildernessPosition:state.wildernessPosition,position:state.player.position,
        removedActors:before.activeActors,inventory:state.inventory,statuses:state.player.statuses});
    }
    async function checkpoint(phase) {
      state=await snapshot();
      const bytes=await invoke("save_game",{savedAt:"2026-09-13T00:00:00Z"});
      await writeFile(path.join(directory,`${kind}.${phase}.rfbsave`),Buffer.from(bytes));
      await writeFile(path.join(directory,`${kind}.${phase}.json`),JSON.stringify({kind,phase,departure,firstInstance,
        hash:state.stateHash,contentHash:state.contentHash,protocolVersion:state.protocolVersion},null,2)+"\n");
      report.checks.push({type:"checkpoint",kind,phase,hash:state.stateHash,floor:state.floorId,instance:state.dungeonInstanceId});
      await writeFile(path.join(directory,"progress.json"),JSON.stringify(report,null,2)+"\n");
    }
    async function stairs() {
      await closeDialogs();state=await snapshot();
      await keyboard.key(">");await changed(state.stateHash,`${kind} stairs`);state=await snapshot();
    }
    async function entered(name) {
      assert.ok(state.floorId.startsWith(`demo.floor.random-${kind}-depth-`));
      const depth=Number(state.floorId.split("-").at(-1));
      assert.ok(depth>=bounds[kind][0] && depth<=bounds[kind][1]);
      assert.ok(state.dungeonInstanceId);
      const location=await driver.execute('return document.querySelector("#hud-location-value").textContent');
      assert.ok(location.includes(String(depth)) && !location.includes("$depth"));
      report.checks.push({type:"generated-entry",kind,depth,location,instance:state.dungeonInstanceId,actors:state.activeActors.length});
      await prepare("route");await capture(`${kind}-${name}`);await nativeSaveRoundTrip(`${kind}-${name}`);
      state=await snapshot();
    }
    function returned() {
      assert.equal(state.floorId,"core.floor.wilderness");
      assert.deepEqual(state.wildernessPosition,departure.world);
      assert.deepEqual(state.player.position,departure.local);
      assert.equal(state.dungeonInstanceId,null);
    }
    if(resume && kind===resumeKind) {
      const saved=JSON.parse(await readFile(path.join(directory,`${kind}.${resumePhase}.json`),"utf8"));
      const bytes=await readFile(path.join(directory,`${kind}.${resumePhase}.rfbsave`));
      assert.equal(saved.contentHash,report.contentHash,"checkpoint content must match this build");
      assert.equal(saved.protocolVersion,report.protocolVersion);
      // Use the existing CDP connection: numeric byte arrays exceed WebDriver's body limit.
      await keyboard.evaluate(`(()=>{const files=new DataTransfer();
        const bytes=Uint8Array.from(atob(${JSON.stringify(bytes.toString("base64"))}),byte=>byte.charCodeAt(0));
        files.items.add(new File([bytes],"random-checkpoint.rfbsave"));
        const input=document.querySelector("#load-input");input.files=files.files;input.dispatchEvent(new Event("change",{bubbles:true}));return true;})()`);
      await driver.waitFor('return document.querySelector("#hash-value").title===arguments[0] && document.querySelector("#connection-status").classList.contains("ready")',"random checkpoint restored",30_000,[saved.hash]);
      state=await snapshot();assert.equal(state.stateHash,saved.hash);
      ({departure,firstInstance}=saved);start=phases.indexOf(resumePhase);
      report.checks.push({type:"resume",...saved});
    } else {
      if(kind!=="forest") {
        await click("#session-new-game");await selectCreationRace(driver,"demo.race.rfb-human");await selectCreationBuild(driver,"demo.build.warrior");
        await driver.execute('document.querySelector("#session-seed").value="42";document.querySelector("#session-seed").dispatchEvent(new Event("input",{bubbles:true}));return true;');
        await click("#session-start-game");
        await driver.waitFor('return document.documentElement.dataset.appMode==="playing" && document.querySelector("#connection-status").classList.contains("ready")',"random new character",60_000);
      }
      state=await snapshot();
      if(state.player.pendingRaceMutationChoice) {
        const index=state.player.pendingRaceMutationChoice.candidates.findIndex(row=>row.id==="rfb.mutation.sacred-vitality");assert.ok(index>=0);
        await click("#player-ui-character-open");await click("#character-tab-other");
        await driver.execute('document.querySelectorAll(".mutation-choice-candidate")[arguments[0]].focus();return true;',[index]);
        await keyboard.key("Enter");await changed(state.stateHash,"human talent");await closeDialogs();
      }
    }
    if(start<0) {
      await prepare("arrival");
      assert.ok(state.cells.some(cell=>cell.terrainId===`demo.terrain.random-${kind}-entrance`));
      departure={world:state.wildernessPosition,local:state.player.position};
      await click("#look-mode-toggle");
      await driver.waitFor('return document.querySelector("#target-mode-status").textContent.includes(arguments[0])',"source entrance name displayed",15_000,[names[kind]]);
      report.checks.push({type:"entrance-display",kind,text:await driver.execute('return document.querySelector("#target-mode-status").textContent')});
      await capture(`${kind}-entrance`);await click("#look-mode-toggle");
      await nativeSaveRoundTrip(`${kind}-entrance`);await checkpoint("arrival");
    }
    if(start<1) {
      await stairs();await entered("entered");firstInstance=state.dungeonInstanceId;await checkpoint("entered");
    }
    if(start<2) {
      await prepare("stairs");await stairs();returned();await capture(`${kind}-ascended`);await checkpoint("ascended");
    }
    if(start<3) {
      await stairs();assert.notEqual(state.dungeonInstanceId,firstInstance);await entered("reentered");await checkpoint("reentered");
    }
    if(start<4) {
      await closeDialogs();await click("#player-ui-inventory-open");
      await driver.execute('for(const input of document.querySelectorAll("#inventory-list input:checked")) input.click();document.querySelector(\'#inventory-list [data-item-id="e2e.random.recall"] input[type="checkbox"]\').click();return true;');
      state=await snapshot();await click("#inventory-use");await changed(state.stateHash,"Homeward recall");await closeDialogs();
      await nativeSaveRoundTrip(`${kind}-recall-pending`);await checkpoint("recall-pending");
    }
    if(start<5) {
      for(let attempt=0;attempt<40;attempt++) {
        state=await snapshot();if(state.floorId==="core.floor.wilderness") break;
        await prepare("route");await closeDialogs();await keyboard.key("r");await changed(state.stateHash,"rest through random recall");
      }
      state=await snapshot();returned();await capture(`${kind}-recall-returned`);
      await nativeSaveRoundTrip(`${kind}-returned`);await checkpoint("returned");
    }
  }
}
