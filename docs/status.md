# 状态快照

核对日期：2026-09-10。本次集成包含种族主线 `decec518a` 、法术道具 `934a83392` 与地牢城镇 `78a9cc982`；本页记录已合入的代码/配置事实和注明范围的验收证据。当前数值以链接的源文件为准。

## 版本与源内容

| 项目 | 快照值 | 依据 |
| --- | --- | --- |
| 应用版本 | 0.1.0 | [Cargo.toml](../Cargo.toml)、[Tauri 配置](../web/src-tauri/tauri.conf.json) |
| 协议 | 1.241 | [协议常量](../crates/rfb-protocol/src/lib.rs) |
| State Hash Schema | 118 | [核心常量](../crates/rfb-core/src/game/mod.rs) |
| save header / payload / 容器 | 13 / 13 / 1 | [协议常量](../crates/rfb-protocol/src/lib.rs)、[rfb-save](../crates/rfb-save/src/lib.rs) |
| 内容包 | 1.402.0 | [pack](../packs/rfb-demo-original/pack.json)、[lock](../packs/rfb-demo-original/content.lock.json) |
| 契约政策 | contract-v319，26 条 active scenario | [baseline-policy.json](../tests/fixtures/active/baseline-policy.json)与[场景目录](../tests/fixtures/active/scenarios/) |

正式源目录含 6 个 Class、13 个 Build、57 个 Race、32 本能力书、1,840 个 ability 文件、370 个 item、1,402 个 actor、168 个 affix、152 个 mutation。世界定义含 25 个 dungeon 条目；城镇源目录有 6 个 town、60 个 shop、62 个 townFacility。这些是定义/源文件数量，不是完整规则或已验收内容数量。

权威内容统计工具是 `rfb-contentc inspect-source`。本次集成已运行内容编译；静态统计不替代行为验收。

## 玩家入口

新游戏白名单在 [session-shell.ts](../web/src/session-shell.ts)，表单在 [web/index.html](../web/index.html)。当前开放 6 个构筑、46 个种族：

| 构筑 | 稳定 Build ID | 范围 |
| --- | --- | --- |
| 战士 | `demo.build.warrior` | 非施法基线 |
| 高阶法师（死亡） | `demo.build.high-mage-death` | 死亡领域书本与施法 |
| 弓箭手 | `demo.build.archer` | 制造弹药与射击 |
| 圣骑士（死亡） | `demo.build.paladin-death` | 死亡领域与随机祈祷学习 |
| 骑兵 | `demo.build.cavalry` | 骑乘相关行为 |
| 狙击手 | `demo.build.sniper` | 专注与特殊射击 |

Death、Arcane、Sorcery、Armageddon、Nature、Life、Daemon、Crusade 各有四册内容、领域 Build 和相关规则测试路径；当前新游戏仅开放 Death。其余七领域应补入口与相应流程验证，而非从头重做导入。

托姆特（`rfb-legacy.race.tomte`）已进入正式新游戏白名单。六个当前职业的出生装备合并、知识美德及核心探测、头饰惩罚、39/40 级感知鉴定、拾取、保存恢复和继续行动已验证；桌面抽查战士、死亡高阶法师和弓箭手。其他 Race 仍按定义、入口与实际验收范围区分。

冬贝利（`rfb-legacy.race.tonberry`）已进入正式新游戏白名单。六职业正式出生、军刀熟练度与“独立”美德、成长被动和攻次／混乱抗性边界已有核心测试；完整核心链为装备军刀、升至 10 级、命中、换回原武器、保存恢复并继续行动。普通武器配置在较高等级可能降至零攻次，界面明确提示该下限。尚缺的决斗者／重槌兵／灵能者关联、死神镰刀武器反噬、神器 247 专属掉落、种族首领和变形怪选择关联，见 `602a33a75` 的原版审计，不计入本批完成范围。

世界中存在 Outpost、Anambar、Thalos、Morivant、Telmora、Angwil 六个城镇记录及多种地牢条目。条目存在不证明所有原版设施、守卫、任务链和完整通关已经验证；实际地点进入条件、替代关系和获取路径以运行时与本批测试为准。

Ent、Spectre 的新游戏入口已开放。种族主线还接入原始经验值与种族等级阈值、永久种族变更、生命力耗尽后的转种族/死亡和相关界面投影；实现与专项测试见 `084f341f0`、`4359de538`、`decec518a`。已有桌面验收只证明各提交记录的范围，本次集成不重复宣称桌面或人工试玩通过。

## 物品与共享生成

法术道具分支已合入护甲、非 Craft Ego、共享加权工艺、真实装备估值、负向装备/诅咒消费者、龙系底材生成、背包与箭袋容量，以及随机神器实例身份和消费者。168 个 affix 定义不等于全部均可自然获取：保留原版零稀有度及专用入口约束。

随机神器已接入武器/挖掘工具、远程/竖琴、护甲、首饰和费艾诺光源的自然调度，保留原版底材例外、概率与强制模式，Craft 不参与抽样。实例保存名称、骰数、重量、特性、诅咒和激活；内部工厂含职业/主题 bias、原版命名、195 项激活池及最多 1001 个独立候选的价值筛选。成功和被拒绝候选的名字登记进入存档，恢复后继续生成保持一致。首饰自然入口另按原版等级/模式阈值筛选完整候选，1000 次拒绝后直接采用新生成的第 1001 个结果；每轮都包含 Ego/随机神器分支及诅咒，显式配置物化保持原入口。生成验收要求规则相同和本项目内确定性，不要求与原版同种子产物相同。无权威中文的名字保留原字符串并记录 unresolved。八领域入口范围保持上表状态。实现范围见[共享生成计划](../design/ego-shared-generation-plan.md)，实例表示见[随机神器身份契约](../design/contract-v316-random-artifact-identity.md)。

