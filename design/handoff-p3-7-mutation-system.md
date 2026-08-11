# P3.7 变异系统交接

本文档是变异方向的当前入口。接手对话应先读本文，再按需要查阅
[`phase-19-legacy-item-integration.md`](phase-19-legacy-item-integration.md) 的
P3.7-M0 至 M4E 原始批次记录。权威内容覆盖账本是
[`legacy-mutation-plan.json`](../packs/rfb-demo-original/legacy-mutation-plan.json)。

## 1. 当前基线

- 当前工作树：`D:/codex/RoguelikeFansBand-Rewrite`
- 分支：`main`
- M0--M4E 已进入当前集成历史；`366960bc`（M4D）与 `54af6c07`（M4E）均为
  当前 `HEAD` 的祖先，没有待合并的旧 item 工作树提交。
- M4F-A 已由 `a9a5dbde feat: activate M4F-A passive mutations` 独立提交；
  M4F-B1 在该提交之上实现 P3.8 Elvish Waybread 与兰巴斯不耐受；M4F-B2/B3
  继续完成 Ill Norm 与 ESP。
- RFB 权威源：`D:/codex/Frogcomposband` 的 Git `master` ref；覆盖审计当前锁定
  `efd63661302866038f58d8cd2553b23e6af3bf9d`。不要读取该仓库工作树文件替代
  `master` Git 对象。

当前发布标尺：

| 项目 | 当前值 |
| --- | --- |
| Protocol | `1.161` |
| Save container | `v1` |
| State Hash Schema | `v80` |
| Contract baseline | `contract-v240`，21 个 exact fixtures |
| Contract Schema | `v4` |
| Content pack | `1.233.0` |
| Content hash | `6d326f2dd4735e6122b34a6f71903afe2c480e56d3095cd88c7d45e097b06a19` |

当前 83 个 content、333 个 core、57 个 legacy-import 测试全部通过，并保留 21 个
exact fixtures。M4F-B1 为两座城镇杂货店加入兰巴斯并改变公共初始化 RNG；
M4F-B3 为实体投影增加 glyph，并持久化 Weird Mind 的感知结果，因此协议、状态
哈希 Schema 与全部精简 fixture 在本批次统一推进。

早期计划中基于 `436b1967` 推算的 Protocol `1.152`、State Hash Schema `73`、
Pack `1.212.0` 已经是历史值。后续只能从上表当前值顺延，并且版本、生成绑定、
State Hash Schema、content lock 与共享 fixtures 由集成工作树集中更新一次。

## 2. 覆盖状态

RFB `master` 共 152 项变异：104 个基础随机候选，48 个零权重身份。当前审计为：

| 机制族 | active | blocked | 总数 |
| --- | ---: | ---: | ---: |
| passive-bonus | 44 | 7 | 51 |
| cross-system-query | 12 | 26 | 38 |
| activation | 0 | 35 | 35 |
| periodic-effect | 0 | 28 | 28 |
| 合计 | 56 | 96 | 152 |

随机候选中已有 41 项 active，仍有 63 项 blocked。现有随机选择器会读取所有
`randomWeight > 0` 的正式定义，并不知道覆盖账本的 `active/blocked` 状态。因此在
104 个随机候选全部拥有真实行为前，不得把 Polymorph 或其他随机获得入口接到
`gain_random_mutation`，否则会正式授予无行为的壳。

`rfb.mutation.merchants-friend` 是唯一一个 RFB 本来就没有数值效果的身份；它已
被明确记录为 active，不应给它补写本地商店加成。

## 3. 已完成批次

| 批次 | 结果 |
| --- | --- |
| M0 | 冻结全部 152 项的源索引、稳定 ID、权威中文名/描述、评级、权重、资格条件、互斥边和 blocker。 |
| M1 | `MutationDefinition`、角色 active/locked 集合、存档、投影、校验和状态哈希已建立。 |
| M2 | 统一 gain/lose/random/bulk-loss 事务；锁定保护、互斥移除顺序、零候选零 RNG、刷新边界和未锁定变异再生衰减已完成。 |
| M3 | 六项个人属性潜力、HP progression 重掷和 New Life 原子事务已完成；新生药水已 active。 |
| M4A | 14 项属性、速度和护甲被动 active。 |
| M4B | 7 项抗性、感知和飞行被动 active。 |
| M4C | 5 项再生、接触光环和固有光照被动 active。 |
| M4D | 11 项天生攻击及战斗被动 active；提交 `366960bc` 已集成。 |
| M4E | 13 项经验、知识、步行、施法、商店和怪物能力等跨系统变异 active；提交 `54af6c07` 已集成。 |
| M4F-A | Infravision、Elemental Vulnerability、Pultitis 复用现有红外、四系易伤和属性管线 active。 |
| M4F-B1 | Elvish Waybread 已按原版食物行为加入；Waybread Intolerance 复用饥饿、麻痹、解毒与飞行管线 active。 |
| M4F-B2 | Ill Norm 屏蔽其他变异的魅力修正，并在最终属性上保证 `8 + 2 × 等级` 的最低魅力。 |
| M4F-B3 | ESP 接入 Empty/Weird Mind 标记；Weird Mind 按 1/10 刷新感知，非视觉目标只投影白色原 glyph 与通用“怪物”身份。 |

