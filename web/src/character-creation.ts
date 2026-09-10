// SPDX-License-Identifier: MPL-2.0
// RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c, src/py_birth.c:
// b_race_groups and _class_groups order and membership, restricted to the existing creation entries.

import type { Localization } from "./localization.ts";

interface CreationLeaf {
  readonly id: string;
  readonly nameKey: string;
  readonly descriptionKey: string;
  readonly notes: readonly string[];
}

function race<const S extends string>(slug: S, notes: readonly string[] = []) {
  return { id: `rfb-legacy.race.${slug}` as const, nameKey: `race-legacy-${slug}-name`, descriptionKey: `race-legacy-${slug}-description`, notes };
}

const HUMAN = { id: "demo.race.rfb-human", nameKey: "race-demo-rfb-human-name", descriptionKey: "race-demo-rfb-human-description", notes: [] } as const;
export const DRACONIAN_RACES = (["red", "white", "blue", "black", "green", "bronze", "crystal", "gold", "shadow"] as const).map(
  (color) => ({ ...race(`draconian-${color}`), descriptionKey: "race-legacy-draconian-description" }),
);
const DRACONIAN = { id: "draconian", nameKey: "session-race-group-draconian", descriptionKey: "race-legacy-draconian-description", notes: [], children: DRACONIAN_RACES } as const;
interface CreationBranch extends CreationLeaf { readonly children: readonly CreationLeaf[] }
type CreationOption = CreationLeaf | CreationBranch;
interface CreationGroup { readonly id: string; readonly options: readonly CreationOption[] }

export const RACE_GROUPS = [
  { id: "human", options: [race("amberite"), race("barbarian"), race("dunadan"), HUMAN] },
  { id: "elf", options: [race("dark-elf"), race("high-elf"), race("tomte", ["trait-tomte-headgear-rule"]), race("wood-elf")] },
  { id: "small", options: [race("dwarf"), race("gnome"), race("hobbit"), race("nibelung")] },
  { id: "fairy", options: [race("shadow-fairy"), race("sprite")] },
  { id: "celestial", options: [race("archon"), race("imp")] },
  { id: "giant", options: [race("cyclops"), race("half-giant"), race("half-orc"), race("half-titan"), race("half-troll"), race("kobold"), race("ogre"), race("snotling")] },
  { id: "undead", options: [race("einheri"), race("skeleton"), race("spectre", ["basics", "defenses", "senses", "passage", "density", "diet", "power", "birth"].map(rule => `trait-spectre-rule-${rule}`)), race("zombie")] },
  { id: "other", options: [race("beastman"), race("boit"), DRACONIAN, race("ent", ["basics", "growth", "digging", "fire", "diet", "forest", "power", "birth"].map(rule => `trait-ent-rule-${rule}`)), race("golem"), race("klackon"), race("kutar"), race("mindflayer"), race("tonberry", ["basics", "speed", "damage", "attacks", "confusion", "birth"].map(rule => `trait-tonberry-rule-${rule}`)), race("yeek")] },
] as const satisfies readonly CreationGroup[];

type GroupEntry = (typeof RACE_GROUPS)[number]["options"][number];
export type PlaytestRaceId = Exclude<GroupEntry, { children: unknown }>["id"] | (typeof DRACONIAN_RACES)[number]["id"];
export const CREATION_RACES = RACE_GROUPS.flatMap(group => group.options.flatMap<CreationLeaf>(entry => "children" in entry ? entry.children : [entry]));
export const PLAYTEST_RACE_IDS = CREATION_RACES.map(race => race.id as PlaytestRaceId);

function career<const S extends string>(slug: S) {
  return { id: `demo.build.${slug}` as const, nameKey: `class-demo-${slug}-name`, descriptionKey: `class-demo-${slug}-description`, notes: [] };
}

