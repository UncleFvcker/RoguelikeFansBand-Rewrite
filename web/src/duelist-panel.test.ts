// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Node's built-in test runner.
import assert from "node:assert/strict";
import test from "node:test";
import { AppState } from "./app-state.ts";
import { DuelistPanel } from "./duelist-panel.ts";
import { GameSession } from "./game-session.ts";

function fixture() {
  class Element extends EventTarget {
    dataset = {}; options = []; value = ""; open = false;
    setAttribute() {}
    replaceChildren(...options) { this.options = options; this.value = options[0]?.value ?? ""; }
    showModal() { this.open = true; }
    close() { this.open = false; }
    focus() { document.activeElement = this; }
    click() { if (!this.disabled) this.dispatchEvent(new Event("click")); }
  }
  const elements = new Map();
  const document = {
    createElement: () => new Element(),
    getElementById(id) {
      if (!elements.has(id)) elements.set(id, Object.assign(new Element(), { ownerDocument: this }));
      return elements.get(id);
    },
  };
  const state = new AppState();
  state.mode = "playing";
  state.status = { mapScale: "local", player: { build: { classId: "demo.class.duelist" },
    abilities: [{ id: "mark", canCast: true, effects: [{ type: "duelist-challenge" }] }],
  }, entities: [{ id: "a", kindId: "sheep", position: { x: 2, y: 1 } }] };
  const commands = [], targets = [];
  const panel = new DuelistPanel({ document, state, localization: { format: (key, args) => key + (args ? JSON.stringify(args) : "") },
    contentName: id => id, dispatch: async command => { commands.push(command); },
    startTargeting: ability => targets.push(ability.id), beforePrompt() {},
  });
  return { state, panel, commands, targets, document, element: id => document.getElementById(id) };
}

test("challenge HUD uses projected identities, preserves unseen challenges, and separates change from clear", () => {
  const { state, panel, element, commands, targets } = fixture();
  panel.render();
  assert.equal(element("duelist-status").hidden, false);
  assert.equal(element("duelist-clear").disabled, true);
  element("duelist-mark").click();
  assert.deepEqual(targets, ["mark"]);
  state.status.player.duelistTargetId = "a";
  panel.render();
  assert.match(element("duelist-status-value").textContent, /sheep/);
  element("duelist-clear").click();
  assert.deepEqual(commands, [{ type: "clear-duelist-challenge" }]);
  state.status.entities = [];
  panel.render();
  assert.equal(element("duelist-status-value").textContent, "duelist-challenge-unseen");
  assert.equal(state.status.player.duelistTargetId, "a");
  state.status.player.duelistTargetId = null;
  state.status.player.abilities[0].canCast = false;
  state.status.player.abilities[0].unavailableReason = "duelist-heavy-armor";
  panel.render();
  assert.equal(element("duelist-mark").disabled, true);
  assert.equal(element("duelist-mark").title, "ability-unavailable-duelist-heavy-armor");
});

test("all suspended choices send only their resolver; Escape declines, busy prevents duplicate answers", () => {
  const { state, panel, element, commands } = fixture();
  const dialog = element("duelist-choice-dialog");
  for (const type of ["charge", "block-teleport", "follow-teleport", "challenge"]) {
    state.status.player.pendingDuelist = { type, targetEntityId: "a", sourceEntityId: "a", distance: 9, range: 5 };
    panel.render();
    assert.equal(dialog.open, true);
    assert.equal(state.commandBlocked, true);
    element("duelist-choice-accept").click();
    assert.deepEqual(commands.at(-1), { type: "resolve-duelist-choice", choice: type === "challenge" ? { type: "challenge", entityId: "a" } : { type: "confirm", accepted: true } });
    state.busy = true;
    panel.render();
    const count = commands.length;
    dialog.dispatchEvent(new Event("cancel", { cancelable: true }));
    assert.equal(commands.length, count);
    state.busy = false;
    panel.render();
    const cancel = new Event("cancel", { cancelable: true });
    dialog.dispatchEvent(cancel);
    assert.equal(cancel.defaultPrevented, true);
    assert.deepEqual(commands.at(-1).choice, type === "challenge" ? { type: "challenge", entityId: null } : { type: "confirm", accepted: false });
  }
  state.status.player.pendingDuelist = null;
  panel.render();
  assert.equal(dialog.open, false);
  assert.equal(state.commandBlocked, false);
});

test("free challenge keeps the selected instance across renders and excludes the mount", () => {
  const { state, panel, element } = fixture();
  state.status.player.pendingDuelist = { type: "challenge" };
  state.status.player.ridingActorId = "mount";
  state.status.entities.push({ id: "b", kindId: "sheep", position: { x: 3, y: 1 } }, { id: "mount" });
  panel.render();
  const target = element("duelist-choice-target");
  assert.deepEqual(target.options.map(option => option.value), ["a", "b"]);
  target.value = "b";
  panel.render();
  assert.equal(target.value, "b");
});

test("session rejects ordinary commands while a Duelist choice or response is pending", async () => {
  const { state } = fixture();
  state.status.player.pendingDuelist = { type: "challenge" };
  const commands = [];
  let finish;
  const session = new GameSession({ state, execute: command => { commands.push(command); return new Promise(resolve => { finish = resolve; }); },
    applyUpdate() {}, refreshBusyControls() {}, showError: error => { throw error; },
  });
  await session.dispatch({ type: "move", direction: "east" });
  await session.dispatch({ type: "clear-duelist-challenge" });
  assert.deepEqual(commands, []);
  const response = { type: "resolve-duelist-choice", choice: { type: "challenge", entityId: null } };
  const first = session.dispatch(response);
  await session.dispatch(response);
  assert.deepEqual(commands, [response]);
  finish({}); await first;
});
