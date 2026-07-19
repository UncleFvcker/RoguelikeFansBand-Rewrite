// SPDX-License-Identifier: MPL-2.0

import { FluentBundle, FluentResource, type FluentVariable } from "@fluent/bundle";

export const SUPPORTED_LOCALES = ["en-US", "zh-CN"] as const;
export type SupportedLocale = (typeof SUPPORTED_LOCALES)[number];

export const MESSAGE_KEYS = [
  "app-title",
  "app-eyebrow",
  "app-heading",
  "action-export-replay",
  "action-export-save",
  "action-load-save",
  "map-aria-label",
  "panel-map-title",
  "connection-starting",
  "connection-ready",
  "connection-error",
  "settings-input-label",
  "settings-tileset-label",
  "settings-camera-label",
  "settings-zoom-label",
  "settings-language-label",
  "language-name-en-us",
  "language-name-zh-cn",
  "input-preset-numpad",
  "input-preset-vi",
  "input-preset-wasd",
  "tileset-ascii",
  "tileset-image",
  "camera-full-map",
  "camera-player-centered",
  "zoom-75",
  "zoom-100",
  "zoom-125",
  "zoom-150",
  "zoom-200",
  "panel-status-title",
  "status-turn",
  "status-health",
  "status-health-value",
  "status-health-value-bonus",
  "status-attack",
  "status-defense",
  "status-stat-value",
  "status-stat-value-bonus",
  "status-effects",
  "status-effects-none",
  "status-effect-entry",
  "status-position",
  "status-hash",
  "panel-inventory-title",
  "inventory-stack-count",
  "inventory-empty",
  "inventory-quantity",
  "inventory-selected-count",
  "inventory-equippable",
  "inventory-drop-quantity-label",
  "item-modifier-max-hp",
  "item-modifier-attack",
  "item-modifier-defense",
  "action-inventory-equip",
  "action-inventory-drop",
  "panel-equipment-title",
  "equipment-empty",
  "action-equipment-unequip",
  "equipment-slot-charm",
  "equipment-slot-unknown",
  "panel-native-save-title",
  "action-native-save-refresh",
  "native-save-name-label",
  "native-save-name-placeholder",
  "action-native-save-create",
  "action-native-save-load",
  "action-native-save-overwrite",
  "action-native-save-delete",
  "confirm-native-save-delete",
  "native-save-empty",
  "native-save-status-ready",
  "native-save-status-recoverable",
  "native-save-status-corrupt",
  "native-save-meta",
  "native-save-meta-unavailable",
  "native-save-date-unknown",
  "native-save-location-unknown",
  "panel-message-title",
  "action-clear-messages",
  "action-target-start",
  "action-target-cancel",
  "target-status-ready",
  "target-status-unavailable",
  "target-status-active",
  "controls-numpad",
  "controls-vi",
  "controls-wasd",
  "message-core-started",
  "message-input-preset-changed",
  "message-target-mode-started",
  "message-target-mode-cancelled",
  "message-target-mode-unavailable",
  "message-target-selection-invalid",
  "message-save-exported",
  "message-replay-exported",
  "message-crash-diagnostic-created",
  "message-save-loaded",
  "message-native-save-created",
  "message-native-save-overwritten",
  "message-native-save-loaded",
  "message-native-save-backup-loaded",
  "message-native-save-deleted",
  "message-native-save-failed",
  "native-save-error-name-invalid",
  "native-save-error-not-found",
  "native-save-error-corrupt",
  "native-save-error-unavailable",
  "message-tileset-load-failed",
  "message-error",
  "message-tileset-loaded",
  "message-tileset-image-too-small",
  "message-tileset-image-load-failed",
  "message-game-wait",
  "message-move-blocked",
  "message-combat-hit",
  "message-combat-hit-resisted",
  "message-combat-hit-amplified",
  "message-combat-hit-immune",
  "message-combat-slay",
  "message-combat-player-miss",
  "message-combat-monster-miss",
  "message-combat-monster-hit",
  "message-combat-monster-hit-resisted",
  "message-combat-monster-hit-amplified",
  "message-combat-monster-hit-immune",
  "message-combat-player-death",
  "message-projectile-unavailable",
  "message-projectile-ammo-unavailable",
  "message-projectile-target-unavailable",
  "message-projectile-landed",
  "message-projectile-miss",
  "message-projectile-hit",
  "message-projectile-slay",
  "message-projectile-ammo-recovered",
  "message-projectile-ammo-broken",
  "message-status-player-damage",
  "message-status-entity-damage",
  "message-status-player-expired",
  "message-status-entity-expired",
  "message-status-player-death",
  "message-status-entity-death",
  "message-status-fear-blocked",
  "message-item-pickup-success",
  "message-item-pickup-none",
  "message-item-equip-success",
  "message-item-equip-swap",
  "message-item-equip-unavailable",
  "message-item-unequip-success",
  "message-item-unequip-none",
  "message-item-drop-success",
  "message-item-drop-none",
  "message-item-thrown",
  "message-throw-miss",
  "message-throw-hit",
  "message-throw-slay",
  "message-item-throw-unavailable",
  "message-unknown-event",
  "item-demo-luminous-shard-name",
  "item-demo-echo-charm-name",
  "item-demo-echo-blade-name",
  "item-demo-echo-blade-description",
  "item-demo-resonance-sling-name",
  "item-demo-resonance-sling-description",
  "item-demo-resonance-pellet-name",
  "item-demo-resonance-pellet-description",
  "item-unknown-name",
  "actor-demo-ember-mote-name",
  "actor-demo-acid-seep-name",
  "actor-demo-storm-spark-name",
  "actor-demo-frost-wisp-name",
  "actor-demo-venom-spore-name",
  "actor-demo-echo-hound-name",
  "actor-demo-echo-hound-description",
  "actor-unknown-name",
  "status-poison-name",
  "status-bleeding-name",
  "status-haste-name",
  "status-slow-name",
  "status-stun-name",
  "status-fear-name",
  "status-unknown-name",
  "damage-type-physical-name",
  "damage-type-acid-name",
  "damage-type-electricity-name",
  "damage-type-fire-name",
  "damage-type-cold-name",
  "damage-type-poison-name",
  "world-demo-original-lab-name",
] as const;

