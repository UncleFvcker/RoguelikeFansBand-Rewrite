# rfb-core 测试缩减：第二步执行记录

日期：2026-09-09。以[第一步的 904 项快照](core-test-inventory.md)为基线，只压缩能力和物品测试。

后续状态：[第三步](core-test-reduction-stage3.md)已将当前 Core 压缩为 871 项。下文保留第二步完成时的 875 项结果。

## 实际结果

Core 测试入口 **904 → 875（减少 29）**：39 个旧入口由 10 个集中测试及现有专项测试承接。Content 仍为 131 个测试入口，在现有能力验证中补充 128 个能力的固定参数契约。没有把 Core 运行测试移到其他 crate，也没有新增 ignored 测试。

约 670 项是阶段预算，本轮没有达到，仍相差 205 项。用户随后明确：没有其他能删的就不删。本次删改限于已逐段对照并落实替代的部分；第一步的其他候选仍保留，不再为入口预算继续删除，也不能将它们视为已完成压缩。

| 文件 | 测试入口变化 | 行数变化 |
|---|---:|---:|
| [crates/rfb-core/src/game/tests/abilities.rs](../crates/rfb-core/src/game/tests/abilities.rs) | 115 → 110 | -266 |
| [crates/rfb-core/src/game/tests/high_mage.rs](../crates/rfb-core/src/game/tests/high_mage.rs) | 105 → 84 | -1,585 |
| [crates/rfb-core/src/game/tests/hunger.rs](../crates/rfb-core/src/game/tests/hunger.rs) | 26 → 26 | +15 |
| [crates/rfb-core/src/game/tests/items.rs](../crates/rfb-core/src/game/tests/items.rs) | 52 → 49 | -55 |
| [crates/rfb-content/src/tests/abilities.rs](../crates/rfb-content/src/tests/abilities.rs) | 10 → 10 | +60 |
| [crates/rfb-content/src/tests/spell-effect-parameters.json](../crates/rfb-content/src/tests/spell-effect-parameters.json) | 128 条静态参数契约 | +1,026 |

Core 测试源码净减 1,891 行；补充 Content 校验代码 60 行、静态数据 1,026 行后，代码及数据合计净减 805 行。不把函数数等同于行为数：种族、物品、状态、治疗等参数行继续运行。状态 15 行、治疗 8 行、伤害 3 场景 × 3 种法术强度共享各自的准备；等级公式保留偏移、截断、曲线及上限边界；字段路由按效果类型与字段去重，仍检查非目标字段不变。

## 固定参数与运行机制的分工

已有 Content 书籍测试只检查了部分参数，不能直接替代被移除的数值投影检查。因此扩充现有 `abilities_validate_actor_detection_control_and_level_scaling`，读取 `spell-effect-parameters.json`：检查效果类型／次序／数量、固定参数、等级增长系数、法术强度字段及高法师覆盖参数。静态期望来自本轮开始时未修改的内容数据；地形集合按编译器既有排序规范记录。本次不修改内容包、规则或中文名称，也不宣称重新完成原版规则校对。

Core 保留共有数值计算、字段写入和目标范围同步，以及真实效果分派中的治疗、减毒／止血、状态载荷、抗性及掷骰后伤害缩放。现有 `spell_power_uses_shared_formula_and_modifier_sources_in_projection` 继续检查角色来源到投影的集成；大范围睡眠的等级分支仍由 `sorcery_identify_and_mass_sleep_switch_at_the_original_levels` 检查。

明确减少的覆盖：不再逐个法术遍历多个等级／强度并重查整份 DTO；简单状态／治疗参数行改为直接调用真实效果分派器，省去各内容入口重复的职业、学习和费用准备。它们不等价于原来的逐入口端到端抽样；施法阻断、支付、取消和特殊效果继续由具名测试负责。

## 逐项替换账本

以下函数名用于稳定定位；第一步清单中的旧行号是历史位置。

### 资源、种族失败与物品知晓

