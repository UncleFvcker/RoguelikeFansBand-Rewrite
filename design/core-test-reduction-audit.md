# rfb-core 测试缩减审查：第一步

日期：2026-09-09。对象：第一步工作区快照中 `rfb-core --lib` 的 904 个测试入口。只建立分类、保留清单及后续改动条件，本步不删除或改写测试。

后续状态：第二步压缩为 875 项；[第三步](core-test-reduction-stage3.md)为 871 项，主要减少世界生成、赶路和掉落抽样；[第四步](core-test-reduction-stage4.md)为 867 项，收敛种族、职业出生和战斗分支矩阵；[第五步](core-test-reduction-stage5.md)当前为 863 项，收敛必填字段、投影恢复和重复规则计算。下文保留第一步历史分类及行号，不代表当前测试总数；第二步替换记录见[对应账本](core-test-reduction-stage2.md)。

## 结论

已将全部 904 项列入 [逐项清单](core-test-inventory.md)，并和 Cargo 可运行列表逐项对齐。下表不是“已证明可以删掉多少项”：独立机制采用保守保留，参数变体仍可能包含必须保留的独有断言，低收益抽样指采样方式值得改写。

| 分类 | 数量 | 当前处置 |
|---|---:|---|
| 独立机制 | 785 | 保留当前测试；没有经过证明的整项替代。 |
| 参数变体 | 111 | 保留行为与数据行；按下面 P01–P13 共享准备或断言。 |
| 重复验证 | 0 | 本轮没有确认任何剩余测试可由另一项完整替代并直接删除。 |
| 低收益抽样 | 8 | 按 S01–S08 建立固定分支样本后缩减采样；不能直接移除。 |

正文复核明确标记了 20 项，其他项为结构初筛。不能将保守归类解读成对每项独立性都做了完整证明。此前清理已删除的测试不在本清单内，不能再次计算收益。

300 是待验证的预算，不是本审查已经证明能无损达到的结果。当前证据不足以支持删除 604 个入口；把所有测试包进少数函数不会减少保障成本，也不计作简化成果。参数行数、初始化次数和运行耗时应与函数数分别记录。

## 必须保留的保障

