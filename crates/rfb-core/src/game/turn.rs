// SPDX-License-Identifier: MPL-2.0

use super::*;

impl Game {
    pub(super) fn advance_until_player_ready(
        &mut self,
        resting: bool,
        local_floor_active: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        loop {
            self.world_tick = self.world_tick.saturating_add(1);
            self.process_status_tick(events, changed, removed_entities, local_floor_active)?;
            if self.player_is_dead() {
                break;
            }
            self.process_hunger(events);
            if self.player_is_dead() {
                break;
            }
            self.process_natural_hp_regeneration(resting);
            if local_floor_active {
                self.process_monster_regeneration();
            }
            self.process_equipped_light_fuel(events);
            self.process_equipment_regeneration(events);
            self.process_inventory_device_recovery(events);
            if local_floor_active {
                if !self.current_floor_has_active_task() {
                    self.process_ambient_monster_allocation(changed)?;
                }
                self.process_monster_energy_pulse(events, changed, removed_entities)?;
            }
            if self.player_is_dead() {
                break;
            }
            let speed = derived_speed(&self.player_derived_stats().speed);
            gain_energy(&mut self.player.energy_need, speed);
            if self.player.energy_need <= 0 {
                break;
            }
        }
        if local_floor_active {
            self.advance_summon_lifetimes(events, changed, removed_entities);
        }
        if local_floor_active && !self.player_is_dead() {
            self.advance_recall(events, changed)?;
        }
        Ok(())
    }

    pub(super) fn process_natural_hp_regeneration(&mut self, resting: bool) {
        if self.wilderness_blocks_regeneration()
            || !self
                .world_tick
                .is_multiple_of(NATURAL_HP_REGENERATION_INTERVAL_TICKS)
            || self.player.hp >= self.effective_player_max_hp()
            || self
                .player
                .statuses
                .iter()
                .any(|status| status.kind_id == STATUS_POISON || status.kind_id == STATUS_BLEEDING)
        {
            return;
        }
        let period = u64::from(self.world_tick / NATURAL_HP_REGENERATION_INTERVAL_TICKS);
        let maximum = u64::try_from(self.effective_player_max_hp().max(1))
            .expect("positive maximum HP must fit u64");
        let factor = self.nutrition_regeneration_factor() * if resting { 2 } else { 1 };
        let regeneration = maximum
            .saturating_mul(factor)
            .saturating_add(NATURAL_HP_REGENERATION_BASE)
            .saturating_mul(self.player_regeneration_rate_percent())
            .saturating_div(100)
            .saturating_mul(self.mutation_regeneration_percent())
            .saturating_div(100);
        let recovered = period
            .saturating_mul(regeneration)
            .saturating_div(NATURAL_HP_REGENERATION_SCALE)
            .saturating_sub(
                period
                    .saturating_sub(1)
                    .saturating_mul(regeneration)
                    .saturating_div(NATURAL_HP_REGENERATION_SCALE),
            );
        if recovered == 0 {
            return;
        }
        let recovered = i32::try_from(recovered).unwrap_or(i32::MAX);
        self.player.hp = self
            .player
            .hp
            .saturating_add(recovered)
            .min(self.effective_player_max_hp());
    }

    pub(super) fn process_monster_regeneration(&mut self) {
        if !self
            .world_tick
            .is_multiple_of(MONSTER_REGENERATION_INTERVAL_TICKS)
        {
            return;
        }
        for index in 0..self.entities.len() {
            let actor = &self.entities[index];
            if actor.hp <= 0 || actor.hp >= actor.max_hp {
                continue;
            }
            let mut recovered = actor.max_hp / 100;
            if recovered == 0 && self.rng.bounded(2) == 0 {
                recovered = 1;
            }
            if self
                .actor_runtime_definition(actor)
                .is_some_and(|definition| definition.regenerates)
            {
                recovered = recovered.saturating_mul(2);
            }
            recovered = recovered.min(MONSTER_REGENERATION_MAXIMUM);
            self.entities[index].hp = self.entities[index]
                .hp
                .saturating_add(recovered)
                .min(self.entities[index].max_hp);
        }
    }

