// SPDX-License-Identifier: MPL-2.0

import type { AppState } from "./app-state";
import type { Localization } from "./localization";
import type {
  CellDto,
  HomeDto,
  ItemDto,
  Position,
  ShopDto,
  TaskServiceDto,
  VisibilityState,
} from "./protocol";

export type ObjectListCategory = "interesting" | "items";

export interface ObjectListEntry {
  readonly id: string;
  readonly category: ObjectListCategory;
  readonly position: Position;
  readonly name: string;
  readonly glyph: string;
  readonly distance: number;
  readonly offsetX: number;
  readonly offsetY: number;
  readonly quantity?: number;
}

export interface ObjectListProjection {
  readonly playerPosition: Position;
  readonly floorId: string;
  readonly cells: Iterable<CellDto>;
  readonly shops: readonly ShopDto[];
  readonly homes: readonly HomeDto[];
  readonly taskServices: readonly TaskServiceDto[];
  readonly items: readonly ItemDto[];
  readonly includeStairs: boolean;
  readonly visibilityAt: (position: Position) => VisibilityState | undefined;
  readonly glyphFor: (contentId: string) => string | undefined;
  readonly localize: (nameKey: string) => string;
  readonly contentName: (contentId: string) => string;
  readonly visibleItemName: (displayNameKey: string, kindId: string) => string;
}

interface ObjectListDom {
  readonly dialog: HTMLDialogElement;
  readonly close: HTMLButtonElement;
  readonly stairsToggle: HTMLButtonElement;
  readonly host: HTMLElement;
}

export class ObjectListPanel {
  readonly #document: Document;
  readonly #window: Window;
  readonly #state: AppState;
  readonly #localization: Localization;
  readonly #contentName: (contentId: string) => string;
  readonly #visibleItemName: (displayNameKey: string, kindId: string) => string;
  readonly #onTravel: (position: Position) => void;
  readonly #dom: ObjectListDom;
  #entries: ObjectListEntry[] = [];
  #selectedIndex = 0;
  #includeStairs = false;
  #installed = false;

  constructor(options: {
    document: Document;
    window: Window;
    state: AppState;
    localization: Localization;
    contentName: (contentId: string) => string;
    visibleItemName: (displayNameKey: string, kindId: string) => string;
    onTravel: (position: Position) => void;
  }) {
    this.#document = options.document;
    this.#window = options.window;
    this.#state = options.state;
    this.#localization = options.localization;
    this.#contentName = options.contentName;
    this.#visibleItemName = options.visibleItemName;
    this.#onTravel = options.onTravel;
    this.#dom = createObjectListDom(this.#document);
  }

