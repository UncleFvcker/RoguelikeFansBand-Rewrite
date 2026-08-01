// SPDX-License-Identifier: MPL-2.0

import { MAP_CELL_SIZE } from "./camera.ts";
import type { AppDom } from "./app-dom";
import type { AppState, TargetingIntent } from "./app-state";
import type { Localization, MessageKey } from "./localization";
import type {
  AbilityDto,
  Direction,
  GameCommand,
  GameSnapshot,
  GameUpdate,
  TargetSpecDto,
} from "./protocol";
import {
  beginTargeting,
  moveTargetCursor,
  targetSelectionAtCursor,
} from "./targeting.ts";
import {
  terrainInteractionCommand,
  terrainInteractionForDirection,
  terrainInteractionsForMode,
  terrainInteractionModeForKey,
  terrainSearchCommandForKey,
  type TerrainInteractionMode,
} from "./terrain-interaction.ts";

export type InputPreset = "numpad" | "vi" | "wasd";

type InputDom = Pick<
  AppDom,
  | "mapHost"
  | "targetCursor"
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
  readonly #onLookOrTargeting: (interaction: "look" | "targeting") => void;
  readonly #announce: (
    key: MessageKey,
    args: Record<string, string | number> | undefined,
    kind: string,
  ) => void;
  #installed = false;

  constructor(options: {
    state: AppState;
    dom: InputDom;
    localization: Localization;
    window: Window;
    getInputPreset: () => InputPreset;
    getZoom: () => number;
    dispatch: (command: GameCommand) => Promise<void>;
    describeLook: (position: { readonly x: number; readonly y: number }) => string;
    onLookOrTargeting: (interaction: "look" | "targeting") => void;
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
    this.#onLookOrTargeting = options.onLookOrTargeting;
    this.#announce = options.announce;
  }

  install(): void {
    if (this.#installed) return;
    this.#installed = true;
    this.#window.addEventListener("keydown", this.#handleKeydown);
    this.#window.addEventListener("resize", this.#handleResize);
    this.#dom.targetModeToggle.addEventListener("click", this.#handleTargetToggle);
    this.#dom.lookModeToggle.addEventListener("click", this.#handleLookToggle);
  }

  dispose(): void {
    if (!this.#installed) return;
    this.#installed = false;
    this.#window.removeEventListener("keydown", this.#handleKeydown);
    this.#window.removeEventListener("resize", this.#handleResize);
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
    this.render();
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
    this.#state.targeting = next;
    this.#state.targetingIntent = intent;
    this.#announce("message-target-mode-started", undefined, "system");
    this.#onLookOrTargeting("targeting");
    this.render();
  }

  cancelTargeting(announce = true): void {
    if (!this.#state.targeting) return;
    this.#state.targeting = undefined;
    this.#state.targetingIntent = undefined;
    if (announce) {
      this.#announce("message-target-mode-cancelled", undefined, "system");
    }
    this.render();
  }

  reconcileStatus(state: GameSnapshot | GameUpdate): void {
    if (
      this.#state.targeting &&
      (this.#state.targeting.origin.x !== state.player.position.x ||
        this.#state.targeting.origin.y !== state.player.position.y ||
        !this.#state.targetingIntent ||
        (this.#state.targetingIntent.type !== "look" &&
          !targetSpecForIntent(state, this.#state.targetingIntent)))
    ) {
      this.cancelTargeting(false);
    }
  }

  render(): void {
    const looking = this.#state.targetingIntent?.type === "look";
    const targeting = Boolean(this.#state.targeting && !looking);
    const available = Boolean(
      this.#state.status &&
        beginTargeting(
          this.#state.status.player.position,
          this.#state.status.player.projectileProfile?.targetSpec,
        ),
    );
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
    this.#dom.targetModeStatus.textContent = looking
      ? this.#localization.format("look-status-active", {
          x: cursor.x,
          y: cursor.y,
          contents: this.#describeLook(cursor),
        })
      : this.#localization.format("target-status-active", {
          x: cursor.x,
          y: cursor.y,
          range: spec.range,
        });
  }

  readonly #handleTargetToggle = (): void => {
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

  readonly #handleResize = (): void => {
    this.#window.requestAnimationFrame(() => this.render());
  };

  readonly #handleKeydown = (event: KeyboardEvent): void => {
    if (this.#state.busy || this.#state.commandBlocked || isTextInput(event.target)) return;
    if (this.#state.targeting) {
      this.#handleTargetingKey(event);
      return;
    }
    if (this.#state.terrainInteractionMode) {
      this.#handleTerrainDirection(event);
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
    if (event.key.toLowerCase() === "x") {
      event.preventDefault();
      this.startLookMode();
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
      this.cancelTargeting();
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      if (this.#state.targetingIntent?.type === "look") this.cancelTargeting(false);
      else void this.#confirmTargeting();
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
    if (this.#state.targetingIntent?.type === "look") this.#onLookOrTargeting("look");
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
      intent.type === "ability"
        ? { type: "cast-ability", abilityId: intent.abilityId, target }
        : intent.type === "item"
          ? { type: "use-item", itemId: intent.itemId, target }
          : { type: "fire-target", target },
    );
  }
}

export function commandForKeyboardInput(
  event: Pick<KeyboardEvent, "key" | "code">,
  preset: InputPreset,
): GameCommand | undefined {
  const key = event.key.toLowerCase();
  if (key === "g") return { type: "pick-up" };
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
  if (intent.type === "look") return undefined;
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
