// SPDX-License-Identifier: MPL-2.0
// Focused title acceptance against an ordinary standalone EXE. Global preferences are never written.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { readFile, writeFile, mkdir, mkdtemp, copyFile } from "node:fs/promises";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { connectKeyboard } from "./character-creation-layout.e2e.mjs";
import { selectCreationRace, selectCreationBuild } from "./character-creation.e2e.mjs";

const root = path.resolve(import.meta.dirname, "../..");
const directory = path.join(root, "test-results/title-screen");
await mkdir(directory, { recursive: true });
const install = await mkdtemp(path.join(directory, "game-"));
const profile = await mkdtemp(path.join(directory, "webview-"));
const executable = path.join(install, "rfb-tauri.exe");
await copyFile(process.env.RFB_STANDALONE_EXE ?? path.join(root, "target/release/rfb-tauri.exe"), executable);
const child = spawn(executable, ["--edge-webview-switches=--remote-debugging-port=0"], {
  cwd: install, windowsHide: true, stdio: ["ignore", "pipe", "pipe"],
  env: { ...process.env, WEBVIEW2_USER_DATA_FOLDER: profile },
});
const logs = [], checks = [], layouts = [];
child.stdout.on("data", bytes => logs.push(String(bytes)));
child.stderr.on("data", bytes => logs.push(String(bytes)));
let keyboard;
try {
  let found = false;
  for (let attempt = 0; attempt < 200; attempt++) {
    assert.equal(child.exitCode, null, "standalone process exited during startup");
    try {
      const port = (await readFile(path.join(profile, "EBWebView/DevToolsActivePort"), "utf8")).split("\n")[0];
      const pages = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
      if (pages.some(page => page.type === "page" && page.url.includes("tauri.localhost"))) { found = true; break; }
    } catch (error) { if (error.code !== "ENOENT" && error.cause?.code !== "ECONNREFUSED") throw error; }
    await delay(100);
  }
  assert.ok(found, "WebView debugging page");
  keyboard = await connectKeyboard(profile);
  const driver = {
    execute: (body, args = []) => keyboard.evaluate(`(function(){${body}}).apply(null,${JSON.stringify(args)})`),
    async waitFor(body, label, timeout = 30000, args = []) {
      const start = Date.now();
      while (Date.now() - start < timeout) { if (await this.execute(body, args)) return; await delay(80); }
      throw new Error(`Timed out: ${label}`);
    },
  };
  const click = selector => driver.execute("document.querySelector(arguments[0]).click(); return true;", [selector]);
  const focus = () => driver.execute("return document.activeElement.id");
  const shot = async name => writeFile(path.join(directory, `${name}.png`), await keyboard.screenshot(), "base64");
  const ready = () => driver.waitFor('return document.documentElement?.dataset.appMode === "title" && document.querySelector("#session-new-game")?.disabled === false', "title ready");
  const drawn = () => driver.waitFor('return document.querySelector("#title-background").dataset.rendered === "true"', "shader first frame");
  const probe = `
    window.__titleProbe = { draws:0, errors:[], shaderErrors:[], keys:[], clicks:[], gl:null, preferences:null };
    window.addEventListener('keydown', event => { window.__titleProbe.keys.push({key:event.key,prevented:event.defaultPrevented,target:event.target.id}); });
    window.addEventListener('click', event => { window.__titleProbe.clicks.push(event.target.id); });
    const originalError = console.error;
    console.error = (...args) => { window.__titleProbe.errors.push(args.map(String).join(' ')); originalError(...args); };
    const getContext = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function(type, ...args) {
      const context = getContext.call(this, type, ...args);
      if (context && (type === 'webgl2' || type === 'webgl') && !context.__titleCounted) {
        context.__titleCounted = true;
        const link = context.linkProgram;
        context.linkProgram = program => {
          link.call(context,program);
          if(!context.getProgramParameter(program,context.LINK_STATUS))window.__titleProbe.shaderErrors.push({
            log:context.getProgramInfoLog(program),shaders:context.getAttachedShaders(program).map(s=>({log:context.getShaderInfoLog(s),source:context.getShaderSource(s)}))});
        };
        for (const method of ['drawArrays','drawElements']) {
          const draw = context[method];
          context[method] = (...values) => {
            if (this.closest('#title-background')) { window.__titleProbe.draws++; window.__titleProbe.gl = context; }
            return draw.apply(context, values);
          };
        }
      }
      return context;
    };
    // UI-language projection only; native stored preferences and game rules remain untouched.
    const nativeFetch = window.fetch;
    window.fetch = async (url, options) => {
      const response = await nativeFetch(url, options);
      const locale = sessionStorage.getItem('title-test-locale');
      if (typeof url === 'string' && (url.endsWith('/load_preferences') || url.endsWith('/default_preferences'))) {
        const value = await response.clone().json();
        if (url.endsWith('/load_preferences')) window.__titleProbe.preferences = value;
        if (value && locale) {
          if (value.preferences) value.preferences.locale = locale; else value.locale = locale;
          return new Response(JSON.stringify(value), {status:response.status, headers:response.headers});
        }
      }
      return response;
    };
  `;
  await ready();
  await keyboard.command("Page.addScriptToEvaluateOnNewDocument", { source: probe });
  await keyboard.reload();
  await keyboard.command("Page.bringToFront", {});
  await ready(); await drawn();
  assert.deepEqual(await driver.execute("return window.__titleProbe.shaderErrors"), []);
  assert.deepEqual(await driver.execute("return window.__titleProbe.errors"), []);
  const originalPreferences = await driver.execute("return window.__titleProbe.preferences");
  assert.equal(await focus(), "session-new-game");
  assert.equal(await driver.execute('return document.querySelector("#session-continue").disabled'), true);
  assert.equal(await driver.execute('return document.querySelector("#session-heading").textContent'), "RoguelikeFansBand");
  const version = JSON.parse(await readFile(path.join(root, "web/package.json"), "utf8")).version;
  assert.equal(await driver.execute('return document.querySelector("#session-version").textContent'), `v${version}`);
  checks.push("Empty installation: brand, actual version, disabled Continue and New Game focus");

  for (const [width, height] of [[2560, 1440], [1920, 1080], [1280, 720], [390, 844]]) {
    await keyboard.viewport(width, height); await delay(300);
    const layout = await driver.execute(`
      const shell = document.querySelector('#session-shell'), heading = document.querySelector('#session-heading').getBoundingClientRect();
      const canvas = document.querySelector('#title-background canvas');
      return { size:[innerWidth,innerHeight], background:[canvas.width,canvas.height], overflow:shell.scrollWidth>shell.clientWidth+1,
        heading:[heading.left,heading.right], buttons:[...document.querySelectorAll('#session-title-view button')].map(b=>{
          const r=b.getBoundingClientRect(); return {id:b.id,x:r.x,right:r.right,bottom:r.bottom,height:r.height};}) };
    `);
    assert.equal(layout.overflow, false, `horizontal overflow at ${width}`);
    assert.ok(layout.background[0] <= 960 && layout.background[1] <= 600, "3D background stays within its pixel budget");
    assert.ok(layout.heading[0] >= 0 && layout.heading[1] <= width + 1);
    assert.equal(layout.buttons.length, 6);
    assert.ok(layout.buttons.every(b => b.x >= 0 && b.right <= width + 1 && b.height >= 44 && b.bottom <= height + 1), JSON.stringify(layout));
    layouts.push(layout); await shot(`title-${width}`);
  }
  checks.push("Four viewport sizes: full title, six reachable buttons, no horizontal overflow");
  await keyboard.viewport(1280, 720);
  await keyboard.key("ArrowUp"); assert.equal(await focus(), "session-exit");
  await keyboard.key("ArrowDown"); assert.equal(await focus(), "session-new-game");
  await keyboard.key("ArrowDown"); assert.equal(await focus(), "session-load-game");
  await keyboard.key("End"); assert.equal(await focus(), "session-exit");
  await keyboard.key("Home"); assert.equal(await focus(), "session-new-game");
  await keyboard.key("Tab"); assert.equal(await focus(), "session-load-game");
  await keyboard.key("Tab", 8); assert.equal(await focus(), "session-new-game");
  await keyboard.key("Enter");
  await driver.waitFor('return document.documentElement.dataset.appMode === "new-game"', "creation opened");
  await delay(200);
  const paused = await driver.execute("return window.__titleProbe.draws"); await delay(350);
  assert.equal(await driver.execute("return window.__titleProbe.draws"), paused);
  await click("#session-new-game-back"); assert.equal(await focus(), "session-new-game");
  await drawn(); await delay(100);
  checks.push("Native keyboard navigation skips disabled Continue; creation pauses GPU draws and returns focus");

  const startDraws = await driver.execute("return window.__titleProbe.draws"); await delay(1600);
  const drawing = await driver.execute("return window.__titleProbe.draws");
  assert.ok(drawing > startDraws, "animation draws while focused");
  await keyboard.command("Emulation.setEmulatedMedia", {features:[{name:"prefers-reduced-motion",value:"reduce"}]});
  await delay(150); const frozen = await driver.execute("return window.__titleProbe.draws"); await delay(400);
  assert.equal(await driver.execute("return window.__titleProbe.draws"), frozen);
  await keyboard.command("Emulation.setEmulatedMedia", {features:[]}); await delay(200);
  await driver.execute('document.hasFocus = () => false; window.dispatchEvent(new Event("blur")); return true;');
  const blurred = await driver.execute("return window.__titleProbe.draws"); await delay(350);
  assert.equal(await driver.execute("return window.__titleProbe.draws"), blurred);
  await driver.execute('delete document.hasFocus; window.dispatchEvent(new Event("focus")); return true;');
  checks.push(`Shader draws while active (${drawing-startDraws} draw calls / 1.6s), freezes for reduced motion and simulated loss of focus`);

  await driver.execute('document.querySelector("#session-settings").focus();return true;');
  await keyboard.key(" ");
  assert.equal(await driver.execute('return document.querySelector("#player-ui-settings-dialog").open'), true);
  await keyboard.key("Escape");
  await click("#title-high-scores");
  assert.ok(await driver.execute('return [...document.querySelectorAll("dialog[open]")].length > 0'));
  await keyboard.key("Escape");
  await click("#session-load-game");
  assert.equal(await driver.execute('return document.querySelector("#session-load-view").hidden'), false);
  await driver.waitFor('return !document.querySelector("#session-load-back").disabled', "save list refreshed");
  await click("#session-load-back");
  await ready();
  checks.push("Settings, scores and empty save browser open and return");

  await click("#session-new-game");
  await driver.waitFor('return document.documentElement.dataset.appMode === "new-game"', "creation reopened");
  await selectCreationRace(driver, "demo.race.rfb-human");
  await selectCreationBuild(driver, "demo.build.warrior");
  await driver.execute('document.querySelector("#session-character-name").value="Title acceptance"; document.querySelector("#session-seed").value="511";return true;');
  await click("#session-start-game");
  await driver.waitFor('return document.documentElement.dataset.appMode === "playing" && document.querySelector("#connection-status").classList.contains("ready")', "actual new character");
  await click("#save-button");
  await driver.waitFor('return window.__TAURI_INTERNALS__.invoke("list_native_saves").then(s=>s.some(x=>!x.museumCheckpoint))', "saved character");
  await keyboard.reload(); await ready(); await drawn();
  assert.equal(await focus(), "session-continue");
  assert.match(await driver.execute('return document.querySelector("#session-continue-summary").textContent'), /Title acceptance/);
  await shot("title-saved");
  await keyboard.key("Enter");
  await driver.waitFor('return document.documentElement.dataset.appMode === "playing" && document.querySelector("#connection-status").classList.contains("ready")', "continued character");
  assert.equal(await driver.execute('return document.querySelector("#character-name-value").textContent'), "Title acceptance");
  checks.push("Actual new game, native save, default Continue focus, matching summary and successful load");

  await keyboard.reload(); await ready(); await drawn();
  const source = await readFile(path.join(root,"web/src/title-background.ts"),"utf8");
  const wgsl = source.match(/const gpuSource = `([\s\S]*?)`;/)[1];
  const gpu = await driver.execute(`return (async()=>{
    if(!navigator.gpu)return {available:false}; const adapter=await navigator.gpu.requestAdapter();
    if(!adapter)return {available:false}; const device=await adapter.requestDevice();
    try {const module=device.createShaderModule({code:arguments[0]});
      const messages=await module.getCompilationInfo();
      return {available:true,messages:messages.messages.map(m=>({type:m.type,message:m.message}))};
    } finally {device.destroy();}
  })();`, [wgsl]);
  if(gpu.available) assert.deepEqual(gpu.messages.filter(m=>m.type==="error"),[]);
  checks.push(gpu.available ? "WebGPU WGSL compilation passed on a real adapter (runtime scene uses WebGL)" : "WebGPU adapter unavailable; WebGPU branch not accepted");

  await driver.execute('sessionStorage.setItem("title-test-locale","en-US");return true;');
  await keyboard.reload(); await ready(); await drawn();
  await driver.waitFor('return document.querySelector("#session-new-game").textContent === "New Game"', "English labels");
  assert.equal(await driver.execute('return document.querySelector("#session-version").textContent'), `v${version}`);
  await keyboard.viewport(390,844); await delay(200); await shot("title-english-390");
  assert.deepEqual(keyboard.errors,[]);
  assert.deepEqual(await driver.execute("return window.__titleProbe.errors"),[]);
  checks.push("English title uses an intercepted preferences projection; no global preferences saved");
  await driver.execute('window.__titleProbe.gl.getExtension("WEBGL_lose_context").loseContext();return true;');
  await driver.waitFor('return document.querySelectorAll("#title-background canvas").length === 0', "lost context static fallback");
  await shot("title-static-fallback");
  await click("#session-load-game");
  assert.equal(await driver.execute('return document.querySelector("#session-load-view").hidden'), false);
  await driver.waitFor('return !document.querySelector("#session-load-back").disabled', "fallback save list refreshed");
  await click("#session-load-back");
  await ready();
  const afterPreferences = await keyboard.evaluate('window.__TAURI_INTERNALS__.invoke("load_preferences")');
  // Fetch hook changes the projection only; remove the locale override before comparing the native value.
  await driver.execute('sessionStorage.removeItem("title-test-locale");return true;');
  const actualPreferences = await keyboard.evaluate('window.__TAURI_INTERNALS__.invoke("load_preferences")');
  assert.deepEqual(actualPreferences,originalPreferences);
  assert.equal(afterPreferences?.revision,originalPreferences?.revision);
  checks.push("WebGL context loss keeps the static title and save browser usable; global preferences unchanged");
  assert.deepEqual(keyboard.errors,[]);
  await click("#session-exit");
  for(let n=0;n<50 && child.exitCode===null;n++)await delay(100);
  assert.equal(child.exitCode,0);
  checks.push("Native Exit closes the test process");
  await writeFile(path.join(directory,"report.json"),JSON.stringify({executable,checks,layouts,gpu,errors:keyboard.errors},null,2));
  console.log(JSON.stringify({checks,errors:keyboard.errors},null,2));
} catch(error) {
  if(keyboard) {
    try {await writeFile(path.join(directory,"failure.png"),await keyboard.screenshot(),"base64");
      console.error(await keyboard.evaluate('JSON.stringify({probe:window.__titleProbe?.errors,shaders:window.__titleProbe?.shaderErrors,keys:window.__titleProbe?.keys,clicks:window.__titleProbe?.clicks,focus:document.activeElement?.id,mode:document.documentElement.dataset.appMode,summary:document.querySelector("#session-continue-summary")?.textContent})'));
    } catch {}
  }
  throw error;
} finally {
  keyboard?.close(); if(child.exitCode===null)child.kill();
  await writeFile(path.join(directory,"standalone.log"),logs.join(""));
}
