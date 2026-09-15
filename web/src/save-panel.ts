// SPDX-License-Identifier: MPL-2.0
import type { MessageKey } from "./localization";
import { desktopErrorCode, nativeSaveErrorCategory, type NativeSaveStorage } from "./native-save-storage.ts";

type NativeSaveOptions = {
    storage: Pick<NativeSaveStorage, "save">;
    isGameBusy: () => boolean;
    beforeSessionAccess: () => Promise<void>;
    setGameBusy: (busy: boolean) => void;
    announce: (key: MessageKey, args: Record<string, string | number> | undefined, kind: string) => void;
    onSaved?: (summary: Awaited<ReturnType<NativeSaveStorage["save"]>>) => void;
    logError?: (error: unknown) => void;
};

export class NativeSaveCommands {
  readonly #options: NativeSaveOptions;
  #busy = false;
  constructor(options: NativeSaveOptions) { this.#options = options; }

  async saveFromShortcut(requestName?: () => string | null, afterSave?: () => Promise<void>, newSlot = false): Promise<boolean> {
    if (this.#busy) return false;
    this.#busy = true;
    let gameLocked = false, saved = false;
    const o = this.#options;
    try {
      await o.beforeSessionAccess();
      if (o.isGameBusy()) return false;
      const slotName = requestName?.()?.trim();
      if (requestName && !slotName) return false;
      o.setGameBusy(true); gameLocked = true;
      const summary = await o.storage.save(slotName, undefined, newSlot);
      o.announce("message-native-save-created", { name: summary.slotName }, "system");
      o.onSaved?.(summary); saved = true;
    } catch (error) {
      o.announce(nativeSaveErrorKey(desktopErrorCode(error)), undefined, "error");
      (o.logError ?? console.error)(error);
    } finally {
      if (gameLocked) o.setGameBusy(false);
      this.#busy = false;
    }
    if (saved) await afterSave?.();
    return saved;
  }
}

export function nativeSaveErrorKey(code: string): MessageKey {
  switch (code) {
    case "museum-collection-stale": return "museum-error-stale";
    case "museum-character-stale": return "museum-error-character-stale";
    case "museum-busy": return "museum-error-busy";
    case "museum-profile-mismatch":
    case "museum-character-missing": return "museum-error-profile";
    case "museum-unbound-save": return "museum-error-unbound";
  }
  if (code.startsWith("museum-")) return "museum-error-invalid";
  switch (nativeSaveErrorCategory(code)) {
    case "name-invalid":
      return "native-save-error-name-invalid";
    case "not-found":
      return "native-save-error-not-found";
    case "corrupt":
      return "native-save-error-corrupt";
    case "read":
      return "native-save-error-read";
    case "write":
      return "native-save-error-write";
    case "unavailable":
      return "native-save-error-unavailable";
    case "internal":
      return "native-save-error-internal";
  }
}
