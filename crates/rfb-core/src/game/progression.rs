// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeMap;

use rfb_content::{
    CharacterBuildDefinition, ClassDefinition, ContentCatalog, PersonalityDefinition,
    RaceDefinition, SkillSetDefinition, StatModifiers,
};
use rfb_protocol::StatModifiersDto;

use crate::{
    effect::STATUS_UNWELL,
    error::CoreError,
    event::DomainEvent,
    rng::RfbRng,
    state::ResourcePool,
    stats::{
        AttributeKind, AttributeSet, CharacterBuildIdentity, CharacterProgress, SkillProgress,
        modify_attribute_value,
    },
};

use super::Game;

pub(super) type CharacterDefinitions<'a> = (
    &'a CharacterBuildDefinition,
    &'a RaceDefinition,
    &'a ClassDefinition,
    &'a PersonalityDefinition,
);

struct AttributeIncreasePlan {
    attributes: AttributeSet,
    maximum_attributes: AttributeSet,
    pending_attribute_increases: u16,
}

struct ExperienceGainPlan {
    amount: u64,
    progress: CharacterProgress,
    levels: Vec<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AttributeMutationOutcome {
    pub(super) attribute: AttributeKind,
    pub(super) before: u16,
    pub(super) after: u16,
    pub(super) maximum_before: u16,
    pub(super) maximum_after: u16,
    pub(super) changed: bool,
}

fn attribute_mutation_outcome(
    progress: &CharacterProgress,
    attribute: AttributeKind,
    before: u16,
    maximum_before: u16,
    changed: bool,
) -> AttributeMutationOutcome {
    AttributeMutationOutcome {
        attribute,
        before,
        after: progress.attributes.value(attribute),
        maximum_before,
        maximum_after: progress.maximum_attributes.value(attribute),
        changed,
    }
}

pub(super) fn apply_attribute_drain(
    progress: &mut CharacterProgress,
    attribute: AttributeKind,
    rng: &mut RfbRng,
) -> AttributeMutationOutcome {
    let before = progress.attributes.value(attribute);
    let maximum_before = progress.maximum_attributes.value(attribute);
    let changed = progress.drain_attribute(attribute, rng);
    attribute_mutation_outcome(progress, attribute, before, maximum_before, changed)
}

pub(super) fn apply_attribute_drain_with_amount(
    progress: &mut CharacterProgress,
    attribute: AttributeKind,
    amount: u8,
    rng: &mut RfbRng,
) -> AttributeMutationOutcome {
    let before = progress.attributes.value(attribute);
    let maximum_before = progress.maximum_attributes.value(attribute);
    let changed = progress.drain_attribute_by(attribute, amount, rng);
    attribute_mutation_outcome(progress, attribute, before, maximum_before, changed)
}

pub(super) fn apply_permanent_attribute_drain(
    progress: &mut CharacterProgress,
    attribute: AttributeKind,
    amount: u8,
    rng: &mut RfbRng,
) -> AttributeMutationOutcome {
    let before = progress.attributes.value(attribute);
    let maximum_before = progress.maximum_attributes.value(attribute);
    let changed = progress.permanently_drain_attribute(attribute, amount, rng);
    attribute_mutation_outcome(progress, attribute, before, maximum_before, changed)
}

pub(super) fn apply_attribute_restoration(
    progress: &mut CharacterProgress,
    attribute: AttributeKind,
) -> AttributeMutationOutcome {
    let before = progress.attributes.value(attribute);
    let maximum_before = progress.maximum_attributes.value(attribute);
    let changed = progress.restore_attribute(attribute);
    attribute_mutation_outcome(progress, attribute, before, maximum_before, changed)
}

pub(super) fn apply_permanent_attribute_increase(
    progress: &mut CharacterProgress,
    attribute: AttributeKind,
    victorious: bool,
    rng: &mut RfbRng,
) -> AttributeMutationOutcome {
    let before = progress.attributes.value(attribute);
    let maximum_before = progress.maximum_attributes.value(attribute);
    let changed = progress.increase_attribute_permanently(attribute, victorious, rng);
    attribute_mutation_outcome(progress, attribute, before, maximum_before, changed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ExperienceRestorationOutcome {
    pub(super) before: u64,
    pub(super) after: u64,
}

pub(super) fn apply_experience_restoration(
    progress: &mut CharacterProgress,
) -> ExperienceRestorationOutcome {
    let before = progress.experience;
    progress.experience = progress.maximum_experience;
    ExperienceRestorationOutcome {
        before,
        after: progress.experience,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LifeForceRestoration {
    Add(u16),
    AtLeast(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LifeForceRestorationRequest {
    restoration: LifeForceRestoration,
}

impl LifeForceRestorationRequest {
    pub(super) const fn add(amount: u16) -> Self {
        Self {
            restoration: LifeForceRestoration::Add(amount),
        }
    }

    pub(super) const fn at_least(minimum: u16) -> Self {
        Self {
            restoration: LifeForceRestoration::AtLeast(minimum),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LifeForceRestorationOutcome {
    pub(super) before: u16,
    pub(super) after: u16,
}

pub(super) fn apply_life_force_restoration(
    progress: &mut CharacterProgress,
    request: LifeForceRestorationRequest,
) -> LifeForceRestorationOutcome {
    let before = progress.life_force;
    progress.life_force = match request.restoration {
        LifeForceRestoration::Add(amount) => progress.life_force.saturating_add(amount).min(1_000),
        LifeForceRestoration::AtLeast(minimum) => progress.life_force.max(minimum).min(1_000),
    };
    LifeForceRestorationOutcome {
        before,
        after: progress.life_force,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LearningCapacityIncreaseOutcome {
    pub(super) before: u16,
    pub(super) after: u16,
}

pub(super) fn apply_learning_capacity_increase(
    bonus_capacity: &mut u16,
    eligible: bool,
) -> LearningCapacityIncreaseOutcome {
    let before = *bonus_capacity;
    if eligible {
        *bonus_capacity = bonus_capacity.saturating_add(1);
    }
    LearningCapacityIncreaseOutcome {
        before,
        after: *bonus_capacity,
    }
}

pub(super) fn resolve_character_build(
    content: &ContentCatalog,
    build_id: Option<&str>,
) -> Result<Option<CharacterBuildIdentity>, CoreError> {
    let Some(build_id) = build_id else {
        return Ok(None);
    };
    let build = content
        .build(build_id)
        .ok_or_else(|| CoreError::UnknownCharacterBuild(build_id.to_owned()))?;
    Ok(Some(CharacterBuildIdentity {
        build_id: build.id.clone(),
        race_id: build.race_id.clone(),
        class_id: build.class_id.clone(),
        personality_id: build.personality_id.clone(),
    }))
}

pub(super) fn build_definitions<'a>(
    content: &'a ContentCatalog,
    identity: &'a CharacterBuildIdentity,
) -> Result<CharacterDefinitions<'a>, CoreError> {
    let build = content
        .build(&identity.build_id)
        .ok_or_else(|| CoreError::UnknownCharacterBuild(identity.build_id.clone()))?;
    let race = content
        .race(&identity.race_id)
        .ok_or_else(|| CoreError::UnknownCharacterBuild(identity.build_id.clone()))?;
    let class = content
        .class(&identity.class_id)
        .ok_or_else(|| CoreError::UnknownCharacterBuild(identity.build_id.clone()))?;
    let personality = content
        .personality(&identity.personality_id)
        .ok_or_else(|| CoreError::UnknownCharacterBuild(identity.build_id.clone()))?;
    Ok((build, race, class, personality))
}

pub(super) fn initial_character_attributes(definition: &CharacterBuildDefinition) -> AttributeSet {
    AttributeSet {
        strength: definition.attributes.strength,
        intelligence: definition.attributes.intelligence,
        wisdom: definition.attributes.wisdom,
        dexterity: definition.attributes.dexterity,
        constitution: definition.attributes.constitution,
        charisma: definition.attributes.charisma,
    }
}

pub(super) fn character_skill_progress(
    content: &ContentCatalog,
    identity: Option<&CharacterBuildIdentity>,
    level: u16,
) -> Result<BTreeMap<String, SkillProgress>, CoreError> {
    let Some(identity) = identity else {
        return Ok(BTreeMap::new());
    };
    let (_, race, class, personality) = build_definitions(content, identity)?;
    skill_progress_for_definitions(content, identity, race, class, personality, level)
}

fn skill_progress_for_definitions(
    content: &ContentCatalog,
    identity: &CharacterBuildIdentity,
    race: &RaceDefinition,
    class: &ClassDefinition,
    personality: &PersonalityDefinition,
    level: u16,
) -> Result<BTreeMap<String, SkillProgress>, CoreError> {
    let mut totals = BTreeMap::<String, (i32, i32, i32)>::new();
    for skill_set_id in [
        race.skill_set_id.as_str(),
        class.skill_set_id.as_str(),
        personality.skill_set_id.as_str(),
    ] {
        let skill_set = content
            .skill_set(skill_set_id)
            .ok_or_else(|| CoreError::UnknownCharacterBuild(identity.build_id.clone()))?;
        accumulate_skill_set(content, skill_set, &mut totals, identity)?;
    }
    Ok(totals
        .into_iter()
        .map(|(id, (base, growth, maximum))| {
            (id, SkillProgress::at_level(base, growth, maximum, level))
        })
        .collect())
}

fn accumulate_skill_set(
    content: &ContentCatalog,
    skill_set: &SkillSetDefinition,
    totals: &mut BTreeMap<String, (i32, i32, i32)>,
    identity: &CharacterBuildIdentity,
) -> Result<(), CoreError> {
    for entry in &skill_set.entries {
        let maximum = content
            .skill(&entry.skill_id)
            .ok_or_else(|| CoreError::UnknownCharacterBuild(identity.build_id.clone()))?
            .maximum;
        let total = totals
            .entry(entry.skill_id.clone())
            .or_insert((0, 0, maximum));
        total.0 = total.0.saturating_add(entry.base);
        total.1 = total.1.saturating_add(entry.growth_per_ten_levels);
        total.2 = total.2.min(maximum);
    }
    Ok(())
}

pub(super) fn combine_percentages(percentages: [u16; 3]) -> u16 {
    let product = percentages.into_iter().fold(1_u64, |total, percentage| {
        total.saturating_mul(u64::from(percentage))
    });
    u16::try_from(product.saturating_add(5_000).saturating_div(10_000)).unwrap_or(u16::MAX)
}

fn apply_attribute_modifiers(
    attributes: AttributeSet,
    modifiers: &StatModifiers,
    cap: u16,
) -> AttributeSet {
    AttributeSet {
        strength: modify_attribute_value(attributes.strength, modifiers.strength, cap),
        intelligence: modify_attribute_value(attributes.intelligence, modifiers.intelligence, cap),
        wisdom: modify_attribute_value(attributes.wisdom, modifiers.wisdom, cap),
        dexterity: modify_attribute_value(attributes.dexterity, modifiers.dexterity, cap),
        constitution: modify_attribute_value(attributes.constitution, modifiers.constitution, cap),
        charisma: modify_attribute_value(attributes.charisma, modifiers.charisma, cap),
    }
}

fn apply_attribute_dto_modifiers(
    attributes: AttributeSet,
    modifiers: StatModifiersDto,
    cap: u16,
) -> AttributeSet {
    AttributeSet {
        strength: modify_attribute_value(attributes.strength, modifiers.strength, cap),
        intelligence: modify_attribute_value(attributes.intelligence, modifiers.intelligence, cap),
        wisdom: modify_attribute_value(attributes.wisdom, modifiers.wisdom, cap),
        dexterity: modify_attribute_value(attributes.dexterity, modifiers.dexterity, cap),
        constitution: modify_attribute_value(attributes.constitution, modifiers.constitution, cap),
        charisma: modify_attribute_value(attributes.charisma, modifiers.charisma, cap),
    }
}

fn effective_attributes<'a>(
    mut attributes: AttributeSet,
    character_modifiers: Option<[&StatModifiers; 3]>,
    mutation_modifiers: impl IntoIterator<Item = &'a StatModifiers>,
    equipment_modifiers: StatModifiersDto,
    status_modifiers: impl IntoIterator<Item = StatModifiersDto>,
    normal_appearance_minimum: Option<u16>,
    cap: u16,
) -> AttributeSet {
    if let Some(modifiers) = character_modifiers {
        for modifiers in modifiers {
            attributes = apply_attribute_modifiers(attributes, modifiers, cap);
        }
    }
    for modifiers in mutation_modifiers {
        let charisma = attributes.charisma;
        attributes = apply_attribute_modifiers(attributes, modifiers, cap);
        if normal_appearance_minimum.is_some() {
            attributes.charisma = charisma;
        }
    }
    attributes = apply_attribute_dto_modifiers(attributes, equipment_modifiers, cap);
    for modifiers in status_modifiers {
        attributes = apply_attribute_dto_modifiers(attributes, modifiers, cap);
    }
    if let Some(minimum) = normal_appearance_minimum {
        attributes.charisma = attributes.charisma.max(minimum.min(cap));
    }
    attributes
}

fn character_experience_percent(definitions: Option<CharacterDefinitions<'_>>) -> u16 {
    definitions.map_or(100, |(_, race, class, personality)| {
        combine_percentages([
            race.experience_percent,
            class.experience_percent,
            personality.experience_percent,
        ])
    })
}

fn character_modifier_total(
    definitions: Option<CharacterDefinitions<'_>>,
    value: impl Fn(&StatModifiers) -> i32,
) -> i32 {
    definitions.map_or(0, |(_, race, class, personality)| {
        value(&race.modifiers)
            .saturating_add(value(&class.modifiers))
            .saturating_add(value(&personality.modifiers))
    })
}

fn character_base_max_hp_at_level(
    hp_progression: &[i32],
    level: u16,
    definitions: Option<CharacterDefinitions<'_>>,
    constitution_percent: i32,
) -> i32 {
    let mut base = hp_progression
        .get(usize::from(level.saturating_sub(1)))
        .copied()
        .unwrap_or(1);
    let mut life_percent = 100_u16;
    if let Some((_, race, class, personality)) = definitions {
        base = base
            .saturating_add(race.base_hp)
            .saturating_add(class.base_hp)
            .saturating_add(personality.base_hp)
            .max(1);
        life_percent = combine_percentages([
            race.life_percent,
            class.life_percent,
            personality.life_percent,
        ]);
    }
    base.saturating_mul(i32::from(life_percent))
        .saturating_add(50)
        .saturating_div(100)
        .saturating_mul(constitution_percent)
        .saturating_add(50)
        .saturating_div(100)
        .max(1)
}

pub(super) fn profile_resource_maximum(
    level: u16,
    attribute_index: u8,
    params: (u32, u32, u32),
) -> u32 {
    let (base_capacity, capacity_per_level, capacity_per_attribute_index) = params;
    base_capacity
        .saturating_add(capacity_per_level.saturating_mul(u32::from(level)))
        .saturating_add(capacity_per_attribute_index.saturating_mul(u32::from(attribute_index)))
}

pub(super) fn initial_resource_pool(
    content: &ContentCatalog,
    resource_id: &str,
    maximum: u32,
) -> ResourcePool {
    let fill_percent = content
        .resource(resource_id)
        .map_or(100, |definition| u32::from(definition.initial_fill_percent));
    let current = u32::try_from(u64::from(maximum) * u64::from(fill_percent) / 100)
        .expect("initial resource fill must fit u32");
    ResourcePool { current, maximum }
}

fn plan_attribute_increase(
    progress: &CharacterProgress,
    attribute: AttributeKind,
    victorious: bool,
) -> Option<AttributeIncreasePlan> {
    let mut planned = progress.clone();
    planned.increase_attribute(attribute, victorious)?;
    Some(AttributeIncreasePlan {
        attributes: planned.attributes,
        maximum_attributes: planned.maximum_attributes,
        pending_attribute_increases: planned.pending_attribute_increases,
    })
}

fn plan_experience_gain(
    progress: &CharacterProgress,
    amount: u64,
    victorious: bool,
) -> ExperienceGainPlan {
    let mut progress = progress.clone();
    let levels = progress.gain_experience(amount, victorious);
    ExperienceGainPlan {
        amount,
        progress,
        levels,
    }
}

fn scale_experience_reward(amount: u64, experience_percent: u16) -> u64 {
    amount
        .saturating_mul(u64::from(experience_percent))
        .saturating_add(50)
        .saturating_div(100)
}

fn rescale_i32(current: i32, previous_maximum: i32, next_maximum: i32) -> i32 {
    i32::try_from(
        i64::from(current)
            .saturating_mul(i64::from(next_maximum))
            .saturating_div(i64::from(previous_maximum)),
    )
    .unwrap_or_else(|_| {
        if current.is_negative() {
            i32::MIN
        } else {
            i32::MAX
        }
    })
}

fn rescale_u32(current: u32, previous_maximum: u32, next_maximum: u32) -> u32 {
    u32::try_from(
        u64::from(current)
            .saturating_mul(u64::from(next_maximum))
            .saturating_div(u64::from(previous_maximum)),
    )
    .unwrap_or(u32::MAX)
    .min(next_maximum)
}

impl Game {
    pub(super) fn character_definitions(&self) -> Option<CharacterDefinitions<'_>> {
        let (build, base_race, class, personality) = self
            .build
            .as_ref()
            .map(|identity| build_definitions(&self.content, identity))
            .transpose()
            .expect("validated character build must remain available")?;
        let race = self
            .player
            .statuses
            .iter()
            .filter_map(|status| {
                status
                    .granted_race_id
                    .as_deref()
                    .map(|race_id| (status.kind_id.as_str(), race_id))
            })
            .min_by(|left, right| left.cmp(right))
            .and_then(|(_, race_id)| self.content.race(race_id))
            .unwrap_or(base_race);
        Some((build, race, class, personality))
    }

    pub(super) fn effective_player_attributes(&self) -> AttributeSet {
        let cap = CharacterProgress::attribute_cap(self.victory_level_cap_unlocked());
        let active_mutations = self
            .content
            .mutations()
            .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
            .collect::<Vec<_>>();
        let normal_appearance_minimum = active_mutations
            .iter()
            .any(|mutation| mutation.normal_appearance)
            .then(|| 8_u16.saturating_add(self.progress.level.saturating_mul(2)));
        let character_modifiers =
            self.character_definitions()
                .map(|(_, race, class, personality)| {
                    [&race.modifiers, &class.modifiers, &personality.modifiers]
                });
        let status_modifiers = self
            .player
            .statuses
            .iter()
            .map(|status| {
                let mut modifiers = status.granted_modifiers;
                if status.kind_id == STATUS_UNWELL {
                    let penalty = if status.remaining_ticks > 55 {
                        0
                    } else if status.remaining_ticks > 30 {
                        4
                    } else {
                        i32::try_from(status.remaining_ticks.div_ceil(10)).unwrap_or(i32::MAX)
                    };
                    modifiers.dexterity = modifiers.dexterity.saturating_sub(penalty);
                    modifiers.constitution = modifiers.constitution.saturating_sub(penalty);
                }
                modifiers
            })
            .collect::<Vec<_>>();
        effective_attributes(
            self.progress.attributes,
            character_modifiers,
            active_mutations.iter().map(|mutation| &mutation.modifiers),
            self.equipment_modifiers(),
            status_modifiers,
            normal_appearance_minimum,
            cap,
        )
    }

    pub(super) fn effective_player_skill_progress(&self) -> BTreeMap<String, SkillProgress> {
        let Some((_, race, class, personality)) = self.character_definitions() else {
            return self.progress.skills.clone();
        };
        let identity = self
            .build
            .as_ref()
            .expect("character definitions require a build identity");
        skill_progress_for_definitions(
            &self.content,
            identity,
            race,
            class,
            personality,
            self.progress.level,
        )
        .expect("validated character skills must remain available")
    }

    fn character_experience_percent(&self) -> u16 {
        character_experience_percent(self.character_definitions())
    }

    fn character_modifier_total(&self, value: impl Fn(&StatModifiers) -> i32) -> i32 {
        character_modifier_total(self.character_definitions(), value)
    }

    fn mutation_modifier_total(&self, value: impl Fn(&StatModifiers) -> i32) -> i32 {
        self.content
            .mutations()
            .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
            .fold(0, |total, mutation| {
                total.saturating_add(value(&mutation.modifiers))
            })
    }

    pub(super) fn refresh_character_skills(&mut self) {
        let skills =
            character_skill_progress(&self.content, self.build.as_ref(), self.progress.level)
                .expect("validated character skills must remain available");
        self.progress.replace_skills(skills);
    }

    pub(super) fn player_max_hp_at_level(&self, level: u16) -> i32 {
        self.character_base_max_hp_at_level(level)
            .saturating_add(self.character_modifier_total(|modifiers| modifiers.max_hp))
            .saturating_add(self.mutation_modifier_total(|modifiers| modifiers.max_hp))
            .saturating_add(self.equipment_modifiers().max_hp)
            .max(1)
    }

    pub(super) fn character_base_max_hp_at_level(&self, level: u16) -> i32 {
        character_base_max_hp_at_level(
            &self.progress.hp_progression,
            level,
            self.character_definitions(),
            i32::from(self.effective_player_attributes().constitution_hp_percent()),
        )
    }

    pub(super) fn apply_player_experience(&mut self, amount: u64, events: &mut Vec<DomainEvent>) {
        let amount = scale_experience_reward(amount, self.character_experience_percent());
        self.apply_unscaled_player_experience(amount, events);
    }

    pub(super) fn apply_unscaled_player_experience(
        &mut self,
        amount: u64,
        events: &mut Vec<DomainEvent>,
    ) {
        let previous_level = self.progress.level;
        let mut previous_max_hp = self.player_max_hp_at_level(previous_level);
        let ExperienceGainPlan {
            amount,
            progress,
            levels,
        } = plan_experience_gain(&self.progress, amount, self.victory_level_cap_unlocked());
        self.progress = progress;
        if !levels.is_empty() {
            self.refresh_character_skills();
            self.refresh_player_resource_maxima();
        }
        if amount > 0 {
            events.push(DomainEvent::ExperienceGained {
                amount,
                total: self.progress.experience,
            });
        }
        for level in levels {
            let max_hp = self.player_max_hp_at_level(level);
            if previous_max_hp > 0 {
                self.player.hp = rescale_i32(self.player.hp, previous_max_hp, max_hp);
            }
            previous_max_hp = max_hp;
            events.push(DomainEvent::PlayerLevelGained {
                level,
                max_hp,
                pending_attribute_increases: self.progress.pending_attribute_increases,
            });
        }
    }

    pub(super) fn apply_player_experience_drain(
        &mut self,
        amount: u64,
        source_kind_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> u64 {
        let before = self.progress.experience;
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let lost_levels = self.progress.lose_experience(amount);
        let drained = before.saturating_sub(self.progress.experience);
        if !lost_levels.is_empty() {
            self.refresh_character_skills();
            self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        }
        events.push(DomainEvent::ExperienceDrained {
            source_kind_id: source_kind_id.to_owned(),
            amount: drained,
            total: self.progress.experience,
        });
        for level in lost_levels {
            events.push(DomainEvent::PlayerLevelLost {
                level,
                max_hp: self.player_max_hp_at_level(level),
            });
        }
        drained
    }

    pub(super) fn increase_player_attribute(
        &mut self,
        attribute: AttributeKind,
    ) -> Option<(u16, u16, u8)> {
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let victorious = self.victory_level_cap_unlocked();
        let plan = plan_attribute_increase(&self.progress, attribute, victorious)?;
        self.progress.attributes = plan.attributes;
        self.progress.maximum_attributes = plan.maximum_attributes;
        self.progress.pending_attribute_increases = plan.pending_attribute_increases;
        self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        let effective = self.effective_player_attributes();
        Some((
            self.progress.attributes.value(attribute),
            effective.value(attribute),
            effective.index(attribute),
        ))
    }

    pub(super) fn player_resource_maxima(&self) -> BTreeMap<String, (u32, u32)> {
        self.resources
            .iter()
            .map(|(id, pool)| (id.clone(), (pool.current, pool.maximum)))
            .collect()
    }

    pub(super) fn refresh_after_attribute_change(
        &mut self,
        previous_max_hp: i32,
        previous_resource_maxima: &BTreeMap<String, (u32, u32)>,
    ) {
        let next_max_hp = self.effective_player_max_hp();
        if previous_max_hp > 0 && next_max_hp != previous_max_hp {
            self.player.hp = rescale_i32(self.player.hp, previous_max_hp, next_max_hp);
        }
        self.refresh_player_ability_state();
        for (resource_id, (previous_current, previous_maximum)) in previous_resource_maxima {
            let Some(pool) = self.resources.get_mut(resource_id) else {
                continue;
            };
            if *previous_maximum > 0 && pool.maximum != *previous_maximum {
                pool.current = rescale_u32(*previous_current, *previous_maximum, pool.maximum);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_appearance_masks_only_mutation_charisma_and_applies_its_level_floor() {
        let base = AttributeSet {
            charisma: 10,
            ..AttributeSet::default()
        };
        let character = StatModifiers {
            charisma: 1,
            ..StatModifiers::default()
        };
        let mutation = StatModifiers {
            strength: 1,
            charisma: -3,
            ..StatModifiers::default()
        };
        let equipment = StatModifiersDto {
            charisma: 1,
            ..StatModifiersDto::default()
        };
        let status = StatModifiersDto {
            charisma: -1,
            ..StatModifiersDto::default()
        };

        let attributes = effective_attributes(
            base,
            Some([
                &character,
                &StatModifiers::default(),
                &StatModifiers::default(),
            ]),
            [&mutation],
            equipment,
            [status],
            Some(18),
            118,
        );

        assert_eq!(
            attributes.strength, 14,
            "non-charisma mutation bonuses remain active"
        );
        assert_eq!(attributes.charisma, 18, "the level floor is applied last");

        let above_floor = effective_attributes(
            AttributeSet {
                charisma: 18,
                ..AttributeSet::default()
            },
            Some([
                &character,
                &StatModifiers::default(),
                &StatModifiers::default(),
            ]),
            [&mutation],
            equipment,
            [status],
            Some(8),
            118,
        );
        assert_eq!(
            above_floor.charisma, 28,
            "character, equipment, and status charisma remain effective"
        );
    }

    #[test]
    fn progression_capabilities_report_bounded_source_neutral_outcomes() {
        let mut progress = CharacterProgress::new(0, 10);
        progress.attributes.strength = 8;
        progress.maximum_attributes.strength = 13;
        let attribute = apply_attribute_restoration(&mut progress, AttributeKind::Strength);
        assert_eq!(attribute.before, 8);
        assert_eq!(attribute.after, 13);
        assert_eq!(attribute.maximum_before, 13);
        assert_eq!(attribute.maximum_after, 13);
        assert!(attribute.changed);

        progress.experience = 40;
        progress.maximum_experience = 100;
        assert_eq!(
            apply_experience_restoration(&mut progress),
            ExperienceRestorationOutcome {
                before: 40,
                after: 100,
            }
        );

        progress.life_force = 900;
        assert_eq!(
            apply_life_force_restoration(&mut progress, LifeForceRestorationRequest::add(150),),
            LifeForceRestorationOutcome {
                before: 900,
                after: 1_000,
            }
        );
        assert_eq!(
            apply_life_force_restoration(&mut progress, LifeForceRestorationRequest::at_least(700),),
            LifeForceRestorationOutcome {
                before: 1_000,
                after: 1_000,
            }
        );

        let mut bonus_capacity = 2;
        assert_eq!(
            apply_learning_capacity_increase(&mut bonus_capacity, false),
            LearningCapacityIncreaseOutcome {
                before: 2,
                after: 2,
            }
        );
        assert_eq!(
            apply_learning_capacity_increase(&mut bonus_capacity, true),
            LearningCapacityIncreaseOutcome {
                before: 2,
                after: 3,
            }
        );
    }
}
