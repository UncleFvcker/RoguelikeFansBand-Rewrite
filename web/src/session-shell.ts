// SPDX-License-Identifier: MPL-2.0

import type { NewSessionRequest } from "./core-transport.ts";
import type { InputPreset } from "./input-controller.ts";
import type { Localization, SupportedLocale } from "./localization.ts";
import { isSupportedLocale } from "./localization.ts";
import {
  desktopErrorCode,
  type NativeLoadResult,
  type NativeSaveStorage,
  type NativeSaveSummary,
} from "./native-save-storage.ts";
import { nativeSaveErrorKey } from "./save-panel.ts";
import type { GameSnapshot } from "./protocol.ts";

export const PLAYTEST_BUILD_IDS = [
  "demo.build.warrior",
  "demo.build.high-mage-death",
  "demo.build.archer",
  "demo.build.paladin-death",
  "demo.build.cavalry",
  "demo.build.sniper",
] as const;

export type PlaytestBuildId = (typeof PLAYTEST_BUILD_IDS)[number];
export const PLAYTEST_RACE_IDS = [
  "demo.race.rfb-human",
  "rfb-legacy.race.half-orc",
  "rfb-legacy.race.high-elf",
  "rfb-legacy.race.dunadan",
  "rfb-legacy.race.barbarian",
  "rfb-legacy.race.hobbit",
  "rfb-legacy.race.kobold",
  "rfb-legacy.race.dwarf",
  "rfb-legacy.race.nibelung",
  "rfb-legacy.race.gnome",
  "rfb-legacy.race.half-giant",
] as const;
export type PlaytestRaceId = (typeof PLAYTEST_RACE_IDS)[number];
export type SessionView = "title" | "new-game" | "load" | "settings";

export function createNewSessionRequest(
  seed: string,
  buildId: PlaytestBuildId,
  raceId: PlaytestRaceId,
  playerName: string,
): NewSessionRequest {
  return { seed, buildId, raceId, playerName };
}

const MAX_SESSION_SEED = (1n << 64n) - 1n;
const MAX_CHARACTER_NAME_LENGTH = 32;

type SessionStorage = Pick<NativeSaveStorage, "list" | "load" | "delete">;
type DocumentLookup = Pick<Document, "getElementById">;

interface SessionShellDom {
  readonly root: HTMLElement;
  readonly gameRoot: HTMLElement;
  readonly titleView: HTMLElement;
  readonly newGameView: HTMLFormElement;
  readonly loadView: HTMLElement;
  readonly settingsView: HTMLElement;
  readonly newGameButton: HTMLButtonElement;
  readonly continueButton: HTMLButtonElement;
  readonly loadGameButton: HTMLButtonElement;
  readonly settingsButton: HTMLButtonElement;
  readonly exitButton: HTMLButtonElement;
  readonly warriorBuild: HTMLInputElement;
  readonly highMageDeathBuild: HTMLInputElement;
  readonly archerBuild: HTMLInputElement;
  readonly paladinDeathBuild: HTMLInputElement;
  readonly cavalryBuild: HTMLInputElement;
  readonly sniperBuild: HTMLInputElement;
  readonly raceSelect: HTMLSelectElement;
  readonly characterNameInput: HTMLInputElement;
  readonly seedInput: HTMLInputElement;
  readonly randomizeSeedButton: HTMLButtonElement;
  readonly startGameButton: HTMLButtonElement;
  readonly newGameBackButton: HTMLButtonElement;
  readonly loadRefreshButton: HTMLButtonElement;
  readonly loadList: HTMLUListElement;
  readonly loadBackButton: HTMLButtonElement;
  readonly settingsLanguage: HTMLSelectElement;
  readonly settingsInput: HTMLSelectElement;
  readonly settingsBackButton: HTMLButtonElement;
  readonly status: HTMLElement;
  readonly error: HTMLElement;
  readonly runBuildValue: HTMLElement;
  readonly runSeedValue: HTMLElement;
}

