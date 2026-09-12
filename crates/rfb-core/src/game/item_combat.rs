// SPDX-License-Identifier: MPL-2.0

use super::ability_scaling::device_power_value;
use super::item_use::ItemUsePlan;
use super::projectile_geometry::rfb_area_damage;
use super::*;

impl Game {
    pub(super) fn item_is_death_scythe(&self, item: &ItemInstance) -> bool {
        self.content
            .item(&item.kind_id)
            .and_then(|kind| kind.rfb_base_kind)
            .is_some_and(|base| (base.tval, base.sval) == (22, 50))
    }

    /// master a0d92b6378, cmd1.c::death_scythe_miss. None is HAND_NONE on return.
    pub(super) fn resolve_death_scythe_backlash(
        &mut self,
        item: &ItemInstance,
        hand: Option<&super::player_stats::ResolvedAttackProfile>,
        events: &mut Vec<DomainEvent>,
    ) {
        if self.player_is_dead() {
            return;
        }
        let weapon = self
            .item_throw_profile(item)
            .expect("death scythe weapon profile");
        let (dice, sides, to_damage) = hand.map_or(
            (weapon.damage.dice, weapon.damage.sides, weapon.to_damage),
            |profile| (profile.damage_dice, profile.damage_sides, profile.to_damage),
        );
        let mut damage = self.roll_damage(dice, sides);
        // Granted race identities already distinguish ordinary races from the
        // source mimic forms (1000+); an arbitrary form is not its birth race.
        let race = self
            .character_definitions()
            .and_then(|(_, race, _, _)| race.legacy_index);
        let mut multiplier = match race {
            Some(0 | 2 | 8 | 10 | 15 | 16 | 29 | 33 | 67) => 25,
            Some(
                1 | 6 | 7 | 11..=14 | 20 | 22 | 24..=27 | 32 | 40 | 41 | 57 | 63 | 1000..=1002,
            ) => 30,
            _ => 10,
        };
        if self.player_alignment() < 0 {
            multiplier = multiplier.max(20);
        }
        for element in [
            DamageType::Acid,
            DamageType::Electricity,
            DamageType::Fire,
            DamageType::Cold,
            DamageType::Poison,
        ] {
            // res_save_default draws first, including at multiplier >=25 and immunity.
            let saved = (self.rng.bounded(55) as i32) < self.player_resistance_percent(element);
            if !saved {
                multiplier = multiplier.max(25);
            }
        }
        if (self.item_has_weapon_trait(item, WeaponTraitDto::ManaBrand)
            || self.player_has_status_kind(STATUS_MANA_BRAND))
            && let Some(resource_id) = self
                .casting_profile()
                .map(|profile| profile.resource_id.clone())
            && let Some(pool) = self.resources.get_mut(&resource_id)
            && pool.current > pool.maximum / 30
        {
            pool.current -= 1 + pool.maximum / 30;
            multiplier = multiplier * 3 / 2 + 15;
        }
        damage = damage.saturating_mul(multiplier) / 10;
        let weight = self.item_instance_weight(item);
        let hold = crate::stats::strength_hold_pounds(self.effective_player_attributes().strength)
            * if self.weapon_uses_two_hands(item) {
                2
            } else {
                1
            };
        let hand_hit = hand.map_or(0, |profile| {
            profile.to_hit - weapon.to_hit
                + self.player_attribute_to_hit()
                + self
                    .weapon_proficiency_hit_modifier(&item.kind_id)
                    .map_or(0, |(_, bonus)| bonus / 3)
                + 2 * (i32::from(hold) - i32::from(weight / 10)).min(0)
                + i32::from(self.player_has_status_kind("rfb.status.blessed")) * 10
                + i32::from(self.player_has_status_kind("rfb.status.hero")) * 12
                + i32::from(self.player_has_status_kind(STATUS_BERSERK)) * 24
        });
        let two_hands = hand.is_some()
            && self.weapon_uses_two_hands(item)
            && !self.player_is_duelist()
            && crate::stats::strength_hold_pounds(self.effective_player_attributes().strength) * 2
                >= weight / 5;
        let chance = i32::from(weight)
            + hand_hit * 3
            + weapon.to_hit * 5
            + i32::from(self.progress.level) * 3;
        if (self.rng.bounded(if two_hands { 4000 } else { 5000 }) + 1) as i32 <= chance {
            let quality = u64::from(weight) + self.rng.bounded(650) + 1;
            let critical = match quality {
                0..=399 => 200,
                400..=699 => 250,
                700..=899 => 300,
                900..=1299 => 350,
                _ => 400,
            };
            damage = damage.saturating_mul(critical) / 100;
        }
        if self.rng.bounded(6) == 0 {
            let mut vorpal = 2;
            while self.rng.bounded(4) == 0 {
                vorpal += 1;
            }
            damage = damage.saturating_mul(vorpal);
        }
        damage = damage.saturating_add(to_damage).max(0);
        // DAMAGE_FORCE bypasses invulnerability/wraith reduction, but still
        // uses the common transcendence, HP and below-zero death path.
        self.resolve_item_life_loss(&item.kind_id, damage as u32, events);
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_item_bladeturner(
        &mut self,
        source_kind_id: String,
        profile_id: Option<String>,
        plan: ItemUsePlan,
        device_power_bonus: i32,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let ItemUsePlan::Projectile { path } = plan else {
            unreachable!("Bladeturner requires a projectile plan")
        };
        let profile_id = profile_id.expect("Bladeturner activation must carry a profile ID");
        self.mark_item_aware(&source_kind_id);
        self.resolve_player_area_damage_with_base_policy(
            &profile_id,
            path,
            true,
            DamageType::Physical,
            4,
            None,
            i32::try_from(device_power_value(300, device_power_bonus))
                .expect("device-powered missile damage must fit i32"),
            true,
            true,
            true,
            events,
            changed,
            removed_entities,
        )?;
        // EFFECT_BLADETURNER rolls once, after fire_ball and its consumers.
        let duration = u32::try_from(device_power_value(
            u64::try_from(50 + self.roll_damage(1, 50)).expect("positive duration"),
            device_power_bonus,
        ))
        .expect("device-powered duration must fit u32");
        if self.player.hp > 0 {
            self.resolve_item_heroism(
                &source_kind_id,
                0,
                0,
                duration,
                AbilityStatusStackingDefinition::KeepStrongest,
                events,
            );
            self.resolve_item_blessing(
                &source_kind_id,
                0,
                0,
                duration,
                AbilityStatusStackingDefinition::KeepStrongest,
                events,
            );
            self.resolve_item_basic_resistance(&source_kind_id, 0, 0, duration, events);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_item_activation_area_damage(
        &mut self,
        source_kind_id: String,
        profile_id: Option<String>,
        effect: ItemUseEffectDefinition,
        plan: ItemUsePlan,
        device_power_bonus: i32,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let ItemUseEffectDefinition::AreaDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            radius,
        } = effect
        else {
            unreachable!("item area-damage executor requires an area-damage effect")
        };
        let ItemUsePlan::Projectile { path } = plan else {
            unreachable!("item area-damage executor requires a projectile plan")
        };
        let profile_id =
            profile_id.expect("dynamic area-damage activation must carry a profile ID");
        let raw_damage = i32::try_from(device_power_value(
            u64::try_from(
                self.roll_damage(damage_dice, damage_sides)
                    .saturating_add(i32::from(damage_bonus))
                    .max(0),
            )
            .expect("non-negative device area damage must fit u64"),
            device_power_bonus,
        ))
        .expect("device-powered area damage must fit i32");
        self.mark_item_aware(&source_kind_id);
        self.resolve_player_area_damage_with_base(
            &profile_id,
            path,
            true,
            DamageType::from(damage_type),
            radius,
            None,
            raw_damage,
            true,
            events,
            changed,
            removed_entities,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_item_activation_beam_damage(
        &mut self,
        source_kind_id: String,
        profile_id: Option<String>,
        effect: ItemUseEffectDefinition,
        plan: ItemUsePlan,
        device_power_bonus: i32,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let ItemUseEffectDefinition::BeamDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
        } = effect
        else {
            unreachable!("item activation beam executor requires a beam damage effect")
        };
        let ItemUsePlan::Projectile { path } = plan else {
            unreachable!("item activation beam executor requires a projectile plan")
        };
        let profile_id = profile_id.expect("dynamic beam activation must carry a profile ID");
        let damage_type = DamageType::from(damage_type);
        let (trace, _) =
            self.trace_projectile_path_with_damage_policy(path, false, Some(damage_type));
        let affected_positions = trace.traversed.clone();
        self.resolve_projectile_terrain_effects(&affected_positions, damage_type, changed);
        self.resolve_ground_item_projectile_effects(
            &source_kind_id,
            &affected_positions,
            damage_type,
            true,
            events,
            changed,
            removed_entities,
        );
        let targets = self.beam_damage_targets(&affected_positions);
        changed.extend(affected_positions.iter().copied());
        self.mark_item_aware(&source_kind_id);
        if targets.is_empty() {
            events.push(DomainEvent::ItemActivationLanded {
                source_kind_id,
                profile_id,
                trace,
            });
            return Ok(());
        }

        let raw_damage = i32::try_from(device_power_value(
            u64::try_from(
                self.roll_damage(damage_dice, damage_sides)
                    .saturating_add(i32::from(damage_bonus))
                    .max(0),
            )
            .expect("non-negative device beam damage must fit u64"),
            device_power_bonus,
        ))
        .expect("device-powered beam damage must fit i32");
        for entity_id in targets {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            let definition = self
                .actor_runtime_definition(&self.entities[index])
                .expect("activation target definition must remain available");
            let target_kind_id = definition.id.clone();
            let target_position = self.entities[index].position;
            let target_stats = self.actor_derived_stats(&self.entities[index], definition, false);
            let resistance = self.entities[index].resistances.level(damage_type);
            let damage = resolve_armored_damage(
                raw_damage,
                damage_type,
                target_stats.armor_class.value,
                resistance,
            );
            self.entities[index].alerted = true;
            let application = plan_damage_application(
                &self.entities[index],
                damage,
                FatalityPolicy::AtOrBelowZero,
            );
            commit_damage_application(&mut self.entities[index], &application);
            changed.insert(target_position);
            self.wake_entity_after_damage(index, damage.applied, events);
            if !application.fatal {
                self.resolve_monster_fear_aura(index, "hurt", true, events);
            }
            if application.fatal {
                self.resolve_actor_death(
                    index,
                    DomainEvent::ItemActivationSlew {
                        source_kind_id: source_kind_id.clone(),
                        profile_id: profile_id.clone(),
                        target_kind_id,
                        damage,
                        trace: trace.clone(),
                    },
                    events,
                    changed,
                    removed_entities,
                )?;
            } else {
                events.push(DomainEvent::ItemActivationHit {
                    source_kind_id: source_kind_id.clone(),
                    profile_id: profile_id.clone(),
                    target_kind_id,
                    damage,
                    trace: trace.clone(),
                });
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_item_random_element_cone_damage(
        &mut self,
        source_kind_id: String,
        profile_id: Option<String>,
        effect: ItemUseEffectDefinition,
        plan: ItemUsePlan,
        device_power_bonus: i32,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let ItemUseEffectDefinition::RandomElementConeDamage {
            damage,
            damage_types,
            radius,
        } = effect
        else {
            unreachable!("item cone executor requires a random elemental cone effect")
        };
        let ItemUsePlan::Cone {
            path,
            direction,
            radius: planned_radius,
        } = plan
        else {
            unreachable!("item cone executor requires a cone plan")
        };
        debug_assert_eq!(radius, planned_radius);
        let profile_id = profile_id.expect("dynamic cone activation must carry a profile ID");
        let choice = usize::try_from(self.rng.bounded(
            u64::try_from(damage_types.len()).expect("validated damage type count must fit u64"),
        ))
        .expect("random damage type index must fit usize");
        let damage_type = DamageType::from(damage_types[choice]);
        let (trace, _) =
            self.trace_projectile_path_with_damage_policy(path, false, Some(damage_type));
        let (affected_positions, targets) =
            self.cone_damage_targets(&trace.traversed, direction, radius, damage_type);
        self.resolve_projectile_terrain_effects(&affected_positions, damage_type, changed);
        self.resolve_ground_item_projectile_effects(
            &source_kind_id,
            &affected_positions,
            damage_type,
            true,
            events,
            changed,
            removed_entities,
        );
        changed.extend(affected_positions);
        self.mark_item_aware(&source_kind_id);
        if targets.is_empty() {
            events.push(DomainEvent::ItemActivationLanded {
                source_kind_id,
                profile_id,
                trace,
            });
            return Ok(());
        }

        let base_raw_damage =
            i32::try_from(device_power_value(u64::from(damage), device_power_bonus))
                .expect("device-powered cone damage must fit i32");
        for (entity_id, lateral_distance) in targets {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            let definition = self
                .actor_runtime_definition(&self.entities[index])
                .expect("activation target definition must remain available");
            let target_kind_id = definition.id.clone();
            let target_position = self.entities[index].position;
            let target_stats = self.actor_derived_stats(&self.entities[index], definition, false);
            let resistance = self.entities[index].resistances.level(damage_type);
            let damage = resolve_armored_damage(
                rfb_area_damage(base_raw_damage, lateral_distance),
                damage_type,
                target_stats.armor_class.value,
                resistance,
            );
            self.entities[index].alerted = true;
            let application = plan_damage_application(
                &self.entities[index],
                damage,
                FatalityPolicy::AtOrBelowZero,
            );
            commit_damage_application(&mut self.entities[index], &application);
            changed.insert(target_position);
            self.wake_entity_after_damage(index, damage.applied, events);
            if !application.fatal {
                self.resolve_monster_fear_aura(index, "hurt", true, events);
            }
            if application.fatal {
                self.resolve_actor_death(
                    index,
                    DomainEvent::ItemActivationSlew {
                        source_kind_id: source_kind_id.clone(),
                        profile_id: profile_id.clone(),
                        target_kind_id,
                        damage,
                        trace: trace.clone(),
                    },
                    events,
                    changed,
                    removed_entities,
                )?;
            } else {
                events.push(DomainEvent::ItemActivationHit {
                    source_kind_id: source_kind_id.clone(),
                    profile_id: profile_id.clone(),
                    target_kind_id,
                    damage,
                    trace: trace.clone(),
                });
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_item_activation_damage(
        &mut self,
        source_kind_id: String,
        profile_id: Option<String>,
        effect: ItemUseEffectDefinition,
        plan: ItemUsePlan,
        device_power_bonus: i32,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let ItemUseEffectDefinition::Damage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
        } = effect
        else {
            unreachable!("item activation damage executor requires a damage effect")
        };
        let ItemUsePlan::Projectile { path } = plan else {
            unreachable!("item activation damage executor requires a projectile plan")
        };
        let profile_id = profile_id.expect("dynamic damage activation must carry a profile ID");
        let (trace, target_index) = self.trace_projectile_path(path);
        self.mark_item_aware(&source_kind_id);
        let Some(target_index) = target_index else {
            events.push(DomainEvent::ItemActivationLanded {
                source_kind_id,
                profile_id,
                trace,
            });
            return Ok(());
        };
        let target_kind_id = self.entities[target_index].kind_id.clone();
        let target_position = self.entities[target_index].position;
        let definition = self
            .content
            .actor(&target_kind_id)
            .expect("activation target definition must remain available")
            .clone();
        let target_stats =
            self.actor_derived_stats(&self.entities[target_index], &definition, false);
        let raw_damage = i32::try_from(device_power_value(
            u64::try_from(
                self.roll_damage(damage_dice, damage_sides)
                    .saturating_add(i32::from(damage_bonus))
                    .max(0),
            )
            .expect("non-negative device damage must fit u64"),
            device_power_bonus,
        ))
        .expect("device-powered damage must fit i32");
        let damage_type = DamageType::from(damage_type);
        if self.try_reflect_player_bolt(
            target_index,
            &source_kind_id,
            raw_damage,
            damage_type,
            false,
            events,
            changed,
            removed_entities,
        )? {
            return Ok(());
        }
        let resistance = self.entities[target_index].resistances.level(damage_type);
        let damage = resolve_armored_damage(
            raw_damage,
            damage_type,
            target_stats.armor_class.value,
            resistance,
        );
        self.entities[target_index].alerted = true;
        let application = plan_damage_application(
            &self.entities[target_index],
            damage,
            FatalityPolicy::AtOrBelowZero,
        );
        commit_damage_application(&mut self.entities[target_index], &application);
        changed.insert(target_position);
        self.wake_entity_after_damage(target_index, damage.applied, events);
        if !application.fatal {
            self.resolve_monster_fear_aura(target_index, "hurt", true, events);
        }
        if application.fatal {
            self.resolve_actor_death(
                target_index,
                DomainEvent::ItemActivationSlew {
                    source_kind_id,
                    profile_id,
                    target_kind_id,
                    damage,
                    trace,
                },
                events,
                changed,
                removed_entities,
            )?;
        } else {
            events.push(DomainEvent::ItemActivationHit {
                source_kind_id,
                profile_id,
                target_kind_id,
                damage,
                trace,
            });
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_item_dispel_category(
        &mut self,
        source_kind_id: &str,
        category: &str,
        amount: u32,
        actor_ids: Vec<String>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let mut affected = false;
        for actor_id in actor_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == actor_id && entity.hp > 0)
            else {
                continue;
            };
            let definition = self
                .content
                .actor(&self.entities[index].kind_id)
                .expect("item dispel target definition must remain available")
                .clone();
            if !actor_matches_category(&definition, category)
                || definition.tags.iter().any(|tag| tag == "resist-all")
            {
                continue;
            }

            affected = true;
            let target_kind_id = definition.id;
            let target_position = self.entities[index].position;
            let damage = resolve_damage(
                DamagePacket::new(
                    i32::try_from(amount).expect("validated item dispel damage must fit i32"),
                    DamageType::HolyFire,
                ),
                ResistanceLevel::Normal,
            );
            self.entities[index].alerted = true;
            let application = plan_damage_application(
                &self.entities[index],
                damage,
                FatalityPolicy::AtOrBelowZero,
            );
            commit_damage_application(&mut self.entities[index], &application);
            changed.insert(target_position);
            self.wake_entity_after_damage(index, damage.applied, events);
            if !application.fatal {
                self.resolve_monster_fear_aura(index, "hurt", true, events);
            }
            if application.fatal {
                self.resolve_actor_death(
                    index,
                    DomainEvent::ItemDispelSlew {
                        source_kind_id: source_kind_id.to_owned(),
                        target_kind_id,
                        damage,
                    },
                    events,
                    changed,
                    removed_entities,
                )?;
            } else {
                events.push(DomainEvent::ItemDispelHit {
                    source_kind_id: source_kind_id.to_owned(),
                    target_kind_id,
                    damage,
                });
            }
        }
        if affected {
            self.mark_item_aware(source_kind_id);
        } else {
            events.push(DomainEvent::ItemDispelNoEffect {
                source_kind_id: source_kind_id.to_owned(),
            });
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_item_elemental_blast(
        &mut self,
        source_kind_id: &str,
        base_damage: u32,
        damage_type: DamageType,
        radius: u8,
        backlash_sides: u16,
        backlash_bonus: u16,
        backlash_damage_type: DamageType,
        backlash_uses_resistance: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        self.mark_item_aware(source_kind_id);
        let (affected_positions, targets) =
            self.area_damage_targets(self.player.position, radius, None);
        changed.extend(affected_positions);
        events.push(DomainEvent::ItemElementalBlast {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            target_count: targets.len(),
        });
        let base_damage =
            i32::try_from(base_damage).expect("validated elemental blast damage must fit i32");
        for (actor_id, distance) in targets {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == actor_id && entity.hp > 0)
            else {
                continue;
            };
            let definition = self
                .content
                .actor(&self.entities[index].kind_id)
                .expect("elemental blast target definition must remain available")
                .clone();
            let target_kind_id = definition.id.clone();
            let target_position = self.entities[index].position;
            let target = self.actor_derived_stats(&self.entities[index], &definition, false);
            let resistance = self.entities[index].resistances.level(damage_type);
            let damage = resolve_armored_damage(
                rfb_area_damage(base_damage, distance),
                damage_type,
                target.armor_class.value,
                resistance,
            );
            self.entities[index].alerted = true;
            let application = plan_damage_application(
                &self.entities[index],
                damage,
                FatalityPolicy::AtOrBelowZero,
            );
            commit_damage_application(&mut self.entities[index], &application);
            changed.insert(target_position);
            self.wake_entity_after_damage(index, damage.applied, events);
            if !application.fatal {
                self.resolve_monster_fear_aura(index, "hurt", true, events);
            }
            if application.fatal {
                self.resolve_actor_death(
                    index,
                    DomainEvent::ItemElementalBlastSlew {
                        source_kind_id: source_kind_id.to_owned(),
                        target_kind_id,
                        damage,
                    },
                    events,
                    changed,
                    removed_entities,
                )?;
            } else {
                events.push(DomainEvent::ItemElementalBlastHit {
                    source_kind_id: source_kind_id.to_owned(),
                    target_kind_id,
                    damage,
                });
            }
        }

        if backlash_sides == 0 && backlash_bonus == 0 {
            return Ok(());
        }
        let backlash_raw = if backlash_sides == 0 {
            0
        } else {
            self.roll_damage(1, backlash_sides)
        }
        .saturating_add(i32::from(backlash_bonus));
        let backlash_resistance = if backlash_uses_resistance {
            self.effective_player_resistances()
                .level(backlash_damage_type)
        } else {
            ResistanceLevel::Normal
        };
        let backlash = self.reduce_player_damage(resolve_damage(
            DamagePacket::new(backlash_raw, backlash_damage_type),
            backlash_resistance,
        ));
        let application = self.apply_final_player_damage(backlash, FatalityPolicy::BelowZero);
        let backlash = application.damage;
        self.damage_player_inventory(
            source_kind_id,
            backlash_damage_type,
            false,
            backlash.applied,
            events,
        );
        events.push(DomainEvent::ItemElementalBlastBacklash {
            source_kind_id: source_kind_id.to_owned(),
            damage: backlash,
            fatal: application.fatal,
        });
        Ok(())
    }

    pub(super) fn resolve_item_detonation(
        &mut self,
        source_kind_id: &str,
        damage_dice: u16,
        damage_sides: u16,
        stun_ticks: u32,
        bleeding_ticks: u32,
        events: &mut Vec<DomainEvent>,
    ) {
        let raw_damage = self.roll_damage(damage_dice, damage_sides);
        let damage = self.reduce_player_damage(resolve_damage(
            DamagePacket::new(raw_damage, DamageType::Physical),
            ResistanceLevel::Normal,
        ));
        let application = self.apply_final_player_damage(damage, FatalityPolicy::BelowZero);
        let damage = application.damage;
        let fatal = application.fatal;
        if !fatal {
            let immunities = self.player_status_immunities();
            if !immunities.contains(STATUS_STUN) {
                apply_status(
                    &mut self.player.statuses,
                    StatusApplication {
                        status: StatusInstance {
                            kind_id: STATUS_STUN.to_owned(),
                            intensity: 1,
                            remaining_ticks: stun_ticks,
                            source_id: Some(source_kind_id.to_owned()),
                            granted_resistances: BTreeMap::new(),
                            granted_brands: BTreeSet::new(),
                            granted_modifiers: StatModifiersDto::default(),
                            granted_equipment_bonuses: EquipmentBonusesDto::default(),
                            granted_status_immunities: BTreeSet::new(),
                            granted_race_id: None,
                            grants_wall_passage: false,
                            incoming_damage_percent: 100,
                        },
                        stacking: StatusStacking::KeepStrongest,
                    },
                );
            }
            if !immunities.contains(STATUS_BLEEDING) {
                apply_status(
                    &mut self.player.statuses,
                    StatusApplication {
                        status: StatusInstance {
                            kind_id: STATUS_BLEEDING.to_owned(),
                            intensity: 1,
                            remaining_ticks: bleeding_ticks,
                            source_id: Some(source_kind_id.to_owned()),
                            granted_resistances: BTreeMap::new(),
                            granted_brands: BTreeSet::new(),
                            granted_modifiers: StatModifiersDto::default(),
                            granted_equipment_bonuses: EquipmentBonusesDto::default(),
                            granted_status_immunities: BTreeSet::new(),
                            granted_race_id: None,
                            grants_wall_passage: false,
                            incoming_damage_percent: 100,
                        },
                        stacking: StatusStacking::Extend,
                    },
                );
            }
        }
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemDetonation {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            damage,
            fatal,
        });
    }

    pub(super) fn resolve_item_life_loss(
        &mut self,
        source_kind_id: &str,
        amount: u32,
        events: &mut Vec<DomainEvent>,
    ) {
        let amount = i32::try_from(amount).expect("validated life loss must fit i32");
        let damage = resolve_damage(
            DamagePacket::new(amount, DamageType::Physical),
            ResistanceLevel::Normal,
        );
        let application = self.apply_final_player_damage(damage, FatalityPolicy::BelowZero);
        let amount = application.damage.applied;
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemLifeLost {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            amount,
            fatal: self.player_is_dead(),
        });
    }
}
