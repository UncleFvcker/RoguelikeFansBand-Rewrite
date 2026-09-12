// SPDX-License-Identifier: MPL-2.0

use super::*;

impl Game {
    pub(super) fn start_fishing(
        &mut self,
        source: &str,
        direction: Direction,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let position = self.position_in_direction(direction);
        if !self.fishing_water(position) {
            events.push(DomainEvent::FishingNoWater);
            return false;
        }
        if let Some(actor) = self
            .entities
            .iter()
            .find(|actor| actor.hp > 0 && actor.position == position)
        {
            events.push(DomainEvent::FishingBlocked {
                actor_kind_id: actor.kind_id.clone(),
            });
            return false;
        }
        self.fishing_direction = Some(direction);
        self.mark_item_aware(source);
        events.push(DomainEvent::FishingStarted);
        true
    }

    fn fishing_water(&self, position: Position) -> bool {
        self.index(position)
            .and_then(|index| self.content.terrain(&self.terrain[index]))
            .is_some_and(|terrain| terrain.tags.iter().any(|tag| tag == "water"))
    }

    pub(in crate::game) fn fishing_state_is_valid(&self) -> bool {
        let Some(direction) = self.fishing_direction else {
            return false;
        };
        let position = self.position_in_direction(direction);
        self.map_scale == MapScaleDto::Local
            && !self.player_is_dead()
            && self.pending_duelist.is_none()
            && self.pending_mutation_direction.is_none()
            && self.pending_ability_direction.is_none()
            && ![
                STATUS_PARALYSIS,
                STATUS_CONFUSION,
                STATUS_BLINDNESS,
                STATUS_STUN,
            ]
            .iter()
            .any(|kind| self.player_has_status_kind(kind))
            && self.fishing_water(position)
            && !self
                .entities
                .iter()
                .any(|actor| actor.hp > 0 && actor.position == position)
    }

