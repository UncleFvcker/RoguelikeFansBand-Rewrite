# E8 六项共享生成契约实施计划

日期：2026-09-10。状态：E8.1–E8.6 已实现，早期验证记录见 [contract-v312](contract-v312-real-equipment-value.md)、[contract-v313](contract-v313-negative-equipment.md)、[contract-v314](contract-v314-dragon-base-equipment.md)、[contract-v315](contract-v315-bag-containers.md)、[contract-v316](contract-v316-random-artifact-identity.md)；下一项为 E8.7 的构筑适用性核对。正/负向随机神器和首饰外围价值重试已接回自然调度。

工作树：`D:/codex/RoguelikeFansBand-Rewrite-realms-items`，分支：`codex/realms-items`。
代码基线：`1c9e62a2e`。缺口来自 [E8 集成审计](ego-integration-audit.md)。
本次通过 Git 对象核对 RFB `master`，解析为 `a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`；
后续每批执行时重新解析 `D:/codex/Frogcomposband/master` 的 `master`，记录差异，不读取其工作树。

目标是补齐生成、实例状态和实际消费者。前五项组成当前可玩构筑的收口里程碑；
第六项随真实职业、种族开放逐项验收，在全部适用分支完成前保留全原版范围的未完成状态。
Craft 领域四册/32 法术、怪物主题的完整基础物品分配表仍是独立任务。

## 1. 顺序与依赖

推荐按下表顺序实施，每批独立提交。随机神器体量最大，拆为三个有实际交付物的小批次。
背包可独立于估值工作推进；表中的顺序是推荐提交顺序，不表示所有批次存在硬依赖。

| 批次 | 交付结果 | 硬依赖 | 对应缺口 | 相对规模 |
| --- | --- | --- | --- | --- |
| E8.1（已实现） | 生成上下文与真实装备估值 | 当前基线 | 1、2、3 的共同前置 | 大 |
| E8.2 | 负向 power、负属性与价值驱动诅咒 | E8.1 | 1 | 中到大 |
| E8.3 | 龙牙及五类龙系底材的基础生成 | E8.1 的 power/模式上下文 | 4 | 小到中 |
| E8.4 | 三种背包的实例容量及 Ego 消费者 | 当前容器系统；使用 E8.1 上下文 | 5 | 中 |
| E8.5a（已实现） | 随机神器实例身份、属性表示及消费者 | E8.1、E8.2 | 2 | 中 |
| E8.5b（已实现） | 真实 `create_artifact`、命名与估值筛选 | E8.5a；E8.3 的底材处理 | 2 | 大 |
| E8.5c（已实现） | 各非弹药类型的随机神器调度 | E8.5b | 2；3 的前置 | 中 |
| E8.6（已实现） | 首饰价值上下限和完整重试 | E8.1、E8.2、E8.5c | 3 | 中 |
| E8.7 | 职业/种族专属分支逐项接入 | 对应构筑真实可玩，及其用到的前述批次 | 6 | 按构筑拆分 |
| E8.8 | 当前构筑集成验收与更新审计 | E8.1–E8.6；当前开放构筑适用的 E8.7 | 前五项＋已开放构筑 | 中 |

主依赖链：**真实估值 → 诅咒 → 随机神器 → 首饰完整重试 → 集成验收**。
龙系和背包无需等随机神器完成才交付。E8.7 的源端分支清单在 E8.1 开始建立，实施随构筑到位。

## 2. E8.1：共享前置

### 生成上下文

修改现有 [loot.rs](../crates/rfb-core/src/game/loot.rs) 和 [ego.rs](../crates/rfb-core/src/game/ego.rs)，
让生成过程传递源码语义的有符号 `power`，以及真实调用点需要的生成模式、有效等级、玩家和主题上下文。
当前 `Ordinary/Fine/Exceptional` 不能表达 `-2/-1/0/1/2/3`，不能继续从显示质量反推 power。
power 只用于生成过程，不额外保存；显示质量和诅咒已知状态在最终实例投影时处理。

先列出自然掉落、主题掉落、奖励、显式创建、Craft 的入口及源码模式映射，区分
`GOOD/GREAT/SPECIAL/FORCE_EGO/QUEST/CRAFTING/NO_FIXED_ART` 等有不同短路语义的条件。
仅给实际入口增加必要参数，不建设通用规则引擎；源码存在而本项目尚无入口的模式明确列为未开放。
按调用顺序核对有效生成等级与原版 `object_level` 的取值，不能默认二者在奖励/特殊入口总相同。

