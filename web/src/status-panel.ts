// SPDX-License-Identifier: MPL-2.0

import type { AppDom } from "./app-dom";
import type { AppState } from "./app-state";
import type { Localization, MessageKey } from "./localization";
import type {
  AbilityDto,
  AbilityLearningDto,
  AbilityStudyModeDto,
  AttributeKindDto,
  GameCommand,
  GameSnapshot,
  GameUpdate,
  PlayerBuildDto,
  PlayerMutationDto,
  PlayerProgressDto,
  ResourcePoolDto,
  SummonCommandDto,
  SummonCommandModeDto,
} from "./protocol";
import { REST_UNTIL_RECOVERED_TURNS } from "./rest.ts";
import { goldVisualId } from "./render-world.ts";
import { equippedLightText } from "./shop-panel.ts";

type StatusDom = Pick<
  AppDom,
  | "mapHost"
  | "turnValue"
  | "hpValue"
  | "healthMeter"
  | "healthMeterFill"
  | "goldValue"
  | "nutritionValue"
  | "lightValue"
  | "attackValue"
  | "defenseValue"
  | "effectsValue"
  | "positionValue"
  | "hashValue"
  | "progressionNameValue"
  | "progressionLevelValue"
  | "progressionExperienceValue"
  | "progressionCapValue"
  | "progressionPointsValue"
  | "progressionBuildValue"
  | "progressionRaceValue"
  | "progressionClassValue"
  | "progressionPersonalityValue"
  | "progressionMultipliersValue"
  | "attributeList"
  | "skillList"
  | "mutationList"
  | "resourceList"
  | "abilityList"
  | "resourceRest"
  | "nearbyCurrent"
  | "nearbyList"
  | "summonCommandStatus"
  | "summonCommandButtons"
  | "taskLogList"
  | "campaignStatusValue"
  | "campaignScoreValue"
  | "campaignDungeonsValue"
  | "campaignTasksValue"
  | "campaignRetire"
>;

const ATTRIBUTE_KINDS: AttributeKindDto[] = [
  "strength",
  "intelligence",
  "wisdom",
  "dexterity",
  "constitution",
  "charisma",
];

const WILDERNESS_DAY_TICKS = 100_000;

export type WildernessClock = {
  day: number;
  hour: number;
  minute: number;
  daytime: boolean;
};

export function wildernessClock(worldTick: number): WildernessClock {
  const withinDay = worldTick % WILDERNESS_DAY_TICKS;
  const clockTick = (withinDay + WILDERNESS_DAY_TICKS / 4) % WILDERNESS_DAY_TICKS;
  const minuteOfDay = Math.floor((clockTick * 24 * 60) / WILDERNESS_DAY_TICKS);
  return {
    day: Math.floor((worldTick + WILDERNESS_DAY_TICKS / 4) / WILDERNESS_DAY_TICKS) + 1,
    hour: Math.floor(minuteOfDay / 60),
    minute: minuteOfDay % 60,
    daytime: withinDay < WILDERNESS_DAY_TICKS / 2,
  };
}

