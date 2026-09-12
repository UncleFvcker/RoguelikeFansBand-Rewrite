// SPDX-License-Identifier: MPL-2.0

import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { selectCreationBuild, selectCreationRace } from "./character-creation.e2e.mjs";

// CDP supplies browser default keyboard behavior; the WebDriver plugin's keys
// only dispatch JS events and cannot exercise native Tab/Enter/Space handling.
export async function connectKeyboard(profile) {
  const port = (await readFile(path.join(profile, "EBWebView", "DevToolsActivePort"), "utf8")).split("\n")[0];
  const pages = await (await fetch(`http://127.0.0.1:${port}/json/list`).catch(error => { throw new Error(`WebView2 debugging port ${port}: ${String(error.cause)}`); })).json();
  const page = pages.find(page => page.type === "page" && page.url.includes("tauri.localhost"));
  assert.ok(page, "Tauri WebView2 debugging target");
  const socket = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    socket.addEventListener("open", resolve, { once: true });
    socket.addEventListener("error", reject, { once: true });
  });
  let nextId = 0;
  const send = (method, params) => new Promise((resolve, reject) => {
    const id = ++nextId;
    const timer = setTimeout(() => { socket.removeEventListener("message", receive); reject(new Error(`CDP timeout: ${method}`)); }, 10_000);
    function receive(event) {
      const response = JSON.parse(event.data);
      if (response.id !== id) return;
      clearTimeout(timer);
      socket.removeEventListener("message", receive);
      if (response.error) reject(new Error(JSON.stringify(response.error)));
      else resolve(response.result);
    }
    socket.addEventListener("message", receive);
    socket.send(JSON.stringify({ id, method, params }));
  });
  const keys = { Tab: ["Tab", 9], Enter: ["Enter", 13], Escape: ["Escape", 27], " ": ["Space", 32], Home: ["Home", 36], End: ["End", 35], ArrowLeft: ["ArrowLeft", 37], ArrowUp: ["ArrowUp", 38], ArrowRight: ["ArrowRight", 39], ArrowDown: ["ArrowDown", 40], a: ["KeyA", 65], "2": ["Numpad2", 98], "5": ["Numpad5", 101], "6": ["Numpad6", 102] };
  for (const digit of ["1", "3", "4", "7", "8", "9"]) keys[digit] = [`Numpad${digit}`, 96 + Number(digit)];
  keys.g = ["KeyG", 71];
  keys.o = ["KeyO", 79];
  keys.B = ["KeyB", 66, 8];
  keys[">"] = ["Period", 190, 8];
  const errors = [];
  socket.addEventListener("message", event => {
    const message = JSON.parse(event.data);
    if (message.method === "Runtime.exceptionThrown") errors.push(message.params.exceptionDetails.exception?.description ?? message.params.exceptionDetails.text);
  });
  await send("Runtime.enable", {});
  return {
    errors,
    async key(key, modifiers = 0) {
      const [code, windowsVirtualKeyCode, implicitModifiers = 0] = keys[key];
      const params = { key, code, windowsVirtualKeyCode, modifiers: modifiers | implicitModifiers };
      await send("Input.dispatchKeyEvent", { type: "keyDown", ...params, ...(key === " " ? { text: " " } : key === "Enter" ? { text: "\r" } : {}) });
      await send("Input.dispatchKeyEvent", { type: "keyUp", ...params });
    },
    async text(text) { await send("Input.insertText", { text }); },
    async evaluate(expression) {
      const result = await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
      if (result.exceptionDetails) throw new Error(result.exceptionDetails.exception?.description ?? result.exceptionDetails.text);
      return result.result.value;
    },
    async reload() {
      await send("Page.enable", {});
      const loaded = new Promise((resolve, reject) => {
        const timer = setTimeout(() => { socket.removeEventListener("message", receive); reject(new Error("CDP page reload timed out")); }, 30_000);
        function receive(event) {
          if (JSON.parse(event.data).method !== "Page.loadEventFired") return;
          clearTimeout(timer); socket.removeEventListener("message", receive); resolve();
        }
        socket.addEventListener("message", receive);
      });
      await send("Page.reload", {}); await loaded;
    },
    close() { socket.close(); },
  };
}

