// SPDX-License-Identifier: MPL-2.0

use super::*;

const RFB_RACE_SNOTLING: u16 = 6;
const RFB_RACE_YEEK: u16 = 15;
const RFB_MIMIC_SMALL_KOBOLD: u16 = 1_007;
const RFB_MIMIC_MANGY_LEPER: u16 = 1_008;
const RFB_MAX_RACES: u16 = 75;
const POLYMORPH_CANDIDATE_TAG: &str = "polymorph-candidate";
const POLYMORPH_IMMUNE_TAG: &str = "polymorph-immune";

pub(super) fn melee_effect_chance(effect: &MeleeBlowEffectDefinition) -> Option<u8> {
    match effect {
        MeleeBlowEffectDefinition::Damage { chance_percent, .. }
        | MeleeBlowEffectDefinition::Shatter { chance_percent, .. }
        | MeleeBlowEffectDefinition::Bomb { chance_percent, .. }
        | MeleeBlowEffectDefinition::Poison { chance_percent, .. }
        | MeleeBlowEffectDefinition::Disease { chance_percent, .. }
        | MeleeBlowEffectDefinition::DrainAttributes { chance_percent, .. }
        | MeleeBlowEffectDefinition::DrainResource { chance_percent, .. }
        | MeleeBlowEffectDefinition::DrainCharges { chance_percent }
        | MeleeBlowEffectDefinition::DrainExperience { chance_percent, .. }
        | MeleeBlowEffectDefinition::Unlife { chance_percent, .. }
        | MeleeBlowEffectDefinition::Bleeding { chance_percent, .. }
        | MeleeBlowEffectDefinition::Blind { chance_percent }
        | MeleeBlowEffectDefinition::Confusion { chance_percent, .. }
        | MeleeBlowEffectDefinition::Paralysis { chance_percent }
        | MeleeBlowEffectDefinition::Amnesia { chance_percent }
        | MeleeBlowEffectDefinition::Time { chance_percent }
        | MeleeBlowEffectDefinition::Slow { chance_percent }
        | MeleeBlowEffectDefinition::Inertia { chance_percent }
        | MeleeBlowEffectDefinition::PolymorphPlayer { chance_percent }
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

fn reduce_disenchanted_component(rng: &mut RfbRng, value: i16) -> i16 {
    if value <= 0 {
        return value;
    }
    let mut value = value - 1;
    if value > 5 && rng.bounded(100) < 20 {
        value -= 1;
    }
    value
}

impl Game {
    pub(super) fn scale_monster_damage(&self, source_entity_id: &str, damage: i32) -> i32 {
        let power_per_mille = self
            .entities
            .iter()
            .find(|entity| entity.id == source_entity_id)
            .map_or(BASE_ACTOR_POWER_PER_MILLE, |entity| entity.power_per_mille);
        scale_actor_power(damage, power_per_mille)
    }

    pub(super) fn player_is_nonliving(&self) -> bool {
        self.character_definitions()
            .is_some_and(|(_, race, _, _)| race.tags.iter().any(|tag| tag == "nonliving"))
    }

    pub(super) fn heal_vampiric_melee_source(
        &mut self,
        source_entity_id: &str,
        damage: i32,
        changed: &mut BTreeSet<Position>,
    ) {
        let Some(source) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == source_entity_id)
        else {
            return;
        };
        let hp_before = source.hp;
        source.hp = source.hp.saturating_add(damage.max(0)).min(source.max_hp);
        if source.hp != hp_before {
            changed.insert(source.position);
        }
    }

    pub(super) fn roll_monster_melee_effect(
        &mut self,
        source_index: usize,
        dice: u16,
        sides: u16,
        nice: bool,
    ) -> i32 {
        let rolled = self.roll_damage(dice, sides);
        nice_melee_roll(
            scale_actor_power(rolled, self.entities[source_index].power_per_mille),
            nice,
        )
    }

    pub(super) fn resolve_monster_unlife_against_player(
        &mut self,
        source_index: usize,
        amount: u16,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        if amount == 0 || self.player_is_nonliving() || self.player_saves_unlife(source_index) {
            return;
        }
        let life_force = self.drain_player_life_force(amount);
        let source = &mut self.entities[source_index];
        let power_before = source.power_per_mille;
        source.power_per_mille = source.power_per_mille.saturating_add(amount);
        changed.insert(source.position);
        events.push(DomainEvent::MonsterUnlifeDrained {
            source_kind_id: source.kind_id.clone(),
            amount,
            life_force_before: life_force.before,
            life_force_after: life_force.after,
            power_before,
            power_after: source.power_per_mille,
        });
    }

    fn player_saves_unlife(&mut self, source_index: usize) -> bool {
        let sources = self.player_hold_life_sources();
        if sources == 0 {
            return false;
        }
        const CHARISMA_SAVE_ADJUSTMENT: [i32; 38] = [
            -25, -15, -10, -7, -6, -5, -4, -3, -2, -2, -1, -1, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8,
            9, 10, 12, 14, 16, 18, 20, 23, 26, 29, 33, 37, 42, 50,
        ];
        let mut player_level = i32::from(self.progress.level);
        player_level = if player_level <= 40 {
            player_level.saturating_add(5)
        } else {
            45_i32.saturating_add(player_level.saturating_sub(40).saturating_mul(2))
        };
        let charisma_index = usize::from(
            self.effective_player_attributes()
                .index(AttributeKind::Charisma),
        )
        .min(CHARISMA_SAVE_ADJUSTMENT.len() - 1);
        let player_power = u64::try_from(
            player_level
                .saturating_add(CHARISMA_SAVE_ADJUSTMENT[charisma_index])
                .max(1),
        )
        .unwrap_or(1);
        let source = &self.entities[source_index];
        let monster_level = self
            .actor_runtime_definition(source)
            .map_or(1, |definition| definition.level.max(4));
        let monster_power = u64::try_from(
            scale_actor_power(
                i32::try_from(monster_level).unwrap_or(i32::MAX),
                source.power_per_mille,
            )
            .max(1),
        )
        .unwrap_or(1);
        (0..sources).any(|_| self.rng.bounded(monster_power) <= self.rng.bounded(player_power))
    }

    pub(super) fn resolve_monster_unlife_against_actor(
        &mut self,
        source_kind_id: &str,
        target_index: usize,
        amount: u16,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let target_is_living = self
            .actor_runtime_definition(&self.entities[target_index])
            .is_some_and(|definition| !definition.tags.iter().any(|tag| tag == "nonliving"));
        if amount == 0 || !target_is_living {
            return;
        }
        let target = &mut self.entities[target_index];
        let power_before = target.power_per_mille;
        target.power_per_mille = target.power_per_mille.saturating_sub(amount).max(100);
        if target.power_per_mille == power_before {
            return;
        }
        changed.insert(target.position);
        events.push(DomainEvent::MonsterUnlifeWeakened {
            source_kind_id: source_kind_id.to_owned(),
            target_kind_id: target.kind_id.clone(),
            amount,
            power_before,
            power_after: target.power_per_mille,
        });
    }

    pub(super) fn actor_has_status_immunity(&self, index: usize, status_kind_id: &str) -> bool {
        self.actor_runtime_definition(&self.entities[index])
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

    fn player_is_polymorph_immune(&self) -> bool {
        self.character_definitions()
            .is_some_and(|(_, race, _, _)| race.tags.iter().any(|tag| tag == POLYMORPH_IMMUNE_TAG))
    }

    fn permanent_player_race_legacy_index(&self) -> Option<u16> {
        self.build
            .as_ref()
            .and_then(|identity| self.content.race(&identity.race_id))
            .and_then(|race| race.legacy_index)
    }

    fn polymorph_race_id(&mut self) -> Option<String> {
        let base_index = self.permanent_player_race_legacy_index();
        let branch = self.rng.bounded(5);
        let fixed_index = if branch == 0 && base_index != Some(RFB_RACE_SNOTLING) {
            Some(RFB_RACE_SNOTLING)
        } else if branch <= 1 && base_index != Some(RFB_RACE_YEEK) {
            Some(RFB_RACE_YEEK)
        } else if branch <= 2 {
            Some(RFB_MIMIC_SMALL_KOBOLD)
        } else if branch <= 3 {
            Some(RFB_MIMIC_MANGY_LEPER)
        } else {
            None
        };
        if let Some(index) = fixed_index {
            return self
                .content
                .race_by_legacy_index(index)
                .map(|race| race.id.clone());
        }
        loop {
            let index = u16::try_from(self.rng.bounded(u64::from(RFB_MAX_RACES)))
                .expect("RFB race roll must fit u16");
            if base_index == Some(index) {
                continue;
            }
            let Some(race) = self.content.race_by_legacy_index(index) else {
                continue;
            };
            if race.tags.iter().any(|tag| tag == POLYMORPH_CANDIDATE_TAG) {
                return Some(race.id.clone());
            }
        }
    }

    pub(super) fn reconcile_player_body_slots(&mut self, next_slots: Vec<BodySlot>) {
        let old_slot_types = self
            .body_slots
            .iter()
            .map(|slot| (slot.id.as_str(), slot.slot_type.as_str()))
            .collect::<BTreeMap<_, _>>();
        let mut equipped_indices = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| matches!(item.location, ItemLocation::Equipped { .. }))
            .map(|(index, item)| (item.id.clone(), index))
            .collect::<Vec<_>>();
        equipped_indices.sort_by(|left, right| left.0.cmp(&right.0));
        let mut occupied = BTreeSet::new();
        let mut plan = Vec::with_capacity(equipped_indices.len());
        for (_, item_index) in equipped_indices {
            let ItemLocation::Equipped { slot_id } = &self.items[item_index].location else {
                unreachable!("equipped item plan must retain its location")
            };
            let old_slot_type = old_slot_types.get(slot_id.as_str()).copied();
            let declared_slot_type = self
                .content
                .item(&self.items[item_index].kind_id)
                .and_then(|definition| definition.equipment_slot.as_deref());
            let compatible = |slot: &BodySlot| {
                declared_slot_type
                    .is_some_and(|declared| item_can_occupy_slot_type(declared, &slot.slot_type))
                    && !occupied.contains(&slot.id)
            };
            let next_slot = next_slots
                .iter()
                .find(|slot| slot.id == *slot_id && compatible(slot))
                .or_else(|| {
                    next_slots.iter().find(|slot| {
                        old_slot_type == Some(slot.slot_type.as_str()) && compatible(slot)
                    })
                })
                .or_else(|| next_slots.iter().find(|slot| compatible(slot)))
                .map(|slot| slot.id.clone());
            if let Some(slot_id) = &next_slot {
                occupied.insert(slot_id.clone());
            }
            plan.push((item_index, next_slot));
        }
        let mut unequipped = Vec::new();
        for (item_index, slot_id) in plan {
            self.items[item_index].location = slot_id.map_or(ItemLocation::Inventory, |slot_id| {
                ItemLocation::Equipped { slot_id }
            });
            if matches!(self.items[item_index].location, ItemLocation::Inventory) {
                unequipped.push(item_index);
            }
        }
        self.body_slots = next_slots;
        unequipped.sort_by(|left, right| self.items[*left].id.cmp(&self.items[*right].id));
        while self.inventory_used_slots() > self.inventory_slot_capacity() {
            let item_index = unequipped.pop().or_else(|| {
                self.items
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| item.location == ItemLocation::Inventory)
                    .max_by(|(_, left), (_, right)| left.id.cmp(&right.id))
                    .map(|(index, _)| index)
            });
            let Some(item_index) = item_index else {
                break;
            };
            self.items[item_index].location = ItemLocation::Ground(self.player.position);
        }
    }

    pub(super) fn resolve_player_polymorph(
        &mut self,
        source_kind_id: &str,
        source_level: u32,
        events: &mut Vec<DomainEvent>,
    ) {
        if self.player_is_polymorph_immune()
            || self.monster_saving_throw(source_kind_id, source_level, events)
        {
            return;
        }
        let Some(race_id) = self.polymorph_race_id() else {
            return;
        };
        let duration = 51_u32.saturating_add(
            u32::try_from(self.rng.bounded(50)).expect("polymorph duration roll must fit u32"),
        );
        let race = self
            .content
            .race(&race_id)
            .expect("selected polymorph race must remain available");
        let next_slots = body_slots_for_race(race);
        let status = StatusInstance {
            kind_id: STATUS_PLAYER_POLYMORPH.to_owned(),
            intensity: 1,
            remaining_ticks: duration,
            source_id: Some(source_kind_id.to_owned()),
            granted_resistances: BTreeMap::new(),
            granted_brands: BTreeSet::new(),
            granted_modifiers: StatModifiersDto::default(),
            granted_equipment_bonuses: EquipmentBonusesDto::default(),
            granted_status_immunities: BTreeSet::new(),
            granted_race_id: Some(race_id),
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        };
        apply_status(
            &mut self.player.statuses,
            StatusApplication {
                status,
                stacking: StatusStacking::Replace,
            },
        );
        self.reconcile_player_body_slots(next_slots);
        self.refresh_player_resource_maxima();
        self.clamp_player_hp_to_effective_max();
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
        let raw_damage = self.scale_monster_damage(source_entity_id, raw_damage);
        let prepared_damage = self.scale_monster_damage(source_entity_id, prepared_damage);
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
        let raw_damage = self.scale_monster_damage(source_entity_id, raw_damage);
        let prepared_damage = self.scale_monster_damage(source_entity_id, prepared_damage);
        let resistance = self.effective_player_resistances().level(damage_type);
        self.record_monster_player_resistance(source_entity_id, damage_type, resistance);
        let damage = self.reduce_player_damage(resolve_damage(
            DamagePacket::after_armor(raw_damage, prepared_damage, damage_type),
            resistance,
        ));
        let damage = self.apply_evasion_to_monster_ability_damage(ability_id, damage);
        let application = self.apply_final_player_damage(damage, FatalityPolicy::BelowZero);
        let damage = application.damage;
        self.damage_player_inventory(source_kind_id, damage_type, false, damage.applied, events);
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

    pub(super) fn apply_evasion_to_monster_ability_damage(
        &mut self,
        ability_id: &str,
        damage: DamageOutcome,
    ) -> DamageOutcome {
        if !self.player_evades_innate_monster_attacks()
            || !self.content.ability(ability_id).is_some_and(|ability| {
                ability
                    .effect
                    .ordered_effects()
                    .iter()
                    .any(|effect| match effect {
                        AbilityEffectDefinition::BreathDamage { .. } => true,
                        AbilityEffectDefinition::AreaDamage {
                            damage_dice,
                            damage_sides,
                            damage_type: ActorDamageType::Shards,
                            ..
                        }
                        | AbilityEffectDefinition::Damage {
                            damage_dice,
                            damage_sides,
                            damage_type: ActorDamageType::Physical,
                            ..
                        } => *damage_dice == 1 && *damage_sides == 1,
                        _ => false,
                    })
            })
        {
            return damage;
        }
        let reduction = 11_u8
            .saturating_add(u8::try_from(self.rng.bounded(10)).expect("evasion roll must fit u8"));
        scale_damage_outcome(damage, 100_u8.saturating_sub(reduction))
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
            .actor_runtime_definition(&self.entities[source_index])
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
            if adjacent(self.entities[source_index].position, target.position())
                && let Some(broken) = self.try_monster_break_warding_glyph(
                    source_index,
                    target.position(),
                    events,
                    changed,
                )
                && !broken
            {
                return Ok(());
            }
            let source_entity_id = self.entities[source_index].id.clone();
            let player_hp_before = self.player.hp;
            let self_destructs =
                self.resolve_monster_melee(source_index, events, changed, removed_entities)?;
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
            } else if self
                .entities
                .iter()
                .any(|entity| entity.id == source_entity_id)
            {
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
            .actor_runtime_definition(&self.entities[source_index])
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
                .actor_runtime_definition(&self.entities[target_index])
                .expect("monster melee target definition must remain available");
            let target_is_nonliving = target_definition.tags.iter().any(|tag| tag == "nonliving");
            let target_stats =
                self.actor_derived_stats(&self.entities[target_index], target_definition, false);
            let ability = attacker.melee_skill.with_modifier(
                StatLayer::Base,
                blow.method_id.as_deref().unwrap_or(definition.id.as_str()),
                blow.to_hit,
                StatBounds::NON_NEGATIVE,
            );
            if !blow.effects.is_empty()
                && !resolve_check(
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

            if blow.effects.is_empty() {
                events.push(DomainEvent::MonsterBeggedEntity {
                    source_kind_id: source_kind_id.clone(),
                    target_kind_id: target.kind_id().to_owned(),
                });
                continue;
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
                let vampiric = matches!(
                    effect,
                    MeleeBlowEffectDefinition::Damage { vampiric: true, .. }
                ) && !target_is_nonliving;
                let damage = match effect {
                    MeleeBlowEffectDefinition::Damage {
                        damage_dice,
                        damage_sides,
                        damage_type,
                        armor_mitigated,
                        ..
                    } => {
                        let raw = self.roll_monster_melee_effect(
                            source_index,
                            *damage_dice,
                            *damage_sides,
                            false,
                        );
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
                    MeleeBlowEffectDefinition::Shatter {
                        damage_dice,
                        damage_sides,
                        ..
                    } => {
                        let raw = self.roll_monster_melee_effect(
                            source_index,
                            *damage_dice,
                            *damage_sides,
                            false,
                        );
                        Some(resolve_armored_damage(
                            raw,
                            DamageType::Physical,
                            target_stats.armor_class.value,
                            self.entities[target_index]
                                .resistances
                                .level(DamageType::Physical),
                        ))
                    }
                    MeleeBlowEffectDefinition::Poison {
                        damage_dice,
                        damage_sides,
                        ..
                    } => {
                        let raw = self.roll_monster_melee_effect(
                            source_index,
                            *damage_dice,
                            *damage_sides,
                            false,
                        );
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
                        let raw = self.roll_monster_melee_effect(
                            source_index,
                            *damage_dice,
                            *damage_sides,
                            false,
                        );
                        Some(resolve_damage(
                            DamagePacket::new(raw, DamageType::Poison),
                            self.entities[target_index]
                                .resistances
                                .level(DamageType::Poison),
                        ))
                    }
                    MeleeBlowEffectDefinition::Bomb { .. }
                    | MeleeBlowEffectDefinition::DrainAttributes { .. }
                    | MeleeBlowEffectDefinition::DrainResource { .. }
                    | MeleeBlowEffectDefinition::DrainCharges { .. }
                    | MeleeBlowEffectDefinition::DrainExperience { .. }
                    | MeleeBlowEffectDefinition::Disenchant { .. }
                    | MeleeBlowEffectDefinition::Amnesia { .. }
                    | MeleeBlowEffectDefinition::Time { .. }
                    | MeleeBlowEffectDefinition::PolymorphPlayer { .. } => None,
                    MeleeBlowEffectDefinition::Unlife {
                        amount_dice,
                        amount_sides,
                        ..
                    } => {
                        let amount = u16::try_from(
                            self.roll_monster_melee_effect(
                                source_index,
                                *amount_dice,
                                *amount_sides,
                                false,
                            )
                            .max(0),
                        )
                        .unwrap_or(u16::MAX);
                        self.resolve_monster_unlife_against_actor(
                            &source_kind_id,
                            target_index,
                            amount,
                            events,
                            changed,
                        );
                        None
                    }
                    MeleeBlowEffectDefinition::Bleeding {
                        duration_dice,
                        duration_sides,
                        ..
                    } => {
                        let duration = self.roll_monster_melee_effect(
                            source_index,
                            *duration_dice,
                            *duration_sides,
                            false,
                        );
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
                        let raw = (*damage_dice > 0).then(|| {
                            self.roll_monster_melee_effect(
                                source_index,
                                *damage_dice,
                                *damage_sides,
                                false,
                            )
                        });
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
                    MeleeBlowEffectDefinition::Inertia { .. } => {
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
                        let duration = self.roll_monster_melee_effect(
                            source_index,
                            *duration_dice,
                            *duration_sides,
                            false,
                        );
                        self.apply_actor_melee_status(
                            target_index,
                            STATUS_STUN,
                            duration,
                            &source_kind_id,
                        );
                        None
                    }
                    MeleeBlowEffectDefinition::Terrify { .. } => {
                        let duration = resisted_status_duration(
                            u32::try_from(melee_terrify_duration(&definition)).unwrap_or(u32::MAX),
                            self.entities[target_index]
                                .resistances
                                .level(DamageType::Fear),
                        );
                        self.apply_actor_melee_status(
                            target_index,
                            STATUS_FEAR,
                            i32::try_from(duration).unwrap_or(i32::MAX),
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
                let shatters = matches!(effect, MeleeBlowEffectDefinition::Shatter { .. })
                    && damage.applied > 23;
                let quake_center = self.entities[source_index].position;
                let application = plan_damage_application(
                    &self.entities[target_index],
                    damage,
                    FatalityPolicy::AtOrBelowZero,
                );
                commit_damage_application(&mut self.entities[target_index], &application);
                self.wake_entity_after_damage(target_index, damage.applied, events);
                if application.fatal {
                    let slain = self.entities[target_index].clone();
                    self.reward_controlled_actor_kill(&source_entity_id, &slain, events);
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
                    if shatters {
                        self.resolve_monster_shatter_earthquake(
                            quake_center,
                            source_kind_id.clone(),
                            events,
                            changed,
                            removed_entities,
                        )?;
                    }
                    break;
                }
                if vampiric {
                    self.heal_vampiric_melee_source(&source_entity_id, damage.applied, changed);
                }
                events.push(DomainEvent::MonsterMeleeEntityHit {
                    source_kind_id: source_kind_id.clone(),
                    target_kind_id: target.kind_id().to_owned(),
                    method_id: blow.method_id.clone(),
                    damage,
                });
                if shatters {
                    self.resolve_monster_shatter_earthquake(
                        quake_center,
                        source_kind_id.clone(),
                        events,
                        changed,
                        removed_entities,
                    )?;
                }
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
        removed_entities: &mut Vec<String>,
    ) -> Result<bool, CoreError> {
        self.resolve_monster_melee_blow(index, None, events, changed, removed_entities)
    }

    pub(super) fn resolve_monster_revenge_blow(
        &mut self,
        index: usize,
        blow_index: usize,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<bool, CoreError> {
        self.resolve_monster_melee_blow(index, Some(blow_index), events, changed, removed_entities)
    }

    fn resolve_monster_melee_blow(
        &mut self,
        index: usize,
        only_blow_index: Option<usize>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<bool, CoreError> {
        let source_entity_id = self.entities[index].id.clone();
        let kind_id = self.entities[index].kind_id.clone();
        let nice = self.entities[index].nice;
        let definition = self
            .actor_runtime_definition(&self.entities[index])
            .expect("monster actor definition must remain available")
            .clone();
        let attacker = self.actor_derived_stats(&self.entities[index], &definition, false);
        let target = self.player_derived_stats();
        let armor_class = target.armor_class.value;
        let mut blink_after_melee = false;
        for (blow_index, blow) in resolved_melee_blows(&definition).into_iter().enumerate() {
            if only_blow_index.is_some_and(|selected| selected != blow_index) {
                continue;
            }
            let player_hp_before = self.player.hp;
            let ability = attacker.melee_skill.with_modifier(
                StatLayer::Base,
                blow.method_id.as_deref().unwrap_or(definition.id.as_str()),
                blow.to_hit,
                StatBounds::NON_NEGATIVE,
            );
            if !blow.effects.is_empty()
                && !resolve_check(
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
                self.check_human_dexterity_sprain(150, events);
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

            if blow.effects.is_empty() {
                events.push(DomainEvent::MonsterBegged {
                    source_kind_id: kind_id.clone(),
                });
                continue;
            }

            for effect in &blow.effects {
                if melee_effect_chance(effect)
                    .is_some_and(|chance| self.rng.bounded(100) >= u64::from(chance))
                {
                    continue;
                }
                let vampiric = matches!(
                    effect,
                    MeleeBlowEffectDefinition::Damage { vampiric: true, .. }
                ) && !self.player_is_nonliving();
                let mut inventory_damage_type = None;
                let damage = match effect {
                    MeleeBlowEffectDefinition::Damage {
                        damage_dice,
                        damage_sides,
                        damage_type,
                        armor_mitigated,
                        ..
                    } => {
                        let raw = self.roll_monster_melee_effect(
                            index,
                            *damage_dice,
                            *damage_sides,
                            nice,
                        );
                        if *damage_type == ActorDamageType::Curse
                            && self.monster_curse_save(&kind_id, events)
                        {
                            None
                        } else {
                            let damage_type = DamageType::from(*damage_type);
                            inventory_damage_type = Some(damage_type);
                            let resistance = self.effective_player_resistances().level(damage_type);
                            Some(self.reduce_player_damage(if *armor_mitigated {
                                resolve_armored_damage(raw, damage_type, armor_class, resistance)
                            } else {
                                resolve_damage(DamagePacket::new(raw, damage_type), resistance)
                            }))
                        }
                    }
                    MeleeBlowEffectDefinition::Shatter {
                        damage_dice,
                        damage_sides,
                        ..
                    } => {
                        let raw = self.roll_monster_melee_effect(
                            index,
                            *damage_dice,
                            *damage_sides,
                            nice,
                        );
                        Some(
                            self.reduce_player_damage(resolve_armored_damage(
                                raw,
                                DamageType::Physical,
                                armor_class,
                                self.effective_player_resistances()
                                    .level(DamageType::Physical),
                            )),
                        )
                    }
                    MeleeBlowEffectDefinition::Poison {
                        damage_dice,
                        damage_sides,
                        ..
                    } => {
                        let raw = self.roll_monster_melee_effect(
                            index,
                            *damage_dice,
                            *damage_sides,
                            nice,
                        );
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
                        let raw = self.roll_monster_melee_effect(
                            index,
                            *damage_dice,
                            *damage_sides,
                            nice,
                        );
                        Some(
                            self.reduce_player_damage(resolve_damage(
                                DamagePacket::new(raw, DamageType::Physical),
                                self.effective_player_resistances()
                                    .level(DamageType::Physical),
                            )),
                        )
                    }
                    MeleeBlowEffectDefinition::Bomb { .. } => {
                        unreachable!("bomb effects require a self-destructing blow")
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
                        let requested = u32::try_from(
                            self.roll_monster_melee_effect(
                                index,
                                *amount_dice,
                                *amount_sides,
                                nice,
                            )
                            .max(0),
                        )
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
                    MeleeBlowEffectDefinition::DrainCharges { .. } => {
                        if !self.player_has_device_charge_drain_immunity() {
                            let candidates = self
                                .items
                                .iter()
                                .enumerate()
                                .filter(|(_, item)| {
                                    item.location == ItemLocation::Inventory
                                        && item
                                            .charges
                                            .as_ref()
                                            .is_some_and(|charges| charges.current > 0)
                                })
                                .map(|(item_index, _)| item_index)
                                .collect::<Vec<_>>();
                            if candidates.is_empty() {
                                let before = self.nutrition;
                                self.nutrition =
                                    self.nutrition.saturating_sub(1_000).min(before * 2 / 3);
                                events.push(DomainEvent::MonsterNutritionDrained {
                                    source_kind_id: kind_id.clone(),
                                    amount: before.saturating_sub(self.nutrition),
                                });
                            } else {
                                let candidate =
                                    usize::try_from(self.rng.bounded(candidates.len() as u64))
                                        .expect("device candidate roll must fit usize");
                                let item_index = candidates[candidate];
                                let target_kind_id = self.items[item_index].kind_id.clone();
                                let charges = self.items[item_index]
                                    .charges
                                    .as_mut()
                                    .expect("selected device must retain charges");
                                let drained = charges.current.min(definition.level);
                                charges.current -= drained;
                                let caster = &mut self.entities[index];
                                caster.hp =
                                    caster.max_hp.min(caster.hp.saturating_add(
                                        i32::try_from(drained).unwrap_or(i32::MAX),
                                    ));
                                changed.insert(caster.position);
                                events.push(DomainEvent::MonsterChargesDrained {
                                    source_kind_id: kind_id.clone(),
                                    target_kind_id,
                                    amount: drained,
                                });
                            }
                        }
                        None
                    }
                    MeleeBlowEffectDefinition::DrainExperience {
                        amount_dice,
                        amount_sides,
                        ..
                    } => {
                        let rolled = self.roll_monster_melee_effect(
                            index,
                            *amount_dice,
                            *amount_sides,
                            nice,
                        );
                        let requested = u64::try_from(rolled.max(0))
                            .unwrap_or(u64::MAX)
                            .saturating_add(self.progress.experience.saturating_mul(2) / 100)
                            .min(25_000);
                        self.apply_player_experience_drain(requested, &kind_id, events);
                        None
                    }
                    MeleeBlowEffectDefinition::Unlife {
                        amount_dice,
                        amount_sides,
                        ..
                    } => {
                        let amount = u16::try_from(
                            self.roll_monster_melee_effect(
                                index,
                                *amount_dice,
                                *amount_sides,
                                nice,
                            )
                            .max(0),
                        )
                        .unwrap_or(u16::MAX);
                        self.resolve_monster_unlife_against_player(index, amount, events, changed);
                        None
                    }
                    MeleeBlowEffectDefinition::Bleeding {
                        duration_dice,
                        duration_sides,
                        ..
                    } => {
                        let duration = self.roll_monster_melee_effect(
                            index,
                            *duration_dice,
                            *duration_sides,
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
                            self.roll_monster_melee_effect(index, *damage_dice, *damage_sides, nice)
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
                    MeleeBlowEffectDefinition::Amnesia { .. } => {
                        if !self.monster_curse_save(&kind_id, events) {
                            let cleared_cells = self.clear_current_floor_memory(changed);
                            events.push(DomainEvent::MonsterMeleeAmnesia {
                                source_kind_id: kind_id.clone(),
                                cleared_cells,
                            });
                        }
                        None
                    }
                    MeleeBlowEffectDefinition::Time { .. } => {
                        self.resolve_time_melee(&kind_id, events);
                        None
                    }
                    MeleeBlowEffectDefinition::Slow { .. } => {
                        self.apply_player_melee_status(STATUS_SLOW, 25, &kind_id);
                        None
                    }
                    MeleeBlowEffectDefinition::Inertia { .. } => {
                        let amount = if self.player_status_immunities().contains(STATUS_PARALYSIS) {
                            1
                        } else {
                            5
                        };
                        self.minor_slow = self.minor_slow.saturating_add(amount).min(10);
                        None
                    }
                    MeleeBlowEffectDefinition::PolymorphPlayer { .. } => {
                        self.resolve_player_polymorph(&kind_id, definition.level, events);
                        None
                    }
                    MeleeBlowEffectDefinition::Stun {
                        duration_dice,
                        duration_sides,
                        ..
                    } => {
                        let duration = self.roll_monster_melee_effect(
                            index,
                            *duration_dice,
                            *duration_sides,
                            nice,
                        );
                        self.apply_player_melee_status(STATUS_STUN, duration, &kind_id);
                        None
                    }
                    MeleeBlowEffectDefinition::Terrify { .. } => {
                        let duration = resisted_status_duration(
                            u32::try_from(melee_terrify_duration(&definition)).unwrap_or(u32::MAX),
                            self.effective_player_resistances().level(DamageType::Fear),
                        );
                        self.apply_player_melee_status(
                            STATUS_FEAR,
                            i32::try_from(duration).unwrap_or(i32::MAX),
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
                let shatters = matches!(effect, MeleeBlowEffectDefinition::Shatter { .. })
                    && damage.applied > 23;
                let quake_center = self.entities[index].position;
                let application = self.apply_final_player_damage(damage, FatalityPolicy::BelowZero);
                let damage = application.damage;
                if let Some(damage_type) = inventory_damage_type {
                    self.damage_player_inventory(
                        &kind_id,
                        damage_type,
                        true,
                        damage.applied,
                        events,
                    );
                }
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
                if vampiric {
                    self.heal_vampiric_melee_source(&source_entity_id, damage.applied, changed);
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
                if shatters {
                    self.resolve_monster_shatter_earthquake(
                        quake_center,
                        kind_id.clone(),
                        events,
                        changed,
                        removed_entities,
                    )?;
                    if self.player_is_dead() {
                        return Ok(false);
                    }
                }
            }
            let blow_damage = player_hp_before
                .saturating_sub(self.player.hp)
                .clamp(0, 200);
            if blow_damage > 0 {
                self.resolve_riding_fall(blow_damage, false, events, changed);
                if self.player_is_dead() {
                    return Ok(false);
                }
            }
            if melee_method_triggers_contact_aura(blow.method_id.as_deref())
                && self.resolve_mutation_contact_auras(index, events, changed, removed_entities)?
            {
                return Ok(false);
            }
        }
        if blink_after_melee && self.player.hp > 0 {
            self.blink_monster_after_theft(index, events, changed);
        }
        Ok(false)
    }

    fn resolve_mutation_contact_auras(
        &mut self,
        target_index: usize,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<bool, CoreError> {
        for damage_type in [DamageType::Fire, DamageType::Electricity, DamageType::Cold] {
            let level = self
                .content
                .mutations()
                .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
                .filter(|mutation| mutation.contact_aura.map(DamageType::from) == Some(damage_type))
                .count()
                + usize::from(self.player_has_status_kind(STATUS_ULTIMATE_RESISTANCE));
            if level == 0 {
                continue;
            }
            let resistance = self.entities[target_index].resistances.level(damage_type);
            if matches!(
                resistance,
                ResistanceLevel::Resistant | ResistanceLevel::Strong | ResistanceLevel::Immune
            ) {
                continue;
            }
            let level = u16::try_from(level).expect("validated mutation count must fit u16");
            let player_level = self.progress.level / 10;
            let raw = 2_i32.saturating_add(
                self.roll_damage(
                    level
                        .saturating_mul(2)
                        .saturating_sub(1)
                        .saturating_add(player_level),
                    2_u16.saturating_add(player_level),
                ),
            );
            let damage =
                resolve_damage(DamagePacket::new(raw, damage_type), ResistanceLevel::Normal);
            let target_kind_id = self.entities[target_index].kind_id.clone();
            let application = plan_damage_application(
                &self.entities[target_index],
                damage,
                FatalityPolicy::AtOrBelowZero,
            );
            commit_damage_application(&mut self.entities[target_index], &application);
            if application.fatal {
                self.resolve_actor_death(
                    target_index,
                    DomainEvent::MutationAuraSlew {
                        target_kind_id,
                        damage,
                    },
                    events,
                    changed,
                    removed_entities,
                )?;
                return Ok(true);
            }
            events.push(DomainEvent::MutationAuraHit {
                target_kind_id,
                damage,
            });
            self.wake_entity_after_damage(target_index, damage.applied, events);
        }
        Ok(false)
    }

    pub(super) fn resolve_time_melee(
        &mut self,
        source_kind_id: &str,
        events: &mut Vec<DomainEvent>,
    ) {
        let resistance = self
            .effective_player_resistances()
            .level(DamageType::Time)
            .reduction_percent()
            .max(0) as u64;
        if self.rng.bounded(100) < resistance {
            return;
        }
        match self.rng.bounded(10) {
            0..=4 => {
                let amount =
                    100_u64.saturating_add(self.progress.experience.saturating_mul(2) / 100);
                self.apply_player_experience_drain(amount, source_kind_id, events);
            }
            5..=8 => {
                let attributes = [
                    AttributeKind::Strength,
                    AttributeKind::Intelligence,
                    AttributeKind::Wisdom,
                    AttributeKind::Dexterity,
                    AttributeKind::Constitution,
                    AttributeKind::Charisma,
                ];
                let index = usize::try_from(self.rng.bounded(attributes.len() as u64))
                    .expect("time attribute roll must fit usize");
                self.ravage_time_attributes(&[attributes[index]], 3, 4);
                events.push(DomainEvent::MonsterTimeRavaged {
                    source_kind_id: source_kind_id.to_owned(),
                    attribute_count: 1,
                });
            }
            _ => {
                self.ravage_time_attributes(
                    &[
                        AttributeKind::Strength,
                        AttributeKind::Intelligence,
                        AttributeKind::Wisdom,
                        AttributeKind::Dexterity,
                        AttributeKind::Constitution,
                        AttributeKind::Charisma,
                    ],
                    7,
                    8,
                );
                events.push(DomainEvent::MonsterTimeRavaged {
                    source_kind_id: source_kind_id.to_owned(),
                    attribute_count: 6,
                });
            }
        }
    }

    fn ravage_time_attributes(
        &mut self,
        attributes: &[AttributeKind],
        numerator: u16,
        denominator: u16,
    ) {
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let mut changed = false;
        for attribute in attributes {
            let current = self.progress.attributes.value(*attribute);
            let next = current.saturating_mul(numerator) / denominator;
            let next = next.max(3);
            changed |= current != next;
            match attribute {
                AttributeKind::Strength => self.progress.attributes.strength = next,
                AttributeKind::Intelligence => self.progress.attributes.intelligence = next,
                AttributeKind::Wisdom => self.progress.attributes.wisdom = next,
                AttributeKind::Dexterity => self.progress.attributes.dexterity = next,
                AttributeKind::Constitution => self.progress.attributes.constitution = next,
                AttributeKind::Charisma => self.progress.attributes.charisma = next,
            }
        }
        if changed {
            self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        }
    }

    pub(super) fn resolve_monster_attribute_drain(&mut self, attribute: AttributeKind) {
        if self.player_sustains_attribute(attribute) {
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

fn melee_method_triggers_contact_aura(method_id: Option<&str>) -> bool {
    matches!(
        method_id,
        None | Some(
            "rfb.blow.hit"
                | "rfb.blow.touch"
                | "rfb.blow.punch"
                | "rfb.blow.kick"
                | "rfb.blow.claw"
                | "rfb.blow.bite"
                | "rfb.blow.sting"
                | "rfb.blow.slash"
                | "rfb.blow.butt"
                | "rfb.blow.crush"
                | "rfb.blow.engulf"
                | "rfb.blow.charge"
                | "rfb.blow.crawl"
        )
    )
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
