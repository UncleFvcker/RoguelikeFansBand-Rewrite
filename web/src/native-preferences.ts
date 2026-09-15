// SPDX-License-Identifier: MPL-2.0
import { invoke } from "@tauri-apps/api/core";
import type { Preferences, PreferenceSnapshot, PreferenceStorage } from "./preferences";

export const nativePreferences: PreferenceStorage = {
  defaults: () => invoke<Preferences>("default_preferences"),
  load: () => invoke<PreferenceSnapshot | null>("load_preferences"),
  save: (preferences: Preferences, expectedRevision: number | null) =>
    invoke<PreferenceSnapshot>("save_preferences", { preferences, expectedRevision }),
};
