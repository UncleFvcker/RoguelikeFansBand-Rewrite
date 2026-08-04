// SPDX-License-Identifier: MPL-2.0

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
import { NativeSavePanel, nativeSaveErrorKey } from "./save-panel";
import { createAppDom } from "./app-dom";
import { AppState, type ConnectionState } from "./app-state";
import type { NewSessionRequest } from "./core-transport";
import { InputController } from "./input-controller";
import { GameSession } from "./game-session";
import {
  SettingsPanel,
  inputPresetMessageKey,
  isInputPreset,
  readLocale,
} from "./settings-panel";
import { StatusPanel, formatAttributeValue } from "./status-panel";
import {
  InventoryPanel,
  createItemCurseSeverityName,
  formatTenthsPoundArgument,
} from "./inventory-panel";
import type { GameCommand, GameEventDto, GameSnapshot } from "./protocol";
import { TauriNativeTransport } from "./tauri-native-transport";
import { installRendererProfileHook } from "./render-profile";
import { createSessionShellDom, SessionShell } from "./session-shell";
import { JourneyGuidance } from "./journey-guidance";
import { JourneyResult } from "./journey-result";
import { PlayerUiLayout } from "./player-ui-layout";
import { ShopPanel } from "./shop-panel";
import { HomePanel } from "./home-panel";

const core = new TauriNativeTransport();
const crashDiagnostics = new DesktopCrashDiagnostics();
const nativeSaveStorage = new NativeSaveStorage();
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
  turnValue,
  nativeSaveName,
  nativeSaveCreate,
  nativeSaveRefresh,
  nativeSaveList,
  replayButton,
  saveButton,
  loadInput,
  clearMessages,
} = appDom;

