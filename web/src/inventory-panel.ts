// SPDX-License-Identifier: MPL-2.0

import type { AppDom } from "./app-dom";
import type { AppState, TargetingIntent } from "./app-state";
import type { Localization, MessageKey } from "./localization";
import { originalCommandKey, type CommandShortcut, type ItemShortcut } from "./command-shortcuts.ts";
import { itemSelectionLabels, itemSelectionConfirmations } from "./item-selection-labels.ts";
import type {
  BodySlotDto,
  EquipmentBonusesDto,
  EquipmentItemDto,
  EquipmentPassiveDto,
  GameCommand,
  InventoryItemDto,
  ItemDto,
  ItemDestructionElementDto,
  ItemCurseSeverityDto,
  ItemEnchantmentsDto,
  ItemPropertyDto,
  ResistanceDto,
  ResistanceLevelDto,
  SlayDto,
  SlayTargetDto,
  StatModifiersDto,
  TargetSpecDto,
  WeaponBrandDto,
} from "./protocol";

type InventoryDom = Pick<
  AppDom,
  | "inventoryCount"
  | "inventoryFilters"
  | "inventorySearch"
  | "inventoryFilterReset"
  | "inventoryDetailDialog"
  | "inventoryDetailTitle"
  | "inventoryDetailBody"
  | "inventoryDetailClose"
  | "inventoryDetailActions"
  | "inventoryMore"
  | "inventoryMoreDialog"
  | "inventoryActionDialog"
  | "inventoryActionForm"
  | "inventoryActionTitle"
  | "inventoryActionItem"
  | "inventoryQuantityField"
  | "inventoryInscriptionField"
  | "inventoryActionCancel"
  | "inventoryActionConfirm"
  | "inventorySelectionCount"
  | "inventoryUse"
  | "inventoryAbsorb"
  | "inventoryRead"
  | "inventoryUseOnMount"
  | "inventoryAppraise"
  | "inventoryEquip"
  | "inventoryDrop"
  | "inventoryDropQuantity"
  | "inventoryInscription"
  | "inventoryInscribe"
  | "inventoryDestroy"
  | "inventoryList"
  | "equipmentList"
>;

interface InventoryFormatter {
  visibleItemName(displayNameKey: string, kindId: string, artifactName?: string | null): string;
  itemPropertyName(nameKey: string): string;
  itemQualityName(quality: NonNullable<InventoryItemDto["quality"]>): string;
  equipmentSlotName(slot: string): string;
  damageTypeName(type: ResistanceDto["damageType"]): string;
  statusName(id: string | undefined): string;
}

