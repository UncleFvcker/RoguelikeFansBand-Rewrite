// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { readFile, writeFile, rename, mkdir, unlink, rmdir } from "node:fs/promises";
import path from "node:path";

export async function runNativeSaves(ctx) {
  const { click, create, capture, snapshot, invoke, launch, stop, moveInstall, checks } = ctx;
  const driver = () => ctx.driver();
  const keyboard = () => ctx.keyboard();
  const list = async () => (await invoke("list_native_saves")).filter(s => !s.museumCheckpoint);
  const playing = () => driver().waitFor('return document.documentElement.dataset.appMode === "playing" && !document.querySelector("#save-button").disabled', "game ready");
  const save = async () => { await click("#save-button"); await playing(); };
  const step = async () => {
    const before = await snapshot();
    await keyboard().key("6");
    await driver().waitFor('return window.__TAURI_INTERNALS__.invoke("refresh_museum").then(s=>s.turn>arguments[0])', "move completed", 30000, [before.turn]);
    await playing();
  };
  const loadList = async () => { await click("#load-button"); await driver().waitFor('return document.documentElement.dataset.appMode === "load" && !document.querySelector("#session-load-refresh").disabled', "save list ready"); };
  const load = id => click(`[data-slot-id="${id}"] [data-session-load-action="load"]`);
  const decision = async choice => {
    await driver().waitFor('return document.querySelector("#save-decision-dialog").open', "unsaved progress choice");
    await click(`#save-decision-dialog button[value="${choice}"]`);
  };
  await create("便携存档一", 811);
  assert.equal(await driver().execute('return document.querySelectorAll("#load-input,input[accept*=rfbsave]").length'), 0);
  const fresh = await snapshot();
  await save();
  let saves = await list(); assert.equal(saves.length, 1);
  const firstId = saves[0].slotId;
  assert.equal(saves[0].characterName, "便携存档一");
  assert.equal(saves[0].characterLevel, 1);
  assert.equal(saves[0].stateHash, fresh.stateHash);
  await keyboard().key("s", 2); await playing();
  assert.equal((await list()).length, 1);
  await step();
  const moved = await snapshot(); assert.notEqual(moved.stateHash, fresh.stateHash);
  await loadList(); await capture("load-chinese");
  await load(firstId); await decision("cancel");
  assert.equal((await snapshot()).stateHash, moved.stateHash);
  await click("#session-load-back"); await playing();
  await loadList(); await load(firstId); await decision("discard"); await playing();
  assert.equal((await snapshot()).stateHash, fresh.stateHash);
  checks.push("Default slot, Ctrl+S overwrite, metadata, in-game load cancellation and discard");

  await keyboard().withDialog([{ accept: true, promptText: "战前备份" }], () => click("#save-as-button"));
  await playing(); saves = await list(); assert.equal(saves.length, 2);
  const copyId = saves.find(s => s.slotId !== firstId).slotId;
  await step();
  await loadList(); await load(firstId); await decision("save"); await playing();
  assert.equal((await list()).find(s => s.slotId === copyId).turn, moved.turn);
  assert.equal((await snapshot()).stateHash, fresh.stateHash);
  await step(); await save();
  assert.equal((await list()).find(s => s.slotId === firstId).turn, moved.turn, "loading binds the selected slot for subsequent saves");
  await stop(true); await launch();
  await click("#session-continue"); await playing();
  assert.equal((await snapshot()).stateHash, moved.stateHash);
  await stop(true); await launch(); await create("便携存档二", 812); await save();
  saves = await list(); assert.equal(saves.length, 3);
  const secondId = saves.find(s => s.characterName === "便携存档二").slotId;
  assert.notEqual(secondId, firstId);
  await stop(true);
  checks.push("Save-as, save before switching, loaded-slot binding, restart Continue and independent characters");

  await moveInstall(); await launch();
  await click("#session-continue"); await playing();
  assert.equal((await snapshot()).player.name, "便携存档二");
  await loadList(); await load(firstId); await playing();
  assert.equal((await snapshot()).stateHash, moved.stateHash);
  await save(); await stop(true);
  const primary = path.join(ctx.installDirectory(), "userdata/saves", `${firstId}.rfbsave`);
  await readFile(primary); // Assert that normal UI saves used the executable-relative directory.
  await writeFile(primary, "deliberately corrupt test primary");
  await launch(); await click("#session-load-game");
  await driver().waitFor('return document.querySelector("[data-slot-id=\\"'+firstId+'\\"]") !== null', "recovery list");
  assert.equal((await list()).find(s => s.slotId === firstId).status, "recoverable");
  await capture("recover-backup"); await load(firstId); await playing();
  assert.equal((await snapshot()).stateHash, moved.stateHash);
  await save();
  assert.equal((await list()).find(s => s.slotId === firstId).status, "ready");
  checks.push("Moving the complete game directory preserves profile binding and all saves; corrupt primary recovers from a valid backup");
  await loadList();
  await keyboard().withDialog(true, () => click(`[data-slot-id="${copyId}"] [data-session-load-action="delete"]`));
  await driver().waitFor('return !document.querySelector("[data-slot-id=\\"'+copyId+'\\"]")', "slot deleted");
  assert.equal((await list()).length, 2);
  await click("#session-load-back"); await playing();
  await ctx.setPreferences({ locale: "en-US" });
  await loadList(); await keyboard().viewport(390, 844);
  assert.equal(await driver().execute('const c=document.querySelector(".session-card");return c.scrollWidth<=c.clientWidth && c.getBoundingClientRect().width<=innerWidth;'), true);
  await capture("load-english-narrow"); await click("#session-load-back"); await playing();
  checks.push("In-game slot deletion and English narrow-window layout");
  await save();
  await step();
  const pending = await snapshot();
  await click("#session-exit"); await decision("cancel"); await playing();
  assert.equal((await snapshot()).stateHash, pending.stateHash);
  const beforeFailure = await readFile(primary);
  const blocked = primary + ".bak3", heldBackup = blocked + ".test-held";
  let hadBackup = false;
  try { await rename(blocked, heldBackup); hadBackup = true; }
  catch (error) { if (error.code !== "ENOENT") throw error; }
  await mkdir(blocked); await writeFile(path.join(blocked, "blocker"), "test write failure");
  try {
    await click("#save-exit-button");
    await driver().waitFor('return document.querySelector("#message-list").textContent.includes("could not be updated")', "save failure reported");
    await playing();
    assert.equal((await snapshot()).stateHash, pending.stateHash);
    assert.deepEqual(await readFile(primary), beforeFailure, "failed save preserves committed primary");
  } finally {
    await unlink(path.join(blocked, "blocker")); await rmdir(blocked);
    if (hadBackup) await rename(heldBackup, blocked);
  }
  await driver().execute('document.activeElement?.blur(); return true;');
  await keyboard().key("x", 2); await ctx.waitExit();
  checks.push("Closing with unsaved progress can be cancelled; write failure preserves the save and keeps the game open; Ctrl+X saves then exits");
}
