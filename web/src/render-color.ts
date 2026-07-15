// SPDX-License-Identifier: MPL-2.0

export function ensureContrast(foreground: number, background: number, minimumRatio = 3): number {
  if (contrastRatio(foreground, background) >= minimumRatio) return foreground;
  const blackRatio = contrastRatio(0x000000, background);
  const whiteRatio = contrastRatio(0xffffff, background);
  return whiteRatio >= blackRatio ? 0xffffff : 0x000000;
}

export function contrastRatio(left: number, right: number): number {
  const lighter = Math.max(relativeLuminance(left), relativeLuminance(right));
  const darker = Math.min(relativeLuminance(left), relativeLuminance(right));
  return (lighter + 0.05) / (darker + 0.05);
}

function relativeLuminance(color: number): number {
  const red = linearChannel((color >> 16) & 0xff);
  const green = linearChannel((color >> 8) & 0xff);
  const blue = linearChannel(color & 0xff);
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
}

function linearChannel(channel: number): number {
  const value = channel / 255;
  return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
}
