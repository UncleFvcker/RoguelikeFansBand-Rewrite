// SPDX-License-Identifier: MPL-2.0
import { VisualEditor } from "./visual-editor.ts";
import { parsePrf, exportPrf, type PrfPreview } from "./prf-preferences.ts";
import type { VisualPreferences } from "./visual-preferences";
import type { AppDom } from "./app-dom";
import type { AppState } from "./app-state";
import type { InputPreset } from "./input-controller";
import type { Localization, MessageKey } from "./localization";
import type { CameraMode, ZoomLevel } from "./camera.ts";
import type { TilesetWarning } from "./tileset-runtime";
import { defaultPreferences, parsePreferences, behaviorPreferences, type Preferences, type PreferencesClient } from "./preferences.ts";

import { DISPLAY_FIELDS, DEFAULT_DISPLAY, HUD_DISPLAY_FIELDS } from "./display-preferences.ts";

export type TilesetPreset = "ascii" | "image";
const TRAVEL_CONTROLS = { alwaysPickup: "travel-always-pickup", autoDetectTraps: "travel-auto-detect", autoMapArea: "travel-auto-map", disturbTrapDetect: "travel-disturb-detect" } as const;
export const TILESET_MANIFESTS: Readonly<Record<TilesetPreset, string>> = {
  ascii: "/tilesets/ascii-default/tileset.json", image: "/tilesets/rfb-pixel-28/tileset.json",
};
type SettingsDom = Pick<AppDom, "inputPresetSelect" | "tilesetPresetSelect" | "cameraModeSelect" | "zoomSelect" | "controlsHelp" | "languageSelect">;
interface SettingsRenderer {
  setMeleeCameraShake(enabled: boolean): void;
  setTileset(manifestUrl: string): Promise<{ id: string; warnings: readonly TilesetWarning[] }>;
  setCameraMode(mode: CameraMode): void;
  setZoom(zoom: ZoomLevel): void;
  setVisualPreferences(preferences: VisualPreferences): void;
  visualBase(id: string): { glyph: string; foreground: string; background?: string };
  setCanvasLabel(label: string): void;
}
interface SettingsOptions {
  dom: SettingsDom; document: Document; state: AppState; localization: Localization;
  renderer: SettingsRenderer; rendererReady: () => boolean; preferences: PreferencesClient;
  beforeEdit: () => Promise<void>; openKeys: () => void; openMogaminator: () => void;
  download: (bytes: Uint8Array, name: string) => void;
  renderTargeting: () => void; renderLocaleDependentUi: () => void;
  onBehaviorChange: (preferences: Preferences) => Promise<void>;
  announce: (key: MessageKey, args: Record<string, string | number> | undefined, kind: string) => void;
}
const OPERATION_BOOLS = ["cutCorners", "travelIgnoreItems", "targetPets", "easyOpen", "easyDisarm", "autoRepeat"] as const;
const RUN_STOPS = ["stairs", "openDoors", "knownTreasure"] as const;
const DISPLAY_IDS = DISPLAY_FIELDS.map(field => "display-" + field);
const HUD_GROUPS = {
  header: { field: "showCharacterInfo", sections: ["showCharacterInfo"], arrows: ["▴", "▾"] },
  sidebar: { field: "showSidebar", sections: ["showNearby", "showMessages"], arrows: ["▸", "◂"] },
  footer: { field: "showFooter", sections: ["showMapActions", "showCombatSummary", "showDungeonInfo", "showShortcutBar"], arrows: ["▾", "▴"] },
} as const;
const OPERATION_IDS = ["operation-defaultTarget", ...OPERATION_BOOLS.map(field => "operation-" + field), ...RUN_STOPS.map(field => "run-stop-" + field)];

export class SettingsPanel {
  readonly #o: SettingsOptions;
  readonly #dialog: HTMLDialogElement;
  readonly #search: HTMLInputElement;
  readonly #preview: HTMLElement;
  readonly #status: HTMLElement;
  #base: Preferences = defaultPreferences();
  #draft: Preferences = defaultPreferences();
  #revision: number | undefined;
  #working = false;
  #installed = false;
  #visualEditor: VisualEditor | undefined;
  #prf: { preview: PrfPreview; base: string } | undefined;
  #appliedTileset: TilesetPreset = "ascii";
  #appliedBehavior: string | undefined;
  readonly #remove: (() => void)[] = [];

