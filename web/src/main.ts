// SPDX-License-Identifier: MPL-2.0

import "./styles.css";

import {
  Localization,
  isSupportedLocale,
  type LocalizationArgs,
  type MessageKey,
} from "./localization";
import { LOCALIZATION_SOURCES } from "./localization-resources";
import { MapRenderer, type CameraMode } from "./map-renderer";
import { MAP_CELL_SIZE, parseZoomLevel, type ZoomLevel } from "./camera";
import {
  DesktopCrashDiagnostics,
  type CrashDiagnosticStatus,
} from "./crash-diagnostics";
import {
  NativeSaveStorage,
  desktopErrorCode,
  type NativeSaveSummary,
} from "./native-save-storage";
import type {
  DamageResolutionDto,
  DamageTypeDto,
  Direction,
  EquipmentItemDto,
  GameCommand,
  GameEventDto,
  GameSnapshot,
  GameUpdate,
  InventoryItemDto,
  StatModifiersDto,
} from "./protocol";
import { TauriNativeTransport } from "./tauri-native-transport";
import type { TilesetWarning } from "./tileset-runtime";
import { installRendererProfileHook } from "./render-profile";
import {
  beginTargeting,
  moveTargetCursor,
  targetSelectionAtCursor,
  type TargetingState,
} from "./targeting";

const core = new TauriNativeTransport();
const crashDiagnostics = new DesktopCrashDiagnostics();
const nativeSaveStorage = new NativeSaveStorage();
const renderer = new MapRenderer();
let busy = false;
let playerDead = false;
let nativeSaveBusy = false;
let recordingFrontendCrash = false;
let announcedCrashReport: string | undefined;
let dropQuantityItemId: string | undefined;
let targeting: TargetingState | undefined;
let mapWidth = 0;
let mapHeight = 0;

const mapHost = element<HTMLElement>("map-host");
const targetCursor = element<HTMLElement>("target-cursor");
const targetModeToggle = element<HTMLButtonElement>("target-mode-toggle");
const targetModeStatus = element<HTMLElement>("target-mode-status");
const connectionStatus = element<HTMLElement>("connection-status");
const messageList = element<HTMLOListElement>("message-list");
const turnValue = element<HTMLElement>("turn-value");
const hpValue = element<HTMLElement>("hp-value");
const attackValue = element<HTMLElement>("attack-value");
const defenseValue = element<HTMLElement>("defense-value");
const effectsValue = element<HTMLElement>("effects-value");
const positionValue = element<HTMLElement>("position-value");
const hashValue = element<HTMLElement>("hash-value");
const inventoryCount = element<HTMLElement>("inventory-count");
const inventorySelectionCount = element<HTMLElement>("inventory-selection-count");
const inventoryUse = element<HTMLButtonElement>("inventory-use");
const inventoryEquip = element<HTMLButtonElement>("inventory-equip");
const inventoryDrop = element<HTMLButtonElement>("inventory-drop");
const inventoryDropQuantity = element<HTMLInputElement>("inventory-drop-quantity");
const inventoryList = element<HTMLUListElement>("inventory-list");
const equipmentList = element<HTMLUListElement>("equipment-list");
const nativeSaveName = element<HTMLInputElement>("native-save-name");
const nativeSaveCreate = element<HTMLButtonElement>("native-save-create");
const nativeSaveRefresh = element<HTMLButtonElement>("native-save-refresh");
const nativeSaveList = element<HTMLUListElement>("native-save-list");
const replayButton = element<HTMLButtonElement>("replay-button");
const saveButton = element<HTMLButtonElement>("save-button");
const loadInput = element<HTMLInputElement>("load-input");
const clearMessages = element<HTMLButtonElement>("clear-messages");
const inputPresetSelect = element<HTMLSelectElement>("input-preset");
const tilesetPresetSelect = element<HTMLSelectElement>("tileset-preset");
const cameraModeSelect = element<HTMLSelectElement>("camera-mode");
const zoomSelect = element<HTMLSelectElement>("zoom-level");
const controlsHelp = element<HTMLElement>("controls-help");
const languageSelect = element<HTMLSelectElement>("language-select");

type InputPreset = "numpad" | "vi" | "wasd";
type TilesetPreset = "ascii" | "image";
type ConnectionState = "starting" | "ready" | "error";
type MessageRecord =
  | {
      source: "key";
      turn: string;
      kind: string;
      key: MessageKey;
      args?: Record<string, string | number>;
    }
  | { source: "event"; turn: string; kind: string; event: GameEventDto };
const INPUT_PRESET_STORAGE_KEY = "rfb.input-preset";
const TILESET_PRESET_STORAGE_KEY = "rfb.tileset-preset";
const CAMERA_MODE_STORAGE_KEY = "rfb.camera-mode";
const ZOOM_STORAGE_KEY = "rfb.zoom";
const LOCALE_STORAGE_KEY = "rfb.locale";
const TILESET_MANIFESTS: Record<TilesetPreset, string> = {
  ascii: "/tilesets/ascii-default/tileset.json",
  image: "/tilesets/image-demo/tileset.json",
};
let inputPreset = readInputPreset();
let tilesetPreset = readTilesetPreset();
let cameraMode = readCameraMode();
let zoom = readZoomLevel();
const localization = new Localization(readLocale(), LOCALIZATION_SOURCES);
let connectionState: ConnectionState = "starting";
let currentStatus: GameSnapshot | GameUpdate | undefined;
let currentInventory: InventoryItemDto[] = [];
let currentEquipment: EquipmentItemDto[] = [];
const selectedInventoryIds = new Set<string>();
let nativeSaves: NativeSaveSummary[] = [];
const messageRecords: MessageRecord[] = [];
inputPresetSelect.value = inputPreset;
tilesetPresetSelect.value = tilesetPreset;
cameraModeSelect.value = cameraMode;
zoomSelect.value = String(zoom);
languageSelect.value = localization.locale;
localization.localizeDocument();
localizeNativeSaveControls();
renderConnectionStatus();
renderInputHelp();
renderTargeting();
renderNativeSaves();
installFrontendCrashHandlers();
installRendererProfileHook();

void start();