### 真实装备估值

新增一个小型职责明确的 `game/item_value.rs`，承接 `object2.c:1178` 的 `obj_value_real` 装备分派，
按 `object3.c` 的 `weapon_cost/ammo_cost/bow_cost/armor_cost/jewelry_cost/lite_cost/quiver_cost`
及其共享计算移植所需语义。装置仅在本批真实调用链需要时纳入，沿用其独立 `device_value` 契约；
尸体等无关类型不借此扩成全部经济系统改造。

- 输入为实际底材和完整实例属性，包含原始 flags/pval 所表达的信息、骰数、重量、加值、激活、诅咒和神器身份。
- 先核对现有属性表示是否保留估值所需区别；若有信息丢失，只补充无法推导的实例信息，不能用静态 `baseValue` 或 affix 数量代替。
- 保留原版整数截断、分段、乘算、相反标记消解和价值归零顺序；独立验证神器光源走首饰估值的分派。
- 估值无 RNG、无修改实例、无修改鉴定状态。真实估值不受玩家是否已知影响。
- 当前 `town.rs` 商店使用基础价格，本批不顺带改定价，也不向 UI 泄露未知物品的真实价值。

验收：使用从权威源码独立求得的输入/期望值，覆盖各装备类型、正负 pval、叠加倍率、激活、
诅咒和价值阈值；验证鉴定前后值相同、估值前后 RNG 和物品不变。测试期望值不得调用待测 Rust 函数自行生成。
先交付估值及调用点需要的表示，再启用依赖它的筛选；不加入临时近似估值兜底。

## 3. E8.2：自然装备的负向质量

来源：`object2.c:2044–2150` 的 `apply_magic`，各类型 `obj_create_*`，
`artifact.c:350` 的 `one_biff`、`:1960` 的 `curse_object`，以及 `ego_finalize`。

实施位置：`loot.rs`、现有各类型 Ego 模块和 [item_curses.rs](../crates/rfb-core/src/game/item_curses.rs)。

1. 补齐 Good 失败后才执行的诅咒掷骰、`-1 → -2` 判断、`no_egos` 条件，以及深层普通负品质回退。
   戒指/项链/装置不走该深层回退；所有条件短路按源码保留，强制正向模式不得多消耗负向骰。
2. 按类型传入负 power，处理武器/护甲负加值、首饰负向规则、装置和光源自身分支；
   不把所有负 power 简化成同一种诅咒，也不把正向 Ego 物化结果简单取负。
3. 移植 `_add_bad_flag/one_biff/get_curse` 的实际适用规则、相反标记处理与尝试次数，复用现有诅咒消费者。
   `curse_object` 按诅咒前真实价值计算强度，保留嵌套掷骰；核对 RFB 对 `randint1(0)` 的定义与 RNG 消耗。
4. `power == -2` 的最终诅咒置于源码规定的属性物化之后，处理所需 pval 补值，避免随后通用附魔覆盖结果。

验收：固定输入覆盖 `-2/-1/0`、深度回退、强制模式、相反属性和每种可生成诅咒的消费者；
比较结果及 RNG 终态。自然获取后实际装备/解除诅咒/保存恢复，验证未知物品不提前泄露负属性。
此批可完成负向调度和普通/Ego 诅咒；随机神器的负向结果待 E8.5 一并验证后才算该交叉分支闭合。

## 4. E8.3：龙系基础装备

来源：`object2.c:1581` 的 `dragon_resist`、`:2312/:2348` 的前置分支，
以及 `obj_kind.c:496` 的 `object_is_dragon_armor`。

准确范围是龙牙，以及龙系头盔、披风、盾牌、手套、靴子；**该判定明确不包含龙鳞甲**。
按权威 tval/sval 核对当前底材，不能用名称含“龙”或已有龙甲 Ego 作为判定。

- 在 `loot.rs` 调用现有抗性/品牌抽样函数，把基础随机属性写入 `intrinsic_properties`。
- 保留至少一次抽样、龙牙元素品牌分支、元素/高抗选择、重复抽样和随次数变化的继续概率。
- 基础属性生成后再进行通常的 1/3 power 保留判断；Craft 跳过整个基础重掷分支。
  护甲 `SPECIAL + NO_FIXED_ART` 的例外与龙牙的规则分别实现，不能合并成一个近似条件。
