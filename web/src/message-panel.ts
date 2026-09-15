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
  readonly #format: (record: MessageRecord) => string;
  readonly #records: Array<MessageRecord & { count: number }> = [];

  constructor(limit: number, format: (record: MessageRecord) => string) {
    this.#limit = limit;
    this.#format = format;
  }

  get records(): readonly (MessageRecord & { count: number })[] {
    return this.#records;
  }

  append(record: MessageRecord): "added" | "stacked" | "evicted" {
    const last = this.#records.at(-1);
    if (last && last.kind === record.kind && this.#format(last) === this.#format(record)) {
      this.#records[this.#records.length - 1] = { ...record, count: last.count + 1 };
      return "stacked";
    }
    this.#records.push({ ...record, count: 1 });
    if (this.#records.length <= this.#limit) return "added";
    this.#records.shift();
    return "evicted";
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
    this.#history = new MessageHistory(options.historyLimit, record => this.#formatRecord(record));
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
    if (event.outcome?.type === "rest" && event.outcome.resolution.requestedTurns === 1 &&
        event.outcome.resolution.stopReason === "turn-limit") return;
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
    const followEnd = this.#isAtEnd();
    const scrollTop = this.#list.scrollTop;
    this.#list.replaceChildren();
    for (const record of this.#history.records) this.#renderRecord(record);
    if (followEnd) this.#scrollToEnd();
    else this.#list.scrollTop = scrollTop;
  }

  #append(record: MessageRecord): void {
    const followEnd = this.#isAtEnd();
    const change = this.#history.append(record);
    if (change === "evicted") {
      this.#list.firstElementChild?.remove();
    }
    const last = this.#history.records.at(-1);
    if (last) this.#renderRecord(last, change === "stacked" ? this.#list.lastElementChild ?? undefined : undefined);
    if (followEnd) this.#scrollToEnd();
  }

  #formatRecord(record: MessageRecord): string {
    return record.source === "event"
      ? this.#formatEvent(record.event)
      : this.#localization.format(record.key, this.#localizedArgs(record));
  }

  #renderRecord(record: MessageRecord & { count: number }, existing?: Element): void {
    const item = existing ?? this.#list.ownerDocument.createElement("li");
    if (existing) item.replaceChildren();
    item.className = `message message-${record.kind.replaceAll(".", "-")}`;
    const turn = this.#list.ownerDocument.createElement("span");
    turn.className = "message-turn";
    turn.textContent = record.turn;
    const content = this.#list.ownerDocument.createElement("span");
    content.textContent = this.#formatRecord(record) + (record.count > 1 ? ` ×${record.count}` : "");
    item.append(turn, content);
    if (!existing) this.#list.append(item);
  }

  #scrollToEnd(): void {
    this.#list.scrollTop = this.#list.scrollHeight;
  }

  #isAtEnd(): boolean {
    return this.#list.scrollHeight - this.#list.clientHeight - this.#list.scrollTop <= 2;
  }
}
