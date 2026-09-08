// SPDX-License-Identifier: MPL-2.0
// Focused browser-only acceptance; no Tauri process or game save is created.
// Supply Playwright through the test environment (NODE_PATH if bundled externally).
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { readFile, mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const { chromium } = createRequire(import.meta.url)("playwright");
const root = fileURLToPath(new URL("../", import.meta.url));
const html = (await readFile(new URL("../index.html", import.meta.url), "utf8"))
  .replace(/<script\b[^>]*>[\s\S]*?<\/script>/g, "");
const sources = {};
for (const locale of ["zh-CN", "en-US"]) {
  sources[locale] = await Promise.all(["ui", "game"].map((file) => readFile(new URL(`../../locales/${locale}/${file}.ftl`, import.meta.url), "utf8")));
  sources[locale].push(`fixture-weapon-long = ${locale === "zh-CN"
    ? "验收用超长武器名称：多装备槽与已知属性来源换行显示检查"
    : "Acceptance weapon with an unusually long visible name for source attribution and multiple equipment slots"}`);
  sources[locale].push(locale === 'zh-CN'
    ? 'fixture-task-name = 测试任务（测试城镇）\nfixture-task-description = 这是任务日志排版验收用的说明。保留原始任务名称，显示目标进度，并检查窄窗口中的长文本换行。'
    : 'fixture-task-name = Test task (Test town)\nfixture-task-description = Task description for layout acceptance. Preserve the original task name, show objective progress, and wrap long text inside narrow windows.');
}
const server = await createServer({
  root, configFile: false, server: { host: "127.0.0.1", port: 0 },
  plugins: [{
    name: "menu-layout-fixture",
    configureServer(server) {
      server.middlewares.use("/menu-layout-fixture", (_request, response) => {
        response.setHeader("Content-Type", "text/html; charset=utf-8");
        response.end(html);
      });
    },
  }],
});
let browser;
try {
  await server.listen();
  browser = await chromium.launch({ channel: "chrome", headless: true });
  const page = await browser.newPage();
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.stack));
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/menu-layout-fixture`);
  await page.addStyleTag({ path: fileURLToPath(new URL("../src/styles.css", import.meta.url)) });
  await page.evaluate(async (sources) => {
    const [{ PlayerUiLayout }, { InventoryPanel }, { createAppDom }, { AppState }, { Localization }, { renderTaskLog }] = await Promise.all([
      import("/src/player-ui-layout.ts"), import("/src/inventory-panel.ts"),
      import("/src/app-dom.ts"), import("/src/app-state.ts"), import("/src/localization.ts"),
      import("/src/status-panel.ts"),
    ]);
    document.getElementById("session-shell").hidden = true;
    document.getElementById("app").hidden = false;
    const localization = new Localization("zh-CN", sources);
    const layout = new PlayerUiLayout({ document, window, localization });
    layout.initialize();
    layout.install();
    const state = new AppState();
    state.status = { items: [], player: {
      carriedWeightTenthsPound: 260, carryCapacityTenthsPound: 1500,
      inventoryUsedSlots: 26, inventorySlotCapacity: 26,
    } };
    const panel = new InventoryPanel({
      dom: createAppDom(document), state, localization,
      formatter: { visibleItemName: (name) => name, equipmentSlotName: (slot) => slot },
      dispatch: async () => {}, startTargeting: () => {},
      updateCampaignAction: () => {}, announce: () => {},
    });
    panel.install();
    window.menuFixture = {
      layout,
      abandonedTasks: [],
      renderTasks(tasks, locale, disabled = false) {
        localization.setLocale(locale);
        localization.localizeDocument(document.getElementById('player-page-dialog'));
        renderTaskLog(document.getElementById('task-log-list'), tasks, localization, disabled,
          (task) => window.menuFixture.abandonedTasks.push(task.taskId));
      },
      render(full, locale) {
        localization.setLocale(locale);
        for (const id of ["player-page-dialog", "inventory-panel"]) {
          localization.localizeDocument(document.getElementById(id));
        }
        layout.localize();
        state.bodySlots = Array.from({ length: full ? 40 : 0 }, (_, index) => ({
          id: `slot-${index}`, slotType: "weapon",
        }));
        const items = Array.from({ length: full ? 26 : 0 }, (_, index) => ({
          id: `item-${index}`, kindId: "fixture", displayNameKey: locale === "zh-CN"
            ? "验收用长物品名称".repeat(12) : "Long inventory item name ".repeat(12),
          quantity: index ? index * 11 : 3, weightTenthsPound: index % 2 ? 13 : 10, equipmentSlot: "weapon", usable: true,
          fuel: index % 2 ? { current: 1500, maximum: 5000 } : undefined,
          identification: "unexamined", modifiers: { attack: 0, defense: 0, maxHp: 0, speed: 0 },
        }));
        state.selectedInventoryIds.clear();
        if (full) state.selectedInventoryIds.add("item-0");
        panel.render(items, []);
        layout.open("inventory");
      },
    };
  }, sources);

  async function measure() {
    return page.evaluate(() => {
      const rect = (selector) => {
        const element = document.querySelector(selector);
        const box = element.getBoundingClientRect();
        return { x: box.x, y: box.y, width: box.width, height: box.height,
          bottom: box.bottom, right: box.right, clientHeight: element.clientHeight,
          scrollHeight: element.scrollHeight, clientWidth: element.clientWidth, scrollWidth: element.scrollWidth };
      };
      return {
        frame: rect("#player-page-dialog"), header: rect("#player-page-dialog > header"),
        footer: rect("#player-page-footer"), content: rect("#player-page-host"),
        workspace: rect(".inventory-workspace"),
        equipment: rect("#inventory-equipment-column"), pack: rect("#inventory-pack-column"),
        switcher: rect("#inventory-view-switch"), list: rect("#inventory-list"),
        body: rect("body"), document: rect("html"),
      };
    });
  }

  let cases = 0;
  for (const locale of ["zh-CN", "en-US"]) {
    for (const full of [false, true]) {
      await page.evaluate(({ full, locale }) => window.menuFixture.render(full, locale), { full, locale });
      for (const [width, height] of [
        [3440, 1440], [2560, 1408], [1280, 720], [960, 600],
        [640, 480], [375, 667], [320, 360], [800, 360], [2560, 1408],
      ]) {
        await page.setViewportSize({ width, height });
        const m = await measure();
        const label = `${locale} ${full ? "full" : "empty"} ${width}x${height}`;
        assert.ok(m.frame.x >= 0 && m.frame.y >= 0 && m.frame.right <= width && m.frame.bottom <= height, label);
        assert.ok(m.footer.bottom <= m.frame.bottom && m.footer.y >= m.content.bottom - 1, label);
        assert.ok(m.header.bottom <= m.content.y && m.content.height > 35, label);
        for (const key of ["frame", "workspace", "body", "document"]) {
          assert.ok(m[key].scrollHeight <= m[key].clientHeight + 1, `${label} ${key} vertical overflow`);
          assert.ok(m[key].scrollWidth <= m[key].clientWidth + 1, `${label} ${key} horizontal overflow`);
        }
        if (width <= 640) {
          assert.ok(m.switcher.height > 0 && m.equipment.height === 0 && m.pack.height > 0, label);
          await page.locator('input[name="inventory-view"][value="pack"]').focus();
          await page.keyboard.press("ArrowLeft");
          assert.equal(await page.locator('input[name="inventory-view"][value="equipment"]').isChecked(), true);
          const switched = await measure();
          assert.ok(switched.equipment.height > 0 && switched.pack.height === 0, label);
          assert.equal(switched.frame.height, m.frame.height, label);
          await page.keyboard.press("ArrowRight");
        } else {
          assert.ok(m.switcher.height === 0 && m.equipment.height > 0 && m.pack.height > 0, label);
          assert.ok(m.equipment.right <= m.pack.x + 1, `${label} columns overlap`);
          const font = await page.locator('#player-page-dialog').evaluate((el) => parseFloat(getComputedStyle(el).fontSize));
          assert.ok(m.equipment.width <= 28 * font + 1, `${label} equipment takes too much width`);
        }
        if (full) {
          const card = await page.locator(".equipment-slot-button").first().boundingBox();
          const font = await page.locator('#player-page-dialog').evaluate((el) => parseFloat(getComputedStyle(el).fontSize));
          if (card) assert.ok(card.width <= 7.5 * font + 1, label);
          assert.ok(m.list.scrollHeight > m.list.clientHeight || height > 1000, label);
          const columns = await page.locator('#inventory-list').evaluate((list) => {
            const rows = [...list.querySelectorAll('.inventory-item')];
            return ['.inventory-quantity', '.inventory-item-status', '.inventory-item-weight', '.inventory-item-inspect'].map((selector) =>
              rows.map((row) => row.querySelector(selector).getBoundingClientRect().right));
          });
          if (width > 1100) for (const edges of columns) assert.equal(new Set(edges).size, 1, `${label}: aligned inventory metadata columns`);
        }
        const toolbar = await page.locator('#player-page-filters').evaluate((node) => ({
          fits: node.scrollWidth <= node.clientWidth + 1,
          searchWidth: node.querySelector('.inventory-search-control').getBoundingClientRect().width,
          font: parseFloat(getComputedStyle(node).fontSize),
        }));
        assert.ok(toolbar.fits && toolbar.searchWidth <= 24 * toolbar.font + 1, `${label}: bounded wrapping toolbar`);
        cases++;
      }
    }
  }
  // A child modal consumes the first Escape; the next closes the character menu.
  await page.locator(".inventory-item-inspect").first().click();
  await page.keyboard.press("Escape");
  assert.equal(await page.locator("#inventory-detail-dialog").evaluate((el) => el.open), false);
  assert.equal(await page.locator("#player-page-dialog").evaluate((el) => el.open), true);
  await page.keyboard.press("Escape");
  assert.equal(await page.locator("#player-page-dialog").evaluate((el) => el.open), false);
  await page.evaluate(() => window.menuFixture.layout.open("tasks"));
  const tasks = await measure();
  assert.ok(tasks.content.height > 100 && tasks.footer.bottom <= tasks.frame.bottom);
  // Character panes retain the existing bindings, and only the active pane scrolls.
  await page.evaluate(() => {
    window.menuFixture.layout.open("character");
    const ownership = {
      overview: ["attribute-list", "skill-list", "progression-experience-value"],
      proficiencies: ["character-proficiency-tables"],
      other: ["virtue-list", "mutation-list", "material-list", "progression-cap-value", "progression-multipliers-value"],
    };
    for (const [name, ids] of Object.entries(ownership)) {
      const pane = document.getElementById(`character-page-${name}`);
      for (const id of ids) {
        if (!pane.contains(document.getElementById(id))) throw new Error(`Wrong character pane: ${id}`);
      }
      const content = document.createElement("div");
      content.style.height = "2500px";
      pane.append(content); // Exercise independent scroll offsets even with initially empty lists.
    }
    if (document.querySelector("#character-page-proficiencies details")) throw new Error("Proficiencies still collapsed");
  });
  for (const viewport of [{ width: 1280, height: 720 }, { width: 375, height: 667 }, { width: 2560, height: 1408 }]) {
    await page.setViewportSize(viewport);
    const frame = await page.locator("#player-page-dialog").boundingBox();
    for (const name of ["overview", "proficiencies", "other"]) {
      await page.locator(`#character-tab-${name}`).click();
      assert.equal(await page.locator("#character-details-panel > [role=tabpanel]:visible").count(), 1);
      await page.locator(`#character-page-${name}`).evaluate((pane) => { pane.scrollTop = 150; });
      assert.deepEqual(await page.locator("#player-page-dialog").boundingBox(), frame);
    }
    for (const name of ["overview", "proficiencies", "other"]) {
      await page.locator(`#character-tab-${name}`).click();
      assert.equal(await page.locator(`#character-page-${name}`).evaluate((pane) => pane.scrollTop), 150);
    }
    await page.keyboard.press("Home");
    assert.equal(await page.locator("#character-tab-overview").getAttribute("aria-selected"), "true");
    await page.keyboard.press("End");
    assert.equal(await page.locator("#character-tab-other").getAttribute("aria-selected"), "true");
    await page.locator("#player-page-tab-inventory").click();
    await page.locator("#player-page-tab-character").click();
    assert.equal(await page.locator("#character-page-other").evaluate((pane) => pane.scrollTop), 150);
    for (const name of ["overview", "proficiencies", "other"]) {
      await page.locator(`#character-tab-${name}`).click();
      assert.equal(await page.locator(`#character-page-${name}`).evaluate((pane) => pane.scrollTop), 150);
    }
    await page.locator("#character-page-other").evaluate((pane) => { pane.scrollTop = 225; });
    await page.keyboard.press("Escape");
    await page.evaluate(() => window.menuFixture.layout.open("character"));
    assert.equal(await page.locator("#character-page-other").evaluate((pane) => pane.scrollTop), 225);
    for (const selector of ["#character-details-panel", "#player-page-dialog"]) {
      assert.equal(await page.locator(selector).evaluate((el) => el.scrollHeight <= el.clientHeight + 1), true);
    }
  }
  await page.evaluate(() => window.menuFixture.render(true, "zh-CN"));
  const artifacts = new URL("../../test-results/player-menu/", import.meta.url);
  await mkdir(artifacts, { recursive: true });
  for (const [name, width, height] of [["wide", 2560, 1408], ["narrow", 375, 667]]) {
    await page.setViewportSize({ width, height });
    await page.screenshot({ path: fileURLToPath(new URL(`${name}.png`, artifacts)) });
  }
  await page.evaluate(async (sources) => {
    const { renderCharacterOverview, renderCharacterTraits } = await import("/src/status-panel.ts");
    const { createAppDom } = await import("/src/app-dom.ts");
    const { Localization } = await import("/src/localization.ts");
    const localization = new Localization("zh-CN", sources);
    window.menuFixture.layout.open("character");
    document.getElementById("character-tab-overview").click();
    document.querySelector("#character-page-overview > div[style]")?.remove();
    localization.localizeDocument(document.getElementById("character-details-panel"));
    renderCharacterOverview(createAppDom(document), {
      name: "角色总览验收".repeat(12), gold: 14288, hp: 241, maxHp: 241, armorClass: 104, speed: 113,
      progress: { level: 27, experience: 113781n, maximumExperience: 113781n, experienceForNextLevel: 131250n },
      resources: [{ nameKey: "fixture-mana", current: 260, maximum: 267 }],
    }, 900000, localization);
    window.menuFixture.traitProgress = {
      pendingAttributeIncreases: 0, attributeCap: 118,
      attributes: Object.fromEntries(["strength", "intelligence", "wisdom", "dexterity", "constitution", "charisma"]
        .map((key) => [key, { natural: 18, maximumNatural: 18, potential: 118, effective: 38, index: 20 }])),
      skills: ["fixture-melee", "fixture-ranged", "fixture-saving"].map((nameKey) =>
        ({ nameKey, current: 71, maximum: 100, growthPerTenLevels: 12 })),
    };
    window.menuFixture.traitCommands = [];
    window.menuFixture.renderTraits = (state = {}) => renderCharacterTraits(
      createAppDom(document), window.menuFixture.traitProgress,
      { busy: false, playerDead: false, worldMap: false, ...state }, localization,
      async (command) => { window.menuFixture.traitCommands.push(command); },
    );
    window.menuFixture.renderTraits();
    document.getElementById("character-page-overview").scrollTop = 0;
  }, { ...sources, "zh-CN": [...sources["zh-CN"], "fixture-mana = 测试资源\nfixture-melee = 近战\nfixture-ranged = 射击\nfixture-saving = 豁免"] });
  for (const [name, width, height] of [["wide", 2560, 1408], ["medium", 1000, 700], ["narrow", 375, 667]]) {
    await page.setViewportSize({ width, height });
    const columns = await page.locator(".character-overview-column").evaluateAll((columns) =>
      columns.map((column) => { const r = column.getBoundingClientRect(); return { x: r.x, y: r.y, right: r.right }; }));
    if (width > 760) assert.ok(columns[0].right <= columns[1].x && columns[0].y === columns[1].y);
    assert.equal(await page.locator("#character-page-overview").evaluate((pane) => pane.scrollWidth <= pane.clientWidth + 1), true);
    const m = await measure();
    assert.ok(m.footer.bottom <= m.frame.bottom && m.content.bottom <= m.footer.y + 1);
    await page.screenshot({ path: fileURLToPath(new URL(`character-overview-${name}.png`, artifacts)) });
  }
  await page.setViewportSize({ width: 1280, height: 720 });
  const strength = page.locator("#attribute-list > li").first();
  const tooltip = page.locator("#character-attribute-strength-tooltip");
  assert.equal(await strength.locator(".attribute-value").textContent(), "18/20");
  assert.equal(await page.locator("#skill-list .skill-value").first().textContent(), "71");
  assert.equal(await page.locator("#attribute-list button").count(), 0);
  assert.equal(await page.locator("#hud-attribute-list [tabindex], #hud-attribute-list [popover]").count(), 0);
  await strength.hover();
  assert.equal(await tooltip.evaluate((el) => el.matches(":popover-open")), true);
  assert.match(await tooltip.textContent(), /历史自然上限：18/);
  await tooltip.hover();
  assert.equal(await tooltip.evaluate((el) => el.matches(":popover-open")), true);
  await page.keyboard.press("Escape");
  assert.equal(await tooltip.evaluate((el) => el.matches(":popover-open")), false);
  assert.equal(await page.locator("#player-page-dialog").evaluate((el) => el.open), true);
  await page.mouse.move(0, 0);
  await strength.focus();
  await page.locator("#character-attribute-strength-tooltip:popover-open").waitFor({ state: "visible" });
  assert.equal(await tooltip.evaluate((el) => el.matches(":popover-open")), true);
  await page.keyboard.press("Tab");
  assert.equal(await tooltip.evaluate((el) => el.matches(":popover-open")), false);
  await page.locator("#character-tab-other").click();
  assert.equal(await page.locator(".character-stat-tooltip:popover-open").count(), 0);
  await page.locator("#character-tab-overview").click();
  const skillTooltip = page.locator("#character-skill-0-tooltip");
  await page.locator("#skill-list > li").first().hover();
  assert.equal(await skillTooltip.evaluate((el) => el.matches(":popover-open")), true);
  assert.match(await skillTooltip.textContent(), /上限：100/);
  assert.match(await skillTooltip.textContent(), /每十级成长：12/);
  await page.evaluate(() => {
    window.menuFixture.traitProgress.pendingAttributeIncreases = 1;
    window.menuFixture.traitProgress.attributes.intelligence.maximumNatural = 118;
    window.menuFixture.renderTraits();
  });
  assert.equal(await page.locator("#attribute-list button").count(), 6);
  assert.equal(await page.locator("#attribute-list button").nth(1).isDisabled(), true);
  await strength.locator("button").click();
  assert.deepEqual(await page.evaluate(() => window.menuFixture.traitCommands), [{ type: "increase-attribute", attribute: "strength" }]);
  for (const flag of ["busy", "playerDead", "worldMap"]) {
    await page.evaluate((flag) => window.menuFixture.renderTraits({ [flag]: true }), flag);
    assert.equal(await page.locator("#attribute-list button:disabled").count(), 6);
  }
  await page.evaluate(() => window.menuFixture.renderTraits());
  await strength.hover();
  await page.setViewportSize({ width: 375, height: 667 });
  await page.waitForFunction(() => !document.querySelector(".character-stat-tooltip:popover-open"));
  assert.equal(await page.locator(".character-stat-tooltip:popover-open").count(), 0);
  await page.locator('label:has(input[name="character-overview-group"][value="attributes"])').click();
  await strength.hover();
  const tipBounds = await tooltip.boundingBox();
  assert.ok(tipBounds.x >= 0 && tipBounds.x + tipBounds.width <= 375 && tipBounds.y + tipBounds.height <= 667);
  const tipPane = await page.locator('#character-page-overview').boundingBox();
  assert.ok(tipBounds.y >= tipPane.y && tipBounds.y + tipBounds.height <= tipPane.y + tipPane.height + 1,
    'hover details remain inside the central pane, clear of navigation and footer');
  await page.screenshot({ path: fileURLToPath(new URL("character-tooltip-narrow.png", artifacts)) });
  await page.setViewportSize({ width: 375, height: 340 });
  await strength.hover();
  await page.locator("#character-page-overview").evaluate((pane) => { pane.scrollTop += 40; });
  await page.waitForFunction(() => !document.querySelector(".character-stat-tooltip:popover-open"));
  await page.locator("#player-page-tab-inventory").click();
  assert.equal(await page.locator(".character-stat-tooltip:popover-open").count(), 0);
  await page.evaluate(async (sources) => {
    const { renderCharacterProficiencies } = await import("/src/status-panel.ts");
    const { Localization } = await import("/src/localization.ts");
    window.menuFixture.layout.open("character");
    document.getElementById("character-tab-proficiencies").click();
    document.querySelector("#character-page-proficiencies > div[style]")?.remove();
    window.menuFixture.renderProficiencies = (locale = "zh-CN", empty = false) => {
      const localization = new Localization(locale, sources);
      const groups = ["sword", "polearm", "bow", "hafted", "digging", "other"];
      const ranks = ["unskilled", "beginner", "skilled", "expert", "master"];
      const progress = {
        weaponProficiencies: empty ? [] : groups.flatMap((group, index) =>
          Array.from({ length: index === 0 ? 70 : 9 }, (_, i) => ({
            itemKindId: `${group}-${i}`, nameKey: "fixture-weapon", group,
            equipped: i === 0, rank: ranks[i % ranks.length], current: i * 100, maximum: 8000, hitBonus: -20,
          }))),
        ridingProficiency: { rank: "beginner", current: 4000, maximum: 8000 },
        miningProficiency: { rank: "expert", current: 7000, maximum: 8000, diggingPower: 42 },
      };
      renderCharacterProficiencies(document.getElementById("character-proficiency-tables"), progress, localization);
    };
    window.menuFixture.renderProficiencies();
  }, Object.fromEntries(Object.entries(sources).map(([locale, entries]) =>
    [locale, [...entries, `fixture-weapon = ${locale === "zh-CN" ? "测试长名称武器" : "Long test weapon name"}`]])));
  for (const [name, width, height, columns] of [["wide", 2560, 1408, 3], ["medium", 1000, 700, 2], ["narrow", 375, 667, 1]]) {
    await page.setViewportSize({ width, height });
    const host = page.locator("#character-proficiency-tables");
    assert.equal(await host.evaluate((el) => Number(getComputedStyle(el).columnCount)), columns);
    assert.equal(await host.locator(".proficiency-entry").count(), 117);
    assert.equal(await host.locator(".proficiency-equipped").count(), 6);
    assert.equal(await host.locator("th", { hasText: "剑类" }).count(), 2);
    assert.equal(await host.locator(".proficiency-rank").nth(2).textContent(), "[熟练]");
    assert.equal(await page.locator("#character-page-proficiencies").evaluate((pane) => pane.scrollWidth <= pane.clientWidth + 1), true);
    const entry = host.locator(".proficiency-entry").first();
    await entry.click();
    assert.equal(await entry.getAttribute("aria-expanded"), "true");
    assert.match(await page.locator("#character-proficiency-0-detail").textContent(), /0 \/ 8,000.*命中 -20/);
    await page.keyboard.press("Escape");
    assert.equal(await entry.getAttribute("aria-expanded"), "false");
    assert.equal(await page.locator("#player-page-dialog").evaluate((el) => el.open), true);
    await entry.focus();
    await page.keyboard.press("Enter");
    assert.equal(await entry.getAttribute("aria-expanded"), "true");
    await page.keyboard.press("Tab");
    assert.equal(await entry.getAttribute("aria-expanded"), "false");
    await entry.focus();
    await page.keyboard.press("Enter");
    await page.keyboard.press("Escape");
    await page.screenshot({ path: fileURLToPath(new URL(`character-proficiencies-${name}.png`, artifacts)) });
  }
  await page.evaluate(() => window.menuFixture.renderProficiencies("en-US"));
  assert.equal(await page.locator(".proficiency-rank").nth(2).textContent(), "[Skilled]");
  await page.evaluate(() => window.menuFixture.renderProficiencies("zh-CN", true));
  assert.equal(await page.locator(".proficiency-entry").count(), 2);
  await page.evaluate(async (sources) => {
    const { renderCharacterOtherLists, renderCharacterMutations } = await import("/src/status-panel.ts");
    const { createAppDom } = await import("/src/app-dom.ts");
    const { Localization } = await import("/src/localization.ts");
    document.getElementById("character-tab-other").click();
    document.querySelector("#character-page-other > div[style]")?.remove();
    const localization = new Localization("zh-CN", sources);
    localization.localizeDocument(document.getElementById("character-page-other"));
    window.menuFixture.mutationCommands = [];
    window.menuFixture.renderOther = (full, state = {}, locale = "zh-CN", count = 1) => {
      const localization = new Localization(locale, sources);
      localization.localizeDocument(document.getElementById("character-page-other"));
      const dom = createAppDom(document);
      const mutations = full ? Array.from({ length: count }, (_, i) => ({ id: `fixture-mutation-${i}`,
        name: (locale === "zh-CN" ? "测试变异" : "Long mutation name ").repeat(6), rating: "good", locked: true,
        description: "测试说明，默认折叠，仅展开后可见。".repeat(15) })) : [];
      renderCharacterOtherLists(dom,
        full ? ["compassion", "honour", "justice", "sacrifice", "knowledge", "faith", "enlightenment", "enchantment"]
          .map((kind, i) => ({ kind, value: i * 10 - 30 })) : [],
        full ? [{ nameKey: "material-iron-ore", quantity: 12345 }] : [], localization);
      renderCharacterMutations(dom.mutationList, mutations,
        full ? { rewardId: "fixture-reward", candidates: [{ id: "candidate", name: "测试候选", rating: "great", description: "候选说明" }] } : null,
        { busy: false, playerDead: false, campaignEnded: false, ...state }, localization,
        async (command) => { window.menuFixture.mutationCommands.push(command); });
    };
    window.menuFixture.renderOther(false);
  }, sources);
  assert.equal(await page.locator("#character-page-other .character-other-empty").count(), 3);
  assert.equal(await page.locator(".character-growth-details").getAttribute("open"), null);
  for (const empty of await page.locator("#character-page-other .character-other-empty").all()) {
    assert.ok((await empty.boundingBox()).height < 40);
  }
  await page.evaluate(() => window.menuFixture.renderOther(true));
  assert.equal(await page.locator("#virtue-list .character-other-row").count(), 8);
  assert.equal(await page.locator("#material-list .character-other-value").textContent(), "12345");
  assert.equal(await page.locator("#mutation-list details[open]").count(), 0);
  assert.equal(await page.locator("#mutation-list .mutation-locked").count(), 1);
  const mutation = page.locator("#mutation-list .mutation-row details");
  await mutation.locator("summary").focus();
  await page.keyboard.press("Enter");
  assert.equal(await mutation.evaluate((el) => el.open), true);
  await page.evaluate(() => window.menuFixture.renderOther(true));
  assert.equal(await mutation.evaluate((el) => el.open), true);
  await page.locator(".mutation-choice-candidate").click();
  assert.deepEqual(await page.evaluate(() => window.menuFixture.mutationCommands), [
    { type: "choose-race-mutation", rewardId: "fixture-reward", mutationId: "candidate" },
  ]);
  for (const flag of ["busy", "playerDead", "campaignEnded"]) {
    await page.evaluate((flag) => window.menuFixture.renderOther(true, { [flag]: true }), flag);
    assert.equal(await page.locator(".mutation-choice-candidate").isDisabled(), true);
  }
  await page.evaluate(() => window.menuFixture.renderOther(true));
  await page.locator(".character-growth-details > summary").click();
  assert.equal(await page.locator("#progression-cap-value").isVisible(), true);
  for (const [name, width, height] of [["wide", 2560, 1408], ["narrow", 375, 667]]) {
    await page.setViewportSize({ width, height });
    assert.equal(await page.locator("#virtue-list").evaluate((el) => getComputedStyle(el).gridTemplateColumns.split(" ").length), 2);
    assert.equal(await page.locator("#character-page-other").evaluate((pane) => pane.scrollWidth <= pane.clientWidth + 1), true);
    const m = await measure();
    assert.ok(m.footer.bottom <= m.frame.bottom && m.content.bottom <= m.footer.y + 1);
    await page.screenshot({ path: fileURLToPath(new URL(`character-other-${name}.png`, artifacts)) });
  }
  await page.evaluate(async (sources) => {
    const { renderCharacterOverview, renderCharacterTraits } = await import("/src/status-panel.ts");
    const { createAppDom } = await import("/src/app-dom.ts");
    const { Localization } = await import("/src/localization.ts");
    window.menuFixture.acceptanceOverview = (locale) => {
      const localization = new Localization(locale, sources);
      localization.localizeDocument(document);
      const progress = { ...window.menuFixture.traitProgress, level: 27, experience: 113781n,
        maximumExperience: 113781n, experienceForNextLevel: 131250n, pendingAttributeIncreases: 2,
        skills: [{ nameKey: "fixture-resource", current: 71, maximum: 100, growthPerTenLevels: 12 }] };
      const dom = createAppDom(document);
      renderCharacterOverview(dom, { name: localization.format("fixture-resource"), hp: 241, maxHp: 241,
        armorClass: 104, speed: 113, gold: 14288, progress,
        resources: Array.from({ length: 6 }, () => ({ nameKey: "fixture-resource", current: 0, maximum: 50 })) }, 900000, localization);
      renderCharacterTraits(dom, progress, { busy: false, playerDead: false, worldMap: false }, localization, async () => {});
    };
  }, Object.fromEntries(Object.entries(sources).map(([locale, entries]) => [locale, [...entries,
    `fixture-resource = ${(locale === "zh-CN" ? "很长的测试名称" : "Very long test name ").repeat(8)}`]])));
  for (const locale of ["zh-CN", "en-US"]) {
    await page.evaluate((locale) => {
      window.menuFixture.acceptanceOverview(locale);
      window.menuFixture.renderProficiencies(locale);
      window.menuFixture.renderOther(true, {}, locale, 25);
    }, locale);
    for (const [width, height] of [[3440, 1440], [1000, 700], [375, 667], [320, 480], [2560, 1408]]) {
      await page.setViewportSize({ width, height });
      await page.locator("#character-tab-overview").click();
      if (width <= 760) {
        for (const group of ["identity", "progress", "journey", "attributes", "vitals", "skills"]) {
          await page.locator(`label:has(input[name="character-overview-group"][value="${group}"])`).click();
          assert.equal(await page.locator(".character-overview-group:visible").count(), 1);
          assert.equal(await page.locator(".character-overview-group:visible").getAttribute("data-overview-group"), group);
        }
      } else {
        assert.equal(await page.locator(".character-overview-group:visible").count(), 6);
        assert.ok(await page.locator(".character-facts > div").first().evaluate((el) =>
          el.lastElementChild.getBoundingClientRect().x - el.getBoundingClientRect().x <= parseFloat(getComputedStyle(el).fontSize) * 12.6 + 1));
      }
      await page.locator("#character-tab-proficiencies").click();
      if (width <= 760) {
        for (const group of ["sword", "polearm", "bow", "hafted", "digging", "other", "misc"]) {
          await page.locator(`label:has(input[name="character-proficiency-group"][value="${group}"])`).click();
          const visibleGroups = await page.locator(".proficiency-table:visible").evaluateAll((tables) => [...new Set(tables.map((t) => t.dataset.group))]);
          assert.deepEqual(visibleGroups, [group]);
        }
        await page.evaluate((locale) => window.menuFixture.renderProficiencies(locale), locale);
        assert.equal(await page.locator('input[name="character-proficiency-group"]:checked').inputValue(), "misc");
      } else {
        assert.equal(await page.locator(".proficiency-entry:visible").count(), 117);
      }
      for (const tab of ["overview", "proficiencies", "other"]) {
        await page.locator(`#character-tab-${tab}`).click();
        const m = await measure();
        assert.ok(m.header.y >= m.frame.y && m.header.bottom <= m.content.y);
        assert.ok(m.footer.bottom <= m.frame.bottom && m.content.bottom <= m.footer.y + 1);
        assert.ok(m.document.scrollHeight <= m.document.clientHeight + 1 && m.document.scrollWidth <= m.document.clientWidth + 1);
        assert.equal(await page.locator(`#character-page-${tab}`).evaluate((pane) => pane.scrollWidth <= pane.clientWidth + 1), true);
        if (width === 375) await page.screenshot({ path: fileURLToPath(new URL(`responsive-${locale}-${tab}.png`, artifacts)) });
      }
    }
  }
  // Attribute details are a data-free skeleton with independently scrolling categories.
  await page.locator("#character-tab-details").click();
  assert.equal(await page.locator(".character-detail-pane:visible").count(), 1);
  assert.equal(await page.locator(".character-detail-pane table, .character-detail-pane dl").count(), 0);
  await page.locator(".character-detail-pane").evaluateAll((panes) => {
    for (const pane of panes) {
      const filler = document.createElement("div");
      filler.style.height = "2500px";
      pane.append(filler);
    }
  });
  for (const [width, height] of [[2560, 1408], [1000, 700], [320, 480], [2560, 1408]]) {
    await page.setViewportSize({ width, height });
    for (const [index, category] of ["sources", "defenses", "offense"].entries()) {
      const tab = page.locator(`#character-detail-tab-${category}`);
      await tab.click();
      await tab.click();
      assert.equal(await page.locator(".character-detail-pane:visible").count(), 1);
      await page.locator(`#character-detail-${category}`).evaluate((pane, offset) => { pane.scrollTop = offset; }, 100 + index * 50);
    }
    await page.locator("#character-detail-tab-offense").press("Home");
    assert.equal(await page.locator("#character-detail-sources").evaluate((pane) => pane.scrollTop), 100);
    await page.locator("#character-detail-tab-sources").press("End");
    assert.equal(await page.locator("#character-detail-offense").evaluate((pane) => pane.scrollTop), 200);
    await page.locator("#character-tab-overview").click();
    await page.locator("#character-tab-details").click();
    await page.evaluate(() => {
      window.menuFixture.layout.open("inventory");
      window.menuFixture.layout.open("character");
    });
    await page.keyboard.press("Escape");
    await page.evaluate(() => window.menuFixture.layout.open("character"));
    assert.equal(await page.locator("#character-detail-offense").evaluate((pane) => pane.scrollTop), 200);
    const m = await measure();
    assert.ok(m.header.y >= m.frame.y && m.header.bottom <= m.content.y);
    assert.ok(m.footer.bottom <= m.frame.bottom && m.content.bottom <= m.footer.y + 1);
    assert.ok(m.document.scrollHeight <= m.document.clientHeight + 1 && m.document.scrollWidth <= m.document.clientWidth + 1);
    for (const selector of ["#character-details-panel", "#character-page-details", "#player-page-host"]) {
      assert.equal(await page.locator(selector).evaluate((el) => el.scrollHeight <= el.clientHeight + 1 && el.scrollWidth <= el.clientWidth + 1), true);
    }
  }
  await page.evaluate(async (sources) => {
    const { renderCharacterAttributeSources } = await import("/src/status-panel.ts");
    const { Localization } = await import("/src/localization.ts");
    document.querySelectorAll('.character-detail-pane > div[style="height: 2500px;"]').forEach((filler) => filler.remove());
    const step = (kind, modifier, extra = {}) => ({ kind, modifier, complete: true, suppressed: false,
      sourceId: null, nameKey: null, effectiveAfter: 18, upperLimitApplied: false, ...extra });
    const rows = ["strength", "intelligence", "wisdom", "dexterity", "constitution", "charisma"].map((attribute) => ({
      attribute, natural: 18, effective: 118, minimum: 3, maximum: 118,
      normalAppearanceMinimum: attribute === "charisma" ? 58 : null,
      sources: [step("race", 1), step("class", 2), step("personality", -1),
        step("mutation", -3, { sourceId: "fixture-mutation", suppressed: attribute === "charisma" }),
        step("equipment", 1, { complete: false, effectiveAfter: null, upperLimitApplied: null }),
        step("temporary-effect", 3, { sourceId: "fixture-status", effectiveAfter: null, upperLimitApplied: null }),
        ...(attribute === "charisma" ? [step("normal-appearance", 0, { effectiveAfter: null, upperLimitApplied: null })] : [])],
    }));
    const modifiers = Object.fromEntries(rows.map((row) => [row.attribute, 1]));
    const equipment = [{ id: "fixture-item", slotId: "ring-1", displayNameKey: "attribute-strength", knowledge: "aware", identification: "unexamined", modifiers },
      { id: "fixture-tool", slotId: "tool-1", displayNameKey: "attribute-wisdom", knowledge: "aware", identification: "identified", modifiers }];
    window.menuFixture.renderSources = (locale = "zh-CN", known = false) => {
      const localization = new Localization(locale, sources);
      localization.localizeDocument(document);
      renderCharacterAttributeSources(document.getElementById("character-attribute-sources"), rows.map((row) => ({ ...row,
        sources: row.sources.map((source) => known ? { ...source, complete: true, effectiveAfter: 118, upperLimitApplied: source.kind === "equipment" } : source) })),
        equipment, [{ id: "ring-1", slotType: "ring" }, { id: "tool-1", slotType: "tool" }],
        [{ id: "fixture-mutation", name: "<fixture mutation>" }], localization, () => "<fixture status>");
    };
    window.menuFixture.renderSources();
  }, sources);
  await page.locator("#character-detail-tab-sources").click();
  for (const locale of ["zh-CN", "en-US"]) {
    await page.evaluate((locale) => window.menuFixture.renderSources(locale), locale);
    for (const [width, height] of [[2560, 1408], [1000, 700], [320, 480], [2560, 1408]]) {
      await page.setViewportSize({ width, height });
      assert.equal(await page.locator(".attribute-source-table tbody tr").count(), 6);
      assert.equal(await page.locator('.attribute-source-table [data-source-focus="strength-current"]').textContent(), "18/100");
      assert.equal(await page.locator('#character-attribute-sources .attribute-increase').count(), 0);
      if (locale === "zh-CN" && width === 1000) {
        await page.locator("#character-detail-sources").evaluate((pane) => { pane.scrollTop = 0; pane.scrollLeft = 0; });
        await page.screenshot({ path: fileURLToPath(new URL("attribute-sources-table.png", artifacts)) });
      }
      if (width === 320) {
        const select = page.locator('.attribute-source-narrow select');
        await select.selectOption("strength");
        const inline = page.locator('.attribute-source-inline');
        assert.equal(await page.locator('.attribute-source-matrix').isVisible(), false);
        assert.match(await inline.textContent(), /18\/100/);
        assert.match(await inline.textContent(), /<fixture mutation>/);
        assert.match(await inline.textContent(), locale === "zh-CN" ? /未公开/ : /Withheld/);
        await inline.locator('summary').click();
        assert.match(await inline.locator('details').textContent(), /ring-1/);
        assert.doesNotMatch(await inline.locator('details').textContent(), /tool-1/);
        await select.selectOption("charisma");
        assert.match(await inline.textContent(), /18\/40/);
        await page.evaluate((locale) => window.menuFixture.renderSources(locale), locale);
        assert.equal(await select.inputValue(), "charisma");
        const m = await measure();
        assert.ok(m.document.scrollHeight <= m.document.clientHeight + 1 && m.document.scrollWidth <= m.document.clientWidth + 1);
        assert.ok(m.footer.bottom <= m.frame.bottom && m.content.bottom <= m.footer.y + 1);
        assert.equal(await page.locator('#character-detail-sources').evaluate((el) => el.scrollWidth <= el.clientWidth + 1), true);
        if (locale === "zh-CN") await page.screenshot({ path: fileURLToPath(new URL("attribute-sources-inline-320.png", artifacts)) });
        continue;
      }
      const cell = page.locator('[data-source-focus="strength-equipment"]');
      await page.locator('[data-source-focus="strength-attribute"]').focus();
      await cell.focus();
      await page.locator("#attribute-source-tip-strength-equipment:popover-open").waitFor({ state: "visible" });
      assert.equal(await page.locator("#attribute-source-tip-strength-equipment").evaluate((el) => el.matches(":popover-open")), true, `${locale} ${width}: focus tooltip`);
      await cell.press("Enter");
      const detail = page.locator("#attribute-source-detail-strength");
      assert.equal(await detail.evaluate((el) => el.matches(":popover-open")), true);
      assert.match(await detail.textContent(), /<fixture mutation>/);
      assert.match(await detail.textContent(), /<fixture status>/);
      assert.match(await detail.textContent(), locale === "zh-CN" ? /未公开/ : /Withheld/);
      assert.equal(await detail.locator("fixture").count(), 0);
      await detail.locator("summary").click();
      assert.match(await detail.locator("details").textContent(), /ring-1/);
      assert.doesNotMatch(await detail.locator("details").textContent(), /tool-1/);
      if (locale === "zh-CN" && [1000, 320].includes(width)) {
        await page.screenshot({ path: fileURLToPath(new URL(`attribute-sources-detail-${width}.png`, artifacts)) });
      }
      const bounds = await detail.evaluate((el) => {
        const r = el.getBoundingClientRect();
        return { top: r.top, bottom: r.bottom, right: r.right, left: r.left, scrollHeight: el.scrollHeight, height: el.clientHeight };
      });
      assert.ok(bounds.top >= 0 && bounds.bottom <= height && bounds.left >= 0 && bounds.right <= width);
      assert.ok(bounds.scrollHeight <= bounds.height + 1);
      await page.keyboard.press("Escape");
      assert.equal(await detail.evaluate((el) => el.matches(":popover-open")), false);
      assert.equal(await page.locator("#player-page-dialog").evaluate((el) => el.open), true);
      const m = await measure();
      assert.ok(m.document.scrollHeight <= m.document.clientHeight + 1 && m.document.scrollWidth <= m.document.clientWidth + 1);
      assert.ok(m.footer.bottom <= m.frame.bottom && m.content.bottom <= m.footer.y + 1);
    }
    await page.locator('[data-source-focus="strength-equipment"]').click();
    if (!await page.locator("#attribute-source-detail-strength details").evaluate((el) => el.open)) {
      await page.locator("#attribute-source-detail-strength summary").click();
    }
    await page.locator("#attribute-source-detail-strength .attribute-source-detail-body").evaluate((el) => { el.scrollTop = 100; });
    await page.evaluate((locale) => window.menuFixture.renderSources(locale, true), locale);
    assert.equal(await page.locator("#attribute-source-detail-strength").evaluate((el) => el.matches(":popover-open")), true);
    assert.match(await page.locator("#attribute-source-detail-strength").textContent(), locale === "zh-CN" ? /上限截断/ : /upper limit was applied/);
    assert.equal(await page.locator("#attribute-source-detail-strength details").evaluate((el) => el.open), true);
    await page.keyboard.press("Escape");
    await page.locator('[data-source-focus="charisma-other"]').click();
    assert.match(await page.locator("#attribute-source-detail-charisma").textContent(), /18\/40/);
    await page.locator("#character-tab-overview").click();
    assert.equal(await page.locator(".attribute-source-detail:popover-open").count(), 0);
    await page.locator("#character-tab-details").click();
  }
  console.log("Attribute source table: bilingual values, unknown equipment, source expansion, hover/focus/Enter/Escape, refresh and resize checks passed.");
  await page.setViewportSize({ width: 700, height: 600 });
  const sticky = await page.locator('.attribute-source-matrix').evaluate((matrix) => {
    matrix.scrollLeft = matrix.scrollWidth;
    const bounds = matrix.getBoundingClientRect();
    const row = matrix.querySelector('tbody tr');
    return { overflow: matrix.scrollWidth > matrix.clientWidth,
      first: row.firstElementChild.getBoundingClientRect().left - bounds.left,
      last: row.lastElementChild.getBoundingClientRect().right - bounds.right };
  });
  assert.equal(sticky.overflow, true);
  assert.ok(Math.abs(sticky.first) <= 1 && Math.abs(sticky.last) <= 1);
  await page.setViewportSize({ width: 2560, height: 1408 });
  await page.evaluate(async (sources) => {
    const { renderCharacterTraitsDetails } = await import("/src/character-traits-panel.ts");
    const { Localization } = await import("/src/localization.ts");
    const damageTypes = ["physical", "acid", "electricity", "fire", "cold", "poison", "light", "dark", "blindness", "fear", "confusion", "nether", "nexus", "sound", "shards", "rock", "chaos", "disenchant", "time", "mana", "gravity", "inertia", "plasma", "force", "nuke", "disintegrate", "storm", "holy-fire", "hell-fire", "ice", "water", "psi", "curse", "meteor", "rocket", "telekinesis"];
    const passives = ["regeneration", "warning", ...["animal", "undead", "demon", "orc", "troll", "giant", "dragon", "human", "good", "evil", "living", "nonliving"].map((id) => `esp-${id}`),
      "levitation", "telepathy", "slow-digestion", "hold-life", "see-invisible", ...["strength", "intelligence", "wisdom", "dexterity", "constitution", "charisma"].map((id) => `sustain-${id}`)];
    const stats = ["speed", "melee-skill", "ranged-skill", "throwing-skill", "device-skill", "saving-throw", "stealth", "search", "perception", "disarm", "digging", "equipment-life", "infravision", "natural-regeneration", "mutation-regeneration", "melee-attacks", "ranged-base-shot", "ranged-energy"];
    window.menuFixture.renderTraits = (locale, known, protectedAction = false, empty = false) => {
      const localization = new Localization(locale, sources);
      localization.localizeDocument(document);
      const source = { kind: "equipment", sourceId: "fixture-trait-sword", resistances: [{ damageType: "fire", level: "resistant" }, { damageType: "fire", level: "vulnerable" }],
        statusImmunities: ["fixture-status", ...(protectedAction ? ["rfb.status.paralysis"] : [])], passives: ["hold-life", "levitation"], reflectsBolts: false, passesWalls: false, lifePercent: 5 };
      const traitDetails = {
        activeWeaponId: "fixture-trait-sword", activeLauncherId: "fixture-trait-launcher",
        auras: ["fire", "electricity", "cold", "mana"].map((damageType) => ({ damageType, sourceIds: ["fixture-status"], evilOnly: damageType === "mana" })),
        negatives: known ? [
          { sourceId: "fixture-trait-sword", curse: "heavy", effects: [{ effect: "aggravate", active: true, asStealthPenalty: true }, { effect: "teleport", active: false, asStealthPenalty: false }] },
          { sourceId: "fixture-trait-second", curse: "heavy", effects: [] },
        ] : [],
        equipmentComplete: known, sources: [source],
        resistances: damageTypes.map((damageType) => ({ damageType, level: known ? "normal" : null, reductionPercent: known ? 0 : null })),
        passives: passives.map((passive) => ({ passive, active: known ? source.passives.includes(passive) : source.passives.includes(passive) ? true : null,
          sourceCount: known && ["hold-life", "see-invisible"].includes(passive) ? (passive === "hold-life" ? 2 : 0) : null })),
        statusImmunities: known ? source.statusImmunities : null, reflectsBolts: known ? false : null, passesWalls: false,
        stats: stats.map((id) => ({ id, value: known ? 110 : null, sources: known ? [{ sourceId: "fixture-trait-sword", amount: 5 }] : [] })),
        attacks: [
          { sourceId: "fixture-trait-sword", scope: "armed-melee", slays: [{ target: "dragon", level: "slay" }], brands: ["fire"], vampiric: false },
          { sourceId: "fixture-trait-sword", scope: "own-weapon", slays: [], brands: [], vampiric: true },
          { sourceId: "fixture-trait-ammo", scope: "current-ammunition", slays: [], brands: ["cold"], vampiric: false },
        ],
      };
      if (empty) {
        traitDetails.activeWeaponId = null;
        traitDetails.activeLauncherId = null;
        traitDetails.attacks = [];
        traitDetails.auras = [];
        traitDetails.negatives = [];
        traitDetails.sources = [];
      }
      renderCharacterTraitsDetails(document.getElementById("character-trait-defenses"), document.getElementById("character-trait-attacks"),
        { traitDetails, kindId: "fixture-player", statuses: [] },
        empty ? [] : [{ id: "fixture-trait-sword", displayNameKey: "fixture-weapon-long", slotId: "weapon-1", equipmentBonuses: { meleeAttacks: 1 } },
          { id: "fixture-trait-second", displayNameKey: "attribute-charisma", slotId: "weapon-2" },
          { id: "fixture-trait-launcher", displayNameKey: "attribute-dexterity", slotId: "launcher", equipmentBonuses: { baseShotDeltaPercent: 50 } },
          { id: "fixture-trait-ammo", displayNameKey: "attribute-wisdom" }], localization, () => "<fixture status>",
          [{ id: "weapon-1", slotType: "weapon" }, { id: "weapon-2", slotType: "weapon" }, { id: "launcher", slotType: "launcher" }]);
    };
  }, sources);
  for (const locale of ["zh-CN", "en-US"]) {
    for (const known of [false, true]) {
      await page.evaluate(([locale, known]) => window.menuFixture.renderTraits(locale, known), [locale, known]);
      await page.locator("#character-detail-tab-defenses").click();
      assert.equal(await page.locator('[data-trait^="resistance-"]').count(), 36);
      assert.equal(await page.locator('[data-trait^="passive-"]').count(), 25);
      assert.equal(await page.locator('[data-trait-section="trait-sustains"] details').count(), 6);
      assert.equal(await page.locator('[data-trait-section="trait-senses"] details').count(), 14);
      const freeAction = page.locator('[data-trait="free-action"]');
      assert.match(await freeAction.locator(".trait-value").textContent(), known
        ? locale === "zh-CN" ? /^未生效$/ : /^Inactive$/
        : locale === "zh-CN" ? /^未确认$/ : /^Unconfirmed$/);
      await page.evaluate(([locale, known]) => window.menuFixture.renderTraits(locale, known, true), [locale, known]);
      assert.equal(await freeAction.locator(".trait-value").textContent(), locale === "zh-CN" ? "生效" : "Active");
      assert.match(await freeAction.locator(".trait-row-body").textContent(), /weapon-1/);
      assert.equal(await page.locator('[data-trait="immunity-rfb.status.paralysis"]').count(), 0);
      await page.evaluate(([locale, known]) => window.menuFixture.renderTraits(locale, known), [locale, known]);
      assert.doesNotMatch(await page.locator("#character-trait-defenses").textContent(), /\[(trait-|item-passive-|damage-type-|resistance-level-)/);
      const fire = page.locator('[data-trait="resistance-fire"]');
      assert.match(await fire.locator("summary").textContent(), known ? /0%/ : locale === "zh-CN" ? /未确认/ : /Unconfirmed/);
      await fire.locator("summary").focus();
      if (!await fire.evaluate((el) => el.open)) await fire.locator("summary").press("Enter");
      assert.match(await fire.locator(".trait-row-body").textContent(), /weapon-1/);
      assert.match(await fire.locator(".trait-row-body").textContent(), locale === "zh-CN" ? /易伤/ : /vulnerable/);
      await page.evaluate(([locale, known]) => window.menuFixture.renderTraits(locale, known), [locale, known]);
      assert.equal(await fire.evaluate((el) => el.open), true);
      assert.equal(await page.evaluate(() => document.activeElement.dataset.traitFocus), "resistance-fire");
      assert.doesNotMatch(await page.locator('[data-trait="passive-levitation"] summary').textContent(), /来源|sources/);
      assert.match(await page.locator('[data-trait="passive-hold-life"] summary').textContent(), known ? /2/ : locale === "zh-CN" ? /未确认/ : /Unconfirmed/);
      for (const [width, height] of [[2560, 1408], [1000, 700], [320, 480], [2560, 1408]]) {
        await page.setViewportSize({ width, height });
        for (const category of ["defenses", "offense"]) {
          await page.locator(`#character-detail-tab-${category}`).click();
          const m = await measure();
          assert.ok(m.header.bottom <= m.content.y && m.footer.bottom <= m.frame.bottom && m.content.bottom <= m.footer.y + 1);
          assert.ok(m.document.scrollHeight <= m.document.clientHeight + 1 && m.document.scrollWidth <= m.document.clientWidth + 1);
          assert.equal(await page.locator(`#character-detail-${category}`).evaluate((el) => el.scrollWidth <= el.clientWidth + 1), true);
          if (width === 320) {
            const host = page.locator(category === "defenses" ? "#character-trait-defenses" : "#character-trait-attacks");
            const selects = host.locator('.trait-narrow-navigation select');
            const section = category === "defenses" ? "trait-senses" : "trait-weapons";
            await selects.nth(0).selectOption(section);
            const entry = category === "defenses" ? "passive-esp-dragon" : "weapon-fixture-trait-second";
            await selects.nth(1).selectOption(entry);
            assert.equal(await host.locator('[data-trait-section]:visible').count(), 1);
            assert.equal(await host.locator('.trait-row:visible').count(), 1);
            assert.equal(await host.locator('.trait-row:visible').getAttribute('data-trait'), entry);
            assert.equal(await host.locator('.trait-row:visible .trait-row-body').isVisible(), true);
            await page.evaluate(([locale, known]) => window.menuFixture.renderTraits(locale, known), [locale, known]);
            assert.equal(await selects.nth(0).inputValue(), section);
            assert.equal(await selects.nth(1).inputValue(), entry);
            // Empty categories keep their explanation without a nonfunctional entry select.
            if (category === "offense" && !known) {
              await selects.nth(0).selectOption("trait-negatives");
              assert.equal(await selects.nth(1).isVisible(), false);
              assert.equal(await host.locator('[data-trait-section="trait-negatives"]').isVisible(), true);
            }
          } else if (width === 2560) {
            const entries = page.locator(category === "defenses"
              ? '[data-trait-section="trait-resistances"] .trait-entries'
              : '[data-trait-section="trait-weapons"] .trait-entries');
            assert.ok(await entries.evaluate((el) => getComputedStyle(el).gridTemplateColumns.split(' ').length >= 2));
          }
          if (locale === "zh-CN" && known && [2560, 1000, 320].includes(width)) {
            await page.locator(`#character-detail-${category}`).evaluate((el) => { el.scrollTop = 0; });
            await page.screenshot({ path: fileURLToPath(new URL(`character-traits-${category}-${width}.png`, artifacts)) });
          }
        }
      }
      const offense = page.locator("#character-trait-attacks");
      assert.equal(await offense.locator('[data-trait-section="trait-attack-sources"] details').count(), 3);
      assert.equal(await offense.locator('[data-trait-section="trait-weapons"] details').count(), 3);
      assert.equal(await offense.locator('[data-trait-section="trait-auras"] details').count(), 4);
      assert.equal(await offense.locator('[data-trait="weapon-fixture-trait-second"] .trait-value').textContent(), locale === "zh-CN" ? "非当前攻击来源" : "Not the selected attack source");
      assert.equal(await offense.locator('[data-trait="weapon-fixture-trait-sword"] .trait-value').textContent(), locale === "zh-CN" ? "当前攻击来源" : "Selected attack source");
      assert.equal(await page.locator('#character-trait-defenses [data-trait="stat-melee-attacks"]').count(), 0);
      assert.equal(await offense.locator('[data-trait-section="trait-negatives"] details').count(), known ? 2 : 0);
      if (known) {
        assert.match(await offense.locator('[data-trait="stat-ranged-energy"] .trait-value').textContent(), locale === "zh-CN" ? /能量/ : /energy/);
        assert.doesNotMatch(await offense.locator('[data-trait="negative-fixture-trait-second"]').textContent(), /随机传送|Random teleport/);
        assert.match(await offense.locator('[data-trait="negative-fixture-trait-sword"]').textContent(), locale === "zh-CN" ? /潜行惩罚/ : /stealth penalty/);
      }
      assert.doesNotMatch(await offense.textContent(), /\[(trait-|weapon-brand-|item-|slay-target-)/);
      assert.match(await offense.textContent(), locale === "zh-CN" ? /仅该武器/ : /This weapon only/);
      assert.match(await offense.textContent(), locale === "zh-CN" ? /当前弹药/ : /Current ammunition/);
    }
  }
  console.log("Trait details: 36 resistance types, 25 abilities, bilingual known/unknown values, attack scopes, native keyboard expansion, refresh and local-scroll resize checks passed.");
  for (const locale of ["zh-CN", "en-US"]) {
    await page.evaluate((locale) => window.menuFixture.renderTraits(locale, true, false, true), locale);
    await page.locator('#character-detail-tab-offense').click();
    for (const [width, height] of [[2560, 1408], [320, 480], [1000, 700], [2560, 1408]]) {
      await page.setViewportSize({ width, height });
      const weapons = page.locator('[data-trait-section="trait-weapons"]');
      assert.equal(await weapons.locator('details').count(), 0);
      assert.match(await weapons.textContent(), locale === "zh-CN" ? /没有装备/ : /No weapon/);
      if (width === 320) {
        await page.locator('#character-trait-attacks .trait-narrow-navigation select').first().selectOption('trait-weapons');
        assert.equal(await weapons.isVisible(), true);
        assert.equal(await page.locator('#character-trait-attacks .trait-narrow-navigation select').nth(1).isVisible(), false);
      }
      const m = await measure();
      assert.ok(m.header.bottom <= m.content.y && m.footer.bottom <= m.frame.bottom && m.content.bottom <= m.footer.y + 1);
      assert.ok(m.document.scrollHeight <= m.document.clientHeight + 1 && m.document.scrollWidth <= m.document.clientWidth + 1);
    }
  }
  console.log("Final detail acceptance: empty equipment, long bilingual weapon names, sticky matrix edges and narrow category/entry selection passed.");
  await page.locator('#character-tab-overview').click();
  for (const [width, height, expected] of [
    [1280, 720, 16], [1920, 1080, 18], [2560, 1440, 20.6667],
    [3840, 2160, 26], [5120, 2880, 28], [3840, 720, 16], [1280, 720, 16],
  ]) {
    await page.setViewportSize({ width, height });
    const typography = await page.evaluate(() => {
      const font = (selector) => parseFloat(getComputedStyle(document.querySelector(selector)).fontSize);
      const result = {
        body: ['.character-facts > div', '#character-page-overview .attribute-row', '#character-page-overview .skill-row',
          '.attribute-source-table', '.attribute-source-detail-body', '.character-stat-tooltip', '.trait-row', '.proficiency-entry'].map(font),
        title: font('.character-overview-group > h3'),
        navigation: font('#player-page-tab-character'),
        note: font('#player-page-footer'),
      };
      // All four player pages share a scale; HUD, map and unrelated dialogs do not.
      const selectors = ['html', 'body', '#hp-value', '#map-host', '#object-list-dialog', '#player-ui-settings-title'];
      const outside = selectors.map(font);
      const dialog = document.getElementById('player-page-dialog');
      dialog.dataset.page = 'inventory';
      result.outsideUnchanged = JSON.stringify(outside) === JSON.stringify(selectors.map(font));
      result.inventoryNavigation = font('#player-page-tab-character');
      dialog.dataset.page = 'character';
      return result;
    });
    for (const size of typography.body) assert.ok(Math.abs(size - expected) < 0.02, `${width}x${height}: body ${size}`);
    assert.ok(Math.abs(typography.title - expected * 1.1) < 0.02);
    assert.ok(Math.abs(typography.navigation - expected) < 0.02);
    assert.ok(Math.abs(typography.note - expected * 0.875) < 0.02);
    assert.equal(typography.outsideUnchanged, true);
    assert.ok(Math.abs(typography.inventoryNavigation - expected) < 0.02);
    const m = await measure();
    assert.ok(m.header.bottom <= m.content.y && m.content.bottom <= m.footer.y + 1 && m.footer.bottom <= m.frame.bottom);
    assert.ok(m.document.scrollHeight <= m.document.clientHeight + 1 && m.document.scrollWidth <= m.document.clientWidth + 1);
    if (width === 2560) await page.screenshot({ path: fileURLToPath(new URL('character-typography-1440p.png', artifacts)) });
  }
  // DPR emulation checks that the CSS scale is not multiplied by physical pixels.
  // This is not a substitute for manual Windows display-scaling acceptance.
  for (const deviceScaleFactor of [1, 1.5, 2]) {
    const context = await browser.newContext({ viewport: { width: 1920, height: 1080 }, deviceScaleFactor });
    try {
      const dpiPage = await context.newPage();
      await dpiPage.goto(`http://127.0.0.1:${server.httpServer.address().port}/menu-layout-fixture`);
      await dpiPage.addStyleTag({ path: fileURLToPath(new URL('../src/styles.css', import.meta.url)) });
      const size = await dpiPage.locator('#player-page-dialog').evaluate((dialog) => {
        dialog.dataset.page = 'character';
        return parseFloat(getComputedStyle(dialog).fontSize);
      });
      assert.equal(size, 18);
    } finally {
      await context.close();
    }
  }
  console.log('Character typography: viewport-based 16–28px, short-window cap, shared-surface isolation and unchanged CSS sizes at DPR 1/1.5/2 passed.');
  await page.evaluate(async (sources) => {
    const { renderCharacterOverview, renderCharacterTraits } = await import('/src/status-panel.ts');
    const { createAppDom } = await import('/src/app-dom.ts');
    const { Localization } = await import('/src/localization.ts');
    window.menuFixture.renderOverviewLayout = (locale, extraResources) => {
      const localization = new Localization(locale, Object.fromEntries(Object.entries(sources).map(([key, files]) =>
        [key, [...files, key === 'zh-CN' ? 'fixture-skill = 测试技能\nfixture-resource = 测试资源' : 'fixture-skill = Test skill\nfixture-resource = Test resource']])));
      localization.localizeDocument(document);
      const dom = createAppDom(document);
      renderCharacterOverview(dom, {
        name: 'RFB Demo Character', gold: 337, hp: 56, maxHp: 56, armorClass: 150, speed: 110,
        progress: { level: 1, experience: 0n, maximumExperience: 0n, experienceForNextLevel: 10n },
        resources: Array.from({ length: extraResources }, () => ({ nameKey: 'fixture-resource', current: 5, maximum: 10 })),
      }, 0, localization);
      renderCharacterTraits(dom, { ...window.menuFixture.traitProgress, pendingAttributeIncreases: 0,
        skills: Array.from({ length: 9 }, () => ({ nameKey: 'fixture-skill', current: 18, maximum: 100, growthPerTenLevels: 12 })),
      }, { busy: false, playerDead: false, worldMap: false }, localization, async () => {});
    };
  }, sources);
  for (const locale of ['zh-CN', 'en-US']) {
    for (const resources of [0, 6]) {
      await page.evaluate(([locale, resources]) => window.menuFixture.renderOverviewLayout(locale, resources), [locale, resources]);
      for (const [width, height] of [[2560, 1440], [1280, 720], [3840, 2160], [2560, 1440]]) {
        await page.setViewportSize({ width, height });
        const layout = await page.locator('#character-page-overview').evaluate((pane) => {
          pane.scrollTop = 0;
          const bounds = pane.getBoundingClientRect();
          const columns = [...pane.querySelectorAll('.character-overview-column')].map((column) => ({
            bottom: column.getBoundingClientRect().bottom,
            groups: [...column.querySelectorAll('.character-overview-group')].map((group) => ({
              id: group.dataset.overviewGroup, top: group.getBoundingClientRect().top,
              bottom: group.getBoundingClientRect().bottom, height: group.getBoundingClientRect().height,
            })),
          }));
          const row = pane.querySelector('.attribute-row');
          return { bottom: bounds.bottom, columns, font: parseFloat(getComputedStyle(row).fontSize), row: row.getBoundingClientRect().height };
        });
        assert.deepEqual(layout.columns.map((column) => column.groups.map((group) => group.id)),
          [['identity', 'progress', 'journey'], ['attributes', 'vitals', 'skills']]);
        for (const column of layout.columns) {
          assert.ok(Math.abs(column.bottom - layout.bottom) <= 1, 'column extends to pane bottom');
          for (let i = 1; i < column.groups.length; i++) assert.ok(column.groups[i].top >= column.groups[i - 1].bottom);
        }
        assert.ok(layout.row < layout.font * 2.3, 'rows do not stretch to fill spare height');
        if (!resources) assert.ok(layout.columns[1].groups[2].height > layout.columns[1].groups[1].height, 'nine skills receive more space than three vitals');
        const m = await measure();
        assert.ok(m.header.bottom <= m.content.y && m.content.bottom <= m.footer.y + 1 && m.footer.bottom <= m.frame.bottom);
        assert.ok(m.document.scrollHeight <= m.document.clientHeight + 1 && m.document.scrollWidth <= m.document.clientWidth + 1);
        if (locale === 'zh-CN' && !resources && width === 2560) {
          await page.screenshot({ path: fileURLToPath(new URL('character-overview-full-height.png', artifacts)) });
        }
      }
    }
  }
  console.log('Overview full-height columns: preserved grouping, content-sized groups, bounded rows, 0/6 resources and repeated resize passed.');
  for (const locale of ['zh-CN', 'en-US']) {
    await page.evaluate((locale) => window.menuFixture.acceptanceOverview(locale), locale);
    for (const [width, height] of [[3840, 2160], [1000, 700], [320, 480], [2560, 1440]]) {
      await page.setViewportSize({ width, height });
      for (const group of ['identity', 'attributes', 'vitals', 'skills']) {
        if (width <= 760) await page.locator(`label:has(input[name="character-overview-group"][value="${group}"])`).click();
        const rows = await page.locator(`[data-overview-group="${group}"]`).evaluate((section) =>
          [...section.querySelectorAll('.character-facts > div, .attribute-row, .skill-row')].map((row) => {
            const [name, value] = row.children;
            const button = row.querySelector('button');
            const bounds = row.getBoundingClientRect();
            return {
              valueX: value.getBoundingClientRect().left,
              offset: value.getBoundingClientRect().left - bounds.left,
              font: parseFloat(getComputedStyle(row).fontSize),
              separated: name.getBoundingClientRect().right <= value.getBoundingClientRect().left,
              wrapping: getComputedStyle(name).whiteSpace === 'normal' && getComputedStyle(value).whiteSpace === 'normal',
              buttonFits: !button || (button.getBoundingClientRect().left >= value.getBoundingClientRect().right
                && button.getBoundingClientRect().right <= bounds.right + 1),
              buttonNear: !button || button.getBoundingClientRect().left - value.getBoundingClientRect().right <= parseFloat(getComputedStyle(row).fontSize) * 0.6 + 1,
            };
          }));
        for (const row of rows) {
          assert.ok(Math.abs(row.valueX - rows[0].valueX) <= 1, `${group}: values align`);
          assert.ok(row.offset <= row.font * 12.6 + 1);
          assert.ok(row.separated && row.wrapping && row.buttonFits && row.buttonNear, `${locale} ${width} ${group}: readable rows and nearby actions`);
        }
      }
      if (width > 760) {
        const positions = await page.locator('#character-page-overview').evaluate((pane) =>
          ['.attribute-value', '#character-vitals-list dd', '.skill-value'].map((selector) => pane.querySelector(selector).getBoundingClientRect().left));
        assert.ok(Math.max(...positions) - Math.min(...positions) <= 1, 'attributes, vitals and skills share a value column');
        assert.ok(await page.locator('#attribute-list').evaluate((list) =>
          Math.abs(list.getBoundingClientRect().width - list.parentElement.getBoundingClientRect().width) <= 1));
      }
      assert.equal(await page.locator('#character-page-overview').evaluate((pane) => pane.scrollWidth <= pane.clientWidth + 1), true);
    }
  }
  console.log('Overview horizontal layout: font-relative labels, aligned values, full-width attributes, bilingual wrapping and nearby raise buttons passed.');
  for (const locale of ['zh-CN', 'en-US']) {
    await page.evaluate((locale) => {
      window.menuFixture.renderSources(locale, true);
      window.menuFixture.renderTraits(locale, true);
      window.menuFixture.renderProficiencies(locale);
      window.menuFixture.renderOther(true, {}, locale, 25);
    }, locale);
    for (const [width, height, columns] of [[3840, 2160, 3], [1280, 720, 2], [320, 480, 1], [2560, 1440, 3]]) {
      await page.setViewportSize({ width, height });
      await page.locator('#character-tab-proficiencies').click();
      const proficiency = await page.locator('#character-proficiency-tables').evaluate((host) => {
        const entries = [...host.querySelectorAll('.proficiency-entry')].filter((entry) => entry.getClientRects().length);
        return {
          columns: new Set(entries.map((entry) => Math.round(entry.getBoundingClientRect().left))).size,
          ranksFit: entries.every((entry) => {
            const rank = entry.querySelector('.proficiency-rank');
            return rank.getBoundingClientRect().right <= entry.getBoundingClientRect().right + 1
              && rank.scrollWidth <= rank.clientWidth + 1 && getComputedStyle(rank).whiteSpace === 'nowrap';
          }),
        };
      });
      assert.equal(proficiency.columns, columns, `${locale} ${width}: actual proficiency columns`);
      assert.ok(proficiency.ranksFit, 'complete rank names remain visible');
      await page.locator('#character-tab-details').click();
      for (const category of ['sources', 'defenses', 'offense']) {
        await page.locator(`#character-detail-tab-${category}`).click();
        const pane = page.locator(`#character-detail-${category}`);
        assert.ok(await pane.evaluate((el) => el.scrollWidth <= el.clientWidth + 1));
        if (category === 'sources' && width > 600) {
          assert.ok(await pane.locator('.attribute-source-table').evaluate((el) =>
            el.getBoundingClientRect().width >= 44 * parseFloat(getComputedStyle(el).fontSize) - 1));
        } else if (width > 600) {
          assert.ok(await pane.locator('.trait-entries').evaluateAll((grids) => grids.every((grid) => {
            const font = parseFloat(getComputedStyle(grid).fontSize);
            // auto-fit collapses unused tracks to zero; only occupied tracks need a minimum.
            return getComputedStyle(grid).gridTemplateColumns.split(' ').map(parseFloat).filter((column) => column > 0)
              .every((column) => column >= Math.min(grid.clientWidth, 22 * font) - 1);
          })), 'trait columns reserve font-relative readable widths');
        }
        if (locale === 'zh-CN' && width === 3840) {
          await pane.evaluate((el) => { el.scrollTop = 0; });
          await page.screenshot({ path: fileURLToPath(new URL(`character-scaled-${category}-4k.png`, artifacts)) });
        }
      }
      await page.locator('#character-tab-other').click();
      assert.ok(await page.locator('#character-page-other').evaluate((pane) => {
        const font = getComputedStyle(document.getElementById('player-page-dialog')).fontSize;
        return pane.scrollWidth <= pane.clientWidth + 1
          && [...pane.querySelectorAll('.character-other-row, .mutation-details, .character-growth-details')]
            .every((el) => getComputedStyle(el).fontSize === font);
      }));
      const m = await measure();
      assert.ok(m.header.bottom <= m.content.y && m.content.bottom <= m.footer.y + 1 && m.footer.bottom <= m.frame.bottom);
      assert.ok(m.document.scrollHeight <= m.document.clientHeight + 1 && m.document.scrollWidth <= m.document.clientWidth + 1);
    }
  }
  console.log('Scaled character subpages: bilingual 4K/720p/narrow/1440p, actual 3/2/category columns, complete ranks, font-relative matrices/traits and fixed footer passed.');
  for (const locale of ['zh-CN', 'en-US']) {
    await page.evaluate((locale) => window.menuFixture.acceptanceOverview(locale), locale);
    for (const [width, height] of [[3840, 2160], [320, 360], [800, 360], [2560, 1440]]) {
      await page.setViewportSize({ width, height });
      await page.locator('#character-tab-details').click();
      await page.locator('#character-detail-tab-sources').press('End');
      assert.equal(await page.locator('#character-detail-tab-offense').getAttribute('aria-selected'), 'true');
      const controls = await page.evaluate(() => {
        const dialog = document.getElementById('player-page-dialog');
        const font = parseFloat(getComputedStyle(dialog).fontSize);
        const footer = document.getElementById('player-page-footer');
        const kbd = footer.querySelector('kbd');
        const close = document.getElementById('player-page-close');
        const tab = document.getElementById('character-detail-tab-offense');
        const tabRect = tab.getBoundingClientRect(), navRect = tab.parentElement.getBoundingClientRect();
        return {
          font, closeHeight: close.getBoundingClientRect().height,
          closeFits: close.getBoundingClientRect().right <= dialog.getBoundingClientRect().right,
          guideFont: getComputedStyle(kbd).fontSize === getComputedStyle(footer).fontSize,
          guideFits: kbd.getBoundingClientRect().top >= footer.getBoundingClientRect().top
            && kbd.getBoundingClientRect().bottom <= footer.getBoundingClientRect().bottom,
          focusedTabVisible: tabRect.left >= navRect.left - 1 && tabRect.right <= navRect.right + 1,
          contentHeight: document.getElementById('character-detail-offense').clientHeight,
          frameFits: dialog.scrollHeight <= dialog.clientHeight + 1 && dialog.scrollWidth <= dialog.clientWidth + 1,
          noTransform: getComputedStyle(dialog).transform === 'none',
        };
      });
      assert.ok(controls.font >= 16 && controls.closeHeight >= controls.font * 2.25);
      assert.ok(controls.closeFits && controls.guideFont && controls.guideFits && controls.focusedTabVisible);
      assert.ok(controls.frameFits && controls.noTransform && controls.contentHeight > 60, `${locale} ${width}x${height}: bounded controls leave readable central content`);
      if (locale === 'zh-CN' && [3840, 320].includes(width)) {
        await page.screenshot({ path: fileURLToPath(new URL(`character-controls-${width}.png`, artifacts)) });
      }
    }
  }
  console.log('Character controls: font-sized hit areas and Esc guide, visible keyboard navigation, fixed frame and central scrolling at 4K/320x360/shallow/re-maximized sizes passed.');
  assert.deepEqual(errors, []);
  console.log("Attribute details: nested navigation, independent scroll retention, Escape/reopen and four resize stages passed.");
  console.log(`Player menu: ${cases} base cases + bilingual character acceptance at five resize stages; group/category switching, long names, six resources, pending points, 117 proficiency rows, 25 mutations, fixed navigation/footer passed.`);
  // Shared menu scale: populated/empty sibling pages, unchanged frame and local scrolling.
  for (const locale of ['zh-CN', 'en-US']) {
    for (const full of [false, true]) {
      await page.evaluate(({ locale, full }) => {
        window.menuFixture.render(full, locale);
        const zh = locale === 'zh-CN';
        document.getElementById('ability-list').innerHTML = full
          ? Array.from({ length: 24 }, () => `<li class="ability-row"><div class="ability-details"><span class="ability-name">${zh ? '测试技能' : 'Test ability'}</span><span class="ability-description">${zh ? '用于检查字号和换行的技能说明。' : 'Ability description for readable typography and line wrapping.'}</span><span class="ability-summary">${zh ? '等级 1，消耗 5，失败率 20%' : 'Level 1, cost 5, failure 20%'}</span></div><div class="ability-actions"><button>${zh ? '学习' : 'Study'}</button><button disabled>${zh ? '遗忘' : 'Forget'}</button><button>${zh ? '施放' : 'Cast'}</button></div></li>`).join('')
          : `<li class="ability-empty">${zh ? '当前没有可用技能。' : 'No abilities available.'}</li>`;
        window.menuFixture.renderTasks(full ? Array.from({ length: 40 }, (_, i) => ({
          taskId: `task-${i}`, nameKey: 'fixture-task-name', descriptionKey: 'fixture-task-description',
          status: 'active', current: i, required: 40, stage: 1, stages: 1, retakesUsed: 0,
        })) : [], locale);
      }, { locale, full });
      for (const [width, height] of [[1280, 720], [1920, 1080], [2560, 1440], [3840, 2160], [320, 360], [800, 360], [2560, 1440]]) {
        await page.setViewportSize({ width, height });
        let shell;
        for (const name of ['character', 'ability', 'inventory', 'tasks', 'character']) {
          await page.evaluate((name) => {
            window.menuFixture.layout.open(name);
            if (name === 'character') document.getElementById('character-tab-overview').click();
          }, name);
          const m = await page.evaluate((name) => {
            const el = (s) => document.querySelector(s);
            const font = (s) => parseFloat(getComputedStyle(el(s)).fontSize);
            const bounds = (s) => el(s).getBoundingClientRect().toJSON();
            const selector = { character: '#character-page-overview', ability: '#ability-panel', inventory: '#inventory-list', tasks: '#task-log-panel' }[name];
            const content = el(selector);
            const samples = {
              character: ['.character-facts > div'], ability: ['.ability-name', '.ability-description', '.ability-empty', '.ability-actions button'],
              inventory: ['#inventory-list .inventory-item', '.equipment-slot-button', '#inventory-list .inventory-empty'], tasks: ['#task-log-list li', '#task-log-list button'],
            }[name].filter((s) => el(s));
            const close = bounds('#player-page-close');
            const frame = bounds('#player-page-dialog');
            const footer = bounds('#player-page-footer');
            return {
              shell: [frame, bounds('#player-page-dialog > header'), footer],
              font: font('#player-page-dialog'), samples: samples.map(font),
              navigation: font('#player-page-tabs button'), note: font('#player-page-footer'),
              fits: ['#player-page-dialog', '#player-page-host', selector].every((s) => el(s).scrollWidth <= el(s).clientWidth + 1),
              closeFits: close.right <= frame.right && close.bottom <= frame.bottom,
              contentFits: content.getBoundingClientRect().bottom <= footer.top + 1 && content.clientHeight > 35,
              scrolls: content.scrollHeight > content.clientHeight,
            };
          }, name);
          const label = `${locale} ${full} ${name} ${width}x${height}`;
          const expected = Math.max(16, Math.min(28, 10 + Math.min(width / 240, height / 135)));
          assert.ok(Math.abs(m.font - expected) < 0.02 && Math.abs(m.navigation - expected) < 0.02, label);
          for (const font of m.samples) assert.ok(Math.abs(font - expected) < 0.02, `${label}: body ${font}`);
          assert.ok(Math.abs(m.note - expected * 0.875) < 0.02, label);
          assert.ok(m.fits && m.closeFits && m.contentFits, `${label}: content and controls fit ${JSON.stringify(m)}`);
          if (shell) assert.deepEqual(m.shell, shell, `${label}: stable frame, header and footer across page switches`);
          shell = m.shell;
          if (full && height <= 720 && ['ability', 'tasks'].includes(name)) assert.ok(m.scrolls, `${label}: scroll inside the page`);
          if (locale === 'zh-CN' && width === 2560 && name !== 'character') {
            await page.screenshot({ path: fileURLToPath(new URL(`shared-menu-${name}-${full ? 'full' : 'empty'}.png`, artifacts)) });
          }
        }
      }
    }
  }
  console.log('Shared menu typography: all four pages, bilingual empty/full content, 720p/1080p/1440p/4K/narrow/shallow, stable navigation/footer and local scrolling passed.');
  // Exercise the production task renderer, including refresh while keyboard focus is inside a task.
  const statuses = ['locked', 'completed', 'available', 'paused', 'active', 'taken', 'failed', 'reward-available', 'abandoned'];
  const taskEntries = statuses.map((status, index) => ({
    taskId: `journal-${status}`, nameKey: 'fixture-task-name',
    descriptionKey: status === 'locked' ? null : 'fixture-task-description', status,
    current: index, required: 12, stage: 1, stages: status === 'active' ? 3 : 1,
    retakesUsed: 1, maxRetakes: status === 'paused' ? 2 : null,
  }));
  for (const locale of ['zh-CN', 'en-US']) {
    await page.setViewportSize({ width: 2560, height: 1440 });
    await page.evaluate(({ tasks, locale }) => {
      document.getElementById('task-log-list').replaceChildren();
      window.menuFixture.renderTasks(tasks, locale);
      window.menuFixture.layout.open('tasks');
    }, { tasks: taskEntries, locale });
    const groups = await page.locator('.task-log-group').evaluateAll((nodes) => nodes.map((node) => ({
      key: node.dataset.taskLogKey, open: node.open, count: node.querySelectorAll('.task-log-task').length,
    })));
    assert.deepEqual(groups, [
      { key: 'group:current', open: true, count: 4 }, { key: 'group:available', open: true, count: 1 },
      { key: 'group:locked', open: false, count: 1 }, { key: 'group:history', open: false, count: 3 },
    ]);
    assert.equal(await page.locator('.task-log-task').first().getAttribute('data-task-log-key'), 'task:journal-reward-available');
    assert.equal(await page.locator('.task-log-stage').count(), 1);
    assert.equal(await page.locator('.task-log-objective').count(), statuses.length);
    assert.equal(await page.locator('.task-log-body .task-log-stage').count(), 0);
    assert.equal(await page.locator('.task-log-body button').count(), 2);
    assert.ok(!(await page.locator('#task-log-list').textContent()).includes('[task-'));
    const activeTask = page.locator('[data-task-log-key="task:journal-active"]');
    assert.equal(await activeTask.locator('summary .task-log-stage').isVisible(), true, 'multi-stage progress is visible without expanding');
    assert.equal(await activeTask.locator('.task-log-description').isVisible(), false);
    assert.equal(await activeTask.locator('.task-log-name').textContent(), locale === 'zh-CN' ? '测试任务（测试城镇）' : 'Test task (Test town)');
    assert.equal(await activeTask.locator('.task-log-objective').textContent(), locale === 'zh-CN' ? '进度：4/12' : 'Progress: 4/12');
    await activeTask.locator('summary').focus();
    await page.keyboard.press('Enter');
    assert.equal(await activeTask.evaluate((el) => el.open), true);
    await activeTask.locator('button').focus();
    await page.evaluate(({ tasks, locale }) => window.menuFixture.renderTasks(tasks, locale), {
      tasks: taskEntries.map((task) => ({ ...task, current: task.current + 1 })), locale,
    });
    assert.equal(await activeTask.evaluate((el) => el.open), true);
    assert.equal(await activeTask.locator('button').evaluate((el) => el === document.activeElement), true);
    assert.equal(await activeTask.locator('.task-log-objective').textContent(), locale === 'zh-CN' ? '进度：5/12' : 'Progress: 5/12');
    await page.keyboard.press('Enter');
    assert.equal(await page.evaluate(() => window.menuFixture.abandonedTasks.at(-1)), 'journal-active');
    await page.evaluate(({ tasks, locale }) => window.menuFixture.renderTasks(tasks, locale, true), { tasks: taskEntries, locale });
    assert.equal(await activeTask.locator('button').isDisabled(), true);
    const locked = page.locator('[data-task-log-key="group:locked"]');
    await locked.locator(':scope > summary').focus();
    await page.keyboard.press('Space');
    assert.equal(await locked.evaluate((el) => el.open), true);
    await locked.locator('.task-log-task > summary').click();
    assert.ok((await locked.locator('.task-log-description').textContent()).includes(locale === 'zh-CN' ? '暂无' : 'No task'));
    await page.evaluate(({ tasks, locale }) => window.menuFixture.renderTasks(tasks, locale), { tasks: taskEntries, locale });
    assert.equal(await locked.evaluate((el) => el.open), true);
    for (const [width, height] of [[3840, 2160], [2560, 1440], [1280, 720], [640, 480], [320, 360]]) {
      await page.setViewportSize({ width, height });
      const layout = await page.locator('#task-log-panel').evaluate((panel) => ({
        columns: getComputedStyle(panel.querySelector('.task-log-items')).gridTemplateColumns.split(' ').length,
        fits: [...panel.querySelectorAll('.task-log-task, summary, .task-log-body')].filter((node) => node.getClientRects().length)
          .every((node) => node.scrollWidth <= node.clientWidth + 1),
      }));
      assert.equal(layout.columns, width >= 1440 ? 2 : 1);
      assert.equal(layout.fits, true, `${locale} ${width}: task content wraps within its columns`);
      if (locale === 'zh-CN' && [2560, 320].includes(width)) await page.screenshot({ path: fileURLToPath(new URL(`task-journal-layout-${width}.png`, artifacts)) });
    }
    await page.setViewportSize({ width: 2560, height: 1440 });
    await page.locator('#player-page-dialog').evaluate((dialog) => { dialog.style.width = '900px'; });
    assert.equal(await page.locator('.task-log-items').first().evaluate((el) => getComputedStyle(el).gridTemplateColumns.split(' ').length), 1,
      'a narrow panel stays single-column even on a wide screen');
    await page.locator('#player-page-dialog').evaluate((dialog) => { dialog.style.removeProperty('width'); });
    assert.equal(await page.locator('.task-log-items').first().evaluate((el) => getComputedStyle(el).gridTemplateColumns.split(' ').length), 2);
    await activeTask.locator('button').focus();
    await page.evaluate(({ tasks, locale }) => window.menuFixture.renderTasks(tasks, locale), {
      tasks: taskEntries.map((task) => task.status === 'active' ? { ...task, status: 'completed' } : task), locale,
    });
    assert.equal(await page.locator('[data-task-log-key="group:history"] > summary').evaluate((el) => el === document.activeElement), true);
  }
  console.log('Task journal: nine statuses, priority/group counts, native keyboard expansion, descriptions, optional stages/retakes, disabled actions, refresh focus and bilingual responsive columns passed.');
  await page.evaluate(async (sources) => {
    const [{ StatusPanel }, { createAppDom }, { AppState }, { Localization }] = await Promise.all([
      import('/src/status-panel.ts'), import('/src/app-dom.ts'), import('/src/app-state.ts'), import('/src/localization.ts'),
    ]);
    const state = new AppState();
    const localization = new Localization('zh-CN', sources);
    const commands = [];
    window.menuFixture.statusCommands = commands;
    const panel = new StatusPanel({ dom: createAppDom(document), state, localization,
      dispatch: async (command) => { commands.push(command); }, contentName: (id) => id ?? '', statusName: (id) => id ?? '',
      selectItemTarget: () => {}, startAbilityTargeting: () => {}, reconcileTargeting: () => {},
      renderTargeting: () => {}, refreshInventoryActions: () => {},
    });
    const snapshot = {
      turn: 1, worldTick: 0, worldId: 'fixture', floorId: 'fixture', mapScale: 'local', width: 1, height: 1,
      cells: [], entities: [], bodySlots: [], equipment: [], inventory: [], items: [], goldPiles: [], tasks: [], stateHash: 'fixture',
      campaign: { status: 'active', score: 0, conqueredDungeons: 0, completedTasks: 0 },
      player: { name: 'Test', hp: 10, maxHp: 10, gold: 0, isDead: false, attack: 1, defense: 0, armorClass: 0, speed: 110,
        position: { x: 0, y: 0 }, nutrition: 5000, nutritionState: 'normal', equipmentModifiers: { attack: 0, defense: 0, maxHp: 0 },
        statuses: [], virtues: [], abilities: [], resources: [],
      },
    };
    window.menuFixture.renderAbilities = (locale, mode) => {
      localization.setLocale(locale);
      localization.localizeDocument(document.getElementById('player-page-dialog'));
      snapshot.player.resources = mode === 'none' ? [] : [{ nameKey: 'fixture-task-name', current: 10, maximum: 20, restRecoveryAmount: 1, waitRecoveryAmount: 0 }];
      snapshot.player.abilities = mode === 'hidden' ? [{ id: 'hidden', uiGroupNameKey: 'fixture-task-name', minimumLevel: 10 }]
        : mode !== 'full' ? [] : Array.from({ length: 12 }, (_, i) => ({
          id: `ability-${i}`, nameKey: 'fixture-task-name', descriptionKey: i % 2 ? 'fixture-weapon-long' : 'fixture-task-description',
          bookNameKey: i < 6 ? 'fixture-task-name' : 'fixture-weapon-long', bookRank: i < 6 ? 1 : 2,
          bookItemId: 'fixture-book', minimumLevel: 1, resourceCost: i + 1, baseResourceCost: i + 1, failurePercent: 20,
          proficiency: 0, proficiencyCap: 100, proficiencyRank: 'unskilled', castCount: 0, failCount: 0,
          source: 'class', effects: [], canStudy: i % 2 === 0, canForget: false, canCast: i % 2 === 1, targetSpec: { modes: ['self'] },
        }));
      panel.render(snapshot);
      window.menuFixture.layout.open('ability');
    };
    window.menuFixture.renderActionTasks = (busy, world) => {
      state.busy = busy;
      snapshot.mapScale = world ? 'world' : 'local';
      snapshot.tasks = ['active', 'paused'].map((status) => ({ taskId: status, status, nameKey: 'fixture-task-name',
        current: 0, required: 1, stage: 1, stages: 1, retakesUsed: 0 }));
      panel.render(snapshot);
      window.menuFixture.layout.open('tasks');
    };
  }, sources);
  for (const locale of ['zh-CN', 'en-US']) {
    for (const mode of ['none', 'resource', 'hidden', 'full']) {
      await page.evaluate(([locale, mode]) => window.menuFixture.renderAbilities(locale, mode), [locale, mode]);
      assert.equal(await page.locator('#ability-list .ability-empty').count(), mode === 'full' ? 0 : 1);
      assert.equal(await page.locator('#resource-list .resource-row').count(), mode === 'none' ? 0 : 1);
      if (mode !== 'full') continue;
      assert.equal(await page.locator('.ability-book-heading').count(), 2);
      for (const [width, height] of [[2560, 1440], [1280, 720], [640, 480], [320, 360]]) {
        await page.setViewportSize({ width, height });
        const metrics = await page.locator('#ability-panel').evaluate((panel) => ({
          fits: [...panel.querySelectorAll('.ability-row, .ability-details, .ability-actions')].every((node) => node.scrollWidth <= node.clientWidth + 1),
          ends: [...panel.querySelectorAll('.ability-actions')].map((node) => node.getBoundingClientRect().right),
          columns: getComputedStyle(panel.querySelector('.ability-row')).gridTemplateColumns.split(' ').length,
        }));
        assert.equal(metrics.fits, true, `${locale} ${width}: real ability controls fit`);
        assert.equal(new Set(metrics.ends).size, 1, `${locale} ${width}: action columns align`);
        assert.equal(metrics.columns, width <= 640 ? 1 : 2);
        if (locale === 'zh-CN' && [2560, 320].includes(width)) await page.screenshot({ path: fileURLToPath(new URL(`ability-layout-${width}.png`, artifacts)) });
      }
    }
  }
  console.log('Production ability renderer: empty/resource-only/level-filtered states, preserved book grouping, variable descriptions, disabled controls and bilingual aligned actions passed.');
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.locator('.ability-actions button').filter({ hasText: 'Study' }).first().click();
  await page.locator('.ability-cast-action:not(:disabled)').first().click();
  assert.deepEqual(await page.evaluate(() => window.menuFixture.statusCommands), [
    { type: 'study-ability', bookItemId: 'fixture-book', abilityId: 'ability-0' },
    { type: 'cast-ability', abilityId: 'ability-1', target: { type: 'self' } },
  ]);
  for (const [busy, world] of [[true, false], [false, true], [false, false]]) {
    await page.evaluate(([busy, world]) => window.menuFixture.renderActionTasks(busy, world), [busy, world]);
    for (const status of ['active', 'paused']) {
      const task = page.locator(`[data-task-log-key="task:${status}"]`);
      if (!(await task.evaluate((node) => node.open))) await task.locator('summary').click();
      assert.equal(await task.locator('button').isDisabled(), busy || world);
      if (!busy && !world) await task.locator('button').click();
    }
  }
  assert.deepEqual(await page.evaluate(() => window.menuFixture.statusCommands.slice(2)), [
    { type: 'abandon-task' }, { type: 'abandon-paused-task', taskId: 'paused' },
  ]);
  console.log('Action acceptance: study/cast command payloads, active/paused abandon commands, busy/world-map disabling passed.');
  // Main-screen typography uses the real shell and message renderer, without a game save.
  await page.evaluate(async (sources) => {
    const { Localization } = await import('/src/localization.ts');
    const { MessagePanel } = await import('/src/message-panel.ts');
    const { renderHudExperience } = await import('/src/status-panel.ts');
    window.menuFixture.layout.closePage();
    window.menuFixture.renderHud = (locale, full) => {
      const localization = new Localization(locale, sources);
      localization.localizeDocument(document);
      const zh = locale === 'zh-CN';
      const text = (id, value) => { document.getElementById(id).textContent = value; };
      text('progression-identity-value', zh ? '测试角色 人类战士' : 'Test Character Human Warrior');
      text('progression-level-value', '27 / 50');
      text('hud-location-value', zh ? '测试地牢 · 深度 3 / 20' : 'Test dungeon · Depth 3 / 20');
      renderHudExperience(document.getElementById('hud-experience'),
        { experience: 25n, experienceForNextLevel: 100n }, localization);
      text('hp-value', '241 / 241');
      text('gold-value', '14288');
      text('nutrition-value', '99%');
      text('light-value', zh ? '油灯 1234' : 'Lantern 1234');
      text('nearby-current', zh ? '脚下：林间草地' : 'Underfoot: forest grass');
      text('effects-value', zh ? '集中 · 加速' : 'Focused · Hasted');
      document.getElementById('hud-attribute-list').innerHTML =
        (zh ? ['力量', '智力', '感知', '敏捷', '体质', '魅力'] : ['STR', 'INT', 'WIS', 'DEX', 'CON', 'CHR'])
          .map((name) => `<li class="attribute-row"><span class="attribute-name">${name}</span><span class="attribute-value">18/20</span></li>`).join('');
      document.getElementById('resource-list').innerHTML = full
        ? Array.from({ length: 6 }, () => `<li class="resource-row"><div class="resource-heading"><span class="resource-name">${zh ? '法力' : 'Mana'}</span><strong class="resource-value">260 / 267</strong></div></li>`).join('')
        : `<li class="resource-empty">${zh ? '当前没有其他资源。' : 'No other resources.'}</li>`;
      document.getElementById('nearby-list').innerHTML = full
        ? Array.from({ length: 25 }, () => `<li class="nearby-row"><span class="nearby-glyph">!</span><div class="nearby-details"><strong>${zh ? '治疗轻伤药水' : 'Potion of Cure Light Wounds'}</strong><span>${zh ? '东 · 2 格 · 数量 2' : 'East · 2 tiles · Quantity 2'}</span></div></li>`).join('') : '';
      const messages = new MessagePanel({ list: document.getElementById('message-list'), localization,
        formatEvent: () => '', currentTurn: () => zh ? '50 · 第 1 天 06:07 · 白昼' : '50 · Day 1 06:07 · Daylight',
        localizedArgs: () => undefined, historyLimit: 100 });
      messages.clear();
      if (full) for (let i = 0; i < 30; i++) messages.addLocalized('action-resource-rest', undefined, 'info');
      document.querySelector('.hud-menu').open = full;
    };
  }, sources);
  for (const locale of ['zh-CN', 'en-US']) {
    for (const full of [false, true]) {
      await page.evaluate(([locale, full]) => window.menuFixture.renderHud(locale, full), [locale, full]);
      for (const [width, height, expected] of [[1280, 720, 15], [1920, 1080, 18], [2560, 1440, 22], [3840, 2160, 26], [900, 620, 15], [2560, 1440, 22]]) {
        await page.setViewportSize({ width, height });
        const shellBeforeMenu = await page.locator('.app-header, #map-host, .shortcut-bar').evaluateAll((nodes) =>
          nodes.map((node) => node.getBoundingClientRect().toJSON()));
        await page.locator('.hud-menu > summary').click();
        const shellAfterMenu = await page.locator('.app-header, #map-host, .shortcut-bar').evaluateAll((nodes) =>
          nodes.map((node) => node.getBoundingClientRect().toJSON()));
        assert.deepEqual(shellAfterMenu, shellBeforeMenu, `${locale} ${width}: toggling the HUD menu must not resize or shift the shell`);
        if (!full) {
          const menu = await page.locator('.hud-menu-content').boundingBox();
          assert.ok(menu.x >= 0 && menu.x + menu.width <= width && menu.y + menu.height <= height,
            `${width}: menu overlay remains in the viewport`);
          assert.ok(await page.locator('#player-ui-settings-open').isVisible());
        }
        await page.locator('.hud-menu > summary').click();
        const hud = await page.evaluate(() => {
          const el = (selector) => document.querySelector(selector);
          const map = el('#map-host').getBoundingClientRect();
          return {
            font: parseFloat(getComputedStyle(el('#hp-value')).fontSize),
            fits: ['html', '#app', '.game-layout', '.play-column'].every((selector) => {
              const node = el(selector);
              return node.scrollHeight <= node.clientHeight + 1 && node.scrollWidth <= node.clientWidth + 1;
            }),
            mapHeight: map.height,
            bottom: el('.shortcut-bar').getBoundingClientRect().bottom,
            fullHeight: el('#app').getBoundingClientRect().bottom,
            rightFilled: Math.abs(el('#hud-vitals-host').getBoundingClientRect().right - el('#app').getBoundingClientRect().right) <= 1,
            meter: el('#hud-experience').value,
            timeLines: el('.message-turn') ? el('.message-turn').getBoundingClientRect().height / parseFloat(getComputedStyle(el('.message-turn')).lineHeight) : 0,
            rowsFit: [...document.querySelectorAll('.nearby-row, #resource-list .resource-row')].every((row) =>
              [...row.querySelectorAll('strong, .nearby-details span, .resource-value')].every((child) =>
                child.getBoundingClientRect().bottom <= row.getBoundingClientRect().bottom + 1)),
            localScroll: ['#nearby-list', '#message-list', '#resource-list'].every((selector) => {
              const node = el(selector);
              return (node.clientHeight > 0 || (selector === '#resource-list' && node.querySelector('.resource-empty')))
                && node.scrollWidth <= node.clientWidth + 1;
            }),
          };
        });
        assert.ok(Math.abs(hud.font - expected) < 0.02, `${width}: HUD font ${hud.font}`);
        assert.ok(hud.fits && Math.abs(hud.bottom - hud.fullHeight) <= 1, `${locale} ${width} ${full}: fixed full-height shell`);
        assert.ok(hud.rightFilled && hud.meter === 250, 'vitals fill the old menu column and XP is projected');
        assert.ok(hud.mapHeight > height * 0.3, `${locale} ${width} ${full}: map retains useful height (${hud.mapHeight})`);
        assert.ok(hud.localScroll && hud.rowsFit && hud.timeLines <= 2.1, `${locale} ${width}: readable non-overlapping local lists and timestamp`);
        if (locale === 'zh-CN' && [1280, 2560].includes(width)) await page.screenshot({ path: fileURLToPath(new URL(`hud-typography-${width}-${full ? 'full' : 'empty'}.png`, artifacts)) });
      }
    }
  }
  assert.deepEqual(errors, []);
  console.log('HUD typography: bilingual 720p/1080p/1440p/4K and restore/maximize-sized viewports, empty/full logs and six resources, readable timestamps and full-height map shell passed.');
} finally {
  await browser?.close();
  await server.close();
}
