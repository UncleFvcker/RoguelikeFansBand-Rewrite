// SPDX-License-Identifier: MPL-2.0

import type { AppState } from "./app-state";
import type { Localization } from "./localization";
import type { GameCommand, GameEventDto, GameSnapshot, GameUpdate, HomeDto, HomeItemDto } from "./protocol";

type HomeMode = "withdraw" | "deposit";

interface HomeDom {
  dialog: HTMLDialogElement;
  title: HTMLElement;
  description: HTMLElement;
  close: HTMLButtonElement;
  withdrawTab: HTMLButtonElement;
  depositTab: HTMLButtonElement;
  list: HTMLUListElement;
  weight: HTMLElement;
  selection: HTMLElement;
  quantity: HTMLInputElement;
  decrease: HTMLButtonElement;
  increase: HTMLButtonElement;
  maximum: HTMLButtonElement;
  weightAfter: HTMLElement;
  confirm: HTMLButtonElement;
  feedback: HTMLElement;
}

export class HomePanel {
  readonly #state: AppState;
  readonly #localization: Localization;
  readonly #dispatch: (command: GameCommand) => Promise<void>;
  readonly #formatEvent: (event: GameEventDto) => string;
  readonly #visibleItemName: (displayNameKey: string, kindId: string) => string;
  readonly #beforeOpen: () => void;
  readonly #dom: HomeDom;
  #mode: HomeMode = "withdraw";
  #home: HomeDto | undefined;
  #selectedItemId: string | undefined;
  #dismissedId: string | undefined;
  #feedback: GameEventDto | undefined;
  #installed = false;

  constructor(options: {
    document: Document;
    state: AppState;
    localization: Localization;
    dispatch: (command: GameCommand) => Promise<void>;
    formatEvent: (event: GameEventDto) => string;
    visibleItemName: (displayNameKey: string, kindId: string) => string;
    beforeOpen: () => void;
  }) {
    this.#state = options.state;
    this.#localization = options.localization;
    this.#dispatch = options.dispatch;
    this.#formatEvent = options.formatEvent;
    this.#visibleItemName = options.visibleItemName;
    this.#beforeOpen = options.beforeOpen;
    this.#dom = createHomeDom(options.document);
  }

