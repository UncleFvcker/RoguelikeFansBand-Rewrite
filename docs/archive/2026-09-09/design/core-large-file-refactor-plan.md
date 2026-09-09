> 历史快照（2026-09-09 归档）：本文保留当时的设计、版本与验收记录，不作为当前工作指令或待办。现行说明见 [文档索引](../../../README.md)。

# 核心大文件拆分计划

日期：2026-09-09。检查基线：`5698fd0d6`。状态：批次 A–G 及整轮重构里程碑验收均已完成，行为和契约基线保持不变。

本计划响应降低大文件阅读和修改成本的需求，首轮限定于 `rfb-core/src/game/mod.rs`、`abilities.rs` 及其实际依赖。它是新一轮维护计划，不重开或改写 `architecture-convergence-plan.md` 已完成的历史阶段。

## 1. 目标与边界

目标是让“改能力说明”“改掉落生成”“改目标选择”“改某类法术效果”有可预测的文件入口，并缩小一次修改需要理解的上下文。

- `Game` 继续独占权威状态、RNG、实体/物品 ID 和命令提交；子模块继续用既有 `impl Game` 方法直接访问状态。
- 保留构造、`dispatch`、快照、存档及回放公共 API。不增加 manager、trait、服务对象、上下文容器或第二条规则路径。
- 一批只移动一个职责族及必要的类型、常量和辅助函数；算法简化、命名清理、性能优化、玩法修正另开批次。
- 不更改协议、内容包、存档结构、状态哈希 Schema 或 fixture 预期。不添加旧开发存档兼容。
- 文件行数作为结果观测，不作为硬性验收指标。允许完整且连贯的匹配表达式保留原状。
- 首轮不拆旧版导入器、协议 DTO、前端或其他大型规则文件；不顺带进行全仓重构。

## 2. 已检查的职责分布

以下行号用于定位基线，执行时按符号和调用点确认，不能用固定行号机械切割。

| 当前位置 | 已识别职责 | 处理方向 |
| --- | --- | --- |
| `game/mod.rs`，9,188 行 | 状态、初始化、命令协调，以及多个独立规则/投影职责 | 分批迁出职责完整的代码族 |
| `mod.rs:1538`，`dispatch` 约 1,530 行 | 命令前置条件、行动、时间推进、收尾和更新输出 | 首轮保留整个交易顺序 |
| `mod.rs:7154–7763` | 等级曲线、法术威力和装置威力计算 | `ability_scaling.rs` |
| `mod.rs:7764–8832` | 能力效果和目标的 DTO 投影 | `ability_projection.rs` |
| `mod.rs:5269–6119` 及相关类型/纯函数 | 携带掉落、死亡掉落、品质、神器和生成草稿 | `loot.rs` |
| `mod.rs:4079–4473` 及底部相关几何函数 | 弹道、射线、区域和锥形范围 | `projectile_geometry.rs` |
| `mod.rs:6156–6534` 及 `LightSource` 等 | 可见性、探索记忆、感知与光照 | 感知进入 `visibility.rs`，光照归入现有 `lighting.rs` |
| `mod.rs:1122–1537` 及出生专用辅助函数 | 新游戏初始化 | 后期按实际收益移到 `initialization.rs` |
| `abilities.rs`，11,530 行 | 施法事务、目标规划、效果分发、各类效果执行 | 改为同名私有目录模块 |
| `abilities.rs:166` | 玩家能力来源、失败率、资源支付和进度 | `abilities/casting.rs` |
| `abilities.rs:667` | 效果与目标计划的分发 | 保留于 `abilities/mod.rs` |
| `abilities.rs:10621`，约 850 行 | `ability_target_plan` | `abilities/targeting.rs` |
| `abilities.rs:3910`、`:8125`、`:9050` 等 | 有序效果和复杂组合能力 | `abilities/compound.rs`，保留各自执行顺序 |

已确认的跨模块调用说明不能简单按“玩家法术”封闭拆分：

