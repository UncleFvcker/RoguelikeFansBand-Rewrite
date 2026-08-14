// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    AbilityDetectSubjectDefinition, AbilityTargetDefinition, AbilityTargetModeDefinition,
    ContentError, EquipmentBonuses, ITEM_SCHEMA, ItemDefinition, ItemEnchantmentRollDefinition,
    ItemFuelKindDefinition, ItemMountUseDefinition, ItemSummonSelectorDefinition,
    ItemUseEffectDefinition, StatModifiers,
};

use super::shared::{
    attribute_modifiers_out_of_range, equipment_bonuses_out_of_range, insert_definition_id,
    normalize_tags, require_format_version, require_reference, require_schema,
    validate_definition_id, validate_definition_text, validate_equipment_slot, validate_glyph,
    validate_id, validate_message_key, validate_status_immunities,
};

pub(crate) fn valid_item_effect(
    effect: &ItemUseEffectDefinition,
    terrain_tags: &BTreeMap<String, BTreeSet<String>>,
    actor_tag_values: &BTreeSet<String>,
    item_tag_values: &BTreeSet<String>,
    resource_ids: &BTreeSet<String>,
    affix_ids: &BTreeSet<String>,
    loot_table_ids: &BTreeSet<String>,
) -> bool {
    match effect {
        ItemUseEffectDefinition::NoNumericEffect => true,
        ItemUseEffectDefinition::IncreaseNutrition { amount } => (1..=15_000).contains(amount),
        ItemUseEffectDefinition::SatisfyHunger => true,
        ItemUseEffectDefinition::Heal { amount }
        | ItemUseEffectDefinition::SelfLifeLoss { amount } => (1..=1_000_000).contains(amount),
        ItemUseEffectDefinition::SelfDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
        } => {
            (1..=100).contains(damage_dice)
                && (1..=10_000).contains(damage_sides)
                && *damage_bonus <= 10_000
        }
        ItemUseEffectDefinition::ApplyDetonation {
            damage_dice,
            damage_sides,
            stun_ticks,
            bleeding_ticks,
        } => {
            (1..=100).contains(damage_dice)
                && (1..=10_000).contains(damage_sides)
                && *stun_ticks > 0
                && *bleeding_ticks > 0
        }
        ItemUseEffectDefinition::RestoreLifeLevels { life_force_amount } => {
            (1..=1_000).contains(life_force_amount)
        }
        ItemUseEffectDefinition::RestoreAllVitality { life_force_amount } => {
            (1..=1_000).contains(life_force_amount)
        }
        ItemUseEffectDefinition::ApplyRestorativeFeast {
            healing_dice,
            healing_sides,
        } => (1..=100).contains(healing_dice) && (1..=10_000).contains(healing_sides),
        ItemUseEffectDefinition::ApplyElvishWaybread {
            nutrition,
            healing_dice,
            healing_sides,
        } => {
            *nutrition > 0
                && (1..=100).contains(healing_dice)
                && (1..=10_000).contains(healing_sides)
        }
        ItemUseEffectDefinition::ApplySaltWater | ItemUseEffectDefinition::ApplyFastRecovery => {
            true
        }
        ItemUseEffectDefinition::ApplyLifeRestoration {
            healing_amount,
            life_force_amount,
        } => (1..=1_000_000).contains(healing_amount) && (1..=1_000).contains(life_force_amount),
        ItemUseEffectDefinition::RestoreAllAttributes
        | ItemUseEffectDefinition::ApplyBooze
        | ItemUseEffectDefinition::DrainAttribute { .. }
        | ItemUseEffectDefinition::RestoreAttribute { .. }
        | ItemUseEffectDefinition::IncreaseAttribute { .. }
        | ItemUseEffectDefinition::AugmentAttributes
        | ItemUseEffectDefinition::NewLife
        | ItemUseEffectDefinition::PolymorphMutations
        | ItemUseEffectDefinition::IdentifyInventory
        | ItemUseEffectDefinition::SelfKnowledge
        | ItemUseEffectDefinition::TriggerTsuyoshiCrash
        | ItemUseEffectDefinition::MundanifyItem => true,
        ItemUseEffectDefinition::HealDice { dice, sides } => {
            (1..=100).contains(dice) && (1..=10_000).contains(sides)
        }
        ItemUseEffectDefinition::Bless {
            duration_dice,
            duration_sides,
            duration_bonus,
        }
        | ItemUseEffectDefinition::ApplySlowness {
            duration_dice,
            duration_sides,
            duration_bonus,
        }
        | ItemUseEffectDefinition::ApplySpeed {
            duration_dice,
            duration_sides,
            duration_bonus,
        }
        | ItemUseEffectDefinition::ApplyHeroism {
            duration_dice,
            duration_sides,
            duration_bonus,
        }
        | ItemUseEffectDefinition::ApplyBerserkStrength {
            duration_dice,
            duration_sides,
            duration_bonus,
        }
        | ItemUseEffectDefinition::ApplyPoeticInspiration {
            duration_dice,
            duration_sides,
            duration_bonus,
        }
        | ItemUseEffectDefinition::ApplyStoneSkin {
            duration_dice,
            duration_sides,
            duration_bonus,
        }
        | ItemUseEffectDefinition::ApplyThermalResistance {
            duration_dice,
            duration_sides,
            duration_bonus,
        }
        | ItemUseEffectDefinition::ApplyBasicResistance {
            duration_dice,
            duration_sides,
            duration_bonus,
        }
        | ItemUseEffectDefinition::ApplyPoison {
            duration_dice,
            duration_sides,
            duration_bonus,
        }
        | ItemUseEffectDefinition::ApplyBlindness {
            duration_dice,
            duration_sides,
            duration_bonus,
        }
        | ItemUseEffectDefinition::ApplyTsuyoshi {
            duration_dice,
            duration_sides,
            duration_bonus,
        }
        | ItemUseEffectDefinition::Vengeance {
            duration_dice,
            duration_sides,
            duration_bonus,
        } => {
            (1..=100).contains(duration_dice)
                && (1..=10_000).contains(duration_sides)
                && *duration_bonus <= 1_000_000
        }
        ItemUseEffectDefinition::ApplyStatus {
            status_kind_id,
            duration_dice,
            duration_sides,
            duration_bonus,
            granted_resistances,
            granted_modifiers,
            granted_equipment_bonuses,
            incoming_damage_percent,
            ..
        } => {
            validate_id(status_kind_id).is_ok()
                && (1..=100).contains(duration_dice)
                && (1..=10_000).contains(duration_sides)
                && *duration_bonus <= 1_000_000
                && granted_resistances.len() <= 29
                && granted_modifiers.max_hp.abs() <= 1_000_000
                && granted_modifiers.attack.abs() <= 1_000_000
                && granted_modifiers.defense.abs() <= 1_000_000
                && (-100..=100).contains(&granted_modifiers.speed)
                && !attribute_modifiers_out_of_range(granted_modifiers)
                && !equipment_bonuses_out_of_range(granted_equipment_bonuses)
                && *incoming_damage_percent <= 100
        }
        ItemUseEffectDefinition::ApplyGiantStrength {
            duration_dice,
            duration_sides,
            duration_bonus,
        } => {
            (1..=100).contains(duration_dice)
                && (1..=10_000).contains(duration_sides)
                && *duration_bonus <= 1_000_000
        }
        ItemUseEffectDefinition::SelfCenteredElementalBlast {
            base_damage,
            radius,
            backlash_sides,
            backlash_bonus,
            ..
        } => {
            (1..=1_000_000).contains(base_damage)
                && (1..=8).contains(radius)
                && (1..=10_000).contains(backlash_sides)
                && *backlash_bonus <= 10_000
        }
        ItemUseEffectDefinition::ProtectionFromEvil
        | ItemUseEffectDefinition::PrepareConfusingStrike
        | ItemUseEffectDefinition::AggravateMonsters
        | ItemUseEffectDefinition::IncreaseSpellLearningCapacity
        | ItemUseEffectDefinition::DestroyAdjacentTrapsAndDoors => true,
        ItemUseEffectDefinition::MassGenocide { power, radius } => *power > 0 && *radius > 0,
        ItemUseEffectDefinition::Genocide { power } => (1..=1_000).contains(power),
        ItemUseEffectDefinition::RechargeFromDevice { power } => (1..=1_000).contains(power),
        ItemUseEffectDefinition::CreateAdjacentTerrain {
            source_terrain_ids,
            target_terrain_id,
        }
        | ItemUseEffectDefinition::CreateCurrentTerrain {
            source_terrain_ids,
            target_terrain_id,
        } => {
            !source_terrain_ids.is_empty()
                && source_terrain_ids.len() <= 32
                && source_terrain_ids.windows(2).all(|pair| pair[0] != pair[1])
                && source_terrain_ids
                    .iter()
                    .all(|source_id| terrain_tags.contains_key(source_id))
                && terrain_tags.contains_key(target_terrain_id)
                && source_terrain_ids
                    .iter()
                    .all(|source_id| source_id != target_terrain_id)
        }
        ItemUseEffectDefinition::SetFloorGlow { radius, .. } => {
            (1..=32).contains(radius) || *radius == u8::MAX
        }
        ItemUseEffectDefinition::AreaDestruction {
            minimum_radius,
            maximum_radius,
            floor_terrain_id,
            wall_terrain_id,
            quartz_terrain_id,
            magma_terrain_id,
        } => {
            (1..=32).contains(minimum_radius)
                && minimum_radius <= maximum_radius
                && maximum_radius <= &32
                && [
                    floor_terrain_id,
                    wall_terrain_id,
                    quartz_terrain_id,
                    magma_terrain_id,
                ]
                .into_iter()
                .all(|terrain_id| terrain_tags.contains_key(terrain_id))
        }
        ItemUseEffectDefinition::RemoveStatus { status_kind_id } => {
            validate_id(status_kind_id).is_ok()
        }
        ItemUseEffectDefinition::ReduceStatus {
            status_kind_id,
            minimum_reduction,
            reduction_divisor,
        } => {
            validate_id(status_kind_id).is_ok()
                && (1..=1_000_000).contains(minimum_reduction)
                && (1..=100).contains(reduction_divisor)
        }
        ItemUseEffectDefinition::LoseExperienceFraction { divisor } => (2..=100).contains(divisor),
        ItemUseEffectDefinition::GainRelativeExperience {
            divisor,
            bonus,
            maximum_gain,
        } => {
            (2..=100).contains(divisor)
                && (1..=1_000_000).contains(bonus)
                && bonus <= maximum_gain
                && maximum_gain <= &1_000_000
        }
        ItemUseEffectDefinition::RestoreResource {
            resource_id,
            amount,
        } => resource_ids.contains(resource_id) && (1..=1_000_000).contains(amount),
        ItemUseEffectDefinition::RestoreResourceDice {
            resource_id,
            dice,
            sides,
            bonus,
        } => {
            resource_ids.contains(resource_id)
                && (1..=100).contains(dice)
                && (1..=10_000).contains(sides)
                && *bonus <= 1_000_000
        }
        ItemUseEffectDefinition::RestoreResourceFull { resource_id } => {
            resource_ids.contains(resource_id)
        }
        ItemUseEffectDefinition::DrainResourceFull { resource_id } => {
            resource_ids.contains(resource_id)
        }
        ItemUseEffectDefinition::IdentifyItem { .. } => true,
        ItemUseEffectDefinition::Acquirement {
            loot_table_id,
            minimum_count,
            maximum_count,
        } => {
            loot_table_ids.contains(loot_table_id)
                && (1..=8).contains(minimum_count)
                && minimum_count <= maximum_count
                && maximum_count <= &8
        }
        ItemUseEffectDefinition::CraftItem {
            weapon_affix_ids,
            armor_affix_ids,
        } => {
            let valid_candidates = |candidates: &[String]| {
                !candidates.is_empty()
                    && candidates.len() <= 32
                    && candidates.windows(2).all(|pair| pair[0] < pair[1])
                    && candidates.iter().all(|id| affix_ids.contains(id))
            };
            valid_candidates(weapon_affix_ids) && valid_candidates(armor_affix_ids)
        }
        ItemUseEffectDefinition::ShowRumour { message_key } => {
            validate_message_key(message_key).is_ok()
        }
        ItemUseEffectDefinition::EnchantItem {
            to_hit,
            to_damage,
            to_armor,
        } => {
            let valid_roll = |roll: &ItemEnchantmentRollDefinition| {
                (roll.dice == 0 && roll.sides == 0 && (1..=100).contains(&roll.bonus))
                    || ((1..=10).contains(&roll.dice)
                        && (1..=100).contains(&roll.sides)
                        && roll.bonus <= 100)
            };
            let weapon_rolls = to_hit.iter().chain(to_damage).count();
            let armor_rolls = usize::from(to_armor.is_some());
            ((weapon_rolls > 0 && armor_rolls == 0) || (weapon_rolls == 0 && armor_rolls == 1))
                && to_hit
                    .iter()
                    .chain(to_damage)
                    .chain(to_armor)
                    .all(valid_roll)
        }
        ItemUseEffectDefinition::CurseEquippedItem { .. }
        | ItemUseEffectDefinition::RemoveEquippedCurses { .. } => true,
        ItemUseEffectDefinition::SummonCategory {
            selector,
            count_dice,
            count_sides,
            count_bonus,
            hostile,
            group_chance_percent,
            group_count_dice,
            group_count_sides,
            group_count_bonus,
            allow_unique,
            radius,
            duration_turns,
            ..
        } => {
            let selector_is_valid = match selector {
                ItemSummonSelectorDefinition::AnyMonster
                | ItemSummonSelectorDefinition::PlayerKin => true,
                ItemSummonSelectorDefinition::Category { category } => {
                    actor_tag_values.contains(category)
                }
            };
            selector_is_valid
                && (1..=8).contains(count_dice)
                && (1..=8).contains(count_sides)
                && u16::from(*count_dice) * u16::from(*count_sides) + u16::from(*count_bonus) <= 8
                && *group_chance_percent <= 100
                && if *group_chance_percent == 0 {
                    *group_count_dice == 0 && *group_count_sides == 0 && *group_count_bonus == 0
                } else {
                    (1..=8).contains(group_count_dice)
                        && (1..=8).contains(group_count_sides)
                        && u16::from(*group_count_dice) * u16::from(*group_count_sides)
                            + u16::from(*group_count_bonus)
                            <= 8
                }
                && (!*allow_unique || *hostile)
                && (1..=8).contains(radius)
                && *duration_turns == 0
        }
        ItemUseEffectDefinition::Sequence { effects } => {
            (2..=12).contains(&effects.len())
                && effects.iter().all(|effect| {
                    matches!(
                        effect,
                        ItemUseEffectDefinition::NoNumericEffect
                            | ItemUseEffectDefinition::IncreaseNutrition { .. }
                            | ItemUseEffectDefinition::SatisfyHunger
                            | ItemUseEffectDefinition::Heal { .. }
                            | ItemUseEffectDefinition::HealDice { .. }
                            | ItemUseEffectDefinition::ApplyFastRecovery
                            | ItemUseEffectDefinition::ApplyPoison { .. }
                            | ItemUseEffectDefinition::ApplyBlindness { .. }
                            | ItemUseEffectDefinition::ApplyStatus { .. }
                            | ItemUseEffectDefinition::ApplyGiantStrength { .. }
                            | ItemUseEffectDefinition::SelfDamage { .. }
                            | ItemUseEffectDefinition::LoseExperienceFraction { .. }
                            | ItemUseEffectDefinition::GainRelativeExperience { .. }
                            | ItemUseEffectDefinition::ApplyTsuyoshi { .. }
                            | ItemUseEffectDefinition::TriggerTsuyoshiCrash
                            | ItemUseEffectDefinition::DrainAttribute { .. }
                            | ItemUseEffectDefinition::RestoreAttribute { .. }
                            | ItemUseEffectDefinition::IncreaseAttribute { .. }
                            | ItemUseEffectDefinition::RemoveStatus { .. }
                            | ItemUseEffectDefinition::ReduceStatus { .. }
                            | ItemUseEffectDefinition::RestoreResource { .. }
                            | ItemUseEffectDefinition::RestoreResourceDice { .. }
                            | ItemUseEffectDefinition::RestoreResourceFull { .. }
                            | ItemUseEffectDefinition::DrainResourceFull { .. }
                            | ItemUseEffectDefinition::IdentifyInventory
                            | ItemUseEffectDefinition::SelfKnowledge
                            | ItemUseEffectDefinition::Detect { .. }
                            | ItemUseEffectDefinition::SetFloorGlow { .. }
                    ) && valid_item_effect(
                        effect,
                        terrain_tags,
                        actor_tag_values,
                        item_tag_values,
                        resource_ids,
                        affix_ids,
                        loot_table_ids,
                    )
                })
        }
        ItemUseEffectDefinition::Damage {
            damage_dice,
            damage_sides,
            damage_bonus,
            ..
        }
        | ItemUseEffectDefinition::BeamDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            ..
        } => {
            ((*damage_dice == 0 && *damage_sides == 0 && *damage_bonus > 0)
                || ((1..=100).contains(damage_dice) && (1..=10_000).contains(damage_sides)))
                && *damage_bonus <= 10_000
        }
        ItemUseEffectDefinition::RandomElementConeDamage {
            damage,
            damage_types,
            radius,
        } => {
            (1..=1_000_000).contains(damage)
                && (2..=16).contains(&damage_types.len())
                && damage_types.iter().copied().collect::<BTreeSet<_>>().len() == damage_types.len()
                && (1..=8).contains(radius)
        }
        ItemUseEffectDefinition::DispelCategory { category, damage } => {
            actor_tag_values.contains(category) && (1..=1_000_000).contains(damage)
        }
        ItemUseEffectDefinition::BanishVisible { maximum_distance } => {
            (1..=200).contains(maximum_distance)
        }
        ItemUseEffectDefinition::Detect {
            subject,
            category,
            radius,
            persistent,
            through_walls,
        } => {
            !category.is_empty()
                && category.len() <= 64
                && category.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'-' | b'_')
                })
                && (*radius > 0 && (*radius <= 30 || (*radius == u8::MAX && *through_walls)))
                && match subject {
                    AbilityDetectSubjectDefinition::Terrain => {
                        if category == "map" {
                            *persistent && *through_walls
                        } else {
                            terrain_tags.values().any(|tags| tags.contains(category))
                        }
                    }
                    AbilityDetectSubjectDefinition::Actor => {
                        !persistent
                            && (category == "any-monster" || actor_tag_values.contains(category))
                    }
                    AbilityDetectSubjectDefinition::Item => {
                        !persistent && (category == "item" || item_tag_values.contains(category))
                    }
                    AbilityDetectSubjectDefinition::Gold => !persistent && category == "gold",
                    AbilityDetectSubjectDefinition::Curse => !persistent && category == "curse",
                }
        }
        ItemUseEffectDefinition::RandomTeleport { maximum_distance } => {
            (1..=200).contains(maximum_distance)
        }
        ItemUseEffectDefinition::TeleportLevel | ItemUseEffectDefinition::ResetRecall => true,
        ItemUseEffectDefinition::Recall {
            delay_dice,
            delay_sides,
            delay_bonus,
        } => {
            (1..=10).contains(delay_dice)
                && (1..=100).contains(delay_sides)
                && *delay_bonus <= 1_000
        }
    }
}

