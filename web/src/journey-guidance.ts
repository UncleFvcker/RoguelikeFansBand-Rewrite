// SPDX-License-Identifier: MPL-2.0

import type { AppDom } from "./app-dom";
import type { InputPreset } from "./input-controller";
import type { Localization, MessageKey } from "./localization";
import type { GameCommand, GameSnapshot, GameUpdate } from "./protocol";

type JourneyState = GameSnapshot | GameUpdate;

export type JourneyObjectiveId =
  | "prepare"
  | "enter"
  | "descend"
  | "guardian"
  | "return"
  | "retire"
  | "complete";

export interface JourneyObjective {
  readonly id: JourneyObjectiveId;
  readonly depth?: number;
  readonly returningFromOtherFloor?: boolean;
}

export type OnboardingPromptId =
  | "movement"
  | "look"
  | "pickup"
  | "inventory"
  | "equipment"
  | "combat"
  | "stairs"
  | "resources"
  | "messages"
  | "save";

export type OnboardingPromptKind = "journey" | "optional";

export interface OnboardingPrompt {
  readonly id: OnboardingPromptId;
  readonly kind: OnboardingPromptKind;
  readonly titleKey: MessageKey;
  readonly detailKey: MessageKey;
  readonly controlKey: MessageKey;
}

export type GuidanceInteraction = "look" | "inventory" | "targeting" | "save";

type GuidanceDom = Pick<
  AppDom,
  | "journeyPanel"
  | "journeyObjectiveTitle"
  | "journeyObjectiveDetail"
  | "journeyLocation"
  | "onboardingKind"
  | "onboardingTitle"
  | "onboardingDetail"
  | "onboardingControl"
  | "onboardingProgress"
  | "onboardingHideOptional"
  | "onboardingReset"
>;

type GuidanceStorage = Pick<Storage, "getItem" | "setItem" | "removeItem">;

const COMPLETED_STORAGE_KEY = "rfb.journey-guidance.completed.v1";
const HIDE_OPTIONAL_STORAGE_KEY = "rfb.journey-guidance.hide-optional.v1";

export const ONBOARDING_PROMPTS: readonly OnboardingPrompt[] = [
  {
    id: "movement",
    kind: "journey",
    titleKey: "onboarding-movement-title",
    detailKey: "onboarding-movement-detail",
    controlKey: "onboarding-movement-control-numpad",
  },
  {
    id: "look",
    kind: "optional",
    titleKey: "onboarding-look-title",
    detailKey: "onboarding-look-detail",
    controlKey: "onboarding-look-control",
  },
  {
    id: "pickup",
    kind: "journey",
    titleKey: "onboarding-pickup-title",
    detailKey: "onboarding-pickup-detail",
    controlKey: "onboarding-pickup-control",
  },
  {
    id: "inventory",
    kind: "optional",
    titleKey: "onboarding-inventory-title",
    detailKey: "onboarding-inventory-detail",
    controlKey: "onboarding-inventory-control",
  },
  {
    id: "equipment",
    kind: "journey",
    titleKey: "onboarding-equipment-title",
    detailKey: "onboarding-equipment-detail",
    controlKey: "onboarding-equipment-control",
  },
  {
    id: "combat",
    kind: "journey",
    titleKey: "onboarding-combat-title",
    detailKey: "onboarding-combat-detail",
    controlKey: "onboarding-combat-control",
  },
  {
    id: "stairs",
    kind: "journey",
    titleKey: "onboarding-stairs-title",
    detailKey: "onboarding-stairs-detail",
    controlKey: "onboarding-stairs-control",
  },
  {
    id: "resources",
    kind: "optional",
    titleKey: "onboarding-resources-title",
    detailKey: "onboarding-resources-detail",
    controlKey: "onboarding-resources-control",
  },
  {
    id: "messages",
    kind: "optional",
    titleKey: "onboarding-messages-title",
    detailKey: "onboarding-messages-detail",
    controlKey: "onboarding-messages-control",
  },
  {
    id: "save",
    kind: "optional",
    titleKey: "onboarding-save-title",
    detailKey: "onboarding-save-detail",
    controlKey: "onboarding-save-control",
  },
] as const;

const PROMPT_IDS = new Set<OnboardingPromptId>(
  ONBOARDING_PROMPTS.map((prompt) => prompt.id),
);

export function selectJourneyObjective(state: JourneyState): JourneyObjective {
  if (state.campaign.status === "retired") return { id: "complete" };
  if (state.campaign.status === "victorious") {
    return state.floorId === "demo.floor.surface" ? { id: "retire" } : { id: "return" };
  }

  const depth = echoDepth(state.floorId);
  if (depth !== undefined) {
    return depth >= 3 ? { id: "guardian", depth } : { id: "descend", depth };
  }
  if (state.floorId !== "demo.floor.surface") {
    return { id: "enter", returningFromOtherFloor: true };
  }
  return state.inventory.length === 0 && state.equipment.length === 0
    ? { id: "prepare" }
    : { id: "enter" };
}

