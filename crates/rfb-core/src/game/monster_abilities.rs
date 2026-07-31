// SPDX-License-Identifier: MPL-2.0

use super::*;

impl Game {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_monster_cone_damage_plan(
        &mut self,
        source_index: usize,
        source_entity_id: &str,
        source_kind_id: &str,
        plan: &MonsterAbilityPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> MonsterAbilityPlanResolution {
        let MonsterAbilityTargetPlan::Cone {
            target,
            trace,
            affected_positions,
        } = &plan.target
        else {
            unreachable!("monster cone executor requires a cone target plan")
        };
        let (raw_damage, damage_type, radius) = match &plan.ability.effect {
            AbilityEffectDefinition::ConeDamage {
                damage_dice,
                damage_sides,
                damage_bonus,
                damage_type,
                radius,
            } => (
                self.roll_damage(*damage_dice, *damage_sides)
                    .saturating_add(i32::from(*damage_bonus))
                    .max(0),
                damage_type,
                radius,
            ),
            AbilityEffectDefinition::BreathDamage {
                hp_percent,
                max_damage,
                damage_type,
                radius,
            } => {
                // Breath scales with the caster's current vigor: no damage
                // dice are rolled, and the elemental cap bounds a healthy caster.
                let caster_hp = self
                    .entities
                    .iter()
                    .find(|entity| entity.id == source_entity_id)
                    .map_or(0, |entity| entity.hp)
                    .max(0);
                let scaled = caster_hp
                    .saturating_mul(i32::from(*hp_percent))
                    .div_euclid(100);
                (scaled.min(i32::from(*max_damage)), damage_type, radius)
            }
            _ => unreachable!("monster cone plan must retain a cone or breath effect"),
        };
        let origin = self
            .entities
            .iter()
            .find(|entity| entity.id == source_entity_id)
            .map_or(trace.origin, |entity| entity.position);
        let direction = direction_toward(origin, target.position())
            .expect("validated monster cone retains a direction");
        let lateral_distances = self
            .cone_damage_cells(origin, &trace.traversed, direction, *radius)
            .into_iter()
            .map(|(_, lateral, position)| (position, lateral))
            .collect::<BTreeMap<_, _>>();
        let target_actors =
            self.monster_targets_in_footprint(source_index, target, affected_positions);
        let mut targets = Vec::with_capacity(target_actors.len());
        for affected_target in target_actors {
            let lateral_distance = lateral_distances
                .get(&affected_target.position())
                .copied()
                .unwrap_or(0);
            let prepared = rfb_area_damage(raw_damage, lateral_distance);
            let effect = self.resolve_monster_damage_to_hostile(
                source_entity_id,
                source_kind_id,
                &plan.ability.id,
                0,
                raw_damage,
                prepared,
                DamageType::from(*damage_type),
                &affected_target,
                events,
            );
            changed.insert(affected_target.position());
            targets.push(MonsterAbilityTargetResolutionDto {
                target_entity_id: affected_target.entity_id().to_owned(),
                target_kind_id: affected_target.kind_id().to_owned(),
                target_position: affected_target.position(),
                effects: vec![effect],
            });
        }
        let effects = targets
            .iter()
            .find(|resolution| resolution.target_entity_id == target.entity_id())
            .map(|resolution| resolution.effects.clone())
            .unwrap_or_default();
        self.remove_defeated_player_summons(
            targets
                .iter()
                .map(|target| target.target_entity_id.as_str()),
            changed,
            removed_entities,
        );
        MonsterAbilityPlanResolution {
            target_entity_id: target.entity_id().to_owned(),
            target_kind_id: target.kind_id().to_owned(),
            affected_positions: affected_positions.clone(),
            summon: None,
            effects,
            targets,
            trace: Some(trace.clone()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_monster_beam_damage_plan(
        &mut self,
        source_index: usize,
        source_entity_id: &str,
        source_kind_id: &str,
        plan: &MonsterAbilityPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> MonsterAbilityPlanResolution {
        let MonsterAbilityTargetPlan::Beam {
            target,
            trace,
            affected_positions,
        } = &plan.target
        else {
            unreachable!("monster beam executor requires a beam target plan")
        };
        let AbilityEffectDefinition::BeamDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
        } = &plan.ability.effect
        else {
            unreachable!("monster beam plan must retain a beam effect");
        };
        let raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        let target_actors =
            self.monster_targets_in_footprint(source_index, target, affected_positions);
        let mut targets = Vec::with_capacity(target_actors.len());
        for affected_target in target_actors {
            let effect = self.resolve_monster_damage_to_hostile(
                source_entity_id,
                source_kind_id,
                &plan.ability.id,
                0,
                raw_damage,
                raw_damage,
                DamageType::from(*damage_type),
                &affected_target,
                events,
            );
            changed.insert(affected_target.position());
            targets.push(MonsterAbilityTargetResolutionDto {
                target_entity_id: affected_target.entity_id().to_owned(),
                target_kind_id: affected_target.kind_id().to_owned(),
                target_position: affected_target.position(),
                effects: vec![effect],
            });
        }
        let effects = targets
            .iter()
            .find(|resolution| resolution.target_entity_id == target.entity_id())
            .map(|resolution| resolution.effects.clone())
            .unwrap_or_default();
        self.remove_defeated_player_summons(
            targets
                .iter()
                .map(|target| target.target_entity_id.as_str()),
            changed,
            removed_entities,
        );
        MonsterAbilityPlanResolution {
            target_entity_id: target.entity_id().to_owned(),
            target_kind_id: target.kind_id().to_owned(),
            affected_positions: affected_positions.clone(),
            summon: None,
            effects,
            targets,
            trace: Some(trace.clone()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_monster_area_damage_plan(
        &mut self,
        source_index: usize,
        source_entity_id: &str,
        source_kind_id: &str,
        plan: &MonsterAbilityPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> MonsterAbilityPlanResolution {
        let MonsterAbilityTargetPlan::Area {
            target,
            trace,
            affected_positions,
        } = &plan.target
        else {
            unreachable!("monster area executor requires an area target plan")
        };
        let AbilityEffectDefinition::AreaDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            ..
        } = &plan.ability.effect
        else {
            unreachable!("monster area plan must retain an area effect");
        };
        let raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        let target_actors =
            self.monster_targets_in_footprint(source_index, target, affected_positions);
        let mut targets = Vec::with_capacity(target_actors.len());
        for affected_target in target_actors {
            let position = affected_target.position();
            let distance = target
                .position()
                .x
                .abs_diff(position.x)
                .max(target.position().y.abs_diff(position.y));
            let prepared = rfb_area_damage(raw_damage, distance);
            let effect = self.resolve_monster_damage_to_hostile(
                source_entity_id,
                source_kind_id,
                &plan.ability.id,
                0,
                raw_damage,
                prepared,
                DamageType::from(*damage_type),
                &affected_target,
                events,
            );
            changed.insert(position);
            targets.push(MonsterAbilityTargetResolutionDto {
                target_entity_id: affected_target.entity_id().to_owned(),
                target_kind_id: affected_target.kind_id().to_owned(),
                target_position: position,
                effects: vec![effect],
            });
        }
        let effects = targets
            .iter()
            .find(|resolution| resolution.target_entity_id == target.entity_id())
            .map(|resolution| resolution.effects.clone())
            .unwrap_or_default();
        self.remove_defeated_player_summons(
            targets
                .iter()
                .map(|target| target.target_entity_id.as_str()),
            changed,
            removed_entities,
        );
        MonsterAbilityPlanResolution {
            target_entity_id: target.entity_id().to_owned(),
            target_kind_id: target.kind_id().to_owned(),
            affected_positions: affected_positions.clone(),
            summon: None,
            effects,
            targets,
            trace: Some(trace.clone()),
        }
    }

    pub(super) fn resolve_monster_self_effects(
        &mut self,
        source_index: usize,
        ability: &AbilityDefinition,
    ) -> Vec<AbilityEffectResolutionDto> {
        let effects = ability.effect.ordered_effects();
        let mut resolutions = Vec::with_capacity(effects.len());
        for (index, effect) in effects.iter().enumerate() {
            let effect_index =
                u8::try_from(index).expect("validated monster ability effect index must fit u8");
            let resolution = match effect {
                AbilityEffectDefinition::Heal { amount } => {
                    let amount =
                        i32::try_from(*amount).expect("validated healing amount must fit i32");
                    let actor = &mut self.entities[source_index];
                    let outcome = apply_effect(
                        &mut EffectTarget {
                            hp: &mut actor.hp,
                            max_hp: actor.max_hp,
                            resistances: &actor.resistances,
                            statuses: &mut actor.statuses,
                        },
                        EffectSpec::Heal { amount },
                    );
                    let EffectOutcome::Healed { requested, applied } = outcome else {
                        unreachable!("monster healing must produce a healing outcome");
                    };
                    AbilityEffectResolutionDto::Heal {
                        effect_index,
                        resolution: HealingResolutionDto { requested, applied },
                    }
                }
                AbilityEffectDefinition::ApplyStatus {
                    status_kind_id,
                    intensity,
                    duration_ticks,
                    duration_dice,
                    duration_sides,
                    stacking,
                    resistance_type,
                    power,
                    granted_resistances,
                    granted_brands,
                    granted_modifiers,
                    granted_equipment_bonuses,
                    granted_status_immunities,
                    granted_race_id,
                    grants_wall_passage,
                    incoming_damage_percent,
                } => apply_ability_status_effect(
                    &mut self.entities[source_index],
                    &ability.id,
                    effect_index,
                    status_kind_id,
                    *intensity,
                    *duration_ticks,
                    *duration_dice,
                    *duration_sides,
                    *stacking,
                    *resistance_type,
                    *power,
                    granted_resistances,
                    granted_brands,
                    granted_modifiers,
                    granted_equipment_bonuses,
                    granted_status_immunities,
                    granted_race_id.as_deref(),
                    *grants_wall_passage,
                    *incoming_damage_percent,
                    None,
                    None,
                    &mut self.rng,
                ),
                AbilityEffectDefinition::RemoveStatus { status_kind_id } => {
                    remove_ability_status_effect(
                        &mut self.entities[source_index],
                        effect_index,
                        status_kind_id,
                    )
                }
                _ => unreachable!("validated monster self effects must remain actor effects"),
            };
            resolutions.push(resolution);
        }
        resolutions
    }
}