  install(): void {
    if (this.#installed) return;
    this.#installed = true;
    this.#dom.close.addEventListener("click", this.#close);
    this.#dom.stairsToggle.addEventListener("click", this.#toggleStairs);
    this.#dom.host.addEventListener("click", this.#selectRow);
    this.#dom.dialog.addEventListener("keydown", this.#handleKeydown);
  }

  dispose(): void {
    if (!this.#installed) return;
    this.#installed = false;
    this.#dom.close.removeEventListener("click", this.#close);
    this.#dom.stairsToggle.removeEventListener("click", this.#toggleStairs);
    this.#dom.host.removeEventListener("click", this.#selectRow);
    this.#dom.dialog.removeEventListener("keydown", this.#handleKeydown);
  }

  open(): void {
    const status = this.#state.status;
    if (!status || this.#state.worldMap) return;
    this.#selectedIndex = 0;
    this.#render();
    if (!this.#dom.dialog.open) this.#dom.dialog.showModal();
    this.#window.requestAnimationFrame(() => this.#focusSelected());
  }

  close(): void {
    if (this.#dom.dialog.open) this.#dom.dialog.close();
  }

  localize(): void {
    if (this.#dom.dialog.open) {
      this.#render();
      this.#focusSelected();
    } else this.#renderStairsToggle();
  }

  readonly #close = (): void => this.close();

  readonly #toggleStairs = (): void => {
    this.#includeStairs = !this.#includeStairs;
    this.#render();
    this.#focusSelected();
  };

  readonly #selectRow = (event: MouseEvent): void => {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const row = target.closest<HTMLButtonElement>("[data-object-list-index]");
    if (!row) return;
    const index = Number(row.dataset.objectListIndex);
    if (Number.isInteger(index)) this.#setSelectedIndex(index);
  };

  readonly #handleKeydown = (event: KeyboardEvent): void => {
    const key = event.key.toLowerCase();
    if (event.key === "Escape" || key === "q") {
      event.preventDefault();
      event.stopPropagation();
      this.close();
      return;
    }
    if (key === "s") {
      event.preventDefault();
      event.stopPropagation();
      this.#toggleStairs();
      return;
    }
    if (event.key === "J" || event.key === "(" || event.key === "`") {
      event.preventDefault();
      event.stopPropagation();
      const destination = this.#entries[this.#selectedIndex]?.position;
      if (!destination) return;
      this.close();
      this.#onTravel(destination);
      return;
    }

    const movement = objectListMovementForKey(event.key);
    if (movement !== undefined) {
      event.preventDefault();
      event.stopPropagation();
      this.#setSelectedIndex(
        movement === "start"
          ? 0
          : movement === "end"
            ? this.#entries.length - 1
            : this.#selectedIndex + movement,
      );
      return;
    }

    if (event.key.length !== 1 || event.ctrlKey || event.altKey || event.metaKey) return;
    const match = nextEntryStartingWith(this.#entries, this.#selectedIndex, event.key);
    if (match === undefined) return;
    event.preventDefault();
    event.stopPropagation();
    this.#setSelectedIndex(match);
  };

  #render(): void {
    const status = this.#state.status;
    if (!status) return;
    const selectedId = this.#entries[this.#selectedIndex]?.id;
    this.#entries = buildObjectListEntries({
      playerPosition: status.player.position,
      floorId: status.floorId,
      cells: this.#state.cells.values(),
      shops: status.shops,
      homes: status.homes,
      taskServices: status.taskServices,
      items: status.items,
      includeStairs: this.#includeStairs,
      visibilityAt: (position) => this.#state.cellVisibility.get(positionKey(position)),
      glyphFor: (contentId) => this.#state.contentGlyphs.get(contentId),
      localize: (nameKey) => this.#localization.format(nameKey),
      contentName: this.#contentName,
      visibleItemName: this.#visibleItemName,
    });
    this.#selectedIndex = Math.max(
      0,
      selectedId === undefined
        ? Math.min(this.#selectedIndex, this.#entries.length - 1)
        : Math.max(0, this.#entries.findIndex((entry) => entry.id === selectedId)),
    );
    this.#renderStairsToggle();
    this.#dom.host.replaceChildren(
      this.#renderCategory("interesting", "object-list-category-interesting"),
      this.#renderCategory("items", "object-list-category-items"),
    );
  }

  #renderCategory(category: ObjectListCategory, titleKey: string): HTMLElement {
    const section = this.#document.createElement("section");
    section.className = "object-list-section";
    const title = this.#document.createElement("h3");
    title.textContent = this.#localization.format(titleKey);
    section.append(title);

    const categoryEntries = this.#entries
      .map((entry, index) => ({ entry, index }))
      .filter(({ entry }) => entry.category === category);
    if (categoryEntries.length === 0) {
      const empty = this.#document.createElement("p");
      empty.className = "object-list-empty";
      empty.textContent = this.#localization.format("object-list-category-empty");
      section.append(empty);
      return section;
    }

    const list = this.#document.createElement("ol");
    list.className = "object-list-entries";
    for (const { entry, index } of categoryEntries) {
      const item = this.#document.createElement("li");
      const button = this.#document.createElement("button");
      button.type = "button";
      button.className = "object-list-row";
      button.dataset.objectListIndex = String(index);
      button.tabIndex = index === this.#selectedIndex ? 0 : -1;
      button.dataset.selected = String(index === this.#selectedIndex);

      const glyph = this.#document.createElement("span");
      glyph.className = "object-list-glyph";
      glyph.textContent = entry.glyph;
      glyph.setAttribute("aria-hidden", "true");
      const name = this.#document.createElement("span");
      name.className = "object-list-name";
      name.textContent = entry.quantity && entry.quantity > 1
        ? `${entry.name} ×${entry.quantity}`
        : entry.name;
      const position = this.#document.createElement("span");
      position.className = "object-list-position";
      position.textContent = this.#localization.format("object-list-position", {
        vertical: this.#localization.format(
          `nearby-direction-${entry.offsetY > 0 ? "s" : "n"}`,
        ),
        verticalDistance: Math.abs(entry.offsetY),
        horizontal: this.#localization.format(
          `nearby-direction-${entry.offsetX > 0 ? "e" : "w"}`,
        ),
        horizontalDistance: Math.abs(entry.offsetX),
        distance: entry.distance,
      });
      button.append(glyph, name, position);
      item.append(button);
      list.append(item);
    }
    section.append(list);
    return section;
  }

  #renderStairsToggle(): void {
    this.#dom.stairsToggle.textContent = this.#localization.format(
      this.#includeStairs ? "object-list-hide-stairs" : "object-list-show-stairs",
    );
    this.#dom.stairsToggle.setAttribute("aria-pressed", String(this.#includeStairs));
  }

  #setSelectedIndex(index: number): void {
    if (this.#entries.length === 0) return;
    this.#selectedIndex = Math.max(0, Math.min(index, this.#entries.length - 1));
    this.#focusSelected();
  }

  #focusSelected(): void {
    const rows = this.#dom.host.querySelectorAll<HTMLButtonElement>(
      "[data-object-list-index]",
    );
    for (const row of rows) {
      const selected = Number(row.dataset.objectListIndex) === this.#selectedIndex;
      row.tabIndex = selected ? 0 : -1;
      row.dataset.selected = String(selected);
      if (selected) {
        row.focus({ preventScroll: true });
        row.scrollIntoView({ block: "nearest" });
      }
    }
    if (rows.length === 0) this.#dom.stairsToggle.focus({ preventScroll: true });
  }
}

