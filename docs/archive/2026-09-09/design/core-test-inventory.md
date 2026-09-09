> 历史快照（2026-09-09 归档）：本文保留当时的设计、版本与验收记录，不作为当前工作指令或待办。现行说明见 [文档索引](../../../README.md)。

# rfb-core 测试逐项分类（2026-09-09）

对应 [审查结论与保留规则](core-test-reduction-audit.md)。基线：第一步工作区快照的 904 个可运行测试；不包含此前已删除的测试。

第二步为 875 项，第三步为 871 项，第四步为 867 项，第五步当前为 863 项，删除和替代函数见[第二步执行记录](core-test-reduction-stage2.md)、[第三步执行记录](core-test-reduction-stage3.md)、[第四步执行记录](core-test-reduction-stage4.md)及[第五步执行记录](core-test-reduction-stage5.md)。本文件保留原始分类与行号，作为替换账本的对照基线。

每行对应一个 Cargo 完整测试 ID。分类是保留决策，不是删除授权。`K`：无已证明替代，保留；`Pxx`：保留全部行为，考虑共享准备／命名参数行；`Sxx`：先建立固定分支保障，再缩减抽样。所有规则见审查结论。

证据“正文”表示本轮对该函数完整正文复核；“初筛”表示按函数职责、调用、循环及断言位置分类，未证明整项可删。未标记历史缺陷来源的测试不能被宣称为已确认缺陷回归，也不能据此删除。源码行号仅适用于本次工作区快照，函数 ID 用于后续定位。

