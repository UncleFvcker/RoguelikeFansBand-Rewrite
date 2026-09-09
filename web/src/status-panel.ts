// SPDX-License-Identifier: MPL-2.0

import type { AppDom } from "./app-dom";
import type { AppState } from "./app-state";
import type { Localization, MessageKey } from "./localization";
import type {
  AbilityDto,
  AbilityLearningDto,
  AbilityStudyModeDto,
  AttributeKindDto,
  AttributeSourceDto,
  AttributeBreakdownDto,
  EquipmentItemDto,
  BodySlotDto,
  GameCommand,
  GameSnapshot,
  GameUpdate,
  PlayerBuildDto,
  PlayerDto,
  PlayerMutationDto,
  MaterialDto,
  PetUpkeepDto,
  PetDto,
  PendingRaceMutationChoiceDto,
  PlayerProgressDto,
  ProficiencyRankDto,
  ResourcePoolDto,
  SniperConcentrationDto,
  SummonCommandDto,
  SummonCommandModeDto,
  TaskStatusDto,
  TaskStatusKindDto,
  WeaponProficiencyGroupDto,
  WeaponProficiencyDto,
  VirtueDto,
} from "./protocol";
import { REST_UNTIL_RECOVERED_TURNS } from "./rest.ts";
import { goldVisualId } from "./render-world.ts";
import { equippedLightText } from "./shop-panel.ts";
import { selectJourneyDungeonStatus } from "./journey-guidance.ts";
import { renderCharacterTraitsDetails } from "./character-traits-panel.ts";

type CharacterOverviewDom = Pick<AppDom,
  | "progressionExperienceValue"
  | "progressionPersonalityValue"
  | "characterNameValue"
  | "characterRaceValue"
  | "characterClassValue"
  | "characterLevelValue"
  | "characterMaximumExperienceValue"
  | "characterNextExperienceValue"
  | "characterGoldValue"
  | "characterWorldTimeValue"
  | "characterVitalsList"
>;

type StatusDom = CharacterOverviewDom & Pick<
  AppDom,
  | "mapHost"
  | "turnValue"
  | "hpValue"
  | "healthMeter"
  | "healthMeterFill"
  | "goldValue"
  | "nutritionValue"
  | "lightValue"
  | "attackValue"
  | "defenseValue"
  | "effectsValue"
  | "positionValue"
  | "hashValue"
  | "progressionIdentityValue"
  | "hudLocationValue"
  | "hudExperience"
  | "progressionLevelValue"
  | "progressionExperienceValue"
  | "progressionCapValue"
  | "progressionPointsValue"
  | "progressionPersonalityValue"
  | "progressionMultipliersValue"
  | "attributeList"
  | "hudAttributeList"
  | "skillList"
  | "characterProficiencyTables"
  | "characterAttributeSources"
  | "characterTraitDefenses"
  | "characterTraitAttacks"
  | "materialList"
  | "virtueList"
  | "mutationList"
  | "resourceList"
  | "abilityList"
  | "resourceRest"
  | "nearbyCurrent"
  | "nearbyList"
  | "summonCommandStatus"
  | "summonCommandButtons"
  | "dismissPets"
  | "petList"
  | "taskLogList"
  | "campaignStatusValue"
  | "campaignScoreValue"
  | "campaignDungeonsValue"
  | "campaignTasksValue"
  | "campaignRetire"
  | "dungeonInfoName"
  | "dungeonInfoDepthRow"
  | "dungeonInfoDepth"
  | "dungeonInfoBossRow"
  | "dungeonInfoBoss"
>;

const ATTRIBUTE_KINDS: AttributeKindDto[] = [
  "strength",
  "intelligence",
  "wisdom",
  "dexterity",
  "constitution",
  "charisma",
];

const WILDERNESS_DAY_TICKS = 100_000;

export function renderTaskLog(list: HTMLUListElement, tasks: readonly TaskStatusDto[],
  localization: Localization, disabled: boolean, abandon: (task: TaskStatusDto) => void): void {
  const document = list.ownerDocument;
  const previous = new Map([...list.querySelectorAll<HTMLDetailsElement>('details[data-task-log-key]')]
    .map((node) => [node.dataset.taskLogKey!, node.open]));
  const active = document.activeElement as HTMLElement | null;
  const focusKey = active && list.contains(active) ? active.closest<HTMLElement>('details[data-task-log-key]')?.dataset.taskLogKey : undefined;
  const focusButton = active?.tagName === 'BUTTON';
  const scrollHost = list.parentElement!;
  const scrollTop = scrollHost.scrollTop;
  let focusTarget: HTMLElement | undefined;
  const text = (tag: string, className: string, value: string): HTMLElement => {
    const node = document.createElement(tag);
    node.className = className;
    node.textContent = value;
    return node;
  };
  const groups: readonly [string, readonly TaskStatusKindDto[]][] = [
    ['current', ['reward-available', 'active', 'taken', 'paused']],
    ['available', ['available']], ['locked', ['locked']],
    ['history', ['completed', 'failed', 'abandoned']],
  ];
  const rows: HTMLElement[] = [];
  for (const [group, statuses] of groups) {
    const entries = statuses.flatMap((status) => tasks.filter((task) => task.status === status));
    if (!entries.length) continue;
    const row = document.createElement('li');
    const section = document.createElement('details');
    section.className = 'task-log-group';
    section.dataset.taskLogKey = `group:${group}`;
    section.open = previous.get(section.dataset.taskLogKey) ?? (group === 'current' || group === 'available');
    const heading = document.createElement('summary');
    heading.append(text('span', 'task-log-group-name', localization.format(`task-log-group-${group}`)),
      text('span', 'task-log-count', localization.format('task-log-count', { count: entries.length })));
    if (focusKey === section.dataset.taskLogKey) focusTarget = heading;
    const items = document.createElement('ul');
    items.className = 'task-log-items';
    for (const task of entries) {
      const item = document.createElement('li');
      const details = document.createElement('details');
      details.className = 'task-log-task';
      details.dataset.taskLogKey = `task:${task.taskId}`;
      details.open = previous.get(details.dataset.taskLogKey) ?? false;
      const summary = document.createElement('summary');
      const title = text('span', 'task-log-name', localization.format(task.nameKey));
      const status = text('span', 'task-log-status', localization.format(`task-status-${task.status}`));
      status.dataset.status = task.status;
      summary.append(title, status);
      const progress = document.createElement('span');
      progress.className = 'task-log-progress';
      progress.append(text('span', 'task-log-objective', localization.format('task-service-progress', {
        current: task.current, required: task.required,
      })));
      if (task.stages > 1) progress.append(text('span', 'task-log-stage', localization.format('task-log-stage', {
        stage: task.stage, stages: task.stages,
      })));
      summary.append(progress);
      const body = document.createElement('div');
      body.className = 'task-log-body';
      body.append(text('p', 'task-log-description', localization.format(task.descriptionKey ?? 'task-log-no-description')));
      if (task.maxRetakes != null) body.append(text('p', 'task-log-retakes', localization.format('task-log-retakes', {
        used: task.retakesUsed, maximum: task.maxRetakes,
      })));
      if (focusKey === details.dataset.taskLogKey) focusTarget = section.open ? summary : heading;
      if (task.status === 'active' || task.status === 'paused') {
        const button = document.createElement('button');
        button.type = 'button';
        button.textContent = localization.format('action-task-abandon');
        button.disabled = disabled;
        button.addEventListener('click', () => abandon(task));
        body.append(button);
        if (focusKey === details.dataset.taskLogKey && focusButton && !disabled && section.open) focusTarget = button;
      }
      details.append(summary, body);
      item.append(details);
      items.append(item);
    }
    section.append(heading, items);
    row.append(section);
    rows.push(row);
  }
  if (!rows.length) rows.push(text('li', 'task-log-empty', localization.format('task-log-empty')));
  list.replaceChildren(...rows);
  focusTarget?.focus({ preventScroll: true });
  scrollHost.scrollTop = scrollTop;
}

export function renderHudExperience(meter: HTMLProgressElement, progress: PlayerProgressDto | undefined,
  localization: Localization): void {
  meter.hidden = !progress;
  if (!progress) return;
  const next = progress.experienceForNextLevel;
  // The projection exposes cumulative XP thresholds, not this level's starting XP.
  meter.value = next == null ? 1000 : next > 0n
    ? Math.max(0, Math.min(1000, Number(BigInt(progress.experience) * 1000n / BigInt(next)))) : 0;
  const description = localization.format("hud-experience-detail", {
    experience: String(progress.experience),
    next: next == null ? localization.format("character-no-next-level") : String(next),
  });
  meter.title = description;
  meter.setAttribute("aria-valuetext", description);
}

export function hudLocationText(state: Pick<GameSnapshot, "mapScale" | "floorId" | "town">,
  localization: Localization, contentName: (id: string) => string): string {
  if (state.mapScale === "world") return localization.format("hud-location-world");
  if (state.town) return localization.format(state.town.nameKey);
  if (state.floorId === "core.floor.wilderness") return localization.format("hud-location-wilderness");
  // Content floor IDs encode depth; the shared name key omits the numeric suffix.
  const floor = /^(demo\.floor\..+-depth)-(\d+)$/.exec(state.floorId);
  return floor ? localization.format("hud-location-depth", { name: contentName(floor[1]!), depth: floor[2]! })
    : contentName(state.floorId);
}