function deathCaster<const S extends string>(slug: S) {
  return { ...career(slug), id: slug, children: [{
    id: `demo.build.${slug}-death` as const, nameKey: "session-career-death-name",
    descriptionKey: `build-demo-${slug}-death-description`, notes: ["session-career-available-realms"],
  }] };
}

export const CAREER_GROUPS = [
  { id: "melee", options: [career("warrior")] },
  { id: "archery", options: [career("archer"), career("sniper")] },
  { id: "magic", options: [deathCaster("high-mage")] },
  { id: "hybrid", options: [deathCaster("paladin")] },
  { id: "riding", options: [career("cavalry")] },
] as const satisfies readonly CreationGroup[];
type CareerEntry = (typeof CAREER_GROUPS)[number]["options"][number];
export type PlaytestBuildId = Exclude<CareerEntry, { children: unknown }>["id"] | Extract<CareerEntry, { children: unknown }>["children"][number]["id"];
export const CREATION_BUILDS = CAREER_GROUPS.flatMap(group => group.options.flatMap<CreationLeaf>(entry => "children" in entry ? entry.children : [entry]));
export const PLAYTEST_BUILD_IDS = CREATION_BUILDS.map(build => build.id as PlaytestBuildId);

// Both creation pages have categories, options and one optional child level.
export class CreationMenu {
  readonly #root: HTMLElement;
  readonly #groups: HTMLElement;
  readonly #options: HTMLElement;
  readonly #path: HTMLElement;
  readonly #backButton: HTMLButtonElement;
  readonly #pendingNote: HTMLElement;
  readonly #title: HTMLElement;
  readonly #description: HTMLElement;
  readonly #notes: HTMLElement;
  readonly #localization: Localization;
  readonly #onChange: () => void;
  readonly #onBack: () => void;
  readonly #kind: "race" | "career";
  readonly #catalog: readonly CreationGroup[];
  #selected: CreationLeaf;
  #group: CreationGroup;
  #branch: CreationBranch | undefined;
  #pending = false;
  #viewed: CreationOption;
  #busy = false;

  constructor(kind: "race" | "career", root: HTMLElement, localization: Localization, onChange: () => void, onBack: () => void) {
    this.#kind = kind;
    this.#catalog = kind === "race" ? RACE_GROUPS : CAREER_GROUPS;
    this.#selected = kind === "race" ? HUMAN : CAREER_GROUPS[0].options[0];
    this.#group = this.#catalog[0]!;
    this.#viewed = this.#selected;
    this.#root = root;
    this.#localization = localization;
    this.#onChange = onChange;
    this.#onBack = onBack;
    this.#groups = root.querySelector<HTMLElement>(`#session-${this.#kind}-groups`)!;
    this.#options = root.querySelector<HTMLElement>(`#session-${this.#kind}-options`)!;
    this.#path = root.querySelector<HTMLElement>(`#session-${this.#kind}-path`)!;
    this.#backButton = root.querySelector<HTMLButtonElement>(`#session-${this.#kind}-back`)!;
    this.#pendingNote = root.querySelector<HTMLElement>(`#session-${this.#kind}-pending`)!;
    this.#title = root.querySelector<HTMLElement>(`#session-${this.#kind}-detail-title`)!;
    this.#description = root.querySelector<HTMLElement>(`#session-${this.#kind}-description`)!;
    this.#notes = root.querySelector<HTMLElement>(`#session-${this.#kind}-notes`)!;
  }