export class InventoryPanel {
  readonly #dom: InventoryDom;
  readonly #state: AppState;
  readonly #localization: Localization;
  readonly #formatter: InventoryFormatter;
  readonly #dispatch: (command: GameCommand) => Promise<void>;
  readonly #onInventoryInteraction: () => void;
  readonly #startTargeting: (
    spec: TargetSpecDto | null | undefined,
    intent: TargetingIntent,
  ) => void;
  readonly #announce: (
    key: MessageKey,
    args: Record<string, string | number> | undefined,
    kind: string,
  ) => void;
  readonly #itemCurseSeverityName: (
    curse: ItemCurseSeverityDto | string | undefined,
  ) => string;
  #installed = false;
  #detailItemId: string | undefined;
  #pendingAction: { kind: "drop" | "inscribe" | "destroy"; itemId: string } | undefined;
  #closeItemSelection: (() => void) | undefined;
  #selectionSnapshot: AppState["status"];
  #actionSnapshot: AppState["status"];

  constructor(options: {
    dom: InventoryDom;
    state: AppState;
    localization: Localization;
    formatter: InventoryFormatter;
    dispatch: (command: GameCommand) => Promise<void>;
    onInventoryInteraction?: () => void;
    startTargeting: (
      spec: TargetSpecDto | null | undefined,
      intent: TargetingIntent,
    ) => void;
    announce: (
      key: MessageKey,
      args: Record<string, string | number> | undefined,
      kind: string,
    ) => void;
    itemCurseSeverityName: (
      curse: ItemCurseSeverityDto | string | undefined,
    ) => string;
  }) {
    this.#dom = options.dom;
    this.#state = options.state;
    this.#localization = options.localization;
    this.#formatter = options.formatter;
    this.#dispatch = options.dispatch;
    this.#onInventoryInteraction = options.onInventoryInteraction ?? (() => undefined);
    this.#startTargeting = options.startTargeting;
    this.#announce = options.announce;
    this.#itemCurseSeverityName = options.itemCurseSeverityName;
  }

  install(): void {
    if (this.#installed) return;
    this.#installed = true;
    this.#dom.inventoryDetailClose.addEventListener("click", this.#closeDetail);
    this.#dom.inventoryDetailDialog.addEventListener("close", this.#handleDetailClosed);
    this.#dom.inventoryMore.addEventListener("click", this.#openMore);
    this.#dom.inventoryActionForm.addEventListener("submit", this.#submitAction);
    this.#dom.inventoryActionCancel.addEventListener("click", this.#closeAction);
    this.#dom.inventoryActionDialog.addEventListener("close", this.#handleActionClosed);
    this.#dom.inventoryFilters.addEventListener("change", this.#handleFilterChange);
    this.#dom.inventorySearch.addEventListener("input", this.#handleFilterChange);
    this.#dom.inventoryFilterReset.addEventListener("click", this.#handleFilterReset);
    this.#dom.inventoryUse.addEventListener("click", this.#handleUse);
    this.#dom.inventoryAbsorb.addEventListener("click", this.#handleAbsorb);
    this.#dom.inventoryRead.addEventListener("click", this.#handleRead);
    this.#dom.inventoryUseOnMount.addEventListener("click", this.#handleUseOnMount);
    this.#dom.inventoryAppraise.addEventListener("click", this.#handleAppraise);
    this.#dom.inventoryEquip.addEventListener("click", this.#handleEquip);
    this.#dom.inventoryDrop.addEventListener("click", this.#handleDrop);
    this.#dom.inventoryInscribe.addEventListener("click", this.#handleInscribe);
    this.#dom.inventoryDestroy.addEventListener("click", this.#handleDestroy);
    this.#dom.inventoryDropQuantity.addEventListener("input", this.#handleQuantityInput);
  }

  dispose(): void {
    if (!this.#installed) return;
    this.reset();
    this.#installed = false;
    this.#dom.inventoryDetailClose.removeEventListener("click", this.#closeDetail);
    this.#dom.inventoryDetailDialog.removeEventListener("close", this.#handleDetailClosed);
    this.#closeDetail();
    this.#dom.inventoryMore.removeEventListener("click", this.#openMore);
    this.#dom.inventoryActionForm.removeEventListener("submit", this.#submitAction);
    this.#dom.inventoryActionCancel.removeEventListener("click", this.#closeAction);
    this.#dom.inventoryActionDialog.removeEventListener("close", this.#handleActionClosed);
    this.#closeMore();
    this.#closeAction();
    this.#dom.inventoryFilters.removeEventListener("change", this.#handleFilterChange);
    this.#dom.inventorySearch.removeEventListener("input", this.#handleFilterChange);
    this.#dom.inventoryFilterReset.removeEventListener("click", this.#handleFilterReset);
    this.#dom.inventoryUse.removeEventListener("click", this.#handleUse);
    this.#dom.inventoryAbsorb.removeEventListener("click", this.#handleAbsorb);
    this.#dom.inventoryRead.removeEventListener("click", this.#handleRead);
    this.#dom.inventoryUseOnMount.removeEventListener("click", this.#handleUseOnMount);
    this.#dom.inventoryAppraise.removeEventListener("click", this.#handleAppraise);
    this.#dom.inventoryEquip.removeEventListener("click", this.#handleEquip);
    this.#dom.inventoryDrop.removeEventListener("click", this.#handleDrop);
    this.#dom.inventoryInscribe.removeEventListener("click", this.#handleInscribe);
    this.#dom.inventoryDestroy.removeEventListener("click", this.#handleDestroy);
    this.#dom.inventoryDropQuantity.removeEventListener("input", this.#handleQuantityInput);
  }

  render(inventory: InventoryItemDto[], equipment: EquipmentItemDto[]): void {
    if (this.#selectionSnapshot !== this.#state.status) this.#closeItemSelection?.();
    if (this.#actionSnapshot !== this.#state.status) this.#closeAction();
    this.#state.inventory = inventory.map((item) => ({ ...item }));
    this.#state.equipment = equipment.map((item) => ({ ...item }));
    const stacks = this.#localization.format("inventory-stack-count", {
      count: inventory.length,
    });
    const encumbrance = this.#state.status
      ? this.#state.status.player.encumbranceSpeedPenalty > 0
        ? this.#localization.format("inventory-encumbrance-penalty", {
            penalty: this.#state.status.player.encumbranceSpeedPenalty,
          })
        : ""
      : "";
    this.#dom.inventoryCount.dataset.overburdened = String(
      Boolean(
        this.#state.status &&
          this.#state.status.player.carriedWeightTenthsPound >
            this.#state.status.player.carryCapacityTenthsPound,
      ),
    );
    this.#dom.inventoryCount.style.color =
      this.#dom.inventoryCount.dataset.overburdened === "true" ? "#f87171" : "";
    this.#dom.inventoryCount.textContent = this.#state.status
      ? this.#localization.format("inventory-weight-summary", {
          stacks,
          usedSlots: this.#state.status.player.inventoryUsedSlots,
          slotCapacity: this.#state.status.player.inventorySlotCapacity,
          weight: formatTenthsPound(this.#state.status.player.carriedWeightTenthsPound),
          capacity: formatTenthsPound(this.#state.status.player.carryCapacityTenthsPound),
          encumbrance,
        })
      : stacks;
    this.#renderInventoryItems(inventory);
    this.#renderEquipment(equipment);
    if (this.#dom.inventoryDetailDialog.open) this.#renderDetail();
    this.updateActions();
  }

  updateActions(): void {
    const worldMap = this.#state.worldMap;
    const selected = this.#selectedItems();
    this.#dom.inventorySelectionCount.textContent = this.#localization.format(
      "inventory-selected-count",
      { count: selected.length },
    );
    const blocked = this.#state.busy || this.#state.playerDead || worldMap;
    const item = selected.length === 1 ? selected[0] : undefined;
    const actions: [HTMLButtonElement, boolean][] = [
      [this.#dom.inventoryEquip, Boolean(item?.equipmentSlot)],
      [this.#dom.inventoryUse, Boolean(item?.usable || selectedRechargingItems(selected))],
      [this.#dom.inventoryAbsorb, absorbableItemCandidates(this.#state,
        (key, kindId, artifactName) => this.#formatter.visibleItemName(key, kindId, artifactName)).length > 0],
      [this.#dom.inventoryRead, this.#readableItems().length > 0],
      [this.#dom.inventoryUseOnMount, Boolean(item?.mountUsable && this.#state.status?.player.ridingActorId)],
      [this.#dom.inventoryAppraise, item?.identification === "unexamined"],
      [this.#dom.inventoryDrop, selected.length > 0],
      [this.#dom.inventoryInscribe, Boolean(item)],
      [this.#dom.inventoryDestroy, Boolean(item)],
    ];
    for (const [button, available] of actions) {
      button.hidden = !available;
      button.disabled = blocked || !available;
    }
    this.#dom.inventoryMore.hidden = [
      this.#dom.inventoryAbsorb, this.#dom.inventoryRead, this.#dom.inventoryUseOnMount, this.#dom.inventoryAppraise,
      this.#dom.inventoryInscribe, this.#dom.inventoryDestroy,
    ].every((button) => button.hidden);
    this.#dom.inventoryMore.disabled = blocked;
    if (this.#pendingAction) {
      if (!item || item.id !== this.#pendingAction.itemId) {
        this.#closeAction();
      } else {
        const inscription = this.#pendingAction.kind === "inscribe";
        this.#dom.inventoryDropQuantity.max = String(item.quantity);
        this.#dom.inventoryDropQuantity.disabled = blocked || inscription;
        this.#dom.inventoryInscription.disabled = blocked || !inscription;
        this.#dom.inventoryActionConfirm.disabled = blocked ||
          (!inscription && parseDropQuantity(this.#dom.inventoryDropQuantity.value, item.quantity) === undefined);
      }
    }
    for (const checkbox of this.#dom.inventoryList.querySelectorAll<HTMLInputElement>(
      'input[type="checkbox"]',
    )) {
      checkbox.disabled = this.#state.busy || this.#state.playerDead || worldMap;
    }
    for (const button of this.#dom.inventoryDetailActions.querySelectorAll<HTMLButtonElement>("button")) {
      const refuelTargetId = button.dataset.refuelTargetId;
      const activationItem = this.#state.equipment.find(item => item.id === button.dataset.activationItemId);
      button.disabled =
        this.#state.busy ||
        this.#state.playerDead ||
        worldMap ||
        Boolean(activationItem?.useUnavailableReason || (activationItem?.activation && !activationItem.usable)) ||
        (refuelTargetId !== undefined && this.#refuelSourceForTarget(refuelTargetId) === undefined);
    }
  }

  selectItemTarget(
    excludedItemId: string | undefined,
    onSelect: (itemId: string) => Promise<void>,
    onCancel?: () => Promise<void>,
    allowedItemIds?: readonly string[],
    command?: CommandShortcut,
  ): void {
    const candidates = itemTargetCandidates(
      this.#state,
      excludedItemId,
      (displayNameKey, kindId, artifactName) => this.#formatter.visibleItemName(displayNameKey, kindId, artifactName),
    );
    this.#selectItemTargetFrom(candidates.filter(candidate => !allowedItemIds || allowedItemIds.includes(candidate.id)), onSelect, onCancel, "item-target-title", command);
  }

  swapRings(): void {
    if (this.#state.busy || this.#state.commandBlocked || this.#state.worldMap) return;
    const slots = this.#state.bodySlots.filter(slot => slot.slotType === "ring");
    if (!this.#state.equipment.some(item => slots.some(slot => slot.id === item.slotId))) {
      this.#announce("message-ring-swap-empty", undefined, "system"); return;
    }
    if (slots.length < 2) {
      this.#announce("item-ring-swap-unavailable", undefined, "system"); return;
    }
    const swap = (firstSlotId: string, secondSlotId: string) =>
      this.#dispatch({ type: "swap-rings", firstSlotId, secondSlotId });
    if (slots.length === 2) { void swap(slots[0]!.id, slots[1]!.id); return; }
    const candidates = slots.map((slot, index) => ({
      id: slot.id,
      label: this.#localization.format("equipment-slot-ordinal", {
        slot: this.#formatter.equipmentSlotName("ring"), ordinal: index + 1,
      }),
    }));
    this.#selectOptionFrom(candidates, async first => {
      this.#selectOptionFrom(candidates.filter(slot => slot.id !== first),
        second => swap(first, second), "ring-swap-second-slot");
    }, "ring-swap-first-slot");
  }

  reset(): void {
    this.#closeItemSelection?.();
    this.#closeAction();
    this.#closeDetail();
    this.#closeMore();
    this.#state.selectedInventoryIds.clear();
  }

  selectItemTargets(excludedItemId: string, onSelect: (ids: string[]) => Promise<void>, onCancel: () => Promise<void>, command: CommandShortcut, multiple: boolean): () => void {
    const selected: string[] = [];
    let close: (() => void) | undefined;
    const next = () => {
      const candidates = itemTargetCandidates(this.#state, excludedItemId,
        (key, kind, name) => this.#formatter.visibleItemName(key, kind, name)).filter(item => !selected.includes(item.id));
      if (candidates.length === 0 && selected.length > 0) { void onSelect(selected); return; }
      this.#selectItemTargetFrom(candidates, async id => {
        selected.push(id);
        if (multiple) next(); else await onSelect(selected);
      }, onCancel, "item-target-title", command,
      multiple && selected.length > 0 ? () => onSelect(selected) : undefined);
      close = this.#closeItemSelection;
    };
    next();
    return () => close?.();
  }

  confirmItemChoice(itemId: string, command?: CommandShortcut): boolean {
    if (this.#state.busy || (command !== "inspect" && (this.#state.commandBlocked || this.#state.worldMap))) return false;
    const snapshot = this.#state.status;
    const item = [...this.#state.inventory, ...this.#state.equipment, ...(snapshot?.items ?? []),
      ...(snapshot?.player.magicEater?.slots.flatMap(slot => slot.item ? [slot.item] : []) ?? []),
      ...(snapshot?.player.magicEater?.deviceCommands.flatMap(entry => entry.items) ?? [])]
      .find(item => item.id === itemId);
    if (!item) return false;
    const confirmations = itemSelectionConfirmations(item.inscription, command ? originalCommandKey(command) : undefined);
    for (let index = 0; index < confirmations; index++) {
      if (!this.#dom.inventoryList.ownerDocument.defaultView?.confirm(this.#localization.format("item-selection-inscription-confirm", {
        name: this.#formatter.visibleItemName(item.displayNameKey, item.kindId, item.artifactName),
      }))) return false;
    }
    return snapshot === this.#state.status;
  }

  #confirmSelected(command?: CommandShortcut): boolean {
    const selected = this.#selectedItems();
    return selected.length > 0 && selected.every(item => this.confirmItemChoice(item.id, command));
  }

  confirmRepeatedCommand(command: GameCommand): boolean {
    let ids: string[] = [];
    let shortcut: CommandShortcut | undefined;
    switch (command.type) {
      case "drop": ids = command.itemIds; shortcut = "drop"; break;
      case "drop-quantity": ids = [command.itemId]; shortcut = "drop"; break;
      case "equip": ids = [command.itemId]; shortcut = "equip"; break;
      case "unequip": {
        const item = this.#state.equipment.find(item => item.slotId === command.slotId);
        if (!item) return false;
        ids = [item.id]; shortcut = "unequip"; break;
      }
      case "destroy-item": ids = [command.itemId]; shortcut = "destroy"; break;
      case "inscribe-item": ids = [command.itemId]; shortcut = command.inscription ? "inscribe" : "uninscribe"; break;
      case "throw": ids = [command.itemId]; shortcut = "throw"; break;
      case "refuel-light": ids = [command.targetItemId, command.sourceItemId]; shortcut = "refuel"; break;
      case "use-item": case "use-item-by-glyph": case "use-item-for-recharge":
        ids = [command.itemId]; shortcut = this.#itemUseCommand(command.itemId);
        if (command.type === "use-item-for-recharge") ids.push(command.sourceItemId, command.targetItemId);
        break;
      case "use-absorbed-device": ids = [command.itemId]; shortcut = "cast";
        ids.push(...command.targets.flatMap(target => "itemId" in target ? [target.itemId] : [])); break;
      case "absorb-device": ids = [command.itemId]; shortcut = "power"; break;
      case "appraise": ids = [command.itemId]; shortcut = "inspect"; break;
      case "open-chest": case "disarm-chest": ids = [command.itemId]; shortcut = command.type === "open-chest" ? "open" : "disarm"; break;
      case "study-ability": case "study-prayer": ids = [command.bookItemId]; shortcut = "study"; break;
      case "cast-ability": {
        const ability = this.#state.status?.player.abilities?.find(ability => ability.id === command.abilityId);
        if (!ability) return false;
        shortcut = ability.source === "learned" ? "cast" : "power";
        if (ability.bookItemId) ids.push(ability.bookItemId);
        break;
      }
      default: return true;
    }
    if ("target" in command && command.target && "itemId" in command.target) ids.push(command.target.itemId);
    return [...new Set(ids)].every(id => this.confirmItemChoice(id, shortcut));
  }

  openCommand(command: ItemShortcut, count?: number): void {
    if (this.#state.busy || this.#state.commandBlocked || this.#state.worldMap) return;
    const inventory = this.#state.inventory;
    const equipment = this.#state.equipment;
    const candidates: Array<InventoryItemDto | EquipmentItemDto> =
      command === "unequip" ? equipment :
      command === "inspect" || command === "throw" || command === "activate" ? [...inventory, ...equipment] :
      command === "refuel" ? equipment : inventory;
    const eligible = candidates.filter(item => {
      if (command === "equip") return "equipmentSlot" in item && Boolean(item.equipmentSlot);
      if (command === "activate") return Boolean(item.activation) && item.usable;
      if (command === "throw") return Boolean(item.throwTargetSpec);
      if (command === "refuel") return Boolean(this.#refuelSourceForTarget(item.id));
      if (command === "uninscribe") return Boolean(item.inscription);
      if (["food", "potion", "scroll", "wand", "staff", "rod"].includes(command)) {
        return "useCategory" in item && item.useCategory === command && item.usable;
      }
      return true;
    });
    this.#selectItemTargetFrom(eligible.map(item => ({ id: item.id, label: this.#itemName(item) })), async itemId => {
      if (this.#state.busy || this.#state.commandBlocked) return;
      const item = [...this.#state.inventory, ...this.#state.equipment].find(item => item.id === itemId);
      if (!item) return;
      if (command === "inspect") { this.#showDetail(itemId); return; }
      if (command === "unequip" && "slotId" in item) { await this.#unequipItem(item.slotId); return; }
      if (command === "refuel") { this.#refuelItem(itemId); return; }
      if (command === "throw") { this.#startTargeting(item.throwTargetSpec, { type: "throw", itemId }); return; }
      if (command === "uninscribe") { await this.#dispatch({ type: "inscribe-item", itemId, inscription: null }); return; }
      if (command === "activate" && "slotId" in item) { this.#activateEquippedItem(item); return; }
      this.#state.selectedInventoryIds.clear();
      this.#state.selectedInventoryIds.add(itemId);
      this.updateActions();
      if (command === "equip") await this.#equipSelectedItem();
      else if (command === "drop" && count !== undefined) {
        this.#dom.inventoryDropQuantity.value = String(Math.min(count, item.quantity));
        await this.#dropSelectedItems();
      }
      else if (command === "drop") this.#startDrop();
      else if (command === "destroy" && count !== undefined) {
        this.#dom.inventoryDropQuantity.value = String(Math.min(count, item.quantity));
        await this.#destroySelectedItem();
      }
      else if (command === "destroy") this.#openAction("destroy");
      else if (command === "inscribe") this.#openAction("inscribe");
      else await this.#useSelectedItem(command);
    }, undefined, `shortcut-${command}`, command);
  }

  selectChest(command: "open-chest" | "disarm-chest", items: readonly ItemDto[], dispatch = this.#dispatch): void {
    this.#selectItemTargetFrom(items.map(item => ({ id: item.id, label: this.#formatter.visibleItemName(item.displayNameKey, item.kindId, item.artifactName) })),
      itemId => dispatch({ type: command, itemId }), undefined, "item-target-title", command === "open-chest" ? "open" : "disarm");
  }

  readonly #handleUse = (): void => {
    const selected = this.#selectedItems();
    const source = selectedRechargingItems(selected)?.item ?? selected[0];
    if (!source || !this.#confirmSelected(this.#itemUseCommand(source.id))) return;
    void this.#useSelectedItem();
  };

  #readableItems(): Array<{ id: string; label: string }> {
    return [...this.#state.inventory, ...(this.#state.status?.items ?? [])]
      .filter(item => item.readable)
      .map(item => ({ id: item.id, label: this.#formatter.visibleItemName(item.displayNameKey, item.kindId, item.artifactName) }));
  }

  readonly #handleRead = (): void => {
    this.#closeMore();
    if (this.#state.busy || this.#state.playerDead || this.#state.worldMap) return;
    this.#onInventoryInteraction();
    this.#selectItemTargetFrom(this.#readableItems(), itemId => this.#dispatch({ type: "use-item", itemId }), undefined, "item-target-title", "scroll");
  };

  readonly #handleAbsorb = (): void => {
    this.#closeMore();
    if (this.#state.busy || this.#state.playerDead || this.#state.worldMap) return;
    this.#onInventoryInteraction();
    this.#selectItemTargetFrom(
      absorbableItemCandidates(
        this.#state,
        (displayNameKey, kindId, artifactName) => this.#formatter.visibleItemName(displayNameKey, kindId, artifactName),
      ),
      (itemId) => this.#dispatch({ type: "absorb-device", itemId }),
      undefined, "item-target-title", "power",
    );
  };

  readonly #handleUseOnMount = (): void => {
    this.#closeMore();
    if (!this.#confirmSelected("potion")) return;
    void this.#useSelectedItemOnMount();
  };

  readonly #handleAppraise = (): void => {
    this.#closeMore();
    if (!this.#confirmSelected("inspect")) return;
    void this.#appraiseSelectedItem();
  };

  readonly #handleEquip = (): void => {
    if (!this.#confirmSelected("equip")) return;
    void this.#equipSelectedItem();
  };

  readonly #handleDrop = (): void => {
    if (this.#confirmSelected("drop")) this.#startDrop();
  };

  #startDrop(): void {
    if (this.#state.busy || this.#state.playerDead || this.#state.worldMap) return;
    const selected = this.#selectedItems();
    if (selected.length === 1 && selected[0]!.quantity > 1) {
      this.#openAction("drop");
    } else {
      this.#dom.inventoryDropQuantity.value = "1";
      void this.#dropSelectedItems();
    }
  }

  readonly #handleInscribe = (): void => {
    if (this.#confirmSelected("inscribe")) this.#openAction("inscribe");
  };

  readonly #handleDestroy = (): void => {
    if (this.#confirmSelected("destroy")) this.#openAction("destroy");
  };

  readonly #openMore = (): void => {
    if (!this.#dom.inventoryMore.disabled && !this.#dom.inventoryMoreDialog.open) {
      this.#dom.inventoryMoreDialog.showModal();
    }
  };

  readonly #closeMore = (): void => {
    if (this.#dom.inventoryMoreDialog.open) this.#dom.inventoryMoreDialog.close();
  };

  #openAction(kind: "drop" | "inscribe" | "destroy"): void {
    this.#closeMore();
    const selected = this.#selectedItems();
    const item = selected[0];
    if (this.#state.busy || this.#state.playerDead || this.#state.worldMap || selected.length !== 1 || !item) return;
    this.#pendingAction = { kind, itemId: item.id };
    this.#actionSnapshot = this.#state.status;
    const title = this.#localization.format(`action-inventory-${kind}`);
    this.#dom.inventoryActionTitle.textContent = title;
    this.#dom.inventoryActionConfirm.textContent = title;
    this.#dom.inventoryActionItem.textContent = this.#itemName(item);
    this.#dom.inventoryDropQuantity.value = String(item.quantity);
    this.#dom.inventoryInscription.value = item.inscription ?? "";
    this.#dom.inventoryQuantityField.hidden = kind === "inscribe";
    this.#dom.inventoryInscriptionField.hidden = kind !== "inscribe";
    this.updateActions();
    this.#dom.inventoryActionDialog.showModal();
    (kind === "inscribe" ? this.#dom.inventoryInscription : this.#dom.inventoryDropQuantity).focus();
  }

  readonly #closeAction = (): void => {
    this.#pendingAction = undefined;
    if (this.#dom.inventoryActionDialog.open) this.#dom.inventoryActionDialog.close();
  };

  readonly #handleActionClosed = (): void => {
    if (!this.#dom.inventoryActionDialog.open) this.#pendingAction = undefined;
  };

  readonly #submitAction = (event: Event): void => {
    event.preventDefault();
    this.updateActions();
    const pending = this.#pendingAction;
    if (this.#actionSnapshot !== this.#state.status) { this.#closeAction(); return; }
    if (!pending || !this.#dom.inventoryActionDialog.open || this.#dom.inventoryActionConfirm.disabled) return;
    if (pending.kind === "drop") void this.#dropSelectedItems();
    else if (pending.kind === "inscribe") void this.#inscribeSelectedItem();
    else void this.#destroySelectedItem();
    this.#closeAction();
  };

  readonly #handleQuantityInput = (): void => {
    this.updateActions();
  };

  readonly #handleFilterChange = (): void => {
    this.#renderInventoryItems(this.#state.inventory);
    this.#dom.inventoryList.scrollTop = 0;
    this.updateActions();
    this.#onInventoryInteraction();
  };

  readonly #handleFilterReset = (): void => {
    this.#dom.inventoryFilters.querySelector<HTMLInputElement>('input[value="all"]')!.checked = true;
    this.#dom.inventorySearch.value = "";
    this.#handleFilterChange();
  };

  #renderInventoryItems(inventory: InventoryItemDto[]): void {
    const filter = this.#dom.inventoryFilters.querySelector<HTMLInputElement>("input:checked")!.value as InventoryFilter;
    const visible = filterInventoryItems(inventory, filter, this.#dom.inventorySearch.value, (item) => this.#itemName(item));
    const visibleIds = new Set(visible.map((item) => item.id));
    for (const itemId of this.#state.selectedInventoryIds) {
      if (!visibleIds.has(itemId)) this.#state.selectedInventoryIds.delete(itemId);
    }
    const document = this.#dom.inventoryList.ownerDocument;
    const scrollTop = this.#dom.inventoryList.scrollTop;
    const rows: HTMLElement[] = [];
    if (visible.length === 0) {
      const empty = document.createElement("li");
      empty.className = "inventory-empty";
      empty.textContent = this.#localization.format(inventory.length === 0 ? "inventory-empty" : "inventory-filter-empty");
      this.#dom.inventoryList.replaceChildren(empty);
      return;
    }
    for (const item of visible) {
      const row = document.createElement("li");
      row.className = "inventory-item";
      row.dataset.itemId = item.id;
      row.dataset.itemKindId = item.kindId;
      const label = document.createElement("label");
      const checkbox = document.createElement("input");
      checkbox.type = "checkbox";
      checkbox.checked = this.#state.selectedInventoryIds.has(item.id);
      checkbox.addEventListener("change", () => {
        if (checkbox.checked) this.#state.selectedInventoryIds.add(item.id);
        else this.#state.selectedInventoryIds.delete(item.id);
        this.#onInventoryInteraction();
        this.updateActions();
      });
      const name = document.createElement("span");
      name.className = "inventory-item-name";
      name.textContent = this.#itemName(item);
      name.title = name.textContent;
      const quantity = document.createElement("span");
      quantity.className = "inventory-quantity";
      quantity.textContent = this.#localization.format("inventory-quantity", {
        quantity: item.quantity,
      });
      const status = document.createElement("span");
      status.className = "inventory-item-status";
      status.textContent = [this.#briefStatus(item), this.#state.display.showDiscounts && item.discountPercent > 0
        ? this.#localization.format("display-item-discount", { percent: item.discountPercent }) : ""].filter(Boolean).join(" · ");
      status.title = status.textContent;
      const weight = document.createElement("span");
      weight.className = "inventory-item-weight";
      weight.hidden = !this.#state.display.showWeights;
      weight.textContent = this.#localization.format("inventory-item-weight", {
        weight: formatTenthsPound(item.weightTenthsPound * item.quantity),
      });
      const inspect = document.createElement("button");
      inspect.type = "button";
      inspect.className = "inventory-item-inspect";
      inspect.textContent = this.#localization.format("action-inventory-details");
      inspect.setAttribute("aria-label", this.#localization.format("inventory-details-for", { name: this.#itemName(item) }));
      inspect.addEventListener("click", () => this.openDetail(item.id));
      label.append(checkbox, this.#itemGlyph(item), name, quantity, status, weight);
      row.append(label, inspect);
      rows.push(row);
    }
    this.#dom.inventoryList.replaceChildren(...rows);
    this.#dom.inventoryList.scrollTop = scrollTop;
  }

  #renderEquipment(equipment: EquipmentItemDto[]): void {
    const document = this.#dom.equipmentList.ownerDocument;
    const scrollTop = this.#dom.equipmentList.scrollTop;
    const rows: HTMLElement[] = [];
    const slots: BodySlotDto[] =
      this.#state.bodySlots.length > 0
        ? this.#state.bodySlots
        : equipment.map((item) => ({ id: item.slotId, slotType: item.slotId }));
    if (slots.length === 0) {
      const empty = document.createElement("li");
      empty.className = "equipment-empty";
      empty.textContent = this.#localization.format("equipment-empty");
      this.#dom.equipmentList.replaceChildren(empty);
      return;
    }
    const byInstance = new Map(equipment.map((item) => [item.slotId, item]));
    const typeCounts = new Map<string, number>();
    for (const slot of slots) {
      typeCounts.set(slot.slotType, (typeCounts.get(slot.slotType) ?? 0) + 1);
    }
    const typeOrdinals = new Map<string, number>();
    for (const slot of slots) {
      const ordinal = (typeOrdinals.get(slot.slotType) ?? 0) + 1;
      typeOrdinals.set(slot.slotType, ordinal);
      const slotLabel =
        (typeCounts.get(slot.slotType) ?? 1) > 1
          ? this.#localization.format("equipment-slot-ordinal", {
              slot: this.#formatter.equipmentSlotName(slot.slotType),
              ordinal,
            })
          : this.#formatter.equipmentSlotName(slot.slotType);
      const row = document.createElement("li");
      row.dataset.slotId = slot.id;
      const item = byInstance.get(slot.id);
      const slotButton = document.createElement("button");
      slotButton.type = "button";
      slotButton.className = "equipment-slot-button";
      const slotTag = document.createElement("span");
      slotTag.className = "equipment-slot";
      slotTag.textContent = slotLabel;
      slotTag.hidden = !this.#state.display.describeSlots && !!item;
      const name = document.createElement("span");
      name.className = "equipment-slot-name";
      name.textContent = item ? this.#itemName(item) + (this.#state.display.showDiscounts && item.discountPercent > 0
        ? " · " + this.#localization.format("display-item-discount", { percent: item.discountPercent }) : "") : this.#localization.format("equipment-slot-vacant");
      slotButton.title = this.#localization.format("equipment-slot-summary", { slot: slotLabel, name: name.textContent });
      slotButton.setAttribute("aria-label", slotButton.title);
      slotButton.append(slotTag, this.#itemGlyph(item), name);
      slotButton.addEventListener("click", () => {
        const current = this.#state.equipment.find((entry) => entry.slotId === slot.id);
        if (current) this.openDetail(current.id);
        else this.#chooseSlotItem(slot.id);
      });
      row.className = item ? "equipment-item" : "equipment-item equipment-slot-vacant";
      row.append(slotButton);
      rows.push(row);
    }
    this.#dom.equipmentList.replaceChildren(...rows);
    this.#dom.equipmentList.scrollTop = scrollTop;
  }

  #itemGlyph(item?: InventoryItemDto | EquipmentItemDto): HTMLElement {
    const glyph = this.#dom.inventoryList.ownerDocument.createElement("span");
    glyph.className = "inventory-item-glyph";
    glyph.hidden = !this.#state.display.showItemIcons;
    glyph.setAttribute("aria-hidden", "true");
    if (item) this.#state.paintVisual(glyph, item.visual.id, item.visual.glyph);
    return glyph;
  }

  #briefStatus(item: InventoryItemDto): string {
    if (item.feeling) return this.#localization.format(`item-feeling-${item.feeling}`);
    if (item.charges) return this.#localization.format("inventory-charges", item.charges);
    if (item.fuel) return this.#localization.format("inventory-fuel", {
      current: item.fuel.current, maximum: item.fuel.maximum,
    });
    if (item.curse) return this.#itemCurseSeverityName(item.curse);
    const identificationKey = itemIdentificationMessageKey(item.identification);
    return item.equipmentSlot && identificationKey ? this.#localization.format(identificationKey) : "";
  }

  #chooseSlotItem(slotId: string): void {
    if (this.#state.busy || this.#state.playerDead || this.#state.worldMap) return;
    const slot = this.#state.bodySlots.find((entry) => entry.id === slotId);
    if (!slot) return;
    const candidates = this.#state.inventory.filter((item) => itemFitsBodySlot(item, slot));
    this.#selectItemTargetFrom(candidates.map((item) => ({ id: item.id, label: this.#itemName(item) })), async (itemId) => {
      const currentSlot = this.#state.bodySlots.find((entry) => entry.id === slotId);
      const currentItem = this.#state.inventory.find((entry) => entry.id === itemId);
      if (this.#state.busy || this.#state.playerDead || this.#state.worldMap ||
        !currentSlot || !currentItem || !itemFitsBodySlot(currentItem, currentSlot) ||
        this.#state.equipment.some((entry) => entry.slotId === slotId)) return;
      await this.#dispatch({ type: "equip", itemId, slotId });
    }, undefined, "item-target-title", "equip");
  }

  openDetail(itemId: string): void {
    const owned = [...this.#state.inventory, ...this.#state.equipment,
      ...(this.#state.status?.player.magicEater?.slots.flatMap(slot => slot.item ? [slot.item] : []) ?? []),
      ...(this.#state.status?.player.magicEater?.deviceCommands.flatMap(command => command.items) ?? [])]
      .some(item => item.id === itemId);
    if (owned && !this.confirmItemChoice(itemId, "inspect")) return;
    this.#showDetail(itemId);
  }

  #showDetail(itemId: string): void {
    this.#detailItemId = itemId;
    this.#renderDetail();
    if (this.#detailItemId && !this.#dom.inventoryDetailDialog.open) {
      this.#dom.inventoryDetailDialog.showModal();
    }
    this.updateActions();
  }

  readonly #closeDetail = (): void => {
    this.#detailItemId = undefined;
    if (this.#dom.inventoryDetailDialog.open) this.#dom.inventoryDetailDialog.close();
  };

  readonly #handleDetailClosed = (): void => {
    if (!this.#dom.inventoryDetailDialog.open) this.#detailItemId = undefined;
  };

  #renderDetail(): void {
    const homeItems = (this.#state.status?.homes ?? []).flatMap((home) => [...home.storedItems, ...home.depositItems])
      .flatMap((item) => item.details ? [item.details] : []);
    const absorbed = this.#state.status?.player.magicEater?.slots.flatMap(slot => slot.item ? [slot.item] : []) ?? [];
    const ordinaryDevices = this.#state.status?.player.magicEater?.deviceCommands.flatMap(command => command.items) ?? [];
    const item = [...this.#state.inventory, ...this.#state.equipment, ...homeItems, ...absorbed, ...ordinaryDevices].find((entry) => entry.id === this.#detailItemId);
    if (!item) { this.#closeDetail(); return; }
    this.#dom.inventoryDetailTitle.textContent = this.#itemName(item);
    const body = this.#dom.inventoryDetailBody;
    const scrollTop = body.scrollTop;
    body.replaceChildren();
    this.#appendItemDetails(body, item);
    this.#dom.inventoryDetailActions.replaceChildren();
    if ("slotId" in item) this.#appendEquipmentActions(this.#dom.inventoryDetailActions, item);
    if (item.throwTargetSpec) {
      const button = body.ownerDocument.createElement("button");
      button.type = "button";
      button.textContent = this.#localization.format("action-inventory-throw");
      button.disabled = this.#state.busy || this.#state.playerDead || this.#state.worldMap;
      button.addEventListener("click", () => {
        if (this.#state.busy || this.#state.playerDead || this.#state.worldMap) return;
        if (!this.confirmItemChoice(item.id, "throw")) return;
        this.#closeDetail();
        this.#startTargeting(item.throwTargetSpec, { type: "throw", itemId: item.id });
      });
      this.#dom.inventoryDetailActions.append(button);
    }
    body.scrollTop = scrollTop;
  }

  #activateEquippedItem(item: EquipmentItemDto): void {
    if (this.#state.busy || this.#state.playerDead || this.#state.worldMap || item.useUnavailableReason || (item.activation && !item.usable)) return;
    if (item.activation?.recallChoice) {
      this.#closeDetail();
      this.#selectJewelRecall(item.id);
      return;
    }
    if (item.requiresRechargeTargets) {
      this.#closeDetail();
      this.#selectRechargeSource(item.id, true);
    } else if (item.useTargetSpec?.modes.includes("self")) {
      void this.#dispatch({ type: "use-item", itemId: item.id, target: { type: "self" } });
    } else if (item.useTargetSpec) {
      this.#closeDetail();
      this.#startTargeting(item.useTargetSpec, { type: "item", itemId: item.id });
    } else {
      void this.#dispatch({ type: "use-item", itemId: item.id });
    }
  }

  #refuelItem(itemId: string): void {
    if (this.#state.busy || this.#state.playerDead || this.#state.worldMap) return;
    const sources = this.#refuelSourcesForTarget(itemId);
    const refuel = (sourceItemId: string) => this.#dispatch({ type: "refuel-light", targetItemId: itemId, sourceItemId });
    if (sources.length === 1) {
      if (this.confirmItemChoice(sources[0]!.id, "refuel")) void refuel(sources[0]!.id);
    } else {
      this.#selectItemTargetFrom(sources.map(item => ({ id: item.id, label: this.#itemName(item) })), refuel, undefined, "item-target-title", "refuel");
    }
  }

  #appendEquipmentActions(container: HTMLElement, item: EquipmentItemDto): void {
    const document = container.ownerDocument;
    const row = document.createElement("div");
    row.className = "equipment-actions";
    if (item.usable || item.activation || (item.captureBall && item.useTargetSpec)) {
      const activate = document.createElement("button");
      activate.type = "button";
      activate.className = "equipment-activate";
      activate.dataset.activationItemId = item.id;
      activate.textContent = this.#localization.format("action-equipment-activate");
      activate.disabled = this.#state.busy || Boolean(item.useUnavailableReason) || (Boolean(item.activation) && !item.usable);
      activate.addEventListener("click", () => { if (this.confirmItemChoice(item.id, "activate")) this.#activateEquippedItem(item); });
      row.append(activate);
    }
    if (item.fuel && item.fuel.kind !== "oil" && item.fuel.current < item.fuel.maximum) {
      const refuel = document.createElement("button");
      refuel.type = "button";
      refuel.className = "equipment-refuel";
      refuel.dataset.refuelTargetId = item.id;
      refuel.textContent = this.#localization.format("action-equipment-refuel");
      refuel.disabled = this.#refuelSourceForTarget(item.id) === undefined;
      refuel.addEventListener("click", () => { if (this.confirmItemChoice(item.id, "refuel")) this.#refuelItem(item.id); });
      row.append(refuel);
    }
    const unequip = document.createElement("button");
    unequip.type = "button";
    unequip.textContent = this.#localization.format("action-equipment-unequip");
    unequip.disabled = this.#state.busy;
    unequip.addEventListener("click", () => { if (this.confirmItemChoice(item.id, "unequip")) void this.#unequipItem(item.slotId); });
    row.append(unequip);
    container.append(row);
  }

  #appendItemDetails(container: HTMLElement, item: InventoryItemDto | EquipmentItemDto): void {
    const document = container.ownerDocument;
    const name = document.createElement("span");
    name.className = "inventory-item-name";
    name.textContent = this.#itemName(item);
    container.append(name);
    if (item.useUnavailableReason) {
      this.#appendDetail(container, "inventory-use-unavailable", this.#localization.format(`item-use-unavailable-${item.useUnavailableReason}`));
    }
    this.#appendDetail(container, "inventory-quantity", this.#localization.format("inventory-quantity", { quantity: item.quantity }));
    this.#appendDetail(container, "inventory-item-weight", this.#localization.format("inventory-item-weight", {
      weight: formatTenthsPound(item.weightTenthsPound * item.quantity),
    }));
    this.#appendInscription(container, item.inscription);
    if (this.#state.display.showOrigins) this.#appendDetail(container, "item-origin", this.#localization.format("display-item-origin", { origin: this.#localization.format("item-origin-" + (item.originKind ?? "unknown")) }));
    if (item.discountPercent > 0) this.#appendDetail(container, "item-discount", this.#localization.format("display-item-discount", { percent: item.discountPercent }));
    const slotType = "slotId" in item
      ? this.#state.bodySlots.find((slot) => slot.id === item.slotId)?.slotType ?? item.slotId
      : item.equipmentSlot;
    if (slotType) {
      const equippable = document.createElement("span");
      equippable.className = "inventory-equippable";
      equippable.textContent = this.#localization.format("inventory-equippable", {
        slot: this.#formatter.equipmentSlotName(slotType),
      });
      container.append(equippable);
    }
    if (item.charges) {
      const charges = document.createElement("span");
      charges.className = "inventory-charges";
      charges.textContent = this.#localization.format("inventory-charges", {
        current: item.charges.current,
        maximum: item.charges.maximum,
      });
      container.append(charges);
    }
    this.#appendItemFuel(container, item);
    if (item.activation) {
      const activation = document.createElement("span");
      activation.className = "inventory-activation";
      activation.textContent = this.#localization.format("inventory-activation", {
        activation: this.#localization.format(item.activation.nameKey as MessageKey),
        power: item.activation.power,
        cost: item.activation.cost,
      });
      container.append(activation);
    }
    this.#appendKnownDetails(container, item);
  }

  #appendItemFuel(
    container: HTMLElement,
    item: InventoryItemDto | EquipmentItemDto,
  ): void {
    if (!item.fuel) return;
    const fuel = container.ownerDocument.createElement("span");
    fuel.className = "inventory-fuel";
    fuel.textContent = this.#localization.format("inventory-fuel", {
      current: item.fuel.current,
      maximum: item.fuel.maximum,
    });
    container.append(fuel);
  }

  #appendInscription(container: HTMLElement, inscription: string | null | undefined): void {
    if (!inscription) return;
    const value = container.ownerDocument.createElement("span");
    value.className = "inventory-inscription";
    value.textContent = this.#localization.format("inventory-inscription", { inscription });
    container.append(value);
  }

  #refuelSourceForTarget(targetItemId: string): InventoryItemDto | undefined {
    return this.#refuelSourcesForTarget(targetItemId)[0];
  }

  #refuelSourcesForTarget(targetItemId: string): InventoryItemDto[] {
    const target = this.#state.equipment.find((item) => item.id === targetItemId);
    if (!target?.fuel || target.fuel.current >= target.fuel.maximum) return [];
    return this.#state.inventory.filter((item) => {
      if (!item.fuel || item.fuel.current === 0) return false;
      return target.fuel?.kind === "torch"
        ? item.fuel.kind === "torch"
        : target.fuel?.kind === "lantern" &&
            (item.fuel.kind === "lantern" || item.fuel.kind === "oil");
    });
  }

  #appendKnownDetails(
    container: HTMLElement,
    item: InventoryItemDto | EquipmentItemDto,
  ): void {
    if (item.captureBall) {
      const captured = container.ownerDocument.createElement("span");
      captured.className = "capture-ball-status";
      captured.textContent = item.capturedActor
        ? this.#localization.format("capture-ball-contained", {
            actor: item.capturedActor.customName ?? this.#localization.format(item.capturedActor.nameKey as MessageKey),
            hp: item.capturedActor.hp,
            maximum: item.capturedActor.maxHp,
            experience: item.capturedActor.experience,
          })
        : this.#localization.format("capture-ball-empty");
      container.append(captured);
    }
    if (item.feeling) {
      this.#appendDetail(container, "item-feeling", this.#localization.format("item-feeling-label", {
        feeling: this.#localization.format(`item-feeling-${item.feeling}`),
      }));
    }
    const identificationKey = itemIdentificationMessageKey(item.identification);
    if (("slotId" in item || item.equipmentSlot !== null) && identificationKey) {
      const identification = container.ownerDocument.createElement("span");
      identification.className = `item-identification item-identification-${item.identification}`;
      identification.textContent = this.#localization.format(identificationKey);
      container.append(identification);
    }
    this.#appendItemModifiers(container, item.modifiers);
    if (item.bagCapacity !== undefined && item.bagCapacity !== null) {
      this.#appendDetail(container, "inventory-bag-capacity", this.#localization.format("inventory-bag-capacity", {
        capacity: item.bagCapacity,
      }));
    }
    this.#appendItemEnchantments(container, item.enchantments);
    this.#appendItemCurse(container, item.curse);
    this.#appendEquipmentBonuses(container, item.equipmentBonuses);
    this.#appendItemDefenses(container, item.resistances, item.statusImmunities);
    this.#appendDestructionImmunities(container, item.permanentDestructionImmunities);
    this.#appendItemOffense(container, item.slays, item.brands);
    this.#appendEquipmentPassives(container, item.passives);
    this.#appendItemQuality(container, item.quality);
    this.#appendKnownItemProperties(container, item.knownProperties);
  }

  #itemName(item: InventoryItemDto | EquipmentItemDto): string {
    const ball = this.#formatter.visibleItemName(item.displayNameKey, item.kindId, item.artifactName);
    return item.capturedActor
      ? this.#localization.format("capture-ball-name-contained", {
          ball,
          actor: item.capturedActor.customName ?? this.#localization.format(item.capturedActor.nameKey as MessageKey),
        })
      : ball;
  }

  async #equipSelectedItem(): Promise<void> {
    const selected = this.#selectedItems();
    if (this.#state.busy || selected.length !== 1 || !selected[0]?.equipmentSlot) return;
    const item = selected[0];
    if (item.equipmentSlot !== "tool") {
      await this.#dispatch({ type: "equip", itemId: item.id });
      return;
    }
    const eligibleSlots = this.#state.bodySlots.filter(
      (slot) => slot.slotType === "tool" || slot.slotType === "weapon",
    );
    const typeCounts = new Map<string, number>();
    for (const slot of eligibleSlots) {
      typeCounts.set(slot.slotType, (typeCounts.get(slot.slotType) ?? 0) + 1);
    }
    const typeOrdinals = new Map<string, number>();
    const candidates = eligibleSlots.map((slot) => {
      const ordinal = (typeOrdinals.get(slot.slotType) ?? 0) + 1;
      typeOrdinals.set(slot.slotType, ordinal);
      const slotName = this.#formatter.equipmentSlotName(slot.slotType);
      return {
        id: slot.id,
        label:
          (typeCounts.get(slot.slotType) ?? 1) > 1
            ? this.#localization.format("equipment-slot-ordinal", { slot: slotName, ordinal })
            : slotName,
      };
    });
    this.#selectOptionFrom(candidates, (slotId) =>
      this.#dispatch({ type: "equip", itemId: item.id, slotId }),
    );
  }

  async #appraiseSelectedItem(): Promise<void> {
    const selected = this.#selectedItems();
    if (
      this.#state.busy ||
      selected.length !== 1 ||
      selected[0]?.identification !== "unexamined"
    ) {
      return;
    }
    await this.#dispatch({ type: "appraise", itemId: selected[0].id });
  }

  async #useSelectedItem(shortcut?: ItemShortcut): Promise<void> {
    const selected = this.#selectedItems();
    if (this.#state.busy) return;
    const recharge = selectedRechargingItems(selected);
    if (recharge) {
      this.#selectRechargeTarget(recharge.item.id, recharge.source.id,
        recharge.item.activation ? () => this.#dispatch({ type: "use-item", itemId: recharge.item.id }) : undefined,
        shortcut ?? this.#itemUseCommand(recharge.item.id));
      return;
    }
    if (selected.length !== 1 || !selected[0]?.usable) return;
    const item = selected[0];
    const command = shortcut ?? this.#itemUseCommand(item.id);
    if (item.activation?.recallChoice) {
      this.#selectJewelRecall(item.id);
      return;
    }
    if (item.requiresRechargeTargets) {
      this.#selectRechargeSource(item.id, Boolean(item.activation), command);
      return;
    }
    if (item.mundanityTargets) {
      this.selectItemTarget(item.id, async (itemId) => {
        const option = item.mundanityTargets?.find(option => option.itemId === itemId);
        if (!option) return;
        if (option.confirmationKey && !this.#dom.inventoryList.ownerDocument.defaultView?.confirm(
          this.#localization.format(option.confirmationKey as MessageKey))) return;
        await this.#dispatch({ type: "use-item", itemId: item.id, target: option.target });
      }, undefined, item.mundanityTargets.map(option => option.itemId), command);
      return;
    }
    if (item.artifactCreationTargets) {
      const candidates = itemTargetCandidates(this.#state, item.id,
        (key, kind, name) => this.#formatter.visibleItemName(key, kind, name))
        .filter((candidate) => item.artifactCreationTargets!.includes(candidate.id));
      this.#selectItemTargetFrom(candidates, async (targetItemId) => {
        const target = [...this.#state.inventory, ...this.#state.equipment, ...(this.#state.status?.items ?? [])]
          .find((candidate) => candidate.id === targetItemId);
        if (!target || this.#state.busy || this.#state.playerDead || this.#state.worldMap) return;
        const view = this.#dom.inventoryList.ownerDocument.defaultView;
        if (target.quantity > 1 && !view?.confirm(this.#localization.format("inventory-artifact-creation-confirm", {
          quantity: target.quantity - 1,
        }))) return;
        const name = view?.prompt(this.#localization.format("inventory-artifact-creation-name"), "") || undefined;
        await this.#dispatch({ type: "use-item", itemId: item.id,
          target: { type: "artifact-creation-item", itemId: targetItemId, quantity: target.quantity, ...(name ? { name } : {}) } });
      }, undefined, "item-target-title", command);
      return;
    }
    if (item.requiresCraftingTarget) {
      this.selectItemTarget(item.id, async (targetItemId) => {
        const target = [...this.#state.inventory, ...this.#state.equipment, ...(this.#state.status?.items ?? [])]
          .find((candidate) => candidate.id === targetItemId);
        if (!target || this.#state.busy || this.#state.playerDead || this.#state.worldMap) return;
        if (target.quantity > 30 && target.quantity <= 59 &&
          !this.#dom.inventoryList.ownerDocument.defaultView?.confirm(
            this.#localization.format("inventory-crafting-confirm", {
              quantity: target.quantity,
              chance: Math.trunc((target.quantity * 20 - 597) / 6),
            }),
          )) return;
        await this.#dispatch({ type: "use-item", itemId: item.id,
          target: { type: "crafting-item", itemId: targetItemId, quantity: target.quantity } });
      }, undefined, undefined, command);
      return;
    }
    if (item.requiresTargetGlyph) {
      this.#selectGlyphTarget((glyph) =>
        this.#dispatch({ type: "use-item-by-glyph", itemId: item.id, glyph }),
      );
      return;
    }
    if (item.useTargetSpec?.modes.includes("self")) {
      await this.#dispatch({ type: "use-item", itemId: item.id, target: { type: "self" } });
      return;
    }
    if (item.useTargetSpec?.modes.includes("item")) {
      this.selectItemTarget(item.id, (itemId) =>
        this.#dispatch({
          type: "use-item",
          itemId: item.id,
          target: { type: "item", itemId },
        }),
        () => this.#dispatch({ type: "use-item", itemId: item.id }),
        undefined, command,
      );
      return;
    }
    if (item.useTargetSpec) {
      this.#startTargeting(item.useTargetSpec, { type: "item", itemId: item.id });
      return;
    }
    await this.#dispatch({ type: "use-item", itemId: item.id });
  }

  async #useSelectedItemOnMount(): Promise<void> {
    const [item] = this.#selectedItems();
    const entityId = this.#state.status?.player.ridingActorId;
    if (this.#state.busy || !item?.mountUsable || !entityId) return;
    await this.#dispatch({
      type: "use-item",
      itemId: item.id,
      target: { type: "entity", entityId },
    });
  }

  async #dropSelectedItems(): Promise<void> {
    const selected = this.#selectedItems();
    if (this.#state.busy || selected.length === 0) return;
    const [item] = selected;
    if (selected.length === 1 && item) {
      const quantity = parseDropQuantity(this.#dom.inventoryDropQuantity.value, item.quantity);
      if (quantity === undefined) return;
      if (quantity < item.quantity) {
        await this.#dispatch({ type: "drop-quantity", itemId: item.id, quantity });
        return;
      }
    }
    const itemIds = selected.map((item) => item.id).sort();
    await this.#dispatch({ type: "drop", itemIds });
  }

  async #inscribeSelectedItem(): Promise<void> {
    const selected = this.#selectedItems();
    if (this.#state.busy || selected.length !== 1 || !selected[0]) return;
    await this.#dispatch({
      type: "inscribe-item",
      itemId: selected[0].id,
      inscription: this.#dom.inventoryInscription.value || null,
    });
  }

  async #destroySelectedItem(): Promise<void> {
    const selected = this.#selectedItems();
    if (this.#state.busy || selected.length !== 1 || !selected[0]) return;
    const item = selected[0];
    const quantity = parseDropQuantity(this.#dom.inventoryDropQuantity.value, item.quantity);
    if (quantity === undefined) return;
    const name = this.#itemName(item);
    const confirmed = this.#dom.inventoryList.ownerDocument.defaultView?.confirm(
      this.#localization.format("inventory-destroy-confirm", { name, quantity }),
    );
    if (!confirmed) return;
    await this.#dispatch({ type: "destroy-item", itemId: item.id, quantity });
  }

  async #unequipItem(slotId: string): Promise<void> {
    if (this.#state.busy || this.#state.playerDead || this.#state.worldMap) return;
    await this.#dispatch({ type: "unequip", slotId });
  }

  #selectedItems(): InventoryItemDto[] {
    return this.#state.inventory.filter((item) => this.#state.selectedInventoryIds.has(item.id));
  }

  #itemUseCommand(itemId: string): ItemShortcut | undefined {
    const item = [...this.#state.inventory, ...this.#state.equipment].find(item => item.id === itemId);
    if (item && "useCategory" in item && item.useCategory) return item.useCategory;
    return item?.activation ? "activate" : undefined;
  }

  #selectRechargeSource(itemId: string, activation: boolean, command = this.#itemUseCommand(itemId)): void {
    const onCancel = activation ? () => this.#dispatch({ type: "use-item", itemId }) : undefined;
    const candidates = [...this.#state.inventory, ...(this.#state.status?.items ?? [])]
      .filter(item => item.id !== itemId && item.canSupplyRecharge)
      .map(item => ({ id: item.id, label: this.#formatter.visibleItemName(item.displayNameKey, item.kindId, item.artifactName) }));
    this.#selectItemTargetFrom(candidates, async sourceItemId => {
      this.#selectRechargeTarget(itemId, sourceItemId, onCancel, command);
    }, onCancel, "inventory-recharge-source-title", command);
  }

  #selectRechargeTarget(itemId: string, sourceItemId: string, onCancel?: () => Promise<void>, command = this.#itemUseCommand(itemId)): void {
    const candidates = [...this.#state.inventory, ...(this.#state.status?.items ?? [])]
      .filter(
        (item) => item.id !== itemId && item.id !== sourceItemId && item.canReceiveRecharge,
      )
      .map((item) => ({
        id: item.id,
        label: this.#formatter.visibleItemName(item.displayNameKey, item.kindId, item.artifactName),
      }));
    this.#selectItemTargetFrom(candidates, (targetItemId) =>
      this.#dispatch({
        type: "use-item-for-recharge",
        itemId,
        sourceItemId,
        targetItemId,
      }), onCancel, "inventory-recharge-target-title", command,
    );
  }

  #selectJewelRecall(itemId: string): void {
    this.#selectOptionFrom([
      { id: "no", label: this.#localization.format("jewel-activate-without-recall") },
      { id: "yes", label: this.#localization.format("jewel-activate-with-recall") },
    ], async (choice) => {
      if (this.#state.busy || this.#state.playerDead || this.#state.worldMap) return;
      await this.#dispatch({ type: "use-jewel", itemId, recall: choice === "yes" });
    }, "jewel-recall-title", "jewel-recall-choice");
  }

  #selectItemTargetFrom(
    candidates: Array<{ id: string; label: string }>,
    onSelect: (itemId: string) => Promise<void>,
    onCancel?: () => Promise<void>,
    titleKey: MessageKey = "item-target-title",
    command?: CommandShortcut,
    onFinish?: () => Promise<void>,
  ): void {
    this.#closeItemSelection?.();
    const snapshot = this.#state.status;
    this.#selectionSnapshot = snapshot;
    const commandKey = command ? originalCommandKey(command) : undefined;
    const items = [...this.#state.inventory, ...this.#state.equipment, ...(snapshot?.items ?? [])];
    const entries = candidates.map(candidate => ({ ...candidate, source: itemSelectionSource(this.#state, candidate.id),
      inscription: items.find(item => item.id === candidate.id)?.inscription }));
    const sources: ItemSelectionSource[] = ["pack", "equipment", "quiver", "floor"];
    const tabs = sources.map(source => ({ source, entries: entries.filter(entry => entry.source === source), page: 0 }))
      .filter(tab => tab.entries.length > 0);
    if (tabs.length === 0) {
      this.#announce("message-item-selection-empty", undefined, "system");
      void onCancel?.();
      return;
    }
    const document = this.#dom.inventoryList.ownerDocument;
    const dialog = document.createElement("dialog");
    dialog.className = "item-target-dialog item-selection-dialog";
    const form = document.createElement("form");
    form.method = "dialog";
    const title = document.createElement("h2");
    title.textContent = this.#localization.format(titleKey);
    const label = document.createElement("label");
    const labelText = document.createElement("span");
    labelText.textContent = this.#localization.format("item-target-label");
    const select = document.createElement("select");
    let activeTab = tabs[0]!;
    let ignoreInscriptions = false;
    let labels: string[] = [];
    label.append(labelText, select);
    const actions = document.createElement("div");
    actions.className = "item-target-actions";
    const cancel = document.createElement("button");
    cancel.type = "button";
    cancel.textContent = this.#localization.format("action-dialog-cancel");
    cancel.addEventListener("click", () => dialog.close());
    const confirm = document.createElement("button");
    confirm.type = "submit";
    confirm.textContent = this.#localization.format("action-item-target-confirm");
    actions.append(cancel, confirm);
    form.append(title, label, actions);
    const pages = document.createElement("div");
    pages.className = "item-selection-pages";
    const previous = document.createElement("button");
    previous.type = "button";
    previous.textContent = this.#localization.format("item-selection-page-previous");
    const next = document.createElement("button");
    next.type = "button";
    next.textContent = this.#localization.format("item-selection-page-next");
    const pageLabel = document.createElement("span");
    pageLabel.setAttribute("aria-live", "polite");
    pages.append(previous, pageLabel, next);
    const help = document.createElement("p");
    help.textContent = this.#localization.format("item-selection-key-help");
    const details = document.createElement("div");
    details.className = "item-selection-details";
    details.setAttribute("aria-live", "polite");
    details.hidden = true;
    form.append(pages, help, details);
    const sourceControls = document.createElement("div");
    sourceControls.className = "item-selection-sources";
    sourceControls.setAttribute("role", "group");
    sourceControls.setAttribute("aria-label", this.#localization.format("item-selection-sources"));
    const sourceButtons = tabs.map(tab => {
      const button = document.createElement("button");
      button.type = "button";
      button.dataset.source = tab.source;
      button.textContent = this.#localization.format(`item-selection-source-${tab.source}`);
      button.addEventListener("click", () => selectSource(tab.source));
      sourceControls.append(button);
      return button;
    });
    form.append(sourceControls);
    const inscriptionToggle = document.createElement("button");
    inscriptionToggle.type = "button";
    inscriptionToggle.textContent = this.#localization.format("item-selection-ignore-inscriptions");
    inscriptionToggle.addEventListener("click", () => toggleInscriptions());
    form.append(inscriptionToggle);
    if (onFinish) {
      const done = document.createElement("button");
      done.type = "button";
      done.textContent = this.#localization.format("item-selection-finish");
      done.addEventListener("click", () => {
        if (!dialog.open || this.#state.busy || this.#state.commandBlocked || snapshot !== this.#state.status) return;
        selected = true; dialog.close(); void onFinish();
      });
      form.append(done);
    }
    const visibleEntries = () => activeTab.entries.slice(activeTab.page * 26, (activeTab.page + 1) * 26);
    const renderPage = () => {
      select.replaceChildren();
      labels = itemSelectionLabels(visibleEntries().map(entry => entry.inscription), commandKey, ignoreInscriptions);
      visibleEntries().forEach((candidate, index) => {
        const option = document.createElement("option");
        option.value = candidate.id;
        option.textContent = `${labels[index]}) ${candidate.label}`;
        if (candidate.source) option.dataset.source = candidate.source;
        select.append(option);
      });
      select.size = Math.min(10, visibleEntries().length);
      select.value = activeTab.entries[activeTab.page * 26]!.id;
      const pageCount = Math.ceil(activeTab.entries.length / 26);
      pages.hidden = pageCount === 1;
      pageLabel.textContent = this.#localization.format("item-selection-page", { page: activeTab.page + 1, pages: pageCount });
      sourceButtons.forEach((button, index) => button.setAttribute("aria-pressed", String(tabs[index] === activeTab)));
      inscriptionToggle.setAttribute("aria-pressed", String(ignoreInscriptions));
      details.hidden = true;
    };
    const toggleInscriptions = () => {
      if (this.#state.busy || !dialog.open) return;
      ignoreInscriptions = !ignoreInscriptions;
      const itemId = select.value;
      renderPage();
      select.value = itemId;
      select.focus();
    };
    const selectSource = (source: ItemSelectionSource) => {
      if (this.#state.busy || this.#state.commandBlocked || this.#state.worldMap || !dialog.open) return;
      const tab = tabs.find(tab => tab.source === source);
      if (!tab) return;
      activeTab = tab;
      renderPage(); select.focus();
    };
    const turnPage = (step: number) => {
      if (this.#state.busy || !dialog.open) return;
      const pageCount = Math.ceil(activeTab.entries.length / 26);
      activeTab.page = (activeTab.page + step + pageCount) % pageCount;
      renderPage(); select.focus();
    };
    previous.addEventListener("click", () => turnPage(-1));
    next.addEventListener("click", () => turnPage(1));
    let selected = false;
    let closed = false;
    const currentCandidate = (itemId: string) => {
      if (closed || !dialog.open || this.#state.busy || this.#state.commandBlocked || this.#state.worldMap) return undefined;
      const candidate = visibleEntries().find(entry => entry.id === itemId);
      if (!candidate || !candidate.source || this.#state.status !== snapshot || itemSelectionSource(this.#state, itemId) !== candidate.source) {
        this.#announce("message-item-selection-stale", undefined, "system");
        return undefined;
      }
      return candidate;
    };
    const choose = (itemId: string) => {
      const candidate = currentCandidate(itemId);
      if (!candidate) return;
      if (!this.confirmItemChoice(itemId, command) || !currentCandidate(itemId)) return;
      selected = true;
      dialog.close();
      void onSelect(candidate.id);
    };
    form.addEventListener("submit", (event) => {
      event.preventDefault();
      choose(select.value);
    });
    dialog.addEventListener("keydown", event => {
      if (event.isComposing || event.metaKey || event.altKey) return;
      const target = event.target as HTMLElement | null;
      if (target?.isContentEditable || target?.tagName === "INPUT" || target?.tagName === "TEXTAREA") return;
      if (event.ctrlKey) {
        const shortcuts: Partial<Record<string, ItemSelectionSource>> = { p: "pack", e: "equipment", q: "quiver", f: "floor" };
        const source = shortcuts[event.key.toLowerCase()];
        if (source) {
          event.preventDefault(); event.stopImmediatePropagation();
          if (!event.repeat) selectSource(source);
        }
        return;
      }
      if (event.repeat && (/^[a-z0-9]$/i.test(event.key) || ["Enter", "Escape", "PageDown", "PageUp", " ", "/", "\\", "-", "@"].includes(event.key))) {
        event.preventDefault(); event.stopImmediatePropagation(); return;
      }
      const exactLabel = labels.indexOf(event.key);
      if (exactLabel >= 0) {
        event.preventDefault(); event.stopImmediatePropagation();
        choose(visibleEntries()[exactLabel]!.id); return;
      }
      if (event.key === "@") {
        event.preventDefault(); event.stopImmediatePropagation(); toggleInscriptions(); return;
      }
      if (event.key === "Escape") {
        event.preventDefault(); event.stopImmediatePropagation(); dialog.close(); return;
      }
      if (event.key === "/" || event.key === "\\") {
        event.preventDefault(); event.stopImmediatePropagation();
        const index = (tabs.indexOf(activeTab) + (event.key === "/" ? 1 : -1) + tabs.length) % tabs.length;
        selectSource(tabs[index]!.source); return;
      }
      if (event.key === "-") {
        event.preventDefault(); event.stopImmediatePropagation();
        selectSource("floor");
        if (activeTab.source === "floor" && activeTab.entries.length === 1 && titleKey !== "shortcut-inscribe" && titleKey !== "shortcut-uninscribe") {
          choose(activeTab.entries[0]!.id);
        }
        return;
      }
      if (["PageDown", "PageUp", "3", "9"].includes(event.key) || (event.key === " " && target?.tagName !== "BUTTON")) {
        event.preventDefault(); event.stopImmediatePropagation();
        turnPage(event.key === "PageUp" || event.key === "9" ? -1 : 1); return;
      }
      if (event.key === "Enter" && target?.tagName !== "BUTTON") {
        event.preventDefault(); event.stopImmediatePropagation(); choose(select.value); return;
      }
      if (!/^[a-z0-9]$/i.test(event.key)) return;
      event.preventDefault(); event.stopImmediatePropagation();
      if (!/^[A-Z]$/.test(event.key)) return;
      const entry = visibleEntries()[labels.indexOf(event.key.toLowerCase())];
      if (!entry) return;
      const candidate = currentCandidate(entry.id);
      if (!candidate) return;
      select.value = candidate.id;
      const item = [...this.#state.inventory, ...this.#state.equipment, ...(this.#state.status?.items ?? [])].find(item => item.id === candidate.id);
      if (!item) return;
      details.replaceChildren();
      const heading = document.createElement("h3");
      heading.textContent = candidate.label;
      details.append(heading);
      if ("identification" in item) this.#appendItemDetails(details, item);
      else this.#appendInscription(details, item.inscription);
      details.hidden = false;
    }, true);
    const closeSilently = () => { selected = true; dialog.close(); };
    this.#closeItemSelection = closeSilently;
    dialog.addEventListener("close", () => {
      if (closed) return;
      closed = true;
      dialog.remove();
      if (this.#closeItemSelection === closeSilently) this.#closeItemSelection = undefined;
      if (!selected && this.#state.status === snapshot) void onCancel?.();
    }, { once: true });
    renderPage();
    dialog.append(form);
    document.body.append(dialog);
    dialog.showModal();
    select.focus();
  }

  #selectOptionFrom(
    candidates: Array<{ id: string; label: string }>,
    onSelect: (slotId: string) => Promise<void>,
    titleKey: MessageKey = "equipment-slot-target-title",
    labelKey: MessageKey = "equipment-slot-target-label",
  ): void {
    this.#closeItemSelection?.();
    const snapshot = this.#state.status;
    this.#selectionSnapshot = snapshot;
    if (candidates.length === 0) {
      this.#announce("message-target-mode-unavailable", undefined, "system");
      return;
    }
    const document = this.#dom.inventoryList.ownerDocument;
    const dialog = document.createElement("dialog");
    dialog.className = "item-target-dialog";
    const form = document.createElement("form");
    form.method = "dialog";
    const title = document.createElement("h2");
    title.textContent = this.#localization.format(titleKey);
    const label = document.createElement("label");
    const labelText = document.createElement("span");
    labelText.textContent = this.#localization.format(labelKey);
    const select = document.createElement("select");
    for (const candidate of candidates) {
      const option = document.createElement("option");
      option.value = candidate.id;
      option.textContent = candidate.label;
      select.append(option);
    }
    label.append(labelText, select);
    const actions = document.createElement("div");
    actions.className = "item-target-actions";
    const cancel = document.createElement("button");
    cancel.type = "button";
    cancel.textContent = this.#localization.format("action-dialog-cancel");
    cancel.addEventListener("click", () => dialog.close());
    const confirm = document.createElement("button");
    confirm.type = "submit";
    confirm.textContent = this.#localization.format("action-item-target-confirm");
    actions.append(cancel, confirm);
    form.append(title, label, actions);
    form.addEventListener("submit", (event) => {
      event.preventDefault();
      const slotId = select.value;
      if (!dialog.open || this.#state.busy || this.#state.commandBlocked || snapshot !== this.#state.status || !candidates.some(slot => slot.id === slotId)) return;
      dialog.close();
      void onSelect(slotId);
    });
    const close = () => dialog.close();
    this.#closeItemSelection = close;
    dialog.addEventListener("close", () => {
      dialog.remove();
      if (this.#closeItemSelection === close) this.#closeItemSelection = undefined;
    }, { once: true });
    dialog.append(form);
    document.body.append(dialog);
    dialog.showModal();
    select.focus();
  }

  #selectGlyphTarget(onSelect: (glyph: string) => Promise<void>): void {
    this.#closeItemSelection?.();
    const snapshot = this.#state.status;
    this.#selectionSnapshot = snapshot;
    const document = this.#dom.inventoryList.ownerDocument;
    const dialog = document.createElement("dialog");
    dialog.className = "item-target-dialog";
    const form = document.createElement("form");
    form.method = "dialog";
    const title = document.createElement("h2");
    title.textContent = this.#localization.format("item-use-glyph-title");
    const label = document.createElement("label");
    const labelText = document.createElement("span");
    labelText.textContent = this.#localization.format("item-use-glyph-label");
    const input = document.createElement("input");
    input.type = "text";
    input.autocomplete = "off";
    input.spellcheck = false;
    label.append(labelText, input);
    const actions = document.createElement("div");
    actions.className = "item-target-actions";
    const cancel = document.createElement("button");
    cancel.type = "button";
    cancel.textContent = this.#localization.format("action-dialog-cancel");
    cancel.addEventListener("click", () => dialog.close());
    const confirm = document.createElement("button");
    confirm.type = "submit";
    confirm.textContent = this.#localization.format("item-use-glyph-confirm");
    actions.append(cancel, confirm);
    form.append(title, label, actions);
    form.addEventListener("submit", (event) => {
      event.preventDefault();
      const characters = [...input.value];
      if (!dialog.open || this.#state.busy || this.#state.commandBlocked || snapshot !== this.#state.status) return;
      if (characters.length !== 1 || /\p{Cc}/u.test(characters[0] ?? "")) {
        input.setCustomValidity(this.#localization.format("item-use-glyph-invalid"));
        input.reportValidity();
        return;
      }
      input.setCustomValidity("");
      dialog.close();
      void onSelect(characters[0] ?? "");
    });
    const close = () => dialog.close();
    this.#closeItemSelection = close;
    dialog.addEventListener("close", () => {
      dialog.remove();
      if (this.#closeItemSelection === close) this.#closeItemSelection = undefined;
    }, { once: true });
    dialog.append(form);
    document.body.append(dialog);
    dialog.showModal();
    input.focus();
  }

  #appendItemModifiers(container: HTMLElement, modifiers: StatModifiersDto): void {
    const entries: Array<[MessageKey, number]> = [
      ["item-modifier-attack", modifiers.attack],
      ["item-modifier-defense", modifiers.defense],
      ["item-modifier-max-hp", modifiers.maxHp],
      ["item-modifier-speed", modifiers.speed],
    ];
    this.#appendSignedEntries(container, entries);
  }

  #appendItemEnchantments(
    container: HTMLElement,
    enchantments: ItemEnchantmentsDto | undefined,
  ): void {
    if (!enchantments) return;
    this.#appendSignedEntries(container, [
      ["item-enchantment-to-hit", enchantments.toHit],
      ["item-enchantment-to-damage", enchantments.toDamage],
      ["item-enchantment-to-armor", enchantments.toArmor],
    ]);
  }

  #appendItemCurse(
    container: HTMLElement,
    curse: ItemCurseSeverityDto | null | undefined,
  ): void {
    if (!curse) return;
    this.#appendDetail(container, "item-modifier", this.#itemCurseSeverityName(curse));
  }

  #appendEquipmentBonuses(
    container: HTMLElement,
    bonuses: EquipmentBonusesDto | undefined,
  ): void {
    if (!bonuses) return;
    this.#appendSignedEntries(container, [
      ["item-bonus-melee-attacks", bonuses.meleeAttacks + (bonuses.meleeAttacksDeltaPercent ?? 0) / 100],
      ["item-bonus-weapon-dice", bonuses.weaponDiceBonus ?? 0],
      ["item-bonus-spell-capacity", (bonuses.spellCapacityBonus ?? 0) * 5],
      ["item-bonus-magic-resistance", bonuses.magicResistancePercent ?? 0],
      ["item-bonus-melee-skill", bonuses.meleeSkill],
      ["item-bonus-ranged-skill", bonuses.rangedSkill],
      ["item-bonus-throwing-skill", bonuses.throwingSkill],
      ["item-bonus-device-skill", bonuses.deviceSkill],
      ["item-bonus-saving-throw-skill", bonuses.savingThrowSkill],
      ["item-bonus-stealth-skill", bonuses.stealthSkill],
      ["item-bonus-search-skill", bonuses.searchSkill],
      ["item-bonus-perception-skill", bonuses.perceptionSkill],
      ["item-bonus-disarming-skill", bonuses.disarmingSkill],
      ["item-bonus-digging-skill", bonuses.diggingSkill],
      ["item-bonus-infravision", bonuses.infravision],
      ["item-bonus-light-radius", bonuses.lightRadius],
    ]);
  }

  #appendSignedEntries(container: HTMLElement, entries: Array<[MessageKey, number]>): void {
    for (const [key, value] of entries) {
      if (value === 0) continue;
      this.#appendDetail(
        container,
        "item-modifier",
        this.#localization.format(key, { value: signedModifier(value) }),
      );
    }
  }

  #appendEquipmentPassives(
    container: HTMLElement,
    passives: EquipmentPassiveDto[] | undefined,
  ): void {
    for (const passive of passives ?? []) {
      this.#appendDetail(
        container,
        "item-modifier",
        this.#localization.format(`item-passive-${passive}` as MessageKey),
      );
    }
  }

  #appendItemDefenses(
    container: HTMLElement,
    resistances: ResistanceDto[] | undefined,
    statusImmunities: string[] | undefined,
  ): void {
    const levelKeys: Partial<Record<ResistanceLevelDto, MessageKey>> = {
      vulnerable: "resistance-level-vulnerable",
      resistant: "resistance-level-resistant",
      strong: "resistance-level-strong",
      immune: "resistance-level-immune",
    };
    for (const resistance of resistances ?? []) {
      const levelKey = levelKeys[resistance.level];
      if (!levelKey) continue;
      this.#appendDetail(
        container,
        "item-modifier",
        this.#localization.format("item-resistance-label", {
          type: this.#formatter.damageTypeName(resistance.damageType),
          level: this.#localization.format(levelKey),
        }),
      );
    }
    for (const statusId of statusImmunities ?? []) {
      this.#appendDetail(
        container,
        "item-modifier",
        this.#localization.format("item-status-immunity-label", {
          status: this.#formatter.statusName(statusId),
        }),
      );
    }
  }

  #appendDestructionImmunities(
    container: HTMLElement,
    elements: ItemDestructionElementDto[] | undefined,
  ): void {
    for (const element of elements ?? []) {
      this.#appendDetail(
        container,
        "item-modifier",
        this.#localization.format("item-destruction-immunity-label", {
          type: this.#formatter.damageTypeName(element),
        }),
      );
    }
  }

  #appendItemOffense(
    container: HTMLElement,
    slays: SlayDto[] | undefined,
    brands: WeaponBrandDto[] | undefined,
  ): void {
    for (const slay of slays ?? []) {
      this.#appendDetail(
        container,
        "item-modifier",
        this.#localization.format(
          slay.level === "kill" ? "item-kill-label" : "item-slay-label",
          { target: this.#slayTargetName(slay.target) },
        ),
      );
    }
    for (const brand of brands ?? []) {
      this.#appendDetail(
        container,
        "item-modifier",
        this.#localization.format("item-brand-label", {
          brand: this.#weaponBrandName(brand),
        }),
      );
    }
  }

  #appendKnownItemProperties(
    container: HTMLElement,
    properties: ItemPropertyDto[] | undefined,
  ): void {
    for (const property of properties ?? []) {
      this.#appendDetail(
        container,
        "item-property",
        this.#localization.format("item-property-label", {
          property: this.#formatter.itemPropertyName(property.nameKey),
        }),
      );
    }
  }

  #appendItemQuality(container: HTMLElement, quality: InventoryItemDto["quality"]): void {
    if (!quality) return;
    this.#appendDetail(
      container,
      "item-quality",
      this.#localization.format("item-quality-label", {
        quality: this.#formatter.itemQualityName(quality),
      }),
    );
  }

  #appendDetail(container: HTMLElement, className: string, text: string): void {
    const label = container.ownerDocument.createElement("span");
    label.className = className;
    label.textContent = text;
    container.append(label);
  }

  #slayTargetName(target: SlayTargetDto): string {
    return this.#localization.format(`slay-target-${target}-name` as MessageKey);
  }

  #weaponBrandName(brand: WeaponBrandDto): string {
    return this.#localization.format(`weapon-brand-${brand}-name` as MessageKey);
  }
}

