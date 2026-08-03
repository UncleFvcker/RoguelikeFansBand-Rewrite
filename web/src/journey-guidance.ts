// SPDX-License-Identifier: MPL-2.0

import type { AppDom } from "./app-dom";
import type { InputPreset } from "./input-controller";
import type { Localization, MessageKey } from "./localization";
import type { GameCommand, GameSnapshot, GameUpdate } from "./protocol";

type JourneyState = GameSnapshot | GameUpdate;

export interface JourneyDungeonStatus {
  readonly dungeonNameKey: MessageKey;
  readonly currentDepth?: number;
  readonly maximumDepth?: number;
  readonly bossNameKey?: MessageKey;
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
  | "journeyDungeonName"
  | "journeyDepth"
  | "journeyBoss"
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

export function selectJourneyDungeonStatus(
  state: JourneyState,
  knownWorldId?: string,
): JourneyDungeonStatus {
  const worldId = "worldId" in state ? state.worldId : knownWorldId;
  if (worldId !== "demo.world.warrens-journey") {
    return { dungeonNameKey: "journey-dungeon-none" };
  }
  if (state.floorId === "demo.floor.surface") {
    return {
      dungeonNameKey: "floor-demo-surface-name",
      bossNameKey:
        state.campaign.status === "active"
          ? "actor-demo-warrens-keeper-name"
          : undefined,
    };
  }
  return {
    dungeonNameKey: "dungeon-demo-warrens-name",
    currentDepth: warrensDepth(state.floorId) ?? 0,
    maximumDepth: 9,
    bossNameKey:
      state.campaign.status === "active"
        ? "actor-demo-warrens-keeper-name"
        : undefined,
  };
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
  #worldId: string | undefined;
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
    if ("worldId" in state) this.#worldId = state.worldId;
    const dungeon = selectJourneyDungeonStatus(state, this.#worldId);
    this.#dom.journeyPanel.dataset.dungeon = dungeon.dungeonNameKey;
    this.#dom.journeyDungeonName.textContent = this.#localization.format(
      dungeon.dungeonNameKey,
    );
    this.#dom.journeyDepth.hidden =
      dungeon.currentDepth === undefined || dungeon.maximumDepth === undefined;
    this.#dom.journeyDepth.textContent = this.#dom.journeyDepth.hidden
      ? ""
      : this.#localization.format("journey-dungeon-depth", {
          current: dungeon.currentDepth ?? 0,
          maximum: dungeon.maximumDepth ?? 0,
        });
    this.#dom.journeyBoss.hidden = dungeon.bossNameKey === undefined;
    this.#dom.journeyBoss.textContent = dungeon.bossNameKey
      ? this.#localization.format("journey-dungeon-boss", {
          boss: this.#localization.format(dungeon.bossNameKey),
        })
      : "";

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
    const surfaceEntrance =
      prompt.id === "stairs" &&
      this.#worldId === "demo.world.warrens-journey" &&
      state.floorId === "demo.floor.surface";
    this.#dom.onboardingTitle.textContent = this.#localization.format(
      surfaceEntrance ? "onboarding-warrens-entrance-title" : prompt.titleKey,
    );
    this.#dom.onboardingDetail.textContent = this.#localization.format(
      surfaceEntrance ? "onboarding-warrens-entrance-detail" : prompt.detailKey,
    );
    this.#dom.onboardingControl.textContent = this.#localization.format(
      surfaceEntrance
        ? "onboarding-warrens-entrance-control"
        : promptControlKey(prompt, this.#getInputPreset()),
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

function warrensDepth(floorId: string): number | undefined {
  const match = /^demo\.floor\.warrens-depth-(\d+)$/.exec(floorId);
  return match ? Number(match[1]) : undefined;
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
