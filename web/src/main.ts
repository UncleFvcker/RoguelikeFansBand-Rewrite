// SPDX-License-Identifier: MPL-2.0
import { HighScorePanel, confirmCharacterEnd } from "./high-scores";

import "./styles.css";

import { getCurrentWindow } from "@tauri-apps/api/window";

import {
  Localization,
  type LocalizationArgs,
  type MessageKey,
} from "./localization";
import { LOCALIZATION_SOURCES } from "./localization-resources";
import { MapRenderer } from "./map-renderer";
import {
  DesktopCrashDiagnostics,
  type CrashDiagnosticStatus,
} from "./crash-diagnostics";
import {
  NativeSaveStorage,
  desktopErrorCode,
} from "./native-save-storage";
import { createPresentationFormatter } from "./event-format";
import { MessagePanel, type MessageRecord } from "./message-panel";
import { NativeSaveCommands, nativeSaveErrorKey } from "./save-panel";
import { createAppDom } from "./app-dom";
import { AppState, type ConnectionState } from "./app-state";
import type { NewSessionRequest } from "./core-transport";
import { InputController } from "./input-controller";
import type { CommandShortcut, ItemShortcut } from "./command-shortcuts";
import { GameSession } from "./game-session";
import { DuelistPanel } from "./duelist-panel";
import { PetMenu } from "./pet-menu";
import { MagicEaterPanel } from "./magic-eater-panel";
import { SpellRealmsPanel } from "./spell-realms-panel";
import {
  SettingsPanel,
  inputPresetMessageKey,
  isInputPreset,
} from "./settings-panel";
import { StatusPanel, formatAttributeValue, abilityConfirmationMessageKey, hudLocationText } from "./status-panel";
import { isAutomaticallyRepeatedCommand } from "./terrain-interaction";
import {
  InventoryPanel,
  createItemCurseSeverityName,
  formatTenthsPoundArgument,
} from "./inventory-panel";
import type { GameCommand, GameEventDto, GameSnapshot, MogaminatorDto } from "./protocol";
import { TauriNativeTransport } from "./tauri-native-transport";
import { installRendererProfileHook } from "./render-profile";
import { createSessionShellDom, SessionShell } from "./session-shell";
import { JourneyResult } from "./journey-result";
import { PlayerUiLayout } from "./player-ui-layout";
import { ShopPanel } from "./shop-panel";
import { HomePanel } from "./home-panel";
import { TaskServicePanel } from "./task-service-panel";
import { ObjectListPanel } from "./object-list";
import { MogaminatorEditor } from "./mogaminator-editor";
import { MonsterProbePanel } from "./monster-probe-panel";
import { MapIntelligencePanel, type MapInquiry } from "./map-intelligence";
import { HelpKnowledgePanel, type KnowledgeEntry } from "./help-knowledge";
import { ConfigRecords } from "./config-records";
import { PreferencesClient, behaviorPreferences } from "./preferences";
import { nativePreferences } from "./native-preferences";
import { CombatSummaryPanel } from "./combat-summary";

const core = new TauriNativeTransport();
const currentWindow = getCurrentWindow();
const crashDiagnostics = new DesktopCrashDiagnostics();
const nativeSaveStorage = new NativeSaveStorage();
const preferences = new PreferencesClient(nativePreferences);
const renderer = new MapRenderer();
const appState = new AppState();
let rendererInitialized = false;
let recordingFrontendCrash = false;
let announcedCrashReport: string | undefined;
const announcedCrashDiagnosticErrors = new Set<string>();

const appDom = createAppDom(document);
const sessionShellDom = createSessionShellDom(document);
const {
  mapHost,
  targetCursor,
  connectionStatus,
  messageList,
  combatSummaryList,
  turnValue,
  replayButton,
  saveButton,
  loadButton,
  clearMessages,
} = appDom;

const localization = new Localization("zh-CN", LOCALIZATION_SOURCES);
const playerUiLayout = new PlayerUiLayout({
  document,
  window,
  localization,
});
const itemCurseSeverityName = createItemCurseSeverityName(localization);
const {
  formatEvent,
  damageTypeName,
  contentName,
  visibleItemName,
  itemPropertyName,
  itemQualityName,
  equipmentSlotName,
  statusName,
} = createPresentationFormatter(
  localization,
  () => ({
    currentInventory: appState.inventory,
    currentEquipment: appState.equipment,
    bodySlots: appState.bodySlots,
    currentStatus: appState.status,
    currentWorldId: appState.worldId,
  }),
  {
    formatAttributeValueArgument,
    formatTenthsPoundArgument,
    itemCurseSeverityName,
  },
);
const MESSAGE_HISTORY_LIMIT = 500;
const messagePanel = new MessagePanel({
  list: messageList,
  localization,
  formatEvent,
  currentTurn: () => turnValue.textContent ?? "0",
  localizedArgs: localizedMessageArgs,
  historyLimit: MESSAGE_HISTORY_LIMIT,
});
const combatSummaryPanel = new CombatSummaryPanel({
  list: combatSummaryList,
  localization,
  formatEvent,
});
const addLocalizedMessage = (
  key: MessageKey,
  args: Record<string, string | number> | undefined,
  kind: string,
) => messagePanel.addLocalized(key, args, kind);
const addGameEvent = (event: GameEventDto) => messagePanel.addEvent(event);
let dispatch: (command: GameCommand) => Promise<void> = async () => {};
let mogaminatorEditor: MogaminatorEditor | undefined;
let promptedMogaminatorItemId: string | undefined;
const objectListPanel = new ObjectListPanel({
  document,
  window,
  state: appState,
  localization,
  contentName,
  visibleItemName,
  onTravel: (position) => void inputController.travelLocalTo(position),
  onCommand: (command) => void dispatch(command),
});
const monsterProbePanel = new MonsterProbePanel({
  state: appState,
  document,
  window,
  localization,
  contentName,
  damageTypeName,
  statusName,
});
const mapIntelligencePanel = new MapIntelligencePanel(appState, localization, document,
  () => settingsPanel.inputPreset, contentName, () => renderer.recenter());
