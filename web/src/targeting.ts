// SPDX-License-Identifier: MPL-2.0

import type {
  Direction,
  Position,
  TargetSelection,
  TargetSpecDto,
} from "./protocol";

export interface TargetingState {
  origin: Position;
  cursor: Position;
  spec: TargetSpecDto;
}

export interface TargetableEntity {
  id: string;
  position: Position;
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

export function beginTargeting(
  origin: Position,
  spec: TargetSpecDto | undefined,
): TargetingState | undefined {
  if (
    !spec ||
    spec.range < 1 ||
    (!spec.modes.includes("position") && !spec.modes.includes("entity"))
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

export function targetSelectionAtCursor(
  state: TargetingState,
  entities: readonly TargetableEntity[],
): TargetSelection | undefined {
  if (samePosition(state.cursor, state.origin)) return undefined;
  if (state.spec.modes.includes("entity")) {
    const entity = [...entities]
      .filter((candidate) => samePosition(candidate.position, state.cursor))
      .sort((left, right) => left.id.localeCompare(right.id))[0];
    if (entity) return { type: "entity", entityId: entity.id };
  }
  return state.spec.modes.includes("position")
    ? { type: "position", position: { ...state.cursor } }
    : undefined;
}

export function chebyshevDistance(left: Position, right: Position): number {
  return Math.max(Math.abs(left.x - right.x), Math.abs(left.y - right.y));
}

function samePosition(left: Position, right: Position): boolean {
  return left.x === right.x && left.y === right.y;
}
