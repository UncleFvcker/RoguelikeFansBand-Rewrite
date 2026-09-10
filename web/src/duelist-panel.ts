// SPDX-License-Identifier: MPL-2.0

import type { AppState } from "./app-state";
import type { Localization } from "./localization";
import type { AbilityDto, DuelistChoiceDto, GameCommand } from "./protocol";

// The core owns the challenge and suspended action. This panel only selects a response.
export class DuelistPanel {
  readonly #state: AppState;
  readonly #localization: Localization;
  readonly #contentName: (id: string) => string;
  readonly #dispatch: (command: GameCommand) => Promise<void>;
  readonly #startTargeting: (ability: AbilityDto) => void;
  readonly #beforePrompt: () => void;
  readonly #status: HTMLElement;
  readonly #value: HTMLElement;
  readonly #reason: HTMLElement;
  readonly #mark: HTMLButtonElement;
  readonly #clear: HTMLButtonElement;
  readonly #dialog: HTMLDialogElement;
  readonly #title: HTMLElement;
  readonly #description: HTMLElement;
  readonly #target: HTMLSelectElement;
  readonly #accept: HTMLButtonElement;
  readonly #decline: HTMLButtonElement;

  constructor(options: {
    document: Document;
    state: AppState;
    localization: Localization;
    contentName: (id: string) => string;
    dispatch: (command: GameCommand) => Promise<void>;
    startTargeting: (ability: AbilityDto) => void;
    beforePrompt: () => void;
  }) {
    this.#state = options.state;
    this.#localization = options.localization;
    this.#contentName = options.contentName;
    this.#dispatch = options.dispatch;
    this.#startTargeting = options.startTargeting;
    this.#beforePrompt = options.beforePrompt;
    const element = <T extends HTMLElement>(id: string) => options.document.getElementById(id) as T;
    this.#status = element("duelist-status");
    this.#value = element("duelist-status-value");
    this.#reason = element("duelist-status-reason");
    this.#mark = element("duelist-mark");
    this.#clear = element("duelist-clear");
    this.#dialog = element("duelist-choice-dialog");
    this.#title = element("duelist-choice-title");
    this.#description = element("duelist-choice-description");
    this.#target = element("duelist-choice-target");
    this.#accept = element("duelist-choice-accept");
    this.#decline = element("duelist-choice-decline");
    this.#mark.addEventListener("click", () => {
      const ability = this.#markAbility;
      if (ability?.canCast && !this.#state.busy && !this.#state.commandBlocked) this.#startTargeting(ability);
    });
    this.#clear.addEventListener("click", () => void this.#dispatch({ type: "clear-duelist-challenge" }));
    this.#accept.addEventListener("click", () => this.#respond(true));
    this.#decline.addEventListener("click", () => this.#respond(false));
    this.#dialog.addEventListener("cancel", event => { event.preventDefault(); this.#respond(false); });
  }

  get #markAbility(): AbilityDto | undefined {
    return this.#state.status?.player.abilities?.find(ability => ability.effects.some(effect => effect.type === "duelist-challenge"));
  }

  render(): void {
    const snapshot = this.#state.status;
    const player = snapshot?.player;
    const format = this.#localization.format.bind(this.#localization);
    this.#status.hidden = player?.build?.classId !== "demo.class.duelist";
    const targetId = player?.duelistTargetId;
    const target = snapshot?.entities.find(entity => entity.id === targetId);
    this.#value.textContent = !targetId ? format("duelist-challenge-none")
      : target ? format("duelist-challenge-current", { target: this.#contentName(target.kindId), x: target.position.x, y: target.position.y })
      : format("duelist-challenge-unseen");
    this.#value.title = format("duelist-auto-challenge-help");
    this.#status.dataset.targetId = targetId ?? "";
    const ability = this.#markAbility;
    this.#mark.textContent = format(targetId ? "duelist-challenge-reselect" : "ability-demo-duelist-mark-target-name");
    this.#mark.disabled = this.#state.busy || this.#state.commandBlocked || this.#state.worldMap || !ability?.canCast;
    this.#mark.title = ability?.unavailableReason ? format(`ability-unavailable-${ability.unavailableReason}`) : "";
    this.#reason.hidden = !ability?.unavailableReason;
    this.#reason.textContent = ability?.unavailableReason ? format("duelist-challenge-unavailable", { reason: this.#mark.title }) : "";
    this.#clear.textContent = format("duelist-challenge-clear");
    this.#clear.title = format("duelist-challenge-clear-help");
    this.#clear.disabled = this.#state.busy || this.#state.commandBlocked || this.#state.worldMap || !targetId;

    const prompt = player?.pendingDuelist;
    if (!prompt) {
      if (this.#dialog.open) this.#dialog.close();
      return;
    }
    this.#dialog.dataset.prompt = prompt.type;
    this.#title.textContent = format(`duelist-choice-${prompt.type}-title`);
    const source = snapshot?.entities.find(entity => entity.id === ("sourceEntityId" in prompt ? prompt.sourceEntityId : "targetEntityId" in prompt ? prompt.targetEntityId : ""));
    this.#description.textContent = format(`duelist-choice-${prompt.type}-help`, {
      target: source ? this.#contentName(source.kindId) : format("duelist-challenge-unseen-target"),
      distance: prompt.type === "charge" ? prompt.distance : 0,
      range: prompt.type === "charge" ? prompt.range : 0,
    });
    this.#target.hidden = prompt.type !== "challenge";
    this.#target.setAttribute("aria-label", format("duelist-choice-challenge-title"));
    if (prompt.type === "challenge") {
      const selected = this.#target.value;
      const document = this.#target.ownerDocument;
      this.#target.replaceChildren(...(snapshot?.entities ?? [])
        .filter(entity => entity.id !== player?.ridingActorId)
        .map(entity => {
          const option = document.createElement("option");
          option.value = entity.id;
          option.textContent = format("duelist-choice-target", { target: this.#contentName(entity.kindId), x: entity.position.x, y: entity.position.y });
          return option;
        }));
      if ([...this.#target.options].some(option => option.value === selected)) this.#target.value = selected;
    }
    this.#accept.textContent = format(prompt.type === "challenge" ? "ability-demo-duelist-mark-target-name" : "duelist-choice-accept");
    this.#decline.textContent = format(prompt.type === "challenge" ? "duelist-choice-skip" : "duelist-choice-decline");
    this.#target.disabled = this.#state.busy;
    this.#accept.disabled = this.#state.busy || (prompt.type === "challenge" && !this.#target.value);
    this.#decline.disabled = this.#state.busy;
    if (!this.#dialog.open) {
      this.#beforePrompt();
      this.#dialog.showModal();
      (prompt.type === "challenge" ? this.#target : this.#decline).focus();
    }
  }

  #respond(accepted: boolean): void {
    const prompt = this.#state.status?.player.pendingDuelist;
    if (this.#state.busy || !prompt || (accepted && this.#accept.disabled)) return;
    const choice: DuelistChoiceDto = prompt.type === "challenge"
      ? { type: "challenge", entityId: accepted ? this.#target.value : null }
      : { type: "confirm", accepted };
    void this.#dispatch({ type: "resolve-duelist-choice", choice });
  }
}
