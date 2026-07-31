// SPDX-License-Identifier: MPL-2.0

mod abilities;
mod actors;
mod affixes;
mod characters;
mod items;
mod shared;
mod tables;
mod terrain;
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
use shared::insert_definition_id;
pub(crate) use shared::{
    require_format_version, require_schema, validate_definition_id, validate_id,
    validate_message_key, validate_pack_relations, validate_semver,
};
use tables::{TableDefinitions, TableValidationOutputs, TableValidationRefs, validate_tables};
use terrain::{TerrainValidationOutputs, validate_terrain};
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
    content.worlds.sort_by(|left, right| left.id.cmp(&right.id));
    let mut all_ids = BTreeSet::new();
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
            ability_book_ids: &ability_book_ids,
            actor_corpse_item_ids,
            ability_corpse_item_ids,
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
            ability_ids: &ability_ids,
            abilities: &content.abilities,
        },
        &mut all_ids,
    )?;

    let TableValidationOutputs {
        loot_table_ids,
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
            actor_loot_table_ids,
            actor_roles: &actor_roles,
            actor_levels: &actor_levels,
            terrain_ids: &terrain_ids,
            terrain_walkability: &terrain_walkability,
            terrain_connectability: &terrain_connectability,
            terrain: &content.terrain,
        },
        &mut all_ids,
    )?;

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
                terrain_walkability: &terrain_walkability,
                terrain_tags: &terrain_tags,
                terrain_open_targets: &terrain_open_targets,
                terrain_traps: &terrain_traps,
                actor_roles: &actor_roles,
                actor_levels: &actor_levels,
                item_limits: &item_limits,
                affix_ids: &affix_ids,
                encounter_tables: &encounter_tables_by_id,
                loot_table_ids: &loot_table_ids,
                theme_tables: &theme_tables_by_id,
                region_tables: &region_tables_by_id,
                terrain_feature_tables: &terrain_feature_tables_by_id,
                vaults: &vaults_by_id,
                build_ids: &build_ids,
            },
        )?;
    }
    Ok(())
}
