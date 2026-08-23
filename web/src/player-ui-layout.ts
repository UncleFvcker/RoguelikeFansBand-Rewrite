// SPDX-License-Identifier: MPL-2.0

import type { Localization } from "./localization";

export type PlayerPage = "inventory" | "ability" | "character";

interface PlayerUiDom {
  readonly app: HTMLElement;
  readonly intelSidebar: HTMLElement;
  readonly intelNearbyTab: HTMLButtonElement;
  readonly intelMessageTab: HTMLButtonElement;
  readonly hudIdentityHost: HTMLElement;
  readonly hudVitalsHost: HTMLElement;
  readonly hudMenuContent: HTMLElement;
  readonly settingsOpen: HTMLButtonElement;
  readonly settingsClose: HTMLButtonElement;
  readonly settingsDialog: HTMLDialogElement;
  readonly gameplaySettingsHost: HTMLElement;
  readonly inventoryOpen: HTMLButtonElement;
  readonly abilityOpen: HTMLButtonElement;
  readonly characterOpen: HTMLButtonElement;
  readonly pageDialog: HTMLDialogElement;
  readonly pageTitle: HTMLElement;
  readonly pageClose: HTMLButtonElement;
  readonly pageHost: HTMLElement;
  readonly parking: HTMLElement;
  readonly inventoryPanel: HTMLElement;
  readonly abilityPanel: HTMLElement;
  readonly characterPanel: HTMLElement;
  readonly messagePanel: HTMLElement;
  readonly messagePanelHost: HTMLElement;
  readonly supportPanelHost: HTMLElement;
  readonly supportPanels: readonly HTMLElement[];
  readonly progressionPanel: HTMLElement;
  readonly statusPanel: HTMLElement;
  readonly resourcePanel: HTMLElement;
  readonly hudAttributeList: HTMLUListElement;
  readonly playerPageActions: HTMLElement;
}

export class PlayerUiLayout {
  readonly #document: Document;
  readonly #window: Window;
  readonly #localization: Localization;
  readonly #dom: PlayerUiDom;
  #openPage: PlayerPage | undefined;
  #installed = false;

  constructor(options: {
    document: Document;
    window: Window;
    localization: Localization;
  }) {
    this.#document = options.document;
    this.#window = options.window;
    this.#localization = options.localization;
    this.#dom = createPlayerUiDom(this.#document);
  }

