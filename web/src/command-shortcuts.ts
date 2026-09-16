// SPDX-License-Identifier: MPL-2.0

import type { InputPreset } from "./input-controller";

export type ItemShortcut = "equip" | "unequip" | "drop" | "destroy" | "inspect" | "activate" |
  "food" | "potion" | "scroll" | "wand" | "staff" | "rod" | "refuel" | "throw" | "inscribe" | "uninscribe";
export type CommandShortcut = ItemShortcut | "inventory" | "equipment" | "study" | "browse" | "power" | "cast" |
  "help" | "knowledge" | "end-character" | "glyphs" | "colors" | "advanced-preferences" | "reload-pickup-rules" |
  "input-config" | "command-menu" | "notes" | "screen-export" | "record-register" | "play-register" |
  "map-overview" | "map-locate" | "map-center" | "monster-list" | "symbol-query" | "floor-feeling" |
  "character" | "tasks" | "messages" | "settings" | "save" | "save-exit" | "repeat-last" | "swap-rings" | "pets" |
  "spike" | "alter" | "look" | "search" | "toggle-search" | "open" | "disarm" | "close" | "bash" | "dig" | "fire" | "rest" | "pickup" | "resume-travel" | "nearest-unknown-item";

const ORIGINAL: Partial<Record<string, CommandShortcut>> = {
  "@": "input-config", ":": "notes", ")": "screen-export", '"': "record-register", "'": "play-register", Enter: "command-menu",
  "%": "glyphs", "&": "colors", "!": "advanced-preferences", "$": "reload-pickup-rules",
  "?": "help", "~": "knowledge", Q: "end-character",
  M: "map-overview", L: "map-locate", "[": "monster-list", Y: "monster-list", "/": "symbol-query",
  n: "repeat-last",
  W: "swap-rings",
  p: "pets",
  j: "spike", "+": "alter", w: "equip", t: "unequip", d: "drop", k: "destroy", e: "equipment", i: "inventory", I: "inspect",
  G: "study", b: "browse", U: "power", m: "cast", A: "activate", E: "food", q: "potion", r: "scroll",
  a: "wand", u: "staff", z: "rod", F: "refuel", v: "throw", "{": "inscribe", "}": "uninscribe",
  C: "character", "=": "settings", l: "look", s: "search", S: "toggle-search", o: "open", D: "disarm", c: "close",
  B: "bash", T: "dig", f: "fire", R: "rest", g: "pickup", J: "resume-travel", H: "nearest-unknown-item",
};

export const commandMenuEntries: readonly (readonly [string, string])[] = [
  ...Object.entries(ORIGINAL).map(([key, command]) => [key, command!] as const),
  ["1", "south-west"], ["2", "south"], ["3", "south-east"], ["4", "west"], ["5", "stay"],
  ["6", "east"], ["7", "north-west"], ["8", "north"], ["9", "north-east"],
  [";", "walk"], ["-", "special-walk"], [".", "run"], ["Z", "auto-explore"], ["0", "count"],
  ["Tab", "auto-attack"],
  ["*", "target"], ["`", "local-travel"], ["<", "connection"], ["]", "object-list"], ["_", "pickup-rules"],
  ["Ctrl+v", "map-center"], ["Ctrl+f", "floor-feeling"], ["Ctrl+s", "save"], ["Ctrl+x", "save-exit"],
  ["Ctrl+q", "tasks"], ["Ctrl+p", "messages"], ["Ctrl+g", "auto-get"],
];

// Inscriptions name the canonical command, independently of the active key preset.
export function originalCommandKey(command: CommandShortcut): string | undefined {
  return Object.entries(ORIGINAL).find(([, shortcut]) => shortcut === command)?.[0];
}
const ROGUELIKE: Partial<Record<string, CommandShortcut>> = {
  X: "repeat-last",
  ...ORIGINAL, W: "map-locate", S: "spike", "#": "toggle-search", T: "unequip", t: "fire", f: "bash", x: "look", z: "wand", a: "rod", O: "power", "(": "resume-travel",
};
// These characters are directions or reserved for RFB operations not implemented yet.
for (const key of "hjklyubnHJKLYUBN") delete ROGUELIKE[key];
// Rogue W maps to source L (locate on map); ring exchange is the literal command \W.

export function commandShortcut(
  event: Pick<KeyboardEvent, "key" | "ctrlKey" | "altKey" | "metaKey" | "shiftKey">,
  preset: InputPreset,
): CommandShortcut | undefined {
  if (event.altKey || event.metaKey) return undefined;
  if (event.ctrlKey) {
    if (event.shiftKey) return undefined;
    const key = event.key.toLowerCase();
    if (key === "c" || (key === "k" && preset === "original")) return "end-character";
    if (key === "v") return "map-center";
    if (key === "f") return "floor-feeling";
    if (key === "s") return "save";
    if (key === "x") return "save-exit";
    if (key === "q") return "tasks";
    if (key === "p") return "messages";
    if (key === "]") return "screen-export";
    if (key === "e" && preset !== "roguelike") return "input-config";
    if (preset === "roguelike" && key === "d") return "destroy";
    if (preset === "roguelike" && key === "t") return "dig";
    if (preset === "roguelike" && key === "e") return "nearest-unknown-item";
    return undefined;
  }
  if (event.key === "!") return "advanced-preferences";
  if (event.key === "$") return "reload-pickup-rules";
  if (event.key === "%") return "glyphs";
  if (event.key === "&") return "colors";
  if (event.key === "=") return "settings";
  if (event.key === "?") return "help";
  if (event.key === "~") return "knowledge";
  if (event.key === "@") return "input-config";
  if (event.key === ":") return "notes";
  if (event.key === ")") return "screen-export";
  if (event.key === "Enter") return "command-menu";
  return preset === "original" ? ORIGINAL[event.key] : preset === "roguelike" ? ROGUELIKE[event.key] : undefined;
}
