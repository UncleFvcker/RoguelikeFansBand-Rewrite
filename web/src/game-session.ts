// SPDX-License-Identifier: MPL-2.0

import type { AppState } from "./app-state";
import type { GameCommand, GameUpdate } from "./protocol";

export type DispatchResult = "applied" | "blocked" | "failed";

export class GameSession {
  readonly #state: AppState;
  readonly #execute: (command: GameCommand) => Promise<GameUpdate>;
  readonly #applyUpdate: (update: GameUpdate, command: GameCommand) => void;
  readonly #refreshBusyControls: () => void;
  readonly #showError: (error: unknown) => void;
  readonly #onRememberedCommand: (command: GameCommand | undefined) => void;
  #idle: Promise<void> | undefined;
  #lastCommand: GameCommand | undefined;
  #generation = 0;

  get lastCommand(): GameCommand | undefined {
    return this.#lastCommand && structuredClone(this.#lastCommand);
  }

  resetCommandHistory(): void {
    this.#generation++;
    this.#lastCommand = undefined;
  }

  get isDispatching(): boolean {
    return this.#idle !== undefined;
  }

  whenIdle(): Promise<void> {
    return this.#idle ?? Promise.resolve();
  }

  constructor(options: {
    state: AppState;
    execute: (command: GameCommand) => Promise<GameUpdate>;
    applyUpdate: (update: GameUpdate, command: GameCommand) => void;
    refreshBusyControls: () => void;
    showError: (error: unknown) => void;
    onRememberedCommand?: (command: GameCommand | undefined) => void;
  }) {
    this.#state = options.state;
    this.#execute = options.execute;
    this.#applyUpdate = options.applyUpdate;
    this.#refreshBusyControls = options.refreshBusyControls;
    this.#showError = options.showError;
    this.#onRememberedCommand = options.onRememberedCommand ?? (() => {});
  }

  async dispatch(command: GameCommand, repeatCommand: GameCommand | false = command): Promise<DispatchResult> {
    if (
      this.#state.busy ||
      (this.#state.commandBlocked &&
        !(command.type === "configure-preferences" && this.#state.mode === "playing" && !this.#state.playerDead && !this.#state.campaignEnded) &&
        !(this.#state.status?.player.pendingMaiaPathChoice &&
          ["choose-maia-path", "set-interface-locale"].includes(command.type)) &&
        command.type !== "resolve-mutation-direction" &&
        command.type !== "resolve-ability-direction" &&
        command.type !== "cancel-ability-direction" &&
        command.type !== "resolve-realm-change" &&
        command.type !== "resolve-duelist-choice" &&
        !(this.#state.status?.player.magicEater?.pendingAbsorption &&
          ["select-magic-absorption-slot", "resolve-magic-absorption", "set-interface-locale"].includes(command.type))) ||
      (this.#state.worldMap &&
        command.type !== "end-character" &&
        command.type !== "choose-maia-path" &&
        command.type !== "move" &&
        command.type !== "walk-special" &&
        command.type !== "travel-world" &&
        command.type !== "leave-world-map" &&
        command.type !== "configure-mogaminator" &&
        command.type !== "configure-travel" &&
        command.type !== "configure-preferences" &&
        command.type !== "set-interface-locale")
    ) return "blocked";
    const generation = this.#generation;
    const before = this.#state.status;
    const remember = repeatCommand !== false && !isInternalCommand(repeatCommand);
    const recorded = remember && canRepeatLast(repeatCommand) ? structuredClone(repeatCommand) : undefined;
    let settled!: () => void;
    const idle = new Promise<void>(resolve => { settled = resolve; });
    this.#idle = idle;
    this.#state.busy = true;
    this.#refreshBusyControls();
    try {
      const update = await this.#execute(command);
      // Update renderers observe the final idle state and emit their final
      // controls directly, avoiding a redundant second panel render.
      this.#state.busy = false;
      this.#applyUpdate(update, command);
      if (generation === this.#generation) {
        if (remember) this.#lastCommand = recorded;
        // Coordinates and item selections belong to this floor/session.
        if ((before && (before.floorId !== update.floorId || before.mapScale !== update.mapScale)) ||
            (update.mapTranslation && (update.mapTranslation.x !== 0 || update.mapTranslation.y !== 0))) {
          this.#lastCommand = undefined;
        }
        if (remember) this.#onRememberedCommand(this.lastCommand);
      }
      return "applied";
    } catch (error) {
      if (remember && generation === this.#generation) {
        this.#lastCommand = undefined;
        this.#onRememberedCommand(undefined);
      }
      this.#showError(error);
      return "failed";
    } finally {
      if (this.#idle === idle) this.#idle = undefined;
      settled();
      if (this.#state.busy) {
        this.#state.busy = false;
        this.#refreshBusyControls();
      }
    }
  }
}

function isInternalCommand(command: GameCommand): boolean {
  return command.type.startsWith("continue-") || command.type.startsWith("cancel-") ||
    command.type.startsWith("resolve-") || command.type.startsWith("configure-") ||
    command.type === "set-interface-locale" || command.type === "select-magic-absorption-slot";
}

function canRepeatLast(command: GameCommand): boolean {
  // Explicit gameplay commands only; purchases, retirement and permanent character
  // choices must keep their own UI entry and confirmation flow.
  switch (command.type) {
    case "move": case "walk-special": case "wait": case "stay": case "swap-rings": case "search": case "toggle-search": case "set-pet-target":
    case "alter": case "spike-door": case "open-door": case "close-door":
    case "bash-door": case "dig-terrain": case "disarm-trap":
    case "open-chest": case "disarm-chest": case "ride": case "traverse-stairs":
    case "run": case "auto-explore": case "rest": case "rest-for-turns": case "rest-until-resources":
    case "travel-local": case "travel-world": case "travel-unknown-item":
    case "find-nearest-unknown-item": case "enter-world-map": case "leave-world-map":
    case "pick-up": case "equip": case "unequip": case "drop": case "drop-quantity":
    case "destroy-item": case "inscribe-item": case "refuel-light":
    case "fire": case "fire-target": case "throw": case "cast-ability":
    case "use-item": case "use-item-by-glyph": case "use-item-for-recharge":
    case "use-absorbed-device": case "absorb-device": case "appraise":
    case "study-ability": case "study-prayer":
      return true;
    default: return false;
  }
}