export class SessionShell {
  readonly #dom: SessionShellDom;
  readonly #storage: SessionStorage;
  readonly #localization: Localization;
  readonly #onStart: (request: NewSessionRequest) => Promise<GameSnapshot>;
  readonly #onLoad: (
    result: NativeLoadResult,
    summary: NativeSaveSummary,
  ) => Promise<void>;
  readonly #onExit: () => Promise<void>;
  readonly #onLocaleChange: (locale: SupportedLocale) => void;
  readonly #onInputPresetChange: (preset: InputPreset) => void;
  readonly #getInputPreset: () => InputPreset;
  readonly #randomSeed: () => string;
  readonly #confirm: (message: string) => boolean;
  readonly #logError: (error: unknown) => void;
  #view: SessionView = "title";
  #busy = false;
  #installed = false;
  #saves: NativeSaveSummary[] = [];
  #activeSnapshot: GameSnapshot | undefined;
  #activeRequest: NewSessionRequest | undefined;

  constructor(options: {
    dom: SessionShellDom;
    storage: SessionStorage;
    localization: Localization;
    onStart: (request: NewSessionRequest) => Promise<GameSnapshot>;
    onLoad: (result: NativeLoadResult, summary: NativeSaveSummary) => Promise<void>;
    onExit: () => Promise<void>;
    onLocaleChange: (locale: SupportedLocale) => void;
    onInputPresetChange: (preset: InputPreset) => void;
    getInputPreset: () => InputPreset;
    randomSeed?: () => string;
    confirm?: (message: string) => boolean;
    logError?: (error: unknown) => void;
  }) {
    this.#dom = options.dom;
    this.#storage = options.storage;
    this.#localization = options.localization;
    this.#onStart = options.onStart;
    this.#onLoad = options.onLoad;
    this.#onExit = options.onExit;
    this.#onLocaleChange = options.onLocaleChange;
    this.#onInputPresetChange = options.onInputPresetChange;
    this.#getInputPreset = options.getInputPreset;
    this.#randomSeed = options.randomSeed ?? randomSessionSeed;
    this.#confirm = options.confirm ?? ((message) => window.confirm(message));
    this.#logError = options.logError ?? console.error;
  }

  install(): void {
    if (this.#installed) return;
    this.#installed = true;
    this.#dom.newGameButton.addEventListener("click", this.#openNewGame);
    this.#dom.continueButton.addEventListener("click", this.#continueLatest);
    this.#dom.loadGameButton.addEventListener("click", this.#openLoad);
    this.#dom.settingsButton.addEventListener("click", this.#openSettings);
    this.#dom.exitButton.addEventListener("click", this.#exit);
    this.#dom.newGameView.addEventListener("submit", this.#startNewGame);
    this.#dom.randomizeSeedButton.addEventListener("click", this.#randomizeSeed);
    this.#dom.newGameBackButton.addEventListener("click", this.#backToTitle);
    this.#dom.loadRefreshButton.addEventListener("click", this.#refreshSaves);
    this.#dom.loadBackButton.addEventListener("click", this.#backToTitle);
    this.#dom.settingsLanguage.addEventListener("change", this.#changeLocale);
    this.#dom.settingsInput.addEventListener("change", this.#changeInputPreset);
    this.#dom.settingsBackButton.addEventListener("click", this.#backToTitle);
  }

  dispose(): void {
    if (!this.#installed) return;
    this.#installed = false;
    this.#dom.newGameButton.removeEventListener("click", this.#openNewGame);
    this.#dom.continueButton.removeEventListener("click", this.#continueLatest);
    this.#dom.loadGameButton.removeEventListener("click", this.#openLoad);
    this.#dom.settingsButton.removeEventListener("click", this.#openSettings);
    this.#dom.exitButton.removeEventListener("click", this.#exit);
    this.#dom.newGameView.removeEventListener("submit", this.#startNewGame);
    this.#dom.randomizeSeedButton.removeEventListener("click", this.#randomizeSeed);
    this.#dom.newGameBackButton.removeEventListener("click", this.#backToTitle);
    this.#dom.loadRefreshButton.removeEventListener("click", this.#refreshSaves);
    this.#dom.loadBackButton.removeEventListener("click", this.#backToTitle);
    this.#dom.settingsLanguage.removeEventListener("change", this.#changeLocale);
    this.#dom.settingsInput.removeEventListener("change", this.#changeInputPreset);
    this.#dom.settingsBackButton.removeEventListener("click", this.#backToTitle);
  }

  async initialize(): Promise<void> {
    this.#dom.seedInput.value = this.#randomSeed();
    this.#dom.settingsLanguage.value = this.#localization.locale;
    this.#dom.settingsInput.value = this.#getInputPreset();
    this.#showView("title");
    this.localize();
    await this.#refresh();
  }

  localize(): void {
    this.#localization.localizeDocument(this.#dom.root);
    this.#dom.seedInput.placeholder = this.#localization.format("session-seed-placeholder");
    this.#dom.settingsLanguage.value = this.#localization.locale;
    this.#dom.settingsInput.value = this.#getInputPreset();
    this.#renderSaves();
    this.#renderReadyStatus();
    this.#renderRunMetadata();
  }

  showGame(snapshot: GameSnapshot, request?: NewSessionRequest): void {
    this.#activeSnapshot = snapshot;
    this.#activeRequest = request;
    this.#renderRunMetadata();
    const build = snapshot.player.build;
    this.#dom.gameRoot.dataset.sessionBuildId = build?.buildId ?? "unknown";
    if (request) this.#dom.gameRoot.dataset.sessionSeed = request.seed;
    else delete this.#dom.gameRoot.dataset.sessionSeed;
    this.#dom.root.hidden = true;
    this.#dom.gameRoot.hidden = false;
    this.#dom.root.ownerDocument.documentElement.dataset.appMode = "playing";
  }

  get restartRequest(): NewSessionRequest | undefined {
    return this.#activeRequest ? { ...this.#activeRequest } : undefined;
  }

  showTitle(): void {
    this.#showShell("title");
    void this.#refresh();
  }

  showNewGame(randomizeSeed = false): void {
    this.#showShell("new-game");
    if (randomizeSeed) this.#dom.seedInput.value = this.#randomSeed();
    this.#dom.characterNameInput.focus();
  }

  showLoad(): void {
    this.#showShell("load");
    void this.#refresh();
  }

  readonly #openNewGame = (): void => {
    if (this.#busy) return;
    this.showNewGame();
  };

  readonly #continueLatest = (): void => {
    const latest = this.#saves.find((summary) => summary.status !== "corrupt");
    if (latest) void this.#load(latest);
  };

  readonly #openLoad = (): void => {
    if (this.#busy) return;
    this.showLoad();
  };

  readonly #openSettings = (): void => {
    if (this.#busy) return;
    this.#clearError();
    this.#dom.settingsLanguage.value = this.#localization.locale;
    this.#dom.settingsInput.value = this.#getInputPreset();
    this.#showView("settings");
  };

  readonly #backToTitle = (): void => {
    if (this.#busy) return;
    this.#clearError();
    this.#showView("title");
  };

  readonly #startNewGame = (event: SubmitEvent): void => {
    event.preventDefault();
    if (this.#busy) return;
    const seed = canonicalSessionSeed(this.#dom.seedInput.value);
    if (!seed) {
      this.#dom.error.textContent = this.#localization.format("session-seed-invalid");
      this.#dom.seedInput.focus();
      return;
    }
    const buildId = this.#selectedBuild();
    if (!buildId) {
      this.#dom.error.textContent = this.#localization.format("session-build-invalid");
      return;
    }
    const raceId = this.#selectedRace();
    if (!raceId) {
      this.#dom.error.textContent = this.#localization.format("session-race-invalid");
      this.#dom.raceSelect.focus();
      return;
    }
    const playerName = canonicalCharacterName(this.#dom.characterNameInput.value);
    if (!playerName) {
      this.#dom.error.textContent = this.#localization.format("session-character-name-invalid");
      this.#dom.characterNameInput.focus();
      return;
    }
    void this.#start(createNewSessionRequest(seed, buildId, raceId, playerName));
  };

  readonly #randomizeSeed = (): void => {
    if (this.#busy) return;
    this.#dom.seedInput.value = this.#randomSeed();
    this.#clearError();
  };

  readonly #refreshSaves = (): void => {
    void this.#refresh();
  };

  readonly #changeLocale = (): void => {
    const locale = this.#dom.settingsLanguage.value;
    if (isSupportedLocale(locale)) this.#onLocaleChange(locale);
  };

  readonly #changeInputPreset = (): void => {
    const preset = this.#dom.settingsInput.value;
    if (isInputPreset(preset)) this.#onInputPresetChange(preset);
  };

  readonly #exit = (): void => {
    if (!this.#busy) void this.#onExit().catch((error) => this.#showError(error));
  };

  async #start(request: NewSessionRequest): Promise<void> {
    this.#setBusy(true, "session-status-starting");
    try {
      const snapshot = await this.#onStart(request);
      this.showGame(snapshot, request);
    } catch (error) {
      this.#showError(error);
    } finally {
      this.#setBusy(false);
    }
  }

  async #load(summary: NativeSaveSummary): Promise<void> {
    if (this.#busy || summary.status === "corrupt") return;
    this.#setBusy(true, "session-status-loading");
    const previousRequest = this.#activeRequest;
    this.#activeRequest = undefined;
    try {
      const result = await this.#storage.load(summary.slotId);
      await this.#onLoad(result, summary);
      this.showGame(result.snapshot);
    } catch (error) {
      this.#activeRequest = previousRequest;
      this.#showError(error);
    } finally {
      this.#setBusy(false);
    }
  }

  async #delete(summary: NativeSaveSummary): Promise<void> {
    if (
      this.#busy ||
      !this.#confirm(
        this.#localization.format("confirm-native-save-delete", {
          name: summary.slotName,
        }),
      )
    ) {
      return;
    }
    this.#setBusy(true, "session-status-deleting-save");
    let deleted = false;
    try {
      await this.#storage.delete(summary.slotId);
      this.#saves = await this.#storage.list();
      this.#renderSaves();
      this.#clearError();
      deleted = true;
    } catch (error) {
      this.#showError(error);
    } finally {
      this.#setBusy(false);
      if (deleted) {
        this.#dom.status.textContent = this.#localization.format("session-status-save-deleted", {
          name: summary.slotName,
        });
      }
    }
  }

  async #refresh(): Promise<void> {
    if (this.#busy) return;
    this.#setBusy(true, "session-status-reading-saves");
    try {
      this.#saves = await this.#storage.list();
      this.#renderSaves();
      this.#clearError();
    } catch (error) {
      this.#showError(error);
    } finally {
      this.#setBusy(false);
    }
  }

  #selectedBuild(): PlaytestBuildId | undefined {
    return [
      this.#dom.warriorBuild,
      this.#dom.highMageDeathBuild,
      this.#dom.archerBuild,
      this.#dom.paladinDeathBuild,
      this.#dom.cavalryBuild,
      this.#dom.sniperBuild,
    ]
      .find((input) => input.checked)?.value as PlaytestBuildId | undefined;
  }

  #selectedRace(): PlaytestRaceId | undefined {
    const raceId = this.#dom.raceSelect.value;
    return PLAYTEST_RACE_IDS.includes(raceId as PlaytestRaceId)
      ? (raceId as PlaytestRaceId)
      : undefined;
  }

  #showView(view: SessionView): void {
    this.#view = view;
    this.#dom.root.dataset.view = view;
    this.#dom.titleView.hidden = view !== "title";
    this.#dom.newGameView.hidden = view !== "new-game";
    this.#dom.loadView.hidden = view !== "load";
    this.#dom.settingsView.hidden = view !== "settings";
    this.#dom.root.ownerDocument.documentElement.dataset.appMode = view;
    this.#updateControls();
  }

  #showShell(view: SessionView): void {
    if (this.#busy) return;
    this.#clearError();
    this.#dom.gameRoot.hidden = true;
    this.#dom.root.hidden = false;
    this.#showView(view);
  }

  #setBusy(busy: boolean, statusKey?: string): void {
    this.#busy = busy;
    if (statusKey) this.#dom.status.textContent = this.#localization.format(statusKey);
    this.#updateControls();
    if (!busy) this.#renderReadyStatus();
  }

  #updateControls(): void {
    const validSave = this.#saves.some((summary) => summary.status !== "corrupt");
    this.#dom.continueButton.disabled = this.#busy || !validSave;
    for (const control of this.#dom.root.querySelectorAll<
      HTMLButtonElement | HTMLInputElement | HTMLSelectElement
    >("button, input, select")) {
      if (control === this.#dom.continueButton) continue;
      control.disabled = this.#busy;
    }
    for (const button of this.#dom.loadList.querySelectorAll<HTMLButtonElement>("button")) {
      const row = button.closest<HTMLElement>(".native-save-item");
      const summary = this.#saves.find((save) => save.slotId === row?.dataset.slotId);
      button.disabled =
        this.#busy ||
        (button.dataset.sessionLoadAction === "load" && summary?.status === "corrupt");
    }
  }

  #renderSaves(): void {
    this.#dom.loadList.replaceChildren();
    if (this.#saves.length === 0) {
      const empty = this.#dom.loadList.ownerDocument.createElement("li");
      empty.className = "native-save-empty";
      empty.textContent = this.#localization.format("native-save-empty");
      this.#dom.loadList.append(empty);
      this.#updateControls();
      return;
    }

    for (const summary of this.#saves) {
      const row = this.#dom.loadList.ownerDocument.createElement("li");
      row.className = "native-save-item";
      row.dataset.slotId = summary.slotId;

      const header = this.#dom.loadList.ownerDocument.createElement("div");
      header.className = "native-save-header";
      const name = this.#dom.loadList.ownerDocument.createElement("span");
      name.className = "native-save-name";
      name.textContent = summary.slotName;
      const status = this.#dom.loadList.ownerDocument.createElement("span");
      status.className = `native-save-status native-save-status-${summary.status}`;
      status.textContent = this.#localization.format(sessionSaveStatusKey(summary.status));
      header.append(name, status);

      const metadata = this.#dom.loadList.ownerDocument.createElement("p");
      metadata.className = "native-save-meta";
      metadata.textContent = this.#metadata(summary);

      const actions = this.#dom.loadList.ownerDocument.createElement("div");
      actions.className = "native-save-actions";
      const load = this.#dom.loadList.ownerDocument.createElement("button");
      load.type = "button";
      load.dataset.sessionLoadAction = "load";
      load.textContent =
        summary.status === "recoverable"
          ? this.#localization.format("action-native-save-recover", {
              backup: summary.recoveryBackup ?? "?",
            })
          : this.#localization.format("action-native-save-load");
      load.disabled = this.#busy || summary.status === "corrupt";
      load.addEventListener("click", () => void this.#load(summary));
      const remove = this.#dom.loadList.ownerDocument.createElement("button");
      remove.type = "button";
      remove.dataset.sessionLoadAction = "delete";
      remove.textContent = this.#localization.format(
        summary.status === "corrupt"
          ? "action-native-save-delete-corrupt"
          : "action-native-save-delete",
      );
      remove.disabled = this.#busy;
      remove.addEventListener("click", () => void this.#delete(summary));
      actions.append(load, remove);
      row.append(header, metadata, actions);
      this.#dom.loadList.append(row);
    }
    this.#updateControls();
  }

  #metadata(summary: NativeSaveSummary): string {
    if (summary.turn === null || summary.savedAt === null) {
      return this.#localization.format("native-save-meta-unavailable");
    }
    const metadata = this.#localization.format("native-save-meta", {
      location:
        summary.locationKey &&
        this.#localization.hasMessage(this.#localization.locale, summary.locationKey)
          ? this.#localization.format(summary.locationKey)
          : this.#localization.format("native-save-location-unknown"),
      turn: summary.turn,
      savedAt: this.#date(summary.savedAt),
    });
    return summary.status === "recoverable"
      ? `${metadata} · ${this.#localization.format("native-save-recovery-meta", {
          backup: summary.recoveryBackup ?? "?",
        })}`
      : metadata;
  }

  #date(savedAt: string): string {
    const date = new Date(savedAt);
    return Number.isNaN(date.getTime())
      ? this.#localization.format("native-save-date-unknown")
      : new Intl.DateTimeFormat(this.#localization.locale, {
          dateStyle: "short",
          timeStyle: "short",
        }).format(date);
  }

  #renderReadyStatus(): void {
    if (this.#busy) return;
    this.#dom.status.textContent = this.#localization.format("session-status-ready", {
      saves: this.#saves.filter((summary) => summary.status !== "corrupt").length,
    });
  }

  #renderRunMetadata(): void {
    const snapshot = this.#activeSnapshot;
    if (!snapshot) return;
    const build = snapshot.player.build;
    this.#dom.runBuildValue.textContent = build
      ? this.#localization.format(build.buildNameKey)
      : this.#localization.format("session-build-unknown");
    this.#dom.runSeedValue.textContent =
      this.#activeRequest?.seed ?? this.#localization.format("run-seed-loaded-save");
  }

  #clearError(): void {
    this.#dom.error.replaceChildren();
  }

  #showError(error: unknown): void {
    const code = desktopErrorCode(error);
    this.#dom.error.textContent =
      code === "desktop-storage-unknown"
        ? this.#localization.format("session-error", { error: errorMessage(error) })
        : this.#localization.format(nativeSaveErrorKey(code), { code });
    this.#logError(error);
  }
}

