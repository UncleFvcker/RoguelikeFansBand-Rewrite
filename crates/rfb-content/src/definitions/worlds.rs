// SPDX-License-Identifier: MPL-2.0

#[cfg(feature = "schemas")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{ActorDamageType, ActorMovementMode, ItemQuality};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerrainDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub name_key: String,
    pub description_key: String,
    pub glyph: String,
    pub walkable: bool,
    pub blocks_sight: bool,
    #[serde(default)]
    pub allows_wall_passage: bool,
    #[serde(default)]
    pub open_to_terrain_id: Option<String>,
    #[serde(default)]
    pub open_check_difficulty: Option<i32>,
    #[serde(default)]
    pub close_to_terrain_id: Option<String>,
    #[serde(default)]
    pub bash_to_terrain_id: Option<String>,
    #[serde(default)]
    pub bash_check_difficulty: Option<i32>,
    #[serde(default)]
    pub monster_door_power: Option<u16>,
    #[serde(default)]
    pub monster_unlock_to_terrain_id: Option<String>,
    #[serde(default)]
    pub dig_to_terrain_id: Option<String>,
    #[serde(default)]
    pub dig_check_difficulty: Option<i32>,
    /// Result of the original `FF_HURT_DISI` terrain transition.
    #[serde(default)]
    pub monster_destroy_to_terrain_id: Option<String>,
    #[serde(default)]
    pub concealed_as_terrain_id: Option<String>,
    #[serde(default)]
    pub search_check_difficulty: Option<i32>,
    #[serde(default)]
    pub perception_check_difficulty: Option<i32>,
    #[serde(default)]
    pub trap: Option<TerrainTrapDefinition>,
    #[serde(default)]
    pub movement_modes: Vec<ActorMovementMode>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerrainTrapDefinition {
    pub damage: i32,
    #[serde(default)]
    pub damage_type: ActorDamageType,
    pub disarm_to_terrain_id: String,
    pub disarm_check_difficulty: i32,
    #[serde(default)]
    pub saving_throw_difficulty: Option<i32>,
    #[serde(default)]
    pub avoided_by_movement_modes: Vec<ActorMovementMode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentPosition {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerrainOverride {
    pub terrain_id: String,
    pub positions: Vec<ContentPosition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorSpawn {
    pub instance_id: String,
    pub kind_id: String,
    pub position: ContentPosition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemSpawn {
    pub instance_id: String,
    pub kind_id: String,
    pub position: ContentPosition,
    pub quantity: u32,
    #[serde(default)]
    pub quality: ItemQuality,
    #[serde(default)]
    pub affix_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorldDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub name_key: String,
    pub initial_floor_id: String,
    #[serde(default)]
    pub town_id: Option<String>,
    #[serde(default)]
    pub wilderness: Option<WildernessDefinition>,
    pub width: u16,
    pub height: u16,
    pub fill_terrain_id: String,
    pub border_terrain_id: String,
    pub terrain_overrides: Vec<TerrainOverride>,
    pub player: ActorSpawn,
    #[serde(default)]
    pub player_build_id: Option<String>,
    pub actors: Vec<ActorSpawn>,
    pub items: Vec<ItemSpawn>,
    #[serde(default)]
    pub surface_actor_allocation: Option<SurfaceActorAllocationDefinition>,
    #[serde(default)]
    pub dungeons: Vec<DungeonDefinition>,
    #[serde(default)]
    pub campaign: Option<CampaignDefinition>,
    #[serde(default)]
    pub tasks: Vec<TaskDefinition>,
    pub procedural_floors: Vec<ProceduralFloorDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WildernessDefinition {
    pub width: u16,
    pub height: u16,
    pub start_position: ContentPosition,
    pub legend: Vec<WildernessLegendEntry>,
    pub rows: Vec<String>,
    pub locations: Vec<WildernessLocationDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WildernessLegendEntry {
    pub symbol: String,
    pub terrain: WildernessTerrain,
    pub level: u16,
    pub road: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum WildernessTerrain {
    Edge,
    Town,
    DeepWater,
    ShallowWater,
    Swamp,
    Dirt,
    Grass,
    Trees,
    Desert,
    ShallowLava,
    DeepLava,
    Mountain,
    Glacier,
    Snow,
    PackIce,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum WildernessLocationDefinition {
    Town {
        position: ContentPosition,
        town_id: String,
    },
    Dungeon {
        position: ContentPosition,
        dungeon_id: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SurfaceActorAllocationDefinition {
    #[cfg_attr(feature = "schemas", schemars(range(min = 1, max = 64)))]
    pub rolls: u16,
    #[cfg_attr(feature = "schemas", schemars(range(min = 1, max = 100)))]
    pub level: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CampaignDefinition {
    pub victory_dungeon_ids: Vec<String>,
    pub dungeon_conquest_points: u32,
    pub task_completion_points: u32,
    pub victory_bonus: u32,
    pub turn_penalty_interval: u32,
    pub turn_penalty_points: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DungeonDefinition {
    pub id: String,
    #[serde(default)]
    pub legacy_index: Option<u16>,
    pub root_floor_id: String,
    pub guardian_actor_kind_id: String,
    #[serde(default)]
    pub instance_lifecycle: DungeonInstanceLifecycle,
    #[serde(default)]
    pub entrance_guardian: Option<DungeonEntranceGuardianDefinition>,
    #[serde(default)]
    pub entry_requirements: Vec<DungeonEntryRequirementDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[derive(Default)]
pub enum DungeonInstanceLifecycle {
    #[default]
    ResetOnSurface,
    Persistent,
    TurnTtl {
        #[cfg_attr(feature = "schemas", schemars(range(min = 1)))]
        ttl_turns: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DungeonEntranceGuardianDefinition {
    pub instance_id: String,
    pub actor_kind_id: String,
    pub position: ContentPosition,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum DungeonEntryRequirementDefinition {
    TaskStatus {
        task_id: String,
        status: DungeonEntryTaskStatus,
    },
    DungeonConquered {
        dungeon_id: String,
    },
    CarriedItem {
        item_kind_id: String,
        quantity: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum DungeonEntryTaskStatus {
    Available,
    Active,
    Paused,
    Completed,
    Failed,
    Abandoned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralFloorDefinition {
    pub id: String,
    pub name_key: String,
    pub return_floor_id: String,
    #[serde(default)]
    pub lifecycle: FloorLifecycle,
    #[serde(default)]
    pub dungeon_id: Option<String>,
    #[serde(default)]
    pub final_floor: bool,
    #[serde(default)]
    pub guardian: Option<DungeonGuardianDefinition>,
    #[serde(default)]
    pub theme_id: Option<String>,
    #[serde(default)]
    pub vault_id: Option<String>,
    #[serde(default)]
    pub encounter_table_id: Option<String>,
    #[serde(default)]
    pub loot_table_id: Option<String>,
    #[serde(default)]
    pub loot_allocation: Option<ProceduralLootAllocationDefinition>,
    #[serde(default)]
    pub gold_allocation: Option<ProceduralGoldAllocationDefinition>,
    #[serde(default)]
    pub guaranteed_items: Vec<ProceduralGuaranteedItemDefinition>,
    #[serde(default)]
    pub theme_table_id: Option<String>,
    #[serde(default)]
    pub region_table_id: Option<String>,
    #[serde(default)]
    pub terrain_feature_table_id: Option<String>,
    #[serde(default)]
    pub layout: Option<ProceduralLayoutDefinition>,
    #[serde(default)]
    pub inline_map: Option<InlineFloorMapDefinition>,
    #[serde(default)]
    pub generation_budget: Option<ProceduralGenerationBudgetDefinition>,
    #[serde(default)]
    pub nest: Option<ProceduralNestDefinition>,
    #[serde(default)]
    pub entry_terrain_id: Option<String>,
    #[serde(default)]
    pub available_entry_terrain_id: Option<String>,
    #[serde(default)]
    pub entry_connection_id: Option<String>,
    #[serde(default)]
    pub completed_entry_terrain_id: Option<String>,
    #[serde(default)]
    pub failed_entry_terrain_id: Option<String>,
    #[serde(default)]
    pub abandoned_entry_terrain_id: Option<String>,
    #[serde(default = "default_allow_early_task_exit")]
    pub allow_early_task_exit: bool,
    #[serde(default)]
    pub retakeable: bool,
    #[serde(default)]
    #[cfg_attr(feature = "schemas", schemars(range(min = 1, max = 16)))]
    pub max_retakes: Option<u16>,
    #[serde(default)]
    pub retake_floor_policy: RetakeFloorPolicy,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub next_floor_id: Option<String>,
    #[serde(default)]
    pub connections: Vec<ProceduralFloorConnectionDefinition>,
    pub depth: u16,
    pub width: u16,
    pub height: u16,
    pub wall_terrain_id: String,
    pub floor_terrain_id: String,
    pub up_stair_terrain_id: String,
    #[serde(default)]
    pub down_stair_terrain_id: Option<String>,
    pub closed_door_terrain_id: String,
    pub trap_terrain_id: String,
    pub actor_spawns: Vec<ProceduralActorSpawnDefinition>,
    pub loot_spawns: Vec<ProceduralLootSpawnDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InlineFloorMapDefinition {
    pub player_position: ContentPosition,
    pub terrain_overrides: Vec<InlineTerrainOverrideDefinition>,
    #[serde(default)]
    pub actor_spawns: Vec<ActorSpawn>,
    #[serde(default)]
    pub loot_spawns: Vec<InlineFloorLootSpawnDefinition>,
    #[serde(default)]
    pub monster_formation: Option<InlineMonsterFormationDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InlineTerrainOverrideDefinition {
    pub terrain_id: String,
    pub positions: Vec<ContentPosition>,
    #[serde(default = "default_terrain_override_chance_percent")]
    pub chance_percent: u8,
    #[serde(default)]
    pub otherwise_terrain_id: Option<String>,
}

const fn default_terrain_override_chance_percent() -> u8 {
    100
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InlineFloorLootSpawnDefinition {
    pub id: String,
    pub position: ContentPosition,
    pub loot_table_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InlineMonsterFormationDefinition {
    pub candidate_actor_kind_ids: Vec<String>,
    pub draw_count: u16,
    pub placement_indices: Vec<u16>,
    pub positions: Vec<ContentPosition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralFloorConnectionDefinition {
    pub id: String,
    pub kind: FloorConnectionKind,
    pub terrain_id: String,
    pub target_floor_id: String,
    #[serde(default)]
    pub target_connection_id: Option<String>,
    #[serde(default)]
    pub target_candidates: Vec<ProceduralFloorConnectionCandidateDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralFloorConnectionCandidateDefinition {
    pub target_floor_id: String,
    pub target_connection_id: String,
    #[cfg_attr(feature = "schemas", schemars(range(min = 1)))]
    pub weight: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum FloorConnectionKind {
    Stairs,
    Shaft,
}

const fn default_allow_early_task_exit() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DungeonGuardianDefinition {
    pub instance_id: String,
    pub actor_kind_id: String,
    #[serde(default)]
    pub reward_loot_table_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralNormalAllocationDefinition {
    pub mean: u16,
    pub standard_deviation: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralLootAllocationDefinition {
    pub reference_area_tiles: u32,
    pub room_objects: ProceduralNormalAllocationDefinition,
    pub anywhere_objects: ProceduralNormalAllocationDefinition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralGoldAllocationDefinition {
    pub reference_area_tiles: u32,
    pub piles: ProceduralNormalAllocationDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralGuaranteedItemDefinition {
    pub id: String,
    #[cfg_attr(feature = "schemas", schemars(range(min = 2, max = 1000)))]
    pub chance_one_in: u16,
    pub entries: Vec<ProceduralGuaranteedItemEntryDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralGuaranteedItemEntryDefinition {
    pub item_kind_id: String,
    #[cfg_attr(feature = "schemas", schemars(range(min = 1, max = 1000)))]
    pub weight: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskObjectiveDefinition {
    pub kind: TaskObjectiveKind,
    #[serde(default)]
    pub floor_id: Option<String>,
    #[serde(default = "default_task_objective_required")]
    pub required: u32,
    #[serde(default)]
    pub item_instance_id: Option<String>,
    #[serde(default)]
    pub item_kind_id: Option<String>,
    #[serde(default)]
    pub actor_instance_id: Option<String>,
    #[serde(default)]
    pub actor_kind_id: Option<String>,
}

const fn default_task_objective_required() -> u32 {
    1
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum TaskObjectiveKind {
    ClearFloor,
    CollectItem,
    EnterFloor,
    KillActor,
    KillActorKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskDefinition {
    pub id: String,
    pub name_key: String,
    pub description_key: String,
    #[serde(default)]
    pub source_facility_id: Option<String>,
    #[serde(default)]
    pub prerequisite_task_id: Option<String>,
    pub location: TaskLocationDefinition,
    pub objectives: Vec<TaskObjectiveDefinition>,
    #[serde(default)]
    pub target_placements: Vec<TaskTargetPlacementDefinition>,
    #[serde(default)]
    pub completion_exit_terrain_id: Option<String>,
    pub reward: TaskRewardDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TaskLocationDefinition {
    DedicatedFloors { floor_ids: Vec<String> },
    DungeonDepth { dungeon_id: String, depth: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskTargetPlacementDefinition {
    pub objective_index: u32,
    pub floor_id: String,
    pub spawn_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskRewardDefinition {
    pub item_instance_id: String,
    pub item_kind_id: String,
    pub quantity: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum RetakeFloorPolicy {
    #[default]
    PreserveFloor,
    RegenerateFloor,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum FloorLifecycle {
    #[default]
    Dungeon,
    OneShot,
    Town,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralActorSpawnDefinition {
    pub instance_id: String,
    pub room_id: String,
    pub actor_kind_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralLootSpawnDefinition {
    pub id: String,
    pub room_id: String,
    pub loot_table_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralGenerationBudgetDefinition {
    pub actor_slots: u16,
    pub loot_placements: u16,
    #[serde(default)]
    pub region_placements: Option<u16>,
    #[serde(default)]
    pub room_placements: Option<u16>,
    #[serde(default)]
    pub room_area_tiles: Option<u32>,
    #[serde(default)]
    pub cavern_area_tiles: Option<u32>,
    #[serde(default)]
    pub lake_area_tiles: Option<u32>,
    #[serde(default)]
    pub lake_deep_area_tiles: Option<u32>,
    #[serde(default)]
    pub river_area_tiles: Option<u32>,
    #[serde(default)]
    pub maze_floor_tiles: Option<u32>,
    #[serde(default)]
    pub destruction_centers: Option<u16>,
    #[serde(default)]
    pub destroyed_area_tiles: Option<u32>,
    #[serde(default)]
    pub streamer_placements: Option<u16>,
    #[serde(default)]
    pub streamer_area_tiles: Option<u32>,
    #[serde(default)]
    pub pit_placements: Option<u16>,
    #[serde(default)]
    pub pit_actor_slots: Option<u16>,
    #[serde(default)]
    pub vault_placements: Option<u16>,
    #[serde(default)]
    pub vault_area_tiles: Option<u32>,
    #[serde(default)]
    pub group_placements: Option<u16>,
    #[serde(default)]
    pub group_actor_slots: Option<u16>,
    #[serde(default)]
    pub feature_placements: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralRoomGeometryDefinition {
    #[serde(default)]
    pub placement: ProceduralRoomPlacement,
    pub min_width: u16,
    pub max_width: u16,
    pub min_height: u16,
    pub max_height: u16,
    pub shapes: Vec<ProceduralRoomShapeCandidateDefinition>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ProceduralRoomPlacement {
    #[default]
    Partitioned,
    Free,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralLayoutDefinition {
    #[serde(default)]
    pub mode: ProceduralLayoutMode,
    #[serde(default)]
    pub rooms: Option<ProceduralRoomGeometryDefinition>,
    #[serde(default)]
    pub cavern: Option<ProceduralCavernDefinition>,
    #[serde(default)]
    pub lake: Option<ProceduralLakeDefinition>,
    #[serde(default)]
    pub river: Option<ProceduralRiverDefinition>,
    #[serde(default)]
    pub maze: Option<ProceduralMazeDefinition>,
    #[serde(default)]
    pub destroyed: Option<ProceduralDestroyedDefinition>,
    #[serde(default)]
    pub streamers: Vec<ProceduralStreamerCandidateDefinition>,
    #[serde(default)]
    pub pit: Option<ProceduralPitDefinition>,
    #[serde(default)]
    pub stairs: Option<ProceduralStairLayoutDefinition>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ProceduralLayoutMode {
    #[default]
    Rooms,
    MazeOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralPitDefinition {
    pub encounter_table_id: String,
    pub inner_width: u16,
    pub inner_height: u16,
    pub roster_size: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralCavernDefinition {
    pub terrain_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralLakeDefinition {
    pub deep_terrain_id: String,
    pub shallow_terrain_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralRiverDefinition {
    pub deep_terrain_id: String,
    pub shallow_terrain_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralMazeDefinition {
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralDestroyedDefinition {
    pub terrain_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralStreamerCandidateDefinition {
    pub terrain_id: String,
    pub weight: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralRoomShapeCandidateDefinition {
    pub shape: ProceduralRoomShape,
    pub weight: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ProceduralRoomShape {
    Rectangle,
    Cross,
    Cavern,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralStairLayoutDefinition {
    pub up: ProceduralCountRangeDefinition,
    #[serde(default)]
    pub down: Option<ProceduralCountRangeDefinition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralCountRangeDefinition {
    pub minimum: u16,
    pub maximum: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralNestDefinition {
    pub room_id: String,
    pub spawn_count: u16,
}
