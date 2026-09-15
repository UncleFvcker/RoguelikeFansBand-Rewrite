// SPDX-License-Identifier: MPL-2.0
// Presentation only: never injected into Core behavior, saves, or replays.
export const DEFAULT_HUD_DISPLAY = {
  showCharacterInfo: true,
  showSidebar: true,
  showFooter: true,
  showNearby: true,
  showMessages: true,
  showMapActions: true,
  showCombatSummary: true,
  showDungeonInfo: true,
  showShortcutBar: true,
};
export const HUD_DISPLAY_FIELDS = Object.keys(DEFAULT_HUD_DISPLAY) as (keyof typeof DEFAULT_HUD_DISPLAY)[];
export const DEFAULT_DISPLAY = {
  ...DEFAULT_HUD_DISPLAY,
  highlightPlayer: false,
  targetPath: false,
  unsafeGrids: false,
  alertTrapDetect: false,
  effectiveSpeed: false,
  decimalStats: true,
  experienceNeeded: false,
  showOrigins: true,
  describeSlots: true,
  showWeights: true,
  showDiscounts: true,
  showItemIcons: true,
  monsterDistance: false,
  listStairs: false,
  alertPoison: true,
  hpWarningPercent: 50,
  manaWarningPercent: 0,
};
export type DisplayPreferences = typeof DEFAULT_DISPLAY;
export const DISPLAY_FIELDS = Object.keys(DEFAULT_DISPLAY) as (keyof DisplayPreferences)[];
export function validDisplay(value: unknown): value is DisplayPreferences {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const d = value as Record<string, unknown>;
  return Object.keys(d).sort().join() === [...DISPLAY_FIELDS].sort().join() && DISPLAY_FIELDS.every(key =>
    typeof DEFAULT_DISPLAY[key] === "boolean" ? typeof d[key] === "boolean" :
      typeof d[key] === "number" && Number.isInteger(d[key]) && d[key] >= 0 && d[key] <= 90 && d[key] % 10 === 0);
}
