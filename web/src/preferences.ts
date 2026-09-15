// SPDX-License-Identifier: MPL-2.0
import type { SupportedLocale } from "./localization";
import type { InputPreset } from "./input-controller";
import type { CameraMode, ZoomLevel } from "./camera.ts";
import { parseBindings, type KeyBinding } from "./command-recording.ts";
import type { BehaviorPreferencesDto, MogaminatorPreferencesDto, OperationOptionsDto, TravelOptionsDto } from "./protocol";

import { DEFAULT_DISPLAY, validDisplay, type DisplayPreferences } from "./display-preferences.ts";

import { defaultVisuals, validVisuals, type VisualPreferences } from "./visual-preferences.ts";

export interface Preferences {
  visuals: VisualPreferences;
  display: DisplayPreferences;
  formatVersion: 5;
  locale: SupportedLocale;
  inputPreset: InputPreset;
  tilesetPreset: "ascii" | "image";
  cameraMode: CameraMode;
  zoom: ZoomLevel;
  keyBindings: KeyBinding[];
  travel: TravelOptionsDto;
  operations: OperationOptionsDto;
  mogaminator: MogaminatorPreferencesDto;
}
export interface PreferenceSnapshot { revision: number; preferences: Preferences }
export interface PreferenceStorage {
  defaults(): Promise<Preferences>;
  load(): Promise<PreferenceSnapshot | null>;
  save(preferences: Preferences, expectedRevision: number | null): Promise<PreferenceSnapshot>;
}
export function defaultPreferences(): Preferences {
  // Inert bootstrap/test values. Runtime defaults, including rule sources, come from Core.
  return { visuals: defaultVisuals(), display: { ...DEFAULT_DISPLAY }, formatVersion: 5, locale: "zh-CN", inputPreset: "original", tilesetPreset: "ascii", cameraMode: "player-centered", zoom: 1, keyBindings: [],
    travel: { alwaysPickup: false, autoDetectTraps: false, autoMapArea: false, disturbTrapDetect: true },
    operations: { runStops: { stairs: true, openDoors: false, knownTreasure: false }, cutCorners: false,
      travelIgnoreItems: true, defaultTarget: "manual", targetPets: false, easyOpen: true, easyDisarm: true, autoRepeat: true },
    mogaminator: { enabled: false, leaveDestroyedItems: false, autoGetMode: "off", zhCnSource: "", enUsSource: "" } };
}
export function behaviorPreferences(p: Preferences): BehaviorPreferencesDto { return { locale: p.locale, travel: p.travel, operations: p.operations, mogaminator: p.mogaminator }; }
export function parsePreferences(text: string): Preferences {
  const p = JSON.parse(text);
  // Additive visual preference: retain existing version-5 settings and explicit overrides.
  if (p?.visuals && typeof p.visuals === "object" && !Array.isArray(p.visuals) && !Object.hasOwn(p.visuals, "uniqueEffect")) {
    p.visuals.uniqueEffect = "flowing";
  }
  if (!p || typeof p !== "object" || Object.keys(p).sort().join() !== "cameraMode,display,formatVersion,inputPreset,keyBindings,locale,mogaminator,operations,tilesetPreset,travel,visuals,zoom" ||
      p.formatVersion !== 5 || !validVisuals(p.visuals) || !validDisplay(p.display) || !["zh-CN", "en-US"].includes(p.locale) ||
      !["original", "roguelike"].includes(p.inputPreset) ||
      !["ascii", "image"].includes(p.tilesetPreset) || !["player-centered", "full-map"].includes(p.cameraMode) ||
      ![0.75, 1, 1.25, 1.5, 2].includes(p.zoom)) throw new Error("preferences-invalid");
  if (!p.travel || Object.keys(p.travel).sort().join() !== "alwaysPickup,autoDetectTraps,autoMapArea,disturbTrapDetect" || Object.values(p.travel).some(v => typeof v !== "boolean") ||
      !p.mogaminator || Object.keys(p.mogaminator).sort().join() !== "autoGetMode,enUsSource,enabled,leaveDestroyedItems,zhCnSource" ||
      typeof p.mogaminator.enabled !== "boolean" || typeof p.mogaminator.leaveDestroyedItems !== "boolean" ||
      !["off", "ammo", "wanted"].includes(p.mogaminator.autoGetMode) ||
      typeof p.mogaminator.zhCnSource !== "string" || typeof p.mogaminator.enUsSource !== "string") throw new Error("preferences-invalid");
  const o = p.operations;
  if (!o || Object.keys(o).sort().join() !== "autoRepeat,cutCorners,defaultTarget,easyDisarm,easyOpen,runStops,targetPets,travelIgnoreItems" ||
      !["manual", "old-target", "nearest-enemy", "old-then-nearest"].includes(o.defaultTarget) ||
      [o.autoRepeat, o.cutCorners, o.easyDisarm, o.easyOpen, o.targetPets, o.travelIgnoreItems].some(v => typeof v !== "boolean") ||
      !o.runStops || Object.keys(o.runStops).sort().join() !== "knownTreasure,openDoors,stairs" ||
      Object.values(o.runStops).some(v => typeof v !== "boolean")) throw new Error("preferences-invalid");
  return { ...p, keyBindings: parseBindings(JSON.stringify(p.keyBindings)) };
}

