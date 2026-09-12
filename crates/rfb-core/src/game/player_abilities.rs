// SPDX-License-Identifier: MPL-2.0

use super::ability_scaling::{
    apply_ability_level_scaling, apply_ability_spell_power, prorated_level_value, spell_power_value,
};
use super::*;

const SPELL_EXP_BEGINNER: u16 = 900;
const SPELL_EXP_SKILLED: u16 = 1200;
pub(in crate::game) const SPELL_EXP_EXPERT: u16 = 1400;
pub(in crate::game) const SPELL_EXP_MASTER: u16 = 1600;
const SPELL_MANA_CONST: u64 = 2400;
const SPELL_MANA_EXPERT: u64 = 1400;

pub(super) fn clear_mind_recovery_amount(level: u16) -> u32 {
    2 + u32::from(level / 30)
}
const RFB_MAGIC_FAILURE_MINIMUM: [u8; 38] = [
    99, 99, 99, 99, 99, 50, 30, 20, 15, 12, 11, 10, 9, 8, 7, 6, 6, 5, 5, 5, 4, 4, 4, 4, 3, 3, 2, 2,
    2, 2, 1, 1, 1, 1, 1, 0, 0, 0,
];
const RFB_MAGIC_STAT_ADJUSTMENT: [u8; 38] = [
    0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 3, 3, 3, 3, 3, 4, 4, 5, 6, 7, 8, 9, 10, 11, 12,
    13, 14, 15, 16, 17, 18, 19, 20,
];
const RFB_MAGIC_MANA: [u8; 38] = [
    0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 5, 6, 7, 8, 9, 10, 11, 11, 12, 12, 13, 14, 15, 16, 17, 18, 19,
    20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30,
];
const RFB_MAGIC_STUDY: [u8; 38] = [
    0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 5, 5, 5, 5, 5,
    5, 5, 5, 5, 6, 6,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::game) struct AbilityProgress {
    pub(in crate::game) proficiency: u16,
    pub(in crate::game) proficiency_cap: u16,
    pub(in crate::game) cast_count: u32,
    pub(in crate::game) fail_count: u32,
    pub(in crate::game) cooldown_remaining: u16,
}

impl AbilityProgress {
    const fn new(initial: u16, cap: u16) -> Self {
        Self {
            proficiency: initial,
            proficiency_cap: cap,
            cast_count: 0,
            fail_count: 0,
            cooldown_remaining: 0,
        }
    }
}

impl Game {
    pub(super) fn item_has_glove_encumbrance(&self, item: &ItemInstance) -> bool {
        if !self
            .casting_profile()
            .and_then(|profile| profile.encumbrance.as_ref())
            .is_some_and(|encumbrance| encumbrance.glove_encumbrance)
        {
            return false;
        }
        let definition = self
            .content
            .item(&item.kind_id)
            .expect("validated item kind");
        if definition.equipment_slot.as_deref() != Some("gloves") {
            return false;
        }
        // obj_flags + shared pval, before knowledge filtering. Flag presence
        // matters: MAGIC_MASTERY exempts even a zero/negative pval glove.
        let mut flags = BTreeSet::new();
        let mut pval = 0;
        if let Some(raw) = &definition.rfb_value {
            flags.extend(raw.flags.iter().map(String::as_str));
            pval = raw.pval;
        }
        if let Some(artifact) = &definition.artifact_generation {
            let base = self
                .content
                .item(&artifact.base_item_kind_id)
                .expect("validated artifact base");
            if let Some(raw) = &base.rfb_value {
                flags.extend(raw.flags.iter().map(String::as_str));
            }
        }
        if definition
            .status_immunities
            .iter()
            .any(|id| id == STATUS_PARALYSIS)
        {
            flags.insert("FREE_ACT");
        }
        for id in &item.affix_ids {
            let affix = self.content.affix(id).expect("validated affix");
            if let Some(ego) = &affix.rfb_ego {
                flags.extend(ego.flags.iter().map(String::as_str));
            }
            if affix
                .status_immunities
                .iter()
                .any(|id| id == STATUS_PARALYSIS)
            {
                flags.insert("FREE_ACT");
            }
        }
        for properties in std::iter::once(&item.intrinsic_properties)
            .chain(item.rolled_affixes.iter().map(|roll| &roll.properties))
        {
            flags.extend(properties.rfb_flags.iter().map(String::as_str));
            if properties
                .status_immunities
                .iter()
                .any(|id| id == STATUS_PARALYSIS)
            {
                flags.insert("FREE_ACT");
            }
            if let Some(raw) = &properties.rfb_pval {
                flags.extend(raw.flags.iter().map(|flag| flag.source_flag()));
                pval = raw.value;
            }
        }
        !(flags.contains("FREE_ACT")
            || flags.contains("MAGIC_MASTERY")
            || flags.contains("DEX") && pval > 0)
    }

    pub(super) fn item_is_icky(&self, item: &ItemInstance, assume_identified: bool) -> bool {
        if self.priest_weapon_is_unblessed_blade(item)
            && (assume_identified
                || self.item_identification(item) == ItemIdentificationDto::Identified)
        {
            return true;
        }
        (assume_identified
            || self.item_identification(item) != ItemIdentificationDto::Unexamined
            || self
                .item_property_knowledge
                .get(&item.id)
                .is_some_and(|knowledge| {
                    matches!(
                        knowledge.feeling,
                        Some(
                            rfb_protocol::ItemFeelingDto::Average
                                | rfb_protocol::ItemFeelingDto::Good
                        )
                    )
                }))
            && self.item_has_glove_encumbrance(item)
    }

    pub(super) fn casting_spell_damage_bonus(&self) -> u16 {
        let level = self.progress.level;
        self.casting_profile()
            .map_or(0, |profile| {
                profile.spell_damage_bonus_base.saturating_add(
                    profile.spell_damage_bonus_per_level.saturating_mul(
                        level / u16::from(profile.spell_damage_bonus_level_divisor),
                    ),
                )
            })
            .saturating_add(self.armor_spell_damage_bonus())
    }

    pub(super) fn player_ability_parameters(
        ability: &AbilityDefinition,
    ) -> &PlayerAbilityDefinition {
        ability
            .player
            .as_ref()
            .expect("validated player ability must have casting parameters")
    }

    pub(super) fn casting_profile(&self) -> Option<&CastingProfileDefinition> {
        self.character_definitions()
            .and_then(|(_, _, class, _)| class.casting_profile.as_ref())
    }

    pub(super) fn active_casting_realm_profiles(&self) -> Vec<&CastingRealmProfileDefinition> {
        let Some((build, _, class, _)) = self.character_definitions() else {
            return Vec::new();
        };
        let Some(profile) = class.casting_profile.as_ref() else {
            return Vec::new();
        };
        [
            build.first_realm_id.as_deref(),
            self.current_second_realm_id(),
        ]
        .into_iter()
        .flatten()
        .filter_map(|realm_id| {
            profile
                .realm_profiles
                .iter()
                .find(|realm| realm.realm_id == realm_id)
        })
        .collect()
    }

    pub(super) fn active_casting_book_ids(&self) -> Vec<&str> {
        self.active_casting_realm_profiles()
            .into_iter()
            .flat_map(|realm| realm.ability_book_ids.iter().map(String::as_str))
            .collect()
    }

    pub(super) fn class_ability_activation(
        &self,
        ability_id: &str,
    ) -> Option<&ClassAbilityDefinition> {
        self.character_definitions().and_then(|(_, _, class, _)| {
            class.abilities.iter().find(|activation| {
                activation.ability_id == ability_id && self.class_power_matches_realm(ability_id)
            })
        })
    }

    pub(super) fn player_is_good_priest(&self) -> bool {
        self.player_is_priest()
            && self
                .character_definitions()
                .is_some_and(|(build, _, _, _)| {
                    matches!(build.first_realm_id.as_deref(), Some("life" | "crusade"))
                })
    }

    pub(super) fn class_power_matches_realm(&self, ability_id: &str) -> bool {
        match ability_id {
            "demo.ability.priest-bless-weapon" => self.player_is_good_priest(),
            "demo.ability.priest-evocation" => {
                self.player_is_priest() && !self.player_is_good_priest()
            }
            _ => true,
        }
    }

    pub(super) fn priest_weapon_is_unblessed_blade(&self, item: &ItemInstance) -> bool {
        super::player_stats::good_priest_weapon_penalty(
            self.player_is_priest(),
            self.player_is_good_priest(),
            self.content
                .item(&item.kind_id)
                .and_then(|kind| kind.rfb_base_kind)
                .map(|base| base.tval),
            self.item_has_weapon_trait(item, rfb_protocol::WeaponTraitDto::Blessed),
        )
    }

