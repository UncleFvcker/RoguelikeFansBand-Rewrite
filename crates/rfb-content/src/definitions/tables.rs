// SPDX-License-Identifier: MPL-2.0

#[cfg(feature = "schemas")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{ContentPosition, TerrainOverride};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ItemQuality {
    #[default]
    Ordinary,
    Fine,
    Exceptional,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LootEntryDefinition {
    pub item_kind_id: String,
    pub weight: u32,
    pub quantity: u32,
    #[serde(default)]
    pub min_depth: u16,
    #[serde(default = "maximum_depth")]
    pub max_depth: u16,
}

const fn maximum_depth() -> u16 {
    u16::MAX
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LootRollDiceDefinition {
    pub dice: u16,
    pub sides: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LootQualityWeightDefinition {
    pub quality: ItemQuality,
    pub weight: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LootAffixWeightDefinition {
    #[serde(default)]
    pub affix_id: Option<String>,
    pub weight: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LootTableDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub rolls: u16,
    #[serde(default)]
    pub roll_chance_percent: Option<u8>,
    #[serde(default)]
    pub roll_dice: Option<LootRollDiceDefinition>,
    pub entries: Vec<LootEntryDefinition>,
    pub quality_weights: Vec<LootQualityWeightDefinition>,
    pub affix_weights: Vec<LootAffixWeightDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncounterTableDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub rolls: u16,
    #[serde(default)]
    pub global_allocation: Option<GlobalMonsterAllocationDefinition>,
    #[serde(default)]
    pub entries: Vec<EncounterEntryDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GlobalMonsterAllocationDefinition {
    pub preferred_glyphs: Vec<String>,
    /// Weight numerator over the original fixed denominator of 64.
    pub special_div: u8,
    pub ambient_chance_one_in: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncounterEntryDefinition {
    pub actor_kind_id: String,
    pub weight: u32,
    pub min_depth: u16,
    pub max_depth: u16,
    #[serde(default)]
    pub group: Option<EncounterGroupDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncounterGroupDefinition {
    #[serde(default)]
    pub friends: Option<EncounterFriendsDefinition>,
    #[serde(default)]
    pub escort: Option<EncounterEscortDefinition>,
    pub formation: EncounterFormation,
    #[serde(default)]
    pub pack_ai: EncounterPackAiDefinition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncounterPackAiDefinition {
    pub leader: MonsterPackBehavior,
    pub friends: MonsterPackBehavior,
    pub escorts: MonsterPackBehavior,
}

impl Default for EncounterPackAiDefinition {
    fn default() -> Self {
        Self {
            leader: MonsterPackBehavior::Seek,
            friends: MonsterPackBehavior::Surround,
            escorts: MonsterPackBehavior::GuardLeader,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum MonsterPackBehavior {
    #[default]
    Seek,
    Surround,
    GuardLeader,
    GuardPosition,
    Lure,
    Shoot,
    MaintainDistance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncounterFriendsDefinition {
    pub min_count: u16,
    pub max_count: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncounterEscortDefinition {
    pub min_count: u16,
    pub max_count: u16,
    pub entries: Vec<EncounterEscortEntryDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncounterEscortEntryDefinition {
    pub actor_kind_id: String,
    pub weight: u32,
    pub min_depth: u16,
    pub max_depth: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum EncounterFormation {
    Cluster,
    Ring,
}

impl EncounterGroupDefinition {
    #[must_use]
    pub fn min_companion_count(&self) -> u16 {
        self.friends
            .as_ref()
            .map_or(0, |friends| friends.min_count)
            .saturating_add(self.escort.as_ref().map_or(0, |escort| escort.min_count))
    }

    #[must_use]
    pub fn max_companion_count(&self) -> u16 {
        self.friends
            .as_ref()
            .map_or(0, |friends| friends.max_count)
            .saturating_add(self.escort.as_ref().map_or(0, |escort| escort.max_count))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeTableDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub entries: Vec<ThemeEntryDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeEntryDefinition {
    pub theme_id: String,
    pub floor_terrain_id: String,
    pub weight: u32,
    pub min_depth: u16,
    pub max_depth: u16,
    #[serde(default)]
    pub vault_candidates: Vec<ThemeVaultCandidateDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeVaultCandidateDefinition {
    pub vault_id: String,
    pub weight: u32,
    pub min_depth: u16,
    pub max_depth: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegionTableDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub entries: Vec<RegionEntryDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegionEntryDefinition {
    pub region_id: String,
    pub theme_table_id: String,
    pub theme_id: String,
    pub encounter_table_id: String,
    pub loot_table_id: String,
    pub weight: u32,
    pub min_depth: u16,
    pub max_depth: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerrainFeatureTableDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub rolls: u16,
    pub entries: Vec<TerrainFeatureEntryDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerrainFeatureEntryDefinition {
    pub terrain_id: String,
    pub placement: TerrainFeaturePlacement,
    pub weight: u32,
    pub min_depth: u16,
    pub max_depth: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum TerrainFeaturePlacement {
    Room,
    Corridor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub name_key: String,
    pub theme_id: String,
    pub width: u16,
    pub height: u16,
    pub base_terrain_id: String,
    #[serde(default)]
    pub entrance_position: Option<ContentPosition>,
    #[serde(default)]
    pub entrance_positions: Vec<ContentPosition>,
    #[serde(default)]
    pub transforms: Vec<VaultTransform>,
    pub terrain_overrides: Vec<TerrainOverride>,
    pub encounter_groups: Vec<VaultEncounterGroupDefinition>,
    pub loot_spawns: Vec<VaultLootSpawnDefinition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum VaultTransform {
    Identity,
    Rotate90,
    Rotate180,
    Rotate270,
    MirrorHorizontal,
    MirrorVertical,
    MirrorMainDiagonal,
    MirrorAntiDiagonal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultEncounterGroupDefinition {
    pub id: String,
    pub member_positions: Vec<ContentPosition>,
    pub entries: Vec<VaultEncounterEntryDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultEncounterEntryDefinition {
    pub actor_kind_id: String,
    pub weight: u32,
    pub min_depth: u16,
    pub max_depth: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultLootSpawnDefinition {
    pub id: String,
    pub position: ContentPosition,
    pub loot_table_id: String,
}
