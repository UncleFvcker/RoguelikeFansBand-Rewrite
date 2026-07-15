// SPDX-License-Identifier: MPL-2.0

export type Direction =
  | "north"
  | "north-east"
  | "east"
  | "south-east"
  | "south"
  | "south-west"
  | "west"
  | "north-west";

export type GameCommand =
  | { type: "move"; direction: Direction }
  | { type: "wait" };

export interface Position {
  x: number;
  y: number;
}

export interface CellDto {
  position: Position;
  terrainId: string;
  actorId?: string;
}

export interface PlayerDto {
  id: string;
  kindId: string;
  position: Position;
  hp: number;
  maxHp: number;
}

export interface EntityDto {
  id: string;
  kindId: string;
  position: Position;
  hp: number;
  maxHp: number;
}

export interface GameEventDto {
  kind: string;
  messageKey: string;
  args: Record<string, string>;
}

export interface GameSnapshot {
  protocolVersion: string;
  revision: number;
  turn: number;
  lastCommandSeq: number;
  width: number;
  height: number;
  cells: CellDto[];
  player: PlayerDto;
  entities: EntityDto[];
  contentHash: string;
  stateHash: string;
}

export interface GameUpdate {
  baseRevision: number;
  revision: number;
  turn: number;
  commandSeq: number;
  events: GameEventDto[];
  changedCells: CellDto[];
  player: PlayerDto;
  entities: EntityDto[];
  removedEntities: string[];
  stateHash: string;
}