    pub(super) fn priest_blade_failure_penalty(&self) -> i32 {
        25 * self
            .equipped_melee_weapons()
            .into_iter()
            .take(2)
            .filter(|item| self.priest_weapon_is_unblessed_blade(item))
            .count() as i32
    }

    pub(super) fn race_ability_activation(
        &self,
        ability_id: &str,
    ) -> Option<&InnatePowerDefinition> {
        self.character_definitions().and_then(|(_, race, _, _)| {
            race.abilities
                .iter()
                .find(|activation| activation.ability_id == ability_id)
        })
    }

    pub(super) fn uses_spell_scrolls(&self) -> bool {
        self.character_definitions()
            .is_some_and(|(_, _, class, _)| class.uses_spell_scrolls)
    }

    pub(super) fn effective_casting_ability(
        &self,
        _profile: &CastingProfileDefinition,
        ability: &AbilityDefinition,
    ) -> AbilityDefinition {
        let mut effective = ability.clone();
        let player = effective
            .player
            .as_mut()
            .expect("validated casting-profile ability must have player parameters");
        if let Some(override_) = self
            .active_casting_realm_profiles()
            .into_iter()
            .flat_map(|realm| realm.ability_overrides.iter())
            .find(|override_| override_.ability_id == ability.id)
        {
            player.minimum_level = override_.minimum_level;
            player.resource_cost = override_.resource_cost;
            player.base_failure_percent = override_.base_failure_percent;
            if let Some(experience) = override_.first_success_experience {
                player.first_success_experience = experience;
            }
            if !override_.level_scaling.is_empty() {
                effective.level_scaling.clone_from(&override_.level_scaling);
            }
        }
        // lawyer_hack applies these adjustments to every book caster.
        if ability.id == "demo.ability.death-vampirism-true" {
            player.resource_cost =
                (player.resource_cost + player.resource_cost.clamp(50, 100)).min(250);
        }
        if ability.id == "demo.ability.life-warding-true"
            && self
                .character_definitions()
                .is_some_and(|(build, _, _, _)| build.first_realm_id.as_deref() != Some("life"))
        {
            player.minimum_level = 99;
        }
        if self.player_uses_dual_realm_learning() {
            player.proficiency.cap = if self.ability_is_secondary_realm(&ability.id) {
                SPELL_EXP_EXPERT
            } else {
                SPELL_EXP_MASTER
            };
        }
        effective
    }

    pub(super) fn ability_is_secondary_realm(&self, ability_id: &str) -> bool {
        self.active_casting_realm_profiles()
            .get(1)
            .is_some_and(|realm| {
                realm.ability_book_ids.iter().any(|id| {
                    self.content
                        .ability_book(id)
                        .is_some_and(|book| book.ability_ids.iter().any(|id| id == ability_id))
                })
            })
    }

    pub(super) fn apply_player_level_scaling(ability: &mut AbilityDefinition, level: u16) {
        let sleeping_dust_visible_at_level = match &ability.effect {
            AbilityEffectDefinition::SleepingDust { visible_at_level } => Some(*visible_at_level),
            _ => None,
        };
        if let Some(visible_at_level) = sleeping_dust_visible_at_level {
            ability.effect = if level >= visible_at_level {
                AbilityEffectDefinition::VisibleApplyStatus {
                    status_kind_id: STATUS_SLEEP.to_owned(),
                    intensity: 1,
                    duration_ticks: 500,
                    duration_dice: 0,
                    duration_sides: 0,
                    stacking: AbilityStatusStackingDefinition::KeepStrongest,
                    resistance_type: None,
                    power: Some(level),
                    target_category: None,
                }
            } else {
                AbilityEffectDefinition::Sanctuary {
                    power: level,
                    radius: 1,
                }
            };
        }
        match &mut ability.effect {
            AbilityEffectDefinition::IdentifyOrMassIdentify {
                mass_at_level,
                upgraded_name_key,
                upgraded_description_key,
                mass,
            } => {
                *mass = level >= *mass_at_level;
                if *mass {
                    ability.name_key.clone_from(upgraded_name_key);
                    ability.description_key.clone_from(upgraded_description_key);
                    ability.target.modes = vec![AbilityTargetModeDefinition::SelfTarget];
                }
            }
            AbilityEffectDefinition::MassSleepOrStasis {
                stasis_at_level,
                sleep_power_multiplier,
                stasis_power_multiplier,
                power_divisor,
                upgraded_name_key,
                upgraded_description_key,
                stasis,
                power,
            } => {
                *stasis = level >= *stasis_at_level;
                if *stasis {
                    ability.name_key.clone_from(upgraded_name_key);
                    ability.description_key.clone_from(upgraded_description_key);
                }
                let multiplier = if *stasis {
                    *stasis_power_multiplier
                } else {
                    *sleep_power_multiplier
                };
                *power = level
                    .saturating_mul(multiplier)
                    .checked_div(*power_divisor)
                    .expect("validated mass sleep divisor must be positive");
            }
            _ => {}
        }
        if let AbilityEffectDefinition::RandomChoice { branches, .. } = &mut ability.effect {
            for branch in branches {
                for scaling in branch.level_scaling.clone() {
                    let effects = match branch.effect.as_mut() {
                        AbilityEffectDefinition::Sequence { effects } => effects.as_mut_slice(),
                        effect => std::slice::from_mut(effect),
                    };
                    let effect = effects
                        .get_mut(usize::from(scaling.effect_index))
                        .expect("validated random branch scaling index must remain available");
                    apply_ability_level_scaling(effect, &scaling, level);
                }
            }
        }
        for scaling in ability.level_scaling.clone() {
            let effect = match &mut ability.effect {
                AbilityEffectDefinition::Sequence { effects } => effects
                    .get_mut(usize::from(scaling.effect_index))
                    .expect("validated level scaling effect index must remain available"),
                effect => {
                    debug_assert_eq!(scaling.effect_index, 0);
                    effect
                }
            };
            apply_ability_level_scaling(effect, &scaling, level);
        }
        fn apply_area_damage_multiplier(effect: &mut AbilityEffectDefinition, level: u16) {
            match effect {
                AbilityEffectDefinition::BoltOrAreaDamage {
                    damage_dice,
                    damage_bonus,
                    area_from_level,
                    area_damage_multiplier,
                    ..
                } if level >= *area_from_level => {
                    *damage_dice = damage_dice.saturating_mul(u16::from(*area_damage_multiplier));
                    *damage_bonus = damage_bonus.saturating_mul(u16::from(*area_damage_multiplier));
                }
                AbilityEffectDefinition::Sequence { effects } => {
                    for effect in effects {
                        apply_area_damage_multiplier(effect, level);
                    }
                }
                AbilityEffectDefinition::RandomChoice { branches, .. } => {
                    for branch in branches {
                        apply_area_damage_multiplier(&mut branch.effect, level);
                    }
                }
                _ => {}
            }
        }
        apply_area_damage_multiplier(&mut ability.effect, level);
        match &ability.effect {
            AbilityEffectDefinition::DimensionDoor { range } => ability.target.range = *range,
            AbilityEffectDefinition::BeamDamage {
                maximum_range: Some(maximum_range),
                ..
            } => ability.target.range = *maximum_range,
            _ => {}
        }
    }

    pub(super) fn apply_player_status_power_attribute(&self, ability: &mut AbilityDefinition) {
        let Some(attribute) = ability.status_power_attribute else {
            return;
        };
        let AbilityEffectDefinition::ApplyStatus {
            power: Some(power), ..
        } = &mut ability.effect
        else {
            unreachable!("validated status-power attribute requires a direct powered status");
        };
        let attribute_index = self
            .effective_player_attributes()
            .index(Self::item_attribute_kind(&attribute));
        *power = u16::try_from(
            i32::from(*power)
                .saturating_add(crate::stats::original_save_adjustment(attribute_index))
                .max(1),
        )
        .expect("adjusted player status power must fit u16");
    }

    pub(super) fn apply_player_spell_power(ability: &mut AbilityDefinition, bonus: i32) {
        ability.spell_power_bonus = bonus;
        for definition in ability.spell_power_fields.clone() {
            if definition.effect_index == 0
                && definition.field == AbilitySpellPowerField::StatusPower
                && let AbilityEffectDefinition::MassSleepOrStasis { power, .. } =
                    &mut ability.effect
            {
                *power = u16::try_from(spell_power_value(u64::from(*power), bonus))
                    .expect("validated mass sleep power must fit u16");
                continue;
            }
            let effect = match &mut ability.effect {
                AbilityEffectDefinition::Sequence { effects } => effects
                    .get_mut(usize::from(definition.effect_index))
                    .expect("validated spell power effect index must remain available"),
                effect => {
                    debug_assert_eq!(definition.effect_index, 0);
                    effect
                }
            };
            apply_ability_spell_power(effect, definition, bonus);
        }
        match &ability.effect {
            AbilityEffectDefinition::DimensionDoor { range } => ability.target.range = *range,
            AbilityEffectDefinition::BeamDamage {
                maximum_range: Some(maximum_range),
                ..
            } => ability.target.range = *maximum_range,
            _ => {}
        }
    }

