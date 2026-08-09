// SPDX-License-Identifier: MPL-2.0

use super::*;

pub(super) fn melee_effect_chance(effect: &MeleeBlowEffectDefinition) -> Option<u8> {
    match effect {
        MeleeBlowEffectDefinition::Damage { chance_percent, .. }
        | MeleeBlowEffectDefinition::Poison { chance_percent, .. }
        | MeleeBlowEffectDefinition::Disease { chance_percent, .. }
        | MeleeBlowEffectDefinition::DrainAttributes { chance_percent, .. }
        | MeleeBlowEffectDefinition::DrainResource { chance_percent, .. }
        | MeleeBlowEffectDefinition::DrainExperience { chance_percent, .. }
        | MeleeBlowEffectDefinition::Bleeding { chance_percent, .. }
        | MeleeBlowEffectDefinition::Blind { chance_percent }
        | MeleeBlowEffectDefinition::Confusion { chance_percent, .. }
        | MeleeBlowEffectDefinition::Paralysis { chance_percent }
        | MeleeBlowEffectDefinition::Slow { chance_percent }
        | MeleeBlowEffectDefinition::Stun { chance_percent, .. }
        | MeleeBlowEffectDefinition::Terrify { chance_percent }
        | MeleeBlowEffectDefinition::Disenchant { chance_percent }
        | MeleeBlowEffectDefinition::EatGold { chance_percent }
        | MeleeBlowEffectDefinition::EatItem { chance_percent }
        | MeleeBlowEffectDefinition::EatFood { chance_percent }
        | MeleeBlowEffectDefinition::EatLight { chance_percent } => *chance_percent,
    }
}