document.getElementById("title-high-scores")!.addEventListener("click", () => { void highScorePanel.open(); });
document.getElementById("result-high-scores")!.addEventListener("click", () => { void highScorePanel.open(); });
const highScorePanel = new HighScorePanel(document, localization, () => nativeSaveStorage.scores());
const helpKnowledgePanel = new HelpKnowledgePanel(appState, localization, document,
  () => settingsPanel.inputPreset, openKnowledgeView, contentName, statusName);
const settingsPanel = new SettingsPanel({
  dom: appDom,
  state: appState,
  localization,
  renderer, document, preferences, rendererReady: () => rendererInitialized,
  beforeEdit: async () => {
    if (appState.mode === "playing") await inputController.prepareSessionAccess();
    await gameSession.whenIdle();
    inputController.cancelTargeting(false);
    configRecords.finishRecording();
    playerUiLayout.closePage();
  },
  openKeys: () => configRecords.open("input-config"),
  openMogaminator: () => mogaminatorEditor?.open(),
  download: downloadBytes,
  renderTargeting: () => inputController.render(),
  renderLocaleDependentUi: () => {
    renderConnectionStatus();
    if (appState.status) statusPanel.render(appState.status);
    inputController.render();
    renderContinuousAction();
    inventoryPanel.render(appState.inventory, appState.equipment);
    sessionShell.localize();
    journeyResult.localize();
    playerUiLayout.localize();
    shopPanel.localize();
    homePanel.localize();
    taskServicePanel.localize();
    objectListPanel.localize();
    monsterProbePanel.localize();
    mogaminatorEditor?.localize();
    messagePanel.render();
    combatSummaryPanel.localize();
    magicEaterPanel.render();
  },
  onBehaviorChange: async (p) => {
    if (await gameSession.dispatch({ type: "configure-preferences", preferences: behaviorPreferences(p) }) !== "applied") throw new Error("preferences-apply-failed");
  },
  announce: addLocalizedMessage,
});
const gameSession = new GameSession({
  state: appState,
  execute: (command) => core.dispatch(command),
  applyUpdate: (update, command) => {
    const mapResized = renderer.applyUpdate(update);
    if (mapResized) {
      appState.setMapSize(update.width, update.height);
      appState.replaceCells(update.changedCells);
      appState.replaceVisualCells(update.changedVisualCells);
    } else {
      appState.updateCells(update.changedCells);
      appState.updateVisualCells(update.changedVisualCells);
    }
    mapHost.dataset.mapScale = update.mapScale;
    statusPanel.render(update);
    if (update.player.pendingMaiaPathChoice) playerUiLayout.showMaiaChoice();
    objectListPanel.reconcileStatus();
    mapIntelligencePanel.reconcileStatus();
    helpKnowledgePanel.reconcileStatus();
    configRecords.reconcileStatus();
    inventoryPanel.render(update.inventory, update.equipment);
    shopPanel.render(update);
    homePanel.render(update);
    taskServicePanel.render(update);
    mogaminatorEditor?.render(update.mogaminator);
    promptMogaminatorQuery(update.mogaminator);
    monsterProbePanel.observe(update.events);
    combatSummaryPanel.observe(update.events, update.turn);
    for (const event of update.events) addGameEvent(event);
    journeyResult.renderUpdate(update);
    refreshSaveControls();
  },
  refreshBusyControls,
  showError,
  onRememberedCommand: command => configRecords.observe(command),
});
dispatch = async (command: GameCommand) => {
  try {
    if (inputController.continuousAction) await inputController.prepareSessionAccess();
    if (isAutomaticallyRepeatedCommand(command)) await inputController.dispatchCounted(command);
    else await gameSession.dispatch(command);
  } catch (error) { showError(error); }
};

function refreshSaveControls(): void {
  for (const id of ["save-button", "save-as-button", "save-exit-button", "load-button"]) {
    (document.getElementById(id) as HTMLButtonElement).disabled = appState.busy || appState.mode !== "playing";
  }
}
function refreshBusyControls(): void {
  refreshSaveControls();
  inventoryPanel.updateActions();
  shopPanel.updateActions();
  homePanel.updateActions();
  taskServicePanel.updateActions();
  inputController.render();
  renderContinuousAction();
  duelistPanel.render();
  spellRealmsPanel.render();
  magicEaterPanel.render();
  renderTravelOptions();
  statusPanel.updateAbilityActions();
}

function promptMogaminatorQuery(mogaminator: MogaminatorDto): void {
  const pending = mogaminator.pendingQuery;
  if (!pending) {
    promptedMogaminatorItemId = undefined;
    return;
  }
  if (promptedMogaminatorItemId === pending.itemId) return;
  void inputController.stopContinuousAction();
  promptedMogaminatorItemId = pending.itemId;
  const pickUp = window.confirm(
    localization.format("mogaminator-query-pick-up", {
      item: contentName(pending.itemKindId),
    }),
  );
  queueMicrotask(() => {
    void dispatch({
      type: "resolve-mogaminator-query",
      itemId: pending.itemId,
      pickUp,
    });
  });
}