export type MessageKey = (typeof MESSAGE_KEYS)[number];
export type LocalizationArgs = Record<string, FluentVariable>;
export type LocaleSources = Record<SupportedLocale, readonly string[]>;

export class Localization {
  readonly #bundles: Record<SupportedLocale, FluentBundle>;
  #locale: SupportedLocale;

  constructor(locale: SupportedLocale, sources: LocaleSources) {
    this.#locale = locale;
    this.#bundles = {
      "en-US": createBundle("en-US", sources["en-US"]),
      "zh-CN": createBundle("zh-CN", sources["zh-CN"]),
    };
  }

  get locale(): SupportedLocale {
    return this.#locale;
  }

  setLocale(locale: SupportedLocale): void {
    this.#locale = locale;
  }

  hasMessage(locale: SupportedLocale, key: MessageKey): boolean {
    return this.#bundles[locale].hasMessage(key);
  }

  format(key: MessageKey, args?: LocalizationArgs): string {
    return this.#formatFrom(this.#locale, key, args) ?? this.#formatFrom("en-US", key, args) ?? `[${key}]`;
  }

  localizeDocument(root: ParentNode = document): void {
    for (const element of root.querySelectorAll<HTMLElement>("[data-l10n-id]")) {
      const key = element.dataset.l10nId as MessageKey | undefined;
      if (key) element.textContent = this.format(key);
    }
    for (const element of root.querySelectorAll<HTMLElement>("[data-l10n-aria-label]")) {
      const key = element.dataset.l10nAriaLabel as MessageKey | undefined;
      if (key) element.setAttribute("aria-label", this.format(key));
    }
    document.documentElement.lang = this.#locale;
  }

  #formatFrom(locale: SupportedLocale, key: MessageKey, args?: LocalizationArgs): string | undefined {
    const bundle = this.#bundles[locale];
    const message = bundle.getMessage(key);
    if (!message?.value) return undefined;
    const errors: Error[] = [];
    const value = bundle.formatPattern(message.value, args, errors);
    if (errors.length > 0) {
      console.warn(`Fluent formatting error for ${locale}/${key}`, errors);
    }
    return value;
  }
}

export function isSupportedLocale(value: string | null): value is SupportedLocale {
  return SUPPORTED_LOCALES.some((locale) => locale === value);
}

function createBundle(locale: SupportedLocale, sources: readonly string[]): FluentBundle {
  const bundle = new FluentBundle(locale, { useIsolating: false });
  for (const source of sources) {
    const errors = bundle.addResource(new FluentResource(source));
    if (errors.length > 0) {
      throw new Error(`Invalid Fluent resource for ${locale}: ${errors.join("; ")}`);
    }
  }
  return bundle;
}
