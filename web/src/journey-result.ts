// SPDX-License-Identifier: MPL-2.0

import type { AppDom } from "./app-dom";
import type { Localization } from "./localization";
import type {
  GameEventDto,
  GameSnapshot,
  GameUpdate,
} from "./protocol";

type JourneyState = GameSnapshot | GameUpdate;

export type JourneyResultKind = "death" | "victory-return" | "retired";

type ResultDom = Pick<
  AppDom,
  | "journeyResult"
  | "resultKind"
  | "resultTitle"
  | "resultDetail"
  | "resultBuild"
  | "resultSeed"
  | "resultTurn"
  | "resultScore"
  | "resultDungeons"
  | "resultTasks"
  | "resultContinue"
  | "resultRestart"
  | "resultNewGame"
  | "resultLoad"
  | "resultMenu"
  | "resultExit"
  | "resultError"
>;

const PLAYER_DEATH_MESSAGES = new Set([
  "combat-player-death",
  "status-player-death",
  "item-use-life-loss-death",
  "item-use-detonation-death",
  "item-use-elemental-backlash-death",
]);

export function selectJourneyResultKind(
  state: JourneyState,
): JourneyResultKind | undefined {
  if (state.player.isDead) return "death";
  if (state.campaign.status === "retired") return "retired";
  if (state.campaign.status === "victorious") return "victory-return";
  return undefined;
}

export function selectJourneyResultEvent(
  kind: JourneyResultKind,
  events: readonly GameEventDto[],
): GameEventDto | undefined {
  return [...events].reverse().find((event) => {
    switch (kind) {
      case "death":
        return PLAYER_DEATH_MESSAGES.has(event.messageKey);
      case "victory-return":
        return event.kind === "campaign.victorious";
      case "retired":
        return event.kind === "campaign.retired";
    }
  });
}

export class JourneyResult {
  readonly #dom: ResultDom;
  readonly #localization: Localization;
  readonly #formatEvent: (event: GameEventDto) => string;
  readonly #currentSeed: () => string | undefined;
  readonly #canRestart: () => boolean;
  readonly #onRestart: () => Promise<void>;
  readonly #onNewGame: () => void;
  readonly #onLoad: () => void;
  readonly #onMenu: () => void;
  readonly #onExit: () => Promise<void>;
  #state: JourneyState | undefined;
  #events: readonly GameEventDto[] = [];
  #kind: JourneyResultKind | undefined;
  #victoryAcknowledged = false;
  #busy = false;
  #installed = false;

  constructor(options: {
    dom: ResultDom;
    localization: Localization;
    formatEvent: (event: GameEventDto) => string;
    currentSeed: () => string | undefined;
    canRestart: () => boolean;
    onRestart: () => Promise<void>;
    onNewGame: () => void;
    onLoad: () => void;
    onMenu: () => void;
    onExit: () => Promise<void>;
  }) {
    this.#dom = options.dom;
    this.#localization = options.localization;
    this.#formatEvent = options.formatEvent;
    this.#currentSeed = options.currentSeed;
    this.#canRestart = options.canRestart;
    this.#onRestart = options.onRestart;
    this.#onNewGame = options.onNewGame;
    this.#onLoad = options.onLoad;
    this.#onMenu = options.onMenu;
    this.#onExit = options.onExit;
  }

