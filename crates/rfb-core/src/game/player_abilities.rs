// SPDX-License-Identifier: MPL-2.0

use super::*;

const SPELL_EXP_BEGINNER: u16 = 900;
const SPELL_EXP_SKILLED: u16 = 1200;
pub(in crate::game) const SPELL_EXP_EXPERT: u16 = 1400;
pub(in crate::game) const SPELL_EXP_MASTER: u16 = 1600;
const SPELL_MANA_CONST: u64 = 2400;
const SPELL_MANA_EXPERT: u64 = 1400;
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
            build.second_realm_id.as_deref(),
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
            class
                .abilities
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
            if !override_.level_scaling.is_empty() {
                effective.level_scaling.clone_from(&override_.level_scaling);
            }
        }
        effective
    }

    pub(super) fn apply_player_level_scaling(ability: &mut AbilityDefinition, level: u16) {
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
    }

    pub(super) fn apply_casting_profile_effect_scaling(
        profile: &CastingProfileDefinition,
        ability: &mut AbilityDefinition,
        level: u16,
    ) {
        let AbilityEffectDefinition::BoltOrBeamDamage {
            beam_chance_percent,
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
            .clamp(0, 100);
        *beam_chance_percent =
            u8::try_from(chance).expect("clamped casting beam chance must fit u8");
    }

    pub(super) fn apply_casting_profile_damage_bonus(
        profile: &CastingProfileDefinition,
        ability: &mut AbilityDefinition,
        level: u16,
    ) {
        let bonus = profile.spell_damage_bonus_base.saturating_add(
            profile
                .spell_damage_bonus_per_level
                .saturating_mul(level / u16::from(profile.spell_damage_bonus_level_divisor)),
        );
        if bonus == 0 {
            return;
        }
        fn apply(effect: &mut AbilityEffectDefinition, bonus: u16) {
            match effect {
                AbilityEffectDefinition::Damage { damage_bonus, .. }
                | AbilityEffectDefinition::AreaDamage { damage_bonus, .. }
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
                    .saturating_add(modifier_percent)
                    .saturating_add(i32::try_from(resource_penalty).unwrap_or(i32::MAX))
                    .clamp(i32::from(minimum_failure_percent), 95)
                    .saturating_sub(proficiency_adjustment)
                    .max(0)
            }
        };
        u8::try_from(chance).expect("validated ability failure chance must fit u8")
    }

    fn casting_attribute_kind(attribute: CastingAttribute) -> AttributeKind {
        match attribute {
            CastingAttribute::Intelligence => AttributeKind::Intelligence,
            CastingAttribute::Wisdom => AttributeKind::Wisdom,
            CastingAttribute::Charisma => AttributeKind::Charisma,
        }
    }

    fn casting_resource_maximum(&self, profile: &CastingProfileDefinition) -> u32 {
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
                let mut value = u32::from(RFB_MAGIC_MANA[usize::from(attribute_index)])
                    .saturating_mul(u32::from(self.progress.level).saturating_add(3))
                    / 4;
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
                self.content.item(&item.kind_id)
            });
            let mut weight = 0_u32;
            let mut cumbersome_gloves = false;
            for item in equipped {
                match item.equipment_slot.as_deref() {
                    Some("body" | "head" | "shield" | "cloak" | "gloves" | "boots") => {
                        weight = weight.saturating_add(u32::from(item.weight_tenths_pound));
                        cumbersome_gloves |= item.equipment_slot.as_deref() == Some("gloves");
                    }
                    Some("weapon") => {
                        weight = weight.saturating_add(
                            u32::from(item.weight_tenths_pound)
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
        maximum.saturating_mul(u32::from(profile.capacity_percent)) / 100
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

    pub(super) fn mutation_failure_percent(&self, activation: &MutationActivationDefinition) -> u8 {
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
            .clamp(
                i32::from(
                    activation
                        .minimum_failure_percent
                        .unwrap_or(RFB_MAGIC_FAILURE_MINIMUM[index])
                        .max(RFB_MAGIC_FAILURE_MINIMUM[index]),
                ),
                95,
            );
        u8::try_from(chance).expect("bounded mutation failure chance must fit u8")
    }

    pub(super) fn class_ability_failure_percent(&self, activation: &ClassAbilityDefinition) -> u8 {
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
            .clamp(
                i32::from(
                    activation
                        .minimum_failure_percent
                        .max(RFB_MAGIC_FAILURE_MINIMUM[index]),
                ),
                95,
            );
        u8::try_from(chance).expect("bounded class ability failure chance must fit u8")
    }

    pub(super) fn mutation_resource_cost(&self, activation: &MutationActivationDefinition) -> u32 {
        let extra = activation.cost_scaling.map_or(0, |scaling| {
            if self.progress.level < scaling.start_level {
                0
            } else {
                u32::from((self.progress.level - scaling.start_level) / scaling.level_interval + 1)
                    .saturating_mul(scaling.amount)
            }
        });
        activation.cost.saturating_add(extra)
    }

    pub(super) fn ability_learning_capacity(&self, profile: &CastingProfileDefinition) -> u16 {
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
            CastingLearningFormula::RfbSingleRealm => {
                let index = usize::from(
                    self.effective_player_attributes()
                        .index(Self::casting_attribute_kind(profile.casting_attribute))
                        .min(crate::stats::PRE_VICTORY_ATTRIBUTE_INDEX_CAP),
                );
                u32::from(RFB_MAGIC_STUDY[index])
                    .saturating_mul(u32::from(self.progress.level))
                    .saturating_div(2)
                    .saturating_add(1)
                    .saturating_div(2)
            }
        };
        (raw.min(u32::from(
            profile.learning_capacity_cap.saturating_add(realm_bonus),
        )) as u16)
            .saturating_add(self.bonus_spell_learning_capacity)
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
        self.learned_abilities.clear();
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
                let player = Self::player_ability_parameters(ability);
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
            .filter(|item| item.location == ItemLocation::Inventory)
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
        u32::try_from((numerator / SPELL_MANA_CONST).max(1))
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
                .saturating_add(player.proficiency.success_gain)
                .min(progress.proficiency_cap);
        } else {
            progress.fail_count = progress.fail_count.saturating_add(1);
            progress.proficiency = progress
                .proficiency
                .saturating_add(player.proficiency.failure_gain)
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
    }

    pub(super) fn recover_player_resources(
        &mut self,
        resting: bool,
    ) -> Vec<ResourceRecoveryResolutionDto> {
        let recovery_amounts = self
            .resources
            .keys()
            .map(|id| {
                let definition = self
                    .content
                    .resource(id)
                    .expect("player resource definition must remain available");
                let amount = if resting {
                    definition.rest_recovery_amount
                } else {
                    definition.wait_recovery_amount
                };
                let percent = self.casting_profile().map_or(100, |profile| {
                    if profile.resource_id == *id {
                        profile.resource_recovery_percent
                    } else {
                        100
                    }
                });
                (id.clone(), amount.saturating_mul(u32::from(percent)) / 100)
            })
            .collect::<BTreeMap<_, _>>();
        let mut recovered = Vec::new();
        for (id, pool) in &mut self.resources {
            let before = pool.current;
            pool.current = pool
                .current
                .saturating_add(recovery_amounts[id])
                .min(pool.maximum);
            if pool.current > before {
                recovered.push(ResourceRecoveryResolutionDto {
                    resource_id: id.clone(),
                    before,
                    after: pool.current,
                    recovered: pool.current - before,
                });
            }
        }
        recovered
    }

    pub(super) fn player_resource_recovery_amount(&self, id: &str, resting: bool) -> u32 {
        let Some(definition) = self.content.resource(id) else {
            return 0;
        };
        let amount = if resting {
            definition.rest_recovery_amount
        } else {
            definition.wait_recovery_amount
        };
        let percent = self.casting_profile().map_or(100, |profile| {
            if profile.resource_id == id {
                profile.resource_recovery_percent
            } else {
                100
            }
        });
        amount.saturating_mul(u32::from(percent)) / 100
    }

    fn player_has_depleted_recoverable_resource(&self, resting: bool) -> bool {
        self.resources.iter().any(|(id, pool)| {
            if pool.current >= pool.maximum {
                return false;
            }
            self.content.resource(id).is_some_and(|definition| {
                if resting {
                    definition.rest_recovery_amount > 0
                } else {
                    definition.wait_recovery_amount > 0
                }
            })
        })
    }

    fn player_has_rest_need(&self) -> bool {
        self.player.hp < self.effective_player_max_hp()
            || self.player_has_depleted_recoverable_resource(true)
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
        } else if !self.player_has_rest_need() {
            RestStopReasonDto::FullResources
        } else if self.visible_hostile_exists() {
            RestStopReasonDto::EnemyVisible
        } else {
            loop {
                let hp_before = self.player.hp;
                spend_energy(&mut self.player.energy_need, STANDARD_ACTION_COST);
                self.advance_until_player_ready(true, true, events, changed, removed_entities)?;
                completed_turns = completed_turns.saturating_add(1);
                if self.pending_mutation_direction.is_some() {
                    break RestStopReasonDto::MutationDirectionRequired;
                }
                if self.player_is_dead() {
                    break RestStopReasonDto::PlayerDied;
                }
                if self.player.hp < hp_before {
                    break RestStopReasonDto::Damaged;
                }
                if self.visible_hostile_exists() {
                    break RestStopReasonDto::EnemyVisible;
                }
                self.recover_player_resources(true);
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
