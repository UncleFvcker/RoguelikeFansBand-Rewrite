// SPDX-License-Identifier: MPL-2.0

import type { AppState } from "./app-state";
import type { Localization } from "./localization";
import type {
  GameCommand,
  MogaminatorDiagnosticDto,
  MogaminatorDto,
  MogaminatorLineDto,
} from "./protocol";

interface MogaminatorEditorDom {
  readonly dialog: HTMLDialogElement;
  readonly close: HTMLButtonElement;
  readonly enabled: HTMLInputElement;
  readonly leaveDestroyed: HTMLInputElement;
  readonly locale: HTMLElement;
  readonly source: HTMLTextAreaElement;
  readonly explanation: HTMLElement;
  readonly diagnostics: HTMLOListElement;
  readonly matches: HTMLElement;
  readonly importFile: HTMLInputElement;
  readonly import: HTMLButtonElement;
  readonly export: HTMLButtonElement;
  readonly reset: HTMLButtonElement;
  readonly apply: HTMLButtonElement;
}

export class MogaminatorEditor {
  readonly #window: Window;
  readonly #state: AppState;
  readonly #localization: Localization;
  readonly #dispatch: (command: GameCommand) => Promise<void>;
  readonly #dom: MogaminatorEditorDom;
  #status: MogaminatorDto | undefined;
  #pendingApply = false;
  #dirty = false;
  #installed = false;

  constructor(options: {
    document: Document;
    window: Window;
    state: AppState;
    localization: Localization;
    dispatch: (command: GameCommand) => Promise<void>;
  }) {
    this.#window = options.window;
    this.#state = options.state;
    this.#localization = options.localization;
    this.#dispatch = options.dispatch;
    this.#dom = createDom(options.document);
  }