    pub(super) fn apply_casting_profile_effect_scaling(
        profile: &CastingProfileDefinition,
        ability: &mut AbilityDefinition,
        level: u16,
    ) {
        let AbilityEffectDefinition::BoltOrBeamDamage {
            beam_chance_percent,
            beam_chance_modifier,
            ..
        } = &mut ability.effect
        else {
            return;
        };
        if profile.beam_chance_level_multiplier == 0 {
            return;
        }
        let chance = i32::from(level)
            .saturating_mul(i32::from(profile.beam_chance_level_multiplier))
            .saturating_div(i32::from(profile.beam_chance_level_divisor))
            .saturating_add(i32::from(profile.beam_chance_bonus))
            .saturating_add(i32::from(*beam_chance_modifier))
            .clamp(0, 100);
        *beam_chance_percent =
            u8::try_from(chance).expect("clamped casting beam chance must fit u8");
    }

    pub(super) fn apply_casting_profile_damage_bonus(
        &self,
        profile: &CastingProfileDefinition,
        ability: &mut AbilityDefinition,
        level: u16,
    ) {
        let bonus = profile.spell_damage_bonus_base.saturating_add(
            profile
                .spell_damage_bonus_per_level
                .saturating_mul(level / u16::from(profile.spell_damage_bonus_level_divisor)),
        );
        let bonus = bonus
            .saturating_add(self.armor_spell_damage_bonus())
            .saturating_mul(
                if ability
                    .tags
                    .iter()
                    .any(|tag| tag == "double-spell-damage-bonus")
                {
                    2
                } else {
                    1
                },
            );
        if bonus == 0 {
            return;
        }
        fn apply(effect: &mut AbilityEffectDefinition, bonus: u16) {
            match effect {
                AbilityEffectDefinition::Damage { damage_bonus, .. }
                | AbilityEffectDefinition::Malediction { damage_bonus, .. }
                | AbilityEffectDefinition::AreaDamage { damage_bonus, .. }
                | AbilityEffectDefinition::LavaFlow { damage_bonus, .. }
                | AbilityEffectDefinition::InsanityCircle { damage_bonus, .. }
                | AbilityEffectDefinition::Hellfire { damage_bonus, .. }
                | AbilityEffectDefinition::JumpDamage { damage_bonus, .. }
                | AbilityEffectDefinition::BeamDamage { damage_bonus, .. }
                | AbilityEffectDefinition::BoltOrBeamDamage { damage_bonus, .. }
                | AbilityEffectDefinition::BoltOrAreaDamage { damage_bonus, .. }
                | AbilityEffectDefinition::ConeDamage { damage_bonus, .. }
                | AbilityEffectDefinition::CurseDamage { damage_bonus, .. }
                | AbilityEffectDefinition::VisibleDamage { damage_bonus, .. }
                | AbilityEffectDefinition::DrainLife { damage_bonus, .. } => {
                    *damage_bonus = damage_bonus.saturating_add(bonus);
                }
                AbilityEffectDefinition::Sequence { effects } => {
                    for effect in effects {
                        apply(effect, bonus);
                    }
                }
                AbilityEffectDefinition::RandomChoice { branches, .. } => {
                    for branch in branches {
                        apply(&mut branch.effect, bonus);
                    }
                }
                _ => {}
            }
        }
        apply(&mut ability.effect, bonus);
    }

    fn profile_failure_percent(
        &self,
        profile: &CastingProfileDefinition,
        ability: &AbilityDefinition,
        modifier_percent: i32,
    ) -> u8 {
        let player = Self::player_ability_parameters(ability);
        let attribute_index = self
            .effective_player_attributes()
            .index(Self::casting_attribute_kind(profile.casting_attribute))
            .min(crate::stats::PRE_VICTORY_ATTRIBUTE_INDEX_CAP);
        let level_adjustment =
            i32::from(self.progress.level.saturating_sub(player.minimum_level)).saturating_mul(3);
        let proficiency = self.ability_progress_value(ability).proficiency;
        let proficiency_adjustment =
            i32::from(proficiency >= SPELL_EXP_EXPERT) + i32::from(proficiency >= SPELL_EXP_MASTER);
        let easy_spell = i32::from(
            self.player_equipment_passives()
                .contains(&EquipmentPassive::EasySpell),
        );
        let chance = match profile.failure_formula {
            CastingFailureFormula::Linear => i32::from(player.base_failure_percent)
                .saturating_sub(level_adjustment)
                .saturating_sub(i32::from(attribute_index))
                .saturating_sub(proficiency_adjustment)
                .saturating_add(modifier_percent)
                .clamp(i32::from(profile.minimum_failure_percent), 95),
            CastingFailureFormula::RfbMagic => {
                let attribute_adjustment =
                    (i32::from(RFB_MAGIC_STAT_ADJUSTMENT[usize::from(attribute_index)]) - 1)
                        .saturating_mul(3);
                let minimum_failure_percent = profile
                    .minimum_failure_percent
                    .max(RFB_MAGIC_FAILURE_MINIMUM[usize::from(attribute_index)]);
                let effective_resource_cost = self
                    .ability_effective_resource_cost(ability, self.ability_progress_value(ability));
                let resource_penalty = self
                    .resources
                    .get(&profile.resource_id)
                    .map_or(effective_resource_cost, |pool| {
                        effective_resource_cost.saturating_sub(pool.current)
                    })
                    .saturating_mul(5);
                i32::from(player.base_failure_percent)
                    .saturating_sub(level_adjustment)
                    .saturating_sub(attribute_adjustment)
                    .saturating_add(if self.player_uses_dual_realm_learning() {
                        5 * i32::from(
                            (self.player_is_mage() || self.player_is_priest())
                                && self.ability_is_secondary_realm(&ability.id),
                        ) + self.book_spell_alignment_modifier(&ability.id)
                    } else {
                        0
                    })
                    .saturating_add(modifier_percent)
                    .saturating_add(i32::try_from(resource_penalty).unwrap_or(i32::MAX))
                    .saturating_add(self.priest_blade_failure_penalty())
                    .saturating_sub(4 * easy_spell)
                    .max(i32::from(minimum_failure_percent))
                    .saturating_add(if self.player_uses_dual_realm_learning() {
                        self.player
                            .statuses
                            .iter()
                            .filter(|status| status.kind_id == STATUS_STUN)
                            .map(|status| i32::from(status.intensity).min(100) / 2)
                            .max()
                            .unwrap_or(0)
                    } else {
                        0
                    })
                    .min(95)
                    .saturating_sub(proficiency_adjustment)
                    .saturating_sub(easy_spell)
                    .max(0)
            }
        };
        u8::try_from(chance).expect("validated ability failure chance must fit u8")
    }

    fn casting_attribute_kind(attribute: CastingAttribute) -> AttributeKind {
        match attribute {
            CastingAttribute::Strength => AttributeKind::Strength,
            CastingAttribute::Intelligence => AttributeKind::Intelligence,
            CastingAttribute::Wisdom => AttributeKind::Wisdom,
            CastingAttribute::Dexterity => AttributeKind::Dexterity,
            CastingAttribute::Constitution => AttributeKind::Constitution,
            CastingAttribute::Charisma => AttributeKind::Charisma,
        }
    }

