// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ACTOR_SCHEMA, ActorDamageType, ActorDefinition, ActorRole, ContentError,
    MeleeBlowEffectDefinition, MonsterCastingDefinition, MonsterDropKindDefinition,
};

use super::shared::{
    insert_definition_id, normalize_tags, require_format_version, require_schema,
    validate_definition_id, validate_definition_text, validate_glyph, validate_id,
    validate_status_immunities,
};

pub(super) struct ActorValidationOutputs {
    pub(super) actor_roles: BTreeMap<String, ActorRole>,
    pub(super) actor_tag_values: BTreeSet<String>,
    pub(super) actor_levels: BTreeMap<String, u32>,
    pub(super) actor_loot_table_ids: Vec<(String, String)>,
    pub(super) actor_monster_casting: Vec<(String, MonsterCastingDefinition)>,
    pub(super) actor_corpse_item_ids: Vec<(String, String)>,
}

pub(super) fn validate_actors(
    actors: &mut [ActorDefinition],
    all_ids: &mut BTreeSet<String>,
) -> Result<ActorValidationOutputs, ContentError> {
    let mut actor_roles = BTreeMap::new();
    let mut actor_tag_values = BTreeSet::new();
    let mut actor_levels = BTreeMap::new();
    let mut actor_loot_table_ids = Vec::new();
    let mut actor_monster_casting = Vec::new();
    let mut actor_corpse_item_ids = Vec::new();
    let mut allocation_indices = BTreeSet::new();
    for actor in actors {
        require_schema(&actor.schema, ACTOR_SCHEMA, &actor.id)?;
        require_format_version(actor.format_version, &actor.id)?;
        validate_definition_id(&actor.id, "actor")?;
        validate_definition_text(&actor.id, &actor.name_key, &actor.description_key)?;
        validate_glyph(&actor.id, &actor.glyph)?;
        validate_status_immunities(&actor.id, &mut actor.status_immunities)?;
        actor.movement.modes.sort_unstable();
        actor.movement.modes.dedup();
        let hit_point_dice_are_valid = actor.hit_point_dice.is_none_or(|hit_points| {
            let maximum = i32::from(hit_points.dice).saturating_mul(i32::from(hit_points.sides));
            let expected_definition_hp = if hit_points.force_maximum {
                maximum
            } else {
                i32::from(hit_points.dice)
                    .saturating_mul(i32::from(hit_points.sides).saturating_add(1))
                    / 2
            };
            actor.role == ActorRole::Monster
                && hit_points.dice > 0
                && (1..=10_000).contains(&hit_points.sides)
                && maximum <= 1_000_000
                && actor.max_hp == expected_definition_hp
        });
        if actor.level > 10_000
            || actor.experience_value > 999_999_999
            || (actor.role == ActorRole::Player && actor.experience_value != 0)
            || actor.max_hp <= 0
            || actor.max_hp > 1_000_000
            || actor.speed > 199
            || actor.attack <= 0
            || actor.attack > 1_000_000
            || actor.defense < 0
            || actor.defense > 1_000_000
            || actor.door_skill < 0
            || actor.door_skill > 1_000_000
            || actor.bash_power < 0
            || actor.bash_power > 1_000_000
            || actor.search_skill < 0
            || actor.search_skill > 1_000_000
            || actor.damage_dice == 0
            || actor.damage_dice > 100
            || actor.damage_sides == 0
            || actor.damage_sides > 10_000
            || !hit_point_dice_are_valid
            || (actor.role != ActorRole::Monster
                && (actor.door_interaction.opens || actor.door_interaction.bashes))
            || (actor.role != ActorRole::Monster
                && (!actor.movement.modes.is_empty() || actor.movement.never_moves))
            || (actor.role != ActorRole::Monster && !actor.status_immunities.is_empty())
            || (actor.role != ActorRole::Monster && actor.reflects_bolts)
        {
            return Err(ContentError::InvalidActorStats(actor.id.clone()));
        }
        if (actor.role == ActorRole::Player
            && (actor.inventory_slot_capacity == 0 || actor.inventory_slot_capacity > 1_000))
            || (actor.role == ActorRole::Monster && actor.inventory_slot_capacity != 0)
        {
            return Err(ContentError::InvalidActorInventoryCapacity(
                actor.id.clone(),
            ));
        }
        if actor.awareness.as_ref().is_some_and(|awareness| {
            actor.role != ActorRole::Monster
                || !(1..=1_000_000).contains(&awareness.detection_difficulty)
                || awareness.detection_range == 0
                || awareness.detection_range > 64
        }) {
            return Err(ContentError::InvalidActorStats(actor.id.clone()));
        }
        if let Some(casting) = &actor.monster_casting {
            let mut ability_ids = BTreeSet::new();
            if actor.role != ActorRole::Monster
                || !(1..=100).contains(&casting.frequency_percent)
                || casting
                    .preferred_distance
                    .is_some_and(|distance| !(2..=16).contains(&distance))
                || casting.flee_hp_percent > 99
                || casting.abilities.is_empty()
                || casting.abilities.len() > 64
                || casting.abilities.iter().any(|candidate| {
                    validate_id(&candidate.ability_id).is_err()
                        || !(1..=1_000_000).contains(&candidate.weight)
                        || !ability_ids.insert(candidate.ability_id.clone())
                })
            {
                return Err(ContentError::InvalidMonsterCasting(actor.id.clone()));
            }
            actor_monster_casting.push((actor.id.clone(), casting.clone()));
        }
        if let Some(routine) = &actor.melee_routine
            && (actor.role != ActorRole::Monster
                || routine.blows.len() > 8
                || routine
                    .blows
                    .iter()
                    .filter(|blow| blow.self_destructs)
                    .count()
                    > 1
                || routine.blows.iter().any(|blow| {
                    validate_id(&blow.method_id).is_err()
                        || blow.to_hit < -1_000_000
                        || blow.to_hit > 1_000_000
                        || (blow.effects.is_empty()
                            && (blow.method_id != "rfb.blow.beg" || blow.self_destructs))
                        || blow.effects.len() > 8
                        || (blow.self_destructs
                            && blow.effects.iter().any(|effect| {
                                !matches!(
                                    effect,
                                    MeleeBlowEffectDefinition::Damage { .. }
                                        | MeleeBlowEffectDefinition::Poison { .. }
                                )
                            }))
                        || blow
                            .effects
                            .iter()
                            .any(|effect| !valid_melee_effect(effect))
                }))
        {
            return Err(ContentError::InvalidMeleeRoutine(actor.id.clone()));
        }
        if actor.light.is_some_and(|light| {
            actor.role != ActorRole::Monster || !(1..=8).contains(&light.radius)
        }) || (actor.role != ActorRole::Monster
            && (actor.terrain_interaction.destroys_walls
                || actor.terrain_interaction.destroys_items
                || actor.terrain_interaction.picks_up_items))
        {
            return Err(ContentError::InvalidActorStats(actor.id.clone()));
        }
        if let Some(loot_table_id) = &actor.loot_table_id {
            if actor.role != ActorRole::Monster || validate_id(loot_table_id).is_err() {
                return Err(ContentError::InvalidActorLootTable(actor.id.clone()));
            }
            actor_loot_table_ids.push((actor.id.clone(), loot_table_id.clone()));
        }
        if actor.gold_drop_chance_percent.is_some_and(|chance| {
            actor.role != ActorRole::Monster
                || actor.loot_table_id.is_none()
                || !(1..=100).contains(&chance)
        }) {
            return Err(ContentError::InvalidActorLootTable(actor.id.clone()));
        }
        if let Some(drop) = &actor.death_drop {
            let allows_items = matches!(
                drop.kind,
                MonsterDropKindDefinition::Items | MonsterDropKindDefinition::ItemsAndGold
            );
            let maximum_rolls = u32::from(drop.base_rolls)
                .saturating_add(u32::try_from(drop.chance_rolls.len()).unwrap_or(u32::MAX))
                .saturating_add(drop.count_dice.iter().fold(0_u32, |total, dice| {
                    total.saturating_add(u32::from(dice.dice) * u32::from(dice.sides))
                }));
            if actor.role != ActorRole::Monster
                || actor.loot_table_id.is_some()
                || actor.gold_drop_chance_percent.is_some()
                || allows_items != drop.item_table_id.is_some()
                || (!allows_items
                    && (drop.theme_table_id.is_some() || drop.theme_chance_percent != 0))
                || (drop.theme_table_id.is_some() != (drop.theme_chance_percent > 0))
                || drop.theme_chance_percent > 100
                || maximum_rolls == 0
                || maximum_rolls > 32
                || drop
                    .chance_rolls
                    .iter()
                    .any(|roll| !(1..=100).contains(&roll.percent))
                || drop.count_dice.iter().any(|dice| {
                    dice.dice == 0
                        || dice.sides == 0
                        || u16::from(dice.dice) * u16::from(dice.sides) > 32
                })
            {
                return Err(ContentError::InvalidActorLootTable(actor.id.clone()));
            }
            if let Some(table_id) = &drop.item_table_id {
                if validate_id(table_id).is_err() {
                    return Err(ContentError::InvalidActorLootTable(actor.id.clone()));
                }
                actor_loot_table_ids.push((format!("{}#drop", actor.id), table_id.clone()));
            }
            if let Some(table_id) = &drop.theme_table_id {
                if validate_id(table_id).is_err() {
                    return Err(ContentError::InvalidActorLootTable(actor.id.clone()));
                }
                actor_loot_table_ids.push((format!("{}#drop-theme", actor.id), table_id.clone()));
            }
        }
        if let Some(loot_table_id) = &actor.carried_loot_table_id {
            if actor.role != ActorRole::Monster || validate_id(loot_table_id).is_err() {
                return Err(ContentError::InvalidActorLootTable(actor.id.clone()));
            }
            actor_loot_table_ids.push((format!("{}#carried", actor.id), loot_table_id.clone()));
        }
        if let Some(corpse_item_kind_id) = &actor.corpse_item_kind_id {
            if actor.role != ActorRole::Monster || validate_id(corpse_item_kind_id).is_err() {
                return Err(ContentError::InvalidActorStats(actor.id.clone()));
            }
            actor_corpse_item_ids.push((actor.id.clone(), corpse_item_kind_id.clone()));
        }
        if let Some(remains) = &actor.remains {
            let valid_corpse = remains.corpse_item_kind_id.is_some() == (remains.corpse_weight > 0);
            let valid_skeleton =
                remains.skeleton_item_kind_id.is_some() == (remains.skeleton_weight > 0);
            if actor.role != ActorRole::Monster
                || actor.corpse_item_kind_id.is_some()
                || remains.chance_denominator == 0
                || remains.chance_denominator > 10_000
                || (!valid_corpse || !valid_skeleton)
                || remains
                    .corpse_weight
                    .saturating_add(remains.skeleton_weight)
                    == 0
            {
                return Err(ContentError::InvalidActorStats(actor.id.clone()));
            }
            if let Some(item_kind_id) = &remains.corpse_item_kind_id {
                if validate_id(item_kind_id).is_err() {
                    return Err(ContentError::InvalidActorStats(actor.id.clone()));
                }
                actor_corpse_item_ids.push((actor.id.clone(), item_kind_id.clone()));
            }
            if let Some(item_kind_id) = &remains.skeleton_item_kind_id {
                if validate_id(item_kind_id).is_err() {
                    return Err(ContentError::InvalidActorStats(actor.id.clone()));
                }
                actor_corpse_item_ids.push((actor.id.clone(), item_kind_id.clone()));
            }
        }
        if let Some(allocation) = &actor.allocation {
            let friends_are_valid = allocation.friends.is_none_or(|friends| {
                let fixed_dice = friends.dice > 0
                    && friends.sides > 0
                    && u16::from(friends.dice) * u16::from(friends.sides) <= 32;
                let depth_adjusted = friends.dice == 0 && friends.sides == 0;
                (fixed_dice || depth_adjusted) && friends.chance_percent <= 100
            });
            if actor.role != ActorRole::Monster
                || allocation.legacy_index == 0
                || !allocation_indices.insert(allocation.legacy_index)
                || allocation.rarity == 0
                || allocation.rarity > 1_000_000
                || allocation.max_depth > 10_000
                || allocation.legacy_dungeon_indices.contains(&0)
                || allocation
                    .legacy_dungeon_indices
                    .windows(2)
                    .any(|window| window[0] >= window[1])
                || !matches!(allocation.random_movement_percent, 0 | 25 | 50 | 75)
                || (actor.movement.never_moves && allocation.random_movement_percent != 0)
                || !friends_are_valid
                || (allocation.friends.is_some() && allocation.escort)
            {
                return Err(ContentError::InvalidActorStats(actor.id.clone()));
            }
        }
        if actor.contact_auras.len() > 8
            || actor.contact_auras.iter().any(|aura| {
                !matches!(
                    aura.damage_type,
                    ActorDamageType::Poison
                        | ActorDamageType::Acid
                        | ActorDamageType::Fire
                        | ActorDamageType::Cold
                        | ActorDamageType::Electricity
                        | ActorDamageType::Curse
                ) || !(1..=100).contains(&aura.damage_dice)
                    || !(1..=10_000).contains(&aura.damage_sides)
                    || aura
                        .chance_percent
                        .is_some_and(|chance| !(1..=100).contains(&chance))
            })
        {
            return Err(ContentError::InvalidActorStats(actor.id.clone()));
        }
        normalize_tags(&actor.id, &mut actor.tags)?;
        for tag in &actor.tags {
            actor_tag_values.insert(tag.clone());
        }
        insert_definition_id(all_ids, &actor.id)?;
        actor_roles.insert(actor.id.clone(), actor.role);
        actor_levels.insert(actor.id.clone(), actor.level);
    }
    Ok(ActorValidationOutputs {
        actor_roles,
        actor_tag_values,
        actor_levels,
        actor_loot_table_ids,
        actor_monster_casting,
        actor_corpse_item_ids,
    })
}

