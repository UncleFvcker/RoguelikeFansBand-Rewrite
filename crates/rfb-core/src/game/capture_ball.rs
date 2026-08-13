// SPDX-License-Identifier: MPL-2.0

use rfb_content::ActorCapturePolicyDefinition;

use super::*;

pub(super) const CAPTURE_BALL_RANGE: u16 = 8;
const CAPTURE_BALL_RELEASE_RADIUS: u8 = 5;
const CAPTURE_BALL_REGEN_INTERVAL: u32 = 30;
const CAPTURE_BALL_UNIQUE_REGEN_INTERVAL: u32 = 600;

impl Game {
    pub(super) fn capture_ball_target_spec(&self, item: &ItemInstance) -> Option<TargetSpecDto> {
        self.content
            .item(&item.kind_id)
            .is_some_and(|definition| definition.capture_ball)
            .then(|| {
                if item.captured_actor.is_some() {
                    TargetSpecDto {
                        modes: vec![TargetModeDto::Direction],
                        range: 1,
                        requires_line_of_effect: false,
                    }
                } else {
                    TargetSpecDto {
                        modes: vec![TargetModeDto::Entity],
                        range: CAPTURE_BALL_RANGE,
                        requires_line_of_effect: true,
                    }
                }
            })
    }

    pub(super) fn use_capture_ball(
        &mut self,
        item_index: usize,
        target: Option<&TargetSelection>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) {
        if self.items[item_index].captured_actor.is_some() {
            self.release_capture_ball(item_index, target, events, changed);
        } else {
            self.capture_with_ball(item_index, target, events, changed, removed_entities);
        }
    }

