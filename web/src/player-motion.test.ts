// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Node's built-in TypeScript test runner.
import assert from "node:assert/strict";
import test from "node:test";
import { canAnimatePlayerStep, PLAYER_STEP_MS, CONTINUOUS_STEP_MS, playerStepPosition, playerStepProgress,
  PLAYER_MELEE_MS, playerMeleeAttack, playerMeleeOffset, meleeEffectTarget, meleeImpact } from "./player-motion.ts";
import { computeCameraOffset } from "./camera.ts";

const before = { worldId: "world", floorId: "floor", mapScale: "local", entities: [],
  player: { id: "player", position: { x: 8, y: 8 }, hp: 10 } };
const update = () => ({ ...before, events: [],
  player: { ...before.player, position: { x: 9, y: 8 } } });
const move = { type: "move", direction: "east" };

test("only successful local adjacent movement starts animation", () => {
  assert.equal(canAnimatePlayerStep(before, update(), move), true);
  assert.equal(canAnimatePlayerStep(before, { ...update(), player: before.player }, move), false);
  for (const type of ["run", "continue-run", "auto-explore", "continue-auto-explore", "travel-local", "travel-unknown-item", "auto-get"]) {
    assert.equal(canAnimatePlayerStep(before, update(), { type }), true, type);
    assert.equal(canAnimatePlayerStep(before, { ...update(), player: before.player }, { type }), false, `${type}: opening/pickup/blocked`);
  }
  for (const type of ["rest", "cancel-run", "cancel-auto-explore", "cast-ability"])
    assert.equal(canAnimatePlayerStep(before, update(), { type }), false);
  assert.equal(canAnimatePlayerStep(undefined, update(), move), false);
  for (const patch of [{ floorId: "another-floor" }, { player: { ...update().player, id: "another-player" } },
    { mapScale: "world" }, { mapTranslation: { x: 1, y: 0 } },
    { player: { ...before.player, position: { x: 20, y: 8 } } },
    { player: { ...update().player, hp: 0 } }]) {
    assert.equal(canAnimatePlayerStep(before, { ...update(), ...patch }, move), false);
  }
});

test("continuous steps keep uniform speed and the exact turn corner without overshoot", () => {
  const a = { x: 8, y: 8 }, corner = { x: 9, y: 8 }, b = { x: 9, y: 9 };
  assert.equal(playerStepProgress(CONTINUOUS_STEP_MS / 2, true), 0.5);
  assert.deepEqual(playerStepPosition(a, corner, CONTINUOUS_STEP_MS / 2, true), { x: 8.5, y: 8 });
  assert.deepEqual(playerStepPosition(a, corner, CONTINUOUS_STEP_MS, true), corner);
  assert.deepEqual(playerStepPosition(corner, b, 0, true), corner);
  assert.deepEqual(playerStepPosition(corner, b, CONTINUOUS_STEP_MS / 2, true), { x: 9, y: 8.5 });
  assert.deepEqual(playerStepPosition(corner, b, 5000, true), b);
});

test("adjacent teleports and monster displacement do not masquerade as walking", () => {
  for (const event of [
    { kind: "item.use-teleported" },
    { kind: "ability.teleport", outcome: { type: "ability-teleport", resolution: {} } },
    { kind: "monster.blinked-target", outcome: { type: "monster-displacement", resolution: { actorId: "player" } } },
  ]) assert.equal(canAnimatePlayerStep(before, { ...update(), events: [event] }, move), false);
  assert.equal(canAnimatePlayerStep(before, { ...update(), events: [
    { kind: "monster.teleported", outcome: { type: "monster-displacement", resolution: { actorId: "monster" } } },
  ] }, move), true);
});