fn item_effect_is_self_targeted(effect: &ItemUseEffectDefinition) -> bool {
    match effect {
        ItemUseEffectDefinition::Damage { .. }
        | ItemUseEffectDefinition::BeamDamage { .. }
        | ItemUseEffectDefinition::RandomElementConeDamage { .. }
        | ItemUseEffectDefinition::IdentifyItem { .. }
        | ItemUseEffectDefinition::EnchantItem { .. }
        | ItemUseEffectDefinition::MundanifyItem
        | ItemUseEffectDefinition::CraftItem { .. } => false,
        ItemUseEffectDefinition::Sequence { effects } => {
            effects.iter().all(item_effect_is_self_targeted)
        }
        _ => true,
    }
}

pub(super) struct ItemValidationRefs<'a> {
    pub(super) terrain_tags: &'a BTreeMap<String, BTreeSet<String>>,
    pub(super) actor_tag_values: &'a BTreeSet<String>,
    pub(super) item_tag_values: &'a BTreeSet<String>,
    pub(super) resource_ids: &'a BTreeSet<String>,
    pub(super) affix_ids: &'a BTreeSet<String>,
    pub(super) loot_table_ids: &'a BTreeSet<String>,
    pub(super) ability_book_ids: &'a BTreeSet<String>,
    pub(super) actor_corpse_item_ids: Vec<(String, String)>,
    pub(super) ability_corpse_item_ids: Vec<(String, String)>,
    pub(super) ability_created_item_ids: Vec<(String, String)>,
    pub(super) ability_plain_created_items: Vec<(String, String, u32)>,
}

