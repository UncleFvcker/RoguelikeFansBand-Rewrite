// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { setTimeout as delay } from "node:timers/promises";
import { connectKeyboard } from "./character-creation-layout.e2e.mjs";

// Run only after build:standalone:debug. This uses the actual ordinary binary,
// with a fresh WebView profile; no WebDriver server or in-game session is needed.
const root = fileURLToPath(new URL("../../", import.meta.url));
const executable = path.join(root,"target","debug","rfb-tauri.exe");
const directory = path.join(root,"test-results","asgard");
await mkdir(directory,{recursive:true});
await mkdir(path.join(root,"target","e2e"),{recursive:true});
const profile = await mkdtemp(path.join(root,"target","e2e","asgard-standalone-"));
// Elevated WebView2 hosts ignore environment-based browser arguments.
// Use the documented host command-line switch for this local IPC probe.
const child = spawn(executable,["--edge-webview-switches=--remote-debugging-port=0 --disable-gpu"],{cwd:root,windowsHide:true,stdio:["ignore","pipe","pipe"],env:{...process.env,
  WEBVIEW2_USER_DATA_FOLDER:profile}});
let keyboard;
const logs=[];
child.stdout.on("data",data=>logs.push(String(data)));
child.stderr.on("data",data=>logs.push(String(data)));
let launchError;
child.on("error",error=>{launchError=error;});
try {
  let ready=false;
  for(let attempt=0;attempt<150;attempt++) {
    if(launchError) throw launchError;
    assert.equal(child.exitCode,null,"ordinary executable exited during startup");
    try {
      const port=(await readFile(path.join(profile,"EBWebView","DevToolsActivePort"),"utf8")).split("\n")[0];
      const pages=await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
      ready=pages.some(page=>page.type==="page" && page.url.includes("tauri.localhost"));
    } catch(error) {
      if(error.code!=="ENOENT" && error.cause?.code!=="ECONNREFUSED") throw error;
    }
    if(ready) break;
    await delay(200);
  }
  assert.ok(ready,"ordinary WebView debugging target");
  keyboard=await connectKeyboard(profile);
  for(let attempt=0;attempt<100;attempt++) {
    if(await keyboard.evaluate("Boolean(window.__TAURI_INTERNALS__ && document.documentElement.dataset.appMode)")) break;
    await delay(100);
  }
  const checks=await keyboard.evaluate(`(async()=>{
    const results=[];
    for(const [phase,targetId] of [['arrival',null],['route',null],['battle','demo.guardian.asgard.1']]) {
      try { await window.__TAURI_INTERNALS__.invoke('prepare_asgard_e2e',{phase,targetId});results.push({phase,accepted:true}); }
      catch(error) { results.push({phase,error:String(error)}); }
    }
    return results;
  })()`);
  assert.equal(checks.length,3);
  for(const check of checks) assert.equal(check.error,"Asgard E2E fixture is unavailable");
  assert.deepEqual(keyboard.errors,[]);
  await writeFile(path.join(directory,"standalone-guard-report.json"),JSON.stringify({executable,
    sha256:createHash("sha256").update(await readFile(executable)).digest("hex"),checks,errors:keyboard.errors},null,2)+"\n");
  process.stdout.write("Ordinary Tauri standalone rejected all Asgard preparation phases.\n");
} finally {
  keyboard?.close();
  if(child.exitCode===null && child.signalCode===null) child.kill();
  await writeFile(path.join(directory,"standalone-guard.log"),logs.join(""));
}
