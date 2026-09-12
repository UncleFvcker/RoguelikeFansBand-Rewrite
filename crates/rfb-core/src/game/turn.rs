// SPDX-License-Identifier: MPL-2.0

use super::*;

impl Game {
    pub(super) fn process_monster_minor_slow_recovery(&mut self, index: usize) {
        let minor_slow = self.entities[index].minor_slow;
        if minor_slow == 0 {
            return;
        }
        let regenerates = self
            .actor_runtime_definition(&self.entities[index])
            .is_some_and(|definition| definition.regenerates);
        let denominator = if regenerates { 50 } else { 100 };
        if self.rng.bounded(denominator) < u64::from(minor_slow) {
            self.entities[index].minor_slow -= 1;
        }
    }

    pub(super) fn try_clear_monster_confusion(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let clears_confusion = self
            .actor_runtime_definition(&self.entities[index])
            .is_some_and(|definition| definition.tags.iter().any(|tag| tag == "clear-head"));
        if !clears_confusion || self.rng.bounded(4) != 0 {
            return false;
        }
        let before = self.entities[index].statuses.len();
        self.entities[index]
            .statuses
            .retain(|status| status.kind_id != STATUS_CONFUSION);
        if self.entities[index].statuses.len() == before {
            return false;
        }
        events.push(DomainEvent::EntityStatusExpired {
            target_kind_id: self.entities[index].kind_id.clone(),
            status_kind_id: STATUS_CONFUSION.to_owned(),
        });
        true
    }

    pub(super) fn quantum_slot_denominator(entity_id: &str) -> u64 {
        entity_id
            .bytes()
            .fold(14_695_981_039_346_656_037_u64, |hash, byte| {
                (hash ^ u64::from(byte)).wrapping_mul(1_099_511_628_211)
            })
            % 100
            + 10
    }

    pub(super) fn try_quantum_turn(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<bool, CoreError> {
        let Some(definition) = self.actor_runtime_definition(&self.entities[index]) else {
            return Ok(false);
        };
        if !definition.tags.iter().any(|tag| tag == "quantum") {
            return Ok(false);
        }
        let questor = definition
            .allocation
            .as_ref()
            .is_some_and(|allocation| allocation.task_id.is_some())
            || definition.tags.iter().any(|tag| tag == "guardian");
        if self.rng.bounded(2) == 0 {
            return Ok(true);
        }
        let denominator = Self::quantum_slot_denominator(&self.entities[index].id);
        if self.rng.bounded(denominator) != 0 || questor {
            return Ok(false);
        }
        let source_kind_id = self.entities[index].kind_id.clone();
        self.resolve_actor_death_without_credit(
            index,
            DomainEvent::MonsterQuantumVanished { source_kind_id },
            events,
            changed,
            removed_entities,
        )?;
        Ok(true)
    }

    pub(super) fn try_trump_blink(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let trump = self
            .actor_runtime_definition(&self.entities[index])
            .is_some_and(|definition| definition.tags.iter().any(|tag| tag == "trump"));
        if !trump || self.rng.bounded(2) != 0 {
            return;
        }
        if self.entities[index]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_SLEEP)
        {
            return;
        }
        let actor_id = self.entities[index].id.clone();
        let source_kind_id = self.entities[index].kind_id.clone();
        let from = self.entities[index].position;
        let destinations = self.open_positions_around_for_actor_kind(from, 5, &source_kind_id);
        if destinations.is_empty() {
            return;
        }
        let choice = usize::try_from(self.rng.bounded(destinations.len() as u64))
            .expect("trump destination index must fit usize");
        let to = destinations[choice];
        self.entities[index].position = to;
        changed.extend([from, to]);
        events.push(DomainEvent::MonsterBlinked {
            source_kind_id,
            resolution: MonsterDisplacementResolutionDto { actor_id, from, to },
        });
    }

