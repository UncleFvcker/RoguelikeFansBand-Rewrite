// SPDX-License-Identifier: MPL-2.0

use crate::combat::resolve_armored_damage;
use crate::effect::{
    DamagePacket, STATUS_BLEEDING, STATUS_CONFUSION, STATUS_FEAR, STATUS_STUN, resolve_damage,
};
use crate::error::CoreError;
use crate::event::{BoltReflectionOutcome, DomainEvent, ProjectileTrace};
use crate::game::abilities::AbilityTargetPlan;
use crate::game::ability_scaling::{spell_power_value, spell_powered_ability_value};
use crate::game::damage::{FatalityPolicy, commit_damage_application, plan_damage_application};
use crate::game::hunger;
use crate::game::projectile_geometry::{
    has_disintegration_line_of_effect, projectile_path_between, rfb_area_damage, rfb_distance,
};
use crate::game::status_effects::apply_ability_status_effect;
use crate::game::terrain::TerrainChangeSource;
use crate::game::{Game, TERRAIN_INTERACTION_DIRECTIONS, actor_matches_category};
use crate::resistance::{DamageType, ResistanceLevel, ResistanceProfile};
use rfb_content::{
    AbilityDefinition, AbilityEffectDefinition, AbilitySpellPowerField,
    AbilityStatusStackingDefinition, DraconianStrikeModeDefinition, EquipmentBonuses,
    StatModifiers,
};
use rfb_protocol::{
    AbilityAreaDamageResolutionDto, AbilityBeamDamageResolutionDto, AbilityConeDamageResolutionDto,
    AbilityEffectResolutionDto, AbilityEffectSkipReasonDto, AbilityEffectsResolutionDto,
    AbilityTerrainTransformResolutionDto, AbilityVisibleDamageResolutionDto, HealingResolutionDto,
    Position, VirtueKindDto,
};
use std::collections::{BTreeMap, BTreeSet};

const DEATH_VAMPIRIC_DRAIN_ABILITY_ID: &str = "demo.ability.death-vampiric-drain";

const DEATH_VAMPIRISM_TRUE_ABILITY_ID: &str = "demo.ability.death-vampirism-true";