- power 被压为 0 仍保留刚生成的底材属性；后续 Ego/神器候选复制底材时也不能丢失它们。

验收：普通质量仍能带基础抗性、Ego 被抑制仍保留抗性、Craft 保留原属性且无额外抽样、
龙鳞甲不误入、特殊模式的短路 RNG 正确；在实际伤害/装备消费者和保存恢复中核对结果。

## 5. E8.4：背包与现有容器系统

来源：`ego.c:3691` 的 `obj_create_quiver`，`quiver.c` 的 `bag_capacity/bag_carry/bag_weight`，
`object3.c:865` 的估值，以及各 Ego 的真实消费者。
三种物品为 `fabric-bag/leather-pouch/dwarven-backpack`。原先使用静态
`inventorySlotBonus = 4/8/12`；E8.4 已核对 source identity、pval 和权威中文名，
改用 `TV_QUIVER/SV_BAG` 生成及实例最终容量，详见 [验收记录](contract-v315-bag-containers.md)。

- 基础容量 `(pval + 1) * 4`；`power == 1` 加 2；`power > 1` 使用原版 Quiver Ego 选择，
  Holding 容量翻倍，Phase 容器自身重量为 0。背包不使用箭袋的基础随机容量和 `+20/+50`。
- 复用 [noncraft.rs](../crates/rfb-core/src/game/ego/noncraft.rs) 的选择/物化入口，按 sval 分开容量结果。
  不能为了“适用”先过滤掉源码允许选中的 Ego；逐项追踪 Protection/Endless 在背包上的消费者，
  对源码无背包效果的分支保留真实行为及审计说明，不能凭名字发明效果。
- 在既有属性/实例结构增加一个必要的背包容量值，采用一种明确的最终容量表示。
  它接入 [inventory.rs](../crates/rfb-core/src/game/inventory.rs) 的拾取、堆叠、换包、卸包、容量投影和保存恢复；
  生成背包时不再将静态 bonus 和实例最终容量重复相加。
- 保持背包按非弹药物品格计数、箭袋按弹药数量计数。先核对当前统一容器表示与原版独立 bag 的差异；
  若现有表示允许弹药占用背包新增格，必须修正实际分配/计数，再宣布该项等价，无需为 UI 另造一套容器页面。
- Phase 背包只按源端处理自身重量；`bag_weight` 仍计算内容重量，不能照搬 Phase 箭袋的内容减重。

验收：三种底材 × 普通/Good/Great、四种 Ego 的源码可达结果；满包拾取、可合并堆叠、
换小包/卸包失败时物品不丢失、弹药与普通物品边界、实际重量和存档。
UI 显示真实已知容量和权威 Ego 名，不恢复“完全鉴定”“无 ego”提示。

## 6. E8.5：非弹药随机神器

### E8.5a：实例身份与消费者

已实现，验收与源端差别见 [contract-v316](contract-v316-random-artifact-identity.md)。以下为本批原始范围。

当前神器判定多依赖物品定义的 `artifact` tag；随机神器需要保留普通底材并拥有实例神器身份和生成名称。
在 [state.rs](../crates/rfb-core/src/state.rs) 增加必要实例信息，属性、激活、诅咒优先复用已有字段。
重量和骰数等目前放在 `RolledAffixState` 的信息，若随机神器无法无损表示，应把共用信息放到实际实例层；
不创建虚假 Ego ID 来挂随机神器属性，也不为每件随机神器动态注册一种内容定义。

统一实际调用点的神器判断：命名/鉴定、堆叠、Craft 目标限制、毁坏与抗性、附魔/诅咒、自动拾取、
估值、投影和存档。固定神器的唯一性登记仍按原版固定身份处理，随机神器不误占固定神器生成记录。
逐个消费者读源码确定差别，不假定所有固定神器特例都应推广。

验收：准备一个真实实例，通过上述消费者和保存恢复核对；这一批是内部表示交付，尚不启用自然随机神器分支。

### E8.5b：完整生成器与价值筛选（已实现）

来源：RFB `master` 提交 `a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`。工厂在 `game/random_artifact/`；正式包 `randomArtifacts/source.json` 收录 30 个原版命名文件、195 项非零稀有度激活及 bias。规则测试覆盖槽位、特殊底材、职业/主题、重试与命名顺序，并通过真实激活、投掷和保存消费者验证。调用方拥有连续生成共用的名字表，自然调度及其持久化接线属于 E8.5c。

