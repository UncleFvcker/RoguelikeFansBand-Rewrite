// SPDX-License-Identifier: MPL-2.0
import type { AppState, TargetingIntent } from "./app-state";
import type { Localization } from "./localization";
import type { AbsorbedDeviceSlotDto, GameCommand, InventoryItemDto, TargetSpecDto } from "./protocol";
import { itemTargetCandidates } from "./inventory-panel.ts";

interface MagicEaterPanelOptions {
  document: Document;
  state: AppState;
  localization: Localization;
  dispatch: (command: GameCommand) => Promise<void>;
  visibleItemName: (key: string, id: string, artifact?: string | null) => string;
  inspectItem: (id: string) => void;
  selectItemTarget: (ids: string[], onSelect: (id: string) => Promise<void>) => void;
  startTargeting: (spec: TargetSpecDto, intent: TargetingIntent) => void;
  beforeOpen: () => void;
  exportSave: () => Promise<void>;
  importSave: () => void;
}

export class MagicEaterPanel {
  readonly #options: MagicEaterPanelOptions;
  readonly #dialog: HTMLDialogElement;
  readonly #category: HTMLSelectElement;
  #ordinary = false;
  #deviceCommand = false;
  #swapSlot: number | undefined;
  #exchanging = false;
  #inscribing = false;
  #promptIdentity = "";
  #auxiliary: HTMLDialogElement | undefined;