## `crates/rfb-core/src/check.rs`（3）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`check::tests::check_result_keeps_context_rolls_and_threshold`](../../../../crates/rfb-core/src/check.rs#L163) | 独立机制 | K | 初筛 |
| [`check::tests::non_positive_ability_fails_without_a_contest_roll`](../../../../crates/rfb-core/src/check.rs#L178) | 独立机制 | K | 初筛 |
| [`check::tests::forced_failure_is_checked_after_the_automatic_percentile_bands`](../../../../crates/rfb-core/src/check.rs#L194) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/effect.rs`（5）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`effect::tests::elemental_resistance_uses_deterministic_integer_reduction`](../../../../crates/rfb-core/src/effect.rs#L413) | 参数变体 | P13 | 初筛 |
| [`effect::tests::armor_and_resistance_reductions_remain_separate_in_the_outcome`](../../../../crates/rfb-core/src/effect.rs#L446) | 独立机制 | K | 初筛 |
| [`effect::tests::status_application_is_sorted_and_obeys_explicit_stacking`](../../../../crates/rfb-core/src/effect.rs#L460) | 独立机制 | K | 初筛 |
| [`effect::tests::effect_pipeline_mutates_only_the_supplied_authoritative_target`](../../../../crates/rfb-core/src/effect.rs#L489) | 独立机制 | K | 初筛 |
| [`effect::tests::status_ticks_expire_in_stable_kind_order`](../../../../crates/rfb-core/src/effect.rs#L534) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/event.rs`（3）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`event::tests::typed_events_project_to_the_existing_protocol_contract`](../../../../crates/rfb-core/src/event.rs#L5826) | 独立机制 | K | 初筛 |
| [`event::tests::numeric_domain_values_are_formatted_only_at_the_dto_boundary`](../../../../crates/rfb-core/src/event.rs#L5842) | 独立机制 | K | 初筛 |
| [`event::tests::digging_failure_projects_its_repeat_decision`](../../../../crates/rfb-core/src/event.rs#L5860) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/capabilities/healing.rs`（1）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::capabilities::healing::tests::healing_clamps_requested_and_applied_amounts`](../../../../crates/rfb-core/src/game/capabilities/healing.rs#L38) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/capabilities/resources.rs`（1）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::capabilities::resources::tests::restoration_reports_missing_bounded_and_full_resource_outcomes`](../../../../crates/rfb-core/src/game/capabilities/resources.rs#L72) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/capabilities/statuses.rs`（1）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::capabilities::statuses::tests::status_application_and_removal_report_source_neutral_outcomes`](../../../../crates/rfb-core/src/game/capabilities/statuses.rs#L77) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/damage.rs`（3）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::damage::tests::fatality_policy_preserves_player_and_actor_zero_hp_distinction`](../../../../crates/rfb-core/src/game/damage.rs#L250) | 独立机制 | K | 初筛 |
| [`game::damage::tests::application_saturates_hp_and_only_wakes_surviving_damaged_targets`](../../../../crates/rfb-core/src/game/damage.rs#L263) | 独立机制 | K | 初筛 |
| [`game::damage::tests::no_air_damage_ramps_with_elapsed_ticks_and_bypasses_physical_resistance`](../../../../crates/rfb-core/src/game/damage.rs#L287) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/ego.rs`（27）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::ego::tests::fixed_weapon_ego_activations_materialize_one_full_charge`](../../../../crates/rfb-core/src/game/ego.rs#L2335) | 独立机制 | K | 初筛 |
| [`game::ego::tests::biased_activation_selection_uses_source_order_weights_and_depth`](../../../../crates/rfb-core/src/game/ego.rs#L2403) | 独立机制 | K | 初筛 |
| [`game::ego::tests::daemon_bias_activation_overrides_fixed_destruction_in_exact_rng_order`](../../../../crates/rfb-core/src/game/ego.rs#L2444) | 独立机制 | K | 初筛 |
| [`game::ego::tests::mage_bias_activation_belongs_to_arcane_not_mana`](../../../../crates/rfb-core/src/game/ego.rs#L2491) | 独立机制 | K | 初筛 |
| [`game::ego::tests::basic_weapon_egos_materialize_fixed_seed_results`](../../../../crates/rfb-core/src/game/ego.rs#L2549) | 参数变体 | P09 | 初筛 |
| [`game::ego::tests::fire_brand_adds_light_and_c_rolls_are_independent`](../../../../crates/rfb-core/src/game/ego.rs#L2633) | 独立机制 | K | 初筛 |
| [`game::ego::tests::special_weapon_egos_materialize_fixed_seed_results`](../../../../crates/rfb-core/src/game/ego.rs#L2667) | 参数变体 | P09 | 初筛 |
| [`game::ego::tests::morgul_and_death_materialize_concrete_heavy_curses_and_darkness`](../../../../crates/rfb-core/src/game/ego.rs#L2744) | 独立机制 | K | 初筛 |
| [`game::ego::tests::authoritative_heavy_mask_contains_exactly_ten_effects`](../../../../crates/rfb-core/src/game/ego.rs#L2807) | 低收益抽样 | S01 | 正文 |
| [`game::ego::tests::special_weapon_ego_restrictions_reject_without_partial_state`](../../../../crates/rfb-core/src/game/ego.rs#L2831) | 独立机制 | K | 初筛 |
| [`game::ego::tests::shared_slaying_helper_keeps_ammunition_rng_contract`](../../../../crates/rfb-core/src/game/ego.rs#L2860) | 独立机制 | K | 初筛 |
| [`game::ego::tests::digger_egos_materialize_fixed_seed_results`](../../../../crates/rfb-core/src/game/ego.rs#L2878) | 参数变体 | P09 | 初筛 |
| [`game::ego::tests::incompatible_digger_ego_retries_before_atomic_materialization`](../../../../crates/rfb-core/src/game/ego.rs#L2924) | 独立机制 | K | 初筛 |
| [`game::ego::tests::all_weapon_and_digger_source_indices_have_a_materialization_branch`](../../../../crates/rfb-core/src/game/ego.rs#L2964) | 独立机制 | K | 初筛 |
| [`game::ego::tests::ego_weight_matches_below_in_range_and_above_max_penalties`](../../../../crates/rfb-core/src/game/ego.rs#L3002) | 独立机制 | K | 初筛 |
| [`game::ego::tests::ego_materialization_preserves_roll_then_activation_rng_order`](../../../../crates/rfb-core/src/game/ego.rs#L3009) | 独立机制 | K | 初筛 |
| [`game::ego::tests::ego_materialization_commits_complete_instance_state_only_after_success`](../../../../crates/rfb-core/src/game/ego.rs#L3041) | 独立机制 | K | 初筛 |
| [`game::ego::tests::rolled_weapon_ego_state_round_trips_without_rng_draws`](../../../../crates/rfb-core/src/game/ego.rs#L3126) | 独立机制 | K | 初筛 |
| [`game::ego::tests::ranged_materialization_state_is_atomic_projected_and_save_stable`](../../../../crates/rfb-core/src/game/ego.rs#L3183) | 独立机制 | K | 初筛 |
| [`game::ego::tests::ordinary_harp_rolls_intrinsic_charisma_and_is_not_a_projectile_launcher`](../../../../crates/rfb-core/src/game/ego.rs#L3323) | 独立机制 | K | 初筛 |
| [`game::ego::tests::harp_egos_reuse_base_pval_and_round_trip_without_rerolling`](../../../../crates/rfb-core/src/game/ego.rs#L3364) | 独立机制 | K | 初筛 |
| [`game::ego::tests::ammunition_egos_share_dynamic_helpers_typed_behaviors_and_supercharge`](../../../../crates/rfb-core/src/game/ego.rs#L3467) | 独立机制 | K | 初筛 |
| [`game::ego::tests::basic_launcher_egos_follow_authoritative_rng_and_profile_order`](../../../../crates/rfb-core/src/game/ego.rs#L3545) | 独立机制 | K | 初筛 |
| [`game::ego::tests::launcher_ego_profile_uses_final_multiplier_range_and_shot_rate`](../../../../crates/rfb-core/src/game/ego.rs#L3646) | 独立机制 | K | 初筛 |
| [`game::ego::tests::restricted_launcher_egos_retry_without_partial_rolls`](../../../../crates/rfb-core/src/game/ego.rs#L3736) | 独立机制 | K | 初筛 |
| [`game::ego::tests::ego_selection_uses_source_order_for_duplicate_names_and_one_draw`](../../../../crates/rfb-core/src/game/ego.rs#L3870) | 独立机制 | K | 初筛 |
| [`game::ego::tests::ego_selection_matches_any_type_and_excludes_zero_rarity_or_missing_metadata`](../../../../crates/rfb-core/src/game/ego.rs#L3910) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/gold.rs`（2）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::gold::tests::character_birth_gold_stays_in_the_rfb_range`](../../../../crates/rfb-core/src/game/gold.rs#L224) | 低收益抽样 | S02 | 正文 |
| [`game::gold::tests::mining_gold_uses_original_level_and_amount_bonuses`](../../../../crates/rfb-core/src/game/gold.rs#L238) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/item_curses.rs`（5）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::item_curses::tests::equipped_aggravation_wakes_sleepers_until_its_curse_is_removed`](../../../../crates/rfb-core/src/game/item_curses.rs#L158) | 独立机制 | K | 初筛 |
| [`game::item_curses::tests::shadow_fairy_form_converts_equipped_aggravation_into_a_stealth_penalty`](../../../../crates/rfb-core/src/game/item_curses.rs#L193) | 独立机制 | K | 初筛 |
| [`game::item_curses::tests::intrinsic_ego_drawbacks_survive_without_a_curse_severity`](../../../../crates/rfb-core/src/game/item_curses.rs#L255) | 独立机制 | K | 初筛 |
| [`game::item_curses::tests::darkness_is_an_intrinsic_equipment_radius_penalty`](../../../../crates/rfb-core/src/game/item_curses.rs#L279) | 独立机制 | K | 初筛 |
| [`game::item_curses::tests::random_teleport_checks_once_per_rfb_world_interval_and_stops_after_uncursing`](../../../../crates/rfb-core/src/game/item_curses.rs#L313) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/mining.rs`（4）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::mining::tests::original_mining_gain_formulas_use_feature_power_and_floor_depth`](../../../../crates/rfb-core/src/game/mining.rs#L379) | 独立机制 | K | 初筛 |
| [`game::mining::tests::mining_item_mode_uses_original_scaled_adjacent_thresholds`](../../../../crates/rfb-core/src/game/mining.rs#L395) | 参数变体 | P13 | 初筛 |
| [`game::mining::tests::mining_artifact_mode_accepts_a_twentieth_attempt_without_allocating_discarded_drafts`](../../../../crates/rfb-core/src/game/mining.rs#L420) | 独立机制 | K | 初筛 |
| [`game::mining::tests::mining_artifact_mode_falls_back_once_to_great_when_all_artifacts_are_ineligible`](../../../../crates/rfb-core/src/game/mining.rs#L457) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/mogaminator.rs`（10）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::mogaminator::tests::character_keeps_independent_bilingual_sources_and_applies_atomically`](../../../../crates/rfb-core/src/game/mogaminator.rs#L1255) | 独立机制 | K | 初筛 |
| [`game::mogaminator::tests::auto_get_uses_original_projectability_distance_and_stable_id_order`](../../../../crates/rfb-core/src/game/mogaminator.rs#L1306) | 独立机制 | K | 初筛 |
| [`game::mogaminator::tests::wanted_auto_get_does_not_reveal_hidden_gold`](../../../../crates/rfb-core/src/game/mogaminator.rs#L1368) | 独立机制 | K | 初筛 |
| [`game::mogaminator::tests::auto_get_moves_one_step_then_picks_up_ammo_without_time`](../../../../crates/rfb-core/src/game/mogaminator.rs#L1396) | 独立机制 | K | 初筛 |
| [`game::mogaminator::tests::auto_get_rejects_stale_id_and_collects_only_the_locked_gold`](../../../../crates/rfb-core/src/game/mogaminator.rs#L1436) | 独立机制 | K | 初筛 |
| [`game::mogaminator::tests::bilingual_defaults_match_every_current_item_kind_equally`](../../../../crates/rfb-core/src/game/mogaminator.rs#L1475) | 独立机制 | K | 初筛 |
| [`game::mogaminator::tests::original_conditions_compare_against_every_candidate`](../../../../crates/rfb-core/src/game/mogaminator.rs#L1583) | 独立机制 | K | 初筛 |
| [`game::mogaminator::tests::enabled_rules_return_the_first_match`](../../../../crates/rfb-core/src/game/mogaminator.rs#L1594) | 独立机制 | K | 初筛 |
| [`game::mogaminator::tests::query_rules_round_trip_and_rejection_is_not_repeated`](../../../../crates/rfb-core/src/game/mogaminator.rs#L1629) | 独立机制 | K | 初筛 |
| [`game::mogaminator::tests::deferred_predicates_use_content_build_book_and_corpse_data`](../../../../crates/rfb-core/src/game/mogaminator.rs#L1682) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/monster_combat.rs`（1）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::monster_combat::nice_melee_tests::spawn_grace_limits_only_rolls_above_fifty`](../../../../crates/rfb-core/src/game/monster_combat.rs#L2378) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/monster_ecology.rs`（1）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::monster_ecology::w3_tests::daylight_excludes_light_vulnerable_wilderness_monsters`](../../../../crates/rfb-core/src/game/monster_ecology.rs#L932) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/player_combat.rs`（5）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::player_combat::tests::ammunition_damage_and_bonus_are_scaled_before_launcher_bonus`](../../../../crates/rfb-core/src/game/player_combat.rs#L2847) | 独立机制 | K | 初筛 |
| [`game::player_combat::tests::concentration_reduces_armor_and_scales_critical_chance_before_bolt_bonus`](../../../../crates/rfb-core/src/game/player_combat.rs#L2856) | 独立机制 | K | 初筛 |
| [`game::player_combat::tests::elemental_and_shining_sniper_multipliers_follow_original_focus_rules`](../../../../crates/rfb-core/src/game/player_combat.rs#L2863) | 独立机制 | K | 初筛 |
| [`game::player_combat::tests::advanced_sniper_multipliers_use_the_stronger_special_or_ammunition_modifier`](../../../../crates/rfb-core/src/game/player_combat.rs#L2954) | 独立机制 | K | 初筛 |
| [`game::player_combat::tests::explosion_radius_and_needle_nested_rng_follow_original_boundaries`](../../../../crates/rfb-core/src/game/player_combat.rs#L3032) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/progression.rs`（2）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::progression::tests::normal_appearance_masks_only_mutation_charisma_and_applies_its_level_floor`](../../../../crates/rfb-core/src/game/progression.rs#L1092) | 独立机制 | K | 初筛 |
| [`game::progression::tests::progression_capabilities_report_bounded_source_neutral_outcomes`](../../../../crates/rfb-core/src/game/progression.rs#L1162) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/riding_proficiency.rs`（2）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::riding_proficiency::tests::riding_uses_its_own_original_rank_thresholds`](../../../../crates/rfb-core/src/game/riding_proficiency.rs#L504) | 独立机制 | K | 初筛 |
| [`game::riding_proficiency::tests::rodeo_unique_level_adjustment_precedes_high_level_compression`](../../../../crates/rfb-core/src/game/riding_proficiency.rs#L518) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/abilities.rs`（115）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::abilities::spell_blocking_statuses_reject_without_spending_resources`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L11) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::damage_bonus_adds_flat_amount_to_monster_cast_damage`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L53) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::breath_damage_scales_with_caster_hp_and_caps_at_max`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L126) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::spawned_entities_get_content_declared_resistances_stamped`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L197) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::death_abilities_materialize_player_level_scaling_in_projection`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L273) | 参数变体 | P01 | 初筛 |
| [`game::tests::abilities::spell_power_uses_shared_formula_and_modifier_sources_in_projection`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L339) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::malediction_resolves_all_riders_and_skips_the_d1000_when_not_triggered`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L460) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::corrected_death_spells_project_authoritative_values_at_levels_one_twenty_and_fifty`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L619) | 参数变体 | P01 | 初筛 |
| [`game::tests::abilities::death_vampiric_drain_heals_and_feeds_up_to_the_original_caps`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L666) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::death_second_book_materializes_original_mage_scaling_and_beam_profile`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L708) | 参数变体 | P01 | 初筛 |
| [`game::tests::abilities::death_weapon_branding_targets_plain_weapons_across_player_locations`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L798) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::death_weapon_branding_rejects_nonplain_or_unavailable_weapons_without_rng`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L931) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::death_third_book_materializes_original_scaling_and_prorated_cap`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L973) | 参数变体 | P01 | 初筛 |
| [`game::tests::abilities::berserk_and_battle_frenzy_roll_independent_durations_and_round_trip`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L1088) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::vampirism_true_retraces_the_path_after_each_kill`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L1175) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::invoke_spirits_scales_every_source_formula_without_nested_random_effects`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L1238) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::invoke_spirits_resolves_all_twenty_three_branches_deterministically`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L1423) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::invoke_spirits_lowest_outcome_updates_chance_and_unlife`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L1609) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::bolt_or_beam_damage_uses_one_roll_and_changes_only_penetration`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L1654) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::p86e_mirror_shield_reflects_monster_bolts_once_with_exact_three_of_four_gate`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L1756) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::mirror_shield_does_not_reflect_beams_balls_or_breaths`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L1864) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::reflecting_monsters_redirect_only_single_target_bolts`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L1951) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::reflected_rock_uses_the_original_shards_and_sound_riders`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L2078) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::rock_projectiles_destroy_trees_and_cold_vulnerable_ground_items`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L2168) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::genocide_erases_without_rewards_or_corpses_and_uniques_resist`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L2199) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::ordinary_death_creates_a_corpse_and_animate_dead_consumes_it_persistently`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L2304) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::monster_animate_dead_consumes_failed_remains_and_spawns_hostile_summons`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L2394) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::sleep_power_resolves_then_skips_energy_and_damage_wakes_the_target`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L2480) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::temporary_status_resistances_apply_expire_and_round_trip`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L2591) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::magic_affinity_and_strong_mind_gate_existing_dispel_and_resource_drain_effects`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L2664) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::hand_of_doom_uses_a_save_gated_nonlethal_percentage_of_current_hp`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L2734) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::waiting_and_resting_recover_mana_until_the_pool_is_full`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L2796) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::natural_regeneration_and_rest_restore_warrior_health`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L2856) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::formal_ogre_sustains_intelligence_and_places_capped_explosive_runes`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L2909) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::explosive_rune_step_explodes_or_is_destroyed_by_the_authoritative_roll`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L3039) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::formal_snotling_devours_flesh_while_confused_and_round_trips`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L3144) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::formal_boit_vomits_poison_while_afraid_or_confused_and_pays_empty_stomach_energy`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L3230) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::formal_einheri_halves_shared_healing_but_keeps_full_natural_regeneration`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L3384) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::snotling_mushroom_boost_follows_the_effective_race`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L3475) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::formal_sprite_sleeping_dust_switches_from_adjacent_to_visible_at_twenty_five`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L3573) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::formal_wood_elf_nature_awareness_unlocks_at_twenty_and_reuses_full_detection`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L3699) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::formal_kutar_expansion_fixes_saving_throw_and_adds_thirty_five_armor`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L3816) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::formal_amberite_passives_and_powers_match_the_authoritative_behavior`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L3958) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::formal_golem_stone_skin_unlocks_at_twenty_without_spell_power_scaling`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L4169) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::formal_zombie_restore_life_unlocks_at_thirty_and_restores_experience_and_life_force`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L4285) | 参数变体 | P04 | 正文 |
| [`game::tests::abilities::formal_skeleton_restore_life_unlocks_at_thirty_and_restores_vitality`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L4388) | 参数变体 | P04 | 正文 |
| [`game::tests::abilities::race_ability_follows_the_effective_race_and_projects_its_source`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L4437) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::formal_kobold_poison_dart_is_a_fixed_level_poison_bolt_without_ammunition`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L4515) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::formal_kobold_poison_dart_failure_spills_sp_into_hp_without_projecting`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L4615) | 参数变体 | P05 | 正文 |
| [`game::tests::abilities::kobold_intrinsics_follow_the_effective_race`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L4668) | 参数变体 | P03 | 初筛 |
| [`game::tests::abilities::formal_dwarf_detection_powers_reveal_original_terrain_categories_only`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L4733) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::formal_nibelung_intrinsics_and_detection_powers_unlock_at_level_ten`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L4946) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::formal_gnome_phase_door_is_distinct_from_the_sorcery_spell`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L5018) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::formal_half_giant_stone_to_mud_does_not_grant_mining_rewards`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L5088) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::formal_half_troll_regeneration_and_berserk_follow_the_effective_race`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L5166) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::half_titan_probe_knowledge_survives_losing_the_race_power_and_reloading`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L5267) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::cyclops_throw_boulder_scales_stuns_and_round_trips_deterministically`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L5363) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::yeek_scare_monster_and_level_acid_immunity_follow_the_effective_race`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L5588) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::klackon_acid_spit_and_speed_growth_follow_the_effective_race`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L5849) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::dark_elf_magic_missile_capacity_and_sight_follow_the_effective_race`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L6105) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::mindflayer_mind_blast_sustains_and_senses_follow_the_effective_race`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L6323) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::imp_fire_upgrade_and_demon_traits_follow_the_effective_race`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L6537) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::draconian_breath_uses_current_hp_maturity_shape_and_deadly_upgrade`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L6827) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::draconian_strike_applies_elemental_stun_confusion_vorpal_and_vampiric_modes`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L6967) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::formal_dwarf_detection_failure_spills_mana_into_hp_without_revealing`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L7088) | 参数变体 | P05 | 正文 |
| [`game::tests::abilities::dwarf_intrinsics_follow_the_effective_race_without_replacing_birth_rewards`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L7161) | 参数变体 | P03 | 初筛 |
| [`game::tests::abilities::racial_berserk_pays_hp_obeys_fear_and_never_shortens_a_stronger_rage`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L7245) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::formal_barbarian_berserk_spills_sp_into_hp_pays_on_failure_and_rejects_zero_budget`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L7326) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::formal_hobbit_create_food_projects_and_round_trips_an_acquired_ration`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L7445) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::formal_hobbit_create_food_failure_pays_and_creates_nothing`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L7534) | 参数变体 | P05 | 正文 |
| [`game::tests::abilities::hobbit_intrinsics_follow_the_effective_race`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L7586) | 参数变体 | P03 | 初筛 |
| [`game::tests::abilities::create_item_ability_places_an_acquired_item_and_merges_repeated_casts`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L7737) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::create_item_ability_uses_rfb_nearby_scoring_and_failure_creates_nothing`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L7815) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::active_mutation_projects_without_learning_progress_or_persistent_cooldown`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L7884) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::fear_blocks_class_and_mutation_power_sources_without_cost_or_rng`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L7949) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::mutation_cast_spills_sp_into_hp_and_keeps_rejections_atomic`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L8002) | 独立机制 | K | 正文 |
| [`game::tests::abilities::mutation_level_and_failure_paths_do_not_create_ability_progress`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L8125) | 独立机制 | K | 正文 |
| [`game::tests::abilities::active_mutation_batches_project_scaled_costs_and_effects`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L8180) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::mutation_eat_rock_and_midas_touch_commit_their_narrow_transactions`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L8343) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::mutation_eat_magic_and_weigh_magic_use_existing_device_and_status_state`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L8421) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::mutation_grow_mold_and_sterility_persist_only_authoritative_state`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L8538) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::mutation_earthquake_panic_hit_and_polymorph_enforce_their_boundaries`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L8581) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::mutation_telekinesis_and_swap_position_reuse_directional_targeting`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L8714) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::mutation_detection_recall_and_resistance_use_existing_authoritative_state`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L8811) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::mutation_vampirism_feeds_without_crossing_the_original_full_cap`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L8890) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::mutation_spit_acid_changes_from_bolt_to_area_at_level_twenty_five`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L8924) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::blink_other_moves_the_target_within_ten_tiles_using_one_destination_draw`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L8971) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::level_based_jump_damage_uses_no_damage_rng_then_blinks`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L9046) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::bird_drop_flies_away_or_drops_targets_with_levitation_reduction`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L9150) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::p55b_eagle_summon_includes_unseen_unique_eagles`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L9352) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::p75a_no_summon_monsters_are_rejected_by_shared_candidate_filter`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L9397) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::p56b_gospel_summon_caps_one_d_four_at_three_tracking_pixels`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L9415) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::p60_gragomani_rolls_count_then_one_weighted_kind_for_the_whole_batch`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L9477) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::p70_aegir_rolls_count_then_floods_then_selects_one_retinue_kind`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L9557) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::p79_special_summons_keep_hermes_count_and_odin_retinue_choice`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L9650) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::p80_variant_maintainer_cast_summons_only_software_bugs`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L9727) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::p71_banor_rupart_split_and_merge_preserve_hp_without_recording_deaths`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L9775) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::p71_banor_rupart_split_requires_one_adjacent_open_cell`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L9888) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::monster_polymorph_reuses_mutation_and_actor_form_transactions`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L9915) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::death_fourth_book_materializes_original_level_curves`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L10059) | 参数变体 | P01 | 初筛 |
| [`game::tests::abilities::raise_dead_is_deterministic_and_enforces_faction_group_and_unique_rules`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L10158) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::vampiric_transformation_overlays_race_but_preserves_body_slots`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L10267) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::p76_unique_summons_use_the_caster_level_window_and_exclude_unique2`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L10327) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::p76_osiris_family_summon_creates_horus_and_isis_as_one_cast`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L10368) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::p83_gertrude_summons_each_available_sister_once`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L10410) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::p76_air_breath_is_unresisted_and_levitation_reduces_damage_by_one_quarter`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L10472) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::p76_chicken_deals_flat_damage_and_applies_sound_stun_and_fear`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L10531) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::p76_no_air_applies_once_for_forty_ticks`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L10572) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::p77_dead_unique_resurrection_preserves_the_spent_lifetime_slot`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L10657) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::p77_dead_unique_summon_disintegrates_radius_five_and_falls_back_to_star_blades`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L10712) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::ground_item_elements_respect_ignore_flags_and_artifact_protection`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L10777) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::hell_fire_destroys_only_cursed_ground_items`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L10815) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::ground_item_destruction_is_ordered_by_position_then_instance_id`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L10835) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::shattered_potion_runs_its_area_program_after_removal`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L10869) | 独立机制 | K | 初筛 |
| [`game::tests::abilities::shattered_potion_healing_uses_area_falloff`](../../../../crates/rfb-core/src/game/tests/abilities.rs#L10891) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/archer.rs`（10）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::archer::archer_birth_uses_the_original_class_identity_skills_and_kit`](../../../../crates/rfb-core/src/game/tests/archer.rs#L14) | 参数变体 | P02 | 初筛 |
| [`game::tests::archer::equipped_quiver_carries_sixty_ammunition_outside_the_pack`](../../../../crates/rfb-core/src/game/tests/archer.rs#L123) | 独立机制 | K | 初筛 |
| [`game::tests::archer::archer_makes_original_quantity_ammunition_from_terrain_and_skeletons`](../../../../crates/rfb-core/src/game/tests/archer.rs#L154) | 独立机制 | K | 初筛 |
| [`game::tests::archer::archer_breakage_and_projectile_critical_hooks_are_active`](../../../../crates/rfb-core/src/game/tests/archer.rs#L241) | 独立机制 | K | 初筛 |
| [`game::tests::archer::archer_shooting_energy_and_heavy_launcher_rules_match_original`](../../../../crates/rfb-core/src/game/tests/archer.rs#L277) | 独立机制 | K | 初筛 |
| [`game::tests::archer::blindness_blocks_create_ammunition_without_consuming_the_source`](../../../../crates/rfb-core/src/game/tests/archer.rs#L375) | 独立机制 | K | 初筛 |
| [`game::tests::archer::create_ammunition_accepts_original_junk_bones_and_skeleton_corpses_on_floor_or_pack`](../../../../crates/rfb-core/src/game/tests/archer.rs#L406) | 独立机制 | K | 初筛 |
| [`game::tests::archer::original_neutral_apply_magic_covers_quality_curses_egos_and_damage_dice`](../../../../crates/rfb-core/src/game/tests/archer.rs#L497) | 低收益抽样 | S03 | 正文 |
| [`game::tests::archer::archer_ammunition_uses_one_shared_dynamic_roll_and_persists_the_stack`](../../../../crates/rfb-core/src/game/tests/archer.rs#L579) | 独立机制 | K | 初筛 |
| [`game::tests::archer::player_made_ammunition_dice_and_rolled_brands_feed_the_projectile_profile`](../../../../crates/rfb-core/src/game/tests/archer.rs#L639) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/attribute_sources.rs`（4）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::attribute_sources::attribute_sources_follow_calculation_order_without_mutating_state`](../../../../crates/rfb-core/src/game/tests/attribute_sources.rs#L27) | 独立机制 | K | 初筛 |
| [`game::tests::attribute_sources::attribute_sources_hide_unknown_equipment_and_reveal_only_after_identification`](../../../../crates/rfb-core/src/game/tests/attribute_sources.rs#L76) | 独立机制 | K | 初筛 |
| [`game::tests::attribute_sources::attribute_sources_project_caps_normal_appearance_and_unwell_phases`](../../../../crates/rfb-core/src/game/tests/attribute_sources.rs#L173) | 独立机制 | K | 初筛 |
| [`game::tests::attribute_sources::attribute_sources_track_damaged_attributes_and_real_status_expiry`](../../../../crates/rfb-core/src/game/tests/attribute_sources.rs#L245) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/capture_ball.rs`（5）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::capture_ball::capture_policy_and_health_gates_preserve_rng_until_a_real_attempt`](../../../../crates/rfb-core/src/game/tests/capture_ball.rs#L54) | 独立机制 | K | 初筛 |
| [`game::tests::capture_ball::captured_mount_falls_resets_bond_and_releases_as_a_new_pet`](../../../../crates/rfb-core/src/game/tests/capture_ball.rs#L105) | 独立机制 | K | 初筛 |
| [`game::tests::capture_ball::blocked_release_keeps_the_ball_and_drop_uses_the_exact_hostility_roll`](../../../../crates/rfb-core/src/game/tests/capture_ball.rs#L165) | 独立机制 | K | 初筛 |
| [`game::tests::capture_ball::drop_and_destruction_release_the_actor_before_finishing_the_item_lifecycle`](../../../../crates/rfb-core/src/game/tests/capture_ball.rs#L210) | 独立机制 | K | 初筛 |
| [`game::tests::capture_ball::captured_state_round_trips_projects_details_and_regenerates_on_schedule`](../../../../crates/rfb-core/src/game/tests/capture_ball.rs#L261) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/cavalry.rs`（3）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::cavalry::cavalry_birth_uses_original_identity_skills_proficiencies_and_kit`](../../../../crates/rfb-core/src/game/tests/cavalry.rs#L45) | 参数变体 | P02 | 初筛 |
| [`game::tests::cavalry::rodeo_mounts_and_tames_a_wild_adjacent_monster`](../../../../crates/rfb-core/src/game/tests/cavalry.rs#L147) | 独立机制 | K | 初筛 |
| [`game::tests::cavalry::guardian_and_questor_mounts_are_thrown_off_without_becoming_pets`](../../../../crates/rfb-core/src/game/tests/cavalry.rs#L187) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/combat.rs`（38）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::combat::p62_polymorph_immunity_and_successful_save_do_not_draw_a_form_or_duration`](../../../../crates/rfb-core/src/game/tests/combat.rs#L63) | 独立机制 | K | 初筛 |
| [`game::tests::combat::p62_polymorph_preserves_legacy_branches_rejection_rng_and_temporary_state`](../../../../crates/rfb-core/src/game/tests/combat.rs#L108) | 独立机制 | K | 初筛 |
| [`game::tests::combat::p62_polymorph_melee_changes_the_player_without_polymorphing_the_attacker`](../../../../crates/rfb-core/src/game/tests/combat.rs#L221) | 独立机制 | K | 初筛 |
| [`game::tests::combat::p62_polymorph_reconciles_body_slots_and_expiry_does_not_reequip_items`](../../../../crates/rfb-core/src/game/tests/combat.rs#L254) | 独立机制 | K | 初筛 |
| [`game::tests::combat::p60_melee_curse_damage_uses_the_existing_monster_curse_save`](../../../../crates/rfb-core/src/game/tests/combat.rs#L354) | 独立机制 | K | 初筛 |
| [`game::tests::combat::mutation_contact_auras_retaliate_only_against_unresisted_contact_attacks`](../../../../crates/rfb-core/src/game/tests/combat.rs#L384) | 独立机制 | K | 初筛 |
| [`game::tests::combat::ultimate_resistance_reuses_fire_electricity_and_cold_contact_auras`](../../../../crates/rfb-core/src/game/tests/combat.rs#L459) | 独立机制 | K | 初筛 |
| [`game::tests::combat::effectless_beg_always_succeeds_without_damage_contact_or_rng`](../../../../crates/rfb-core/src/game/tests/combat.rs#L491) | 独立机制 | K | 初筛 |
| [`game::tests::combat::monster_contact_auras_apply_elemental_damage_and_curse_saves`](../../../../crates/rfb-core/src/game/tests/combat.rs#L532) | 独立机制 | K | 初筛 |
| [`game::tests::combat::monster_revenge_aura_uses_one_blow_and_cannot_recurse`](../../../../crates/rfb-core/src/game/tests/combat.rs#L622) | 独立机制 | K | 初筛 |
| [`game::tests::combat::fatal_mutation_aura_uses_the_shared_actor_death_transaction`](../../../../crates/rfb-core/src/game/tests/combat.rs#L693) | 独立机制 | K | 初筛 |
| [`game::tests::combat::innate_critical_roll_uses_original_weight_level_and_quality_bands`](../../../../crates/rfb-core/src/game/tests/combat.rs#L724) | 独立机制 | K | 初筛 |
| [`game::tests::combat::zero_dice_hurt_hits_without_dealing_damage`](../../../../crates/rfb-core/src/game/tests/combat.rs#L741) | 独立机制 | K | 初筛 |
| [`game::tests::combat::resource_drain_melee_heals_six_times_the_amount_actually_drained`](../../../../crates/rfb-core/src/game/tests/combat.rs#L768) | 独立机制 | K | 初筛 |
| [`game::tests::combat::percent_gated_resource_drain_uses_level_power_and_heals_the_caster`](../../../../crates/rfb-core/src/game/tests/combat.rs#L795) | 独立机制 | K | 初筛 |
| [`game::tests::combat::inertia_melee_uses_minor_slow_and_free_action_reduces_it`](../../../../crates/rfb-core/src/game/tests/combat.rs#L827) | 独立机制 | K | 初筛 |
| [`game::tests::combat::amberite_death_can_curse_equipment_and_apply_multiple_nonlethal_ty_curses`](../../../../crates/rfb-core/src/game/tests/combat.rs#L858) | 独立机制 | K | 初筛 |
| [`game::tests::combat::variant_maintainer_death_leaves_four_software_bugs`](../../../../crates/rfb-core/src/game/tests/combat.rs#L924) | 独立机制 | K | 初筛 |
| [`game::tests::combat::bomb_death_explosion_splits_sound_and_shards_with_status_riders`](../../../../crates/rfb-core/src/game/tests/combat.rs#L968) | 独立机制 | K | 初筛 |
| [`game::tests::combat::slow_death_explosion_uses_radius_free_action_and_monster_saves`](../../../../crates/rfb-core/src/game/tests/combat.rs#L1045) | 独立机制 | K | 初筛 |
| [`game::tests::combat::charge_drain_melee_consumes_a_carried_device_or_player_nutrition`](../../../../crates/rfb-core/src/game/tests/combat.rs#L1126) | 独立机制 | K | 初筛 |
| [`game::tests::combat::vampiric_melee_heals_from_applied_damage_but_not_from_nonliving_players`](../../../../crates/rfb-core/src/game/tests/combat.rs#L1223) | 独立机制 | K | 初筛 |
| [`game::tests::combat::shatter_melee_uses_the_shared_earthquake_only_above_the_damage_threshold`](../../../../crates/rfb-core/src/game/tests/combat.rs#L1265) | 独立机制 | K | 初筛 |
| [`game::tests::combat::gaze_projects_the_melee_routine_to_a_distant_target`](../../../../crates/rfb-core/src/game/tests/combat.rs#L1310) | 独立机制 | K | 初筛 |
| [`game::tests::combat::melee_amnesia_uses_the_existing_save_and_floor_memory_wipe`](../../../../crates/rfb-core/src/game/tests/combat.rs#L1344) | 独立机制 | K | 初筛 |
| [`game::tests::combat::dice_less_time_uses_exp_or_fractional_attribute_ravaging_without_damage`](../../../../crates/rfb-core/src/game/tests/combat.rs#L1380) | 独立机制 | K | 初筛 |
| [`game::tests::combat::unlife_melee_drains_life_force_and_persistently_empowers_the_monster`](../../../../crates/rfb-core/src/game/tests/combat.rs#L1446) | 独立机制 | K | 初筛 |
| [`game::tests::combat::hold_life_can_save_against_unlife_without_changing_life_force_or_monster_power`](../../../../crates/rfb-core/src/game/tests/combat.rs#L1537) | 独立机制 | K | 初筛 |
| [`game::tests::combat::disenchant_melee_removes_positive_status_or_reduces_equipment_enchantments`](../../../../crates/rfb-core/src/game/tests/combat.rs#L1595) | 独立机制 | K | 初筛 |
| [`game::tests::combat::haste_and_slow_modify_scheduler_speed_without_changing_base_speed`](../../../../crates/rfb-core/src/game/tests/combat.rs#L1684) | 独立机制 | K | 初筛 |
| [`game::tests::combat::bleeding_ticks_as_physical_damage_in_stable_status_order`](../../../../crates/rfb-core/src/game/tests/combat.rs#L1735) | 独立机制 | K | 初筛 |
| [`game::tests::combat::content_driven_fire_melee_uses_the_player_resistance_profile`](../../../../crates/rfb-core/src/game/tests/combat.rs#L1788) | 独立机制 | K | 初筛 |
| [`game::tests::combat::explicit_empty_melee_routine_performs_no_attack`](../../../../crates/rfb-core/src/game/tests/combat.rs#L1835) | 独立机制 | K | 初筛 |
| [`game::tests::combat::item_theft_splits_a_stack_into_monster_carried_loot_and_blinks`](../../../../crates/rfb-core/src/game/tests/combat.rs#L1861) | 独立机制 | K | 初筛 |
| [`game::tests::combat::gold_theft_uses_the_original_amount_and_dexterity_protection`](../../../../crates/rfb-core/src/game/tests/combat.rs#L1904) | 独立机制 | K | 初筛 |
| [`game::tests::combat::food_and_light_eating_consume_one_food_and_leave_one_light_fuel`](../../../../crates/rfb-core/src/game/tests/combat.rs#L1944) | 独立机制 | K | 初筛 |
| [`game::tests::combat::leader_death_dissolves_pack_before_remaining_members_act`](../../../../crates/rfb-core/src/game/tests/combat.rs#L1999) | 独立机制 | K | 初筛 |
| [`game::tests::combat::rfb_style_armor_reduction_uses_the_legacy_linear_cap`](../../../../crates/rfb-core/src/game/tests/combat.rs#L2059) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/generation.rs`（1）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::generation::free_room_placement_uses_the_full_floor_without_overlap`](../../../../crates/rfb-core/src/game/tests/generation.rs#L7) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/gold.rs`（3）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::gold::warrens_floor_gold_is_seeded_walkable_and_persistent`](../../../../crates/rfb-core/src/game/tests/gold.rs#L9) | 独立机制 | K | 初筛 |
| [`game::tests::gold::seeing_gold_discovers_and_refreshes_its_cell`](../../../../crates/rfb-core/src/game/tests/gold.rs#L33) | 独立机制 | K | 初筛 |
| [`game::tests::gold::invalid_gold_state_and_allocator_are_rejected`](../../../../crates/rfb-core/src/game/tests/gold.rs#L65) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/high_mage.rs`（105）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::high_mage::high_mage_birth_preserves_the_shared_class_kit_and_isolates_each_realm`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L407) | 参数变体 | P02 | 初筛 |
| [`game::tests::high_mage::daemon_first_book_projects_original_level_and_spell_power_formulas`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L566) | 参数变体 | P01 | 初筛 |
| [`game::tests::high_mage::crusade_first_book_projects_original_level_and_spell_power_formulas`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L701) | 参数变体 | P01 | 初筛 |
| [`game::tests::high_mage::crusade_second_book_projects_original_level_damage_duration_and_spell_power`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L769) | 参数变体 | P01 | 初筛 |
| [`game::tests::high_mage::crusade_advanced_books_project_original_power_damage_and_compound_formulas`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L921) | 参数变体 | P01 | 初筛 |
| [`game::tests::high_mage::crusade_banish_evil_filters_visible_targets_and_uses_teleport_resistance_path`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L1040) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::crusade_wrath_of_the_god_drops_eleven_to_twenty_disintegration_balls`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L1094) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::crusade_divine_intervention_resolves_damage_controls_then_healing`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L1126) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::crusade_final_spell_hastens_good_pets_scares_failures_and_grants_battle_buffs`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L1187) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::crusade_arrest_uses_raw_power_and_only_rolls_for_non_unique_evil_targets`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L1282) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::crusade_angelic_cloak_grants_resistances_and_harms_only_evil_contact_attackers`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L1367) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::crusade_holy_blade_adds_a_permanent_slay_evil_affix`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L1434) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::crusade_summon_angel_preserves_one_in_three_hostility_and_fixed_level_cap`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L1475) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::crusade_scatter_evil_stops_at_the_first_actor_and_filters_non_evil_targets`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L1529) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::crusade_holy_orb_uses_original_alignment_damage`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L1594) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::crusade_exorcism_rolls_undead_and_demon_damage_independently`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L1652) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::crusade_holy_word_resolves_damage_then_healing_then_status_cures`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L1688) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::crusade_sanctuary_reaches_unseen_adjacent_monsters_but_not_distant_or_immune_ones`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L1769) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::crusade_stardust_resolves_ten_independent_reflectable_light_bolts`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L1839) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::crusade_purification_uses_the_original_poison_reduction_and_cures_cut_and_stun`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L1919) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::daemon_second_book_projects_and_resolves_original_damage_formulas`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L1965) | 参数变体 | P01 | 初筛 |
| [`game::tests::high_mage::daemon_third_book_projects_original_formulas_and_high_mage_parameters`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L2081) | 参数变体 | P01 | 初筛 |
| [`game::tests::high_mage::daemon_devilish_cloak_grants_three_resistances_and_a_fire_contact_aura`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L2249) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::daemon_flow_of_lava_centers_damage_and_rewrites_non_permanent_terrain`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L2306) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::daemon_polymorph_demon_overlays_race_preserves_body_and_enables_breath`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L2378) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::daemon_doom_hand_preserves_unique_immunity_save_rng_and_current_hp_percentage`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L2456) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::daemon_fourth_book_projects_original_formulas_and_support_effects`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L2574) | 参数变体 | P01 | 初筛 |
| [`game::tests::high_mage::daemon_insanity_circle_applies_both_balls_then_confusion_and_charm`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L2694) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::daemon_explode_pets_processes_instance_order_and_lets_uniques_escape`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L2744) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::daemon_greater_demon_consumes_only_a_successful_humanoid_sacrifice`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L2818) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::daemon_hellfire_resolves_good_target_damage_before_life_backlash`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L2894) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::daemon_send_to_hell_and_demonlord_form_keep_their_terminal_semantics`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L2941) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::daemon_summon_demon_preserves_one_in_three_dynamic_level_and_group_boundary`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L3006) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::daemon_summon_manes_preserves_group_pet_and_failed_summon_semantics`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L3120) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::daemon_hellish_flame_doubles_good_damage_destroys_curses_and_cancels_atomically`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L3175) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::life_first_book_projects_final_healing_light_and_status_formulas`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L3275) | 参数变体 | P01 | 初筛 |
| [`game::tests::high_mage::life_first_book_applies_final_healing_and_light_after_the_roll`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L3349) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::life_first_book_applies_blessing_regeneration_and_cures`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L3407) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::life_fasting_starts_atomically_persists_and_recasts_for_free`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L3511) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::life_fasting_uses_the_original_three_roll_gate_and_all_eight_restorations`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L3558) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::life_fasting_ends_on_every_real_nutrition_increase`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L3611) | 独立机制 | K | 正文 |
| [`game::tests::high_mage::life_second_book_reuses_curse_healing_resistance_mapping_and_glyph_transactions`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L3625) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::life_turn_undead_uses_level_power_and_only_changes_unlife_after_success`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L3785) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::life_third_book_projects_and_resolves_dispel_undead_without_a_damage_roll`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L3845) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::life_sustain_attributes_uses_the_original_order_and_one_shared_duration`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L3915) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::life_cure_mutation_prefers_harmful_mutations_and_preserves_locked_mutations`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L3967) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::life_transcendence_absorbs_direct_and_status_damage_with_mana_first`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L4011) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::life_warding_true_creates_the_current_and_eight_adjacent_glyphs_atomically`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L4082) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::life_fourth_book_projects_original_power_and_duration_formulas`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L4126) | 参数变体 | P01 | 初筛 |
| [`game::tests::high_mage::life_fourth_book_sterilization_and_clairvoyance_match_rng_and_side_effect_boundaries`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L4188) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::life_fourth_book_annihilate_undead_filters_targets_and_changes_virtues_on_success`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L4240) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::life_fourth_book_restoration_and_true_healing_restore_the_original_state_sets`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L4309) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::life_fourth_book_ultimate_resistance_feeds_shared_player_passive_pipelines`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L4377) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::nature_first_book_projects_level_and_spell_power_formulas`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L4475) | 参数变体 | P01 | 初筛 |
| [`game::tests::high_mage::nature_first_book_applies_food_levitation_environment_and_curing`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L4529) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::nature_daylight_burns_an_unprotected_vampire_form`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L4625) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::nature_second_book_projects_bolts_entangle_and_fixed_healing`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L4654) | 参数变体 | P01 | 初筛 |
| [`game::tests::high_mage::nature_entangle_uses_the_original_unique_immunity_and_old_slow_save`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L4721) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::nature_gate_uses_all_three_level_bands_and_creates_upkeep_pets`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L4779) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::nature_herbal_healing_scales_fixed_healing_and_cures_statuses`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L4848) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::commit32_nature_third_book_projects_and_applies_stone_skin_and_shared_resistance`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L4893) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::commit32_nature_forest_and_stone_wall_share_adjacent_terrain_rules`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L4989) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::commit32_nature_corrosion_protection_is_permanent_visible_and_location_agnostic`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L5052) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::commit32_nature_call_sunlight_maps_lights_reveals_without_esp_and_burns_vampires`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L5242) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::commit33_nature_fourth_book_projects_original_damage_radius_and_spell_power`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L5302) | 参数变体 | P01 | 初筛 |
| [`game::tests::high_mage::commit33_natures_wrath_selects_all_six_branches_and_orders_the_elemental_storms`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L5398) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::commit33_natures_wrath_direction_prompt_is_atomic_cancelable_and_persistent`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L5461) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::armageddon_first_book_projects_original_level_beam_and_damage_formulas`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L5591) | 参数变体 | P01 | 初筛 |
| [`game::tests::high_mage::armageddon_second_book_projects_original_level_beam_and_damage_formulas`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L5668) | 参数变体 | P01 | 初筛 |
| [`game::tests::high_mage::armageddon_special_projectiles_share_original_resistance_status_and_cell_rules`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L5726) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::armageddon_advanced_books_projects_original_formulas_and_breath_radius_boundary`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L5919) | 参数变体 | P01 | 初筛 |
| [`game::tests::high_mage::armageddon_breath_damage_matches_projection_and_affects_items_and_terrain`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L6121) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::armageddon_ice_and_water_use_original_resistance_and_stun_rules`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L6210) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::armageddon_sound_and_inertia_use_distinct_original_monster_riders`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L6284) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::armageddon_disintegration_cone_crosses_destructible_terrain_but_stops_at_permanent_terrain`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L6410) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::sorcery_identify_and_mass_sleep_switch_at_the_original_levels`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L6517) | 参数变体 | P01 | 初筛 |
| [`game::tests::high_mage::sorcery_mass_identify_appraises_all_carried_items`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L6587) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::sorcery_mass_stasis_suspends_visible_non_unique_monsters_only`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L6617) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::sorcery_third_book_statuses_use_the_original_spell_powered_durations`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L6691) | 参数变体 | P06 | 初筛 |
| [`game::tests::high_mage::sorcery_self_knowledge_reuses_the_read_only_character_report`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L6734) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::sorcery_teleport_town_lists_only_visited_destinations_and_moves_without_a_fare`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L6759) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::sorcery_dimension_door_cancellation_is_atomic_and_failed_steps_cost_extra_energy`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L6802) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::sorcery_dimension_door_success_uses_one_failure_roll_and_one_extra_energy_charge`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L6857) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::sorcery_create_stair_respects_surface_and_permanent_terrain`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L6899) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::sorcery_fourth_book_projection_uses_level_and_spell_power`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L6990) | 参数变体 | P01 | 初筛 |
| [`game::tests::high_mage::sorcery_probe_reveals_true_identity_and_create_door_uses_only_empty_floor`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L7122) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::sorcery_device_mastery_banish_and_invulnerability_commit_shared_rules`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L7226) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::arcane_phlogiston_adds_half_capacity_and_caps_an_equipped_light`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L7438) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::arcane_cure_poison_uses_the_original_fractional_reduction`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L7485) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::arcane_resist_cold_and_fire_create_independent_spell_powered_statuses`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L7538) | 参数变体 | P06 | 初筛 |
| [`game::tests::high_mage::arcane_magic_item_detection_uses_instance_identity_and_enchantment`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L7584) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::arcane_door_trap_detection_remembers_stairs_through_walls`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L7624) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::arcane_first_book_jams_and_destroys_doors_and_cures_light_wounds`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L7658) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::astral_guide_reduces_successful_arcane_blink_energy_to_one_third`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L7750) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::arcane_cure_medium_wounds_uses_spell_powered_healing_and_original_bleeding_formula`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L7780) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::arcane_identify_performs_basic_identification_without_an_extra_rng_roll`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L7816) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::arcane_stone_to_mud_uses_the_rock_power_roll_and_preserves_permanent_walls`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L7845) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::arcane_fourth_book_statuses_keep_see_invisible_separate_from_sight`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L7910) | 参数变体 | P06 | 初筛 |
| [`game::tests::high_mage::arcane_teleport_away_beams_through_monsters_and_honors_original_resistance`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L7953) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::arcane_recharging_is_atomic_and_keeps_player_failure_separate_from_device_explosion`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L8049) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::arcane_detection_recall_and_level_teleport_reuse_existing_transactions`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L8235) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::arcane_clairvoyance_maps_lights_reveals_and_grants_conditional_telepathy`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L8313) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::death_high_mage_cannot_study_foreign_realms`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L8442) | 独立机制 | K | 初筛 |
| [`game::tests::high_mage::death_high_mage_projects_original_mana_and_spell_table`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L8468) | 参数变体 | P01 | 初筛 |
| [`game::tests::high_mage::death_high_mage_damage_bonus_and_level_twenty_five_power_are_active`](../../../../crates/rfb-core/src/game/tests/high_mage.rs#L8554) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/hunger.rs`（26）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::hunger::warrior_birth_rolls_five_to_nine_rations_after_gold`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L11) | 独立机制 | K | 初筛 |
| [`game::tests::hunger::formal_golem_birth_replaces_rations_with_a_full_nothing_staff_and_keeps_torches`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L47) | 独立机制 | K | 初筛 |
| [`game::tests::hunger::formal_snotling_birth_adds_one_to_three_fast_recovery_mushrooms`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L77) | 独立机制 | K | 初筛 |
| [`game::tests::hunger::undead_birth_starts_at_night_without_rations_and_round_trips`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L105) | 独立机制 | K | 初筛 |
| [`game::tests::hunger::formal_zombie_uses_undead_food_and_device_metabolism`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L150) | 独立机制 | K | 初筛 |
| [`game::tests::hunger::formal_skeleton_food_magic_drop_rules_and_potion_shatter_match_rfb`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L226) | 独立机制 | K | 初筛 |
| [`game::tests::hunger::formal_skeleton_temporary_form_controls_food_fallthrough`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L348) | 独立机制 | K | 初筛 |
| [`game::tests::hunger::ration_use_consumes_one_restores_food_and_pays_normal_action_cost`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L398) | 独立机制 | K | 初筛 |
| [`game::tests::hunger::ration_caps_at_maximum_before_bloated_world_processing`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L439) | 独立机制 | K | 初筛 |
| [`game::tests::hunger::satisfy_hunger_sets_food_to_the_original_maximum_minus_one`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L469) | 独立机制 | K | 初筛 |
| [`game::tests::hunger::hallucination_food_applies_status_drains_mana_then_adds_nutrition`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L497) | 独立机制 | K | 初筛 |
| [`game::tests::hunger::sleep_potion_uses_the_paralysis_status`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L544) | 独立机制 | K | 初筛 |
| [`game::tests::hunger::elvish_waybread_uses_normal_and_intolerant_branches`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L571) | 独立机制 | K | 初筛 |
| [`game::tests::hunger::salt_water_affects_living_players_but_is_inert_for_nonliving_players`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L684) | 独立机制 | K | 初筛 |
| [`game::tests::hunger::fast_recovery_mushroom_heals_eases_bleeding_and_grants_timed_regeneration`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L769) | 独立机制 | K | 初筛 |
| [`game::tests::hunger::digestion_uses_world_tick_and_current_scheduler_speed`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L829) | 独立机制 | K | 初筛 |
| [`game::tests::hunger::golem_slow_digestion_and_food_magic_follow_construct_metabolism`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L851) | 独立机制 | K | 初筛 |
| [`game::tests::hunger::golem_absorbs_inventory_and_floor_devices_without_consuming_them`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L935) | 独立机制 | K | 初筛 |
| [`game::tests::hunger::golem_device_absorption_round_trips_and_replays_deterministically`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L1054) | 独立机制 | K | 初筛 |
| [`game::tests::hunger::empty_device_absorption_spends_a_turn_without_changing_energy_or_food`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L1088) | 独立机制 | K | 初筛 |
| [`game::tests::hunger::ultimate_resistance_slow_digestion_halves_normal_food_use`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L1130) | 独立机制 | K | 初筛 |
| [`game::tests::hunger::wait_and_rest_share_the_hunger_world_clock`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L1143) | 独立机制 | K | 初筛 |
| [`game::tests::hunger::nutrition_thresholds_apply_rfb_regeneration_factors`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L1162) | 参数变体 | P13 | 初筛 |
| [`game::tests::hunger::fainting_rolls_once_and_skips_rng_while_already_paralyzed`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L1178) | 独立机制 | K | 初筛 |
| [`game::tests::hunger::starvation_damage_precedes_recovery_and_can_kill`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L1206) | 独立机制 | K | 初筛 |
| [`game::tests::hunger::warrens_ration_attempts_are_deterministic_walkable_and_persistent`](../../../../crates/rfb-core/src/game/tests/hunger.rs#L1228) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/inventory.rs`（16）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::inventory::fabric_bag_adds_four_shared_inventory_slots`](../../../../crates/rfb-core/src/game/tests/inventory.rs#L5) | 独立机制 | K | 初筛 |
| [`game::tests::inventory::armor_hit_modifier_only_changes_melee_skill`](../../../../crates/rfb-core/src/game/tests/inventory.rs#L33) | 独立机制 | K | 初筛 |
| [`game::tests::inventory::gauntlets_add_their_hit_and_damage_modifiers_to_melee`](../../../../crates/rfb-core/src/game/tests/inventory.rs#L56) | 独立机制 | K | 初筛 |
| [`game::tests::inventory::shovel_equips_as_a_tool_without_replacing_the_melee_weapon_profile`](../../../../crates/rfb-core/src/game/tests/inventory.rs#L79) | 独立机制 | K | 初筛 |
| [`game::tests::inventory::original_diggers_use_weight_and_tunneling_pval_without_stacking_with_weapons`](../../../../crates/rfb-core/src/game/tests/inventory.rs#L109) | 独立机制 | K | 初筛 |
| [`game::tests::inventory::tool_rejects_an_unrelated_target_slot_without_changing_inventory`](../../../../crates/rfb-core/src/game/tests/inventory.rs#L156) | 独立机制 | K | 初筛 |
| [`game::tests::inventory::sling_bow_and_crossbow_resolve_their_compatible_ammunition_profiles`](../../../../crates/rfb-core/src/game/tests/inventory.rs#L176) | 独立机制 | K | 初筛 |
| [`game::tests::inventory::pickup_on_empty_ground_is_zero_time`](../../../../crates/rfb-core/src/game/tests/inventory.rs#L232) | 独立机制 | K | 初筛 |
| [`game::tests::inventory::stale_revision_is_rejected_without_mutation`](../../../../crates/rfb-core/src/game/tests/inventory.rs#L251) | 独立机制 | K | 初筛 |
| [`game::tests::inventory::inventory_item_missing_its_kind_is_an_invariant_error`](../../../../crates/rfb-core/src/game/tests/inventory.rs#L262) | 独立机制 | K | 初筛 |
| [`game::tests::inventory::elemental_brand_is_suppressed_only_by_matching_immunity`](../../../../crates/rfb-core/src/game/tests/inventory.rs#L284) | 独立机制 | K | 初筛 |
| [`game::tests::inventory::offensive_flag_dto_hides_unknown_affix_contributions`](../../../../crates/rfb-core/src/game/tests/inventory.rs#L345) | 独立机制 | K | 初筛 |
| [`game::tests::inventory::p88b_protection_quiver_skips_quivered_ammunition_without_rng`](../../../../crates/rfb-core/src/game/tests/inventory.rs#L427) | 独立机制 | K | 初筛 |
| [`game::tests::inventory::p88b_quiver_overflow_remains_vulnerable_and_emits_partial_destruction`](../../../../crates/rfb-core/src/game/tests/inventory.rs#L474) | 独立机制 | K | 初筛 |
| [`game::tests::inventory::p88b_protection_boundaries_preserve_other_destruction_rules`](../../../../crates/rfb-core/src/game/tests/inventory.rs#L536) | 独立机制 | K | 初筛 |
| [`game::tests::inventory::p88e_protection_quiver_does_not_cover_unequipped_fired_or_ground_ammunition`](../../../../crates/rfb-core/src/game/tests/inventory.rs#L614) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/items.rs`（52）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::items::p90b_olog_hai_affix_materializes_and_runs_existing_berserk_activation`](../../../../crates/rfb-core/src/game/tests/items.rs#L16) | 独立机制 | K | 初筛 |
| [`game::tests::items::p97e_multi_hued_dragon_breath_randomizes_five_elements_across_a_cone`](../../../../crates/rfb-core/src/game/tests/items.rs#L139) | 低收益抽样 | S04 | 正文 |
| [`game::tests::items::p99e_paurnimmen_cold_beam_hits_each_actor_before_the_wall`](../../../../crates/rfb-core/src/game/tests/items.rs#L221) | 独立机制 | K | 初筛 |
| [`game::tests::items::p100e_soulsword_rolls_and_persists_one_extra_power_and_increases_life`](../../../../crates/rfb-core/src/game/tests/items.rs#L297) | 独立机制 | K | 初筛 |
| [`game::tests::items::p100e_soulsword_warning_reveals_and_stops_before_a_hidden_trap`](../../../../crates/rfb-core/src/game/tests/items.rs#L378) | 独立机制 | K | 初筛 |
| [`game::tests::items::booze_applies_original_confusion_hallucination_and_blackout_ranges`](../../../../crates/rfb-core/src/game/tests/items.rs#L457) | 独立机制 | K | 初筛 |
| [`game::tests::items::booze_keeps_a_longer_existing_confusion_duration`](../../../../crates/rfb-core/src/game/tests/items.rs#L501) | 独立机制 | K | 正文 |
| [`game::tests::items::booze_refreshes_existing_statuses_without_identifying_itself`](../../../../crates/rfb-core/src/game/tests/items.rs#L539) | 独立机制 | K | 正文 |
| [`game::tests::items::restorative_item_sequence_recovers_resource_then_removes_status`](../../../../crates/rfb-core/src/game/tests/items.rs#L596) | 独立机制 | K | 初筛 |
| [`game::tests::items::full_resource_restoration_is_deterministic_and_round_trips`](../../../../crates/rfb-core/src/game/tests/items.rs#L644) | 独立机制 | K | 初筛 |
| [`game::tests::items::successful_restoration_reveals_later_no_effect_events`](../../../../crates/rfb-core/src/game/tests/items.rs#L697) | 独立机制 | K | 初筛 |
| [`game::tests::items::missing_player_resource_consumes_restorative_without_claiming_awareness`](../../../../crates/rfb-core/src/game/tests/items.rs#L727) | 独立机制 | K | 初筛 |
| [`game::tests::items::identify_scroll_rejects_missing_and_self_targets_before_consumption`](../../../../crates/rfb-core/src/game/tests/items.rs#L760) | 独立机制 | K | 初筛 |
| [`game::tests::items::enchantment_artifact_and_ammunition_pile_gates_follow_original_order`](../../../../crates/rfb-core/src/game/tests/items.rs#L789) | 独立机制 | K | 初筛 |
| [`game::tests::items::curse_scroll_lands_on_equipped_weapon_and_artifact_can_resist`](../../../../crates/rfb-core/src/game/tests/items.rs#L814) | 独立机制 | K | 初筛 |
| [`game::tests::items::curse_scroll_without_a_matching_equipped_item_consumes_without_rng_or_awareness`](../../../../crates/rfb-core/src/game/tests/items.rs#L894) | 独立机制 | K | 初筛 |
| [`game::tests::items::cleansing_scrolls_respect_heavy_and_permanent_curse_boundaries`](../../../../crates/rfb-core/src/game/tests/items.rs#L924) | 独立机制 | K | 初筛 |
| [`game::tests::items::spell_scroll_increases_only_eligible_learning_capacity_without_rng`](../../../../crates/rfb-core/src/game/tests/items.rs#L1063) | 独立机制 | K | 初筛 |
| [`game::tests::items::slowness_potion_refreshes_existing_slow_without_becoming_aware`](../../../../crates/rfb-core/src/game/tests/items.rs#L1143) | 参数变体 | P11 | 初筛 |
| [`game::tests::items::veil_draught_awareness_and_rng_follow_existing_blindness_and_immunity`](../../../../crates/rfb-core/src/game/tests/items.rs#L1198) | 参数变体 | P11 | 初筛 |
| [`game::tests::items::fury_draught_awareness_depends_on_new_berserk_or_actual_healing`](../../../../crates/rfb-core/src/game/tests/items.rs#L1273) | 参数变体 | P11 | 初筛 |
| [`game::tests::items::renewal_tonic_awareness_depends_on_either_restoration`](../../../../crates/rfb-core/src/game/tests/items.rs#L1352) | 参数变体 | P11 | 初筛 |
| [`game::tests::items::temperate_tonic_extends_existing_resistance_without_becoming_aware`](../../../../crates/rfb-core/src/game/tests/items.rs#L1426) | 参数变体 | P11 | 初筛 |
| [`game::tests::items::shatterburst_draught_uses_damage_scaling_and_existing_status_stacking`](../../../../crates/rfb-core/src/game/tests/items.rs#L1491) | 独立机制 | K | 初筛 |
| [`game::tests::items::mortal_draught_life_loss_bypasses_incoming_damage_reduction_without_rng`](../../../../crates/rfb-core/src/game/tests/items.rs#L1572) | 独立机制 | K | 初筛 |
| [`game::tests::items::friendly_item_summons_are_permanent_controlled_and_round_trip`](../../../../crates/rfb-core/src/game/tests/items.rs#L1609) | 独立机制 | K | 初筛 |
| [`game::tests::items::visible_actor_scrolls_consume_empty_results_without_rng_or_awareness`](../../../../crates/rfb-core/src/game/tests/items.rs#L1671) | 独立机制 | K | 初筛 |
| [`game::tests::items::mass_genocide_scroll_consumes_empty_result_with_awareness_and_zero_rng`](../../../../crates/rfb-core/src/game/tests/items.rs#L1708) | 独立机制 | K | 初筛 |
| [`game::tests::items::genocide_scroll_rejects_invalid_glyphs_and_consumes_an_empty_selection_without_rng`](../../../../crates/rfb-core/src/game/tests/items.rs#L1743) | 独立机制 | K | 初筛 |
| [`game::tests::items::adjacent_terrain_creation_consumes_empty_result_as_tried_without_rng`](../../../../crates/rfb-core/src/game/tests/items.rs#L1802) | 独立机制 | K | 初筛 |
| [`game::tests::items::p3_4_light_and_darkness_reuse_persisted_floor_glow`](../../../../crates/rfb-core/src/game/tests/items.rs#L1861) | 独立机制 | K | 初筛 |
| [`game::tests::items::p3_4_rune_requires_clean_floor_and_uses_original_break_threshold`](../../../../crates/rfb-core/src/game/tests/items.rs#L1918) | 独立机制 | K | 初筛 |
| [`game::tests::items::vengeance_retaliates_against_monster_spells_but_not_after_player_death`](../../../../crates/rfb-core/src/game/tests/items.rs#L2009) | 独立机制 | K | 初筛 |
| [`game::tests::items::travel_scroll_random_teleport_is_deterministic_and_rejects_without_space_atomically`](../../../../crates/rfb-core/src/game/tests/items.rs#L2118) | 独立机制 | K | 初筛 |
| [`game::tests::items::p3_2_refreshments_are_deliberate_no_numeric_effects`](../../../../crates/rfb-core/src/game/tests/items.rs#L2183) | 独立机制 | K | 初筛 |
| [`game::tests::items::p3_2_lose_memories_preserves_historical_experience`](../../../../crates/rfb-core/src/game/tests/items.rs#L2210) | 独立机制 | K | 初筛 |
| [`game::tests::items::p3_2_invulnerability_and_giant_strength_reuse_status_payloads`](../../../../crates/rfb-core/src/game/tests/items.rs#L2241) | 独立机制 | K | 初筛 |
| [`game::tests::items::p3_7_experience_potion_uses_unscaled_relative_gain_and_level_cap`](../../../../crates/rfb-core/src/game/tests/items.rs#L2299) | 独立机制 | K | 初筛 |
| [`game::tests::items::p3_7_neo_tsuyoshi_round_trips_and_crashes_on_expiry`](../../../../crates/rfb-core/src/game/tests/items.rs#L2347) | 独立机制 | K | 初筛 |
| [`game::tests::items::p3_7_tsuyoshi_special_triggers_the_same_permanent_crash_immediately`](../../../../crates/rfb-core/src/game/tests/items.rs#L2427) | 独立机制 | K | 初筛 |
| [`game::tests::items::p3_3_treasure_detection_reports_stable_gold_pile_ids`](../../../../crates/rfb-core/src/game/tests/items.rs#L2457) | 独立机制 | K | 初筛 |
| [`game::tests::items::p3_5_acquirement_uses_stable_ids_current_position_and_exact_rng_draws`](../../../../crates/rfb-core/src/game/tests/items.rs#L2520) | 独立机制 | K | 初筛 |
| [`game::tests::items::p3_5_mundanity_splits_one_unit_and_rejects_fixed_artifacts_atomically`](../../../../crates/rfb-core/src/game/tests/items.rs#L2583) | 独立机制 | K | 初筛 |
| [`game::tests::items::p3_5_crafting_splits_ammunition_identifies_ego_and_cancels_invalid_targets`](../../../../crates/rfb-core/src/game/tests/items.rs#L2666) | 独立机制 | K | 初筛 |
| [`game::tests::items::p3_5_crafting_split_allocation_failure_is_atomic`](../../../../crates/rfb-core/src/game/tests/items.rs#L2750) | 独立机制 | K | 初筛 |
| [`game::tests::items::p3_5_crafting_materializes_player_made_armor`](../../../../crates/rfb-core/src/game/tests/items.rs#L2789) | 独立机制 | K | 初筛 |
| [`game::tests::items::p3_5_rumour_is_localized_without_core_rng`](../../../../crates/rfb-core/src/game/tests/items.rs#L2826) | 独立机制 | K | 初筛 |
| [`game::tests::items::fixed_artifact_selection_uses_source_order_ood_rarity_and_uniqueness`](../../../../crates/rfb-core/src/game/tests/items.rs#L2851) | 独立机制 | K | 初筛 |
| [`game::tests::items::item_generation_modes_keep_drafts_unallocated_until_commit`](../../../../crates/rfb-core/src/game/tests/items.rs#L2928) | 独立机制 | K | 初筛 |
| [`game::tests::items::p103d_mana_storm_staff_hits_radius_five_without_backlash`](../../../../crates/rfb-core/src/game/tests/items.rs#L2992) | 独立机制 | K | 初筛 |
| [`game::tests::items::p107e_frost_ball_and_confusing_light_reuse_area_and_status_resolvers`](../../../../crates/rfb-core/src/game/tests/items.rs#L3076) | 独立机制 | K | 初筛 |
| [`game::tests::items::p107f_diamond_edge_vorpal_flag_multiplies_regular_melee_blows`](../../../../crates/rfb-core/src/game/tests/items.rs#L3189) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/lighting.rs`（13）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::lighting::warrior_birth_rolls_three_to_seven_matching_torches_after_food`](../../../../crates/rfb-core/src/game/tests/lighting.rs#L68) | 独立机制 | K | 初筛 |
| [`game::tests::lighting::fuel_items_start_with_original_capacity_weight_and_radius`](../../../../crates/rfb-core/src/game/tests/lighting.rs#L114) | 独立机制 | K | 初筛 |
| [`game::tests::lighting::torch_refuel_consumes_a_torch_adds_source_fuel_plus_five_and_costs_fifty_energy`](../../../../crates/rfb-core/src/game/tests/lighting.rs#L153) | 独立机制 | K | 初筛 |
| [`game::tests::lighting::lantern_accepts_oil_or_another_lantern_and_caps_after_consuming_the_source`](../../../../crates/rfb-core/src/game/tests/lighting.rs#L195) | 独立机制 | K | 初筛 |
| [`game::tests::lighting::unavailable_refuel_is_zero_world_time_rng_and_item_mutation`](../../../../crates/rfb-core/src/game/tests/lighting.rs#L226) | 独立机制 | K | 初筛 |
| [`game::tests::lighting::equipped_light_spends_one_fuel_per_ten_ticks_and_reports_extinction`](../../../../crates/rfb-core/src/game/tests/lighting.rs#L259) | 独立机制 | K | 初筛 |
| [`game::tests::lighting::surface_is_ambient_lit_and_dungeon_visibility_follows_equipped_light_radius`](../../../../crates/rfb-core/src/game/tests/lighting.rs#L287) | 独立机制 | K | 初筛 |
| [`game::tests::lighting::infravision_does_not_reveal_cold_blooded_monsters`](../../../../crates/rfb-core/src/game/tests/lighting.rs#L370) | 独立机制 | K | 初筛 |
| [`game::tests::lighting::invisible_actors_are_hidden_until_detected_and_detection_round_trips`](../../../../crates/rfb-core/src/game/tests/lighting.rs#L407) | 独立机制 | K | 初筛 |
| [`game::tests::lighting::intrinsic_race_see_invisible_stacks_and_follows_the_current_form`](../../../../crates/rfb-core/src/game/tests/lighting.rs#L459) | 独立机制 | K | 初筛 |
| [`game::tests::lighting::intrinsic_race_see_invisible_uses_the_original_detection_roll`](../../../../crates/rfb-core/src/game/tests/lighting.rs#L496) | 独立机制 | K | 初筛 |
| [`game::tests::lighting::room_glow_darkening_persists_in_stored_floor_save_and_state_hash`](../../../../crates/rfb-core/src/game/tests/lighting.rs#L543) | 独立机制 | K | 初筛 |
| [`game::tests::lighting::warrens_light_attempts_are_seeded_walkable_weighted_and_persistent`](../../../../crates/rfb-core/src/game/tests/lighting.rs#L567) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/mining_progress.rs`（7）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::mining_progress::only_player_digging_that_removes_a_vein_trains_mining`](../../../../crates/rfb-core/src/game/tests/mining_progress.rs#L5) | 独立机制 | K | 初筛 |
| [`game::tests::mining_progress::mining_and_sparse_materials_project_and_round_trip_strictly`](../../../../crates/rfb-core/src/game/tests/mining_progress.rs#L56) | 独立机制 | K | 初筛 |
| [`game::tests::mining_progress::mining_and_material_save_fields_are_required`](../../../../crates/rfb-core/src/game/tests/mining_progress.rs#L157) | 参数变体 | P10 | 初筛 |
| [`game::tests::mining_progress::hidden_treasure_veins_use_their_real_yield_for_digging_rewards`](../../../../crates/rfb-core/src/game/tests/mining_progress.rs#L170) | 独立机制 | K | 初筛 |
| [`game::tests::mining_progress::magic_destruction_of_treasure_veins_only_places_ordinary_gold`](../../../../crates/rfb-core/src/game/tests/mining_progress.rs#L217) | 独立机制 | K | 初筛 |
| [`game::tests::mining_progress::every_visible_and_hidden_treasure_vein_uses_the_mining_item_path`](../../../../crates/rfb-core/src/game/tests/mining_progress.rs#L256) | 独立机制 | K | 初筛 |
| [`game::tests::mining_progress::rubble_item_origin_round_trips_on_any_generated_item_kind`](../../../../crates/rfb-core/src/game/tests/mining_progress.rs#L317) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/monster_ai.rs`（3）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::monster_ai::monster_level_teleport_uses_resistance_save_and_floor_transition`](../../../../crates/rfb-core/src/game/tests/monster_ai.rs#L5) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ai::monster_shriek_excludes_the_caster_and_aggravates_other_monsters`](../../../../crates/rfb-core/src/game/tests/monster_ai.rs#L114) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ai::monster_world_grants_three_or_four_actions_without_recursive_casts`](../../../../crates/rfb-core/src/game/tests/monster_ai.rs#L194) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/monster_doors.rs`（2）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::monster_doors::opening_an_ordinary_door_spends_the_action_without_moving`](../../../../crates/rfb-core/src/game/tests/monster_doors.rs#L19) | 独立机制 | K | 初筛 |
| [`game::tests::monster_doors::a_successful_bash_moves_the_monster_into_the_doorway`](../../../../crates/rfb-core/src/game/tests/monster_doors.rs#L39) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/monster_ecology.rs`（36）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::monster_ecology::surface_habitat_requires_the_declared_terrain_unless_wild_all`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L31) | 参数变体 | P13 | 初筛 |
| [`game::tests::monster_ecology::compost_monsters_allocate_only_in_the_sewer_task`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L95) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::shapechanger_projects_another_monster_and_rerolls_each_action`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L116) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::tanuki_keeps_true_runtime_stats_behind_one_persistent_disguise`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L149) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::fear_aura_uses_apparent_level_and_only_lands_once_per_tick_at_range`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L194) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::chameleon_keeps_its_identity_while_its_form_drives_runtime_behavior`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L223) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::chameleon_change_check_uses_one_in_thirteen_before_selecting_a_form`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L294) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::eldritch_horror_triggers_on_fresh_sight_and_persists_its_repeat_gate`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L321) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::eldritch_horror_reuses_attribute_amnesia_and_weird_mind_contracts`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L364) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::p86e_camelot_admits_only_its_dungeon_two_roster`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L392) | 参数变体 | P12 | 初筛 |
| [`game::tests::monster_ecology::depth_nine_two_stage_out_of_depth_roll_reaches_level_fourteen`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L437) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::non_preferred_glyph_uses_original_monster_div_sixteen_weight`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L446) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::preferred_glyph_or_tag_uses_full_original_weight_without_rng`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L470) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::p87b_movement_mode_or_habitat_preference_uses_full_original_weight`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L497) | 参数变体 | P12 | 初筛 |
| [`game::tests::monster_ecology::p87b_dungeon_definition_excludes_a_tagless_guardian_from_allocation`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L532) | 参数变体 | P12 | 初筛 |
| [`game::tests::monster_ecology::p88c_icky_cave_glyphs_are_or_preferences_and_queen_is_a_guardian`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L544) | 参数变体 | P12 | 初筛 |
| [`game::tests::monster_ecology::dungeon_allocation_preserves_ecology_location_locks_and_guardian_exclusions`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L568) | 参数变体 | P12 | 初筛 |
| [`game::tests::monster_ecology::warg_friend_count_uses_three_d_three_including_the_leader`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L776) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::mughash_escort_uses_lower_level_kobolds`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L792) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::giant_white_mouse_reproduction_adds_one_adjacent_mouse`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L827) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::sterility_suppresses_reproduction_without_spending_rng`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L854) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::same_kind_reproduction_stops_at_one_hundred_living_monsters`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L872) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::original_pack_members_share_one_selected_behavior`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L900) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::fixed_guardian_without_allocation_generates_on_global_allocation_floor`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L937) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::warg_random_movement_replaces_normal_tracking`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L965) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::ambient_allocation_adds_a_distant_warrens_monster`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L990) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::defeated_unique_state_round_trips_after_normal_unique_death`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L1011) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::unique2_allows_one_living_instance_but_returns_after_death`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L1052) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::fixed_unique_summon_plans_only_one_available_instance`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L1092) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::save_rejects_duplicate_living_normal_unique_instances`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L1132) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::nazgul_lifetime_limit_counts_current_and_stored_floors_and_round_trips`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L1159) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::nazgul_deaths_permanently_consume_the_five_instance_limit`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L1244) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::fixed_and_category_summons_share_the_nazgul_lifetime_quota`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L1290) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::nazgul_is_immune_to_monster_target_polymorph`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L1355) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::p71_one_split_death_closes_the_shared_lifetime_and_round_trips`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L1374) | 独立机制 | K | 初筛 |
| [`game::tests::monster_ecology::defeated_limited_actor_counts_are_required_in_new_saves`](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs#L1445) | 参数变体 | P10 | 初筛 |

## `crates/rfb-core/src/game/tests/monster_hit_points.rs`（3）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::monster_hit_points::normal_monster_hit_points_roll_each_die_once_at_birth`](../../../../crates/rfb-core/src/game/tests/monster_hit_points.rs#L6) | 独立机制 | K | 初筛 |
| [`game::tests::monster_hit_points::force_maximum_hit_points_use_the_full_product_without_rng`](../../../../crates/rfb-core/src/game/tests/monster_hit_points.rs#L21) | 独立机制 | K | 初筛 |
| [`game::tests::monster_hit_points::rolled_instance_hit_points_remain_authoritative_after_load`](../../../../crates/rfb-core/src/game/tests/monster_hit_points.rs#L34) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/monster_movement.rs`（13）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::monster_movement::movement_domains_allow_only_their_supported_terrain`](../../../../crates/rfb-core/src/game/tests/monster_movement.rs#L7) | 参数变体 | P13 | 初筛 |
| [`game::tests::monster_movement::kill_body_attacks_a_weaker_actor_blocking_the_next_step`](../../../../crates/rfb-core/src/game/tests/monster_movement.rs#L56) | 独立机制 | K | 初筛 |
| [`game::tests::monster_movement::move_body_swaps_with_a_weaker_actor_and_wakes_it`](../../../../crates/rfb-core/src/game/tests/monster_movement.rs#L89) | 独立机制 | K | 初筛 |
| [`game::tests::monster_movement::move_body_requires_the_displaced_actor_to_cross_the_origin_terrain`](../../../../crates/rfb-core/src/game/tests/monster_movement.rs#L141) | 独立机制 | K | 初筛 |
| [`game::tests::monster_movement::move_body_requires_strictly_greater_experience_value`](../../../../crates/rfb-core/src/game/tests/monster_movement.rs#L164) | 独立机制 | K | 初筛 |
| [`game::tests::monster_movement::monster_regeneration_is_shared_doubled_and_capped`](../../../../crates/rfb-core/src/game/tests/monster_movement.rs#L182) | 独立机制 | K | 初筛 |
| [`game::tests::monster_movement::low_hp_monster_regeneration_uses_one_minimum_recovery_draw`](../../../../crates/rfb-core/src/game/tests/monster_movement.rs#L228) | 独立机制 | K | 初筛 |
| [`game::tests::monster_movement::ranged_melee_uses_the_melee_routine_at_rfb_two_grid_reach`](../../../../crates/rfb-core/src/game/tests/monster_movement.rs#L250) | 独立机制 | K | 初筛 |
| [`game::tests::monster_movement::living_trump_blinks_for_free_before_its_action`](../../../../crates/rfb-core/src/game/tests/monster_movement.rs#L285) | 独立机制 | K | 初筛 |
| [`game::tests::monster_movement::quantum_turn_uses_the_stable_entity_id_and_can_naturally_vanish`](../../../../crates/rfb-core/src/game/tests/monster_movement.rs#L321) | 独立机制 | K | 初筛 |
| [`game::tests::monster_movement::clear_head_has_a_one_in_four_chance_to_remove_monster_confusion`](../../../../crates/rfb-core/src/game/tests/monster_movement.rs#L365) | 独立机制 | K | 初筛 |
| [`game::tests::monster_movement::never_move_monster_waits_at_range_and_can_attack_adjacent`](../../../../crates/rfb-core/src/game/tests/monster_movement.rs#L410) | 独立机制 | K | 初筛 |
| [`game::tests::monster_movement::never_move_monster_can_still_blink`](../../../../crates/rfb-core/src/game/tests/monster_movement.rs#L454) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/mutations.rs`（44）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::mutations::demigod_passives_apply_level_hp_spell_power_and_attribute_costs`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L130) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::distant_damage_builds_original_monster_anger_and_talents_suppress_each_source`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L159) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::monster_anger_increases_cast_frequency_resets_on_cast_and_round_trips`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L184) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::evasion_reduces_innate_damage_without_affecting_other_attacks`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L210) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::evasion_avoids_half_of_earthquake_crushes_after_the_grid_is_selected`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L232) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::cult_of_personality_can_turn_a_hostile_summon_into_a_pet`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L289) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::peerless_tracker_and_fantastic_frenzy_use_shared_ability_transactions`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L351) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::fantastic_frenzy_preserves_unused_normal_melee_energy_after_a_kill`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L426) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::demigod_passives_scale_player_healing_and_potion_energy`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L511) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::weapon_skills_raises_every_available_weapon_cap_to_master`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L536) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::infernal_deal_recovers_hp_or_hp_and_casting_resource_on_visible_death`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L557) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::human_strength_stops_later_criticals_and_adds_one_fifth_action_energy`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L595) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::human_dexterity_sprain_applies_one_speed_penalty_and_still_rolls_while_slow`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L612) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::human_constitution_only_rolls_for_unwell_when_the_status_is_absent`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L648) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::human_intelligence_only_reduces_fear_checks`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L682) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::human_charisma_applies_skill_spell_and_forced_hit_penalties`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L712) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::chaos_gift_assigns_and_persists_one_authoritative_patron`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L761) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::chaos_gift_rewards_only_a_new_highest_level`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L782) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::chaos_weapon_table_preserves_original_level_bands`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L835) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::m4f_c_luck_bias_adjusts_quality_depth_and_attribute_thresholds`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L862) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::rfb_depth_quality_uses_original_thresholds_and_one_draw`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L914) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::m4f_c_easy_tiring_accumulates_and_recovers_shared_minor_slow`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L993) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::m4f_c_impotence_penalizes_staffs_and_rods_but_not_wands`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L1019) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::periodic_mutations_use_source_order_and_exact_trigger_draws`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L1078) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::periodic_mutations_skip_world_map_without_rng_and_consume_one_draw_on_miss`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L1106) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::m6_a_berserk_and_invulnerability_reuse_authoritative_status_payloads`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L1151) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::m6_a_speed_flux_minor_slow_round_trips_and_feeds_speed`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L1188) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::m6_a_resource_conversion_and_hypochondria_use_existing_stat_resources`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L1205) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::m6_a_produce_mana_persists_its_prompt_then_resolves_a_directional_ball`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L1247) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::m6_b_random_teleport_and_banish_reuse_existing_displacement_rules`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L1286) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::m6_b_fumbling_deals_one_d_twenty_five_and_drops_a_removable_weapon`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L1323) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::m6_b_shadow_walk_persists_then_only_regenerates_ordinary_procedural_dungeons`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L1362) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::m6_c_flatulence_and_raw_chaos_reuse_centered_area_damage`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L1441) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::m6_c_attractions_reuse_category_summons_and_original_friendliness`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L1472) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::m6_c_eat_light_heals_halves_fuel_damages_and_extinguishes_the_area`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L1534) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::m6_d_normality_respects_locks_and_wasting_respects_sustains`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L1590) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::m6_d_wraithform_and_polymorph_wounds_reuse_shared_status_transactions`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L1661) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::m6_d_random_telepathy_toggles_and_nausea_sets_the_original_weak_threshold`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L1727) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::m6_d_warning_sums_every_living_monster_at_or_above_player_level`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L1762) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::m7_polymorph_rare_cure_preserves_locks_and_loses_in_source_order`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L1801) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::m7_polymorph_empty_candidate_set_terminates_without_rng`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L1845) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::m7_polymorph_chains_gains_and_orders_conflict_before_gain`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L1864) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::m7_polymorph_loss_threshold_changes_after_five_unlocked_mutations`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L1923) | 独立机制 | K | 初筛 |
| [`game::tests::mutations::m7_polymorph_potion_consumes_and_becomes_aware_after_a_change`](../../../../crates/rfb-core/src/game/tests/mutations.rs#L1974) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/paladin.rs`（3）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::paladin::death_paladin_birth_uses_the_original_class_identity_skills_and_kit`](../../../../crates/rfb-core/src/game/tests/paladin.rs#L11) | 参数变体 | P02 | 初筛 |
| [`game::tests::paladin::death_paladin_projects_divine_study_mana_and_the_original_spell_table`](../../../../crates/rfb-core/src/game/tests/paladin.rs#L79) | 参数变体 | P01 | 初筛 |
| [`game::tests::paladin::death_paladin_unlocks_hell_lance_and_fear_resistance_at_original_levels`](../../../../crates/rfb-core/src/game/tests/paladin.rs#L155) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/persistence.rs`（8）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::persistence::exploration_memory_does_not_change_authoritative_state_hash`](../../../../crates/rfb-core/src/game/tests/persistence.rs#L5) | 独立机制 | K | 初筛 |
| [`game::tests::persistence::content_hash_is_reported_but_not_part_of_authoritative_state_hash`](../../../../crates/rfb-core/src/game/tests/persistence.rs#L24) | 独立机制 | K | 初筛 |
| [`game::tests::persistence::malformed_exploration_memory_is_rejected`](../../../../crates/rfb-core/src/game/tests/persistence.rs#L47) | 独立机制 | K | 初筛 |
| [`game::tests::persistence::malformed_revealed_terrain_knowledge_is_rejected`](../../../../crates/rfb-core/src/game/tests/persistence.rs#L59) | 独立机制 | K | 初筛 |
| [`game::tests::persistence::save_with_different_content_hash_is_rejected`](../../../../crates/rfb-core/src/game/tests/persistence.rs#L71) | 独立机制 | K | 初筛 |
| [`game::tests::persistence::generated_artifact_state_round_trips_and_changes_the_state_hash`](../../../../crates/rfb-core/src/game/tests/persistence.rs#L82) | 独立机制 | K | 初筛 |
| [`game::tests::persistence::malformed_generated_artifact_state_is_rejected`](../../../../crates/rfb-core/src/game/tests/persistence.rs#L102) | 独立机制 | K | 初筛 |
| [`game::tests::persistence::fixed_artifact_instance_requires_its_generation_record`](../../../../crates/rfb-core/src/game/tests/persistence.rs#L121) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/pet_upkeep.rs`（6）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::pet_upkeep::upkeep_uses_the_class_divisor_unique_cost_and_strict_control`](../../../../crates/rfb-core/src/game/tests/pet_upkeep.rs#L34) | 独立机制 | K | 初筛 |
| [`game::tests::pet_upkeep::upkeep_scales_positive_mana_recovery_and_drains_above_one_hundred_percent`](../../../../crates/rfb-core/src/game/tests/pet_upkeep.rs#L73) | 独立机制 | K | 初筛 |
| [`game::tests::pet_upkeep::dismiss_pets_is_zero_time_and_removes_every_controlled_actor`](../../../../crates/rfb-core/src/game/tests/pet_upkeep.rs#L115) | 独立机制 | K | 初筛 |
| [`game::tests::pet_upkeep::advancing_actions_drain_excess_upkeep_and_zero_mana_interrupts_rest`](../../../../crates/rfb-core/src/game/tests/pet_upkeep.rs#L132) | 独立机制 | K | 初筛 |
| [`game::tests::pet_upkeep::exactly_one_hundred_percent_upkeep_is_not_a_recoverable_rest_need`](../../../../crates/rfb-core/src/game/tests/pet_upkeep.rs#L163) | 独立机制 | K | 初筛 |
| [`game::tests::pet_upkeep::neglected_pet_checks_preserve_the_original_rng_gate_order`](../../../../crates/rfb-core/src/game/tests/pet_upkeep.rs#L180) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/prayer_study.rs`（3）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::prayer_study::chosen_and_divine_study_modes_keep_distinct_commands`](../../../../crates/rfb-core/src/game/tests/prayer_study.rs#L17) | 独立机制 | K | 初筛 |
| [`game::tests::prayer_study::divine_study_is_deterministic_and_accepts_a_book_at_the_players_feet`](../../../../crates/rfb-core/src/game/tests/prayer_study.rs#L81) | 独立机制 | K | 初筛 |
| [`game::tests::prayer_study::blindness_darkness_and_confusion_block_study_before_rng`](../../../../crates/rfb-core/src/game/tests/prayer_study.rs#L114) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/progression.rs`（43）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::progression::birth_race_passives_mutation_overrides_and_class_exclusions_are_resolved`](../../../../crates/rfb-core/src/game/tests/progression.rs#L133) | 独立机制 | K | 初筛 |
| [`game::tests::progression::race_level_stat_scaling_preserves_klackon_and_enables_formal_golem_intrinsics`](../../../../crates/rfb-core/src/game/tests/progression.rs#L230) | 独立机制 | K | 初筛 |
| [`game::tests::progression::formal_golem_creation_and_temporary_form_apply_and_remove_intrinsics_and_stone_skin`](../../../../crates/rfb-core/src/game/tests/progression.rs#L290) | 独立机制 | K | 初筛 |
| [`game::tests::progression::formal_zombie_creation_and_temporary_form_apply_and_remove_intrinsics`](../../../../crates/rfb-core/src/game/tests/progression.rs#L367) | 参数变体 | P03 | 初筛 |
| [`game::tests::progression::formal_skeleton_creation_and_temporary_form_apply_and_remove_intrinsics`](../../../../crates/rfb-core/src/game/tests/progression.rs#L462) | 参数变体 | P03 | 初筛 |
| [`game::tests::progression::formal_wood_elf_and_temporary_form_cross_trees_without_delay`](../../../../crates/rfb-core/src/game/tests/progression.rs#L549) | 独立机制 | K | 初筛 |
| [`game::tests::progression::formal_archon_and_temporary_form_apply_and_remove_static_passives`](../../../../crates/rfb-core/src/game/tests/progression.rs#L633) | 参数变体 | P03 | 初筛 |
| [`game::tests::progression::formal_sprite_and_temporary_form_apply_level_speed_and_static_passives`](../../../../crates/rfb-core/src/game/tests/progression.rs#L673) | 参数变体 | P03 | 初筛 |
| [`game::tests::progression::formal_shadow_fairy_and_temporary_form_apply_and_remove_intrinsics`](../../../../crates/rfb-core/src/game/tests/progression.rs#L765) | 参数变体 | P03 | 初筛 |
| [`game::tests::progression::draconian_subraces_are_available_to_formal_character_creation`](../../../../crates/rfb-core/src/game/tests/progression.rs#L824) | 独立机制 | K | 初筛 |
| [`game::tests::progression::draconian_level_35_reward_revalidates_all_nine_completed_powers`](../../../../crates/rfb-core/src/game/tests/progression.rs#L847) | 独立机制 | K | 初筛 |
| [`game::tests::progression::draconian_metamorphosis_replaces_body_and_derives_combat_save_and_hash_state`](../../../../crates/rfb-core/src/game/tests/progression.rs#L1063) | 独立机制 | K | 初筛 |
| [`game::tests::progression::draconian_metamorphosis_uses_class_multipliers_and_original_exclusions`](../../../../crates/rfb-core/src/game/tests/progression.rs#L1175) | 独立机制 | K | 初筛 |
| [`game::tests::progression::race_level_mutation_rewards_are_derived_locked_and_zero_time`](../../../../crates/rfb-core/src/game/tests/progression.rs#L1220) | 独立机制 | K | 初筛 |
| [`game::tests::progression::casting_attribute_race_reward_uses_the_class_profile`](../../../../crates/rfb-core/src/game/tests/progression.rs#L1341) | 独立机制 | K | 初筛 |
| [`game::tests::progression::formal_human_weakness_uses_each_current_build_casting_attribute_once`](../../../../crates/rfb-core/src/game/tests/progression.rs#L1359) | 独立机制 | K | 初筛 |
| [`game::tests::progression::attribute_potentials_project_save_hash_and_reject_invalid_values`](../../../../crates/rfb-core/src/game/tests/progression.rs#L1400) | 独立机制 | K | 初筛 |
| [`game::tests::progression::mutation_state_projects_saves_hashes_and_rejects_invalid_references`](../../../../crates/rfb-core/src/game/tests/progression.rs#L1439) | 独立机制 | K | 初筛 |
| [`game::tests::progression::mutation_transactions_preserve_locks_remove_conflicts_and_emit_source_order`](../../../../crates/rfb-core/src/game/tests/progression.rs#L1518) | 独立机制 | K | 初筛 |
| [`game::tests::progression::passive_mutations_feed_existing_attribute_speed_armor_and_hp_pipelines`](../../../../crates/rfb-core/src/game/tests/progression.rs#L1599) | 独立机制 | K | 初筛 |
| [`game::tests::progression::m4b_passives_feed_resistance_sense_skill_and_flight_pipelines`](../../../../crates/rfb-core/src/game/tests/progression.rs#L1709) | 独立机制 | K | 初筛 |
| [`game::tests::progression::esp_respects_mind_flags_and_conceals_nonvisual_identity`](../../../../crates/rfb-core/src/game/tests/progression.rs#L1805) | 独立机制 | K | 初筛 |
| [`game::tests::progression::m4c_regeneration_and_fire_light_feed_existing_player_pipelines`](../../../../crates/rfb-core/src/game/tests/progression.rs#L1880) | 独立机制 | K | 初筛 |
| [`game::tests::progression::m4d_passive_combat_modifiers_feed_existing_attribute_and_skill_pipelines`](../../../../crates/rfb-core/src/game/tests/progression.rs#L1935) | 独立机制 | K | 初筛 |
| [`game::tests::progression::m4e_cross_system_mutations_reuse_stats_energy_experience_and_item_knowledge`](../../../../crates/rfb-core/src/game/tests/progression.rs#L1970) | 独立机制 | K | 初筛 |
| [`game::tests::progression::new_life_is_one_seeded_transaction_with_locked_mutation_protection`](../../../../crates/rfb-core/src/game/tests/progression.rs#L2055) | 独立机制 | K | 初筛 |
| [`game::tests::progression::random_mutation_transactions_are_weighted_and_empty_candidates_use_no_rng`](../../../../crates/rfb-core/src/game/tests/progression.rs#L2179) | 独立机制 | K | 初筛 |
| [`game::tests::progression::locked_mutations_do_not_reduce_regeneration`](../../../../crates/rfb-core/src/game/tests/progression.rs#L2208) | 独立机制 | K | 初筛 |
| [`game::tests::progression::unlocked_mutation_count_scales_natural_regeneration`](../../../../crates/rfb-core/src/game/tests/progression.rs#L2237) | 独立机制 | K | 初筛 |
| [`game::tests::progression::build_skill_growth_experience_multiplier_and_save_identity_are_deterministic`](../../../../crates/rfb-core/src/game/tests/progression.rs#L2269) | 独立机制 | K | 初筛 |
| [`game::tests::progression::formal_race_selection_changes_the_warrior_profile_and_defaults_to_human`](../../../../crates/rfb-core/src/game/tests/progression.rs#L2293) | 独立机制 | K | 初筛 |
| [`game::tests::progression::high_elf_intrinsics_and_identity_round_trip`](../../../../crates/rfb-core/src/game/tests/progression.rs#L2510) | 参数变体 | P03 | 初筛 |
| [`game::tests::progression::dunadan_sustain_talent_and_identity_are_authoritative`](../../../../crates/rfb-core/src/game/tests/progression.rs#L2571) | 参数变体 | P03 | 初筛 |
| [`game::tests::progression::half_orc_infravision_and_level_thirty_talent_are_authoritative`](../../../../crates/rfb-core/src/game/tests/progression.rs#L2653) | 参数变体 | P03 | 初筛 |
| [`game::tests::progression::barbarian_fear_power_and_level_thirty_talent_are_authoritative`](../../../../crates/rfb-core/src/game/tests/progression.rs#L2726) | 独立机制 | K | 初筛 |
| [`game::tests::progression::formal_einheri_chooses_the_shared_demigod_talent_at_level_thirty`](../../../../crates/rfb-core/src/game/tests/progression.rs#L2849) | 参数变体 | P03 | 初筛 |
| [`game::tests::progression::selected_formal_race_overrides_the_build_default_and_round_trips`](../../../../crates/rfb-core/src/game/tests/progression.rs#L2896) | 独立机制 | K | 初筛 |
| [`game::tests::progression::attribute_increase_command_commits_growth_without_rng_or_world_progression`](../../../../crates/rfb-core/src/game/tests/progression.rs#L2918) | 独立机制 | K | 初筛 |
| [`game::tests::progression::unavailable_attribute_increase_rejects_without_mutation_or_rng`](../../../../crates/rfb-core/src/game/tests/progression.rs#L2974) | 独立机制 | K | 初筛 |
| [`game::tests::progression::restore_life_uses_historical_experience`](../../../../crates/rfb-core/src/game/tests/progression.rs#L3006) | 独立机制 | K | 初筛 |
| [`game::tests::progression::attribute_and_experience_history_round_trip_and_reject_invalid_values`](../../../../crates/rfb-core/src/game/tests/progression.rs#L3041) | 独立机制 | K | 初筛 |
| [`game::tests::progression::attribute_resource_refresh_scales_the_prechange_current_value_once`](../../../../crates/rfb-core/src/game/tests/progression.rs#L3087) | 独立机制 | K | 初筛 |
| [`game::tests::progression::formal_beastman_birth_level_mutations_and_regeneration_match_rfb`](../../../../crates/rfb-core/src/game/tests/progression.rs#L3121) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/race_attribute_sustains.rs`（2）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::race_attribute_sustains::race_and_equipment_attribute_sustains_use_the_effective_race`](../../../../crates/rfb-core/src/game/tests/race_attribute_sustains.rs#L40) | 独立机制 | K | 初筛 |
| [`game::tests::race_attribute_sustains::race_sustain_guards_every_attribute_drain_entry_point_without_extra_rng`](../../../../crates/rfb-core/src/game/tests/race_attribute_sustains.rs#L67) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/riding.rs`（11）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::riding::riding_proficiency_uses_original_melee_archery_and_fall_training_rules`](../../../../crates/rfb-core/src/game/tests/riding.rs#L26) | 独立机制 | K | 初筛 |
| [`game::tests::riding::riding_proficiency_is_authoritative_save_and_snapshot_state`](../../../../crates/rfb-core/src/game/tests/riding.rs#L60) | 独立机制 | K | 初筛 |
| [`game::tests::riding::riding_proficiency_save_field_is_required`](../../../../crates/rfb-core/src/game/tests/riding.rs#L92) | 参数变体 | P10 | 初筛 |
| [`game::tests::riding::mount_moves_with_player_round_trips_and_dismounts`](../../../../crates/rfb-core/src/game/tests/riding.rs#L102) | 独立机制 | K | 初筛 |
| [`game::tests::riding::sheep_preserves_the_authoritative_refusal`](../../../../crates/rfb-core/src/game/tests/riding.rs#L159) | 独立机制 | K | 初筛 |
| [`game::tests::riding::ordinary_riding_rejects_wild_monsters_without_taming_or_rng`](../../../../crates/rfb-core/src/game/tests/riding.rs#L181) | 独立机制 | K | 初筛 |
| [`game::tests::riding::mounted_speed_uses_original_riding_control_formula`](../../../../crates/rfb-core/src/game/tests/riding.rs#L205) | 独立机制 | K | 初筛 |
| [`game::tests::riding::mounted_weapon_and_projectile_rules_match_original_branches`](../../../../crates/rfb-core/src/game/tests/riding.rs#L227) | 独立机制 | K | 初筛 |
| [`game::tests::riding::forced_fall_moves_to_an_adjacent_cell_and_collision_stays_mounted`](../../../../crates/rfb-core/src/game/tests/riding.rs#L325) | 独立机制 | K | 初筛 |
| [`game::tests::riding::damage_fall_trains_riding_and_mount_death_uses_existing_cleanup`](../../../../crates/rfb-core/src/game/tests/riding.rs#L366) | 独立机制 | K | 初筛 |
| [`game::tests::riding::current_mount_follows_a_floor_transition`](../../../../crates/rfb-core/src/game/tests/riding.rs#L390) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/riding_bond.rs`（4）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::riding_bond::pet_experience_evolves_in_place_and_resets_the_bond`](../../../../crates/rfb-core/src/game/tests/riding_bond.rs#L25) | 独立机制 | K | 初筛 |
| [`game::tests::riding_bond::a_kill_that_completes_the_bond_uses_full_experience_sharing`](../../../../crates/rfb-core/src/game/tests/riding_bond.rs#L64) | 独立机制 | K | 初筛 |
| [`game::tests::riding_bond::riding_bond_potions_use_the_current_mount_and_keep_haste_rng_exact`](../../../../crates/rfb-core/src/game/tests/riding_bond.rs#L99) | 独立机制 | K | 初筛 |
| [`game::tests::riding_bond::mount_enabled_potions_remain_usable_by_the_player_without_a_target`](../../../../crates/rfb-core/src/game/tests/riding_bond.rs#L182) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/snapshots.rs`（1）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::snapshots::ground_item_projection_requires_sight_or_detection_and_round_trips`](../../../../crates/rfb-core/src/game/tests/snapshots.rs#L5) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/sniper.rs`（17）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::sniper::sniper_birth_uses_original_identity_skills_proficiencies_kit_and_techniques`](../../../../crates/rfb-core/src/game/tests/sniper.rs#L204) | 参数变体 | P02 | 初筛 |
| [`game::tests::sniper::sniper_profile_uses_original_level_boundaries_bolt_hit_and_half_excess_speed`](../../../../crates/rfb-core/src/game/tests/sniper.rs#L486) | 独立机制 | K | 初筛 |
| [`game::tests::sniper::concentrate_reaches_and_holds_the_cap_while_projecting_requirements`](../../../../crates/rfb-core/src/game/tests/sniper.rs#L512) | 独立机制 | K | 初筛 |
| [`game::tests::sniper::concentration_only_clears_after_a_real_action_or_valid_shot`](../../../../crates/rfb-core/src/game/tests/sniper.rs#L563) | 独立机制 | K | 初筛 |
| [`game::tests::sniper::class_ability_concentration_and_hit_point_costs_are_atomic`](../../../../crates/rfb-core/src/game/tests/sniper.rs#L622) | 独立机制 | K | 初筛 |
| [`game::tests::sniper::bolt_and_focus_multiply_the_critical_chance_without_affecting_other_classes`](../../../../crates/rfb-core/src/game/tests/sniper.rs#L666) | 独立机制 | K | 初筛 |
| [`game::tests::sniper::sniper_state_round_trips_and_rejects_invalid_build_or_bounds`](../../../../crates/rfb-core/src/game/tests/sniper.rs#L696) | 独立机制 | K | 初筛 |
| [`game::tests::sniper::shining_disarming_and_shatter_shots_mutate_only_the_projectile_path`](../../../../crates/rfb-core/src/game/tests/sniper.rs#L751) | 独立机制 | K | 初筛 |
| [`game::tests::sniper::knockback_and_piercing_share_collision_training_and_focus_rules`](../../../../crates/rfb-core/src/game/tests/sniper.rs#L793) | 独立机制 | K | 初筛 |
| [`game::tests::sniper::retreat_uses_focus_scaled_range_and_special_abilities_use_shot_energy`](../../../../crates/rfb-core/src/game/tests/sniper.rs#L848) | 独立机制 | K | 初筛 |
| [`game::tests::sniper::double_shot_uses_two_instances_and_degrades_to_one_when_ammunition_is_short`](../../../../crates/rfb-core/src/game/tests/sniper.rs#L907) | 独立机制 | K | 初筛 |
| [`game::tests::sniper::ammunition_ego_returning_rolls_only_after_target_validation_and_before_split`](../../../../crates/rfb-core/src/game/tests/sniper.rs#L959) | 独立机制 | K | 初筛 |
| [`game::tests::sniper::ammunition_ego_exploding_uses_hit_damage_and_yields_to_sniper_mode`](../../../../crates/rfb-core/src/game/tests/sniper.rs#L1059) | 独立机制 | K | 初筛 |
| [`game::tests::sniper::ammunition_ego_endurance_orders_breakage_and_all_destruction_protection`](../../../../crates/rfb-core/src/game/tests/sniper.rs#L1165) | 独立机制 | K | 初筛 |
| [`game::tests::sniper::exploding_shot_uses_focus_radius_and_final_shot_applies_original_recoil`](../../../../crates/rfb-core/src/game/tests/sniper.rs#L1290) | 独立机制 | K | 初筛 |
| [`game::tests::sniper::probe_projects_each_visible_projectable_monster_and_records_lore_by_kind`](../../../../crates/rfb-core/src/game/tests/sniper.rs#L1366) | 独立机制 | K | 初筛 |
| [`game::tests::sniper::ranged_easy_tiring_uses_the_original_extra_chance_after_a_real_shot`](../../../../crates/rfb-core/src/game/tests/sniper.rs#L1449) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/snow.rs`（4）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::snow::snow_action_cost_matches_original_weight_mount_and_world_caps`](../../../../crates/rfb-core/src/game/tests/snow.rs#L46) | 独立机制 | K | 初筛 |
| [`game::tests::snow::snow_penalty_applies_only_after_a_successful_local_move`](../../../../crates/rfb-core/src/game/tests/snow.rs#L75) | 独立机制 | K | 初筛 |
| [`game::tests::snow::flight_high_elf_and_snow_adapted_mounts_ignore_snow`](../../../../crates/rfb-core/src/game/tests/snow.rs#L104) | 独立机制 | K | 初筛 |
| [`game::tests::snow::successful_world_map_move_into_snow_uses_the_capped_surcharge`](../../../../crates/rfb-core/src/game/tests/snow.rs#L136) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/tasks.rs`（37）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::tasks::base_item_natural_egos_cover_completed_weapon_digger_and_ranged_types`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L61) | 低收益抽样 | S05 | 正文 |
| [`game::tests::tasks::shared_base_and_warrior_loot_use_depth_instead_of_dungeon_identity`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L158) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::warrens_keeper_drop_count_is_one_d_two_and_items_only`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L236) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::warrens_monster_drops_follow_original_probability_and_remains_profiles`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L263) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::campaign_victory_plan_commits_ordered_events_once_without_rng`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L339) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::external_task_service_projects_sparse_available_state_and_accepts_at_entrance`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L375) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::p107_task_substitutions_are_correlated_persisted_and_hide_losing_variants`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L447) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::p107_failed_prerequisites_unlock_and_optional_status_descriptions_project`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L508) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::p107j_rewardless_service_task_waits_for_conclusion_without_creating_an_item`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L574) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::p110_thalos_projects_five_correlated_tasks_from_each_quest_line`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L662) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::external_task_prerequisite_stays_locked_without_materializing_state`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L744) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::accepted_external_task_binds_while_inside_its_dungeon_depth`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L781) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::external_task_service_rejects_unavailable_commands_without_rng_or_state_changes`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L809) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::task_rewards_use_one_weighted_default_choice_and_class_affix_overrides`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L848) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::accepting_thieves_hideout_at_the_count_opens_its_count_district_entry`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L913) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::trouble_at_home_runs_from_white_horse_targets_only_mercenaries_and_rewards_warrior`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L940) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::crows_nest_unlocks_after_trouble_at_home_clears_all_birds_and_rewards_a_staff`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L1058) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::old_man_willow_unlocks_after_crows_nest_and_rewards_an_elemental_ring`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L1147) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::vapor_quest_unlocks_after_old_man_willow_clears_the_cellar_and_rewards_detection`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L1253) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::old_castle_unlocks_after_vapor_quest_and_rewards_the_warrior_artifact_pool`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L1349) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::old_castle_reward_is_forced_even_when_the_artifact_was_generated_before_claim`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L1425) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::clearing_thieves_hideout_closes_the_floor_without_granting_the_reward`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L1460) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::leaving_thieves_hideout_uncleared_fails_and_closes_the_entry`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L1508) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::count_grants_the_warrior_broad_sword_only_when_claimed`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L1535) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::pest_control_unlocks_only_after_the_thieves_reward_is_claimed`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L1598) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::count_accepts_pest_control_without_advancing_rng`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L1644) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::pest_control_floor_places_the_remaining_wargs_and_hides_downstairs`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L1676) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::final_pest_control_kill_reveals_a_magic_stair_without_rng`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L1707) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::leaving_pest_control_incomplete_fails_and_discards_the_blocked_floor`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L1774) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::count_grants_the_fur_cloak_only_when_pest_control_is_claimed`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L1806) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::count_follow_up_tasks_unlock_in_the_original_order`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L1845) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::royal_crypt_places_five_archliches_on_its_level_seventy_fixed_floor`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L1897) | 独立机制 | K | 初筛 |
| [`game::tests::tasks::warrens_dungeon_conquest_returns_retires_and_round_trips`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L1938) | 参数变体 | P08 | 初筛 |
| [`game::tests::tasks::orc_cave_guardian_conquest_reward_and_surface_return_round_trip`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L2095) | 参数变体 | P08 | 初筛 |
| [`game::tests::tasks::p86d_camelot_entrance_recall_conquest_and_reward_round_trip`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L2222) | 参数变体 | P08 | 初筛 |
| [`game::tests::tasks::p87d_tidal_cave_entrance_recall_conquest_and_reward_round_trip`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L2417) | 参数变体 | P08 | 初筛 |
| [`game::tests::tasks::p88d_icky_cave_entrance_recall_conquest_and_reward_round_trip`](../../../../crates/rfb-core/src/game/tests/tasks.rs#L2620) | 参数变体 | P08 | 初筛 |

## `crates/rfb-core/src/game/tests/town.rs`（40）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::town::outpost_shops_are_projected_from_authoritative_content`](../../../../crates/rfb-core/src/game/tests/town.rs#L185) | 独立机制 | K | 初筛 |
| [`game::tests::town::p108c_thalos_projects_its_embedded_icky_cave_and_returns_to_town`](../../../../crates/rfb-core/src/game/tests/town.rs#L244) | 独立机制 | K | 初筛 |
| [`game::tests::town::p109c_thalos_inn_travels_to_a_visited_town_for_five_hundred_gold`](../../../../crates/rfb-core/src/game/tests/town.rs#L300) | 独立机制 | K | 初筛 |
| [`game::tests::town::p109d_thalos_museum_stores_ordinary_items_and_rejects_true_artifacts`](../../../../crates/rfb-core/src/game/tests/town.rs#L328) | 独立机制 | K | 初筛 |
| [`game::tests::town::shroomery_trade_maintenance_and_save_round_trip_use_existing_shop_state`](../../../../crates/rfb-core/src/game/tests/town.rs#L401) | 独立机制 | K | 初筛 |
| [`game::tests::town::shroomery_refuses_formal_and_temporary_snotlings`](../../../../crates/rfb-core/src/game/tests/town.rs#L466) | 独立机制 | K | 初筛 |
| [`game::tests::town::anambar_inn_stay_advances_half_day_and_restores_the_player`](../../../../crates/rfb-core/src/game/tests/town.rs#L506) | 独立机制 | K | 初筛 |
| [`game::tests::town::anambar_inn_rejections_do_not_charge_or_advance_time`](../../../../crates/rfb-core/src/game/tests/town.rs#L581) | 独立机制 | K | 初筛 |
| [`game::tests::town::white_horse_inn_uses_its_content_price`](../../../../crates/rfb-core/src/game/tests/town.rs#L625) | 独立机制 | K | 初筛 |
| [`game::tests::town::inn_travel_requires_a_visited_town_and_arrives_at_its_inn`](../../../../crates/rfb-core/src/game/tests/town.rs#L658) | 独立机制 | K | 初筛 |
| [`game::tests::town::outpost_count_identifies_carried_items_for_fifty_gold`](../../../../crates/rfb-core/src/game/tests/town.rs#L712) | 独立机制 | K | 初筛 |
| [`game::tests::town::outpost_count_legal_name_change_is_validated_saved_and_projected`](../../../../crates/rfb-core/src/game/tests/town.rs#L752) | 独立机制 | K | 初筛 |
| [`game::tests::town::p104c_anambar_library_identifies_researches_and_identifies_all_without_time_or_rng`](../../../../crates/rfb-core/src/game/tests/town.rs#L798) | 独立机制 | K | 初筛 |
| [`game::tests::town::p105c_anambar_facilities_apply_roles_prices_recovery_enchantment_assessment_and_recall`](../../../../crates/rfb-core/src/game/tests/town.rs#L927) | 独立机制 | K | 初筛 |
| [`game::tests::town::outpost_temple_has_walkable_space_on_both_sides_and_to_the_south`](../../../../crates/rfb-core/src/game/tests/town.rs#L1201) | 独立机制 | K | 初筛 |
| [`game::tests::town::home_deposit_withdraw_grouping_and_save_are_authoritative`](../../../../crates/rfb-core/src/game/tests/town.rs#L1223) | 独立机制 | K | 初筛 |
| [`game::tests::town::anambar_home_uses_the_outpost_home_inventory`](../../../../crates/rfb-core/src/game/tests/town.rs#L1309) | 独立机制 | K | 初筛 |
| [`game::tests::town::overburdened_player_can_withdraw_from_home`](../../../../crates/rfb-core/src/game/tests/town.rs#L1400) | 独立机制 | K | 初筛 |
| [`game::tests::town::home_inventory_ids_are_reserved_by_the_global_allocator`](../../../../crates/rfb-core/src/game/tests/town.rs#L1448) | 独立机制 | K | 初筛 |
| [`game::tests::town::entering_general_store_entrance_marks_persistent_shop_visit`](../../../../crates/rfb-core/src/game/tests/town.rs#L1480) | 独立机制 | K | 初筛 |
| [`game::tests::town::malformed_town_state_is_rejected`](../../../../crates/rfb-core/src/game/tests/town.rs#L1507) | 独立机制 | K | 初筛 |
| [`game::tests::town::runtime_town_validation_requires_complete_home_state`](../../../../crates/rfb-core/src/game/tests/town.rs#L1518) | 独立机制 | K | 初筛 |
| [`game::tests::town::missing_unentered_shop_state_is_created_on_first_entry`](../../../../crates/rfb-core/src/game/tests/town.rs#L1529) | 独立机制 | K | 初筛 |
| [`game::tests::town::initial_shop_stock_is_seeded_independent_and_persistent`](../../../../crates/rfb-core/src/game/tests/town.rs#L1548) | 独立机制 | K | 初筛 |
| [`game::tests::town::current_warrior_uses_rfb_price_factor_and_trade_values`](../../../../crates/rfb-core/src/game/tests/town.rs#L1588) | 独立机制 | K | 初筛 |
| [`game::tests::town::player_made_ammunition_keeps_its_ninety_nine_percent_shop_discount`](../../../../crates/rfb-core/src/game/tests/town.rs#L1609) | 独立机制 | K | 初筛 |
| [`game::tests::town::black_market_uses_original_warrior_markup_and_markdown`](../../../../crates/rfb-core/src/game/tests/town.rs#L1650) | 独立机制 | K | 初筛 |
| [`game::tests::town::temple_purchase_and_alchemist_visit_use_independent_shop_state`](../../../../crates/rfb-core/src/game/tests/town.rs#L1702) | 独立机制 | K | 初筛 |
| [`game::tests::town::bookstore_purchase_can_supply_an_original_spellbook_for_study`](../../../../crates/rfb-core/src/game/tests/town.rs#L1766) | 独立机制 | K | 初筛 |
| [`game::tests::town::shared_forge_shops_group_stock_and_sell_equipment_that_can_be_used`](../../../../crates/rfb-core/src/game/tests/town.rs#L1849) | 独立机制 | K | 初筛 |
| [`game::tests::town::magic_shop_purchase_device_use_and_save_are_authoritative`](../../../../crates/rfb-core/src/game/tests/town.rs#L1943) | 独立机制 | K | 初筛 |
| [`game::tests::town::quantity_purchase_is_atomic_zero_time_and_identified`](../../../../crates/rfb-core/src/game/tests/town.rs#L2033) | 独立机制 | K | 初筛 |
| [`game::tests::town::rejected_purchase_preserves_rng_and_business_state`](../../../../crates/rfb-core/src/game/tests/town.rs#L2102) | 独立机制 | K | 初筛 |
| [`game::tests::town::overburdened_player_can_purchase`](../../../../crates/rfb-core/src/game/tests/town.rs#L2133) | 独立机制 | K | 初筛 |
| [`game::tests::town::corpse_sale_is_rejected`](../../../../crates/rfb-core/src/game/tests/town.rs#L2162) | 独立机制 | K | 初筛 |
| [`game::tests::town::sold_item_can_be_bought_back_with_full_instance_state`](../../../../crates/rfb-core/src/game/tests/town.rs#L2185) | 独立机制 | K | 初筛 |
| [`game::tests::town::compatible_shop_instances_project_and_trade_as_one_row`](../../../../crates/rfb-core/src/game/tests/town.rs#L2252) | 独立机制 | K | 初筛 |
| [`game::tests::town::maintenance_refills_only_after_interval_at_entrance`](../../../../crates/rfb-core/src/game/tests/town.rs#L2379) | 独立机制 | K | 初筛 |
| [`game::tests::town::p106_bounty_office_projects_and_redeems_daily_and_wanted_remains`](../../../../crates/rfb-core/src/game/tests/town.rs#L2414) | 独立机制 | K | 初筛 |
| [`game::tests::town::p106_dynamic_bounty_spawns_only_counted_targets_and_round_trips`](../../../../crates/rfb-core/src/game/tests/town.rs#L2491) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/trait_details.rs`（9）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::trait_details::trait_details_are_stable_across_save_round_trip`](../../../../crates/rfb-core/src/game/tests/trait_details.rs#L47) | 独立机制 | K | 初筛 |
| [`game::tests::trait_details::trait_details_auras_share_combat_sources_without_rolling_damage`](../../../../crates/rfb-core/src/game/tests/trait_details.rs#L58) | 独立机制 | K | 初筛 |
| [`game::tests::trait_details::trait_details_curses_do_not_infer_effects_from_severity_or_reveal_unknown_affixes`](../../../../crates/rfb-core/src/game/tests/trait_details.rs#L97) | 独立机制 | K | 初筛 |
| [`game::tests::trait_details::trait_details_attack_counts_follow_the_selected_weapon_and_launcher`](../../../../crates/rfb-core/src/game/tests/trait_details.rs#L180) | 独立机制 | K | 初筛 |
| [`game::tests::trait_details::trait_details_follow_resistance_merge_and_do_not_mutate_state`](../../../../crates/rfb-core/src/game/tests/trait_details.rs#L255) | 独立机制 | K | 初筛 |
| [`game::tests::trait_details::trait_details_withhold_unknown_totals_and_reveal_only_known_properties`](../../../../crates/rfb-core/src/game/tests/trait_details.rs#L327) | 独立机制 | K | 初筛 |
| [`game::tests::trait_details::trait_details_count_only_rule_counted_abilities_and_preserve_units`](../../../../crates/rfb-core/src/game/tests/trait_details.rs#L434) | 独立机制 | K | 初筛 |
| [`game::tests::trait_details::trait_details_action_protection_and_targeted_senses_follow_known_current_sources`](../../../../crates/rfb-core/src/game/tests/trait_details.rs#L532) | 独立机制 | K | 初筛 |
| [`game::tests::trait_details::trait_details_separate_armed_melee_ammunition_and_own_weapon`](../../../../crates/rfb-core/src/game/tests/trait_details.rs#L619) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/virtue_state.rs`（6）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::virtue_state::hidden_draconian_subraces_keep_the_authoritative_enchantment_virtue`](../../../../crates/rfb-core/src/game/tests/virtue_state.rs#L27) | 参数变体 | P03 | 初筛 |
| [`game::tests::virtue_state::rfb_virtue_initialization_keeps_class_race_and_realm_order_then_fills_unique_slots`](../../../../crates/rfb-core/src/game/tests/virtue_state.rs#L53) | 独立机制 | K | 初筛 |
| [`game::tests::virtue_state::formal_races_receive_their_original_race_virtues`](../../../../crates/rfb-core/src/game/tests/virtue_state.rs#L100) | 参数变体 | P03 | 初筛 |
| [`game::tests::virtue_state::virtue_changes_use_the_three_rfb_soft_caps_and_hard_bounds`](../../../../crates/rfb-core/src/game/tests/virtue_state.rs#L299) | 参数变体 | P13 | 初筛 |
| [`game::tests::virtue_state::chance_virtue_adjusts_rolls_with_the_authoritative_repeated_d400_rule`](../../../../crates/rfb-core/src/game/tests/virtue_state.rs#L319) | 独立机制 | K | 初筛 |
| [`game::tests::virtue_state::virtue_state_round_trips_and_rejects_invalid_slots`](../../../../crates/rfb-core/src/game/tests/virtue_state.rs#L335) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/weapon_ego_activations.rs`（3）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::weapon_ego_activations::riding_charge_cancellation_preserves_charge_and_rng`](../../../../crates/rfb-core/src/game/tests/weapon_ego_activations.rs#L197) | 独立机制 | K | 初筛 |
| [`game::tests::weapon_ego_activations::riding_charge_moves_mount_attacks_and_uses_profile_recovery`](../../../../crates/rfb-core/src/game/tests/weapon_ego_activations.rs#L230) | 独立机制 | K | 初筛 |
| [`game::tests::weapon_ego_activations::biased_ego_activation_reuses_the_ability_effect_resolver`](../../../../crates/rfb-core/src/game/tests/weapon_ego_activations.rs#L310) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/tests/weapon_proficiency.rs`（8）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::weapon_proficiency::display_groups_follow_rfb_tval_and_equipment_resolves_to_base_kind`](../../../../crates/rfb-core/src/game/tests/weapon_proficiency.rs#L68) | 独立机制 | K | 初筛 |
| [`game::tests::weapon_proficiency::growth_uses_original_gates_rng_remainders_and_bonus_notifications`](../../../../crates/rfb-core/src/game/tests/weapon_proficiency.rs#L132) | 独立机制 | K | 初筛 |
| [`game::tests::weapon_proficiency::ordinary_melee_trains_once_before_multiple_missed_blows`](../../../../crates/rfb-core/src/game/tests/weapon_proficiency.rs#L173) | 独立机制 | K | 初筛 |
| [`game::tests::weapon_proficiency::projectile_collision_trains_but_an_empty_shot_does_not_touch_progress_or_rng`](../../../../crates/rfb-core/src/game/tests/weapon_proficiency.rs#L213) | 独立机制 | K | 初筛 |
| [`game::tests::weapon_proficiency::artifact_training_uses_only_the_canonical_base_weapon_key`](../../../../crates/rfb-core/src/game/tests/weapon_proficiency.rs#L263) | 独立机制 | K | 初筛 |
| [`game::tests::weapon_proficiency::progression_projection_lists_base_weapons_with_original_ranks_and_bonuses`](../../../../crates/rfb-core/src/game/tests/weapon_proficiency.rs#L288) | 独立机制 | K | 初筛 |
| [`game::tests::weapon_proficiency::sparse_weapon_progress_round_trips_and_rejects_noncanonical_or_out_of_range_entries`](../../../../crates/rfb-core/src/game/tests/weapon_proficiency.rs#L329) | 独立机制 | K | 初筛 |
| [`game::tests::weapon_proficiency::weapon_proficiency_save_field_is_required_for_new_progress_payloads`](../../../../crates/rfb-core/src/game/tests/weapon_proficiency.rs#L409) | 参数变体 | P10 | 初筛 |

## `crates/rfb-core/src/game/tests/weapon_traits.rs`（9）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::weapon_traits::mana_weapon_uses_current_dice_and_only_pays_for_successful_affordable_hits`](../../../../crates/rfb-core/src/game/tests/weapon_traits.rs#L90) | 独立机制 | K | 初筛 |
| [`game::tests::weapon_traits::vorpal_and_vorpal2_chain_after_the_shared_weapon_critical_path`](../../../../crates/rfb-core/src/game/tests/weapon_traits.rs#L151) | 独立机制 | K | 初筛 |
| [`game::tests::weapon_traits::order_weapon_rolls_maximum_damage_and_skips_weapon_dice_and_critical_rng`](../../../../crates/rfb-core/src/game/tests/weapon_traits.rs#L212) | 独立机制 | K | 初筛 |
| [`game::tests::weapon_traits::impact_weapon_reuses_earthquake_and_strong_hit_stun_without_extra_trigger_rng`](../../../../crates/rfb-core/src/game/tests/weapon_traits.rs#L248) | 独立机制 | K | 初筛 |
| [`game::tests::weapon_traits::stun_weapon_checks_post_critical_damage_and_respects_immunity`](../../../../crates/rfb-core/src/game/tests/weapon_traits.rs#L321) | 独立机制 | K | 初筛 |
| [`game::tests::weapon_traits::blessed_weapon_resists_curses_and_exempts_good_priest_weapon_penalties`](../../../../crates/rfb-core/src/game/tests/weapon_traits.rs#L398) | 独立机制 | K | 初筛 |
| [`game::tests::weapon_traits::wild_weapon_activates_on_hit_for_two_ticks_without_spending_rng_on_miss`](../../../../crates/rfb-core/src/game/tests/weapon_traits.rs#L471) | 独立机制 | K | 初筛 |
| [`game::tests::weapon_traits::wild_weapon_selects_inactive_powers_before_filling_or_replacing_five_slots`](../../../../crates/rfb-core/src/game/tests/weapon_traits.rs#L513) | 独立机制 | K | 初筛 |
| [`game::tests::weapon_traits::wild_weapon_weight_table_builds_all_fourteen_concrete_status_effects`](../../../../crates/rfb-core/src/game/tests/weapon_traits.rs#L555) | 低收益抽样 | S06 | 正文 |

## `crates/rfb-core/src/game/tests/world.rs`（66）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::tests::world::p89b_substitute_selection_is_seeded_persisted_and_hashed`](../../../../crates/rfb-core/src/game/tests/world.rs#L161) | 独立机制 | K | 初筛 |
| [`game::tests::world::p89c_outpost_shared_entrance_routes_only_to_the_active_dungeon`](../../../../crates/rfb-core/src/game/tests/world.rs#L211) | 独立机制 | K | 初筛 |
| [`game::tests::world::p96c_numenor_atlantis_selection_and_shared_entrance_are_seed_stable`](../../../../crates/rfb-core/src/game/tests/world.rs#L274) | 独立机制 | K | 初筛 |
| [`game::tests::world::p96d_all_numenor_atlantis_floors_generate_water_veins_and_ordinary_stairs`](../../../../crates/rfb-core/src/game/tests/world.rs#L353) | 参数变体 | P07 | 初筛 |
| [`game::tests::world::alternate_dungeons_project_only_the_selected_original_position`](../../../../crates/rfb-core/src/game/tests/world.rs#L423) | 独立机制 | K | 初筛 |
| [`game::tests::world::p99f_all_giants_hall_and_snow_castle_floors_generate_without_doors`](../../../../crates/rfb-core/src/game/tests/world.rs#L485) | 参数变体 | P07 | 初筛 |
| [`game::tests::world::p100f_all_graveyard_floors_generate_shallow_water_layers_and_shafts`](../../../../crates/rfb-core/src/game/tests/world.rs#L543) | 参数变体 | P07 | 初筛 |
| [`game::tests::world::p101d_all_witch_wood_and_plains_of_oz_floors_generate_their_outdoor_layers`](../../../../crates/rfb-core/src/game/tests/world.rs#L619) | 参数变体 | P07 | 初筛 |
| [`game::tests::world::p101e_guardian_reward_uses_the_players_first_realm_third_book`](../../../../crates/rfb-core/src/game/tests/world.rs#L699) | 独立机制 | K | 初筛 |
| [`game::tests::world::p89d_hideout_reward_materializes_a_nonblank_am_quest_amulet`](../../../../crates/rfb-core/src/game/tests/world.rs#L746) | 独立机制 | K | 初筛 |
| [`game::tests::world::p90c_troll_cave_generation_keeps_terrain_mix_lakes_shafts_and_connectivity`](../../../../crates/rfb-core/src/game/tests/world.rs#L827) | 参数变体 | P07 | 初筛 |
| [`game::tests::world::p90c_troll_cave_shared_entry_shafts_conquest_and_reward_are_one_shot`](../../../../crates/rfb-core/src/game/tests/world.rs#L948) | 参数变体 | P08 | 初筛 |
| [`game::tests::world::p91c_eyrie_generation_keeps_caverns_rivers_shafts_and_connectivity`](../../../../crates/rfb-core/src/game/tests/world.rs#L1045) | 参数变体 | P07 | 初筛 |
| [`game::tests::world::p91c_eyrie_guardians_shafts_conquest_and_new_life_reward_are_one_shot`](../../../../crates/rfb-core/src/game/tests/world.rs#L1141) | 参数变体 | P08 | 初筛 |
| [`game::tests::world::p92c_labyrinth_generation_keeps_the_small_perfect_maze_connected`](../../../../crates/rfb-core/src/game/tests/world.rs#L1284) | 参数变体 | P07 | 初筛 |
| [`game::tests::world::p92c_labyrinth_forgets_after_movement_and_drops_the_fixed_recall_rod`](../../../../crates/rfb-core/src/game/tests/world.rs#L1398) | 独立机制 | K | 初筛 |
| [`game::tests::world::p93c_lonely_mountain_generation_keeps_lava_caverns_lakes_and_destruction`](../../../../crates/rfb-core/src/game/tests/world.rs#L1488) | 参数变体 | P07 | 初筛 |
| [`game::tests::world::p93c_smaug_drops_arkenstone_with_clairvoyance_and_replacement`](../../../../crates/rfb-core/src/game/tests/world.rs#L1563) | 参数变体 | P08 | 初筛 |
| [`game::tests::world::p97e_dragon_lair_generation_keeps_lava_caverns_lakes_and_guardians`](../../../../crates/rfb-core/src/game/tests/world.rs#L1668) | 参数变体 | P07 | 初筛 |
| [`game::tests::world::p97e_dragon_lair_guardians_and_scale_mail_reward_are_one_shot`](../../../../crates/rfb-core/src/game/tests/world.rs#L1743) | 参数变体 | P08 | 初筛 |
| [`game::tests::world::p98c_castle_generation_keeps_rooms_stairs_and_representative_layers`](../../../../crates/rfb-core/src/game/tests/world.rs#L1825) | 参数变体 | P07 | 初筛 |
| [`game::tests::world::p98c_castle_guardians_and_conquest_are_one_shot`](../../../../crates/rfb-core/src/game/tests/world.rs#L1913) | 参数变体 | P08 | 初筛 |
| [`game::tests::world::p100f_graveyard_guardians_and_rolled_soulsword_reward_are_one_shot`](../../../../crates/rfb-core/src/game/tests/world.rs#L1975) | 参数变体 | P08 | 初筛 |
| [`game::tests::world::p94c_mine_generation_selects_dry_water_or_lava_rivers_with_rich_veins`](../../../../crates/rfb-core/src/game/tests/world.rs#L2059) | 独立机制 | K | 初筛 |
| [`game::tests::world::p94c_mine_guardians_and_star_healing_reward_are_one_shot`](../../../../crates/rfb-core/src/game/tests/world.rs#L2117) | 参数变体 | P08 | 初筛 |
| [`game::tests::world::p95c_battlefield_generation_keeps_alignment_ecology_and_mixed_ground`](../../../../crates/rfb-core/src/game/tests/world.rs#L2183) | 参数变体 | P07 | 初筛 |
| [`game::tests::world::p95c_battlefield_guardians_reward_and_no_enchant_are_one_shot`](../../../../crates/rfb-core/src/game/tests/world.rs#L2231) | 参数变体 | P08 | 初筛 |
| [`game::tests::world::p89f_hideout_conquest_and_am_quest_reward_are_one_shot`](../../../../crates/rfb-core/src/game/tests/world.rs#L2358) | 参数变体 | P08 | 初筛 |
| [`game::tests::world::p89f_man_cave_conquest_lotharang_activation_and_replacement_are_one_shot`](../../../../crates/rfb-core/src/game/tests/world.rs#L2393) | 参数变体 | P08 | 初筛 |
| [`game::tests::world::middle_earth_starts_on_an_outdoor_surface_with_a_working_warrens_entrance`](../../../../crates/rfb-core/src/game/tests/world.rs#L2519) | 独立机制 | K | 初筛 |
| [`game::tests::world::dungeon_round_trip_restores_the_scrolled_town_position`](../../../../crates/rfb-core/src/game/tests/world.rs#L2553) | 独立机制 | K | 初筛 |
| [`game::tests::world::thieves_hideout_inline_floor_preserves_the_fixed_map_and_six_member_formation`](../../../../crates/rfb-core/src/game/tests/world.rs#L2595) | 独立机制 | K | 初筛 |
| [`game::tests::world::trouble_at_home_inline_floor_preserves_map_spawns_and_two_item_scramble`](../../../../crates/rfb-core/src/game/tests/world.rs#L2707) | 独立机制 | K | 初筛 |
| [`game::tests::world::crows_nest_inline_floor_preserves_map_birds_and_group_scramble`](../../../../crates/rfb-core/src/game/tests/world.rs#L2874) | 独立机制 | K | 初筛 |
| [`game::tests::world::old_man_willow_inline_floor_preserves_the_original_grove_and_formation`](../../../../crates/rfb-core/src/game/tests/world.rs#L3014) | 独立机制 | K | 初筛 |
| [`game::tests::world::vapor_quest_inline_floor_preserves_the_original_cellar_formation_and_jewelry`](../../../../crates/rfb-core/src/game/tests/world.rs#L3107) | 独立机制 | K | 初筛 |
| [`game::tests::world::warrens_surface_reentry_starts_a_fresh_expedition_with_new_monsters`](../../../../crates/rfb-core/src/game/tests/world.rs#L3220) | 独立机制 | K | 初筛 |
| [`game::tests::world::p87c_tidal_cave_room_water_and_optional_river_use_existing_terrain`](../../../../crates/rfb-core/src/game/tests/world.rs#L3255) | 独立机制 | K | 初筛 |
| [`game::tests::world::p88e_icky_cave_all_depths_keep_the_terrain_mix_and_stairs_reachable`](../../../../crates/rfb-core/src/game/tests/world.rs#L3312) | 参数变体 | P07 | 初筛 |
| [`game::tests::world::p87e_tidal_cave_all_depths_keep_water_and_stairs_reachable`](../../../../crates/rfb-core/src/game/tests/world.rs#L3437) | 参数变体 | P07 | 初筛 |
| [`game::tests::world::warrens_maps_are_seeded_connected_varied_and_persistent`](../../../../crates/rfb-core/src/game/tests/world.rs#L3506) | 独立机制 | K | 初筛 |
| [`game::tests::world::warrens_every_generated_floor_has_a_normal_descent_and_return_route`](../../../../crates/rfb-core/src/game/tests/world.rs#L3657) | 低收益抽样 | S07 | 正文 |
| [`game::tests::world::terrain_interaction_plans_reject_unsupported_actions_without_rng`](../../../../crates/rfb-core/src/game/tests/world.rs#L3742) | 独立机制 | K | 初筛 |
| [`game::tests::world::digging_uses_original_soft_hard_and_permanent_resolution`](../../../../crates/rfb-core/src/game/tests/world.rs#L3769) | 独立机制 | K | 初筛 |
| [`game::tests::world::digging_ignores_ground_items_and_turns_a_blocking_monster_into_melee`](../../../../crates/rfb-core/src/game/tests/world.rs#L3824) | 独立机制 | K | 初筛 |
| [`game::tests::world::warrens_location_requires_its_local_entrance_and_restores_the_outpost`](../../../../crates/rfb-core/src/game/tests/world.rs#L3891) | 独立机制 | K | 初筛 |
| [`game::tests::world::world_map_projects_authoritative_wilderness_cells_and_restores_the_local_map`](../../../../crates/rfb-core/src/game/tests/world.rs#L3941) | 独立机制 | K | 初筛 |
| [`game::tests::world::world_map_movement_uses_original_time_scale_without_advancing_hidden_monsters`](../../../../crates/rfb-core/src/game/tests/world.rs#L4027) | 独立机制 | K | 初筛 |
| [`game::tests::world::entering_world_map_advances_the_wilderness_generation_and_clears_cached_terrain`](../../../../crates/rfb-core/src/game/tests/world.rs#L4061) | 独立机制 | K | 初筛 |
| [`game::tests::world::world_map_round_trip_preserves_the_visible_town_surface`](../../../../crates/rfb-core/src/game/tests/world.rs#L4085) | 独立机制 | K | 初筛 |
| [`game::tests::world::wilderness_daylight_drives_surface_ambient_light`](../../../../crates/rfb-core/src/game/tests/world.rs#L4110) | 独立机制 | K | 初筛 |
| [`game::tests::world::wilderness_ambush_enters_local_combat_and_locks_world_map_until_cleared`](../../../../crates/rfb-core/src/game/tests/world.rs#L4127) | 独立机制 | K | 初筛 |
| [`game::tests::world::local_wilderness_is_coordinate_seeded_and_restores_from_save`](../../../../crates/rfb-core/src/game/tests/world.rs#L4235) | 独立机制 | K | 初筛 |
| [`game::tests::world::small_town_excludes_only_its_rectangle_from_wilderness_monsters`](../../../../crates/rfb-core/src/game/tests/world.rs#L4287) | 独立机制 | K | 初筛 |
| [`game::tests::world::walking_into_the_outer_band_scrolls_and_normalizes_the_wilderness_view`](../../../../crates/rfb-core/src/game/tests/world.rs#L4306) | 独立机制 | K | 初筛 |
| [`game::tests::world::wilderness_scroll_translates_overlap_and_crops_entities_items_gold_and_packs`](../../../../crates/rfb-core/src/game/tests/world.rs#L4367) | 独立机制 | K | 初筛 |
| [`game::tests::world::diagonal_wilderness_scroll_translates_by_one_chunk_on_each_axis`](../../../../crates/rfb-core/src/game/tests/world.rs#L4530) | 独立机制 | K | 初筛 |
| [`game::tests::world::wilderness_scroll_populates_only_the_new_strip_without_using_ambush_rolls`](../../../../crates/rfb-core/src/game/tests/world.rs#L4560) | 独立机制 | K | 初筛 |
| [`game::tests::world::wilderness_scroll_keeps_new_monsters_outside_a_visible_small_town`](../../../../crates/rfb-core/src/game/tests/world.rs#L4616) | 独立机制 | K | 初筛 |
| [`game::tests::world::local_wilderness_cannot_roll_or_activate_a_world_map_ambush`](../../../../crates/rfb-core/src/game/tests/world.rs#L4661) | 独立机制 | K | 初筛 |
| [`game::tests::world::scrolling_into_and_out_of_a_town_stays_on_the_continuous_wilderness_surface`](../../../../crates/rfb-core/src/game/tests/world.rs#L4685) | 独立机制 | K | 初筛 |
| [`game::tests::world::wilderness_view_offset_round_trips_and_rejects_out_of_range_values`](../../../../crates/rfb-core/src/game/tests/world.rs#L4796) | 独立机制 | K | 初筛 |
| [`game::tests::world::returning_to_the_outpost_coordinate_restores_its_preserved_floor`](../../../../crates/rfb-core/src/game/tests/world.rs#L4837) | 独立机制 | K | 初筛 |
| [`game::tests::world::p102c_chameleon_cave_generates_chameleons_and_rewards_polymorph`](../../../../crates/rfb-core/src/game/tests/world.rs#L4870) | 独立机制 | K | 初筛 |
| [`game::tests::world::formal_towns_share_the_continuous_surface_and_initialize_facilities_lazily`](../../../../crates/rfb-core/src/game/tests/world.rs#L4930) | 独立机制 | K | 初筛 |
| [`game::tests::world::p103e_volcano_generates_lava_guardians_and_fixed_staff_reward`](../../../../crates/rfb-core/src/game/tests/world.rs#L5022) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/weapon_proficiency.rs`（4）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::weapon_proficiency::tests::original_interpolation_clamps_and_truncates_like_rfb`](../../../../crates/rfb-core/src/game/weapon_proficiency.rs#L274) | 参数变体 | P13 | 初筛 |
| [`game::weapon_proficiency::tests::bows_and_crossbows_keep_their_distinct_original_bonus_formulas`](../../../../crates/rfb-core/src/game/weapon_proficiency.rs#L282) | 独立机制 | K | 初筛 |
| [`game::weapon_proficiency::tests::original_rank_boundaries_are_projected_exactly`](../../../../crates/rfb-core/src/game/weapon_proficiency.rs#L291) | 参数变体 | P13 | 初筛 |
| [`game::weapon_proficiency::tests::active_class_supplies_distinct_birth_values_and_training_caps`](../../../../crates/rfb-core/src/game/weapon_proficiency.rs#L302) | 参数变体 | P13 | 初筛 |

## `crates/rfb-core/src/game/wilderness.rs`（18）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::wilderness::w3_tests::wilderness_daylight_uses_original_half_day_boundaries`](../../../../crates/rfb-core/src/game/wilderness.rs#L2286) | 独立机制 | K | 初筛 |
| [`game::wilderness::w3_tests::wilderness_ambush_denominator_applies_road_and_night_modifiers`](../../../../crates/rfb-core/src/game/wilderness.rs#L2295) | 独立机制 | K | 初筛 |
| [`game::wilderness::w3_tests::wilderness_initial_monster_rolls_follow_original_road_density`](../../../../crates/rfb-core/src/game/wilderness.rs#L2302) | 独立机制 | K | 初筛 |
| [`game::wilderness::tests::exposed_wilderness_area_is_one_third_or_five_ninths`](../../../../crates/rfb-core/src/game/wilderness.rs#L2314) | 独立机制 | K | 初筛 |
| [`game::wilderness::tests::scroll_monster_rolls_round_by_absolute_chunk_coordinates`](../../../../crates/rfb-core/src/game/wilderness.rs#L2331) | 独立机制 | K | 初筛 |
| [`game::wilderness::tests::monster_rolls_scale_to_the_non_town_area`](../../../../crates/rfb-core/src/game/wilderness.rs#L2352) | 独立机制 | K | 初筛 |
| [`game::wilderness::tests::monster_candidates_exclude_the_actual_town_rectangle_only`](../../../../crates/rfb-core/src/game/wilderness.rs#L2364) | 独立机制 | K | 初筛 |
| [`game::wilderness::tests::adjacent_cached_views_share_the_same_overlapping_terrain`](../../../../crates/rfb-core/src/game/wilderness.rs#L2382) | 独立机制 | K | 初筛 |
| [`game::wilderness::tests::town_terrain_is_composed_after_the_seeded_cache`](../../../../crates/rfb-core/src/game/wilderness.rs#L2404) | 独立机制 | K | 初筛 |
| [`game::wilderness::tests::town_terrain_stays_fixed_when_wilderness_seed_advances`](../../../../crates/rfb-core/src/game/wilderness.rs#L2428) | 独立机制 | K | 初筛 |
| [`game::wilderness::tests::town_overlay_remains_continuous_when_revealed_from_each_direction`](../../../../crates/rfb-core/src/game/wilderness.rs#L2453) | 独立机制 | K | 初筛 |
| [`game::wilderness::tests::birth_town_uses_the_continuous_wilderness_surface`](../../../../crates/rfb-core/src/game/wilderness.rs#L2515) | 独立机制 | K | 初筛 |
| [`game::wilderness::tests::town_state_moves_to_backing_storage_and_returns_with_the_view`](../../../../crates/rfb-core/src/game/wilderness.rs#L2534) | 独立机制 | K | 初筛 |
| [`game::wilderness::tests::first_visible_anambar_slice_initializes_its_backing_floor`](../../../../crates/rfb-core/src/game/wilderness.rs#L2642) | 独立机制 | K | 初筛 |
| [`game::wilderness::tests::wilderness_terrain_cache_is_derived_bounded_and_seeded_by_generation`](../../../../crates/rfb-core/src/game/wilderness.rs#L2666) | 独立机制 | K | 初筛 |
| [`game::wilderness::tests::world_movement_rejects_edges_but_allows_unmounted_deep_water_entry`](../../../../crates/rfb-core/src/game/wilderness.rs#L2697) | 独立机制 | K | 初筛 |
| [`game::wilderness::tests::world_travel_uses_eight_direction_pathfinding`](../../../../crates/rfb-core/src/game/wilderness.rs#L2707) | 独立机制 | K | 初筛 |
| [`game::wilderness::tests::low_level_interesting_sites_paint_the_authoritative_ruined_home`](../../../../crates/rfb-core/src/game/wilderness.rs#L2726) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/game/world/generation.rs`（1）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`game::world::generation::tests::streamer_treasure_rolls_known_then_hidden_after_a_miss`](../../../../crates/rfb-core/src/game/world/generation.rs#L4839) | 低收益抽样 | S08 | 正文 |

## `crates/rfb-core/src/mogaminator.rs`（6）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`mogaminator::tests::parses_original_actions_bilingual_predicates_and_inscription`](../../../../crates/rfb-core/src/mogaminator.rs#L971) | 独立机制 | K | 初筛 |
| [`mogaminator::tests::parses_conditions_with_english_and_chinese_aliases`](../../../../crates/rfb-core/src/mogaminator.rs#L1011) | 独立机制 | K | 初筛 |
| [`mogaminator::tests::keeps_blank_comments_and_plain_text_searches`](../../../../crates/rfb-core/src/mogaminator.rs#L1041) | 独立机制 | K | 初筛 |
| [`mogaminator::tests::returns_stable_diagnostics_for_structural_errors`](../../../../crates/rfb-core/src/mogaminator.rs#L1055) | 独立机制 | K | 初筛 |
| [`mogaminator::tests::accepts_every_predicate_alias_and_action_symbol`](../../../../crates/rfb-core/src/mogaminator.rs#L1077) | 参数变体 | P13 | 初筛 |
| [`mogaminator::tests::compiler_rejects_unsupported_variables_even_below_false_condition`](../../../../crates/rfb-core/src/mogaminator.rs#L1110) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/resistance.rs`（1）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`resistance::tests::normal_levels_are_implicit_and_non_normal_levels_are_stable`](../../../../crates/rfb-core/src/resistance.rs#L309) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/rng.rs`（1）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`rng::tests::m_bonus_uses_explicit_level_and_supports_large_deviations`](../../../../crates/rfb-core/src/rng.rs#L141) | 独立机制 | K | 初筛 |

## `crates/rfb-core/src/scheduler.rs`（2）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`scheduler::tests::standard_speed_gains_ten_energy_per_world_tick`](../../../../crates/rfb-core/src/scheduler.rs#L51) | 独立机制 | K | 正文 |
| [`scheduler::tests::energy_curve_is_monotonic_and_bounded`](../../../../crates/rfb-core/src/scheduler.rs#L58) | 独立机制 | K | 正文 |

## `crates/rfb-core/src/stats.rs`（16）

| 测试 | 分类 | 规则 | 证据 |
|---|---|---|---|
| [`stats::tests::pipeline_orders_sources_by_layer_priority_and_id`](../../../../crates/rfb-core/src/stats.rs#L1029) | 独立机制 | K | 初筛 |
| [`stats::tests::pipeline_clamps_only_the_final_saturating_total`](../../../../crates/rfb-core/src/stats.rs#L1063) | 独立机制 | K | 初筛 |
| [`stats::tests::rfb_attribute_buckets_extend_from_18_220_to_18_820_after_victory`](../../../../crates/rfb-core/src/stats.rs#L1075) | 参数变体 | P13 | 初筛 |
| [`stats::tests::strength_digging_uses_the_original_38_bucket_table`](../../../../crates/rfb-core/src/stats.rs#L1097) | 参数变体 | P13 | 初筛 |
| [`stats::tests::carrying_capacity_uses_the_original_strength_table_and_caps_at_195_pounds`](../../../../crates/rfb-core/src/stats.rs#L1107) | 参数变体 | P13 | 初筛 |
| [`stats::tests::encumbrance_penalty_starts_at_twenty_percent_over_capacity`](../../../../crates/rfb-core/src/stats.rs#L1127) | 独立机制 | K | 初筛 |
| [`stats::tests::experience_thresholds_preserve_rfb_then_extend_to_level_100`](../../../../crates/rfb-core/src/stats.rs#L1135) | 独立机制 | K | 初筛 |
| [`stats::tests::victory_unlocks_banked_experience_through_level_100`](../../../../crates/rfb-core/src/stats.rs#L1145) | 独立机制 | K | 初筛 |
| [`stats::tests::regaining_drained_levels_does_not_repeat_attribute_rewards`](../../../../crates/rfb-core/src/stats.rs#L1161) | 独立机制 | K | 初筛 |
| [`stats::tests::hp_progression_is_seeded_without_using_the_simulation_rng`](../../../../crates/rfb-core/src/stats.rs#L1180) | 独立机制 | K | 初筛 |
| [`stats::tests::birth_attribute_potentials_are_seeded_balanced_and_source_encoded`](../../../../crates/rfb-core/src/stats.rs#L1200) | 独立机制 | K | 初筛 |
| [`stats::tests::hp_rating_filter_uses_early_lower_bounds_and_final_upper_bound`](../../../../crates/rfb-core/src/stats.rs#L1223) | 独立机制 | K | 初筛 |
| [`stats::tests::skill_growth_uses_deterministic_per_ten_level_proration`](../../../../crates/rfb-core/src/stats.rs#L1241) | 独立机制 | K | 初筛 |
| [`stats::tests::attribute_increases_spend_points_and_respect_stage_caps`](../../../../crates/rfb-core/src/stats.rs#L1252) | 独立机制 | K | 初筛 |
| [`stats::tests::attribute_drain_preserves_floor_and_only_rolls_above_18`](../../../../crates/rfb-core/src/stats.rs#L1280) | 参数变体 | P13 | 初筛 |
| [`stats::tests::permanent_attribute_increase_uses_potion_bands_without_spending_level_points`](../../../../crates/rfb-core/src/stats.rs#L1296) | 参数变体 | P13 | 初筛 |
