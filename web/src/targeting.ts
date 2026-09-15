// SPDX-License-Identifier: MPL-2.0

import type {
  Direction,
  Position,
  TargetSelection,
  TargetSpecDto,
  DefaultTargetModeDto,
} from "./protocol";

export interface TargetingState {
  origin: Position;
  cursor: Position;
  spec: TargetSpecDto;
  list?: boolean;
  targetPets?: boolean;
}

export interface TargetableEntity {
  id: string;
  position: Position;
  faction?: string;
  inLineOfEffect?: boolean;
}

function isCandidate(state: TargetingState, entity: TargetableEntity): boolean {
  return (state.targetPets || entity.faction !== "player")
    && (!state.spec.requiresLineOfEffect || entity.inLineOfEffect !== false)
    && chebyshevDistance(state.origin, entity.position) <= state.spec.range;
}

export function defaultTargetState(state: TargetingState, mode: DefaultTargetModeDto, remembered: TargetSelection | undefined, entities: readonly TargetableEntity[]): TargetingState {
  if ((mode === "old-target" || mode === "old-then-nearest") && remembered) {
    const old = rememberedTargetState(state, remembered, entities);
    if (old) return old;
  }
  if (mode === "nearest-enemy" || mode === "old-then-nearest") {
    const enemies = entities.filter(entity => entity.faction === "hostile" && isCandidate(state, entity));
    if (enemies.length) return cycleTarget({ ...state, cursor: { ...state.origin }, list: false }, enemies, 0);
  }
  return state;
}

const DIRECTION_DELTAS: Record<Direction, readonly [number, number]> = {
  north: [0, -1],
  "north-east": [1, -1],
  east: [1, 0],
  "south-east": [1, 1],
  south: [0, 1],
  "south-west": [-1, 1],
  west: [-1, 0],
  "north-west": [-1, -1],
};

// Navigation consumes projected entities; Core validates the actual action.
export function cycleTarget(state: TargetingState, entities: readonly TargetableEntity[], step: number): TargetingState {
  const candidates = entities.filter(entity => isCandidate(state, entity))
    .sort((a, b) => targetDistance(state.origin, a.position) - targetDistance(state.origin, b.position) || a.id.localeCompare(b.id));
  if (!candidates.length) return { ...state, list: false };
  const current = candidates.findIndex(entity => samePosition(entity.position, state.cursor));
  const index = state.list && current >= 0
    ? (current + step + candidates.length) % candidates.length
    : candidates.reduce((best, entity, i) => targetDistance(state.cursor, entity.position) < targetDistance(state.cursor, candidates[best]!.position) ? i : best, 0);
  return { ...state, cursor: { ...candidates[index]!.position }, list: true };
}

export function moveTarget(state: TargetingState, direction: Direction, entities: readonly TargetableEntity[], width: number, height: number): TargetingState {
  if (state.list) {
    const [dx, dy] = DIRECTION_DELTAS[direction];
    const candidates = entities.filter(entity => {
      const x = entity.position.x - state.cursor.x, y = entity.position.y - state.cursor.y;
      return isCandidate(state, entity) && (dx === 0 ? Math.abs(x) <= Math.abs(y) : Math.sign(x) === dx)
        && (dy === 0 ? Math.abs(y) <= Math.abs(x) : Math.sign(y) === dy)
        && !samePosition(entity.position, state.cursor)
        && chebyshevDistance(state.origin, entity.position) <= state.spec.range;
    }).sort((a, b) => targetDistance(state.cursor, a.position) - targetDistance(state.cursor, b.position) || a.id.localeCompare(b.id));
    if (candidates[0]) return { ...state, cursor: { ...candidates[0].position } };
  }
  return moveTargetCursor({ ...state, list: false }, direction, width, height);
}

