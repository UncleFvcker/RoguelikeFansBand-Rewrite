// SPDX-License-Identifier: MPL-2.0
import type { EditableVisualDto } from "./protocol";
// RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c, variable.c base 16 colors.
export const BASE_PALETTE = ["#000000", "#ffffff", "#808080", "#ff8000", "#c00000", "#008040", "#0000ff", "#804000",
  "#404040", "#c0c0c0", "#ff00ff", "#ffff00", "#ff0000", "#00ff00", "#00ffff", "#c08040"];
export const DEFAULT_THEME = {
  background: "#090d12", grid: "#18212d", memoryColor: "#12213a", hiddenColor: "#000000",
  player: "#fff2a8", path: "#88dcff", uncovered: "#c99c64", pet: "#245c46",
  memoryOpacity: 0.58, lightTintOpacity: 0.18, darknessOpacity: 0.62,
};
export type MapTheme = typeof DEFAULT_THEME;
export interface VisualOverride { glyph?: string; foreground?: string; background?: string }
export interface VisualPreferences { overrides: Record<string, VisualOverride>; palette: string[]; theme: MapTheme }
export function defaultVisuals(): VisualPreferences {
  return { overrides: {}, palette: [...BASE_PALETTE], theme: { ...DEFAULT_THEME } };
}
export const validColor = (v: unknown): v is string => typeof v === "string" && /^#[0-9a-fA-F]{6}$/.test(v);
export const validGlyph = (v: unknown): v is string => typeof v === "string" && [...v].length === 1 &&
  v.trim().length > 0 && !/[\u0000-\u001f\u007f-\u009f\ud800-\udfff\u00ad\u061c\u200b-\u200f\u202a-\u202e\u2060-\u206f\ufeff\ufe00-\ufe0f\u{e0000}-\u{e0fff}]/u.test(v);
const record = (v: unknown): v is Record<string, unknown> => !!v && typeof v === "object" && !Array.isArray(v);
export function validVisuals(v: unknown): v is VisualPreferences {
  if (!record(v) || Object.keys(v).sort().join() !== "overrides,palette,theme" || !record(v.overrides) ||
      !Array.isArray(v.palette) || v.palette.length !== 16 || !v.palette.every(validColor) || !record(v.theme)) return false;
  if (Object.keys(v.overrides).length > 20000 || !Object.entries(v.overrides).every(([id, item]) =>
    /^[a-z0-9_-]+(?:\.[a-z0-9_-]+){2,}$/.test(id) && id.length <= 192 && record(item) &&
    Object.keys(item).length > 0 && Object.entries(item).every(([field, value]) =>
      field === "glyph" ? validGlyph(value) : ["foreground", "background"].includes(field) && validColor(value)))) return false;
  const theme = v.theme;
  return Object.keys(theme).sort().join() === Object.keys(DEFAULT_THEME).sort().join() &&
    Object.entries(DEFAULT_THEME).every(([key, value]) => typeof value === "string" ? validColor(theme[key]) :
      typeof theme[key] === "number" && Number.isFinite(theme[key]) && theme[key] >= 0 && theme[key] <= 1);
}
export function paletteColor(p: VisualPreferences, color: string): string {
  const index = BASE_PALETTE.indexOf(color.toLowerCase());
  return index < 0 ? color : p.palette[index]!;
}
export function prfVisualId(visual: Pick<EditableVisualDto, "id" | "prf">): string {
  return visual.prf?.startsWith("K:") ? "core.prf-kind." + visual.prf.slice(2).replace(":", "-") : visual.id;
}
export function visualOverride(p: VisualPreferences, visual: EditableVisualDto): VisualOverride {
  return { ...p.overrides[prfVisualId(visual)], ...p.overrides[visual.id] };
}
export function visualStyle(p: VisualPreferences, id: string, base: { glyph: string; foreground: string; background?: string }, known: boolean, visual?: EditableVisualDto) {
  const override = known ? visual ? visualOverride(p, visual) : p.overrides[id] : undefined;
  return { glyph: override?.glyph ?? base.glyph,
    foreground: paletteColor(p, override?.foreground ?? base.foreground),
    background: override?.background !== undefined || base.background !== undefined
      ? paletteColor(p, override?.background ?? base.background!) : undefined };
}
