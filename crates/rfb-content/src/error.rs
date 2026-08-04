// SPDX-License-Identifier: MPL-2.0

use std::path::PathBuf;

use thiserror::Error;

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
    #[error("terrain glyph must not use an ASCII letter reserved for actors: {0}")]
    InvalidTerrainGlyph(String),
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
    #[error("item base value is outside supported limits: {0}")]
    InvalidItemValue(String),
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
    #[error("item fuel definition is invalid: {0}")]
    InvalidItemFuel(String),
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
    #[error("town definition or ownership is invalid: {0}")]
    InvalidTown(String),
    #[error("shop definition or entrance is invalid: {0}")]
    InvalidShop(String),
    #[error("town facility definition or entrance is invalid: {0}")]
    InvalidTownFacility(String),
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
