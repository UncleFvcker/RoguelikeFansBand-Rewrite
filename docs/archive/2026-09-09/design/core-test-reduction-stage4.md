> 历史快照（2026-09-09 归档）：本文保留当时的设计、版本与验收记录，不作为当前工作指令或待办。现行说明见 [文档索引](../../../README.md)。

# rfb-core 测试缩减：第四步执行记录

日期：2026-09-09。基线为[第三步](core-test-reduction-stage3.md)完成后的 871 项。范围为成长、出生和战斗测试。

## 实际结果

Core **871 → 867 项**，七个测试文件合计净减 **377 行**。7 个种族／资源吸取旧测试合并为 3 个机制测试；4 个职业出生测试重组为 1 个共享出生矩阵和 3 个职业专项测试，这部分入口数不变。没有新增 ignored 测试或迁移到其他 crate。

约 320 项预算没有达到。依照用户“没有其他能删的就不删”的要求，没有为了入口数删除独有机制保障，也没有将仅改名或搬动的测试计作删除。

| 文件 | 测试数 | 源码行数变化 |
|---|---:|---:|
| [progression.rs](../../../../crates/rfb-core/src/game/tests/progression.rs) | 43 → 41 | -103 |
| [combat.rs](../../../../crates/rfb-core/src/game/tests/combat.rs) | 38 → 37 | +6 |
| [archer.rs](../../../../crates/rfb-core/src/game/tests/archer.rs) | 10 → 10 | -72 |
| [cavalry.rs](../../../../crates/rfb-core/src/game/tests/cavalry.rs) | 3 → 3 | -61 |
| [paladin.rs](../../../../crates/rfb-core/src/game/tests/paladin.rs) | 3 → 2 | -68 |
| [sniper.rs](../../../../crates/rfb-core/src/game/tests/sniper.rs) | 17 → 17 | -59 |
| [support.rs](../../../../crates/rfb-core/src/game/tests/support.rs) | 0 → 0 | -20 |

删除了合并后无调用的 `archon_game`、`shadow_fairy_game` 构造辅助函数。本步只改测试及执行记录，没有修改玩法、内容包、协议或存档。

## 种族与成长矩阵

以下函数均位于 `game::tests::progression`。

| 原测试 | 替代测试 | 保留的行为 |
|---|---|---|
| `formal_zombie_creation_and_temporary_form_apply_and_remove_intrinsics`、`formal_skeleton_creation_and_temporary_form_apply_and_remove_intrinsics` | `undead_race_intrinsics_share_cold_unlock_and_temporary_form_lifecycle` | 2 个种族行；正式身份，冷抗等级 4/5 与 9/10，毒抗及各自幽冥／碎片抗性，生命保持、看隐形、非生命属性，30 级临时形态与能力增加／移除。 |
| `formal_archon_and_temporary_form_apply_and_remove_static_passives`、`formal_sprite_and_temporary_form_apply_level_speed_and_static_passives`、`formal_shadow_fairy_and_temporary_form_apply_and_remove_intrinsics` | `flying_races_share_passive_application_removal_and_preserve_unique_traits` | 3 个种族行；正式和临时形态的飞行、感知、光抗或弱点，保存恢复及哈希。Sprite 9/10/19/20 级速度和睡眠粉能力，Shadow-Fairy 潜行仍分别检查。 |
| `formal_race_selection_changes_the_warrior_profile_and_defaults_to_human` | 原函数内改为 5 个种族参数行 | Half-Orc、Dunadan、Barbarian、Hobbit、High-Elf 的原有属性／技能差值、生命值比较、商店价格与经验倍率断言；默认 Human 的状态哈希、RNG 等价和无效种族拒绝保留。 |
| `high_elf_intrinsics_and_identity_round_trip` 的出生属性部分 | 上一行的 High-Elf 参数行 | 六项属性差值移入共用矩阵。原函数继续检查红外、看隐形、光抗、无待选奖励及 Finrod 身份的完整快照／哈希往返。 |

原来的种族被动测试保留种族、种子与等级。Sprite 速度四行现在复用一个角色、只改变等级。High-Elf 属性差值的样本由种子 84／Finrod 改为种子 83／默认名字；种子 84／Finrod 的种族固有属性与保存测试仍在。这里不宣称保留全部原输入组合。

未改动种族升级奖励、锁定突变、临时种族不发出生奖励、奖励取消／拒绝、职业技能成长及属性点非法操作等专项测试，包括 `race_level_mutation_rewards_are_derived_locked_and_zero_time`、`draconian_level_35_reward_revalidates_all_nine_completed_powers` 和各 30 级天赋测试。

## 职业出生去重

新增 `game::tests::progression::class_birth_applies_attributes_skills_and_equipped_kit_from_the_selected_build`，用 Archer、Paladin、Cavalry、Sniper 四行共用断言，保留原种子与原先检查的属性、技能基础值／成长、身份投影、生命／经验倍率、装备位置及弹药数量。Paladin 的初始书仍检查存在；原先未检查的属性用 `None` 明示，不推测新期望值。