- 能力投影由 `snapshot.rs` 消费；等级和威力计算由 `player_abilities.rs` 消费。
- 弹道几何也由 `monster_abilities.rs` 使用。
- 反射路径由 `item_combat.rs` 使用；地震同时由玩家战斗、怪物战斗和能力触发。
- 加权抽样由 `ego.rs`、`tasks.rs` 等共享，不能随掉落整体移动后形成错误的所有权归属。
- `AbilityTargetPlan` 被现有白盒测试引用；模块层级变化必须保留所需的内部可见性。

## 3. `abilities` 的目标职责

现已采用一层私有子目录，避免把同前缀文件散落在 `game` 根目录。以下文件均已在对应职责迁入时创建。

| 文件 | 归属内容 | 保持清晰的边界 |
| --- | --- | --- |
| `abilities/mod.rs` | 模块声明、效果分发、少量共享名称 | 不重新堆积效果实现，也不加逐层转发包装 |
| `casting.rs` | 能力来源检查、失败/消耗/熟练度、随机分支选择、待补方向施法的恢复 | 继续调用现有 `player_abilities.rs` 的数值与资源策略 |
| `targeting.rs` | `AbilityTargetPlan` 和 `ability_target_plan` | 保持当前查询/候选生成语义，不提前抽随机数 |
| `damage.rs` | bolt、beam、ball、cone、可见目标伤害、反射、伤害附带效果 | 共用现有伤害提交、死亡和几何方法 |
| `control.rs` | 控制、怪物状态、群体睡眠/静止、驱逐或消灭目标等 | 不复制 `status_effects.rs` 或 `damage.rs` 的底层规则 |
| `restoration.rs` | 玩家治疗、状态恢复、饥饿、属性维持、变异治疗、单体增益和自我状态查询 | 保留既有 capabilities 的状态修改入口 |
| `items.rs` | 鉴定、品牌、防腐蚀、解除装备诅咒、补充光源燃料、生成物品/弹药、转换、吸取和充能 | 物品底层状态继续由现有库存/物品模块处理 |
| `terrain.rs` | 地形射线、造门/楼梯、地震、区域破坏、照明、探测与怪物探查 | 地震来源类型随实现移动，保留战斗侧调用 |
| `travel.rs` | 传送、城镇传送、位置交换、召回、换层、现实改变 | 实际楼层切换仍调用既有 floor/travel 机制 |
| `summoning.rs` | 类别/群体召唤、自然之门、天使/恶魔召唤、尸体复生 | 保持候选、生成位置、阵营、ID 和 RNG 次序 |
| `compound.rs` | 通用有序效果、空效果事件、自然之怒、神圣干预、圣战、近战后传送、宠物爆破等跨效果编排 | 组合能力整体迁移，复用其他效果方法 |

这是按目前符号检查拟定的完整归属方向。实际迁移前需为该批所有方法、类型、常量、自由函数列出唯一目标，尤其检查品牌、岩石伤害、阳光伤害等跨职责辅助函数；有多个消费者时按主要职责归属并显式导入。

不按死亡、自然、圣战等领域全面分文件：大量能力共享伤害、目标和地形机制，领域划分会增加跨文件往返。独特组合能力保留完整实现。

## 4. 执行顺序与提交边界

每批单独可编译、可测试、可回退。上一批验证通过后再开始下一批。

