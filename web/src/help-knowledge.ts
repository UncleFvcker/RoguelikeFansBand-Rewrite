// SPDX-License-Identifier: MPL-2.0
import type { AppState } from "./app-state";
import type { Localization } from "./localization";
import type { InputPreset } from "./input-controller";
import type { DiscoveryDto } from "./protocol";

export type ArchiveEntry = "objects" | "artifacts" | "egos" | "monsters" | "uniques" | "kills" | "dungeons";
type ArchiveRow = { id: string; name: string; description: string; kills: number;
  monster?: DiscoveryDto["monsters"][number]; dungeon?: DiscoveryDto["dungeons"][number] };
export function archiveRows(archive: DiscoveryDto, entry: ArchiveEntry, name: (key: string) => string, glyph: (id: string, original: string) => string = (_id, original) => original): ArchiveRow[] {
  if (entry === "objects" || entry === "artifacts" || entry === "egos") return archive[entry].map(item => ({
    id: item.id, name: item.customName ? `${item.customName} · ${name(item.nameKey)}` : name(item.nameKey),
    description: name(item.descriptionKey), kills: 0,
  }));
  if (entry === "dungeons") return archive.dungeons.map(dungeon => ({
    id: dungeon.dungeonId, name: name(dungeon.nameKey), description: "", kills: 0, dungeon,
  }));
  return archive.monsters.filter(row => entry !== "uniques" || row.monster.unique)
    .filter(row => entry !== "kills" || row.kills > 0)
    .map(row => ({ id: row.monster.kindId, name: `${glyph(row.monster.kindId, row.monster.glyph)} ${name(row.monster.nameKey)}`,
      description: row.monster.knowledge ? name(row.monster.knowledge.descriptionKey) : name("intel-unresearched"),
      kills: row.kills, monster: row,
    })).sort((a, b) => b.kills - a.kills || a.id.localeCompare(b.id));
}

// Menu order and letters follow RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c,
// src/cmd4.c::do_cmd_knowledge. Destinations describe the rewrite's available views.
export const KNOWLEDGE_GROUPS = [
  { id: "objects", entries: [["a", "artifacts"], ["o", "objects"], ["e", "egos"], ["_", "autopick"], ["c", "materials"]] },
  { id: "monsters", entries: [["m", "monsters"], ["w", "wanted"], ["u", "uniques"], ["k", "kills"], ["p", "pets"]] },
  { id: "dungeons", entries: [["d", "dungeons"], ["q", "quests"], ["t", "terrain"]] },
  { id: "self", entries: [["@", "self"], ["W", "weapon"], ["S", "shooter"], ["M", "mutations"], ["v", "virtues"], ["x", "extra"], ["H", "scores"]] },
  { id: "skills", entries: [["P", "weapon-skills"], ["s", "spell-skills"]] },
] as const;
export type KnowledgeEntry = typeof KNOWLEDGE_GROUPS[number]["entries"][number][1];
export const HELP_TOPICS = ["basics", "commands", "movement", "continuous", "targeting", "items", "magic", "pets", "intel", "character", "saving", "options"] as const;

export class HelpKnowledgePanel {
  readonly state: AppState;
  readonly localization: Localization;
  readonly document: Document;
  readonly preset: () => InputPreset;
  readonly navigate: (entry: KnowledgeEntry) => void;
  readonly contentName: (id: string) => string;
  readonly statusName: (id: string) => string;
  readonly #dialog: HTMLDialogElement;
  readonly #body: HTMLElement;
  readonly #title: HTMLElement;
  #mode: "help" | "knowledge" = "help";
  #entry: KnowledgeEntry | undefined;
  #snapshot: AppState["status"];

