// SPDX-License-Identifier: MPL-2.0

use super::gold::gold_visual_id;
use super::*;

pub(super) fn actor_can_cross_terrain(
    actor: &rfb_content::ActorDefinition,
    terrain: &rfb_content::TerrainDefinition,
) -> bool {
    terrain.walkable
        || actor
            .movement
            .modes
            .iter()
            .any(|mode| terrain.movement_modes.contains(mode))
}

pub(super) fn actor_avoids_terrain_trap(
    actor: &rfb_content::ActorDefinition,
    terrain: &rfb_content::TerrainDefinition,
) -> bool {
    terrain.trap.as_ref().is_some_and(|trap| {
        actor
            .movement
            .modes
            .iter()
            .any(|mode| trap.avoided_by_movement_modes.contains(mode))
    })
}

fn actor_can_interact_with_terrain(
    actor: &rfb_content::ActorDefinition,
    terrain: &rfb_content::TerrainDefinition,
) -> bool {
    (terrain.monster_door_power.is_some()
        && ((actor.door_interaction.opens && terrain.open_to_terrain_id.is_some())
            || (actor.door_interaction.bashes && terrain.bash_to_terrain_id.is_some())))
        || (actor.terrain_interaction.destroys_walls
            && terrain.monster_destroy_to_terrain_id.is_some())
}

impl Game {
    fn item_resists_monster_destruction(
        &self,
        item: &ItemInstance,
        actor: &Actor,
        actor_definition: &rfb_content::ActorDefinition,
    ) -> bool {
        let Some(definition) = self.content.item(&item.kind_id) else {
            return true;
        };
        if definition.resists_monster_destruction
            || definition.tags.iter().any(|tag| tag == "artifact")
        {
            return true;
        }
        let mut resists = false;
        let mut inspect_properties = |slays: &BTreeMap<SlayTarget, SlayLevel>,
                                      brands: &BTreeSet<WeaponBrand>,
                                      protected: bool| {
            resists |= protected
                || slays
                    .keys()
                    .any(|target| slay_target_matches(*target, actor_definition))
                || brands.iter().any(|brand| {
                    actor.resistances.level(brand_damage_type(*brand)) != ResistanceLevel::Immune
                });
        };
        inspect_properties(&definition.slays, &definition.brands, false);
        for affix_id in &item.affix_ids {
            if let Some(affix) = self.content.affix(affix_id) {
                inspect_properties(
                    &affix.slays,
                    &affix.brands,
                    affix.resists_monster_destruction,
                );
            }
        }
        for rolled in &item.rolled_affixes {
            inspect_properties(
                &rolled.properties.slays,
                &rolled.properties.brands,
                self.content
                    .affix(&rolled.affix_id)
                    .is_some_and(|affix| affix.resists_monster_destruction),
            );
        }
        resists
    }

    pub(super) fn destroy_items_under_monster(
        &mut self,
        actor_index: usize,
        position: Position,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let actor = self.entities[actor_index].clone();
        let Some(actor_definition) = self.content.actor(&actor.kind_id).cloned() else {
            return;
        };
        if !actor_definition.terrain_interaction.destroys_items {
            return;
        }
        let mut destroyed = self
            .items
            .iter()
            .filter(|item| matches!(item.location, ItemLocation::Ground(item_position) if item_position == position))
            .filter(|item| {
                !self.item_resists_monster_destruction(item, &actor, &actor_definition)
            })
            .map(|item| (item.id.clone(), item.kind_id.clone(), item.quantity))
            .collect::<Vec<_>>();
        destroyed.sort_by(|left, right| left.0.cmp(&right.0));
        let mut destroyed_gold = self
            .gold_piles
            .iter()
            .filter(|pile| pile.position == position)
            .map(|pile| {
                (
                    pile.id.clone(),
                    gold_visual_id(pile.appearance).to_owned(),
                    pile.amount,
                )
            })
            .collect::<Vec<_>>();
        destroyed_gold.sort_by(|left, right| left.0.cmp(&right.0));
        if destroyed.is_empty() && destroyed_gold.is_empty() {
            return;
        }
        let destroyed_ids = destroyed
            .iter()
            .map(|(item_id, _, _)| item_id.as_str())
            .collect::<BTreeSet<_>>();
        self.items
            .retain(|item| !destroyed_ids.contains(item.id.as_str()));
        self.gold_piles.retain(|pile| pile.position != position);
        changed.insert(position);
        for (_, target_kind_id, quantity) in destroyed {
            events.push(DomainEvent::MonsterItemDestroyed {
                source_kind_id: actor.kind_id.clone(),
                target_kind_id,
                quantity,
                position,
            });
        }
        for (_, target_kind_id, amount) in destroyed_gold {
            events.push(DomainEvent::MonsterItemDestroyed {
                source_kind_id: actor.kind_id.clone(),
                target_kind_id,
                quantity: amount,
                position,
            });
        }
    }

