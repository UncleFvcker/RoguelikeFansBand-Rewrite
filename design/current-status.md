# 当前状态

核对日期：2026-09-09。代码与测试基线以当前提交及下表版本为准。本文是当前能力、玩家入口和验收范围的统一记录；历史 contract、阶段方案和分支交接中的“当前”只指各自记录时点。

## 状态口径

| 状态 | 判定依据 | 不代表什么 |
| --- | --- | --- |
| 内容已定义 | 正式包中存在可加载的定义与引用 | 不代表规则已执行，也不代表玩家能选到 |
| 规则已实现 | 有实际运行路径，并注明核心测试覆盖范围 | 不代表完整原版行为或玩家操作流程已验收 |
| 玩家入口已开放 | 当前新游戏菜单或游戏内正常操作可达 | 不代表该入口下所有内容都已验收 |
| 实际验收通过 | 注明日期、版本、环境、具体操作及结果 | 不从一个职业、法术或平台推广到其他范围 |

“未验收”表示没有本次范围内的通过证据，不等同于功能故障。后续人工试玩由用户执行；代码检查和自动测试分别记录，不记作人工试玩。

## 当前基线与内容定义

| 项目 | 值 | 来源 |
| --- | --- | --- |
| 协议 | 1.232 | [协议常量](../crates/rfb-protocol/src/lib.rs) |
| State Hash Schema | v110 | [核心常量](../crates/rfb-core/src/game/mod.rs) |
| save header / payload | v5 / v5；容器 v1 | [协议常量](../crates/rfb-protocol/src/lib.rs)、[存档格式](save-format-v1.md) |
| 内容包 | 1.388.0 | [pack.json](../packs/rfb-demo-original/pack.json)、[content.lock.json](../packs/rfb-demo-original/content.lock.json) |
| 行为基线 | contract-v308，26 个 exact fixture | [baseline-policy.json](../tests/fixtures/active/baseline-policy.json) |
| 内容定义数量 | 地形 201、角色 1402、物品 363、能力 1838、词缀 136、能力书 32、掉落表 34、变异 152 | [正式内容目录](../packs/rfb-demo-original/) |
| 角色配置数量 | Class 6、Build 13、Race 57、SkillSet 65 | 同上；这些是定义数量，不是菜单选项数量 |

版本与哈希以源文件为准。玩家入口以 [PLAYTEST_BUILD_IDS / PLAYTEST_RACE_IDS](../web/src/session-shell.ts) 和 [新游戏表单](../web/index.html) 为准。

## 职业与玩家入口

当前新游戏提供 **6 个职业构筑、42 个种族**。种族定义总数 57 不代表全部开放；具体可选 ID 见上述入口列表与[职业种族交接](class-race-import-handoff.md)。

| 职业范围 | 内容已定义 | 规则已实现的范围与证据 | 玩家入口已开放 | 2026-09-09 实际验收 |
| --- | --- | --- | --- | --- |
| 战士 | Class 与 Build | 非施法出生、装备与行动；[核心测试](../crates/rfb-core/src/game/tests/) | `demo.build.warrior` | 同源桌面自动验收通过新游戏、菜单、装备、保存恢复及继续行动 |
| 高阶法师：死亡 | Class、Build、4 册 / 32 法术 | 出生、学习、施法及领域规则；[high_mage.rs](../crates/rfb-core/src/game/tests/high_mage.rs) | `demo.build.high-mage-death` | 种子 7：学习并成功施放“探测非生命体”，保存恢复及继续行动；不代表 32 法术逐项桌面验收 |
| 高阶法师：其余七领域 | 每领域有 Build、4 册 / 32 法术 | 已有领域运行路径、出生隔离及领域行为核心测试；同上 | **未开放新游戏入口** | 未做这七个领域的玩家流程验收 |
| 弓箭手 | Class 与 Build | 制造弹药与射击；[archer.rs](../crates/rfb-core/src/game/tests/archer.rs) | `demo.build.archer` | 本次未验收 |
| 圣骑士：固定死亡 | Class 与 Build | 祈祷学习和职业能力；[paladin.rs](../crates/rfb-core/src/game/tests/paladin.rs) | `demo.build.paladin-death` | 本次未验收 |
| 骑兵 | Class 与 Build | 骑乘、坐骑成长和捕获球；[cavalry.rs](../crates/rfb-core/src/game/tests/cavalry.rs)、[riding.rs](../crates/rfb-core/src/game/tests/riding.rs) | `demo.build.cavalry` | 本次未验收 |
| 狙击手 | Class 与 Build | 专注、特殊射击和探测；[sniper.rs](../crates/rfb-core/src/game/tests/sniper.rs) | `demo.build.sniper` | 本次未验收 |

### 高阶法师八领域

八个领域均有四册、32 个法术定义，共 32 册、256 个法术；“四册内容及规则已接入”不写成“八领域都已向玩家开放”。

