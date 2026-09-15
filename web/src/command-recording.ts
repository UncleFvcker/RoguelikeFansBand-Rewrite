// SPDX-License-Identifier: MPL-2.0
import type { GameCommand } from "./protocol";
import { commandMenuEntries } from "./command-shortcuts.ts";

export type RecordedCommand = { command: GameCommand; description: string };
export const MAX_MACRO_STEPS = 256;

// Continuous operations have their own scheduler. Registers may repeat them, but
// a multi-command macro records only complete, single actions.
export function isMacroCommand(command: GameCommand): boolean {
  return !["end-character", "retire", "run", "auto-explore", "rest", "rest-for-turns", "rest-until-resources", "travel-local", "travel-world",
    "travel-unknown-item", "find-nearest-unknown-item", "enter-world-map", "leave-world-map", "traverse-stairs"].includes(command.type);
}

export class CommandRecording {
  readonly registers = new Map<string, RecordedCommand[]>();
  recording: { register: string; single: boolean; steps: RecordedCommand[] } | undefined;
  playing = false;

  start(register: string, single: boolean): void {
    if (!/^[a-zA-Z0-9]$/.test(register)) throw new Error("Invalid register");
    this.recording = { register, single, steps: [] };
  }
  observe(command: GameCommand | undefined, description: string): "unsupported" | "full" | undefined {
    if (this.playing || !this.recording) return;
    if (!command || (!this.recording.single && !isMacroCommand(command))) { this.finish(); return "unsupported"; }
    this.recording.steps.push({ command: structuredClone(command), description });
    if (this.recording.single) this.finish();
    else if (this.recording.steps.length === MAX_MACRO_STEPS) { this.finish(); return "full"; }
  }
  finish(): void {
    if (this.recording?.steps.length) this.registers.set(this.recording.register, this.recording.steps);
    this.recording = undefined;
  }
  reset(): void { this.recording = undefined; this.registers.clear(); this.playing = false; }
}

export type KeyBinding = { preset: string; trigger: string; action: string };
export function parseBindings(text: string): KeyBinding[] {
  const value: unknown = JSON.parse(text);
  if (!Array.isArray(value) || value.length > 256) throw new Error("Invalid key bindings");
  const seen = new Set<string>();
  for (const binding of value) {
    if (!binding || typeof binding !== "object" || Object.keys(binding).sort().join() !== "action,preset,trigger" ||
        !["original", "roguelike"].includes(binding.preset) ||
        typeof binding.trigger !== "string" || !binding.trigger || binding.trigger.length > 64 || /[\u0000-\u001f\u007f-\u009f]/.test(binding.trigger) ||
        typeof binding.action !== "string" || !(commandMenuEntries.some(([key]) => key === binding.action) || /^Register:[a-zA-Z0-9.]$/.test(binding.action)) ||
        /(?:^|\+)(Escape|\\|Meta|Control|Shift|Alt)$/.test(binding.trigger) || seen.has(binding.preset + ":" + binding.trigger)) {
      throw new Error("Invalid key binding");
    }
    seen.add(binding.preset + ":" + binding.trigger);
  }
  return value.map(({ preset, trigger, action }) => ({ preset, trigger, action }));
}
export function keyToken(event: Pick<KeyboardEvent, "key" | "ctrlKey" | "altKey" | "metaKey" | "shiftKey">): string | undefined {
  if (event.metaKey || ["Control", "Shift", "Alt", "Meta", "Escape", "\\", "Dead", "Unidentified"].includes(event.key)) return;
  const key = event.key === " " ? "Space" : event.ctrlKey || event.altKey ? event.key.toLowerCase() : event.key;
  return (event.ctrlKey ? "Ctrl+" : "") + (event.altKey ? "Alt+" : "") +
    (event.shiftKey && ([...event.key].length > 1 || event.ctrlKey || event.altKey) ? "Shift+" : "") + key;
}
export function originalKey(token: string): KeyboardEventInit {
  const ctrlKey = token.startsWith("Ctrl+");
  const key = ctrlKey ? token.slice(5) : token;
  return { key, ctrlKey, shiftKey: !ctrlKey && /^[A-Z~!@#$%^&*()_+?:<>]$/.test(key) };
}
