// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeSet;

use rfb_content::{ContentCatalog, TerrainDefinition};
use rfb_protocol::{Direction, Position, TerrainInteractionUnavailableReasonDto};

use crate::{
    check::{CheckContext, CheckKind, resolve_check},
    state::{Actor, ItemInstance, ItemLocation},
    stats::{DerivedStat, DerivedStatsPipeline, StatBounds, StatKind, StatLayer},
};

use super::{Game, TERRAIN_INTERACTION_DIRECTIONS};

pub(super) enum TrapDisarmOutcome {
    Succeeded { position: Position },
    Failed { position: Position },
}

pub(super) enum TerrainDigOutcome {
    Succeeded { position: Position },
    Failed { position: Position },
}

pub(super) enum DoorOpenOutcome {
    Opened { position: Position },
    Unlocked { position: Position },
    UnlockFailed { position: Position },
}

pub(super) enum DoorBashOutcome {
    Succeeded { position: Position },
    Failed { position: Position },
}

struct TerrainInteractionContext<'a> {
    content: &'a ContentCatalog,
    terrain: &'a [String],
    revealed_terrain: &'a BTreeSet<Position>,
    entities: &'a [Actor],
    items: &'a [ItemInstance],
    width: u16,
    height: u16,
    origin: Position,
}

