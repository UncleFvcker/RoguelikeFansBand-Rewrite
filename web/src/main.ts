// SPDX-License-Identifier: MPL-2.0

import "./styles.css";

import { CoreClient } from "./core-client";
import { MapRenderer } from "./map-renderer";
import type { Direction, GameCommand, GameEventDto, GameSnapshot, GameUpdate } from "./protocol";

const core = new CoreClient();
const renderer = new MapRenderer();
let busy = false;

const mapHost = element<HTMLElement>("map-host");
const connectionStatus = element<HTMLElement>("connection-status");
const messageList = element<HTMLOListElement>("message-list");
const turnValue = element<HTMLElement>("turn-value");
const hpValue = element<HTMLElement>("hp-value");
const positionValue = element<HTMLElement>("position-value");
const hashValue = element<HTMLElement>("hash-value");
const saveButton = element<HTMLButtonElement>("save-button");
const loadInput = element<HTMLInputElement>("load-input");
const clearMessages = element<HTMLButtonElement>("clear-messages");

void start();

async function start(): Promise<void> {
  try {
    const snapshot = await core.initialize("42");
    await renderer.initialize(mapHost, snapshot.width, snapshot.height);
    renderer.applySnapshot(snapshot);
    renderStatus(snapshot);
    addMessage("Rust/WASM 核心已启动；地图与文字由不同渲染层管理。", "system");
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
loadInput.addEventListener("change", () => void importSave());
clearMessages.addEventListener("click", () => messageList.replaceChildren());
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
    const blob = new Blob([bytes.slice().buffer as ArrayBuffer], {
      type: "application/octet-stream",
    });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = "rfb-rewrite-demo.rfbsave";
    anchor.click();
    URL.revokeObjectURL(url);
    addMessage("已导出带校验和的 .rfbsave 存档。", "system");
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
    renderer.applySnapshot(snapshot);
    renderStatus(snapshot);
    addMessage("存档校验与载入成功。", "system");
  } catch (error) {
    showError(error);
  }
}

function renderStatus(state: GameSnapshot | GameUpdate): void {
  turnValue.textContent = String(state.turn);
  hpValue.textContent = `${state.player.hp} / ${state.player.maxHp}`;
  positionValue.textContent = `${state.player.position.x}, ${state.player.position.y}`;
  hashValue.textContent = state.stateHash.slice(0, 12);
  hashValue.title = state.stateHash;
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

function commandForKeyboardEvent(event: KeyboardEvent): GameCommand | undefined {
  const key = event.key.toLowerCase();
  const directionByCode: Partial<Record<string, Direction>> = {
    Numpad8: "north",
    Numpad9: "north-east",
    Numpad6: "east",
    Numpad3: "south-east",
    Numpad2: "south",
    Numpad1: "south-west",
    Numpad4: "west",
    Numpad7: "north-west",
  };
  const directionByKey: Record<string, Direction> = {
    arrowup: "north",
    w: "north",
    k: "north",
    "8": "north",
    arrowright: "east",
    d: "east",
    l: "east",
    "6": "east",
    arrowdown: "south",
    s: "south",
    j: "south",
    "2": "south",
    arrowleft: "west",
    a: "west",
    h: "west",
    "4": "west",
    q: "north-west",
    y: "north-west",
    "7": "north-west",
    home: "north-west",
    e: "north-east",
    u: "north-east",
    "9": "north-east",
    pageup: "north-east",
    z: "south-west",
    b: "south-west",
    "1": "south-west",
    end: "south-west",
    c: "south-east",
    n: "south-east",
    "3": "south-east",
    pagedown: "south-east",
  };
  if (key === "." || key === "5" || key === " " || event.code === "Numpad5") {
    return { type: "wait" };
  }
  const direction = directionByCode[event.code] ?? directionByKey[key];
  return direction ? { type: "move", direction } : undefined;
}

function isTextInput(target: EventTarget | null): boolean {
  return target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement;
}

function element<T extends HTMLElement>(id: string): T {
  const found = document.getElementById(id);
  if (!found) throw new Error(`Missing element #${id}`);
  return found as T;
}
