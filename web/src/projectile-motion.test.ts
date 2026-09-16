// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Node's built-in TypeScript test runner.
import assert from "node:assert/strict";
import test from "node:test";
import { projectileFlights, projectilePosition, projectileArrival, projectileAreaFrame, projectileColor, PROJECTILE_IMPACT_MS } from "./projectile-motion.ts";
import { meleeEffectTarget } from "./player-motion.ts";

const start = (kind = "arrow") => ({ kind: "animation.projectile-start", messageKey: "",
  args: { visualKind: kind, source: "source", damageType: "fire" } });
const trace = (x = 4) => ({ origin: { x: 0, y: 0 }, impact: { x, y: 0 }, landing: { x, y: 0 },
  traversed: Array.from({ length: x }, (_, i) => ({ x: i + 1, y: 0 })) });
const end = (t = trace()) => ({ kind: "animation.projectile-end", args: {}, trace: t });
const hit = (kind, id, x) => ({ kind, args: { attackTarget: id, source: "source" }, trace: trace(x) });

test("identical consecutive shots remain distinct; piercing hits and death share one flight", () => {
  const events = [start(), hit("combat.projectile-hit", "a", 2), hit("combat.projectile-slay", "a", 2),
    hit("combat.projectile-hit", "b", 4), end(), start(), hit("combat.projectile-hit", "b", 4), end()];
  const flights = projectileFlights(events);
  assert.equal(flights.length, 2);
  assert.deepEqual(flights[0].hits.map(h => [h.targetId, h.outcome]), [["a", "kill"], ["b", "hit"]]);
  assert.ok(flights[0].hits[0].at < flights[0].hits[1].at);
  assert.ok(flights[1].delay > flights[0].delay);
  assert.deepEqual(flights[0].path, flights[1].path);
});

test("empty shots, misses and wall impacts still fly; unrelated beams are not guessed into bolts", () => {
  for (const kind of ["arrow", "throw", "bolt"]) {
    const t = { ...trace(), landing: { x: 3, y: 0 } };
    const [flight] = projectileFlights([start(kind), end(t)]);
    assert.equal(flight.kind, kind);
    assert.deepEqual(flight.path.at(-1), t.impact, "use collision, not ammunition landing");
    assert.deepEqual(flight.hits, []);
    assert.equal(flight.wallImpact, true);
  }
  assert.deepEqual(projectileFlights([hit("ability.hit", "a", 4)]), []);
  assert.deepEqual(projectileFlights([start("beam"), end()]), []);
  assert.deepEqual(projectileFlights([start()]), []);
});

test("nested on-hit bolts and reflected traces follow the incoming flight without an unbounded queue", () => {
  const reflected = { kind: "combat.bolt-reflected-hit", args: {},
    trace: { origin: { x: 4, y: 0 }, traversed: [{ x: 3, y: 1 }, { x: 2, y: 2 }],
      impact: { x: 2, y: 2 }, landing: { x: 2, y: 2 } } };
  const flights = projectileFlights([start("bolt"), start("bolt"), hit("ability.hit", "a", 4), end(), reflected, end()]);
  assert.equal(flights.length, 3);
  assert.equal(flights[0].hits.length, 0, "child outcomes do not become parent hits");
  assert.ok(flights[1].delay >= flights[0].duration);
  assert.deepEqual(flights[2].path, [reflected.trace.origin, ...reflected.trace.traversed]);
  assert.ok(Math.max(...flights.map(f => f.delay + f.duration + PROJECTILE_IMPACT_MS)) <= 400.00001);
});

test("flight sampling follows actual segments, clamps endpoints and keeps fractional movement", () => {
  const t = { origin: { x: 0, y: 0 }, traversed: [{ x: 1, y: 0 }, { x: 2, y: 1 }],
    impact: { x: 2, y: 1 }, landing: { x: 2, y: 1 } };
  const [flight] = projectileFlights([start(), end(t)]);
  assert.deepEqual(projectilePosition(flight, -1).position, t.origin);
  assert.deepEqual(projectilePosition(flight, flight.duration / 4).position, { x: 0.5, y: 0 });
  assert.deepEqual(projectilePosition(flight, flight.duration * 0.75).position, { x: 1.5, y: 0.5 });
  assert.deepEqual(projectilePosition(flight, 999).position, t.impact);
});

