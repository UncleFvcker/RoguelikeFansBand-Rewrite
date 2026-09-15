// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Node's built-in TypeScript runner.
import assert from "node:assert/strict";
import test from "node:test";
import { confirmCharacterEnd } from "./high-scores.ts";
import { commandShortcut } from "./command-shortcuts.ts";
import { isMacroCommand } from "./command-recording.ts";
import { selectJourneyResultKind } from "./journey-result.ts";

test("ending requires both confirmations and never accepts an empty or cancelled prompt", () => {
  let prompted = false;
  assert.equal(confirmCharacterEnd(false, key => key, () => false, () => { prompted = true; return "@"; }), false);
  assert.equal(prompted, false);
  for (const answer of [null, "", "yes", " @", "@@"]) assert.equal(confirmCharacterEnd(false, key => key, () => true, () => answer), false);
  let question;
  assert.equal(confirmCharacterEnd(true, key => key, text => { question = text; return true; }, () => "@"), true);
  assert.equal(question, "end-retire-confirm");
});

test("RFB ending keys route to confirmation and the final command is excluded from macros", () => {
  const event = (key, ctrlKey = false) => ({ key, ctrlKey, altKey: false, metaKey: false, shiftKey: !ctrlKey && key === "Q" });
  for (const preset of ["original", "roguelike"]) {
    assert.equal(commandShortcut(event("Q"), preset), "end-character");
    assert.equal(commandShortcut(event("c", true), preset), "end-character");
  }
  assert.equal(commandShortcut(event("k", true), "original"), "end-character");
  assert.equal(isMacroCommand({ type: "end-character" }), false);
  assert.equal(selectJourneyResultKind({ player: { isDead: false }, campaign: { status: "abandoned" } }), "abandoned");
});
