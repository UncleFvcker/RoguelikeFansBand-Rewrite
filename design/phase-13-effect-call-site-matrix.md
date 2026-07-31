# Phase 13 Effect Call-Site Matrix

Status: characterized from `main` at `153c0311` before Phase 13 production-code movement.

This document is the compatibility checklist for Phase 13. It records the current transaction owners, effect-family entry points, downstream domain owners, and existing characterization evidence. It does not propose a common effect engine. Player casting, item use, and monster casting remain distinct transactions throughout the phase.

## Player ability transaction

`Game::resolve_player_ability` is the authoritative transaction shell. Its observable order is:

1. reject confusion before profile, content, target, resource, or RNG work;
2. resolve the technique/casting profile and effective level-scaled ability;
3. reject learning, level, profile, book, and cooldown failures;
4. build `AbilityTargetPlan` and reject an invalid target without resource, proficiency, cooldown, or RNG changes;
5. calculate and deduct the resource cost;
6. draw the cast percentile and record proficiency, cast/fail count, and cooldown;
7. emit cast failure or success;
8. for `RandomChoice`, draw the branch, emit its resolution, rewrite the leaf effect, and rebuild its cast/self target plan;
9. resolve the selected effect family and append its trace/resolution events in current order.

| Effect family | Variants | Current entry points | Authoritative dependencies that stay in place |
| --- | --- | --- | --- |
| Target planning | all player-castable variants | `ability_target_plan`, `ability_path`, `beam_ability_path` | terrain, occupancy, LOS, visibility, inventory knowledge, floor state |
| Direct projectile damage | `Damage` | inline arm in `resolve_player_ability`, `resolve_player_ability_damage` in `player_combat.rs` | Phase 12 damage/death, target wake, changed cells, event projection |
| Area damage | `AreaDamage` | inline arm, `resolve_player_ability_damage` | one shared damage roll, stable footprint order and falloff, target relookup |
| Beam damage | `BeamDamage` | inline arm, `resolve_player_ability_damage` | one shared damage roll, wall stop, stable path order, target relookup |
| Bolt or beam | `BoltOrBeamDamage` | `resolve_ability_bolt_or_beam`, `resolve_player_ability_damage` | cast roll before penetration roll, one damage roll, stable trace |
| Cone damage | `ConeDamage` | inline arm, `resolve_player_ability_damage` | one shared damage roll, lateral falloff, eight-direction geometry |
| Death ray | `DeathRay` | `resolve_ability_death_ray` | living/unique/level gates, resistance roll, Phase 12 death commit |
| Drain life | `DrainLife` | `resolve_ability_drain_life` | category filter, repeated target relookup, actual HP loss used for healing |
| Visible damage | `VisibleDamage` | inline arm, `resolve_player_ability_damage` | visible actor snapshot, optional category filter, one shared roll |
| Self healing/vitality | `Heal`, `RestoreVitality` | inline arms | current/max HP, life force, progression/resource DTOs |
| Actor status/control | `ApplyStatus`, `RemoveStatus`, `Control`, actor `Sequence` | `resolve_ability_actor_effects`, `resolve_ability_control`, `apply_ability_status_effect`, `remove_ability_status_effect` | duration/resistance RNG, immunity, pack/allegiance mutation, target-death skip |
| Visible status | `VisibleApplyStatus` | inline arm, `apply_ability_status_effect` | visible actor snapshot, stable order, optional category filter |
| Player movement | `Teleport` | inline arm, `relocate_player` | arrival trap semantics, changed cells, scheduling remains outside |
| Detection | `Detect` | inline arm and subject-specific detection helpers | persistent exploration/revealed state, transient DTO-only results |
| Terrain transform | `TransformTerrain` | inline arm | authoritative terrain, occupancy/connections/borders, changed cells |
| Fixed summon | `Summon` | inline arm | entity identity, actor construction, resistance stamping, placement |
| Category summon | `SummonCategory` | `resolve_category_summon` | candidate order, hostility/group/count RNG, ID allocation, uniqueness |
| Animate dead | `AnimateDead` | `resolve_ability_animate_dead` | corpse item selection/consumption, faction/group/unique rules, IDs |
| Genocide | `Genocide` | `resolve_ability_genocide` | explicit removal bypass, unique resistance, fatigue direct HP loss, task events |
| Item knowledge | `IdentifyItem` | inline arm | inventory item identity, knowledge maps, full-identify RNG |
| Equipment mutation | `EnchantEquippedWeapon` | inline arm | equipped-item lookup, affix state and knowledge, derived stat refresh |
| Composition | `RandomChoice`, `NoOp`, `Sequence` | transaction branch handling, inline `NoOp`, `resolve_ability_actor_effects` for actor sequences | branch RNG, target-plan recalculation, trace/DTO ordering, dead-target skip |
| Monster-only variants | `BreathDamage`, `CurseDamage`, `TeleportAway`, `DrainResource`, `Amnesia`, `BlinkSelf`, `TeleportSelf`, `TeleportTarget` | rejected by player `ability_target_plan`; handled only by monster paths | no player transaction support is introduced in Phase 13 |

