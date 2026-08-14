// SPDX-License-Identifier: MPL-2.0

import type { AppDom } from "./app-dom";
import type { AppState, TargetingIntent } from "./app-state";
import type { Localization, MessageKey } from "./localization";
import type {
  BodySlotDto,
  EquipmentBonusesDto,
  EquipmentItemDto,
  EquipmentPassiveDto,
  GameCommand,
  InventoryItemDto,
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
  | "inventorySelectionCount"
  | "inventoryUse"
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
  visibleItemName(displayNameKey: string, kindId: string): string;
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
  readonly #updateCampaignAction: () => void;
  readonly #announce: (
    key: MessageKey,
    args: Record<string, string | number> | undefined,
    kind: string,
  ) => void;
  readonly #itemCurseSeverityName: (
    curse: ItemCurseSeverityDto | string | undefined,
  ) => string;
  #installed = false;

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
    updateCampaignAction: () => void;
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
    this.#updateCampaignAction = options.updateCampaignAction;
    this.#announce = options.announce;
    this.#itemCurseSeverityName = options.itemCurseSeverityName;
  }

  install(): void {
    if (this.#installed) return;
    this.#installed = true;
    this.#dom.inventoryUse.addEventListener("click", this.#handleUse);
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
    this.#installed = false;
    this.#dom.inventoryUse.removeEventListener("click", this.#handleUse);
    this.#dom.inventoryUseOnMount.removeEventListener("click", this.#handleUseOnMount);
    this.#dom.inventoryAppraise.removeEventListener("click", this.#handleAppraise);
    this.#dom.inventoryEquip.removeEventListener("click", this.#handleEquip);
    this.#dom.inventoryDrop.removeEventListener("click", this.#handleDrop);
    this.#dom.inventoryInscribe.removeEventListener("click", this.#handleInscribe);
    this.#dom.inventoryDestroy.removeEventListener("click", this.#handleDestroy);
    this.#dom.inventoryDropQuantity.removeEventListener("input", this.#handleQuantityInput);
  }

  render(inventory: InventoryItemDto[], equipment: EquipmentItemDto[]): void {
    this.#state.inventory = inventory.map((item) => ({ ...item }));
    this.#state.equipment = equipment.map((item) => ({ ...item }));
    const availableIds = new Set(inventory.map((item) => item.id));
    for (const itemId of this.#state.selectedInventoryIds) {
      if (!availableIds.has(itemId)) this.#state.selectedInventoryIds.delete(itemId);
    }
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
    this.updateActions();
  }

  updateActions(): void {
    this.#updateCampaignAction();
    const worldMap = this.#state.worldMap;
    const selected = this.#selectedItems();
    this.#dom.inventorySelectionCount.textContent = this.#localization.format(
      "inventory-selected-count",
      { count: selected.length },
    );
    this.#dom.inventoryEquip.disabled =
      this.#state.busy ||
      this.#state.playerDead ||
      worldMap ||
      selected.length !== 1 ||
      !selected[0]?.equipmentSlot;
    this.#dom.inventoryUse.disabled =
      this.#state.busy ||
      this.#state.playerDead ||
      worldMap ||
      !(
        (selected.length === 1 &&
          selected[0]?.usable &&
          !selected[0].requiresRechargeTargets) ||
        selectedRechargingItems(selected)
      );
    this.#dom.inventoryUseOnMount.disabled =
      this.#state.busy ||
      this.#state.playerDead ||
      worldMap ||
      selected.length !== 1 ||
      !selected[0]?.mountUsable ||
      !this.#state.status?.player.ridingActorId;
    this.#dom.inventoryAppraise.disabled =
      this.#state.busy ||
      this.#state.playerDead ||
      worldMap ||
      selected.length !== 1 ||
      selected[0]?.identification !== "unexamined";
    const [item] = selected;
    if (selected.length === 1 && item) {
      if (this.#state.dropQuantityItemId !== item.id) {
        this.#state.dropQuantityItemId = item.id;
        this.#dom.inventoryDropQuantity.value = String(item.quantity);
      }
      if (
        this.#dom.inventoryInscription.ownerDocument.activeElement !==
        this.#dom.inventoryInscription
      ) {
        this.#dom.inventoryInscription.value = item.inscription ?? "";
      }
      this.#dom.inventoryDropQuantity.min = "1";
      this.#dom.inventoryDropQuantity.max = String(item.quantity);
      this.#dom.inventoryDropQuantity.disabled =
        this.#state.busy || this.#state.playerDead || worldMap;
      this.#dom.inventoryDrop.disabled =
        this.#state.busy ||
        this.#state.playerDead ||
        worldMap ||
        parseDropQuantity(this.#dom.inventoryDropQuantity.value, item.quantity) === undefined;
      this.#dom.inventoryInscription.disabled =
        this.#state.busy || this.#state.playerDead || worldMap;
      this.#dom.inventoryInscribe.disabled =
        this.#state.busy || this.#state.playerDead || worldMap;
      this.#dom.inventoryDestroy.disabled = this.#dom.inventoryDrop.disabled;
    } else {
      this.#state.dropQuantityItemId = undefined;
      this.#dom.inventoryDropQuantity.value = "";
      this.#dom.inventoryDropQuantity.disabled = true;
      this.#dom.inventoryDrop.disabled =
        this.#state.busy || this.#state.playerDead || worldMap || selected.length === 0;
      this.#dom.inventoryInscription.value = "";
      this.#dom.inventoryInscription.disabled = true;
      this.#dom.inventoryInscribe.disabled = true;
      this.#dom.inventoryDestroy.disabled = true;
    }
    for (const checkbox of this.#dom.inventoryList.querySelectorAll<HTMLInputElement>(
      'input[type="checkbox"]',
    )) {
      checkbox.disabled = this.#state.busy || this.#state.playerDead || worldMap;
    }
    for (const button of this.#dom.equipmentList.querySelectorAll<HTMLButtonElement>("button")) {
      const refuelTargetId = button.dataset.refuelTargetId;
      button.disabled =
        this.#state.busy ||
        this.#state.playerDead ||
        worldMap ||
        (refuelTargetId !== undefined && this.#refuelSourceForTarget(refuelTargetId) === undefined);
    }
  }

  selectItemTarget(
    excludedItemId: string | undefined,
    onSelect: (itemId: string) => Promise<void>,
  ): void {
    const candidates = itemTargetCandidates(
      this.#state,
      excludedItemId,
      (displayNameKey, kindId) => this.#formatter.visibleItemName(displayNameKey, kindId),
    );
    this.#selectItemTargetFrom(candidates, onSelect);
  }

  readonly #handleUse = (): void => {
    void this.#useSelectedItem();
  };

  readonly #handleUseOnMount = (): void => {
    void this.#useSelectedItemOnMount();
  };

  readonly #handleAppraise = (): void => {
    void this.#appraiseSelectedItem();
  };

  readonly #handleEquip = (): void => {
    void this.#equipSelectedItem();
  };

  readonly #handleDrop = (): void => {
    void this.#dropSelectedItems();
  };

  readonly #handleInscribe = (): void => {
    void this.#inscribeSelectedItem();
  };

  readonly #handleDestroy = (): void => {
    void this.#destroySelectedItem();
  };

  readonly #handleQuantityInput = (): void => {
    this.updateActions();
  };

  #renderInventoryItems(inventory: InventoryItemDto[]): void {
    const document = this.#dom.inventoryList.ownerDocument;
    this.#dom.inventoryList.replaceChildren();
    if (inventory.length === 0) {
      const empty = document.createElement("li");
      empty.className = "inventory-empty";
      empty.textContent = this.#localization.format("inventory-empty");
      this.#dom.inventoryList.append(empty);
      return;
    }
    for (const item of inventory) {
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
      const details = document.createElement("span");
      details.className = "inventory-item-details";
      this.#appendItemDetails(details, item);
      const quantity = document.createElement("span");
      quantity.className = "inventory-quantity";
      quantity.textContent = this.#localization.format("inventory-quantity", {
        quantity: item.quantity,
      });
      label.append(checkbox, details, quantity);
      row.append(label);
      this.#dom.inventoryList.append(row);
    }
  }

  #renderEquipment(equipment: EquipmentItemDto[]): void {
    const document = this.#dom.equipmentList.ownerDocument;
    this.#dom.equipmentList.replaceChildren();
    const slots: BodySlotDto[] =
      this.#state.bodySlots.length > 0
        ? this.#state.bodySlots
        : equipment.map((item) => ({ id: item.slotId, slotType: item.slotId }));
    if (slots.length === 0) {
      const empty = document.createElement("li");
      empty.className = "equipment-empty";
      empty.textContent = this.#localization.format("equipment-empty");
      this.#dom.equipmentList.append(empty);
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
      const details = document.createElement("span");
      details.className = "equipment-item-details";
      const slotTag = document.createElement("span");
      slotTag.className = "equipment-slot";
      slotTag.textContent = slotLabel;
      if (item) {
        row.className = "equipment-item";
        const name = document.createElement("span");
        name.textContent = this.#itemName(item);
        details.append(name, slotTag);
        this.#appendInscription(details, item.inscription);
        this.#appendItemFuel(details, item);
        this.#appendKnownDetails(details, item);
        if (item.usable || item.activation || (item.captureBall && item.useTargetSpec)) {
          const activate = document.createElement("button");
          activate.type = "button";
          activate.className = "equipment-activate";
          activate.textContent = this.#localization.format("action-equipment-activate");
          activate.disabled = this.#state.busy || (Boolean(item.activation) && !item.usable);
          activate.addEventListener("click", () => {
            if (item.useTargetSpec?.modes.includes("self")) {
              void this.#dispatch({ type: "use-item", itemId: item.id, target: { type: "self" } });
            } else if (item.useTargetSpec) {
              this.#startTargeting(item.useTargetSpec, { type: "item", itemId: item.id });
            } else {
              void this.#dispatch({ type: "use-item", itemId: item.id });
            }
          });
          row.append(activate);
        }
        if (item.fuel && item.fuel.kind !== "oil" && item.fuel.current < item.fuel.maximum) {
          const refuel = document.createElement("button");
          refuel.type = "button";
          refuel.className = "equipment-refuel";
          refuel.dataset.refuelTargetId = item.id;
          refuel.textContent = this.#localization.format("action-equipment-refuel");
          refuel.disabled = this.#refuelSourceForTarget(item.id) === undefined;
          refuel.addEventListener("click", () => {
            const source = this.#refuelSourceForTarget(item.id);
            if (!source) return;
            void this.#dispatch({
              type: "refuel-light",
              targetItemId: item.id,
              sourceItemId: source.id,
            });
          });
          row.append(refuel);
        }
        const unequip = document.createElement("button");
        unequip.type = "button";
        unequip.textContent = this.#localization.format("action-equipment-unequip");
        unequip.disabled = this.#state.busy;
        unequip.addEventListener("click", () => void this.#unequipItem(item.slotId));
        row.append(details, unequip);
      } else {
        row.className = "equipment-item equipment-slot-vacant";
        const vacant = document.createElement("span");
        vacant.className = "equipment-vacant-label";
        vacant.textContent = this.#localization.format("equipment-slot-vacant");
        details.append(slotTag, vacant);
        row.append(details);
      }
      this.#dom.equipmentList.append(row);
    }
  }

  #appendItemDetails(container: HTMLElement, item: InventoryItemDto): void {
    const document = container.ownerDocument;
    const name = document.createElement("span");
    name.className = "inventory-item-name";
    name.textContent = this.#itemName(item);
    container.append(name);
    this.#appendInscription(container, item.inscription);
    if (item.equipmentSlot) {
      const equippable = document.createElement("span");
      equippable.className = "inventory-equippable";
      equippable.textContent = this.#localization.format("inventory-equippable", {
        slot: this.#formatter.equipmentSlotName(item.equipmentSlot),
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
    const target = this.#state.equipment.find((item) => item.id === targetItemId);
    if (!target?.fuel || target.fuel.current >= target.fuel.maximum) return undefined;
    return this.#state.inventory.find((item) => {
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
            actor: this.#localization.format(item.capturedActor.nameKey as MessageKey),
            hp: item.capturedActor.hp,
            maximum: item.capturedActor.maxHp,
            experience: item.capturedActor.experience,
          })
        : this.#localization.format("capture-ball-empty");
      container.append(captured);
    }
    if ("slotId" in item || item.equipmentSlot !== null) {
      const identification = container.ownerDocument.createElement("span");
      identification.className = `item-identification item-identification-${item.identification}`;
      identification.textContent = this.#localization.format(
        itemIdentificationMessageKey(item.identification, item.knownProperties?.length ?? 0),
        { count: item.knownProperties?.length ?? 0 },
      );
      container.append(identification);
    }
    this.#appendItemModifiers(container, item.modifiers);
    this.#appendItemEnchantments(container, item.enchantments);
    this.#appendItemCurse(container, item.curse);
    this.#appendEquipmentBonuses(container, item.equipmentBonuses);
    this.#appendItemDefenses(container, item.resistances, item.statusImmunities);
    this.#appendItemOffense(container, item.slays, item.brands);
    this.#appendEquipmentPassives(container, item.passives);
    this.#appendItemQuality(container, item.quality);
    this.#appendKnownItemProperties(container, item.knownProperties);
  }

  #itemName(item: InventoryItemDto | EquipmentItemDto): string {
    const ball = this.#formatter.visibleItemName(item.displayNameKey, item.kindId);
    return item.capturedActor
      ? this.#localization.format("capture-ball-name-contained", {
          ball,
          actor: this.#localization.format(item.capturedActor.nameKey as MessageKey),
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
    this.#selectEquipmentSlotFrom(candidates, (slotId) =>
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

  async #useSelectedItem(): Promise<void> {
    const selected = this.#selectedItems();
    if (this.#state.busy) return;
    const recharge = selectedRechargingItems(selected);
    if (recharge) {
      this.#selectRechargeTarget(recharge.item.id, recharge.source.id);
      return;
    }
    if (selected.length !== 1 || !selected[0]?.usable) return;
    const item = selected[0];
    if (item.requiresRechargeTargets) return;
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
    if (this.#state.busy) return;
    await this.#dispatch({ type: "unequip", slotId });
  }

  #selectedItems(): InventoryItemDto[] {
    return this.#state.inventory.filter((item) => this.#state.selectedInventoryIds.has(item.id));
  }

  #selectRechargeTarget(itemId: string, sourceItemId: string): void {
    const candidates = this.#state.inventory
      .filter(
        (item) => item.id !== itemId && item.id !== sourceItemId && item.canReceiveRecharge,
      )
      .map((item) => ({
        id: item.id,
        label: this.#formatter.visibleItemName(item.displayNameKey, item.kindId),
      }));
    this.#selectItemTargetFrom(candidates, (targetItemId) =>
      this.#dispatch({
        type: "use-item-for-recharge",
        itemId,
        sourceItemId,
        targetItemId,
      }),
    );
  }

  #selectItemTargetFrom(
    candidates: Array<{ id: string; label: string }>,
    onSelect: (itemId: string) => Promise<void>,
  ): void {
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
    title.textContent = this.#localization.format("item-target-title");
    const label = document.createElement("label");
    const labelText = document.createElement("span");
    labelText.textContent = this.#localization.format("item-target-label");
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
      const itemId = select.value;
      dialog.close();
      void onSelect(itemId);
    });
    dialog.addEventListener("close", () => dialog.remove(), { once: true });
    dialog.append(form);
    document.body.append(dialog);
    dialog.showModal();
    select.focus();
  }

  #selectEquipmentSlotFrom(
    candidates: Array<{ id: string; label: string }>,
    onSelect: (slotId: string) => Promise<void>,
  ): void {
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
    title.textContent = this.#localization.format("equipment-slot-target-title");
    const label = document.createElement("label");
    const labelText = document.createElement("span");
    labelText.textContent = this.#localization.format("equipment-slot-target-label");
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
      dialog.close();
      void onSelect(slotId);
    });
    dialog.addEventListener("close", () => dialog.remove(), { once: true });
    dialog.append(form);
    document.body.append(dialog);
    dialog.showModal();
    select.focus();
  }

  #selectGlyphTarget(onSelect: (glyph: string) => Promise<void>): void {
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
      if (characters.length !== 1 || /\p{Cc}/u.test(characters[0] ?? "")) {
        input.setCustomValidity(this.#localization.format("item-use-glyph-invalid"));
        input.reportValidity();
        return;
      }
      input.setCustomValidity("");
      dialog.close();
      void onSelect(characters[0] ?? "");
    });
    dialog.addEventListener("close", () => dialog.remove(), { once: true });
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
      ["item-bonus-melee-attacks", bonuses.meleeAttacks],
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

export function itemTargetCandidates(
  state: Pick<AppState, "inventory" | "equipment" | "status">,
  excludedItemId: string | undefined,
  visibleItemName: (displayNameKey: string, kindId: string) => string,
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
      label: visibleItemName(item.displayNameKey, item.kindId),
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
  knownPropertyCount: number,
): MessageKey {
  if (identification === "unexamined") return "item-identification-unexamined";
  if (identification === "appraised") return "item-identification-appraised";
  return knownPropertyCount > 0
    ? "item-identification-identified-ego"
    : "item-identification-identified-ordinary";
}

export function formatTenthsPoundArgument(value: string | undefined): string {
  if (value === undefined) return "?";
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) && parsed >= 0 ? formatTenthsPound(parsed) : "?";
}

function signedModifier(value: number): string {
  return value > 0 ? `+${value}` : String(value);
}