fn valid_melee_effect(effect: &MeleeBlowEffectDefinition) -> bool {
    let valid_chance = |chance: Option<u8>| chance.is_none_or(|chance| (1..=100).contains(&chance));
    let valid_dice =
        |dice: u16, sides: u16| (1..=100).contains(&dice) && (1..=10_000).contains(&sides);
    match effect {
        MeleeBlowEffectDefinition::Damage {
            chance_percent,
            damage_dice,
            damage_sides,
            damage_type,
            armor_mitigated,
            ..
        } => {
            valid_chance(*chance_percent)
                && ((*armor_mitigated
                    && *damage_type == ActorDamageType::Physical
                    && *damage_dice == 0
                    && *damage_sides == 0)
                    || valid_dice(*damage_dice, *damage_sides))
        }
        MeleeBlowEffectDefinition::Shatter {
            chance_percent,
            damage_dice,
            damage_sides,
        } => valid_chance(*chance_percent) && valid_dice(*damage_dice, *damage_sides),
        MeleeBlowEffectDefinition::Poison {
            chance_percent,
            damage_dice,
            damage_sides,
        } => valid_chance(*chance_percent) && valid_dice(*damage_dice, *damage_sides),
        MeleeBlowEffectDefinition::Disease {
            chance_percent,
            damage_dice,
            damage_sides,
        } => {
            valid_chance(*chance_percent)
                && ((*damage_dice == 0 && *damage_sides == 0)
                    || valid_dice(*damage_dice, *damage_sides))
        }
        MeleeBlowEffectDefinition::DrainAttributes {
            chance_percent,
            attributes,
        } => {
            valid_chance(*chance_percent)
                && !attributes.is_empty()
                && attributes.len() <= 6
                && !attributes
                    .iter()
                    .enumerate()
                    .any(|(index, attribute)| attributes[..index].contains(attribute))
        }
        MeleeBlowEffectDefinition::DrainResource {
            chance_percent,
            amount_dice,
            amount_sides,
        }
        | MeleeBlowEffectDefinition::DrainExperience {
            chance_percent,
            amount_dice,
            amount_sides,
        }
        | MeleeBlowEffectDefinition::Unlife {
            chance_percent,
            amount_dice,
            amount_sides,
        } => valid_chance(*chance_percent) && valid_dice(*amount_dice, *amount_sides),
        MeleeBlowEffectDefinition::Bleeding {
            chance_percent,
            duration_dice,
            duration_sides,
        }
        | MeleeBlowEffectDefinition::Stun {
            chance_percent,
            duration_dice,
            duration_sides,
        } => valid_chance(*chance_percent) && valid_dice(*duration_dice, *duration_sides),
        MeleeBlowEffectDefinition::Confusion {
            chance_percent,
            damage_dice,
            damage_sides,
        } => {
            valid_chance(*chance_percent)
                && ((*damage_dice == 0 && *damage_sides == 0)
                    || valid_dice(*damage_dice, *damage_sides))
        }
        MeleeBlowEffectDefinition::Blind { chance_percent }
        | MeleeBlowEffectDefinition::DrainCharges { chance_percent }
        | MeleeBlowEffectDefinition::Paralysis { chance_percent }
        | MeleeBlowEffectDefinition::Slow { chance_percent }
        | MeleeBlowEffectDefinition::Terrify { chance_percent }
        | MeleeBlowEffectDefinition::Disenchant { chance_percent }
        | MeleeBlowEffectDefinition::EatGold { chance_percent }
        | MeleeBlowEffectDefinition::EatItem { chance_percent }
        | MeleeBlowEffectDefinition::EatFood { chance_percent }
        | MeleeBlowEffectDefinition::EatLight { chance_percent } => valid_chance(*chance_percent),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor_with_effectless_blow(method_id: &str) -> ActorDefinition {
        serde_json::from_value(serde_json::json!({
            "$schema": ACTOR_SCHEMA,
            "formatVersion": 1,
            "id": "test.actor.effectless-blow",
            "role": "monster",
            "nameKey": "test-actor-effectless-blow-name",
            "descriptionKey": "test-actor-effectless-blow-description",
            "glyph": "t",
            "level": 1,
            "maxHp": 1,
            "attack": 1,
            "defense": 0,
            "damageDice": 1,
            "damageSides": 1,
            "meleeRoutine": {
                "blows": [{ "methodId": method_id, "toHit": 20, "effects": [] }]
            },
            "tags": []
        }))
        .expect("synthetic actor should deserialize")
    }

    #[test]
    fn only_beg_allows_an_effectless_melee_blow() {
        let mut beg = vec![actor_with_effectless_blow("rfb.blow.beg")];
        if let Err(error) = validate_actors(&mut beg, &mut BTreeSet::new()) {
            panic!("{error:?}");
        }

        let mut hit = vec![actor_with_effectless_blow("rfb.blow.hit")];
        assert!(matches!(
            validate_actors(&mut hit, &mut BTreeSet::new()),
            Err(ContentError::InvalidMeleeRoutine(id)) if id == "test.actor.effectless-blow"
        ));
    }
}
