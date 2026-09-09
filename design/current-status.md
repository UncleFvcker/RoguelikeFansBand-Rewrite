# 当前状态

核对日期：2026-09-09。版本表按 `codex/dungeons-towns` 当前源码更新；下述桌面里程碑验收基线仍为 `3d127279c`。本文是当前能力、玩家入口和验收范围的统一记录；历史 contract、阶段方案和分支交接中的“当前”只指各自记录时点。

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
| 协议 | 1.237 | [协议常量](../crates/rfb-protocol/src/lib.rs) |
| State Hash Schema | v111 | [核心常量](../crates/rfb-core/src/game/mod.rs) |
| save header / payload | v6 / v8；容器 v1 | [协议常量](../crates/rfb-protocol/src/lib.rs)、[存档格式](save-format-v1.md) |
| 内容包 | 1.384.16 | [pack.json](../packs/rfb-demo-original/pack.json)、[content.lock.json](../packs/rfb-demo-original/content.lock.json) |
| 行为基线 | contract-v306，26 个 exact fixture | [baseline-policy.json](../tests/fixtures/active/baseline-policy.json) |
| 内容定义数量 | 地形 210、角色 1402、物品 357、能力 1838、词缀 65、能力书 32、掉落表 34、变异 152 | [正式内容目录](../packs/rfb-demo-original/) |
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

| 环境 / 检查 | 已通过范围 | 证据或限制 |
| --- | --- | --- |
| 2026-09-09，同源 Tauri WebDriver 调试包 | 上表战士 / 死亡高阶法师流程；菜单、镜头、缩放、外观、回放导出与异常报告 | [验收记录](playable-release-20260909.md)、[可执行场景](../web/e2e/tauri.e2e.mjs)；机器结果在忽略目录 `test-results/playable-acceptance.json` |
| 同次源码检查 | 前端 180 项；Rust workspace（排除 Tauri 壳）；Clippy；协议 / 内容检查；26 个已提交 fixture 的完整回放 | 通过仅证明所列检查；不等于所有职业和玩法完成 |
| 正式 Windows x64 standalone，提交 `3d127279c` | 优化构建成功，EXE 启动并读到标题页，无 Vite 服务 | 正式 EXE 未完成五项交互复测；原生工具受限。五项操作证据来自同源调试包 |
| 人工试玩、全职业 / 全种族 / 全法术、完整通关、Android | 未在本次验收 | 需要后续明确范围及独立结果；独立补给循环和渲染性能实验也未在本次运行 |

城镇、荒野、地下城、任务、物品、怪物和变异已有内容与规则接入记录；本次五项流程验收不覆盖这些系统的所有内容。各专题中的历史“完成”保留其原有范围，不作为当前全量验收结论。

安格维尔已开放完整森林模板、九类商店、旅店四项服务、共享 Home 与博物馆。透明地形沿用荒野生成及城镇保存流程；核心测试覆盖滚屏、离城返回、读档、交易和服务收费。11 处 A2 建筑及庄园任务尚未开放，本批未做桌面或人工试玩验收；范围见[安格维尔计划](angwil-town-plan-20260909.md)。

莫里凡特已接入完整地图、九类商店、共享 Home、旅店和已支持建筑服务；巫术之塔的批量鉴定、盗贼公会的住宿/批量鉴定、旅店 2 金餐饮、驯兽师 1500 金怪物研究与王牌之塔指定地牢层传送已开放。报价、饱食和研究知识由 Rust 处理，研究资料和各地牢召回层可保存恢复；传闻因原文再分发许可未明确而保持隐藏。Rogue 职业身份和完整原版服务计价仍未接入。入口、来源、适配差异及核心检查范围见[城镇适配](../docs/morivant-town-adaptation.md)，后续安排见[非任务建筑服务计划](morivant-building-services-plan-20260909.md)；这些增量未计入上述桌面验收。

莫里凡特与萨洛斯博物馆已共享同一馆藏，支持存取、按已知信息检视、神器拒收和捐赠细节。Tauri 已接入本地资料中的跨角色馆藏事务、刷新、过期角色恢复和只读转移检查点；真实子进程验证了提交前后中断与竞争取出。存档绑定本地资料，从新角色开始；共享输入会开启新的回放段。详见[跨角色馆藏](../docs/shared-museum.md)。尚未进行本批桌面 UI E2E 或重建 standalone 产物。

2026-09-09 地牢城镇方向增量：按权威 RFB master 修正水晶城堡入口世界坐标为 `(37,40)`，将入口守卫从玩家落点移至相邻位置，补充进入、返回地表与保存恢复的核心测试；修复无固定奖励地牢的来源校验。竞技场地牢仅新增来源计划，正式入口尚未开放，生成规则与最终奖励仍有依赖。具体范围与验证见[本批交付记录](dungeons-towns-source-audit-20260909.md)，不计为桌面或人工试玩验收。

## 更新方式

- 内容变更：更新正式定义、包版本与 lock，并同步本页对应数量或范围。
- 规则变更：记录实际路径和相关测试，不把定义数量写成实现进度。
- 入口变更：以菜单白名单与正常操作链为依据，单独更新“玩家入口”。
- 验收完成：写明提交、平台、角色 / 领域、具体操作和结果；未覆盖部分保留“未验收”。
- README 只保留摘要并链接本页；专题文档保留实现细节，旧版本号和旧批次结果标明历史时点。