| 保障 | 现有定位 | 保留要求 |
|---|---|---|
| 取消／拒绝原子性 | [`mutation_cast_spills_sp_into_hp_and_keeps_rejections_atomic`](../crates/rfb-core/src/game/tests/abilities.rs#L8002) | 继续检查 HP／SP、RNG 与错误事件；还须保留各适配入口独有副作用检查。 |
| 待选方向的取消及恢复 | [`commit33_natures_wrath_direction_prompt_is_atomic_cancelable_and_persistent`](../crates/rfb-core/src/game/tests/high_mage.rs#L5461) | 保留 pending 状态、取消、保存恢复与提交路径，不能只测成功施法。 |
| 多单位物品拆分原子性 | [`p3_5_crafting_split_allocation_failure_is_atomic`](../crates/rfb-core/src/game/tests/items.rs#L2750) | 保留分配失败时的原实例、数量及提交边界。 |
| 购买拒绝及业务状态 | [`rejected_purchase_preserves_rng_and_business_state`](../crates/rfb-core/src/game/tests/town.rs#L2102) | 保留 RNG、收费、库存和拒绝状态；通用 fixture 只覆盖自己的场景。 |
| 新存档校验 | [`malformed_generated_artifact_state_is_rejected`](../crates/rfb-core/src/game/tests/persistence.rs#L102) | 所有非法字段／引用／边界行继续保留；不加旧开发存档兼容。 |
| 哈希内容与探索边界 | [`content_hash_is_reported_but_not_part_of_authoritative_state_hash`](../crates/rfb-core/src/game/tests/persistence.rs#L24) | 保留 contentHash 报告但不参与状态哈希的契约，以及探索记忆独立契约。 |
| 精确随机顺序 | [`crusade_exorcism_rolls_undead_and_demon_damage_independently`](../crates/rfb-core/src/game/tests/high_mage.rs#L1652) | 保留分别掷骰与消耗；同样保护召唤数量／选种、状态次序及矿藏 1/2/2 消耗。 |
| 有限唯一生物的生命周期 | [`nazgul_lifetime_limit_counts_current_and_stored_floors_and_round_trips`](../crates/rfb-core/src/game/tests/monster_ecology.rs#L1159) | 保留当前层、存储层、死亡后的永久消耗及复活例外，不能退成只数当前层。 |
| 无变化与真实增加边界 | [`life_fasting_ends_on_every_real_nutrition_increase`](../crates/rfb-core/src/game/tests/high_mage.rs#L3611) | 满营养时增加为零继续禁食；真实增加才终止，短测试仍有独立价值。 |
| 未知属性与展示隔离 | [`trait_details_withhold_unknown_totals_and_reveal_only_known_properties`](../crates/rfb-core/src/game/tests/trait_details.rs#L327) | 保留未知属性不泄露与鉴定后展示，不能仅比较存档前后 DTO。 |
| 双语规则语义等价 | [`bilingual_defaults_match_every_current_item_kind_equally`](../crates/rfb-core/src/game/mogaminator.rs#L1475) | 保留全部当前物品与六种知识组合；中英文合法规则 AST 不必相同，不能用 AST 相等替代。 |

以上是关键保障的定位入口，完整保留范围仍是逐项清单中的全部断言，并非只保留表内测试。真实缺陷回归优先级最高，但本轮没有逐项建立 issue／修复提交关联；`pNN`、`commitNN` 和 `corrected` 等名称不足以证明历史缺陷来源。未核实来源的条目保持现状。

## 参数变体的合并条件

P 类是候选测试族，不是已经存在的替代测试。一个函数内若还包含独有生命周期、保存或 RNG 检查，该部分要留在具名案例中。只共享相同准备与断言；不为所有系统建立统一测试框架。

| 规则 | 测试族 | 当前入口数 | 后续必须保留 |
|---|---|---:|---|
| P01 | 法术投影公式 | 24 | 按法术、等级、法术强度和半径边界保留命名参数行；混在投影测试中的实际施法断言也须迁入对应效果案例。内容绑定不能代替运行时计算。 |
| P02 | 职业出生档案 | 5 | 共享身份、技能、初始装备检查；各职业独有的熟练度、箭筒、技术和法术领域限制必须保留。已有多职业矩阵仍按原参数行计数。 |
| P03 | 有效种族与被动档案 | 14 | 按正式种族／临时形态／恢复原形保留每行属性、等级阈值和来源断言；存档与奖励检查不能因共用档案准备而移除。 |
| P04 | 同一复苏能力的种族入口 | 2 | Zombie 与 Skeleton 使用同一能力 ID。合并为两种种族的成功／解锁案例；Zombie 的失败扣费、Wisdom、费用和效果 DTO 断言继续保留。Skeleton 入口行不可直接丢弃。 |
| P05 | 种族施法失败的副作用边界 | 3 | 共同准备失败种子和 SP→HP 支付；分别保留时间、HP 差值、探测记忆／金堆、物品序号／数量。通用 mutation 测试目前未包含全部副作用断言，不能充当已完成替代。 |
| P06 | 简单状态的施法适配 | 3 | 只合并状态 ID、独立持续时间与法术强度等共同准备；各状态的可见性、抗性及原持续时间不可缩成“存在状态”。 |
| P07 | 地牢布局断言族 | 13 | 按布局机制组织命名案例：地形比例、永久地形、连通性、楼梯、河流、迷宫等。当前全深度集合先保留；减少深度是单独的覆盖取舍，不能借参数化默默删行。 |
| P08 | 地牢征服公共流程 | 15 | 共享进入、征服一次、领奖、返回、保存恢复的准备；各地牢的守卫、强制神器、替代奖励、激活效果、禁附魔和退休终态仍归各自命名案例。禁止包成一个无分支定位的大通关测试。 |
| P09 | 词缀固定种子实例 | 3 | 保留 source index、种子、物品字段和精确 RNG 次数；按武器／挖掘工具等现有边界共享断言。不能只断言“有词缀”。 |
| P10 | 新存档必填字段 | 4 | 保留每一个被移除的字段以及对应拒绝结果；只共享 JSON 删除字段与解码准备，不添加旧开发存档兼容。 |
| P11 | 无效果／已有状态的物品知晓 | 5 | 保留具体知晓状态、剩余持续时间、免疫、有无资源以及 RNG 差值。公共 noticed 规则与各适配入口都有必要，参数化不表示可以删除入口。 |
| P12 | 地牢怪物分配参数 | 5 | 保留区域锁、守卫排除、偏好权重与水生等例外；已有全种类负例与固定随机序列检查不丢弃。参数化候选只指共同分配入口。 |
| P13 | 已有命名数据矩阵 | 15 | 已是集中断言或数据循环，保留原行；短小的原版数表、公式边界、全别名枚举没有证明值得再压缩。此分类不计作新增可减函数。 |

正文对照中的两个结论：

- P04 的 Skeleton 成功断言是 Zombie 效果断言的子集，但两者是不同种族入口。因此分类为参数变体，保留两行，而非把 Skeleton 整项当作已覆盖删除。
- P05 与通用 mutation 支付测试有重复准备，但探测失败时的记忆／金堆、造食失败时的物品序号／数量分别不同。Kobold 现测试未放置被击目标，只检查时间、资源和序号，不能依据函数名宣称已完整验证“无投射伤害”。

## 低收益抽样的逐项改写依据

本轮未逐项计时；以下次数来自源码，不能当成耗时排行。标注“上限”的循环会提前退出。S01、S05 是固定大循环，适合先改；S03、S06、S08 的搜索能转成清楚的分支样本。

| 规则／现有测试 | 现状 | 必须留下的保障 | 拟改方式与明确取舍 |
|---|---|---|---|
| S01 [`authoritative_heavy_mask_contains_exactly_ten_effects`](../crates/rfb-core/src/game/ego.rs#L2807) | 固定遍历 10,000 个种子 | 十种重诅咒的完整结果集合与禁止结果 | 改为覆盖十种结果的明确种子；另保留选择范围检查。失去其余种子对非法结果的抽样探测，不声称固定十行证明全空间。 |
| S02 [`character_birth_gold_stays_in_the_rfb_range`](../crates/rfb-core/src/game/gold.rs#L224) | 64 个种子仅检查区间 | 有角色构建时的 202..=800 金币范围 | 以公式端点、特殊输入分支及少量种子取代宽泛区间抽样。当前没有完整替代测试，不能直接删除；减少随机输入覆盖。 |
| S03 [`original_neutral_apply_magic_covers_quality_curses_egos_and_damage_dice`](../crates/rfb-core/src/game/tests/archer.rs#L497) | 上限 20,000；找到类别后提前退出 | 普通／优良／重诅咒／杀戮／元素／超充六类、字段约束、超充实例保存及非法折扣拒绝 | 记录触发每类的固定种子，仍走真实 apply_magic；保留存档成功及失败断言。失去搜索途中其他种子的字段检查。 |
| S04 [`p97e_multi_hued_dragon_breath_randomizes_five_elements_across_a_cone`](../crates/rfb-core/src/game/tests/items.rs#L139) | 上限 10,000；集齐五元素后退出 | 五元素、两目标同元素、125/250 衰减、使用后充能为零 | 为五元素各保留成功激活种子和原断言；不再搜索其余种子，不新增“均匀分布已验证”的结论。 |
| S05 [`base_item_natural_egos_cover_completed_weapon_digger_and_ranged_types`](../crates/rfb-core/src/game/tests/tasks.rs#L61) | 固定 20,000 次连续掉落生成 | 武器／挖掘／发射器／弹药／竖琴池、保护词缀、非兼容降级、rolled 字段、受限 subtype | 优先拆成真实 loot 入口的有限命名样本；保持各类别与受限 subtype 的正负断言。各 ego 单元测试只覆盖实例化，不能代替 loot 接线。失去长 RNG 序列上的组合探测。 |
| S06 [`wild_weapon_weight_table_builds_all_fourteen_concrete_status_effects`](../crates/rfb-core/src/game/tests/weapon_traits.rs#L555) | 两处上限 10,000 的种子搜索，均可提前结束 | 十四种状态及载荷、实际派生能力、旧加速覆盖为两回合且不重复 | 固定十四种结果与加速覆盖种子；保留所有载荷和运行时断言。失去搜索途中输入覆盖；不能删成只检查状态数量。 |
| S07 [`warrens_every_generated_floor_has_a_normal_descent_and_return_route`](../crates/rfb-core/src/game/tests/world.rs#L3657) | 16 个种子 × 九层下行及回程 | 每层路线、守卫、遭遇首领数量、物品范围／深度门槛、24 个矿脉及返回地表 | 保留完整一次往返，将其他种子改为直接生成的布局／分配案例。若减少种子，要明确放弃对应种子的跨层联动覆盖；不能只剩楼梯存在断言。 |
| S08 [`streamer_treasure_rolls_known_then_hidden_after_a_miss`](../crates/rfb-core/src/game/world/generation.rs#L4839) | 上限 10,000；集齐三分支后退出 | 已知矿藏先判定；三结果消耗分别为 1、2、2 次 RNG，参数 60／20 | 固定三个分支的种子并保留精确消耗；失去种子搜索覆盖。RNG 顺序保障仍由这三个命名案例负责。 |

这些替代案例本步尚未实现。当前剩余保障就是原测试；只有新案例已包含上表的检查并通过后，才能移除原搜索或遍历。若后续决定直接放弃某个行为，需在此表单独记明，不得写成“由通用测试覆盖”。

不是所有循环都低收益：`energy_curve_is_monotonic_and_bounded` 是对 u16 输入空间的廉价完整枚举，仍保留；具体 RNG 分支种子搜索也不自动判定为冗余。`booze_keeps_a_longer_existing_confusion_duration` 保留更长持续时间及来源，另一个 booze 测试检查延长短状态却不知晓，两者不能互相替代。

## 下一步执行顺序与记录规则

1. 先处理 S01、S05 的固定大循环和 S08 的三个 RNG 分支；记下改前／后实际次数与聚焦测试耗时。先落地真实入口的分支案例，再删除遍历。
2. 用 P04 试做最小两行合并，再推进 P01 的同一公式族；每行失败信息包含种族／法术、等级、强度、种子。禁止一个断言失败后无法定位后续分支的大集合包装。
3. P07、P08 最后处理，先保留每个深度和特殊奖励断言。若选择代表深度而不再全深度，明确列出不再验证的深度及跨层路径。
4. 删除记录必须包含：原函数 ID、保留行为、实际新函数及参数行、明确放弃的覆盖、聚焦验证结果。仅有候选规则编号不足以删测试；不通过 ignored、搬到另一 crate 或改名规避入口预算。

## 本步校验

- `cargo test -p rfb-core --lib -- --list`：904 tests，0 benchmarks；清单与可运行 ID 集合一致，无重复、无遗漏。
- 只新增本审查文档和逐项清单。本步没有运行完整测试或桌面 E2E；前一步的 904 项全通过、92.00 秒是历史基线，不能视为本步重新计时。
- 本轮未改生产行为、内容、协议、状态哈希或 fixture，因此不触发 verify-all／refresh-all。
