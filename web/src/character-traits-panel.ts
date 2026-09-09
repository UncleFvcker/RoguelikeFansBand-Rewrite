// SPDX-License-Identifier: MPL-2.0
import type { Localization } from "./localization";
import type { BodySlotDto, CharacterStatDto, CharacterTraitDetailsDto, CharacterTraitSourceDto, EquipmentItemDto, InventoryItemDto, MeleeDamagePreviewDto, PlayerDto } from "./protocol";

const ATTACK_STATS = ["melee-attacks-hundredths", "ranged-base-shot", "ranged-energy"];

export function meleeDamagePreviewValue(preview: MeleeDamagePreviewDto, localization: Localization): string {
  return preview.baseDamage === null ? localization.format("trait-value-unknown")
    : localization.format("trait-melee-damage-range", { minimum: preview.baseDamage[0], maximum: preview.baseDamage[1] });
}

export function traitActionProtection(data: CharacterTraitDetailsDto): boolean | null {
  // The current core represents free action as paralysis immunity, not a counted passive.
  return data.statusImmunities?.includes("rfb.status.paralysis")
    ?? (data.sources.some((source) => source.statusImmunities.includes("rfb.status.paralysis")) ? true : null);
}

export function traitStatValue(stat: CharacterStatDto, localization: Localization): string {
  if (stat.value == null) return localization.format("trait-value-unknown");
  const unit = ["equipment-life", "natural-regeneration", "mutation-regeneration", "ranged-base-shot"].includes(stat.id)
    ? "percent" : stat.id === "infravision" ? "tiles" : stat.id === "melee-attacks-hundredths" ? "attacks" : stat.id === "ranged-energy" ? "energy" : "points";
  const value = stat.id === "melee-attacks-hundredths" ? (stat.value / 100).toFixed(2) : stat.value;
  return localization.format(`trait-unit-${unit}`, { value });
}

export function traitStatSourceValue(stat: CharacterStatDto, amount: number, localization: Localization): string {
  const value = ["equipment-life", "natural-regeneration"].includes(stat.id)
    ? localization.format("trait-unit-percentage-points", { value: amount })
    : traitStatValue({ ...stat, value: amount }, localization);
  return `${amount >= 0 ? "+" : ""}${value}`;
}