async function start(): Promise<void> {
  try {
    const snapshot = await core.initialize("42");
    mapWidth = snapshot.width;
    mapHeight = snapshot.height;
    const contentGlyphs = Object.fromEntries(
      snapshot.contentVisuals.map((visual) => [visual.id, visual.glyph]),
    );
    renderContentMetadata(snapshot);
    const tileset = await renderer.initialize(
      mapHost,
      snapshot.width,
      snapshot.height,
      TILESET_MANIFESTS[tilesetPreset],
      contentGlyphs,
      localization.format("map-aria-label"),
      cameraMode,
      zoom,
    );
    mapHost.append(targetCursor);
    renderer.applySnapshot(snapshot);
    await synchronizeCrashDiagnosticContext(snapshot);
    renderStatus(snapshot);
    renderInventory(snapshot.inventory, snapshot.equipment);
    addLocalizedMessage("message-core-started", undefined, "system");
    announceTileset(tileset.id, tileset.warnings);
    connectionState = "ready";
    renderConnectionStatus();
    await refreshCrashDiagnosticStatus();
    await refreshNativeSaves();
  } catch (error) {
    showError(error);
  }
}

window.addEventListener("keydown", (event) => {
  if (busy || playerDead || isTextInput(event.target)) return;
  if (targeting) {
    if (event.key === "Escape") {
      event.preventDefault();
      cancelTargeting();
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      void confirmTargeting();
      return;
    }
    const direction = directionForKeyboardEvent(event);
    if (direction) {
      event.preventDefault();
      targeting = moveTargetCursor(targeting, direction, mapWidth, mapHeight);
      renderTargeting();
    }
    return;
  }
  if (event.key.toLowerCase() === "f") {
    event.preventDefault();
    startTargeting();
    return;
  }
  const command = commandForKeyboardEvent(event);
  if (command) {
    event.preventDefault();
    void dispatch(command);
  }
});

saveButton.addEventListener("click", () => void exportSave());
replayButton.addEventListener("click", () => void exportReplay());
loadInput.addEventListener("change", () => void importSave());
nativeSaveCreate.addEventListener("click", () => void createNativeSave());
nativeSaveRefresh.addEventListener("click", () => void refreshNativeSaves());
nativeSaveName.addEventListener("input", updateNativeSaveControls);
inventoryUse.addEventListener("click", () => void useSelectedInventoryItem());
inventoryEquip.addEventListener("click", () => void equipSelectedInventoryItem());
inventoryDrop.addEventListener("click", () => void dropSelectedInventoryItems());
inventoryDropQuantity.addEventListener("input", updateInventoryActions);
targetModeToggle.addEventListener("click", () => {
  if (targeting) cancelTargeting();
  else startTargeting();
});
clearMessages.addEventListener("click", () => {
  messageRecords.length = 0;
  renderMessages();
});
inputPresetSelect.addEventListener("change", () => {
  inputPreset = isInputPreset(inputPresetSelect.value) ? inputPresetSelect.value : "numpad";
  localStorage.setItem(INPUT_PRESET_STORAGE_KEY, inputPreset);
  renderInputHelp();
  addLocalizedMessage("message-input-preset-changed", { preset: inputPreset }, "system");
});
tilesetPresetSelect.addEventListener("change", () => void changeTileset());
cameraModeSelect.addEventListener("change", () => {
  cameraMode = isCameraMode(cameraModeSelect.value) ? cameraModeSelect.value : "full-map";
  localStorage.setItem(CAMERA_MODE_STORAGE_KEY, cameraMode);
  renderer.setCameraMode(cameraMode);
  renderTargeting();
});
zoomSelect.addEventListener("change", () => {
  zoom = parseZoomLevel(zoomSelect.value);
  zoomSelect.value = String(zoom);
  localStorage.setItem(ZOOM_STORAGE_KEY, String(zoom));
  renderer.setZoom(zoom);
  renderTargeting();
});
languageSelect.addEventListener("change", () => {
  const locale = isSupportedLocale(languageSelect.value) ? languageSelect.value : "zh-CN";
  localization.setLocale(locale);
  localStorage.setItem(LOCALE_STORAGE_KEY, locale);
  localization.localizeDocument();
  languageSelect.value = locale;
  renderer.setCanvasLabel(localization.format("map-aria-label"));
  renderConnectionStatus();
  if (currentStatus) renderStatus(currentStatus);
  renderInputHelp();
  renderTargeting();
  renderInventory(currentInventory, currentEquipment);
  localizeNativeSaveControls();
  renderNativeSaves();
  renderMessages();
});
window.addEventListener("beforeunload", () => {
  renderer.destroy();
  core.dispose();
});
window.addEventListener("resize", () => requestAnimationFrame(renderTargeting));

function installFrontendCrashHandlers(): void {
  window.addEventListener("error", () => recordFrontendCrash("window-error"));
  window.addEventListener("unhandledrejection", () => recordFrontendCrash("unhandled-rejection"));
}

function recordFrontendCrash(kind: "window-error" | "unhandled-rejection"): void {
  if (recordingFrontendCrash) return;
  recordingFrontendCrash = true;
  void crashDiagnostics
    .recordFrontendCrash(kind)
    .then(announceCrashDiagnostic)
    .catch((error: unknown) => console.error("Could not persist crash diagnostic", error))
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
    console.error("Could not update crash diagnostic context", error);
  }
}

