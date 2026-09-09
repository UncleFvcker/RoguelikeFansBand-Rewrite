# 状态快照

核对日期：2026-09-09。源码快照：`ad8c1a3a5`（冬贝利开放与专项验收）。本页记录这一提交的代码/配置事实和既有验收证据；并行分支中的后续工作不自动算入。当前 HEAD 的数值以链接的源文件为准。

## 版本与源内容

| 项目 | 快照值 | 依据 |
| --- | --- | --- |
| 应用版本 | 0.1.0 | [Cargo.toml](../Cargo.toml)、[Tauri 配置](../web/src-tauri/tauri.conf.json) |
| 协议 | 1.234 | [协议常量](../crates/rfb-protocol/src/lib.rs) |
| State Hash Schema | 109 | [核心常量](../crates/rfb-core/src/game/mod.rs) |
| save header / payload / 容器 | 5 / 5 / 1 | [协议常量](../crates/rfb-protocol/src/lib.rs)、[rfb-save](../crates/rfb-save/src/lib.rs) |
| 内容包 | 1.390.0 | [pack](../packs/rfb-demo-original/pack.json)、[lock](../packs/rfb-demo-original/content.lock.json) |
| 契约政策 | contract-v307，26 条 active scenario | [baseline-policy.json](../tests/fixtures/active/baseline-policy.json)与[场景目录](../tests/fixtures/active/scenarios/) |

正式源目录含 6 个 Class、13 个 Build、57 个 Race、32 本能力书、1,838 个 ability 文件、356 个 item、1,402 个 actor、65 个 affix、152 个 mutation。世界定义含 25 个 dungeon 条目；城镇源目录有 3 个 town、30 个 shop、24 个 townFacility。这些是定义/源文件数量，不是完整规则或已验收内容数量。

权威内容统计工具是 `rfb-contentc inspect-source`。冬贝利开放批次已运行内容编译和锁验证；静态统计不替代行为验收。

## 玩家入口

新游戏白名单在 [session-shell.ts](../web/src/session-shell.ts)，表单在 [web/index.html](../web/index.html)。专项验收后开放 6 个构筑、44 个种族：

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

世界中存在 Outpost、Anambar、Thalos 的城镇记录及多种地牢条目。条目存在不证明所有原版设施、守卫、任务链和完整通关已经验证；实际地点进入条件、替代关系和获取路径以运行时与本批测试为准。

## 已有验收证据

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