| 原函数 | 实际替代函数 |
|---|---|
| `formal_zombie_restore_life_unlocks_at_thirty_and_restores_experience_and_life_force` | `undead_restore_life_shares_unlock_payment_and_vitality_rules` |
| `formal_skeleton_restore_life_unlocks_at_thirty_and_restores_vitality` | `undead_restore_life_shares_unlock_payment_and_vitality_rules` |
| `formal_kobold_poison_dart_failure_spills_sp_into_hp_without_projecting` | `racial_cast_failures_pay_without_revealing_or_creating_items` |
| `formal_dwarf_detection_failure_spills_mana_into_hp_without_revealing` | `racial_cast_failures_pay_without_revealing_or_creating_items` |
| `formal_hobbit_create_food_failure_pays_and_creates_nothing` | `racial_cast_failures_pay_without_revealing_or_creating_items` |
| `full_resource_restoration_is_deterministic_and_round_trips` | `resource_restorative_preserves_awareness_order_and_missing_resource_semantics` |
| `successful_restoration_reveals_later_no_effect_events` | `resource_restorative_preserves_awareness_order_and_missing_resource_semantics` |
| `missing_player_resource_consumes_restorative_without_claiming_awareness` | `resource_restorative_preserves_awareness_order_and_missing_resource_semantics` |
| `slowness_potion_refreshes_existing_slow_without_becoming_aware` | `status_item_refresh_preserves_source_and_does_not_make_the_kind_aware` |
| `temperate_tonic_extends_existing_resistance_without_becoming_aware` | `status_item_refresh_preserves_source_and_does_not_make_the_kind_aware` |

- `undead_restore_life_shares_unlock_payment_and_vitality_rules`：Zombie／Skeleton 两行；原解锁、费用、成功效果及失败扣费全部保留，Skeleton 也检查失败。
- `racial_cast_failures_pay_without_revealing_or_creating_items`：三种失败支付均保留原时间／HP／SP；隐藏矿藏、金堆、生成物品序号和数量在每行检查。
- `resource_restorative_preserves_awareness_order_and_missing_resource_semantics`：同一物品三种资源／状态前置：保留恢复 DTO、保存快照、事件命名及 missing resource 不知晓。
- `status_item_refresh_preserves_source_and_does_not_make_the_kind_aware`：缓慢与冷热抗性两行，保留持续时间、状态来源、抗性、一次 RNG、物品消耗、Tried 知晓及对应无效果事件。共享初始角色；缓慢案例的无关世界初始化种子由 82 改为 85，持续时间仍强制原最大掷骰。

### 固定参数投影

下列 14 项的固定内容参数归 Content 契约；运行时计算由 `ability_level_curves_preserve_offset_rounding_and_cap_boundaries`、`ability_scaling_changes_only_the_selected_effect_field`、`level_and_spell_power_scaling_keep_effect_and_target_ranges_in_sync` 及上面的既有投影集成承接。

- `death_abilities_materialize_player_level_scaling_in_projection`
- `corrected_death_spells_project_authoritative_values_at_levels_one_twenty_and_fifty`
- `death_second_book_materializes_original_mage_scaling_and_beam_profile`
- `death_third_book_materializes_original_scaling_and_prorated_cap`
- `death_fourth_book_materializes_original_level_curves`
- `crusade_first_book_projects_original_level_and_spell_power_formulas`
- `crusade_second_book_projects_original_level_damage_duration_and_spell_power`
- `life_first_book_projects_final_healing_light_and_status_formulas`
- `life_fourth_book_projects_original_power_and_duration_formulas`
- `nature_first_book_projects_level_and_spell_power_formulas`
- `nature_second_book_projects_bolts_entangle_and_fixed_healing`
- `armageddon_first_book_projects_original_level_beam_and_damage_formulas`
- `armageddon_second_book_projects_original_level_beam_and_damage_formulas`
- `armageddon_advanced_books_projects_original_formulas_and_breath_radius_boundary`

### 状态、治疗和伤害

下列 15 项的固定数值投影归同一 Content 契约；其中实际状态、治疗及伤害行为分别归 `self_status_spells_share_duration_payload_and_passive_rules`、`curing_spells_share_healing_and_fractional_status_reduction`、`bolt_and_area_spells_apply_power_after_the_damage_roll` 的命名参数行。

