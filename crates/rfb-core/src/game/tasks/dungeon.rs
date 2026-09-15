// SPDX-License-Identifier: MPL-2.0

// RFB master a0d92b6378: quest.c (automatic Angband quests).
use std::collections::{BTreeMap, BTreeSet};

use rfb_content::{TaskDefinition, TaskLocationDefinition, WorldDefinition};
use rfb_protocol::{ItemOriginKindDto, Position, TaskStatusKindDto as Status, VirtueKindDto};

use super::{Game, TaskState, resolved_task_objective, task_applies_to_floor, task_is_birth_taken};
use crate::game::loot::{ItemGenerationMode, LootContext, LootSource};
use crate::game::{
    INITIAL_MONSTER_ENERGY_NEED, actor_starts_alerted, rfb_distance, spawn_actor_from_definition,
};
use crate::{error::CoreError, event::DomainEvent, state::Actor};

pub(in crate::game) fn automatic(task: &TaskDefinition) -> bool {
    task_is_birth_taken(task)
        || (task.source_facility_id.is_none()
            && matches!(
                task.location,
                TaskLocationDefinition::RandomDungeonDepth { .. }
            ))
}

fn unfinished(state: &TaskState) -> bool {
    matches!(
        state.status,
        Status::Available | Status::Taken | Status::Active | Status::Paused
    )
}

/// Every downward consumer uses this check, including transitions without stairs.
pub(in crate::game) fn travel_allowed(
    world: &WorldDefinition,
    states: &BTreeMap<String, TaskState>,
    from: &str,
    to: &str,
) -> bool {
    let Some(target) = world.procedural_floors.iter().find(|floor| floor.id == to) else {
        return true;
    };
    let source_depth = world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == from && floor.dungeon_id == target.dungeon_id)
        .map_or(0, |floor| floor.depth);
    if target.depth <= source_depth {
        return true;
    }
    !world
        .tasks
        .iter()
        .filter(|task| automatic(task))
        .any(|task| {
            states.get(&task.id).is_some_and(|state| {
                unfinished(state)
                    && world.procedural_floors.iter().any(|floor| {
                        floor.dungeon_id == target.dungeon_id
                            && floor.depth >= source_depth
                            && floor.depth < target.depth
                            && task_applies_to_floor(task, floor, Some(state))
                    })
            })
        })
}

pub(in crate::game) fn connection_terrain<'a>(
    world: &'a WorldDefinition,
    states: &BTreeMap<String, TaskState>,
    floor: &'a rfb_content::ProceduralFloorDefinition,
    connection: &'a rfb_content::ProceduralFloorConnectionDefinition,
    target: &str,
) -> &'a str {
    if travel_allowed(world, states, &floor.id, target) {
        &connection.terrain_id
    } else {
        &floor.floor_terrain_id
    }
}

impl Game {
    pub(in crate::game) fn dungeon_task_id_for_floor(
        &self,
        floor: &rfb_content::ProceduralFloorDefinition,
    ) -> Option<&str> {
        self.content
            .world(&self.world_id)?
            .tasks
            .iter()
            .find(|task| {
                automatic(task)
                    && self.task_states.get(&task.id).is_some_and(|state| {
                        unfinished(state) && task_applies_to_floor(task, floor, Some(state))
                    })
            })
            .map(|task| task.id.as_str())
    }

    pub(in crate::game) fn validate_dungeon_tasks(&self) -> Result<(), CoreError> {
        let world = self.content.world(&self.world_id).expect("world");
        for task in world.tasks.iter().filter(|task| automatic(task)) {
            let state = self
                .task_states
                .get(&task.id)
                .ok_or(CoreError::InvalidSave("missing automatic task"))?;
            let kind = resolved_task_objective(task, state)
                .and_then(|objective| objective.actor_kind_id)
                .ok_or(CoreError::InvalidSave("automatic task has no target"))?;
            let dead = self
                .content
                .actor(&kind)
                .and_then(|actor| actor.finite_lifetime_instance_limit())
                .is_some_and(|limit| {
                    self.defeated_limited_actor_counts
                        .get(&kind)
                        .copied()
                        .unwrap_or(0)
                        >= limit
                });
            if matches!(
                state.status,
                Status::Failed | Status::Locked | Status::RewardAvailable
            ) || (state.status == Status::Abandoned && state.random_assignment.is_none())
                || (state.status == Status::Skipped && !dead)
                || (unfinished(state) && state.current >= state.required)
            {
                return Err(CoreError::InvalidSave("automatic task status is invalid"));
            }
        }
        Ok(())
    }