export async function runCreationLayoutScenario(driver, artifactDirectory, debugProfile) {
  await driver.waitFor('return !!document.documentElement.dataset.appMode && !!document.querySelector("#session-new-game")', "WebView page loaded", 60_000);
  const keyboard = await connectKeyboard(debugProfile);
  const click = selector => driver.execute('document.querySelector(arguments[0]).click(); return true;', [selector]);
  const visible = selector => driver.execute('return document.querySelector(arguments[0]).checkVisibility();', [selector]);
  const summary = () => driver.execute('return document.querySelector("#session-creation-summary").textContent;');
  const focusIs = selector => driver.execute('return document.activeElement.matches(arguments[0]);', [selector]);
  const layouts = [];
  const identities = [];
  const screenshot = async name => {
    await new Promise(resolve => setTimeout(resolve, 150));
    await writeFile(path.join(artifactDirectory, `creation-${name}.png`), await driver.screenshot(), "base64");
  };
  async function invoke(command, args) {
    await driver.execute(`window.__creationViewportDone = false; window.__creationViewportError = null;
      window.__TAURI_INTERNALS__.invoke(arguments[0], arguments[1]).then(() => window.__creationViewportDone = true, error => window.__creationViewportError = String(error)); return true;`, [command, args]);
    await driver.waitFor('return window.__creationViewportDone || window.__creationViewportError', command);
    assert.equal(await driver.execute('return window.__creationViewportError'), null);
  }
  async function viewport(width, height, zoom = 1) {
    await invoke("plugin:webview|set_webview_zoom", { label: "main", value: zoom });
    const rect = await driver.command("GET", "/window/rect");
    const current = await driver.execute('return { width: innerWidth, height: innerHeight }');
    await driver.command("POST", "/window/rect", { width: Math.round(rect.width + (width - current.width) * zoom), height: Math.round(rect.height + (height - current.height) * zoom) });
    await driver.waitFor('return Math.abs(innerWidth - arguments[0]) <= 1 && Math.abs(innerHeight - arguments[1]) <= 1', `${width}x${height} at ${zoom}`, 10_000, [width, height]);
    await driver.execute('window.__creationLayoutReady = false; requestAnimationFrame(() => requestAnimationFrame(() => window.__creationLayoutReady = true)); return true;');
    await driver.waitFor('return window.__creationLayoutReady', "responsive layout settled");
  }
  async function checkLayout(label) {
    const result = await driver.execute(`
      const bounds = node => { const r = node.getBoundingClientRect(); return { x:r.x, y:r.y, right:r.right, bottom:r.bottom, width:r.width, height:r.height }; };
      const panel = [...document.querySelectorAll('[data-creation-panel]')].find(node => !node.hidden);
      const controls = [...document.querySelectorAll('.session-creation-header button, #session-creation-tabs button, .session-creation-footer button'), ...panel.querySelectorAll('.session-menu-navigation button')].filter(node => node.checkVisibility());
      const content = [...panel.querySelectorAll('.session-menu-options, .session-menu-details')].filter(node => node.checkVisibility());
      return { viewport:[innerWidth, innerHeight], frame:bounds(document.querySelector('.session-card')), panelHeight:panel.clientHeight, page:panel.dataset.creationPanel, content:content.map(node => ({ id:node.id, ...bounds(node), scrollWidth:node.scrollWidth, clientWidth:node.clientWidth })), controls:controls.map(node => ({ text:node.textContent, ...bounds(node) })), document:[document.documentElement.scrollWidth, document.documentElement.scrollHeight], name:bounds(document.querySelector('#session-character-name')), dpr:devicePixelRatio };
    `);
    const [width, height] = result.viewport;
    assert.deepEqual(result.document, result.viewport, label);
    assert.ok(result.panelHeight >= 30, `${label}: usable panel`);
    assert.ok(Math.abs(result.frame.width - (width <= 640 ? width - 16 : width * .84)) < 2, label);
    assert.ok(Math.abs(result.frame.height - (width <= 640 ? height - 16 : height * .84)) < 2, label);
    if (result.page === "overview") assert.ok(result.name.height > 0 && result.name.y >= 0 && result.name.bottom <= result.frame.bottom, `${label}: name input reachable`);
    for (const control of result.controls) {
      assert.ok(control.width > 0 && control.height > 0 && control.x >= 0 && control.y >= 0 && control.right <= width + 1 && control.bottom <= height + 1, `${label}: ${control.text}`);
    }
    for (const content of result.content) {
      assert.ok(content.height >= 30 && content.width >= 100, `${label}: usable ${content.id}`);
      assert.ok(content.scrollWidth <= content.clientWidth + 1, `${label}: horizontal content overflow`);
      assert.ok(content.bottom <= result.frame.bottom, label);
    }
    layouts.push({ label, ...result });
  }
  async function reload(locale = "zh-CN") {
    await driver.execute('window.__creationReloadPending = true; localStorage.setItem("rfb.locale", arguments[0]); localStorage.setItem("rfb.input-preset", "numpad"); setTimeout(() => location.reload(), 50); return true;', [locale]);
    await driver.waitFor('return !window.__creationReloadPending && document.documentElement.dataset.appMode === "title" && !document.querySelector("#session-new-game").disabled', "title after reload", 60_000);
    await driver.execute('window.__creationUiErrors = []; window.addEventListener("error", event => window.__creationUiErrors.push(event.message)); window.addEventListener("unhandledrejection", event => window.__creationUiErrors.push(String(event.reason))); return true;');
  }
  async function tabTo(selector) {
    for (let step = 0; step < 40; step++) {
      if (await focusIs(selector)) return;
      await keyboard.key("Tab");
      assert.equal(await driver.execute('return document.activeElement === document.body || document.activeElement.checkVisibility()'), true, "focus must stay visible");
    }
    throw new Error(`Tab could not reach ${selector}`);
  }
  async function arrowTo(selector, key = "ArrowDown") {
    for (let step = 0; step < 15; step++) {
      if (await focusIs(selector)) return;
      await keyboard.key(key);
    }
    throw new Error(`Arrow navigation could not reach ${selector}`);
  }
  async function replaceText(text) { await keyboard.key("a", 2); await keyboard.text(text); }
  async function verifyGame(name, race, build, seed) {
    await driver.waitFor('return document.documentElement.dataset.appMode === "playing" && document.querySelector("#connection-status").classList.contains("ready")', "representative creation", 60_000);
    const identity = await driver.execute(`return { name:document.querySelector('#character-name-value').textContent, race:document.querySelector('#character-race-value').textContent, career:document.querySelector('#character-class-value').textContent, build:document.querySelector('#app').dataset.sessionBuildId, seed:document.querySelector('#app').dataset.sessionSeed, turn:parseInt(document.querySelector('#turn-value').textContent, 10) };`);
    assert.equal(identity.name, name); assert.match(identity.race, race); assert.equal(identity.build, build); assert.equal(identity.seed, seed);
    await keyboard.key("5");
    await driver.waitFor('return parseInt(document.querySelector("#turn-value").textContent, 10) > arguments[0]', "effective game action", 10_000, [identity.turn]);
    identities.push({ ...identity, afterTurn: await driver.execute('return parseInt(document.querySelector("#turn-value").textContent, 10)') });
    assert.deepEqual(await driver.execute('return window.__creationUiErrors'), []);
    process.stdout.write(`Creation and action: ${build} passed.\n`);
  }
  try {
    await reload();
    await invoke("plugin:window|set_min_size", { label: "main", value: null });
    for (const locale of ["zh-CN", "en-US"]) {
      await reload(locale);
      await click("#session-new-game");
      for (const [width, height, zoom] of [[1280, 720, 1], [1280, 820, 1], [1920, 1080, 1], [2560, 1440, 1], [900, 620, 1], [390, 844, 1], [450, 310, 2], [640, 360, 2]]) {
        await viewport(width, height, zoom);
        await click("#session-tab-overview");
        await checkLayout(`${locale}-${width}x${height}-overview-${zoom}`);
        await selectCreationRace(driver, "rfb-legacy.race.spectre");
        if (width === 900) {
          const confirmed = await summary();
          await tabTo('[data-race-group="undead"]'); await keyboard.key("ArrowRight");
          assert.equal(await focusIs('[data-race-group="other"]'), true);
          await keyboard.key("Enter"); assert.equal(await summary(), confirmed);
          await tabTo('[data-race-group="other"]'); await keyboard.key("ArrowLeft"); await keyboard.key("Enter");
          assert.equal(await focusIs('[data-race-id="rfb-legacy.race.spectre"]'), true);
        }
        await checkLayout(`${locale}-${width}x${height}-race-${zoom}`);
        if (width <= 640) {
          const confirmed = await summary();
          await screenshot(`${locale}-${width}x${height}-options-${zoom}`);
          await click('#session-page-race [data-menu-view="details"]');
          assert.equal(await visible("#session-race-options"), false);
          assert.equal(await visible("#session-race-details"), true);
          assert.equal(await driver.execute('return document.querySelector("#session-race-back").textContent'), locale === "zh-CN" ? "返回上一级" : "Back one level");
          assert.equal(await summary(), confirmed);
          await checkLayout(`${locale}-${width}x${height}-details-${zoom}`);
          await screenshot(`${locale}-${width}x${height}-details-${zoom}`);
          await driver.execute('document.querySelector("#session-race-details").focus(); return true;');
          await keyboard.key("End");
          await driver.waitFor('const details = document.querySelector("#session-race-details"); return details.scrollTop > 0 && Math.abs(details.scrollHeight - details.clientHeight - details.scrollTop) < 2;', "last special note reachable by keyboard scrolling");
          if (locale === "zh-CN") await click("#session-race-back");
          else await keyboard.key("Escape");
          assert.equal(await focusIs('[data-race-id="rfb-legacy.race.spectre"]'), true);
          assert.equal(await visible("#session-race-details"), false);
        }
        await selectCreationBuild(driver, "demo.build.high-mage-death");
        await checkLayout(`${locale}-${width}x${height}-career-${zoom}`);
        if (width <= 640) {
          await click('#session-page-career [data-menu-view="details"]');
          await checkLayout(`${locale}-${width}x${height}-realm-${zoom}`);
        }
        await screenshot(`${locale}-${width}x${height}-career-${zoom}`);
      }
      assert.deepEqual(await driver.execute('return window.__creationUiErrors'), []);
      await click("#session-new-game-back");
    }

    // A long native error and a maximum-length name must leave the menu usable at 200%.
    await click("#session-new-game");
    await driver.execute(`const input = document.querySelector('#session-character-name'); input.value = '长'.repeat(32); input.dispatchEvent(new Event('input', { bubbles:true }));
      window.__creationFetch = window.fetch;
      const url = window.__TAURI_INTERNALS__.convertFileSrc('initialize_game', 'ipc');
      window.fetch = (target, options) => target === url ? Promise.resolve(new Response('creation-error '.repeat(40), { status:400, headers:{'Content-Type':'text/plain','Tauri-Response':'error'} })) : window.__creationFetch(target, options); return true;`);
    await selectCreationBuild(driver, "demo.build.high-mage-death");
    await click("#session-start-game");
    await driver.waitFor('return document.querySelector("#session-error").textContent.includes("creation-error") && !document.querySelector("#session-start-game").disabled', "long creation error");
    await checkLayout("en-US-640x360-long-error-2");
    await tabTo(".session-feedback"); await keyboard.key("End");
    await driver.waitFor('const box = document.querySelector(".session-feedback"); return Math.abs(box.scrollHeight - box.clientHeight - box.scrollTop) < 2;', "long error readable by keyboard");
    await checkLayout("en-US-640x360-scrolled-error-2");
    await screenshot("en-US-640x360-long-error-2");
    await driver.execute('window.fetch = window.__creationFetch; return true;');
    process.stdout.write("Responsive sizes, languages and long error passed.\n");

    // Resizing may hide the focused pane or its toggle. Recover into a visible control.
    await viewport(1280, 720);
    await reload(); await click("#session-new-game");
    await selectCreationRace(driver, "rfb-legacy.race.draconian-red");
    await driver.execute('document.querySelector("#session-race-details").focus(); return true;');
    await viewport(390, 844);
    assert.equal(await focusIs('#session-page-race [data-menu-view="options"]'), true);
    await viewport(1280, 720);
    assert.equal(await focusIs('[data-race-id="rfb-legacy.race.draconian-red"]'), true);

    // Start at the title and use only native browser keys through game creation.
    await reload();
    await tabTo("#session-new-game"); await keyboard.key("Enter");
    assert.equal(await focusIs("#session-character-name"), true);
    await replaceText("Keyboard Traveler");
    await keyboard.key("Escape");
    assert.equal(await focusIs("#session-character-name"), true, "editing Escape must not leave creation");
    await tabTo("#session-seed"); await replaceText("83");
    await tabTo("#session-randomize-seed"); await keyboard.key("Enter");
    assert.notEqual(await driver.execute('return document.querySelector("#session-seed").value'), "83");
    await keyboard.key("Tab", 8);
    assert.equal(await focusIs("#session-seed"), true);
    await replaceText("83");
    await tabTo("#session-tab-overview"); await keyboard.key("ArrowRight");
    await tabTo('[data-race-group="human"]'); await keyboard.key("End"); await keyboard.key("Enter");
    await arrowTo('[data-race-id="draconian"]'); await keyboard.key("Enter");
    assert.equal(await driver.execute('return document.querySelector("#session-start-game").disabled'), true);
    await keyboard.key("Escape"); assert.equal(await focusIs('[data-race-id="draconian"]'), true);
    await keyboard.key("Enter"); await keyboard.key(" ");
    const red = await summary();
    await keyboard.key("ArrowDown"); assert.equal(await summary(), red);
    await keyboard.key("Home");
    await driver.execute('document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key:"Escape", isComposing:true, bubbles:true, cancelable:true })); return true;');
    assert.equal(await focusIs('[data-race-id="rfb-legacy.race.draconian-red"]'), true);
    await tabTo("#session-tab-race"); await keyboard.key("ArrowRight");
    await tabTo('[data-career-group="melee"]'); await arrowTo('[data-career-group="magic"]');
    await keyboard.key("Enter"); await arrowTo('[data-career-id="high-mage"]'); await keyboard.key("Enter");
    await keyboard.key("Escape"); assert.equal(await focusIs('[data-career-id="high-mage"]'), true);
    await keyboard.key("Enter"); await keyboard.key("Enter");
    assert.equal(await driver.execute('return document.documentElement.dataset.appMode'), "new-game", "realm Enter must not start the game");
    await keyboard.key("Escape"); await keyboard.key("Escape");
    assert.equal(await focusIs("#session-tab-overview"), true);
    const confirmed = await summary();
    await keyboard.key("Escape");
    assert.equal(await driver.execute('return document.documentElement.dataset.appMode'), "title");
    await tabTo("#session-new-game"); await keyboard.key("Enter");
    assert.equal(await summary(), confirmed);
    assert.equal(await driver.execute('return document.querySelector("#session-seed").value'), "83");
    await screenshot("keyboard-confirmed");
    await tabTo("#session-start-game"); await keyboard.key(" ");
    await verifyGame("Keyboard Traveler", /红色/, "demo.build.high-mage-death", "83");

    for (const [name, raceId, raceName, build] of [["普通战士", "demo.race.rfb-human", /人类/, "demo.build.warrior"], ["不死圣骑士", "rfb-legacy.race.skeleton", /骷髅/, "demo.build.paladin-death"]]) {
      await reload(); await click("#session-new-game");
      await driver.execute('document.querySelector("#session-character-name").value = arguments[0]; document.querySelector("#session-seed").value = "83"; return true;', [name]);
      await selectCreationRace(driver, raceId); await selectCreationBuild(driver, build);
      await click("#session-start-game");
      await verifyGame(name, raceName, build, "83");
    }
    // Terminal projection fixture tests the result-page route, without playing a
    // whole campaign. Only this response is changed; the next run uses Rust again.
    await driver.execute(`window.__creationFetch = window.fetch;
      const url = window.__TAURI_INTERNALS__.convertFileSrc('dispatch_game_command', 'ipc');
      window.fetch = async (target, options) => {
        const response = await window.__creationFetch(target, options);
        if (target !== url) return response;
        window.fetch = window.__creationFetch;
        const update = await response.json(); update.campaign.status = 'retired';
        return new Response(JSON.stringify(update), { status:response.status, headers:response.headers });
      }; return true;`);
    await keyboard.key("5");
    await driver.waitFor('return !document.querySelector("#journey-result").hidden', "terminal projection result");
    await tabTo("#result-new-game"); await keyboard.key("Enter");
    await driver.waitFor('return document.documentElement.dataset.appMode === "new-game"', "new game from result");
    assert.equal(await focusIs("#session-character-name"), true);
    const newSeed = await driver.execute('return document.querySelector("#session-seed").value');
    assert.match(newSeed, /^\d+$/); assert.notEqual(newSeed, "83");
    assert.match(await summary(), /不死圣骑士.*骷髅.*圣骑士.*死亡/);
    await tabTo("#session-start-game"); await keyboard.key("Enter");
    await verifyGame("不死圣骑士", /骷髅/, "demo.build.paladin-death", newSeed);
    await writeFile(path.join(artifactDirectory, "character-creation-layout-acceptance.json"), JSON.stringify({ layouts, identities, keyboard: "WebView2 CDP native default actions; IME composing guard uses a synthetic composing key event", resultRoute: "UI-only retired projection fixture, followed by an actual new Rust session with a fresh seed" }, null, 2));
  } finally {
    keyboard.close();
  }
}