async function refreshCrashDiagnosticStatus(): Promise<void> {
  try {
    announceCrashDiagnostic(await crashDiagnostics.status());
  } catch (error) {
    console.error("Could not read crash diagnostic status", error);
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

async function dispatch(command: GameCommand): Promise<void> {
  if (playerDead) return;
  busy = true;
  updateInventoryActions();
  renderTargeting();
  try {
    const update = await core.dispatch(command);
    renderer.applyUpdate(update);
    renderStatus(update);
    renderInventory(update.inventory, update.equipment);
    for (const event of update.events) addGameEvent(event);
  } catch (error) {
    showError(error);
  } finally {
    busy = false;
    updateInventoryActions();
    renderTargeting();
  }
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

async function refreshNativeSaves(): Promise<void> {
  if (nativeSaveBusy) return;
  nativeSaveBusy = true;
  updateNativeSaveControls();
  try {
    nativeSaves = await nativeSaveStorage.list();
    renderNativeSaves();
  } catch (error) {
    showNativeSaveError(error);
  } finally {
    nativeSaveBusy = false;
    updateNativeSaveControls();
  }
}

async function createNativeSave(): Promise<void> {
  const slotName = nativeSaveName.value.trim();
  if (nativeSaveBusy || !slotName) return;
  nativeSaveBusy = true;
  updateNativeSaveControls();
  try {
    const summary = await nativeSaveStorage.save(slotName);
    nativeSaveName.value = "";
    replaceNativeSaveSummary(summary);
    addLocalizedMessage("message-native-save-created", { name: summary.slotName }, "system");
  } catch (error) {
    showNativeSaveError(error);
  } finally {
    nativeSaveBusy = false;
    updateNativeSaveControls();
  }
}

async function overwriteNativeSave(summary: NativeSaveSummary): Promise<void> {
  if (nativeSaveBusy) return;
  nativeSaveBusy = true;
  updateNativeSaveControls();
  try {
    const updated = await nativeSaveStorage.save(summary.slotName, summary.slotId);
    replaceNativeSaveSummary(updated);
    addLocalizedMessage("message-native-save-overwritten", { name: updated.slotName }, "system");
  } catch (error) {
    showNativeSaveError(error);
  } finally {
    nativeSaveBusy = false;
    updateNativeSaveControls();
  }
}

async function loadNativeSave(summary: NativeSaveSummary): Promise<void> {
  if (nativeSaveBusy || busy || summary.status === "corrupt") return;
  nativeSaveBusy = true;
  busy = true;
  updateNativeSaveControls();
  updateInventoryActions();
  try {
    const result = await nativeSaveStorage.load(summary.slotId);
    applyLoadedSnapshot(result.snapshot);
    if (result.recoveryBackup === null) {
      addLocalizedMessage("message-native-save-loaded", { name: summary.slotName }, "system");
    } else {
      addLocalizedMessage(
        "message-native-save-backup-loaded",
        { name: summary.slotName, backup: result.recoveryBackup },
        "system",
      );
    }
    await refreshNativeSavesAfterOperation();
  } catch (error) {
    showNativeSaveError(error);
  } finally {
    busy = false;
    nativeSaveBusy = false;
    updateNativeSaveControls();
    updateInventoryActions();
  }
}

async function deleteNativeSave(summary: NativeSaveSummary): Promise<void> {
  if (
    nativeSaveBusy ||
    !window.confirm(localization.format("confirm-native-save-delete", { name: summary.slotName }))
  ) {
    return;
  }
  nativeSaveBusy = true;
  updateNativeSaveControls();
  try {
    await nativeSaveStorage.delete(summary.slotId);
    nativeSaves = nativeSaves.filter((save) => save.slotId !== summary.slotId);
    renderNativeSaves();
    addLocalizedMessage("message-native-save-deleted", { name: summary.slotName }, "system");
  } catch (error) {
    showNativeSaveError(error);
  } finally {
    nativeSaveBusy = false;
    updateNativeSaveControls();
  }
}

function applyLoadedSnapshot(snapshot: GameSnapshot): void {
  cancelTargeting(false);
  mapWidth = snapshot.width;
  mapHeight = snapshot.height;
  core.synchronize(snapshot);
  renderContentMetadata(snapshot);
  renderer.applySnapshot(snapshot);
  renderStatus(snapshot);
  renderInventory(snapshot.inventory, snapshot.equipment);
}

function replaceNativeSaveSummary(summary: NativeSaveSummary): void {
  nativeSaves = [summary, ...nativeSaves.filter((save) => save.slotId !== summary.slotId)];
  renderNativeSaves();
}

async function refreshNativeSavesAfterOperation(): Promise<void> {
  nativeSaves = await nativeSaveStorage.list();
  renderNativeSaves();
}

function renderNativeSaves(): void {
  nativeSaveList.replaceChildren();
  if (nativeSaves.length === 0) {
    const empty = document.createElement("li");
    empty.className = "native-save-empty";
    empty.textContent = localization.format("native-save-empty");
    nativeSaveList.append(empty);
    updateNativeSaveControls();
    return;
  }

  for (const summary of nativeSaves) {
    const row = document.createElement("li");
    row.className = "native-save-item";
    row.dataset.slotId = summary.slotId;

    const header = document.createElement("div");
    header.className = "native-save-header";
    const name = document.createElement("span");
    name.className = "native-save-name";
    name.textContent = summary.slotName;
    name.title = summary.slotName;
    const status = document.createElement("span");
    status.className = `native-save-status native-save-status-${summary.status}`;
    status.textContent = localization.format(nativeSaveStatusKey(summary.status));
    header.append(name, status);

    const metadata = document.createElement("p");
    metadata.className = "native-save-meta";
    metadata.textContent = nativeSaveMetadata(summary);

    const actions = document.createElement("div");
    actions.className = "native-save-actions";
    const load = nativeSaveActionButton("load", "action-native-save-load", () =>
      void loadNativeSave(summary),
    );
    load.disabled = summary.status === "corrupt" || nativeSaveBusy || busy;
    const overwrite = nativeSaveActionButton(
      "overwrite",
      "action-native-save-overwrite",
      () => void overwriteNativeSave(summary),
    );
    overwrite.disabled = nativeSaveBusy;
    const remove = nativeSaveActionButton("delete", "action-native-save-delete", () =>
      void deleteNativeSave(summary),
    );
    remove.disabled = nativeSaveBusy;
    actions.append(load, overwrite, remove);

    row.append(header, metadata, actions);
    nativeSaveList.append(row);
  }
  updateNativeSaveControls();
}

function nativeSaveActionButton(
  actionName: string,
  key: MessageKey,
  action: () => void,
): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.dataset.nativeSaveAction = actionName;
  button.textContent = localization.format(key);
  button.addEventListener("click", action);
  return button;
}

function nativeSaveStatusKey(status: NativeSaveSummary["status"]): MessageKey {
  const keys: Record<NativeSaveSummary["status"], MessageKey> = {
    ready: "native-save-status-ready",
    recoverable: "native-save-status-recoverable",
    corrupt: "native-save-status-corrupt",
  };
  return keys[status];
}

function nativeSaveMetadata(summary: NativeSaveSummary): string {
  if (summary.turn === null || summary.savedAt === null) {
    return localization.format("native-save-meta-unavailable");
  }
  return localization.format("native-save-meta", {
    location: nativeSaveLocation(summary.locationKey),
    turn: summary.turn,
    savedAt: nativeSaveDate(summary.savedAt),
  });
}

function nativeSaveLocation(locationKey: string | null): string {
  return locationKey === "world-demo-original-lab-name"
    ? localization.format("world-demo-original-lab-name")
    : localization.format("native-save-location-unknown");
}

function nativeSaveDate(savedAt: string): string {
  const date = new Date(savedAt);
  return Number.isNaN(date.getTime())
    ? localization.format("native-save-date-unknown")
    : new Intl.DateTimeFormat(localization.locale, {
        dateStyle: "short",
        timeStyle: "short",
      }).format(date);
}

function localizeNativeSaveControls(): void {
  nativeSaveName.placeholder = localization.format("native-save-name-placeholder");
  updateNativeSaveControls();
}

function updateNativeSaveControls(): void {
  nativeSaveName.disabled = nativeSaveBusy;
  nativeSaveCreate.disabled = nativeSaveBusy || nativeSaveName.value.trim().length === 0;
  nativeSaveRefresh.disabled = nativeSaveBusy;
  for (const button of nativeSaveList.querySelectorAll<HTMLButtonElement>("button")) {
    const row = button.closest<HTMLElement>(".native-save-item");
    const summary = nativeSaves.find((save) => save.slotId === row?.dataset.slotId);
    button.disabled =
      nativeSaveBusy ||
      (button.dataset.nativeSaveAction === "load" &&
        (busy || summary?.status === "corrupt"));
  }
}

function showNativeSaveError(error: unknown): void {
  addLocalizedMessage(
    "message-native-save-failed",
    { code: desktopErrorCode(error) },
    "error",
  );
  console.error(error);
}

function nativeSaveErrorKey(code: string): MessageKey {
  if (code === "native-save-name-invalid") return "native-save-error-name-invalid";
  if (code === "native-save-not-found") return "native-save-error-not-found";
  if (code === "native-save-invalid") return "native-save-error-corrupt";
  return "native-save-error-unavailable";
}

async function changeTileset(): Promise<void> {
  const requested = isTilesetPreset(tilesetPresetSelect.value)
    ? tilesetPresetSelect.value
    : "ascii";
  if (requested === tilesetPreset || busy) {
    tilesetPresetSelect.value = tilesetPreset;
    return;
  }
  busy = true;
  updateInventoryActions();
  try {
    const result = await renderer.setTileset(TILESET_MANIFESTS[requested]);
    tilesetPreset = requested;
    localStorage.setItem(TILESET_PRESET_STORAGE_KEY, tilesetPreset);
    announceTileset(result.id, result.warnings);
  } catch (error) {
    tilesetPresetSelect.value = tilesetPreset;
    const message = error instanceof Error ? error.message : String(error);
    addLocalizedMessage("message-tileset-load-failed", { error: message }, "error");
    console.error(error);
  } finally {
    busy = false;
    updateInventoryActions();
  }
}

function renderStatus(state: GameSnapshot | GameUpdate): void {
  currentStatus = state;
  playerDead = state.player.isDead;
  if (
    targeting &&
    (targeting.origin.x !== state.player.position.x ||
      targeting.origin.y !== state.player.position.y ||
      !state.player.projectileProfile)
  ) {
    cancelTargeting(false);
  }
  document.documentElement.dataset.playerState = playerDead ? "dead" : "alive";
  turnValue.textContent = String(state.turn);
  hpValue.textContent = localization.format(
    state.player.equipmentModifiers.maxHp > 0
      ? "status-health-value-bonus"
      : "status-health-value",
    {
      hp: state.player.hp,
      maxHp: state.player.maxHp,
      bonus: state.player.equipmentModifiers.maxHp,
    },
  );
  renderCombatStat(attackValue, state.player.attack, state.player.equipmentModifiers.attack);
  renderCombatStat(
    defenseValue,
    state.player.defense,
    state.player.equipmentModifiers.defense,
  );
  effectsValue.textContent =
    state.player.statuses.length === 0
      ? localization.format("status-effects-none")
      : state.player.statuses
          .map((status) =>
            localization.format("status-effect-entry", {
              status: statusName(status.kindId),
              intensity: status.intensity,
              ticks: status.remainingTicks,
            }),
          )
          .join(" · ");
  positionValue.textContent = `${state.player.position.x}, ${state.player.position.y}`;
  hashValue.textContent = state.stateHash.slice(0, 12);
  hashValue.title = state.stateHash;
  mapHost.dataset.itemCount = String(state.items.length);
  mapHost.dataset.inventoryStackCount = String(state.inventory.length);
  mapHost.dataset.equipmentCount = String(state.equipment.length);
  mapHost.dataset.carriedWeightTenthsPound = String(
    state.player.carriedWeightTenthsPound,
  );
  mapHost.dataset.carryCapacityTenthsPound = String(
    state.player.carryCapacityTenthsPound,
  );
  mapHost.dataset.playerStatusCount = String(state.player.statuses.length);
  updateInventoryActions();
  renderTargeting();
}

function renderContentMetadata(snapshot: GameSnapshot): void {
  mapHost.dataset.protocolVersion = snapshot.protocolVersion;
  mapHost.dataset.contentId = snapshot.contentId;
  mapHost.dataset.contentHash = snapshot.contentHash;
  mapHost.dataset.worldId = snapshot.worldId;
  mapHost.dataset.contentVisualCount = String(snapshot.contentVisuals.length);
  mapHost.dataset.visualCellCount = String(snapshot.visualCells.length);
}

function renderInventory(
  inventory: InventoryItemDto[],
  equipment: EquipmentItemDto[],
): void {
  currentInventory = inventory.map((item) => ({ ...item }));
  currentEquipment = equipment.map((item) => ({ ...item }));
  const availableIds = new Set(inventory.map((item) => item.id));
  for (const itemId of selectedInventoryIds) {
    if (!availableIds.has(itemId)) selectedInventoryIds.delete(itemId);
  }
  const stacks = localization.format("inventory-stack-count", {
    count: inventory.length,
  });
  inventoryCount.textContent = currentStatus
    ? localization.format("inventory-weight-summary", {
        stacks,
        weight: formatTenthsPound(currentStatus.player.carriedWeightTenthsPound),
        capacity: formatTenthsPound(currentStatus.player.carryCapacityTenthsPound),
      })
    : stacks;
  inventoryList.replaceChildren();
  if (inventory.length === 0) {
    const empty = document.createElement("li");
    empty.className = "inventory-empty";
    empty.textContent = localization.format("inventory-empty");
    inventoryList.append(empty);
  } else {
    for (const item of inventory) {
      const row = document.createElement("li");
      row.className = "inventory-item";
      row.dataset.itemId = item.id;
      const label = document.createElement("label");
      const checkbox = document.createElement("input");
      checkbox.type = "checkbox";
      checkbox.checked = selectedInventoryIds.has(item.id);
      checkbox.addEventListener("change", () => {
        if (checkbox.checked) selectedInventoryIds.add(item.id);
        else selectedInventoryIds.delete(item.id);
        updateInventoryActions();
      });
      const details = document.createElement("span");
      details.className = "inventory-item-details";
      const name = document.createElement("span");
      name.className = "inventory-item-name";
      name.textContent = visibleItemName(item.displayNameKey, item.kindId);
      details.append(name);
      if (item.equipmentSlot) {
        const equippable = document.createElement("span");
        equippable.className = "inventory-equippable";
        equippable.textContent = localization.format("inventory-equippable", {
          slot: equipmentSlotName(item.equipmentSlot),
        });
        details.append(equippable);
      }
      appendItemModifiers(details, item.modifiers);
      const quantity = document.createElement("span");
      quantity.className = "inventory-quantity";
      quantity.textContent = localization.format("inventory-quantity", {
        quantity: item.quantity,
      });
      label.append(checkbox, details, quantity);
      row.append(label);
      inventoryList.append(row);
    }
  }
  renderEquipment(equipment);
  updateInventoryActions();
}

function renderEquipment(equipment: EquipmentItemDto[]): void {
  equipmentList.replaceChildren();
  if (equipment.length === 0) {
    const empty = document.createElement("li");
    empty.className = "equipment-empty";
    empty.textContent = localization.format("equipment-empty");
    equipmentList.append(empty);
    return;
  }
  for (const item of equipment) {
    const row = document.createElement("li");
    row.className = "equipment-item";
    row.dataset.slotId = item.slotId;
    const details = document.createElement("span");
    details.className = "equipment-item-details";
    const name = document.createElement("span");
    name.textContent = visibleItemName(item.displayNameKey, item.kindId);
    const slot = document.createElement("span");
    slot.className = "equipment-slot";
    slot.textContent = equipmentSlotName(item.slotId);
    details.append(name, slot);
    appendItemModifiers(details, item.modifiers);
    const unequip = document.createElement("button");
    unequip.type = "button";
    unequip.textContent = localization.format("action-equipment-unequip");
    unequip.disabled = busy;
    unequip.addEventListener("click", () => void unequipItem(item.slotId));
    row.append(details, unequip);
    equipmentList.append(row);
  }
}

async function equipSelectedInventoryItem(): Promise<void> {
  const selected = selectedInventoryItems();
  if (busy || selected.length !== 1 || !selected[0]?.equipmentSlot) return;
  await dispatch({ type: "equip", itemId: selected[0].id });
}

async function useSelectedInventoryItem(): Promise<void> {
  const selected = selectedInventoryItems();
  if (busy || selected.length !== 1 || !selected[0]?.usable) return;
  await dispatch({ type: "use-item", itemId: selected[0].id });
}

async function dropSelectedInventoryItems(): Promise<void> {
  const selected = selectedInventoryItems();
  if (busy || selected.length === 0) return;
  const [item] = selected;
  if (selected.length === 1 && item) {
    const quantity = selectedDropQuantity(item);
    if (quantity === undefined) return;
    if (quantity < item.quantity) {
      await dispatch({ type: "drop-quantity", itemId: item.id, quantity });
      return;
    }
  }
  const itemIds = selected.map((item) => item.id).sort();
  await dispatch({ type: "drop", itemIds });
}

async function unequipItem(slotId: string): Promise<void> {
  if (busy) return;
  await dispatch({ type: "unequip", slotId });
}

function selectedInventoryItems(): InventoryItemDto[] {
  return currentInventory.filter((item) => selectedInventoryIds.has(item.id));
}

function updateInventoryActions(): void {
  const selected = selectedInventoryItems();
  inventorySelectionCount.textContent = localization.format("inventory-selected-count", {
    count: selected.length,
  });
  inventoryEquip.disabled =
    busy || playerDead || selected.length !== 1 || !selected[0]?.equipmentSlot;
  inventoryUse.disabled = busy || playerDead || selected.length !== 1 || !selected[0]?.usable;
  const [item] = selected;
  if (selected.length === 1 && item) {
    if (dropQuantityItemId !== item.id) {
      dropQuantityItemId = item.id;
      inventoryDropQuantity.value = String(item.quantity);
    }
    inventoryDropQuantity.min = "1";
    inventoryDropQuantity.max = String(item.quantity);
    inventoryDropQuantity.disabled = busy || playerDead;
    inventoryDrop.disabled = busy || playerDead || selectedDropQuantity(item) === undefined;
  } else {
    dropQuantityItemId = undefined;
    inventoryDropQuantity.value = "";
    inventoryDropQuantity.disabled = true;
    inventoryDrop.disabled = busy || playerDead || selected.length === 0;
  }
  for (const checkbox of inventoryList.querySelectorAll<HTMLInputElement>(
    'input[type="checkbox"]',
  )) {
    checkbox.disabled = busy || playerDead;
  }
  for (const button of equipmentList.querySelectorAll<HTMLButtonElement>("button")) {
    button.disabled = busy || playerDead;
  }
}

function selectedDropQuantity(item: InventoryItemDto): number | undefined {
  const quantity = Number(inventoryDropQuantity.value);
  return Number.isSafeInteger(quantity) && quantity >= 1 && quantity <= item.quantity
    ? quantity
    : undefined;
}

function renderCombatStat(
  element: HTMLElement,
  value: number,
  equipmentModifier: number,
): void {
  element.textContent = localization.format(
    equipmentModifier === 0 ? "status-stat-value" : "status-stat-value-bonus",
    {
      value,
      bonus: signedModifier(equipmentModifier),
    },
  );
}

function appendItemModifiers(
  container: HTMLElement,
  modifiers: StatModifiersDto,
): void {
  const entries: Array<[MessageKey, number]> = [
    ["item-modifier-attack", modifiers.attack],
    ["item-modifier-defense", modifiers.defense],
    ["item-modifier-max-hp", modifiers.maxHp],
  ];
  for (const [key, value] of entries) {
    if (value === 0) continue;
    const modifier = document.createElement("span");
    modifier.className = "item-modifier";
    modifier.textContent = localization.format(key, { value: signedModifier(value) });
    container.append(modifier);
  }
}

function signedModifier(value: number): string {
  return value > 0 ? `+${value}` : String(value);
}

function formatTenthsPound(value: number): string {
  return `${Math.trunc(value / 10)}.${Math.abs(value % 10)}`;
}

function formatTenthsPoundArgument(value: string | undefined): string {
  if (value === undefined) return "?";
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) && parsed >= 0 ? formatTenthsPound(parsed) : "?";
}

function formatEvent(event: GameEventDto): string {
  switch (event.messageKey) {
    case "game-wait":
      return localization.format("message-game-wait");
    case "game-move-blocked":
      return localization.format("message-move-blocked");
    case "combat-player-hit":
      return formatPlayerDamageEvent(event);
    case "combat-player-slay":
      return localization.format("message-combat-slay", {
        target: contentName(event.args.target),
      });
    case "combat-player-miss":
      return localization.format("message-combat-player-miss", {
        target: contentName(event.args.target),
      });
    case "combat-monster-miss":
      return localization.format("message-combat-monster-miss", {
        source: contentName(event.args.source),
      });
    case "combat-monster-hit":
      return formatMonsterDamageEvent(event);
    case "combat-player-death":
      return localization.format("message-combat-player-death", {
        source: contentName(event.args.source),
      });
    case "projectile-unavailable":
      return localization.format("message-projectile-unavailable");
    case "projectile-ammo-unavailable":
      return localization.format("message-projectile-ammo-unavailable", {
        target: visibleItemNameForKind(event.args.target),
      });
    case "projectile-target-unavailable":
      return localization.format("message-projectile-target-unavailable");
    case "projectile-landed":
      return localization.format("message-projectile-landed");
    case "projectile-miss":
      return localization.format("message-projectile-miss", {
        target: contentName(event.args.target),
      });
    case "projectile-hit":
      return localization.format("message-projectile-hit", {
        target: contentName(event.args.target),
        damage: event.args.damage ?? "?",
      });
    case "projectile-slay":
      return localization.format("message-projectile-slay", {
        target: contentName(event.args.target),
      });
    case "projectile-ammo-recovered":
      return localization.format("message-projectile-ammo-recovered", {
        target: visibleItemNameForKind(event.args.target),
      });
    case "projectile-ammo-broken":
      return localization.format("message-projectile-ammo-broken", {
        target: visibleItemNameForKind(event.args.target),
      });
    case "status-player-damage":
      return localization.format("message-status-player-damage", {
        status: statusName(event.args.status),
        damage: event.args.damage ?? "?",
      });
    case "status-entity-damage":
      return localization.format("message-status-entity-damage", {
        target: contentName(event.args.target),
        status: statusName(event.args.status),
        damage: event.args.damage ?? "?",
      });
    case "status-player-expired":
      return localization.format("message-status-player-expired", {
        status: statusName(event.args.status),
      });
    case "status-entity-expired":
      return localization.format("message-status-entity-expired", {
        target: contentName(event.args.target),
        status: statusName(event.args.status),
      });
    case "status-player-death":
      return localization.format("message-status-player-death", {
        status: statusName(event.args.status),
      });
    case "status-entity-death":
      return localization.format("message-status-entity-death", {
        target: contentName(event.args.target),
        status: statusName(event.args.status),
      });
    case "status-fear-blocked":
      return localization.format("message-status-fear-blocked", {
        status: statusName(event.args.status),
      });
    case "item-pickup-success":
      return localization.format("message-item-pickup-success", {
        target: visibleItemNameForKind(event.args.target),
        quantity: event.args.quantity ?? "?",
      });
    case "item-pickup-over-capacity":
      return localization.format("message-item-pickup-over-capacity", {
        target: visibleItemNameForKind(event.args.target),
        quantity: event.args.quantity ?? "?",
        currentWeight: formatTenthsPoundArgument(event.args.currentWeight),
        pickupWeight: formatTenthsPoundArgument(event.args.pickupWeight),
        capacity: formatTenthsPoundArgument(event.args.capacity),
      });
    case "item-pickup-none":
      return localization.format("message-item-pickup-none");
    case "item-equip-success":
      return localization.format("message-item-equip-success", {
        target: visibleItemNameForKind(event.args.target),
        slot: equipmentSlotName(event.args.slot),
      });
    case "item-equip-swap":
      return localization.format("message-item-equip-swap", {
        target: visibleItemNameForKind(event.args.target),
        replaced: visibleItemNameForKind(event.args.replaced),
        slot: equipmentSlotName(event.args.slot),
      });
    case "item-equip-unavailable":
      return localization.format("message-item-equip-unavailable");
    case "item-unequip-success":
      return localization.format("message-item-unequip-success", {
        target: visibleItemNameForKind(event.args.target),
        slot: equipmentSlotName(event.args.slot),
      });
    case "item-unequip-none":
      return localization.format("message-item-unequip-none", {
        slot: equipmentSlotName(event.args.slot),
      });
    case "item-drop-success":
      return localization.format("message-item-drop-success", {
        stacks: event.args.stacks ?? "?",
        quantity: event.args.quantity ?? "?",
      });
    case "item-use-heal":
      return localization.format("message-item-use-heal", {
        target: visibleItemName(event.args.nameKey, event.args.target),
        amount: event.args.amount ?? "?",
      });
    case "item-use-no-effect":
      return localization.format("message-item-use-no-effect", {
        target: visibleItemName(event.args.nameKey, event.args.target),
      });
    case "item-use-unavailable":
      return localization.format("message-item-use-unavailable");
    case "item-thrown":
      return localization.format("message-item-thrown", {
        target: visibleItemNameForKind(event.args.target),
      });
    case "throw-miss":
      return localization.format("message-throw-miss", {
        source: visibleItemNameForKind(event.args.source),
        target: contentName(event.args.target),
      });
    case "throw-hit":
      return localization.format("message-throw-hit", {
        source: visibleItemNameForKind(event.args.source),
        target: contentName(event.args.target),
        damage: event.args.damage ?? "?",
      });
    case "throw-slay":
      return localization.format("message-throw-slay", {
        source: visibleItemNameForKind(event.args.source),
        target: contentName(event.args.target),
      });
    case "item-throw-unavailable":
      return localization.format("message-item-throw-unavailable");
    case "item-drop-none":
      return localization.format("message-item-drop-none");
    default:
      return localization.format("message-unknown-event", { key: event.messageKey });
  }
}

function formatPlayerDamageEvent(event: GameEventDto): string {
  const target = contentName(event.args.target);
  const resolution = damageResolution(event);
  if (!resolution) {
    return localization.format("message-combat-hit", {
      target,
      damage: event.args.damage ?? "?",
    });
  }
  const args = {
    target,
    damage: resolution.finalDamage,
    type: damageTypeName(resolution.damageType),
    adjustment: Math.abs(resolution.resistanceAdjustment),
  };
  if (resolution.resistance === "immune") {
    return localization.format("message-combat-hit-immune", args);
  }
  if (resolution.resistanceAdjustment > 0) {
    return localization.format("message-combat-hit-resisted", args);
  }
  if (resolution.resistanceAdjustment < 0) {
    return localization.format("message-combat-hit-amplified", args);
  }
  return localization.format("message-combat-hit", args);
}

function formatMonsterDamageEvent(event: GameEventDto): string {
  const source = contentName(event.args.source);
  const resolution = damageResolution(event);
  if (!resolution) {
    return localization.format("message-combat-monster-hit", {
      source,
      damage: event.args.damage ?? "?",
    });
  }
  const args = {
    source,
    damage: resolution.finalDamage,
    type: damageTypeName(resolution.damageType),
    adjustment: Math.abs(resolution.resistanceAdjustment),
  };
  if (resolution.resistance === "immune") {
    return localization.format("message-combat-monster-hit-immune", args);
  }
  if (resolution.resistanceAdjustment > 0) {
    return localization.format("message-combat-monster-hit-resisted", args);
  }
  if (resolution.resistanceAdjustment < 0) {
    return localization.format("message-combat-monster-hit-amplified", args);
  }
  return localization.format("message-combat-monster-hit", args);
}

function damageResolution(event: GameEventDto): DamageResolutionDto | undefined {
  const outcome = event.outcome;
  return outcome?.type === "damage" || outcome?.type === "death"
    ? outcome.resolution
    : undefined;
}

function damageTypeName(damageType: DamageTypeDto): string {
  const keys: Record<DamageTypeDto, MessageKey> = {
    physical: "damage-type-physical-name",
    acid: "damage-type-acid-name",
    electricity: "damage-type-electricity-name",
    fire: "damage-type-fire-name",
    cold: "damage-type-cold-name",
    poison: "damage-type-poison-name",
  };
  return localization.format(keys[damageType]);
}

function contentName(id: string | undefined): string {
  if (id === "demo.item.luminous-shard") {
    return localization.format("item-demo-luminous-shard-name");
  }
  if (id === "demo.item.echo-charm") {
    return localization.format("item-demo-echo-charm-name");
  }
  if (id === "demo.item.echo-blade") {
    return localization.format("item-demo-echo-blade-name");
  }
  if (id === "demo.item.resonance-sling") {
    return localization.format("item-demo-resonance-sling-name");
  }
  if (id === "demo.item.resonance-pellet") {
    return localization.format("item-demo-resonance-pellet-name");
  }
  if (id === "demo.actor.ember-mote") {
    return localization.format("actor-demo-ember-mote-name");
  }
  if (id === "demo.actor.acid-seep") {
    return localization.format("actor-demo-acid-seep-name");
  }
  if (id === "demo.actor.storm-spark") {
    return localization.format("actor-demo-storm-spark-name");
  }
  if (id === "demo.actor.frost-wisp") {
    return localization.format("actor-demo-frost-wisp-name");
  }
  if (id === "demo.actor.venom-spore") {
    return localization.format("actor-demo-venom-spore-name");
  }
  if (id === "demo.actor.echo-hound") {
    return localization.format("actor-demo-echo-hound-name");
  }
  return localization.format(
    id?.startsWith("demo.item.") ? "item-unknown-name" : "actor-unknown-name",
  );
}

function visibleItemName(
  displayNameKey: string | undefined,
  fallbackKindId: string | undefined,
): string {
  switch (displayNameKey) {
    case "item-demo-luminous-shard-name":
    case "item-demo-unfamiliar-shard-name":
    case "item-demo-echo-charm-name":
    case "item-demo-echo-blade-name":
    case "item-demo-resonance-sling-name":
    case "item-demo-resonance-pellet-name":
    case "item-unknown-name":
      return localization.format(displayNameKey);
    default:
      return contentName(fallbackKindId);
  }
}

function visibleItemNameForKind(kindId: string | undefined): string {
  if (!kindId) return localization.format("item-unknown-name");
  const projected =
    currentInventory.find((item) => item.kindId === kindId) ??
    currentEquipment.find((item) => item.kindId === kindId) ??
    currentStatus?.items.find((item) => item.kindId === kindId);
  if (projected) return visibleItemName(projected.displayNameKey, kindId);
  if (kindId === "demo.item.luminous-shard") {
    return localization.format("item-demo-unfamiliar-shard-name");
  }
  return contentName(kindId);
}

function equipmentSlotName(slotId: string | undefined): string {
  if (slotId === "charm") return localization.format("equipment-slot-charm");
  return localization.format("equipment-slot-unknown", { slot: slotId ?? "?" });
}

function statusName(statusId: string | undefined): string {
  if (statusId === "rfb.status.bleeding") {
    return localization.format("status-bleeding-name");
  }
  if (statusId === "rfb.status.poison") {
    return localization.format("status-poison-name");
  }
  if (statusId === "rfb.status.haste") {
    return localization.format("status-haste-name");
  }
  if (statusId === "rfb.status.slow") {
    return localization.format("status-slow-name");
  }
  if (statusId === "rfb.status.stun") {
    return localization.format("status-stun-name");
  }
  if (statusId === "rfb.status.fear") {
    return localization.format("status-fear-name");
  }
  return localization.format("status-unknown-name");
}

function addLocalizedMessage(
  key: MessageKey,
  args: Record<string, string | number> | undefined,
  kind: string,
): void {
  messageRecords.push({
    source: "key",
    turn: turnValue.textContent ?? "0",
    kind,
    key,
    args,
  });
  renderMessages();
}

function addGameEvent(event: GameEventDto): void {
  messageRecords.push({
    source: "event",
    turn: turnValue.textContent ?? "0",
    kind: event.kind,
    event,
  });
  renderMessages();
}

function renderMessages(): void {
  messageList.replaceChildren();
  for (const record of messageRecords) renderMessage(record);
  messageList.scrollTop = messageList.scrollHeight;
}

function renderMessage(record: MessageRecord): void {
  const item = document.createElement("li");
  item.className = `message message-${record.kind.replaceAll(".", "-")}`;
  const turn = document.createElement("span");
  turn.className = "message-turn";
  turn.textContent = record.turn;
  const content = document.createElement("span");
  content.textContent =
    record.source === "event"
      ? formatEvent(record.event)
      : localization.format(record.key, localizedMessageArgs(record));
  item.append(turn, content);
  messageList.append(item);
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
    return {
      reason: localization.format(nativeSaveErrorKey(String(record.args.code))),
    };
  }
  return record.args;
}

