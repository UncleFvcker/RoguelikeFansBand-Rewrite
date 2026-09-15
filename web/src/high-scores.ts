// SPDX-License-Identifier: MPL-2.0
import type { Localization } from "./localization";

export interface HighScore {
  characterId: string;
  name: string;
  raceNameKey: string | null;
  classNameKey: string | null;
  level: number;
  score: number;
  turn: number;
  outcome: "dead" | "retired" | "abandoned";
  locationKey: string;
  endedAt: number;
}

export class HighScorePanel {
  readonly dialog: HTMLDialogElement;
  readonly document: Document;
  readonly localization: Localization;
  readonly load: () => Promise<HighScore[]>;
  #generation = 0;
  constructor(document: Document, localization: Localization,
    load: () => Promise<HighScore[]>) {
    this.document = document; this.localization = localization;
    this.load = load;
    this.dialog = document.createElement("dialog");
    this.dialog.id = "high-scores-dialog";
    this.dialog.className = "item-target-dialog help-knowledge-dialog";
    this.dialog.setAttribute("aria-labelledby", "high-scores-title");
    this.dialog.addEventListener("close", () => this.#generation++);
    document.body.append(this.dialog);
  }
  async open(): Promise<void> {
    const generation = ++this.#generation;
    const d = this.document, l = this.localization;
    const title = d.createElement("h2"); title.id = "high-scores-title"; title.textContent = l.format("guide-entry-scores");
    const close = d.createElement("button"); close.type = "button"; close.textContent = l.format("action-dialog-close");
    close.addEventListener("click", () => this.dialog.close());
    const status = d.createElement("p"); status.setAttribute("role", "status"); status.textContent = l.format("scores-loading");
    this.dialog.replaceChildren(title, close, status);
    if (!this.dialog.open) this.dialog.showModal();
    close.focus();
    try {
      const records = await this.load();
      if (generation !== this.#generation || !this.dialog.open) return;
      status.textContent = l.format(records.length ? "scores-scope" : "scores-empty");
      const table = d.createElement("table");
      const head = d.createElement("thead"), headings = d.createElement("tr");
      for (const key of ["rank", "name", "build", "level", "score", "turn", "outcome", "location", "date"]) {
        const cell = d.createElement("th"); cell.scope = "col"; cell.textContent = l.format(`scores-${key}`); headings.append(cell);
      }
      head.append(headings); table.append(head);
      const body = d.createElement("tbody");
      records.forEach((record, index) => {
        const row = d.createElement("tr"); row.dataset.characterId = record.characterId;
        const values = [index + 1, record.name, [record.raceNameKey, record.classNameKey].filter((key): key is string => key !== null).map(key => l.format(key)).join(" · "),
          record.level, record.score, record.turn, l.format(`scores-${record.outcome}`), l.format(record.locationKey), new Date(record.endedAt * 1000).toLocaleString(l.locale)];
        for (const value of values) { const cell = d.createElement("td"); cell.textContent = String(value); row.append(cell); }
        body.append(row);
      });
      table.append(body); const scroll = d.createElement("div"); scroll.style.overflowX = "auto";
      scroll.tabIndex = 0; scroll.setAttribute("role", "region"); scroll.setAttribute("aria-label", l.format("guide-entry-scores"));
      scroll.append(table); this.dialog.append(scroll);
    } catch (error) {
      if (generation === this.#generation && this.dialog.open) status.textContent = l.format("scores-error", { detail: String(error) });
    }
  }
}

export function confirmCharacterEnd(victorious: boolean, format: (key: string) => string,
  confirm: (message: string) => boolean, prompt: (message: string) => string | null): boolean {
  return confirm(format(victorious ? "end-retire-confirm" : "end-abandon-confirm")) && prompt(format("end-character-confirm-token")) === "@";
}
