// SPDX-License-Identifier: MPL-2.0

export function formatManaRecovery(per65536: number): string {
  if (per65536 === 0) return "0";
  const amount = Math.abs(per65536) / 65536;
  const sign = per65536 > 0 ? "+" : "−";
  return `${sign}${amount < 0.001 ? "<0.001" : amount.toFixed(3)}`;
}
