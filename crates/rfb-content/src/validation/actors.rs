// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

use crate::{ACTOR_SCHEMA, ActorDefinition, ActorRole, ContentError, MonsterCastingDefinition};

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
                && (1..=100).contains(&hit_points.dice)
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
            || (actor.role != ActorRole::Monster && !actor.movement.modes.is_empty())
            || (actor.role != ActorRole::Monster && !actor.status_immunities.is_empty())
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
                || routine.blows.is_empty()
                || routine.blows.len() > 8
                || routine.blows.iter().any(|blow| {
                    validate_id(&blow.method_id).is_err()
                        || blow.to_hit < -1_000_000
                        || blow.to_hit > 1_000_000
                        || blow.damage_dice == 0
                        || blow.damage_dice > 100
                        || blow.damage_sides == 0
                        || blow.damage_sides > 10_000
                }))
        {
            return Err(ContentError::InvalidMeleeRoutine(actor.id.clone()));
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
                || !matches!(allocation.random_movement_percent, 0 | 25 | 50 | 75)
                || !friends_are_valid
                || (allocation.friends.is_some() && allocation.escort)
            {
                return Err(ContentError::InvalidActorStats(actor.id.clone()));
            }
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
