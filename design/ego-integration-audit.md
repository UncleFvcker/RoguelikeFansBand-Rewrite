# E8 Ego 集成审计

核对日期：2026-09-10。工作分支：`codex/realms-items`。

E8.8 已完成当前可玩范围共享生成契约与四类代表性桌面验收，见[当前证据](#e88-当前桌面验收)。
下列原始六项审计保留 v311 时的发现；后续完成情况以共享生成计划和本页当前证据为准。
全原版范围仍有未开放身份、未导入底材和对象表示限制，不声明整个 `apply_magic` 与原版等价。

## 权威来源与可复核产物

通过 Git 对象读取 `D:/codex/Frogcomposband/master` 的 `master`，本次解析为
`a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`。没有读取其工作树内容。

运行以下命令重新核对并生成[完整 160 条审计矩阵](ego-contract-audit.json)：

```powershell
node scripts/audit-egos.mjs D:/codex/Frogcomposband/master
```

矩阵逐条记录 source index、唯一 ID、权威中文名、类型、等级上下界、rarity、标准池/Craft 可达性、
物化位置、显式激活以及 importer 未映射标记；同时记录基础装备映射和共享缺口。
`runtimeParityComplete` 明确保持 `false`。它是身份、内容与实现位置的审计结果，不是 C/Rust 差分执行证明。

| 核对项 | 结果与证据边界 |
| --- | --- |
| Source 身份与中英文名 | 160 条一一对应，中文 unresolved 为 0；按 `e_info.txt` 和源版中文表核对 |
| 标准选择与 Craft | 156 条非零 rarity；Craft 类型 122 条、标准候选 121 条；103、210、211、260 禁止进入标准池 |
| 实例持久化 | 160 条分别在等级 20、80 物化或走特殊入口，共 320 次检查；恢复物品、RNG 和 state hash |
| 无效果排查 | 检查静态属性或动态物化结果；Duration 实际消耗燃料，Blasted 走真实诅咒入口。不能用这项检查代替每个消费者的语义测试 |
| 基础装备 | 通用池 138 件装备/弹药：134 件权威类型验证；3 个背包和捕获球归属独立容器/捕获系统 |
| 自然入口 | 通用池和 12 个怪物主题池统一使用 RFB Ego policy；这些池不再保留旧 `affixWeights` |
| 未映射标记 | 41 种逐项分类；有消费者的记录具体文件，`AWARE` 记录源端声明但无运行时消费者，不人为添加功能 |
| 内容清理 | 160 条正式 Ego 均无旧 `rollGroups`；移除重复 Combat 近似定义，保留 8 条用途不同的非 source affix |

实例检查见 [ego/contracts.rs](../crates/rfb-core/src/game/ego/contracts.rs)。各类型行为仍由
[武器/远程物化与测试](../crates/rfb-core/src/game/ego.rs)、[护甲](../crates/rfb-core/src/game/ego/armor.rs)、
[首饰](../crates/rfb-core/src/game/ego/jewelry.rs)、[光源/箭袋/装置](../crates/rfb-core/src/game/ego/noncraft.rs)
及相关消费者专项覆盖；现有类型测试的通过范围不外推为所有分支和所有职业已验收。

## 本次修正

- 显式武器、挖掘工具、远程、弹药和竖琴 Ego 通过实际物化分支创建，避免仅写入 affix ID。
- 删除 Protection、Speed、Combat 的旧 importer 近似配方和重复 Combat 定义；Orc Cave 奖励引用正式 Combat ring。
- 补齐 broad-sword、sabre、spear、diamond-edge、wizardstaff 的权威基础身份；巫师法杖进入普通池，实际减耗由施法费用测试验证。
- 正向质量改为源码条件短路掷骰，恢复 Chance 美德、深度上限、普通与强制 Great 的固定神器尝试顺序及 Good Luck 额外尝试。
- 补回基础武器附魔、特殊底材排除与 Diamond Edge 分支；发射器伤害在通用 `C:` 加值之前缩放。
- 自然生成的强化弹药骰数传入正式物品实例；固定种子 2295 的 6 骰 sheaf arrow 验证生成、提交和保存恢复全过程。
- 按用户要求去掉物品列表及详情中的“完全鉴定”“无 ego”提示，保留内部鉴定规则和已知属性；补齐箭袋槽位名称，并在显示词缀名时解释源版 `&`/`~` 格式控制符，权威本地化原文保持原样。

以上质量与神器修正只覆盖已实现的正向/固定神器分支。负向 power、随机神器和值驱动的重试尚未闭合，不能声称完整 RNG 与原版相同。

## 当前共享契约状态

前文及后文的版本、测试数和桌面记录属于最初的 v311 审计。后续 E8.1–E8.6 已接入真实估值、负向装备、龙系基础、背包、非弹药随机神器和首饰完整价值重试；B0–B6 又完成当前可玩构筑/导入基础池的类别与主题分配、发现计数、Tailored、Acquirement 重试及 standalone 获取装备保存验收。这些旧缺口不再列为待实现；源内容覆盖和未开放身份继续单列。当前证据及适用范围见[共享生成计划](ego-shared-generation-plan.md)、[B6 验收](base-allocation-acquirement-plan.md#b6接入验收审计与交付)和[生成矩阵](ego-contract-audit.json)。

E8.7 补齐主题 Ego 筛选、Bad Luck 固定神器参考层级和帽子 pval 限制，并从实际新游戏入口枚举构筑适用性。Vortex 已核实为演化装备模板、正向 BLOWS/天生攻击消费者和类别分配的间接输入，没有以身份直接判断的生成分支。

剩余工作明确分为：未开放职业/种族及神器卷轴/重铸入口；未导入的身份敏感固定神器及其他 source kind；B1 书本单实例和普通物品堆叠表示限制。当前导入池的底材分配器及可达 Acquirement/书本计数/偏好已由 B0–B6 完成，E8.8 补齐其余桌面证据；`runtimeParityComplete` 保持 `false`。Craft 四册/32 法术属于另一个导入任务。

## E8.8 当前桌面验收

2026-09-10 在 `codex/realms-items` 完成 Windows Tauri standalone WebDriver 自动验收。
使用实际新建的人类死亡高阶法师导出存档，保留 museum binding；核心准备器只固定基底并筛选共享生成器的真实实例，
不增加生产调试接口。每例由界面拾取未知物品、用鉴定卷轴鉴定、装备、验证效果，再导出存档、行动、
恢复精确状态 hash 与装备详情，最后继续行动。自然概率仍由核心测试验证。

| 案例 | 实际观察 |
| --- | --- |
| 负向 Ego | seed 122；速度负属性 −2，角色速度 108，相同装备去掉该负属性后为 110；诅咒阻止卸下，装备保留 |
| 随机神器 | seed 1697；实例名 `'Lone Star'`、搜索/潜行/红外等属性正常显示；探测门与楼梯激活充能 1/1 → 0/1，保存恢复保留实例名、属性及充能 |
| 龙皮盾 | seed 8；基础地狱抗性进入角色减伤投影，Ego 附魔同时贡献护甲；护甲 310，去除附魔值后为 230 |
| 动态背包 | seed 3；Holding 布袋容量 8，角色总槽位 34；实际装满 34 堆，货物 34.0 磅加袋重 1.0 磅，总重 35.0 磅；满载卸装与继续拾取被拒绝，货物保留 |

复用 B6 已通过的 Acquirement 桌面流程。本轮发现并修复 Nature 四册旧来源索引错误：
508–511 / tval 92 才是 Nature，原先 512–515 / tval 93 指向 Chaos。已同步权威名称、来源账本和正式分配池，
其中第四册恢复深度 70 / 权重 50，并增加源书类别/册数与执行领域绑定的回归检查。
另对齐 v320 契约常量和已变更的实体书括号、药水后缀测试；内容包版本见[状态快照](../docs/status.md)。

实际检查：

- `cargo test --workspace --exclude rfb-tauri` 全部通过，包含核心 1177、内容 144、导入器 187、本地化 39 项。
- 26 条 active contract 通过，未刷新断言；content lock 和 Ego 来源审计通过。
- 前端 195 项、TypeScript/Vite 构建及受影响 crates 的 Clippy 全部通过；协议/内容类型没有变化。
- 最终内容的 Tauri standalone 构建和四例桌面专项通过。

复现入口（在 `web` 目录）：

```powershell
npm run e2e:build
node e2e/tauri.e2e.mjs --ego
```

专项自动导出新角色存档，再调用忽略测试 `export_ego_desktop_acceptance_save` 准备四例。
无需事先构造无绑定存档；单独调用准备器时必须设置 `E88_DESKTOP_INPUT` 指向真实桌面新角色导出文件。
报告为 `test-results/ego-desktop-report.json`，截图为 `e88-{negative,randart,dragon,bag}.png` 和 `e88-bag-filled.png`，
另保留各例操作后的 `.rfbsave`。可运行产物为 `target/e2e/debug/rfb-tauri.exe`，不依赖 Vite。

当前可玩范围共享生成契约完成；全原版范围 `runtimeParityComplete` 仍为 `false`。
这不是人工试玩、全身份/全部 160 条逐项桌面验收或 Android 验收；未开放身份、未导入 source kind 和 B1 对象表示限制继续单列。
