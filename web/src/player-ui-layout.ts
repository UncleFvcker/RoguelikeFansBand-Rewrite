// SPDX-License-Identifier: MPL-2.0

import type { Localization } from "./localization";

export type PlayerPage = "inventory" | "ability";
export type PanelPresentation = "page" | "column";

const STORAGE_KEYS: Record<PlayerPage, string> = {
  inventory: "rfb.panel.inventory-presentation",
  ability: "rfb.panel.ability-presentation",
};

interface PlayerUiDom {
  readonly app: HTMLElement;
  readonly settingsOpen: HTMLButtonElement;
  readonly settingsClose: HTMLButtonElement;
  readonly settingsDialog: HTMLDialogElement;
  readonly gameplaySettingsHost: HTMLElement;
  readonly inventoryPresentation: HTMLSelectElement;
  readonly abilityPresentation: HTMLSelectElement;
  readonly inventoryOpen: HTMLButtonElement;
  readonly abilityOpen: HTMLButtonElement;
  readonly pageDialog: HTMLDialogElement;
  readonly pageTitle: HTMLElement;
  readonly pageClose: HTMLButtonElement;
  readonly pageHost: HTMLElement;
  readonly parking: HTMLElement;
  readonly toolColumn: HTMLElement;
  readonly inventoryPanel: HTMLElement;
  readonly abilityPanel: HTMLElement;
  readonly messagePanel: HTMLElement;
  readonly messagePanelHost: HTMLElement;
  readonly supportPanelHost: HTMLElement;
  readonly supportPanels: readonly HTMLElement[];
}

export class PlayerUiLayout {
  readonly #document: Document;
  readonly #window: Window;
  readonly #storage: Storage;
  readonly #localization: Localization;
  readonly #dom: PlayerUiDom;
  #inventoryPresentation: PanelPresentation;
  #abilityPresentation: PanelPresentation;
  #openPage: PlayerPage | undefined;
  #installed = false;

  constructor(options: {
    document: Document;
    window: Window;
    storage: Storage;
    localization: Localization;
  }) {
    this.#document = options.document;
    this.#window = options.window;
    this.#storage = options.storage;
    this.#localization = options.localization;
    this.#dom = createPlayerUiDom(this.#document);
    this.#inventoryPresentation = readPanelPresentation(this.#storage, "inventory");
    this.#abilityPresentation = readPanelPresentation(this.#storage, "ability");
  }

  initialize(): void {
    this.#moveGameplaySettings();
    this.#dom.messagePanelHost.append(this.#dom.messagePanel);
    this.#dom.supportPanelHost.append(...this.#dom.supportPanels);
    this.#dom.inventoryPresentation.value = this.#inventoryPresentation;
    this.#dom.abilityPresentation.value = this.#abilityPresentation;
    this.#applyPanelLayout();
  }

