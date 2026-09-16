// SPDX-License-Identifier: MPL-2.0
import type { GameEventDto, Position, ProjectileTraceDto } from "./protocol";
import type { RenderCell } from "./renderer-backend";

export const PROJECTILE_IMPACT_MS = 80;
export interface ProjectileHit {
  targetId: string;
  position: Position;
  outcome: "hit" | "kill";
  at: number;
  previous?: RenderCell;
  target?: RenderCell;
}
export interface ProjectileFlight {
  kind: "arrow" | "throw" | "bolt" | "beam" | "ball" | "storm" | "meteor";
  source: string;
  damageType: string;
  path: Position[];
  affectedPositions: Position[];
  center: Position;
  spreadDistance: number;
  travelDuration: number;
  spreadDuration: number;
  duration: number;
  delay: number;
  wallImpact: boolean;
  hits: ProjectileHit[];
}
const same = (a: Position, b: Position) => a.x === b.x && a.y === b.y;

/** Core delimiters distinguish identical volleys from one piercing projectile. */
export function projectileFlights(events: readonly GameEventDto[],
  targetPositions: ReadonlyMap<string, Position> = new Map()): ProjectileFlight[] {
  const stack: { start: GameEventDto; events: GameEventDto[]; children: ProjectileFlight[] }[] = [];
  const flights: ProjectileFlight[] = [];
  let volley = 0;
  for (const event of events) {
    if (event.kind === "animation.projectile-start") {
      stack.push({ start: event, events: [], children: [] });
    } else if (event.kind === "animation.projectile-end") {
      const group = stack.pop();
      if (!group || !event.trace) continue;
      const kind = group.start.args.visualKind;
      if (kind !== "arrow" && kind !== "throw" && kind !== "bolt" && kind !== "beam" &&
          kind !== "ball" && kind !== "storm" && kind !== "meteor") continue;
      const isArea = kind === "ball" || kind === "storm" || kind === "meteor";
      const shape = group.events.find(value => value.args.target === group.start.args.source &&
        value.outcome?.type === (isArea ? "ability-area-damage" : "ability-beam-damage"))?.outcome;
      const area = isArea && shape?.type === "ability-area-damage" ? shape.resolution : undefined;
      const beam = kind === "beam" && shape?.type === "ability-beam-damage" ? shape.resolution : undefined;
      if ((isArea && !area) || (kind === "beam" && !beam)) continue;
      const center = area?.center ?? event.trace.impact;
      const path = kind === "meteor" ? [center] : projectilePath(event.trace, area?.center);
      const travelDuration = kind === "meteor" ? 60 : path.length === 1 ? 0 : Math.min(kind === "beam" ? 100 : kind === "bolt" ? 220 : 200,
        Math.max(kind === "beam" ? 60 : kind === "bolt" ? 100 : 80, (path.length - 1) * 18));
      const spreadDuration = area?.affectedPositions.some(position => !same(position, center)) ? 110 : 0;
      const duration = travelDuration + spreadDuration + (kind === "storm" ? 140 : 0);
      const hits: ProjectileHit[] = [];
      const flight: ProjectileFlight = { kind, source: group.start.args.source ?? "",
        damageType: group.start.args.damageType ?? "physical", path, center,
        affectedPositions: area?.affectedPositions ?? beam?.affectedPositions ?? [],
        spreadDistance: Math.max(1, ...(area?.affectedPositions ?? []).map(cell => Math.hypot(cell.x - center.x, cell.y - center.y))),
        travelDuration, spreadDuration, duration, delay: 0,
        wallImpact: !isArea && !same(event.trace.impact, event.trace.landing), hits };
      const prefix = kind === "arrow" ? "combat.projectile-" : kind === "throw" ? "combat.throw-" : "ability.";
      for (const hit of group.events) {
        if (!hit.trace || !same(hit.trace.origin, event.trace.origin) || !hit.args.attackTarget ||
            ((kind === "bolt" || kind === "beam" || isArea) && hit.args.source !== group.start.args.source) ||
            ![prefix + "hit", prefix + "slay"].includes(hit.kind)) continue;
        const position = area || beam ? targetPositions.get(hit.args.attackTarget) : hit.trace.impact;
        if (!position || !(area || beam ? flight.affectedPositions : path).some(cell => same(cell, position))) continue;
        const existing = hits.find(value => value.targetId === hit.args.attackTarget && same(value.position, position));
        if (existing) {
          if (hit.kind.endsWith("slay")) existing.outcome = "kill";
        } else hits.push({ targetId: hit.args.attackTarget, position,
          outcome: hit.kind.endsWith("slay") ? "kill" : "hit",
          at: projectileArrival(flight, position) });
      }
      flight.wallImpact &&= hits.length === 0;
      // Reflection has its own authoritative return trace; never draw a straight line back by guesswork.
      for (const reflected of group.events) {
        if (kind === "bolt" && reflected.kind.startsWith("combat.bolt-reflected") && reflected.trace) {
          const reflectedPath = projectilePath(reflected.trace);
          const reflectedDuration = Math.min(140, Math.max(80, (reflectedPath.length - 1) * 18));
          group.children.push({ ...flight, path: reflectedPath, duration: reflectedDuration, travelDuration: reflectedDuration,
            wallImpact: !same(reflected.trace.impact, reflected.trace.landing), hits: [] });
        }
      }
      for (const child of group.children) child.delay += duration;
      const sequence = [flight, ...group.children];
      const parent = stack.at(-1);
      if (parent) parent.children.push(...sequence);
      else {
        const stagger = volley++ * 25;
        for (const part of sequence) part.delay += stagger;
        flights.push(...sequence);
      }
    } else stack.at(-1)?.events.push(event);
  }
  // Scale the entire volley together, preserving distinct centers and child ordering.
  const span = Math.max(0, ...flights.map(flight => flight.delay + flight.duration));
  if (span > 320) for (const flight of flights) {
    const scale = 320 / span;
    flight.delay *= scale;
    flight.duration *= scale;
    flight.travelDuration *= scale;
    flight.spreadDuration *= scale;
    for (const hit of flight.hits) hit.at *= scale;
  }
  return flights;
}

