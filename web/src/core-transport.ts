// SPDX-License-Identifier: MPL-2.0

import type { GameCommand, GameSnapshot, GameUpdate } from "./protocol";

export interface NewSessionRequest {
  readonly seed: string;
  readonly buildId: string;
  readonly raceId: string;
  readonly playerName: string;
}

export interface CoreTransport {
  initialize(request: NewSessionRequest): Promise<GameSnapshot>;
  dispatch(command: GameCommand): Promise<GameUpdate>;
  save(): Promise<Uint8Array>;
  load(data: Uint8Array): Promise<GameSnapshot>;
  exportReplay(): Promise<Uint8Array>;
  dispose(): void;
}
