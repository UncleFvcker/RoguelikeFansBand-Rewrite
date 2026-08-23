// SPDX-License-Identifier: MPL-2.0

import type { Localization } from "./localization";
import type { GameEventDto } from "./protocol";

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

  constructor(options: {
    list: HTMLOListElement;
    localization: Localization;
    formatEvent: (event: GameEventDto) => string;
  }) {
    this.#list = options.list;
    this.#localization = options.localization;
    this.#formatEvent = options.formatEvent;
    this.render();
  }

  observe(events: readonly GameEventDto[], turn: number): void {
    const relevant = events.filter(isCombatSummaryEvent);
    if (relevant.length === 0) return;
    for (const event of relevant) {
      this.#records.push({ turn: String(turn), event });
    }
    this.#records.splice(0, Math.max(0, this.#records.length - SUMMARY_LIMIT));
    this.render();
  }

  clear(): void {
    this.#records.length = 0;
    this.render();
  }

  localize(): void {
    this.render();
  }

  render(): void {
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
}

export function isCombatSummaryEvent(event: GameEventDto): boolean {
  if (event.outcome && COMBAT_OUTCOME_TYPES.has(event.outcome.type)) return true;
  if (COMBAT_KIND_PREFIXES.some((prefix) => event.kind.startsWith(prefix))) return true;
  return COMBAT_MESSAGE_SUFFIX.test(event.messageKey);
}
