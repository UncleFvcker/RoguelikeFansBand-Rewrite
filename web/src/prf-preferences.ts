// SPDX-License-Identifier: MPL-2.0
// Exchange subset of RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c src/files.c.
import { parsePreferences, type Preferences } from "./preferences.ts";
import { commandShortcut, commandMenuEntries } from "./command-shortcuts.ts";
import { parseBindings, type KeyBinding } from "./command-recording.ts";
import { BASE_PALETTE, prfVisualId } from "./visual-preferences.ts";
import type { EditableVisualDto } from "./protocol";

export interface PrfDiagnostic { line: number; input: string; kind: "error" | "unsupported" | "notice"; reason: string }
export interface PrfPreview { preferences: Preferences; diagnostics: PrfDiagnostic[]; blocked: boolean; needsSubset: boolean }
// One alias table serves import and export. Values are paths in the current format, not old runtime flags.
const BOOLS: Record<string, string> = {
  always_pickup: "travel.alwaysPickup", auto_detect_traps: "travel.autoDetectTraps", auto_map_area: "travel.autoMapArea",
  disturb_trap_detect: "travel.disturbTrapDetect", always_repeat: "operations.autoRepeat", target_pet: "operations.targetPets",
  easy_open: "operations.easyOpen", easy_disarm: "operations.easyDisarm", find_cut: "operations.cutCorners",
  travel_ignore_items: "operations.travelIgnoreItems", leave_mogaminator: "mogaminator.leaveDestroyedItems",
  no_mogaminator: "!mogaminator.enabled", find_ignore_stairs: "!operations.runStops.stairs",
  find_ignore_doors: "!operations.runStops.openDoors", find_ignore_veins: "!operations.runStops.knownTreasure",
  view_unsafe_grids: "display.unsafeGrids", hilite_player: "display.highlightPlayer", display_path: "display.targetPath",
  effective_speed: "display.effectiveSpeed", decimal_stats: "display.decimalStats", exp_need: "display.experienceNeeded",
  show_origins: "display.showOrigins", alert_trap_detect: "display.alertTrapDetect", alert_poison: "display.alertPoison",
  describe_slots: "display.describeSlots", show_weights: "display.showWeights", show_discounts: "display.showDiscounts",
  show_item_graph: "display.showItemIcons", display_distance: "display.monsterDistance", list_stairs: "display.listStairs",
};
function boolValue(p: Preferences, path: string, value?: boolean): boolean {
  const inverse = path.startsWith("!"), parts = path.replace(/^!/, "").split(".");
  let node = p as unknown as Record<string, unknown>;
  for (const part of parts.slice(0, -1)) node = node[part] as Record<string, unknown>;
  const key = parts.at(-1)!;
  if (value !== undefined) node[key] = inverse ? !value : value;
  return inverse ? !node[key] : node[key] as boolean;
}
function unsupportedOption(name: string): string {
  if (/^(view_|lite_|graph_)/.test(name)) return "theme-tileset";
  if (/^(plain_descriptions|abbrev_)/.test(name)) return "contextual-descriptions";
  if (/^(disturb_|ignore_unview)/.test(name)) return "enemy-stop-policy";
  if (/^(birth_|ironman_|coffee_|adult_|random_artifacts|preserve_mode|small_levels|empty_levels)/.test(name)) return "character-creation";
  if (/^(destroy_|leave_)/.test(name)) return "mogaminator-rules";
  return "no-equivalent-option";
}
const SINGLE_KEY = /^[\x21-\x7e]$/;
// PRF escape/control encodings are deliberately not interpreted.
const literalKey = (key: string) => SINGLE_KEY.test(key) && key !== "\\" && key !== "^";
export function prfAction(key: string, preset: "original" | "roguelike"): string | undefined {
  if (!literalKey(key) || [';', '-', '.', '+', '0', '"', "'"].includes(key)) return;
  if (/^[1-9]$/.test(key)) return key;
  if (preset === "roguelike" && "hjklyubn".includes(key)) return "42867913"["hjklyubn".indexOf(key)];
  const shortcut = commandShortcut({ key, ctrlKey: false, altKey: false, metaKey: false, shiftKey: /[A-Z]/.test(key) }, preset);
  if (shortcut) return commandMenuEntries.find(([, action]) => action === shortcut)?.[0];
  return ["Z", "*", "`", "<", "]", "_"].includes(key) ? key : undefined;
}
function numbers(text: string, count: number): number[] {
  const parts = text.split(/[:/]/);
  if (parts.length !== count || parts.some(p => !/^(?:0[xX][\da-fA-F]+|0[0-7]*|[1-9]\d*)$/.test(p))) throw new Error("syntax");
  const values = parts.map(p => /^0[0-7]+$/.test(p) ? parseInt(p, 8) : Number(p));
  if (values.some(v => !Number.isSafeInteger(v))) throw new Error("range");
  return values;
}
export function parsePrf(text: string, current: Preferences, catalog: readonly EditableVisualDto[]): PrfPreview {
  const p = structuredClone(current), diagnostics: PrfDiagnostic[] = [];
  let old = ["old-target", "old-then-nearest"].includes(p.operations.defaultTarget);
  let auto = ["nearest-enemy", "old-then-nearest"].includes(p.operations.defaultTarget);
  let ammo = p.mogaminator.autoGetMode === "ammo", objects = p.mogaminator.autoGetMode === "wanted";
  let action: string | undefined, actionLine = 0, actionUsed = false;
  const lines = text.replace(/^\uFEFF/, "").split(/\r?\n/);
  lines.forEach((input, index) => {
    const line = index + 1;
    const report = (kind: PrfDiagnostic["kind"], reason: string) => diagnostics.push({ line, input, kind, reason });
    if (!input || input.startsWith("#")) return;
    if (/^\s/.test(input)) { report("notice", "leading-space"); return; }
    if (input[1] !== ":" || /[\u0000-\u0008\u000b-\u001f\ufffd]/.test(input)) { report("error", "syntax"); return; }
    const op = input[0], value = input.slice(2);
    try {
      if (op === "?" || op === "%") { report("error", "context-required"); return; }
      if (op === "X" || op === "Y") {
        if (!/^[a-z][a-z0-9_]*$/.test(value)) throw new Error("syntax");
        const enabled = op === "Y";
        if (Object.hasOwn(BOOLS, value)) boolValue(p, BOOLS[value]!, enabled);
        else switch (value) {
          case "rogue_like_commands": p.inputPreset = enabled ? "roguelike" : "original"; break;
          case "center_player": p.cameraMode = enabled ? "player-centered" : "full-map"; break;
          case "use_old_target": old = enabled; break;
          case "auto_target": auto = enabled; break;
          case "auto_get_ammo": ammo = enabled; break;
          case "auto_get_objects": objects = enabled; break;
          default: report("unsupported", unsupportedOption(value));
        }
      } else if (op === "V") {
        const [id, kv, r, g, b] = numbers(value, 5) as [number, number, number, number, number];
        if (id > 15 || kv !== 0 || [r, g, b].some(v => v > 255)) throw new Error("palette-range");
        p.visuals.palette[id] = "#" + [r, g, b].map(v => v.toString(16).padStart(2, "0")).join("");
      } else if (op === "R" || op === "K") {
        const nums = numbers(value, op === "R" ? 3 : 4), glyph = nums.pop()!, attr = nums.pop()!;
        if (attr > 15 || (glyph !== 0 && (glyph < 33 || glyph > 126))) throw new Error("visual-range");
        const key = op + ":" + nums.join(":"), matches = catalog.filter(v => v.prf === key);
        const ids = new Set(matches.map(prfVisualId));
        if (ids.size !== 1) { report("unsupported", "unmapped-visual"); return; }
        const id = [...ids][0]!;
        if (attr || glyph) p.visuals.overrides[id] = { ...p.visuals.overrides[id],
          ...(attr || glyph ? { foreground: BASE_PALETTE[attr]! } : {}),
          ...(glyph ? { glyph: String.fromCharCode(glyph) } : {}) };
      } else if (op === "A") {
        if (action !== undefined && !actionUsed) diagnostics.push({ line: actionLine, input: lines[actionLine - 1]!, kind: "unsupported", reason: "unused-action" });
        action = value; actionLine = line; actionUsed = false;
        if (!literalKey(value)) report("unsupported", "single-key-only");
      } else if (op === "C") {
        const match = /^([01]):(.+)$/.exec(value);
        if (!match || action === undefined) throw new Error("mapping-context");
        actionUsed = true;
        const preset = match[1] === "0" ? "original" : "roguelike", trigger = match[2]!;
        const canonical = prfAction(action, preset);
        if (!literalKey(trigger) || !canonical) { report("unsupported", "single-key-only"); return; }
        const bindings = p.keyBindings.filter(b => b.preset !== preset || b.trigger !== trigger);
        p.keyBindings = parseBindings(JSON.stringify([...bindings, { preset, trigger, action: canonical }]));
      } else report("unsupported", ({ F: "terrain-index", S: "misc-index", U: "appearance-category", E: "appearance-category",
        G: "theme-tileset", Z: "effect-colors", P: "macro-sequence", T: "terminal-template" } as Record<string, string>)[op!] ?? "unknown-directive");
    } catch (error) { report("error", error instanceof Error ? error.message.startsWith("Invalid key") ? "mapping-invalid" : error.message : "syntax"); }
  });
  if (action !== undefined && !actionUsed) diagnostics.push({ line: actionLine, input: lines[actionLine - 1]!, kind: "unsupported", reason: "unused-action" });
  p.operations.defaultTarget = old ? auto ? "old-then-nearest" : "old-target" : auto ? "nearest-enemy" : "manual";
  p.mogaminator.autoGetMode = objects ? "wanted" : ammo ? "ammo" : "off";
  const blocked = diagnostics.some(d => d.kind === "error");
  return { preferences: blocked ? structuredClone(current) : parsePreferences(JSON.stringify(p)), diagnostics,
    blocked, needsSubset: diagnostics.some(d => d.kind === "unsupported") };
}
function exportedBinding(binding: KeyBinding): string | undefined {
  if (!["original", "roguelike"].includes(binding.preset) || !literalKey(binding.trigger)) return;
  const preset = binding.preset as "original" | "roguelike";
  const key = Array.from({ length: 94 }, (_, i) => String.fromCharCode(i + 33)).find(key => prfAction(key, preset) === binding.action);
  return key ? `A:${key}\nC:${preset === "original" ? 0 : 1}:${binding.trigger}` : undefined;
}
export function exportPrf(p: Preferences, catalog: readonly EditableVisualDto[]): { text: string; omitted: string[] } {
  const lines = ["# RFB Rewrite supported preference subset (UTF-8)"], omitted = [
    "locale", "zoom", "tilesetPreset", "visuals.theme", "display.hpWarningPercent", "display.manaWarningPercent",
    "mogaminator.zhCnSource", "mogaminator.enUsSource", "hotbar",
  ];
  const bool = (name: string, value: boolean) => lines.push(`${value ? "Y" : "X"}:${name}`);
  for (const [name, path] of Object.entries(BOOLS)) bool(name, boolValue(p, path));
  if (["original", "roguelike"].includes(p.inputPreset)) bool("rogue_like_commands", p.inputPreset === "roguelike");
  else omitted.push("inputPreset");
  bool("center_player", p.cameraMode === "player-centered");
  bool("use_old_target", ["old-target", "old-then-nearest"].includes(p.operations.defaultTarget));
  bool("auto_target", ["nearest-enemy", "old-then-nearest"].includes(p.operations.defaultTarget));
  bool("auto_get_ammo", p.mogaminator.autoGetMode === "ammo");
  bool("auto_get_objects", p.mogaminator.autoGetMode === "wanted");
  p.visuals.palette.forEach((color, index) => lines.push(`V:${index}:0:${[1, 3, 5].map(i => parseInt(color.slice(i, i + 2), 16)).join(":")}`));
  for (const [id, visual] of Object.entries(p.visuals.overrides)) {
    const entry = catalog.find(v => v.prf && prfVisualId(v) === id), key = entry?.prf;
    const attr = visual.foreground === undefined ? -1 : BASE_PALETTE.indexOf(visual.foreground.toLowerCase());
    const code = visual.glyph?.codePointAt(0) ?? 0;
    // An ASCII character also writes attr in RFB. Glyph-only and background overrides need JSON.
    if (!key || visual.background !== undefined || attr < 0 || (attr === 0 && code === 0) || (code !== 0 && (code < 33 || code > 126))) omitted.push(`visuals.overrides.${id}`);
    else lines.push(`${key}:${attr}:${code}`);
  }
  for (const binding of p.keyBindings) {
    const line = exportedBinding(binding);
    if (line) lines.push(line); else omitted.push(`keyBindings.${binding.preset}.${binding.trigger}`);
  }
  return { text: lines.join("\n") + "\n", omitted };
}
