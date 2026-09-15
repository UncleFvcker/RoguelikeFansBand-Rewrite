// SPDX-License-Identifier: MPL-2.0
import type { AppState } from "./app-state";
import type { Localization } from "./localization";
import type { VisualCategoryDto } from "./protocol";
import { BASE_PALETTE, DEFAULT_THEME, defaultVisuals, validColor, validGlyph, visualStyle, visualOverride, prfVisualId, type VisualPreferences, type VisualOverride } from "./visual-preferences.ts";

export class VisualEditor {
  valid = true;
  #page: "glyphs" | "colors" = "glyphs";
  #category: VisualCategoryDto = "item";
  #search = "";
  #selected = "";
  readonly host: HTMLElement;
  readonly state: AppState;
  readonly localization: Localization;
  readonly draft: () => VisualPreferences;
  readonly changed: () => void;
  readonly base: (id: string) => { glyph: string; foreground: string; background?: string };
  constructor(host: HTMLElement, state: AppState, localization: Localization,
    draft: () => VisualPreferences, changed: () => void,
    base: (id: string) => { glyph: string; foreground: string; background?: string }) {
    this.host = host; this.state = state; this.localization = localization;
    this.draft = draft; this.changed = changed; this.base = base;
  }

  open(page: "glyphs" | "colors"): void { this.#page = page; this.render(); this.host.scrollIntoView({ block: "start" }); }
  render(): void {
    this.valid = true;
    this.host.replaceChildren();
    const document = this.host.ownerDocument;
    const t = (key: string) => this.localization.format(key);
    const add = (tag: string, key: string) => { const node = document.createElement(tag); node.textContent = t(key); this.host.append(node); return node; };
    add("h3", "visual-" + this.#page);
    add("p", "visual-help");
    if (this.#page === "colors") { this.#colors(); return; }
    const category = document.createElement("select"); category.setAttribute("aria-label", t("visual-category"));
    for (const value of ["item", "monster", "terrain", "marker"] as const) category.add(new Option(t("visual-category-" + value), value));
    category.value = this.#category;
    category.onchange = () => { this.#category = category.value as VisualCategoryDto; this.#selected = ""; this.render(); };
    const search = document.createElement("input"); search.type = "search"; search.value = this.#search;
    search.setAttribute("aria-label", t("visual-search")); search.placeholder = t("visual-search");
    const list = document.createElement("select"); list.size = 9; list.setAttribute("aria-label", t("visual-known-identities"));
    const detail = document.createElement("div"); detail.className = "visual-detail";
    const entries = (this.state.status?.player.visualCatalog ?? []).filter(v => v.category === this.#category);
    const fill = () => {
      this.#search = search.value;
      const query = this.#search.trim().toLocaleLowerCase();
      list.replaceChildren(...entries.filter(v => `${t(v.nameKey)} ${v.id} ${v.glyph} ${visualOverride(this.draft(), v).glyph ?? ""}`.toLocaleLowerCase().includes(query))
        .map(v => new Option(`${t(v.nameKey)} · ${v.id}`, v.id)));
      if ([...list.options].some(o => o.value === this.#selected)) list.value = this.#selected;
      else list.selectedIndex = list.options.length ? 0 : -1;
      this.#selected = list.value; this.#glyphDetail(detail, this.#selected);
    };
    search.oninput = fill;
    list.onchange = () => { this.#selected = list.value; this.#glyphDetail(detail, list.value); };
    this.host.append(category, search, list, detail);
    const reset = add("button", "visual-reset-category") as HTMLButtonElement; reset.type = "button";
    reset.onclick = () => { for (const entry of entries) { delete this.draft().overrides[entry.id]; delete this.draft().overrides[prfVisualId(entry)]; } this.changed(); fill(); };
    fill();
  }

  #glyphDetail(host: HTMLElement, id: string): void {
    this.valid = true; host.replaceChildren();
    const entry = this.state.status?.player.visualCatalog.find(v => v.id === id);
    if (!entry) { host.textContent = this.localization.format("visual-empty"); return; }
    const document = host.ownerDocument, base = this.base(id);
    const preview = document.createElement("output"); preview.className = "visual-glyph-preview";
    const error = document.createElement("p"); error.setAttribute("role", "alert");
    const paint = () => {
      const style = visualStyle(this.draft(), id, base, true, entry);
      preview.textContent = style.glyph; preview.style.color = style.foreground; preview.style.backgroundColor = style.background ?? "";
    };
    const controls = new Map<keyof VisualOverride, HTMLInputElement>();
    for (const field of ["glyph", "foreground", "background"] as const) {
      const label = document.createElement("label"); label.textContent = this.localization.format("visual-" + field);
      const input = document.createElement("input"); input.type = "text";
      input.value = this.draft().overrides[id]?.[field] ?? "";
      input.placeholder = this.draft().overrides[prfVisualId(entry)]?.[field] ?? base[field] ?? this.localization.format("visual-inherit");
      input.dataset.visualField = field;
      controls.set(field, input); label.append(input); host.append(label);
    }
    const change = () => {
      const override: VisualOverride = {};
      this.valid = true;
      for (const [field, input] of controls) {
        const value = input.value;
        const valid = !value || (field === "glyph" ? validGlyph(value) : validColor(value));
        input.setAttribute("aria-invalid", String(!valid)); this.valid &&= valid;
        if (value) override[field] = value;
      }
      error.textContent = this.valid ? "" : this.localization.format("visual-invalid");
      if (!this.valid) return;
      if (Object.keys(override).length) this.draft().overrides[id] = override;
      else delete this.draft().overrides[id];
      this.changed(); paint();
    };
    controls.forEach(input => input.oninput = change);
    const reset = document.createElement("button"); reset.type = "button"; reset.textContent = this.localization.format("visual-reset-item");
    reset.onclick = () => { delete this.draft().overrides[id]; this.changed(); this.#glyphDetail(host, id); };
    const sharedId = prfVisualId(entry);
    if (sharedId !== id && this.draft().overrides[sharedId]) {
      const shared = document.createElement("button"); shared.type = "button";
      shared.textContent = this.localization.format("prf-reset-base");
      shared.onclick = () => { delete this.draft().overrides[sharedId]; this.changed(); this.#glyphDetail(host, id); };
      host.append(shared);
    }
    host.append(preview, error, reset); paint();
  }

  #colors(): void {
    const document = this.host.ownerDocument;
    const preview = document.createElement("div"); preview.className = "visual-theme-preview";
    const paint = () => {
      const theme = this.draft().theme;
      preview.style.backgroundColor = theme.background;
      preview.replaceChildren(...["visible", "memory", "dark"].map(kind => {
        const cell = document.createElement("span"); cell.style.position = "relative";
        cell.textContent = `@ · ${this.localization.format("visual-preview-" + kind)}`;
        cell.style.color = this.draft().palette[1]!; cell.style.borderColor = theme.grid;
        const mask = document.createElement("i"); mask.style.cssText = "position:absolute;inset:0;pointer-events:none";
        mask.style.backgroundColor = kind === "memory" ? theme.memoryColor : "#000000";
        mask.style.opacity = String(kind === "memory" ? theme.memoryOpacity : kind === "dark" ? theme.darknessOpacity : 0);
        cell.append(mask); return cell;
      }));
    };
    const palette = document.createElement("fieldset");
    const legend = document.createElement("legend"); legend.textContent = this.localization.format("visual-palette"); palette.append(legend);
    this.draft().palette.forEach((color, index) => {
      const label = document.createElement("label"); label.textContent = `${index} · ${BASE_PALETTE[index]}`;
      const input = document.createElement("input"); input.type = "color"; input.value = color;
      input.oninput = () => { this.draft().palette[index] = input.value; this.changed(); paint(); };
      label.append(input); palette.append(label);
    });
    const theme = document.createElement("fieldset");
    const title = document.createElement("legend"); title.textContent = this.localization.format("visual-theme"); theme.append(title);
    for (const [key, value] of Object.entries(DEFAULT_THEME)) {
      const label = document.createElement("label"); label.textContent = this.localization.format("visual-theme-" + key);
      const input = document.createElement("input"); input.type = typeof value === "string" ? "color" : "range";
      if (input.type === "range") { input.min = "0"; input.max = "1"; input.step = "0.01"; }
      input.value = String(this.draft().theme[key as keyof typeof DEFAULT_THEME]);
      const output = document.createElement("output"); output.textContent = input.value;
      input.oninput = () => {
        Object.assign(this.draft().theme, { [key]: typeof value === "string" ? input.value : Number(input.value) });
        output.textContent = input.value; this.changed(); paint();
      };
      label.append(input, output); theme.append(label);
    }
    const reset = document.createElement("button"); reset.type = "button"; reset.textContent = this.localization.format("visual-reset-colors");
    reset.onclick = () => { const defaults = defaultVisuals(); this.draft().palette = defaults.palette; this.draft().theme = defaults.theme; this.changed(); this.render(); };
    this.host.append(preview, palette, theme, reset); paint();
  }
}