impl TerrainInteractionContext<'_> {
    fn position_in_direction(&self, direction: Direction) -> Position {
        let (dx, dy) = direction.delta();
        Position {
            x: self.origin.x + dx,
            y: self.origin.y + dy,
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

    fn terrain_at(&self, position: Position) -> Option<(usize, &TerrainDefinition)> {
        let index = self.index(position)?;
        Some((index, self.content.terrain(&self.terrain[index])?))
    }

    fn known_terrain_at(&self, position: Position) -> Option<(usize, &TerrainDefinition)> {
        let (index, terrain) = self.terrain_at(position)?;
        if !self.revealed_terrain.contains(&position)
            && let Some(concealed_as) = terrain.concealed_as_terrain_id.as_deref()
        {
            Some((index, self.content.terrain(concealed_as)?))
        } else {
            Some((index, terrain))
        }
    }

    fn unavailable_reason(
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
}

struct TerrainMutationPlan {
    position: Position,
    index: usize,
    source_id: String,
    target_id: String,
    difficulty: Option<i32>,
    clear_revealed: bool,
}

struct TerrainSearchPlan {
    position: Position,
    terrain_id: String,
    difficulty: i32,
}

fn plan_open_door(
    context: &TerrainInteractionContext<'_>,
    direction: Direction,
) -> Option<TerrainMutationPlan> {
    let position = context.position_in_direction(direction);
    if context.unavailable_reason(position).is_some() {
        return None;
    }
    let (index, terrain) = context.known_terrain_at(position)?;
    Some(TerrainMutationPlan {
        position,
        index,
        source_id: terrain.id.clone(),
        target_id: terrain.open_to_terrain_id.clone()?,
        difficulty: terrain.open_check_difficulty,
        clear_revealed: true,
    })
}

fn plan_close_door(
    context: &TerrainInteractionContext<'_>,
    direction: Direction,
) -> Option<TerrainMutationPlan> {
    let position = context.position_in_direction(direction);
    if context.unavailable_reason(position).is_some() {
        return None;
    }
    let (index, terrain) = context.terrain_at(position)?;
    Some(TerrainMutationPlan {
        position,
        index,
        source_id: terrain.id.clone(),
        target_id: terrain.close_to_terrain_id.clone()?,
        difficulty: None,
        clear_revealed: false,
    })
}

fn plan_bash_door(
    context: &TerrainInteractionContext<'_>,
    direction: Direction,
) -> Option<TerrainMutationPlan> {
    let position = context.position_in_direction(direction);
    if context.unavailable_reason(position).is_some() {
        return None;
    }
    let (index, terrain) = context.known_terrain_at(position)?;
    Some(TerrainMutationPlan {
        position,
        index,
        source_id: terrain.id.clone(),
        target_id: terrain.bash_to_terrain_id.clone()?,
        difficulty: Some(terrain.bash_check_difficulty?),
        clear_revealed: true,
    })
}

fn plan_disarm_trap(
    context: &TerrainInteractionContext<'_>,
    direction: Direction,
) -> Option<TerrainMutationPlan> {
    let position = context.position_in_direction(direction);
    if !context.revealed_terrain.contains(&position)
        || context.unavailable_reason(position).is_some()
    {
        return None;
    }
    let (index, terrain) = context.terrain_at(position)?;
    let trap = terrain.trap.as_ref()?;
    Some(TerrainMutationPlan {
        position,
        index,
        source_id: terrain.id.clone(),
        target_id: trap.disarm_to_terrain_id.clone(),
        difficulty: Some(trap.disarm_check_difficulty),
        clear_revealed: true,
    })
}

fn plan_dig_terrain(
    context: &TerrainInteractionContext<'_>,
    direction: Direction,
) -> Option<TerrainMutationPlan> {
    let position = context.position_in_direction(direction);
    if context.unavailable_reason(position).is_some() {
        return None;
    }
    let (index, terrain) = context.known_terrain_at(position)?;
    Some(TerrainMutationPlan {
        position,
        index,
        source_id: terrain.id.clone(),
        target_id: terrain.dig_to_terrain_id.clone()?,
        difficulty: Some(terrain.dig_check_difficulty?),
        clear_revealed: true,
    })
}

fn plan_search_hidden_terrain(context: &TerrainInteractionContext<'_>) -> Vec<TerrainSearchPlan> {
    TERRAIN_INTERACTION_DIRECTIONS
        .into_iter()
        .filter_map(|direction| {
            let position = context.position_in_direction(direction);
            if context.revealed_terrain.contains(&position) {
                return None;
            }
            let (_, terrain) = context.terrain_at(position)?;
            Some(TerrainSearchPlan {
                position,
                terrain_id: terrain.id.clone(),
                difficulty: terrain.search_check_difficulty?,
            })
        })
        .collect()
}

fn action_difficulty(source_id: &str, difficulty: i32) -> DerivedStat {
    let mut pipeline = DerivedStatsPipeline::new();
    pipeline.add(
        StatKind::ActionDifficulty,
        StatLayer::Environment,
        source_id,
        difficulty,
    );
    pipeline.resolve(StatKind::ActionDifficulty, StatBounds::NON_NEGATIVE)
}

impl Game {
    fn terrain_interaction_context(&self) -> TerrainInteractionContext<'_> {
        TerrainInteractionContext {
            content: &self.content,
            terrain: &self.terrain,
            revealed_terrain: &self.revealed_terrain,
            entities: &self.entities,
            items: &self.items,
            width: self.width,
            height: self.height,
            origin: self.player.position,
        }
    }

    pub(super) fn terrain_interaction_unavailable_reason(
        &self,
        position: Position,
    ) -> Option<TerrainInteractionUnavailableReasonDto> {
        self.terrain_interaction_context()
            .unavailable_reason(position)
    }

    fn terrain_check_succeeded(
        &mut self,
        plan: &TerrainMutationPlan,
        kind: CheckKind,
        ability: DerivedStat,
    ) -> bool {
        resolve_check(
            &mut self.rng,
            CheckContext {
                kind,
                actor_id: self.player.id.clone(),
                target_id: Some(plan.source_id.clone()),
                ability,
                difficulty: action_difficulty(
                    &plan.source_id,
                    plan.difficulty
                        .expect("checked terrain plan must retain its difficulty"),
                ),
            },
        )
        .succeeded()
    }

    fn commit_terrain_mutation(&mut self, plan: TerrainMutationPlan) {
        self.terrain[plan.index] = plan.target_id;
        if plan.clear_revealed {
            self.revealed_terrain.remove(&plan.position);
        }
    }

    pub(super) fn open_door(&mut self, direction: Direction) -> Option<DoorOpenOutcome> {
        let plan = plan_open_door(&self.terrain_interaction_context(), direction)?;
        let checked = plan.difficulty.is_some();
        if checked {
            let ability = self.player_derived_stats().door_skill;
            if !self.terrain_check_succeeded(&plan, CheckKind::UnlockDoor, ability) {
                return Some(DoorOpenOutcome::UnlockFailed {
                    position: plan.position,
                });
            }
        }
        let position = plan.position;
        self.commit_terrain_mutation(plan);
        Some(if checked {
            DoorOpenOutcome::Unlocked { position }
        } else {
            DoorOpenOutcome::Opened { position }
        })
    }

    pub(super) fn close_door(&mut self, direction: Direction) -> Option<Position> {
        let plan = plan_close_door(&self.terrain_interaction_context(), direction)?;
        let position = plan.position;
        self.commit_terrain_mutation(plan);
        Some(position)
    }

    pub(super) fn bash_door(&mut self, direction: Direction) -> Option<DoorBashOutcome> {
        let plan = plan_bash_door(&self.terrain_interaction_context(), direction)?;
        let ability = self.player_derived_stats().bash_power;
        if !self.terrain_check_succeeded(&plan, CheckKind::BashDoor, ability) {
            return Some(DoorBashOutcome::Failed {
                position: plan.position,
            });
        }
        let position = plan.position;
        self.commit_terrain_mutation(plan);
        Some(DoorBashOutcome::Succeeded { position })
    }

    pub(super) fn disarm_trap(&mut self, direction: Direction) -> Option<TrapDisarmOutcome> {
        let plan = plan_disarm_trap(&self.terrain_interaction_context(), direction)?;
        let ability = self.player_derived_stats().disarm_skill;
        if !self.terrain_check_succeeded(&plan, CheckKind::DisarmTrap, ability) {
            return Some(TrapDisarmOutcome::Failed {
                position: plan.position,
            });
        }
        let position = plan.position;
        self.commit_terrain_mutation(plan);
        Some(TrapDisarmOutcome::Succeeded { position })
    }

    pub(super) fn dig_terrain(&mut self, direction: Direction) -> Option<TerrainDigOutcome> {
        let plan = plan_dig_terrain(&self.terrain_interaction_context(), direction)?;
        let ability = self.player_derived_stats().dig_skill;
        if !self.terrain_check_succeeded(&plan, CheckKind::DigTerrain, ability) {
            return Some(TerrainDigOutcome::Failed {
                position: plan.position,
            });
        }
        let position = plan.position;
        self.commit_terrain_mutation(plan);
        Some(TerrainDigOutcome::Succeeded { position })
    }

    pub(super) fn search_hidden_terrain(&mut self) -> Vec<Position> {
        let plans = plan_search_hidden_terrain(&self.terrain_interaction_context());
        let ability = self.player_derived_stats().search_skill;
        let mut discovered = Vec::new();
        for plan in plans {
            let check = resolve_check(
                &mut self.rng,
                CheckContext {
                    kind: CheckKind::SearchTerrain,
                    actor_id: self.player.id.clone(),
                    target_id: Some(plan.terrain_id.clone()),
                    ability: ability.clone(),
                    difficulty: action_difficulty(&plan.terrain_id, plan.difficulty),
                },
            );
            if check.succeeded() {
                self.revealed_terrain.insert(plan.position);
                discovered.push(plan.position);
            }
        }
        discovered
    }
}