  constructor(options: SettingsOptions) {
    this.#o = options;
    this.#dialog = this.#element("player-ui-settings-dialog");
    options.document.body.append(this.#dialog);
    this.#search = this.#element("preferences-search");
    this.#preview = this.#element("preferences-preview");
    this.#status = this.#element("preferences-status");
  }
  #element<T extends HTMLElement>(id: string): T {
    const element = this.#o.document.getElementById(id);
    if (!element) throw new Error("Missing element #" + id);
    return element as T;
  }
  get inputPreset(): InputPreset { return this.#o.preferences.snapshot?.preferences.inputPreset ?? "original"; }
  get cameraMode(): CameraMode { return this.#o.preferences.snapshot?.preferences.cameraMode ?? "player-centered"; }
  get zoom(): ZoomLevel { return this.#o.preferences.snapshot?.preferences.zoom ?? 1; }
  get tilesetManifest(): string { return TILESET_MANIFESTS[this.#o.preferences.snapshot?.preferences.tilesetPreset ?? "ascii"]; }

  initialize(): void {
    this.#o.localization.localizeDocument();
    this.#resetDraft();
    this.#help();
  }
  install(): void {
    if (this.#installed) return;
    this.#installed = true;
    const on = (node: EventTarget, type: string, handler: (event: Event) => void) => {
      node.addEventListener(type, handler);
      this.#remove.push(() => node.removeEventListener(type, handler));
    };
    for (const control of this.#controls()) on(control, "change", () => { this.#readControls(); this.#renderPreview(); });
    for (const id of [...OPERATION_IDS, ...DISPLAY_IDS]) on(this.#element(id), "change", () => { this.#readControls(); this.#renderPreview(); });
    for (const id of Object.values(TRAVEL_CONTROLS)) on(this.#element(id), "change", () => { this.#readControls(); this.#renderPreview(); });
    on(this.#element("player-ui-settings-open"), "click", () => { void this.open(); });
    for (const [name, group] of Object.entries(HUD_GROUPS)) {
      on(this.#element("hud-toggle-" + name), "click", () => {
        if (this.#dialog.open) return;
        void this.#run(async () => {
          const saved = this.#o.preferences.snapshot;
          if (!saved) throw new Error("preferences-unavailable");
          const next = structuredClone(saved.preferences);
          const expanded = next.display[group.field] && group.sections.some(field => next.display[field]);
          next.display[group.field] = !expanded;
          // Keep individual selections when collapsing; an explicit expansion must reveal something.
          if (!expanded && !group.sections.some(field => next.display[field])) {
            for (const field of group.sections) next.display[field] = true;
          }
          await this.commit(next, saved.revision);
          this.#resetDraft();
        });
      });
    }
    on(this.#element("player-ui-settings-close"), "click", () => this.close());
    on(this.#dialog, "cancel", event => { event.preventDefault(); this.close(); });
    on(this.#search, "input", () => this.#filter());
    on(this.#element("preferences-save"), "click", () => { void this.#run(async () => {
      if (this.#revision === undefined) throw new Error("preferences-unavailable");
      if (this.#visualEditor?.valid === false) throw new Error(this.#o.localization.format("visual-invalid"));
      this.#readControls();
      await this.commit(this.#draft, this.#revision);
      this.#resetDraft();
      this.#status.textContent = this.#o.localization.format("preferences-saved");
      this.#status.dataset.savedRevision = String(this.#revision);
    }); });
    on(this.#element("preferences-reset"), "click", () => {
      this.#draft = structuredClone(this.#o.preferences.defaults); this.#writeControls(); this.#renderPreview();
    });
    on(this.#element("preferences-reload"), "click", () => {
      if (!this.#discard()) return;
      void this.#run(async () => { await this.#o.preferences.load(); await this.apply(); this.#resetDraft(); });
    });
    on(this.#element("preferences-export"), "click", () => {
      const saved = this.#o.preferences.snapshot;
      if (!saved) return;
      this.#o.download(new TextEncoder().encode(JSON.stringify(saved.preferences, null, 2)), "rfb-preferences.json");
    });
    on(this.#element("preferences-import"), "change", event => {
      const input = event.target as HTMLInputElement, file = input.files?.[0];
      input.value = "";
      if (!file) return;
      void this.#run(async () => {
        if (file.size > 1024 * 1024) throw new Error("preferences-import-too-large");
        const candidate = parsePreferences((await file.text()).replace(/^\uFEFF/, ""));
        this.#draft = candidate;
        this.#writeControls(); this.#renderPreview();
        this.#status.textContent = this.#o.localization.format("preferences-import-preview");
      });
    });
    for (const page of ["general", "behavior", "autopick", "display", "glyphs", "colors", "advanced", "changes"] as const)
      on(this.#element("preferences-" + page), "click", () => this.#openPage(page));
    on(this.#element("preferences-prf-preview"), "click", () => {
      void this.#run(async () => this.#previewPrf(this.#element<HTMLInputElement>("preferences-command").value));
    });
    on(this.#element("preferences-prf-file"), "change", event => {
      const input = event.target as HTMLInputElement, file = input.files?.[0]; input.value = "";
      if (!file) return;
      void this.#run(async () => {
        this.#prf = undefined;
        if (file.size > 1024 * 1024) throw new Error("preferences-import-too-large");
        this.#previewPrf(new TextDecoder("utf-8", { fatal: true }).decode(await file.arrayBuffer()));
      });
    });
    on(this.#element("preferences-prf-accept"), "click", () => { void this.#run(async () => {
      if (!this.#prf || this.#prf.preview.blocked) return;
      this.#readControls();
      if (preferenceSignature(this.#draft) !== this.#prf.base) throw new Error(this.#o.localization.format("prf-preview-stale"));
      if (this.#prf.preview.needsSubset && !this.#element<HTMLInputElement>("preferences-prf-subset").checked)
        throw new Error(this.#o.localization.format("prf-subset-required"));
      this.#draft = this.#prf.preview.preferences; this.#prf = undefined;
      this.#writeControls(); this.#renderPreview();
      this.#status.textContent = this.#o.localization.format("preferences-import-preview");
    }); });
    on(this.#element("preferences-prf-export"), "click", () => {
      const saved = this.#o.preferences.snapshot;
      if (!saved) return;
      const result = exportPrf(saved.preferences, this.#o.state.status?.player.visualCatalog ?? []);
      this.#o.download(new TextEncoder().encode(result.text), "rfb-preferences.prf");
      this.#element("preferences-prf-report").textContent = this.#o.localization.format("prf-export-omitted", { fields: result.omitted.join(", ") });
    });
    on(this.#element("preferences-keys"), "click", () => { if (this.close()) this.#o.openKeys(); });
    on(this.#element("preferences-mogaminator"), "click", () => {
      if (this.#o.state.mode === "playing" && this.#o.state.status?.mogaminator && this.close()) this.#o.openMogaminator();
    });
  }
  dispose(): void { for (const remove of this.#remove.splice(0)) remove(); this.#installed = false; }

  #openVisuals(page: "glyphs" | "colors"): void {
    this.#visualEditor ??= new VisualEditor(this.#element("preferences-visual-editor"), this.#o.state, this.#o.localization,
      () => this.#draft.visuals, () => this.#renderPreview(), id => this.#o.renderer.visualBase(id));
    this.#visualEditor.open(page);
  }
  #openPage(page: "general" | "behavior" | "autopick" | "display" | "glyphs" | "colors" | "advanced" | "changes"): void {
    this.#search.value = "";
    this.#page = page === "glyphs" || page === "colors" ? "visuals" : page;
    this.#filter();
    for (const button of this.#dialog.querySelectorAll<HTMLElement>("[data-settings-page]"))
      button.setAttribute("aria-pressed", String(button.dataset.settingsPage === page));
    if (page === "glyphs" || page === "colors") this.#openVisuals(page);
    if (page === "advanced") this.#element("preferences-command").focus();
  }
  #previewPrf(text: string): void {
    this.#prf = undefined;
    this.#readControls();
    const preview = parsePrf(text, this.#draft, this.#o.state.status?.player.visualCatalog ?? []);
    this.#prf = { preview, base: preferenceSignature(this.#draft) };
    this.#element<HTMLInputElement>("preferences-prf-subset").checked = false;
    const changes = Object.keys(this.#draft).filter(key => preferenceSignature(this.#draft[key as keyof Preferences]) !== preferenceSignature(preview.preferences[key as keyof Preferences]));
    this.#element("preferences-prf-report").textContent = [
      ...preview.diagnostics.map(d => `${d.line}: ${d.input}\n${this.#o.localization.format("prf-reason-" + d.reason)}`),
      this.#o.localization.format(preview.blocked ? "prf-blocked" : "prf-ready"),
      ...changes.map(key => `${key}: ${JSON.stringify(this.#draft[key as keyof Preferences])} → ${JSON.stringify(preview.preferences[key as keyof Preferences])}`),
    ].join("\n");
  }
  async open(page?: "glyphs" | "colors" | "advanced"): Promise<void> {
    if (this.#working) return;
    if (this.#dialog.open) { if (page) this.#openPage(page); return; }
    await this.#run(async () => {
      await this.#o.beforeEdit();
      this.#resetDraft(); this.#search.value = ""; this.#filter();
      this.#dialog.showModal();
      this.#openPage(page ?? "general");
      if (!page) this.#search.focus();
    });
    // #run disables editable controls while opening; focus after they are enabled.
    if (page === "advanced" && this.#dialog.open) this.#element("preferences-command").focus();
  }
  close(): boolean {
    if (this.#working) return false;
    if (!this.#dialog.open) return true;
    if (!this.#discard()) return false;
    this.#dialog.close(); return true;
  }
  #discard(): boolean {
    return preferenceSignature(this.#draft) === preferenceSignature(this.#base) ||
      this.#o.document.defaultView!.confirm(this.#o.localization.format("preferences-discard"));
  }

  async commit(preferences: Preferences, revision: number): Promise<void> {
    await this.#o.beforeEdit();
    await this.#o.preferences.save(preferences, revision);
    try { await this.apply(); }
    catch (error) { throw new Error(this.#o.localization.format("preferences-saved-apply-error", { error: detail(error) })); }
  }
  noteMogaminatorApplied(mogaminator: Preferences["mogaminator"]): void {
    if (this.#appliedBehavior) this.#appliedBehavior = preferenceSignature({ ...JSON.parse(this.#appliedBehavior), mogaminator });
  }
  async apply(): Promise<void> {
    const p = this.#o.preferences.snapshot?.preferences;
    if (!p) return;
    this.#o.state.display = { ...p.display };
    this.#o.renderer.setMeleeCameraShake(p.display.meleeCameraShake);
    for (const field of HUD_DISPLAY_FIELDS) this.#element("app").dataset[field] = String(p.display[field]);
    this.#o.state.visuals = structuredClone(p.visuals);
    this.#o.localization.setLocale(p.locale);
    this.#o.localization.localizeDocument();
    this.#help();
    for (const [name, group] of Object.entries(HUD_GROUPS)) {
      const expanded = p.display[group.field] && group.sections.some(field => p.display[field]);
      this.#element("app").dataset[name + "Expanded"] = String(expanded);
      const button = this.#element<HTMLButtonElement>("hud-toggle-" + name);
      button.textContent = group.arrows[expanded ? 0 : 1];
      button.dataset.l10nAriaLabel = `hud-${expanded ? "hide" : "show"}-${name}`;
      button.title = this.#o.localization.format(button.dataset.l10nAriaLabel);
      button.setAttribute("aria-label", button.title);
      button.setAttribute("aria-expanded", String(expanded));
    }
    if (this.#o.rendererReady()) {
      this.#o.renderer.setVisualPreferences(p.visuals);
      if (this.#appliedTileset !== p.tilesetPreset) {
        const result = await this.#o.renderer.setTileset(TILESET_MANIFESTS[p.tilesetPreset]);
        this.announceTileset(result.id, result.warnings);
      }
      this.#o.renderer.setCameraMode(p.cameraMode);
      this.#o.renderer.setZoom(p.zoom);
      this.#o.renderer.setCanvasLabel(this.#o.localization.format("map-aria-label"));
      this.#o.renderTargeting();
    }
    this.#appliedTileset = p.tilesetPreset;
    this.#o.renderLocaleDependentUi();
    const behavior = preferenceSignature(behaviorPreferences(p));
    if (this.#o.state.mode === "playing" && !this.#o.state.playerDead && !this.#o.state.campaignEnded && this.#appliedBehavior !== behavior) await this.#o.onBehaviorChange(p);
    this.#appliedBehavior = behavior;
  }
  announceTileset(id: string, warnings: readonly TilesetWarning[]): void {
    this.#o.announce("message-tileset-loaded", { id }, "system");
    for (const warning of warnings) this.#o.announce(warning === "image-too-small" ? "message-tileset-image-too-small" : "message-tileset-image-load-failed", undefined, "system");
  }
  showLoadError(error: unknown): void {
    this.#status.textContent = this.#o.localization.format("preferences-error", { error: detail(error) });
    this.#availability();
    if (!this.#dialog.open) this.#o.announce("preferences-error", { error: detail(error) }, "system");
  }

  #resetDraft(): void {
    this.#prf = undefined;
    this.#element("preferences-prf-report").replaceChildren();
    const saved = this.#o.preferences.snapshot;
    this.#revision = saved?.revision;
    this.#base = structuredClone(saved?.preferences ?? defaultPreferences());
    this.#draft = structuredClone(this.#base);
    this.#writeControls(); this.#renderPreview();
    this.#status.textContent = saved ? this.#o.preferences.warnings.map(warning =>
      this.#o.localization.format("preferences-migration-warning", { detail: warning })).join("\n") : this.#o.localization.format("preferences-unavailable");
    this.#availability();
    this.#element("travel-options").hidden = false;
    this.#element("preferences-session-details").hidden = this.#o.state.mode !== "playing";
  }
  #controls(): HTMLSelectElement[] {
    const d = this.#o.dom;
    return [d.languageSelect, d.inputPresetSelect, d.tilesetPresetSelect, d.cameraModeSelect, d.zoomSelect];
  }
  #writeControls(): void {
    this.#visualEditor?.render();
    const d = this.#o.dom, p = this.#draft;
    for (const field of ["hpWarningPercent", "manaWarningPercent"]) {
      for (const option of this.#element("display-" + field).querySelectorAll<HTMLOptionElement>("option"))
        option.textContent = this.#o.localization.format("display-warning-threshold", { percent: Number(option.value) });
    }
    d.languageSelect.value = p.locale; d.inputPresetSelect.value = p.inputPreset;
    d.tilesetPresetSelect.value = p.tilesetPreset; d.cameraModeSelect.value = p.cameraMode;
    d.zoomSelect.value = String(p.zoom);
    for (const field of DISPLAY_FIELDS) {
      const control = this.#element<HTMLInputElement>("display-" + field);
      if (typeof DEFAULT_DISPLAY[field] === "boolean") control.checked = p.display[field] as boolean;
      else control.value = String(p.display[field]);
    }
    (this.#element("operation-defaultTarget") as HTMLSelectElement).value = p.operations.defaultTarget;
    for (const field of OPERATION_BOOLS) (this.#element("operation-" + field) as HTMLInputElement).checked = p.operations[field];
    for (const field of RUN_STOPS) (this.#element("run-stop-" + field) as HTMLInputElement).checked = p.operations.runStops[field];
    for (const [field, id] of Object.entries(TRAVEL_CONTROLS)) (this.#element(id) as HTMLInputElement).checked = p.travel[field as keyof typeof p.travel];
  }
  #readControls(): void {
    const d = this.#o.dom;
    this.#draft = parsePreferences(JSON.stringify({ ...this.#draft, locale: d.languageSelect.value, inputPreset: d.inputPresetSelect.value,
      tilesetPreset: d.tilesetPresetSelect.value, cameraMode: d.cameraModeSelect.value, zoom: Number(d.zoomSelect.value),
      display: Object.fromEntries(DISPLAY_FIELDS.map(field => {
        const control = this.#element<HTMLInputElement>("display-" + field);
        return [field, typeof DEFAULT_DISPLAY[field] === "boolean" ? control.checked : Number(control.value)];
      })),
      operations: { ...Object.fromEntries(OPERATION_BOOLS.map(field => [field, (this.#element("operation-" + field) as HTMLInputElement).checked])),
        defaultTarget: (this.#element("operation-defaultTarget") as HTMLSelectElement).value,
        runStops: Object.fromEntries(RUN_STOPS.map(field => [field, (this.#element("run-stop-" + field) as HTMLInputElement).checked])) },
      travel: Object.fromEntries(Object.entries(TRAVEL_CONTROLS).map(([field, id]) => [field, (this.#element(id) as HTMLInputElement).checked])) }));
  }
  #renderPreview(): void {
    const changes = Object.keys(this.#draft).filter(key => preferenceSignature(this.#draft[key as keyof Preferences]) !== preferenceSignature(this.#base[key as keyof Preferences]));
    this.#preview.replaceChildren();
    if (!changes.length) { this.#preview.textContent = this.#o.localization.format("preferences-no-changes"); return; }
    for (const key of changes) {
      const row = this.#o.document.createElement("p");
      const value = (p: Preferences) => key === "keyBindings" ? String(p.keyBindings.length) : typeof p[key as keyof Preferences] === "object" ? JSON.stringify(p[key as keyof Preferences]) : String(p[key as keyof Preferences]);
      row.textContent = this.#o.localization.format("preferences-field-" + key) + ": " + value(this.#base) + " → " + value(this.#draft);
      this.#preview.append(row);
    }
    if (changes.includes("keyBindings")) {
      const bindings = this.#o.document.createElement("pre");
      bindings.textContent = JSON.stringify(this.#draft.keyBindings, null, 2); this.#preview.append(bindings);
    }
  }
  #help(): void {
    this.#o.dom.controlsHelp.textContent = this.#o.localization.format("controls-" + this.inputPreset);
  }
  #page = "general";
  #filter(): void {
    const query = this.#search.value.toLocaleLowerCase().trim();
    for (const panel of this.#dialog.querySelectorAll<HTMLElement>("[data-settings-panel]")) {
      const page = panel.dataset.settingsPanel;
      panel.hidden = query ? !panel.textContent?.toLocaleLowerCase().includes(query) : page !== this.#page;
    }
    for (const row of this.#dialog.querySelectorAll<HTMLElement>("[data-preference-row]")) {
      row.hidden = !row.textContent?.toLocaleLowerCase().includes(query);
    }
  }
  #availability(): void {
    for (const name of Object.keys(HUD_GROUPS)) {
      this.#element<HTMLButtonElement>("hud-toggle-" + name).disabled = this.#working || !this.#o.preferences.snapshot;
    }
    const unavailable = !this.#o.preferences.snapshot;
    for (const control of this.#element("preferences-visual-editor").querySelectorAll<HTMLInputElement | HTMLSelectElement | HTMLButtonElement>("input, select, button")) control.disabled = unavailable || this.#working;
    for (const id of [...OPERATION_IDS, ...DISPLAY_IDS]) (this.#element(id) as HTMLInputElement | HTMLSelectElement).disabled = unavailable || this.#working;
    for (const control of this.#controls()) control.disabled = unavailable || this.#working;
    for (const id of Object.values(TRAVEL_CONTROLS)) (this.#element(id) as HTMLInputElement).disabled = unavailable || this.#working;
    for (const id of ["preferences-command", "preferences-prf-preview", "preferences-prf-file", "preferences-prf-export", "preferences-prf-subset", "preferences-advanced", "preferences-save", "preferences-reset", "preferences-export", "preferences-import", "preferences-keys", "preferences-mogaminator", "preferences-glyphs", "preferences-colors"]) {
      (this.#element(id) as HTMLButtonElement | HTMLInputElement).disabled = unavailable || this.#working;
    }
    this.#element<HTMLButtonElement>("preferences-prf-accept").disabled = unavailable || this.#working || !this.#prf || this.#prf.preview.blocked;
    const canEditMogaminator = this.#o.state.mode === "playing" && !!this.#o.state.status?.mogaminator;
    this.#element<HTMLButtonElement>("preferences-mogaminator").disabled = unavailable || this.#working || !canEditMogaminator;
    this.#element("preferences-mogaminator-unavailable").hidden = canEditMogaminator;
    (this.#element("preferences-reload") as HTMLButtonElement).disabled = this.#working;
    (this.#element("player-ui-settings-close") as HTMLButtonElement).disabled = this.#working;
    this.#dialog.setAttribute("aria-busy", String(this.#working));
  }
  async #run(action: () => Promise<void>): Promise<void> {
    if (this.#working) return;
    this.#working = true; this.#availability();
    delete this.#status.dataset.savedRevision;
    try { await action(); }
    catch (error) { this.showLoadError(error); }
    finally { this.#working = false; this.#availability(); }
  }
}
// Form assembly and JSON import can have different key order for the same preferences.
function preferenceSignature(value: unknown): string {
  return JSON.stringify(value, (_key, item) => item && typeof item === "object" && !Array.isArray(item)
    ? Object.fromEntries(Object.entries(item).sort(([a], [b]) => a.localeCompare(b))) : item);
}
function detail(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (error && typeof error === "object" && "code" in error && "detail" in error) return String(error.code) + ": " + String(error.detail);
  return String(error);
}
export function isInputPreset(value: string | null): value is InputPreset {
  return value === "original" || value === "roguelike";
}
export function inputPresetMessageKey(preset: InputPreset): MessageKey { return "input-preset-" + preset; }
