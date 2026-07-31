// SPDX-License-Identifier: MPL-2.0

import { parseZoomLevel, type ZoomLevel } from "./camera.ts";
import type { AppDom } from "./app-dom";
import type { AppState } from "./app-state";
import type { InputPreset } from "./input-controller";
import type { Localization, MessageKey, SupportedLocale } from "./localization";
import { isSupportedLocale } from "./localization.ts";
import type { CameraMode } from "./map-renderer";
import type { TilesetWarning } from "./tileset-runtime";

export type TilesetPreset = "ascii" | "image";

const INPUT_PRESET_STORAGE_KEY = "rfb.input-preset";
const TILESET_PRESET_STORAGE_KEY = "rfb.tileset-preset";
const CAMERA_MODE_STORAGE_KEY = "rfb.camera-mode";
const ZOOM_STORAGE_KEY = "rfb.zoom";
const LOCALE_STORAGE_KEY = "rfb.locale";
const TILESET_MANIFESTS: Record<TilesetPreset, string> = {
  ascii: "/tilesets/ascii-default/tileset.json",
  image: "/tilesets/image-demo/tileset.json",
};

type SettingsDom = Pick<
  AppDom,
  | "inputPresetSelect"
  | "tilesetPresetSelect"
  | "cameraModeSelect"
  | "zoomSelect"
  | "controlsHelp"
  | "languageSelect"
>;

interface SettingsRenderer {
  setTileset(manifestUrl: string): Promise<{ id: string; warnings: readonly TilesetWarning[] }>;
  setCameraMode(mode: CameraMode): void;
  setZoom(zoom: ZoomLevel): void;
  setCanvasLabel(label: string): void;
}

export class SettingsPanel {
  readonly #dom: SettingsDom;
  readonly #state: AppState;
  readonly #localization: Localization;
  readonly #renderer: SettingsRenderer;
  readonly #storage: Storage;
  readonly #renderTargeting: () => void;
  readonly #renderLocaleDependentUi: () => void;
  readonly #refreshBusyControls: () => void;
  readonly #announce: (
    key: MessageKey,
    args: Record<string, string | number> | undefined,
    kind: string,
  ) => void;
  readonly #logError: (error: unknown) => void;
  #inputPreset: InputPreset;
  #tilesetPreset: TilesetPreset;
  #cameraMode: CameraMode;
  #zoom: ZoomLevel;
  #installed = false;

  constructor(options: {
    dom: SettingsDom;
    state: AppState;
    localization: Localization;
    renderer: SettingsRenderer;
    storage: Storage;
    renderTargeting: () => void;
    renderLocaleDependentUi: () => void;
    refreshBusyControls: () => void;
    announce: (
      key: MessageKey,
      args: Record<string, string | number> | undefined,
      kind: string,
    ) => void;
    logError?: (error: unknown) => void;
  }) {
    this.#dom = options.dom;
    this.#state = options.state;
    this.#localization = options.localization;
    this.#renderer = options.renderer;
    this.#storage = options.storage;
    this.#renderTargeting = options.renderTargeting;
    this.#renderLocaleDependentUi = options.renderLocaleDependentUi;
    this.#refreshBusyControls = options.refreshBusyControls;
    this.#announce = options.announce;
    this.#logError = options.logError ?? console.error;
    this.#inputPreset = readInputPreset(this.#storage);
    this.#tilesetPreset = readTilesetPreset(this.#storage);
    this.#cameraMode = readCameraMode(this.#storage);
    this.#zoom = readZoomLevel(this.#storage);
  }

  get inputPreset(): InputPreset {
    return this.#inputPreset;
  }

  get cameraMode(): CameraMode {
    return this.#cameraMode;
  }

  get zoom(): ZoomLevel {
    return this.#zoom;
  }

  get tilesetManifest(): string {
    return TILESET_MANIFESTS[this.#tilesetPreset];
  }

  initialize(): void {
    this.#dom.inputPresetSelect.value = this.#inputPreset;
    this.#dom.tilesetPresetSelect.value = this.#tilesetPreset;
    this.#dom.cameraModeSelect.value = this.#cameraMode;
    this.#dom.zoomSelect.value = String(this.#zoom);
    this.#dom.languageSelect.value = this.#localization.locale;
    this.#localization.localizeDocument();
    this.#renderInputHelp();
  }

  install(): void {
    if (this.#installed) return;
    this.#installed = true;
    this.#dom.inputPresetSelect.addEventListener("change", this.#handleInputPresetChange);
    this.#dom.tilesetPresetSelect.addEventListener("change", this.#handleTilesetChange);
    this.#dom.cameraModeSelect.addEventListener("change", this.#handleCameraModeChange);
    this.#dom.zoomSelect.addEventListener("change", this.#handleZoomChange);
    this.#dom.languageSelect.addEventListener("change", this.#handleLanguageChange);
  }

  announceTileset(id: string, warnings: readonly TilesetWarning[]): void {
    this.#announce("message-tileset-loaded", { id }, "system");
    for (const warning of warnings) {
      this.#announce(tilesetWarningMessageKey(warning), undefined, "system");
    }
  }

  dispose(): void {
    if (!this.#installed) return;
    this.#installed = false;
    this.#dom.inputPresetSelect.removeEventListener("change", this.#handleInputPresetChange);
    this.#dom.tilesetPresetSelect.removeEventListener("change", this.#handleTilesetChange);
    this.#dom.cameraModeSelect.removeEventListener("change", this.#handleCameraModeChange);
    this.#dom.zoomSelect.removeEventListener("change", this.#handleZoomChange);
    this.#dom.languageSelect.removeEventListener("change", this.#handleLanguageChange);
  }

  readonly #handleInputPresetChange = (): void => {
    this.#inputPreset = isInputPreset(this.#dom.inputPresetSelect.value)
      ? this.#dom.inputPresetSelect.value
      : "numpad";
    this.#storage.setItem(INPUT_PRESET_STORAGE_KEY, this.#inputPreset);
    this.#renderInputHelp();
    this.#announce(
      "message-input-preset-changed",
      { preset: this.#inputPreset },
      "system",
    );
  };

  readonly #handleTilesetChange = (): void => {
    void this.#changeTileset();
  };

  readonly #handleCameraModeChange = (): void => {
    this.#cameraMode = isCameraMode(this.#dom.cameraModeSelect.value)
      ? this.#dom.cameraModeSelect.value
      : "full-map";
    this.#storage.setItem(CAMERA_MODE_STORAGE_KEY, this.#cameraMode);
    this.#renderer.setCameraMode(this.#cameraMode);
    this.#renderTargeting();
  };

  readonly #handleZoomChange = (): void => {
    this.#zoom = parseZoomLevel(this.#dom.zoomSelect.value);
    this.#dom.zoomSelect.value = String(this.#zoom);
    this.#storage.setItem(ZOOM_STORAGE_KEY, String(this.#zoom));
    this.#renderer.setZoom(this.#zoom);
    this.#renderTargeting();
  };

  readonly #handleLanguageChange = (): void => {
    const locale = isSupportedLocale(this.#dom.languageSelect.value)
      ? this.#dom.languageSelect.value
      : "zh-CN";
    this.#localization.setLocale(locale);
    this.#storage.setItem(LOCALE_STORAGE_KEY, locale);
    this.#localization.localizeDocument();
    this.#dom.languageSelect.value = locale;
    this.#renderer.setCanvasLabel(this.#localization.format("map-aria-label"));
    this.#renderInputHelp();
    this.#renderLocaleDependentUi();
  };

  async #changeTileset(): Promise<void> {
    const requested = isTilesetPreset(this.#dom.tilesetPresetSelect.value)
      ? this.#dom.tilesetPresetSelect.value
      : "ascii";
    if (requested === this.#tilesetPreset || this.#state.busy) {
      this.#dom.tilesetPresetSelect.value = this.#tilesetPreset;
      return;
    }
    this.#state.busy = true;
    this.#refreshBusyControls();
    try {
      const result = await this.#renderer.setTileset(TILESET_MANIFESTS[requested]);
      this.#tilesetPreset = requested;
      this.#storage.setItem(TILESET_PRESET_STORAGE_KEY, this.#tilesetPreset);
      this.announceTileset(result.id, result.warnings);
    } catch (error) {
      this.#dom.tilesetPresetSelect.value = this.#tilesetPreset;
      const message = error instanceof Error ? error.message : String(error);
      this.#announce("message-tileset-load-failed", { error: message }, "error");
      this.#logError(error);
    } finally {
      this.#state.busy = false;
      this.#refreshBusyControls();
    }
  }

  #renderInputHelp(): void {
    const keys: Record<InputPreset, MessageKey> = {
      numpad: "controls-numpad",
      vi: "controls-vi",
      wasd: "controls-wasd",
    };
    this.#dom.controlsHelp.textContent = this.#localization.format(keys[this.#inputPreset]);
  }
}

