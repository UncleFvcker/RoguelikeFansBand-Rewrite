// SPDX-License-Identifier: MPL-2.0

import { MAP_CELL_SIZE } from "./camera.ts";
import type { AppDom } from "./app-dom";
import type { AppState, TargetingIntent } from "./app-state";
import type { Localization, MessageKey } from "./localization";
import type {
  AbilityDto,
  AutoGetTargetDto,
  Direction,
  GameCommand,
  GameSnapshot,
  GameUpdate,
  Position,
  TargetSpecDto,
} from "./protocol";
import {
  beginTargeting,
  moveTargetCursor,
  targetSelectionAtCursor,
  translateTargetingState,
} from "./targeting.ts";
import {
  terrainInteractionCommand,
  terrainInteractionForDirection,
  terrainInteractionsForMode,
  terrainInteractionModeForKey,
  terrainSearchCommandForKey,
  type TerrainInteractionMode,
} from "./terrain-interaction.ts";
import { REST_UNTIL_RECOVERED_TURNS } from "./rest.ts";

export type InputPreset = "numpad" | "vi" | "wasd";

type InputDom = Pick<
  AppDom,
  | "mapHost"
  | "targetCursor"
  | "traverseStairs"
  | "targetModeToggle"
  | "lookModeToggle"
  | "targetModeStatus"
>;

