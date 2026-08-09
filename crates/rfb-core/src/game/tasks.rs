// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeMap;

use rfb_content::{
    CampaignDefinition, ProceduralFloorDefinition, TaskDefinition, TaskLocationDefinition,
    TaskObjectiveDefinition, TaskObjectiveKind, TaskRewardDefinition, TownFacilityCategory,
    WorldDefinition,
};
use rfb_protocol::{CampaignStatusDto, ItemEnchantmentsDto, ItemQualityDto, TaskStatusKindDto};

use crate::{
    error::CoreError,
    event::DomainEvent,
    save::{initial_item_fuel, position_from_content},
    state::{ItemInstance, ItemLocation},
    stats::CharacterProgress,
};

use super::{
    ActorDeathRecord, DungeonState, Game, initial_item_curse, initial_item_runtime_state,
    inventory::item_instances_stack_compatible,
};

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
    RewardAvailable,
    Failed,
    Abandoned,
}

struct TaskProgressPlan {
    task_id: String,
    state: TaskState,
    completion_origin: Option<rfb_protocol::Position>,
}

struct CampaignTransitionPlan {
    state: CampaignState,
    score: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TaskRewardOutcome {
    pub(super) item_kind_id: String,
    pub(super) quantity: u32,
}

pub(super) fn floor_task_id(floor: &ProceduralFloorDefinition) -> &str {
    floor.task_id.as_deref().unwrap_or(&floor.id)
}

pub(super) fn task_definition<'a>(
    world: &'a WorldDefinition,
    task_id: &str,
) -> Option<&'a TaskDefinition> {
    world.tasks.iter().find(|task| task.id == task_id)
}

fn task_initial_status(
    task: &TaskDefinition,
    states: &BTreeMap<String, TaskState>,
) -> TaskStatusKindDto {
    if task.source_facility_id.is_some()
        && task.prerequisite_task_id.as_ref().is_some_and(|id| {
            !states
                .get(id)
                .is_some_and(|state| state.status == TaskStatusKindDto::Completed)
        })
    {
        TaskStatusKindDto::Locked
    } else {
        TaskStatusKindDto::Available
    }
}

pub(super) fn task_initial_state(
    task: &TaskDefinition,
    states: &BTreeMap<String, TaskState>,
) -> TaskState {
    TaskState {
        status: task_initial_status(task, states),
        stage_index: 0,
        current: 0,
        required: task
            .objectives
            .first()
            .map_or(1, |objective| objective.required),
        active_floor_id: None,
        retakes_used: 0,
    }
}

pub(super) fn projected_task_state(
    world: &WorldDefinition,
    states: &BTreeMap<String, TaskState>,
    task_id: &str,
) -> Option<TaskState> {
    let task = task_definition(world, task_id)?;
    Some(
        states
            .get(task_id)
            .cloned()
            .unwrap_or_else(|| task_initial_state(task, states)),
    )
}

pub(super) fn task_applies_to_floor(
    task: &TaskDefinition,
    floor: &ProceduralFloorDefinition,
) -> bool {
    match &task.location {
        TaskLocationDefinition::DedicatedFloors { floor_ids } => floor_ids.contains(&floor.id),
        TaskLocationDefinition::DungeonDepth { dungeon_id, depth } => {
            floor.dungeon_id.as_deref() == Some(dungeon_id.as_str()) && floor.depth == *depth
        }
    }
}

pub(super) fn task_floors<'a>(
    world: &'a WorldDefinition,
    task_id: &str,
) -> impl Iterator<Item = &'a ProceduralFloorDefinition> {
    let task = task_definition(world, task_id);
    world
        .procedural_floors
        .iter()
        .filter(move |floor| task.is_some_and(|task| task_applies_to_floor(task, floor)))
}

pub(super) fn task_objectives<'a>(
    world: &'a WorldDefinition,
    task_id: &str,
) -> &'a [TaskObjectiveDefinition] {
    task_definition(world, task_id).map_or(&[], |task| task.objectives.as_slice())
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
    for task in &world.tasks {
        if task.source_facility_id.is_none() {
            states.insert(task.id.clone(), task_initial_state(task, &states));
        }
    }
    states
}