| 领域 | Build ID 后缀（前缀 `demo.build.high-mage-`） | 新游戏入口 |
| --- | --- | --- |
| 死亡 | `death` | 已开放 |
| 奥秘 | `arcane` | 未开放 |
| 咒术 | `sorcery` | 未开放 |
| 毁灭 | `armageddon` | 未开放 |
| 自然 | `nature` | 未开放 |
| 生命 | `life` | 未开放 |
| 恶魔 | `daemon` | 未开放 |
| 圣战 | `crusade` | 未开放 |

内容引用见 [builds](../packs/rfb-demo-original/builds/) 和 [abilityBooks](../packs/rfb-demo-original/abilityBooks/)。领域身份、源码核对与机制边界见[法术领域交接](spell-realm-import-handoff.md)。未开放领域需要单独安排入口变更与玩家流程验收，本文不改变开放范围。

## 版本验收与限制

2026-09-09，`codex/realms-items` 完成 E5 后半 48 条，头冠、头盔、披风、手套和靴子接入后，
[护甲 Ego 审计](armor-ego-import-audit.md)中的 76 条均有生成分支、底材限制、随机属性和激活处理。
75 条可自然生成；“戒灵的”保留原版稀有度 0，仅显式物化。权威来源为 RFB `master`
`a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`，中文名及激活名直接读取源版表和运行时字符串。
本批新增铁王冠、秘银铁头靴底材，并接入冰电/复仇光环、夜视、法力恢复、反魔法、魔法减伤、禁止附魔，
以及护甲命中/伤害的近战、射击、法术分流。源氏手套作用于实际双持：副手槽可装备武器，两手独立
结算，使用原版重量与熟练度公式；六职业的双持上限来自 `master:s_info.txt`，训练状态进入存档。
精灵斗篷的基础与词缀属性共用 pval，女巫帽的随机元素保护及全部动态词缀属性均保存为实例状态。

19 个护甲专项测试覆盖全部自然生成池、76 条各 48 个种子的存档往返、底材限制及主要装备消费者。
新增存档字段、公共属性投影和生成 RNG 行为变更使用 State Hash Schema v110，刷新并复验 26 条
active fixture。前端支持副手武器选择及新增属性展示。本批未做桌面或人工试玩，不记作玩家流程验收。

2026-09-09，`codex/realms-items` 从 `main@62f959f3b` 增补鹤嘴锄（`demo.item.mattock`，
RFB master `a0d92b6378d148c5262cc236b8fa6ed2ca06a54c` 的 k_info 156）：正式物品数增加至 357，
内容包为 1.385.0。规则复用已有挖掘、DIGGER ego 选择/物化与化石为泥激活；普通掉落在生成等级
50 起可获得，权重为 100。核心测试已覆盖工具槽挖掘加成 85、自然生成瓦解词条、固定随机属性、
保存恢复、取消目标、定向移除岩石及 50 tick 充能恢复。中文名直接采用 `kind_name_zh.inc` 的
“鹤嘴锄”，像素映射复用已有挖掘工具图块。本批未做桌面或人工试玩，也未扩大领域入口。

| 环境 / 检查 | 已通过范围 | 证据或限制 |
| --- | --- | --- |
| 2026-09-09，同源 Tauri WebDriver 调试包 | 上表战士 / 死亡高阶法师流程；菜单、镜头、缩放、外观、回放导出与异常报告 | [验收记录](playable-release-20260909.md)、[可执行场景](../web/e2e/tauri.e2e.mjs)；机器结果在忽略目录 `test-results/playable-acceptance.json` |
| 同次源码检查 | 前端 180 项；Rust workspace（排除 Tauri 壳）；Clippy；协议 / 内容检查；26 个已提交 fixture 的完整回放 | 通过仅证明所列检查；不等于所有职业和玩法完成 |
| 正式 Windows x64 standalone，提交 `3d127279c` | 优化构建成功，EXE 启动并读到标题页，无 Vite 服务 | 正式 EXE 未完成五项交互复测；原生工具受限。五项操作证据来自同源调试包 |
| 人工试玩、全职业 / 全种族 / 全法术、完整通关、Android | 未在本次验收 | 需要后续明确范围及独立结果；独立补给循环和渲染性能实验也未在本次运行 |

城镇、荒野、地下城、任务、物品、怪物和变异已有内容与规则接入记录；本次五项流程验收不覆盖这些系统的所有内容。各专题中的历史“完成”保留其原有范围，不作为当前全量验收结论。

## 更新方式

- 内容变更：更新正式定义、包版本与 lock，并同步本页对应数量或范围。
- 规则变更：记录实际路径和相关测试，不把定义数量写成实现进度。
- 入口变更：以菜单白名单与正常操作链为依据，单独更新“玩家入口”。
- 验收完成：写明提交、平台、角色 / 领域、具体操作和结果；未覆盖部分保留“未验收”。
- README 只保留摘要并链接本页；专题文档保留实现细节，旧版本号和旧批次结果标明历史时点。