pub(super) fn validate_items(
    items: &mut [ItemDefinition],
    refs: ItemValidationRefs<'_>,
    all_ids: &mut BTreeSet<String>,
) -> Result<BTreeMap<String, (u32, bool)>, ContentError> {
    let ItemValidationRefs {
        terrain_tags,
        actor_tag_values,
        item_tag_values,
        resource_ids,
        affix_ids,
        loot_table_ids,
        ability_book_ids,
        actor_corpse_item_ids,
        ability_corpse_item_ids,
        ability_created_item_ids,
        ability_plain_created_items,
    } = refs;
    let valid_item_effect_target =
        |effect: &ItemUseEffectDefinition, target: &AbilityTargetDefinition| {
            let mut modes = BTreeSet::new();
            let modes_are_unique =
                target.modes.iter().all(|mode| modes.insert(*mode)) && !target.modes.is_empty();
            let self_target = target.modes.as_slice() == [AbilityTargetModeDefinition::SelfTarget]
                && target.range == 0
                && !target.requires_line_of_effect;
            let projectile_target = !target
                .modes
                .contains(&AbilityTargetModeDefinition::SelfTarget)
                && target.modes.iter().all(|mode| {
                    matches!(
                        mode,
                        AbilityTargetModeDefinition::Direction
                            | AbilityTargetModeDefinition::Position
                            | AbilityTargetModeDefinition::Entity
                    )
                })
                && (1..=64).contains(&target.range)
                && target.requires_line_of_effect;
            modes_are_unique
                && match effect {
                    ItemUseEffectDefinition::IncreaseNutrition { .. }
                    | ItemUseEffectDefinition::SatisfyHunger
                    | ItemUseEffectDefinition::Heal { .. }
                    | ItemUseEffectDefinition::HealDice { .. }
                    | ItemUseEffectDefinition::Bless { .. }
                    | ItemUseEffectDefinition::ApplySlowness { .. }
                    | ItemUseEffectDefinition::ApplySpeed { .. }
                    | ItemUseEffectDefinition::ApplyHeroism { .. }
                    | ItemUseEffectDefinition::ApplyBerserkStrength { .. }
                    | ItemUseEffectDefinition::ApplyPoeticInspiration { .. }
                    | ItemUseEffectDefinition::ApplyStoneSkin { .. }
                    | ItemUseEffectDefinition::RestoreLifeLevels { .. }
                    | ItemUseEffectDefinition::RestoreAllAttributes
                    | ItemUseEffectDefinition::RestoreAllVitality { .. }
                    | ItemUseEffectDefinition::ApplyRestorativeFeast { .. }
                    | ItemUseEffectDefinition::ApplyElvishWaybread { .. }
                    | ItemUseEffectDefinition::ApplySaltWater
                    | ItemUseEffectDefinition::ApplyBooze
                    | ItemUseEffectDefinition::ApplyFastRecovery
                    | ItemUseEffectDefinition::ApplyLifeRestoration { .. }
                    | ItemUseEffectDefinition::DrainAttribute { .. }
                    | ItemUseEffectDefinition::RestoreAttribute { .. }
                    | ItemUseEffectDefinition::IncreaseAttribute { .. }
                    | ItemUseEffectDefinition::AugmentAttributes
                    | ItemUseEffectDefinition::NewLife
                    | ItemUseEffectDefinition::PolymorphMutations
                    | ItemUseEffectDefinition::IdentifyInventory
                    | ItemUseEffectDefinition::SelfKnowledge
                    | ItemUseEffectDefinition::Acquirement { .. }
                    | ItemUseEffectDefinition::ShowRumour { .. }
                    | ItemUseEffectDefinition::ApplyThermalResistance { .. }
                    | ItemUseEffectDefinition::ApplyBasicResistance { .. }
                    | ItemUseEffectDefinition::ApplyPoison { .. }
                    | ItemUseEffectDefinition::ApplyBlindness { .. }
                    | ItemUseEffectDefinition::ApplyStatus { .. }
                    | ItemUseEffectDefinition::ApplyGiantStrength { .. }
                    | ItemUseEffectDefinition::ApplyDetonation { .. }
                    | ItemUseEffectDefinition::SelfLifeLoss { .. }
                    | ItemUseEffectDefinition::SelfDamage { .. }
                    | ItemUseEffectDefinition::LoseExperienceFraction { .. }
                    | ItemUseEffectDefinition::GainRelativeExperience { .. }
                    | ItemUseEffectDefinition::ApplyTsuyoshi { .. }
                    | ItemUseEffectDefinition::TriggerTsuyoshiCrash
                    | ItemUseEffectDefinition::Vengeance { .. }
                    | ItemUseEffectDefinition::ProtectionFromEvil
                    | ItemUseEffectDefinition::PrepareConfusingStrike
                    | ItemUseEffectDefinition::SelfCenteredElementalBlast { .. }
                    | ItemUseEffectDefinition::AggravateMonsters
                    | ItemUseEffectDefinition::MassGenocide { .. }
                    | ItemUseEffectDefinition::Genocide { .. }
                    | ItemUseEffectDefinition::IncreaseSpellLearningCapacity
                    | ItemUseEffectDefinition::CreateAdjacentTerrain { .. }
                    | ItemUseEffectDefinition::CreateCurrentTerrain { .. }
                    | ItemUseEffectDefinition::SetFloorGlow { .. }
                    | ItemUseEffectDefinition::AreaDestruction { .. }
                    | ItemUseEffectDefinition::DestroyAdjacentTrapsAndDoors
                    | ItemUseEffectDefinition::RemoveStatus { .. }
                    | ItemUseEffectDefinition::ReduceStatus { .. }
                    | ItemUseEffectDefinition::RestoreResource { .. }
                    | ItemUseEffectDefinition::RestoreResourceDice { .. }
                    | ItemUseEffectDefinition::RestoreResourceFull { .. }
                    | ItemUseEffectDefinition::DrainResourceFull { .. }
                    | ItemUseEffectDefinition::Sequence { .. }
                    | ItemUseEffectDefinition::NoNumericEffect
                    | ItemUseEffectDefinition::Detect { .. }
                    | ItemUseEffectDefinition::RandomTeleport { .. }
                    | ItemUseEffectDefinition::TeleportLevel
                    | ItemUseEffectDefinition::Recall { .. }
                    | ItemUseEffectDefinition::ResetRecall
                    | ItemUseEffectDefinition::CurseEquippedItem { .. }
                    | ItemUseEffectDefinition::RemoveEquippedCurses { .. }
                    | ItemUseEffectDefinition::SummonCategory { .. }
                    | ItemUseEffectDefinition::DispelCategory { .. }
                    | ItemUseEffectDefinition::BanishVisible { .. } => self_target,
                    ItemUseEffectDefinition::RechargeFromDevice { .. } => false,
                    ItemUseEffectDefinition::Damage { .. }
                    | ItemUseEffectDefinition::BeamDamage { .. } => projectile_target,
                    ItemUseEffectDefinition::RandomElementConeDamage { .. } => {
                        target.modes.as_slice() == [AbilityTargetModeDefinition::Direction]
                            && projectile_target
                    }
                    ItemUseEffectDefinition::IdentifyItem { .. }
                    | ItemUseEffectDefinition::EnchantItem { .. }
                    | ItemUseEffectDefinition::MundanifyItem
                    | ItemUseEffectDefinition::CraftItem { .. } => {
                        target.modes.as_slice() == [AbilityTargetModeDefinition::Item]
                            && target.range == 0
                            && !target.requires_line_of_effect
                    }
                }
        };

    let proficiency_items = items
        .iter()
        .map(|item| {
            (
                item.id.clone(),
                (
                    item.weapon_proficiency_base_item_id.clone(),
                    item.melee_profile.is_some(),
                    item.projectile_profile.is_some(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let artifact_items = items
        .iter()
        .map(|item| {
            (
                item.id.clone(),
                (
                    item.tags.iter().any(|tag| tag == "artifact"),
                    item.equipment_slot.clone(),
                    item.melee_profile.is_some(),
                    item.projectile_profile.is_some(),
                    item.ammunition_profile.is_some(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut artifact_source_indices = BTreeSet::new();
    let mut item_limits = BTreeMap::new();
    for item in items.iter_mut() {
        require_schema(&item.schema, ITEM_SCHEMA, &item.id)?;
        require_format_version(item.format_version, &item.id)?;
        validate_definition_id(&item.id, "item")?;
        validate_definition_text(&item.id, &item.name_key, &item.description_key)?;
        if let Some(appearance_name_key) = &item.appearance_name_key {
            validate_message_key(appearance_name_key)?;
            if appearance_name_key == &item.name_key {
                return Err(ContentError::InvalidItemAppearance(item.id.clone()));
            }
        }
        validate_glyph(&item.id, &item.glyph)?;
        if item.weight_tenths_pound == 0 || item.weight_tenths_pound > 10_000 {
            return Err(ContentError::InvalidItemWeight(item.id.clone()));
        }
        if !(-100..=100).contains(&item.tunneling_pval)
            || (item.tunneling_pval != 0
                && !matches!(item.equipment_slot.as_deref(), Some("weapon" | "tool")))
        {
            return Err(ContentError::InvalidItemModifiers(item.id.clone()));
        }
        if item.max_stack == 0 || item.max_stack > 1_000_000 {
            return Err(ContentError::InvalidItemStack(item.id.clone()));
        }
        if item.base_value > 999_999_999 {
            return Err(ContentError::InvalidItemValue(item.id.clone()));
        }
        if item.break_chance_percent > 100 {
            return Err(ContentError::InvalidItemBreakChance(item.id.clone()));
        }
        if item.riding_weapon_kind.is_some() && item.melee_profile.is_none() {
            return Err(ContentError::InvalidAttackProfile(item.id.clone()));
        }
        if let Some(fuel) = item.fuel {
            let valid = fuel.maximum > 0
                && fuel.initial <= fuel.maximum
                && match fuel.kind {
                    ItemFuelKindDefinition::Torch => {
                        item.equipment_slot.as_deref() == Some("light")
                            && item.max_stack == 1
                            && fuel.light_radius == 1
                    }
                    ItemFuelKindDefinition::Lantern => {
                        item.equipment_slot.as_deref() == Some("light")
                            && item.max_stack == 1
                            && fuel.light_radius == 2
                    }
                    ItemFuelKindDefinition::Oil => {
                        item.equipment_slot.is_none()
                            && fuel.light_radius == 0
                            && fuel.initial == fuel.maximum
                    }
                };
            if !valid {
                return Err(ContentError::InvalidItemFuel(item.id.clone()));
            }
        }
        if let Some(slot) = &item.equipment_slot
            && (item.max_stack != 1 || validate_equipment_slot(slot).is_err())
        {
            return Err(ContentError::InvalidEquipmentSlot(item.id.clone()));
        }
        if let Some(base_item_id) = &item.weapon_proficiency_base_item_id {
            let Some((base_alias, base_melee, base_projectile)) =
                proficiency_items.get(base_item_id)
            else {
                return Err(ContentError::InvalidWeaponProficiency(item.id.clone()));
            };
            if base_item_id == &item.id
                || base_alias.is_some()
                || (item.melee_profile.is_none() && item.projectile_profile.is_none())
                || item.melee_profile.is_some() != *base_melee
                || item.projectile_profile.is_some() != *base_projectile
            {
                return Err(ContentError::InvalidWeaponProficiency(item.id.clone()));
            }
        }
        if let Some(generation) = &item.artifact_generation {
            let Some((base_is_artifact, base_slot, base_melee, base_projectile, base_ammunition)) =
                artifact_items.get(&generation.base_item_kind_id)
            else {
                return Err(ContentError::InvalidArtifactGeneration(item.id.clone()));
            };
            if generation.source_index == 0
                || generation.rarity_one_in == 0
                || !artifact_source_indices.insert(generation.source_index)
                || !item.tags.iter().any(|tag| tag == "artifact")
                || item.max_stack != 1
                || generation.base_item_kind_id == item.id
                || *base_is_artifact
                || item.equipment_slot != *base_slot
                || item.melee_profile.is_some() != *base_melee
                || item.projectile_profile.is_some() != *base_projectile
                || item.ammunition_profile.is_some() != *base_ammunition
                || generation.affix_ids.len() > 8
                || generation
                    .affix_ids
                    .iter()
                    .any(|affix_id| !affix_ids.contains(affix_id))
                || generation
                    .affix_ids
                    .windows(2)
                    .any(|pair| pair[0] >= pair[1])
            {
                return Err(ContentError::InvalidArtifactGeneration(item.id.clone()));
            }
        }
        if item.inventory_slot_bonus > 100
            || (item.inventory_slot_bonus > 0
                && (item.equipment_slot.as_deref() != Some("container") || item.max_stack != 1))
        {
            return Err(ContentError::InvalidEquipmentSlot(item.id.clone()));
        }
        if item.ammunition_capacity > 500
            || (item.ammunition_capacity > 0
                && (item.equipment_slot.as_deref() != Some("quiver")
                    || item.max_stack != 1
                    || item.inventory_slot_bonus > 0))
        {
            return Err(ContentError::InvalidEquipmentSlot(item.id.clone()));
        }
        if item.capture_ball
            && (item.max_stack != 1
                || item.equipment_slot.as_deref() != Some("shield")
                || item.use_action.is_some()
                || item.device_generation.is_some())
        {
            return Err(ContentError::InvalidEquipmentSlot(item.id.clone()));
        }
        if item.modifiers.max_hp < 0
            || item.modifiers.max_hp > 1_000_000
            || item.modifiers.attack < -1_000_000
            || item.modifiers.attack > 1_000_000
            || item.modifiers.defense < -1_000_000
            || item.modifiers.defense > 1_000_000
            || !(-100..=100).contains(&item.modifiers.speed)
            || attribute_modifiers_out_of_range(&item.modifiers)
            || equipment_bonuses_out_of_range(&item.equipment_bonuses)
            || (item.initial_curse.is_some() && item.equipment_slot.is_none())
            || (item.equipment_slot.is_none()
                && (item.modifiers != StatModifiers::default()
                    || item.equipment_bonuses != EquipmentBonuses::default()))
        {
            return Err(ContentError::InvalidItemModifiers(item.id.clone()));
        }
        validate_status_immunities(&item.id, &mut item.status_immunities)?;
        if item.equipment_slot.is_none()
            && (!item.resistances.is_empty()
                || !item.status_immunities.is_empty()
                || !item.slays.is_empty()
                || !item.brands.is_empty()
                || !item.passives.is_empty()
                || item.reflects_bolts)
        {
            return Err(ContentError::InvalidItemModifiers(item.id.clone()));
        }
        if let Some(profile) = &item.melee_profile
            && (item.max_stack != 1
                || !matches!(item.equipment_slot.as_deref(), Some("weapon" | "tool"))
                || profile.attacks == 0
                || profile.attacks > 8
                || profile.to_hit < -1_000_000
                || profile.to_hit > 1_000_000
                || profile.to_damage < -1_000_000
                || profile.to_damage > 1_000_000
                || (profile.damage_dice == 0) != (profile.damage_sides == 0)
                || profile.damage_dice > 100
                || profile.damage_sides > 10_000)
        {
            return Err(ContentError::InvalidAttackProfile(item.id.clone()));
        }
        if let Some(profile) = &item.projectile_profile
            && (item.max_stack != 1
                || item.equipment_slot.as_deref() != Some("launcher")
                || profile.range == 0
                || profile.range > 32
                || profile.damage_multiplier_percent < 100
                || profile.damage_multiplier_percent > 1_000
                || profile.shot_energy == 0
                || profile.to_hit < -1_000_000
                || profile.to_hit > 1_000_000
                || profile.to_damage < -1_000_000
                || profile.to_damage > 1_000_000)
        {
            return Err(ContentError::InvalidProjectileProfile(item.id.clone()));
        }
        let is_ammunition = item.tags.iter().any(|tag| tag == "ammunition");
        if is_ammunition != item.ammunition_profile.is_some()
            || item.ammunition_profile.as_ref().is_some_and(|profile| {
                item.equipment_slot.is_some()
                    || item.max_stack <= 1
                    || profile.to_hit < -1_000_000
                    || profile.to_hit > 1_000_000
                    || profile.to_damage < -1_000_000
                    || profile.to_damage > 1_000_000
                    || profile.damage_dice == 0
                    || profile.damage_dice > 100
                    || profile.damage_sides == 0
                    || profile.damage_sides > 10_000
            })
        {
            return Err(ContentError::InvalidProjectileProfile(item.id.clone()));
        }
        if let Some(profile) = &item.throw_profile
            && (profile.to_hit < -1_000_000
                || profile.to_hit > 1_000_000
                || profile.to_damage < -1_000_000
                || profile.to_damage > 1_000_000
                || profile.damage_dice == 0
                || profile.damage_dice > 100
                || profile.damage_sides == 0
                || profile.damage_sides > 10_000)
        {
            return Err(ContentError::InvalidThrowProfile(item.id.clone()));
        }
        if let Some(action) = &mut item.use_action {
            match &mut action.effect {
                ItemUseEffectDefinition::CreateAdjacentTerrain {
                    source_terrain_ids, ..
                }
                | ItemUseEffectDefinition::CreateCurrentTerrain {
                    source_terrain_ids, ..
                } => source_terrain_ids.sort(),
                _ => {}
            }
        }
        if let Some(action) = &item.use_action {
            let valid_effect = valid_item_effect(
                &action.effect,
                terrain_tags,
                actor_tag_values,
                item_tag_values,
                resource_ids,
                affix_ids,
                loot_table_ids,
            ) && (item_effect_is_self_targeted(&action.effect)
                || matches!(
                    action.effect,
                    ItemUseEffectDefinition::IdentifyItem { .. }
                        | ItemUseEffectDefinition::EnchantItem { .. }
                        | ItemUseEffectDefinition::MundanifyItem
                        | ItemUseEffectDefinition::CraftItem { .. }
                ));
            let valid_charges = action.charges.is_none_or(|charges| {
                charges.maximum > 0
                    && charges.maximum <= 1_000_000
                    && charges.initial <= charges.maximum
                    && charges.cost > 0
                    && charges.cost <= charges.maximum
            });
            if item.equipment_slot.is_some()
                || !valid_effect
                || !valid_charges
                || action
                    .device_check_difficulty
                    .is_some_and(|difficulty| !(1..=1_000_000).contains(&difficulty))
                || (action.device_check_difficulty.is_some()
                    && !item.tags.iter().any(|tag| tag == "device"))
                || (matches!(
                    action.effect,
                    ItemUseEffectDefinition::RechargeFromDevice { .. }
                        | ItemUseEffectDefinition::IncreaseSpellLearningCapacity
                        | ItemUseEffectDefinition::IncreaseNutrition { .. }
                        | ItemUseEffectDefinition::ApplySlowness { .. }
                        | ItemUseEffectDefinition::ApplySpeed { .. }
                        | ItemUseEffectDefinition::ApplyHeroism { .. }
                        | ItemUseEffectDefinition::ApplyBerserkStrength { .. }
                        | ItemUseEffectDefinition::ApplyPoeticInspiration { .. }
                        | ItemUseEffectDefinition::ApplyStoneSkin { .. }
                        | ItemUseEffectDefinition::RestoreLifeLevels { .. }
                        | ItemUseEffectDefinition::RestoreAllAttributes
                        | ItemUseEffectDefinition::RestoreAllVitality { .. }
                        | ItemUseEffectDefinition::ApplyRestorativeFeast { .. }
                        | ItemUseEffectDefinition::ApplyElvishWaybread { .. }
                        | ItemUseEffectDefinition::ApplySaltWater
                        | ItemUseEffectDefinition::ApplyLifeRestoration { .. }
                        | ItemUseEffectDefinition::IncreaseAttribute { .. }
                        | ItemUseEffectDefinition::AugmentAttributes
                        | ItemUseEffectDefinition::IdentifyInventory
                        | ItemUseEffectDefinition::SelfKnowledge
                        | ItemUseEffectDefinition::ApplyThermalResistance { .. }
                        | ItemUseEffectDefinition::ApplyBasicResistance { .. }
                        | ItemUseEffectDefinition::ApplyPoison { .. }
                        | ItemUseEffectDefinition::ApplyBlindness { .. }
                        | ItemUseEffectDefinition::ApplyDetonation { .. }
                        | ItemUseEffectDefinition::SelfLifeLoss { .. }
                ) && (action.device_check_difficulty.is_some()
                    || action.charges.is_some()
                    || !item.tags.iter().any(|tag| tag == "consumable")))
                || (action.charges.is_some()
                    && (item.max_stack != 1
                        || action.device_check_difficulty.is_none()
                        || !item.tags.iter().any(|tag| tag == "device")))
            {
                return Err(ContentError::InvalidItemUseAction(item.id.clone()));
            }
        }
        if let Some(mount_use) = &item.mount_use {
            let valid = item.use_action.is_some()
                && item.equipment_slot.is_none()
                && item.tags.iter().any(|tag| tag == "potion")
                && match mount_use {
                    ItemMountUseDefinition::Heal {
                        minimum_bond,
                        dice,
                        sides,
                        amount,
                        full,
                        clear_statuses,
                    } => {
                        (1..=10_000).contains(minimum_bond)
                            && ((*full && *dice == 0 && *sides == 0 && *amount == 0)
                                || (!*full
                                    && (((1..=100).contains(dice)
                                        && (1..=10_000).contains(sides)
                                        && *amount == 0)
                                        || (*dice == 0
                                            && *sides == 0
                                            && (1..=1_000_000).contains(amount)))))
                            && clear_statuses
                                .iter()
                                .all(|status| validate_id(status).is_ok())
                    }
                    ItemMountUseDefinition::Haste {
                        minimum_bond,
                        duration_dice,
                        duration_sides,
                        duration_bonus,
                        extension,
                    } => {
                        (1..=10_000).contains(minimum_bond)
                            && (1..=100).contains(duration_dice)
                            && (1..=10_000).contains(duration_sides)
                            && *duration_bonus <= 10_000
                            && (1..=10_000).contains(extension)
                    }
                };
            if !valid {
                return Err(ContentError::InvalidItemUseAction(item.id.clone()));
            }
        }
        if let Some(shatter) = &item.shatter_effect {
            let valid_shatter_step = |effect: &ItemUseEffectDefinition| {
                matches!(
                    effect,
                    ItemUseEffectDefinition::Damage { .. }
                        | ItemUseEffectDefinition::Heal { .. }
                        | ItemUseEffectDefinition::HealDice { .. }
                )
            };
            let valid_shape = match &shatter.effect {
                ItemUseEffectDefinition::Sequence { effects } => {
                    !effects.is_empty() && effects.iter().all(valid_shatter_step)
                }
                effect => valid_shatter_step(effect),
            };
            if !(1..=8).contains(&shatter.radius)
                || !item.tags.iter().any(|tag| tag == "consumable")
                || !item
                    .elemental_destruction_vulnerabilities
                    .contains(&crate::ItemDestructionElement::Cold)
                || !valid_shape
                || !valid_item_effect(
                    &shatter.effect,
                    terrain_tags,
                    actor_tag_values,
                    item_tag_values,
                    resource_ids,
                    affix_ids,
                    loot_table_ids,
                )
            {
                return Err(ContentError::InvalidItemUseAction(item.id.clone()));
            }
        }
        if let Some(generation) = &mut item.device_generation {
            generation
                .activations
                .sort_by(|left, right| left.id.cmp(&right.id));
            for activation in &mut generation.activations {
                match &mut activation.effect {
                    ItemUseEffectDefinition::CreateAdjacentTerrain {
                        source_terrain_ids, ..
                    }
                    | ItemUseEffectDefinition::CreateCurrentTerrain {
                        source_terrain_ids, ..
                    } => source_terrain_ids.sort(),
                    _ => {}
                }
            }
            let mut activation_ids = BTreeSet::new();
            let valid_activations = (1..=256).contains(&generation.activations.len())
                && generation.activations.iter().all(|activation| {
                    activation_ids.insert(activation.id.clone())
                        && validate_id(&activation.id).is_ok()
                        && validate_message_key(&activation.name_key).is_ok()
                        && (1..=1_000_000).contains(&activation.weight)
                        && (1..=100).contains(&activation.min_depth)
                        && activation.min_depth <= activation.max_depth
                        && activation.max_depth <= 100
                        && (1..=1_000_000).contains(&activation.device_check_difficulty)
                        && (1..=1_000_000).contains(&activation.charges.minimum)
                        && activation.charges.minimum <= activation.charges.maximum
                        && activation.charges.maximum <= 1_000_000
                        && (1..=activation.charges.minimum).contains(&activation.charges.cost)
                        && valid_item_effect(
                            &activation.effect,
                            terrain_tags,
                            actor_tag_values,
                            item_tag_values,
                            resource_ids,
                            affix_ids,
                            loot_table_ids,
                        )
                        && !matches!(
                            activation.effect,
                            ItemUseEffectDefinition::IncreaseSpellLearningCapacity
                                | ItemUseEffectDefinition::ApplySlowness { .. }
                                | ItemUseEffectDefinition::ApplySpeed { .. }
                                | ItemUseEffectDefinition::ApplyHeroism { .. }
                                | ItemUseEffectDefinition::ApplyBerserkStrength { .. }
                                | ItemUseEffectDefinition::ApplyPoeticInspiration { .. }
                                | ItemUseEffectDefinition::ApplyStoneSkin { .. }
                                | ItemUseEffectDefinition::RestoreLifeLevels { .. }
                                | ItemUseEffectDefinition::RestoreAllAttributes
                                | ItemUseEffectDefinition::RestoreAllVitality { .. }
                                | ItemUseEffectDefinition::ApplyRestorativeFeast { .. }
                                | ItemUseEffectDefinition::ApplyLifeRestoration { .. }
                                | ItemUseEffectDefinition::IncreaseAttribute { .. }
                                | ItemUseEffectDefinition::AugmentAttributes
                                | ItemUseEffectDefinition::ApplyThermalResistance { .. }
                                | ItemUseEffectDefinition::ApplyBasicResistance { .. }
                                | ItemUseEffectDefinition::ApplyPoison { .. }
                                | ItemUseEffectDefinition::ApplyBlindness { .. }
                                | ItemUseEffectDefinition::ApplyDetonation { .. }
                                | ItemUseEffectDefinition::SelfLifeLoss { .. }
                        )
                        && valid_item_effect_target(&activation.effect, &activation.target)
                })
                && (1..=100).all(|depth| {
                    generation.activations.iter().any(|activation| {
                        activation.min_depth <= depth && depth <= activation.max_depth
                    })
                });
            let equipment_activation = item.equipment_slot.is_some()
                && ((item.artifact_generation.is_some()
                    && item.tags.iter().any(|tag| tag == "artifact"))
                    || item.tags.iter().any(|tag| tag == "activatable"));
            if item.use_action.is_some()
                || (item.equipment_slot.is_some() && !equipment_activation)
                || item.max_stack != 1
                || (!equipment_activation && !item.tags.iter().any(|tag| tag == "device"))
                || generation.recovery.is_some_and(|recovery| {
                    !(1..=10_000).contains(&recovery.interval_ticks)
                        || !(1..=1_000).contains(&recovery.energy_per_mille)
                })
                || !valid_activations
            {
                return Err(ContentError::InvalidItemUseAction(item.id.clone()));
            }
        }
        if let Some(ability_book_id) = &item.ability_book_id {
            if item.max_stack != 1
                || item.equipment_slot.is_some()
                || item.use_action.is_some()
                || item.device_generation.is_some()
            {
                return Err(ContentError::InvalidAbilityBookItem(item.id.clone()));
            }
            require_reference(ability_book_ids, ability_book_id, &item.id)?;
        }
        normalize_tags(&item.id, &mut item.tags)?;
        insert_definition_id(all_ids, &item.id)?;
        item_limits.insert(
            item.id.clone(),
            (item.max_stack, item.equipment_slot.is_some()),
        );
    }

    for (owner, corpse_item_kind_id) in actor_corpse_item_ids
        .into_iter()
        .chain(ability_corpse_item_ids)
    {
        if !item_limits.contains_key(&corpse_item_kind_id) {
            return Err(ContentError::DanglingReference {
                owner,
                target: corpse_item_kind_id,
            });
        }
        let corpse_item = items
            .iter()
            .find(|item| item.id == corpse_item_kind_id)
            .expect("validated corpse item must remain available");
        if corpse_item.equipment_slot.is_some()
            || corpse_item.max_stack != 1
            || !corpse_item.tags.iter().any(|tag| tag == "corpse")
        {
            return Err(ContentError::InvalidItemModifiers(corpse_item.id.clone()));
        }
    }
    for (owner, item_kind_id) in ability_created_item_ids {
        let Some(item) = items.iter().find(|item| item.id == item_kind_id) else {
            return Err(ContentError::DanglingReference {
                owner,
                target: item_kind_id,
            });
        };
        if item.ammunition_profile.is_none() {
            return Err(ContentError::InvalidItemModifiers(item.id.clone()));
        }
    }

    for (owner, item_kind_id, quantity) in ability_plain_created_items {
        let Some(item) = items.iter().find(|item| item.id == item_kind_id) else {
            return Err(ContentError::DanglingReference {
                owner,
                target: item_kind_id,
            });
        };
        if quantity > item.max_stack
            || item.artifact_generation.is_some()
            || item.tags.iter().any(|tag| tag == "artifact")
            || item.initial_curse.is_some()
            || item.device_generation.is_some()
            || item.fuel.is_some()
        {
            return Err(ContentError::InvalidItemModifiers(item.id.clone()));
        }
    }

    for item in items.iter() {
        let Some(profile) = &item.projectile_profile else {
            continue;
        };
        if !items.iter().any(|candidate| {
            candidate
                .ammunition_profile
                .as_ref()
                .is_some_and(|ammo| ammo.ammunition_type == profile.ammunition_type)
        }) {
            return Err(ContentError::InvalidProjectileProfile(item.id.clone()));
        }
    }
    Ok(item_limits)
}
