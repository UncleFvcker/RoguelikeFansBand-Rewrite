// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import { NativeSaveCommands, nativeSaveErrorKey } from "./save-panel.ts";

test("save shortcuts await cancellation before locking the settled game", async t => {
  for (const action of ["shortcut", "save-exit"]) await t.test(action, async () => {
    const boundary = Promise.withResolvers(), operation = Promise.withResolvers();
    let busy = false, revision = 1, stopRequested = false, exited = false;
    const calls = [], errors = [];
    const summary = { slotId: "slot", slotName: "checkpoint", status: "ready", turn: null, savedAt: null };
    const storageOperation = async () => { calls.push({ revision, busy }); return operation.promise; };
    const panel = new NativeSaveCommands({
      storage: { save: storageOperation },
      isGameBusy: () => busy, setGameBusy: value => { busy = value; },
      beforeSessionAccess: async () => { stopRequested = true; await boundary.promise; },
      announce() {},
      logError: error => errors.push(error),
    });
    busy = true;
    const trigger = () => panel.saveFromShortcut(() => "checkpoint", action === "save-exit" ? async () => { assert.equal(busy, false); exited = true; } : undefined);
    void trigger(); void trigger();
    await Promise.resolve();
    assert.equal(stopRequested, true);
    assert.deepEqual(calls, []);
    revision = 2; busy = false; boundary.resolve();
    for (let i = 0; i < 6; i++) await Promise.resolve();
    assert.deepEqual(calls, [{ revision: 2, busy: true }]);
    assert.equal(exited, false, "exit waits for successful storage");
    operation.resolve(summary);
    for (let i = 0; i < 6; i++) await Promise.resolve();
    assert.equal(busy, false);
    assert.deepEqual(errors, []);
    assert.equal(revision, 2);
    assert.equal(exited, action === "save-exit");
  });
});

test("save-and-exit never exits after cancelled naming, failed settlement or a storage failure", async () => {
  for (const failure of ["name", "settlement", "storage"]) {
    let busy = false, writes = 0, exits = 0;
    const panel = new NativeSaveCommands({ storage: { save: async () => { writes++; throw new Error("write failed"); } },
      isGameBusy: () => busy, setGameBusy: value => { busy = value; },
      beforeSessionAccess: async () => { if (failure === "settlement") throw new Error("cancel failed"); },
      announce() {}, logError() {},
    });
    await panel.saveFromShortcut(() => failure === "name" ? null : "checkpoint", async () => { exits++; });
    assert.equal(writes, failure === "storage" ? 1 : 0);
    assert.equal(exits, 0);
    assert.equal(busy, false);
  }
});

test("native save panel preserves actionable error message categories", () => {
  assert.equal(nativeSaveErrorKey("native-save-name-invalid"), "native-save-error-name-invalid");
  assert.equal(nativeSaveErrorKey("native-save-not-found"), "native-save-error-not-found");
  assert.equal(nativeSaveErrorKey("native-save-invalid"), "native-save-error-corrupt");
  assert.equal(nativeSaveErrorKey("native-save-read"), "native-save-error-read");
  assert.equal(nativeSaveErrorKey("native-save-write"), "native-save-error-write");
  assert.equal(nativeSaveErrorKey("native-save-lock"), "native-save-error-unavailable");
  assert.equal(nativeSaveErrorKey("unexpected"), "native-save-error-internal");
  assert.equal(nativeSaveErrorKey("museum-collection-stale"), "museum-error-stale");
  assert.equal(nativeSaveErrorKey("museum-character-stale"), "museum-error-character-stale");
  assert.equal(nativeSaveErrorKey("museum-profile-mismatch"), "museum-error-profile");
  assert.equal(nativeSaveErrorKey("museum-unbound-save"), "museum-error-unbound");
});

test("ordinary saves need no name prompt while save-as explicitly allocates a slot", async () => {
  const calls = [], saved = [];
  const panel = new NativeSaveCommands({
    storage: { save: async (...args) => { calls.push(args); return { slotId: "one", slotName: "Hero" }; } },
    beforeSessionAccess: async () => {}, isGameBusy: () => false, setGameBusy() {}, announce() {},
    onSaved: summary => saved.push(summary.slotId),
  });
  assert.equal(await panel.saveFromShortcut(), true);
  assert.equal(await panel.saveFromShortcut(() => " Other ", undefined, true), true);
  assert.deepEqual(calls, [[undefined, undefined, false], ["Other", undefined, true]]);
  assert.deepEqual(saved, ["one", "one"]);
});

test("a write failure announces its localized reason and never runs the exit action", async () => {
  const messages = [];
  let exited = false;
  const panel = new NativeSaveCommands({
    storage: { save: async () => { throw { code: "native-save-backup-remove" }; } },
    beforeSessionAccess: async () => {}, isGameBusy: () => false, setGameBusy() {}, logError() {},
    announce: (...args) => messages.push(args),
  });
  assert.equal(await panel.saveFromShortcut(undefined, async () => { exited = true; }), false);
  assert.deepEqual(messages, [["native-save-error-write", undefined, "error"]]);
  assert.equal(exited, false);
});