    pub(in crate::game) fn active_dungeon_task_id(&self) -> Option<&str> {
        let world = self.content.world(&self.world_id)?;
        world
            .tasks
            .iter()
            .find(|task| {
                automatic(task)
                    && self.task_states.get(&task.id).is_some_and(|state| {
                        state.status == Status::Active
                            && state.active_floor_id.as_deref() == Some(&self.current_floor_id)
                    })
            })
            .map(|task| task.id.as_str())
    }

    pub(in crate::game) fn dungeon_task_travel_allowed(&self, from: &str, to: &str) -> bool {
        travel_allowed(
            self.content.world(&self.world_id).expect("active world"),
            &self.task_states,
            from,
            to,
        )
    }

    pub(in crate::game) fn dungeon_task_departure(
        &mut self,
        abandon: bool,
        events: &mut Vec<DomainEvent>,
    ) {
        let Some(id) = self.active_dungeon_task_id().map(str::to_owned) else {
            return;
        };
        let task = self
            .content
            .world(&self.world_id)
            .expect("world")
            .tasks
            .iter()
            .find(|task| task.id == id)
            .expect("task");
        let kind = resolved_task_objective(task, &self.task_states[&id])
            .and_then(|objective| objective.actor_kind_id)
            .expect("target");
        if !abandon
            && !self
                .content
                .actor(&kind)
                .expect("target")
                .tags
                .iter()
                .any(|tag| tag == "unique")
        {
            // delete_monster_idx on retreat is not a death: no drops, XP, or progress.
            let removed = self
                .entities
                .iter()
                .filter(|actor| actor.kind_id == kind)
                .map(|actor| actor.id.clone())
                .collect::<BTreeSet<_>>();
            for actor_id in &removed {
                self.clear_riding_bond_for(actor_id);
                self.clear_duelist_challenge_for(actor_id);
            }
            if self
                .riding_actor_id
                .as_ref()
                .is_some_and(|id| removed.contains(id))
            {
                self.riding_actor_id = None;
            }
            self.entities.retain(|actor| !removed.contains(&actor.id));
            for actor in &mut self.entities {
                if actor
                    .pack
                    .as_ref()
                    .is_some_and(|pack| removed.contains(&pack.leader_id))
                {
                    actor.pack = None;
                }
            }
            self.items.retain(|item| !matches!(&item.location, crate::state::ItemLocation::CarriedBy { actor_id } if removed.contains(actor_id)));
        }
        let state = self.task_states.get_mut(&id).expect("active task");
        state.status = if abandon {
            Status::Abandoned
        } else {
            Status::Paused
        };
        state.active_floor_id = None;
        if abandon {
            self.add_virtue(VirtueKindDto::Valour, -2);
            self.fame_on_failure();
            events.push(DomainEvent::TaskAbandoned { floor_id: id });
        } else {
            events.push(DomainEvent::TaskPaused { floor_id: id });
        }
    }

    pub(in crate::game) fn refresh_dungeon_task_stairs(&mut self) {
        let world = self.content.world(&self.world_id).expect("active world");
        for (id, width, terrain, connections) in std::iter::once((
            &self.current_floor_id,
            self.width,
            &mut self.terrain,
            &self.floor_connections,
        ))
        .chain(self.stored_floors.values_mut().map(|floor| {
            (
                &floor.id,
                floor.width,
                &mut floor.terrain,
                &floor.connections,
            )
        })) {
            let Some(floor) = world.procedural_floors.iter().find(|floor| floor.id == *id) else {
                continue;
            };
            for state in connections {
                let connection = floor
                    .connections
                    .iter()
                    .find(|connection| connection.id == state.id)
                    .expect("floor connection");
                let target = state
                    .target_floor_id
                    .as_deref()
                    .unwrap_or(&connection.target_floor_id);
                let index =
                    state.position.y as usize * usize::from(width) + state.position.x as usize;
                terrain[index] =
                    connection_terrain(world, &self.task_states, floor, connection, target)
                        .to_owned();
            }
        }
    }

