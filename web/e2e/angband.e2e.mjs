// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";

const directions = {"-1,1":["1","south-west"],"0,1":["2","south"],"1,1":["3","south-east"],"-1,0":["4","west"],"1,0":["6","east"],"-1,-1":["7","north-west"],"0,-1":["8","north"],"1,-1":["9","north-east"]};
const depth = state => state.dungeon?.currentDepth ?? 0;
export async function runAngbandScenario(context) {
  const {driver,keyboard,report,invoke,snapshot,click,changed,closeDialogs,reloadPrepared,capture,nativeSaveRoundTrip,directory} = context;
  report.fixture = "Normal Human Warrior birth at level 1. WebDriver-only preparation places the player at the generated (57,40) entrance, grants level-51 XP capped to 50 before victory, a generated broad sword and Homeward scroll, long flight/see-invisible/invulnerability, +2000 HP and +1000 melee skill/damage plus status immunities. Subsequent preparation moves/refills the player, lights maps and clears non-target actors. All ten random targets, Oberon and the Serpent keep source HP/defenses/energy/AI; no quest, victory, death or reward state is fabricated. Stairs and attacks use native input and production commands. This is prepared progression acceptance, not natural leveling or difficulty acceptance.";
  let state = await snapshot();
  function verifyVictoryProgress() {
    assert.ok(state.player.progress.victoryLevelCapUnlocked);
    assert.ok(state.player.progress.levelCap>50);
    assert.ok(state.player.progress.attributeIndexCap>37);
    if(depth(state)===100) {
      for(const kind of ["demo.item.grond","demo.item.crown-of-chaos"]) assert.ok(state.groundItems.some(item=>item.kindId===kind));
    }
  }
  async function prepare(phase,targetId=null,display=true) {
    const before = await snapshot();
    await invoke("prepare_angband_e2e",{phase,targetId});
    state = display ? await reloadPrepared() : await snapshot();
    report.checks.push({type:"preparation",phase,targetId,before:before.stateHash,after:state.stateHash,depth:depth(state),player:state.player.position,hp:state.player.hp});
    return state;
  }
  async function continueVictory() {
    await driver.execute('const button=document.querySelector("#result-continue");if(button?.checkVisibility())button.click();return true;');
    await closeDialogs();
  }
  async function checkpoint(name,native=true) {
    await continueVictory();
    if(native && state.campaign.status!=="retired") await nativeSaveRoundTrip(name);
    state=await snapshot();
    const bytes=await invoke("save_game",{savedAt:"2026-09-13T12:00:00Z"});
    await writeFile(path.join(directory,`${name}.rfbsave`),Buffer.from(bytes));
    await writeFile(path.join(directory,`${name}.json`),JSON.stringify({hash:state.stateHash,contentHash:state.contentHash,protocolVersion:state.protocolVersion},null,2)+"\n");
    report.checks.push({type:"checkpoint",name,hash:state.stateHash,depth:depth(state),campaign:state.campaign,tasks:state.tasks});
    await writeFile(path.join(directory,"progress.json"),JSON.stringify(report,null,2)+"\n");
  }
  const resume=process.argv.find(arg=>arg.startsWith("--angband-resume="))?.split("=")[1];
  if(resume) {
    assert.match(resume,/^[a-z0-9-]+$/);
    const previous=JSON.parse(await readFile(path.join(directory,"progress.json"),"utf8"));
    assert.equal(previous.contentHash,report.contentHash);
    report.checks=previous.checks;report.screenshots=previous.screenshots;report.retreated=previous.retreated;
    const saved=JSON.parse(await readFile(path.join(directory,`${resume}.json`),"utf8"));
    assert.equal(saved.contentHash,report.contentHash);assert.equal(saved.protocolVersion,report.protocolVersion);
    report.resume={checkpoint:resume,hash:saved.hash,contentHash:saved.contentHash};
    const bytes=await readFile(path.join(directory,`${resume}.rfbsave`));
    await keyboard.evaluate(`(()=>{const files=new DataTransfer();files.items.add(new File([Uint8Array.from(atob(${JSON.stringify(bytes.toString("base64"))}),byte=>byte.charCodeAt(0))],"angband-checkpoint.rfbsave"));const input=document.querySelector("#load-input");input.files=files.files;input.dispatchEvent(new Event("change",{bubbles:true}));return true;})()`);
    await driver.waitFor('return document.querySelector("#hash-value").title===arguments[0] && document.querySelector("#connection-status").classList.contains("ready")',"Angband resume hash",60_000,[saved.hash]);
    state=await snapshot();await continueVictory();
    if(state.campaign.status==="victorious") {
      verifyVictoryProgress();
      if(depth(state)===100) await checkpoint("victory",false);
    }
  } else {
    assert.equal(state.player.progress.level,1);
    await prepare("arrival");
    if(state.player.pendingRaceMutationChoice) {
      const index=state.player.pendingRaceMutationChoice.candidates.findIndex(row=>row.id==="rfb.mutation.sacred-vitality");
      assert.ok(index>=0);await click("#player-ui-character-open");await click("#character-tab-other");
      await driver.execute('document.querySelectorAll(".mutation-choice-candidate")[arguments[0]].focus();return true;',[index]);
      await keyboard.key("Enter");await changed(state.stateHash,"human talent");await closeDialogs();state=await snapshot();
    }
    await capture("formal-entrance");await checkpoint("entrance");
  }
  async function stairs(up=false) {
    const native=up || [0,1,99,100,101,126,127].includes(depth(state)) || state.tasks.some(task=>task.depth===depth(state));
    if(depth(state)) await prepare(up ? "stairs-up" : "stairs-down",null,native);
    const from=state.floorId;
    if(native) {
      await continueVictory();await keyboard.key(">");await changed(state.stateHash,"Angband native stairs");state=await snapshot();
    } else {
      const update=await invoke("dispatch_game_command",{commandSeq:state.lastCommandSeq+1,expectedRevision:state.revision,command:{type:"traverse-stairs"}});
      state=await snapshot();
      report.checks.push({type:"stairs-events",from,events:update.events});
      if([99,100,101,127].includes(depth(state)) || state.tasks.some(task=>task.status==="active" && task.depth===depth(state))) state=await reloadPrepared();
    }
    assert.notEqual(state.floorId,from);report.checks.push({type:native?"native-stairs":"production-stairs",from,to:state.floorId,hash:state.stateHash});
  }
  async function battle(task) {
    await prepare("route");
    assert.equal(state.activeActors.length,1,"unique quest target retained by route preparation");
    const source=state.activeActors[0];
    assert.ok(source,`source target for ${task.taskId}`);
    assert.equal(source.hp,source.maxHp,"quest combat starts at full source HP");
    const combat={type:"combat",taskId:task.taskId,source,trace:[]};report.checks.push(combat);
    while(combat.trace.length<512) {
      if(!state.activeActors.some(actor=>actor.id===source.id)) break;
      await prepare("battle",source.id,combat.trace.length===0);
      if(combat.trace.length===0) await closeDialogs();
      const actor=state.activeActors.find(actor=>actor.id===source.id);
      const [key,direction]=directions[`${actor.position.x-state.player.position.x},${actor.position.y-state.player.position.y}`];
      // One native attack proves the UI input path. Remaining strikes retain exact events.
      if(combat.trace.length===0) {
        await keyboard.key(key);await changed(state.stateHash,"Angband native melee");state=await snapshot();
        combat.trace.push({input:"native",key,hp:state.activeActors.find(row=>row.id===source.id)?.hp ?? 0,hash:state.stateHash});
      } else {
        const update=await invoke("dispatch_game_command",{commandSeq:state.lastCommandSeq+1,expectedRevision:state.revision,command:{type:"move",direction}});
        combat.trace.push({input:"production-melee",events:update.events,hash:update.stateHash});
        state=await snapshot();
        if(combat.trace.length%16===0 || !state.activeActors.some(row=>row.id===source.id)) state=await reloadPrepared();
      }
      assert.ok(!state.player.isDead,`prepared player survives ${task.taskId}`);
    }
    assert.ok(!state.activeActors.some(actor=>actor.id===source.id));
    assert.equal(state.tasks.find(row=>row.taskId===task.taskId).status,"completed");combat.completed=true;
    process.stdout.write(`Angband depth ${depth(state)}: ${task.taskId} defeated in ${combat.trace.length} attacks.\n`);
    await continueVictory();await capture(`defeated-${depth(state)}`);await checkpoint(`completed-${depth(state)}`);
  }
  if(state.campaign.status==="retired") return;
  if(depth(state)===0 && state.campaign.status==="active") await stairs();
  while(depth(state)>0 && depth(state)<127) {
    const task=state.tasks.find(task=>task.status==="active" && task.depth===depth(state));
    if(task) {
      assert.ok(!state.cells.some(cell=>cell.terrainId==="demo.terrain.stairs-down"),"unfinished quest blocks descending");
      await checkpoint(depth(state)===99?"oberon-before":depth(state)===100?"serpent-before":`random-${depth(state)}-before`);
      if(!report.retreated && task.canAbandon) {
        await stairs(true);assert.equal(state.tasks.find(row=>row.taskId===task.taskId).status,"paused");
        report.retreated=true;await checkpoint("random-paused");
        await stairs();assert.equal(state.tasks.find(row=>row.taskId===task.taskId).status,"active");await checkpoint("random-resumed");
      }
      await battle(task);
      if(depth(state)===99) assert.equal(state.campaign.status,"active");
      if(depth(state)===100) {
        assert.equal(state.campaign.status,"victorious");verifyVictoryProgress();
        await checkpoint("victory");
      }
    }
    await stairs();
    if(depth(state)===101) {assert.equal(state.campaign.status,"victorious");await capture("post-victory-101");await checkpoint("depth-101");}
  }
  if(depth(state)===127) {
    await prepare("route");await capture("depth-127");await checkpoint("depth-127");
    await click("#player-ui-inventory-open");
    await driver.execute('for(const input of document.querySelectorAll("#inventory-list input:checked"))input.click();document.querySelector(\'#inventory-list [data-item-id="e2e.angband.recall"] input\').click();return true;');
    await click("#inventory-use");await changed(state.stateHash,"postgame recall");state=await snapshot();await closeDialogs();
    for(let i=0;depth(state)>0 && i<40;i++) {
      await prepare("route",null,i===0);
      if(i===0) {
        await continueVictory();await keyboard.key("r");await changed(state.stateHash,"recall rest");state=await snapshot();
        report.checks.push({type:"native-recall-rest",hash:state.stateHash});
      } else {
        const update=await invoke("dispatch_game_command",{commandSeq:state.lastCommandSeq+1,expectedRevision:state.revision,command:{type:"rest",turns:100}});
        report.checks.push({type:"recall-rest-events",events:update.events,hash:update.stateHash});
        state=await snapshot();
      }
    }
    state=await reloadPrepared();
  }
  assert.equal(depth(state),0);assert.deepEqual(state.wildernessPosition,{x:57,y:40});assert.ok(state.campaign.canRetire);
  await checkpoint("returned");
  await driver.execute('window.__angbandConfirm=window.confirm;window.confirm=()=>false;document.querySelector("#campaign-retire").click();return true;');
  assert.equal((await snapshot()).campaign.status,"victorious");
  await driver.execute('window.confirm=()=>true;document.querySelector("#campaign-retire").click();return true;');
  await changed(state.stateHash,"retirement");state=await snapshot();assert.equal(state.campaign.status,"retired");
  await capture("retired");await checkpoint("retired",false);
  await driver.execute('window.confirm=window.__angbandConfirm;return true;');
}