export function selectOnboardingPrompt(
  state: JourneyState,
  completed: ReadonlySet<OnboardingPromptId>,
  hideOptional: boolean,
): OnboardingPrompt | undefined {
  return ONBOARDING_PROMPTS.find(
    (prompt) =>
      !completed.has(prompt.id) &&
      (!hideOptional || prompt.kind === "journey") &&
      promptApplies(prompt.id, state),
  );
}

export function completedPromptsForUpdate(
  command: GameCommand,
  before: JourneyState | undefined,
  update: GameUpdate,
  currentPrompt: OnboardingPromptId | undefined,
): ReadonlySet<OnboardingPromptId> {
  const completed = new Set<OnboardingPromptId>();
  if (before && !samePosition(before.player.position, update.player.position)) {
    completed.add("movement");
  }
  if (update.events.some((event) => event.messageKey === "item-pickup-success")) {
    completed.add("pickup");
  }
  if (
    (command.type === "equip" || command.type === "use-item") &&
    update.events.some((event) => successfulEquipmentOrUseEvent(event.messageKey))
  ) {
    completed.add("equipment");
  }
  if (update.events.some((event) => combatOrTargetingEvent(event.kind, event.messageKey))) {
    completed.add("combat");
  }
  if (before && before.floorId !== update.floorId) completed.add("stairs");
  if (
    command.type === "rest" &&
    update.events.some((event) => event.messageKey.startsWith("rest-"))
  ) {
    completed.add("resources");
  }
  if (currentPrompt === "messages" && update.events.length > 0) completed.add("messages");
  return completed;
}

export class JourneyGuidance {
  readonly #dom: GuidanceDom;
  readonly #localization: Localization;
  readonly #storage: GuidanceStorage;
  readonly #getInputPreset: () => InputPreset;
  readonly #completed: Set<OnboardingPromptId>;
  #hideOptional: boolean;
  #state: JourneyState | undefined;
  #currentPrompt: OnboardingPromptId | undefined;
  #installed = false;

  constructor(options: {
    dom: GuidanceDom;
    localization: Localization;
    storage: GuidanceStorage;
    getInputPreset: () => InputPreset;
  }) {
    this.#dom = options.dom;
    this.#localization = options.localization;
    this.#storage = options.storage;
    this.#getInputPreset = options.getInputPreset;
    this.#completed = readCompletedPrompts(options.storage);
    this.#hideOptional = options.storage.getItem(HIDE_OPTIONAL_STORAGE_KEY) === "true";
  }

  install(): void {
    if (this.#installed) return;
    this.#installed = true;
    this.#dom.onboardingHideOptional.addEventListener("change", this.#handleHideOptional);
    this.#dom.onboardingReset.addEventListener("click", this.#handleReset);
  }

  dispose(): void {
    if (!this.#installed) return;
    this.#installed = false;
    this.#dom.onboardingHideOptional.removeEventListener("change", this.#handleHideOptional);
    this.#dom.onboardingReset.removeEventListener("click", this.#handleReset);
  }

  render(state: JourneyState): void {
    this.#state = state;
    const objective = selectJourneyObjective(state);
    this.#dom.journeyPanel.dataset.objectiveId = objective.id;
    this.#dom.journeyObjectiveTitle.textContent = this.#localization.format(
      `journey-objective-${objective.id}-title`,
    );
    this.#dom.journeyObjectiveDetail.textContent = this.#localization.format(
      objectiveDetailKey(objective),
      objective.depth === undefined
        ? undefined
        : { depth: objective.depth, nextDepth: objective.depth + 1 },
    );
    this.#dom.journeyLocation.textContent = formatJourneyLocation(
      this.#localization,
      state.floorId,
    );