test("projectile death feedback retains only visible known occupants", () => {
  const target = { actorId: "a", actorKindId: "monster", visibility: "visible" };
  const hit = { targetId: "a", outcome: "kill" };
  assert.equal(meleeEffectTarget(hit, target, { visibility: "visible" }), target);
  assert.equal(meleeEffectTarget(hit, target, { visibility: "remembered" }), undefined);
  assert.equal(meleeEffectTarget(hit, { ...target, actorGlyph: "?" }, { visibility: "visible" }), undefined);
  assert.equal(meleeEffectTarget(hit, undefined, { visibility: "visible" }), undefined);
});

const shape = (kind, positions, center = { x: 3, y: 0 }) => ({
  kind: kind === "beam" ? "ability.beam-damage" : "ability.area-damage", args: { target: "source" },
  outcome: { type: kind === "beam" ? "ability-beam-damage" : "ability-area-damage",
    resolution: { center, radius: 2, affectedPositions: positions, damageType: "fire", baseRawDamage: 10, targetCount: 2 } },
});

test("beam hits follow known occupants along the beam, not the shared impact at its end", () => {
  const t = trace(6);
  const positions = new Map([["near", { x: 2, y: 0 }], ["far", { x: 5, y: 0 }]]);
  const [beam] = projectileFlights([start("beam"), shape("beam", t.traversed),
    hit("ability.hit", "near", 6), hit("ability.hit", "far", 6), hit("ability.slay", "far", 6),
    hit("ability.hit", "unknown", 6), end(t)], positions);
  assert.deepEqual(beam.hits.map(h => [h.position.x, h.outcome]), [[2, "hit"], [5, "kill"]]);
  assert.ok(beam.hits[0].at < beam.hits[1].at);
  assert.ok(beam.duration <= 100);
  assert.deepEqual(beam.affectedPositions, t.traversed);
});

test("ball flies to the landing center and expands only over the supplied affected cells", () => {
  const center = { x: 3, y: 0 }, outer = { x: 3, y: 2 };
  const affected = [center, { x: 3, y: 1 }, outer]; // Wall side x=4 excluded by core.
  const t = { ...trace(4), landing: center, traversed: trace(3).traversed };
  const [ball] = projectileFlights([start("ball"), shape("ball", affected, center),
    hit("ability.hit", "center", 4), hit("ability.hit", "outer", 4), end(t)],
    new Map([["center", center], ["outer", outer]]));
  assert.deepEqual(ball.path.at(-1), center);
  assert.deepEqual(projectilePosition(ball, ball.travelDuration).position, center);
  assert.deepEqual(ball.affectedPositions, affected);
  assert.equal(ball.wallImpact, false);
  assert.equal(ball.hits[0].at, ball.travelDuration);
  assert.equal(ball.hits[1].at, ball.duration);
  assert.equal(projectileArrival(ball, outer), ball.hits[1].at);
});

test("self-centered and radius-zero balls burst without a fabricated flight", () => {
  const center = { x: 0, y: 0 }, t = { origin: center, impact: center, landing: center, traversed: [] };
  const [ball] = projectileFlights([start("ball"), shape("ball", [center], center), end(t)]);
  assert.equal(ball.travelDuration, 0);
  assert.equal(ball.duration, 0, "a single-cell burst needs no empty expansion wait");
  assert.deepEqual(ball.path, [center]);
  assert.deepEqual(projectilePosition(ball, 0).position, center);
  assert.equal(projectileArrival(ball, center), 0);
});

