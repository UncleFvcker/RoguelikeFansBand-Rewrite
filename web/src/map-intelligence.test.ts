// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.
import assert from "node:assert/strict";
import test from "node:test";
import { AppState } from "./app-state.ts";
import { filterMonsterRecall, knownMapGlyphs, monsterRecallAt, MapIntelligencePanel } from "./map-intelligence.ts";
import { commandShortcut } from "./command-shortcuts.ts";

test("look recall uses the projected apparent identity and cannot reveal fuzzy or hallucinated monsters", () => {
  const state = new AppState();
  const position = { x: 3, y: 2 };
  const apparent = { kindId: "sheep", nameKey: "sheep", glyph: "q" };
  state.status = { player: { statuses: [], monsterRecall: [apparent, { kindId: "dragon" }] },
    entities: [{ id: "one", kindId: "sheep", position }] };
  assert.equal(monsterRecallAt(state, position), apparent);
  assert.equal(monsterRecallAt(state, { x: 0, y: 0 }), undefined);
  state.status.entities[0].kindId = "core.actor.fuzzy-monster";
  assert.equal(monsterRecallAt(state, position), undefined);
  state.status.entities[0].kindId = "sheep";
  state.status.player.statuses = [{ kindId: "rfb.status.hallucination" }];
  assert.equal(monsterRecallAt(state, position), undefined);
  state.status.player.statuses = [];
  state.status.entities = [];
  assert.equal(monsterRecallAt(state, position), undefined);
});

test("monster recall opens expanded knowledge and r or Escape returns to the unchanged look cursor", () => {
  class Element extends EventTarget {
    children = []; open = false; textContent = "";
    constructor(tag) { super(); this.tag = tag; }
    setAttribute() {}
    append(...nodes) { this.children.push(...nodes); }
    prepend(...nodes) { this.children.unshift(...nodes); }
    replaceChildren(...nodes) { this.children = nodes; }
    querySelector(tag) { return this.children.find(node => node.tag === tag); }
    focus() {}
    showModal() { this.open = true; }
    close() { this.open = false; }
  }
  const document = { body: new Element("body"), createElement: tag => new Element(tag) };
  const state = new AppState();
  state.mode = "playing";
  state.paintVisual = () => {};
  const cursor = { x: 3, y: 2 };
  state.targeting = { cursor }; state.targetingIntent = { type: "look" };
  const monster = { kindId: "sheep", nameKey: "sheep", glyph: "q" };
  state.status = { player: { statuses: [], monsterRecall: [monster] }, entities: [{ kindId: "sheep", position: cursor }] };
  const panel = new MapIntelligencePanel(state, { format: key => key }, document, () => "original", id => id, () => {}, id => id);
  panel.openMonsterRecall(cursor);
  const dialog = document.body.children[0], body = dialog.children[1];
  assert.equal(dialog.open, true);
  assert.equal(body.children[1].open, true);
  assert.equal(body.children[1].children[1].textContent, "intel-unresearched");
  for (const key of ["r", "Escape"]) {
    panel.openMonsterRecall(cursor);
    const event = new Event("keydown", { cancelable: true }); Object.assign(event, { key });
    dialog.dispatchEvent(event);
    assert.equal(dialog.open, false); assert.equal(event.defaultPrevented, true);
    assert.equal(state.targeting.cursor, cursor); assert.equal(state.targetingIntent.type, "look");
  }
  monster.knowledge = { descriptionKey: "known-description", maxHp: 12, armorClass: 3, speed: 110,
    resistances: [], statusImmunities: [], meleeRoutine: { blows: [] }, abilityIds: ["known-ability"] };
  panel.openMonsterRecall(cursor);
  assert.match(body.children[1].children[1].textContent, /known-description.*HP 12.*AC 3/);
  assert.match(body.children[1].children.at(-1).textContent, /known-ability/);
  state.status = { ...state.status }; panel.reconcileStatus(); assert.equal(dialog.open, false);
});

test("overview hides unexplored terrain and uses only projected monster identities", () => {
  const state = new AppState();
  state.replaceCells([0,1,2].map(x => ({ position: {x, y: 0}, terrainId: "wall" })));
  state.contentGlyphs.set("wall", "#");
  state.replaceVisualCells(["visible", "remembered", "hidden"].map((visibility, x) => ({position: {x,y:0},visibility})));
  state.status = { player: { kindId: "demo.actor.player", visualCatalog: [], position: { x: 0,y:0 } }, entities: [{position:{x:3,y:0},glyph:"D",kindId:"core.actor.fuzzy-monster"}], items: [] };
  const glyphs = knownMapGlyphs(state);
  assert.deepEqual([...glyphs], [["0,0","@"],["1,0","#"],["3,0","D"]]);
  assert.equal(glyphs.has("2,0"), false);
});

test("symbol filters retain case, identity and researched detail boundaries", () => {
  const monsters = [{kindId:"dragon",glyph:"D",nameKey:"Red Dragon",unique:false,rideable:true},
    {kindId:"wyrm",glyph:"d",nameKey:"Wyrm",unique:true,rideable:false}];
  for (const [query, mode, expected] of [["D","glyph","dragon"],["wyrM","name","wyrm"],["","unique","wyrm"],["","normal","dragon"],["","rideable","dragon"]]) {
    assert.deepEqual(filterMonsterRecall(monsters, query, mode, key => key).map(m => m.kindId), [expected]);
  }
  assert.equal(filterMonsterRecall(monsters, "", "all", key => key).length, 2);
  assert.equal(monsters[0].knowledge, undefined);
});

test("inquiry shortcuts preserve Rogue movement and literal ring swap", () => {
  for (const preset of ["original", "roguelike"]) {
    for (const [key, command, ctrlKey=false] of [["M","map-overview"],["[","monster-list"],["/","symbol-query"],["v","map-center",true],["f","floor-feeling",true]]) {
      assert.equal(commandShortcut({key,ctrlKey}, preset), command);
      assert.equal(commandShortcut({key,ctrlKey,altKey:true}, preset), undefined);
    }
  }
  assert.equal(commandShortcut({key:"L"},"original"),"map-locate");
  assert.equal(commandShortcut({key:"L"},"roguelike"),undefined);
  assert.equal(commandShortcut({key:"W"},"roguelike"),"map-locate");
  assert.equal(commandShortcut({key:"W"},"original"),"swap-rings");
});
