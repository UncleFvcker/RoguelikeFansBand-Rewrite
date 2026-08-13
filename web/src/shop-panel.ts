// SPDX-License-Identifier: MPL-2.0

import type { AppState } from "./app-state.ts";
import { formatTenthsPound } from "./inventory-panel.ts";
import type { Localization, MessageKey } from "./localization.ts";
import type {
  GameCommand,
  GameEventDto,
  GameSnapshot,
  GameUpdate,
  InventoryItemDto,
  ShopDto,
  ShopSellQuoteDto,
  ShopStockItemDto,
} from "./protocol.ts";

export type ShopMode = "buy" | "sell";

export interface ShopTransactionPreview {
  readonly totalPrice: number;
  readonly goldAfter: number;
  readonly weightAfterTenthsPound: number;
}

interface ShopDom {
  readonly dialog: HTMLDialogElement;
  readonly title: HTMLElement;
  readonly description: HTMLElement;
  readonly owner: HTMLElement;
  readonly close: HTMLButtonElement;
  readonly buyTab: HTMLButtonElement;
  readonly sellTab: HTMLButtonElement;
  readonly gold: HTMLElement;
  readonly weight: HTMLElement;
  readonly nutrition: HTMLElement;
  readonly light: HTMLElement;
  readonly list: HTMLUListElement;
  readonly selection: HTMLElement;
  readonly quantity: HTMLInputElement;
  readonly quantityDecrease: HTMLButtonElement;
  readonly quantityIncrease: HTMLButtonElement;
  readonly quantityMaximum: HTMLButtonElement;
  readonly total: HTMLElement;
  readonly goldAfter: HTMLElement;
  readonly weightAfter: HTMLElement;
  readonly confirm: HTMLButtonElement;
  readonly stay: HTMLButtonElement;
  readonly innTravel: HTMLElement;
  readonly innDestination: HTMLSelectElement;
  readonly innTravelConfirm: HTMLButtonElement;
  readonly feedback: HTMLElement;
}

type Feedback =
  | { readonly source: "message"; readonly key: MessageKey; readonly kind: string }
  | { readonly source: "event"; readonly event: GameEventDto; readonly kind: string };

interface ShopSelection {
  readonly id: string;
  readonly kindId: string;
  readonly displayNameKey: string;
  readonly quantity: number;
  readonly inscription?: string;
  readonly capturedActor?: InventoryItemDto["capturedActor"];
  readonly maximumQuantity: number;
  readonly unitPrice: number;
  readonly weightTenthsPound: number;
  readonly unavailableReason?: string;
  readonly fuel?: ShopStockItemDto["fuel"];
}

export class ShopPanel {
  readonly #state: AppState;
  readonly #localization: Localization;
  readonly #dispatch: (command: GameCommand) => Promise<void>;
  readonly #formatEvent: (event: GameEventDto) => string;
  readonly #visibleItemName: (displayNameKey: string, kindId: string) => string;
  readonly #contentName: (id: string | undefined) => string;
  readonly #beforeOpen: () => void;
  readonly #dom: ShopDom;
  #mode: ShopMode = "buy";
  #shop: ShopDto | undefined;
  #selectedItemId: string | undefined;
  #dismissedShopId: string | undefined;
  #feedback: Feedback | undefined;
  #installed = false;

  constructor(options: {
    document: Document;
    state: AppState;
    localization: Localization;
    dispatch: (command: GameCommand) => Promise<void>;
    formatEvent: (event: GameEventDto) => string;
    visibleItemName: (displayNameKey: string, kindId: string) => string;
    contentName: (id: string | undefined) => string;
    beforeOpen: () => void;
  }) {
    this.#state = options.state;
    this.#localization = options.localization;
    this.#dispatch = options.dispatch;
    this.#formatEvent = options.formatEvent;
    this.#visibleItemName = options.visibleItemName;
    this.#contentName = options.contentName;
    this.#beforeOpen = options.beforeOpen;
    this.#dom = createShopDom(options.document);
  }