  install(): void {
    if (this.#installed) return;
    this.#installed = true;
    this.#dom.close.addEventListener("click", this.#close);
    this.#dom.import.addEventListener("click", this.#openImport);
    this.#dom.importFile.addEventListener("change", this.#import);
    this.#dom.export.addEventListener("click", this.#export);
    this.#dom.reset.addEventListener("click", this.#reset);
    this.#dom.apply.addEventListener("click", this.#apply);
    this.#dom.source.addEventListener("input", this.#sourceChanged);
    this.#dom.source.addEventListener("click", this.#renderLineExplanation);
    this.#dom.source.addEventListener("keyup", this.#renderLineExplanation);
  }

  dispose(): void {
    if (!this.#installed) return;
    this.#installed = false;
    this.#dom.close.removeEventListener("click", this.#close);
    this.#dom.import.removeEventListener("click", this.#openImport);
    this.#dom.importFile.removeEventListener("change", this.#import);
    this.#dom.export.removeEventListener("click", this.#export);
    this.#dom.reset.removeEventListener("click", this.#reset);
    this.#dom.apply.removeEventListener("click", this.#apply);
    this.#dom.source.removeEventListener("input", this.#sourceChanged);
    this.#dom.source.removeEventListener("click", this.#renderLineExplanation);
    this.#dom.source.removeEventListener("keyup", this.#renderLineExplanation);
  }

  open(): void {
    const status = this.#state.status?.mogaminator;
    if (!status) return;
    this.#status = { ...status, diagnostics: [] };
    this.#loadAuthoritativeSource();
    if (!this.#dom.dialog.open) this.#dom.dialog.showModal();
    this.#window.requestAnimationFrame(() => this.#dom.source.focus());
  }

  close(): void {
    if (this.#dom.dialog.open) this.#dom.dialog.close();
  }

  render(status: MogaminatorDto): void {
    const localeChanged = this.#status?.locale !== status.locale;
    const failedApply = this.#pendingApply && status.diagnostics.length > 0;
    this.#status = status;
    if (!this.#dom.dialog.open) return;
    if (localeChanged || (this.#pendingApply && !failedApply)) {
      this.#loadAuthoritativeSource();
    }
    this.#pendingApply = false;
    this.#renderMetadata();
    this.#renderDiagnostics();
    this.#renderLineExplanation();
  }

  localize(): void {
    if (!this.#dom.dialog.open) return;
    this.#renderMetadata();
    this.#renderDiagnostics();
    this.#renderLineExplanation();
  }

  readonly #close = (): void => this.close();

  readonly #openImport = (): void => this.#dom.importFile.click();

  readonly #import = (): void => {
    const file = this.#dom.importFile.files?.[0];
    if (!file) return;
    void file
      .text()
      .then((source) => {
        this.#dom.source.value = source;
        this.#sourceChanged();
        this.#dom.source.focus();
      })
      .catch(() => this.#window.alert(this.#localization.format("mogaminator-import-error")))
      .finally(() => {
        this.#dom.importFile.value = "";
      });
  };

  readonly #export = (): void => {
    const url = URL.createObjectURL(
      new Blob([this.#dom.source.value], { type: "text/plain;charset=utf-8" }),
    );
    const link = this.#dom.source.ownerDocument.createElement("a");
    link.href = url;
    link.download = `mogaminator-${this.#localization.locale}.prf`;
    link.click();
    URL.revokeObjectURL(url);
  };

  readonly #reset = (): void => {
    if (!this.#status) return;
    this.#dom.source.value = this.#status.defaultSource;
    this.#dirty = true;
    this.#renderLineExplanation();
    this.#dom.source.focus();
  };

  readonly #apply = (): void => {
    if (!this.#status || this.#state.busy) return;
    this.#pendingApply = true;
    void this.#dispatch({
      type: "configure-mogaminator",
      enabled: this.#dom.enabled.checked,
      leaveDestroyedItems: this.#dom.leaveDestroyed.checked,
      locale: this.#localization.locale,
      source: this.#dom.source.value,
    });
  };

  readonly #sourceChanged = (): void => {
    this.#dirty = true;
    this.#renderLineExplanation();
  };

  readonly #renderLineExplanation = (): void => {
    const status = this.#status;
    if (!status) return;
    if (this.#dirty) {
      this.#dom.explanation.textContent = this.#localization.format(
        "mogaminator-line-draft",
      );
      return;
    }
    const lineNumber = currentLineNumber(this.#dom.source);
    const line = status.lines.find((candidate) => candidate.lineNumber === lineNumber);
    this.#dom.explanation.textContent = line
      ? this.#describeLine(line)
      : this.#localization.format("mogaminator-line-empty", { line: lineNumber });
  };

  #loadAuthoritativeSource(): void {
    if (!this.#status) return;
    this.#dom.enabled.checked = this.#status.enabled;
    this.#dom.leaveDestroyed.checked = this.#status.leaveDestroyedItems;
    this.#dom.source.value = this.#status.source;
    this.#dirty = false;
    this.#renderMetadata();
    this.#renderDiagnostics();
    this.#renderLineExplanation();
  }

  #renderMetadata(): void {
    if (!this.#status) return;
    this.#dom.locale.textContent = this.#localization.format("mogaminator-locale", {
      locale: this.#localization.format(
        `mogaminator-locale-${this.#status.locale.toLowerCase()}`,
      ),
    });
    this.#dom.matches.textContent = this.#localization.format("mogaminator-match-summary", {
      count: this.#status.matches.length,
    });
  }

  #renderDiagnostics(): void {
    const diagnostics = this.#status?.diagnostics ?? [];
    this.#dom.diagnostics.replaceChildren(
      ...diagnostics.map((diagnostic) => {
        const item = this.#dom.diagnostics.ownerDocument.createElement("li");
        item.textContent = this.#formatDiagnostic(diagnostic);
        return item;
      }),
    );
  }

  #formatDiagnostic(diagnostic: MogaminatorDiagnosticDto): string {
    const key = diagnostic.code.replace("mogaminator.", "mogaminator-diagnostic-");
    return this.#localization.format(key, {
      line: diagnostic.line,
      column: diagnostic.column,
      argument: diagnostic.arguments.join(", "),
    });
  }

  #describeLine(line: MogaminatorLineDto): string {
    if (line.kind !== "rule") {
      return this.#localization.format(`mogaminator-line-${line.kind}`, {
        line: line.lineNumber,
      });
    }
    return this.#localization.format("mogaminator-line-rule", {
      line: line.lineNumber,
      action: this.#localization.format(
        `mogaminator-action-${line.action?.disposition ?? "pick-up"}`,
      ),
      predicates: line.predicateCount,
      search: line.search ?? this.#localization.format("mogaminator-search-any"),
    });
  }
}

function currentLineNumber(source: HTMLTextAreaElement): number {
  return source.value.slice(0, source.selectionStart).split("\n").length;
}

function createDom(document: Document): MogaminatorEditorDom {
  return {
    dialog: element(document, "mogaminator-dialog"),
    close: element(document, "mogaminator-close"),
    enabled: element(document, "mogaminator-enabled"),
    leaveDestroyed: element(document, "mogaminator-leave-destroyed"),
    locale: element(document, "mogaminator-locale"),
    source: element(document, "mogaminator-source"),
    explanation: element(document, "mogaminator-line-explanation"),
    diagnostics: element(document, "mogaminator-diagnostics"),
    matches: element(document, "mogaminator-match-summary"),
    importFile: element(document, "mogaminator-import-file"),
    import: element(document, "mogaminator-import"),
    export: element(document, "mogaminator-export"),
    reset: element(document, "mogaminator-reset"),
    apply: element(document, "mogaminator-apply"),
  };
}

function element<T extends HTMLElement>(document: Document, id: string): T {
  const found = document.getElementById(id);
  if (!found) throw new Error(`Missing element #${id}`);
  return found as T;
}