    fn process_equipment_regeneration(&mut self, events: &mut Vec<DomainEvent>) {
        if self.wilderness_blocks_regeneration()
            || !self
                .world_tick
                .is_multiple_of(EQUIPMENT_REGENERATION_INTERVAL_TICKS)
            || !self
                .player_equipment_passives()
                .contains(&EquipmentPassive::Regeneration)
        {
            return;
        }
        let maximum = self.effective_player_max_hp();
        let before = self.player.hp;
        self.player.hp = self.player.hp.saturating_add(1).min(maximum);
        let applied = self.player.hp.saturating_sub(before);
        if applied > 0 {
            events.push(DomainEvent::EquipmentRegenerated {
                resolution: HealingResolutionDto {
                    requested: 1,
                    applied,
                },
            });
        }
    }

    pub(super) fn process_inventory_device_recovery(&mut self, events: &mut Vec<DomainEvent>) {
        let world_tick = self.world_tick;
        let content = &self.content;
        for item in &mut self.items {
            if item.location != ItemLocation::Inventory {
                continue;
            }
            let Some(recovery) = content
                .item(&item.kind_id)
                .and_then(|definition| definition.device_generation.as_ref())
                .and_then(|generation| generation.recovery)
            else {
                continue;
            };
            if !world_tick.is_multiple_of(u32::from(recovery.interval_ticks)) {
                continue;
            }
            let Some(charges) = item.charges.as_mut() else {
                continue;
            };
            if charges.current >= charges.maximum {
                item.device_recovery_progress = 0;
                continue;
            }
            let scaled = u64::from(charges.maximum)
                .saturating_mul(u64::from(recovery.energy_per_mille))
                .saturating_add(u64::from(item.device_recovery_progress));
            let gain =
                u32::try_from(scaled / 1_000).expect("validated device recovery gain must fit u32");
            item.device_recovery_progress =
                u16::try_from(scaled % 1_000).expect("recovery remainder must fit u16");
            if gain == 0 {
                continue;
            }
            let before = charges.current;
            charges.current = charges.current.saturating_add(gain).min(charges.maximum);
            let applied = charges.current.saturating_sub(before);
            if charges.current == charges.maximum {
                item.device_recovery_progress = 0;
            }
            if applied > 0 {
                events.push(DomainEvent::DeviceEnergyRecovered {
                    target_item_id: item.id.clone(),
                    target_kind_id: item.kind_id.clone(),
                    amount: applied,
                    current: charges.current,
                    maximum: charges.maximum,
                });
            }
        }
    }