| 原测试 | 剩余专项测试 |
|---|---|
| `archer_birth_uses_the_original_class_identity_skills_and_kit` | `archer_birth_projects_ammunition_creation_level_gates`：3 个职业能力、分组及各等级可用性。 |
| `cavalry_birth_uses_original_identity_skills_proficiencies_and_kit` | `cavalry_birth_projects_proficiencies_and_rodeo`：骑乘／短弓熟练度投影及 Rodeo 等级、目标模式、范围、效果和不可施放边界。 |
| `sniper_birth_uses_original_identity_skills_proficiencies_kit_and_techniques` | `sniper_birth_projects_proficiencies_and_techniques`：骑乘／弩熟练度、狙击系数及 17 个能力的等级、代价、分组和效果。 |
| `death_paladin_birth_uses_the_original_class_identity_skills_and_kit` | 全部并入共享出生矩阵；同模块的神术学习、法力、原始法术表与实战测试保留。 |

移除 Cavalry 的 `pet_upkeep_divisor`、`riding_combat_expert`、`mounted_non_arrow_base_shot_cap` 以及 Sniper 的 `pet_upkeep_divisor` 静态断言：这些固定字段已经由 `rfb-content/src/tests/catalog.rs` 直接检查。职业运行时属性、技能和装备投影不能由静态目录检查替代，因此继续放在 Core 矩阵中。

这部分减少重复代码，不声称减少初始化工作：原四个出生测试合计创建 4 个角色，重组后矩阵创建 4 个、专项测试创建 3 个，共 7 个。High-Mage 已有八领域隔离矩阵保持原样。

## 战斗分支与准备成本

以下函数位于 `game::tests::combat`。

`resource_drain_melee_heals_six_times_the_amount_actually_drained` 与 `percent_gated_resource_drain_uses_level_power_and_heals_the_caster` 合并为 `melee_resource_drain_shares_gate_exhaustion_and_actual_healing_rules`。两种怪物效果配置各编译一次，克隆执行七行：

| 配置 | 参数行 | 期望 |
|---|---|---|
| 无门控、1d1 | 资源 1 | 耗尽 1，回复 6。 |
| 无门控、1d1 | 空资源池 | 吸取与回复均为 0。 |
| 无门控、1d1 | 无资源池 | 吸取与回复均为 0，不创建资源池。 |
| 25% 门控、1d25 | 资源 25，原始 RNG 种子 1 | 吸取 9，回复 54。 |
| 25% 门控、1d25 | 资源 3，原始 RNG 种子 1 | 只吸取实际剩余的 3，回复 18。 |
| 25% 门控、1d25 | 最大 HP 10，原始 RNG 种子 1 | 回复封顶至 10。 |
| 25% 门控、1d25 | 原始 RNG 种子 0 | 门控拒绝，资源与 HP 不变。 |

全部经过真实怪物近战解析器，检查未出现未命中事件、最终资源、怪物 HP 和精确 RNG 消耗。保留原成功与耗尽行为，新增空池、缺失池、门控拒绝及封顶边界。

`content_driven_fire_melee_uses_the_player_resistance_profile` 保留函数名，改为一次编译后克隆五行。固定原始 RNG 种子 2、请求伤害 2，检查弱点／普通／抗性／强抗／免疫的实际伤害分别为 3／2／1／1／0，以及对应 HP。原普通／抗性比较继续覆盖，新增其余三个等级。

原资源门控测试最多搜索 100 个世界种子，火焰测试最多搜索 1,000 个；每次尝试均重编译测试内容。现在直接准备命名分支，不再执行搜索途中输入或世界初始化。上述上限不是原运行的实际尝试次数，也不是随机分布验证；本步不以搜索上限计算性能收益。

独特战斗机制继续保持独立，包括免疫／豁免不消耗形态 RNG、接触光环抵抗、致死光环的共享死亡事务、死亡爆炸、非生命吸血例外、生命保持、资源装置耗尽、地震伤害阈值与复仇不可递归。

## 验证

- 第一批两个种族矩阵与两个战斗矩阵分别通过聚焦测试。
- `cargo test -p rfb-core --lib birth_`：17 项通过。
- `cargo test -p rfb-core --lib`：**867 passed，0 failed，0 ignored**，63.77 秒。
- `cargo fmt --all -- --check`、`git diff --check`：通过；Git 仅报告工作区既有文件的换行转换提示。

上一阶段完整 Core 运行是 68.20 秒，本轮为 63.77 秒。两者只是同机各一次普通测试运行，不能当作稳定性能基准。本步未更改状态哈希输入、共享投影、生产初始化、RNG 或内容包，未刷新合同 fixture，未运行桌面 E2E。