    const prompt = selectOnboardingPrompt(state, this.#completed, this.#hideOptional);
    this.#currentPrompt = prompt?.id;
    this.#dom.journeyPanel.dataset.promptId = prompt?.id ?? "complete";
    this.#dom.journeyPanel.dataset.promptKind = prompt?.kind ?? "complete";
    this.#dom.onboardingHideOptional.checked = this.#hideOptional;
    this.#dom.onboardingProgress.textContent = this.#localization.format(
      "onboarding-progress",
      {
        completed: this.#completed.size,
        total: ONBOARDING_PROMPTS.length,
      },
    );
    if (!prompt) {
      this.#dom.onboardingKind.textContent = this.#localization.format(
        "onboarding-kind-complete",
      );
      this.#dom.onboardingTitle.textContent = this.#localization.format(
        "onboarding-complete-title",
      );
      this.#dom.onboardingDetail.textContent = this.#localization.format(
        "onboarding-complete-detail",
      );
      this.#dom.onboardingControl.replaceChildren();
      return;
    }
    this.#dom.onboardingKind.textContent = this.#localization.format(
      `onboarding-kind-${prompt.kind}`,
    );
    this.#dom.onboardingTitle.textContent = this.#localization.format(prompt.titleKey);
    this.#dom.onboardingDetail.textContent = this.#localization.format(prompt.detailKey);
    this.#dom.onboardingControl.textContent = this.#localization.format(
      promptControlKey(prompt, this.#getInputPreset()),
    );
  }

  localize(): void {
    if (this.#state) this.render(this.#state);
  }

  observeCommand(
    command: GameCommand,
    before: JourneyState | undefined,
    update: GameUpdate,
  ): void {
    for (const prompt of completedPromptsForUpdate(
      command,
      before,
      update,
      this.#currentPrompt,
    )) {
      this.#completed.add(prompt);
    }
    this.#persistCompleted();
    this.render(update);
  }

  recordInteraction(interaction: GuidanceInteraction): void {
    const prompt: Record<GuidanceInteraction, OnboardingPromptId> = {
      look: "look",
      inventory: "inventory",
      targeting: "combat",
      save: "save",
    };
    this.#completed.add(prompt[interaction]);
    this.#persistCompleted();
    if (this.#state) this.render(this.#state);
  }

  readonly #handleHideOptional = (): void => {
    this.#hideOptional = this.#dom.onboardingHideOptional.checked;
    this.#storage.setItem(HIDE_OPTIONAL_STORAGE_KEY, String(this.#hideOptional));
    if (this.#state) this.render(this.#state);
  };

  readonly #handleReset = (): void => {
    this.#completed.clear();
    this.#storage.removeItem(COMPLETED_STORAGE_KEY);
    if (this.#state) this.render(this.#state);
  };

  #persistCompleted(): void {
    this.#storage.setItem(COMPLETED_STORAGE_KEY, JSON.stringify([...this.#completed].sort()));
  }
}

function promptApplies(prompt: OnboardingPromptId, state: JourneyState): boolean {
  switch (prompt) {
    case "movement":
    case "look":
    case "stairs":
      return true;
    case "pickup":
      return state.items.length > 0;
    case "inventory":
      return state.inventory.length > 0;
    case "equipment":
      return (
        state.equipment.length > 0 ||
        state.inventory.some((item) => item.equipmentSlot !== null || item.usable)
      );
    case "combat":
      return state.entities.length > 0;
    case "resources":
      return (state.player.resources?.length ?? 0) > 0;
    case "messages":
    case "save":
      return state.turn > 0;
  }
}

function promptControlKey(prompt: OnboardingPrompt, preset: InputPreset): MessageKey {
  if (prompt.id !== "movement") return prompt.controlKey;
  return `onboarding-movement-control-${preset}`;
}

function objectiveDetailKey(objective: JourneyObjective): MessageKey {
  if (objective.id === "enter" && objective.returningFromOtherFloor) {
    return "journey-objective-enter-return-detail";
  }
  return `journey-objective-${objective.id}-detail`;
}

function formatJourneyLocation(localization: Localization, floorId: string): string {
  if (floorId === "demo.floor.surface") {
    return localization.format("journey-location-surface");
  }
  const depth = echoDepth(floorId);
  if (depth !== undefined) {
    return localization.format("journey-location-echo", {
      depth,
      route: localization.format(echoRouteKey(floorId)),
    });
  }
  return localization.format("journey-location-other", { floor: floorId });
}

function echoDepth(floorId: string): number | undefined {
  const match = /^demo\.floor\.echo-depth-(\d)/.exec(floorId);
  return match ? Number(match[1]) : undefined;
}

function echoRouteKey(floorId: string): MessageKey {
  if (floorId.endsWith("-mirror")) return "journey-route-mirror";
  if (floorId.endsWith("-branch")) return "journey-route-branch";
  if (floorId.endsWith("-shaft")) return "journey-route-shaft";
  return "journey-route-main";
}

function readCompletedPrompts(storage: GuidanceStorage): Set<OnboardingPromptId> {
  try {
    const raw = storage.getItem(COMPLETED_STORAGE_KEY);
    if (!raw) return new Set();
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return new Set();
    return new Set(
      parsed.filter(
        (value): value is OnboardingPromptId =>
          typeof value === "string" && PROMPT_IDS.has(value as OnboardingPromptId),
      ),
    );
  } catch {
    return new Set();
  }
}

function successfulEquipmentOrUseEvent(messageKey: string): boolean {
  if (messageKey === "item-equip-success" || messageKey === "item-equip-swap") return true;
  return (
    messageKey.startsWith("item-use-") &&
    !messageKey.includes("unavailable") &&
    !messageKey.includes("failed")
  );
}

function combatOrTargetingEvent(kind: string, messageKey: string): boolean {
  return (
    kind.startsWith("combat.") ||
    (messageKey.startsWith("projectile-") && !messageKey.includes("unavailable")) ||
    messageKey === "ability-cast-success" ||
    messageKey === "ability-cast-failure" ||
    messageKey === "item-thrown" ||
    messageKey === "throw-hit" ||
    messageKey === "throw-miss" ||
    messageKey === "throw-slay"
  );
}

function samePosition(
  left: { readonly x: number; readonly y: number },
  right: { readonly x: number; readonly y: number },
): boolean {
  return left.x === right.x && left.y === right.y;
}
