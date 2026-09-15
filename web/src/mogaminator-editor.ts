// SPDX-License-Identifier: MPL-2.0

import type { AppState } from "./app-state";
import type { Localization } from "./localization";
import type { Preferences, PreferenceSnapshot, PreferencesClient } from "./preferences";
import type {
  AutoGetModeDto,
  MogaminatorDiagnosticDto,
  MogaminatorDto,
  MogaminatorLineDto,
} from "./protocol";

interface MogaminatorEditorDom {
  readonly dialog: HTMLDialogElement;
  readonly close: HTMLButtonElement;
  readonly enabled: HTMLInputElement;
  readonly leaveDestroyed: HTMLInputElement;
  readonly autoGetMode: HTMLSelectElement;
  readonly locale: HTMLElement;
  readonly source: HTMLTextAreaElement;
  readonly explanation: HTMLElement;
  readonly diagnostics: HTMLOListElement;
  readonly matches: HTMLElement;
  readonly importFile: HTMLInputElement;
  readonly import: HTMLButtonElement;
  readonly export: HTMLButtonElement;
  readonly reset: HTMLButtonElement;
  readonly reload: HTMLButtonElement;
  readonly apply: HTMLButtonElement;
  readonly template: HTMLSelectElement;
  readonly templatePreview: HTMLElement;
  readonly insertTemplate: HTMLButtonElement;
}

