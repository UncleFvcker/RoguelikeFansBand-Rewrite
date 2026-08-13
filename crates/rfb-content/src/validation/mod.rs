// SPDX-License-Identifier: MPL-2.0

mod abilities;
mod actors;
mod affixes;
mod characters;
mod items;
mod shared;
mod tables;
mod terrain;
mod towns;
mod worlds;

use std::collections::BTreeSet;

use super::*;
use abilities::{
    AbilityDefinitions, AbilityValidationOutputs, AbilityValidationRefs, validate_abilities,
};
use actors::{ActorValidationOutputs, validate_actors};
use affixes::validate_affixes;
use characters::{CharacterDefinitions, CharacterValidationRefs, validate_characters};
pub(crate) use items::valid_item_effect;
use items::{ItemValidationRefs, validate_items};
use shared::{attribute_modifiers_out_of_range, insert_definition_id, validate_status_immunities};
pub(crate) use shared::{
    require_format_version, require_schema, validate_definition_id, validate_id,
    validate_message_key, validate_pack_relations, validate_semver,
};
use tables::{TableDefinitions, TableValidationOutputs, TableValidationRefs, validate_tables};
use terrain::{TerrainValidationOutputs, validate_terrain};
use towns::{TownValidationOutputs, TownValidationRefs, validate_towns_and_shops};
use worlds::{WorldValidationRefs, validate_world};

