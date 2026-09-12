// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { selectCreationBuild, selectCreationRace } from "./character-creation.e2e.mjs";

const DUNGEON = "demo.dungeon.asgard";
const HEIMDALL = "demo.actor.heimdall-guardian-of-bifrost";
const ODIN = "demo.actor.odin-the-all-father";
const VIDARR = "demo.actor.vidarr-the-silent-avenger";
const protectedKinds = new Set([HEIMDALL, ODIN, VIDARR]);
const directions = {"-1,1":["1","south-west"],"0,1":["2","south"],"1,1":["3","south-east"],"-1,0":["4","west"],"1,0":["6","east"],"-1,-1":["7","north-west"],"0,-1":["8","north"],"1,-1":["9","north-east"]};
const dungeon = state => state.dungeonStates.find(row => row.dungeonId === DUNGEON);

// Called by the shared route runner too: hidden actors and the live avenger
// must be recorded from Rust, not inferred from the visible entity projection.
export async function prepareAsgard({invoke,snapshot,reloadPrepared,report}, phase, targetId = null) {
  const before = await snapshot();
  await invoke("prepare_asgard_e2e", {phase,targetId});
  const after = await reloadPrepared();
  const removed = before.activeActors.filter(actor => !after.activeActors.some(current => current.id === actor.id));
  if (phase !== "arrival") {
    assert.ok(removed.every(actor => !protectedKinds.has(actor.kindId)), "route clear removed a required combat actor");
    for (const actor of before.activeActors.filter(actor => protectedKinds.has(actor.kindId))) {
      const retained = after.activeActors.find(current => current.id === actor.id);
      assert.ok(retained);
      assert.equal(retained.hp, actor.hp);
      assert.equal(retained.maxHp, actor.maxHp);
      assert.equal(retained.kindId, actor.kindId);
      assert.equal(retained.energyNeed, actor.energyNeed);
      assert.deepEqual(retained.statuses, actor.statuses);
    }
  }
  report.checks.push({type:"preparation",phase,targetId,beforeHash:before.stateHash,afterHash:after.stateHash,
    removed:removed.map(({id,kindId,position})=>({id,kindId,position})),
    retained:after.activeActors.map(({id,kindId,position,hp,maxHp,energyNeed})=>({id,kindId,position,hp,maxHp,energyNeed})),
    player:{position:after.player.position,progress:after.player.progress,hp:after.player.hp,equipment:after.equipment,statuses:after.player.statuses}});
  return after;
}