export function buildObjectListEntries(options: ObjectListProjection): ObjectListEntry[] {
  const interesting = new Map<string, ObjectListEntry>();
  const addFacility = (
    kind: "shop" | "home" | "task-service",
    facility: ShopDto | HomeDto | TaskServiceDto,
  ): void => {
    if (!isExplored(options.visibilityAt(facility.entrancePosition))) return;
    interesting.set(positionKey(facility.entrancePosition), {
      id: `${kind}:${facility.id}`,
      category: "interesting",
      position: facility.entrancePosition,
      name: options.localize(facility.nameKey),
      glyph: options.glyphFor(facility.entranceTerrainId) ?? "?",
      distance: gridDistance(options.playerPosition, facility.entrancePosition),
      offsetX: facility.entrancePosition.x - options.playerPosition.x,
      offsetY: facility.entrancePosition.y - options.playerPosition.y,
    });
  };
  for (const shop of options.shops) addFacility("shop", shop);
  for (const home of options.homes) addFacility("home", home);
  for (const service of options.taskServices) addFacility("task-service", service);

  for (const cell of options.cells) {
    if (
      interesting.has(positionKey(cell.position)) ||
      !isExplored(options.visibilityAt(cell.position))
    ) {
      continue;
    }
    const glyph = options.glyphFor(cell.terrainId) ?? "?";
    const stairs = isStairs(cell.terrainId, glyph);
    const surfaceDungeonEntrance = options.floorId.endsWith(".surface") && stairs;
    if (
      !isTaskOrDungeonEntrance(cell.terrainId) &&
      !surfaceDungeonEntrance &&
      !(options.includeStairs && stairs)
    ) {
      continue;
    }
    interesting.set(positionKey(cell.position), {
      id: `terrain:${cell.position.x},${cell.position.y}`,
      category: "interesting",
      position: cell.position,
      name: options.contentName(cell.terrainId),
      glyph,
      distance: gridDistance(options.playerPosition, cell.position),
      offsetX: cell.position.x - options.playerPosition.x,
      offsetY: cell.position.y - options.playerPosition.y,
    });
  }

  const items = options.items.map<ObjectListEntry>((item) => ({
    id: `item:${item.id}`,
    category: "items",
    position: item.position,
    name: options.visibleItemName(item.displayNameKey, item.kindId),
    glyph: options.glyphFor(item.kindId) ?? "?",
    distance: gridDistance(options.playerPosition, item.position),
    offsetX: item.position.x - options.playerPosition.x,
    offsetY: item.position.y - options.playerPosition.y,
    quantity: item.quantity,
  }));
  const compare = (left: ObjectListEntry, right: ObjectListEntry): number =>
    left.distance - right.distance ||
    left.position.y - right.position.y ||
    left.position.x - right.position.x ||
    left.name.localeCompare(right.name) ||
    left.id.localeCompare(right.id);
  return [...interesting.values()].sort(compare).concat(items.sort(compare));
}

