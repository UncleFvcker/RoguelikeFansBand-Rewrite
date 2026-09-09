// SPDX-License-Identifier: MPL-2.0

use super::AbilityTargetPlan;
use crate::game::ability_scaling::spell_powered_ability_value;
use crate::game::{
    AbilityCastResolutionDto, AbilityDefinition, AbilityEffectDefinition,
    AbilityEffectResolutionDto, AbilityEffectsResolutionDto, AbilityProgress,
    AbilityRandomTargetDefinition, AbilitySourceDto, AbilitySpellPowerField,
    AbilityTargetModeDefinition, CoreError, Direction, DomainEvent, Game, Position, STATUS_BERSERK,
    STATUS_BLINDNESS, STATUS_CONFUSION, STATUS_FEAR, TargetSelection, VirtueKindDto,
};
use std::collections::BTreeSet;

const DEATH_INVOKE_SPIRITS_ABILITY_ID: &str = "demo.ability.death-invoke-spirits";
const NATURE_WRATH_ABILITY_ID: &str = "demo.ability.nature-natures-wrath";

pub(in crate::game) fn nature_wrath_direction_roll(events: &[DomainEvent]) -> Option<u8> {
    events.iter().rev().find_map(|event| match event {
        DomainEvent::AbilityEffectsResolved {
            ability_id,
            resolution,
            ..
        } if ability_id == NATURE_WRATH_ABILITY_ID => {
            resolution.effects.iter().find_map(|effect| match effect {
                AbilityEffectResolutionDto::RandomChoice { roll, .. } if matches!(*roll, 2 | 6) => {
                    u8::try_from(*roll).ok()
                }
                _ => None,
            })
        }
        _ => None,
    })
}

