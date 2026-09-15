// SPDX-License-Identifier: MPL-2.0
import type { AppState } from "./app-state";
import type { Localization } from "./localization";
import type { PreferencesClient } from "./preferences";
import type { EquipmentItemDto, InventoryItemDto } from "./protocol";

export const HOTBAR_ACTIONS = ["food", "potion", "scroll", "wand", "staff", "rod", "activate", "equip", "throw"] as const;
export type HotbarItemAction = typeof HOTBAR_ACTIONS[number];
export type HotbarBinding = { type: "ability"; id: string } | {
  type: "item"; kindId: string; artifactName: string | null; inscription: string | null; action: HotbarItemAction;
};
export const emptyHotbar = (): (HotbarBinding | null)[] => Array.from({ length: 60 }, () => null);
export function validHotbar(value: unknown): value is (HotbarBinding | null)[] {
  return Array.isArray(value) && value.length === 60 && value.every(binding => {
    if (binding === null) return true;
    if (!binding || typeof binding !== "object") return false;
    const id = binding.type === "ability" ? binding.id : binding.kindId;
    if (typeof id !== "string" || !id.length || id.length > 256) return false;
    if (binding.type === "ability") return Object.keys(binding).sort().join() === "id,type";
    return binding.type === "item" && Object.keys(binding).sort().join() === "action,artifactName,inscription,kindId,type" &&
      HOTBAR_ACTIONS.includes(binding.action) && [binding.artifactName, binding.inscription].every(text => text === null || (typeof text === "string" && text.length <= 1024));
  });
}
export function itemHotbarBinding(item: InventoryItemDto | EquipmentItemDto): HotbarBinding | undefined {
  const action = item.activation ? "activate" : "useCategory" in item && item.useCategory ? item.useCategory :
    "equipmentSlot" in item && item.equipmentSlot ? "equip" : item.throwTargetSpec ? "throw" : undefined;
  if (!action || !HOTBAR_ACTIONS.includes(action as HotbarItemAction)) return;
  return { type: "item", kindId: item.kindId, artifactName: item.artifactName ?? null, inscription: item.inscription ?? null, action: action as HotbarItemAction };
}
export function hotbarItems(binding: Extract<HotbarBinding, { type: "item" }>, inventory: readonly InventoryItemDto[], equipment: readonly EquipmentItemDto[]) {
  const items = binding.action === "activate" || binding.action === "throw" ? [...inventory, ...equipment] : inventory;
  return items.filter(item => item.kindId === binding.kindId && (item.artifactName ?? null) === binding.artifactName &&
    (item.inscription ?? null) === binding.inscription);
}