## Item-use transaction

`Game::use_inventory_item` is the authoritative transaction shell. Its observable order is:

1. resolve an inventory item and its static use action or dynamic activation;
2. build `ItemUsePlan` and reject an invalid target before tried state, charge/stack changes, or RNG;
3. reject insufficient charges;
4. mark the item kind tried;
5. run the optional device check and emit failure without spending a charge or stack;
6. after a successful check, deduct dynamic/static charges or consume the stack and remove instance property knowledge when required;
7. resolve exactly one item effect family, including ordered `Sequence` leaves;
8. mark the kind aware only under that family's observed-outcome rule.

`use_recharging_item` remains a separate targeted source/target transaction because it validates two inventory items and pays the source before resolving target recharge failure.

| Effect family | Variants | Current entry points | Authoritative dependencies that stay in place |
| --- | --- | --- | --- |
| Preflight and settlement | all variants | `inventory_item_use_context`, `inventory_item_use_effect`, `item_use_plan`, `item_use_is_zero_time_unavailable`, `use_inventory_item` | inventory, activation profile, charges, tried/aware, stack and knowledge cleanup |
| Healing/resources | `Heal`, `HealDice`, `RestoreResource`, `RestoreResourceDice`, `RestoreResourceFull` | `resolve_item_self_effect`, `resolve_item_healing`, `resolve_item_resource_restoration` | resource pools, actual-change awareness, dice RNG |
| Vitality/restoration | `RestoreLifeLevels`, `RestoreAllAttributes`, `RestoreAllVitality`, `ApplyRestorativeFeast`, `ApplyLifeRestoration` | `resolve_item_self_effect`, restoration helpers | progression attribute/life-force state and derived refresh |
| Timed positive statuses | `Bless`, `ApplySpeed`, `ApplyHeroism`, `ApplyBerserkStrength`, `ApplyPoeticInspiration`, `ApplyStoneSkin`, `ApplyThermalResistance`, `ApplyBasicResistance`, `Vengeance`, `ProtectionFromEvil`, `PrepareConfusingStrike` | family-specific `resolve_item_*` helpers | duration RNG, concrete stacking, current-status observation, equipment/combat consumers |
| Harmful/self effects | `ApplySlowness`, `ApplyPoison`, `ApplyBlindness`, `ApplyDetonation`, `SelfLifeLoss` | family-specific helpers, Phase 12 `item_combat.rs` for detonation | saving/immunity rules, direct-life-loss bypass, status and damage order |
| Status removal | `RemoveStatus` | `resolve_item_self_effect`, concrete status helper | current status state and outcome-dependent awareness |
| Attributes | `DrainAttribute`, `RestoreAttribute`, `IncreaseAttribute`, `AugmentAttributes` | `resolve_item_drain_attribute`, `resolve_item_restore_attribute`, `resolve_item_increase_attributes` | attribute RNG/order, progression refresh and proportional HP/resources |
| Self-centered damage | `SelfCenteredElementalBlast` | `use_inventory_item`, `item_combat.rs` | visible snapshot, actor damage/death, backlash and resistance semantics |
| Aggravation | `AggravateMonsters` | `resolve_item_aggravation` | actor alert state, stable actor order, awareness |
| Genocide | `MassGenocide`, `Genocide` | `resolve_item_mass_genocide`, `resolve_item_genocide` | explicit removal bypass, glyph/radius gates, fatigue, tried/aware distinctions |
| Terrain | `CreateAdjacentTerrain`, `DestroyAdjacentTrapsAndDoors` | `item_use_plan`, inline commit arms | eight-direction order, occupancy/connections, revealed cleanup, changed cells |
| Projectile damage | `Damage` | `item_effect_path`, inline arm, `item_combat.rs` | target validation, trace, damage/death, awareness and event order |
| Dispel/banishment | `DispelCategory`, `BanishVisible` | `resolve_item_dispel_category` in `item_combat.rs`, `resolve_item_banish_visible` | visible snapshot, resist-all, destination RNG/order, removal policy |
| Detection | `Detect` | inline arm and subject-specific detection helpers | through-wall flag, persistent knowledge versus transient result, awareness |
| Summoning | `SummonCategory` | `item_category_summon_plan`, `resolve_category_summon` | selector/depth/level/kin/unique filters, positions, RNG/IDs, hostility/group |
| Identification | `IdentifyItem` | `item_is_valid_identify_target`, `identify_item_instance` | source/target exclusion, item and affix knowledge, no target RNG |
| Enchantment | `EnchantItem` | `item_is_valid_enchant_target`, `roll_item_enchantment_attempts`, `resolve_item_enchantment_component`, `enchant_item_instance` | component RNG order, artifact/ammunition gates, item state and awareness |
| Curses | `CurseEquippedItem`, `RemoveEquippedCurses` | `curse_equipped_item`, inline removal arm | equipment-slot order, artifact resistance RNG, severity boundaries |
| Recharge | `RechargeFromDevice` | `use_recharging_item`, recharge helpers | two-item validation, source payment timing, target energy/destruction RNG |
| Composition | `Sequence` | `resolve_item_self_effect` | declared order, accumulated noticed flag, per-leaf events and RNG |
| Random teleport | `RandomTeleport` | `item_use_plan`, inline arm | stable candidates, one destination roll, arrival trap semantics |
| Floor transition | `TeleportLevel` | `item_use_plan`, inline arm, Phase 11 `floor.rs` | direction roll, ordered/deduplicated targets, transition commit/events |
| Recall | `Recall`, `ResetRecall` | `item_use_plan`, inline arms, Phase 11 `floor.rs` | consumption/awareness before delay dice, start/cancel/reset and trigger order |