async function reloadSavedMogaminator(): Promise<void> {
  if (appState.mode !== "playing" || appState.playerDead || appState.campaignEnded) throw new Error(localization.format("prf-reload-playing"));
  await inputController.prepareSessionAccess();
  await gameSession.whenIdle();
  const saved = await preferences.savedMogaminator();
  if (await gameSession.dispatch({ type: "configure-mogaminator-preferences", preferences: saved }) !== "applied") throw new Error("preferences-apply-failed");
  settingsPanel.noteMogaminatorApplied(saved);
}

mogaminatorEditor = new MogaminatorEditor({
  document,
  window,
  state: appState,
  localization,
  preferences,
  commit: (p, revision) => settingsPanel.commit(p, revision),
  reloadSaved: reloadSavedMogaminator,
});
const inputController = new InputController({
  state: appState,
  dom: appDom,
  localization,
  window,
  getInputPreset: () => settingsPanel.inputPreset,
  getZoom: () => settingsPanel.zoom,
  dispatch: (command, repeatCommand) => gameSession.dispatch(command, repeatCommand),
  getLastCommand: () => gameSession.lastCommand,
  confirmRepeat: command => {
    if (!inventoryPanel.confirmRepeatedCommand(command)) return false;
    if (command.type === "cast-ability") {
      const key = abilityConfirmationMessageKey(command.abilityId);
      if (key && !window.confirm(localization.format(key))) return false;
    }
    const target = "target" in command ? command.target : undefined;
    if (command.type === "destroy-item" || target?.type === "item" || target?.type.endsWith("-item")) {
      return window.confirm(localization.format("confirm-repeat-item-operation"));
    }
    return true;
  },
  whenIdle: () => gameSession.whenIdle(),
  onShortcut: handleCommandShortcut,
  customKey: (event, execute) => configRecords.handleBinding(event, execute),
  hasCustomKey: event => configRecords.hasBinding(event),
  chooseChest: (command, items, count) => inventoryPanel.selectChest(command, items, selected => inputController.dispatchCounted(selected, count)),
  onContinuousActionChange: renderContinuousAction,
  describeLook: describeLookPosition,
  openObjectList: () => objectListPanel.open(),
  openMogaminator: () => mogaminatorEditor?.open(),
  openDeviceCommand: key => magicEaterPanel.openDeviceCommand(key),
  onLookFocusChange: (position) => renderer.setCameraFocus(position),
  announce: addLocalizedMessage,
});
const configRecords = new ConfigRecords({
  state: appState, localization, document, storage: localStorage, preferences, preset: () => settingsPanel.inputPreset,
  saveBindings: async (keyBindings, revision) => {
    if (!preferences.snapshot) throw new Error("preferences-unavailable");
    await settingsPanel.commit({ ...preferences.snapshot.preferences, keyBindings }, revision);
  },
  execute: key => inputController.executeOriginalKey(key),
  repeat: command => inputController.repeatCommand(command), play: commands => inputController.playMacro(commands),
  lastCommand: () => gameSession.lastCommand,
  describe: () => document.querySelector("#message-list li:last-child")?.textContent ?? localization.format("cfg-recorded-command"),
  location: () => hudLocationText(appState.status!, localization, contentName),
  capturePng: () => renderer.capturePng(), download: downloadBytes,
  message: (key, args) => addLocalizedMessage(key, args, "system"), error: showError,
});
function renderContinuousAction(): void {
  const kind = inputController.continuousAction;
  appDom.stopContinuousAction.hidden = !kind;
  appDom.stopContinuousAction.disabled = false;
  appDom.stopContinuousAction.textContent = localization.format("action-stop-continuous");
  appDom.continuousActionStatus.textContent = kind ? localization.format(`continuous-action-${kind}`) : "";
}

