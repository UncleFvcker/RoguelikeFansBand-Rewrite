// SPDX-License-Identifier: MPL-2.0

use schemars::schema_for;
use serde::Serialize;

use super::{
    ABILITY_BOOK_SCHEMA, ABILITY_PROGRAM_SCHEMA, ABILITY_SCHEMA, ACTOR_SCHEMA, AFFIX_SCHEMA,
    AbilityBookDefinition, AbilityProgramDefinition, ActorDefinition, AffixDefinition,
    BUILD_SCHEMA, CLASS_SCHEMA, CharacterBuildDefinition, ClassDefinition, EFFECT_PROGRAM_SCHEMA,
    ENCOUNTER_TABLE_SCHEMA, EffectProgramDefinition, EncounterTableDefinition, ITEM_SCHEMA,
    LOOT_TABLE_SCHEMA, LootTableDefinition, PACK_SCHEMA, PERSONALITY_SCHEMA,
    PLAYER_ABILITY_BINDING_SCHEMA, PackManifest, PersonalityDefinition,
    PlayerAbilityBindingDefinition, RACE_SCHEMA, REGION_TABLE_SCHEMA, RESOURCE_SCHEMA,
    RaceDefinition, RegionTableDefinition, ResourceDefinition, SKILL_SCHEMA, SKILL_SET_SCHEMA,
    SkillDefinition, SkillSetDefinition, SourceAbilityDefinition, SourceItemDefinition,
    TERRAIN_FEATURE_TABLE_SCHEMA, TERRAIN_SCHEMA, THEME_TABLE_SCHEMA, TerrainDefinition,
    TerrainFeatureTableDefinition, ThemeTableDefinition, VAULT_SCHEMA, VaultDefinition,
    WORLD_SCHEMA, WorldDefinition,
};

pub fn generated_schema_documents() -> Result<Vec<(&'static str, String)>, serde_json::Error> {
    Ok(vec![
        schema_document("pack.schema.json", PACK_SCHEMA, schema_for!(PackManifest))?,
        schema_document(
            "terrain.schema.json",
            TERRAIN_SCHEMA,
            schema_for!(TerrainDefinition),
        )?,
        schema_document(
            "actor.schema.json",
            ACTOR_SCHEMA,
            schema_for!(ActorDefinition),
        )?,
        schema_document(
            "item.schema.json",
            ITEM_SCHEMA,
            schema_for!(SourceItemDefinition),
        )?,
        schema_document(
            "effect-program.schema.json",
            EFFECT_PROGRAM_SCHEMA,
            schema_for!(EffectProgramDefinition),
        )?,
        schema_document(
            "resource.schema.json",
            RESOURCE_SCHEMA,
            schema_for!(ResourceDefinition),
        )?,
        schema_document(
            "ability.schema.json",
            ABILITY_SCHEMA,
            schema_for!(SourceAbilityDefinition),
        )?,
        schema_document(
            "ability-program.schema.json",
            ABILITY_PROGRAM_SCHEMA,
            schema_for!(AbilityProgramDefinition),
        )?,
        schema_document(
            "player-ability-binding.schema.json",
            PLAYER_ABILITY_BINDING_SCHEMA,
            schema_for!(PlayerAbilityBindingDefinition),
        )?,
        schema_document(
            "ability-book.schema.json",
            ABILITY_BOOK_SCHEMA,
            schema_for!(AbilityBookDefinition),
        )?,
        schema_document(
            "skill.schema.json",
            SKILL_SCHEMA,
            schema_for!(SkillDefinition),
        )?,
        schema_document(
            "skill-set.schema.json",
            SKILL_SET_SCHEMA,
            schema_for!(SkillSetDefinition),
        )?,
        schema_document("race.schema.json", RACE_SCHEMA, schema_for!(RaceDefinition))?,
        schema_document(
            "class.schema.json",
            CLASS_SCHEMA,
            schema_for!(ClassDefinition),
        )?,
        schema_document(
            "personality.schema.json",
            PERSONALITY_SCHEMA,
            schema_for!(PersonalityDefinition),
        )?,
        schema_document(
            "build.schema.json",
            BUILD_SCHEMA,
            schema_for!(CharacterBuildDefinition),
        )?,
        schema_document(
            "affix.schema.json",
            AFFIX_SCHEMA,
            schema_for!(AffixDefinition),
        )?,
        schema_document(
            "encounter-table.schema.json",
            ENCOUNTER_TABLE_SCHEMA,
            schema_for!(EncounterTableDefinition),
        )?,
        schema_document(
            "loot-table.schema.json",
            LOOT_TABLE_SCHEMA,
            schema_for!(LootTableDefinition),
        )?,
        schema_document(
            "theme-table.schema.json",
            THEME_TABLE_SCHEMA,
            schema_for!(ThemeTableDefinition),
        )?,
        schema_document(
            "region-table.schema.json",
            REGION_TABLE_SCHEMA,
            schema_for!(RegionTableDefinition),
        )?,
        schema_document(
            "terrain-feature-table.schema.json",
            TERRAIN_FEATURE_TABLE_SCHEMA,
            schema_for!(TerrainFeatureTableDefinition),
        )?,
        schema_document(
            "vault.schema.json",
            VAULT_SCHEMA,
            schema_for!(VaultDefinition),
        )?,
        schema_document(
            "world.schema.json",
            WORLD_SCHEMA,
            schema_for!(WorldDefinition),
        )?,
    ])
}

fn schema_document<T: Serialize>(
    file_name: &'static str,
    schema_id: &str,
    schema: T,
) -> Result<(&'static str, String), serde_json::Error> {
    let mut value = serde_json::to_value(schema)?;
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "$id".to_owned(),
            serde_json::Value::String(schema_id.to_owned()),
        );
    }
    let mut output = serde_json::to_string_pretty(&value)?;
    output.push('\n');
    Ok((file_name, output))
}