const LEGACY_KEYS = {
  locale: "rfb.locale", inputPreset: "rfb.input-preset", tilesetPreset: "rfb.tileset-preset",
  cameraMode: "rfb.camera-mode", zoom: "rfb.zoom", keyBindings: "rfb.custom-keys.v1",
} as const;

export function migratePreferences(storage: Pick<Storage, "getItem">, defaults = defaultPreferences()): { preferences: Preferences; migrated: string[]; invalid: string[] } {
  let preferences = structuredClone(defaults);
  const migrated: string[] = [], invalid: string[] = [];
  for (const [field, key] of Object.entries(LEGACY_KEYS)) {
    const raw = storage.getItem(key);
    if (raw === null) continue;
    try {
      const value = field === "zoom" ? Number(raw) : field === "keyBindings" ? JSON.parse(raw) : raw;
      preferences = parsePreferences(JSON.stringify({ ...preferences, [field]: value }));
      migrated.push(key);
    } catch { invalid.push(key); }
  }
  return { preferences, migrated, invalid };
}

export class PreferencesClient {
  snapshot: PreferenceSnapshot | undefined;
  warnings: string[] = [];
  defaults: Preferences = defaultPreferences();
  readonly #storage: PreferenceStorage;
  #legacy: Pick<Storage, "getItem" | "removeItem"> | undefined;
  constructor(storage: PreferenceStorage) { this.#storage = storage; }

  // Read without replacing the current snapshot or any editor's revision/draft.
  async savedMogaminator(): Promise<MogaminatorPreferencesDto> {
    const saved = await this.#storage.load();
    if (!saved) throw new Error("preferences-unavailable");
    return parsePreferences(JSON.stringify(saved.preferences)).mogaminator;
  }

  async load(legacy?: Pick<Storage, "getItem" | "removeItem">): Promise<void> {
    if (legacy) this.#legacy = legacy;
    legacy = this.#legacy;
    this.defaults = parsePreferences(JSON.stringify(await this.#storage.defaults()));
    const saved = await this.#storage.load(); // A corrupt file is an error, never a migration trigger.
    this.warnings = [];
    if (saved) { this.snapshot = saved; return; }
    const migration = legacy ? migratePreferences(legacy, this.defaults) : { preferences: structuredClone(this.defaults), migrated: [], invalid: [] };
    const created = await this.#storage.save(migration.preferences, null);
    this.snapshot = created;
    this.warnings = migration.invalid.map(key => `preferences-migration-invalid: ${key}`);
    for (const key of migration.migrated) {
      try { legacy!.removeItem(key); }
      catch { this.warnings.push(`preferences-migration-cleanup: ${key}`); }
    }
  }

  async save(preferences: Preferences, expectedRevision: number): Promise<void> {
    const validated = parsePreferences(JSON.stringify(preferences));
    this.snapshot = await this.#storage.save(validated, expectedRevision);
  }
}