  install(): void {
    if (this.#installed) return;
    this.#installed = true;
    this.#dom.close.addEventListener("click", this.#close);
    this.#dom.dialog.addEventListener("close", this.#handleClosed);
    this.#dom.buyTab.addEventListener("click", this.#showBuy);
    this.#dom.sellTab.addEventListener("click", this.#showSell);
    this.#dom.list.addEventListener("click", this.#selectItem);
    this.#dom.quantity.addEventListener("input", this.#changeQuantity);
    this.#dom.quantityDecrease.addEventListener("click", this.#decreaseQuantity);
    this.#dom.quantityIncrease.addEventListener("click", this.#increaseQuantity);
    this.#dom.quantityMaximum.addEventListener("click", this.#maximizeQuantity);
    this.#dom.confirm.addEventListener("click", this.#confirmTransaction);
    this.#dom.stay.addEventListener("click", this.#stayAtInn);
    this.#dom.innDestination.addEventListener("change", this.#renderTransaction);
    this.#dom.innTravelConfirm.addEventListener("click", this.#travelFromInn);
  }

  dispose(): void {
    if (!this.#installed) return;
    this.#installed = false;
    this.#dom.close.removeEventListener("click", this.#close);
    this.#dom.dialog.removeEventListener("close", this.#handleClosed);
    this.#dom.buyTab.removeEventListener("click", this.#showBuy);
    this.#dom.sellTab.removeEventListener("click", this.#showSell);
    this.#dom.list.removeEventListener("click", this.#selectItem);
    this.#dom.quantity.removeEventListener("input", this.#changeQuantity);
    this.#dom.quantityDecrease.removeEventListener("click", this.#decreaseQuantity);
    this.#dom.quantityIncrease.removeEventListener("click", this.#increaseQuantity);
    this.#dom.quantityMaximum.removeEventListener("click", this.#maximizeQuantity);
    this.#dom.confirm.removeEventListener("click", this.#confirmTransaction);
    this.#dom.stay.removeEventListener("click", this.#stayAtInn);
    this.#dom.innDestination.removeEventListener("change", this.#renderTransaction);
    this.#dom.innTravelConfirm.removeEventListener("click", this.#travelFromInn);
  }

  render(state: GameSnapshot | GameUpdate): void {
    const transactionEvent = lastShopEvent(state);
    if (transactionEvent) {
      this.#feedback = {
        source: "event",
        event: transactionEvent,
        kind: transactionEvent.kind.endsWith("unavailable") ? "error" : "success",
      };
    }

    const shop = state.shops.find((candidate) => candidate.playerAtEntrance);
    if (!shop) {
      this.#shop = undefined;
      this.#selectedItemId = undefined;
      this.#dismissedShopId = undefined;
      this.#feedback = undefined;
      if (this.#dom.dialog.open) this.#dom.dialog.close();
      return;
    }

    const enteringDifferentShop = this.#shop?.id !== shop.id;
    this.#shop = shop;
    if (enteringDifferentShop) {
      this.#mode = "buy";
      this.#selectedItemId = undefined;
      if (!transactionEvent) this.#feedback = undefined;
    }
    this.#renderPanel();

    if (!this.#dom.dialog.open && this.#dismissedShopId !== shop.id) {
      this.#beforeOpen();
      this.#dom.dialog.showModal();
      this.#focusSelection();
    }
  }

  localize(): void {
    if (this.#shop) this.#renderPanel();
  }

  updateActions(): void {
    if (this.#shop) this.#renderTransaction();
  }

  reset(): void {
    this.#shop = undefined;
    this.#selectedItemId = undefined;
    this.#dismissedShopId = undefined;
    this.#feedback = undefined;
    if (this.#dom.dialog.open) this.#dom.dialog.close();
  }

  readonly #close = (): void => {
    if (this.#dom.dialog.open) this.#dom.dialog.close();
  };

  readonly #handleClosed = (): void => {
    if (this.#shop?.playerAtEntrance) this.#dismissedShopId = this.#shop.id;
  };

  readonly #showBuy = (): void => this.#setMode("buy");
  readonly #showSell = (): void => this.#setMode("sell");

  readonly #selectItem = (event: Event): void => {
    const target = event.target;
    if (!(target instanceof HTMLElement)) return;
    const button = target.closest<HTMLButtonElement>("[data-shop-item-id]");
    if (!button || button.disabled) return;
    this.#selectedItemId = button.dataset.shopItemId;
    this.#dom.quantity.value = "1";
    this.#feedback = undefined;
    this.#renderItems();
    this.#renderTransaction();
  };

  readonly #changeQuantity = (): void => this.#renderTransaction();

  readonly #decreaseQuantity = (): void => {
    this.#stepQuantity(-1);
  };

  readonly #increaseQuantity = (): void => {
    this.#stepQuantity(1);
  };

  readonly #maximizeQuantity = (): void => {
    const selection = this.#selection();
    if (!selection) return;
    this.#dom.quantity.value = String(selection.maximumQuantity);
    this.#renderTransaction();
  };

  readonly #confirmTransaction = (): void => {
    const shop = this.#shop;
    const selection = this.#selection();
    const quantity = selection
      ? parseShopQuantity(this.#dom.quantity.value, selection.maximumQuantity)
      : undefined;
    if (!shop || !selection || quantity === undefined || this.#state.busy) return;

    this.#feedback = {
      source: "message",
      key: "shop-transaction-pending",
      kind: "pending",
    };
    this.#renderTransaction();
    const command: GameCommand =
      this.#mode === "buy"
        ? {
            type: "buy-from-shop",
            shopId: shop.id,
            itemId: selection.id,
            quantity,
          }
        : {
            type: "sell-to-shop",
            shopId: shop.id,
            itemId: selection.id,
            quantity,
          };
    void this.#dispatch(command).then(() => {
      if (this.#feedback?.source === "message" && this.#feedback.kind === "pending") {
        this.#feedback = {
          source: "message",
          key: "shop-transaction-no-response",
          kind: "error",
        };
        this.#renderTransaction();
      }
    });
  };

  readonly #stayAtInn = (): void => {
    const command = stayAtInnCommand(this.#shop);
    if (!command || this.#state.busy) return;
    this.#feedback = {
      source: "message",
      key: "inn-stay-pending",
      kind: "pending",
    };
    this.#renderTransaction();
    void this.#dispatch(command).then(() => {
      if (this.#feedback?.source === "message" && this.#feedback.kind === "pending") {
        this.#feedback = {
          source: "message",
          key: "inn-stay-no-response",
          kind: "error",
        };
        this.#renderTransaction();
      }
    });
  };

  readonly #travelFromInn = (): void => {
    const command = travelFromInnCommand(this.#shop, this.#dom.innDestination.value);
    if (!command || this.#state.busy) return;
    this.#feedback = {
      source: "message",
      key: "inn-travel-pending",
      kind: "pending",
    };
    this.#renderTransaction();
    void this.#dispatch(command).then(() => {
      if (this.#feedback?.source === "message" && this.#feedback.kind === "pending") {
        this.#feedback = {
          source: "message",
          key: "inn-travel-no-response",
          kind: "error",
        };
        this.#renderTransaction();
      }
    });
  };

  #setMode(mode: ShopMode): void {
    if (this.#mode === mode) return;
    this.#mode = mode;
    this.#selectedItemId = undefined;
    this.#dom.quantity.value = "1";
    this.#feedback = undefined;
    this.#renderPanel();
    this.#focusSelection();
  }

  #stepQuantity(delta: number): void {
    const selection = this.#selection();
    if (!selection) return;
    const current = parseShopQuantity(
      this.#dom.quantity.value,
      selection.maximumQuantity,
    );
    const next = Math.max(
      1,
      Math.min(selection.maximumQuantity, (current ?? 1) + delta),
    );
    this.#dom.quantity.value = String(next);
    this.#renderTransaction();
  }

  #renderPanel(): void {
    const shop = this.#shop;
    const status = this.#state.status;
    if (!shop || !status) return;
    this.#dom.title.textContent = this.#localization.format(shop.nameKey);
    this.#dom.description.textContent = this.#localization.format(shop.descriptionKey);
    this.#dom.owner.textContent = this.#localization.format("shop-owner-summary", {
      owner: this.#localization.format(shop.owner.nameKey),
      factor: shop.owner.priceFactorPercent,
    });
    this.#dom.buyTab.setAttribute("aria-selected", String(this.#mode === "buy"));
    this.#dom.sellTab.setAttribute("aria-selected", String(this.#mode === "sell"));
    this.#dom.buyTab.tabIndex = this.#mode === "buy" ? 0 : -1;
    this.#dom.sellTab.tabIndex = this.#mode === "sell" ? 0 : -1;
    this.#dom.gold.textContent = status.player.gold.toLocaleString(this.#localization.locale);
    this.#dom.weight.textContent = this.#localization.format("shop-weight-current", {
      weight: formatTenthsPound(status.player.carriedWeightTenthsPound),
      capacity: formatTenthsPound(status.player.carryCapacityTenthsPound),
    });
    this.#dom.nutrition.textContent = this.#localization.format("status-nutrition-value", {
      state: this.#localization.format(`nutrition-state-${status.player.nutritionState}`),
      percent: Math.floor(status.player.nutrition / 100),
    });
    this.#dom.light.textContent = equippedLightText(
      status.equipment,
      this.#localization,
      this.#contentName,
    );
    const isInn = shop.innStayCost !== undefined;
    this.#dom.stay.hidden = !isInn;
    this.#dom.stay.textContent = this.#localization.format("action-inn-stay", {
      cost: shop.innStayCost ?? 0,
    });
    this.#dom.innTravel.hidden = !isInn;
    const selectedTownId = this.#dom.innDestination.value;
    this.#dom.innDestination.replaceChildren();
    if (shop.innTravelDestinations.length === 0) {
      const option = this.#dom.innDestination.ownerDocument.createElement("option");
      option.textContent = this.#localization.format("inn-travel-no-destinations");
      option.value = "";
      this.#dom.innDestination.append(option);
    } else {
      for (const destination of shop.innTravelDestinations) {
        const option = this.#dom.innDestination.ownerDocument.createElement("option");
        option.value = destination.townId;
        option.textContent = this.#localization.format("inn-travel-destination", {
          town: this.#localization.format(destination.townNameKey),
          cost: destination.cost,
        });
        this.#dom.innDestination.append(option);
      }
      if (shop.innTravelDestinations.some((destination) => destination.townId === selectedTownId)) {
        this.#dom.innDestination.value = selectedTownId;
      }
    }
    this.#dom.innDestination.disabled = shop.innTravelDestinations.length === 0;
    this.#dom.innTravelConfirm.disabled =
      this.#state.busy || shop.innTravelDestinations.length === 0;
    this.#renderItems();
    this.#renderTransaction();
  }

  #renderItems(): void {
    const shop = this.#shop;
    if (!shop) return;
    const selections = this.#selections();
    if (!selections.some((item) => item.id === this.#selectedItemId && item.maximumQuantity > 0)) {
      this.#selectedItemId = selections.find((item) => item.maximumQuantity > 0)?.id;
      this.#dom.quantity.value = "1";
    }

    this.#dom.list.replaceChildren();
    if (selections.length === 0) {
      const empty = this.#dom.list.ownerDocument.createElement("li");
      empty.className = "shop-empty";
      empty.textContent = this.#localization.format(
        this.#mode === "buy" ? "shop-buy-empty" : "shop-sell-empty",
      );
      this.#dom.list.append(empty);
      return;
    }

    for (const selection of selections) {
      const row = this.#dom.list.ownerDocument.createElement("li");
      row.className = "shop-item-row";
      row.dataset.unavailable = String(selection.maximumQuantity === 0);
      const button = this.#dom.list.ownerDocument.createElement("button");
      button.type = "button";
      button.className = "shop-item-select";
      button.dataset.shopItemId = selection.id;
      button.disabled = selection.maximumQuantity === 0;
      button.setAttribute("aria-pressed", String(selection.id === this.#selectedItemId));

      const plainName = this.#itemName(selection);
      const name = this.#span(
        "shop-item-name",
        selection.inscription
          ? this.#localization.format("inventory-inscribed-name", {
              name: plainName,
              inscription: selection.inscription,
            })
          : plainName,
      );
      const details = this.#span(
        "shop-item-details",
        this.#itemDetails(selection),
      );
      const price = this.#span(
        "shop-item-price",
        this.#localization.format("shop-unit-price", { price: selection.unitPrice }),
      );
      const quantity = this.#span(
        "shop-item-stock",
        this.#localization.format(
          this.#mode === "buy" ? "shop-stock-count" : "shop-owned-count",
          { quantity: selection.quantity },
        ),
      );
      button.append(name, details, price, quantity);
      if (selection.unavailableReason) {
        const reason = this.#span(
          "shop-item-unavailable",
          shopTransactionReason(selection.unavailableReason, this.#localization),
        );
        button.append(reason);
      }
      row.append(button);
      this.#dom.list.append(row);
    }
  }

  #renderTransaction(): void {
    const status = this.#state.status;
    const selection = this.#selection();
    const maximum = selection?.maximumQuantity ?? 0;
    const quantity = selection
      ? parseShopQuantity(this.#dom.quantity.value, maximum)
      : undefined;
    const valid = selection !== undefined && quantity !== undefined;

    this.#dom.quantity.min = "1";
    this.#dom.quantity.max = String(maximum);
    this.#dom.quantity.disabled = this.#state.busy || maximum === 0;
    this.#dom.quantityDecrease.disabled =
      this.#state.busy || !valid || (quantity ?? 0) <= 1;
    this.#dom.quantityIncrease.disabled =
      this.#state.busy || !valid || (quantity ?? 0) >= maximum;
    this.#dom.quantityMaximum.disabled = this.#state.busy || maximum <= 1;
    this.#dom.confirm.disabled = this.#state.busy || !valid;
    this.#dom.stay.disabled = this.#state.busy;
    this.#dom.confirm.textContent = this.#localization.format(
      this.#mode === "buy" ? "action-shop-buy" : "action-shop-sell",
    );

    if (!status || !selection || quantity === undefined) {
      this.#dom.selection.textContent = this.#localization.format("shop-selection-none");
      this.#dom.total.textContent = this.#localization.format("value-unavailable");
      this.#dom.goldAfter.textContent = status
        ? status.player.gold.toLocaleString(this.#localization.locale)
        : this.#localization.format("value-unavailable");
      this.#dom.weightAfter.textContent = status
        ? formatTenthsPound(status.player.carriedWeightTenthsPound)
        : this.#localization.format("value-unavailable");
    } else {
      const name = this.#itemName(selection);
      const preview = calculateShopTransactionPreview(
        this.#mode,
        quantity,
        selection.unitPrice,
        selection.weightTenthsPound,
        status.player.gold,
        status.player.carriedWeightTenthsPound,
      );
      this.#dom.selection.textContent = this.#localization.format("shop-selection-summary", {
        item: name,
        maximum: maximum,
      });
      this.#dom.total.textContent = this.#localization.format("shop-total-price", {
        total: preview.totalPrice,
      });
      this.#dom.goldAfter.textContent = preview.goldAfter.toLocaleString(
        this.#localization.locale,
      );
      this.#dom.weightAfter.textContent = this.#localization.format("shop-weight-after", {
        weight: formatTenthsPound(preview.weightAfterTenthsPound),
        capacity: formatTenthsPound(status.player.carryCapacityTenthsPound),
      });
    }

    const feedback = this.#feedback;
    if (feedback) {
      this.#dom.feedback.textContent =
        feedback.source === "event"
          ? this.#formatEvent(feedback.event)
          : this.#localization.format(feedback.key);
    } else {
      this.#dom.feedback.replaceChildren();
    }
    this.#dom.feedback.dataset.kind = feedback?.kind ?? "none";
  }

  #selections(): ShopSelection[] {
    const shop = this.#shop;
    if (!shop) return [];
    if (this.#mode === "buy") {
      return shop.stock.map((item) => ({
        id: item.id,
        kindId: item.kindId,
        displayNameKey: item.displayNameKey,
        quantity: item.quantity,
        inscription: item.inscription ?? undefined,
        capturedActor: item.capturedActor,
        maximumQuantity: item.maximumQuantity,
        unitPrice: item.unitPrice,
        weightTenthsPound: item.weightTenthsPound,
        fuel: item.fuel,
      }));
    }
    const quotes = new Map(shop.sellQuotes.map((quote) => [quote.itemId, quote]));
    return this.#state.inventory.flatMap((item) => {
      const quote = quotes.get(item.id);
      return quote ? [sellSelection(item, quote)] : [];
    });
  }

  #selection(): ShopSelection | undefined {
    return this.#selections().find((item) => item.id === this.#selectedItemId);
  }

  #itemDetails(selection: ShopSelection): string {
    const details = [
      this.#localization.format("shop-item-weight", {
        weight: formatTenthsPound(selection.weightTenthsPound),
      }),
    ];
    if (selection.fuel) {
      details.push(
        this.#localization.format("inventory-fuel", {
          current: selection.fuel.current,
          maximum: selection.fuel.maximum,
        }),
      );
    }
    if (selection.capturedActor) {
      details.push(
        this.#localization.format("capture-ball-contained", {
          actor: this.#localization.format(selection.capturedActor.nameKey as MessageKey),
          hp: selection.capturedActor.hp,
          maximum: selection.capturedActor.maxHp,
          experience: selection.capturedActor.experience,
        }),
      );
    }
    return details.join(" | ");
  }

  #itemName(selection: ShopSelection): string {
    const ball = this.#visibleItemName(selection.displayNameKey, selection.kindId);
    return selection.capturedActor
      ? this.#localization.format("capture-ball-name-contained", {
          ball,
          actor: this.#localization.format(selection.capturedActor.nameKey as MessageKey),
        })
      : ball;
  }

  #span(className: string, text: string): HTMLSpanElement {
    const span = this.#dom.list.ownerDocument.createElement("span");
    span.className = className;
    span.textContent = text;
    return span;
  }

  #focusSelection(): void {
    this.#dom.list
      .querySelector<HTMLButtonElement>("[data-shop-item-id]:not(:disabled)")
      ?.focus();
  }
}

