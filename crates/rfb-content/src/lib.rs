// SPDX-License-Identifier: MPL-2.0

mod ability_programs;
mod artifact;
mod catalog;
mod definitions;
mod effect_programs;
mod error;
mod player_ability_bindings;
#[cfg(feature = "schemas")]
mod schemas;
mod source;
mod validation;

#[cfg(feature = "schemas")]
use ability_programs::SourceAbilityDefinition;
pub use ability_programs::{
    AbilityProgramDefinition, AbilityProgramInputDefinition, AbilityProgramStepDefinition,
};
pub use artifact::{CompiledArtifact, decode_content, encode_content, read_compiled_file};
pub use catalog::{CompiledContentV1, ContentCatalog, ContentLockV1, ContentSummary};
pub(crate) use definitions::valid_ability_level_scaling;
pub use definitions::*;
pub use effect_programs::{
    EffectProgramDefinition, EffectProgramInputDefinition, EffectProgramStepDefinition,
};
pub use error::ContentError;
pub use player_ability_bindings::PlayerAbilityBindingDefinition;
#[cfg(feature = "schemas")]
pub use schemas::generated_schema_documents;
#[cfg(feature = "schemas")]
use source::SourceItemDefinition;
pub use source::{compile_pack_dir, verify_pack_lock};

pub const CONTENT_FORMAT: &str = "rfb-content";
pub const CONTENT_FORMAT_VERSION: u16 = 1;
pub const PACK_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/pack.schema.json";
pub const TERRAIN_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/terrain.schema.json";
pub const ACTOR_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/actor.schema.json";
pub const ITEM_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/item.schema.json";
pub const EFFECT_PROGRAM_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/effect-program.schema.json";
pub const AFFIX_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/affix.schema.json";
pub const ENCOUNTER_TABLE_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/encounter-table.schema.json";
pub const LOOT_TABLE_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/loot-table.schema.json";
pub const THEME_TABLE_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/theme-table.schema.json";
pub const REGION_TABLE_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/region-table.schema.json";
pub const TERRAIN_FEATURE_TABLE_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/terrain-feature-table.schema.json";
pub const VAULT_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/vault.schema.json";
pub const WORLD_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/world.schema.json";
pub const SKILL_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/skill.schema.json";
pub const SKILL_SET_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/skill-set.schema.json";
pub const RACE_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/race.schema.json";
pub const CLASS_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/class.schema.json";
pub const PERSONALITY_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/personality.schema.json";
pub const BUILD_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/build.schema.json";
pub const RESOURCE_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/resource.schema.json";
pub const ABILITY_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/ability.schema.json";
pub const ABILITY_PROGRAM_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/ability-program.schema.json";
pub const PLAYER_ABILITY_BINDING_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/player-ability-binding.schema.json";
pub const ABILITY_BOOK_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/ability-book.schema.json";

#[cfg(test)]
mod tests;