    pub(in crate::game) fn continue_fishing(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> bool {
        if !self.fishing_state_is_valid() {
            self.fishing_direction = None;
            return false;
        }
        if self.rng.bounded(1000) != 0 {
            return true;
        }
        let direction = self.fishing_direction.take().unwrap();
        let position = self.position_in_direction(direction);
        let caught = self
            .select_fishing_monster()
            .filter(|_| self.rng.bounded(2) == 0)
            .filter(|kind_id| {
                let definition = self.content.actor(kind_id).unwrap();
                let terrain = self
                    .content
                    .terrain(&self.terrain[self.index(position).unwrap()])
                    .unwrap();
                super::super::movement::actor_can_cross_terrain(definition, terrain)
                    && self.actor_kind_available_instance_count(kind_id) > 0
            });
        if let Some(kind_id) = caught {
            let definition = self.content.actor(&kind_id).unwrap();
            let id = format!(
                "{}.fishing.{}",
                self.current_floor_id, self.last_command_seq
            );
            let actor = spawn_actor_from_definition(
                &mut self.rng,
                definition,
                &id,
                position,
                INITIAL_MONSTER_ENERGY_NEED,
                true,
            );
            self.entities.push(actor);
            changed.insert(position);
            events.push(DomainEvent::FishingCaught {
                actor_kind_id: kind_id,
            });
        } else {
            events.push(DomainEvent::FishingBaitLost);
        }
        true
    }

    pub(super) fn return_pets(
        &mut self,
        source: &str,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> bool {
        // master:devices.c::ang_sort_comp_pet. Pet nicknames are not represented;
        // stable instance IDs provide the final tie-break across save/load.
        let mut pets = self
            .entities
            .iter()
            .filter(|actor| {
                actor.hp > 0
                    && actor.controller_id.as_deref() == Some(self.player.id.as_str())
                    && self.riding_actor_id.as_deref() != Some(actor.id.as_str())
            })
            .map(|actor| {
                let definition = self.actor_runtime_definition(actor).unwrap();
                (
                    std::cmp::Reverse(definition.tags.iter().any(|tag| tag == "unique")),
                    std::cmp::Reverse(definition.level),
                    std::cmp::Reverse(actor.hp),
                    actor.id.clone(),
                )
            })
            .collect::<Vec<_>>();
        pets.sort();
        let noticed = !pets.is_empty();
        for (_, _, _, actor_id) in pets {
            let index = self
                .entities
                .iter()
                .position(|actor| actor.id == actor_id)
                .unwrap();
            // spells3.c::teleport_monster_to consumes its power-100 skill roll.
            self.rng.bounded(100);
            let mut maximum = 2;
            let mut minimum = 1;
            let mut destination = None;
            'search: for _ in 0..499 {
                maximum = maximum.min(200);
                for _ in 0..500 {
                    let position = loop {
                        let position = Position {
                            y: self.player.position.y
                                + self.rng.bounded((maximum * 2 + 1) as u64) as i32
                                - maximum,
                            x: self.player.position.x
                                + self.rng.bounded((maximum * 2 + 1) as u64) as i32
                                - maximum,
                        };
                        let distance = rfb_distance(self.player.position, position);
                        if distance >= minimum as u32 && distance <= maximum as u32 {
                            break position;
                        }
                    };
                    if self.pet_recall_position(index, position) {
                        destination = Some(position);
                        break 'search;
                    }
                }
                maximum *= 2;
                minimum /= 2;
            }
            if let Some(to) = destination {
                let from = self.entities[index].position;
                self.entities[index].position = to;
                changed.extend([from, to]);
                events.push(DomainEvent::MonsterBlinked {
                    source_kind_id: self.entities[index].kind_id.clone(),
                    resolution: MonsterDisplacementResolutionDto { actor_id, from, to },
                });
            }
        }
        if noticed {
            self.mark_item_aware(source);
        }
        noticed
    }

    fn pet_recall_position(&self, actor_index: usize, position: Position) -> bool {
        let Some(index) = self.index(position) else {
            return false;
        };
        let terrain = self.content.terrain(&self.terrain[index]).unwrap();
        // TELEPORT_PASSIVE ignores the pet's terrain movement modes, but still
        // requires teleportable floor and excludes glyphs, monster traps and occupancy.
        Self::terrain_allows_passive_monster_displacement(terrain)
            && position != self.player.position
            && !self.entities.iter().enumerate().any(|(other, actor)| {
                other != actor_index && actor.hp > 0 && actor.position == position
            })
    }

    pub(super) fn resolve_stunning_kick(
        &mut self,
        source: &str,
        power: u16,
        target: Option<&str>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> bool {
        if self.dungeon_blocks_melee() {
            events.push(DomainEvent::PlayerMeleeBlocked);
            return false;
        }
        let Some(index) = self
            .entities
            .iter()
            .position(|actor| Some(actor.id.as_str()) == target && actor.hp > 0)
        else {
            return false;
        };
        let definition = self
            .actor_runtime_definition(&self.entities[index])
            .unwrap();
        let resist_all = definition.tags.iter().any(|tag| tag == "resist-all");
        let immune = resist_all || self.actor_has_status_immunity(index, STATUS_STUN);
        let resisted = [DamageType::Sound, DamageType::Force]
            .into_iter()
            .any(|kind| {
                self.entities[index]
                    .resistances
                    .level(kind)
                    .reduction_percent()
                    > 0
            });
        let previous = self.entities[index]
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_STUN)
            .map_or(0, |status| status.remaining_ticks);
        let duration = previous
            .saturating_add((u32::from(power) / (1 + previous / 20)).max(1))
            .min(200);
        let rejection = if immune {
            Some(AbilityStatusChangeDto::Immune)
        } else if resisted {
            Some(AbilityStatusChangeDto::Resisted)
        } else {
            None
        };
        if !resist_all {
            self.wake_entity(index, events);
            self.entities[index].alerted = true;
            changed.insert(self.entities[index].position);
        }
        let resolution = self.apply_monster_status_result(
            index,
            source,
            STATUS_STUN,
            duration,
            AbilityStatusStackingDefinition::Replace,
            power,
            None,
            None,
            rejection,
        );
        if !immune && !resisted {
            self.anger_monster_from_control_effect(index);
        }
        self.push_actor_effect_resolution(source, index, resolution, events);
        self.mark_item_aware(source);
        true
    }
}
