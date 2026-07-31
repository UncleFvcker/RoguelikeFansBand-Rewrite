// SPDX-License-Identifier: MPL-2.0

use super::*;

impl Game {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_monster_damage_to_hostile(
        &mut self,
        source_entity_id: &str,
        source_kind_id: &str,
        ability_id: &str,
        effect_index: u8,
        raw_damage: i32,
        prepared_damage: i32,
        damage_type: DamageType,
        target: &MonsterHostileTarget,
        events: &mut Vec<DomainEvent>,
    ) -> AbilityEffectResolutionDto {
        if target.is_player() {
            return self.resolve_monster_damage_to_player(
                source_entity_id,
                source_kind_id,
                ability_id,
                effect_index,
                raw_damage,
                prepared_damage,
                damage_type,
                events,
            );
        }
        let Some(target_index) = self
            .entities
            .iter()
            .position(|entity| entity.id == target.entity_id() && entity.hp > 0)
        else {
            return AbilityEffectResolutionDto::Skipped {
                effect_index,
                reason: AbilityEffectSkipReasonDto::TargetDead,
            };
        };
        let resistance = self.entities[target_index].resistances.level(damage_type);
        let damage = resolve_damage(
            DamagePacket::after_armor(raw_damage, prepared_damage, damage_type),
            resistance,
        );
        let application = plan_damage_application(
            &self.entities[target_index],
            damage,
            FatalityPolicy::AtOrBelowZero,
        );
        commit_damage_application(&mut self.entities[target_index], &application);
        self.wake_entity_after_damage(target_index, damage.applied, events);
        AbilityEffectResolutionDto::Damage {
            effect_index,
            resolution: damage.into(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_monster_damage_to_player(
        &mut self,
        source_entity_id: &str,
        source_kind_id: &str,
        ability_id: &str,
        effect_index: u8,
        raw_damage: i32,
        prepared_damage: i32,
        damage_type: DamageType,
        events: &mut Vec<DomainEvent>,
    ) -> AbilityEffectResolutionDto {
        let resistance = self.effective_player_resistances().level(damage_type);
        self.record_monster_player_resistance(source_entity_id, damage_type, resistance);
        let damage = self.reduce_player_damage(resolve_damage(
            DamagePacket::after_armor(raw_damage, prepared_damage, damage_type),
            resistance,
        ));
        let application = plan_damage_application(&self.player, damage, FatalityPolicy::BelowZero);
        commit_damage_application(&mut self.player, &application);
        if application.fatal {
            events.push(DomainEvent::PlayerDied {
                source_kind_id: source_kind_id.to_owned(),
                method_id: Some(ability_id.to_owned()),
                damage,
            });
        }
        AbilityEffectResolutionDto::Damage {
            effect_index,
            resolution: damage.into(),
        }
    }

    pub(super) fn record_monster_player_resistance(
        &mut self,
        source_entity_id: &str,
        damage_type: DamageType,
        resistance: ResistanceLevel,
    ) {
        let Some(source_index) = self
            .entities
            .iter()
            .position(|entity| entity.id == source_entity_id)
        else {
            return;
        };
        let smart = self
            .content
            .actor(&self.entities[source_index].kind_id)
            .and_then(|actor| actor.monster_casting.as_ref())
            .is_some_and(|casting| casting.smart);
        if smart {
            self.entities[source_index]
                .observed_player_resistances
                .insert(damage_type, resistance);
        }
    }

    pub(super) fn resolve_monster_melee_target(
        &mut self,
        source_index: usize,
        target: &MonsterHostileTarget,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        if target.is_player() {
            let source_entity_id = self.entities[source_index].id.clone();
            let player_hp_before = self.player.hp;
            self.resolve_monster_melee(source_index, events);
            self.resolve_vengeance_retaliation(
                &source_entity_id,
                player_hp_before.saturating_sub(self.player.hp),
                events,
                changed,
                removed_entities,
            )?;
            return Ok(());
        }
        let source_kind_id = self.entities[source_index].kind_id.clone();
        let definition = self
            .content
            .actor(&source_kind_id)
            .expect("monster actor definition must remain available")
            .clone();
        let attacker = self.actor_derived_stats(&self.entities[source_index], &definition, false);
        for blow in resolved_melee_blows(&definition) {
            let Some(target_index) = self
                .entities
                .iter()
                .position(|entity| entity.id == target.entity_id() && entity.hp > 0)
            else {
                break;
            };
            let target_definition = self
                .content
                .actor(&self.entities[target_index].kind_id)
                .expect("monster melee target definition must remain available");
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
                    actor_id: self.entities[source_index].id.clone(),
                    target_id: Some(target.entity_id().to_owned()),
                    ability,
                    difficulty: target_stats.armor_class.clone(),
                },
            )
            .succeeded()
            {
                events.push(DomainEvent::MonsterMeleeEntityMissed {
                    source_kind_id: source_kind_id.clone(),
                    target_kind_id: target.kind_id().to_owned(),
                    method_id: blow.method_id,
                });
                continue;
            }

            let raw_damage = self.roll_damage(blow.damage_dice, blow.damage_sides);
            let resistance = self.entities[target_index]
                .resistances
                .level(blow.damage_type);
            let damage = resolve_armored_damage(
                raw_damage,
                blow.damage_type,
                target_stats.armor_class.value,
                resistance,
            );
            let application = plan_damage_application(
                &self.entities[target_index],
                damage,
                FatalityPolicy::AtOrBelowZero,
            );
            commit_damage_application(&mut self.entities[target_index], &application);
            self.wake_entity_after_damage(target_index, damage.applied, events);
            if application.fatal {
                events.push(DomainEvent::MonsterMeleeEntitySlew {
                    source_kind_id: source_kind_id.clone(),
                    target_kind_id: target.kind_id().to_owned(),
                    method_id: blow.method_id,
                    damage,
                });
                let removed = self.entities.remove(target_index);
                changed.insert(removed.position);
                removed_entities.push(removed.id);
                break;
            }
            events.push(DomainEvent::MonsterMeleeEntityHit {
                source_kind_id: source_kind_id.clone(),
                target_kind_id: target.kind_id().to_owned(),
                method_id: blow.method_id,
                damage,
            });
        }
        Ok(())
    }

    pub(super) fn resolve_monster_melee(&mut self, index: usize, events: &mut Vec<DomainEvent>) {
        let kind_id = self.entities[index].kind_id.clone();
        let definition = self
            .content
            .actor(&kind_id)
            .expect("monster actor definition must remain available")
            .clone();
        let attacker = self.actor_derived_stats(&self.entities[index], &definition, false);
        let target = self.player_derived_stats();
        let armor_class = target.armor_class.value;
        for blow in resolved_melee_blows(&definition) {
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
                    actor_id: self.entities[index].id.clone(),
                    target_id: Some(self.player.id.clone()),
                    ability,
                    difficulty: target.armor_class.clone(),
                },
            )
            .succeeded()
            {
                events.push(DomainEvent::MonsterMeleeMissed {
                    source_kind_id: kind_id.clone(),
                    method_id: blow.method_id,
                });
                continue;
            }

            if self.protection_from_evil_repels(&definition) {
                events.push(DomainEvent::MonsterMeleeRepelled {
                    source_kind_id: kind_id.clone(),
                    method_id: blow.method_id,
                });
                continue;
            }

            let raw_damage = self.roll_damage(blow.damage_dice, blow.damage_sides);
            let resistance = self.effective_player_resistances().level(blow.damage_type);
            let damage = self.reduce_player_damage(resolve_armored_damage(
                raw_damage,
                blow.damage_type,
                armor_class,
                resistance,
            ));
            let application =
                plan_damage_application(&self.player, damage, FatalityPolicy::BelowZero);
            commit_damage_application(&mut self.player, &application);
            events.push(DomainEvent::MonsterMeleeHit {
                source_kind_id: kind_id.clone(),
                method_id: blow.method_id.clone(),
                damage,
            });
            if application.fatal {
                events.push(DomainEvent::PlayerDied {
                    source_kind_id: kind_id.clone(),
                    method_id: blow.method_id,
                    damage,
                });
                break;
            }
        }
    }

    pub(super) fn protection_from_evil_repels(
        &mut self,
        definition: &rfb_content::ActorDefinition,
    ) -> bool {
        if !self.player_has_status_kind(STATUS_PROTECTION_FROM_EVIL)
            || !definition.tags.iter().any(|tag| tag == "evil")
        {
            return false;
        }

        const ORIGINAL_SAVE_ADJUSTMENT: [i32; 38] = [
            -25, -15, -10, -7, -6, -5, -4, -3, -2, -2, -1, -1, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8,
            9, 10, 12, 14, 16, 18, 20, 23, 26, 29, 33, 37, 42, 50,
        ];
        let wisdom_index = usize::from(
            self.effective_player_attributes()
                .index(AttributeKind::Wisdom),
        )
        .min(ORIGINAL_SAVE_ADJUSTMENT.len() - 1);
        let player_power = i64::from(self.progress.level)
            .saturating_add(i64::from(ORIGINAL_SAVE_ADJUSTMENT[wisdom_index]))
            .max(1) as u64;
        let monster_power = u64::from(if definition.tags.iter().any(|tag| tag == "unique") {
            definition.level.saturating_add(definition.level / 5)
        } else {
            definition.level
        })
        .max(1);
        let player_roll = self.rng.bounded(player_power).saturating_add(1);
        let monster_roll = self.rng.bounded(monster_power).saturating_add(1);
        if player_roll <= monster_roll {
            return false;
        }
        self.rng.bounded(3) != 0
    }

    pub(super) fn resolve_vengeance_retaliation(
        &mut self,
        source_entity_id: &str,
        applied_damage: i32,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        if applied_damage <= 0
            || self.player_is_dead()
            || !self.player_has_status_kind(STATUS_VENGEANCE)
        {
            return Ok(());
        }
        let source_index = self
            .entities
            .iter()
            .position(|entity| entity.id == source_entity_id && entity.hp > 0)
            .ok_or_else(|| {
                CoreError::Invariant(format!(
                    "vengeance source actor {source_entity_id} is missing"
                ))
            })?;
        let target_kind_id = self.entities[source_index].kind_id.clone();
        let target_position = self.entities[source_index].position;
        let damage = resolve_damage(
            DamagePacket::new(applied_damage, DamageType::Physical),
            ResistanceLevel::Normal,
        );
        let application = plan_damage_application(
            &self.entities[source_index],
            damage,
            FatalityPolicy::AtOrBelowZero,
        );
        commit_damage_application(&mut self.entities[source_index], &application);
        changed.insert(target_position);
        if application.fatal {
            self.resolve_actor_death(
                source_index,
                DomainEvent::VengeanceSlew {
                    target_kind_id,
                    damage,
                },
                events,
                changed,
                removed_entities,
            )?;
        } else {
            events.push(DomainEvent::VengeanceHit {
                target_kind_id,
                damage,
            });
        }

        let status_index = self
            .player
            .statuses
            .iter()
            .position(|status| status.kind_id == STATUS_VENGEANCE)
            .ok_or_else(|| {
                CoreError::Invariant(
                    "active vengeance status disappeared during retaliation".into(),
                )
            })?;
        self.player.statuses[status_index].remaining_ticks = self.player.statuses[status_index]
            .remaining_ticks
            .saturating_sub(5);
        if self.player.statuses[status_index].remaining_ticks == 0 {
            self.player.statuses.remove(status_index);
            events.push(DomainEvent::PlayerStatusExpired {
                status_kind_id: STATUS_VENGEANCE.to_owned(),
            });
        }
        Ok(())
    }
}
