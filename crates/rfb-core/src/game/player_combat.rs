// SPDX-License-Identifier: MPL-2.0

use super::*;

fn projectile_raw_damage(
    rolled_ammunition_damage: i32,
    ammunition_to_damage: i32,
    damage_multiplier_percent: u16,
    launcher_to_damage: i32,
) -> i32 {
    rolled_ammunition_damage
        .saturating_add(ammunition_to_damage)
        .saturating_mul(i32::from(damage_multiplier_percent))
        / 100
        + launcher_to_damage
}

impl Game {
    pub(super) fn gain_player_melee_resources(
        &mut self,
        source: ResourceGainSourceDto,
        events: &mut Vec<DomainEvent>,
    ) {
        let resource_ids = self.resources.keys().cloned().collect::<Vec<_>>();
        for resource_id in resource_ids {
            let amount = {
                let definition = self
                    .content
                    .resource(&resource_id)
                    .expect("player resource definition must remain available");
                match source {
                    ResourceGainSourceDto::MeleeHit => definition.melee_hit_gain_amount,
                    ResourceGainSourceDto::MeleeKill => definition.melee_kill_gain_amount,
                }
            };
            if amount == 0 {
                continue;
            }
            // A capped gain still counts as touching the pool so the turn
            // decay never erodes a resource that is being actively fed.
            self.resources_touched.insert(resource_id.clone());
            let pool = self
                .resources
                .get_mut(&resource_id)
                .expect("player resource pool must remain available");
            let before = pool.current;
            pool.current = pool.current.saturating_add(amount).min(pool.maximum);
            if pool.current > before {
                events.push(DomainEvent::ResourceGained {
                    resolution: ResourceGainResolutionDto {
                        resource_id: resource_id.clone(),
                        source,
                        before,
                        after: pool.current,
                        gained: pool.current - before,
                    },
                });
            }
        }
    }

