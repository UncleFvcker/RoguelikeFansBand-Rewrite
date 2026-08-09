// SPDX-License-Identifier: MPL-2.0

import type { AppState } from "./app-state";
import type { GameCommand, GameUpdate } from "./protocol";

export class GameSession {
  readonly #state: AppState;
  readonly #execute: (command: GameCommand) => Promise<GameUpdate>;
  readonly #applyUpdate: (update: GameUpdate, command: GameCommand) => void;
  readonly #refreshBusyControls: () => void;
  readonly #showError: (error: unknown) => void;

  constructor(options: {
    state: AppState;
    execute: (command: GameCommand) => Promise<GameUpdate>;
    applyUpdate: (update: GameUpdate, command: GameCommand) => void;
    refreshBusyControls: () => void;
    showError: (error: unknown) => void;
  }) {
    this.#state = options.state;
    this.#execute = options.execute;
    this.#applyUpdate = options.applyUpdate;
    this.#refreshBusyControls = options.refreshBusyControls;
    this.#showError = options.showError;
  }

  async dispatch(command: GameCommand): Promise<void> {
    if (
      this.#state.commandBlocked ||
      (this.#state.worldMap &&
        command.type !== "move" &&
        command.type !== "travel-world" &&
        command.type !== "leave-world-map")
    ) return;
    this.#state.busy = true;
    this.#refreshBusyControls();
    try {
      const update = await this.#execute(command);
      // Update renderers observe the final idle state and emit their final
      // controls directly, avoiding a redundant second panel render.
      this.#state.busy = false;
      this.#applyUpdate(update, command);
    } catch (error) {
      this.#showError(error);
    } finally {
      if (this.#state.busy) {
        this.#state.busy = false;
        this.#refreshBusyControls();
      }
    }
  }
}
