// SPDX-License-Identifier: MPL-2.0

import type { Localization, LocalizationArgs, MessageKey } from "./localization";
import type { GameEventDto } from "./protocol";

export type MessageRecord =
  | {
      source: "key";
      turn: string;
      kind: string;
      key: MessageKey;
      args?: Record<string, string | number>;
    }
  | { source: "event"; turn: string; kind: string; event: GameEventDto };

export class MessageHistory {
  readonly #limit: number;
  readonly #records: MessageRecord[] = [];

  constructor(limit: number) {
    this.#limit = limit;
  }

  get records(): readonly MessageRecord[] {
    return this.#records;
  }

  append(record: MessageRecord): boolean {
    this.#records.push(record);
    if (this.#records.length <= this.#limit) return false;
    this.#records.shift();
    return true;
  }

  clear(): void {
    this.#records.length = 0;
  }
}

export class MessagePanel {
  readonly #list: HTMLOListElement;
  readonly #localization: Localization;
  readonly #formatEvent: (event: GameEventDto) => string;
  readonly #currentTurn: () => string;
  readonly #localizedArgs: (
    record: Extract<MessageRecord, { source: "key" }>,
  ) => LocalizationArgs | undefined;
  readonly #history: MessageHistory;

  constructor(options: {
    list: HTMLOListElement;
    localization: Localization;
    formatEvent: (event: GameEventDto) => string;
    currentTurn: () => string;
    localizedArgs: (
      record: Extract<MessageRecord, { source: "key" }>,
    ) => LocalizationArgs | undefined;
    historyLimit: number;
  }) {
    this.#list = options.list;
    this.#localization = options.localization;
    this.#formatEvent = options.formatEvent;
    this.#currentTurn = options.currentTurn;
    this.#localizedArgs = options.localizedArgs;
    this.#history = new MessageHistory(options.historyLimit);
  }

  addLocalized(
    key: MessageKey,
    args: Record<string, string | number> | undefined,
    kind: string,
  ): void {
    this.#append({
      source: "key",
      turn: this.#currentTurn(),
      kind,
      key,
      args,
    });
  }

  addEvent(event: GameEventDto): void {
    this.#append({
      source: "event",
      turn: this.#currentTurn(),
      kind: event.kind,
      event,
    });
  }

  clear(): void {
    this.#history.clear();
    this.render();
  }

  render(): void {
    this.#list.replaceChildren();
    for (const record of this.#history.records) this.#renderRecord(record);
    this.#scrollToEnd();
  }

  #append(record: MessageRecord): void {
    if (this.#history.append(record)) {
      this.#list.firstElementChild?.remove();
    }
    this.#renderRecord(record);
    this.#scrollToEnd();
  }

  #renderRecord(record: MessageRecord): void {
    const item = this.#list.ownerDocument.createElement("li");
    item.className = `message message-${record.kind.replaceAll(".", "-")}`;
    const turn = this.#list.ownerDocument.createElement("span");
    turn.className = "message-turn";
    turn.textContent = record.turn;
    const content = this.#list.ownerDocument.createElement("span");
    content.textContent =
      record.source === "event"
        ? this.#formatEvent(record.event)
        : this.#localization.format(record.key, this.#localizedArgs(record));
    item.append(turn, content);
    this.#list.append(item);
  }

  #scrollToEnd(): void {
    this.#list.scrollTop = this.#list.scrollHeight;
  }
}