export type InventoryFilter = "all" | "equippable" | "usable" | "devices" | "light";

export function itemFitsBodySlot(item: InventoryItemDto, slot: BodySlotDto): boolean {
  return item.equipmentSlot === slot.slotType || (item.equipmentSlot === "tool" && slot.slotType === "weapon") || (item.equipmentSlot === "weapon" && slot.slotType === "shield");
}

export function filterInventoryItems(
  inventory: readonly InventoryItemDto[],
  filter: InventoryFilter,
  search: string,
  visibleName: (item: InventoryItemDto) => string,
): InventoryItemDto[] {
  const query = search.trim().toLocaleLowerCase();
  return inventory.filter((item) => {
    let matches: boolean;
    switch (filter) {
      case "all": matches = true; break;
      case "equippable": matches = Boolean(item.equipmentSlot); break;
      case "usable": matches = item.usable; break;
      case "devices": matches = Boolean(item.charges || item.canReceiveRecharge || item.canSupplyRecharge); break;
      case "light": matches = item.equipmentSlot === "light" || item.fuel?.kind === "torch" || item.fuel?.kind === "lantern"; break;
    }
    return matches && (query.length === 0 || [visibleName(item), item.inscription ?? ""].some(
      (text) => text.toLocaleLowerCase().includes(query),
    ));
  });
}