  install(): void {
    if (this.#installed) return;
    this.#installed = true;
    this.#dom.settingsOpen.addEventListener("click", this.#showSettings);
    this.#dom.settingsClose.addEventListener("click", this.#closeSettings);
    this.#dom.inventoryOpen.addEventListener("click", this.#openInventory);
    this.#dom.abilityOpen.addEventListener("click", this.#openAbility);
    this.#dom.pageClose.addEventListener("click", this.#closePageFromButton);
    this.#dom.pageDialog.addEventListener("close", this.#handlePageClosed);
    this.#dom.pageDialog.addEventListener("click", this.#handlePageAction, true);
    this.#dom.inventoryPresentation.addEventListener(
      "change",
      this.#handleInventoryPresentation,
    );
    this.#dom.abilityPresentation.addEventListener(
      "change",
      this.#handleAbilityPresentation,
    );
    this.#window.addEventListener("keydown", this.#handleShortcut);
  }

  dispose(): void {
    if (!this.#installed) return;
    this.#installed = false;
    this.#dom.settingsOpen.removeEventListener("click", this.#showSettings);
    this.#dom.settingsClose.removeEventListener("click", this.#closeSettings);
    this.#dom.inventoryOpen.removeEventListener("click", this.#openInventory);
    this.#dom.abilityOpen.removeEventListener("click", this.#openAbility);
    this.#dom.pageClose.removeEventListener("click", this.#closePageFromButton);
    this.#dom.pageDialog.removeEventListener("close", this.#handlePageClosed);
    this.#dom.pageDialog.removeEventListener("click", this.#handlePageAction, true);
    this.#dom.inventoryPresentation.removeEventListener(
      "change",
      this.#handleInventoryPresentation,
    );
    this.#dom.abilityPresentation.removeEventListener(
      "change",
      this.#handleAbilityPresentation,
    );
    this.#window.removeEventListener("keydown", this.#handleShortcut);
  }

  localize(): void {
    if (this.#openPage) this.#updatePageTitle(this.#openPage);
  }

  closePage(): void {
    if (!this.#dom.pageDialog.open) return;
    this.#returnOpenPanel();
    this.#dom.pageDialog.close();
  }

  open(page: PlayerPage): void {
    if (this.#presentationFor(page) === "column") {
      const panel = this.#panelFor(page);
      panel.scrollIntoView({ behavior: "smooth", block: "nearest" });
      panel.focus({ preventScroll: true });
      panel.classList.remove("panel-attention");
      this.#window.requestAnimationFrame(() => panel.classList.add("panel-attention"));
      return;
    }
    if (this.#dom.pageDialog.open && this.#openPage === page) {
      this.closePage();
      return;
    }
    if (this.#dom.pageDialog.open) this.closePage();
    const panel = this.#panelFor(page);
    this.#openPage = page;
    this.#dom.pageHost.append(panel);
    this.#updatePageTitle(page);
    this.#dom.pageDialog.showModal();
  }

  readonly #showSettings = (): void => {
    if (!this.#dom.settingsDialog.open) this.#dom.settingsDialog.showModal();
  };

  readonly #closeSettings = (): void => {
    if (this.#dom.settingsDialog.open) this.#dom.settingsDialog.close();
  };

  readonly #openInventory = (): void => this.open("inventory");
  readonly #openAbility = (): void => this.open("ability");
  readonly #closePageFromButton = (): void => this.closePage();

  readonly #handlePageClosed = (): void => {
    this.#returnOpenPanel();
  };

  readonly #handlePageAction = (event: Event): void => {
    if (!this.#openPage || this.#presentationFor(this.#openPage) !== "page") return;
    const target = event.target;
    if (!(target instanceof HTMLElement)) return;
    if (
      (this.#openPage === "inventory" && target.closest("#inventory-use")) ||
      (this.#openPage === "ability" && target.closest(".ability-cast-action"))
    ) {
      this.closePage();
    }
  };

  readonly #handleInventoryPresentation = (): void => {
    this.#inventoryPresentation = panelPresentationOrDefault(
      this.#dom.inventoryPresentation.value,
    );
    this.#persistPresentation("inventory", this.#inventoryPresentation);
    this.#applyPanelLayout();
  };

  readonly #handleAbilityPresentation = (): void => {
    this.#abilityPresentation = panelPresentationOrDefault(
      this.#dom.abilityPresentation.value,
    );
    this.#persistPresentation("ability", this.#abilityPresentation);
    this.#applyPanelLayout();
  };

  readonly #handleShortcut = (event: KeyboardEvent): void => {
    if (
      event.defaultPrevented ||
      event.repeat ||
      event.altKey ||
      event.ctrlKey ||
      event.metaKey ||
      this.#dom.app.hidden ||
      isEditableTarget(event.target)
    ) {
      return;
    }
    const page = playerPageForShortcut(event.key);
    if (!page) return;
    const openDialog = this.#document.querySelector<HTMLDialogElement>("dialog[open]");
    if (openDialog && openDialog !== this.#dom.pageDialog) return;
    event.preventDefault();
    this.open(page);
  };

  #applyPanelLayout(): void {
    if (this.#dom.pageDialog.open) this.closePage();
    for (const page of ["inventory", "ability"] as const) {
      const panel = this.#panelFor(page);
      const presentation = this.#presentationFor(page);
      panel.tabIndex = presentation === "column" ? -1 : 0;
      (presentation === "column" ? this.#dom.toolColumn : this.#dom.parking).append(panel);
    }
    const hasToolColumn =
      this.#inventoryPresentation === "column" || this.#abilityPresentation === "column";
    this.#dom.toolColumn.hidden = !hasToolColumn;
    this.#dom.app.dataset.toolColumn = hasToolColumn ? "visible" : "hidden";
  }

  #returnOpenPanel(): void {
    if (!this.#openPage) return;
    const page = this.#openPage;
    this.#openPage = undefined;
    const destination =
      this.#presentationFor(page) === "column" ? this.#dom.toolColumn : this.#dom.parking;
    destination.append(this.#panelFor(page));
  }

  #updatePageTitle(page: PlayerPage): void {
    this.#dom.pageTitle.textContent = this.#localization.format(
      page === "inventory" ? "panel-inventory-title" : "panel-ability-title",
    );
  }

  #panelFor(page: PlayerPage): HTMLElement {
    return page === "inventory" ? this.#dom.inventoryPanel : this.#dom.abilityPanel;
  }

  #presentationFor(page: PlayerPage): PanelPresentation {
    return page === "inventory" ? this.#inventoryPresentation : this.#abilityPresentation;
  }

  #persistPresentation(page: PlayerPage, presentation: PanelPresentation): void {
    try {
      this.#storage.setItem(STORAGE_KEYS[page], presentation);
    } catch {
      // The setting still applies for this session when storage is unavailable.
    }
  }

  #moveGameplaySettings(): void {
    for (const id of [
      "input-preset",
      "tileset-preset",
      "camera-mode",
      "zoom-level",
      "language-select",
    ]) {
      const control = this.#document.getElementById(id);
      const label = control?.closest("label");
      if (label) this.#dom.gameplaySettingsHost.append(label);
    }
    const controls = this.#document.getElementById("controls-help");
    if (controls) this.#dom.gameplaySettingsHost.append(controls);
  }
}