    fn casting_resource_maximum(&self, profile: &CastingProfileDefinition) -> u32 {
        if self.progress.level < profile.first_spell_level {
            return 0;
        }
        let attribute = Self::casting_attribute_kind(profile.casting_attribute);
        let attribute_index = self
            .effective_player_attributes()
            .index(attribute)
            .min(crate::stats::PRE_VICTORY_ATTRIBUTE_INDEX_CAP);
        let mut maximum = match profile.capacity_formula {
            CastingCapacityFormula::Linear => profile_resource_maximum(
                self.progress.level,
                attribute_index,
                (
                    profile.base_capacity,
                    profile.capacity_per_level,
                    profile.capacity_per_attribute_index,
                ),
            ),
            CastingCapacityFormula::RfbMana => {
                let mut value =
                    u32::from(RFB_MAGIC_MANA[usize::from(attribute_index)]).saturating_mul(
                        u32::from(self.progress.level - profile.first_spell_level + 1)
                            .saturating_add(3),
                    ) / 4;
                if value > 0 {
                    value = value.saturating_add(1);
                }
                let racial_adjustment = self
                    .character_definitions()
                    .map_or(0, |(_, race, _, _)| match attribute {
                        AttributeKind::Intelligence => race.modifiers.intelligence,
                        AttributeKind::Wisdom => race.modifiers.wisdom,
                        AttributeKind::Charisma => race.modifiers.charisma,
                        _ => 0,
                    })
                    .clamp(-5, 5);
                if racial_adjustment >= 0 {
                    value =
                        value.saturating_add(value.saturating_mul(racial_adjustment as u32) / 20);
                } else {
                    value = value.saturating_sub(
                        value.saturating_mul(racial_adjustment.unsigned_abs()) / 20,
                    );
                }
                value
            }
        };
        if let Some(encumbrance) = &profile.encumbrance {
            let equipped = self.items.iter().filter_map(|item| {
                let ItemLocation::Equipped { .. } = &item.location else {
                    return None;
                };
                self.content
                    .item(&item.kind_id)
                    .map(|definition| (item, definition))
            });
            let mut weight = 0_u32;
            let mut cumbersome_gloves = false;
            for (instance, item) in equipped {
                match item.equipment_slot.as_deref() {
                    Some("body" | "head" | "shield" | "cloak" | "gloves" | "boots") => {
                        weight =
                            weight.saturating_add(u32::from(self.item_instance_weight(instance)));
                        cumbersome_gloves |= self.item_has_glove_encumbrance(instance);
                    }
                    Some("weapon") => {
                        weight = weight.saturating_add(
                            u32::from(self.item_instance_weight(instance))
                                .saturating_mul(u32::from(encumbrance.weapon_weight_percent))
                                / 100,
                        );
                    }
                    _ => {}
                }
            }
            if encumbrance.glove_encumbrance && cumbersome_gloves {
                maximum = maximum.saturating_mul(3) / 4;
            }
            let excess = weight.saturating_sub(encumbrance.maximum_weight_tenths_pound);
            maximum = maximum.saturating_sub(
                maximum.saturating_mul(excess) / encumbrance.penalty_weight_tenths_pound,
            );
        }
        let racial_capacity_percent = self
            .character_definitions()
            .map_or(0, |(_, race, _, _)| race.spell_capacity_bonus)
            .saturating_mul(5);
        let capacity_percent = i32::from(profile.capacity_percent)
            .saturating_add(racial_capacity_percent)
            .saturating_add(self.player_equipment_bonuses().spell_capacity_bonus * 5)
            .max(0);
        maximum.saturating_mul(u32::try_from(capacity_percent).unwrap_or(u32::MAX)) / 100
    }

    fn mutation_attribute_kind(attribute: TechniqueAttribute) -> AttributeKind {
        match attribute {
            TechniqueAttribute::Strength => AttributeKind::Strength,
            TechniqueAttribute::Intelligence => AttributeKind::Intelligence,
            TechniqueAttribute::Wisdom => AttributeKind::Wisdom,
            TechniqueAttribute::Dexterity => AttributeKind::Dexterity,
            TechniqueAttribute::Constitution => AttributeKind::Constitution,
            TechniqueAttribute::Charisma => AttributeKind::Charisma,
        }
    }

    pub(super) fn ability_cost_spills_into_hit_points(
        source: AbilitySourceDto,
        ability_id: &str,
    ) -> bool {
        matches!(source, AbilitySourceDto::Mutation | AbilitySourceDto::Race)
            || (source == AbilitySourceDto::Class
                && matches!(
                    ability_id,
                    "demo.ability.ranger-probe-monsters"
                        | "demo.ability.priest-bless-weapon"
                        | "demo.ability.priest-evocation"
                ))
    }

    pub(super) fn innate_power_failure_percent(&self, activation: &InnatePowerDefinition) -> u8 {
        if self.progress.level < activation.minimum_level {
            return 100;
        }
        if activation.base_failure_percent == 0 {
            return 0;
        }
        let attribute = Self::mutation_attribute_kind(activation.governing_attribute);
        let index = usize::from(
            self.effective_player_attributes()
                .index(attribute)
                .min(crate::stats::PRE_VICTORY_ATTRIBUTE_INDEX_CAP),
        );
        let chance = i32::from(activation.base_failure_percent)
            .saturating_sub(
                i32::from(self.progress.level.saturating_sub(activation.minimum_level))
                    .saturating_mul(3),
            )
            .saturating_sub((i32::from(RFB_MAGIC_STAT_ADJUSTMENT[index]) - 1).saturating_mul(3))
            .saturating_add(self.player_spell_failure_modifier_percent())
            .max(i32::from(
                activation
                    .minimum_failure_percent
                    .unwrap_or(RFB_MAGIC_FAILURE_MINIMUM[index])
                    .max(RFB_MAGIC_FAILURE_MINIMUM[index]),
            ))
            .min(95);
        u8::try_from(chance.max(self.player_spell_failure_minimum_percent()))
            .expect("bounded mutation failure chance must fit u8")
    }

    pub(super) fn class_ability_failure_percent(&self, activation: &ClassAbilityDefinition) -> u8 {
        if self.progress.level < activation.minimum_level {
            return 100;
        }
        if activation.base_failure_percent == 0 {
            return 0;
        }
        let attribute = Self::mutation_attribute_kind(
            activation
                .governing_attribute
                .expect("validated failable class ability requires an attribute"),
        );
        let index = usize::from(
            self.effective_player_attributes()
                .index(attribute)
                .min(crate::stats::PRE_VICTORY_ATTRIBUTE_INDEX_CAP),
        );
        let chance = i32::from(activation.base_failure_percent)
            .saturating_sub(
                i32::from(self.progress.level.saturating_sub(activation.minimum_level))
                    .saturating_mul(3),
            )
            .saturating_sub((i32::from(RFB_MAGIC_STAT_ADJUSTMENT[index]) - 1).saturating_mul(3))
            .saturating_add(self.player_spell_failure_modifier_percent())
            .saturating_sub(
                if self.player_has_mindcraft_stone()
                    || ((self.player_is_priest() || self.player_is_warrior_mage())
                        && self
                            .player_equipment_passives()
                            .contains(&EquipmentPassive::EasySpell))
                {
                    4
                } else {
                    0
                },
            )
            .max(i32::from(
                activation
                    .minimum_failure_percent
                    .max(RFB_MAGIC_FAILURE_MINIMUM[index]),
            ))
            .saturating_add(
                if self.player_is_mindcrafter()
                    || self.player_is_berserker()
                    || self.player_is_priest()
                    || self.player_is_warrior_mage()
                {
                    self.player
                        .statuses
                        .iter()
                        .filter(|status| status.kind_id == STATUS_STUN)
                        .map(|status| i32::from(status.intensity).min(100) / 2)
                        .max()
                        .unwrap_or(0)
                } else {
                    0
                },
            )
            .min(95)
            .saturating_sub(
                if self.player_has_mindcraft_stone()
                    || ((self.player_is_priest() || self.player_is_warrior_mage())
                        && self
                            .player_equipment_passives()
                            .contains(&EquipmentPassive::EasySpell))
                {
                    1
                } else {
                    0
                },
            );
        u8::try_from(chance.max(self.player_spell_failure_minimum_percent()))
            .expect("bounded class ability failure chance must fit u8")
    }

    pub(super) fn innate_power_resource_cost(&self, activation: &InnatePowerDefinition) -> u32 {
        let extra = activation
            .cost_scaling
            .map_or(0, |scaling| match scaling.curve {
                InnatePowerCostScalingCurveDefinition::Step => {
                    if self.progress.level < scaling.start_level {
                        0
                    } else {
                        u32::from(
                            (self.progress.level - scaling.start_level) / scaling.level_interval
                                + 1,
                        )
                        .saturating_mul(scaling.amount)
                    }
                }
                InnatePowerCostScalingCurveDefinition::Prorated => {
                    let value = prorated_level_value(
                        u64::from(scaling.amount),
                        self.progress.level,
                        scaling.linear_weight,
                        scaling.quadratic_weight,
                        scaling.cubic_weight,
                    );
                    let divisor = u64::from(scaling.divisor);
                    let scaled = if scaling.round_up {
                        value.saturating_add(divisor - 1) / divisor
                    } else {
                        value / divisor
                    };
                    u32::try_from(scaled).unwrap_or(u32::MAX)
                }
            });
        let cost = activation.cost.saturating_add(extra);
        let Some(AbilityEffectDefinition::DraconianBreathDamage {
            enhancing_mutation_id,
            ..
        }) = self
            .content
            .ability(&activation.ability_id)
            .map(|ability| &ability.effect)
        else {
            return cost;
        };
        if self.player_has_mutation(enhancing_mutation_id) {
            cost.max(1)
        } else {
            cost.saturating_mul(2).checked_div(3).unwrap_or(0).max(1)
        }
    }

