// SPDX-License-Identifier: MPL-2.0
import type { AppState } from "./app-state";
import type { Localization } from "./localization";
import type { InputPreset } from "./input-controller";
import { directionForKeyboardInput } from "./input-controller.ts";
import type { Position, ResearchMonsterDto } from "./protocol";

export type MapInquiry = "map-overview" | "map-locate" | "monster-list" | "symbol-query" | "floor-feeling";
const SVG = "http://www.w3.org/2000/svg";
const vectors: Record<string, [number, number]> = { north: [0,-1], south: [0,1], west: [-1,0], east: [1,0],
  "north-west": [-1,-1], "north-east": [1,-1], "south-west": [-1,1], "south-east": [1,1] };

export function knownMapGlyphs(state: AppState): Map<string, string> {
  const result = new Map<string, string>();
  for (const [key, cell] of state.cells) {
    if (["visible", "remembered"].includes(state.cellVisibility.get(key) ?? "hidden")) {
      result.set(key, state.visualGlyph(cell.terrainId));
    }
  }
  for (const item of state.status?.items ?? []) result.set(`${item.position.x},${item.position.y}`, state.visualGlyph(item.visual.id, item.visual.glyph));
  for (const entity of state.status?.entities ?? []) result.set(`${entity.position.x},${entity.position.y}`, state.visualGlyph(entity.kindId, entity.glyph));
  const player = state.status?.player.position;
  if (player) result.set(`${player.x},${player.y}`, state.visualGlyph(state.status!.player.kindId, "@"));
  return result;
}

export function filterMonsterRecall(monsters: readonly ResearchMonsterDto[], query: string, mode: string,
  name: (key: string) => string, glyph: (monster: ResearchMonsterDto) => string = monster => monster.glyph): ResearchMonsterDto[] {
  return monsters.filter(monster => (mode !== "unique" || monster.unique) &&
    (mode !== "normal" || !monster.unique) && (mode !== "rideable" || monster.rideable) &&
    (mode === "name" ? name(monster.nameKey).toLocaleLowerCase().includes(query.toLocaleLowerCase()) :
      mode !== "glyph" || monster.glyph === query || glyph(monster) === query));
}

export function monsterRecallAt(state: AppState, position: Position): ResearchMonsterDto | undefined {
  const status = state.status;
  if (!status || status.player.statuses.some(effect => effect.kindId === "rfb.status.hallucination")) return;
  const entity = status.entities.find(entity => entity.position.x === position.x && entity.position.y === position.y);
  return entity && status.player.monsterRecall.find(monster => monster.kindId === entity.kindId);
}

export class MapIntelligencePanel {
  readonly state: AppState;
  readonly localization: Localization;
  readonly document: Document;
  readonly preset: () => InputPreset;
  readonly contentName: (id: string) => string;
  readonly statusName: (id: string) => string;
  readonly recenter: () => void;
  readonly #dialog: HTMLDialogElement;
  readonly #body: HTMLElement;
  readonly #title: HTMLElement;
  #mode: MapInquiry = "map-overview";
  #queryMode: "glyph" | "all" | "unique" = "glyph";
  #origin: Position = { x: 0, y: 0 };
  #snapshot: AppState["status"];
  #recalling = false;