export function readPanelPresentation(
  storage: Pick<Storage, "getItem">,
  page: PlayerPage,
): PanelPresentation {
  try {
    return panelPresentationOrDefault(storage.getItem(STORAGE_KEYS[page]));
  } catch {
    return "page";
  }
}

export function playerPageForShortcut(key: string): PlayerPage | undefined {
  const normalized = key.toLowerCase();
  return normalized === "i" ? "inventory" : normalized === "m" ? "ability" : undefined;
}

function panelPresentationOrDefault(value: string | null): PanelPresentation {
  return value === "column" ? "column" : "page";
}

function isEditableTarget(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLSelectElement ||
    (target instanceof HTMLElement && target.isContentEditable)
  );
}

function createPlayerUiDom(document: Document): PlayerUiDom {
  const element = <T extends HTMLElement>(id: string): T => {
    const found = document.getElementById(id);
    if (!found) throw new Error(`Missing element #${id}`);
    return found as T;
  };
  return {
    app: element("app"),
    settingsOpen: element("player-ui-settings-open"),
    settingsClose: element("player-ui-settings-close"),
    settingsDialog: element("player-ui-settings-dialog"),
    gameplaySettingsHost: element("gameplay-settings-host"),
    inventoryPresentation: element("inventory-presentation"),
    abilityPresentation: element("ability-presentation"),
    inventoryOpen: element("player-ui-inventory-open"),
    abilityOpen: element("player-ui-ability-open"),
    pageDialog: element("player-page-dialog"),
    pageTitle: element("player-page-title"),
    pageClose: element("player-page-close"),
    pageHost: element("player-page-host"),
    parking: element("player-page-parking"),
    toolColumn: element("player-tool-column"),
    inventoryPanel: element("inventory-panel"),
    abilityPanel: element("ability-panel"),
    messagePanel: element("message-panel"),
    messagePanelHost: element("message-panel-host"),
    supportPanelHost: element("support-panel-host"),
    supportPanels: [
      element("summon-command-panel"),
      element("campaign-panel"),
      element("task-log-panel"),
      element("native-save-panel"),
    ],
  };
}