function handleCommandShortcut(command: CommandShortcut, count?: number): void {
  if (command === "end-character") {
    if (!appState.status || appState.busy || appState.commandBlocked || inputController.continuousAction) return;
    if (!confirmCharacterEnd(appState.status.campaign.status === "victorious", key => localization.format(key), message => window.confirm(message), message => window.prompt(message))) return;
    configRecords.finishRecording();
    void dispatch({ type: "end-character" });
    return;
  }
  if (["input-config", "command-menu", "notes", "screen-export", "record-register", "play-register"].includes(command)) {
    if (!inputController.continuousAction) {
      if (!settingsPanel.close()) return;
      playerUiLayout.closePage();
      configRecords.open(command as "input-config" | "command-menu" | "notes" | "screen-export" | "record-register" | "play-register");
    }
    return;
  }
  if (command === "help" || command === "knowledge") {
    if (!inputController.continuousAction) helpKnowledgePanel.open(command);
    return;
  }
  if (command === "map-center") { renderer.recenter(); return; }
  if (["map-overview", "map-locate", "monster-list", "symbol-query", "floor-feeling"].includes(command)) {
    mapIntelligencePanel.open(command as MapInquiry); return;
  }
  if (command === "save" || command === "save-exit") {
    void nativeSavePanel.saveFromShortcut(
      undefined,
      command === "save-exit" ? () => currentWindow.destroy() : undefined,
    );
    return;
  }
  if (command === "glyphs" || command === "colors") { void settingsPanel.open(command); return; }
  if (command === "advanced-preferences") { void settingsPanel.open("advanced"); return; }
  if (command === "reload-pickup-rules") {
    void reloadSavedMogaminator().then(() => addLocalizedMessage("mogaminator-reloaded", undefined, "system"))
      .catch(error => addLocalizedMessage("mogaminator-preferences-error", { error: error instanceof Error ? error.message : String(error) }, "system"));
    return;
  }
  if (command === "settings") { void settingsPanel.open(); return; }
  if (command === "messages") { playerUiLayout.showMessages(); return; }
  if (command === "pets") { openPetMenu(); return; }
  if (command === "swap-rings") { inventoryPanel.swapRings(); return; }
  if (command === "character" || command === "tasks" || command === "inventory") { playerUiLayout.open(command); return; }
  if (command === "equipment") {
    playerUiLayout.open("inventory");
    appDom.equipmentList.tabIndex = -1;
    appDom.equipmentList.focus();
    appDom.equipmentList.scrollIntoView({ block: "nearest" });
    return;
  }
  if (command === "study" || command === "browse" || command === "power" || command === "cast") {
    if (command === "cast" && magicEaterPanel.openDeviceCommand("m")) return;
    playerUiLayout.open("ability");
    statusPanel.focusCommand(command);
    return;
  }
  playerUiLayout.open("inventory");
  inventoryPanel.openCommand(command as ItemShortcut, count);
}
const inventoryPanel = new InventoryPanel({
  dom: appDom,
  state: appState,
  localization,
  formatter: {
    visibleItemName,
    itemPropertyName,
    itemQualityName,
    equipmentSlotName,
    damageTypeName,
    statusName,
  },
  dispatch,
  startTargeting: (spec, intent) => {
    playerUiLayout.closePage();
    inputController.startTargetingWithSpec(spec, intent);
  },
  announce: addLocalizedMessage,
  itemCurseSeverityName,
});
const duelistPanel = new DuelistPanel({
  document, state: appState, localization, contentName, dispatch,
  startTargeting: ability => {
    playerUiLayout.closePage();
    inputController.startAbilityTargeting(ability);
  },
  beforePrompt: () => playerUiLayout.closePage(),
});
const magicEaterPanel = new MagicEaterPanel({
  document, state: appState, localization, dispatch, visibleItemName,
  inspectItem: id => inventoryPanel.openDetail(id),
  selectItemTarget: (ids, select, cancel, command) => inventoryPanel.selectItemTarget(undefined, select, cancel, ids, command),
  selectItemTargets: (excluded, select, cancel, command, multiple) => inventoryPanel.selectItemTargets(excluded, select, cancel, command, multiple),
  confirmItemChoice: (id, command) => inventoryPanel.confirmItemChoice(id, command),
  startTargeting: (spec, intent) => { playerUiLayout.closePage(); inputController.startTargetingWithSpec(spec, intent); },
  beforeOpen: () => { playerUiLayout.closePage(); inputController.cancelTargeting(false); },
  saveGame: async () => { await nativeSavePanel.saveFromShortcut(); },
  loadGame: () => { void openSaveList(); },
});
const petMenu = new PetMenu({
  document, state: appState, localization, dispatch, contentName,
  confirm: message => window.confirm(message),
  startRiding: () => inputController.startRiding(),
});
function openKnowledgeView(entry: KnowledgeEntry): void {
  if (entry === "scores") { void highScorePanel.open(); return; }
  switch (entry) {
    case "artifacts": case "objects": case "egos": playerUiLayout.open("inventory"); break;
    case "autopick": mogaminatorEditor?.open(); break;
    case "materials": case "mutations": case "virtues": case "extra": playerUiLayout.showCharacter("other"); break;
    case "self": playerUiLayout.showCharacter("overview"); break;
    case "weapon": case "shooter": playerUiLayout.showCharacter("details", "offense"); break;
    case "weapon-skills": playerUiLayout.showCharacter("proficiencies"); break;
    case "spell-skills": playerUiLayout.open("ability"); break;
    case "monsters": case "uniques": mapIntelligencePanel.open("symbol-query", entry === "uniques" ? "unique" : "all"); break;
    case "terrain": mapIntelligencePanel.open("symbol-query"); break;
    case "dungeons": mapIntelligencePanel.open("map-overview"); break;
    case "quests": playerUiLayout.open("tasks"); break;
  }
}