- `daemon_first_book_projects_original_level_and_spell_power_formulas`
- `daemon_second_book_projects_and_resolves_original_damage_formulas`
- `daemon_third_book_projects_original_formulas_and_high_mage_parameters`
- `daemon_fourth_book_projects_original_formulas_and_support_effects`
- `life_first_book_applies_blessing_regeneration_and_cures`
- `nature_first_book_applies_food_levitation_environment_and_curing`
- `nature_herbal_healing_scales_fixed_healing_and_cures_statuses`
- `commit32_nature_third_book_projects_and_applies_stone_skin_and_shared_resistance`
- `commit33_nature_fourth_book_projects_original_damage_radius_and_spell_power`
- `sorcery_third_book_statuses_use_the_original_spell_powered_durations`
- `arcane_resist_cold_and_fire_create_independent_spell_powered_statuses`
- `arcane_fourth_book_statuses_keep_see_invisible_separate_from_sight`
- `crusade_purification_uses_the_original_poison_reduction_and_cures_cut_and_stun`
- `arcane_cure_poison_uses_the_original_fractional_reduction`
- `arcane_cure_medium_wounds_uses_spell_powered_healing_and_original_bleeding_formula`

混合测试内的特殊检查另有明确归属：

- 原恶魔第三书的 40 级变身吐息投影迁入 `daemon_polymorph_demon_overlays_race_preserves_body_and_enables_breath`，保留原 32 级变身／实际吐息，再检查 40 级来源、费用和两个锥形分支。
- 生命系饱食能力的营养上限归既有 `hunger::satisfy_hunger_sets_food_to_the_original_maximum_minus_one`，与物品饱食分别调用真实分派器。
- 自然系造食的固定物品绑定归 Content；实际造物与重复堆叠由既有 `create_item_ability_places_an_acquired_item_and_merges_repeated_casts` 检查。减少了自然系造食入口的单独运行抽样。

## 保留的独特机制

检测与传送的专项测试没有发现可完整替代的共同断言，本轮保留，包括：

- `formal_dwarf_detection_powers_reveal_original_terrain_categories_only`、`arcane_magic_item_detection_uses_instance_identity_and_enchantment`、`arcane_door_trap_detection_remembers_stairs_through_walls`：分类过滤、实例身份与穿墙记忆。
- `crusade_banish_evil_filters_visible_targets_and_uses_teleport_resistance_path`、`arcane_teleport_away_beams_through_monsters_and_honors_original_resistance`：可见性／阵营过滤和传送抗性。
- `sorcery_dimension_door_cancellation_is_atomic_and_failed_steps_cost_extra_energy`、`sorcery_dimension_door_success_uses_one_failure_roll_and_one_extra_energy_charge`、`travel_scroll_random_teleport_is_deterministic_and_rejects_without_space_atomically`：取消、失败额外能量、确定性与无空间拒绝。
- `spell_blocking_statuses_reject_without_spending_resources`、`mutation_cast_spills_sp_into_hp_and_keeps_rejections_atomic`：拒绝与支付原子性。
- `commit33_natures_wrath_direction_prompt_is_atomic_cancelable_and_persistent`、`crusade_exorcism_rolls_undead_and_demon_damage_independently`：交互状态持久化与独立随机顺序。
- 神圣之言、神圣干预、恶魔光环／混乱圆环、变身、终极抗性等独特组合继续保留自己的运行测试。

## 验证

- 所有新增集中测试、变身吐息承接测试、饱食承接测试及扩充的 Content 参数验证均已单独通过。
- `cargo fmt --all` 已执行。
- `cargo test -p rfb-core -p rfb-content --lib`：Core **875 通过**，0 失败／忽略，96.28 秒；Content **131 通过**，0 失败／忽略，52.04 秒。历史 Core 基线为 904 项、92.00 秒；两次并非受控基准，本次没有测出总运行时间收益，不宣称提速。
- 本轮没有修改生产代码、协议、初始化、RNG 或状态哈希输入，也没有修改内容包及回放 fixture；不触发 verify-all／refresh-all。未运行桌面 E2E。