  install(): void {
    if (this.#installed) return;
    this.#installed = true;
    this.#dom.close.addEventListener("click", this.#close);
    this.#dom.dialog.addEventListener("close", this.#closed);
    this.#dom.withdrawTab.addEventListener("click", this.#showWithdraw);
    this.#dom.depositTab.addEventListener("click", this.#showDeposit);
    this.#dom.list.addEventListener("click", this.#select);
    this.#dom.quantity.addEventListener("input", this.#renderTransaction);
    this.#dom.decrease.addEventListener("click", this.#decrease);
    this.#dom.increase.addEventListener("click", this.#increase);
    this.#dom.maximum.addEventListener("click", this.#maximize);
    this.#dom.confirm.addEventListener("click", this.#confirm);
  }

  dispose(): void {
    if (!this.#installed) return;
    this.#installed = false;
    this.#dom.close.removeEventListener("click", this.#close);
    this.#dom.dialog.removeEventListener("close", this.#closed);
    this.#dom.withdrawTab.removeEventListener("click", this.#showWithdraw);
    this.#dom.depositTab.removeEventListener("click", this.#showDeposit);
    this.#dom.list.removeEventListener("click", this.#select);
    this.#dom.quantity.removeEventListener("input", this.#renderTransaction);
    this.#dom.decrease.removeEventListener("click", this.#decrease);
    this.#dom.increase.removeEventListener("click", this.#increase);
    this.#dom.maximum.removeEventListener("click", this.#maximize);
    this.#dom.confirm.removeEventListener("click", this.#confirm);
  }

  render(state: GameSnapshot | GameUpdate): void {
    const event = lastHomeEvent(state);
    if (event) this.#feedback = event;
    const home = state.homes.find((candidate) => candidate.playerAtEntrance);
    if (!home) {
      this.reset();
      return;
    }
    const changed = this.#home?.id !== home.id;
    this.#home = home;
    if (changed) {
      this.#mode = "withdraw";
      this.#selectedItemId = undefined;
      this.#feedback = undefined;
    }
    this.#renderPanel();
    if (!this.#dom.dialog.open && this.#dismissedId !== home.id) {
      this.#beforeOpen();
      this.#dom.dialog.showModal();
      this.#focusSelection();
    }
  }

  localize(): void {
    if (this.#home) this.#renderPanel();
  }

  updateActions(): void {
    if (this.#home) this.#renderTransaction();
  }

  reset(): void {
    this.#home = undefined;
    this.#selectedItemId = undefined;
    this.#dismissedId = undefined;
    this.#feedback = undefined;
    if (this.#dom.dialog.open) this.#dom.dialog.close();
  }

  readonly #close = (): void => {
    if (this.#dom.dialog.open) this.#dom.dialog.close();
  };
  readonly #closed = (): void => {
    if (this.#home?.playerAtEntrance) this.#dismissedId = this.#home.id;
  };
  readonly #showWithdraw = (): void => this.#setMode("withdraw");
  readonly #showDeposit = (): void => this.#setMode("deposit");
  readonly #select = (event: Event): void => {
    const target = event.target;
    if (!(target instanceof HTMLElement)) return;
    const button = target.closest<HTMLButtonElement>("[data-home-item-id]");
    if (!button || button.disabled) return;
    this.#selectedItemId = button.dataset.homeItemId;
    this.#dom.quantity.value = "1";
    this.#feedback = undefined;
    this.#renderItems();
    this.#renderTransaction();
  };
  readonly #decrease = (): void => this.#step(-1);
  readonly #increase = (): void => this.#step(1);
  readonly #maximize = (): void => {
    const item = this.#selection();
    if (!item) return;
    this.#dom.quantity.value = String(item.maximumQuantity);
    this.#renderTransaction();
  };
  readonly #confirm = (): void => {
    const home = this.#home;
    const item = this.#selection();
    const quantity = item ? parseHomeQuantity(this.#dom.quantity.value, item.maximumQuantity) : undefined;
    if (!home || !item || quantity === undefined || this.#state.busy) return;
    const command: GameCommand = this.#mode === "withdraw"
      ? { type: "withdraw-from-home", facilityId: home.id, itemId: item.id, quantity }
      : { type: "deposit-at-home", facilityId: home.id, itemId: item.id, quantity };
    void this.#dispatch(command);
  };

  #setMode(mode: HomeMode): void {
    if (this.#mode === mode) return;
    this.#mode = mode;
    this.#selectedItemId = undefined;
    this.#dom.quantity.value = "1";
    this.#feedback = undefined;
    this.#renderPanel();
    this.#focusSelection();
  }

  #step(delta: number): void {
    const item = this.#selection();
    if (!item) return;
    const current = parseHomeQuantity(this.#dom.quantity.value, item.maximumQuantity) ?? 1;
    this.#dom.quantity.value = String(Math.max(1, Math.min(item.maximumQuantity, current + delta)));
    this.#renderTransaction();
  }

  #renderPanel(): void {
    const home = this.#home;
    const status = this.#state.status;
    if (!home || !status) return;
    this.#dom.title.textContent = this.#localization.format(home.nameKey);
    this.#dom.description.textContent = this.#localization.format(home.descriptionKey);
    this.#dom.withdrawTab.setAttribute("aria-selected", String(this.#mode === "withdraw"));
    this.#dom.depositTab.setAttribute("aria-selected", String(this.#mode === "deposit"));
    this.#dom.weight.textContent = this.#localization.format("shop-weight-current", {
      weight: formatTenths(status.player.carriedWeightTenthsPound),
      capacity: formatTenths(status.player.carryCapacityTenthsPound),
    });
    this.#renderItems();
    this.#renderTransaction();
  }

  #renderItems(): void {
    const items = this.#items();
    if (!items.some((item) => item.id === this.#selectedItemId && item.maximumQuantity > 0)) {
      this.#selectedItemId = items.find((item) => item.maximumQuantity > 0)?.id;
      this.#dom.quantity.value = "1";
    }
    this.#dom.list.replaceChildren();
    if (items.length === 0) {
      const empty = this.#dom.list.ownerDocument.createElement("li");
      empty.className = "shop-empty";
      empty.textContent = this.#localization.format(this.#mode === "withdraw" ? "home-storage-empty" : "home-pack-empty");
      this.#dom.list.append(empty);
      return;
    }
    for (const item of items) {
      const row = this.#dom.list.ownerDocument.createElement("li");
      row.className = "shop-item-row";
      const button = this.#dom.list.ownerDocument.createElement("button");
      button.type = "button";
      button.className = "shop-item-select";
      button.dataset.homeItemId = item.id;
      button.disabled = item.maximumQuantity === 0;
      button.setAttribute("aria-pressed", String(item.id === this.#selectedItemId));
      button.append(
        span(this.#dom.list, "shop-item-name", this.#visibleItemName(item.displayNameKey, item.kindId)),
        span(this.#dom.list, "shop-item-details", this.#localization.format("shop-item-weight", { weight: formatTenths(item.weightTenthsPound) })),
        span(this.#dom.list, "shop-item-stock", this.#localization.format(this.#mode === "withdraw" ? "home-stored-count" : "shop-owned-count", { quantity: item.quantity })),
      );
      row.append(button);
      this.#dom.list.append(row);
    }
  }

  readonly #renderTransaction = (): void => {
    const status = this.#state.status;
    const item = this.#selection();
    const maximum = item?.maximumQuantity ?? 0;
    const quantity = item ? parseHomeQuantity(this.#dom.quantity.value, maximum) : undefined;
    const valid = item !== undefined && quantity !== undefined;
    this.#dom.quantity.max = String(maximum);
    this.#dom.quantity.disabled = this.#state.busy || maximum === 0;
    this.#dom.decrease.disabled = this.#state.busy || !valid || (quantity ?? 0) <= 1;
    this.#dom.increase.disabled = this.#state.busy || !valid || (quantity ?? 0) >= maximum;
    this.#dom.maximum.disabled = this.#state.busy || maximum <= 1;
    this.#dom.confirm.disabled = this.#state.busy || !valid;
    this.#dom.confirm.textContent = this.#localization.format(this.#mode === "withdraw" ? "action-home-withdraw" : "action-home-deposit");
    if (!status || !item || quantity === undefined) {
      this.#dom.selection.textContent = this.#localization.format("shop-selection-none");
      this.#dom.weightAfter.textContent = status ? formatTenths(status.player.carriedWeightTenthsPound) : "--";
    } else {
      this.#dom.selection.textContent = this.#localization.format("shop-selection-summary", {
        item: this.#visibleItemName(item.displayNameKey, item.kindId),
        maximum,
      });
      const delta = item.weightTenthsPound * quantity * (this.#mode === "withdraw" ? 1 : -1);
      this.#dom.weightAfter.textContent = this.#localization.format("shop-weight-after", {
        weight: formatTenths(Math.max(0, status.player.carriedWeightTenthsPound + delta)),
        capacity: formatTenths(status.player.carryCapacityTenthsPound),
      });
    }
    this.#dom.feedback.textContent = this.#feedback ? this.#formatEvent(this.#feedback) : "";
    this.#dom.feedback.dataset.kind = this.#feedback?.kind === "home.transfer-unavailable" ? "error" : this.#feedback ? "success" : "none";
  };

  #items(): HomeItemDto[] {
    if (!this.#home) return [];
    return this.#mode === "withdraw" ? this.#home.storedItems : this.#home.depositItems;
  }
  #selection(): HomeItemDto | undefined {
    return this.#items().find((item) => item.id === this.#selectedItemId);
  }
  #focusSelection(): void {
    this.#dom.list.querySelector<HTMLButtonElement>("[data-home-item-id]:not(:disabled)")?.focus();
  }
}

export function parseHomeQuantity(value: string, maximum: number): number | undefined {
  const quantity = Number(value);
  return Number.isSafeInteger(quantity) && quantity >= 1 && quantity <= maximum ? quantity : undefined;
}

function formatTenths(value: number): string {
  return `${Math.floor(value / 10)}.${Math.abs(value % 10)}`;
}

function span(list: HTMLUListElement, className: string, text: string): HTMLSpanElement {
  const element = list.ownerDocument.createElement("span");
  element.className = className;
  element.textContent = text;
  return element;
}

function lastHomeEvent(state: GameSnapshot | GameUpdate): GameEventDto | undefined {
  if (!("events" in state)) return undefined;
  for (let index = state.events.length - 1; index >= 0; index -= 1) {
    const event = state.events[index];
    if (event?.kind.startsWith("home.")) return event;
  }
  return undefined;
}

function createHomeDom(document: Document): HomeDom {
  const element = <T extends HTMLElement>(id: string): T => {
    const found = document.getElementById(id);
    if (!found) throw new Error(`Missing element #${id}`);
    return found as T;
  };
  return {
    dialog: element("home-dialog"), title: element("home-title"), description: element("home-description"),
    close: element("home-close"), withdrawTab: element("home-withdraw-tab"), depositTab: element("home-deposit-tab"),
    list: element("home-item-list"), weight: element("home-weight-value"), selection: element("home-selection"),
    quantity: element("home-quantity"), decrease: element("home-quantity-decrease"), increase: element("home-quantity-increase"),
    maximum: element("home-quantity-maximum"), weightAfter: element("home-weight-after"), confirm: element("home-confirm"),
    feedback: element("home-feedback"),
  };
}
