// SPDX-License-Identifier: MPL-2.0

import type { GameCommand } from "./protocol";

export const REST_UNTIL_RECOVERED_TURNS = 9_999;

export type RestCommand = Extract<GameCommand, { type: "rest" | "rest-for-turns" | "rest-until-resources" }>;

export function parseRestInput(input: string): RestCommand | undefined {
  const value = input.trim();
  if (value === "&") return { type: "rest", turns: REST_UNTIL_RECOVERED_TURNS };
  if (value === "*") return { type: "rest-until-resources", turns: REST_UNTIL_RECOVERED_TURNS };
  if (/^\d+$/.test(value) && Number(value) > 0) {
    return { type: "rest-for-turns", turns: Math.min(Number(value), REST_UNTIL_RECOVERED_TURNS) };
  }
  return undefined;
}