export function createItemCurseSeverityName(
  localization: Localization,
): (curse: ItemCurseSeverityDto | string | undefined) => string {
  return (curse) => {
    switch (curse) {
      case "normal":
        return localization.format("item-curse-normal");
      case "heavy":
        return localization.format("item-curse-heavy");
      case "permanent":
        return localization.format("item-curse-permanent");
      default:
        return curse ?? "?";
    }
  };
}

export function selectedRechargingItems(
  selected: InventoryItemDto[],
): { item: InventoryItemDto; source: InventoryItemDto } | undefined {
  if (selected.length !== 2) return undefined;
  const item = selected.find((candidate) => candidate.requiresRechargeTargets);
  const source = selected.find(
    (candidate) => candidate.id !== item?.id && candidate.canSupplyRecharge,
  );
  return item && source ? { item, source } : undefined;
}

export type ItemSelectionSource = "pack" | "equipment" | "quiver" | "floor";

// Source classification does not grant selection eligibility; each caller keeps its filter.
export function itemSelectionSource(
  state: Pick<AppState, "inventory" | "equipment" | "status">,
  itemId: string,
): ItemSelectionSource | undefined {
  if (state.inventory.some(item => item.id === itemId)) {
    return state.status?.player.quiverItemIds?.includes(itemId) ? "quiver" : "pack";
  }
  if (state.equipment.some(item => item.id === itemId)) return "equipment";
  if (state.status?.items.some(item => item.id === itemId)) return "floor";
  return undefined;
}

