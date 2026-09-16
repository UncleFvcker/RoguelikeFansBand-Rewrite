// SPDX-License-Identifier: MPL-2.0
import type { RenderCell } from "./renderer-backend";
import type { MapTheme } from "./visual-preferences";

interface Overlay { color: number; alpha: number }
const rgb = (color: string) => Number.parseInt(color.slice(1), 16);

function overlays(cell: RenderCell, theme: MapTheme) {
  const visible = cell.visibility === "visible";
  const intensity = Math.max(0, Math.min(1, cell.light.intensity));
  return {
    fog: {
      color: rgb(cell.visibility === "remembered" ? theme.memoryColor : theme.hiddenColor),
      alpha: visible ? 0 : cell.visibility === "remembered" ? theme.memoryOpacity : 1,
    },
    tint: { color: cell.light.color, alpha: visible ? Math.max(0, intensity - 0.5) * theme.lightTintOpacity : 0 },
    darkness: { color: 0, alpha: visible ? (1 - intensity) * theme.darknessOpacity : 0 },
  };
}

function blend(from: Overlay, to: Overlay, progress: number): Overlay {
  if (progress === 0) return from;
  if (progress === 1) return to;
  const fromAlpha = from.alpha * (1 - progress), toAlpha = to.alpha * progress;
  const alpha = fromAlpha + toAlpha;
  if (alpha === 0) return { color: to.color, alpha: 0 };
  // Premultiplied colors keep a fading light's hue instead of darkening it twice.
  const channel = (shift: number) => Math.round(
    (((from.color >> shift) & 255) * fromAlpha + ((to.color >> shift) & 255) * toAlpha) / alpha,
  );
  return { color: (channel(16) << 16) | (channel(8) << 8) | channel(0), alpha };
}

/** Display-only blending of two authoritative cell projections, never a new FOV calculation. */
export function cellAppearance(from: RenderCell, to: RenderCell, theme: MapTheme, progress: number) {
  const before = overlays(from, theme), after = overlays(to, theme);
  const revealing = from.visibility !== "visible" && to.visibility === "visible";
  return {
    fog: blend(before.fog, after.fog, progress),
    tint: blend(before.tint, after.tint, progress),
    darkness: blend(before.darkness, after.darkness, progress),
    // Only current occupants are rendered; departed/unseen actors are never retained.
    actorAlpha: revealing ? progress : 1,
    itemAlpha: revealing && !from.itemKindId ? progress : 1,
  };
}