当前唯一权威角色状态仍是 `CharacterProgress.active_mutation_ids` 与
`locked_mutation_ids`。属性潜力继续由 `CharacterProgress.attribute_potentials`
持有，HP 成长继续只使用 `hp_progression`。没有第二套 mutation、HP、资源或
能力持久状态。

主要代码所有者：

- 内容定义与校验：`crates/rfb-content/src/definitions/mutations.rs`、
  `crates/rfb-content/src/validation/mod.rs`
- 核心事务与跨系统查询：`crates/rfb-core/src/game/mutations.rs`
- 存档和载入校验：`crates/rfb-core/src/save.rs`、
  `crates/rfb-core/src/game/persistence.rs`、`validation.rs`
- 投影：`crates/rfb-core/src/game/snapshot.rs`、`crates/rfb-protocol/src/lib.rs`
- 内容账本：`packs/rfb-demo-original/legacy-mutation-plan.json`
- 主要回归测试：`crates/rfb-content/src/tests/catalog.rs`、
  `crates/rfb-core/src/game/tests/progression.rs`

## 4. M7 前缺失的随机候选闭环

原计划的 M5 包含 source index 0--30 的 31 个常规主动候选，M6 包含 27 个
常规周期候选。M4F-A 与 M4F-B1 已完成其中 4 项；即使 M5/M6 全部完成，仍需闭合以下 7 个
不属于 M5/M6 的随机候选：

| ID | 中文名 | 当前账本 blocker | 闭环重点 |
| --- | --- | --- | --- |
| `rfb.mutation.chaos-gift` | 混沌神明 | `mutation-cross-system-query` | 审计并收窄混沌馈赠消费者 |
| `rfb.mutation.ill-norm` | 安心幻象 | `mutation-cross-system-query` | 审计并收窄外观/知识投影 |
| `rfb.mutation.esp` | 心灵感应 | `actor-telepathy-mind-flags-and-fuzzy-projection` | 怪物 mind flags 与模糊身份投影 |
| `rfb.mutation.bad-luck` | 黑色光环 | `luck-item-generation-device-outcome-and-curse-consumers` | 统一全部坏运气消费者 |
| `rfb.mutation.good-luck` | 白色光环 | `luck-item-generation-device-outcome-and-sensing-consumers` | 统一全部好运气消费者 |
| `rfb.mutation.easy-tiring` | 易疲劳 | `fatigue-minislow-state-recovery-and-physical-action-consumers` | `minislow`、恢复与物理动作 |
| `rfb.mutation.impotence` | 魔法无能 | `device-category-specific-failure-modifiers` | 设备类别失败率消费者 |

将这一门槛记为 **M4F：随机候选闭环**。它可以和 M5/M6 分别设计，但必须在
M7 之前合并并通过审计。建议按以下顺序推进：

1. M4F-A（已完成）：Infravision、Elemental Vulnerability、Pultitis 已复用现有
   派生值和抗性管线。
2. M4F-B1（已完成）：Waybread Intolerance 与 P3.8 Elvish Waybread 同批完成。
   M4F-B 后续只保留 ESP 与 Ill Norm 的怪物身份、知识和投影支撑。
3. M4F-C：Good Luck、Bad Luck、Easy Tiring、Impotence、Chaos Gift 等多消费者
   规则。必须先统一所有消费者，再把账本状态改成 active。

M7 前置断言应直接检查：152 项定义仍完整、`randomWeight > 0` 恰好 104 项，且
这 104 项账本状态全部为 active。只检查总 active 数不够。

## 5. P3.7-M5：主动变异能力

M5 的常规范围是 source index 0--30 的 31 个随机候选。RFB 另有 4 个零权重
主动身份：Peerless Tracker、Fantastic Frenzy、Draconian Strike、Draconian
Kin；它们不属于 104 候选的 M7 门槛，应保留独立的职业/龙族身份 blocker，除非
对应前置系统已完整存在。

### M5.0 统一施放合同

在 `MutationDefinition` 增加一个可选、窄范围的主动能力配置：

