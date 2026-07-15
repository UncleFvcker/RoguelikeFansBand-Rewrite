// SPDX-License-Identifier: MPL-2.0

import { invoke } from "@tauri-apps/api/core";

import type { CoreTransport } from "./core-transport";
import type { GameCommand, GameSnapshot, GameUpdate } from "./protocol";

export class TauriNativeTransport implements CoreTransport {
  #revision = 0;
  #commandSeq = 0;

  async initialize(seed: string): Promise<GameSnapshot> {
    const snapshot = await invoke<GameSnapshot>("initialize_game", {
      seed,
      createdAt: new Date().toISOString(),
    });
    this.#syncSnapshot(snapshot);
    return snapshot;
  }

  async dispatch(command: GameCommand): Promise<GameUpdate> {
    const update = await invoke<GameUpdate>("dispatch_game_command", {
      commandSeq: this.#commandSeq + 1,
      expectedRevision: this.#revision,
      command,
    });
    this.#revision = update.revision;
    this.#commandSeq = update.commandSeq;
    return update;
  }

  async save(): Promise<Uint8Array> {
    const bytes = await invoke<number[]>("save_game", {
      savedAt: new Date().toISOString(),
    });
    return Uint8Array.from(bytes);
  }

  async load(data: Uint8Array): Promise<GameSnapshot> {
    const snapshot = await invoke<GameSnapshot>("load_game", {
      data: Array.from(data),
    });
    this.#syncSnapshot(snapshot);
    return snapshot;
  }

  async exportReplay(): Promise<Uint8Array> {
    const bytes = await invoke<number[]>("export_replay");
    return Uint8Array.from(bytes);
  }

  dispose(): void {
    // The native game session is owned by the Tauri application and ends with it.
  }

  #syncSnapshot(snapshot: GameSnapshot): void {
    this.#revision = snapshot.revision;
    this.#commandSeq = snapshot.lastCommandSeq;
  }
}
