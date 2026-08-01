// SPDX-License-Identifier: MPL-2.0

use std::path::PathBuf;

use thiserror::Error;

mod ability_programs;
mod artifact;
mod catalog;
mod definitions;
mod effect_programs;
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

#[derive(Debug, Error)]
pub enum ContentError {
    #[error("content pack root is invalid or is a symlink: {0}")]
    InvalidPackRoot(PathBuf),
    #[error("content directory is invalid or is a symlink: {0}")]
    InvalidContentDirectory(PathBuf),
    #[error("content entry must be a regular .json file: {0}")]
    InvalidContentFile(PathBuf),
    #[error("content source file exceeds the 1 MiB limit: {0}")]
    SourceFileTooLarge(PathBuf),
    #[error("content source pack exceeds the 16 MiB limit: {0} bytes")]
    SourcePackTooLarge(usize),
    #[error("content source pack exceeds the file-count limit: {0}")]
    TooManySourceFiles(usize),
    #[error("invalid JSON in {path}: {source}")]
    InvalidJson {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("schema identifier does not match for {0}")]
    SchemaMismatch(String),
    #[error("unsupported source format version {version} in {owner}")]
    UnsupportedSourceVersion { owner: String, version: u16 },
    #[error("unsupported content root {0}")]
    UnsupportedContentRoot(String),
    #[error("duplicate content root {0}")]
    DuplicateContentRoot(String),
    #[error("invalid stable content ID {0}")]
    InvalidStableId(String),
    #[error("content ID {id} must use category {expected}")]
    WrongIdCategory { id: String, expected: String },
    #[error("invalid pack semantic version {0}")]
    InvalidPackVersion(String),
    #[error("invalid dependency {0}")]
    InvalidDependency(String),
    #[error("invalid dependency version requirement {0}")]
    InvalidVersionRequirement(String),
    #[error("invalid loadAfter entry {0}")]
    InvalidLoadAfter(String),
    #[error("invalid localization message key {0}")]
    InvalidMessageKey(String),
    #[error("definition name or description key is invalid: {0}")]
    InvalidDefinitionText(String),
    #[error("definition glyph must contain one non-control Unicode scalar: {0}")]
    InvalidGlyph(String),
    #[error("terrain open/close transition is invalid: {0}")]
    InvalidTerrainTransition(String),
    #[error("invalid tag {tag} in {id}")]
    InvalidTag { id: String, tag: String },
    #[error("duplicate tag in {0}")]
    DuplicateTag(String),
    #[error("duplicate definition ID {0}")]
    DuplicateDefinitionId(String),
    #[error("actor stats are outside supported limits: {0}")]
    InvalidActorStats(String),
    #[error("actor carry capacity is invalid for its role: {0}")]
    InvalidActorCarryCapacity(String),
    #[error("actor melee routine is invalid or requires the monster role: {0}")]
    InvalidMeleeRoutine(String),
    #[error("actor monster casting profile is invalid or references an unsupported ability: {0}")]
    InvalidMonsterCasting(String),
    #[error("actor loot table reference is invalid or requires the monster role: {0}")]
    InvalidActorLootTable(String),
    #[error("item stack limit is outside supported limits: {0}")]
    InvalidItemStack(String),
    #[error("item weight is outside supported limits: {0}")]
    InvalidItemWeight(String),
    #[error("item appearance must use a distinct valid message key: {0}")]
    InvalidItemAppearance(String),
    #[error("item break chance is outside 0..=100 percent: {0}")]
    InvalidItemBreakChance(String),
    #[error("item equipment slot is invalid or requires maxStack 1: {0}")]
    InvalidEquipmentSlot(String),
    #[error("race body slots are invalid: {0}")]
    InvalidBodySlots(String),
    #[error("status immunity list is invalid: {0}")]
    InvalidStatusImmunities(String),
    #[error("item stat modifiers are invalid or require an equipment slot: {0}")]
    InvalidItemModifiers(String),
    #[error("item attack profile is invalid or requires the weapon slot: {0}")]
    InvalidAttackProfile(String),
    #[error("item projectile profile is invalid or requires the launcher slot: {0}")]
    InvalidProjectileProfile(String),
    #[error("item throw profile is invalid: {0}")]
    InvalidThrowProfile(String),
    #[error("item use action is invalid: {0}")]
    InvalidItemUseAction(String),
    #[error("effect program definition is invalid: {0}")]
    InvalidEffectProgram(String),
    #[error("ability program definition is invalid: {0}")]
    InvalidAbilityProgram(String),
    #[error("player ability binding is duplicated for ability: {0}")]
    DuplicatePlayerAbilityBinding(String),
    #[error("player ability binding definition is invalid: {0}")]
    InvalidPlayerAbilityBinding(String),
    #[error("resource definition is invalid: {0}")]
    InvalidResource(String),
    #[error("ability definition is invalid: {0}")]
    InvalidAbility(String),
    #[error("ability book definition is invalid: {0}")]
    InvalidAbilityBook(String),
    #[error("ability book item must be a single non-equippable, non-usable item: {0}")]
    InvalidAbilityBookItem(String),
    #[error("class casting profile is invalid: {0}")]
    InvalidCastingProfile(String),
    #[error("class technique profile is invalid: {0}")]
    InvalidTechniqueProfile(String),
    #[error("class device recharge profile is invalid: {0}")]
    InvalidDeviceRechargeProfile(String),
    #[error("affix stat modifiers are invalid: {0}")]
    InvalidAffixModifiers(String),
    #[error("skill definition is invalid: {0}")]
    InvalidSkill(String),
    #[error("content rule requires a missing skill kind: {0}")]
    MissingRequiredSkillKind(String),
    #[error("skill set definition is invalid: {0}")]
    InvalidSkillSet(String),
    #[error("race, class, or personality definition is invalid: {0}")]
    InvalidCharacterSource(String),
    #[error("starting item definition is invalid: {0}")]
    InvalidStartingItems(String),
    #[error("character build definition is invalid: {0}")]
    InvalidCharacterBuild(String),
    #[error("loot table weights, entries, or generated item constraints are invalid: {0}")]
    InvalidLootTable(String),
    #[error("encounter table weights, depth ranges, or actor entries are invalid: {0}")]
    InvalidEncounterTable(String),
    #[error("theme table weights, depth ranges, terrain, or vault candidates are invalid: {0}")]
    InvalidThemeTable(String),
    #[error("region table weights, depth ranges, or local table references are invalid: {0}")]
    InvalidRegionTable(String),
    #[error("terrain feature table weights, depth ranges, terrain, or placements are invalid: {0}")]
    InvalidTerrainFeatureTable(String),
    #[error("vault terrain, encounters, or loot definition is invalid: {0}")]
    InvalidVault(String),
    #[error("world dimensions are outside supported limits: {0}")]
    InvalidWorldDimensions(String),
    #[error("procedural floor definition is invalid: {0}")]
    InvalidProceduralFloor(String),
    #[error("content reference from {owner} to {target} is unresolved")]
    DanglingReference { owner: String, target: String },
    #[error("actor has the wrong role for this spawn: {0}")]
    WrongActorRole(String),
    #[error("duplicate runtime instance ID {0}")]
    DuplicateInstanceId(String),
    #[error("two actors occupy the same world position: {0}")]
    DuplicateActorPosition(String),
    #[error("content position is outside world bounds: {0}")]
    PositionOutOfBounds(String),
    #[error("world spawn is placed on non-walkable terrain: {0}")]
    SpawnOnBlockedTerrain(String),
    #[error("terrain override is duplicated or touches the generated border: {0}")]
    InvalidTerrainOverride(String),
    #[error("item spawn quantity is invalid: {0}")]
    InvalidItemQuantity(String),
    #[error("item spawn affix references are invalid: {0}")]
    InvalidItemAffixes(String),
    #[error("compiled content metadata is invalid")]
    InvalidCompiledMetadata,
    #[error("compiled content payload exceeds the 32 MiB limit: {0} bytes")]
    CompiledPayloadTooLarge(usize),
    #[error("compiled content container is invalid or truncated")]
    InvalidContainer,
    #[error("unsupported compiled content container version {0}")]
    UnsupportedContainerVersion(u16),
    #[error("unsupported compiled content container flags 0x{0:04x}")]
    UnsupportedContainerFlags(u16),
    #[error("compiled content checksum does not match")]
    ChecksumMismatch,
    #[error("compiled content is not in canonical sorted form")]
    NonCanonicalCompiledContent,
    #[error("content.lock.json does not match the deterministic compiled pack")]
    ContentLockMismatch,
    #[error("content length overflow")]
    LengthOverflow,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("MessagePack encode error: {0}")]
    MessagePackEncode(#[from] rmp_serde::encode::Error),
    #[error("MessagePack decode error: {0}")]
    MessagePackDecode(#[from] rmp_serde::decode::Error),
}

#[cfg(test)]
mod tests;
