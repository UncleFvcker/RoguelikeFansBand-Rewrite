// SPDX-License-Identifier: MPL-2.0
import type { AppState } from "./app-state";
import type { Position } from "./protocol";
import { MAP_CELL_SIZE } from "./camera.ts";

const key = (p: Position) => `${p.x},${p.y}`;

// A visual centerline, not a simulation of spell area, actor interception or hit chance.
export function knownAimPath(origin: Position, target: Position, range: number,
  known: (p: Position) => boolean, passage: (p: Position) => boolean): Position[] {
  const result: Position[] = [];
  let { x, y } = origin;
  const dx = Math.abs(target.x - x), dy = Math.abs(target.y - y);
  const sx = Math.sign(target.x - x), sy = Math.sign(target.y - y);
  let error = dx - dy;
  while ((x !== target.x || y !== target.y) && result.length < range) {
    const twice = 2 * error;
    if (twice > -dy) { error -= dy; x += sx; }
    if (twice < dx) { error += dx; y += sy; }
    const p = { x, y };
    if (!known(p)) break;
    result.push(p);
    if (!passage(p)) break;
  }
  return result;
}

export class MapDisplay {
  #overlay: SVGSVGElement | undefined;
  #marks: SVGGElement | undefined;
  #playerMark: SVGRectElement | undefined;
  #renderedZoom = 1;
  #previous: { floor: string; position: string; covered: boolean; turn: number } | undefined;

  reset(): void {
    this.#previous = undefined;
    this.#overlay?.remove();
    this.#overlay = undefined;
    this.#marks = undefined;
    this.#playerMark = undefined;
  }

  render(host: HTMLElement, state: AppState, zoom: number): boolean {
    this.#marks = undefined;
    this.#playerMark = undefined;
    const status = state.status;
    if (!status || state.mode !== "playing" || state.worldMap) {
      this.#previous = undefined;
      this.#overlay?.replaceChildren();
      return false;
    }
    if (![state.display.highlightPlayer, state.display.targetPath, state.display.unsafeGrids, state.display.alertTrapDetect].some(Boolean)) {
      this.#previous = undefined;
      this.#overlay?.replaceChildren();
      return false;
    }
    const coverage = new Set(status.player.trapDetectedGrids.map(key));
    const position = key(status.player.position), covered = coverage.has(position);
    const previous = this.#previous;
    const leftCoverage = state.display.alertTrapDetect && !("cells" in status) && !!previous &&
      previous.floor === status.floorId && status.turn >= previous.turn &&
      previous.position !== position && previous.covered && !covered;
    this.#previous = { floor: status.floorId, position, covered, turn: status.turn };
    const ns = "http://www.w3.org/2000/svg";
    if (!this.#overlay) {
      this.#overlay = host.ownerDocument.createElementNS(ns, "svg");
      this.#overlay.classList.add("map-display-overlay");
      this.#overlay.setAttribute("aria-hidden", "true");
      host.append(this.#overlay);
    }
    const size = MAP_CELL_SIZE * zoom;
    this.#renderedZoom = zoom;
    const marks: SVGRectElement[] = [];
    const rect = (p: Position, color: string, inset: number, opacity: number) => {
      const mark = host.ownerDocument.createElementNS(ns, "rect");
      mark.setAttribute("x", String((p.x + inset) * size));
      mark.setAttribute("y", String((p.y + inset) * size));
      mark.setAttribute("width", String(size * (1 - 2 * inset)));
      mark.setAttribute("height", String(size * (1 - 2 * inset)));
      mark.setAttribute("fill", "none");
      mark.setAttribute("stroke", color);
      mark.setAttribute("stroke-width", "2");
      mark.setAttribute("opacity", String(opacity));
      marks.push(mark);
      return mark;
    };
    const known = (p: Position) => {
      const visibility = state.cellVisibility.get(key(p));
      return visibility === "visible" || visibility === "remembered";
    };
    if (state.display.unsafeGrids) {
      for (const cell of state.cells.values()) {
        if (known(cell.position) && !coverage.has(key(cell.position))) rect(cell.position, state.visuals.theme.uncovered, 0.42, 0.65);
      }
    }
    const targeting = state.targeting;
    if (state.display.targetPath && targeting && !["look", "local-travel"].includes(state.targetingIntent?.type ?? "")) {
      for (const p of knownAimPath(targeting.origin, targeting.cursor, targeting.spec.range, known,
        p => state.cellAt(p)?.knownProjectilePassage === true)) rect(p, state.visuals.theme.path, 0.3, 0.9);
    }
    if (state.display.highlightPlayer) this.#playerMark = rect(status.player.position, state.visuals.theme.player, 0.08, 1);
    this.#marks = host.ownerDocument.createElementNS(ns, "g");
    this.#marks.replaceChildren(...marks);
    this.#overlay.replaceChildren(this.#marks);
    this.updateCamera(host, zoom);
    return leftCoverage;
  }

  // Animation frames move existing marks; do not rebuild paths/coverage or emit alerts.
  updateCamera(host: HTMLElement, zoom: number): void {
    const x = Number(host.dataset.cameraX ?? 0), y = Number(host.dataset.cameraY ?? 0);
    this.#marks?.setAttribute("transform", `translate(${x} ${y}) scale(${zoom / this.#renderedZoom})`);
    if (this.#playerMark && host.dataset.playerDisplayX !== undefined && host.dataset.playerDisplayY !== undefined) {
      this.#playerMark.setAttribute("x", String((Number(host.dataset.playerDisplayX) + 0.08) * MAP_CELL_SIZE * this.#renderedZoom));
      this.#playerMark.setAttribute("y", String((Number(host.dataset.playerDisplayY) + 0.08) * MAP_CELL_SIZE * this.#renderedZoom));
    }
  }
}