function attachStatTooltip(row: HTMLElement, id: string, text: string, clickTrigger?: HTMLButtonElement,
  showWhen: () => boolean = () => true): () => void {
  const tooltip = row.ownerDocument.createElement("div");
  tooltip.id = id;
  tooltip.className = "character-stat-tooltip";
  tooltip.popover = "auto";
  tooltip.setAttribute("role", "tooltip");
  tooltip.textContent = text;
  const trigger = clickTrigger ?? row;
  trigger.tabIndex = 0;
  trigger.setAttribute("aria-describedby", id);
  row.append(tooltip);
  let anchor = row.getBoundingClientRect();
  const hide = () => tooltip.hidePopover();
  const show = () => {
    if (!showWhen()) return;
    if (tooltip.matches(":popover-open")) return;
    anchor = row.getBoundingClientRect();
    const viewport = row.ownerDocument.documentElement;
    const pane = row.closest(".character-detail-pane, .character-subpage")?.getBoundingClientRect();
    const left = Math.max(8, pane?.left ?? 0);
    const right = Math.min(viewport.clientWidth - 8, pane?.right ?? viewport.clientWidth);
    const top = Math.max(8, pane?.top ?? 0);
    const bottom = Math.min(viewport.clientHeight - 8, pane?.bottom ?? viewport.clientHeight);
    tooltip.style.maxWidth = `${right - left}px`;
    tooltip.style.maxHeight = `${bottom - top}px`;
    tooltip.showPopover();
    const bounds = tooltip.getBoundingClientRect();
    tooltip.style.left = `${Math.max(left, Math.min(anchor.left, right - bounds.width))}px`;
    tooltip.style.top = `${Math.max(top, Math.min(bottom - bounds.height,
      anchor.bottom + bounds.height <= bottom ? anchor.bottom : anchor.top - bounds.height))}px`;
  };
  const leave = () => {
    if (!row.matches(":hover") && !row.contains(row.ownerDocument.activeElement)) hide();
  };
  if (clickTrigger) {
    clickTrigger.setAttribute("aria-expanded", "false");
    clickTrigger.addEventListener("click", () => tooltip.matches(":popover-open") ? hide() : show());
    row.addEventListener("focusout", (event) => {
      if (!row.contains(event.relatedTarget as Node | null)) hide();
    });
  } else {
    row.addEventListener("pointerenter", show);
    row.addEventListener("pointerleave", leave);
    // Native popover dismissal can restore focus during another show operation.
    // Wait until that operation finishes before opening the focused row's tooltip.
    row.addEventListener("focusin", () => row.ownerDocument.defaultView!.requestAnimationFrame(() => {
      if (row.isConnected && row.contains(row.ownerDocument.activeElement)) show();
    }));
    row.addEventListener("focusout", leave);
  }
  // Only listen while open: scrolling/resize dismisses a now-displaced tooltip.
  let dismissListeners: AbortController | undefined;
  tooltip.addEventListener("beforetoggle", (event) => {
    clickTrigger?.setAttribute("aria-expanded", String((event as ToggleEvent).newState === "open"));
    dismissListeners?.abort();
    if ((event as ToggleEvent).newState !== "open") return;
    dismissListeners = new AbortController();
    const options = { capture: true, signal: dismissListeners.signal };
    row.ownerDocument.addEventListener("scroll", (event) => {
      const current = row.getBoundingClientRect();
      if (event.target !== tooltip && (current.top !== anchor.top || current.left !== anchor.left)) hide();
    }, options);
    row.ownerDocument.defaultView?.addEventListener("resize", hide, options);
  });
  tooltip.addEventListener("toggle", (event) => {
    if ((event as ToggleEvent).newState === "closed") dismissListeners?.abort();
  });
  return hide;
}

export function attributeSourceCell(sources: readonly AttributeSourceDto[], localization: Localization): string {
  if (!sources.length) return "—";
  const values = sources.filter((source) => source.modifier !== 0 || source.kind === "normal-appearance").map((source) =>
    source.kind === "normal-appearance" ? localization.format("attribute-source-rule")
      : `${signedModifier(source.modifier)}${source.suppressed ? localization.format("attribute-source-suppressed-short") : ""}`);
  const text = values.join(" · ") || "0";
  return sources.some((source) => !source.complete)
    ? localization.format("attribute-source-known-value", { value: text }) : text;
}

export function renderCharacterAttributeSources(
  host: HTMLElement, rows: readonly AttributeBreakdownDto[], equipment: readonly EquipmentItemDto[],
  slots: readonly BodySlotDto[], mutations: readonly PlayerMutationDto[], localization: Localization,
  statusName: (id: string | undefined) => string,
): void {
  const document = host.ownerDocument;
  const openPanel = host.querySelector<HTMLElement>(".attribute-source-detail:popover-open");
  const openAttribute = openPanel?.dataset.attribute;
  const previousOpener = openPanel?.dataset.sourceOpener;
  const detailScroll = openPanel?.querySelector(".attribute-source-detail-body")?.scrollTop ?? 0;
  const equipmentExpanded = openPanel?.querySelector("details")?.open ?? false;
  const inlineExpanded = host.querySelector<HTMLDetailsElement>(".attribute-source-inline details")?.open ?? false;
  const focused = host.contains(document.activeElement)
    ? (document.activeElement as HTMLElement).dataset.sourceFocus : undefined;
  for (const popover of host.querySelectorAll<HTMLElement>(":popover-open")) popover.hidePopover();
  const text = (tag: string, value: string, className = "") => {
    const element = document.createElement(tag);
    element.textContent = value;
    element.className = className;
    return element;
  };
  if (!rows.length) {
    host.replaceChildren(text("p", localization.format("progression-unavailable"), "character-detail-empty"));
    return;
  }
  const note = text("p", localization.format("attribute-source-guide"), "attribute-source-guide");
  const table = document.createElement("table");
  table.className = "attribute-source-table";
  table.setAttribute("aria-label", localization.format("character-detail-sources"));
  const columns = ["attribute", "base", "race", "class", "personality", "equipment", "other", "current"] as const;
  const header = table.createTHead().insertRow();
  for (const column of columns) {
    const th = document.createElement("th");
    th.scope = "col";
    th.textContent = localization.format(`attribute-source-column-${column}` as MessageKey);
    header.append(th);
  }
  const body = table.createTBody();
  const panels: HTMLElement[] = [];
  for (const row of rows) {
    const name = localization.format(`attribute-${row.attribute}` as MessageKey);
    const detail = document.createElement("div");
    detail.className = "attribute-source-detail";
    detail.id = `attribute-source-detail-${row.attribute}`;
    detail.dataset.attribute = row.attribute;
    detail.popover = "auto";
    detail.setAttribute("role", "dialog");
    detail.setAttribute("aria-labelledby", `${detail.id}-title`);
    const heading = text("h3", localization.format("attribute-source-detail-title", { name }));
    heading.id = `${detail.id}-title`;
    const close = document.createElement("button");
    close.type = "button";
    close.textContent = localization.format("attribute-source-close");
    close.dataset.sourceFocus = `${row.attribute}-close`;
    const top = document.createElement("header");
    top.append(heading, close);
    const content = document.createElement("div");
    content.className = "attribute-source-detail-body";
    content.append(text("p", localization.format("attribute-source-range", {
      base: formatAttributeValue(row.natural), current: formatAttributeValue(row.effective),
      minimum: formatAttributeValue(row.minimum), maximum: formatAttributeValue(row.maximum),
    })), text("p", localization.format("attribute-source-guide")));
    if (row.sources.some((source) => !source.complete)) {
      content.append(text("p", localization.format("attribute-source-incomplete"), "attribute-source-warning"));
    }
    const list = document.createElement("ol");
    for (const source of row.sources) {
      const entry = document.createElement("li");
      const kind = localization.format(`attribute-source-kind-${source.kind}` as MessageKey);
      const sourceName = source.nameKey ? localization.format(source.nameKey as MessageKey)
        : source.kind === "mutation" ? mutations.find((mutation) => mutation.id === source.sourceId)?.name
        : source.kind === "temporary-effect" ? statusName(source.sourceId ?? undefined) : undefined;
      const label = sourceName ? `${kind} · ${sourceName}` : kind;
      const result = source.effectiveAfter == null ? localization.format("attribute-source-result-hidden")
        : formatAttributeValue(source.effectiveAfter);
      const line = localization.format("attribute-source-step", {
        name: label, modifier: attributeSourceCell([source], localization), result,
      });
      entry.append(text("p", line));
      if (source.upperLimitApplied) entry.append(text("p", localization.format("attribute-source-capped"), "attribute-source-warning"));
      if (source.suppressed) entry.append(text("p", localization.format("attribute-source-suppressed"), "attribute-source-warning"));
      if (source.kind === "normal-appearance" && row.normalAppearanceMinimum != null) {
        entry.append(text("p", localization.format("attribute-source-charisma-floor", {
          value: formatAttributeValue(row.normalAppearanceMinimum),
        })));
      }
      if (source.kind === "equipment") {
        const items = equipment.filter((item) => slots.find((slot) => slot.id === item.slotId)?.slotType !== "tool");
        const equipmentDetails = document.createElement("details");
        const summary = text("summary", localization.format("attribute-source-equipment-details"));
        summary.dataset.sourceFocus = `${row.attribute}-equipment-detail`;
        equipmentDetails.append(summary);
        for (const item of items) {
          const slot = slots.find((slot) => slot.id === item.slotId)!;
          const known = item.knowledge === "aware" && item.identification === "identified";
          const value = signedModifier(item.modifiers[row.attribute]);
          equipmentDetails.append(text("p", localization.format("attribute-source-item", {
            slot: `${localization.format(`equipment-slot-${slot.slotType}` as MessageKey)} (${slot.id})`,
            name: localization.format(item.displayNameKey as MessageKey),
            value: known ? value : localization.format("attribute-source-known-value", { value }),
          })));
        }
        if (!items.length) equipmentDetails.append(text("p", localization.format("attribute-source-no-equipment")));
        entry.append(equipmentDetails);
      }
      list.append(entry);
    }
    content.append(list);
    detail.append(top, content);
    panels.push(detail);
    const tr = body.insertRow();
    let opener: HTMLButtonElement | undefined;
    const triggers: HTMLButtonElement[] = [];
    close.addEventListener("click", () => {
      detail.hidePopover();
      (opener ?? host.querySelector<HTMLButtonElement>(`[data-source-focus="${detail.dataset.sourceOpener}"]`))?.focus();
    });
    detail.addEventListener("beforetoggle", (event) => {
      for (const trigger of triggers) trigger.setAttribute("aria-expanded", String((event as ToggleEvent).newState === "open"));
    });
    for (const column of columns) {
      const sources = row.sources.filter((source) => column === "other"
        ? ["mutation", "temporary-effect", "normal-appearance"].includes(source.kind) : source.kind === column);
      const value = column === "attribute" ? name : column === "base" ? formatAttributeValue(row.natural)
        : column === "current" ? formatAttributeValue(row.effective) : attributeSourceCell(sources, localization);
      const cell = document.createElement(column === "attribute" ? "th" : "td");
      if (cell instanceof HTMLTableCellElement && column === "attribute") cell.scope = "row";
      const trigger = document.createElement("button");
      trigger.type = "button";
      trigger.textContent = value;
      trigger.dataset.sourceFocus = `${row.attribute}-${column}`;
      trigger.setAttribute("aria-label", localization.format("attribute-source-cell-label", {
        name, column: localization.format(`attribute-source-column-${column}` as MessageKey), value,
      }));
      trigger.setAttribute("aria-controls", detail.id);
      trigger.setAttribute("aria-haspopup", "dialog");
      trigger.setAttribute("aria-expanded", "false");
      cell.append(trigger);
      const hideTooltip = attachStatTooltip(cell, `attribute-source-tip-${row.attribute}-${column}`,
        `${name} · ${localization.format(`attribute-source-column-${column}` as MessageKey)}: ${value}\n${localization.format("attribute-source-hover")}`,
        undefined, () => !host.querySelector(".attribute-source-detail:popover-open"));
      cell.tabIndex = -1; // The button is the single keyboard stop for this cell.
      trigger.setAttribute("aria-describedby", `attribute-source-tip-${row.attribute}-${column}`);
      trigger.addEventListener("click", () => {
        hideTooltip();
        opener = trigger;
        detail.dataset.sourceOpener = trigger.dataset.sourceFocus;
        detail.showPopover();
        close.focus();
      });
      triggers.push(trigger);
      tr.append(cell);
    }
  }
  const matrix = text("div", "", "attribute-source-matrix");
  matrix.append(table);
  const narrow = text("div", "", "attribute-source-narrow");
  const label = text("label", localization.format("attribute-source-column-attribute"), "detail-selector");
  const select = document.createElement("select");
  select.dataset.sourceFocus = "narrow-select";
  for (const row of rows) {
    const option = document.createElement("option");
    option.value = row.attribute;
    option.textContent = localization.format(`attribute-${row.attribute}` as MessageKey);
    select.append(option);
  }
  select.value = rows.some((row) => row.attribute === host.dataset.sourceAttribute)
    ? host.dataset.sourceAttribute! : rows[0]!.attribute;
  const inline = text("div", "", "attribute-source-inline");
  const selectAttribute = () => {
    host.dataset.sourceAttribute = select.value;
    // Reuse formatted, knowledge-filtered sources. Native details need no handlers.
    const content = panels.find((panel) => panel.dataset.attribute === select.value)!
      .querySelector(".attribute-source-detail-body")!.cloneNode(true) as HTMLElement;
    content.className = "attribute-source-inline-body";
    content.querySelectorAll("[data-source-focus]").forEach((element) => element.removeAttribute("data-source-focus"));
    const equipmentDetails = content.querySelector("details");
    if (equipmentDetails) equipmentDetails.open = inlineExpanded;
    inline.replaceChildren(content);
  };
  select.addEventListener("change", selectAttribute);
  selectAttribute();
  label.append(select);
  narrow.append(label, inline);
  host.replaceChildren(note, matrix, narrow, ...panels);
  if (openAttribute) {
    const panel = panels.find((panel) => panel.dataset.attribute === openAttribute);
    if (panel) {
      if (previousOpener) panel.dataset.sourceOpener = previousOpener;
      panel.showPopover();
      panel.querySelector("details")!.open = equipmentExpanded;
      panel.querySelector(".attribute-source-detail-body")!.scrollTop = detailScroll;
    }
  }
  if (focused) host.querySelector<HTMLElement>(`[data-source-focus="${focused}"]`)?.focus();
}