export function parseShopQuantity(value: string, maximum: number): number | undefined {
  const quantity = Number(value);
  return Number.isSafeInteger(quantity) && quantity >= 1 && quantity <= maximum
    ? quantity
    : undefined;
}

export function stayAtInnCommand(
  shop: Pick<ShopDto, "id" | "innStayCost"> | undefined,
): GameCommand | undefined {
  return shop?.innStayCost !== undefined
    ? { type: "stay-at-inn", facilityId: shop.id }
    : undefined;
}

export function travelFromInnCommand(
  shop: Pick<ShopDto, "id" | "innTravelDestinations"> | undefined,
  destinationTownId: string,
): GameCommand | undefined {
  return shop?.innTravelDestinations.some(
    (destination) => destination.townId === destinationTownId,
  )
    ? {
        type: "travel-from-inn",
        facilityId: shop.id,
        destinationTownId,
      }
    : undefined;
}

export function calculateShopTransactionPreview(
  mode: ShopMode,
  quantity: number,
  unitPrice: number,
  weightTenthsPound: number,
  currentGold: number,
  currentWeightTenthsPound: number,
): ShopTransactionPreview {
  const totalPrice = unitPrice * quantity;
  const weightDelta = weightTenthsPound * quantity;
  return {
    totalPrice,
    goldAfter: mode === "buy" ? currentGold - totalPrice : currentGold + totalPrice,
    weightAfterTenthsPound:
      mode === "buy"
        ? currentWeightTenthsPound + weightDelta
        : Math.max(0, currentWeightTenthsPound - weightDelta),
  };
}