    pub(super) fn resolve_player_projectile(
        &mut self,
        target: TargetSelection,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let Some(profile) = self.player_projectile_profile() else {
            events.push(DomainEvent::ProjectileUnavailable);
            return Ok(());
        };
        let Some(path) = self.projectile_path(&target, profile.range) else {
            events.push(DomainEvent::ProjectileTargetUnavailable);
            return Ok(());
        };
        let Some(ammo_item_id) = &profile.ammo_item_id else {
            events.push(DomainEvent::ProjectileAmmoUnavailable {
                ammo_kind_id: profile.ammo_kind_id,
            });
            return Ok(());
        };
        let Some(ammunition) = self.take_inventory_item(ammo_item_id)? else {
            events.push(DomainEvent::ProjectileAmmoUnavailable {
                ammo_kind_id: profile.ammo_kind_id,
            });
            return Ok(());
        };
        let (trace, target_index) = self.trace_projectile_path(path);
        if let Some(index) = target_index {
            let definition = self
                .content
                .actor(&self.entities[index].kind_id)
                .expect("projectile target definition must remain available")
                .clone();
            let target_kind_id = definition.id.clone();
            self.entities[index].alerted = true;
            let attacker = self.player_derived_stats();
            let ranged_skill = attacker.ranged_skill.with_modifier(
                StatLayer::Equipment,
                profile.ammo_kind_id.clone(),
                profile.ammunition_to_hit,
                StatBounds::NON_NEGATIVE,
            );
            let target = self.actor_derived_stats(&self.entities[index], &definition, false);
            changed.insert(self.entities[index].position);
            if !resolve_check(
                &mut self.rng,
                CheckContext {
                    kind: CheckKind::ProjectileHit,
                    actor_id: self.player.id.clone(),
                    target_id: Some(self.entities[index].id.clone()),
                    ability: ranged_skill,
                    difficulty: target.armor_class.clone(),
                },
            )
            .succeeded()
            {
                events.push(DomainEvent::ProjectileMissed {
                    target_kind_id,
                    trace: trace.clone(),
                });
            } else {
                let raw_damage = projectile_raw_damage(
                    self.roll_damage(profile.damage_dice, profile.damage_sides),
                    profile.ammunition_to_damage,
                    profile.damage_multiplier_percent,
                    profile.launcher_to_damage,
                )
                .max(0);
                let resistance = self.entities[index].resistances.level(profile.damage_type);
                let damage = resolve_armored_damage(
                    raw_damage,
                    profile.damage_type,
                    target.armor_class.value,
                    resistance,
                );
                let application = plan_damage_application(
                    &self.entities[index],
                    damage,
                    FatalityPolicy::AtOrBelowZero,
                );
                commit_damage_application(&mut self.entities[index], &application);
                events.push(DomainEvent::ProjectileHit {
                    target_kind_id: target_kind_id.clone(),
                    damage,
                    trace: trace.clone(),
                });
                self.wake_entity_after_damage(index, damage.applied, events);
                if application.fatal {
                    self.resolve_actor_death(
                        index,
                        DomainEvent::ProjectileSlew {
                            target_kind_id,
                            damage,
                            trace: trace.clone(),
                        },
                        events,
                        changed,
                        removed_entities,
                    )?;
                }
            }
        } else {
            events.push(DomainEvent::ProjectileLanded {
                trace: trace.clone(),
            });
        }
        self.settle_projectile_ammunition(
            ammunition,
            trace.landing,
            target_index.is_some(),
            profile.ammo_break_chance_percent,
            events,
            changed,
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_ability_damage_to_entity(
        &mut self,
        index: usize,
        ability_id: &str,
        damage_type: DamageType,
        raw_damage: i32,
        trace: ProjectileTrace,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<DamageOutcome, CoreError> {
        let definition = self
            .content
            .actor(&self.entities[index].kind_id)
            .expect("ability target definition must remain available")
            .clone();
        let target_kind_id = definition.id.clone();
        self.entities[index].alerted = true;
        changed.insert(self.entities[index].position);
        let target = self.actor_derived_stats(&self.entities[index], &definition, false);
        let resistance = self.entities[index].resistances.level(damage_type);
        let damage = resolve_armored_damage(
            raw_damage,
            damage_type,
            target.armor_class.value,
            resistance,
        );
        let application =
            plan_damage_application(&self.entities[index], damage, FatalityPolicy::AtOrBelowZero);
        commit_damage_application(&mut self.entities[index], &application);
        events.push(DomainEvent::AbilityHit {
            ability_id: ability_id.to_owned(),
            target_kind_id: target_kind_id.clone(),
            damage,
            trace: trace.clone(),
        });
        self.wake_entity_after_damage(index, damage.applied, events);
        if application.fatal {
            self.resolve_actor_death(
                index,
                DomainEvent::AbilitySlew {
                    ability_id: ability_id.to_owned(),
                    target_kind_id,
                    damage,
                    trace,
                },
                events,
                changed,
                removed_entities,
            )?;
        }
        Ok(damage)
    }

    pub(super) fn throw_inventory_item(
        &mut self,
        item_id: &str,
        direction: rfb_protocol::Direction,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let Some(item) = self.items.iter().find(|item| {
            item.id == item_id && item.location == ItemLocation::Inventory && item.quantity > 0
        }) else {
            events.push(DomainEvent::ItemThrowUnavailable);
            return Ok(());
        };
        let definition = self
            .content
            .item(&item.kind_id)
            .expect("throwable item definition must remain available");
        let mighty_throw = self.player_has_mighty_throw();
        let range = throw_range(definition.weight_tenths_pound, mighty_throw);
        let profile = definition
            .throw_profile
            .as_ref()
            .map(|profile| ResolvedThrowProfile {
                to_hit: profile
                    .to_hit
                    .saturating_add(i32::from(item.enchantments.to_hit)),
                to_damage: profile
                    .to_damage
                    .saturating_add(i32::from(item.enchantments.to_damage)),
                damage_dice: profile.damage_dice,
                damage_sides: profile.damage_sides,
                damage_type: DamageType::from(profile.damage_type),
            });
        let Some(mut thrown) = self.take_inventory_item(item_id)? else {
            events.push(DomainEvent::ItemThrowUnavailable);
            return Ok(());
        };
        let source_kind_id = thrown.kind_id.clone();
        self.mark_item_tried(&source_kind_id);
        let path = self
            .projectile_path(&TargetSelection::Direction { direction }, range)
            .expect("direction targeting must always produce a path");
        let (trace, target_index) = self.trace_projectile_path(path);
        let landing = trace.landing;
        if let (Some(profile), Some(index)) = (profile, target_index) {
            let target_definition = self
                .content
                .actor(&self.entities[index].kind_id)
                .expect("throw target definition must remain available")
                .clone();
            let target_kind_id = target_definition.id.clone();
            self.entities[index].alerted = true;
            let attacker = self.player_derived_stats();
            let target = self.actor_derived_stats(&self.entities[index], &target_definition, false);
            let ability = attacker.throwing_skill.with_modifier(
                StatLayer::Equipment,
                &thrown.id,
                profile.to_hit,
                StatBounds::NON_NEGATIVE,
            );
            changed.insert(self.entities[index].position);
            if !resolve_check(
                &mut self.rng,
                CheckContext {
                    kind: CheckKind::ThrowHit,
                    actor_id: self.player.id.clone(),
                    target_id: Some(self.entities[index].id.clone()),
                    ability,
                    difficulty: target.armor_class.clone(),
                },
            )
            .succeeded()
            {
                events.push(DomainEvent::ItemThrowMissed {
                    source_kind_id: source_kind_id.clone(),
                    target_kind_id,
                    trace: trace.clone(),
                });
            } else {
                let raw_damage = self
                    .roll_damage(profile.damage_dice, profile.damage_sides)
                    .saturating_add(profile.to_damage)
                    .saturating_mul(if mighty_throw { 2 } else { 1 })
                    .max(0);
                let resistance = self.entities[index].resistances.level(profile.damage_type);
                let damage = resolve_armored_damage(
                    raw_damage,
                    profile.damage_type,
                    target.armor_class.value,
                    resistance,
                );
                let application = plan_damage_application(
                    &self.entities[index],
                    damage,
                    FatalityPolicy::AtOrBelowZero,
                );
                commit_damage_application(&mut self.entities[index], &application);
                events.push(DomainEvent::ItemThrowHit {
                    source_kind_id: source_kind_id.clone(),
                    target_kind_id: target_kind_id.clone(),
                    damage,
                    trace: trace.clone(),
                });
                self.wake_entity_after_damage(index, damage.applied, events);
                if application.fatal {
                    self.resolve_actor_death(
                        index,
                        DomainEvent::ItemThrowSlew {
                            source_kind_id: source_kind_id.clone(),
                            target_kind_id,
                            damage,
                            trace: trace.clone(),
                        },
                        events,
                        changed,
                        removed_entities,
                    )?;
                }
            }
        } else {
            events.push(DomainEvent::ItemThrown {
                target_kind_id: source_kind_id,
                trace,
            });
        }
        thrown.location = ItemLocation::Ground(landing);
        self.items.push(thrown);
        changed.insert(landing);
        Ok(())
    }

    pub(super) fn resolve_player_melee(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let definition = self
            .content
            .actor(&self.entities[index].kind_id)
            .expect("monster actor definition must remain available")
            .clone();
        let target_kind = self.entities[index].kind_id.clone();
        self.entities[index].alerted = true;
        let attacker = self.player_derived_stats();
        let target = self.actor_derived_stats(&self.entities[index], &definition, false);
        let weapon_profile = self.player_melee_profile(&attacker);
        let equipped_weapon_id = weapon_profile.source_item_id.clone();
        let mut profiles = vec![weapon_profile];
        profiles.extend(
            self.player_mutation_innate_attack_profiles(&attacker, equipped_weapon_id.as_deref()),
        );
        let mut vampiric_drain_remaining = 50_i32;
        'profiles: for profile in profiles {
            let vampiric_weapon = profile.source_item_id.as_ref().is_some_and(|item_id| {
                self.items
                    .iter()
                    .find(|item| &item.id == item_id)
                    .is_some_and(|item| {
                        self.item_passives(item)
                            .contains(&EquipmentPassive::Vampiric)
                    })
            });
            let damage_multiplier =
                self.player_melee_damage_multiplier(&profile, &self.entities[index], &definition);
            for _ in 0..profile.attacks {
                if profile.melee_skill.value <= 0
                    || !resolve_check(
                        &mut self.rng,
                        CheckContext {
                            kind: CheckKind::MeleeHit,
                            actor_id: self.player.id.clone(),
                            target_id: Some(self.entities[index].id.clone()),
                            ability: profile.melee_skill.clone(),
                            difficulty: target.armor_class.clone(),
                        },
                    )
                    .succeeded()
                {
                    events.push(profile.miss_event(&target_kind));
                    continue;
                }

                let weapon_damage = self.roll_damage(profile.damage_dice, profile.damage_sides);
                let mut base_damage = weapon_damage
                    .saturating_mul(damage_multiplier)
                    .saturating_div(10);
                if let Some(weight) = profile.critical_weight_tenths_pound {
                    base_damage = base_damage
                        .saturating_mul(
                            self.roll_innate_critical_multiplier(weight, profile.to_hit),
                        )
                        .saturating_div(100);
                }
                let rolled_damage = base_damage.saturating_add(profile.to_damage).max(0);
                let damage_type = profile.damage_type;
                let resistance = self.entities[index].resistances.level(damage_type);
                let damage =
                    resolve_damage(DamagePacket::new(rolled_damage, damage_type), resistance);
                let application = plan_damage_application(
                    &self.entities[index],
                    damage,
                    FatalityPolicy::AtOrBelowZero,
                );
                commit_damage_application(&mut self.entities[index], &application);
                events.push(profile.hit_event(&target_kind, damage));
                self.wake_entity_after_damage(index, damage.applied, events);
                let contact_aura_fatal = self.resolve_monster_contact_auras(&definition, events);
                if contact_aura_fatal {
                    if application.fatal {
                        self.resolve_actor_death(
                            index,
                            profile.slew_event(&target_kind, damage),
                            events,
                            changed,
                            removed_entities,
                        )?;
                    }
                    break 'profiles;
                }
                if vampiric_weapon
                    && vampiric_drain_remaining > 0
                    && damage.applied > 5
                    && actor_matches_category(&definition, "living")
                {
                    let raw_requested = self
                        .roll_damage(
                            2,
                            u16::try_from(damage.applied / 6)
                                .expect("positive vampiric healing die must fit u16"),
                        )
                        .min(vampiric_drain_remaining);
                    vampiric_drain_remaining =
                        vampiric_drain_remaining.saturating_sub(raw_requested);
                    let requested = raw_requested.saturating_mul(
                        i32::try_from(self.mutation_regeneration_percent())
                            .expect("mutation regeneration percent must fit i32"),
                    ) / 100;
                    let max_hp = self.effective_player_max_hp();
                    let EffectOutcome::Healed { requested, applied } = apply_effect(
                        &mut EffectTarget {
                            hp: &mut self.player.hp,
                            max_hp,
                            resistances: &self.player.resistances,
                            statuses: &mut self.player.statuses,
                        },
                        EffectSpec::Heal { amount: requested },
                    ) else {
                        unreachable!("vampiric melee healing must produce a healing outcome");
                    };
                    events.push(DomainEvent::PlayerVampiricHealed {
                        resolution: HealingResolutionDto { requested, applied },
                    });
                }
                self.gain_player_melee_resources(ResourceGainSourceDto::MeleeHit, events);
                if application.fatal {
                    self.resolve_actor_death(
                        index,
                        profile.slew_event(&target_kind, damage),
                        events,
                        changed,
                        removed_entities,
                    )?;
                    self.gain_player_melee_resources(ResourceGainSourceDto::MeleeKill, events);
                    break 'profiles;
                }
                self.resolve_confusing_strike(index, &definition, events);
            }
        }
        Ok(())
    }

    pub(super) fn roll_innate_critical_multiplier(
        &mut self,
        weight_tenths_pound: u16,
        to_hit: i32,
    ) -> i32 {
        let chance = i64::from(weight_tenths_pound)
            .saturating_add(i64::from(to_hit).saturating_mul(5))
            .saturating_add(i64::from(self.progress.level).saturating_mul(3));
        let roll =
            i64::try_from(self.rng.bounded(5_000) + 1).expect("critical-hit roll must fit i64");
        if roll > chance {
            return 100;
        }
        let quality = u64::from(weight_tenths_pound) + self.rng.bounded(650) + 1;
        match quality {
            0..=399 => 200,
            400..=699 => 250,
            700..=899 => 300,
            900..=1_299 => 350,
            _ => 400,
        }
    }

    pub(super) fn resolve_monster_contact_auras(
        &mut self,
        definition: &rfb_content::ActorDefinition,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        for aura in &definition.contact_auras {
            if aura
                .chance_percent
                .is_some_and(|chance| self.rng.bounded(100) >= u64::from(chance))
            {
                continue;
            }
            let raw = self.roll_damage(aura.damage_dice, aura.damage_sides);
            let damage_type = DamageType::from(aura.damage_type);
            let damage = resolve_damage(
                DamagePacket::new(raw, damage_type),
                self.effective_player_resistances().level(damage_type),
            );
            if aura.damage_type != rfb_content::ActorDamageType::Poison {
                if damage.applied <= 0 {
                    continue;
                }
                let application =
                    plan_damage_application(&self.player, damage, FatalityPolicy::BelowZero);
                commit_damage_application(&mut self.player, &application);
                events.push(DomainEvent::MonsterMeleeHit {
                    source_kind_id: definition.id.clone(),
                    method_id: None,
                    damage,
                });
                if application.fatal {
                    events.push(DomainEvent::PlayerDied {
                        source_kind_id: definition.id.clone(),
                        method_id: None,
                        damage,
                    });
                    return true;
                }
                continue;
            }
            let duration = damage.applied.saturating_mul(7) / 4;
            if duration <= 0 || self.player_status_immunities().contains(STATUS_POISON) {
                continue;
            }
            let duration = u32::try_from(duration).unwrap_or(u32::MAX);
            apply_status(
                &mut self.player.statuses,
                monster_combat::melee_status(STATUS_POISON, duration, &definition.id),
            );
            events.push(DomainEvent::MonsterContactAuraApplied {
                source_kind_id: definition.id.clone(),
                status_kind_id: STATUS_POISON.to_owned(),
                duration,
            });
        }
        false
    }

    pub(super) fn resolve_confusing_strike(
        &mut self,
        index: usize,
        definition: &rfb_content::ActorDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        if !self.confusing_strike_ready {
            return;
        }
        self.confusing_strike_ready = false;
        let target_kind_id = self.entities[index].kind_id.clone();
        if definition
            .status_immunities
            .iter()
            .any(|status| status == STATUS_CONFUSION)
        {
            events.push(DomainEvent::ConfusingStrikeImmune { target_kind_id });
            return;
        }
        if self.rng.bounded(100) < u64::from(definition.level) {
            events.push(DomainEvent::ConfusingStrikeResisted { target_kind_id });
            return;
        }
        let duration = 10_u32.saturating_add(
            u32::try_from(self.rng.bounded(u64::from(self.progress.level.max(1))))
                .expect("confusing strike duration roll must fit u32")
                / 5,
        );
        apply_status(
            &mut self.entities[index].statuses,
            StatusApplication {
                status: StatusInstance {
                    kind_id: STATUS_CONFUSION.to_owned(),
                    intensity: 1,
                    remaining_ticks: duration,
                    source_id: None,
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
        events.push(DomainEvent::ConfusingStrikeApplied {
            target_kind_id,
            duration,
        });
    }

    pub(super) fn resolve_player_summon_melee(
        &mut self,
        source_index: usize,
        target_entity_id: &str,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let source_entity_id = self.entities[source_index].id.clone();
        let source_kind_id = self.entities[source_index].kind_id.clone();
        let definition = self
            .content
            .actor(&source_kind_id)
            .expect("summon actor definition must remain available")
            .clone();
        let attacker = self.actor_derived_stats(&self.entities[source_index], &definition, false);
        for blow in resolved_melee_blows(&definition) {
            let Some(target_index) = self
                .entities
                .iter()
                .position(|entity| entity.id == target_entity_id && entity.hp > 0)
            else {
                break;
            };
            let target_kind_id = self.entities[target_index].kind_id.clone();
            let target_position = self.entities[target_index].position;
            let target_definition = self
                .content
                .actor(&target_kind_id)
                .expect("summon melee target definition must remain available");
            let target_stats =
                self.actor_derived_stats(&self.entities[target_index], target_definition, false);
            let ability = attacker.melee_skill.with_modifier(
                StatLayer::Base,
                blow.method_id.as_deref().unwrap_or(definition.id.as_str()),
                blow.to_hit,
                StatBounds::NON_NEGATIVE,
            );
            if !resolve_check(
                &mut self.rng,
                CheckContext {
                    kind: CheckKind::MeleeHit,
                    actor_id: source_entity_id.clone(),
                    target_id: Some(target_entity_id.to_owned()),
                    ability,
                    difficulty: target_stats.armor_class.clone(),
                },
            )
            .succeeded()
            {
                events.push(DomainEvent::SummonMeleeMissed {
                    source_kind_id: source_kind_id.clone(),
                    target_kind_id,
                    method_id: blow.method_id,
                });
                continue;
            }

            if blow.self_destructs {
                if let Some(source_index) = self
                    .entities
                    .iter()
                    .position(|entity| entity.id == source_entity_id)
                {
                    self.resolve_actor_death_without_rewards(
                        source_index,
                        Some(DomainEvent::MonsterSelfDestructed {
                            source_kind_id: source_kind_id.clone(),
                        }),
                        events,
                        changed,
                        removed_entities,
                    )?;
                }
                break;
            }

            self.entities[target_index].alerted = true;
            for effect in &blow.effects {
                if monster_combat::melee_effect_chance(effect)
                    .is_some_and(|chance| self.rng.bounded(100) >= u64::from(chance))
                {
                    continue;
                }
                let Some(target_index) = self
                    .entities
                    .iter()
                    .position(|entity| entity.id == target_entity_id && entity.hp > 0)
                else {
                    break;
                };
                let damage = match effect {
                    MeleeBlowEffectDefinition::Damage {
                        damage_dice,
                        damage_sides,
                        damage_type,
                        armor_mitigated,
                        ..
                    } => {
                        let raw = self.roll_damage(*damage_dice, *damage_sides);
                        let damage_type = DamageType::from(*damage_type);
                        let resistance = self.entities[target_index].resistances.level(damage_type);
                        Some(if *armor_mitigated {
                            resolve_armored_damage(
                                raw,
                                damage_type,
                                target_stats.armor_class.value,
                                resistance,
                            )
                        } else {
                            resolve_damage(DamagePacket::new(raw, damage_type), resistance)
                        })
                    }
                    MeleeBlowEffectDefinition::Poison {
                        damage_dice,
                        damage_sides,
                        ..
                    } => {
                        let raw = self.roll_damage(*damage_dice, *damage_sides);
                        let duration = resolve_damage(
                            DamagePacket::new(raw, DamageType::Poison),
                            self.entities[target_index]
                                .resistances
                                .level(DamageType::Poison),
                        )
                        .applied
                        .saturating_mul(7)
                            / 4;
                        self.apply_actor_melee_status(
                            target_index,
                            STATUS_POISON,
                            duration,
                            &source_kind_id,
                        );
                        None
                    }
                    MeleeBlowEffectDefinition::Disease {
                        damage_dice,
                        damage_sides,
                        ..
                    } => {
                        let raw = self.roll_damage(*damage_dice, *damage_sides);
                        Some(resolve_damage(
                            DamagePacket::new(raw, DamageType::Poison),
                            self.entities[target_index]
                                .resistances
                                .level(DamageType::Poison),
                        ))
                    }
                    MeleeBlowEffectDefinition::DrainAttributes { .. }
                    | MeleeBlowEffectDefinition::DrainResource { .. }
                    | MeleeBlowEffectDefinition::DrainExperience { .. }
                    | MeleeBlowEffectDefinition::Disenchant { .. } => None,
                    MeleeBlowEffectDefinition::Bleeding {
                        duration_dice,
                        duration_sides,
                        ..
                    } => {
                        let duration = self.roll_damage(*duration_dice, *duration_sides);
                        self.apply_actor_melee_status(
                            target_index,
                            STATUS_BLEEDING,
                            duration,
                            &source_kind_id,
                        );
                        None
                    }
                    MeleeBlowEffectDefinition::Blind { .. } => {
                        let duration = 12 + i32::try_from(self.rng.bounded(4)).unwrap_or(0);
                        self.apply_actor_melee_status(
                            target_index,
                            STATUS_BLINDNESS,
                            duration,
                            &source_kind_id,
                        );
                        None
                    }
                    MeleeBlowEffectDefinition::Confusion {
                        damage_dice,
                        damage_sides,
                        ..
                    } => {
                        let resistance = self.entities[target_index]
                            .resistances
                            .level(DamageType::Confusion);
                        let raw = (*damage_dice > 0)
                            .then(|| self.roll_damage(*damage_dice, *damage_sides));
                        let duration = resisted_status_duration(
                            u32::try_from(10 + self.roll_damage(1, 20)).unwrap_or(u32::MAX),
                            resistance,
                        );
                        self.apply_actor_melee_status(
                            target_index,
                            STATUS_CONFUSION,
                            i32::try_from(duration).unwrap_or(i32::MAX),
                            &source_kind_id,
                        );
                        raw.map(|raw| {
                            resolve_damage(
                                DamagePacket::new(raw, DamageType::Confusion),
                                resistance,
                            )
                        })
                    }
                    MeleeBlowEffectDefinition::Paralysis { .. } => {
                        let duration = self.roll_damage(1, 3);
                        self.apply_actor_melee_status(
                            target_index,
                            STATUS_PARALYSIS,
                            duration,
                            &source_kind_id,
                        );
                        None
                    }
                    MeleeBlowEffectDefinition::Slow { .. } => {
                        self.apply_actor_melee_status(
                            target_index,
                            STATUS_SLOW,
                            25,
                            &source_kind_id,
                        );
                        None
                    }
                    MeleeBlowEffectDefinition::Stun {
                        duration_dice,
                        duration_sides,
                        ..
                    } => {
                        let duration = self.roll_damage(*duration_dice, *duration_sides);
                        self.apply_actor_melee_status(
                            target_index,
                            STATUS_STUN,
                            duration,
                            &source_kind_id,
                        );
                        None
                    }
                    MeleeBlowEffectDefinition::Terrify { .. } => {
                        self.apply_actor_melee_status(
                            target_index,
                            STATUS_FEAR,
                            monster_combat::melee_terrify_duration(&definition),
                            &source_kind_id,
                        );
                        None
                    }
                    MeleeBlowEffectDefinition::EatGold { .. }
                    | MeleeBlowEffectDefinition::EatItem { .. }
                    | MeleeBlowEffectDefinition::EatFood { .. }
                    | MeleeBlowEffectDefinition::EatLight { .. } => None,
                };
                let Some(damage) = damage else {
                    continue;
                };
                let application = plan_damage_application(
                    &self.entities[target_index],
                    damage,
                    FatalityPolicy::AtOrBelowZero,
                );
                commit_damage_application(&mut self.entities[target_index], &application);
                changed.insert(target_position);
                self.wake_entity_after_damage(target_index, damage.applied, events);
                if application.fatal {
                    self.resolve_actor_death(
                        target_index,
                        DomainEvent::SummonSlew {
                            source_kind_id: source_kind_id.clone(),
                            target_kind_id: target_kind_id.clone(),
                            method_id: blow.method_id.clone(),
                            damage,
                        },
                        events,
                        changed,
                        removed_entities,
                    )?;
                    break;
                }
                if matches!(effect, MeleeBlowEffectDefinition::Disease { .. }) {
                    let duration = resolve_damage(
                        DamagePacket::new(damage.applied, DamageType::Poison),
                        self.entities[target_index]
                            .resistances
                            .level(DamageType::Poison),
                    )
                    .applied;
                    self.apply_actor_melee_status(
                        target_index,
                        STATUS_POISON,
                        duration,
                        &source_kind_id,
                    );
                }
                events.push(DomainEvent::SummonMeleeHit {
                    source_kind_id: source_kind_id.clone(),
                    target_kind_id: target_kind_id.clone(),
                    method_id: blow.method_id.clone(),
                    damage,
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::projectile_raw_damage;

    #[test]
    fn ammunition_damage_and_bonus_are_scaled_before_launcher_bonus() {
        assert_eq!(projectile_raw_damage(7, 2, 250, 3), 25);
        assert_eq!(projectile_raw_damage(7, 2, 350, 3), 34);
    }
}