impl Game {
    pub(in crate::game) fn resolve_player_projectile_damage_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::Damage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
        } = &ability.effect
        else {
            unreachable!("player projectile damage executor requires a damage effect");
        };
        let (trace, target_index) = self.trace_projectile_path(path);
        self.resolve_projectile_terrain_effects(
            &[trace.impact],
            DamageType::from(*damage_type),
            changed,
        );
        if ability.affects_ground_items {
            self.resolve_ground_item_projectile_effects(
                &ability.id,
                &[trace.landing],
                DamageType::from(*damage_type),
                true,
                events,
                changed,
                removed_entities,
            );
        }
        let Some(index) = target_index else {
            events.push(DomainEvent::AbilityLanded {
                ability_id: ability.id.clone(),
                trace,
            });
            return Ok(());
        };
        let raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        let raw_damage = i32::try_from(spell_powered_ability_value(
            ability,
            0,
            AbilitySpellPowerField::FinalDamage,
            u64::try_from(raw_damage).expect("ability damage must be non-negative"),
        ))
        .expect("spell-powered ability damage must fit i32");
        self.resolve_player_projectile_damage_target_with_base(
            &ability.id,
            trace,
            index,
            DamageType::from(*damage_type),
            raw_damage,
            ability.affects_ground_items,
            events,
            changed,
            removed_entities,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_player_projectile_damage_with_base(
        &mut self,
        source_id: &str,
        path: Vec<Position>,
        damage_type: DamageType,
        raw_damage: i32,
        affects_ground_items: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let (trace, target_index) = self.trace_projectile_path(path);
        self.resolve_projectile_terrain_effects(&[trace.impact], damage_type, changed);
        if affects_ground_items {
            self.resolve_ground_item_projectile_effects(
                source_id,
                &[trace.landing],
                damage_type,
                true,
                events,
                changed,
                removed_entities,
            );
        }
        let Some(index) = target_index else {
            events.push(DomainEvent::AbilityLanded {
                ability_id: source_id.to_owned(),
                trace,
            });
            return Ok(());
        };
        self.resolve_player_projectile_damage_target_with_base(
            source_id,
            trace,
            index,
            damage_type,
            raw_damage,
            affects_ground_items,
            events,
            changed,
            removed_entities,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_player_projectile_damage_target_with_base(
        &mut self,
        source_id: &str,
        trace: ProjectileTrace,
        index: usize,
        damage_type: DamageType,
        raw_damage: i32,
        affects_ground_items: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        if self.try_reflect_player_bolt(
            index,
            source_id,
            raw_damage,
            damage_type,
            affects_ground_items,
            events,
            changed,
            removed_entities,
        )? {
            return Ok(());
        }
        self.resolve_ability_damage_to_entity(
            index,
            source_id,
            damage_type,
            raw_damage,
            trace,
            events,
            changed,
            removed_entities,
        )?;
        Ok(())
    }

    pub(super) fn resolve_player_malediction_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::Malediction {
            damage_dice,
            damage_sides,
            damage_bonus,
        } = ability.effect
        else {
            unreachable!("Malediction executor requires a Malediction effect");
        };
        let raw_damage = self
            .roll_damage(damage_dice, damage_sides)
            .saturating_add(i32::from(damage_bonus))
            .max(0);
        let raw_damage = i32::try_from(spell_powered_ability_value(
            ability,
            0,
            AbilitySpellPowerField::FinalDamage,
            u64::try_from(raw_damage).expect("Malediction damage must be non-negative"),
        ))
        .expect("spell-powered Malediction damage must fit i32");
        let (trace, target_index) = self.trace_projectile_path(path.clone());
        self.resolve_projectile_terrain_effects(&[trace.impact], DamageType::HellFire, changed);
        if ability.affects_ground_items {
            self.resolve_ground_item_projectile_effects(
                &ability.id,
                &[trace.landing],
                DamageType::HellFire,
                true,
                events,
                changed,
                removed_entities,
            );
        }
        if let Some(target_index) = target_index {
            self.resolve_ability_damage_to_entity(
                target_index,
                &ability.id,
                DamageType::HellFire,
                raw_damage,
                trace.clone(),
                events,
                changed,
                removed_entities,
            )?;
        } else {
            events.push(DomainEvent::AbilityLanded {
                ability_id: ability.id.clone(),
                trace: trace.clone(),
            });
        }

        let trigger_roll =
            u16::try_from(self.rng.bounded(5) + 1).expect("Malediction trigger roll must fit u16");
        let mut choices = vec![AbilityEffectResolutionDto::RandomChoice {
            effect_index: 0,
            roll: i32::from(trigger_roll),
            branch_index: u16::from(trigger_roll == 1),
            maximum_roll: 5,
        }];
        if trigger_roll != 1 {
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability.id.clone(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: None,
                    target_kind_id: None,
                    effects: choices,
                },
                trace: Some(trace),
            });
            return Ok(());
        }

        let rider_roll = u16::try_from(self.rng.bounded(1_000) + 1)
            .expect("Malediction rider roll must fit u16");
        let branch_index = if rider_roll == 666 {
            0
        } else if rider_roll < 500 {
            1
        } else if rider_roll < 800 {
            2
        } else {
            3
        };
        choices.push(AbilityEffectResolutionDto::RandomChoice {
            effect_index: 0,
            roll: i32::from(rider_roll),
            branch_index,
            maximum_roll: 1_000,
        });
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: choices,
            },
            trace: Some(trace),
        });

        let level = self.progress.level;
        if rider_roll == 666 {
            let mut rider = ability.clone();
            rider.effect = AbilityEffectDefinition::DeathRay {
                power: u32::try_from(spell_powered_ability_value(
                    ability,
                    0,
                    AbilitySpellPowerField::MaledictionDeathRayPower,
                    u64::from(level) * 200,
                ))
                .expect("spell-powered Malediction death ray must fit u32"),
            };
            return self.resolve_player_death_ray_effect(
                &rider,
                path,
                events,
                changed,
                removed_entities,
            );
        }

        let (status_kind_id, duration_ticks, power) = if rider_roll < 500 {
            let duration_sides = level / 2;
            let duration_ticks = if duration_sides == 0 {
                1
            } else {
                u32::try_from(self.rng.bounded(u64::from(duration_sides)) + 1)
                    .expect("Malediction fear duration roll must fit u32")
                    .saturating_mul(3)
                    .saturating_add(1)
            };
            let power = u16::try_from(spell_powered_ability_value(
                ability,
                0,
                AbilitySpellPowerField::MaledictionFearPower,
                u64::from(level),
            ))
            .expect("spell-powered Malediction fear power must fit u16");
            (STATUS_FEAR, duration_ticks, Some(power))
        } else if rider_roll < 800 {
            let power = (level / 2)
                .max(u16::try_from(raw_damage.min(100)).expect("Malediction damage must fit u16"));
            let duration_sides = power / 2;
            let duration_ticks = u32::try_from(self.rng.bounded(u64::from(duration_sides)) + 1)
                .expect("Malediction confusion duration roll must fit u32")
                .saturating_mul(3)
                .saturating_add(1);
            (STATUS_CONFUSION, duration_ticks, Some(power))
        } else {
            (
                STATUS_STUN,
                u32::try_from(raw_damage).expect("Malediction damage must be non-negative"),
                None,
            )
        };
        self.resolve_player_malediction_status_rider(
            &ability.id,
            path,
            status_kind_id,
            duration_ticks,
            power,
            events,
            changed,
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_player_malediction_status_rider(
        &mut self,
        ability_id: &str,
        path: Vec<Position>,
        status_kind_id: &str,
        duration_ticks: u32,
        power: Option<u16>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let (trace, target_index) = self.trace_projectile_path(path);
        let Some(target_index) = target_index else {
            events.push(DomainEvent::AbilityLanded {
                ability_id: ability_id.to_owned(),
                trace: trace.clone(),
            });
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability_id.to_owned(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: None,
                    target_kind_id: None,
                    effects: vec![AbilityEffectResolutionDto::Skipped {
                        effect_index: 0,
                        reason: AbilityEffectSkipReasonDto::NoTarget,
                    }],
                },
                trace: Some(trace),
            });
            return;
        };

        let target_entity_id = self.entities[target_index].id.clone();
        let target_kind_id = self.entities[target_index].kind_id.clone();
        let definition = self
            .actor_runtime_definition(&self.entities[target_index])
            .expect("Malediction target definition must remain available");
        let target_level =
            definition
                .level
                .saturating_add(if definition.tags.iter().any(|tag| tag == "unique") {
                    3
                } else {
                    0
                });
        let immunities = if self.actor_has_status_immunity(target_index, status_kind_id) {
            BTreeSet::from([status_kind_id.to_owned()])
        } else {
            BTreeSet::new()
        };
        let resistances = ResistanceProfile::default();
        self.entities[target_index].alerted = true;
        changed.insert(self.entities[target_index].position);
        let resolution = apply_ability_status_effect(
            &mut self.entities[target_index],
            ability_id,
            0,
            status_kind_id,
            1,
            duration_ticks,
            0,
            0,
            AbilityStatusStackingDefinition::Extend,
            None,
            power,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers::default(),
            &EquipmentBonuses::default(),
            &BTreeSet::new(),
            None,
            false,
            100,
            Some(target_level),
            Some((&resistances, &immunities, None)),
            &mut self.rng,
        );
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability_id.to_owned(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(target_entity_id),
                target_kind_id: Some(target_kind_id),
                effects: vec![resolution],
            },
            trace: Some(trace),
        });
    }

    pub(in crate::game) fn resolve_player_area_damage_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        stop_at_actor: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::AreaDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            radius,
            target_category,
        } = &ability.effect
        else {
            unreachable!("player area damage executor requires an area damage effect");
        };
        let base_raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        let base_raw_damage = i32::try_from(spell_powered_ability_value(
            ability,
            0,
            AbilitySpellPowerField::FinalDamage,
            u64::try_from(base_raw_damage).expect("area damage must be non-negative"),
        ))
        .expect("spell-powered area damage must fit i32");
        self.resolve_player_area_damage_with_base(
            &ability.id,
            path,
            stop_at_actor,
            DamageType::from(*damage_type),
            *radius,
            target_category.as_deref(),
            base_raw_damage,
            ability.affects_ground_items,
            events,
            changed,
            removed_entities,
        )
    }

    pub(super) fn resolve_player_lava_flow_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::LavaFlow {
            damage_dice,
            damage_sides,
            damage_bonus,
            radius,
            target_terrain_id,
        } = &ability.effect
        else {
            unreachable!("lava flow executor requires a lava-flow effect");
        };
        let base_raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        let base_raw_damage = i32::try_from(spell_powered_ability_value(
            ability,
            0,
            AbilitySpellPowerField::FinalDamage,
            u64::try_from(base_raw_damage).expect("lava-flow damage must be non-negative"),
        ))
        .expect("spell-powered lava-flow damage must fit i32");
        self.resolve_player_area_damage_with_base(
            &ability.id,
            Vec::new(),
            false,
            DamageType::Fire,
            *radius,
            None,
            base_raw_damage,
            ability.affects_ground_items,
            events,
            changed,
            removed_entities,
        )?;

        // The original rolls 3 or 4 as the hidden terrain projection's
        // power. Both values create deep lava, but the draw is observable in
        // subsequent RNG and therefore remains part of the cast.
        let _terrain_power = self.rng.bounded(2) + 3;
        let connections = self
            .floor_connections
            .iter()
            .map(|connection| connection.position)
            .collect::<BTreeSet<_>>();
        let transformed_positions = self
            .area_damage_cells(self.player.position, *radius)
            .into_iter()
            .map(|(_, position)| position)
            .filter(|position| !connections.contains(position))
            .filter(|position| {
                let index = self
                    .index(*position)
                    .expect("lava-flow footprint must remain in bounds");
                self.terrain[index] != *target_terrain_id
                    && self
                        .content
                        .terrain(&self.terrain[index])
                        .is_some_and(|terrain| !terrain.tags.iter().any(|tag| tag == "permanent"))
            })
            .collect::<Vec<_>>();
        let source_terrain_ids = transformed_positions
            .iter()
            .filter_map(|position| {
                self.index(*position)
                    .map(|index| self.terrain[index].clone())
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        for position in &transformed_positions {
            self.replace_terrain_from_source(
                *position,
                target_terrain_id,
                TerrainChangeSource::Magic,
                events,
                changed,
            );
        }
        events.push(DomainEvent::AbilityTerrainTransformed {
            ability_id: ability.id.clone(),
            resolution: AbilityTerrainTransformResolutionDto {
                center: self.player.position,
                radius: *radius,
                source_terrain_ids,
                target_terrain_id: target_terrain_id.clone(),
                transformed_positions,
            },
        });
        Ok(())
    }

    pub(super) fn resolve_player_doom_hand_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::DoomHand = ability.effect else {
            unreachable!("doom-hand executor requires a doom-hand effect");
        };
        let (trace, target_index) = self.trace_projectile_path(path);
        let Some(index) = target_index else {
            events.push(DomainEvent::AbilityLanded {
                ability_id: ability.id.clone(),
                trace,
            });
            return Ok(());
        };
        let definition = self
            .actor_runtime_definition(&self.entities[index])
            .expect("doom-hand target definition must remain available")
            .clone();
        let unique = definition.tags.iter().any(|tag| tag == "unique");
        let power = spell_power_value(
            u64::from(self.progress.level.saturating_mul(3)),
            ability.spell_power_bonus,
        )
        .max(1);
        let succeeded = if unique {
            false
        } else {
            let power_roll = self.rng.bounded(power) + 1;
            let resistance_roll = u64::from(definition.level) + self.rng.bounded(20) + 1;
            power_roll >= resistance_roll
        };
        if !succeeded {
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability.id.clone(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: Some(self.entities[index].id.clone()),
                    target_kind_id: Some(definition.id),
                    effects: vec![AbilityEffectResolutionDto::NoOp {
                        effect_index: 0,
                        reason: if unique { "unique" } else { "resisted" }.to_owned(),
                    }],
                },
                trace: Some(trace),
            });
            return Ok(());
        }
        let percent =
            i32::try_from(self.rng.bounded(20) + 41).expect("doom-hand percentage must fit i32");
        let raw_damage = self.entities[index].hp.saturating_mul(percent) / 100;
        self.resolve_ability_damage_to_entity(
            index,
            &ability.id,
            DamageType::Curse,
            raw_damage,
            trace,
            events,
            changed,
            removed_entities,
        )?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::game) fn resolve_player_area_damage_with_base(
        &mut self,
        source_id: &str,
        path: Vec<Position>,
        stop_at_actor: bool,
        damage_type: DamageType,
        radius: u8,
        target_category: Option<&str>,
        base_raw_damage: i32,
        affects_ground_items: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        self.resolve_player_area_damage_with_base_policy(
            source_id,
            path,
            stop_at_actor,
            damage_type,
            radius,
            target_category,
            base_raw_damage,
            affects_ground_items,
            true,
            events,
            changed,
            removed_entities,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_player_area_damage_with_base_policy(
        &mut self,
        source_id: &str,
        path: Vec<Position>,
        stop_at_actor: bool,
        damage_type: DamageType,
        radius: u8,
        target_category: Option<&str>,
        base_raw_damage: i32,
        affects_ground_items: bool,
        affects_terrain: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let (trace, _) = self.trace_projectile_path_with_actor_policy(path, stop_at_actor);
        let center = trace.landing;
        let (affected_positions, targets) =
            self.area_damage_targets_for_type(center, radius, target_category, damage_type);
        if affects_terrain {
            self.resolve_projectile_terrain_effects(&affected_positions, damage_type, changed);
        }
        if affects_ground_items {
            self.resolve_ground_item_projectile_effects(
                source_id,
                &affected_positions,
                damage_type,
                true,
                events,
                changed,
                removed_entities,
            );
        }
        changed.extend(affected_positions.iter().copied());
        events.push(DomainEvent::AbilityAreaDamage {
            ability_id: source_id.to_owned(),
            resolution: AbilityAreaDamageResolutionDto {
                center,
                radius,
                base_raw_damage,
                damage_type: damage_type.into(),
                affected_positions,
                target_count: u16::try_from(targets.len()).unwrap_or(u16::MAX),
            },
            trace: trace.clone(),
        });
        for (entity_id, distance) in targets {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            let falloff_damage = rfb_area_damage(base_raw_damage, distance);
            self.resolve_ability_damage_to_entity(
                index,
                source_id,
                damage_type,
                falloff_damage,
                trace.clone(),
                events,
                changed,
                removed_entities,
            )?;
        }
        Ok(())
    }

    pub(in crate::game) fn resolve_player_beam_damage_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::BeamDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            ..
        } = &ability.effect
        else {
            unreachable!("player beam damage executor requires a beam damage effect");
        };
        let base_raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        let base_raw_damage = i32::try_from(spell_powered_ability_value(
            ability,
            0,
            AbilitySpellPowerField::FinalDamage,
            u64::try_from(base_raw_damage).expect("beam damage must be non-negative"),
        ))
        .expect("spell-powered beam damage must fit i32");
        self.resolve_player_beam_damage_with_base(
            &ability.id,
            path,
            DamageType::from(*damage_type),
            base_raw_damage,
            ability.affects_ground_items,
            events,
            changed,
            removed_entities,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::game) fn resolve_player_beam_damage_with_base(
        &mut self,
        source_id: &str,
        path: Vec<Position>,
        damage_type: DamageType,
        base_raw_damage: i32,
        affects_ground_items: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let (trace, _) = self.trace_projectile_path_with_actor_policy(path, false);
        let affected_positions = trace.traversed.clone();
        self.resolve_projectile_terrain_effects(&affected_positions, damage_type, changed);
        self.resolve_projectile_terrain_effects(&[trace.impact], damage_type, changed);
        if affects_ground_items {
            self.resolve_ground_item_projectile_effects(
                source_id,
                &affected_positions,
                damage_type,
                true,
                events,
                changed,
                removed_entities,
            );
        }
        let targets = self.beam_damage_targets(&affected_positions);
        changed.extend(affected_positions.iter().copied());
        events.push(DomainEvent::AbilityBeamDamage {
            ability_id: source_id.to_owned(),
            resolution: AbilityBeamDamageResolutionDto {
                base_raw_damage,
                damage_type: damage_type.into(),
                affected_positions,
                target_count: u16::try_from(targets.len()).unwrap_or(u16::MAX),
            },
            trace: trace.clone(),
        });
        for entity_id in targets {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            self.resolve_ability_damage_to_entity(
                index,
                source_id,
                damage_type,
                base_raw_damage,
                trace.clone(),
                events,
                changed,
                removed_entities,
            )?;
        }
        Ok(())
    }

    pub(in crate::game) fn resolve_player_bolt_or_beam_damage_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::BoltOrBeamDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            beam_chance_percent,
            ..
        } = &ability.effect
        else {
            unreachable!("bolt-or-beam executor requires a bolt-or-beam damage effect");
        };
        let damage_type = DamageType::from(*damage_type);
        let beam = self.rng.bounded(100) < u64::from(*beam_chance_percent);
        let base_raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        let base_raw_damage = i32::try_from(spell_powered_ability_value(
            ability,
            0,
            AbilitySpellPowerField::FinalDamage,
            u64::try_from(base_raw_damage).expect("bolt or beam damage must be non-negative"),
        ))
        .expect("spell-powered bolt or beam damage must fit i32");
        if beam {
            let (trace, _) = self.trace_projectile_path_with_actor_policy(path, false);
            let affected_positions = trace.traversed.clone();
            self.resolve_projectile_terrain_effects(&affected_positions, damage_type, changed);
            self.resolve_projectile_terrain_effects(&[trace.impact], damage_type, changed);
            if ability.affects_ground_items {
                self.resolve_ground_item_projectile_effects(
                    &ability.id,
                    &affected_positions,
                    damage_type,
                    true,
                    events,
                    changed,
                    removed_entities,
                );
            }
            let targets = self.beam_damage_targets(&affected_positions);
            changed.extend(affected_positions.iter().copied());
            events.push(DomainEvent::AbilityBeamDamage {
                ability_id: ability.id.clone(),
                resolution: AbilityBeamDamageResolutionDto {
                    base_raw_damage,
                    damage_type: damage_type.into(),
                    affected_positions,
                    target_count: u16::try_from(targets.len()).unwrap_or(u16::MAX),
                },
                trace: trace.clone(),
            });
            for entity_id in targets {
                let Some(index) = self
                    .entities
                    .iter()
                    .position(|entity| entity.id == entity_id && entity.hp > 0)
                else {
                    continue;
                };
                self.resolve_ability_damage_to_entity(
                    index,
                    &ability.id,
                    damage_type,
                    base_raw_damage,
                    trace.clone(),
                    events,
                    changed,
                    removed_entities,
                )?;
            }
        } else {
            let (trace, target_index) = self.trace_projectile_path_with_actor_policy(path, true);
            self.resolve_projectile_terrain_effects(&[trace.impact], damage_type, changed);
            let Some(index) = target_index else {
                events.push(DomainEvent::AbilityLanded {
                    ability_id: ability.id.clone(),
                    trace,
                });
                return Ok(());
            };
            if self.try_reflect_player_bolt(
                index,
                &ability.id,
                base_raw_damage,
                damage_type,
                false,
                events,
                changed,
                removed_entities,
            )? {
                return Ok(());
            }
            self.resolve_ability_damage_to_entity(
                index,
                &ability.id,
                damage_type,
                base_raw_damage,
                trace,
                events,
                changed,
                removed_entities,
            )?;
        }
        Ok(())
    }

    pub(super) fn resolve_player_stardust_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::Stardust {
            damage_dice,
            damage_sides,
            count,
            deviation,
        } = ability.effect
        else {
            unreachable!("stardust executor requires a stardust effect");
        };
        let Some(aim) = path.last().copied() else {
            unreachable!("validated stardust target path must not be empty");
        };
        let spread = u32::from(deviation).max(1);
        for _ in 0..count {
            let destination = loop {
                let width = u64::from(spread.saturating_mul(2).saturating_add(1));
                let x = aim.x
                    + i32::try_from(self.rng.bounded(width)).expect("spread draw must fit i32")
                    - i32::try_from(spread).expect("validated spread must fit i32");
                let y = aim.y
                    + i32::try_from(self.rng.bounded(width)).expect("spread draw must fit i32")
                    - i32::try_from(spread).expect("validated spread must fit i32");
                let candidate = Position { x, y };
                if candidate != self.player.position && rfb_distance(aim, candidate) <= spread {
                    break candidate;
                }
            };
            let path = self
                .untargeted_projectile_path(destination, ability.target.range)
                .unwrap_or_else(|| path.clone());
            let raw_damage = self.roll_damage(damage_dice, damage_sides).max(0);
            let impact = self.trace_projectile_path(path.clone()).0.impact;
            if let Some(index) = self.index(impact) {
                self.glow[index] = true;
                changed.insert(impact);
            }
            self.resolve_player_projectile_damage_with_base(
                &ability.id,
                path,
                DamageType::Light,
                raw_damage,
                false,
                events,
                changed,
                removed_entities,
            )?;
        }
        Ok(())
    }

    pub(super) fn resolve_player_bolt_or_area_damage_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        stop_at_actor: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::BoltOrAreaDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            area_from_level,
            radius,
            ..
        } = ability.effect
        else {
            unreachable!("bolt-or-area executor requires a matching effect");
        };
        let mut resolved = ability.clone();
        if self.progress.level < area_from_level {
            resolved.effect = AbilityEffectDefinition::Damage {
                damage_dice,
                damage_sides,
                damage_bonus,
                damage_type,
            };
            self.resolve_player_projectile_damage_effect(
                &resolved,
                path,
                events,
                changed,
                removed_entities,
            )
        } else {
            resolved.effect = AbilityEffectDefinition::AreaDamage {
                damage_dice,
                damage_sides,
                damage_bonus,
                damage_type,
                radius,
                target_category: None,
            };
            self.resolve_player_area_damage_effect(
                &resolved,
                path,
                stop_at_actor,
                events,
                changed,
                removed_entities,
            )
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::game) fn try_reflect_player_bolt(
        &mut self,
        reflector_index: usize,
        source_kind_id: &str,
        raw_damage: i32,
        damage_type: DamageType,
        affects_ground_items: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<bool, CoreError> {
        let reflector_kind_id = self.entities[reflector_index].kind_id.clone();
        if !self
            .actor_runtime_definition(&self.entities[reflector_index])
            .is_some_and(|definition| definition.reflects_bolts)
            || self.rng.bounded(4) == 0
        {
            return Ok(false);
        }

        let origin = self.entities[reflector_index].position;
        let path = self.reflected_bolt_path(origin, self.player.position);
        let can_hit_player = self.rng.bounded(2) != 0;
        self.resolve_reflected_bolt(
            reflector_kind_id,
            source_kind_id,
            raw_damage,
            damage_type,
            affects_ground_items,
            origin,
            path,
            can_hit_player,
            events,
            changed,
            removed_entities,
        )?;
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::game) fn try_reflect_monster_bolt(
        &mut self,
        source_kind_id: &str,
        source_position: Position,
        raw_damage: i32,
        damage_type: DamageType,
        affects_ground_items: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<bool, CoreError> {
        if !self.player_reflects_bolts() || self.rng.bounded(4) == 0 {
            return Ok(false);
        }

        let origin = self.player.position;
        let path = self.reflected_bolt_path(origin, source_position);
        self.resolve_reflected_bolt(
            self.player.kind_id.clone(),
            source_kind_id,
            raw_damage,
            damage_type,
            affects_ground_items,
            origin,
            path,
            false,
            events,
            changed,
            removed_entities,
        )?;
        Ok(true)
    }

    fn reflected_bolt_path(&mut self, origin: Position, source: Position) -> Vec<Position> {
        let range = self.width.max(self.height);
        let mut reflected_path = None;
        for _ in 0..10 {
            let y =
                source.y + i32::try_from(self.rng.bounded(5)).expect("bounded draw fits i32") - 2;
            let x =
                source.x + i32::try_from(self.rng.bounded(5)).expect("bounded draw fits i32") - 2;
            let destination = Position { x, y };
            let Some(path) = projectile_path_between(origin, destination, range) else {
                continue;
            };
            if path
                .iter()
                .all(|position| self.index(*position).is_some() && self.is_walkable(*position))
            {
                reflected_path = Some(path);
                break;
            }
        }
        reflected_path
            .or_else(|| projectile_path_between(origin, source, range))
            .expect("an incoming bolt must retain a reverse reflection path")
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_reflected_bolt(
        &mut self,
        reflector_kind_id: String,
        source_kind_id: &str,
        raw_damage: i32,
        damage_type: DamageType,
        affects_ground_items: bool,
        origin: Position,
        path: Vec<Position>,
        can_hit_player: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let mut impact = origin;
        let mut landing = origin;
        let mut traversed = Vec::new();
        let mut hit_player = false;
        let mut hit_actor_index = None;
        for position in path {
            impact = position;
            if self.index(position).is_none() || !self.is_walkable(position) {
                break;
            }
            landing = position;
            traversed.push(position);
            if can_hit_player && position == self.player.position {
                hit_player = true;
                break;
            }
            if let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.hp > 0 && entity.position == position)
            {
                hit_actor_index = Some(index);
                break;
            }
        }
        let trace = ProjectileTrace {
            origin,
            impact,
            landing,
            traversed,
        };
        self.resolve_projectile_terrain_effects(&[trace.impact], damage_type, changed);
        if affects_ground_items {
            self.resolve_ground_item_projectile_effects(
                source_kind_id,
                &[trace.landing],
                damage_type,
                true,
                events,
                changed,
                removed_entities,
            );
        }

        if hit_player {
            if damage_type == DamageType::Rock {
                self.resolve_reflected_rock_player_rider(&reflector_kind_id, raw_damage, events);
            }
            let target = self.player_derived_stats();
            let resistance = self.effective_player_resistances().level(damage_type);
            let damage = self.reduce_player_damage(resolve_armored_damage(
                raw_damage,
                damage_type,
                target.armor_class.value,
                resistance,
            ));
            let application = self.apply_final_player_damage(damage, FatalityPolicy::BelowZero);
            let damage = application.damage;
            events.push(DomainEvent::BoltReflected {
                reflector_kind_id: reflector_kind_id.clone(),
                source_kind_id: source_kind_id.to_owned(),
                outcome: BoltReflectionOutcome::Hit {
                    target_kind_id: self.player.kind_id.clone(),
                    damage,
                    fatal: application.fatal,
                },
                trace,
            });
            if application.fatal {
                events.push(DomainEvent::PlayerDied {
                    source_kind_id: reflector_kind_id,
                    method_id: Some(source_kind_id.to_owned()),
                    damage,
                });
            }
            return Ok(());
        }

        if let Some(index) = hit_actor_index {
            let definition = self
                .content
                .actor(&self.entities[index].kind_id)
                .expect("reflected bolt target definition must remain available")
                .clone();
            let target_kind_id = definition.id.clone();
            let target = self.actor_derived_stats(&self.entities[index], &definition, false);
            let resistance = self.entities[index].resistances.level(damage_type);
            let damage = resolve_armored_damage(
                raw_damage,
                damage_type,
                target.armor_class.value,
                resistance,
            );
            let application = plan_damage_application(
                &self.entities[index],
                damage,
                FatalityPolicy::AtOrBelowZero,
            );
            commit_damage_application(&mut self.entities[index], &application);
            self.entities[index].alerted = true;
            changed.insert(self.entities[index].position);
            self.wake_entity_after_damage(index, damage.applied, events);
            events.push(DomainEvent::BoltReflected {
                reflector_kind_id,
                source_kind_id: source_kind_id.to_owned(),
                outcome: BoltReflectionOutcome::Hit {
                    target_kind_id,
                    damage,
                    fatal: application.fatal,
                },
                trace,
            });
            if !application.fatal && damage_type == DamageType::Rock {
                self.resolve_ability_damage_rider(
                    index,
                    source_kind_id,
                    damage_type,
                    raw_damage,
                    resistance,
                    changed,
                );
            } else if application.fatal {
                self.resolve_actor_death_without_rewards(
                    index,
                    None,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            return Ok(());
        }

        events.push(DomainEvent::BoltReflected {
            reflector_kind_id,
            source_kind_id: source_kind_id.to_owned(),
            outcome: BoltReflectionOutcome::Landed,
            trace,
        });
        Ok(())
    }

    fn resolve_reflected_rock_player_rider(
        &mut self,
        source_kind_id: &str,
        raw_damage: i32,
        events: &mut Vec<DomainEvent>,
    ) {
        let resistances = self.effective_player_resistances();
        if self.rng.bounded(2) == 0 {
            if matches!(
                resistances.level(DamageType::Shards),
                ResistanceLevel::Vulnerable | ResistanceLevel::Normal
            ) {
                self.apply_player_melee_status(
                    STATUS_BLEEDING,
                    raw_damage.max(0) / 2,
                    source_kind_id,
                );
            }
            self.damage_player_inventory(
                source_kind_id,
                DamageType::Shards,
                false,
                raw_damage,
                events,
            );
        } else {
            if matches!(
                resistances.level(DamageType::Sound),
                ResistanceLevel::Vulnerable | ResistanceLevel::Normal
            ) {
                let maximum = if raw_damage > 90 {
                    35
                } else {
                    raw_damage.max(0) / 3 + 5
                };
                let duration = i32::try_from(self.rng.bounded(maximum as u64) + 1)
                    .expect("rock stun roll must fit i32");
                self.apply_player_melee_status(STATUS_STUN, duration, source_kind_id);
            }
            self.damage_player_inventory(
                source_kind_id,
                DamageType::Sound,
                false,
                raw_damage,
                events,
            );
        }
    }

    pub(in crate::game) fn resolve_player_cone_damage_effect(
        &mut self,
        ability: &AbilityDefinition,
        target_plan: AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::ConeDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            radius,
        } = &ability.effect
        else {
            unreachable!("player cone damage executor requires a cone damage effect");
        };
        let AbilityTargetPlan::Cone {
            path,
            direction,
            radius: planned_radius,
        } = target_plan
        else {
            unreachable!("player cone damage executor requires a cone target plan");
        };
        debug_assert_eq!(*radius, planned_radius);
        let damage_type = DamageType::from(*damage_type);
        let (trace, _) =
            self.trace_projectile_path_with_damage_policy(path, false, Some(damage_type));
        let (affected_positions, targets) =
            self.cone_damage_targets(&trace.traversed, direction, *radius, damage_type);
        self.resolve_projectile_terrain_effects(&affected_positions, damage_type, changed);
        if ability.affects_ground_items {
            self.resolve_ground_item_projectile_effects(
                &ability.id,
                &affected_positions,
                damage_type,
                true,
                events,
                changed,
                removed_entities,
            );
        }
        changed.extend(affected_positions.iter().copied());
        let base_raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        let base_raw_damage = i32::try_from(spell_powered_ability_value(
            ability,
            0,
            AbilitySpellPowerField::FinalDamage,
            u64::try_from(base_raw_damage).expect("cone damage must be non-negative"),
        ))
        .expect("spell-powered cone damage must fit i32");
        events.push(DomainEvent::AbilityConeDamage {
            ability_id: ability.id.clone(),
            resolution: AbilityConeDamageResolutionDto {
                radius: *radius,
                base_raw_damage,
                damage_type: damage_type.into(),
                affected_positions,
                target_count: u16::try_from(targets.len()).unwrap_or(u16::MAX),
            },
            trace: trace.clone(),
        });
        for (entity_id, lateral_distance) in targets {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            let falloff_damage = rfb_area_damage(base_raw_damage, lateral_distance);
            self.resolve_ability_damage_to_entity(
                index,
                &ability.id,
                damage_type,
                falloff_damage,
                trace.clone(),
                events,
                changed,
                removed_entities,
            )?;
        }
        Ok(())
    }

    pub(in crate::game) fn resolve_player_visible_damage_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::VisibleDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            target_category,
            unlife_change_on_hit,
        } = &ability.effect
        else {
            unreachable!("visible damage executor requires a visible damage effect");
        };
        let base_raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        let base_raw_damage = i32::try_from(spell_powered_ability_value(
            ability,
            0,
            AbilitySpellPowerField::FinalDamage,
            u64::try_from(base_raw_damage).expect("visible damage must be non-negative"),
        ))
        .expect("spell-powered visible damage must fit i32");
        let affected = self.resolve_player_visible_damage_with_base(
            &ability.id,
            DamageType::from(*damage_type),
            target_category.as_deref(),
            base_raw_damage,
            events,
            changed,
            removed_entities,
        )?;
        if affected > 0 && *unlife_change_on_hit != 0 {
            self.add_virtue(VirtueKindDto::Unlife, i16::from(*unlife_change_on_hit));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_player_visible_damage_with_base(
        &mut self,
        source_id: &str,
        damage_type: DamageType,
        target_category: Option<&str>,
        base_raw_damage: i32,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<usize, CoreError> {
        let target_ids = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && self.entity_is_visible_to_player(entity)
                    && target_category.is_none_or(|category| {
                        self.content
                            .actor(&entity.kind_id)
                            .is_some_and(|definition| actor_matches_category(definition, category))
                    })
            })
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        let target_count = target_ids.len();
        let affected_positions = target_ids
            .iter()
            .filter_map(|id| self.entities.iter().find(|entity| &entity.id == id))
            .map(|entity| entity.position)
            .collect::<Vec<_>>();
        events.push(DomainEvent::AbilityVisibleDamage {
            ability_id: source_id.to_owned(),
            resolution: AbilityVisibleDamageResolutionDto {
                base_raw_damage,
                damage_type: damage_type.into(),
                affected_positions,
                target_count: u16::try_from(target_ids.len()).unwrap_or(u16::MAX),
            },
        });
        let trace = ProjectileTrace {
            origin: self.player.position,
            impact: self.player.position,
            landing: self.player.position,
            traversed: Vec::new(),
        };
        for entity_id in target_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            self.resolve_ability_damage_to_entity(
                index,
                source_id,
                damage_type,
                base_raw_damage,
                trace.clone(),
                events,
                changed,
                removed_entities,
            )?;
        }
        Ok(target_count)
    }

    pub(in crate::game) fn resolve_player_death_ray_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::DeathRay { power } = ability.effect else {
            unreachable!("death ray executor requires a death ray effect");
        };
        let (trace, target_index) = self.trace_projectile_path(path);
        let Some(target_index) = target_index else {
            events.push(DomainEvent::AbilityLanded {
                ability_id: ability.id.clone(),
                trace,
            });
            return Ok(());
        };
        let target_entity_id = self.entities[target_index].id.clone();
        let target_kind_id = self.entities[target_index].kind_id.clone();
        let definition = self
            .content
            .actor(&target_kind_id)
            .expect("death ray target definition must remain available")
            .clone();
        let living = actor_matches_category(&definition, "living");
        let unique = definition.tags.iter().any(|tag| tag == "unique");
        let unique_roll = if living && unique {
            Some(
                u16::try_from(self.rng.bounded(888) + 1)
                    .expect("death ray unique roll must fit u16"),
            )
        } else {
            None
        };
        let unique_resisted = unique_roll.is_some_and(|roll| roll != 666);
        let (target_level_roll, caster_level_roll) = if living && !unique_resisted {
            (
                Some(
                    u16::try_from(self.rng.bounded(20) + 1)
                        .expect("death ray target roll must fit u16"),
                ),
                Some(
                    u32::try_from(self.rng.bounded(u64::from(power.max(1))) + 1)
                        .expect("validated death ray caster roll must fit u32"),
                ),
            )
        } else {
            (None, None)
        };
        let resisted = !living
            || unique_resisted
            || target_level_roll.zip(caster_level_roll).is_some_and(
                |(target_roll, caster_roll)| {
                    definition.level.saturating_add(u32::from(target_roll)) > caster_roll
                },
            );
        let damage = if resisted {
            None
        } else {
            let raw_damage = i32::from(self.progress.level).saturating_mul(200);
            let damage = resolve_damage(
                DamagePacket::new(raw_damage, DamageType::Curse),
                ResistanceLevel::Normal,
            );
            self.entities[target_index].alerted = true;
            let application = plan_damage_application(
                &self.entities[target_index],
                damage,
                FatalityPolicy::AtOrBelowZero,
            );
            commit_damage_application(&mut self.entities[target_index], &application);
            changed.insert(application.position);
            events.push(DomainEvent::AbilityHit {
                ability_id: ability.id.clone(),
                target_kind_id: target_kind_id.clone(),
                damage,
                trace: trace.clone(),
            });
            self.wake_entity_after_damage(target_index, damage.applied, events);
            if !application.fatal {
                self.resolve_monster_fear_aura(target_index, "hurt", true, events);
            }
            if application.fatal {
                self.resolve_actor_death(
                    target_index,
                    DomainEvent::AbilitySlew {
                        ability_id: ability.id.clone(),
                        target_kind_id: target_kind_id.clone(),
                        damage,
                        trace: trace.clone(),
                    },
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            Some(damage.into())
        };
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(target_entity_id),
                target_kind_id: Some(target_kind_id),
                effects: vec![AbilityEffectResolutionDto::DeathRay {
                    effect_index: 0,
                    power,
                    target_level: definition.level,
                    living,
                    unique,
                    unique_roll,
                    target_level_roll,
                    caster_level_roll,
                    resisted,
                    resolution: damage,
                }],
            },
            trace: Some(trace),
        });
        Ok(())
    }

    pub(in crate::game) fn resolve_player_drain_life_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::DrainLife {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            target_category,
            repeat,
            feeds,
        } = &ability.effect
        else {
            unreachable!("drain life executor requires a drain life effect");
        };
        if ability.id == DEATH_VAMPIRISM_TRUE_ABILITY_ID {
            self.add_virtue(VirtueKindDto::Sacrifice, -1);
            self.add_virtue(VirtueKindDto::Vitality, -1);
        }
        for _ in 0..*repeat {
            let (trace, target_index) = self.trace_projectile_path(path.clone());
            let Some(target_index) = target_index else {
                events.push(DomainEvent::AbilityLanded {
                    ability_id: ability.id.clone(),
                    trace: trace.clone(),
                });
                events.push(DomainEvent::AbilityEffectsResolved {
                    ability_id: ability.id.clone(),
                    resolution: AbilityEffectsResolutionDto {
                        target_entity_id: None,
                        target_kind_id: None,
                        effects: vec![AbilityEffectResolutionDto::Skipped {
                            effect_index: 0,
                            reason: AbilityEffectSkipReasonDto::NoTarget,
                        }],
                    },
                    trace: Some(trace),
                });
                continue;
            };
            let target_entity_id = self.entities[target_index].id.clone();
            let target_kind_id = self.entities[target_index].kind_id.clone();
            let eligible = self
                .content
                .actor(&target_kind_id)
                .is_some_and(|definition| actor_matches_category(definition, target_category));
            if !eligible {
                events.push(DomainEvent::AbilityEffectsResolved {
                    ability_id: ability.id.clone(),
                    resolution: AbilityEffectsResolutionDto {
                        target_entity_id: Some(target_entity_id),
                        target_kind_id: Some(target_kind_id),
                        effects: vec![AbilityEffectResolutionDto::Skipped {
                            effect_index: 0,
                            reason: AbilityEffectSkipReasonDto::Ineligible,
                        }],
                    },
                    trace: Some(trace),
                });
                continue;
            }
            let hp_before = self.entities[target_index].hp.max(0);
            let raw_damage = self
                .roll_damage(*damage_dice, *damage_sides)
                .saturating_add(i32::from(*damage_bonus))
                .max(0);
            let raw_damage = i32::try_from(spell_powered_ability_value(
                ability,
                0,
                AbilitySpellPowerField::FinalDamage,
                u64::try_from(raw_damage).expect("drain life damage must be non-negative"),
            ))
            .expect("spell-powered drain life damage must fit i32");
            let damage = self.resolve_ability_damage_to_entity(
                target_index,
                &ability.id,
                DamageType::from(*damage_type),
                raw_damage,
                trace.clone(),
                events,
                changed,
                removed_entities,
            )?;
            if ability.id == DEATH_VAMPIRIC_DRAIN_ABILITY_ID && damage.applied > 0 {
                self.add_virtue(VirtueKindDto::Sacrifice, -1);
                self.add_virtue(VirtueKindDto::Vitality, -1);
            }
            let requested = if !*feeds || self.nutrition < hunger::NUTRITION_FULL {
                damage.applied.min(hp_before)
            } else {
                0
            };
            let outcome = self.apply_player_healing(requested);
            let requested = outcome.requested;
            let applied = outcome.applied;
            if *feeds && damage.applied > 0 {
                let nutrition = u16::try_from(raw_damage.saturating_mul(100).min(5_000))
                    .expect("bounded vampiric nutrition must fit u16");
                if self.nutrition < rfb_protocol::PLAYER_NUTRITION_MAXIMUM {
                    let before = self.nutrition;
                    self.nutrition = self
                        .nutrition
                        .saturating_add(nutrition)
                        .min(rfb_protocol::PLAYER_NUTRITION_MAXIMUM - 1);
                    if self.nutrition > before {
                        self.fasting = false;
                    }
                }
            }
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability.id.clone(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: Some(target_entity_id),
                    target_kind_id: Some(target_kind_id),
                    effects: vec![AbilityEffectResolutionDto::DrainLife {
                        effect_index: 0,
                        resolution: damage.into(),
                        healing: HealingResolutionDto { requested, applied },
                    }],
                },
                trace: Some(trace),
            });
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_player_wrath_of_god_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        stop_at_actor: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::WrathOfGod = ability.effect else {
            unreachable!("Wrath of the God executor requires its dedicated effect");
        };
        let (trace, _) = self.trace_projectile_path_with_actor_policy(path, stop_at_actor);
        let target = trace.landing;
        let raw_damage = self
            .progress
            .level
            .saturating_mul(3)
            .saturating_add(25)
            .saturating_add(self.casting_spell_damage_bonus());
        let damage = i32::try_from(spell_power_value(
            u64::from(raw_damage),
            ability.spell_power_bonus,
        ))
        .expect("Wrath of the God damage must fit i32");
        let count =
            u8::try_from(self.rng.bounded(10) + 11).expect("Wrath of the God count must fit u8");
        for _ in 0..count {
            let mut center = None;
            for _ in 0..20 {
                let x = target.x.saturating_add(
                    i32::try_from(self.rng.bounded(11)).expect("bounded offset must fit i32") - 5,
                );
                let y = target.y.saturating_add(
                    i32::try_from(self.rng.bounded(11)).expect("bounded offset must fit i32") - 5,
                );
                let candidate = Position { x, y };
                if rfb_distance(target, candidate) < 5 {
                    center = Some(candidate);
                    break;
                }
            }
            let Some(center) = center else {
                continue;
            };
            let Some(index) = self.index(center) else {
                continue;
            };
            let permanent = self
                .content
                .terrain(&self.terrain[index])
                .is_some_and(|terrain| terrain.tags.iter().any(|tag| tag == "permanent"));
            if permanent || !has_disintegration_line_of_effect(self, target, center) {
                continue;
            }
            self.resolve_player_area_damage_with_base(
                &ability.id,
                vec![center],
                false,
                DamageType::Disintegrate,
                2,
                None,
                damage,
                ability.affects_ground_items,
                events,
                changed,
                removed_entities,
            )?;
        }
        Ok(())
    }

    pub(super) fn resolve_player_hellfire_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        stop_at_actor: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::Hellfire {
            damage_bonus,
            radius,
            backlash_dice,
            backlash_sides,
            backlash_bonus,
        } = ability.effect
        else {
            unreachable!("hellfire executor requires a hellfire effect");
        };
        self.resolve_player_area_damage_with_base(
            &ability.id,
            path,
            stop_at_actor,
            DamageType::HellFire,
            radius,
            None,
            i32::from(damage_bonus),
            ability.affects_ground_items,
            events,
            changed,
            removed_entities,
        )?;
        let backlash = self
            .roll_damage(backlash_dice, backlash_sides)
            .saturating_add(i32::from(backlash_bonus));
        let damage = resolve_damage(
            DamagePacket::new(backlash, DamageType::Physical),
            ResistanceLevel::Normal,
        );
        let application = self.apply_final_player_damage(damage, FatalityPolicy::BelowZero);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::SelfDamage {
                    effect_index: 0,
                    damage: application.damage.applied,
                    fatal: self.player_is_dead(),
                }],
            },
            trace: None,
        });
        Ok(())
    }

    pub(super) fn resolve_player_melee_adjacent_effect(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let target_ids = TERRAIN_INTERACTION_DIRECTIONS
            .iter()
            .filter_map(|direction| {
                let position = self.position_in_direction(*direction);
                self.entities
                    .iter()
                    .find(|entity| {
                        entity.hp > 0
                            && entity.position == position
                            && !self.actor_is_player_side(entity)
                    })
                    .map(|entity| entity.id.clone())
            })
            .collect::<Vec<_>>();
        for target_id in target_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == target_id && entity.hp > 0)
            else {
                continue;
            };
            self.resolve_player_melee(index, false, events, changed, removed_entities)?;
            if self.player_is_dead() {
                break;
            }
        }
        Ok(())
    }

    pub(super) fn resolve_player_draconian_strike_effect(
        &mut self,
        mode: DraconianStrikeModeDefinition,
        target_entity_id: &str,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let Some(index) = self
            .entities
            .iter()
            .position(|entity| entity.id == target_entity_id && entity.hp > 0)
        else {
            return Ok(());
        };
        self.resolve_player_draconian_strike(index, mode, events, changed, removed_entities)?;
        Ok(())
    }
}
