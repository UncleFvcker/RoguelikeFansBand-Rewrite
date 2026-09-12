// SPDX-License-Identifier: MPL-2.0

use crate::effect::{
    DamagePacket, STATUS_BLEEDING, STATUS_DEVICE_MASTERY, STATUS_HOLD_LIFE, STATUS_INVULNERABILITY,
    STATUS_POISON, STATUS_SUSTAIN_CHARISMA, STATUS_SUSTAIN_CONSTITUTION, STATUS_SUSTAIN_DEXTERITY,
    STATUS_SUSTAIN_INTELLIGENCE, STATUS_SUSTAIN_STRENGTH, STATUS_SUSTAIN_WISDOM, StatusInstance,
    resolve_damage,
};
use crate::error::CoreError;
use crate::event::DomainEvent;
use crate::game::Game;
use crate::game::ability_scaling::spell_powered_ability_value;
use crate::game::damage::FatalityPolicy;
use crate::game::progression::{LifeForceRestorationRequest, apply_experience_restoration};
use crate::game::status_effects::apply_ability_status_effect;
use crate::resistance::{DamageType, ResistanceLevel};
use crate::stats::{AttributeKind, AttributeSet, CharacterProgress, drain_attribute_value};
use rfb_content::{
    AbilityDefinition, AbilityEffectDefinition, AbilitySpellPowerField,
    AbilityStatusStackingDefinition, ActorDamageType, ActorResistanceLevel, EquipmentBonuses,
    StatModifiers,
};
use rfb_protocol::{
    AbilityEffectResolutionDto, AbilityEffectsResolutionDto, HealingResolutionDto, Position,
};
use std::collections::{BTreeMap, BTreeSet};

fn attribute_kind_dto(kind: AttributeKind) -> rfb_protocol::AttributeKindDto {
    match kind {
        AttributeKind::Strength => rfb_protocol::AttributeKindDto::Strength,
        AttributeKind::Intelligence => rfb_protocol::AttributeKindDto::Intelligence,
        AttributeKind::Wisdom => rfb_protocol::AttributeKindDto::Wisdom,
        AttributeKind::Dexterity => rfb_protocol::AttributeKindDto::Dexterity,
        AttributeKind::Constitution => rfb_protocol::AttributeKindDto::Constitution,
        AttributeKind::Charisma => rfb_protocol::AttributeKindDto::Charisma,
    }
}

fn set_attribute_value(attributes: &mut AttributeSet, kind: AttributeKind, value: u16) {
    match kind {
        AttributeKind::Strength => attributes.strength = value,
        AttributeKind::Intelligence => attributes.intelligence = value,
        AttributeKind::Wisdom => attributes.wisdom = value,
        AttributeKind::Dexterity => attributes.dexterity = value,
        AttributeKind::Constitution => attributes.constitution = value,
        AttributeKind::Charisma => attributes.charisma = value,
    }
}

