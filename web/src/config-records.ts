// SPDX-License-Identifier: MPL-2.0
import type { AppState } from "./app-state";
import type { Localization } from "./localization";
import type { GameCommand } from "./protocol";
import type { InputPreset } from "./input-controller";
import { commandMenuEntries } from "./command-shortcuts.ts";
import { CommandRecording, MAX_MACRO_STEPS, isMacroCommand, keyToken, originalKey, parseBindings, type KeyBinding } from "./command-recording.ts";
import { knownMapGlyphs } from "./map-intelligence.ts";

import type { PreferencesClient } from "./preferences";
export { parseBindings } from "./command-recording.ts";

type View = "input-config" | "command-menu" | "notes" | "screen-export" | "record-register" | "play-register";
type Note = { id: string; player: string; floor: string; turn: number; text: string; map: string };
const NOTES_KEY = "rfb.player-notes.v1";

export function mapText(state: AppState, width = state.mapWidth, height = state.mapHeight): string {
  const player = state.status!.player.position;
  width = Math.min(width, state.mapWidth); height = Math.min(height, state.mapHeight);
  const left = Math.max(0, Math.min(state.mapWidth - width, player.x - Math.floor(width / 2)));
  const top = Math.max(0, Math.min(state.mapHeight - height, player.y - Math.floor(height / 2)));
  const glyphs = knownMapGlyphs(state);
  return Array.from({ length: height }, (_, y) => Array.from({ length: width }, (_, x) =>
    glyphs.get(`${left + x},${top + y}`) ?? " ").join("")).join("\n");
}

type ConfigRecordsOptions = {
    state: AppState; localization: Localization; document: Document; storage: Storage;
    preferences: PreferencesClient; saveBindings: (bindings: KeyBinding[], revision: number) => Promise<void>;
    preset: () => InputPreset; execute: (key: KeyboardEventInit) => void;
    repeat: (command: GameCommand) => Promise<void>; play: (commands: readonly GameCommand[]) => Promise<void>;
    lastCommand: () => GameCommand | undefined; describe: () => string; location: () => string;
    capturePng: () => string; download: (bytes: Uint8Array, name: string) => void;
    message: (key: string, args?: Record<string, string | number>) => void; error: (error: unknown) => void;
};

export class ConfigRecords {
  readonly recording = new CommandRecording();
  readonly #options: ConfigRecordsOptions;
  readonly #dialog: HTMLDialogElement;
  readonly #body: HTMLElement;
  readonly #title: HTMLElement;
  #bindings: KeyBinding[] | undefined;
  #bindingRevision: number | undefined;
  #savingBindings = false;
  #bindingError = false;
  #view: View = "input-config";
  #register = "a";
  #snapshot: AppState["status"];
  #generation = 0;