  initialize(): void {
    this.#moveTopHud();
    this.#moveGameplaySettings();
    this.#dom.messagePanelHost.append(this.#dom.messagePanel);
    this.#dom.supportPanelHost.append(...this.#dom.supportPanels);
    this.#dom.parking.append(
      this.#dom.inventoryPanel,
      this.#dom.abilityPanel,
      this.#dom.characterPanel,
    );
    this.#selectIntelPanel("nearby");
  }

  install(): void {
    if (this.#installed) return;
    this.#installed = true;
    this.#dom.settingsOpen.addEventListener("click", this.#showSettings);
    this.#dom.settingsClose.addEventListener("click", this.#closeSettings);
    this.#dom.inventoryOpen.addEventListener("click", this.#openInventory);
    this.#dom.abilityOpen.addEventListener("click", this.#openAbility);
    this.#dom.characterOpen.addEventListener("click", this.#openCharacter);
    this.#dom.intelNearbyTab.addEventListener("click", this.#showNearbyIntel);
    this.#dom.intelMessageTab.addEventListener("click", this.#showMessageIntel);
    this.#dom.pageClose.addEventListener("click", this.#closePageFromButton);
    this.#dom.pageDialog.addEventListener("close", this.#handlePageClosed);
    this.#dom.pageDialog.addEventListener("click", this.#handlePageAction, true);
    this.#window.addEventListener("keydown", this.#handleShortcut);
  }

  dispose(): void {
    if (!this.#installed) return;
    this.#installed = false;
    this.#dom.settingsOpen.removeEventListener("click", this.#showSettings);
    this.#dom.settingsClose.removeEventListener("click", this.#closeSettings);
    this.#dom.inventoryOpen.removeEventListener("click", this.#openInventory);
    this.#dom.abilityOpen.removeEventListener("click", this.#openAbility);
    this.#dom.characterOpen.removeEventListener("click", this.#openCharacter);
    this.#dom.intelNearbyTab.removeEventListener("click", this.#showNearbyIntel);
    this.#dom.intelMessageTab.removeEventListener("click", this.#showMessageIntel);
    this.#dom.pageClose.removeEventListener("click", this.#closePageFromButton);
    this.#dom.pageDialog.removeEventListener("close", this.#handlePageClosed);
    this.#dom.pageDialog.removeEventListener("click", this.#handlePageAction, true);
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
  readonly #openCharacter = (): void => this.open("character");
  readonly #showNearbyIntel = (): void => this.#selectIntelPanel("nearby");
  readonly #showMessageIntel = (): void => this.#selectIntelPanel("message");
  readonly #closePageFromButton = (): void => this.closePage();

  readonly #handlePageClosed = (): void => {
    this.#returnOpenPanel();
  };

  readonly #handlePageAction = (event: Event): void => {
    if (!this.#openPage) return;
    const target = event.target;
    if (!(target instanceof HTMLElement)) return;
    if (
      (this.#openPage === "inventory" &&
        target.closest("#inventory-use, #inventory-absorb")) ||
      (this.#openPage === "ability" && target.closest(".ability-cast-action"))
    ) {
      this.closePage();
    }
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

  #returnOpenPanel(): void {
    if (!this.#openPage) return;
    const page = this.#openPage;
    this.#openPage = undefined;
    this.#dom.parking.append(this.#panelFor(page));
  }

  #updatePageTitle(page: PlayerPage): void {
    const titleKey =
      page === "inventory"
        ? "panel-inventory-title"
        : page === "ability"
          ? "panel-ability-title"
          : "panel-character-details-title";
    this.#dom.pageTitle.textContent = this.#localization.format(titleKey);
  }

  #panelFor(page: PlayerPage): HTMLElement {
    return page === "inventory"
      ? this.#dom.inventoryPanel
      : page === "ability"
        ? this.#dom.abilityPanel
        : this.#dom.characterPanel;
  }

  #selectIntelPanel(panel: "nearby" | "message"): void {
    this.#dom.intelSidebar.dataset.intelPanel = panel;
    this.#dom.intelNearbyTab.setAttribute(
      "aria-pressed",
      String(panel === "nearby"),
    );
    this.#dom.intelMessageTab.setAttribute(
      "aria-pressed",
      String(panel === "message"),
    );
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

  #moveTopHud(): void {
    this.#dom.hudIdentityHost.append(
      this.#dom.progressionPanel,
      this.#dom.hudAttributeList,
    );
    this.#dom.hudVitalsHost.append(this.#dom.statusPanel, this.#dom.resourcePanel);
    this.#dom.hudMenuContent.append(this.#dom.playerPageActions);
  }
}

export function playerPageForShortcut(key: string): PlayerPage | undefined {
  const normalized = key.toLowerCase();
  return normalized === "i" ? "inventory" : normalized === "m" ? "ability" : undefined;
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
    intelSidebar: element("player-sidebar"),
    intelNearbyTab: element<HTMLButtonElement>("intel-nearby-tab"),
    intelMessageTab: element<HTMLButtonElement>("intel-message-tab"),
    hudIdentityHost: element("hud-identity-host"),
    hudVitalsHost: element("hud-vitals-host"),
    hudMenuContent: element("hud-menu-content"),
    settingsOpen: element("player-ui-settings-open"),
    settingsClose: element("player-ui-settings-close"),
    settingsDialog: element("player-ui-settings-dialog"),
    gameplaySettingsHost: element("gameplay-settings-host"),
    inventoryOpen: element("player-ui-inventory-open"),
    abilityOpen: element("player-ui-ability-open"),
    characterOpen: element("player-ui-character-open"),
    pageDialog: element("player-page-dialog"),
    pageTitle: element("player-page-title"),
    pageClose: element("player-page-close"),
    pageHost: element("player-page-host"),
    parking: element("player-page-parking"),
    inventoryPanel: element("inventory-panel"),
    abilityPanel: element("ability-panel"),
    characterPanel: element("character-details-panel"),
    messagePanel: element("message-panel"),
    messagePanelHost: element("message-panel-host"),
    supportPanelHost: element("support-panel-host"),
    supportPanels: [
      element("dungeon-info-panel"),
      element("summon-command-panel"),
      element("campaign-panel"),
      element("task-log-panel"),
      element("native-save-panel"),
    ],
    progressionPanel: element("progression-panel"),
    statusPanel: element("status-panel"),
    resourcePanel: element("resource-panel"),
    hudAttributeList: element<HTMLUListElement>("hud-attribute-list"),
    playerPageActions: element("player-page-actions"),
  };
}
