// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";

export async function checkDiscovery({ root, directory, driver, keyboard, click, hash, save, load, capture, checks }) {
  const original = await save();
  const originalHash = await hash();
  const result = await promisify(execFile)("cargo", ["test", "-p", "rfb-core", "--lib",
    "game::tests::discovery::export_discovery_desktop_save", "--", "--ignored", "--exact"], {
    cwd: root, windowsHide: true, timeout: 300000, maxBuffer: 4 * 1024 * 1024,
    env: { ...process.env, DISCOVERY_INPUT: path.join(directory, "fresh.rfbsave") },
  });
  await writeFile(path.join(directory, "discovery-preparation.log"), result.stdout + result.stderr);
  const scenario = JSON.parse(await readFile(path.join(directory, "discovery-scenario.json"), "utf8"));
  await load(await readFile(path.join(directory, "discovery-prepared.rfbsave")), scenario.hash);
  for (const entry of ["objects", "artifacts", "egos", "monsters", "uniques", "kills", "dungeons"]) {
    await click('[data-guide="knowledge"]');
    await click('[data-knowledge-entry="' + entry + '"]');
    const expected = entry === "uniques" ? scenario.discovery.monsters.filter(row => row.monster.unique)
      : entry === "kills" ? scenario.discovery.monsters.filter(row => row.kills > 0) : scenario.discovery[entry];
    assert.ok(expected.length > 0, entry + " prepared data");
    assert.equal(await driver.execute('return document.querySelectorAll("[data-discovery-id]").length'), expected.length, entry);
    await driver.execute('for(const e of document.querySelectorAll("#archive-list details")) e.open=true;return true;');
    if (entry === "uniques") {
      for (const [filter, alive] of [["alive", true], ["dead", false]]) {
        await driver.execute('const f=document.querySelector("#archive-filter");f.value=arguments[0];f.dispatchEvent(new Event("change"));return true;', [filter]);
        assert.equal(await driver.execute('return document.querySelectorAll("[data-discovery-id]").length'), expected.filter(row => row.alive === alive).length);
      }
    }
    if (entry === "kills") assert.match(await driver.execute('return document.querySelector("#archive-list").textContent'), /本角色累计击杀: 1/);
    if (entry === "dungeons") assert.match(await driver.execute('return document.querySelector("#archive-list").textContent'), /最深到达层数: 3/);
    if (entry === "artifacts" || entry === "monsters") {
      await keyboard.viewport(390, 844);
      assert.equal(await driver.execute('const d=document.querySelector("#help-knowledge-dialog");return d.scrollWidth<=d.clientWidth'), true);
      await capture("discovery-" + entry + "-narrow");
      await keyboard.viewport(1280, 720);
    }
    assert.equal(await hash(), scenario.hash, entry + " is read only");
    await keyboard.key("Escape"); await keyboard.key("Escape");
  }
  checks.push("Seven persistent discovery pages display absent items, researched monsters, credited kills, alive/dead uniques and deepest dungeon level; queries preserve state");
  await click('[data-guide="knowledge"]');
  await click('[data-knowledge-entry="artifacts"]');
  await load(await save(), scenario.hash);
  assert.equal(await driver.execute('return document.querySelector("#help-knowledge-dialog").open'), false);
  await click('[data-guide="knowledge"]'); await click('[data-knowledge-entry="artifacts"]');
  assert.equal(await driver.execute('return document.querySelectorAll("[data-discovery-id]").length'), scenario.discovery.artifacts.length);
  await keyboard.key("Escape"); await keyboard.key("Escape");
  checks.push("Discovery archive survives native save/load with an identical hash and closes stale queries");
  await load(original, originalHash);
}