  constructor(state: AppState, localization: Localization, document: Document,
    preset: () => InputPreset, navigate: (entry: KnowledgeEntry) => void,
    contentName: (id: string) => string, statusName: (id: string) => string) {
    this.state = state; this.localization = localization; this.document = document;
    this.preset = preset; this.navigate = navigate;
    this.contentName = contentName; this.statusName = statusName;
    this.#dialog = document.createElement("dialog");
    this.#dialog.id = "help-knowledge-dialog";
    this.#dialog.className = "item-target-dialog help-knowledge-dialog";
    this.#dialog.tabIndex = -1;
    this.#title = document.createElement("h2"); this.#title.id = "help-knowledge-title";
    this.#dialog.setAttribute("aria-labelledby", this.#title.id);
    this.#body = document.createElement("div");
    this.#dialog.append(this.#title, this.#body);
    this.#dialog.addEventListener("keydown", event => this.#key(event));
    this.#dialog.addEventListener("cancel", event => {
      if (this.#entry) { event.preventDefault(); this.#entry = undefined; this.#render(); }
    });
    document.body.append(this.#dialog);
  }

  open(mode: "help" | "knowledge"): void {
    if (!this.state.status || this.state.busy || (this.state.commandBlocked && !this.state.playerDead && !this.state.campaignEnded) || this.state.targeting || this.state.terrainInteractionMode) return;
    this.#snapshot = this.state.status; this.#mode = mode; this.#entry = undefined;
    this.#render();
    if (!this.#dialog.open) this.#dialog.showModal();
    this.#dialog.focus();
  }
  close(): void { this.#dialog.close(); }
  reconcileStatus(): void { if (this.#snapshot !== this.state.status) this.close(); }

  #text(tag: string, key: string): HTMLElement {
    const element = this.document.createElement(tag); element.textContent = this.localization.format(key); return element;
  }
  #button(key: string, action: () => void): HTMLButtonElement {
    const button = this.document.createElement("button"); button.type = "button";
    button.textContent = this.localization.format(key); button.addEventListener("click", action); return button;
  }
  #render(): void {
    this.#title.textContent = this.localization.format(`guide-${this.#mode}`);
    this.#body.replaceChildren();
    const nav = this.document.createElement("nav"); nav.setAttribute("aria-label", this.#title.textContent);
    for (const mode of ["help", "knowledge"] as const) nav.append(this.#button(`guide-${mode}`, () => {
      this.#mode = mode; this.#entry = undefined; this.#render();
    }));
    nav.append(this.#button("action-dialog-close", () => this.close())); this.#body.append(nav);
    if (this.#mode === "help") this.#help();
    else if (this.#entry) this.#detail(this.#entry);
    else this.#knowledge();
    if (this.#dialog.open) this.#dialog.focus();
    this.#dialog.scrollTop = 0;
  }
  #help(): void {
    const preset = this.#text("p", `input-preset-${this.preset()}`); preset.className = "guide-preset";
    this.#body.append(preset, this.#text("p", "guide-help-intro"));
    const label = this.#text("label", "guide-search");
    const search = this.document.createElement("input"); search.type = "search"; search.id = "guide-search";
    label.append(search); this.#body.append(label);
    const chapters = this.document.createElement("div");
    const index = this.document.createElement("nav"); index.setAttribute("aria-label", this.localization.format("guide-contents"));
    const sections = HELP_TOPICS.map(topic => {
      const section = this.document.createElement("section"); section.id = `guide-topic-${topic}`; section.tabIndex = -1;
      section.append(this.#text("h3", `guide-topic-${topic}`));
      section.append(this.#text("p", topic === "commands" ? `controls-${this.preset()}` : `guide-text-${topic}`));
      if (topic === "items") section.append(this.#text("p", "item-selection-key-help"));
      const link = this.document.createElement("button"); link.type = "button";
      link.textContent = this.localization.format(`guide-topic-${topic}`);
      link.addEventListener("click", () => { section.focus(); section.scrollIntoView({ block: "start" }); });
      index.append(link); chapters.append(section); return { section, link };
    });
    const empty = this.#text("p", "guide-no-results"); empty.hidden = true; empty.setAttribute("role", "status");
    search.addEventListener("input", () => {
      const query = search.value.trim().toLocaleLowerCase();
      for (const { section, link } of sections) section.hidden = link.hidden = !section.textContent!.toLocaleLowerCase().includes(query);
      empty.hidden = sections.some(({ section }) => !section.hidden);
    });
    this.#body.append(index, empty, chapters);
  }
  #knowledge(): void {
    this.#body.append(this.#text("p", "guide-knowledge-intro"));
    const groups = this.document.createElement("div"); groups.className = "guide-knowledge-groups";
    for (const group of KNOWLEDGE_GROUPS) {
      const section = this.document.createElement("section"); section.append(this.#text("h3", `guide-group-${group.id}`));
      for (const [key, entry] of group.entries) {
        const button = this.#button(`guide-entry-${entry}`, () => { this.#entry = entry; this.#render(); });
        button.dataset.knowledgeEntry = entry; button.textContent = `${key} · ${button.textContent}`;
        section.append(button);
      }
      groups.append(section);
    }
    this.#body.append(groups);
  }
  #detail(entry: KnowledgeEntry): void {
    this.#body.append(this.#button("guide-back", () => { this.#entry = undefined; this.#render(); }),
      this.#text("h3", `guide-entry-${entry}`), this.#text("p", `guide-scope-${entry}`));
    if (["objects", "artifacts", "egos", "monsters", "uniques", "kills", "dungeons"].includes(entry)) {
      this.#archive(entry as ArchiveEntry);
    } else if (entry === "scores") {
      this.close(); this.navigate("scores");
    } else if (entry === "pets") {
      const player = this.state.status!.player;
      const summary = this.document.createElement("p");
      summary.textContent = this.localization.format("summon-command-status", {
        mode: this.localization.format(`summon-command-mode-${player.summonCommand?.mode ?? "follow"}`),
        count: player.petUpkeep.controlledPets, upkeep: player.petUpkeep.upkeepPercent,
      });
      const list = this.document.createElement("ul");
      for (const pet of player.pets ?? []) {
        const row = this.document.createElement("li");
        row.textContent = this.localization.format("guide-pet", { name: pet.customName ?? this.localization.format(pet.nameKey), level: pet.level }) +
          (pet.riding ? this.localization.format("pet-riding") : "");
        if (pet.highlight) row.className = "pet-highlight";
        list.append(row);
      }
      if (!list.children.length) list.append(this.#text("li", "intel-empty"));
      this.#body.append(summary, list);
    } else if (entry === "wanted") {
      const offices = this.state.status!.taskServices.flatMap(service => service.bountyOffice ? [service.bountyOffice] : []);
      if (!offices.length) this.#body.append(this.#text("p", "guide-wanted-unavailable"));
      for (const office of offices) {
        const daily = this.document.createElement("p"); daily.textContent = this.localization.format("guide-daily-target", { name: this.localization.format(office.dailyTarget.actorNameKey) });
        const list = this.document.createElement("ul");
        for (const target of office.wantedTargets) {
          const row = this.document.createElement("li"); row.textContent = this.localization.format(target.completed ? "guide-wanted-done" : "guide-wanted-open", { name: this.localization.format(target.actorNameKey) }); list.append(row);
        }
        this.#body.append(daily, list);
      }
    } else if (entry !== "kills") {
      const button = this.#button("guide-open-view", () => {
        if (this.#snapshot === this.state.status && !this.state.busy && !this.state.commandBlocked) this.navigate(entry);
      });
      button.id = "guide-open-view"; this.#body.append(button);
    }
  }
  #archive(entry: ArchiveEntry): void {
    const rows = archiveRows(this.state.status!.player.discovery, entry, key => this.localization.format(key), (id, glyph) => this.state.visualGlyph(id, glyph));
    const label = this.#text("label", "archive-search");
    const search = this.document.createElement("input"); search.type = "search"; search.id = "archive-search"; label.append(search);
    const filter = this.document.createElement("select"); filter.id = "archive-filter";
    filter.setAttribute("aria-label", this.localization.format("archive-filter"));
    for (const value of entry === "uniques" ? ["all", "alive", "dead"] : ["all"]) {
      const option = this.document.createElement("option"); option.value = value;
      option.textContent = this.localization.format(`archive-${value}`); filter.append(option);
    }
    filter.hidden = entry !== "uniques";
    const list = this.document.createElement("div"); list.id = "archive-list";
    const count = this.document.createElement("p"); count.setAttribute("aria-live", "polite");
    const render = () => {
      list.replaceChildren();
      const matches = rows.filter(row => `${row.name} ${row.description}`.toLocaleLowerCase().includes(search.value.trim().toLocaleLowerCase()))
        .filter(row => filter.value === "all" || row.monster?.alive === (filter.value === "alive"));
      count.textContent = this.localization.format("archive-count", { count: matches.length });
      for (const row of matches) {
        const detail = this.document.createElement("details"); detail.dataset.discoveryId = row.id;
        const summary = this.document.createElement("summary"); summary.textContent = row.name;
        const description = this.document.createElement("p"); description.textContent = row.description;
        detail.append(summary, description);
        const line = (key: string, value: string | number) => {
          const p = this.document.createElement("p"); p.textContent = `${this.localization.format(key)}: ${value}`; detail.append(p);
        };
        if (row.dungeon) {
          line("archive-depth", row.dungeon.maxDepth);
          line("archive-conquered", this.localization.format(row.dungeon.conquered ? "archive-yes" : "archive-no"));
        }
        if (row.monster) {
          line("archive-kills", row.kills);
          if (row.monster.alive !== null) line("archive-life", this.localization.format(row.monster.alive ? "archive-alive" : "archive-dead"));
          const knowledge = row.monster.monster.knowledge;
          if (knowledge) {
            line("monster-research-base-hp", knowledge.maxHp);
            line("monster-probe-speed", knowledge.speed);
            line("monster-probe-armor-class", knowledge.armorClass);
            line("monster-probe-resistances", knowledge.resistances.filter(r => r.level !== "normal")
              .map(r => `${this.localization.format(`damage-type-${r.damageType}-name`)}: ${this.localization.format(`resistance-level-${r.level}`)}`).join(", ") || "—");
            line("monster-probe-status-immunities", knowledge.statusImmunities.map(this.statusName).join(", ") || "—");
            line("monster-probe-melee", knowledge.meleeRoutine.blows.map(b => `${this.contentName(b.methodId)} ${b.damage.dice}d${b.damage.sides}`).join(", ") || "—");
            line("monster-probe-abilities", knowledge.abilityIds.map(this.contentName).join(", ") || "—");
          }
        }
        list.append(detail);
      }
      if (!matches.length) list.append(this.#text("p", "archive-empty"));
    };
    search.addEventListener("input", render); filter.addEventListener("change", render);
    this.#body.append(label, filter, count, list); render();
  }
  #key(event: KeyboardEvent): void {
    if (event.isComposing || event.repeat || event.ctrlKey || event.altKey || event.metaKey) return;
    if (event.key === "Escape") {
      event.preventDefault(); event.stopPropagation();
      if (this.#entry) { this.#entry = undefined; this.#render(); }
      else this.close();
      return;
    }
    if ((event.target as HTMLElement).closest("input, textarea, select, [contenteditable=true]")) return;
    if (this.#entry && event.key === "/") {
      const search = this.#body.querySelector<HTMLInputElement>("#archive-search");
      if (search) { event.preventDefault(); search.focus(); }
      return;
    }
    if (this.#mode === "help" && event.key === "/") {
      event.preventDefault(); this.#body.querySelector<HTMLInputElement>("input")!.focus(); return;
    }
    if (this.#mode !== "knowledge" || this.#entry) return;
    for (const group of KNOWLEDGE_GROUPS) for (const [key, entry] of group.entries) if (event.key === key) {
      event.preventDefault(); event.stopPropagation(); this.#entry = entry; this.#render(); return;
    }
  }
}
