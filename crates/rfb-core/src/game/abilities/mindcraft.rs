// SPDX-License-Identifier: MPL-2.0

use super::AbilityTargetPlan;
use crate::game::ability_scaling::spell_power_value;
use crate::game::status_effects::apply_ability_status_effect;
use crate::game::*;
use rfb_content::{
    AbilityDetectSubjectDefinition, AbilityTargetModeDefinition, ActorResistanceLevel,
    ClassAbilityDefinition,
};
use rfb_protocol::{AbilityEffectSpecDto, ResistanceDto};

const STATUS_HERO: &str = "rfb.status.hero";

pub(in crate::game) fn is_mindcraft_spell(ability: &AbilityDefinition) -> bool {
    ability.tags.iter().any(|tag| tag == "mindcrafter-spell")
}

pub(in crate::game) fn project_mindcraft_effect(
    spec: &mut AbilityEffectSpecDto,
    level: u16,
    power: i32,
) {
    let powered = |value: u32| spell_power_value(u64::from(value), power) as u32;
    match spec {
        AbilityEffectSpecDto::Precognition {
            detect_invisible,
            detect_traps_and_doors,
            detect_objects_and_stairs,
            maps_area,
            illuminates_floor,
            telepathy_minimum_ticks,
            telepathy_maximum_ticks,
        } => {
            *detect_invisible = level >= 15;
            *detect_traps_and_doors = level >= 5;
            *detect_objects_and_stairs = level >= 30;
            *maps_area = level >= 20;
            *illuminates_floor = level >= 45;
            if (25..40).contains(&level) {
                *telepathy_minimum_ticks = level + 1;
                *telepathy_maximum_ticks = level * 2;
            }
        }
        AbilityEffectSpecDto::MindArmor {
            minimum_duration_ticks,
            maximum_duration_ticks,
            resistances,
            ..
        } => {
            *minimum_duration_ticks = powered(u32::from(level) + 1);
            *maximum_duration_ticks = powered(u32::from(level) * 2);
            *resistances = mind_armor_resistances(level)
                .map(|(_, kind, _)| ResistanceDto {
                    damage_type: DamageType::from(kind).into(),
                    level: ResistanceLevel::Resistant.into(),
                })
                .collect();
        }
        AbilityEffectSpecDto::Adrenaline {
            minimum_duration_ticks,
            maximum_duration_ticks,
            healing_if_not_already_hasted_and_heroic,
        } => {
            *minimum_duration_ticks = powered(16);
            *maximum_duration_ticks = powered(15 + u32::from(level) * 3 / 2);
            *healing_if_not_already_hasted_and_heroic = level;
        }
        _ => {}
    }
}

fn mind_armor_resistances(
    level: u16,
) -> impl Iterator<Item = (u16, ActorDamageType, &'static str)> {
    [
        (15, ActorDamageType::Acid, "rfb.status.resist-acid"),
        (20, ActorDamageType::Fire, "rfb.status.resist-fire"),
        (25, ActorDamageType::Cold, "rfb.status.resist-cold"),
        (
            30,
            ActorDamageType::Electricity,
            "rfb.status.resist-electricity",
        ),
        (35, ActorDamageType::Poison, "rfb.status.resist-poison"),
    ]
    .into_iter()
    .filter(move |(minimum, _, _)| level >= *minimum)
}

impl Game {
    /// Desktop UI acceptance: advance through real experience thresholds and refill mana.
    #[doc(hidden)]
    pub fn debug_prepare_mindcrafter_e2e(&mut self, level: u16) {
        self.entities.clear();
        self.items
            .retain(|item| !matches!(item.location, ItemLocation::CarriedBy { .. }));
        let experience = self
            .experience_required_for_level(level)
            .saturating_sub(self.progress.experience);
        self.apply_player_experience(experience, &mut Vec::new());
        for pool in self.resources.values_mut() {
            pool.current = pool.maximum;
        }
    }

    pub(in crate::game) fn mindcraft_cast_is_zero_time_unavailable(
        &self,
        ability_id: &str,
        target: &TargetSelection,
    ) -> bool {
        let Some(ability) = self
            .content
            .ability(ability_id)
            .filter(|ability| is_mindcraft_spell(ability))
        else {
            return false;
        };
        let Some(activation) = self.class_ability_activation(ability_id) else {
            return true;
        };
        if self.progress.level < activation.minimum_level
            || self.player_has_status_kind(STATUS_CONFUSION)
            || self.player_has_status_kind(STATUS_FEAR)
        {
            return true;
        }
        let (_, cost) = self.class_ability_resource_cost(activation);
        if activation
            .resource_id
            .as_deref()
            .and_then(|id| self.resources.get(id))
            .is_none_or(|pool| pool.current < cost)
        {
            return true;
        }
        let mut ability = ability.clone();
        self.apply_mindcraft_variant(&mut ability);
        Self::apply_player_level_scaling(&mut ability, self.progress.level);
        self.ability_target_plan(&ability, target).is_none()
    }