  constructor(state: AppState, localization: Localization,
    document: Document, preset: () => InputPreset,
    contentName: (id: string) => string, recenter: () => void, statusName: (id: string) => string) {
    this.state = state; this.localization = localization; this.document = document;
    this.preset = preset; this.contentName = contentName; this.recenter = recenter;
    this.statusName = statusName;
    this.#dialog = document.createElement("dialog");
    this.#dialog.id = "map-intelligence-dialog";
    this.#dialog.tabIndex = -1;
    this.#dialog.className = "item-target-dialog map-intelligence-dialog";
    this.#title = document.createElement("h2"); this.#title.id = "map-intelligence-title";
    this.#dialog.setAttribute("aria-labelledby", this.#title.id);
    this.#body = document.createElement("div");
    this.#dialog.append(this.#title, this.#body);
    this.#dialog.addEventListener("keydown", event => this.#key(event));
    document.body.append(this.#dialog);
  }

  open(mode: MapInquiry, queryMode: "glyph" | "all" | "unique" = "glyph"): void {
    if (!this.state.status || this.state.busy || this.state.commandBlocked) return;
    this.#recalling = false;
    this.#mode = mode; this.#queryMode = queryMode; this.#snapshot = this.state.status;
    this.#origin = { ...this.state.status.player.position };
    this.#render();
    if (!this.#dialog.open) this.#dialog.showModal();
    (this.#body.querySelector("input") ?? this.#body.querySelector("button"))?.focus();
  }
  close(): void { this.#dialog.close(); }
  reconcileStatus(): void { if (this.#snapshot !== this.state.status) this.close(); }

  openMonsterRecall(position: Position): void {
    // RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c, xtra2.c: look recalls the apparent race.
    if (this.state.busy || this.state.commandBlocked) return;
    const monster = monsterRecallAt(this.state, position);
    if (!monster) return;
    this.#snapshot = this.state.status;
    this.#recalling = true;
    this.#title.textContent = this.localization.format(monster.nameKey);
    const detail = this.#monsterDetail(monster);
    detail.open = true;
    this.#body.replaceChildren(this.#button("action-dialog-close", () => this.close()), detail);
    if (!this.#dialog.open) this.#dialog.showModal();
    this.#body.querySelector("button")?.focus();
  }

  #button(key: string, action: () => void): HTMLButtonElement {
    const button = this.document.createElement("button"); button.type = "button";
    button.textContent = this.localization.format(key); button.addEventListener("click", action); return button;
  }
  #render(): void {
    this.#title.textContent = this.localization.format(`intel-${this.#mode}`);
    this.#body.replaceChildren();
    if (this.#dialog.open) this.#dialog.focus();
    this.#body.append(this.#button("action-dialog-close", () => this.close()));
    if (this.#mode === "floor-feeling") {
      const text = this.document.createElement("p");
      text.textContent = this.localization.format(this.state.status!.player.floorFeelingMessageKey);
      const help = this.document.createElement("p"); help.textContent = this.localization.format("intel-feeling-help");
      this.#body.append(text, help); return;
    }
    if (this.#mode === "monster-list") { this.#monsters(); return; }
    if (this.#mode === "symbol-query") { this.#symbols(); return; }
    this.#map();
  }

  #map(): void {
    const overview = this.#mode === "map-overview";
    const width = overview ? this.state.mapWidth : Math.min(31, this.state.mapWidth);
    const height = overview ? this.state.mapHeight : Math.min(19, this.state.mapHeight);
    const x = overview ? 0 : Math.max(0, Math.min(this.state.mapWidth - width, this.#origin.x - Math.floor(width / 2)));
    const y = overview ? 0 : Math.max(0, Math.min(this.state.mapHeight - height, this.#origin.y - Math.floor(height / 2)));
    const help = this.document.createElement("p");
    help.textContent = this.localization.format(overview ? "intel-map-help" : "intel-locate-help", { x, y });
    this.#body.append(help);
    const svg = this.document.createElementNS(SVG, "svg"); svg.setAttribute("viewBox", `0 0 ${width * 10} ${height * 16}`);
    svg.setAttribute("role", "img"); svg.setAttribute("aria-label", this.#title.textContent!);
    const glyphs = knownMapGlyphs(this.state);
    for (let row = 0; row < height; row++) {
      const text = this.document.createElementNS(SVG, "text");
      text.setAttribute("x", "0"); text.setAttribute("y", String((row + 1) * 16 - 3));
      text.setAttribute("xml:space", "preserve");
      text.setAttribute("textLength", String(width * 10));
      text.setAttribute("lengthAdjust", "spacingAndGlyphs");
      text.textContent = Array.from({ length: width }, (_, col) => glyphs.get(`${col + x},${row + y}`) ?? " ").join("");
      svg.append(text);
    }
    if (overview) {
      const player = this.state.status!.player.position;
      const marker = this.document.createElementNS(SVG, "circle");
      marker.setAttribute("cx", String((player.x + 0.5) * 10));
      marker.setAttribute("cy", String((player.y + 0.5) * 16));
      marker.setAttribute("r", String(Math.max(10, width / 8)));
      marker.setAttribute("fill", "none"); marker.setAttribute("stroke", "#f6c773");
      marker.setAttribute("stroke-width", "2"); marker.setAttribute("vector-effect", "non-scaling-stroke");
      svg.append(marker);
    }
    this.#body.append(svg);
    if (overview) this.#body.append(this.#button("intel-map-locate", () => { this.#mode = "map-locate"; this.#render(); }));
    if (!overview) {
      const controls = this.document.createElement("div"); controls.className = "intel-directions";
      for (const [direction, [dx, dy]] of Object.entries(vectors)) {
        controls.append(this.#button(`direction-${direction}`, () => this.#pan(dx, dy)));
      }
      this.#body.append(controls, this.#button("intel-map-center", () => { this.#origin = { ...this.state.status!.player.position }; this.#render(); }));
    }
  }
  #pan(dx: number, dy: number): void {
    this.#origin = { x: Math.max(0, Math.min(this.state.mapWidth - 1, this.#origin.x + dx * 15)),
      y: Math.max(0, Math.min(this.state.mapHeight - 1, this.#origin.y + dy * 9)) };
    this.#render();
  }
  #monsters(): void {
    const status = this.state.status!;
    const list = this.document.createElement("ol");
    const player = status.player.position;
    const distance = (position: Position) => Math.max(Math.abs(position.x - player.x), Math.abs(position.y - player.y));
    const monsters = (status.player.statuses.some(effect => effect.kindId === "rfb.status.hallucination") ? [] : [...status.entities])
      .sort((a, b) => distance(a.position) - distance(b.position) || a.id.localeCompare(b.id));
    for (const monster of monsters) {
      const row = this.document.createElement("li");
      row.dataset.entityId = monster.id;
      row.classList.toggle("intel-pet", monster.highlightList);
      row.textContent = ` ${monster.customName ?? this.contentName(monster.kindId)} · ${this.localization.format(this.state.display.monsterDistance ? "intel-monster-position" : "intel-monster-offset", {
        distance: distance(monster.position), x: monster.position.x - player.x, y: monster.position.y - player.y })}`;
      const symbol = this.document.createElement("span"); this.state.paintVisual(symbol, monster.kindId, monster.glyph); row.prepend(symbol);
      const show = this.#button("intel-map-locate", () => { this.#origin = { ...monster.position }; this.#mode = "map-locate"; this.#render(); });
      row.append(show); list.append(row);
    }
    if (!monsters.length) list.textContent = this.localization.format("intel-empty");
    this.#body.append(list);
  }
  #symbols(): void {
    const label = this.document.createElement("label"); label.textContent = this.localization.format("intel-symbol-label");
    const query = this.document.createElement("input"); query.id = "intel-symbol-input";
    const mode = this.document.createElement("select"); mode.setAttribute("aria-label", this.localization.format("intel-symbol-mode"));
    for (const value of ["glyph", "name", "all", "unique", "normal", "rideable"]) {
      const option = this.document.createElement("option"); option.value = value; option.textContent = this.localization.format(`intel-query-${value}`); mode.append(option);
    }
    const result = this.document.createElement("div"); result.setAttribute("aria-live", "polite");
    mode.value = this.#queryMode;
    const render = () => {
      result.replaceChildren();
      if (mode.value === "glyph" && [...query.value].length === 1) {
        const key = `intel-symbol-${query.value.codePointAt(0)}`;
        const text = this.document.createElement("p");
        text.textContent = this.localization.hasMessage(this.localization.locale, key) ? this.localization.format(key) : this.localization.format("intel-symbol-unknown");
        result.append(text);
      }
      for (const monster of filterMonsterRecall(this.state.status!.player.monsterRecall, query.value, mode.value, key => this.localization.format(key), monster => this.state.visualGlyph(monster.kindId, monster.glyph))) {
        result.append(this.#monsterDetail(monster));
      }
      if (!result.children.length) result.textContent = this.localization.format("intel-empty");
    };
    query.addEventListener("input", render); mode.addEventListener("change", render);
    query.addEventListener("keydown", event => {
      if (!event.ctrlKey || event.altKey || event.metaKey || event.isComposing) return;
      const target = ({ a: "all", u: "unique", n: "normal", r: "rideable", m: "name" } as Record<string, string>)[event.key.toLowerCase()];
      if (target) { event.preventDefault(); event.stopPropagation(); mode.value = target; render(); }
    });
    label.append(query); this.#body.append(label, mode, result); render();
  }
  #monsterDetail(monster: ResearchMonsterDto): HTMLDetailsElement {
    const detail = this.document.createElement("details");
    const summary = this.document.createElement("summary"); summary.textContent = ` ${this.localization.format(monster.nameKey)}`;
    const glyph = this.document.createElement("span"); this.state.paintVisual(glyph, monster.kindId, monster.glyph); summary.prepend(glyph);
    detail.append(summary);
    const text = this.document.createElement("p");
    const knowledge = monster.knowledge;
    text.textContent = knowledge ? this.localization.format(knowledge.descriptionKey) +
      ` · HP ${knowledge.maxHp} · AC ${knowledge.armorClass} · ${this.localization.format("intel-speed", { speed: knowledge.speed })}` : this.localization.format("intel-unresearched");
    detail.append(text);
    if (knowledge) {
      const line = (key: string, values: string[]) => {
        const p = this.document.createElement("p");
        p.textContent = `${this.localization.format(key)}: ${values.join(", ") || "—"}`;
        detail.append(p);
      };
      line("monster-probe-resistances", knowledge.resistances.filter(r => r.level !== "normal")
        .map(r => `${this.localization.format(`damage-type-${r.damageType}-name`)}: ${this.localization.format(`resistance-level-${r.level}`)}`));
      line("monster-probe-status-immunities", knowledge.statusImmunities.map(this.statusName));
      line("monster-probe-melee", knowledge.meleeRoutine.blows.map(b => `${this.contentName(b.methodId)} ${b.damage.dice}d${b.damage.sides}`));
      line("monster-probe-abilities", knowledge.abilityIds.map(this.contentName));
    }
    return detail;
  }
  #key(event: KeyboardEvent): void {
    if (event.isComposing || event.repeat) return;
    if (event.key === "Escape" || (this.#recalling && event.key === "r" && !event.ctrlKey && !event.altKey && !event.metaKey)) {
      event.preventDefault(); event.stopPropagation(); this.close(); return;
    }
    if (this.#recalling) return;
    if (this.#mode !== "map-locate") return;
    if (event.ctrlKey && event.key.toLowerCase() === "v") {
      event.preventDefault(); this.#origin = { ...this.state.status!.player.position }; this.recenter(); this.#render(); return;
    }
    if (event.key === "5") { event.preventDefault(); this.close(); return; }
    if (event.ctrlKey || event.metaKey || event.altKey) return;
    const direction = ({ ArrowUp: "north", ArrowDown: "south", ArrowLeft: "west", ArrowRight: "east" } as Record<string,string>)[event.key]
      ?? directionForKeyboardInput(event, this.preset());
    if (direction && vectors[direction]) { event.preventDefault(); this.#pan(...vectors[direction]); }
  }
}
