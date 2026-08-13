// SPDX-License-Identifier: MPL-2.0

use super::gold::gold_visual_id;
use super::*;

pub(super) fn actor_can_cross_terrain(
    actor: &rfb_content::ActorDefinition,
    terrain: &rfb_content::TerrainDefinition,
) -> bool {
    use rfb_content::ActorMovementMode;

    if terrain.tags.iter().any(|tag| tag == "warding-glyph") {
        return false;
    }
    if actor.movement.modes.contains(&ActorMovementMode::PassWall) && terrain.allows_wall_passage {
        return true;
    }
    let flies = actor.movement.modes.contains(&ActorMovementMode::Fly);
    if actor.movement.modes.contains(&ActorMovementMode::Aquatic) {
        return terrain.tags.iter().any(|tag| tag == "water")
            || (flies
                && (terrain.walkable || terrain.movement_modes.contains(&ActorMovementMode::Fly)));
    }
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
    terrain.tags.iter().any(|tag| tag == "warding-glyph")
        || (terrain.monster_door_power.is_some()
            && ((actor.door_interaction.opens && terrain.open_to_terrain_id.is_some())
                || (actor.door_interaction.bashes && terrain.bash_to_terrain_id.is_some())))
        || (actor.terrain_interaction.destroys_walls
            && terrain.monster_destroy_to_terrain_id.is_some())
}

impl Game {
    pub(super) fn try_monster_break_warding_glyph(
        &mut self,
        actor_index: usize,
        position: Position,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Option<bool> {
        let terrain_index = self.index(position)?;
        let terrain = self.content.terrain(&self.terrain[terrain_index])?;
        if !terrain.tags.iter().any(|tag| tag == "warding-glyph") {
            return None;
        }
        let source_kind_id = self.entities[actor_index].kind_id.clone();
        let held = if self.actor_is_player_side(&self.entities[actor_index]) {
            true
        } else {
            let level = self
                .actor_runtime_definition(&self.entities[actor_index])?
                .level;
            let resistance = if position == self.player.position {
                550_u64 * 2 / 3
            } else {
                550
            };
            self.rng.bounded(resistance) + 1 >= u64::from(level)
        };
        if held {
            events.push(DomainEvent::WardingGlyphHeld { source_kind_id });
            return Some(false);
        }
        let replacement = terrain
            .monster_destroy_to_terrain_id
            .clone()
            .expect("validated warding glyph must define a destruction target");
        self.terrain[terrain_index] = replacement;
        self.revealed_terrain.remove(&position);
        changed.insert(position);
        events.push(DomainEvent::WardingGlyphBroken {
            source_kind_id,
            position,
        });
        Some(true)
    }
}

impl Game {
    fn item_harms_monster(
        &self,
        item: &ItemInstance,
        actor: &Actor,
        actor_definition: &rfb_content::ActorDefinition,
    ) -> bool {
        let Some(definition) = self.content.item(&item.kind_id) else {
            return true;
        };
        let mut harmful = false;
        let mut inspect_properties =
            |slays: &BTreeMap<SlayTarget, SlayLevel>, brands: &BTreeSet<WeaponBrand>| {
                harmful |= slays
                    .keys()
                    .any(|target| slay_target_matches(*target, actor_definition))
                    || brands.iter().any(|brand| {
                        actor.resistances.level(brand_damage_type(*brand))
                            != ResistanceLevel::Immune
                    });
            };
        inspect_properties(&definition.slays, &definition.brands);
        for affix_id in &item.affix_ids {
            if let Some(affix) = self.content.affix(affix_id) {
                inspect_properties(&affix.slays, &affix.brands);
            }
        }
        for rolled in &item.rolled_affixes {
            inspect_properties(&rolled.properties.slays, &rolled.properties.brands);
        }
        harmful
    }

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
        definition.resists_monster_destruction
            || item.affix_ids.iter().any(|affix_id| {
                self.content
                    .affix(affix_id)
                    .is_some_and(|affix| affix.resists_monster_destruction)
            })
            || item.rolled_affixes.iter().any(|rolled| {
                self.content
                    .affix(&rolled.affix_id)
                    .is_some_and(|affix| affix.resists_monster_destruction)
            })
            || self.item_harms_monster(item, actor, actor_definition)
    }

