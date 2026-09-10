# E8 Ego 集成审计

日期：2026-09-09。工作分支：`codex/realms-items`。

本次完成旧近似清理、160 条身份与实例检查，以及代表性桌面流程。
**E8 的完整原版等价验收尚未完成**：下列六项共享生成契约仍有缺口。
E3–E7 的“已接入”表示各类型已有物化分支和消费者，不能据此声称整个 `apply_magic` 调度与原版等价。

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

前文及后文的版本、测试数和桌面记录属于最初的 v311 审计。后续 E8.1–E8.6 已接入真实估值、负向装备、龙系基础、背包、非弹药随机神器和首饰完整价值重试；这些旧缺口不再列为待实现。当前证据及适用范围见[共享生成计划](ego-shared-generation-plan.md)和[生成矩阵](ego-contract-audit.json)。

E8.7 补齐主题 Ego 筛选、Bad Luck 固定神器参考层级和帽子 pval 限制，并从实际新游戏入口枚举构筑适用性。Vortex 已核实为演化装备模板、正向 BLOWS/天生攻击消费者和类别分配的间接输入，没有以身份直接判断的生成分支。

剩余工作明确分为：未开放职业/种族及神器卷轴/重铸入口；未导入的身份敏感固定神器；完整底材分配器及其当前可达的 Acquirement/书本计数/偏好条件。最后一类不能以职业尚未开放为由注销。正式主题表仍是改编基础池；`runtimeParityComplete` 保持 `false`。Craft 四册/32 法术属于另一个导入任务。

## 验证与桌面复现

本次使用 Protocol 1.234、State Hash Schema v111、save header/payload v6、内容包 1.391.0、contract-v311。
没有新增持久字段；自然生成 RNG 改变，按明确集成里程碑对全部 26 条 active fixture 刷新并复验，保持场景和命令不变。
26 条断言复算后与提交内容相同，未产生 assertion 文件差异；基线标记与代码常量统一推进至 v311。

| 验证 | 本次结果 |
| --- | --- |
| Rust workspace（排除 Tauri） | 全量通过；最终主题池变更后复验核心 909 项及相关内容检查，后续等价 lint 清理复验首饰 7 项、Ty Curse 6 项、effect program 5 项 |
| 内容与导入器 | 内容 133 项、导入器 174 项通过；content lock 校验通过 |
| 契约 | `verify-all` 26 条通过；里程碑专用 `committed_contract_fixtures_pass --ignored` 通过，零 waiver |
| 前端与生成物 | 181 项前端测试、TypeScript 类型检查、协议 bindings 与 content schemas 一致性检查通过 |
| 静态检查 | `cargo clippy --workspace --exclude rfb-tauri --all-targets -- -D warnings`、格式和 diff 检查通过 |
| 桌面 | 最终源码 standalone Tauri 构建、普通 E2E 和 Ego 专项均通过 |

桌面专项通过真实 `.rfbsave` 导入准备状态，再由 UI 执行操作，不增加生产调试接口：

```powershell
cargo test -p rfb-core export_ego_desktop_acceptance_save -- --ignored
cd web
npm run e2e:build
npm run e2e:tauri
node e2e/tauri.e2e.mjs --ego
```

`e2e:build` 调用 Tauri standalone build，运行不依赖 Vite 开发服务器。
专项使用未知的照明灯、Combat ring、Endless quiver，拾取后使用 revelation scroll 鉴定并装备；
实际激活照明灯并确认能量消耗，再对无名匕首使用工艺卷轴，确认已知 Ego 和卷轴消耗；最后导出、继续行动、
导入存档，验证精确 hash 和装备恢复。自然随机获取分布由核心生成测试覆盖，桌面使用准备存档不冒充自然掉落概率测试。
产物写入 `test-results/ego-desktop-report.json` 和 `test-results/ego-desktop.png`。

常规桌面 E2E 另覆盖战士及死亡领域高阶法师的新游戏、菜单、装备、学习施法和保存恢复。
人工试玩由用户执行，本次不记录为人工试玩、全职业或全部 160 条的逐项桌面验收。