  get selectedId(): string { return this.#selected.id; }
  get pending(): boolean { return this.#pending; }
  get selectedName(): string { return this.#name(this.#selected); }

  install(): void {
    this.#root.addEventListener("click", this.#click);
    this.#root.addEventListener("focusin", this.#focus);
    this.#root.addEventListener("pointerover", this.#hover);
    this.#root.addEventListener("keydown", this.#keydown);
  }

  dispose(): void {
    this.#root.removeEventListener("click", this.#click);
    this.#root.removeEventListener("focusin", this.#focus);
    this.#root.removeEventListener("pointerover", this.#hover);
    this.#root.removeEventListener("keydown", this.#keydown);
  }

  setBusy(busy: boolean): void { this.#busy = busy; }

  // Entering or leaving the page cancels a draft branch, never the confirmed leaf.
  reset(): void {
    this.#group = this.#catalog.find(group => group.options.some(entry => entry.id === this.#selected.id || ("children" in entry && entry.children.some(leaf => leaf.id === this.#selected.id))))!;
    this.#branch = this.#parent(this.#selected.id);
    this.#pending = false;
    this.#viewed = this.#selected;
    this.localize();
  }

  localize(): void {
    const active = this.#root.ownerDocument.activeElement as HTMLElement | null;
    const focusedId = active?.dataset[this.#kind + "Id"];
    const focusedGroup = active?.dataset[this.#kind + "Group"];
    this.#groups.replaceChildren(...this.#catalog.map(group => {
      const button = this.#button(this.#localization.format(`session-${this.#kind}-category-${group.id}`));
      button.dataset[this.#kind + "Group"] = group.id;
      button.setAttribute("aria-pressed", String(group.id === this.#group.id));
      button.tabIndex = group.id === this.#group.id ? 0 : -1;
      return button;
    }));
    this.#options.replaceChildren(...this.#entries.map(entry => {
      const button = this.#button(this.#localization.format(entry.nameKey));
      button.dataset[this.#kind + "Id"] = entry.id;
      button.tabIndex = entry.id === this.#viewed.id ? 0 : -1;
      button.setAttribute("aria-describedby", `session-${this.#kind}-description`);
      if ("children" in entry) {
        button.setAttribute("aria-label", this.#localization.format(`session-${this.#kind}-open-children`, { name: this.#localization.format(entry.nameKey) }));
        button.classList.add("session-menu-parent");
      } else button.setAttribute("aria-pressed", String(entry.id === this.#selected.id));
      return button;
    }));
    this.#path.textContent = [this.#localization.format(`session-${this.#kind}-label`), this.#localization.format(`session-${this.#kind}-category-${this.#group.id}`), ...(this.#branch ? [this.#localization.format(this.#branch.nameKey)] : [])].join(" › ");
    this.#backButton.textContent = this.#localization.format(this.#branch ? "session-menu-back" : "session-menu-back-overview");
    this.#pendingNote.hidden = !this.#pending;
    this.#preview(this.#viewed);
    if (focusedId) this.#optionButton(focusedId)?.focus();
    if (focusedGroup) this.#groups.querySelector<HTMLButtonElement>(`[data-${this.#kind}-group="${focusedGroup}"]`)?.focus();
  }

  #button(label: string): HTMLButtonElement {
    const button = this.#root.ownerDocument.createElement("button");
    button.type = "button";
    button.textContent = label;
    button.disabled = this.#busy;
    return button;
  }

  get #entries(): readonly CreationOption[] { return this.#branch ? this.#branch.children : this.#group.options; }
  #optionButton(id: string): HTMLButtonElement | null { return this.#options.querySelector(`[data-${this.#kind}-id="${id}"]`); }

  #parent(id: string): CreationBranch | undefined {
    return this.#catalog.flatMap(group => group.options).find((entry): entry is CreationBranch => "children" in entry && entry.children.some(leaf => leaf.id === id));
  }

  #name(race: CreationOption): string {
    const name = this.#localization.format(race.nameKey);
    const parent = this.#parent(race.id);
    return parent ? `${this.#localization.format(parent.nameKey)} · ${name}` : name;
  }

  #preview(entry: CreationOption): void {
    this.#viewed = entry;
    this.#title.textContent = this.#name(entry);
    this.#description.textContent = this.#localization.format(entry.descriptionKey);
    this.#notes.replaceChildren(...entry.notes.map(key => {
      const item = this.#root.ownerDocument.createElement("li");
      item.textContent = this.#localization.format(key);
      return item;
    }));
    this.#notes.hidden = entry.notes.length === 0;
    this.#root.querySelector<HTMLElement>(`#session-${this.#kind}-details`)!.scrollTop = 0;
  }

