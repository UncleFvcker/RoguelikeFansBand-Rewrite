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
    pub(in crate::game) fn dungeon_blocks_player_ability(&self, ability_id: &str) -> bool {
        self.dungeon_blocks_magic()
            && self.mutation_activation_for_ability(ability_id).is_none()
            && self.race_ability_activation(ability_id).is_none()
            && self.class_ability_activation(ability_id).map_or_else(
                || self.casting_profile().is_some(),
                |activation| activation.blocked_by_dungeon_anti_magic,
            )
    }

    pub(in crate::game) fn dungeon_blocks_vampirism(&self, ability_id: &str) -> bool {
        self.dungeon_blocks_melee()
            && matches!(
                ability_id,
                "rfb.ability.race.vampirism" | "rfb.ability.mutation.vampirism"
            )
    }

    pub(in crate::game) fn ability_state_unavailable_reason(
        &self,
        ability_id: &str,
    ) -> Option<&'static str> {
        if self.dungeon_blocks_player_ability(ability_id) {
            return Some("anti-magic");
        }
        let ability = self.content.ability(ability_id)?;
        if self.player_is_duelist() && ability.tags.iter().any(|tag| tag == "duelist-technique") {
            if let Some(reason) = self.duelist_equipment_error() {
                return Some(reason);
            }
            if self.player_has_anti_magic() {
                return Some("anti-magic");
            }
            if self.player_has_status_kind(STATUS_BERSERK) {
                return Some("berserk");
            }
        }
        if super::mindcraft::is_mindcraft_spell(ability) {
            if self.player_has_anti_magic() {
                return Some("anti-magic");
            }
            if self.player_has_status_kind(STATUS_BERSERK) {
                return Some("berserk");
            }
        }
        match ability.effect {
            AbilityEffectDefinition::BeginFasting if self.fasting => Some("already-fasting"),
            AbilityEffectDefinition::ClearMind if self.pet_upkeep().controlled_pets > 0 => {
                Some("pets-require-attention")
            }
            _ => None,
        }
    }

    pub(in crate::game) fn resolve_player_ability(
        &mut self,
        ability_id: &str,
        target: TargetSelection,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<Option<Position>, CoreError> {
        let ability = self.content.ability(ability_id).cloned();
        if !self.class_power_matches_realm(ability_id) {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "realm-unavailable".to_owned(),
            });
            return Ok(None);
        }
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
            return Ok(None);
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
            return Ok(None);
        }
        let Some(ability) = ability else {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "unknown-ability".to_owned(),
            });
            return Ok(None);
        };
        if ability.tags.iter().any(|tag| tag == "requires-sight")
            && self.player_has_status_kind(STATUS_BLINDNESS)
        {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "blind".to_owned(),
            });
            return Ok(None);
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
        if self.dungeon_blocks_player_ability(ability_id) {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "anti-magic".to_owned(),
            });
            return Ok(None);
        }
        let innate_power = matches!(source, AbilitySourceDto::Mutation | AbilitySourceDto::Race);
        let resource_spills = Self::ability_cost_spills_into_hit_points(source, ability_id);
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
            return Ok(None);
        }
        if source == AbilitySourceDto::Learned && self.player_has_anti_magic() {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "anti-magic".to_owned(),
            });
            return Ok(None);
        }
        if source == AbilitySourceDto::Learned && self.player_has_status_kind(STATUS_BERSERK) {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "berserk".to_owned(),
            });
            return Ok(None);
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
        self.apply_mindcraft_variant(&mut ability);
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
            return Ok(None);
        }
        if matches!(ability.effect, AbilityEffectDefinition::Rodeo)
            && self.riding_actor_id.is_some()
        {
            events.push(DomainEvent::RodeoAlreadyRiding);
            return Ok(None);
        }

        // Validate the target before charging resources/HP or drawing the
        // failure/damage RNG. The command remains a normal scheduled action,
        // but an impossible target cannot consume resources or proficiency.
        // Blocked vampirism reaches its source failure check without needing a target.
        let vampirism_blocked = self.dungeon_blocks_vampirism(ability_id);
        let target_plan = self.ability_target_plan(&ability, &target);
        let unavailable_glyph = (self.player_is_mage() || self.player_is_warrior_mage())
            && ability.effect.ordered_effects().iter().any(|effect| {
                if let AbilityEffectDefinition::CreateCurrentTerrain {
                    source_terrain_ids,
                    target_terrain_id,
                } = effect
                {
                    self.current_terrain_creation_replacement(source_terrain_ids, target_terrain_id)
                        .is_none()
                } else {
                    false
                }
            });
        if (target_plan.is_none() || unavailable_glyph) && !vampirism_blocked {
            events.push(DomainEvent::AbilityTargetUnavailable {
                ability_id: ability.id,
            });
            return Ok(None);
        }

        // Selecting a body slot is a pending transaction, not a cast attempt.
        // In particular, the source's zero-failure power must not draw failure RNG.
        if matches!(ability.effect, AbilityEffectDefinition::MagicEaterAbsorb) {
            let Some(AbilityTargetPlan::Item { item_id }) = target_plan else {
                unreachable!("validated absorption target");
            };
            self.begin_magic_absorption(&item_id, events)?;
            return Ok(None);
        }

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
                let (base, effective) = self.class_ability_resource_cost(activation);
                (base, effective, activation.resource_id.clone())
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
            class_activation.as_ref().map_or(0, |activation| {
                self.class_ability_hit_point_cost(activation)
            })
        } else {
            0
        };
        if !resource_spills
            && resource_cost > 0
            && resource_id
                .as_deref()
                .is_none_or(|id| !self.resources.contains_key(id))
        {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "resource-unavailable".to_owned(),
            });
            return Ok(None);
        }
        let resource_paid = if resource_spills {
            resource_before.min(resource_cost)
        } else {
            resource_cost
        };
        let hp_paid = if resource_spills {
            resource_cost.saturating_sub(resource_paid)
        } else {
            class_hit_point_cost
        };
        let affordable = if resource_spills {
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
            return Ok(None);
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
            return Ok(None);
        }
        let percentile_roll =
            u8::try_from(self.rng.bounded(100)).expect("percentile ability roll must fit u8");
        let succeeded = percentile_roll >= failure_percent;
        // spells.c::do_cmd_power rolls failure before SPELL_CAST. Vampirism's
        // NO_MELEE cancellation then refunds time and cost; a failed power still pays.
        if succeeded && vampirism_blocked {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "anti-melee".to_owned(),
            });
            return Ok(None);
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
        if hp_paid > 0 && source != AbilitySourceDto::Class {
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
        let book_spell =
            source == AbilitySourceDto::Learned && self.player_uses_dual_realm_learning();
        let progress_after = if source != AbilitySourceDto::Learned {
            mutation_progress
        } else {
            self.record_ability_cast(&ability, succeeded)
        };
        let mut resolution = AbilityCastResolutionDto {
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
            if source == AbilitySourceDto::Class {
                resolution.hp_paid = self.pay_class_ability_hit_points(hp_paid);
            }
            events.push(DomainEvent::AbilityCastFailed { resolution });
            if super::mindcraft::is_mindcraft_spell(&ability) {
                self.resolve_mindcraft_failure(
                    &ability,
                    failure_percent,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            if book_spell {
                self.resolve_book_spell_failure(&ability, failure_percent, events, changed);
            }
            return Ok(None);
        }
        let cast_event_index = events.len();
        if source == AbilitySourceDto::Class {
            resolution.hp_paid = 0;
        }
        let mut target_plan =
            target_plan.expect("successful non-cancelled cast retains its target plan");
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

        let practice = book_spell.then(|| (ability.clone(), self.spell_practice_targets()));
        let result = self.resolve_player_ability_effect(
            ability,
            target_plan,
            events,
            changed,
            removed_entities,
        );
        if result.is_ok() && source == AbilitySourceDto::Class && self.duelist_prompt().is_some() {
            events.remove(cast_event_index);
            self.continue_after_duelist_choice(rfb_protocol::DuelistContinuationDto::ClassCast {
                resolution,
                hit_point_cost: hp_paid,
            });
        } else if result.is_ok() && source == AbilitySourceDto::Class {
            let paid = self.pay_class_ability_hit_points(hp_paid);
            if let DomainEvent::AbilityCastSucceeded { resolution } = &mut events[cast_event_index]
            {
                resolution.hp_paid = paid;
            }
        }
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
        if result.is_ok()
            && !direction_pending
            && let Some((ability, targets)) = practice
        {
            self.apply_book_spell_cast_virtues(
                &ability.id,
                resource_cost,
                failure_percent,
                progress_before.cast_count == 0,
            );
            let progress = self.grow_book_spell(&ability, &targets, &events[cast_event_index..]);
            if let DomainEvent::AbilityCastSucceeded { resolution } = &mut events[cast_event_index]
            {
                resolution.proficiency_after = progress.proficiency;
                resolution.proficiency_rank = Self::ability_proficiency_rank(progress.proficiency);
            }
        }
        result
    }

    pub(in crate::game) fn pay_class_ability_hit_points(&mut self, cost: u32) -> u32 {
        // spells.c: CASTER_USE_HP pays after the effect, including vampiric healing.
        // take_hit ignores a player who already died during the effect.
        if cost == 0 || self.player_is_dead() {
            return 0;
        }
        self.player.hp = self
            .player
            .hp
            .saturating_sub(i32::try_from(cost).expect("validated class HP cost fits i32"));
        if self.player.hp == 0 {
            self.add_virtue(VirtueKindDto::Sacrifice, 1);
            self.add_virtue(VirtueKindDto::Chance, 2);
        }
        cost
    }

    pub(in crate::game) fn select_player_random_choice_branch(
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
        let cast_event_index = events.len();
        let practice_targets = self
            .player_uses_dual_realm_learning()
            .then(|| self.spell_practice_targets());
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
        if let Some(targets) = practice_targets {
            self.apply_book_spell_cast_virtues(
                &ability.id,
                resolution.resource_cost,
                resolution.failure_percent,
                resolution.cast_count == 1,
            );
            let progress = self.grow_book_spell(&ability, &targets, &events[cast_event_index..]);
            if let DomainEvent::AbilityCastSucceeded { resolution } = &mut events[cast_event_index]
            {
                resolution.proficiency_after = progress.proficiency;
                resolution.proficiency_rank = Self::ability_proficiency_rank(progress.proficiency);
            }
        }
        Ok(())
    }
}
