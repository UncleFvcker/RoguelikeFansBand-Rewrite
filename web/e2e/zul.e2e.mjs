// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { MAP_CELL_SIZE } from "../src/camera.ts";

// The AT4 runner owns native input, map navigation, screenshots and save/load.
export async function runZulScenario({driver,keyboard,report,invoke,snapshot,click,changed,closeDialogs,reloadPrepared,capture,walkTo,nativeSaveRoundTrip,travelFromInn,saveListCount,setShift,getShift}) {
  if(process.argv.includes("--zul-map-review")) {
    const full=JSON.parse(await readFile(new URL("../../test-results/zul/report.json",import.meta.url),"utf8"));
    report.fixture="Full-map visual review of the four native active-task checkpoints from the completed Zul run; explicit enemy clear/reveal and 45% WebView zoom. The final travel checkpoint is restored afterward.";
    const listsBeforeRefresh=await saveListCount();
    await click("#native-save-refresh");
    await driver.waitFor('return !document.querySelector("#native-save-refresh").disabled && document.querySelectorAll("#native-save-list .native-save-item").length>0',"explicit save list refresh",60_000);
    assert.equal(await saveListCount(),listsBeforeRefresh+1);
    async function loadCheckpoint(name) {
      await closeDialogs();const before=await snapshot();
      await driver.execute(`const row=[...document.querySelectorAll('#native-save-list .native-save-item')].find(row=>row.querySelector('.native-save-name')?.textContent.startsWith(arguments[0]));
        if(!row)throw new Error('missing native checkpoint '+arguments[0]);row.querySelector('[data-native-save-action="load"]').click();return true;`,[`Z6 ${name} `]);
      await changed(before.stateHash,`load ${name} for map review`);
      await driver.waitFor('return !document.querySelector("#look-mode-toggle").disabled',"map review input idle");
      assert.equal(await saveListCount(),listsBeforeRefresh+1,"selected load must not list unrelated saves");
      report.checks.push({type:"save-scope",action:"load",name,listCalls:0});
      return snapshot();
    }
    await invoke("plugin:webview|set_webview_zoom",{label:"main",value:0.45});
    try {
      for(const id of ["eddies","sorcery-node","chaos-node","nature-node"]) {
        let state=await loadCheckpoint(`zul-${id}-active`);
        assert.equal(state.floorId,`demo.floor.zul-${id}`);
        await invoke("prepare_zul_e2e",{clearEnemies:true});state=await reloadPrepared();
        await capture(`zul-${id}-full-map`);
        const view=report.screenshots.at(-1).diagnostics;
        assert.ok(state.width*MAP_CELL_SIZE*Number(view.zoom)<=Number(view.viewportWidth));
        assert.ok(state.height*MAP_CELL_SIZE*Number(view.zoom)<=Number(view.viewportHeight));
        report.checks.push({type:"full-map",id,width:state.width,height:state.height});
      }
    } finally { await invoke("plugin:webview|set_webview_zoom",{label:"main",value:1}); }
    const final=await loadCheckpoint("zul-final-travel");
    assert.equal(final.stateHash,full.checks.find(row=>row.type==="round-trip").finalHash);
    report.checks.push({type:"final-state-restored",hash:final.stateHash});
    return;
  }
  report.fixture = "Normal Beastman Sorcery/Nature Mage creation. WebDriver preparation visits/reveals Zul, grants level 50 and one Life book, levitation/invulnerability, eight nonzero virtues and 10000000 ground gold. Route and task success preparation removes enemies and their carried items only; a normal wait evaluates task objectives. Terrain, entrance rules, rewards, travel, shops, realm changes and saves use production rules. Route starts/entrances use native keys; long segments use planned production Rust movement commands followed by exact native-save restoration. Hostile actors are explicitly cleared before each segment with the WebDriver preparation. This is not natural combat/progression acceptance.";
  await invoke("prepare_town_map_e2e",{townId:"demo.town.zul"});
  await invoke("prepare_zul_e2e",{clearEnemies:false});
  await invoke("prepare_supply_e2e",{amount:10000000});
  let state=await reloadPrepared();
  await closeDialogs(); await keyboard.key("g"); await changed(state.stateHash,"collect Zul test funds");
  await capture("zul-arrival");
  const towers={sorcery:{x:65,y:16},chaos:{x:57,y:16},nature:{x:72,y:16}};
  const facility=(state,realm)=>state.taskServices.find(row=>row.id===`demo.town-facility.zul-${realm}-tower`);
  const task=(state,id)=>state.tasks.find(row=>row.taskId===`demo.task.zul-${id}`);
  async function action(selector,label) {
    const before=await snapshot();
    assert.equal(await driver.execute('return !!document.querySelector(arguments[0]) && !document.querySelector(arguments[0]).disabled',[selector]),true,selector);
    await click(selector); await changed(before.stateHash,label); return snapshot();
  }
  async function tower(realm) {
    const state=await walkTo(towers[realm]);
    await driver.waitFor('return document.querySelector("#task-service-dialog").open',`${realm} tower dialog`);
    return state;
  }
  async function dropItem(item) {
    await closeDialogs();
    const before=await snapshot();
    await click("#player-ui-inventory-open");
    await driver.execute(`const selected=document.querySelector(arguments[0]);
      for(const input of document.querySelectorAll('#inventory-list input[type="checkbox"]:checked')) if(input!==selected) input.click();
      if(!selected.checked) selected.click();return true;`,[`#inventory-list [data-item-id="${item.id}"] input[type="checkbox"]`]);
    await click("#inventory-drop");
    if(await driver.execute('return document.querySelector("#inventory-action-dialog").open')) {
      await driver.execute('document.querySelector("#inventory-drop-quantity").value="1";return true;');await click("#inventory-action-confirm");
    }
    await changed(before.stateHash,"drop purchased item");await closeDialogs();
    const after=await snapshot();
    const ground=after.items.find(row=>row.kindId===item.kindId && row.position.x===after.player.position.x && row.position.y===after.player.position.y);
    assert.ok(ground);const shift=getShift();
    return {...ground,position:{x:ground.position.x+shift.x,y:ground.position.y+shift.y}};
  }
  let dropped;
  for(const [category,position] of [["general-store",{x:31,y:27}],["jeweler",{x:16,y:29}],["dragonskin",{x:58,y:30}]]) {
    state=await walkTo(position);
    const shop=state.shops.find(row=>row.id===`demo.shop.zul-${category}`);
    assert.ok(shop?.playerAtEntrance);
    await driver.waitFor('return document.querySelector("#shop-dialog").open',`${category} shop dialog`);
    await capture(`zul-${category}`);
    const stock=shop.stock[0];
    const quantity=state.inventory.filter(row=>row.kindId===stock.kindId).reduce((sum,row)=>sum+row.quantity,0);
    await click(`[data-shop-item-id="${stock.id}"]`);
    await driver.execute('const input=document.querySelector("#shop-quantity");input.value="1";input.dispatchEvent(new Event("input",{bubbles:true}));return true;');
    state=await action("#shop-confirm",`${category} purchase`);
    assert.equal(state.inventory.filter(row=>row.kindId===stock.kindId).reduce((sum,row)=>sum+row.quantity,0),quantity+1);
    const acquired=state.inventory.find(row=>row.kindId===stock.kindId);
    await nativeSaveRoundTrip(`zul-${category}`);
    if(category==="general-store") {
      await walkTo({x:53,y:32});dropped=await dropItem(acquired);
    }
    report.checks.push({type:"shop",id:shop.id,kind:stock.kindId,price:stock.unitPrice});
  }
  state=await tower("sorcery");
  assert.equal(facility(state,"sorcery").membership,"owner");
  assert.deepEqual(facility(state,"sorcery").innTravelDestinations??[],[]);
  await capture("zul-sorcery-locked");
  if(await driver.execute('return !document.querySelector(\'[data-facility-action="identify-all"]\').disabled')) {
    state=await action('[data-facility-action="identify-all"]',"identify all");
    assert.ok([...state.inventory,...state.equipment].every(row=>row.identification!=="unexamined"));
  }
  state=await tower("chaos");assert.equal(facility(state,"chaos").membership,"member");
  await capture("zul-chaos-member");
  const cure='button[data-facility-service="cure-mutation"]';
  if(await driver.execute('return !document.querySelector(arguments[0]).disabled',[cure])) {
    const before=state.player.gold;state=await action(cure,"cure Beastman mutation");assert.ok(state.player.gold<before);
    report.checks.push({type:"mutation-cure",cost:before-state.player.gold});
  }
  state=await tower("nature");assert.equal(facility(state,"nature").membership,"owner");
  await capture("zul-nature-owner");
  state=await action('button[data-facility-service="balance-ritual"]',"balance ritual");
  assert.equal(state.player.virtues.length,8);assert.ok(state.player.virtues.every(row=>row.value===0));
  // The granted Life book drives a real secondary-realm confirmation and changed tower identity.
  await closeDialogs();await click("#player-ui-ability-open");
  await action('#realm-change-books [data-realm-id="life"]',"request Life realm");
  await action("#realm-change-accept","confirm Life realm");await closeDialogs();
  state=await snapshot();assert.equal(facility(state,"nature").membership,"visitor");
  await walkTo({x:71,y:16});state=await tower("nature");await capture("zul-nature-visitor");
  assert.equal(task(state,"nature-node").unavailableReason,"task-membership-required");
  assert.equal(await driver.execute('return document.querySelector(\'button[data-facility-service="balance-ritual"]\').disabled'),false);
  await closeDialogs();await click("#player-ui-ability-open");
  await action('#realm-change-books [data-realm-id="nature"]',"request Nature realm");
  await action("#realm-change-accept","restore Nature realm");await closeDialogs();
  report.checks.push({type:"realm-membership",sequence:["nature-owner","life-visitor","nature-owner"],chaos:"beastman-member"});
  // Explore both axes outside the initial view; Rust supplies all scroll/rebase results.
  for(const [name,position] of [["east",{x:152,y:32}],["south",{x:90,y:70}],["northwest",{x:-10,y:-10}]]) {
    await walkTo(position);await capture(`zul-scroll-${name}`);await nativeSaveRoundTrip(`zul-scroll-${name}`);
  }
  for(const [id,realm,entry] of [["eddies","sorcery",{x:91,y:32}],["sorcery-node","sorcery",{x:78,y:44}],["chaos-node","chaos",{x:9,y:1}],["nature-node","nature",{x:61,y:4}]]) {
    state=await tower(realm);
    await action(`[data-task-id="demo.task.zul-${id}"][data-task-action="accept"]`,`${id} acceptance`);
    state=await walkTo(entry);
    assert.equal(state.cells.find(cell=>cell.position.x===state.player.position.x && cell.position.y===state.player.position.y).terrainId,`demo.terrain.zul-${id}-entry`);
    await capture(`zul-${id}-entrance`);
    const surfaceShift=getShift();
    state=await action("#traverse-stairs",`${id} entry`);setShift({x:0,y:0});
    assert.equal(state.floorId,`demo.floor.zul-${id}`);
    const exit={...state.player.position}, enemyCount=state.activeActorCount;
    await capture(`zul-${id}-inside`);
    await nativeSaveRoundTrip(`zul-${id}-active`);
    await invoke("prepare_zul_e2e",{clearEnemies:true});state=await reloadPrepared();await closeDialogs();
    await capture(`zul-${id}-cleared-map`);
    await keyboard.key("5");await changed(state.stateHash,`${id} objective evaluation`);state=await snapshot();
    assert.equal(task(state,id).status,"reward-available");
    if(id==="eddies") {
      for(const position of [{x:6,y:32},{x:22,y:32}]) {
        state=await walkTo(position);await keyboard.key("g");await changed(state.stateHash,"collect source bat cloak");
      }
    }
    await walkTo(exit);state=await action("#traverse-stairs",`${id} return`);setShift(surfaceShift);
    assert.equal(state.town.id,"demo.town.zul");
    assert.deepEqual({x:state.player.position.x+surfaceShift.x,y:state.player.position.y+surfaceShift.y},entry);
    await capture(`zul-${id}-returned`);
    await tower(realm);state=await action(`[data-task-id="demo.task.zul-${id}"][data-task-action="claim"]`,`${id} source reward`);
    assert.equal(task(state,id).status,"completed");
    const reward=state.inventory.find(row=>row.id===`demo.task.zul-${id}.reward.1`);assert.ok(reward);
    await nativeSaveRoundTrip(`zul-${id}-completed`);
    report.checks.push({type:"task",id,enemyCount,preparation:"explicit enemy clear; normal wait/return/claim",reward:reward.kindId});
  }
  state=await tower("sorcery");await capture("zul-sorcery-unlocked");
  await action('[data-facility-action="travel-town"][data-town-id="demo.town.outpost"]',"Zul tower to old town");setShift({x:0,y:0});
  assert.equal((await snapshot()).town.id,"demo.town.outpost");await capture("zul-old-town-inn");
  await travelFromInn("demo.town.zul");state=await snapshot();
  assert.deepEqual(state.player.position,towers.sorcery);
  assert.deepEqual(state.items.find(row=>row.id===dropped.id),dropped);
  await nativeSaveRoundTrip("zul-final-travel");await closeDialogs();await capture("zul-final");
  report.checks.push({type:"round-trip",destination:"demo.town.outpost",return:state.player.position,groundItem:dropped,finalHash:state.stateHash});
}