- `minimumLevel`
- `governingAttribute`
- `cost`
- `baseFailurePercent`
- `abilityId`

active mutation 动态授予引用的既有 AbilityDefinition，但使用独立的变异施放
参数。不要把它写入学习列表或 `ability_progress`：变异能力不占学习容量、不遗忘、
不获得熟练度，也不需要新增持久状态。

变异只负责来源、触发、冷却和消耗；伤害、治疗、状态、传送、召唤、地形、资源和
物品变化继续调用来源无关的现有事务。只有出现第二个真实调用方时才抽取窄核心
事务，不合并现有 Ability/Item 效果枚举，也不建立大一统效果脚本引擎。

施放成本保留 RFB 的 SP 优先、短缺部分扣 HP 规则。复用现有资源和伤害事务；
没有现成 SP 池时，不为变异伪造一个资源池。目标取消、等级不足或其他前置拒绝的
成本/RNG 边界必须逐项按权威行为锁定。

### M5 分批

1. M5-A：探测、传送和位移。优先复用 Detect、Teleport、Blink、Swap Position、
   Recall、Banish 等现有能力。
2. M5-B：射线、吐息、凝视、近战触碰和范围攻击。复用伤害、状态、目标和死亡
   事务，不复制第二套战斗结算。
3. M5-C：地形、物品、炼金和资源操作。Eat Rock、Midas Touch、Eat Magic、
   Weigh Magic 等只能在目标、物品实例、堆叠和资源语义完整后 active。
4. M5-D：召唤、变形和特殊能力。Grow Mold、Sterility 和主动 Polymorph 等不得
   用近似效果代替。主动 Polymorph 的原版 `do_poly_self` 合同明显大于药水的
   变异增删算法，二者只能共享底层事务，不能当成同一个效果。

每个子批验收至少覆盖成功、失败、等级不足、SP 足够、SP 不足转扣 HP、HP 不足、
取消目标、RNG 次数、事件顺序、存档往返，以及 gain/lose 后能力投影即时增删。

## 6. P3.7-M6：周期副作用

M6 的常规范围是 27 个有随机权重的 periodic-effect 身份。零权重的 Human
Constitution（`rfb.mutation.human-con`）不属于 M7 的 104 候选门槛，应保留其
独立人物身份和完整周期合同 blocker。

### M6.0 世界 tick 合同

- 只在本地地图 tick 处理；世界地图模式不触发。
- 按 `sourceIndex` 升序检查 active 周期变异，不能依赖集合迭代顺序。
- 每项先锁定权威触发概率，再执行该项自己的后续 RNG；无适用目标时是否消耗 RNG
  必须由原版行为决定。
- UI 动画、文本扰动和幻觉表现不得消耗核心 RNG。
- 伤害、状态、传送、召唤、资源、物品和地形改变继续走现有事务。

### M6 分批

1. M6-A：状态与资源，包括 Berserk Rage、Cowardice、Alcohol、Hallucination、
   Produce Mana、Speed Flux、Invulnerability、SP/HP 转换和 Hypochondria。
2. M6-B：传送、位移和放逐，包括 Random Teleport、Random Banish、Shadow Walk
   与 Fumbling。
3. M6-C：召唤、环境和范围影响，包括 Flatulence、Attract Demon/Animal/Dragon、
   Raw Chaos 与 Eat Light。
4. M6-D：变异、伤口、感知和特殊效果，包括 Normality、Polymorph Wounds、
   Wasting、Random Telepathy、Nausea、Warning 与 Wraithform。

每个子批使用固定种子测试准确触发/不触发分支、同 tick 多项顺序、死亡短路、地图
范围、事件顺序和 RNG draw count。世界地图测试必须证明零周期触发、零额外 RNG。

## 7. P3.7-M7：Polymorph 药水

M7 只能在 M4F、M5、M6 合并后开始，并以“104 个随机候选全部有真实行为”的
覆盖审计为硬门槛。`gain_random_mutation` 和 `lose_random_mutation` 已存在，M7
只应增加一个可审计的 Polymorph 事务，不再创建第二套候选选择器。

药水 source index 459 的原版算法：

1. 统计未锁定变异数 `count`。
2. 当 `count > 1` 时，以 `1/23` 清除全部未锁定变异；锁定变异保留。
3. 否则进入变更循环，每轮以 `1/2` 在 gain 与 lose 之间选择。
4. 当 `count > 5` 时 loss 分支总是允许；否则仅以 `1 / (6 - count)` 允许。
5. 在第一次实际 gain/loss 前必须继续循环；每次实际改变后，以 `1/2` 概率继续。
6. 没有合法 gain/loss 候选时必须安全结束，不能死循环，也不能额外消耗无意义 RNG。
7. 整个物品事务完成后只刷新一次属性、HP、资源和能力投影。

