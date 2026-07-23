// SPDX-License-Identifier: MPL-2.0
// Game aggregate and rule orchestration.

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::Arc,
};

use crate::resistance::DamageType;
use crate::{
    action::GameAction,
    check::{CheckContext, CheckKind, resolve_check},
    combat::{
        adjacent, apply_melee_armor_reduction, monster_melee_skill, rating_to_armor_class,
        rating_to_combat_value,
    },
    effect::{
        DamageOutcome, DamagePacket, EffectOutcome, EffectSpec, EffectTarget, STATUS_BLEEDING,
        STATUS_FEAR, STATUS_HASTE, STATUS_POISON, STATUS_SLOW, STATUS_STUN, advance_status_ticks,
        apply_effect, resolve_damage,
    },
    error::CoreError,
    event::{DomainEvent, ProjectileTrace, project_events},
    rng::{RNG_ALGORITHM, RfbRng},
    save::{
        GENERATED_ITEM_ID_PREFIX, actor_from_entity, actor_from_player, actor_from_spawn,
        actors_to_save, carried_item_from_dto, carried_items_to_save,
        derive_next_item_instance_serial, equipment_item_from_dto, equipment_to_save,
        floor_connections_from_save, floor_connections_to_save, floor_from_save,
        floor_regions_from_save, floor_regions_to_save, floor_to_save, inventory_item_from_dto,
        inventory_to_save, item_from_dto, items_to_save, player_to_save, position_from_content,
        revealed_terrain_from_save,
    },
    scheduler::{
        INITIAL_MONSTER_ENERGY_NEED, INITIAL_PLAYER_ENERGY_NEED, STANDARD_ACTION_COST, gain_energy,
        spend_energy,
    },
    state::{
        Actor, EquipOutcome, FloorConnectionState, FloorRegionState, FloorState, ItemInstance,
        ItemLocation, MonsterPackIdentity,
    },
    stats::{DerivedStat, DerivedStatsPipeline, StatBounds, StatKind, StatLayer},
};
use rfb_content::{
    ActorRole, ContentCatalog, ContentPosition, EncounterEntryDefinition, EncounterFormation,
    EncounterTableDefinition, FloorLifecycle, ItemUseEffectDefinition, MonsterPackBehavior,
    ProceduralFloorDefinition, ProceduralLayoutMode, ProceduralMazeDefinition,
    ProceduralPitDefinition, ProceduralRoomGeometryDefinition, ProceduralRoomShape,
    ProceduralStreamerCandidateDefinition, RetakeFloorPolicy, TaskObjectiveDefinition,
    TaskObjectiveKind, TerrainFeatureEntryDefinition, TerrainFeaturePlacement,
    ThemeVaultCandidateDefinition, VaultDefinition, VaultTransform,
};
use rfb_protocol::{
    ActorSaveDto, AttackProfileDto, CarriedItemSaveDto, CellDto, CellLightDto, CellVisualDto,
    ContentVisualDto, DamageDiceDto, Direction, DungeonStateSaveDto, EntityDto, EquipmentItemDto,
    EquipmentItemSaveDto, FloorConnectionSaveDto, FloorRegionSaveDto, FloorSaveDto,
    GameCommandEnvelope, GameSnapshot, GameUpdate, InventoryItemDto, InventoryItemSaveDto, ItemDto,
    ItemIdentificationDto, ItemKnowledgeDto, ItemKnowledgeSaveDto, ItemPropertyDto,
    ItemPropertyKnowledgeSaveDto, ItemQualityDto, ItemSaveDto, MeleeBlowDto, MeleeRoutineDto,
    MonsterPackBehaviorDto, MonsterPackRoleDto, PROTOCOL_VERSION, PlayerDto, PlayerSaveDto,
    Position, ProjectileProfileDto, RngSaveDto, SavePayloadV1, StatModifiersDto, TargetModeDto,
    TargetSelection, TargetSpecDto, TaskStateSaveDto, TaskStatusDto, TaskStatusKindDto,
    TerrainInteractionDto, TerrainInteractionKindDto, TerrainInteractionUnavailableReasonDto,
    TerrainSaveDto, ThrowProfileDto, VisibilityState,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub const BUILT_IN_WORLD_ID: &str = "demo.world.original-v1";
const PREVIOUS_BUILT_IN_CONTENT_HASHES: [&str; 58] = [
    "880610557b208e7c2459ff876c4ace1cb2ef9903986cb7883a04d511ca13c025",
    "0a76daadea3a9683ea8173aa8f65e6195a5582bdf7fdad215cea1a2896dfefcc",
    "cd2c813d224189c925a940e60a915fe3dcf6efa0ccadfc7363d06d428f56525f",
    "36bdba260173b9ba7477e85b886c134affed0369aa4f7a485e59e4408e618ebd",
    "d0537220f093719e623b51bf589dd0a3d8a67ccdc534a1502adcebe094120e9b",
    "e597eb10e3eec454ea78e8ad4e874a8ef41732c6f497083f4fb698d9a1935c69",
    "ee3446edab3354c091bd1edc6e0b5e8d478fd090767fee6796614d9372286a53",
    "12ba3295dfa8a9884bc7464a78b7dbb9cded01409ff22777db02df85d1aabed7",
    "dc371da0d48375a811a6421f1ccaa2e1310daa7aab856f852388f7da1a04c2b5",
    "6449bc9fa8717d7f6ffc4a2a9643c8e40d20f04c196fa80f23bec2823de8e3d5",
    "ce3d3810b9be824f20230d83d5978dbb555f5766813b5ac43c059be0e6293fe0",
    "cb56a8e9dd6d7280b38fe4e388fc0f7ce08fd4a40cef2c8886907e3c662ffc96",
    "87e77fccea2c1ea40a6d952abf8d0b38d286c049b34b73f0da93f00288d1c2ae",
    "154f5c333d2e352ff13734823a8cfded3e513b545c7b2e934663954887c375cf",
    "479728aa3cead56c7dbf886a1beb4a9f20b5034085da8836cb82f2191246e979",
    "43b38c37bc03ae81f8fe1e5a3f3c8afeba47921ff05321011bc227fb5813387f",
    "52c3db16ad5240ff83ba652b09ef70cccac991a586b593f84c11956a55539596",
    "419260921954602e9b707dd8c260f80ad3ff1ad0504ea2dfbde739ec64ca2d54",
    "130f0f9fbddbdb12d7742d222e2e4deceabddb51810834c264da45678e15d474",
    "b37af3a660c95c024d12c8232b6b5467cb7d57982e09431748f1516ed3c550c3",
    "a3b8149e550f4211b496d6500171e52031baccc2223c7c60bbb1874cf2015cab",
    "bdefe542bb40a876ae29f1e504ad8d9c7fcbbc4e5eba8092d937782fb88a74c3",
    "febe50b7a55a637a05d78135f14aa8f72fa457632ae8d705c002e92acf9e4fd9",
    "51ffdccfe19a9f159adc15c2f62965ff4a5d44b55990eb9f29df96870937a043",
    "f060f44c88033e8ef75478929a354d6b5b0bc5f933ca2772e79c3440940942e8",
    "2d2900d8052b0a600346d0b87cc3b3d5bb5138f851abbf2b95afa196bbbaaca2",
    "e69258b4a303a38c10221f90d01c49628eb9ef737e97c7e777fe30070a025f81",
    "224e4cc12f1f1a99e245b5e1a96e7c9371a6873460b6197c0f18007542c1a079",
    "4fdb1018d89fadee287aeff70b2ca059f62b867cfd8db8ed7f6409f7bbbd4765",
    "8319b75e64585ef782358ed5287e087d14fab3626dfa854296696751f66896ac",
    "830b8ededc0dadb5600436137da7edb41353f945a09a4325d05546e16e75c4a8",
    "738d40e03f4c4eaebb91d47c74ad7decd7c13ddd12cc41238d177408f66ea0cf",
    "c390fb30dcc041b266ee895e72441cf656dbacc470a24ba86bd8d7b948be994f",
    "b44f98cea0cc7f125421faebf3085a23c79228be2573daca38acef63abcca6ea",
    "328600bfda30da20bd2efe7faac1f97eda03cccecb3ae0b36f4b683e74e5869e",
    "02df91742a4ad4daf3aebe88c397f0a70396e36f9afc293cd87bdc310715929b",
    "9ff7c821379c543d13fc5ee690a84c71fa4267f210381781a54378040a876403",
    "7a65a77e6fec214a86be9ba7e6abbbebae14c7a68094b628f55d5960002e0b4f",
    "b37398cb9d005302c958a9e300d07a435e8631d6a5cd44ba63b0086069577c43",
    "0e6cf15310644e7b3eb2f7acb0c18a8b1a7fb08739e981e7492d4079e61ab44a",
    "e03cb30ea8e1cd5821c14b54c4a038d30323cfc2cb6e0d6c483cbb006d70916f",
    "ae7b19dd780d73091a5b34aed2f67dcbc5650d2e2ed1d7748cc86f48020f8fb0",
    "9c8fc3226c20300a308d21a5da69033efb853169214f4c411e6c740800bdf9ad",
    "5d65fd9ca827dd05fc035650b82046edb592d563565c7e4075b32512a43f4e1f",
    "7eea25faef326b6d2250af357359902d0acf32d393c831655508a7e7eee5f2f0",
    "de045e1652d6e484937743b84a98e5e77887f28340a6492e72e8c6e1f72326e6",
    "1f8848e160b4ec51ca36acc512920946888fec20a36d7ac7b860bdb126aff79a",
    "11a28d24125572468148dce77f0082340ab82a3a7ef87637303578681b31c4e9",
    "e3c0d8653f86663c6bb7eb2cf99caf9d1ba5a259566560d7d70bb9592de2b1e9",
    "461242cb2164434a7ef44a3692f1c9fa4ffe9921f07c17e0857c96f2f2d95041",
    "d209d68a6a39af21eee8d1a951684be86e847ab570823c9c2604fa199e4571e1",
    "ee07c276bbe568fafc1e1d6942e9d57d158bd250ed452b32c01c774d8521e96d",
    "4cdcad204a7ccad6d67b8dcb50ccdcc188220a72d258c37219974fad51e5274d",
    "9789fcbbd8431ed745d8a0305cc81a54cc7e45ce79be86ed76e0227d66564a02",
    "56fc449617a4c05c12ff11716c14b4f5c680cada9ad86c6ece736b52fa904bc2",
    "9d25687c1296bc6f9953024bd76bb9eefc4c1e3955280b96d34d565ff7ca289d",
    "246f51864965fac494c7a39959f591caa0434d9fa4eac839501f9d09526eb617",
    "9f3e3d5dee1e8777179179259380990b9253aa7f195f08cd29cbbd58562793df",
];
const BUILT_IN_CONTENT_HASH: &str =
    "834acbe3d025810eb1399db74689d35a4d3dae34862bcbf1271c8d20ad11d9fc";
const BUILT_IN_CONTENT_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/rfb-demo-original.rfbcontent"));
const VISIBILITY_RADIUS: i32 = 8;
const BASE_THROW_RANGE_BUDGET: u16 = 50;
const MIN_THROW_RANGE: u16 = 2;
const MAX_THROW_RANGE: u16 = 10;
const AMBIENT_LIGHT: u8 = 28;
const PLAYER_LIGHT_RADIUS: i32 = 6;
const TERRAIN_INTERACTION_DIRECTIONS: [Direction; 8] = [
    Direction::North,
    Direction::NorthEast,
    Direction::East,
    Direction::SouthEast,
    Direction::South,
    Direction::SouthWest,
    Direction::West,
    Direction::NorthWest,
];
const ACTOR_LIGHT_RADIUS: i32 = 5;
const ITEM_LIGHT_RADIUS: i32 = 4;
const PLAYER_LIGHT_COLOR: u32 = 0xffd7a3;
const ACTOR_LIGHT_COLOR: u32 = 0xff8a4c;
const ITEM_LIGHT_COLOR: u32 = 0x8ad9ff;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StateHashPayloadV25 {
    schema_version: u16,
    revision: u32,
    turn: u32,
    world_tick: u32,
    last_command_seq: u32,
    terrain: TerrainSaveDto,
    player: PlayerSaveDto,
    entities: Vec<ActorSaveDto>,
    items: Vec<ItemSaveDto>,
    inventory: Vec<InventoryItemSaveDto>,
    equipment: Vec<EquipmentItemSaveDto>,
    carried_items: Vec<CarriedItemSaveDto>,
    item_knowledge: Vec<ItemKnowledgeSaveDto>,
    item_property_knowledge: Vec<ItemPropertyKnowledgeSaveDto>,
    task_states: Vec<TaskStateSaveDto>,
    dungeon_states: Vec<DungeonStateSaveDto>,
    next_item_instance_serial: u64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    explored: Vec<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    revealed_terrain: Vec<Position>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    floor_connections: Vec<FloorConnectionSaveDto>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    floor_regions: Vec<FloorRegionSaveDto>,
    rng: RngSaveDto,
    content_id: String,
    content_hash: String,
    world_id: String,
    current_floor_id: String,
    current_dungeon_instance_id: Option<String>,
    stored_floors: Vec<FloorSaveDto>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LootContext {
    table_id: String,
    floor_id: String,
    depth: u16,
    source: LootSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LootSource {
    MonsterCarried { actor_id: String },
    MonsterDeath { actor_id: String },
    FloorRoom { room_id: String, spawn_id: String },
    Vault { vault_id: String, spawn_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GeneratedRoom {
    id: String,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    shape: ProceduralRoomShape,
}

impl GeneratedRoom {
    fn center(&self) -> Position {
        Position {
            x: self.x + self.width / 2,
            y: self.y + self.height / 2,
        }
    }

    fn contains(&self, position: Position) -> bool {
        if position.x < self.x
            || position.x >= self.x + self.width
            || position.y < self.y
            || position.y >= self.y + self.height
        {
            return false;
        }
        match self.shape {
            ProceduralRoomShape::Rectangle => true,
            ProceduralRoomShape::Cross => {
                position.x == self.center().x || position.y == self.center().y
            }
        }
    }

    fn area(&self) -> u32 {
        match self.shape {
            ProceduralRoomShape::Rectangle => (self.width * self.height) as u32,
            ProceduralRoomShape::Cross => (self.width + self.height - 1) as u32,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GeneratedVaultPlacement {
    vault: VaultDefinition,
    origin: Position,
    transform: VaultTransform,
    ordinal: u16,
    connector_cells: Vec<Position>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GeneratedVaultPlacementCandidate {
    origin: Position,
    transform: VaultTransform,
    connector_cells: Vec<Position>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GeneratedPitPlacement {
    definition: ProceduralPitDefinition,
    origin: Position,
    outer_entrance: Position,
    inner_entrance: Position,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GeneratedTerrainFeature {
    terrain_id: String,
    position: Position,
}

struct TerrainFeaturePlacementContext<'a> {
    rooms: &'a [GeneratedRoom],
    reserved: &'a BTreeSet<Position>,
    floor_terrain_id: &'a str,
    room_floor_terrain_ids: &'a BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GeneratedRegion {
    state: FloorRegionState,
    room_ids: Vec<String>,
    floor_terrain_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TaskState {
    status: TaskStatusKindDto,
    stage_index: u32,
    current: u32,
    required: u32,
    active_floor_id: Option<String>,
    retakes_used: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DungeonState {
    guardian_defeated: bool,
    next_instance_ordinal: u32,
}

struct TaskRestoreContext<'a> {
    current_floor_id: &'a str,
    terrain: &'a [String],
    stored_floors: &'a BTreeMap<String, FloorState>,
    entities: &'a [Actor],
    items: &'a [ItemInstance],
    legacy_progress: &'a BTreeMap<String, u32>,
    saved_states: &'a [TaskStateSaveDto],
    allow_missing_states: bool,
}

fn floor_task_id(floor: &ProceduralFloorDefinition) -> &str {
    floor.task_id.as_deref().unwrap_or(&floor.id)
}

fn initial_dungeon_states(world: &rfb_content::WorldDefinition) -> BTreeMap<String, DungeonState> {
    world
        .dungeons
        .iter()
        .map(|dungeon| {
            (
                dungeon.id.clone(),
                DungeonState {
                    guardian_defeated: false,
                    next_instance_ordinal: 0,
                },
            )
        })
        .collect()
}

fn restore_dungeon_states(
    world: &rfb_content::WorldDefinition,
    saved_states: &[DungeonStateSaveDto],
    allow_missing_states: bool,
) -> Result<BTreeMap<String, DungeonState>, CoreError> {
    let mut states = initial_dungeon_states(world);
    if saved_states.is_empty() {
        return Ok(states);
    }
    let mut restored = BTreeMap::new();
    for saved in saved_states {
        if !states.contains_key(&saved.dungeon_id)
            || restored
                .insert(
                    saved.dungeon_id.clone(),
                    DungeonState {
                        guardian_defeated: saved.guardian_defeated,
                        next_instance_ordinal: saved.next_instance_ordinal,
                    },
                )
                .is_some()
        {
            return Err(CoreError::InvalidSave("dungeon state is invalid"));
        }
    }
    if !allow_missing_states && restored.len() != states.len() {
        return Err(CoreError::InvalidSave("dungeon state set is incomplete"));
    }
    states.extend(restored);
    Ok(states)
}

fn dungeon_instance_storage_key(instance_id: Option<&str>, floor_id: &str) -> String {
    match instance_id {
        Some(instance_id) => format!("{instance_id}::{floor_id}"),
        None => floor_id.to_owned(),
    }
}

fn floor_dungeon_id(world: &rfb_content::WorldDefinition, floor_id: &str) -> Option<String> {
    world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == floor_id)
        .and_then(|floor| floor.dungeon_id.clone())
}

fn dungeon_instance_id(dungeon_id: &str, ordinal: u32) -> String {
    format!("{dungeon_id}.instance.{ordinal}")
}

fn parse_dungeon_instance_ordinal(instance_id: &str, dungeon_id: &str) -> Option<u32> {
    instance_id
        .strip_prefix(&format!("{dungeon_id}.instance."))
        .and_then(|ordinal| ordinal.parse::<u32>().ok())
        .filter(|ordinal| *ordinal > 0)
}

fn task_objectives<'a>(
    world: &'a rfb_content::WorldDefinition,
    task_id: &str,
) -> Vec<&'a TaskObjectiveDefinition> {
    if let Some(stages) = world
        .procedural_floors
        .iter()
        .find(|floor| floor_task_id(floor) == task_id && !floor.task_stages.is_empty())
        .map(|floor| floor.task_stages.iter().collect::<Vec<_>>())
    {
        return stages;
    }
    world
        .procedural_floors
        .iter()
        .find(|floor| floor_task_id(floor) == task_id)
        .and_then(|floor| floor.task_objective.as_ref())
        .into_iter()
        .collect()
}

fn task_succeeded(world: &rfb_content::WorldDefinition, task_id: &str, state: &TaskState) -> bool {
    let objectives = task_objectives(world, task_id);
    usize::try_from(state.stage_index)
        .ok()
        .is_some_and(|stage| stage + 1 == objectives.len())
        && state.current >= state.required
}

fn task_death_target_kind(event: &DomainEvent) -> Option<&str> {
    match event {
        DomainEvent::PlayerSlew { target_kind_id, .. }
        | DomainEvent::ProjectileSlew { target_kind_id, .. }
        | DomainEvent::ItemThrowSlew { target_kind_id, .. }
        | DomainEvent::EntityDiedFromStatus { target_kind_id, .. } => Some(target_kind_id.as_str()),
        _ => None,
    }
}

fn initial_task_states(world: &rfb_content::WorldDefinition) -> BTreeMap<String, TaskState> {
    let mut states = BTreeMap::new();
    for floor in world
        .procedural_floors
        .iter()
        .filter(|floor| floor.lifecycle == FloorLifecycle::OneShot)
    {
        states
            .entry(floor_task_id(floor).to_owned())
            .or_insert_with(|| TaskState {
                status: TaskStatusKindDto::Available,
                stage_index: 0,
                current: 0,
                required: task_objectives(world, floor_task_id(floor))
                    .first()
                    .map_or(1, |objective| objective.required),
                active_floor_id: None,
                retakes_used: 0,
            });
    }
    states
}

fn restore_task_states(
    world: &rfb_content::WorldDefinition,
    context: TaskRestoreContext<'_>,
) -> Result<BTreeMap<String, TaskState>, CoreError> {
    let mut states = initial_task_states(world);
    if !context.saved_states.is_empty() {
        let mut restored = BTreeMap::new();
        for saved in context.saved_states {
            let Some(expected) = states.get(&saved.task_id) else {
                return Err(CoreError::InvalidSave("task state ID is invalid"));
            };
            let objectives = task_objectives(world, &saved.task_id);
            let Some(objective) = usize::try_from(saved.stage_index)
                .ok()
                .and_then(|stage| objectives.get(stage))
            else {
                return Err(CoreError::InvalidSave("task stage is invalid"));
            };
            let members = world
                .procedural_floors
                .iter()
                .filter(|floor| floor_task_id(floor) == saved.task_id)
                .collect::<Vec<_>>();
            let active_floor_is_valid = saved.active_floor_id.as_ref().is_some_and(|floor_id| {
                floor_id == context.current_floor_id
                    && members.iter().any(|floor| floor.id == *floor_id)
            });
            let paused_floor_exists = members.iter().any(|floor| {
                context
                    .stored_floors
                    .values()
                    .any(|stored| stored.id == floor.id)
            });
            let max_retakes = members.first().and_then(|floor| floor.max_retakes);
            let status_is_valid = match saved.status {
                TaskStatusKindDto::Active => active_floor_is_valid,
                TaskStatusKindDto::Paused => saved.active_floor_id.is_none() && paused_floor_exists,
                TaskStatusKindDto::Completed => {
                    saved.active_floor_id.is_none()
                        && usize::try_from(saved.stage_index)
                            .ok()
                            .is_some_and(|stage| stage + 1 == objectives.len())
                        && saved.current == saved.required
                }
                TaskStatusKindDto::Available
                | TaskStatusKindDto::Failed
                | TaskStatusKindDto::Abandoned => saved.active_floor_id.is_none(),
            };
            if (saved.stage_index == 0 && expected.required != objective.required)
                || saved.required != objective.required
                || saved.current > saved.required
                || max_retakes.is_some_and(|maximum| saved.retakes_used > maximum)
                || !status_is_valid
                || restored
                    .insert(
                        saved.task_id.clone(),
                        TaskState {
                            status: saved.status,
                            stage_index: saved.stage_index,
                            current: saved.current,
                            required: saved.required,
                            active_floor_id: saved.active_floor_id.clone(),
                            retakes_used: saved.retakes_used,
                        },
                    )
                    .is_some()
            {
                return Err(CoreError::InvalidSave("task state is invalid"));
            }
        }
        if restored.len() != states.len() && !context.allow_missing_states {
            return Err(CoreError::InvalidSave("task state set is incomplete"));
        }
        states.extend(restored);
        return Ok(states);
    }

    let surface_terrain = if context.current_floor_id == world.initial_floor_id {
        Some(context.terrain)
    } else {
        context
            .stored_floors
            .get(&world.initial_floor_id)
            .map(|floor| floor.terrain.as_slice())
    };
    for (task_id, state) in &mut states {
        let members = world
            .procedural_floors
            .iter()
            .filter(|floor| floor_task_id(floor) == task_id)
            .collect::<Vec<_>>();
        let active = members
            .iter()
            .copied()
            .find(|floor| floor.id == context.current_floor_id);
        state.status = if active.is_some() {
            TaskStatusKindDto::Active
        } else if surface_terrain.is_some_and(|surface| {
            members.iter().any(|floor| {
                floor
                    .completed_entry_terrain_id
                    .as_ref()
                    .is_some_and(|id| surface.contains(id))
            })
        }) {
            TaskStatusKindDto::Completed
        } else if surface_terrain.is_some_and(|surface| {
            members.iter().any(|floor| {
                floor
                    .failed_entry_terrain_id
                    .as_ref()
                    .is_some_and(|id| surface.contains(id))
            })
        }) {
            TaskStatusKindDto::Failed
        } else if surface_terrain.is_some_and(|surface| {
            members.iter().any(|floor| {
                floor
                    .abandoned_entry_terrain_id
                    .as_ref()
                    .is_some_and(|id| surface.contains(id))
            })
        }) {
            TaskStatusKindDto::Abandoned
        } else if members.iter().any(|floor| {
            context
                .stored_floors
                .values()
                .any(|stored| stored.id == floor.id)
        }) {
            TaskStatusKindDto::Paused
        } else {
            TaskStatusKindDto::Available
        };
        state.active_floor_id = active.map(|floor| floor.id.clone());
        state.stage_index = 0;
        state.current = context.legacy_progress.get(task_id).copied().unwrap_or(0);
        if state.status == TaskStatusKindDto::Completed {
            state.current = state.required;
        } else if let Some(floor) = active {
            let objective = floor
                .task_objective
                .as_ref()
                .expect("validated task objective must remain available");
            match objective.kind {
                TaskObjectiveKind::CollectItem => {
                    if objective.item_instance_id.as_ref().is_some_and(|id| {
                        context.items.iter().any(|item| {
                            &item.id == id
                                && matches!(
                                    item.location,
                                    ItemLocation::Inventory | ItemLocation::Equipped { .. }
                                )
                        })
                    }) {
                        state.current = 1;
                    }
                }
                TaskObjectiveKind::KillActor => {
                    if objective
                        .actor_instance_id
                        .as_ref()
                        .is_some_and(|id| !context.entities.iter().any(|entity| &entity.id == id))
                    {
                        state.current = 1;
                    }
                }
                TaskObjectiveKind::EnterFloor | TaskObjectiveKind::KillActorKind => {}
            }
        }
        state.current = state.current.min(state.required);
    }
    Ok(states)
}

#[derive(Debug, Clone)]
pub struct Game {
    content: Arc<ContentCatalog>,
    world_id: String,
    current_floor_id: String,
    current_dungeon_instance_id: Option<String>,
    stored_floors: BTreeMap<String, FloorState>,
    width: u16,
    height: u16,
    terrain: Vec<String>,
    player: Actor,
    entities: Vec<Actor>,
    items: Vec<ItemInstance>,
    item_knowledge: BTreeMap<String, ItemKnowledgeState>,
    item_property_knowledge: BTreeMap<String, ItemPropertyKnowledgeState>,
    task_states: BTreeMap<String, TaskState>,
    dungeon_states: BTreeMap<String, DungeonState>,
    next_item_instance_serial: u64,
    explored: Vec<bool>,
    revealed_terrain: BTreeSet<Position>,
    floor_connections: Vec<FloorConnectionState>,
    floor_regions: Vec<FloorRegionState>,
    rng: RfbRng,
    revision: u32,
    turn: u32,
    world_tick: u32,
    last_command_seq: u32,
}

impl Game {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self::from_content(
            seed,
            load_built_in_content().expect("built-in content should decode"),
            BUILT_IN_WORLD_ID,
        )
        .expect("built-in world should create a game")
    }

    pub fn from_content(
        seed: u64,
        content: Arc<ContentCatalog>,
        world_id: &str,
    ) -> Result<Self, CoreError> {
        let world = content
            .world(world_id)
            .ok_or_else(|| CoreError::UnknownWorld(world_id.to_owned()))?;
        let width = world.width;
        let height = world.height;
        let mut terrain =
            vec![world.fill_terrain_id.clone(); usize::from(width) * usize::from(height)];
        for y in 0..height {
            for x in 0..width {
                if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
                    terrain[usize::from(y) * usize::from(width) + usize::from(x)] =
                        world.border_terrain_id.clone();
                }
            }
        }
        for terrain_override in &world.terrain_overrides {
            for position in &terrain_override.positions {
                terrain[usize::from(position.y) * usize::from(width) + usize::from(position.x)] =
                    terrain_override.terrain_id.clone();
            }
        }
        let player_definition = content
            .actor(&world.player.kind_id)
            .ok_or_else(|| CoreError::UnknownActor(world.player.kind_id.clone()))?;
        let player = actor_from_spawn(
            &world.player.instance_id,
            &world.player.kind_id,
            world.player.position,
            player_definition.max_hp,
            player_definition.speed,
            INITIAL_PLAYER_ENERGY_NEED,
        );
        let entities = world
            .actors
            .iter()
            .map(|spawn| {
                let definition = content
                    .actor(&spawn.kind_id)
                    .ok_or_else(|| CoreError::UnknownActor(spawn.kind_id.clone()))?;
                Ok(actor_from_spawn(
                    &spawn.instance_id,
                    &spawn.kind_id,
                    spawn.position,
                    definition.max_hp,
                    definition.speed,
                    INITIAL_MONSTER_ENERGY_NEED,
                ))
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
        let items = world
            .items
            .iter()
            .map(|spawn| ItemInstance {
                id: spawn.instance_id.clone(),
                kind_id: spawn.kind_id.clone(),
                quantity: spawn.quantity,
                quality: item_quality_dto(spawn.quality),
                affix_ids: spawn.affix_ids.clone(),
                location: ItemLocation::Ground(position_from_content(spawn.position)),
            })
            .collect::<Vec<_>>();
        let next_item_instance_serial =
            derive_next_item_instance_serial(&player, &entities, &items)?;
        let initial_floor_id = world.initial_floor_id.clone();
        let task_states = initial_task_states(world);
        let dungeon_states = initial_dungeon_states(world);
        let mut game = Self {
            content,
            world_id: world_id.to_owned(),
            current_floor_id: initial_floor_id,
            current_dungeon_instance_id: None,
            stored_floors: BTreeMap::new(),
            width,
            height,
            terrain,
            player,
            entities,
            items,
            item_knowledge: BTreeMap::new(),
            item_property_knowledge: BTreeMap::new(),
            task_states,
            dungeon_states,
            next_item_instance_serial,
            explored: vec![false; usize::from(width) * usize::from(height)],
            revealed_terrain: BTreeSet::new(),
            floor_connections: Vec::new(),
            floor_regions: Vec::new(),
            rng: RfbRng::seeded(seed),
            revision: 0,
            turn: 0,
            world_tick: 0,
            last_command_seq: 0,
        };
        game.initialize_carried_loot()?;
        game.reveal_current_visibility();
        game.validate_state()?;
        Ok(game)
    }

    pub fn from_save(payload: SavePayloadV1) -> Result<Self, CoreError> {
        Self::from_save_with_content(
            payload,
            load_built_in_content().expect("built-in content should decode"),
        )
    }

    pub fn from_save_with_content(
        payload: SavePayloadV1,
        content: Arc<ContentCatalog>,
    ) -> Result<Self, CoreError> {
        if payload.schema_version != 1 {
            return Err(CoreError::UnsupportedSaveVersion(payload.schema_version));
        }
        if payload.content_id != content.pack_id()
            || (payload.content_hash != content.content_hash()
                && !(content.pack_id() == "rfb.demo.original-v1"
                    && content.content_hash() == BUILT_IN_CONTENT_HASH
                    && PREVIOUS_BUILT_IN_CONTENT_HASHES.contains(&payload.content_hash.as_str())))
        {
            return Err(CoreError::ContentMismatch);
        }
        let migrating_previous_content = payload.content_hash != content.content_hash();
        let world = content
            .world(&payload.world_id)
            .ok_or_else(|| CoreError::UnknownWorld(payload.world_id.clone()))?;
        let mut legacy_task_progress = BTreeMap::new();
        for progress in &payload.task_progress {
            let Some(floor) = world
                .procedural_floors
                .iter()
                .find(|floor| floor_task_id(floor) == progress.task_id)
                .or_else(|| {
                    world
                        .procedural_floors
                        .iter()
                        .find(|floor| floor.id == progress.task_id)
                })
            else {
                return Err(CoreError::InvalidSave("task progress floor ID is invalid"));
            };
            let task_id = floor_task_id(floor).to_owned();
            let required = floor
                .task_objective
                .as_ref()
                .map_or(1, |objective| objective.required);
            if progress.current > required
                || legacy_task_progress
                    .insert(task_id, progress.current)
                    .is_some()
            {
                return Err(CoreError::InvalidSave("task progress is invalid"));
            }
        }
        let current_floor_id = if payload.current_floor_id.is_empty() {
            world.initial_floor_id.clone()
        } else {
            payload.current_floor_id.clone()
        };
        if current_floor_id != world.initial_floor_id
            && !world
                .procedural_floors
                .iter()
                .any(|floor| floor.id == current_floor_id)
        {
            return Err(CoreError::InvalidSave("current floor ID is invalid"));
        }
        let mut current_dungeon_instance_id = payload.current_dungeon_instance_id.clone();
        if let Some(dungeon_id) = floor_dungeon_id(world, &current_floor_id) {
            if current_dungeon_instance_id.is_none() {
                current_dungeon_instance_id = Some(dungeon_instance_id(&dungeon_id, 1));
            }
            if current_dungeon_instance_id
                .as_deref()
                .and_then(|instance| parse_dungeon_instance_ordinal(instance, &dungeon_id))
                .is_none()
            {
                return Err(CoreError::InvalidSave(
                    "current dungeon instance ID is invalid",
                ));
            }
        } else if current_dungeon_instance_id.is_some() {
            return Err(CoreError::InvalidSave(
                "surface or task floor cannot have a dungeon instance ID",
            ));
        }
        let expected_len = usize::from(payload.terrain.width) * usize::from(payload.terrain.height);
        if expected_len == 0 || payload.terrain.terrain_ids.len() != expected_len {
            return Err(CoreError::InvalidSave("terrain dimensions are invalid"));
        }
        let terrain = payload
            .terrain
            .terrain_ids
            .iter()
            .map(|id| {
                content
                    .terrain(id)
                    .map(|_| id.clone())
                    .ok_or_else(|| CoreError::UnknownTerrain(id.clone()))
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
        let player = actor_from_player(payload.player, &content)?;
        let entities = payload
            .entities
            .into_iter()
            .map(|entity| actor_from_entity(entity, &content))
            .collect::<Result<Vec<_>, CoreError>>()?;
        let mut items = payload
            .items
            .into_iter()
            .map(item_from_dto)
            .collect::<Vec<_>>();
        items.extend(
            payload
                .inventory
                .into_iter()
                .map(|item| inventory_item_from_dto(item, &content))
                .collect::<Result<Vec<_>, CoreError>>()?,
        );
        items.extend(
            payload
                .equipment
                .into_iter()
                .map(|item| equipment_item_from_dto(item, &content))
                .collect::<Result<Vec<_>, CoreError>>()?,
        );
        items.extend(
            payload
                .carried_items
                .into_iter()
                .map(|item| carried_item_from_dto(item, &content))
                .collect::<Result<Vec<_>, CoreError>>()?,
        );
        let mut stored_floors = BTreeMap::new();
        for floor in payload.stored_floors {
            let mut floor = floor;
            if floor.dungeon_instance_id.is_none()
                && let Some(dungeon_id) = floor_dungeon_id(world, &floor.id)
            {
                floor.dungeon_instance_id = Some(
                    current_dungeon_instance_id
                        .as_deref()
                        .filter(|_| {
                            current_floor_id != world.initial_floor_id
                                && floor_dungeon_id(world, &current_floor_id).as_deref()
                                    == Some(dungeon_id.as_str())
                        })
                        .map_or_else(|| dungeon_instance_id(&dungeon_id, 1), str::to_owned),
                );
            }
            let floor = floor_from_save(floor, &content)?;
            let storage_key =
                dungeon_instance_storage_key(floor.dungeon_instance_id.as_deref(), &floor.id);
            if (floor.id == current_floor_id
                && floor.dungeon_instance_id == current_dungeon_instance_id)
                || (floor.id != world.initial_floor_id
                    && !world
                        .procedural_floors
                        .iter()
                        .any(|definition| definition.id == floor.id))
                || stored_floors.insert(storage_key, floor).is_some()
            {
                return Err(CoreError::InvalidSave("stored floor state is invalid"));
            }
        }
        if current_floor_id == world.initial_floor_id {
            stored_floors.retain(|floor_id, _| {
                world.procedural_floors.iter().any(|floor| {
                    floor.id == *floor_id
                        && floor.lifecycle == FloorLifecycle::OneShot
                        && floor.retakeable
                })
            });
        }
        let mut allocator_entities = entities.clone();
        let mut allocator_items = items.clone();
        for floor in stored_floors.values() {
            allocator_entities.extend(floor.entities.iter().cloned());
            allocator_items.extend(floor.items.iter().cloned());
        }
        let derived_next_item_instance_serial =
            derive_next_item_instance_serial(&player, &allocator_entities, &allocator_items)?;
        let next_item_instance_serial = if payload.next_item_instance_serial == 0 {
            derived_next_item_instance_serial
        } else if payload.next_item_instance_serial < derived_next_item_instance_serial {
            return Err(CoreError::InvalidSave(
                "item instance allocator is behind existing IDs",
            ));
        } else {
            payload.next_item_instance_serial
        };
        let mut explored = payload.explored;
        if explored.is_empty() {
            explored = vec![false; expected_len];
        } else if explored.len() != expected_len {
            return Err(CoreError::InvalidSave(
                "exploration memory dimensions are invalid",
            ));
        }
        let revealed_terrain = revealed_terrain_from_save(
            payload.revealed_terrain,
            &terrain,
            payload.terrain.width,
            payload.terrain.height,
            &content,
        )?;
        let floor_connections = floor_connections_from_save(
            payload.floor_connections.clone(),
            payload.terrain.width,
            payload.terrain.height,
        )?;
        let floor_regions = floor_regions_from_save(
            payload.floor_regions.clone(),
            payload.terrain.width,
            payload.terrain.height,
            &content,
        )?;
        let item_knowledge = item_knowledge_from_save(payload.item_knowledge, &content)?;
        let mut item_property_knowledge =
            item_property_knowledge_from_save(payload.item_property_knowledge, &items, &content)?;
        for item in &items {
            if matches!(item.location, ItemLocation::Equipped { .. }) {
                let knowledge = item_property_knowledge.entry(item.id.clone()).or_default();
                knowledge.appraised = true;
                knowledge.identified = true;
                knowledge
                    .known_affix_ids
                    .extend(item.affix_ids.iter().cloned());
            }
        }
        let task_states = restore_task_states(
            world,
            TaskRestoreContext {
                current_floor_id: &current_floor_id,
                terrain: &terrain,
                stored_floors: &stored_floors,
                entities: &entities,
                items: &items,
                legacy_progress: &legacy_task_progress,
                saved_states: &payload.task_states,
                allow_missing_states: migrating_previous_content,
            },
        )?;
        let mut dungeon_states =
            restore_dungeon_states(world, &payload.dungeon_states, migrating_previous_content)?;
        for instance_id in current_dungeon_instance_id.iter().chain(
            stored_floors
                .values()
                .filter_map(|floor| floor.dungeon_instance_id.as_ref()),
        ) {
            if let Some(dungeon_id) = floor_dungeon_id(world, &current_floor_id).or_else(|| {
                stored_floors
                    .values()
                    .find(|floor| floor.dungeon_instance_id.as_deref() == Some(instance_id))
                    .and_then(|floor| floor_dungeon_id(world, &floor.id))
            }) && let Some(ordinal) = parse_dungeon_instance_ordinal(instance_id, &dungeon_id)
                && let Some(state) = dungeon_states.get_mut(&dungeon_id)
            {
                state.next_instance_ordinal = state.next_instance_ordinal.max(ordinal);
            }
        }
        let mut game = Self {
            content,
            world_id: payload.world_id,
            current_floor_id,
            current_dungeon_instance_id,
            stored_floors,
            width: payload.terrain.width,
            height: payload.terrain.height,
            terrain,
            player,
            entities,
            items,
            item_knowledge,
            item_property_knowledge,
            task_states,
            dungeon_states,
            next_item_instance_serial,
            explored,
            revealed_terrain,
            floor_connections,
            floor_regions,
            rng: RfbRng::from_save(&payload.rng)?,
            revision: payload.revision,
            turn: payload.turn,
            world_tick: payload.world_tick,
            last_command_seq: payload.last_command_seq,
        };
        if migrating_previous_content {
            let world = game
                .content
                .world(&game.world_id)
                .expect("restored world must remain available")
                .clone();
            if !floor_connections_are_valid(
                &game.current_floor_id,
                game.width,
                game.height,
                &game.terrain,
                &game.floor_connections,
                &world,
            ) {
                game.floor_connections.clear();
            }
            for floor in game.stored_floors.values_mut() {
                if !floor_connections_are_valid(
                    &floor.id,
                    floor.width,
                    floor.height,
                    &floor.terrain,
                    &floor.connections,
                    &world,
                ) {
                    floor.connections.clear();
                }
            }
        }
        game.reveal_current_visibility();
        game.validate_state()?;
        Ok(game)
    }

    #[must_use]
    pub fn to_save(&self) -> SavePayloadV1 {
        SavePayloadV1 {
            schema_version: 1,
            revision: self.revision,
            turn: self.turn,
            world_tick: self.world_tick,
            last_command_seq: self.last_command_seq,
            terrain: TerrainSaveDto {
                width: self.width,
                height: self.height,
                terrain_ids: self.terrain.clone(),
            },
            player: player_to_save(&self.player),
            entities: actors_to_save(&self.entities),
            items: items_to_save(&self.items),
            inventory: inventory_to_save(&self.items),
            equipment: equipment_to_save(&self.items),
            carried_items: carried_items_to_save(&self.items),
            item_knowledge: self.item_knowledge_to_save(),
            item_property_knowledge: self.item_property_knowledge_to_save(),
            task_progress: Vec::new(),
            task_states: self.task_states_to_save(),
            dungeon_states: self.dungeon_states_to_save(),
            next_item_instance_serial: self.next_item_instance_serial,
            explored: self.explored.clone(),
            revealed_terrain: self.revealed_terrain.iter().copied().collect(),
            floor_connections: floor_connections_to_save(&self.floor_connections),
            floor_regions: floor_regions_to_save(&self.floor_regions),
            rng: self.rng.to_save(),
            content_id: self.content.pack_id().to_owned(),
            content_hash: self.content.content_hash().to_owned(),
            world_id: self.world_id.clone(),
            current_floor_id: self.current_floor_id.clone(),
            current_dungeon_instance_id: self.current_dungeon_instance_id.clone(),
            stored_floors: self.stored_floors.values().map(floor_to_save).collect(),
        }
    }

    #[must_use]
    pub fn snapshot(&self) -> GameSnapshot {
        let mut cells = Vec::with_capacity(self.terrain.len());
        for y in 0..self.height {
            for x in 0..self.width {
                cells.push(self.cell_dto(Position {
                    x: i32::from(x),
                    y: i32::from(y),
                }));
            }
        }
        let visual_cells = self.visual_cells();
        GameSnapshot {
            protocol_version: PROTOCOL_VERSION.to_owned(),
            revision: self.revision,
            turn: self.turn,
            world_tick: self.world_tick,
            last_command_seq: self.last_command_seq,
            width: self.width,
            height: self.height,
            cells,
            visual_cells,
            player: self.player_dto(),
            entities: self.entities_dto(),
            items: self.items_dto(),
            inventory: self.inventory_dto(),
            equipment: self.equipment_dto(),
            content_id: self.content.pack_id().to_owned(),
            content_hash: self.content.content_hash().to_owned(),
            content_visuals: self.content_visuals(),
            world_id: self.world_id.clone(),
            floor_id: self.current_floor_id.clone(),
            dungeon_instance_id: self.current_dungeon_instance_id.clone(),
            terrain_interactions: self.terrain_interactions(),
            tasks: self.task_statuses(),
            state_hash: self.state_hash(),
        }
    }

    pub fn dispatch(&mut self, envelope: GameCommandEnvelope) -> Result<GameUpdate, CoreError> {
        if envelope.expected_revision != self.revision {
            return Err(CoreError::RevisionMismatch {
                expected: self.revision,
                received: envelope.expected_revision,
            });
        }
        let expected_seq = self.last_command_seq.saturating_add(1);
        if envelope.command_seq != expected_seq {
            return Err(CoreError::CommandSequence {
                expected: expected_seq,
                received: envelope.command_seq,
            });
        }
        if self.player_is_dead() {
            return Err(CoreError::PlayerDead);
        }

        let base_revision = self.revision;
        let previous_visuals = self.visual_cells();
        let mut changed = BTreeSet::new();
        let mut events = Vec::new();
        let mut removed_entities = Vec::new();
        let action = GameAction::from(envelope.command);
        let action_cost = action.energy_cost();

        match action {
            GameAction::AbandonPausedTask { task_id } => {
                if let Some(positions) = self.abandon_paused_task(&task_id) {
                    changed.extend(positions);
                    events.push(DomainEvent::TaskAbandoned {
                        floor_id: task_id.clone(),
                    });
                    events.push(DomainEvent::OneShotFloorClosed { floor_id: task_id });
                } else {
                    events.push(DomainEvent::TaskAbandonUnavailable);
                }
            }
            GameAction::Appraise { item_id } => {
                if let Some((target_kind_id, quality)) = self.appraise_inventory_item(&item_id) {
                    events.push(DomainEvent::ItemAppraised {
                        target_kind_id,
                        quality,
                    });
                } else {
                    events.push(DomainEvent::ItemAppraiseUnavailable);
                }
            }
            GameAction::BashDoor { direction } => match self.bash_door(direction) {
                Some(DoorBashOutcome::Succeeded { position }) => {
                    changed.insert(position);
                    events.push(DomainEvent::DoorBashedOpen { position });
                }
                Some(DoorBashOutcome::Failed { position }) => {
                    events.push(DomainEvent::DoorBashFailed { position });
                }
                None => events.push(DomainEvent::DoorBashUnavailable),
            },
            GameAction::CloseDoor { direction } => {
                if let Some(position) = self.close_door(direction) {
                    changed.insert(position);
                    events.push(DomainEvent::DoorClosed { position });
                } else {
                    events.push(DomainEvent::DoorCloseUnavailable);
                }
            }
            GameAction::Drop { item_ids } => {
                if let Some((stacks, quantity)) = self.drop_inventory_items(&item_ids) {
                    changed.insert(self.player.position);
                    events.push(DomainEvent::ItemsDropped { stacks, quantity });
                } else {
                    events.push(DomainEvent::NoItemsDropped);
                }
            }
            GameAction::DropQuantity { item_id, quantity } => {
                if let Some((stacks, dropped_quantity)) =
                    self.drop_inventory_quantity(&item_id, quantity)?
                {
                    changed.insert(self.player.position);
                    events.push(DomainEvent::ItemsDropped {
                        stacks,
                        quantity: dropped_quantity,
                    });
                } else {
                    events.push(DomainEvent::NoItemsDropped);
                }
            }
            GameAction::Equip { item_id } => {
                if let Some(outcome) = self.equip_inventory_item(&item_id) {
                    let discovered_affix_ids = outcome.discovered_affix_ids.clone();
                    let equipped_kind_id = outcome.kind_id.clone();
                    events.push(DomainEvent::ItemEquipped {
                        target_kind_id: outcome.kind_id,
                        slot_id: outcome.slot_id,
                        replaced_kind_id: outcome.replaced_kind_id,
                    });
                    for affix_id in discovered_affix_ids {
                        let property_name_key = self
                            .content
                            .affix(&affix_id)
                            .expect("equipped affix must remain available")
                            .name_key
                            .clone();
                        events.push(DomainEvent::ItemPropertyDiscovered {
                            target_kind_id: equipped_kind_id.clone(),
                            property_name_key,
                        });
                    }
                } else {
                    events.push(DomainEvent::ItemEquipUnavailable);
                }
            }
            GameAction::Fire { direction } => self.resolve_player_projectile(
                TargetSelection::Direction { direction },
                &mut events,
                &mut changed,
                &mut removed_entities,
            )?,
            GameAction::FireTarget { target } => self.resolve_player_projectile(
                target,
                &mut events,
                &mut changed,
                &mut removed_entities,
            )?,
            GameAction::Throw { item_id, direction } => {
                self.throw_inventory_item(
                    &item_id,
                    direction,
                    &mut events,
                    &mut changed,
                    &mut removed_entities,
                )?;
            }
            action @ (GameAction::TraverseStairs | GameAction::AbandonTask) => {
                let abandon_task = matches!(action, GameAction::AbandonTask);
                if let Some(transition) = self.traverse_stairs(abandon_task)? {
                    for y in 0..self.height {
                        for x in 0..self.width {
                            changed.insert(Position {
                                x: i32::from(x),
                                y: i32::from(y),
                            });
                        }
                    }
                    events.push(DomainEvent::FloorTransitioned {
                        from_floor_id: transition.from_floor_id,
                        to_floor_id: transition.to_floor_id,
                    });
                    if transition.expedition_ended {
                        events.push(DomainEvent::DungeonExpeditionEnded);
                    }
                    if let Some(floor_id) = transition.task_resumed {
                        events.push(DomainEvent::TaskResumed { floor_id });
                    }
                    if let Some(floor_id) = transition.task_paused {
                        events.push(DomainEvent::TaskPaused { floor_id });
                    }
                    if let Some((floor_id, resolution)) = transition.one_shot_closed {
                        events.push(match resolution {
                            TaskResolution::Completed => DomainEvent::TaskCompleted {
                                floor_id: floor_id.clone(),
                            },
                            TaskResolution::Failed => DomainEvent::TaskFailed {
                                floor_id: floor_id.clone(),
                            },
                            TaskResolution::Abandoned => DomainEvent::TaskAbandoned {
                                floor_id: floor_id.clone(),
                            },
                        });
                        if resolution == TaskResolution::Completed
                            && let Some(reward) = self
                                .content
                                .world(&self.world_id)
                                .and_then(|world| {
                                    world
                                        .procedural_floors
                                        .iter()
                                        .find(|floor| floor_task_id(floor) == floor_id)
                                })
                                .and_then(|floor| {
                                    self.content.world(&self.world_id).and_then(|world| {
                                        world
                                            .procedural_floors
                                            .iter()
                                            .filter(|member| {
                                                floor_task_id(member) == floor_task_id(floor)
                                            })
                                            .find_map(|member| member.task_reward.as_ref())
                                    })
                                })
                        {
                            events.push(DomainEvent::TaskRewarded {
                                item_kind_id: reward.item_kind_id.clone(),
                                quantity: reward.quantity,
                            });
                        }
                        events.push(DomainEvent::OneShotFloorClosed { floor_id });
                    }
                } else {
                    events.push(DomainEvent::FloorTransitionUnavailable);
                }
            }
            GameAction::UseItem { item_id } => {
                self.use_inventory_item(&item_id, &mut events);
            }
            GameAction::Wait => events.push(DomainEvent::Waited),
            GameAction::PickUp => match self.pick_up_at_player()? {
                PickUpOutcome::Picked { kind_id, quantity } => {
                    changed.insert(self.player.position);
                    events.push(DomainEvent::ItemPickedUp {
                        target_kind_id: kind_id,
                        quantity,
                    });
                }
                PickUpOutcome::OverCapacity {
                    kind_id,
                    quantity,
                    current_weight,
                    pickup_weight,
                    capacity,
                } => events.push(DomainEvent::ItemPickupOverCapacity {
                    target_kind_id: kind_id,
                    quantity,
                    current_weight,
                    pickup_weight,
                    capacity,
                }),
                PickUpOutcome::Nothing => events.push(DomainEvent::NothingToPickUp),
            },
            GameAction::Unequip { slot_id } => {
                if let Some(kind_id) = self.unequip_slot(&slot_id) {
                    events.push(DomainEvent::ItemUnequipped {
                        target_kind_id: kind_id,
                        slot_id,
                    });
                } else {
                    events.push(DomainEvent::ItemUnequipUnavailable { slot_id });
                }
            }
            GameAction::Move { direction } => {
                let (dx, dy) = direction.delta();
                let target = Position {
                    x: self.player.position.x + dx,
                    y: self.player.position.y + dy,
                };
                if !self.is_walkable(target) {
                    events.push(DomainEvent::MoveBlocked);
                } else if let Some(index) = self
                    .entities
                    .iter()
                    .position(|entity| entity.position == target)
                {
                    changed.insert(target);
                    if self.player_fear_blocks_melee(index) {
                        events.push(DomainEvent::PlayerFearBlocked {
                            status_kind_id: STATUS_FEAR.to_owned(),
                        });
                    } else {
                        self.resolve_player_melee(
                            index,
                            &mut events,
                            &mut changed,
                            &mut removed_entities,
                        )?;
                    }
                } else {
                    let old_position = self.player.position;
                    self.player.position = target;
                    changed.insert(old_position);
                    changed.insert(target);
                    if let Some((source_kind_id, damage)) = self.trigger_player_trap(target) {
                        events.push(DomainEvent::TrapTriggered {
                            position: target,
                            damage,
                        });
                        if self.player_is_dead() {
                            events.push(DomainEvent::PlayerDied {
                                source_kind_id,
                                method_id: None,
                                damage,
                            });
                        }
                    }
                }
            }
            GameAction::OpenDoor { direction } => match self.open_door(direction) {
                Some(DoorOpenOutcome::Opened { position }) => {
                    changed.insert(position);
                    events.push(DomainEvent::DoorOpened { position });
                }
                Some(DoorOpenOutcome::Unlocked { position }) => {
                    changed.insert(position);
                    events.push(DomainEvent::DoorUnlocked { position });
                    events.push(DomainEvent::DoorOpened { position });
                }
                Some(DoorOpenOutcome::UnlockFailed { position }) => {
                    events.push(DomainEvent::DoorUnlockFailed { position });
                }
                None => events.push(DomainEvent::DoorOpenUnavailable),
            },
            GameAction::Search => {
                let discovered = self.search_hidden_terrain();
                if discovered.is_empty() {
                    events.push(DomainEvent::SearchFoundNothing);
                } else {
                    for position in discovered {
                        changed.insert(position);
                        events.push(DomainEvent::SecretTerrainDiscovered { position });
                    }
                }
            }
            GameAction::DisarmTrap { direction } => match self.disarm_trap(direction) {
                Some(TrapDisarmOutcome::Succeeded { position }) => {
                    changed.insert(position);
                    events.push(DomainEvent::TrapDisarmed { position });
                }
                Some(TrapDisarmOutcome::Failed { position }) => {
                    events.push(DomainEvent::TrapDisarmFailed { position });
                }
                None => events.push(DomainEvent::TrapDisarmUnavailable),
            },
            GameAction::DigTerrain { direction } => match self.dig_terrain(direction) {
                Some(TerrainDigOutcome::Succeeded { position }) => {
                    changed.insert(position);
                    events.push(DomainEvent::TerrainDug { position });
                }
                Some(TerrainDigOutcome::Failed { position }) => {
                    events.push(DomainEvent::TerrainDigFailed { position });
                }
                None => events.push(DomainEvent::TerrainDigUnavailable),
            },
        }

        spend_energy(&mut self.player.energy_need, action_cost);
        self.advance_until_player_ready(&mut events, &mut changed, &mut removed_entities)?;
        self.apply_task_events(&events);

        self.last_command_seq = envelope.command_seq;
        self.turn = self.turn.saturating_add(1);
        self.revision = self.revision.saturating_add(1);
        self.reveal_current_visibility();
        let changed_visual_cells = self.changed_visual_cells(&previous_visuals);
        let events = project_events(events);

        Ok(GameUpdate {
            base_revision,
            revision: self.revision,
            turn: self.turn,
            world_tick: self.world_tick,
            command_seq: self.last_command_seq,
            floor_id: self.current_floor_id.clone(),
            dungeon_instance_id: self.current_dungeon_instance_id.clone(),
            events,
            changed_cells: changed
                .into_iter()
                .map(|position| self.cell_dto(position))
                .collect(),
            changed_visual_cells,
            player: self.player_dto(),
            entities: self.entities_dto(),
            items: self.items_dto(),
            inventory: self.inventory_dto(),
            equipment: self.equipment_dto(),
            removed_entities,
            terrain_interactions: self.terrain_interactions(),
            tasks: self.task_statuses(),
            state_hash: self.state_hash(),
        })
    }

    #[must_use]
    pub fn state_hash(&self) -> String {
        let payload = StateHashPayloadV25 {
            schema_version: 25,
            revision: self.revision,
            turn: self.turn,
            world_tick: self.world_tick,
            last_command_seq: self.last_command_seq,
            terrain: TerrainSaveDto {
                width: self.width,
                height: self.height,
                terrain_ids: self.terrain.clone(),
            },
            player: player_to_save(&self.player),
            entities: actors_to_save(&self.entities),
            items: items_to_save(&self.items),
            inventory: inventory_to_save(&self.items),
            equipment: equipment_to_save(&self.items),
            carried_items: carried_items_to_save(&self.items),
            item_knowledge: self.item_knowledge_to_save(),
            item_property_knowledge: self.item_property_knowledge_to_save(),
            task_states: self.task_states_to_save(),
            dungeon_states: self.dungeon_states_to_save(),
            next_item_instance_serial: self.next_item_instance_serial,
            explored: Vec::new(),
            revealed_terrain: self.revealed_terrain.iter().copied().collect(),
            floor_connections: floor_connections_to_save(&self.floor_connections),
            floor_regions: floor_regions_to_save(&self.floor_regions),
            rng: self.rng.to_save(),
            content_id: self.content.pack_id().to_owned(),
            content_hash: self.content.content_hash().to_owned(),
            world_id: self.world_id.clone(),
            current_floor_id: self.current_floor_id.clone(),
            current_dungeon_instance_id: self.current_dungeon_instance_id.clone(),
            stored_floors: self
                .stored_floors
                .values()
                .map(|floor| {
                    let mut floor = floor_to_save(floor);
                    floor.explored.clear();
                    floor
                })
                .collect(),
        };
        let bytes = rmp_serde::to_vec_named(&payload)
            .expect("serializing the internal save state should not fail");
        let digest = Sha256::digest(bytes);
        format!("{digest:x}")
    }

    #[must_use]
    pub const fn rng_draw_counter(&self) -> u64 {
        self.rng.draw_counter
    }

    #[must_use]
    pub const fn rng_algorithm(&self) -> &'static str {
        RNG_ALGORITHM
    }

    #[must_use]
    pub fn content_id(&self) -> &str {
        self.content.pack_id()
    }

    #[must_use]
    pub fn content_hash(&self) -> &str {
        self.content.content_hash()
    }

    #[must_use]
    pub fn world_id(&self) -> &str {
        &self.world_id
    }

    #[must_use]
    pub fn location_key(&self) -> &str {
        let world = self
            .content
            .world(&self.world_id)
            .expect("game world must remain in its content catalog");
        world
            .procedural_floors
            .iter()
            .find(|floor| floor.id == self.current_floor_id)
            .map_or(&world.name_key, |floor| &floor.name_key)
    }

    fn floor_depth(&self, floor_id: &str) -> u16 {
        let world = self
            .content
            .world(&self.world_id)
            .expect("game world must remain in its content catalog");
        world
            .procedural_floors
            .iter()
            .find(|floor| floor.id == floor_id)
            .map_or(0, |floor| floor.depth)
    }

    fn player_dto(&self) -> PlayerDto {
        let stats = self.player_derived_stats();
        let melee_profile = self.player_melee_profile(&stats);
        let melee_profile_dto = melee_profile.to_dto();
        let equipment_modifiers = self.equipment_modifiers();
        let definition = self
            .content
            .actor(&self.player.kind_id)
            .expect("player actor definition must remain available");
        PlayerDto {
            id: self.player.id.clone(),
            kind_id: self.player.kind_id.clone(),
            position: self.player.position,
            hp: self.player.hp,
            max_hp: stats.max_hp.value,
            speed: derived_speed(&stats.speed),
            energy_need: self.player.energy_need,
            carried_weight_tenths_pound: self.carried_weight_tenths_pound(),
            carry_capacity_tenths_pound: definition.carry_capacity_tenths_pound,
            base_max_hp: self.player.max_hp,
            attack: stats.attack.value,
            base_attack: definition.attack,
            defense: stats.defense.value,
            base_defense: definition.defense,
            melee_skill: stats.melee_skill.value,
            armor_class: stats.armor_class.value,
            melee_damage: DamageDiceDto {
                dice: melee_profile.damage_dice,
                sides: melee_profile.damage_sides,
                damage_type: melee_profile.damage_type.into(),
            },
            melee_profile: melee_profile_dto,
            projectile_profile: self
                .player_projectile_profile()
                .map(|profile| profile.to_dto()),
            is_dead: self.player_is_dead(),
            equipment_modifiers,
            statuses: self
                .player
                .statuses
                .iter()
                .map(crate::effect::StatusInstance::to_dto)
                .collect(),
            resistances: self.player.resistances.to_dtos(),
        }
    }

    fn entities_dto(&self) -> Vec<EntityDto> {
        let mut entities = self
            .entities
            .iter()
            .map(|entity| {
                let definition = self
                    .content
                    .actor(&entity.kind_id)
                    .expect("entity actor definition must remain available");
                let stats = self.actor_derived_stats(entity, definition, false);
                EntityDto {
                    id: entity.id.clone(),
                    kind_id: entity.kind_id.clone(),
                    position: entity.position,
                    hp: entity.hp,
                    max_hp: entity.max_hp,
                    speed: derived_speed(&stats.speed),
                    energy_need: entity.energy_need,
                    attack: stats.attack.value,
                    defense: stats.defense.value,
                    melee_skill: stats.melee_skill.value,
                    armor_class: stats.armor_class.value,
                    melee_damage: DamageDiceDto {
                        dice: definition.damage_dice,
                        sides: definition.damage_sides,
                        damage_type: DamageType::from(definition.damage_type).into(),
                    },
                    melee_profile: AttackProfileDto {
                        attacks: 1,
                        to_hit: 0,
                        to_damage: 0,
                        damage: DamageDiceDto {
                            dice: definition.damage_dice,
                            sides: definition.damage_sides,
                            damage_type: DamageType::from(definition.damage_type).into(),
                        },
                        source_item_id: None,
                    },
                    melee_routine: actor_melee_routine_dto(definition),
                    statuses: entity
                        .statuses
                        .iter()
                        .map(crate::effect::StatusInstance::to_dto)
                        .collect(),
                }
            })
            .collect::<Vec<_>>();
        entities.sort_by(|left, right| left.id.cmp(&right.id));
        entities
    }

    fn items_dto(&self) -> Vec<ItemDto> {
        let mut items = self
            .items
            .iter()
            .filter_map(|item| {
                let ItemLocation::Ground(position) = &item.location else {
                    return None;
                };
                Some(ItemDto {
                    id: item.id.clone(),
                    kind_id: item.kind_id.clone(),
                    display_name_key: self.item_display_name_key(&item.kind_id),
                    knowledge: self.item_knowledge_dto(&item.kind_id),
                    position: *position,
                    quantity: item.quantity,
                })
            })
            .collect::<Vec<_>>();
        items.sort_by(|left, right| left.id.cmp(&right.id));
        items
    }

    fn inventory_dto(&self) -> Vec<InventoryItemDto> {
        let mut inventory = self
            .items
            .iter()
            .filter_map(|item| {
                if item.location != ItemLocation::Inventory {
                    return None;
                }
                Some(InventoryItemDto {
                    id: item.id.clone(),
                    kind_id: item.kind_id.clone(),
                    display_name_key: self.item_display_name_key(&item.kind_id),
                    knowledge: self.item_knowledge_dto(&item.kind_id),
                    usable: self
                        .content
                        .item(&item.kind_id)
                        .is_some_and(|definition| definition.use_action.is_some()),
                    quantity: item.quantity,
                    weight_tenths_pound: self.item_weight_tenths_pound(&item.kind_id),
                    equipment_slot: self
                        .content
                        .item(&item.kind_id)
                        .and_then(|definition| definition.equipment_slot.clone()),
                    modifiers: self.visible_item_modifiers(item),
                    identification: self.item_identification(item),
                    quality: self.visible_item_quality(item),
                    known_properties: self.known_item_properties(item),
                    melee_profile: self.visible_item_melee_profile(item),
                    projectile_profile: self.visible_item_projectile_profile(item),
                    throw_profile: self.visible_item_throw_profile(item),
                })
            })
            .collect::<Vec<_>>();
        inventory.sort_by(|left, right| left.id.cmp(&right.id));
        inventory
    }

    fn equipment_dto(&self) -> Vec<EquipmentItemDto> {
        let mut equipment = self
            .items
            .iter()
            .filter_map(|item| {
                let ItemLocation::Equipped { slot_id } = &item.location else {
                    return None;
                };
                Some(EquipmentItemDto {
                    id: item.id.clone(),
                    kind_id: item.kind_id.clone(),
                    display_name_key: self.item_display_name_key(&item.kind_id),
                    knowledge: self.item_knowledge_dto(&item.kind_id),
                    quantity: item.quantity,
                    weight_tenths_pound: self.item_weight_tenths_pound(&item.kind_id),
                    slot_id: slot_id.clone(),
                    modifiers: self.visible_item_modifiers(item),
                    identification: self.item_identification(item),
                    quality: self.visible_item_quality(item),
                    known_properties: self.known_item_properties(item),
                    melee_profile: self.visible_item_melee_profile(item),
                    projectile_profile: self.visible_item_projectile_profile(item),
                    throw_profile: self.visible_item_throw_profile(item),
                })
            })
            .collect::<Vec<_>>();
        equipment.sort_by(|left, right| left.slot_id.cmp(&right.slot_id));
        equipment
    }

    fn drop_inventory_items(&mut self, item_ids: &[String]) -> Option<(usize, u64)> {
        let selected = item_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
        if selected.is_empty() {
            return None;
        }
        let mut stacks = 0_usize;
        let mut quantity = 0_u64;
        for item in &mut self.items {
            if item.location == ItemLocation::Inventory && selected.contains(item.id.as_str()) {
                item.location = ItemLocation::Ground(self.player.position);
                stacks += 1;
                quantity = quantity.saturating_add(u64::from(item.quantity));
            }
        }
        if stacks == 0 {
            return None;
        }
        Some((stacks, quantity))
    }

    fn appraise_inventory_item(&mut self, item_id: &str) -> Option<(String, ItemQualityDto)> {
        let item = self
            .items
            .iter()
            .find(|item| item.id == item_id && item.location == ItemLocation::Inventory)?;
        let item_instance_id = item.id.clone();
        let kind_id = item.kind_id.clone();
        let quality = item.quality;
        let knowledge = self
            .item_property_knowledge
            .entry(item_instance_id)
            .or_default();
        if knowledge.appraised || knowledge.identified {
            return None;
        }
        knowledge.appraised = true;
        Some((kind_id, quality))
    }

    fn drop_inventory_quantity(
        &mut self,
        item_id: &str,
        quantity: u32,
    ) -> Result<Option<(usize, u64)>, CoreError> {
        let Some(index) = self
            .items
            .iter()
            .position(|item| item.id == item_id && item.location == ItemLocation::Inventory)
        else {
            return Ok(None);
        };
        if quantity == 0 || quantity > self.items[index].quantity {
            return Ok(None);
        }
        if quantity == self.items[index].quantity {
            self.items[index].location = ItemLocation::Ground(self.player.position);
        } else {
            let id = self.allocate_item_instance_id()?;
            let kind_id = self.items[index].kind_id.clone();
            self.items[index].quantity -= quantity;
            self.items.push(ItemInstance {
                id,
                kind_id,
                quantity,
                quality: ItemQualityDto::Ordinary,
                affix_ids: Vec::new(),
                location: ItemLocation::Ground(self.player.position),
            });
        }
        Ok(Some((1, u64::from(quantity))))
    }

    fn equip_inventory_item(&mut self, item_id: &str) -> Option<EquipOutcome> {
        let inventory_index = self
            .items
            .iter()
            .position(|item| item.id == item_id && item.location == ItemLocation::Inventory)?;
        let carried = &self.items[inventory_index];
        let slot_id = self
            .content
            .item(&carried.kind_id)?
            .equipment_slot
            .clone()?;
        if carried.quantity != 1 {
            return None;
        }
        let replaced_kind_id = self
            .items
            .iter()
            .position(|equipped| {
                matches!(
                    &equipped.location,
                    ItemLocation::Equipped { slot_id: equipped_slot } if equipped_slot == &slot_id
                )
            })
            .map(|index| {
                let kind_id = self.items[index].kind_id.clone();
                self.items[index].location = ItemLocation::Inventory;
                kind_id
            });
        let kind_id = self.items[inventory_index].kind_id.clone();
        let item_instance_id = self.items[inventory_index].id.clone();
        let affix_ids = self.items[inventory_index].affix_ids.clone();
        self.items[inventory_index].location = ItemLocation::Equipped {
            slot_id: slot_id.clone(),
        };
        self.clamp_player_hp_to_effective_max();
        let knowledge = self
            .item_property_knowledge
            .entry(item_instance_id)
            .or_default();
        knowledge.appraised = true;
        knowledge.identified = true;
        let discovered_affix_ids = affix_ids
            .into_iter()
            .filter(|affix_id| knowledge.known_affix_ids.insert(affix_id.clone()))
            .collect();
        Some(EquipOutcome {
            kind_id,
            slot_id,
            replaced_kind_id,
            discovered_affix_ids,
        })
    }

    fn unequip_slot(&mut self, slot_id: &str) -> Option<String> {
        let index = self.items.iter().position(|item| {
            matches!(
                &item.location,
                ItemLocation::Equipped { slot_id: equipped_slot } if equipped_slot == slot_id
            )
        })?;
        let kind_id = self.items[index].kind_id.clone();
        self.items[index].location = ItemLocation::Inventory;
        self.clamp_player_hp_to_effective_max();
        Some(kind_id)
    }

    fn pick_up_at_player(&mut self) -> Result<PickUpOutcome, CoreError> {
        let Some(index) = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.location == ItemLocation::Ground(self.player.position))
            .min_by(|(_, left), (_, right)| left.id.cmp(&right.id))
            .map(|(index, _)| index)
        else {
            return Ok(PickUpOutcome::Nothing);
        };

        let kind_id = self.items[index].kind_id.clone();
        let definition = self
            .content
            .item(&kind_id)
            .ok_or_else(|| CoreError::UnknownItem(kind_id.clone()))?;
        let max_stack = definition.max_stack;
        let original_quantity = self.items[index].quantity;
        let current_weight = self.carried_weight_tenths_pound();
        let pickup_weight =
            u32::from(definition.weight_tenths_pound).saturating_mul(original_quantity);
        let capacity = self
            .content
            .actor(&self.player.kind_id)
            .expect("player actor definition must remain available")
            .carry_capacity_tenths_pound;
        if current_weight.saturating_add(pickup_weight) > capacity {
            return Ok(PickUpOutcome::OverCapacity {
                kind_id,
                quantity: original_quantity,
                current_weight,
                pickup_weight,
                capacity,
            });
        }
        let mut remaining = original_quantity;
        let mut stack_indices = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, carried)| {
                carried.location == ItemLocation::Inventory
                    && carried.kind_id == kind_id
                    && carried.quantity < max_stack
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        stack_indices.sort_by(|left, right| self.items[*left].id.cmp(&self.items[*right].id));
        for stack_index in stack_indices {
            let stack = &mut self.items[stack_index];
            let transferred = remaining.min(max_stack - stack.quantity);
            stack.quantity += transferred;
            remaining -= transferred;
            if remaining == 0 {
                break;
            }
        }
        if remaining == 0 {
            let removed = self.items.remove(index);
            self.item_property_knowledge.remove(&removed.id);
        } else {
            self.items[index].quantity = remaining;
            self.items[index].location = ItemLocation::Inventory;
        }
        Ok(PickUpOutcome::Picked {
            kind_id,
            quantity: original_quantity,
        })
    }

    fn item_base_modifiers(&self, kind_id: &str) -> StatModifiersDto {
        self.content
            .item(kind_id)
            .map_or_else(StatModifiersDto::default, |definition| StatModifiersDto {
                attack: definition.modifiers.attack,
                defense: definition.modifiers.defense,
                max_hp: definition.modifiers.max_hp,
            })
    }

    fn item_modifiers(&self, item: &ItemInstance) -> StatModifiersDto {
        item.affix_ids.iter().fold(
            self.item_base_modifiers(&item.kind_id),
            |total, affix_id| {
                let affix = self
                    .content
                    .affix(affix_id)
                    .expect("item affix must remain available");
                StatModifiersDto {
                    attack: total.attack.saturating_add(affix.modifiers.attack),
                    defense: total.defense.saturating_add(affix.modifiers.defense),
                    max_hp: total.max_hp.saturating_add(affix.modifiers.max_hp),
                }
            },
        )
    }

    fn item_knowledge_dto(&self, kind_id: &str) -> ItemKnowledgeDto {
        let Some(definition) = self.content.item(kind_id) else {
            return ItemKnowledgeDto::Unknown;
        };
        if definition.appearance_name_key.is_none() {
            return ItemKnowledgeDto::Aware;
        }
        self.item_knowledge
            .get(kind_id)
            .map_or(ItemKnowledgeDto::Unknown, |knowledge| {
                if knowledge.aware {
                    ItemKnowledgeDto::Aware
                } else if knowledge.tried {
                    ItemKnowledgeDto::Tried
                } else {
                    ItemKnowledgeDto::Unknown
                }
            })
    }

    fn item_display_name_key(&self, kind_id: &str) -> String {
        let Some(definition) = self.content.item(kind_id) else {
            return "item-unknown-name".to_owned();
        };
        if self.item_knowledge_dto(kind_id) == ItemKnowledgeDto::Aware {
            definition.name_key.clone()
        } else {
            definition
                .appearance_name_key
                .clone()
                .unwrap_or_else(|| definition.name_key.clone())
        }
    }

    fn mark_item_tried(&mut self, kind_id: &str) {
        if self
            .content
            .item(kind_id)
            .is_some_and(|definition| definition.appearance_name_key.is_some())
        {
            self.item_knowledge
                .entry(kind_id.to_owned())
                .or_default()
                .tried = true;
        }
    }

    fn mark_item_aware(&mut self, kind_id: &str) {
        if self
            .content
            .item(kind_id)
            .is_some_and(|definition| definition.appearance_name_key.is_some())
        {
            let knowledge = self.item_knowledge.entry(kind_id.to_owned()).or_default();
            knowledge.tried = true;
            knowledge.aware = true;
        }
    }

    fn visible_item_modifiers(&self, item: &ItemInstance) -> StatModifiersDto {
        if self.item_knowledge_dto(&item.kind_id) != ItemKnowledgeDto::Aware {
            return StatModifiersDto::default();
        }
        let known = self.item_property_knowledge.get(&item.id);
        item.affix_ids.iter().fold(
            self.item_base_modifiers(&item.kind_id),
            |total, affix_id| {
                let Some(affix) = known
                    .filter(|knowledge| knowledge.known_affix_ids.contains(affix_id))
                    .and_then(|_| self.content.affix(affix_id))
                else {
                    return total;
                };
                StatModifiersDto {
                    attack: total.attack.saturating_add(affix.modifiers.attack),
                    defense: total.defense.saturating_add(affix.modifiers.defense),
                    max_hp: total.max_hp.saturating_add(affix.modifiers.max_hp),
                }
            },
        )
    }

    fn known_item_properties(&self, item: &ItemInstance) -> Vec<ItemPropertyDto> {
        self.item_property_knowledge
            .get(&item.id)
            .into_iter()
            .flat_map(|knowledge| &knowledge.known_affix_ids)
            .filter_map(|affix_id| {
                self.content.affix(affix_id).map(|affix| ItemPropertyDto {
                    affix_id: affix.id.clone(),
                    name_key: affix.name_key.clone(),
                    modifiers: StatModifiersDto {
                        attack: affix.modifiers.attack,
                        defense: affix.modifiers.defense,
                        max_hp: affix.modifiers.max_hp,
                    },
                })
            })
            .collect()
    }

    fn item_identification(&self, item: &ItemInstance) -> ItemIdentificationDto {
        self.item_property_knowledge.get(&item.id).map_or(
            ItemIdentificationDto::Unexamined,
            |knowledge| {
                if knowledge.identified {
                    ItemIdentificationDto::Identified
                } else if knowledge.appraised {
                    ItemIdentificationDto::Appraised
                } else {
                    ItemIdentificationDto::Unexamined
                }
            },
        )
    }

    fn visible_item_quality(&self, item: &ItemInstance) -> Option<ItemQualityDto> {
        (self.item_identification(item) != ItemIdentificationDto::Unexamined)
            .then_some(item.quality)
    }

    fn visible_item_melee_profile(&self, item: &ItemInstance) -> Option<AttackProfileDto> {
        (self.item_knowledge_dto(&item.kind_id) == ItemKnowledgeDto::Aware)
            .then(|| self.item_melee_profile(item))
            .flatten()
    }

    fn visible_item_projectile_profile(&self, item: &ItemInstance) -> Option<ProjectileProfileDto> {
        (self.item_knowledge_dto(&item.kind_id) == ItemKnowledgeDto::Aware)
            .then(|| self.item_projectile_profile(item))
            .flatten()
    }

    fn visible_item_throw_profile(&self, item: &ItemInstance) -> Option<ThrowProfileDto> {
        (self.item_knowledge_dto(&item.kind_id) == ItemKnowledgeDto::Aware)
            .then(|| self.item_throw_profile(item))
            .flatten()
    }

    fn item_knowledge_to_save(&self) -> Vec<ItemKnowledgeSaveDto> {
        self.item_knowledge
            .iter()
            .map(|(kind_id, knowledge)| ItemKnowledgeSaveDto {
                kind_id: kind_id.clone(),
                tried: knowledge.tried,
                aware: knowledge.aware,
            })
            .collect()
    }

    fn item_property_knowledge_to_save(&self) -> Vec<ItemPropertyKnowledgeSaveDto> {
        self.item_property_knowledge
            .iter()
            .filter(|(_, knowledge)| {
                knowledge.appraised || knowledge.identified || !knowledge.known_affix_ids.is_empty()
            })
            .map(|(item_id, knowledge)| ItemPropertyKnowledgeSaveDto {
                item_id: item_id.clone(),
                appraised: knowledge.appraised,
                identified: knowledge.identified,
                known_affix_ids: knowledge.known_affix_ids.iter().cloned().collect(),
            })
            .collect()
    }

    fn task_states_to_save(&self) -> Vec<TaskStateSaveDto> {
        self.task_states
            .iter()
            .map(|(task_id, state)| TaskStateSaveDto {
                task_id: task_id.clone(),
                status: state.status,
                stage_index: state.stage_index,
                current: state.current,
                required: state.required,
                active_floor_id: state.active_floor_id.clone(),
                retakes_used: state.retakes_used,
            })
            .collect()
    }

    fn dungeon_states_to_save(&self) -> Vec<DungeonStateSaveDto> {
        self.dungeon_states
            .iter()
            .map(|(dungeon_id, state)| DungeonStateSaveDto {
                dungeon_id: dungeon_id.clone(),
                guardian_defeated: state.guardian_defeated,
                next_instance_ordinal: state.next_instance_ordinal,
            })
            .collect()
    }

    fn equipment_modifiers(&self) -> StatModifiersDto {
        self.items
            .iter()
            .filter(|item| matches!(&item.location, ItemLocation::Equipped { .. }))
            .fold(StatModifiersDto::default(), |total, item| {
                let item = self.item_modifiers(item);
                StatModifiersDto {
                    attack: total.attack.saturating_add(item.attack),
                    defense: total.defense.saturating_add(item.defense),
                    max_hp: total.max_hp.saturating_add(item.max_hp),
                }
            })
    }

    fn effective_player_max_hp(&self) -> i32 {
        self.player_derived_stats().max_hp.value
    }

    fn player_derived_stats(&self) -> ActorDerivedStats {
        let definition = self
            .content
            .actor(&self.player.kind_id)
            .expect("player actor definition must remain available");
        self.actor_derived_stats(&self.player, definition, true)
    }

    fn item_melee_profile(&self, item: &ItemInstance) -> Option<AttackProfileDto> {
        self.content
            .item(&item.kind_id)
            .and_then(|definition| definition.melee_profile.as_ref())
            .map(|profile| AttackProfileDto {
                attacks: profile.attacks,
                to_hit: profile.to_hit,
                to_damage: profile.to_damage,
                damage: DamageDiceDto {
                    dice: profile.damage_dice,
                    sides: profile.damage_sides,
                    damage_type: DamageType::from(profile.damage_type).into(),
                },
                source_item_id: Some(item.id.clone()),
            })
    }

    fn item_projectile_profile(&self, item: &ItemInstance) -> Option<ProjectileProfileDto> {
        self.content
            .item(&item.kind_id)
            .and_then(|definition| definition.projectile_profile.as_ref())
            .map(|profile| ProjectileProfileDto {
                range: profile.range,
                to_hit: profile.to_hit,
                to_damage: profile.to_damage,
                damage: DamageDiceDto {
                    dice: profile.damage_dice,
                    sides: profile.damage_sides,
                    damage_type: DamageType::from(profile.damage_type).into(),
                },
                ammo_kind_id: profile.ammo_kind_id.clone(),
                target_spec: projectile_target_spec(profile.range),
                source_item_id: item.id.clone(),
            })
    }

    fn item_weight_tenths_pound(&self, kind_id: &str) -> u16 {
        self.content
            .item(kind_id)
            .map_or(0, |definition| definition.weight_tenths_pound)
    }

    fn carried_weight_tenths_pound(&self) -> u32 {
        self.items
            .iter()
            .filter(|item| {
                matches!(
                    item.location,
                    ItemLocation::Inventory | ItemLocation::Equipped { .. }
                )
            })
            .fold(0_u32, |total, item| {
                total.saturating_add(
                    u32::from(self.item_weight_tenths_pound(&item.kind_id))
                        .saturating_mul(item.quantity),
                )
            })
    }

    fn item_throw_profile(&self, item: &ItemInstance) -> Option<ThrowProfileDto> {
        let definition = self.content.item(&item.kind_id)?;
        definition
            .throw_profile
            .as_ref()
            .map(|profile| ThrowProfileDto {
                range: throw_range(definition.weight_tenths_pound),
                to_hit: profile.to_hit,
                to_damage: profile.to_damage,
                damage: DamageDiceDto {
                    dice: profile.damage_dice,
                    sides: profile.damage_sides,
                    damage_type: DamageType::from(profile.damage_type).into(),
                },
                source_item_id: item.id.clone(),
            })
    }

    fn player_projectile_profile(&self) -> Option<ResolvedProjectileProfile> {
        self.items.iter().find_map(|item| {
            let ItemLocation::Equipped { slot_id } = &item.location else {
                return None;
            };
            if slot_id != "launcher" {
                return None;
            }
            self.content
                .item(&item.kind_id)?
                .projectile_profile
                .as_ref()
                .and_then(|profile| {
                    let ammo_break_chance_percent = self
                        .content
                        .item(&profile.ammo_kind_id)?
                        .break_chance_percent;
                    Some(ResolvedProjectileProfile {
                        range: profile.range,
                        to_hit: profile.to_hit,
                        to_damage: profile.to_damage,
                        damage_dice: profile.damage_dice,
                        damage_sides: profile.damage_sides,
                        damage_type: DamageType::from(profile.damage_type),
                        ammo_kind_id: profile.ammo_kind_id.clone(),
                        ammo_break_chance_percent,
                        source_item_id: item.id.clone(),
                    })
                })
        })
    }

    fn player_melee_profile(&self, stats: &ActorDerivedStats) -> ResolvedAttackProfile {
        let definition = self
            .content
            .actor(&self.player.kind_id)
            .expect("player actor definition must remain available");
        let equipped_weapon = self.items.iter().find_map(|item| {
            let ItemLocation::Equipped { slot_id } = &item.location else {
                return None;
            };
            if slot_id != "weapon" {
                return None;
            }
            self.content
                .item(&item.kind_id)
                .and_then(|definition| definition.melee_profile.as_ref())
                .map(|profile| (item.id.clone(), profile))
        });
        let (source_item_id, dice, sides, damage_type, to_hit) = equipped_weapon.map_or_else(
            || {
                (
                    None,
                    definition.damage_dice,
                    definition.damage_sides,
                    definition.damage_type,
                    0,
                )
            },
            |(item_id, profile)| {
                (
                    Some(item_id),
                    profile.damage_dice,
                    profile.damage_sides,
                    profile.damage_type,
                    profile.to_hit,
                )
            },
        );
        ResolvedAttackProfile {
            attacks: u16::try_from(stats.melee_attacks.value)
                .expect("derived melee attack count must fit u16"),
            to_hit,
            to_damage: stats.melee_damage_bonus.value,
            damage_dice: dice,
            damage_sides: sides,
            damage_type: DamageType::from(damage_type),
            source_item_id,
        }
    }

    fn actor_derived_stats(
        &self,
        actor: &Actor,
        definition: &rfb_content::ActorDefinition,
        include_equipment: bool,
    ) -> ActorDerivedStats {
        let mut pipeline = DerivedStatsPipeline::new();
        let base_source = definition.id.as_str();
        pipeline.add(StatKind::MaxHp, StatLayer::Base, base_source, actor.max_hp);
        pipeline.add(
            StatKind::Attack,
            StatLayer::Base,
            base_source,
            definition.attack,
        );
        pipeline.add(
            StatKind::Defense,
            StatLayer::Base,
            base_source,
            definition.defense,
        );
        pipeline.add(
            StatKind::Speed,
            StatLayer::Base,
            base_source,
            i32::from(actor.speed),
        );
        pipeline.add(
            StatKind::MeleeSkill,
            StatLayer::Base,
            base_source,
            if definition.role == ActorRole::Monster {
                monster_melee_skill(definition.attack, definition.level)
            } else {
                rating_to_combat_value(definition.attack)
            },
        );
        pipeline.add(
            StatKind::ArmorClass,
            StatLayer::Base,
            base_source,
            rating_to_armor_class(definition.defense),
        );
        pipeline.add(StatKind::MeleeAttacks, StatLayer::Base, base_source, 1);
        pipeline.add(StatKind::MeleeDamageBonus, StatLayer::Base, base_source, 0);
        pipeline.add(
            StatKind::RangedSkill,
            StatLayer::Base,
            base_source,
            rating_to_combat_value(definition.attack),
        );
        pipeline.add(
            StatKind::ThrowingSkill,
            StatLayer::Base,
            base_source,
            rating_to_combat_value(definition.attack),
        );
        pipeline.add(
            StatKind::DoorSkill,
            StatLayer::Base,
            base_source,
            definition.door_skill,
        );
        pipeline.add(
            StatKind::BashPower,
            StatLayer::Base,
            base_source,
            definition.bash_power,
        );
        pipeline.add(
            StatKind::SearchSkill,
            StatLayer::Base,
            base_source,
            definition.search_skill,
        );
        pipeline.add(
            StatKind::DisarmSkill,
            StatLayer::Base,
            base_source,
            definition.disarm_skill,
        );
        pipeline.add(
            StatKind::DigSkill,
            StatLayer::Base,
            base_source,
            definition.dig_skill,
        );

        if include_equipment {
            for item in self
                .items
                .iter()
                .filter(|item| matches!(&item.location, ItemLocation::Equipped { .. }))
            {
                let modifiers = self.item_modifiers(item);
                add_equipment_stat(&mut pipeline, StatKind::MaxHp, &item.id, modifiers.max_hp);
                add_equipment_stat(&mut pipeline, StatKind::Attack, &item.id, modifiers.attack);
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::Defense,
                    &item.id,
                    modifiers.defense,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::MeleeSkill,
                    &item.id,
                    rating_to_combat_value(modifiers.attack),
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::ArmorClass,
                    &item.id,
                    rating_to_armor_class(modifiers.defense),
                );
                if let Some(profile) = self
                    .content
                    .item(&item.kind_id)
                    .and_then(|definition| definition.melee_profile.as_ref())
                {
                    add_equipment_stat(
                        &mut pipeline,
                        StatKind::MeleeAttacks,
                        &item.id,
                        i32::from(profile.attacks).saturating_sub(1),
                    );
                    add_equipment_stat(
                        &mut pipeline,
                        StatKind::MeleeSkill,
                        &item.id,
                        profile.to_hit,
                    );
                    add_equipment_stat(
                        &mut pipeline,
                        StatKind::MeleeDamageBonus,
                        &item.id,
                        profile.to_damage,
                    );
                }
                if let Some(profile) = self
                    .content
                    .item(&item.kind_id)
                    .and_then(|definition| definition.projectile_profile.as_ref())
                {
                    add_equipment_stat(
                        &mut pipeline,
                        StatKind::RangedSkill,
                        &item.id,
                        profile.to_hit,
                    );
                }
            }
        }

        for status in &actor.statuses {
            let amount = i32::from(status.intensity).saturating_mul(10);
            if status.kind_id == STATUS_HASTE {
                pipeline.add_with_origin(
                    StatKind::Speed,
                    StatLayer::Status,
                    &status.kind_id,
                    status.source_id.clone(),
                    amount,
                );
            } else if status.kind_id == STATUS_SLOW {
                pipeline.add_with_origin(
                    StatKind::Speed,
                    StatLayer::Status,
                    &status.kind_id,
                    status.source_id.clone(),
                    amount.saturating_neg(),
                );
            }
            if status.kind_id == STATUS_STUN {
                pipeline.add_with_origin(
                    StatKind::MeleeSkill,
                    StatLayer::Status,
                    &status.kind_id,
                    status.source_id.clone(),
                    i32::from(status.intensity)
                        .saturating_mul(10)
                        .saturating_neg(),
                );
                pipeline.add_with_origin(
                    StatKind::ThrowingSkill,
                    StatLayer::Status,
                    &status.kind_id,
                    status.source_id.clone(),
                    i32::from(status.intensity)
                        .saturating_mul(10)
                        .saturating_neg(),
                );
            }
        }

        ActorDerivedStats {
            max_hp: pipeline.resolve(StatKind::MaxHp, StatBounds::UNBOUNDED),
            attack: pipeline.resolve(StatKind::Attack, StatBounds::NON_NEGATIVE),
            defense: pipeline.resolve(StatKind::Defense, StatBounds::NON_NEGATIVE),
            speed: pipeline.resolve(StatKind::Speed, StatBounds::ACTOR_SPEED),
            melee_skill: pipeline.resolve(StatKind::MeleeSkill, StatBounds::NON_NEGATIVE),
            armor_class: pipeline.resolve(StatKind::ArmorClass, StatBounds::NON_NEGATIVE),
            melee_attacks: pipeline.resolve(StatKind::MeleeAttacks, StatBounds::NON_NEGATIVE),
            melee_damage_bonus: pipeline.resolve(StatKind::MeleeDamageBonus, StatBounds::UNBOUNDED),
            ranged_skill: pipeline.resolve(StatKind::RangedSkill, StatBounds::NON_NEGATIVE),
            throwing_skill: pipeline.resolve(StatKind::ThrowingSkill, StatBounds::NON_NEGATIVE),
            door_skill: pipeline.resolve(StatKind::DoorSkill, StatBounds::NON_NEGATIVE),
            bash_power: pipeline.resolve(StatKind::BashPower, StatBounds::NON_NEGATIVE),
            search_skill: pipeline.resolve(StatKind::SearchSkill, StatBounds::NON_NEGATIVE),
            disarm_skill: pipeline.resolve(StatKind::DisarmSkill, StatBounds::NON_NEGATIVE),
            dig_skill: pipeline.resolve(StatKind::DigSkill, StatBounds::NON_NEGATIVE),
        }
    }

    fn resolve_player_projectile(
        &mut self,
        target: TargetSelection,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let Some(profile) = self.player_projectile_profile() else {
            events.push(DomainEvent::ProjectileUnavailable);
            return Ok(());
        };
        let Some(path) = self.projectile_path(&target, profile.range) else {
            events.push(DomainEvent::ProjectileTargetUnavailable);
            return Ok(());
        };
        let Some(ammunition) = self.take_inventory_item_kind(&profile.ammo_kind_id)? else {
            events.push(DomainEvent::ProjectileAmmoUnavailable {
                ammo_kind_id: profile.ammo_kind_id,
            });
            return Ok(());
        };
        let (trace, target_index) = self.trace_projectile_path(path);
        if let Some(index) = target_index {
            let definition = self
                .content
                .actor(&self.entities[index].kind_id)
                .expect("projectile target definition must remain available")
                .clone();
            let target_kind_id = definition.id.clone();
            let attacker = self.player_derived_stats();
            let target = self.actor_derived_stats(&self.entities[index], &definition, false);
            changed.insert(self.entities[index].position);
            if !resolve_check(
                &mut self.rng,
                CheckContext {
                    kind: CheckKind::ProjectileHit,
                    actor_id: self.player.id.clone(),
                    target_id: Some(self.entities[index].id.clone()),
                    ability: attacker.ranged_skill,
                    difficulty: target.armor_class.clone(),
                },
            )
            .succeeded()
            {
                events.push(DomainEvent::ProjectileMissed {
                    target_kind_id,
                    trace: trace.clone(),
                });
            } else {
                let raw_damage = self
                    .roll_damage(profile.damage_dice, profile.damage_sides)
                    .saturating_add(profile.to_damage)
                    .max(0);
                let prepared = if profile.damage_type == DamageType::Physical {
                    apply_melee_armor_reduction(raw_damage, target.armor_class.value)
                } else {
                    raw_damage
                };
                let resistance = self.entities[index].resistances.level(profile.damage_type);
                let damage = resolve_damage(
                    DamagePacket::after_armor(raw_damage, prepared, profile.damage_type),
                    resistance,
                );
                self.entities[index].hp = self.entities[index].hp.saturating_sub(damage.applied);
                events.push(DomainEvent::ProjectileHit {
                    target_kind_id: target_kind_id.clone(),
                    damage,
                    trace: trace.clone(),
                });
                if self.entities[index].hp <= 0 {
                    self.resolve_actor_death(
                        index,
                        DomainEvent::ProjectileSlew {
                            target_kind_id,
                            damage,
                            trace: trace.clone(),
                        },
                        events,
                        changed,
                        removed_entities,
                    )?;
                }
            }
        } else {
            events.push(DomainEvent::ProjectileLanded {
                trace: trace.clone(),
            });
        }
        self.settle_projectile_ammunition(
            ammunition,
            trace.landing,
            target_index.is_some(),
            profile.ammo_break_chance_percent,
            events,
            changed,
        );
        Ok(())
    }

    fn projectile_path(&self, target: &TargetSelection, range: u16) -> Option<Vec<Position>> {
        let origin = self.player.position;
        match target {
            TargetSelection::Direction { direction } => {
                let (dx, dy) = direction.delta();
                Some(
                    (1..=range)
                        .map(|step| Position {
                            x: origin.x + dx * i32::from(step),
                            y: origin.y + dy * i32::from(step),
                        })
                        .collect(),
                )
            }
            TargetSelection::Position { position } => {
                self.targeted_projectile_path(*position, range)
            }
            TargetSelection::Entity { entity_id } => {
                let position = self
                    .entities
                    .iter()
                    .find(|entity| entity.id == *entity_id)
                    .map(|entity| entity.position)?;
                self.targeted_projectile_path(position, range)
            }
        }
    }

    fn targeted_projectile_path(&self, target: Position, range: u16) -> Option<Vec<Position>> {
        let origin = self.player.position;
        if target == origin
            || self.index(target).is_none()
            || !self.is_visible(target)
            || origin.x.abs_diff(target.x).max(origin.y.abs_diff(target.y)) > u32::from(range)
        {
            return None;
        }

        let mut x = origin.x;
        let mut y = origin.y;
        let dx = (target.x - x).abs();
        let sx = if x < target.x { 1 } else { -1 };
        let dy = -(target.y - y).abs();
        let sy = if y < target.y { 1 } else { -1 };
        let mut error = dx + dy;
        let mut path = Vec::new();
        while x != target.x || y != target.y {
            let doubled = error.saturating_mul(2);
            if doubled >= dy {
                error += dy;
                x += sx;
            }
            if doubled <= dx {
                error += dx;
                y += sy;
            }
            path.push(Position { x, y });
        }
        Some(path)
    }

    fn trace_projectile_path(&self, path: Vec<Position>) -> (ProjectileTrace, Option<usize>) {
        let origin = self.player.position;
        let mut impact = origin;
        let mut landing = origin;
        let mut traversed = Vec::new();
        let mut target_index = None;
        for position in path {
            impact = position;
            if self.index(position).is_none() || !self.is_walkable(position) {
                break;
            }
            landing = position;
            traversed.push(position);
            if let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.position == position)
            {
                target_index = Some(index);
                break;
            }
        }
        (
            ProjectileTrace {
                origin,
                impact,
                landing,
                traversed,
            },
            target_index,
        )
    }

    fn take_inventory_item_kind(
        &mut self,
        kind_id: &str,
    ) -> Result<Option<ItemInstance>, CoreError> {
        let Some(index) = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                item.kind_id == kind_id
                    && item.location == ItemLocation::Inventory
                    && item.quantity > 0
            })
            .min_by(|(_, left), (_, right)| left.id.cmp(&right.id))
            .map(|(index, _)| index)
        else {
            return Ok(None);
        };
        if self.items[index].quantity == 1 {
            Ok(Some(self.items.remove(index)))
        } else {
            let id = self.allocate_item_instance_id()?;
            self.items[index].quantity -= 1;
            Ok(Some(ItemInstance {
                id,
                kind_id: kind_id.to_owned(),
                quantity: 1,
                quality: ItemQualityDto::Ordinary,
                affix_ids: Vec::new(),
                location: ItemLocation::Inventory,
            }))
        }
    }

    fn settle_projectile_ammunition(
        &mut self,
        mut ammunition: ItemInstance,
        landing: Position,
        hit_body: bool,
        break_chance_percent: u8,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let broken = hit_body && self.rng.bounded(100) < u64::from(break_chance_percent);
        if broken {
            self.item_property_knowledge.remove(&ammunition.id);
            events.push(DomainEvent::ProjectileAmmoBroken {
                ammo_kind_id: ammunition.kind_id,
            });
            return;
        }
        ammunition.location = ItemLocation::Ground(landing);
        let ammo_kind_id = ammunition.kind_id.clone();
        self.items.push(ammunition);
        changed.insert(landing);
        events.push(DomainEvent::ProjectileAmmoRecovered { ammo_kind_id });
    }

    fn throw_inventory_item(
        &mut self,
        item_id: &str,
        direction: rfb_protocol::Direction,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let Some(item) = self.items.iter().find(|item| {
            item.id == item_id && item.location == ItemLocation::Inventory && item.quantity > 0
        }) else {
            events.push(DomainEvent::ItemThrowUnavailable);
            return Ok(());
        };
        let definition = self
            .content
            .item(&item.kind_id)
            .expect("throwable item definition must remain available");
        let range = throw_range(definition.weight_tenths_pound);
        let profile = definition
            .throw_profile
            .as_ref()
            .map(|profile| ResolvedThrowProfile {
                to_hit: profile.to_hit,
                to_damage: profile.to_damage,
                damage_dice: profile.damage_dice,
                damage_sides: profile.damage_sides,
                damage_type: DamageType::from(profile.damage_type),
            });
        let Some(mut thrown) = self.take_inventory_item(item_id)? else {
            events.push(DomainEvent::ItemThrowUnavailable);
            return Ok(());
        };
        let source_kind_id = thrown.kind_id.clone();
        self.mark_item_tried(&source_kind_id);
        let path = self
            .projectile_path(&TargetSelection::Direction { direction }, range)
            .expect("direction targeting must always produce a path");
        let (trace, target_index) = self.trace_projectile_path(path);
        let landing = trace.landing;
        if let (Some(profile), Some(index)) = (profile, target_index) {
            let target_definition = self
                .content
                .actor(&self.entities[index].kind_id)
                .expect("throw target definition must remain available")
                .clone();
            let target_kind_id = target_definition.id.clone();
            let attacker = self.player_derived_stats();
            let target = self.actor_derived_stats(&self.entities[index], &target_definition, false);
            let ability = attacker.throwing_skill.with_modifier(
                StatLayer::Equipment,
                &thrown.id,
                profile.to_hit,
                StatBounds::NON_NEGATIVE,
            );
            changed.insert(self.entities[index].position);
            if !resolve_check(
                &mut self.rng,
                CheckContext {
                    kind: CheckKind::ThrowHit,
                    actor_id: self.player.id.clone(),
                    target_id: Some(self.entities[index].id.clone()),
                    ability,
                    difficulty: target.armor_class.clone(),
                },
            )
            .succeeded()
            {
                events.push(DomainEvent::ItemThrowMissed {
                    source_kind_id: source_kind_id.clone(),
                    target_kind_id,
                    trace: trace.clone(),
                });
            } else {
                let raw_damage = self
                    .roll_damage(profile.damage_dice, profile.damage_sides)
                    .saturating_add(profile.to_damage)
                    .max(0);
                let prepared = if profile.damage_type == DamageType::Physical {
                    apply_melee_armor_reduction(raw_damage, target.armor_class.value)
                } else {
                    raw_damage
                };
                let resistance = self.entities[index].resistances.level(profile.damage_type);
                let damage = resolve_damage(
                    DamagePacket::after_armor(raw_damage, prepared, profile.damage_type),
                    resistance,
                );
                self.entities[index].hp = self.entities[index].hp.saturating_sub(damage.applied);
                events.push(DomainEvent::ItemThrowHit {
                    source_kind_id: source_kind_id.clone(),
                    target_kind_id: target_kind_id.clone(),
                    damage,
                    trace: trace.clone(),
                });
                if self.entities[index].hp <= 0 {
                    self.resolve_actor_death(
                        index,
                        DomainEvent::ItemThrowSlew {
                            source_kind_id: source_kind_id.clone(),
                            target_kind_id,
                            damage,
                            trace: trace.clone(),
                        },
                        events,
                        changed,
                        removed_entities,
                    )?;
                }
            }
        } else {
            events.push(DomainEvent::ItemThrown {
                target_kind_id: source_kind_id,
                trace,
            });
        }
        thrown.location = ItemLocation::Ground(landing);
        self.items.push(thrown);
        changed.insert(landing);
        Ok(())
    }

    fn take_inventory_item(&mut self, item_id: &str) -> Result<Option<ItemInstance>, CoreError> {
        let Some(index) = self.items.iter().position(|item| {
            item.id == item_id && item.location == ItemLocation::Inventory && item.quantity > 0
        }) else {
            return Ok(None);
        };
        if self.items[index].quantity == 1 {
            Ok(Some(self.items.remove(index)))
        } else {
            let id = self.allocate_item_instance_id()?;
            self.items[index].quantity -= 1;
            Ok(Some(ItemInstance {
                id,
                kind_id: self.items[index].kind_id.clone(),
                quantity: 1,
                quality: ItemQualityDto::Ordinary,
                affix_ids: Vec::new(),
                location: ItemLocation::Inventory,
            }))
        }
    }

    fn use_inventory_item(&mut self, item_id: &str, events: &mut Vec<DomainEvent>) {
        let Some(index) = self.items.iter().position(|item| {
            item.id == item_id && item.location == ItemLocation::Inventory && item.quantity > 0
        }) else {
            events.push(DomainEvent::ItemUseUnavailable);
            return;
        };
        let kind_id = self.items[index].kind_id.clone();
        let Some(action) = self
            .content
            .item(&kind_id)
            .and_then(|definition| definition.use_action.clone())
        else {
            events.push(DomainEvent::ItemUseUnavailable);
            return;
        };

        if self.items[index].quantity == 1 {
            let removed = self.items.remove(index);
            self.item_property_knowledge.remove(&removed.id);
        } else {
            self.items[index].quantity -= 1;
        }
        self.mark_item_tried(&kind_id);

        let (requested, applied) = match action.effect {
            ItemUseEffectDefinition::Heal { amount } => {
                let amount = i32::try_from(amount).expect("validated healing amount must fit i32");
                let max_hp = self.effective_player_max_hp();
                let player = &mut self.player;
                let outcome = apply_effect(
                    &mut EffectTarget {
                        hp: &mut player.hp,
                        max_hp,
                        resistances: &player.resistances,
                        statuses: &mut player.statuses,
                    },
                    EffectSpec::Heal { amount },
                );
                let EffectOutcome::Healed { requested, applied } = outcome else {
                    unreachable!("healing effects must produce healing outcomes");
                };
                (requested, applied)
            }
        };
        if applied > 0 {
            self.mark_item_aware(&kind_id);
        }
        events.push(DomainEvent::ItemUsed {
            display_name_key: self.item_display_name_key(&kind_id),
            source_kind_id: kind_id,
            requested,
            applied,
        });
    }

    fn player_is_dead(&self) -> bool {
        self.player.hp < 0
    }

    fn player_fear_blocks_melee(&mut self, target_index: usize) -> bool {
        let Some(fear) = self
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_FEAR)
            .cloned()
        else {
            return false;
        };
        let ability = self.player_derived_stats().melee_skill;
        let mut difficulty = DerivedStatsPipeline::new();
        difficulty.add_with_origin(
            StatKind::ActionDifficulty,
            StatLayer::Status,
            &fear.kind_id,
            fear.source_id,
            i32::from(fear.intensity).saturating_mul(40),
        );
        !resolve_check(
            &mut self.rng,
            CheckContext {
                kind: CheckKind::FearAction,
                actor_id: self.player.id.clone(),
                target_id: Some(self.entities[target_index].id.clone()),
                ability,
                difficulty: difficulty
                    .resolve(StatKind::ActionDifficulty, StatBounds::NON_NEGATIVE),
            },
        )
        .succeeded()
    }

    fn resolve_player_melee(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let definition = self
            .content
            .actor(&self.entities[index].kind_id)
            .expect("monster actor definition must remain available")
            .clone();
        let target_kind = self.entities[index].kind_id.clone();
        let attacker = self.player_derived_stats();
        let target = self.actor_derived_stats(&self.entities[index], &definition, false);
        let profile = self.player_melee_profile(&attacker);
        for _ in 0..profile.attacks {
            if attacker.melee_skill.value <= 0
                || !resolve_check(
                    &mut self.rng,
                    CheckContext {
                        kind: CheckKind::MeleeHit,
                        actor_id: self.player.id.clone(),
                        target_id: Some(self.entities[index].id.clone()),
                        ability: attacker.melee_skill.clone(),
                        difficulty: target.armor_class.clone(),
                    },
                )
                .succeeded()
            {
                events.push(DomainEvent::PlayerMeleeMissed {
                    target_kind_id: target_kind.clone(),
                });
                continue;
            }

            let rolled_damage = self
                .roll_damage(profile.damage_dice, profile.damage_sides)
                .saturating_add(profile.to_damage)
                .max(0);
            let damage_type = profile.damage_type;
            let resistance = self.entities[index].resistances.level(damage_type);
            let damage = resolve_damage(DamagePacket::new(rolled_damage, damage_type), resistance);
            self.entities[index].hp = self.entities[index].hp.saturating_sub(damage.applied);
            events.push(DomainEvent::PlayerMeleeHit {
                target_kind_id: target_kind.clone(),
                damage,
            });
            if self.entities[index].hp <= 0 {
                self.resolve_actor_death(
                    index,
                    DomainEvent::PlayerSlew {
                        target_kind_id: target_kind.clone(),
                        damage,
                    },
                    events,
                    changed,
                    removed_entities,
                )?;
                break;
            }
        }
        Ok(())
    }

    fn advance_until_player_ready(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        loop {
            self.world_tick = self.world_tick.saturating_add(1);
            self.process_status_tick(events, changed, removed_entities)?;
            if self.player_is_dead() {
                break;
            }
            self.process_monster_energy_pulse(events, changed);
            if self.player_is_dead() {
                break;
            }
            let speed = derived_speed(&self.player_derived_stats().speed);
            gain_energy(&mut self.player.energy_need, speed);
            if self.player.energy_need <= 0 {
                break;
            }
        }
        Ok(())
    }

    fn process_status_tick(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let player_tick = process_actor_status_tick(&mut self.player, false);
        for damage in player_tick.damage {
            events.push(DomainEvent::PlayerStatusDamaged {
                status_kind_id: damage.status_kind_id,
                damage: damage.outcome,
            });
        }
        for status_kind_id in player_tick.expired {
            events.push(DomainEvent::PlayerStatusExpired { status_kind_id });
        }
        if let Some(damage) = player_tick.fatal_damage {
            events.push(DomainEvent::PlayerDiedFromStatus {
                status_kind_id: damage.status_kind_id,
                damage: damage.outcome,
            });
            return Ok(());
        }

        let mut entity_ids = self
            .entities
            .iter()
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        entity_ids.sort();
        for entity_id in entity_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id)
            else {
                continue;
            };
            let target_kind_id = self.entities[index].kind_id.clone();
            let tick = process_actor_status_tick(&mut self.entities[index], true);
            for damage in tick.damage {
                events.push(DomainEvent::EntityStatusDamaged {
                    target_kind_id: target_kind_id.clone(),
                    status_kind_id: damage.status_kind_id,
                    damage: damage.outcome,
                });
            }
            for status_kind_id in tick.expired {
                events.push(DomainEvent::EntityStatusExpired {
                    target_kind_id: target_kind_id.clone(),
                    status_kind_id,
                });
            }
            if let Some(damage) = tick.fatal_damage {
                self.resolve_actor_death(
                    index,
                    DomainEvent::EntityDiedFromStatus {
                        target_kind_id,
                        status_kind_id: damage.status_kind_id,
                        damage: damage.outcome,
                    },
                    events,
                    changed,
                    removed_entities,
                )?;
            }
        }
        Ok(())
    }

    fn process_monster_energy_pulse(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let mut entity_ids = self
            .entities
            .iter()
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        entity_ids.sort();
        let mut surround_reservations = BTreeSet::new();

        for entity_id in entity_ids {
            if self.player_is_dead() {
                break;
            }
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id)
            else {
                continue;
            };
            let definition = self
                .content
                .actor(&self.entities[index].kind_id)
                .expect("monster actor definition must remain available");
            let speed = derived_speed(
                &self
                    .actor_derived_stats(&self.entities[index], definition, false)
                    .speed,
            );
            gain_energy(&mut self.entities[index].energy_need, speed);
            if self.entities[index].energy_need > 0 {
                continue;
            }
            spend_energy(&mut self.entities[index].energy_need, STANDARD_ACTION_COST);
            self.resolve_monster_action(index, events, changed, &mut surround_reservations);
        }
    }

    fn resolve_monster_action(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        surround_reservations: &mut BTreeSet<Position>,
    ) {
        let behavior = self.entities[index]
            .pack
            .as_ref()
            .map_or(MonsterPackBehaviorDto::Seek, |pack| pack.behavior);
        if adjacent(self.entities[index].position, self.player.position) {
            if behavior == MonsterPackBehaviorDto::Surround {
                surround_reservations.insert(self.entities[index].position);
            }
            self.resolve_monster_melee(index, events);
            return;
        }
        let next_position = match behavior {
            MonsterPackBehaviorDto::Seek => self.next_monster_step(index),
            MonsterPackBehaviorDto::Surround => self
                .next_surround_step(index, surround_reservations)
                .or_else(|| self.next_monster_step(index)),
            MonsterPackBehaviorDto::GuardLeader => {
                let leader_position = self.entities[index].pack.as_ref().and_then(|pack| {
                    self.entities
                        .iter()
                        .find(|entity| entity.id == pack.leader_id)
                        .map(|leader| leader.position)
                });
                match leader_position {
                    Some(position) if adjacent(self.entities[index].position, position) => None,
                    Some(position) => self.next_monster_step_toward(index, position, true),
                    None => self.next_monster_step(index),
                }
            }
        };
        let Some(next_position) = next_position else {
            return;
        };
        let old_position = self.entities[index].position;
        self.entities[index].position = next_position;
        changed.insert(old_position);
        changed.insert(next_position);
    }

    fn resolve_monster_melee(&mut self, index: usize, events: &mut Vec<DomainEvent>) {
        let kind_id = self.entities[index].kind_id.clone();
        let definition = self
            .content
            .actor(&kind_id)
            .expect("monster actor definition must remain available")
            .clone();
        let attacker = self.actor_derived_stats(&self.entities[index], &definition, false);
        let target = self.player_derived_stats();
        let armor_class = target.armor_class.value;
        for blow in resolved_melee_blows(&definition) {
            let ability = attacker.melee_skill.with_modifier(
                StatLayer::Base,
                blow.method_id.as_deref().unwrap_or(definition.id.as_str()),
                blow.to_hit,
                StatBounds::NON_NEGATIVE,
            );
            if !resolve_check(
                &mut self.rng,
                CheckContext {
                    kind: CheckKind::MeleeHit,
                    actor_id: self.entities[index].id.clone(),
                    target_id: Some(self.player.id.clone()),
                    ability,
                    difficulty: target.armor_class.clone(),
                },
            )
            .succeeded()
            {
                events.push(DomainEvent::MonsterMeleeMissed {
                    source_kind_id: kind_id.clone(),
                    method_id: blow.method_id,
                });
                continue;
            }

            let raw_damage = self.roll_damage(blow.damage_dice, blow.damage_sides);
            let prepared_damage = if blow.damage_type == DamageType::Physical {
                apply_melee_armor_reduction(raw_damage, armor_class)
            } else {
                raw_damage
            };
            let resistance = self.player.resistances.level(blow.damage_type);
            let damage = resolve_damage(
                DamagePacket::after_armor(raw_damage, prepared_damage, blow.damage_type),
                resistance,
            );
            self.player.hp = self.player.hp.saturating_sub(damage.applied);
            events.push(DomainEvent::MonsterMeleeHit {
                source_kind_id: kind_id.clone(),
                method_id: blow.method_id.clone(),
                damage,
            });
            if self.player_is_dead() {
                events.push(DomainEvent::PlayerDied {
                    source_kind_id: kind_id.clone(),
                    method_id: blow.method_id,
                    damage,
                });
                break;
            }
        }
    }

    fn next_monster_step(&self, index: usize) -> Option<Position> {
        self.next_monster_step_toward(index, self.player.position, true)
    }

    fn next_surround_step(
        &self,
        index: usize,
        reservations: &mut BTreeSet<Position>,
    ) -> Option<Position> {
        const DELTAS: [(i32, i32); 8] = [
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ];

        let pack = self.entities[index].pack.as_ref()?;
        let mut surround_members = self
            .entities
            .iter()
            .filter(|entity| {
                entity.pack.as_ref().is_some_and(|candidate| {
                    candidate.id == pack.id
                        && candidate.behavior == MonsterPackBehaviorDto::Surround
                })
            })
            .map(|entity| entity.id.as_str())
            .collect::<Vec<_>>();
        surround_members.sort_unstable();
        let rank = surround_members
            .iter()
            .position(|actor_id| *actor_id == self.entities[index].id)
            .unwrap_or(0);
        let occupied = self
            .entities
            .iter()
            .enumerate()
            .filter(|(entity_index, _)| *entity_index != index)
            .map(|(_, entity)| entity.position)
            .collect::<BTreeSet<_>>();
        for offset in 0..DELTAS.len() {
            let (dx, dy) = DELTAS[(rank + offset) % DELTAS.len()];
            let target = Position {
                x: self.player.position.x + dx,
                y: self.player.position.y + dy,
            };
            if target == self.player.position
                || occupied.contains(&target)
                || reservations.contains(&target)
                || !self.is_walkable(target)
            {
                continue;
            }
            if let Some(step) = self.next_monster_step_toward(index, target, false) {
                reservations.insert(target);
                return Some(step);
            }
        }
        None
    }

    fn next_monster_step_toward(
        &self,
        index: usize,
        target: Position,
        stop_adjacent: bool,
    ) -> Option<Position> {
        const DELTAS: [(i32, i32); 8] = [
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ];

        let start = self.entities[index].position;
        let movement_region = self
            .floor_regions
            .iter()
            .find(|region| region.cells.contains(&start));
        let occupied_now = self
            .entities
            .iter()
            .enumerate()
            .filter(|(entity_index, _)| *entity_index != index)
            .map(|(_, entity)| entity.position)
            .collect::<BTreeSet<_>>();
        let moving_pack_id = self.entities[index]
            .pack
            .as_ref()
            .map(|pack| pack.id.as_str());
        let path_blockers =
            self.entities
                .iter()
                .enumerate()
                .filter(|(entity_index, entity)| {
                    *entity_index != index
                        && !entity.pack.as_ref().is_some_and(|pack| {
                            moving_pack_id.is_some_and(|moving| moving == pack.id)
                        })
                })
                .map(|(_, entity)| entity.position)
                .collect::<BTreeSet<_>>();
        let mut visited = BTreeSet::from([start]);
        let mut queue = VecDeque::new();

        let mut initial = DELTAS
            .iter()
            .enumerate()
            .map(|(order, (dx, dy))| {
                let position = Position {
                    x: start.x + dx,
                    y: start.y + dy,
                };
                (squared_distance(position, target), order, position)
            })
            .collect::<Vec<_>>();
        initial.sort();
        for (_, _, position) in initial {
            if position == self.player.position
                || occupied_now.contains(&position)
                || !self.is_walkable(position)
                || movement_region.is_some_and(|region| !region.cells.contains(&position))
                || !visited.insert(position)
            {
                continue;
            }
            if (!stop_adjacent && position == target)
                || (stop_adjacent && adjacent(position, target))
            {
                return Some(position);
            }
            queue.push_back((position, position));
        }

        while let Some((position, first_step)) = queue.pop_front() {
            let mut neighbors = DELTAS
                .iter()
                .enumerate()
                .map(|(order, (dx, dy))| {
                    let next = Position {
                        x: position.x + dx,
                        y: position.y + dy,
                    };
                    (squared_distance(next, target), order, next)
                })
                .collect::<Vec<_>>();
            neighbors.sort();
            for (_, _, next) in neighbors {
                if next == self.player.position
                    || path_blockers.contains(&next)
                    || !self.is_walkable(next)
                    || movement_region.is_some_and(|region| !region.cells.contains(&next))
                    || !visited.insert(next)
                {
                    continue;
                }
                if (!stop_adjacent && next == target) || (stop_adjacent && adjacent(next, target)) {
                    return Some(first_step);
                }
                queue.push_back((next, first_step));
            }
        }
        None
    }

    fn roll_damage(&mut self, dice: u16, sides: u16) -> i32 {
        (0..dice).fold(0_i32, |total, _| {
            let roll = i32::try_from(self.rng.bounded(u64::from(sides)))
                .unwrap_or(i32::MAX)
                .saturating_add(1);
            total.saturating_add(roll)
        })
    }

    fn initialize_carried_loot(&mut self) -> Result<(), CoreError> {
        let floor_id = self.current_floor_id.clone();
        let depth = self.floor_depth(&floor_id);
        let actors = self.entities.clone();
        let generated = self.generate_carried_loot_for_actors(&actors, &floor_id, depth)?;
        self.items.extend(generated);
        Ok(())
    }

    fn generate_carried_loot_for_actors(
        &mut self,
        actors: &[Actor],
        floor_id: &str,
        depth: u16,
    ) -> Result<Vec<ItemInstance>, CoreError> {
        let mut carriers = actors
            .iter()
            .filter_map(|actor| {
                self.content
                    .actor(&actor.kind_id)
                    .and_then(|definition| definition.carried_loot_table_id.clone())
                    .map(|table_id| (actor.id.clone(), table_id))
            })
            .collect::<Vec<_>>();
        carriers.sort_by(|left, right| left.0.cmp(&right.0));
        let mut items = Vec::new();
        for (actor_id, table_id) in carriers {
            let generated = self.generate_loot_instances(
                &LootContext {
                    table_id,
                    floor_id: floor_id.to_owned(),
                    depth,
                    source: LootSource::MonsterCarried {
                        actor_id: actor_id.clone(),
                    },
                },
                ItemLocation::CarriedBy { actor_id },
            )?;
            items.extend(generated);
        }
        Ok(items)
    }

    fn resolve_actor_death(
        &mut self,
        index: usize,
        death_event: DomainEvent,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let actor = self.entities[index].clone();
        let generated = self.generate_death_loot(&actor)?;
        let mut carried = self
            .items
            .iter()
            .filter_map(|item| match &item.location {
                ItemLocation::CarriedBy { actor_id } if actor_id == &actor.id => {
                    Some((item.id.clone(), item.kind_id.clone(), item.quantity))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        carried.sort_by(|left, right| left.0.cmp(&right.0));
        let has_drops = !carried.is_empty() || !generated.is_empty();
        let dissolved_pack_id = actor
            .pack
            .as_ref()
            .and_then(|pack| (pack.role == MonsterPackRoleDto::Leader).then(|| pack.id.clone()));

        let removed = self.entities.remove(index);
        if let Some(pack_id) = dissolved_pack_id {
            for entity in &mut self.entities {
                if entity.pack.as_ref().is_some_and(|pack| pack.id == pack_id) {
                    entity.pack = None;
                }
            }
        }
        removed_entities.push(removed.id.clone());
        events.push(death_event);
        let defeated_guardian = self
            .content
            .world(&self.world_id)
            .and_then(|world| {
                world
                    .procedural_floors
                    .iter()
                    .find(|floor| floor.id == self.current_floor_id)
            })
            .and_then(|floor| {
                floor.guardian.as_ref().and_then(|guardian| {
                    (guardian.instance_id == removed.id).then(|| {
                        (
                            floor
                                .dungeon_id
                                .clone()
                                .expect("guardian floor must have a dungeon ID"),
                            floor.id.clone(),
                            guardian.actor_kind_id.clone(),
                        )
                    })
                })
            });
        if let Some((dungeon_id, floor_id, target_kind_id)) = defeated_guardian {
            let state = self
                .dungeon_states
                .get_mut(&dungeon_id)
                .expect("guardian dungeon state must remain available");
            let first_defeat = !state.guardian_defeated;
            if first_defeat {
                state.guardian_defeated = true;
                events.push(DomainEvent::DungeonGuardianDefeated {
                    dungeon_id: dungeon_id.clone(),
                    floor_id,
                    target_kind_id,
                });
                let mirror_ids = self
                    .content
                    .world(&self.world_id)
                    .expect("active world must remain available")
                    .procedural_floors
                    .iter()
                    .filter(|floor| {
                        floor.dungeon_id.as_deref() == Some(dungeon_id.as_str())
                            && floor.final_floor
                    })
                    .filter_map(|floor| {
                        floor
                            .guardian
                            .as_ref()
                            .map(|guardian| guardian.instance_id.as_str())
                    })
                    .collect::<BTreeSet<_>>();
                for floor in self.stored_floors.values_mut() {
                    floor
                        .entities
                        .retain(|entity| !mirror_ids.contains(entity.id.as_str()));
                    floor.items.retain(|item| {
                        !matches!(&item.location, ItemLocation::CarriedBy { actor_id } if mirror_ids.contains(actor_id.as_str()))
                    });
                }
            }
        }

        for (item_id, target_kind_id, quantity) in carried {
            let item = self
                .items
                .iter_mut()
                .find(|item| item.id == item_id)
                .expect("carried item collected from authoritative item set");
            item.location = ItemLocation::Ground(removed.position);
            events.push(DomainEvent::LootDropped {
                source_kind_id: removed.kind_id.clone(),
                target_kind_id,
                quantity,
            });
        }
        for item in generated {
            events.push(DomainEvent::LootDropped {
                source_kind_id: removed.kind_id.clone(),
                target_kind_id: item.kind_id.clone(),
                quantity: item.quantity,
            });
            self.items.push(item);
        }
        if has_drops {
            changed.insert(removed.position);
        }
        Ok(())
    }

    fn apply_task_events(&mut self, events: &[DomainEvent]) {
        let Some((task_id, stage_index)) = self.task_states.iter().find_map(|(task_id, state)| {
            (state.status == TaskStatusKindDto::Active
                && state.active_floor_id.as_deref() == Some(self.current_floor_id.as_str()))
            .then_some((task_id.clone(), state.stage_index))
        }) else {
            return;
        };
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        let objectives = task_objectives(world, &task_id);
        let Some(objective) = usize::try_from(stage_index)
            .ok()
            .and_then(|stage| objectives.get(stage))
            .copied()
            .cloned()
        else {
            return;
        };
        let increment = match objective.kind {
            TaskObjectiveKind::CollectItem => events.iter().any(|event| {
                matches!(event, DomainEvent::ItemPickedUp { .. })
                    && objective.item_instance_id.as_ref().is_some_and(|id| {
                        self.items.iter().any(|item| {
                            &item.id == id
                                && matches!(
                                    item.location,
                                    ItemLocation::Inventory | ItemLocation::Equipped { .. }
                                )
                        })
                    })
            }) as u32,
            TaskObjectiveKind::EnterFloor => events.iter().any(|event| {
                matches!(
                    event,
                    DomainEvent::FloorTransitioned { to_floor_id, .. }
                        if objective.floor_id.as_deref() == Some(to_floor_id.as_str())
                )
            }) as u32,
            TaskObjectiveKind::KillActor => events.iter().any(|event| {
                task_death_target_kind(event).is_some()
                    && objective
                        .actor_instance_id
                        .as_ref()
                        .is_some_and(|id| !self.entities.iter().any(|entity| &entity.id == id))
            }) as u32,
            TaskObjectiveKind::KillActorKind => events
                .iter()
                .filter(|event| task_death_target_kind(event) == objective.actor_kind_id.as_deref())
                .count()
                .try_into()
                .unwrap_or(u32::MAX),
        };
        if increment > 0 {
            let state = self
                .task_states
                .get_mut(&task_id)
                .expect("active task state must remain available");
            state.current = state.current.saturating_add(increment).min(state.required);
            if state.current >= state.required
                && usize::try_from(state.stage_index)
                    .ok()
                    .is_some_and(|stage| stage + 1 < objectives.len())
            {
                state.stage_index = state.stage_index.saturating_add(1);
                state.current = 0;
                state.required = objectives[usize::try_from(state.stage_index)
                    .expect("validated task stage must fit usize")]
                .required;
            }
        }
    }

    fn generate_death_loot(&mut self, actor: &Actor) -> Result<Vec<ItemInstance>, CoreError> {
        let Some(table_id) = self
            .content
            .actor(&actor.kind_id)
            .and_then(|definition| definition.loot_table_id.clone())
        else {
            return Ok(Vec::new());
        };
        self.generate_loot_instances(
            &LootContext {
                table_id,
                floor_id: self.current_floor_id.clone(),
                depth: self.floor_depth(&self.current_floor_id),
                source: LootSource::MonsterDeath {
                    actor_id: actor.id.clone(),
                },
            },
            ItemLocation::Ground(actor.position),
        )
    }

    fn generate_loot_instances(
        &mut self,
        context: &LootContext,
        location: ItemLocation,
    ) -> Result<Vec<ItemInstance>, CoreError> {
        let context_is_valid = !context.floor_id.is_empty()
            && match &context.source {
                LootSource::MonsterCarried { actor_id } | LootSource::MonsterDeath { actor_id } => {
                    !actor_id.is_empty()
                }
                LootSource::FloorRoom { room_id, spawn_id } => {
                    context.depth > 0 && !room_id.is_empty() && !spawn_id.is_empty()
                }
                LootSource::Vault { vault_id, spawn_id } => {
                    context.depth > 0 && !vault_id.is_empty() && !spawn_id.is_empty()
                }
            };
        debug_assert!(context_is_valid, "validated loot context must remain valid");
        let table = self
            .content
            .loot_table(&context.table_id)
            .expect("validated actor loot table must remain available")
            .clone();
        self.next_item_instance_serial
            .checked_add(u64::from(table.rolls))
            .ok_or(CoreError::ItemIdExhausted)?;
        let entry_weights = table
            .entries
            .iter()
            .map(|entry| entry.weight)
            .collect::<Vec<_>>();
        let quality_weights = table
            .quality_weights
            .iter()
            .map(|entry| entry.weight)
            .collect::<Vec<_>>();
        let affix_weights = table
            .affix_weights
            .iter()
            .map(|entry| entry.weight)
            .collect::<Vec<_>>();
        let mut generated = Vec::with_capacity(usize::from(table.rolls));
        for _ in 0..table.rolls {
            let entry_index = self.roll_weighted_index(&entry_weights);
            let quality_index = self.roll_weighted_index(&quality_weights);
            let affix_index = self.roll_weighted_index(&affix_weights);
            let entry = &table.entries[entry_index];
            let quality = item_quality_dto(table.quality_weights[quality_index].quality);
            let affix_ids = if quality == ItemQualityDto::Ordinary {
                Vec::new()
            } else {
                table.affix_weights[affix_index]
                    .affix_id
                    .iter()
                    .cloned()
                    .collect()
            };
            let item = ItemInstance {
                id: self.allocate_item_instance_id()?,
                kind_id: entry.item_kind_id.clone(),
                quantity: entry.quantity,
                quality,
                affix_ids,
                location: location.clone(),
            };
            generated.push(item);
        }
        Ok(generated)
    }

    fn roll_weighted_index(&mut self, weights: &[u32]) -> usize {
        let total = weights.iter().map(|weight| u64::from(*weight)).sum();
        let mut roll = self.rng.bounded(total);
        for (index, weight) in weights.iter().enumerate() {
            let weight = u64::from(*weight);
            if roll < weight {
                return index;
            }
            roll -= weight;
        }
        unreachable!("validated positive weighted table must select an entry")
    }

    fn clamp_player_hp_to_effective_max(&mut self) {
        self.player.hp = self.player.hp.min(self.effective_player_max_hp());
    }

    fn allocate_item_instance_id(&mut self) -> Result<String, CoreError> {
        loop {
            let serial = self.next_item_instance_serial;
            let next = serial.checked_add(1).ok_or(CoreError::ItemIdExhausted)?;
            let candidate = format!("{GENERATED_ITEM_ID_PREFIX}{serial}");
            self.next_item_instance_serial = next;
            if !self.instance_id_exists(&candidate) {
                return Ok(candidate);
            }
        }
    }

    fn instance_id_exists(&self, candidate: &str) -> bool {
        self.player.id == candidate
            || self.entities.iter().any(|entity| entity.id == candidate)
            || self.items.iter().any(|item| item.id == candidate)
    }

    fn content_visuals(&self) -> Vec<ContentVisualDto> {
        self.content
            .visual_glyphs()
            .into_iter()
            .map(|(id, glyph)| ContentVisualDto { id, glyph })
            .collect()
    }

    fn cell_dto(&self, position: Position) -> CellDto {
        let actor_id = if self.player.position == position {
            Some(self.player.id.clone())
        } else {
            self.entities
                .iter()
                .find(|entity| entity.position == position)
                .map(|entity| entity.id.clone())
        };
        CellDto {
            position,
            terrain_id: self.known_terrain_at(position).to_owned(),
            item_id: self
                .items
                .iter()
                .find(|item| item.location == ItemLocation::Ground(position))
                .map(|item| item.id.clone()),
            actor_id,
        }
    }

    fn visual_cells(&self) -> Vec<CellVisualDto> {
        let mut visuals = Vec::with_capacity(self.terrain.len());
        for y in 0..self.height {
            for x in 0..self.width {
                visuals.push(self.cell_visual(Position {
                    x: i32::from(x),
                    y: i32::from(y),
                }));
            }
        }
        visuals
    }

    fn changed_visual_cells(&self, previous: &[CellVisualDto]) -> Vec<CellVisualDto> {
        self.visual_cells()
            .into_iter()
            .zip(previous.iter())
            .filter_map(|(current, before)| (current != *before).then_some(current))
            .collect()
    }

    fn cell_visual(&self, position: Position) -> CellVisualDto {
        let index = self.index(position).expect("validated visual position");
        CellVisualDto {
            position,
            visibility: if self.is_visible(position) {
                VisibilityState::Visible
            } else if self.explored[index] {
                VisibilityState::Remembered
            } else {
                VisibilityState::Hidden
            },
            light: self.light_at(position),
        }
    }

    fn reveal_current_visibility(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                let position = Position {
                    x: i32::from(x),
                    y: i32::from(y),
                };
                if self.is_visible(position) {
                    let index = self.index(position).expect("visibility position is valid");
                    self.explored[index] = true;
                }
            }
        }
    }

    fn is_visible(&self, position: Position) -> bool {
        if squared_distance(self.player.position, position) > VISIBILITY_RADIUS * VISIBILITY_RADIUS
        {
            return false;
        }
        has_line_of_sight(self, self.player.position, position)
    }

    fn light_at(&self, position: Position) -> CellLightDto {
        let mut strongest = (0_u8, PLAYER_LIGHT_COLOR);
        let player_boost =
            source_intensity(self.player.position, position, PLAYER_LIGHT_RADIUS, 72);
        if player_boost > strongest.0 {
            strongest = (player_boost, PLAYER_LIGHT_COLOR);
        }

        for entity in &self.entities {
            let Some(definition) = self.content.actor(&entity.kind_id) else {
                continue;
            };
            if !definition.tags.iter().any(|tag| tag == "light-source") {
                continue;
            }
            let boost = source_intensity(entity.position, position, ACTOR_LIGHT_RADIUS, 64);
            if boost > strongest.0 {
                strongest = (boost, ACTOR_LIGHT_COLOR);
            }
        }

        for item in &self.items {
            let ItemLocation::Ground(item_position) = &item.location else {
                continue;
            };
            let Some(definition) = self.content.item(&item.kind_id) else {
                continue;
            };
            if !definition.tags.iter().any(|tag| tag == "light-source") {
                continue;
            }
            let boost = source_intensity(*item_position, position, ITEM_LIGHT_RADIUS, 52);
            if boost > strongest.0 {
                strongest = (boost, ITEM_LIGHT_COLOR);
            }
        }

        CellLightDto {
            color: strongest.1,
            intensity: AMBIENT_LIGHT.saturating_add(strongest.0),
        }
    }

    fn discard_stored_task_floors(&mut self, members: &[ProceduralFloorDefinition]) {
        let mut discarded_item_ids = BTreeSet::new();
        for definition in members {
            if let Some(floor) = self.stored_floors.remove(&definition.id) {
                discarded_item_ids.extend(floor.items.into_iter().map(|item| item.id));
            }
        }
        self.item_property_knowledge
            .retain(|item_id, _| !discarded_item_ids.contains(item_id));
    }

    fn discard_stored_dungeon_instance(&mut self, instance_id: &str) {
        let mut discarded_item_ids = BTreeSet::new();
        self.stored_floors.retain(|_, floor| {
            if floor.dungeon_instance_id.as_deref() == Some(instance_id) {
                discarded_item_ids.extend(floor.items.iter().map(|item| item.id.clone()));
                false
            } else {
                true
            }
        });
        self.item_property_knowledge
            .retain(|item_id, _| !discarded_item_ids.contains(item_id));
    }

    fn abandon_paused_task(&mut self, task_id: &str) -> Option<Vec<Position>> {
        let world = self.content.world(&self.world_id)?;
        if self.current_floor_id != world.initial_floor_id
            || self
                .task_states
                .get(task_id)
                .is_none_or(|state| state.status != TaskStatusKindDto::Paused)
        {
            return None;
        }
        let members = world
            .procedural_floors
            .iter()
            .filter(|floor| {
                floor.lifecycle == FloorLifecycle::OneShot
                    && floor.retakeable
                    && floor_task_id(floor) == task_id
            })
            .cloned()
            .collect::<Vec<_>>();
        let initial_required = initial_task_states(world).get(task_id)?.required;
        if members.is_empty() {
            return None;
        }

        self.discard_stored_task_floors(&members);
        let mut changed = BTreeSet::new();
        for definition in &members {
            let (Some(entry_id), Some(abandoned_id)) = (
                definition.entry_terrain_id.as_deref(),
                definition.abandoned_entry_terrain_id.as_deref(),
            ) else {
                continue;
            };
            for (index, terrain_id) in self.terrain.iter_mut().enumerate() {
                if terrain_id == entry_id {
                    *terrain_id = abandoned_id.to_owned();
                    changed.insert(Position {
                        x: i32::try_from(index % usize::from(self.width)).ok()?,
                        y: i32::try_from(index / usize::from(self.width)).ok()?,
                    });
                }
            }
        }
        let state = self
            .task_states
            .get_mut(task_id)
            .expect("paused task state must remain available");
        state.status = TaskStatusKindDto::Abandoned;
        state.stage_index = 0;
        state.current = 0;
        state.required = initial_required;
        state.active_floor_id = None;
        Some(changed.into_iter().collect())
    }

    fn traverse_stairs(
        &mut self,
        abandon_task: bool,
    ) -> Result<Option<FloorTransitionOutcome>, CoreError> {
        let terrain_id = self.terrain_at(self.player.position).to_owned();
        let terrain = self
            .content
            .terrain(&terrain_id)
            .expect("active terrain must remain available");
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        let initial_floor_id = world.initial_floor_id.clone();
        let procedural_floors = world.procedural_floors.clone();
        let initial_task_states_by_id = initial_task_states(world);
        if abandon_task
            && !procedural_floors.iter().any(|floor| {
                floor.id == self.current_floor_id && floor.lifecycle == FloorLifecycle::OneShot
            })
        {
            return Ok(None);
        }
        let (target_floor_id, arrival_connection_id, departure_connection_id) = if abandon_task {
            (initial_floor_id.clone(), None, None)
        } else if self.current_floor_id == initial_floor_id {
            let Some(target) = procedural_floors.iter().find(|floor| {
                floor.return_floor_id == initial_floor_id
                    && floor.entry_terrain_id.as_deref() == Some(terrain_id.as_str())
            }) else {
                return Ok(None);
            };
            (target.id.clone(), target.entry_connection_id.clone(), None)
        } else if let Some(current) = procedural_floors
            .iter()
            .find(|floor| floor.id == self.current_floor_id)
        {
            if let Some(connection_state) = self
                .floor_connections
                .iter()
                .find(|connection| connection.position == self.player.position)
            {
                let connection = current
                    .connections
                    .iter()
                    .find(|connection| connection.id == connection_state.id)
                    .ok_or(CoreError::InvalidSave(
                        "active floor connection is missing from content",
                    ))?;
                let target_floor_id = connection_state
                    .target_floor_id
                    .clone()
                    .unwrap_or_else(|| connection.target_floor_id.clone());
                let target_connection_id = if connection_state.target_floor_id.is_some() {
                    connection_state.target_connection_id.clone()
                } else {
                    connection.target_connection_id.clone()
                };
                (
                    target_floor_id,
                    target_connection_id,
                    Some(connection_state.id.clone()),
                )
            } else if terrain.tags.iter().any(|tag| tag == "stairs-up") {
                (current.return_floor_id.clone(), None, None)
            } else if terrain.tags.iter().any(|tag| tag == "stairs-down") {
                (
                    current.next_floor_id.clone().ok_or(CoreError::InvalidSave(
                        "downward floor connection is missing",
                    ))?,
                    None,
                    None,
                )
            } else {
                return Ok(None);
            }
        } else {
            return Ok(None);
        };

        if let Some(target) = procedural_floors
            .iter()
            .find(|floor| floor.id == target_floor_id && floor.lifecycle == FloorLifecycle::OneShot)
        {
            let task_id = floor_task_id(target);
            let state = self
                .task_states
                .get(task_id)
                .expect("target task state must remain available");
            if state.status == TaskStatusKindDto::Paused
                && target
                    .max_retakes
                    .is_some_and(|maximum| state.retakes_used >= maximum)
            {
                return Ok(None);
            }
            let required_floor_id = task_objectives(world, task_id)
                .get(usize::try_from(state.stage_index).unwrap_or(usize::MAX))
                .and_then(|objective| objective.floor_id.as_deref());
            if required_floor_id.is_some_and(|floor_id| floor_id != target_floor_id) {
                return Ok(None);
            }
        }

        let from_floor_id = self.current_floor_id.clone();
        let from_dungeon_instance_id = self.current_dungeon_instance_id.clone();
        let source_definition = procedural_floors
            .iter()
            .find(|floor| floor.id == from_floor_id);
        let expedition_ended = target_floor_id == initial_floor_id
            && source_definition.is_some_and(|floor| floor.lifecycle == FloorLifecycle::Dungeon);
        let mut allocated_dungeon_instance = None;
        let target_dungeon_instance_id = if let Some(target) = procedural_floors
            .iter()
            .find(|floor| floor.id == target_floor_id)
            .filter(|floor| floor.lifecycle == FloorLifecycle::Dungeon)
        {
            let dungeon_id = target
                .dungeon_id
                .as_deref()
                .expect("dungeon floor must retain a dungeon ID");
            if source_definition
                .is_some_and(|source| source.dungeon_id.as_deref() == Some(dungeon_id))
            {
                from_dungeon_instance_id.clone()
            } else if from_floor_id == initial_floor_id {
                let state = self
                    .dungeon_states
                    .get(dungeon_id)
                    .expect("target dungeon state must remain available");
                let ordinal = state
                    .next_instance_ordinal
                    .checked_add(1)
                    .ok_or(CoreError::InvalidSave("dungeon instance ordinal overflow"))?;
                allocated_dungeon_instance = Some((dungeon_id.to_owned(), ordinal));
                Some(dungeon_instance_id(dungeon_id, ordinal))
            } else {
                return Err(CoreError::InvalidSave(
                    "cross-dungeon floor transition is invalid",
                ));
            }
        } else {
            None
        };
        let one_shot_source = source_definition
            .filter(|floor| {
                target_floor_id == initial_floor_id && floor.lifecycle == FloorLifecycle::OneShot
            })
            .cloned();
        let one_shot_task_id = one_shot_source
            .as_ref()
            .map(floor_task_id)
            .map(str::to_owned);
        let task_members = one_shot_task_id.as_ref().map_or_else(Vec::new, |task_id| {
            procedural_floors
                .iter()
                .filter(|floor| {
                    floor.lifecycle == FloorLifecycle::OneShot && floor_task_id(floor) == task_id
                })
                .cloned()
                .collect::<Vec<_>>()
        });
        let task_succeeded = one_shot_task_id.as_ref().is_some_and(|task_id| {
            self.task_states
                .get(task_id)
                .is_some_and(|state| task_succeeded(world, task_id, state))
        });
        if !abandon_task
            && one_shot_source.as_ref().is_some_and(|floor| {
                !floor.retakeable && !floor.allow_early_task_exit && !task_succeeded
            })
        {
            return Ok(None);
        }
        let task_resolution = if one_shot_source.is_none() {
            None
        } else if abandon_task {
            Some(TaskResolution::Abandoned)
        } else if task_succeeded {
            Some(TaskResolution::Completed)
        } else if one_shot_source
            .as_ref()
            .is_some_and(|floor| floor.retakeable)
        {
            None
        } else {
            Some(TaskResolution::Failed)
        };
        let all_items = std::mem::take(&mut self.items);
        let (floor_items, global_items): (Vec<_>, Vec<_>) =
            all_items.into_iter().partition(|item| {
                matches!(
                    item.location,
                    ItemLocation::Ground(_) | ItemLocation::CarriedBy { .. }
                )
            });
        let current = FloorState {
            id: from_floor_id.clone(),
            dungeon_instance_id: from_dungeon_instance_id.clone(),
            width: self.width,
            height: self.height,
            terrain: std::mem::take(&mut self.terrain),
            player_position: self.player.position,
            entities: std::mem::take(&mut self.entities),
            items: floor_items,
            explored: std::mem::take(&mut self.explored),
            revealed_terrain: std::mem::take(&mut self.revealed_terrain),
            connections: std::mem::take(&mut self.floor_connections),
            regions: std::mem::take(&mut self.floor_regions),
        };
        self.stored_floors.insert(
            dungeon_instance_storage_key(from_dungeon_instance_id.as_deref(), &from_floor_id),
            current,
        );

        let task_resumed = procedural_floors
            .iter()
            .find(|floor| {
                floor.id == target_floor_id
                    && floor.lifecycle == FloorLifecycle::OneShot
                    && floor.retakeable
            })
            .is_some_and(|floor| {
                self.task_states
                    .get(floor_task_id(floor))
                    .is_some_and(|state| state.status == TaskStatusKindDto::Paused)
            });
        if task_resumed
            && let Some(target) = procedural_floors
                .iter()
                .find(|floor| floor.id == target_floor_id)
            && target.retake_floor_policy == RetakeFloorPolicy::RegenerateFloor
        {
            let resumed_task_id = floor_task_id(target);
            let resumed_members = procedural_floors
                .iter()
                .filter(|floor| {
                    floor.lifecycle == FloorLifecycle::OneShot
                        && floor_task_id(floor) == resumed_task_id
                })
                .cloned()
                .collect::<Vec<_>>();
            self.discard_stored_task_floors(&resumed_members);
        }
        let target_storage_key =
            dungeon_instance_storage_key(target_dungeon_instance_id.as_deref(), &target_floor_id);
        let mut destination_was_generated = false;
        let mut destination = if let Some(floor) = self.stored_floors.remove(&target_storage_key) {
            floor
        } else if let Some(definition) = procedural_floors
            .iter()
            .find(|floor| floor.id == target_floor_id)
        {
            destination_was_generated = true;
            self.generate_procedural_floor(definition, target_dungeon_instance_id.clone())?
        } else {
            return Err(CoreError::InvalidSave("return floor state is missing"));
        };
        if destination_was_generated
            && let (Some(arrival_connection_id), Some(departure_connection_id)) = (
                arrival_connection_id.as_ref(),
                departure_connection_id.as_ref(),
            )
            && let Some(connection) = destination
                .connections
                .iter_mut()
                .find(|connection| connection.id == *arrival_connection_id)
        {
            connection.target_floor_id = Some(from_floor_id.clone());
            connection.target_connection_id = Some(departure_connection_id.clone());
        }
        if let Some(arrival_connection_id) = arrival_connection_id {
            if let Some(connection) = destination
                .connections
                .iter()
                .find(|connection| connection.id == arrival_connection_id)
            {
                destination.player_position = connection.position;
            } else if !destination.connections.is_empty() {
                return Err(CoreError::InvalidSave(
                    "destination floor connection is missing",
                ));
            }
        }
        if let Some((dungeon_id, ordinal)) = allocated_dungeon_instance {
            self.dungeon_states
                .get_mut(&dungeon_id)
                .expect("target dungeon state must remain available")
                .next_instance_ordinal = ordinal;
        }
        if expedition_ended {
            let instance_id = from_dungeon_instance_id
                .as_deref()
                .ok_or(CoreError::InvalidSave(
                    "active dungeon floor is missing its instance ID",
                ))?;
            self.discard_stored_dungeon_instance(instance_id);
        }
        if one_shot_source.is_some()
            && let Some(task_resolution) = task_resolution
        {
            self.discard_stored_task_floors(&task_members);
            for definition in &task_members {
                if let (Some(entry_id), Some(result_id)) = (
                    definition.entry_terrain_id.as_deref(),
                    match task_resolution {
                        TaskResolution::Completed => {
                            definition.completed_entry_terrain_id.as_deref()
                        }
                        TaskResolution::Failed => definition.failed_entry_terrain_id.as_deref(),
                        TaskResolution::Abandoned => {
                            definition.abandoned_entry_terrain_id.as_deref()
                        }
                    },
                ) {
                    for terrain_id in &mut destination.terrain {
                        if terrain_id == entry_id {
                            *terrain_id = result_id.to_owned();
                        }
                    }
                }
            }
            if task_resolution == TaskResolution::Completed
                && let Some(reward) = task_members
                    .iter()
                    .find_map(|definition| definition.task_reward.as_ref())
            {
                destination.items.push(ItemInstance {
                    id: reward.item_instance_id.clone(),
                    kind_id: reward.item_kind_id.clone(),
                    quantity: reward.quantity,
                    quality: ItemQualityDto::Ordinary,
                    affix_ids: Vec::new(),
                    location: ItemLocation::Ground(destination.player_position),
                });
            }
        }
        if let Some(task_id) = &one_shot_task_id {
            let state = self
                .task_states
                .get_mut(task_id)
                .expect("active task state must remain available");
            state.active_floor_id = None;
            state.status = match task_resolution {
                Some(TaskResolution::Completed) => {
                    state.current = state.required;
                    TaskStatusKindDto::Completed
                }
                Some(TaskResolution::Failed) => {
                    state.stage_index = 0;
                    state.current = 0;
                    state.required = initial_task_states_by_id[task_id].required;
                    TaskStatusKindDto::Failed
                }
                Some(TaskResolution::Abandoned) => {
                    state.stage_index = 0;
                    state.current = 0;
                    state.required = initial_task_states_by_id[task_id].required;
                    TaskStatusKindDto::Abandoned
                }
                None => TaskStatusKindDto::Paused,
            };
        }
        if let Some(target) = procedural_floors
            .iter()
            .find(|floor| floor.id == target_floor_id && floor.lifecycle == FloorLifecycle::OneShot)
        {
            let state = self
                .task_states
                .get_mut(floor_task_id(target))
                .expect("target task state must remain available");
            if task_resumed {
                state.retakes_used = state.retakes_used.saturating_add(1);
            }
            state.status = TaskStatusKindDto::Active;
            state.active_floor_id = Some(target.id.clone());
        }
        self.activate_floor(destination, global_items);
        Ok(Some(FloorTransitionOutcome {
            from_floor_id,
            to_floor_id: target_floor_id.clone(),
            expedition_ended,
            one_shot_closed: one_shot_source.as_ref().and_then(|floor| {
                task_resolution.map(|resolution| (floor_task_id(floor).to_owned(), resolution))
            }),
            task_paused: one_shot_source
                .filter(|floor| task_resolution.is_none() && floor.retakeable)
                .map(|floor| floor_task_id(&floor).to_owned()),
            task_resumed: task_resumed.then(|| {
                procedural_floors
                    .iter()
                    .find(|floor| floor.id == target_floor_id)
                    .map(floor_task_id)
                    .unwrap_or(&target_floor_id)
                    .to_owned()
            }),
        }))
    }

    fn activate_floor(&mut self, floor: FloorState, mut global_items: Vec<ItemInstance>) {
        self.current_floor_id = floor.id;
        self.current_dungeon_instance_id = floor.dungeon_instance_id;
        self.width = floor.width;
        self.height = floor.height;
        self.terrain = floor.terrain;
        self.player.position = floor.player_position;
        self.entities = floor.entities;
        global_items.extend(floor.items);
        self.items = global_items;
        self.explored = floor.explored;
        self.revealed_terrain = floor.revealed_terrain;
        self.floor_connections = floor.connections;
        self.floor_regions = floor.regions;
        self.reveal_current_visibility();
    }

    fn generate_procedural_floor(
        &mut self,
        definition: &ProceduralFloorDefinition,
        dungeon_instance_id: Option<String>,
    ) -> Result<FloorState, CoreError> {
        let maze_only = definition
            .layout
            .as_ref()
            .is_some_and(|layout| layout.mode == ProceduralLayoutMode::MazeOnly);
        let selected_region_entries = if let Some(table_id) = &definition.region_table_id {
            let table = self
                .content
                .region_table(table_id)
                .expect("validated region table must remain available")
                .clone();
            let mut eligible = table
                .entries
                .into_iter()
                .filter(|entry| {
                    entry.min_depth <= definition.depth && definition.depth <= entry.max_depth
                })
                .collect::<Vec<_>>();
            let placement_count = definition
                .generation_budget
                .as_ref()
                .and_then(|budget| budget.region_placements)
                .expect("validated region floor must retain a placement budget");
            let mut selected = Vec::with_capacity(usize::from(placement_count));
            for _ in 0..placement_count {
                let weights = eligible
                    .iter()
                    .map(|entry| entry.weight)
                    .collect::<Vec<_>>();
                let selected_index = self.roll_weighted_index(&weights);
                selected.push(eligible.remove(selected_index));
            }
            selected
        } else {
            Vec::new()
        };
        let eligible_themes = definition
            .theme_table_id
            .as_ref()
            .and_then(|table_id| self.content.theme_table(table_id))
            .map(|table| {
                table
                    .entries
                    .iter()
                    .filter(|entry| {
                        entry.min_depth <= definition.depth && definition.depth <= entry.max_depth
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let selected_theme = if eligible_themes.is_empty() {
            None
        } else if eligible_themes.len() == 1 {
            Some(eligible_themes[0].clone())
        } else {
            let weights = eligible_themes
                .iter()
                .map(|entry| entry.weight)
                .collect::<Vec<_>>();
            Some(eligible_themes[self.roll_weighted_index(&weights)].clone())
        };
        let generated_floor_terrain_id = selected_theme
            .as_ref()
            .map(|entry| entry.floor_terrain_id.clone())
            .unwrap_or_else(|| definition.floor_terrain_id.clone());
        let uses_spatial_vault_budget =
            definition.generation_budget.as_ref().is_some_and(|budget| {
                budget.vault_placements.is_some() && budget.vault_area_tiles.is_some()
            });
        let eligible_vault_candidates = selected_theme
            .as_ref()
            .map(|theme| {
                theme
                    .vault_candidates
                    .iter()
                    .filter(|candidate| {
                        candidate.min_depth <= definition.depth
                            && definition.depth <= candidate.max_depth
                            && self
                                .content
                                .vault(&candidate.vault_id)
                                .is_some_and(|vault| {
                                    uses_spatial_vault_budget
                                        || vault.width <= 6
                                            && vault.height <= 5
                                            && vault.entrance_positions.len() == 1
                                            && vault.entrance_positions[0].x == vault.width / 2
                                            && vault.entrance_positions[0].y == 0
                                })
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let legacy_vault = if uses_spatial_vault_budget || maze_only {
            None
        } else if eligible_vault_candidates.is_empty() {
            definition
                .vault_id
                .as_ref()
                .and_then(|vault_id| self.content.vault(vault_id))
                .cloned()
        } else if eligible_vault_candidates.len() == 1 {
            self.content
                .vault(&eligible_vault_candidates[0].vault_id)
                .cloned()
        } else {
            let weights = eligible_vault_candidates
                .iter()
                .map(|candidate| candidate.weight)
                .collect::<Vec<_>>();
            let vault_id = &eligible_vault_candidates[self.roll_weighted_index(&weights)].vault_id;
            self.content.vault(vault_id).cloned()
        };
        let guardian = definition.guardian.as_ref().filter(|_| {
            definition.dungeon_id.as_ref().is_some_and(|dungeon_id| {
                self.dungeon_states
                    .get(dungeon_id)
                    .is_some_and(|state| !state.guardian_defeated)
            })
        });
        let task_objectives = self
            .content
            .world(&self.world_id)
            .and_then(|world| {
                world
                    .procedural_floors
                    .iter()
                    .find(|floor| {
                        floor_task_id(floor) == floor_task_id(definition)
                            && !floor.task_stages.is_empty()
                    })
                    .map(|floor| {
                        floor
                            .task_stages
                            .iter()
                            .filter(|stage| {
                                stage.floor_id.as_deref() == Some(definition.id.as_str())
                            })
                            .cloned()
                            .collect::<Vec<_>>()
                    })
            })
            .unwrap_or_else(|| definition.task_objective.iter().cloned().collect());
        let width = definition.width;
        let height = definition.height;
        let mut terrain =
            vec![definition.wall_terrain_id.clone(); usize::from(width) * usize::from(height)];
        let cavern_origin = definition.layout.as_ref().and_then(|layout| {
            layout.cavern.as_ref().map(|cavern| {
                self.generate_connected_cavern(definition, &cavern.terrain_id, &mut terrain)
            })
        });
        let lake_origin = definition.layout.as_ref().and_then(|layout| {
            layout.lake.as_ref().map(|lake| {
                self.generate_connected_lake(
                    definition,
                    &lake.deep_terrain_id,
                    &lake.shallow_terrain_id,
                    &mut terrain,
                )
            })
        });
        let maze_walkable = if maze_only {
            let maze = definition
                .layout
                .as_ref()
                .and_then(|layout| layout.maze.as_ref())
                .expect("validated maze-only layout must retain maze geometry");
            self.generate_maze(definition, maze, &generated_floor_terrain_id, &mut terrain)
        } else {
            BTreeSet::new()
        };
        let rooms = if maze_only {
            Vec::new()
        } else if let Some(layout) = &definition.layout {
            self.generate_budgeted_rooms(
                definition,
                layout
                    .rooms
                    .as_ref()
                    .expect("validated rooms layout must retain room geometry"),
            )
        } else {
            let room_width = 6_i32;
            let room_height = 5_i32;
            let first_x = 1 + i32::try_from(self.rng.bounded(3)).unwrap_or(0);
            let first_y = 1 + i32::try_from(self.rng.bounded(4)).unwrap_or(0);
            let second_x = 11 + i32::try_from(self.rng.bounded(3)).unwrap_or(0);
            let second_y = 11 + i32::try_from(self.rng.bounded(3)).unwrap_or(0);
            vec![
                GeneratedRoom {
                    id: "entry".to_owned(),
                    x: first_x,
                    y: first_y,
                    width: room_width,
                    height: room_height,
                    shape: ProceduralRoomShape::Rectangle,
                },
                GeneratedRoom {
                    id: "remote".to_owned(),
                    x: second_x,
                    y: second_y,
                    width: room_width,
                    height: room_height,
                    shape: ProceduralRoomShape::Rectangle,
                },
            ]
        };
        let content_rooms = if definition
            .layout
            .as_ref()
            .is_some_and(|layout| layout.pit.is_some())
        {
            &rooms[..rooms.len() - 1]
        } else {
            rooms.as_slice()
        };
        let room_region_indexes =
            assign_generated_rooms_to_regions(content_rooms, selected_region_entries.len());
        let mut generated_regions = selected_region_entries
            .iter()
            .enumerate()
            .map(|(region_index, entry)| {
                let theme = self
                    .content
                    .theme_table(&entry.theme_table_id)
                    .and_then(|table| {
                        table
                            .entries
                            .iter()
                            .find(|theme| theme.theme_id == entry.theme_id)
                    })
                    .expect("validated region theme must remain available");
                let room_ids = content_rooms
                    .iter()
                    .zip(&room_region_indexes)
                    .filter(|(_, assigned_region)| **assigned_region == region_index)
                    .map(|(room, _)| room.id.clone())
                    .collect::<Vec<_>>();
                let mut cells = content_rooms
                    .iter()
                    .zip(&room_region_indexes)
                    .filter(|(_, assigned_region)| **assigned_region == region_index)
                    .flat_map(|(room, _)| generated_room_cells(room))
                    .collect::<Vec<_>>();
                cells.sort();
                GeneratedRegion {
                    state: FloorRegionState {
                        region_id: entry.region_id.clone(),
                        theme_id: entry.theme_id.clone(),
                        encounter_table_id: entry.encounter_table_id.clone(),
                        loot_table_id: entry.loot_table_id.clone(),
                        cells,
                    },
                    room_ids,
                    floor_terrain_id: theme.floor_terrain_id.clone(),
                }
            })
            .collect::<Vec<_>>();
        for (room_index, room) in rooms.iter().enumerate() {
            let room_terrain_id = room_region_indexes
                .get(room_index)
                .and_then(|region_index| generated_regions.get(*region_index))
                .map_or(generated_floor_terrain_id.as_str(), |region| {
                    region.floor_terrain_id.as_str()
                });
            carve_generated_room(&mut terrain, width, room, room_terrain_id);
        }
        let (first_center, second_center) = if maze_only {
            maze_floor_anchors(&maze_walkable)
        } else {
            (rooms[0].center(), rooms[1].center())
        };
        let legacy_vault_origin = legacy_vault.as_ref().map(|vault| Position {
            x: second_center.x - i32::from(vault.entrance_positions[0].x),
            y: rooms
                .get(1)
                .expect("legacy vault placement requires a remote room")
                .y,
        });
        if let Some(destroyed) = definition
            .layout
            .as_ref()
            .and_then(|layout| layout.destroyed.as_ref())
        {
            self.generate_destroyed_region(definition, &destroyed.terrain_id, &mut terrain);
        }
        if let Some(river) = definition
            .layout
            .as_ref()
            .and_then(|layout| layout.river.as_ref())
        {
            self.generate_river(
                definition,
                &river.deep_terrain_id,
                &river.shallow_terrain_id,
                lake_origin.unwrap_or(Position {
                    x: i32::from(width / 2),
                    y: i32::from(height / 2),
                }),
                &mut terrain,
            );
        }
        if definition
            .layout
            .as_ref()
            .is_some_and(|layout| layout.destroyed.is_some() || layout.river.is_some())
        {
            for room in &rooms {
                let room_index = rooms
                    .iter()
                    .position(|candidate| candidate.id == room.id)
                    .expect("generated room must retain its stable index");
                let room_terrain_id = room_region_indexes
                    .get(room_index)
                    .and_then(|region_index| generated_regions.get(*region_index))
                    .map_or(generated_floor_terrain_id.as_str(), |region| {
                        region.floor_terrain_id.as_str()
                    });
                carve_generated_room(&mut terrain, width, room, room_terrain_id);
            }
        }
        for connected_rooms in rooms.windows(2) {
            carve_generated_corridor(
                &mut terrain,
                width,
                connected_rooms[0].center(),
                connected_rooms[1].center(),
                &generated_floor_terrain_id,
            );
        }
        if let Some(cavern_origin) = cavern_origin {
            carve_generated_corridor(
                &mut terrain,
                width,
                first_center,
                cavern_origin,
                &generated_floor_terrain_id,
            );
        }
        if let Some(layout) = &definition.layout
            && !layout.streamers.is_empty()
        {
            self.generate_streamers(definition, &layout.streamers, &mut terrain);
        }
        let pit_placement = definition
            .layout
            .as_ref()
            .and_then(|layout| layout.pit.as_ref())
            .map(|pit| {
                self.place_classic_pit(
                    definition,
                    pit,
                    rooms[rooms.len() - 2].center(),
                    &generated_floor_terrain_id,
                    &mut terrain,
                )
            });
        let door_position = (!maze_only).then_some(Position {
            x: (first_center.x + second_center.x) / 2,
            y: first_center.y,
        });
        if let Some(door_position) = door_position {
            set_generated_terrain(
                &mut terrain,
                width,
                door_position,
                &definition.closed_door_terrain_id,
            );
        }
        let down_stair_position = if maze_only {
            second_center
        } else {
            Position {
                x: first_center.x - 1,
                y: first_center.y,
            }
        };
        let fixed_trap_position = if maze_only {
            let route = maze_floor_path(&maze_walkable, first_center, second_center);
            route[route.len() / 2]
        } else {
            Position {
                x: first_center.x,
                y: first_center.y + 1,
            }
        };
        let mut floor_connections = if definition.connections.is_empty() {
            set_generated_terrain(
                &mut terrain,
                width,
                first_center,
                &definition.up_stair_terrain_id,
            );
            if let Some(down_stair_terrain_id) = &definition.down_stair_terrain_id {
                set_generated_terrain(
                    &mut terrain,
                    width,
                    down_stair_position,
                    down_stair_terrain_id,
                );
            }
            Vec::new()
        } else {
            let (primary_up_id, primary_down_id) = primary_floor_connection_ids(definition);
            for (connection_id, position) in [
                (primary_up_id, first_center),
                (primary_down_id, down_stair_position),
            ] {
                if let Some(connection) = connection_id.and_then(|connection_id| {
                    definition
                        .connections
                        .iter()
                        .find(|connection| connection.id == connection_id)
                }) {
                    set_generated_terrain(&mut terrain, width, position, &connection.terrain_id);
                }
            }
            Vec::new()
        };
        set_generated_terrain(
            &mut terrain,
            width,
            fixed_trap_position,
            &definition.trap_terrain_id,
        );
        let vault_placements = if let Some(vault) = legacy_vault.clone() {
            let placement = GeneratedVaultPlacement {
                vault,
                origin: legacy_vault_origin.expect("present vault must have an origin"),
                transform: VaultTransform::Identity,
                ordinal: 1,
                connector_cells: Vec::new(),
            };
            paint_generated_vault(&mut terrain, width, &placement);
            vec![placement]
        } else if uses_spatial_vault_budget {
            self.select_spatial_vault_placements(
                definition,
                &eligible_vault_candidates,
                guardian.is_some(),
                &generated_floor_terrain_id,
                &mut terrain,
            )
        } else {
            Vec::new()
        };
        for placement in &vault_placements {
            let entrance = transformed_vault_position(
                &placement.vault,
                placement.transform,
                placement.vault.entrance_positions[0],
            );
            let anchor = Position {
                x: placement.origin.x + entrance.x,
                y: placement.origin.y + entrance.y,
            };
            let (vault_width, vault_height) =
                transformed_vault_dimensions(&placement.vault, placement.transform);
            let footprint = (0..vault_height).flat_map(|y| {
                (0..vault_width).map(move |x| Position {
                    x: placement.origin.x + i32::from(x),
                    y: placement.origin.y + i32::from(y),
                })
            });
            assign_generated_footprint_to_region(
                &mut generated_regions,
                content_rooms,
                anchor,
                footprint,
            );
        }
        if let Some(pit) = &pit_placement {
            let total_width = pit.definition.inner_width + 6;
            let total_height = pit.definition.inner_height + 6;
            let footprint = (0..total_height).flat_map(|y| {
                (0..total_width).map(move |x| Position {
                    x: pit.origin.x + i32::from(x),
                    y: pit.origin.y + i32::from(y),
                })
            });
            assign_generated_footprint_to_region(
                &mut generated_regions,
                content_rooms,
                pit.outer_entrance,
                footprint,
            );
        }
        if !definition.connections.is_empty() {
            floor_connections = place_generated_floor_connections(
                definition,
                first_center,
                down_stair_position,
                fixed_trap_position,
                &generated_floor_terrain_id,
                &mut terrain,
                &mut self.rng,
            )?;
        }
        let mut feature_reserved = BTreeSet::from([fixed_trap_position]);
        if floor_connections.is_empty() {
            feature_reserved.insert(first_center);
        } else {
            feature_reserved.extend(
                floor_connections
                    .iter()
                    .map(|connection| connection.position),
            );
        }
        if let Some(door_position) = door_position {
            feature_reserved.insert(door_position);
        }
        if floor_connections.is_empty() && definition.down_stair_terrain_id.is_some() {
            feature_reserved.insert(down_stair_position);
        }
        for placement in &vault_placements {
            let (vault_width, vault_height) =
                transformed_vault_dimensions(&placement.vault, placement.transform);
            for y in 0..vault_height {
                for x in 0..vault_width {
                    feature_reserved.insert(Position {
                        x: placement.origin.x + i32::from(x),
                        y: placement.origin.y + i32::from(y),
                    });
                }
            }
            feature_reserved.extend(placement.connector_cells.iter().copied());
        }
        if let Some(pit) = &pit_placement {
            let total_width = pit.definition.inner_width + 6;
            let total_height = pit.definition.inner_height + 6;
            for y in 0..total_height {
                for x in 0..total_width {
                    feature_reserved.insert(Position {
                        x: pit.origin.x + i32::from(x),
                        y: pit.origin.y + i32::from(y),
                    });
                }
            }
            feature_reserved.insert(pit.outer_entrance);
            feature_reserved.insert(pit.inner_entrance);
        }
        let room_floor_terrain_ids = generated_regions
            .iter()
            .map(|region| region.floor_terrain_id.clone())
            .collect::<BTreeSet<_>>();
        let terrain_features = if let Some(table_id) = &definition.terrain_feature_table_id {
            let table = self
                .content
                .terrain_feature_table(table_id)
                .expect("validated terrain feature table must remain available")
                .clone();
            let eligible_entries = table
                .entries
                .iter()
                .filter(|entry| {
                    entry.min_depth <= definition.depth && definition.depth <= entry.max_depth
                })
                .cloned()
                .collect::<Vec<_>>();
            self.place_terrain_features(
                definition,
                &eligible_entries,
                TerrainFeaturePlacementContext {
                    rooms: content_rooms,
                    reserved: &feature_reserved,
                    floor_terrain_id: &generated_floor_terrain_id,
                    room_floor_terrain_ids: &room_floor_terrain_ids,
                },
                &mut terrain,
            )
        } else {
            Vec::new()
        };
        let mut occupied = BTreeSet::from([first_center]);
        occupied.extend(
            floor_connections
                .iter()
                .map(|connection| connection.position),
        );
        if maze_only {
            occupied.insert(fixed_trap_position);
        }
        occupied.extend(terrain_features.iter().map(|feature| feature.position));
        if let Some(pit) = &pit_placement {
            let total_width = pit.definition.inner_width + 6;
            let total_height = pit.definition.inner_height + 6;
            for y in 0..total_height {
                for x in 0..total_width {
                    occupied.insert(Position {
                        x: pit.origin.x + i32::from(x),
                        y: pit.origin.y + i32::from(y),
                    });
                }
            }
        }
        for placement in &vault_placements {
            occupied.extend(
                placement
                    .vault
                    .encounter_groups
                    .iter()
                    .flat_map(|group| &group.member_positions)
                    .map(|local| {
                        let local = transformed_vault_position(
                            &placement.vault,
                            placement.transform,
                            *local,
                        );
                        Position {
                            x: placement.origin.x + local.x,
                            y: placement.origin.y + local.y,
                        }
                    }),
            );
            occupied.extend(placement.vault.loot_spawns.iter().map(|spawn| {
                let local = transformed_vault_position(
                    &placement.vault,
                    placement.transform,
                    spawn.position,
                );
                Position {
                    x: placement.origin.x + local.x,
                    y: placement.origin.y + local.y,
                }
            }));
        }
        if floor_connections.is_empty() && definition.down_stair_terrain_id.is_some() {
            occupied.insert(down_stair_position);
        }
        let guardian_position = guardian.map(|_| Position {
            x: first_center.x + 1,
            y: first_center.y,
        });
        occupied.extend(guardian_position);
        let reserved_actor_slots = definition
            .generation_budget
            .as_ref()
            .and_then(|budget| budget.pit_actor_slots)
            .unwrap_or(0)
            .saturating_add(definition.nest.as_ref().map_or(0, |nest| nest.spawn_count))
            .saturating_add(if guardian.is_some() { 1 } else { 0 })
            .saturating_add(
                vault_placements
                    .iter()
                    .flat_map(|placement| &placement.vault.encounter_groups)
                    .map(|group| {
                        u16::try_from(group.member_positions.len())
                            .expect("validated vault group size must fit u16")
                    })
                    .sum::<u16>(),
            );
        let mut entities = Vec::new();
        let mut regional_loot_allocations = Vec::new();
        if !generated_regions.is_empty() {
            let budget = definition
                .generation_budget
                .as_ref()
                .expect("validated region floor must retain a generation budget");
            let region_count = u16::try_from(generated_regions.len())
                .expect("validated region count must fit u16");
            if budget.group_placements.is_some() && budget.group_actor_slots.is_some() {
                let host = &generated_regions[0];
                let table = self
                    .content
                    .encounter_table(&host.state.encounter_table_id)
                    .expect("validated regional group table must remain available")
                    .clone();
                let eligible_entries = table
                    .entries
                    .iter()
                    .filter(|entry| {
                        entry.min_depth <= definition.depth
                            && definition.depth <= entry.max_depth
                            && self
                                .content
                                .actor(&entry.actor_kind_id)
                                .is_some_and(|actor| actor.level <= u32::from(definition.depth))
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let room_id = &host.room_ids[0];
                let id_prefix = format!("{}.region.{}", definition.id, host.state.region_id);
                entities.extend(self.generate_dynamic_encounter_groups(
                    definition,
                    &table,
                    &eligible_entries,
                    content_rooms,
                    room_id,
                    reserved_actor_slots,
                    region_count,
                    false,
                    &id_prefix,
                    &mut occupied,
                ));
            }
            let actor_budget = budget
                .actor_slots
                .saturating_sub(reserved_actor_slots)
                .saturating_sub(
                    u16::try_from(entities.len())
                        .expect("generated regional group size must fit u16"),
                );
            let loot_budget = budget.loot_placements.saturating_sub(
                vault_placements
                    .iter()
                    .map(|placement| {
                        u16::try_from(placement.vault.loot_spawns.len())
                            .expect("validated vault loot count must fit u16")
                    })
                    .sum::<u16>(),
            );
            let (regional_actor_allocations, loot_allocations) =
                allocate_generated_region_placements(
                    &generated_regions,
                    &terrain,
                    width,
                    &self.content,
                    &occupied,
                    actor_budget,
                    loot_budget,
                );
            regional_loot_allocations = loot_allocations;
            for (region_index, region) in generated_regions.iter().enumerate() {
                let placements = regional_actor_allocations[region_index];
                let table = self
                    .content
                    .encounter_table(&region.state.encounter_table_id)
                    .expect("validated region encounter table must remain available")
                    .clone();
                let eligible_entries = table
                    .entries
                    .iter()
                    .filter(|entry| {
                        entry.group.is_none()
                            && entry.min_depth <= definition.depth
                            && definition.depth <= entry.max_depth
                            && self
                                .content
                                .actor(&entry.actor_kind_id)
                                .is_some_and(|actor| actor.level <= u32::from(definition.depth))
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let weights = eligible_entries
                    .iter()
                    .map(|entry| entry.weight)
                    .collect::<Vec<_>>();
                for ordinal in 0..placements {
                    let entry = &eligible_entries[self.roll_weighted_index(&weights)];
                    let position =
                        self.choose_generated_region_position(region, &terrain, width, &occupied);
                    occupied.insert(position);
                    entities.push(self.generated_actor(
                        format!(
                            "{}.region.{}.encounter.plain.{}",
                            definition.id,
                            region.state.region_id,
                            ordinal + 1
                        ),
                        &entry.actor_kind_id,
                        position,
                    ));
                }
            }
        } else if let Some(table_id) = &definition.encounter_table_id {
            let table = self
                .content
                .encounter_table(table_id)
                .expect("validated floor encounter table must remain available")
                .clone();
            let eligible_entries = table
                .entries
                .iter()
                .filter(|entry| {
                    entry.min_depth <= definition.depth
                        && definition.depth <= entry.max_depth
                        && self
                            .content
                            .actor(&entry.actor_kind_id)
                            .is_some_and(|actor| actor.level <= u32::from(definition.depth))
                })
                .cloned()
                .collect::<Vec<_>>();
            let weights = eligible_entries
                .iter()
                .map(|entry| entry.weight)
                .collect::<Vec<_>>();
            let room_id = if legacy_vault.is_some() {
                "entry"
            } else {
                "remote"
            };
            if definition.generation_budget.as_ref().is_some_and(|budget| {
                budget.group_placements.is_some() && budget.group_actor_slots.is_some()
            }) {
                entities.extend(self.generate_dynamic_encounter_groups(
                    definition,
                    &table,
                    &eligible_entries,
                    content_rooms,
                    room_id,
                    reserved_actor_slots,
                    1,
                    true,
                    &definition.id,
                    &mut occupied,
                ));
            } else {
                let encounter_rolls =
                    definition
                        .generation_budget
                        .as_ref()
                        .map_or(table.rolls, |budget| {
                            table
                                .rolls
                                .min(budget.actor_slots.saturating_sub(reserved_actor_slots))
                        });
                for ordinal in 0..encounter_rolls {
                    let entry = &eligible_entries[self.roll_weighted_index(&weights)];
                    let placement_room_id = if maze_only {
                        "maze"
                    } else if definition.layout.is_some() {
                        generated_non_entry_room_id(content_rooms, ordinal)
                    } else {
                        room_id
                    };
                    let position = if maze_only {
                        choose_generated_maze_position(&maze_walkable, first_center, &occupied)
                    } else {
                        self.choose_generated_room_position(
                            content_rooms,
                            placement_room_id,
                            &occupied,
                        )
                    };
                    occupied.insert(position);
                    entities.push(self.generated_actor(
                        format!("{}.encounter.{}", definition.id, ordinal + 1),
                        &entry.actor_kind_id,
                        position,
                    ));
                }
            }
            if let Some(nest) = &definition.nest {
                let entry = &eligible_entries[self.roll_weighted_index(&weights)];
                for ordinal in 0..nest.spawn_count {
                    let position =
                        self.choose_generated_room_position(&rooms, &nest.room_id, &occupied);
                    occupied.insert(position);
                    let actor = self
                        .content
                        .actor(&entry.actor_kind_id)
                        .expect("validated nest actor must remain available");
                    entities.push(actor_from_spawn(
                        &format!("{}.nest.{}", definition.id, ordinal + 1),
                        &entry.actor_kind_id,
                        ContentPosition {
                            x: u16::try_from(position.x).expect("nest actor x must fit u16"),
                            y: u16::try_from(position.y).expect("nest actor y must fit u16"),
                        },
                        actor.max_hp,
                        actor.speed,
                        INITIAL_MONSTER_ENERGY_NEED,
                    ));
                }
            }
        } else {
            for spawn in &definition.actor_spawns {
                let eligible_kind_ids = spawn
                    .actor_kind_ids
                    .iter()
                    .filter(|kind_id| {
                        self.content
                            .actor(kind_id)
                            .is_some_and(|actor| actor.level <= u32::from(definition.depth))
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let kind_index = usize::try_from(
                    self.rng.bounded(
                        u64::try_from(eligible_kind_ids.len())
                            .expect("validated actor candidate count must fit u64"),
                    ),
                )
                .expect("bounded actor candidate index must fit usize");
                let kind_id = &eligible_kind_ids[kind_index];
                let position =
                    self.choose_generated_room_position(&rooms, &spawn.room_id, &occupied);
                occupied.insert(position);
                let actor = self
                    .content
                    .actor(kind_id)
                    .expect("validated procedural actor kind must remain available");
                entities.push(actor_from_spawn(
                    &spawn.instance_id,
                    kind_id,
                    ContentPosition {
                        x: u16::try_from(position.x).expect("generated actor x must fit u16"),
                        y: u16::try_from(position.y).expect("generated actor y must fit u16"),
                    },
                    actor.max_hp,
                    actor.speed,
                    INITIAL_MONSTER_ENERGY_NEED,
                ));
            }
        }
        if let Some(pit) = &pit_placement {
            entities.extend(self.generate_classic_pit_actors(definition, pit, &mut occupied));
        }
        for placement in &vault_placements {
            for group in &placement.vault.encounter_groups {
                let eligible_entries = group
                    .entries
                    .iter()
                    .filter(|entry| {
                        entry.min_depth <= definition.depth
                            && definition.depth <= entry.max_depth
                            && self
                                .content
                                .actor(&entry.actor_kind_id)
                                .is_some_and(|actor| actor.level <= u32::from(definition.depth))
                    })
                    .collect::<Vec<_>>();
                let weights = eligible_entries
                    .iter()
                    .map(|entry| entry.weight)
                    .collect::<Vec<_>>();
                for (ordinal, local) in group.member_positions.iter().enumerate() {
                    let entry = eligible_entries[self.roll_weighted_index(&weights)];
                    let actor = self
                        .content
                        .actor(&entry.actor_kind_id)
                        .expect("validated vault encounter actor must remain available");
                    let local =
                        transformed_vault_position(&placement.vault, placement.transform, *local);
                    let position = Position {
                        x: placement.origin.x + local.x,
                        y: placement.origin.y + local.y,
                    };
                    occupied.insert(position);
                    let instance_id = if uses_spatial_vault_budget {
                        format!(
                            "{}.vault.{}.{}.{}",
                            definition.id,
                            placement.ordinal,
                            group.id,
                            ordinal + 1
                        )
                    } else {
                        format!("{}.{}.{}", definition.id, group.id, ordinal + 1)
                    };
                    entities.push(actor_from_spawn(
                        &instance_id,
                        &entry.actor_kind_id,
                        ContentPosition {
                            x: u16::try_from(position.x).expect("vault actor x must fit u16"),
                            y: u16::try_from(position.y).expect("vault actor y must fit u16"),
                        },
                        actor.max_hp,
                        actor.speed,
                        INITIAL_MONSTER_ENERGY_NEED,
                    ));
                }
            }
        }
        if let Some(guardian) = guardian {
            let actor = self
                .content
                .actor(&guardian.actor_kind_id)
                .expect("validated dungeon guardian must remain available");
            let max_hp = actor.max_hp;
            let speed = actor.speed;
            let position = guardian_position.expect("present guardian must retain a position");
            entities.push(actor_from_spawn(
                &guardian.instance_id,
                &guardian.actor_kind_id,
                ContentPosition {
                    x: u16::try_from(position.x).expect("guardian x must fit u16"),
                    y: u16::try_from(position.y).expect("guardian y must fit u16"),
                },
                max_hp,
                speed,
                INITIAL_MONSTER_ENERGY_NEED,
            ));
        }
        let mut items =
            self.generate_carried_loot_for_actors(&entities, &definition.id, definition.depth)?;
        if !generated_regions.is_empty() {
            for (region_index, region) in generated_regions.iter().enumerate() {
                let placements = regional_loot_allocations[region_index];
                for ordinal in 0..placements {
                    let room_id = &region.room_ids[usize::from(ordinal) % region.room_ids.len()];
                    let position =
                        self.choose_generated_region_position(region, &terrain, width, &occupied);
                    occupied.insert(position);
                    items.extend(self.generate_loot_instances(
                        &LootContext {
                            table_id: region.state.loot_table_id.clone(),
                            floor_id: definition.id.clone(),
                            depth: definition.depth,
                            source: LootSource::FloorRoom {
                                room_id: room_id.clone(),
                                spawn_id: format!(
                                    "{}.region.{}.loot.{}",
                                    definition.id,
                                    region.state.region_id,
                                    ordinal + 1
                                ),
                            },
                        },
                        ItemLocation::Ground(position),
                    )?);
                }
            }
        } else if let Some(table_id) = &definition.loot_table_id {
            let room_id = if legacy_vault.is_some() {
                "entry"
            } else {
                "remote"
            };
            let floor_loot_placements = definition.generation_budget.as_ref().map_or(1, |budget| {
                budget.loot_placements.saturating_sub(
                    vault_placements
                        .iter()
                        .map(|placement| {
                            u16::try_from(placement.vault.loot_spawns.len())
                                .expect("validated vault loot count must fit u16")
                        })
                        .sum::<u16>(),
                )
            });
            for ordinal in 0..floor_loot_placements {
                let placement_room_id = if maze_only {
                    "maze"
                } else if definition.layout.is_some() {
                    generated_non_entry_room_id(content_rooms, ordinal)
                } else {
                    room_id
                };
                let position = if maze_only {
                    choose_generated_maze_position(&maze_walkable, first_center, &occupied)
                } else {
                    self.choose_generated_room_position(content_rooms, placement_room_id, &occupied)
                };
                occupied.insert(position);
                items.extend(self.generate_loot_instances(
                    &LootContext {
                        table_id: table_id.clone(),
                        floor_id: definition.id.clone(),
                        depth: definition.depth,
                        source: LootSource::FloorRoom {
                            room_id: placement_room_id.to_owned(),
                            spawn_id: format!("{}.loot-table.{}", definition.id, ordinal + 1),
                        },
                    },
                    ItemLocation::Ground(position),
                )?);
            }
        } else {
            for spawn in &definition.loot_spawns {
                let position =
                    self.choose_generated_room_position(&rooms, &spawn.room_id, &occupied);
                occupied.insert(position);
                items.extend(self.generate_loot_instances(
                    &LootContext {
                        table_id: spawn.loot_table_id.clone(),
                        floor_id: definition.id.clone(),
                        depth: definition.depth,
                        source: LootSource::FloorRoom {
                            room_id: spawn.room_id.clone(),
                            spawn_id: spawn.id.clone(),
                        },
                    },
                    ItemLocation::Ground(position),
                )?);
            }
        }
        for placement in &vault_placements {
            for spawn in &placement.vault.loot_spawns {
                let local = transformed_vault_position(
                    &placement.vault,
                    placement.transform,
                    spawn.position,
                );
                let position = Position {
                    x: placement.origin.x + local.x,
                    y: placement.origin.y + local.y,
                };
                occupied.insert(position);
                items.extend(self.generate_loot_instances(
                    &LootContext {
                        table_id: spawn.loot_table_id.clone(),
                        floor_id: definition.id.clone(),
                        depth: definition.depth,
                        source: LootSource::Vault {
                            vault_id: placement.vault.id.clone(),
                            spawn_id: spawn.id.clone(),
                        },
                    },
                    ItemLocation::Ground(position),
                )?);
            }
        }
        for objective in &task_objectives {
            match objective.kind {
                TaskObjectiveKind::CollectItem => items.push(ItemInstance {
                    id: objective
                        .item_instance_id
                        .clone()
                        .expect("validated item objective must have an instance ID"),
                    kind_id: objective
                        .item_kind_id
                        .clone()
                        .expect("validated item objective must have a kind ID"),
                    quantity: 1,
                    quality: ItemQualityDto::Ordinary,
                    affix_ids: Vec::new(),
                    location: ItemLocation::Ground(first_center),
                }),
                TaskObjectiveKind::KillActor => {
                    let kind_id = objective
                        .actor_kind_id
                        .as_ref()
                        .expect("validated kill objective must have a kind ID");
                    let actor = self
                        .content
                        .actor(kind_id)
                        .expect("validated objective actor must remain available");
                    entities.push(actor_from_spawn(
                        objective
                            .actor_instance_id
                            .as_ref()
                            .expect("validated kill objective must have an instance ID"),
                        kind_id,
                        ContentPosition {
                            x: u16::try_from(first_center.x + 1).expect("objective x must fit u16"),
                            y: u16::try_from(first_center.y).expect("objective y must fit u16"),
                        },
                        actor.max_hp,
                        actor.speed,
                        INITIAL_MONSTER_ENERGY_NEED,
                    ));
                }
                TaskObjectiveKind::KillActorKind => {
                    let kind_id = objective
                        .actor_kind_id
                        .as_ref()
                        .expect("validated counted kill objective must have a kind ID");
                    let actor = self
                        .content
                        .actor(kind_id)
                        .expect("validated objective actor must remain available");
                    let remaining = self
                        .task_states
                        .get(floor_task_id(definition))
                        .map_or(objective.required, |state| {
                            state.required.saturating_sub(state.current)
                        });
                    let spawn_count = objective
                        .spawn_count
                        .unwrap_or(objective.required)
                        .min(remaining);
                    for ordinal in 0..spawn_count {
                        entities.push(actor_from_spawn(
                            &format!("{}.task-target.{}", definition.id, ordinal + 1),
                            kind_id,
                            ContentPosition {
                                x: u16::try_from(
                                    first_center.x + 1 + i32::try_from(ordinal).unwrap_or(i32::MAX),
                                )
                                .expect("objective x must fit u16"),
                                y: u16::try_from(first_center.y).expect("objective y must fit u16"),
                            },
                            actor.max_hp,
                            actor.speed,
                            INITIAL_MONSTER_ENERGY_NEED,
                        ));
                    }
                }
                TaskObjectiveKind::EnterFloor => {}
            }
        }
        for region in &mut generated_regions {
            region.state.cells.sort();
            region.state.cells.dedup();
        }
        generated_regions.sort_by(|left, right| left.state.region_id.cmp(&right.state.region_id));
        self.resolve_floor_connection_targets(definition, &mut floor_connections)?;
        Ok(FloorState {
            id: definition.id.clone(),
            dungeon_instance_id,
            width,
            height,
            terrain,
            player_position: first_center,
            entities,
            items,
            explored: vec![false; usize::from(width) * usize::from(height)],
            revealed_terrain: BTreeSet::new(),
            connections: floor_connections,
            regions: generated_regions
                .into_iter()
                .map(|region| region.state)
                .collect(),
        })
    }

    fn resolve_floor_connection_targets(
        &mut self,
        definition: &ProceduralFloorDefinition,
        connections: &mut [FloorConnectionState],
    ) -> Result<(), CoreError> {
        let mut selected_dynamic_targets = BTreeSet::new();
        for state in connections {
            let connection = definition
                .connections
                .iter()
                .find(|connection| connection.id == state.id)
                .ok_or(CoreError::InvalidSave(
                    "generated floor connection is missing from content",
                ))?;
            if connection.target_candidates.is_empty() {
                state.target_floor_id = Some(connection.target_floor_id.clone());
                state.target_connection_id = connection.target_connection_id.clone();
                continue;
            }
            let mut eligible = connection
                .target_candidates
                .iter()
                .filter(|candidate| !selected_dynamic_targets.contains(&candidate.target_floor_id))
                .collect::<Vec<_>>();
            if eligible.is_empty() {
                eligible.extend(connection.target_candidates.iter());
            }
            let weights = eligible
                .iter()
                .map(|candidate| u32::from(candidate.weight))
                .collect::<Vec<_>>();
            let selected = eligible[self.roll_weighted_index(&weights)];
            state.target_floor_id = Some(selected.target_floor_id.clone());
            state.target_connection_id = Some(selected.target_connection_id.clone());
            selected_dynamic_targets.insert(selected.target_floor_id.clone());
        }
        Ok(())
    }

    fn generate_budgeted_rooms(
        &mut self,
        definition: &ProceduralFloorDefinition,
        geometry: &ProceduralRoomGeometryDefinition,
    ) -> Vec<GeneratedRoom> {
        let budget = definition
            .generation_budget
            .as_ref()
            .expect("room geometry requires a generation budget");
        let placement_count = budget
            .room_placements
            .expect("validated room placement count must remain available");
        let mut remaining_area = budget
            .room_area_tiles
            .expect("validated room area budget must remain available");
        let columns = if placement_count <= 4 { 2 } else { 3 };
        let rows = placement_count.div_ceil(columns);
        let interior_width = definition.width - 2;
        let interior_height = definition.height - 2;
        let minimum_room_area = geometry
            .shapes
            .iter()
            .map(|candidate| match candidate.shape {
                ProceduralRoomShape::Rectangle => {
                    u32::from(geometry.min_width) * u32::from(geometry.min_height)
                }
                ProceduralRoomShape::Cross => {
                    u32::from(geometry.min_width) + u32::from(geometry.min_height) - 1
                }
            })
            .min()
            .expect("validated room geometry must retain a shape");
        let mut rooms = Vec::with_capacity(usize::from(placement_count));

        for ordinal in 0..placement_count {
            let column = ordinal % columns;
            let row = ordinal / columns;
            let cell_left = 1 + interior_width * column / columns;
            let cell_right = 1 + interior_width * (column + 1) / columns;
            let cell_top = 1 + interior_height * row / rows;
            let cell_bottom = 1 + interior_height * (row + 1) / rows;
            let future_room_count = placement_count - ordinal - 1;
            let maximum_room_area =
                remaining_area - u32::from(future_room_count) * minimum_room_area;
            let mut shape_candidates = Vec::new();

            for shape_candidate in &geometry.shapes {
                let mut candidates = Vec::new();
                for y in cell_top..cell_bottom {
                    for x in cell_left..cell_right {
                        for height in geometry.min_height..=geometry.max_height {
                            for width in geometry.min_width..=geometry.max_width {
                                if x + width > cell_right || y + height > cell_bottom {
                                    continue;
                                }
                                let room = GeneratedRoom {
                                    id: String::new(),
                                    x: i32::from(x),
                                    y: i32::from(y),
                                    width: i32::from(width),
                                    height: i32::from(height),
                                    shape: shape_candidate.shape,
                                };
                                if room.area() <= maximum_room_area {
                                    candidates.push(room);
                                }
                            }
                        }
                    }
                }
                if !candidates.is_empty() {
                    shape_candidates.push((shape_candidate.weight, candidates));
                }
            }
            let shape_index = if shape_candidates.len() == 1 {
                0
            } else {
                let weights = shape_candidates
                    .iter()
                    .map(|(weight, _)| *weight)
                    .collect::<Vec<_>>();
                self.roll_weighted_index(&weights)
            };
            let candidates = &shape_candidates[shape_index].1;
            let candidate_index = if candidates.len() == 1 {
                0
            } else {
                usize::try_from(
                    self.rng.bounded(
                        u64::try_from(candidates.len())
                            .expect("room geometry candidate count must fit u64"),
                    ),
                )
                .expect("room geometry candidate index must fit usize")
            };
            let mut room = candidates[candidate_index].clone();
            room.id = match ordinal {
                0 => "entry".to_owned(),
                1 => "remote".to_owned(),
                _ => format!("room.{}", ordinal + 1),
            };
            remaining_area -= room.area();
            rooms.push(room);
        }

        rooms
    }

    fn place_classic_pit(
        &mut self,
        floor: &ProceduralFloorDefinition,
        pit: &ProceduralPitDefinition,
        approach: Position,
        floor_terrain_id: &str,
        terrain: &mut [String],
    ) -> GeneratedPitPlacement {
        let placement_count = floor
            .generation_budget
            .as_ref()
            .and_then(|budget| budget.room_placements)
            .expect("validated pit requires room placement budget");
        let columns = if placement_count <= 4 { 2 } else { 3 };
        let rows = placement_count.div_ceil(columns);
        let ordinal = placement_count - 1;
        let column = ordinal % columns;
        let row = ordinal / columns;
        let interior_width = floor.width - 2;
        let interior_height = floor.height - 2;
        let cell_left = 1 + interior_width * column / columns;
        let cell_right = 1 + interior_width * (column + 1) / columns;
        let cell_top = 1 + interior_height * row / rows;
        let cell_bottom = 1 + interior_height * (row + 1) / rows;
        let total_width = pit.inner_width + 6;
        let total_height = pit.inner_height + 6;
        let maximum_x = i32::from(floor.width - total_width - 1);
        let maximum_y = i32::from(floor.height - total_height - 1);
        let origin = Position {
            x: ((i32::from(cell_left + cell_right) - i32::from(total_width)) / 2)
                .clamp(1, maximum_x),
            y: ((i32::from(cell_top + cell_bottom) - i32::from(total_height)) / 2)
                .clamp(1, maximum_y),
        };
        let center_y = origin.y + i32::from(total_height / 2);
        let outer_entrance = Position {
            x: origin.x,
            y: center_y,
        };
        let inner_entrance = Position {
            x: origin.x + 2,
            y: center_y,
        };

        for local_y in 0..total_height {
            for local_x in 0..total_width {
                let on_outer_wall = local_x == 0
                    || local_y == 0
                    || local_x + 1 == total_width
                    || local_y + 1 == total_height;
                let on_inner_wall = local_x == 2
                    || local_y == 2
                    || local_x + 3 == total_width
                    || local_y + 3 == total_height;
                let terrain_id = if on_outer_wall || on_inner_wall {
                    &floor.wall_terrain_id
                } else {
                    floor_terrain_id
                };
                set_generated_terrain(
                    terrain,
                    floor.width,
                    Position {
                        x: origin.x + i32::from(local_x),
                        y: origin.y + i32::from(local_y),
                    },
                    terrain_id,
                );
            }
        }
        set_generated_terrain(terrain, floor.width, outer_entrance, floor_terrain_id);
        carve_generated_corridor(
            terrain,
            floor.width,
            approach,
            outer_entrance,
            floor_terrain_id,
        );
        set_generated_terrain(
            terrain,
            floor.width,
            inner_entrance,
            &floor.closed_door_terrain_id,
        );
        GeneratedPitPlacement {
            definition: pit.clone(),
            origin,
            outer_entrance,
            inner_entrance,
        }
    }

    fn generate_connected_cavern(
        &mut self,
        definition: &ProceduralFloorDefinition,
        terrain_id: &str,
        terrain: &mut [String],
    ) -> Position {
        const CARDINAL_OFFSETS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
        let area = definition
            .generation_budget
            .as_ref()
            .and_then(|budget| budget.cavern_area_tiles)
            .expect("validated cavern area budget must remain available");
        let origin = Position {
            x: i32::from(definition.width / 2),
            y: i32::from(definition.height / 2),
        };
        let mut carved = BTreeSet::from([origin]);
        set_generated_terrain(terrain, definition.width, origin, terrain_id);

        while carved.len() < usize::try_from(area).expect("cavern area must fit usize") {
            let mut frontier = carved
                .iter()
                .flat_map(|position| {
                    CARDINAL_OFFSETS.map(|(dx, dy)| Position {
                        x: position.x + dx,
                        y: position.y + dy,
                    })
                })
                .filter(|position| {
                    position.x > 0
                        && position.y > 0
                        && position.x + 1 < i32::from(definition.width)
                        && position.y + 1 < i32::from(definition.height)
                        && !carved.contains(position)
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            frontier.sort_by_key(|position| (position.y, position.x));
            let index = if frontier.len() == 1 {
                0
            } else {
                usize::try_from(self.rng.bounded(
                    u64::try_from(frontier.len()).expect("cavern frontier count must fit u64"),
                ))
                .expect("cavern frontier index must fit usize")
            };
            let position = frontier[index];
            carved.insert(position);
            set_generated_terrain(terrain, definition.width, position, terrain_id);
        }

        origin
    }

    fn generate_connected_lake(
        &mut self,
        definition: &ProceduralFloorDefinition,
        deep_terrain_id: &str,
        shallow_terrain_id: &str,
        terrain: &mut [String],
    ) -> Position {
        const CARDINAL_OFFSETS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
        let budget = definition
            .generation_budget
            .as_ref()
            .expect("lake requires a generation budget");
        let area = usize::try_from(
            budget
                .lake_area_tiles
                .expect("validated lake area budget must remain available"),
        )
        .expect("lake area must fit usize");
        let deep_area = usize::try_from(
            budget
                .lake_deep_area_tiles
                .expect("validated deep lake area budget must remain available"),
        )
        .expect("deep lake area must fit usize");
        let origin = Position {
            x: i32::from(definition.width / 2),
            y: i32::from(definition.height / 2),
        };
        let mut selected = BTreeSet::from([origin]);
        let mut insertion_order = vec![origin];

        while insertion_order.len() < area {
            let mut frontier = selected
                .iter()
                .flat_map(|position| {
                    CARDINAL_OFFSETS.map(|(dx, dy)| Position {
                        x: position.x + dx,
                        y: position.y + dy,
                    })
                })
                .filter(|position| {
                    position.x > 0
                        && position.y > 0
                        && position.x + 1 < i32::from(definition.width)
                        && position.y + 1 < i32::from(definition.height)
                        && !selected.contains(position)
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            frontier.sort_by_key(|position| (position.y, position.x));
            let index = if frontier.len() == 1 {
                0
            } else {
                usize::try_from(self.rng.bounded(
                    u64::try_from(frontier.len()).expect("lake frontier count must fit u64"),
                ))
                .expect("lake frontier index must fit usize")
            };
            let position = frontier[index];
            selected.insert(position);
            insertion_order.push(position);
        }

        for (ordinal, position) in insertion_order.into_iter().enumerate() {
            let terrain_id = if ordinal < deep_area {
                deep_terrain_id
            } else {
                shallow_terrain_id
            };
            set_generated_terrain(terrain, definition.width, position, terrain_id);
        }
        origin
    }

    fn generate_river(
        &mut self,
        definition: &ProceduralFloorDefinition,
        deep_terrain_id: &str,
        shallow_terrain_id: &str,
        target: Position,
        terrain: &mut [String],
    ) {
        const CARDINAL_OFFSETS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
        let area = usize::try_from(
            definition
                .generation_budget
                .as_ref()
                .and_then(|budget| budget.river_area_tiles)
                .expect("validated river area budget must remain available"),
        )
        .expect("river area must fit usize");
        let side = self.rng.bounded(4);
        let start = match side {
            0 => Position {
                x: 1 + i32::try_from(self.rng.bounded(u64::from(definition.width - 2)))
                    .expect("river start x must fit i32"),
                y: 1,
            },
            1 => Position {
                x: i32::from(definition.width - 2),
                y: 1 + i32::try_from(self.rng.bounded(u64::from(definition.height - 2)))
                    .expect("river start y must fit i32"),
            },
            2 => Position {
                x: 1 + i32::try_from(self.rng.bounded(u64::from(definition.width - 2)))
                    .expect("river start x must fit i32"),
                y: i32::from(definition.height - 2),
            },
            _ => Position {
                x: 1,
                y: 1 + i32::try_from(self.rng.bounded(u64::from(definition.height - 2)))
                    .expect("river start y must fit i32"),
            },
        };
        let mut current = start;
        let mut centerline = vec![current];
        while current != target {
            let move_x = current.x != target.x;
            let move_y = current.y != target.y;
            let advance_x = move_x && (!move_y || self.rng.bounded(2) == 0);
            if advance_x {
                current.x += (target.x - current.x).signum();
            } else {
                current.y += (target.y - current.y).signum();
            }
            centerline.push(current);
        }
        debug_assert!(centerline.len() <= area);
        let mut painted = centerline.iter().copied().collect::<BTreeSet<_>>();
        for position in &centerline {
            set_generated_terrain(terrain, definition.width, *position, deep_terrain_id);
        }

        while painted.len() < area {
            let mut frontier = painted
                .iter()
                .flat_map(|position| {
                    CARDINAL_OFFSETS.map(|(dx, dy)| Position {
                        x: position.x + dx,
                        y: position.y + dy,
                    })
                })
                .filter(|position| {
                    position.x > 0
                        && position.y > 0
                        && position.x + 1 < i32::from(definition.width)
                        && position.y + 1 < i32::from(definition.height)
                        && !painted.contains(position)
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            frontier.sort_by_key(|position| (position.y, position.x));
            let index = if frontier.len() == 1 {
                0
            } else {
                usize::try_from(self.rng.bounded(
                    u64::try_from(frontier.len()).expect("river frontier count must fit u64"),
                ))
                .expect("river frontier index must fit usize")
            };
            let position = frontier[index];
            painted.insert(position);
            set_generated_terrain(terrain, definition.width, position, shallow_terrain_id);
        }
    }

    fn generated_actor(&self, id: String, kind_id: &str, position: Position) -> Actor {
        let actor = self
            .content
            .actor(kind_id)
            .expect("validated generated actor must remain available");
        actor_from_spawn(
            &id,
            kind_id,
            ContentPosition {
                x: u16::try_from(position.x).expect("generated actor x must fit u16"),
                y: u16::try_from(position.y).expect("generated actor y must fit u16"),
            },
            actor.max_hp,
            actor.speed,
            INITIAL_MONSTER_ENERGY_NEED,
        )
    }

    fn generated_pack_actor(
        &self,
        id: String,
        kind_id: &str,
        position: Position,
        pack: MonsterPackIdentity,
    ) -> Actor {
        let mut actor = self.generated_actor(id, kind_id, position);
        actor.pack = Some(pack);
        actor
    }

    fn generate_classic_pit_actors(
        &mut self,
        definition: &ProceduralFloorDefinition,
        pit: &GeneratedPitPlacement,
        occupied: &mut BTreeSet<Position>,
    ) -> Vec<Actor> {
        let table = self
            .content
            .encounter_table(&pit.definition.encounter_table_id)
            .expect("validated pit encounter table must remain available")
            .clone();
        let eligible = table
            .entries
            .iter()
            .filter(|entry| {
                entry.min_depth <= definition.depth
                    && definition.depth <= entry.max_depth
                    && self
                        .content
                        .actor(&entry.actor_kind_id)
                        .is_some_and(|actor| actor.level <= u32::from(definition.depth))
            })
            .cloned()
            .collect::<Vec<_>>();
        let pit_weights = eligible
            .iter()
            .map(|entry| entry.weight)
            .collect::<Vec<_>>();
        let mut roster = (0..pit.definition.roster_size)
            .map(|_| {
                eligible[self.roll_weighted_index(&pit_weights)]
                    .actor_kind_id
                    .clone()
            })
            .collect::<Vec<_>>();
        roster.sort_by(|left, right| {
            let left_level = self
                .content
                .actor(left)
                .expect("pit roster actor must remain available")
                .level;
            let right_level = self
                .content
                .actor(right)
                .expect("pit roster actor must remain available")
                .level;
            right_level.cmp(&left_level).then_with(|| left.cmp(right))
        });

        let half_width = pit.definition.inner_width / 2;
        let half_height = pit.definition.inner_height / 2;
        let maximum_rank = pit.definition.roster_size - 1;
        let mut ordinal = 0_u16;
        let mut actors = Vec::new();
        for local_y in 0..pit.definition.inner_height {
            for local_x in 0..pit.definition.inner_width {
                let dx = local_x.abs_diff(half_width);
                let dy = local_y.abs_diff(half_height);
                let horizontal_rank = dx * maximum_rank / half_width;
                let vertical_rank = dy * maximum_rank / half_height;
                let rank = usize::from(horizontal_rank.max(vertical_rank));
                let kind_id = &roster[rank];
                let position = Position {
                    x: pit.origin.x + 3 + i32::from(local_x),
                    y: pit.origin.y + 3 + i32::from(local_y),
                };
                occupied.insert(position);
                ordinal += 1;
                actors.push(self.generated_actor(
                    format!("{}.pit.{}", definition.id, ordinal),
                    kind_id,
                    position,
                ));
            }
        }
        actors
    }

    #[allow(clippy::too_many_arguments)]
    fn generate_dynamic_encounter_groups(
        &mut self,
        definition: &ProceduralFloorDefinition,
        table: &EncounterTableDefinition,
        eligible_entries: &[EncounterEntryDefinition],
        rooms: &[GeneratedRoom],
        room_id: &str,
        reserved_actor_slots: u16,
        ordinary_actor_reserve: u16,
        fill_plain: bool,
        id_prefix: &str,
        occupied: &mut BTreeSet<Position>,
    ) -> Vec<Actor> {
        let budget = definition
            .generation_budget
            .as_ref()
            .expect("dynamic encounters require a generation budget");
        let group_placement_limit = budget
            .group_placements
            .expect("validated group placement budget must remain available");
        let mut remaining_group_actor_slots = budget
            .group_actor_slots
            .expect("validated group actor budget must remain available");
        let mut remaining_actor_slots = budget.actor_slots.saturating_sub(reserved_actor_slots);
        let grouped_entries = eligible_entries
            .iter()
            .filter(|entry| entry.group.is_some())
            .cloned()
            .collect::<Vec<_>>();
        let plain_entries = eligible_entries
            .iter()
            .filter(|entry| entry.group.is_none())
            .cloned()
            .collect::<Vec<_>>();
        let minimum_group_companions = grouped_entries
            .iter()
            .filter_map(|entry| entry.group.as_ref())
            .map(rfb_content::EncounterGroupDefinition::min_companion_count)
            .min()
            .expect("validated dynamic floor must have a grouped encounter");
        let mut generated = Vec::new();
        let mut leader_ordinal = 0_u16;

        for group_slot in 0..group_placement_limit {
            let future_group_count = group_placement_limit - group_slot - 1;
            let future_companion_reserve =
                future_group_count.saturating_mul(minimum_group_companions);
            let future_actor_reserve = future_group_count
                .saturating_mul(minimum_group_companions.saturating_add(1))
                .saturating_add(ordinary_actor_reserve);
            let available_companion_slots = remaining_group_actor_slots
                .saturating_sub(future_companion_reserve)
                .min(
                    remaining_actor_slots
                        .saturating_sub(future_actor_reserve)
                        .saturating_sub(1),
                );
            let mut candidates = grouped_entries
                .iter()
                .filter(|entry| {
                    entry.group.as_ref().is_some_and(|group| {
                        group.min_companion_count() <= available_companion_slots
                    })
                })
                .cloned()
                .collect::<Vec<_>>();
            let mut placed_group = None;
            while !candidates.is_empty() {
                let weights = candidates
                    .iter()
                    .map(|entry| entry.weight)
                    .collect::<Vec<_>>();
                let selected_index = if candidates.len() == 1 {
                    0
                } else {
                    self.roll_weighted_index(&weights)
                };
                let entry = candidates.remove(selected_index);
                let group = entry
                    .group
                    .as_ref()
                    .expect("grouped encounter candidate must retain its group");
                let friend_min = group
                    .friends
                    .as_ref()
                    .map_or(0, |friends| friends.min_count);
                let friend_max = group
                    .friends
                    .as_ref()
                    .map_or(0, |friends| friends.max_count);
                let escort_min = group.escort.as_ref().map_or(0, |escort| escort.min_count);
                let escort_max = group.escort.as_ref().map_or(0, |escort| escort.max_count);
                let friend_upper =
                    friend_max.min(available_companion_slots.saturating_sub(escort_min));
                let mut friend_count = self.roll_inclusive(friend_min, friend_upper);
                let escort_upper =
                    escort_max.min(available_companion_slots.saturating_sub(friend_count));
                let mut escort_count = self.roll_inclusive(escort_min, escort_upper);
                let formation_placement = loop {
                    let placement_candidates = formation_placement_candidates(
                        rooms,
                        room_id,
                        occupied,
                        group.formation,
                        friend_count.saturating_add(escort_count),
                    );
                    if !placement_candidates.is_empty() {
                        let placement_index = if placement_candidates.len() == 1 {
                            0
                        } else {
                            usize::try_from(
                                self.rng
                                    .bounded(u64::try_from(placement_candidates.len()).expect(
                                        "formation placement candidate count must fit u64",
                                    )),
                            )
                            .expect("formation placement candidate index must fit usize")
                        };
                        break Some(placement_candidates[placement_index].clone());
                    }
                    if escort_count > escort_min {
                        escort_count -= 1;
                    } else if friend_count > friend_min {
                        friend_count -= 1;
                    } else {
                        break None;
                    }
                };
                let Some((leader_position, companion_positions)) = formation_placement else {
                    continue;
                };
                placed_group = Some((
                    entry,
                    friend_count,
                    escort_count,
                    leader_position,
                    companion_positions,
                ));
                break;
            }
            let Some((entry, friend_count, escort_count, leader_position, companion_positions)) =
                placed_group
            else {
                break;
            };

            leader_ordinal += 1;
            occupied.insert(leader_position);
            let leader_id = format!("{id_prefix}.encounter.{leader_ordinal}");
            let pack_id = format!("{id_prefix}.pack.{leader_ordinal}");
            let pack_ai = entry
                .group
                .as_ref()
                .expect("grouped encounter must retain pack AI")
                .pack_ai;
            generated.push(self.generated_pack_actor(
                leader_id.clone(),
                &entry.actor_kind_id,
                leader_position,
                MonsterPackIdentity {
                    id: pack_id.clone(),
                    leader_id: leader_id.clone(),
                    role: MonsterPackRoleDto::Leader,
                    behavior: monster_pack_behavior_dto(pack_ai.leader),
                },
            ));
            for (index, position) in companion_positions
                .iter()
                .take(usize::from(friend_count))
                .copied()
                .enumerate()
            {
                occupied.insert(position);
                generated.push(self.generated_pack_actor(
                    format!(
                        "{id_prefix}.encounter.{leader_ordinal}.friend.{}",
                        index + 1
                    ),
                    &entry.actor_kind_id,
                    position,
                    MonsterPackIdentity {
                        id: pack_id.clone(),
                        leader_id: leader_id.clone(),
                        role: MonsterPackRoleDto::Member,
                        behavior: monster_pack_behavior_dto(pack_ai.friends),
                    },
                ));
            }
            if escort_count > 0 {
                let escort = entry
                    .group
                    .as_ref()
                    .and_then(|group| group.escort.as_ref())
                    .expect("positive escort count must retain an escort table");
                let eligible_escorts = escort
                    .entries
                    .iter()
                    .filter(|escort_entry| {
                        escort_entry.min_depth <= definition.depth
                            && definition.depth <= escort_entry.max_depth
                            && self
                                .content
                                .actor(&escort_entry.actor_kind_id)
                                .is_some_and(|actor| actor.level <= u32::from(definition.depth))
                    })
                    .collect::<Vec<_>>();
                let escort_weights = eligible_escorts
                    .iter()
                    .map(|escort_entry| escort_entry.weight)
                    .collect::<Vec<_>>();
                for (index, position) in companion_positions
                    .iter()
                    .skip(usize::from(friend_count))
                    .take(usize::from(escort_count))
                    .copied()
                    .enumerate()
                {
                    let escort_index = if eligible_escorts.len() == 1 {
                        0
                    } else {
                        self.roll_weighted_index(&escort_weights)
                    };
                    let kind_id = &eligible_escorts[escort_index].actor_kind_id;
                    occupied.insert(position);
                    generated.push(self.generated_pack_actor(
                        format!(
                            "{id_prefix}.encounter.{leader_ordinal}.escort.{}",
                            index + 1
                        ),
                        kind_id,
                        position,
                        MonsterPackIdentity {
                            id: pack_id.clone(),
                            leader_id: leader_id.clone(),
                            role: MonsterPackRoleDto::Member,
                            behavior: monster_pack_behavior_dto(pack_ai.escorts),
                        },
                    ));
                }
            }
            let companion_count = friend_count.saturating_add(escort_count);
            remaining_group_actor_slots =
                remaining_group_actor_slots.saturating_sub(companion_count);
            remaining_actor_slots =
                remaining_actor_slots.saturating_sub(companion_count.saturating_add(1));
        }

        let plain_weights = plain_entries
            .iter()
            .map(|entry| entry.weight)
            .collect::<Vec<_>>();
        while fill_plain && leader_ordinal < table.rolls && remaining_actor_slots > 0 {
            let entry_index = if plain_entries.len() == 1 {
                0
            } else {
                self.roll_weighted_index(&plain_weights)
            };
            let entry = &plain_entries[entry_index];
            let position = self.choose_generated_room_position(rooms, room_id, occupied);
            occupied.insert(position);
            leader_ordinal += 1;
            generated.push(self.generated_actor(
                format!("{}.encounter.{leader_ordinal}", definition.id),
                &entry.actor_kind_id,
                position,
            ));
            remaining_actor_slots -= 1;
        }
        generated
    }

    fn roll_inclusive(&mut self, minimum: u16, maximum: u16) -> u16 {
        debug_assert!(minimum <= maximum);
        if minimum == maximum {
            minimum
        } else {
            minimum
                + u16::try_from(self.rng.bounded(u64::from(maximum - minimum) + 1))
                    .expect("bounded encounter group count must fit u16")
        }
    }

    fn select_spatial_vault_placements(
        &mut self,
        definition: &ProceduralFloorDefinition,
        eligible_candidates: &[ThemeVaultCandidateDefinition],
        guardian_present: bool,
        corridor_terrain_id: &str,
        terrain: &mut [String],
    ) -> Vec<GeneratedVaultPlacement> {
        let budget = definition
            .generation_budget
            .as_ref()
            .expect("spatial vault placement requires a generation budget");
        let placement_limit = budget
            .vault_placements
            .expect("validated spatial vault count must remain available");
        let mut remaining_area = budget
            .vault_area_tiles
            .expect("validated spatial vault area must remain available");
        let fixed_actor_slots = definition
            .nest
            .as_ref()
            .map_or(0, |nest| nest.spawn_count)
            .saturating_add(u16::from(guardian_present));
        let ordinary_placement_reserve = budget.region_placements.unwrap_or(1);
        let mut remaining_vault_actor_slots = budget
            .actor_slots
            .saturating_sub(fixed_actor_slots)
            .saturating_sub(ordinary_placement_reserve);
        let mut remaining_vault_loot_placements = budget
            .loot_placements
            .saturating_sub(ordinary_placement_reserve);
        let mut remaining_candidates = eligible_candidates.to_vec();
        let mut placements = Vec::new();

        'placement_slots: for ordinal in 1..=placement_limit {
            loop {
                let affordable = remaining_candidates
                    .iter()
                    .enumerate()
                    .filter_map(|(index, candidate)| {
                        let vault = self
                            .content
                            .vault(&candidate.vault_id)
                            .expect("validated spatial vault must remain available");
                        let actor_cost = vault
                            .encounter_groups
                            .iter()
                            .map(|group| {
                                u16::try_from(group.member_positions.len())
                                    .expect("validated vault actor count must fit u16")
                            })
                            .sum::<u16>();
                        let loot_cost = u16::try_from(vault.loot_spawns.len())
                            .expect("validated vault loot count must fit u16");
                        let area = u32::from(vault.width) * u32::from(vault.height);
                        (actor_cost <= remaining_vault_actor_slots
                            && loot_cost <= remaining_vault_loot_placements
                            && area <= remaining_area)
                            .then_some((index, candidate.weight))
                    })
                    .collect::<Vec<_>>();
                if affordable.is_empty() {
                    break 'placement_slots;
                }
                let selected_affordable = if affordable.len() == 1 {
                    0
                } else {
                    let weights = affordable
                        .iter()
                        .map(|(_, weight)| *weight)
                        .collect::<Vec<_>>();
                    self.roll_weighted_index(&weights)
                };
                let candidate_index = affordable[selected_affordable].0;
                let candidate = remaining_candidates.remove(candidate_index);
                let vault = self
                    .content
                    .vault(&candidate.vault_id)
                    .expect("validated spatial vault must remain available")
                    .clone();
                let placement_candidates = free_vault_placement_candidates(
                    terrain,
                    definition.width,
                    definition.height,
                    &definition.wall_terrain_id,
                    corridor_terrain_id,
                    &vault,
                    &self.content,
                );
                if placement_candidates.is_empty() {
                    continue;
                }
                let placement_index = if placement_candidates.len() == 1 {
                    0
                } else {
                    usize::try_from(
                        self.rng.bounded(
                            u64::try_from(placement_candidates.len())
                                .expect("vault placement candidate count must fit u64"),
                        ),
                    )
                    .expect("vault placement candidate index must fit usize")
                };
                let candidate = placement_candidates[placement_index].clone();
                let actor_cost = vault
                    .encounter_groups
                    .iter()
                    .map(|group| {
                        u16::try_from(group.member_positions.len())
                            .expect("validated vault actor count must fit u16")
                    })
                    .sum::<u16>();
                let loot_cost = u16::try_from(vault.loot_spawns.len())
                    .expect("validated vault loot count must fit u16");
                let area = u32::from(vault.width) * u32::from(vault.height);
                let placement = GeneratedVaultPlacement {
                    vault,
                    origin: candidate.origin,
                    transform: candidate.transform,
                    ordinal,
                    connector_cells: candidate.connector_cells,
                };
                apply_generated_vault_placement(
                    terrain,
                    definition.width,
                    corridor_terrain_id,
                    &placement,
                );
                remaining_vault_actor_slots =
                    remaining_vault_actor_slots.saturating_sub(actor_cost);
                remaining_vault_loot_placements =
                    remaining_vault_loot_placements.saturating_sub(loot_cost);
                remaining_area = remaining_area.saturating_sub(area);
                placements.push(placement);
                break;
            }
        }
        placements
    }

    fn place_terrain_features(
        &mut self,
        definition: &ProceduralFloorDefinition,
        eligible_entries: &[TerrainFeatureEntryDefinition],
        context: TerrainFeaturePlacementContext<'_>,
        terrain: &mut [String],
    ) -> Vec<GeneratedTerrainFeature> {
        let placement_limit = definition
            .generation_budget
            .as_ref()
            .and_then(|budget| budget.feature_placements)
            .expect("terrain feature placement requires a validated budget");
        let mut placements = Vec::new();

        'placement_slots: for _ in 0..placement_limit {
            let mut remaining_entries = eligible_entries.to_vec();
            loop {
                if remaining_entries.is_empty() {
                    break 'placement_slots;
                }
                let selected_index = if remaining_entries.len() == 1 {
                    0
                } else {
                    let weights = remaining_entries
                        .iter()
                        .map(|entry| entry.weight)
                        .collect::<Vec<_>>();
                    self.roll_weighted_index(&weights)
                };
                let entry = remaining_entries.remove(selected_index);
                let candidates = terrain_feature_placement_candidates(
                    terrain,
                    definition.width,
                    context.floor_terrain_id,
                    context.room_floor_terrain_ids,
                    context.rooms,
                    context.reserved,
                    entry.placement,
                );
                if candidates.is_empty() {
                    continue;
                }
                let position_index = if candidates.len() == 1 {
                    0
                } else {
                    usize::try_from(
                        self.rng.bounded(
                            u64::try_from(candidates.len())
                                .expect("terrain feature candidate count must fit u64"),
                        ),
                    )
                    .expect("terrain feature candidate index must fit usize")
                };
                let position = candidates[position_index];
                set_generated_terrain(terrain, definition.width, position, &entry.terrain_id);
                placements.push(GeneratedTerrainFeature {
                    terrain_id: entry.terrain_id,
                    position,
                });
                break;
            }
        }
        placements
    }

    fn choose_generated_room_position(
        &mut self,
        rooms: &[GeneratedRoom],
        room_id: &str,
        occupied: &BTreeSet<Position>,
    ) -> Position {
        let room = rooms
            .iter()
            .find(|room| room.id == room_id)
            .expect("validated procedural room ID must remain available");
        let candidates = (room.y..room.y + room.height)
            .flat_map(|y| (room.x..room.x + room.width).map(move |x| Position { x, y }))
            .filter(|position| room.contains(*position) && !occupied.contains(position))
            .collect::<Vec<_>>();
        let index = usize::try_from(self.rng.bounded(
            u64::try_from(candidates.len()).expect("generated room candidate count must fit u64"),
        ))
        .expect("bounded generated room candidate index must fit usize");
        candidates[index]
    }

    fn choose_generated_region_position(
        &mut self,
        region: &GeneratedRegion,
        terrain: &[String],
        width: u16,
        occupied: &BTreeSet<Position>,
    ) -> Position {
        let candidates =
            generated_region_open_positions(region, terrain, width, &self.content, occupied);
        let index = usize::try_from(self.rng.bounded(
            u64::try_from(candidates.len()).expect("regional candidate count must fit u64"),
        ))
        .expect("regional candidate index must fit usize");
        candidates[index]
    }

    fn terrain_at(&self, position: Position) -> &str {
        &self.terrain[self.index(position).expect("validated map position")]
    }

    fn known_terrain_at(&self, position: Position) -> &str {
        let terrain_id = self.terrain_at(position);
        let definition = self
            .content
            .terrain(terrain_id)
            .expect("active terrain must remain available");
        if !self.revealed_terrain.contains(&position)
            && let Some(concealed_as) = definition.concealed_as_terrain_id.as_deref()
        {
            concealed_as
        } else {
            terrain_id
        }
    }

    fn terrain_interactions(&self) -> Vec<TerrainInteractionDto> {
        let mut interactions = Vec::new();
        for direction in TERRAIN_INTERACTION_DIRECTIONS {
            let position = self.position_in_direction(direction);
            if self.index(position).is_none() {
                continue;
            }
            let Some(terrain) = self.content.terrain(self.known_terrain_at(position)) else {
                continue;
            };
            let unavailable_reason = self.terrain_interaction_unavailable_reason(position);
            let available = unavailable_reason.is_none();
            if terrain.open_to_terrain_id.is_some() {
                interactions.push(TerrainInteractionDto {
                    kind: TerrainInteractionKindDto::OpenDoor,
                    direction,
                    position,
                    terrain_id: terrain.id.clone(),
                    requires_check: terrain.open_check_difficulty.is_some(),
                    available,
                    unavailable_reason,
                });
            }
            if terrain.close_to_terrain_id.is_some() {
                interactions.push(TerrainInteractionDto {
                    kind: TerrainInteractionKindDto::CloseDoor,
                    direction,
                    position,
                    terrain_id: terrain.id.clone(),
                    requires_check: false,
                    available,
                    unavailable_reason,
                });
            }
            if terrain.bash_to_terrain_id.is_some() {
                interactions.push(TerrainInteractionDto {
                    kind: TerrainInteractionKindDto::BashDoor,
                    direction,
                    position,
                    terrain_id: terrain.id.clone(),
                    requires_check: true,
                    available,
                    unavailable_reason,
                });
            }
            if terrain.trap.is_some() {
                interactions.push(TerrainInteractionDto {
                    kind: TerrainInteractionKindDto::DisarmTrap,
                    direction,
                    position,
                    terrain_id: terrain.id.clone(),
                    requires_check: true,
                    available,
                    unavailable_reason,
                });
            }
            if terrain.dig_to_terrain_id.is_some() {
                interactions.push(TerrainInteractionDto {
                    kind: TerrainInteractionKindDto::DigTerrain,
                    direction,
                    position,
                    terrain_id: terrain.id.clone(),
                    requires_check: true,
                    available,
                    unavailable_reason,
                });
            }
        }
        interactions
    }

    fn task_statuses(&self) -> Vec<TaskStatusDto> {
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        self.task_states
            .iter()
            .map(|(task_id, state)| {
                let floor = world
                    .procedural_floors
                    .iter()
                    .find(|floor| floor_task_id(floor) == task_id)
                    .expect("task state must have a representative floor");
                let stages = u32::try_from(task_objectives(world, task_id).len())
                    .expect("validated task stage count must fit u32");
                TaskStatusDto {
                    task_id: task_id.clone(),
                    floor_id: floor.id.clone(),
                    name_key: floor.name_key.clone(),
                    status: state.status,
                    current: state.current,
                    required: state.required,
                    stage: state.stage_index.saturating_add(1),
                    stages,
                    retakes_used: state.retakes_used,
                    max_retakes: floor.max_retakes,
                }
            })
            .collect()
    }

    fn trigger_player_trap(&mut self, position: Position) -> Option<(String, DamageOutcome)> {
        let index = self.index(position)?;
        let terrain = self.content.terrain(&self.terrain[index])?;
        let source_kind_id = terrain.id.clone();
        let trap = terrain.trap.as_ref()?;
        let damage = resolve_damage(
            DamagePacket::new(trap.damage, trap.damage_type.into()),
            self.player.resistances.level(trap.damage_type.into()),
        );
        self.player.hp = self.player.hp.saturating_sub(damage.applied);
        self.revealed_terrain.insert(position);
        Some((source_kind_id, damage))
    }

    fn disarm_trap(&mut self, direction: Direction) -> Option<TrapDisarmOutcome> {
        let position = self.position_in_direction(direction);
        let index = self.index(position)?;
        if !self.revealed_terrain.contains(&position)
            || self
                .terrain_interaction_unavailable_reason(position)
                .is_some()
        {
            return None;
        }
        let terrain = self.content.terrain(&self.terrain[index])?;
        let source_id = terrain.id.clone();
        let trap = terrain.trap.as_ref()?;
        let target_id = trap.disarm_to_terrain_id.clone();
        let difficulty = trap.disarm_check_difficulty;
        let ability = self.player_derived_stats().disarm_skill;
        let mut difficulty_pipeline = DerivedStatsPipeline::new();
        difficulty_pipeline.add(
            StatKind::ActionDifficulty,
            StatLayer::Environment,
            &source_id,
            difficulty,
        );
        let check = resolve_check(
            &mut self.rng,
            CheckContext {
                kind: CheckKind::DisarmTrap,
                actor_id: self.player.id.clone(),
                target_id: Some(source_id),
                ability,
                difficulty: difficulty_pipeline
                    .resolve(StatKind::ActionDifficulty, StatBounds::NON_NEGATIVE),
            },
        );
        if !check.succeeded() {
            return Some(TrapDisarmOutcome::Failed { position });
        }
        self.terrain[index] = target_id;
        self.revealed_terrain.remove(&position);
        Some(TrapDisarmOutcome::Succeeded { position })
    }

    fn dig_terrain(&mut self, direction: Direction) -> Option<TerrainDigOutcome> {
        let position = self.position_in_direction(direction);
        let index = self.index(position)?;
        if self
            .terrain_interaction_unavailable_reason(position)
            .is_some()
        {
            return None;
        }
        let terrain = self.content.terrain(self.known_terrain_at(position))?;
        let source_id = terrain.id.clone();
        let target_id = terrain.dig_to_terrain_id.clone()?;
        let difficulty = terrain.dig_check_difficulty?;
        let ability = self.player_derived_stats().dig_skill;
        let mut difficulty_pipeline = DerivedStatsPipeline::new();
        difficulty_pipeline.add(
            StatKind::ActionDifficulty,
            StatLayer::Environment,
            &source_id,
            difficulty,
        );
        let check = resolve_check(
            &mut self.rng,
            CheckContext {
                kind: CheckKind::DigTerrain,
                actor_id: self.player.id.clone(),
                target_id: Some(source_id),
                ability,
                difficulty: difficulty_pipeline
                    .resolve(StatKind::ActionDifficulty, StatBounds::NON_NEGATIVE),
            },
        );
        if !check.succeeded() {
            return Some(TerrainDigOutcome::Failed { position });
        }
        self.terrain[index] = target_id;
        self.revealed_terrain.remove(&position);
        Some(TerrainDigOutcome::Succeeded { position })
    }

    fn search_hidden_terrain(&mut self) -> Vec<Position> {
        let candidates = TERRAIN_INTERACTION_DIRECTIONS
            .into_iter()
            .filter_map(|direction| {
                let position = self.position_in_direction(direction);
                let index = self.index(position)?;
                if self.revealed_terrain.contains(&position) {
                    return None;
                }
                let terrain = self.content.terrain(&self.terrain[index])?;
                Some((
                    position,
                    terrain.id.clone(),
                    terrain.search_check_difficulty?,
                ))
            })
            .collect::<Vec<_>>();
        let ability = self.player_derived_stats().search_skill;
        let mut discovered = Vec::new();
        for (position, terrain_id, difficulty) in candidates {
            let mut difficulty_pipeline = DerivedStatsPipeline::new();
            difficulty_pipeline.add(
                StatKind::ActionDifficulty,
                StatLayer::Environment,
                &terrain_id,
                difficulty,
            );
            let check = resolve_check(
                &mut self.rng,
                CheckContext {
                    kind: CheckKind::SearchTerrain,
                    actor_id: self.player.id.clone(),
                    target_id: Some(terrain_id),
                    ability: ability.clone(),
                    difficulty: difficulty_pipeline
                        .resolve(StatKind::ActionDifficulty, StatBounds::NON_NEGATIVE),
                },
            );
            if check.succeeded() {
                self.revealed_terrain.insert(position);
                discovered.push(position);
            }
        }
        discovered
    }

    fn terrain_interaction_unavailable_reason(
        &self,
        position: Position,
    ) -> Option<TerrainInteractionUnavailableReasonDto> {
        if self
            .entities
            .iter()
            .any(|entity| entity.position == position)
        {
            return Some(TerrainInteractionUnavailableReasonDto::OccupiedByActor);
        }
        if self.items.iter().any(|item| {
            matches!(item.location, ItemLocation::Ground(item_position) if item_position == position)
        }) {
            return Some(TerrainInteractionUnavailableReasonDto::OccupiedByItem);
        }
        None
    }

    fn open_door(&mut self, direction: rfb_protocol::Direction) -> Option<DoorOpenOutcome> {
        let position = self.position_in_direction(direction);
        let index = self.index(position)?;
        if self
            .terrain_interaction_unavailable_reason(position)
            .is_some()
        {
            return None;
        }
        let terrain = self.content.terrain(self.known_terrain_at(position))?;
        let source_id = terrain.id.clone();
        let target_id = terrain.open_to_terrain_id.clone()?;
        let difficulty = terrain.open_check_difficulty;
        if let Some(difficulty) = difficulty {
            let stats = self.player_derived_stats();
            let mut difficulty_pipeline = DerivedStatsPipeline::new();
            difficulty_pipeline.add(
                StatKind::ActionDifficulty,
                StatLayer::Environment,
                &source_id,
                difficulty,
            );
            let check = resolve_check(
                &mut self.rng,
                CheckContext {
                    kind: CheckKind::UnlockDoor,
                    actor_id: self.player.id.clone(),
                    target_id: Some(source_id),
                    ability: stats.door_skill,
                    difficulty: difficulty_pipeline
                        .resolve(StatKind::ActionDifficulty, StatBounds::NON_NEGATIVE),
                },
            );
            if !check.succeeded() {
                return Some(DoorOpenOutcome::UnlockFailed { position });
            }
        }
        self.terrain[index] = target_id;
        self.revealed_terrain.remove(&position);
        Some(if difficulty.is_some() {
            DoorOpenOutcome::Unlocked { position }
        } else {
            DoorOpenOutcome::Opened { position }
        })
    }

    fn bash_door(&mut self, direction: rfb_protocol::Direction) -> Option<DoorBashOutcome> {
        let position = self.position_in_direction(direction);
        let index = self.index(position)?;
        if self
            .terrain_interaction_unavailable_reason(position)
            .is_some()
        {
            return None;
        }
        let terrain = self.content.terrain(self.known_terrain_at(position))?;
        let source_id = terrain.id.clone();
        let target_id = terrain.bash_to_terrain_id.clone()?;
        let difficulty = terrain.bash_check_difficulty?;
        let stats = self.player_derived_stats();
        let mut difficulty_pipeline = DerivedStatsPipeline::new();
        difficulty_pipeline.add(
            StatKind::ActionDifficulty,
            StatLayer::Environment,
            &source_id,
            difficulty,
        );
        let check = resolve_check(
            &mut self.rng,
            CheckContext {
                kind: CheckKind::BashDoor,
                actor_id: self.player.id.clone(),
                target_id: Some(source_id),
                ability: stats.bash_power,
                difficulty: difficulty_pipeline
                    .resolve(StatKind::ActionDifficulty, StatBounds::NON_NEGATIVE),
            },
        );
        if !check.succeeded() {
            return Some(DoorBashOutcome::Failed { position });
        }
        self.terrain[index] = target_id;
        self.revealed_terrain.remove(&position);
        Some(DoorBashOutcome::Succeeded { position })
    }

    fn close_door(&mut self, direction: rfb_protocol::Direction) -> Option<Position> {
        let position = self.position_in_direction(direction);
        let index = self.index(position)?;
        if self
            .terrain_interaction_unavailable_reason(position)
            .is_some()
        {
            return None;
        }
        let target_id = self
            .content
            .terrain(&self.terrain[index])?
            .close_to_terrain_id
            .clone()?;
        self.terrain[index] = target_id;
        Some(position)
    }

    fn position_in_direction(&self, direction: rfb_protocol::Direction) -> Position {
        let (dx, dy) = direction.delta();
        Position {
            x: self.player.position.x + dx,
            y: self.player.position.y + dy,
        }
    }

    fn index(&self, position: Position) -> Option<usize> {
        if position.x < 0
            || position.y < 0
            || position.x >= i32::from(self.width)
            || position.y >= i32::from(self.height)
        {
            return None;
        }
        Some(position.y as usize * usize::from(self.width) + position.x as usize)
    }

    fn is_walkable(&self, position: Position) -> bool {
        self.index(position)
            .and_then(|index| self.content.terrain(&self.terrain[index]))
            .is_some_and(|terrain| terrain.walkable)
    }

    fn validate_state(&self) -> Result<(), CoreError> {
        let world = self
            .content
            .world(&self.world_id)
            .ok_or_else(|| CoreError::UnknownWorld(self.world_id.clone()))?;
        let valid_floor = |floor_id: &str| {
            floor_id == world.initial_floor_id
                || world
                    .procedural_floors
                    .iter()
                    .any(|floor| floor.id == floor_id)
        };
        if !valid_floor(&self.current_floor_id)
            || self
                .stored_floors
                .values()
                .any(|floor| !valid_floor(&floor.id))
        {
            return Err(CoreError::InvalidSave("floor identity is invalid"));
        }
        let current_dungeon_id = floor_dungeon_id(world, &self.current_floor_id);
        match (&current_dungeon_id, &self.current_dungeon_instance_id) {
            (Some(dungeon_id), Some(instance_id))
                if parse_dungeon_instance_ordinal(instance_id, dungeon_id).is_some() => {}
            (None, None) => {}
            _ => {
                return Err(CoreError::InvalidSave(
                    "active floor dungeon instance identity is invalid",
                ));
            }
        }
        for floor in self.stored_floors.values() {
            let expected_instance = floor_dungeon_id(world, &floor.id).is_some();
            if expected_instance != floor.dungeon_instance_id.is_some() {
                return Err(CoreError::InvalidSave(
                    "stored floor dungeon instance identity is invalid",
                ));
            }
        }
        if !floor_connections_are_valid(
            &self.current_floor_id,
            self.width,
            self.height,
            &self.terrain,
            &self.floor_connections,
            world,
        ) {
            return Err(CoreError::InvalidSave(
                "active floor connection state is invalid",
            ));
        }
        if !floor_regions_are_valid(
            &self.current_floor_id,
            (self.width, self.height),
            &self.floor_regions,
            &self.entities,
            &self.items,
            world,
            &self.content,
        ) {
            return Err(CoreError::InvalidSave(
                "active floor region state is invalid",
            ));
        }
        if self.explored.len() != self.terrain.len() {
            return Err(CoreError::InvalidSave(
                "exploration memory dimensions are invalid",
            ));
        }
        if !revealed_terrain_is_valid(
            &self.revealed_terrain,
            &self.terrain,
            self.width,
            self.height,
            &self.content,
        ) {
            return Err(CoreError::InvalidSave(
                "revealed terrain knowledge is invalid",
            ));
        }
        for terrain_id in &self.terrain {
            if self.content.terrain(terrain_id).is_none() {
                return Err(CoreError::UnknownTerrain(terrain_id.clone()));
            }
        }
        self.validate_actor(&self.player, ActorRole::Player)?;
        if !self.is_walkable(self.player.position) {
            return Err(CoreError::InvalidSave("player position is invalid"));
        }
        let mut instance_ids = BTreeSet::new();
        instance_ids.insert(self.player.id.clone());
        let mut monster_ids = BTreeSet::new();
        let mut positions = BTreeSet::new();
        positions.insert(self.player.position);
        for entity in &self.entities {
            self.validate_actor(entity, ActorRole::Monster)?;
            if !instance_ids.insert(entity.id.clone())
                || !self.is_walkable(entity.position)
                || !positions.insert(entity.position)
            {
                return Err(CoreError::InvalidSave("entity position is invalid"));
            }
            monster_ids.insert(entity.id.clone());
        }
        if !monster_packs_are_valid(&self.entities) {
            return Err(CoreError::InvalidSave("monster pack state is invalid"));
        }
        let mut equipment_slots = BTreeSet::new();
        for item in &self.items {
            let definition = self
                .content
                .item(&item.kind_id)
                .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?;
            let affixes_are_valid = item.affix_ids.windows(2).all(|pair| pair[0] < pair[1])
                && item
                    .affix_ids
                    .iter()
                    .all(|affix_id| self.content.affix(affix_id).is_some())
                && (item.affix_ids.is_empty()
                    || (definition.max_stack == 1
                        && definition.equipment_slot.is_some()
                        && item.quantity == 1
                        && item.quality != ItemQualityDto::Ordinary))
                && (item.quality == ItemQualityDto::Ordinary
                    || (definition.max_stack == 1 && item.quantity == 1));
            let common_valid = instance_ids.insert(item.id.clone()) && item.quantity != 0;
            if !affixes_are_valid {
                return Err(CoreError::InvalidSave(
                    "item quality or affix state is invalid",
                ));
            }
            match &item.location {
                ItemLocation::Ground(position) => {
                    if !common_valid
                        || !self.is_walkable(*position)
                        || item.quantity > definition.max_stack
                    {
                        return Err(CoreError::InvalidSave("item state is invalid"));
                    }
                }
                ItemLocation::Inventory => {
                    if !common_valid || item.quantity > definition.max_stack {
                        return Err(CoreError::InvalidSave("inventory item state is invalid"));
                    }
                }
                ItemLocation::Equipped { slot_id } => {
                    let fully_identified =
                        self.item_property_knowledge
                            .get(&item.id)
                            .is_some_and(|knowledge| {
                                knowledge.identified
                                    && item.affix_ids.iter().all(|affix_id| {
                                        knowledge.known_affix_ids.contains(affix_id)
                                    })
                            });
                    if !common_valid
                        || item.quantity != 1
                        || definition.equipment_slot.as_deref() != Some(slot_id.as_str())
                        || !equipment_slots.insert(slot_id.clone())
                        || !fully_identified
                    {
                        return Err(CoreError::InvalidSave("equipment item state is invalid"));
                    }
                }
                ItemLocation::CarriedBy { actor_id } => {
                    if !common_valid
                        || !monster_ids.contains(actor_id)
                        || item.quantity > definition.max_stack
                    {
                        return Err(CoreError::InvalidSave("carried item state is invalid"));
                    }
                }
            }
        }
        for floor in self.stored_floors.values() {
            let expected_len = usize::from(floor.width) * usize::from(floor.height);
            if floor.terrain.len() != expected_len
                || floor.explored.len() != expected_len
                || !revealed_terrain_is_valid(
                    &floor.revealed_terrain,
                    &floor.terrain,
                    floor.width,
                    floor.height,
                    &self.content,
                )
                || (floor.id == self.current_floor_id
                    && floor.dungeon_instance_id == self.current_dungeon_instance_id)
                || !floor_position_is_walkable(floor, floor.player_position, &self.content)
            {
                return Err(CoreError::InvalidSave("stored floor state is invalid"));
            }
            if !floor_connections_are_valid(
                &floor.id,
                floor.width,
                floor.height,
                &floor.terrain,
                &floor.connections,
                world,
            ) {
                return Err(CoreError::InvalidSave(
                    "stored floor connection state is invalid",
                ));
            }
            if !floor_regions_are_valid(
                &floor.id,
                (floor.width, floor.height),
                &floor.regions,
                &floor.entities,
                &floor.items,
                world,
                &self.content,
            ) {
                return Err(CoreError::InvalidSave(
                    "stored floor region state is invalid",
                ));
            }
            for terrain_id in &floor.terrain {
                if self.content.terrain(terrain_id).is_none() {
                    return Err(CoreError::UnknownTerrain(terrain_id.clone()));
                }
            }
            let mut floor_positions = BTreeSet::new();
            let mut floor_monster_ids = BTreeSet::new();
            for entity in &floor.entities {
                self.validate_actor(entity, ActorRole::Monster)?;
                if !instance_ids.insert(entity.id.clone())
                    || !floor_position_is_walkable(floor, entity.position, &self.content)
                    || !floor_positions.insert(entity.position)
                {
                    return Err(CoreError::InvalidSave(
                        "stored floor entity state is invalid",
                    ));
                }
                floor_monster_ids.insert(entity.id.clone());
            }
            if !monster_packs_are_valid(&floor.entities) {
                return Err(CoreError::InvalidSave(
                    "stored floor monster pack state is invalid",
                ));
            }
            for item in &floor.items {
                let definition = self
                    .content
                    .item(&item.kind_id)
                    .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?;
                let affixes_are_valid = item.affix_ids.windows(2).all(|pair| pair[0] < pair[1])
                    && item
                        .affix_ids
                        .iter()
                        .all(|affix_id| self.content.affix(affix_id).is_some())
                    && (item.affix_ids.is_empty()
                        || (definition.max_stack == 1
                            && definition.equipment_slot.is_some()
                            && item.quantity == 1
                            && item.quality != ItemQualityDto::Ordinary))
                    && (item.quality == ItemQualityDto::Ordinary
                        || (definition.max_stack == 1 && item.quantity == 1));
                let location_is_valid = match &item.location {
                    ItemLocation::Ground(position) => {
                        floor_position_is_walkable(floor, *position, &self.content)
                    }
                    ItemLocation::CarriedBy { actor_id } => floor_monster_ids.contains(actor_id),
                    ItemLocation::Inventory | ItemLocation::Equipped { .. } => false,
                };
                if !instance_ids.insert(item.id.clone())
                    || item.quantity == 0
                    || item.quantity > definition.max_stack
                    || !affixes_are_valid
                    || !location_is_valid
                {
                    return Err(CoreError::InvalidSave("stored floor item state is invalid"));
                }
            }
        }
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        let expected_tasks = initial_task_states(world);
        if self.task_states.len() != expected_tasks.len() {
            return Err(CoreError::InvalidSave("task state set is invalid"));
        }
        for (task_id, state) in &self.task_states {
            let Some(expected) = expected_tasks.get(task_id) else {
                return Err(CoreError::InvalidSave("task state ID is invalid"));
            };
            let members = world
                .procedural_floors
                .iter()
                .filter(|floor| floor_task_id(floor) == task_id)
                .collect::<Vec<_>>();
            let objectives = task_objectives(world, task_id);
            let Some(objective) = usize::try_from(state.stage_index)
                .ok()
                .and_then(|stage| objectives.get(stage))
            else {
                return Err(CoreError::InvalidSave("task stage is invalid"));
            };
            let active_is_valid = state.active_floor_id.as_ref().is_some_and(|floor_id| {
                floor_id == &self.current_floor_id
                    && members.iter().any(|floor| floor.id == *floor_id)
            });
            let paused_is_valid = members.iter().any(|floor| {
                self.stored_floors
                    .values()
                    .any(|stored| stored.id == floor.id)
            });
            let status_is_valid = match state.status {
                TaskStatusKindDto::Active => active_is_valid,
                TaskStatusKindDto::Paused => state.active_floor_id.is_none() && paused_is_valid,
                TaskStatusKindDto::Completed => {
                    state.active_floor_id.is_none()
                        && usize::try_from(state.stage_index)
                            .ok()
                            .is_some_and(|stage| stage + 1 == objectives.len())
                        && state.current == state.required
                }
                TaskStatusKindDto::Available
                | TaskStatusKindDto::Failed
                | TaskStatusKindDto::Abandoned => state.active_floor_id.is_none(),
            };
            if (state.stage_index == 0 && expected.required != objective.required)
                || state.required != objective.required
                || state.current > state.required
                || members
                    .first()
                    .and_then(|floor| floor.max_retakes)
                    .is_some_and(|maximum| state.retakes_used > maximum)
                || !status_is_valid
            {
                return Err(CoreError::InvalidSave("task state is invalid"));
            }
        }
        let expected_dungeons = initial_dungeon_states(world);
        if self.dungeon_states.len() != expected_dungeons.len() {
            return Err(CoreError::InvalidSave("dungeon state set is invalid"));
        }
        for (dungeon_id, state) in &self.dungeon_states {
            if !expected_dungeons.contains_key(dungeon_id) {
                return Err(CoreError::InvalidSave("dungeon state ID is invalid"));
            }
            for final_floor in world.procedural_floors.iter().filter(|floor| {
                floor.dungeon_id.as_deref() == Some(dungeon_id.as_str()) && floor.final_floor
            }) {
                let guardian_id = &final_floor
                    .guardian
                    .as_ref()
                    .expect("validated final floor must retain a guardian")
                    .instance_id;
                let guardian_present = if self.current_floor_id == final_floor.id {
                    Some(self.entities.iter().any(|actor| &actor.id == guardian_id))
                } else {
                    self.stored_floors
                        .values()
                        .find(|stored| stored.id == final_floor.id)
                        .map(|floor| floor.entities.iter().any(|actor| &actor.id == guardian_id))
                };
                if guardian_present.is_some_and(|present| present == state.guardian_defeated) {
                    return Err(CoreError::InvalidSave("dungeon guardian state is invalid"));
                }
            }
        }
        for (item_id, knowledge) in &self.item_property_knowledge {
            let Some(item) = self
                .items
                .iter()
                .chain(
                    self.stored_floors
                        .values()
                        .flat_map(|floor| floor.items.iter()),
                )
                .find(|item| &item.id == item_id)
            else {
                return Err(CoreError::InvalidSave(
                    "item property knowledge state is invalid",
                ));
            };
            let empty_knowledge = !knowledge.appraised
                && !knowledge.identified
                && knowledge.known_affix_ids.is_empty();
            let identification_without_appraisal = knowledge.identified && !knowledge.appraised;
            let foreign_affix = knowledge
                .known_affix_ids
                .iter()
                .any(|affix_id| !item.affix_ids.contains(affix_id));
            let incomplete_identification = knowledge.identified
                && item
                    .affix_ids
                    .iter()
                    .any(|affix_id| !knowledge.known_affix_ids.contains(affix_id));
            if empty_knowledge
                || identification_without_appraisal
                || foreign_affix
                || incomplete_identification
            {
                return Err(CoreError::InvalidSave(
                    "item property knowledge state is invalid",
                ));
            }
        }
        let mut allocator_entities = self.entities.clone();
        let mut allocator_items = self.items.clone();
        for floor in self.stored_floors.values() {
            allocator_entities.extend(floor.entities.iter().cloned());
            allocator_items.extend(floor.items.iter().cloned());
        }
        if self.next_item_instance_serial == 0
            || self.next_item_instance_serial
                < derive_next_item_instance_serial(
                    &self.player,
                    &allocator_entities,
                    &allocator_items,
                )?
        {
            return Err(CoreError::InvalidSave(
                "item instance allocator is behind existing IDs",
            ));
        }
        Ok(())
    }

    fn validate_actor(&self, actor: &Actor, expected_role: ActorRole) -> Result<(), CoreError> {
        let definition = self
            .content
            .actor(&actor.kind_id)
            .ok_or_else(|| CoreError::UnknownActor(actor.kind_id.clone()))?;
        let effective_max_hp = if expected_role == ActorRole::Player {
            self.effective_player_max_hp()
        } else {
            actor.max_hp
        };
        let statuses_are_valid = actor.statuses.iter().all(|status| {
            status.intensity > 0
                && status.remaining_ticks > 0
                && !status.kind_id.is_empty()
                && status.kind_id.len() <= 128
        }) && actor
            .statuses
            .windows(2)
            .all(|window| window[0].kind_id < window[1].kind_id);
        if definition.role != expected_role
            || actor.max_hp != definition.max_hp
            || actor.speed != definition.speed
            || actor.speed > 199
            || !statuses_are_valid
            || (expected_role == ActorRole::Monster && actor.hp <= 0)
            || (expected_role == ActorRole::Player && actor.hp < -1_000_000)
            || (expected_role == ActorRole::Monster
                && !(1..=STANDARD_ACTION_COST).contains(&actor.energy_need))
            || (expected_role == ActorRole::Player && actor.hp >= 0 && actor.energy_need > 0)
            || actor.energy_need < -STANDARD_ACTION_COST
            || actor.hp > effective_max_hp
            || (expected_role == ActorRole::Player && actor.pack.is_some())
        {
            return Err(CoreError::InvalidSave("actor state is invalid"));
        }
        Ok(())
    }
}

fn monster_packs_are_valid(entities: &[Actor]) -> bool {
    let mut packs = BTreeMap::<&str, Vec<&Actor>>::new();
    for entity in entities {
        let Some(pack) = &entity.pack else {
            continue;
        };
        let valid_id = |id: &str| {
            !id.is_empty()
                && id.len() <= 128
                && id.bytes().all(|byte| {
                    byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
                })
        };
        if !valid_id(&pack.id) || !valid_id(&pack.leader_id) {
            return false;
        }
        packs.entry(&pack.id).or_default().push(entity);
    }
    packs.into_values().all(|members| {
        let leaders = members
            .iter()
            .filter(|entity| {
                entity.pack.as_ref().is_some_and(|pack| {
                    pack.role == MonsterPackRoleDto::Leader
                        && pack.behavior != MonsterPackBehaviorDto::GuardLeader
                })
            })
            .copied()
            .collect::<Vec<_>>();
        leaders.len() == 1
            && members.iter().all(|entity| {
                let pack = entity
                    .pack
                    .as_ref()
                    .expect("pack member must retain identity");
                pack.leader_id == leaders[0].id
                    && ((pack.role == MonsterPackRoleDto::Leader && entity.id == pack.leader_id)
                        || (pack.role == MonsterPackRoleDto::Member && entity.id != pack.leader_id))
            })
    })
}

struct ActorStatusTick {
    damage: Vec<StatusDamageTick>,
    expired: Vec<String>,
    fatal_damage: Option<StatusDamageTick>,
}

#[derive(Clone)]
struct StatusDamageTick {
    status_kind_id: String,
    outcome: DamageOutcome,
}

struct ActorDerivedStats {
    max_hp: DerivedStat,
    attack: DerivedStat,
    defense: DerivedStat,
    speed: DerivedStat,
    melee_skill: DerivedStat,
    armor_class: DerivedStat,
    melee_attacks: DerivedStat,
    melee_damage_bonus: DerivedStat,
    ranged_skill: DerivedStat,
    throwing_skill: DerivedStat,
    door_skill: DerivedStat,
    bash_power: DerivedStat,
    search_skill: DerivedStat,
    disarm_skill: DerivedStat,
    dig_skill: DerivedStat,
}

enum TrapDisarmOutcome {
    Succeeded { position: Position },
    Failed { position: Position },
}

enum TerrainDigOutcome {
    Succeeded { position: Position },
    Failed { position: Position },
}

struct FloorTransitionOutcome {
    from_floor_id: String,
    to_floor_id: String,
    expedition_ended: bool,
    one_shot_closed: Option<(String, TaskResolution)>,
    task_paused: Option<String>,
    task_resumed: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TaskResolution {
    Completed,
    Failed,
    Abandoned,
}

enum DoorOpenOutcome {
    Opened { position: Position },
    Unlocked { position: Position },
    UnlockFailed { position: Position },
}

enum DoorBashOutcome {
    Succeeded { position: Position },
    Failed { position: Position },
}

#[derive(Clone)]
struct ResolvedAttackProfile {
    attacks: u16,
    to_hit: i32,
    to_damage: i32,
    damage_dice: u16,
    damage_sides: u16,
    damage_type: DamageType,
    source_item_id: Option<String>,
}

struct ResolvedMeleeBlow {
    method_id: Option<String>,
    to_hit: i32,
    damage_dice: u16,
    damage_sides: u16,
    damage_type: DamageType,
}

struct ResolvedProjectileProfile {
    range: u16,
    to_hit: i32,
    to_damage: i32,
    damage_dice: u16,
    damage_sides: u16,
    damage_type: DamageType,
    ammo_kind_id: String,
    ammo_break_chance_percent: u8,
    source_item_id: String,
}

#[derive(Clone)]
struct ResolvedThrowProfile {
    to_hit: i32,
    to_damage: i32,
    damage_dice: u16,
    damage_sides: u16,
    damage_type: DamageType,
}

#[derive(Debug, Clone, Default)]
struct ItemKnowledgeState {
    tried: bool,
    aware: bool,
}

#[derive(Debug, Clone, Default)]
struct ItemPropertyKnowledgeState {
    appraised: bool,
    identified: bool,
    known_affix_ids: BTreeSet<String>,
}

fn item_knowledge_from_save(
    entries: Vec<ItemKnowledgeSaveDto>,
    content: &ContentCatalog,
) -> Result<BTreeMap<String, ItemKnowledgeState>, CoreError> {
    let mut knowledge = BTreeMap::new();
    for entry in entries {
        let valid_kind = content
            .item(&entry.kind_id)
            .is_some_and(|definition| definition.appearance_name_key.is_some());
        if !valid_kind
            || !entry.tried
            || knowledge
                .insert(
                    entry.kind_id,
                    ItemKnowledgeState {
                        tried: entry.tried,
                        aware: entry.aware,
                    },
                )
                .is_some()
        {
            return Err(CoreError::InvalidSave("item knowledge state is invalid"));
        }
    }
    Ok(knowledge)
}

fn item_property_knowledge_from_save(
    entries: Vec<ItemPropertyKnowledgeSaveDto>,
    items: &[ItemInstance],
    content: &ContentCatalog,
) -> Result<BTreeMap<String, ItemPropertyKnowledgeState>, CoreError> {
    let mut knowledge = BTreeMap::new();
    for entry in entries {
        let Some(item) = items.iter().find(|item| item.id == entry.item_id) else {
            return Err(CoreError::InvalidSave(
                "item property knowledge state is invalid",
            ));
        };
        let known_affix_count = entry.known_affix_ids.len();
        let known_affix_ids = entry.known_affix_ids.into_iter().collect::<BTreeSet<_>>();
        let all_affixes_known = item
            .affix_ids
            .iter()
            .all(|affix_id| known_affix_ids.contains(affix_id));
        let identified = entry.identified || (!known_affix_ids.is_empty() && all_affixes_known);
        let appraised = entry.appraised || identified;
        if (!appraised && !identified && known_affix_ids.is_empty())
            || known_affix_ids.len() != known_affix_count
            || known_affix_ids.iter().any(|affix_id| {
                !item.affix_ids.contains(affix_id) || content.affix(affix_id).is_none()
            })
            || (identified && !all_affixes_known)
            || knowledge
                .insert(
                    entry.item_id,
                    ItemPropertyKnowledgeState {
                        appraised,
                        identified,
                        known_affix_ids,
                    },
                )
                .is_some()
        {
            return Err(CoreError::InvalidSave(
                "item property knowledge state is invalid",
            ));
        }
    }
    Ok(knowledge)
}

fn item_quality_dto(quality: rfb_content::ItemQuality) -> ItemQualityDto {
    match quality {
        rfb_content::ItemQuality::Ordinary => ItemQualityDto::Ordinary,
        rfb_content::ItemQuality::Fine => ItemQualityDto::Fine,
        rfb_content::ItemQuality::Exceptional => ItemQualityDto::Exceptional,
    }
}

enum PickUpOutcome {
    Picked {
        kind_id: String,
        quantity: u32,
    },
    OverCapacity {
        kind_id: String,
        quantity: u32,
        current_weight: u32,
        pickup_weight: u32,
        capacity: u32,
    },
    Nothing,
}

fn throw_range(weight_tenths_pound: u16) -> u16 {
    (BASE_THROW_RANGE_BUDGET / weight_tenths_pound.max(1)).clamp(MIN_THROW_RANGE, MAX_THROW_RANGE)
}

fn projectile_target_spec(range: u16) -> TargetSpecDto {
    TargetSpecDto {
        modes: vec![
            TargetModeDto::Direction,
            TargetModeDto::Position,
            TargetModeDto::Entity,
        ],
        range,
        requires_line_of_effect: true,
    }
}

impl ResolvedProjectileProfile {
    fn to_dto(&self) -> ProjectileProfileDto {
        ProjectileProfileDto {
            range: self.range,
            to_hit: self.to_hit,
            to_damage: self.to_damage,
            damage: DamageDiceDto {
                dice: self.damage_dice,
                sides: self.damage_sides,
                damage_type: self.damage_type.into(),
            },
            ammo_kind_id: self.ammo_kind_id.clone(),
            target_spec: projectile_target_spec(self.range),
            source_item_id: self.source_item_id.clone(),
        }
    }
}

fn resolved_melee_blows(definition: &rfb_content::ActorDefinition) -> Vec<ResolvedMeleeBlow> {
    definition.melee_routine.as_ref().map_or_else(
        || {
            vec![ResolvedMeleeBlow {
                method_id: None,
                to_hit: 0,
                damage_dice: definition.damage_dice,
                damage_sides: definition.damage_sides,
                damage_type: DamageType::from(definition.damage_type),
            }]
        },
        |routine| {
            routine
                .blows
                .iter()
                .map(|blow| ResolvedMeleeBlow {
                    method_id: Some(blow.method_id.clone()),
                    to_hit: blow.to_hit,
                    damage_dice: blow.damage_dice,
                    damage_sides: blow.damage_sides,
                    damage_type: DamageType::from(blow.damage_type),
                })
                .collect()
        },
    )
}

fn actor_melee_routine_dto(definition: &rfb_content::ActorDefinition) -> MeleeRoutineDto {
    MeleeRoutineDto {
        blows: resolved_melee_blows(definition)
            .into_iter()
            .map(|blow| MeleeBlowDto {
                method_id: blow
                    .method_id
                    .unwrap_or_else(|| "rfb.blow.innate".to_owned()),
                to_hit: blow.to_hit,
                damage: DamageDiceDto {
                    dice: blow.damage_dice,
                    sides: blow.damage_sides,
                    damage_type: blow.damage_type.into(),
                },
            })
            .collect(),
    }
}

impl ResolvedAttackProfile {
    fn to_dto(&self) -> AttackProfileDto {
        AttackProfileDto {
            attacks: self.attacks,
            to_hit: self.to_hit,
            to_damage: self.to_damage,
            damage: DamageDiceDto {
                dice: self.damage_dice,
                sides: self.damage_sides,
                damage_type: self.damage_type.into(),
            },
            source_item_id: self.source_item_id.clone(),
        }
    }
}

fn add_equipment_stat(
    pipeline: &mut DerivedStatsPipeline,
    kind: StatKind,
    source_id: &str,
    amount: i32,
) {
    if amount != 0 {
        pipeline.add(kind, StatLayer::Equipment, source_id, amount);
    }
}

fn derived_speed(speed: &DerivedStat) -> u16 {
    u16::try_from(speed.value).expect("derived actor speed must fit u16")
}

fn process_actor_status_tick(actor: &mut Actor, lethal_at_zero: bool) -> ActorStatusTick {
    let periodic = actor
        .statuses
        .iter()
        .filter_map(|status| {
            let damage_type = match status.kind_id.as_str() {
                STATUS_BLEEDING => DamageType::Physical,
                STATUS_POISON => DamageType::Poison,
                _ => return None,
            };
            Some((
                status.kind_id.clone(),
                i32::from(status.intensity),
                damage_type,
            ))
        })
        .collect::<Vec<_>>();
    let mut damage = Vec::new();
    let mut fatal_damage = None;
    for (status_kind_id, amount, damage_type) in periodic {
        let mut target = EffectTarget {
            hp: &mut actor.hp,
            max_hp: actor.max_hp,
            resistances: &actor.resistances,
            statuses: &mut actor.statuses,
        };
        let EffectOutcome::Damage(outcome) = apply_effect(
            &mut target,
            EffectSpec::Damage(DamagePacket::new(amount, damage_type)),
        ) else {
            unreachable!("damage effects must produce damage outcomes");
        };
        let damage_tick = StatusDamageTick {
            status_kind_id: status_kind_id.clone(),
            outcome,
        };
        damage.push(damage_tick.clone());
        if actor.hp < 0 || (lethal_at_zero && actor.hp == 0) {
            fatal_damage = Some(damage_tick);
            break;
        }
    }
    let expired = advance_status_ticks(&mut actor.statuses, 1);
    ActorStatusTick {
        damage,
        expired,
        fatal_damage,
    }
}

fn squared_distance(left: Position, right: Position) -> i32 {
    let dx = left.x - right.x;
    let dy = left.y - right.y;
    dx * dx + dy * dy
}

const fn monster_pack_behavior_dto(behavior: MonsterPackBehavior) -> MonsterPackBehaviorDto {
    match behavior {
        MonsterPackBehavior::Seek => MonsterPackBehaviorDto::Seek,
        MonsterPackBehavior::Surround => MonsterPackBehaviorDto::Surround,
        MonsterPackBehavior::GuardLeader => MonsterPackBehaviorDto::GuardLeader,
    }
}

fn generated_non_entry_room_id(rooms: &[GeneratedRoom], ordinal: u16) -> &str {
    let room_index = 1 + usize::from(ordinal) % (rooms.len() - 1);
    &rooms[room_index].id
}

fn maze_floor_anchors(walkable: &BTreeSet<Position>) -> (Position, Position) {
    let seed = walkable
        .iter()
        .min_by_key(|position| (position.y, position.x))
        .copied()
        .expect("validated maze must retain walkable terrain");
    let entry = farthest_maze_position(walkable, seed);
    let remote = farthest_maze_position(walkable, entry);
    (entry, remote)
}

fn farthest_maze_position(walkable: &BTreeSet<Position>, start: Position) -> Position {
    let distances = maze_floor_distances(walkable, start);
    let mut positions = distances.keys().copied().collect::<Vec<_>>();
    positions.sort_by(|left, right| {
        distances[right]
            .cmp(&distances[left])
            .then_with(|| left.y.cmp(&right.y))
            .then_with(|| left.x.cmp(&right.x))
    });
    positions[0]
}

fn maze_floor_distances(walkable: &BTreeSet<Position>, start: Position) -> BTreeMap<Position, u32> {
    const CARDINAL_OFFSETS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
    let mut distances = BTreeMap::from([(start, 0_u32)]);
    let mut frontier = VecDeque::from([start]);
    while let Some(position) = frontier.pop_front() {
        let next_distance = distances[&position] + 1;
        for (dx, dy) in CARDINAL_OFFSETS {
            let neighbor = Position {
                x: position.x + dx,
                y: position.y + dy,
            };
            if walkable.contains(&neighbor) && !distances.contains_key(&neighbor) {
                distances.insert(neighbor, next_distance);
                frontier.push_back(neighbor);
            }
        }
    }
    distances
}

fn maze_floor_path(walkable: &BTreeSet<Position>, start: Position, end: Position) -> Vec<Position> {
    const CARDINAL_OFFSETS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
    let mut predecessors = BTreeMap::new();
    let mut visited = BTreeSet::from([start]);
    let mut frontier = VecDeque::from([start]);
    while let Some(position) = frontier.pop_front() {
        if position == end {
            break;
        }
        for (dx, dy) in CARDINAL_OFFSETS {
            let neighbor = Position {
                x: position.x + dx,
                y: position.y + dy,
            };
            if walkable.contains(&neighbor) && visited.insert(neighbor) {
                predecessors.insert(neighbor, position);
                frontier.push_back(neighbor);
            }
        }
    }
    let mut path = vec![end];
    let mut current = end;
    while current != start {
        current = predecessors[&current];
        path.push(current);
    }
    path.reverse();
    path
}

fn choose_generated_maze_position(
    walkable: &BTreeSet<Position>,
    entry: Position,
    occupied: &BTreeSet<Position>,
) -> Position {
    let distances = maze_floor_distances(walkable, entry);
    let mut candidates = walkable
        .iter()
        .filter(|position| !occupied.contains(position))
        .copied()
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        distances[right]
            .cmp(&distances[left])
            .then_with(|| left.y.cmp(&right.y))
            .then_with(|| left.x.cmp(&right.x))
    });
    candidates[0]
}

fn formation_placement_candidates(
    rooms: &[GeneratedRoom],
    room_id: &str,
    occupied: &BTreeSet<Position>,
    formation: EncounterFormation,
    companion_count: u16,
) -> Vec<(Position, Vec<Position>)> {
    const RING_OFFSETS: [(i32, i32); 8] = [
        (0, -1),
        (1, -1),
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
        (-1, -1),
    ];
    const CLUSTER_ORDER: [usize; 8] = [0, 2, 4, 6, 1, 3, 5, 7];
    let room = rooms
        .iter()
        .find(|room| room.id == room_id)
        .expect("validated formation room must remain available");
    let mut candidates = Vec::new();
    for leader_y in room.y..room.y + room.height {
        for leader_x in room.x..room.x + room.width {
            let leader = Position {
                x: leader_x,
                y: leader_y,
            };
            if !room.contains(leader) || occupied.contains(&leader) {
                continue;
            }
            for orientation in 0..RING_OFFSETS.len() {
                let offsets = (0..usize::from(companion_count))
                    .map(|index| {
                        let base_index = match formation {
                            EncounterFormation::Cluster => CLUSTER_ORDER[index],
                            EncounterFormation::Ring => {
                                index * RING_OFFSETS.len() / usize::from(companion_count)
                            }
                        };
                        RING_OFFSETS[(base_index + orientation) % RING_OFFSETS.len()]
                    })
                    .collect::<Vec<_>>();
                let companions = offsets
                    .iter()
                    .map(|(dx, dy)| Position {
                        x: leader.x + dx,
                        y: leader.y + dy,
                    })
                    .collect::<Vec<_>>();
                if companions
                    .iter()
                    .all(|position| room.contains(*position) && !occupied.contains(position))
                {
                    candidates.push((leader, companions));
                }
            }
        }
    }
    candidates
}

fn transformed_vault_dimensions(vault: &VaultDefinition, transform: VaultTransform) -> (u16, u16) {
    match transform {
        VaultTransform::Identity
        | VaultTransform::Rotate180
        | VaultTransform::MirrorHorizontal
        | VaultTransform::MirrorVertical => (vault.width, vault.height),
        VaultTransform::Rotate90
        | VaultTransform::Rotate270
        | VaultTransform::MirrorMainDiagonal
        | VaultTransform::MirrorAntiDiagonal => (vault.height, vault.width),
    }
}

fn transformed_vault_position(
    vault: &VaultDefinition,
    transform: VaultTransform,
    position: ContentPosition,
) -> Position {
    let x = i32::from(position.x);
    let y = i32::from(position.y);
    let max_x = i32::from(vault.width - 1);
    let max_y = i32::from(vault.height - 1);
    match transform {
        VaultTransform::Identity => Position { x, y },
        VaultTransform::Rotate90 => Position { x: max_y - y, y: x },
        VaultTransform::Rotate180 => Position {
            x: max_x - x,
            y: max_y - y,
        },
        VaultTransform::Rotate270 => Position { x: y, y: max_x - x },
        VaultTransform::MirrorHorizontal => Position { x: max_x - x, y },
        VaultTransform::MirrorVertical => Position { x, y: max_y - y },
        VaultTransform::MirrorMainDiagonal => Position { x: y, y: x },
        VaultTransform::MirrorAntiDiagonal => Position {
            x: max_y - y,
            y: max_x - x,
        },
    }
}

fn free_vault_placement_candidates(
    terrain: &[String],
    width: u16,
    height: u16,
    wall_terrain_id: &str,
    corridor_terrain_id: &str,
    vault: &VaultDefinition,
    content: &ContentCatalog,
) -> Vec<GeneratedVaultPlacementCandidate> {
    let transforms = if vault.transforms.is_empty() {
        vec![VaultTransform::Identity]
    } else {
        vault.transforms.clone()
    };
    let mut candidates = Vec::new();
    for transform in transforms {
        let (transformed_width, transformed_height) =
            transformed_vault_dimensions(vault, transform);
        if transformed_width + 2 > width || transformed_height + 2 > height {
            continue;
        }
        let mut entrances = vault
            .entrance_positions
            .iter()
            .map(|position| transformed_vault_position(vault, transform, *position))
            .collect::<Vec<_>>();
        entrances.sort_by_key(|position| (position.y, position.x));
        for origin_y in 1..=i32::from(height - transformed_height - 1) {
            for origin_x in 1..=i32::from(width - transformed_width - 1) {
                let origin = Position {
                    x: origin_x,
                    y: origin_y,
                };
                let footprint_is_free = (0..i32::from(transformed_height)).all(|local_y| {
                    (0..i32::from(transformed_width)).all(|local_x| {
                        let position = Position {
                            x: origin.x + local_x,
                            y: origin.y + local_y,
                        };
                        let index = position.y as usize * usize::from(width) + position.x as usize;
                        terrain
                            .get(index)
                            .is_some_and(|terrain_id| terrain_id == wall_terrain_id)
                    })
                });
                if !footprint_is_free {
                    continue;
                }
                let footprint = (0..i32::from(transformed_height))
                    .flat_map(|local_y| {
                        (0..i32::from(transformed_width)).map(move |local_x| Position {
                            x: origin.x + local_x,
                            y: origin.y + local_y,
                        })
                    })
                    .collect::<BTreeSet<_>>();
                let mut connector_cells = BTreeSet::new();
                let mut all_entrances_connect = true;
                for entrance in &entrances {
                    let outward =
                        vault_entrance_outward(*entrance, transformed_width, transformed_height);
                    let outside = Position {
                        x: origin.x + entrance.x + outward.x,
                        y: origin.y + entrance.y + outward.y,
                    };
                    let Some(path) = vault_connector_path(
                        terrain,
                        width,
                        wall_terrain_id,
                        &footprint,
                        &connector_cells,
                        outside,
                        content,
                    ) else {
                        all_entrances_connect = false;
                        break;
                    };
                    connector_cells.extend(path);
                }
                if !all_entrances_connect {
                    continue;
                }
                let connector_cells = connector_cells.into_iter().collect::<Vec<_>>();
                let placement = GeneratedVaultPlacement {
                    vault: vault.clone(),
                    origin,
                    transform,
                    ordinal: 0,
                    connector_cells: connector_cells.clone(),
                };
                let mut proof_terrain = terrain.to_vec();
                apply_generated_vault_placement(
                    &mut proof_terrain,
                    width,
                    corridor_terrain_id,
                    &placement,
                );
                if generated_terrain_is_connected(&proof_terrain, width, height, content) {
                    candidates.push(GeneratedVaultPlacementCandidate {
                        origin,
                        transform,
                        connector_cells,
                    });
                }
            }
        }
    }
    candidates
}

const MAX_VAULT_CONNECTOR_TILES: usize = 12;

fn vault_entrance_outward(
    entrance: Position,
    transformed_width: u16,
    transformed_height: u16,
) -> Position {
    if entrance.y == 0 {
        Position { x: 0, y: -1 }
    } else if entrance.x + 1 == i32::from(transformed_width) {
        Position { x: 1, y: 0 }
    } else if entrance.y + 1 == i32::from(transformed_height) {
        Position { x: 0, y: 1 }
    } else {
        Position { x: -1, y: 0 }
    }
}

fn vault_connector_path(
    terrain: &[String],
    width: u16,
    wall_terrain_id: &str,
    footprint: &BTreeSet<Position>,
    existing_connectors: &BTreeSet<Position>,
    start: Position,
    content: &ContentCatalog,
) -> Option<Vec<Position>> {
    let height = i32::try_from(terrain.len() / usize::from(width)).ok()?;
    if start.x <= 0
        || start.y <= 0
        || start.x >= i32::from(width - 1)
        || start.y >= height - 1
        || footprint.contains(&start)
    {
        return None;
    }
    let is_target = |position: Position| {
        if existing_connectors.contains(&position) {
            return true;
        }
        let index = position.y as usize * usize::from(width) + position.x as usize;
        terrain.get(index).is_some_and(|terrain_id| {
            terrain_id != wall_terrain_id && terrain_is_connectable(content, terrain_id)
        })
    };
    if is_target(start) {
        return Some(Vec::new());
    }
    let start_index = start.y as usize * usize::from(width) + start.x as usize;
    if terrain
        .get(start_index)
        .is_none_or(|id| id != wall_terrain_id)
    {
        return None;
    }

    let mut pending = VecDeque::from([start]);
    let mut distance = BTreeMap::from([(start, 0_usize)]);
    let mut parent = BTreeMap::new();
    while let Some(position) = pending.pop_front() {
        let current_distance = distance[&position];
        for direction in [
            Position { x: 0, y: -1 },
            Position { x: 1, y: 0 },
            Position { x: 0, y: 1 },
            Position { x: -1, y: 0 },
        ] {
            let next = Position {
                x: position.x + direction.x,
                y: position.y + direction.y,
            };
            if next.x <= 0
                || next.y <= 0
                || next.x >= i32::from(width - 1)
                || next.y >= height - 1
                || footprint.contains(&next)
                || distance.contains_key(&next)
            {
                continue;
            }
            let index = next.y as usize * usize::from(width) + next.x as usize;
            let terrain_id = terrain.get(index)?;
            if is_target(next) {
                parent.insert(next, position);
                let mut path = Vec::new();
                let mut cursor = next;
                while cursor != start {
                    cursor = parent[&cursor];
                    path.push(cursor);
                }
                path.reverse();
                path.retain(|cell| !existing_connectors.contains(cell));
                return (path.len() <= MAX_VAULT_CONNECTOR_TILES).then_some(path);
            }
            if terrain_id != wall_terrain_id || current_distance >= MAX_VAULT_CONNECTOR_TILES {
                continue;
            }
            distance.insert(next, current_distance + 1);
            parent.insert(next, position);
            pending.push_back(next);
        }
    }
    None
}

fn terrain_is_connectable(content: &ContentCatalog, terrain_id: &str) -> bool {
    content.terrain(terrain_id).is_some_and(|terrain| {
        terrain.walkable
            || terrain.open_to_terrain_id.is_some()
            || terrain.bash_to_terrain_id.is_some()
            || terrain.dig_to_terrain_id.is_some()
    })
}

fn generated_terrain_is_connected(
    terrain: &[String],
    width: u16,
    height: u16,
    content: &ContentCatalog,
) -> bool {
    let connectable = terrain
        .iter()
        .enumerate()
        .filter_map(|(index, terrain_id)| {
            terrain_is_connectable(content, terrain_id).then_some(Position {
                x: i32::try_from(index % usize::from(width)).expect("floor x must fit i32"),
                y: i32::try_from(index / usize::from(width)).expect("floor y must fit i32"),
            })
        })
        .collect::<BTreeSet<_>>();
    let Some(start) = connectable.first().copied() else {
        return false;
    };
    let mut reached = BTreeSet::new();
    let mut pending = VecDeque::from([start]);
    while let Some(position) = pending.pop_front() {
        if !connectable.contains(&position) || !reached.insert(position) {
            continue;
        }
        for direction in [
            Position { x: 0, y: -1 },
            Position { x: 1, y: 0 },
            Position { x: 0, y: 1 },
            Position { x: -1, y: 0 },
        ] {
            let next = Position {
                x: position.x + direction.x,
                y: position.y + direction.y,
            };
            if next.x >= 0 && next.y >= 0 && next.x < i32::from(width) && next.y < i32::from(height)
            {
                pending.push_back(next);
            }
        }
    }
    reached == connectable
}

fn apply_generated_vault_placement(
    terrain: &mut [String],
    width: u16,
    corridor_terrain_id: &str,
    placement: &GeneratedVaultPlacement,
) {
    paint_generated_vault(terrain, width, placement);
    for position in &placement.connector_cells {
        set_generated_terrain(terrain, width, *position, corridor_terrain_id);
    }
}

fn paint_generated_vault(terrain: &mut [String], width: u16, placement: &GeneratedVaultPlacement) {
    for local_y in 0..placement.vault.height {
        for local_x in 0..placement.vault.width {
            let local = transformed_vault_position(
                &placement.vault,
                placement.transform,
                ContentPosition {
                    x: local_x,
                    y: local_y,
                },
            );
            set_generated_terrain(
                terrain,
                width,
                Position {
                    x: placement.origin.x + local.x,
                    y: placement.origin.y + local.y,
                },
                &placement.vault.base_terrain_id,
            );
        }
    }

    for terrain_override in &placement.vault.terrain_overrides {
        for position in &terrain_override.positions {
            let local =
                transformed_vault_position(&placement.vault, placement.transform, *position);
            set_generated_terrain(
                terrain,
                width,
                Position {
                    x: placement.origin.x + local.x,
                    y: placement.origin.y + local.y,
                },
                &terrain_override.terrain_id,
            );
        }
    }
}

impl Game {
    fn generate_maze(
        &mut self,
        definition: &ProceduralFloorDefinition,
        maze: &ProceduralMazeDefinition,
        floor_terrain_id: &str,
        terrain: &mut [String],
    ) -> BTreeSet<Position> {
        let left = i32::from((definition.width - maze.width) / 2);
        let top = i32::from((definition.height - maze.height) / 2);
        for y in top..top + i32::from(maze.height) {
            for x in left..left + i32::from(maze.width) {
                set_generated_terrain(
                    terrain,
                    definition.width,
                    Position { x, y },
                    &definition.wall_terrain_id,
                );
            }
        }

        let columns = usize::from(maze.width.div_ceil(2));
        let rows = usize::from(maze.height.div_ceil(2));
        let vertex_count = columns * rows;
        let root = usize::try_from(
            self.rng
                .bounded(u64::try_from(vertex_count).expect("maze vertex count must fit u64")),
        )
        .expect("maze root must fit usize");
        let node_position = |node: usize| Position {
            x: left + i32::try_from((node % columns) * 2).expect("maze x must fit i32"),
            y: top + i32::try_from((node / columns) * 2).expect("maze y must fit i32"),
        };
        let mut visited = BTreeSet::from([root]);
        let mut stack = vec![root];
        let mut carved = BTreeSet::new();
        let root_position = node_position(root);
        carved.insert(root_position);
        set_generated_terrain(terrain, definition.width, root_position, floor_terrain_id);

        while let Some(&node) = stack.last() {
            let column = node % columns;
            let row = node / columns;
            let mut neighbors = Vec::new();
            if row > 0 && !visited.contains(&(node - columns)) {
                neighbors.push(node - columns);
            }
            if column + 1 < columns && !visited.contains(&(node + 1)) {
                neighbors.push(node + 1);
            }
            if row + 1 < rows && !visited.contains(&(node + columns)) {
                neighbors.push(node + columns);
            }
            if column > 0 && !visited.contains(&(node - 1)) {
                neighbors.push(node - 1);
            }
            if neighbors.is_empty() {
                stack.pop();
                continue;
            }
            let neighbor_index = if neighbors.len() == 1 {
                0
            } else {
                usize::try_from(self.rng.bounded(
                    u64::try_from(neighbors.len()).expect("maze neighbor count must fit u64"),
                ))
                .expect("maze neighbor index must fit usize")
            };
            let neighbor = neighbors[neighbor_index];
            let from = node_position(node);
            let to = node_position(neighbor);
            let connector = Position {
                x: (from.x + to.x) / 2,
                y: (from.y + to.y) / 2,
            };
            for position in [connector, to] {
                carved.insert(position);
                set_generated_terrain(terrain, definition.width, position, floor_terrain_id);
            }
            visited.insert(neighbor);
            stack.push(neighbor);
        }

        carved
    }

    fn generate_destroyed_region(
        &mut self,
        definition: &ProceduralFloorDefinition,
        terrain_id: &str,
        terrain: &mut [String],
    ) -> BTreeSet<Position> {
        const CARDINAL_OFFSETS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
        let budget = definition
            .generation_budget
            .as_ref()
            .expect("destroyed region requires a generation budget");
        let center_count = usize::from(
            budget
                .destruction_centers
                .expect("validated destruction center budget must remain available"),
        );
        let area = usize::try_from(
            budget
                .destroyed_area_tiles
                .expect("validated destroyed area budget must remain available"),
        )
        .expect("destroyed area must fit usize");
        let margin_x = i32::from((definition.width / 5).max(2));
        let margin_y = i32::from((definition.height / 5).max(2));
        let mut center_candidates = (margin_y..i32::from(definition.height) - margin_y)
            .flat_map(|y| {
                (margin_x..i32::from(definition.width) - margin_x).map(move |x| Position { x, y })
            })
            .collect::<Vec<_>>();
        let mut selected = BTreeSet::new();
        for _ in 0..center_count {
            let index = if center_candidates.len() == 1 {
                0
            } else {
                usize::try_from(
                    self.rng.bounded(
                        u64::try_from(center_candidates.len())
                            .expect("destruction center count must fit u64"),
                    ),
                )
                .expect("destruction center index must fit usize")
            };
            selected.insert(center_candidates.remove(index));
        }

        while selected.len() < area {
            let mut frontier = selected
                .iter()
                .flat_map(|position| {
                    CARDINAL_OFFSETS.map(|(dx, dy)| Position {
                        x: position.x + dx,
                        y: position.y + dy,
                    })
                })
                .filter(|position| {
                    position.x > 0
                        && position.y > 0
                        && position.x + 1 < i32::from(definition.width)
                        && position.y + 1 < i32::from(definition.height)
                        && !selected.contains(position)
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            frontier.sort_by_key(|position| (position.y, position.x));
            let index = if frontier.len() == 1 {
                0
            } else {
                usize::try_from(self.rng.bounded(
                    u64::try_from(frontier.len()).expect("destroyed frontier count must fit u64"),
                ))
                .expect("destroyed frontier index must fit usize")
            };
            selected.insert(frontier[index]);
        }
        for position in &selected {
            set_generated_terrain(terrain, definition.width, *position, terrain_id);
        }
        selected
    }

    fn generate_streamers(
        &mut self,
        definition: &ProceduralFloorDefinition,
        streamers: &[ProceduralStreamerCandidateDefinition],
        terrain: &mut [String],
    ) -> BTreeSet<Position> {
        const DIRECTIONS: [(i32, i32); 8] = [
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ];
        let budget = definition
            .generation_budget
            .as_ref()
            .expect("streamers require a generation budget");
        let placement_count = budget
            .streamer_placements
            .expect("validated streamer placement count must remain available");
        let area = usize::try_from(
            budget
                .streamer_area_tiles
                .expect("validated streamer area budget must remain available"),
        )
        .expect("streamer area must fit usize");
        let weights = streamers
            .iter()
            .map(|candidate| candidate.weight)
            .collect::<Vec<_>>();
        let mut assignments = BTreeMap::<Position, String>::new();

        for _ in 0..placement_count {
            let streamer_index = if streamers.len() == 1 {
                0
            } else {
                self.roll_weighted_index(&weights)
            };
            let streamer = &streamers[streamer_index];
            let mut starts = Vec::new();
            for y in (definition.height / 3)..=(definition.height * 2 / 3) {
                for x in (definition.width / 3)..=(definition.width * 2 / 3) {
                    let position = Position {
                        x: i32::from(x),
                        y: i32::from(y),
                    };
                    if terrain[generated_terrain_index(definition.width, position)]
                        == definition.wall_terrain_id
                    {
                        starts.push(position);
                    }
                }
            }
            if starts.is_empty() {
                starts = generated_wall_positions(definition, terrain);
            }
            if starts.is_empty() {
                break;
            }
            starts.sort_by_key(|position| (position.y, position.x));
            let start_index = if starts.len() == 1 {
                0
            } else {
                usize::try_from(self.rng.bounded(
                    u64::try_from(starts.len()).expect("streamer start count must fit u64"),
                ))
                .expect("streamer start index must fit usize")
            };
            let direction_index =
                usize::try_from(self.rng.bounded(8)).expect("streamer direction must fit usize");
            let (dx, dy) = DIRECTIONS[direction_index];
            let mut cursor = starts[start_index];
            while cursor.x > 0
                && cursor.y > 0
                && cursor.x + 1 < i32::from(definition.width)
                && cursor.y + 1 < i32::from(definition.height)
            {
                for y in cursor.y - 1..=cursor.y + 1 {
                    for x in cursor.x - 1..=cursor.x + 1 {
                        let position = Position { x, y };
                        if position.x > 0
                            && position.y > 0
                            && position.x + 1 < i32::from(definition.width)
                            && position.y + 1 < i32::from(definition.height)
                            && terrain[generated_terrain_index(definition.width, position)]
                                == definition.wall_terrain_id
                        {
                            assignments
                                .entry(position)
                                .or_insert_with(|| streamer.terrain_id.clone());
                        }
                    }
                }
                cursor.x += dx;
                cursor.y += dy;
            }
        }

        let mut painted = BTreeSet::new();
        while painted.len() < area {
            let mut candidates = assignments
                .iter()
                .filter_map(|(position, terrain_id)| {
                    (!painted.contains(position)
                        && terrain[generated_terrain_index(definition.width, *position)]
                            == definition.wall_terrain_id)
                        .then_some((*position, terrain_id.as_str()))
                })
                .collect::<Vec<_>>();
            candidates.sort_by_key(|(position, _)| (position.y, position.x));
            if candidates.is_empty() {
                let fallback = generated_wall_positions(definition, terrain);
                if fallback.is_empty() {
                    break;
                }
                let index = if fallback.len() == 1 {
                    0
                } else {
                    usize::try_from(
                        self.rng.bounded(
                            u64::try_from(fallback.len())
                                .expect("streamer fallback count must fit u64"),
                        ),
                    )
                    .expect("streamer fallback index must fit usize")
                };
                let position = fallback[index];
                set_generated_terrain(
                    terrain,
                    definition.width,
                    position,
                    &streamers[0].terrain_id,
                );
                painted.insert(position);
                continue;
            }
            let index = if candidates.len() == 1 {
                0
            } else {
                usize::try_from(self.rng.bounded(
                    u64::try_from(candidates.len()).expect("streamer candidate count must fit u64"),
                ))
                .expect("streamer candidate index must fit usize")
            };
            let (position, terrain_id) = candidates[index];
            set_generated_terrain(terrain, definition.width, position, terrain_id);
            painted.insert(position);
        }
        painted
    }
}

fn assign_generated_rooms_to_regions(rooms: &[GeneratedRoom], region_count: usize) -> Vec<usize> {
    if region_count == 0 {
        return Vec::new();
    }
    debug_assert!(region_count <= rooms.len());
    let anchors = if region_count == 1 {
        vec![0]
    } else {
        (0..region_count)
            .map(|index| index * (rooms.len() - 1) / (region_count - 1))
            .collect::<Vec<_>>()
    };
    rooms
        .iter()
        .map(|room| {
            let center = room.center();
            anchors
                .iter()
                .enumerate()
                .min_by_key(|(region_index, anchor)| {
                    let anchor_center = rooms[**anchor].center();
                    (
                        center.x.abs_diff(anchor_center.x) + center.y.abs_diff(anchor_center.y),
                        *region_index,
                    )
                })
                .map(|(region_index, _)| region_index)
                .expect("region floor must retain an anchor")
        })
        .collect()
}

fn generated_room_cells(room: &GeneratedRoom) -> Vec<Position> {
    (room.y..room.y + room.height)
        .flat_map(|y| {
            (room.x..room.x + room.width).filter_map(move |x| {
                let position = Position { x, y };
                room.contains(position).then_some(position)
            })
        })
        .collect()
}

fn allocate_generated_region_placements(
    regions: &[GeneratedRegion],
    terrain: &[String],
    width: u16,
    content: &ContentCatalog,
    occupied: &BTreeSet<Position>,
    actor_placements: u16,
    loot_placements: u16,
) -> (Vec<u16>, Vec<u16>) {
    let region_count = u16::try_from(regions.len()).expect("regional count must fit u16");
    debug_assert!(actor_placements >= region_count);
    debug_assert!(loot_placements >= region_count);

    let mut remaining_capacity = regions
        .iter()
        .map(|region| {
            generated_region_open_positions(region, terrain, width, content, occupied).len()
        })
        .collect::<Vec<_>>();
    assert!(
        remaining_capacity.iter().all(|capacity| *capacity >= 2),
        "each generated region must retain room space for an actor and loot"
    );

    let mut actor_allocations = vec![1_u16; regions.len()];
    let mut loot_allocations = vec![1_u16; regions.len()];
    for capacity in &mut remaining_capacity {
        *capacity -= 2;
    }

    let mut actor_remaining = actor_placements - region_count;
    let mut region_index = 0_usize;
    while actor_remaining > 0 {
        if remaining_capacity[region_index] > 0 {
            actor_allocations[region_index] += 1;
            remaining_capacity[region_index] -= 1;
            actor_remaining -= 1;
        }
        region_index = (region_index + 1) % regions.len();
        assert!(
            actor_remaining == 0 || remaining_capacity.iter().any(|capacity| *capacity > 0),
            "generated regions must retain enough room space for actor placements"
        );
    }

    let mut loot_remaining = loot_placements - region_count;
    while loot_remaining > 0 {
        if remaining_capacity[region_index] > 0 {
            loot_allocations[region_index] += 1;
            remaining_capacity[region_index] -= 1;
            loot_remaining -= 1;
        }
        region_index = (region_index + 1) % regions.len();
        assert!(
            loot_remaining == 0 || remaining_capacity.iter().any(|capacity| *capacity > 0),
            "generated regions must retain enough room space for loot placements"
        );
    }

    (actor_allocations, loot_allocations)
}

fn generated_region_open_positions(
    region: &GeneratedRegion,
    terrain: &[String],
    width: u16,
    content: &ContentCatalog,
    occupied: &BTreeSet<Position>,
) -> Vec<Position> {
    region
        .state
        .cells
        .iter()
        .copied()
        .filter(|position| !occupied.contains(position))
        .filter(|position| {
            content
                .terrain(&terrain[generated_terrain_index(width, *position)])
                .is_some_and(|definition| definition.walkable)
        })
        .collect()
}

fn assign_generated_footprint_to_region(
    regions: &mut [GeneratedRegion],
    rooms: &[GeneratedRoom],
    anchor: Position,
    footprint: impl IntoIterator<Item = Position>,
) {
    let footprint = footprint.into_iter().collect::<BTreeSet<_>>();
    let Some(region_index) = regions
        .iter()
        .enumerate()
        .min_by_key(|(region_index, region)| {
            let distance = region
                .room_ids
                .iter()
                .filter_map(|room_id| rooms.iter().find(|room| room.id == *room_id))
                .map(|room| {
                    let center = room.center();
                    anchor.x.abs_diff(center.x) + anchor.y.abs_diff(center.y)
                })
                .min()
                .unwrap_or(u32::MAX);
            (distance, *region_index)
        })
        .map(|(region_index, _)| region_index)
    else {
        return;
    };
    for region in regions.iter_mut() {
        region
            .state
            .cells
            .retain(|position| !footprint.contains(position));
    }
    regions[region_index].state.cells.extend(footprint);
}

fn carve_generated_room(
    terrain: &mut [String],
    width: u16,
    room: &GeneratedRoom,
    floor_terrain_id: &str,
) {
    for y in room.y..room.y + room.height {
        for x in room.x..room.x + room.width {
            let position = Position { x, y };
            if room.contains(position) {
                set_generated_terrain(terrain, width, position, floor_terrain_id);
            }
        }
    }
}

fn carve_generated_corridor(
    terrain: &mut [String],
    width: u16,
    from: Position,
    to: Position,
    floor_terrain_id: &str,
) {
    for x in from.x.min(to.x)..=from.x.max(to.x) {
        set_generated_terrain(terrain, width, Position { x, y: from.y }, floor_terrain_id);
    }
    for y in from.y.min(to.y)..=from.y.max(to.y) {
        set_generated_terrain(terrain, width, Position { x: to.x, y }, floor_terrain_id);
    }
}

fn terrain_feature_placement_candidates(
    terrain: &[String],
    width: u16,
    floor_terrain_id: &str,
    room_floor_terrain_ids: &BTreeSet<String>,
    rooms: &[GeneratedRoom],
    reserved: &BTreeSet<Position>,
    placement: TerrainFeaturePlacement,
) -> Vec<Position> {
    terrain
        .iter()
        .enumerate()
        .filter_map(|(index, terrain_id)| {
            let position = Position {
                x: i32::try_from(index % usize::from(width))
                    .expect("terrain feature x must fit i32"),
                y: i32::try_from(index / usize::from(width))
                    .expect("terrain feature y must fit i32"),
            };
            if reserved.contains(&position) {
                return None;
            }
            let inside_room = rooms.iter().any(|room| room.contains(position));
            match placement {
                TerrainFeaturePlacement::Room
                    if inside_room
                        && (room_floor_terrain_ids.is_empty()
                            && terrain_id == floor_terrain_id
                            || room_floor_terrain_ids.contains(terrain_id)) =>
                {
                    Some(position)
                }
                TerrainFeaturePlacement::Corridor
                    if !inside_room && terrain_id == floor_terrain_id =>
                {
                    Some(position)
                }
                _ => None,
            }
        })
        .collect()
}

fn set_generated_terrain(terrain: &mut [String], width: u16, position: Position, terrain_id: &str) {
    let index = generated_terrain_index(width, position);
    terrain[index] = terrain_id.to_owned();
}

fn place_generated_floor_connections(
    definition: &ProceduralFloorDefinition,
    entry_anchor: Position,
    down_stair_anchor: Position,
    fixed_trap_position: Position,
    floor_terrain_id: &str,
    terrain: &mut [String],
    rng: &mut RfbRng,
) -> Result<Vec<FloorConnectionState>, CoreError> {
    let terrain_ref: &[String] = terrain;
    let mut candidates = (1..definition.height - 1)
        .flat_map(|y| {
            (1..definition.width - 1).filter_map(move |x| {
                let position = Position {
                    x: i32::from(x),
                    y: i32::from(y),
                };
                (position != fixed_trap_position
                    && terrain_ref[generated_terrain_index(definition.width, position)]
                        == floor_terrain_id)
                    .then_some(position)
            })
        })
        .collect::<Vec<_>>();
    let (primary_up_id, primary_down_id) = primary_floor_connection_ids(definition);
    let mut ordered_connections = Vec::with_capacity(definition.connections.len());
    for connection_id in [primary_up_id, primary_down_id].into_iter().flatten() {
        ordered_connections.push(
            definition
                .connections
                .iter()
                .find(|connection| connection.id == connection_id)
                .expect("selected primary connection must remain available"),
        );
    }
    ordered_connections.extend(definition.connections.iter().filter(|connection| {
        primary_up_id != Some(connection.id.as_str())
            && primary_down_id != Some(connection.id.as_str())
    }));

    let mut placed = Vec::with_capacity(definition.connections.len());
    for connection in ordered_connections {
        let position = if primary_up_id == Some(connection.id.as_str()) {
            entry_anchor
        } else if primary_down_id == Some(connection.id.as_str()) {
            down_stair_anchor
        } else {
            if candidates.is_empty() {
                return Err(CoreError::InvalidSave(
                    "generated floor has insufficient connection space",
                ));
            }
            let candidate_index = usize::try_from(rng.bounded(candidates.len() as u64))
                .expect("bounded connection index must fit usize");
            candidates[candidate_index]
        };
        candidates.retain(|candidate| *candidate != position);
        set_generated_terrain(terrain, definition.width, position, &connection.terrain_id);
        placed.push(FloorConnectionState {
            id: connection.id.clone(),
            position,
            target_floor_id: None,
            target_connection_id: None,
        });
    }
    placed.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(placed)
}

fn primary_floor_connection_ids(
    definition: &ProceduralFloorDefinition,
) -> (Option<&str>, Option<&str>) {
    let primary_up_id = definition.entry_connection_id.as_deref().or_else(|| {
        definition
            .connections
            .iter()
            .find(|connection| connection.terrain_id == definition.up_stair_terrain_id)
            .map(|connection| connection.id.as_str())
    });
    let primary_down_id = definition
        .down_stair_terrain_id
        .as_ref()
        .and_then(|terrain_id| {
            definition
                .connections
                .iter()
                .find(|connection| {
                    connection.terrain_id == *terrain_id
                        && primary_up_id != Some(connection.id.as_str())
                })
                .map(|connection| connection.id.as_str())
        });
    (primary_up_id, primary_down_id)
}

fn floor_regions_are_valid(
    floor_id: &str,
    dimensions: (u16, u16),
    regions: &[FloorRegionState],
    entities: &[Actor],
    items: &[ItemInstance],
    world: &rfb_content::WorldDefinition,
    content: &ContentCatalog,
) -> bool {
    let (width, height) = dimensions;
    if regions.is_empty() {
        return true;
    }
    let Some(definition) = world
        .procedural_floors
        .iter()
        .find(|definition| definition.id == floor_id)
    else {
        return false;
    };
    let Some(table) = definition
        .region_table_id
        .as_deref()
        .and_then(|table_id| content.region_table(table_id))
    else {
        return false;
    };
    let expected_count = definition
        .generation_budget
        .as_ref()
        .and_then(|budget| budget.region_placements)
        .map(usize::from);
    if expected_count != Some(regions.len())
        || regions
            .windows(2)
            .any(|pair| pair[0].region_id >= pair[1].region_id)
    {
        return false;
    }
    let mut cells = BTreeSet::new();
    for region in regions {
        let candidate_is_valid = table.entries.iter().any(|entry| {
            entry.region_id == region.region_id
                && entry.theme_id == region.theme_id
                && entry.encounter_table_id == region.encounter_table_id
                && entry.loot_table_id == region.loot_table_id
                && entry.min_depth <= definition.depth
                && definition.depth <= entry.max_depth
        });
        if !candidate_is_valid
            || region.cells.is_empty()
            || region.cells.windows(2).any(|pair| pair[0] >= pair[1])
            || region.cells.iter().any(|position| {
                position.x < 0
                    || position.y < 0
                    || position.x >= i32::from(width)
                    || position.y >= i32::from(height)
                    || !cells.insert(*position)
            })
        {
            return false;
        }
    }
    if entities
        .iter()
        .any(|entity| !cells.contains(&entity.position))
    {
        return false;
    }
    items.iter().all(|item| match &item.location {
        ItemLocation::Ground(position) => cells.contains(position),
        ItemLocation::CarriedBy { actor_id } => entities
            .iter()
            .find(|entity| &entity.id == actor_id)
            .is_some_and(|entity| cells.contains(&entity.position)),
        ItemLocation::Inventory | ItemLocation::Equipped { .. } => true,
    })
}

fn floor_connections_are_valid(
    floor_id: &str,
    width: u16,
    height: u16,
    terrain: &[String],
    connections: &[FloorConnectionState],
    world: &rfb_content::WorldDefinition,
) -> bool {
    if connections.is_empty() {
        return true;
    }
    if floor_id == world.initial_floor_id {
        return false;
    }
    let Some(definition) = world
        .procedural_floors
        .iter()
        .find(|definition| definition.id == floor_id)
    else {
        return false;
    };
    if definition.connections.len() != connections.len() {
        return false;
    }
    let expected_ids = definition
        .connections
        .iter()
        .map(|connection| connection.id.as_str())
        .collect::<BTreeSet<_>>();
    let actual_ids = connections
        .iter()
        .map(|connection| connection.id.as_str())
        .collect::<BTreeSet<_>>();
    let unique_positions = connections
        .iter()
        .map(|connection| connection.position)
        .collect::<BTreeSet<_>>();
    expected_ids == actual_ids
        && unique_positions.len() == connections.len()
        && connections.iter().all(|state| {
            let position = state.position;
            let Some(connection) = definition
                .connections
                .iter()
                .find(|connection| connection.id == state.id)
            else {
                return false;
            };
            position.x >= 0
                && position.y >= 0
                && position.x < i32::from(width)
                && position.y < i32::from(height)
                && terrain
                    .get(position.y as usize * usize::from(width) + position.x as usize)
                    .is_some_and(|terrain_id| terrain_id == &connection.terrain_id)
                && floor_connection_target_is_valid(floor_id, connection, state, world)
        })
}

fn floor_connection_target_is_valid(
    floor_id: &str,
    connection: &rfb_content::ProceduralFloorConnectionDefinition,
    state: &FloorConnectionState,
    world: &rfb_content::WorldDefinition,
) -> bool {
    match (&state.target_floor_id, &state.target_connection_id) {
        (None, None) => true,
        (Some(target_floor_id), None) => {
            target_floor_id == &world.initial_floor_id
                && connection.target_floor_id == world.initial_floor_id
                && connection.target_connection_id.is_none()
        }
        (Some(target_floor_id), Some(target_connection_id)) => {
            let directly_declared = (connection.target_floor_id == *target_floor_id
                && connection.target_connection_id.as_deref() == Some(target_connection_id))
                || connection.target_candidates.iter().any(|candidate| {
                    candidate.target_floor_id == *target_floor_id
                        && candidate.target_connection_id == *target_connection_id
                });
            if directly_declared {
                return true;
            }
            world
                .procedural_floors
                .iter()
                .find(|floor| floor.id == *target_floor_id)
                .and_then(|floor| {
                    floor
                        .connections
                        .iter()
                        .find(|candidate| candidate.id == *target_connection_id)
                })
                .is_some_and(|parent_connection| {
                    (parent_connection.target_floor_id == floor_id
                        && parent_connection.target_connection_id.as_deref()
                            == Some(state.id.as_str()))
                        || parent_connection.target_candidates.iter().any(|candidate| {
                            candidate.target_floor_id == floor_id
                                && candidate.target_connection_id == state.id
                        })
                })
        }
        _ => false,
    }
}

fn generated_terrain_index(width: u16, position: Position) -> usize {
    position.y as usize * usize::from(width) + position.x as usize
}

fn generated_wall_positions(
    definition: &ProceduralFloorDefinition,
    terrain: &[String],
) -> Vec<Position> {
    let mut positions = (1..definition.height - 1)
        .flat_map(|y| {
            (1..definition.width - 1).filter_map(move |x| {
                let position = Position {
                    x: i32::from(x),
                    y: i32::from(y),
                };
                (terrain[generated_terrain_index(definition.width, position)]
                    == definition.wall_terrain_id)
                    .then_some(position)
            })
        })
        .collect::<Vec<_>>();
    positions.sort_by_key(|position| (position.y, position.x));
    positions
}

fn floor_position_is_walkable(
    floor: &FloorState,
    position: Position,
    content: &ContentCatalog,
) -> bool {
    if position.x < 0
        || position.y < 0
        || position.x >= i32::from(floor.width)
        || position.y >= i32::from(floor.height)
    {
        return false;
    }
    let index = position.y as usize * usize::from(floor.width) + position.x as usize;
    floor
        .terrain
        .get(index)
        .and_then(|terrain_id| content.terrain(terrain_id))
        .is_some_and(|terrain| terrain.walkable)
}

fn revealed_terrain_is_valid(
    revealed: &BTreeSet<Position>,
    terrain: &[String],
    width: u16,
    height: u16,
    content: &ContentCatalog,
) -> bool {
    revealed.iter().all(|position| {
        if position.x < 0
            || position.y < 0
            || position.x >= i32::from(width)
            || position.y >= i32::from(height)
        {
            return false;
        }
        let index = position.y as usize * usize::from(width) + position.x as usize;
        terrain
            .get(index)
            .and_then(|terrain_id| content.terrain(terrain_id))
            .is_some_and(|definition| definition.concealed_as_terrain_id.is_some())
    })
}

fn source_intensity(source: Position, target: Position, radius: i32, maximum: u8) -> u8 {
    let distance = squared_distance(source, target);
    let radius_squared = radius * radius;
    if distance > radius_squared {
        return 0;
    }
    let remaining = radius_squared - distance;
    u8::try_from(
        (u32::from(maximum) * u32::try_from(remaining).unwrap_or(0))
            / u32::try_from(radius_squared).unwrap_or(1),
    )
    .unwrap_or(maximum)
}

fn has_line_of_sight(game: &Game, from: Position, to: Position) -> bool {
    let mut x = from.x;
    let mut y = from.y;
    let dx = (to.x - from.x).abs();
    let dy = (to.y - from.y).abs();
    let step_x = if from.x < to.x { 1 } else { -1 };
    let step_y = if from.y < to.y { 1 } else { -1 };
    let mut error = dx - dy;

    loop {
        if x == to.x && y == to.y {
            return true;
        }
        if !(x == from.x && y == from.y)
            && game
                .index(Position { x, y })
                .and_then(|index| game.content.terrain(&game.terrain[index]))
                .is_some_and(|terrain| terrain.blocks_sight)
        {
            return false;
        }
        let double_error = error * 2;
        if double_error > -dy {
            error -= dy;
            x += step_x;
        }
        if double_error < dx {
            error += dx;
            y += step_y;
        }
    }
}

pub fn load_built_in_content() -> Result<Arc<ContentCatalog>, CoreError> {
    Ok(Arc::new(ContentCatalog::from_bytes(
        BUILT_IN_CONTENT_BYTES,
    )?))
}

#[cfg(test)]
mod tests {
    use crate::effect::StatusInstance;
    use rfb_protocol::{
        DamageTypeDto, Direction, GameCommand, GameCommandEnvelope, GameEventOutcomeDto,
        ResistanceLevelDto, ResistanceSaveDto, StatusSaveDto, VisibilityState,
    };

    use super::*;

    fn command(seq: u32, revision: u32, command: GameCommand) -> GameCommandEnvelope {
        GameCommandEnvelope {
            command_seq: seq,
            expected_revision: revision,
            command,
        }
    }

    fn dispatch_next(game: &mut Game, command_value: GameCommand) -> GameUpdate {
        let snapshot = game.snapshot();
        game.dispatch(command(
            snapshot.last_command_seq + 1,
            snapshot.revision,
            command_value,
        ))
        .expect("test command should execute")
    }

    fn descend_one_floor(game: &mut Game) {
        if game.current_floor_id == "demo.floor.surface" {
            game.player.position = Position { x: 3, y: 4 };
        } else {
            let down_index = game
                .terrain
                .iter()
                .position(|terrain_id| terrain_id == "demo.terrain.stairs-down")
                .expect("current floor should contain descending stairs");
            game.player.position = Position {
                x: i32::try_from(down_index % usize::from(game.width))
                    .expect("descending stair x must fit i32"),
                y: i32::try_from(down_index / usize::from(game.width))
                    .expect("descending stair y must fit i32"),
            };
        }
        game.traverse_stairs(false)
            .expect("descent should resolve")
            .expect("descent should transition");
    }

    fn connection_position(game: &Game, connection_id: &str) -> Position {
        game.floor_connections
            .iter()
            .find(|connection| connection.id == connection_id)
            .unwrap_or_else(|| panic!("floor should contain connection {connection_id}"))
            .position
    }

    fn traverse_connection(game: &mut Game, connection_id: &str) {
        game.player.position = connection_position(game, connection_id);
        game.traverse_stairs(false)
            .expect("connection traversal should resolve")
            .expect("connection traversal should transition");
    }

    fn stored_floor<'a>(game: &'a Game, floor_id: &str) -> &'a FloorState {
        game.stored_floors
            .values()
            .find(|floor| floor.id == floor_id)
            .unwrap_or_else(|| panic!("stored floor {floor_id} should exist"))
    }

    fn region_at(game: &Game, position: Position) -> &FloorRegionState {
        game.floor_regions
            .iter()
            .find(|region| region.cells.contains(&position))
            .unwrap_or_else(|| panic!("position {position:?} should belong to a floor region"))
    }

    fn visual_at(snapshot: &GameSnapshot, position: Position) -> CellVisualDto {
        *snapshot
            .visual_cells
            .iter()
            .find(|visual| visual.position == position)
            .expect("snapshot should contain every visual cell")
    }

    #[test]
    fn built_in_game_is_created_from_the_compiled_content_pack() {
        let snapshot = Game::new(42).snapshot();
        let shard = snapshot
            .items
            .iter()
            .find(|item| item.id == "demo.item.luminous-shard.1")
            .expect("compiled world should spawn its item");

        assert_eq!(snapshot.content_id, "rfb.demo.original-v1");
        assert_eq!(snapshot.content_hash, BUILT_IN_CONTENT_HASH);
        assert_eq!(snapshot.world_id, BUILT_IN_WORLD_ID);
        assert_eq!(
            snapshot.player.melee_damage.damage_type,
            DamageTypeDto::Physical
        );
        assert_eq!(
            snapshot.entities[0].melee_damage.damage_type,
            DamageTypeDto::Fire
        );
        assert_eq!(snapshot.player.id, "demo.actor.player.1");
        assert_eq!(snapshot.player.kind_id, "demo.actor.explorer");
        assert_eq!(snapshot.player.base_attack, 2);
        assert_eq!(snapshot.player.attack, 2);
        assert_eq!(snapshot.player.base_defense, 1);
        assert_eq!(snapshot.player.defense, 1);
        assert!(snapshot.inventory.is_empty());
        assert!(snapshot.equipment.is_empty());
        assert_eq!(snapshot.items.len(), 5);
        assert_eq!(snapshot.entities[0].position, Position { x: 8, y: 5 });
        assert_eq!(snapshot.entities[0].attack, 1);
        assert_eq!(snapshot.entities[0].defense, 1);
        assert_eq!(shard.position, Position { x: 4, y: 3 });
        assert_eq!(
            snapshot
                .cells
                .iter()
                .find(|cell| cell.position == shard.position)
                .and_then(|cell| cell.item_id.as_deref()),
            Some("demo.item.luminous-shard.1")
        );
        assert!(
            snapshot
                .content_visuals
                .iter()
                .any(|visual| visual.id == "demo.item.luminous-shard" && visual.glyph == "!")
        );
        assert_eq!(snapshot.visual_cells.len(), snapshot.cells.len());
        assert_eq!(
            visual_at(&snapshot, snapshot.player.position).visibility,
            VisibilityState::Visible
        );
        assert_eq!(
            visual_at(&snapshot, Position { x: 19, y: 19 }).visibility,
            VisibilityState::Hidden
        );
        assert_eq!(
            visual_at(&snapshot, Position { x: 8, y: 5 }).light.color,
            ACTOR_LIGHT_COLOR
        );
    }

    #[test]
    fn movement_produces_fov_deltas_and_remembers_explored_cells() {
        let mut game = Game::new(42);
        clear_monsters(&mut game);
        let first = game
            .dispatch(command(
                1,
                0,
                GameCommand::Move {
                    direction: Direction::East,
                },
            ))
            .expect("movement should execute");
        assert!(!first.changed_visual_cells.is_empty());
        let snapshot = game.snapshot();
        assert_eq!(
            visual_at(&snapshot, Position { x: 11, y: 3 }).visibility,
            VisibilityState::Visible
        );
        assert_eq!(
            visual_at(&snapshot, Position { x: 12, y: 3 }).visibility,
            VisibilityState::Hidden
        );

        for seq in 2..=7 {
            game.dispatch(command(
                seq,
                seq - 1,
                GameCommand::Move {
                    direction: Direction::East,
                },
            ))
            .expect("eastward exploration should execute");
        }
        assert_eq!(
            visual_at(&game.snapshot(), Position { x: 1, y: 3 }).visibility,
            VisibilityState::Remembered
        );
    }

    #[test]
    fn procedural_floor_transition_is_deterministic_persistent_and_reversible() {
        let mut left = Game::new(27);
        let mut right = Game::new(27);
        for game in [&mut left, &mut right] {
            game.player.position = Position { x: 3, y: 4 };
        }

        let left_update = left
            .dispatch(command(1, 0, GameCommand::TraverseStairs))
            .expect("descending should generate the first floor");
        let right_update = right
            .dispatch(command(1, 0, GameCommand::TraverseStairs))
            .expect("the same seed should generate the same floor");

        assert_eq!(left_update.floor_id, "demo.floor.echo-depth-1");
        assert_eq!(left_update.state_hash, right_update.state_hash);
        assert_eq!(left.rng.draw_counter, 21);
        assert_eq!(left.entities.len(), 4);
        let room_encounter = left
            .entities
            .iter()
            .find(|entity| entity.id == "demo.floor.echo-depth-1.encounter.1")
            .expect("floor encounter table should spawn its declared roll");
        assert_eq!(room_encounter.position, Position { x: 15, y: 11 });
        assert!(matches!(
            room_encounter.kind_id.as_str(),
            "demo.actor.acid-seep" | "demo.actor.echo-hound" | "demo.actor.frost-wisp"
        ));
        let nest = left
            .entities
            .iter()
            .filter(|entity| entity.id.starts_with("demo.floor.echo-depth-1.nest."))
            .collect::<Vec<_>>();
        assert_eq!(nest.len(), 3);
        assert!(nest.iter().all(|entity| entity.kind_id == nest[0].kind_id));
        assert!(nest.iter().all(|entity| !matches!(
            entity.kind_id.as_str(),
            "demo.actor.storm-spark" | "demo.actor.venom-spore"
        )));
        assert!(
            left.entities
                .iter()
                .all(|entity| !entity.id.contains(".vault-group."))
        );
        let floor_loot = left
            .items
            .iter()
            .find(|item| matches!(item.location, ItemLocation::Ground(_)))
            .expect("the generated floor should contain ground loot");
        assert_eq!(floor_loot.id, "generated.item.2");
        assert_eq!(floor_loot.kind_id, "demo.item.luminous-shard");
        assert_eq!(floor_loot.quantity, 2);
        assert_eq!(left.stored_floors.len(), 1);
        assert_eq!(
            left.terrain_at(left.player.position),
            "demo.terrain.stairs-up"
        );
        assert!(left_update.events.iter().any(|event| {
            event.kind == "floor.transition"
                && event.args["from"] == "demo.floor.surface"
                && event.args["to"] == "demo.floor.echo-depth-1"
        }));

        let mut restored = Game::from_save(left.to_save()).expect("generated floor should reload");
        assert_eq!(restored.state_hash(), left.state_hash());
        let return_update = restored
            .dispatch(command(2, 1, GameCommand::TraverseStairs))
            .expect("ascending should restore the entrance floor");
        assert_eq!(return_update.floor_id, "demo.floor.surface");
        assert_eq!(restored.player.position, Position { x: 3, y: 4 });
        assert_eq!(restored.entities.len(), 1);
        assert!(restored.stored_floors.is_empty());
        assert!(
            return_update
                .events
                .iter()
                .any(|event| event.message_key == "floor-expedition-ended")
        );

        let draws_before_reentry = restored.rng.draw_counter;
        let reentry_update = restored
            .dispatch(command(3, 2, GameCommand::TraverseStairs))
            .expect("descending again should generate a new expedition floor");
        assert_eq!(reentry_update.floor_id, "demo.floor.echo-depth-1");
        assert!(restored.rng.draw_counter > draws_before_reentry);
        assert_eq!(restored.entities.len(), 4);
        assert_eq!(restored.items.len(), 1);
    }

    #[test]
    fn dungeon_instances_are_numbered_and_old_instance_lifecycle_is_scoped() {
        let mut game = Game::new(27);
        game.player.position = Position { x: 3, y: 4 };
        game.traverse_stairs(false)
            .expect("first dungeon entry should resolve")
            .expect("first dungeon entry should transition");
        assert_eq!(
            game.current_dungeon_instance_id.as_deref(),
            Some("demo.dungeon.echo-depths.instance.1")
        );
        assert_eq!(
            game.stored_floors["demo.floor.surface"].dungeon_instance_id,
            None
        );
        let first_payload = game.to_save();
        assert_eq!(
            first_payload.current_dungeon_instance_id.as_deref(),
            Some("demo.dungeon.echo-depths.instance.1")
        );
        assert_eq!(
            first_payload
                .dungeon_states
                .iter()
                .find(|state| state.dungeon_id == "demo.dungeon.echo-depths")
                .map(|state| state.next_instance_ordinal),
            Some(1)
        );
        let mut legacy_v64_payload = first_payload.clone();
        legacy_v64_payload.current_dungeon_instance_id = None;
        for floor in &mut legacy_v64_payload.stored_floors {
            floor.dungeon_instance_id = None;
        }
        for state in &mut legacy_v64_payload.dungeon_states {
            state.next_instance_ordinal = 0;
        }
        let migrated = Game::from_save(legacy_v64_payload)
            .expect("v64 dungeon save should migrate its first instance");
        assert_eq!(
            migrated.current_dungeon_instance_id.as_deref(),
            Some("demo.dungeon.echo-depths.instance.1")
        );
        assert_eq!(
            migrated.dungeon_states["demo.dungeon.echo-depths"].next_instance_ordinal,
            1
        );
        assert_eq!(migrated.state_hash(), game.state_hash());

        traverse_connection(&mut game, "demo.connection.echo-depth-1.surface-up");
        assert_eq!(game.current_floor_id, "demo.floor.surface");
        assert!(game.stored_floors.values().all(|floor| {
            floor.dungeon_instance_id.as_deref() != Some("demo.dungeon.echo-depths.instance.1")
        }));

        game.player.position = Position { x: 3, y: 4 };
        game.traverse_stairs(false)
            .expect("second dungeon entry should resolve")
            .expect("second dungeon entry should transition");
        assert_eq!(
            game.current_dungeon_instance_id.as_deref(),
            Some("demo.dungeon.echo-depths.instance.2")
        );
        assert_eq!(
            game.dungeon_states["demo.dungeon.echo-depths"].next_instance_ordinal,
            2
        );
    }

    #[test]
    fn ending_a_dungeon_instance_does_not_clear_stored_task_floors() {
        let mut game = Game::new(27);
        game.player.position = Position { x: 3, y: 4 };
        game.traverse_stairs(false)
            .expect("dungeon entry should resolve")
            .expect("dungeon entry should transition");
        let task_definition = game
            .content
            .world(&game.world_id)
            .expect("world should remain available")
            .procedural_floors
            .iter()
            .find(|floor| floor.lifecycle == FloorLifecycle::OneShot)
            .cloned()
            .expect("demo should contain a task floor");
        let task_floor = game
            .generate_procedural_floor(&task_definition, None)
            .expect("task floor should generate for the fixture");
        game.stored_floors.insert(task_floor.id.clone(), task_floor);
        traverse_connection(&mut game, "demo.connection.echo-depth-1.surface-up");
        assert_eq!(game.current_floor_id, "demo.floor.surface");
        assert!(game.stored_floors.values().any(|floor| {
            floor.id == task_definition.id && floor.dungeon_instance_id.is_none()
        }));
    }

    #[test]
    fn floor_connections_are_seeded_distinct_and_content_authoritative() {
        let mut left = Game::new(27);
        let mut right = Game::new(27);
        for game in [&mut left, &mut right] {
            game.player.position = Position { x: 3, y: 4 };
            game.traverse_stairs(false)
                .expect("echo dungeon entry should resolve")
                .expect("echo dungeon entry should transition");
        }

        assert_eq!(left.floor_connections, right.floor_connections);
        assert_eq!(left.terrain, right.terrain);
        assert_eq!(left.floor_connections.len(), 4);
        assert_eq!(
            left.player.position,
            connection_position(&left, "demo.connection.echo-depth-1.surface-up")
        );
        let positions = left
            .floor_connections
            .iter()
            .map(|connection| connection.position)
            .collect::<BTreeSet<_>>();
        assert_eq!(positions.len(), left.floor_connections.len());
        for connection in &left.floor_connections {
            let definition = left
                .content
                .world(BUILT_IN_WORLD_ID)
                .expect("built-in world should exist")
                .procedural_floors
                .iter()
                .find(|floor| floor.id == left.current_floor_id)
                .expect("current procedural floor should exist")
                .connections
                .iter()
                .find(|candidate| candidate.id == connection.id)
                .expect("generated connection should exist in content");
            assert_eq!(left.terrain_at(connection.position), definition.terrain_id);
        }

        let layouts = (0..8)
            .map(|seed| {
                let mut game = Game::new(seed);
                game.player.position = Position { x: 3, y: 4 };
                game.traverse_stairs(false)
                    .expect("echo dungeon entry should resolve")
                    .expect("echo dungeon entry should transition");
                game.floor_connections
                    .iter()
                    .map(|connection| {
                        (
                            connection.id.clone(),
                            connection.position.x,
                            connection.position.y,
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<BTreeSet<_>>();
        assert!(layouts.len() > 1);
    }

    #[test]
    fn paired_stairs_and_shaft_use_independent_arrival_connections() {
        let mut game = Game::new(71);
        game.player.position = Position { x: 3, y: 4 };
        game.traverse_stairs(false)
            .expect("echo dungeon entry should resolve")
            .expect("echo dungeon entry should transition");

        let down_a = connection_position(&game, "demo.connection.echo-depth-1.down-a");
        let down_b = connection_position(&game, "demo.connection.echo-depth-1.down-b");
        traverse_connection(&mut game, "demo.connection.echo-depth-1.down-a");
        assert_eq!(game.current_floor_id, "demo.floor.echo-depth-2");
        assert_eq!(
            game.player.position,
            connection_position(&game, "demo.connection.echo-depth-2.up-a")
        );
        traverse_connection(&mut game, "demo.connection.echo-depth-2.up-a");
        assert_eq!(game.player.position, down_a);

        traverse_connection(&mut game, "demo.connection.echo-depth-1.down-b");
        assert_eq!(game.current_floor_id, "demo.floor.echo-depth-2-mirror");
        assert_eq!(
            game.player.position,
            connection_position(&game, "demo.connection.echo-depth-2-mirror.up-a")
        );
        traverse_connection(&mut game, "demo.connection.echo-depth-2-mirror.up-a");
        assert_eq!(game.player.position, down_b);

        let shaft_down = connection_position(&game, "demo.connection.echo-depth-1.shaft-down");
        traverse_connection(&mut game, "demo.connection.echo-depth-1.shaft-down");
        assert_eq!(game.current_floor_id, "demo.floor.echo-depth-3-shaft");
        assert_eq!(
            game.player.position,
            connection_position(&game, "demo.connection.echo-depth-3-shaft.shaft-up")
        );
        traverse_connection(&mut game, "demo.connection.echo-depth-3-shaft.shaft-up");
        assert_eq!(game.current_floor_id, "demo.floor.echo-depth-1");
        assert_eq!(game.player.position, shaft_down);
    }

    #[test]
    fn floor_connections_round_trip_and_reject_invalid_authoritative_state() {
        let mut game = Game::new(93);
        game.player.position = Position { x: 3, y: 4 };
        game.traverse_stairs(false)
            .expect("echo dungeon entry should resolve")
            .expect("echo dungeon entry should transition");
        let payload = game.to_save();
        let restored = Game::from_save(payload.clone()).expect("connections should round-trip");
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.floor_connections, game.floor_connections);

        let mut missing = payload.clone();
        missing.floor_connections.pop();
        assert!(Game::from_save(missing).is_err());

        let mut duplicate = payload.clone();
        duplicate.floor_connections[1].id = duplicate.floor_connections[0].id.clone();
        assert!(Game::from_save(duplicate).is_err());

        let mut mismatched_terrain = payload;
        let position = mismatched_terrain.floor_connections[0].position;
        let index = position.y as usize * usize::from(mismatched_terrain.terrain.width)
            + position.x as usize;
        mismatched_terrain.terrain.terrain_ids[index] = "demo.terrain.floor".to_owned();
        assert!(Game::from_save(mismatched_terrain).is_err());

        let mut undeclared_target = game.to_save();
        undeclared_target.floor_connections[0].target_floor_id =
            Some("demo.floor.echo-depth-3".to_owned());
        undeclared_target.floor_connections[0].target_connection_id =
            Some("demo.connection.echo-depth-3.up-a".to_owned());
        assert!(Game::from_save(undeclared_target).is_err());
    }

    #[test]
    fn dynamic_connection_targets_form_distinct_branches_and_survive_reload() {
        let mut game = Game::new(27);
        game.player.position = Position { x: 3, y: 4 };
        game.traverse_stairs(false)
            .expect("echo dungeon entry should resolve")
            .expect("echo dungeon entry should transition");
        let down_a = game
            .floor_connections
            .iter()
            .find(|connection| connection.id == "demo.connection.echo-depth-1.down-a")
            .expect("dynamic down-a should exist")
            .clone();
        let down_b = game
            .floor_connections
            .iter()
            .find(|connection| connection.id == "demo.connection.echo-depth-1.down-b")
            .expect("dynamic down-b should exist")
            .clone();
        assert!(down_a.target_floor_id.is_some());
        assert!(down_b.target_floor_id.is_some());
        assert_ne!(down_a.target_floor_id, down_b.target_floor_id);

        let payload = game.to_save();
        let restored = Game::from_save(payload.clone()).expect("dynamic targets should reload");
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.floor_connections, game.floor_connections);

        let mut legacy = payload;
        for connection in &mut legacy.floor_connections {
            connection.target_floor_id = None;
            connection.target_connection_id = None;
        }
        let legacy = Game::from_save(legacy).expect("missing target fields should use content");
        assert!(
            legacy
                .floor_connections
                .iter()
                .all(|connection| connection.target_floor_id.is_none())
        );
    }

    #[test]
    fn previous_v57_floor_without_connection_state_uses_legacy_stairs_without_rebuild() {
        let mut game = Game::new(117);
        game.player.position = Position { x: 3, y: 4 };
        game.traverse_stairs(false)
            .expect("echo dungeon entry should resolve")
            .expect("echo dungeon entry should transition");
        let down_a = connection_position(&game, "demo.connection.echo-depth-1.down-a");
        let surface_up = connection_position(&game, "demo.connection.echo-depth-1.surface-up");
        let mut payload = game.to_save();
        for connection in &payload.floor_connections {
            let index = connection.position.y as usize * usize::from(payload.terrain.width)
                + connection.position.x as usize;
            payload.terrain.terrain_ids[index] = if connection.position == down_a {
                "demo.terrain.stairs-down".to_owned()
            } else if connection.position == surface_up {
                "demo.terrain.stairs-up".to_owned()
            } else {
                "demo.terrain.floor".to_owned()
            };
        }
        payload.player.position = down_a;
        payload.floor_connections.clear();
        payload.content_hash =
            "d209d68a6a39af21eee8d1a951684be86e847ab570823c9c2604fa199e4571e1".to_owned();
        let expected_terrain = payload.terrain.clone();
        let expected_entities = payload.entities.clone();
        let expected_items = payload.items.clone();
        let saved_draw_counter = payload.rng.draw_counter;

        let mut restored = Game::from_save(payload).expect("v57 floor should migrate");
        assert_eq!(restored.to_save().terrain, expected_terrain);
        assert_eq!(actors_to_save(&restored.entities), expected_entities);
        assert_eq!(items_to_save(&restored.items), expected_items);
        assert_eq!(restored.rng.draw_counter, saved_draw_counter);
        assert!(restored.floor_connections.is_empty());

        restored
            .traverse_stairs(false)
            .expect("legacy stairs should resolve")
            .expect("legacy stairs should transition");
        assert_eq!(restored.current_floor_id, "demo.floor.echo-depth-2");
    }

    #[test]
    fn locked_door_checks_update_collision_visibility_and_persist() {
        let mut game = Game::new(27);
        game.player.position = Position { x: 3, y: 4 };
        game.dispatch(command(1, 0, GameCommand::TraverseStairs))
            .expect("descending should generate the closed door");
        let door_position = Position { x: 10, y: 4 };
        assert_eq!(game.terrain_at(door_position), "demo.terrain.door-secret");
        assert!(!game.is_walkable(door_position));

        game.player.position = Position { x: 9, y: 4 };
        game.revealed_terrain.insert(door_position);
        assert_eq!(
            visual_at(&game.snapshot(), Position { x: 11, y: 4 }).visibility,
            VisibilityState::Hidden
        );
        let draws_before_unlock = game.rng.draw_counter;
        let mut saw_failed_unlock = false;
        let open_update = (0..12)
            .find_map(|_| {
                let update = dispatch_next(
                    &mut game,
                    GameCommand::OpenDoor {
                        direction: Direction::East,
                    },
                );
                saw_failed_unlock |= update
                    .events
                    .iter()
                    .any(|event| event.kind == "terrain.door-unlock-failed");
                (game.terrain_at(door_position) == "demo.terrain.door-open").then_some(update)
            })
            .expect("fixed seed should eventually unlock the door");
        assert!(saw_failed_unlock);
        assert_eq!(game.terrain_at(door_position), "demo.terrain.door-open");
        assert!(game.is_walkable(door_position));
        assert!(game.rng.draw_counter > draws_before_unlock);
        let terrain_events = open_update
            .events
            .iter()
            .filter(|event| event.kind.starts_with("terrain."))
            .map(|event| event.kind.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            terrain_events,
            ["terrain.door-unlocked", "terrain.door-opened"]
        );
        assert_eq!(
            visual_at(&game.snapshot(), Position { x: 11, y: 4 }).visibility,
            VisibilityState::Visible
        );

        let mut restored = Game::from_save(game.to_save()).expect("open door should reload");
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.terrain_at(door_position), "demo.terrain.door-open");

        restored.player.position = Position { x: 5, y: 4 };
        dispatch_next(&mut restored, GameCommand::TraverseStairs);
        dispatch_next(&mut restored, GameCommand::TraverseStairs);
        let fresh_door_index = restored
            .terrain
            .iter()
            .position(|terrain_id| terrain_id == "demo.terrain.door-secret")
            .expect("fresh floor should contain a secret door");
        let fresh_door_position = Position {
            x: i32::try_from(fresh_door_index % usize::from(restored.width))
                .expect("door x must fit i32"),
            y: i32::try_from(fresh_door_index / usize::from(restored.width))
                .expect("door y must fit i32"),
        };
        assert_eq!(
            restored.terrain_at(fresh_door_position),
            "demo.terrain.door-secret"
        );

        restored.player.position = Position {
            x: fresh_door_position.x - 1,
            y: fresh_door_position.y,
        };
        let close_update = dispatch_next(
            &mut restored,
            GameCommand::CloseDoor {
                direction: Direction::East,
            },
        );
        assert_eq!(
            restored.terrain_at(fresh_door_position),
            "demo.terrain.door-secret"
        );
        assert!(
            close_update
                .events
                .iter()
                .any(|event| event.kind == "terrain.door-close-unavailable")
        );

        let unavailable = dispatch_next(
            &mut restored,
            GameCommand::CloseDoor {
                direction: Direction::East,
            },
        );
        assert!(
            unavailable
                .events
                .iter()
                .any(|event| event.kind == "terrain.door-close-unavailable")
        );
    }

    #[test]
    fn bashing_a_locked_door_is_deterministic_and_leaves_a_broken_door() {
        let mut game = Game::new(0);
        game.player.position = Position { x: 3, y: 4 };
        game.dispatch(command(1, 0, GameCommand::TraverseStairs))
            .expect("descending should generate the locked door");
        game.player.position = Position { x: 9, y: 4 };
        let door_position = Position { x: 10, y: 4 };
        game.revealed_terrain.insert(door_position);
        let draws_before_bash = game.rng.draw_counter;
        let mut saw_failed_bash = false;
        let succeeded = (0..12)
            .find_map(|_| {
                let update = dispatch_next(
                    &mut game,
                    GameCommand::BashDoor {
                        direction: Direction::East,
                    },
                );
                saw_failed_bash |= update
                    .events
                    .iter()
                    .any(|event| event.kind == "terrain.door-bash-failed");
                (game.terrain_at(door_position) == "demo.terrain.door-broken").then_some(update)
            })
            .expect("fixed seed should eventually bash the door open");
        assert!(saw_failed_bash);
        assert_eq!(game.terrain_at(door_position), "demo.terrain.door-broken");
        assert!(game.is_walkable(door_position));
        assert!(game.rng.draw_counter > draws_before_bash);
        assert!(
            succeeded
                .events
                .iter()
                .any(|event| event.kind == "terrain.door-bashed-open")
        );

        let mut restored = Game::from_save(game.to_save()).expect("broken door should reload");
        assert_eq!(
            restored.terrain_at(door_position),
            "demo.terrain.door-broken"
        );
        let unavailable = dispatch_next(
            &mut restored,
            GameCommand::CloseDoor {
                direction: Direction::East,
            },
        );
        assert!(
            unavailable
                .events
                .iter()
                .any(|event| event.kind == "terrain.door-close-unavailable")
        );
    }

    #[test]
    fn terrain_interaction_query_is_stable_and_reports_blockers() {
        let mut game = Game::new(27);
        game.player.position = Position { x: 3, y: 4 };
        game.dispatch(command(1, 0, GameCommand::TraverseStairs))
            .expect("descending should generate the locked door");
        game.player.position = Position { x: 9, y: 4 };
        let door_position = Position { x: 10, y: 4 };

        assert!(game.snapshot().terrain_interactions.is_empty());
        assert_eq!(game.known_terrain_at(door_position), "demo.terrain.wall");
        game.revealed_terrain.insert(door_position);
        let locked = game.snapshot().terrain_interactions;
        assert_eq!(locked.len(), 2);
        assert_eq!(
            locked
                .iter()
                .map(|interaction| (
                    interaction.kind,
                    interaction.direction,
                    interaction.position,
                    interaction.terrain_id.as_str(),
                    interaction.requires_check,
                    interaction.available,
                    interaction.unavailable_reason,
                ))
                .collect::<Vec<_>>(),
            [
                (
                    TerrainInteractionKindDto::OpenDoor,
                    Direction::East,
                    door_position,
                    "demo.terrain.door-secret",
                    true,
                    true,
                    None,
                ),
                (
                    TerrainInteractionKindDto::BashDoor,
                    Direction::East,
                    door_position,
                    "demo.terrain.door-secret",
                    true,
                    true,
                    None,
                ),
            ]
        );

        (0..12)
            .find(|_| {
                dispatch_next(
                    &mut game,
                    GameCommand::OpenDoor {
                        direction: Direction::East,
                    },
                );
                game.terrain_at(door_position) == "demo.terrain.door-open"
            })
            .expect("fixed seed should eventually unlock the queried door");
        let open = game.snapshot().terrain_interactions;
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].kind, TerrainInteractionKindDto::CloseDoor);
        assert!(!open[0].requires_check);
        assert!(open[0].available);

        game.items[0].location = ItemLocation::Ground(door_position);
        let blocked_by_item = game.snapshot().terrain_interactions;
        assert!(!blocked_by_item[0].available);
        assert_eq!(
            blocked_by_item[0].unavailable_reason,
            Some(TerrainInteractionUnavailableReasonDto::OccupiedByItem)
        );

        game.entities[0].position = door_position;
        let blocked_by_actor = game.snapshot().terrain_interactions;
        assert!(!blocked_by_actor[0].available);
        assert_eq!(
            blocked_by_actor[0].unavailable_reason,
            Some(TerrainInteractionUnavailableReasonDto::OccupiedByActor)
        );
    }

    #[test]
    fn search_discovers_secret_terrain_without_leaking_true_terrain() {
        let mut game = Game::new(27);
        game.player.position = Position { x: 3, y: 4 };
        game.dispatch(command(1, 0, GameCommand::TraverseStairs))
            .expect("descending should generate the secret door");
        game.player.position = Position { x: 9, y: 4 };
        let door_position = Position { x: 10, y: 4 };
        assert_eq!(game.terrain_at(door_position), "demo.terrain.door-secret");
        assert_eq!(game.known_terrain_at(door_position), "demo.terrain.wall");
        assert!(game.snapshot().terrain_interactions.is_empty());
        let draws_before_search = game.rng.draw_counter;

        let hidden_open = game
            .dispatch(command(
                2,
                1,
                GameCommand::OpenDoor {
                    direction: Direction::East,
                },
            ))
            .expect("an undiscovered secret door should reject direct opening");
        assert_eq!(game.rng.draw_counter, draws_before_search);
        assert!(
            hidden_open
                .events
                .iter()
                .any(|event| event.kind == "terrain.door-open-unavailable")
        );

        let discovered = (0..12)
            .find_map(|_| {
                let update = dispatch_next(&mut game, GameCommand::Search);
                game.revealed_terrain
                    .contains(&door_position)
                    .then_some(update)
            })
            .expect("fixed seed should eventually discover the secret door");
        assert!(game.rng.draw_counter > draws_before_search);
        assert_eq!(
            game.known_terrain_at(door_position),
            "demo.terrain.door-secret"
        );
        assert!(game.revealed_terrain.contains(&door_position));
        assert!(
            discovered
                .events
                .iter()
                .any(|event| event.kind == "terrain.secret-discovered")
        );
        assert_eq!(discovered.terrain_interactions.len(), 2);
        assert!(
            discovered
                .changed_cells
                .iter()
                .any(|cell| cell.position == door_position
                    && cell.terrain_id == "demo.terrain.door-secret")
        );
        let mut hidden_again = game.clone();
        hidden_again.revealed_terrain.clear();
        assert_ne!(hidden_again.state_hash(), game.state_hash());

        let mut restored =
            Game::from_save(game.to_save()).expect("terrain knowledge should reload");
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(
            restored.known_terrain_at(door_position),
            "demo.terrain.door-secret"
        );
        restored.player.position = Position { x: 5, y: 4 };
        dispatch_next(&mut restored, GameCommand::TraverseStairs);
        dispatch_next(&mut restored, GameCommand::TraverseStairs);
        let fresh_door_index = restored
            .terrain
            .iter()
            .position(|terrain_id| terrain_id == "demo.terrain.door-secret")
            .expect("fresh floor should contain a secret door");
        let fresh_door_position = Position {
            x: i32::try_from(fresh_door_index % usize::from(restored.width))
                .expect("door x must fit i32"),
            y: i32::try_from(fresh_door_index / usize::from(restored.width))
                .expect("door y must fit i32"),
        };
        assert_eq!(
            restored.known_terrain_at(fresh_door_position),
            "demo.terrain.wall"
        );
    }

    #[test]
    fn stairs_command_off_stairs_keeps_the_current_floor() {
        let mut game = Game::new(42);
        let update = game
            .dispatch(command(1, 0, GameCommand::TraverseStairs))
            .expect("unavailable stairs command should remain a valid turn");

        assert_eq!(update.floor_id, "demo.floor.surface");
        assert!(
            update
                .events
                .iter()
                .any(|event| event.kind == "floor.transition-unavailable")
        );
        assert!(game.stored_floors.is_empty());
    }

    #[test]
    fn exploration_memory_does_not_change_authoritative_state_hash() {
        let mut game = Game::new(42);
        let before = game.state_hash();
        game.explored.fill(true);
        assert_eq!(game.state_hash(), before);

        game.player.position = Position { x: 3, y: 4 };
        game.dispatch(command(1, 0, GameCommand::TraverseStairs))
            .expect("descending should store the entrance floor");
        let before_stored_memory_change = game.state_hash();
        game.stored_floors
            .get_mut("demo.floor.surface")
            .expect("the entrance floor should be stored")
            .explored
            .fill(false);
        assert_eq!(game.state_hash(), before_stored_memory_change);
    }

    #[test]
    fn malformed_exploration_memory_is_rejected() {
        let mut payload = Game::new(42).to_save();
        payload.explored.pop();
        assert!(matches!(
            Game::from_save(payload),
            Err(CoreError::InvalidSave(
                "exploration memory dimensions are invalid"
            ))
        ));
    }

    #[test]
    fn malformed_revealed_terrain_knowledge_is_rejected() {
        let mut payload = Game::new(42).to_save();
        payload.revealed_terrain = vec![Position { x: 3, y: 3 }];
        assert!(matches!(
            Game::from_save(payload),
            Err(CoreError::InvalidSave(
                "revealed terrain knowledge is invalid"
            ))
        ));
    }

    #[test]
    fn haste_and_slow_modify_scheduler_speed_without_changing_base_speed() {
        let mut haste_payload = Game::new(42).to_save();
        haste_payload.player.statuses = vec![StatusSaveDto {
            kind_id: STATUS_HASTE.to_owned(),
            intensity: 1,
            remaining_ticks: 20,
            source_id: None,
        }];
        let mut haste = Game::from_save(haste_payload).expect("haste setup should load");
        assert_eq!(haste.snapshot().player.speed, 120);
        let haste_update = haste
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("hasted wait should execute");
        assert_eq!(haste_update.world_tick, 5);
        assert_eq!(haste_update.player.speed, 120);
        assert_eq!(haste.to_save().player.base_speed, 110);
        assert_eq!(haste_update.player.statuses[0].remaining_ticks, 15);

        let mut slow_payload = Game::new(42).to_save();
        slow_payload.player.statuses = vec![StatusSaveDto {
            kind_id: STATUS_SLOW.to_owned(),
            intensity: 1,
            remaining_ticks: 40,
            source_id: None,
        }];
        let mut slow = Game::from_save(slow_payload).expect("slow setup should load");
        let slow_update = slow
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("slowed wait should execute");
        assert_eq!(slow_update.world_tick, 20);
        assert_eq!(slow_update.player.speed, 100);
        assert_eq!(slow_update.player.statuses[0].remaining_ticks, 20);
    }

    #[test]
    fn poison_uses_resistance_then_expires_and_round_trips() {
        let mut payload = Game::new(42).to_save();
        payload.player.statuses = vec![StatusSaveDto {
            kind_id: STATUS_POISON.to_owned(),
            intensity: 2,
            remaining_ticks: 3,
            source_id: Some("demo.actor.ember-mote.1".to_owned()),
        }];
        payload.player.resistances = vec![ResistanceSaveDto {
            damage_type: DamageTypeDto::Poison,
            level: ResistanceLevelDto::Resistant,
        }];
        let mut game = Game::from_save(payload).expect("poison setup should load");
        let update = game
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("poisoned wait should execute");

        assert_eq!(update.player.hp, 7);
        assert!(update.player.statuses.is_empty());
        assert_eq!(update.player.resistances.len(), 1);
        assert_eq!(
            update
                .events
                .iter()
                .filter(|event| event.message_key == "status-player-damage")
                .count(),
            3
        );
        assert!(
            update
                .events
                .iter()
                .any(|event| event.message_key == "status-player-expired")
        );
        let restored = Game::from_save(game.to_save()).expect("status save should restore");
        assert_eq!(restored.state_hash(), game.state_hash());
    }

    #[test]
    fn bleeding_ticks_as_physical_damage_in_stable_status_order() {
        let mut payload = Game::new(42).to_save();
        payload.player.statuses = vec![
            StatusSaveDto {
                kind_id: STATUS_POISON.to_owned(),
                intensity: 1,
                remaining_ticks: 1,
                source_id: None,
            },
            StatusSaveDto {
                kind_id: STATUS_BLEEDING.to_owned(),
                intensity: 2,
                remaining_ticks: 2,
                source_id: None,
            },
        ];
        let mut game = Game::from_save(payload).expect("bleeding setup should load");
        let update = game
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("bleeding wait should execute");

        assert_eq!(update.player.hp, 5);
        assert!(update.player.statuses.is_empty());
        let damage_statuses = update
            .events
            .iter()
            .filter(|event| event.message_key == "status-player-damage")
            .map(|event| event.args["status"].as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            damage_statuses,
            [STATUS_BLEEDING, STATUS_POISON, STATUS_BLEEDING]
        );
    }

    #[test]
    fn content_driven_fire_melee_uses_the_player_resistance_profile() {
        let (seed, normal_damage) = (0_u64..1_000)
            .find_map(|seed| {
                let mut game = Game::new(42);
                game.rng = RfbRng::seeded(seed);
                let mut events = Vec::new();
                game.resolve_monster_melee(0, &mut events);
                events.into_iter().find_map(|event| match event {
                    DomainEvent::MonsterMeleeHit { damage, .. } if damage.applied >= 2 => {
                        Some((seed, damage.applied))
                    }
                    _ => None,
                })
            })
            .expect("a deterministic seed should produce a fire hit of at least two damage");

        let mut resistant = Game::new(42);
        resistant.player.resistances.set(
            DamageType::Fire,
            crate::resistance::ResistanceLevel::Resistant,
        );
        resistant.rng = RfbRng::seeded(seed);
        let mut events = Vec::new();
        resistant.resolve_monster_melee(0, &mut events);
        let resisted_damage = events
            .into_iter()
            .find_map(|event| match event {
                DomainEvent::MonsterMeleeHit { damage, .. } => Some(damage.applied),
                _ => None,
            })
            .expect("the same seed should preserve the hit result");

        assert_eq!(resisted_damage, normal_damage - normal_damage / 2);
        assert_eq!(resistant.player.hp, 10 - resisted_damage);
    }

    #[test]
    fn content_driven_monster_routine_resolves_blows_in_declared_order() {
        let mut game = Game::new(0);
        game.entities[0].kind_id = "demo.actor.echo-hound".to_owned();
        let routine = game.snapshot().entities[0].melee_routine.clone();

        assert_eq!(routine.blows.len(), 2);
        assert_eq!(routine.blows[0].method_id, "rfb.blow.echo-bite");
        assert_eq!(routine.blows[1].method_id, "rfb.blow.echo-rake");

        let mut events = Vec::new();
        game.resolve_monster_melee(0, &mut events);
        let projected = project_events(events);

        assert_eq!(projected.len(), 2);
        assert_eq!(projected[0].args["method"], "rfb.blow.echo-bite");
        assert_eq!(projected[1].args["method"], "rfb.blow.echo-rake");
    }

    #[test]
    fn lethal_monster_status_removes_the_entity_before_energy_actions() {
        let mut payload = Game::new(42).to_save();
        payload.entities[0].statuses = vec![StatusSaveDto {
            kind_id: STATUS_POISON.to_owned(),
            intensity: 3,
            remaining_ticks: 1,
            source_id: Some("demo.player.1".to_owned()),
        }];
        let mut game = Game::from_save(payload).expect("monster poison setup should load");
        let update = game
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("wait should process monster poison");

        assert!(update.entities.is_empty());
        assert_eq!(update.removed_entities, ["demo.monster.ember-mote.1"]);
        assert!(
            update
                .events
                .iter()
                .any(|event| event.message_key == "status-entity-death")
        );
    }

    #[test]
    fn leader_death_dissolves_pack_before_remaining_members_act() {
        let mut payload = Game::new(42).to_save();
        let leader_id = payload.entities[0].id.clone();
        let pack_id = "test.pack.leader-death".to_owned();
        payload.entities[0].statuses = vec![StatusSaveDto {
            kind_id: STATUS_POISON.to_owned(),
            intensity: 3,
            remaining_ticks: 1,
            source_id: Some("demo.player.1".to_owned()),
        }];
        payload.entities[0].pack = Some(rfb_protocol::MonsterPackSaveDto {
            id: pack_id.clone(),
            leader_id: leader_id.clone(),
            role: MonsterPackRoleDto::Leader,
            behavior: MonsterPackBehaviorDto::Seek,
        });
        let mut member = payload.entities[0].clone();
        member.id = "test.pack.member".to_owned();
        member.position = Position { x: 8, y: 6 };
        member.statuses.clear();
        member.pack = Some(rfb_protocol::MonsterPackSaveDto {
            id: pack_id,
            leader_id,
            role: MonsterPackRoleDto::Member,
            behavior: MonsterPackBehaviorDto::GuardLeader,
        });
        payload.entities.push(member);

        let mut game = Game::from_save(payload).expect("pack death setup should load");
        game.dispatch(command(1, 0, GameCommand::Wait))
            .expect("leader death should resolve");

        assert_eq!(game.entities.len(), 1);
        assert_eq!(game.entities[0].id, "test.pack.member");
        assert!(game.entities[0].pack.is_none());
        Game::from_save(game.to_save()).expect("dissolved pack should remain saveable");
    }

    #[test]
    fn content_driven_loot_generation_is_deterministic_and_persistent() {
        let mut left = Game::new(42);
        let initial = left.to_save();
        assert_eq!(initial.carried_items.len(), 1);
        assert_eq!(initial.carried_items[0].id, "generated.item.1");
        assert_eq!(
            initial.carried_items[0].actor_id,
            "demo.monster.ember-mote.1"
        );
        assert_eq!(initial.carried_items[0].kind_id, "demo.item.echo-charm");
        assert_eq!(left.snapshot().items.len(), 5);
        assert_eq!(left.rng.draw_counter, 3);
        left.entities[0].statuses = vec![StatusInstance {
            kind_id: STATUS_POISON.to_owned(),
            intensity: 3,
            remaining_ticks: 1,
            source_id: Some(left.player.id.clone()),
        }];
        let mut right = left.clone();
        let death_position = left.entities[0].position;

        let left_update = left
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("loot-bearing monster death should execute");
        let right_update = right
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("same loot context should execute");

        assert_eq!(left_update.state_hash, right_update.state_hash);
        assert_eq!(left.rng.draw_counter, 6);
        assert_eq!(left.rng.draw_counter, right.rng.draw_counter);
        let drops = left_update
            .events
            .iter()
            .filter(|event| event.message_key == "loot-drop")
            .collect::<Vec<_>>();
        assert_eq!(drops.len(), 2);
        assert_eq!(drops[0].args["target"], "demo.item.echo-charm");
        assert_eq!(drops[1].args["source"], "demo.actor.ember-mote");
        let carried = left
            .items
            .iter()
            .find(|item| item.id == "generated.item.1")
            .expect("carried loot should preserve its stable item ID");
        assert_eq!(carried.location, ItemLocation::Ground(death_position));
        assert_eq!(carried.kind_id, "demo.item.echo-charm");
        let generated = left
            .items
            .iter()
            .find(|item| item.id == "generated.item.2")
            .expect("death loot should allocate the next stable item ID");
        assert_eq!(generated.location, ItemLocation::Ground(death_position));
        assert_eq!(generated.quantity, 1);
        assert_eq!(generated.kind_id, "demo.item.echo-charm");
        assert_eq!(generated.quality, ItemQualityDto::Ordinary);
        assert!(generated.affix_ids.is_empty());
        let restored = Game::from_save(left.to_save()).expect("generated loot should reload");
        assert_eq!(restored.state_hash(), left.state_hash());
    }

    #[test]
    fn carried_item_save_rejects_a_missing_monster_owner() {
        let mut payload = Game::new(42).to_save();
        payload.carried_items[0].actor_id = "demo.monster.missing".to_owned();

        assert!(matches!(
            Game::from_save(payload),
            Err(CoreError::InvalidSave("carried item state is invalid"))
        ));
    }

    #[test]
    fn previous_built_in_content_hash_migrates_without_spawning_new_items() {
        for previous_hash in PREVIOUS_BUILT_IN_CONTENT_HASHES {
            let mut payload = Game::new(42).to_save();
            payload.content_hash = previous_hash.to_owned();
            payload.carried_items.clear();
            payload.items.retain(|item| {
                item.kind_id != "demo.item.echo-charm"
                    && item.kind_id != "demo.item.echo-blade"
                    && item.kind_id != "demo.item.resonance-sling"
                    && item.kind_id != "demo.item.resonance-pellet"
            });

            let restored = Game::from_save(payload).expect("known previous content should migrate");
            let snapshot = restored.snapshot();
            assert_eq!(snapshot.content_hash, BUILT_IN_CONTENT_HASH);
            assert_eq!(snapshot.items.len(), 1);
            assert!(snapshot.items.iter().all(|item| {
                item.kind_id != "demo.item.echo-charm"
                    && item.kind_id != "demo.item.echo-blade"
                    && item.kind_id != "demo.item.resonance-sling"
                    && item.kind_id != "demo.item.resonance-pellet"
            }));
        }
    }

    #[test]
    fn previous_task_state_set_adds_new_tasks_as_available() {
        let mut current_payload = Game::new(42).to_save();
        current_payload
            .task_states
            .retain(|state| state.task_id != "demo.task.echo-chain");
        assert!(matches!(
            Game::from_save(current_payload),
            Err(CoreError::InvalidSave("task state set is incomplete"))
        ));

        let mut payload = Game::new(42).to_save();
        payload.content_hash =
            "b37398cb9d005302c958a9e300d07a435e8631d6a5cd44ba63b0086069577c43".to_owned();
        payload
            .task_states
            .retain(|state| state.task_id != "demo.task.echo-chain");

        let restored = Game::from_save(payload).expect("v44 task state set should migrate");
        let chain = restored
            .snapshot()
            .tasks
            .into_iter()
            .find(|task| task.task_id == "demo.task.echo-chain")
            .expect("new staged task should be added during migration");
        assert_eq!(chain.status, TaskStatusKindDto::Available);
        assert_eq!((chain.stage, chain.stages), (1, 3));
        assert_eq!((chain.current, chain.required), (0, 1));
    }

    #[test]
    fn paused_task_can_be_abandoned_from_the_surface() {
        let mut game = Game::new(27);
        let entry = Position { x: 4, y: 4 };
        game.player.position = entry;
        game.traverse_stairs(false)
            .expect("bounty entry should resolve")
            .expect("bounty entry should transition");
        game.traverse_stairs(false)
            .expect("bounty pause should resolve")
            .expect("bounty pause should return to the surface");

        let paused = game
            .snapshot()
            .tasks
            .into_iter()
            .find(|task| task.task_id == "demo.task.echo-bounty")
            .expect("bounty task should be projected");
        assert_eq!(paused.status, TaskStatusKindDto::Paused);
        assert_eq!((paused.retakes_used, paused.max_retakes), (0, Some(1)));
        assert!(
            game.stored_floors
                .contains_key("demo.floor.echo-bounty-rift")
        );

        let update = dispatch_next(
            &mut game,
            GameCommand::AbandonPausedTask {
                task_id: "demo.task.echo-bounty".to_owned(),
            },
        );
        assert_eq!(game.current_floor_id, "demo.floor.surface");
        assert_eq!(
            game.task_states["demo.task.echo-bounty"].status,
            TaskStatusKindDto::Abandoned
        );
        assert!(
            !game
                .stored_floors
                .contains_key("demo.floor.echo-bounty-rift")
        );
        assert_eq!(game.terrain_at(entry), "demo.terrain.bounty-rift-abandoned");
        assert!(
            update
                .events
                .iter()
                .any(|event| event.kind == "task.abandoned")
        );
        assert!(
            update
                .changed_cells
                .iter()
                .any(|cell| cell.position == entry
                    && cell.terrain_id == "demo.terrain.bounty-rift-abandoned")
        );
        Game::from_save(game.to_save()).expect("surface abandonment should round-trip");
    }

    #[test]
    fn regenerated_retake_preserves_progress_and_enforces_the_limit() {
        let mut game = Game::new(27);
        game.player.position = Position { x: 4, y: 4 };
        game.traverse_stairs(false)
            .expect("bounty entry should resolve")
            .expect("bounty entry should transition");
        assert_eq!(game.entities.len(), 2);
        game.entities.pop();
        game.task_states
            .get_mut("demo.task.echo-bounty")
            .expect("bounty task should exist")
            .current = 1;
        game.traverse_stairs(false)
            .expect("partial bounty pause should resolve")
            .expect("partial bounty should return to the surface");
        assert_eq!(
            game.stored_floors["demo.floor.echo-bounty-rift"]
                .entities
                .len(),
            1
        );
        let draws_before_retake = game.rng.draw_counter;

        game.traverse_stairs(false)
            .expect("first bounty retake should resolve")
            .expect("first bounty retake should regenerate the floor");
        assert!(game.rng.draw_counter > draws_before_retake);
        assert_eq!(game.entities.len(), 1);
        let active = &game.task_states["demo.task.echo-bounty"];
        assert_eq!(active.status, TaskStatusKindDto::Active);
        assert_eq!(
            (active.current, active.required, active.retakes_used),
            (1, 2, 1)
        );

        game.traverse_stairs(false)
            .expect("second bounty pause should resolve")
            .expect("second bounty pause should return to the surface");
        let draws_before_rejected_retake = game.rng.draw_counter;
        assert!(
            game.traverse_stairs(false)
                .expect("exhausted bounty entry should resolve")
                .is_none()
        );
        assert_eq!(game.current_floor_id, "demo.floor.surface");
        assert_eq!(game.rng.draw_counter, draws_before_rejected_retake);

        let mut invalid = game.to_save();
        invalid
            .task_states
            .iter_mut()
            .find(|state| state.task_id == "demo.task.echo-bounty")
            .expect("bounty save state should exist")
            .retakes_used = 2;
        assert!(matches!(
            Game::from_save(invalid),
            Err(CoreError::InvalidSave("task state is invalid"))
        ));
    }

    #[test]
    fn v60_task_state_defaults_to_zero_retakes_without_rng_drift() {
        let mut payload = Game::new(27).to_save();
        payload.content_hash =
            "9789fcbbd8431ed745d8a0305cc81a54cc7e45ce79be86ed76e0227d66564a02".to_owned();
        let saved_draws = payload.rng.draw_counter;
        let restored = Game::from_save(payload).expect("v60 task state should migrate");

        assert_eq!(restored.rng.draw_counter, saved_draws);
        assert!(
            restored
                .snapshot()
                .tasks
                .iter()
                .all(|task| task.retakes_used == 0)
        );
    }

    #[test]
    fn dungeon_guardian_state_migrates_and_rejects_entity_mismatch() {
        let mut old_payload = Game::new(42).to_save();
        old_payload.content_hash =
            "0e6cf15310644e7b3eb2f7acb0c18a8b1a7fb08739e981e7492d4079e61ab44a".to_owned();
        old_payload.dungeon_states.clear();
        let restored = Game::from_save(old_payload).expect("v45 save should add dungeon state");
        assert!(!restored.dungeon_states["demo.dungeon.echo-depths"].guardian_defeated);
        assert!(!restored.dungeon_states["demo.dungeon.resonance-descent"].guardian_defeated);

        let mut v48_payload = Game::new(42).to_save();
        v48_payload.content_hash =
            "9c8fc3226c20300a308d21a5da69033efb853169214f4c411e6c740800bdf9ad".to_owned();
        v48_payload
            .dungeon_states
            .retain(|state| state.dungeon_id == "demo.dungeon.echo-depths");
        let restored =
            Game::from_save(v48_payload).expect("v48 save should add the pressure dungeon state");
        assert!(!restored.dungeon_states["demo.dungeon.echo-depths"].guardian_defeated);
        assert!(!restored.dungeon_states["demo.dungeon.resonance-descent"].guardian_defeated);

        let mut current_payload = Game::new(42).to_save();
        current_payload
            .dungeon_states
            .retain(|state| state.dungeon_id == "demo.dungeon.echo-depths");
        assert!(matches!(
            Game::from_save(current_payload),
            Err(CoreError::InvalidSave("dungeon state set is incomplete"))
        ));

        let mut game = Game::new(27);
        game.player.position = Position { x: 3, y: 4 };
        game.traverse_stairs(false)
            .expect("echo dungeon entry should resolve")
            .expect("echo dungeon entry should transition");
        descend_one_floor(&mut game);
        descend_one_floor(&mut game);
        assert!(
            game.content
                .world(&game.world_id)
                .expect("world should remain available")
                .procedural_floors
                .iter()
                .any(|floor| floor.id == game.current_floor_id && floor.final_floor)
        );
        let mut payload = game.to_save();
        payload.dungeon_states[0].guardian_defeated = true;
        let result = Game::from_save(payload);
        assert!(
            matches!(
                result,
                Err(CoreError::InvalidSave("dungeon guardian state is invalid"))
            ),
            "unexpected guardian mismatch result: {result:?}"
        );
    }

    #[test]
    fn guardian_mirrors_share_conquest_and_are_removed_from_other_final_floors() {
        let mut game = Game::new(71);
        game.player.position = Position { x: 3, y: 4 };
        game.traverse_stairs(false)
            .expect("echo dungeon entry should resolve")
            .expect("echo dungeon entry should transition");

        traverse_connection(&mut game, "demo.connection.echo-depth-1.down-a");
        traverse_connection(&mut game, "demo.connection.echo-depth-2.down-b");
        assert!(
            game.entities
                .iter()
                .any(|actor| actor.id == "demo.guardian.echo-depths.2")
        );
        traverse_connection(&mut game, "demo.connection.echo-depth-3-mirror.up-a");
        traverse_connection(&mut game, "demo.connection.echo-depth-2.up-a");
        traverse_connection(&mut game, "demo.connection.echo-depth-1.down-b");
        traverse_connection(&mut game, "demo.connection.echo-depth-2-mirror.down-a");

        assert!(
            stored_floor(&game, "demo.floor.echo-depth-3-mirror")
                .entities
                .iter()
                .any(|actor| actor.id == "demo.guardian.echo-depths.2")
        );
        let guardian_index = game
            .entities
            .iter()
            .position(|actor| actor.id == "demo.guardian.echo-depths.3")
            .expect("branch final floor should contain its guardian mirror");
        let mut events = Vec::new();
        let mut changed = BTreeSet::new();
        let mut removed_entities = Vec::new();
        game.resolve_actor_death(
            guardian_index,
            DomainEvent::Waited,
            &mut events,
            &mut changed,
            &mut removed_entities,
        )
        .expect("guardian mirror death should resolve");

        assert!(game.dungeon_states["demo.dungeon.echo-depths"].guardian_defeated);
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, DomainEvent::DungeonGuardianDefeated { .. }))
                .count(),
            1
        );
        assert_eq!(removed_entities, ["demo.guardian.echo-depths.3"]);
        assert!(
            stored_floor(&game, "demo.floor.echo-depth-3-mirror")
                .entities
                .iter()
                .all(|actor| actor.id != "demo.guardian.echo-depths.2")
        );
        assert!(stored_floor(&game, "demo.floor.echo-depth-3-mirror")
            .items
            .iter()
            .all(|item| {
                !matches!(&item.location, ItemLocation::CarriedBy { actor_id } if actor_id == "demo.guardian.echo-depths.2")
            }));

        let mut restored = Game::from_save(game.to_save()).expect("shared conquest should persist");
        traverse_connection(&mut restored, "demo.connection.echo-depth-3-branch.up-a");
        traverse_connection(&mut restored, "demo.connection.echo-depth-2-mirror.up-a");
        traverse_connection(&mut restored, "demo.connection.echo-depth-1.shaft-down");
        assert_eq!(restored.current_floor_id, "demo.floor.echo-depth-3-shaft");
        assert!(
            restored
                .entities
                .iter()
                .all(|actor| actor.id != "demo.guardian.echo-depths.4")
        );
    }

    #[test]
    fn v62_floor_with_obsolete_connection_set_uses_legacy_stair_fallback() {
        let mut game = Game::new(93);
        game.player.position = Position { x: 3, y: 4 };
        game.traverse_stairs(false)
            .expect("echo dungeon entry should resolve")
            .expect("echo dungeon entry should transition");
        traverse_connection(&mut game, "demo.connection.echo-depth-1.down-a");
        let mut payload = game.to_save();
        payload.content_hash =
            "9d25687c1296bc6f9953024bd76bb9eefc4c1e3955280b96d34d565ff7ca289d".to_owned();
        let occupied = payload
            .floor_connections
            .iter()
            .map(|connection| connection.position)
            .chain(std::iter::once(payload.player.position))
            .collect::<BTreeSet<_>>();
        let legacy_index = payload
            .terrain
            .terrain_ids
            .iter()
            .enumerate()
            .find(|(index, terrain_id)| {
                let position = Position {
                    x: i32::try_from(index % usize::from(payload.terrain.width))
                        .expect("x should fit i32"),
                    y: i32::try_from(index / usize::from(payload.terrain.width))
                        .expect("y should fit i32"),
                };
                terrain_id.as_str() == "demo.terrain.floor" && !occupied.contains(&position)
            })
            .map(|(index, _)| index)
            .expect("generated floor should retain a legacy stair candidate");
        let legacy_position = Position {
            x: i32::try_from(legacy_index % usize::from(payload.terrain.width))
                .expect("x should fit i32"),
            y: i32::try_from(legacy_index / usize::from(payload.terrain.width))
                .expect("y should fit i32"),
        };
        payload.terrain.terrain_ids[legacy_index] = "demo.terrain.stairs-up".to_owned();
        payload.floor_connections.push(FloorConnectionSaveDto {
            id: "demo.connection.echo-depth-2.up-b".to_owned(),
            position: legacy_position,
            target_floor_id: None,
            target_connection_id: None,
        });
        let expected_terrain = payload.terrain.clone();
        let expected_entities = payload.entities.clone();
        let expected_draws = payload.rng.draw_counter;

        let restored = Game::from_save(payload).expect("v62 connection set should migrate");
        assert!(restored.floor_connections.is_empty());
        assert_eq!(restored.to_save().terrain, expected_terrain);
        assert_eq!(actors_to_save(&restored.entities), expected_entities);
        assert_eq!(restored.rng.draw_counter, expected_draws);
    }

    #[test]
    fn previous_generated_floor_is_not_backfilled_with_v27_room_content() {
        let mut game = Game::new(27);
        game.player.position = Position { x: 3, y: 4 };
        game.dispatch(command(1, 0, GameCommand::TraverseStairs))
            .expect("current content should generate the procedural floor");
        let mut payload = game.to_save();
        payload.content_hash =
            "febe50b7a55a637a05d78135f14aa8f72fa457632ae8d705c002e92acf9e4fd9".to_owned();
        payload.entities.clear();
        payload.items.clear();
        payload.carried_items.clear();
        payload.next_item_instance_serial = 2;

        let restored = Game::from_save(payload).expect("v26 generated floor should migrate");
        assert_eq!(restored.current_floor_id, "demo.floor.echo-depth-1");
        assert!(restored.entities.is_empty());
        assert!(restored.items.is_empty());
        assert_eq!(restored.next_item_instance_serial, 2);
    }

    #[test]
    fn previous_generated_floor_is_not_backfilled_with_v28_door() {
        let mut game = Game::new(27);
        game.player.position = Position { x: 3, y: 4 };
        game.dispatch(command(1, 0, GameCommand::TraverseStairs))
            .expect("current content should generate the procedural floor");
        let mut payload = game.to_save();
        payload.content_hash =
            "51ffdccfe19a9f159adc15c2f62965ff4a5d44b55990eb9f29df96870937a043".to_owned();
        let door_index = 4_usize * usize::from(payload.terrain.width) + 10;
        payload.terrain.terrain_ids[door_index] = "demo.terrain.floor".to_owned();

        let restored = Game::from_save(payload).expect("v27 generated floor should migrate");
        assert_eq!(restored.current_floor_id, "demo.floor.echo-depth-1");
        assert_eq!(
            restored.terrain_at(Position { x: 10, y: 4 }),
            "demo.terrain.floor"
        );
    }

    #[test]
    fn previous_generated_floor_is_not_upgraded_to_a_v29_locked_door() {
        let mut game = Game::new(27);
        game.player.position = Position { x: 3, y: 4 };
        game.dispatch(command(1, 0, GameCommand::TraverseStairs))
            .expect("current content should generate the procedural floor");
        let mut payload = game.to_save();
        payload.content_hash =
            "f060f44c88033e8ef75478929a354d6b5b0bc5f933ca2772e79c3440940942e8".to_owned();
        let door_index = 4_usize * usize::from(payload.terrain.width) + 10;
        payload.terrain.terrain_ids[door_index] = "demo.terrain.door-closed".to_owned();

        let restored = Game::from_save(payload).expect("v28 generated floor should migrate");
        assert_eq!(restored.current_floor_id, "demo.floor.echo-depth-1");
        assert_eq!(
            restored.terrain_at(Position { x: 10, y: 4 }),
            "demo.terrain.door-closed"
        );
    }

    #[test]
    fn previous_generated_floor_is_not_upgraded_to_a_v31_secret_door() {
        let mut game = Game::new(27);
        game.player.position = Position { x: 3, y: 4 };
        game.dispatch(command(1, 0, GameCommand::TraverseStairs))
            .expect("current content should generate the procedural floor");
        let mut payload = game.to_save();
        payload.content_hash =
            "2d2900d8052b0a600346d0b87cc3b3d5bb5138f851abbf2b95afa196bbbaaca2".to_owned();
        let door_index = 4_usize * usize::from(payload.terrain.width) + 10;
        payload.terrain.terrain_ids[door_index] = "demo.terrain.door-locked".to_owned();
        payload.revealed_terrain.clear();

        let restored = Game::from_save(payload).expect("v30 generated floor should migrate");
        let door_position = Position { x: 10, y: 4 };
        assert_eq!(restored.current_floor_id, "demo.floor.echo-depth-1");
        assert_eq!(
            restored.terrain_at(door_position),
            "demo.terrain.door-locked"
        );
        assert_eq!(
            restored.known_terrain_at(door_position),
            "demo.terrain.door-locked"
        );
    }

    #[test]
    fn previous_equipment_content_migrates_to_derived_modifiers() {
        let mut game = Game::new(42);
        collect_both_demo_items(&mut game);
        game.dispatch(command(
            5,
            4,
            GameCommand::Equip {
                item_id: "demo.item.echo-charm.1".to_owned(),
            },
        ))
        .expect("equip should execute");
        let mut payload = game.to_save();
        payload.content_hash = PREVIOUS_BUILT_IN_CONTENT_HASHES[1].to_owned();
        payload.carried_items.clear();
        payload.player.base_max_hp = 0;
        payload.next_item_instance_serial = 0;

        let restored = Game::from_save(payload).expect("known 1.1 content should migrate");
        let snapshot = restored.snapshot();
        assert_eq!(snapshot.content_hash, BUILT_IN_CONTENT_HASH);
        assert_eq!(snapshot.player.base_max_hp, 10);
        assert_eq!(snapshot.player.max_hp, 14);
        assert_eq!(snapshot.player.attack, 4);
        assert_eq!(snapshot.player.defense, 2);
        assert_eq!(snapshot.player.equipment_modifiers.attack, 2);
        assert_eq!(snapshot.player.equipment_modifiers.defense, 1);
        assert_eq!(snapshot.player.equipment_modifiers.max_hp, 4);
        assert_eq!(restored.next_item_instance_serial, 1);
    }

    #[test]
    fn previous_combat_content_migrates_to_current_actor_stats() {
        let mut game = Game::new(42);
        collect_both_demo_items(&mut game);
        game.dispatch(command(
            5,
            4,
            GameCommand::Equip {
                item_id: "demo.item.echo-charm.1".to_owned(),
            },
        ))
        .expect("equip should execute");
        let mut payload = game.to_save();
        payload.content_hash = PREVIOUS_BUILT_IN_CONTENT_HASHES[2].to_owned();

        let restored = Game::from_save(payload).expect("known 1.2 content should migrate");
        let snapshot = restored.snapshot();
        assert_eq!(snapshot.content_hash, BUILT_IN_CONTENT_HASH);
        assert_eq!(snapshot.player.base_attack, 2);
        assert_eq!(snapshot.player.attack, 4);
        assert_eq!(snapshot.player.base_defense, 1);
        assert_eq!(snapshot.player.defense, 2);
        assert_eq!(snapshot.entities[0].attack, 1);
        assert_eq!(snapshot.entities[0].defense, 1);
    }

    #[test]
    fn fixed_seed_and_commands_are_deterministic() {
        let mut left = Game::new(42);
        let mut right = Game::new(42);
        let commands = [
            GameCommand::Move {
                direction: Direction::East,
            },
            GameCommand::Move {
                direction: Direction::South,
            },
            GameCommand::Wait,
        ];

        for (index, game_command) in commands.into_iter().enumerate() {
            let seq = index as u32 + 1;
            let revision = index as u32;
            left.dispatch(command(seq, revision, game_command.clone()))
                .expect("left command should execute");
            right
                .dispatch(command(seq, revision, game_command))
                .expect("right command should execute");
        }

        assert_eq!(left.state_hash(), right.state_hash());
    }

    #[test]
    fn normal_speed_monster_tracks_once_per_player_action() {
        let mut game = Game::new(42);
        let update = game
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("wait should advance the scheduler");

        assert_eq!(update.world_tick, 10);
        assert_eq!(update.player.energy_need, 0);
        assert_eq!(update.entities[0].position, Position { x: 7, y: 4 });
        assert_eq!(update.entities[0].energy_need, STANDARD_ACTION_COST);
        assert_eq!(update.changed_cells.len(), 2);
    }

    #[test]
    fn fast_and_slow_monsters_use_the_same_energy_scheduler() {
        let mut fast = Game::new(42);
        fast.entities[0].speed = 120;
        let fast_update = fast
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("fast scheduler case should execute");
        assert_eq!(fast_update.world_tick, 10);
        assert_eq!(fast_update.entities[0].position, Position { x: 6, y: 3 });
        assert_eq!(fast_update.entities[0].energy_need, STANDARD_ACTION_COST);

        let mut slow = Game::new(42);
        slow.entities[0].speed = 100;
        let first = slow
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("first slow scheduler case should execute");
        assert_eq!(first.entities[0].position, Position { x: 8, y: 5 });
        assert_eq!(first.entities[0].energy_need, 50);
        let second = slow
            .dispatch(command(2, 1, GameCommand::Wait))
            .expect("second slow scheduler case should execute");
        assert_eq!(second.entities[0].position, Position { x: 7, y: 4 });
        assert_eq!(second.entities[0].energy_need, STANDARD_ACTION_COST);
    }

    #[test]
    fn multiple_monsters_use_stable_id_order_when_paths_compete() {
        let mut left = Game::new(42);
        let mut second = left.entities[0].clone();
        second.id = "demo.monster.ember-mote.0".to_owned();
        second.position = Position { x: 8, y: 6 };
        left.entities.push(second);

        let mut right = left.clone();
        right.entities.reverse();

        let left_update = left
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("left scheduler should execute");
        let right_update = right
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("right scheduler should execute");

        assert_eq!(left_update.entities, right_update.entities);
        assert_eq!(left_update.changed_cells, right_update.changed_cells);
        assert_eq!(left_update.state_hash, right_update.state_hash);
        assert_ne!(
            left_update.entities[0].position,
            left_update.entities[1].position
        );
    }

    #[test]
    fn player_death_stops_the_remaining_monster_queue_immediately() {
        let mut game = Game::new(0);
        game.entities[0].id = "demo.monster.ember-mote.0".to_owned();
        game.entities[0].position = Position { x: 4, y: 3 };
        let mut second = game.entities[0].clone();
        second.id = "demo.monster.ember-mote.1".to_owned();
        second.position = Position { x: 4, y: 4 };
        game.entities.push(second);
        game.player.hp = 0;

        let update = game
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("fatal scheduler case should execute");

        assert!(update.player.is_dead);
        assert_eq!(
            update
                .events
                .iter()
                .filter(|event| event.message_key == "combat-player-death")
                .count(),
            1
        );
        let second = update
            .entities
            .iter()
            .find(|entity| entity.id == "demo.monster.ember-mote.1")
            .expect("second monster should remain present");
        assert_eq!(second.energy_need, 10);
    }

    #[test]
    fn save_payload_restores_identical_state() {
        let mut game = Game::new(7);
        collect_both_demo_items(&mut game);
        game.dispatch(command(
            5,
            4,
            GameCommand::Equip {
                item_id: "demo.item.echo-charm.1".to_owned(),
            },
        ))
        .expect("equip should execute");

        let restored = Game::from_save(game.to_save()).expect("save should restore");
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.snapshot(), game.snapshot());
        assert_eq!(restored.snapshot().equipment.len(), 1);
    }

    #[test]
    fn pickup_moves_the_ground_stack_into_inventory() {
        let mut game = Game::new(42);
        clear_monsters(&mut game);
        game.dispatch(command(
            1,
            0,
            GameCommand::Move {
                direction: Direction::East,
            },
        ))
        .expect("move should execute");
        let update = game
            .dispatch(command(2, 1, GameCommand::PickUp))
            .expect("pickup should execute");

        assert_eq!(update.items.len(), 4);
        assert_eq!(update.inventory.len(), 1);
        assert_eq!(update.inventory[0].id, "demo.item.luminous-shard.1");
        assert_eq!(update.inventory[0].quantity, 5);
        assert_eq!(update.player.carried_weight_tenths_pound, 50);
        assert_eq!(update.player.carry_capacity_tenths_pound, 100);
        assert_eq!(update.changed_cells.len(), 1);
        assert_eq!(update.changed_cells[0].position, Position { x: 4, y: 3 });
        assert_eq!(update.changed_cells[0].item_id, None);
        assert_eq!(update.events[0].message_key, "item-pickup-success");
    }

    #[test]
    fn pickup_over_capacity_rejects_the_whole_ground_stack() {
        let mut game = Game::new(42);
        clear_monsters(&mut game);
        game.player.position = Position { x: 6, y: 4 };
        for kind_id in [
            "demo.item.luminous-shard",
            "demo.item.echo-charm",
            "demo.item.echo-blade",
            "demo.item.resonance-sling",
        ] {
            game.items
                .iter_mut()
                .find(|item| item.kind_id == kind_id)
                .expect("carried fixture item should exist")
                .location = ItemLocation::Inventory;
        }
        assert_eq!(game.carried_weight_tenths_pound(), 100);

        let update = game
            .dispatch(command(1, 0, GameCommand::PickUp))
            .expect("over-capacity pickup should resolve as an action");

        let event = &update.events[0];
        assert_eq!(event.kind, "item.pickup.over-capacity");
        assert_eq!(event.args["target"], "demo.item.resonance-pellet");
        assert_eq!(event.args["quantity"], "6");
        assert_eq!(event.args["currentWeight"], "100");
        assert_eq!(event.args["pickupWeight"], "12");
        assert_eq!(event.args["capacity"], "100");
        assert_eq!(update.player.carried_weight_tenths_pound, 100);
        assert!(update.items.iter().any(|item| {
            item.id == "demo.item.resonance-pellet.1"
                && item.quantity == 6
                && item.position == Position { x: 6, y: 4 }
        }));
    }

    #[test]
    fn themed_vault_paints_template_and_spawns_depth_eligible_group_and_loot() {
        let game = (1..=64)
            .find_map(|seed| {
                let mut game = Game::new(seed);
                descend_one_floor(&mut game);
                descend_one_floor(&mut game);
                (game.current_floor_id == "demo.floor.echo-depth-2"
                    && game
                        .entities
                        .iter()
                        .any(|entity| entity.id.contains("harmonic-sepulcher-sentinels")))
                .then_some(game)
            })
            .expect("a harmonic sepulcher seed should remain reachable");

        assert_eq!(game.current_floor_id, "demo.floor.echo-depth-2");
        assert_eq!(game.floor_connections.len(), 3);
        assert_eq!(game.floor_regions.len(), 2);
        assert_eq!(game.entities.len(), 5);
        let regional_encounters = game
            .entities
            .iter()
            .filter(|entity| entity.id.contains(".encounter.plain."))
            .collect::<Vec<_>>();
        assert_eq!(regional_encounters.len(), 2);
        assert!(regional_encounters.iter().all(|entity| matches!(
            entity.kind_id.as_str(),
            "demo.actor.echo-hound"
                | "demo.actor.storm-spark"
                | "demo.actor.acid-seep"
                | "demo.actor.venom-spore"
        )));
        let vault_members = game
            .entities
            .iter()
            .filter(|entity| {
                entity.id.starts_with(
                    "demo.floor.echo-depth-2.demo.vault-group.harmonic-sepulcher-sentinels.",
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(vault_members.len(), 3);
        assert!(vault_members.iter().all(|entity| {
            matches!(
                entity.kind_id.as_str(),
                "demo.actor.frost-wisp" | "demo.actor.storm-spark" | "demo.actor.venom-spore"
            )
        }));

        let first_member = vault_members
            .iter()
            .find(|entity| entity.id.ends_with(".1"))
            .expect("vault should contain its first group member");
        let vault_origin = Position {
            x: first_member.position.x - 1,
            y: first_member.position.y - 1,
        };
        let vault_region_id = region_at(&game, first_member.position).region_id.clone();
        for y in vault_origin.y..vault_origin.y + 5 {
            for x in vault_origin.x..vault_origin.x + 6 {
                assert_eq!(
                    region_at(&game, Position { x, y }).region_id,
                    vault_region_id
                );
            }
        }
        assert!(regional_encounters.iter().all(|entity| {
            match region_at(&game, entity.position).region_id.as_str() {
                "demo.region.resonance-grotto" => matches!(
                    entity.kind_id.as_str(),
                    "demo.actor.acid-seep" | "demo.actor.venom-spore"
                ),
                "demo.region.resonance-gallery" => matches!(
                    entity.kind_id.as_str(),
                    "demo.actor.echo-hound" | "demo.actor.storm-spark"
                ),
                _ => false,
            }
        }));
        assert_eq!(
            game.terrain_at(Position {
                x: vault_origin.x + 3,
                y: vault_origin.y,
            }),
            "demo.terrain.door-secret"
        );
        assert_eq!(game.terrain_at(vault_origin), "demo.terrain.wall");
        assert!(game.items.iter().any(|item| {
            item.location
                == ItemLocation::Ground(Position {
                    x: vault_origin.x + 2,
                    y: vault_origin.y + 3,
                })
                && matches!(
                    item.kind_id.as_str(),
                    "demo.item.echo-blade" | "demo.item.echo-charm"
                )
        }));
        assert!(game.items.iter().all(|item| {
            matches!(item.location, ItemLocation::Ground(position) if !region_at(&game, position).region_id.is_empty())
        }));
        let mut instance_ids = BTreeSet::from([game.player.id.clone()]);
        instance_ids.extend(game.entities.iter().map(|entity| entity.id.clone()));
        for item in &game.items {
            assert!(
                instance_ids.insert(item.id.clone()),
                "duplicate item ID: {}",
                item.id
            );
            let definition = game
                .content
                .item(&item.kind_id)
                .expect("generated item kind must remain available");
            assert!(item.quantity <= definition.max_stack);
            if let ItemLocation::Ground(position) = item.location {
                assert!(
                    game.is_walkable(position),
                    "item {} is on non-walkable {} at {position:?}",
                    item.id,
                    game.terrain_at(position)
                );
            }
        }

        let restored = Game::from_save(game.to_save()).expect("vault floor save should restore");
        assert_eq!(restored.state_hash(), game.state_hash());
    }

    #[test]
    fn weighted_vault_candidates_are_deterministic_and_both_reachable() {
        let mut harmonic = 0;
        let mut resonant = 0;
        for seed in 1..=64 {
            let mut left = Game::new(seed);
            let mut right = Game::new(seed);
            for game in [&mut left, &mut right] {
                descend_one_floor(game);
                descend_one_floor(game);
            }
            assert_eq!(left.state_hash(), right.state_hash());
            if left
                .entities
                .iter()
                .any(|entity| entity.id.contains("harmonic-sepulcher-sentinels"))
            {
                harmonic += 1;
            } else if left
                .entities
                .iter()
                .any(|entity| entity.id.contains("resonant-gallery-chorus"))
            {
                resonant += 1;
            } else {
                panic!("depth two must select one eligible themed vault");
            }
        }
        assert!(harmonic > resonant);
        assert!(resonant > 0);
    }

    #[test]
    fn regional_themes_are_weighted_deterministic_and_keep_local_content_in_bounds() {
        let mut grotto_entry_count = 0;
        let mut gallery_entry_count = 0;
        for seed in 1..=64 {
            let mut left = Game::new(seed);
            left.player.position = Position { x: 3, y: 2 };
            left.traverse_stairs(false)
                .expect("resonance entry should resolve")
                .expect("resonance entry should transition");
            descend_one_floor(&mut left);

            let mut right = Game::new(seed);
            right.player.position = Position { x: 3, y: 2 };
            right
                .traverse_stairs(false)
                .expect("matching resonance entry should resolve")
                .expect("matching resonance entry should transition");
            descend_one_floor(&mut right);

            assert_eq!(left.current_floor_id, "demo.floor.resonance-depth-2");
            assert_eq!(left.floor_regions, right.floor_regions);
            assert_eq!(left.state_hash(), right.state_hash());
            assert_eq!(left.floor_regions.len(), 2);
            assert_eq!(left.entities.len(), 4);
            assert_eq!(left.items.len(), 2);

            let entry_region = left
                .floor_regions
                .iter()
                .find(|region| region.cells.contains(&left.player.position))
                .expect("entry room must belong to one region");
            match entry_region.region_id.as_str() {
                "demo.region.resonance-grotto" => grotto_entry_count += 1,
                "demo.region.resonance-gallery" => gallery_entry_count += 1,
                _ => panic!("unexpected generated region"),
            }

            let mut all_cells = BTreeSet::new();
            for region in &left.floor_regions {
                assert_eq!(region.cells.len(), 30);
                assert!(
                    region
                        .cells
                        .iter()
                        .all(|position| all_cells.insert(*position))
                );
                let expected_terrain = match region.region_id.as_str() {
                    "demo.region.resonance-grotto" => "demo.terrain.resonance-cavern",
                    "demo.region.resonance-gallery" => "demo.terrain.resonant-floor",
                    _ => panic!("unexpected generated region"),
                };
                assert!(
                    region
                        .cells
                        .iter()
                        .any(|position| left.terrain_at(*position) == expected_terrain)
                );
            }
            assert!(left.terrain.iter().enumerate().any(|(index, terrain_id)| {
                let position = Position {
                    x: i32::try_from(index % usize::from(left.width)).unwrap_or_default(),
                    y: i32::try_from(index / usize::from(left.width)).unwrap_or_default(),
                };
                terrain_id == "demo.terrain.floor" && !all_cells.contains(&position)
            }));

            for entity in &left.entities {
                let region = left
                    .floor_regions
                    .iter()
                    .find(|region| region.cells.contains(&entity.position))
                    .expect("regional actor must remain inside its assigned region");
                assert!(match region.region_id.as_str() {
                    "demo.region.resonance-grotto" => matches!(
                        entity.kind_id.as_str(),
                        "demo.actor.acid-seep" | "demo.actor.venom-spore"
                    ),
                    "demo.region.resonance-gallery" => matches!(
                        entity.kind_id.as_str(),
                        "demo.actor.echo-hound" | "demo.actor.storm-spark"
                    ),
                    _ => false,
                });
            }
            for item in &left.items {
                let ItemLocation::Ground(position) = item.location else {
                    panic!("regional floor loot must be placed on the ground");
                };
                let region = left
                    .floor_regions
                    .iter()
                    .find(|region| region.cells.contains(&position))
                    .expect("regional loot must remain inside its assigned region");
                assert_eq!(
                    item.kind_id,
                    match region.region_id.as_str() {
                        "demo.region.resonance-grotto" => "demo.item.luminous-shard",
                        "demo.region.resonance-gallery" => "demo.item.resonance-pellet",
                        _ => panic!("unexpected generated region"),
                    }
                );
            }
        }
        assert!(grotto_entry_count > gallery_entry_count);
        assert!(gallery_entry_count > 0);
    }

    #[test]
    fn floor_regions_round_trip_reject_overlap_and_v59_missing_state_stays_empty() {
        let mut game = Game::new(17);
        game.player.position = Position { x: 3, y: 2 };
        game.traverse_stairs(false)
            .expect("resonance entry should resolve")
            .expect("resonance entry should transition");
        descend_one_floor(&mut game);

        let payload = game.to_save();
        assert_eq!(payload.floor_regions.len(), 2);
        let restored = Game::from_save(payload.clone()).expect("region state should restore");
        assert_eq!(restored.floor_regions, game.floor_regions);
        assert_eq!(restored.state_hash(), game.state_hash());

        let mut overlap = payload.clone();
        let duplicate = overlap.floor_regions[0].cells[0];
        overlap.floor_regions[1].cells.push(duplicate);
        assert!(matches!(
            Game::from_save(overlap),
            Err(CoreError::InvalidSave("floor region state is invalid"))
        ));

        let mut legacy = payload;
        legacy.content_hash =
            "4cdcad204a7ccad6d67b8dcb50ccdcc188220a72d258c37219974fad51e5274d".to_owned();
        legacy.floor_regions.clear();
        let draw_counter = legacy.rng.draw_counter;
        let legacy_entities = legacy.entities.clone();
        let legacy_items = legacy.items.clone();
        let restored =
            Game::from_save(legacy).expect("v59 regionless floor should remain loadable");
        assert!(restored.floor_regions.is_empty());
        assert_eq!(restored.rng.draw_counter, draw_counter);
        assert_eq!(actors_to_save(&restored.entities), legacy_entities);
        assert_eq!(items_to_save(&restored.items), legacy_items);
    }

    #[test]
    fn generation_budgets_scale_across_the_ten_depth_pressure_dungeon() {
        let mut game = Game::new(49);
        game.player.position = Position { x: 3, y: 2 };
        game.traverse_stairs(false)
            .expect("pressure dungeon entry should resolve")
            .expect("pressure dungeon entry should transition");

        let actor_slots = [2_usize, 4, 4, 5, 6, 7, 8, 9, 1, 30];
        let loot_placements = [1_usize, 2, 1, 1, 2, 2, 2, 4, 3, 3];
        let feature_placements = [0_usize, 0, 2, 3, 4, 4, 4, 4, 0, 4];
        for depth in 1..=10 {
            assert_eq!(
                game.current_floor_id,
                format!("demo.floor.resonance-depth-{depth}")
            );
            assert_eq!(
                game.entities.len(),
                actor_slots[depth - 1],
                "depth {depth} actor budget"
            );
            assert_eq!(
                game.items.len(),
                loot_placements[depth - 1],
                "depth {depth} loot budget"
            );
            let terrain_feature_tiles = game
                .terrain
                .iter()
                .filter(|terrain| {
                    matches!(
                        terrain.as_str(),
                        "demo.terrain.trap-echo-snare"
                            | "demo.terrain.echo-rubble"
                            | "demo.terrain.door-locked"
                            | "demo.terrain.door-secret"
                    )
                })
                .count();
            let mandatory_feature_tiles = if depth == 9 {
                1
            } else {
                2 + usize::from(depth == 8) * 5 + usize::from(depth == 10)
            };
            assert_eq!(
                terrain_feature_tiles - mandatory_feature_tiles,
                feature_placements[depth - 1]
            );
            if depth == 4 {
                assert!(
                    game.terrain
                        .iter()
                        .any(|terrain| terrain == "demo.terrain.door-locked")
                );
            }
            let guardian_slots = if depth == 10 { 1 } else { 0 };
            let vault_slots = if depth == 8 { 3 } else { 0 };
            let pit_slots = if depth == 10 { 25 } else { 0 };
            assert_eq!(
                game.entities
                    .iter()
                    .filter(|entity| entity.id.contains(".encounter."))
                    .count(),
                actor_slots[depth - 1] - guardian_slots - vault_slots - pit_slots
            );
            if depth == 8 {
                assert_eq!(
                    game.entities
                        .iter()
                        .filter(|entity| entity.id.contains(".vault."))
                        .count(),
                    3
                );
                assert!(
                    game.entities
                        .iter()
                        .any(|entity| { entity.id.contains("resonance-spindle-watch") })
                );
                assert!(
                    game.entities
                        .iter()
                        .any(|entity| entity.id.contains("resonance-crossroads-watch"))
                );
                assert!(
                    !game
                        .entities
                        .iter()
                        .any(|entity| entity.id.contains("sealed-resonance-monolith"))
                );
                assert_eq!(
                    game.terrain
                        .iter()
                        .filter(|terrain| *terrain == "demo.terrain.door-secret")
                        .count(),
                    6
                );
            }
            if depth == 10 {
                let pit = game
                    .entities
                    .iter()
                    .filter(|entity| entity.id.contains(".pit."))
                    .collect::<Vec<_>>();
                assert_eq!(pit.len(), 25);
                let xs = pit
                    .iter()
                    .map(|entity| entity.position.x)
                    .collect::<BTreeSet<_>>();
                let ys = pit
                    .iter()
                    .map(|entity| entity.position.y)
                    .collect::<BTreeSet<_>>();
                assert_eq!(xs.len(), 5);
                assert_eq!(ys.len(), 5);
                let center = Position {
                    x: (*xs.first().expect("pit must have a left edge")
                        + *xs.last().expect("pit must have a right edge"))
                        / 2,
                    y: (*ys.first().expect("pit must have a top edge")
                        + *ys.last().expect("pit must have a bottom edge"))
                        / 2,
                };
                let center_actor = pit
                    .iter()
                    .find(|entity| entity.position == center)
                    .expect("pit must fill its center");
                let center_level = game
                    .content
                    .actor(&center_actor.kind_id)
                    .expect("pit actor must remain available")
                    .level;
                assert!(
                    pit.iter()
                        .filter(|entity| {
                            xs.contains(&entity.position.x) && ys.contains(&entity.position.y)
                        })
                        .all(|entity| {
                            center_level
                                >= game
                                    .content
                                    .actor(&entity.kind_id)
                                    .expect("pit actor must remain available")
                                    .level
                        })
                );
                let inner_door = Position {
                    x: *xs.first().expect("pit must have a left edge") - 1,
                    y: center.y,
                };
                assert_eq!(
                    game.terrain[generated_terrain_index(game.width, inner_door)],
                    "demo.terrain.door-secret"
                );
            }
            if matches!(depth, 1 | 3) {
                assert!(
                    game.terrain
                        .iter()
                        .any(|terrain| terrain == "demo.terrain.floor")
                );
                assert!(
                    !game
                        .terrain
                        .iter()
                        .any(|terrain| terrain == "demo.terrain.resonant-floor")
                );
            } else if depth == 2 {
                assert!(
                    game.terrain
                        .iter()
                        .any(|terrain| terrain == "demo.terrain.floor")
                );
                assert!(
                    game.terrain
                        .iter()
                        .any(|terrain| terrain == "demo.terrain.resonant-floor")
                );
                assert!(
                    game.terrain
                        .iter()
                        .any(|terrain| terrain == "demo.terrain.resonance-cavern")
                );
            } else {
                assert!(
                    game.terrain
                        .iter()
                        .any(|terrain| terrain == "demo.terrain.resonant-floor")
                );
            }
            if depth < 10 {
                descend_one_floor(&mut game);
            }
        }
        assert!(
            game.entities
                .iter()
                .any(|entity| entity.id == "demo.guardian.resonance-descent.1")
        );
        assert_eq!(game.stored_floors.len(), 10);
        let restored = Game::from_save(game.to_save())
            .expect("pressure dungeon final floor should round-trip");
        assert_eq!(restored.state_hash(), game.state_hash());
    }

    #[test]
    fn regional_vault_and_pit_composition_is_deterministic_and_persistent() {
        let mut game = Game::new(49);
        game.player.position = Position { x: 3, y: 2 };
        game.traverse_stairs(false)
            .expect("pressure dungeon entry should resolve")
            .expect("pressure dungeon entry should transition");
        for _ in 1..8 {
            descend_one_floor(&mut game);
        }
        assert_eq!(game.current_floor_id, "demo.floor.resonance-depth-8");
        assert_eq!(game.floor_regions.len(), 2);
        assert_eq!(
            game.entities
                .iter()
                .filter(|entity| entity.id.contains(".vault."))
                .count(),
            3
        );
        assert!(game.entities.iter().all(|entity| {
            !entity.id.contains(".vault.")
                || !region_at(&game, entity.position).region_id.is_empty()
        }));
        assert!(game.items.iter().all(|item| {
            matches!(item.location, ItemLocation::Ground(position) if !region_at(&game, position).region_id.is_empty())
        }));
        let mut all_region_cells = BTreeSet::new();
        for region in &game.floor_regions {
            assert!(
                region
                    .cells
                    .iter()
                    .all(|cell| all_region_cells.insert(*cell))
            );
        }
        let depth_eight_hash = game.state_hash();
        let restored =
            Game::from_save(game.to_save()).expect("regional Vault floor should restore");
        assert_eq!(restored.state_hash(), depth_eight_hash);

        descend_one_floor(&mut game);
        descend_one_floor(&mut game);
        assert_eq!(game.current_floor_id, "demo.floor.resonance-depth-10");
        for terrain_id in [
            "demo.terrain.resonance-cavern",
            "demo.terrain.resonance-water-deep",
            "demo.terrain.resonance-water-shallow",
            "demo.terrain.resonance-ruin",
            "demo.terrain.resonance-vein",
        ] {
            assert!(
                game.terrain.iter().any(|candidate| candidate == terrain_id),
                "depth ten should contain {terrain_id}"
            );
        }
        let pit = game
            .entities
            .iter()
            .filter(|entity| entity.id.contains(".pit."))
            .collect::<Vec<_>>();
        assert_eq!(pit.len(), 25);
        let pit_region_id = region_at(&game, pit[0].position).region_id.clone();
        let min_x = pit
            .iter()
            .map(|entity| entity.position.x)
            .min()
            .expect("pit x");
        let max_x = pit
            .iter()
            .map(|entity| entity.position.x)
            .max()
            .expect("pit x");
        let min_y = pit
            .iter()
            .map(|entity| entity.position.y)
            .min()
            .expect("pit y");
        let max_y = pit
            .iter()
            .map(|entity| entity.position.y)
            .max()
            .expect("pit y");
        for y in min_y - 3..=max_y + 3 {
            for x in min_x - 3..=max_x + 3 {
                assert_eq!(region_at(&game, Position { x, y }).region_id, pit_region_id);
            }
        }
        assert!(game.entities.iter().all(|entity| {
            !entity.id.contains(".pit.")
                || region_at(&game, entity.position).region_id == pit_region_id
        }));
        assert!(game.entities.iter().any(|entity| {
            entity.id == "demo.guardian.resonance-descent.1"
                && !region_at(&game, entity.position).region_id.is_empty()
        }));
        assert!(game.items.iter().all(|item| {
            matches!(item.location, ItemLocation::Ground(position) if !region_at(&game, position).region_id.is_empty())
        }));
        let final_hash = game.state_hash();
        let mut same_seed = Game::new(49);
        same_seed.player.position = Position { x: 3, y: 2 };
        same_seed
            .traverse_stairs(false)
            .expect("matching pressure dungeon entry should resolve")
            .expect("matching pressure dungeon entry should transition");
        for _ in 1..10 {
            descend_one_floor(&mut same_seed);
        }
        assert_eq!(same_seed.state_hash(), final_hash);
        let restored = Game::from_save(game.to_save()).expect("regional pit floor should restore");
        assert_eq!(restored.state_hash(), final_hash);
    }

    #[test]
    fn regional_composition_round_trips_across_pressure_seeds() {
        for seed in [49, 77, 97, 156, 173, 211] {
            let mut game = Game::new(seed);
            game.player.position = Position { x: 3, y: 2 };
            game.traverse_stairs(false)
                .expect("pressure dungeon entry should resolve")
                .expect("pressure dungeon entry should transition");
            for depth in 1..=10 {
                Game::from_save(game.to_save()).unwrap_or_else(|error| {
                    panic!("seed {seed} depth {depth} should round-trip: {error}")
                });
                if depth < 10 {
                    descend_one_floor(&mut game);
                }
            }
        }
    }

    #[test]
    fn budgeted_rooms_and_connected_cavern_obey_geometric_limits() {
        let mut game = Game::new(49);
        let definition = game
            .content
            .world(BUILT_IN_WORLD_ID)
            .expect("built-in world should exist")
            .procedural_floors
            .iter()
            .find(|floor| floor.id == "demo.floor.resonance-depth-10")
            .expect("fixture should contain the first layout floor")
            .clone();
        let layout = definition
            .layout
            .as_ref()
            .expect("fixture should contain a layout");
        let room_geometry = layout
            .rooms
            .as_ref()
            .expect("fixture should contain room geometry");
        let rooms = game.generate_budgeted_rooms(&definition, room_geometry);

        assert_eq!(rooms.len(), 5);
        assert_eq!(rooms[0].id, "entry");
        assert_eq!(rooms[1].id, "remote");
        assert!(rooms.iter().map(GeneratedRoom::area).sum::<u32>() <= 112);
        let mut room_tiles = BTreeSet::new();
        for room in &rooms {
            for y in room.y..room.y + room.height {
                for x in room.x..room.x + room.width {
                    let position = Position { x, y };
                    if room.contains(position) {
                        assert!(room_tiles.insert(position));
                    }
                }
            }
        }

        let mut terrain = vec![
            definition.wall_terrain_id.clone();
            usize::from(definition.width) * usize::from(definition.height)
        ];
        let cavern_origin = game.generate_connected_cavern(
            &definition,
            "demo.terrain.resonance-cavern",
            &mut terrain,
        );
        let cavern_tiles = terrain
            .iter()
            .enumerate()
            .filter_map(|(index, terrain_id)| {
                (terrain_id == "demo.terrain.resonance-cavern").then_some(Position {
                    x: i32::try_from(index % usize::from(definition.width))
                        .expect("cavern x must fit i32"),
                    y: i32::try_from(index / usize::from(definition.width))
                        .expect("cavern y must fit i32"),
                })
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(cavern_tiles.len(), 64);
        let mut reached = BTreeSet::from([cavern_origin]);
        let mut frontier = VecDeque::from([cavern_origin]);
        while let Some(position) = frontier.pop_front() {
            for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                let neighbor = Position {
                    x: position.x + dx,
                    y: position.y + dy,
                };
                if cavern_tiles.contains(&neighbor) && reached.insert(neighbor) {
                    frontier.push_back(neighbor);
                }
            }
        }
        assert_eq!(reached, cavern_tiles);

        let mut rectangles = 0;
        let mut crosses = 0;
        for seed in 1..=64 {
            let mut seeded = Game::new(seed);
            for room in seeded.generate_budgeted_rooms(&definition, room_geometry) {
                match room.shape {
                    ProceduralRoomShape::Rectangle => rectangles += 1,
                    ProceduralRoomShape::Cross => crosses += 1,
                }
            }
        }
        assert!(rectangles > 0);
        assert!(crosses > 0);
    }

    #[test]
    fn lake_and_river_obey_exact_hydrology_budgets_and_connectivity() {
        let mut lake_game = Game::new(77);
        let lake_definition = lake_game
            .content
            .world(BUILT_IN_WORLD_ID)
            .expect("built-in world should exist")
            .procedural_floors
            .iter()
            .find(|floor| floor.id == "demo.floor.resonance-depth-10")
            .expect("fixture should contain the lake floor")
            .clone();
        let mut lake_terrain = vec![
            lake_definition.wall_terrain_id.clone();
            usize::from(lake_definition.width)
                * usize::from(lake_definition.height)
        ];
        let lake_origin = lake_game.generate_connected_lake(
            &lake_definition,
            "demo.terrain.resonance-water-deep",
            "demo.terrain.resonance-water-shallow",
            &mut lake_terrain,
        );
        let water_tiles = lake_terrain
            .iter()
            .enumerate()
            .filter_map(|(index, terrain_id)| {
                matches!(
                    terrain_id.as_str(),
                    "demo.terrain.resonance-water-deep" | "demo.terrain.resonance-water-shallow"
                )
                .then_some(Position {
                    x: i32::try_from(index % usize::from(lake_definition.width))
                        .expect("lake x must fit i32"),
                    y: i32::try_from(index / usize::from(lake_definition.width))
                        .expect("lake y must fit i32"),
                })
            })
            .collect::<BTreeSet<_>>();
        let deep_tiles = lake_terrain
            .iter()
            .enumerate()
            .filter_map(|(index, terrain_id)| {
                (terrain_id == "demo.terrain.resonance-water-deep").then_some(Position {
                    x: i32::try_from(index % usize::from(lake_definition.width))
                        .expect("deep lake x must fit i32"),
                    y: i32::try_from(index / usize::from(lake_definition.width))
                        .expect("deep lake y must fit i32"),
                })
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(water_tiles.len(), 76);
        assert_eq!(deep_tiles.len(), 30);
        for expected in [&water_tiles, &deep_tiles] {
            let mut reached = BTreeSet::from([lake_origin]);
            let mut frontier = VecDeque::from([lake_origin]);
            while let Some(position) = frontier.pop_front() {
                for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                    let neighbor = Position {
                        x: position.x + dx,
                        y: position.y + dy,
                    };
                    if expected.contains(&neighbor) && reached.insert(neighbor) {
                        frontier.push_back(neighbor);
                    }
                }
            }
            assert_eq!(&reached, expected);
        }

        let mut river_game = Game::new(93);
        let river_definition = river_game
            .content
            .world(BUILT_IN_WORLD_ID)
            .expect("built-in world should exist")
            .procedural_floors
            .iter()
            .find(|floor| floor.id == "demo.floor.resonance-depth-10")
            .expect("fixture should contain the river floor")
            .clone();
        let mut river_terrain = vec![
            river_definition.wall_terrain_id.clone();
            usize::from(river_definition.width)
                * usize::from(river_definition.height)
        ];
        let target = Position {
            x: i32::from(river_definition.width / 2),
            y: i32::from(river_definition.height / 2),
        };
        river_game.generate_river(
            &river_definition,
            "demo.terrain.resonance-water-deep",
            "demo.terrain.resonance-water-shallow",
            target,
            &mut river_terrain,
        );
        let river_water_count = river_terrain
            .iter()
            .filter(|terrain_id| {
                matches!(
                    terrain_id.as_str(),
                    "demo.terrain.resonance-water-deep" | "demo.terrain.resonance-water-shallow"
                )
            })
            .count();
        assert_eq!(river_water_count, 52);
        assert_eq!(
            river_terrain[generated_terrain_index(river_definition.width, target)],
            "demo.terrain.resonance-water-deep"
        );
        assert!(
            (1..i32::from(river_definition.width - 1)).any(|x| {
                [1, i32::from(river_definition.height - 2)]
                    .into_iter()
                    .any(|y| {
                        river_terrain
                            [generated_terrain_index(river_definition.width, Position { x, y })]
                            == "demo.terrain.resonance-water-deep"
                    })
            }) || (1..i32::from(river_definition.height - 1)).any(|y| {
                [1, i32::from(river_definition.width - 2)]
                    .into_iter()
                    .any(|x| {
                        river_terrain
                            [generated_terrain_index(river_definition.width, Position { x, y })]
                            == "demo.terrain.resonance-water-deep"
                    })
            })
        );
    }

    #[test]
    fn maze_destroyed_regions_and_streamers_obey_geometric_budgets() {
        let mut game = Game::new(151);
        let (maze_definition, destroyed_definition) = {
            let world = game
                .content
                .world(BUILT_IN_WORLD_ID)
                .expect("built-in world should exist");
            (
                world
                    .procedural_floors
                    .iter()
                    .find(|floor| floor.id == "demo.floor.resonance-depth-9")
                    .expect("fixture should contain the maze floor")
                    .clone(),
                world
                    .procedural_floors
                    .iter()
                    .find(|floor| floor.id == "demo.floor.resonance-depth-10")
                    .expect("fixture should contain the destroyed floor")
                    .clone(),
            )
        };
        let maze_layout = maze_definition
            .layout
            .as_ref()
            .expect("fixture should contain a layout");
        let mut maze_terrain = vec![
            maze_definition.wall_terrain_id.clone();
            usize::from(maze_definition.width)
                * usize::from(maze_definition.height)
        ];
        let maze_tiles = game.generate_maze(
            &maze_definition,
            maze_layout
                .maze
                .as_ref()
                .expect("fixture should contain a maze"),
            "demo.terrain.resonant-floor",
            &mut maze_terrain,
        );
        assert_eq!(maze_tiles.len(), 127);
        let root = *maze_tiles
            .iter()
            .next()
            .expect("maze should contain a floor");
        let mut reached = BTreeSet::from([root]);
        let mut frontier = VecDeque::from([root]);
        while let Some(position) = frontier.pop_front() {
            for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                let neighbor = Position {
                    x: position.x + dx,
                    y: position.y + dy,
                };
                if maze_tiles.contains(&neighbor) && reached.insert(neighbor) {
                    frontier.push_back(neighbor);
                }
            }
        }
        assert_eq!(reached, maze_tiles);

        let before_streamers = maze_terrain.clone();
        let streamer_tiles =
            game.generate_streamers(&maze_definition, &maze_layout.streamers, &mut maze_terrain);
        assert_eq!(streamer_tiles.len(), 24);
        assert!(streamer_tiles.iter().all(|position| {
            before_streamers[generated_terrain_index(maze_definition.width, *position)]
                == maze_definition.wall_terrain_id
                && maze_terrain[generated_terrain_index(maze_definition.width, *position)]
                    == "demo.terrain.resonance-vein"
        }));

        let mut destroyed_terrain = vec![
            destroyed_definition.wall_terrain_id.clone();
            usize::from(destroyed_definition.width)
                * usize::from(destroyed_definition.height)
        ];
        let destroyed_tiles = game.generate_destroyed_region(
            &destroyed_definition,
            "demo.terrain.resonance-ruin",
            &mut destroyed_terrain,
        );
        assert_eq!(destroyed_tiles.len(), 48);
        assert!(destroyed_tiles.iter().all(|position| {
            destroyed_terrain[generated_terrain_index(destroyed_definition.width, *position)]
                == "demo.terrain.resonance-ruin"
        }));
        let mut remaining = destroyed_tiles.clone();
        let mut component_count = 0;
        while let Some(&start) = remaining.iter().next() {
            component_count += 1;
            let mut component_frontier = VecDeque::from([start]);
            remaining.remove(&start);
            while let Some(position) = component_frontier.pop_front() {
                for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                    let neighbor = Position {
                        x: position.x + dx,
                        y: position.y + dy,
                    };
                    if remaining.remove(&neighbor) {
                        component_frontier.push_back(neighbor);
                    }
                }
            }
        }
        assert!((1..=2).contains(&component_count));
    }

    #[test]
    fn maze_only_floor_uses_reachable_region_anchors_without_room_overlay() {
        let mut game = Game::new(151);
        game.player.position = Position { x: 3, y: 2 };
        game.traverse_stairs(false)
            .expect("pressure dungeon entry should resolve")
            .expect("pressure dungeon entry should transition");
        for _ in 1..9 {
            descend_one_floor(&mut game);
        }

        assert_eq!(game.current_floor_id, "demo.floor.resonance-depth-9");
        let walkable = game
            .terrain
            .iter()
            .enumerate()
            .filter_map(|(index, terrain_id)| {
                game.content
                    .terrain(terrain_id)
                    .is_some_and(|terrain| terrain.walkable)
                    .then_some(Position {
                        x: i32::try_from(index % usize::from(game.width))
                            .expect("maze x must fit i32"),
                        y: i32::try_from(index / usize::from(game.width))
                            .expect("maze y must fit i32"),
                    })
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(walkable.len(), 127);
        let (entry, remote) = maze_floor_anchors(&walkable);
        assert_eq!(game.player.position, entry);
        assert_eq!(game.terrain_at(entry), "demo.terrain.stairs-up");
        assert_eq!(game.terrain_at(remote), "demo.terrain.stairs-down");
        assert_eq!(maze_floor_distances(&walkable, entry).len(), walkable.len());
        assert!(
            game.terrain
                .iter()
                .all(|terrain| terrain != "demo.terrain.door-secret")
        );
        assert!(game.entities.iter().all(|entity| {
            entity.id.contains(".encounter.") && walkable.contains(&entity.position)
        }));
        assert!(game.items.iter().all(|item| {
            matches!(item.location, ItemLocation::Ground(position) if walkable.contains(&position))
        }));

        let mut same_seed = Game::new(151);
        same_seed.player.position = Position { x: 3, y: 2 };
        same_seed
            .traverse_stairs(false)
            .expect("matching pressure dungeon entry should resolve")
            .expect("matching pressure dungeon entry should transition");
        for _ in 1..9 {
            descend_one_floor(&mut same_seed);
        }
        assert_eq!(same_seed.state_hash(), game.state_hash());
    }

    #[test]
    fn dynamic_friends_and_escorts_obey_group_budgets_and_formations() {
        let mut game = Game::new(49);
        game.player.position = Position { x: 3, y: 2 };
        game.traverse_stairs(false)
            .expect("pressure dungeon entry should resolve")
            .expect("pressure dungeon entry should transition");
        for _ in 1..6 {
            descend_one_floor(&mut game);
        }

        assert_eq!(game.current_floor_id, "demo.floor.resonance-depth-6");
        assert_eq!(game.entities.len(), 7);
        let captain = game
            .entities
            .iter()
            .find(|entity| entity.kind_id == "demo.actor.chorus-captain")
            .expect("depth six should contain one chorus captain");
        let captain_position = captain.position;
        let friends = game
            .entities
            .iter()
            .filter(|entity| entity.id.contains(".friend."))
            .collect::<Vec<_>>();
        let escorts = game
            .entities
            .iter()
            .filter(|entity| entity.id.contains(".escort."))
            .collect::<Vec<_>>();
        assert!((1..=2).contains(&friends.len()));
        assert!((1..=2).contains(&escorts.len()));
        assert!(friends.len() + escorts.len() <= 4);
        assert!(friends.iter().all(|friend| {
            friend.kind_id == "demo.actor.chorus-captain"
                && adjacent(friend.position, captain_position)
        }));
        assert!(escorts.iter().all(|escort| {
            matches!(
                escort.kind_id.as_str(),
                "demo.actor.frost-wisp" | "demo.actor.storm-spark"
            ) && adjacent(escort.position, captain_position)
        }));
        let captain_pack = captain
            .pack
            .as_ref()
            .expect("dynamic leader should retain a pack identity");
        assert_eq!(captain_pack.role, MonsterPackRoleDto::Leader);
        assert_eq!(captain_pack.behavior, MonsterPackBehaviorDto::Seek);
        assert!(friends.iter().all(|friend| {
            friend.pack.as_ref().is_some_and(|pack| {
                pack.id == captain_pack.id
                    && pack.leader_id == captain.id
                    && pack.role == MonsterPackRoleDto::Member
                    && pack.behavior == MonsterPackBehaviorDto::Surround
            })
        }));
        assert!(escorts.iter().all(|escort| {
            escort.pack.as_ref().is_some_and(|pack| {
                pack.id == captain_pack.id
                    && pack.leader_id == captain.id
                    && pack.role == MonsterPackRoleDto::Member
                    && pack.behavior == MonsterPackBehaviorDto::GuardLeader
            })
        }));
        let captain_region_id = region_at(&game, captain_position).region_id.clone();
        assert!(
            game.entities
                .iter()
                .filter(|entity| entity.pack.is_some())
                .all(|entity| region_at(&game, entity.position).region_id == captain_region_id)
        );
        assert!(
            game.entities
                .iter()
                .filter(|entity| entity.pack.is_none())
                .all(
                    |entity| match region_at(&game, entity.position).region_id.as_str() {
                        "demo.region.resonance-grotto" => matches!(
                            entity.kind_id.as_str(),
                            "demo.actor.acid-seep" | "demo.actor.venom-spore"
                        ),
                        "demo.region.resonance-gallery" => matches!(
                            entity.kind_id.as_str(),
                            "demo.actor.echo-hound" | "demo.actor.storm-spark"
                        ),
                        _ => false,
                    }
                )
        );
        let room_feature_positions = game
            .terrain
            .iter()
            .enumerate()
            .filter_map(|(index, terrain_id)| {
                matches!(
                    terrain_id.as_str(),
                    "demo.terrain.trap-echo-snare" | "demo.terrain.echo-rubble"
                )
                .then_some(Position {
                    x: i32::try_from(index % usize::from(game.width)).expect("x must fit i32"),
                    y: i32::try_from(index / usize::from(game.width)).expect("y must fit i32"),
                })
            })
            .collect::<Vec<_>>();
        assert!(room_feature_positions.len() >= 2);
        assert!(
            room_feature_positions
                .iter()
                .all(|position| !region_at(&game, *position).region_id.is_empty())
        );
        descend_one_floor(&mut game);
        assert_eq!(game.current_floor_id, "demo.floor.resonance-depth-7");
        assert_eq!(game.entities.len(), 8);
        let shepherd = game
            .entities
            .iter()
            .find(|entity| entity.kind_id == "demo.actor.spore-shepherd")
            .expect("depth seven should contain one spore shepherd");
        let shepherd_position = shepherd.position;
        let friends = game
            .entities
            .iter()
            .filter(|entity| entity.id.contains(".friend."))
            .collect::<Vec<_>>();
        let escorts = game
            .entities
            .iter()
            .filter(|entity| entity.id.contains(".escort."))
            .collect::<Vec<_>>();
        assert!((1..=2).contains(&friends.len()));
        assert!((2..=3).contains(&escorts.len()));
        assert!(friends.len() + escorts.len() <= 5);
        assert!(friends.iter().all(|friend| {
            friend.kind_id == "demo.actor.spore-shepherd"
                && adjacent(friend.position, shepherd_position)
        }));
        assert!(escorts.iter().all(|escort| {
            matches!(
                escort.kind_id.as_str(),
                "demo.actor.venom-spore" | "demo.actor.echo-hound"
            ) && adjacent(escort.position, shepherd_position)
        }));
        let shepherd_region_id = region_at(&game, shepherd_position).region_id.clone();
        assert!(
            game.entities
                .iter()
                .filter(|entity| entity.pack.is_some())
                .all(|entity| region_at(&game, entity.position).region_id == shepherd_region_id)
        );

        let restored =
            Game::from_save(game.to_save()).expect("dynamic encounter groups should round-trip");
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(
            actors_to_save(&restored.entities),
            actors_to_save(&game.entities)
        );
    }

    #[test]
    fn pack_ai_reserves_surround_targets_and_guards_the_leader() {
        let mut game = Game::new(42);
        game.player.position = Position { x: 10, y: 10 };
        let base = game.entities[0].clone();
        let pack_id = "test.pack.1";
        let leader_id = "test.pack.leader";
        let pack = |role, behavior| {
            Some(MonsterPackIdentity {
                id: pack_id.to_owned(),
                leader_id: leader_id.to_owned(),
                role,
                behavior,
            })
        };
        let mut leader = base.clone();
        leader.id = leader_id.to_owned();
        leader.position = Position { x: 9, y: 7 };
        leader.pack = pack(MonsterPackRoleDto::Leader, MonsterPackBehaviorDto::Seek);
        let mut friend_one = base.clone();
        friend_one.id = "test.pack.friend.1".to_owned();
        friend_one.position = Position { x: 7, y: 9 };
        friend_one.pack = pack(MonsterPackRoleDto::Member, MonsterPackBehaviorDto::Surround);
        let mut friend_two = base.clone();
        friend_two.id = "test.pack.friend.2".to_owned();
        friend_two.position = Position { x: 7, y: 11 };
        friend_two.pack = pack(MonsterPackRoleDto::Member, MonsterPackBehaviorDto::Surround);
        let mut escort = base;
        escort.id = "test.pack.escort.1".to_owned();
        escort.position = Position { x: 6, y: 7 };
        escort.pack = pack(
            MonsterPackRoleDto::Member,
            MonsterPackBehaviorDto::GuardLeader,
        );
        game.entities = vec![leader, friend_one, friend_two, escort];
        game.items.clear();

        let mut reservations = BTreeSet::new();
        assert!(game.next_surround_step(1, &mut reservations).is_some());
        assert!(game.next_surround_step(2, &mut reservations).is_some());
        assert_eq!(reservations.len(), 2);
        assert!(reservations.iter().all(|target| {
            adjacent(*target, game.player.position) && game.is_walkable(*target)
        }));

        let leader_position = game.entities[0].position;
        let before = squared_distance(game.entities[3].position, leader_position);
        game.resolve_monster_action(
            3,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
        );
        assert!(squared_distance(game.entities[3].position, leader_position) < before);

        let restored = Game::from_save(game.to_save()).expect("pack state should round-trip");
        assert_eq!(
            actors_to_save(&restored.entities),
            actors_to_save(&game.entities)
        );
        assert_eq!(restored.state_hash(), game.state_hash());
    }

    #[test]
    fn malformed_pack_identity_is_rejected_and_v58_remains_independent() {
        let mut malformed = Game::new(42).to_save();
        malformed.entities[0].pack = Some(rfb_protocol::MonsterPackSaveDto {
            id: "test.pack.missing-leader".to_owned(),
            leader_id: "test.actor.missing".to_owned(),
            role: MonsterPackRoleDto::Member,
            behavior: MonsterPackBehaviorDto::GuardLeader,
        });
        assert!(matches!(
            Game::from_save(malformed),
            Err(CoreError::InvalidSave("monster pack state is invalid"))
        ));

        let mut legacy = Game::new(49);
        legacy.player.position = Position { x: 3, y: 2 };
        legacy
            .traverse_stairs(false)
            .expect("pressure dungeon entry should resolve")
            .expect("pressure dungeon entry should transition");
        for _ in 1..6 {
            descend_one_floor(&mut legacy);
        }
        let mut payload = legacy.to_save();
        payload.content_hash =
            "ee07c276bbe568fafc1e1d6942e9d57d158bd250ed452b32c01c774d8521e96d".to_owned();
        for entity in &mut payload.entities {
            entity.pack = None;
        }
        let restored = Game::from_save(payload).expect("v58 actors without pack state should load");
        assert!(restored.entities.iter().all(|entity| entity.pack.is_none()));
    }

    #[test]
    fn terrain_features_filter_by_depth_and_remain_deterministic() {
        let mut locked_door_seeds = 0;
        let mut secret_door_seeds = 0;
        for seed in 1..=64 {
            let mut left = Game::new(seed);
            let mut right = Game::new(seed);
            for game in [&mut left, &mut right] {
                game.player.position = Position { x: 3, y: 2 };
                game.traverse_stairs(false)
                    .expect("pressure dungeon entry should resolve")
                    .expect("pressure dungeon entry should transition");
                descend_one_floor(game);
                descend_one_floor(game);
            }
            assert_eq!(left.current_floor_id, "demo.floor.resonance-depth-3");
            assert_eq!(left.state_hash(), right.state_hash());
            assert_eq!(
                left.terrain
                    .iter()
                    .filter(|terrain| {
                        matches!(
                            terrain.as_str(),
                            "demo.terrain.trap-echo-snare" | "demo.terrain.echo-rubble"
                        )
                    })
                    .count(),
                3
            );
            assert!(
                !left
                    .terrain
                    .iter()
                    .any(|terrain| terrain == "demo.terrain.door-locked")
            );

            descend_one_floor(&mut left);
            if left
                .terrain
                .iter()
                .any(|terrain| terrain == "demo.terrain.door-locked")
            {
                locked_door_seeds += 1;
            }
            assert_eq!(
                left.terrain
                    .iter()
                    .filter(|terrain| *terrain == "demo.terrain.door-secret")
                    .count(),
                1
            );

            descend_one_floor(&mut left);
            descend_one_floor(&mut left);
            if left
                .terrain
                .iter()
                .filter(|terrain| *terrain == "demo.terrain.door-secret")
                .count()
                > 1
            {
                secret_door_seeds += 1;
            }
        }
        assert!(locked_door_seeds > 0);
        assert!(secret_door_seeds > 0);
    }

    #[test]
    fn terrain_feature_space_failure_falls_back_without_overlap() {
        let seed = (1..=64)
            .find(|seed| {
                let mut rng = RfbRng::seeded(*seed);
                rng.bounded(101) < 100
            })
            .expect("a seed should select the impossible corridor candidate first");
        let mut game = Game::new(seed);
        let mut definition = game
            .content
            .world(BUILT_IN_WORLD_ID)
            .expect("demo world should remain available")
            .procedural_floors
            .iter()
            .find(|floor| floor.id == "demo.floor.resonance-depth-3")
            .expect("fixture should contain a terrain feature floor")
            .clone();
        definition.width = 4;
        definition.height = 4;
        definition
            .generation_budget
            .as_mut()
            .expect("fixture should contain a generation budget")
            .feature_placements = Some(2);
        let rooms = [GeneratedRoom {
            id: "entry".to_owned(),
            x: 1,
            y: 1,
            width: 1,
            height: 1,
            shape: ProceduralRoomShape::Rectangle,
        }];
        let target = Position { x: 1, y: 1 };
        let mut terrain = vec!["demo.terrain.wall".to_owned(); 16];
        set_generated_terrain(&mut terrain, definition.width, target, "demo.terrain.floor");
        let entries = [
            TerrainFeatureEntryDefinition {
                terrain_id: "demo.terrain.door-locked".to_owned(),
                placement: TerrainFeaturePlacement::Corridor,
                weight: 100,
                min_depth: 1,
                max_depth: 10,
            },
            TerrainFeatureEntryDefinition {
                terrain_id: "demo.terrain.trap-echo-snare".to_owned(),
                placement: TerrainFeaturePlacement::Room,
                weight: 1,
                min_depth: 1,
                max_depth: 10,
            },
        ];

        let placements = game.place_terrain_features(
            &definition,
            &entries,
            TerrainFeaturePlacementContext {
                rooms: &rooms,
                reserved: &BTreeSet::new(),
                floor_terrain_id: "demo.terrain.floor",
                room_floor_terrain_ids: &BTreeSet::new(),
            },
            &mut terrain,
        );

        assert_eq!(placements.len(), 1);
        assert_eq!(placements[0].position, target);
        assert_eq!(placements[0].terrain_id, "demo.terrain.trap-echo-snare");
        assert_eq!(
            terrain_feature_placement_candidates(
                &terrain,
                definition.width,
                "demo.terrain.floor",
                &BTreeSet::new(),
                &rooms,
                &BTreeSet::new(),
                TerrainFeaturePlacement::Room,
            ),
            Vec::<Position>::new()
        );
    }

    #[test]
    fn formation_space_pressure_shrinks_then_falls_back_atomically() {
        let seed = (1..=64)
            .find(|seed| {
                let mut rng = RfbRng::seeded(*seed);
                rng.bounded(2) == 1 && rng.bounded(2) == 1
            })
            .expect("a seed should request both maximum companion counts");
        let mut game = Game::new(seed);
        let definition = game
            .content
            .world(BUILT_IN_WORLD_ID)
            .expect("built-in world should exist")
            .procedural_floors
            .iter()
            .find(|floor| floor.id == "demo.floor.resonance-depth-6")
            .expect("fixture should contain the ring formation floor")
            .clone();
        let mut table = game
            .content
            .encounter_table("demo.encounter-table.resonance-formations")
            .expect("fixture should contain the formation encounter table")
            .clone();
        table.rolls = 1;
        let eligible_entries = table
            .entries
            .iter()
            .filter(|entry| entry.min_depth <= 6 && 6 <= entry.max_depth)
            .cloned()
            .collect::<Vec<_>>();
        let rooms = [GeneratedRoom {
            id: "remote".to_owned(),
            x: 0,
            y: 0,
            width: 3,
            height: 3,
            shape: ProceduralRoomShape::Rectangle,
        }];
        let free = BTreeSet::from([
            Position { x: 1, y: 0 },
            Position { x: 1, y: 1 },
            Position { x: 1, y: 2 },
        ]);
        let mut occupied = (0..3)
            .flat_map(|y| (0..3).map(move |x| Position { x, y }))
            .filter(|position| !free.contains(position))
            .collect::<BTreeSet<_>>();

        let shrunk = game.generate_dynamic_encounter_groups(
            &definition,
            &table,
            &eligible_entries,
            &rooms,
            "remote",
            0,
            1,
            true,
            &definition.id,
            &mut occupied,
        );
        assert_eq!(shrunk.len(), 3);
        assert_eq!(
            shrunk
                .iter()
                .filter(|actor| actor.id.contains(".friend.") || actor.id.contains(".escort."))
                .count(),
            2
        );

        let mut left = Game::new(seed);
        let mut right = Game::new(seed);
        let only_one_free = BTreeSet::from([Position { x: 1, y: 1 }]);
        let occupied = (0..3)
            .flat_map(|y| (0..3).map(move |x| Position { x, y }))
            .filter(|position| !only_one_free.contains(position))
            .collect::<BTreeSet<_>>();
        let mut left_occupied = occupied.clone();
        let mut right_occupied = occupied;
        let left_generated = left.generate_dynamic_encounter_groups(
            &definition,
            &table,
            &eligible_entries,
            &rooms,
            "remote",
            0,
            1,
            true,
            &definition.id,
            &mut left_occupied,
        );
        let right_generated = right.generate_dynamic_encounter_groups(
            &definition,
            &table,
            &eligible_entries,
            &rooms,
            "remote",
            0,
            1,
            true,
            &definition.id,
            &mut right_occupied,
        );
        assert_eq!(left_generated, right_generated);
        assert_eq!(left_generated.len(), 1);
        assert!(left_generated[0].id.ends_with(".encounter.1"));
        assert!(!left_generated[0].id.contains(".friend."));
        assert!(!left_generated[0].id.contains(".escort."));
    }

    #[test]
    fn vault_coordinate_transforms_cover_rotations_and_reflections() {
        let game = Game::new(1);
        let vault = game
            .content
            .vault("demo.vault.resonance-spindle")
            .expect("fixture should contain the transformable Vault");

        assert_eq!(
            transformed_vault_dimensions(vault, VaultTransform::Rotate90),
            (4, 3)
        );
        assert_eq!(
            transformed_vault_position(
                vault,
                VaultTransform::Rotate90,
                vault.entrance_positions[0]
            ),
            Position { x: 3, y: 1 }
        );
        assert_eq!(
            transformed_vault_position(
                vault,
                VaultTransform::MirrorHorizontal,
                ContentPosition { x: 0, y: 1 }
            ),
            Position { x: 2, y: 1 }
        );
        assert_eq!(
            transformed_vault_position(
                vault,
                VaultTransform::MirrorMainDiagonal,
                ContentPosition { x: 0, y: 1 }
            ),
            Position { x: 1, y: 0 }
        );
        assert_eq!(
            transformed_vault_position(
                vault,
                VaultTransform::MirrorAntiDiagonal,
                ContentPosition { x: 0, y: 1 }
            ),
            Position { x: 2, y: 2 }
        );
    }

    #[test]
    fn spatial_vault_placement_falls_back_after_an_impossible_weighted_candidate() {
        let mut game = Game::new(1);
        let definition = game
            .content
            .world(BUILT_IN_WORLD_ID)
            .expect("built-in world should exist")
            .procedural_floors
            .iter()
            .find(|floor| floor.id == "demo.floor.resonance-depth-8")
            .expect("fixture should contain the spatial Vault floor")
            .clone();
        let theme = game
            .content
            .theme_table("demo.theme-table.resonance-descent")
            .expect("fixture should contain the pressure theme table")
            .entries
            .iter()
            .find(|entry| entry.min_depth <= 8 && 8 <= entry.max_depth)
            .expect("fixture should contain the deep theme");
        let mut impossible = theme
            .vault_candidates
            .iter()
            .find(|candidate| candidate.vault_id == "demo.vault.sealed-resonance-monolith")
            .expect("fixture should contain the impossible candidate")
            .clone();
        impossible.weight = u32::MAX;
        let mut fallback = theme
            .vault_candidates
            .iter()
            .find(|candidate| candidate.vault_id == "demo.vault.resonance-spindle")
            .expect("fixture should contain the fallback candidate")
            .clone();
        fallback.weight = 1;
        let mut probe = RfbRng::seeded(1);
        assert!(probe.bounded(u64::from(u32::MAX) + 1) < u64::from(u32::MAX));

        let mut terrain = vec![
            definition.wall_terrain_id.clone();
            usize::from(definition.width) * usize::from(definition.height)
        ];
        for x in 1..i32::from(definition.width - 1) {
            set_generated_terrain(
                &mut terrain,
                definition.width,
                Position { x, y: 10 },
                "demo.terrain.resonant-floor",
            );
        }
        let placements = game.select_spatial_vault_placements(
            &definition,
            &[impossible, fallback],
            false,
            "demo.terrain.resonant-floor",
            &mut terrain,
        );

        assert_eq!(placements.len(), 1);
        assert_eq!(placements[0].vault.id, "demo.vault.resonance-spindle");
    }

    #[test]
    fn large_multi_entrance_vault_stitches_into_a_connected_floor() {
        let mut game = Game::new(64);
        let definition = game
            .content
            .world(BUILT_IN_WORLD_ID)
            .expect("built-in world should exist")
            .procedural_floors
            .iter()
            .find(|floor| floor.id == "demo.floor.resonance-depth-8")
            .expect("fixture should contain the spatial Vault floor")
            .clone();
        let candidate = ThemeVaultCandidateDefinition {
            vault_id: "demo.vault.resonance-crossroads".to_owned(),
            weight: 1,
            min_depth: 8,
            max_depth: 8,
        };
        let mut terrain = vec![
            definition.wall_terrain_id.clone();
            usize::from(definition.width) * usize::from(definition.height)
        ];
        for x in 1..i32::from(definition.width - 1) {
            set_generated_terrain(
                &mut terrain,
                definition.width,
                Position { x, y: 10 },
                "demo.terrain.resonant-floor",
            );
        }
        for y in 1..i32::from(definition.height - 1) {
            set_generated_terrain(
                &mut terrain,
                definition.width,
                Position { x: 10, y },
                "demo.terrain.resonant-floor",
            );
        }

        let placements = game.select_spatial_vault_placements(
            &definition,
            &[candidate],
            false,
            "demo.terrain.resonant-floor",
            &mut terrain,
        );

        assert_eq!(placements.len(), 1);
        let placement = &placements[0];
        assert_eq!(placement.vault.entrance_positions.len(), 4);
        assert!(!placement.connector_cells.is_empty());
        assert!(placement.connector_cells.iter().all(|position| {
            terrain[generated_terrain_index(definition.width, *position)]
                == "demo.terrain.resonant-floor"
        }));
        assert!(generated_terrain_is_connected(
            &terrain,
            definition.width,
            definition.height,
            &game.content,
        ));
        let (vault_width, vault_height) =
            transformed_vault_dimensions(&placement.vault, placement.transform);
        for entrance in &placement.vault.entrance_positions {
            let entrance =
                transformed_vault_position(&placement.vault, placement.transform, *entrance);
            let outward = vault_entrance_outward(entrance, vault_width, vault_height);
            let outside = Position {
                x: placement.origin.x + entrance.x + outward.x,
                y: placement.origin.y + entrance.y + outward.y,
            };
            assert!(terrain_is_connectable(
                &game.content,
                &terrain[generated_terrain_index(definition.width, outside)]
            ));
        }
    }

    #[test]
    fn previous_v63_generated_floor_is_not_rebuilt_for_multi_entry_vaults() {
        let mut game = Game::new(93);
        game.player.position = Position { x: 3, y: 2 };
        game.traverse_stairs(false)
            .expect("pressure dungeon entry should resolve")
            .expect("pressure dungeon entry should transition");
        for _ in 1..8 {
            descend_one_floor(&mut game);
        }
        assert_eq!(game.current_floor_id, "demo.floor.resonance-depth-8");

        let mut payload = game.to_save();
        payload.content_hash =
            "246f51864965fac494c7a39959f591caa0434d9fa4eac839501f9d09526eb617".to_owned();
        let expected_terrain = payload.terrain.clone();
        let expected_entities = payload.entities.clone();
        let expected_items = payload.items.clone();
        let expected_draw_counter = payload.rng.draw_counter;

        let restored = Game::from_save(payload).expect("v63 generated floor should migrate");

        assert_eq!(restored.to_save().terrain, expected_terrain);
        assert_eq!(actors_to_save(&restored.entities), expected_entities);
        assert_eq!(items_to_save(&restored.items), expected_items);
        assert_eq!(restored.rng.draw_counter, expected_draw_counter);
    }

    #[test]
    fn previous_v49_generated_floor_is_not_backfilled_with_spatial_vaults() {
        let mut game = Game::new(49);
        game.player.position = Position { x: 3, y: 2 };
        game.traverse_stairs(false)
            .expect("pressure dungeon entry should resolve")
            .expect("pressure dungeon entry should transition");
        for _ in 1..8 {
            descend_one_floor(&mut game);
        }
        let mut payload = game.to_save();
        payload.content_hash =
            "5d65fd9ca827dd05fc035650b82046edb592d563565c7e4075b32512a43f4e1f".to_owned();
        let removed_positions = payload
            .entities
            .iter()
            .filter(|entity| entity.id.contains(".vault."))
            .map(|entity| entity.position)
            .collect::<Vec<_>>();
        payload
            .entities
            .retain(|entity| !entity.id.contains(".vault."));
        for position in removed_positions {
            let index =
                position.y as usize * usize::from(payload.terrain.width) + position.x as usize;
            payload.terrain.terrain_ids[index] = "demo.terrain.wall".to_owned();
        }
        let expected_terrain = payload.terrain.clone();
        let expected_entities = payload.entities.clone();
        let saved_draw_counter = payload.rng.draw_counter;

        let restored = Game::from_save(payload).expect("v49 generated floor should migrate");

        assert_eq!(restored.current_floor_id, "demo.floor.resonance-depth-8");
        assert_eq!(restored.to_save().terrain, expected_terrain);
        assert_eq!(actors_to_save(&restored.entities), expected_entities);
        assert_eq!(restored.rng.draw_counter, saved_draw_counter);
        assert!(
            restored
                .entities
                .iter()
                .all(|entity| !entity.id.contains(".vault."))
        );
    }

    #[test]
    fn previous_v50_generated_floor_is_not_backfilled_with_dynamic_groups() {
        let mut game = Game::new(49);
        game.player.position = Position { x: 3, y: 2 };
        game.traverse_stairs(false)
            .expect("pressure dungeon entry should resolve")
            .expect("pressure dungeon entry should transition");
        for _ in 1..6 {
            descend_one_floor(&mut game);
        }
        let mut payload = game.to_save();
        payload.content_hash =
            "7eea25faef326b6d2250af357359902d0acf32d393c831655508a7e7eee5f2f0".to_owned();
        payload.entities.retain(|entity| entity.pack.is_none());
        let expected_terrain = payload.terrain.clone();
        let expected_entities = payload.entities.clone();
        let saved_draw_counter = payload.rng.draw_counter;

        let restored = Game::from_save(payload).expect("v50 generated floor should migrate");

        assert_eq!(restored.current_floor_id, "demo.floor.resonance-depth-6");
        assert_eq!(restored.to_save().terrain, expected_terrain);
        assert_eq!(actors_to_save(&restored.entities), expected_entities);
        assert_eq!(restored.rng.draw_counter, saved_draw_counter);
        assert!(
            restored.entities.iter().all(|entity| {
                !entity.id.contains(".friend.") && !entity.id.contains(".escort.")
            })
        );
    }

    #[test]
    fn previous_v51_generated_floor_is_not_backfilled_with_terrain_features() {
        let mut game = Game::new(49);
        game.player.position = Position { x: 3, y: 2 };
        game.traverse_stairs(false)
            .expect("pressure dungeon entry should resolve")
            .expect("pressure dungeon entry should transition");
        descend_one_floor(&mut game);
        descend_one_floor(&mut game);
        let mut payload = game.to_save();
        payload.content_hash =
            "de045e1652d6e484937743b84a98e5e77887f28340a6492e72e8c6e1f72326e6".to_owned();
        let fixed_trap_position = Position {
            x: payload.player.position.x,
            y: payload.player.position.y + 1,
        };
        for index in 0..payload.terrain.terrain_ids.len() {
            let position = Position {
                x: i32::try_from(index % usize::from(payload.terrain.width))
                    .expect("terrain x must fit i32"),
                y: i32::try_from(index / usize::from(payload.terrain.width))
                    .expect("terrain y must fit i32"),
            };
            if payload.terrain.terrain_ids[index] == "demo.terrain.echo-rubble"
                || payload.terrain.terrain_ids[index] == "demo.terrain.trap-echo-snare"
                    && position != fixed_trap_position
            {
                payload.terrain.terrain_ids[index] = "demo.terrain.floor".to_owned();
            }
        }
        let expected_terrain = payload.terrain.clone();
        let expected_entities = payload.entities.clone();
        let saved_draw_counter = payload.rng.draw_counter;

        let restored = Game::from_save(payload).expect("v51 generated floor should migrate");

        assert_eq!(restored.current_floor_id, "demo.floor.resonance-depth-3");
        assert_eq!(restored.to_save().terrain, expected_terrain);
        assert_eq!(actors_to_save(&restored.entities), expected_entities);
        assert_eq!(restored.rng.draw_counter, saved_draw_counter);
        assert!(
            !restored
                .terrain
                .iter()
                .any(|terrain| terrain == "demo.terrain.echo-rubble")
        );
        assert_eq!(
            restored
                .terrain
                .iter()
                .filter(|terrain| *terrain == "demo.terrain.trap-echo-snare")
                .count(),
            1
        );
    }

    #[test]
    fn previous_v52_generated_floor_is_not_backfilled_with_layout_terrain() {
        let mut game = Game::new(49);
        game.player.position = Position { x: 3, y: 2 };
        game.traverse_stairs(false)
            .expect("pressure dungeon entry should resolve")
            .expect("pressure dungeon entry should transition");
        for _ in 1..9 {
            descend_one_floor(&mut game);
        }
        let mut payload = game.to_save();
        payload.content_hash =
            "1f8848e160b4ec51ca36acc512920946888fec20a36d7ac7b860bdb126aff79a".to_owned();
        for terrain_id in &mut payload.terrain.terrain_ids {
            if terrain_id == "demo.terrain.resonance-cavern" {
                *terrain_id = "demo.terrain.wall".to_owned();
            }
        }
        let expected_terrain = payload.terrain.clone();
        let expected_entities = payload.entities.clone();
        let expected_items = payload.items.clone();
        let saved_draw_counter = payload.rng.draw_counter;

        let restored = Game::from_save(payload).expect("v52 generated floor should migrate");

        assert_eq!(restored.current_floor_id, "demo.floor.resonance-depth-9");
        assert_eq!(restored.to_save().terrain, expected_terrain);
        assert_eq!(actors_to_save(&restored.entities), expected_entities);
        assert_eq!(items_to_save(&restored.items), expected_items);
        assert_eq!(restored.rng.draw_counter, saved_draw_counter);
        assert!(
            restored
                .terrain
                .iter()
                .all(|terrain| terrain != "demo.terrain.resonance-cavern")
        );
    }

    #[test]
    fn previous_v53_generated_floor_is_not_backfilled_with_hydrology() {
        let mut game = Game::new(77);
        game.player.position = Position { x: 3, y: 2 };
        game.traverse_stairs(false)
            .expect("pressure dungeon entry should resolve")
            .expect("pressure dungeon entry should transition");
        for _ in 1..9 {
            descend_one_floor(&mut game);
        }
        let mut payload = game.to_save();
        payload.content_hash =
            "11a28d24125572468148dce77f0082340ab82a3a7ef87637303578681b31c4e9".to_owned();
        for terrain_id in &mut payload.terrain.terrain_ids {
            if matches!(
                terrain_id.as_str(),
                "demo.terrain.resonance-water-deep" | "demo.terrain.resonance-water-shallow"
            ) {
                *terrain_id = "demo.terrain.resonance-cavern".to_owned();
            }
        }
        let expected_terrain = payload.terrain.clone();
        let expected_entities = payload.entities.clone();
        let expected_items = payload.items.clone();
        let saved_draw_counter = payload.rng.draw_counter;

        let restored = Game::from_save(payload).expect("v53 generated floor should migrate");

        assert_eq!(restored.current_floor_id, "demo.floor.resonance-depth-9");
        assert_eq!(restored.to_save().terrain, expected_terrain);
        assert_eq!(actors_to_save(&restored.entities), expected_entities);
        assert_eq!(items_to_save(&restored.items), expected_items);
        assert_eq!(restored.rng.draw_counter, saved_draw_counter);
        assert!(restored.terrain.iter().all(|terrain| {
            !matches!(
                terrain.as_str(),
                "demo.terrain.resonance-water-deep" | "demo.terrain.resonance-water-shallow"
            )
        }));
    }

    #[test]
    fn previous_v54_generated_floors_are_not_backfilled_with_late_terrain_stages() {
        let mut game = Game::new(151);
        game.player.position = Position { x: 3, y: 2 };
        game.traverse_stairs(false)
            .expect("pressure dungeon entry should resolve")
            .expect("pressure dungeon entry should transition");
        for _ in 1..10 {
            descend_one_floor(&mut game);
        }
        let mut payload = game.to_save();
        payload.content_hash =
            "e3c0d8653f86663c6bb7eb2cf99caf9d1ba5a259566560d7d70bb9592de2b1e9".to_owned();
        for terrain_id in &mut payload.terrain.terrain_ids {
            if matches!(
                terrain_id.as_str(),
                "demo.terrain.resonance-vein" | "demo.terrain.resonance-ruin"
            ) {
                *terrain_id = "demo.terrain.wall".to_owned();
            }
        }
        for floor in &mut payload.stored_floors {
            for terrain_id in &mut floor.terrain.terrain_ids {
                if matches!(
                    terrain_id.as_str(),
                    "demo.terrain.resonance-vein" | "demo.terrain.resonance-ruin"
                ) {
                    *terrain_id = "demo.terrain.wall".to_owned();
                }
            }
        }
        let expected_terrain = payload.terrain.clone();
        let expected_stored_floors = payload.stored_floors.clone();
        let expected_entities = payload.entities.clone();
        let expected_items = payload.items.clone();
        let saved_draw_counter = payload.rng.draw_counter;

        let restored = Game::from_save(payload).expect("v54 generated floors should migrate");
        let restored_payload = restored.to_save();

        assert_eq!(restored.current_floor_id, "demo.floor.resonance-depth-10");
        assert_eq!(restored_payload.terrain, expected_terrain);
        assert_eq!(restored_payload.stored_floors, expected_stored_floors);
        assert_eq!(actors_to_save(&restored.entities), expected_entities);
        assert_eq!(items_to_save(&restored.items), expected_items);
        assert_eq!(restored.rng.draw_counter, saved_draw_counter);
        assert!(restored.terrain.iter().all(|terrain| {
            !matches!(
                terrain.as_str(),
                "demo.terrain.resonance-vein" | "demo.terrain.resonance-ruin"
            )
        }));
    }

    #[test]
    fn previous_v55_generated_floor_is_not_backfilled_with_a_pit() {
        let mut game = Game::new(156);
        game.player.position = Position { x: 3, y: 2 };
        game.traverse_stairs(false)
            .expect("pressure dungeon entry should resolve")
            .expect("pressure dungeon entry should transition");
        for _ in 1..9 {
            descend_one_floor(&mut game);
        }
        let mut payload = game.to_save();
        payload.content_hash =
            "52c3db16ad5240ff83ba652b09ef70cccac991a586b593f84c11956a55539596".to_owned();
        payload
            .entities
            .retain(|entity| !entity.id.contains(".pit."));
        let expected_entities = payload.entities.clone();
        let expected_terrain = payload.terrain.clone();
        let expected_items = payload.items.clone();
        let saved_draw_counter = payload.rng.draw_counter;

        let restored = Game::from_save(payload).expect("v55 generated floor should migrate");

        assert_eq!(restored.current_floor_id, "demo.floor.resonance-depth-9");
        assert_eq!(actors_to_save(&restored.entities), expected_entities);
        assert_eq!(restored.to_save().terrain, expected_terrain);
        assert_eq!(items_to_save(&restored.items), expected_items);
        assert_eq!(restored.rng.draw_counter, saved_draw_counter);
        assert!(
            restored
                .entities
                .iter()
                .all(|entity| !entity.id.contains(".pit."))
        );
    }

    #[test]
    fn previous_v56_generated_floor_is_not_rebuilt_as_maze_only() {
        let mut game = Game::new(156);
        game.player.position = Position { x: 3, y: 2 };
        game.traverse_stairs(false)
            .expect("pressure dungeon entry should resolve")
            .expect("pressure dungeon entry should transition");
        for _ in 1..9 {
            descend_one_floor(&mut game);
        }
        let mut payload = game.to_save();
        payload.content_hash =
            "461242cb2164434a7ef44a3692f1c9fa4ffe9921f07c17e0857c96f2f2d95041".to_owned();
        payload.entities[0].id = "demo.floor.resonance-depth-9.pit.1".to_owned();
        let marker_index = payload
            .terrain
            .terrain_ids
            .iter()
            .position(|terrain| terrain == "demo.terrain.wall")
            .expect("generated floor should retain a wall");
        payload.terrain.terrain_ids[marker_index] = "demo.terrain.resonance-cavern".to_owned();
        let expected_terrain = payload.terrain.clone();
        let mut expected_entities = payload.entities.clone();
        expected_entities.sort_by(|left, right| left.id.cmp(&right.id));
        let expected_items = payload.items.clone();
        let saved_draw_counter = payload.rng.draw_counter;

        let restored = Game::from_save(payload).expect("v56 generated floor should migrate");

        assert_eq!(restored.current_floor_id, "demo.floor.resonance-depth-9");
        assert_eq!(restored.to_save().terrain, expected_terrain);
        assert_eq!(actors_to_save(&restored.entities), expected_entities);
        assert_eq!(items_to_save(&restored.items), expected_items);
        assert_eq!(restored.rng.draw_counter, saved_draw_counter);
        assert!(
            restored
                .entities
                .iter()
                .any(|entity| entity.id.contains(".pit."))
        );
    }

    #[test]
    fn previous_v48_floor_and_dungeon_state_are_not_backfilled() {
        let mut game = Game::new(27);
        descend_one_floor(&mut game);
        let mut payload = game.to_save();
        payload.content_hash =
            "9c8fc3226c20300a308d21a5da69033efb853169214f4c411e6c740800bdf9ad".to_owned();
        payload
            .dungeon_states
            .retain(|state| state.dungeon_id == "demo.dungeon.echo-depths");
        let expected_entities = payload.entities.clone();
        let expected_items = payload.items.clone();
        let saved_draw_counter = payload.rng.draw_counter;

        let restored = Game::from_save(payload).expect("v48 floor should migrate");

        assert_eq!(actors_to_save(&restored.entities), expected_entities);
        assert_eq!(items_to_save(&restored.items), expected_items);
        assert_eq!(restored.rng.draw_counter, saved_draw_counter);
        assert!(!restored.dungeon_states["demo.dungeon.resonance-descent"].guardian_defeated);
    }

    #[test]
    fn previous_v47_generated_floor_is_not_backfilled_with_tables_or_nest() {
        let mut game = Game::new(27);
        descend_one_floor(&mut game);
        let mut payload = game.to_save();
        payload.content_hash =
            "ae7b19dd780d73091a5b34aed2f67dcbc5650d2e2ed1d7748cc86f48020f8fb0".to_owned();
        payload
            .entities
            .retain(|entity| entity.id == "demo.floor.echo-depth-1.encounter.1");
        payload.entities[0].id = "demo.monster.echo-depth-1.1".to_owned();
        let saved_draw_counter = payload.rng.draw_counter;

        let restored = Game::from_save(payload).expect("v47 generated floor should migrate");

        assert_eq!(restored.current_floor_id, "demo.floor.echo-depth-1");
        assert_eq!(restored.entities.len(), 1);
        assert_eq!(restored.entities[0].id, "demo.monster.echo-depth-1.1");
        assert!(
            restored
                .entities
                .iter()
                .all(|entity| !entity.id.contains(".nest.") && !entity.id.contains(".encounter."))
        );
        assert_eq!(restored.rng.draw_counter, saved_draw_counter);
    }

    #[test]
    fn equipping_and_unequipping_moves_an_item_between_authoritative_lists() {
        let mut game = Game::new(42);
        clear_monsters(&mut game);
        collect_both_demo_items(&mut game);
        let carried = game.snapshot();
        let charm = carried
            .inventory
            .iter()
            .find(|item| item.kind_id == "demo.item.echo-charm")
            .expect("collected charm should be in inventory");
        assert_eq!(charm.modifiers.attack, 1);
        assert_eq!(charm.identification, ItemIdentificationDto::Unexamined);
        assert_eq!(charm.quality, None);
        assert!(charm.known_properties.is_empty());
        assert!(game.to_save().item_property_knowledge.is_empty());
        let equipped = game
            .dispatch(command(
                5,
                4,
                GameCommand::Equip {
                    item_id: "demo.item.echo-charm.1".to_owned(),
                },
            ))
            .expect("equipping should execute");

        assert_eq!(equipped.inventory.len(), 1);
        assert_eq!(equipped.equipment.len(), 1);
        assert_eq!(equipped.equipment[0].slot_id, "charm");
        assert_eq!(equipped.equipment[0].modifiers.attack, 2);
        assert_eq!(equipped.equipment[0].modifiers.defense, 1);
        assert_eq!(equipped.equipment[0].modifiers.max_hp, 4);
        assert_eq!(equipped.player.base_max_hp, 10);
        assert_eq!(equipped.player.max_hp, 14);
        assert_eq!(equipped.player.base_attack, 2);
        assert_eq!(equipped.player.attack, 4);
        assert_eq!(equipped.player.base_defense, 1);
        assert_eq!(equipped.player.defense, 2);
        assert_eq!(equipped.player.equipment_modifiers.attack, 2);
        assert_eq!(equipped.player.equipment_modifiers.defense, 1);
        assert_eq!(equipped.player.equipment_modifiers.max_hp, 4);
        assert_eq!(equipped.player.carried_weight_tenths_pound, 55);
        assert_eq!(equipped.events[0].message_key, "item-equip-success");
        assert_eq!(equipped.events[1].message_key, "item-property-discovered");
        assert_eq!(equipped.equipment[0].known_properties.len(), 1);
        assert_eq!(
            equipped.equipment[0].identification,
            ItemIdentificationDto::Identified
        );
        assert_eq!(equipped.equipment[0].quality, Some(ItemQualityDto::Fine));
        assert_eq!(
            equipped.equipment[0].known_properties[0].affix_id,
            "demo.affix.harmonic-edge"
        );
        let saved = game.to_save();
        assert_eq!(saved.item_property_knowledge.len(), 1);
        let restored = Game::from_save(saved.clone()).expect("affix knowledge should round trip");
        assert_eq!(restored.state_hash(), game.state_hash());
        let mut invalid = saved;
        invalid.item_property_knowledge[0].known_affix_ids = vec!["demo.affix.missing".to_owned()];
        assert!(matches!(
            Game::from_save(invalid),
            Err(CoreError::InvalidSave(
                "item property knowledge state is invalid"
            ))
        ));

        game.player.hp = 14;

        let unequipped = game
            .dispatch(command(
                6,
                5,
                GameCommand::Unequip {
                    slot_id: "charm".to_owned(),
                },
            ))
            .expect("unequipping should execute");
        assert_eq!(unequipped.inventory.len(), 2);
        assert!(unequipped.equipment.is_empty());
        assert_eq!(unequipped.player.carried_weight_tenths_pound, 55);
        assert_eq!(unequipped.player.hp, 10);
        assert_eq!(unequipped.player.max_hp, 10);
        assert_eq!(unequipped.player.attack, 2);
        assert_eq!(unequipped.player.defense, 1);
        assert_eq!(unequipped.events[0].message_key, "item-unequip-success");
    }

    #[test]
    fn appraising_reveals_quality_without_revealing_affixes() {
        let mut game = Game::new(42);
        clear_monsters(&mut game);
        collect_both_demo_items(&mut game);

        let before = game.snapshot();
        let charm = before
            .inventory
            .iter()
            .find(|item| item.kind_id == "demo.item.echo-charm")
            .expect("collected charm should be in inventory");
        assert_eq!(charm.identification, ItemIdentificationDto::Unexamined);
        assert_eq!(charm.quality, None);
        assert!(charm.known_properties.is_empty());

        let appraised = game
            .dispatch(command(
                5,
                4,
                GameCommand::Appraise {
                    item_id: "demo.item.echo-charm.1".to_owned(),
                },
            ))
            .expect("appraisal should execute");
        let charm = appraised
            .inventory
            .iter()
            .find(|item| item.kind_id == "demo.item.echo-charm")
            .expect("appraised charm should remain in inventory");
        assert_eq!(charm.identification, ItemIdentificationDto::Appraised);
        assert_eq!(charm.quality, Some(ItemQualityDto::Fine));
        assert_eq!(charm.modifiers.attack, 1);
        assert!(charm.known_properties.is_empty());
        assert_eq!(appraised.player.attack, 2);
        assert_eq!(appraised.events[0].message_key, "item-appraise-success");
        assert_eq!(appraised.events[0].args["quality"], "fine");

        let saved = game.to_save();
        assert!(saved.item_property_knowledge[0].appraised);
        assert!(!saved.item_property_knowledge[0].identified);
        assert!(saved.item_property_knowledge[0].known_affix_ids.is_empty());
        let restored = Game::from_save(saved).expect("appraisal knowledge should round trip");
        assert_eq!(restored.state_hash(), game.state_hash());
    }

    #[test]
    fn player_derived_stats_retain_equipment_and_status_sources() {
        let mut game = Game::new(42);
        clear_monsters(&mut game);
        collect_both_demo_items(&mut game);
        game.dispatch(command(
            5,
            4,
            GameCommand::Equip {
                item_id: "demo.item.echo-charm.1".to_owned(),
            },
        ))
        .expect("equipping should execute");
        game.player.statuses.push(StatusInstance {
            kind_id: STATUS_HASTE.to_owned(),
            intensity: 2,
            remaining_ticks: 3,
            source_id: Some("demo.item.temporary-tonic.1".to_owned()),
        });
        game.player.statuses.push(StatusInstance {
            kind_id: STATUS_STUN.to_owned(),
            intensity: 2,
            remaining_ticks: 3,
            source_id: Some("demo.monster.impact.1".to_owned()),
        });
        game.player
            .statuses
            .sort_by(|left, right| left.kind_id.cmp(&right.kind_id));

        let stats = game.player_derived_stats();

        assert_eq!(stats.attack.value, 4);
        assert_eq!(stats.speed.value, 130);
        assert_eq!(stats.melee_skill.value, 60);
        assert!(stats.attack.contributions.iter().any(|contribution| {
            contribution.layer == StatLayer::Equipment
                && contribution.source_id == "demo.item.echo-charm.1"
                && contribution.amount == 2
        }));
        assert!(stats.speed.contributions.iter().any(|contribution| {
            contribution.layer == StatLayer::Status
                && contribution.source_id == STATUS_HASTE
                && contribution.origin_id.as_deref() == Some("demo.item.temporary-tonic.1")
                && contribution.amount == 20
        }));
        assert!(stats.melee_skill.contributions.iter().any(|contribution| {
            contribution.layer == StatLayer::Status
                && contribution.source_id == STATUS_STUN
                && contribution.origin_id.as_deref() == Some("demo.monster.impact.1")
                && contribution.amount == -20
        }));
    }

    #[test]
    fn fear_check_can_consume_a_melee_action_without_attacking() {
        let mut game = Game::new(0);
        game.rng = RfbRng::seeded(0);
        game.entities[0].position = Position { x: 4, y: 3 };
        game.entities[0].statuses.push(StatusInstance {
            kind_id: STATUS_SLOW.to_owned(),
            intensity: 10,
            remaining_ticks: 20,
            source_id: None,
        });
        game.player.statuses.push(StatusInstance {
            kind_id: STATUS_FEAR.to_owned(),
            intensity: 2,
            remaining_ticks: 20,
            source_id: Some("demo.monster.ember-mote.1".to_owned()),
        });

        let update = game
            .dispatch(command(
                1,
                0,
                GameCommand::Move {
                    direction: Direction::East,
                },
            ))
            .expect("fear-blocked action should still execute");

        assert_eq!(update.player.position, Position { x: 3, y: 3 });
        assert_eq!(update.entities[0].hp, 3);
        assert_eq!(update.turn, 1);
        assert_eq!(update.player.statuses[0].kind_id, STATUS_FEAR);
        assert_eq!(update.player.statuses[0].remaining_ticks, 10);
        assert_eq!(game.rng.draw_counter, 2);
        assert_eq!(update.events.len(), 1);
        assert_eq!(update.events[0].message_key, "status-fear-blocked");
    }

    #[test]
    fn item_instance_identity_survives_location_transitions() {
        let mut game = Game::new(42);
        clear_monsters(&mut game);
        let original_instance_count = game.items.len();
        collect_both_demo_items(&mut game);

        let charm_id = "demo.item.echo-charm.1";
        assert_eq!(game.items.len(), original_instance_count);
        assert!(game.items.iter().any(|item| {
            item.id == charm_id && item.location == ItemLocation::Inventory && item.quantity == 1
        }));

        game.dispatch(command(
            5,
            4,
            GameCommand::Equip {
                item_id: charm_id.to_owned(),
            },
        ))
        .expect("equip should execute");
        assert!(game.items.iter().any(|item| {
            item.id == charm_id
                && item.location
                    == ItemLocation::Equipped {
                        slot_id: "charm".to_owned(),
                    }
        }));

        game.dispatch(command(
            6,
            5,
            GameCommand::Unequip {
                slot_id: "charm".to_owned(),
            },
        ))
        .expect("unequip should execute");
        game.dispatch(command(
            7,
            6,
            GameCommand::Drop {
                item_ids: vec![charm_id.to_owned()],
            },
        ))
        .expect("drop should execute");

        assert_eq!(game.items.len(), original_instance_count);
        assert!(game.items.iter().any(|item| {
            item.id == charm_id
                && item.location == ItemLocation::Ground(game.player.position)
                && item.quantity == 1
        }));
    }

    #[test]
    fn equipped_attack_modifier_changes_authoritative_melee_skill() {
        let mut game = Game::new(42);
        collect_both_demo_items(&mut game);
        game.dispatch(command(
            5,
            4,
            GameCommand::Equip {
                item_id: "demo.item.echo-charm.1".to_owned(),
            },
        ))
        .expect("equip should execute");
        game.entities[0].position = Position {
            x: game.player.position.x + 1,
            y: game.player.position.y,
        };
        game.entities[0].energy_need = STANDARD_ACTION_COST;
        game.rng = RfbRng::seeded(42);
        let update = game
            .dispatch(command(
                6,
                5,
                GameCommand::Move {
                    direction: Direction::East,
                },
            ))
            .expect("equipped attack should execute");

        assert_eq!(update.events[0].message_key, "combat-player-hit");
        assert_eq!(update.player.melee_skill, 80);
        assert_eq!(update.events[0].args["damage"], "2");
        assert_eq!(update.entities[0].hp, 1);
    }

    #[test]
    fn equipped_weapon_profile_drives_two_stable_player_attacks() {
        let mut game = Game::new(42);
        game.rng = RfbRng::seeded(42);
        let weapon = game
            .items
            .iter_mut()
            .find(|item| item.kind_id == "demo.item.echo-blade")
            .expect("demo weapon should exist");
        weapon.location = ItemLocation::Equipped {
            slot_id: "weapon".to_owned(),
        };
        let snapshot = game.snapshot();
        let profile = snapshot.player.melee_profile;

        assert_eq!(profile.attacks, 2);
        assert_eq!(profile.to_hit, 10);
        assert_eq!(profile.to_damage, 1);
        assert_eq!(profile.damage.dice, 1);
        assert_eq!(profile.damage.sides, 2);
        assert_eq!(
            profile.source_item_id.as_deref(),
            Some("demo.item.echo-blade.1")
        );
        assert_eq!(snapshot.equipment[0].melee_profile, Some(profile));

        let mut events = Vec::new();
        let mut changed = BTreeSet::new();
        let mut removed = Vec::new();
        game.resolve_player_melee(0, &mut events, &mut changed, &mut removed)
            .expect("melee resolution should succeed");

        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(
                    event,
                    DomainEvent::PlayerMeleeHit { .. } | DomainEvent::PlayerMeleeMissed { .. }
                ))
                .count(),
            2
        );
        assert!(removed.is_empty());
    }

    #[test]
    fn equipped_launcher_traces_to_first_target_and_resolves_damage() {
        let mut game = Game::new(0);
        game.rng = RfbRng::seeded(0);
        game.items
            .iter_mut()
            .find(|item| item.kind_id == "demo.item.resonance-sling")
            .expect("demo launcher should exist")
            .location = ItemLocation::Equipped {
            slot_id: "launcher".to_owned(),
        };
        game.items
            .iter_mut()
            .find(|item| item.kind_id == "demo.item.resonance-pellet")
            .expect("demo ammunition should exist")
            .location = ItemLocation::Inventory;
        game.entities[0].position = Position { x: 7, y: 3 };
        game.entities[0].hp = 10;

        let update = game
            .dispatch(command(
                1,
                0,
                GameCommand::Fire {
                    direction: Direction::East,
                },
            ))
            .expect("projectile action should execute");

        let projectile = update
            .events
            .iter()
            .find(|event| event.kind.starts_with("combat.projectile-"))
            .expect("projectile event should be emitted");
        let trace = projectile
            .trace
            .as_ref()
            .expect("projectile trace should exist");
        assert_eq!(trace.origin, Position { x: 3, y: 3 });
        assert_eq!(trace.impact, Position { x: 7, y: 3 });
        assert_eq!(trace.landing, Position { x: 7, y: 3 });
        assert_eq!(trace.traversed.len(), 4);
        assert_eq!(projectile.kind, "combat.projectile-hit");
        assert!(update.entities[0].hp < 10);
        assert!(
            update
                .events
                .iter()
                .any(|event| event.kind == "combat.projectile-ammo-recovered")
        );
        assert_eq!(
            update
                .inventory
                .iter()
                .find(|item| item.kind_id == "demo.item.resonance-pellet")
                .map(|item| item.quantity),
            Some(5)
        );
        assert!(update.items.iter().any(|item| {
            item.id == "generated.item.2"
                && item.kind_id == "demo.item.resonance-pellet"
                && item.quantity == 1
                && item.position == Position { x: 7, y: 3 }
        }));
    }

    #[test]
    fn ammunition_breakage_is_checked_after_hitting_a_body() {
        let mut game = Game::new(16);
        game.rng = RfbRng::seeded(16);
        game.items
            .iter_mut()
            .find(|item| item.kind_id == "demo.item.resonance-sling")
            .expect("demo launcher should exist")
            .location = ItemLocation::Equipped {
            slot_id: "launcher".to_owned(),
        };
        game.items
            .iter_mut()
            .find(|item| item.kind_id == "demo.item.resonance-pellet")
            .expect("demo ammunition should exist")
            .location = ItemLocation::Inventory;
        game.entities[0].position = Position { x: 7, y: 3 };
        game.entities[0].hp = 10;

        let update = game
            .dispatch(command(
                1,
                0,
                GameCommand::Fire {
                    direction: Direction::East,
                },
            ))
            .expect("projectile action should execute");

        assert!(
            update
                .events
                .iter()
                .any(|event| event.kind == "combat.projectile-ammo-broken")
        );
        assert_eq!(update.inventory[0].quantity, 5);
        assert!(!update.items.iter().any(|item| {
            item.kind_id == "demo.item.resonance-pellet" && item.position == Position { x: 7, y: 3 }
        }));
        assert_eq!(game.next_item_instance_serial, 3);
    }

    #[test]
    fn ammunition_that_hits_no_body_lands_without_a_breakage_roll() {
        let mut game = Game::new(0);
        game.items
            .iter_mut()
            .find(|item| item.kind_id == "demo.item.resonance-sling")
            .expect("demo launcher should exist")
            .location = ItemLocation::Equipped {
            slot_id: "launcher".to_owned(),
        };
        game.items
            .iter_mut()
            .find(|item| item.kind_id == "demo.item.resonance-pellet")
            .expect("demo ammunition should exist")
            .location = ItemLocation::Inventory;
        let rng_draws = game.rng_draw_counter();

        let update = game
            .dispatch(command(
                1,
                0,
                GameCommand::Fire {
                    direction: Direction::North,
                },
            ))
            .expect("projectile action should execute");

        assert_eq!(game.rng_draw_counter(), rng_draws);
        assert_eq!(update.events[0].kind, "combat.projectile-landed");
        assert_eq!(update.events[1].kind, "combat.projectile-ammo-recovered");
        assert!(update.items.iter().any(|item| {
            item.kind_id == "demo.item.resonance-pellet" && item.position == Position { x: 3, y: 1 }
        }));
    }

    #[test]
    fn launcher_without_inventory_ammunition_does_not_advance_rng() {
        let mut game = Game::new(0);
        game.items
            .iter_mut()
            .find(|item| item.kind_id == "demo.item.resonance-sling")
            .expect("demo launcher should exist")
            .location = ItemLocation::Equipped {
            slot_id: "launcher".to_owned(),
        };
        let rng_draws = game.rng_draw_counter();

        let update = game
            .dispatch(command(
                1,
                0,
                GameCommand::Fire {
                    direction: Direction::East,
                },
            ))
            .expect("unavailable fire action should execute deterministically");

        assert_eq!(update.events[0].kind, "combat.projectile-ammo-unavailable");
        assert_eq!(game.rng_draw_counter(), rng_draws);
        assert!(update.inventory.is_empty());
        assert!(
            update
                .items
                .iter()
                .any(|item| { item.kind_id == "demo.item.resonance-pellet" && item.quantity == 6 })
        );
    }

    #[test]
    fn entity_targeting_uses_a_stable_off_axis_line() {
        let mut game = Game::new(0);
        game.items
            .iter_mut()
            .find(|item| item.kind_id == "demo.item.resonance-sling")
            .expect("demo launcher should exist")
            .location = ItemLocation::Equipped {
            slot_id: "launcher".to_owned(),
        };
        game.items
            .iter_mut()
            .find(|item| item.kind_id == "demo.item.resonance-pellet")
            .expect("demo ammunition should exist")
            .location = ItemLocation::Inventory;
        game.entities[0].position = Position { x: 9, y: 5 };
        game.entities[0].hp = 10;
        let expected_path = vec![
            Position { x: 4, y: 3 },
            Position { x: 5, y: 4 },
            Position { x: 6, y: 4 },
            Position { x: 7, y: 4 },
            Position { x: 8, y: 5 },
            Position { x: 9, y: 5 },
        ];
        assert_eq!(
            game.projectile_path(
                &TargetSelection::Position {
                    position: Position { x: 9, y: 5 },
                },
                6,
            ),
            Some(expected_path.clone())
        );

        let update = game
            .dispatch(command(
                1,
                0,
                GameCommand::FireTarget {
                    target: TargetSelection::Entity {
                        entity_id: "demo.monster.ember-mote.1".to_owned(),
                    },
                },
            ))
            .expect("targeted projectile action should execute");

        let projectile = update
            .events
            .iter()
            .find(|event| event.kind == "combat.projectile-hit")
            .expect("targeted projectile should hit");
        let trace = projectile.trace.as_ref().expect("trace should exist");
        assert_eq!(trace.impact, Position { x: 9, y: 5 });
        assert_eq!(trace.traversed, expected_path);
        let target_spec = update
            .player
            .projectile_profile
            .as_ref()
            .expect("equipped launcher profile should exist")
            .target_spec
            .clone();
        assert_eq!(target_spec.range, 6);
        assert_eq!(
            target_spec.modes,
            [
                TargetModeDto::Direction,
                TargetModeDto::Position,
                TargetModeDto::Entity,
            ]
        );
    }

    #[test]
    fn invalid_entity_target_preserves_ammunition_and_rng() {
        let mut game = Game::new(0);
        game.items
            .iter_mut()
            .find(|item| item.kind_id == "demo.item.resonance-sling")
            .expect("demo launcher should exist")
            .location = ItemLocation::Equipped {
            slot_id: "launcher".to_owned(),
        };
        game.items
            .iter_mut()
            .find(|item| item.kind_id == "demo.item.resonance-pellet")
            .expect("demo ammunition should exist")
            .location = ItemLocation::Inventory;
        let rng_draws = game.rng_draw_counter();

        let update = game
            .dispatch(command(
                1,
                0,
                GameCommand::FireTarget {
                    target: TargetSelection::Entity {
                        entity_id: "demo.monster.missing.1".to_owned(),
                    },
                },
            ))
            .expect("invalid target should produce a deterministic event");

        assert_eq!(
            update.events[0].kind,
            "combat.projectile-target-unavailable"
        );
        assert_eq!(game.rng_draw_counter(), rng_draws);
        assert_eq!(update.inventory[0].quantity, 6);
    }

    #[test]
    fn throwing_one_item_splits_the_stack_and_lands_before_a_wall() {
        let mut game = Game::new(0);
        game.rng = RfbRng::seeded(0);
        game.player.position = Position { x: 10, y: 3 };
        game.items
            .iter_mut()
            .find(|item| item.kind_id == "demo.item.luminous-shard")
            .expect("demo throwable stack should exist")
            .location = ItemLocation::Inventory;

        let update = game
            .dispatch(command(
                1,
                0,
                GameCommand::Throw {
                    item_id: "demo.item.luminous-shard.1".to_owned(),
                    direction: Direction::East,
                },
            ))
            .expect("throw action should execute");

        let thrown = update
            .events
            .iter()
            .find(|event| event.kind == "item.thrown")
            .expect("throw event should be emitted");
        let trace = thrown.trace.as_ref().expect("throw trace should exist");
        assert_eq!(trace.origin, Position { x: 10, y: 3 });
        assert_eq!(trace.impact, Position { x: 11, y: 3 });
        assert_eq!(trace.landing, Position { x: 10, y: 3 });
        assert!(trace.traversed.is_empty());
        assert_eq!(update.inventory[0].quantity, 4);
        assert!(update.items.iter().any(|item| {
            item.id == "generated.item.2"
                && item.kind_id == "demo.item.luminous-shard"
                && item.quantity == 1
                && item.position == Position { x: 10, y: 3 }
        }));
    }

    #[test]
    fn throwable_profile_uses_weight_range_and_resolves_damage() {
        let mut game = Game::new(0);
        game.rng = RfbRng::seeded(0);
        game.item_knowledge.insert(
            "demo.item.luminous-shard".to_owned(),
            ItemKnowledgeState {
                tried: true,
                aware: true,
            },
        );
        game.items
            .iter_mut()
            .find(|item| item.kind_id == "demo.item.luminous-shard")
            .expect("demo throwable stack should exist")
            .location = ItemLocation::Inventory;
        game.entities[0].position = Position { x: 6, y: 3 };
        game.entities[0].hp = 10;
        let inventory = game.snapshot().inventory;
        let shard = inventory
            .iter()
            .find(|item| item.kind_id == "demo.item.luminous-shard")
            .expect("throwable should be projected into inventory");
        assert_eq!(shard.weight_tenths_pound, 10);
        assert_eq!(
            shard
                .throw_profile
                .as_ref()
                .expect("shard should expose its throw profile")
                .range,
            5
        );

        let update = game
            .dispatch(command(
                1,
                0,
                GameCommand::Throw {
                    item_id: "demo.item.luminous-shard.1".to_owned(),
                    direction: Direction::East,
                },
            ))
            .expect("throw attack should execute");

        let hit = update
            .events
            .iter()
            .find(|event| event.kind == "combat.throw-hit")
            .expect("throw hit should be emitted");
        assert_eq!(hit.args["source"], "demo.item.luminous-shard");
        assert_eq!(hit.args["target"], "demo.actor.ember-mote");
        assert_eq!(hit.args["damage"], "1");
        assert_eq!(update.entities[0].hp, 9);
        assert_eq!(update.inventory[0].quantity, 4);
        assert!(update.items.iter().any(|item| {
            item.id == "generated.item.2"
                && item.kind_id == "demo.item.luminous-shard"
                && item.position == Position { x: 6, y: 3 }
        }));
    }

    #[test]
    fn throwing_an_unknown_item_marks_the_kind_tried_and_preserves_its_appearance() {
        let mut game = Game::new(42);
        clear_monsters(&mut game);
        game.items
            .iter_mut()
            .find(|item| item.kind_id == "demo.item.luminous-shard")
            .expect("demo unknown stack should exist")
            .location = ItemLocation::Inventory;
        let before = game.snapshot();
        let shard = before
            .inventory
            .iter()
            .find(|item| item.kind_id == "demo.item.luminous-shard")
            .expect("unknown shard should be projected");
        assert_eq!(shard.knowledge, ItemKnowledgeDto::Unknown);
        assert_eq!(shard.display_name_key, "item-demo-unfamiliar-shard-name");
        assert!(shard.throw_profile.is_none());

        let update = game
            .dispatch(command(
                1,
                0,
                GameCommand::Throw {
                    item_id: "demo.item.luminous-shard.1".to_owned(),
                    direction: Direction::North,
                },
            ))
            .expect("throwing an unknown item should execute");

        let remaining = update
            .inventory
            .iter()
            .find(|item| item.kind_id == "demo.item.luminous-shard")
            .expect("remaining stack should stay carried");
        assert_eq!(remaining.knowledge, ItemKnowledgeDto::Tried);
        assert_eq!(
            remaining.display_name_key,
            "item-demo-unfamiliar-shard-name"
        );
        assert!(remaining.throw_profile.is_none());
        assert_eq!(game.to_save().item_knowledge.len(), 1);
        let restored = Game::from_save(game.to_save()).expect("tried knowledge should reload");
        assert_eq!(restored.snapshot(), game.snapshot());
    }

    #[test]
    fn aware_item_knowledge_reveals_the_true_name_and_profile_after_reload() {
        let mut game = Game::new(7);
        game.items
            .iter_mut()
            .find(|item| item.kind_id == "demo.item.luminous-shard")
            .expect("demo unknown stack should exist")
            .location = ItemLocation::Inventory;
        let mut payload = game.to_save();
        payload.item_knowledge = vec![ItemKnowledgeSaveDto {
            kind_id: "demo.item.luminous-shard".to_owned(),
            tried: true,
            aware: true,
        }];

        let restored = Game::from_save(payload).expect("aware knowledge should load");
        let shard = restored
            .snapshot()
            .inventory
            .into_iter()
            .find(|item| item.kind_id == "demo.item.luminous-shard")
            .expect("aware shard should be projected");
        assert_eq!(shard.knowledge, ItemKnowledgeDto::Aware);
        assert_eq!(shard.display_name_key, "item-demo-luminous-shard-name");
        assert!(shard.throw_profile.is_some());

        let mut invalid = restored.to_save();
        invalid.item_knowledge[0].tried = false;
        assert!(matches!(
            Game::from_save(invalid),
            Err(CoreError::InvalidSave("item knowledge state is invalid"))
        ));
    }

    #[test]
    fn observable_item_use_consumes_one_heals_and_marks_the_kind_aware() {
        let mut game = Game::new(42);
        clear_monsters(&mut game);
        game.player.hp = 3;
        game.items
            .iter_mut()
            .find(|item| item.kind_id == "demo.item.luminous-shard")
            .expect("demo usable stack should exist")
            .location = ItemLocation::Inventory;

        let update = game
            .dispatch(command(
                1,
                0,
                GameCommand::UseItem {
                    item_id: "demo.item.luminous-shard.1".to_owned(),
                },
            ))
            .expect("using a healing item should execute");

        assert_eq!(update.player.hp, 7);
        let shard = update
            .inventory
            .iter()
            .find(|item| item.kind_id == "demo.item.luminous-shard")
            .expect("the remaining stack should stay carried");
        assert_eq!(shard.quantity, 4);
        assert!(shard.usable);
        assert_eq!(shard.knowledge, ItemKnowledgeDto::Aware);
        assert_eq!(shard.display_name_key, "item-demo-luminous-shard-name");
        assert!(shard.throw_profile.is_some());
        assert_eq!(update.events[0].kind, "item.use-heal");
        assert_eq!(
            update.events[0].args["nameKey"],
            "item-demo-luminous-shard-name"
        );
        assert!(matches!(
            update.events[0].outcome,
            Some(GameEventOutcomeDto::Heal { resolution })
                if resolution.requested == 4 && resolution.applied == 4
        ));
        let restored = Game::from_save(game.to_save()).expect("aware use result should reload");
        assert_eq!(restored.snapshot(), game.snapshot());
    }

    #[test]
    fn unobservable_item_use_consumes_one_but_only_marks_the_kind_tried() {
        let mut game = Game::new(42);
        clear_monsters(&mut game);
        game.items
            .iter_mut()
            .find(|item| item.kind_id == "demo.item.luminous-shard")
            .expect("demo usable stack should exist")
            .location = ItemLocation::Inventory;

        let update = game
            .dispatch(command(
                1,
                0,
                GameCommand::UseItem {
                    item_id: "demo.item.luminous-shard.1".to_owned(),
                },
            ))
            .expect("using an item at full health should execute");

        assert_eq!(update.player.hp, 10);
        let shard = update
            .inventory
            .iter()
            .find(|item| item.kind_id == "demo.item.luminous-shard")
            .expect("the remaining stack should stay carried");
        assert_eq!(shard.quantity, 4);
        assert_eq!(shard.knowledge, ItemKnowledgeDto::Tried);
        assert_eq!(shard.display_name_key, "item-demo-unfamiliar-shard-name");
        assert!(shard.throw_profile.is_none());
        assert_eq!(update.events[0].kind, "item.use-no-effect");
        assert_eq!(
            update.events[0].args["nameKey"],
            "item-demo-unfamiliar-shard-name"
        );
        assert!(matches!(
            update.events[0].outcome,
            Some(GameEventOutcomeDto::Heal { resolution })
                if resolution.requested == 4 && resolution.applied == 0
        ));
    }

    #[test]
    fn unusable_inventory_item_is_not_consumed_or_added_to_knowledge() {
        let mut game = Game::new(42);
        clear_monsters(&mut game);
        game.items
            .iter_mut()
            .find(|item| item.kind_id == "demo.item.echo-charm")
            .expect("demo non-consumable should exist")
            .location = ItemLocation::Inventory;

        let update = game
            .dispatch(command(
                1,
                0,
                GameCommand::UseItem {
                    item_id: "demo.item.echo-charm.1".to_owned(),
                },
            ))
            .expect("an unavailable use attempt should remain a valid action");

        assert_eq!(update.events[0].kind, "item.use-unavailable");
        assert!(
            update
                .inventory
                .iter()
                .any(|item| item.id == "demo.item.echo-charm.1" && item.quantity == 1)
        );
        assert!(game.to_save().item_knowledge.is_empty());
    }

    #[test]
    fn missed_throw_still_lands_at_the_collided_target() {
        let mut game = Game::new(3);
        game.rng = RfbRng::seeded(3);
        game.items
            .iter_mut()
            .find(|item| item.kind_id == "demo.item.luminous-shard")
            .expect("demo throwable stack should exist")
            .location = ItemLocation::Inventory;
        game.entities[0].position = Position { x: 6, y: 3 };
        game.entities[0].hp = 10;

        let update = game
            .dispatch(command(
                1,
                0,
                GameCommand::Throw {
                    item_id: "demo.item.luminous-shard.1".to_owned(),
                    direction: Direction::East,
                },
            ))
            .expect("missed throw should execute");

        assert_eq!(update.events[0].kind, "combat.throw-miss");
        assert_eq!(update.entities[0].hp, 10);
        assert!(update.items.iter().any(|item| {
            item.kind_id == "demo.item.luminous-shard" && item.position == Position { x: 6, y: 3 }
        }));
    }

    #[test]
    fn dropping_multiple_selected_stacks_is_atomic_and_deterministic() {
        let mut game = Game::new(42);
        collect_both_demo_items(&mut game);
        let update = game
            .dispatch(command(
                5,
                4,
                GameCommand::Drop {
                    item_ids: vec![
                        "demo.item.luminous-shard.1".to_owned(),
                        "demo.item.echo-charm.1".to_owned(),
                    ],
                },
            ))
            .expect("batch drop should execute");

        assert!(update.inventory.is_empty());
        assert_eq!(update.items.len(), 5);
        assert!(
            update
                .items
                .iter()
                .filter(|item| {
                    item.kind_id != "demo.item.echo-blade"
                        && item.kind_id != "demo.item.resonance-sling"
                        && item.kind_id != "demo.item.resonance-pellet"
                })
                .all(|item| item.position == Position { x: 5, y: 3 })
        );
        assert_eq!(update.changed_cells.len(), 1);
        assert_eq!(update.events[0].message_key, "item-drop-success");
        assert_eq!(update.events[0].args["stacks"], "2");
        assert_eq!(update.events[0].args["quantity"], "6");
    }

    #[test]
    fn pickup_on_empty_ground_is_a_deterministic_turn() {
        let mut game = Game::new(42);
        clear_monsters(&mut game);
        let before = game.state_hash();
        let update = game
            .dispatch(command(1, 0, GameCommand::PickUp))
            .expect("empty pickup should still execute");

        assert_eq!(update.turn, 1);
        assert!(update.changed_cells.is_empty());
        assert!(update.inventory.is_empty());
        assert_eq!(update.events[0].message_key, "item-pickup-none");
        assert_ne!(update.state_hash, before);
    }

    #[test]
    fn pickup_merges_into_the_lowest_id_compatible_stack() {
        let mut game = Game::new(42);
        clear_monsters(&mut game);
        game.items.push(ItemInstance {
            id: "demo.inventory.resonance-pellet.1".to_owned(),
            kind_id: "demo.item.resonance-pellet".to_owned(),
            quantity: 19,
            quality: ItemQualityDto::Ordinary,
            affix_ids: Vec::new(),
            location: ItemLocation::Inventory,
        });
        game.player.position = Position { x: 6, y: 4 };
        let update = game
            .dispatch(command(1, 0, GameCommand::PickUp))
            .expect("pickup should execute");

        assert_eq!(update.inventory.len(), 2);
        assert_eq!(update.inventory[0].id, "demo.inventory.resonance-pellet.1");
        assert_eq!(update.inventory[0].quantity, 20);
        assert_eq!(update.inventory[1].id, "demo.item.resonance-pellet.1");
        assert_eq!(update.inventory[1].quantity, 5);
    }

    #[test]
    fn partial_drop_allocates_stable_ids_and_survives_save_round_trip() {
        let mut game = Game::new(42);
        game.dispatch(command(
            1,
            0,
            GameCommand::Move {
                direction: Direction::East,
            },
        ))
        .expect("move should execute");
        game.dispatch(command(2, 1, GameCommand::PickUp))
            .expect("pickup should execute");
        let first_drop = game
            .dispatch(command(
                3,
                2,
                GameCommand::DropQuantity {
                    item_id: "demo.item.luminous-shard.1".to_owned(),
                    quantity: 2,
                },
            ))
            .expect("partial drop should execute");

        assert_eq!(first_drop.inventory[0].quantity, 3);
        assert!(first_drop.items.iter().any(|item| {
            item.id == "generated.item.2"
                && item.quantity == 2
                && item.position == Position { x: 4, y: 3 }
        }));
        assert_eq!(game.next_item_instance_serial, 3);

        let mut restored = Game::from_save(game.to_save()).expect("save should preserve allocator");
        let second_drop = restored
            .dispatch(command(
                4,
                3,
                GameCommand::DropQuantity {
                    item_id: "demo.item.luminous-shard.1".to_owned(),
                    quantity: 1,
                },
            ))
            .expect("second partial drop should execute");
        assert!(
            second_drop
                .items
                .iter()
                .any(|item| item.id == "generated.item.3" && item.quantity == 1)
        );
        assert_eq!(restored.next_item_instance_serial, 4);
    }

    #[test]
    fn stale_revision_is_rejected_without_mutation() {
        let mut game = Game::new(1);
        let before = game.state_hash();
        let error = game
            .dispatch(command(1, 99, GameCommand::Wait))
            .expect_err("stale command should fail");
        assert!(matches!(error, CoreError::RevisionMismatch { .. }));
        assert_eq!(game.state_hash(), before);
    }

    #[test]
    fn rfb_style_armor_reduction_uses_the_legacy_linear_cap() {
        assert_eq!(apply_melee_armor_reduction(100, 0), 100);
        assert_eq!(apply_melee_armor_reduction(100, 90), 70);
        assert_eq!(apply_melee_armor_reduction(100, 180), 40);
        assert_eq!(apply_melee_armor_reduction(100, 999), 40);
    }

    #[test]
    fn fixed_seed_exercises_player_miss_and_death_rejection() {
        let mut miss_game = Game::new(0);
        miss_game.rng = RfbRng::seeded(0);
        miss_game.entities[0].position = Position { x: 4, y: 4 };
        miss_game.entities[0].energy_need = STANDARD_ACTION_COST;
        let miss_update = miss_game
            .dispatch(command(
                1,
                0,
                GameCommand::Move {
                    direction: Direction::SouthEast,
                },
            ))
            .expect("fixed-seed player attack should execute");
        assert!(
            miss_update
                .events
                .iter()
                .any(|event| event.message_key == "combat-player-miss")
        );

        let mut game = Game::new(0);
        game.rng = RfbRng::seeded(0);
        game.entities[0].position = Position { x: 4, y: 4 };
        game.entities[0].energy_need = STANDARD_ACTION_COST;
        game.player.hp = 0;
        let update = game
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("adjacent monster turn should execute");
        assert!(update.player.is_dead);
        assert!(
            update
                .events
                .iter()
                .any(|event| event.message_key == "combat-player-death")
        );
        assert!(matches!(
            game.dispatch(command(2, 1, GameCommand::Wait)),
            Err(CoreError::PlayerDead)
        ));

        let mut full_health_game = Game::new(0);
        full_health_game.entities[0].position = Position { x: 4, y: 4 };
        full_health_game.entities[0].energy_need = STANDARD_ACTION_COST;
        let death_command = (1..100_u32).find(|seq| {
            full_health_game
                .dispatch(command(*seq, *seq - 1, GameCommand::Wait))
                .is_ok_and(|update| update.player.is_dead)
        });
        assert!(death_command.is_some());
    }

    fn collect_both_demo_items(game: &mut Game) {
        game.dispatch(command(
            1,
            0,
            GameCommand::Move {
                direction: Direction::East,
            },
        ))
        .expect("movement to shard should execute");
        game.dispatch(command(2, 1, GameCommand::PickUp))
            .expect("shard pickup should execute");
        game.dispatch(command(
            3,
            2,
            GameCommand::Move {
                direction: Direction::East,
            },
        ))
        .expect("movement to charm should execute");
        game.dispatch(command(4, 3, GameCommand::PickUp))
            .expect("charm pickup should execute");
    }

    fn clear_monsters(game: &mut Game) {
        game.entities.clear();
        game.items
            .retain(|item| !matches!(item.location, ItemLocation::CarriedBy { .. }));
    }
}