| 批次 | 工作内容 | 预期收益 | 聚焦验证 |
| --- | --- | --- | --- |
| A（已完成） | 拆出 `ability_scaling.rs`、`ability_projection.rs` | 根文件减少 1,679 行，计算和说明各有入口 | 355 项聚焦核心测试、3 项回放、3 条契约及静态检查通过，见执行记录 |
| B（已完成） | 拆出 `loot.rs`，迁移生成草稿、来源、品质和神器相关定义 | 掉落生成有独立入口；B–C 合计使根文件减少 2,012 行 | 95 项聚焦测试及 A–C 阶段验收通过，见执行记录 |
| C1（已完成） | 拆出 `projectile_geometry.rs` | 玩家、怪物、装置共同使用一处几何实现 | 176 项聚焦测试及 A–C 阶段验收通过，见执行记录 |
| C2（已完成） | 拆分感知/探索记忆，光照归入 `lighting.rs` | 显示与感知更易定位 | 完整核心、回放及相关契约通过，见执行记录 |
| D（已完成） | 将 `abilities.rs` 改为 `abilities/mod.rs`；迁出 casting、targeting | 原单文件 11,536 行，现入口及效果主体 9,978 行，施法 637 行、目标规划 953 行 | 395 项聚焦核心测试、3 项回放、3 条契约及静态检查通过，见执行记录 |
| E1（已完成） | 迁出 damage、control | 最常改的战斗能力形成稳定入口 | 281 项聚焦测试及静态检查通过 |
| E2（已完成） | 迁出 restoration、items、terrain、travel；按职责逐项迁移和验证 | 生活、物品、地图效果可独立阅读 | 四组分别通过 154、169、162、269 项聚焦测试及静态检查 |
| E3（已完成） | 迁出 summoning、compound，收尾所有剩余效果的归属 | 入口降至 697 行，仅保留声明、导入和效果分发 | 完整核心 863 项、回放 8 项、相关契约 5 条及静态检查通过 |
| F（已完成） | 出生初始化迁入 `initialization.rs`，五个学习/遗忘方法归入现有 `player_abilities.rs` | 根模块 5,491 → 4,743 行，保留状态、共享入口与命令协调 | 完整核心 863 项、回放 8 项、相关契约 5 条及静态检查通过 |
| G（已完成） | 将 `tests/abilities.rs` 按十个行为族拆分 | 原 10,659 行文件变为十个测试文件及 133 行共享入口 | 110 个测试逐项映射，完整核心仍为 863 项且全部通过；整轮里程碑验收通过 |

聚焦验证表给出的是现有测试模块/场景入口，执行时按真实调用图选取并补充，不能据表机械排除跨系统调用者。

第一步执行 A，第二步执行 B–C，第三步执行 D，第四步执行 E1–E3，第五步执行 F，第六步执行 G，现均已完成。生产代码职责拆分、测试文件拆分及整轮里程碑验收均已完成。

## 5. 根模块保留什么

完成 A–F 后，`game/mod.rs` 仍保留：

- `Game` 权威字段与稳定公共查询；
- 命令边界及 `dispatch` 的整体执行顺序；
- RNG 与 ID 权威分配入口；
- 尚未形成独立职责的跨系统协调方法；
- 必要的模块声明和显式内部导入。

`dispatch` 第一轮不拆成大量小型 action handler。它目前包含自然之怒回退处理、行动前后可见性比较、时间推进及连续行动中断等相互关联的步骤。先移走外围职责，再判断其前置校验/输出构造是否值得单独提取；不能仅为缩短函数把十几个局部变量装进新的上下文对象。

初始化迁移也放在后期：出生物品、身体槽、装置生成辅助函数可能还有装备、变形、保存恢复等消费者。只把出生专用内容迁入初始化，共享规则归实际领域；不要把共享规则藏进初始化模块。

预估结果为根文件约 4,000–5,500 行，`abilities/mod.rs` 约 700–1,000 行，多数效果文件约 500–2,000 行；伤害和复杂组合模块可能更大。该范围是职责分布估算，不是承诺或硬上限。总生产代码应基本持平，增加主要来自模块声明和必要导入。

## 6. Rust 可见性与依赖规则

- 新增的 `game` 同级模块默认私有；只给真实消费者开放 `pub(super)`。
- `abilities` 子模块之间使用最小可见性；仍由 `game` 的其他模块或白盒测试调用的方法/类型，按需使用 `pub(in crate::game)`。子模块中的 `pub(super)` 仅覆盖 `abilities`，不能误当作覆盖整个 `game`。
- `AbilityTargetPlan` 可在 `abilities/mod.rs` 做一次必要的内部重导出，保持既有使用入口。禁止为所有方法建立兼容包装。
- 迁移自由函数时同时迁移其私有辅助类型和常量；共享类型字段按真实需要开放，不把所有字段改成 `pub(crate)`。
- 新代码明确写出关键跨职责导入，不借机全量改写仓库现有 `use super::*` 风格。
- 不新增 `utils.rs`、`common.rs`、`helpers.rs` 来接收不确定的归属。

## 7. 行为保持与验证

迁移时保持函数主体、签名、分支顺序、容器类型、数值截断点、随机抽取次数、目标遍历顺序、ID 分配、事件顺序、错误及首次失败顺序。测试不降低断言、不补兼容默认值、不刷新预期。