    pub(super) fn advance_until_player_ready(
        &mut self,
        resting: bool,
        local_floor_active: bool,
        pet_neglect_allowed: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        loop {
            let visible_monster_auras_before_tick = self.visible_monster_aura_entity_ids();
            self.world_tick = self.world_tick.saturating_add(1);
            self.clear_daylight_suppression_at_dawn();
            let light_blocks_regeneration = self.process_vampire_light_damage(events);
            if self.player_is_dead() {
                break;
            }
            // RFB processes wall damage before the final tick of Wraith/Invulnerability expires.
            let wall_blocks_regeneration =
                local_floor_active && self.process_player_wall_damage(events);
            if self.player_is_dead() {
                break;
            }
            let waste_exposure = local_floor_active
                .then(|| self.process_player_waste_damage(events))
                .flatten();
            if self.player_is_dead() {
                break;
            }
            let water_lava_exposure =
                local_floor_active && self.process_player_interior_water_lava_damage(events);
            if self.player_is_dead() {
                break;
            }
            self.process_status_tick(events, changed, removed_entities, local_floor_active)?;
            if self.player_is_dead() {
                break;
            }
            // Source poison damage precedes FF_ACID. Newly accumulated poison starts
            // on the next status tick, rather than becoming a second immediate hit.
            if let Some((source, poison)) = &waste_exposure {
                self.apply_player_melee_status(STATUS_POISON, *poison, source);
            }
            self.process_hunger(events);
            if self.player_is_dead() {
                break;
            }
            if !wall_blocks_regeneration
                && !light_blocks_regeneration
                && waste_exposure.is_none()
                && !water_lava_exposure
            {
                self.process_natural_hp_regeneration(resting);
                self.process_equipment_regeneration(events);
            }
            self.process_fasting(events);
            self.process_minor_slow_recovery();
            if local_floor_active {
                self.process_monster_regeneration();
            }
            self.process_equipped_light_fuel(events);
            if local_floor_active {
                self.process_equipped_curse_effects(events, changed, removed_entities)?;
                if self.player_is_dead() {
                    return Ok(());
                }
            }
            self.process_periodic_mutations(
                local_floor_active,
                resting,
                events,
                changed,
                removed_entities,
            )?;
            if self.pending_mutation_direction.is_some() {
                return Ok(());
            }
            let ready = self.finish_world_tick_after_periodic_mutations(
                local_floor_active,
                pet_neglect_allowed,
                &visible_monster_auras_before_tick,
                events,
                changed,
                removed_entities,
            )?;
            if self.duelist_prompt().is_some() {
                self.continue_after_duelist_choice(
                    rfb_protocol::DuelistContinuationDto::WorldTick {
                        resting,
                        local_floor_active,
                        pet_neglect_allowed,
                    },
                );
                return Ok(());
            }
            if ready {
                break;
            }
        }
        self.finish_player_ready_advance(local_floor_active, events, changed, removed_entities)?;
        Ok(())
    }