export function renderCharacterTraits(
  dom: Pick<AppDom, "attributeList" | "hudAttributeList" | "skillList">,
  progress: PlayerProgressDto,
  state: Pick<AppState, "busy" | "playerDead" | "worldMap">,
  localization: Localization,
  dispatch: (command: GameCommand) => Promise<void>,
): void {
  const document = dom.attributeList.ownerDocument;
  // Close before replacement, including the temporary document listeners.
  for (const tooltip of document.querySelectorAll<HTMLElement>(".character-stat-tooltip:popover-open")) {
    tooltip.hidePopover();
  }
  const attributeRows = ATTRIBUTE_KINDS.map((attribute) => {
    const value = progress.attributes[attribute];
    const row = document.createElement("li");
    row.className = "attribute-row";
    const label = document.createElement("span");
    label.className = "attribute-name";
    label.textContent = localization.format(`attribute-${attribute}` as MessageKey);
    const current = document.createElement("span");
    current.className = "attribute-value";
    current.textContent = formatAttributeValue(value.effective);
    row.append(label, current);
    return row;
  });
  dom.hudAttributeList.replaceChildren(...attributeRows.map((row) => row.cloneNode(true)));
  attributeRows.forEach((row, index) => {
    const attribute = ATTRIBUTE_KINDS[index]!;
    const value = progress.attributes[attribute];
    attachStatTooltip(row, `character-attribute-${attribute}-tooltip`, localization.format("character-attribute-detail", {
      natural: formatAttributeValue(value.natural),
      maximumNatural: formatAttributeValue(value.maximumNatural),
      potential: formatAttributeValue(value.potential),
      effective: formatAttributeValue(value.effective),
      index: value.index,
    }));
    if (progress.pendingAttributeIncreases > 0) {
      const increase = document.createElement("button");
      increase.type = "button";
      increase.className = "attribute-increase";
      increase.textContent = localization.format("action-increase-attribute");
      increase.disabled = state.busy || state.playerDead || state.worldMap ||
        value.maximumNatural >= Math.min(progress.attributeCap, value.potential);
      increase.addEventListener("click", () => void dispatch({ type: "increase-attribute", attribute }));
      row.append(increase);
    }
  });
  dom.attributeList.replaceChildren(...attributeRows);
  dom.skillList.replaceChildren(...progress.skills.map((skill, index) => {
    const row = document.createElement("li");
    row.className = "skill-row";
    const name = document.createElement("span");
    name.className = "skill-name";
    name.textContent = localization.format(skill.nameKey as MessageKey);
    const value = document.createElement("span");
    value.className = "skill-value";
    value.textContent = String(skill.current);
    row.append(name, value);
    attachStatTooltip(row, `character-skill-${index}-tooltip`, `${name.textContent}\n${localization.format("character-skill-detail", {
      current: skill.current, maximum: skill.maximum, growth: skill.growthPerTenLevels,
    })}`);
    return row;
  }));
}

export type WildernessClock = {
  day: number;
  hour: number;
  minute: number;
  daytime: boolean;
};

export function wildernessClock(worldTick: number): WildernessClock {
  const withinDay = worldTick % WILDERNESS_DAY_TICKS;
  const clockTick = (withinDay + WILDERNESS_DAY_TICKS / 4) % WILDERNESS_DAY_TICKS;
  const minuteOfDay = Math.floor((clockTick * 24 * 60) / WILDERNESS_DAY_TICKS);
  return {
    day: Math.floor((worldTick + WILDERNESS_DAY_TICKS / 4) / WILDERNESS_DAY_TICKS) + 1,
    hour: Math.floor(minuteOfDay / 60),
    minute: minuteOfDay % 60,
    daytime: withinDay < WILDERNESS_DAY_TICKS / 2,
  };
}

export function abilityConfirmationMessageKey(abilityId: string): MessageKey | undefined {
  return abilityId === "rfb.ability.race.devour-flesh"
    ? "confirm-ability-devour-flesh"
    : undefined;
}

