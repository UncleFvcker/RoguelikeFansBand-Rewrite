// SPDX-License-Identifier: MPL-2.0

use super::item_use::ItemUsePlan;
use super::*;

impl Game {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_item_activation_damage(
        &mut self,
        source_kind_id: String,
        profile_id: Option<String>,
        effect: ItemUseEffectDefinition,
        plan: ItemUsePlan,
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
        let raw_damage = self
            .roll_damage(damage_dice, damage_sides)
            .saturating_add(i32::from(damage_bonus))
            .max(0);
        let damage_type = DamageType::from(damage_type);
        if self.try_reflect_player_bolt(
            target_index,
            &source_kind_id,
            raw_damage,
            damage_type,
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

        let backlash_raw = self
            .roll_damage(1, backlash_sides)
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
        let application =
            plan_damage_application(&self.player, backlash, FatalityPolicy::BelowZero);
        commit_damage_application(&mut self.player, &application);
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
        let application = plan_damage_application(&self.player, damage, FatalityPolicy::BelowZero);
        commit_damage_application(&mut self.player, &application);
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
        self.player.hp = self.player.hp.saturating_sub(amount);
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemLifeLost {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            amount,
            fatal: self.player_is_dead(),
        });
    }
}