function openPetMenu(): void {
  if (inputController.continuousAction) return;
  playerUiLayout.closePage();
  petMenu.open();
}
const travelControls = {
  alwaysPickup: document.getElementById("travel-always-pickup") as HTMLInputElement,
  autoDetectTraps: document.getElementById("travel-auto-detect") as HTMLInputElement,
  autoMapArea: document.getElementById("travel-auto-map") as HTMLInputElement,
  disturbTrapDetect: document.getElementById("travel-disturb-detect") as HTMLInputElement,
};
function renderTravelOptions(): void {
  if ((document.getElementById("player-ui-settings-dialog") as HTMLDialogElement).open) return;
  const options = preferences.snapshot?.preferences.travel;
  for (const [key, control] of Object.entries(travelControls)) {
    control.disabled = !options;
    if (options) control.checked = options[key as keyof typeof options];
  }
}
const spellRealmsPanel = new SpellRealmsPanel({
  document, state: appState, localization, dispatch,
  afterPrompt: () => playerUiLayout.open("ability"),
});
const statusPanel = new StatusPanel({
  dom: appDom,
  state: appState,
  localization,
  dispatch,
  restUntilRecovered: () => { playerUiLayout.closePage(); return inputController.chooseRestMode(); },
  contentName,
  statusName,
  selectItemTarget: (excludedItemId, onSelect, allowedItemIds, command, onCancel) =>
    inventoryPanel.selectItemTarget(excludedItemId, onSelect, onCancel, allowedItemIds, command),
  confirmItemChoice: (id, command) => inventoryPanel.confirmItemChoice(id, command),
  startAbilityTargeting: (ability) => {
    playerUiLayout.closePage();
    inputController.startAbilityTargeting(ability);
  },
  reconcileTargeting: (state) => {
    inputController.reconcileStatus(state);
    duelistPanel.render();
    spellRealmsPanel.render();
    magicEaterPanel.render();
    renderTravelOptions();
  },
  renderTargeting: () => inputController.render(),
  refreshInventoryActions: () => inventoryPanel.updateActions(),
});
const shopPanel = new ShopPanel({
  document,
  state: appState,
  localization,
  dispatch,
  formatEvent,
  visibleItemName,
  contentName,
  beforeOpen: () => {
    playerUiLayout.closePage();
    inputController.cancelTargeting(false);
  },
});
const homePanel = new HomePanel({
  document,
  state: appState,
  localization,
  dispatch,
  formatEvent,
  visibleItemName,
  inspectItem: (itemId) => inventoryPanel.openDetail(itemId),
  refreshMuseum: async () => {
    if (appState.busy) return;
    appState.busy = true;
    refreshBusyControls();
    try { applyLoadedSnapshot(await core.refreshMuseum()); }
    catch (error) { showError(error); }
    finally { appState.busy = false; refreshBusyControls(); }
  },
  beforeOpen: () => {
    playerUiLayout.closePage();
    inputController.cancelTargeting(false);
  },
});
const taskServicePanel = new TaskServicePanel({
  document,
  state: appState,
  localization,
  dispatch,
  formatEvent,
  visibleItemName,
  contentName,
  statusName,
  beforeOpen: () => {
    playerUiLayout.closePage();
    inputController.cancelTargeting(false);
  },
});
let savedStateHash: string | undefined;
let activeCharacter = false;
const nativeSavePanel = new NativeSaveCommands({
  storage: nativeSaveStorage,
  isGameBusy: () => appState.busy,
  beforeSessionAccess: () => inputController.prepareSessionAccess(),
  setGameBusy: (value) => {
    appState.busy = value;
    refreshBusyControls();
  },
  announce: addLocalizedMessage,
  onSaved: () => { savedStateHash = appState.status?.stateHash; },
});
const sessionShell = new SessionShell({
  dom: sessionShellDom,
  storage: nativeSaveStorage,
  localization,
  onStart: startNewSession,
  onLoad: async (result, summary) => {
    await preferences.load();
    await settingsPanel.apply();
    await initializeGameView(result.snapshot);
    activeCharacter = true;
    savedStateHash = result.snapshot.stateHash;
    if (result.museumRecovered) {
      addLocalizedMessage("message-museum-character-recovered", {}, "system");
      return;
    }
    if (result.recoveryBackup === null) {
      addLocalizedMessage(
        "message-native-save-loaded",
        { name: summary.slotName },
        "system",
      );
    } else {
      addLocalizedMessage(
        "message-native-save-backup-loaded",
        { name: summary.slotName, backup: result.recoveryBackup },
        "system",
      );
    }
  },
  onExit: closeSessionWindow,
  beforeLoad: confirmSessionChange,
  onResume: () => { appState.mode = "playing"; refreshBusyControls(); },
  onOpenSettings: () => { void settingsPanel.open(); },
  confirm: (message) => window.confirm(message),
});
const journeyResult = new JourneyResult({
  dom: appDom,
  localization,
  formatEvent,
  currentSeed: () => sessionShell.restartRequest?.seed,
  canRestart: () => sessionShell.restartRequest !== undefined,
  onRestart: restartSameSetup,
  onNewGame: () => showSessionView("new-game"),
  onLoad: () => showSessionView("load"),
  onMenu: () => showSessionView("title"),
  onExit: closeSessionWindow,
});
playerUiLayout.initialize();
settingsPanel.initialize();
renderConnectionStatus();
inputController.render();
installFrontendCrashHandlers();
installRendererProfileHook();
installSupplyE2eHook();
void installNativeResizeSync();

void start();

async function start(): Promise<void> {
  appState.mode = "title";
  await sessionShell.initialize();
  try {
    await preferences.load(localStorage);
    await settingsPanel.apply();
    settingsPanel.initialize();
  } catch (error) {
    await settingsPanel.open();
    settingsPanel.showLoadError(error);
  }
  await refreshCrashDiagnosticStatus();
}

