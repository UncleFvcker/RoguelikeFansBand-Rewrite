// SPDX-License-Identifier: MPL-2.0
import type { GameCommand, GameSnapshot, GameUpdate, Position } from "./protocol";
import type { RenderCell } from "./renderer-backend";

export const PLAYER_STEP_MS = 100;
export const CONTINUOUS_STEP_MS = 80;
export type PlayerFrame = Pick<GameSnapshot, "floorId" | "mapScale" | "player" | "entities">;
export const PLAYER_MELEE_MS = 150;

const movementCommands = new Set<GameCommand["type"]>([
  "move", "walk-special", "run", "continue-run", "auto-explore", "continue-auto-explore",
  "travel-local", "travel-unknown-item", "auto-get",
]);

export function canAnimatePlayerStep(before: PlayerFrame | undefined, after: GameUpdate,
  command: GameCommand | undefined): boolean {
  if (!before || !command || !movementCommands.has(command.type) ||
      before.mapScale !== "local" || after.mapScale !== "local" ||
      before.floorId !== after.floorId ||
      before.player.id !== after.player.id || after.player.hp <= 0 ||
      (after.mapTranslation && (after.mapTranslation.x !== 0 || after.mapTranslation.y !== 0))) return false;
  const from = before.player.position, to = after.player.position;
  if (Math.max(Math.abs(to.x - from.x), Math.abs(to.y - from.y)) !== 1) return false;
  return !playerWasDisplaced(after);
}

function playerWasDisplaced(after: GameUpdate): boolean {
  return after.events.some(event => {
    const outcome = event.outcome;
    if (outcome?.type === "monster-displacement") return outcome.resolution.actorId === after.player.id;
    return outcome?.type === "ability-teleport" || event.kind.includes("teleport");
  });
}

const meleeKinds = new Set(["combat.hit", "combat.miss", "combat.slay",
  "mutation.melee-hit", "mutation.melee-miss", "mutation.melee-slay"]);

export interface PlayerMeleeAttack {
  direction: Position;
  targetId: string;
  targetPosition?: Position;
  outcome: "miss" | "hit" | "kill";
}

/** One presentation per update, using the first actual adjacent melee target. */
export function playerMeleeAttack(before: PlayerFrame | undefined, after: GameUpdate,
  command: GameCommand | undefined): PlayerMeleeAttack | undefined {
  if (!before || before.mapScale !== "local" || after.mapScale !== "local" ||
      before.floorId !== after.floorId || before.player.id !== after.player.id || after.player.hp <= 0 ||
      before.player.position.x !== after.player.position.x || before.player.position.y !== after.player.position.y ||
      after.mapTranslation?.x || after.mapTranslation?.y || playerWasDisplaced(after)) return undefined;
  for (const event of after.events) {
    if (!meleeKinds.has(event.kind) || !event.args.attackTarget) continue;
    // Prefer the pre-attack position: the victim may already be dead, knocked back or fleeing.
    const target = before.entities.find(entity => entity.id === event.args.attackTarget) ??
      after.entities.find(entity => entity.id === event.args.attackTarget);
    const blows = after.events.filter(blow => meleeKinds.has(blow.kind) && blow.args.attackTarget === event.args.attackTarget);
    const outcome = blows.some(blow => blow.kind.endsWith("slay")) ? "kill"
      : blows.some(blow => blow.kind.endsWith("hit")) ? "hit" : "miss";
    if (target) {
      const x = target.position.x - before.player.position.x, y = target.position.y - before.player.position.y;
      if (Math.max(Math.abs(x), Math.abs(y)) === 1) return {
        direction: { x, y }, targetId: target.id, targetPosition: { ...target.position }, outcome,
      };
    } else if (command?.type === "move" || command?.type === "walk-special" || command?.type === "alter") {
      // Unseen enemies have no entity projection. Only show the player's known swing direction.
      const direction = command.direction;
      return { direction: { x: direction.includes("east") ? 1 : direction.includes("west") ? -1 : 0,
        y: direction.includes("south") ? 1 : direction.includes("north") ? -1 : 0 },
        targetId: event.args.attackTarget, outcome };
    }
  }
  return undefined;
}

export function meleeEffectTarget(attack: Pick<PlayerMeleeAttack, "targetId" | "outcome">, before: RenderCell | undefined,
  current: RenderCell | undefined): RenderCell | undefined {
  if (attack.outcome === "miss" || current?.visibility !== "visible") return undefined;
  const target = attack.outcome === "kill" ? before : current;
  // Fuzzy/detected contacts and remembered tiles must not reveal a monster's true appearance.
  return target?.visibility === "visible" && target.actorId === attack.targetId && !target.actorGlyph
    ? target : undefined;
}

export function meleeImpact(elapsedMs: number) {
  const t = Math.max(0, Math.min(1, (elapsedMs - 45) / (PLAYER_MELEE_MS - 45)));
  return {
    flash: elapsedMs < 45 ? 0 : (1 - t) ** 2,
    corpseAlpha: 1 - t,
    cameraPixels: elapsedMs < 45 ? 0 : 2 * Math.sin(Math.PI * t) * (1 - t),
  };
}

export function playerMeleeOffset(direction: Position, elapsedMs: number): Position {
  if (elapsedMs <= 0 || elapsedMs >= PLAYER_MELEE_MS) return { x: 0, y: 0 };
  const progress = elapsedMs < 45 ? 1 - (1 - elapsedMs / 45) ** 2
    : elapsedMs < 70 ? 1 : (1 - (elapsedMs - 70) / 80) ** 2;
  const distance = 0.2 * progress / Math.hypot(direction.x, direction.y);
  return { x: direction.x * distance, y: direction.y * distance };
}

export function playerStepProgress(elapsedMs: number, continuous = false): number {
  const t = Math.max(0, Math.min(1, elapsedMs / (continuous ? CONTINUOUS_STEP_MS : PLAYER_STEP_MS)));
  // Immediate start, gently settle on the destination; no spring overshoot.
  return continuous ? t : 1 - (1 - t) * (1 - t);
}

export function playerStepPosition(from: Position, to: Position, elapsedMs: number, continuous = false): Position {
  const progress = playerStepProgress(elapsedMs, continuous);
  return { x: from.x + (to.x - from.x) * progress, y: from.y + (to.y - from.y) * progress };
}