    fn advance_summon_lifetimes(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) {
        let mut entity_ids = self
            .entities
            .iter()
            .filter(|entity| entity.summon.is_some())
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        entity_ids.sort();
        for entity_id in entity_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id)
            else {
                continue;
            };
            let expires = self.entities[index]
                .summon
                .as_ref()
                .is_some_and(|summon| summon.remaining_turns <= 1);
            if expires {
                let position = self.entities[index].position;
                let target_kind_id = self.entities[index].kind_id.clone();
                let removed_id = self.entities[index].id.clone();
                self.entities.remove(index);
                if self.riding_actor_id.as_deref() == Some(removed_id.as_str()) {
                    self.riding_actor_id = None;
                }
                changed.insert(position);
                removed_entities.push(removed_id.clone());
                events.push(DomainEvent::SummonExpired {
                    entity_id: removed_id,
                    target_kind_id,
                });
            } else if let Some(summon) = self.entities[index].summon.as_mut() {
                summon.remaining_turns = summon.remaining_turns.saturating_sub(1);
            }
        }
    }

    pub(super) fn process_status_tick(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
        process_entities: bool,
    ) -> Result<(), CoreError> {
        let tsuyoshi_expiration = self
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_TSUYOSHI && status.remaining_ticks <= 1)
            .map(|status| {
                (
                    status
                        .source_id
                        .clone()
                        .unwrap_or_else(|| STATUS_TSUYOSHI.to_owned()),
                    self.effective_player_max_hp(),
                    self.player_resource_maxima(),
                )
            });
        let player_damage_percent = self.player_incoming_damage_percent();
        let player_tick = process_actor_status_tick(&mut self.player, false, player_damage_percent);
        let player_status_expired = !player_tick.expired.is_empty();
        let tsuyoshi_expired = player_tick
            .expired
            .iter()
            .any(|status_kind_id| status_kind_id == STATUS_TSUYOSHI);
        for damage in player_tick.damage {
            events.push(DomainEvent::PlayerStatusDamaged {
                status_kind_id: damage.status_kind_id,
                damage: damage.outcome,
            });
        }
        for status_kind_id in player_tick.expired {
            events.push(DomainEvent::PlayerStatusExpired { status_kind_id });
        }
        if let Some((source_kind_id, previous_max_hp, previous_resource_maxima)) =
            tsuyoshi_expiration.filter(|_| tsuyoshi_expired)
        {
            self.apply_tsuyoshi_crash(
                &source_kind_id,
                previous_max_hp,
                &previous_resource_maxima,
                events,
            );
        }
        if player_status_expired {
            self.refresh_player_resource_maxima();
        }
        self.clamp_player_hp_to_effective_max();
        if let Some(damage) = player_tick.fatal_damage {
            events.push(DomainEvent::PlayerDiedFromStatus {
                status_kind_id: damage.status_kind_id,
                damage: damage.outcome,
            });
            return Ok(());
        }
        if !process_entities {
            return Ok(());
        }

        let mut entity_ids = self
            .entities
            .iter()
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        entity_ids.sort();
        for entity_id in entity_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id)
            else {
                continue;
            };
            let target_kind_id = self.entities[index].kind_id.clone();
            let tick = process_actor_status_tick(&mut self.entities[index], true, 100);
            if tick.awakened {
                events.push(DomainEvent::EntityAwakened {
                    target_kind_id: target_kind_id.clone(),
                });
            }
            for damage in tick.damage {
                events.push(DomainEvent::EntityStatusDamaged {
                    target_kind_id: target_kind_id.clone(),
                    status_kind_id: damage.status_kind_id,
                    damage: damage.outcome,
                });
            }
            for status_kind_id in tick.expired {
                events.push(DomainEvent::EntityStatusExpired {
                    target_kind_id: target_kind_id.clone(),
                    status_kind_id,
                });
            }
            if let Some(damage) = tick.fatal_damage {
                self.resolve_actor_death(
                    index,
                    DomainEvent::EntityDiedFromStatus {
                        target_kind_id,
                        status_kind_id: damage.status_kind_id,
                        damage: damage.outcome,
                    },
                    events,
                    changed,
                    removed_entities,
                )?;
            }
        }
        Ok(())
    }

    pub(super) fn process_monster_energy_pulse(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let mut entity_ids = self
            .entities
            .iter()
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        entity_ids.sort();
        let mut surround_reservations = BTreeSet::new();

        for entity_id in entity_ids {
            if self.player_is_dead() {
                break;
            }
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id)
            else {
                continue;
            };
            if self.riding_actor_id.as_deref() == Some(entity_id.as_str()) {
                continue;
            }
            let definition = self
                .actor_runtime_definition(&self.entities[index])
                .expect("monster actor definition must remain available");
            let speed = derived_speed(
                &self
                    .actor_derived_stats(&self.entities[index], definition, false)
                    .speed,
            );
            gain_energy(&mut self.entities[index].energy_need, speed);
            if self.entities[index].energy_need > 0 {
                continue;
            }
            spend_energy(&mut self.entities[index].energy_need, STANDARD_ACTION_COST);
            if self.entities[index]
                .statuses
                .iter()
                .any(|status| status.kind_id == STATUS_SLEEP)
            {
                events.push(DomainEvent::MonsterSlept {
                    target_kind_id: self.entities[index].kind_id.clone(),
                });
                continue;
            }
            if self.try_original_reproduction(index, changed) {
                continue;
            }
            let floor_id = self.current_floor_id.clone();
            self.resolve_monster_action(
                index,
                events,
                changed,
                removed_entities,
                &mut surround_reservations,
            )?;
            if self.current_floor_id != floor_id {
                break;
            }
        }
        Ok(())
    }
}