export class InputController {
  readonly #state: AppState;
  readonly #dom: InputDom;
  readonly #localization: Localization;
  readonly #window: Window;
  readonly #getInputPreset: () => InputPreset;
  readonly #getZoom: () => number;
  readonly #dispatch: (command: GameCommand) => Promise<void>;
  readonly #describeLook: (position: { readonly x: number; readonly y: number }) => string;
  readonly #openObjectList: () => void;
  readonly #openMogaminator: () => void;
  readonly #onLookOrTargeting: (interaction: "look" | "targeting") => void;
  readonly #onLookFocusChange: (position: Position | undefined) => void;
  readonly #announce: (
    key: MessageKey,
    args: Record<string, string | number> | undefined,
    kind: string,
  ) => void;
  #installed = false;
  #ridingDirection = false;
  #worldTravelDestination: Position | undefined;
  #localTravelDestination: Position | undefined;
  #localTravelFloorId: string | undefined;

  constructor(options: {
    state: AppState;
    dom: InputDom;
    localization: Localization;
    window: Window;
    getInputPreset: () => InputPreset;
    getZoom: () => number;
    dispatch: (command: GameCommand) => Promise<void>;
    describeLook: (position: { readonly x: number; readonly y: number }) => string;
    openObjectList: () => void;
    openMogaminator: () => void;
    onLookOrTargeting: (interaction: "look" | "targeting") => void;
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
    this.#describeLook = options.describeLook;
    this.#openObjectList = options.openObjectList;
    this.#openMogaminator = options.openMogaminator;
    this.#onLookOrTargeting = options.onLookOrTargeting;
    this.#onLookFocusChange = options.onLookFocusChange;
    this.#announce = options.announce;
  }

  install(): void {
    if (this.#installed) return;
    this.#installed = true;
    this.#window.addEventListener("keydown", this.#handleKeydown);
    this.#window.addEventListener("resize", this.#handleResize);
    this.#dom.traverseStairs.addEventListener("click", this.#handleTraverseStairs);
    this.#dom.targetModeToggle.addEventListener("click", this.#handleTargetToggle);
    this.#dom.lookModeToggle.addEventListener("click", this.#handleLookToggle);
  }

  dispose(): void {
    if (!this.#installed) return;
    this.#installed = false;
    this.#window.removeEventListener("keydown", this.#handleKeydown);
    this.#window.removeEventListener("resize", this.#handleResize);
    this.#dom.traverseStairs.removeEventListener("click", this.#handleTraverseStairs);
    this.#dom.targetModeToggle.removeEventListener("click", this.#handleTargetToggle);
    this.#dom.lookModeToggle.removeEventListener("click", this.#handleLookToggle);
  }

  startProjectileTargeting(): void {
    if (this.#state.busy || this.#state.playerDead || !this.#state.status) return;
    this.startTargetingWithSpec(
      this.#state.status.player.projectileProfile?.targetSpec,
      { type: "projectile" },
    );
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
    this.#onLookOrTargeting("look");
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
    this.#localTravelDestination = undefined;
    this.#localTravelFloorId = undefined;
  }

  async autoGet(): Promise<void> {
    const initial = this.#state.status;
    if (!initial || initial.mapScale !== "local" || this.#state.commandBlocked) return;
    await this.#dispatch({ type: "pick-up" });
    let current = this.#state.status;
    if (autoGetInterrupted(initial, current)) return;

    for (;;) {
      const target = current?.mogaminator.autoGetTarget;
      if (!current || !target) return;
      for (;;) {
        const before = current;
        await this.#dispatch({ type: "auto-get", objectId: target.objectId });
        current = this.#state.status;
        if (autoGetStopsAfterStep(before, current, target)) return;
        if (!current || !autoGetObjectExists(current, target.objectId)) break;
      }
    }
  }

  startAbilityTargeting(ability: AbilityDto): void {
    if (
      this.#state.busy ||
      this.#state.playerDead ||
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
    if (!this.#state.status) return;
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
    this.#state.targeting = next;
    this.#state.targetingIntent = intent;
    this.#announce("message-target-mode-started", undefined, "system");
    this.#onLookOrTargeting("targeting");
    this.render();
  }

  cancelTargeting(announce = true): void {
    if (!this.#state.targeting) return;
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
  }

  reconcileStatus(state: GameSnapshot | GameUpdate): void {
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
        this.#onLookOrTargeting("targeting");
      }
    } else if (!pendingDirection && this.#state.targetingIntent?.type === "mutation-direction") {
      this.cancelTargeting(false);
    }
  }

  render(): void {
    const looking = this.#state.targetingIntent?.type === "look";
    const localTravel = this.#state.targetingIntent?.type === "local-travel";
    const targeting = Boolean(this.#state.targeting && !looking && !localTravel);
    const available = Boolean(
      !this.#state.worldMap &&
        this.#state.status &&
        beginTargeting(
          this.#state.status.player.position,
          this.#state.status.player.projectileProfile?.targetSpec,
        ),
    );
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
      localTravel ||
      (!targeting && !available);
    this.#dom.lookModeToggle.textContent = this.#localization.format(
      looking ? "action-look-cancel" : "action-look-start",
    );
    this.#dom.lookModeToggle.setAttribute("aria-pressed", looking ? "true" : "false");
    this.#dom.lookModeToggle.disabled = this.#state.busy || this.#state.commandBlocked;
    this.#dom.mapHost.dataset.targeting = this.#state.targeting ? "true" : "false";
    this.#dom.mapHost.dataset.targetingAction = this.#state.targetingIntent?.type ?? "none";
    this.#dom.targetCursor.hidden = !this.#state.targeting;
    if (!this.#state.targeting) {
      this.#dom.targetModeStatus.textContent = this.#localization.format(
        available ? "target-status-ready" : "target-status-unavailable",
      );
      delete this.#dom.mapHost.dataset.targetX;
      delete this.#dom.mapHost.dataset.targetY;
      return;
    }

    const { cursor, spec } = this.#state.targeting;
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
        : this.#localization.format("target-status-active", {
            x: cursor.x,
            y: cursor.y,
            range: spec.range,
          });
  }

  readonly #handleTargetToggle = (): void => {
    if (this.#state.targetingIntent?.type === "mutation-direction") return;
    if (this.#state.targeting && this.#state.targetingIntent?.type !== "look") {
      this.cancelTargeting();
    } else {
      this.cancelTargeting(false);
      this.startProjectileTargeting();
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

  readonly #handleResize = (): void => {
    this.#window.requestAnimationFrame(() => this.render());
  };

  readonly #handleKeydown = (event: KeyboardEvent): void => {
    if (
      this.#state.busy ||
      this.#dom.mapHost.ownerDocument.querySelector("dialog[open]") ||
      isTextInput(event.target)
    ) return;
    if (this.#state.targeting) {
      this.#handleTargetingKey(event);
      return;
    }
    if (this.#state.commandBlocked) return;
    if (this.#state.terrainInteractionMode) {
      this.#handleTerrainDirection(event);
      return;
    }
    if (this.#ridingDirection) {
      this.#handleRidingDirection(event);
      return;
    }

    const key = event.key.toLowerCase();
    if (key === "x") {
      event.preventDefault();
      this.startLookMode();
      return;
    }
    if (event.key === "_") {
      event.preventDefault();
      this.#openMogaminator();
      return;
    }
    if (isAutoGetShortcut(event)) {
      event.preventDefault();
      if (!this.#state.worldMap) void this.autoGet();
      return;
    }
    if (this.#state.worldMap) {
      const travelDestination =
        this.#worldTravelDestination ?? this.#state.status?.worldTravelDestination;
      if (event.key === "J" && travelDestination) {
        event.preventDefault();
        void this.#travelWorldTo(travelDestination);
        return;
      }
      const direction = directionForKeyboardInput(event, this.#getInputPreset());
      if (direction) void this.#dispatch({ type: "move", direction });
      else if (key === ">") void this.#dispatch({ type: "leave-world-map" });
      event.preventDefault();
      return;
    }
    if (event.key === "`") {
      event.preventDefault();
      this.startLocalTravelSelection();
      return;
    }
    if (event.key === "J" && this.#localTravelDestination) {
      event.preventDefault();
      void this.travelLocalTo(this.#localTravelDestination);
      return;
    }
    if (isObjectListShortcut(event)) {
      event.preventDefault();
      this.#openObjectList();
      return;
    }
    if (key === "<" && connectionActionForState(this.#state) === "enter-world-map") {
      event.preventDefault();
      void this.#enterWorldMap();
      return;
    }

    const nextTerrainInteractionMode = terrainInteractionModeForKey(event.key);
    if (nextTerrainInteractionMode) {
      event.preventDefault();
      this.#startTerrainInteraction(nextTerrainInteractionMode);
      return;
    }
    const searchCommand = terrainSearchCommandForKey(event.key);
    if (searchCommand) {
      event.preventDefault();
      void this.#dispatch(searchCommand);
      return;
    }
    if (event.key.toLowerCase() === "f") {
      event.preventDefault();
      this.startProjectileTargeting();
      return;
    }
    if (event.key.toLowerCase() === "v") {
      event.preventDefault();
      this.#ridingDirection = true;
      this.#announce("message-riding-mode-started", undefined, "system");
      return;
    }
    const command = commandForKeyboardInput(event, this.#getInputPreset());
    if (command) {
      event.preventDefault();
      void this.#dispatch(command);
    }
  };

  #handleTargetingKey(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      if (this.#state.targetingIntent?.type === "mutation-direction") return;
      this.cancelTargeting();
      return;
    }
    const localTravel = this.#state.targetingIntent?.type === "local-travel";
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
    this.#state.targeting = moveTargetCursor(
      this.#state.targeting,
      direction,
      this.#state.mapWidth,
      this.#state.mapHeight,
    );
    if (
      this.#state.targetingIntent?.type === "look" ||
      this.#state.targetingIntent?.type === "local-travel"
    ) {
      this.#onLookOrTargeting("look");
      this.#onLookFocusChange(this.#state.targeting.cursor);
    }
    this.render();
  }

  #handleTerrainDirection(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      this.#state.terrainInteractionMode = undefined;
      this.#announce("message-door-mode-cancelled", undefined, "system");
      return;
    }
    const direction = directionForKeyboardInput(event, this.#getInputPreset());
    if (!direction || !this.#state.terrainInteractionMode) return;
    event.preventDefault();
    const mode = this.#state.terrainInteractionMode;
    this.#state.terrainInteractionMode = undefined;
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
    void this.#dispatch(terrainInteractionCommand(mode, direction));
  }

  #handleRidingDirection(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
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
    if (
      !this.#state.status ||
      terrainInteractionsForMode(this.#state.status.terrainInteractions, mode).length === 0
    ) {
      this.#announce("message-terrain-interaction-mode-unavailable", undefined, "system");
      return;
    }
    this.#state.terrainInteractionMode = mode;
    this.#announce(terrainModeMessageKey(mode), undefined, "system");
  }

  async #confirmTargeting(): Promise<void> {
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
    const target = targetSelectionAtCursor(state, status.entities);
    if (!target) {
      this.#announce("message-target-selection-invalid", undefined, "system");
      return;
    }
    this.cancelTargeting(false);
    await this.#dispatch(
      intent.type === "mutation-direction" && target.type === "direction"
        ? { type: "resolve-mutation-direction", direction: target.direction }
        : intent.type === "ability"
        ? { type: "cast-ability", abilityId: intent.abilityId, target }
        : intent.type === "item"
          ? { type: "use-item", itemId: intent.itemId, target }
          : { type: "fire-target", target },
    );
  }

  async #enterWorldMap(): Promise<void> {
    const status = this.#state.status;
    if (!status) return;
    const leavePets = status.entities.some(
      (entity) =>
        entity.id !== status.player.ridingActorId &&
        (entity.controllerId === status.player.id || entity.summon?.ownerId === status.player.id),
    );
    if (
      leavePets &&
      !this.#window.confirm(this.#localization.format("confirm-world-map-leave-pets"))
    ) {
      return;
    }
    const cancelRecall = status.player.recall?.remainingTurns != null;
    if (
      cancelRecall &&
      !this.#window.confirm(this.#localization.format("confirm-world-map-cancel-recall"))
    ) {
      return;
    }
    await this.#dispatch({ type: "enter-world-map", leavePets, cancelRecall });
  }

  async travelLocalTo(destination: Position): Promise<void> {
    const initial = this.#state.status;
    if (!initial || initial.mapScale !== "local" || this.#state.commandBlocked) return;
    this.#localTravelDestination = destination;
    this.#localTravelFloorId = initial.floorId;
    this.#announce("message-local-travel-started", undefined, "system");
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
      await this.#dispatch({ type: "travel-local", destination: activeDestination });
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
      await this.#dispatch({ type: "travel-world", destination });
      const current = this.#state.status;
      if (
        !current ||
        current.mapScale !== "world" ||
        (current.player.position.x === previous.x && current.player.position.y === previous.y)
      ) {
        return;
      }
    }
  }
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
    current.player.hp < before.player.hp ||
    current.player.statuses.some((status) => status.kindId === "rfb.status.confusion") ||
    current.entities.some((entity) => entity.faction === "hostile") ||
    samePosition(current.player.position, before.player.position) ||
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
        autoGetObjectExists(current, target.objectId),
    )
  );
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
    current.player.hp < before.player.hp ||
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

export function commandForKeyboardInput(
  event: Pick<KeyboardEvent, "key" | "code">,
  preset: InputPreset,
): GameCommand | undefined {
  const key = event.key.toLowerCase();
  if (key === "g") return { type: "pick-up" };
  if (key === "r") return { type: "rest", turns: REST_UNTIL_RECOVERED_TURNS };
  if (key === ">" || key === "<") return { type: "traverse-stairs" };
  const direction = directionForKeyboardInput(event, preset);
  if (preset === "numpad") {
    if (event.code === "Numpad5") return { type: "wait" };
    return direction ? { type: "move", direction } : undefined;
  }
  if (preset === "vi") {
    if (key === ".") return { type: "wait" };
    return direction ? { type: "move", direction } : undefined;
  }
  if (key === " ") return { type: "wait" };
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
  if (preset === "numpad") return NUMPAD_DIRECTIONS[event.code];
  const key = event.key.toLowerCase();
  return preset === "vi" ? VI_DIRECTIONS[key] : WASD_DIRECTIONS[key];
}

function targetSpecForIntent(
  state: GameSnapshot | GameUpdate,
  intent: TargetingIntent,
): TargetSpecDto | null | undefined {
  if (intent.type === "look" || intent.type === "local-travel") return undefined;
  if (intent.type === "mutation-direction") {
    return {
      modes: ["direction"],
      range: Math.max(state.width, state.height),
      requiresLineOfEffect: false,
    };
  }
  if (intent.type === "projectile") return state.player.projectileProfile?.targetSpec;
  if (intent.type === "item") {
    return state.inventory.find(
      (item) => item.id === intent.itemId && item.usable,
    )?.useTargetSpec;
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

function samePosition(left: Position, right: Position): boolean {
  return left.x === right.x && left.y === right.y;
}

function terrainModeMessageKey(mode: TerrainInteractionMode): MessageKey {
  return mode === "open-door"
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
    target instanceof HTMLSelectElement
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

const WASD_DIRECTIONS: Partial<Record<string, Direction>> = {
  w: "north",
  e: "north-east",
  d: "east",
  c: "south-east",
  s: "south",
  z: "south-west",
  a: "west",
  q: "north-west",
};