export function createSessionShellDom(document: DocumentLookup): SessionShellDom {
  return {
    root: element<HTMLElement>(document, "session-shell"),
    gameRoot: element<HTMLElement>(document, "app"),
    titleView: element<HTMLElement>(document, "session-title-view"),
    newGameView: element<HTMLFormElement>(document, "session-new-game-view"),
    loadView: element<HTMLElement>(document, "session-load-view"),
    settingsView: element<HTMLElement>(document, "session-settings-view"),
    newGameButton: element<HTMLButtonElement>(document, "session-new-game"),
    continueButton: element<HTMLButtonElement>(document, "session-continue"),
    loadGameButton: element<HTMLButtonElement>(document, "session-load-game"),
    settingsButton: element<HTMLButtonElement>(document, "session-settings"),
    exitButton: element<HTMLButtonElement>(document, "session-exit"),
    warriorBuild: element<HTMLInputElement>(document, "session-build-warrior"),
    highMageDeathBuild: element<HTMLInputElement>(document, "session-build-high-mage-death"),
    archerBuild: element<HTMLInputElement>(document, "session-build-archer"),
    paladinDeathBuild: element<HTMLInputElement>(document, "session-build-paladin-death"),
    cavalryBuild: element<HTMLInputElement>(document, "session-build-cavalry"),
    sniperBuild: element<HTMLInputElement>(document, "session-build-sniper"),
    raceSelect: element<HTMLSelectElement>(document, "session-race"),
    characterNameInput: element<HTMLInputElement>(document, "session-character-name"),
    seedInput: element<HTMLInputElement>(document, "session-seed"),
    randomizeSeedButton: element<HTMLButtonElement>(document, "session-randomize-seed"),
    startGameButton: element<HTMLButtonElement>(document, "session-start-game"),
    newGameBackButton: element<HTMLButtonElement>(document, "session-new-game-back"),
    loadRefreshButton: element<HTMLButtonElement>(document, "session-load-refresh"),
    loadList: element<HTMLUListElement>(document, "session-load-list"),
    loadBackButton: element<HTMLButtonElement>(document, "session-load-back"),
    settingsLanguage: element<HTMLSelectElement>(document, "session-settings-language"),
    settingsInput: element<HTMLSelectElement>(document, "session-settings-input"),
    settingsBackButton: element<HTMLButtonElement>(document, "session-settings-back"),
    status: element<HTMLElement>(document, "session-status"),
    error: element<HTMLElement>(document, "session-error"),
    runBuildValue: element<HTMLElement>(document, "run-build-value"),
    runSeedValue: element<HTMLElement>(document, "run-seed-value"),
  };
}