  install(): void {
    if (this.#installed) return;
    this.#installed = true;
    this.#dom.resultContinue.addEventListener("click", this.#continue);
    this.#dom.resultRestart.addEventListener("click", this.#restart);
    this.#dom.resultNewGame.addEventListener("click", this.#newGame);
    this.#dom.resultLoad.addEventListener("click", this.#load);
    this.#dom.resultMenu.addEventListener("click", this.#menu);
    this.#dom.resultExit.addEventListener("click", this.#exit);
  }

  dispose(): void {
    if (!this.#installed) return;
    this.#installed = false;
    this.#dom.resultContinue.removeEventListener("click", this.#continue);
    this.#dom.resultRestart.removeEventListener("click", this.#restart);
    this.#dom.resultNewGame.removeEventListener("click", this.#newGame);
    this.#dom.resultLoad.removeEventListener("click", this.#load);
    this.#dom.resultMenu.removeEventListener("click", this.#menu);
    this.#dom.resultExit.removeEventListener("click", this.#exit);
  }

  renderSnapshot(snapshot: GameSnapshot): void {
    this.#kind = undefined;
    this.#victoryAcknowledged = false;
    this.#render(snapshot, []);
  }

  renderUpdate(update: GameUpdate): void {
    this.#render(update, update.events);
  }

  localize(): void {
    this.#localization.localizeDocument(this.#dom.journeyResult);
    if (this.#state) this.#render(this.#state, this.#events);
  }

  hideForNavigation(): void {
    this.#state = undefined;
    this.#events = [];
    this.#kind = undefined;
    this.#victoryAcknowledged = false;
    this.#dom.journeyResult.hidden = true;
    delete this.#dom.journeyResult.dataset.resultKind;
    this.#dom.journeyResult.ownerDocument.documentElement.dataset.journeyResult = "none";
  }

  #render(state: JourneyState, events: readonly GameEventDto[]): void {
    this.#state = state;
    this.#events = events;
    const kind = selectJourneyResultKind(state);
    if (!kind) {
      this.#kind = undefined;
      this.#victoryAcknowledged = false;
      this.#hide("none");
      return;
    }
    if (kind === "victory-return" && this.#kind !== "victory-return") {
      this.#victoryAcknowledged = false;
    }
    this.#kind = kind;
    if (kind === "victory-return" && this.#victoryAcknowledged) {
      this.#hide("victory-return-acknowledged");
      return;
    }

    const wasHidden = this.#dom.journeyResult.hidden;
    this.#dom.journeyResult.hidden = false;
    this.#dom.journeyResult.dataset.resultKind = kind;
    this.#dom.journeyResult.ownerDocument.documentElement.dataset.journeyResult = kind;
    this.#dom.resultKind.textContent = this.#localization.format(`result-kind-${kind}`);
    this.#dom.resultTitle.textContent = this.#localization.format(`result-title-${kind}`);
    const event = selectJourneyResultEvent(kind, events);
    this.#dom.resultDetail.textContent = event
      ? this.#formatEvent(event)
      : this.#localization.format(`result-detail-${kind}`);
    this.#dom.resultBuild.textContent = state.player.build
      ? this.#localization.format(state.player.build.buildNameKey)
      : this.#localization.format("session-build-unknown");
    this.#dom.resultSeed.textContent =
      this.#currentSeed() ?? this.#localization.format("result-seed-loaded-save");
    this.#dom.resultTurn.textContent = String(state.turn);
    this.#dom.resultScore.textContent = String(state.campaign.score);
    this.#dom.resultDungeons.textContent = String(state.campaign.conqueredDungeons);
    this.#dom.resultTasks.textContent = String(state.campaign.completedTasks);
    this.#dom.resultContinue.hidden = kind !== "victory-return";
    this.#dom.resultRestart.title = this.#canRestart()
      ? ""
      : this.#localization.format("result-restart-unavailable");
    this.#dom.resultError.replaceChildren();
    this.#updateActions();
    if (wasHidden) {
      (kind === "victory-return"
        ? this.#dom.resultContinue
        : this.#canRestart()
          ? this.#dom.resultRestart
          : this.#dom.resultNewGame
      ).focus();
    }
  }

  #hide(datasetValue: string): void {
    this.#dom.journeyResult.hidden = true;
    delete this.#dom.journeyResult.dataset.resultKind;
    this.#dom.journeyResult.ownerDocument.documentElement.dataset.journeyResult = datasetValue;
  }

  #updateActions(): void {
    for (const button of [
      this.#dom.resultContinue,
      this.#dom.resultRestart,
      this.#dom.resultNewGame,
      this.#dom.resultLoad,
      this.#dom.resultMenu,
      this.#dom.resultExit,
    ]) {
      button.disabled = this.#busy;
    }
    this.#dom.resultRestart.disabled = this.#busy || !this.#canRestart();
  }

  readonly #continue = (): void => {
    if (this.#busy || this.#kind !== "victory-return") return;
    this.#victoryAcknowledged = true;
    this.#hide("victory-return-acknowledged");
  };

  readonly #restart = (): void => {
    if (this.#busy || !this.#canRestart()) return;
    void this.#run(this.#onRestart);
  };

  readonly #newGame = (): void => {
    if (this.#busy) return;
    this.hideForNavigation();
    this.#onNewGame();
  };

  readonly #load = (): void => {
    if (this.#busy) return;
    this.hideForNavigation();
    this.#onLoad();
  };

  readonly #menu = (): void => {
    if (this.#busy) return;
    this.hideForNavigation();
    this.#onMenu();
  };

  readonly #exit = (): void => {
    if (!this.#busy) void this.#run(this.#onExit);
  };

  async #run(action: () => Promise<void>): Promise<void> {
    this.#busy = true;
    this.#dom.resultError.replaceChildren();
    this.#updateActions();
    try {
      await action();
    } catch (error) {
      this.#dom.resultError.textContent = this.#localization.format("result-action-error", {
        error: error instanceof Error ? error.message : String(error),
      });
    } finally {
      this.#busy = false;
      this.#updateActions();
    }
  }
}