完成后把 Polymorph 药水加入正式 items、权威中英文名称/flavor、Black Market 或
权威获取途径，并把 item adaptation/coverage 账本从 blocked 改为 active。

验收必须覆盖：零候选、只有锁定变异、`count` 为 0/1/5/6、`1/23` 全清、连续多次
改变、gain-time 互斥移除、事件顺序、物品消耗、RNG 次数和确定性重放。

## 8. P3.7-M8：界面与集中收口

核心已经通过 `PlayerDto.mutations` 投影名称、描述、评级和锁定标记，但 Web 当前
只生成了 TypeScript 类型，没有角色面板消费。M8 需要：

1. 角色面板显示 active mutation 列表、评级、锁定标记和描述。
2. 主动变异进入现有能力界面；来源应可区分为 mutation，但不要复制一套施放 UI。
3. 补齐 gain、lose、冲突移除、周期触发、施放失败/成本和 Polymorph 的权威中英文
   事件文本。
4. 确认 New Life 与 Polymorph 两件物品均为 active，拥有来源身份、flavor 和获取
   途径，item 与 mutation 两份覆盖账本数字一致。
5. 由集成工作树统一升级实际需要的 Protocol、State Hash Schema、pack version、
   content lock、生成绑定和受影响 fixtures。内容-only 变化不得无故升级协议或
   State Hash Schema。

P3.7 发布门槛：

- New Life 与 Polymorph 药水均 active。
- 104 个随机候选全部具备真实行为；随机入口不会授予行为壳。
- active/locked 状态、能力来源和周期效果可存档、投影、重放并保持确定性。
- 没有变异专用的旁路 HP、SP、属性、能力进度、状态或 RNG。
- 内容审计、Schema、绑定、聚焦测试和受影响 fixtures 全绿。

这一定义不等于“152 项全部 active”。如果 M4F/M5/M6 只完成 104 个随机候选，
预计变异账本为 119 active / 33 blocked；剩余 33 个零权重职业、种族、人格或特殊
身份应另开 M9 完整目录收口，不能为了宣布全量完成而给它们通用近似行为。

## 9. 工作树与合并边界

当前只保留 `main` 工作树。M4F、M5、M6、M7 应在同一连续变异基线上按顺序推进，
因为它们都会修改
`MutationDefinition`、`game/mutations.rs`、覆盖账本和同一组测试。若必须拆给多个
对话，应显式独占文件，并在每批后先合并回统一变异基线，避免平行引入两种能力或
tick 合同。

M8 Web 界面可以在协议形状冻结后使用单独工作树推进。集成工作树独占：

- `crates/rfb-protocol` 版本与生成绑定
- State Hash Schema 版本
- `pack.json`、`content.lock.json`
- 共享 replay/contract fixtures 与最终 baseline

功能工作树可以提出这些文件的必要变更，但交接中必须写清结构影响和刷新范围，
不要与集成工作树同时改版本号或全量 fixtures。

## 10. 每批验证

常规 item/变异批次至少运行：

```powershell
$env:RFB_LEGACY_SOURCE='D:/codex/Frogcomposband'
cargo run -p rfb-legacy-import -- audit-demo-mutations packs/rfb-demo-original/legacy-mutation-plan.json
cargo test -p rfb-content -p rfb-core -p rfb-legacy-import
cargo run -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original
cargo run -p rfb-content --features schemas --bin generate-content-schemas -- --check
cargo fmt --all -- --check
cargo clippy -p rfb-content -p rfb-core -p rfb-legacy-import --all-targets -- -D warnings
git diff --check
```

只有协议投影改变时才运行并提交协议绑定生成；只有状态哈希输入、共享初始化或 RNG
行为改变时才刷新对应 replay/contract fixtures。过期桌面 E2E 已退出本阶段验收，
不再运行或维护。

## 11. 下一对话启动清单

1. 确认从当前 `main` 的干净基线继续；不要恢复已经删除的 `work/items` 工作树。
2. 运行 mutation audit，记录开始时的 active/blocked 和 104 候选闭环数字。
3. 一次只认领 M4F、M5、M6、M7 或 M8 中一个明确子批，并声明拥有的文件。
4. 开工前用 `git show master:<path>` 读取 RFB 权威实现；中文名必须使用 master
   运行时中文字符串，没有权威中文名就保持 unresolved。
5. 完成行为、来源身份、中文名/描述、获取或触发路径、测试和账本后，才把对应身份
   标为 active。
6. 提交功能分支并留下：提交哈希、覆盖数字、剩余 blocker、版本/Schema 影响和
   实际运行的验证命令。