每批执行：

1. 提交前记录本批完整符号归属和调用点；同一职责只保留一份实现。
2. `cargo fmt --all -- --check`、`git diff --check`、`cargo check -p rfb-core --all-targets`。
3. 对应聚焦核心测试及实际受影响的 replay/contract 场景；移动测试时核对发现数量。
4. `cargo clippy -p rfb-core --all-targets -- -D warnings`。
5. 检查 diff：协议、内容包、存档/状态哈希定义、fixture 预期均无变化。检查内部可见性没有不必要扩大。

阶段验收：A–C 完成后、D–F 完成后分别执行完整 `rfb-core --lib` 与 `rfb-replay` 测试。如 G 执行，另核对并执行移动后的完整核心测试。无需每移动一个文件重跑全部检查。

本轮执行完成时设一次明确的重构里程碑验收：

- workspace 测试和 Clippy（排除 Tauri），Tauri 的 all-target 编译检查。
- 协议绑定、内容 schema、正式内容 source/lock 验证。
- 前端类型检查和测试，确认消费边界不变。
- 完整重放当前 26 个已提交 exact fixture（显式运行 ignored 的 `committed_contract_fixtures_pass`），验证已有预期不变；不执行 refresh。
- 只有涉及原生/前端行为、发现相关失败或用户明确要求时，才增加桌面 E2E。当前规划不默认要求 Android 或完整桌面套件。

出现状态哈希、RNG、事件、错误或快照漂移时，先定位/撤回本批移动，不修改基线来让测试通过。原代码若有独立缺陷，另外记录并在行为变更任务中处理。

## 8. 可读性验收与完成记录

每批完成记录迁出的职责、最终路径、行数变化、必要的可见性调整和已执行验证。除测试通过外，检查四个具体问题：

- 改能力说明是否能直接定位 `ability_projection.rs`？
- 改等级/威力计算是否能直接定位 `ability_scaling.rs`？
- 改掉落是否能定位 `loot.rs`，且 RNG/ID 权威仍一目了然？
- 改法术效果是否能从唯一分发点直接到达对应效果族，不需要穿过新增包装层？

`tests/abilities.rs` 如需拆分，按上述行为族组织并保留现有测试体、断言与公共测试辅助函数；不一法术一文件，不按生产方法逐个配测试文件。`tests/high_mage.rs`、旧版导入器和其他大文件留待独立职责审计。

完成标准是职责能被直接找到、共享规则没有复制、公共行为和确定性证据保持不变。达到后停止，不为了行数继续拆分。

## 9. 批次 A 执行记录（2026-09-09）

| 文件 | 结果 |
| --- | --- |
| `game/mod.rs` | 9,188 → 7,509 行，减少 1,679 行 |
| `game/ability_scaling.rs` | 新增 618 行，承接 8 个等级、法术威力、装置威力计算函数 |
| `game/ability_projection.rs` | 新增 1,092 行，承接 4 个效果及目标投影函数 |

迁移函数完整归属：

- scaling：`scaled_ability_level_value`、`prorated_level_value`、`apply_ability_level_scaling`、`spell_power_value`、`device_power_value`、`apply_ability_spell_power`、`ability_has_spell_power_field`、`spell_powered_ability_value`。
- projection：`ability_effect_spec_dto`、`player_ability_effect_spec_dto`、`ability_target_spec_dto`、`target_spec_dto`。

两个模块均私有，12 个函数仅向 `game` 内部开放 `pub(super)`；其中基础投影和曲线函数继续供现有白盒测试直接调用。消费者显式导入新模块：能力执行、施法参数、装置使用/伤害、龙人变形属性、词缀激活目标和快照。根模块只保留自身装置初始化所需的 `target_spec_dto` 导入，没有增加转发函数。原来借根模块导入的测试专用类型改由测试模块导入。

已将迁移前后的 12 个函数整体对比，排除内部可见性声明与空白格式后完全一致。已有测试只调整导入，测试体和断言不变；没有改动 `dispatch`、存档/状态哈希定义、协议、内容或 fixture 预期。

实际验证：