export function renderCharacterOverview(
  dom: CharacterOverviewDom,
  player: PlayerDto,
  worldTick: number,
  localization: Localization,
): void {
  const unavailable = localization.format("progression-unavailable");
  const { build, progress } = player;
  dom.characterNameValue.textContent = player.name;
  dom.characterNameValue.title = player.name;
  dom.characterRaceValue.textContent = build ? localization.format(build.raceNameKey) : unavailable;
  dom.characterClassValue.textContent = build ? localization.format(build.classNameKey) : unavailable;
  dom.progressionPersonalityValue.textContent = build ? localization.format(build.personalityNameKey) : unavailable;
  dom.characterLevelValue.textContent = progress ? String(progress.level) : unavailable;
  dom.progressionExperienceValue.textContent = progress ? String(progress.experience) : unavailable;
  dom.characterMaximumExperienceValue.textContent = progress ? String(progress.maximumExperience) : unavailable;
  dom.characterNextExperienceValue.textContent = progress
    ? progress.experienceForNextLevel == null ? localization.format("character-no-next-level") : String(progress.experienceForNextLevel)
    : unavailable;
  dom.characterGoldValue.textContent = player.gold.toLocaleString(localization.locale);
  const clock = wildernessClock(worldTick);
  dom.characterWorldTimeValue.textContent = localization.format("character-world-time-value", {
    day: clock.day, hour: String(clock.hour).padStart(2, "0"), minute: String(clock.minute).padStart(2, "0"),
  });
  const values: [MessageKey, string][] = [
    ["status-health", localization.format("status-health-value", { hp: player.hp, maxHp: player.maxHp })],
    ...(player.resources ?? []).map((resource): [MessageKey, string] =>
      [resource.nameKey, `${resource.current} / ${resource.maximum}`]),
  ];
  if (progress) values.push(["status-life-force", `${progress.lifeForce} / 1000`]);
  if (player.sniperConcentration) {
    values.push(["sniper-concentration", `${player.sniperConcentration.current} / ${player.sniperConcentration.maximum}`]);
  }
  values.push(["character-armor-class", String(player.armorClass)], ["character-speed", String(player.speed)]);
  const document = dom.characterVitalsList.ownerDocument;
  dom.characterVitalsList.replaceChildren(...values.map(([key, value]) => {
    const row = document.createElement("div");
    const label = document.createElement("dt");
    label.textContent = localization.format(key);
    const text = document.createElement("dd");
    text.textContent = value;
    row.append(label, text);
    return row;
  }));
}

export function weaponProficienciesByGroup(
  proficiencies: readonly WeaponProficiencyDto[],
  group: WeaponProficiencyGroupDto,
): WeaponProficiencyDto[] {
  return proficiencies.filter((proficiency) => proficiency.group === group);
}

export function proficiencyRankMessageKey(rank: ProficiencyRankDto): MessageKey {
  return `proficiency-rank-${rank}` as MessageKey;
}

export function renderCharacterProficiencies(
  host: HTMLElement, progress: PlayerProgressDto, localization: Localization,
): void {
  for (const tooltip of host.querySelectorAll<HTMLElement>(":popover-open")) tooltip.hidePopover();
  const document = host.ownerDocument;
  const selectedGroup = host.querySelector<HTMLInputElement>("input:checked")?.value;
  let rowIndex = 0;
  const makeRow = (name: string, rank: ProficiencyRankDto, details: string, equipped = false) => {
    const row = document.createElement("tr");
    const cell = document.createElement("td");
    const button = document.createElement("button");
    button.type = "button";
    button.className = "proficiency-entry";
    button.classList.toggle("is-equipped", equipped);
    const label = document.createElement("span");
    label.className = "proficiency-name";
    label.textContent = name;
    button.append(label);
    if (equipped) {
      const marker = document.createElement("span");
      marker.className = "proficiency-equipped";
      marker.textContent = localization.format("proficiency-equipped");
      button.append(marker);
    }
    const value = document.createElement("span");
    value.className = "proficiency-rank";
    value.dataset.rank = rank;
    value.textContent = `[${localization.format(proficiencyRankMessageKey(rank))}]`;
    button.append(value);
    cell.append(button);
    attachStatTooltip(cell, `character-proficiency-${rowIndex++}-detail`, `${name}\n${details}`, button);
    row.append(cell);
    return row;
  };
  const makeTable = (group: string, title: string, rows: HTMLTableRowElement[]) => {
    const table = document.createElement("table");
    table.className = "proficiency-table";
    table.dataset.group = group;
    const heading = document.createElement("th");
    heading.scope = "col";
    heading.textContent = title;
    const headRow = table.createTHead().insertRow();
    headRow.append(heading);
    table.createTBody().append(...rows);
    return table;
  };
  const groups: WeaponProficiencyGroupDto[] = ["sword", "polearm", "bow", "hafted", "digging", "other"];
  // Chromium does not repeat table headers across CSS columns. Split long groups
  // into named, unbroken tables sized from this projection, without spacer rows.
  const rowsPerTable = Math.max(1, Math.ceil((progress.weaponProficiencies.length + 2) / 3));
  const tables = groups.flatMap((group) => {
    const entries = weaponProficienciesByGroup(progress.weaponProficiencies, group);
    const parts: HTMLTableElement[] = [];
    for (let offset = 0; offset < entries.length; offset += rowsPerTable) {
      const title = localization.format(`proficiency-group-${group}` as MessageKey);
      parts.push(makeTable(group, offset ? localization.format("proficiency-continued", { name: title }) : title,
        entries.slice(offset, offset + rowsPerTable).map((entry) =>
          makeRow(localization.format(entry.nameKey as MessageKey), entry.rank,
            localization.format("weapon-proficiency-value", {
              rank: localization.format(proficiencyRankMessageKey(entry.rank)),
              current: entry.current, maximum: entry.maximum,
              hit: `${entry.hitBonus >= 0 ? "+" : ""}${entry.hitBonus}`,
            }), entry.equipped))));
    }
    return parts;
  });
  const riding = progress.ridingProficiency;
  const mining = progress.miningProficiency;
  tables.push(makeTable("misc", localization.format("proficiency-group-misc"), [
    makeRow(localization.format("riding-proficiency"), riding.rank,
      localization.format("riding-proficiency-value", {
        rank: localization.format(proficiencyRankMessageKey(riding.rank)), current: riding.current, maximum: riding.maximum,
      })),
    makeRow(localization.format("mining-proficiency"), mining.rank,
      localization.format("mining-proficiency-value", {
        rank: localization.format(proficiencyRankMessageKey(mining.rank)), current: mining.current,
        maximum: mining.maximum, power: mining.diggingPower,
      })),
  ]));
  const switcher = document.createElement("div");
  switcher.className = "character-group-switch";
  switcher.setAttribute("role", "group");
  switcher.setAttribute("aria-label", localization.format("character-proficiency-groups"));
  const available = [...new Set(tables.map((table) => table.dataset.group!))];
  const selected = available.includes(selectedGroup ?? "") ? selectedGroup : available[0];
  for (const group of available) {
    const label = document.createElement("label");
    const radio = document.createElement("input");
    radio.type = "radio";
    radio.name = "character-proficiency-group";
    radio.value = group;
    radio.checked = group === selected;
    const name = document.createElement("span");
    name.textContent = localization.format(`proficiency-group-${group}` as MessageKey);
    label.append(radio, name);
    switcher.append(label);
  }
  host.replaceChildren(switcher, ...tables);
}

export function renderCharacterOtherLists(
  dom: Pick<AppDom, "virtueList" | "materialList">,
  virtues: VirtueDto[], materials: MaterialDto[], localization: Localization,
): void {
  const render = (list: HTMLUListElement, entries: [string, number][], emptyKey: MessageKey) => {
    const document = list.ownerDocument;
    const rows = entries.map(([name, value]) => {
      const row = document.createElement("li");
      row.className = "character-other-row";
      const label = document.createElement("span");
      label.className = "character-other-name";
      label.textContent = name;
      const amount = document.createElement("span");
      amount.className = "character-other-value";
      amount.textContent = String(value);
      row.append(label, amount);
      return row;
    });
    if (!rows.length) {
      const empty = document.createElement("li");
      empty.className = "character-other-empty";
      empty.textContent = localization.format(emptyKey);
      rows.push(empty);
    }
    list.replaceChildren(...rows);
  };
  render(dom.virtueList, virtues.map((entry) => [localization.format(`virtue-${entry.kind}`), entry.value]), "character-virtues-empty");
  render(dom.materialList, materials.map((entry) => [localization.format(entry.nameKey as MessageKey), entry.quantity]), "character-materials-empty");
}

export function renderCharacterMutations(
  list: HTMLUListElement, mutations: PlayerMutationDto[],
  pendingChoice: PendingRaceMutationChoiceDto | null | undefined,
  state: Pick<AppState, "busy" | "playerDead" | "campaignEnded">,
  localization: Localization, dispatch: (command: GameCommand) => Promise<void>,
): void {
  const document = list.ownerDocument;
  const expanded = new Set([...list.querySelectorAll<HTMLDetailsElement>("details[open]")]
    .map((details) => details.dataset.mutationId));
  const disclosure = (mutation: Pick<PlayerMutationDto, "id" | "name" | "description" | "rating">, key: string, locked = false) => {
    const details = document.createElement("details");
    details.className = "mutation-details";
    details.dataset.mutationId = key;
    details.open = expanded.has(key);
    const summary = document.createElement("summary");
    const heading = document.createElement("span");
    heading.className = "mutation-heading";
    const name = document.createElement("span");
    name.className = "mutation-name";
    name.textContent = mutation.name;
    const badges = document.createElement("span");
    badges.className = "mutation-badges";
    const rating = document.createElement("span");
    rating.className = "mutation-rating";
    rating.textContent = localization.format(mutationRatingMessageKey(mutation.rating));
    badges.append(rating);
    if (locked) {
      const badge = document.createElement("span");
      badge.className = "mutation-locked";
      badge.textContent = localization.format("mutation-locked");
      badges.append(badge);
    }
    heading.append(name, badges);
    summary.append(heading);
    const description = document.createElement("p");
    description.className = "mutation-description";
    description.textContent = mutation.description;
    details.append(summary, description);
    return details;
  };
  const rows: HTMLLIElement[] = [];
  if (pendingChoice) {
    const choice = document.createElement("li");
    choice.className = "mutation-choice-card";
    const title = document.createElement("strong");
    title.textContent = localization.format("mutation-choice-required");
    const prompt = document.createElement("p");
    prompt.textContent = localization.format("mutation-choice-prompt");
    choice.append(title, prompt);
    for (const candidate of pendingChoice.candidates) {
      const row = document.createElement("div");
      row.className = "mutation-choice-row";
      const button = document.createElement("button");
      button.type = "button";
      button.className = "mutation-choice-candidate";
      button.textContent = localization.format("mutation-choice-select");
      button.setAttribute("aria-label", localization.format("mutation-choice-action", { mutation: candidate.name }));
      button.disabled = state.busy || state.playerDead || state.campaignEnded;
      button.addEventListener("click", () => void dispatch({
        type: "choose-race-mutation", rewardId: pendingChoice.rewardId, mutationId: candidate.id,
      }));
      row.append(disclosure(candidate, `choice:${pendingChoice.rewardId}:${candidate.id}`), button);
      choice.append(row);
    }
    rows.push(choice);
  }
  for (const mutation of mutations) {
    const row = document.createElement("li");
    row.className = "mutation-row";
    row.dataset.rating = mutation.rating;
    row.append(disclosure(mutation, mutation.id, mutation.locked));
    rows.push(row);
  }
  if (!rows.length) {
    const empty = document.createElement("li");
    empty.className = "character-other-empty";
    empty.textContent = localization.format("mutation-empty");
    rows.push(empty);
  }
  list.replaceChildren(...rows);
}