function showError(error: unknown): void {
  const message = error instanceof Error ? error.message : String(error);
  connectionState = "error";
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

function startTargeting(): void {
  if (busy || playerDead || !currentStatus) return;
  const next = beginTargeting(
    currentStatus.player.position,
    currentStatus.player.projectileProfile?.targetSpec,
  );
  if (!next) {
    addLocalizedMessage("message-target-mode-unavailable", undefined, "system");
    renderTargeting();
    return;
  }
  targeting = next;
  addLocalizedMessage("message-target-mode-started", undefined, "system");
  renderTargeting();
}

function cancelTargeting(announce = true): void {
  if (!targeting) return;
  targeting = undefined;
  if (announce) addLocalizedMessage("message-target-mode-cancelled", undefined, "system");
  renderTargeting();
}

async function confirmTargeting(): Promise<void> {
  const state = targeting;
  const status = currentStatus;
  if (!state || !status || busy || playerDead) return;
  const target = targetSelectionAtCursor(state, status.entities);
  if (!target) {
    addLocalizedMessage("message-target-selection-invalid", undefined, "system");
    return;
  }
  cancelTargeting(false);
  await dispatch({ type: "fire-target", target });
}

function renderTargeting(): void {
  const available = Boolean(
    currentStatus &&
      beginTargeting(
        currentStatus.player.position,
        currentStatus.player.projectileProfile?.targetSpec,
      ),
  );
  targetModeToggle.textContent = localization.format(
    targeting ? "action-target-cancel" : "action-target-start",
  );
  targetModeToggle.setAttribute("aria-pressed", targeting ? "true" : "false");
  targetModeToggle.disabled = busy || playerDead || (!targeting && !available);
  mapHost.dataset.targeting = targeting ? "true" : "false";
  targetCursor.hidden = !targeting;
  if (!targeting) {
    targetModeStatus.textContent = localization.format(
      available ? "target-status-ready" : "target-status-unavailable",
    );
    delete mapHost.dataset.targetX;
    delete mapHost.dataset.targetY;
    return;
  }

  const { cursor, spec } = targeting;
  const cameraX = Number(mapHost.dataset.cameraX ?? 0);
  const cameraY = Number(mapHost.dataset.cameraY ?? 0);
  const renderedCellSize = MAP_CELL_SIZE * zoom;
  targetCursor.style.left = `${cameraX + cursor.x * renderedCellSize}px`;
  targetCursor.style.top = `${cameraY + cursor.y * renderedCellSize}px`;
  targetCursor.style.width = `${renderedCellSize}px`;
  targetCursor.style.height = `${renderedCellSize}px`;
  mapHost.dataset.targetX = String(cursor.x);
  mapHost.dataset.targetY = String(cursor.y);
  targetModeStatus.textContent = localization.format("target-status-active", {
    x: cursor.x,
    y: cursor.y,
    range: spec.range,
  });
}

function commandForKeyboardEvent(event: KeyboardEvent): GameCommand | undefined {
  const key = event.key.toLowerCase();
  if (key === "g") return { type: "pick-up" };
  const direction = directionForKeyboardEvent(event);
  if (inputPreset === "numpad") {
    if (event.code === "Numpad5") return { type: "wait" };
    return direction ? { type: "move", direction } : undefined;
  }

  if (inputPreset === "vi") {
    if (key === ".") return { type: "wait" };
    return direction ? { type: "move", direction } : undefined;
  }

  if (key === " ") return { type: "wait" };
  return direction ? { type: "move", direction } : undefined;
}

function directionForKeyboardEvent(event: KeyboardEvent): Direction | undefined {
  if (inputPreset === "numpad") return NUMPAD_DIRECTIONS[event.code];
  const key = event.key.toLowerCase();
  return inputPreset === "vi" ? VI_DIRECTIONS[key] : WASD_DIRECTIONS[key];
}

function isTextInput(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLSelectElement
  );
}