- `cargo check -p rfb-core --all-targets`、`cargo clippy -p rfb-core --all-targets -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check`：通过。
- `generate-bindings --check`：通过。
- 核心聚焦测试 355 项通过、0 失败、0 ignored；508 项未在本批执行。覆盖 abilities、high_mage、snapshots、trait_details、weapon_ego_activations、items、combat、progression、sniper、paladin、cavalry。
- 回放测试 3 项通过：`combat_replay_records_authoritative_rng_draws`、`item_replay_survives_shop_save_reload`、`draconian_metamorphosis_choice_and_body_are_replayable`。
- 契约复验 3 条通过：`19-save-round-trip-initial.json`、`390-scroll-phase-door.json`、`50-player-projectile-launcher-unavailable.json`；未刷新基线。
- 未运行全量核心测试、全量 fixture 重放、桌面 E2E 或 Android 构建；阶段及里程碑验证留待对应阶段。

## 10. 批次 B–C 执行记录（2026-09-09）

| 文件 | 结果 |
| --- | --- |
| `game/mod.rs` | 7,509 → 5,497 行，减少 2,012 行；相对初始基线累计减少 3,691 行 |
| `game/loot.rs` | 新增 1,010 行，承接携带/死亡掉落、物品品质、神器和生成草稿 |
| `game/projectile_geometry.rs` | 新增 586 行，承接弹道路径、射线、区域、锥形范围及相关几何辅助函数 |
| `game/visibility.rs` | 新增 294 行，承接可见性、探索记忆、感知和视线判断 |
| `game/lighting.rs` | 现为 427 行，承接环境光、地图发光、光源及光强计算；既有燃料规则保持原样 |

`LootContext`、`LootSource`、`ItemGenerationMode` 和 `GeneratedItemDraft` 随生成规则迁移；RNG、ID 分配和跨领域使用的加权抽样继续保留在根模块。弹道几何、视线判断及光照辅助类型的消费者直接从所属模块导入，根模块没有增加转发包装。

新模块均私有，仅对真实消费者和既有白盒测试开放必要的 `pub(super)`。生成上下文和草稿字段按调用方构造需要开放；`LightSource` 字段、光源包含判断及各模块专用方法保持私有。

已逐块比较迁移前后的掉落、几何、可见性及光照代码，除内部可见性、空白和 rustfmt 补齐的签名尾逗号外一致。既有测试仅调整导入，测试体和断言不变。未更改算法、RNG 抽取、ID 分配、事件顺序、协议、内容、存档/状态哈希结构或 fixture 预期。

实际验证：

- B 聚焦测试 95 项通过：items、mining、mining_progress、generation、monster_ecology。
- C1 聚焦测试 176 项通过：combat、abilities、monster_ai、sniper、weapon_traits。
- A–C 阶段验收：完整 `rfb-core --lib` 863 项通过，完整 `rfb-replay` 8 项通过，均为 0 失败、0 ignored。
- `cargo check -p rfb-core --all-targets`、`cargo clippy -p rfb-core --all-targets -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check`：通过。
- 契约复验 5 条通过：`19-save-round-trip-initial.json`、`484-mining-treasure-vein-rewards.json`、`461-outpost-general-store-purchase.json`、`459-light-torch-refuel.json`、`50-player-projectile-launcher-unavailable.json`；未刷新基线。
- 未运行全量 fixture 重放、桌面 E2E 或 Android 构建；整轮里程碑验收留待后续批次完成。

## 11. 批次 D 执行记录（2026-09-09）

| 文件 | 结果 |
| --- | --- |
| `game/abilities.rs` | 原 11,536 行，已改为同名目录模块 |
| `game/abilities/mod.rs` | 9,978 行，保留效果分发、具体效果及相关辅助定义，比原单文件减少 1,558 行 |
| `game/abilities/casting.rs` | 637 行，承接施法流程、随机分支及待补方向恢复 |
| `game/abilities/targeting.rs` | 953 行，承接目标计划类型和完整目标规划 |

迁移符号完整归属：

- casting：`ability_state_unavailable_reason`、`resolve_player_ability`、`select_player_random_choice_branch`、`resolve_pending_ability_direction`、`nature_wrath_direction_roll`，以及仅由施法流程使用的 `DEATH_INVOKE_SPIRITS_ABILITY_ID`、`NATURE_WRATH_ABILITY_ID`。
- targeting：`AbilityTargetPlan`、`ability_target_plan`。继续调用既有弹道几何、物品资格和召唤候选查询，不移动或复制这些共享规则。

