// SPDX-License-Identifier: MPL-2.0

import type { GameCommand, GameSnapshot, GameUpdate } from "./protocol";

export interface CoreTransport {
  initialize(seed: string): Promise<GameSnapshot>;
  dispatch(command: GameCommand): Promise<GameUpdate>;
  save(): Promise<Uint8Array>;
  load(data: Uint8Array): Promise<GameSnapshot>;
  dispose(): void;
}