test("single-step motion and its camera share a bounded fractional position", () => {
  const from = { x: 8, y: 8 }, to = { x: 9, y: 9 };
  assert.deepEqual(playerStepPosition(from, to, 0), from);
  assert.deepEqual(playerStepPosition(from, to, PLAYER_STEP_MS), to);
  assert.deepEqual(playerStepPosition(from, to, 5000), to);
  const displayed = playerStepPosition(from, to, 25);
  assert.ok(displayed.x > from.x && displayed.x < to.x);
  assert.equal(displayed.x, displayed.y);
  const camera = computeCameraOffset({ mode: "player-centered", focus: displayed,
    worldWidth: 1000, worldHeight: 1000, viewportWidth: 420, viewportHeight: 420 });
  assert.equal(camera.x + (displayed.x + 0.5) * 28, 210);
  assert.equal(camera.y + (displayed.y + 0.5) * 28, 210);
});

test("melee uses the actual pre-attack target once, including misses, kills and natural attacks", () => {
  const prior = { ...before, entities: [{ id: "victim", position: { x: 9, y: 8 } }] };
  for (const kind of ["combat.hit", "combat.miss", "combat.slay", "mutation.melee-hit", "mutation.melee-miss", "mutation.melee-slay"]) {
    const attack = { kind, args: { attackTarget: "victim" } };
    const after = { ...prior, entities: [], events: [attack, attack, attack] };
    assert.deepEqual(playerMeleeAttack(prior, after, move)?.direction, { x: 1, y: 0 }, kind);
    assert.equal(canAnimatePlayerStep(prior, after, move), false, "bump attacks do not move the logical player");
    after.entities = [{ id: "victim", position: { x: 12, y: 8 } }];
    assert.deepEqual(playerMeleeAttack(prior, after, move)?.direction, { x: 1, y: 0 }, "knockback does not redirect the swing");
  }
});

test("melee rejects non-attacks, remote targets and scene discontinuities", () => {
  const prior = { ...before, entities: [{ id: "victim", position: { x: 9, y: 8 } }] };
  const attack = { kind: "combat.hit", args: { attackTarget: "victim" } };
  const after = { ...prior, events: [attack] };
  for (const kind of ["combat.monster-hit", "projectile.hit", "ability.hit", "terrain.blocked"]) {
    assert.equal(playerMeleeAttack(prior, { ...after, events: [{ ...attack, kind }] }, move), undefined);
  }
  assert.equal(playerMeleeAttack(prior, { ...after, events: [] }, move), undefined);
  assert.equal(playerMeleeAttack(undefined, after, move), undefined);
  for (const patch of [{ floorId: "other" }, { mapScale: "world" }, { mapTranslation: { x: 1, y: 0 } },
    { player: { ...prior.player, id: "other" } }, { player: { ...prior.player, hp: 0 } },
    { player: { ...prior.player, position: { x: 9, y: 8 } } },
    { events: [attack, { kind: "ability.teleport", outcome: { type: "ability-teleport" } }] },
    { events: [attack, { kind: "monster.blinked-target", outcome: { type: "monster-displacement", resolution: { actorId: "player" } } }] }]) {
    assert.equal(playerMeleeAttack(prior, { ...after, ...patch }, move), undefined);
  }
  const far = { ...prior, entities: [{ id: "victim", position: { x: 15, y: 8 } }] };
  assert.equal(playerMeleeAttack(far, { ...far, events: [attack] }, move), undefined);
});

test("an unseen melee victim uses only the commanded direction after a confirmed attack", () => {
  const after = { ...before, events: [{ kind: "combat.miss", args: { attackTarget: "unseen" } }] };
  assert.deepEqual(playerMeleeAttack(before, after, { type: "move", direction: "north-west" })?.direction, { x: -1, y: -1 });
  assert.equal(playerMeleeAttack(before, after, { type: "rest", turns: 1 }), undefined);
  assert.equal(playerMeleeAttack(before, { ...after, events: [] }, move), undefined);
});

