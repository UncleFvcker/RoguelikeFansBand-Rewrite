// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { connectKeyboard } from "./character-creation-layout.e2e.mjs";
import { selectCreationBuild, selectCreationRace } from "./character-creation.e2e.mjs";
import { nextWalk } from "./berserker.e2e.mjs";
import { runZulScenario } from "./zul.e2e.mjs";
import { runAsgardScenario, prepareAsgard } from "./asgard.e2e.mjs";

export async function runTownMapScenario(driver, directory, profile, scenario = "towns") {
  const zulOnly = scenario === "zul", asgardOnly = scenario === "asgard", batchWalk = zulOnly || asgardOnly;
  await mkdir(directory, { recursive: true });
  const keyboard = await connectKeyboard(profile);
  const terrainDirectory = new URL("../../packs/rfb-demo-original/terrain/", import.meta.url);
  const terrain = await Promise.all((await readdir(terrainDirectory)).filter(name => name.endsWith(".json"))
    .map(async name => JSON.parse(await readFile(new URL(name, terrainDirectory), "utf8"))));
  // Rust executes each key; the Zul fixture can fly and explicitly clear route blockers.
  const walkable = new Set(terrain.filter(row => (!asgardOnly || !row.trap) && (row.walkable || batchWalk && row.movementModes?.includes("fly"))).map(row => row.id));
  const doors = new Map(terrain.filter(row => !row.walkable && (row.openToTerrainId || row.bashToTerrainId)).map(row => [row.id, row.openToTerrainId ? "o" : "B"]));
  const directions = { "1": [-1,1], "2": [0,1], "3": [1,1], "4": [-1,0], "6": [1,0], "7": [-1,-1], "8": [0,-1], "9": [1,-1] };
  const report = { fixture: "New level-one Warrior; visited town and revealed its surface with the WebDriver-only fixture; 10000 test gold. No altered terrain, task states, XP, combat results or granted items. All route steps use native keyboard input; transactions, tasks, travel and saves use the application UI.", checks: [], screenshots: [] };
  let shift = { x: 0, y: 0 };
  const click = selector => driver.execute('document.querySelector(arguments[0]).click(); return true;', [selector]);
  const ready = () => driver.waitFor('return document.querySelector("#connection-status").classList.contains("ready")', "town UI ready");
  const saveListCount = async () => (await readFile(new URL("../../test-results/e2e-rfb-desktop.log",import.meta.url),"utf8")).split("native-save-list-started").length-1;
  async function invoke(command, args = {}) {
    await driver.execute(`window.__townReply=undefined; window.__townError=null;
      window.__TAURI_INTERNALS__.invoke(arguments[0],arguments[1]).then(value=>window.__townReply=value,error=>window.__townError=String(error));return true;`, [command,args]);
    await driver.waitFor('return window.__townReply!==undefined || window.__townError', command, 30_000);
    assert.equal(await driver.execute('return window.__townError'), null);
    return driver.execute('return window.__townReply');
  }
  const snapshot = () => invoke("inspect_game_e2e");
  async function closeDialogs() {
    await driver.execute(`for(const [dialog,button] of [["shop-dialog","shop-close"],["home-dialog","home-close"],["task-service-dialog","task-service-close"],["player-page-dialog","player-page-close"]]) {
      if(document.getElementById(dialog)?.open) document.getElementById(button).click();
    } document.activeElement?.blur(); return true;`);
  }
  async function changed(before, label) {
    try {
      await driver.waitFor('return document.querySelector("#hash-value").title!==arguments[0] && document.querySelector("#connection-status").classList.contains("ready")', label, 15_000, [before]);
    } catch(error) {
      const state=await snapshot();
      const ui=await driver.execute('return {keys:window.__townKeys,active:{tag:document.activeElement?.tagName,id:document.activeElement?.id},dialogs:[...document.querySelectorAll("dialog[open]")].map(row=>row.id),map:{...document.querySelector("#map-host").dataset}}');
      await writeFile(path.join(directory,"input-failure.json"),JSON.stringify({label,before,state,ui},null,2));
      throw error;
    }
  }
  async function reloadPrepared() {
    const before = await snapshot();
    const bytes = await invoke("save_game", { savedAt: "2026-09-12T12:00:00Z" });
    await driver.execute(`const files=new DataTransfer();files.items.add(new File([new Uint8Array(window.__townReply)],"town-map-prepared.rfbsave"));
      const input=document.querySelector("#load-input");input.files=files.files;input.dispatchEvent(new Event("change",{bubbles:true}));return true;`);
    await driver.waitFor('return document.querySelector("#connection-status").classList.contains("error") || document.querySelector("#hash-value").title===arguments[0] && document.querySelector("#connection-status").classList.contains("ready")', "prepared native save displayed", 30_000, [before.stateHash]);
    if(await driver.execute('return document.querySelector("#connection-status").classList.contains("error")')) {
      await writeFile(path.join(directory,"prepared-save-failure.rfbsave"),Buffer.from(bytes));
      throw new Error("Prepared native save rejected; exact bytes saved for diagnosis");
    }
    assert.equal((await snapshot()).stateHash, before.stateHash);
    return before;
  }
  async function capture(name) {
    await driver.execute('window.__townPainted=false;requestAnimationFrame(()=>requestAnimationFrame(()=>window.__townPainted=true));return true;');
    await driver.waitFor('return window.__townPainted', "town frame painted");
    const layout = await driver.execute(`const host=document.querySelector("#map-host"),canvas=host.querySelector("canvas");
      return {viewport:[innerWidth,innerHeight],host:[host.clientWidth,host.clientHeight],canvas:canvas?[canvas.width,canvas.height]:null,diagnostics:{...host.dataset},pageFits:document.documentElement.scrollWidth<=innerWidth};`);
    assert.ok(layout.canvas?.every(value => value > 0), "real map canvas");
    assert.ok(layout.pageFits, "map must not overflow the window");
    await writeFile(path.join(directory, `${name}.png`), await driver.screenshot(), "base64");
    report.screenshots.push({ name, ...layout });
  }
  async function walkTo(local) {
    let state=await snapshot();
    const start={x:state.player.position.x+shift.x,y:state.player.position.y+shift.y};
    let steps=0;
    while(steps<500) {
      if(batchWalk && state.floorId==="core.floor.wilderness") shift={
        x:(state.wildernessPosition.x-(asgardOnly?94:77))*state.width+state.wildernessViewOffset.x*state.width/3,
        y:(state.wildernessPosition.y-(asgardOnly?11:6))*state.height+state.wildernessViewOffset.y*state.height/3,
      };
      if(asgardOnly && state.floorId!=="core.floor.wilderness") shift={x:0,y:0};
      const target={x:local.x-shift.x,y:local.y-shift.y};
      const {x,y}=state.player.position;
      if(x===target.x && y===target.y) {
        report.checks.push({type:"walk",town:state.town?.id,start,destination:local,steps,shift:{...shift}});
        process.stdout.write(`${state.town?.id}: walked ${steps} steps to ${local.x},${local.y}; shift ${shift.x},${shift.y}.\n`);
        return state;
      }
      if(zulOnly && state.activeActorCount>0) {
        const count=state.activeActorCount;
        await invoke("prepare_zul_e2e",{clearEnemies:true});state=await reloadPrepared();
        report.checks.push({type:"route-clear",enemies:count,position:state.player.position});
      }
      if(asgardOnly && state.activeActors.some(actor=>!["demo.actor.heimdall-guardian-of-bifrost","demo.actor.odin-the-all-father","demo.actor.vidarr-the-silent-avenger"].includes(actor.kindId))) {
        state=await prepareAsgard({invoke,snapshot,reloadPrepared,report},"route");
      }
      await closeDialogs();
      await driver.waitFor('return !document.querySelector("#look-mode-toggle").disabled',"town input idle");
      const navigation={...state,player:{...state.player,position:{...state.player.position}},
        cells:state.cells.filter(cell=>(walkable.has(cell.terrainId) || asgardOnly && doors.has(cell.terrainId)) && (!cell.actorId || cell.actorId===state.player.id))
          .map(cell=>({...cell,terrainId:"demo.terrain.floor"}))};
      const destination=target.x<0 || target.x>=state.width || target.y<0 || target.y>=state.height
        ? navigation.cells.reduce((best,cell)=>Math.hypot(cell.position.x-target.x,cell.position.y-target.y)<Math.hypot(best.x-target.x,best.y-target.y)?cell.position:best,navigation.cells[0].position)
        : target;
      // Keep native input at route starts and entrances; batch long stretches through production Rust commands.
      let native=!batchWalk || steps===0 || Math.max(Math.abs(target.x-x),Math.abs(target.y-y))<=2;
      const keys=[];
      for(let i=0;i<(native?1:16);i++) {
        if(navigation.player.position.x===destination.x && navigation.player.position.y===destination.y) break;
        const key=nextWalk(navigation,new Set(),destination),[dx,dy]=directions[key];
        const nextCell=state.cells.find(cell=>cell.position.x===navigation.player.position.x+dx && cell.position.y===navigation.player.position.y+dy);
        if(asgardOnly && doors.has(nextCell?.terrainId)) {
          if(keys.length===0) { keys.push(key); native=true; }
          break;
        }
        keys.push(key);
        navigation.player.position={x:navigation.player.position.x+dx,y:navigation.player.position.y+dy};
      }
      assert.ok(keys.length,"route must propose a step");
      let moved;
      if(native) {
        const [dx,dy]=directions[keys[0]];
        const cell=state.cells.find(cell=>cell.position.x===x+dx && cell.position.y===y+dy);
        if(asgardOnly && doors.has(cell?.terrainId)) {
          await keyboard.key(doors.get(cell.terrainId));
          await keyboard.key(keys[0]);await changed(state.stateHash,"open route door");
          report.checks.push({type:"door",position:cell.position,terrain:cell.terrainId});
          state=await snapshot();steps++;continue;
        }
        await keyboard.key(keys[0]);await changed(state.stateHash,`walk ${keys[0]} toward ${local.x},${local.y}`);
        steps++;
      } else {
        await driver.execute(`window.__townBatch=undefined;window.__townBatchError=null;
          (async()=>{const trace=[];let previous={...arguments[1]},seq=arguments[3],revision=arguments[4];
            const names={'1':'south-west','2':'south','3':'south-east','4':'west','6':'east','7':'north-west','8':'north','9':'north-east'};
            for(const key of arguments[0]) {
              const update=await window.__TAURI_INTERNALS__.invoke('dispatch_game_command',{commandSeq:++seq,expectedRevision:revision,command:{type:'move',direction:names[key]}});
              revision=update.revision;
              const {x,y}=update.player.position,[dx,dy]=arguments[2][key];
              trace.push({key,position:{x,y},hash:update.stateHash});
              if(x!==previous.x+dx || y!==previous.y+dy) break;
              previous={x,y};
            } return trace;
          })().then(value=>window.__townBatch=value,error=>window.__townBatchError=String(error));return true;`,[keys,state.player.position,directions,state.lastCommandSeq,state.revision]);
        await driver.waitFor('return window.__townBatch!==undefined || window.__townBatchError',"planned route segment",60_000);
        assert.equal(await driver.execute('return window.__townBatchError'),null);
        const trace=await driver.execute('return window.__townBatch');steps+=trace.length;
        report.checks.push({type:"route-batch",input:"production Rust movement commands",trace});
        await reloadPrepared();
      }
      moved=await snapshot();
      if(!batchWalk && (Math.abs(moved.player.position.x-x)>1 || Math.abs(moved.player.position.y-y)>1)) {
        const [dx,dy]=directions[keys[0]];shift={x:shift.x+x+dx-moved.player.position.x,y:shift.y+y+dy-moved.player.position.y};
      }
      state=moved;
    }
    throw new Error(`Town walk did not reach ${JSON.stringify(local)}`);
  }
  async function nativeSaveRoundTrip(name) {
    await closeDialogs();
    const before=await snapshot();
    const saveName=`${asgardOnly ? "AS6" : zulOnly ? "Z6" : "AT4"} ${name} ${Date.now()}`;
    await driver.execute('const input=document.querySelector("#native-save-name");input.value=arguments[0];input.dispatchEvent(new Event("input",{bubbles:true}));document.querySelector("#native-save-create").click();return true;', [saveName]);
    try {
      await driver.waitFor('return [...document.querySelectorAll(".native-save-name")].some(row=>row.textContent===arguments[0])', "town native save", 15_000,[saveName]);
    } catch(error) {
      const bytes=await invoke("save_game",{savedAt:"2026-09-12T12:00:00Z"});
      await writeFile(path.join(directory,`${name}-invalid.rfbsave`),new Uint8Array(bytes));
      await writeFile(path.join(directory,`${name}-invalid.json`),JSON.stringify(before,null,2));
      throw error;
    }
    await keyboard.key("5");
    await changed(before.stateHash,"post-save wait");
    const listsBeforeLoad=batchWalk ? await saveListCount() : 0;
    await driver.execute('const row=[...document.querySelectorAll(".native-save-item")].find(row=>row.querySelector(".native-save-name")?.textContent===arguments[0]);row.querySelector(\'[data-native-save-action="load"]\').click();return true;',[saveName]);
    await driver.waitFor('return document.querySelector("#hash-value").title===arguments[0] && document.querySelector("#connection-status").classList.contains("ready")', "native save exact restoration",30_000,[before.stateHash]);
    assert.equal((await snapshot()).stateHash,before.stateHash);
    if(batchWalk) assert.equal(await saveListCount(),listsBeforeLoad,"selected load must not list unrelated saves");
    report.checks.push({type:"native-save",name,hash:before.stateHash,position:before.player.position,shift:{...shift}});
  }
  async function travelFromInn(destination) {
    const before=await snapshot();
    await driver.waitFor('return document.querySelector("#shop-dialog").open',"inn travel dialog");
    await driver.execute('const select=document.querySelector("#shop-inn-destination");select.value=arguments[0];select.dispatchEvent(new Event("change",{bubbles:true}));return true;',[destination]);
    await click("#shop-inn-travel-confirm");
    await changed(before.stateHash,"inn travel");
    shift={x:0,y:0};
    assert.equal((await snapshot()).town.id,destination);
  }
  try {
    await driver.waitFor('return document.documentElement.dataset.appMode==="title"',"town title",60_000);
    await driver.execute('window.__townReload=true;localStorage.setItem("rfb.locale","zh-CN");localStorage.setItem("rfb.input-preset","numpad");setTimeout(()=>location.reload(),50);return true;');
    await driver.waitFor('return !window.__townReload && document.documentElement.dataset.appMode==="title"',"Chinese town title",60_000);
    await click("#session-new-game");
    await selectCreationRace(driver,zulOnly ? "rfb-legacy.race.beastman" : "demo.race.rfb-human");
    await selectCreationBuild(driver,zulOnly ? "demo.build.mage-sorcery-nature" : "demo.build.warrior");
    await driver.execute('document.querySelector("#session-seed").value="42";document.querySelector("#session-seed").dispatchEvent(new Event("input",{bubbles:true}));return true;');
    const listsBeforeBirth=batchWalk ? await saveListCount() : 0;
    await click("#session-start-game");
    await driver.waitFor('return document.documentElement.dataset.appMode==="playing"',"town character",60_000);
    await ready();
    const born=await snapshot();
    if(batchWalk) {
      assert.equal(await saveListCount(),listsBeforeBirth,"new game must not list unrelated saves");
      report.checks.push({type:"save-scope",action:"new-game",listCalls:0});
    }
    await driver.execute('window.__townKeys=[];window.addEventListener("keydown",event=>queueMicrotask(()=>{window.__townKeys.push({key:event.key,code:event.code,prevented:event.defaultPrevented,target:event.target?.tagName,lookDisabled:document.querySelector("#look-mode-toggle").disabled});window.__townKeys=window.__townKeys.slice(-8);}));return true;');
    report.contentHash=born.contentHash;
    report.protocolVersion=born.protocolVersion;
    if(batchWalk) {
      const run = asgardOnly ? runAsgardScenario : runZulScenario;
      try {
        await run({driver,keyboard,report,invoke,snapshot,click,changed,closeDialogs,reloadPrepared,capture,walkTo,nativeSaveRoundTrip,travelFromInn,saveListCount,
          setShift:value=>{shift=value;},getShift:()=>({...shift})});
        assert.deepEqual(keyboard.errors,[]);
        report.status="passed";
      } catch(error) {
        report.status="failed";report.error=String(error);throw error;
      } finally {
        report.errors=keyboard.errors;
        await writeFile(path.join(directory,process.argv.includes("--zul-map-review") ? "map-review-report.json" : "report.json"),JSON.stringify(report,null,2)+"\n");
      }
      return;
    }
    for(const town of [
      {name:"anambar",shop:{x:92,y:45},second:{x:92,y:46},giver:{x:110,y:28},task:"orc-camp",gate:{x:131,y:41},entry:{x:183,y:62},inn:{x:60,y:54}},
      {name:"thalos",shop:{x:76,y:42},giver:{x:21,y:41},task:"shadow-fairies",gate:{x:131,y:39},entry:{x:141,y:23},inn:{x:24,y:49}},
    ]) {
      const id=`demo.town.${town.name}`,taskId=`demo.task.${town.name}-${town.task}`;
      await invoke("prepare_town_map_e2e",{townId:id});
      await invoke("prepare_supply_e2e",{amount:10000});
      shift={x:0,y:0};
      const prepared=await reloadPrepared();
      assert.equal(prepared.town.id,id);
      assert.equal(prepared.width,198);assert.equal(prepared.height,66);
      await closeDialogs();
      await keyboard.key("g");await changed(prepared.stateHash,"collect test travel funds");
      assert.ok((await snapshot()).player.gold>=10000);
      await capture(`${town.name}-west`);
      let state=await walkTo(town.shop);
      const shop=state.shops.find(shop=>shop.id===`demo.shop.${town.name}-general-store`);
      assert.ok(shop.playerAtEntrance);
      const stock=shop.stock[0], quantity=state.inventory.filter(item=>item.kindId===stock.kindId).reduce((sum,item)=>sum+item.quantity,0);
      await driver.waitFor('return document.querySelector("#shop-dialog").open',"general store opened");
      await capture(`${town.name}-shop`);
      await click(`[data-shop-item-id="${stock.id}"]`);
      await driver.execute('const input=document.querySelector("#shop-quantity");input.value="1";input.dispatchEvent(new Event("input",{bubbles:true}));return true;');
      await click("#shop-confirm");await changed(state.stateHash,"town purchase");
      state=await snapshot();
      assert.equal(state.inventory.filter(item=>item.kindId===stock.kindId).reduce((sum,item)=>sum+item.quantity,0),quantity+1);
      const remaining=state.shops.find(shop=>shop.id===`demo.shop.${town.name}-general-store`).stock;
      if(town.second) {
        state=await walkTo(town.second);
        assert.deepEqual(state.shops.find(candidate=>candidate.id===shop.id).stock,remaining);
      }
      await walkTo({x:99,y:33});
      state=await snapshot();
      const carried=state.inventory.find(item=>item.kindId===stock.kindId);
      await click("#player-ui-inventory-open");
      await driver.execute(`const selected=document.querySelector(arguments[0]);
        for(const input of document.querySelectorAll('#inventory-list input[type="checkbox"]:checked')) if(input!==selected) input.click();
        if(!selected.checked) selected.click(); return true;`,[`#inventory-list [data-item-id="${carried.id}"] input[type="checkbox"]`]);
      await driver.waitFor('return !document.querySelector("#inventory-drop").disabled',"selected item can be dropped");
      await click("#inventory-drop");
      if(await driver.execute('return document.querySelector("#inventory-action-dialog").open')) {
        await driver.execute('document.querySelector("#inventory-drop-quantity").value="1";return true;');
        await click("#inventory-action-confirm");
      }
      await changed(state.stateHash,"town item drop");
      await closeDialogs();
      const dropped=(await snapshot()).items.find(item=>item.kindId===stock.kindId && item.position.x===99 && item.position.y===33);
      assert.ok(dropped,"purchased item on town ground");
      state=await walkTo(town.giver);
      await driver.waitFor('return document.querySelector("#task-service-dialog").open',"quest giver");
      await click(`[data-task-id="${taskId}"][data-task-action="accept"]`);await changed(state.stateHash,"accept town quest");
      await walkTo(town.gate);await capture(`${town.name}-gate`);
      state=await walkTo(town.entry);
      assert.notDeepEqual(shift,{x:0,y:0},"route must cross an actual wilderness scroll boundary");
      assert.equal(state.cells.find(cell=>cell.position.x===state.player.position.x && cell.position.y===state.player.position.y).terrainId,`demo.terrain.${town.name}-${town.task}-entry`);
      await capture(`${town.name}-far-entry`);
      await nativeSaveRoundTrip(`${town.name}-scrolled`);
      state=await snapshot();await click("#traverse-stairs");await changed(state.stateHash,"enter town task");
      state=await snapshot();assert.equal(state.floorId,`demo.floor.${town.name}-${town.task}`);
      assert.equal(state.cells.find(cell=>cell.position.x===state.player.position.x && cell.position.y===state.player.position.y).terrainId,"demo.terrain.stairs-up");
      await click("#traverse-stairs");await changed(state.stateHash,"return from town task");
      state=await snapshot();assert.equal(state.town.id,id);
      assert.equal(state.tasks.find(task=>task.taskId===taskId).status,"failed");
      shift={x:town.entry.x-state.player.position.x,y:town.entry.y-state.player.position.y};
      await capture(`${town.name}-task-return`);
      await nativeSaveRoundTrip(`${town.name}-returned`);
      state=await snapshot();await click("#traverse-stairs");await changed(state.stateHash,"leave town to world map");
      assert.equal((await snapshot()).mapScale,"world");
      state=await snapshot();await click("#traverse-stairs");await changed(state.stateHash,"reenter town from world map");
      state=await snapshot();
      const projectedStore=state.shops.find(candidate=>candidate.id===shop.id).entrancePosition;
      shift={x:town.shop.x-projectedStore.x,y:town.shop.y-projectedStore.y};
      assert.equal(state.town.id,id);
      assert.deepEqual(state.items.find(item=>item.id===dropped.id),{...dropped,position:{x:dropped.position.x-shift.x,y:dropped.position.y-shift.y}});
      await walkTo(town.inn);
      await travelFromInn("demo.town.outpost");
      await travelFromInn(id);
      state=await snapshot();
      assert.deepEqual(state.items.find(item=>item.id===dropped.id),dropped);
      await nativeSaveRoundTrip(`${town.name}-travel`);
      await closeDialogs();await capture(`${town.name}-travel-restored`);
      assert.deepEqual(keyboard.errors,[]);
      report.checks.push({type:"town",id,shop:shop.id,purchasedKind:stock.kindId,groundItem:dropped,taskId,failedReturn:true,worldMapRoundTrip:true,innRoundTrip:true,finalHash:(await snapshot()).stateHash});
      process.stdout.write(`${town.name}: walked, traded, scrolled, returned from quest, saved and revisited.\n`);
    }
    assert.deepEqual(keyboard.errors,[]);
    report.errors=keyboard.errors;
    await writeFile(path.join(directory,"report.json"),JSON.stringify(report,null,2)+"\n");
  } finally { keyboard.close(); }
}