新增 `game/random_artifact.rs` 承载有实际调用关系的 `artifact.c:2122 create_artifact` 移植，
继续复用已有抽样、装备属性和激活执行能力，不建设插件式生成框架。

- 按源码先后实现底材处理、职业/主题 bias、能力次数、抗性/属性/武器能力抽样、pval、
  重量/骰数/加值、激活、标记整理、估值与命名，再处理诅咒及最终名字。
- 每个生成分支都应有真实消费者；当前可玩构筑需要而尚无消费者的能力在本批补齐后才能开放。
- 命名读取权威 `get_random_name` 及其数据文件，保留它占用的 RNG；中文只使用 master 中的权威字符串/表。
  无权威中文的内容记录 unresolved，不能本地翻译或用固定占位名宣称闭合。
- 移植 `ego.c:313 _art_create_random`：等级插值、槽位比例、上下限、softmax 额外拒绝、负 power 的诅咒模式。
  1000 次候选耗尽后另生成一次，不复用最后一个失败候选。
- 每次从同一输入底材复制候选，RNG 持续前进；失败候选不得污染下一候选或提交全局物品身份。
  除最终物品外，原版确有的状态副作用另行逐项核对，不一概回滚 RNG。

验收：各合法非弹药槽位、特殊底材、正/负 power、已有可玩职业/主题 bias，
候选拒绝、softmax 短路、1000 次耗尽后的第 1001 次、命名和诅咒的先后，以及本项目内固定种子连续性。
重复生成的统计仅辅助排查，不能替代分支及数值验收；弹药继续使用它自己的源码分支。

### E8.5c：接回真实调度（已实现）

来源仍为 `a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`。`loot.rs` 在固定神器、底材和基础强化之后调用 `random_artifact/scheduling.rs`；长袍先判定专用 Ego，龙系保留 power 压制，费艾诺光源保留先掷骰再判断强制 power 的顺序。Craft 继续使用直接 Ego 路径。生成草稿携带完整神器属性，提交时才分配物品 ID。`Game.random_artifact_names` 保存成功及被拒绝候选的名字，进入存档校验和状态哈希，防止恢复后重复命名或生成序列分叉。

逐个接入 `ego.c:303 _check_rand_art` 的调用者，包括武器/挖掘工具、远程/竖琴、护甲、首饰和光源特殊入口。
保留不同 base 概率、等级修正、Craft 排除、`power > 2`、首饰等级调整和项链额外条件。
固定神器尝试与随机神器尝试各自在原版位置执行；失败/成功后按源码决定是否进入 Ego 分支。

验收：自然获取的完整结果及 RNG、强制模式、固定神器已成功时不多掷随机分支、
Craft 不新增随机神器抽样、光源特殊入口、随机神器保存后继续生成的序列一致。
同步检验 E8.2 负向随机神器与 E8.3 龙系底材的交叉分支。

## 7. E8.6：戒指与项链的价值重试（已实现）

来源提交仍为 `a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`。`jewelry.rs` 在循环外选择阈值并执行一次 `GREAT_OBJ` 抽样，每轮从原始草稿重新执行随机神器判定或 Ego 生成及诅咒结算，再用真实估值筛选；第 1001 个候选无条件采用。淘汰的候选保留 RNG 和神器名字登记，但不分配物品 ID。自然入口覆盖正/负 power 与强制神器模式；阈值函数按源位标志保留 FORCE_EGO/GREAT/QUEST 优先于 GOOD 的规则。现有显式 kind/affix 配置仍直接物化，不把它们宣称为完整的源端 FORCE_EGO/QUEST 奖励调度，入口差异见 [E8.1 契约](contract-v312-real-equipment-value.md)。

来源：`ego.c:373 _get_jewelry_power_limit`、`:408 ego_create_ring`、`:430 ego_create_amulet`，
以及内部 `_create_ring_aux/_create_amulet_aux`。修改 [jewelry.rs](../crates/rfb-core/src/game/ego/jewelry.rs)。

1. 阈值表按严格 `level < lvl` 选择；`max == 0` 表示无上限。保留
   `FORCE_EGO/GREAT/QUEST` 与 `GOOD` 的最低价值调整及一次 `GREAT_OBJ` 阈值升级抽样。