    pub(super) fn actor_kind_can_enter_position(&self, kind_id: &str, position: Position) -> bool {
        let Some(index) = self.index(position) else {
            return false;
        };
        let Some(actor) = self.content.actor(kind_id) else {
            return false;
        };
        self.content
            .terrain(&self.terrain[index])
            .is_some_and(|terrain| actor_can_cross_terrain(actor, terrain))
    }

    pub(super) fn actor_can_enter_position(&self, index: usize, position: Position) -> bool {
        self.actor_kind_can_enter_position(&self.entities[index].kind_id, position)
    }

    pub(super) fn actor_can_traverse_or_interact(&self, index: usize, position: Position) -> bool {
        let Some(terrain_index) = self.index(position) else {
            return false;
        };
        let Some(actor) = self.content.actor(&self.entities[index].kind_id) else {
            return false;
        };
        self.content
            .terrain(&self.terrain[terrain_index])
            .is_some_and(|terrain| {
                actor_can_cross_terrain(actor, terrain)
                    || actor_can_interact_with_terrain(actor, terrain)
            })
    }

    pub(super) fn monster_hostile_target_can_enter_position(
        &self,
        target: &MonsterHostileTarget,
        position: Position,
    ) -> bool {
        match target {
            MonsterHostileTarget::Player { .. } => self.is_walkable(position),
            MonsterHostileTarget::Summon { kind_id, .. } => {
                self.actor_kind_can_enter_position(kind_id, position)
            }
        }
    }