两个子模块均私有。跨 `game` 消费的方法、目标计划类型与待补方向事件查询使用 `pub(in crate::game)`，保持原来的内部可见范围；随机分支选择仍私有。`abilities/mod.rs` 仅重导出既有消费者所需的 `AbilityTargetPlan` 和 `nature_wrath_direction_roll`，没有新增转发函数。新文件显式导入依赖；既有生产消费者与测试文件无需因目录层级改变而修改。

已逐函数核对迁出的 6 个函数、目标计划枚举及保留的效果模块。除必要的内部可见性、目标规划中的类型引用路径与格式调整外一致。来源优先级、目标校验先于资源支付/失败 RNG、随机分支选择、进度更新、待补方向续施/取消及首次成功经验顺序保持原样。`dispatch`、测试体与断言、协议、内容、存档/状态哈希定义及 fixture 预期均未在本批修改。

实际验证：

- `cargo check -p rfb-core --all-targets`、`cargo clippy -p rfb-core --all-targets -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check`：通过。
- 核心聚焦测试 395 项通过、0 失败、0 ignored；468 项未在本批执行。覆盖 abilities、high_mage、archer、paladin、sniper、mutations、items、monster_ecology、progression、cavalry，包含无效目标/取消不消耗资源或 RNG，以及待补方向流程取消、续施、存档恢复的现有断言。
- 回放测试 3 项通过：`combat_replay_records_authoritative_rng_draws`、`item_replay_survives_shop_save_reload`、`draconian_metamorphosis_choice_and_body_are_replayable`。
- 契约复验 3 条通过：`19-save-round-trip-initial.json`、`390-scroll-phase-door.json`、`50-player-projectile-launcher-unavailable.json`；未刷新基线。
- 未重跑完整核心、完整 fixture、桌面 E2E 或 Android 构建；D–F 阶段验收留待对应批次完成。

## 12. 批次 E1–E3 执行记录（2026-09-09）

| 文件 | 行数 | 归属 |
| --- | --- | --- |
| `abilities/mod.rs` | 9,978 → 697，减少 9,281 | 模块声明、既有两个内部重导出、必要导入及唯一效果分发方法 |
| `abilities/damage.rs` | 2,034 | 29 个伤害方法，包括弹道/范围伤害、反射、伤害附带效果及近战能力 |
| `abilities/control.rs` | 1,272 | 15 个控制方法，包括怪物状态、驱逐、群体控制及消灭目标 |
| `abilities/restoration.rs` | 951 | 18 个玩家恢复、变异、增益及自我状态方法；属性转换/赋值辅助函数随自身变形迁移 |
| `abilities/items.rs` | 978 | 17 个物品效果及资格/生成辅助方法；品牌专用常量随实现迁移 |
| `abilities/terrain.rs` | 1,564 | 23 个地形、照明与探测方法；`EarthquakeSource` 随地震实现迁移 |
| `abilities/travel.rs` | 431 | 10 个传送、召回、换层及目的地处理方法 |
| `abilities/summoning.rs` | 606 | 9 个召唤、尸体复生及候选处理方法；复生专用常量随实现迁移 |
| `abilities/compound.rs` | 1,648 | 12 个有序效果、组合能力及专用辅助方法，完整保留各自事件与 RNG 顺序 |
| `game/player_abilities.rs` | 增加 11 行 | 承接既有 `casting_spell_damage_bonus`，供伤害、组合能力及快照共用 |

八个效果模块均私有。原先向 `game` 开放的方法改用 `pub(in crate::game)`，保持内部可见范围；原私有方法仅在分发器、兄弟模块或已有 casting/targeting 调用时开放 `pub(super)`，其余辅助方法继续私有。地震来源类型只向 `abilities` 内部开放。新模块显式导入依赖，直接复用既有伤害提交、物品状态、地形修改、移动、召唤及数值计算入口，没有新增管理器、包装函数、共享工具模块或第二份规则实现。

