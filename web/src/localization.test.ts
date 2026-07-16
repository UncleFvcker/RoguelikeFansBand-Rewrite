// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import test from "node:test";

import { Localization, MESSAGE_KEYS } from "./localization.ts";

const sources = {
  "en-US": readLocale("en-US"),
  "zh-CN": readLocale("zh-CN"),
};

test("English and Simplified Chinese resources contain matching keys and variables", () => {
  const english = extractMessages(sources["en-US"]);
  const chinese = extractMessages(sources["zh-CN"]);

  assert.deepEqual([...english.keys()].sort(), [...MESSAGE_KEYS].sort());
  assert.deepEqual([...chinese.keys()].sort(), [...MESSAGE_KEYS].sort());
  for (const key of MESSAGE_KEYS) {
    assert.deepEqual(
      [...(chinese.get(key) ?? [])].sort(),
      [...(english.get(key) ?? [])].sort(),
      `${key} must use the same named variables in both locales`,
    );
  }
});

test("Fluent formats locale-specific grammar and plural selection", () => {
  const localization = new Localization("en-US", sources);
  assert.equal(localization.format("inventory-stack-count", { count: 1 }), "1 stack");
  assert.equal(localization.format("inventory-stack-count", { count: 2 }), "2 stacks");
  assert.equal(
    localization.format("message-item-drop-success", { stacks: 1, quantity: 2 }),
    "You drop 1 stack containing 2 items.",
  );
  assert.equal(
    localization.format("message-item-pickup-success", {
      target: "luminous shard",
      quantity: 3,
    }),
    "You pick up luminous shard ×3.",
  );

  localization.setLocale("zh-CN");
  assert.equal(localization.format("inventory-stack-count", { count: 2 }), "2 堆");
  assert.equal(
    localization.format("message-item-pickup-success", {
      target: "发光碎片",
      quantity: 3,
    }),
    "你将 3 个发光碎片收入了背包。",
  );
});

test("a missing active-locale message falls back to English", () => {
  const localization = new Localization("zh-CN", {
    "en-US": ["app-title = English fallback"],
    "zh-CN": ["app-heading = 中文标题"],
  });
  assert.equal(localization.format("app-title"), "English fallback");
});

test("front-end sources do not reintroduce high-confidence hardcoded UI text", () => {
  const sourceDirectory = new URL("./", import.meta.url);
  for (const entry of readdirSync(sourceDirectory, { withFileTypes: true })) {
    if (!entry.isFile() || !entry.name.endsWith(".ts") || entry.name.endsWith(".test.ts")) continue;
    const source = readFileSync(new URL(entry.name, sourceDirectory), "utf8");
    assert.doesNotMatch(source, /[\p{Script=Han}]/u, `${entry.name} contains hardcoded Chinese text`);
    assert.doesNotMatch(
      source,
      /(?:textContent|innerText)\s*=\s*["']/,
      `${entry.name} assigns a literal directly to visible DOM text`,
    );
    assert.doesNotMatch(source, /\baddMessage\s*\(/, `${entry.name} bypasses localized messages`);
  }

  const html = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  for (const match of html.matchAll(/>([^<]+)</g)) {
    const text = match[1].trim();
    assert.match(text, /^(?:0|--)?$/, `index.html contains hardcoded text: ${text}`);
  }
});

function readLocale(locale: "en-US" | "zh-CN"): string[] {
  return ["ui.ftl", "game.ftl", "content.ftl"].map((file) =>
    readFileSync(new URL(`../../locales/${locale}/${file}`, import.meta.url), "utf8"),
  );
}

function extractMessages(resources: readonly string[]): Map<string, Set<string>> {
  const messages = new Map<string, Set<string>>();
  for (const resource of resources) {
    let currentKey;
    for (const line of resource.split(/\r?\n/)) {
      const declaration = /^([a-z][a-z0-9-]*)\s*=/.exec(line);
      if (declaration) {
        currentKey = declaration[1];
        if (messages.has(currentKey)) throw new Error(`duplicate Fluent key ${currentKey}`);
        messages.set(currentKey, new Set());
      }
      if (!currentKey) continue;
      for (const variable of line.matchAll(/\$([a-z][a-z0-9-]*)/g)) {
        messages.get(currentKey).add(variable[1]);
      }
    }
  }
  return messages;
}
