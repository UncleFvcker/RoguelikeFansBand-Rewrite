// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.
import assert from "node:assert/strict";
import test from "node:test";
import { AppState } from "./app-state.ts";
import { filterMonsterRecall, knownMapGlyphs } from "./map-intelligence.ts";
import { commandShortcut } from "./command-shortcuts.ts";

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
