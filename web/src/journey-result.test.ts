// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import {
  JourneyResult,
  selectJourneyResultEvent,
  selectJourneyResultKind,
} from "./journey-result.ts";

function state(overrides = {}) {
  return {
    turn: 12,
    player: { isDead: false },
    campaign: { status: "active", score: 0n, conqueredDungeons: 0, completedTasks: 0 },
    ...overrides,
  };
}

test("journey results distinguish play, victory return, retirement, and death", () => {
  assert.equal(selectJourneyResultKind(state()), undefined);
  assert.equal(
    selectJourneyResultKind(state({ campaign: { status: "victorious" } })),
    "victory-return",
  );
  assert.equal(
    selectJourneyResultKind(state({ campaign: { status: "retired" } })),
    "retired",
  );
  assert.equal(
    selectJourneyResultKind(
      state({ player: { isDead: true }, campaign: { status: "victorious" } }),
    ),
    "death",
  );
});

test("journey results select only the event relevant to the visible outcome", () => {
  const events = [
    { kind: "combat.entity-death", messageKey: "combat-entity-death", args: {} },
    { kind: "campaign.victorious", messageKey: "campaign-victorious", args: {} },
    { kind: "campaign.retired", messageKey: "campaign-retired", args: {} },
    { kind: "combat.player-death", messageKey: "combat-player-death", args: {} },
  ];
  assert.equal(selectJourneyResultEvent("death", events)?.kind, "combat.player-death");
  assert.equal(
    selectJourneyResultEvent("victory-return", events)?.kind,
    "campaign.victorious",
  );
  assert.equal(selectJourneyResultEvent("retired", events)?.kind, "campaign.retired");
});

function resultDom() {
  const documentElement = { dataset: {} };
  const ownerDocument = { documentElement };
  const element = () => {
    const listeners = new Map();
    return {
      ownerDocument,
      dataset: {},
      hidden: false,
      disabled: false,
      textContent: "",
      title: "",
      focused: false,
      addEventListener(type, listener) {
        listeners.set(type, listener);
      },
      removeEventListener(type) {
        listeners.delete(type);
      },
      click() {
        listeners.get("click")?.();
      },
      focus() {
        this.focused = true;
      },
      replaceChildren() {
        this.textContent = "";
      },
    };
  };
  const dom = {
    documentElement,
    journeyResult: element(),
    resultKind: element(),
    resultTitle: element(),
    resultDetail: element(),
    resultBuild: element(),
    resultSeed: element(),
    resultTurn: element(),
    resultScore: element(),
    resultDungeons: element(),
    resultTasks: element(),
    resultContinue: element(),
    resultRestart: element(),
    resultNewGame: element(),
    resultLoad: element(),
    resultMenu: element(),
    resultExit: element(),
    resultError: element(),
  };
  dom.journeyResult.hidden = true;
  return dom;
}

function resultState(overrides = {}) {
  return state({
    player: {
      isDead: false,
      build: { buildNameKey: "build-demo-warrior-name" },
    },
    campaign: {
      status: "active",
      score: 0n,
      conqueredDungeons: 0,
      completedTasks: 0,
    },
    ...overrides,
  });
}

function controller(dom, options = {}) {
  const localization = {
    format(key, args) {
      return args ? `${key}:${JSON.stringify(args)}` : key;
    },
    localizeDocument() {},
  };
  return new JourneyResult({
    dom,
    localization,
    formatEvent: (event) => `event:${event.kind}`,
    currentSeed: () => options.seed,
    canRestart: () => options.canRestart ?? false,
    onRestart: options.onRestart ?? (async () => {}),
    onNewGame: options.onNewGame ?? (() => {}),
    onLoad: options.onLoad ?? (() => {}),
    onMenu: options.onMenu ?? (() => {}),
    onExit: options.onExit ?? (async () => {}),
  });
}

test("victory result acknowledges once before retirement becomes terminal", () => {
  const dom = resultDom();
  const result = controller(dom, { seed: "42", canRestart: true });
  result.install();
  result.renderSnapshot(
    resultState({ campaign: { status: "victorious", score: 60_000n, conqueredDungeons: 1, completedTasks: 0 } }),
  );
  assert.equal(dom.journeyResult.hidden, false);
  assert.equal(dom.journeyResult.dataset.resultKind, "victory-return");
  assert.equal(dom.resultSeed.textContent, "42");
  assert.equal(dom.resultScore.textContent, "60000");
  dom.resultContinue.click();
  assert.equal(dom.journeyResult.hidden, true);
  assert.equal(dom.documentElement.dataset.journeyResult, "victory-return-acknowledged");

  result.renderUpdate(
    resultState({ campaign: { status: "victorious", score: 60_000n, conqueredDungeons: 1, completedTasks: 0 }, events: [] }),
  );
  assert.equal(dom.journeyResult.hidden, true);
  result.renderUpdate(
    resultState({ campaign: { status: "retired", score: 60_000n, conqueredDungeons: 1, completedTasks: 0 }, events: [] }),
  );
  assert.equal(dom.journeyResult.hidden, false);
  assert.equal(dom.journeyResult.dataset.resultKind, "retired");
});

test("loaded death disables unknown-seed restart but keeps recovery routes", () => {
  const dom = resultDom();
  let route = "";
  const result = controller(dom, {
    onNewGame: () => {
      route = "new-game";
    },
  });
  result.install();
  result.renderSnapshot(resultState({ player: { isDead: true, build: undefined } }));
  assert.equal(dom.resultRestart.disabled, true);
  assert.equal(dom.resultSeed.textContent, "result-seed-loaded-save");
  assert.equal(dom.resultNewGame.focused, true);
  dom.resultNewGame.click();
  assert.equal(route, "new-game");
  assert.equal(dom.journeyResult.hidden, true);
});