export class MogaminatorEditor {
  readonly #window: Window;
  readonly #state: AppState;
  readonly #localization: Localization;
  readonly #preferences: PreferencesClient;
  readonly #commit: (preferences: Preferences, revision: number) => Promise<void>;
  readonly #reloadSaved: () => Promise<void>;
  #base: PreferenceSnapshot | undefined;
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
    preferences: PreferencesClient;
    commit: (preferences: Preferences, revision: number) => Promise<void>;
    reloadSaved: () => Promise<void>;
  }) {
    this.#window = options.window;
    this.#state = options.state;
    this.#localization = options.localization;
    this.#preferences = options.preferences;
    this.#commit = options.commit;
    this.#reloadSaved = options.reloadSaved;
    this.#dom = createDom(options.document);
  }

  install(): void {
    if (this.#installed) return;
    this.#installed = true;
    this.#dom.dialog.addEventListener("cancel", this.#cancel);
    this.#dom.close.addEventListener("click", this.#close);
    this.#dom.import.addEventListener("click", this.#openImport);
    this.#dom.importFile.addEventListener("change", this.#import);
    this.#dom.export.addEventListener("click", this.#export);
    this.#dom.reload.addEventListener("click", this.#reload);
    this.#dom.reset.addEventListener("click", this.#reset);
    this.#dom.apply.addEventListener("click", this.#apply);
    this.#dom.template.addEventListener("change", this.#previewTemplate);
    this.#dom.insertTemplate.addEventListener("click", this.#insertTemplate);
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
    this.#dom.reload.removeEventListener("click", this.#reload);
    this.#dom.reset.removeEventListener("click", this.#reset);
    this.#dom.dialog.removeEventListener("cancel", this.#cancel);
    this.#dom.apply.removeEventListener("click", this.#apply);
    this.#dom.template.removeEventListener("change", this.#previewTemplate);
    this.#dom.insertTemplate.removeEventListener("click", this.#insertTemplate);
    this.#dom.source.removeEventListener("input", this.#sourceChanged);
    this.#dom.source.removeEventListener("click", this.#renderLineExplanation);
    this.#dom.source.removeEventListener("keyup", this.#renderLineExplanation);
  }

  open(): void {
    if (this.#pendingApply) return;
    this.#base = structuredClone(this.#preferences.snapshot);
    if (!this.#base) return;
    const status = this.#state.status?.mogaminator;
    if (!status) return;
    this.#status = { ...status, diagnostics: [] };
    this.#renderTemplates();
    this.#loadAuthoritativeSource();
    if (!this.#dom.dialog.open) this.#dom.dialog.showModal();
    this.#window.requestAnimationFrame(() => this.#dom.source.focus());
  }

  close(): void {
    if (this.#pendingApply) return;
    if (this.#dom.dialog.open) this.#dom.dialog.close();
  }

  render(status: MogaminatorDto): void {
    const localeChanged = this.#status?.locale !== status.locale;
    this.#status = status;
    if (!this.#dom.dialog.open) return;
    if (localeChanged) this.#renderTemplates();
    if (localeChanged && !this.#pendingApply) {
      this.#loadAuthoritativeSource();
    }
    this.#renderMetadata();
    this.#renderDiagnostics();
    this.#renderLineExplanation();
  }

  localize(): void {
    if (!this.#dom.dialog.open) return;
    this.#renderTemplates();
    this.#renderMetadata();
    this.#renderDiagnostics();
    this.#renderLineExplanation();
  }

  readonly #close = (): void => this.close();

  #renderTemplates(): void {
    const selected = this.#dom.template.value;
    this.#dom.template.replaceChildren(...(this.#status?.protectionTemplates ?? []).map(template => {
      const option = this.#dom.template.ownerDocument.createElement("option");
      option.value = template.id;
      option.textContent = this.#localization.format("mogaminator-protect-" + template.id);
      return option;
    }));
    if (selected) this.#dom.template.value = selected;
    this.#previewTemplate();
  }

  readonly #previewTemplate = (): void => {
    const template = this.#status?.protectionTemplates.find(t => t.id === this.#dom.template.value);
    this.#dom.templatePreview.textContent = template?.source ?? "";
    this.#dom.insertTemplate.disabled = this.#pendingApply || !template;
  };

  readonly #insertTemplate = (): void => {
    if (this.#pendingApply) return;
    const template = this.#status?.protectionTemplates.find(t => t.id === this.#dom.template.value);
    if (!template) return;
    this.#dom.source.value = template.source + "\n" + this.#dom.source.value;
    this.#sourceChanged();
    this.#dom.source.focus();
  };
  readonly #cancel = (event: Event): void => { if (this.#pendingApply) event.preventDefault(); };

  readonly #openImport = (): void => this.#dom.importFile.click();

  readonly #import = (): void => {
    if (this.#pendingApply) return;
    const file = this.#dom.importFile.files?.[0];
    if (!file) return;
    void file
      .text()
      .then((source) => {
        if (this.#pendingApply) return;
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

  readonly #reload = (): void => {
    if (this.#pendingApply || this.#state.busy) return;
    this.#pendingApply = true; this.#setPendingControls();
    void this.#reloadSaved().then(() => {
      this.#dom.diagnostics.textContent = this.#localization.format("mogaminator-reloaded");
    }).catch(error => {
      this.#dom.diagnostics.textContent = this.#localization.format("mogaminator-preferences-error", { error: error instanceof Error ? error.message : String(error) });
    }).finally(() => { this.#pendingApply = false; this.#setPendingControls(); });
  };

  readonly #reset = (): void => {
    if (!this.#status) return;
    this.#dom.source.value = this.#status.defaultSource;
    this.#dirty = true;
    this.#renderLineExplanation();
    this.#dom.source.focus();
  };

  readonly #apply = (): void => {
    if (!this.#status || !this.#base || this.#state.busy || this.#pendingApply) return;
    this.#pendingApply = true;
    this.#setPendingControls();
    const p = this.#base.preferences;
    void this.#commit({ ...p, mogaminator: { ...p.mogaminator,
      enabled: this.#dom.enabled.checked, leaveDestroyedItems: this.#dom.leaveDestroyed.checked,
      autoGetMode: this.#dom.autoGetMode.value as AutoGetModeDto,
      [this.#localization.locale === "zh-CN" ? "zhCnSource" : "enUsSource"]: this.#dom.source.value,
    } }, this.#base.revision).then(() => {
      this.#base = structuredClone(this.#preferences.snapshot);
      this.#loadAuthoritativeSource();
    }).catch(error => {
      this.#dirty = true;
      const message = error instanceof Error ? error.message : JSON.stringify(error);
      this.#dom.diagnostics.textContent = this.#localization.format("mogaminator-preferences-error", { error: message });
    }).finally(() => { this.#pendingApply = false; this.#setPendingControls(); });
  };

  #setPendingControls(): void {
    for (const control of [this.#dom.close, this.#dom.enabled, this.#dom.leaveDestroyed,
      this.#dom.autoGetMode, this.#dom.source, this.#dom.import, this.#dom.importFile,
      this.#dom.reset, this.#dom.reload, this.#dom.apply, this.#dom.template, this.#dom.insertTemplate]) control.disabled = this.#pendingApply;
    this.#dom.dialog.setAttribute("aria-busy", String(this.#pendingApply));
  }

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
    this.#dom.autoGetMode.value = this.#status.autoGetMode;
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
    autoGetMode: element(document, "mogaminator-auto-get-mode"),
    locale: element(document, "mogaminator-locale"),
    source: element(document, "mogaminator-source"),
    explanation: element(document, "mogaminator-line-explanation"),
    diagnostics: element(document, "mogaminator-diagnostics"),
    matches: element(document, "mogaminator-match-summary"),
    importFile: element(document, "mogaminator-import-file"),
    import: element(document, "mogaminator-import"),
    export: element(document, "mogaminator-export"),
    reset: element(document, "mogaminator-reset"),
    reload: element(document, "mogaminator-reload"),
    apply: element(document, "mogaminator-apply"),
    template: element(document, "mogaminator-template"),
    templatePreview: element(document, "mogaminator-template-preview"),
    insertTemplate: element(document, "mogaminator-insert-template"),
  };
}

function element<T extends HTMLElement>(document: Document, id: string): T {
  const found = document.getElementById(id);
  if (!found) throw new Error(`Missing element #${id}`);
  return found as T;
}