function projectilePath(trace: ProjectileTraceDto, center?: Position): Position[] {
  const path = [trace.origin];
  if (center && same(center, trace.origin)) return path;
  for (const position of [...trace.traversed, trace.impact]) {
    if (!same(path.at(-1)!, position)) path.push(position);
    if (center && same(center, position)) break;
  }
  return path;
}

export function projectilePosition(flight: ProjectileFlight, elapsed: number): { position: Position; angle: number } {
  const step = (flight.travelDuration === 0 ? 1 : Math.max(0, Math.min(1, elapsed / flight.travelDuration))) * (flight.path.length - 1);
  const index = Math.min(Math.floor(step), Math.max(0, flight.path.length - 2));
  const from = flight.path[index]!, to = flight.path[index + 1] ?? from;
  const t = step - index;
  return { position: { x: from.x + (to.x - from.x) * t, y: from.y + (to.y - from.y) * t },
    angle: Math.atan2(to.y - from.y, to.x - from.x) };
}

/** Presentation timing only: the core supplies the exact affected cells. */
export function projectileArrival(flight: ProjectileFlight, position: Position): number {
  if (flight.kind === "ball" || flight.kind === "storm" || flight.kind === "meteor") {
    const distance = (cell: Position) => Math.hypot(cell.x - flight.center.x, cell.y - flight.center.y);
    return flight.travelDuration + flight.spreadDuration * distance(position) / flight.spreadDistance;
  }
  return flight.travelDuration * Math.max(0, flight.path.findIndex(cell => same(cell, position))) /
    Math.max(1, flight.path.length - 1);
}

interface ProjectileAreaSample {
  position: Position;
  color: number;
  alpha: number;
  radius: number;
  coreAlpha: number;
  angle?: number;
}

/** Visible cells are supplied by the renderer. Overlapping blasts draw each cell once. */
export function projectileAreaFrame(views: readonly {
  flight: ProjectileFlight; color: number; blast: readonly { position: Position; at: number }[];
}[], elapsed: number): ProjectileAreaSample[] {
  const cells = new Map<string, ProjectileAreaSample>();
  for (const { flight, color, blast } of views) {
    const time = elapsed - flight.delay;
    for (const { position, at } of blast) {
      const age = time - at;
      if (age < 0) continue;
      const storm = flight.kind === "storm";
      const fade = 1 - Math.max(0, storm ? time - flight.duration : age) / PROJECTILE_IMPACT_MS;
      if (fade <= 0) continue;
      const phase = position.x * 0.73 + position.y * 1.17;
      const angle = phase + (time - flight.travelDuration) / Math.max(1, flight.duration - flight.travelDuration) * Math.PI;
      const envelope = storm ? Math.min(1, age / 25) * fade : fade;
      const sample: ProjectileAreaSample = { position, color,
        alpha: envelope * (storm ? 0.24 + 0.06 * Math.sin(angle) : 0.45),
        radius: storm ? 0.42 : 0.22 + 0.26 * (1 - fade),
        coreAlpha: envelope * (storm ? 0.25 : 0.65),
        angle: storm ? angle : undefined };
      const key = `${position.x},${position.y}`;
      if (sample.alpha > (cells.get(key)?.alpha ?? 0)) cells.set(key, sample);
    }
  }
  return [...cells.values()];
}

export function projectileColor(type: string): number {
  switch (type) {
    case "fire": case "plasma": case "hell-fire": case "meteor": return 0xff8000;
    case "cold": case "ice": return 0x00ffff;
    case "electricity": case "light": case "storm": return 0xffff00;
    case "poison": case "acid": return 0x008040;
    case "mana": case "chaos": case "psi": case "psi-storm": return 0xff00ff;
    case "gravity": return 0xc080ff;
    case "dark": case "darkness": case "nether": return 0xc08040;
    default: return 0xffffff;
  }
}