test("all eight melee directions lunge at most 0.2 tiles and return exactly to the standing tile", () => {
  for (const x of [-1, 0, 1]) for (const y of [-1, 0, 1]) {
    if (x === 0 && y === 0) continue;
    const direction = { x, y };
    assert.deepEqual(playerMeleeOffset(direction, 0), { x: 0, y: 0 });
    assert.deepEqual(playerMeleeOffset(direction, PLAYER_MELEE_MS), { x: 0, y: 0 });
    assert.deepEqual(playerMeleeOffset(direction, 5000), { x: 0, y: 0 });
    const contact = playerMeleeOffset(direction, 45);
    assert.ok(Math.abs(Math.hypot(contact.x, contact.y) - 0.2) < 1e-12);
    assert.deepEqual(playerMeleeOffset(direction, 69), contact);
    for (const time of [1, 25, 70, 110, 149]) {
      const offset = playerMeleeOffset(direction, time);
      assert.ok(Math.hypot(offset.x, offset.y) <= 0.2 + 1e-12);
      assert.equal(Math.sign(offset.x), x);
      assert.equal(Math.sign(offset.y), y);
    }
  }
});

test("multi-blow feedback combines only the selected victim's real outcomes", () => {
  const prior = { ...before, entities: [{ id: "victim", position: { x: 9, y: 8 } }] };
  const miss = { kind: "combat.miss", args: { attackTarget: "victim" } };
  const after = { ...prior, events: [miss] };
  assert.equal(playerMeleeAttack(prior, after, move).outcome, "miss");
  after.events.push({ ...miss, kind: "combat.hit" });
  assert.equal(playerMeleeAttack(prior, after, move).outcome, "hit");
  after.events.push({ kind: "combat.slay", args: { attackTarget: "other" } });
  assert.equal(playerMeleeAttack(prior, after, move).outcome, "hit");
  after.events.push({ ...miss, kind: "combat.slay" });
  assert.equal(playerMeleeAttack(prior, after, move).outcome, "kill");
});

test("impact visuals never expose unseen, fuzzy or replacement monsters", () => {
  const target = { index: 1, x: 9, y: 8, actorId: "victim", actorKindId: "monster",
    terrainId: "floor", visibility: "visible", light: { color: 0xffffff, intensity: 1 } };
  const attack = { direction: { x: 1, y: 0 }, targetId: "victim", outcome: "hit" };
  assert.equal(meleeEffectTarget(attack, target, target), target);
  assert.equal(meleeEffectTarget({ ...attack, outcome: "miss" }, target, target), undefined);
  assert.equal(meleeEffectTarget(attack, target, { ...target, actorId: "other" }), undefined);
  const dead = { ...target, actorId: undefined, actorKindId: undefined };
  const kill = { ...attack, outcome: "kill" };
  assert.equal(meleeEffectTarget(kill, target, dead), target, "death keeps only the previously drawn appearance");
  for (const visibility of ["remembered", "hidden"]) {
    assert.equal(meleeEffectTarget(kill, { ...target, visibility }, dead), undefined);
    assert.equal(meleeEffectTarget(kill, target, { ...dead, visibility }), undefined);
  }
  assert.equal(meleeEffectTarget(kill, { ...target, actorGlyph: "?" }, dead), undefined);
  assert.equal(meleeEffectTarget(kill, undefined, dead), undefined);
});

test("contact feedback starts at contact and fully settles with the 150 ms player animation", () => {
  assert.deepEqual(meleeImpact(0), { flash: 0, corpseAlpha: 1, cameraPixels: 0 });
  assert.equal(meleeImpact(44).flash, 0);
  assert.equal(meleeImpact(45).flash, 1);
  const middle = meleeImpact(90);
  assert.ok(middle.flash > 0 && middle.flash < 1);
  assert.ok(middle.corpseAlpha > 0 && middle.corpseAlpha < 1);
  assert.ok(middle.cameraPixels > 0 && middle.cameraPixels <= 2);
  for (const elapsed of [PLAYER_MELEE_MS, 5000]) {
    assert.deepEqual(meleeImpact(elapsed), { flash: 0, corpseAlpha: 0, cameraPixels: 0 });
  }
});
