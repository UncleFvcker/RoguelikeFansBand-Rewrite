// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import type { GameEventDto, ProbedMonsterDto } from "./protocol.ts";
import { latestMonsterProbe } from "./monster-probe-panel.ts";

const sheep: ProbedMonsterDto = {
  entityId: "actor-7",
  kindId: "demo.actor.sheep",
  glyph: "q",
  position: { x: 4, y: 5 },
  hp: 3,
  maxHp: 4,
  speed: 100,
  armorClass: 2,
  alignment: "neutral",
  faction: "hostile",
  resistances: [],
  statusImmunities: [],
  meleeRoutine: { blows: [] },
  abilityIds: [],
};

test("latestMonsterProbe returns the newest typed probe without grouping instances", () => {
  const events = [
    {
      kind: "test",
      messageKey: "test",
      args: {},
      outcome: {
        type: "ability-monster-probe",
        resolution: { monsters: [sheep] },
      },
    },
    { kind: "test", messageKey: "test", args: {} },
    {
      kind: "test",
      messageKey: "test",
      args: {},
      outcome: {
        type: "ability-monster-probe",
        resolution: {
          monsters: [sheep, { ...sheep, entityId: "actor-8" }],
        },
      },
    },
  ] satisfies GameEventDto[];

  assert.deepEqual(
    latestMonsterProbe(events)?.map((monster) => monster.entityId),
    ["actor-7", "actor-8"],
  );
});

test("latestMonsterProbe ignores updates without a probe result", () => {
  assert.equal(
    latestMonsterProbe([{ kind: "test", messageKey: "test", args: {} }]),
    undefined,
  );
});
