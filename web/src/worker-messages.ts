// SPDX-License-Identifier: MPL-2.0

import type { GameCommand, GameSnapshot, GameUpdate } from "./protocol";

export type WorkerRequest =
  | {
      requestId: number;
      type: "initialize";
      seed: string;
      createdAt: string;
    }
  | {
      requestId: number;
      type: "command";
      commandSeq: number;
      expectedRevision: number;
      command: GameCommand;
    }
  | { requestId: number; type: "save"; savedAt: string }
  | { requestId: number; type: "load"; data: ArrayBuffer };

export type WorkerResponse =
  | { requestId: number; ok: true; type: "snapshot"; payload: GameSnapshot }
  | { requestId: number; ok: true; type: "update"; payload: GameUpdate }
  | { requestId: number; ok: true; type: "save"; payload: ArrayBuffer }
  | { requestId: number; ok: false; type: "error"; message: string };

