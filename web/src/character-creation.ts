// SPDX-License-Identifier: MPL-2.0
// RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c, src/py_birth.c:
// b_race_groups order and membership, restricted to the existing creation entries.

import type { Localization } from "./localization.ts";

interface RaceLeaf {
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
type RaceOption = RaceLeaf | typeof DRACONIAN;
interface RaceGroup { readonly id: string; readonly races: readonly RaceOption[] }

export const RACE_GROUPS = [
  { id: "human", races: [race("amberite"), race("barbarian"), race("dunadan"), HUMAN] },
  { id: "elf", races: [race("dark-elf"), race("high-elf"), race("tomte", ["trait-tomte-headgear-rule"]), race("wood-elf")] },
  { id: "small", races: [race("dwarf"), race("gnome"), race("hobbit"), race("nibelung")] },
  { id: "fairy", races: [race("shadow-fairy"), race("sprite")] },
  { id: "celestial", races: [race("archon"), race("imp")] },
  { id: "giant", races: [race("cyclops"), race("half-giant"), race("half-orc"), race("half-titan"), race("half-troll"), race("kobold"), race("ogre"), race("snotling")] },
  { id: "undead", races: [race("einheri"), race("skeleton"), race("spectre", ["basics", "defenses", "senses", "passage", "density", "diet", "power", "birth"].map(rule => `trait-spectre-rule-${rule}`)), race("zombie")] },
  { id: "other", races: [race("beastman"), race("boit"), DRACONIAN, race("ent", ["basics", "growth", "digging", "fire", "diet", "forest", "power", "birth"].map(rule => `trait-ent-rule-${rule}`)), race("golem"), race("klackon"), race("kutar"), race("mindflayer"), race("tonberry", ["basics", "speed", "damage", "attacks", "confusion", "birth"].map(rule => `trait-tonberry-rule-${rule}`)), race("yeek")] },
] as const satisfies readonly RaceGroup[];

type GroupEntry = (typeof RACE_GROUPS)[number]["races"][number];
export type PlaytestRaceId = Exclude<GroupEntry, { children: unknown }>["id"] | (typeof DRACONIAN_RACES)[number]["id"];
export const CREATION_RACES = RACE_GROUPS.flatMap(group => group.races.flatMap<RaceLeaf>(entry => "children" in entry ? entry.children : [entry]));
export const PLAYTEST_RACE_IDS = CREATION_RACES.map(race => race.id as PlaytestRaceId);

export class RaceMenu {
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
  #selected: RaceLeaf = HUMAN;
  #group: RaceGroup = RACE_GROUPS[0];
  #subraces = false;
  #pending = false;
  #viewed: RaceOption = HUMAN;
  #busy = false;

  constructor(root: HTMLElement, localization: Localization, onChange: () => void, onBack: () => void) {
    this.#root = root;
    this.#localization = localization;
    this.#onChange = onChange;
    this.#onBack = onBack;
    this.#groups = root.querySelector<HTMLElement>("#session-race-groups")!;
    this.#options = root.querySelector<HTMLElement>("#session-race-options")!;
    this.#path = root.querySelector<HTMLElement>("#session-race-path")!;
    this.#backButton = root.querySelector<HTMLButtonElement>("#session-race-back")!;
    this.#pendingNote = root.querySelector<HTMLElement>("#session-race-pending")!;
    this.#title = root.querySelector<HTMLElement>("#session-race-detail-title")!;
    this.#description = root.querySelector<HTMLElement>("#session-race-description")!;
    this.#notes = root.querySelector<HTMLElement>("#session-race-notes")!;
  }

  get raceId(): PlaytestRaceId { return this.#selected.id as PlaytestRaceId; }
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
    this.#subraces = DRACONIAN_RACES.some(race => race.id === this.#selected.id);
    this.#group = RACE_GROUPS.find(group => group.races.some(race => race.id === this.#selected.id || ("children" in race && this.#subraces)))!;
    this.#pending = false;
    this.#viewed = this.#selected;
    this.localize();
  }

  localize(): void {
    const active = this.#root.ownerDocument.activeElement as HTMLElement | null;
    const focusedId = active?.dataset.raceId;
    const focusedGroup = active?.dataset.raceGroup;
    this.#groups.replaceChildren(...RACE_GROUPS.map(group => {
      const button = this.#button(this.#localization.format(`session-race-category-${group.id}`));
      button.dataset.raceGroup = group.id;
      button.setAttribute("aria-pressed", String(group.id === this.#group.id));
      button.tabIndex = group.id === this.#group.id ? 0 : -1;
      return button;
    }));
    this.#options.replaceChildren(...this.#entries.map(entry => {
      const button = this.#button(this.#localization.format(entry.nameKey));
      button.dataset.raceId = entry.id;
      button.tabIndex = entry.id === this.#viewed.id ? 0 : -1;
      button.setAttribute("aria-describedby", "session-race-description");
      if ("children" in entry) {
        button.setAttribute("aria-label", this.#localization.format("session-race-open-subraces", { name: this.#localization.format(entry.nameKey) }));
        button.classList.add("session-race-parent");
      } else button.setAttribute("aria-pressed", String(entry.id === this.#selected.id));
      return button;
    }));
    this.#path.textContent = [this.#localization.format("session-race-label"), this.#localization.format(`session-race-category-${this.#group.id}`), ...(this.#subraces ? [this.#localization.format(DRACONIAN.nameKey)] : [])].join(" › ");
    this.#backButton.textContent = this.#localization.format(this.#subraces ? "session-race-back" : "session-race-back-overview");
    this.#pendingNote.hidden = !this.#pending;
    this.#preview(this.#viewed);
    if (focusedId) this.#optionButton(focusedId)?.focus();
    if (focusedGroup) this.#groups.querySelector<HTMLButtonElement>(`[data-race-group="${focusedGroup}"]`)?.focus();
  }