    fn finish_world_tick_after_periodic_mutations(
        &mut self,
        local_floor_active: bool,
        pet_neglect_allowed: bool,
        visible_monster_auras_before_tick: &BTreeSet<String>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<bool, CoreError> {
        if self.player_is_dead() {
            return Ok(true);
        }
        self.process_inventory_device_recovery(events);
        self.process_class_item_sensing();
        self.process_captured_actor_regeneration();
        let reality_changed =
            local_floor_active && self.advance_reality_change(events, changed, removed_entities)?;
        if local_floor_active && !reality_changed {
            if !self.current_floor_has_active_task() {
                self.process_ambient_monster_allocation(changed)?;
            }
            self.resolve_newly_visible_monster_auras(
                visible_monster_auras_before_tick,
                events,
                changed,
            );
            self.process_monster_energy_pulse(
                pet_neglect_allowed,
                events,
                changed,
                removed_entities,
            )?;
            if self.duelist_prompt().is_some() {
                return Ok(false);
            }
        }
        if self.player_is_dead() {
            return Ok(true);
        }
        let speed = derived_speed(&self.player_derived_stats().speed);
        gain_energy(&mut self.player.energy_need, speed);
        Ok(self.player.energy_need <= 0)
    }

    fn finish_player_ready_advance(
        &mut self,
        local_floor_active: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        if local_floor_active {
            self.advance_summon_lifetimes(events, changed, removed_entities);
        }
        if local_floor_active && !self.player_is_dead() {
            self.advance_recall(events, changed)?;
        }
        Ok(())
    }

    pub(super) fn resume_duelist_world_tick(
        &mut self,
        resting: bool,
        local_floor_active: bool,
        pet_neglect_allowed: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        if !self.player_is_dead() {
            let speed = derived_speed(&self.player_derived_stats().speed);
            gain_energy(&mut self.player.energy_need, speed);
        }
        if self.player_is_dead() || self.player.energy_need <= 0 {
            self.finish_player_ready_advance(local_floor_active, events, changed, removed_entities)
        } else {
            self.advance_until_player_ready(
                resting,
                local_floor_active,
                pet_neglect_allowed,
                events,
                changed,
                removed_entities,
            )
        }
    }

    pub(super) fn resume_after_periodic_mutation(
        &mut self,
        pending: PendingMutationDirectionDto,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let local_floor_active = self.map_scale != MapScaleDto::World;
        self.resume_periodic_mutations(&pending, events, changed, removed_entities)?;
        if self.pending_mutation_direction.is_some() {
            return Ok(());
        }
        let visible_monster_auras = self.visible_monster_aura_entity_ids();
        let ready = self.finish_world_tick_after_periodic_mutations(
            local_floor_active,
            self.pet_upkeep().unsafe_warning(),
            &visible_monster_auras,
            events,
            changed,
            removed_entities,
        )?;
        if self.duelist_prompt().is_some() {
            self.continue_after_duelist_choice(rfb_protocol::DuelistContinuationDto::WorldTick {
                resting: pending.resting,
                local_floor_active,
                pet_neglect_allowed: self.pet_upkeep().unsafe_warning(),
            });
            return Ok(());
        }
        if ready {
            self.finish_player_ready_advance(local_floor_active, events, changed, removed_entities)
        } else {
            self.advance_until_player_ready(
                pending.resting,
                local_floor_active,
                self.pet_upkeep().unsafe_warning(),
                events,
                changed,
                removed_entities,
            )
        }
    }

    pub(super) fn process_vampire_light_damage(&mut self, events: &mut Vec<DomainEvent>) -> bool {
        if !self
            .world_tick
            .is_multiple_of(NATURAL_HP_REGENERATION_INTERVAL_TICKS)
            || !self.player_is_vampire()
            || self.player_resistance_percent(DamageType::Light) >= 0
        {
            return false;
        }
        let invulnerable = self.player_has_status_kind(STATUS_INVULNERABILITY);
        let sunlight = !invulnerable
            && self.floor_has_environment_light()
            && self.wilderness_is_daytime()
            && self.ambient_light(self.player.position, &self.collect_light_sources()) > 0;
        // RFB tests the equipped light's darkness flag, even when its fuel is exhausted.
        let burning_light = self
            .items
            .iter()
            .find(|item| {
                matches!(&item.location, ItemLocation::Equipped { .. })
                    && self.content.item(&item.kind_id).is_some_and(|definition| {
                        definition.equipment_slot.as_deref() == Some("light")
                    })
                    && !self.item_has_darkness(item)
            })
            .map(|item| item.kind_id.clone());
        let blocks_regeneration = sunlight || burning_light.is_some();
        let sources = sunlight
            .then_some(None)
            .into_iter()
            .chain(burning_light.filter(|_| !invulnerable).map(Some));
        for source_kind_id in sources {
            let damage = resolve_damage(
                DamagePacket::new(1, DamageType::Light),
                ResistanceLevel::Normal,
            );
            let application = self.apply_final_player_damage(damage, FatalityPolicy::BelowZero);
            events.push(DomainEvent::PlayerLightDamaged {
                source_kind_id,
                damage: application.damage,
            });
            if application.fatal {
                events.push(DomainEvent::PlayerDiedFromLight {
                    damage: application.damage,
                });
                break;
            }
        }
        blocks_regeneration
    }

    pub(super) fn process_player_wall_damage(&mut self, events: &mut Vec<DomainEvent>) -> bool {
        if !self
            .world_tick
            .is_multiple_of(NATURAL_HP_REGENERATION_INTERVAL_TICKS)
            || self.map_scale != MapScaleDto::Local
            || self.player_has_status_kind(STATUS_INVULNERABILITY)
            || self.player_has_status_kind(STATUS_WRAITHFORM)
        {
            return false;
        }
        let terrain = self
            .content
            .terrain(
                &self.terrain[self
                    .index(self.player.position)
                    .expect("player position must remain in bounds")],
            )
            .expect("player terrain must exist");
        // FF_CAN_FLY exempts the terrain, independently of whether the player can fly.
        if terrain.walkable
            || terrain
                .movement_modes
                .contains(&rfb_content::ActorMovementMode::Fly)
        {
            return false;
        }
        let terrain_id = terrain.id.clone();
        let crushing = !self.player_can_pass_walls();
        if !crushing
            && self
                .player_equipment_passives()
                .contains(&EquipmentPassive::NoPasswallDamage)
        {
            return false;
        }
        let mut raw_damage = 1 + i32::from(self.progress.level / 5);
        if !crushing
            && self
                .build
                .as_ref()
                .is_some_and(|build| build.race_id == "rfb-legacy.race.spectre")
        {
            raw_damage = raw_damage.min(self.player.hp);
        }
        if raw_damage <= 0 {
            return false;
        }
        // The native-race cap precedes take_hit; preserve its shared Transcendence SP payment.
        let damage = resolve_damage(
            DamagePacket::new(raw_damage, DamageType::Physical),
            ResistanceLevel::Normal,
        );
        let application = self.apply_final_player_damage(damage, FatalityPolicy::BelowZero);
        events.push(DomainEvent::PlayerWallDamaged {
            crushing,
            damage: application.damage,
        });
        if application.fatal {
            events.push(DomainEvent::PlayerDied {
                source_kind_id: terrain_id,
                method_id: None,
                damage: application.damage,
            });
        }
        true
    }

    pub(super) fn process_player_interior_water_lava_damage(
        &mut self,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        // Keep wilderness's existing action-time exposure. Interior floors use the
        // same ten-tick world interval as wall/waste damage, including while resting.
        if self.map_scale != MapScaleDto::Local
            || self.is_wilderness_floor()
            || !self
                .world_tick
                .is_multiple_of(NATURAL_HP_REGENERATION_INTERVAL_TICKS)
        {
            return false;
        }
        let terrain = self
            .content
            .terrain(
                &self.terrain[self
                    .index(self.player.position)
                    .expect("player position must remain in bounds")],
            )
            .expect("player terrain must exist");
        let lava = terrain.tags.iter().any(|tag| tag == "lava");
        let water = terrain.tags.iter().any(|tag| tag == "water");
        let deep = terrain.tags.iter().any(|tag| tag == "deep");
        let terrain_id = terrain.id.clone();
        let flying = self.active_traveler_has_mode(rfb_content::ActorMovementMode::Fly);
        let damage_type;
        let amount;
        if lava && !self.player_has_status_kind(STATUS_INVULNERABILITY) {
            let base = if deep {
                6000 + self.rng.bounded(4000) as i32
            } else if !flying {
                3000 + self.rng.bounded(2000) as i32
            } else {
                return false;
            };
            // dungeon.c FF_LAVA resists hundredths of HP before levitation and rounding.
            let resisted = self
                .resist_player_damage(resolve_damage(
                    DamagePacket::new(base, DamageType::Fire),
                    self.effective_player_resistances().level(DamageType::Fire),
                ))
                .applied;
            let resisted = if flying { resisted / 5 } else { resisted };
            if resisted == 0 {
                return false;
            }
            amount = resisted / 100 + i32::from(self.rng.bounded(100) < (resisted % 100) as u64);
            damage_type = DamageType::Fire;
        } else if water
            && deep
            && !flying
            && !self.active_traveler_has_mode(rfb_content::ActorMovementMode::Swim)
            && !self.active_traveler_has_mode(rfb_content::ActorMovementMode::Aquatic)
            && self.carried_weight_tenths_pound() > self.player_carry_capacity_tenths_pound()
        {
            amount = self.roll_damage(1, self.progress.level.max(1));
            damage_type = DamageType::Water;
        } else {
            return false;
        }
        let application = self.apply_final_player_damage(
            resolve_damage(
                DamagePacket::new(amount, damage_type),
                ResistanceLevel::Normal,
            ),
            FatalityPolicy::BelowZero,
        );
        events.push(DomainEvent::WildernessTerrainDamaged {
            terrain_id: terrain_id.clone(),
            damage: application.damage,
        });
        if application.fatal {
            events.push(DomainEvent::PlayerDied {
                source_kind_id: terrain_id,
                method_id: None,
                damage: application.damage,
            });
        }
        true
    }

    pub(super) fn process_player_waste_damage(
        &mut self,
        events: &mut Vec<DomainEvent>,
    ) -> Option<(String, i32)> {
        // One source process_world interval maps to the existing ten core world ticks.
        // Like wall damage, this runs before the final tick of Invulnerability expires.
        if !self
            .world_tick
            .is_multiple_of(NATURAL_HP_REGENERATION_INTERVAL_TICKS)
            || self.map_scale != MapScaleDto::Local
            || self.player_has_status_kind(STATUS_INVULNERABILITY)
        {
            return None;
        }
        let terrain = self
            .content
            .terrain(
                &self.terrain[self
                    .index(self.player.position)
                    .expect("player position must remain in bounds")],
            )
            .expect("player terrain must exist");
        if !terrain.tags.iter().any(|tag| tag == "acid") || self.rng.bounded(3) == 0 {
            return None;
        }
        let deep = terrain.tags.iter().any(|tag| tag == "deep");
        let terrain_id = terrain.id.clone();
        let flying = if self.riding_actor_id.is_some() {
            self.active_traveler_definition()
                .movement
                .modes
                .contains(&rfb_content::ActorMovementMode::Fly)
        } else {
            self.player_levitates()
        };
        let base = if deep {
            1400 + self.rng.bounded(800) as i32
        } else if !flying {
            700 + self.rng.bounded(400) as i32
        } else {
            0
        };
        let base = if flying {
            base / if deep { 15 } else { 10 }
        } else {
            base
        };
        // Poison starts from the pre-acid-resistance amount, as in dungeon.c FF_ACID.
        let resisted = |amount, damage_type| {
            self.resist_player_damage(resolve_damage(
                DamagePacket::new(amount, damage_type),
                self.effective_player_resistances().level(damage_type),
            ))
            .applied
        };
        let acid = resisted(base, DamageType::Acid);
        let poison = resisted(base * 6 / 5, DamageType::Poison);
        if acid == 0 && poison == 0 {
            return None;
        }
        // Both rounding draws occur even if one component is zero.
        let mut acid = acid / 100 + i32::from(self.rng.bounded(100) < (acid % 100) as u64);
        let poison = poison / 100 + i32::from(self.rng.bounded(100) < (poison % 100) as u64);
        if acid > 0 && self.rng.bounded(16) == 0 && self.corrode_player_armor(events) {
            acid = (acid + 1) / 2;
        }
        if acid > 0 || poison > 0 {
            // Resistance has already been applied. Use the shared final damage boundary.
            let damage = resolve_damage(
                DamagePacket::new(acid, DamageType::Acid),
                ResistanceLevel::Normal,
            );
            let application = self.apply_final_player_damage(damage, FatalityPolicy::BelowZero);
            events.push(DomainEvent::PlayerWasteDamaged {
                terrain_id: terrain_id.clone(),
                flying,
                damage: application.damage,
            });
            if application.fatal {
                events.push(DomainEvent::PlayerDied {
                    source_kind_id: terrain_id.clone(),
                    method_id: None,
                    damage: application.damage,
                });
            }
        }
        if self.rng.bounded(32) == 0
            && self.rng.bounded(55) as i32 >= self.player_resistance_percent(DamageType::Poison)
        {
            self.resolve_monster_attribute_drain(AttributeKind::Constitution);
        }
        // Source suppresses HP recovery even when both fractional amounts round down.
        Some((terrain_id, poison))
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
            || !self.world_tick.is_multiple_of(
                EQUIPMENT_REGENERATION_INTERVAL_TICKS
                    * if self.player_has_equipped_curse_effect(ItemCurseEffectDto::SlowRegeneration)
                    {
                        5
                    } else {
                        1
                    },
            )
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
        let mut recovery_order = self
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let rate = matches!(item.location, ItemLocation::Absorbed { .. }).then(|| {
                    if world_tick.is_multiple_of(10)
                        && item.charges.is_some_and(|sp| sp.current < sp.maximum)
                    {
                        self.absorbed_device_recovery_per_mille(item)
                    } else {
                        // Off-period/full devices still follow the same cleanup and ordering.
                        0
                    }
                });
                (index, rate)
            })
            .collect::<Vec<_>>();
        // Saves group containers separately; body recovery must keep source category/slot order.
        recovery_order.sort_by_key(|(index, _)| match self.items[*index].location {
            ItemLocation::Absorbed { category, slot } => Some((category, slot)),
            _ => None,
        });
        let content = &self.content;
        for (index, body_rate) in recovery_order {
            let item = &mut self.items[index];
            if item.location == ItemLocation::Inventory && item.is_artifact_mushroom(content) {
                item.device_recovery_progress = item.device_recovery_progress.saturating_sub(1);
                continue;
            }
            if !matches!(
                item.location,
                ItemLocation::Inventory
                    | ItemLocation::Equipped { .. }
                    | ItemLocation::Absorbed { .. }
            ) {
                continue;
            }
            // Equipment activation timeouts recover only while worn; absorbed devices use body rates.
            if item.activation.is_some()
                && item.location == ItemLocation::Inventory
                && content
                    .item(&item.kind_id)
                    .is_some_and(|kind| kind.equipment_slot.is_some())
            {
                continue;
            }
            let Some(recovery) = body_rate
                .map(
                    |energy_per_mille| rfb_content::ItemDeviceRecoveryDefinition {
                        interval_ticks: 10,
                        energy_per_mille,
                    },
                )
                .or_else(|| {
                    item_device_generation(
                        content,
                        &item.kind_id,
                        &item.affix_ids,
                        item.activation
                            .as_ref()
                            .map(|activation| activation.profile_id.as_str()),
                        item.artifact_name.is_some(),
                    )
                    .and_then(|generation| {
                        item.activation
                            .as_ref()
                            .and_then(|activation| {
                                generation
                                    .activations
                                    .iter()
                                    .find(|profile| profile.id == activation.profile_id)
                            })
                            .and_then(|profile| profile.recovery)
                            .or(generation.recovery)
                    })
                })
            else {
                continue;
            };
            let Some(charges) = item.charges.as_ref() else {
                continue;
            };
            if charges.current >= charges.maximum {
                item.device_recovery_progress = 0;
                continue;
            }
            let regeneration =
                if body_rate.is_none() && super::ego::item_has_ego(content, item, 252) {
                    1 + super::ego::device_pval(item)
                } else {
                    1
                };
            let charges = item
                .charges
                .as_mut()
                .expect("recovering device retains charges");
            // Single-charge activations recover after their own elapsed cooldown.
            let gain = if charges.maximum == 1 && recovery.energy_per_mille == 1_000 {
                item.device_recovery_progress += 1;
                if item.device_recovery_progress < recovery.interval_ticks {
                    continue;
                }
                1
            } else {
                if !world_tick.is_multiple_of(u32::from(recovery.interval_ticks)) {
                    continue;
                }
                let source_device = body_rate.is_some()
                    || content
                        .item(&item.kind_id)
                        .and_then(|definition| definition.device_generation.as_ref())
                        .is_some_and(|generation| generation.rfb_device.is_some());
                let mut scaled = u64::from(charges.maximum)
                    .saturating_mul(u64::from(recovery.energy_per_mille))
                    .saturating_mul(u64::from(regeneration));
                if source_device {
                    // devices.c stores hundredths of SP and stochastically
                    // rounds the next hundredth. Reuse our saved thousandths.
                    scaled *= 100;
                    scaled =
                        (scaled / 1000 + u64::from(self.rng.bounded(1000) < scaled % 1000)) * 10;
                }
                scaled += u64::from(item.device_recovery_progress);
                let gain = u32::try_from(scaled / 1_000)
                    .expect("validated device recovery gain must fit u32");
                item.device_recovery_progress =
                    u16::try_from(scaled % 1_000).expect("recovery remainder must fit u16");
                gain
            };
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
                self.clear_duelist_challenge_for(&removed_id);
                if self.riding_actor_id.as_deref() == Some(removed_id.as_str()) {
                    self.riding_actor_id = None;
                }
                self.clear_riding_bond_for(&removed_id);
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
        let nonliving = self.player_is_nonliving();
        let berserker = self.player_is_berserker();
        let no_stun = berserker && self.progress.level >= 35;
        if nonliving || berserker {
            self.player.statuses.retain(|status| {
                if (nonliving && matches!(status.kind_id.as_str(), STATUS_BLEEDING | STATUS_UNWELL))
                    || (berserker
                        && matches!(status.kind_id.as_str(), STATUS_FEAR | STATUS_PARALYSIS))
                    || (no_stun && status.kind_id == STATUS_STUN)
                {
                    events.push(DomainEvent::PlayerStatusExpired {
                        status_kind_id: status.kind_id.clone(),
                    });
                    false
                } else {
                    true
                }
            });
        }
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
        let player_race_status_expiring =
            self.player.statuses.iter().any(|status| {
                status.kind_id == STATUS_PLAYER_POLYMORPH && status.remaining_ticks <= 1
            });
        let invulnerability_expiring =
            self.player.statuses.iter().any(|status| {
                status.kind_id == STATUS_INVULNERABILITY && status.remaining_ticks <= 1
            });
        let player_damage_percent = self.player_incoming_damage_percent();
        let ignores_suffocation = self.player_is_nonliving();
        // The current status model recovers one wound tick per turn. Apply
        // dungeon.c's (recovery + game_turn % 3) / 3 without slowing its damage.
        if self.player_has_equipped_curse_effect(ItemCurseEffectDto::OpenWounds)
            && self.world_tick % 3 != 2
        {
            for status in &mut self.player.statuses {
                if status.kind_id == STATUS_BLEEDING {
                    status.remaining_ticks = status.remaining_ticks.saturating_add(1);
                }
            }
        }
        let transcendence = self.player_has_status_kind(STATUS_TRANSCENDENCE);
        let mut mana = self.resources.get_mut("demo.resource.mana");
        let player_tick = process_actor_status_tick_with(
            &mut self.player,
            false,
            player_damage_percent,
            ignores_suffocation,
            |player, damage, fatality_policy| {
                commit_final_player_damage(
                    player,
                    mana.as_deref_mut(),
                    transcendence,
                    damage,
                    fatality_policy,
                )
            },
        );
        let player_status_expired = !player_tick.expired.is_empty();
        let tsuyoshi_expired = player_tick
            .expired
            .iter()
            .any(|status_kind_id| status_kind_id == STATUS_TSUYOSHI);
        for damage in player_tick.damage {
            if damage.outcome.applied > 0 {
                self.fishing_direction = None;
            }
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
        if player_race_status_expiring {
            self.reconcile_player_body_slots_for_current_form();
        }
        if invulnerability_expiring {
            spend_energy(&mut self.player.energy_need, STANDARD_ACTION_COST);
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
        pet_neglect_allowed: bool,
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
        self.continue_monster_energy_pulse(
            entity_ids,
            BTreeSet::new(),
            pet_neglect_allowed,
            events,
            changed,
            removed_entities,
        )
    }

    pub(super) fn continue_monster_energy_pulse(
        &mut self,
        entity_ids: Vec<String>,
        mut surround_reservations: BTreeSet<Position>,
        pet_neglect_allowed: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let mut entity_ids = entity_ids.into_iter();
        while let Some(entity_id) = entity_ids.next() {
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
            self.process_monster_minor_slow_recovery(index);
            if self.resolve_neglected_pet(
                index,
                pet_neglect_allowed,
                events,
                changed,
                removed_entities,
            ) {
                continue;
            }
            self.try_trump_blink(index, events, changed);
            if self.try_quantum_turn(index, events, changed, removed_entities)? {
                continue;
            }
            self.try_clear_monster_confusion(index, events);
            self.wake_monster_for_equipped_aggravation(index, events, changed);
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
            let visible_monster_auras_before_action = self.visible_monster_aura_entity_ids();
            self.resolve_monster_action(
                index,
                events,
                changed,
                removed_entities,
                &mut surround_reservations,
            )?;
            if self.duelist_prompt().is_some() {
                self.continue_after_duelist_choice(
                    rfb_protocol::DuelistContinuationDto::MonsterPulse {
                        remaining_entity_ids: entity_ids.collect(),
                        floor_id,
                        surround_reservations: surround_reservations.into_iter().collect(),
                        visible_auras_before: visible_monster_auras_before_action
                            .into_iter()
                            .collect(),
                        pet_neglect_allowed,
                    },
                );
                break;
            }
            self.resolve_newly_visible_monster_auras(
                &visible_monster_auras_before_action,
                events,
                changed,
            );
            if self.current_floor_id != floor_id {
                break;
            }
        }
        Ok(())
    }
}
