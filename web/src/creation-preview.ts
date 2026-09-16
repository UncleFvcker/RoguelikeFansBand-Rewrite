// SPDX-License-Identifier: MPL-2.0

import type { CharacterCreationPreviewDto } from "./protocol.ts";
import type { Localization } from "./localization.ts";
import { formatAttributeValue } from "./status-panel.ts";

type Selection = { buildId: string; raceId: string };

/** One preview follows the active creation page; only its latest request may render. */
export class CreationPreview {
  readonly #root: HTMLElement;
  readonly #localization: Localization;
  readonly #load: (buildId: string, raceId: string) => Promise<CharacterCreationPreviewDto>;
  #key = "";
  #request = 0;

  constructor(document: Document, localization: Localization, load: (buildId: string, raceId: string) => Promise<CharacterCreationPreviewDto>) {
    this.#root = document.createElement("section");
    this.#root.className = "creation-preview";
    this.#root.tabIndex = -1;
    this.#localization = localization;
    this.#load = load;
  }

  reset(): void {
    this.#request++;
    this.#key = "";
    this.#root.remove();
  }

  async show(host: HTMLElement, selection: Selection | undefined, confirmed: Selection | undefined = selection): Promise<void> {
    const compare = !!selection && !!confirmed && (selection.buildId !== confirmed.buildId || selection.raceId !== confirmed.raceId);
    const key = JSON.stringify([host.id, selection, confirmed, this.#localization.locale]);
    if (key === this.#key) return;
    this.#key = key;
    const request = ++this.#request;
    if (this.#root.parentElement !== host) host.append(this.#root);
    this.#root.setAttribute("aria-label", this.#localization.format("creation-preview-heading"));
    this.#root.setAttribute("aria-busy", String(!!selection));
    this.#root.dataset.state = selection ? "loading" : "incomplete";
    const status = this.#element("p", this.#localization.format(selection ? "creation-preview-loading" : "creation-preview-incomplete"));
    status.setAttribute("role", "status");
    // Replacing a focused retry/disclosure must not drop keyboard focus to the body.
    if (this.#root.contains(this.#root.ownerDocument.activeElement)) this.#root.focus({ preventScroll: true });
    this.#root.replaceChildren(status);
    if (!selection) return;
    try {
      const [candidate, baseline] = await Promise.allSettled([
        this.#load(selection.buildId, selection.raceId),
        compare ? this.#load(confirmed!.buildId, confirmed!.raceId) : Promise.resolve(undefined),
      ]);
      if (request !== this.#request) return;
      if (candidate.status === "rejected") throw candidate.reason;
      this.#root.setAttribute("aria-busy", "false");
      this.#render(candidate.value, baseline.status === "fulfilled" ? baseline.value : undefined);
      this.#root.dataset.state = "ready";
      if (baseline.status === "rejected") this.#root.prepend(this.#element("p", this.#localization.format("creation-compare-unavailable"), "creation-preview-note"));
    } catch (error) {
      if (request !== this.#request) return;
      this.#root.setAttribute("aria-busy", "false");
      this.#root.dataset.state = "error";
      status.textContent = this.#localization.format("creation-preview-error");
      const detail = this.#element("p", error instanceof Error ? error.message : String(error), "creation-preview-note");
      const retry = this.#element("button", this.#localization.format("creation-preview-retry"));
      retry.type = "button";
      retry.addEventListener("click", () => { this.#key = ""; void this.show(host, selection, confirmed); });
      this.#root.append(detail, retry);
    }
  }

  #element<K extends keyof HTMLElementTagNameMap>(tag: K, text = "", className = ""): HTMLElementTagNameMap[K] {
    const element = this.#root.ownerDocument.createElement(tag);
    element.textContent = text;
    if (className) element.className = className;
    return element;
  }

  #render(preview: CharacterCreationPreviewDto, baseline?: CharacterCreationPreviewDto): void {
    const l10n = this.#localization;
    const build = preview.build;
    const heading = this.#element("h4", l10n.format("creation-preview-heading"));
    const identity = this.#element("p", [build.raceNameKey, build.buildNameKey, build.personalityNameKey].map(key => l10n.format(key)).join(" · "), "creation-preview-identity");
    if (baseline) identity.append(this.#element("small", l10n.format("creation-compare-with", { name: [baseline.build.raceNameKey, baseline.build.buildNameKey].map(key => l10n.format(key)).join(" · ") })));
    const note = this.#element("p", l10n.format("creation-preview-scope"), "creation-preview-note");
    const table = this.#element("table");
    const caption = this.#element("caption", l10n.format("creation-preview-attributes"));
    const head = table.createTHead().insertRow();
    for (const key of ["attribute", "base", "modifier", "effective"]) {
      const cell = this.#element("th", l10n.format(`creation-preview-${key}`));
      cell.scope = "col";
      head.append(cell);
    }
    table.prepend(caption);
    const body = table.createTBody();
    for (const attribute of preview.attributes) {
      const row = body.insertRow();
      const name = this.#element("th", l10n.format(`attribute-${attribute.attribute}`));
      name.scope = "row";
      if (attribute.attribute === preview.castingAttribute) {
        row.className = "creation-casting-attribute";
        name.append(this.#element("small", l10n.format("creation-preview-casting")));
      }
      row.append(name);
      for (const value of [formatAttributeValue(attribute.natural), `${attribute.modifier > 0 ? "+" : ""}${attribute.modifier}`, formatAttributeValue(attribute.effective)]) {
        row.append(this.#element("td", value));
      }
      const previous = baseline?.attributes.find(entry => entry.attribute === attribute.attribute);
      if (previous && previous.effective !== attribute.effective) row.lastElementChild!.append(this.#element("small", `${formatAttributeValue(previous.effective)} → ${formatAttributeValue(attribute.effective)}`, "creation-comparison"));
    }
    const metrics = this.#element("dl", "", "creation-preview-metrics");
    for (const [key, value] of [["life", `${build.lifePercent}%`], ["hp", String(preview.baseHp)], ["experience", `${build.experiencePercent}%`]]) {
      const group = this.#element("div");
      group.append(this.#element("dt", l10n.format(`creation-preview-${key}`)), this.#element("dd", value));
      metrics.append(group);
    }
    const metricNote = this.#element("p", l10n.format("creation-preview-metrics-help"), "creation-preview-note");
    if (baseline) {
      const difference = (value: number) => `${value > 0 ? "+" : ""}${value}`;
      const changes = l10n.format("creation-compare-metrics", {
        life: difference(build.lifePercent - baseline.build.lifePercent),
        hp: difference(preview.baseHp - baseline.baseHp),
        experience: preview.experienceNoteKey === baseline.experienceNoteKey ? l10n.format("creation-compare-percentage-points", { value: difference(build.experiencePercent - baseline.build.experiencePercent) }) : l10n.format("creation-compare-not-applicable"),
      });
      metricNote.prepend(this.#element("small", changes, "creation-metric-comparison"));
    }
    if (preview.experienceNoteKey) metricNote.append(this.#element("span", ` ${l10n.format(preview.experienceNoteKey)}`));
    const skillHeading = this.#element("h4", l10n.format("creation-preview-skills"));
    const skills = this.#element("dl", "", "creation-preview-skills");
    for (const entry of preview.skills) {
      const group = this.#element("div");
      const rating = this.#element("dd", l10n.format(entry.ratingKey));
      rating.dataset.rating = entry.ratingKey;
      group.append(this.#element("dt", l10n.format(entry.skill.nameKey)), rating);
      const previous = baseline?.skills.find(row => row.skill.id === entry.skill.id);
      if (previous && (entry.skill.base !== previous.skill.base || entry.skill.growthPerTenLevels !== previous.skill.growthPerTenLevels)) {
        rating.append(this.#element("small", l10n.format("creation-compare-skill", { base: signed(entry.skill.base - previous.skill.base), growth: signed(entry.skill.growthPerTenLevels - previous.skill.growthPerTenLevels) }), "creation-comparison"));
      }
      skills.append(group);
    }
    const skillNote = this.#element("p", l10n.format("creation-preview-skills-help"), "creation-preview-note");
    this.#root.replaceChildren(heading, identity, note, table, metrics, metricNote, skillHeading, skills, skillNote);
    this.#renderSources(preview);
    this.#renderFeatures(preview);
  }

  #renderSources(preview: CharacterCreationPreviewDto): void {
    const f = this.#localization.format.bind(this.#localization);
    const details = this.#element("details", "", "creation-sources");
    details.append(this.#element("summary", f("creation-sources-heading")), this.#element("p", f("creation-sources-help"), "creation-preview-note"));
    if (preview.experienceNoteKey) details.append(this.#element("p", f("creation-sources-special-experience"), "creation-preview-note"));
    for (const source of preview.sources) {
      const heading = this.#element("h4", `${f(`attribute-source-kind-${source.kind}`)} · ${f(source.nameKey)}`);
      const table = this.#element("table");
      table.append(this.#element("caption", f("creation-source-attributes")));
      const head = table.createTHead().insertRow(), values = table.createTBody().insertRow();
      for (const row of preview.attributes) {
        const th = this.#element("th", f(`attribute-${row.attribute}`)); th.scope = "col"; head.append(th);
        values.append(this.#element("td", signed(source.modifiers[row.attribute])));
      }
      const metrics = this.#element("p", f("creation-source-metrics", { life: source.lifePercent, hp: signed(source.baseHp), experience: source.experiencePercent }), "creation-preview-note");
      const list = this.#element("ul", "", "creation-source-skills");
      for (const row of preview.skills) {
        const skill = source.skills.find(entry => entry.id === row.skill.id);
        list.append(this.#element("li", f("creation-source-skill", { name: f(row.skill.nameKey), base: signed(skill?.base ?? 0), growth: signed(skill?.growthPerTenLevels ?? 0) })));
      }
      details.append(heading, table, metrics, list);
    }
    this.#root.append(details);
  }

  #renderFeatures(preview: CharacterCreationPreviewDto): void {
    const f = this.#localization.format.bind(this.#localization);
    for (const category of ["restrictions", "initial", "growth"] as const) {
      const entries = preview.features.filter(entry => category === "restrictions" ? entry.negative : !entry.negative && (category === "initial" ? entry.minimumLevel <= 1 : entry.minimumLevel > 1));
      if (!entries.length) continue;
      const section = this.#element(category === "growth" ? "details" : "section", "", `creation-features creation-features-${category}`);
      section.append(this.#element(category === "growth" ? "summary" : "h4", f(`creation-features-${category}`)));
      const list = this.#element("ul");
      for (const entry of entries) {
        const item = this.#element("li");
        const args: Record<string, number> = entry.value === null ? {} : { value: entry.value };
        item.append(this.#element("span", entry.name ?? f(entry.nameKey, args)));
        const level = entry.maximumLevel === null ? f("creation-feature-level", { level: entry.minimumLevel }) : f("creation-feature-level-range", { minimum: entry.minimumLevel, maximum: entry.maximumLevel });
        item.append(this.#element("small", `${f(entry.sourceNameKey)} · ${level} · ${f(entry.acquisitionKey, args)}`));
        if (entry.detailKey?.startsWith("resistance-level-") || entry.detailKey === "creation-status-immune") {
          item.prepend(this.#element("strong", f(entry.detailKey)));
        } else if (entry.description || entry.detailKey) {
          const detail = this.#element("details");
          detail.append(this.#element("summary", f("creation-feature-description")), this.#element("p", entry.description ?? f(entry.detailKey!, args)));
          item.append(detail);
        }
        list.append(item);
      }
      section.append(list); this.#root.append(section);
    }
  }
}

function signed(value: number): string { return `${value > 0 ? "+" : ""}${value}`; }
