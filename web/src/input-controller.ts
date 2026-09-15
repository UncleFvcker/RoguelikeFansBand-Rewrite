// SPDX-License-Identifier: MPL-2.0

import { MapDisplay } from "./map-display.ts";
import { MAP_CELL_SIZE } from "./camera.ts";
import type { AppDom } from "./app-dom";
import type { AppState, TargetingIntent } from "./app-state";
import type { Localization, MessageKey } from "./localization";
import type {
  AbilityDto,
  AutoGetTargetDto,
  Direction,
  ItemDto,
  GameCommand,
  GameSnapshot,
  GameUpdate,
  Position,
  TargetSpecDto,
  TargetSelection,
} from "./protocol";
import {
  beginTargeting,
  cycleTarget,
  defaultTargetState,
  rememberedTargetState,
  moveTarget,
  targetSelectionAtCursor,
  translateTargetingState,
} from "./targeting.ts";
import {
  terrainInteractionCommand,
  terrainInteractionForDirection,
  terrainInteractionsForMode,
  isAutomaticallyRepeatedCommand,
  terrainInteractionModeForKey,
  terrainSearchCommandForKey,
  type TerrainInteractionMode,
} from "./terrain-interaction.ts";
import { REST_UNTIL_RECOVERED_TURNS, parseRestInput, type RestCommand } from "./rest.ts";
import type { DispatchResult } from "./game-session.ts";
import { commandShortcut, type CommandShortcut } from "./command-shortcuts.ts";

export type InputPreset = "original" | "roguelike";

type ContinuousAction = {
  kind: "local-travel" | "world-travel" | "auto-get" | "rest" | "fishing" | "run" | "auto-explore" | "repeat" | "macro";
  cancelled: boolean;
  done: Promise<void>;
  resume?: () => void;
};

type InputDom = Pick<
  AppDom,
  | "mapHost"
  | "targetCursor"
  | "traverseStairs"
  | "autoExplore"
  | "nearestUnknownItem"
  | "searchModeToggle"
  | "targetModeToggle"
  | "lookModeToggle"
  | "targetModeStatus"
  | "combatContext"
>;