共迁出 133 个效果方法及 1 个数值查询，保留 1 个效果分发方法。已对全部 135 个方法和 9 个相关定义逐项比较，忽略可见性声明、必要的物品辅助函数引用路径与格式变化后，代码 token 一致；16 个既有 lint 属性均保留。`casting.rs` 和 `targeting.rs` 与第三步结果完全一致。`game/mod.rs` 仅清理失去生产消费者的导入，测试专用名称直接导入对应测试文件；既有测试体和断言未改动。

分批验证（各组存在重复覆盖，不累加为独立测试总数）：

- E1：281 项通过，覆盖 abilities、high_mage、combat、monster_ai、weapon_ego_activations、mutations。
- E2/restoration：154 项通过，覆盖 high_mage、mutations、hunger。
- E2/items：169 项通过，覆盖 items、archer、abilities。
- E2/terrain：162 项通过，覆盖 lighting、world、high_mage。
- E2/travel：269 项通过，覆盖 world、abilities、high_mage、riding。
- 上述各批均通过 `cargo check -p rfb-core --all-targets` 和 `cargo clippy -p rfb-core --all-targets -- -D warnings`。

E3 收尾验证：

- 完整 `rfb-core --lib` 863 项、完整 `rfb-replay` 8 项通过，均为 0 失败、0 ignored。
- `cargo check -p rfb-core --all-targets`、`cargo clippy -p rfb-core --all-targets -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check`：通过。
- 契约复验 5 条通过：`19-save-round-trip-initial.json`、`390-scroll-phase-door.json`、`50-player-projectile-launcher-unavailable.json`、`459-light-torch-refuel.json`、`476-potion-new-life.json`。
- 协议、内容、存档/状态哈希定义及 fixture 预期无变化，未刷新基线。未执行全量 fixture、桌面 E2E 或 Android 构建；F–G 及整轮里程碑验收留待后续执行。

## 13. 批次 F 执行记录（2026-09-09）

| 文件 | 结果 |
| --- | --- |
| `game/mod.rs` | 5,491 → 4,743 行，减少 748 行；相对初始 9,188 行累计减少 4,445 行 |
| `game/initialization.rs` | 新增 651 行，承接公共构造器、内部构造流程及出生专用辅助逻辑 |
| `game/player_abilities.rs` | 1,313 → 1,446 行，承接五个学习/遗忘方法 |

初始化迁移的完整符号归属：

- 关联常量与构造器：`DEFAULT_PLAYER_NAME`、`new`、`new_with_build`、`new_with_build_and_name`、`new_with_build_race_and_name`、`from_content`、`from_content_with_build`、`from_content_internal`。
- 出生专用规则：`dungeon_substitution_uses_alternate`、`initial_dungeon_states`、`resolve_body_slots`、`append_starting_items`、`append_starting_item`、`initialize_starting_item_knowledge`。
- 学习/遗忘归入 `player_abilities.rs`：`study_player_ability`、`study_random_player_ability`、`ability_study_unavailable_reason`、`study_book_id`、`forget_player_ability`。

残余职责复查：根模块继续保留 `Game` 状态定义、完整 `dispatch`、RNG/ID 分配、公共查询与跨系统行动协调。`normalize_player_name` 仍被构造、存档恢复和城镇改名共享；`base_dungeon_states` 仍供构造、恢复和验证使用。身体槽类型、通用槽位规则及装置/物品初始状态仍被装备、变形、掉落、商店或存档等流程共享，未藏入出生模块。出生身体槽解析本身只有构造器和现有白盒测试使用，因此随初始化迁移。

`initialization` 为私有模块。全部公共构造器和关联常量保持原来的 `Game::…` API，没有转发包装；`from_content_internal`、地城替代判断和出生身体槽解析仅为现有白盒测试开放 `pub(super)`，其余出生辅助逻辑私有。学习/随机学习/遗忘入口及快照使用的学习前置检查开放 `pub(super)`，书籍解析保持私有。两个测试文件仅调整对应辅助函数的显式导入，测试体和断言保持原样。

已核对迁移后的完整文件与原代码提取结果：除必要的导入、可见性和格式调整外，保留代码与全部 18 个迁移函数的 token 一致。公共构造行为、错误顺序、出生 RNG、物品数量/ID、初始化收尾顺序、随机学习抽样及遗忘规则未改变。`dispatch`、状态字段、协议、内容及存档/状态哈希结构未修改。

