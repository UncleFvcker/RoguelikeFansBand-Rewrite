// SPDX-License-Identifier: MPL-2.0

import type { AppState } from "./app-state";
import type { Localization } from "./localization";
import type { GameCommand } from "./protocol";

export class SpellRealmsPanel {
  readonly #state: AppState;
  readonly #localization: Localization;
  readonly #dispatch: (command: GameCommand) => Promise<void>;
  readonly #afterPrompt: () => void;
  readonly #root: HTMLElement;
  readonly #value: HTMLElement;
  readonly #history: HTMLElement;
  readonly #studyHelp: HTMLElement;
  readonly #weaponNote: HTMLElement;
  readonly #books: HTMLElement;
  readonly #dialog: HTMLDialogElement;
  readonly #title: HTMLElement;
  readonly #description: HTMLElement;
  readonly #accept: HTMLButtonElement;
  readonly #decline: HTMLButtonElement;

  constructor(options: {
    document: Document;
    state: AppState;
    localization: Localization;
    dispatch: (command: GameCommand) => Promise<void>;
    afterPrompt: () => void;
  }) {
    this.#state = options.state;
    this.#localization = options.localization;
    this.#dispatch = options.dispatch;
    this.#afterPrompt = options.afterPrompt;
    const element = <T extends HTMLElement>(id: string) => options.document.getElementById(id) as T;
    this.#root = element("spell-realms");
    this.#value = element("spell-realms-value");
    this.#history = element("spell-realms-history");
    this.#studyHelp = element("spell-realms-study-help");
    this.#weaponNote = element("spell-realms-weapon-note");
    this.#books = element("realm-change-books");
    this.#dialog = element("realm-change-dialog");
    this.#title = element("realm-change-title");
    this.#description = element("realm-change-description");
    this.#accept = element("realm-change-accept");
    this.#decline = element("realm-change-decline");
    this.#accept.addEventListener("click", () => this.#respond(true));
    this.#decline.addEventListener("click", () => this.#respond(false));
    this.#dialog.addEventListener("cancel", event => { event.preventDefault(); this.#respond(false); });
  }

  render(): void {
    const realms = this.#state.status?.player.abilityLearning?.realms;
    this.#root.hidden = !realms;
    const format = this.#localization.format.bind(this.#localization);
    const name = (id: string) => format(`realm-${id}-name`);
    if (realms) {
      this.#studyHelp.textContent = format(this.#state.status?.player.abilityLearning?.studyMode === "divine-random"
        ? "ability-random-study-help" : "ability-mage-study-help");
      const penalty = this.#state.status?.player.traitDetails?.stats.find(stat => stat.id === "priest-blade-failure");
      this.#weaponNote.hidden = !penalty;
      this.#weaponNote.textContent = penalty ? format(penalty.value == null
        ? "ability-priest-blade-unknown" : "ability-priest-blade-penalty", { value: penalty.value ?? 0 }) : "";
      this.#value.textContent = format("ability-realms-value", { first: name(realms.firstRealmId), second: name(realms.secondRealmId) });
      this.#history.hidden = realms.previousRealmIds.length === 0;
      this.#history.textContent = format("ability-realms-history", { realms: realms.previousRealmIds.map(name).join(" / ") });
      const focused = (this.#books.ownerDocument.activeElement as HTMLElement | null)?.dataset.realmId;
      // A realm needs only one button; each command uses a book approved by the core.
      const choices = new Map<string, string>();
      for (const book of realms.changeBooks) if (!choices.has(book.realmId)) choices.set(book.realmId, book.bookItemId);
      this.#books.replaceChildren(...[...choices].map(([realmId, bookItemId]) => {
        const button = this.#books.ownerDocument.createElement("button");
        button.type = "button";
        button.dataset.realmId = realmId;
        button.textContent = format("realm-change-begin", { realm: name(realmId) });
        button.disabled = this.#state.busy || this.#state.commandBlocked || this.#state.worldMap;
        button.addEventListener("click", () => void this.#dispatch({ type: "begin-realm-change", bookItemId }));
        return button;
      }));
      if (focused) this.#books.querySelector<HTMLButtonElement>(`[data-realm-id="${focused}"]`)?.focus();
    }
    const prompt = realms?.pendingChange;
    if (!prompt) {
      if (this.#dialog.open) {
        this.#dialog.close();
        if (realms) {
          this.#afterPrompt();
          this.#value.focus();
        }
      }
      return;
    }
    this.#title.textContent = format("realm-change-title", { realm: name(prompt.realmId) });
    this.#description.textContent = format(this.#state.status?.player.abilityLearning?.studyMode === "divine-random"
      ? "realm-change-random-description" : "realm-change-description", {
      old: name(realms.secondRealmId), first: name(realms.firstRealmId), next: name(prompt.realmId),
    });
    this.#accept.disabled = this.#decline.disabled = this.#state.busy;
    if (!this.#dialog.open) {
      this.#dialog.showModal();
      this.#decline.focus();
    }
  }

  #respond(confirm: boolean): void {
    if (this.#state.busy || !this.#state.status?.player.abilityLearning?.realms?.pendingChange) return;
    void this.#dispatch({ type: "resolve-realm-change", confirm });
  }
}