2. 上下限在重试循环外计算一次。每次复制原始输入，执行完整内部生成，使用 E8.1 真实估值决定接受；
   内部可能生成 Ego，也可能进入 E8.5 随机神器，其内部重试和所有 RNG 都要保留。
3. 1000 次拒绝后再次调用内部生成，并直接采用该次结果；不得返回空物品、最后失败候选或自行钳制属性。
4. 淘汰现有自然入口“选一次 Ego 即提交”的路径，显式强制创建仍按其真实源端入口处理。

验收：每个等级分段前一值/边界值，模式优先级、无上限、恰好上下限、下限不足/上限超出、
第 1 次/多次/第 1001 次结果、负 power 和内部随机神器；比较最终实例及 RNG 终态。
保留当前各首饰 Ego 分支测试，并补自然生成集成测试；首饰实测价值分布作为辅助报告。

阈值、完整候选及自然入口验收位于 [generation_tests.rs](../crates/rfb-core/src/game/ego/jewelry/generation_tests.rs)。辅助采样保留正式基础分配的等级约束，按戒指/项链、等级 10/29/59/80 和 Good/Great 各连续生成 128 次；固定神器登记后单独计数，报告只作分布观察。运行 `cargo test -p rfb-core --lib --no-default-features jewelry_value_distribution_report -- --ignored --nocapture`，结果写入 `target/e86-jewelry-value-distribution.json`。

## 8. E8.7：尚未开放的职业/种族

此项不是在生成器里加几个永远为假的布尔开关。先记录“源码条件 → 真实构筑入口 → 生成变化 → 消费者 → 测试”。
在前述批次实现当前已开放构筑实际需要的差异；未开放构筑随职业/种族任务完成后接入，不阻塞前五项交付。

| 构筑/条件 | 已核实的来源与工作 | 完成前置与验收 |
| --- | --- | --- |
| Mauler | `ego.c:1641 ego_weapon_adjust_weight`；`artifact.c` bias 等分支；`object2.c` 相关底材选择 | 真实职业入口和重武器消费者；骰数强化后的重量、负重/战斗结果、Ego 与随机神器路径 |
| Bard | `object2.c:2298` 竖琴基础 pval；`artifact.c:2044` 槽位估值比例、bias 与竖琴相关修正 | 真实职业及竖琴使用；与非 Bard 在同输入下比较 pval、价值限制和实际能力 |
| Monster Ring | `ego.c:455 ACTIVATION_CHANCE`；`object2.c` 首饰选择/主题分支 | 真实种族与装备/激活入口；首饰基础池、激活概率及可实际使用的结果 |
| Vortex | 三个生成源文件没有直接身份分支；`r_vortex.c:764-810` 使用演化后的 `mon_get_equip_template`，`b_info.txt` 定义 3–8 个 ANY 槽；`equip.c:1622` 对正向 BLOWS 减半 | 分类为装备模板/天生攻击消费者；`equip.c:372` 的 ANY 接受除 BOW 外的类别，因此 `object2.c:3078` 间接将弓/箭袋、弹药类别权重减半。等待真实种族入口，不增加直接生成开关 |
| 其余条件 | 枚举上述源码中的其他职业、种族、变异、人格和主题条件，不把前三个例子当成穷举 | 区分已开放、待开放、源端无对应生成分支；按真实适用范围分别完成 |

每个构筑使用可从新游戏创建的真实配置验证生成、装备/激活和保存恢复；
测试中伪造身份只能做函数级分支测试，不能作为该构筑已接入的验收。
把 Vortex 等证据修正同步到 `scripts/audit-egos.mjs` 和生成的审计矩阵；本计划不提前修改机器审计结果。

本批来源为 `master` 对象 `a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`。
[机器矩阵](ego-contract-audit.json) 的 `buildApplicability` 从前端实际新游戏入口读取 6 个构筑、46 个种族，
逐行收录三个源文件的身份、变异、人格、领域、德行和主题条件，并按生成变化、入口、消费者、测试和前置条件分类。
运行 `node scripts/audit-egos.mjs D:/codex/Frogcomposband/master` 重生成；源提交改变或出现未分类条件时审计失败。
矩阵中的测试路径是证据索引，审计命令本身不执行这些测试。

