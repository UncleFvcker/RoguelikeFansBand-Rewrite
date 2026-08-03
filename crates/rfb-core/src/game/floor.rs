// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

use rfb_content::{
    DungeonDefinition, DungeonEntryRequirementDefinition, DungeonEntryTaskStatus,
    DungeonInstanceLifecycle, FloorLifecycle, ProceduralFloorDefinition, RetakeFloorPolicy,
    TerrainDefinition, WorldDefinition,
};
use rfb_protocol::{
    ItemEnchantmentsDto, ItemQualityDto, Position, RecallStateDto, SummonCommandModeDto,
    TaskStatusKindDto,
};

use crate::{
    error::CoreError,
    event::DomainEvent,
    save::initial_item_fuel,
    state::{Actor, FloorConnectionState, FloorState, ItemInstance, ItemLocation},
};

use super::tasks::{
    TaskResolution, TaskState, activated_task_state, floor_task_id, initial_task_states,
    task_objectives, task_resolution_for_departure, task_state_after_departure, task_succeeded,
};
use super::{
    DungeonState, Game, chebyshev_distance, initial_item_curse, initial_item_runtime_state,
};

pub(super) type ActorIdentity = (String, String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct FloorTransitionTarget {
    pub(super) floor_id: String,
    pub(super) arrival_connection_id: Option<String>,
    pub(super) departure_connection_id: Option<String>,
}

pub(super) struct FloorTransitionOutcome {
    pub(super) from_floor_id: String,
    pub(super) to_floor_id: String,
    pub(super) expedition_ended: bool,
    pub(super) one_shot_closed: Option<(String, TaskResolution)>,
    pub(super) task_paused: Option<String>,
    pub(super) task_resumed: Option<String>,
    pub(super) summons_followed: Vec<ActorIdentity>,
    pub(super) summons_could_not_follow: Vec<ActorIdentity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RecallUseAction {
    Start,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RecallDestination {
    pub(super) dungeon_id: String,
    pub(super) floor_id: String,
}

enum RetainedInstanceAction {
    None,
    Take {
        dungeon_id: String,
    },
    Expire {
        dungeon_id: String,
        instance_id: String,
    },
}

enum ExpeditionEndAction {
    Reset {
        instance_id: String,
    },
    Retain {
        dungeon_id: String,
        instance_id: String,
        retained_at_turn: u32,
    },
}

struct OneShotDeparturePlan {
    task_id: String,
    members: Vec<ProceduralFloorDefinition>,
    resolution: Option<TaskResolution>,
    initial_required: u32,
    retakeable: bool,
}

struct OneShotArrivalPlan {
    task_id: String,
    floor_id: String,
    resumed: bool,
    regenerate_members: Vec<ProceduralFloorDefinition>,
}

struct FloorTransitionPlan {
    from_floor_id: String,
    from_dungeon_instance_id: Option<String>,
    from_storage_key: String,
    target_floor_id: String,
    target_dungeon_instance_id: Option<String>,
    target_storage_key: String,
    target_definition: Option<ProceduralFloorDefinition>,
    arrival_connection_id: Option<String>,
    departure_connection_id: Option<String>,
    retained_instance_action: RetainedInstanceAction,
    allocated_dungeon_instance: Option<(String, u32)>,
    expedition_end: Option<ExpeditionEndAction>,
    one_shot_departure: Option<OneShotDeparturePlan>,
    one_shot_arrival: Option<OneShotArrivalPlan>,
    following_summon_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RecallAdvancePlan {
    Countdown(u16),
    Trigger {
        from_floor_id: String,
        target_floor_id: String,
    },
}

pub(super) fn dungeon_instance_storage_key(instance_id: Option<&str>, floor_id: &str) -> String {
    match instance_id {
        Some(instance_id) => format!("{instance_id}::{floor_id}"),
        None => floor_id.to_owned(),
    }
}

pub(super) fn floor_dungeon_id(world: &WorldDefinition, floor_id: &str) -> Option<String> {
    world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == floor_id)
        .and_then(|floor| floor.dungeon_id.clone())
}

pub(super) fn dungeon_instance_id(dungeon_id: &str, ordinal: u32) -> String {
    format!("{dungeon_id}.instance.{ordinal}")
}

pub(super) fn parse_dungeon_instance_ordinal(instance_id: &str, dungeon_id: &str) -> Option<u32> {
    instance_id
        .strip_prefix(&format!("{dungeon_id}.instance."))
        .and_then(|ordinal| ordinal.parse::<u32>().ok())
        .filter(|ordinal| *ordinal > 0)
}

fn dungeon_entry_requirements_met(
    dungeon: &DungeonDefinition,
    task_states: &BTreeMap<String, TaskState>,
    dungeon_states: &BTreeMap<String, DungeonState>,
    items: &[ItemInstance],
) -> bool {
    dungeon
        .entry_requirements
        .iter()
        .all(|requirement| match requirement {
            DungeonEntryRequirementDefinition::TaskStatus { task_id, status } => {
                task_states.get(task_id).is_some_and(|state| {
                    matches!(
                        (state.status, status),
                        (
                            TaskStatusKindDto::Available,
                            DungeonEntryTaskStatus::Available
                        ) | (TaskStatusKindDto::Active, DungeonEntryTaskStatus::Active)
                            | (TaskStatusKindDto::Paused, DungeonEntryTaskStatus::Paused)
                            | (
                                TaskStatusKindDto::Completed,
                                DungeonEntryTaskStatus::Completed
                            )
                            | (TaskStatusKindDto::Failed, DungeonEntryTaskStatus::Failed)
                            | (
                                TaskStatusKindDto::Abandoned,
                                DungeonEntryTaskStatus::Abandoned
                            )
                    )
                })
            }
            DungeonEntryRequirementDefinition::DungeonConquered { dungeon_id } => dungeon_states
                .get(dungeon_id)
                .is_some_and(|state| state.guardian_defeated),
            DungeonEntryRequirementDefinition::CarriedItem {
                item_kind_id,
                quantity,
            } => {
                items
                    .iter()
                    .filter(|item| {
                        item.kind_id == *item_kind_id
                            && matches!(
                                item.location,
                                ItemLocation::Inventory | ItemLocation::Equipped { .. }
                            )
                    })
                    .fold(0_u32, |total, item| total.saturating_add(item.quantity))
                    >= *quantity
            }
        })
}

fn stair_transition_target(
    world: &WorldDefinition,
    current_floor_id: &str,
    terrain_id: &str,
    terrain: &TerrainDefinition,
    floor_connections: &[FloorConnectionState],
    player_position: Position,
    abandon_task: bool,
) -> Result<Option<FloorTransitionTarget>, CoreError> {
    if abandon_task
        && !world
            .procedural_floors
            .iter()
            .any(|floor| floor.id == current_floor_id && floor.lifecycle == FloorLifecycle::OneShot)
    {
        return Ok(None);
    }
    if abandon_task {
        return Ok(Some(FloorTransitionTarget {
            floor_id: world.initial_floor_id.clone(),
            arrival_connection_id: None,
            departure_connection_id: None,
        }));
    }
    if current_floor_id == world.initial_floor_id {
        return Ok(world
            .procedural_floors
            .iter()
            .find(|floor| {
                floor.return_floor_id == world.initial_floor_id
                    && floor.entry_terrain_id.as_deref() == Some(terrain_id)
            })
            .map(|target| FloorTransitionTarget {
                floor_id: target.id.clone(),
                arrival_connection_id: target.entry_connection_id.clone(),
                departure_connection_id: None,
            }));
    }
    let Some(current) = world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == current_floor_id)
    else {
        return Ok(None);
    };
    if let Some(connection_state) = floor_connections
        .iter()
        .find(|connection| connection.position == player_position)
    {
        let connection = current
            .connections
            .iter()
            .find(|connection| connection.id == connection_state.id)
            .ok_or(CoreError::InvalidSave(
                "active floor connection is missing from content",
            ))?;
        return Ok(Some(FloorTransitionTarget {
            floor_id: connection_state
                .target_floor_id
                .clone()
                .unwrap_or_else(|| connection.target_floor_id.clone()),
            arrival_connection_id: if connection_state.target_floor_id.is_some() {
                connection_state.target_connection_id.clone()
            } else {
                connection.target_connection_id.clone()
            },
            departure_connection_id: Some(connection_state.id.clone()),
        }));
    }
    if terrain.tags.iter().any(|tag| tag == "stairs-up") {
        return Ok(Some(FloorTransitionTarget {
            floor_id: current.return_floor_id.clone(),
            arrival_connection_id: None,
            departure_connection_id: None,
        }));
    }
    if terrain.tags.iter().any(|tag| tag == "stairs-down") {
        return Ok(Some(FloorTransitionTarget {
            floor_id: current.next_floor_id.clone().ok_or(CoreError::InvalidSave(
                "downward floor connection is missing",
            ))?,
            arrival_connection_id: None,
            departure_connection_id: None,
        }));
    }
    Ok(None)
}

fn plan_retained_instance_action(
    dungeon_id: &str,
    lifecycle: &DungeonInstanceLifecycle,
    state: Option<&DungeonState>,
    stored_floors: &BTreeMap<String, FloorState>,
    turn: u32,
) -> Result<(RetainedInstanceAction, Option<String>), CoreError> {
    let Some(state) = state else {
        return Err(CoreError::InvalidSave("dungeon state is missing"));
    };
    let Some(instance_id) = state.retained_instance_id.clone() else {
        return Ok((RetainedInstanceAction::None, None));
    };
    let retained_at_turn = state
        .retained_at_turn
        .ok_or(CoreError::InvalidSave("retained dungeon turn is missing"))?;
    if !stored_floors
        .values()
        .any(|floor| floor.dungeon_instance_id.as_deref() == Some(instance_id.as_str()))
    {
        return Err(CoreError::InvalidSave(
            "retained dungeon instance is missing",
        ));
    }
    let expired = matches!(
        lifecycle,
        DungeonInstanceLifecycle::TurnTtl { ttl_turns }
            if turn.saturating_sub(retained_at_turn) >= *ttl_turns
    );
    if expired {
        Ok((
            RetainedInstanceAction::Expire {
                dungeon_id: dungeon_id.to_owned(),
                instance_id,
            },
            None,
        ))
    } else {
        Ok((
            RetainedInstanceAction::Take {
                dungeon_id: dungeon_id.to_owned(),
            },
            Some(instance_id),
        ))
    }
}

fn recall_destination_for_current_floor(
    world: &WorldDefinition,
    current_floor_id: &str,
    recall: Option<&RecallStateDto>,
) -> Option<RecallStateDto> {
    let current = world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == current_floor_id && floor.lifecycle == FloorLifecycle::Dungeon)?;
    let dungeon_id = current
        .dungeon_id
        .as_ref()
        .expect("validated dungeon floor must retain its dungeon ID")
        .clone();
    let should_update = recall.is_none_or(|recall| {
        if recall.dungeon_id != dungeon_id {
            return true;
        }
        world
            .procedural_floors
            .iter()
            .find(|floor| floor.id == recall.floor_id)
            .is_none_or(|destination| current.depth >= destination.depth)
    });
    should_update.then(|| RecallStateDto {
        dungeon_id,
        floor_id: current.id.clone(),
        remaining_turns: recall.and_then(|recall| recall.remaining_turns),
    })
}

impl Game {
    pub(super) fn dungeon_entry_requirements_met(&self, dungeon: &DungeonDefinition) -> bool {
        dungeon_entry_requirements_met(
            dungeon,
            &self.task_states,
            &self.dungeon_states,
            &self.items,
        )
    }

    pub(super) fn teleport_level_targets(
        &self,
    ) -> (Vec<FloorTransitionTarget>, Vec<FloorTransitionTarget>) {
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        if self.current_floor_id == world.initial_floor_id {
            let downward = self
                .recall
                .as_ref()
                .filter(|recall| recall.remaining_turns.is_none())
                .and_then(|recall| {
                    let floor = world.procedural_floors.iter().find(|floor| {
                        floor.id == recall.floor_id
                            && floor.lifecycle == FloorLifecycle::Dungeon
                            && floor.dungeon_id.as_deref() == Some(recall.dungeon_id.as_str())
                    })?;
                    let dungeon = world
                        .dungeons
                        .iter()
                        .find(|dungeon| dungeon.id == recall.dungeon_id)?;
                    self.dungeon_entry_requirements_met(dungeon)
                        .then_some(FloorTransitionTarget {
                            floor_id: floor.id.clone(),
                            arrival_connection_id: None,
                            departure_connection_id: None,
                        })
                })
                .into_iter()
                .collect();
            return (Vec::new(), downward);
        }

        let Some(current) = world
            .procedural_floors
            .iter()
            .find(|floor| floor.id == self.current_floor_id)
            .filter(|floor| floor.lifecycle == FloorLifecycle::Dungeon)
        else {
            return (Vec::new(), Vec::new());
        };
        let current_dungeon_id = current.dungeon_id.as_deref();
        let mut upward = Vec::new();
        let mut downward = Vec::new();
        for state in &self.floor_connections {
            let Some(connection) = current
                .connections
                .iter()
                .find(|connection| connection.id == state.id)
            else {
                continue;
            };
            let target_floor_id = state
                .target_floor_id
                .as_ref()
                .unwrap_or(&connection.target_floor_id);
            let target_depth = if target_floor_id == &world.initial_floor_id {
                Some(0)
            } else {
                world
                    .procedural_floors
                    .iter()
                    .find(|floor| {
                        floor.id == *target_floor_id
                            && floor.lifecycle == FloorLifecycle::Dungeon
                            && floor.dungeon_id.as_deref() == current_dungeon_id
                    })
                    .map(|floor| floor.depth)
            };
            let Some(target_depth) = target_depth else {
                continue;
            };
            let target = FloorTransitionTarget {
                floor_id: target_floor_id.clone(),
                arrival_connection_id: state
                    .target_connection_id
                    .clone()
                    .or_else(|| connection.target_connection_id.clone()),
                departure_connection_id: Some(state.id.clone()),
            };
            if target_depth < current.depth {
                upward.push(target);
            } else if target_depth > current.depth {
                downward.push(target);
            }
        }
        if upward.is_empty() {
            upward.push(FloorTransitionTarget {
                floor_id: current.return_floor_id.clone(),
                arrival_connection_id: None,
                departure_connection_id: None,
            });
        }
        if downward.is_empty()
            && let Some(next_floor_id) = &current.next_floor_id
        {
            downward.push(FloorTransitionTarget {
                floor_id: next_floor_id.clone(),
                arrival_connection_id: None,
                departure_connection_id: None,
            });
        }
        upward.sort();
        upward.dedup();
        downward.sort();
        downward.dedup();
        (upward, downward)
    }

    pub(super) fn recall_use_plan(&self) -> Option<RecallUseAction> {
        let recall = self.recall.as_ref()?;
        if recall.remaining_turns.is_some() {
            return Some(RecallUseAction::Cancel);
        }
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        if self.current_floor_id == world.initial_floor_id {
            let dungeon = world
                .dungeons
                .iter()
                .find(|dungeon| dungeon.id == recall.dungeon_id)?;
            return self
                .dungeon_entry_requirements_met(dungeon)
                .then_some(RecallUseAction::Start);
        }
        floor_dungeon_id(world, &self.current_floor_id).map(|_| RecallUseAction::Start)
    }

    pub(super) fn recall_reset_plan(&self) -> Option<RecallDestination> {
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        (self
            .recall
            .as_ref()
            .is_none_or(|recall| recall.remaining_turns.is_none()))
        .then(|| floor_dungeon_id(world, &self.current_floor_id))
        .flatten()
        .map(|dungeon_id| RecallDestination {
            dungeon_id,
            floor_id: self.current_floor_id.clone(),
        })
    }

    pub(super) fn start_recall(&mut self, delay: u16) -> RecallDestination {
        let recall = self
            .recall
            .as_mut()
            .expect("planned recall must retain its destination");
        recall.remaining_turns = Some(delay.saturating_add(1));
        RecallDestination {
            dungeon_id: recall.dungeon_id.clone(),
            floor_id: recall.floor_id.clone(),
        }
    }

    pub(super) fn cancel_recall(&mut self) {
        self.recall
            .as_mut()
            .expect("planned recall cancellation must retain its destination")
            .remaining_turns = None;
    }

    pub(super) fn reset_recall(&mut self, destination: RecallDestination) {
        self.recall = Some(RecallStateDto {
            dungeon_id: destination.dungeon_id,
            floor_id: destination.floor_id,
            remaining_turns: None,
        });
    }

    fn plan_floor_transition(
        &self,
        target: FloorTransitionTarget,
        abandon_task: bool,
    ) -> Result<Option<FloorTransitionPlan>, CoreError> {
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        let initial_floor_id = &world.initial_floor_id;

        let target_definition = world
            .procedural_floors
            .iter()
            .find(|floor| floor.id == target.floor_id);
        if self.current_floor_id == *initial_floor_id
            && let Some(target_floor) =
                target_definition.filter(|floor| floor.lifecycle == FloorLifecycle::Dungeon)
        {
            let dungeon = world
                .dungeons
                .iter()
                .find(|dungeon| target_floor.dungeon_id.as_deref() == Some(dungeon.id.as_str()))
                .expect("validated dungeon floor must retain its dungeon definition");
            if !self.dungeon_entry_requirements_met(dungeon) {
                return Ok(None);
            }
        }

        if let Some(target_floor) =
            target_definition.filter(|floor| floor.lifecycle == FloorLifecycle::OneShot)
        {
            let task_id = floor_task_id(target_floor);
            let state = self
                .task_states
                .get(task_id)
                .expect("target task state must remain available");
            if state.status == TaskStatusKindDto::Paused
                && target_floor
                    .max_retakes
                    .is_some_and(|maximum| state.retakes_used >= maximum)
            {
                return Ok(None);
            }
            let required_floor_id = task_objectives(world, task_id)
                .get(usize::try_from(state.stage_index).unwrap_or(usize::MAX))
                .and_then(|objective| objective.floor_id.as_deref());
            if required_floor_id.is_some_and(|floor_id| floor_id != target.floor_id) {
                return Ok(None);
            }
        }

        let from_floor_id = self.current_floor_id.clone();
        let from_dungeon_instance_id = self.current_dungeon_instance_id.clone();
        let source_definition = world
            .procedural_floors
            .iter()
            .find(|floor| floor.id == from_floor_id);
        let mut retained_instance_action = RetainedInstanceAction::None;
        let mut allocated_dungeon_instance = None;
        let target_dungeon_instance_id = if let Some(target_floor) =
            target_definition.filter(|floor| floor.lifecycle == FloorLifecycle::Dungeon)
        {
            let dungeon_id = target_floor
                .dungeon_id
                .as_deref()
                .expect("dungeon floor must retain a dungeon ID");
            let lifecycle = &world
                .dungeons
                .iter()
                .find(|dungeon| dungeon.id == dungeon_id)
                .expect("validated dungeon floor must retain its definition")
                .instance_lifecycle;
            if source_definition
                .is_some_and(|source| source.dungeon_id.as_deref() == Some(dungeon_id))
            {
                from_dungeon_instance_id.clone()
            } else if from_floor_id == *initial_floor_id {
                let (action, retained_instance_id) = plan_retained_instance_action(
                    dungeon_id,
                    lifecycle,
                    self.dungeon_states.get(dungeon_id),
                    &self.stored_floors,
                    self.turn,
                )?;
                retained_instance_action = action;
                if let Some(instance_id) = retained_instance_id {
                    Some(instance_id)
                } else {
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
                }
            } else {
                return Err(CoreError::InvalidSave(
                    "cross-dungeon floor transition is invalid",
                ));
            }
        } else {
            None
        };

        let one_shot_source = source_definition.filter(|floor| {
            target.floor_id == *initial_floor_id && floor.lifecycle == FloorLifecycle::OneShot
        });
        let one_shot_departure = if let Some(source) = one_shot_source {
            let task_id = floor_task_id(source).to_owned();
            let members = world
                .procedural_floors
                .iter()
                .filter(|floor| {
                    floor.lifecycle == FloorLifecycle::OneShot && floor_task_id(floor) == task_id
                })
                .cloned()
                .collect::<Vec<_>>();
            let succeeded = self
                .task_states
                .get(&task_id)
                .is_some_and(|state| task_succeeded(world, &task_id, state));
            if !abandon_task && !source.retakeable && !source.allow_early_task_exit && !succeeded {
                return Ok(None);
            }
            let resolution =
                task_resolution_for_departure(Some(source.retakeable), abandon_task, succeeded);
            let initial_required = initial_task_states(world)[&task_id].required;
            Some(OneShotDeparturePlan {
                task_id,
                members,
                resolution,
                initial_required,
                retakeable: source.retakeable,
            })
        } else {
            None
        };

        let one_shot_arrival = target_definition
            .filter(|floor| floor.lifecycle == FloorLifecycle::OneShot)
            .map(|target_floor| {
                let task_id = floor_task_id(target_floor).to_owned();
                let resumed = target_floor.retakeable
                    && self
                        .task_states
                        .get(&task_id)
                        .is_some_and(|state| state.status == TaskStatusKindDto::Paused);
                let regenerate_members = if resumed
                    && target_floor.retake_floor_policy == RetakeFloorPolicy::RegenerateFloor
                {
                    world
                        .procedural_floors
                        .iter()
                        .filter(|floor| {
                            floor.lifecycle == FloorLifecycle::OneShot
                                && floor_task_id(floor) == task_id
                        })
                        .cloned()
                        .collect()
                } else {
                    Vec::new()
                };
                OneShotArrivalPlan {
                    task_id,
                    floor_id: target_floor.id.clone(),
                    resumed,
                    regenerate_members,
                }
            });

        let from_storage_key =
            dungeon_instance_storage_key(from_dungeon_instance_id.as_deref(), &from_floor_id);
        let target_storage_key =
            dungeon_instance_storage_key(target_dungeon_instance_id.as_deref(), &target.floor_id);
        if !self.stored_floors.contains_key(&target_storage_key) && target_definition.is_none() {
            return Err(CoreError::InvalidSave("return floor state is missing"));
        }

        let expedition_end = if target.floor_id == *initial_floor_id
            && source_definition.is_some_and(|floor| floor.lifecycle == FloorLifecycle::Dungeon)
        {
            let instance_id = from_dungeon_instance_id
                .clone()
                .ok_or(CoreError::InvalidSave(
                    "active dungeon floor is missing its instance ID",
                ))?;
            let dungeon_id = source_definition
                .and_then(|floor| floor.dungeon_id.as_deref())
                .ok_or(CoreError::InvalidSave(
                    "active dungeon floor is missing its dungeon ID",
                ))?;
            let lifecycle = &world
                .dungeons
                .iter()
                .find(|dungeon| dungeon.id == dungeon_id)
                .expect("validated dungeon floor must retain its definition")
                .instance_lifecycle;
            Some(if lifecycle == &DungeonInstanceLifecycle::ResetOnSurface {
                ExpeditionEndAction::Reset { instance_id }
            } else {
                ExpeditionEndAction::Retain {
                    dungeon_id: dungeon_id.to_owned(),
                    instance_id,
                    retained_at_turn: self.turn.saturating_add(1),
                }
            })
        } else {
            None
        };

        let mut following_summon_ids = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && self.actor_is_player_aligned(entity)
                    && chebyshev_distance(entity.position, self.player.position) <= 2
            })
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        following_summon_ids.sort();

        Ok(Some(FloorTransitionPlan {
            from_floor_id,
            from_dungeon_instance_id,
            from_storage_key,
            target_floor_id: target.floor_id,
            target_dungeon_instance_id,
            target_storage_key,
            target_definition: target_definition.cloned(),
            arrival_connection_id: target.arrival_connection_id,
            departure_connection_id: target.departure_connection_id,
            retained_instance_action,
            allocated_dungeon_instance,
            expedition_end,
            one_shot_departure,
            one_shot_arrival,
            following_summon_ids,
        }))
    }

    fn apply_retained_instance_action(&mut self, action: &RetainedInstanceAction) {
        match action {
            RetainedInstanceAction::None => {}
            RetainedInstanceAction::Take { dungeon_id } => {
                let state = self
                    .dungeon_states
                    .get_mut(dungeon_id)
                    .expect("target dungeon state must remain available");
                state.retained_instance_id = None;
                state.retained_at_turn = None;
            }
            RetainedInstanceAction::Expire {
                dungeon_id,
                instance_id,
            } => {
                self.discard_stored_dungeon_instance(instance_id);
                let state = self
                    .dungeon_states
                    .get_mut(dungeon_id)
                    .expect("target dungeon state must remain available");
                state.retained_instance_id = None;
                state.retained_at_turn = None;
            }
        }
    }

    pub(super) fn traverse_stairs(
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
        let Some(target) = stair_transition_target(
            world,
            &self.current_floor_id,
            &terrain_id,
            terrain,
            &self.floor_connections,
            self.player.position,
            abandon_task,
        )?
        else {
            return Ok(None);
        };
        self.transition_floor(
            target.floor_id,
            target.arrival_connection_id,
            target.departure_connection_id,
            abandon_task,
        )
    }

    pub(super) fn transition_floor(
        &mut self,
        target_floor_id: String,
        arrival_connection_id: Option<String>,
        departure_connection_id: Option<String>,
        abandon_task: bool,
    ) -> Result<Option<FloorTransitionOutcome>, CoreError> {
        let Some(plan) = self.plan_floor_transition(
            FloorTransitionTarget {
                floor_id: target_floor_id,
                arrival_connection_id,
                departure_connection_id,
            },
            abandon_task,
        )?
        else {
            return Ok(None);
        };
        self.commit_floor_transition(plan).map(Some)
    }

    fn commit_floor_transition(
        &mut self,
        plan: FloorTransitionPlan,
    ) -> Result<FloorTransitionOutcome, CoreError> {
        self.apply_retained_instance_action(&plan.retained_instance_action);

        let following_summon_ids = plan
            .following_summon_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut following_summons = Vec::with_capacity(following_summon_ids.len());
        let mut remaining_entities = Vec::with_capacity(self.entities.len());
        for entity in std::mem::take(&mut self.entities) {
            if following_summon_ids.contains(&entity.id) {
                following_summons.push(entity);
            } else {
                remaining_entities.push(entity);
            }
        }
        following_summons.sort_by(|left, right| left.id.cmp(&right.id));
        self.entities = remaining_entities;

        let all_items = std::mem::take(&mut self.items);
        let (floor_items, global_items): (Vec<_>, Vec<_>) =
            all_items.into_iter().partition(|item| {
                matches!(
                    item.location,
                    ItemLocation::Ground(_) | ItemLocation::CarriedBy { .. }
                )
            });
        let current = FloorState {
            id: plan.from_floor_id.clone(),
            dungeon_instance_id: plan.from_dungeon_instance_id.clone(),
            width: self.width,
            height: self.height,
            terrain: std::mem::take(&mut self.terrain),
            player_position: self.player.position,
            entities: std::mem::take(&mut self.entities),
            items: floor_items,
            gold_piles: std::mem::take(&mut self.gold_piles),
            explored: std::mem::take(&mut self.explored),
            revealed_terrain: std::mem::take(&mut self.revealed_terrain),
            connections: std::mem::take(&mut self.floor_connections),
            regions: std::mem::take(&mut self.floor_regions),
        };
        self.stored_floors
            .insert(plan.from_storage_key.clone(), current);

        if let Some(arrival) = &plan.one_shot_arrival
            && !arrival.regenerate_members.is_empty()
        {
            self.discard_stored_task_floors(&arrival.regenerate_members);
        }

        let mut destination_was_generated = false;
        let mut destination =
            if let Some(floor) = self.stored_floors.remove(&plan.target_storage_key) {
                floor
            } else if let Some(definition) = &plan.target_definition {
                destination_was_generated = true;
                self.generate_procedural_floor(definition, plan.target_dungeon_instance_id.clone())?
            } else {
                unreachable!("planned destination must remain available")
            };
        if destination_was_generated
            && let (Some(arrival_connection_id), Some(departure_connection_id)) = (
                plan.arrival_connection_id.as_ref(),
                plan.departure_connection_id.as_ref(),
            )
            && let Some(connection) = destination
                .connections
                .iter_mut()
                .find(|connection| connection.id == *arrival_connection_id)
        {
            connection.target_floor_id = Some(plan.from_floor_id.clone());
            connection.target_connection_id = Some(departure_connection_id.clone());
        }
        if let Some(arrival_connection_id) = &plan.arrival_connection_id {
            if let Some(connection) = destination
                .connections
                .iter()
                .find(|connection| connection.id == *arrival_connection_id)
            {
                destination.player_position = connection.position;
            } else if !destination.connections.is_empty() {
                return Err(CoreError::InvalidSave(
                    "destination floor connection is missing",
                ));
            }
        }

        if let Some((dungeon_id, ordinal)) = &plan.allocated_dungeon_instance {
            self.dungeon_states
                .get_mut(dungeon_id)
                .expect("target dungeon state must remain available")
                .next_instance_ordinal = *ordinal;
        }
        if let Some(expedition_end) = &plan.expedition_end {
            match expedition_end {
                ExpeditionEndAction::Reset { instance_id } => {
                    self.discard_stored_dungeon_instance(instance_id);
                }
                ExpeditionEndAction::Retain {
                    dungeon_id,
                    instance_id,
                    retained_at_turn,
                } => {
                    let state = self
                        .dungeon_states
                        .get_mut(dungeon_id)
                        .expect("active dungeon state must remain available");
                    state.retained_instance_id = Some(instance_id.clone());
                    state.retained_at_turn = Some(*retained_at_turn);
                }
            }
        }

        if let Some(departure) = &plan.one_shot_departure
            && let Some(resolution) = departure.resolution
        {
            self.discard_stored_task_floors(&departure.members);
            for definition in &departure.members {
                if let (Some(entry_id), Some(result_id)) = (
                    definition.entry_terrain_id.as_deref(),
                    match resolution {
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
            if resolution == TaskResolution::Completed
                && let Some(reward) = departure
                    .members
                    .iter()
                    .find_map(|definition| definition.task_reward.as_ref())
            {
                let (activation, charges) = initial_item_runtime_state(
                    &self.content,
                    &mut self.rng,
                    &reward.item_kind_id,
                    1,
                );
                destination.items.push(ItemInstance {
                    id: reward.item_instance_id.clone(),
                    kind_id: reward.item_kind_id.clone(),
                    quantity: reward.quantity,
                    quality: ItemQualityDto::Ordinary,
                    affix_ids: Vec::new(),
                    rolled_affixes: Vec::new(),
                    enchantments: ItemEnchantmentsDto::default(),
                    curse: initial_item_curse(&self.content, &reward.item_kind_id),
                    activation,
                    charges,
                    fuel: initial_item_fuel(&self.content, &reward.item_kind_id),
                    device_recovery_progress: 0,
                    location: ItemLocation::Ground(destination.player_position),
                });
            }
        }
        if let Some(departure) = &plan.one_shot_departure {
            let state = self
                .task_states
                .get_mut(&departure.task_id)
                .expect("active task state must remain available");
            *state =
                task_state_after_departure(state, departure.resolution, departure.initial_required);
        }
        if let Some(arrival) = &plan.one_shot_arrival {
            let state = self
                .task_states
                .get_mut(&arrival.task_id)
                .expect("target task state must remain available");
            *state = activated_task_state(state, &arrival.floor_id, arrival.resumed);
        }

        self.activate_floor(destination, global_items);
        let (summons_followed, summons_could_not_follow) =
            self.place_following_summons(following_summons, &plan.from_storage_key);
        if self.summon_command.mode == SummonCommandModeDto::Guard {
            self.summon_command.guard_position = Some(self.player.position);
        }

        Ok(FloorTransitionOutcome {
            from_floor_id: plan.from_floor_id,
            to_floor_id: plan.target_floor_id,
            expedition_ended: plan.expedition_end.is_some(),
            one_shot_closed: plan.one_shot_departure.as_ref().and_then(|departure| {
                departure
                    .resolution
                    .map(|resolution| (departure.task_id.clone(), resolution))
            }),
            task_paused: plan.one_shot_departure.as_ref().and_then(|departure| {
                (departure.resolution.is_none() && departure.retakeable)
                    .then(|| departure.task_id.clone())
            }),
            task_resumed: plan
                .one_shot_arrival
                .filter(|arrival| arrival.resumed)
                .map(|arrival| arrival.task_id),
            summons_followed,
            summons_could_not_follow,
        })
    }

    pub(super) fn record_floor_transition(
        &self,
        transition: FloorTransitionOutcome,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
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
        for (entity_id, target_kind_id) in transition.summons_followed {
            events.push(DomainEvent::SummonFollowedFloor {
                entity_id,
                target_kind_id,
            });
        }
        for (entity_id, target_kind_id) in transition.summons_could_not_follow {
            events.push(DomainEvent::SummonCouldNotFollow {
                entity_id,
                target_kind_id,
            });
        }
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
                                .filter(|member| floor_task_id(member) == floor_task_id(floor))
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
    }

    pub(super) fn discard_stored_task_floors(&mut self, members: &[ProceduralFloorDefinition]) {
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

    fn place_following_summons(
        &mut self,
        following_summons: Vec<Actor>,
        from_storage_key: &str,
    ) -> (Vec<ActorIdentity>, Vec<ActorIdentity>) {
        if following_summons.is_empty() {
            return (Vec::new(), Vec::new());
        }
        let positions = self.open_positions_around(self.player.position, 5);
        if positions.len() < following_summons.len() {
            let summaries = following_summons
                .iter()
                .map(|entity| (entity.id.clone(), entity.kind_id.clone()))
                .collect::<Vec<_>>();
            if let Some(source) = self.stored_floors.get_mut(from_storage_key) {
                source.entities.extend(following_summons);
                source
                    .entities
                    .sort_by(|left, right| left.id.cmp(&right.id));
                return (Vec::new(), summaries);
            }
            return (Vec::new(), summaries);
        }
        let mut followed = Vec::with_capacity(following_summons.len());
        for (mut entity, position) in following_summons.into_iter().zip(positions) {
            entity.position = position;
            followed.push((entity.id.clone(), entity.kind_id.clone()));
            self.entities.push(entity);
        }
        self.entities.sort_by(|left, right| left.id.cmp(&right.id));
        (followed, Vec::new())
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
        self.gold_piles = floor.gold_piles;
        self.explored = floor.explored;
        self.revealed_terrain = floor.revealed_terrain;
        self.floor_connections = floor.connections;
        self.floor_regions = floor.regions;
        self.mark_current_town_visited();
        self.mark_shop_visited_at_player();
        self.update_recall_destination_for_current_floor();
        self.reveal_current_visibility();
    }

    pub(super) fn update_recall_destination_for_current_floor(&mut self) {
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        if let Some(recall) = recall_destination_for_current_floor(
            world,
            &self.current_floor_id,
            self.recall.as_ref(),
        ) {
            self.recall = Some(recall);
        }
    }

    fn recall_advance_plan(&self) -> Option<RecallAdvancePlan> {
        let remaining_turns = self
            .recall
            .as_ref()
            .and_then(|recall| recall.remaining_turns)?;
        if remaining_turns > 1 {
            return Some(RecallAdvancePlan::Countdown(remaining_turns - 1));
        }
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        Some(RecallAdvancePlan::Trigger {
            from_floor_id: self.current_floor_id.clone(),
            target_floor_id: if self.current_floor_id == world.initial_floor_id {
                self.recall
                    .as_ref()
                    .expect("pending recall must retain its destination")
                    .floor_id
                    .clone()
            } else {
                world.initial_floor_id.clone()
            },
        })
    }

    pub(super) fn advance_recall(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Result<(), CoreError> {
        let Some(plan) = self.recall_advance_plan() else {
            return Ok(());
        };
        match plan {
            RecallAdvancePlan::Countdown(remaining_turns) => {
                self.recall
                    .as_mut()
                    .expect("pending recall must retain its destination")
                    .remaining_turns = Some(remaining_turns);
            }
            RecallAdvancePlan::Trigger {
                from_floor_id,
                target_floor_id,
            } => {
                self.recall
                    .as_mut()
                    .expect("pending recall must retain its destination")
                    .remaining_turns = None;
                let Some(transition) = self.transition_floor(target_floor_id, None, None, false)?
                else {
                    return Ok(());
                };
                events.push(DomainEvent::RecallTriggered {
                    from_floor_id,
                    to_floor_id: transition.to_floor_id.clone(),
                });
                self.record_floor_transition(transition, events, changed);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dungeon_instance_identity_helpers_round_trip_positive_ordinals() {
        let dungeon_id = "demo.dungeon.echo-depths";
        let instance_id = dungeon_instance_id(dungeon_id, 7);

        assert_eq!(instance_id, "demo.dungeon.echo-depths.instance.7");
        assert_eq!(
            parse_dungeon_instance_ordinal(&instance_id, dungeon_id),
            Some(7)
        );
        assert_eq!(parse_dungeon_instance_ordinal(&instance_id, "other"), None);
        assert_eq!(
            parse_dungeon_instance_ordinal("demo.dungeon.echo-depths.instance.0", dungeon_id),
            None
        );
        assert_eq!(
            dungeon_instance_storage_key(Some(&instance_id), "demo.floor.echo-depth-1"),
            "demo.dungeon.echo-depths.instance.7::demo.floor.echo-depth-1"
        );
    }

    #[test]
    fn transition_preflight_records_identity_without_mutating_the_game() {
        let game = Game::new(27);
        let before = game.to_save();
        let draws = game.rng.draw_counter;

        let plan = game
            .plan_floor_transition(
                FloorTransitionTarget {
                    floor_id: "demo.floor.echo-depth-1".to_owned(),
                    arrival_connection_id: None,
                    departure_connection_id: None,
                },
                false,
            )
            .expect("valid dungeon entry should plan")
            .expect("valid dungeon entry should be available");

        assert_eq!(plan.from_floor_id, "demo.floor.surface");
        assert_eq!(plan.from_storage_key, "demo.floor.surface");
        assert_eq!(
            plan.target_dungeon_instance_id.as_deref(),
            Some("demo.dungeon.echo-depths.instance.1")
        );
        assert_eq!(
            plan.target_storage_key,
            "demo.dungeon.echo-depths.instance.1::demo.floor.echo-depth-1"
        );
        assert!(matches!(
            plan.allocated_dungeon_instance,
            Some((ref dungeon_id, 1)) if dungeon_id == "demo.dungeon.echo-depths"
        ));
        assert_eq!(game.to_save(), before);
        assert_eq!(game.rng.draw_counter, draws);
        assert!(game.stored_floors.is_empty());
        assert_eq!(
            game.dungeon_states["demo.dungeon.echo-depths"].next_instance_ordinal,
            0
        );
    }

    #[test]
    fn unavailable_task_preflight_is_zero_mutation() {
        let mut game = Game::new(31);
        let task = game
            .task_states
            .get_mut("demo.task.echo-bounty")
            .expect("bounty task state should exist");
        task.status = TaskStatusKindDto::Paused;
        task.retakes_used = 1;
        let before = game.to_save();
        let draws = game.rng.draw_counter;

        let plan = game
            .plan_floor_transition(
                FloorTransitionTarget {
                    floor_id: "demo.floor.echo-bounty-rift".to_owned(),
                    arrival_connection_id: None,
                    departure_connection_id: None,
                },
                false,
            )
            .expect("retake limit should be normal unavailability");

        assert!(plan.is_none());
        assert_eq!(game.to_save(), before);
        assert_eq!(game.rng.draw_counter, draws);
    }

    #[test]
    fn recall_plans_preserve_delay_offset_and_trigger_target() {
        let mut game = Game::new(2);
        game.recall = Some(RecallStateDto {
            dungeon_id: "demo.dungeon.echo-depths".to_owned(),
            floor_id: "demo.floor.echo-depth-2".to_owned(),
            remaining_turns: None,
        });

        assert_eq!(game.recall_use_plan(), Some(RecallUseAction::Start));
        let destination = game.start_recall(3);
        assert_eq!(destination.floor_id, "demo.floor.echo-depth-2");
        assert_eq!(game.recall.as_ref().unwrap().remaining_turns, Some(4));
        assert_eq!(
            game.recall_advance_plan(),
            Some(RecallAdvancePlan::Countdown(3))
        );

        game.recall.as_mut().unwrap().remaining_turns = Some(1);
        assert_eq!(
            game.recall_advance_plan(),
            Some(RecallAdvancePlan::Trigger {
                from_floor_id: "demo.floor.surface".to_owned(),
                target_floor_id: "demo.floor.echo-depth-2".to_owned(),
            })
        );
    }
}