function readInputPreset(): InputPreset {
  const stored = localStorage.getItem(INPUT_PRESET_STORAGE_KEY);
  return isInputPreset(stored) ? stored : "numpad";
}

function isInputPreset(value: string | null): value is InputPreset {
  return value === "numpad" || value === "vi" || value === "wasd";
}

function readTilesetPreset(): TilesetPreset {
  const stored = localStorage.getItem(TILESET_PRESET_STORAGE_KEY);
  return isTilesetPreset(stored) ? stored : "ascii";
}

function readLocale(): "en-US" | "zh-CN" {
  const stored = localStorage.getItem(LOCALE_STORAGE_KEY);
  return isSupportedLocale(stored) ? stored : "zh-CN";
}

function readCameraMode(): CameraMode {
  const stored = localStorage.getItem(CAMERA_MODE_STORAGE_KEY);
  return isCameraMode(stored) ? stored : "full-map";
}

function readZoomLevel(): ZoomLevel {
  return parseZoomLevel(localStorage.getItem(ZOOM_STORAGE_KEY));
}

function isCameraMode(value: string | null): value is CameraMode {
  return value === "full-map" || value === "player-centered";
}

function isTilesetPreset(value: string | null): value is TilesetPreset {
  return value === "ascii" || value === "image";
}