export function shopTransactionReason(reason: string, localization: Localization): string {
  const key = `shop-transaction-reason-${reason}`;
  return localization.hasMessage(localization.locale, key) || localization.hasMessage("en-US", key)
    ? localization.format(key)
    : localization.format("shop-transaction-reason-unknown");
}

export function equippedLightText(
  equipment: readonly { readonly kindId: string; readonly slotId: string; readonly fuel?: ShopStockItemDto["fuel"] }[],
  localization: Localization,
  contentName: (id: string | undefined) => string,
): string {
  const light = equipment.find((item) => item.slotId === "light");
  if (!light) return localization.format("status-light-none");
  const item = contentName(light.kindId);
  return light.fuel
    ? localization.format("status-light-fuel", {
        item,
        current: light.fuel.current,
        maximum: light.fuel.maximum,
      })
    : localization.format("status-light-equipped", { item });
}

function sellSelection(item: InventoryItemDto, quote: ShopSellQuoteDto): ShopSelection {
  return {
    id: item.id,
    kindId: item.kindId,
    displayNameKey: item.displayNameKey,
    quantity: quote.unavailableReason ? item.quantity : quote.maximumQuantity,
    inscription: item.inscription ?? undefined,
    capturedActor: item.capturedActor,
    maximumQuantity: quote.maximumQuantity,
    unitPrice: quote.unitPrice,
    weightTenthsPound: item.weightTenthsPound,
    unavailableReason: quote.unavailableReason ?? undefined,
    fuel: item.fuel,
  };
}

