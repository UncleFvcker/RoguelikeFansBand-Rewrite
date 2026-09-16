// SPDX-License-Identifier: MPL-2.0
import { defaultVisuals, paletteColor, type VisualPreferences } from "./visual-preferences.ts";
import type { EditableVisualDto, Position } from "./protocol";

import {
  Application,
  Container,
  Graphics,
  Rectangle,
  RendererType,
  Sprite,
  type Filter,
  type Texture,
} from "pixi.js";

import { MAP_CELL_SIZE, type CameraTransform } from "./camera.ts";
import { ensureContrast } from "./render-color";
import {
  TERRAIN_CHUNK_SIZE,
  chunkIndexForCell,
  createRenderChunkLayout,
  updateTerrainChunkState,
  visibleRenderChunkIndexes,
  type RenderChunk,
  type RenderChunkLayout,
} from "./render-chunks";
import type {
  BackendInitialization,
  RenderCell,
  RendererBackend,
  RendererBackendDiagnostics,
  TilesetChangeResult,
} from "./renderer-backend";
import { TilesetRuntime, type RuntimeTileVisual } from "./tileset-runtime";
import { createUniqueRainbowFilter } from "./unique-rainbow-filter";
import { PLAYER_MELEE_MS, meleeImpact, playerMeleeOffset, playerStepPosition, playerStepProgress,
  type PlayerMeleeAttack } from "./player-motion.ts";
import { cellAppearance } from "./cell-appearance.ts";
import { PROJECTILE_IMPACT_MS, projectilePosition, projectileArrival, projectileAreaFrame, projectileColor, type ProjectileFlight, type ProjectileHit } from "./projectile-motion.ts";

const rgb = (color: string) => Number.parseInt(color.slice(1), 16);
const DYNAMIC_DISPLAY_OBJECTS_PER_CELL = 7;
const MAX_SPARE_DYNAMIC_VIEWS_PER_SHAPE = 1;

export interface PixiRendererBackendOptions {
  terrainChunkSize?: number;
}

interface CellView {
  itemBackground: Graphics;
  itemSymbol: Sprite;
  actorBackground: Graphics;
  actorSymbol: Sprite;
  visibilityMask: Graphics;
  lightColor: Graphics;
  darkness: Graphics;
}

interface TerrainChunkView {
  descriptor: RenderChunk;
  terrainSprite: Sprite;
  terrainTexture?: Texture;
}

interface DynamicChunkView {
  descriptorIndex?: number;
  cellWidth: number;
  cellHeight: number;
  objectLayer: Container;
  actorLayer: Container;
  visibilityLayer: Container;
  lightingLayer: Container;
  cells: CellView[];
}

interface ProjectileView {
  flight: ProjectileFlight;
  sprite: Sprite;
  color: number;
  blast: { position: Position; at: number }[];
  hits: { hit: ProjectileHit; ghost: Sprite; flash: Sprite }[];
}