function announceTileset(id: string, warnings: readonly TilesetWarning[]): void {
  addLocalizedMessage("message-tileset-loaded", { id }, "system");
  for (const warning of warnings) {
    addLocalizedMessage(tilesetWarningMessageKey(warning), undefined, "system");
  }
}

function renderInputHelp(): void {
  const keys: Record<InputPreset, MessageKey> = {
    numpad: "controls-numpad",
    vi: "controls-vi",
    wasd: "controls-wasd",
  };
  controlsHelp.textContent = localization.format(keys[inputPreset]);
}

function inputPresetMessageKey(preset: InputPreset): MessageKey {
  const keys: Record<InputPreset, MessageKey> = {
    numpad: "input-preset-numpad",
    vi: "input-preset-vi",
    wasd: "input-preset-wasd",
  };
  return keys[preset];
}

function tilesetWarningMessageKey(warning: TilesetWarning): MessageKey {
  return warning === "image-too-small"
    ? "message-tileset-image-too-small"
    : "message-tileset-image-load-failed";
}

function renderConnectionStatus(): void {
  const keys: Record<ConnectionState, MessageKey> = {
    starting: "connection-starting",
    ready: "connection-ready",
    error: "connection-error",
  };
  connectionStatus.textContent = localization.format(keys[connectionState]);
  connectionStatus.classList.toggle("ready", connectionState === "ready");
  connectionStatus.classList.toggle("error", connectionState === "error");
}

function element<T extends HTMLElement>(id: string): T {
  const found = document.getElementById(id);
  if (!found) throw new Error(`Missing element #${id}`);
  return found as T;
}

const NUMPAD_DIRECTIONS: Partial<Record<string, Direction>> = {
  Numpad8: "north",
  Numpad9: "north-east",
  Numpad6: "east",
  Numpad3: "south-east",
  Numpad2: "south",
  Numpad1: "south-west",
  Numpad4: "west",
  Numpad7: "north-west",
};

const VI_DIRECTIONS: Partial<Record<string, Direction>> = {
  k: "north",
  u: "north-east",
  l: "east",
  n: "south-east",
  j: "south",
  b: "south-west",
  h: "west",
  y: "north-west",
};

const WASD_DIRECTIONS: Partial<Record<string, Direction>> = {
  w: "north",
  e: "north-east",
  d: "east",
  c: "south-east",
  s: "south",
  z: "south-west",
  a: "west",
  q: "north-west",
};