export class StatusPanel {
  readonly #dom: StatusDom;
  readonly #state: AppState;
  readonly #localization: Localization;
  readonly #dispatch: (command: GameCommand) => Promise<void>;
  readonly #contentName: (id: string | undefined) => string;
  readonly #statusName: (id: string | undefined) => string;
  readonly #selectItemTarget: (
    excludedItemId: string | undefined,
    onSelect: (itemId: string) => Promise<void>,
  ) => void;
  readonly #startAbilityTargeting: (ability: AbilityDto) => void;
  readonly #reconcileTargeting: (state: GameSnapshot | GameUpdate) => void;
  readonly #renderTargeting: () => void;
  readonly #refreshInventoryActions: () => void;
  #installed = false;

  constructor(options: {
    dom: StatusDom;
    state: AppState;
    localization: Localization;
    dispatch: (command: GameCommand) => Promise<void>;
    contentName: (id: string | undefined) => string;
    statusName: (id: string | undefined) => string;
    selectItemTarget: (
      excludedItemId: string | undefined,
      onSelect: (itemId: string) => Promise<void>,
    ) => void;
    startAbilityTargeting: (ability: AbilityDto) => void;
    reconcileTargeting: (state: GameSnapshot | GameUpdate) => void;
    renderTargeting: () => void;
    refreshInventoryActions: () => void;
  }) {
    this.#dom = options.dom;
    this.#state = options.state;
    this.#localization = options.localization;
    this.#dispatch = options.dispatch;
    this.#contentName = options.contentName;
    this.#statusName = options.statusName;
    this.#selectItemTarget = options.selectItemTarget;
    this.#startAbilityTargeting = options.startAbilityTargeting;
    this.#reconcileTargeting = options.reconcileTargeting;
    this.#renderTargeting = options.renderTargeting;
    this.#refreshInventoryActions = options.refreshInventoryActions;
  }

  install(): void {
    if (this.#installed) return;
    this.#installed = true;
    this.#dom.campaignRetire.addEventListener("click", this.#handleRetire);
    this.#dom.resourceRest.addEventListener("click", this.#handleRest);
    for (const [mode, button] of Object.entries(this.#dom.summonCommandButtons) as [
      SummonCommandModeDto,
      HTMLButtonElement,
    ][]) {
      button.addEventListener("click", this.#summonCommandHandlers[mode]);
    }
  }

  dispose(): void {
    if (!this.#installed) return;
    this.#installed = false;
    this.#dom.campaignRetire.removeEventListener("click", this.#handleRetire);
    this.#dom.resourceRest.removeEventListener("click", this.#handleRest);
    for (const [mode, button] of Object.entries(this.#dom.summonCommandButtons) as [
      SummonCommandModeDto,
      HTMLButtonElement,
    ][]) {
      button.removeEventListener("click", this.#summonCommandHandlers[mode]);
    }
  }

  render(state: GameSnapshot | GameUpdate): void {
    this.#state.status = state;
    this.#state.playerDead = state.player.isDead;
    this.#state.campaignEnded = state.campaign.status === "retired";
    this.#reconcileTargeting(state);
    this.#dom.mapHost.ownerDocument.documentElement.dataset.playerState = this.#state.playerDead
      ? "dead"
      : "alive";
    const clock = wildernessClock(state.worldTick);
    this.#dom.turnValue.textContent = this.#localization.format("status-turn-time", {
      turn: state.turn,
      day: clock.day,
      hour: String(clock.hour).padStart(2, "0"),
      minute: String(clock.minute).padStart(2, "0"),
      phase: this.#localization.format(
        clock.daytime ? "wilderness-daytime" : "wilderness-nighttime",
      ),
    });
    this.#dom.hpValue.textContent = this.#localization.format(
      state.player.equipmentModifiers.maxHp > 0
        ? "status-health-value-bonus"
        : "status-health-value",
      {
        hp: state.player.hp,
        maxHp: state.player.maxHp,
        bonus: state.player.equipmentModifiers.maxHp,
      },
    );
    const healthRatio =
      state.player.maxHp > 0 ? Math.max(0, Math.min(1, state.player.hp / state.player.maxHp)) : 0;
    this.#dom.healthMeterFill.style.width = `${healthRatio * 100}%`;
    this.#dom.healthMeter.dataset.healthState =
      healthRatio <= 0.25 ? "critical" : healthRatio <= 0.5 ? "wounded" : "healthy";
    this.#dom.healthMeter.setAttribute("role", "progressbar");
    this.#dom.healthMeter.setAttribute("aria-valuemin", "0");
    this.#dom.healthMeter.setAttribute("aria-valuemax", String(state.player.maxHp));
    this.#dom.healthMeter.setAttribute("aria-valuenow", String(state.player.hp));
    this.#dom.goldValue.textContent = state.player.gold.toLocaleString(this.#localization.locale);
    this.#dom.nutritionValue.textContent = this.#localization.format("status-nutrition-value", {
      state: this.#localization.format(`nutrition-state-${state.player.nutritionState}`),
      percent: nutritionPercentage(state.player.nutrition),
    });
    this.#dom.lightValue.textContent = equippedLightText(
      state.equipment,
      this.#localization,
      this.#contentName,
    );
    this.#renderCombatStat(
      this.#dom.attackValue,
      state.player.attack,
      state.player.equipmentModifiers.attack,
    );
    this.#renderCombatStat(
      this.#dom.defenseValue,
      state.player.defense,
      state.player.equipmentModifiers.defense,
    );
    this.#dom.progressionNameValue.textContent = state.player.name;
    this.#renderProgression(state.player.progress, state.player.build);
    this.#renderMutations(state.player.mutations ?? []);
    this.#renderAbilities(
      state.player.abilities ?? [],
      state.player.resources ?? [],
      state.player.abilityLearning,
      state.player.progress?.level ?? 1,
    );
    this.#renderSummonCommand(state.player.summonCommand, state.entities);
    this.#renderNearby(state);
    const activeEffects = state.player.statuses.map((status) =>
      this.#localization.format("status-effect-entry", {
        status: this.#statusName(status.kindId),
        intensity: status.intensity,
        ticks: status.remainingTicks,
      }),
    );
    if (state.player.confusingStrikeReady) {
      activeEffects.push(this.#localization.format("status-effect-confusing-strike-ready"));
    }
    this.#dom.effectsValue.textContent =
      activeEffects.length === 0
        ? this.#localization.format("status-effects-none")
        : activeEffects.join(" \u00b7 ");
    this.#renderTasks(state);
    this.#dom.campaignStatusValue.textContent = this.#localization.format(
      `campaign-status-${state.campaign.status}` as MessageKey,
    );
    this.#dom.campaignScoreValue.textContent = String(state.campaign.score);
    this.#dom.campaignDungeonsValue.textContent = String(state.campaign.conqueredDungeons);
    this.#dom.campaignTasksValue.textContent = String(state.campaign.completedTasks);
    this.updateCampaignAction();
    this.#dom.positionValue.textContent = `${state.player.position.x}, ${state.player.position.y}`;
    this.#dom.hashValue.textContent = state.stateHash.slice(0, 12);
    this.#dom.hashValue.title = state.stateHash;
    this.#dom.mapHost.dataset.itemCount = String(state.items.length);
    this.#dom.mapHost.dataset.goldPileCount = String(state.goldPiles.length);
    this.#dom.mapHost.dataset.playerGold = String(state.player.gold);
    this.#dom.mapHost.dataset.playerNutrition = String(state.player.nutrition);
    this.#dom.mapHost.dataset.inventoryStackCount = String(state.inventory.length);
    this.#dom.mapHost.dataset.equipmentCount = String(state.equipment.length);
    this.#dom.mapHost.dataset.carriedWeightTenthsPound = String(
      state.player.carriedWeightTenthsPound,
    );
    this.#dom.mapHost.dataset.carryCapacityTenthsPound = String(
      state.player.carryCapacityTenthsPound,
    );
    this.#dom.mapHost.dataset.playerStatusCount = String(state.player.statuses.length);
    this.#refreshInventoryActions();
    this.#renderTargeting();
  }

  updateCampaignAction(): void {
    const state = this.#state.status;
    this.#dom.campaignRetire.disabled =
      this.#state.busy ||
      this.#state.playerDead ||
      this.#state.worldMap ||
      !state ||
      state.campaign.status !== "victorious" ||
      state.floorId !== "demo.floor.surface" ||
      state.dungeonInstanceId != null;
  }

  readonly #handleRetire = (): void => {
    void this.#dispatch({ type: "retire" });
  };

  readonly #handleRest = (): void => {
    void this.#dispatch({ type: "rest", turns: REST_UNTIL_RECOVERED_TURNS });
  };

  readonly #summonCommandHandlers: Record<SummonCommandModeDto, () => void> = {
    follow: () => void this.#dispatch({ type: "set-summon-command", mode: "follow" }),
    attack: () => void this.#dispatch({ type: "set-summon-command", mode: "attack" }),
    "keep-distance": () =>
      void this.#dispatch({ type: "set-summon-command", mode: "keep-distance" }),
    guard: () => void this.#dispatch({ type: "set-summon-command", mode: "guard" }),
  };

  #renderTasks(state: GameSnapshot | GameUpdate): void {
    const document = this.#dom.taskLogList.ownerDocument;
    this.#dom.taskLogList.replaceChildren(
      ...state.tasks.map((task) => {
        const row = document.createElement("li");
        row.textContent = this.#localization.format("task-log-entry", {
          task: this.#localization.format(task.nameKey),
          status: this.#localization.format(`task-status-${task.status}` as MessageKey),
          stage: task.stage,
          stages: task.stages,
          current: task.current,
          required: task.required,
        });
        const maxRetakes = task.maxRetakes;
        if (maxRetakes !== undefined && maxRetakes !== null) {
          row.append(
            " ",
            this.#localization.format("task-log-retakes", {
              used: task.retakesUsed,
              maximum: maxRetakes,
            }),
          );
        }
        if (task.status === "active" || task.status === "paused") {
          const abandon = document.createElement("button");
          abandon.type = "button";
          abandon.textContent = this.#localization.format("action-task-abandon");
          abandon.disabled = this.#state.busy || this.#state.worldMap;
          abandon.addEventListener("click", () =>
            void this.#dispatch(
              task.status === "active"
                ? { type: "abandon-task" }
                : { type: "abandon-paused-task", taskId: task.taskId },
            ),
          );
          row.append(" ", abandon);
        }
        return row;
      }),
    );
  }

  #renderProgression(
    progress: PlayerProgressDto | undefined,
    build: PlayerBuildDto | null | undefined,
  ): void {
    if (!progress) {
      const unavailable = this.#localization.format("progression-unavailable");
      this.#dom.progressionLevelValue.textContent = unavailable;
      this.#dom.progressionExperienceValue.textContent = unavailable;
      this.#dom.progressionCapValue.textContent = unavailable;
      this.#dom.progressionPointsValue.textContent = unavailable;
      this.#dom.progressionBuildValue.textContent = unavailable;
      this.#dom.progressionRaceValue.textContent = unavailable;
      this.#dom.progressionClassValue.textContent = unavailable;
      this.#dom.progressionPersonalityValue.textContent = unavailable;
      this.#dom.progressionMultipliersValue.textContent = unavailable;
      this.#dom.attributeList.replaceChildren();
      this.#dom.skillList.replaceChildren();
      return;
    }
    this.#dom.progressionLevelValue.textContent = this.#localization.format(
      "progression-level-value",
      { level: progress.level, maxLevel: progress.maxLevel },
    );
    this.#dom.progressionExperienceValue.textContent = this.#localization.format(
      "progression-experience-value",
      {
        experience: String(progress.experience),
        next:
          progress.experienceForNextLevel === undefined ||
          progress.experienceForNextLevel === null
            ? "\u2014"
            : String(progress.experienceForNextLevel),
      },
    );
    this.#dom.progressionCapValue.textContent = this.#localization.format(
      "progression-cap-value",
      {
        levelCap: progress.levelCap,
        attributeCap: formatAttributeValue(progress.attributeCap),
        attributeIndexCap: progress.attributeIndexCap,
      },
    );
    this.#dom.progressionPointsValue.textContent = String(progress.pendingAttributeIncreases);
    this.#dom.progressionBuildValue.textContent = build
      ? this.#localization.format(build.buildNameKey as MessageKey)
      : this.#localization.format("progression-unavailable");
    this.#dom.progressionRaceValue.textContent = build
      ? this.#localization.format(build.raceNameKey as MessageKey)
      : this.#localization.format("progression-unavailable");
    this.#dom.progressionClassValue.textContent = build
      ? this.#localization.format(build.classNameKey as MessageKey)
      : this.#localization.format("progression-unavailable");
    this.#dom.progressionPersonalityValue.textContent = build
      ? this.#localization.format(build.personalityNameKey as MessageKey)
      : this.#localization.format("progression-unavailable");
    this.#dom.progressionMultipliersValue.textContent = build
      ? this.#localization.format("progression-multipliers-value", {
          life: build.lifePercent,
          experience: build.experiencePercent,
        })
      : this.#localization.format("progression-unavailable");
    const document = this.#dom.attributeList.ownerDocument;
    this.#dom.attributeList.replaceChildren(
      ...ATTRIBUTE_KINDS.map((attribute) => {
        const value = progress.attributes[attribute];
        const row = document.createElement("li");
        row.className = "attribute-row";
        const label = document.createElement("span");
        label.className = "attribute-name";
        label.textContent = this.#localization.format(`attribute-${attribute}` as MessageKey);
        const values = document.createElement("span");
        values.className = "attribute-value";
        values.textContent = this.#localization.format("attribute-value", {
          natural: formatAttributeValue(value.natural),
          maximumNatural: formatAttributeValue(value.maximumNatural),
          potential: formatAttributeValue(value.potential),
          effective: formatAttributeValue(value.effective),
          index: value.index,
        });
        const increase = document.createElement("button");
        increase.type = "button";
        increase.className = "attribute-increase";
        increase.textContent = this.#localization.format("action-increase-attribute");
        increase.disabled =
          this.#state.busy ||
          this.#state.playerDead ||
          this.#state.worldMap ||
          progress.pendingAttributeIncreases === 0 ||
          value.maximumNatural >= Math.min(progress.attributeCap, value.potential);
        increase.addEventListener("click", () =>
          void this.#dispatch({ type: "increase-attribute", attribute }),
        );
        row.append(label, values, increase);
        return row;
      }),
    );
    this.#dom.skillList.replaceChildren(
      ...progress.skills.map((skill) => {
        const row = document.createElement("li");
        row.className = "skill-row";
        const name = document.createElement("span");
        name.className = "skill-name";
        name.textContent = this.#localization.format(skill.nameKey as MessageKey);
        const value = document.createElement("span");
        value.className = "skill-value";
        value.textContent = this.#localization.format("skill-value", {
          current: skill.current,
          maximum: skill.maximum,
          growth: skill.growthPerTenLevels,
        });
        row.append(name, value);
        return row;
      }),
    );
  }

  #renderSummonCommand(
    command: SummonCommandDto | undefined,
    entities: GameSnapshot["entities"],
  ): void {
    const mode = command?.mode ?? "follow";
    const count = entities.filter(
      (entity) => entity.faction === "player" && entity.summon != null,
    ).length;
    this.#dom.summonCommandStatus.textContent = this.#localization.format(
      "summon-command-status",
      {
        mode: this.#localization.format(`summon-command-mode-${mode}` as MessageKey),
        count,
      },
    );
    for (const [buttonMode, button] of Object.entries(this.#dom.summonCommandButtons) as [
      SummonCommandModeDto,
      HTMLButtonElement,
    ][]) {
      const selected = buttonMode === mode;
      button.disabled =
        this.#state.busy || this.#state.commandBlocked || this.#state.worldMap || selected;
      button.setAttribute("aria-pressed", String(selected));
    }
  }

  #renderMutations(mutations: PlayerMutationDto[]): void {
    const document = this.#dom.mutationList.ownerDocument;
    if (mutations.length === 0) {
      const empty = document.createElement("li");
      empty.className = "mutation-empty";
      empty.textContent = this.#localization.format("mutation-empty");
      this.#dom.mutationList.replaceChildren(empty);
      return;
    }
    this.#dom.mutationList.replaceChildren(
      ...mutations.map((mutation) => {
        const row = document.createElement("li");
        row.className = "mutation-row";
        row.dataset.rating = mutation.rating;
        const heading = document.createElement("div");
        heading.className = "mutation-heading";
        const name = document.createElement("strong");
        name.className = "mutation-name";
        name.textContent = mutation.name;
        const badges = document.createElement("span");
        badges.className = "mutation-badges";
        const rating = document.createElement("span");
        rating.className = "mutation-rating";
        rating.textContent = this.#localization.format(mutationRatingMessageKey(mutation.rating));
        badges.append(rating);
        if (mutation.locked) {
          const locked = document.createElement("span");
          locked.className = "mutation-locked";
          locked.textContent = this.#localization.format("mutation-locked");
          badges.append(locked);
        }
        const description = document.createElement("p");
        description.className = "mutation-description";
        description.textContent = mutation.description;
        heading.append(name, badges);
        row.append(heading, description);
        return row;
      }),
    );
  }

  #renderAbilities(
    abilities: AbilityDto[],
    resources: ResourcePoolDto[],
    learning: AbilityLearningDto | null | undefined,
    playerLevel: number,
  ): void {
    const document = this.#dom.abilityList.ownerDocument;
    this.#dom.resourceList.replaceChildren();
    this.#dom.abilityList.replaceChildren();
    this.#dom.resourceRest.disabled =
      this.#state.busy ||
      this.#state.playerDead ||
      this.#state.worldMap ||
      !this.#state.status ||
      (this.#state.status.player.hp >= this.#state.status.player.maxHp &&
        !resources.some(
          (resource) => resource.restRecoveryAmount > 0 && resource.current < resource.maximum,
        ));
    if (resources.length === 0) {
      const unavailable = document.createElement("li");
      unavailable.className = "resource-empty";
      unavailable.textContent = this.#localization.format("resource-unavailable");
      this.#dom.resourceList.append(unavailable);
    }
    if (resources.length === 0 && abilities.length === 0) {
      const unavailable = document.createElement("li");
      unavailable.className = "ability-empty";
      unavailable.textContent = this.#localization.format("ability-unavailable");
      this.#dom.abilityList.append(unavailable);
      return;
    }
    for (const resource of resources) {
      const row = document.createElement("li");
      row.className = "resource-row";
      const name = this.#localization.format(resource.nameKey as MessageKey);
      const heading = document.createElement("div");
      heading.className = "resource-heading";
      const label = document.createElement("span");
      label.className = "resource-name";
      label.textContent = name;
      const value = document.createElement("strong");
      value.className = "resource-value";
      value.textContent = `${resource.current} / ${resource.maximum}`;
      heading.append(label, value);
      const meter = document.createElement("span");
      meter.className = "resource-meter";
      meter.setAttribute("role", "progressbar");
      meter.setAttribute("aria-label", name);
      meter.setAttribute("aria-valuemin", "0");
      meter.setAttribute("aria-valuemax", String(resource.maximum));
      meter.setAttribute("aria-valuenow", String(resource.current));
      const fill = document.createElement("span");
      fill.style.width = `${resource.maximum > 0 ? Math.max(0, Math.min(100, resource.current / resource.maximum * 100)) : 0}%`;
      meter.append(fill);
      const recovery = document.createElement("span");
      recovery.className = "resource-recovery";
      recovery.textContent = this.#localization.format("ability-resource-value", {
        resource: name,
        current: resource.current,
        maximum: resource.maximum,
        wait: resource.waitRecoveryAmount,
        rest: resource.restRecoveryAmount,
      });
      row.append(heading, meter, recovery);
      this.#dom.resourceList.append(row);
    }
    if (learning) {
      const row = document.createElement("li");
      row.className = "resource-row";
      row.textContent = this.#localization.format("ability-learning-value", {
        learned: learning.learnedCount,
        capacity: learning.capacity,
        remaining: learning.remainingSlots,
      });
      this.#dom.resourceList.append(row);
    }
    const studyMode = learning?.studyMode ?? "chosen";
    for (const entry of abilityPresentation(abilities, playerLevel)) {
      if (entry.type === "heading") {
        const heading = document.createElement("li");
        heading.className = "ability-book-heading";
        const label = document.createElement("span");
        label.textContent = this.#localization.format(entry.nameKey as MessageKey);
        heading.append(label);
        const bookItemId = entry.bookItemId;
        if (studyMode === "divine-random" && bookItemId) {
          const study = this.#abilityAction("action-ability-study-prayer", () =>
            void this.#dispatch({ type: "study-prayer", bookItemId }),
          );
          study.disabled =
            this.#state.busy ||
            this.#state.playerDead ||
            this.#state.worldMap ||
            !entry.canStudy;
          heading.append(study);
        }
        this.#dom.abilityList.append(heading);
      } else {
        this.#dom.abilityList.append(this.#abilityRow(entry.ability, studyMode));
      }
    }
  }

  #renderNearby(state: GameSnapshot | GameUpdate): void {
    const document = this.#dom.nearbyList.ownerDocument;
    const player = state.player.position;
    const currentCell = this.#state.cellAt(player);
    this.#dom.nearbyCurrent.textContent = currentCell
      ? this.#localization.format("nearby-current-terrain", {
          terrain: this.#contentName(currentCell.terrainId),
        })
      : this.#localization.format("nearby-current-unknown");

    const entries: NearbyEntry[] = [];
    for (const entity of state.entities) {
      if (!this.#isVisible(entity.position)) continue;
      const distance = chebyshevDistance(player, entity.position);
      if (distance === 0) continue;
      entries.push({
        id: `actor:${entity.id}`,
        kind: entity.faction === "hostile" ? "hostile" : "ally",
        contentId: entity.kindId,
        glyph: entity.glyph,
        name: this.#contentName(entity.kindId),
        distance,
        direction: directionKey(player, entity.position),
        hp: entity.hp,
        maxHp: entity.maxHp,
      });
    }
    for (const item of state.items) {
      if (!this.#isVisible(item.position)) continue;
      const distance = chebyshevDistance(player, item.position);
      entries.push({
        id: `item:${item.id}`,
        kind: "item",
        contentId: item.kindId,
        name: this.#contentName(item.kindId),
        distance,
        direction: distance === 0 ? "here" : directionKey(player, item.position),
        quantity: item.quantity,
      });
    }
    for (const pile of state.goldPiles) {
      if (!this.#isVisible(pile.position)) continue;
      const distance = chebyshevDistance(player, pile.position);
      entries.push({
        id: `gold:${pile.id}`,
        kind: "gold",
        contentId: goldVisualId(pile.appearance),
        name: this.#localization.format(`gold-appearance-${pile.appearance}`),
        distance,
        direction: distance === 0 ? "here" : directionKey(player, pile.position),
        amount: pile.amount,
      });
    }
    entries.sort((left, right) =>
      left.distance - right.distance ||
      nearbyKindPriority(left.kind) - nearbyKindPriority(right.kind) ||
      left.name.localeCompare(right.name) ||
      left.id.localeCompare(right.id),
    );
    if (entries.length === 0) {
      const empty = document.createElement("li");
      empty.className = "nearby-empty";
      empty.textContent = this.#localization.format("nearby-empty");
      this.#dom.nearbyList.replaceChildren(empty);
      return;
    }
    this.#dom.nearbyList.replaceChildren(
      ...entries.slice(0, 8).map((entry) => {
        const row = document.createElement("li");
        row.className = `nearby-row nearby-${entry.kind}`;
        const glyph = document.createElement("span");
        glyph.className = "nearby-glyph";
        glyph.setAttribute("aria-hidden", "true");
        glyph.textContent = entry.glyph ?? this.#state.contentGlyphs.get(entry.contentId) ?? "?";
        const details = document.createElement("span");
        details.className = "nearby-details";
        const name = document.createElement("strong");
        name.textContent = entry.name;
        const meta = document.createElement("span");
        const direction = this.#localization.format(`nearby-direction-${entry.direction}`);
        meta.textContent =
          entry.kind === "item"
            ? this.#localization.format("nearby-item-meta", {
                direction,
                distance: entry.distance,
                quantity: entry.quantity ?? 1,
              })
            : entry.kind === "gold"
              ? this.#localization.format("nearby-gold-meta", {
                  direction,
                  distance: entry.distance,
                  amount: entry.amount ?? 0,
                })
            : this.#localization.format("nearby-actor-meta", {
                direction,
                distance: entry.distance,
                hp: entry.hp ?? 0,
                maxHp: entry.maxHp ?? 0,
              });
        details.append(name, meta);
        row.append(glyph, details);
        return row;
      }),
    );
  }

  #isVisible(position: { readonly x: number; readonly y: number }): boolean {
    return this.#state.cellVisibility.get(`${position.x},${position.y}`) === "visible";
  }

  #abilityRow(ability: AbilityDto, studyMode: AbilityStudyModeDto): HTMLLIElement {
    const document = this.#dom.abilityList.ownerDocument;
    const row = document.createElement("li");
    row.className = "ability-row";
    const details = document.createElement("div");
    details.className = "ability-details";
    const name = document.createElement("span");
    name.className = "ability-name";
    name.textContent = this.#localization.format(ability.nameKey as MessageKey);
    const description = document.createElement("span");
    description.className = "ability-description";
    description.textContent = this.#localization.format(ability.descriptionKey as MessageKey);
    const summary = document.createElement("span");
    summary.className = "ability-summary";
    summary.textContent = this.#localization.format("ability-summary", {
      level: ability.minimumLevel,
      baseCost: ability.baseResourceCost,
      cost: ability.resourceCost,
      failure: ability.failurePercent,
    });
    const proficiency = document.createElement("span");
    proficiency.className = "ability-summary";
    proficiency.textContent = this.#localization.format("ability-proficiency-summary", {
      rank: this.#localization.format(
        `ability-proficiency-${ability.proficiencyRank}` as MessageKey,
      ),
      current: ability.proficiency,
      maximum: ability.proficiencyCap,
      casts: ability.castCount,
      fails: ability.failCount,
    });
    const status = document.createElement("span");
    status.className = "ability-status";
    status.textContent = this.#localization.format(abilityStatusMessageKey(ability));
    details.append(name, description, summary, proficiency, status);
    this.#appendAbilityDetails(details, ability);
    const actions = document.createElement("div");
    actions.className = "ability-actions";
    const study = this.#abilityAction("action-ability-study", () => {
      if (!ability.bookItemId) return;
      void this.#dispatch({
        type: "study-ability",
        bookItemId: ability.bookItemId,
        abilityId: ability.id,
      });
    });
    study.disabled =
      this.#state.busy ||
      this.#state.playerDead ||
      this.#state.worldMap ||
      !ability.canStudy ||
      !ability.bookItemId;
    const forget = this.#abilityAction("action-ability-forget", () =>
      void this.#dispatch({ type: "forget-ability", abilityId: ability.id }),
    );
    forget.disabled =
      this.#state.busy || this.#state.playerDead || this.#state.worldMap || !ability.canForget;
    const cast = this.#abilityAction("action-ability-cast", () => this.#castAbility(ability));
    cast.classList.add("ability-cast-action");
    cast.disabled =
      this.#state.busy || this.#state.playerDead || this.#state.worldMap || !ability.canCast;
    if (studyMode === "chosen") actions.append(study);
    actions.append(forget, cast);
    row.append(details, actions);
    return row;
  }

  #appendAbilityDetails(details: HTMLDivElement, ability: AbilityDto): void {
    const append = (key: MessageKey, args?: Record<string, string | number>): void => {
      const element = details.ownerDocument.createElement("span");
      element.className = "ability-status";
      element.textContent = this.#localization.format(key, args);
      details.append(element);
    };
    if (ability.areaRadius != null) append("ability-area-summary", { radius: ability.areaRadius });
    if (ability.beamDamage) append("ability-beam-summary");
    if (ability.coneRadius != null) append("ability-cone-summary", { radius: ability.coneRadius });
    if (ability.teleport) append("ability-teleport-summary");
    if (ability.summon != null) {
      append("ability-summon-summary", {
        count: ability.summon.count,
        radius: ability.summon.radius,
        turns: ability.summon.durationTurns,
      });
    }
    if (ability.detect != null) {
      append("ability-detect-summary", {
        category: ability.detect.category,
        radius: ability.detect.radius,
        persistence: this.#localization.format(
          ability.detect.persistent
            ? "ability-detect-persistent"
            : "ability-detect-transient",
        ),
      });
    }
    if (ability.terrainTransform != null) {
      append("ability-terrain-transform-summary", {
        sources: ability.terrainTransform.sourceTerrainIds.length,
        terrain: this.#contentName(ability.terrainTransform.targetTerrainId),
        radius: ability.terrainTransform.radius,
      });
    }
    if (ability.effects.length > 1) {
      append("ability-effects-summary", { count: ability.effects.length });
    }
    if (ability.cooldownTurns > 0) {
      append("ability-cooldown-summary", {
        remaining: ability.cooldownRemaining,
        turns: ability.cooldownTurns,
      });
    }
  }

  #abilityAction(key: MessageKey, action: () => void): HTMLButtonElement {
    const button = this.#dom.abilityList.ownerDocument.createElement("button");
    button.type = "button";
    button.textContent = this.#localization.format(key);
    button.addEventListener("click", action);
    return button;
  }

  #castAbility(ability: AbilityDto): void {
    if (ability.targetSpec.modes.includes("self")) {
      void this.#dispatch({
        type: "cast-ability",
        abilityId: ability.id,
        target: { type: "self" },
      });
      return;
    }
    if (ability.targetSpec.modes.includes("item")) {
      this.#selectItemTarget(undefined, (itemId) =>
        this.#dispatch({
          type: "cast-ability",
          abilityId: ability.id,
          target: { type: "item", itemId },
        }),
      );
      return;
    }
    this.#startAbilityTargeting(ability);
  }

  #renderCombatStat(element: HTMLElement, value: number, equipmentModifier: number): void {
    element.textContent = this.#localization.format(
      equipmentModifier === 0 ? "status-stat-value" : "status-stat-value-bonus",
      { value, bonus: signedModifier(equipmentModifier) },
    );
  }
}