export class StatusPanel {
  readonly #dom: StatusDom;
  readonly #state: AppState;
  readonly #localization: Localization;
  readonly #dispatch: (command: GameCommand) => Promise<void>;
  readonly #contentName: (id: string | undefined) => string;
  readonly #statusName: (id: string | undefined) => string;
  readonly #selectItemTarget: (
    excludedItemId: string | undefined,
    onSelect: (itemId: string) => Promise<void>,
  ) => void;
  readonly #startAbilityTargeting: (ability: AbilityDto) => void;
  readonly #reconcileTargeting: (state: GameSnapshot | GameUpdate) => void;
  readonly #renderTargeting: () => void;
  readonly #refreshInventoryActions: () => void;
  #worldId: string | undefined;
  #installed = false;

  constructor(options: {
    dom: StatusDom;
    state: AppState;
    localization: Localization;
    dispatch: (command: GameCommand) => Promise<void>;
    contentName: (id: string | undefined) => string;
    statusName: (id: string | undefined) => string;
    selectItemTarget: (
      excludedItemId: string | undefined,
      onSelect: (itemId: string) => Promise<void>,
    ) => void;
    startAbilityTargeting: (ability: AbilityDto) => void;
    reconcileTargeting: (state: GameSnapshot | GameUpdate) => void;
    renderTargeting: () => void;
    refreshInventoryActions: () => void;
  }) {
    this.#dom = options.dom;
    this.#state = options.state;
    this.#localization = options.localization;
    this.#dispatch = options.dispatch;
    this.#contentName = options.contentName;
    this.#statusName = options.statusName;
    this.#selectItemTarget = options.selectItemTarget;
    this.#startAbilityTargeting = options.startAbilityTargeting;
    this.#reconcileTargeting = options.reconcileTargeting;
    this.#renderTargeting = options.renderTargeting;
    this.#refreshInventoryActions = options.refreshInventoryActions;
  }

  install(): void {
    if (this.#installed) return;
    this.#installed = true;
    this.#dom.campaignRetire.addEventListener("click", this.#handleRetire);
    this.#dom.resourceRest.addEventListener("click", this.#handleRest);
    for (const [mode, button] of Object.entries(this.#dom.summonCommandButtons) as [
      SummonCommandModeDto,
      HTMLButtonElement,
    ][]) {
      button.addEventListener("click", this.#summonCommandHandlers[mode]);
    }
    this.#dom.dismissPets.addEventListener("click", this.#handleDismissPets);
  }

  dispose(): void {
    if (!this.#installed) return;
    this.#installed = false;
    this.#dom.campaignRetire.removeEventListener("click", this.#handleRetire);
    this.#dom.resourceRest.removeEventListener("click", this.#handleRest);
    for (const [mode, button] of Object.entries(this.#dom.summonCommandButtons) as [
      SummonCommandModeDto,
      HTMLButtonElement,
    ][]) {
      button.removeEventListener("click", this.#summonCommandHandlers[mode]);
    }
    this.#dom.dismissPets.removeEventListener("click", this.#handleDismissPets);
  }

  render(state: GameSnapshot | GameUpdate): void {
    this.#state.status = state;
    if ("bodySlots" in state) this.#state.bodySlots = state.bodySlots;
    if ("worldId" in state) this.#worldId = state.worldId;
    this.#state.playerDead = state.player.isDead;
    this.#state.campaignEnded = state.campaign.status === "retired";
    this.#reconcileTargeting(state);
    this.#dom.mapHost.ownerDocument.documentElement.dataset.playerState = this.#state.playerDead
      ? "dead"
      : "alive";
    const clock = wildernessClock(state.worldTick);
    this.#dom.turnValue.textContent = this.#localization.format("status-turn-time", {
      turn: state.turn,
      day: clock.day,
      hour: String(clock.hour).padStart(2, "0"),
      minute: String(clock.minute).padStart(2, "0"),
      phase: this.#localization.format(
        clock.daytime ? "wilderness-daytime" : "wilderness-nighttime",
      ),
    });
    this.#dom.hpValue.textContent = this.#localization.format(
      state.player.equipmentModifiers.maxHp > 0
        ? "status-health-value-bonus"
        : "status-health-value",
      {
        hp: state.player.hp,
        maxHp: state.player.maxHp,
        bonus: state.player.equipmentModifiers.maxHp,
      },
    );
    const healthRatio =
      state.player.maxHp > 0 ? Math.max(0, Math.min(1, state.player.hp / state.player.maxHp)) : 0;
    this.#dom.healthMeterFill.style.width = `${healthRatio * 100}%`;
    this.#dom.healthMeter.dataset.healthState =
      healthRatio <= 0.25 ? "critical" : healthRatio <= 0.5 ? "wounded" : "healthy";
    this.#dom.healthMeter.setAttribute("role", "progressbar");
    this.#dom.healthMeter.setAttribute("aria-valuemin", "0");
    this.#dom.healthMeter.setAttribute("aria-valuemax", String(state.player.maxHp));
    this.#dom.healthMeter.setAttribute("aria-valuenow", String(state.player.hp));
    this.#dom.healthMeter.title = state.player.progress
      ? this.#localization.format("status-life-force-detail", { lifeForce: String(state.player.progress.lifeForce) })
      : "";
    this.#dom.goldValue.textContent = state.player.gold.toLocaleString(this.#localization.locale);
    this.#dom.nutritionValue.textContent = this.#localization.format("status-nutrition-value", {
      state: this.#localization.format(`nutrition-state-${state.player.nutritionState}`),
      percent: nutritionPercentage(state.player.nutrition),
    });
    this.#dom.lightValue.textContent = equippedLightText(
      state.equipment,
      this.#localization,
      this.#contentName,
    );
    const dungeon = selectJourneyDungeonStatus(state, this.#worldId);
    this.#dom.dungeonInfoName.textContent = this.#localization.format(dungeon.dungeonNameKey);
    this.#dom.dungeonInfoDepthRow.hidden =
      dungeon.currentDepth === undefined || dungeon.maximumDepth === undefined;
    this.#dom.dungeonInfoDepth.textContent =
      dungeon.currentDepth === undefined || dungeon.maximumDepth === undefined
        ? ""
        : this.#localization.format("journey-dungeon-depth", {
            current: dungeon.currentDepth,
            maximum: dungeon.maximumDepth,
          });
    this.#dom.dungeonInfoBossRow.hidden = dungeon.bossNameKey === undefined;
    this.#dom.hudLocationValue.textContent = hudLocationText(state, this.#localization, this.#contentName);
    this.#dom.hudLocationValue.title = this.#dom.hudLocationValue.textContent;
    this.#dom.dungeonInfoBoss.textContent = dungeon.bossNameKey
      ? this.#localization.format(dungeon.bossNameKey)
      : "";
    this.#renderCombatStat(
      this.#dom.attackValue,
      state.player.attack,
      state.player.equipmentModifiers.attack,
    );
    this.#renderCombatStat(
      this.#dom.defenseValue,
      state.player.defense,
      state.player.equipmentModifiers.defense,
    );
    this.#renderProgression(state.player.name, state.player.progress, state.player.build);
    renderCharacterOverview(this.#dom, state.player, state.worldTick, this.#localization);
    renderCharacterAttributeSources(this.#dom.characterAttributeSources,
      state.player.progress?.attributeSources ?? [], state.equipment, this.#state.bodySlots,
      state.player.mutations ?? [], this.#localization, this.#statusName);
    renderCharacterTraitsDetails(this.#dom.characterTraitDefenses, this.#dom.characterTraitAttacks,
      state.player, [...state.equipment, ...state.inventory], this.#localization, this.#statusName, this.#state.bodySlots);
    renderCharacterOtherLists(this.#dom, state.player.virtues, state.player.progress?.materials ?? [], this.#localization);
    renderCharacterMutations(this.#dom.mutationList,
      state.player.mutations ?? [],
      state.player.pendingRaceMutationChoice,
      this.#state, this.#localization, this.#dispatch,
    );
    this.#renderAbilities(
      state.player.abilities ?? [],
      state.player.resources ?? [],
      state.player.abilityLearning,
      state.player.progress?.level ?? 1,
      state.player.sniperConcentration,
    );
    this.#renderSummonCommand(
      state.player.summonCommand,
      state.player.petUpkeep,
      state.player.pets ?? [],
    );
    this.#renderNearby(state);
    const activeEffects = state.player.statuses.map((status) =>
      this.#localization.format("status-effect-entry", {
        status: this.#statusName(status.kindId),
        intensity: status.intensity,
        ticks: status.remainingTicks,
      }),
    );
    if (state.player.confusingStrikeReady) {
      activeEffects.push(this.#localization.format("status-effect-confusing-strike-ready"));
    }
    this.#dom.effectsValue.textContent =
      activeEffects.length === 0
        ? this.#localization.format("status-effects-none")
        : activeEffects.join(" \u00b7 ");
    this.#renderTasks(state);
    this.#dom.campaignStatusValue.textContent = this.#localization.format(
      `campaign-status-${state.campaign.status}` as MessageKey,
    );
    this.#dom.campaignScoreValue.textContent = String(state.campaign.score);
    this.#dom.campaignDungeonsValue.textContent = String(state.campaign.conqueredDungeons);
    this.#dom.campaignTasksValue.textContent = String(state.campaign.completedTasks);
    this.updateCampaignAction();
    this.#dom.positionValue.textContent = `${state.player.position.x}, ${state.player.position.y}`;
    this.#dom.hashValue.textContent = state.stateHash.slice(0, 12);
    this.#dom.hashValue.title = state.stateHash;
    this.#dom.mapHost.dataset.itemCount = String(state.items.length);
    this.#dom.mapHost.dataset.goldPileCount = String(state.goldPiles.length);
    this.#dom.mapHost.dataset.playerGold = String(state.player.gold);
    this.#dom.mapHost.dataset.playerNutrition = String(state.player.nutrition);
    this.#dom.mapHost.dataset.inventoryStackCount = String(state.inventory.length);
    this.#dom.mapHost.dataset.equipmentCount = String(state.equipment.length);
    this.#dom.mapHost.dataset.carriedWeightTenthsPound = String(
      state.player.carriedWeightTenthsPound,
    );
    this.#dom.mapHost.dataset.carryCapacityTenthsPound = String(
      state.player.carryCapacityTenthsPound,
    );
    this.#dom.mapHost.dataset.playerStatusCount = String(state.player.statuses.length);
    this.#refreshInventoryActions();
    this.#renderTargeting();
  }

  updateCampaignAction(): void {
    const state = this.#state.status;
    this.#dom.campaignRetire.disabled =
      this.#state.busy ||
      this.#state.playerDead ||
      this.#state.worldMap ||
      !state ||
      state.campaign.status !== "victorious" ||
      state.floorId !== "demo.floor.surface" ||
      state.dungeonInstanceId != null;
  }

  readonly #handleRetire = (): void => {
    void this.#dispatch({ type: "retire" });
  };

  readonly #handleRest = (): void => {
    void this.#dispatch({ type: "rest", turns: REST_UNTIL_RECOVERED_TURNS });
  };

  readonly #handleDismissPets = (): void => {
    void this.#dispatch({ type: "dismiss-pets" });
  };

  readonly #summonCommandHandlers: Record<SummonCommandModeDto, () => void> = {
    follow: () => void this.#dispatch({ type: "set-summon-command", mode: "follow" }),
    attack: () => void this.#dispatch({ type: "set-summon-command", mode: "attack" }),
    "keep-distance": () =>
      void this.#dispatch({ type: "set-summon-command", mode: "keep-distance" }),
    guard: () => void this.#dispatch({ type: "set-summon-command", mode: "guard" }),
  };

  #renderTasks(state: GameSnapshot | GameUpdate): void {
    renderTaskLog(this.#dom.taskLogList, state.tasks, this.#localization,
      this.#state.busy || this.#state.worldMap, (task) => void this.#dispatch(
        task.status === 'active' ? { type: 'abandon-task' } : { type: 'abandon-paused-task', taskId: task.taskId },
      ));
  }

  #renderProgression(
    playerName: string,
    progress: PlayerProgressDto | undefined,
    build: PlayerBuildDto | null | undefined,
  ): void {
    this.#dom.progressionIdentityValue.textContent = build
      ? this.#localization.format("progression-identity-value", {
          name: playerName,
          race: this.#localization.format(build.raceNameKey as MessageKey),
          class: this.#localization.format(build.classNameKey as MessageKey),
        })
      : playerName;
    renderHudExperience(this.#dom.hudExperience, progress, this.#localization);
    if (!progress) {
      const unavailable = this.#localization.format("progression-unavailable");
      this.#dom.progressionLevelValue.textContent = unavailable;
      this.#dom.progressionCapValue.textContent = unavailable;
      this.#dom.progressionPointsValue.textContent = unavailable;
      this.#dom.progressionMultipliersValue.textContent = unavailable;
      this.#dom.attributeList.replaceChildren();
      this.#dom.hudAttributeList.replaceChildren();
      this.#dom.skillList.replaceChildren();
      this.#dom.characterProficiencyTables.replaceChildren();
      return;
    }
    this.#dom.progressionLevelValue.textContent = this.#localization.format(
      "progression-level-value",
      { level: progress.level, maxLevel: progress.maxLevel },
    );
    this.#dom.progressionCapValue.textContent = this.#localization.format(
      "progression-cap-value",
      {
        levelCap: progress.levelCap,
        attributeCap: formatAttributeValue(progress.attributeCap),
        attributeIndexCap: progress.attributeIndexCap,
      },
    );
    this.#dom.progressionPointsValue.textContent = String(progress.pendingAttributeIncreases);
    this.#dom.progressionMultipliersValue.textContent = build
      ? this.#localization.format("progression-multipliers-value", {
          life: build.lifePercent,
          experience: build.experiencePercent,
        })
      : this.#localization.format("progression-unavailable");
    renderCharacterTraits(this.#dom, progress, this.#state, this.#localization, this.#dispatch);
    renderCharacterProficiencies(this.#dom.characterProficiencyTables, progress, this.#localization);
  }

  #renderSummonCommand(
    command: SummonCommandDto | undefined,
    upkeep: PetUpkeepDto | undefined,
    pets: PetDto[],
  ): void {
    const mode = command?.mode ?? "follow";
    const count = upkeep?.controlledPets ?? 0;
    this.#dom.summonCommandStatus.textContent = this.#localization.format(
      "summon-command-status",
      {
        mode: this.#localization.format(`summon-command-mode-${mode}` as MessageKey),
        count,
        upkeep: upkeep?.upkeepPercent ?? 0,
      },
    );
    this.#dom.summonCommandStatus.classList.toggle("warning", upkeep?.unsafeWarning ?? false);
    for (const [buttonMode, button] of Object.entries(this.#dom.summonCommandButtons) as [
      SummonCommandModeDto,
      HTMLButtonElement,
    ][]) {
      const selected = buttonMode === mode;
      button.disabled =
        this.#state.busy || this.#state.commandBlocked || this.#state.worldMap || selected;
      button.setAttribute("aria-pressed", String(selected));
    }
    this.#dom.dismissPets.disabled =
      this.#state.busy || this.#state.commandBlocked || this.#state.worldMap || count === 0;
    const document = this.#dom.petList.ownerDocument;
    this.#dom.petList.replaceChildren(
      ...pets.map((pet) => {
        const row = document.createElement("li");
        row.className = "ability-row";
        const name = document.createElement("span");
        name.className = "ability-name";
        name.textContent = this.#localization.format(pet.nameKey as MessageKey);
        const details = document.createElement("span");
        details.className = "ability-details";
        const experience =
          pet.requiredExperience == null
            ? this.#localization.format("pet-experience-none")
            : this.#localization.format("pet-experience-value", {
                current: pet.experience,
                required: pet.requiredExperience,
              });
        details.textContent = this.#localization.format("pet-status-value", {
          level: pet.level,
          experience,
          riding: pet.riding ? this.#localization.format("pet-riding") : "",
          bond:
            pet.bondPercent == null
              ? ""
              : this.#localization.format("pet-bond-value", { percent: pet.bondPercent }),
        });
        row.append(name, details);
        return row;
      }),
    );
  }


  #renderAbilities(
    abilities: AbilityDto[],
    resources: ResourcePoolDto[],
    learning: AbilityLearningDto | null | undefined,
    playerLevel: number,
    concentration: SniperConcentrationDto | null | undefined,
  ): void {
    const document = this.#dom.abilityList.ownerDocument;
    this.#dom.resourceList.replaceChildren();
    this.#dom.abilityList.replaceChildren();
    const presentation = abilityPresentation(abilities, playerLevel);
    this.#dom.resourceRest.disabled =
      this.#state.busy ||
      this.#state.playerDead ||
      this.#state.worldMap ||
      !this.#state.status ||
      (this.#state.status.player.hp >= this.#state.status.player.maxHp &&
        !resources.some(
          (resource) => resource.restRecoveryAmount > 0 && resource.current < resource.maximum,
        ));
    if (resources.length === 0 && !concentration) {
      const unavailable = document.createElement("li");
      unavailable.className = "resource-empty";
      unavailable.textContent = this.#localization.format("resource-unavailable");
      this.#dom.resourceList.append(unavailable);
    }
    if (presentation.length === 0) {
      const unavailable = document.createElement("li");
      unavailable.className = "ability-empty";
      unavailable.textContent = this.#localization.format("ability-unavailable");
      this.#dom.abilityList.append(unavailable);
    }
    if (concentration) {
      const row = document.createElement("li");
      row.className = "resource-row";
      const heading = document.createElement("div");
      heading.className = "resource-heading";
      const label = document.createElement("span");
      label.className = "resource-name";
      label.textContent = this.#localization.format("sniper-concentration");
      const value = document.createElement("strong");
      value.className = "resource-value";
      value.textContent = `${concentration.current} / ${concentration.maximum}`;
      heading.append(label, value);
      const meter = document.createElement("span");
      meter.className = "resource-meter";
      meter.setAttribute("role", "progressbar");
      meter.setAttribute("aria-label", label.textContent);
      meter.setAttribute("aria-valuemin", "0");
      meter.setAttribute("aria-valuemax", String(concentration.maximum));
      meter.setAttribute("aria-valuenow", String(concentration.current));
      const fill = document.createElement("span");
      fill.style.width = `${concentration.maximum > 0 ? concentration.current / concentration.maximum * 100 : 0}%`;
      meter.append(fill);
      row.append(heading, meter);
      this.#dom.resourceList.append(row);
    }
    for (const resource of resources) {
      const row = document.createElement("li");
      row.className = "resource-row";
      const name = this.#localization.format(resource.nameKey as MessageKey);
      const heading = document.createElement("div");
      heading.className = "resource-heading";
      const label = document.createElement("span");
      label.className = "resource-name";
      label.textContent = name;
      const value = document.createElement("strong");
      value.className = "resource-value";
      value.textContent = `${resource.current} / ${resource.maximum}`;
      heading.append(label, value);
      const meter = document.createElement("span");
      meter.className = "resource-meter";
      meter.setAttribute("role", "progressbar");
      meter.setAttribute("aria-label", name);
      meter.setAttribute("aria-valuemin", "0");
      meter.setAttribute("aria-valuemax", String(resource.maximum));
      meter.setAttribute("aria-valuenow", String(resource.current));
      const fill = document.createElement("span");
      fill.style.width = `${resource.maximum > 0 ? Math.max(0, Math.min(100, resource.current / resource.maximum * 100)) : 0}%`;
      meter.append(fill);
      const recovery = document.createElement("span");
      recovery.className = "resource-recovery";
      recovery.textContent = this.#localization.format("ability-resource-value", {
        resource: name,
        current: resource.current,
        maximum: resource.maximum,
        wait: resource.waitRecoveryAmount,
        rest: resource.restRecoveryAmount,
      });
      row.append(heading, meter, recovery);
      this.#dom.resourceList.append(row);
    }
    if (learning) {
      const row = document.createElement("li");
      row.className = "resource-row";
      row.textContent = this.#localization.format("ability-learning-value", {
        learned: learning.learnedCount,
        capacity: learning.capacity,
        remaining: learning.remainingSlots,
      });
      this.#dom.resourceList.append(row);
    }
    const studyMode = learning?.studyMode ?? "chosen";
    for (const entry of presentation) {
      if (entry.type === "heading") {
        const heading = document.createElement("li");
        heading.className = "ability-book-heading";
        const label = document.createElement("span");
        label.textContent = this.#localization.format(entry.nameKey as MessageKey);
        heading.append(label);
        const bookItemId = entry.bookItemId;
        if (studyMode === "divine-random" && bookItemId) {
          const study = this.#abilityAction("action-ability-study-prayer", () =>
            void this.#dispatch({ type: "study-prayer", bookItemId }),
          );
          study.disabled =
            this.#state.busy ||
            this.#state.playerDead ||
            this.#state.worldMap ||
            !entry.canStudy;
          heading.append(study);
        }
        this.#dom.abilityList.append(heading);
      } else {
        this.#dom.abilityList.append(this.#abilityRow(entry.ability, studyMode));
      }
    }
  }

  #renderNearby(state: GameSnapshot | GameUpdate): void {
    const document = this.#dom.nearbyList.ownerDocument;
    const player = state.player.position;
    const currentCell = this.#state.cellAt(player);
    this.#dom.nearbyCurrent.textContent = currentCell
      ? this.#localization.format("nearby-current-terrain", {
          terrain: this.#contentName(currentCell.terrainId),
        })
      : this.#localization.format("nearby-current-unknown");

    const entries: NearbyEntry[] = [];
    for (const entity of state.entities) {
      if (!this.#isVisible(entity.position)) continue;
      const distance = chebyshevDistance(player, entity.position);
      if (distance === 0) continue;
      entries.push({
        id: `actor:${entity.id}`,
        kind: entity.faction === "hostile" ? "hostile" : "ally",
        contentId: entity.kindId,
        glyph: entity.glyph,
        name: this.#contentName(entity.kindId),
        distance,
        direction: directionKey(player, entity.position),
        hp: entity.hp,
        maxHp: entity.maxHp,
      });
    }
    for (const item of state.items) {
      if (!this.#isVisible(item.position)) continue;
      const distance = chebyshevDistance(player, item.position);
      entries.push({
        id: `item:${item.id}`,
        kind: "item",
        contentId: item.kindId,
        name: this.#contentName(item.kindId),
        distance,
        direction: distance === 0 ? "here" : directionKey(player, item.position),
        quantity: item.quantity,
      });
    }
    for (const pile of state.goldPiles) {
      if (!this.#isVisible(pile.position)) continue;
      const distance = chebyshevDistance(player, pile.position);
      entries.push({
        id: `gold:${pile.id}`,
        kind: "gold",
        contentId: goldVisualId(pile.appearance),
        name: this.#localization.format(`gold-appearance-${pile.appearance}`),
        distance,
        direction: distance === 0 ? "here" : directionKey(player, pile.position),
        amount: pile.amount,
      });
    }
    entries.sort((left, right) =>
      left.distance - right.distance ||
      nearbyKindPriority(left.kind) - nearbyKindPriority(right.kind) ||
      left.name.localeCompare(right.name) ||
      left.id.localeCompare(right.id),
    );
    if (entries.length === 0) {
      const empty = document.createElement("li");
      empty.className = "nearby-empty";
      empty.textContent = this.#localization.format("nearby-empty");
      this.#dom.nearbyList.replaceChildren(empty);
      return;
    }
    this.#dom.nearbyList.replaceChildren(
      ...entries.slice(0, 8).map((entry) => {
        const row = document.createElement("li");
        row.className = `nearby-row nearby-${entry.kind}`;
        const glyph = document.createElement("span");
        glyph.className = "nearby-glyph";
        glyph.setAttribute("aria-hidden", "true");
        glyph.textContent = entry.glyph ?? this.#state.contentGlyphs.get(entry.contentId) ?? "?";
        const details = document.createElement("span");
        details.className = "nearby-details";
        const name = document.createElement("strong");
        name.textContent = entry.name;
        const meta = document.createElement("span");
        const direction = this.#localization.format(`nearby-direction-${entry.direction}`);
        meta.textContent =
          entry.kind === "item"
            ? this.#localization.format("nearby-item-meta", {
                direction,
                distance: entry.distance,
                quantity: entry.quantity ?? 1,
              })
            : entry.kind === "gold"
              ? this.#localization.format("nearby-gold-meta", {
                  direction,
                  distance: entry.distance,
                  amount: entry.amount ?? 0,
                })
            : this.#localization.format("nearby-actor-meta", {
                direction,
                distance: entry.distance,
                hp: entry.hp ?? 0,
                maxHp: entry.maxHp ?? 0,
              });
        details.append(name, meta);
        row.append(glyph, details);
        return row;
      }),
    );
  }

  #isVisible(position: { readonly x: number; readonly y: number }): boolean {
    return this.#state.cellVisibility.get(`${position.x},${position.y}`) === "visible";
  }

  #abilityRow(ability: AbilityDto, studyMode: AbilityStudyModeDto): HTMLLIElement {
    const document = this.#dom.abilityList.ownerDocument;
    const row = document.createElement("li");
    row.className = "ability-row";
    const details = document.createElement("div");
    details.className = "ability-details";
    const name = document.createElement("span");
    name.className = "ability-name";
    name.textContent = this.#localization.format(ability.nameKey as MessageKey);
    const description = document.createElement("span");
    description.className = "ability-description";
    description.textContent = this.#localization.format(ability.descriptionKey as MessageKey);
    const summary = document.createElement("span");
    summary.className = "ability-summary";
    summary.textContent = this.#localization.format(
      ability.governingAttribute ? "ability-summary-governed" : "ability-summary",
      {
        level: ability.minimumLevel,
        attribute: ability.governingAttribute
          ? abilityAttributeAbbreviation(ability.governingAttribute)
          : "",
        baseCost: ability.baseResourceCost,
        cost: ability.resourceCost,
        failure: ability.failurePercent,
      },
    );
    const proficiency = document.createElement("span");
    proficiency.className = "ability-summary";
    proficiency.textContent = this.#localization.format("ability-proficiency-summary", {
      rank: this.#localization.format(
        `ability-proficiency-${ability.proficiencyRank}` as MessageKey,
      ),
      current: ability.proficiency,
      maximum: ability.proficiencyCap,
      casts: ability.castCount,
      fails: ability.failCount,
    });
    const status = document.createElement("span");
    status.className = "ability-status";
    status.textContent = this.#localization.format(abilityStatusMessageKey(ability));
    details.append(name, description, summary, proficiency, status);
    this.#appendAbilityDetails(details, ability);
    const actions = document.createElement("div");
    actions.className = "ability-actions";
    const study = this.#abilityAction("action-ability-study", () => {
      if (!ability.bookItemId) return;
      void this.#dispatch({
        type: "study-ability",
        bookItemId: ability.bookItemId,
        abilityId: ability.id,
      });
    });
    study.disabled =
      this.#state.busy ||
      this.#state.playerDead ||
      this.#state.worldMap ||
      !ability.canStudy ||
      !ability.bookItemId;
    const forget = this.#abilityAction("action-ability-forget", () =>
      void this.#dispatch({ type: "forget-ability", abilityId: ability.id }),
    );
    forget.disabled =
      this.#state.busy || this.#state.playerDead || this.#state.worldMap || !ability.canForget;
    let townTarget: HTMLSelectElement | undefined;
    if (ability.targetSpec.modes.includes("town")) {
      townTarget = document.createElement("select");
      townTarget.className = "ability-town-target";
      townTarget.setAttribute(
        "aria-label",
        this.#localization.format("action-ability-town-target"),
      );
      for (const target of ability.townTargets ?? []) {
        const option = document.createElement("option");
        option.value = target.townId;
        option.textContent = this.#localization.format(target.townNameKey as MessageKey);
        townTarget.append(option);
      }
      actions.append(townTarget);
    }
    const cast = this.#abilityAction("action-ability-cast", () =>
      this.#castAbility(ability, townTarget?.value),
    );
    cast.classList.add("ability-cast-action");
    cast.disabled =
      this.#state.busy ||
      this.#state.playerDead ||
      this.#state.worldMap ||
      !ability.canCast ||
      (ability.targetSpec.modes.includes("town") && (ability.townTargets?.length ?? 0) === 0);
    if (studyMode === "chosen") actions.append(study);
    actions.append(forget, cast);
    row.append(details, actions);
    return row;
  }

  #appendAbilityDetails(details: HTMLDivElement, ability: AbilityDto): void {
    const append = (key: MessageKey, args?: Record<string, string | number>): void => {
      const element = details.ownerDocument.createElement("span");
      element.className = "ability-status";
      element.textContent = this.#localization.format(key, args);
      details.append(element);
    };
    if (ability.minimumConcentration > 0) {
      append("ability-concentration-summary", { concentration: ability.minimumConcentration });
    }
    if (ability.hitPointCost > 0) {
      append("ability-hit-point-cost-summary", { cost: ability.hitPointCost });
    }
    if (ability.areaRadius != null) append("ability-area-summary", { radius: ability.areaRadius });
    if (ability.beamDamage) append("ability-beam-summary");
    if (ability.coneRadius != null) append("ability-cone-summary", { radius: ability.coneRadius });
    if (ability.teleport) append("ability-teleport-summary");
    if (ability.summon != null) {
      append("ability-summon-summary", {
        count: ability.summon.count,
        radius: ability.summon.radius,
        turns: ability.summon.durationTurns,
      });
    }
    if (ability.detect != null) {
      append("ability-detect-summary", {
        category: ability.detect.category,
        radius: ability.detect.radius,
        persistence: this.#localization.format(
          ability.detect.persistent
            ? "ability-detect-persistent"
            : "ability-detect-transient",
        ),
      });
    }
    if (ability.terrainTransform != null) {
      append("ability-terrain-transform-summary", {
        sources: ability.terrainTransform.sourceTerrainIds.length,
        terrain: this.#contentName(ability.terrainTransform.targetTerrainId),
        radius: ability.terrainTransform.radius,
      });
    }
    if (ability.effects.length > 1) {
      append("ability-effects-summary", { count: ability.effects.length });
    }
    if (ability.cooldownTurns > 0) {
      append("ability-cooldown-summary", {
        remaining: ability.cooldownRemaining,
        turns: ability.cooldownTurns,
      });
    }
  }

  #abilityAction(key: MessageKey, action: () => void): HTMLButtonElement {
    const button = this.#dom.abilityList.ownerDocument.createElement("button");
    button.type = "button";
    button.textContent = this.#localization.format(key);
    button.addEventListener("click", action);
    return button;
  }

  #castAbility(ability: AbilityDto, townId?: string): void {
    const confirmationKey = abilityConfirmationMessageKey(ability.id);
    const view = this.#dom.abilityList.ownerDocument.defaultView;
    if (confirmationKey && view && !view.confirm(this.#localization.format(confirmationKey))) {
      return;
    }
    if (ability.targetSpec.modes.includes("town")) {
      if (!townId) return;
      void this.#dispatch({
        type: "cast-ability",
        abilityId: ability.id,
        target: { type: "town", townId },
      });
      return;
    }
    if (ability.targetSpec.modes.includes("self")) {
      void this.#dispatch({
        type: "cast-ability",
        abilityId: ability.id,
        target: { type: "self" },
      });
      return;
    }
    if (ability.targetSpec.modes.includes("item")) {
      this.#selectItemTarget(undefined, (itemId) =>
        this.#dispatch({
          type: "cast-ability",
          abilityId: ability.id,
          target: { type: "item", itemId },
        }),
      );
      return;
    }
    this.#startAbilityTargeting(ability);
  }

  #renderCombatStat(element: HTMLElement, value: number, equipmentModifier: number): void {
    element.textContent = this.#localization.format(
      equipmentModifier === 0 ? "status-stat-value" : "status-stat-value-bonus",
      { value, bonus: signedModifier(equipmentModifier) },
    );
  }
}