export class InputController {
  readonly #mapDisplay = new MapDisplay();
  readonly #state: AppState;
  readonly #dom: InputDom;
  readonly #localization: Localization;
  readonly #window: Window;
  readonly #getInputPreset: () => InputPreset;
  readonly #getZoom: () => number;
  readonly #dispatch: (command: GameCommand, repeatCommand?: GameCommand | false) => Promise<DispatchResult>;
  readonly #getLastCommand: () => GameCommand | undefined;
  readonly #confirmRepeat: (command: GameCommand) => boolean;
  readonly #whenIdle: () => Promise<void>;
  readonly #onShortcut: (shortcut: CommandShortcut, count?: number) => void;
  readonly #customKey: (event: KeyboardEvent, execute: (key: KeyboardEventInit) => void) => boolean;
  readonly #hasCustomKey: (event: KeyboardEvent) => boolean;
  readonly #chooseChest: (command: "open-chest" | "disarm-chest", items: ItemDto[], count?: number) => void;
  readonly #onContinuousActionChange: () => void;
  readonly #describeLook: (position: { readonly x: number; readonly y: number }) => string;
  readonly #openObjectList: () => void;
  readonly #openMogaminator: () => void;
  readonly #openDeviceCommand: (key: string) => boolean;
  readonly #onLookFocusChange: (position: Position | undefined) => void;
  readonly #announce: (
    key: MessageKey,
    args: Record<string, string | number> | undefined,
    kind: string,
  ) => void;
  #installed = false;
  #fishingStopped = false;
  #sessionGeneration = 0;
  #literalCommand = false;
  #countInput: number | undefined;
  #commandCount: number | undefined;
  #runDirectionPreset: InputPreset | undefined;
  #walkDirection: { preset: InputPreset; special: boolean } | undefined;
  #glyphPromptPending = false;
  #ridingDirection = false;
  #worldTravelDestination: Position | undefined;
  #localTravelDestination: Position | undefined;
  #localTravelFloorId: string | undefined;
  #localTravelObjectId: string | undefined;
  #rememberedTarget: { target: TargetSelection; floorId: string } | undefined;
  #continuousAction: ContinuousAction | undefined;
  #heldMovement: { key: KeyboardEvent; direction: Direction } | undefined;

  constructor(options: {
    state: AppState;
    dom: InputDom;
    localization: Localization;
    window: Window;
    getInputPreset: () => InputPreset;
    getZoom: () => number;
    dispatch: (command: GameCommand, repeatCommand?: GameCommand | false) => Promise<DispatchResult>;
    getLastCommand?: () => GameCommand | undefined;
    confirmRepeat?: (command: GameCommand) => boolean;
    whenIdle: () => Promise<void>;
    onShortcut?: (shortcut: CommandShortcut, count?: number) => void;
    customKey?: (event: KeyboardEvent, execute: (key: KeyboardEventInit) => void) => boolean;
    hasCustomKey?: (event: KeyboardEvent) => boolean;
    chooseChest?: (command: "open-chest" | "disarm-chest", items: ItemDto[], count?: number) => void;
    onContinuousActionChange?: () => void;
    describeLook: (position: { readonly x: number; readonly y: number }) => string;
    openObjectList: () => void;
    openMogaminator: () => void;
    openDeviceCommand?: (key: string) => boolean;
    onLookFocusChange: (position: Position | undefined) => void;
    announce: (
      key: MessageKey,
      args: Record<string, string | number> | undefined,
      kind: string,
    ) => void;
  }) {
    this.#state = options.state;
    this.#dom = options.dom;
    this.#localization = options.localization;
    this.#window = options.window;
    this.#getInputPreset = options.getInputPreset;
    this.#getZoom = options.getZoom;
    this.#dispatch = options.dispatch;
    this.#getLastCommand = options.getLastCommand ?? (() => undefined);
    this.#confirmRepeat = options.confirmRepeat ?? (() => false);
    this.#whenIdle = options.whenIdle;
    this.#onShortcut = options.onShortcut ?? (() => {});
    this.#customKey = options.customKey ?? (() => false);
    this.#hasCustomKey = options.hasCustomKey ?? (() => false);
    this.#chooseChest = options.chooseChest ?? (() => {});
    this.#onContinuousActionChange = options.onContinuousActionChange ?? (() => {});
    this.#describeLook = options.describeLook;
    this.#openObjectList = options.openObjectList;
    this.#openMogaminator = options.openMogaminator;
    this.#openDeviceCommand = options.openDeviceCommand ?? (() => false);
    this.#onLookFocusChange = options.onLookFocusChange;
    this.#announce = options.announce;
  }

  install(): void {
    if (this.#installed) return;
    this.#installed = true;
    this.#window.addEventListener("keydown", this.#handleKeydown);
    this.#window.addEventListener("keyup", this.#handleKeyup);
    this.#window.addEventListener("keydown", this.#interruptContinuousKey, true);
    this.#window.addEventListener("click", this.#interruptContinuousClick, true);
    this.#window.addEventListener("blur", this.#stopOnBlur);
    this.#window.document.addEventListener("visibilitychange", this.#stopWhenHidden);
    this.#window.addEventListener("resize", this.#handleResize);
    this.#dom.mapHost.addEventListener("map-camera-change", this.#handleResize);
    this.#dom.traverseStairs.addEventListener("click", this.#handleTraverseStairs);
    this.#dom.searchModeToggle.addEventListener("click", this.#handleSearchModeToggle);
    this.#dom.autoExplore.addEventListener("click", this.#handleAutoExplore);
    this.#dom.nearestUnknownItem.addEventListener("click", this.#handleNearestUnknownItem);
    this.#dom.targetModeToggle.addEventListener("click", this.#handleTargetToggle);
    this.#dom.lookModeToggle.addEventListener("click", this.#handleLookToggle);
  }

  dispose(): void {
    if (!this.#installed) return;
    this.#installed = false;
    this.resetSession();
    this.#window.removeEventListener("keydown", this.#handleKeydown);
    this.#window.removeEventListener("keyup", this.#handleKeyup);
    this.#window.removeEventListener("keydown", this.#interruptContinuousKey, true);
    this.#window.removeEventListener("click", this.#interruptContinuousClick, true);
    this.#window.removeEventListener("blur", this.#stopOnBlur);
    this.#window.document.removeEventListener("visibilitychange", this.#stopWhenHidden);
    this.#window.removeEventListener("resize", this.#handleResize);
    this.#dom.mapHost.removeEventListener("map-camera-change", this.#handleResize);
    this.#dom.traverseStairs.removeEventListener("click", this.#handleTraverseStairs);
    this.#dom.searchModeToggle.removeEventListener("click", this.#handleSearchModeToggle);
    this.#dom.autoExplore.removeEventListener("click", this.#handleAutoExplore);
    this.#dom.nearestUnknownItem.removeEventListener("click", this.#handleNearestUnknownItem);
    this.#dom.targetModeToggle.removeEventListener("click", this.#handleTargetToggle);
    this.#dom.lookModeToggle.removeEventListener("click", this.#handleLookToggle);
  }

  startProjectileTargeting(): void {
    if (this.#state.busy || this.#state.commandBlocked || !this.#state.status) return;
    this.startTargetingWithSpec(
      this.#state.status.player.projectileProfile?.targetSpec,
      { type: "projectile" },
    );
  }

  startTargetSelection(): void {
    if (this.#state.worldMap) { this.startLookMode(); return; }
    const status = this.#state.status;
    if (!status || this.#state.busy || this.#state.commandBlocked) return;
    this.startTargetingWithSpec({ modes: ["entity", "position"], range: Math.max(status.width, status.height), requiresLineOfEffect: false }, { type: "select-target" });
    this.#rememberedTarget = undefined;
    if (this.#state.targeting) this.#state.targeting = cycleTarget(this.#state.targeting, status.entities, 0);
    this.render();
  }

  startLookMode(): void {
    const status = this.#state.status;
    if (this.#state.busy || this.#state.commandBlocked || !status) return;
    const next = beginTargeting(status.player.position, {
      modes: ["position"],
      range: Math.max(this.#state.mapWidth, this.#state.mapHeight),
      requiresLineOfEffect: false,
    });
    if (!next) return;
    this.#state.targeting = next;
    this.#state.targetingIntent = { type: "look" };
    this.#announce("message-look-mode-started", undefined, "system");
    this.#onLookFocusChange(next.cursor);
    this.render();
  }

  startLocalTravelSelection(): void {
    const status = this.#state.status;
    if (this.#state.busy || this.#state.commandBlocked || !status || this.#state.worldMap) return;
    const next = beginTargeting(status.player.position, {
      modes: ["position"],
      range: Math.max(this.#state.mapWidth, this.#state.mapHeight),
      requiresLineOfEffect: false,
    });
    if (!next) return;
    this.#state.targeting = next;
    this.#state.targetingIntent = { type: "local-travel" };
    this.#announce("message-local-travel-select", undefined, "system");
    this.#onLookFocusChange(next.cursor);
    this.render();
  }

  resetLocalTravel(): void {
    void this.stopContinuousAction();
    this.#localTravelDestination = undefined;
    this.#localTravelFloorId = undefined;
    this.#localTravelObjectId = undefined;
  }

  resetSession(): void {
    this.#heldMovement = undefined;
    this.#mapDisplay.reset();
    this.#rememberedTarget = undefined;
    this.#sessionGeneration++;
    this.#glyphPromptPending = false;
    const action = this.#continuousAction;
    if (action) { action.cancelled = true; action.resume?.(); }
    this.#continuousAction = undefined;
    this.#fishingStopped = true; // A loaded fishing action needs a new explicit player action.
    this.#ridingDirection = false;
    this.#runDirectionPreset = undefined;
    this.#walkDirection = undefined;
    this.#literalCommand = false;
    this.#countInput = undefined;
    this.#commandCount = undefined;
    this.#state.terrainInteractionMode = undefined;
    this.#worldTravelDestination = undefined;
    this.#localTravelDestination = undefined;
    this.#localTravelFloorId = undefined;
    this.#localTravelObjectId = undefined;
    this.cancelTargeting(false);
    this.#onContinuousActionChange();
  }

  async prepareSessionAccess(): Promise<void> {
    await this.stopContinuousAction();
    if (this.#state.status?.player.running) {
      if (await this.#dispatch({ type: "cancel-run" }) !== "applied") {
        throw new Error(this.#localization.format("message-run-cancel-required"));
      }
    }
    if (this.#state.status?.player.autoExplore) {
      if (await this.#dispatch({ type: "cancel-auto-explore" }) !== "applied") {
        throw new Error(this.#localization.format("message-auto-explore-cancel-required"));
      }
    }
    if (this.#state.status?.player.fishingDirection) {
      throw new Error(this.#localization.format("message-fishing-cancel-required"));
    }
  }

  async autoGet(): Promise<void> {
    const initial = this.#state.status;
    if (!initial || initial.mapScale !== "local" || this.#state.commandBlocked) return;
    await this.#runContinuousAction("auto-get", async action => {
      if (!await this.#continuousStep(action, { type: "pick-up" })) return;
      let current = this.#state.status;
      if (autoGetInterrupted(initial, current)) return;

      for (;;) {
        const target = current?.mogaminator.autoGetTarget;
        if (!current || !target) return;
        for (;;) {
          const before = current;
          if (!await this.#continuousStep(action, { type: "auto-get", objectId: target.objectId })) return;
          current = this.#state.status;
          if (autoGetStopsAfterStep(before, current, target)) return;
          if (!current || !autoGetObjectExists(current, target.objectId)) break;
        }
      }
    });
  }

  async restUntilRecovered(count?: number): Promise<void> {
    await this.#rest({ type: count === undefined ? "rest" : "rest-for-turns", turns: count ?? REST_UNTIL_RECOVERED_TURNS });
  }

  async chooseRestMode(count?: number): Promise<void> {
    if (this.#state.busy || this.#state.commandBlocked || this.#state.worldMap || !this.#state.status ||
        this.#state.targeting || this.#state.terrainInteractionMode || this.continuousAction) return;
    if (count !== undefined) { await this.restUntilRecovered(count); return; }
    const generation = this.#sessionGeneration;
    const input = this.#window.prompt(this.#localization.format("message-rest-mode-prompt"), "&");
    if (input === null || generation !== this.#sessionGeneration) return;
    const command = parseRestInput(input);
    if (!command) {
      if (input.trim() && !/^0+$/.test(input.trim())) this.#announce("message-rest-mode-invalid", undefined, "system");
      return;
    }
    await this.#rest(command);
  }

  async #rest(command: RestCommand): Promise<void> {
    if (this.#state.worldMap) return;
    await this.#runContinuousAction("rest", async action => {
      for (let turn = 0; turn < command.turns; turn++) {
        if (!await this.#continuousStep(action, { ...command, turns: 1 }, command)) return;
        const update = this.#state.status;
        const outcome = update && "events" in update
          ? update.events.find(event => event.outcome?.type === "rest")?.outcome : undefined;
        if (outcome?.type !== "rest") throw new Error("Core rest command returned no rest resolution");
        if (outcome.resolution.completedTurns !== 1 || outcome.resolution.stopReason !== "turn-limit") return;
      }
      if (command.type !== "rest-for-turns") this.#announce("message-rest-budget-reached", undefined, "system");
    });
  }

  startAbilityTargeting(ability: AbilityDto): void {
    if (
      this.#state.busy ||
      this.#state.commandBlocked ||
      !this.#state.status ||
      !ability.canCast
    ) {
      return;
    }
    this.startTargetingWithSpec(ability.targetSpec, {
      type: "ability",
      abilityId: ability.id,
    });
  }

  startTargetingWithSpec(
    spec: TargetSpecDto | null | undefined,
    intent: TargetingIntent,
  ): void {
    if (!this.#state.status || this.#state.busy || this.#state.commandBlocked) return;
    const next = beginTargeting(this.#state.status.player.position, spec ?? undefined);
    if (!next) {
      this.#announce("message-target-mode-unavailable", undefined, "system");
      this.render();
      return;
    }
    if (
      this.#state.targetingIntent?.type === "look" ||
      this.#state.targetingIntent?.type === "local-travel"
    ) {
      this.#onLookFocusChange(undefined);
    }
    const options = this.#state.status.operationOptions;
    next.targetPets = options.targetPets;
    const remembered = this.#rememberedTarget?.floorId === this.#state.status.floorId ? this.#rememberedTarget.target : undefined;
    this.#state.targeting = intent.type === "select-target" ? next
      : defaultTargetState(next, options.defaultTarget, remembered, this.#state.status.entities);
    this.#state.targetingIntent = intent;
    this.#announce("message-target-mode-started", undefined, "system");
    this.render();
  }

  cancelTargeting(announce = true): void {
    if (!this.#state.targeting) return;
    const cancelledDevice = announce && this.#state.targetingIntent?.type === "absorbed-device"
      ? this.#state.targetingIntent.itemId : undefined;
    const intent = this.#state.targetingIntent;
    const cancelledActivation = announce && intent?.type === "item"
      && [...this.#state.inventory, ...this.#state.equipment].some(item =>
        item.id === intent.itemId && item.activation && item.usable);
    const wasMapCursor =
      this.#state.targetingIntent?.type === "look" ||
      this.#state.targetingIntent?.type === "local-travel";
    this.#state.targeting = undefined;
    this.#state.targetingIntent = undefined;
    if (wasMapCursor) this.#onLookFocusChange(undefined);
    if (announce) {
      this.#announce("message-target-mode-cancelled", undefined, "system");
    }
    this.render();
    if (cancelledDevice) void this.#dispatch({ type: "use-absorbed-device", itemId: cancelledDevice, targets: [] });
    if (cancelledActivation && intent?.type === "item") {
      void this.#dispatch({ type: "use-item", itemId: intent.itemId });
    }
  }

  reconcileStatus(state: GameSnapshot | GameUpdate): void {
    const remembered = this.#rememberedTarget;
    if (remembered) {
      const target = remembered.target;
      if (state.floorId !== remembered.floorId || state.mapScale !== "local" ||
          (target.type === "entity" && !state.entities.some(entity => entity.id === target.entityId))) {
        this.#rememberedTarget = undefined;
      } else if (target.type === "position" && "mapTranslation" in state && state.mapTranslation) {
        const position = translatedLocalPosition(target.position, state.mapTranslation);
        if (position.x < 0 || position.y < 0 || position.x >= state.width || position.y >= state.height) this.#rememberedTarget = undefined;
        else remembered.target = { type: "position", position };
      }
    }
    if (!state.player.fishingDirection) this.#fishingStopped = false;
    else if (this.#installed && !this.#fishingStopped) void this.#startFishing();
    if (state.player.pendingDuelist) {
      this.cancelTargeting(false);
      this.#state.terrainInteractionMode = undefined;
      this.#ridingDirection = false;
    }
    this.#worldTravelDestination = state.worldTravelDestination ?? undefined;
    const mapTranslation = "mapTranslation" in state ? state.mapTranslation : undefined;
    if (
      this.#localTravelDestination &&
      this.#localTravelFloorId === state.floorId &&
      state.mapScale === "local" &&
      mapTranslation
    ) {
      this.#localTravelDestination = translatedLocalPosition(
        this.#localTravelDestination,
        mapTranslation,
      );
    }
    if (
      this.#localTravelFloorId &&
      (this.#localTravelFloorId !== state.floorId || state.mapScale === "world")
    ) {
      this.resetLocalTravel();
    }
    if (this.#state.targeting && mapTranslation) {
      const translated = translateTargetingState(
        this.#state.targeting,
        mapTranslation,
        state.width,
        state.height,
      );
      if (translated) {
        this.#state.targeting = translated;
        if (
          this.#state.targetingIntent?.type === "look" ||
          this.#state.targetingIntent?.type === "local-travel"
        ) {
          this.#onLookFocusChange(translated.cursor);
        }
      } else this.cancelTargeting(false);
    }
    if (
      this.#state.targeting &&
      (this.#state.targeting.origin.x !== state.player.position.x ||
        this.#state.targeting.origin.y !== state.player.position.y ||
        !this.#state.targetingIntent ||
        (this.#state.targetingIntent.type !== "look" &&
          this.#state.targetingIntent.type !== "local-travel" &&
          !targetSpecForIntent(state, this.#state.targetingIntent)))
    ) {
      this.cancelTargeting(false);
    }
    const pendingDirection = state.player.pendingMutationDirection;
    if (pendingDirection && this.#state.targetingIntent?.type !== "mutation-direction") {
      this.cancelTargeting(false);
      const targeting = beginTargeting(state.player.position, {
        modes: ["direction"],
        range: Math.max(state.width, state.height),
        requiresLineOfEffect: false,
      });
      if (targeting) {
        this.#state.targeting = targeting;
        this.#state.targetingIntent = { type: "mutation-direction" };
        this.#announce("message-mutation-direction-required", undefined, "mutation");
      }
    } else if (!pendingDirection && this.#state.targetingIntent?.type === "mutation-direction") {
      this.cancelTargeting(false);
    }
    if (state.player.pendingAbilityGlyph && !this.#glyphPromptPending) {
      this.cancelTargeting(false);
      this.#glyphPromptPending = true;
      const generation = this.#sessionGeneration;
      this.#window.setTimeout(() => {
        try {
          if (generation !== this.#sessionGeneration || !this.#state.status?.player.pendingAbilityGlyph) return;
          let glyph: string | null;
          do {
            glyph = this.#window.prompt(this.#localization.format("message-ability-glyph-required"));
          } while (glyph !== null && ([...glyph].length !== 1 || /[\u0000-\u001f\u007f-\u009f]/u.test(glyph)));
          if (generation !== this.#sessionGeneration) return;
          void this.#dispatch({ type: "resolve-ability-glyph", glyph });
        } finally {
          if (generation === this.#sessionGeneration) this.#glyphPromptPending = false;
        }
      }, 0);
    }
    const pendingAbilityDirection = state.player.pendingAbilityDirection;
    if (
      !pendingDirection &&
      pendingAbilityDirection &&
      this.#state.targetingIntent?.type !== "ability-direction"
    ) {
      this.cancelTargeting(false);
      const targeting = beginTargeting(state.player.position, {
        modes: ["direction"],
        range: Math.max(state.width, state.height),
        requiresLineOfEffect: false,
      });
      if (targeting) {
        this.#state.targeting = targeting;
        this.#state.targetingIntent = { type: "ability-direction" };
        this.#announce(
          pendingAbilityDirection.abilityId === "demo.ability.trump-shuffle"
            ? "message-trump-direction-required"
            : pendingAbilityDirection.abilityId === "demo.ability.chaos-call-chaos"
            ? "message-chaos-direction-required"
            : "message-ability-direction-required",
          undefined,
          "ability",
        );
      }
    } else if (
      !pendingAbilityDirection &&
      this.#state.targetingIntent?.type === "ability-direction"
    ) {
      this.cancelTargeting(false);
    }
  }

  render(): void {
    if (this.#mapDisplay.render(this.#dom.mapHost, this.#state, this.#getZoom()))
      this.#announce("display-left-trap-detection", undefined, "system");
    const looking = this.#state.targetingIntent?.type === "look";
    const localTravel = this.#state.targetingIntent?.type === "local-travel";
    const targeting = Boolean(this.#state.targeting && !looking && !localTravel);
    const connectionAction = connectionActionForState(this.#state);
    const waitingAtWarrensSurface =
      this.#state.worldId === "demo.world.middle-earth" &&
      this.#state.status?.floorId === "demo.floor.surface";
    this.#dom.traverseStairs.textContent = this.#localization.format(
      connectionAction === "enter-world-map"
        ? "action-enter-world-map"
        : connectionAction === "leave-world-map"
          ? "action-leave-world-map"
      : connectionAction === "enter-warrens"
        ? "action-enter-warrens"
        : connectionAction === "ascend"
          ? "action-stairs-ascend"
          : connectionAction === "descend"
            ? "action-stairs-descend"
            : waitingAtWarrensSurface
              ? "action-enter-warrens-unavailable"
              : "action-stairs-unavailable",
    );
    this.#dom.autoExplore.disabled = this.#state.busy || this.#state.commandBlocked ||
      this.#state.status?.mapScale !== "local" || Boolean(this.#state.targeting) ||
      Boolean(this.#state.terrainInteractionMode) || this.#ridingDirection || Boolean(this.#runDirectionPreset) || Boolean(this.#walkDirection) || Boolean(this.continuousAction);
    this.#dom.nearestUnknownItem.disabled = this.#dom.autoExplore.disabled;
    this.#dom.searchModeToggle.disabled = this.#dom.autoExplore.disabled;
    this.#dom.searchModeToggle.setAttribute("aria-pressed", String(Boolean(this.#state.status?.player.searching)));
    this.#dom.searchModeToggle.textContent = this.#localization.format(
      this.#state.status?.player.searching ? "action-search-mode-on" : "action-search-mode-off",
    );
    this.#dom.searchModeToggle.title = this.#localization.format("action-search-mode-help");
    this.#dom.traverseStairs.disabled =
      this.#state.busy || this.#state.commandBlocked || connectionAction === undefined;
    this.#dom.mapHost.dataset.connectionAction = connectionAction ?? "unavailable";
    this.#dom.targetModeToggle.textContent = this.#localization.format(
      targeting ? "action-target-cancel" : "action-target-start",
    );
    this.#dom.targetModeToggle.setAttribute(
      "aria-pressed",
      targeting ? "true" : "false",
    );
    this.#dom.targetModeToggle.disabled =
      this.#state.busy ||
      this.#state.playerDead ||
      this.#state.worldMap ||
      this.#state.status?.player.pendingDuelist != null ||
      localTravel ||
      (!targeting && this.#state.commandBlocked);
    this.#dom.lookModeToggle.textContent = this.#localization.format(
      looking ? "action-look-cancel" : "action-look-start",
    );
    this.#dom.lookModeToggle.setAttribute("aria-pressed", looking ? "true" : "false");
    this.#dom.lookModeToggle.disabled = this.#state.busy || this.#state.commandBlocked;
    this.#dom.mapHost.dataset.targeting = this.#state.targeting ? "true" : "false";
    this.#dom.mapHost.dataset.targetingAction = this.#state.targetingIntent?.type ?? "none";
    this.#dom.combatContext.dataset.targeting = this.#state.targeting ? "true" : "false";
    this.#dom.targetCursor.hidden = !this.#state.targeting;
    if (!this.#state.targeting) {
      this.#dom.targetModeStatus.textContent = this.#localization.format(
        this.#state.status?.player.fishingDirection ? "target-status-fishing"
          : "target-status-ready",
      );
      delete this.#dom.mapHost.dataset.targetX;
      delete this.#dom.mapHost.dataset.targetY;
      return;
    }

    const { origin, cursor, spec } = this.#state.targeting;
    const cameraX = Number(this.#dom.mapHost.dataset.cameraX ?? 0);
    const cameraY = Number(this.#dom.mapHost.dataset.cameraY ?? 0);
    const renderedCellSize = MAP_CELL_SIZE * this.#getZoom();
    this.#dom.targetCursor.style.left = `${cameraX + cursor.x * renderedCellSize}px`;
    this.#dom.targetCursor.style.top = `${cameraY + cursor.y * renderedCellSize}px`;
    this.#dom.targetCursor.style.width = `${renderedCellSize}px`;
    this.#dom.targetCursor.style.height = `${renderedCellSize}px`;
    this.#dom.mapHost.dataset.targetX = String(cursor.x);
    this.#dom.mapHost.dataset.targetY = String(cursor.y);
    this.#dom.targetModeStatus.textContent =
      looking || localTravel
        ? this.#localization.format(
            localTravel ? "local-travel-status-active" : "look-status-active",
            {
              x: cursor.x,
              y: cursor.y,
              contents: this.#describeLook(cursor),
            },
          )
        : this.#localization.format(this.#state.targetingIntent?.type === "select-target" ? "target-select-status-active" : "target-status-active", {
            contents: this.#describeLook(cursor),
            direction: this.#localization.format(
              `nearby-direction-${targetDirectionKey(origin, cursor)}`,
            ),
            distance: gridDistance(origin, cursor),
            range: spec.range,
          });
  }

  readonly #handleTargetToggle = (): void => {
    if (this.#state.targetingIntent?.type === "mutation-direction") return;
    if (this.#state.targeting && this.#state.targetingIntent?.type !== "look") {
      this.cancelTargeting();
    } else {
      this.cancelTargeting(false);
      this.startTargetSelection();
    }
  };

  readonly #handleLookToggle = (): void => {
    if (this.#state.targetingIntent?.type === "look") this.cancelTargeting();
    else {
      this.cancelTargeting(false);
      this.startLookMode();
    }
  };

  readonly #handleTraverseStairs = (): void => {
    if (this.#dom.traverseStairs.disabled) return;
    const action = connectionActionForState(this.#state);
    if (action === "enter-world-map") void this.#enterWorldMap();
    else void this.#dispatch(
      action === "leave-world-map"
        ? { type: "leave-world-map" }
        : { type: "traverse-stairs" },
    );
  };

  readonly #handleSearchModeToggle = (): void => { void this.#dispatch({ type: "toggle-search" }); };
  readonly #handleAutoExplore = (): void => { void this.autoExplore(); };
  readonly #handleNearestUnknownItem = (): void => { void this.travelToNearestUnknownItem(); };

  readonly #handleResize = (): void => {
    this.#window.requestAnimationFrame(() => this.render());
  };

  readonly #handleKeydown = (event: KeyboardEvent): void => this.#processKeydown(event);

  readonly #handleKeyup = (event: KeyboardEvent): void => {
    if (event.key === this.#heldMovement?.key.key ||
        (event.code && event.code === this.#heldMovement?.key.code)) this.#heldMovement = undefined;
  };

  executeOriginalKey(key: KeyboardEventInit): void {
    this.#processKeydown(new KeyboardEvent("keydown", key), "original");
  }

  #processKeydown(event: KeyboardEvent, forcedPreset?: InputPreset,
    movementKey: KeyboardEvent | undefined = forcedPreset ? undefined : event): void {
    if (!event.repeat) this.#heldMovement = undefined;
    if (
      event.defaultPrevented || event.isComposing || this.continuousAction || this.#window.document.hidden ||
      this.#dom.mapHost.ownerDocument.querySelector("dialog[open]") ||
      isTextInput(event.target)
    ) { this.#heldMovement = undefined; return; }
    if (event.repeat) {
      const held = this.#heldMovement;
      if (!held || this.#state.commandBlocked || this.#state.targeting || this.#state.terrainInteractionMode ||
          this.#ridingDirection || this.#walkDirection || this.#runDirectionPreset ||
          this.#state.status?.mogaminator.pendingQuery ||
          event.key !== held.key.key || event.code !== held.key.code ||
          (["ctrlKey", "altKey", "metaKey", "shiftKey"] as const).some(modifier =>
            Boolean(event[modifier]) !== Boolean(held.key[modifier]))) {
        this.#heldMovement = undefined;
        return;
      }
      event.preventDefault(); event.stopImmediatePropagation();
      // Use only OS key-repeat events. Busy commands are dropped, never queued.
      if (!this.#state.busy) void this.#dispatch({ type: "move", direction: held.direction });
      return;
    }
    const preset = forcedPreset ?? (this.#literalCommand ? "original" : this.#getInputPreset());
    if (!forcedPreset && !this.#literalCommand && this.#countInput === undefined && event.key !== "\\" &&
        !this.#state.busy && !this.#state.commandBlocked && !this.#state.targeting && !this.#state.terrainInteractionMode &&
        !this.#ridingDirection && !this.#runDirectionPreset && !this.#walkDirection &&
        this.#customKey(event, key => this.#processKeydown(new KeyboardEvent("keydown", key), "original", movementKey))) {
      event.preventDefault(); event.stopImmediatePropagation(); return;
    }
    if (event.key === "\\" && !event.ctrlKey && !event.altKey && !event.metaKey && !this.#state.busy && !this.#state.commandBlocked &&
        !this.#state.targeting && !this.#state.terrainInteractionMode && !this.#ridingDirection && !this.#runDirectionPreset && !this.#walkDirection) {
      event.preventDefault(); this.#literalCommand = true;
      this.#announce("message-command-literal", undefined, "system"); return;
    }
    if (!this.#state.busy && !this.#state.commandBlocked && !this.#state.targeting &&
        !this.#state.terrainInteractionMode && !this.#ridingDirection && !this.#runDirectionPreset && !this.#walkDirection &&
        (preset === "original" || preset === "roguelike")) {
      if (this.#countInput !== undefined) {
        if (event.key === "Escape") {
          event.preventDefault(); event.stopImmediatePropagation();
          this.#countInput = undefined; this.#commandCount = undefined;
          this.#announce("message-door-mode-cancelled", undefined, "system"); return;
        }
        if (event.key === "Backspace" || event.key === "Delete" || (event.ctrlKey && event.key.toLowerCase() === "h")) {
          event.preventDefault(); event.stopImmediatePropagation();
          this.#countInput = Math.trunc(this.#countInput / 10);
          this.#announce("message-command-count", { count: this.#countInput }, "system"); return;
        }
        if (!event.ctrlKey && !event.metaKey && !event.altKey && /^[0-9]$/.test(event.key)) {
          event.preventDefault(); event.stopImmediatePropagation();
          this.#countInput = Math.min(9999, this.#countInput * 10 + Number(event.key));
          this.#announce("message-command-count", { count: this.#countInput }, "system"); return;
        }
        this.#commandCount = this.#countInput || 99; this.#countInput = undefined;
        if (event.key === " " || event.key === "Enter") {
          event.preventDefault(); event.stopImmediatePropagation();
          this.#announce("message-command-count-ready", { count: this.#commandCount }, "system"); return;
        }
      } else if (event.key === "0" && !event.ctrlKey && !event.metaKey && !event.altKey) {
        event.preventDefault(); event.stopImmediatePropagation();
        this.#countInput = 0; this.#commandCount = undefined;
        this.#announce("message-command-count", { count: 0 }, "system"); return;
      }
      if (event.key === "Escape") { this.#commandCount = undefined; this.#literalCommand = false; }
    }
    const shortcut = commandShortcut(event, this.#literalCommand ? "original" : preset);
    const alterDirection = alterDirectionForKeyboardInput(event, preset);
    if (alterDirection && !this.#state.busy && !this.#state.commandBlocked && !this.#state.worldMap &&
        !this.#state.targeting && !this.#state.terrainInteractionMode && !this.#ridingDirection && !this.#runDirectionPreset && !this.#walkDirection) {
      event.preventDefault(); event.stopImmediatePropagation(); this.#literalCommand = false;
      void this.dispatchCounted({ type: "alter", direction: alterDirection }); return;
    }
    if (this.#state.busy && shortcut !== "save" && shortcut !== "save-exit") return;
    if (shortcut && (event.ctrlKey || event.key === "=" || ["help", "knowledge", "end-character", "input-config", "command-menu", "notes", "screen-export", "glyphs", "colors", "advanced-preferences", "reload-pickup-rules"].includes(shortcut)) &&
        !this.#state.targeting && !this.#state.terrainInteractionMode && !this.#ridingDirection && !this.#runDirectionPreset && !this.#walkDirection) {
      this.#literalCommand = false;
      event.preventDefault();
      this.#executeShortcut(shortcut);
      return;
    }
    const deviceShortcut = !this.#state.targeting && !this.#state.terrainInteractionMode && !this.#ridingDirection && !this.#runDirectionPreset && !this.#walkDirection &&
      !this.#state.worldMap && !this.#state.commandBlocked && event.altKey && !event.ctrlKey && !event.metaKey;
    if (event.ctrlKey || event.metaKey || event.altKey) {
      this.#commandCount = undefined;
      if (deviceShortcut && this.#openDeviceCommand(event.key.toLowerCase())) event.preventDefault();
      else if (isAutoGetShortcut(event) && !this.#state.targeting && !this.#state.terrainInteractionMode && !this.#ridingDirection && !this.#runDirectionPreset && !this.#walkDirection) {
        event.preventDefault();
        if (!this.#state.worldMap) void this.autoGet();
      }
      return;
    }
    if (event.target instanceof HTMLButtonElement && (event.key === " " || event.key === "Enter")) return;
    if (this.#state.targeting) {
      this.#handleTargetingKey(event);
      event.stopImmediatePropagation();
      return;
    }
    if (this.#state.commandBlocked) return;
    if (this.#state.terrainInteractionMode) {
      this.#handleTerrainDirection(event);
      event.stopImmediatePropagation();
      return;
    }
    if (this.#ridingDirection) {
      this.#handleRidingDirection(event);
      event.stopImmediatePropagation();
      return;
    }

    if (this.#walkDirection) {
      event.preventDefault(); event.stopImmediatePropagation();
      if (event.key === "Escape") {
        this.#walkDirection = undefined;
        this.#commandCount = undefined;
        this.#announce("message-door-mode-cancelled", undefined, "system");
      } else {
        const direction = directionForKeyboardInput(event, this.#walkDirection.preset);
        if (direction) {
          const type = this.#walkDirection.special ? "walk-special" : "move";
          this.#walkDirection = undefined;
          void this.dispatchCounted({ type, direction });
        }
      }
      return;
    }
    if (this.#runDirectionPreset) {
      event.preventDefault(); event.stopImmediatePropagation();
      const selectedPreset = this.#runDirectionPreset;
      if (event.key === "Escape") {
        this.#runDirectionPreset = undefined;
        this.#commandCount = undefined;
        this.#announce("message-door-mode-cancelled", undefined, "system");
      } else {
        const direction = directionForKeyboardInput(event, selectedPreset);
        if (direction) { this.#runDirectionPreset = undefined; void this.run(direction); }
      }
      return;
    }
    const runPreset = this.#literalCommand ? "original" : preset;
    if (event.key === "*") {
      event.preventDefault(); this.#commandCount = undefined; this.#literalCommand = false;
      this.startTargetSelection(); return;
    }
    if ((runPreset === "original" || runPreset === "roguelike") && (event.key === "-" || event.key === ";")) {
      event.preventDefault(); event.stopImmediatePropagation(); this.#literalCommand = false;
      this.#walkDirection = { preset: runPreset, special: event.key === "-" };
      this.#announce(event.key === "-" ? "message-special-walk-direction" : "message-walk-direction", undefined, "system");
      return;
    }
    const runDirection = runDirectionForKeyboardInput(event, runPreset);
    if (runDirection || isRunPrefix(event.key, runPreset)) {
      event.preventDefault(); event.stopImmediatePropagation(); this.#literalCommand = false;
      if (!this.#state.worldMap) {
        if (runDirection) void this.run(runDirection);
        else { this.#runDirectionPreset = runPreset; this.#announce("message-run-direction", undefined, "system"); }
      } else this.#commandCount = undefined;
      return;
    }

    if (preset === "original" || preset === "roguelike") {
      this.#literalCommand = false;
      if (shortcut) { event.preventDefault(); this.#executeShortcut(shortcut); return; }
      if (["Z", "_", "`", "]", "O", "<", ">"].includes(event.key)) this.#commandCount = undefined;
      if (event.key === "Z") { event.preventDefault(); void this.autoExplore(); }
      else if (event.key === "_") { event.preventDefault(); this.#openMogaminator(); }
      else if (event.key === "`") { event.preventDefault(); this.startLocalTravelSelection(); }
      else if (event.key === "]" || (preset === "original" && event.key === "O")) { event.preventDefault(); this.#openObjectList(); }
      else if (event.key === "<" || event.key === ">") { event.preventDefault(); this.#handleTraverseStairs(); }
      else {
        const command = commandForKeyboardInput(event, runPreset);
        if (command) {
          if (command.type === "move" && movementKey && this.#commandCount === undefined) {
            this.#heldMovement = { key: movementKey, direction: command.direction };
          }
          event.preventDefault(); void this.dispatchCounted(command);
        }
        else this.#commandCount = undefined;
      }
      event.stopImmediatePropagation();
      return;
    }

  };

  #executeShortcut(shortcut: CommandShortcut): void {
    const terrain = { spike: "spike-door", alter: "alter", open: "open-door", close: "close-door", bash: "bash-door", disarm: "disarm-trap", dig: "dig-terrain" } as const;
    if (shortcut in terrain) { this.#startTerrainInteraction(terrain[shortcut as keyof typeof terrain]); return; }
    const count = this.#takeCommandCount();
    if (shortcut === "look") this.startLookMode();
    else if (shortcut === "repeat-last") void this.repeatLastCommand(count);
    else if (shortcut === "toggle-search") this.#handleSearchModeToggle();
    else if (shortcut === "search") void this.dispatchCounted({ type: "search" }, count);
    else if (shortcut === "fire") this.startProjectileTargeting();
    else if (shortcut === "rest") void this.chooseRestMode(count);
    else if (shortcut === "pickup") void this.#dispatch({ type: "pick-up" });
    else if (shortcut === "nearest-unknown-item") void this.travelToNearestUnknownItem();
    else if (shortcut === "resume-travel") {
      const destination = this.#state.worldMap ? (this.#worldTravelDestination ?? this.#state.status?.worldTravelDestination) : this.#localTravelDestination;
      if (destination) void (this.#state.worldMap ? this.#travelWorldTo(destination) : this.travelLocalTo(destination, this.#localTravelObjectId));
      else this.#announce("message-local-travel-no-destination", undefined, "system");
    } else this.#onShortcut(shortcut, count);
  }

  startRiding(): void {
    if (this.#state.busy || this.#state.commandBlocked || this.#state.worldMap || this.continuousAction) return;
    this.#runDirectionPreset = undefined;
    this.#walkDirection = undefined;
    this.#countInput = undefined;
    this.#commandCount = undefined;
    this.#ridingDirection = true;
    this.#announce("message-riding-mode-started", undefined, "system");
  }

  #handleTargetingKey(event: KeyboardEvent): void {
    if (event.key === "Escape" || event.key === "q") {
      event.preventDefault();
      event.stopImmediatePropagation();
      if (this.#state.targetingIntent?.type === "mutation-direction") return;
      if (this.#state.targetingIntent?.type === "ability-direction") {
        this.cancelTargeting(false);
        void this.#dispatch({ type: "cancel-ability-direction" });
        return;
      }
      this.cancelTargeting();
      return;
    }
    const localTravel = this.#state.targetingIntent?.type === "local-travel";
    const aim = this.#state.targeting;
    const intent = this.#state.targetingIntent;
    const interactiveTarget = aim && intent && !["look", "local-travel", "mutation-direction", "ability-direction"].includes(intent.type);
    if (interactiveTarget && [" ", "*", "+", "-", "m", "o", "p"].includes(event.key)) {
      event.preventDefault();
      if (event.key === "o" || event.key === "p") {
        this.#state.targeting = { ...aim, list: false, cursor: event.key === "p" ? { ...aim.origin } : aim.cursor };
      } else {
        this.#state.targeting = cycleTarget(aim, this.#state.status?.entities ?? [], event.key === "-" ? -1 : event.key === "m" ? 0 : 1);
      }
      this.render(); return;
    }
    if (interactiveTarget && ["t", "T", ".", "5", "0"].includes(event.key)) {
      event.preventDefault();
      if (samePosition(aim.cursor, aim.origin) && intent.type !== "select-target") {
        const remembered = this.#rememberedTarget;
        const next = remembered && rememberedTargetState(aim, remembered.target, this.#state.status?.entities ?? []);
        if (!next) { this.#announce("message-target-selection-invalid", undefined, "system"); return; }
        this.#state.targeting = next;
        const oldTarget = remembered.target;
        void this.#confirmTargeting((oldTarget.type === "entity" || oldTarget.type === "position") && aim.spec.modes.includes(oldTarget.type) ? oldTarget : undefined);
        return;
      }
      void this.#confirmTargeting(); return;
    }
    if (localTravel && (event.key === "<" || event.key === ">")) {
      event.preventDefault();
      const current = this.#state.targeting?.cursor;
      const destination = current
        ? nextTravelConnectionPosition(this.#state, event.key, current)
        : undefined;
      if (!destination || !this.#state.targeting) {
        this.#announce("message-local-travel-stair-unavailable", undefined, "system");
        return;
      }
      this.#state.targeting = { ...this.#state.targeting, cursor: destination };
      this.#onLookFocusChange(destination);
      this.render();
      return;
    }
    if (
      event.key === "Enter" ||
      (localTravel && [" ", ".", "5", "0", "t"].includes(event.key))
    ) {
      event.preventDefault();
      if (this.#state.targetingIntent?.type === "look") {
        const destination = this.#state.targeting?.cursor;
        const worldMap = this.#state.worldMap;
        this.cancelTargeting(false);
        if (worldMap && destination) void this.#travelWorldTo(destination);
      } else if (localTravel) {
        const destination = this.#state.targeting?.cursor;
        const origin = this.#state.targeting?.origin;
        if (!destination || !origin || samePosition(destination, origin)) {
          this.#announce("message-target-selection-invalid", undefined, "system");
          return;
        }
        this.cancelTargeting(false);
        void this.travelLocalTo(destination);
      } else void this.#confirmTargeting();
      return;
    }
    const direction = directionForKeyboardInput(event, this.#getInputPreset());
    if (!direction || !this.#state.targeting) return;
    event.preventDefault();
    this.#state.targeting = moveTarget(
      this.#state.targeting,
      direction,
      this.#state.status?.entities ?? [],
      this.#state.mapWidth,
      this.#state.mapHeight,
    );
    if (
      this.#state.targetingIntent?.type === "look" ||
      this.#state.targetingIntent?.type === "local-travel"
    ) {
      this.#onLookFocusChange(this.#state.targeting.cursor);
    }
    this.render();
  }

  #handleTerrainDirection(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopImmediatePropagation();
      this.#state.terrainInteractionMode = undefined;
      this.#commandCount = undefined;
      this.#announce("message-door-mode-cancelled", undefined, "system");
      return;
    }
    if (["5", ".", " "].includes(event.key) && this.#state.terrainInteractionMode) {
      const mode = this.#state.terrainInteractionMode;
      const chests = this.#adjacentChests(mode).filter(item => samePosition(item.position, this.#state.status!.player.position));
      if (chests.length) {
        event.preventDefault();
        this.#state.terrainInteractionMode = undefined;
        this.#chooseChest(mode === "open-door" ? "open-chest" : "disarm-chest", chests, this.#takeCommandCount());
      }
      return;
    }
    const direction = directionForKeyboardInput(event, this.#getInputPreset()) ??
      (this.#state.terrainInteractionMode === "alter"
        ? runDirectionForKeyboardInput({ key: event.key, code: event.code, shiftKey: true }, "original") : undefined);
    if (!direction || !this.#state.terrainInteractionMode) return;
    event.preventDefault();
    const mode = this.#state.terrainInteractionMode;
    this.#state.terrainInteractionMode = undefined;
    if (mode === "alter" || mode === "spike-door") {
      void this.dispatchCounted(terrainInteractionCommand(mode, direction));
      return;
    }
    const chest = this.#adjacentChests(mode).filter(item => {
      const origin = this.#state.status!.player.position;
      const spec = { modes: ["direction" as const], range: 1, requiresLineOfEffect: false };
      const selection = targetSelectionAtCursor({ origin, cursor: item.position, spec }, []);
      return selection?.type === "direction" && selection.direction === direction;
    });
    if (chest.length) {
      this.#chooseChest(mode === "open-door" ? "open-chest" : "disarm-chest", chest, this.#takeCommandCount());
      return;
    }
    if (this.#commandCount !== undefined) {
      void this.dispatchCounted(terrainInteractionCommand(mode, direction)); return;
    }
    const interaction = this.#state.status
      ? terrainInteractionForDirection(
          this.#state.status.terrainInteractions,
          mode,
          direction,
        )
      : undefined;
    if (!interaction) {
      this.#announce("message-terrain-interaction-not-applicable", undefined, "system");
      return;
    }
    if (!interaction.available) {
      this.#announce(
        interaction.unavailableReason === "occupied-by-actor"
          ? "message-terrain-interaction-blocked-actor"
          : "message-terrain-interaction-blocked-item",
        undefined,
        "system",
      );
      return;
    }
    void this.dispatchCounted(terrainInteractionCommand(mode, direction));
  }

  #handleRidingDirection(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopImmediatePropagation();
      this.#ridingDirection = false;
      this.#announce("message-riding-mode-cancelled", undefined, "system");
      return;
    }
    const direction = directionForKeyboardInput(event, this.#getInputPreset());
    if (!direction) return;
    event.preventDefault();
    this.#ridingDirection = false;
    void this.#dispatch({ type: "ride", direction });
  }

  #startTerrainInteraction(mode: TerrainInteractionMode): void {
    if (this.#state.busy || this.#state.commandBlocked || this.#state.worldMap) {
      this.#commandCount = undefined; return;
    }
    if (
      !this.#state.status ||
      (this.#commandCount === undefined && mode !== "alter" && mode !== "spike-door" && terrainInteractionsForMode(this.#state.status.terrainInteractions, mode).length === 0 && this.#adjacentChests(mode).length === 0)
    ) {
      this.#commandCount = undefined;
      this.#announce("message-terrain-interaction-mode-unavailable", undefined, "system");
      return;
    }
    this.#state.terrainInteractionMode = mode;
    this.#announce(terrainModeMessageKey(mode), undefined, "system");
  }

  #adjacentChests(mode: TerrainInteractionMode): ItemDto[] {
    const status = this.#state.status;
    if (!status || (mode !== "open-door" && mode !== "disarm-trap")) return [];
    return status.items.filter(item => item.chest && gridDistance(status.player.position, item.position) <= 1 &&
      (mode === "open-door" ? item.chest.canOpen : item.chest.canDisarm));
  }

  async #startFishing(cancelled = false): Promise<void> {
    const generation = this.#sessionGeneration;
    await this.#runContinuousAction("fishing", async action => {
      // Loading may synchronize the interface locale before fishing can resume.
      await this.#whenIdle();
      if (generation !== this.#sessionGeneration) return;
      if (!action.cancelled) await this.#pauseContinuousAction(action);
      while (!action.cancelled && this.#installed && this.#state.status?.player.fishingDirection) {
        if (!await this.#continuousStep(action, { type: "continue-fishing" })) break;
      }
      if (generation !== this.#sessionGeneration) return;
      this.#fishingStopped = true;
      if (action.cancelled && this.#installed && this.#state.status?.player.fishingDirection) {
        await this.#dispatch({ type: "cancel-fishing" });
      }
    }, cancelled);
  }

  async #confirmTargeting(selected?: TargetSelection): Promise<void> {
    const state = this.#state.targeting;
    const status = this.#state.status;
    const intent = this.#state.targetingIntent;
    if (
      !state ||
      !status ||
      !intent ||
      intent.type === "look" ||
      intent.type === "local-travel" ||
      this.#state.busy ||
      this.#state.playerDead
    ) {
      return;
    }
    const target = selected ?? targetSelectionAtCursor(state, status.entities);
    if (!target) {
      this.#announce("message-target-selection-invalid", undefined, "system");
      return;
    }
    if (target.type !== "direction") this.#rememberedTarget = { target, floorId: status.floorId };
    this.cancelTargeting(false);
    if (intent.type === "select-target") {
      this.#announce("message-target-selected", undefined, "system");
      return;
    }
    await this.#dispatch(
      intent.type === "mutation-direction" && target.type === "direction"
        ? { type: "resolve-mutation-direction", direction: target.direction }
        : intent.type === "ability-direction" && target.type === "direction"
          ? { type: "resolve-ability-direction", direction: target.direction }
        : intent.type === "ability"
        ? { type: "cast-ability", abilityId: intent.abilityId, target }
        : intent.type === "item"
          ? { type: "use-item", itemId: intent.itemId, target }
          : intent.type === "absorbed-device"
            ? { type: "use-absorbed-device", itemId: intent.itemId, targets: [target] }
          : intent.type === "throw" && target.type === "direction"
            ? { type: "throw", itemId: intent.itemId, direction: target.direction }
          : { type: "fire-target", target },
    );
  }

  async #enterWorldMap(): Promise<void> {
    const status = this.#state.status;
    if (!status) return;
    const cancelRecall = status.player.recall?.remainingTurns != null;
    if (
      cancelRecall &&
      !this.#window.confirm(this.#localization.format("confirm-world-map-cancel-recall"))
    ) {
      return;
    }
    await this.#dispatch({ type: "enter-world-map", cancelRecall });
  }

  #takeCommandCount(): number | undefined {
    const count = this.#commandCount;
    this.#commandCount = undefined;
    return count;
  }

  async repeatLastCommand(count?: number): Promise<void> {
    if (this.#state.busy || this.#state.commandBlocked || this.continuousAction ||
        this.#state.targeting || this.#state.terrainInteractionMode) return;
    const command = this.#getLastCommand();
    if (!command) {
      this.#announce("message-repeat-last-empty", undefined, "system");
      return;
    }
    await this.repeatCommand(command, count);
  }

  async repeatCommand(command: GameCommand, count?: number): Promise<void> {
    if (this.#state.busy || this.#state.commandBlocked || this.continuousAction ||
        this.#state.targeting || this.#state.terrainInteractionMode || !this.#confirmRepeat(command)) return;
    switch (command.type) {
      case "run": await this.run(command.direction, count ?? command.maxSteps ?? undefined); return;
      case "auto-explore": await this.autoExplore(); return;
      case "rest": case "rest-for-turns": case "rest-until-resources":
        await this.#rest(count === undefined ? command : { type: "rest-for-turns", turns: count }); return;
      case "travel-local": await this.travelLocalTo(command.destination); return;
      case "travel-unknown-item": await this.travelLocalTo(command.destination, command.objectId); return;
      case "travel-world": await this.#travelWorldTo(command.destination); return;
      case "find-nearest-unknown-item": await this.travelToNearestUnknownItem(); return;
      case "enter-world-map": await this.#enterWorldMap(); return;
      // RFB applies always_repeat before expanding n, so n alone retries once.
      default: await this.dispatchCounted(command, count ?? (isAutomaticallyRepeatedCommand(command) ? 1 : undefined));
    }
  }

  async dispatchCounted(command: GameCommand, count = this.#takeCommandCount()): Promise<void> {
    if (count === undefined && isAutomaticallyRepeatedCommand(command) && this.#state.status?.operationOptions.autoRepeat) count = 99;
    if (count === undefined) { await this.#dispatch(command); return; }
    await this.#runContinuousAction("repeat", async action => {
      for (let completed = 0; completed < count; completed++) {
        if (!await this.#continuousStep(action, command)) return;
        const status = this.#state.status;
        if (!status || !("commandRepeatable" in status) || !status.commandRepeatable) return;
      }
    });
  }

  async playMacro(commands: readonly GameCommand[]): Promise<void> {
    if (this.#state.targeting || this.#state.terrainInteractionMode || this.#ridingDirection || this.#walkDirection || this.#runDirectionPreset) return;
    await this.#runContinuousAction("macro", async action => {
      for (const command of commands) {
        const before = this.#state.status;
        if (!before || !this.#confirmRepeat(command) || !await this.#continuousStep(action, structuredClone(command))) return;
        const after = this.#state.status!;
        if (after.floorId !== before.floorId || after.mapScale !== before.mapScale || ("mapTranslation" in after && (after.mapTranslation?.x || after.mapTranslation?.y)) ||
            after.player.hp < before.player.hp || after.entities.some(entity => entity.faction === "hostile") || searchDiscoveredSomething(after) ||
            after.player.statuses.some(status => ["rfb.status.confusion", "rfb.status.blindness"].includes(status.kindId)) ||
            (["move", "walk-special"].includes(command.type) && before.player.position.x === after.player.position.x && before.player.position.y === after.player.position.y)) return;
      }
    });
  }

  async run(direction: Direction, maxSteps = this.#takeCommandCount()): Promise<void> {
    await this.#automaticMovement("run", { type: "run", direction, ...(maxSteps === undefined ? {} : { maxSteps }) });
  }

  async autoExplore(): Promise<void> {
    if (this.#state.targeting || this.#state.terrainInteractionMode || this.#ridingDirection || this.#runDirectionPreset || this.#walkDirection) return;
    await this.#automaticMovement("auto-explore", { type: "auto-explore" });
  }

  async #automaticMovement(kind: "run" | "auto-explore", start: GameCommand): Promise<void> {
    if (this.#state.status?.mapScale !== "local") return;
    const generation = this.#sessionGeneration;
    const active = () => kind === "run" ? this.#state.status?.player.running : this.#state.status?.player.autoExplore;
    const continueCommand: GameCommand = { type: kind === "run" ? "continue-run" : "continue-auto-explore" };
    const cancelCommand: GameCommand = { type: kind === "run" ? "cancel-run" : "cancel-auto-explore" };
    await this.#runContinuousAction(kind, async action => {
      try {
        if (!await this.#continuousStep(action, start)) return;
        while (active()) {
          if (!await this.#continuousStep(action, continueCommand)) return;
        }
      } finally {
        if (generation === this.#sessionGeneration && active()) await this.#dispatch(cancelCommand);
      }
    });
  }

  async travelToNearestUnknownItem(): Promise<void> {
    if (this.#state.status?.mapScale !== "local" || this.#state.targeting || this.#state.terrainInteractionMode || this.#ridingDirection || this.#runDirectionPreset || this.#walkDirection) return;
    await this.#runContinuousAction("local-travel", async action => {
      if (!await this.#continuousStep(action, { type: "find-nearest-unknown-item" })) return;
      const status = this.#state.status;
      if (!status || !("events" in status)) return;
      const outcome = status.events.find(event => event.outcome?.type === "unknown-item-travel-target")?.outcome;
      if (outcome?.type !== "unknown-item-travel-target") return;
      this.#localTravelDestination = outcome.target.position;
      this.#localTravelObjectId = outcome.target.objectId;
      this.#localTravelFloorId = status.floorId;
      await this.#followLocalTravel(action, false);
    });
  }

  async travelLocalTo(destination: Position, objectId?: string): Promise<void> {
    const initial = this.#state.status;
    if (!initial || initial.mapScale !== "local" || this.#state.commandBlocked) return;
    await this.#runContinuousAction("local-travel", async action => {
      this.#localTravelDestination = destination;
      this.#localTravelFloorId = initial.floorId;
      this.#localTravelObjectId = objectId;
      this.#announce("message-local-travel-started", undefined, "system");
      await this.#followLocalTravel(action);
    });
  }

  async #followLocalTravel(action: ContinuousAction, remember = true): Promise<void> {
    for (;;) {
      const before = this.#state.status;
      const activeDestination = this.#localTravelDestination;
      if (
        !before ||
        !activeDestination ||
        before.mapScale !== "local" ||
        before.floorId !== this.#localTravelFloorId ||
        samePosition(before.player.position, activeDestination)
      ) {
        return;
      }
      const command: GameCommand = this.#localTravelObjectId
        ? { type: "travel-unknown-item", objectId: this.#localTravelObjectId, destination: activeDestination }
        : { type: "travel-local", destination: activeDestination };
      if (!await this.#continuousStep(action, command, remember ? command : false)) return;
      const current = this.#state.status;
      const translatedDestination = this.#localTravelDestination;
      if (
        !translatedDestination ||
        localTravelStopsAfterStep(before, current, translatedDestination)
      ) {
        return;
      }
    }
  }

  async #travelWorldTo(destination: Position): Promise<void> {
    await this.#runContinuousAction("world-travel", async action => {
      this.#worldTravelDestination = destination;
      for (;;) {
        const status = this.#state.status;
        if (!status || status.mapScale !== "world") return;
        if (
          status.player.position.x === destination.x &&
          status.player.position.y === destination.y
        ) {
          this.#worldTravelDestination = undefined;
          return;
        }
        const previous = status.player.position;
        if (!await this.#continuousStep(action, { type: "travel-world", destination })) return;
        const current = this.#state.status;
        if (
          !current ||
          current.mapScale !== "world" ||
          (current.player.position.x === previous.x && current.player.position.y === previous.y)
        ) {
          return;
        }
      }
    });
  }

  get continuousAction(): ContinuousAction["kind"] | undefined {
    return this.#continuousAction?.kind ?? (this.#state.status?.player.fishingDirection ? "fishing" : undefined);
  }

  async stopContinuousAction(): Promise<void> {
    this.#heldMovement = undefined;
    this.#countInput = undefined;
    this.#commandCount = undefined;
    this.#state.terrainInteractionMode = undefined;
    this.#literalCommand = false;
    this.#runDirectionPreset = undefined;
    this.#walkDirection = undefined;
    const generation = this.#sessionGeneration;
    const action = this.#continuousAction;
    this.#fishingStopped = true;
    if (action && !action.cancelled) {
      action.cancelled = true;
      action.resume?.();
      this.#announce("message-continuous-action-stopped", undefined, "system");
    }
    await this.#whenIdle();
    if (action) await action.done;
    if (generation !== this.#sessionGeneration) return;
    if (!action && this.#installed && this.#state.status?.player.fishingDirection) {
      if (this.#continuousAction) await this.stopContinuousAction();
      else await this.#startFishing(true);
    }
  }

  async #runContinuousAction(kind: ContinuousAction["kind"], run: (action: ContinuousAction) => Promise<void>, cancelled = false): Promise<void> {
    if (this.#continuousAction || this.#state.busy || this.#state.commandBlocked) return;
    const action: ContinuousAction = { kind, cancelled, done: Promise.resolve() };
    this.#continuousAction = action;
    action.done = Promise.resolve().then(async () => {
      if (!action.cancelled || kind === "fishing") await run(action);
    }).finally(() => {
      if (this.#continuousAction === action) {
        this.#continuousAction = undefined;
        this.#onContinuousActionChange();
      }
    });
    this.#onContinuousActionChange();
    await action.done;
  }

  async #continuousStep(action: ContinuousAction, command: GameCommand, repeatCommand: GameCommand | false = command): Promise<boolean> {
    if (action.cancelled || this.#state.commandBlocked) return false;
    if (await this.#dispatch(command, repeatCommand) !== "applied" || action.cancelled || this.#state.commandBlocked ||
        this.#state.status?.mogaminator.pendingQuery) return false;
    await this.#pauseContinuousAction(action);
    return !action.cancelled && !this.#state.commandBlocked;
  }

  #pauseContinuousAction(action: ContinuousAction): Promise<void> {
    // Yield a task between commands so native input can stop the next step.
    return new Promise<void>(resolve => {
      const timer = this.#window.setTimeout(() => { action.resume = undefined; resolve(); }, action.kind === "fishing" ? 10 : 0);
      action.resume = () => { this.#window.clearTimeout(timer); action.resume = undefined; resolve(); };
    });
  }

  readonly #stopOnBlur = (): void => { void this.stopContinuousAction(); };
  readonly #stopWhenHidden = (): void => {
    if (this.#window.document.hidden) void this.stopContinuousAction();
  };

  readonly #interruptContinuousKey = (event: KeyboardEvent): void => {
    if (event.defaultPrevented || event.isComposing || (event.repeat && (this.continuousAction === "run" || this.continuousAction === "auto-explore" || this.continuousAction === "repeat" || this.continuousAction === "macro"))) return;
    const shortcut = commandShortcut(event, this.#getInputPreset());
    if (this.continuousAction && (shortcut === "save" || shortcut === "save-exit")) {
      event.preventDefault(); event.stopImmediatePropagation(); this.#onShortcut(shortcut); return;
    }
    if (event.key === "Escape" && (event.ctrlKey || event.altKey || event.metaKey)) return;
    if (!this.continuousAction) return;
    if (this.#hasCustomKey(event) && !isTextInput(event.target)) {
      event.preventDefault(); event.stopImmediatePropagation(); void this.stopContinuousAction(); return;
    }
    if (event.key !== "Escape") {
      if (isTextInput(event.target)) return;
      const deviceShortcut = !this.#state.worldMap && this.#state.status?.player.magicEater &&
        !event.ctrlKey && !event.metaKey && ["m", "a", "u", "z"].includes(event.key.toLowerCase());
      if ((event.ctrlKey || event.metaKey || event.altKey) && !shortcut && !isAutoGetShortcut(event) && !deviceShortcut && !alterDirectionForKeyboardInput(event, this.#getInputPreset())) return;
      if (!shortcut && !alterDirectionForKeyboardInput(event, this.#getInputPreset()) && !runDirectionForKeyboardInput(event, this.#getInputPreset()) &&
          !isRunPrefix(event.key, this.#getInputPreset()) && !commandForKeyboardInput(event, this.#getInputPreset()) &&
          !terrainInteractionModeForKey(event.key) && !terrainSearchCommandForKey(event.key) &&
          !["0", "-", ";", "_", "]", "O", "J", "`", "Z"].includes(event.key) &&
          !["x", "f", "v", "i", "m"].includes(event.key.toLowerCase()) && !isAutoGetShortcut(event) && !deviceShortcut) return;
    }
    event.preventDefault();
    event.stopImmediatePropagation();
    void this.stopContinuousAction();
  };

  readonly #interruptContinuousClick = (event: MouseEvent): void => {
    this.#heldMovement = undefined;
    if (this.#countInput !== undefined || this.#commandCount !== undefined || this.#walkDirection) {
      this.#countInput = undefined;
      this.#commandCount = undefined;
      this.#state.terrainInteractionMode = undefined;
      this.#runDirectionPreset = undefined;
      this.#walkDirection = undefined;
    }
    if (!this.continuousAction) return;
    if (event.target instanceof Node && this.#dom.mapHost.contains(event.target)) {
      event.preventDefault();
      event.stopImmediatePropagation();
    }
    void this.stopContinuousAction();
  };
}

export function isObjectListShortcut(
  event: Pick<KeyboardEvent, "key" | "shiftKey" | "ctrlKey" | "altKey" | "metaKey">,
): boolean {
  if (event.ctrlKey || event.altKey || event.metaKey) return false;
  return event.key === "]" || (event.shiftKey && event.key.toLowerCase() === "o");
}

export function isAutoGetShortcut(
  event: Pick<
    KeyboardEvent,
    "key" | "shiftKey" | "ctrlKey" | "altKey" | "metaKey"
  >,
): boolean {
  return (
    event.ctrlKey &&
    !event.shiftKey &&
    !event.altKey &&
    !event.metaKey &&
    event.key.toLowerCase() === "g"
  );
}

export function nextTravelConnectionPosition(
  state: AppState,
  key: "<" | ">",
  current: Position,
): Position | undefined {
  const positions = [...state.cells.values()]
    .filter((cell) => {
      const visibility = state.cellVisibility.get(
        `${cell.position.x},${cell.position.y}`,
      );
      return (
        (visibility === "visible" || visibility === "remembered") &&
        state.contentGlyphs.get(cell.terrainId) === key
      );
    })
    .map((cell) => cell.position)
    .sort(
      (left, right) =>
        gridDistance(state.status?.player.position, left) -
          gridDistance(state.status?.player.position, right) ||
        left.y - right.y ||
        left.x - right.x,
    );
  if (positions.length === 0) return undefined;
  const currentIndex = positions.findIndex((position) => samePosition(position, current));
  return positions[(currentIndex + 1) % positions.length];
}

function searchDiscoveredSomething(current: GameSnapshot | GameUpdate): boolean {
  return "events" in current && current.events.some(event =>
    event.kind === "terrain.secret-discovered" || event.messageKey === "chest-trap-found",
  );
}

export function localTravelStopsAfterStep(
  before: GameSnapshot | GameUpdate,
  current: GameSnapshot | GameUpdate | undefined,
  destination: Position,
): boolean {
  return (
    !current ||
    current.mapScale !== "local" ||
    current.floorId !== before.floorId ||
    current.player.isDead ||
    current.player.pendingDuelist != null ||
    current.player.hp < before.player.hp ||
    searchDiscoveredSomething(current) ||
    current.player.statuses.some((status) => status.kindId === "rfb.status.confusion") ||
    current.entities.some((entity) => entity.faction === "hostile") ||
    (samePosition(current.player.position, before.player.position) && !doorOpenedAfterStep(before, current)) ||
    samePosition(current.player.position, destination)
  );
}

export function translatedLocalPosition(position: Position, translation: Position): Position {
  return {
    x: position.x + translation.x,
    y: position.y + translation.y,
  };
}

export function autoGetStopsAfterStep(
  before: GameSnapshot | GameUpdate,
  current: GameSnapshot | GameUpdate | undefined,
  target: AutoGetTargetDto,
): boolean {
  return (
    autoGetInterrupted(before, current) ||
    Boolean(
      current &&
        samePosition(current.player.position, before.player.position) &&
        !doorOpenedAfterStep(before, current) &&
        autoGetObjectExists(current, target.objectId),
    )
  );
}

function doorOpenedAfterStep(before: GameSnapshot | GameUpdate, current: GameSnapshot | GameUpdate): boolean {
  return current.revision > before.revision && "events" in current &&
    current.events.some(event => event.kind === "terrain.door-opened");
}

function autoGetInterrupted(
  before: GameSnapshot | GameUpdate,
  current: GameSnapshot | GameUpdate | undefined,
): boolean {
  return (
    !current ||
    current.mapScale !== "local" ||
    current.floorId !== before.floorId ||
    current.player.isDead ||
    current.player.pendingDuelist != null ||
    current.player.hp < before.player.hp ||
    searchDiscoveredSomething(current) ||
    current.player.inventoryUsedSlots >= current.player.inventorySlotCapacity ||
    current.player.statuses.some(
      (status) =>
        status.kindId === "rfb.status.confusion" ||
        status.kindId === "rfb.status.blindness",
    ) ||
    current.entities.some((entity) => entity.faction === "hostile") ||
    Boolean(current.mogaminator.pendingQuery)
  );
}

function autoGetObjectExists(
  state: GameSnapshot | GameUpdate,
  objectId: string,
): boolean {
  return (
    state.items.some((item) => item.id === objectId) ||
    state.goldPiles.some((pile) => pile.id === objectId)
  );
}

function isRunPrefix(key: string, preset: InputPreset): boolean {
  return (preset === "original" && key === ".") || (preset === "roguelike" && key === ",");
}

export function alterDirectionForKeyboardInput(
  event: Pick<KeyboardEvent, "key" | "code" | "ctrlKey" | "altKey" | "metaKey" | "shiftKey">,
  preset: InputPreset,
): Direction | undefined {
  if (!event.ctrlKey || event.altKey || event.metaKey || event.shiftKey) return undefined;
  // Both RFB presets support Windows navigation/numpad macros.
  return runDirectionForKeyboardInput({ key: event.key, code: event.code, shiftKey: true }, "original") ??
    directionForKeyboardInput({ code: event.code, key: event.key.toLowerCase() }, preset);
}

export function runDirectionForKeyboardInput(
  event: Pick<KeyboardEvent, "key" | "code" | "shiftKey">,
  preset: InputPreset,
): Direction | undefined {
  if (event.shiftKey) {
    const direction = NUMPAD_DIRECTIONS[event.code] ?? ({
      ArrowUp: "north", ArrowDown: "south", ArrowLeft: "west", ArrowRight: "east",
      Home: "north-west", End: "south-west", PageUp: "north-east", PageDown: "south-east",
    } as Record<string, Direction>)[event.code];
    if (direction) return direction;
  }
  if (preset === "roguelike" && /^[HJKLYUBN]$/.test(event.key)) return VI_DIRECTIONS[event.key.toLowerCase()];
  return undefined;
}

export function commandForKeyboardInput(
  event: Pick<KeyboardEvent, "key" | "code">,
  preset: InputPreset,
): GameCommand | undefined {
  if (event.key === "5" || event.code === "Numpad5" || event.key === (preset === "roguelike" ? "." : ",")) return { type: "stay" };
  const direction = directionForKeyboardInput(event, preset);
  return direction ? { type: "move", direction } : undefined;
}

export type ConnectionAction =
  | "enter-world-map"
  | "leave-world-map"
  | "enter-warrens"
  | "ascend"
  | "descend";

export function connectionActionForState(state: AppState): ConnectionAction | undefined {
  const status = state.status;
  if (!status) return undefined;
  if (status.mapScale === "world") return "leave-world-map";
  const terrainId = state.cellAt(status.player.position)?.terrainId;
  const glyph = terrainId ? state.contentGlyphs.get(terrainId) : undefined;
  if (glyph === "<") return "ascend";
  if (glyph === ">") {
    return state.worldId === "demo.world.middle-earth" &&
      status.floorId === "demo.floor.surface"
      ? "enter-warrens"
      : "descend";
  }
  const ambushThreatRemains =
    status.floorId === "core.floor.wilderness" &&
    status.entities.some(
      (entity) =>
        entity.faction === "hostile" &&
        (entity.id.includes(".ambush.") || entity.summon?.ownerId.includes(".ambush.")),
    );
  return state.worldId === "demo.world.middle-earth" &&
    !ambushThreatRemains &&
    (status.floorId === "demo.floor.surface" ||
      status.floorId === "core.floor.wilderness")
    ? "enter-world-map"
    : undefined;
}

export function directionForKeyboardInput(
  event: Pick<KeyboardEvent, "key" | "code">,
  preset: InputPreset,
): Direction | undefined {
  return NUMPAD_DIRECTIONS[event.code] ?? NUMPAD_DIRECTIONS[`Numpad${event.key}`] ??
    ({ ArrowUp: "north", ArrowDown: "south", ArrowLeft: "west", ArrowRight: "east" } as Record<string, Direction>)[event.key] ??
    (preset === "roguelike" ? VI_DIRECTIONS[event.key] : undefined);
}

function targetSpecForIntent(
  state: GameSnapshot | GameUpdate,
  intent: TargetingIntent,
): TargetSpecDto | null | undefined {
  if (intent.type === "look" || intent.type === "local-travel") return undefined;
  if (intent.type === "select-target") return { modes: ["position", "entity"], range: Math.max(state.width, state.height), requiresLineOfEffect: false };
  if (intent.type === "mutation-direction" || intent.type === "ability-direction") {
    return {
      modes: ["direction"],
      range: Math.max(state.width, state.height),
      requiresLineOfEffect: false,
    };
  }
  if (intent.type === "projectile") return state.player.projectileProfile?.targetSpec;
  if (intent.type === "throw") {
    return [...state.inventory, ...state.equipment].find(item => item.id === intent.itemId)?.throwTargetSpec;
  }
  if (intent.type === "item") {
    return [...state.inventory, ...state.equipment, ...(state.player.magicEater?.deviceCommands.flatMap(command => command.items) ?? [])].find(
      (item) => item.id === intent.itemId && item.usable,
    )?.useTargetSpec;
  }
  if (intent.type === "absorbed-device") {
    return state.player.magicEater?.slots.find(slot => slot.item?.id === intent.itemId && slot.item.usable)?.item?.useTargetSpec;
  }
  return (state.player.abilities ?? []).find(
    (ability) => ability.id === intent.abilityId && ability.canCast,
  )?.targetSpec;
}

function gridDistance(from: Position | undefined, to: Position): number {
  return from
    ? Math.max(Math.abs(to.x - from.x), Math.abs(to.y - from.y))
    : 0;
}

function targetDirectionKey(
  from: Position,
  to: Position,
): "n" | "ne" | "e" | "se" | "s" | "sw" | "w" | "nw" | "here" {
  const x = Math.sign(to.x - from.x);
  const y = Math.sign(to.y - from.y);
  if (x === 0 && y === 0) return "here";
  if (x === 0) return y < 0 ? "n" : "s";
  if (y === 0) return x < 0 ? "w" : "e";
  if (x > 0) return y < 0 ? "ne" : "se";
  return y < 0 ? "nw" : "sw";
}

function samePosition(left: Position, right: Position): boolean {
  return left.x === right.x && left.y === right.y;
}

function terrainModeMessageKey(mode: TerrainInteractionMode): MessageKey {
  return mode === "spike-door" ? "message-door-mode-spike" : mode === "alter" ? "message-terrain-mode-alter" : mode === "open-door"
    ? "message-door-mode-open"
    : mode === "close-door"
      ? "message-door-mode-close"
      : mode === "bash-door"
        ? "message-door-mode-bash"
        : mode === "disarm-trap"
          ? "message-trap-mode-disarm"
          : "message-terrain-mode-dig";
}

function isTextInput(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLSelectElement ||
    (target instanceof HTMLElement && target.isContentEditable)
  );
}

const NUMPAD_DIRECTIONS: Partial<Record<string, Direction>> = {
  Numpad8: "north",
  Numpad9: "north-east",
  Numpad6: "east",
  Numpad3: "south-east",
  Numpad2: "south",
  Numpad1: "south-west",
  Numpad4: "west",
  Numpad7: "north-west",
};

const VI_DIRECTIONS: Partial<Record<string, Direction>> = {
  k: "north",
  u: "north-east",
  l: "east",
  n: "south-east",
  j: "south",
  b: "south-west",
  h: "west",
  y: "north-west",
};