function lastShopEvent(state: GameSnapshot | GameUpdate): GameEventDto | undefined {
  if (!("events" in state)) return undefined;
  for (let index = state.events.length - 1; index >= 0; index -= 1) {
    const event = state.events[index];
    if (event && (event.kind.startsWith("shop.") || event.kind.startsWith("inn."))) return event;
  }
  return undefined;
}

function createShopDom(document: Document): ShopDom {
  const element = <T extends HTMLElement>(id: string): T => {
    const found = document.getElementById(id);
    if (!found) throw new Error(`Missing element #${id}`);
    return found as T;
  };
  return {
    dialog: element("shop-dialog"),
    title: element("shop-title"),
    description: element("shop-description"),
    owner: element("shop-owner"),
    close: element("shop-close"),
    buyTab: element("shop-buy-tab"),
    sellTab: element("shop-sell-tab"),
    gold: element("shop-gold-value"),
    weight: element("shop-weight-value"),
    nutrition: element("shop-nutrition-value"),
    light: element("shop-light-value"),
    list: element("shop-item-list"),
    selection: element("shop-selection"),
    quantity: element("shop-quantity"),
    quantityDecrease: element("shop-quantity-decrease"),
    quantityIncrease: element("shop-quantity-increase"),
    quantityMaximum: element("shop-quantity-maximum"),
    total: element("shop-total-value"),
    goldAfter: element("shop-gold-after"),
    weightAfter: element("shop-weight-after"),
    confirm: element("shop-confirm"),
    stay: element("shop-stay"),
    innTravel: element("shop-inn-travel"),
    innDestination: element("shop-inn-destination"),
    innTravelConfirm: element("shop-inn-travel-confirm"),
    feedback: element("shop-feedback"),
  };
}
