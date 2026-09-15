// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeSet;

use rfb_content::{
    ContentCatalog, TerrainDefinition, TerrainDiggingDefinition, TerrainDiggingResolution,
};
use rfb_protocol::{Direction, MapScaleDto, Position, TerrainInteractionUnavailableReasonDto};

use crate::{
    action::GameAction,
    check::{CheckContext, CheckKind, resolve_check},
    effect::STATUS_CONFUSION,
    state::{Actor, ItemInstance, ItemLocation},
    stats::{DerivedStat, DerivedStatsPipeline, StatBounds, StatKind, StatLayer},
};

use super::{DomainEvent, Game, TERRAIN_INTERACTION_DIRECTIONS};

pub(super) enum TrapDisarmOutcome {
    Succeeded { position: Position },
    Failed { position: Position },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TerrainChangeSource {
    Dig,
    Magic,
    Monster,
    Projectile,
    Disintegration,
    Destruction,
}

pub(super) enum TerrainDigOutcome {
    Succeeded {
        position: Position,
        proficiency_improved: bool,
    },
    Failed {
        position: Position,
        retryable: bool,
    },
    ActorBlocked {
        position: Position,
        index: usize,
    },
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

struct TerrainDigPlan {
    position: Position,
    digging: TerrainDiggingDefinition,
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
) -> Option<TerrainDigPlan> {
    let position = context.position_in_direction(direction);
    let (_, terrain) = context.terrain_at(position)?;
    Some(TerrainDigPlan {
        position,
        digging: terrain.digging.clone()?,
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
    // RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c, cmd1.c move_player.
    // Convenience applies only to walking; explicit alter/open/disarm stay independent.
    pub(super) fn convenient_walk_action(&self, action: GameAction) -> GameAction {
        let GameAction::Move {
            direction,
            flip_pickup,
        } = &action
        else {
            return action;
        };
        if self.map_scale != MapScaleDto::Local || self.player_has_status_kind(STATUS_CONFUSION) {
            return action;
        }
        let position = self.position_in_direction(*direction);
        if self.index(position).is_none()
            || self
                .entities
                .iter()
                .any(|e| e.hp > 0 && e.position == position)
        {
            return action;
        }
        let terrain = self
            .content
            .terrain(self.known_terrain_at(position))
            .unwrap();
        if self.operation_options.easy_open
            && terrain.open_to_terrain_id.is_some()
            && !self.player_can_enter_position(position)
        {
            return GameAction::OpenDoor {
                direction: *direction,
            };
        }
        if (self.operation_options.easy_disarm ^ *flip_pickup)
            && !self.player_is_berserker()
            && self
                .index(position)
                .is_some_and(|index| self.explored[index])
            && terrain.trap.is_some()
        {
            return GameAction::DisarmTrap {
                direction: *direction,
            };
        }
        action
    }

    // RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c, cmd2.c do_cmd_alter.
    // Use mimic terrain; neither a hidden trap nor a secret door grants free knowledge.
    pub(super) fn alter_action(
        &mut self,
        direction: Direction,
        events: &mut Vec<DomainEvent>,
    ) -> crate::action::GameAction {
        use crate::action::GameAction;
        let direction = self.confused_direction(direction, events);
        let context = self.terrain_interaction_context();
        let position = context.position_in_direction(direction);
        if self.entities.iter().any(|actor| actor.position == position) {
            return GameAction::AttackAdjacent { direction };
        }
        if let Some((_, terrain)) = context.known_terrain_at(position) {
            if terrain.open_to_terrain_id.is_some() {
                return GameAction::OpenDoor { direction };
            }
            if terrain.bash_to_terrain_id.is_some() {
                return GameAction::BashDoor { direction };
            }
            if terrain.digging.is_some() {
                return GameAction::DigTerrain { direction };
            }
            if terrain.close_to_terrain_id.is_some() {
                return GameAction::CloseDoor { direction };
            }
            if terrain.trap.is_some() {
                return GameAction::DisarmTrap { direction };
            }
        }
        GameAction::Alter { direction }
    }

    // RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c, cmd2.c do_cmd_spike.
    pub(super) fn prepare_spike_action(
        &mut self,
        direction: Direction,
        events: &mut Vec<DomainEvent>,
    ) -> Option<crate::action::GameAction> {
        use crate::action::GameAction;
        let direction = self.confused_direction(direction, events);
        let position = self.position_in_direction(direction);
        if self
            .terrain_interaction_context()
            .known_terrain_at(position)
            .is_none_or(|(_, terrain)| terrain.jam_to_terrain_id.is_none())
        {
            events.push(DomainEvent::DoorSpikeUnavailable);
            return None;
        }
        if self.entities.iter().any(|actor| actor.position == position) {
            events.push(DomainEvent::DoorSpikeMonsterBlocked);
            return Some(GameAction::AttackAdjacent { direction });
        }
        if self.inventory_spike_index().is_none() {
            events.push(DomainEvent::DoorSpikeMissing);
            return None;
        }
        Some(GameAction::SpikeDoor { direction })
    }

    fn inventory_spike_index(&self) -> Option<usize> {
        self.items.iter().position(|item| {
            item.location == ItemLocation::Inventory
                && self
                    .content
                    .item(&item.kind_id)
                    .and_then(|kind| kind.rfb_base_kind.as_ref())
                    .is_some_and(|kind| kind.tval == 5)
        })
    }

    pub(super) fn spike_door(&mut self, direction: Direction) -> Position {
        let position = self.position_in_direction(direction);
        let context = self.terrain_interaction_context();
        let (index, terrain) = context
            .known_terrain_at(position)
            .expect("prepared spike terrain");
        let target = terrain
            .jam_to_terrain_id
            .clone()
            .expect("prepared spike transition");
        self.terrain[index] = target;
        self.revealed_terrain.remove(&position);
        let item_index = self
            .inventory_spike_index()
            .expect("prepared inventory spike");
        if self.items[item_index].quantity == 1 {
            let removed = self.items.remove(item_index);
            self.item_property_knowledge.remove(&removed.id);
        } else {
            self.items[item_index].quantity -= 1;
        }
        position
    }

    pub(super) fn in_forest_dungeon(&self) -> bool {
        let world = self.content.world(&self.world_id).expect("active world");
        world
            .procedural_floors
            .iter()
            .find(|floor| floor.id == self.current_floor_id)
            .and_then(|floor| floor.dungeon_id.as_ref())
            .is_some_and(|id| {
                world.dungeons.iter().any(|dungeon| {
                    &dungeon.id == id
                        && dungeon.wilderness_terrain == Some(rfb_content::WildernessTerrain::Trees)
                })
            })
    }

    pub(super) fn replace_terrain_from_source(
        &mut self,
        position: Position,
        target_id: &str,
        source: TerrainChangeSource,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> bool {
        let index = self
            .index(position)
            .expect("planned terrain replacement must remain in bounds");
        let source_definition = self
            .content
            .terrain(&self.terrain[index])
            .expect("planned source terrain must remain available")
            .clone();
        let target_definition = self
            .content
            .terrain(target_id)
            .expect("planned target terrain must remain available")
            .clone();
        self.terrain[index] = target_id.to_owned();
        self.revealed_terrain.remove(&position);
        changed.insert(position);
        if !target_definition.allows_items() {
            let mut index = 0;
            while index < self.items.len() {
                if self.items[index].location != ItemLocation::Ground(position) {
                    index += 1;
                    continue;
                }
                if let Some(destination) = self
                    .ground_drop_position(position, self.items[index].is_artifact(&self.content))
                {
                    self.items[index].location = ItemLocation::Ground(destination);
                    changed.insert(destination);
                    index += 1;
                } else {
                    let item = self.items.remove(index);
                    self.item_property_knowledge.remove(&item.id);
                    events.push(DomainEvent::ItemDestroyed {
                        target_kind_id: item.kind_id,
                        quantity: item.quantity,
                        rule_line: None,
                    });
                }
            }
            let destination = self.ground_drop_position(position, false);
            self.gold_piles.retain_mut(|pile| {
                if pile.position != position {
                    return true;
                }
                if let Some(destination) = destination {
                    pile.position = destination;
                    changed.insert(destination);
                    true
                } else {
                    false
                }
            });
        }
        self.resolve_terrain_change_rewards(
            &source_definition,
            &target_definition,
            position,
            source,
            events,
            changed,
        )
    }

    pub(super) fn try_monster_door_interaction(
        &mut self,
        actor_index: usize,
        position: Position,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Option<bool> {
        let terrain_index = self.index(position)?;
        let terrain = self.content.terrain(&self.terrain[terrain_index])?.clone();
        let power = terrain.monster_door_power?;
        if !self.pet_door_interaction_allowed(actor_index) {
            return None;
        }
        let interaction = self
            .actor_runtime_definition(&self.entities[actor_index])?
            .door_interaction;
        let roll_bound = u64::try_from(self.entities[actor_index].hp.max(0) / 10).unwrap_or(0);
        let original_roll = |game: &mut Game| {
            if roll_bound <= 1 {
                0
            } else {
                game.rng.bounded(roll_bound)
            }
        };

        if interaction.opens && terrain.open_to_terrain_id.is_some() {
            if power == 0 {
                self.terrain[terrain_index] = terrain.open_to_terrain_id?;
                self.revealed_terrain.remove(&position);
                changed.insert(position);
                events.push(DomainEvent::DoorOpened { position });
                return Some(false);
            }
            if original_roll(self) > u64::from(power) {
                self.terrain[terrain_index] = terrain.monster_unlock_to_terrain_id?;
                self.revealed_terrain.remove(&position);
                changed.insert(position);
                events.push(DomainEvent::DoorUnlocked { position });
                return Some(false);
            }
            if !interaction.bashes {
                events.push(DomainEvent::DoorUnlockFailed { position });
                return Some(false);
            }
        }

        if interaction.bashes {
            if original_roll(self) > u64::from(power) {
                let target = match terrain.open_to_terrain_id {
                    Some(open) if self.rng.bounded(100) >= 50 => open,
                    _ => terrain.bash_to_terrain_id?,
                };
                self.terrain[terrain_index] = target;
                self.revealed_terrain.remove(&position);
                changed.insert(position);
                events.push(DomainEvent::DoorBashedOpen { position });
                return Some(true);
            }
            events.push(DomainEvent::DoorBashFailed { position });
            return Some(false);
        }
        None
    }

    pub(super) fn try_monster_destroy_terrain(
        &mut self,
        actor_index: usize,
        position: Position,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> bool {
        let Some(terrain_index) = self.index(position) else {
            return false;
        };
        if position.x == 0
            || position.y == 0
            || position.x == i32::from(self.width) - 1
            || position.y == i32::from(self.height) - 1
            || self
                .floor_connections
                .iter()
                .any(|connection| connection.position == position)
        {
            return false;
        }
        let Some(actor) = self.actor_runtime_definition(&self.entities[actor_index]) else {
            return false;
        };
        if !actor.terrain_interaction.destroys_walls {
            return false;
        }
        let Some(terrain) = self.content.terrain(&self.terrain[terrain_index]) else {
            return false;
        };
        let Some(target_id) = terrain.monster_destroy_to_terrain_id.clone() else {
            return false;
        };
        let source_id = terrain.id.clone();
        self.replace_terrain_from_source(
            position,
            &target_id,
            TerrainChangeSource::Monster,
            events,
            changed,
        );
        events.push(DomainEvent::MonsterTerrainDestroyed {
            source_kind_id: self.entities[actor_index].kind_id.clone(),
            terrain_kind_id: source_id,
            replacement_terrain_kind_id: target_id,
            position,
        });
        true
    }

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
        let succeeds = if self.player_is_berserker() {
            let door_power = self
                .content
                .terrain(&plan.source_id)
                .expect("planned terrain exists")
                .monster_door_power
                .unwrap_or(0);
            let chance = ((ability.value - i32::from(door_power) * 10) * 2).max(1);
            self.rng.bounded(100) < chance as u64
        } else {
            self.terrain_check_succeeded(&plan, CheckKind::BashDoor, ability)
        };
        if !succeeds {
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

    pub(super) fn dig_terrain(
        &mut self,
        direction: Direction,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Option<TerrainDigOutcome> {
        let plan = plan_dig_terrain(&self.terrain_interaction_context(), direction)?;
        if let Some(index) = self
            .entities
            .iter()
            .position(|entity| entity.position == plan.position)
        {
            return Some(TerrainDigOutcome::ActorBlocked {
                position: plan.position,
                index,
            });
        }
        let dig_skill = self.player_derived_stats().dig_skill.value;
        let succeeded = match plan.digging.resolution {
            TerrainDiggingResolution::Permanent => false,
            TerrainDiggingResolution::Soft => {
                dig_skill
                    > i32::try_from(
                        self.rng
                            .bounded(u64::from(plan.digging.power).saturating_mul(20)),
                    )
                    .unwrap_or(i32::MAX)
            }
            TerrainDiggingResolution::Hard => {
                dig_skill
                    > i32::from(plan.digging.power).saturating_add(
                        i32::try_from(
                            self.rng
                                .bounded(u64::from(plan.digging.power).saturating_mul(40)),
                        )
                        .unwrap_or(i32::MAX),
                    )
            }
        };
        if !succeeded {
            let retryable = match plan.digging.resolution {
                TerrainDiggingResolution::Soft => true,
                TerrainDiggingResolution::Hard => dig_skill > i32::from(plan.digging.power),
                TerrainDiggingResolution::Permanent => false,
            };
            return Some(TerrainDigOutcome::Failed {
                position: plan.position,
                retryable,
            });
        }
        let target_id = plan
            .digging
            .result_terrain_id
            .expect("successful digging must retain a replacement terrain");
        let proficiency_improved = self.replace_terrain_from_source(
            plan.position,
            &target_id,
            TerrainChangeSource::Dig,
            events,
            changed,
        );
        Some(TerrainDigOutcome::Succeeded {
            position: plan.position,
            proficiency_improved,
        })
    }

    // RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c: cmd1.c search,
    // move_player_effect(MPE_ENERGY_USE); discoveries disturb(0), keeping SEARCH.
    pub(super) fn search_surroundings(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> bool {
        let discovered = self.search_hidden_terrain();
        let found_chest = self.search_chest_traps(events, changed);
        let found = !discovered.is_empty() || found_chest;
        for position in discovered {
            changed.insert(position);
            events.push(DomainEvent::SecretTerrainDiscovered { position });
        }
        if found {
            self.running = None;
            self.auto_explore = None;
        }
        found
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