已补共享生成的两个实际缺口：主题先筛选首饰/护甲 Ego 候选，只有主题类型池为空才回到通用池；
Bad Luck 在每次固定神器尝试前单独降低参考层级，并在 Tomte 帽速度 pval 增长时先掷继续骰、再停止增长。
帽子规则由自然生成、Craft 和显式物化共同使用。验收见 [applicability.rs](../crates/rfb-core/src/game/ego/applicability.rs)：
真实 Warrior 接收 Mage/Dwarf 主题装备后装备与保存恢复，真实 Tomte 获得 Bad Luck 后比较帽子生成、速度消费者和恢复；
空主题池及参考层级/RNG 顺序另做函数级验证。没有把伪造 Mauler/Bard/Monster Ring 身份的测试当成入口验收。

E8.7 留下的当前可玩底材分配缺口已由 B0–B6 完成：Acquirement 的 Archer/Sniper 弓、Cavalry 骑乘武器、
High Mage 装置/领域书偏好，装备兼容/最爱武器筛选、发现书本计数、失败重试及当前导入池的主题底材分配，
均已通过规则、消费者和保存验收。入口矩阵、standalone Acquirement 及对象表示限制见
[完整底材分配与 Acquirement 计划](base-allocation-acquirement-plan.md#b6接入验收审计与交付)。
其余待开放条件包括 Berserker、Sexy/Aphrodite、神器卷轴职业 bias/德行、Inspired Smithing 重铸、
Draconian Metamorphosis 和固定神器身份分支。正式包已有 Dr Jones 鞭的普通行为，Archaeologist 奖励分支未开放；
其他九件身份敏感固定神器和固定神器竖琴尚未导入。Monster Ring 的类别加权条件还比较了分配表未使用的
`kind_is_jewelry` hook，矩阵记作源表下不可达，不据此编造加权规则。全范围 `runtimeParityComplete` 保持 `false`。

## 9. 验证、版本与完成判定

每批先运行所改模块的核心测试、相关内容/导入器验证和格式检查；协议或前端变化时补 bindings、schema、
TypeScript 与对应 UI 测试。按 2026-09-10 的用户决定，后续生成验收核对源码规则、阈值、抽样条件与分支顺序，
并验证 Rust 固定种子连续性；不要求同种子产物或 RNG 终态与 C 实现相同，也不要求跨实现逐次随机输入对齐。
历史批次已保存的参考向量仍保留；确定性数值（如估值和阈值）继续用独立来源核对。

- 重试测试用最小测试内输入控制覆盖拒绝/接受边界，不为测试添加生产调试开关或通用模拟框架。
- 复用现有保存恢复路径。新增持久字段时同步 save/protocol/state-hash 输入和生成物；
  按各自实际结构/语义变化推进版本，不预定未来版本号，不保留旧开发存档兼容。
- 内容变更更新 pack version/content lock。纯内容改动只验证行为受影响的 fixture 类别；
  公共 RNG、初始化、共享投影或 state-hash 输入发生改变时，按项目规则进行完整契约验证/必要刷新，记录原因。
  不因刷新而改变 fixture 场景、放松断言或把 `contentHash` 加回 state hash。
- 契约保持单一行为；非移动主题使用直接位置前置条件，普通商店购买选首个投影库存。
- E8.8 为明确里程碑，运行完整契约及相关 workspace/内容/前端检查，并用 standalone Tauri 构建做代表性桌面验收。
  普通小批次不反复跑大型桌面 E2E。

桌面验收分别准备负向 Ego、随机神器、带基础抗性的龙系装备和动态背包的真实存档，
通过 UI 完成获取/鉴定、装备和实际效果、容量/重量、保存恢复；自然概率由核心生成测试验证。
保留未知信息边界和当前用户要求的简洁物品显示。人工试玩由用户执行，自动化记录不冒充人工试玩。

更新 [审计说明](ego-integration-audit.md)、[机器矩阵](ego-contract-audit.json) 和
[总计划](../docs/archive/2026-09-09/design/ego-import-plan.md) 时，逐项附已实现入口、消费者、参考向量与验证记录。
前五项及全部已开放构筑通过后，可标记“当前可玩范围共享生成契约完成”；
尚未开放构筑对应契约继续列明依赖，不能把全范围 `runtimeParityComplete` 提前改成 `true`。

下一步为 E8.8 其余桌面里程碑；B6 已完成的 Acquirement 桌面流程无需重复。未开放身份随真实职业/种族入口接入，
未导入 source kind 与 B1 书本/普通堆叠表示限制继续单列，不计作全源生成对齐。
