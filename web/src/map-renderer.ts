// SPDX-License-Identifier: MPL-2.0
import { defaultVisuals, type VisualPreferences } from "./visual-preferences.ts";
import type { EditableVisualDto } from "./protocol";

import {
  computeCameraOffset,
  computeFullMapScroll,
  MAP_CELL_SIZE,
  type CameraMode,
  type ZoomLevel,
} from "./camera";
import { PixiRendererBackend } from "./pixi-renderer-backend";
import type { GameCommand, GameSnapshot, GameUpdate, Position } from "./protocol";
import { canAnimatePlayerStep, playerMeleeAttack, meleeEffectTarget, type PlayerFrame } from "./player-motion.ts";
import { RenderWorld } from "./render-world";
import { projectileFlights } from "./projectile-motion.ts";
import type {
  RendererBackend,
  TilesetChangeResult,
} from "./renderer-backend";

export { type TilesetChangeResult } from "./renderer-backend";
export { type CameraMode } from "./camera";
export { type ZoomLevel } from "./camera";

export class MapRenderer {
  #visuals = defaultVisuals();
  #visualCatalog: readonly EditableVisualDto[] = [];
  visualBase(id: string) { return this.#backend.visualBase(id); }
  setVisualPreferences(preferences: VisualPreferences): void {
    this.#visuals = preferences;
    if (this.#world && this.#backend.setVisuals(preferences, this.#visualCatalog)) this.#backend.applyCells(this.#world.allCells());
  }
  capturePng(): string { return this.#backend.capturePng(); }
  readonly #backend: RendererBackend;
  #world: RenderWorld | undefined;
  #host: HTMLElement | undefined;
  #cameraMode: CameraMode = "player-centered";
  #zoom: ZoomLevel = 1;
  #resizeObserver: ResizeObserver | undefined;
  #width = 0;
  #height = 0;
  #cameraFocus: Position | undefined;
  #displayPlayerPosition: Position | undefined;
  #cameraImpulse: Position = { x: 0, y: 0 };
  #lastPlayerFrame: PlayerFrame | undefined;
  #totalAppliedCells = 0;

  constructor(backend: RendererBackend = new PixiRendererBackend()) {
    this.#backend = backend;
  }

  async initialize(
    host: HTMLElement,
    width: number,
    height: number,
    tilesetManifestUrl: string,
    contentGlyphs: Readonly<Record<string, string>>,
    canvasLabel: string,
    cameraMode: CameraMode = "player-centered",
    zoom: ZoomLevel = 1,
  ): Promise<TilesetChangeResult> {
    this.#host = host;
    this.#totalAppliedCells = 0;
    this.#width = width;
    this.#height = height;
    this.#cameraMode = cameraMode;
    this.#zoom = zoom;
    this.#world = new RenderWorld(width, height);
    const result = await this.#backend.initialize({
      host,
      width,
      height,
      tilesetManifestUrl,
      contentGlyphs,
      canvasLabel,
      zoom,
      onPlayerPosition: position => {
        this.#displayPlayerPosition = { ...position };
        if (this.#host) {
          this.#host.dataset.playerDisplayX = String(position.x);
          this.#host.dataset.playerDisplayY = String(position.y);
        }
        this.#updateCamera();
      },
      onCameraImpulse: offset => { this.#cameraImpulse = offset; this.#updateCamera(); },
    });
    host.dataset.rendererBackend = this.#backend.id;
    host.dataset.rendererLayerCount = "8";
    host.dataset.rendererLayers = "terrain,object,actor,melee,projectile,visibility,player,lighting";
    host.dataset.terrainMode = "chunk-render-texture-v1";
    host.dataset.dynamicViewMode = "visible-chunk-reuse-v1";
    host.dataset.visibilityMode = "rust-fov-memory-v1";
    host.dataset.lightingMode = "rust-content-lights-v1";
    this.#configureViewport();
    if (typeof ResizeObserver !== "undefined") {
      this.#resizeObserver = new ResizeObserver(() => this.#updateCamera());
      this.#resizeObserver.observe(host);
    }
    this.#recordRender("tileset", 0, result.id);
    this.#updateCamera();
    return result;
  }

  setCanvasLabel(label: string): void {
    this.#backend.setCanvasLabel(label);
  }

  setCameraMode(mode: CameraMode): void {
    this.#cameraMode = mode;
    this.#configureViewport();
    this.#updateCamera();
  }

  setZoom(zoom: ZoomLevel): void {
    this.#zoom = zoom;
    this.#configureViewport();
    this.#updateCamera();
  }

  setCameraFocus(position: Position | undefined): void {
    this.#cameraFocus = position ? { ...position } : undefined;
    this.#updateCamera();
  }

  refreshLayout(): void {
    this.#updateCamera();
  }

  recenter(): void {
    this.#cameraFocus = undefined;
    this.#updateCamera();
    if (this.#host && this.#world && this.#cameraMode === "full-map") {
      const size = MAP_CELL_SIZE * this.#zoom;
      this.#host.scrollTo({ left: (this.#world.playerPosition.x + 0.5) * size - this.#host.clientWidth / 2,
        top: (this.#world.playerPosition.y + 0.5) * size - this.#host.clientHeight / 2, behavior: "auto" });
      this.#host.dataset.scrollX = String(this.#host.scrollLeft);
      this.#host.dataset.scrollY = String(this.#host.scrollTop);
    }
  }

  async setTileset(tilesetManifestUrl: string): Promise<TilesetChangeResult> {
    const result = await this.#backend.setTileset(tilesetManifestUrl);
    const appliedCells = this.#backend.applyCells(this.#requireWorld().allCells());
    this.#recordRender("tileset", appliedCells, result.id);
    this.#recordBackendDiagnostics();
    return result;
  }

  applySnapshot(snapshot: GameSnapshot): void {
    this.#cameraFocus = undefined;
    this.#resizeWorld(snapshot.width, snapshot.height);
    this.#visualCatalog = snapshot.player.visualCatalog;
    this.#backend.setVisuals(this.#visuals, this.#visualCatalog);
    const cells = this.#requireWorld().applySnapshot(snapshot);
    this.#backend.setPlayerPosition(snapshot.player.position, false);
    const appliedCells = this.#backend.applyCells(cells);
    this.#recordRender("snapshot", appliedCells);
    this.#recordVisualState();
    this.#lastPlayerFrame = snapshot;
  }

  get playerMoving(): boolean { return this.#backend.playerMoving; }
  setMeleeCameraShake(enabled: boolean): void { this.#backend.setMeleeCameraShake(enabled); }

  whenPlayerSettled(): Promise<void> { return this.#backend.whenPlayerSettled(); }

  applyUpdate(update: GameUpdate, command?: GameCommand, continuous = false): boolean {
    const animate = canAnimatePlayerStep(this.#lastPlayerFrame, update, command);
    const before = this.#lastPlayerFrame;
    const attack = playerMeleeAttack(before, update, command);
    const oldTarget = attack?.targetPosition ? this.#requireWorld().cellAt(attack.targetPosition) : undefined;
    const flights = before?.floorId === update.floorId && before.mapScale === "local" && update.mapScale === "local" &&
      before.player.id === update.player.id && update.player.hp > 0 && !update.mapTranslation?.x && !update.mapTranslation?.y
      && before.player.position.x === update.player.position.x && before.player.position.y === update.player.position.y
      ? projectileFlights(update.events, new Map(before.entities.map(entity => [entity.id, entity.position]))) : [];
    for (const flight of flights) for (const hit of flight.hits) hit.previous = this.#requireWorld().cellAt(hit.position);
    const preserveMovement = (command?.type === "cancel-run" || command?.type === "cancel-auto-explore") &&
      before !== undefined && before.floorId === update.floorId && before.mapScale === update.mapScale &&
      before.player.id === update.player.id && update.player.hp > 0 &&
      before.player.position.x === update.player.position.x && before.player.position.y === update.player.position.y &&
      !update.mapTranslation?.x && !update.mapTranslation?.y;
    if (this.#lastPlayerFrame && (this.#lastPlayerFrame.floorId !== update.floorId ||
        this.#lastPlayerFrame.mapScale !== update.mapScale)) {
      this.#cameraFocus = undefined;
    }
    const resized = this.#resizeWorld(update.width, update.height);
    this.#visualCatalog = update.player.visualCatalog;
    const visualsChanged = this.#backend.setVisuals(this.#visuals, this.#visualCatalog);
    const updated = this.#requireWorld().applyUpdate(update);
    const cells = visualsChanged ? this.#requireWorld().allCells() : updated;
    // Start the shared display step before replacing cell light/FOV projections.
    if (!preserveMovement || resized) this.#backend.setPlayerPosition(update.player.position, animate && !resized, continuous);
    const appliedCells = this.#backend.applyCells(cells);
    if (attack && !resized) {
      const position = update.entities.find(entity => entity.id === attack.targetId)?.position ?? attack.targetPosition;
      const currentTarget = position ? this.#requireWorld().cellAt(position) : undefined;
      this.#backend.playPlayerMelee(attack, meleeEffectTarget(attack, oldTarget, currentTarget));
    }
    if (flights.length && !resized) {
      for (const flight of flights) for (const hit of flight.hits) {
        const position = update.entities.find(entity => entity.id === hit.targetId)?.position ?? hit.position;
        hit.target = meleeEffectTarget(hit, hit.previous, this.#requireWorld().cellAt(position));
      }
      this.#backend.playProjectiles(flights);
    }
    this.#recordRender("update", appliedCells);
    this.#recordVisualState();
    this.#lastPlayerFrame = update;
    return resized;
  }

  destroy(): void {
    this.#resizeObserver?.disconnect();
    this.#resizeObserver = undefined;
    this.#backend.destroy();
    if (this.#host) {
      delete this.#host.dataset.playerDisplayX;
      delete this.#host.dataset.playerDisplayY;
    }
    this.#world = undefined;
    this.#host = undefined;
    this.#width = 0;
    this.#height = 0;
    this.#cameraFocus = undefined;
    this.#displayPlayerPosition = undefined;
    this.#lastPlayerFrame = undefined;
  }

  #configureViewport(): void {
    const host = this.#host;
    if (!host) return;
    const worldWidth = this.#width * MAP_CELL_SIZE * this.#zoom;
    const worldHeight = this.#height * MAP_CELL_SIZE * this.#zoom;
    host.dataset.cameraMode = this.#cameraMode;
    host.style.setProperty("--map-world-width", `${worldWidth}px`);
    host.style.setProperty("--map-world-height", `${worldHeight}px`);
  }

  #resizeWorld(width: number, height: number): boolean {
    if (width === this.#width && height === this.#height) return false;
    this.#width = width;
    this.#height = height;
    this.#world = new RenderWorld(width, height);
    this.#backend.resize(width, height);
    this.#configureViewport();
    return true;
  }

  #updateCamera(): void {
    const host = this.#host;
    const world = this.#world;
    if (!host || !world) return;
    const focus = this.#cameraFocus ?? this.#displayPlayerPosition ?? world.playerPosition;
    const viewportWidth = host.clientWidth || this.#width * MAP_CELL_SIZE;
    const viewportHeight = host.clientHeight || this.#height * MAP_CELL_SIZE;
    const offset = computeCameraOffset({
      mode: this.#cameraMode,
      focus,
      worldWidth: this.#width * MAP_CELL_SIZE,
      worldHeight: this.#height * MAP_CELL_SIZE,
      viewportWidth,
      viewportHeight,
      zoom: this.#zoom,
    });
    offset.x += this.#cameraImpulse.x;
    offset.y += this.#cameraImpulse.y;
    this.#backend.setCameraTransform({
      x: offset.x,
      y: offset.y,
      zoom: this.#zoom,
      viewportWidth,
      viewportHeight,
      cullingEnabled: this.#cameraMode === "player-centered",
    });
    host.dataset.cameraX = String(offset.x);
    host.dataset.cameraY = String(offset.y);
    host.dataset.zoom = String(this.#zoom);
    host.dataset.viewportWidth = String(viewportWidth);
    host.dataset.viewportHeight = String(viewportHeight);
    if (this.#cameraMode === "full-map") {
      const scroll = computeFullMapScroll({
        focus,
        worldWidth: this.#width * MAP_CELL_SIZE,
        worldHeight: this.#height * MAP_CELL_SIZE,
        viewportWidth,
        viewportHeight,
        scrollX: host.scrollLeft,
        scrollY: host.scrollTop,
        zoom: this.#zoom,
      });
      host.scrollTo({ left: scroll.x, top: scroll.y, behavior: "auto" });
    } else {
      host.scrollTo({ left: 0, top: 0, behavior: "auto" });
    }
    host.dataset.scrollX = String(host.scrollLeft);
    host.dataset.scrollY = String(host.scrollTop);
    host.dispatchEvent(new Event("map-camera-change"));
    this.#recordBackendDiagnostics();
  }

  #requireWorld(): RenderWorld {
    if (!this.#world) throw new Error("render world is not initialized");
    return this.#world;
  }

  #recordRender(
    kind: "snapshot" | "update" | "tileset",
    appliedCells: number,
    tilesetId?: string,
  ): void {
    const host = this.#host;
    if (!host) return;
    this.#totalAppliedCells += appliedCells;
    host.dataset.renderKind = kind;
    host.dataset.lastAppliedCells = String(appliedCells);
    host.dataset.totalAppliedCells = String(this.#totalAppliedCells);
    if (tilesetId) host.dataset.tilesetId = tilesetId;
  }

  #recordVisualState(): void {
    const host = this.#host;
    const world = this.#world;
    if (!host || !world) return;
    const counts = world.visibilityCounts;
    host.dataset.visibleCellCount = String(counts.visible);
    host.dataset.rememberedCellCount = String(counts.remembered);
    host.dataset.hiddenCellCount = String(counts.hidden);
  }

  #recordBackendDiagnostics(): void {
    const host = this.#host;
    if (!host) return;
    const diagnostics = this.#backend.getDiagnostics();
    host.dataset.terrainChunkSize = String(diagnostics.terrainChunkSize);
    host.dataset.terrainChunkCount = String(diagnostics.terrainChunkCount);
    host.dataset.visibleChunkCount = String(diagnostics.visibleChunkCount);
    host.dataset.culledChunkCount = String(
      diagnostics.terrainChunkCount - diagnostics.visibleChunkCount,
    );
    host.dataset.lastRebuiltTerrainChunks = String(
      diagnostics.lastRebuiltTerrainChunks,
    );
    host.dataset.totalRebuiltTerrainChunks = String(
      diagnostics.totalRebuiltTerrainChunks,
    );
    host.dataset.activeDynamicChunkCount = String(
      diagnostics.activeDynamicChunkCount,
    );
    host.dataset.pooledDynamicChunkCount = String(
      diagnostics.pooledDynamicChunkCount,
    );
    host.dataset.rendererCellViewCount = String(diagnostics.cellViewCount);
    host.dataset.rendererDynamicDisplayObjectCount = String(
      diagnostics.dynamicDisplayObjectCount,
    );
  }
}