export function objectListMovementForKey(
  key: string,
): number | "start" | "end" | undefined {
  if (key === "ArrowUp" || key === "ArrowLeft") return -1;
  if (key === "ArrowDown" || key === "ArrowRight") return 1;
  if (key === "PageUp") return -10;
  if (key === "PageDown") return 10;
  if (key === "Home") return "start";
  if (key === "End") return "end";
  return undefined;
}

export function nextEntryStartingWith(
  entries: readonly Pick<ObjectListEntry, "name">[],
  selectedIndex: number,
  key: string,
): number | undefined {
  const prefix = key.toLocaleLowerCase();
  if (!prefix.trim() || entries.length === 0) return undefined;
  for (let offset = 1; offset <= entries.length; offset += 1) {
    const index = (Math.max(0, selectedIndex) + offset) % entries.length;
    if (entries[index]?.name.trim().toLocaleLowerCase().startsWith(prefix)) return index;
  }
  return undefined;
}

function isTaskOrDungeonEntrance(terrainId: string): boolean {
  const name = terrainId.split(".").at(-1) ?? terrainId;
  return /(?:^|-)(?:rift|entry)(?:$|-available$)/.test(name);
}

function isStairs(terrainId: string, glyph: string): boolean {
  const name = terrainId.split(".").at(-1) ?? terrainId;
  return glyph === "<" || glyph === ">" || /(?:^|-)(?:stairs|shaft)(?:-|$)/.test(name);
}

function isExplored(visibility: VisibilityState | undefined): boolean {
  return visibility === "visible" || visibility === "remembered";
}

function gridDistance(from: Position, to: Position): number {
  return Math.max(Math.abs(to.x - from.x), Math.abs(to.y - from.y));
}

function positionKey(position: Position): string {
  return `${position.x},${position.y}`;
}

function createObjectListDom(document: Document): ObjectListDom {
  const element = <T extends HTMLElement>(id: string): T => {
    const value = document.getElementById(id);
    if (!value) throw new Error(`Missing #${id}`);
    return value as T;
  };
  return {
    dialog: element<HTMLDialogElement>("object-list-dialog"),
    close: element<HTMLButtonElement>("object-list-close"),
    stairsToggle: element<HTMLButtonElement>("object-list-stairs-toggle"),
    host: element<HTMLElement>("object-list-host"),
  };
}