impl Game {
    pub(in crate::game) fn ability_state_unavailable_reason(
        &self,
        ability_id: &str,
    ) -> Option<&'static str> {
        self.content
            .ability(ability_id)
            .is_some_and(|ability| {
                matches!(ability.effect, AbilityEffectDefinition::BeginFasting) && self.fasting
            })
            .then_some("already-fasting")
    }

    pub(in crate::game) fn resolve_player_ability(
        &mut self,
        ability_id: &str,
        target: TargetSelection,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let ability = self.content.ability(ability_id).cloned();
        if self.player_has_status_kind(STATUS_CONFUSION)
            && ability.as_ref().is_none_or(|ability| {
                !ability
                    .tags
                    .iter()
                    .any(|tag| tag == "usable-while-confused")
            })
        {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "confused".to_owned(),
            });
            return Ok(());
        }
        let mutation_activation = self.mutation_activation_for_ability(ability_id).cloned();
        let race_activation = self.race_ability_activation(ability_id).cloned();
        let class_activation = self.class_ability_activation(ability_id).cloned();
        let casting_profile = self.casting_profile().cloned();
        if mutation_activation.is_none()
            && race_activation.is_none()
            && class_activation.is_none()
            && casting_profile.is_none()
        {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "no-casting-profile".to_owned(),
            });
            return Ok(());
        }
        let Some(ability) = ability else {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "unknown-ability".to_owned(),
            });
            return Ok(());
        };
        if ability.tags.iter().any(|tag| tag == "requires-sight")
            && self.player_has_status_kind(STATUS_BLINDNESS)
        {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "blind".to_owned(),
            });
            return Ok(());
        }
        let source = if mutation_activation.is_some() {
            AbilitySourceDto::Mutation
        } else if race_activation.is_some() {
            AbilitySourceDto::Race
        } else if class_activation.is_some() {
            AbilitySourceDto::Class
        } else if casting_profile.is_some() {
            AbilitySourceDto::Learned
        } else {
            unreachable!("at least one validated ability source must be available")
        };
        let uses_casting_profile_offense = source == AbilitySourceDto::Learned
            || ability
                .tags
                .iter()
                .any(|tag| tag == "uses-casting-profile-offense");
        let innate_power = matches!(source, AbilitySourceDto::Mutation | AbilitySourceDto::Race);
        let innate_activation = match source {
            AbilitySourceDto::Mutation => mutation_activation.as_ref(),
            AbilitySourceDto::Race => race_activation.as_ref(),
            AbilitySourceDto::Class | AbilitySourceDto::Learned => None,
        };
        if source != AbilitySourceDto::Learned
            && self.player_has_status_kind(STATUS_FEAR)
            && !ability.tags.iter().any(|tag| tag == "usable-while-afraid")
        {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "afraid".to_owned(),
            });
            return Ok(());
        }
        if source == AbilitySourceDto::Learned && self.player_has_anti_magic() {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "anti-magic".to_owned(),
            });
            return Ok(());
        }
        if source == AbilitySourceDto::Learned && self.player_has_status_kind(STATUS_BERSERK) {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "berserk".to_owned(),
            });
            return Ok(());
        }
        let mut ability = match source {
            AbilitySourceDto::Learned => self.effective_casting_ability(
                casting_profile
                    .as_ref()
                    .expect("learned ability source requires a casting profile"),
                &ability,
            ),
            AbilitySourceDto::Class | AbilitySourceDto::Mutation | AbilitySourceDto::Race => {
                ability
            }
        };
        Self::apply_player_level_scaling(&mut ability, self.progress.level);
        if uses_casting_profile_offense && let Some(profile) = casting_profile.as_ref() {
            Self::apply_casting_profile_effect_scaling(profile, &mut ability, self.progress.level);
        }
        if (!innate_power || uses_casting_profile_offense)
            && let Some(profile) = casting_profile.as_ref()
        {
            self.apply_casting_profile_damage_bonus(profile, &mut ability, self.progress.level);
        }
        Self::apply_player_spell_power(&mut ability, self.effective_player_spell_power_bonus());
        self.apply_player_status_power_attribute(&mut ability);
        self.apply_player_dynamic_effect(&mut ability);
        let unavailable_reason = match source {
            AbilitySourceDto::Mutation | AbilitySourceDto::Race => {
                let activation =
                    innate_activation.expect("innate ability source requires an activation");
                (self.progress.level < activation.minimum_level).then_some("level-too-low")
            }
            AbilitySourceDto::Class => {
                let activation = class_activation
                    .as_ref()
                    .expect("class ability source requires an activation");
                if self.progress.level < activation.minimum_level {
                    Some("level-too-low")
                } else if self.sniper_concentration < activation.minimum_concentration {
                    Some("concentration-too-low")
                } else {
                    None
                }
            }
            AbilitySourceDto::Learned => {
                let player = Self::player_ability_parameters(&ability);
                let profile = casting_profile
                    .as_ref()
                    .expect("learned ability source requires a casting profile");
                if !self.learned_abilities.contains(ability_id) {
                    Some("not-learned")
                } else if self.progress.level < player.minimum_level {
                    Some("level-too-low")
                } else if !self.profile_supports_ability(profile, ability_id) {
                    Some("ability-not-supported")
                } else if self.ability_book_item_id(profile, ability_id).is_none() {
                    Some("book-unavailable")
                } else if self.ability_cooldown_remaining(&ability) > 0 {
                    Some("cooldown")
                } else {
                    None
                }
            }
        };
        if let Some(reason) =
            unavailable_reason.or_else(|| self.ability_state_unavailable_reason(ability_id))
        {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: reason.to_owned(),
            });
            return Ok(());
        }
        if matches!(ability.effect, AbilityEffectDefinition::Rodeo)
            && self.riding_actor_id.is_some()
        {
            events.push(DomainEvent::RodeoAlreadyRiding);
            return Ok(());
        }

        // Validate the target before charging resources/HP or drawing the
        // failure/damage RNG. The command remains a normal scheduled action,
        // but an impossible target cannot consume resources or proficiency.
        let Some(mut target_plan) = self.ability_target_plan(&ability, &target) else {
            events.push(DomainEvent::AbilityTargetUnavailable {
                ability_id: ability.id,
            });
            return Ok(());
        };

        let mutation_progress = AbilityProgress {
            proficiency: 0,
            proficiency_cap: 0,
            cast_count: 0,
            fail_count: 0,
            cooldown_remaining: 0,
        };
        let progress_before = if source != AbilitySourceDto::Learned {
            mutation_progress
        } else {
            self.ability_progress_value(&ability)
        };
        let cooldown_before = if source != AbilitySourceDto::Learned {
            0
        } else {
            self.ability_cooldown_remaining(&ability)
        };
        let (base_resource_cost, resource_cost, resource_id) = match source {
            AbilitySourceDto::Mutation | AbilitySourceDto::Race => {
                let activation =
                    innate_activation.expect("innate ability source requires an activation");
                let cost = self.innate_power_resource_cost(activation);
                (
                    activation.cost,
                    cost,
                    casting_profile
                        .as_ref()
                        .map(|profile| profile.resource_id.clone()),
                )
            }
            AbilitySourceDto::Class => {
                let activation = class_activation
                    .as_ref()
                    .expect("class ability source requires an activation");
                (
                    activation.resource_cost,
                    activation.resource_cost,
                    activation.resource_id.clone(),
                )
            }
            AbilitySourceDto::Learned => {
                let player = Self::player_ability_parameters(&ability);
                (
                    player.resource_cost,
                    self.ability_effective_resource_cost(&ability, progress_before),
                    Some(player.resource_id.clone()),
                )
            }
        };
        let failure_percent = if self.debug_ability_casts_succeed {
            0
        } else {
            match source {
                AbilitySourceDto::Mutation | AbilitySourceDto::Race => self
                    .innate_power_failure_percent(
                        innate_activation.expect("innate ability source requires an activation"),
                    ),
                AbilitySourceDto::Class => self.class_ability_failure_percent(
                    class_activation
                        .as_ref()
                        .expect("class ability source requires an activation"),
                ),
                AbilitySourceDto::Learned => self.ability_failure_percent(
                    casting_profile
                        .as_ref()
                        .expect("learned ability source requires a casting profile"),
                    &ability,
                ),
            }
        };
        let resource_before = resource_id
            .as_deref()
            .and_then(|id| self.resources.get(id))
            .map_or(0, |pool| pool.current);
        let class_hit_point_cost = if source == AbilitySourceDto::Class {
            class_activation
                .as_ref()
                .map_or(0, |activation| activation.hit_point_cost)
        } else {
            0
        };
        if !innate_power
            && resource_cost > 0
            && resource_id
                .as_deref()
                .is_none_or(|id| !self.resources.contains_key(id))
        {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "resource-unavailable".to_owned(),
            });
            return Ok(());
        }
        let resource_paid = if innate_power {
            resource_before.min(resource_cost)
        } else {
            resource_cost
        };
        let hp_paid = if innate_power {
            resource_cost.saturating_sub(resource_paid)
        } else {
            class_hit_point_cost
        };
        let affordable = if innate_power {
            hp_paid <= u32::try_from(self.player.hp.max(0)).unwrap_or(0)
        } else {
            resource_before >= resource_cost
                && hp_paid <= u32::try_from(self.player.hp.max(0)).unwrap_or(0)
        };
        if !affordable {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "insufficient-resource".to_owned(),
            });
            return Ok(());
        }
        if matches!(
            ability.effect,
            AbilityEffectDefinition::RechargeFromPlayer { .. }
        ) && resource_before <= resource_cost
        {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "insufficient-recharge-resource".to_owned(),
            });
            return Ok(());
        }
        if resource_paid > 0 {
            let id = resource_id
                .as_ref()
                .expect("positive resource payment requires a resource id");
            let pool = self
                .resources
                .get_mut(id)
                .expect("positive resource payment requires an available pool");
            pool.current -= resource_paid;
        }
        if hp_paid > 0 {
            self.player.hp = self.player.hp.saturating_sub(
                i32::try_from(hp_paid).expect("validated innate power cost must fit i32"),
            );
        }
        if !matches!(
            ability.effect,
            AbilityEffectDefinition::Concentrate | AbilityEffectDefinition::SniperShot { .. }
        ) {
            self.sniper_concentration = 0;
        }
        let resource_after = resource_before.saturating_sub(resource_paid);
        let percentile_roll =
            u8::try_from(self.rng.bounded(100)).expect("percentile ability roll must fit u8");
        let succeeded = percentile_roll >= failure_percent;
        let progress_after = if source != AbilitySourceDto::Learned {
            mutation_progress
        } else {
            self.record_ability_cast(&ability, succeeded)
        };
        let resolution = AbilityCastResolutionDto {
            ability_id: ability.id.clone(),
            resource_id,
            base_resource_cost,
            resource_cost,
            resource_before,
            resource_after,
            resource_paid,
            hp_paid,
            failure_percent,
            percentile_roll,
            succeeded,
            proficiency_before: progress_before.proficiency,
            proficiency_after: progress_after.proficiency,
            proficiency_rank: Self::ability_proficiency_rank(progress_after.proficiency),
            cast_count: progress_after.cast_count,
            fail_count: progress_after.fail_count,
            cooldown_before,
            cooldown_after: if source != AbilitySourceDto::Learned {
                0
            } else {
                self.ability_cooldown_remaining(&ability)
            },
        };
        if !succeeded {
            events.push(DomainEvent::AbilityCastFailed { resolution });
            return Ok(());
        }
        events.push(DomainEvent::AbilityCastSucceeded {
            resolution: resolution.clone(),
        });
        let first_success_experience =
            if source == AbilitySourceDto::Learned && progress_before.cast_count == 0 {
                Self::player_ability_parameters(&ability).first_success_experience
            } else {
                0
            };

        let random_branch_index = if matches!(
            &ability.effect,
            AbilityEffectDefinition::RandomChoice { .. }
        ) {
            Some(self.select_player_random_choice_branch(
                &mut ability,
                &target,
                &mut target_plan,
                events,
            ))
        } else {
            None
        };

        let result = self.resolve_player_ability_effect(
            ability,
            target_plan,
            events,
            changed,
            removed_entities,
        );
        if result.is_ok()
            && ability_id == DEATH_INVOKE_SPIRITS_ABILITY_ID
            && random_branch_index == Some(0)
        {
            self.add_virtue(VirtueKindDto::Unlife, 1);
        }
        let direction_pending =
            ability_id == NATURE_WRATH_ABILITY_ID && nature_wrath_direction_roll(events).is_some();
        if result.is_ok() && !direction_pending && first_success_experience > 0 {
            self.apply_player_experience(u64::from(first_success_experience), events);
        }
        result
    }

    fn select_player_random_choice_branch(
        &mut self,
        ability: &mut AbilityDefinition,
        target: &TargetSelection,
        target_plan: &mut AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
    ) -> u16 {
        let AbilityEffectDefinition::RandomChoice {
            roll_sides,
            level_bonus_divisor,
            branches,
        } = ability.effect.clone()
        else {
            unreachable!("random choice selector requires a random choice effect");
        };
        let base_roll = u16::try_from(self.rng.bounded(u64::from(roll_sides)) + 1)
            .expect("random ability roll must fit u16");
        let level_bonus = self
            .progress
            .level
            .checked_div(level_bonus_divisor)
            .unwrap_or(0);
        let mut roll = i32::try_from(spell_powered_ability_value(
            ability,
            0,
            AbilitySpellPowerField::RandomChoiceRoll,
            u64::from(base_roll.saturating_add(level_bonus)),
        ))
        .expect("spell-powered random ability roll must fit i32");
        if ability.id == DEATH_INVOKE_SPIRITS_ABILITY_ID {
            roll = self.adjust_roll_by_chance_virtue(roll);
            if roll < 26 {
                self.add_virtue(VirtueKindDto::Chance, 1);
            }
        }
        let (branch_index, branch) = branches
            .iter()
            .enumerate()
            .find(|(_, branch)| roll <= i32::from(branch.maximum_roll))
            .or_else(|| {
                (ability.id == DEATH_INVOKE_SPIRITS_ABILITY_ID)
                    .then(|| branches.iter().enumerate().next_back())
                    .flatten()
            })
            .expect("validated random ability branches must cover every roll");
        let branch_index =
            u16::try_from(branch_index).expect("validated random branch index must fit u16");
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::RandomChoice {
                    effect_index: 0,
                    roll,
                    branch_index,
                    maximum_roll: branch.maximum_roll,
                }],
            },
            trace: None,
        });
        ability.effect = (*branch.effect).clone();
        match branch.target {
            AbilityRandomTargetDefinition::CastTarget => {
                if !matches!(ability.effect, AbilityEffectDefinition::NoOp { .. }) {
                    *target_plan = self
                        .ability_target_plan(ability, target)
                        .expect("validated random branch must accept the cast target");
                }
            }
            AbilityRandomTargetDefinition::SelfTarget => {
                ability.target.modes = vec![AbilityTargetModeDefinition::SelfTarget];
                ability.target.range = 0;
                ability.target.requires_line_of_effect = false;
                *target_plan = self
                    .ability_target_plan(ability, &TargetSelection::SelfTarget)
                    .expect("validated random branch must accept a self target");
            }
        }
        branch_index
    }

    pub(in crate::game) fn resolve_pending_ability_direction(
        &mut self,
        direction: Direction,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let pending = self
            .pending_ability_direction
            .clone()
            .ok_or(CoreError::AbilityDirectionUnavailable)?;
        let profile = self
            .casting_profile()
            .cloned()
            .ok_or(CoreError::AbilityDirectionUnavailable)?;
        let mut ability = self
            .content
            .ability(&pending.ability_id)
            .cloned()
            .ok_or(CoreError::AbilityDirectionUnavailable)?;
        ability = self.effective_casting_ability(&profile, &ability);
        Self::apply_player_level_scaling(&mut ability, self.progress.level);
        Self::apply_casting_profile_effect_scaling(&profile, &mut ability, self.progress.level);
        self.apply_casting_profile_damage_bonus(&profile, &mut ability, self.progress.level);
        Self::apply_player_spell_power(&mut ability, self.effective_player_spell_power_bonus());

        let resolution = pending.cast_resolution;
        if let Some(resource_id) = resolution.resource_id.as_deref() {
            let current = self
                .resources
                .get(resource_id)
                .ok_or(CoreError::AbilityDirectionUnavailable)?
                .current;
            if current != resolution.resource_before {
                return Err(CoreError::AbilityDirectionUnavailable);
            }
        }
        if !self.ability_progress.contains_key(&ability.id) {
            return Err(CoreError::AbilityDirectionUnavailable);
        }
        self.pending_ability_direction = None;
        if let Some(resource_id) = resolution.resource_id.as_deref() {
            self.resources
                .get_mut(resource_id)
                .expect("validated pending ability resource must remain available")
                .current = resolution.resource_after;
        }
        let progress = self
            .ability_progress
            .get_mut(&ability.id)
            .expect("validated pending ability progress must remain available");
        progress.proficiency = resolution.proficiency_after;
        progress.cast_count = resolution.cast_count;
        progress.fail_count = resolution.fail_count;
        progress.cooldown_remaining = resolution.cooldown_after;
        self.sniper_concentration = 0;
        events.push(DomainEvent::AbilityCastSucceeded {
            resolution: resolution.clone(),
        });
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::RandomChoice {
                    effect_index: 0,
                    roll: i32::from(pending.branch_roll),
                    branch_index: u16::from(pending.branch_roll - 1),
                    maximum_roll: 6,
                }],
            },
            trace: None,
        });
        self.resolve_nature_wrath_branch(
            &ability,
            pending.branch_roll,
            Some(direction),
            events,
            changed,
            removed_entities,
        )?;
        if resolution.cast_count == 1 {
            let experience = Self::player_ability_parameters(&ability).first_success_experience;
            if experience > 0 {
                self.apply_player_experience(u64::from(experience), events);
            }
        }
        Ok(())
    }
}