export function mutationRatingMessageKey(rating: PlayerMutationDto["rating"]): MessageKey {
  return `mutation-rating-${rating}`;
}

export function abilityStatusMessageKey(
  ability: Pick<AbilityDto, "source" | "learned">,
): MessageKey {
  if (ability.source === "mutation") return "ability-status-mutation";
  if (ability.source === "class") return "ability-status-class";
  if (ability.source === "race") return "ability-status-innate";
  return ability.learned ? "ability-status-learned" : "ability-status-unlearned";
}

export function abilityAttributeAbbreviation(attribute: AttributeKindDto): string {
  return {
    strength: "STR",
    intelligence: "INT",
    wisdom: "WIS",
    dexterity: "DEX",
    constitution: "CON",
    charisma: "CHR",
  }[attribute];
}

export type AbilityPresentationEntry =
  | {
      type: "heading";
      nameKey: string;
      bookItemId?: string;
      canStudy: boolean;
    }
  | { type: "ability"; ability: AbilityDto };

export function abilityPresentation(
  abilities: readonly AbilityDto[],
  playerLevel: number,
): AbilityPresentationEntry[] {
  const ordered = [...abilities]
    .filter((ability) => !ability.uiGroupNameKey || ability.minimumLevel <= playerLevel)
    .sort(
      (left, right) =>
        (left.uiGroupNameKey ?? "").localeCompare(right.uiGroupNameKey ?? "") ||
        (left.bookRank ?? Number.MAX_SAFE_INTEGER) -
          (right.bookRank ?? Number.MAX_SAFE_INTEGER) ||
        (left.bookNameKey ?? "").localeCompare(right.bookNameKey ?? "") ||
        left.minimumLevel - right.minimumLevel ||
        left.id.localeCompare(right.id),
    );
  const entries: AbilityPresentationEntry[] = [];
  const studyByHeading = new Map<string, { bookItemId?: string; canStudy: boolean }>();
  for (const ability of ordered) {
    if (!ability.bookNameKey) continue;
    const current = studyByHeading.get(ability.bookNameKey);
    studyByHeading.set(ability.bookNameKey, {
      bookItemId: current?.bookItemId ?? ability.bookItemId ?? undefined,
      canStudy: (current?.canStudy ?? false) || ability.canStudy === true,
    });
  }
  let currentHeading: string | undefined;
  for (const ability of ordered) {
    const heading = ability.uiGroupNameKey ?? ability.bookNameKey ?? undefined;
    if (heading && heading !== currentHeading) {
      entries.push({
        type: "heading",
        nameKey: heading,
        bookItemId: studyByHeading.get(heading)?.bookItemId,
        canStudy: studyByHeading.get(heading)?.canStudy ?? false,
      });
    }
    currentHeading = heading;
    entries.push({ type: "ability", ability });
  }
  return entries;
}