    pub(super) fn pick_up_items_under_monster(
        &mut self,
        actor_index: usize,
        position: Position,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let actor = self.entities[actor_index].clone();
        let Some(actor_definition) = self.actor_runtime_definition(&actor).cloned() else {
            return;
        };
        if !actor_definition.terrain_interaction.picks_up_items {
            return;
        }
        let mut picked_up = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| matches!(item.location, ItemLocation::Ground(item_position) if item_position == position))
            .filter(|(_, item)| {
                self.content.item(&item.kind_id).is_some_and(|definition| {
                    !definition.tags.iter().any(|tag| {
                        matches!(tag.as_str(), "artifact" | "corpse" | "skeleton" | "statue")
                    })
                }) && !self.item_harms_monster(item, &actor, &actor_definition)
            })
            .map(|(index, item)| (index, item.id.clone(), item.kind_id.clone(), item.quantity))
            .collect::<Vec<_>>();
        picked_up.sort_by(|left, right| left.1.cmp(&right.1));
        if picked_up.is_empty() {
            return;
        }
        for (index, _, target_kind_id, quantity) in picked_up {
            self.items[index].location = ItemLocation::CarriedBy {
                actor_id: actor.id.clone(),
            };
            events.push(DomainEvent::MonsterItemPickedUp {
                source_kind_id: actor.kind_id.clone(),
                target_kind_id,
                quantity,
                position,
            });
        }
        changed.insert(position);
    }

    pub(super) fn destroy_items_under_monster(
        &mut self,
        actor_index: usize,
        position: Position,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let actor = self.entities[actor_index].clone();
        let Some(actor_definition) = self.actor_runtime_definition(&actor).cloned() else {
            return;
        };
        if !actor_definition.terrain_interaction.destroys_items {
            return;
        }
        if actor_definition.terrain_interaction.picks_up_items {
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
        for (item_id, _, _) in &destroyed {
            self.force_open_capture_ball(item_id, position, false, events, changed);
        }
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
        let Some(terrain_index) = self.index(position) else {
            return false;
        };
        self.actor_runtime_definition(&self.entities[index])
            .and_then(|actor| {
                self.content
                    .terrain(&self.terrain[terrain_index])
                    .map(|terrain| actor_can_cross_terrain(actor, terrain))
            })
            .unwrap_or(false)
    }

    pub(super) fn actor_can_kill_body_blocker(
        &self,
        source_index: usize,
        target_index: usize,
    ) -> bool {
        let source = &self.entities[source_index];
        let target = &self.entities[target_index];
        let Some(source_definition) = self.actor_runtime_definition(source) else {
            return false;
        };
        let Some(target_definition) = self.actor_runtime_definition(target) else {
            return false;
        };
        source_definition.kills_weaker_bodies
            && source_definition.melee_routine.is_some()
            && target.hp > 0
            && self.riding_actor_id.as_deref() != Some(target.id.as_str())
            && self.actor_can_enter_position(source_index, target.position)
            && u64::from(source_definition.level).saturating_mul(source_definition.experience_value)
                > u64::from(target_definition.level)
                    .saturating_mul(target_definition.experience_value)
    }

    pub(super) fn actor_can_move_body_blocker(
        &self,
        source_index: usize,
        target_index: usize,
    ) -> bool {
        let source = &self.entities[source_index];
        let target = &self.entities[target_index];
        let Some(source_definition) = self.actor_runtime_definition(source) else {
            return false;
        };
        let Some(target_definition) = self.actor_runtime_definition(target) else {
            return false;
        };
        source_definition.moves_weaker_bodies
            && !source_definition.movement.never_moves
            && target.hp > 0
            && self.entity_is_player_side(source_index) == self.entity_is_player_side(target_index)
            && self.riding_actor_id.as_deref() != Some(target.id.as_str())
            && self.actor_can_enter_position(source_index, target.position)
            && self.actor_kind_can_enter_position(&target.kind_id, source.position)
            && source_definition.experience_value > target_definition.experience_value
    }

    pub(super) fn actor_can_traverse_or_interact(&self, index: usize, position: Position) -> bool {
        let Some(terrain_index) = self.index(position) else {
            return false;
        };
        let Some(actor) = self.actor_runtime_definition(&self.entities[index]) else {
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
                    && !self.actor_is_player_side(entity)
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

    pub(super) fn actor_is_friendly(&self, actor: &Actor) -> bool {
        actor.friendly
            || self
                .content
                .actor(&actor.kind_id)
                .is_some_and(|definition| definition.friendly)
    }

    pub(super) fn actor_is_player_side(&self, actor: &Actor) -> bool {
        self.actor_is_player_aligned(actor) || self.actor_is_friendly(actor)
    }

    pub(super) fn entity_is_player_side(&self, index: usize) -> bool {
        self.actor_is_player_side(&self.entities[index])
    }

    pub(super) fn monster_hostile_targets(&self, source_index: usize) -> Vec<MonsterHostileTarget> {
        let origin = self.entities[source_index].position;
        let source_is_player_side = self.entity_is_player_side(source_index);
        let mut targets = Vec::new();
        if !source_is_player_side && !self.player_is_dead() {
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
                    *index != source_index
                        && entity.hp > 0
                        && self.entity_is_player_side(*index) != source_is_player_side
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

    pub(super) fn next_monster_hiding_step(&self, index: usize) -> Option<Position> {
        let origin = self.entities[index].position;
        let occupied = self
            .entities
            .iter()
            .enumerate()
            .filter(|(other, entity)| *other != index && entity.hp > 0)
            .map(|(_, entity)| entity.position)
            .collect::<BTreeSet<_>>();
        for radius in 1..10_u32 {
            let radius = i32::try_from(radius).expect("small hiding radius must fit i32");
            let mut candidates = (origin.y - radius..=origin.y + radius)
                .flat_map(|y| {
                    (origin.x - radius..=origin.x + radius).map(move |x| Position { x, y })
                })
                .filter(|position| {
                    rfb_distance(origin, *position) == u32::try_from(radius).unwrap_or(u32::MAX)
                        && rfb_distance(self.player.position, *position) >= 2
                        && !occupied.contains(position)
                        && self.actor_can_enter_position(index, *position)
                        && !has_line_of_effect(self, self.player.position, *position)
                        && has_line_of_effect(self, origin, *position)
                })
                .collect::<Vec<_>>();
            candidates.sort_by_key(|position| {
                (
                    rfb_distance(self.player.position, *position),
                    position.y,
                    position.x,
                )
            });
            if let Some(target) = candidates.first() {
                return self.next_monster_step_toward(index, *target, false);
            }
        }
        None
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
            .filter(|(entity_index, _)| {
                *entity_index != index
                    && !self.actor_can_kill_body_blocker(index, *entity_index)
                    && !self.actor_can_move_body_blocker(index, *entity_index)
            })
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
                        && !self.actor_can_kill_body_blocker(index, *entity_index)
                        && !self.actor_can_move_body_blocker(index, *entity_index)
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
