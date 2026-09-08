// SPDX-License-Identifier: MPL-2.0

import type { Localization } from "./localization";

const PLAYER_PAGE_TITLES = {
  ability: "player-page-ability",
  character: "player-page-character",
  inventory: "player-page-inventory",
  tasks: "panel-task-log-title",
} as const;

export type PlayerPage = keyof typeof PLAYER_PAGE_TITLES;

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
  readonly tasksOpen: HTMLButtonElement;
  readonly pageTabs: Record<PlayerPage, HTMLButtonElement>;
  readonly characterTabs: readonly HTMLButtonElement[];
  readonly characterDetailTabs: readonly HTMLButtonElement[];
  readonly pageDialog: HTMLDialogElement;
  readonly pageTitle: HTMLElement;
  readonly pageClose: HTMLButtonElement;
  readonly pageHost: HTMLElement;
  readonly parking: HTMLElement;
  readonly inventoryPanel: HTMLElement;
  readonly abilityPanel: HTMLElement;
  readonly characterPanel: HTMLElement;
  readonly taskPanel: HTMLElement;
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
  readonly #characterScrollPositions = new Map<string, number>();
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
      this.#dom.taskPanel,
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
    this.#dom.tasksOpen.addEventListener("click", this.#openTasks);
    for (const tab of [...Object.values(this.#dom.pageTabs), ...this.#dom.characterTabs, ...this.#dom.characterDetailTabs]) {
      tab.addEventListener("click", this.#handleTabClick);
      tab.addEventListener("keydown", this.#handleTabKeydown);
    }
    this.#dom.intelNearbyTab.addEventListener("click", this.#showNearbyIntel);
    this.#dom.intelMessageTab.addEventListener("click", this.#showMessageIntel);
    this.#dom.pageClose.addEventListener("click", this.#closePageFromButton);
    this.#dom.pageDialog.addEventListener("close", this.#handlePageClosed);
    this.#dom.pageDialog.addEventListener("cancel", this.#handlePageCancel);
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
    this.#dom.tasksOpen.removeEventListener("click", this.#openTasks);
    for (const tab of [...Object.values(this.#dom.pageTabs), ...this.#dom.characterTabs, ...this.#dom.characterDetailTabs]) {
      tab.removeEventListener("click", this.#handleTabClick);
      tab.removeEventListener("keydown", this.#handleTabKeydown);
    }
    this.#dom.intelNearbyTab.removeEventListener("click", this.#showNearbyIntel);
    this.#dom.intelMessageTab.removeEventListener("click", this.#showMessageIntel);
    this.#dom.pageClose.removeEventListener("click", this.#closePageFromButton);
    this.#dom.pageDialog.removeEventListener("close", this.#handlePageClosed);
    this.#dom.pageDialog.removeEventListener("cancel", this.#handlePageCancel);
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
    if (this.#dom.pageDialog.open && this.#openPage === page) return;
    this.#returnOpenPanel();
    const panel = this.#panelFor(page);
    this.#openPage = page;
    this.#dom.pageHost.append(panel);
    this.#dom.pageDialog.dataset.page = page;
    this.#updatePageTitle(page);
    for (const [key, tab] of Object.entries(this.#dom.pageTabs)) {
      tab.setAttribute("aria-selected", String(key === page));
      tab.tabIndex = key === page ? 0 : -1;
    }
    const activeTab = this.#dom.pageTabs[page];
    this.#dom.pageHost.setAttribute("aria-labelledby", activeTab.id);
    if (!this.#dom.pageDialog.open) this.#dom.pageDialog.showModal();
    activeTab.focus();
    if (page === "character") this.#restoreCharacterScroll();
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
  readonly #openTasks = (): void => this.open("tasks");
  readonly #showNearbyIntel = (): void => this.#selectIntelPanel("nearby");
  readonly #showMessageIntel = (): void => this.#selectIntelPanel("message");
  readonly #closePageFromButton = (): void => this.closePage();

  readonly #handlePageClosed = (): void => {
    if (!this.#dom.pageDialog.open) this.#returnOpenPanel();
  };

  readonly #handlePageCancel = (): void => {
    if (this.#openPage === "character") this.#saveCharacterScroll();
  };

  readonly #handleTabClick = (event: Event): void => {
    const tab = event.currentTarget as HTMLButtonElement;
    this.#activateTab(tab);
  };

  #activateTab(tab: HTMLButtonElement): void {
    if (tab.dataset.playerPage) {
      this.open(tab.dataset.playerPage as PlayerPage);
      return;
    }
    this.#saveCharacterScroll();
    for (const candidate of this.#tabsFor(tab)) {
      const selected = candidate === tab;
      candidate.setAttribute("aria-selected", String(selected));
      candidate.tabIndex = selected ? 0 : -1;
      // Keep each pane mounted so live data bindings stay intact.
      this.#document.getElementById(candidate.getAttribute("aria-controls")!)!.hidden = !selected;
    }
    tab.focus();
    this.#restoreCharacterScroll();
  }

  #saveCharacterScroll(): void {
    for (const pane of this.#activeCharacterPanes()) {
      this.#characterScrollPositions.set(pane.id, pane.scrollTop);
    }
  }

  #restoreCharacterScroll(): void {
    for (const pane of this.#activeCharacterPanes()) {
      pane.scrollTop = this.#characterScrollPositions.get(pane.id) ?? 0;
    }
  }

  #activeCharacterPanes(): HTMLElement[] {
    const panes: HTMLElement[] = [];
    for (const tab of this.#dom.characterTabs) {
      if (tab.getAttribute("aria-selected") !== "true") continue;
      panes.push(this.#document.getElementById(tab.getAttribute("aria-controls")!)!);
      if (tab.dataset.characterPage === "details") {
        for (const detail of this.#dom.characterDetailTabs) {
          if (detail.getAttribute("aria-selected") === "true") {
            panes.push(this.#document.getElementById(detail.getAttribute("aria-controls")!)!);
          }
        }
      }
    }
    return panes;
  }

  #tabsFor(tab: HTMLButtonElement): readonly HTMLButtonElement[] {
    if (tab.dataset.characterDetail) return this.#dom.characterDetailTabs;
    return tab.dataset.characterPage ? this.#dom.characterTabs : Object.values(this.#dom.pageTabs);
  }

  readonly #handleTabKeydown = (event: KeyboardEvent): void => {
    if (event.altKey || event.ctrlKey || event.metaKey) return;
    const tabs = this.#tabsFor(event.currentTarget as HTMLButtonElement);
    const index = tabs.indexOf(event.currentTarget as HTMLButtonElement);
    let next: number;
    switch (event.key) {
      case "ArrowLeft": next = (index + tabs.length - 1) % tabs.length; break;
      case "ArrowRight": next = (index + 1) % tabs.length; break;
      case "Home": next = 0; break;
      case "End": next = tabs.length - 1; break;
      default: return;
    }
    event.preventDefault();
    this.#activateTab(tabs[next]!);
  };

  readonly #handlePageAction = (event: Event): void => {
    if (!this.#openPage) return;
    const target = event.target;
    if (!(target instanceof HTMLElement)) return;
    if (
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
    if (this.#document.querySelector("dialog[open]:not(#player-page-dialog)")) return;
    event.preventDefault();
    if (this.#dom.pageDialog.open && this.#openPage === page) this.closePage();
    else this.open(page);
  };

  #returnOpenPanel(): void {
    if (!this.#openPage) return;
    if (this.#openPage === "character" && this.#dom.pageDialog.open) this.#saveCharacterScroll();
    if (this.#openPage === "inventory") {
      for (const id of ["inventory-detail-dialog", "inventory-action-dialog", "inventory-more-dialog"]) {
        const dialog = this.#document.getElementById(id) as HTMLDialogElement;
        if (dialog.open) dialog.close();
      }
    }
    const page = this.#openPage;
    this.#openPage = undefined;
    this.#dom.parking.append(this.#panelFor(page));
  }

  #updatePageTitle(page: PlayerPage): void {
    this.#dom.pageTitle.textContent = this.#localization.format(PLAYER_PAGE_TITLES[page]);
  }

  #panelFor(page: PlayerPage): HTMLElement {
    return page === "inventory"
      ? this.#dom.inventoryPanel
      : page === "ability"
        ? this.#dom.abilityPanel
        : page === "character"
          ? this.#dom.characterPanel
          : this.#dom.taskPanel;
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
    tasksOpen: element("player-ui-tasks-open"),
    pageTabs: {
      ability: element("player-page-tab-ability"),
      character: element("player-page-tab-character"),
      inventory: element("player-page-tab-inventory"),
      tasks: element("player-page-tab-tasks"),
    },
    characterTabs: ["overview", "details", "proficiencies", "other"].map(
      (page) => element<HTMLButtonElement>(`character-tab-${page}`),
    ),
    characterDetailTabs: ["sources", "defenses", "offense"].map(
      (page) => element<HTMLButtonElement>(`character-detail-tab-${page}`),
    ),
    pageDialog: element("player-page-dialog"),
    pageTitle: element("player-page-title"),
    pageClose: element("player-page-close"),
    pageHost: element("player-page-host"),
    parking: element("player-page-parking"),
    inventoryPanel: element("inventory-panel"),
    abilityPanel: element("ability-panel"),
    characterPanel: element("character-details-panel"),
    taskPanel: element("task-log-panel"),
    messagePanel: element("message-panel"),
    messagePanelHost: element("message-panel-host"),
    supportPanelHost: element("support-panel-host"),
    supportPanels: [
      element("dungeon-info-panel"),
      element("summon-command-panel"),
      element("campaign-panel"),
      element("task-log-entry"),
      element("native-save-panel"),
    ],
    progressionPanel: element("progression-panel"),
    statusPanel: element("status-panel"),
    resourcePanel: element("resource-panel"),
    hudAttributeList: element<HTMLUListElement>("hud-attribute-list"),
    playerPageActions: element("player-page-actions"),
  };
}