export class PixiRendererBackend implements RendererBackend {
  readonly id = "pixi-layered-chunks-v3";
  #visuals = defaultVisuals();
  #visualCatalog: readonly EditableVisualDto[] = [];
  readonly #application = new Application();
  readonly #camera = new Container();
  readonly #terrainLayer = new Container();
  readonly #objectLayer = new Container();
  readonly #actorLayer = new Container();
  readonly #meleeLayer = new Container();
  readonly #meleeCorpse = cellSprite(0, 0);
  readonly #meleeFlash = cellSprite(0, 0);
  readonly #projectileRoot = new Container();
  readonly #projectileLayer = new Container();
  readonly #projectileMask = new Graphics();
  readonly #projectileInk = new Graphics();
  #projectileViews: ProjectileView[] = [];
  readonly #playerLayer = new Container();
  readonly #playerBackground = new Graphics();
  readonly #playerSprite = cellSprite(0, 0);
  #playerTarget: Position | undefined;
  #playerMotion:
    | { kind: "step"; from: Position; to: Position; startedAt: number; continuous: boolean }
    | { kind: "melee"; position: Position; attack: PlayerMeleeAttack; startedAt: number }
    | { kind: "projectiles"; position: Position; startedAt: number; duration: number }
    | undefined;
  #playerSettled: Promise<void> = Promise.resolve();
  #resolvePlayerMotion: (() => void) | undefined;
  #onPlayerPosition: BackendInitialization["onPlayerPosition"];
  #onCameraImpulse: BackendInitialization["onCameraImpulse"];
  #meleeCameraShake = true;
  readonly #visibilityLayer = new Container();
  readonly #lightingLayer = new Container();
  readonly #activeDynamicViews = new Map<number, DynamicChunkView>();
  readonly #dynamicViewPools = new Map<string, DynamicChunkView[]>();
  readonly #allocatedDynamicViews = new Set<DynamicChunkView>();
  readonly #terrainChunkSize: number;
  #layout: RenderChunkLayout | undefined;
  #chunks: TerrainChunkView[] = [];
  #renderCells: Array<RenderCell | undefined> = [];
  readonly #cellTransitionFrom = new Map<number, RenderCell>();
  #cellTransitionProgress = 1;
  #terrainIds: Array<string | undefined> = [];
  #tileset: TilesetRuntime | undefined;
  #contentGlyphs: Readonly<Record<string, string>> = {};
  #host: HTMLElement | undefined;
  #width = 0;
  #height = 0;
  #zoom: CameraTransform["zoom"] = 1;
  #forceTerrainRebuild = true;
  #visibleChunkCount = 0;
  #lastRebuiltTerrainChunks = 0;
  #totalRebuiltTerrainChunks = 0;
  #rainbowFilter: Filter | undefined;
  readonly #rainbowSprites = new Set<Sprite>();
  #motionPreference: MediaQueryList | undefined;
  #animatingRainbow = false;
  readonly #animatePlayer = () => {
    const motion = this.#playerMotion!;
    const elapsed = performance.now() - motion.startedAt;
    if (motion.kind === "projectiles") {
      if (elapsed >= motion.duration) this.#finishPlayerMotion();
      else this.#drawProjectiles(elapsed);
      return;
    }
    if (motion.kind === "melee") {
      if (elapsed >= PLAYER_MELEE_MS) this.#finishPlayerMotion();
      else {
        const offset = playerMeleeOffset(motion.attack.direction, elapsed);
        // Local character offset only: camera, light/FOV and hit testing stay at the real tile.
        this.#playerLayer.position.set((motion.position.x + offset.x) * MAP_CELL_SIZE,
          (motion.position.y + offset.y) * MAP_CELL_SIZE);
        const impact = meleeImpact(elapsed);
        this.#meleeCorpse.alpha = impact.corpseAlpha;
        this.#meleeFlash.alpha = 0.75 * impact.flash *
          (motion.attack.outcome === "kill" ? impact.corpseAlpha : 1);
        if (this.#meleeCameraShake && motion.attack.outcome !== "miss") {
          const direction = motion.attack.direction;
          const scale = impact.cameraPixels / Math.hypot(direction.x, direction.y);
          this.#onCameraImpulse?.({ x: direction.x * scale, y: direction.y * scale });
        }
      }
      return;
    }
    const progress = playerStepProgress(elapsed, motion.continuous);
    if (progress >= 1) this.#finishPlayerMotion();
    else {
      this.#updateCellTransition(progress);
      this.#placePlayer(playerStepPosition(motion.from, motion.to, elapsed, motion.continuous));
    }
  };
  readonly #animateRainbow = () => {
    this.#rainbowFilter!.resources.rainbow.uniforms.uPhase = (performance.now() % 8000) / 8000;
  };
  readonly #syncMapAnimation = () => {
    const motionAllowed = !this.#motionPreference?.matches &&
      this.#host?.ownerDocument.visibilityState === "visible";
    if (!motionAllowed && this.#playerMotion) this.#finishPlayerMotion();
    const moving = this.#rainbowSprites.size > 0 && this.#visuals.uniqueEffect === "flowing" && motionAllowed;
    if (moving !== this.#animatingRainbow) {
      this.#animatingRainbow = moving;
      if (moving) this.#application.ticker.add(this.#animateRainbow);
      else this.#application.ticker.remove(this.#animateRainbow);
    }
    if (!moving && this.#rainbowFilter) this.#rainbowFilter.resources.rainbow.uniforms.uPhase = 0;
  };

  constructor(options: PixiRendererBackendOptions = {}) {
    const terrainChunkSize = options.terrainChunkSize ?? TERRAIN_CHUNK_SIZE;
    if (!Number.isInteger(terrainChunkSize) || terrainChunkSize <= 0) {
      throw new Error("terrain chunk size must be a positive integer");
    }
    this.#terrainChunkSize = terrainChunkSize;
  }

  async initialize(options: BackendInitialization): Promise<TilesetChangeResult> {
    this.#host = options.host;
    this.#onPlayerPosition = options.onPlayerPosition;
    this.#onCameraImpulse = options.onCameraImpulse;
    this.#width = options.width;
    this.#height = options.height;
    this.#zoom = options.zoom ?? 1;
    this.#contentGlyphs = options.contentGlyphs;
    this.#layout = createRenderChunkLayout(
      options.width,
      options.height,
      this.#terrainChunkSize,
    );
    this.#tileset = await TilesetRuntime.load(options.tilesetManifestUrl, options.contentGlyphs);
    await this.#application.init({
      width: options.width * MAP_CELL_SIZE * this.#zoom,
      height: options.height * MAP_CELL_SIZE * this.#zoom,
      background: "#090d12",
      antialias: false,
      resolution: window.devicePixelRatio,
      autoDensity: true,
    });
    this.#application.canvas.setAttribute("aria-label", options.canvasLabel);
    this.#motionPreference = window.matchMedia("(prefers-reduced-motion: reduce)");
    this.#motionPreference.addEventListener("change", this.#syncMapAnimation);
    options.host.ownerDocument.addEventListener("visibilitychange", this.#syncMapAnimation);
    options.host.replaceChildren(this.#application.canvas);
    this.#camera.scale.set(this.#zoom);
    this.#application.stage.addChild(this.#camera);
    this.#camera.addChild(
      this.#terrainLayer,
      this.#objectLayer,
      this.#actorLayer,
      this.#meleeLayer,
      this.#projectileRoot,
      this.#visibilityLayer,
      this.#playerLayer,
      this.#lightingLayer,
    );
    this.#meleeLayer.addChild(this.#meleeCorpse, this.#meleeFlash);
    this.#meleeCorpse.visible = this.#meleeFlash.visible = false;
    this.#meleeFlash.blendMode = "add";
    this.#projectileRoot.addChild(this.#projectileMask, this.#projectileLayer);
    this.#projectileLayer.addChild(this.#projectileInk);
    this.#projectileLayer.mask = this.#projectileMask;
    this.#projectileRoot.visible = false;
    this.#playerSprite.visible = false;
    this.#playerLayer.addChild(this.#playerBackground, this.#playerSprite);
    this.#createTerrainChunks();
    return this.#tilesetResult();
  }

  getDiagnostics(): RendererBackendDiagnostics {
    const cellViewCount = [...this.#allocatedDynamicViews].reduce(
      (total, view) => total + view.cells.length,
      0,
    );
    return {
      terrainChunkSize: this.#terrainChunkSize,
      terrainChunkCount: this.#chunks.length,
      visibleChunkCount: this.#visibleChunkCount,
      lastRebuiltTerrainChunks: this.#lastRebuiltTerrainChunks,
      totalRebuiltTerrainChunks: this.#totalRebuiltTerrainChunks,
      activeDynamicChunkCount: this.#activeDynamicViews.size,
      pooledDynamicChunkCount:
        this.#allocatedDynamicViews.size - this.#activeDynamicViews.size,
      cellViewCount,
      dynamicDisplayObjectCount: cellViewCount * DYNAMIC_DISPLAY_OBJECTS_PER_CELL + 6 +
        this.#projectileViews.reduce((count, view) => count + 1 + view.hits.length * 2, 0),
    };
  }

  resize(width: number, height: number): void {
    if (width === this.#width && height === this.#height) return;
    this.#finishPlayerMotion();
    this.#application.ticker.remove(this.#animatePlayer);
    this.#playerMotion = undefined;
    this.#playerTarget = undefined;
    this.#playerSprite.visible = false;
    this.#playerBackground.clear();

    for (const view of [...this.#allocatedDynamicViews]) this.#destroyDynamicView(view);
    for (const chunk of this.#chunks) {
      chunk.terrainTexture?.destroy(true);
      chunk.terrainSprite.destroy();
    }
    this.#activeDynamicViews.clear();
    this.#dynamicViewPools.clear();
    this.#width = width;
    this.#height = height;
    this.#layout = createRenderChunkLayout(width, height, this.#terrainChunkSize);
    this.#chunks = [];
    this.#renderCells = [];
    this.#terrainIds = [];
    this.#forceTerrainRebuild = true;
    this.#visibleChunkCount = 0;
    this.#lastRebuiltTerrainChunks = 0;
    this.#createTerrainChunks();
    this.#syncMapAnimation();
  }

  setCameraTransform(transform: CameraTransform): void {
    this.#zoom = transform.zoom;
    const canvasWidth = transform.cullingEnabled
      ? transform.viewportWidth
      : Math.max(transform.viewportWidth, this.#width * MAP_CELL_SIZE * this.#zoom);
    const canvasHeight = transform.cullingEnabled
      ? transform.viewportHeight
      : Math.max(transform.viewportHeight, this.#height * MAP_CELL_SIZE * this.#zoom);
    if (
      this.#application.renderer.screen.width !== canvasWidth ||
      this.#application.renderer.screen.height !== canvasHeight
    ) {
      this.#application.renderer.resize(canvasWidth, canvasHeight);
    }
    this.#camera.scale.set(this.#zoom);
    this.#camera.position.set(transform.x, transform.y);
    this.#applyChunkCulling(transform);
  }

  applyCells(cells: readonly RenderCell[]): number {
    const layout = this.#layout;
    if (!layout) return 0;
    const dirtyChunks = updateTerrainChunkState(
      this.#terrainIds,
      cells,
      layout.chunksAcross,
      this.#chunks.length,
      this.#forceTerrainRebuild,
      this.#terrainChunkSize,
    );

    let applied = 0;
    for (const cell of cells) {
      if (cell.index < 0 || cell.index >= this.#renderCells.length) continue;
      const previous = this.#renderCells[cell.index];
      if (this.#playerMotion?.kind === "step" && previous && !this.#cellTransitionFrom.has(cell.index) &&
          (previous.visibility !== cell.visibility || previous.light.color !== cell.light.color ||
           previous.light.intensity !== cell.light.intensity)) {
        this.#cellTransitionFrom.set(cell.index, previous);
      }
      this.#renderCells[cell.index] = cell;
      if (cell.actorPlayer && this.#tileset) {
        const visual = cell.actorKindId ? this.#tileset.resolve(cell.actorKindId) : undefined;
        const terrain = this.#tileset.resolve(cell.terrainId);
        applyLayerVisual(this.#playerBackground, this.#playerSprite, 0, 0, visual,
          terrain.background ?? rgb(this.#visuals.theme.background));
      }
      const chunkIndex = chunkIndexForCell(
        cell.x,
        cell.y,
        layout.chunksAcross,
        this.#terrainChunkSize,
      );
      const view = this.#activeDynamicViews.get(chunkIndex);
      const chunk = this.#chunks[chunkIndex];
      if (view && chunk && this.#tileset) {
        const localX = cell.x - chunk.descriptor.cellX;
        const localY = cell.y - chunk.descriptor.cellY;
        const cellView = view.cells[localY * view.cellWidth + localX];
        if (cellView) {
          this.#applyDynamicCell(cellView, cell, this.#tileset, localX, localY);
        }
      }
      applied += 1;
    }

    for (const chunkIndex of [...dirtyChunks].sort((left, right) => left - right)) {
      this.#rebuildTerrainChunk(chunkIndex);
    }
    this.#forceTerrainRebuild = false;
    this.#lastRebuiltTerrainChunks = dirtyChunks.size;
    this.#totalRebuiltTerrainChunks += dirtyChunks.size;
    // Cell application may rebuild terrain; start timing after that work, at the old appearance.
    if (this.#playerMotion?.kind === "step" && this.#cellTransitionProgress === 0) this.#playerMotion.startedAt = performance.now();
    this.#syncMapAnimation();
    return applied;
  }

  get playerMoving(): boolean { return this.#playerMotion !== undefined; }

  whenPlayerSettled(): Promise<void> { return this.#playerSettled; }

  setPlayerPosition(position: Position, animate: boolean, continuous = false): void {
    // Unexpected rapid manual updates finish at the actual corner, never
    // interpolate across it. Automated actions wait for this step to settle.
    this.#finishPlayerMotion();
    const from = this.#playerTarget;
    this.#playerTarget = { ...position };
    if (animate && from && !this.#motionPreference?.matches &&
        this.#host?.ownerDocument.visibilityState === "visible") {
      this.#playerMotion = { kind: "step", from, to: { ...position }, startedAt: performance.now(), continuous };
      this.#cellTransitionProgress = 0;
      this.#playerSettled = new Promise(resolve => { this.#resolvePlayerMotion = resolve; });
      this.#placePlayer(from);
      this.#application.ticker.add(this.#animatePlayer);
    } else {
      this.#placePlayer(position);
    }
  }

  #placePlayer(position: Position): void {
    this.#playerLayer.position.set(position.x * MAP_CELL_SIZE, position.y * MAP_CELL_SIZE);
    this.#onPlayerPosition?.(position);
  }

  setMeleeCameraShake(enabled: boolean): void {
    this.#meleeCameraShake = enabled;
    if (!enabled) this.#onCameraImpulse?.({ x: 0, y: 0 });
  }

  playPlayerMelee(attack: PlayerMeleeAttack, target?: RenderCell): void {
    this.#finishPlayerMotion();
    if (!this.#playerTarget || this.#motionPreference?.matches ||
        this.#host?.ownerDocument.visibilityState !== "visible") return;
    if (target?.actorKindId && this.#tileset) {
      const visual = this.#tileset.resolve(target.actorKindId);
      applyVisual(this.#meleeCorpse, visual, rgb(this.#visuals.theme.background));
      this.#meleeCorpse.position.set(target.x * MAP_CELL_SIZE, target.y * MAP_CELL_SIZE);
      this.#meleeCorpse.alpha = 1;
      this.#meleeCorpse.visible = attack.outcome === "kill";
      this.#meleeFlash.texture = visual.texture;
      this.#meleeFlash.position.copyFrom(this.#meleeCorpse.position);
      this.#meleeFlash.tint = 0xffffff;
      this.#meleeFlash.alpha = 0;
      this.#meleeFlash.visible = true;
    }
    this.#playerMotion = { kind: "melee", position: { ...this.#playerTarget }, attack, startedAt: performance.now() };
    this.#playerSettled = new Promise(resolve => { this.#resolvePlayerMotion = resolve; });
    this.#application.ticker.add(this.#animatePlayer);
  }

  playProjectiles(flights: readonly ProjectileFlight[]): void {
    this.#finishPlayerMotion();
    if (!flights.length || !this.#tileset || !this.#playerTarget || this.#motionPreference?.matches ||
        this.#host?.ownerDocument.visibilityState !== "visible") return;
    const maskCells = new Set<number>();
    for (const flight of flights) {
      const sprite = cellSprite(0, 0);
      const visual = flight.kind === "throw" ? this.#tileset.resolve(flight.source)
        : this.#tileset.resolveGlyph(flight.kind === "arrow" ? "-" : "*");
      applyVisual(sprite, visual, rgb(this.#visuals.theme.background));
      const color = flight.kind !== "arrow" && flight.kind !== "throw" ? rgb(paletteColor(this.#visuals,
        `#${projectileColor(flight.damageType).toString(16).padStart(6, "0")}`))
        : this.#tileset.resolve(flight.source).tint;
      if (flight.kind !== "throw") sprite.tint = color;
      sprite.anchor.set(0.5);
      this.#projectileLayer.addChild(sprite);
      const hits = flight.hits.filter(hit => hit.target?.actorKindId).map(hit => {
        const target = hit.target!;
        const ghost = cellSprite(target.x, target.y), flash = cellSprite(target.x, target.y);
        const actor = this.#tileset!.resolve(target.actorKindId!);
        applyVisual(ghost, actor, rgb(this.#visuals.theme.background));
        flash.texture = actor.texture;
        flash.tint = 0xffffff;
        flash.blendMode = "add";
        this.#projectileLayer.addChild(ghost, flash);
        maskCells.add(target.index);
        return { hit, ghost, flash };
      });
      const blast = flight.kind === "ball" || flight.kind === "storm" || flight.kind === "meteor" ? flight.affectedPositions
        .filter(position => this.#renderCells[position.y * this.#width + position.x]?.visibility === "visible")
        .map(position => ({ position, at: projectileArrival(flight, position) })) : [];
      this.#projectileViews.push({ flight, sprite, color, hits, blast });
      for (const position of [...flight.path, ...flight.affectedPositions]) {
        if (position.x >= 0 && position.y >= 0 && position.x < this.#width && position.y < this.#height)
          maskCells.add(position.y * this.#width + position.x);
      }
    }
    // A hard visible-cell mask prevents tails and flashes leaking through remembered fog.
    for (const index of maskCells) {
      const cell = this.#renderCells[index];
      if (cell?.visibility === "visible") this.#projectileMask
        .rect(cell.x * MAP_CELL_SIZE, cell.y * MAP_CELL_SIZE, MAP_CELL_SIZE, MAP_CELL_SIZE);
    }
    this.#projectileMask.fill(0xffffff);
    this.#projectileRoot.visible = true;
    this.#playerMotion = { kind: "projectiles", position: { ...this.#playerTarget }, startedAt: performance.now(),
      duration: Math.max(...flights.map(flight => flight.delay + flight.duration + PROJECTILE_IMPACT_MS)) };
    this.#playerSettled = new Promise(resolve => { this.#resolvePlayerMotion = resolve; });
    this.#drawProjectiles(0);
    this.#application.ticker.add(this.#animatePlayer);
  }

  #drawProjectiles(elapsed: number): void {
    const ink = this.#projectileInk.clear();
    const samples = projectileAreaFrame(this.#projectileViews, elapsed);
    const detailStride = Math.max(1, Math.ceil(samples.length / 128));
    for (let i = 0; i < samples.length; i++) {
      const sample = samples[i]!;
      const x = (sample.position.x + 0.5) * MAP_CELL_SIZE, y = (sample.position.y + 0.5) * MAP_CELL_SIZE;
      ink.circle(x, y, MAP_CELL_SIZE * sample.radius).fill({ color: sample.color, alpha: sample.alpha });
      // Keep complete coverage; bound only decorative highlights to 128 per frame.
      if (i % detailStride !== 0) continue;
      if (sample.angle !== undefined) {
        const radius = MAP_CELL_SIZE * 0.28;
        ink.moveTo(x + Math.cos(sample.angle) * radius, y + Math.sin(sample.angle) * radius)
          .arc(x, y, radius, sample.angle, sample.angle + Math.PI * 0.7)
          .stroke({ color: 0xffffff, width: 1, alpha: sample.coreAlpha });
      } else ink.circle(x, y, MAP_CELL_SIZE * 0.1).fill({ color: 0xffffff, alpha: sample.coreAlpha });
    }
    const shownGhosts = new Set<string>();
    const flashes = new Map<string, { hit: ProjectileHit; age: number }>();
    for (const { flight, hits } of this.#projectileViews) for (const { hit } of hits) {
      const age = elapsed - flight.delay - hit.at;
      if (age >= 0 && age < PROJECTILE_IMPACT_MS && age < (flashes.get(hit.targetId)?.age ?? Infinity))
        flashes.set(hit.targetId, { hit, age });
    }
    for (const { flight, sprite, color, hits } of this.#projectileViews) {
      const time = elapsed - flight.delay;
      sprite.visible = flight.kind !== "beam" && time >= 0 && time < flight.travelDuration;
      if (sprite.visible) {
        const head = projectilePosition(flight, time), tail = projectilePosition(flight, time - 16);
        sprite.position.set((head.position.x + 0.5) * MAP_CELL_SIZE, (head.position.y + 0.5) * MAP_CELL_SIZE);
        sprite.rotation = flight.kind === "arrow" ? head.angle : 0;
        if (flight.kind === "meteor") sprite.y -= (1 - time / flight.travelDuration) * MAP_CELL_SIZE * 0.4;
        else ink.moveTo((tail.position.x + 0.5) * MAP_CELL_SIZE, (tail.position.y + 0.5) * MAP_CELL_SIZE)
          .lineTo(sprite.x, sprite.y).stroke({ color, width: flight.kind === "bolt" ? 2 : 1, alpha: 0.4 });
      }
      if (flight.kind === "beam" && time >= 0 && time < flight.duration + PROJECTILE_IMPACT_MS) {
        const head = projectilePosition(flight, time).position;
        const fade = 1 - Math.max(0, time - flight.duration) / PROJECTILE_IMPACT_MS;
        // Retain the revealed polyline: this is a beam, not a moving bolt head.
        const revealed = flight.path.filter(position => projectileArrival(flight, position) <= time);
        for (const stroke of [{ color, width: 7, alpha: 0.3 }, { color, width: 3, alpha: 0.8 },
          { color: 0xffffff, width: 1, alpha: 0.9 }]) {
          const origin = flight.path[0]!;
          ink.moveTo((origin.x + 0.5) * MAP_CELL_SIZE, (origin.y + 0.5) * MAP_CELL_SIZE);
          for (const position of revealed.slice(1)) ink.lineTo((position.x + 0.5) * MAP_CELL_SIZE, (position.y + 0.5) * MAP_CELL_SIZE);
          ink.lineTo((head.x + 0.5) * MAP_CELL_SIZE, (head.y + 0.5) * MAP_CELL_SIZE)
            .stroke({ ...stroke, alpha: stroke.alpha * fade });
        }
      }
      const impactAge = time - flight.duration;
      if (flight.wallImpact && impactAge >= 0 && impactAge < PROJECTILE_IMPACT_MS) {
        const position = flight.path.at(-1)!;
        const fade = 1 - impactAge / PROJECTILE_IMPACT_MS;
        ink.circle((position.x + 0.5) * MAP_CELL_SIZE, (position.y + 0.5) * MAP_CELL_SIZE, 2 + 3 * (1 - fade))
          .stroke({ color, width: 1, alpha: fade * 0.6 });
      }
      for (const { hit, ghost, flash } of hits) {
        const age = time - hit.at;
        const fade = 1 - Math.max(0, Math.min(1, age / PROJECTILE_IMPACT_MS));
        ghost.visible = hit.outcome === "kill" && fade > 0 && !shownGhosts.has(hit.targetId);
        if (ghost.visible) shownGhosts.add(hit.targetId);
        ghost.alpha = fade;
        flash.visible = flashes.get(hit.targetId)?.hit === hit;
        flash.alpha = fade * 0.75;
        if (flash.visible) ink.circle((hit.position.x + 0.5) * MAP_CELL_SIZE,
          (hit.position.y + 0.5) * MAP_CELL_SIZE, 2 + 6 * (1 - fade))
          .stroke({ color, width: 1, alpha: fade * 0.6 });
      }
    }
  }

  #finishPlayerMotion(): void {
    const motion = this.#playerMotion;
    if (!motion) return;
    this.#playerMotion = undefined;
    this.#application.ticker.remove(this.#animatePlayer);
    this.#updateCellTransition(1);
    this.#cellTransitionFrom.clear();
    this.#meleeCorpse.visible = this.#meleeFlash.visible = false;
    if (motion.kind === "projectiles") {
      for (const view of this.#projectileViews) {
        view.sprite.destroy();
        for (const hit of view.hits) { hit.ghost.destroy(); hit.flash.destroy(); }
      }
      this.#projectileViews = [];
      this.#projectileInk.clear();
      this.#projectileMask.clear();
      this.#projectileRoot.visible = false;
    }
    if (motion.kind === "melee") this.#onCameraImpulse?.({ x: 0, y: 0 });
    this.#placePlayer(motion.kind === "step" ? motion.to : motion.position);
    this.#resolvePlayerMotion?.();
    this.#resolvePlayerMotion = undefined;
  }

  async setTileset(tilesetManifestUrl: string): Promise<TilesetChangeResult> {
    const replacement = await TilesetRuntime.load(tilesetManifestUrl, this.#contentGlyphs);
    this.#finishPlayerMotion();
    const previous = this.#tileset;
    replacement.setVisuals(this.#visuals, this.#visualCatalog);
    this.#tileset = replacement;
    this.#forceTerrainRebuild = true;
    previous?.destroy();
    return this.#tilesetResult();
  }

  setVisuals(preferences: VisualPreferences, catalog: readonly EditableVisualDto[]): boolean {
    this.#visuals = preferences; this.#visualCatalog = catalog;
    const changed = this.#tileset?.setVisuals(preferences, catalog) ?? false;
    if (changed) {
      this.#finishPlayerMotion();
      this.#forceTerrainRebuild = true;
      this.#application.renderer.background.color = rgb(preferences.theme.background);
    }
    return changed;
  }
  visualBase(id: string) { return this.#tileset!.visualBase(id); }

  setCanvasLabel(label: string): void {
    if (this.#host) this.#application.canvas.setAttribute("aria-label", label);
  }

  capturePng(): string {
    this.#application.render();
    const source = this.#application.canvas as HTMLCanvasElement;
    const host = this.#host!;
    const scaleX = source.width / source.clientWidth, scaleY = source.height / source.clientHeight;
    const target = host.ownerDocument.createElement("canvas");
    target.width = Math.round(Math.min(host.clientWidth, source.clientWidth - host.scrollLeft) * scaleX);
    target.height = Math.round(Math.min(host.clientHeight, source.clientHeight - host.scrollTop) * scaleY);
    target.getContext("2d")!.drawImage(source, host.scrollLeft * scaleX, host.scrollTop * scaleY,
      target.width, target.height, 0, 0, target.width, target.height);
    return target.toDataURL("image/png");
  }

  destroy(): void {
    this.#finishPlayerMotion();
    this.#host?.ownerDocument.removeEventListener("visibilitychange", this.#syncMapAnimation);
    this.#motionPreference?.removeEventListener("change", this.#syncMapAnimation);
    this.#application.ticker?.remove(this.#animateRainbow);
    this.#application.ticker?.remove(this.#animatePlayer);
    this.#playerMotion = undefined;
    this.#playerTarget = undefined;
    this.#onPlayerPosition = undefined;
    this.#onCameraImpulse = undefined;
    for (const view of [...this.#allocatedDynamicViews]) this.#destroyDynamicView(view);
    for (const chunk of this.#chunks) chunk.terrainTexture?.destroy(true);
    this.#tileset?.destroy();
    this.#tileset = undefined;
    this.#rainbowFilter?.destroy();
    this.#rainbowFilter = undefined;
    this.#rainbowSprites.clear();
    this.#animatingRainbow = false;
    this.#layout = undefined;
    this.#chunks = [];
    this.#renderCells = [];
    this.#terrainIds = [];
    this.#activeDynamicViews.clear();
    this.#dynamicViewPools.clear();
    this.#host = undefined;
    if (this.#application.renderer) this.#application.destroy(true, { children: true });
  }

  #createTerrainChunks(): void {
    const layout = this.#layout;
    if (!layout) throw new Error("render chunk layout is not initialized");
    this.#chunks = layout.chunks.map((descriptor) => {
      const terrainSprite = new Sprite({ roundPixels: true });
      terrainSprite.position.set(
        descriptor.cellX * MAP_CELL_SIZE,
        descriptor.cellY * MAP_CELL_SIZE,
      );
      terrainSprite.width = descriptor.cellWidth * MAP_CELL_SIZE;
      terrainSprite.height = descriptor.cellHeight * MAP_CELL_SIZE;
      terrainSprite.visible = false;
      this.#terrainLayer.addChild(terrainSprite);
      return { descriptor, terrainSprite };
    });
    this.#visibleChunkCount = 0;
    this.#terrainIds = new Array(this.#width * this.#height);
    this.#renderCells = new Array(this.#width * this.#height);
  }

  #createDynamicView(cellWidth: number, cellHeight: number): DynamicChunkView {
    const objectLayer = new Container({ visible: false });
    const actorLayer = new Container({ visible: false });
    const visibilityLayer = new Container({ visible: false });
    const lightingLayer = new Container({ visible: false });
    this.#objectLayer.addChild(objectLayer);
    this.#actorLayer.addChild(actorLayer);
    this.#visibilityLayer.addChild(visibilityLayer);
    this.#lightingLayer.addChild(lightingLayer);
    const cells: CellView[] = [];
    for (let localY = 0; localY < cellHeight; localY += 1) {
      for (let localX = 0; localX < cellWidth; localX += 1) {
        const itemBackground = new Graphics();
        const itemSymbol = cellSprite(localX, localY);
        const actorBackground = new Graphics();
        const actorSymbol = cellSprite(localX, localY);
        const visibilityMask = new Graphics();
        const lightColor = new Graphics();
        const darkness = new Graphics();
        itemSymbol.visible = false;
        actorSymbol.visible = false;
        objectLayer.addChild(itemBackground, itemSymbol);
        actorLayer.addChild(actorBackground, actorSymbol);
        visibilityLayer.addChild(visibilityMask);
        lightingLayer.addChild(lightColor, darkness);
        cells.push({
          itemBackground,
          itemSymbol,
          actorBackground,
          actorSymbol,
          visibilityMask,
          lightColor,
          darkness,
        });
      }
    }
    const view = {
      cellWidth,
      cellHeight,
      objectLayer,
      actorLayer,
      visibilityLayer,
      lightingLayer,
      cells,
    };
    this.#allocatedDynamicViews.add(view);
    return view;
  }

  #assignDynamicView(chunkIndex: number): void {
    const chunk = this.#chunks[chunkIndex];
    if (!chunk || this.#activeDynamicViews.has(chunkIndex)) return;
    const { descriptor } = chunk;
    const poolKey = dynamicViewPoolKey(descriptor.cellWidth, descriptor.cellHeight);
    const pool = this.#dynamicViewPools.get(poolKey);
    const view = pool?.pop() ?? this.#createDynamicView(
      descriptor.cellWidth,
      descriptor.cellHeight,
    );
    view.descriptorIndex = chunkIndex;
    const x = descriptor.cellX * MAP_CELL_SIZE;
    const y = descriptor.cellY * MAP_CELL_SIZE;
    for (const layer of dynamicViewLayers(view)) {
      layer.position.set(x, y);
      layer.visible = true;
    }
    this.#activeDynamicViews.set(chunkIndex, view);
    this.#renderDynamicChunk(view, descriptor);
  }

  #releaseDynamicView(chunkIndex: number): void {
    const view = this.#activeDynamicViews.get(chunkIndex);
    if (!view) return;
    this.#activeDynamicViews.delete(chunkIndex);
    for (const cell of view.cells) this.#clearRainbow(cell.actorSymbol);
    view.descriptorIndex = undefined;
    for (const layer of dynamicViewLayers(view)) layer.visible = false;
    const poolKey = dynamicViewPoolKey(view.cellWidth, view.cellHeight);
    const pool = this.#dynamicViewPools.get(poolKey) ?? [];
    pool.push(view);
    this.#dynamicViewPools.set(poolKey, pool);
  }

  #trimDynamicViewPools(): void {
    for (const [poolKey, pool] of this.#dynamicViewPools) {
      while (pool.length > MAX_SPARE_DYNAMIC_VIEWS_PER_SHAPE) {
        const view = pool.pop();
        if (view) this.#destroyDynamicView(view);
      }
      if (pool.length === 0) this.#dynamicViewPools.delete(poolKey);
    }
  }

  #destroyDynamicView(view: DynamicChunkView): void {
    for (const cell of view.cells) this.#clearRainbow(cell.actorSymbol);
    this.#allocatedDynamicViews.delete(view);
    for (const layer of dynamicViewLayers(view)) layer.destroy({ children: true });
  }

  #renderDynamicChunk(view: DynamicChunkView, descriptor: RenderChunk): void {
    const tileset = this.#tileset;
    if (!tileset) return;
    for (let localY = 0; localY < descriptor.cellHeight; localY += 1) {
      for (let localX = 0; localX < descriptor.cellWidth; localX += 1) {
        const cellView = view.cells[localY * view.cellWidth + localX];
        if (!cellView) continue;
        const worldX = descriptor.cellX + localX;
        const worldY = descriptor.cellY + localY;
        const cell = this.#renderCells[worldY * this.#width + worldX];
        if (cell) this.#applyDynamicCell(cellView, cell, tileset, localX, localY);
        else { this.#clearRainbow(cellView.actorSymbol); resetCellView(cellView); }
      }
    }
  }

  #applyDynamicCell(
    view: CellView,
    cell: RenderCell,
    tileset: TilesetRuntime,
    localX: number,
    localY: number,
  ): void {
    const terrain = tileset.resolve(cell.terrainId);
    const item = cell.itemKindId ? tileset.resolve(cell.itemKindId) : undefined;
    const actor = cell.actorPlayer ? undefined : cell.actorGlyph
      ? tileset.resolveGlyph(cell.actorGlyph)
      : cell.actorKindId
        ? tileset.resolve(cell.actorKindId)
        : undefined;
    const terrainBackground = terrain.background ?? rgb(this.#visuals.theme.background);
    applyLayerVisual(
      view.itemBackground,
      view.itemSymbol,
      localX,
      localY,
      item,
      terrainBackground,
    );
    applyLayerVisual(
      view.actorBackground,
      view.actorSymbol,
      localX,
      localY,
      actor && cell.highlightPet ? { ...actor, background: rgb(this.#visuals.theme.pet) } : actor,
      item?.background ?? terrainBackground,
    );
    // Pixi's existing software Canvas fallback cannot execute custom GPU filters.
    if (this.#application.renderer.type !== RendererType.CANVAS && actor?.source === "glyph" && cell.actorKindId && !cell.actorGlyph &&
        cell.visibility === "visible" && tileset.uniqueEffect(cell.actorKindId, cell.actorUnique === true) !== "off") {
      this.#rainbowFilter ??= createUniqueRainbowFilter();
      if (!this.#rainbowSprites.has(view.actorSymbol)) view.actorSymbol.filters = [this.#rainbowFilter];
      this.#rainbowSprites.add(view.actorSymbol);
      view.actorSymbol.tint = 0xffffff;
    } else this.#clearRainbow(view.actorSymbol);
    this.#drawCellAppearance(view, cell, localX, localY);
  }

  #drawCellAppearance(view: CellView, cell: RenderCell, localX: number, localY: number): void {
    const from = this.#cellTransitionFrom.get(cell.index) ?? cell;
    const appearance = cellAppearance(from, cell, this.#visuals.theme, this.#cellTransitionProgress);
    drawOverlay(view.visibilityMask, localX, localY, appearance.fog);
    drawOverlay(view.lightColor, localX, localY, appearance.tint);
    drawOverlay(view.darkness, localX, localY, appearance.darkness);
    view.actorBackground.alpha = view.actorSymbol.alpha = appearance.actorAlpha;
    view.itemBackground.alpha = view.itemSymbol.alpha = appearance.itemAlpha;
  }

  #updateCellTransition(progress: number): void {
    this.#cellTransitionProgress = progress;
    const layout = this.#layout;
    if (!layout) return;
    // Update only changed masks in active chunks, never terrain textures or actor visuals.
    for (const index of this.#cellTransitionFrom.keys()) {
      const cell = this.#renderCells[index]!;
      const chunkIndex = chunkIndexForCell(cell.x, cell.y, layout.chunksAcross, this.#terrainChunkSize);
      const view = this.#activeDynamicViews.get(chunkIndex);
      if (!view) continue;
      const descriptor = this.#chunks[chunkIndex]!.descriptor;
      const localX = cell.x - descriptor.cellX, localY = cell.y - descriptor.cellY;
      this.#drawCellAppearance(view.cells[localY * view.cellWidth + localX]!, cell, localX, localY);
    }
  }

  #clearRainbow(sprite: Sprite): void {
    if (this.#rainbowSprites.delete(sprite)) sprite.filters = null;
  }

  #rebuildTerrainChunk(chunkIndex: number): void {
    const chunk = this.#chunks[chunkIndex];
    const tileset = this.#tileset;
    if (!chunk || !tileset) return;
    const { descriptor } = chunk;
    const source = new Container();
    for (let localY = 0; localY < descriptor.cellHeight; localY += 1) {
      for (let localX = 0; localX < descriptor.cellWidth; localX += 1) {
        const worldX = descriptor.cellX + localX;
        const worldY = descriptor.cellY + localY;
        const terrainId = this.#terrainIds[worldY * this.#width + worldX];
        if (!terrainId) continue;
        const terrain = tileset.resolve(terrainId);
        const background = terrain.background ?? rgb(this.#visuals.theme.background);
        const terrainBackground = new Graphics();
        drawTerrainBackground(terrainBackground, localX, localY, background, rgb(this.#visuals.theme.grid));
        const terrainSymbol = cellSprite(localX, localY);
        applyVisual(terrainSymbol, terrain, background);
        source.addChild(terrainBackground, terrainSymbol);
      }
    }
    const pixelWidth = descriptor.cellWidth * MAP_CELL_SIZE;
    const pixelHeight = descriptor.cellHeight * MAP_CELL_SIZE;
    const texture = this.#application.renderer.generateTexture({
      target: source,
      frame: new Rectangle(0, 0, pixelWidth, pixelHeight),
      resolution: window.devicePixelRatio,
      antialias: false,
      textureSourceOptions: { scaleMode: "nearest" },
    });
    source.destroy({ children: true });
    chunk.terrainTexture?.destroy(true);
    chunk.terrainTexture = texture;
    chunk.terrainSprite.texture = texture;
    chunk.terrainSprite.width = pixelWidth;
    chunk.terrainSprite.height = pixelHeight;
  }

  #applyChunkCulling(transform: CameraTransform): void {
    const layout = this.#layout;
    if (!layout) return;
    const visible = visibleRenderChunkIndexes(layout.chunks, transform);
    this.#visibleChunkCount = visible.size;
    for (const chunk of this.#chunks) {
      chunk.terrainSprite.visible = visible.has(chunk.descriptor.index);
    }
    for (const chunkIndex of [...this.#activeDynamicViews.keys()]) {
      if (!visible.has(chunkIndex)) this.#releaseDynamicView(chunkIndex);
    }
    for (const chunkIndex of [...visible].sort((left, right) => left - right)) {
      this.#assignDynamicView(chunkIndex);
    }
    this.#trimDynamicViewPools();
    this.#syncMapAnimation();
  }

  #tilesetResult(): TilesetChangeResult {
    const tileset = this.#tileset;
    if (!tileset) throw new Error("tileset runtime is not initialized");
    return { id: tileset.manifest.id, warnings: tileset.warnings };
  }
}

function dynamicViewPoolKey(cellWidth: number, cellHeight: number): string {
  return `${cellWidth}x${cellHeight}`;
}

function dynamicViewLayers(view: DynamicChunkView): Container[] {
  return [
    view.objectLayer,
    view.actorLayer,
    view.visibilityLayer,
    view.lightingLayer,
  ];
}

function resetCellView(view: CellView): void {
  view.itemBackground.clear();
  view.actorBackground.clear();
  view.visibilityMask.clear();
  view.lightColor.clear();
  view.darkness.clear();
  view.itemSymbol.visible = false;
  view.actorSymbol.visible = false;
}

function cellSprite(x: number, y: number): Sprite {
  const sprite = new Sprite({ roundPixels: true });
  sprite.position.set(x * MAP_CELL_SIZE, y * MAP_CELL_SIZE);
  sprite.width = MAP_CELL_SIZE;
  sprite.height = MAP_CELL_SIZE;
  return sprite;
}

function applyLayerVisual(
  background: Graphics,
  sprite: Sprite,
  cellX: number,
  cellY: number,
  visual: RuntimeTileVisual | undefined,
  inheritedBackground: number,
): void {
  if (!visual) {
    background.clear();
    sprite.visible = false;
    return;
  }
  const layerBackground = visual.background ?? inheritedBackground;
  if (visual.background === undefined) background.clear();
  else drawBackground(background, cellX, cellY, layerBackground);
  applyVisual(sprite, visual, layerBackground);
}

function applyVisual(sprite: Sprite, visual: RuntimeTileVisual, background: number): void {
  sprite.visible = true;
  sprite.texture = visual.texture;
  sprite.tint = visual.source === "glyph" ? ensureContrast(visual.tint, background) : visual.tint;
}

function drawTerrainBackground(
  graphics: Graphics,
  cellX: number,
  cellY: number,
  color: number,
  gridColor: number,
): void {
  const x = cellX * MAP_CELL_SIZE;
  const y = cellY * MAP_CELL_SIZE;
  graphics
    .rect(x, y, MAP_CELL_SIZE, MAP_CELL_SIZE)
    .fill(color)
    .rect(x, y, MAP_CELL_SIZE, MAP_CELL_SIZE)
    .stroke({ color: gridColor, width: 1, alpha: 0.55 });
}

function drawBackground(
  graphics: Graphics,
  cellX: number,
  cellY: number,
  color: number,
): void {
  graphics
    .clear()
    .rect(cellX * MAP_CELL_SIZE, cellY * MAP_CELL_SIZE, MAP_CELL_SIZE, MAP_CELL_SIZE)
    .fill(color);
}

function drawOverlay(
  graphics: Graphics,
  cellX: number,
  cellY: number,
  overlay: { color: number; alpha: number },
): void {
  graphics.clear();
  if (overlay.alpha === 0) return;
  graphics
    .rect(cellX * MAP_CELL_SIZE, cellY * MAP_CELL_SIZE, MAP_CELL_SIZE, MAP_CELL_SIZE)
    .fill(overlay);
}