    pub(super) fn apply_player_dynamic_effect(&self, ability: &mut AbilityDefinition) {
        match &mut ability.effect {
            AbilityEffectDefinition::CraftEnchant {
                maximum,
                level_divisor,
                ..
            } if *level_divisor > 0 => {
                *maximum += self.progress.level / *level_divisor;
            }
            AbilityEffectDefinition::ElementalImmunity { duration_base } => {
                *duration_base =
                    spell_power_value(u64::from(*duration_base), ability.spell_power_bonus) as u32;
            }
            _ => {}
        }
        let AbilityEffectDefinition::DraconianBreathDamage {
            base_hp_percent,
            level_cubic_percent_numerator,
            level_cubic_percent_divisor,
            max_damage,
            damage_type,
            enhancing_mutation_id,
        } = &ability.effect
        else {
            return;
        };
        let level = u64::from(self.progress.level);
        let level_percent = level
            .saturating_mul(level)
            .saturating_mul(level)
            .saturating_mul(u64::from(*level_cubic_percent_numerator))
            / u64::from(*level_cubic_percent_divisor);
        let hp_percent = u64::from(*base_hp_percent).saturating_add(level_percent);
        let current_hp = u64::try_from(self.player.hp.max(0)).unwrap_or(0);
        let capped_damage = current_hp
            .saturating_mul(hp_percent)
            .checked_div(100)
            .unwrap_or(0)
            .min(u64::from(*max_damage));
        let damage = if self.player_has_mutation(enhancing_mutation_id) {
            capped_damage
        } else {
            capped_damage / 2
        }
        .max(1);
        let damage_bonus =
            u16::try_from(damage).expect("validated Draconian breath damage cap must fit u16");
        let damage_type = *damage_type;
        ability.effect = if self.progress.level < 20 {
            AbilityEffectDefinition::Damage {
                damage_dice: 0,
                damage_sides: 0,
                damage_bonus,
                damage_type,
            }
        } else if self.progress.level < 30 {
            AbilityEffectDefinition::BeamDamage {
                damage_dice: 0,
                damage_sides: 0,
                damage_bonus,
                damage_type,
                maximum_range: None,
            }
        } else {
            AbilityEffectDefinition::ConeDamage {
                damage_dice: 0,
                damage_sides: 0,
                damage_bonus,
                damage_type,
                radius: u8::try_from(1 + self.progress.level / 20)
                    .expect("player level-derived Draconian breath radius must fit u8"),
            }
        };
    }