export function rememberedTargetState(state: TargetingState, target: TargetSelection, entities: readonly TargetableEntity[]): TargetingState | undefined {
  if (target.type === "entity" && !entities.some(entity => entity.id === target.entityId && isCandidate(state, entity))) return undefined;
  if (target.type === "position" && !state.spec.modes.includes("position") && !state.spec.modes.includes("direction")) return undefined;
  const position = target.type === "entity" ? entities.find(entity => entity.id === target.entityId)?.position
    : target.type === "position" ? target.position : undefined;
  if (!position || chebyshevDistance(state.origin, position) > state.spec.range) return undefined;
  return { ...state, cursor: { ...position }, list: target.type === "entity" };
}

export function beginTargeting(
  origin: Position,
  spec: TargetSpecDto | undefined,
): TargetingState | undefined {
  if (
    !spec ||
    spec.range < 1 ||
    (!spec.modes.includes("position") &&
      !spec.modes.includes("entity") &&
      !spec.modes.includes("direction"))
  ) {
    return undefined;
  }
  return {
    origin: { ...origin },
    cursor: { ...origin },
    spec: { ...spec, modes: [...spec.modes] },
  };
}

export function moveTargetCursor(
  state: TargetingState,
  direction: Direction,
  width: number,
  height: number,
): TargetingState {
  const [dx, dy] = DIRECTION_DELTAS[direction];
  const cursor = { x: state.cursor.x + dx, y: state.cursor.y + dy };
  if (
    cursor.x < 0 ||
    cursor.y < 0 ||
    cursor.x >= width ||
    cursor.y >= height ||
    chebyshevDistance(state.origin, cursor) > state.spec.range
  ) {
    return state;
  }
  return { ...state, cursor };
}

export function translateTargetingState(
  state: TargetingState,
  translation: Position,
  width: number,
  height: number,
): TargetingState | undefined {
  const origin = {
    x: state.origin.x + translation.x,
    y: state.origin.y + translation.y,
  };
  const cursor = {
    x: state.cursor.x + translation.x,
    y: state.cursor.y + translation.y,
  };
  if (
    origin.x < 0 ||
    origin.y < 0 ||
    origin.x >= width ||
    origin.y >= height ||
    cursor.x < 0 ||
    cursor.y < 0 ||
    cursor.x >= width ||
    cursor.y >= height
  ) {
    return undefined;
  }
  return { ...state, origin, cursor };
}

export function targetSelectionAtCursor(
  state: TargetingState,
  entities: readonly TargetableEntity[],
): TargetSelection | undefined {
  if ((state.list !== false || !state.spec.modes.includes("position")) && state.spec.modes.includes("entity")) {
    const entity = [...entities]
      .filter((candidate) => samePosition(candidate.position, state.cursor))
      .sort((left, right) => left.id.localeCompare(right.id))[0];
    if (entity) return { type: "entity", entityId: entity.id };
  }
  if (samePosition(state.cursor, state.origin)) {
    return state.spec.modes.includes("self") && state.spec.modes.includes("position")
      ? { type: "position", position: { ...state.cursor } }
      : undefined;
  }
  if (state.spec.modes.includes("position")) {
    return { type: "position", position: { ...state.cursor } };
  }
  return state.spec.modes.includes("direction")
    ? {
        type: "direction",
        direction: directionFromDelta(
          Math.sign(state.cursor.x - state.origin.x),
          Math.sign(state.cursor.y - state.origin.y),
        ),
      }
    : undefined;
}

function directionFromDelta(dx: number, dy: number): Direction {
  return (Object.entries(DIRECTION_DELTAS) as [Direction, readonly [number, number]][])
    .find(([, [candidateX, candidateY]]) => candidateX === dx && candidateY === dy)![0];
}

export function chebyshevDistance(left: Position, right: Position): number {
  return Math.max(Math.abs(left.x - right.x), Math.abs(left.y - right.y));
}

function targetDistance(left: Position, right: Position): number {
  const dx = Math.abs(left.x - right.x), dy = Math.abs(left.y - right.y);
  return 2 * Math.max(dx, dy) + Math.min(dx, dy);
}

function samePosition(left: Position, right: Position): boolean {
  return left.x === right.x && left.y === right.y;
}
