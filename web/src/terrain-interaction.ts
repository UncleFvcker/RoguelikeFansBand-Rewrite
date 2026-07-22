// SPDX-License-Identifier: MPL-2.0

import type {
  Direction,
  GameCommand,
  TerrainInteractionDto,
  TerrainInteractionKindDto,
} from "./protocol";

export type TerrainInteractionMode =
  | "open-door"
  | "close-door"
  | "bash-door"
  | "disarm-trap"
  | "dig-terrain";

export function terrainInteractionModeForKey(
  key: string,
): TerrainInteractionMode | undefined {
  if (key === "B") return "bash-door";
  if (key === "D") return "disarm-trap";
  if (key === "T") return "dig-terrain";
  const normalized = key.toLowerCase();
  if (normalized === "o") return "open-door";
  if (normalized === "c") return "close-door";
  return undefined;
}

export function terrainSearchCommandForKey(
  key: string,
): GameCommand | undefined {
  return key === "S" ? { type: "search" } : undefined;
}

export function terrainInteractionCommand(
  mode: TerrainInteractionMode,
  direction: Direction,
): GameCommand {
  if (mode === "open-door") return { type: "open-door", direction };
  if (mode === "close-door") return { type: "close-door", direction };
  if (mode === "bash-door") return { type: "bash-door", direction };
  if (mode === "disarm-trap") return { type: "disarm-trap", direction };
  return { type: "dig-terrain", direction };
}

export function terrainInteractionKindForMode(
  mode: TerrainInteractionMode,
): TerrainInteractionKindDto {
  if (mode === "open-door") return "open-door";
  if (mode === "close-door") return "close-door";
  if (mode === "bash-door") return "bash-door";
  if (mode === "disarm-trap") return "disarm-trap";
  return "dig-terrain";
}

export function terrainInteractionsForMode(
  interactions: readonly TerrainInteractionDto[],
  mode: TerrainInteractionMode,
): TerrainInteractionDto[] {
  const kind = terrainInteractionKindForMode(mode);
  return interactions.filter((interaction) => interaction.kind === kind);
}

export function terrainInteractionForDirection(
  interactions: readonly TerrainInteractionDto[],
  mode: TerrainInteractionMode,
  direction: Direction,
): TerrainInteractionDto | undefined {
  return terrainInteractionsForMode(interactions, mode).find(
    (interaction) => interaction.direction === direction,
  );
}