  constructor(options: MagicEaterPanelOptions) {
    this.#options = options;
    this.#dialog = this.#element("dialog");
    this.#category = this.#element("category");
    this.#element("open").addEventListener("click", () => this.open());
    this.#category.addEventListener("change", () => { this.#swapSlot = undefined; this.render(); });
    this.#element("body").addEventListener("click", () => { this.#ordinary = false; this.#deviceCommand = false; this.render(); });
    this.#element("ordinary").addEventListener("click", () => this.openDeviceCommand(this.#category.value === "wand" ? "a" : this.#category.value === "staff" ? "u" : "z"));
    this.#element("absorb").addEventListener("click", () => this.#absorb());
    this.#element("close").addEventListener("click", () => this.#cancel());
    this.#dialog.addEventListener("cancel", event => { event.preventDefault(); this.#cancel(); });
    this.#dialog.addEventListener("keydown", event => this.#key(event));
    this.#element("confirm").addEventListener("click", () => {
      if (!options.state.busy && this.#pending?.replacement) void options.dispatch({ type: "resolve-magic-absorption", confirm: true, inheritInscription: this.#element<HTMLInputElement>("inherit").checked });
    });
    this.#element("save").addEventListener("click", () => { if (!options.state.busy) void options.exportSave(); });
    this.#element("load").addEventListener("click", () => { if (!options.state.busy) options.importSave(); });
  }

  #element<T extends HTMLElement>(suffix: string): T { return this.#options.document.getElementById(`magic-eater-${suffix}`) as T; }
  get #projection() { return this.#options.state.status?.player.magicEater; }
  get #pending() { return this.#projection?.pendingAbsorption; }
  #format(key: string, args?: Record<string, string | number>) { return this.#options.localization.format(key, args); }
  #name(item: InventoryItemDto): string {
    const name = this.#options.visibleItemName(item.displayNameKey, item.kindId, item.artifactName);
    return item.activation ? `${name} · ${this.#format(item.activation.nameKey)}` : name;
  }

  open(): void {
    if (!this.#projection || this.#options.state.busy || this.#options.state.commandBlocked) return;
    this.#ordinary = false; this.#deviceCommand = false; this.#swapSlot = undefined; this.#exchanging = false; this.#inscribing = false;
    this.render(); this.#show();
  }

  openDeviceCommand(key: string): boolean {
    if (!this.#projection || !["m", "a", "u", "z"].includes(key)) return false;
    if (key === "m") { this.open(); return true; }
    if (this.#options.state.busy || this.#options.state.commandBlocked || this.#options.state.worldMap) return true;
    const category = key === "a" ? "wand" : key === "u" ? "staff" : "rod";
    this.#category.value = category;
    this.#ordinary = Boolean(this.#projection.deviceCommands.find(command => command.category === category)?.items.length);
    this.#deviceCommand = true; this.#swapSlot = undefined; this.#exchanging = false; this.#inscribing = false;
    this.render(); this.#show();
    return true;
  }

  #show(): void {
    if (!this.#dialog.open) { this.#options.beforeOpen(); this.#dialog.showModal(); this.#element("slots").focus(); }
  }

  reset(): void {
    const auxiliary = this.#auxiliary; this.#auxiliary = undefined; auxiliary?.close();
    if (this.#dialog.open) this.#dialog.close();
    this.#promptIdentity = ""; this.#swapSlot = undefined; this.#exchanging = false; this.#inscribing = false;
  }

  render(): void {
    const { state, document } = this.#options;
    const projection = this.#projection;
    const open = this.#element<HTMLButtonElement>("open");
    open.hidden = !projection;
    open.disabled = state.busy || state.commandBlocked || state.worldMap;
    if (!projection) { this.reset(); return; }
    const pending = this.#pending;
    let newPrompt = false;
    if (pending) {
      this.#category.value = pending.category;
      this.#ordinary = false; this.#swapSlot = undefined; this.#exchanging = false; this.#inscribing = false;
      const identity = `${pending.sourceItemId}/${pending.replacement?.itemId ?? ""}`;
      if (identity !== this.#promptIdentity) { this.#element<HTMLInputElement>("inherit").checked = false; newPrompt = true; }
      this.#promptIdentity = identity;
      this.#show();
    } else if (this.#promptIdentity) {
      this.#promptIdentity = "";
      this.#element("body").focus();
    }
    const locked = state.busy || state.commandBlocked || state.worldMap;
    for (const control of this.#auxiliary?.querySelectorAll<HTMLInputElement | HTMLButtonElement | HTMLSelectElement>("input, button, select") ?? []) control.disabled = state.busy;
    this.#category.disabled = state.busy || Boolean(pending);
    for (const id of ["body", "ordinary", "absorb"]) this.#element<HTMLButtonElement>(id).disabled = locked;
    const ability = state.status?.player.abilities.find(ability => ability.effects.some(effect => effect.type === "magic-eater-absorb"));
    this.#element<HTMLButtonElement>("absorb").disabled = locked || !ability?.canCast || !ability.itemTargets?.length;
    for (const id of ["save", "load", "close", "confirm"]) this.#element<HTMLButtonElement>(id).disabled = state.busy;
    this.#element<HTMLInputElement>("inherit").disabled = state.busy;
    this.#element("replacement").hidden = !pending?.replacement;
    const replacement = projection.slots.find(slot => slot.item?.id === pending?.replacement?.itemId)?.item;
    this.#element("replacement-description").textContent = replacement ? this.#format("magic-eater-replace-warning", { item: this.#name(replacement), inscription: replacement.inscription ?? "" }) : "";
    this.#element("prompt").textContent = this.#format(pending ? "magic-eater-select-slot" : this.#exchanging ? this.#swapSlot !== undefined ? "magic-eater-swap-target" : "magic-eater-swap-first" : this.#inscribing ? "magic-eater-inscribe-select" : this.#ordinary ? "magic-eater-ordinary-help" : "magic-eater-body-help");
    const list = this.#element("slots");
    const scrollTop = this.#dialog.scrollTop;
    const focused = list.contains(document.activeElement) ? (document.activeElement as HTMLElement).dataset.focus : undefined;
    const slots = projection.slots.filter(slot => slot.category === this.#category.value);
    const items = this.#ordinary ? projection.deviceCommands.find(command => command.category === this.#category.value)?.items ?? [] : [];
    list.replaceChildren(...(this.#ordinary ? items.map((item, index) => this.#row(item, undefined, String(index + 1)))
      : slots.map(slot => this.#row(slot.item, slot, pending || this.#exchanging || this.#inscribing ? String.fromCharCode(97 + slot.slot) : this.#deviceCommand ? slot.deviceLabel : slot.useLabel))));
    this.#dialog.scrollTop = scrollTop;
    if (focused) {
      const target = [...list.querySelectorAll<HTMLElement>("[data-focus]")].find(node => node.dataset.focus === focused);
      if (target?.matches(":disabled")) target.closest<HTMLElement>("li")?.focus();
      else target?.focus();
    }
    if (newPrompt) this.#element(pending?.replacement ? "close" : "slots").focus();
  }

  #row(item: InventoryItemDto | null, slot: AbsorbedDeviceSlotDto | undefined, label: string): HTMLElement {
    const { document, state } = this.#options;
    const row = document.createElement("li");
    row.tabIndex = -1;
    row.dataset.focus = `${slot?.slot ?? item?.id}-row`;
    row.dataset.slot = slot ? String(slot.slot) : "";
    row.dataset.itemId = item?.id ?? "";
    const name = document.createElement("strong");
    name.textContent = `${label}) ${item ? this.#name(item) : this.#format("magic-eater-empty")}`;
    row.append(name);
    if (item) {
      const info = document.createElement("p");
      info.textContent = this.#format("magic-eater-sp", { current: item.charges?.current ?? "?", maximum: item.charges?.maximum ?? "?", cost: item.activation?.cost ?? "?" });
      if (slot) info.textContent += " · " + this.#format("magic-eater-rates", { failure: slot.failurePerMille == null ? "?" : slot.failurePerMille / 10, energy: slot.energyCost ?? "?", recovery: slot.recoveryPerMille ?? "?" });
      if (item.inscription) info.textContent += ` · ${item.inscription}`;
      if (item.useUnavailableReason) info.textContent += " · " + this.#format(`item-use-unavailable-${item.useUnavailableReason}`);
      row.append(info);
    }
    const actions = document.createElement("div"); actions.className = "magic-eater-actions";
    const button = (action: string, text: string, disabled: boolean, click: () => void) => {
      const node = document.createElement("button"); node.type = "button"; node.textContent = this.#format(text);
      node.dataset.action = action; node.dataset.focus = `${slot?.slot ?? item?.id}-${action}`;
      node.disabled = disabled; node.addEventListener("click", click); actions.append(node); return node;
    };
    const locked = state.busy || state.commandBlocked || state.worldMap;
    if (this.#pending && slot) {
      button("select", "magic-eater-select", state.busy || Boolean(this.#pending.replacement), () => void this.#options.dispatch({ type: "select-magic-absorption-slot", slot: slot.slot })).dataset.label = label;
    } else if (this.#exchanging && slot) {
      button("swap-target", "magic-eater-select", locked || this.#swapSlot === slot.slot, () => {
        if (this.#swapSlot === undefined) { this.#swapSlot = slot.slot; this.render(); return; }
        const firstSlot = this.#swapSlot; this.#swapSlot = undefined; this.#exchanging = false;
        void this.#options.dispatch({ type: "swap-absorbed-devices", category: slot.category, firstSlot, secondSlot: slot.slot });
      }).dataset.label = label;
    } else if (item) {
      button("use", "magic-eater-use", locked || !item.usable, () => this.#use(item, slot)).dataset.label = this.#inscribing ? "" : label;
    }
    if (item) {
      button("inspect", "action-inventory-details", state.busy, () => this.#options.inspectItem(item.id));
      if (slot && !this.#pending) {
        button("inscribe", "action-inventory-inscribe", locked, () => this.#inscribe(item)).dataset.label = this.#inscribing ? label : "";
        button("swap", "magic-eater-swap", locked, () => { this.#swapSlot = slot.slot; this.#exchanging = true; this.#inscribing = false; this.render(); });
      }
    }
    row.append(actions); return row;
  }

  #absorb(): void {
    const { state } = this.#options;
    const ability = state.status?.player.abilities.find(ability => ability.effects.some(effect => effect.type === "magic-eater-absorb"));
    if (state.busy || state.commandBlocked || !ability?.canCast) return;
    this.#options.selectItemTarget(ability.itemTargets?.map(target => target.itemId) ?? [], id => this.#options.dispatch({ type: "cast-ability", abilityId: ability.id, target: { type: "item", itemId: id } }));
  }

  #cancel(): void {
    if (this.#options.state.busy) return;
    if (this.#pending) { void this.#options.dispatch({ type: "resolve-magic-absorption", confirm: false, inheritInscription: false }); return; }
    if (this.#exchanging || this.#inscribing) { this.#swapSlot = undefined; this.#exchanging = false; this.#inscribing = false; this.render(); return; }
    this.#dialog.close(); this.#element("open").focus();
  }

  #key(event: KeyboardEvent): void {
    const target = event.target as HTMLElement;
    if (this.#options.state.busy || ["INPUT", "SELECT", "TEXTAREA"].includes(target.tagName) || event.ctrlKey || event.altKey || event.metaKey || event.key === "Escape") return;
    const choice = [...this.#element("slots").querySelectorAll<HTMLButtonElement>("[data-label]")].find(button => button.dataset.label === event.key);
    if (choice) { event.preventDefault(); choice.click(); return; }
    if (this.#pending) return;
    const category = ({ W: "wand", S: "staff", R: "rod" } as const)[event.key as "W" | "S" | "R"];
    if (category) { event.preventDefault(); this.#category.value = category; this.#ordinary = false; this.#swapSlot = undefined; this.render(); }
    if (event.key === "Z" && !this.#ordinary) { event.preventDefault(); this.#inscribing = true; this.#exchanging = false; this.#swapSlot = undefined; this.render(); }
    if (event.key === "X" && !this.#ordinary) { event.preventDefault(); this.#exchanging = true; this.#inscribing = false; this.#swapSlot = undefined; this.render(); }
  }

  #inscribe(item: InventoryItemDto): void {
    const { document } = this.#options;
    const dialog = document.createElement("dialog"); dialog.className = "item-target-dialog";
    const form = document.createElement("form");
    const label = document.createElement("label"); label.textContent = this.#format("action-inventory-inscribe");
    const input = document.createElement("input"); input.value = item.inscription ?? "";
    label.append(input); form.append(label); this.#finishForm(dialog, form, () => {
      this.#inscribing = false;
      void this.#options.dispatch({ type: "inscribe-item", itemId: item.id, inscription: input.value || null });
    });
    input.focus();
  }

  #finishForm(dialog: HTMLDialogElement, form: HTMLFormElement, submit: () => void, cancel?: () => void): void {
    const { document, state } = this.#options;
    dialog.classList.add("magic-eater-editor");
    const close = document.createElement("button"); close.type = "button"; close.textContent = this.#format("action-dialog-cancel"); close.addEventListener("click", () => dialog.close());
    const accept = document.createElement("button"); accept.type = "submit"; accept.textContent = this.#format("action-item-target-confirm");
    form.append(close, accept); let accepted = false;
    form.addEventListener("submit", event => { event.preventDefault(); if (state.busy) return; accepted = true; dialog.close(); submit(); });
    dialog.addEventListener("cancel", event => { if (state.busy) event.preventDefault(); });
    dialog.addEventListener("close", () => { dialog.remove(); const active = this.#auxiliary === dialog; if (active) this.#auxiliary = undefined; if (!accepted && active) cancel?.(); }, { once: true });
    dialog.append(form); document.body.append(dialog); this.#auxiliary = dialog; dialog.showModal();
  }

  #use(item: InventoryItemDto, slot?: AbsorbedDeviceSlotDto): void {
    if (this.#options.state.busy || this.#options.state.commandBlocked || !item.usable) return;
    const spec = item.useTargetSpec;
    const send = (ids: string[]) => this.#options.dispatch(slot
      ? { type: "use-absorbed-device", itemId: item.id, targets: ids.map(itemId => ({ type: "item", itemId })) }
      : { type: "use-item", itemId: item.id, ...(ids[0] ? { target: { type: "item", itemId: ids[0] } } : {}) });
    if (spec?.modes.includes("item")) {
      const { document, state, visibleItemName } = this.#options;
      const dialog = document.createElement("dialog"); dialog.className = "item-target-dialog";
      const form = document.createElement("form"); const label = document.createElement("label");
      label.textContent = this.#format(slot?.allowsMultipleTargets ? "magic-eater-multiple-targets" : "item-target-label");
      const select = document.createElement("select"); select.multiple = Boolean(slot?.allowsMultipleTargets);
      for (const candidate of itemTargetCandidates(state, item.id, visibleItemName)) {
        const option = document.createElement("option"); option.value = candidate.id; option.textContent = candidate.label; select.append(option);
      }
      let ordered: string[] = [];
      select.addEventListener("change", () => { const ids = [...select.selectedOptions].map(option => option.value); ordered = [...ordered.filter(id => ids.includes(id)), ...ids.filter(id => !ordered.includes(id))]; });
      label.append(select); form.append(label);
      this.#finishForm(dialog, form, () => void send(select.multiple ? ordered : [select.value].filter(Boolean)), () => void send([]));
      select.focus(); return;
    }
    if (spec && !spec.modes.includes("self")) {
      this.#dialog.close();
      this.#options.startTargeting(spec, { type: slot ? "absorbed-device" : "item", itemId: item.id }); return;
    }
    void this.#options.dispatch(slot ? { type: "use-absorbed-device", itemId: item.id, targets: [{ type: "self" }] }
      : { type: "use-item", itemId: item.id, target: { type: "self" } });
  }
}
