// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeMap;

#[cfg(feature = "bindings")]
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;
#[cfg(feature = "bindings")]
use ts_rs::{Config, TS};

pub const PROTOCOL_VERSION: &str = "1.52";

const fn default_actor_speed() -> u16 {
    110
}

const fn default_monster_energy_need() -> i32 {
    100
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum Direction {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl Direction {
    #[must_use]
    pub const fn delta(self) -> (i32, i32) {
        match self {
            Self::North => (0, -1),
            Self::NorthEast => (1, -1),
            Self::East => (1, 0),
            Self::SouthEast => (1, 1),
            Self::South => (0, 1),
            Self::SouthWest => (-1, 1),
            Self::West => (-1, 0),
            Self::NorthWest => (-1, -1),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum GameCommand {
    AbandonTask,
    Appraise {
        item_id: String,
    },
    BashDoor {
        direction: Direction,
    },
    CloseDoor {
        direction: Direction,
    },
    DisarmTrap {
        direction: Direction,
    },
    DigTerrain {
        direction: Direction,
    },
    Drop {
        item_ids: Vec<String>,
    },
    DropQuantity {
        item_id: String,
        quantity: u32,
    },
    Equip {
        item_id: String,
    },
    Fire {
        direction: Direction,
    },
    FireTarget {
        target: TargetSelection,
    },
    Move {
        direction: Direction,
    },
    OpenDoor {
        direction: Direction,
    },
    PickUp,
    Search,
    Throw {
        item_id: String,
        direction: Direction,
    },
    TraverseStairs,
    UseItem {
        item_id: String,
    },
    Unequip {
        slot_id: String,
    },
    Wait,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct StatModifiersDto {
    #[serde(default)]
    pub attack: i32,
    #[serde(default)]
    pub defense: i32,
    #[serde(default)]
    pub max_hp: i32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct DamageDiceDto {
    pub dice: u16,
    pub sides: u16,
    #[serde(default)]
    pub damage_type: DamageTypeDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct AttackProfileDto {
    pub attacks: u16,
    pub to_hit: i32,
    pub to_damage: i32,
    pub damage: DamageDiceDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_item_id: Option<String>,
}

impl Default for AttackProfileDto {
    fn default() -> Self {
        Self {
            attacks: 1,
            to_hit: 0,
            to_damage: 0,
            damage: DamageDiceDto::default(),
            source_item_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct MeleeBlowDto {
    pub method_id: String,
    pub to_hit: i32,
    pub damage: DamageDiceDto,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct MeleeRoutineDto {
    pub blows: Vec<MeleeBlowDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum TargetModeDto {
    Direction,
    Position,
    Entity,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct TargetSpecDto {
    pub modes: Vec<TargetModeDto>,
    pub range: u16,
    pub requires_line_of_effect: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum TargetSelection {
    Direction { direction: Direction },
    Position { position: Position },
    Entity { entity_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ProjectileProfileDto {
    pub range: u16,
    pub to_hit: i32,
    pub to_damage: i32,
    pub damage: DamageDiceDto,
    #[serde(default)]
    pub ammo_kind_id: String,
    #[serde(default)]
    pub target_spec: TargetSpecDto,
    pub source_item_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ThrowProfileDto {
    pub range: u16,
    pub to_hit: i32,
    pub to_damage: i32,
    pub damage: DamageDiceDto,
    pub source_item_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ProjectileTraceDto {
    pub origin: Position,
    pub impact: Position,
    #[serde(default)]
    pub landing: Position,
    pub traversed: Vec<Position>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct GameCommandEnvelope {
    pub command_seq: u32,
    pub expected_revision: u32,
    pub command: GameCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum TerrainInteractionKindDto {
    OpenDoor,
    CloseDoor,
    BashDoor,
    DisarmTrap,
    DigTerrain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum TerrainInteractionUnavailableReasonDto {
    OccupiedByActor,
    OccupiedByItem,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct TerrainInteractionDto {
    pub kind: TerrainInteractionKindDto,
    pub direction: Direction,
    pub position: Position,
    pub terrain_id: String,
    pub requires_check: bool,
    pub available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<TerrainInteractionUnavailableReasonDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum TaskStatusKindDto {
    Abandoned,
    Available,
    Active,
    Completed,
    Failed,
    Paused,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct TaskStatusDto {
    #[serde(default)]
    pub task_id: String,
    pub floor_id: String,
    pub name_key: String,
    pub status: TaskStatusKindDto,
    #[serde(default)]
    pub current: u32,
    #[serde(default = "default_task_required")]
    pub required: u32,
    #[serde(default = "default_task_stage")]
    pub stage: u32,
    #[serde(default = "default_task_stage")]
    pub stages: u32,
}

const fn default_task_required() -> u32 {
    1
}

const fn default_task_stage() -> u32 {
    1
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct CellDto {
    pub position: Position,
    pub terrain_id: String,
    pub item_id: Option<String>,
    pub actor_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum VisibilityState {
    Visible,
    Remembered,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct CellLightDto {
    pub color: u32,
    pub intensity: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct CellVisualDto {
    pub position: Position,
    pub visibility: VisibilityState,
    pub light: CellLightDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ContentVisualDto {
    pub id: String,
    pub glyph: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum DamageTypeDto {
    #[default]
    Physical,
    Acid,
    Electricity,
    Fire,
    Cold,
    Poison,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum ResistanceLevelDto {
    Vulnerable,
    Normal,
    Resistant,
    Strong,
    Immune,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ResistanceDto {
    pub damage_type: DamageTypeDto,
    pub level: ResistanceLevelDto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct DamageResolutionDto {
    pub raw_damage: i32,
    pub armor_reduction: i32,
    pub resistance_adjustment: i32,
    pub final_damage: i32,
    pub damage_type: DamageTypeDto,
    pub resistance: ResistanceLevelDto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum GameEventOutcomeDto {
    Damage { resolution: DamageResolutionDto },
    Death { resolution: DamageResolutionDto },
    Heal { resolution: HealingResolutionDto },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct HealingResolutionDto {
    pub requested: i32,
    pub applied: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct StatusDto {
    pub kind_id: String,
    pub intensity: u16,
    pub remaining_ticks: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct PlayerDto {
    pub id: String,
    pub kind_id: String,
    pub position: Position,
    pub hp: i32,
    pub max_hp: i32,
    #[serde(default = "default_actor_speed")]
    pub speed: u16,
    #[serde(default)]
    pub energy_need: i32,
    #[serde(default)]
    pub carried_weight_tenths_pound: u32,
    #[serde(default)]
    pub carry_capacity_tenths_pound: u32,
    #[serde(default)]
    pub base_max_hp: i32,
    #[serde(default)]
    pub attack: i32,
    #[serde(default)]
    pub base_attack: i32,
    #[serde(default)]
    pub defense: i32,
    #[serde(default)]
    pub base_defense: i32,
    #[serde(default)]
    pub melee_skill: i32,
    #[serde(default)]
    pub armor_class: i32,
    #[serde(default)]
    pub melee_damage: DamageDiceDto,
    #[serde(default)]
    pub melee_profile: AttackProfileDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projectile_profile: Option<ProjectileProfileDto>,
    #[serde(default)]
    pub is_dead: bool,
    #[serde(default)]
    pub equipment_modifiers: StatModifiersDto,
    #[serde(default)]
    pub statuses: Vec<StatusDto>,
    #[serde(default)]
    pub resistances: Vec<ResistanceDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct EntityDto {
    pub id: String,
    pub kind_id: String,
    pub position: Position,
    pub hp: i32,
    pub max_hp: i32,
    #[serde(default = "default_actor_speed")]
    pub speed: u16,
    #[serde(default = "default_monster_energy_need")]
    pub energy_need: i32,
    #[serde(default)]
    pub attack: i32,
    #[serde(default)]
    pub defense: i32,
    #[serde(default)]
    pub melee_skill: i32,
    #[serde(default)]
    pub armor_class: i32,
    #[serde(default)]
    pub melee_damage: DamageDiceDto,
    #[serde(default)]
    pub melee_profile: AttackProfileDto,
    #[serde(default)]
    pub melee_routine: MeleeRoutineDto,
    #[serde(default)]
    pub statuses: Vec<StatusDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ItemDto {
    pub id: String,
    pub kind_id: String,
    #[serde(default)]
    pub display_name_key: String,
    #[serde(default)]
    pub knowledge: ItemKnowledgeDto,
    pub position: Position,
    pub quantity: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum ItemKnowledgeDto {
    Unknown,
    Tried,
    #[default]
    Aware,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum ItemQualityDto {
    #[default]
    Ordinary,
    Fine,
    Exceptional,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum ItemIdentificationDto {
    #[default]
    Unexamined,
    Appraised,
    Identified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ItemPropertyDto {
    pub affix_id: String,
    pub name_key: String,
    #[serde(default)]
    pub modifiers: StatModifiersDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct InventoryItemDto {
    pub id: String,
    pub kind_id: String,
    #[serde(default)]
    pub display_name_key: String,
    #[serde(default)]
    pub knowledge: ItemKnowledgeDto,
    #[serde(default)]
    pub usable: bool,
    pub quantity: u32,
    #[serde(default)]
    pub weight_tenths_pound: u16,
    #[serde(default)]
    pub equipment_slot: Option<String>,
    #[serde(default)]
    pub modifiers: StatModifiersDto,
    #[serde(default)]
    pub identification: ItemIdentificationDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<ItemQualityDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub known_properties: Vec<ItemPropertyDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub melee_profile: Option<AttackProfileDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projectile_profile: Option<ProjectileProfileDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub throw_profile: Option<ThrowProfileDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct EquipmentItemDto {
    pub id: String,
    pub kind_id: String,
    #[serde(default)]
    pub display_name_key: String,
    #[serde(default)]
    pub knowledge: ItemKnowledgeDto,
    pub quantity: u32,
    #[serde(default)]
    pub weight_tenths_pound: u16,
    pub slot_id: String,
    #[serde(default)]
    pub modifiers: StatModifiersDto,
    #[serde(default)]
    pub identification: ItemIdentificationDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<ItemQualityDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub known_properties: Vec<ItemPropertyDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub melee_profile: Option<AttackProfileDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projectile_profile: Option<ProjectileProfileDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub throw_profile: Option<ThrowProfileDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct GameEventDto {
    pub kind: String,
    pub message_key: String,
    pub args: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<GameEventOutcomeDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace: Option<ProjectileTraceDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct GameSnapshot {
    pub protocol_version: String,
    pub revision: u32,
    pub turn: u32,
    #[serde(default)]
    pub world_tick: u32,
    pub last_command_seq: u32,
    pub width: u16,
    pub height: u16,
    pub cells: Vec<CellDto>,
    #[serde(default)]
    pub visual_cells: Vec<CellVisualDto>,
    pub player: PlayerDto,
    pub entities: Vec<EntityDto>,
    pub items: Vec<ItemDto>,
    pub inventory: Vec<InventoryItemDto>,
    #[serde(default)]
    pub equipment: Vec<EquipmentItemDto>,
    pub content_id: String,
    pub content_hash: String,
    pub content_visuals: Vec<ContentVisualDto>,
    pub world_id: String,
    pub floor_id: String,
    #[serde(default)]
    pub terrain_interactions: Vec<TerrainInteractionDto>,
    #[serde(default)]
    pub tasks: Vec<TaskStatusDto>,
    pub state_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct GameUpdate {
    pub base_revision: u32,
    pub revision: u32,
    pub turn: u32,
    #[serde(default)]
    pub world_tick: u32,
    pub command_seq: u32,
    pub floor_id: String,
    pub events: Vec<GameEventDto>,
    pub changed_cells: Vec<CellDto>,
    #[serde(default)]
    pub changed_visual_cells: Vec<CellVisualDto>,
    pub player: PlayerDto,
    pub entities: Vec<EntityDto>,
    pub items: Vec<ItemDto>,
    pub inventory: Vec<InventoryItemDto>,
    #[serde(default)]
    pub equipment: Vec<EquipmentItemDto>,
    pub removed_entities: Vec<String>,
    #[serde(default)]
    pub terrain_interactions: Vec<TerrainInteractionDto>,
    #[serde(default)]
    pub tasks: Vec<TaskStatusDto>,
    pub state_hash: String,
}

/// Schema bundle for the types crossing the CoreTransport boundary.
#[cfg(feature = "bindings")]
#[derive(JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolSchemaV1 {
    pub command: GameCommandEnvelope,
    pub snapshot: GameSnapshot,
    pub update: GameUpdate,
}

#[must_use]
#[cfg(feature = "bindings")]
pub fn generated_typescript() -> String {
    let config = Config::default();
    let mut output = String::from(
        "// SPDX-License-Identifier: MPL-2.0\n\
         // @generated by `cargo run -p rfb-protocol --features bindings --bin generate-bindings`; do not edit.\n\n",
    );

    macro_rules! push_declaration {
        ($type:ty) => {{
            output.push_str("export ");
            output.push_str(&<$type as TS>::decl(&config));
            output.push_str("\n\n");
        }};
    }

    push_declaration!(Direction);
    push_declaration!(GameCommand);
    push_declaration!(GameCommandEnvelope);
    push_declaration!(StatModifiersDto);
    push_declaration!(DamageDiceDto);
    push_declaration!(AttackProfileDto);
    push_declaration!(MeleeBlowDto);
    push_declaration!(MeleeRoutineDto);
    push_declaration!(TargetModeDto);
    push_declaration!(TargetSpecDto);
    push_declaration!(TargetSelection);
    push_declaration!(ProjectileProfileDto);
    push_declaration!(ThrowProfileDto);
    push_declaration!(ProjectileTraceDto);
    push_declaration!(Position);
    push_declaration!(TerrainInteractionKindDto);
    push_declaration!(TerrainInteractionUnavailableReasonDto);
    push_declaration!(TerrainInteractionDto);
    push_declaration!(TaskStatusKindDto);
    push_declaration!(TaskStatusDto);
    push_declaration!(CellDto);
    push_declaration!(VisibilityState);
    push_declaration!(CellLightDto);
    push_declaration!(CellVisualDto);
    push_declaration!(ContentVisualDto);
    push_declaration!(DamageTypeDto);
    push_declaration!(ResistanceLevelDto);
    push_declaration!(ResistanceDto);
    push_declaration!(DamageResolutionDto);
    push_declaration!(HealingResolutionDto);
    push_declaration!(GameEventOutcomeDto);
    push_declaration!(StatusDto);
    push_declaration!(PlayerDto);
    push_declaration!(EntityDto);
    push_declaration!(ItemDto);
    push_declaration!(ItemKnowledgeDto);
    push_declaration!(ItemQualityDto);
    push_declaration!(ItemIdentificationDto);
    push_declaration!(ItemPropertyDto);
    push_declaration!(InventoryItemDto);
    push_declaration!(EquipmentItemDto);
    push_declaration!(GameEventDto);
    push_declaration!(GameSnapshot);
    push_declaration!(GameUpdate);

    output
}

#[cfg(feature = "bindings")]
pub fn generated_json_schema() -> Result<String, serde_json::Error> {
    let mut output = serde_json::to_string_pretty(&schema_for!(ProtocolSchemaV1))?;
    output.push('\n');
    Ok(output)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerrainSaveDto {
    pub width: u16,
    pub height: u16,
    pub terrain_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RngSaveDto {
    pub algorithm: String,
    pub state: [u64; 4],
    pub draw_counter: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSaveDto {
    pub id: String,
    pub kind_id: String,
    pub position: Position,
    pub hp: i32,
    #[serde(default)]
    pub base_max_hp: i32,
    #[serde(default = "default_actor_speed")]
    pub base_speed: u16,
    #[serde(default)]
    pub energy_need: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub statuses: Vec<StatusSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resistances: Vec<ResistanceSaveDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorSaveDto {
    pub id: String,
    pub kind_id: String,
    pub position: Position,
    pub hp: i32,
    #[serde(default)]
    pub max_hp: i32,
    #[serde(default = "default_actor_speed")]
    pub base_speed: u16,
    #[serde(default = "default_monster_energy_need")]
    pub energy_need: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub statuses: Vec<StatusSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resistances: Vec<ResistanceSaveDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StatusSaveDto {
    pub kind_id: String,
    pub intensity: u16,
    pub remaining_ticks: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResistanceSaveDto {
    pub damage_type: DamageTypeDto,
    pub level: ResistanceLevelDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemSaveDto {
    pub id: String,
    pub kind_id: String,
    pub position: Position,
    pub quantity: u32,
    #[serde(default)]
    pub quality: ItemQualityDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affix_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryItemSaveDto {
    pub id: String,
    pub kind_id: String,
    pub quantity: u32,
    #[serde(default)]
    pub quality: ItemQualityDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affix_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EquipmentItemSaveDto {
    pub id: String,
    pub kind_id: String,
    pub quantity: u32,
    pub slot_id: String,
    #[serde(default)]
    pub quality: ItemQualityDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affix_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CarriedItemSaveDto {
    pub id: String,
    pub kind_id: String,
    pub quantity: u32,
    pub actor_id: String,
    #[serde(default)]
    pub quality: ItemQualityDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affix_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FloorSaveDto {
    pub id: String,
    pub player_position: Position,
    pub terrain: TerrainSaveDto,
    pub entities: Vec<ActorSaveDto>,
    #[serde(default)]
    pub items: Vec<ItemSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub carried_items: Vec<CarriedItemSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub explored: Vec<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub revealed_terrain: Vec<Position>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemKnowledgeSaveDto {
    pub kind_id: String,
    #[serde(default)]
    pub tried: bool,
    #[serde(default)]
    pub aware: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemPropertyKnowledgeSaveDto {
    pub item_id: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub appraised: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub identified: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub known_affix_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskProgressSaveDto {
    #[serde(alias = "floorId")]
    pub task_id: String,
    pub current: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStateSaveDto {
    pub task_id: String,
    pub status: TaskStatusKindDto,
    #[serde(default)]
    pub stage_index: u32,
    pub current: u32,
    pub required: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_floor_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DungeonStateSaveDto {
    pub dungeon_id: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub guardian_defeated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavePayloadV1 {
    pub schema_version: u16,
    pub revision: u32,
    pub turn: u32,
    #[serde(default)]
    pub world_tick: u32,
    pub last_command_seq: u32,
    pub terrain: TerrainSaveDto,
    pub player: PlayerSaveDto,
    pub entities: Vec<ActorSaveDto>,
    #[serde(default)]
    pub items: Vec<ItemSaveDto>,
    #[serde(default)]
    pub inventory: Vec<InventoryItemSaveDto>,
    #[serde(default)]
    pub equipment: Vec<EquipmentItemSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub carried_items: Vec<CarriedItemSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub item_knowledge: Vec<ItemKnowledgeSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub item_property_knowledge: Vec<ItemPropertyKnowledgeSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub task_progress: Vec<TaskProgressSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub task_states: Vec<TaskStateSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dungeon_states: Vec<DungeonStateSaveDto>,
    #[serde(default)]
    pub next_item_instance_serial: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub explored: Vec<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub revealed_terrain: Vec<Position>,
    pub rng: RngSaveDto,
    pub content_id: String,
    pub content_hash: String,
    #[serde(default)]
    pub world_id: String,
    #[serde(default)]
    pub current_floor_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stored_floors: Vec<FloorSaveDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterSummary {
    pub display_name: String,
    pub level: u32,
    pub location_key: String,
    pub turn: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveHeaderV1 {
    pub format: String,
    pub save_schema_version: u16,
    pub game_version: String,
    pub protocol_version: String,
    #[serde(default)]
    pub slot_name: String,
    pub created_at: String,
    pub saved_at: String,
    pub character_summary: CharacterSummary,
    pub content_id: String,
    pub content_hash: String,
    pub payload_encoding: String,
}

#[derive(Debug, Error)]
pub enum CodecError {
    #[error("failed to encode MessagePack: {0}")]
    Encode(#[from] rmp_serde::encode::Error),
    #[error("failed to decode MessagePack: {0}")]
    Decode(#[from] rmp_serde::decode::Error),
}

pub fn to_msgpack<T: Serialize>(value: &T) -> Result<Vec<u8>, CodecError> {
    Ok(rmp_serde::to_vec_named(value)?)
}

pub fn from_msgpack<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, CodecError> {
    Ok(rmp_serde::from_slice(bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct LegacySavePayloadV1 {
        schema_version: u16,
        revision: u32,
        turn: u32,
        last_command_seq: u32,
        terrain: TerrainSaveDto,
        player: PlayerDto,
        entities: Vec<EntityDto>,
        items: Vec<ItemDto>,
        inventory: Vec<InventoryItemDto>,
        equipment: Vec<EquipmentItemDto>,
        next_item_instance_serial: u64,
        explored: Vec<bool>,
        rng: RngSaveDto,
        content_id: String,
        content_hash: String,
        world_id: String,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct DerivedFieldProbe {
        #[serde(default)]
        attack: Option<i32>,
        #[serde(default)]
        defense: Option<i32>,
        #[serde(default)]
        melee_skill: Option<i32>,
        #[serde(default)]
        armor_class: Option<i32>,
        #[serde(default)]
        equipment_modifiers: Option<StatModifiersDto>,
    }

    #[test]
    fn command_messagepack_round_trip() {
        for (index, command) in [
            GameCommand::Appraise {
                item_id: "demo.item.echo-charm.1".to_owned(),
            },
            GameCommand::BashDoor {
                direction: Direction::South,
            },
            GameCommand::OpenDoor {
                direction: Direction::East,
            },
            GameCommand::CloseDoor {
                direction: Direction::West,
            },
            GameCommand::Move {
                direction: Direction::SouthEast,
            },
            GameCommand::PickUp,
            GameCommand::Equip {
                item_id: "demo.item.echo-charm.1".to_owned(),
            },
            GameCommand::Unequip {
                slot_id: "charm".to_owned(),
            },
            GameCommand::Drop {
                item_ids: vec!["demo.item.echo-charm.1".to_owned()],
            },
            GameCommand::DropQuantity {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                quantity: 2,
            },
            GameCommand::Fire {
                direction: Direction::East,
            },
            GameCommand::FireTarget {
                target: TargetSelection::Entity {
                    entity_id: "demo.monster.ember-mote.1".to_owned(),
                },
            },
            GameCommand::Throw {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                direction: Direction::North,
            },
            GameCommand::TraverseStairs,
            GameCommand::Search,
            GameCommand::UseItem {
                item_id: "demo.item.luminous-shard.1".to_owned(),
            },
            GameCommand::Wait,
        ]
        .into_iter()
        .enumerate()
        {
            let envelope = GameCommandEnvelope {
                command_seq: index as u32 + 1,
                expected_revision: index as u32,
                command,
            };
            let encoded = to_msgpack(&envelope).expect("command should encode");
            let decoded: GameCommandEnvelope =
                from_msgpack(&encoded).expect("command should decode");
            assert_eq!(decoded, envelope);
        }
    }

    #[test]
    fn legacy_game_event_without_outcome_remains_compatible() {
        let legacy = serde_json::json!({
            "kind": "wait",
            "messageKey": "event-wait",
            "args": {}
        });

        let event: GameEventDto =
            serde_json::from_value(legacy).expect("legacy event should decode");
        assert_eq!(event.outcome, None);

        let encoded = serde_json::to_value(&event).expect("event should encode");
        assert_eq!(encoded.get("outcome"), None);
    }

    #[test]
    fn legacy_v1_save_payload_decodes_into_authoritative_save_dtos() {
        let legacy = LegacySavePayloadV1 {
            schema_version: 1,
            revision: 2,
            turn: 2,
            last_command_seq: 2,
            terrain: TerrainSaveDto {
                width: 1,
                height: 1,
                terrain_ids: vec!["demo.terrain.floor".to_owned()],
            },
            player: PlayerDto {
                id: "demo.player".to_owned(),
                kind_id: "demo.actor.explorer".to_owned(),
                position: Position { x: 0, y: 0 },
                hp: 8,
                max_hp: 14,
                speed: 110,
                energy_need: 0,
                carried_weight_tenths_pound: 5,
                carry_capacity_tenths_pound: 100,
                base_max_hp: 10,
                attack: 3,
                base_attack: 2,
                defense: 2,
                base_defense: 1,
                melee_skill: 60,
                armor_class: 20,
                melee_damage: DamageDiceDto {
                    dice: 1,
                    sides: 2,
                    damage_type: DamageTypeDto::Physical,
                },
                melee_profile: AttackProfileDto::default(),
                projectile_profile: None,
                is_dead: false,
                equipment_modifiers: StatModifiersDto {
                    attack: 1,
                    defense: 1,
                    max_hp: 4,
                },
                statuses: Vec::new(),
                resistances: Vec::new(),
            },
            entities: vec![EntityDto {
                id: "demo.monster.1".to_owned(),
                kind_id: "demo.actor.monster".to_owned(),
                position: Position { x: 1, y: 0 },
                hp: 3,
                max_hp: 3,
                speed: 110,
                energy_need: 100,
                attack: 1,
                defense: 1,
                melee_skill: 32,
                armor_class: 10,
                melee_damage: DamageDiceDto {
                    dice: 1,
                    sides: 2,
                    damage_type: DamageTypeDto::Physical,
                },
                melee_profile: AttackProfileDto::default(),
                melee_routine: MeleeRoutineDto::default(),
                statuses: Vec::new(),
            }],
            items: vec![ItemDto {
                id: "demo.item.ground.1".to_owned(),
                kind_id: "demo.item.shard".to_owned(),
                display_name_key: "item-demo-shard-name".to_owned(),
                knowledge: ItemKnowledgeDto::Aware,
                position: Position { x: 0, y: 0 },
                quantity: 2,
            }],
            inventory: vec![InventoryItemDto {
                id: "demo.item.inventory.1".to_owned(),
                kind_id: "demo.item.charm".to_owned(),
                display_name_key: "item-demo-charm-name".to_owned(),
                knowledge: ItemKnowledgeDto::Aware,
                usable: false,
                quantity: 1,
                weight_tenths_pound: 5,
                equipment_slot: Some("charm".to_owned()),
                modifiers: StatModifiersDto {
                    attack: 1,
                    defense: 1,
                    max_hp: 4,
                },
                identification: ItemIdentificationDto::Unexamined,
                quality: None,
                known_properties: Vec::new(),
                melee_profile: None,
                projectile_profile: None,
                throw_profile: None,
            }],
            equipment: vec![EquipmentItemDto {
                id: "demo.item.equipment.1".to_owned(),
                kind_id: "demo.item.charm".to_owned(),
                display_name_key: "item-demo-charm-name".to_owned(),
                knowledge: ItemKnowledgeDto::Aware,
                quantity: 1,
                weight_tenths_pound: 5,
                slot_id: "charm".to_owned(),
                modifiers: StatModifiersDto {
                    attack: 1,
                    defense: 1,
                    max_hp: 4,
                },
                identification: ItemIdentificationDto::Unexamined,
                quality: None,
                known_properties: Vec::new(),
                melee_profile: None,
                projectile_profile: None,
                throw_profile: None,
            }],
            next_item_instance_serial: 4,
            explored: vec![true],
            rng: RngSaveDto {
                algorithm: "rfb-rng-xoshiro256ss-v1".to_owned(),
                state: [1, 2, 3, 4],
                draw_counter: 5,
            },
            content_id: "demo.content".to_owned(),
            content_hash: "0".repeat(64),
            world_id: "demo.world".to_owned(),
        };

        let encoded = to_msgpack(&legacy).expect("legacy payload should encode");
        let decoded: SavePayloadV1 =
            from_msgpack(&encoded).expect("legacy payload should migrate while decoding");

        assert_eq!(decoded.player.base_max_hp, 10);
        assert_eq!(decoded.entities[0].max_hp, 3);
        assert_eq!(decoded.inventory[0].kind_id, "demo.item.charm");
        assert_eq!(decoded.equipment[0].slot_id, "charm");
        assert!(decoded.item_knowledge.is_empty());
        assert!(decoded.item_property_knowledge.is_empty());
        assert!(decoded.revealed_terrain.is_empty());
        assert!(decoded.inventory[0].affix_ids.is_empty());
        assert_eq!(decoded.inventory[0].quality, ItemQualityDto::Ordinary);
    }

    #[test]
    fn authoritative_player_save_omits_derived_combat_fields() {
        let player = PlayerSaveDto {
            id: "demo.player".to_owned(),
            kind_id: "demo.actor.explorer".to_owned(),
            position: Position { x: 0, y: 0 },
            hp: 10,
            base_max_hp: 10,
            base_speed: 110,
            energy_need: 0,
            statuses: Vec::new(),
            resistances: Vec::new(),
        };

        let encoded = to_msgpack(&player).expect("player save should encode");
        let probe: DerivedFieldProbe =
            from_msgpack(&encoded).expect("derived field probe should decode");

        assert_eq!(probe.attack, None);
        assert_eq!(probe.defense, None);
        assert_eq!(probe.melee_skill, None);
        assert_eq!(probe.armor_class, None);
        assert_eq!(probe.equipment_modifiers, None);
    }

    #[cfg(feature = "bindings")]
    #[test]
    fn generated_bindings_follow_the_serialized_contract() {
        let typescript = generated_typescript();
        assert!(typescript.contains("actorId: string | null"));
        assert!(typescript.contains("commandSeq: number"));
        assert!(typescript.contains("itemIds: Array<string>"));
        assert!(typescript.contains("equipmentModifiers: StatModifiersDto"));
        assert!(typescript.contains("baseAttack: number"));
        assert!(typescript.contains("baseDefense: number"));
        assert!(typescript.contains("attack: number"));
        assert!(typescript.contains("defense: number"));
        assert!(typescript.contains("equipment: Array<EquipmentItemDto>"));
        assert!(typescript.contains("{ \"type\": \"wait\" }"));

        let schema: serde_json::Value = serde_json::from_str(
            &generated_json_schema().expect("protocol schema should serialize"),
        )
        .expect("generated protocol schema should be valid JSON");
        assert_eq!(schema["title"], "ProtocolSchemaV1");
        assert!(schema["$defs"]["GameCommand"].is_object());
    }
}
