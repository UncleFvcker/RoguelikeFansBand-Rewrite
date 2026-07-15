// SPDX-License-Identifier: MPL-2.0

import "./styles.css";

import { MapRenderer } from "./map-renderer";
import type { Direction, GameCommand, GameEventDto, GameSnapshot, GameUpdate } from "./protocol";
import { TauriNativeTransport } from "./tauri-native-transport";

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
const replayButton = element<HTMLButtonElement>("replay-button");
const saveButton = element<HTMLButtonElement>("save-button");
const loadInput = element<HTMLInputElement>("load-input");
const clearMessages = element<HTMLButtonElement>("clear-messages");
const inputPresetSelect = element<HTMLSelectElement>("input-preset");
const tilesetPresetSelect = element<HTMLSelectElement>("tileset-preset");
const controlsHelp = element<HTMLElement>("controls-help");

type InputPreset = "numpad" | "vi" | "wasd";
type TilesetPreset = "ascii" | "image";
const INPUT_PRESET_STORAGE_KEY = "rfb.input-preset";
const TILESET_PRESET_STORAGE_KEY = "rfb.tileset-preset";
const TILESET_MANIFESTS: Record<TilesetPreset, string> = {
  ascii: "/tilesets/ascii-default/tileset.json",
  image: "/tilesets/image-demo/tileset.json",
};
let inputPreset = readInputPreset();
let tilesetPreset = readTilesetPreset();
inputPresetSelect.value = inputPreset;
tilesetPresetSelect.value = tilesetPreset;
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
    );
    renderer.applySnapshot(snapshot);
    renderStatus(snapshot);
    addMessage("Tauri 原生 Rust 核心已启动；地图与文字由不同渲染层管理。", "system");
    announceTileset(tileset.id, tileset.warnings);
    connectionStatus.textContent = "核心已连接";
    connectionStatus.classList.add("ready");
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
clearMessages.addEventListener("click", () => messageList.replaceChildren());
inputPresetSelect.addEventListener("change", () => {
  inputPreset = isInputPreset(inputPresetSelect.value) ? inputPresetSelect.value : "numpad";
  localStorage.setItem(INPUT_PRESET_STORAGE_KEY, inputPreset);
  renderInputHelp();
  addMessage(`移动键位已切换为${inputPresetName(inputPreset)}。`, "system");
});
tilesetPresetSelect.addEventListener("change", () => void changeTileset());
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
    for (const event of update.events) addMessage(formatEvent(event), event.kind);
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
    addMessage("已导出带校验和的 .rfbsave 存档。", "system");
  } catch (error) {
    showError(error);
  }
}

async function exportReplay(): Promise<void> {
  try {
    const bytes = await core.exportReplay();
    downloadBytes(bytes, "rfb-rewrite-diagnostic.rfbreplay");
    addMessage("已导出不包含存档和本地路径的诊断回放。", "system");
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
    addMessage("存档校验与载入成功。", "system");
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
    addMessage(`地图外观载入失败：${message}`, "error");
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
}

function renderContentMetadata(snapshot: GameSnapshot): void {
  mapHost.dataset.contentId = snapshot.contentId;
  mapHost.dataset.contentHash = snapshot.contentHash;
  mapHost.dataset.worldId = snapshot.worldId;
  mapHost.dataset.contentVisualCount = String(snapshot.contentVisuals.length);
  mapHost.dataset.itemCount = String(snapshot.items.length);
}

function formatEvent(event: GameEventDto): string {
  const target = entityName(event.args.target);
  switch (event.messageKey) {
    case "game-wait":
      return "你在寂静中停留了一回合。";
    case "game-move-blocked":
      return "前方的结构阻挡了道路。";
    case "combat-player-hit":
      return `你击中了${target}，造成 ${event.args.damage ?? "?"} 点伤害。`;
    case "combat-player-slay":
      return `${target}熄灭了。`;
    default:
      return `[${event.messageKey}]`;
  }
}

function entityName(id: string | undefined): string {
  if (id === "demo.actor.ember-mote") return "发光微粒";
  return "未知实体";
}

function addMessage(text: string, kind: string): void {
  const item = document.createElement("li");
  item.className = `message message-${kind.replaceAll(".", "-")}`;
  const turn = document.createElement("span");
  turn.className = "message-turn";
  turn.textContent = turnValue.textContent ?? "0";
  const content = document.createElement("span");
  content.textContent = text;
  item.append(turn, content);
  messageList.append(item);
  messageList.scrollTop = messageList.scrollHeight;
}

function showError(error: unknown): void {
  const message = error instanceof Error ? error.message : String(error);
  connectionStatus.textContent = "核心错误";
  connectionStatus.classList.add("error");
  addMessage(`错误：${message}`, "error");
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

function isTilesetPreset(value: string | null): value is TilesetPreset {
  return value === "ascii" || value === "image";
}

function announceTileset(id: string, warnings: readonly string[]): void {
  addMessage(`地图外观已载入：${id}。`, "system");
  for (const warning of warnings) addMessage(warning, "system");
}

function renderInputHelp(): void {
  const help: Record<InputPreset, string> = {
    numpad: "小键盘 1–9 八向移动，5 等待。NumLock 开关均可使用。",
    vi: "HJKL 四向移动，YUBN 斜向移动，句点等待。",
    wasd: "WASD 四向移动，QEZC 斜向移动，空格等待。",
  };
  controlsHelp.textContent = `${help[inputPreset]} 撞向发光微粒即可攻击。`;
}

function inputPresetName(preset: InputPreset): string {
  const names: Record<InputPreset, string> = {
    numpad: "小键盘预设",
    vi: "Vi 键位预设",
    wasd: "WASD 预设",
  };
  return names[preset];
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