## Monster ability transaction

Monster AI selection and selected-effect execution are deliberately separated:

```text
resolve_monster_ability_with_changes
  -> frequency roll
  -> monster_ability_plan / monster_targeted_ability_plan
  -> utility and resistance weighting
  -> weighted selection roll
  -> resolve_monster_ability_plan
  -> selected effect-family executor
```

The frequency roll, plan/candidate construction, clean-shot and friendly-risk checks, utility, observed-resistance weighting, weighted selection, cooldown assignment, scheduling, and `MonsterAbilityDecision` stay with the AI transaction owner. Phase 13 may move only `resolve_monster_ability_plan` and its selected-effect helpers.

| Effect family | Variants | Current execution entry points | Required preservation |
| --- | --- | --- | --- |
| Self healing/status | `Heal`, `ApplyStatus`, `RemoveStatus`, self `Sequence` | `resolve_monster_self_effects` | utility remains outside; duration RNG and effect order remain inside execution |
| Direct hostile effects | `Damage`, `CurseDamage`, `DrainResource`, `Amnesia`, `ApplyStatus`, `RemoveStatus`, hostile `Sequence` | `resolve_monster_hostile_effects`, `resolve_monster_player_effects` | player versus summon policy, resistance observation, target-death skip |
| Area/beam/cone/breath | `AreaDamage`, `BeamDamage`, `ConeDamage`, `BreathDamage` | `resolve_monster_ability_plan` | one shared roll or HP calculation, footprint order, player-aligned target cleanup |
| Fixed/category summon | `Summon`, `SummonCategory` | `resolve_monster_ability_plan`, `resolve_category_summon` | hostile ownership, active energy, candidate/group RNG, identity/allocation order |
| Self displacement | `BlinkSelf`, `TeleportSelf` | `resolve_monster_ability_plan` | preplanned candidate order, destination roll, changed cells |
| Target displacement | `TeleportAway`, `TeleportTarget` | `resolve_monster_ability_plan` | player/summon distinction, destination order/RNG, trace and DTO contents |

Player-only effects remain unavailable to the monster planner. Phase 13 does not broaden the monster effect vocabulary.

## Shared concrete helpers and bypasses