const localization = new Localization(readLocale(localStorage), LOCALIZATION_SOURCES);
const playerUiLayout = new PlayerUiLayout({
  document,
  window,
  storage: localStorage,
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
const addLocalizedMessage = (
  key: MessageKey,
  args: Record<string, string | number> | undefined,
  kind: string,
) => messagePanel.addLocalized(key, args, kind);
const addGameEvent = (event: GameEventDto) => messagePanel.addEvent(event);
const journeyGuidance = new JourneyGuidance({
  dom: appDom,
  localization,
  storage: localStorage,
  getInputPreset: () => settingsPanel.inputPreset,
});
const settingsPanel = new SettingsPanel({
  dom: appDom,
  state: appState,
  localization,
  renderer,
  storage: localStorage,
  renderTargeting: () => inputController.render(),
  renderLocaleDependentUi: () => {
    renderConnectionStatus();
    if (appState.status) statusPanel.render(appState.status);
    inputController.render();
    inventoryPanel.render(appState.inventory, appState.equipment);
    nativeSavePanel.localize();
    sessionShell.localize();
    journeyGuidance.localize();
    journeyResult.localize();
    playerUiLayout.localize();
    shopPanel.localize();
    homePanel.localize();
    messagePanel.render();
  },
  refreshBusyControls: () => inventoryPanel.updateActions(),
  announce: addLocalizedMessage,
});
const gameSession = new GameSession({
  state: appState,
  execute: (command) => core.dispatch(command),
  applyUpdate: (update, command) => {
    const previous = appState.status;
    renderer.applyUpdate(update);
    appState.updateCells(update.changedCells);
    appState.updateVisualCells(update.changedVisualCells);
    statusPanel.render(update);
    inventoryPanel.render(update.inventory, update.equipment);
    shopPanel.render(update);
    homePanel.render(update);
    for (const event of update.events) addGameEvent(event);
    journeyGuidance.observeCommand(command, previous, update);
    journeyResult.renderUpdate(update);
  },
  refreshBusyControls: () => {
    inventoryPanel.updateActions();
    shopPanel.updateActions();
    homePanel.updateActions();
    inputController.render();
  },
  showError,
});
const dispatch = (command: GameCommand) => gameSession.dispatch(command);
const inputController = new InputController({
  state: appState,
  dom: appDom,
  localization,
  window,
  getInputPreset: () => settingsPanel.inputPreset,
  getZoom: () => settingsPanel.zoom,
  dispatch,
  describeLook: describeLookPosition,
  onLookOrTargeting: (interaction) => journeyGuidance.recordInteraction(interaction),
  announce: addLocalizedMessage,
});
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
  onInventoryInteraction: () => journeyGuidance.recordInteraction("inventory"),
  startTargeting: (spec, intent) => {
    playerUiLayout.closePage();
    inputController.startTargetingWithSpec(spec, intent);
  },
  updateCampaignAction: () => statusPanel.updateCampaignAction(),
  announce: addLocalizedMessage,
  itemCurseSeverityName,
});
const statusPanel = new StatusPanel({
  dom: appDom,
  state: appState,
  localization,
  dispatch,
  contentName,
  statusName,
  selectItemTarget: (excludedItemId, onSelect) =>
    inventoryPanel.selectItemTarget(excludedItemId, onSelect),
  startAbilityTargeting: (ability) => {
    playerUiLayout.closePage();
    inputController.startAbilityTargeting(ability);
  },
  reconcileTargeting: (state) => inputController.reconcileStatus(state),
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
  beforeOpen: () => {
    playerUiLayout.closePage();
    inputController.cancelTargeting(false);
  },
});
const nativeSavePanel = new NativeSavePanel({
  storage: nativeSaveStorage,
  localization,
  nameInput: nativeSaveName,
  createButton: nativeSaveCreate,
  refreshButton: nativeSaveRefresh,
  list: nativeSaveList,
  isGameBusy: () => appState.busy,
  setGameBusy: (value) => {
    appState.busy = value;
    inventoryPanel.updateActions();
  },
  applySnapshot: applyLoadedSnapshot,
  announce: addLocalizedMessage,
  confirm: (message) => window.confirm(message),
  onSaved: () => journeyGuidance.recordInteraction("save"),
});
const sessionShell = new SessionShell({
  dom: sessionShellDom,
  storage: nativeSaveStorage,
  localization,
  onStart: startNewSession,
  onLoad: async (result, summary) => {
    await initializeGameView(result.snapshot);
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
  onExit: () => getCurrentWindow().close(),
  onLocaleChange: (locale) => {
    appDom.languageSelect.value = locale;
    appDom.languageSelect.dispatchEvent(new Event("change", { bubbles: true }));
  },
  onInputPresetChange: (preset) => {
    appDom.inputPresetSelect.value = preset;
    appDom.inputPresetSelect.dispatchEvent(new Event("change", { bubbles: true }));
  },
  getInputPreset: () => settingsPanel.inputPreset,
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
  onExit: () => getCurrentWindow().close(),
});
playerUiLayout.initialize();
settingsPanel.initialize();
nativeSavePanel.localize();
renderConnectionStatus();
inputController.render();
installFrontendCrashHandlers();
installRendererProfileHook();
installSupplyE2eHook();

void start();

async function start(): Promise<void> {
  appState.mode = "title";
  await sessionShell.initialize();
  await refreshCrashDiagnosticStatus();
}

inputController.install();
settingsPanel.install();
statusPanel.install();
inventoryPanel.install();
shopPanel.install();
homePanel.install();
journeyGuidance.install();
journeyResult.install();
playerUiLayout.install();
saveButton.addEventListener("click", () => void exportSave());
replayButton.addEventListener("click", () => void exportReplay());
loadInput.addEventListener("change", () => void importSave());
nativeSavePanel.install();
sessionShell.install();
clearMessages.addEventListener("click", () => {
  messagePanel.clear();
});
window.addEventListener("beforeunload", () => {
  inventoryPanel.dispose();
  statusPanel.dispose();
  shopPanel.dispose();
  homePanel.dispose();
  settingsPanel.dispose();
  inputController.dispose();
  journeyGuidance.dispose();
  journeyResult.dispose();
  playerUiLayout.dispose();
  sessionShell.dispose();
  renderer.destroy();
  core.dispose();
});

function installFrontendCrashHandlers(): void {
  window.addEventListener("error", () => recordFrontendCrash("window-error"));
  window.addEventListener("unhandledrejection", () => recordFrontendCrash("unhandled-rejection"));
}

function installSupplyE2eHook(): void {
  window.__rfbPrepareSupplyE2e = async (amount: number): Promise<void> => {
    const snapshot = await core.prepareSupplyE2e(amount);
    applyLoadedSnapshot(snapshot);
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

async function exportSave(): Promise<void> {
  try {
    const bytes = await core.save();
    downloadBytes(bytes, "rfb-rewrite-demo.rfbsave");
    addLocalizedMessage("message-save-exported", undefined, "system");
  } catch (error) {
    showError(error);
  }
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

async function importSave(): Promise<void> {
  const file = loadInput.files?.[0];
  loadInput.value = "";
  if (!file) return;
  try {
    const snapshot = await core.load(new Uint8Array(await file.arrayBuffer()));
    applyLoadedSnapshot(snapshot);
    addLocalizedMessage("message-save-loaded", undefined, "system");
  } catch (error) {
    showError(error);
  }
}

function applyLoadedSnapshot(snapshot: GameSnapshot): void {
  inputController.cancelTargeting(false);
  shopPanel.reset();
  homePanel.reset();
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
  appState.bodySlots = snapshot.bodySlots ?? [];
  inventoryPanel.render(snapshot.inventory, snapshot.equipment);
  shopPanel.render(snapshot);
  homePanel.render(snapshot);
  journeyGuidance.render(snapshot);
  sessionShell.showGame(snapshot);
  journeyResult.renderSnapshot(snapshot);
}

async function startNewSession(request: NewSessionRequest): Promise<GameSnapshot> {
  appState.mode = "starting-session";
  appState.connection = "starting";
  renderConnectionStatus();
  try {
    const snapshot = await core.initialize(request);
    await initializeGameView(snapshot);
    addLocalizedMessage("message-core-started", undefined, "system");
    return snapshot;
  } catch (error) {
    appState.mode = "title";
    appState.connection = "error";
    throw error;
  }
}

async function initializeGameView(snapshot: GameSnapshot): Promise<void> {
  inputController.cancelTargeting(false);
  shopPanel.reset();
  homePanel.reset();
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
  appState.bodySlots = snapshot.bodySlots ?? [];
  inventoryPanel.render(snapshot.inventory, snapshot.equipment);
  shopPanel.render(snapshot);
  homePanel.render(snapshot);
  journeyGuidance.render(snapshot);
  journeyResult.renderSnapshot(snapshot);
  appState.connection = "ready";
  renderConnectionStatus();
  await nativeSavePanel.refresh();
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

function showSessionView(view: "title" | "new-game" | "load"): void {
  inputController.cancelTargeting(false);
  shopPanel.reset();
  homePanel.reset();
  appState.mode = "title";
  switch (view) {
    case "title":
      sessionShell.showTitle();
      break;
    case "new-game":
      sessionShell.showNewGame(true);
      break;
    case "load":
      sessionShell.showLoad();
      break;
  }
}

function describeLookPosition(position: { readonly x: number; readonly y: number }): string {
  const status = appState.status;
  if (!status) return localization.format("look-contents-empty");
  const cell = appState.cellAt(position);
  const withTerrain = (contents: string): string =>
    cell
      ? localization.format("look-contents-with-terrain", {
          contents,
          terrain: contentName(cell.terrainId),
        })
      : contents;
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
        actor: contentName(actor.kindId),
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
        item: visibleItemName(item.displayNameKey, item.kindId),
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
