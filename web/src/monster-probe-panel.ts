// SPDX-License-Identifier: MPL-2.0

import type { Localization, MessageKey } from "./localization";
import type {
  DamageTypeDto,
  GameEventDto,
  ProbedMonsterDto,
  ResistanceLevelDto,
} from "./protocol";

interface MonsterProbeDom {
  readonly dialog: HTMLDialogElement;
  readonly close: HTMLButtonElement;
  readonly list: HTMLOListElement;
  readonly detail: HTMLElement;
}

export class MonsterProbePanel {
  readonly #document: Document;
  readonly #window: Window;
  readonly #localization: Localization;
  readonly #contentName: (contentId: string) => string;
  readonly #damageTypeName: (damageType: DamageTypeDto) => string;
  readonly #statusName: (statusId: string) => string;
  readonly #dom: MonsterProbeDom;
  #monsters: ProbedMonsterDto[] = [];
  #selectedIndex = 0;
  #installed = false;

  constructor(options: {
    document: Document;
    window: Window;
    localization: Localization;
    contentName: (contentId: string) => string;
    damageTypeName: (damageType: DamageTypeDto) => string;
    statusName: (statusId: string) => string;
  }) {
    this.#document = options.document;
    this.#window = options.window;
    this.#localization = options.localization;
    this.#contentName = options.contentName;
    this.#damageTypeName = options.damageTypeName;
    this.#statusName = options.statusName;
    this.#dom = createMonsterProbeDom(this.#document);
  }