    pub(super) fn study_player_ability(
        &mut self,
        book_item_id: &str,
        ability_id: &str,
    ) -> Result<(), &'static str> {
        let Some(profile) = self.casting_profile().cloned() else {
            return Err("no-casting-profile");
        };
        if profile.study_mode != CastingStudyMode::Chosen {
            return Err("study-mode-mismatch");
        }
        if let Some(reason) = self.ability_study_unavailable_reason() {
            return Err(reason);
        }
        let Some(ability) = self.content.ability(ability_id) else {
            return Err("unknown-ability");
        };
        let ability = self.effective_casting_ability(&profile, ability);
        let studied = self
            .ability_learning_order
            .iter()
            .any(|id| id == ability_id);
        if studied && !(self.player_is_mage() || self.player_is_warrior_mage()) {
            return Err("already-learned");
        }
        if self.progress.level < Self::player_ability_parameters(&ability).minimum_level {
            return Err("level-too-low");
        }
        if !self.profile_supports_ability(&profile, ability_id) {
            return Err("ability-not-supported");
        }
        if self.ability_learning_remaining(&profile) == 0 {
            return Err("learning-capacity-full");
        }
        let Some(book_id) = self.study_book_id(book_item_id) else {
            return Err("book-unavailable");
        };
        if !self.active_casting_book_ids().contains(&book_id)
            || !self
                .content
                .ability_book(book_id)
                .is_some_and(|book| book.ability_ids.iter().any(|id| id == ability_id))
        {
            return Err("book-mismatch");
        }
        if studied {
            if !self.learned_abilities.contains(ability_id) {
                return Err("ability-forgotten");
            }
            let progress = self
                .ability_progress
                .get_mut(ability_id)
                .expect("supported spell has progress");
            let old = progress.proficiency;
            if old >= progress.proficiency_cap {
                return Err("proficiency-at-cap");
            }
            progress.proficiency = if old >= SPELL_EXP_EXPERT {
                SPELL_EXP_MASTER
            } else if old >= SPELL_EXP_SKILLED {
                (old + 200).min(progress.proficiency_cap)
            } else if old >= SPELL_EXP_BEGINNER {
                SPELL_EXP_SKILLED + (old - SPELL_EXP_BEGINNER) * 2 / 3
            } else {
                SPELL_EXP_BEGINNER + old / 3
            };
        } else {
            self.learned_abilities.insert(ability_id.to_owned());
            self.ability_learning_order.push(ability_id.to_owned());
        }
        if self.player_is_mage() || self.player_is_warrior_mage() {
            self.spent_spell_learning += 1;
            // cmd5.c uses the class spell_book (Mage/Warrior-Mage: SORCERY), not the realm.
            self.add_virtue(VirtueKindDto::Knowledge, 1);
        }
        Ok(())
    }

    pub(super) fn study_random_player_ability(
        &mut self,
        book_item_id: &str,
    ) -> Result<String, &'static str> {
        let Some(profile) = self.casting_profile().cloned() else {
            return Err("no-casting-profile");
        };
        if profile.study_mode != CastingStudyMode::DivineRandom {
            return Err("study-mode-mismatch");
        }
        if let Some(reason) = self.ability_study_unavailable_reason() {
            return Err(reason);
        }
        if self.ability_learning_remaining(&profile) == 0 {
            return Err("learning-capacity-full");
        }
        let Some(book_id) = self.study_book_id(book_item_id).map(str::to_owned) else {
            return Err("book-unavailable");
        };
        if !self.active_casting_book_ids().contains(&book_id.as_str()) {
            return Err("book-mismatch");
        }
        let candidates = self
            .content
            .ability_book(&book_id)
            .ok_or("book-mismatch")?
            .ability_ids
            .iter()
            .filter_map(|ability_id| {
                let ability = self.content.ability(ability_id)?;
                let ability = self.effective_casting_ability(&profile, ability);
                (!self.ability_learning_order.contains(ability_id)
                    && self.progress.level
                        >= Self::player_ability_parameters(&ability).minimum_level)
                    .then(|| ability_id.clone())
            })
            .collect::<Vec<_>>();
        let mut gift = None;
        for (index, ability_id) in candidates.into_iter().enumerate() {
            if self.rng.bounded((index + 1) as u64) == 0 {
                gift = Some(ability_id);
            }
        }
        let ability_id = gift.ok_or("no-learnable-abilities")?;
        self.learned_abilities.insert(ability_id.clone());
        self.ability_learning_order.push(ability_id.clone());
        if self.player_uses_dual_realm_learning() {
            self.spent_spell_learning += 1;
            // cmd5.c uses the class spell_book (Ranger/Priest: LIFE), not the realm.
            self.add_virtue(VirtueKindDto::Faith, 1);
        }
        Ok(ability_id)
    }

    pub(super) fn resolve_prayer_study(
        &mut self,
        book_item_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        match self.study_random_player_ability(book_item_id) {
            Ok(ability_id) => {
                events.push(DomainEvent::AbilityStudied { ability_id });
                true
            }
            Err(reason) => {
                let target_id = self
                    .items
                    .iter()
                    .find(|item| item.id == book_item_id)
                    .map_or_else(|| book_item_id.to_owned(), |item| item.kind_id.clone());
                events.push(DomainEvent::AbilityStudyUnavailable {
                    target_id,
                    reason: reason.to_owned(),
                });
                false
            }
        }
    }

    pub(super) fn ability_study_unavailable_reason(&self) -> Option<&'static str> {
        if self.player_has_status_kind(STATUS_BLINDNESS) {
            Some("blind")
        } else if !self.position_is_lit(self.player.position) {
            Some("no-light")
        } else if self.player_has_status_kind(STATUS_CONFUSION) {
            Some("confused")
        } else {
            None
        }
    }

    pub(super) fn study_book_id(&self, book_item_id: &str) -> Option<&str> {
        self.items
            .iter()
            .find(|item| {
                item.id == book_item_id
                    && (item.location == ItemLocation::Inventory
                        || item.location == ItemLocation::Ground(self.player.position))
            })
            .and_then(|item| self.content.item(&item.kind_id))
            .and_then(|item| item.ability_book_id.as_deref())
    }

    pub(super) fn forget_player_ability(&mut self, ability_id: &str) -> Result<(), &'static str> {
        if self.player_uses_dual_realm_learning() {
            return Err("manual-forgetting-unavailable");
        }
        let Some(profile) = self.casting_profile().cloned() else {
            return Err("no-casting-profile");
        };
        if self.content.ability(ability_id).is_none() {
            return Err("unknown-ability");
        }
        if !self.profile_supports_ability(&profile, ability_id) {
            return Err("ability-not-supported");
        }
        if !self.learned_abilities.remove(ability_id) {
            return Err("not-learned");
        }
        self.ability_learning_order.retain(|id| id != ability_id);
        self.refresh_player_spell_memory();
        Ok(())
    }

    fn remembered_player_abilities(&self) -> BTreeSet<String> {
        let Some(profile) = self.casting_profile() else {
            return BTreeSet::new();
        };
        self.ability_learning_order
            .iter()
            .filter(|id| {
                self.content.ability(id).is_some_and(|ability| {
                    let ability = self.effective_casting_ability(profile, ability);
                    Self::player_ability_parameters(&ability).minimum_level <= self.progress.level
                })
            })
            .take(if self.player_uses_dual_realm_learning() {
                (u32::from(self.ability_learning_capacity(profile))
                    + self.ability_learning_order.len() as u32)
                    .saturating_sub(self.spent_spell_learning) as usize
            } else {
                usize::from(self.ability_learning_capacity(profile))
            })
            .cloned()
            .collect()
    }

    fn refresh_player_spell_memory(&mut self) {
        self.learned_abilities = self.remembered_player_abilities();
    }

    pub(super) fn player_spell_memory_is_valid(&self) -> bool {
        let unique = self.ability_learning_order.iter().collect::<BTreeSet<_>>();
        let spending_valid = if self.player_uses_dual_realm_learning() {
            let maximum_studies: u32 = self
                .ability_learning_order
                .iter()
                .filter_map(|id| self.ability_progress.get(id))
                .map(|progress| {
                    1 + u32::from(progress.proficiency >= SPELL_EXP_BEGINNER)
                        + u32::from(progress.proficiency >= SPELL_EXP_SKILLED)
                        + u32::from(progress.proficiency >= SPELL_EXP_EXPERT)
                        + u32::from(progress.proficiency >= SPELL_EXP_MASTER)
                })
                .sum();
            let historical_capacity = if self.player_is_ranger() {
                (3 * u32::from(self.progress.max_level.saturating_sub(2))).min(80)
            } else if self.player_is_priest() {
                (3 * u32::from(self.progress.max_level)).min(96)
            } else if self.player_is_warrior_mage() {
                (3 * u32::from(self.progress.max_level)).min(84)
            } else {
                (3 * u32::from(self.progress.max_level)).min(100)
            } + u32::from(self.bonus_spell_learning_capacity);
            let has_replaced_realm = self
                .mage_realms
                .as_ref()
                .is_some_and(|realms| !realms.previous_realm_ids.is_empty());
            // Replaced realm entries no longer bound cumulative spending. The source
            // counter survives replacement; at most 64 forgotten slots can augment capacity.
            self.spent_spell_learning >= self.ability_learning_order.len() as u32
                && (has_replaced_realm
                    || if self.player_is_mage() || self.player_is_warrior_mage() {
                        self.spent_spell_learning <= maximum_studies
                    } else {
                        self.spent_spell_learning == self.ability_learning_order.len() as u32
                    })
                && self.spent_spell_learning
                    <= historical_capacity
                        + if has_replaced_realm {
                            64
                        } else {
                            self.ability_learning_order.len() as u32
                        }
                && self.ability_progress.iter().all(|(id, progress)| {
                    self.ability_learning_order.contains(id)
                        || (progress.proficiency == 0
                            && progress.cast_count == 0
                            && progress.fail_count == 0)
                })
        } else {
            self.spent_spell_learning == 0
        };
        spending_valid
            && unique.len() == self.ability_learning_order.len()
            && self.ability_learning_order.iter().all(|id| {
                self.casting_profile().is_some_and(|profile| {
                    self.profile_supports_ability(profile, id)
                        && self.content.ability(id).is_some_and(|ability| {
                            let ability = self.effective_casting_ability(profile, ability);
                            Self::player_ability_parameters(&ability).minimum_level
                                <= self.progress.max_level
                        })
                })
            })
            && self.learned_abilities == self.remembered_player_abilities()
    }

    pub(super) fn ability_learning_remaining(&self, profile: &CastingProfileDefinition) -> u16 {
        let capacity = u32::from(self.ability_learning_capacity(profile));
        let remaining = if self.player_uses_dual_realm_learning() {
            let forgotten = self.ability_learning_order.len() - self.learned_abilities.len();
            (capacity + forgotten as u32).saturating_sub(self.spent_spell_learning)
        } else {
            capacity.saturating_sub(self.learned_abilities.len() as u32)
        };
        u16::try_from(remaining).expect("remaining study budget fits its capacity")
    }

    pub(super) fn ability_learning_capacity(&self, profile: &CastingProfileDefinition) -> u16 {
        if profile.realm_profiles.is_empty() {
            return 0;
        }
        let attribute_index = u32::from(
            self.effective_player_attributes()
                .index(Self::casting_attribute_kind(profile.casting_attribute)),
        );
        let realm_bonus = self
            .active_casting_realm_profiles()
            .into_iter()
            .map(|realm| realm.learning_capacity_bonus)
            .max()
            .unwrap_or(0);
        let raw = match profile.learning_formula {
            CastingLearningFormula::Linear => {
                let level_bonus = u32::from(profile.learning_capacity_per_level)
                    .saturating_mul(u32::from(self.progress.level.saturating_sub(1)));
                let attribute_bonus = u32::from(profile.learning_capacity_per_attribute_index)
                    .saturating_mul(attribute_index);
                u32::from(profile.base_learning_capacity)
                    .saturating_add(level_bonus)
                    .saturating_add(attribute_bonus)
            }
            CastingLearningFormula::RfbSingleRealm | CastingLearningFormula::RfbDualRealm => {
                let index = usize::from(
                    self.effective_player_attributes()
                        .index(Self::casting_attribute_kind(profile.casting_attribute))
                        .min(crate::stats::PRE_VICTORY_ATTRIBUTE_INDEX_CAP),
                );
                let capacity = u32::from(RFB_MAGIC_STUDY[index])
                    .saturating_mul(u32::from(
                        self.progress
                            .level
                            .saturating_sub(profile.first_spell_level - 1),
                    ))
                    .saturating_div(2);
                if profile.learning_formula == CastingLearningFormula::RfbSingleRealm {
                    capacity.saturating_add(1) / 2
                } else {
                    capacity
                }
            }
        };
        (raw.min(u32::from(
            profile.learning_capacity_cap.saturating_add(realm_bonus),
        )) as u16)
            .saturating_add(self.bonus_spell_learning_capacity)
    }

    pub(super) fn resolve_player_clear_mind(&mut self, events: &mut Vec<DomainEvent>) {
        let amount = clear_mind_recovery_amount(self.progress.level);
        let resource_id = self
            .casting_profile()
            .expect("validated Clear Mind class must have a casting profile")
            .resource_id
            .clone();
        let pool = self
            .resources
            .get_mut(&resource_id)
            .expect("casting resource must exist");
        let before = pool.current;
        pool.current = pool.current.saturating_add(amount).min(pool.maximum);
        events.push(DomainEvent::ResourceRecovered {
            resolution: ResourceRecoveryResolutionDto {
                resource_id,
                before,
                after: pool.current,
                recovered: pool.current - before,
            },
        });
    }

    /// Single source for "which resource pools and abilities does the
    /// current build grant": initialization, level-up refresh, and load-time
    /// validation must all agree on this derivation.
    pub(super) fn player_ability_baseline(&self) -> (BTreeMap<String, u32>, BTreeSet<String>) {
        let mut pool_maxima = BTreeMap::new();
        let mut ability_ids = BTreeSet::new();
        if let Some(profile) = self.casting_profile() {
            pool_maxima.insert(
                profile.resource_id.clone(),
                self.casting_resource_maximum(profile),
            );
            ability_ids.extend(
                self.active_casting_book_ids()
                    .into_iter()
                    .filter_map(|book_id| self.content.ability_book(book_id))
                    .flat_map(|book| book.ability_ids.iter().cloned()),
            );
        }
        (pool_maxima, ability_ids)
    }

    pub(super) fn initialize_player_ability_state(&mut self) {
        self.resources.clear();
        self.spent_spell_learning = 0;
        self.learned_abilities.clear();
        self.ability_learning_order.clear();
        self.ability_progress.clear();
        self.refresh_player_ability_state();
    }

    pub(super) fn refresh_player_ability_state(&mut self) {
        self.refresh_player_resource_maxima();
        let (_, ability_ids) = self.player_ability_baseline();
        self.ability_progress
            .retain(|ability_id, _| ability_ids.contains(ability_id));
        for ability_id in ability_ids {
            if !self.ability_progress.contains_key(&ability_id)
                && let Some(ability) = self.content.ability(&ability_id)
            {
                let ability = self.effective_casting_ability(
                    self.casting_profile().expect("book caster"),
                    ability,
                );
                let player = Self::player_ability_parameters(&ability);
                self.ability_progress.insert(
                    ability_id,
                    AbilityProgress::new(player.proficiency.initial, player.proficiency.cap),
                );
            }
        }
    }

    pub(super) fn restore_player_ability_state(
        &mut self,
        saved_resources: Vec<ResourcePoolSaveDto>,
        saved_learned_ability_ids: Vec<String>,
        saved_ability_learning_order: Vec<String>,
        saved_spent_spell_learning: u32,
        saved_ability_progress: Vec<AbilityProgressSaveDto>,
    ) -> Result<(), CoreError> {
        self.initialize_player_ability_state();
        let mut seen = BTreeSet::new();
        for saved in saved_resources {
            let Some(pool) = self.resources.get_mut(&saved.id) else {
                return Err(CoreError::InvalidSave("player resource ID is invalid"));
            };
            if !seen.insert(saved.id)
                || saved.maximum != pool.maximum
                || saved.current > saved.maximum
            {
                return Err(CoreError::InvalidSave("player resource pool is invalid"));
            }
            pool.current = saved.current;
        }
        if seen.len() != self.resources.len() {
            return Err(CoreError::InvalidSave("player resource set is incomplete"));
        }

        let casting_profile = self.casting_profile().cloned();
        if casting_profile.is_none() && !saved_learned_ability_ids.is_empty() {
            return Err(CoreError::InvalidSave(
                "non-caster cannot have learned abilities",
            ));
        }
        if let Some(profile) = &casting_profile {
            let learning_capacity = usize::from(self.ability_learning_capacity(profile));
            if saved_learned_ability_ids.len() > learning_capacity {
                return Err(CoreError::InvalidSave(
                    "learned ability set exceeds learning capacity",
                ));
            }
            for ability_id in saved_learned_ability_ids {
                let Some(ability) = self.content.ability(&ability_id) else {
                    return Err(CoreError::InvalidSave("learned ability ID is invalid"));
                };
                let ability = self.effective_casting_ability(profile, ability);
                if Self::player_ability_parameters(&ability).minimum_level > self.progress.level
                    || !self.profile_supports_ability(profile, &ability_id)
                    || !self.learned_abilities.insert(ability_id)
                {
                    return Err(CoreError::InvalidSave("learned ability set is invalid"));
                }
            }
        }
        self.ability_learning_order = saved_ability_learning_order;
        self.spent_spell_learning = saved_spent_spell_learning;
        let mut seen_progress = BTreeSet::new();
        for saved in saved_ability_progress {
            if !seen_progress.insert(saved.id.clone()) {
                return Err(CoreError::InvalidSave("ability progress set is invalid"));
            }
            let cooldown_turns = self.ability_cooldown_turns(&saved.id);
            let Some(progress) = self.ability_progress.get_mut(&saved.id) else {
                return Err(CoreError::InvalidSave("ability progress ID is invalid"));
            };
            if saved.proficiency_cap != progress.proficiency_cap
                || saved.proficiency > saved.proficiency_cap
                || saved.cooldown_remaining > cooldown_turns
            {
                return Err(CoreError::InvalidSave(
                    "ability progress values are invalid",
                ));
            }
            progress.proficiency = saved.proficiency;
            progress.cast_count = saved.cast_count;
            progress.fail_count = saved.fail_count;
            progress.cooldown_remaining = saved.cooldown_remaining;
        }
        if !self.player_spell_memory_is_valid()
            || (self.player_uses_dual_realm_learning()
                && seen_progress.len() != self.ability_progress.len())
        {
            return Err(CoreError::InvalidSave("player spell memory is invalid"));
        }
        Ok(())
    }

    pub(super) fn refresh_player_resource_maxima(&mut self) {
        let (pool_maxima, _) = self.player_ability_baseline();
        for (resource_id, maximum) in &pool_maxima {
            let initial = initial_resource_pool(*maximum);
            let pool = self.resources.entry(resource_id.clone()).or_insert(initial);
            pool.maximum = *maximum;
            pool.current = pool.current.min(*maximum);
        }
        self.resources.retain(|id, _| pool_maxima.contains_key(id));
        self.refresh_player_spell_memory();
    }

    pub(super) fn profile_supports_ability(
        &self,
        _profile: &CastingProfileDefinition,
        ability_id: &str,
    ) -> bool {
        self.active_casting_book_ids().into_iter().any(|book_id| {
            self.content
                .ability_book(book_id)
                .is_some_and(|book| book.ability_ids.iter().any(|id| id == ability_id))
        })
    }

    pub(super) fn ability_book_item_id(
        &self,
        _profile: &CastingProfileDefinition,
        ability_id: &str,
    ) -> Option<String> {
        self.items
            .iter()
            .filter(|item| {
                item.location == ItemLocation::Inventory
                    || item.location == ItemLocation::Ground(self.player.position)
            })
            .filter_map(|item| {
                let book_id = self
                    .content
                    .item(&item.kind_id)?
                    .ability_book_id
                    .as_deref()?;
                if !self.active_casting_book_ids().contains(&book_id)
                    || !self
                        .content
                        .ability_book(book_id)
                        .is_some_and(|book| book.ability_ids.iter().any(|id| id == ability_id))
                {
                    return None;
                }
                Some(item.id.clone())
            })
            .min()
    }

    pub(super) fn ability_progress_value(&self, ability: &AbilityDefinition) -> AbilityProgress {
        self.ability_progress
            .get(&ability.id)
            .copied()
            .unwrap_or_else(|| {
                let player = Self::player_ability_parameters(ability);
                AbilityProgress::new(player.proficiency.initial, player.proficiency.cap)
            })
    }

    pub(super) fn ability_proficiency_rank(proficiency: u16) -> AbilityProficiencyRankDto {
        if proficiency < SPELL_EXP_BEGINNER {
            AbilityProficiencyRankDto::Unskilled
        } else if proficiency < SPELL_EXP_SKILLED {
            AbilityProficiencyRankDto::Beginner
        } else if proficiency < SPELL_EXP_EXPERT {
            AbilityProficiencyRankDto::Skilled
        } else if proficiency < SPELL_EXP_MASTER {
            AbilityProficiencyRankDto::Expert
        } else {
            AbilityProficiencyRankDto::Master
        }
    }

    pub(super) fn ability_effective_resource_cost(
        &self,
        ability: &AbilityDefinition,
        progress: AbilityProgress,
    ) -> u32 {
        let player = Self::player_ability_parameters(ability);
        let proficiency = u64::from(progress.proficiency.min(SPELL_EXP_MASTER));
        let factor = SPELL_MANA_CONST
            .saturating_add(SPELL_MANA_EXPERT)
            .saturating_sub(proficiency);
        let numerator = u64::from(player.resource_cost)
            .saturating_mul(factor)
            .saturating_add(SPELL_MANA_CONST.saturating_sub(1));
        let reduction = if self
            .player_equipment_passives()
            .contains(&EquipmentPassive::ReducedManaCost)
        {
            3
        } else {
            4
        };
        u32::try_from((numerator * reduction / (SPELL_MANA_CONST * 4)).max(1))
            .expect("validated ability mana cost must fit u32")
    }

    pub(super) fn ability_cooldown_turns(&self, ability_id: &str) -> u16 {
        let Some(ability) = self.content.ability(ability_id) else {
            return 0;
        };
        let Some(cooldown) = ability
            .player
            .as_ref()
            .and_then(|player| player.cooldown.as_ref())
        else {
            return 0;
        };
        let Some(group_id) = cooldown.group_id.as_deref() else {
            return cooldown.turns;
        };
        self.content
            .abilities()
            .filter_map(|candidate| {
                candidate
                    .player
                    .as_ref()
                    .and_then(|player| player.cooldown.as_ref())
                    .and_then(|candidate_cooldown| {
                        (candidate_cooldown.group_id.as_deref() == Some(group_id))
                            .then_some(candidate_cooldown.turns)
                    })
            })
            .max()
            .unwrap_or(cooldown.turns)
    }

    pub(super) fn ability_cooldown_remaining(&self, ability: &AbilityDefinition) -> u16 {
        let Some(cooldown) = ability
            .player
            .as_ref()
            .and_then(|player| player.cooldown.as_ref())
        else {
            return 0;
        };
        if let Some(group_id) = cooldown.group_id.as_deref() {
            self.content
                .abilities()
                .filter(|candidate| {
                    candidate
                        .player
                        .as_ref()
                        .and_then(|player| player.cooldown.as_ref())
                        .and_then(|cooldown| cooldown.group_id.as_deref())
                        == Some(group_id)
                })
                .filter_map(|candidate| self.ability_progress.get(&candidate.id))
                .map(|progress| progress.cooldown_remaining)
                .max()
                .unwrap_or(0)
        } else {
            self.ability_progress
                .get(&ability.id)
                .map_or(0, |progress| progress.cooldown_remaining)
        }
    }

    pub(super) fn decrement_ability_cooldowns(&mut self, turns: u16) {
        if turns == 0 {
            return;
        }
        for progress in self.ability_progress.values_mut() {
            progress.cooldown_remaining = progress.cooldown_remaining.saturating_sub(turns);
        }
    }

    pub(super) fn record_ability_cast(
        &mut self,
        ability: &AbilityDefinition,
        succeeded: bool,
    ) -> AbilityProgress {
        let player = Self::player_ability_parameters(ability).clone();
        let book_practice = self.player_uses_dual_realm_learning();
        let progress = self
            .ability_progress
            .entry(ability.id.clone())
            .or_insert_with(|| {
                AbilityProgress::new(player.proficiency.initial, player.proficiency.cap)
            });
        if succeeded {
            progress.cast_count = progress.cast_count.saturating_add(1);
            progress.proficiency = progress
                .proficiency
                .saturating_add(if book_practice {
                    0
                } else {
                    player.proficiency.success_gain
                })
                .min(progress.proficiency_cap);
        } else {
            progress.fail_count = progress.fail_count.saturating_add(1);
            progress.proficiency = progress
                .proficiency
                .saturating_add(if book_practice {
                    0
                } else {
                    player.proficiency.failure_gain
                })
                .min(progress.proficiency_cap);
        }
        if succeeded && let Some(cooldown) = player.cooldown.as_ref() {
            if let Some(group_id) = cooldown.group_id.as_deref() {
                let group_ids = self
                    .content
                    .abilities()
                    .filter(|candidate| {
                        candidate
                            .player
                            .as_ref()
                            .and_then(|player| player.cooldown.as_ref())
                            .and_then(|cooldown| cooldown.group_id.as_deref())
                            == Some(group_id)
                    })
                    .map(|candidate| candidate.id.clone())
                    .collect::<Vec<_>>();
                for id in group_ids {
                    if let Some(member) = self.ability_progress.get_mut(&id) {
                        member.cooldown_remaining = cooldown.turns;
                    }
                }
            } else {
                progress.cooldown_remaining = cooldown.turns;
            }
        }
        self.ability_progress
            .get(&ability.id)
            .copied()
            .expect("ability progress must remain available")
    }

    pub(super) fn ability_failure_percent(
        &self,
        profile: &CastingProfileDefinition,
        ability: &AbilityDefinition,
    ) -> u8 {
        self.profile_failure_percent(
            profile,
            ability,
            self.player_spell_failure_modifier_percent(),
        )
        .max(
            u8::try_from(self.player_spell_failure_minimum_percent())
                .expect("mutation spell failure minimum must fit u8"),
        )
    }

    pub(super) fn recover_player_resources(
        &mut self,
        resting: bool,
        events: &mut Vec<DomainEvent>,
    ) {
        let changes = self
            .resources
            .keys()
            .map(|id| {
                (
                    id.clone(),
                    self.player_resource_recovery_change(id, resting),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let upkeep_percent = self.pet_upkeep().percent;
        for (id, pool) in &mut self.resources {
            let before = pool.current;
            let change = changes[id];
            if change >= 0 {
                pool.current = pool
                    .current
                    .saturating_add(u32::try_from(change).unwrap_or(u32::MAX))
                    .min(pool.maximum);
            } else {
                pool.current = pool
                    .current
                    .saturating_sub(u32::try_from(-change).unwrap_or(u32::MAX));
            }
            if pool.current > before {
                events.push(DomainEvent::ResourceRecovered {
                    resolution: ResourceRecoveryResolutionDto {
                        resource_id: id.clone(),
                        before,
                        after: pool.current,
                        recovered: pool.current - before,
                    },
                });
            } else if pool.current < before {
                events.push(DomainEvent::PetUpkeepManaLost {
                    resource_id: id.clone(),
                    amount: before - pool.current,
                    upkeep_percent,
                });
            }
        }
        if self.pet_upkeep_dto().dismissal_required {
            events.push(DomainEvent::PetUpkeepDismissalRequired { upkeep_percent });
        }
    }

    pub(super) fn player_resource_recovery_amount(&self, id: &str, resting: bool) -> u32 {
        u32::try_from(self.player_resource_recovery_change(id, resting).max(0)).unwrap_or(u32::MAX)
    }

    fn player_has_depleted_recoverable_resource(&self, resting: bool) -> bool {
        self.resources.iter().any(|(id, pool)| {
            pool.current < pool.maximum && self.player_resource_recovery_amount(id, resting) > 0
        })
    }

    fn player_has_rest_need(&self) -> bool {
        self.player.hp < self.effective_player_max_hp()
            || self.player_has_depleted_recoverable_resource(true)
            || self.magic_eater_can_regen()
    }

    fn visible_hostile_exists(&self) -> bool {
        self.entities.iter().any(|entity| {
            entity.hp > 0
                && !self.actor_is_player_side(entity)
                && self.entity_is_visible_to_player(entity)
        })
    }

    pub(super) fn resolve_player_rest(
        &mut self,
        requested_turns: u16,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<RestResolutionDto, CoreError> {
        let resource_before = self
            .resources
            .iter()
            .map(|(id, pool)| (id.clone(), pool.current))
            .collect::<BTreeMap<_, _>>();
        let mut completed_turns = 0_u16;
        let stop_reason = if requested_turns == 0 || requested_turns > MAX_REST_TURNS {
            RestStopReasonDto::InvalidTurns
        } else if self.pet_upkeep_dto().dismissal_required {
            RestStopReasonDto::PetDismissalRequired
        } else if !self.player_has_rest_need() {
            RestStopReasonDto::FullResources
        } else if self.visible_hostile_exists() {
            RestStopReasonDto::EnemyVisible
        } else {
            loop {
                let hp_before = self.player.hp;
                let pet_neglect_allowed = self.pet_upkeep().unsafe_warning();
                spend_energy(&mut self.player.energy_need, STANDARD_ACTION_COST);
                self.advance_until_player_ready(
                    true,
                    true,
                    pet_neglect_allowed,
                    events,
                    changed,
                    removed_entities,
                )?;
                completed_turns = completed_turns.saturating_add(1);
                if self.duelist_prompt().is_some() {
                    break RestStopReasonDto::DuelistChoiceRequired;
                }
                if self.pending_mutation_direction.is_some() {
                    break RestStopReasonDto::MutationDirectionRequired;
                }
                if self.player_is_dead() {
                    break RestStopReasonDto::PlayerDied;
                }
                // RFB regenerates mana before wall damage's cave_no_regen HP gate.
                self.recover_player_resources(true, events);
                if self.player.hp < hp_before {
                    break RestStopReasonDto::Damaged;
                }
                if self.visible_hostile_exists() {
                    break RestStopReasonDto::EnemyVisible;
                }
                if self.pet_upkeep_dto().dismissal_required {
                    break RestStopReasonDto::PetDismissalRequired;
                }
                if !self.player_has_rest_need() {
                    break RestStopReasonDto::FullResources;
                }
                if completed_turns >= requested_turns {
                    break RestStopReasonDto::TurnLimit;
                }
            }
        };
        let resource_recoveries = self
            .resources
            .iter()
            .filter_map(|(id, pool)| {
                let before = resource_before.get(id).copied().unwrap_or(pool.current);
                (pool.current > before).then(|| ResourceRecoveryResolutionDto {
                    resource_id: id.clone(),
                    before,
                    after: pool.current,
                    recovered: pool.current - before,
                })
            })
            .collect();
        Ok(RestResolutionDto {
            requested_turns,
            completed_turns,
            stop_reason,
            resource_recoveries,
        })
    }
}