  readonly #click = (event: MouseEvent): void => {
    if (this.#busy || !(event.target instanceof Element)) return;
    const button = event.target.closest<HTMLButtonElement>("button");
    if (!button) return;
    if (button === this.#backButton) { this.back(); return; }
    const group = this.#catalog.find(group => group.id === button.dataset[this.#kind + "Group"]);
    if (group) {
      this.#group = group;
      this.#branch = undefined;
      this.#pending = false;
      this.#viewed = group.options.find(entry => entry.id === this.#selected.id) ?? group.options[0]!;
      this.localize();
      this.#optionButton(this.#viewed.id)!.focus();
    } else {
      const entry = this.#entries.find(entry => entry.id === button.dataset[this.#kind + "Id"]);
      if (!entry) return;
      if ("children" in entry) {
        this.#branch = entry;
        this.#pending = true;
        this.#viewed = entry.children.find(leaf => leaf.id === this.#selected.id) ?? entry.children[0]!;
        this.localize();
        this.#optionButton(this.#viewed.id)!.focus();
      } else {
        this.#selected = entry;
        this.#pending = false;
        this.#pendingNote.hidden = true;
        for (const option of this.#options.querySelectorAll<HTMLButtonElement>("[aria-pressed]")) option.setAttribute("aria-pressed", String(option.dataset[this.#kind + "Id"] === entry.id));
        this.#preview(entry);
      }
    }
    this.#onChange();
  };

  back(): void {
    if (!this.#branch) { this.#onBack(); return; }
    const parent = this.#branch;
    this.#branch = undefined;
    this.#pending = false;
    this.#viewed = parent;
    this.localize();
    this.#optionButton(parent.id)!.focus();
    this.#onChange();
  }

  readonly #focus = (event: FocusEvent): void => {
    if (this.#busy || !(event.target instanceof HTMLButtonElement)) return;
    if (event.target.dataset[this.#kind + "Id"] || event.target.dataset[this.#kind + "Group"]) {
      for (const button of event.target.parentElement!.querySelectorAll<HTMLButtonElement>("button")) button.tabIndex = button === event.target ? 0 : -1;
    }
    const entry = this.#entries.find(entry => entry.id === (event.target as HTMLElement).dataset[this.#kind + "Id"]);
    if (entry) this.#preview(entry);
  };

  readonly #hover = (event: PointerEvent): void => {
    if (this.#busy || !(event.target instanceof Element)) return;
    const id = event.target.closest<HTMLElement>(`[data-${this.#kind}-id]`)?.dataset[this.#kind + "Id"];
    const entry = this.#entries.find(entry => entry.id === id);
    if (entry && entry.id !== this.#viewed.id) this.#preview(entry);
  };

  readonly #keydown = (event: KeyboardEvent): void => {
    if (this.#busy || event.isComposing) return;
    if (event.key === "Escape") {
      event.preventDefault(); event.stopPropagation(); this.back(); return;
    }
    if (!(event.target instanceof HTMLButtonElement) || !["ArrowUp", "ArrowDown", "Home", "End"].includes(event.key)) return;
    if (!event.target.dataset[this.#kind + "Id"] && !event.target.dataset[this.#kind + "Group"]) return;
    event.preventDefault(); event.stopPropagation();
    const buttons = [...event.target.parentElement!.querySelectorAll<HTMLButtonElement>("button")];
    const index = buttons.indexOf(event.target);
    const next = event.key === "Home" ? 0 : event.key === "End" ? buttons.length - 1 : (index + (event.key === "ArrowDown" ? 1 : buttons.length - 1)) % buttons.length;
    buttons[next]!.focus();
  };
}