    pub(in crate::game) fn activate_dungeon_task(
        &mut self,
        events: &mut Vec<DomainEvent>,
    ) -> Result<(), CoreError> {
        let world = self.content.world(&self.world_id).expect("active world");
        let Some(floor) = world
            .procedural_floors
            .iter()
            .find(|floor| floor.id == self.current_floor_id)
            .cloned()
        else {
            return Ok(());
        };
        let Some(task) = world
            .tasks
            .iter()
            .find(|task| {
                automatic(task)
                    && self.task_states.get(&task.id).is_some_and(|state| {
                        unfinished(state) && task_applies_to_floor(task, &floor, Some(state))
                    })
            })
            .cloned()
        else {
            return Ok(());
        };
        let state = &self.task_states[&task.id];
        let objective = resolved_task_objective(&task, state).expect("automatic objective");
        let kind = objective
            .actor_kind_id
            .as_deref()
            .expect("resolved automatic target");
        let definition = self.content.actor(kind).expect("task actor").clone();
        // Birth assignments and the two fixed tasks are unique-only. A dead target
        // is a pass, never fabricated kill progress or campaign victory.
        let dead = definition
            .finite_lifetime_instance_limit()
            .is_some_and(|limit| {
                self.defeated_limited_actor_counts
                    .get(kind)
                    .copied()
                    .unwrap_or(0)
                    >= limit
            });
        if dead {
            let state = self.task_states.get_mut(&task.id).expect("task state");
            state.status = Status::Skipped;
            state.active_floor_id = None;
            events.push(DomainEvent::TaskSkipped { task_id: task.id });
            self.refresh_dungeon_task_stairs();
            return Ok(());
        }
        let resumed = state.status == Status::Paused;
        let existing = self
            .entities
            .iter()
            .filter(|actor| actor.hp > 0 && actor.kind_id == kind)
            .count() as u32;
        let remaining = state
            .required
            .saturating_sub(state.current)
            .saturating_sub(existing);
        let retake = state.retakes_used.saturating_add(u16::from(resumed));
        for ordinal in 0..remaining {
            if !self.unique_actor_kind_is_available(kind) {
                return Err(CoreError::Invariant(format!(
                    "task target {kind} exists outside its task floor"
                )));
            }
            let positions = self
                .terrain
                .iter()
                .enumerate()
                .filter_map(|(index, terrain)| {
                    let terrain = self.content.terrain(terrain).expect("floor terrain");
                    let position = Position {
                        x: (index % usize::from(self.width)) as i32,
                        y: (index / usize::from(self.width)) as i32,
                    };
                    ((terrain.walkable
                        || terrain
                            .movement_modes
                            .contains(&rfb_content::ActorMovementMode::Fly))
                        && rfb_distance(self.player.position, position) >= 10
                        && !self
                            .entities
                            .iter()
                            .any(|actor| actor.hp > 0 && actor.position == position)
                        && !self.vault_cells[index]
                        && crate::game::movement::actor_can_cross_terrain(&definition, terrain))
                    .then_some(position)
                })
                .collect::<Vec<_>>();
            if positions.is_empty() {
                return Err(CoreError::Invariant(format!(
                    "no legal placement for task {}",
                    task.id
                )));
            }
            let position = positions[self.rng.bounded(positions.len() as u64) as usize];
            let mut actor = spawn_actor_from_definition(
                &mut self.rng,
                &definition,
                &format!("{}.target.{retake}.{ordinal}", task.id),
                position,
                INITIAL_MONSTER_ENERGY_NEED,
                actor_starts_alerted(&definition),
            );
            actor.no_pet = true; // PM_NO_PET / MFLAG2_NOPET.
            let mut actors = Vec::new();
            let policy = floor
                .encounter_table_id
                .as_ref()
                .and_then(|id| self.content.encounter_table(id))
                .and_then(|table| table.global_allocation.clone());
            if definition
                .allocation
                .as_ref()
                .is_some_and(|allocation| allocation.friends.is_none() && allocation.escort)
                && let Some(policy) = policy
            {
                let mut occupied = self
                    .entities
                    .iter()
                    .map(|actor| actor.position)
                    .collect::<BTreeSet<_>>();
                occupied.extend([position, self.player.position]);
                let terrain = self.terrain.clone();
                let members = self.plan_original_group(
                    &floor.id,
                    &policy,
                    kind,
                    position,
                    floor.depth,
                    Some(&task.id),
                    &terrain,
                    self.width,
                    self.height,
                    &mut occupied,
                );
                if !members.is_empty() {
                    let behavior =
                        self.original_pack_behavior(&definition, true, members.len() + 1);
                    let pack_id = format!("{}.pack", actor.id);
                    actor.pack = Some(crate::state::MonsterPackIdentity {
                        id: pack_id.clone(),
                        leader_id: actor.id.clone(),
                        role: rfb_protocol::MonsterPackRoleDto::Leader,
                        behavior,
                    });
                    for (number, member) in members.into_iter().enumerate() {
                        let mut escort = self.generated_actor(
                            format!("{}.escort.{number}", actor.id),
                            &member.kind_id,
                            member.position,
                        );
                        escort.no_pet = true;
                        escort.pack = Some(crate::state::MonsterPackIdentity {
                            id: pack_id.clone(),
                            leader_id: actor.id.clone(),
                            role: rfb_protocol::MonsterPackRoleDto::Member,
                            behavior,
                        });
                        actors.push(escort);
                    }
                }
            }
            actors.push(actor);
            let items = self.generate_carried_loot_for_actors(&actors, &floor.id, floor.depth)?;
            self.items.extend(items);
            self.entities.extend(actors);
        }
        self.entities.sort_by(|a, b| a.id.cmp(&b.id));
        let state = self.task_states.get_mut(&task.id).expect("task state");
        state.status = Status::Active;
        state.active_floor_id = Some(floor.id);
        state.retakes_used = retake;
        events.push(if resumed {
            DomainEvent::TaskResumed { floor_id: task.id }
        } else {
            DomainEvent::TaskAccepted { task_id: task.id }
        });
        self.refresh_dungeon_task_stairs();
        self.reveal_current_visibility();
        Ok(())
    }