pub(super) fn task_resolution_for_departure(
    source_retakeable: Option<bool>,
    abandon_task: bool,
    succeeded: bool,
    reward_claim_required: bool,
) -> Option<TaskResolution> {
    if source_retakeable.is_none() {
        None
    } else if abandon_task {
        Some(TaskResolution::Abandoned)
    } else if succeeded {
        Some(if reward_claim_required {
            TaskResolution::RewardAvailable
        } else {
            TaskResolution::Completed
        })
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
        Some(TaskResolution::RewardAvailable) => TaskStatusKindDto::RewardAvailable,
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
    let active = task_states
        .iter()
        .filter_map(|(task_id, state)| {
            (state.status == TaskStatusKindDto::Active
                && state.active_floor_id.as_deref() == Some(current_floor_id))
            .then_some((task_id, state.stage_index))
        })
        .collect::<Vec<_>>();
    if active.len() > 1 {
        return Err(CoreError::Invariant(format!(
            "multiple active tasks target floor {current_floor_id}"
        )));
    }
    let Some((task_id, stage_index)) = active.into_iter().next() else {
        return Ok(None);
    };
    let objectives = task_objectives(world, task_id);
    let stage = usize::try_from(stage_index).ok();
    let objective = stage
        .and_then(|stage| objectives.get(stage))
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

fn plan_task_event_reduction(
    world: &WorldDefinition,
    task_states: &BTreeMap<String, TaskState>,
    current_floor_id: &str,
    items: &[ItemInstance],
    clear_floor_completed: bool,
    actor_deaths: &[ActorDeathRecord],
    events: &[DomainEvent],
) -> Result<Option<TaskProgressPlan>, CoreError> {
    let Some((task_id, objective, next_required)) =
        active_task_objective(world, task_states, current_floor_id)?
    else {
        return Ok(None);
    };
    let increment = match objective.kind {
        TaskObjectiveKind::ClearFloor => clear_floor_completed as u32,
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
        TaskObjectiveKind::KillActor => actor_deaths.iter().any(|death| {
            death.credit_player
                && objective
                    .actor_instance_id
                    .as_ref()
                    .is_some_and(|id| id == &death.actor_id)
        }) as u32,
        TaskObjectiveKind::KillActorKind => actor_deaths
            .iter()
            .filter(|death| {
                death.credit_player
                    && Some(death.actor_kind_id.as_str()) == objective.actor_kind_id.as_deref()
            })
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
    if state.current >= state.required
        && task_definition(world, &task_id).is_some_and(|task| task.source_facility_id.is_some())
    {
        state.status = TaskStatusKindDto::RewardAvailable;
        state.active_floor_id = None;
    }
    let completion_origin = (state.current >= state.required)
        .then(|| {
            actor_deaths.iter().rev().find_map(|death| {
                let matches_objective = match objective.kind {
                    TaskObjectiveKind::KillActor => objective
                        .actor_instance_id
                        .as_ref()
                        .is_some_and(|id| id == &death.actor_id),
                    TaskObjectiveKind::KillActorKind => objective
                        .actor_kind_id
                        .as_ref()
                        .is_some_and(|id| id == &death.actor_kind_id),
                    TaskObjectiveKind::ClearFloor => true,
                    TaskObjectiveKind::CollectItem | TaskObjectiveKind::EnterFloor => false,
                };
                (death.credit_player && matches_objective).then_some(death.position)
            })
        })
        .flatten();
    Ok(Some(TaskProgressPlan {
        task_id,
        state,
        completion_origin,
    }))
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

fn task_service_accessible(game: &Game, facility_id: &str) -> bool {
    let Some(town) = game.current_town() else {
        return false;
    };
    let Some(facility) = game.content.town_facility(facility_id) else {
        return false;
    };
    facility.category == TownFacilityCategory::QuestGiver
        && facility.town_id == town.id
        && town.facility_ids.contains(&facility.id)
        && game.current_floor_id == town.floor_id
        && game.player.position == position_from_content(facility.entrance_position)
}

fn reward_item(
    game: &Game,
    reward: &TaskRewardDefinition,
    mut preview_rng: crate::rng::RfbRng,
) -> ItemInstance {
    let (activation, charges) =
        initial_item_runtime_state(&game.content, &mut preview_rng, &reward.item_kind_id, 1);
    ItemInstance {
        id: reward.item_instance_id.clone(),
        kind_id: reward.item_kind_id.clone(),
        quantity: reward.quantity,
        inscription: None,
        origin_actor_kind_id: None,
        quality: ItemQualityDto::Ordinary,
        affix_ids: Vec::new(),
        rolled_affixes: Vec::new(),
        enchantments: ItemEnchantmentsDto::default(),
        curse: initial_item_curse(&game.content, &reward.item_kind_id),
        activation,
        charges,
        fuel: initial_item_fuel(&game.content, &reward.item_kind_id),
        device_recovery_progress: 0,
        location: ItemLocation::Inventory,
    }
}

impl Game {
    pub(super) fn accept_task(
        &mut self,
        facility_id: &str,
        task_id: &str,
    ) -> Result<Vec<rfb_protocol::Position>, &'static str> {
        let Some(facility) = self.content.town_facility(facility_id) else {
            return Err("unknown-task-service");
        };
        if !task_service_accessible(self, facility_id) {
            return Err("task-service-unreachable");
        }
        if !facility.task_ids.iter().any(|id| id == task_id) {
            return Err("task-unavailable");
        }
        let Some(world) = self.content.world(&self.world_id) else {
            return Err("task-unavailable");
        };
        let Some(task) = task_definition(world, task_id).cloned() else {
            return Err("task-unavailable");
        };
        if task.source_facility_id.as_deref() != Some(facility_id) {
            return Err("task-source-mismatch");
        }
        let initial = task_initial_state(&task, &self.task_states);
        if initial.status == TaskStatusKindDto::Locked {
            return Err("task-locked");
        }
        if self
            .task_states
            .get(task_id)
            .is_some_and(|state| state.status != TaskStatusKindDto::Available)
        {
            return Err("task-already-taken");
        }
        let mut state = self.task_states.get(task_id).cloned().unwrap_or(initial);
        if state.status != TaskStatusKindDto::Available {
            return Err("task-unavailable");
        }
        let entry_changes = task_floors(world, task_id)
            .filter_map(|floor| {
                Some((
                    floor.available_entry_terrain_id.as_ref()?.clone(),
                    floor.entry_terrain_id.as_ref()?.clone(),
                ))
            })
            .collect::<Vec<_>>();
        let mut changed = Vec::new();
        for (available_terrain_id, _) in &entry_changes {
            let mut positions = self
                .terrain
                .iter()
                .enumerate()
                .filter_map(|(index, terrain_id)| {
                    (terrain_id == available_terrain_id).then_some(rfb_protocol::Position {
                        x: i32::try_from(index % usize::from(self.width)).ok()?,
                        y: i32::try_from(index / usize::from(self.width)).ok()?,
                    })
                })
                .collect::<Vec<_>>();
            if positions.len() != 1 {
                return Err("task-entry-unavailable");
            }
            changed.append(&mut positions);
        }
        state.status = TaskStatusKindDto::Taken;
        self.task_states.insert(task_id.to_owned(), state);
        for ((_, entry_terrain_id), position) in entry_changes.iter().zip(&changed) {
            let index = usize::try_from(position.y).expect("task entry y must be non-negative")
                * usize::from(self.width)
                + usize::try_from(position.x).expect("task entry x must be non-negative");
            self.terrain[index] = entry_terrain_id.clone();
        }
        Ok(changed)
    }

    pub(super) fn claim_task_reward(
        &mut self,
        facility_id: &str,
        task_id: &str,
    ) -> Result<TaskRewardOutcome, &'static str> {
        let Some(facility) = self.content.town_facility(facility_id) else {
            return Err("unknown-task-service");
        };
        if !task_service_accessible(self, facility_id) {
            return Err("task-service-unreachable");
        }
        if !facility.task_ids.iter().any(|id| id == task_id) {
            return Err("task-unavailable");
        }
        let Some(task) = self
            .content
            .world(&self.world_id)
            .and_then(|world| task_definition(world, task_id))
            .cloned()
        else {
            return Err("task-unavailable");
        };
        if task.source_facility_id.as_deref() != Some(facility_id) {
            return Err("task-source-mismatch");
        }
        if self
            .task_states
            .get(task_id)
            .is_none_or(|state| state.status != TaskStatusKindDto::RewardAvailable)
        {
            return Err("reward-unavailable");
        }
        if self.instance_id_exists(&task.reward.item_instance_id)
            || self.stored_floors.values().any(|floor| {
                floor
                    .items
                    .iter()
                    .any(|item| item.id == task.reward.item_instance_id)
            })
        {
            return Err("reward-item-id-unavailable");
        }

        let preview = reward_item(self, &task.reward, self.rng.clone());
        if self.inventory_quantity_capacity_for(&preview, false) < task.reward.quantity {
            return Err("inventory-full");
        }

        let reward = reward_item(self, &task.reward, self.rng.clone());
        let mut remaining = reward.quantity;
        let definition = self
            .content
            .item(&reward.kind_id)
            .expect("validated task reward kind must remain available");
        let mut stack_indices = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                item.location == ItemLocation::Inventory
                    && item.quantity < definition.max_stack
                    && item_instances_stack_compatible(item, &reward)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        stack_indices.sort_by(|left, right| self.items[*left].id.cmp(&self.items[*right].id));
        for index in stack_indices {
            let transferred = remaining.min(definition.max_stack - self.items[index].quantity);
            self.items[index].quantity += transferred;
            remaining -= transferred;
            if remaining == 0 {
                break;
            }
        }
        if remaining > 0 {
            let mut reward = reward;
            reward.quantity = remaining;
            self.items.push(reward);
        }
        self.rng = {
            let mut committed = self.rng.clone();
            let _ = initial_item_runtime_state(
                &self.content,
                &mut committed,
                &task.reward.item_kind_id,
                1,
            );
            committed
        };
        self.task_states
            .get_mut(task_id)
            .expect("preflighted task state must remain available")
            .status = TaskStatusKindDto::Completed;
        Ok(TaskRewardOutcome {
            item_kind_id: task.reward.item_kind_id,
            quantity: task.reward.quantity,
        })
    }

    fn bind_external_tasks_to_floor_transitions(&mut self, events: &[DomainEvent]) {
        let Some((from_floor_id, to_floor_id)) =
            events.iter().rev().find_map(|event| match event {
                DomainEvent::FloorTransitioned {
                    from_floor_id,
                    to_floor_id,
                } => Some((from_floor_id.as_str(), to_floor_id.as_str())),
                _ => None,
            })
        else {
            return;
        };
        let Some(world) = self.content.world(&self.world_id) else {
            return;
        };
        let Some(target_floor) = world
            .procedural_floors
            .iter()
            .find(|floor| floor.id == to_floor_id)
        else {
            let mut failed_floor_ids = Vec::new();
            for (task_id, state) in &mut self.task_states {
                if state.status == TaskStatusKindDto::Active
                    && state.active_floor_id.as_deref() == Some(from_floor_id)
                {
                    let retakeable = world
                        .procedural_floors
                        .iter()
                        .find(|floor| floor.id == from_floor_id)
                        .is_some_and(|floor| floor.retakeable);
                    state.status = if retakeable {
                        TaskStatusKindDto::Taken
                    } else {
                        if task_definition(world, task_id)
                            .is_some_and(|task| task.completion_exit_terrain_id.is_some())
                        {
                            failed_floor_ids.push(from_floor_id.to_owned());
                        }
                        TaskStatusKindDto::Failed
                    };
                    state.active_floor_id = None;
                }
            }
            self.stored_floors
                .retain(|_, floor| !failed_floor_ids.contains(&floor.id));
            return;
        };
        let bindings = world
            .tasks
            .iter()
            .filter(|task| task.source_facility_id.is_some())
            .map(|task| (task.id.clone(), task_applies_to_floor(task, target_floor)))
            .collect::<Vec<_>>();
        let mut failed_floor_ids = Vec::new();
        for (task_id, applies_to_target) in bindings {
            let Some(state) = self.task_states.get_mut(&task_id) else {
                continue;
            };
            if state.status == TaskStatusKindDto::Active
                && state.active_floor_id.as_deref() == Some(from_floor_id)
            {
                if applies_to_target {
                    state.active_floor_id = Some(to_floor_id.to_owned());
                } else {
                    let retakeable = world
                        .procedural_floors
                        .iter()
                        .find(|floor| floor.id == from_floor_id)
                        .is_some_and(|floor| floor.retakeable);
                    state.status = if retakeable {
                        TaskStatusKindDto::Taken
                    } else {
                        if task_definition(world, &task_id)
                            .is_some_and(|task| task.completion_exit_terrain_id.is_some())
                        {
                            failed_floor_ids.push(from_floor_id.to_owned());
                        }
                        TaskStatusKindDto::Failed
                    };
                    state.active_floor_id = None;
                }
            } else if state.status == TaskStatusKindDto::Taken && applies_to_target {
                state.status = TaskStatusKindDto::Active;
                state.active_floor_id = Some(to_floor_id.to_owned());
            }
        }
        self.stored_floors
            .retain(|_, floor| !failed_floor_ids.contains(&floor.id));
    }

    pub(super) fn active_task_objective(
        &self,
    ) -> Result<Option<(String, TaskObjectiveDefinition, Option<u32>)>, CoreError> {
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        active_task_objective(world, &self.task_states, &self.current_floor_id)
    }

    fn reveal_task_completion_exit(
        &mut self,
        terrain_id: &str,
        floor_terrain_id: &str,
        origin: rfb_protocol::Position,
    ) -> Result<rfb_protocol::Position, CoreError> {
        let position = self
            .terrain
            .iter()
            .enumerate()
            .filter_map(|(index, candidate_terrain_id)| {
                if candidate_terrain_id != floor_terrain_id {
                    return None;
                }
                let position = rfb_protocol::Position {
                    x: i32::try_from(index % usize::from(self.width)).ok()?,
                    y: i32::try_from(index / usize::from(self.width)).ok()?,
                };
                (!self
                    .entities
                    .iter()
                    .any(|entity| entity.hp > 0 && entity.position == position)
                    && !self.items.iter().any(|item| {
                        matches!(item.location, ItemLocation::Ground(ground) if ground == position)
                    })
                    && !self
                        .gold_piles
                        .iter()
                        .any(|pile| pile.position == position))
                .then_some(position)
            })
            .min_by_key(|position| {
                (
                    super::chebyshev_distance(origin, *position),
                    position.y,
                    position.x,
                )
            })
            .ok_or_else(|| {
                CoreError::Invariant(format!(
                    "task completion exit {terrain_id} has no available floor position"
                ))
            })?;
        let index = usize::try_from(position.y).expect("task exit y must be non-negative")
            * usize::from(self.width)
            + usize::try_from(position.x).expect("task exit x must be non-negative");
        self.terrain[index] = terrain_id.to_owned();
        Ok(position)
    }

    pub(super) fn apply_task_events(
        &mut self,
        events: &mut Vec<DomainEvent>,
    ) -> Result<(), CoreError> {
        let clear_floor_completed = self
            .entities
            .iter()
            .all(|entity| entity.hp <= 0 || self.actor_is_player_side(entity));
        let (plan, completion_exit) = {
            let world = self
                .content
                .world(&self.world_id)
                .expect("active world must remain available");
            let plan = plan_task_event_reduction(
                world,
                &self.task_states,
                &self.current_floor_id,
                &self.items,
                clear_floor_completed,
                &self.command_actor_deaths,
                events,
            )?;
            let completion_exit = plan.as_ref().and_then(|plan| {
                (plan.state.status == TaskStatusKindDto::RewardAvailable
                    && self
                        .task_states
                        .get(&plan.task_id)
                        .is_some_and(|state| state.status != TaskStatusKindDto::RewardAvailable))
                .then(|| {
                    let task = task_definition(world, &plan.task_id)?;
                    let terrain_id = task.completion_exit_terrain_id.clone()?;
                    let floor = world
                        .procedural_floors
                        .iter()
                        .find(|floor| floor.id == self.current_floor_id)?;
                    Some((
                        terrain_id,
                        floor.floor_terrain_id.clone(),
                        plan.completion_origin.unwrap_or(self.player.position),
                    ))
                })
                .flatten()
            });
            (plan, completion_exit)
        };
        if let Some(plan) = plan {
            self.task_states.insert(plan.task_id, plan.state);
        }
        if let Some((terrain_id, floor_terrain_id, origin)) = completion_exit {
            let position =
                self.reveal_task_completion_exit(&terrain_id, &floor_terrain_id, origin)?;
            events.push(DomainEvent::TaskExitRevealed {
                floor_id: self.current_floor_id.clone(),
                position,
            });
        }
        self.command_actor_deaths.clear();
        self.bind_external_tasks_to_floor_transitions(events);
        Ok(())
    }

    pub(super) fn current_floor_has_active_task(&self) -> bool {
        let Some(world) = self.content.world(&self.world_id) else {
            return false;
        };
        let Some(floor) = world
            .procedural_floors
            .iter()
            .find(|floor| floor.id == self.current_floor_id)
        else {
            return false;
        };
        world.tasks.iter().any(|task| {
            task_applies_to_floor(task, floor)
                && self.task_states.get(&task.id).is_some_and(|state| {
                    matches!(
                        state.status,
                        TaskStatusKindDto::Active | TaskStatusKindDto::Taken
                    )
                })
        })
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
        let on_surface = self.current_dungeon_instance_id.is_none()
            && self.content.world(&self.world_id).is_some_and(|world| {
                self.current_floor_id == world.initial_floor_id || self.current_town().is_some()
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