pub(super) fn melee_status(
    kind_id: &str,
    duration: u32,
    source_kind_id: &str,
) -> StatusApplication {
    StatusApplication {
        status: StatusInstance {
            kind_id: kind_id.to_owned(),
            intensity: 1,
            remaining_ticks: duration,
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
    }
}

const fn nice_melee_roll(roll: i32, nice: bool) -> i32 {
    if nice && roll > 50 {
        25 + roll / 2
    } else {
        roll
    }
}

pub(super) fn melee_terrify_duration(definition: &rfb_content::ActorDefinition) -> i32 {
    i32::try_from(definition.level)
        .unwrap_or(i32::MAX)
        .saturating_add(i32::from(definition.tags.iter().any(|tag| tag == "unique")) * 3)
}

fn disenchantable_player_status(kind_id: &str) -> bool {
    matches!(
        kind_id,
        STATUS_HASTE
            | STATUS_VENGEANCE
            | STATUS_PROTECTION_FROM_EVIL
            | STATUS_THERMAL_RESISTANCE
            | STATUS_BASIC_RESISTANCE
            | "rfb.status.berserk"
            | "rfb.status.blessed"
            | "rfb.status.hero"
            | "rfb.status.necromantic-resistance"
            | "rfb.status.poetic-inspiration"
            | "rfb.status.poison-branding"
            | "rfb.status.stone-skin"
            | "rfb.status.vampiric-transformation"
            | "rfb.status.wraithform"
    )
}

fn reduce_disenchanted_component(rng: &mut RfbRng, value: u16) -> u16 {
    let mut value = value.saturating_sub(1);
    if value > 5 && rng.bounded(100) < 20 {
        value -= 1;
    }
    value
}

impl Game {
    pub(super) fn actor_has_status_immunity(&self, index: usize, status_kind_id: &str) -> bool {
        self.content
            .actor(&self.entities[index].kind_id)
            .is_some_and(|definition| {
                definition
                    .status_immunities
                    .iter()
                    .any(|immunity| immunity == status_kind_id)
            })
            || self.entities[index]
                .statuses
                .iter()
                .any(|status| status.granted_status_immunities.contains(status_kind_id))
    }

    pub(super) fn apply_actor_melee_status(
        &mut self,
        index: usize,
        status_kind_id: &str,
        duration: i32,
        source_kind_id: &str,
    ) {
        if duration <= 0 || self.actor_has_status_immunity(index, status_kind_id) {
            return;
        }
        apply_status(
            &mut self.entities[index].statuses,
            melee_status(
                status_kind_id,
                u32::try_from(duration).unwrap_or(u32::MAX),
                source_kind_id,
            ),
        );
    }

    pub(super) fn apply_player_melee_status(
        &mut self,
        status_kind_id: &str,
        duration: i32,
        source_kind_id: &str,
    ) {
        if duration <= 0 || self.player_status_immunities().contains(status_kind_id) {
            return;
        }
        apply_status(
            &mut self.player.statuses,
            melee_status(
                status_kind_id,
                u32::try_from(duration).unwrap_or(u32::MAX),
                source_kind_id,
            ),
        );
    }

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
            let self_destructs = self.resolve_monster_melee(source_index, events, changed)?;
            if self_destructs {
                if let Some(index) = self
                    .entities
                    .iter()
                    .position(|entity| entity.id == source_entity_id)
                {
                    self.resolve_actor_death(
                        index,
                        DomainEvent::MonsterSelfDestructed {
                            source_kind_id: self.entities[index].kind_id.clone(),
                        },
                        events,
                        changed,
                        removed_entities,
                    )?;
                }
            } else {
                self.resolve_vengeance_retaliation(
                    &source_entity_id,
                    player_hp_before.saturating_sub(self.player.hp),
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            return Ok(());
        }
        let source_entity_id = self.entities[source_index].id.clone();
        let source_kind_id = self.entities[source_index].kind_id.clone();
        let definition = self
            .content
            .actor(&source_kind_id)
            .expect("monster actor definition must remain available")
            .clone();
        let attacker = self.actor_derived_stats(&self.entities[source_index], &definition, false);
        let mut blink_after_melee = false;
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

            if blow.self_destructs {
                if let Some(source_index) = self
                    .entities
                    .iter()
                    .position(|entity| entity.id == source_entity_id)
                {
                    self.resolve_actor_death(
                        source_index,
                        DomainEvent::MonsterSelfDestructed {
                            source_kind_id: source_kind_id.clone(),
                        },
                        events,
                        changed,
                        removed_entities,
                    )?;
                }
                break;
            }

            for effect in &blow.effects {
                if melee_effect_chance(effect)
                    .is_some_and(|chance| self.rng.bounded(100) >= u64::from(chance))
                {
                    continue;
                }
                let Some(target_index) = self
                    .entities
                    .iter()
                    .position(|entity| entity.id == target.entity_id() && entity.hp > 0)
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
                            melee_terrify_duration(&definition),
                            &source_kind_id,
                        );
                        None
                    }
                    MeleeBlowEffectDefinition::EatGold { .. }
                    | MeleeBlowEffectDefinition::EatItem { .. } => {
                        blink_after_melee |= self.rng.bounded(2) == 0;
                        None
                    }
                    MeleeBlowEffectDefinition::EatFood { .. }
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
                self.wake_entity_after_damage(target_index, damage.applied, events);
                if application.fatal {
                    self.resolve_actor_death_without_rewards(
                        target_index,
                        Some(DomainEvent::MonsterMeleeEntitySlew {
                            source_kind_id: source_kind_id.clone(),
                            target_kind_id: target.kind_id().to_owned(),
                            method_id: blow.method_id.clone(),
                            damage,
                        }),
                        events,
                        changed,
                        removed_entities,
                    )?;
                    break;
                }
                events.push(DomainEvent::MonsterMeleeEntityHit {
                    source_kind_id: source_kind_id.clone(),
                    target_kind_id: target.kind_id().to_owned(),
                    method_id: blow.method_id.clone(),
                    damage,
                });
            }
        }
        if blink_after_melee
            && let Some(source_index) = self
                .entities
                .iter()
                .position(|entity| entity.id == source_entity_id)
        {
            self.blink_monster_after_theft(source_index, events, changed);
        }
        Ok(())
    }

    fn player_prevents_monster_theft(&mut self) -> bool {
        const DEXTERITY_SAFETY: [u8; 38] = [
            0, 1, 2, 3, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 15, 15, 20, 25, 30, 35, 40, 45,
            50, 60, 70, 80, 90, 100, 100, 100, 100, 100, 100, 100, 100,
        ];
        if self.player_has_status_kind(STATUS_PARALYSIS) {
            return false;
        }
        let dexterity_index = usize::from(
            self.effective_player_attributes()
                .index(AttributeKind::Dexterity),
        )
        .min(DEXTERITY_SAFETY.len() - 1);
        self.rng.bounded(100)
            < u64::from(DEXTERITY_SAFETY[dexterity_index]) + u64::from(self.progress.level)
    }

    fn monster_is_confused(&self, index: usize) -> bool {
        self.entities[index]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_CONFUSION)
    }

    fn blink_monster_after_theft(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let actor_id = self.entities[index].id.clone();
        let source_kind_id = self.entities[index].kind_id.clone();
        let from = self.entities[index].position;
        let destinations = self.open_positions_around_for_actor_kind(from, 45, &source_kind_id);
        if destinations.is_empty() {
            return;
        }
        let choice = usize::try_from(
            self.rng
                .bounded(u64::try_from(destinations.len()).expect("candidate count fits")),
        )
        .expect("bounded draw fits usize");
        let to = destinations[choice];
        self.entities[index].position = to;
        changed.insert(from);
        changed.insert(to);
        events.push(DomainEvent::MonsterBlinked {
            source_kind_id,
            resolution: MonsterDisplacementResolutionDto { actor_id, from, to },
        });
    }

    fn monster_steal_gold(&mut self, index: usize, events: &mut Vec<DomainEvent>) -> bool {
        if self.monster_is_confused(index) {
            return false;
        }
        let source_kind_id = self.entities[index].kind_id.clone();
        if self.player_prevents_monster_theft() {
            events.push(DomainEvent::MonsterGoldTheftPrevented { source_kind_id });
            return self.rng.bounded(3) != 0;
        }
        let mut amount = self
            .gold
            .saturating_div(10)
            .saturating_add(u32::try_from(self.rng.bounded(25) + 1).unwrap_or(25));
        amount = amount.max(2);
        if amount > 5_000 {
            amount = self
                .gold
                .saturating_div(20)
                .saturating_add(u32::try_from(self.rng.bounded(3_000) + 1).unwrap_or(3_000));
        }
        amount = amount.min(self.gold);
        self.gold -= amount;
        events.push(DomainEvent::MonsterGoldStolen {
            source_kind_id,
            amount,
        });
        true
    }

    fn monster_steal_item(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
    ) -> Result<bool, CoreError> {
        if self.monster_is_confused(index) {
            return Ok(false);
        }
        let actor_id = self.entities[index].id.clone();
        let source_kind_id = self.entities[index].kind_id.clone();
        if self.player_prevents_monster_theft() {
            events.push(DomainEvent::MonsterItemTheftPrevented { source_kind_id });
            return Ok(true);
        }
        let mut candidates = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.location == ItemLocation::Inventory)
            .filter(|(_, item)| {
                self.content
                    .item(&item.kind_id)
                    .is_some_and(|definition| !definition.tags.iter().any(|tag| tag == "artifact"))
            })
            .map(|(item_index, item)| (item.id.clone(), item_index))
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| left.0.cmp(&right.0));
        if candidates.is_empty() {
            return Ok(false);
        }
        let choice = usize::try_from(
            self.rng
                .bounded(u64::try_from(candidates.len()).expect("candidate count fits")),
        )
        .expect("bounded draw fits usize");
        let item_index = candidates[choice].1;
        let target_kind_id = self.items[item_index].kind_id.clone();
        let item_id = if self.items[item_index].quantity == 1 {
            self.items[item_index].location = ItemLocation::CarriedBy {
                actor_id: actor_id.clone(),
            };
            self.items[item_index].id.clone()
        } else {
            let item_id = self.allocate_item_instance_id()?;
            let mut stolen = self.items[item_index].clone();
            self.items[item_index].quantity -= 1;
            stolen.id.clone_from(&item_id);
            stolen.quantity = 1;
            stolen.location = ItemLocation::CarriedBy {
                actor_id: actor_id.clone(),
            };
            if let Some(knowledge) = self
                .item_property_knowledge
                .get(&self.items[item_index].id)
                .cloned()
            {
                self.item_property_knowledge
                    .insert(item_id.clone(), knowledge);
            }
            self.items.push(stolen);
            item_id
        };
        events.push(DomainEvent::MonsterItemStolen {
            source_kind_id,
            target_kind_id,
            item_id,
        });
        Ok(true)
    }

    fn monster_eat_food(&mut self, index: usize, events: &mut Vec<DomainEvent>) {
        let source_kind_id = self.entities[index].kind_id.clone();
        let mut candidates = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.location == ItemLocation::Inventory)
            .filter(|(_, item)| {
                self.content.item(&item.kind_id).is_some_and(|definition| {
                    definition.tags.iter().any(|tag| tag == "food")
                        && !definition.tags.iter().any(|tag| tag == "artifact")
                })
            })
            .map(|(item_index, item)| (item.id.clone(), item_index))
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| left.0.cmp(&right.0));
        if candidates.is_empty() {
            return;
        }
        let choice = usize::try_from(
            self.rng
                .bounded(u64::try_from(candidates.len()).expect("candidate count fits")),
        )
        .expect("bounded draw fits usize");
        let item_index = candidates[choice].1;
        let target_kind_id = self.items[item_index].kind_id.clone();
        if self.items[item_index].quantity == 1 {
            let removed = self.items.remove(item_index);
            self.item_property_knowledge.remove(&removed.id);
        } else {
            self.items[item_index].quantity -= 1;
        }
        events.push(DomainEvent::MonsterFoodEaten {
            source_kind_id,
            target_kind_id,
        });
    }

    fn monster_eat_light(&mut self, index: usize, events: &mut Vec<DomainEvent>) {
        let Some(item_index) = self.items.iter().position(|item| {
            matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == "light")
                && item.fuel.is_some_and(|fuel| fuel.current > 0)
                && self
                    .content
                    .item(&item.kind_id)
                    .is_some_and(|definition| !definition.tags.iter().any(|tag| tag == "artifact"))
        }) else {
            return;
        };
        let requested = u16::try_from(self.rng.bounded(250) + 251).unwrap_or(500);
        let target_kind_id = self.items[item_index].kind_id.clone();
        let fuel = self.items[item_index]
            .fuel
            .as_mut()
            .expect("selected light must retain fuel");
        let before = fuel.current;
        fuel.current = fuel.current.saturating_sub(requested).max(1);
        events.push(DomainEvent::MonsterLightEaten {
            source_kind_id: self.entities[index].kind_id.clone(),
            target_kind_id,
            amount: before - fuel.current,
        });
    }

    fn resolve_player_disenchantment(&mut self) {
        let remove_status = self.rng.bounded(5) != 0;
        let resistance = self
            .effective_player_resistances()
            .level(DamageType::Disenchant)
            .reduction_percent()
            .clamp(0, 100);
        if resistance == 100
            || (resistance > 0
                && self.rng.bounded(100)
                    < u64::try_from(resistance).expect("clamped resistance fits"))
        {
            return;
        }

        if remove_status {
            let mut candidates = self
                .player
                .statuses
                .iter()
                .map(|status| status.kind_id.clone())
                .filter(|kind_id| disenchantable_player_status(kind_id))
                .collect::<Vec<_>>();
            candidates.sort();
            candidates.dedup();
            if candidates.is_empty() {
                return;
            }
            let choice = usize::try_from(
                self.rng
                    .bounded(u64::try_from(candidates.len()).expect("candidate count fits")),
            )
            .expect("bounded draw fits usize");
            apply_status_removal(
                &mut self.player.statuses,
                StatusRemovalRequest::new(&candidates[choice]),
            );
            return;
        }

        let mut candidates = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(item_index, item)| {
                let ItemLocation::Equipped { slot_id } = &item.location else {
                    return None;
                };
                self.content.item(&item.kind_id).and_then(|definition| {
                    definition
                        .tags
                        .iter()
                        .any(|tag| matches!(tag.as_str(), "weapon" | "armor" | "ammunition"))
                        .then(|| (slot_id.clone(), item.id.clone(), item_index))
                })
            })
            .collect::<Vec<_>>();
        candidates.sort();
        if candidates.is_empty() {
            return;
        }
        let choice = usize::try_from(
            self.rng
                .bounded(u64::try_from(candidates.len()).expect("candidate count fits")),
        )
        .expect("bounded draw fits usize");
        let item_index = candidates[choice].2;
        let enchantments = self.items[item_index].enchantments;
        if enchantments.is_empty() {
            return;
        }
        let is_artifact = self
            .content
            .item(&self.items[item_index].kind_id)
            .is_some_and(|definition| definition.tags.iter().any(|tag| tag == "artifact"));
        if is_artifact && self.rng.bounded(100) < 71 {
            return;
        }
        self.items[item_index].enchantments = ItemEnchantmentsDto {
            to_hit: reduce_disenchanted_component(&mut self.rng, enchantments.to_hit),
            to_damage: reduce_disenchanted_component(&mut self.rng, enchantments.to_damage),
            to_armor: reduce_disenchanted_component(&mut self.rng, enchantments.to_armor),
        };
    }

    pub(super) fn resolve_monster_melee(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Result<bool, CoreError> {
        let kind_id = self.entities[index].kind_id.clone();
        let nice = self.entities[index].nice;
        let definition = self
            .content
            .actor(&kind_id)
            .expect("monster actor definition must remain available")
            .clone();
        let attacker = self.actor_derived_stats(&self.entities[index], &definition, false);
        let target = self.player_derived_stats();
        let armor_class = target.armor_class.value;
        let mut blink_after_melee = false;
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

            if blow.self_destructs {
                return Ok(true);
            }

            for effect in &blow.effects {
                if melee_effect_chance(effect)
                    .is_some_and(|chance| self.rng.bounded(100) >= u64::from(chance))
                {
                    continue;
                }
                let damage = match effect {
                    MeleeBlowEffectDefinition::Damage {
                        damage_dice,
                        damage_sides,
                        damage_type,
                        armor_mitigated,
                        ..
                    } => {
                        let raw =
                            nice_melee_roll(self.roll_damage(*damage_dice, *damage_sides), nice);
                        let damage_type = DamageType::from(*damage_type);
                        let resistance = self.effective_player_resistances().level(damage_type);
                        Some(self.reduce_player_damage(if *armor_mitigated {
                            resolve_armored_damage(raw, damage_type, armor_class, resistance)
                        } else {
                            resolve_damage(DamagePacket::new(raw, damage_type), resistance)
                        }))
                    }
                    MeleeBlowEffectDefinition::Poison {
                        damage_dice,
                        damage_sides,
                        ..
                    } => {
                        let raw =
                            nice_melee_roll(self.roll_damage(*damage_dice, *damage_sides), nice);
                        let duration = resolve_damage(
                            DamagePacket::new(raw, DamageType::Poison),
                            self.effective_player_resistances()
                                .level(DamageType::Poison),
                        )
                        .applied
                        .saturating_mul(7)
                            / 4;
                        if duration > 0 && !self.player_status_immunities().contains(STATUS_POISON)
                        {
                            apply_status(
                                &mut self.player.statuses,
                                melee_status(
                                    STATUS_POISON,
                                    u32::try_from(duration).unwrap_or(u32::MAX),
                                    &kind_id,
                                ),
                            );
                        }
                        None
                    }
                    MeleeBlowEffectDefinition::Disease {
                        damage_dice,
                        damage_sides,
                        ..
                    } => {
                        let raw =
                            nice_melee_roll(self.roll_damage(*damage_dice, *damage_sides), nice);
                        Some(
                            self.reduce_player_damage(resolve_damage(
                                DamagePacket::new(raw, DamageType::Physical),
                                self.effective_player_resistances()
                                    .level(DamageType::Physical),
                            )),
                        )
                    }
                    MeleeBlowEffectDefinition::DrainAttributes { attributes, .. } => {
                        for attribute in attributes {
                            self.resolve_monster_attribute_drain(Game::item_attribute_kind(
                                attribute,
                            ));
                        }
                        None
                    }
                    MeleeBlowEffectDefinition::DrainResource {
                        amount_dice,
                        amount_sides,
                        ..
                    } => {
                        let requested =
                            u32::try_from(self.roll_damage(*amount_dice, *amount_sides).max(0))
                                .unwrap_or(u32::MAX);
                        let pool_id = self
                            .casting_profile()
                            .map(|profile| profile.resource_id.clone())
                            .filter(|id| self.resources.contains_key(id))
                            .or_else(|| {
                                self.resources
                                    .iter()
                                    .find(|(_, pool)| pool.current > 0)
                                    .map(|(id, _)| id.clone())
                            });
                        let drained = pool_id.map_or(0, |id| {
                            let pool = self
                                .resources
                                .get_mut(&id)
                                .expect("selected melee drain pool must remain available");
                            let drained = pool.current.min(requested);
                            pool.current -= drained;
                            drained
                        });
                        if drained > 0 {
                            let caster = &mut self.entities[index];
                            let missing = caster.max_hp.saturating_sub(caster.hp).max(0);
                            let healing = drained.saturating_mul(6);
                            caster.hp = caster.hp.saturating_add(
                                i32::try_from(healing).unwrap_or(i32::MAX).min(missing),
                            );
                            changed.insert(caster.position);
                        }
                        None
                    }
                    MeleeBlowEffectDefinition::DrainExperience {
                        amount_dice,
                        amount_sides,
                        ..
                    } => {
                        let rolled =
                            nice_melee_roll(self.roll_damage(*amount_dice, *amount_sides), nice);
                        let requested = u64::try_from(rolled.max(0))
                            .unwrap_or(u64::MAX)
                            .saturating_add(self.progress.experience.saturating_mul(2) / 100)
                            .min(25_000);
                        self.apply_player_experience_drain(requested, &kind_id, events);
                        None
                    }
                    MeleeBlowEffectDefinition::Bleeding {
                        duration_dice,
                        duration_sides,
                        ..
                    } => {
                        let duration = nice_melee_roll(
                            self.roll_damage(*duration_dice, *duration_sides),
                            nice,
                        );
                        if duration > 0
                            && !self.player_status_immunities().contains(STATUS_BLEEDING)
                        {
                            apply_status(
                                &mut self.player.statuses,
                                melee_status(
                                    STATUS_BLEEDING,
                                    u32::try_from(duration).unwrap_or(u32::MAX),
                                    &kind_id,
                                ),
                            );
                        }
                        None
                    }
                    MeleeBlowEffectDefinition::Blind { .. } => {
                        let duration = 12 + i32::try_from(self.rng.bounded(4)).unwrap_or(0);
                        self.apply_player_melee_status(STATUS_BLINDNESS, duration, &kind_id);
                        None
                    }
                    MeleeBlowEffectDefinition::Confusion {
                        damage_dice,
                        damage_sides,
                        ..
                    } => {
                        let resistance = self
                            .effective_player_resistances()
                            .level(DamageType::Confusion);
                        let raw = (*damage_dice > 0).then(|| {
                            nice_melee_roll(self.roll_damage(*damage_dice, *damage_sides), nice)
                        });
                        let duration = resisted_status_duration(
                            u32::try_from(10 + self.roll_damage(1, 20)).unwrap_or(u32::MAX),
                            resistance,
                        );
                        self.apply_player_melee_status(
                            STATUS_CONFUSION,
                            i32::try_from(duration).unwrap_or(i32::MAX),
                            &kind_id,
                        );
                        raw.map(|raw| {
                            self.reduce_player_damage(resolve_damage(
                                DamagePacket::new(raw, DamageType::Confusion),
                                resistance,
                            ))
                        })
                    }
                    MeleeBlowEffectDefinition::Paralysis { .. } => {
                        let duration = self.roll_damage(1, 3);
                        self.apply_player_melee_status(STATUS_PARALYSIS, duration, &kind_id);
                        None
                    }
                    MeleeBlowEffectDefinition::Slow { .. } => {
                        self.apply_player_melee_status(STATUS_SLOW, 25, &kind_id);
                        None
                    }
                    MeleeBlowEffectDefinition::Stun {
                        duration_dice,
                        duration_sides,
                        ..
                    } => {
                        let duration = nice_melee_roll(
                            self.roll_damage(*duration_dice, *duration_sides),
                            nice,
                        );
                        self.apply_player_melee_status(STATUS_STUN, duration, &kind_id);
                        None
                    }
                    MeleeBlowEffectDefinition::Terrify { .. } => {
                        self.apply_player_melee_status(
                            STATUS_FEAR,
                            melee_terrify_duration(&definition),
                            &kind_id,
                        );
                        None
                    }
                    MeleeBlowEffectDefinition::Disenchant { .. } => {
                        self.resolve_player_disenchantment();
                        None
                    }
                    MeleeBlowEffectDefinition::EatGold { .. } => {
                        blink_after_melee |= self.monster_steal_gold(index, events);
                        None
                    }
                    MeleeBlowEffectDefinition::EatItem { .. } => {
                        blink_after_melee |= self.monster_steal_item(index, events)?;
                        None
                    }
                    MeleeBlowEffectDefinition::EatFood { .. } => {
                        self.monster_eat_food(index, events);
                        None
                    }
                    MeleeBlowEffectDefinition::EatLight { .. } => {
                        self.monster_eat_light(index, events);
                        None
                    }
                };
                let Some(damage) = damage else {
                    continue;
                };
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
                        method_id: blow.method_id.clone(),
                        damage,
                    });
                    return Ok(false);
                }
                if matches!(effect, MeleeBlowEffectDefinition::Disease { .. }) {
                    let duration = resolve_damage(
                        DamagePacket::new(damage.applied, DamageType::Poison),
                        self.effective_player_resistances()
                            .level(DamageType::Poison),
                    )
                    .applied;
                    if duration > 0 && !self.player_status_immunities().contains(STATUS_POISON) {
                        apply_status(
                            &mut self.player.statuses,
                            melee_status(
                                STATUS_POISON,
                                u32::try_from(duration).unwrap_or(u32::MAX),
                                &kind_id,
                            ),
                        );
                    }
                    if self.rng.bounded(100) < 10 {
                        self.resolve_monster_attribute_drain(AttributeKind::Constitution);
                    }
                }
            }
        }
        if blink_after_melee && self.player.hp > 0 {
            self.blink_monster_after_theft(index, events, changed);
        }
        Ok(false)
    }

    fn resolve_monster_attribute_drain(&mut self, attribute: AttributeKind) {
        if self
            .player_equipment_passives()
            .contains(&attribute_sustain_passive(attribute))
        {
            return;
        }
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let outcome = apply_attribute_drain(&mut self.progress, attribute, &mut self.rng);
        if outcome.changed {
            self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
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

#[cfg(test)]
mod nice_melee_tests {
    use super::nice_melee_roll;

    #[test]
    fn spawn_grace_limits_only_rolls_above_fifty() {
        assert_eq!(nice_melee_roll(50, true), 50);
        assert_eq!(nice_melee_roll(51, true), 50);
        assert_eq!(nice_melee_roll(100, true), 75);
        assert_eq!(nice_melee_roll(100, false), 100);
    }
}