inputController.install();
appDom.stopContinuousAction.addEventListener("click", () => void inputController.stopContinuousAction());
void currentWindow.onCloseRequested(async event => {
  event.preventDefault();
  await closeSessionWindow();
});
settingsPanel.install();
statusPanel.install();
inventoryPanel.install();
shopPanel.install();
homePanel.install();
taskServicePanel.install();
objectListPanel.install();
for (const button of document.querySelectorAll<HTMLButtonElement>("[data-map-inquiry]")) {
  button.addEventListener("click", () => handleCommandShortcut(button.dataset.mapInquiry as CommandShortcut));
}
for (const button of document.querySelectorAll<HTMLButtonElement>("[data-guide]")) {
  button.addEventListener("click", () => handleCommandShortcut(button.dataset.guide as "help" | "knowledge"));
}
for (const button of document.querySelectorAll<HTMLButtonElement>("[data-config-record]")) {
  button.addEventListener("click", () => handleCommandShortcut(button.dataset.configRecord as CommandShortcut));
}
document.getElementById("command-record-status")!.addEventListener("click", () => configRecords.finishRecording());
monsterProbePanel.install();
mogaminatorEditor.install();
journeyResult.install();
playerUiLayout.install();
saveButton.addEventListener("click", () => void nativeSavePanel.saveFromShortcut());
document.getElementById("save-as-button")!.addEventListener("click", () => void nativeSavePanel.saveFromShortcut(
  () => window.prompt(localization.format("shortcut-save-name"), appState.status?.player.name ?? ""), undefined, true));
document.getElementById("save-exit-button")!.addEventListener("click", () => handleCommandShortcut("save-exit"));
replayButton.addEventListener("click", () => void exportReplay());
loadButton.addEventListener("click", () => void openSaveList());
sessionShell.install();
clearMessages.addEventListener("click", () => {
  messagePanel.clear();
});
window.addEventListener("beforeunload", () => {
  inventoryPanel.dispose();
  statusPanel.dispose();
  shopPanel.dispose();
  homePanel.dispose();
  taskServicePanel.dispose();
  objectListPanel.dispose();
  monsterProbePanel.dispose();
  mogaminatorEditor?.dispose();
  settingsPanel.dispose();
  inputController.dispose();
  journeyResult.dispose();
  playerUiLayout.dispose();
  sessionShell.dispose();
  renderer.destroy();
  core.dispose();
});

async function installNativeResizeSync(): Promise<void> {
  await currentWindow.onResized(() => {
    window.requestAnimationFrame(() => {
      window.requestAnimationFrame(() => {
        window.dispatchEvent(new Event("resize"));
        renderer.refreshLayout();
      });
    });
  });
}

function installFrontendCrashHandlers(): void {
  window.addEventListener("error", () => recordFrontendCrash("window-error"));
  window.addEventListener("unhandledrejection", () => recordFrontendCrash("unhandled-rejection"));
}

function installSupplyE2eHook(): void {
  window.__rfbPrepareSupplyE2e = async (amount: number): Promise<void> => {
    const snapshot = await core.prepareSupplyE2e(amount);
    applyLoadedSnapshot(snapshot);
  };
  window.__rfbPrepareLifeForceE2e = async (seed: number): Promise<void> => {
    applyLoadedSnapshot(await core.prepareLifeForceE2e(seed));
  };
}

function recordFrontendCrash(kind: "window-error" | "unhandled-rejection"): void {
  if (recordingFrontendCrash) return;
  recordingFrontendCrash = true;
  void crashDiagnostics
    .recordFrontendCrash(kind)
    .then(announceCrashDiagnostic)
    .catch((error: unknown) =>
      announceCrashDiagnosticError("Could not persist crash diagnostic", error),
    )
    .finally(() => {
      recordingFrontendCrash = false;
    });
}

async function synchronizeCrashDiagnosticContext(snapshot: GameSnapshot): Promise<void> {
  try {
    await crashDiagnostics.updateContext(
      snapshot.contentId,
      snapshot.contentHash,
      mapHost.dataset.rendererBackend ?? "unknown",
    );
  } catch (error) {
    announceCrashDiagnosticError("Could not update crash diagnostic context", error);
  }
}

async function refreshCrashDiagnosticStatus(): Promise<void> {
  try {
    announceCrashDiagnostic(await crashDiagnostics.status());
  } catch (error) {
    announceCrashDiagnosticError("Could not read crash diagnostic status", error);
  }
}

function announceCrashDiagnostic(status: CrashDiagnosticStatus): void {
  const fileName = status.reportFileName;
  if (!status.reportCreated || !fileName || announcedCrashReport === fileName) return;
  announcedCrashReport = fileName;
  document.documentElement.dataset.crashDiagnosticReport = fileName;
  document.documentElement.dataset.crashDiagnosticReason = status.reason ?? "unknown";
  addLocalizedMessage("message-crash-diagnostic-created", { file: fileName }, "system");
}

function announceCrashDiagnosticError(context: string, error: unknown): void {
  console.error(context, error);
  const code = desktopErrorCode(error);
  document.documentElement.dataset.crashDiagnosticError = code;
  if (announcedCrashDiagnosticErrors.has(code)) return;
  announcedCrashDiagnosticErrors.add(code);
  addLocalizedMessage("message-crash-diagnostic-unavailable", { code }, "error");
}


async function exportReplay(): Promise<void> {
  try {
    const bytes = await core.exportReplay();
    downloadBytes(bytes, "rfb-rewrite-diagnostic.rfbreplay");
    addLocalizedMessage("message-replay-exported", undefined, "system");
  } catch (error) {
    showError(error);
  }
}