impl Game {
    pub(super) fn resolve_player_ring_of_power_backlash(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::RingOfPowerBacklash { effect_index: 0 }],
            },
            trace: None,
        });
        // cmd6.c ring_of_power calls dec_stat directly, bypassing sustains.
        // Keep effects.c's virtue rolls between the current and maximum rolls.
        for attribute in [
            AttributeKind::Strength,
            AttributeKind::Intelligence,
            AttributeKind::Wisdom,
            AttributeKind::Dexterity,
            AttributeKind::Constitution,
            AttributeKind::Charisma,
        ] {
            let current = self.progress.attributes.value(attribute);
            let maximum = self.progress.maximum_attributes.value(attribute);
            let next = drain_attribute_value(current, 50, &mut self.rng);
            if maximum > 3 {
                self.add_virtue(rfb_protocol::VirtueKindDto::Sacrifice, 1);
                if matches!(
                    attribute,
                    AttributeKind::Intelligence | AttributeKind::Wisdom
                ) {
                    self.add_virtue(rfb_protocol::VirtueKindDto::Enlightenment, -2);
                }
            }
            let mut next_maximum = drain_attribute_value(maximum, 50, &mut self.rng);
            if current == maximum || next_maximum < next {
                next_maximum = next;
            }
            set_attribute_value(&mut self.progress.attributes, attribute, next);
            set_attribute_value(
                &mut self.progress.maximum_attributes,
                attribute,
                next_maximum,
            );
        }
        let lost_experience = self.progress.experience / 4;
        self.progress.experience -= lost_experience;
        self.progress.maximum_experience -= self.progress.experience / 4;
        let lost_levels = self.progress.lose_experience(
            0,
            self.character_experience_percent(),
            self.victory_level_cap_unlocked(),
        );
        self.refresh_character_skills();
        self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        events.push(DomainEvent::ExperienceDrained {
            source_kind_id: ability.id.clone(),
            amount: lost_experience,
            total: self.progress.experience,
        });
        for level in lost_levels {
            events.push(DomainEvent::PlayerLevelLost {
                level,
                max_hp: self.player_max_hp_at_level(level),
            });
        }
    }

    pub(super) fn resolve_player_resource_conversion(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let resource_id = self
            .casting_profile()
            .expect("conversion requires casting profile")
            .resource_id
            .clone();
        let hp_before = self.player.hp;
        let resource_before = self.resources[&resource_id].current;
        let level = u32::from(self.progress.level);
        let converted = match ability.effect {
            AbilityEffectDefinition::HealthToMana => {
                // DAMAGE_USELIFE bypasses ordinary defenses, but take_hit still applies Transcendence.
                let damage = resolve_damage(
                    DamagePacket::new(level as i32, DamageType::Physical),
                    ResistanceLevel::Normal,
                );
                let damage = self
                    .apply_final_player_damage(damage, FatalityPolicy::BelowZero)
                    .damage
                    .applied;
                let gain = damage as u32 / 5;
                let pool = self
                    .resources
                    .get_mut(&resource_id)
                    .expect("casting resource exists");
                pool.current = pool.current.saturating_add(gain).min(pool.maximum);
                gain > 0
            }
            AbilityEffectDefinition::ManaToHealth => {
                let cost = level / 5;
                if resource_before >= cost {
                    self.resources
                        .get_mut(&resource_id)
                        .expect("casting resource exists")
                        .current -= cost;
                    self.apply_player_healing(level as i32);
                    true
                } else {
                    false
                }
            }
            _ => unreachable!("resource conversion effect"),
        };
        events.push(DomainEvent::AbilityResourceConverted {
            ability_id: ability.id.clone(),
            resolution: rfb_protocol::ResourceConversionResolutionDto {
                resource_id: resource_id.clone(),
                hp_before,
                hp_after: self.player.hp,
                resource_before,
                resource_after: self.resources[&resource_id].current,
                converted,
                fatal: self.player_is_dead(),
            },
        });
    }

    pub(in crate::game) fn ability_element_targets(
        &self,
        ability: &AbilityDefinition,
    ) -> Vec<rfb_protocol::DamageTypeDto> {
        use rfb_protocol::DamageTypeDto as E;
        match ability.effect {
            AbilityEffectDefinition::ElementalBrand => [
                (0, E::Fire),
                (30, E::Cold),
                (35, E::Poison),
                (40, E::Acid),
                (45, E::Electricity),
            ]
            .into_iter()
            .filter_map(|(level, element)| (self.progress.level >= level).then_some(element))
            .collect(),
            AbilityEffectDefinition::ElementalImmunity { .. } => {
                vec![E::Fire, E::Cold, E::Acid, E::Electricity]
            }
            _ => Vec::new(),
        }
    }

    pub(super) fn resolve_player_elemental_enchantment(
        &mut self,
        ability: &AbilityDefinition,
        element: rfb_protocol::DamageTypeDto,
        events: &mut Vec<DomainEvent>,
    ) {
        use rfb_protocol::DamageTypeDto as E;
        let damage_type = match element {
            E::Fire => ActorDamageType::Fire,
            E::Cold => ActorDamageType::Cold,
            E::Poison => ActorDamageType::Poison,
            E::Acid => ActorDamageType::Acid,
            E::Electricity => ActorDamageType::Electricity,
            _ => unreachable!("validated elemental choice"),
        };
        let mut brands = BTreeSet::new();
        let mut resistances = BTreeMap::new();
        let (kind, base) = match ability.effect {
            AbilityEffectDefinition::ElementalBrand => {
                brands.insert(match element {
                    E::Fire => rfb_content::WeaponBrand::Fire,
                    E::Cold => rfb_content::WeaponBrand::Cold,
                    E::Poison => rfb_content::WeaponBrand::Poison,
                    E::Acid => rfb_content::WeaponBrand::Acid,
                    E::Electricity => rfb_content::WeaponBrand::Electricity,
                    _ => unreachable!("validated elemental brand"),
                });
                (
                    "rfb.status.elemental-brand",
                    u32::from(self.progress.level / 2),
                )
            }
            AbilityEffectDefinition::ElementalImmunity { duration_base } => {
                resistances.insert(damage_type, ActorResistanceLevel::Immune);
                ("rfb.status.elemental-immunity", duration_base)
            }
            _ => unreachable!("elemental enchantment effect"),
        };
        // A single status per family replaces the previous choice and duration.
        let resolution = apply_ability_status_effect(
            &mut self.player,
            &ability.id,
            0,
            kind,
            1,
            base,
            1,
            base,
            AbilityStatusStackingDefinition::Replace,
            None,
            None,
            &resistances,
            &brands,
            &StatModifiers::default(),
            &EquipmentBonuses::default(),
            &BTreeSet::new(),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![resolution],
            },
            trace: None,
        });
    }

    pub(in crate::game) fn resolve_player_healing_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::Heal { amount } = ability.effect else {
            unreachable!("player healing executor requires a healing effect");
        };
        let amount = i32::try_from(amount).expect("validated healing amount must fit i32");
        let outcome = self.apply_player_healing(amount);
        events.push(DomainEvent::AbilityHealed {
            ability_id: ability.id.clone(),
            resolution: HealingResolutionDto {
                requested: outcome.requested,
                applied: outcome.applied,
            },
        });
    }

    pub(super) fn resolve_player_healing_dice_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::HealDice { dice, sides } = ability.effect else {
            unreachable!("player healing-dice executor requires a healing-dice effect");
        };
        let amount = self.roll_damage(dice, sides).max(0);
        let amount = i32::try_from(spell_powered_ability_value(
            ability,
            0,
            AbilitySpellPowerField::FinalHealing,
            u64::try_from(amount).expect("healing must be non-negative"),
        ))
        .expect("spell-powered healing must fit i32");
        let outcome = self.apply_player_healing(amount);
        events.push(DomainEvent::AbilityHealed {
            ability_id: ability.id.clone(),
            resolution: HealingResolutionDto {
                requested: outcome.requested,
                applied: outcome.applied,
            },
        });
    }

    pub(super) fn resolve_player_status_reduction_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::ReduceStatus {
            ref status_kind_id,
            amount,
            current_divisor,
            remaining_divisor,
        } = ability.effect
        else {
            unreachable!("status reduction executor requires a reduce-status effect");
        };
        let (before, after) =
            self.reduce_player_status(status_kind_id, amount, current_divisor, remaining_divisor);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::ReduceStatus {
                    effect_index: 0,
                    status_kind_id: status_kind_id.clone(),
                    before,
                    after,
                }],
            },
            trace: None,
        });
    }

    pub(super) fn reduce_player_status(
        &mut self,
        status_kind_id: &str,
        minimum_amount: u32,
        current_divisor: Option<u32>,
        remaining_divisor: Option<u32>,
    ) -> (u32, u32) {
        self.player
            .statuses
            .iter()
            .position(|status| status.kind_id == status_kind_id)
            .map_or((0, 0), |index| {
                let before = self.player.statuses[index].remaining_ticks;
                let after = remaining_divisor.map_or_else(
                    || {
                        let amount = current_divisor.map_or(minimum_amount, |divisor| {
                            minimum_amount.max(before / divisor)
                        });
                        before.saturating_sub(amount)
                    },
                    |divisor| (before / divisor).saturating_sub(minimum_amount),
                );
                if after == 0 {
                    self.player.statuses.remove(index);
                } else {
                    self.player.statuses[index].remaining_ticks = after;
                }
                (before, after)
            })
    }

    pub(super) fn resolve_player_satisfy_hunger_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let before_state = self.nutrition_state();
        let nutrition_before = self.nutrition;
        if rfb_protocol::PLAYER_NUTRITION_MAXIMUM - 1 > self.nutrition {
            self.fasting = false;
        }
        self.nutrition = rfb_protocol::PLAYER_NUTRITION_MAXIMUM - 1;
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::SatisfyHunger {
                    effect_index: 0,
                    nutrition_before,
                    nutrition_after: self.nutrition,
                }],
            },
            trace: None,
        });
        let after_state = self.nutrition_state();
        if after_state != before_state {
            events.push(DomainEvent::NutritionStateChanged {
                from: before_state,
                to: after_state,
                nutrition: self.nutrition,
            });
        }
    }

    pub(super) fn resolve_player_devour_flesh_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::DevourFlesh {
            maximum_hp_divisor,
            bleeding_amount,
        } = ability.effect
        else {
            unreachable!("devour flesh executor requires a devour-flesh effect");
        };
        let before_state = self.nutrition_state();
        let nutrition_before = self.nutrition;
        self.nutrition = rfb_protocol::PLAYER_NUTRITION_MAXIMUM - 1;
        if self.nutrition > nutrition_before {
            self.fasting = false;
        }
        let resistances = self.effective_player_resistances();
        let immunities = self.player_status_immunities();
        let bleeding = apply_ability_status_effect(
            &mut self.player,
            &ability.id,
            1,
            STATUS_BLEEDING,
            1,
            bleeding_amount,
            0,
            0,
            AbilityStatusStackingDefinition::Extend,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers::default(),
            &EquipmentBonuses::default(),
            &BTreeSet::new(),
            None,
            false,
            100,
            None,
            Some((&resistances, &immunities, None)),
            &mut self.rng,
        );
        let damage = self.effective_player_max_hp() / i32::from(maximum_hp_divisor);
        self.player.hp = self.player.hp.saturating_sub(damage);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![
                    AbilityEffectResolutionDto::SatisfyHunger {
                        effect_index: 0,
                        nutrition_before,
                        nutrition_after: self.nutrition,
                    },
                    bleeding,
                    AbilityEffectResolutionDto::SelfDamage {
                        effect_index: 2,
                        damage,
                        fatal: self.player_is_dead(),
                    },
                ],
            },
            trace: None,
        });
        let after_state = self.nutrition_state();
        if after_state != before_state {
            events.push(DomainEvent::NutritionStateChanged {
                from: before_state,
                to: after_state,
                nutrition: self.nutrition,
            });
        }
    }

    pub(super) fn resolve_player_vomit_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        debug_assert!(matches!(ability.effect, AbilityEffectDefinition::Vomit));
        const EMPTY_STOMACH_THRESHOLD: u16 = 524;
        const MAXIMUM_NUTRITION_AFTER_VOMITING: u16 = 512;
        const EMPTY_STOMACH_DAMAGE: i32 = 10;
        const EMPTY_STOMACH_EXTRA_ENERGY: u16 = 15;

        let before_state = self.nutrition_state();
        let nutrition_before = self.nutrition;
        let poison_before = self
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_POISON)
            .map_or(0, |status| status.remaining_ticks);
        let poison_damage = i32::try_from(poison_before.saturating_mul(2) / 7).unwrap_or(i32::MAX);
        let empty_stomach = nutrition_before < EMPTY_STOMACH_THRESHOLD;
        let self_damage = if empty_stomach {
            let damage = resolve_damage(
                DamagePacket::new(EMPTY_STOMACH_DAMAGE, DamageType::Physical),
                ResistanceLevel::Normal,
            );
            self.apply_final_player_damage(damage, FatalityPolicy::BelowZero)
                .damage
                .applied
        } else {
            0
        };

        self.nutrition = nutrition_before
            .saturating_sub(100)
            .clamp(1, MAXIMUM_NUTRITION_AFTER_VOMITING);
        self.resolve_player_area_damage_with_base(
            &ability.id,
            Vec::new(),
            false,
            DamageType::Poison,
            1,
            None,
            poison_damage,
            false,
            events,
            changed,
            removed_entities,
        )?;
        self.player
            .statuses
            .retain(|status| status.kind_id != STATUS_POISON);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::Vomit {
                    effect_index: 0,
                    nutrition_before,
                    nutrition_after: self.nutrition,
                    poison_before,
                    poison_damage,
                    poison_removed: poison_before > 0,
                    empty_stomach,
                    self_damage,
                    fatal: self.player_is_dead(),
                    extra_energy_cost: if empty_stomach {
                        EMPTY_STOMACH_EXTRA_ENERGY
                    } else {
                        0
                    },
                }],
            },
            trace: None,
        });
        let after_state = self.nutrition_state();
        if after_state != before_state {
            events.push(DomainEvent::NutritionStateChanged {
                from: before_state,
                to: after_state,
                nutrition: self.nutrition,
            });
        }
        Ok(())
    }

    pub(super) fn resolve_player_begin_fasting_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        debug_assert!(!self.fasting);
        let before_state = self.nutrition_state();
        let nutrition_before = self.nutrition;
        self.nutrition /= 2;
        self.fasting = true;
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::BeginFasting {
                    effect_index: 0,
                    nutrition_before,
                    nutrition_after: self.nutrition,
                }],
            },
            trace: None,
        });
        let after_state = self.nutrition_state();
        if after_state != before_state {
            events.push(DomainEvent::NutritionStateChanged {
                from: before_state,
                to: after_state,
                nutrition: self.nutrition,
            });
        }
    }

    pub(super) fn resolve_player_sustain_attributes_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::SustainAttributes { duration_ticks } = ability.effect else {
            unreachable!("attribute sustain executor requires its matching effect");
        };
        let mut remaining = self.progress.level / 7;
        let mut selected = Vec::new();
        for (denominator, status_kind_id) in [
            (7, STATUS_HOLD_LIFE),
            (6, STATUS_SUSTAIN_CONSTITUTION),
            (5, STATUS_SUSTAIN_STRENGTH),
            (4, STATUS_SUSTAIN_INTELLIGENCE),
            (3, STATUS_SUSTAIN_DEXTERITY),
            (2, STATUS_SUSTAIN_WISDOM),
        ] {
            if self.rng.bounded(denominator) < u64::from(remaining) {
                selected.push(status_kind_id);
                remaining = remaining.saturating_sub(1);
            }
        }
        if remaining > 0 {
            selected.push(STATUS_SUSTAIN_CHARISMA);
        }
        for status_kind_id in &selected {
            let _ = apply_ability_status_effect(
                &mut self.player,
                &ability.id,
                0,
                status_kind_id,
                1,
                duration_ticks,
                0,
                0,
                AbilityStatusStackingDefinition::KeepStrongest,
                None,
                None,
                &BTreeMap::new(),
                &BTreeSet::new(),
                &StatModifiers::default(),
                &EquipmentBonuses::default(),
                &BTreeSet::new(),
                None,
                false,
                100,
                None,
                None,
                &mut self.rng,
            );
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::SustainAttributes {
                    effect_index: 0,
                    duration_ticks,
                    status_kind_ids: selected.into_iter().map(str::to_owned).collect(),
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_cure_mutation_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let denominator = u64::from((100 / self.progress.level.max(1)).max(1));
        let harmful_only = self.rng.bounded(denominator) == 0;
        let removed_mutation_id = self.cure_random_mutation(harmful_only, events);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::CureMutation {
                    effect_index: 0,
                    harmful_only,
                    removed_mutation_id,
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_concentrate_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let maximum = self
            .sniper_max_concentration()
            .expect("validated concentrate ability requires a sniping profile");
        let before = self.sniper_concentration;
        self.sniper_concentration = before.saturating_add(1).min(maximum);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::Concentrate {
                    effect_index: 0,
                    before,
                    after: self.sniper_concentration,
                    maximum,
                }],
            },
            trace: None,
        });
    }

    pub(in crate::game) fn resolve_player_restore_vitality_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::RestoreVitality {
            life_force,
            restore_attributes,
        } = ability.effect
        else {
            unreachable!("vitality executor requires a restore vitality effect");
        };
        if restore_attributes {
            self.restore_all_player_attributes();
        }
        let experience = apply_experience_restoration(&mut self.progress);
        let life_force =
            self.restore_player_life_force(LifeForceRestorationRequest::add(life_force));
        self.apply_player_experience(0, events);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::RestoreVitality {
                    effect_index: 0,
                    experience_before: experience.before,
                    experience_after: experience.after,
                    life_force_before: life_force.before,
                    life_force_after: life_force.after,
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_self_knowledge_effect(
        &self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        events.push(DomainEvent::AbilitySelfKnowledge {
            ability_id: ability.id.clone(),
            name_key: ability.name_key.clone(),
            report: self.self_knowledge_report(),
        });
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::SelfKnowledge { effect_index: 0 }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_device_mastery_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::DeviceMastery {
            duration_base,
            device_power_bonus,
        } = ability.effect
        else {
            unreachable!("device-mastery executor requires a device-mastery effect");
        };
        let resolution = apply_ability_status_effect(
            &mut self.player,
            &ability.id,
            0,
            STATUS_DEVICE_MASTERY,
            1,
            u32::from(duration_base),
            1,
            u32::from(duration_base),
            AbilityStatusStackingDefinition::KeepStrongest,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers {
                device_power_bonus,
                ..StatModifiers::default()
            },
            &EquipmentBonuses::default(),
            &BTreeSet::new(),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        let AbilityEffectResolutionDto::ApplyStatus {
            applied_duration_ticks,
            change,
            ..
        } = resolution
        else {
            unreachable!("device mastery must resolve as a status");
        };
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::DeviceMastery {
                    effect_index: 0,
                    duration_base,
                    duration_ticks: applied_duration_ticks,
                    device_power_bonus,
                    change,
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_invulnerability_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::Invulnerability {
            duration_dice,
            duration_sides,
            duration_bonus,
        } = ability.effect
        else {
            unreachable!("invulnerability executor requires an invulnerability effect");
        };
        let rolled = u64::try_from(self.roll_damage(duration_dice, duration_sides))
            .expect("validated invulnerability duration must be non-negative")
            .saturating_add(u64::from(duration_bonus));
        let duration_ticks = u32::try_from(spell_powered_ability_value(
            ability,
            0,
            AbilitySpellPowerField::InvulnerabilityDuration,
            rolled,
        ))
        .expect("spell-powered invulnerability duration must fit u32");
        let was_invulnerable = self.player_has_status_kind(STATUS_INVULNERABILITY);
        let resolution = apply_ability_status_effect(
            &mut self.player,
            &ability.id,
            0,
            STATUS_INVULNERABILITY,
            1,
            duration_ticks,
            0,
            0,
            AbilityStatusStackingDefinition::KeepStrongest,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers::default(),
            &EquipmentBonuses::default(),
            &BTreeSet::new(),
            None,
            false,
            0,
            None,
            None,
            &mut self.rng,
        );
        let AbilityEffectResolutionDto::ApplyStatus {
            applied_duration_ticks,
            change,
            ..
        } = resolution
        else {
            unreachable!("invulnerability must resolve as a status");
        };
        if !was_invulnerable && applied_duration_ticks > 0 {
            self.apply_invulnerability_opening_virtues();
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::Invulnerability {
                    effect_index: 0,
                    duration_ticks: applied_duration_ticks,
                    change,
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_report_magic_effect(
        &self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let mut statuses = self
            .player
            .statuses
            .iter()
            .map(StatusInstance::to_dto)
            .collect::<Vec<_>>();
        statuses.sort_by(|left, right| left.kind_id.cmp(&right.kind_id));
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::ReportMagic {
                    effect_index: 0,
                    statuses,
                    recall: self.recall.clone(),
                }],
            },
            trace: None,
        });
    }

    pub(in crate::game) fn resolve_player_polymorph_self_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        const ATTRIBUTES: [AttributeKind; 6] = [
            AttributeKind::Strength,
            AttributeKind::Intelligence,
            AttributeKind::Wisdom,
            AttributeKind::Dexterity,
            AttributeKind::Constitution,
            AttributeKind::Charisma,
        ];

        let active_before = self.progress.active_mutation_ids.clone();
        let hp_before = self.player.hp;
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let mut power = i32::from(self.progress.level);

        if power > i32::try_from(self.rng.bounded(30)).expect("polymorph roll must fit i32")
            && self.rng.bounded(6) == 0
        {
            power -= 20;
            for attribute in ATTRIBUTES {
                let amount = u8::try_from(self.rng.bounded(6) + 7)
                    .expect("polymorph attribute drain must fit u8");
                self.progress
                    .permanently_drain_attribute(attribute, amount, &mut self.rng);
            }
            if self.rng.bounded(6) == 0 {
                let dice = u16::try_from(self.rng.bounded(10) + 1)
                    .expect("polymorph life-loss dice must fit u16");
                let damage = resolve_damage(
                    DamagePacket::new(
                        self.roll_damage(dice, self.progress.level.max(1)),
                        DamageType::Physical,
                    ),
                    ResistanceLevel::Normal,
                );
                self.apply_final_player_damage(damage, FatalityPolicy::BelowZero);
                power -= 10;
            }
        }

        if power > i32::try_from(self.rng.bounded(20)).expect("polymorph roll must fit i32")
            && self.rng.bounded(4) == 0
        {
            power -= 10;
            let base_max_hp = self
                .progress
                .hp_progression
                .first()
                .copied()
                .unwrap_or(self.player.max_hp);
            self.progress.hp_progression =
                CharacterProgress::roll_hp_progression(base_max_hp, &mut self.rng);
        }

        while power > i32::try_from(self.rng.bounded(15)).expect("polymorph roll must fit i32")
            && self.rng.bounded(3) == 0
        {
            power -= 7;
            if self.gain_random_mutation_without_refresh(events).is_none() {
                break;
            }
        }

        if power > i32::try_from(self.rng.bounded(5)).expect("polymorph roll must fit i32") {
            power -= 5;
            self.resolve_polymorph_wounds(&ability.id);
        }

        let mut swapped_attributes = Vec::new();
        while power > 0 {
            let left_index = usize::try_from(self.rng.bounded(6))
                .expect("polymorph attribute index must fit usize");
            let mut right_index = usize::try_from(self.rng.bounded(5))
                .expect("polymorph attribute index must fit usize");
            if right_index >= left_index {
                right_index += 1;
            }
            let left = ATTRIBUTES[left_index];
            let right = ATTRIBUTES[right_index];
            let left_current = self.progress.attributes.value(left);
            let right_current = self.progress.attributes.value(right);
            let left_maximum = self.progress.maximum_attributes.value(left);
            let right_maximum = self.progress.maximum_attributes.value(right);
            let left_cap = self.progress.attribute_potentials.value(left);
            let right_cap = self.progress.attribute_potentials.value(right);
            let next_left_maximum = right_maximum.min(left_cap);
            let next_right_maximum = left_maximum.min(right_cap);
            set_attribute_value(
                &mut self.progress.maximum_attributes,
                left,
                next_left_maximum,
            );
            set_attribute_value(
                &mut self.progress.maximum_attributes,
                right,
                next_right_maximum,
            );
            set_attribute_value(
                &mut self.progress.attributes,
                left,
                right_current.min(next_left_maximum),
            );
            set_attribute_value(
                &mut self.progress.attributes,
                right,
                left_current.min(next_right_maximum),
            );
            swapped_attributes.push(attribute_kind_dto(left));
            swapped_attributes.push(attribute_kind_dto(right));
            power -= 1;
        }

        self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        let active_after = &self.progress.active_mutation_ids;
        let gained_mutation_ids = active_after
            .difference(&active_before)
            .cloned()
            .collect::<Vec<_>>();
        let lost_mutation_ids = active_before
            .difference(active_after)
            .cloned()
            .collect::<Vec<_>>();
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::PolymorphSelf {
                    effect_index: 0,
                    gained_mutation_ids,
                    lost_mutation_ids,
                    swapped_attributes,
                    hp_before,
                    hp_after: self.player.hp,
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_resist_elements_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::ResistElements {
            duration_dice,
            duration_sides,
            duration_bonus,
        } = ability.effect
        else {
            unreachable!("resist elements executor requires a resist elements effect");
        };
        let rolled_duration = (0..duration_dice).fold(duration_bonus, |total, _| {
            total.saturating_add(
                u32::try_from(self.rng.bounded(u64::from(duration_sides)) + 1)
                    .expect("validated resistance duration roll must fit u32"),
            )
        });
        let mut remaining = self.progress.level / 10;
        let candidates = [
            (5_u16, ActorDamageType::Acid, "rfb.status.resist-acid"),
            (
                4,
                ActorDamageType::Electricity,
                "rfb.status.resist-electricity",
            ),
            (3, ActorDamageType::Fire, "rfb.status.resist-fire"),
            (2, ActorDamageType::Cold, "rfb.status.resist-cold"),
            (1, ActorDamageType::Poison, "rfb.status.resist-poison"),
        ];
        let empty_brands = BTreeSet::new();
        let empty_immunities = BTreeSet::new();
        let mut resolutions = Vec::new();
        for (denominator, damage_type, status_kind_id) in candidates {
            if remaining == 0 || self.rng.bounded(u64::from(denominator)) >= u64::from(remaining) {
                continue;
            }
            remaining -= 1;
            let mut resistances = BTreeMap::new();
            resistances.insert(damage_type, ActorResistanceLevel::Resistant);
            let effect_index = u8::try_from(resolutions.len())
                .expect("elemental resistance effect count must fit u8");
            resolutions.push(apply_ability_status_effect(
                &mut self.player,
                &ability.id,
                effect_index,
                status_kind_id,
                1,
                rolled_duration,
                0,
                0,
                AbilityStatusStackingDefinition::Replace,
                None,
                None,
                &resistances,
                &empty_brands,
                &StatModifiers::default(),
                &EquipmentBonuses::default(),
                &empty_immunities,
                None,
                false,
                100,
                None,
                None,
                &mut self.rng,
            ));
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: resolutions,
            },
            trace: None,
        });
    }
}