interface HotbarOptions {
    document: Document; window: Window; state: AppState; localization: Localization; preferences: PreferencesClient;
    itemName: (key: string, kindId: string, artifactName?: string | null) => string;
    cast: (id: string) => void; use: (action: HotbarItemAction, ids: string[]) => void;
    blocked: () => boolean; commandNumberInputActive: () => boolean; error: (error: unknown) => void;
}
export class Hotbar {
  readonly #o: HotbarOptions;
  readonly #slots: HTMLButtonElement[];
  #saving = false;
  #page = 0;
  constructor(options: HotbarOptions) {
    this.#o = options;
    this.#slots = [...options.document.querySelectorAll<HTMLButtonElement>("[data-shortcut-slot]")];
  }
  install(): void {
    this.#o.window.addEventListener("keydown", this.#keydown, true);
    for (const slot of this.#slots) {
      slot.addEventListener("click", this.#click);
      slot.addEventListener("contextmenu", this.#clear);
    }
    for (const button of this.#o.document.querySelectorAll<HTMLButtonElement>("[data-hotbar-page]")) button.addEventListener("click", this.#switchPage);
    this.render();
  }
  dispose(): void {
    for (const button of this.#o.document.querySelectorAll<HTMLButtonElement>("[data-hotbar-page]")) button.removeEventListener("click", this.#switchPage);
    this.#o.window.removeEventListener("keydown", this.#keydown, true);
    for (const slot of this.#slots) {
      slot.removeEventListener("click", this.#click);
      slot.removeEventListener("contextmenu", this.#clear);
    }
  }
  #label(binding: HotbarBinding | null | undefined): string {
    const { state, localization: l } = this.#o;
    if (!binding) return l.format("hotbar-empty");
    if (binding.type === "ability") {
      const ability = state.status?.player.abilities?.find(a => a.id === binding.id);
      return ability ? l.format(ability.nameKey) : l.format("hotbar-ability-missing");
    }
    const item = hotbarItems(binding, state.inventory, state.equipment)[0];
    return item ? this.#o.itemName(item.displayNameKey, item.kindId, item.artifactName) : l.format("hotbar-item-missing");
  }
  render(): void {
    const bindings = this.#o.preferences.snapshot?.preferences.hotbar ?? emptyHotbar();
    this.#slots.forEach((slot, index) => {
      const name = this.#label(bindings[this.#page * 10 + index]);
      const key = String((index + 1) % 10);
      slot.textContent = `${key} · ${name}`;
      slot.title = this.#o.localization.format("hotbar-slot-help", { key, name });
      slot.disabled = this.#saving || !this.#o.preferences.snapshot;
    });
    for (const button of this.#o.document.querySelectorAll<HTMLButtonElement>("[data-hotbar-page]")) {
      button.setAttribute("aria-pressed", String(Number(button.dataset.hotbarPage) === this.#page));
      button.title = this.#o.localization.format("hotbar-bank-help", { page: Number(button.dataset.hotbarPage) + 1 });
      button.setAttribute("aria-label", button.title);
    }
  }
  choose(binding: HotbarBinding): void {
    if (this.#saving || !this.#o.preferences.snapshot) return;
    const { document: d, localization: l } = this.#o;
    const dialog = d.createElement("dialog"); dialog.className = "hotbar-bind-dialog";
    const heading = d.createElement("h2"); heading.textContent = l.format("hotbar-bind");
    const help = d.createElement("p"); help.textContent = l.format("hotbar-bind-help");
    dialog.setAttribute("aria-label", heading.textContent);
    const choices = d.createElement("div"); choices.className = "hotbar-bind-choices";
    const feedback = d.createElement("p"); feedback.setAttribute("role", "alert");
    this.#o.preferences.snapshot.preferences.hotbar.forEach((existing, index) => {
      const button = d.createElement("button"); button.type = "button";
      button.textContent = l.format("hotbar-binding-slot", { page: Math.floor(index / 10) + 1, key: String((index + 1) % 10), name: this.#label(existing) });
      button.addEventListener("click", async () => {
        for (const control of dialog.querySelectorAll<HTMLButtonElement>("button")) control.disabled = true;
        try { await this.#save(index, binding); dialog.close(); }
        catch (error) { feedback.textContent = l.format("preferences-error", { error: String(error) }); }
        finally { for (const control of dialog.querySelectorAll<HTMLButtonElement>("button")) control.disabled = false; }
      });
      choices.append(button);
    });
    const cancel = d.createElement("button"); cancel.type = "button"; cancel.textContent = l.format("action-dialog-close");
    cancel.addEventListener("click", () => dialog.close());
    dialog.addEventListener("cancel", event => { if (this.#saving) event.preventDefault(); });
    dialog.addEventListener("close", () => dialog.remove(), { once: true });
    dialog.append(heading, help, choices, feedback, cancel); d.body.append(dialog); dialog.showModal();
  }
  async #save(index: number, binding: HotbarBinding | null): Promise<void> {
    const saved = this.#o.preferences.snapshot;
    if (!saved || this.#saving) throw new Error("preferences-unavailable");
    this.#saving = true; this.render();
    try {
      const next = structuredClone(saved.preferences); next.hotbar[index] = binding;
      await this.#o.preferences.save(next, saved.revision);
    } finally { this.#saving = false; this.render(); }
  }
  #activate(index: number): void {
    if (this.#saving || this.#o.blocked()) return;
    const binding = this.#o.preferences.snapshot?.preferences.hotbar[index];
    if (!binding) { this.#o.window.alert(this.#o.localization.format("hotbar-empty-help")); return; }
    if (binding.type === "ability") { this.#o.cast(binding.id); return; }
    const ids = hotbarItems(binding, this.#o.state.inventory, this.#o.state.equipment).map(item => item.id);
    if (!ids.length) { this.#o.window.alert(this.#o.localization.format("hotbar-item-missing")); return; }
    this.#o.use(binding.action, ids);
  }
  readonly #switchPage = (event: MouseEvent): void => { this.#page = Number((event.currentTarget as HTMLElement).dataset.hotbarPage); this.render(); };
  readonly #click = (event: MouseEvent): void => this.#activate(this.#page * 10 + Number((event.currentTarget as HTMLElement).dataset.shortcutSlot) - 1);
  readonly #clear = (event: MouseEvent): void => {
    event.preventDefault();
    if (this.#saving) return;
    void this.#save(this.#page * 10 + Number((event.currentTarget as HTMLElement).dataset.shortcutSlot) - 1, null).catch(this.#o.error);
  };
  readonly #keydown = (event: KeyboardEvent): void => {
    if (event.ctrlKey || event.metaKey || event.shiftKey || !/^Digit[0-9]$/.test(event.code) || event.isComposing || this.#o.commandNumberInputActive() ||
        this.#o.state.mode !== "playing" ||
        this.#o.document.querySelector("dialog[open]") ||
        (event.target instanceof HTMLElement && (event.target.isContentEditable || event.target.matches("input, textarea, select")))) return;
    if (event.altKey && !/^Digit[1-6]$/.test(event.code)) return;
    event.preventDefault(); event.stopImmediatePropagation();
    if (event.repeat) return;
    if (event.altKey) { this.#page = Number(event.code.slice(-1)) - 1; this.render(); }
    else this.#activate(this.#page * 10 + (Number(event.code.slice(-1)) + 9) % 10);
  };
}