  install(): void {
    if (this.#installed) return;
    this.#installed = true;
    this.#dom.close.addEventListener("click", this.#close);
    this.#dom.list.addEventListener("click", this.#selectMonster);
    this.#dom.dialog.addEventListener("keydown", this.#handleKeydown);
  }

  dispose(): void {
    if (!this.#installed) return;
    this.#installed = false;
    this.#dom.close.removeEventListener("click", this.#close);
    this.#dom.list.removeEventListener("click", this.#selectMonster);
    this.#dom.dialog.removeEventListener("keydown", this.#handleKeydown);
  }

  observe(events: readonly GameEventDto[]): void {
    const monsters = latestMonsterProbe(events);
    if (!monsters) return;
    this.#monsters = [...monsters];
    this.#selectedIndex = 0;
    this.#render();
    if (!this.#dom.dialog.open) this.#dom.dialog.showModal();
    this.#window.requestAnimationFrame(() => this.#focusSelected());
  }

  close(): void {
    if (this.#dom.dialog.open) this.#dom.dialog.close();
  }

  localize(): void {
    if (this.#dom.dialog.open) this.#render();
  }

  readonly #close = (): void => this.close();

  readonly #selectMonster = (event: MouseEvent): void => {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const row = target.closest<HTMLButtonElement>("[data-monster-probe-index]");
    if (!row) return;
    this.#setSelectedIndex(Number(row.dataset.monsterProbeIndex));
  };

  readonly #handleKeydown = (event: KeyboardEvent): void => {
    if (event.key === "Escape" || event.key.toLowerCase() === "q") {
      event.preventDefault();
      event.stopPropagation();
      this.close();
      return;
    }
    const delta = event.key === "ArrowUp" ? -1 : event.key === "ArrowDown" ? 1 : 0;
    if (delta === 0) return;
    event.preventDefault();
    event.stopPropagation();
    this.#setSelectedIndex(this.#selectedIndex + delta);
  };

  #setSelectedIndex(index: number): void {
    if (!Number.isInteger(index) || this.#monsters.length === 0) return;
    this.#selectedIndex = Math.max(0, Math.min(index, this.#monsters.length - 1));
    this.#render();
    this.#focusSelected();
  }

  #render(): void {
    this.#dom.list.replaceChildren();
    if (this.#monsters.length === 0) {
      const empty = this.#document.createElement("li");
      empty.className = "monster-probe-empty";
      empty.textContent = this.#localization.format("monster-probe-empty");
      this.#dom.list.append(empty);
      this.#dom.detail.replaceChildren();
      return;
    }
    this.#monsters.forEach((monster, index) => {
      const item = this.#document.createElement("li");
      const button = this.#document.createElement("button");
      button.type = "button";
      button.className = "monster-probe-row";
      button.dataset.monsterProbeIndex = String(index);
      button.dataset.selected = String(index === this.#selectedIndex);
      button.tabIndex = index === this.#selectedIndex ? 0 : -1;
      const glyph = this.#document.createElement("span");
      glyph.className = "monster-probe-glyph";
      glyph.textContent = monster.glyph;
      const label = this.#document.createElement("span");
      label.textContent = this.#contentName(monster.kindId);
      const hp = this.#document.createElement("span");
      hp.className = "monster-probe-row-hp";
      hp.textContent = `${monster.hp}/${monster.maxHp}`;
      button.append(glyph, label, hp);
      item.append(button);
      this.#dom.list.append(item);
    });
    const selected = this.#monsters[this.#selectedIndex];
    this.#dom.detail.replaceChildren(...(selected ? this.#renderDetail(selected) : []));
  }

  #renderDetail(monster: ProbedMonsterDto): HTMLElement[] {
    const heading = this.#document.createElement("h3");
    heading.textContent = this.#contentName(monster.kindId);
    const stats = this.#document.createElement("dl");
    stats.className = "monster-probe-stats";
    this.#appendStat(stats, "monster-probe-hit-points", `${monster.hp}/${monster.maxHp}`);
    this.#appendStat(stats, "monster-probe-speed", monster.speed);
    this.#appendStat(stats, "monster-probe-armor-class", monster.armorClass);
    this.#appendStat(stats, "monster-probe-position", `${monster.position.x}, ${monster.position.y}`);
    this.#appendStat(
      stats,
      "monster-probe-alignment",
      this.#localization.format(`monster-probe-alignment-${monster.alignment}` as MessageKey),
    );
    this.#appendStat(
      stats,
      "monster-probe-faction",
      this.#localization.format(`monster-probe-faction-${monster.faction}` as MessageKey),
    );
    return [
      heading,
      stats,
      this.#detailGroup(
        "monster-probe-resistances",
        monster.resistances
          .filter((resistance) => resistance.level !== "normal")
          .map(
            (resistance) =>
              `${this.#damageTypeName(resistance.damageType)}: ${this.#resistanceName(resistance.level)}`,
          ),
      ),
      this.#detailGroup(
        "monster-probe-status-immunities",
        monster.statusImmunities.map(this.#statusName),
      ),
      this.#detailGroup(
        "monster-probe-melee",
        monster.meleeRoutine.blows.map(
          (blow) =>
            `${this.#contentName(blow.methodId)} ${blow.damage.dice}d${blow.damage.sides} (${blow.toHit >= 0 ? "+" : ""}${blow.toHit})`,
        ),
      ),
      this.#detailGroup(
        "monster-probe-abilities",
        monster.abilityIds.map(this.#contentName),
      ),
    ];
  }

  #appendStat(list: HTMLDListElement, key: MessageKey, value: string | number): void {
    const term = this.#document.createElement("dt");
    term.textContent = this.#localization.format(key);
    const detail = this.#document.createElement("dd");
    detail.textContent = String(value);
    list.append(term, detail);
  }

  #detailGroup(key: MessageKey, values: readonly string[]): HTMLElement {
    const section = this.#document.createElement("section");
    section.className = "monster-probe-group";
    const heading = this.#document.createElement("h4");
    heading.textContent = this.#localization.format(key);
    const content = this.#document.createElement("p");
    content.textContent =
      values.length > 0 ? values.join(this.#localization.locale === "zh-CN" ? "、" : ", ") : "—";
    section.append(heading, content);
    return section;
  }

  #resistanceName(level: ResistanceLevelDto): string {
    return this.#localization.format(`resistance-level-${level}` as MessageKey);
  }

  #focusSelected(): void {
    this.#dom.list
      .querySelector<HTMLButtonElement>(`[data-monster-probe-index="${this.#selectedIndex}"]`)
      ?.focus();
  }
}

export function latestMonsterProbe(
  events: readonly GameEventDto[],
): readonly ProbedMonsterDto[] | undefined {
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const outcome = events[index]?.outcome;
    if (outcome?.type === "ability-monster-probe") return outcome.resolution.monsters;
  }
  return undefined;
}

function createMonsterProbeDom(document: Document): MonsterProbeDom {
  const element = <T extends HTMLElement>(id: string): T => {
    const value = document.getElementById(id);
    if (!value) throw new Error(`Missing #${id}`);
    return value as T;
  };
  return {
    dialog: element<HTMLDialogElement>("monster-probe-dialog"),
    close: element<HTMLButtonElement>("monster-probe-close"),
    list: element<HTMLOListElement>("monster-probe-list"),
    detail: element<HTMLElement>("monster-probe-detail"),
  };
}