    fn capture_with_ball(
        &mut self,
        item_index: usize,
        target: Option<&TargetSelection>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) {
        let Some(target @ TargetSelection::Entity { entity_id }) = target else {
            events.push(DomainEvent::ItemUseUnavailable);
            return;
        };
        let actor_index = if self.riding_actor_id.as_deref() == Some(entity_id.as_str()) {
            let Some(index) = self.entities.iter().position(|actor| {
                actor.id == *entity_id && actor.position == self.player.position && actor.hp > 0
            }) else {
                events.push(DomainEvent::ItemUseUnavailable);
                return;
            };
            index
        } else {
            let Some(path) = self.projectile_path(target, CAPTURE_BALL_RANGE) else {
                events.push(DomainEvent::ItemUseUnavailable);
                return;
            };
            let (_, Some(index)) = self.trace_projectile_path(path) else {
                events.push(DomainEvent::ItemUseUnavailable);
                return;
            };
            index
        };
        if self.entities[actor_index].id != *entity_id || self.entities[actor_index].hp <= 0 {
            events.push(DomainEvent::ItemUseUnavailable);
            return;
        }

        let actor = self.entities[actor_index].clone();
        let definition = self
            .actor_runtime_definition(&actor)
            .expect("capture target definition must remain available")
            .clone();
        let target_kind_id = actor.kind_id.clone();
        let pet = actor.controller_id.as_deref() == Some(self.player.id.as_str());
        let unavailable = match definition.capture_policy {
            ActorCapturePolicyDefinition::Immune => Some("immune"),
            ActorCapturePolicyDefinition::PetOnly if !pet => Some("pet-only"),
            ActorCapturePolicyDefinition::Normal | ActorCapturePolicyDefinition::PetOnly => None,
        };
        if let Some(reason) = unavailable {
            events.push(DomainEvent::CaptureBallCaptureFailed {
                target_kind_id,
                reason: reason.to_owned(),
            });
            return;
        }

        let threshold = if pet {
            i64::from(actor.max_hp).saturating_mul(4)
        } else {
            i64::from(actor.max_hp).saturating_mul(3) / 20
        };
        if threshold <= 0 || i64::from(actor.hp) >= threshold {
            events.push(DomainEvent::CaptureBallCaptureFailed {
                target_kind_id,
                reason: "too-healthy".to_owned(),
            });
            return;
        }
        let roll = i64::try_from(self.rng.bounded(threshold as u64)).unwrap_or(i64::MAX);
        if i64::from(actor.hp) > roll {
            events.push(DomainEvent::CaptureBallCaptureFailed {
                target_kind_id,
                reason: "resisted".to_owned(),
            });
            return;
        }

        if self.riding_actor_id.as_deref() == Some(actor.id.as_str()) {
            self.riding_actor_id = None;
            self.clear_riding_bond_for(&actor.id.clone());
            if self.player_levitates() {
                events.push(DomainEvent::RidingDismounted {
                    target_kind_id: target_kind_id.clone(),
                });
            } else {
                let damage = resolve_damage(
                    DamagePacket::new(
                        i32::try_from(definition.level)
                            .unwrap_or(i32::MAX)
                            .saturating_add(3),
                        DamageType::Physical,
                    ),
                    ResistanceLevel::Normal,
                );
                let application =
                    plan_damage_application(&self.player, damage, FatalityPolicy::BelowZero);
                commit_damage_application(&mut self.player, &application);
                events.push(DomainEvent::RidingFell {
                    target_kind_id: target_kind_id.clone(),
                    damage,
                });
                if application.fatal {
                    events.push(DomainEvent::PlayerDied {
                        source_kind_id: target_kind_id.clone(),
                        method_id: Some("rfb.capture-ball.fall".to_owned()),
                        damage,
                    });
                }
            }
        }

        let captured = CapturedActor {
            kind_id: target_kind_id.clone(),
            speed: actor.speed,
            hp: actor.hp,
            max_hp: actor.max_hp,
            experience: actor.experience,
        };
        let removed_id = actor.id.clone();
        let position = actor.position;
        self.entities.remove(actor_index);
        let removed_item_ids = self
            .items
            .iter()
            .filter_map(|item| match &item.location {
                ItemLocation::CarriedBy { actor_id } if actor_id == &removed_id => {
                    Some(item.id.clone())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        self.items.retain(|item| {
            !matches!(&item.location, ItemLocation::CarriedBy { actor_id } if actor_id == &removed_id)
        });
        for id in removed_item_ids {
            self.item_property_knowledge.remove(&id);
        }
        self.items[item_index].captured_actor = Some(captured);
        removed_entities.push(removed_id);
        changed.insert(position);
        events.push(DomainEvent::CaptureBallCaptured { target_kind_id });
    }

    fn release_capture_ball(
        &mut self,
        item_index: usize,
        target: Option<&TargetSelection>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let Some(TargetSelection::Direction { direction }) = target else {
            events.push(DomainEvent::ItemUseUnavailable);
            return;
        };
        let captured = self.items[item_index]
            .captured_actor
            .as_ref()
            .expect("filled capture ball must retain actor")
            .clone();
        let (dx, dy) = direction.delta();
        let destination = Position {
            x: self.player.position.x + dx,
            y: self.player.position.y + dy,
        };
        if !self.capture_ball_release_position_is_open(&captured.kind_id, destination) {
            events.push(DomainEvent::CaptureBallReleaseFailed {
                target_kind_id: captured.kind_id,
            });
            return;
        }
        self.items[item_index].captured_actor = None;
        self.spawn_captured_actor(captured, destination, false, events, changed);
    }

    fn capture_ball_release_position_is_open(&self, kind_id: &str, position: Position) -> bool {
        position != self.player.position
            && self.actor_kind_can_enter_position(kind_id, position)
            && !self
                .entities
                .iter()
                .any(|entity| entity.hp > 0 && entity.position == position)
    }

    fn spawn_captured_actor(
        &mut self,
        captured: CapturedActor,
        position: Position,
        hostile: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let definition = self
            .content
            .actor(&captured.kind_id)
            .expect("captured actor definition must remain available")
            .clone();
        let id = self.summon_entity_id("capture-ball", 0);
        let mut actor = actor_from_runtime_spawn(
            &id,
            &captured.kind_id,
            position,
            captured.max_hp,
            captured.speed,
            INITIAL_MONSTER_ENERGY_NEED,
            true,
        );
        actor.hp = captured.hp;
        actor.experience = captured.experience;
        actor.resistances = definition_resistance_profile(&definition);
        if !hostile {
            actor.controller_id = Some(self.player.id.clone());
        }
        let target_kind_id = captured.kind_id;
        self.entities.push(actor);
        changed.insert(position);
        events.push(DomainEvent::CaptureBallReleased {
            target_kind_id,
            hostile,
        });
    }

    pub(super) fn force_open_capture_ball(
        &mut self,
        item_id: &str,
        origin: Position,
        allow_hostile: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let Some(index) = self.items.iter().position(|item| item.id == item_id) else {
            return;
        };
        let Some(captured) = self.items[index].captured_actor.take() else {
            return;
        };
        self.release_captured_actor_near(captured, origin, allow_hostile, events, changed);
    }

    pub(super) fn release_captured_actor_near(
        &mut self,
        captured: CapturedActor,
        origin: Position,
        allow_hostile: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let Some(position) = self
            .capture_ball_release_positions(origin, CAPTURE_BALL_RELEASE_RADIUS, &captured.kind_id)
            .into_iter()
            .next()
        else {
            events.push(DomainEvent::CaptureBallReleaseFailed {
                target_kind_id: captured.kind_id,
            });
            return;
        };
        let hostile = allow_hostile && self.rng.bounded(4) == 0;
        self.spawn_captured_actor(captured, position, hostile, events, changed);
    }

    fn capture_ball_release_positions(
        &self,
        origin: Position,
        radius: u8,
        actor_kind_id: &str,
    ) -> Vec<Position> {
        let mut candidates = Vec::new();
        for y in
            origin.y.saturating_sub(i32::from(radius))..=origin.y.saturating_add(i32::from(radius))
        {
            for x in origin.x.saturating_sub(i32::from(radius))
                ..=origin.x.saturating_add(i32::from(radius))
            {
                let position = Position { x, y };
                let distance = origin.x.abs_diff(x).max(origin.y.abs_diff(y));
                if distance == 0
                    || distance > u32::from(radius)
                    || !self.actor_kind_can_enter_position(actor_kind_id, position)
                    || self
                        .entities
                        .iter()
                        .any(|actor| actor.hp > 0 && actor.position == position)
                {
                    continue;
                }
                candidates.push((distance, y, x, position));
            }
        }
        candidates.sort_unstable_by_key(|(distance, y, x, _)| (*distance, *y, *x));
        candidates
            .into_iter()
            .map(|(_, _, _, position)| position)
            .collect()
    }

    pub(super) fn process_captured_actor_regeneration(&mut self) {
        let tick = self.world_tick;
        for index in 0..self.items.len() {
            if !matches!(
                self.items[index].location,
                ItemLocation::Inventory | ItemLocation::Equipped { .. }
            ) {
                continue;
            }
            let Some(captured) = self.items[index].captured_actor.as_ref() else {
                continue;
            };
            let Some(definition) = self.content.actor(&captured.kind_id) else {
                continue;
            };
            let interval = if definition.capture_policy == ActorCapturePolicyDefinition::PetOnly {
                CAPTURE_BALL_UNIQUE_REGEN_INTERVAL
            } else {
                CAPTURE_BALL_REGEN_INTERVAL
            };
            if !tick.is_multiple_of(interval) || captured.hp >= captured.max_hp {
                continue;
            }
            let mut recovered = captured.max_hp / 100;
            if recovered == 0 && self.rng.bounded(2) == 0 {
                recovered = 1;
            }
            if definition.regenerates {
                recovered = recovered.saturating_mul(2);
            }
            if recovered > 0 {
                let captured = self.items[index]
                    .captured_actor
                    .as_mut()
                    .expect("captured actor must remain available");
                captured.hp = captured.hp.saturating_add(recovered).min(captured.max_hp);
            }
        }
    }
}
