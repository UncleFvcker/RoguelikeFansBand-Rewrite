// SPDX-License-Identifier: MPL-2.0

use std::{collections::BTreeSet, fmt, str::FromStr};

use rfb_core::{CoreError, Game, load_built_in_content};
use rfb_protocol::{
    AbilityDto, AbilityLearningDto, AbilityProgressSaveDto, CampaignStateDto, CampaignStateSaveDto,
    CharacterSummary, EntityFactionDto, EquipmentItemDto, EquipmentItemSaveDto, GameCommand,
    GameCommandEnvelope, GameEventDto, GoldPileDto, HomeDto, InventoryItemDto,
    InventoryItemSaveDto, ItemActivationDto, ItemChargesDto, ItemCurseSeverityDto,
    ItemEnchantmentsDto, ItemFuelDto, ItemKnowledgeSaveDto, ItemPropertyKnowledgeSaveDto,
    ItemQualityDto, MonsterPackSaveDto, NaturalAttributeSetSaveDto, PROTOCOL_VERSION,
    PlayerBuildDto, Position, RecallStateDto, ResistanceDto, ResistanceSaveDto, ResourcePoolDto,
    ResourcePoolSaveDto, RolledAffixSaveDto, SAVE_HEADER_SCHEMA_VERSION, SaveHeaderV1, ShopDto,
    StatusDto, StatusSaveDto, SummonCommandDto, SummonSaveDto, TaskStateSaveDto, TaskStatusDto,
    TerrainInteractionDto, TownDto,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod policy;
pub mod snapshot;

pub const CONTRACT_SCHEMA_VERSION: u16 = 3;
pub const ACTIVE_BASELINE: &str = "contract-v219";
pub const ACTIVE_FIXTURE_DIRECTORY: &str = "active";
pub const LEGACY_BASELINE_COMMIT: &str = "191f48c3fd1cdbc81a3d3395a88cd6758402b4d9";
pub const ORIGINAL_TEST_WORLD: &str = "demo.world.original-v1";
pub const HISTORICAL_TEST_WORLD: &str = "demo.original-v1";
pub const WARRENS_TEST_WORLD: &str = "demo.world.warrens-journey";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContractFixture {
    pub schema_version: u16,
    pub id: String,
    pub category: FixtureCategory,
    pub legacy_commit: String,
    pub determinism: Determinism,
    pub seed: String,
    pub preconditions: Preconditions,
    pub commands: Vec<ContractCommand>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub save_round_trip: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assertions: Option<ContractAssertions>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FixtureCategory {
    Movement,
    System,
    Combat,
    Inventory,
    Dungeon,
    Tasks,
    Campaign,
    Progression,
    Abilities,
    Resources,
    Monsters,
    Techniques,
    StatusEffects,
    Equipment,
    MagicRealms,
    Devices,
    Scrolls,
    Potions,
    Town,
}

impl FixtureCategory {
    pub const ALL: [Self; 19] = [
        Self::Movement,
        Self::System,
        Self::Combat,
        Self::Inventory,
        Self::Dungeon,
        Self::Tasks,
        Self::Campaign,
        Self::Progression,
        Self::Abilities,
        Self::Resources,
        Self::Monsters,
        Self::Techniques,
        Self::StatusEffects,
        Self::Equipment,
        Self::MagicRealms,
        Self::Devices,
        Self::Scrolls,
        Self::Potions,
        Self::Town,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Movement => "movement",
            Self::System => "system",
            Self::Combat => "combat",
            Self::Inventory => "inventory",
            Self::Dungeon => "dungeon",
            Self::Tasks => "tasks",
            Self::Campaign => "campaign",
            Self::Progression => "progression",
            Self::Abilities => "abilities",
            Self::Resources => "resources",
            Self::Monsters => "monsters",
            Self::Techniques => "techniques",
            Self::StatusEffects => "status-effects",
            Self::Equipment => "equipment",
            Self::MagicRealms => "magic-realms",
            Self::Devices => "devices",
            Self::Scrolls => "scrolls",
            Self::Potions => "potions",
            Self::Town => "town",
        }
    }
}

impl fmt::Display for FixtureCategory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for FixtureCategory {
    type Err = FixtureCategoryParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|category| category.as_str() == value)
            .ok_or_else(|| FixtureCategoryParseError(value.to_owned()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("unknown fixture category {0}; use list-categories to see valid categories")]
pub struct FixtureCategoryParseError(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Determinism {
    Exact,
    Semantic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preconditions {
    pub world: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub debug_clear_entities: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub debug_clear_carried_items: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub debug_ability_casts_succeed: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub debug_recharge_attempts_succeed: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub debug_recharge_attempts_fail: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub debug_recharge_sources_survive: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debug_recall_delay_turns: Option<u16>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub debug_item_curses_land: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub debug_item_curses_resisted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_build_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_position: Option<Position>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_hp: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_gold: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_level: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_experience: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_maximum_experience: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_life_force: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_max_level: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_pending_attribute_increases: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_attributes: Option<NaturalAttributeSetSaveDto>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub legacy_player_progress: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_resources: Option<Vec<ResourcePoolSaveDto>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_learned_ability_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_ability_progress: Option<Vec<AbilityProgressSaveDto>>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub legacy_player_ability_state: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub player_statuses: Vec<StatusSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub player_resistances: Vec<ResistanceSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entity_effects: Vec<EntityEffectsPrecondition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inventory_items: Vec<InventoryItemPrecondition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub equipment_items: Vec<EquipmentItemSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terrain_overrides: Vec<TerrainOverridePrecondition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub revealed_terrain: Vec<Position>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub campaign_conquered_dungeons: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub campaign_state: Option<CampaignStateSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub task_states: Vec<TaskStateSaveDto>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub enter_task_floor: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub task_states_after_floor_entry: Vec<TaskStateSaveDto>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub debug_clear_entities_after_task_floor_entry: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EntityEffectsPrecondition {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hp: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_hp: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_speed: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub energy_need: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alerted: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub casting_cooldown_remaining: Option<u16>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub observed_player_resistances: Vec<ResistanceSaveDto>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub clear_pack: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub statuses: Vec<StatusSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resistances: Vec<ResistanceSaveDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summon: Option<SummonSaveDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InventoryItemPrecondition {
    pub id: String,
    pub kind_id: String,
    #[serde(
        default = "default_precondition_quantity",
        skip_serializing_if = "is_default_precondition_quantity"
    )]
    pub quantity: u32,
    #[serde(default, skip_serializing_if = "is_ordinary_item_quality")]
    pub quality: ItemQualityDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affix_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rolled_affixes: Vec<RolledAffixSaveDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_depth: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charges: Option<ItemChargesDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuel: Option<ItemFuelDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activation: Option<ItemActivationDto>,
    #[serde(default, skip_serializing_if = "ItemEnchantmentsDto::is_empty")]
    pub enchantments: ItemEnchantmentsDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curse: Option<ItemCurseSeverityDto>,
    #[serde(default, skip_serializing_if = "is_zero_u16")]
    pub device_recovery_progress: u16,
}

const fn default_precondition_quantity() -> u32 {
    1
}

const fn is_default_precondition_quantity(value: &u32) -> bool {
    *value == default_precondition_quantity()
}

const fn is_ordinary_item_quality(value: &ItemQualityDto) -> bool {
    matches!(value, ItemQualityDto::Ordinary)
}

const fn is_false(value: &bool) -> bool {
    !*value
}

const fn is_true(value: &bool) -> bool {
    *value
}

const fn is_zero_u16(value: &u16) -> bool {
    *value == 0
}

const fn is_zero_u32(value: &u32) -> bool {
    *value == 0
}

const fn is_zero_u64(value: &u64) -> bool {
    *value == 0
}

const fn is_zero_usize(value: &usize) -> bool {
    *value == 0
}

fn summon_command_is_default(value: &SummonCommandDto) -> bool {
    value == &SummonCommandDto::default()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerrainOverridePrecondition {
    pub position: Position,
    pub terrain_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContractCommand {
    pub command: ContractCommandAction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_seq: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_revision: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContractCommandAction {
    Protocol(GameCommand),
    Fixture(ContractOnlyCommand),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ContractOnlyCommand {
    BuyFirstFromShop { shop_id: String, quantity: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContractAssertions {
    pub final_state: FinalStateAssertion,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<GameEventDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_cells: Vec<Position>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_entities: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<CommandErrorAssertion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub save_round_trip_state_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FinalStateAssertion {
    pub revision: u32,
    pub turn: u32,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub world_tick: u32,
    pub last_command_seq: u32,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub rng_draw_counter: u64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub floor_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dungeon_instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub town: Option<TownDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shops: Vec<ShopDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub homes: Vec<HomeDto>,
    pub player_position: Position,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_hp: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_max_hp: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_attack: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_defense: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_speed: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_energy_need: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_carried_weight_tenths_pound: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_carry_capacity_tenths_pound: Option<u32>,
    pub player_encumbrance_speed_penalty: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_gold: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_nutrition: Option<u16>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub player_statuses: Vec<StatusDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub player_resistances: Vec<ResistanceDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_level: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_experience: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_max_level: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_pending_attribute_increases: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_attributes: Option<rfb_protocol::PlayerProgressDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_build: Option<PlayerBuildDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub player_resources: Vec<ResourcePoolDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_ability_learning: Option<AbilityLearningDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub player_abilities: Vec<AbilityDto>,
    #[serde(default, skip_serializing_if = "summon_command_is_default")]
    pub player_summon_command: SummonCommandDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_recall: Option<RecallStateDto>,
    pub entity_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entities: Vec<ActorStateAssertion>,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub ground_item_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gold_piles: Vec<GoldPileDto>,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub inventory_stack_count: usize,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub equipment_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inventory: Vec<InventoryItemDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub equipment: Vec<EquipmentItemDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub item_knowledge: Vec<ItemKnowledgeSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub item_property_knowledge: Vec<ItemPropertyKnowledgeSaveDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_item_instance_serial: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_gold_pile_serial: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terrain_interactions: Vec<TerrainInteractionDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tasks: Vec<TaskStatusDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub campaign: Option<CampaignStateDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub revealed_terrain: Vec<Position>,
    pub state_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorStateAssertion {
    pub id: String,
    pub position: Position,
    pub hp: i32,
    pub speed: u16,
    pub energy_need: i32,
    #[serde(default = "default_assertion_alerted", skip_serializing_if = "is_true")]
    pub alerted: bool,
    #[serde(default, skip_serializing_if = "is_zero_u16")]
    pub casting_cooldown_remaining: u16,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub observed_player_resistances: Vec<ResistanceDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub statuses: Vec<StatusDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack: Option<MonsterPackSaveDto>,
    #[serde(default, skip_serializing_if = "entity_faction_is_hostile")]
    pub faction: EntityFactionDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summon: Option<SummonSaveDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controller_id: Option<String>,
}

fn entity_faction_is_hostile(faction: &EntityFactionDto) -> bool {
    *faction == EntityFactionDto::Hostile
}

const fn default_assertion_alerted() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandErrorAssertion {
    pub step: usize,
    pub kind: CommandErrorKind,
    pub state_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CommandErrorKind {
    RevisionMismatch,
    CommandSequence,
    PlayerDead,
    CampaignEnded,
}

pub fn observe(fixture: &ContractFixture) -> Result<ContractAssertions, ContractError> {
    validate_fixture(fixture)?;
    let seed = parse_seed(&fixture.seed)?;
    let initial_game = match fixture.preconditions.world.as_str() {
        WARRENS_TEST_WORLD => Game::new_warrens_journey_with_build(
            seed,
            fixture
                .preconditions
                .player_build_id
                .as_deref()
                .unwrap_or("demo.build.warrior"),
        )?,
        _ => fixture
            .preconditions
            .player_build_id
            .as_deref()
            .map_or_else(
                || Ok(Game::new(seed)),
                |build_id| Game::new_with_build(seed, build_id),
            )?,
    };
    let mut payload = initial_game.to_save();
    if let Some(player_hp) = fixture.preconditions.player_hp {
        payload.player.hp = player_hp;
    }
    if let Some(player_gold) = fixture.preconditions.player_gold {
        payload.player.gold = player_gold;
    }
    if fixture.preconditions.player_level.is_some()
        || fixture.preconditions.player_experience.is_some()
        || fixture.preconditions.player_maximum_experience.is_some()
        || fixture.preconditions.player_life_force.is_some()
        || fixture.preconditions.player_max_level.is_some()
        || fixture
            .preconditions
            .player_pending_attribute_increases
            .is_some()
        || fixture.preconditions.player_attributes.is_some()
    {
        let progress = payload
            .player
            .progress
            .as_mut()
            .ok_or(ContractError::MissingProgressPrecondition)?;
        if let Some(level) = fixture.preconditions.player_level {
            progress.level = level;
            progress.skills.clear();
        }
        if let Some(experience) = fixture.preconditions.player_experience {
            progress.experience = experience;
        }
        if let Some(maximum_experience) = fixture.preconditions.player_maximum_experience {
            progress.maximum_experience = maximum_experience;
        }
        if let Some(life_force) = fixture.preconditions.player_life_force {
            progress.life_force = life_force;
        }
        if let Some(max_level) = fixture.preconditions.player_max_level {
            progress.max_level = max_level;
        } else if fixture.preconditions.player_level.is_some() {
            progress.max_level = progress.level;
        }
        if let Some(pending) = fixture.preconditions.player_pending_attribute_increases {
            progress.pending_attribute_increases = pending;
        }
        if let Some(attributes) = &fixture.preconditions.player_attributes {
            progress.attributes = *attributes;
            progress.maximum_attributes = Some(*attributes);
        }
    }
    if fixture.preconditions.legacy_player_progress {
        payload.player.progress = None;
    }
    if fixture.preconditions.legacy_player_ability_state {
        payload.player.resources.clear();
        payload.player.learned_ability_ids.clear();
        payload.player.ability_progress.clear();
    } else {
        if let Some(resources) = &fixture.preconditions.player_resources {
            payload.player.resources.clone_from(resources);
        }
        if let Some(ability_ids) = &fixture.preconditions.player_learned_ability_ids {
            payload.player.learned_ability_ids.clone_from(ability_ids);
        }
        if let Some(progress) = &fixture.preconditions.player_ability_progress {
            payload.player.ability_progress.clone_from(progress);
        }
    }
    payload.player.statuses = fixture.preconditions.player_statuses.clone();
    payload.player.resistances = fixture.preconditions.player_resistances.clone();
    let precondition_item_ids = fixture
        .preconditions
        .inventory_items
        .iter()
        .map(|item| item.id.as_str())
        .chain(
            fixture
                .preconditions
                .equipment_items
                .iter()
                .map(|item| item.id.as_str()),
        )
        .collect::<BTreeSet<_>>();
    payload
        .items
        .retain(|item| !precondition_item_ids.contains(item.id.as_str()));
    payload
        .inventory
        .retain(|item| !precondition_item_ids.contains(item.id.as_str()));
    payload
        .equipment
        .retain(|item| !precondition_item_ids.contains(item.id.as_str()));
    payload
        .carried_items
        .retain(|item| !precondition_item_ids.contains(item.id.as_str()));
    for floor in &mut payload.stored_floors {
        floor
            .items
            .retain(|item| !precondition_item_ids.contains(item.id.as_str()));
        floor
            .carried_items
            .retain(|item| !precondition_item_ids.contains(item.id.as_str()));
    }
    for item in fixture
        .preconditions
        .inventory_items
        .iter()
        .filter(|item| item.generation_depth.is_none())
    {
        payload.inventory.push(InventoryItemSaveDto {
            id: item.id.clone(),
            kind_id: item.kind_id.clone(),
            quantity: item.quantity,
            quality: item.quality,
            affix_ids: item.affix_ids.clone(),
            rolled_affixes: item.rolled_affixes.clone(),
            activation: item.activation.clone(),
            charges: item.charges,
            fuel: item.fuel,
            enchantments: item.enchantments,
            curse: item.curse,
            device_recovery_progress: item.device_recovery_progress,
        });
    }
    for item in &fixture.preconditions.equipment_items {
        payload
            .item_property_knowledge
            .retain(|knowledge| knowledge.item_id != item.id);
        payload
            .item_property_knowledge
            .push(ItemPropertyKnowledgeSaveDto {
                item_id: item.id.clone(),
                discovered: true,
                appraised: true,
                identified: true,
                known_affix_ids: item.affix_ids.clone(),
            });
        payload.equipment.push(item.clone());
    }
    for terrain_override in &fixture.preconditions.terrain_overrides {
        if terrain_override.position.x < 0
            || terrain_override.position.y < 0
            || terrain_override.position.x >= i32::from(payload.terrain.width)
            || terrain_override.position.y >= i32::from(payload.terrain.height)
        {
            return Err(ContractError::InvalidTerrainPrecondition(
                terrain_override.position,
            ));
        }
        let index = usize::try_from(terrain_override.position.y)
            .expect("validated terrain y must fit usize")
            * usize::from(payload.terrain.width)
            + usize::try_from(terrain_override.position.x)
                .expect("validated terrain x must fit usize");
        payload.terrain.terrain_ids[index].clone_from(&terrain_override.terrain_id);
    }
    if let Some(position) = fixture.preconditions.player_position {
        let terrain_index =
            fixture_position_index(position, payload.terrain.width, payload.terrain.height)
                .ok_or(ContractError::InvalidPlayerPositionPrecondition(position))?;
        let terrain_id = &payload.terrain.terrain_ids[terrain_index];
        let content = load_built_in_content()?;
        if !content
            .terrain(terrain_id)
            .is_some_and(|terrain| terrain.walkable)
        {
            return Err(ContractError::InvalidPlayerPositionPrecondition(position));
        }
        payload.player.position = position;
        if let Some(town) = content
            .world(&payload.world_id)
            .and_then(|world| world.town_id.as_deref())
            .and_then(|town_id| content.town(town_id))
            .filter(|town| town.floor_id == payload.current_floor_id)
        {
            for shop_id in &town.shop_ids {
                let Some(shop) = content.shop(shop_id) else {
                    continue;
                };
                if position.x == i32::from(shop.entrance_position.x)
                    && position.y == i32::from(shop.entrance_position.y)
                    && let Some(state) = payload
                        .shop_states
                        .iter_mut()
                        .find(|state| &state.shop_id == shop_id)
                {
                    state.visited = true;
                }
            }
            for facility_id in &town.facility_ids {
                let Some(facility) = content.town_facility(facility_id) else {
                    continue;
                };
                if position.x == i32::from(facility.entrance_position.x)
                    && position.y == i32::from(facility.entrance_position.y)
                    && let Some(state) = payload
                        .home_states
                        .iter_mut()
                        .find(|state| &state.facility_id == facility_id)
                {
                    state.visited = true;
                }
            }
        }
    }
    payload
        .revealed_terrain
        .extend(fixture.preconditions.revealed_terrain.iter().copied());
    for dungeon_id in &fixture.preconditions.campaign_conquered_dungeons {
        let state = payload
            .dungeon_states
            .iter_mut()
            .find(|state| &state.dungeon_id == dungeon_id)
            .ok_or_else(|| ContractError::UnknownDungeonPrecondition(dungeon_id.clone()))?;
        state.guardian_defeated = true;
    }
    payload.campaign_state = fixture.preconditions.campaign_state.clone();
    for task_state in &fixture.preconditions.task_states {
        payload
            .task_states
            .retain(|state| state.task_id != task_state.task_id);
        payload.task_states.push(task_state.clone());
    }
    for effects in &fixture.preconditions.entity_effects {
        let entity = payload
            .entities
            .iter_mut()
            .find(|entity| entity.id == effects.id)
            .ok_or_else(|| ContractError::UnknownEntityPrecondition(effects.id.clone()))?;
        if let Some(kind_id) = &effects.kind_id {
            entity.kind_id.clone_from(kind_id);
        }
        if let Some(position) = effects.position {
            entity.position = position;
        }
        if let Some(hp) = effects.hp {
            entity.hp = hp;
        }
        if let Some(max_hp) = effects.max_hp {
            entity.max_hp = max_hp;
        }
        if let Some(base_speed) = effects.base_speed {
            entity.base_speed = base_speed;
        }
        if let Some(energy_need) = effects.energy_need {
            entity.energy_need = energy_need;
        }
        if let Some(alerted) = effects.alerted {
            entity.alerted = Some(alerted);
        }
        if let Some(cooldown) = effects.casting_cooldown_remaining {
            entity.casting_cooldown_remaining = cooldown;
        }
        entity.observed_player_resistances = effects.observed_player_resistances.clone();
        if effects.clear_pack {
            entity.pack = None;
        }
        entity.statuses = effects.statuses.clone();
        entity.resistances = effects.resistances.clone();
        entity.summon = effects.summon.clone();
    }
    if fixture.preconditions.debug_clear_carried_items {
        payload.carried_items.clear();
    }
    if fixture.preconditions.debug_clear_entities {
        payload.entities.clear();
        payload.carried_items.clear();
        for state in &mut payload.dungeon_states {
            if state.dungeon_id == "demo.dungeon.resonance-descent" {
                state.entrance_guardian_defeated = Some(true);
            }
        }
    }
    let mut game = Game::from_save(payload)?;
    if fixture.preconditions.enter_task_floor {
        let envelope = GameCommandEnvelope {
            command_seq: game.last_command_seq().saturating_add(1),
            expected_revision: game.revision(),
            command: GameCommand::TraverseStairs,
        };
        game.dispatch(envelope)
            .map_err(|error| ContractError::UnexpectedCoreError(error.to_string()))?;
        if !fixture
            .preconditions
            .task_states_after_floor_entry
            .is_empty()
        {
            let mut payload = game.to_save();
            for task_state in &fixture.preconditions.task_states_after_floor_entry {
                payload
                    .task_states
                    .retain(|state| state.task_id != task_state.task_id);
                payload.task_states.push(task_state.clone());
            }
            game = Game::from_save(payload)?;
        }
        if fixture
            .preconditions
            .debug_clear_entities_after_task_floor_entry
        {
            let mut payload = game.to_save();
            payload.entities.clear();
            payload.carried_items.clear();
            game = Game::from_save(payload)?;
        }
    }
    for (item, depth) in fixture
        .preconditions
        .inventory_items
        .iter()
        .filter_map(|item| item.generation_depth.map(|depth| (item, depth)))
    {
        game.debug_add_generated_inventory_item(&item.id, &item.kind_id, depth)?;
    }
    game.debug_set_ability_casts_succeed(fixture.preconditions.debug_ability_casts_succeed);
    game.debug_set_recharge_attempts_succeed(fixture.preconditions.debug_recharge_attempts_succeed);
    game.debug_set_recharge_attempts_fail(fixture.preconditions.debug_recharge_attempts_fail);
    game.debug_set_recharge_sources_survive(fixture.preconditions.debug_recharge_sources_survive);
    game.debug_set_recall_delay_turns(fixture.preconditions.debug_recall_delay_turns);
    game.debug_set_item_curses_land(fixture.preconditions.debug_item_curses_land);
    game.debug_set_item_curses_resisted(fixture.preconditions.debug_item_curses_resisted);
    let mut events = Vec::new();
    let mut changed_cells = Vec::new();
    let mut removed_entities = Vec::new();
    let mut errors = Vec::new();

    for (index, contract_command) in fixture.commands.iter().enumerate() {
        let command = resolve_contract_command(&game, &contract_command.command)?;
        let envelope = GameCommandEnvelope {
            command_seq: contract_command
                .command_seq
                .unwrap_or_else(|| game.last_command_seq().saturating_add(1)),
            expected_revision: contract_command
                .expected_revision
                .unwrap_or(game.revision()),
            command,
        };
        match game.dispatch(envelope) {
            Ok(update) => {
                events.extend(update.events);
                changed_cells.extend(update.changed_cells.into_iter().map(|cell| cell.position));
                removed_entities.extend(update.removed_entities);
            }
            Err(error) => errors.push(CommandErrorAssertion {
                step: index + 1,
                kind: command_error_kind(&error)?,
                state_hash: game.state_hash(),
            }),
        }
    }

    let snapshot = game.snapshot();
    let save = game.to_save();
    let save_round_trip_state_hash = fixture
        .save_round_trip
        .then(|| save_round_trip(&game))
        .transpose()?;

    Ok(ContractAssertions {
        final_state: FinalStateAssertion {
            revision: snapshot.revision,
            turn: snapshot.turn,
            world_tick: snapshot.world_tick,
            last_command_seq: snapshot.last_command_seq,
            rng_draw_counter: game.rng_draw_counter(),
            floor_id: snapshot.floor_id.clone(),
            dungeon_instance_id: snapshot.dungeon_instance_id.clone(),
            town: snapshot.town.clone(),
            shops: snapshot.shops.clone(),
            homes: snapshot.homes.clone(),
            player_position: snapshot.player.position,
            player_hp: Some(snapshot.player.hp),
            player_max_hp: Some(snapshot.player.max_hp),
            player_attack: Some(snapshot.player.attack),
            player_defense: Some(snapshot.player.defense),
            player_speed: Some(snapshot.player.speed),
            player_energy_need: Some(snapshot.player.energy_need),
            player_carried_weight_tenths_pound: Some(snapshot.player.carried_weight_tenths_pound),
            player_carry_capacity_tenths_pound: Some(snapshot.player.carry_capacity_tenths_pound),
            player_encumbrance_speed_penalty: Some(snapshot.player.encumbrance_speed_penalty),
            player_gold: Some(snapshot.player.gold),
            player_nutrition: Some(snapshot.player.nutrition),
            player_statuses: snapshot.player.statuses.clone(),
            player_resistances: snapshot.player.resistances.clone(),
            player_level: Some(snapshot.player.progress.level),
            player_experience: Some(snapshot.player.progress.experience),
            player_max_level: Some(snapshot.player.progress.max_level),
            player_pending_attribute_increases: Some(
                snapshot.player.progress.pending_attribute_increases,
            ),
            player_attributes: Some(snapshot.player.progress.clone()),
            player_build: snapshot.player.build.clone(),
            player_resources: snapshot.player.resources.clone(),
            player_ability_learning: snapshot.player.ability_learning,
            player_abilities: snapshot.player.abilities.clone(),
            player_summon_command: snapshot.player.summon_command,
            player_recall: snapshot.player.recall,
            entity_count: snapshot.entities.len(),
            entities: snapshot
                .entities
                .iter()
                .map(|entity| {
                    let saved_entity = save.entities.iter().find(|saved| saved.id == entity.id);
                    ActorStateAssertion {
                        id: entity.id.clone(),
                        position: entity.position,
                        hp: entity.hp,
                        speed: entity.speed,
                        energy_need: entity.energy_need,
                        alerted: entity.alerted,
                        casting_cooldown_remaining: entity.casting_cooldown_remaining,
                        observed_player_resistances: entity.observed_player_resistances.clone(),
                        statuses: entity.statuses.clone(),
                        pack: saved_entity.and_then(|saved| saved.pack.clone()),
                        faction: entity.faction,
                        summon: saved_entity.and_then(|saved| saved.summon.clone()),
                        controller_id: saved_entity.and_then(|saved| saved.controller_id.clone()),
                    }
                })
                .collect(),
            ground_item_count: snapshot.items.len(),
            gold_piles: snapshot.gold_piles,
            inventory_stack_count: snapshot.inventory.len(),
            equipment_count: snapshot.equipment.len(),
            inventory: snapshot.inventory,
            equipment: snapshot.equipment,
            item_knowledge: save.item_knowledge,
            item_property_knowledge: save.item_property_knowledge,
            next_item_instance_serial: Some(save.next_item_instance_serial),
            next_gold_pile_serial: Some(save.next_gold_pile_serial),
            terrain_interactions: snapshot.terrain_interactions,
            tasks: snapshot.tasks,
            campaign: Some(snapshot.campaign),
            revealed_terrain: save.revealed_terrain,
            state_hash: snapshot.state_hash,
        },
        events,
        changed_cells,
        removed_entities,
        errors,
        save_round_trip_state_hash,
    })
}

pub fn verify(fixture: &ContractFixture) -> Result<(), ContractError> {
    let expected = fixture
        .assertions
        .as_ref()
        .ok_or_else(|| ContractError::MissingAssertions(fixture.id.clone()))?;
    let mut migrated_expected = None;
    if let Some(player_attributes) = expected.final_state.player_attributes.as_ref() {
        let maxima = [
            player_attributes.attributes.strength.maximum_natural,
            player_attributes.attributes.intelligence.maximum_natural,
            player_attributes.attributes.wisdom.maximum_natural,
            player_attributes.attributes.dexterity.maximum_natural,
            player_attributes.attributes.constitution.maximum_natural,
            player_attributes.attributes.charisma.maximum_natural,
        ];
        let all_legacy = maxima.iter().all(|maximum| *maximum == 0);
        if maxima.contains(&0) && !all_legacy {
            return Err(ContractError::IncompleteLegacyAttributeProjection(
                fixture.id.clone(),
            ));
        }
        if fixture.schema_version == 1 && all_legacy {
            let mut migrated = expected.clone();
            let progress = migrated
                .final_state
                .player_attributes
                .as_mut()
                .expect("checked player progress must remain available");
            for value in [
                &mut progress.attributes.strength,
                &mut progress.attributes.intelligence,
                &mut progress.attributes.wisdom,
                &mut progress.attributes.dexterity,
                &mut progress.attributes.constitution,
                &mut progress.attributes.charisma,
            ] {
                value.maximum_natural = value.natural;
            }
            migrated_expected = Some(migrated);
        }
    }
    let expected = migrated_expected.as_ref().unwrap_or(expected);
    let actual = observe(fixture)?;
    if &actual == expected {
        return Ok(());
    }
    Err(ContractError::AssertionMismatch {
        id: fixture.id.clone(),
        expected: serde_json::to_string_pretty(expected)?,
        actual: serde_json::to_string_pretty(&actual)?,
    })
}

pub fn validate_fixture_set(fixtures: &[ContractFixture]) -> Result<(), ContractError> {
    let mut ids = BTreeSet::new();
    for fixture in fixtures {
        validate_fixture(fixture)?;
        if !ids.insert(fixture.id.clone()) {
            return Err(ContractError::DuplicateId(fixture.id.clone()));
        }
    }
    Ok(())
}

fn validate_fixture(fixture: &ContractFixture) -> Result<(), ContractError> {
    if !(1..=CONTRACT_SCHEMA_VERSION).contains(&fixture.schema_version) {
        return Err(ContractError::UnsupportedSchema(fixture.schema_version));
    }
    if fixture.legacy_commit != LEGACY_BASELINE_COMMIT {
        return Err(ContractError::LegacyCommit(fixture.legacy_commit.clone()));
    }
    if fixture.preconditions.world != ORIGINAL_TEST_WORLD
        && fixture.preconditions.world != HISTORICAL_TEST_WORLD
        && fixture.preconditions.world != WARRENS_TEST_WORLD
    {
        return Err(ContractError::UnknownWorld(
            fixture.preconditions.world.clone(),
        ));
    }
    if fixture.id.trim().is_empty() {
        return Err(ContractError::EmptyId);
    }
    for item in &fixture.preconditions.inventory_items {
        if let Some(depth) = item.generation_depth
            && (item.quantity != 1
                || item.quality != ItemQualityDto::Ordinary
                || !item.affix_ids.is_empty()
                || !item.rolled_affixes.is_empty()
                || item.activation.is_some()
                || item.charges.is_some()
                || !item.enchantments.is_empty()
                || item.curse.is_some()
                || !(1..=100).contains(&depth))
        {
            return Err(ContractError::InvalidGeneratedItemPrecondition(
                item.id.clone(),
            ));
        }
    }
    if fixture.preconditions.debug_item_curses_land
        && fixture.preconditions.debug_item_curses_resisted
    {
        return Err(ContractError::ConflictingItemCurseDebugPreconditions);
    }
    if fixture
        .preconditions
        .debug_clear_entities_after_task_floor_entry
        && !fixture.preconditions.enter_task_floor
    {
        return Err(ContractError::TaskFloorSetupRequiresEntry);
    }
    if !fixture
        .preconditions
        .task_states_after_floor_entry
        .is_empty()
        && !fixture.preconditions.enter_task_floor
    {
        return Err(ContractError::TaskFloorSetupRequiresEntry);
    }
    Ok(())
}

fn parse_seed(seed: &str) -> Result<u64, ContractError> {
    if let Some(hex) = seed.strip_prefix("0x") {
        return u64::from_str_radix(hex, 16)
            .map_err(|_| ContractError::InvalidSeed(seed.to_owned()));
    }
    seed.parse::<u64>()
        .map_err(|_| ContractError::InvalidSeed(seed.to_owned()))
}

fn fixture_position_index(position: Position, width: u16, height: u16) -> Option<usize> {
    if position.x < 0
        || position.y < 0
        || position.x >= i32::from(width)
        || position.y >= i32::from(height)
    {
        return None;
    }
    Some(
        usize::try_from(position.y).ok()? * usize::from(width)
            + usize::try_from(position.x).ok()?,
    )
}

fn resolve_contract_command(
    game: &Game,
    command: &ContractCommandAction,
) -> Result<GameCommand, ContractError> {
    match command {
        ContractCommandAction::Protocol(command) => Ok(command.clone()),
        ContractCommandAction::Fixture(ContractOnlyCommand::BuyFirstFromShop {
            shop_id,
            quantity,
        }) => {
            let snapshot = game.snapshot();
            let shop = snapshot
                .shops
                .iter()
                .find(|shop| &shop.id == shop_id)
                .ok_or_else(|| ContractError::UnavailableShopSelection(shop_id.clone()))?;
            let item = shop
                .stock
                .first()
                .ok_or_else(|| ContractError::EmptyShopSelection(shop_id.clone()))?;
            Ok(GameCommand::BuyFromShop {
                shop_id: shop_id.clone(),
                item_id: item.id.clone(),
                quantity: *quantity,
            })
        }
    }
}

fn command_error_kind(error: &CoreError) -> Result<CommandErrorKind, ContractError> {
    match error {
        CoreError::RevisionMismatch { .. } => Ok(CommandErrorKind::RevisionMismatch),
        CoreError::CommandSequence { .. } => Ok(CommandErrorKind::CommandSequence),
        CoreError::PlayerDead => Ok(CommandErrorKind::PlayerDead),
        CoreError::CampaignEnded => Ok(CommandErrorKind::CampaignEnded),
        other => Err(ContractError::UnexpectedCoreError(other.to_string())),
    }
}

fn save_round_trip(game: &Game) -> Result<String, ContractError> {
    let snapshot = game.snapshot();
    let header = SaveHeaderV1 {
        format: "rfb-save".to_owned(),
        save_schema_version: SAVE_HEADER_SCHEMA_VERSION,
        game_version: env!("CARGO_PKG_VERSION").to_owned(),
        protocol_version: PROTOCOL_VERSION.to_owned(),
        slot_name: "契约回环".to_owned(),
        created_at: "2026-07-15T00:00:00Z".to_owned(),
        saved_at: "2026-07-15T00:01:00Z".to_owned(),
        character_summary: CharacterSummary {
            display_name: "原创契约测试探索者".to_owned(),
            level: snapshot.player.progress.level.into(),
            location_key: game.location_key().to_owned(),
            turn: snapshot.turn,
        },
        content_id: snapshot.content_id.clone(),
        content_hash: snapshot.content_hash.clone(),
        payload_encoding: "messagepack".to_owned(),
    };
    let bytes = rfb_save::encode(&header, &game.to_save())?;
    let (_, payload) = rfb_save::decode(&bytes)?;
    let restored = Game::from_save(payload)?;
    if restored.snapshot() != snapshot {
        return Err(ContractError::SaveRoundTripMismatch);
    }
    Ok(restored.state_hash())
}

#[derive(Debug, Error)]
pub enum ContractError {
    #[error("unsupported contract schema version {0}")]
    UnsupportedSchema(u16),
    #[error("contract fixture uses unexpected legacy commit {0}")]
    LegacyCommit(String),
    #[error("contract fixture uses unknown test world {0}")]
    UnknownWorld(String),
    #[error("contract fixture ID cannot be empty")]
    EmptyId,
    #[error("contract fixture references unknown entity precondition {0}")]
    UnknownEntityPrecondition(String),
    #[error("unknown dungeon campaign precondition {0}")]
    UnknownDungeonPrecondition(String),
    #[error("player progress precondition is unavailable in the generated save")]
    MissingProgressPrecondition,
    #[error("terrain precondition position is outside the active floor: {0:?}")]
    InvalidTerrainPrecondition(Position),
    #[error("player position precondition is not a walkable cell on the active floor: {0:?}")]
    InvalidPlayerPositionPrecondition(Position),
    #[error("generated item precondition is invalid for {0}")]
    InvalidGeneratedItemPrecondition(String),
    #[error("item curse debug preconditions cannot force both landing and resistance")]
    ConflictingItemCurseDebugPreconditions,
    #[error("task-floor post-entry preconditions require enterTaskFloor")]
    TaskFloorSetupRequiresEntry,
    #[error("fixture cannot select stock from unavailable shop {0}")]
    UnavailableShopSelection(String),
    #[error("fixture cannot select the first item from empty shop {0}")]
    EmptyShopSelection(String),
    #[error("duplicate contract fixture ID {0}")]
    DuplicateId(String),
    #[error("invalid contract seed {0}")]
    InvalidSeed(String),
    #[error("fixture {0} does not contain assertions")]
    MissingAssertions(String),
    #[error("fixture {0} has a partially populated legacy attribute projection")]
    IncompleteLegacyAttributeProjection(String),
    #[error("fixture {id} did not match\nexpected:\n{expected}\nactual:\n{actual}")]
    AssertionMismatch {
        id: String,
        expected: String,
        actual: String,
    },
    #[error("unexpected core error: {0}")]
    UnexpectedCoreError(String),
    #[error("save round trip changed the authoritative snapshot")]
    SaveRoundTripMismatch,
    #[error(transparent)]
    Save(#[from] rfb_save::SaveError),
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