pub(crate) fn validate_and_normalize(content: &mut CompiledContentV1) -> Result<(), ContentError> {
    if content.format != CONTENT_FORMAT || content.format_version != CONTENT_FORMAT_VERSION {
        return Err(ContentError::InvalidCompiledMetadata);
    }
    validate_id(&content.pack_id)?;
    validate_semver(&content.pack_version)?;
    validate_message_key(&content.title_key)?;
    validate_pack_relations(&content.pack_id, &content.dependencies, &content.load_after)?;
    content
        .dependencies
        .sort_by(|left, right| left.id.cmp(&right.id));
    content.load_after.sort();
    content
        .terrain
        .sort_by(|left, right| left.id.cmp(&right.id));
    content.actors.sort_by(|left, right| left.id.cmp(&right.id));
    content
        .affixes
        .sort_by(|left, right| left.id.cmp(&right.id));
    content.items.sort_by(|left, right| left.id.cmp(&right.id));
    content
        .resources
        .sort_by(|left, right| left.id.cmp(&right.id));
    content
        .abilities
        .sort_by(|left, right| left.id.cmp(&right.id));
    content
        .ability_books
        .sort_by(|left, right| left.id.cmp(&right.id));
    content.skills.sort_by(|left, right| left.id.cmp(&right.id));
    content
        .skill_sets
        .sort_by(|left, right| left.id.cmp(&right.id));
    content.races.sort_by(|left, right| left.id.cmp(&right.id));
    content
        .classes
        .sort_by(|left, right| left.id.cmp(&right.id));
    content
        .personalities
        .sort_by(|left, right| left.id.cmp(&right.id));
    content.builds.sort_by(|left, right| left.id.cmp(&right.id));
    content
        .mutations
        .sort_by(|left, right| left.id.cmp(&right.id));
    content
        .encounter_tables
        .sort_by(|left, right| left.id.cmp(&right.id));
    content
        .loot_tables
        .sort_by(|left, right| left.id.cmp(&right.id));
    content
        .theme_tables
        .sort_by(|left, right| left.id.cmp(&right.id));
    content
        .region_tables
        .sort_by(|left, right| left.id.cmp(&right.id));
    content
        .terrain_feature_tables
        .sort_by(|left, right| left.id.cmp(&right.id));
    content.vaults.sort_by(|left, right| left.id.cmp(&right.id));
    content.towns.sort_by(|left, right| left.id.cmp(&right.id));
    content
        .town_facilities
        .sort_by(|left, right| left.id.cmp(&right.id));
    content.shops.sort_by(|left, right| left.id.cmp(&right.id));
    content.worlds.sort_by(|left, right| left.id.cmp(&right.id));
    let mut all_ids = BTreeSet::new();
    let mut mutation_ids = BTreeSet::new();
    let mut mutation_source_indices = BTreeSet::new();
    for mutation in &mut content.mutations {
        require_schema(&mutation.schema, MUTATION_SCHEMA, &mutation.id)?;
        require_format_version(mutation.format_version, &mutation.id)?;
        validate_definition_id(&mutation.id, "mutation")?;
        validate_status_immunities(&mutation.id, &mut mutation.status_immunities)?;
        if mutation.name.trim().is_empty() || mutation.description.trim().is_empty() {
            return Err(ContentError::InvalidDefinitionText(mutation.id.clone()));
        }
        if mutation.modifiers.max_hp.abs() > 1_000_000
            || mutation.modifiers.attack.abs() > 1_000_000
            || mutation.modifiers.defense.abs() > 1_000_000
            || !(-100..=100).contains(&mutation.modifiers.speed)
            || attribute_modifiers_out_of_range(&mutation.modifiers)
            || !(-1_000_000..=1_000_000).contains(&mutation.armor_class)
            || !(-1_000_000..=1_000_000).contains(&mutation.saving_throw_skill)
            || !(-1_000_000..=1_000_000).contains(&mutation.saving_throw_skill_per_five_levels)
            || !(-64..=64).contains(&mutation.device_skill)
            || !(-64..=64).contains(&mutation.melee_skill)
            || !(-64..=64).contains(&mutation.ranged_skill)
            || !(-64..=64).contains(&mutation.stealth_skill)
            || !(-64..=64).contains(&mutation.search_skill)
            || !(-64..=64).contains(&mutation.perception_skill)
            || !(-64..=64).contains(&mutation.infravision)
            || !(-1_000..=1_000).contains(&mutation.regeneration_rate_modifier_percent)
            || !(-10_000..=10_000).contains(&mutation.max_hp_per_level)
            || mutation.healing_bonus_percent > 1_000
            || !(-8..=8).contains(&mutation.light_radius)
            || !(-100..=100).contains(&mutation.spell_failure_modifier_percent)
            || mutation.kill_experience_bonus_percent > 1_000
            || mutation.dispel_resistance_percent > 100
            || mutation
                .weapon_proficiency_maximum
                .is_some_and(|maximum| maximum > 8_000)
        {
            return Err(ContentError::InvalidMutation(mutation.id.clone()));
        }
        if mutation.activation.as_ref().is_some_and(|activation| {
            !(1..=100).contains(&activation.minimum_level)
                || activation.cost > 1_000_000
                || activation.cost_scaling.is_some_and(|scaling| {
                    !(1..=100).contains(&scaling.start_level)
                        || scaling.level_interval == 0
                        || scaling.level_interval > 100
                        || scaling.amount == 0
                        || scaling.amount > 1_000_000
                })
                || activation.base_failure_percent > 95
                || activation
                    .minimum_failure_percent
                    .is_some_and(|minimum| minimum > activation.base_failure_percent)
                || validate_definition_id(&activation.ability_id, "ability").is_err()
        }) {
            return Err(ContentError::InvalidMutation(mutation.id.clone()));
        }
        let mut patron_ids = BTreeSet::new();
        if (!mutation.chaos_patrons.is_empty()
            && (mutation.id != "rfb.mutation.chaos-gift" || mutation.chaos_patrons.len() != 16))
            || mutation.chaos_patrons.iter().any(|patron| {
                validate_definition_id(&patron.id, "chaos-patron").is_err()
                    || patron.name.trim().is_empty()
                    || patron.rewards.len() != 20
                    || !patron_ids.insert(patron.id.clone())
            })
        {
            return Err(ContentError::InvalidMutation(mutation.id.clone()));
        }
        if mutation
            .periodic_effect
            .as_ref()
            .is_some_and(|effect| match effect {
                MutationPeriodicEffectDefinition::ApplyStatus {
                    trigger_one_in,
                    status_kind_id,
                    intensity,
                    duration_ticks,
                    duration_dice,
                    duration_sides,
                    ..
                } => {
                    *trigger_one_in == 0
                        || status_kind_id.trim().is_empty()
                        || *intensity == 0
                        || (*duration_ticks == 0 && *duration_dice == 0)
                        || (*duration_dice == 0) != (*duration_sides == 0)
                }
                MutationPeriodicEffectDefinition::BerserkRage
                | MutationPeriodicEffectDefinition::Cowardice
                | MutationPeriodicEffectDefinition::Alcohol
                | MutationPeriodicEffectDefinition::Hallucination
                | MutationPeriodicEffectDefinition::ProduceMana
                | MutationPeriodicEffectDefinition::SpeedFlux
                | MutationPeriodicEffectDefinition::Invulnerability
                | MutationPeriodicEffectDefinition::SpToHp
                | MutationPeriodicEffectDefinition::HpToSp
                | MutationPeriodicEffectDefinition::Hypochondria
                | MutationPeriodicEffectDefinition::RandomTeleport
                | MutationPeriodicEffectDefinition::RandomBanish
                | MutationPeriodicEffectDefinition::ShadowWalk
                | MutationPeriodicEffectDefinition::Fumbling
                | MutationPeriodicEffectDefinition::Flatulence
                | MutationPeriodicEffectDefinition::AttractDemon
                | MutationPeriodicEffectDefinition::EatLight
                | MutationPeriodicEffectDefinition::AttractAnimal
                | MutationPeriodicEffectDefinition::RawChaos
                | MutationPeriodicEffectDefinition::AttractDragon
                | MutationPeriodicEffectDefinition::Normality
                | MutationPeriodicEffectDefinition::Wraithform
                | MutationPeriodicEffectDefinition::PolymorphWounds
                | MutationPeriodicEffectDefinition::Wasting
                | MutationPeriodicEffectDefinition::RandomTelepathy
                | MutationPeriodicEffectDefinition::Nausea
                | MutationPeriodicEffectDefinition::Warning => false,
            })
        {
            return Err(ContentError::InvalidMutation(mutation.id.clone()));
        }
        if [
            mutation.relative_experience_multiplier,
            mutation.movement_energy_multiplier,
            mutation.scroll_energy_multiplier,
            mutation.potion_energy_multiplier,
        ]
        .into_iter()
        .flatten()
        .any(|ratio| {
            ratio.numerator == 0
                || ratio.denominator == 0
                || ratio.numerator > 1_000
                || ratio.denominator > 1_000
        }) {
            return Err(ContentError::InvalidMutation(mutation.id.clone()));
        }
        if mutation.innate_attack.as_ref().is_some_and(|attack| {
            attack.name.trim().is_empty()
                || attack.damage_dice == 0
                || attack.damage_sides == 0
                || attack.weight_tenths_pound == 0
                || !(-1_000_000..=1_000_000).contains(&attack.to_hit)
                || !(-1_000_000..=1_000_000).contains(&attack.to_damage)
        }) {
            return Err(ContentError::InvalidMutation(mutation.id.clone()));
        }
        if !mutation_source_indices.insert(mutation.source_index) {
            return Err(ContentError::InvalidMutation(mutation.id.clone()));
        }
        mutation_ids.insert(mutation.id.clone());
        insert_definition_id(&mut all_ids, &mutation.id)?;
    }
    for mutation in &content.mutations {
        let mut removed = BTreeSet::new();
        if mutation.removes_on_gain.iter().any(|id| {
            id == &mutation.id || !mutation_ids.contains(id) || !removed.insert(id.clone())
        }) {
            return Err(ContentError::InvalidMutation(mutation.id.clone()));
        }
    }
    let TerrainValidationOutputs {
        terrain_ids,
        terrain_walkability,
        terrain_connectability,
        terrain_tags,
        terrain_open_targets,
        terrain_traps,
    } = validate_terrain(&mut content.terrain, &mut all_ids)?;

    let ActorValidationOutputs {
        actor_roles,
        actor_tag_values,
        actor_levels,
        actor_loot_table_ids,
        actor_monster_casting,
        actor_corpse_item_ids,
    } = validate_actors(&mut content.actors, &mut all_ids)?;

    let item_tag_values = content
        .items
        .iter()
        .flat_map(|item| item.tags.iter().cloned())
        .collect::<BTreeSet<_>>();

    let affix_ids = validate_affixes(&mut content.affixes, &mut all_ids)?.affix_ids;

    let AbilityValidationOutputs {
        resource_ids,
        ability_resources,
        ability_ids,
        ability_corpse_item_ids,
        ability_created_item_ids,
        ability_race_ids,
        ability_books_by_id,
        ability_book_ids,
    } = validate_abilities(
        AbilityDefinitions {
            resources: &mut content.resources,
            abilities: &mut content.abilities,
            ability_books: &mut content.ability_books,
        },
        AbilityValidationRefs {
            actor_tag_values: &actor_tag_values,
            item_tag_values: &item_tag_values,
            terrain_tags: &terrain_tags,
            actor_roles: &actor_roles,
            affix_ids: &affix_ids,
            terrain_ids: &terrain_ids,
            actor_monster_casting,
        },
        &mut all_ids,
    )?;

    let item_limits = validate_items(
        &mut content.items,
        ItemValidationRefs {
            terrain_tags: &terrain_tags,
            actor_tag_values: &actor_tag_values,
            item_tag_values: &item_tag_values,
            resource_ids: &resource_ids,
            affix_ids: &affix_ids,
            loot_table_ids: &content
                .loot_tables
                .iter()
                .map(|table| table.id.clone())
                .collect(),
            ability_book_ids: &ability_book_ids,
            actor_corpse_item_ids,
            ability_corpse_item_ids,
            ability_created_item_ids,
        },
        &mut all_ids,
    )?;

    let build_ids = validate_characters(
        CharacterDefinitions {
            skills: &mut content.skills,
            skill_sets: &mut content.skill_sets,
            races: &mut content.races,
            classes: &mut content.classes,
            personalities: &mut content.personalities,
            builds: &mut content.builds,
        },
        CharacterValidationRefs {
            items: &content.items,
            terrain: &content.terrain,
            actors: &content.actors,
            actor_tag_values: &actor_tag_values,
            ability_race_ids,
            resource_ids: &resource_ids,
            ability_book_ids: &ability_book_ids,
            ability_books_by_id: &ability_books_by_id,
            ability_resources: &ability_resources,
            abilities: &content.abilities,
            mutations: &content.mutations,
        },
        &mut all_ids,
    )?;

    let ordinary_player_ability_ids = content
        .ability_books
        .iter()
        .flat_map(|book| book.ability_ids.iter().cloned())
        .collect::<BTreeSet<_>>();
    let class_ids = content
        .classes
        .iter()
        .map(|class| class.id.clone())
        .collect::<BTreeSet<_>>();
    let mut mutation_ability_ids = BTreeSet::new();
    for mutation in &content.mutations {
        let Some(activation) = &mutation.activation else {
            continue;
        };
        if !ability_ids.contains(&activation.ability_id)
            || ordinary_player_ability_ids.contains(&activation.ability_id)
            || !mutation_ability_ids.insert(activation.ability_id.clone())
        {
            return Err(ContentError::InvalidMutation(mutation.id.clone()));
        }
    }

    let TableValidationOutputs {
        loot_table_ids,
        loot_tables_by_id,
        encounter_tables_by_id,
        vaults_by_id,
        theme_tables_by_id,
        region_tables_by_id,
        terrain_feature_tables_by_id,
    } = validate_tables(
        TableDefinitions {
            loot_tables: &mut content.loot_tables,
            encounter_tables: &mut content.encounter_tables,
            vaults: &mut content.vaults,
            theme_tables: &mut content.theme_tables,
            region_tables: &mut content.region_tables,
            terrain_feature_tables: &mut content.terrain_feature_tables,
        },
        TableValidationRefs {
            item_limits: &item_limits,
            affix_ids: &affix_ids,
            items: &content.items,
            affixes: &content.affixes,
            actor_loot_table_ids,
            actor_roles: &actor_roles,
            actor_tag_values: &actor_tag_values,
            actor_levels: &actor_levels,
            terrain_ids: &terrain_ids,
            terrain_walkability: &terrain_walkability,
            terrain_connectability: &terrain_connectability,
            terrain: &content.terrain,
        },
        &mut all_ids,
    )?;

    let TownValidationOutputs {
        towns_by_id,
        facilities_by_id,
        shops_by_id,
    } = validate_towns_and_shops(
        &mut content.towns,
        &mut content.town_facilities,
        &mut content.shops,
        TownValidationRefs {
            items: &content.items,
            races: &content.races,
        },
        &mut all_ids,
    )?;

    let mut referenced_towns = BTreeSet::new();
    for world in &mut content.worlds {
        require_schema(&world.schema, WORLD_SCHEMA, &world.id)?;
        require_format_version(world.format_version, &world.id)?;
        validate_definition_id(&world.id, "world")?;
        validate_message_key(&world.name_key)?;
        insert_definition_id(&mut all_ids, &world.id)?;
        validate_world(
            world,
            &WorldValidationRefs {
                terrain_ids: &terrain_ids,
                terrain: &content.terrain,
                terrain_walkability: &terrain_walkability,
                terrain_tags: &terrain_tags,
                terrain_open_targets: &terrain_open_targets,
                terrain_traps: &terrain_traps,
                actor_roles: &actor_roles,
                actor_levels: &actor_levels,
                actors: &content.actors,
                item_limits: &item_limits,
                items: &content.items,
                affix_ids: &affix_ids,
                encounter_tables: &encounter_tables_by_id,
                loot_table_ids: &loot_table_ids,
                loot_tables: &loot_tables_by_id,
                theme_tables: &theme_tables_by_id,
                region_tables: &region_tables_by_id,
                terrain_feature_tables: &terrain_feature_tables_by_id,
                vaults: &vaults_by_id,
                build_ids: &build_ids,
                class_ids: &class_ids,
                towns: &towns_by_id,
                town_facilities: &facilities_by_id,
                shops: &shops_by_id,
            },
        )?;
        for task in &world.tasks {
            insert_definition_id(&mut all_ids, &task.id)?;
        }
        if let Some(wilderness) = &world.wilderness {
            for town_id in wilderness
                .locations
                .iter()
                .filter_map(|location| match location {
                    WildernessLocationDefinition::Town { town_id, .. } => Some(town_id),
                    WildernessLocationDefinition::Dungeon { .. } => None,
                })
            {
                if !referenced_towns.insert(town_id.clone()) {
                    return Err(ContentError::InvalidTown(town_id.clone()));
                }
            }
        }
    }
    if referenced_towns.len() != towns_by_id.len() {
        let unowned = towns_by_id
            .keys()
            .find(|town_id| !referenced_towns.contains(*town_id))
            .expect("town count mismatch must identify an unowned town");
        return Err(ContentError::InvalidTown(unowned.clone()));
    }
    Ok(())
}