export function renderCharacterTraitsDetails(
  defenses: HTMLElement, attacks: HTMLElement, player: PlayerDto,
  items: readonly (EquipmentItemDto | InventoryItemDto)[], localization: Localization,
  statusName: (id: string | undefined) => string,
  bodySlots: readonly BodySlotDto[],
): void {
  const document = defenses.ownerDocument;
  const data = player.traitDetails;
  const text = (tag: string, value: string, className = "") => {
    const node = document.createElement(tag);
    node.textContent = value;
    node.className = className;
    return node;
  };
  const f = localization.format.bind(localization);
  if (!data) {
    defenses.replaceChildren(text("p", f("progression-unavailable"), "character-detail-empty"));
    attacks.replaceChildren();
    return;
  }
  const open = new Set([...defenses.querySelectorAll<HTMLDetailsElement>("details[open]"),
    ...attacks.querySelectorAll<HTMLDetailsElement>("details[open]")].map((node) => node.dataset.trait));
  const focus = (document.activeElement as HTMLElement | null)?.dataset.traitFocus;
  const sourceName = (id: string): string => {
    const item = items.find((item) => item.id === id);
    if (item) return `${f(item.displayNameKey)} · ${"slotId" in item ? item.slotId : f("trait-inventory-source")}`;
    const mutation = player.mutations?.find((mutation) => mutation.id === id);
    if (mutation) return mutation.name;
    if (id === player.kindId) return f("trait-origin-base");
    if (id.startsWith("rfb.status.") || player.statuses.some((status) => status.kindId === id)) return statusName(id);
    if (id === player.build?.raceId) return f(player.build.raceNameKey);
    if (id === player.build?.classId) return f(player.build.classNameKey);
    if (id === player.build?.personalityId) return f(player.build.personalityNameKey);
    const key = `trait-source-id-${id.replaceAll(".", "-")}`;
    return localization.hasMessage(localization.locale, key) ? f(key) : id;
  };
  const origin = (source: CharacterTraitSourceDto) => `${f(`trait-origin-${source.kind}`)} · ${sourceName(source.sourceId)}`;
  const unknown = f("trait-value-unknown");
  const active = (value: boolean | null) => value == null ? unknown : f(value ? "trait-active" : "trait-inactive");
  const row = (id: string, label: string, value: string, lines: string[], note?: string, bulleted = false) => {
    const details = document.createElement("details");
    details.className = "trait-row";
    details.dataset.trait = id;
    details.open = open.has(id);
    const summary = document.createElement("summary");
    summary.dataset.traitFocus = id;
    summary.append(text("span", label, "trait-name"), text("span", value, "trait-value"));
    const body = text("div", "", "trait-row-body");
    if (note) body.append(text("p", note));
    if (bulleted) {
      const list = text("ul", "", "race-effects-list");
      for (const line of lines) list.append(text("li", line));
      body.append(list);
    } else {
      for (const line of lines) body.append(text("p", line));
    }
    if (!lines.length) body.append(text("p", f(data.equipmentComplete && value !== unknown ? "trait-no-sources" : "trait-no-known-sources")));
    details.append(summary, body);
    return details;
  };
  const section = (title: string) => {
    const node = document.createElement("section");
    node.dataset.traitSection = title;
    node.append(text("h3", f(title)));
    return node;
  };
  const res = section("trait-resistances");
  for (const resistance of data.resistances) {
    const value = resistance.level == null ? unknown : f("trait-resistance-value", {
      level: f(`resistance-level-${resistance.level}`), percent: resistance.reductionPercent!,
    });
    const lines = data.sources.flatMap((source) => source.resistances.filter((entry) => entry.damageType === resistance.damageType)
      .map((entry) => `${origin(source)}：${f(`resistance-level-${entry.level}`)}`));
    res.append(row(`resistance-${resistance.damageType}`, f(`damage-type-${resistance.damageType}-name`), value, lines, f("trait-resistance-rule")));
  }
  const abilities = section("trait-abilities");
  const sustains = section("trait-sustains");
  const senses = section("trait-senses");
  if (data.tomteHeavyHeadgear !== undefined) {
    senses.append(row("tomte-headgear", f("trait-tomte-headgear"),
      f(data.tomteHeavyHeadgear ? "trait-tomte-headgear-heavy" : "trait-tomte-headgear-light"),
      [f("trait-tomte-headgear-rule")]));
  }
  abilities.append(row("free-action", f("trait-free-action"), active(traitActionProtection(data)),
    data.sources.filter((source) => source.statusImmunities.includes("rfb.status.paralysis")).map(origin), f("trait-free-action-rule")));
  for (const passive of data.passives) {
    const counted = ["hold-life", "see-invisible"].includes(passive.passive);
    const sustain = passive.passive.startsWith("sustain-");
    const sense = passive.passive.startsWith("esp-") || ["telepathy", "see-invisible"].includes(passive.passive);
    const value = counted ? f("trait-count-value", { active: active(passive.active), count: passive.sourceCount ?? unknown }) : active(passive.active);
    const name = passive.passive === "regeneration" ? f("trait-equipment-regeneration") : f(`item-passive-${passive.passive}`);
    const note = passive.passive === "hold-life" ? "trait-hold-life-rule"
      : passive.passive === "see-invisible" ? "trait-see-invisible-rule"
      : sustain ? "trait-sustain-rule" : passive.passive.startsWith("esp-") ? "trait-targeted-sense-rule" : sense ? "trait-sense-rule"
      : passive.passive === "regeneration" ? "trait-regeneration-rule" : "trait-boolean-rule";
    (sustain ? sustains : sense ? senses : abilities).append(row(`passive-${passive.passive}`, name, value,
      data.sources.filter((source) => source.passives.includes(passive.passive)).map(origin),
      `${f(note)}${counted ? ` ${f("trait-count-rule")}` : ""}`));
  }
  abilities.append(row("reflect", f("trait-reflection"), active(data.reflectsBolts), data.sources.filter((source) => source.reflectsBolts).map(origin), f("trait-reflection-rule")));
  abilities.append(row("pass-walls", f("trait-pass-walls"), active(data.passesWalls), data.sources.filter((source) => source.passesWalls).map(origin)));
  const immunities = section("trait-immunities");
  const knownImmunities = [...new Set(data.sources.flatMap((source) => source.statusImmunities))];
  // Paralysis protection is already shown as free action, including when absent/unknown.
  const otherImmunities = (data.statusImmunities ?? knownImmunities).filter((id) => id !== "rfb.status.paralysis");
  for (const id of otherImmunities) {
    immunities.append(row(`immunity-${id}`, statusName(id), f("trait-active"),
      data.sources.filter((source) => source.statusImmunities.includes(id)).map(origin), f("trait-immunity-rule")));
  }
  if (!otherImmunities.length) immunities.append(text("p", data.statusImmunities ? f("trait-no-immunities") : unknown));
  const numeric = section("trait-numeric");
  for (const stat of data.stats) {
    if (ATTACK_STATS.includes(stat.id)) continue;
    numeric.append(row(`stat-${stat.id}`, f(`trait-stat-${stat.id}`), traitStatValue(stat, localization),
      stat.sources.map((source) => `${sourceName(source.sourceId)}：${traitStatSourceValue(stat, source.amount, localization)}`),
      f(stat.id === "equipment-life" ? "trait-life-rule" : stat.id === "mutation-regeneration" ? "trait-mutation-regeneration-rule"
        : stat.id === "natural-regeneration" ? "trait-natural-regeneration-rule" : "trait-stat-rule")));
  }
  const grid = text("div", "", "trait-sections");
  grid.append(res, abilities, sustains, senses, immunities, numeric);
  defenses.replaceChildren(text("p", f("trait-guide"), "attribute-source-guide"),
    ...(data.equipmentComplete ? [] : [text("p", f("trait-incomplete"), "attribute-source-guide")]), grid);
  if (data.sources.some((source) => source.kind === "race" && source.sourceId === "rfb-legacy.race.ent")) {
    defenses.insertBefore(row("ent-rules", f("race-legacy-ent-name"), f("trait-race-effects"),
      ["basics", "growth", "digging", "fire", "diet", "forest", "power", "birth"].map((rule) => f(`trait-ent-rule-${rule}`)), undefined, true), grid);
  }
  const offense = section("trait-attack-sources");
  data.attacks.forEach((entry, index) => {
    const lines = entry.slays.map((slay) => f(slay.level === "kill" ? "item-kill-label" : "item-slay-label", { target: f(`slay-target-${slay.target}-name`) }));
    lines.push(...entry.brands.map((brand) => f("item-brand-label", { brand: f(`weapon-brand-${brand}-name`) })));
    if (entry.vampiric) lines.push(f("item-passive-vampiric"));
    offense.append(row(`attack-${entry.scope}-${entry.sourceId}-${index}`, sourceName(entry.sourceId), f(`trait-scope-${entry.scope}`),
      lines, f(`trait-scope-rule-${entry.scope}`)));
  });
  if (!data.attacks.length) offense.append(text("p", f("trait-no-attack-sources")));
  const weapons = section("trait-weapons");
  for (const item of items) {
    if (!("slotId" in item)) continue;
    const slotType = bodySlots.find((slot) => slot.id === item.slotId)?.slotType;
    if (slotType !== "weapon" && slotType !== "launcher") continue;
    const selected = item.id === (slotType === "weapon" ? data.activeWeaponId : data.activeLauncherId);
    const bonus = slotType === "weapon" ? item.equipmentBonuses?.meleeAttacks : item.equipmentBonuses?.baseShotDeltaPercent;
    const lines = bonus ? [f(slotType === "weapon" ? "trait-known-melee-bonus" : "trait-known-shot-bonus", { value: bonus })] : [];
    weapons.append(row(`weapon-${item.id}`, sourceName(item.id), f(selected ? "trait-selected-attack" : "trait-unselected-attack"), lines,
      f(slotType === "weapon" ? "trait-weapon-selection-rule" : "trait-launcher-selection-rule")));
  }
  if (!weapons.querySelector("details")) weapons.append(text("p", f("trait-no-weapons"), "attribute-source-guide"));
  const rates = section("trait-attack-rates");
  if (data.sources.some((source) => source.kind === "race" && source.sourceId === "rfb-legacy.race.tonberry")) {
    rates.append(row("tonberry-rules", f("race-legacy-tonberry-name"), f("trait-race-effects"),
      ["basics", "speed", "damage", "attacks", "confusion", "birth"].map((rule) => f(`trait-tonberry-rule-${rule}`)), undefined, true));
  }
  for (const stat of data.stats.filter((stat) => ATTACK_STATS.includes(stat.id))) {
    const id = stat.id === "melee-attacks-hundredths" ? data.activeWeaponId : data.activeLauncherId;
    rates.append(row(`stat-${stat.id}`, `${f(`trait-stat-${stat.id}`)} · ${id ? sourceName(id) : f("trait-unarmed")}`, traitStatValue(stat, localization),
      stat.sources.map((source) => `${sourceName(source.sourceId)}：${traitStatSourceValue(stat, source.amount, localization)}`),
      f(stat.id === "melee-attacks-hundredths" ? "trait-melee-rate-rule" : "trait-shot-rate-rule")));
  }
  const auras = section("trait-auras");
  for (const aura of data.auras) {
    auras.append(row(`aura-${aura.damageType}`, f(aura.evilOnly ? "trait-aura-holy" : `trait-aura-${aura.damageType}`), f("trait-contact-scope"),
      aura.sourceIds.map(sourceName), f(aura.evilOnly ? "trait-holy-aura-rule" : "trait-aura-rule")));
  }
  if (!data.auras.length) auras.append(text("p", f("trait-no-auras"), "attribute-source-guide"));
  const negatives = section("trait-negatives");
  for (const entry of data.negatives) {
    const lines = entry.effects.map((effect) => `${f(effect.asStealthPenalty ? "trait-curse-stealth-penalty" : `trait-curse-effect-${effect.effect}`)}：${active(effect.active)} — ${f(`trait-curse-rule-${effect.effect}`)}`);
    negatives.append(row(`negative-${entry.sourceId}`, sourceName(entry.sourceId), entry.curse ? f(`item-curse-${entry.curse}`) : f("trait-no-confirmed-curse"),
      lines, f("trait-curse-severity-rule")));
  }
  if (!data.negatives.length) negatives.append(text("p", f("trait-no-negatives"), "attribute-source-guide"));
  const meleeDamage = section("trait-melee-damage");
  data.meleeDamage.forEach((preview, index) => {
    meleeDamage.append(row(`melee-damage-${index}`, preview.attackName ?? sourceName(preview.sourceId),
      meleeDamagePreviewValue(preview, localization),
      [f("trait-melee-damage-percent", { value: preview.damagePercent })], f("trait-melee-damage-rule")));
  });
  attacks.replaceChildren(text("p", f("trait-attack-guide"), "attribute-source-guide"), weapons, rates, meleeDamage, offense, auras, negatives);
  for (const host of [defenses, attacks]) {
    const sections = [...host.querySelectorAll<HTMLElement>("[data-trait-section]")];
    const controls = text("div", "", "trait-narrow-navigation");
    const categoryLabel = text("label", f("trait-select-category"), "detail-selector");
    const category = document.createElement("select");
    category.dataset.traitFocus = `${host.id}-category`;
    const entryLabel = text("label", f("trait-select-entry"), "detail-selector");
    const entry = document.createElement("select");
    entry.dataset.traitFocus = `${host.id}-entry`;
    for (const section of sections) {
      const option = document.createElement("option");
      option.value = section.dataset.traitSection!;
      option.textContent = section.querySelector("h3")!.textContent;
      category.append(option);
      const entries = text("div", "", "trait-entries");
      entries.append(...[...section.children].filter((node) => node.tagName !== "H3"));
      section.append(entries);
    }
    category.value = sections.some((section) => section.dataset.traitSection === host.dataset.traitCategory)
      ? host.dataset.traitCategory! : sections[0]!.dataset.traitSection!;
    const selectEntry = () => {
      host.dataset.traitEntry = entry.value;
      for (const row of host.querySelectorAll<HTMLElement>(".trait-row")) {
        row.classList.toggle("trait-selected-entry", row.dataset.trait === entry.value);
      }
    };
    const selectCategory = () => {
      host.dataset.traitCategory = category.value;
      entry.replaceChildren();
      for (const section of sections) {
        const selected = section.dataset.traitSection === category.value;
        section.classList.toggle("trait-selected-category", selected);
        if (!selected) continue;
        for (const row of section.querySelectorAll<HTMLElement>(".trait-row")) {
          const option = document.createElement("option");
          option.value = row.dataset.trait!;
          option.textContent = row.querySelector(".trait-name")!.textContent;
          entry.append(option);
        }
      }
      entryLabel.hidden = !entry.options.length;
      if ([...entry.options].some((option) => option.value === host.dataset.traitEntry)) entry.value = host.dataset.traitEntry!;
      selectEntry();
    };
    category.addEventListener("change", selectCategory);
    entry.addEventListener("change", () => {
      selectEntry();
      host.querySelector<HTMLDetailsElement>(".trait-selected-entry")?.setAttribute("open", "");
    });
    selectCategory();
    categoryLabel.append(category);
    entryLabel.append(entry);
    controls.append(categoryLabel, entryLabel);
    host.prepend(controls);
  }
  if (focus) [...defenses.querySelectorAll<HTMLElement>("[data-trait-focus]"), ...attacks.querySelectorAll<HTMLElement>("[data-trait-focus]")]
    .find((summary) => summary.dataset.traitFocus === focus)?.focus({ preventScroll: true });
}
