// SPDX-License-Identifier: MPL-2.0

import type { Localization } from "./localization";
import type { EntityDto, GameEventDto, GameSnapshot, GameUpdate, TargetSelection } from "./protocol";

const SUMMARY_LIMIT = 5;
const COMBAT_OUTCOME_TYPES = new Set([
  "ability-area-damage",
  "ability-visible-damage",
  "ability-beam-damage",
  "ability-cone-damage",
  "damage",
  "death",
  "heal",
]);
const COMBAT_KIND_PREFIXES = ["combat.", "projectile.", "throw."];
const COMBAT_MESSAGE_SUFFIX = /-(?:damage|death|heal(?:ed)?|hit|miss|slay|slew|landed|target-unavailable)$/;

interface CombatSummaryRecord {
  readonly turn: string;
  readonly event: GameEventDto;
}

export class CombatSummaryPanel {
  readonly #list: HTMLOListElement;
  readonly #localization: Localization;
  readonly #formatEvent: (event: GameEventDto) => string;
  readonly #records: CombatSummaryRecord[] = [];
  readonly #targetTitle: HTMLElement;
  readonly #targetHealth: HTMLElement;
  readonly #contentName: (id: string) => string;
  #status: GameSnapshot | GameUpdate | undefined;
  #target: TargetSelection | undefined;
  #lastAttack: EntityDto | undefined;

  constructor(options: {
    list: HTMLOListElement;
    targetTitle: HTMLElement;
    targetHealth: HTMLElement;
    contentName: (id: string) => string;
    localization: Localization;
    formatEvent: (event: GameEventDto) => string;
  }) {
    this.#list = options.list;
    this.#targetTitle = options.targetTitle;
    this.#targetHealth = options.targetHealth;
    this.#contentName = options.contentName;
    this.#localization = options.localization;
    this.#formatEvent = options.formatEvent;
    this.render();
  }

  observe(update: GameUpdate, previousEntities: readonly EntityDto[]): void {
    if (this.#status?.floorId !== update.floorId) this.#lastAttack = undefined;
    this.#status = update;
    for (const event of update.events) {
      const id = event.args.attackTarget;
      if (!id) continue;
      const live = update.entities.find(entity => entity.id === id);
      const previous = previousEntities.find(entity => entity.id === id);
      // The event identifies the actual collision, including misses and each beam victim.
      // Only a confirmed death may retain a no-longer-visible monster's health.
      this.#lastAttack = live ?? (previous && event.outcome?.type === "death"
        ? { ...previous, hp: 0 } : undefined);
    }
    const relevant = update.events.filter(isCombatSummaryEvent);
    if (relevant.length === 0) {
      this.#renderHealth();
      return;
    }
    for (const event of relevant) {
      this.#records.push({ turn: String(update.turn), event });
    }
    this.#records.splice(0, Math.max(0, this.#records.length - SUMMARY_LIMIT));
    this.render();
  }

  clear(): void {
    this.#records.length = 0;
    this.#lastAttack = undefined;
    this.#target = undefined;
    this.#status = undefined;
    this.render();
  }

  localize(): void {
    this.render();
  }

  render(): void {
    this.#renderHealth();
    if (this.#records.length === 0) {
      const empty = this.#list.ownerDocument.createElement("li");
      empty.className = "combat-summary-empty";
      empty.textContent = this.#localization.format("combat-summary-empty");
      this.#list.replaceChildren(empty);
      return;
    }
    this.#list.replaceChildren(
      ...this.#records.map(({ turn, event }) => {
        const item = this.#list.ownerDocument.createElement("li");
        item.className = "combat-summary-item";
        const turnLabel = this.#list.ownerDocument.createElement("span");
        turnLabel.className = "combat-summary-turn";
        turnLabel.textContent = turn;
        const message = this.#list.ownerDocument.createElement("span");
        message.textContent = this.#formatEvent(event);
        item.append(turnLabel, message);
        return item;
      }),
    );
  }

  renderTarget(status: GameSnapshot | GameUpdate | undefined, target: TargetSelection | undefined): void {
    if (this.#status?.floorId !== status?.floorId) this.#lastAttack = undefined;
    this.#status = status;
    this.#target = target;
    this.#renderHealth();
  }

  #renderHealth(): void {
    const entities = this.#status?.mapScale === "local" ? this.#status.entities : [];
    const target = this.#target;
    const selected = target?.type === "entity"
      ? entities.find(entity => entity.id === target.entityId)
      : target?.type === "position"
        ? entities.find(entity => entity.position.x === target.position.x && entity.position.y === target.position.y)
        : undefined;
    const last = this.#lastAttack;
    const monster = this.#status?.mapScale === "local"
      ? selected ?? (last && (entities.find(entity => entity.id === last.id) ?? (last.hp === 0 ? last : undefined)))
      : undefined;
    this.#targetTitle.textContent = this.#localization.format(selected ? "combat-target-selected" : monster ? "combat-target-last" : "combat-target-title");
    const host = this.#targetHealth;
    if (!monster) {
      host.textContent = this.#localization.format("combat-target-none");
      return;
    }
    const name = host.ownerDocument.createElement("div");
    name.className = "combat-target-name";
    name.textContent = monster.customName ?? this.#contentName(monster.kindId);
    name.title = name.textContent;
    const vitals = host.ownerDocument.createElement("div");
    vitals.className = "combat-target-vitals";
    const bar = host.ownerDocument.createElement("progress");
    bar.max = monster.maxHp;
    bar.value = Math.max(0, monster.hp);
    bar.setAttribute("aria-label", name.textContent);
    const value = host.ownerDocument.createElement("span");
    value.textContent = this.#localization.format(monster.hp <= 0 ? "combat-target-dead" : "combat-target-hp", {
      hp: Math.max(0, monster.hp), max: monster.maxHp,
    });
    vitals.append(bar, value);
    host.replaceChildren(name, vitals);
  }
}

export function isCombatSummaryEvent(event: GameEventDto): boolean {
  if (event.outcome && COMBAT_OUTCOME_TYPES.has(event.outcome.type)) return true;
  if (COMBAT_KIND_PREFIXES.some((prefix) => event.kind.startsWith(prefix))) return true;
  return COMBAT_MESSAGE_SUFFIX.test(event.messageKey);
}