D–F 阶段实际验证：

- 完整 `rfb-core --lib` 863 项、完整 `rfb-replay` 8 项通过，均为 0 失败、0 ignored。包含 progression、prayer_study、各职业、world、persistence 等现有初始化及学习场景。
- `cargo check -p rfb-core --all-targets`、`cargo clippy -p rfb-core --all-targets -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check`：通过。
- 契约复验 5 条通过：`19-save-round-trip-initial.json`、`457-gold-warrior-starting-wallet.json`、`390-scroll-phase-door.json`、`141-progression-attribute-point.json`、`50-player-projectile-launcher-unavailable.json`；未刷新基线。
- 未运行全量 fixture、桌面 E2E 或 Android 构建；G 及整轮里程碑验收留待后续执行。

## 14. 批次 G 执行记录（2026-09-09）

原 `game/tests/abilities.rs` 共 10,659 行，已改为同名私有目录模块。测试按所验证的行为归属，完整测试体不拆散。

| 文件 | 行数 | 测试数 |
| --- | --- | --- |
| `tests/abilities/mod.rs` | 133 | 0 |
| `tests/abilities/casting.rs` | 752 | 10 |
| `tests/abilities/compound.rs` | 524 | 5 |
| `tests/abilities/control.rs` | 961 | 8 |
| `tests/abilities/damage.rs` | 3,157 | 27 |
| `tests/abilities/items.rs` | 720 | 11 |
| `tests/abilities/restoration.rs` | 1,338 | 14 |
| `tests/abilities/scaling.rs` | 701 | 6 |
| `tests/abilities/summoning.rs` | 1,155 | 18 |
| `tests/abilities/terrain.rs` | 825 | 7 |
| `tests/abilities/travel.rs` | 463 | 4 |

入口仅包含模块声明、共享导入、常量和六个跨行为族使用的测试辅助函数；其余五个辅助函数放入唯一消费它们的行为文件。27 个常量同样按消费者归属，保持一处定义。继续复用既有 `tests::support`，未增加通用工具模块或复制辅助实现。伤害测试仍为一个连贯行为族，不为行数阈值继续切碎。

全部 110 个测试、11 个辅助函数和 27 个常量逐项比较，忽略格式后代码 token 一致；测试名称、断言和 `#[test]` 属性保留，未新增 ignored。生产代码未在本批改动。

迁移前后分别运行 `cargo test -p rfb-core --lib -- --list`，测试发现数均为 863：110 个测试仅在路径中增加行为族模块，其余 753 个完整测试路径不变。逐项旧路径到新路径的映射及按行为族执行示例见[测试迁移索引](ability-test-module-map.md)。完整核心测试已随下述 workspace 验收执行，全部通过。

## 15. 整轮重构里程碑验收（2026-09-09）

实际执行以下检查，全部通过：

- `cargo test --workspace --exclude rfb-tauri -- --quiet`：workspace 测试通过，其中完整核心 863 项、回放 8 项通过，二者均为 0 失败、0 ignored。
- `cargo clippy --workspace --exclude rfb-tauri --all-targets -- -D warnings`。
- `cargo check -p rfb-core --all-targets` 与 `cargo check -p rfb-tauri --all-targets`。
- `cargo run -p rfb-protocol --features bindings --bin generate-bindings -- --check`。
- `cargo run -p rfb-content --features schemas --bin generate-content-schemas -- --check`。
- `cargo run -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original`：正式内容 source/lock 一致，版本仍为 `1.384.0`。
- 在 `web` 执行 `npm run typecheck` 与 `npm test`：类型检查通过，180 项前端测试全部通过。
- `cargo test -p rfb-contract --test contract_fixtures committed_contract_fixtures_pass -- --ignored --exact`：完整重放当前 26 个已提交 exact fixture，已有预期全部通过。
- `cargo fmt --all -- --check` 与 `git diff --check`。

协议、内容包、存档/状态哈希定义和 fixture 预期均未修改，没有刷新基线。按计划未运行桌面 E2E 或 Android 构建。