export function canonicalSessionSeed(value: string): string | undefined {
  const trimmed = value.trim();
  if (!/^\d+$/.test(trimmed)) return undefined;
  try {
    const parsed = BigInt(trimmed);
    return parsed <= MAX_SESSION_SEED ? parsed.toString() : undefined;
  } catch {
    return undefined;
  }
}

export function canonicalCharacterName(value: string): string | undefined {
  const name = value.trim();
  const length = Array.from(name).length;
  return length > 0 && length <= MAX_CHARACTER_NAME_LENGTH && !/[\u0000-\u001f\u007f]/u.test(name)
    ? name
    : undefined;
}

export function randomSessionSeed(
  source: Pick<Crypto, "getRandomValues"> = globalThis.crypto,
): string {
  const values = source.getRandomValues(new Uint32Array(2));
  const high = BigInt(values[0] ?? 0);
  const low = BigInt(values[1] ?? 0);
  return ((high << 32n) | low).toString();
}

function sessionSaveStatusKey(status: NativeSaveSummary["status"]): string {
  return `native-save-status-${status}`;
}

function isInputPreset(value: string): value is InputPreset {
  return value === "numpad" || value === "vi" || value === "wasd";
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function element<T extends HTMLElement>(document: DocumentLookup, id: string): T {
  const found = document.getElementById(id);
  if (!found) throw new Error(`Missing element #${id}`);
  return found as T;
}
