// SPDX-License-Identifier: MPL-2.0

import type { CharacterCreationPreviewDto, GameCommand, GameSnapshot, GameUpdate } from "./protocol";

export interface NewSessionRequest {
  readonly seed: string;
  readonly buildId: string;
  readonly raceId: string;
  readonly playerName: string;
  readonly easyIdentification?: boolean;
}

export interface CoreTransport {
  previewCharacterCreation(buildId: string, raceId: string): Promise<CharacterCreationPreviewDto>;
  initialize(request: NewSessionRequest): Promise<GameSnapshot>;
  dispatch(command: GameCommand): Promise<GameUpdate>;
  save(): Promise<Uint8Array>;
  load(data: Uint8Array): Promise<{ snapshot: GameSnapshot; museumRecovered: boolean }>;
  exportReplay(): Promise<Uint8Array>;
  dispose(): void;
}