function applyLoadedSnapshot(snapshot: GameSnapshot): void {
  mapIntelligencePanel.close();
  helpKnowledgePanel.close();
  inventoryPanel.reset();
  magicEaterPanel.reset();
  petMenu.close();
  inputController.resetSession();
  gameSession.resetCommandHistory();
  configRecords.reset();
  combatSummaryPanel.clear();
  objectListPanel.close();
  monsterProbePanel.close();
  mogaminatorEditor?.close();
  shopPanel.reset();
  homePanel.reset();
  taskServicePanel.reset();
  appState.mode = "playing";
  appState.setMapSize(snapshot.width, snapshot.height);
  appState.worldId = snapshot.worldId;
  appState.replaceCells(snapshot.cells);
  appState.replaceContentVisuals(snapshot.contentVisuals);
  core.synchronize(snapshot);
  renderContentMetadata(snapshot);
  renderer.applySnapshot(snapshot);
  appState.replaceVisualCells(snapshot.visualCells);
  statusPanel.render(snapshot);
  refreshSaveControls();
  inventoryPanel.render(snapshot.inventory, snapshot.equipment);
  shopPanel.render(snapshot);
  homePanel.render(snapshot);
  taskServicePanel.render(snapshot);
  mogaminatorEditor?.render(snapshot.mogaminator);
  promptMogaminatorQuery(snapshot.mogaminator);
  sessionShell.showGame(snapshot);
  if (snapshot.player.pendingMaiaPathChoice) playerUiLayout.showMaiaChoice();
  journeyResult.renderSnapshot(snapshot);
}

async function startNewSession(request: NewSessionRequest): Promise<GameSnapshot> {
  if (!preferences.snapshot) throw new Error(localization.format("preferences-unavailable"));
  await inputController.prepareSessionAccess();
  await preferences.load();
  await settingsPanel.apply();
  inventoryPanel.reset();
  petMenu.close();
  inputController.resetSession();
  gameSession.resetCommandHistory();
  configRecords.reset();
  appState.mode = "starting-session";
  appState.connection = "starting";
  renderConnectionStatus();
  try {
    const snapshot = await core.initialize(request);
    await initializeGameView(snapshot);
    activeCharacter = true;
    savedStateHash = undefined;
    addLocalizedMessage("message-core-started", undefined, "system");
    return snapshot;
  } catch (error) {
    appState.mode = "title";
    appState.connection = "error";
    throw error;
  }
}

async function initializeGameView(snapshot: GameSnapshot): Promise<void> {
  if (!preferences.snapshot) throw new Error(localization.format("preferences-unavailable"));
  mapIntelligencePanel.close();
  helpKnowledgePanel.close();
  inventoryPanel.reset();
  magicEaterPanel.reset();
  petMenu.close();
  inputController.resetSession();
  gameSession.resetCommandHistory();
  configRecords.reset();
  combatSummaryPanel.clear();
  objectListPanel.close();
  monsterProbePanel.close();
  mogaminatorEditor?.close();
  shopPanel.reset();
  homePanel.reset();
  taskServicePanel.reset();
  appState.setMapSize(snapshot.width, snapshot.height);
  appState.worldId = snapshot.worldId;
  appState.replaceCells(snapshot.cells);
  appState.replaceContentVisuals(snapshot.contentVisuals);
  core.synchronize(snapshot);
  renderContentMetadata(snapshot);
  if (!rendererInitialized) {
    const contentGlyphs = Object.fromEntries(
      snapshot.contentVisuals.map((visual) => [visual.id, visual.glyph]),
    );
    const tileset = await renderer.initialize(
      mapHost,
      snapshot.width,
      snapshot.height,
      settingsPanel.tilesetManifest,
      contentGlyphs,
      localization.format("map-aria-label"),
      settingsPanel.cameraMode,
      settingsPanel.zoom,
    );
    mapHost.append(targetCursor);
    rendererInitialized = true;
    settingsPanel.announceTileset(tileset.id, tileset.warnings);
  }
  renderer.applySnapshot(snapshot);
  appState.replaceVisualCells(snapshot.visualCells);
  await synchronizeCrashDiagnosticContext(snapshot);
  appState.mode = "playing";
  statusPanel.render(snapshot);
  refreshSaveControls();
  mogaminatorEditor?.render(snapshot.mogaminator);
  promptMogaminatorQuery(snapshot.mogaminator);
  inventoryPanel.render(snapshot.inventory, snapshot.equipment);
  shopPanel.render(snapshot);
  homePanel.render(snapshot);
  taskServicePanel.render(snapshot);
  journeyResult.renderSnapshot(snapshot);
  appState.connection = "ready";
  renderConnectionStatus();
  if (snapshot.player.pendingMaiaPathChoice) playerUiLayout.showMaiaChoice();
}

async function restartSameSetup(): Promise<void> {
  const request = sessionShell.restartRequest;
  if (!request) throw new Error(localization.format("result-restart-unavailable"));
  try {
    const snapshot = await startNewSession(request);
    sessionShell.showGame(snapshot, request);
  } catch (error) {
    appState.mode = "playing";
    throw error;
  }
}

let closingSession = false;
async function closeSessionWindow(): Promise<void> {
  if (closingSession) return;
  closingSession = true;
  try {
    await inputController.prepareSessionAccess();
    if (await confirmSessionChange()) await currentWindow.destroy();
  } catch (error) { showError(error); }
  finally { closingSession = false; }
}

async function confirmSessionChange(): Promise<boolean> {
  if (!activeCharacter || appState.playerDead || appState.campaignEnded || appState.status?.stateHash === savedStateHash) return true;
  const dialog = document.getElementById("save-decision-dialog") as HTMLDialogElement;
  if (dialog.open || appState.busy) return false;
  localization.localizeDocument(dialog);
  const choice = await new Promise<string>(resolve => {
    dialog.returnValue = "cancel";
    dialog.addEventListener("close", () => resolve(dialog.returnValue), { once: true });
    dialog.showModal();
  });
  if (choice === "save") return nativeSavePanel.saveFromShortcut();
  return choice === "discard";
}

