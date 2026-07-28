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
  AbilityDto,
  AbilityLearningDto,
  DamageResolutionDto,
  DamageTypeDto,
  Direction,
  BodySlotDto,
  EquipmentBonusesDto,
  EquipmentItemDto,
  EquipmentPassiveDto,
  AttributeKindDto,
  GameCommand,
  GameEventDto,
  GameSnapshot,
  GameUpdate,
  InventoryItemDto,
  ItemPropertyDto,
  PlayerBuildDto,
  PlayerProgressDto,
  ResistanceDto,
  ResistanceLevelDto,
  ResourcePoolDto,
  SlayDto,
  SlayTargetDto,
  StatModifiersDto,
  SummonCommandDto,
  SummonCommandModeDto,
  TargetSpecDto,
  WeaponBrandDto,
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
import {
  terrainInteractionCommand,
  terrainInteractionForDirection,
  terrainInteractionsForMode,
  terrainInteractionModeForKey,
  terrainSearchCommandForKey,
  type TerrainInteractionMode,
} from "./terrain-interaction";

const core = new TauriNativeTransport();
const crashDiagnostics = new DesktopCrashDiagnostics();
const nativeSaveStorage = new NativeSaveStorage();
const renderer = new MapRenderer();
let busy = false;
let playerDead = false;
let campaignEnded = false;
let nativeSaveBusy = false;
let recordingFrontendCrash = false;
let announcedCrashReport: string | undefined;
let dropQuantityItemId: string | undefined;
let targeting: TargetingState | undefined;
let targetingIntent: TargetingIntent | undefined;
let terrainInteractionMode: TerrainInteractionMode | undefined;
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
const progressionLevelValue = element<HTMLElement>("progression-level-value");
const progressionExperienceValue = element<HTMLElement>("progression-experience-value");
const progressionCapValue = element<HTMLElement>("progression-cap-value");
const progressionPointsValue = element<HTMLElement>("progression-points-value");
const progressionBuildValue = element<HTMLElement>("progression-build-value");
const progressionRaceValue = element<HTMLElement>("progression-race-value");
const progressionClassValue = element<HTMLElement>("progression-class-value");
const progressionPersonalityValue = element<HTMLElement>("progression-personality-value");
const progressionMultipliersValue = element<HTMLElement>("progression-multipliers-value");
const attributeList = element<HTMLUListElement>("attribute-list");
const skillList = element<HTMLUListElement>("skill-list");
const resourceList = element<HTMLUListElement>("resource-list");
const abilityList = element<HTMLUListElement>("ability-list");
const resourceRest = element<HTMLButtonElement>("resource-rest");
const summonCommandStatus = element<HTMLElement>("summon-command-status");
const summonCommandButtons: Record<SummonCommandModeDto, HTMLButtonElement> = {
  follow: element<HTMLButtonElement>("summon-command-follow"),
  attack: element<HTMLButtonElement>("summon-command-attack"),
  "keep-distance": element<HTMLButtonElement>("summon-command-keep-distance"),
  guard: element<HTMLButtonElement>("summon-command-guard"),
};
const taskLogList = element<HTMLUListElement>("task-log-list");
const campaignStatusValue = element<HTMLElement>("campaign-status-value");
const campaignScoreValue = element<HTMLElement>("campaign-score-value");
const campaignDungeonsValue = element<HTMLElement>("campaign-dungeons-value");
const campaignTasksValue = element<HTMLElement>("campaign-tasks-value");
const campaignRetire = element<HTMLButtonElement>("campaign-retire");
const inventoryCount = element<HTMLElement>("inventory-count");
const inventorySelectionCount = element<HTMLElement>("inventory-selection-count");
const inventoryUse = element<HTMLButtonElement>("inventory-use");
const inventoryAppraise = element<HTMLButtonElement>("inventory-appraise");
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
type TargetingIntent =
  | { type: "projectile" }
  | { type: "ability"; abilityId: string };
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
let currentBodySlots: BodySlotDto[] = [];
const selectedInventoryIds = new Set<string>();
let nativeSaves: NativeSaveSummary[] = [];
const MESSAGE_HISTORY_LIMIT = 500;
const messageRecords: MessageRecord[] = [];
const ATTRIBUTE_KINDS: AttributeKindDto[] = [
  "strength",
  "intelligence",
  "wisdom",
  "dexterity",
  "constitution",
  "charisma",
];
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
    currentBodySlots = snapshot.bodySlots ?? [];
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
  if (terrainInteractionMode) {
    if (event.key === "Escape") {
      event.preventDefault();
      terrainInteractionMode = undefined;
      addLocalizedMessage("message-door-mode-cancelled", undefined, "system");
      return;
    }
    const direction = directionForKeyboardEvent(event);
    if (direction) {
      event.preventDefault();
      const mode = terrainInteractionMode;
      terrainInteractionMode = undefined;
      const interaction = currentStatus
        ? terrainInteractionForDirection(
            currentStatus.terrainInteractions,
            mode,
            direction,
          )
        : undefined;
      if (!interaction) {
        addLocalizedMessage(
          "message-terrain-interaction-not-applicable",
          undefined,
          "system",
        );
        return;
      }
      if (!interaction.available) {
        addLocalizedMessage(
          interaction.unavailableReason === "occupied-by-actor"
            ? "message-terrain-interaction-blocked-actor"
            : "message-terrain-interaction-blocked-item",
          undefined,
          "system",
        );
        return;
      }
      void dispatch(terrainInteractionCommand(mode, direction));
    }
    return;
  }
  const nextTerrainInteractionMode = terrainInteractionModeForKey(event.key);
  if (nextTerrainInteractionMode) {
    event.preventDefault();
    if (
      !currentStatus ||
      terrainInteractionsForMode(
        currentStatus.terrainInteractions,
        nextTerrainInteractionMode,
      ).length === 0
    ) {
      addLocalizedMessage(
        "message-terrain-interaction-mode-unavailable",
        undefined,
        "system",
      );
      return;
    }
    terrainInteractionMode = nextTerrainInteractionMode;
    addLocalizedMessage(
      nextTerrainInteractionMode === "open-door"
        ? "message-door-mode-open"
        : nextTerrainInteractionMode === "close-door"
          ? "message-door-mode-close"
          : nextTerrainInteractionMode === "bash-door"
            ? "message-door-mode-bash"
            : nextTerrainInteractionMode === "disarm-trap"
              ? "message-trap-mode-disarm"
              : "message-terrain-mode-dig",
      undefined,
      "system",
    );
    return;
  }
  const searchCommand = terrainSearchCommandForKey(event.key);
  if (searchCommand) {
    event.preventDefault();
    void dispatch(searchCommand);
    return;
  }
  if (event.key.toLowerCase() === "f") {
    event.preventDefault();
    startProjectileTargeting();
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
inventoryAppraise.addEventListener("click", () => void appraiseSelectedInventoryItem());
inventoryEquip.addEventListener("click", () => void equipSelectedInventoryItem());
inventoryDrop.addEventListener("click", () => void dropSelectedInventoryItems());
inventoryDropQuantity.addEventListener("input", updateInventoryActions);
targetModeToggle.addEventListener("click", () => {
  if (targeting) cancelTargeting();
  else startProjectileTargeting();
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
  if (playerDead || campaignEnded) return;
  busy = true;
  updateInventoryActions();
  renderTargeting();
  try {
    const update = await core.dispatch(command);
    // Clearing busy before rendering lets renderStatus/renderInventory emit
    // the final control states directly instead of re-rendering every panel
    // a second time afterwards.
    busy = false;
    renderer.applyUpdate(update);
    renderStatus(update);
    renderInventory(update.inventory, update.equipment);
    for (const event of update.events) addGameEvent(event);
  } catch (error) {
    showError(error);
  } finally {
    if (busy) {
      busy = false;
      updateInventoryActions();
      renderTargeting();
    }
  }
}

campaignRetire.addEventListener("click", () => void dispatch({ type: "retire" }));
resourceRest.addEventListener("click", () => void dispatch({ type: "rest", turns: 100 }));
for (const [mode, button] of Object.entries(summonCommandButtons) as [
  SummonCommandModeDto,
  HTMLButtonElement,
][]) {
  button.addEventListener("click", () =>
    void dispatch({ type: "set-summon-command", mode }),
  );
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
  currentBodySlots = snapshot.bodySlots ?? [];
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
  campaignEnded = state.campaign.status === "retired";
  if (
    targeting &&
    (targeting.origin.x !== state.player.position.x ||
      targeting.origin.y !== state.player.position.y ||
      !targetingIntent ||
      !targetSpecForIntent(state, targetingIntent))
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
  renderProgression(state.player.progress, state.player.build);
  renderAbilities(
    state.player.abilities ?? [],
    state.player.resources ?? [],
    state.player.abilityLearning,
  );
  renderSummonCommand(state.player.summonCommand, state.entities);
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
  taskLogList.replaceChildren(
    ...state.tasks.map((task) => {
      const row = document.createElement("li");
      row.textContent = localization.format("task-log-entry", {
        task: contentName(task.floorId),
        status: localization.format(`task-status-${task.status}` as MessageKey),
        stage: task.stage,
        stages: task.stages,
        current: task.current,
        required: task.required,
      });
      const maxRetakes = task.maxRetakes;
      if (maxRetakes !== undefined && maxRetakes !== null) {
        row.append(
          " ",
          localization.format("task-log-retakes", {
            used: task.retakesUsed,
            maximum: maxRetakes,
          }),
        );
      }
      if (task.status === "active") {
        const abandon = document.createElement("button");
        abandon.type = "button";
        abandon.textContent = localization.format("action-task-abandon");
        abandon.disabled = busy;
        abandon.addEventListener("click", () => void dispatch({ type: "abandon-task" }));
        row.append(" ", abandon);
      } else if (task.status === "paused") {
        const abandon = document.createElement("button");
        abandon.type = "button";
        abandon.textContent = localization.format("action-task-abandon");
        abandon.disabled = busy;
        abandon.addEventListener("click", () =>
          void dispatch({ type: "abandon-paused-task", taskId: task.taskId }),
        );
        row.append(" ", abandon);
      }
      return row;
    }),
  );
  campaignStatusValue.textContent = localization.format(
    `campaign-status-${state.campaign.status}` as MessageKey,
  );
  campaignScoreValue.textContent = String(state.campaign.score);
  campaignDungeonsValue.textContent = String(state.campaign.conqueredDungeons);
  campaignTasksValue.textContent = String(state.campaign.completedTasks);
  updateCampaignAction();
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

function renderProgression(
  progress: PlayerProgressDto | undefined,
  build: PlayerBuildDto | null | undefined,
): void {
  if (!progress) {
    const unavailable = localization.format("progression-unavailable");
    progressionLevelValue.textContent = unavailable;
    progressionExperienceValue.textContent = unavailable;
    progressionCapValue.textContent = unavailable;
    progressionPointsValue.textContent = unavailable;
    progressionBuildValue.textContent = unavailable;
    progressionRaceValue.textContent = unavailable;
    progressionClassValue.textContent = unavailable;
    progressionPersonalityValue.textContent = unavailable;
    progressionMultipliersValue.textContent = unavailable;
    attributeList.replaceChildren();
    skillList.replaceChildren();
    return;
  }
  progressionLevelValue.textContent = localization.format("progression-level-value", {
    level: progress.level,
    maxLevel: progress.maxLevel,
  });
  progressionExperienceValue.textContent = localization.format("progression-experience-value", {
    experience: String(progress.experience),
    next: progress.experienceForNextLevel === undefined || progress.experienceForNextLevel === null
      ? "—"
      : String(progress.experienceForNextLevel),
  });
  progressionCapValue.textContent = localization.format("progression-cap-value", {
    levelCap: progress.levelCap,
    attributeCap: formatAttributeValue(progress.attributeCap),
    attributeIndexCap: progress.attributeIndexCap,
  });
  progressionPointsValue.textContent = String(progress.pendingAttributeIncreases);
  progressionBuildValue.textContent = build
    ? localization.format(build.buildNameKey as MessageKey)
    : localization.format("progression-unavailable");
  progressionRaceValue.textContent = build
    ? localization.format(build.raceNameKey as MessageKey)
    : localization.format("progression-unavailable");
  progressionClassValue.textContent = build
    ? localization.format(build.classNameKey as MessageKey)
    : localization.format("progression-unavailable");
  progressionPersonalityValue.textContent = build
    ? localization.format(build.personalityNameKey as MessageKey)
    : localization.format("progression-unavailable");
  progressionMultipliersValue.textContent = build
    ? localization.format("progression-multipliers-value", {
        life: build.lifePercent,
        experience: build.experiencePercent,
      })
    : localization.format("progression-unavailable");
  attributeList.replaceChildren(
    ...ATTRIBUTE_KINDS.map((attribute) => {
      const value = progress.attributes[attribute];
      const row = document.createElement("li");
      row.className = "attribute-row";
      const label = document.createElement("span");
      label.className = "attribute-name";
      label.textContent = localization.format(`attribute-${attribute}` as MessageKey);
      const values = document.createElement("span");
      values.className = "attribute-value";
      values.textContent = localization.format("attribute-value", {
        natural: formatAttributeValue(value.natural),
        effective: formatAttributeValue(value.effective),
        index: value.index,
      });
      const increase = document.createElement("button");
      increase.type = "button";
      increase.className = "attribute-increase";
      increase.textContent = localization.format("action-increase-attribute");
      increase.disabled =
        busy ||
        playerDead ||
        progress.pendingAttributeIncreases === 0 ||
        value.index >= progress.attributeIndexCap;
      increase.addEventListener("click", () =>
        void dispatch({ type: "increase-attribute", attribute }),
      );
      row.append(label, values, increase);
      return row;
    }),
  );
  skillList.replaceChildren(
    ...progress.skills.map((skill) => {
      const row = document.createElement("li");
      row.className = "skill-row";
      const name = document.createElement("span");
      name.className = "skill-name";
      name.textContent = localization.format(skill.nameKey as MessageKey);
      const value = document.createElement("span");
      value.className = "skill-value";
      value.textContent = localization.format("skill-value", {
        current: skill.current,
        maximum: skill.maximum,
        growth: skill.growthPerTenLevels,
      });
      row.append(name, value);
      return row;
    }),
  );
}

function renderSummonCommand(
  command: SummonCommandDto | undefined,
  entities: GameSnapshot["entities"],
): void {
  const mode = command?.mode ?? "follow";
  const count = entities.filter(
    (entity) => entity.faction === "player" && entity.summon != null,
  ).length;
  summonCommandStatus.textContent = localization.format("summon-command-status", {
    mode: localization.format(`summon-command-mode-${mode}` as MessageKey),
    count,
  });
  for (const [buttonMode, button] of Object.entries(summonCommandButtons) as [
    SummonCommandModeDto,
    HTMLButtonElement,
  ][]) {
    const selected = buttonMode === mode;
    button.disabled = busy || playerDead || campaignEnded || selected;
    button.setAttribute("aria-pressed", String(selected));
  }
}

function renderAbilities(
  abilities: AbilityDto[],
  resources: ResourcePoolDto[],
  learning: AbilityLearningDto | null | undefined,
): void {
  resourceList.replaceChildren();
  abilityList.replaceChildren();
  resourceRest.disabled =
    busy ||
    playerDead ||
    !resources.some(
      (resource) => resource.restRecoveryAmount > 0 && resource.current < resource.maximum,
    );
  if (resources.length === 0 && abilities.length === 0) {
    const unavailable = document.createElement("li");
    unavailable.className = "ability-empty";
    unavailable.textContent = localization.format("ability-unavailable");
    abilityList.append(unavailable);
    return;
  }

  for (const resource of resources) {
    const row = document.createElement("li");
    row.className = "resource-row";
    const name = localization.format(resource.nameKey as MessageKey);
    const momentumDriven =
      (resource.meleeHitGainAmount ?? 0) > 0 ||
      (resource.meleeKillGainAmount ?? 0) > 0 ||
      (resource.turnDecayAmount ?? 0) > 0;
    row.textContent = momentumDriven
      ? localization.format("ability-resource-momentum-value", {
          resource: name,
          current: resource.current,
          maximum: resource.maximum,
          hit: resource.meleeHitGainAmount ?? 0,
          kill: resource.meleeKillGainAmount ?? 0,
          decay: resource.turnDecayAmount ?? 0,
        })
      : localization.format("ability-resource-value", {
          resource: name,
          current: resource.current,
          maximum: resource.maximum,
          wait: resource.waitRecoveryAmount,
          rest: resource.restRecoveryAmount,
        });
    resourceList.append(row);
  }
  if (learning) {
    const row = document.createElement("li");
    row.className = "resource-row";
    row.textContent = localization.format("ability-learning-value", {
      learned: learning.learnedCount,
      capacity: learning.capacity,
      remaining: learning.remainingSlots,
    });
    resourceList.append(row);
  }

  for (const ability of abilities) {
    const row = document.createElement("li");
    row.className = "ability-row";
    const details = document.createElement("div");
    details.className = "ability-details";
    const name = document.createElement("span");
    name.className = "ability-name";
    name.textContent = localization.format(ability.nameKey as MessageKey);
    const summary = document.createElement("span");
    summary.className = "ability-summary";
    summary.textContent = localization.format("ability-summary", {
      level: ability.minimumLevel,
      baseCost: ability.baseResourceCost,
      cost: ability.resourceCost,
      failure: ability.failurePercent,
    });
    const proficiency = document.createElement("span");
    proficiency.className = "ability-summary";
    proficiency.textContent = localization.format("ability-proficiency-summary", {
      rank: localization.format(`ability-proficiency-${ability.proficiencyRank}` as MessageKey),
      current: ability.proficiency,
      maximum: ability.proficiencyCap,
      casts: ability.castCount,
      fails: ability.failCount,
    });
    const status = document.createElement("span");
    status.className = "ability-status";
    status.textContent = localization.format(
      ability.innate
        ? "ability-status-innate"
        : ability.learned
          ? "ability-status-learned"
          : "ability-status-unlearned",
    );
    details.append(name, summary, proficiency, status);
    if (ability.areaRadius != null) {
      const area = document.createElement("span");
      area.className = "ability-status";
      area.textContent = localization.format("ability-area-summary", {
        radius: ability.areaRadius,
      });
      details.append(area);
    }
    if (ability.beamDamage) {
      const beam = document.createElement("span");
      beam.className = "ability-status";
      beam.textContent = localization.format("ability-beam-summary");
      details.append(beam);
    }
    if (ability.coneRadius != null) {
      const cone = document.createElement("span");
      cone.className = "ability-status";
      cone.textContent = localization.format("ability-cone-summary", {
        radius: ability.coneRadius,
      });
      details.append(cone);
    }
    if (ability.teleport) {
      const teleport = document.createElement("span");
      teleport.className = "ability-status";
      teleport.textContent = localization.format("ability-teleport-summary");
      details.append(teleport);
    }
    if (ability.summon != null) {
      const summon = document.createElement("span");
      summon.className = "ability-status";
      summon.textContent = localization.format("ability-summon-summary", {
        count: ability.summon.count,
        radius: ability.summon.radius,
        turns: ability.summon.durationTurns,
      });
      details.append(summon);
    }
    if (ability.detect != null) {
      const detect = document.createElement("span");
      detect.className = "ability-status";
      detect.textContent = localization.format("ability-detect-summary", {
        category: ability.detect.category,
        radius: ability.detect.radius,
        persistence: localization.format(
          ability.detect.persistent
            ? "ability-detect-persistent"
            : "ability-detect-transient",
        ),
      });
      details.append(detect);
    }
    if (ability.terrainTransform != null) {
      const transform = document.createElement("span");
      transform.className = "ability-status";
      transform.textContent = localization.format("ability-terrain-transform-summary", {
        sources: ability.terrainTransform.sourceTerrainIds.length,
        terrain: contentName(ability.terrainTransform.targetTerrainId),
        radius: ability.terrainTransform.radius,
      });
      details.append(transform);
    }
    if (ability.effects.length > 1) {
      const effects = document.createElement("span");
      effects.className = "ability-status";
      effects.textContent = localization.format("ability-effects-summary", {
        count: ability.effects.length,
      });
      details.append(effects);
    }
    if (ability.cooldownTurns > 0) {
      const cooldown = document.createElement("span");
      cooldown.className = "ability-status";
      cooldown.textContent = localization.format("ability-cooldown-summary", {
        remaining: ability.cooldownRemaining,
        turns: ability.cooldownTurns,
      });
      details.append(cooldown);
    }

    const actions = document.createElement("div");
    actions.className = "ability-actions";
    const study = document.createElement("button");
    study.type = "button";
    study.textContent = localization.format("action-ability-study");
    study.disabled = busy || playerDead || !ability.canStudy || !ability.bookItemId;
    study.addEventListener("click", () => {
      if (!ability.bookItemId) return;
      void dispatch({
        type: "study-ability",
        bookItemId: ability.bookItemId,
        abilityId: ability.id,
      });
    });
    const forget = document.createElement("button");
    forget.type = "button";
    forget.textContent = localization.format("action-ability-forget");
    forget.disabled = busy || playerDead || !ability.canForget;
    forget.addEventListener("click", () => {
      void dispatch({ type: "forget-ability", abilityId: ability.id });
    });
    const cast = document.createElement("button");
    cast.type = "button";
    cast.textContent = localization.format("action-ability-cast");
    cast.disabled = busy || playerDead || !ability.canCast;
    cast.addEventListener("click", () => {
      if (ability.targetSpec.modes.includes("self")) {
        void dispatch({
          type: "cast-ability",
          abilityId: ability.id,
          target: { type: "self" },
        });
        return;
      }
      startAbilityTargeting(ability);
    });
    actions.append(study, forget, cast);
    row.append(details, actions);
    abilityList.append(row);
  }
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
      if (item.charges) {
        const charges = document.createElement("span");
        charges.className = "inventory-charges";
        charges.textContent = localization.format("inventory-charges", {
          current: item.charges.current,
          maximum: item.charges.maximum,
        });
        details.append(charges);
      }
      appendItemModifiers(details, item.modifiers);
      appendEquipmentBonuses(details, item.equipmentBonuses);
      appendItemDefenses(details, item.resistances, item.statusImmunities);
      appendItemOffense(details, item.slays, item.brands);
      appendEquipmentPassives(details, item.passives);
      appendItemQuality(details, item.quality);
      appendKnownItemProperties(details, item.knownProperties);
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
  // Pre-template snapshots carry no body slots; fall back to listing only
  // the occupied instances so old cores keep rendering.
  const slots: BodySlotDto[] =
    currentBodySlots.length > 0
      ? currentBodySlots
      : equipment.map((item) => ({ id: item.slotId, slotType: item.slotId }));
  if (slots.length === 0) {
    const empty = document.createElement("li");
    empty.className = "equipment-empty";
    empty.textContent = localization.format("equipment-empty");
    equipmentList.append(empty);
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
        ? localization.format("equipment-slot-ordinal", {
            slot: equipmentSlotName(slot.slotType),
            ordinal,
          })
        : equipmentSlotName(slot.slotType);
    const row = document.createElement("li");
    row.dataset.slotId = slot.id;
    const item = byInstance.get(slot.id);
    if (item) {
      row.className = "equipment-item";
      const details = document.createElement("span");
      details.className = "equipment-item-details";
      const name = document.createElement("span");
      name.textContent = visibleItemName(item.displayNameKey, item.kindId);
      const slotTag = document.createElement("span");
      slotTag.className = "equipment-slot";
      slotTag.textContent = slotLabel;
      details.append(name, slotTag);
      appendItemModifiers(details, item.modifiers);
      appendEquipmentBonuses(details, item.equipmentBonuses);
      appendItemDefenses(details, item.resistances, item.statusImmunities);
      appendItemOffense(details, item.slays, item.brands);
      appendEquipmentPassives(details, item.passives);
      appendItemQuality(details, item.quality);
      appendKnownItemProperties(details, item.knownProperties);
      const unequip = document.createElement("button");
      unequip.type = "button";
      unequip.textContent = localization.format("action-equipment-unequip");
      unequip.disabled = busy;
      unequip.addEventListener("click", () => void unequipItem(item.slotId));
      row.append(details, unequip);
    } else {
      row.className = "equipment-item equipment-slot-vacant";
      const details = document.createElement("span");
      details.className = "equipment-item-details";
      const slotTag = document.createElement("span");
      slotTag.className = "equipment-slot";
      slotTag.textContent = slotLabel;
      const vacant = document.createElement("span");
      vacant.className = "equipment-vacant-label";
      vacant.textContent = localization.format("equipment-slot-vacant");
      details.append(slotTag, vacant);
      row.append(details);
    }
    equipmentList.append(row);
  }
}

async function equipSelectedInventoryItem(): Promise<void> {
  const selected = selectedInventoryItems();
  if (busy || selected.length !== 1 || !selected[0]?.equipmentSlot) return;
  await dispatch({ type: "equip", itemId: selected[0].id });
}

async function appraiseSelectedInventoryItem(): Promise<void> {
  const selected = selectedInventoryItems();
  if (busy || selected.length !== 1 || selected[0]?.identification !== "unexamined") return;
  await dispatch({ type: "appraise", itemId: selected[0].id });
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
  updateCampaignAction();
  const selected = selectedInventoryItems();
  inventorySelectionCount.textContent = localization.format("inventory-selected-count", {
    count: selected.length,
  });
  inventoryEquip.disabled =
    busy || playerDead || selected.length !== 1 || !selected[0]?.equipmentSlot;
  inventoryUse.disabled = busy || playerDead || selected.length !== 1 || !selected[0]?.usable;
  inventoryAppraise.disabled =
    busy || playerDead || selected.length !== 1 || selected[0]?.identification !== "unexamined";
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

function updateCampaignAction(): void {
  const state = currentStatus;
  campaignRetire.disabled =
    busy ||
    playerDead ||
    !state ||
    state.campaign.status !== "victorious" ||
    state.floorId !== "demo.floor.surface" ||
    state.dungeonInstanceId != null;
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
    ["item-modifier-speed", modifiers.speed],
  ];
  for (const [key, value] of entries) {
    if (value === 0) continue;
    const modifier = document.createElement("span");
    modifier.className = "item-modifier";
    modifier.textContent = localization.format(key, { value: signedModifier(value) });
    container.append(modifier);
  }
}

function appendEquipmentBonuses(
  container: HTMLElement,
  bonuses: EquipmentBonusesDto | undefined,
): void {
  if (!bonuses) return;
  const entries: Array<[MessageKey, number]> = [
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
  ];
  for (const [key, value] of entries) {
    if (value === 0) continue;
    const label = document.createElement("span");
    label.className = "item-modifier";
    label.textContent = localization.format(key, { value: signedModifier(value) });
    container.append(label);
  }
}

function appendEquipmentPassives(
  container: HTMLElement,
  passives: EquipmentPassiveDto[] | undefined,
): void {
  for (const passive of passives ?? []) {
    const label = document.createElement("span");
    label.className = "item-modifier";
    label.textContent = localization.format(`item-passive-${passive}` as MessageKey);
    container.append(label);
  }
}

function appendItemDefenses(
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
    const label = document.createElement("span");
    label.className = "item-modifier";
    label.textContent = localization.format("item-resistance-label", {
      type: damageTypeName(resistance.damageType),
      level: localization.format(levelKey),
    });
    container.append(label);
  }
  for (const statusId of statusImmunities ?? []) {
    const label = document.createElement("span");
    label.className = "item-modifier";
    label.textContent = localization.format("item-status-immunity-label", {
      status: statusName(statusId),
    });
    container.append(label);
  }
}

function appendItemOffense(
  container: HTMLElement,
  slays: SlayDto[] | undefined,
  brands: WeaponBrandDto[] | undefined,
): void {
  for (const slay of slays ?? []) {
    const label = document.createElement("span");
    label.className = "item-modifier";
    label.textContent = localization.format(
      slay.level === "kill" ? "item-kill-label" : "item-slay-label",
      { target: slayTargetName(slay.target) },
    );
    container.append(label);
  }
  for (const brand of brands ?? []) {
    const label = document.createElement("span");
    label.className = "item-modifier";
    label.textContent = localization.format("item-brand-label", {
      brand: weaponBrandName(brand),
    });
    container.append(label);
  }
}

function slayTargetName(target: SlayTargetDto): string {
  return localization.format(`slay-target-${target}-name` as MessageKey);
}

function weaponBrandName(brand: WeaponBrandDto): string {
  return localization.format(`weapon-brand-${brand}-name` as MessageKey);
}

function appendKnownItemProperties(
  container: HTMLElement,
  properties: ItemPropertyDto[] | undefined,
): void {
  for (const property of properties ?? []) {
    const label = document.createElement("span");
    label.className = "item-property";
    label.textContent = localization.format("item-property-label", {
      property: itemPropertyName(property.nameKey),
    });
    container.append(label);
  }
}

function appendItemQuality(
  container: HTMLElement,
  quality: InventoryItemDto["quality"],
): void {
  if (!quality) return;
  const label = document.createElement("span");
  label.className = "item-quality";
  label.textContent = localization.format("item-quality-label", {
    quality: itemQualityName(quality),
  });
  container.append(label);
}

function signedModifier(value: number): string {
  return value > 0 ? `+${value}` : String(value);
}

function formatAttributeValue(value: number): string {
  return value > 18 ? `18/${value - 18}` : String(value);
}

function formatAttributeValueArgument(value: string | undefined): string {
  if (value === undefined) return "?";
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) && parsed >= 3 ? formatAttributeValue(parsed) : "?";
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
    case "ability-studied":
      return localization.format("message-ability-studied", {
        ability: contentName(event.args.target),
      });
    case "ability-forgotten":
      return localization.format("message-ability-forgotten", {
        ability: contentName(event.args.target),
      });
    case "ability-forget-unavailable":
      return localization.format("message-ability-forget-unavailable", {
        ability: contentName(event.args.target),
        reason: abilityUnavailableReason(event.args.reason),
      });
    case "ability-study-unavailable":
      return localization.format("message-ability-study-unavailable", {
        ability: contentName(event.args.target),
        reason: abilityUnavailableReason(event.args.reason),
      });
    case "ability-cast-unavailable":
      return localization.format("message-ability-cast-unavailable", {
        ability: contentName(event.args.target),
        reason: abilityUnavailableReason(event.args.reason),
      });
    case "ability-cast-success":
    case "ability-cast-failure": {
      const resolution =
        event.outcome?.type === "ability-cast" ? event.outcome.resolution : undefined;
      return localization.format(
        event.messageKey === "ability-cast-success"
          ? "message-ability-cast-success"
          : "message-ability-cast-failure",
        {
          ability: contentName(event.args.target),
          roll: resolution?.percentileRoll ?? "?",
          failure: resolution?.failurePercent ?? "?",
          cost: resolution?.resourceCost ?? "?",
        },
      );
    }
    case "ability-target-unavailable":
      return localization.format("message-ability-target-unavailable", {
        ability: contentName(event.args.target),
      });
    case "ability-landed":
      return localization.format("message-ability-landed", {
        ability: contentName(event.args.target),
      });
    case "ability-area-damage":
      return localization.format("message-ability-area-damage", {
        ability: contentName(event.args.target),
        radius: event.args.radius ?? "?",
        targets: event.args.targets ?? "0",
      });
    case "ability-beam-damage":
      return localization.format("message-ability-beam-damage", {
        ability: contentName(event.args.target),
        targets: event.args.targets ?? "0",
      });
    case "ability-cone-damage":
      return localization.format("message-ability-cone-damage", {
        ability: contentName(event.args.target),
        radius: event.args.radius ?? "?",
        targets: event.args.targets ?? "0",
      });
    case "ability-teleport": {
      const resolution =
        event.outcome?.type === "ability-teleport" ? event.outcome.resolution : undefined;
      return localization.format("message-ability-teleport", {
        ability: contentName(event.args.target),
        fromX: resolution?.from.x ?? event.args.fromX ?? "?",
        fromY: resolution?.from.y ?? event.args.fromY ?? "?",
        toX: resolution?.to.x ?? event.args.toX ?? "?",
        toY: resolution?.to.y ?? event.args.toY ?? "?",
      });
    }
    case "ability-summon":
      return localization.format("message-ability-summon", {
        ability: contentName(event.args.target),
        actor: contentName(event.args.actor),
        count: event.args.count ?? "0",
      });
    case "ability-detect":
      return localization.format("message-ability-detect", {
        ability: contentName(event.args.target),
        category: event.args.category ?? "?",
        count: event.args.count ?? "0",
      });
    case "ability-terrain-transform":
      return localization.format("message-ability-terrain-transform", {
        ability: contentName(event.args.target),
        terrain: contentName(event.args.terrain),
        count: event.args.count ?? "0",
      });
    case "ability-effects":
      return localization.format("message-ability-effects", {
        ability: contentName(event.args.target),
        count: event.args.count ?? "0",
      });
    case "monster-ability-decision": {
      const resolution =
        event.outcome?.type === "monster-ability-decision"
          ? event.outcome.resolution
          : undefined;
      const selectedAbilityId = resolution?.selectedAbilityId;
      return localization.format(
        selectedAbilityId
          ? "message-monster-ability-decision-cast"
          : "message-monster-ability-decision-fallback",
        {
          source: contentName(event.args.source),
          ability: selectedAbilityId ? contentName(selectedAbilityId) : "",
          roll: resolution?.frequencyRoll ?? event.args.roll ?? "?",
          frequency: resolution?.frequencyPercent ?? event.args.frequency ?? "?",
        },
      );
    }
    case "monster-ability-cast": {
      const resolution =
        event.outcome?.type === "monster-ability-cast" ? event.outcome.resolution : undefined;
      if (resolution?.summon) {
        const summonedKinds = resolution.summon.summonedKindIds ?? [];
        const actor =
          summonedKinds.length > 0
            ? [...new Set(summonedKinds)].map(contentName).join("、")
            : contentName(resolution.summon.actorKindId);
        return localization.format("message-monster-ability-summon", {
          source: contentName(event.args.source),
          ability: contentName(event.args.target),
          actor,
          count: resolution.summon.entityIds.length,
        });
      }
      const targets = resolution?.targets ?? [];
      const effectCount =
        targets.length > 0
          ? targets.reduce((count, target) => count + target.effects.length, 0)
          : (resolution?.effects.length ?? Number(event.args.count ?? 0));
      return localization.format("message-monster-ability-cast", {
        source: contentName(event.args.source),
        ability: contentName(event.args.target),
        count: effectCount,
        targetCount: targets.length || 1,
      });
    }
    case "summon-expired":
      return localization.format("message-summon-expired", {
        actor: contentName(event.args.actor),
      });
    case "summon-command-changed":
      return localization.format("message-summon-command-changed", {
        mode: localization.format(
          `summon-command-mode-${event.args.mode ?? "follow"}` as MessageKey,
        ),
        count: event.args.count ?? "0",
      });
    case "summon-followed-floor":
      return localization.format("message-summon-followed-floor", {
        actor: contentName(event.args.actor),
      });
    case "summon-could-not-follow":
      return localization.format("message-summon-could-not-follow", {
        actor: contentName(event.args.actor),
      });
    case "ability-hit":
      return localization.format("message-ability-hit", {
        ability: contentName(event.args.source),
        target: contentName(event.args.target),
        damage: event.args.damage ?? "?",
      });
    case "ability-slay":
      return localization.format("message-ability-slay", {
        ability: contentName(event.args.source),
        target: contentName(event.args.target),
      });
    case "ability-healed":
      return localization.format("message-ability-healed", {
        ability: contentName(event.args.source),
        amount: event.args.amount ?? "?",
      });
    case "resource-recovered":
      return localization.format("message-resource-recovered", {
        resource: contentName(event.args.target),
        amount: event.args.amount ?? "?",
      });
    case "resource-gained":
      return localization.format("message-resource-gained", {
        resource: contentName(event.args.target),
        amount: event.args.amount ?? "?",
      });
    case "monster-blinked":
      return localization.format("message-monster-blinked", {
        source: contentName(event.args.source),
      });
    case "monster-teleported":
      return localization.format("message-monster-teleported", {
        source: contentName(event.args.source),
      });
    case "monster-dragged-target":
      return localization.format("message-monster-dragged-target", {
        source: contentName(event.args.source),
        target: contentName(event.args.target),
      });
    case "monster-banished-target":
      return localization.format("message-monster-banished-target", {
        source: contentName(event.args.source),
        target: contentName(event.args.target),
      });
    case "rest-completed":
    case "rest-interrupted":
      return localization.format(
        event.messageKey === "rest-completed"
          ? "message-rest-completed"
          : "message-rest-interrupted",
        {
          turns: event.args.turns ?? "0",
          reason: restStopReason(event.args.reason),
        },
      );
    case "game-wait":
      return localization.format("message-game-wait");
    case "game-move-blocked":
      return localization.format("message-move-blocked");
    case "floor-transition":
      return localization.format("message-floor-transition", {
        from: floorName(event.args.from),
        to: floorName(event.args.to),
      });
    case "floor-transition-unavailable":
      return localization.format("message-floor-transition-unavailable");
    case "floor-expedition-ended":
      return localization.format("message-floor-expedition-ended");
    case "dungeon-entrance-guardian-defeated":
      return localization.format("message-dungeon-entrance-guardian-defeated", {
        dungeon: event.args.dungeon ?? "?",
      });
    case "campaign-victorious":
      return localization.format("message-campaign-victorious", {
        score: event.args.score ?? "?",
      });
    case "campaign-retired":
      return localization.format("message-campaign-retired", {
        score: event.args.score ?? "?",
      });
    case "campaign-retire-unavailable":
      return localization.format("message-campaign-retire-unavailable");
    case "player-experience-gained":
      return localization.format("message-player-experience-gained", {
        amount: event.args.amount ?? "?",
        total: event.args.total ?? "?",
      });
    case "player-level-gained":
      return localization.format("message-player-level-gained", {
        level: event.args.level ?? "?",
        maxHp: event.args.maxHp ?? "?",
        pending: event.args.pendingAttributeIncreases ?? "?",
      });
    case "player-level-cap-unlocked":
      return localization.format("message-player-level-cap-unlocked", {
        levelCap: event.args.levelCap ?? "?",
        attributeCap: event.args.attributeIndexCap ?? "?",
      });
    case "player-attribute-increased":
      return localization.format("message-player-attribute-increased", {
        attribute: localization.format(
          `attribute-${event.args.attribute ?? "unknown"}` as MessageKey,
        ),
        natural: formatAttributeValueArgument(event.args.natural),
        effective: formatAttributeValueArgument(event.args.effective),
        pending: event.args.pendingAttributeIncreases ?? "?",
      });
    case "player-attribute-increase-unavailable":
      return localization.format("message-player-attribute-increase-unavailable", {
        attribute: localization.format(
          `attribute-${event.args.attribute ?? "unknown"}` as MessageKey,
        ),
      });
    case "floor-one-shot-closed":
      return localization.format("message-floor-one-shot-closed");
    case "task-completed":
      return localization.format("message-task-completed");
    case "task-failed":
      return localization.format("message-task-failed");
    case "task-abandoned":
      return localization.format("message-task-abandoned");
    case "task-abandon-unavailable":
      return localization.format("message-task-abandon-unavailable");
    case "task-paused":
      return localization.format("message-task-paused");
    case "task-resumed":
      return localization.format("message-task-resumed");
    case "task-rewarded":
      return localization.format("message-task-rewarded", {
        target: visibleItemNameForKind(event.args.target),
        quantity: event.args.quantity ?? "?",
      });
    case "door-opened":
      return localization.format("message-door-opened");
    case "door-open-unavailable":
      return localization.format("message-door-open-unavailable");
    case "door-closed":
      return localization.format("message-door-closed");
    case "door-close-unavailable":
      return localization.format("message-door-close-unavailable");
    case "terrain-trap-triggered":
      return localization.format("message-terrain-trap-triggered", {
        damage: damageResolution(event)?.finalDamage ?? "?",
      });
    case "terrain-trap-disarmed":
      return localization.format("message-terrain-trap-disarmed");
    case "terrain-trap-disarm-failed":
      return localization.format("message-terrain-trap-disarm-failed");
    case "terrain-trap-disarm-unavailable":
      return localization.format("message-terrain-trap-disarm-unavailable");
    case "terrain-dug":
      return localization.format("message-terrain-dug");
    case "terrain-dig-failed":
      return localization.format("message-terrain-dig-failed");
    case "terrain-dig-unavailable":
      return localization.format("message-terrain-dig-unavailable");
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
    case "combat-monster-entity-miss":
      return localization.format("message-combat-monster-entity-miss", {
        source: contentName(event.args.source),
        target: contentName(event.args.target),
      });
    case "combat-monster-entity-hit":
      return localization.format("message-combat-monster-entity-hit", {
        source: contentName(event.args.source),
        target: contentName(event.args.target),
        damage: damageResolution(event)?.finalDamage ?? "?",
      });
    case "combat-monster-entity-slew":
      return localization.format("message-combat-monster-entity-slew", {
        source: contentName(event.args.source),
        target: contentName(event.args.target),
      });
    case "combat-monster-fled":
      return localization.format("message-combat-monster-fled", {
        source: contentName(event.args.source),
        target: contentName(event.args.target),
      });
    case "combat-monster-kept-distance":
      return localization.format("message-combat-monster-kept-distance", {
        source: contentName(event.args.source),
        target: contentName(event.args.target),
      });
    case "combat-summon-miss":
      return localization.format("message-combat-summon-miss", {
        source: contentName(event.args.source),
        target: contentName(event.args.target),
      });
    case "combat-summon-hit":
      return localization.format("message-combat-summon-hit", {
        source: contentName(event.args.source),
        target: contentName(event.args.target),
        damage: damageResolution(event)?.finalDamage ?? event.args.damage ?? "?",
      });
    case "combat-summon-slay":
      return localization.format("message-combat-summon-slay", {
        source: contentName(event.args.source),
        target: contentName(event.args.target),
      });
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
    case "status-confused-move":
      return localization.format("message-status-confused-move");
    case "status-paralyzed":
      return localization.format("message-status-paralyzed");
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
    case "item-appraise-success":
      return localization.format("message-item-appraise-success", {
        target: visibleItemNameForKind(event.args.target),
        quality: itemQualityName(event.args.quality),
      });
    case "item-appraise-unavailable":
      return localization.format("message-item-appraise-unavailable");
    case "item-property-discovered":
      return localization.format("message-item-property-discovered", {
        target: visibleItemNameForKind(event.args.target),
        property: itemPropertyName(event.args.propertyNameKey),
      });
    case "loot-drop":
      return localization.format("message-loot-drop", {
        source: contentName(event.args.source),
        target: visibleItemNameForKind(event.args.target),
        quantity: event.args.quantity ?? "?",
      });
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
    light: "damage-type-light-name",
    dark: "damage-type-dark-name",
    confusion: "damage-type-confusion-name",
    nether: "damage-type-nether-name",
    nexus: "damage-type-nexus-name",
    sound: "damage-type-sound-name",
    shards: "damage-type-shards-name",
    chaos: "damage-type-chaos-name",
    disenchant: "damage-type-disenchant-name",
    time: "damage-type-time-name",
    mana: "damage-type-mana-name",
    gravity: "damage-type-gravity-name",
    inertia: "damage-type-inertia-name",
    plasma: "damage-type-plasma-name",
    force: "damage-type-force-name",
    nuke: "damage-type-nuke-name",
    disintegrate: "damage-type-disintegrate-name",
    storm: "damage-type-storm-name",
    "holy-fire": "damage-type-holy-fire-name",
    "hell-fire": "damage-type-hell-fire-name",
    ice: "damage-type-ice-name",
    water: "damage-type-water-name",
    psi: "damage-type-psi-name",
    curse: "damage-type-curse-name",
  };
  return localization.format(keys[damageType]);
}

function floorName(id: string | undefined): string {
  if (id === "demo.floor.surface") {
    return localization.format("world-demo-original-lab-name");
  }
  if (id === "demo.floor.echo-depth-1") {
    return localization.format("floor-demo-echo-depth-1-name");
  }
  return id ?? "?";
}

function contentName(id: string | undefined): string {
  if (id === "demo.resource.mana") {
    return localization.format("resource-demo-mana-name");
  }
  if (id === "demo.ability.resonant-bolt") {
    return localization.format("ability-demo-resonant-bolt-name");
  }
  if (id === "demo.ability.harmonic-spark") {
    return localization.format("ability-demo-harmonic-spark-name");
  }
  if (id === "demo.ability.echo-burst") {
    return localization.format("ability-demo-echo-burst-name");
  }
  if (id === "demo.ability.echo-companion") {
    return localization.format("ability-demo-echo-companion-name");
  }
  if (id === "demo.ability.echo-pulse") {
    return localization.format("ability-demo-echo-pulse-name");
  }
  if (id === "demo.ability.echo-sight") {
    return localization.format("ability-demo-echo-sight-name");
  }
  if (id === "demo.ability.echo-delving") {
    return localization.format("ability-demo-echo-delving-name");
  }
  if (id === "demo.ability.echo-rampart") {
    return localization.format("ability-demo-echo-rampart-name");
  }
  if (id === "demo.ability.echo-binding") {
    return localization.format("ability-demo-echo-binding-name");
  }
  if (id === "demo.ability.echo-quickening") {
    return localization.format("ability-demo-echo-quickening-name");
  }
  if (id === "demo.ability.mending-echo") {
    return localization.format("ability-demo-mending-echo-name");
  }
  if (id === "demo.item.echo-primer") {
    return localization.format("item-demo-echo-primer-name");
  }
  if (id === "demo.item.stillwater-notes") {
    return localization.format("item-demo-stillwater-notes-name");
  }
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
  if (id === "demo.actor.echo-cantor") {
    return localization.format("actor-demo-echo-cantor-name");
  }
  if (id === "demo.terrain.floor") {
    return localization.format("terrain-demo-floor-name");
  }
  if (id === "demo.terrain.echo-rubble") {
    return localization.format("terrain-demo-echo-rubble-name");
  }
  return localization.format(
    id?.startsWith("demo.item.") ? "item-unknown-name" : "actor-unknown-name",
  );
}

function abilityUnavailableReason(reason: string | undefined): string {
  return localization.format(`ability-unavailable-${reason ?? "unknown"}` as MessageKey);
}

function restStopReason(reason: string | undefined): string {
  return localization.format(`rest-stop-${reason ?? "unknown"}` as MessageKey);
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
    case "item-demo-echo-primer-name":
    case "item-demo-stillwater-notes-name":
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

function itemPropertyName(nameKey: string | undefined): string {
  if (nameKey === "affix-demo-harmonic-edge-name") {
    return localization.format(nameKey);
  }
  return localization.format("item-unknown-name");
}

function itemQualityName(quality: string | undefined): string {
  switch (quality) {
    case "ordinary":
      return localization.format("item-quality-ordinary");
    case "fine":
      return localization.format("item-quality-fine");
    case "exceptional":
      return localization.format("item-quality-exceptional");
    default:
      return "?";
  }
}

const EQUIPMENT_SLOT_TYPE_KEYS: Record<string, MessageKey> = {
  charm: "equipment-slot-charm",
  weapon: "equipment-slot-weapon",
  launcher: "equipment-slot-launcher",
  body: "equipment-slot-body",
  head: "equipment-slot-head",
  shield: "equipment-slot-shield",
  cloak: "equipment-slot-cloak",
  gloves: "equipment-slot-gloves",
  boots: "equipment-slot-boots",
  ring: "equipment-slot-ring",
  amulet: "equipment-slot-amulet",
  light: "equipment-slot-light",
};

function equipmentSlotName(slotType: string | undefined): string {
  const key = slotType ? EQUIPMENT_SLOT_TYPE_KEYS[slotType] : undefined;
  if (key) return localization.format(key);
  return localization.format("equipment-slot-unknown", { slot: slotType ?? "?" });
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
  if (statusId === "rfb.status.confusion") {
    return localization.format("status-confusion-name");
  }
  if (statusId === "rfb.status.blindness") {
    return localization.format("status-blindness-name");
  }
  if (statusId === "rfb.status.paralysis") {
    return localization.format("status-paralysis-name");
  }
  return localization.format("status-unknown-name");
}

function addLocalizedMessage(
  key: MessageKey,
  args: Record<string, string | number> | undefined,
  kind: string,
): void {
  appendMessageRecord({
    source: "key",
    turn: turnValue.textContent ?? "0",
    kind,
    key,
    args,
  });
}

function addGameEvent(event: GameEventDto): void {
  appendMessageRecord({
    source: "event",
    turn: turnValue.textContent ?? "0",
    kind: event.kind,
    event,
  });
}

// New messages append a single row; the full rebuild in renderMessages is
// reserved for the clear button and language switches.
function appendMessageRecord(record: MessageRecord): void {
  messageRecords.push(record);
  while (messageRecords.length > MESSAGE_HISTORY_LIMIT) {
    messageRecords.shift();
    messageList.firstElementChild?.remove();
  }
  renderMessage(record);
  messageList.scrollTop = messageList.scrollHeight;
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

function startProjectileTargeting(): void {
  if (busy || playerDead || !currentStatus) return;
  startTargetingWithSpec(
    currentStatus.player.projectileProfile?.targetSpec,
    { type: "projectile" },
  );
}

function startAbilityTargeting(ability: AbilityDto): void {
  if (busy || playerDead || !currentStatus || !ability.canCast) return;
  startTargetingWithSpec(ability.targetSpec, {
    type: "ability",
    abilityId: ability.id,
  });
}

function startTargetingWithSpec(
  spec: TargetSpecDto | null | undefined,
  intent: TargetingIntent,
): void {
  if (!currentStatus) return;
  const next = beginTargeting(currentStatus.player.position, spec ?? undefined);
  if (!next) {
    addLocalizedMessage("message-target-mode-unavailable", undefined, "system");
    renderTargeting();
    return;
  }
  targeting = next;
  targetingIntent = intent;
  addLocalizedMessage("message-target-mode-started", undefined, "system");
  renderTargeting();
}

function cancelTargeting(announce = true): void {
  if (!targeting) return;
  targeting = undefined;
  targetingIntent = undefined;
  if (announce) addLocalizedMessage("message-target-mode-cancelled", undefined, "system");
  renderTargeting();
}

async function confirmTargeting(): Promise<void> {
  const state = targeting;
  const status = currentStatus;
  const intent = targetingIntent;
  if (!state || !status || !intent || busy || playerDead) return;
  const target = targetSelectionAtCursor(state, status.entities);
  if (!target) {
    addLocalizedMessage("message-target-selection-invalid", undefined, "system");
    return;
  }
  cancelTargeting(false);
  await dispatch(
    intent.type === "ability"
      ? { type: "cast-ability", abilityId: intent.abilityId, target }
      : { type: "fire-target", target },
  );
}

function targetSpecForIntent(
  state: GameSnapshot | GameUpdate,
  intent: TargetingIntent,
): TargetSpecDto | null | undefined {
  if (intent.type === "projectile") return state.player.projectileProfile?.targetSpec;
  return (state.player.abilities ?? []).find(
    (ability) => ability.id === intent.abilityId && ability.canCast,
  )?.targetSpec;
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
  mapHost.dataset.targetingAction = targetingIntent?.type ?? "none";
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
  if (key === ">" || key === "<") return { type: "traverse-stairs" };
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