export async function runAsgardScenario(context) {
  const {driver,keyboard,report,invoke,snapshot,click,changed,closeDialogs,reloadPrepared,capture,walkTo,nativeSaveRoundTrip,saveListCount,setShift} = context;
  await driver.execute('window.__asgardErrors=[];window.addEventListener("error",event=>window.__asgardErrors.push(event.message));window.addEventListener("unhandledrejection",event=>window.__asgardErrors.push(String(event.reason)));return true;');
  report.fixture = "Fresh Human Warrior; normal birth seeds are tried until Norse is active, without editing the pantheon mask. WebDriver preparation physically places the surface at 94,11, grants source XP to level 50, a +100/+100 broad sword, two Homeward scrolls, long levitation/invulnerability/see-invisible, full player HP and map/secret discovery. Route preparation removes unrelated actors and carried items, preserves Heimdall/Odin/Vidarr at their source HP/defenses, energy and statuses; their ordinary AI continues. Battle preparation places the player on a free adjacent tile and restores player HP. Victory uses actual player melee commands; this is not natural leveling/difficulty acceptance. No terrain, guardian HP, conquest flags or rewards are fabricated. Native keys cover battle/route starts, doors, stairs and pickup; longer movement/attack sequences call production Rust commands and reload the exact native save.";
  const births = [];
  let state = await snapshot();
  for (let seed = 0; (state.activePantheons & 8) === 0 && seed < 32; seed++) {
    births.push({seed:seed === 0 ? 42 : seed-1,activePantheons:state.activePantheons});
    const lists = await saveListCount();
    await click("#session-new-game");
    await selectCreationRace(driver,"demo.race.rfb-human");
    await selectCreationBuild(driver,"demo.build.warrior");
    await driver.execute('const input=document.querySelector("#session-seed");input.value=String(arguments[0]);input.dispatchEvent(new Event("input",{bubbles:true}));return true;',[seed]);
    await click("#session-start-game");
    await driver.waitFor('return document.documentElement.dataset.appMode==="playing" && document.querySelector("#connection-status").classList.contains("ready")',"new Norse candidate",60_000);
    state=await snapshot();
    assert.equal(await saveListCount(),lists,"birth must not list unrelated saves");
  }
  assert.ok(state.activePantheons & 8,"bounded births must include an active Norse game");
  report.checks.push({type:"birth",seed:await driver.execute('return document.querySelector("#session-seed").value'),skipped:births,activePantheons:state.activePantheons,hash:state.stateHash});
  state=await prepareAsgard(context,"arrival");
  if(state.player.pendingRaceMutationChoice) {
    const index=state.player.pendingRaceMutationChoice.candidates.findIndex(row=>row.id==="rfb.mutation.sacred-vitality");
    assert.ok(index>=0);
    await click("#player-ui-character-open");await click("#character-tab-other");
    await driver.execute('document.querySelectorAll(".mutation-choice-candidate")[arguments[0]].focus();return true;',[index]);
    await keyboard.key("Enter");await changed(state.stateHash,"human talent");await closeDialogs();
    state=await snapshot();
  }
  assert.deepEqual(state.wildernessPosition,{x:94,y:11});
  const entrance=state.cells.find(cell=>cell.terrainId==="demo.terrain.asgard-entrance").position;
  await capture("asgard-bifrost-entrance");
  await nativeSaveRoundTrip("entrance-before-battle");

  async function battle(kind, name) {
    state=await snapshot();
    const source=state.activeActors.find(actor=>actor.kindId===kind);
    assert.ok(source,`${name} must be present`);
    assert.equal(source.hp,source.maxHp,`${name} must start at full source HP`);
    state=await prepareAsgard(context,"battle",source.id);
    await closeDialogs();await capture(`${name}-before-melee`);
    const trace=[];
    const combat={type:"melee",name,source,preparation:"player placement and HP; source enemy HP, definitions, energy and statuses retained; ordinary enemy AI",trace,completed:false};
    report.checks.push(combat);
    while(trace.length<512) {
      let actor=state.activeActors.find(actor=>actor.id===source.id);
      if(!actor) break;
      if(Math.max(Math.abs(actor.position.x-state.player.position.x),Math.abs(actor.position.y-state.player.position.y))>1) {
        state=await prepareAsgard(context,"battle",source.id);
        actor=state.activeActors.find(actor=>actor.id===source.id);
      }
      const delta=`${actor.position.x-state.player.position.x},${actor.position.y-state.player.position.y}`;
      assert.ok(directions[delta],"prepared battle must remain adjacent");
      const [key,direction]=directions[delta];
      if(trace.length===0 || actor.hp<500) {
        await keyboard.key(key);await changed(state.stateHash,`${name} native melee`);
        state=await snapshot();
        trace.push({input:"native",key,hash:state.stateHash,hp:state.activeActors.find(actor=>actor.id===source.id)?.hp ?? 0});
      } else {
        await driver.execute(`window.__asgardAttacks=undefined;window.__asgardAttackError=null;
          (async()=>{let seq=arguments[0],revision=arguments[1];const trace=[];
            for(let i=0;i<16;i++) {
              const update=await window.__TAURI_INTERNALS__.invoke('dispatch_game_command',{commandSeq:++seq,expectedRevision:revision,command:{type:'move',direction:arguments[2]}});
              revision=update.revision;
              const current=await window.__TAURI_INTERNALS__.invoke('inspect_game_e2e');
              const actor=current.activeActors.find(actor=>actor.id===arguments[3]);
              trace.push({input:'production Rust melee',hash:update.stateHash,hp:actor?.hp ?? 0,events:update.events});
              if(!actor || update.player.hp<=0 || update.player.position.x!==arguments[4].x || update.player.position.y!==arguments[4].y || actor.position.x!==arguments[5].x || actor.position.y!==arguments[5].y) break;
            } return trace;
          })().then(value=>window.__asgardAttacks=value,error=>window.__asgardAttackError=String(error));return true;`,[state.lastCommandSeq,state.revision,direction,source.id,state.player.position,actor.position]);
        await driver.waitFor('return window.__asgardAttacks!==undefined || window.__asgardAttackError',`${name} melee segment`,60_000);
        assert.equal(await driver.execute('return window.__asgardAttackError'),null);
        trace.push(...await driver.execute('return window.__asgardAttacks'));
        state=await reloadPrepared();
      }
      assert.ok(state.player.hp>0,`${name} prepared player survived`);
    }
    assert.ok(!state.activeActors.some(actor=>actor.id===source.id),`${name} did not die from bounded melee`);
    combat.completed=true;
    await capture(`${name}-defeated`);
  }

  async function stairs(terrain, expected) {
    state=await prepareAsgard(context,"route");
    const connection=state.cells.find(cell=>cell.terrainId===terrain);
    assert.ok(connection,`${state.floorId}: ${terrain}`);
    const from=state.floorId;
    await walkTo(connection.position);await closeDialogs();
    state=await snapshot();
    await keyboard.key(">");await changed(state.stateHash,`${from} stairs`);
    state=await snapshot();setShift({x:0,y:0});
    assert.equal(state.floorId,expected);
    report.checks.push({type:"stairs",from,to:expected,terrain,arrival:state.player.position});
    return state;
  }

  await battle(HEIMDALL,"heimdall");
  assert.equal(dungeon(state).entranceGuardianDefeated,true);
  assert.ok(!dungeon(state).guardianDefeated);
  await stairs("demo.terrain.asgard-entrance","demo.floor.asgard-depth-64");
  for(const depth of [64,68,72,76,80,82,84,86,88]) {
    if(depth!==64) await stairs("demo.terrain.shaft-down",`demo.floor.asgard-depth-${depth}`);
    state=await prepareAsgard(context,"route");
    if([64,76,80,88].includes(depth)) {
      await capture(`depth-${depth}-arrival`);
      await nativeSaveRoundTrip(`depth-${depth}`);
    }
    if([64,80].includes(depth)) {
      const before=report.screenshots.at(-1).diagnostics;
      const candidates=state.cells.filter(cell=>cell.terrainId==="demo.terrain.floor" && !cell.actorId);
      const far=candidates.reduce((best,cell)=>Math.abs(cell.position.x-state.player.position.x)>Math.abs(best.position.x-state.player.position.x)?cell:best);
      await walkTo(far.position);await capture(`depth-${depth}-scrolled`);
      const after=report.screenshots.at(-1).diagnostics;
      assert.ok(Number(before.scrollX)!==Number(after.scrollX) || Number(before.scrollY)!==Number(after.scrollY),"normal camera should follow a distant walk");
      report.checks.push({type:"camera",depth,from:before,to:after,zoom:1});
    }
  }
  await battle(ODIN,"odin");
  assert.equal(dungeon(state).guardianDefeated,true);
  const avenger=state.activeActors.find(actor=>actor.kindId===VIDARR);
  assert.ok(avenger,"Odin death must summon the real avenger");
  await nativeSaveRoundTrip("odin-dead-vidarr-alive");
  state=await prepareAsgard(context,"route");
  assert.equal(state.activeActors.find(actor=>actor.id===avenger.id)?.hp,avenger.hp,"route clear must preserve the saved avenger");
  await battle(VIDARR,"vidarr");
  await nativeSaveRoundTrip("all-three-defeated");

  async function pickup(kind) {
    state=await snapshot();
    const item=state.groundItems.find(item=>item.kindId===kind);
    assert.ok(item,`missing source reward ${kind}`);
    await walkTo(item.position);
    for(let attempt=0;attempt<24;attempt++) {
      state=await snapshot();
      if(state.inventory.some(row=>row.id===item.id)) return item.id;
      await closeDialogs();await keyboard.key("g");await changed(state.stateHash,`pick up ${kind}`);
    }
    throw new Error(`could not pick up ${kind}`);
  }
  async function details(id,name) {
    await closeDialogs();await click("#player-ui-inventory-open");
    await click(`#inventory-list [data-item-id="${id}"] .inventory-item-inspect`);
    await driver.waitFor('return document.querySelector("#inventory-detail-dialog").open',"reward details");
    const text=await driver.execute('return document.querySelector("#inventory-detail-body").textContent');
    assert.ok(text.trim());await capture(`${name}-details`);
    report.checks.push({type:"reward-details",id,text});
    await click("#inventory-detail-close");await closeDialogs();
  }
  async function use(id) {
    await closeDialogs();await click("#player-ui-inventory-open");
    await driver.execute('for(const input of document.querySelectorAll("#inventory-list input:checked")) input.click();document.querySelector(arguments[0]).click();return true;',[`#inventory-list [data-item-id="${id}"] input[type="checkbox"]`]);
    state=await snapshot();await click("#inventory-use");await changed(state.stateHash,`use ${id}`);await closeDialogs();
    state=await snapshot();assert.ok(!state.inventory.some(item=>item.id===id));
  }
  const rune=await pickup("demo.item.runespear");
  const scroll=await pickup("demo.item.acquirement-scroll");
  await details(rune,"runespear");await details(scroll,"conquest-scroll");
  await use(scroll);
  const acquired=state.groundItems.filter(item=>item.originKind==="acquire");
  assert.ok(acquired.length>0,"conquest scroll must create actual acquisition items");
  report.checks.push({type:"reward-use",scroll,rune,acquired});
  await capture("conquest-scroll-used");await nativeSaveRoundTrip("reward-used");
  for(const depth of [86,84,82,80,76,72,68,64]) await stairs("demo.terrain.shaft-up",`demo.floor.asgard-depth-${depth}`);
  await stairs("demo.terrain.stairs-up","core.floor.wilderness");
  assert.deepEqual(state.wildernessPosition,{x:94,y:11});
  assert.deepEqual(state.player.position,entrance);
  await capture("returned-to-bifrost");await nativeSaveRoundTrip("surface-return");
  for(const [id,expected] of [["e2e.asgard.recall.1","demo.floor.asgard-depth-88"],["e2e.asgard.recall.2","core.floor.wilderness"]]) {
    await use(id);await nativeSaveRoundTrip(`recall-pending-${expected}`);
    for(let turn=0;turn<40;turn++) {
      state=await snapshot();if(state.floorId===expected) break;
      state=await prepareAsgard(context,"route");
      await keyboard.key("5");await changed(state.stateHash,"recall countdown");
    }
    state=await snapshot();assert.equal(state.floorId,expected);
    assert.ok(!state.activeActors.some(actor=>protectedKinds.has(actor.kindId)));
    assert.ok(!state.groundItems.some(item=>item.kindId==="demo.item.acquirement-scroll"));
    assert.equal(state.inventory.filter(item=>item.id===rune).length,1);
    await capture(expected==="core.floor.wilderness"?"recall-surface":"recall-bottom");
    await nativeSaveRoundTrip(expected==="core.floor.wilderness"?"final":"recall-bottom");
  }
  report.checks.push({type:"completed-route",dungeon:dungeon(state),defeatedActorCounts:state.defeatedActorCounts,finalHash:state.stateHash,sourceReward:rune});
  for(const kind of protectedKinds) assert.equal(state.defeatedActorCounts.find(row=>row.actorKindId===kind)?.count,1);
  report.frontendErrors=await driver.execute('return window.__asgardErrors');
  assert.deepEqual(report.frontendErrors,[]);
}