export function mutationRatingMessageKey(rating: PlayerMutationDto["rating"]): MessageKey {
  return `mutation-rating-${rating}`;
}

export function abilityStatusMessageKey(
  ability: Pick<AbilityDto, "source" | "learned">,
): MessageKey {
  if (ability.source === "mutation") return "ability-status-mutation";
  if (ability.source === "class") return "ability-status-class";
  return ability.learned ? "ability-status-learned" : "ability-status-unlearned";
}

export type AbilityPresentationEntry =
  | {
      type: "heading";
      nameKey: string;
      bookItemId?: string;
      canStudy: boolean;
    }
  | { type: "ability"; ability: AbilityDto };

export function abilityPresentation(
  abilities: readonly AbilityDto[],
  playerLevel: number,
): AbilityPresentationEntry[] {
  const ordered = [...abilities]
    .filter((ability) => !ability.uiGroupNameKey || ability.minimumLevel <= playerLevel)
    .sort(
      (left, right) =>
        (left.uiGroupNameKey ?? "").localeCompare(right.uiGroupNameKey ?? "") ||
        (left.bookRank ?? Number.MAX_SAFE_INTEGER) -
          (right.bookRank ?? Number.MAX_SAFE_INTEGER) ||
        (left.bookNameKey ?? "").localeCompare(right.bookNameKey ?? "") ||
        left.minimumLevel - right.minimumLevel ||
        left.id.localeCompare(right.id),
    );
  const entries: AbilityPresentationEntry[] = [];
  const studyByHeading = new Map<string, { bookItemId?: string; canStudy: boolean }>();
  for (const ability of ordered) {
    if (!ability.bookNameKey) continue;
    const current = studyByHeading.get(ability.bookNameKey);
    studyByHeading.set(ability.bookNameKey, {
      bookItemId: current?.bookItemId ?? ability.bookItemId ?? undefined,
      canStudy: (current?.canStudy ?? false) || ability.canStudy === true,
    });
  }
  let currentHeading: string | undefined;
  for (const ability of ordered) {
    const heading = ability.uiGroupNameKey ?? ability.bookNameKey ?? undefined;
    if (heading && heading !== currentHeading) {
      entries.push({
        type: "heading",
        nameKey: heading,
        bookItemId: studyByHeading.get(heading)?.bookItemId,
        canStudy: studyByHeading.get(heading)?.canStudy ?? false,
      });
    }
    currentHeading = heading;
    entries.push({ type: "ability", ability });
  }
  return entries;
}

