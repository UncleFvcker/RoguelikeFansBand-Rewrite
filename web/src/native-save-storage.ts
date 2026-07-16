// SPDX-License-Identifier: MPL-2.0

import { invoke } from "@tauri-apps/api/core";

import type { GameSnapshot } from "./protocol";

export type NativeSaveStatus = "ready" | "recoverable" | "corrupt";

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
