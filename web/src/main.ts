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
import { parseZoomLevel, type ZoomLevel } from "./camera";
import type {
  Direction,
  GameCommand,
  GameEventDto,
  GameSnapshot,
  GameUpdate,
  InventoryItemDto,
} from "./protocol";
import { TauriNativeTransport } from "./tauri-native-transport";
import type { TilesetWarning } from "./tileset-runtime";

const core = new TauriNativeTransport();
const renderer = new MapRenderer();
let busy = false;

const mapHost = element<HTMLElement>("map-host");
const connectionStatus = element<HTMLElement>("connection-status");
const messageList = element<HTMLOListElement>("message-list");
const turnValue = element<HTMLElement>("turn-value");
const hpValue = element<HTMLElement>("hp-value");
const positionValue = element<HTMLElement>("position-value");
const hashValue = element<HTMLElement>("hash-value");
const inventoryCount = element<HTMLElement>("inventory-count");
const inventoryList = element<HTMLUListElement>("inventory-list");
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
let currentInventory: InventoryItemDto[] = [];
const messageRecords: MessageRecord[] = [];
inputPresetSelect.value = inputPreset;
tilesetPresetSelect.value = tilesetPreset;
cameraModeSelect.value = cameraMode;
zoomSelect.value = String(zoom);
languageSelect.value = localization.locale;
localization.localizeDocument();
renderConnectionStatus();
renderInputHelp();

void start();

async function start(): Promise<void> {
  try {
    const snapshot = await core.initialize("42");
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
    renderer.applySnapshot(snapshot);
    renderStatus(snapshot);
    renderInventory(snapshot.inventory);
    addLocalizedMessage("message-core-started", undefined, "system");
    announceTileset(tileset.id, tileset.warnings);
    connectionState = "ready";
    renderConnectionStatus();
  } catch (error) {
    showError(error);
  }
}

window.addEventListener("keydown", (event) => {
  const command = commandForKeyboardEvent(event);
  if (!command || busy || isTextInput(event.target)) return;
  event.preventDefault();
  void dispatch(command);
});

saveButton.addEventListener("click", () => void exportSave());
replayButton.addEventListener("click", () => void exportReplay());
loadInput.addEventListener("change", () => void importSave());
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
});
zoomSelect.addEventListener("change", () => {
  zoom = parseZoomLevel(zoomSelect.value);
  zoomSelect.value = String(zoom);
  localStorage.setItem(ZOOM_STORAGE_KEY, String(zoom));
  renderer.setZoom(zoom);
});
languageSelect.addEventListener("change", () => {
  const locale = isSupportedLocale(languageSelect.value) ? languageSelect.value : "zh-CN";
  localization.setLocale(locale);
  localStorage.setItem(LOCALE_STORAGE_KEY, locale);
  localization.localizeDocument();
  languageSelect.value = locale;
  renderer.setCanvasLabel(localization.format("map-aria-label"));
  renderConnectionStatus();
  renderInputHelp();
  renderInventory(currentInventory);
  renderMessages();
});
window.addEventListener("beforeunload", () => {
  renderer.destroy();
  core.dispose();
});

async function dispatch(command: GameCommand): Promise<void> {
  busy = true;
  try {
    const update = await core.dispatch(command);
    renderer.applyUpdate(update);
    renderStatus(update);
    renderInventory(update.inventory);
    for (const event of update.events) addGameEvent(event);
  } catch (error) {
    showError(error);
  } finally {
    busy = false;
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
    renderContentMetadata(snapshot);
    renderer.applySnapshot(snapshot);
    renderStatus(snapshot);
    renderInventory(snapshot.inventory);
    addLocalizedMessage("message-save-loaded", undefined, "system");
  } catch (error) {
    showError(error);
  }
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
  }
}

function renderStatus(state: GameSnapshot | GameUpdate): void {
  turnValue.textContent = String(state.turn);
  hpValue.textContent = `${state.player.hp} / ${state.player.maxHp}`;
  positionValue.textContent = `${state.player.position.x}, ${state.player.position.y}`;
  hashValue.textContent = state.stateHash.slice(0, 12);
  hashValue.title = state.stateHash;
  mapHost.dataset.itemCount = String(state.items.length);
  mapHost.dataset.inventoryStackCount = String(state.inventory.length);
}

function renderContentMetadata(snapshot: GameSnapshot): void {
  mapHost.dataset.contentId = snapshot.contentId;
  mapHost.dataset.contentHash = snapshot.contentHash;
  mapHost.dataset.worldId = snapshot.worldId;
  mapHost.dataset.contentVisualCount = String(snapshot.contentVisuals.length);
}

function renderInventory(inventory: InventoryItemDto[]): void {
  currentInventory = inventory.map((item) => ({ ...item }));
  inventoryCount.textContent = localization.format("inventory-stack-count", {
    count: inventory.length,
  });
  inventoryList.replaceChildren();
  if (inventory.length === 0) {
    const empty = document.createElement("li");
    empty.className = "inventory-empty";
    empty.textContent = localization.format("inventory-empty");
    inventoryList.append(empty);
    return;
  }
  for (const item of inventory) {
    const row = document.createElement("li");
    row.className = "inventory-item";
    row.dataset.itemId = item.id;
    const name = document.createElement("span");
    name.textContent = contentName(item.kindId);
    const quantity = document.createElement("span");
    quantity.className = "inventory-quantity";
    quantity.textContent = localization.format("inventory-quantity", {
      quantity: item.quantity,
    });
    row.append(name, quantity);
    inventoryList.append(row);
  }
}

function formatEvent(event: GameEventDto): string {
  switch (event.messageKey) {
    case "game-wait":
      return localization.format("message-game-wait");
    case "game-move-blocked":
      return localization.format("message-move-blocked");
    case "combat-player-hit":
      return localization.format("message-combat-hit", {
        target: contentName(event.args.target),
        damage: event.args.damage ?? "?",
      });
    case "combat-player-slay":
      return localization.format("message-combat-slay", {
        target: contentName(event.args.target),
      });
    case "item-pickup-success":
      return localization.format("message-item-pickup-success", {
        target: contentName(event.args.target),
        quantity: event.args.quantity ?? "?",
      });
    case "item-pickup-none":
      return localization.format("message-item-pickup-none");
    default:
      return localization.format("message-unknown-event", { key: event.messageKey });
  }
}

function contentName(id: string | undefined): string {
  if (id === "demo.item.luminous-shard") {
    return localization.format("item-demo-luminous-shard-name");
  }
  if (id === "demo.actor.ember-mote") {
    return localization.format("actor-demo-ember-mote-name");
  }
  return localization.format(
    id?.startsWith("demo.item.") ? "item-unknown-name" : "actor-unknown-name",
  );
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

function commandForKeyboardEvent(event: KeyboardEvent): GameCommand | undefined {
  const key = event.key.toLowerCase();
  if (key === "g") return { type: "pick-up" };
  if (inputPreset === "numpad") {
    const direction = NUMPAD_DIRECTIONS[event.code];
    if (event.code === "Numpad5") return { type: "wait" };
    return direction ? { type: "move", direction } : undefined;
  }

  if (inputPreset === "vi") {
    const direction = VI_DIRECTIONS[key];
    if (key === ".") return { type: "wait" };
    return direction ? { type: "move", direction } : undefined;
  }

  const direction = WASD_DIRECTIONS[key];
  if (key === " ") return { type: "wait" };
  return direction ? { type: "move", direction } : undefined;
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