export function formatAttributeValue(value: number): string {
  return value > 18 ? `18/${value - 18}` : String(value);
}

export function nutritionPercentage(nutrition: number): number {
  return Math.floor(nutrition / 100);
}

type NearbyKind = "hostile" | "ally" | "gold" | "item";
type NearbyDirection = "n" | "ne" | "e" | "se" | "s" | "sw" | "w" | "nw" | "here";

interface NearbyEntry {
  readonly id: string;
  readonly kind: NearbyKind;
  readonly contentId: string;
  readonly glyph?: string;
  readonly name: string;
  readonly distance: number;
  readonly direction: NearbyDirection;
  readonly hp?: number;
  readonly maxHp?: number;
  readonly quantity?: number;
  readonly amount?: number;
}

function chebyshevDistance(
  left: { readonly x: number; readonly y: number },
  right: { readonly x: number; readonly y: number },
): number {
  return Math.max(Math.abs(left.x - right.x), Math.abs(left.y - right.y));
}

function directionKey(
  from: { readonly x: number; readonly y: number },
  to: { readonly x: number; readonly y: number },
): NearbyDirection {
  const x = Math.sign(to.x - from.x);
  const y = Math.sign(to.y - from.y);
  if (x === 0 && y === 0) return "here";
  if (x === 0) return y < 0 ? "n" : "s";
  if (y === 0) return x < 0 ? "w" : "e";
  if (x > 0) return y < 0 ? "ne" : "se";
  return y < 0 ? "nw" : "sw";
}

function nearbyKindPriority(kind: NearbyKind): number {
  return kind === "hostile" ? 0 : kind === "ally" ? 1 : kind === "gold" ? 2 : 3;
}

function signedModifier(value: number): string {
  return value >= 0 ? `+${value}` : String(value);
}
