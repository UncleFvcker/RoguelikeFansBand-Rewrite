// SPDX-License-Identifier: MPL-2.0

import type { Localization, MessageKey } from "./localization";
import {
  desktopErrorCode,
  nativeSaveErrorCategory,
  type NativeLoadResult,
  type NativeSaveStorage,
  type NativeSaveSummary,
} from "./native-save-storage.ts";
import type { GameSnapshot } from "./protocol";

type SaveStorage = Pick<NativeSaveStorage, "list" | "save" | "load" | "delete">;

export class NativeSavePanel {
  readonly #storage: SaveStorage;
  readonly #localization: Localization;
  readonly #nameInput: HTMLInputElement;
  readonly #createButton: HTMLButtonElement;
  readonly #refreshButton: HTMLButtonElement;
  readonly #list: HTMLUListElement;
  readonly #isGameBusy: () => boolean;
  readonly #setGameBusy: (busy: boolean) => void;
  readonly #applySnapshot: (snapshot: GameSnapshot) => void;
  readonly #announce: (
    key: MessageKey,
    args: Record<string, string | number> | undefined,
    kind: string,
  ) => void;
  readonly #confirm: (message: string) => boolean;
  readonly #logError: (error: unknown) => void;
  #busy = false;
  #installed = false;
  #saves: NativeSaveSummary[] = [];

  constructor(options: {
    storage: SaveStorage;
    localization: Localization;
    nameInput: HTMLInputElement;
    createButton: HTMLButtonElement;
    refreshButton: HTMLButtonElement;
    list: HTMLUListElement;
    isGameBusy: () => boolean;
    setGameBusy: (busy: boolean) => void;
    applySnapshot: (snapshot: GameSnapshot) => void;
    announce: (
      key: MessageKey,
      args: Record<string, string | number> | undefined,
      kind: string,
    ) => void;
    confirm: (message: string) => boolean;
    logError?: (error: unknown) => void;
  }) {
    this.#storage = options.storage;
    this.#localization = options.localization;
    this.#nameInput = options.nameInput;
    this.#createButton = options.createButton;
    this.#refreshButton = options.refreshButton;
    this.#list = options.list;
    this.#isGameBusy = options.isGameBusy;
    this.#setGameBusy = options.setGameBusy;
    this.#applySnapshot = options.applySnapshot;
    this.#announce = options.announce;
    this.#confirm = options.confirm;
    this.#logError = options.logError ?? console.error;
  }

  install(): void {
    if (this.#installed) return;
    this.#installed = true;
    this.#createButton.addEventListener("click", () => void this.#create());
    this.#refreshButton.addEventListener("click", () => void this.refresh());
    this.#nameInput.addEventListener("input", () => this.#updateControls());
  }

  async refresh(): Promise<void> {
    if (this.#busy) return;
    this.#busy = true;
    this.#updateControls();
    try {
      this.#saves = await this.#storage.list();
      this.#render();
    } catch (error) {
      this.#showError(error);
    } finally {
      this.#busy = false;
      this.#updateControls();
    }
  }

  localize(): void {
    this.#nameInput.placeholder = this.#localization.format("native-save-name-placeholder");
    this.#render();
  }

  async #create(): Promise<void> {
    const slotName = this.#nameInput.value.trim();
    if (this.#busy || !slotName) return;
    this.#busy = true;
    this.#updateControls();
    try {
      const summary = await this.#storage.save(slotName);
      this.#nameInput.value = "";
      this.#replaceSummary(summary);
      this.#announce("message-native-save-created", { name: summary.slotName }, "system");
    } catch (error) {
      this.#showError(error);
    } finally {
      this.#busy = false;
      this.#updateControls();
    }
  }

  async #overwrite(summary: NativeSaveSummary): Promise<void> {
    if (this.#busy) return;
    this.#busy = true;
    this.#updateControls();
    try {
      const updated = await this.#storage.save(summary.slotName, summary.slotId);
      this.#replaceSummary(updated);
      this.#announce("message-native-save-overwritten", { name: updated.slotName }, "system");
    } catch (error) {
      this.#showError(error);
    } finally {
      this.#busy = false;
      this.#updateControls();
    }
  }

  async #load(summary: NativeSaveSummary): Promise<void> {
    if (this.#busy || this.#isGameBusy() || summary.status === "corrupt") return;
    this.#busy = true;
    this.#setGameBusy(true);
    this.#updateControls();
    try {
      const result = await this.#storage.load(summary.slotId);
      this.#applySnapshot(result.snapshot);
      this.#announceLoad(summary, result);
      this.#saves = await this.#storage.list();
      this.#render();
    } catch (error) {
      this.#showError(error);
    } finally {
      this.#setGameBusy(false);
      this.#busy = false;
      this.#updateControls();
    }
  }

  async #delete(summary: NativeSaveSummary): Promise<void> {
    if (
      this.#busy ||
      !this.#confirm(
        this.#localization.format("confirm-native-save-delete", { name: summary.slotName }),
      )
    ) {
      return;
    }
    this.#busy = true;
    this.#updateControls();
    try {
      await this.#storage.delete(summary.slotId);
      this.#saves = this.#saves.filter((save) => save.slotId !== summary.slotId);
      this.#render();
      this.#announce("message-native-save-deleted", { name: summary.slotName }, "system");
    } catch (error) {
      this.#showError(error);
    } finally {
      this.#busy = false;
      this.#updateControls();
    }
  }

  #announceLoad(summary: NativeSaveSummary, result: NativeLoadResult): void {
    if (result.recoveryBackup === null) {
      this.#announce("message-native-save-loaded", { name: summary.slotName }, "system");
    } else {
      this.#announce(
        "message-native-save-backup-loaded",
        { name: summary.slotName, backup: result.recoveryBackup },
        "system",
      );
    }
  }

  #replaceSummary(summary: NativeSaveSummary): void {
    this.#saves = [summary, ...this.#saves.filter((save) => save.slotId !== summary.slotId)];
    this.#render();
  }

  #render(): void {
    this.#list.replaceChildren();
    if (this.#saves.length === 0) {
      const empty = this.#list.ownerDocument.createElement("li");
      empty.className = "native-save-empty";
      empty.textContent = this.#localization.format("native-save-empty");
      this.#list.append(empty);
      this.#updateControls();
      return;
    }

    for (const summary of this.#saves) {
      const row = this.#list.ownerDocument.createElement("li");
      row.className = "native-save-item";
      row.dataset.slotId = summary.slotId;

      const header = this.#list.ownerDocument.createElement("div");
      header.className = "native-save-header";
      const name = this.#list.ownerDocument.createElement("span");
      name.className = "native-save-name";
      name.textContent = summary.slotName;
      name.title = summary.slotName;
      const status = this.#list.ownerDocument.createElement("span");
      status.className = `native-save-status native-save-status-${summary.status}`;
      status.textContent = this.#localization.format(nativeSaveStatusKey(summary.status));
      header.append(name, status);

      const metadata = this.#list.ownerDocument.createElement("p");
      metadata.className = "native-save-meta";
      metadata.textContent = this.#metadata(summary);

      const actions = this.#list.ownerDocument.createElement("div");
      actions.className = "native-save-actions";
      const load = this.#actionButton("load", "action-native-save-load", () =>
        void this.#load(summary),
      );
      load.disabled = summary.status === "corrupt" || this.#busy || this.#isGameBusy();
      const overwrite = this.#actionButton(
        "overwrite",
        "action-native-save-overwrite",
        () => void this.#overwrite(summary),
      );
      overwrite.disabled = this.#busy;
      const remove = this.#actionButton("delete", "action-native-save-delete", () =>
        void this.#delete(summary),
      );
      remove.disabled = this.#busy;
      actions.append(load, overwrite, remove);

      row.append(header, metadata, actions);
      this.#list.append(row);
    }
    this.#updateControls();
  }

  #actionButton(actionName: string, key: MessageKey, action: () => void): HTMLButtonElement {
    const button = this.#list.ownerDocument.createElement("button");
    button.type = "button";
    button.dataset.nativeSaveAction = actionName;
    button.textContent = this.#localization.format(key);
    button.addEventListener("click", action);
    return button;
  }

  #metadata(summary: NativeSaveSummary): string {
    if (summary.turn === null || summary.savedAt === null) {
      return this.#localization.format("native-save-meta-unavailable");
    }
    return this.#localization.format("native-save-meta", {
      location:
        summary.locationKey === "world-demo-original-lab-name"
          ? this.#localization.format("world-demo-original-lab-name")
          : this.#localization.format("native-save-location-unknown"),
      turn: summary.turn,
      savedAt: this.#date(summary.savedAt),
    });
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

  #updateControls(): void {
    this.#nameInput.disabled = this.#busy;
    this.#createButton.disabled = this.#busy || this.#nameInput.value.trim().length === 0;
    this.#refreshButton.disabled = this.#busy;
    for (const button of this.#list.querySelectorAll<HTMLButtonElement>("button")) {
      const row = button.closest<HTMLElement>(".native-save-item");
      const summary = this.#saves.find((save) => save.slotId === row?.dataset.slotId);
      button.disabled =
        this.#busy ||
        (button.dataset.nativeSaveAction === "load" &&
          (this.#isGameBusy() || summary?.status === "corrupt"));
    }
  }

  #showError(error: unknown): void {
    this.#announce("message-native-save-failed", { code: desktopErrorCode(error) }, "error");
    this.#logError(error);
  }
}

function nativeSaveStatusKey(status: NativeSaveSummary["status"]): MessageKey {
  const keys: Record<NativeSaveSummary["status"], MessageKey> = {
    ready: "native-save-status-ready",
    recoverable: "native-save-status-recoverable",
    corrupt: "native-save-status-corrupt",
  };
  return keys[status];
}

export function nativeSaveErrorKey(code: string): MessageKey {
  switch (nativeSaveErrorCategory(code)) {
    case "name-invalid":
      return "native-save-error-name-invalid";
    case "not-found":
      return "native-save-error-not-found";
    case "corrupt":
      return "native-save-error-corrupt";
    case "read":
      return "native-save-error-read";
    case "write":
      return "native-save-error-write";
    case "unavailable":
      return "native-save-error-unavailable";
    case "internal":
      return "native-save-error-internal";
  }
}