  constructor(options: ConfigRecordsOptions) {
    this.#options = options;
    const d = options.document;
    this.#dialog = d.createElement("dialog"); this.#dialog.id = "config-records-dialog";
    this.#dialog.className = "item-target-dialog config-records-dialog"; this.#dialog.tabIndex = -1;
    this.#title = d.createElement("h2"); this.#title.id = "config-records-title";
    this.#dialog.setAttribute("aria-labelledby", this.#title.id);
    this.#body = d.createElement("div"); this.#dialog.append(this.#title, this.#body); d.body.append(this.#dialog);
    this.#dialog.addEventListener("keydown", event => {
      if (event.isComposing || event.repeat || event.ctrlKey || event.altKey || event.metaKey) return;
      if (event.key === "Escape") { event.preventDefault(); this.close(); return; }
      if ((event.target as HTMLElement).closest("input, textarea, select")) return;
      if (["record-register", "play-register"].includes(this.#view) && /^[a-zA-Z0-9.]$/.test(event.key)) {
        event.preventDefault(); this.#chooseRegister(event.key);
      }
    });
    d.defaultView!.addEventListener("keydown", event => {
      if (event.key === "Escape" && !event.isComposing && !event.repeat && !event.ctrlKey && !event.altKey && !event.metaKey &&
          !(event.target as HTMLElement | null)?.closest?.("input, textarea, select")) this.finishRecording();
    }, true);
    this.#dialog.addEventListener("cancel", event => { event.preventDefault(); this.close(); });
  }

  hasBinding(event: KeyboardEvent): boolean { return !!this.#binding(event); }
  handleBinding(event: KeyboardEvent, execute: (key: KeyboardEventInit) => void): boolean {
    const binding = this.#binding(event);
    if (!binding) return false;
    if (binding.action.startsWith("Register:")) this.#chooseRegister(binding.action.slice(9), false);
    else execute(originalKey(binding.action));
    return true;
  }
  #binding(event: KeyboardEvent): KeyBinding | undefined {
    if (event.isComposing || event.repeat) return;
    const trigger = keyToken(event);
    return this.#options.preferences.snapshot?.preferences.keyBindings.find(binding => binding.preset === this.#options.preset() && binding.trigger === trigger);
  }
  observe(command: GameCommand | undefined): void {
    const outcome = this.recording.observe(command, this.#options.describe());
    if (outcome) this.#options.message(`cfg-record-${outcome}`);
    this.#recordStatus();
  }
  reconcileStatus(): void {
    const after = this.#options.state.status;
    if (this.#snapshot && after && (after.floorId !== this.#snapshot.floorId || after.mapScale !== this.#snapshot.mapScale ||
        ("mapTranslation" in after && (after.mapTranslation?.x || after.mapTranslation?.y)))) this.reset();
    if (this.#dialog.open && this.#snapshot !== after) this.close();
    this.#snapshot = after;
  }
  reset(): void { this.#generation++; this.close(); this.recording.reset(); this.#snapshot = undefined; this.#recordStatus(); }
  finishRecording(): void { this.recording.finish(); this.#recordStatus(); }
  #recordStatus(): void {
    const status = this.#options.document.getElementById("command-record-status");
    if (!status) return;
    status.hidden = !this.recording.recording;
    status.textContent = this.#options.localization.format("cfg-record-status", { register: this.recording.recording?.register ?? "", count: this.recording.recording?.steps.length ?? 0 });
  }
  open(view: View): void {
    if (this.#savingBindings) return;
    if ((view !== "input-config" && (!this.#options.state.status || this.#options.state.commandBlocked)) || this.#options.state.busy || this.#options.state.targeting || this.#options.state.terrainInteractionMode) return;
    const saved = this.#options.preferences.snapshot;
    if (view === "input-config" && !saved) { this.#options.message("preferences-unavailable"); return; }
    this.#bindings = saved ? structuredClone(saved.preferences.keyBindings) : undefined;
    this.#bindingRevision = saved?.revision;
    this.#bindingError = false;
    this.finishRecording(); this.#view = view; this.#snapshot = this.#options.state.status;
    this.#render(); if (!this.#dialog.open) this.#dialog.showModal(); this.#dialog.focus();
  }
  close(): void {
    if (this.#savingBindings) return;
    if (this.#bindingError && !this.#options.document.defaultView!.confirm(this.#options.localization.format("preferences-discard"))) return;
    this.#dialog.close();
  }
  #guard(action: () => void): void { try { action(); } catch (error) { this.#options.error(error); } }
  #text(tag: string, key: string): HTMLElement {
    const node = this.#options.document.createElement(tag); node.textContent = this.#options.localization.format(key); return node;
  }
  #button(key: string, action: () => void): HTMLButtonElement {
    const button = this.#options.document.createElement("button"); button.type = "button";
    button.textContent = this.#options.localization.format(key); button.addEventListener("click", () => this.#guard(action)); return button;
  }
  #label(key: string, input: HTMLElement): HTMLElement { const label = this.#text("label", key); label.append(input); return label; }
  #render(): void {
    this.#title.textContent = this.#options.localization.format(`cfg-${this.#view}`);
    this.#body.replaceChildren(this.#button("action-dialog-close", () => this.close()));
    if (this.#view === "input-config") { this.#keymaps(); if (this.#options.state.status && this.#options.state.mode === "playing") this.#macros(); }
    if (this.#view === "command-menu") this.#commands();
    if (this.#view === "notes") this.#guard(() => this.#notes());
    if (this.#view === "screen-export") this.#guard(() => this.#screen());
    if (this.#view === "play-register" || this.#view === "record-register") this.#registers();
    if (this.#view === "input-config" && this.#bindingError) {
      const message = this.#text("p", "preferences-key-save-error"); message.setAttribute("role", "alert");
      this.#body.prepend(message, this.#button("preferences-save", () => this.#saveBindings(this.#bindings!)));
    }
    for (const control of this.#body.querySelectorAll<HTMLButtonElement | HTMLInputElement | HTMLSelectElement>("button, input, select")) control.disabled = this.#savingBindings;
    this.#dialog.setAttribute("aria-busy", String(this.#savingBindings));
    if (this.#dialog.open) this.#dialog.focus();
  }
  #commandLabel(key: string, command: string): string {
    return `${key} · ${this.#options.localization.format(`cfg-command-${command}`)}`;
  }
  #commands(): void {
    this.#body.append(this.#text("p", "cfg-command-menu-help"));
    const search = this.#options.document.createElement("input"); search.type = "search";
    this.#body.append(this.#label("guide-search", search));
    const list = this.#options.document.createElement("div"); list.className = "config-command-list";
    for (const [key, command] of commandMenuEntries) {
      const button = this.#button(`cfg-command-${command}`, () => { this.close(); this.#options.execute(originalKey(key)); });
      button.dataset.commandKey = key; button.textContent = this.#commandLabel(key, command); list.append(button);
    }
    search.addEventListener("input", () => {
      for (const button of list.children) (button as HTMLElement).hidden = !button.textContent!.toLocaleLowerCase().includes(search.value.toLocaleLowerCase());
    }); this.#body.append(list);
  }
  #keymaps(): void {
    const d = this.#options.document, l = this.#options.localization;
    this.#body.append(this.#text("h3", "cfg-keys"), this.#text("p", "cfg-keys-help"), this.#text("p", `input-preset-${this.#options.preset()}`));
    const capture = this.#button("cfg-capture-key", () => { capture.dataset.capturing = "true"; capture.textContent = l.format("cfg-press-key"); });
    capture.id = "cfg-capture-key";
    let trigger: string | undefined;
    capture.addEventListener("keydown", event => {
      if (capture.dataset.capturing !== "true" || event.key === "Escape") return;
      event.preventDefault(); event.stopPropagation(); if (event.repeat || event.isComposing) return;
      const key = keyToken(event); if (!key) return;
      trigger = key; capture.dataset.capturing = "false"; capture.textContent = key;
    });
    const action = d.createElement("select"); action.id = "cfg-key-action";
    for (const [key, command] of commandMenuEntries) { const option = d.createElement("option"); option.value = key; option.textContent = this.#commandLabel(key, command); action.append(option); }
    for (const key of "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.") {
      const option = d.createElement("option"); option.value = `Register:${key}`; option.textContent = l.format("cfg-register-action", { register: key }); action.append(option);
    }
    const save = this.#button("cfg-key-save", () => {
      if (!trigger || !this.#bindings) return;
      const next = this.#bindings.filter(binding => binding.preset !== this.#options.preset() || binding.trigger !== trigger);
      next.push({ preset: this.#options.preset(), trigger, action: action.value }); this.#saveBindings(next);
    }); save.id = "cfg-key-save";
    this.#body.append(capture, this.#label("cfg-key-action", action), save);
    if (!this.#bindings) this.#body.append(this.#text("p", "cfg-keys-invalid"));
    const list = d.createElement("ul");
    for (const binding of this.#bindings ?? []) if (binding.preset === this.#options.preset()) {
      const row = d.createElement("li"); row.textContent = `${binding.trigger} → ${binding.action} `;
      row.append(this.#button("cfg-remove", () => this.#saveBindings(this.#bindings!.filter(candidate => candidate !== binding)))); list.append(row);
    }
    this.#body.append(list, this.#button("cfg-key-reset", () => this.#saveBindings((this.#bindings ?? []).filter(binding => binding.preset !== this.#options.preset()))),
      this.#button("cfg-key-export", () => { if (this.#bindings) this.#downloadText(JSON.stringify(this.#bindings, null, 2), "rfb-keybindings.json"); }));
    const file = d.createElement("input"); file.type = "file"; file.accept = ".json,application/json"; file.id = "cfg-key-import";
    file.addEventListener("change", () => { const selected = file.files?.[0]; if (!selected) return;
      if (selected.size > 65536) { this.#options.error(new Error(l.format("cfg-import-too-large"))); return; }
      const snapshot = this.#snapshot;
      void selected.text().then(text => {
        if (!this.#dialog.open || this.#view !== "input-config" || snapshot !== this.#snapshot) return;
        this.#saveBindings(parseBindings(text));
      }).catch(this.#options.error);
    }); this.#body.append(this.#label("cfg-key-import", file));
  }
  #saveBindings(bindings: KeyBinding[]): void {
    if (this.#savingBindings || this.#bindingRevision === undefined) return;
    const validated = parseBindings(JSON.stringify(bindings));
    this.#bindings = validated;
    this.#bindingError = false;
    this.#savingBindings = true;
    this.#render();
    void this.#options.saveBindings(validated, this.#bindingRevision).then(() => {
      this.#bindings = structuredClone(this.#options.preferences.snapshot!.preferences.keyBindings);
      this.#bindingRevision = this.#options.preferences.snapshot!.revision;
    }).catch(error => {
      this.#bindingError = true;
      this.#options.error(error);
    }).finally(() => {
      this.#savingBindings = false;
      if (this.#dialog.open && this.#view === "input-config") this.#render();
    });
  }
  #macros(): void {
    const d = this.#options.document;
    this.#body.append(this.#text("h3", "cfg-macros"), this.#text("p", "cfg-macro-help"));
    const register = d.createElement("select"); register.id = "cfg-register";
    for (const key of "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789") { const option = d.createElement("option"); option.value = key; option.textContent = key; register.append(option); }
    register.value = this.#register; register.addEventListener("change", () => { this.#register = register.value; this.#render(); });
    this.#body.append(this.#label("cfg-register", register), this.#button("cfg-record-start", () => {
      this.recording.start(this.#register, false); this.close(); this.#recordStatus();
    }), this.#button("cfg-play", () => this.#chooseRegister(this.#register)), this.#button("cfg-remove-register", () => { this.recording.registers.delete(this.#register); this.#render(); }));
    const steps = this.recording.registers.get(this.#register) ?? [];
    const list = d.createElement("ol");
    steps.forEach((step, index) => {
      const row = d.createElement("li"); row.textContent = step.description;
      row.append(this.#button("cfg-remove", () => { steps.splice(index, 1); this.#render(); }));
      if (index) row.append(this.#button("cfg-up", () => { [steps[index - 1], steps[index]] = [steps[index]!, steps[index - 1]!]; this.#render(); }));
      if (steps.length < MAX_MACRO_STEPS && isMacroCommand(step.command)) row.append(this.#button("cfg-duplicate", () => { steps.splice(index, 0, structuredClone(step)); this.#render(); }));
      list.append(row);
    });
    this.#body.append(list, this.#button("cfg-append-last", () => {
      const command = this.#options.lastCommand();
      if (!command || !isMacroCommand(command) || steps.some(step => !isMacroCommand(step.command)) || steps.length >= MAX_MACRO_STEPS) { this.#options.message("cfg-record-unsupported"); return; }
      steps.push({ command, description: this.#options.describe() }); this.recording.registers.set(this.#register, steps); this.#render();
    }));
  }
  #registers(): void {
    this.#body.append(this.#text("p", "cfg-register-help"));
    const last = this.#options.lastCommand();
    if (this.#view === "play-register" && last) this.#body.append(this.#button("cfg-register-last", () => this.#chooseRegister(".")));
    for (const key of "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789") {
      if (this.#view === "play-register" && !this.recording.registers.has(key)) continue;
      const button = this.#button("cfg-register", () => this.#chooseRegister(key));
      button.textContent = `${key} · ${this.recording.registers.get(key)?.map(step => step.description).join(" → ") ?? ""}`;
      this.#body.append(button);
    }
  }
  #chooseRegister(register: string, record = this.#view === "record-register"): void {
    if (record) {
      if (register === ".") return;
      this.recording.start(register, true); this.close(); this.#recordStatus(); return;
    }
    const last = register === "." ? this.#options.lastCommand() : undefined;
    const commands = last ? [last] : this.recording.registers.get(register)?.map(step => structuredClone(step.command));
    if (!commands?.length) { this.#options.message("cfg-register-empty"); return; }
    this.close(); this.recording.playing = true;
    const generation = this.#generation;
    void (commands.length === 1 ? this.#options.repeat(commands[0]!) : this.#options.play(commands))
      .catch(this.#options.error).finally(() => { if (generation === this.#generation) this.recording.playing = false; });
  }
  #readNotes(): Note[] {
    const notes: unknown = JSON.parse(this.#options.storage.getItem(NOTES_KEY) ?? "[]");
    if (!Array.isArray(notes) || notes.some(note => !note || typeof note.id !== "string" || typeof note.player !== "string" ||
        typeof note.floor !== "string" || !Number.isSafeInteger(note.turn) || typeof note.text !== "string" || typeof note.map !== "string")) throw new Error(this.#options.localization.format("cfg-notes-invalid"));
    return notes;
  }
  #notes(): void {
    const d = this.#options.document;
    const notes = this.#readNotes();
    this.#body.append(this.#text("p", "cfg-notes-help"));
    const input = d.createElement("textarea"); input.maxLength = 1000; input.rows = 4; input.id = "cfg-note-text";
    this.#body.append(this.#label("cfg-note-text", input), this.#button("cfg-note-add", () => {
      const text = input.value.trim(); if (!text || text.length > 1000 || this.#snapshot !== this.#options.state.status) return;
      const status = this.#options.state.status!;
      const next = [...this.#readNotes(), { id: crypto.randomUUID(), player: status.player.name, floor: this.#options.location(), turn: status.turn, text, map: mapText(this.#options.state, 50, 24) }];
      this.#options.storage.setItem(NOTES_KEY, JSON.stringify(next)); this.#options.message("cfg-note-message", { text }); this.#render();
    }), this.#button("cfg-notes-export", () => {
      this.#downloadText(this.#readNotes().map(note => `${note.player} · ${note.floor} · ${note.turn}\n${note.text}\n${note.map}`).join("\n\n"), "rfb-notes.txt");
    }));
    for (const note of [...notes].reverse()) {
      const detail = d.createElement("details"), summary = d.createElement("summary"), pre = d.createElement("pre");
      summary.textContent = `${note.player} · ${note.floor} · ${note.turn} · ${note.text}`; pre.textContent = note.map;
      detail.append(summary, pre, this.#button("cfg-remove", () => {
        this.#options.storage.setItem(NOTES_KEY, JSON.stringify(this.#readNotes().filter(candidate => candidate.id !== note.id))); this.#render();
      })); this.#body.append(detail);
    }
  }
  #screen(): void {
    const d = this.#options.document, status = this.#options.state.status!;
    const png = this.#options.capturePng(), text = `${status.player.name} · ${this.#options.location()} · ${status.turn}\n${mapText(this.#options.state)}`;
    const preview = d.createElement("img"); preview.src = png; preview.alt = this.#options.localization.format("cfg-screen-preview");
    this.#body.append(this.#text("p", "cfg-screen-help"), preview, this.#button("cfg-screen-png", () => {
      this.#options.download(Uint8Array.from(atob(png.slice(png.indexOf(",") + 1)), character => character.charCodeAt(0)), "rfb-map.png");
    }), this.#button("cfg-screen-text", () => this.#downloadText(text, "rfb-map.txt")), this.#button("cfg-screen-html", () => {
      const html = d.implementation.createHTMLDocument(this.#options.localization.format("cfg-screen-preview"));
      html.documentElement.lang = this.#options.localization.locale;
      const meta = html.createElement("meta"); meta.setAttribute("charset", "utf-8"); html.head.prepend(meta);
      const image = html.createElement("img"); image.src = png; image.alt = preview.alt; image.style.maxWidth = "100%";
      const pre = html.createElement("pre"); pre.textContent = text; html.body.append(image, pre);
      this.#downloadText("<!doctype html>\n" + html.documentElement.outerHTML, "rfb-map.html");
    }));
  }
  #downloadText(text: string, filename: string): void { this.#options.download(new TextEncoder().encode(text), filename); }
}