export function readLocale(storage: Pick<Storage, "getItem">): SupportedLocale {
  const stored = storage.getItem(LOCALE_STORAGE_KEY);
  return isSupportedLocale(stored) ? stored : "zh-CN";
}

export function isInputPreset(value: string | null): value is InputPreset {
  return value === "numpad" || value === "vi" || value === "wasd";
}

export function inputPresetMessageKey(preset: InputPreset): MessageKey {
  const keys: Record<InputPreset, MessageKey> = {
    numpad: "input-preset-numpad",
    vi: "input-preset-vi",
    wasd: "input-preset-wasd",
  };
  return keys[preset];
}

function readInputPreset(storage: Pick<Storage, "getItem">): InputPreset {
  const stored = storage.getItem(INPUT_PRESET_STORAGE_KEY);
  return isInputPreset(stored) ? stored : "numpad";
}

function readTilesetPreset(storage: Pick<Storage, "getItem">): TilesetPreset {
  const stored = storage.getItem(TILESET_PRESET_STORAGE_KEY);
  return isTilesetPreset(stored) ? stored : "ascii";
}

function readCameraMode(storage: Pick<Storage, "getItem">): CameraMode {
  const stored = storage.getItem(CAMERA_MODE_STORAGE_KEY);
  return isCameraMode(stored) ? stored : "full-map";
}

function readZoomLevel(storage: Pick<Storage, "getItem">): ZoomLevel {
  return parseZoomLevel(storage.getItem(ZOOM_STORAGE_KEY));
}

function isCameraMode(value: string | null): value is CameraMode {
  return value === "full-map" || value === "player-centered";
}

function isTilesetPreset(value: string | null): value is TilesetPreset {
  return value === "ascii" || value === "image";
}

function tilesetWarningMessageKey(warning: TilesetWarning): MessageKey {
  return warning === "image-too-small"
    ? "message-tileset-image-too-small"
    : "message-tileset-image-load-failed";
}