  #button(label: string): HTMLButtonElement {
    const button = this.#root.ownerDocument.createElement("button");
    button.type = "button";
    button.textContent = label;
    button.disabled = this.#busy;
    return button;
  }

  get #entries(): readonly RaceOption[] { return this.#subraces ? DRACONIAN_RACES : this.#group.races; }
  #optionButton(id: string): HTMLButtonElement | null { return this.#options.querySelector(`[data-race-id="${id}"]`); }

  #name(race: RaceOption): string {
    const name = this.#localization.format(race.nameKey);
    return DRACONIAN_RACES.some(leaf => leaf.id === race.id) ? `${this.#localization.format(DRACONIAN.nameKey)} · ${name}` : name;
  }

  #preview(entry: RaceOption): void {
    this.#viewed = entry;
    this.#title.textContent = this.#name(entry);
    this.#description.textContent = this.#localization.format(entry.descriptionKey);
    this.#notes.replaceChildren(...entry.notes.map(key => {
      const item = this.#root.ownerDocument.createElement("li");
      item.textContent = this.#localization.format(key);
      return item;
    }));
    this.#notes.hidden = entry.notes.length === 0;
    this.#root.querySelector<HTMLElement>("#session-race-details")!.scrollTop = 0;
  }

  readonly #click = (event: MouseEvent): void => {
    if (this.#busy || !(event.target instanceof Element)) return;
    const button = event.target.closest<HTMLButtonElement>("button");
    if (!button) return;
    if (button === this.#backButton) { this.back(); return; }
    const group = RACE_GROUPS.find(group => group.id === button.dataset.raceGroup);
    if (group) {
      this.#group = group;
      this.#subraces = false;
      this.#pending = false;
      this.#viewed = group.races.find(race => race.id === this.#selected.id) ?? group.races[0];
      this.localize();
      this.#optionButton(this.#viewed.id)!.focus();
    } else {
      const entry = this.#entries.find(entry => entry.id === button.dataset.raceId);
      if (!entry) return;
      if ("children" in entry) {
        this.#subraces = true;
        this.#pending = true;
        this.#viewed = DRACONIAN_RACES.find(race => race.id === this.#selected.id) ?? DRACONIAN_RACES[0]!;
        this.localize();
        this.#optionButton(this.#viewed.id)!.focus();
      } else {
        this.#selected = entry;
        this.#pending = false;
        this.#pendingNote.hidden = true;
        for (const option of this.#options.querySelectorAll<HTMLButtonElement>("[aria-pressed]")) option.setAttribute("aria-pressed", String(option.dataset.raceId === entry.id));
        this.#preview(entry);
      }
    }
    this.#onChange();
  };

  back(): void {
    if (!this.#subraces) { this.#onBack(); return; }
    this.#subraces = false;
    this.#pending = false;
    this.#viewed = DRACONIAN;
    this.localize();
    this.#optionButton(DRACONIAN.id)!.focus();
    this.#onChange();
  }

  readonly #focus = (event: FocusEvent): void => {
    if (this.#busy || !(event.target instanceof HTMLButtonElement)) return;
    if (event.target.dataset.raceId || event.target.dataset.raceGroup) {
      for (const button of event.target.parentElement!.querySelectorAll<HTMLButtonElement>("button")) button.tabIndex = button === event.target ? 0 : -1;
    }
    const entry = this.#entries.find(entry => entry.id === (event.target as HTMLElement).dataset.raceId);
    if (entry) this.#preview(entry);
  };

  readonly #hover = (event: PointerEvent): void => {
    if (this.#busy || !(event.target instanceof Element)) return;
    const id = event.target.closest<HTMLElement>("[data-race-id]")?.dataset.raceId;
    const entry = this.#entries.find(entry => entry.id === id);
    if (entry && entry.id !== this.#viewed.id) this.#preview(entry);
  };

  readonly #keydown = (event: KeyboardEvent): void => {
    if (this.#busy || event.isComposing) return;
    if (event.key === "Escape") {
      event.preventDefault(); event.stopPropagation(); this.back(); return;
    }
    if (!(event.target instanceof HTMLButtonElement) || !["ArrowUp", "ArrowDown", "Home", "End"].includes(event.key)) return;
    if (!event.target.dataset.raceId && !event.target.dataset.raceGroup) return;
    event.preventDefault(); event.stopPropagation();
    const buttons = [...event.target.parentElement!.querySelectorAll<HTMLButtonElement>("button")];
    const index = buttons.indexOf(event.target);
    const next = event.key === "Home" ? 0 : event.key === "End" ? buttons.length - 1 : (index + (event.key === "ArrowDown" ? 1 : buttons.length - 1)) % buttons.length;
    buttons[next]!.focus();
  };
}