    pub(in crate::game) fn progress_dungeon_task_on_death(
        &mut self,
        actor: &Actor,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Result<(), CoreError> {
        if !self
            .entities
            .iter()
            .any(|entity| entity.id == actor.id && entity.kind_id == actor.kind_id)
            || self
                .command_actor_deaths
                .iter()
                .any(|death| death.actor_id == actor.id)
        {
            return Ok(());
        }
        let Some(id) = self.active_dungeon_task_id().map(str::to_owned) else {
            return Ok(());
        };
        let task = self
            .content
            .world(&self.world_id)
            .expect("world")
            .tasks
            .iter()
            .find(|task| task.id == id)
            .expect("task");
        let state = &self.task_states[&id];
        if resolved_task_objective(task, state)
            .and_then(|objective| objective.actor_kind_id)
            .as_deref()
            != Some(&actor.kind_id)
            || self.actor_is_dead_unique_resurrection(actor)
        {
            return Ok(());
        }
        // Reward allocation can fail; don't leave a partially paid completed task.
        let mut staged = self.clone();
        let mut task_events = Vec::new();
        let mut task_changed = BTreeSet::new();
        let state = staged.task_states.get_mut(&id).expect("task");
        state.current += 1;
        if state.current == state.required {
            state.status = Status::Completed;
            state.active_floor_id = None;
            staged.add_virtue(VirtueKindDto::Valour, 2);
            staged.fame = staged.fame.saturating_add(1 + staged.rng.bounded(2) as u16);
            if task_is_birth_taken(task) {
                staged.fame = staged.fame.saturating_add(50);
            }
            staged.refresh_dungeon_task_stairs();
            task_changed.extend(
                staged
                    .floor_connections
                    .iter()
                    .map(|connection| connection.position),
            );
            task_events.push(DomainEvent::TaskCompleted {
                floor_id: id.clone(),
            });
            let floor = staged
                .content
                .world(&staged.world_id)
                .expect("world")
                .procedural_floors
                .iter()
                .find(|floor| floor.id == staged.current_floor_id)
                .expect("task floor")
                .clone();
            if let Some(connection) = floor.connections.iter().find(|connection| {
                Some(&connection.terrain_id) == floor.down_stair_terrain_id.as_ref()
            }) {
                let index = staged
                    .floor_connections
                    .iter()
                    .position(|state| state.id == connection.id)
                    .expect("down connection");
                let old = staged.floor_connections[index].position;
                let position = staged.reveal_task_completion_exit(
                    &connection.terrain_id,
                    &floor.floor_terrain_id,
                    actor.position,
                )?;
                let old_index = staged.index(old).expect("connection position");
                staged.terrain[old_index] = floor.floor_terrain_id.clone();
                staged.floor_connections[index].position = position;
                task_changed.extend([old, position]);
                task_events.push(DomainEvent::TaskExitRevealed {
                    floor_id: id.clone(),
                    position,
                });
            }
            let count = floor.depth / 25
                + 1
                + u16::from((15..=24).contains(&floor.depth) && staged.rng.bounded(5) == 0);
            for _ in 0..count {
                let context = LootContext {
                    table_id: "demo.loot-table.base-items".to_owned(),
                    floor_id: floor.id.clone(),
                    depth: floor.depth,
                    source: LootSource::DungeonTask {
                        task_id: id.clone(),
                    },
                };
                let mut draft = staged
                    .generate_one_loot_draft(&context, ItemGenerationMode::TailoredGreat)
                    .ok_or_else(|| {
                        CoreError::Invariant(format!("failed to generate Angband reward for {id}"))
                    })?;
                draft.origin_kind = Some(ItemOriginKindDto::AngbandReward);
                let item_kind_id = draft.kind_id.clone();
                let quantity = draft.quantity;
                let (position, _) = staged
                    .drop_generated_item_near(draft, actor.position)?
                    .ok_or_else(|| {
                        CoreError::Invariant(format!("no space for Angband reward for {id}"))
                    })?;
                task_changed.insert(position);
                task_events.push(DomainEvent::TaskRewarded {
                    item_kind_id,
                    quantity,
                });
            }
        }
        *self = staged;
        events.extend(task_events);
        changed.extend(task_changed);
        Ok(())
    }
}
