// SPDX-License-Identifier: MPL-2.0

import { invoke } from "@tauri-apps/api/core";

import type { GameSnapshot } from "./protocol";

export type NativeSaveStatus = "ready" | "recoverable" | "corrupt";
export type NativeSaveErrorCategory =
  | "name-invalid"
  | "not-found"
  | "corrupt"
  | "read"
  | "write"
  | "unavailable"
  | "internal";

export interface NativeSaveSummary {
  slotId: string;
  slotName: string;
  status: NativeSaveStatus;
  recoveryBackup: number | null;
  savedAt: string | null;
  createdAt: string | null;
  turn: number | null;
  locationKey: string | null;
  contentId: string | null;
  contentHash: string | null;
  stateHash: string | null;
}

export interface NativeLoadResult {
  snapshot: GameSnapshot;
  recoveryBackup: number | null;
}

export interface DesktopCommandError {
  code: string;
  detail: string;
}

export class NativeSaveStorage {
  list(): Promise<NativeSaveSummary[]> {
    return invoke<NativeSaveSummary[]>("list_native_saves");
  }

  save(slotName: string, slotId?: string): Promise<NativeSaveSummary> {
    return invoke<NativeSaveSummary>("save_native_game", {
      slotId: slotId ?? null,
      slotName,
      savedAt: new Date().toISOString(),
    });
  }

  load(slotId: string): Promise<NativeLoadResult> {
    return invoke<NativeLoadResult>("load_native_game", { slotId });
  }

  delete(slotId: string): Promise<void> {
    return invoke<void>("delete_native_save", { slotId });
  }
}

export function desktopErrorCode(error: unknown): string {
  if (typeof error === "object" && error !== null && "code" in error) {
    const code = (error as Partial<DesktopCommandError>).code;
    if (typeof code === "string") return code;
  }
  return "desktop-storage-unknown";
}

export function nativeSaveErrorCategory(code: string): NativeSaveErrorCategory {
  switch (code) {
    case "native-save-name-invalid":
      return "name-invalid";
    case "native-save-not-found":
      return "not-found";
    case "native-save-invalid":
      return "corrupt";
    case "native-save-list":
    case "native-save-read":
    case "native-save-verify-read":
    case "native-save-commit-read":
    case "native-save-temp-list":
      return "read";
    case "native-save-temp-create":
    case "native-save-write":
    case "native-save-sync":
    case "native-save-commit":
    case "native-save-delete":
    case "native-save-backup-remove":
    case "native-save-backup-rotate":
    case "native-save-backup-create":
    case "native-save-temp-clean":
      return "write";
    case "native-save-directory":
    case "native-save-lock":
      return "unavailable";
    default:
      return "internal";
  }
}
