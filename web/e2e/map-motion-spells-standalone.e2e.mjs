// SPDX-License-Identifier: MPL-2.0
// Focused rendering acceptance with a new character and isolated native saves.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdir, mkdtemp, copyFile, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { connectKeyboard } from "./character-creation-layout.e2e.mjs";
import { selectCreationRace, selectCreationBuild } from "./character-creation.e2e.mjs";

const root = path.resolve(import.meta.dirname, "../..");
const output = path.join(root, "test-results/map-motion-spells");
await mkdir(output, { recursive: true });
const install = await mkdtemp(path.join(output, "game-"));
const profile = await mkdtemp(path.join(output, "webview-"));
const exe = path.join(install, "rfb-tauri.exe");
await copyFile(process.env.RFB_STANDALONE_EXE ?? path.join(root, "target/debug/rfb-tauri.exe"), exe);
const child = spawn(exe, ["--edge-webview-switches=--remote-debugging-port=0"], {
  cwd: install, windowsHide: true, stdio: "ignore", env: { ...process.env, WEBVIEW2_USER_DATA_FOLDER: profile },
});
let keyboard;
const report = { layouts: [], movement: [] };
try {
  for (let attempt = 0; attempt < 200; attempt++) {
    assert.equal(child.exitCode, null, "standalone startup");
    try {
      const port = (await readFile(path.join(profile, "EBWebView/DevToolsActivePort"), "utf8")).split("\n")[0];
      const pages = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
      if (pages.some(page => page.type === "page" && page.url.includes("tauri.localhost"))) break;
    } catch (error) { if (error.code !== "ENOENT" && error.cause?.code !== "ECONNREFUSED") throw error; }
    await delay(100);
  }
  keyboard = await connectKeyboard(profile);
  const driver = { execute: (body, args = []) => keyboard.evaluate(`(function(){${body}}).apply(null,${JSON.stringify(args)})`) };
  const wait = async expression => {
    for (let i = 0; i < 300; i++) { if (await keyboard.evaluate(expression)) return; await delay(100); }
    throw new Error(`Timed out: ${expression}`);
  };
  const click = selector => driver.execute("document.querySelector(arguments[0]).click()", [selector]);
  await wait('document.documentElement.dataset.appMode === "title" && !document.querySelector("#session-new-game").disabled');
  await click("#session-new-game");
  await selectCreationRace(driver, "demo.race.rfb-human");
  await selectCreationBuild(driver, "demo.build.mindcrafter");
  await driver.execute('document.querySelector("#session-character-name").value="Render acceptance";document.querySelector("#session-seed").value="511";');
  await click("#session-start-game");
  await wait('document.documentElement.dataset.appMode === "playing" && document.querySelector("#map-host").dataset.playerDisplayX !== undefined');
  report.lightingMode = await keyboard.evaluate('document.querySelector("#map-host").dataset.lightingMode');
  assert.equal(report.lightingMode, "rust-content-lights-v1", "original per-cell lighting is active");
  await keyboard.key("m");
  await wait('document.querySelector("#player-page-dialog").open && document.querySelector(".ability-row") !== null');
  for (const width of [700, 1280, 1920]) {
    await keyboard.viewport(width, 900); await delay(100);
    const layout = await keyboard.evaluate(`(()=>{const list=document.querySelector('#ability-list');
      const rows=[...list.querySelectorAll('.ability-row:not([hidden])')];
      return {width:innerWidth,columns:getComputedStyle(list).gridTemplateColumns.split(' ').length,
        cards:rows.length,damage:rows.map(r=>r.querySelector('.ability-damage')?.textContent).filter(Boolean),
        overflow:rows.some(r=>r.scrollWidth>r.clientWidth+1),closed:rows.every(r=>!r.querySelector('details').open)};})()`);
    assert.ok(layout.columns >= 2 && layout.columns <= 4);
    assert.ok(layout.damage.length > 0); assert.equal(layout.overflow, false); assert.equal(layout.closed, true);
    report.layouts.push(layout);
    await writeFile(path.join(output, `cards-${width}.png`), await keyboard.screenshot(), "base64");
  }
  assert.deepEqual(report.layouts.map(l => l.columns), [2, 3, 4]);
  await keyboard.key("A", 8);
  assert.equal(await keyboard.evaluate("document.querySelector('[data-ability-key=a] details').open"), true);
  await keyboard.key("Escape"); await keyboard.viewport(1280, 900);
  await keyboard.evaluate(`window.__movement=[];window.__captureMovement=true;
    function sample(){const h=document.querySelector('#map-host');window.__movement.push({x:Number(h.dataset.playerDisplayX),y:Number(h.dataset.playerDisplayY),cx:Number(h.dataset.cameraX),cy:Number(h.dataset.cameraY)});if(window.__captureMovement)requestAnimationFrame(sample)};requestAnimationFrame(sample);`);
  await keyboard.key("ArrowRight"); await delay(180);
  await keyboard.key("ArrowDown"); await delay(180);
  report.movement = await keyboard.evaluate("window.__captureMovement=false;window.__movement");
  assert.ok(report.movement.some(p => !Number.isInteger(p.x) || !Number.isInteger(p.y)), "actual adjacent movement interpolates");
  await writeFile(path.join(output, "map.png"), await keyboard.screenshot(), "base64");
  assert.deepEqual(keyboard.errors, []);
  await writeFile(path.join(output, "report.json"), JSON.stringify(report, null, 2));
  console.log(JSON.stringify({ layouts: report.layouts, frames: report.movement.length, lightingMode: report.lightingMode }));
} finally {
  if (keyboard && child.exitCode === null) {
    await keyboard.evaluate('void window.__TAURI_INTERNALS__.invoke("plugin:window|destroy",{label:"main"});true');
    for (let i = 0; i < 20 && child.exitCode === null; i++) await delay(100);
  }
  keyboard?.close();
  if (child.exitCode === null) child.kill();
}
