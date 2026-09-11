// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";

// Opt-in fixture: move to an existing stair, then let the caller use the normal UI to enter.
export async function prepareDungeonEntry(driver) {
  const started = Date.now();
  await driver.execute(`window.__fastEntryDone=false; window.__fastEntryError=null;
    (async()=>{
      const invoke=window.__TAURI_INTERNALS__.invoke;
      const before=await invoke("inspect_game_e2e");
      const stairs=before.cells.find(cell=>cell.terrainId==="demo.terrain.stairs-down");
      if(!stairs) throw new Error("Current map has no projected dungeon stairs");
      const prepared=await invoke("prepare_stairs_e2e",{position:stairs.position});
      const bytes=await invoke("save_game",{savedAt:"2026-09-11T12:00:00Z"});
      window.__fastEntry={before:before.stateHash,prepared:prepared.stateHash,position:stairs.position};
      const original=window.fetch, endpoint=window.__TAURI_INTERNALS__.convertFileSrc("load_game","ipc");
      window.fetch=(url,options)=>{
        if(url!==endpoint) return original(url,options);
        window.fetch=original;
        return original(url,options).then(response=>{setTimeout(()=>window.__fastEntryDone=true,0);return response;});
      };
      const transfer=new DataTransfer(); transfer.items.add(new File([new Uint8Array(bytes)],"fast-entry-test-prepared.rfbsave"));
      const input=document.querySelector("#load-input");input.files=transfer.files;input.dispatchEvent(new Event("change",{bubbles:true}));
    })().catch(error=>window.__fastEntryError=String(error)); return true;`);
  await driver.waitFor(`return window.__fastEntryError || (window.__fastEntryDone
    && document.querySelector("#hash-value").title===window.__fastEntry.prepared
    && document.querySelector("#connection-status").classList.contains("ready"))`, "fast dungeon approach", 30_000);
  assert.equal(await driver.execute('return window.__fastEntryError'), null);
  const result = await driver.execute('return window.__fastEntry');
  result.elapsedMs = Date.now() - started;
  result.preparation = "Skipped approach travel: moved to existing stairs, no granted XP/items or generated floor. Caller still traverses stairs through normal UI.";
  process.stdout.write(`Fast dungeon approach: ${result.elapsedMs} ms; skipped walking, normal stairs/generation follows.\n`);
  return result;
}
