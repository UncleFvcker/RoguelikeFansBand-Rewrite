// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.
import assert from "node:assert/strict";
import test from "node:test";
import { CombatSummaryPanel } from "./combat-summary.ts";

function harness() {
  const document = { createElement() {
    return { ownerDocument: document, children: [], textContent: "", setAttribute() {},
      append(...children) { this.children.push(...children); },
      replaceChildren(...children) { this.textContent = ""; this.children = children; } };
  } };
  const list = document.createElement(), title = document.createElement(), health = document.createElement();
  const panel = new CombatSummaryPanel({ list, targetTitle: title, targetHealth: health,
    contentName: id => id, localization: { format: (key, args) => args ? `${key}:${args.hp ?? 0}/${args.max}` : key },
    formatEvent: event => event.messageKey });
  return { panel, list, title, health, hp: () => health.children[1]?.children[0]?.value };
}
const monster = (id, hp = 12) => ({ id, kindId: "same-kind", hp, maxHp: 20, position: { x: Number(id), y: 1 } });
const state = (entities, events = [], floorId = "floor-1") => ({ entities, events, turn: 1, mapScale: "local", floorId });
const hit = (id, type = "damage") => ({ kind: "combat.hit", messageKey: "hit", args: { attackTarget: id }, outcome: { type } });

test("selected monster takes priority over the last actual attack and refreshes HP by instance ID", () => {
  const h = harness(), a = monster("1"), b = monster("2", 17);
  h.panel.renderTarget(state([a, b]), { type: "entity", entityId: b.id });
  h.panel.observe(state([a, b], [hit(a.id)]), [a, b]);
  assert.equal(h.title.textContent, "combat-target-selected");
  assert.equal(h.hp(), 17);
  h.panel.renderTarget(state([monster("1", 6), b]), undefined);
  assert.equal(h.title.textContent, "combat-target-last");
  assert.equal(h.hp(), 6);
  h.panel.observe(state([monster("1", 5), b], [{ kind: "combat.monster-hit", messageKey: "hit", args: { target: "player" } }]), [a, b]);
  assert.equal(h.hp(), 5, "an incoming attack does not change the tracked monster");
});

test("misses, throws and multi-target damage use the supplied collision ID, never a name match", () => {
  const h = harness(), a = monster("1", 8), b = monster("2", 19);
  h.panel.observe(state([a, b], [{ ...hit(a.id), kind: "combat.projectile-miss", outcome: undefined }]), [a, b]);
  assert.equal(h.hp(), 8);
  h.panel.observe(state([a, b], [hit(a.id), hit(b.id)]), [a, b]);
  assert.equal(h.hp(), 19);
  h.panel.renderTarget(state([a, b]), { type: "position", position: a.position });
  assert.equal(h.hp(), 8);
});

test("confirmed kills show zero, disappearing monsters do not retain stale health, and sessions clear tracking", () => {
  const h = harness(), a = monster("1");
  h.panel.observe(state([], [hit(a.id), hit(a.id, "death")]), [a]);
  assert.equal(h.hp(), 0);
  h.panel.observe(state([a], [hit(a.id, "death")]), [a]);
  assert.equal(h.hp(), 12, "a surviving or reborn actor wins over a death message");
  h.panel.renderTarget(state([]), undefined);
  assert.equal(h.health.textContent, "combat-target-none");
  h.panel.renderTarget(state([a], [], "floor-2"), undefined);
  assert.equal(h.health.textContent, "combat-target-none", "floor-local IDs must not carry between floors");
  h.panel.observe(state([a], [hit(a.id)], "floor-2"), [a]);
  h.panel.clear();
  assert.equal(h.health.textContent, "combat-target-none");
  assert.equal(h.list.children[0].className, "combat-summary-empty");
});