    pub(super) fn resolve_player_domination_effect(
        &mut self,
        ability: &AbilityDefinition,
        target: AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::Domination { power, mass } = ability.effect else {
            unreachable!("domination requires its effect")
        };
        let (ids, trace) = if mass {
            (
                self.entities
                    .iter()
                    .filter(|actor| {
                        actor.hp > 0
                            && visibility::has_line_of_sight(
                                self,
                                self.player.position,
                                actor.position,
                            )
                    })
                    .map(|actor| actor.id.clone())
                    .collect::<Vec<_>>(),
                None,
            )
        } else {
            let AbilityTargetPlan::Projectile {
                path,
                stop_at_actor,
            } = target
            else {
                unreachable!("single domination requires a projectile")
            };
            let (trace, _) = self.trace_projectile_path_with_actor_policy(path, stop_at_actor);
            let index = self
                .entities
                .iter()
                .position(|actor| actor.hp > 0 && actor.position == trace.landing);
            (
                index
                    .map(|index| self.entities[index].id.clone())
                    .into_iter()
                    .collect(),
                Some(trace),
            )
        };
        for id in ids {
            let index = self
                .entities
                .iter()
                .position(|actor| actor.id == id)
                .expect("domination target must still exist");
            let effect = if mass {
                self.resolve_psychic_charm(index, 0, power)
            } else {
                self.resolve_psychic_domination(index, &ability.id, power)
            };
            changed.insert(self.entities[index].position);
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability.id.clone(),
                trace: trace.clone(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: Some(id),
                    target_kind_id: Some(self.entities[index].kind_id.clone()),
                    effects: vec![effect],
                },
            });
        }
    }

    pub(super) fn resolve_mindcraft_failure(
        &mut self,
        ability: &AbilityDefinition,
        failure: u8,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        if self.rng.bounded(100) + 1 >= u64::from(failure / 2) {
            return Ok(());
        }
        let roll = (self.rng.bounded(100) + 1) as u8;
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            trace: None,
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::MindcraftBacklash {
                    effect_index: 0,
                    roll,
                }],
            },
        });
        match roll {
            1..=4 => self.lose_mindcraft_information(changed),
            5..=89 => {
                let (kind, amount) = match roll {
                    5..=14 => (STATUS_HALLUCINATION, 6 + self.rng.bounded(10) as u32),
                    15..=44 => (STATUS_CONFUSION, 1 + self.rng.bounded(8) as u32),
                    _ => (STATUS_STUN, 1 + self.rng.bounded(8) as u32),
                };
                self.apply_player_mental_status(kind, amount as i32, &ability.id);
                changed.insert(self.player.position);
            }
            _ => {
                let level = self.progress.level;
                self.resolve_player_area_damage_with_base(
                    &ability.id,
                    Vec::new(),
                    false,
                    DamageType::Mana,
                    (2 + level / 10) as u8,
                    None,
                    i32::from(level) * 2,
                    true,
                    events,
                    changed,
                    removed,
                )?;
                self.apply_psychic_backlash_damage(
                    &ability.id,
                    &self.player.kind_id.clone(),
                    i32::from(level) * 2,
                    DamageType::Mana,
                    events,
                );
                if let Some(mana) = self.resources.get_mut("demo.resource.mana") {
                    mana.current = mana
                        .current
                        .saturating_sub(u32::from(level * (level / 10).max(1)));
                }
            }
        }
        Ok(())
    }

    pub(in crate::game) fn apply_psychic_backlash_damage(
        &mut self,
        ability_id: &str,
        source_kind_id: &str,
        amount: i32,
        kind: DamageType,
        events: &mut Vec<DomainEvent>,
    ) {
        let damage = crate::effect::resolve_damage(
            crate::effect::DamagePacket::new(amount, kind),
            self.effective_player_resistances().level(kind),
        );
        let damage = crate::game::damage::scale_damage_outcome(
            damage,
            self.player_spell_damage_percent(kind, damage.applied),
        );
        let applied =
            self.apply_final_player_damage(damage, crate::game::damage::FatalityPolicy::BelowZero);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability_id.to_owned(),
            trace: None,
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::Damage {
                    effect_index: 0,
                    resolution: applied.damage.into(),
                }],
            },
        });
        if applied.fatal {
            events.push(DomainEvent::PlayerDied {
                source_kind_id: source_kind_id.to_owned(),
                method_id: Some(ability_id.to_owned()),
                damage: applied.damage,
            });
        }
    }

    pub(in crate::game) fn apply_mindcraft_variant(&self, ability: &mut AbilityDefinition) {
        let level = self.progress.level;
        match ability.id.as_str() {
            "demo.ability.mindcrafter-precognition" => {
                let threshold = [2, 5, 15, 20, 25, 30, 40, 45]
                    .into_iter()
                    .rfind(|value| level >= *value)
                    .unwrap_or(2);
                ability.description_key =
                    format!("ability-demo-mindcrafter-precognition-description-{threshold}");
            }
            "demo.ability.mindcrafter-minor-displacement" if level >= 45 => {
                ability.name_key = "ability-demo-sorcery-dimension-door-name".to_owned();
                ability.description_key =
                    "ability-demo-sorcery-dimension-door-description".to_owned();
                ability.effect = AbilityEffectDefinition::DimensionDoor {
                    range: level / 2 + 10,
                };
                ability.target.modes = vec![AbilityTargetModeDefinition::Position];
                ability.target.range = level / 2 + 10;
                ability.target.requires_line_of_effect = false;
            }
            "demo.ability.mindcrafter-domination" if level >= 30 => {
                ability.effect = AbilityEffectDefinition::Domination {
                    power: level * 3 / 2 + 15,
                    mass: true,
                };
                ability.level_scaling.clear();
                ability.target.modes = vec![AbilityTargetModeDefinition::SelfTarget];
                ability.target.range = 0;
                ability.target.requires_line_of_effect = false;
            }
            "demo.ability.mindcrafter-pulverise" => {
                if let AbilityEffectDefinition::AreaDamage { radius, .. } = &mut ability.effect {
                    *radius = if level > 20 {
                        ((level - 20) / 8 + 1) as u8
                    } else {
                        0
                    };
                }
            }
            "demo.ability.mindcrafter-mind-wave" => {
                if level >= 25 {
                    ability.effect = AbilityEffectDefinition::VisibleDamage {
                        damage_dice: 1,
                        damage_sides: level * ((level - 5) / 10 + 1),
                        damage_bonus: 0,
                        damage_type: ActorDamageType::Psi,
                        target_category: None,
                        unlife_change_on_hit: 0,
                    };
                    ability.level_scaling.clear();
                } else if let AbilityEffectDefinition::AreaDamage { radius, .. } =
                    &mut ability.effect
                {
                    *radius = (2 + level / 10) as u8;
                }
            }
            "demo.ability.mindcrafter-psychometry" if level >= 20 => {
                ability.effect = AbilityEffectDefinition::IdentifyItem {
                    full_identify_power: 0,
                    full_identify_roll_sides: 0,
                };
            }
            _ => {}
        }
    }

    pub(in crate::game) fn class_ability_resource_cost(
        &self,
        activation: &ClassAbilityDefinition,
    ) -> (u32, u32) {
        let extra = match activation.ability_id.as_str() {
            "demo.ability.mindcrafter-precognition" => match self.progress.level {
                45.. => 9,
                30.. => 4,
                25.. => 3,
                20.. => 1,
                _ => 0,
            },
            "demo.ability.mindcrafter-minor-displacement" if self.progress.level >= 45 => 40,
            _ => 0,
        };
        let cost = activation.resource_cost + extra;
        let effective = if cost > 0 && self.player_has_mindcraft_stone() {
            (cost * 3 / 4).max(1)
        } else {
            cost
        };
        (cost, effective)
    }

    pub(in crate::game) fn player_has_mindcraft_stone(&self) -> bool {
        self.player_is_mindcrafter()
            && self.items.iter().any(|item| {
                matches!(item.location, ItemLocation::Equipped { .. })
                    && item.kind_id == "demo.item.stone-of-mind"
            })
    }

    pub(super) fn resolve_player_mindcraft_effect(
        &mut self,
        ability: &AbilityDefinition,
        target: AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let level = self.progress.level;
        let mut part = ability.clone();
        match ability.effect {
            AbilityEffectDefinition::Precognition => {
                if level >= 45 {
                    self.add_virtue(VirtueKindDto::Knowledge, 1);
                    self.add_virtue(VirtueKindDto::Enlightenment, 1);
                    self.reveal_and_light_floor(&ability.id, events, changed);
                }
                let mut detections = vec![(
                    AbilityDetectSubjectDefinition::Actor,
                    "normal-monster",
                    false,
                )];
                if level >= 15 {
                    detections.push((AbilityDetectSubjectDefinition::Actor, "invisible", false));
                }
                if level >= 5 {
                    detections.extend([
                        (AbilityDetectSubjectDefinition::Terrain, "trap", true),
                        (AbilityDetectSubjectDefinition::Terrain, "door", true),
                    ]);
                }
                if (20..45).contains(&level) {
                    detections.push((AbilityDetectSubjectDefinition::Terrain, "map", true));
                }
                if level >= 30 {
                    detections.extend([
                        (AbilityDetectSubjectDefinition::Terrain, "passage", true),
                        (AbilityDetectSubjectDefinition::Item, "item", true),
                        (AbilityDetectSubjectDefinition::Gold, "gold", true),
                    ]);
                }
                for (subject, category, persistent) in detections {
                    part.effect = AbilityEffectDefinition::Detect {
                        subject,
                        category: category.to_owned(),
                        radius: 30,
                        persistent,
                        through_walls: true,
                    };
                    self.resolve_player_detection_effect(&part, events, changed);
                }
                if (25..40).contains(&level) {
                    let duration = u32::from(level) + self.rng.bounded(u64::from(level)) as u32 + 1;
                    self.apply_mindcraft_buff(
                        ability,
                        STATUS_TELEPATHY,
                        duration,
                        &BTreeMap::new(),
                        &StatModifiers::default(),
                        &EquipmentBonuses::default(),
                        &BTreeSet::new(),
                        events,
                    );
                }
            }
            AbilityEffectDefinition::Psychometry => {
                let AbilityTargetPlan::Item { item_id } = target else {
                    unreachable!("psychometry requires an item")
                };
                self.psychometry_item(&item_id);
            }
            AbilityEffectDefinition::MindArmor => {
                let rolled = u32::from(level) + self.rng.bounded(u64::from(level)) as u32 + 1;
                let duration =
                    spell_power_value(u64::from(rolled), ability.spell_power_bonus) as u32;
                self.apply_mindcraft_buff(
                    ability,
                    "rfb.status.stone-skin",
                    duration,
                    &BTreeMap::new(),
                    &StatModifiers {
                        defense: 50,
                        ..Default::default()
                    },
                    &EquipmentBonuses::default(),
                    &BTreeSet::new(),
                    events,
                );
                for (_, kind, status) in mind_armor_resistances(level) {
                    self.apply_mindcraft_buff(
                        ability,
                        status,
                        duration,
                        &BTreeMap::from([(kind, ActorResistanceLevel::Resistant)]),
                        &StatModifiers::default(),
                        &EquipmentBonuses::default(),
                        &BTreeSet::new(),
                        events,
                    );
                }
            }
            AbilityEffectDefinition::Adrenaline => {
                let rolled = 16 + self.rng.bounded(u64::from(level) * 3 / 2) as u32;
                let duration =
                    spell_power_value(u64::from(rolled), ability.spell_power_bonus) as u32;
                let heal = !self.player_has_status_kind(STATUS_HASTE)
                    || !self.player_has_status_kind(STATUS_HERO);
                self.player
                    .statuses
                    .retain(|status| !matches!(status.kind_id.as_str(), STATUS_STUN | STATUS_FEAR));
                self.apply_mindcraft_buff(
                    ability,
                    STATUS_HERO,
                    duration,
                    &BTreeMap::new(),
                    &StatModifiers {
                        max_hp: 10,
                        ..Default::default()
                    },
                    &EquipmentBonuses {
                        melee_skill: 12,
                        ranged_skill: 12,
                        ..Default::default()
                    },
                    &BTreeSet::from([STATUS_FEAR.to_owned()]),
                    events,
                );
                self.apply_mindcraft_buff(
                    ability,
                    STATUS_HASTE,
                    duration,
                    &BTreeMap::new(),
                    &StatModifiers::default(),
                    &EquipmentBonuses::default(),
                    &BTreeSet::new(),
                    events,
                );
                if heal {
                    part.effect = AbilityEffectDefinition::Heal {
                        amount: u32::from(level),
                    };
                    self.resolve_player_healing_effect(&part, events);
                }
            }
            _ => unreachable!("mindcraft executor requires a mindcraft effect"),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_mindcraft_buff(
        &mut self,
        ability: &AbilityDefinition,
        kind: &str,
        duration: u32,
        resistances: &BTreeMap<ActorDamageType, ActorResistanceLevel>,
        modifiers: &StatModifiers,
        bonuses: &EquipmentBonuses,
        immunities: &BTreeSet<String>,
        events: &mut Vec<DomainEvent>,
    ) {
        let effect = apply_ability_status_effect(
            &mut self.player,
            &ability.id,
            0,
            kind,
            1,
            duration,
            0,
            0,
            AbilityStatusStackingDefinition::KeepStrongest,
            None,
            None,
            resistances,
            &BTreeSet::new(),
            modifiers,
            bonuses,
            immunities,
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
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![effect],
            },
            trace: None,
        });
    }
}