    pub(super) fn player_summon_hostile_targets(&self, index: usize) -> Vec<String> {
        let origin = self.entities[index].position;
        let mut targets = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && entity.id != self.entities[index].id
                    && !self.actor_is_player_aligned(entity)
            })
            .map(|entity| {
                (
                    chebyshev_distance(origin, entity.position),
                    entity.id.clone(),
                )
            })
            .collect::<Vec<_>>();
        targets.sort();
        targets
            .into_iter()
            .map(|(_, entity_id)| entity_id)
            .collect()
    }

    pub(super) fn next_player_summon_step_away_from_owner(&self, index: usize) -> Option<Position> {
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
        let current_distance = chebyshev_distance(start, self.player.position);
        let occupied = self
            .entities
            .iter()
            .enumerate()
            .filter(|(entity_index, entity)| *entity_index != index && entity.hp > 0)
            .map(|(_, entity)| entity.position)
            .collect::<BTreeSet<_>>();
        let mut candidates = DELTAS
            .iter()
            .enumerate()
            .filter_map(|(order, (dx, dy))| {
                let position = Position {
                    x: start.x + dx,
                    y: start.y + dy,
                };
                if position == self.player.position
                    || occupied.contains(&position)
                    || !self.actor_can_traverse_or_interact(index, position)
                {
                    return None;
                }
                let distance = chebyshev_distance(position, self.player.position);
                (distance > current_distance).then_some((
                    std::cmp::Reverse(distance),
                    order,
                    position,
                ))
            })
            .collect::<Vec<_>>();
        candidates.sort();
        candidates.first().map(|(_, _, position)| *position)
    }

    /// Row-major enumeration of open destinations for monster displacement:
    /// inside the map, walkable, free of the player and living actors, and
    /// different from the caster's current cell.
    pub(super) fn displacement_destinations(
        &self,
        source_index: usize,
        accepts: impl Fn(Position) -> bool,
    ) -> Vec<Position> {
        let origin = self.entities[source_index].position;
        let mut destinations = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let position = Position {
                    x: i32::from(x),
                    y: i32::from(y),
                };
                if position == origin
                    || position == self.player.position
                    || !self.actor_can_enter_position(source_index, position)
                    || !accepts(position)
                    || self
                        .entities
                        .iter()
                        .any(|entity| entity.hp > 0 && entity.position == position)
                {
                    continue;
                }
                destinations.push(position);
            }
        }
        destinations
    }

    pub(super) fn entity_is_player_aligned(&self, index: usize) -> bool {
        self.actor_is_player_aligned(&self.entities[index])
    }

    pub(super) fn actor_is_player_aligned(&self, actor: &Actor) -> bool {
        actor.controller_id.as_deref() == Some(self.player.id.as_str())
            || actor
                .summon
                .as_ref()
                .is_some_and(|summon| summon.owner_id == self.player.id)
    }

    pub(super) fn monster_hostile_targets(&self, source_index: usize) -> Vec<MonsterHostileTarget> {
        let origin = self.entities[source_index].position;
        let mut targets = Vec::new();
        if !self.player_is_dead() {
            targets.push(MonsterHostileTarget::Player {
                entity_id: self.player.id.clone(),
                kind_id: self.player.kind_id.clone(),
                position: self.player.position,
            });
        }
        targets.extend(
            self.entities
                .iter()
                .enumerate()
                .filter(|(index, entity)| {
                    *index != source_index && entity.hp > 0 && self.entity_is_player_aligned(*index)
                })
                .map(|(_, entity)| MonsterHostileTarget::Summon {
                    entity_id: entity.id.clone(),
                    kind_id: entity.kind_id.clone(),
                    position: entity.position,
                }),
        );
        targets.sort_by(|left, right| {
            let left_position = left.position();
            let right_position = right.position();
            let left_distance = origin
                .x
                .abs_diff(left_position.x)
                .max(origin.y.abs_diff(left_position.y));
            let right_distance = origin
                .x
                .abs_diff(right_position.x)
                .max(origin.y.abs_diff(right_position.y));
            left_distance
                .cmp(&right_distance)
                .then_with(|| right.is_player().cmp(&left.is_player()))
                .then_with(|| left.entity_id().cmp(right.entity_id()))
        });
        targets
    }

    pub(super) fn next_monster_step(&self, index: usize) -> Option<Position> {
        self.monster_hostile_targets(index)
            .first()
            .and_then(|target| self.next_monster_step_toward(index, target.position(), true))
    }

    pub(super) fn next_monster_step_away(&self, index: usize) -> Option<Position> {
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
        let targets = self.monster_hostile_targets(index);
        let minimum_distance = |position: Position| {
            targets
                .iter()
                .map(|target| {
                    position
                        .x
                        .abs_diff(target.position().x)
                        .max(position.y.abs_diff(target.position().y))
                })
                .min()
                .unwrap_or(0)
        };
        let current_distance = minimum_distance(start);
        let movement_region = self
            .floor_regions
            .iter()
            .find(|region| region.cells.contains(&start));
        let occupied = self
            .entities
            .iter()
            .enumerate()
            .filter(|(entity_index, entity)| *entity_index != index && entity.hp > 0)
            .map(|(_, entity)| entity.position)
            .collect::<BTreeSet<_>>();
        let mut candidates = DELTAS
            .iter()
            .enumerate()
            .filter_map(|(order, (dx, dy))| {
                let position = Position {
                    x: start.x + dx,
                    y: start.y + dy,
                };
                if position == self.player.position
                    || occupied.contains(&position)
                    || !self.actor_can_traverse_or_interact(index, position)
                    || movement_region.is_some_and(|region| !region.cells.contains(&position))
                {
                    return None;
                }
                let distance = minimum_distance(position);
                (distance > current_distance).then_some((
                    std::cmp::Reverse(distance),
                    order,
                    position,
                ))
            })
            .collect::<Vec<_>>();
        candidates.sort();
        candidates.first().map(|(_, _, position)| *position)
    }

    pub(super) fn next_surround_step(
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
                || !self.actor_can_traverse_or_interact(index, target)
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

    pub(super) fn next_monster_step_toward(
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
                || !self.actor_can_traverse_or_interact(index, position)
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
                    || !self.actor_can_traverse_or_interact(index, next)
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

    /// Confusion scrambles one in-flight move: a bounded(4) draw of 0 keeps
    /// the intended direction (no event), anything else redirects to a
    /// bounded(8) draw over the canonical direction order. Both draws only
    /// happen while the status is active, so unconfused replays are
    /// byte-identical.
    pub(super) fn confused_direction(
        &mut self,
        intended: Direction,
        events: &mut Vec<DomainEvent>,
    ) -> Direction {
        const CANONICAL_DIRECTIONS: [Direction; 8] = [
            Direction::North,
            Direction::NorthEast,
            Direction::East,
            Direction::SouthEast,
            Direction::South,
            Direction::SouthWest,
            Direction::West,
            Direction::NorthWest,
        ];
        if !self.player_has_status_kind(STATUS_CONFUSION) {
            return intended;
        }
        if self.rng.bounded(4) == 0 {
            return intended;
        }
        let actual =
            CANONICAL_DIRECTIONS[usize::try_from(self.rng.bounded(8)).expect("index fits")];
        events.push(DomainEvent::PlayerConfusedMove { intended, actual });
        actual
    }
}
