// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeMap;

use rfb_content::{
    CampaignDefinition, FloorLifecycle, ProceduralFloorDefinition, TaskObjectiveDefinition,
    TaskObjectiveKind, WorldDefinition,
};
use rfb_protocol::{CampaignStatusDto, TaskStatusKindDto};

use crate::{
    error::CoreError,
    event::DomainEvent,
    state::{Actor, ItemInstance, ItemLocation},
    stats::CharacterProgress,
};

use super::{DungeonState, Game};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TaskState {
    pub(super) status: TaskStatusKindDto,
    pub(super) stage_index: u32,
    pub(super) current: u32,
    pub(super) required: u32,
    pub(super) active_floor_id: Option<String>,
    pub(super) retakes_used: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CampaignState {
    pub(super) status: CampaignStatusDto,
    pub(super) victory_turn: Option<u32>,
    pub(super) retired_turn: Option<u32>,
    pub(super) final_score: Option<u64>,
}

impl Default for CampaignState {
    fn default() -> Self {
        Self {
            status: CampaignStatusDto::Active,
            victory_turn: None,
            retired_turn: None,
            final_score: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum TaskResolution {
    Completed,
    Failed,
    Abandoned,
}

struct TaskProgressPlan {
    task_id: String,
    state: TaskState,
}

struct CampaignTransitionPlan {
    state: CampaignState,
    score: u64,
}

pub(super) fn floor_task_id(floor: &ProceduralFloorDefinition) -> &str {
    floor.task_id.as_deref().unwrap_or(&floor.id)
}

pub(super) fn task_objectives<'a>(
    world: &'a WorldDefinition,
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

pub(super) fn task_succeeded(world: &WorldDefinition, task_id: &str, state: &TaskState) -> bool {
    let objectives = task_objectives(world, task_id);
    usize::try_from(state.stage_index)
        .ok()
        .is_some_and(|stage| stage + 1 == objectives.len())
        && state.current >= state.required
}

pub(super) fn initial_task_states(world: &WorldDefinition) -> BTreeMap<String, TaskState> {
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

pub(super) fn task_resolution_for_departure(
    source_retakeable: Option<bool>,
    abandon_task: bool,
    succeeded: bool,
) -> Option<TaskResolution> {
    if source_retakeable.is_none() {
        None
    } else if abandon_task {
        Some(TaskResolution::Abandoned)
    } else if succeeded {
        Some(TaskResolution::Completed)
    } else if source_retakeable == Some(true) {
        None
    } else {
        Some(TaskResolution::Failed)
    }
}

pub(super) fn task_state_after_departure(
    state: &TaskState,
    resolution: Option<TaskResolution>,
    initial_required: u32,
) -> TaskState {
    let mut planned = state.clone();
    planned.active_floor_id = None;
    planned.status = match resolution {
        Some(TaskResolution::Completed) => {
            planned.current = planned.required;
            TaskStatusKindDto::Completed
        }
        Some(TaskResolution::Failed) => {
            planned.stage_index = 0;
            planned.current = 0;
            planned.required = initial_required;
            TaskStatusKindDto::Failed
        }
        Some(TaskResolution::Abandoned) => {
            planned.stage_index = 0;
            planned.current = 0;
            planned.required = initial_required;
            TaskStatusKindDto::Abandoned
        }
        None => TaskStatusKindDto::Paused,
    };
    planned
}

pub(super) fn activated_task_state(state: &TaskState, floor_id: &str, resumed: bool) -> TaskState {
    let mut planned = state.clone();
    if resumed {
        planned.retakes_used = planned.retakes_used.saturating_add(1);
    }
    planned.status = TaskStatusKindDto::Active;
    planned.active_floor_id = Some(floor_id.to_owned());
    planned
}

pub(super) fn abandoned_task_state(state: &TaskState, initial_required: u32) -> TaskState {
    let mut planned = state.clone();
    planned.status = TaskStatusKindDto::Abandoned;
    planned.stage_index = 0;
    planned.current = 0;
    planned.required = initial_required;
    planned.active_floor_id = None;
    planned
}

fn active_task_objective(
    world: &WorldDefinition,
    task_states: &BTreeMap<String, TaskState>,
    current_floor_id: &str,
) -> Result<Option<(String, TaskObjectiveDefinition, Option<u32>)>, CoreError> {
    let Some((task_id, stage_index)) = task_states.iter().find_map(|(task_id, state)| {
        (state.status == TaskStatusKindDto::Active
            && state.active_floor_id.as_deref() == Some(current_floor_id))
        .then_some((task_id, state.stage_index))
    }) else {
        return Ok(None);
    };
    let objectives = task_objectives(world, task_id);
    let stage = usize::try_from(stage_index).ok();
    let objective = stage
        .and_then(|stage| objectives.get(stage))
        .copied()
        .cloned()
        .ok_or_else(|| {
            CoreError::Invariant(format!(
                "active task {task_id} references missing objective stage {stage_index}"
            ))
        })?;
    let next_required = stage
        .and_then(|stage| stage.checked_add(1))
        .and_then(|stage| objectives.get(stage))
        .map(|objective| objective.required);
    Ok(Some((task_id.clone(), objective, next_required)))
}

fn task_death_target_kind(event: &DomainEvent) -> Option<&str> {
    match event {
        DomainEvent::PlayerSlew { target_kind_id, .. }
        | DomainEvent::SummonSlew { target_kind_id, .. }
        | DomainEvent::ProjectileSlew { target_kind_id, .. }
        | DomainEvent::ItemThrowSlew { target_kind_id, .. }
        | DomainEvent::VengeanceSlew { target_kind_id, .. }
        | DomainEvent::EntityDiedFromStatus { target_kind_id, .. } => Some(target_kind_id.as_str()),
        _ => None,
    }
}

fn plan_task_event_reduction(
    world: &WorldDefinition,
    task_states: &BTreeMap<String, TaskState>,
    current_floor_id: &str,
    items: &[ItemInstance],
    entities: &[Actor],
    events: &[DomainEvent],
) -> Result<Option<TaskProgressPlan>, CoreError> {
    let Some((task_id, objective, next_required)) =
        active_task_objective(world, task_states, current_floor_id)?
    else {
        return Ok(None);
    };
    let increment = match objective.kind {
        TaskObjectiveKind::CollectItem => events.iter().any(|event| {
            matches!(event, DomainEvent::ItemPickedUp { .. })
                && objective.item_instance_id.as_ref().is_some_and(|id| {
                    items.iter().any(|item| {
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
                    .is_some_and(|id| !entities.iter().any(|entity| &entity.id == id))
        }) as u32,
        TaskObjectiveKind::KillActorKind => events
            .iter()
            .filter(|event| task_death_target_kind(event) == objective.actor_kind_id.as_deref())
            .count()
            .try_into()
            .unwrap_or(u32::MAX),
    };
    if increment == 0 {
        return Ok(None);
    }
    let mut state = task_states
        .get(&task_id)
        .cloned()
        .expect("active task state must remain available");
    state.current = state.current.saturating_add(increment).min(state.required);
    if state.current >= state.required
        && let Some(next_required) = next_required
    {
        state.stage_index = state.stage_index.saturating_add(1);
        state.current = 0;
        state.required = next_required;
    }
    Ok(Some(TaskProgressPlan { task_id, state }))
}

fn campaign_victory_reached(
    campaign: Option<&CampaignDefinition>,
    dungeon_states: &BTreeMap<String, DungeonState>,
) -> bool {
    campaign.is_some_and(|campaign| {
        campaign.victory_dungeon_ids.iter().all(|dungeon_id| {
            dungeon_states
                .get(dungeon_id)
                .is_some_and(|state| state.guardian_defeated)
        })
    })
}

fn campaign_counts(
    dungeon_states: &BTreeMap<String, DungeonState>,
    task_states: &BTreeMap<String, TaskState>,
) -> (u32, u32) {
    let conquered = dungeon_states
        .values()
        .filter(|state| state.guardian_defeated)
        .count()
        .try_into()
        .unwrap_or(u32::MAX);
    let completed = task_states
        .values()
        .filter(|state| state.status == TaskStatusKindDto::Completed)
        .count()
        .try_into()
        .unwrap_or(u32::MAX);
    (conquered, completed)
}

fn campaign_score(
    campaign: Option<&CampaignDefinition>,
    state: &CampaignState,
    counts: (u32, u32),
    turn: u32,
) -> u64 {
    let Some(campaign) = campaign else {
        return 0;
    };
    let (conquered, completed) = counts;
    let base = u64::from(conquered)
        .saturating_mul(u64::from(campaign.dungeon_conquest_points))
        .saturating_add(
            u64::from(completed).saturating_mul(u64::from(campaign.task_completion_points)),
        )
        .saturating_add(if state.status != CampaignStatusDto::Active {
            u64::from(campaign.victory_bonus)
        } else {
            0
        });
    let penalty = u64::from(turn / campaign.turn_penalty_interval)
        .saturating_mul(u64::from(campaign.turn_penalty_points));
    base.saturating_sub(penalty)
}

fn plan_campaign_victory(
    campaign: Option<&CampaignDefinition>,
    state: &CampaignState,
    dungeon_states: &BTreeMap<String, DungeonState>,
    task_states: &BTreeMap<String, TaskState>,
    victory_turn: u32,
) -> Option<CampaignTransitionPlan> {
    if state.status != CampaignStatusDto::Active
        || !campaign_victory_reached(campaign, dungeon_states)
    {
        return None;
    }
    let mut state = *state;
    state.status = CampaignStatusDto::Victorious;
    state.victory_turn = Some(victory_turn);
    let score = campaign_score(
        campaign,
        &state,
        campaign_counts(dungeon_states, task_states),
        victory_turn,
    );
    Some(CampaignTransitionPlan { state, score })
}

fn plan_campaign_retirement(
    campaign: Option<&CampaignDefinition>,
    state: &CampaignState,
    dungeon_states: &BTreeMap<String, DungeonState>,
    task_states: &BTreeMap<String, TaskState>,
    on_surface: bool,
    retired_turn: u32,
) -> Option<CampaignTransitionPlan> {
    if state.status != CampaignStatusDto::Victorious || !on_surface {
        return None;
    }
    let score = campaign_score(
        campaign,
        state,
        campaign_counts(dungeon_states, task_states),
        retired_turn,
    );
    let mut state = *state;
    state.status = CampaignStatusDto::Retired;
    state.retired_turn = Some(retired_turn);
    state.final_score = Some(score);
    Some(CampaignTransitionPlan { state, score })
}

impl Game {
    pub(super) fn active_task_objective(
        &self,
    ) -> Result<Option<(String, TaskObjectiveDefinition, Option<u32>)>, CoreError> {
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        active_task_objective(world, &self.task_states, &self.current_floor_id)
    }

    pub(super) fn apply_task_events(&mut self, events: &[DomainEvent]) -> Result<(), CoreError> {
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        let Some(plan) = plan_task_event_reduction(
            world,
            &self.task_states,
            &self.current_floor_id,
            &self.items,
            &self.entities,
            events,
        )?
        else {
            return Ok(());
        };
        self.task_states.insert(plan.task_id, plan.state);
        Ok(())
    }

    pub(super) fn campaign_definition(&self) -> Option<&CampaignDefinition> {
        self.content
            .world(&self.world_id)
            .and_then(|world| world.campaign.as_ref())
    }

    pub(super) fn campaign_victory_reached(&self) -> bool {
        campaign_victory_reached(self.campaign_definition(), &self.dungeon_states)
    }

    pub(super) fn campaign_counts(&self) -> (u32, u32) {
        campaign_counts(&self.dungeon_states, &self.task_states)
    }

    pub(super) fn campaign_score_at(&self, turn: u32) -> u64 {
        campaign_score(
            self.campaign_definition(),
            &self.campaign_state,
            self.campaign_counts(),
            turn,
        )
    }

    pub(super) fn apply_campaign_events(&mut self, events: &mut Vec<DomainEvent>) {
        let victory_turn = self.turn.saturating_add(1);
        let Some(plan) = plan_campaign_victory(
            self.campaign_definition(),
            &self.campaign_state,
            &self.dungeon_states,
            &self.task_states,
            victory_turn,
        ) else {
            return;
        };
        self.campaign_state = plan.state;
        events.push(DomainEvent::CampaignVictorious { score: plan.score });
        events.push(DomainEvent::PlayerLevelCapUnlocked {
            level_cap: CharacterProgress::level_cap(true),
            attribute_index_cap: CharacterProgress::attribute_index_cap(true),
        });
        self.apply_player_experience(0, events);
    }

    pub(super) fn retire_campaign(&mut self) -> Option<u64> {
        let on_surface = self.content.world(&self.world_id).is_some_and(|world| {
            self.current_floor_id == world.initial_floor_id
                && self.current_dungeon_instance_id.is_none()
        });
        let retired_turn = self.turn.saturating_add(1);
        let plan = plan_campaign_retirement(
            self.campaign_definition(),
            &self.campaign_state,
            &self.dungeon_states,
            &self.task_states,
            on_surface,
            retired_turn,
        )?;
        self.campaign_state = plan.state;
        Some(plan.score)
    }
}
