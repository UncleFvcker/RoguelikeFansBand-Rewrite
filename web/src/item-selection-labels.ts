// SPDX-License-Identifier: MPL-2.0
// RFB master@a0d92b6378d148c5262cc236b8fa6ed2ca06a54c:
// obj.c::obj_label / obj_confirm_choice and inv.c::inv_calculate_labels.

function inscriptionLabel(inscription: string, command: string | undefined): string | undefined {
  for (let index = inscription.indexOf("@"); index >= 0; index = inscription.indexOf("@", index + 1)) {
    const next = inscription[index + 1];
    if (command && next === command) {
      const label = inscription[index + 2];
      if (label && /^[a-zA-Z0-9]$/.test(label)) return label;
    } else if (next && /^[0-9]$/.test(next)) return next;
  }
  return undefined;
}

/** Called for one visible page, containing at most 26 candidates. */
export function itemSelectionLabels(
  inscriptions: readonly (string | null | undefined)[],
  command?: string,
  ignoreInscriptions = false,
): string[] {
  const labels = inscriptions.map((_, index) => String.fromCharCode(97 + index));
  if (ignoreInscriptions) return labels;
  inscriptions.forEach((inscription, index) => {
    const label = inscriptionLabel(inscription ?? "", command);
    if (!label) return;
    const displaced = labels.indexOf(label);
    if (displaced >= 0) labels[displaced] = "";
    labels[index] = label;
  });
  for (let index = 0; index < labels.length; index++) {
    if (labels[index]) continue;
    for (const label of "abcdefghijklmnopqrstuvwxyz") {
      if (!labels.includes(label)) { labels[index] = label; break; }
    }
  }
  return labels;
}

/** Preserve repeated guards and source scanning of grouped !sdk and !?123 syntax. */
export function itemSelectionConfirmations(inscription: string | null | undefined, command?: string): number {
  if (!inscription || !command) return 0;
  let count = 0;
  for (let index = inscription.indexOf("!"); index >= 0; index = inscription.indexOf("!", index + 1)) {
    for (;;) {
      const char = inscription[++index];
      if (!char) return count;
      if (char === command || char === "*") count++;
      else if (!/^[a-zA-Z]$/.test(char)) {
        if (char === "!") { index--; break; }
        if (char !== "?") break;
        index++;
        while (index < inscription.length && /^[0-9]$/.test(inscription[index]!)) index++;
        if (index === inscription.length) return count;
        if (inscription[index] === "!") { index--; break; }
        if (/^[a-zA-Z]$/.test(inscription[index]!)) index--;
      }
    }
  }
  return count;
}