集成保留物品感知与实例神器鉴定边界、托姆特实例头饰重量、冬贝利逐武器伤害/攻次和准确来源显示。种族永久状态与新增物品字段共同进入当前保存和状态哈希，版本统一收口；内容 hash 本身不参与状态哈希。

## 城镇与共享存储

地牢城镇分支已合入 Morivant、Telmora、Angwil 的正常旅行入口、商店和设施，城镇大地图布局与荒野衔接、按名望调整服务价格、分档强化、赌场和博物馆跨角色共享存储。博物馆转移通过 Tauri 的存储事务与角色检查点一同提交；固定神器禁止捐赠和导入，随机神器实例允许共享并保留完整属性。家与博物馆复用背包详情投影，保留物品身份、知识与实例重量。

Dr. Jones 的鞭子使用原版神器 162、隔空取物与 300 tick 冷却；新内容与规则来源为 RFB master `a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`。城镇强化复用共享 COST_REAL 估价，并按原版清除估价副本的诅咒影响。单次装备激活按自身经过时间恢复，普通装置保留再生 Ego 加成。

任务审计中的其他依赖与未实现链条仍是后续工作，不因城镇入口开放而计为完成。详见 [Morivant 适配](morivant-town-adaptation.md)、[共享博物馆](shared-museum.md)与本批提交；本次自动验证不替代桌面或 Android 人工试玩。

## 已有验收证据

本次三线集成的冲突回归已修复；末轮城镇/估价/木精灵专项 74 项、估价与 Camelot 保存专项 8 项、Tauri 原生层 23 项、前端相关 71 项及 26 条 active 契约通过。内容、导入器、本地化、协议、保存与回放测试通过，相关 Clippy、类型检查、生成文件和内容锁检查通过。固定神器估价对照同时补齐 `init1.c` 自动添加的四种元素免毁标记；未重建 standalone 或运行桌面 E2E/Android。

| 记录 | 能证明的范围 | 不扩展到 |
| --- | --- | --- |
| `97e8d6719` 托姆特专项 | 六职业正式出生和核心操作链；Windows Tauri standalone WebDriver 包的战士、死亡高阶法师、弓箭手新建、中文说明、头饰提示、探测、摘戴帽子、保存恢复和继续行动；相关测试、Clippy、内容锁和 26 条契约通过 | 升级和感知拾取链仅作核心自动测试；未声明全种族、全原版或 Android 验收。运行 `node e2e/tauri.e2e.mjs --tomte`，本机报告与截图在 `test-results/tomte-*` |
| `ad8c1a3a5` 冬贝利专项 | 六职业正式出生与核心装备、10 级升级、命中、换装、保存恢复、继续行动；Windows Tauri standalone WebDriver 的战士、死亡高阶法师、弓箭手完成新建、中文说明与六条特性列表、说明无障碍关联、武器装卸、精确保存恢复和继续行动；相关测试、Clippy、内容锁和 26 条契约通过 | 升级和命中链仅作核心自动测试；原版关联缺口沿用 `602a33a75`。运行 `node e2e/tauri.e2e.mjs --tonberry`，本机报告与截图在 `test-results/tonberry-*`，独立测试产物在 `target/e2e/debug/rfb-tauri.exe` |
| `3d127279c` 的 2026-09-09 桌面记录 | 同源 Tauri WebDriver 调试包的战士、死亡高阶法师相关新游戏/菜单/装备/学习施法/保存恢复流程；优化 standalone 构建和启动 | 优化 EXE 全部交互、全职业/种族/法术或 Android 真机 |
| `ad0bd6c6a` 六步核心重构记录 | 当时核心 863 项、回放 8 项、前端 180 项，workspace/Clippy、Tauri 检查、生成文件/内容锁及 26 条 exact fixture 通过 | 后续代码自动继承这些测试结果 |
| `b84da4ef1`、`dec5bd0fb` | 已提交的 Tomte 规则及其相关测试代码；行为细节可查提交与源文件 | 本次文档整理没有重新执行它们，也没有证明玩家入口开放 |

原始桌面证据见[归档验收记录](archive/2026-09-09/design/playable-release-20260909.md)，重构证据见[归档执行记录](archive/2026-09-09/design/core-large-file-refactor-plan.md)。忽略目录中的机器日志可能已被用户清理；归档记录保留当时结论，不伪称现有缓存仍在。

## 更新口径

只记录发生变化的范围，并注明提交和证据：

- **内容已定义**：源内容存在且引用关系可检查。
- **规则已实现**：运行路径实际执行所需行为，并注明测试范围。
- **玩家入口已开放**：正常菜单或游戏操作可达。
- **实际验收通过**：给出提交、平台、角色/内容、具体操作和结果。

“未验收”不等于已发现故障；“历史待办”也不等于当前仍缺失。下一批建议见[后续工作](next-work.md)。