export function itemTargetCandidates(
  state: Pick<AppState, "inventory" | "equipment" | "status">,
  excludedItemId: string | undefined,
  visibleItemName: (displayNameKey: string, kindId: string, artifactName?: string | null) => string,
): Array<{ id: string; label: string }> {
  const playerPosition = state.status?.player.position;
  const groundItems = playerPosition
    ? state.status?.items.filter(
        (item) =>
          item.position.x === playerPosition.x && item.position.y === playerPosition.y,
      ) ?? []
    : [];
  return [...state.inventory, ...state.equipment, ...groundItems]
    .filter((item) => item.id !== excludedItemId)
    .map((item) => ({
      id: item.id,
      label: visibleItemName(item.displayNameKey, item.kindId, item.artifactName),
    }));
}

export function absorbableItemCandidates(
  state: Pick<AppState, "inventory" | "status">,
  visibleItemName: (displayNameKey: string, kindId: string, artifactName?: string | null) => string,
): Array<{ id: string; label: string }> {
  const playerPosition = state.status?.player.position;
  const groundItems = playerPosition
    ? state.status?.items.filter(
        (item) =>
          item.absorbable &&
          item.position.x === playerPosition.x &&
          item.position.y === playerPosition.y,
      ) ?? []
    : [];
  return [...state.inventory.filter((item) => item.absorbable), ...groundItems].map((item) => ({
    id: item.id,
    label: visibleItemName(item.displayNameKey, item.kindId, item.artifactName),
  }));
}

export function parseDropQuantity(value: string, itemQuantity: number): number | undefined {
  const quantity = Number(value);
  return Number.isSafeInteger(quantity) && quantity >= 1 && quantity <= itemQuantity
    ? quantity
    : undefined;
}

export function formatTenthsPound(value: number): string {
  return `${Math.trunc(value / 10)}.${Math.abs(value % 10)}`;
}

export function itemIdentificationMessageKey(
  identification: InventoryItemDto["identification"],
): MessageKey | undefined {
  if (identification === "unexamined") return "item-identification-unexamined";
  if (identification === "appraised") return "item-identification-appraised";
  return undefined;
}

export function formatTenthsPoundArgument(value: string | undefined): string {
  if (value === undefined) return "?";
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) && parsed >= 0 ? formatTenthsPound(parsed) : "?";
}

function signedModifier(value: number): string {
  return value > 0 ? `+${value}` : String(value);
}