export function formatAttributeValue(value: number): string {
  return value > 18 ? `18/${value - 18}` : String(value);
}

export function nutritionPercentage(nutrition: number): number {
  return Math.floor(nutrition / 100);
}

type NearbyKind = "hostile" | "ally" | "gold" | "item";
type NearbyDirection = "n" | "ne" | "e" | "se" | "s" | "sw" | "w" | "nw" | "here";

interface NearbyEntry {
  readonly id: string;
  readonly kind: NearbyKind;
  readonly contentId: string;
  readonly glyph?: string;
  readonly name: string;
  readonly distance: number;
  readonly direction: NearbyDirection;
  readonly hp?: number;
  readonly maxHp?: number;
  readonly quantity?: number;
  readonly amount?: number;
}

function chebyshevDistance(
  left: { readonly x: number; readonly y: number },
  right: { readonly x: number; readonly y: number },
): number {
  return Math.max(Math.abs(left.x - right.x), Math.abs(left.y - right.y));
}

function directionKey(
  from: { readonly x: number; readonly y: number },
  to: { readonly x: number; readonly y: number },
): NearbyDirection {
  const x = Math.sign(to.x - from.x);
  const y = Math.sign(to.y - from.y);
  if (x === 0 && y === 0) return "here";
  if (x === 0) return y < 0 ? "n" : "s";
  if (y === 0) return x < 0 ? "w" : "e";
  if (x > 0) return y < 0 ? "ne" : "se";
  return y < 0 ? "nw" : "sw";
}

function nearbyKindPriority(kind: NearbyKind): number {
  return kind === "hostile" ? 0 : kind === "ally" ? 1 : kind === "gold" ? 2 : 3;
}

function signedModifier(value: number): string {
  return value >= 0 ? `+${value}` : String(value);
}