test("multiple ball centers stay separate and scaled expansion stays synchronized with hits", () => {
  const events = [], positions = new Map();
  for (const x of [20, 22]) {
    const center = { x, y: 0 }, outer = { x, y: 2 }, id = `actor-${x}`;
    positions.set(id, outer);
    events.push(start("ball"), shape("ball", [center, outer], center), hit("ability.hit", id, x), end(trace(x)));
  }
  const balls = projectileFlights(events, positions);
  assert.deepEqual(balls.map(ball => ball.center.x), [20, 22]);
  assert.ok(balls[1].delay > 0);
  assert.ok(Math.max(...balls.map(ball => ball.delay + ball.duration + PROJECTILE_IMPACT_MS)) <= 400.00001);
  for (const ball of balls) assert.ok(Math.abs(projectileArrival(ball, ball.hits[0].position) - ball.hits[0].at) < 1e-8);
});

const view = (flight, visible = flight.affectedPositions) => ({ flight, color: projectileColor(flight.damageType),
  blast: visible.map(position => ({ position, at: projectileArrival(flight, position) })) });

test("storm swirls persist after one impact, use only supplied visible cells and finish cleanly", () => {
  const center = { x: 3, y: 0 }, outer = { x: 3, y: 2 };
  const [storm] = projectileFlights([start("storm"), shape("storm", [center, outer], center),
    hit("ability.hit", "a", 3), end(trace(3))], new Map([["a", center]]));
  assert.equal(storm.hits.length, 1, "visual swirl creates no extra damage feedback");
  assert.equal(storm.hits[0].at, storm.travelDuration);
  const visible = view(storm, [center]);
  assert.deepEqual(projectileAreaFrame([visible], storm.travelDuration - 1), []);
  const [sample] = projectileAreaFrame([visible], storm.duration - 1);
  assert.deepEqual(sample.position, center);
  assert.ok(Number.isFinite(sample.angle));
  assert.ok(sample.alpha > 0 && sample.alpha < 0.5);
  assert.deepEqual(projectileAreaFrame([visible], storm.duration + PROJECTILE_IMPACT_MS), []);
  const unknownName = start("ball");
  unknownName.args.source = "not-a-real-storm";
  const area = shape("ball", [center], center);
  area.args.target = unknownName.args.source;
  assert.equal(projectileFlights([unknownName, area, end(trace(3))])[0].kind, "ball", "do not classify by name substrings");
});

test("twenty direct meteors preserve centers and staggered impacts without invented horizontal paths", () => {
  const events = [];
  for (let i = 0; i < 20; i++) {
    const center = { x: i + 1, y: 0 };
    events.push(start("meteor"), shape("meteor", [center, { x: center.x, y: 1 }], center), end(trace(center.x)));
  }
  const flights = projectileFlights(events);
  assert.equal(flights.length, 20);
  assert.equal(new Set(flights.map(flight => flight.delay)).size, 20);
  for (const flight of flights) {
    assert.deepEqual(flight.path, [flight.center]);
    assert.deepEqual(projectilePosition(flight, 0).position, flight.center);
    assert.equal(projectileArrival(flight, flight.center), flight.travelDuration);
  }
  assert.ok(Math.max(...flights.map(f => f.delay + f.duration + PROJECTILE_IMPACT_MS)) <= 400.00001);
});

test("overlapping blast frames draw one bounded sample per visible affected cell", () => {
  const center = { x: 3, y: 0 };
  const [ball] = projectileFlights([start("ball"), shape("ball", [center], center), end(trace(3))]);
  const one = projectileAreaFrame([view(ball)], ball.travelDuration + 20);
  const many = projectileAreaFrame(Array.from({ length: 20 }, () => view(ball)), ball.travelDuration + 20);
  assert.deepEqual(many, one, "overlap does not multiply brightness or draw count");
  assert.equal(many.length, 1);
  assert.deepEqual(projectileAreaFrame([view(ball)], ball.duration + PROJECTILE_IMPACT_MS), []);
});
