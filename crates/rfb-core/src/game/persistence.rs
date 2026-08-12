// SPDX-License-Identifier: MPL-2.0

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use crate::{
    error::CoreError,
    rng::RfbRng,
    save::{
        actor_from_entity, actor_from_player, actors_to_save, carried_item_from_dto,
        carried_items_to_save, derive_next_item_instance_serial, equipment_item_from_dto,
        equipment_to_save, floor_connections_from_save, floor_connections_to_save, floor_from_save,
        floor_regions_from_save, floor_regions_to_save, floor_to_save, gold_piles_from_save,
        gold_piles_to_save, inventory_item_from_dto, inventory_to_save, item_from_dto,
        items_to_save, player_to_save, revealed_terrain_from_save,
    },
    state::{Actor, FloorState, ItemInstance, ItemLocation},
    stats::{AttributeKind, AttributeSet, CharacterProgress, SkillProgress},
};
use rfb_content::{ContentCatalog, FloorLifecycle, TaskObjectiveKind, WildernessTerrain};
use rfb_protocol::{
    AbilityProgressSaveDto, ActorSaveDto, BodySlotSaveDto, CampaignStateSaveDto, CampaignStatusDto,
    CarriedItemSaveDto, DungeonStateSaveDto, EquipmentItemSaveDto, FloorConnectionSaveDto,
    FloorRegionSaveDto, HomeStateSaveDto, InventoryItemSaveDto, ItemKnowledgeSaveDto,
    ItemPropertyKnowledgeSaveDto, ItemSaveDto, MapScaleDto, PlayerProgressSaveDto, PlayerSaveDto,
    Position, ResourcePoolSaveDto, RngSaveDto, SAVE_PAYLOAD_SCHEMA_VERSION, SavePayloadV1,
    ShopStateSaveDto, TaskStateSaveDto, TaskStatusKindDto, TerrainSaveDto, TownStateSaveDto,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::gold::derive_next_gold_pile_serial;
use super::town::{
    home_state_to_save, restore_home_states, restore_town_and_shop_states, shop_state_to_save,
    world_town_for_floor,
};
use super::wilderness;
use super::wilderness::WILDERNESS_FLOOR_ID;
use super::{
    BodySlot, CampaignState, DungeonState, Game, ItemKnowledgeState, ItemPropertyKnowledgeState,
    STATE_HASH_SCHEMA_VERSION, TaskState, character_skill_progress, dungeon_instance_id,
    dungeon_instance_storage_key, floor_dungeon_id, initial_dungeon_states, initial_task_states,
    load_built_in_content, normalize_player_name, parse_dungeon_instance_ordinal,
    resolve_body_slots, resolve_character_build, task_definition, task_floors, task_objectives,
    tasks::task_initial_state,
};

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

fn restore_dungeon_states(
    world: &rfb_content::WorldDefinition,
    saved_states: &[DungeonStateSaveDto],
    allow_missing_states: bool,
) -> Result<BTreeMap<String, DungeonState>, CoreError> {
    let mut states = initial_dungeon_states(world);
    if allow_missing_states {
        for dungeon in &world.dungeons {
            if dungeon.entrance_guardian.is_some() {
                states
                    .get_mut(&dungeon.id)
                    .expect("defined dungeon state must remain available")
                    .entrance_guardian_defeated = true;
            }
        }
    }
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
                        entrance_guardian_defeated: if allow_missing_states {
                            saved
                                .entrance_guardian_defeated
                                .unwrap_or(states[&saved.dungeon_id].entrance_guardian_defeated)
                        } else {
                            saved
                                .entrance_guardian_defeated
                                .ok_or(CoreError::InvalidSave(
                                    "dungeon entrance guardian state is missing",
                                ))?
                        },
                        next_instance_ordinal: saved.next_instance_ordinal,
                        retained_instance_id: saved.retained_instance_id.clone(),
                        retained_at_turn: saved.retained_at_turn,
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

fn restore_campaign_state(
    saved: Option<&CampaignStateSaveDto>,
) -> Result<CampaignState, CoreError> {
    let Some(saved) = saved else {
        return Ok(CampaignState::default());
    };
    let valid = match saved.status {
        CampaignStatusDto::Active => {
            saved.victory_turn.is_none()
                && saved.retired_turn.is_none()
                && saved.final_score.is_none()
        }
        CampaignStatusDto::Victorious => {
            saved.victory_turn.is_some()
                && saved.retired_turn.is_none()
                && saved.final_score.is_none()
        }
        CampaignStatusDto::Retired => {
            saved
                .victory_turn
                .zip(saved.retired_turn)
                .is_some_and(|(victory, retired)| victory <= retired)
                && saved.final_score.is_some()
        }
    };
    if !valid {
        return Err(CoreError::InvalidSave("campaign state is invalid"));
    }
    Ok(CampaignState {
        status: saved.status,
        victory_turn: saved.victory_turn,
        retired_turn: saved.retired_turn,
        final_score: saved.final_score,
    })
}

fn restore_task_states(
    world: &rfb_content::WorldDefinition,
    context: TaskRestoreContext<'_>,
) -> Result<BTreeMap<String, TaskState>, CoreError> {
    let mut states = initial_task_states(world);
    if !context.saved_states.is_empty() {
        let mut restored = BTreeMap::new();
        for saved in context.saved_states {
            let Some(task) = task_definition(world, &saved.task_id) else {
                return Err(CoreError::InvalidSave("task state ID is invalid"));
            };
            let expected = states
                .get(&saved.task_id)
                .cloned()
                .unwrap_or_else(|| task_initial_state(task, &states));
            let objectives = task_objectives(world, &saved.task_id);
            let Some(objective) = usize::try_from(saved.stage_index)
                .ok()
                .and_then(|stage| objectives.get(stage))
            else {
                return Err(CoreError::InvalidSave("task stage is invalid"));
            };
            let members = task_floors(world, &saved.task_id).collect::<Vec<_>>();
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
                TaskStatusKindDto::Available => {
                    saved.active_floor_id.is_none()
                        && (task.source_facility_id.is_none()
                            || expected.status == TaskStatusKindDto::Available)
                }
                TaskStatusKindDto::Failed | TaskStatusKindDto::Abandoned => {
                    saved.active_floor_id.is_none()
                }
                TaskStatusKindDto::Locked => {
                    saved.active_floor_id.is_none()
                        && expected.status == TaskStatusKindDto::Locked
                        && saved.stage_index == 0
                        && saved.current == 0
                }
                TaskStatusKindDto::RewardAvailable => {
                    saved.active_floor_id.is_none()
                        && task_definition(world, &saved.task_id)
                            .is_some_and(|task| task.source_facility_id.is_some())
                        && usize::try_from(saved.stage_index)
                            .ok()
                            .is_some_and(|stage| stage + 1 == objectives.len())
                        && saved.current == saved.required
                }
                TaskStatusKindDto::Taken => {
                    saved.active_floor_id.is_none()
                        && task_definition(world, &saved.task_id)
                            .is_some_and(|task| task.source_facility_id.is_some())
                }
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
        if expected_task_states_missing(&states, &restored) && !context.allow_missing_states {
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
        let members = task_floors(world, task_id).collect::<Vec<_>>();
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
        } else if active.is_some() {
            let objective = task_objectives(world, task_id)
                .first()
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
                TaskObjectiveKind::ClearFloor
                | TaskObjectiveKind::EnterFloor
                | TaskObjectiveKind::KillActorKind => {}
            }
        }
        state.current = state.current.min(state.required);
    }
    Ok(states)
}

fn expected_task_states_missing(
    expected: &BTreeMap<String, TaskState>,
    restored: &BTreeMap<String, TaskState>,
) -> bool {
    expected
        .keys()
        .any(|task_id| !restored.contains_key(task_id))
}

fn restore_character_progress(
    saved: Option<&PlayerProgressSaveDto>,
    saved_active_mutation_ids: &[String],
    saved_locked_mutation_ids: &[String],
    base_max_hp: i32,
    expected_skills: BTreeMap<String, SkillProgress>,
    class_id: Option<&str>,
    content: &ContentCatalog,
) -> Result<CharacterProgress, CoreError> {
    let active_mutation_ids = saved_active_mutation_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let locked_mutation_ids = saved_locked_mutation_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if active_mutation_ids.len() != saved_active_mutation_ids.len()
        || locked_mutation_ids.len() != saved_locked_mutation_ids.len()
        || !locked_mutation_ids.is_subset(&active_mutation_ids)
        || active_mutation_ids
            .iter()
            .any(|id| content.mutation(id).is_none())
    {
        return Err(CoreError::InvalidSave("player mutation state is invalid"));
    }
    let Some(saved) = saved else {
        let mut progress = CharacterProgress::legacy(base_max_hp);
        progress.replace_skills(expected_skills);
        progress.active_mutation_ids = active_mutation_ids;
        progress.locked_mutation_ids = locked_mutation_ids;
        return Ok(progress);
    };
    let mut skills = BTreeMap::new();
    for skill in &saved.skills {
        if skills
            .insert(
                skill.id.clone(),
                SkillProgress {
                    current: skill.current,
                    maximum: skill.maximum,
                    base: skill.base,
                    growth_per_ten_levels: skill.growth_per_ten_levels,
                },
            )
            .is_some()
        {
            return Err(CoreError::InvalidSave("player skill state is invalid"));
        }
    }
    if skills.is_empty() {
        skills = expected_skills;
    } else if skills != expected_skills {
        return Err(CoreError::InvalidSave("player skill state is invalid"));
    }
    let mut weapon_proficiencies = BTreeMap::new();
    for proficiency in &saved.weapon_proficiencies {
        if weapon_proficiencies
            .insert(proficiency.item_kind_id.clone(), proficiency.current)
            .is_some()
        {
            return Err(CoreError::InvalidSave(
                "player weapon proficiency state is invalid",
            ));
        }
    }
    let attributes = AttributeSet {
        strength: saved.attributes.strength,
        intelligence: saved.attributes.intelligence,
        wisdom: saved.attributes.wisdom,
        dexterity: saved.attributes.dexterity,
        constitution: saved.attributes.constitution,
        charisma: saved.attributes.charisma,
    };
    let maximum_attributes = saved
        .maximum_attributes
        .as_ref()
        .map(|maximum| AttributeSet {
            strength: maximum.strength,
            intelligence: maximum.intelligence,
            wisdom: maximum.wisdom,
            dexterity: maximum.dexterity,
            constitution: maximum.constitution,
            charisma: maximum.charisma,
        })
        .unwrap_or(attributes);
    let attribute_potentials = AttributeSet {
        strength: saved.attribute_potentials.strength,
        intelligence: saved.attribute_potentials.intelligence,
        wisdom: saved.attribute_potentials.wisdom,
        dexterity: saved.attribute_potentials.dexterity,
        constitution: saved.attribute_potentials.constitution,
        charisma: saved.attribute_potentials.charisma,
    };
    if ![
        AttributeKind::Strength,
        AttributeKind::Intelligence,
        AttributeKind::Wisdom,
        AttributeKind::Dexterity,
        AttributeKind::Constitution,
        AttributeKind::Charisma,
    ]
    .into_iter()
    .all(|kind| {
        let current = attributes.value(kind);
        let maximum = maximum_attributes.value(kind);
        current <= maximum
    }) {
        return Err(CoreError::InvalidSave("player attribute state is invalid"));
    }
    let progress = CharacterProgress {
        attributes,
        maximum_attributes,
        attribute_potentials,
        experience: saved.experience,
        maximum_experience: if saved.maximum_experience == 0 {
            saved.experience
        } else {
            saved.maximum_experience
        },
        life_force: saved.life_force,
        level: saved.level,
        max_level: saved.max_level,
        pending_attribute_increases: saved.pending_attribute_increases,
        hp_progression: saved.hp_progression.clone(),
        skills,
        weapon_proficiencies,
        active_mutation_ids,
        locked_mutation_ids,
    };
    if !super::weapon_proficiency::weapon_proficiency_progress_is_valid(
        content, class_id, &progress,
    ) {
        return Err(CoreError::InvalidSave(
            "player weapon proficiency state is invalid",
        ));
    }
    Ok(progress)
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
        if !entry.discovered
            || known_affix_ids.len() != known_affix_count
            || known_affix_ids.iter().any(|affix_id| {
                !item.affix_ids.contains(affix_id) || content.affix(affix_id).is_none()
            })
            || (identified && !all_affixes_known)
            || knowledge
                .insert(
                    entry.item_id,
                    ItemPropertyKnowledgeState {
                        discovered: entry.discovered,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StateHashPayloadV82<'a> {
    schema_version: u16,
    revision: u32,
    turn: u32,
    world_tick: u32,
    last_command_seq: u32,
    map_scale: MapScaleDto,
    wilderness_position: Option<Position>,
    wilderness_view_offset: Position,
    wilderness_seed: u64,
    world_travel_destination: Option<Position>,
    interface_locale: rfb_protocol::LocaleDto,
    mogaminator: rfb_protocol::MogaminatorSaveDto,
    terrain: TerrainSaveRef<'a>,
    player: PlayerSaveDto,
    entities: Vec<ActorSaveDto>,
    items: Vec<ItemSaveDto>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    gold_piles: Vec<rfb_protocol::GoldPileDto>,
    inventory: Vec<InventoryItemSaveDto>,
    equipment: Vec<EquipmentItemSaveDto>,
    carried_items: Vec<CarriedItemSaveDto>,
    item_knowledge: Vec<ItemKnowledgeSaveDto>,
    item_property_knowledge: Vec<ItemPropertyKnowledgeSaveDto>,
    task_states: Vec<TaskStateSaveDto>,
    dungeon_states: Vec<DungeonStateSaveDto>,
    defeated_unique_actor_kind_ids: Vec<&'a str>,
    town_states: Vec<TownStateSaveDto>,
    shop_states: Vec<ShopStateSaveDto>,
    home_states: Vec<HomeStateSaveDto>,
    campaign_state: CampaignStateSaveDto,
    next_item_instance_serial: u64,
    next_gold_pile_serial: u64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    explored: Vec<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    revealed_terrain: Vec<Position>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    floor_connections: Vec<FloorConnectionSaveDto>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    floor_regions: Vec<FloorRegionSaveDto>,
    rng: RngSaveDto,
    content_id: &'a str,
    world_id: &'a str,
    current_floor_id: &'a str,
    current_dungeon_instance_id: Option<&'a str>,
    reproduction_suppressed: bool,
    stored_floors: Vec<FloorSaveForHash<'a>>,
}

/// Borrowed twin of [`TerrainSaveDto`]: identical serde field names and order,
/// so the msgpack bytes (and therefore the state hash) stay unchanged while
/// the terrain id vector is no longer cloned per hash.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TerrainSaveRef<'a> {
    width: u16,
    height: u16,
    terrain_ids: &'a [String],
    glow: &'a [bool],
}

/// Borrowed twin of [`rfb_protocol::FloorSaveDto`] for hashing: the `explored`
/// field is omitted entirely, which serializes identically to the
/// cleared-and-skipped vector the owned path used to build and throw away.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FloorSaveForHash<'a> {
    id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    dungeon_instance_id: Option<&'a str>,
    reproduction_suppressed: bool,
    player_position: Position,
    terrain: TerrainSaveRef<'a>,
    entities: Vec<ActorSaveDto>,
    items: Vec<ItemSaveDto>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    gold_piles: Vec<rfb_protocol::GoldPileDto>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    carried_items: Vec<CarriedItemSaveDto>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    revealed_terrain: Vec<Position>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    connections: Vec<FloorConnectionSaveDto>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    regions: Vec<FloorRegionSaveDto>,
}

fn floor_save_for_hash(floor: &FloorState) -> FloorSaveForHash<'_> {
    FloorSaveForHash {
        id: &floor.id,
        dungeon_instance_id: floor.dungeon_instance_id.as_deref(),
        reproduction_suppressed: floor.reproduction_suppressed,
        player_position: floor.player_position,
        terrain: TerrainSaveRef {
            width: floor.width,
            height: floor.height,
            terrain_ids: &floor.terrain,
            glow: &floor.glow,
        },
        entities: actors_to_save(&floor.entities),
        items: items_to_save(&floor.items),
        gold_piles: gold_piles_to_save(&floor.gold_piles),
        carried_items: carried_items_to_save(&floor.items),
        revealed_terrain: floor.revealed_terrain.iter().copied().collect(),
        connections: floor_connections_to_save(&floor.connections),
        regions: floor_regions_to_save(&floor.regions),
    }
}

impl Game {
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
        if payload.schema_version != SAVE_PAYLOAD_SCHEMA_VERSION {
            return Err(CoreError::UnsupportedSaveVersion(payload.schema_version));
        }
        if payload.content_id != content.pack_id() || payload.content_hash != content.content_hash()
        {
            return Err(CoreError::ContentMismatch);
        }
        let mogaminator =
            super::mogaminator::MogaminatorState::from_save(payload.mogaminator.clone())
                .map_err(|_| CoreError::InvalidSave("Mogaminator source is invalid"))?;
        if mogaminator.wanted_actor_kind_ids.len() > 20
            || mogaminator.wanted_actor_kind_ids.iter().any(|actor_id| {
                content.actor(actor_id).is_none_or(|actor| {
                    !actor.tags.iter().any(|tag| tag == "unique")
                        || (actor.corpse_item_kind_id.is_none() && actor.remains.is_none())
                })
            })
        {
            return Err(CoreError::InvalidSave(
                "Mogaminator wanted target state is invalid",
            ));
        }
        let world = content
            .world(&payload.world_id)
            .ok_or_else(|| CoreError::UnknownWorld(payload.world_id.clone()))?;
        let wilderness_position = match (&world.wilderness, payload.wilderness_position) {
            (Some(wilderness), Some(position)) => {
                let symbol = usize::try_from(position.y)
                    .ok()
                    .and_then(|y| wilderness.rows.get(y))
                    .and_then(|row| {
                        usize::try_from(position.x)
                            .ok()
                            .and_then(|x| row.as_bytes().get(x))
                    });
                let valid = symbol.is_some_and(|symbol| {
                    wilderness.legend.iter().any(|entry| {
                        entry.symbol.as_bytes() == [*symbol]
                            && entry.terrain != WildernessTerrain::Edge
                    })
                });
                if !valid {
                    return Err(CoreError::InvalidSave("wilderness position is invalid"));
                }
                Some(position)
            }
            (None, None) => None,
            _ => return Err(CoreError::InvalidSave("wilderness position is invalid")),
        };
        if let Some(destination) = payload.world_travel_destination {
            let valid = world.wilderness.as_ref().is_some_and(|wilderness| {
                usize::try_from(destination.y)
                    .ok()
                    .and_then(|y| wilderness.rows.get(y))
                    .and_then(|row| {
                        usize::try_from(destination.x)
                            .ok()
                            .and_then(|x| row.as_bytes().get(x))
                    })
                    .is_some_and(|symbol| {
                        wilderness.legend.iter().any(|entry| {
                            entry.symbol.as_bytes() == [*symbol]
                                && entry.terrain != WildernessTerrain::Edge
                        })
                    })
            });
            if !valid {
                return Err(CoreError::InvalidSave(
                    "world travel destination is invalid",
                ));
            }
        }
        let mut legacy_task_progress = BTreeMap::new();
        for progress in &payload.task_progress {
            let Some(task) = task_definition(world, &progress.task_id) else {
                return Err(CoreError::InvalidSave("task progress floor ID is invalid"));
            };
            let required = task
                .objectives
                .first()
                .map_or(1, |objective| objective.required);
            if progress.current > required
                || legacy_task_progress
                    .insert(task.id.clone(), progress.current)
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
            && current_floor_id != WILDERNESS_FLOOR_ID
            && !world
                .procedural_floors
                .iter()
                .any(|floor| floor.id == current_floor_id)
        {
            return Err(CoreError::InvalidSave("current floor ID is invalid"));
        }
        if payload.map_scale == MapScaleDto::World
            && (world.wilderness.is_none()
                || (current_floor_id != WILDERNESS_FLOOR_ID
                    && world_town_for_floor(world, &content, &current_floor_id).is_none()))
        {
            return Err(CoreError::InvalidSave("world map state is invalid"));
        }
        if current_floor_id == WILDERNESS_FLOOR_ID && world.wilderness.is_none() {
            return Err(CoreError::InvalidSave("local wilderness state is invalid"));
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
        let mut dungeon_states = restore_dungeon_states(world, &payload.dungeon_states, false)?;
        let (town_states, shop_states) = restore_town_and_shop_states(
            world,
            &content,
            &current_floor_id,
            payload.player.position,
            &payload.town_states,
            &payload.shop_states,
        )?;
        let home_states = restore_home_states(
            world,
            &content,
            &town_states,
            &current_floor_id,
            payload.player.position,
            &payload.home_states,
        )?;
        let expected_len = usize::from(payload.terrain.width) * usize::from(payload.terrain.height);
        if expected_len == 0
            || payload.terrain.terrain_ids.len() != expected_len
            || payload.terrain.glow.len() != expected_len
        {
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
        let player_definition = content
            .actor(&payload.player.kind_id)
            .ok_or_else(|| CoreError::UnknownActor(payload.player.kind_id.clone()))?;
        let build = resolve_character_build(
            &content,
            payload
                .player
                .build
                .as_ref()
                .map(|build| build.build_id.as_str())
                .or(world.player_build_id.as_deref()),
        )?;
        if let (Some(saved), Some(identity)) = (payload.player.build.as_ref(), build.as_ref())
            && (saved.race_id != identity.race_id
                || saved.class_id != identity.class_id
                || saved.personality_id != identity.personality_id)
        {
            return Err(CoreError::InvalidSave("player build identity is invalid"));
        }
        let saved_level = payload
            .player
            .progress
            .as_ref()
            .map_or(1, |progress| progress.level);
        let expected_skills = character_skill_progress(&content, build.as_ref(), saved_level)?;
        let progress = restore_character_progress(
            payload.player.progress.as_ref(),
            &payload.player.active_mutation_ids,
            &payload.player.locked_mutation_ids,
            player_definition.max_hp,
            expected_skills,
            build.as_ref().map(|build| build.class_id.as_str()),
            &content,
        )?;
        let saved_resources = payload.player.resources.clone();
        let bonus_spell_learning_capacity = payload.player.bonus_spell_learning_capacity;
        let saved_learned_ability_ids = payload.player.learned_ability_ids.clone();
        let saved_ability_progress = payload.player.ability_progress.clone();
        let summon_command = payload.player.summon_command.clone();
        let recall = payload.player.recall.clone();
        let riding_actor_id = payload.player.riding_actor_id.clone();
        let confusing_strike_ready = payload.player.confusing_strike_ready;
        let minor_slow = payload.player.minor_slow;
        if minor_slow > 10 {
            return Err(CoreError::InvalidSave("player minor slow is invalid"));
        }
        let minor_slow_energy = payload.player.minor_slow_energy;
        let chaos_patron_id = payload.player.chaos_patron_id.clone();
        let patrons = super::chaos_patron::chaos_patrons(&content);
        if match chaos_patron_id.as_deref() {
            Some(patron_id) => !patrons.iter().any(|patron| patron.id == patron_id),
            None => !patrons.is_empty(),
        } {
            return Err(CoreError::InvalidSave("player chaos patron is invalid"));
        }
        let reality_change_ticks = payload.player.reality_change_ticks;
        if reality_change_ticks > 35 {
            return Err(CoreError::InvalidSave(
                "player reality change countdown is invalid",
            ));
        }
        let pending_mutation_direction = payload.player.pending_mutation_direction.clone();
        if pending_mutation_direction.as_ref().is_some_and(|pending| {
            pending.mutation_id != "rfb.mutation.prod-mana"
                || !progress.active_mutation_ids.contains(&pending.mutation_id)
                || payload.map_scale != MapScaleDto::Local
        }) {
            return Err(CoreError::InvalidSave(
                "pending mutation direction is invalid",
            ));
        }
        // Body slots are save-authoritative once present; pre-template saves
        // derive them from the build's race (or the standard body) with no
        // RNG involvement.
        let body_slots = if payload.player.body_slots.is_empty() {
            resolve_body_slots(&content, build.as_ref())?
        } else {
            let mut seen_slot_ids = BTreeSet::new();
            let slots = payload
                .player
                .body_slots
                .iter()
                .map(|slot| BodySlot {
                    id: slot.id.clone(),
                    slot_type: slot.slot_type.clone(),
                })
                .collect::<Vec<_>>();
            if slots.len() > 64
                || slots.iter().any(|slot| {
                    slot.id.is_empty()
                        || slot.slot_type.is_empty()
                        || !seen_slot_ids.insert(slot.id.clone())
                })
            {
                return Err(CoreError::InvalidSave("player body slots are invalid"));
            }
            slots
        };
        let gold = payload.player.gold;
        let nutrition = payload.player.nutrition;
        let player_name =
            normalize_player_name(&payload.player.name).ok_or(CoreError::InvalidPlayerName)?;
        let player = actor_from_player(payload.player, &content)?;
        let entities = payload
            .entities
            .into_iter()
            .map(|entity| actor_from_entity(entity, &content))
            .collect::<Result<Vec<_>, CoreError>>()?;
        let mut items = payload
            .items
            .into_iter()
            .map(|item| item_from_dto(item, &content))
            .collect::<Result<Vec<_>, CoreError>>()?;
        items.extend(
            payload
                .inventory
                .into_iter()
                .map(|item| inventory_item_from_dto(item, &content))
                .collect::<Result<Vec<_>, CoreError>>()?,
        );
        let gold_piles = gold_piles_from_save(payload.gold_piles);
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
                    && floor.id != wilderness::WILDERNESS_FLOOR_ID
                    && !world
                        .procedural_floors
                        .iter()
                        .any(|definition| definition.id == floor.id))
                || stored_floors.insert(storage_key, floor).is_some()
            {
                return Err(CoreError::InvalidSave("stored floor state is invalid"));
            }
        }
        if world_town_for_floor(world, &content, &current_floor_id).is_some() {
            stored_floors.retain(|_, stored| {
                if world_town_for_floor(world, &content, &stored.id).is_some() {
                    return true;
                }
                if world.procedural_floors.iter().any(|floor| {
                    floor.id == stored.id
                        && floor.lifecycle == FloorLifecycle::OneShot
                        && floor.retakeable
                }) {
                    return true;
                }
                let Some(dungeon_id) = floor_dungeon_id(world, &stored.id) else {
                    return false;
                };
                dungeon_states
                    .get(&dungeon_id)
                    .and_then(|state| state.retained_instance_id.as_deref())
                    == stored.dungeon_instance_id.as_deref()
            });
        }
        let mut allocator_entities = entities.clone();
        let mut allocator_items = items.clone();
        for floor in stored_floors.values() {
            allocator_entities.extend(floor.entities.iter().cloned());
            allocator_items.extend(floor.items.iter().cloned());
        }
        allocator_items.extend(
            shop_states
                .values()
                .flat_map(|state| state.inventory.iter().cloned()),
        );
        allocator_items.extend(
            home_states
                .values()
                .flat_map(|state| state.inventory.iter().cloned()),
        );
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
        let derived_next_gold_pile_serial = derive_next_gold_pile_serial(
            gold_piles.iter().chain(
                stored_floors
                    .values()
                    .flat_map(|floor| floor.gold_piles.iter()),
            ),
        )?;
        let next_gold_pile_serial = if payload.next_gold_pile_serial == 0 {
            derived_next_gold_pile_serial
        } else if payload.next_gold_pile_serial < derived_next_gold_pile_serial {
            return Err(CoreError::InvalidSave(
                "gold pile allocator is behind existing IDs",
            ));
        } else {
            payload.next_gold_pile_serial
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
        let mut item_property_knowledge = item_property_knowledge_from_save(
            payload.item_property_knowledge,
            &allocator_items,
            &content,
        )?;
        for item in &items {
            if matches!(
                item.location,
                ItemLocation::Inventory | ItemLocation::Equipped { .. }
            ) {
                let knowledge = item_property_knowledge.entry(item.id.clone()).or_default();
                knowledge.discovered = true;
            }
            if matches!(item.location, ItemLocation::Equipped { .. }) {
                let knowledge = item_property_knowledge
                    .get_mut(&item.id)
                    .expect("equipped item knowledge was initialized");
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
                allow_missing_states: false,
            },
        )?;
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
        let campaign_state_missing = payload.campaign_state.is_none();
        let campaign_state = restore_campaign_state(payload.campaign_state.as_ref())?;
        let defeated_unique_count = payload.defeated_unique_actor_kind_ids.len();
        let defeated_unique_actor_kind_ids = payload
            .defeated_unique_actor_kind_ids
            .into_iter()
            .collect::<BTreeSet<_>>();
        if defeated_unique_actor_kind_ids.len() != defeated_unique_count
            || defeated_unique_actor_kind_ids.iter().any(|kind_id| {
                !content.actor(kind_id).is_some_and(|definition| {
                    definition.tags.iter().any(|tag| tag == "unique")
                        && !definition.tags.iter().any(|tag| tag == "guardian")
                })
            })
        {
            return Err(CoreError::InvalidSave(
                "defeated unique actor state is invalid",
            ));
        }
        let mut game = Self {
            content,
            world_id: payload.world_id,
            map_scale: payload.map_scale,
            wilderness_position,
            wilderness_view_offset: payload.wilderness_view_offset,
            wilderness_seed: payload.wilderness_seed,
            wilderness_terrain_cache: BTreeMap::new(),
            world_travel_destination: payload.world_travel_destination,
            interface_locale: payload.interface_locale,
            mogaminator,
            current_floor_id,
            current_dungeon_instance_id,
            reproduction_suppressed: payload.reproduction_suppressed,
            stored_floors,
            width: payload.terrain.width,
            height: payload.terrain.height,
            terrain,
            glow: payload.terrain.glow,
            player_name,
            player,
            riding_actor_id,
            gold,
            nutrition,
            build,
            body_slots,
            progress,
            resources: BTreeMap::new(),
            last_visual_cells: None,
            bonus_spell_learning_capacity,
            learned_abilities: BTreeSet::new(),
            ability_progress: BTreeMap::new(),
            entities,
            items,
            gold_piles,
            item_knowledge,
            item_property_knowledge,
            task_states,
            command_actor_deaths: Vec::new(),
            dungeon_states,
            defeated_unique_actor_kind_ids,
            town_states,
            shop_states,
            home_states,
            campaign_state,
            summon_command,
            recall,
            confusing_strike_ready,
            minor_slow,
            minor_slow_energy,
            chaos_patron_id,
            reality_change_ticks,
            pending_mutation_direction,
            next_item_instance_serial,
            next_gold_pile_serial,
            explored,
            revealed_terrain,
            floor_connections,
            floor_regions,
            rng: RfbRng::from_save(&payload.rng)?,
            revision: payload.revision,
            turn: payload.turn,
            world_tick: payload.world_tick,
            last_non_melee_fear_aura_tick: None,
            last_command_seq: payload.last_command_seq,
            debug_ability_casts_succeed: false,
            debug_recharge_attempts_succeed: false,
            debug_recharge_attempts_fail: false,
            debug_recharge_sources_survive: false,
            debug_recall_delay_turns: None,
            debug_item_curses_land: false,
            debug_item_curses_resisted: false,
            monster_division_remainders: BTreeMap::new(),
        };
        game.restore_player_ability_state(
            saved_resources,
            saved_learned_ability_ids,
            saved_ability_progress,
        )?;
        if campaign_state_missing && game.campaign_victory_reached() {
            game.campaign_state.status = CampaignStatusDto::Victorious;
            game.campaign_state.victory_turn = Some(game.turn);
        }
        // A victorious/retired v70 save may contain experience banked at the
        // pre-victory cap. Reconcile the newly unlocked cap during load so
        // the authoritative level and HP are not dependent on a later input.
        game.apply_player_experience(0, &mut Vec::new());
        game.reveal_current_visibility();
        game.clear_stale_mogaminator_query();
        game.validate_loaded_state()?;
        Ok(game)
    }

    #[must_use]
    pub fn to_save(&self) -> SavePayloadV1 {
        SavePayloadV1 {
            schema_version: SAVE_PAYLOAD_SCHEMA_VERSION,
            revision: self.revision,
            turn: self.turn,
            world_tick: self.world_tick,
            last_command_seq: self.last_command_seq,
            map_scale: self.map_scale,
            wilderness_position: self.wilderness_position,
            wilderness_view_offset: self.wilderness_view_offset,
            wilderness_seed: self.wilderness_seed,
            world_travel_destination: self.world_travel_destination,
            interface_locale: self.interface_locale,
            mogaminator: self.mogaminator.to_save(),
            terrain: TerrainSaveDto {
                width: self.width,
                height: self.height,
                terrain_ids: self.terrain.clone(),
                glow: self.glow.clone(),
            },
            player: self.player_save_dto(),
            entities: actors_to_save(&self.entities),
            items: items_to_save(&self.items),
            gold_piles: gold_piles_to_save(&self.gold_piles),
            inventory: inventory_to_save(&self.items),
            equipment: equipment_to_save(&self.items),
            carried_items: carried_items_to_save(&self.items),
            item_knowledge: self.item_knowledge_to_save(),
            item_property_knowledge: self.item_property_knowledge_to_save(),
            task_progress: Vec::new(),
            task_states: self.task_states_to_save(),
            dungeon_states: self.dungeon_states_to_save(),
            defeated_unique_actor_kind_ids: self
                .defeated_unique_actor_kind_ids
                .iter()
                .cloned()
                .collect(),
            town_states: self
                .town_states
                .iter()
                .map(|(town_id, state)| TownStateSaveDto {
                    town_id: town_id.clone(),
                    visited: state.visited,
                })
                .collect(),
            shop_states: self
                .shop_states
                .iter()
                .map(|(shop_id, state)| shop_state_to_save(shop_id, state))
                .collect(),
            home_states: self
                .home_states
                .iter()
                .map(|(facility_id, state)| home_state_to_save(facility_id, state))
                .collect(),
            campaign_state: Some(self.campaign_state_to_save()),
            next_item_instance_serial: self.next_item_instance_serial,
            next_gold_pile_serial: self.next_gold_pile_serial,
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
            reproduction_suppressed: self.reproduction_suppressed,
            stored_floors: self.stored_floors.values().map(floor_to_save).collect(),
        }
    }

    #[must_use]
    pub fn state_hash(&self) -> String {
        let payload = StateHashPayloadV82 {
            schema_version: STATE_HASH_SCHEMA_VERSION,
            revision: self.revision,
            turn: self.turn,
            world_tick: self.world_tick,
            last_command_seq: self.last_command_seq,
            map_scale: self.map_scale,
            wilderness_position: self.wilderness_position,
            wilderness_view_offset: self.wilderness_view_offset,
            wilderness_seed: self.wilderness_seed,
            world_travel_destination: self.world_travel_destination,
            interface_locale: self.interface_locale,
            mogaminator: self.mogaminator.to_save(),
            terrain: TerrainSaveRef {
                width: self.width,
                height: self.height,
                terrain_ids: &self.terrain,
                glow: &self.glow,
            },
            player: self.player_save_dto(),
            entities: actors_to_save(&self.entities),
            items: items_to_save(&self.items),
            gold_piles: gold_piles_to_save(&self.gold_piles),
            inventory: inventory_to_save(&self.items),
            equipment: equipment_to_save(&self.items),
            carried_items: carried_items_to_save(&self.items),
            item_knowledge: self.item_knowledge_to_save(),
            item_property_knowledge: self.item_property_knowledge_to_save(),
            task_states: self.task_states_to_save(),
            dungeon_states: self.dungeon_states_to_save(),
            defeated_unique_actor_kind_ids: self
                .defeated_unique_actor_kind_ids
                .iter()
                .map(String::as_str)
                .collect(),
            town_states: self
                .town_states
                .iter()
                .map(|(town_id, state)| TownStateSaveDto {
                    town_id: town_id.clone(),
                    visited: state.visited,
                })
                .collect(),
            shop_states: self
                .shop_states
                .iter()
                .map(|(shop_id, state)| shop_state_to_save(shop_id, state))
                .collect(),
            home_states: self
                .home_states
                .iter()
                .map(|(facility_id, state)| home_state_to_save(facility_id, state))
                .collect(),
            campaign_state: self.campaign_state_to_save(),
            next_item_instance_serial: self.next_item_instance_serial,
            next_gold_pile_serial: self.next_gold_pile_serial,
            explored: Vec::new(),
            revealed_terrain: self.revealed_terrain.iter().copied().collect(),
            floor_connections: floor_connections_to_save(&self.floor_connections),
            floor_regions: floor_regions_to_save(&self.floor_regions),
            rng: self.rng.to_save(),
            content_id: self.content.pack_id(),
            world_id: &self.world_id,
            current_floor_id: &self.current_floor_id,
            current_dungeon_instance_id: self.current_dungeon_instance_id.as_deref(),
            reproduction_suppressed: self.reproduction_suppressed,
            stored_floors: self
                .stored_floors
                .values()
                .map(floor_save_for_hash)
                .collect(),
        };
        let bytes = rmp_serde::to_vec_named(&payload)
            .expect("serializing the internal save state should not fail");
        let digest = Sha256::digest(bytes);
        format!("{digest:x}")
    }

    fn player_save_dto(&self) -> PlayerSaveDto {
        let mut player = player_to_save(
            &self.player_name,
            &self.player,
            &self.progress,
            self.build.as_ref(),
        );
        player.gold = self.gold;
        player.nutrition = self.nutrition;
        player.resources = self
            .resources
            .iter()
            .map(|(id, pool)| ResourcePoolSaveDto {
                id: id.clone(),
                current: pool.current,
                maximum: pool.maximum,
            })
            .collect();
        player.bonus_spell_learning_capacity = self.bonus_spell_learning_capacity;
        player.learned_ability_ids = self.learned_abilities.iter().cloned().collect();
        player.ability_progress = self
            .ability_progress
            .iter()
            .map(|(id, progress)| AbilityProgressSaveDto {
                id: id.clone(),
                proficiency: progress.proficiency,
                proficiency_cap: progress.proficiency_cap,
                cast_count: progress.cast_count,
                fail_count: progress.fail_count,
                cooldown_remaining: progress.cooldown_remaining,
            })
            .collect();
        player.summon_command = self.summon_command.clone();
        player.recall = self.recall.clone();
        player.riding_actor_id = self.riding_actor_id.clone();
        player.confusing_strike_ready = self.confusing_strike_ready;
        player.minor_slow = self.minor_slow;
        player.minor_slow_energy = self.minor_slow_energy;
        player.chaos_patron_id = self.chaos_patron_id.clone();
        player.reality_change_ticks = self.reality_change_ticks;
        player.pending_mutation_direction = self.pending_mutation_direction.clone();
        player.body_slots = self
            .body_slots
            .iter()
            .map(|slot| BodySlotSaveDto {
                id: slot.id.clone(),
                slot_type: slot.slot_type.clone(),
            })
            .collect();
        player
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
        let all_items = self
            .items
            .iter()
            .chain(
                self.stored_floors
                    .values()
                    .flat_map(|floor| floor.items.iter()),
            )
            .chain(
                self.shop_states
                    .values()
                    .flat_map(|state| state.inventory.iter()),
            )
            .chain(
                self.home_states
                    .values()
                    .flat_map(|state| state.inventory.iter()),
            )
            .collect::<Vec<_>>();
        let mut item_ids = self
            .item_property_knowledge
            .keys()
            .filter(|item_id| all_items.iter().any(|item| item.id == item_id.as_str()))
            .cloned()
            .collect::<BTreeSet<_>>();
        item_ids.extend(
            self.items
                .iter()
                .filter(|item| {
                    matches!(
                        item.location,
                        ItemLocation::Inventory | ItemLocation::Equipped { .. }
                    )
                })
                .map(|item| item.id.clone()),
        );
        item_ids
            .into_iter()
            .map(|item_id| {
                let knowledge = self.item_property_knowledge.get(&item_id);
                let held = self.items.iter().any(|item| {
                    item.id == item_id
                        && matches!(
                            item.location,
                            ItemLocation::Inventory | ItemLocation::Equipped { .. }
                        )
                });
                ItemPropertyKnowledgeSaveDto {
                    item_id,
                    discovered: held || knowledge.is_some_and(|knowledge| knowledge.discovered),
                    appraised: knowledge.is_some_and(|knowledge| knowledge.appraised),
                    identified: knowledge.is_some_and(|knowledge| knowledge.identified),
                    known_affix_ids: knowledge
                        .map(|knowledge| knowledge.known_affix_ids.iter().cloned().collect())
                        .unwrap_or_default(),
                }
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
                entrance_guardian_defeated: Some(state.entrance_guardian_defeated),
                next_instance_ordinal: state.next_instance_ordinal,
                retained_instance_id: state.retained_instance_id.clone(),
                retained_at_turn: state.retained_at_turn,
            })
            .collect()
    }

    fn campaign_state_to_save(&self) -> CampaignStateSaveDto {
        CampaignStateSaveDto {
            status: self.campaign_state.status,
            victory_turn: self.campaign_state.victory_turn,
            retired_turn: self.campaign_state.retired_turn,
            final_score: self.campaign_state.final_score,
        }
    }
}