async function openSaveList(): Promise<void> {
  try {
    await inputController.prepareSessionAccess();
    if (appState.busy) return;
    playerUiLayout.closePage();
    magicEaterPanel.reset();
    appState.mode = "title";
    sessionShell.showLoad(activeCharacter);
  } catch (error) { showError(error); }
}

async function showSessionView(view: "title" | "new-game" | "load"): Promise<void> {
  if (view === "load") { await openSaveList(); return; }
  try { await inputController.prepareSessionAccess(); } catch (error) { showError(error); return; }
  if (!(await confirmSessionChange())) return;
  activeCharacter = false;
  mapIntelligencePanel.close();
  helpKnowledgePanel.close();
  inventoryPanel.reset();
  petMenu.close();
  inputController.resetSession();
  gameSession.resetCommandHistory();
  configRecords.reset();
  objectListPanel.close();
  monsterProbePanel.close();
  mogaminatorEditor?.close();
  shopPanel.reset();
  homePanel.reset();
  taskServicePanel.reset();
  appState.mode = "title";
  switch (view) {
    case "title":
      sessionShell.showTitle();
      break;
    case "new-game":
      sessionShell.showNewGame(true);
      break;
  }
}

function describeLookPosition(position: { readonly x: number; readonly y: number }): string {
  const status = appState.status;
  if (!status) return localization.format("look-contents-empty");
  const cell = appState.cellAt(position);
  const withTerrain = (contents: string): string => {
    if (!cell) return contents;
    const terrain = localization.format("look-contents-with-terrain", {
          contents,
          terrain: contentName(cell.terrainId),
        });
    const danger = cell.dangerLevel;
    if (danger == null) return terrain;
    const locations = (cell.locations ?? []).map((location) => contentName(location.id));
    return localization.format(
      locations.length > 0 ? "look-contents-world-locations" : "look-contents-world",
      {
        contents: terrain,
        danger,
        locations: locations.join(localization.locale === "zh-CN" ? "、" : ", "),
      },
    );
  };
  if (
    status.player.position.x === position.x &&
    status.player.position.y === position.y
  ) {
    return withTerrain(localization.format("look-contents-player"));
  }
  if (appState.cellVisibility.get(`${position.x},${position.y}`) !== "visible") {
    return localization.format("look-contents-unseen");
  }
  const actor = status.entities.find(
    (entity) => entity.position.x === position.x && entity.position.y === position.y,
  );
  if (actor) {
    return withTerrain(
      localization.format("look-contents-actor", {
        actor: actor.customName ?? contentName(actor.kindId),
      }),
    );
  }
  const item = status.items.find(
    (candidate) =>
      candidate.position.x === position.x && candidate.position.y === position.y,
  );
  if (item) {
    return withTerrain(
      localization.format("look-contents-item", {
        item: visibleItemName(item.displayNameKey, item.kindId, item.artifactName),
      }),
    );
  }
  return withTerrain(localization.format("look-contents-empty"));
}

function renderContentMetadata(snapshot: GameSnapshot): void {
  mapHost.dataset.protocolVersion = snapshot.protocolVersion;
  mapHost.dataset.contentId = snapshot.contentId;
  mapHost.dataset.contentHash = snapshot.contentHash;
  mapHost.dataset.worldId = snapshot.worldId;
  mapHost.dataset.mapScale = snapshot.mapScale;
  mapHost.dataset.visualCellCount = String(snapshot.visualCells.length);
}

function formatAttributeValueArgument(value: string | undefined): string {
  if (value === undefined) return "?";
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) && parsed >= 3 ? formatAttributeValue(parsed) : "?";
}

function localizedMessageArgs(
  record: Extract<MessageRecord, { source: "key" }>,
): LocalizationArgs | undefined {
  if (!record.args) return undefined;
  if (record.key === "message-input-preset-changed") {
    const preset = String(record.args.preset);
    return {
      preset: isInputPreset(preset)
        ? localization.format(inputPresetMessageKey(preset))
        : preset,
    };
  }
  if (record.key === "message-native-save-failed") {
    const code = String(record.args.code);
    return {
      reason: localization.format(nativeSaveErrorKey(code), { code }),
    };
  }
  return record.args;
}

function showError(error: unknown): void {
  const message = error instanceof Error ? error.message : String(error);
  const code = message.split(":")[0] ?? "";
  if (code.startsWith("museum-")) {
    addLocalizedMessage(nativeSaveErrorKey(code), {}, "error");
    return;
  }
  appState.connection = "error";
  renderConnectionStatus();
  addLocalizedMessage("message-error", { error: message }, "error");
  console.error(error);
}

function downloadBytes(bytes: Uint8Array, fileName: string): void {
  const blob = new Blob([bytes.slice().buffer as ArrayBuffer], {
    type: "application/octet-stream",
  });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = fileName;
  anchor.click();
  URL.revokeObjectURL(url);
}

function renderConnectionStatus(): void {
  const keys: Record<ConnectionState, MessageKey> = {
    starting: "connection-starting",
    ready: "connection-ready",
    error: "connection-error",
  };
  connectionStatus.textContent = localization.format(keys[appState.connection]);
  connectionStatus.classList.toggle("ready", appState.connection === "ready");
  connectionStatus.classList.toggle("error", appState.connection === "error");
}
