// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

use rfb_content::{ContentCatalog, TaskLocationDefinition, WorldDefinition};
use rfb_protocol::{RandomTaskAssignmentDto, TaskStatusKindDto};

use crate::{error::CoreError, rng::RfbRng};

use super::{Game, TaskState, task_is_birth_taken};

fn spread(base_depth: u16) -> u16 {
    (base_depth / 10).clamp(3, 8)
}

fn target_level_bounds(depth: u16) -> (u16, u16) {
    let adjustment = match depth {
        0..=9 => -2,
        10..=19 => -1,
        81.. => 2,
        71..=80 => 1,
        _ => 0,
    };
    (depth + 1, (i32::from(depth) + 9 + adjustment) as u16)
}

/// Birth only. Validation and restore deliberately never call this reducer.
pub(in crate::game) fn assign_random_tasks(
    world: &WorldDefinition,
    content: &ContentCatalog,
    states: &mut BTreeMap<String, TaskState>,
    rng: &mut RfbRng,
) -> Result<(), CoreError> {
    let mut tasks = world
        .tasks
        .iter()
        .filter_map(|task| {
            if let TaskLocationDefinition::RandomDungeonDepth { base_depth, .. } = task.location {
                Some((base_depth, task))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    tasks.sort_by_key(|(base, task)| (*base, task.id.as_str()));
    let mut reserved = BTreeSet::new();
    let mut last = 0;
    for (base, task) in tasks {
        let span = spread(base);
        let depth = (0..1000)
            .find_map(|_| {
                let depth = base - span + rng.bounded(u64::from(2 * span + 1)) as u16;
                (depth > last).then_some(depth)
            })
            .ok_or_else(|| {
                CoreError::Invariant(format!("random task {} cannot advance its depth", task.id))
            })?;
        let (minimum, maximum) = target_level_bounds(depth);
        let mut pool = world
            .random_task_candidates
            .iter()
            .filter(|entry| !reserved.contains(&entry.actor_kind_id))
            .map(|entry| {
                let actor = content
                    .actor(&entry.actor_kind_id)
                    .expect("validated random task candidate must exist");
                (entry, actor.level)
            })
            .collect::<Vec<_>>();
        pool.sort_by_key(|(entry, level)| (*level, entry.legacy_index));
        // The source eventually relaxes the minimum after 5000 failed draws.
        // Detect a genuinely exhausted pool instead of spinning forever.
        if !pool.iter().any(|(entry, level)| {
            entry.can_be_target
                && *level <= u32::from(maximum.min(127))
                && entry.max_depth >= (minimum + maximum).div_ceil(2).max(*level as u16)
        }) {
            return Err(CoreError::Invariant(format!(
                "random task {} has no available target",
                task.id
            )));
        }
        let mut attempt = 0_u64;
        let actor_kind_id = loop {
            let midpoint = (minimum + maximum).div_ceil(2);
            let level = midpoint + rng.bounded(u64::from(maximum - midpoint + 1)) as u16;
            let level =
                super::super::monster_ecology::original_nasty_allocation_level(rng, level).min(127);
            let available = pool
                .iter()
                .filter(|(entry, actor_level)| {
                    *actor_level <= u32::from(level) && entry.max_depth >= level
                })
                .collect::<Vec<_>>();
            let total = available
                .iter()
                .map(|(entry, _)| u64::from(100 / entry.rarity))
                .sum::<u64>();
            if total > 0 {
                let mut roll = rng.bounded(total);
                let (entry, actor_level) = available
                    .into_iter()
                    .find(|(entry, _)| {
                        let weight = u64::from(100 / entry.rarity);
                        if roll < weight {
                            true
                        } else {
                            roll -= weight;
                            false
                        }
                    })
                    .expect("weighted random task draw must select a candidate");
                if entry.can_be_target
                    && *actor_level <= u32::from(maximum)
                    && (*actor_level > u32::from(minimum) || attempt > 5000)
                {
                    break entry.actor_kind_id.clone();
                }
            }
            attempt += 1;
        };
        reserved.insert(actor_kind_id.clone());
        states
            .get_mut(&task.id)
            .expect("random task must have birth state")
            .random_assignment = Some(RandomTaskAssignmentDto {
            depth,
            actor_kind_id,
        });
        last = depth;
    }
    Ok(())
}

/// Check saved assignments against static source eligibility, never current
/// unique availability: a valid target can already have died during this run.
pub(in crate::game) fn validate_random_task_assignments(
    world: &WorldDefinition,
    content: &ContentCatalog,
    states: &BTreeMap<String, TaskState>,
) -> Result<(), CoreError> {
    let invalid = || CoreError::InvalidSave("random task assignment is invalid");
    let mut assignments = Vec::new();
    let mut targets = BTreeSet::new();
    for task in &world.tasks {
        let state = states.get(&task.id);
        let assignment = state.and_then(|state| state.random_assignment.as_ref());
        let TaskLocationDefinition::RandomDungeonDepth {
            dungeon_id,
            base_depth,
        } = &task.location
        else {
            if assignment.is_some() {
                return Err(invalid());
            }
            if task_is_birth_taken(task) {
                for objective in &task.objectives {
                    if let Some(id) = &objective.actor_kind_id {
                        targets.insert(id.as_str());
                    }
                }
            }
            continue;
        };
        let state = state.ok_or_else(invalid)?;
        let assignment = assignment.ok_or_else(invalid)?;
        let candidate = world
            .random_task_candidates
            .iter()
            .find(|candidate| {
                candidate.actor_kind_id == assignment.actor_kind_id && candidate.can_be_target
            })
            .ok_or_else(invalid)?;
        let actor = content
            .actor(&assignment.actor_kind_id)
            .ok_or_else(invalid)?;
        let span = spread(*base_depth);
        if !(base_depth - span..=base_depth + span).contains(&assignment.depth) {
            return Err(invalid());
        }
        let (minimum, maximum) = target_level_bounds(assignment.depth);
        if actor.level > u32::from(maximum.min(127))
            || candidate.max_depth < (minimum + maximum).div_ceil(2).max(actor.level as u16)
            || state.required != 1
            || state.stage_index != 0
            || state.current > 1
            || matches!(
                state.status,
                TaskStatusKindDto::Locked | TaskStatusKindDto::RewardAvailable
            )
            || (matches!(
                state.status,
                TaskStatusKindDto::Available | TaskStatusKindDto::Taken
            ) && (state.current != 0 || state.retakes_used != 0))
            || !world.procedural_floors.iter().any(|floor| {
                floor.dungeon_id.as_ref() == Some(dungeon_id) && floor.depth == assignment.depth
            })
        {
            return Err(invalid());
        }
        assignments.push((*base_depth, task.id.as_str(), assignment));
    }
    assignments.sort_by_key(|(base, id, _)| (*base, *id));
    let mut last = 0;
    for (_, _, assignment) in assignments {
        if assignment.depth <= last || !targets.insert(&assignment.actor_kind_id) {
            return Err(invalid());
        }
        last = assignment.depth;
    }
    Ok(())
}

impl Game {
    pub(in crate::game) fn actor_kind_is_reserved_task_target(&self, kind_id: &str) -> bool {
        self.content.world(&self.world_id).is_some_and(|world| {
            world.tasks.iter().any(|task| {
                // Source QUESTOR reservation belongs only to the automatic dungeon
                // quests; existing NPC tasks retain their existing allocation rules.
                if !task_is_birth_taken(task)
                    && !matches!(
                        task.location,
                        TaskLocationDefinition::RandomDungeonDepth { .. }
                    )
                {
                    return false;
                }
                self.task_states.get(&task.id).is_some_and(|state| {
                    !matches!(
                        state.status,
                        TaskStatusKindDto::Completed
                            | TaskStatusKindDto::Skipped
                            | TaskStatusKindDto::Failed
                            | TaskStatusKindDto::Abandoned
                    ) && state
                        .random_assignment
                        .as_ref()
                        .map(|assignment| assignment.actor_kind_id.as_str())
                        .or_else(|| {
                            task.objectives
                                .get(state.stage_index as usize)
                                .and_then(|objective| objective.actor_kind_id.as_deref())
                        })
                        == Some(kind_id)
                })
            })
        })
    }
}