| Boundary | Current owner | Phase 13 rule |
| --- | --- | --- |
| Armor/resistance arithmetic | crate-level `combat.rs`, `effect.rs`, `resistance.rs` | reuse without redesign |
| Aggregate HP/status damage | `game/damage.rs` | reuse explicit fatality policies |
| Ordinary actor death | `game/death.rs` | reuse; never absorb genocide/removal-only paths |
| Player/item/monster attack adapters | `player_combat.rs`, `item_combat.rs`, `monster_combat.rs` | effect families call these stable adapters |
| Concrete status application/removal | `apply_ability_status_effect`, `remove_ability_status_effect` in `game/mod.rs` | may move together to `status_effects.rs`; no universal effect context |
| Category summon execution | `resolve_category_summon` in `game/mod.rs` | keep RNG, ID allocation, actor construction, placement, and event semantics explicit |
| Inventory and knowledge | `game/inventory.rs` plus item helpers in `game/mod.rs` | inventory remains authoritative; item effects do not own a parallel model |
| Terrain interactions | `game/terrain.rs` | ability/item terrain transforms keep their distinct tested semantics |
| Floor transitions and recall | `game/floor.rs` | item effects invoke Phase 11 adapters at the original commit point |
| Progression/resources | `game/progression.rs` and aggregate resource state | effect owners request concrete refresh/plan operations only |
| Task/campaign reduction | `game/tasks.rs` | remains downstream of ordered domain events |

Explicit bypasses remain explicit: genocide removes actors without ordinary loot/corpses/rewards, player genocide fatigue and item `SelfLifeLoss` bypass ordinary incoming-damage reduction, and defeated player summons use removal-only semantics.

## Characterization evidence

The existing focused tests already lock the Phase 13 transaction and effect-family distinctions. The primary guards are:

- player transaction and targeting: `area_damage_respects_walls_and_invalid_targets_are_zero_rng`, `beam_self_target_is_zero_rng_and_empty_beam_still_rolls_once`, `cone_invalid_mode_is_zero_rng_and_empty_cone_still_rolls_once`, `summon_space_rejection_is_atomic_before_mana_and_rng`, `actor_effect_sequences_preserve_empty_invalid_and_failure_rng_boundaries`, `failed_cast_costs_mana_but_insufficient_mana_does_not_draw_rng`, and `technique_casts_consume_tempo_and_reject_shortfalls_without_rng`;
- player ordered/special effects: `target_status_sequence_resists_immunizes_and_skips_after_death`, `invoke_spirits_records_deterministic_random_no_op_branches`, `vampirism_true_retraces_the_path_after_each_kill`, `genocide_erases_without_rewards_or_corpses_and_uniques_resist`, `ordinary_death_creates_a_corpse_and_animate_dead_consumes_it_persistently`, and `control_resists_ineligible_targets_and_turns_pack_leaders_into_allies`;
- item transaction and knowledge: `device_skill_check_distinguishes_builds_without_consuming_on_failure`, `charged_device_spends_instance_charges_only_after_a_successful_check_and_round_trips`, `identify_scroll_rejects_missing_and_self_targets_before_consumption`, `enchantment_scroll_rejects_invalid_targets_atomically`, `dynamic_wand_validates_target_before_check_and_spends_only_on_success`, and `recharging_item_rejects_invalid_pairs_and_pays_the_source_before_failure`;
- item outcome semantics: `restorative_item_sequence_recovers_resource_then_removes_status`, `missing_player_resource_consumes_restorative_without_claiming_awareness`, `curse_scroll_without_a_matching_equipped_item_consumes_without_rng_or_awareness`, `item_summon_zero_candidate_and_zero_space_consume_without_awareness_or_rng`, `visible_actor_scrolls_consume_empty_results_without_rng_or_awareness`, and the teleport-level/recall tests;
- monster selection versus execution: `monster_casting_uses_frequency_viability_and_weighted_selection`, `monster_casting_utility_uses_wounds_status_and_distance_without_rng`, `monster_multi_target_plans_reject_secondary_entities`, `monster_casting_cooldown_uses_inverse_frequency_without_rng`, `monster_area_damage_hits_every_player_aligned_target_and_removes_slain_summons`, `smart_caster_learns_only_observed_player_resistance_and_round_trips`, and `lethal_monster_sequence_skips_later_status_without_extra_rng`.

No missing pre-movement behavior required a new test in the characterization gate: the existing suite directly asserts every transaction-order distinction identified by the call-site census. Each later family commit must name and run its applicable tests from this matrix, plus replay and contract fixtures, before the next family moves.
