// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeMap;

use rfb_content::{
    CharacterBuildDefinition, ClassDefinition, ContentCatalog, PersonalityDefinition,
    RaceDefinition, RaceMutationSelectionDefinition, SkillSetDefinition, StatModifiers,
};
use rfb_protocol::{
    AttributeBreakdownDto, AttributeKindDto, AttributeSourceDto, AttributeSourceKindDto,
    ItemIdentificationDto, ItemKnowledgeDto, StatModifiersDto,
};

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

use super::{Game, ItemLocation, player_stats::apply_equipment_life_percent, stat_modifiers_dto};

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
    below_eighteen_threshold: u64,
    rng: &mut RfbRng,
) -> AttributeMutationOutcome {
    let before = progress.attributes.value(attribute);
    let maximum_before = progress.maximum_attributes.value(attribute);
    let changed = progress.increase_attribute_permanently(
        attribute,
        victorious,
        below_eighteen_threshold,
        rng,
    );
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
pub(super) struct LifeForceChangeOutcome {
    pub(super) before: u16,
    pub(super) after: u16,
}

pub(super) fn apply_life_force_restoration(
    progress: &mut CharacterProgress,
    request: LifeForceRestorationRequest,
) -> LifeForceChangeOutcome {
    let before = progress.life_force;
    progress.life_force = match request.restoration {
        LifeForceRestoration::Add(amount) => progress.life_force.saturating_add(amount).min(1_000),
        LifeForceRestoration::AtLeast(minimum) => progress.life_force.max(minimum).min(1_000),
    };
    LifeForceChangeOutcome {
        before,
        after: progress.life_force,
    }
}

pub(super) fn apply_life_force_drain(
    progress: &mut CharacterProgress,
    amount: u16,
) -> LifeForceChangeOutcome {
    let before = progress.life_force;
    progress.life_force = progress.life_force.saturating_sub(amount);
    LifeForceChangeOutcome {
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
    race_id: Option<&str>,
) -> Result<Option<CharacterBuildIdentity>, CoreError> {
    let Some(build_id) = build_id else {
        return Ok(None);
    };
    let build = content
        .build(build_id)
        .ok_or_else(|| CoreError::UnknownCharacterBuild(build_id.to_owned()))?;
    let race_id = race_id.unwrap_or(&build.race_id);
    let race = content
        .race(race_id)
        .ok_or_else(|| CoreError::UnknownCharacterRace(race_id.to_owned()))?;
    if !race.tags.iter().any(|tag| tag == "rfb-compatibility") {
        return Err(CoreError::CharacterRaceUnavailable(race_id.to_owned()));
    }
    Ok(Some(CharacterBuildIdentity {
        build_id: build.id.clone(),
        race_id: race.id.clone(),
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

struct AttributeStep<'a> {
    kind: AttributeSourceKindDto,
    source_id: Option<&'a str>,
    name_key: Option<&'a str>,
    modifiers: StatModifiersDto,
}

fn effective_attributes<'a>(
    mut attributes: AttributeSet,
    steps: impl IntoIterator<Item = AttributeStep<'a>>,
    normal_appearance_minimum: Option<u16>,
    cap: u16,
    mut observe: impl FnMut(&AttributeStep<'a>, AttributeSet, AttributeSet),
) -> AttributeSet {
    for step in steps {
        let before = attributes;
        attributes = apply_attribute_dto_modifiers(attributes, step.modifiers, cap);
        if step.kind == AttributeSourceKindDto::Mutation && normal_appearance_minimum.is_some() {
            attributes.charisma = before.charisma;
        }
        observe(&step, before, attributes);
    }
    if let Some(minimum) = normal_appearance_minimum {
        let before = attributes;
        attributes.charisma = attributes.charisma.max(minimum.min(cap));
        observe(
            &AttributeStep {
                kind: AttributeSourceKindDto::NormalAppearance,
                source_id: None,
                name_key: None,
                modifiers: StatModifiersDto::default(),
            },
            before,
            attributes,
        );
    }
    attributes
}

fn character_experience_percent(definitions: Option<CharacterDefinitions<'_>>) -> u16 {
    definitions.map_or(100, |(_, race, class, personality)| {
        let factor = u64::from(race.experience_percent) * u64::from(class.experience_percent) / 100;
        u16::try_from(factor * u64::from(personality.experience_percent) / 100).unwrap_or(u16::MAX)
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

pub(super) const fn initial_resource_pool(maximum: u32) -> ResourcePool {
    ResourcePool {
        current: maximum,
        maximum,
    }
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
    pub(super) fn restore_player_attribute(
        &mut self,
        attribute: AttributeKind,
    ) -> AttributeMutationOutcome {
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let outcome = apply_attribute_restoration(&mut self.progress, attribute);
        if outcome.changed {
            self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        }
        outcome
    }

    pub(super) fn restore_player_life_force(
        &mut self,
        request: LifeForceRestorationRequest,
    ) -> LifeForceChangeOutcome {
        let previous_max_hp = self.effective_player_max_hp();
        let outcome = apply_life_force_restoration(&mut self.progress, request);
        self.rescale_player_hp_after_life_force_change(previous_max_hp);
        outcome
    }

    pub(super) fn drain_player_life_force(&mut self, amount: u16) -> LifeForceChangeOutcome {
        let previous_max_hp = self.effective_player_max_hp();
        let outcome = apply_life_force_drain(&mut self.progress, amount);
        self.rescale_player_hp_after_life_force_change(previous_max_hp);
        outcome
    }

    fn rescale_player_hp_after_life_force_change(&mut self, previous_max_hp: i32) {
        let next_max_hp = self.effective_player_max_hp();
        if previous_max_hp > 0 && next_max_hp != previous_max_hp {
            self.player.hp = rescale_i32(self.player.hp, previous_max_hp, next_max_hp);
        }
    }

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
        self.player_attributes_with_sources(None)
    }

    pub(super) fn player_attributes_with_sources(
        &self,
        breakdown: Option<&mut Vec<AttributeBreakdownDto>>,
    ) -> AttributeSet {
        self.player_attributes_at_level(self.progress.level, breakdown)
    }

    fn player_attributes_at_level(
        &self,
        level: u16,
        mut breakdown: Option<&mut Vec<AttributeBreakdownDto>>,
    ) -> AttributeSet {
        let cap = CharacterProgress::attribute_cap(self.victory_level_cap_unlocked());
        let active_mutations = self
            .content
            .mutations()
            .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
            .collect::<Vec<_>>();
        let normal_appearance_minimum = active_mutations
            .iter()
            .any(|mutation| mutation.normal_appearance)
            .then(|| 8_u16.saturating_add(level.saturating_mul(2)));
        let mut steps = Vec::new();
        let headgear_excess = self.player_tomte_headgear_excess_weight();
        if let Some((_, race, class, personality)) = self.character_definitions() {
            for (kind, id, name, modifiers) in [
                (
                    AttributeSourceKindDto::Race,
                    &race.id,
                    &race.name_key,
                    &race.modifiers,
                ),
                (
                    AttributeSourceKindDto::Class,
                    &class.id,
                    &class.name_key,
                    &class.modifiers,
                ),
                (
                    AttributeSourceKindDto::Personality,
                    &personality.id,
                    &personality.name_key,
                    &personality.modifiers,
                ),
            ] {
                let mut modifiers = stat_modifiers_dto(modifiers);
                if kind == AttributeSourceKindDto::Race && headgear_excess > 0 {
                    modifiers.intelligence -= i32::from(headgear_excess / 10) + 1;
                }
                if kind == AttributeSourceKindDto::Race && race.id == "rfb-legacy.race.ent" {
                    // RFB master a0d92b6378: races_a.c::ent_get_race.
                    let growth = [26, 41, 46]
                        .into_iter()
                        .map(|threshold| i32::from(level >= threshold))
                        .sum::<i32>();
                    modifiers.strength += growth;
                    modifiers.constitution += growth;
                    modifiers.dexterity -= growth;
                }
                steps.push(AttributeStep {
                    kind,
                    source_id: Some(id),
                    name_key: Some(name),
                    modifiers,
                });
            }
        }
        steps.extend(active_mutations.iter().map(|mutation| AttributeStep {
            kind: AttributeSourceKindDto::Mutation,
            source_id: Some(&mutation.id),
            name_key: None,
            modifiers: stat_modifiers_dto(&mutation.modifiers),
        }));
        steps.push(AttributeStep {
            kind: AttributeSourceKindDto::Equipment,
            source_id: None,
            name_key: None,
            modifiers: self.equipment_modifiers(),
        });
        steps.extend(self.player.statuses.iter().map(|status| {
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
            AttributeStep {
                kind: AttributeSourceKindDto::TemporaryEffect,
                source_id: Some(&status.kind_id),
                name_key: None,
                modifiers,
            }
        }));
        let kinds = [
            (AttributeKind::Strength, AttributeKindDto::Strength),
            (AttributeKind::Intelligence, AttributeKindDto::Intelligence),
            (AttributeKind::Wisdom, AttributeKindDto::Wisdom),
            (AttributeKind::Dexterity, AttributeKindDto::Dexterity),
            (AttributeKind::Constitution, AttributeKindDto::Constitution),
            (AttributeKind::Charisma, AttributeKindDto::Charisma),
        ];
        let mut equipment_complete = true;
        let mut known_equipment = StatModifiersDto::default();
        if let Some(rows) = breakdown.as_deref_mut() {
            *rows = kinds
                .iter()
                .map(|(kind, dto)| AttributeBreakdownDto {
                    attribute: *dto,
                    natural: self.progress.attributes.value(*kind),
                    effective: 0, // Filled from the same calculation below, never serialized as a placeholder.
                    minimum: 3,
                    maximum: cap,
                    normal_appearance_minimum: (*kind == AttributeKind::Charisma)
                        .then_some(normal_appearance_minimum.map(|minimum| minimum.min(cap)))
                        .flatten(),
                    sources: Vec::new(),
                })
                .collect();
            for item in self.items.iter().filter(|item| {
                matches!(&item.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) != Some("tool"))
            }) {
                let visible = self.visible_item_modifiers(item);
                known_equipment.strength = known_equipment.strength.saturating_add(visible.strength);
                known_equipment.intelligence = known_equipment.intelligence.saturating_add(visible.intelligence);
                known_equipment.wisdom = known_equipment.wisdom.saturating_add(visible.wisdom);
                known_equipment.dexterity = known_equipment.dexterity.saturating_add(visible.dexterity);
                known_equipment.constitution = known_equipment.constitution.saturating_add(visible.constitution);
                known_equipment.charisma = known_equipment.charisma.saturating_add(visible.charisma);
                equipment_complete &= self.item_knowledge_dto(&item.kind_id) == ItemKnowledgeDto::Aware
                    && self.item_identification(item) == ItemIdentificationDto::Identified;
            }
        }
        let mut results_known = true;
        let effective = effective_attributes(
            self.progress.attributes,
            steps,
            normal_appearance_minimum,
            cap,
            |step, before, after| {
                let Some(rows) = breakdown.as_deref_mut() else {
                    return;
                };
                let equipment = step.kind == AttributeSourceKindDto::Equipment;
                let complete = !equipment || equipment_complete;
                results_known &= complete;
                let modifiers = if equipment {
                    known_equipment
                } else {
                    step.modifiers
                };
                let modifiers = [
                    modifiers.strength,
                    modifiers.intelligence,
                    modifiers.wisdom,
                    modifiers.dexterity,
                    modifiers.constitution,
                    modifiers.charisma,
                ];
                for ((row, (kind, _)), modifier) in rows.iter_mut().zip(kinds).zip(modifiers) {
                    if step.kind == AttributeSourceKindDto::NormalAppearance
                        && kind != AttributeKind::Charisma
                    {
                        continue;
                    }
                    let suppressed = kind == AttributeKind::Charisma
                        && step.kind == AttributeSourceKindDto::Mutation
                        && normal_appearance_minimum.is_some();
                    row.sources.push(AttributeSourceDto {
                        kind: step.kind,
                        source_id: step.source_id.map(str::to_owned),
                        name_key: step.name_key.map(str::to_owned),
                        modifier,
                        complete,
                        effective_after: results_known.then_some(after.value(kind)),
                        upper_limit_applied: results_known.then(|| {
                            !suppressed
                                && step.kind != AttributeSourceKindDto::NormalAppearance
                                && modify_attribute_value(before.value(kind), modifier, u16::MAX)
                                    != after.value(kind)
                        }),
                        suppressed,
                    });
                }
            },
        );
        if let Some(rows) = breakdown {
            for (row, (kind, _)) in rows.iter_mut().zip(kinds) {
                row.effective = effective.value(kind);
            }
        }
        effective
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

    pub(super) fn character_experience_percent(&self) -> u16 {
        // Ordinary temporary forms do not change calc_exp_factor's true-race input.
        let definitions = self.build.as_ref().map(|identity| {
            build_definitions(&self.content, identity)
                .expect("validated build must remain available")
        });
        character_experience_percent(definitions)
    }

    pub(super) fn experience_required_for_level(&self, level: u16) -> u64 {
        crate::stats::experience_required_for_level_with_factor(
            level,
            self.character_experience_percent(),
        )
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
        apply_equipment_life_percent(
            self.character_base_max_hp_at_level(level)
                .saturating_add(self.character_modifier_total(|modifiers| modifiers.max_hp))
                .saturating_add(self.mutation_modifier_total(|modifiers| modifiers.max_hp))
                .saturating_add(self.equipment_modifiers().max_hp),
            self.player_equipment_life_percent(),
        )
    }

    pub(super) fn character_base_max_hp_at_level(&self, level: u16) -> i32 {
        character_base_max_hp_at_level(
            &self.progress.hp_progression,
            level,
            self.character_definitions(),
            i32::from(
                self.player_attributes_at_level(level, None)
                    .constitution_hp_percent(),
            ),
        )
    }

    pub(super) fn apply_player_experience(&mut self, amount: u64, events: &mut Vec<DomainEvent>) {
        if self.player.hp < 0 {
            return;
        }
        let previous_level = self.progress.level;
        let previous_max_level = self.progress.max_level;
        let mut previous_max_hp = self.player_max_hp_at_level(previous_level);
        let levels = self.progress.gain_experience(
            amount,
            self.character_experience_percent(),
            self.victory_level_cap_unlocked(),
        );
        let newly_reached_levels = levels
            .iter()
            .filter(|level| **level > previous_max_level)
            .count();
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
            if level < previous_level {
                events.push(DomainEvent::PlayerLevelLost { level, max_hp });
            } else {
                events.push(DomainEvent::PlayerLevelGained {
                    level,
                    max_hp,
                    pending_attribute_increases: self.progress.pending_attribute_increases,
                    reached_new_maximum: level > previous_max_level,
                });
            }
        }
        self.apply_birth_race_level_mutation_rolls(newly_reached_levels, events);
        self.apply_race_level_mutation_rewards(events);
    }

    pub(super) fn pending_race_mutation_choice(&self) -> Option<(String, Vec<String>)> {
        let race = self.selected_race_definition()?;
        let excluded = self
            .character_definitions()
            .and_then(|(_, _, class, _)| race.mutation_choice_exclusions_by_class.get(&class.id));
        race.level_mutation_rewards
            .iter()
            .filter(|reward| reward.minimum_level <= self.progress.level)
            .filter(|reward| !self.race_mutation_reward_completed(reward))
            .find_map(|reward| match &reward.selection {
                RaceMutationSelectionDefinition::Choice { mutation_ids } => Some((
                    reward.id.clone(),
                    mutation_ids
                        .iter()
                        .filter(|mutation_id| {
                            excluded.is_none_or(|excluded| !excluded.contains(*mutation_id))
                        })
                        .cloned()
                        .collect(),
                )),
                RaceMutationSelectionDefinition::CastingAttribute { .. } => None,
            })
    }

    pub(super) fn choose_race_mutation(
        &mut self,
        reward_id: &str,
        mutation_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let Some((pending_reward_id, candidates)) = self.pending_race_mutation_choice() else {
            return false;
        };
        if pending_reward_id != reward_id
            || !candidates.iter().any(|candidate| candidate == mutation_id)
        {
            return false;
        }
        self.gain_and_lock_race_mutation(mutation_id, events)
    }

    fn apply_race_level_mutation_rewards(&mut self, events: &mut Vec<DomainEvent>) {
        let Some(race) = self.selected_race_definition() else {
            return;
        };
        let rewards = race.level_mutation_rewards.clone();
        let casting_attribute = self
            .character_definitions()
            .and_then(|(_, _, class, _)| class.casting_profile.as_ref())
            .map(|profile| profile.casting_attribute);
        for reward in rewards {
            if reward.minimum_level > self.progress.level
                || self.race_mutation_reward_completed(&reward)
            {
                continue;
            }
            let RaceMutationSelectionDefinition::CastingAttribute {
                default_mutation_id,
                mutation_ids_by_attribute,
            } = reward.selection
            else {
                continue;
            };
            let mutation_id = casting_attribute
                .and_then(|attribute| mutation_ids_by_attribute.get(&attribute))
                .unwrap_or(&default_mutation_id);
            self.gain_and_lock_race_mutation(mutation_id, events);
        }
    }

    pub(super) fn selected_race_definition(&self) -> Option<&rfb_content::RaceDefinition> {
        self.build
            .as_ref()
            .and_then(|identity| self.content.race(&identity.race_id))
    }

    pub(super) fn birth_race_mutation_override(
        &self,
        mutation_id: &str,
    ) -> Option<&rfb_content::RaceMutationOverrideDefinition> {
        self.selected_race_definition()?
            .mutation_overrides
            .get(mutation_id)
    }

    fn race_mutation_reward_completed(
        &self,
        reward: &rfb_content::RaceLevelMutationRewardDefinition,
    ) -> bool {
        match &reward.selection {
            RaceMutationSelectionDefinition::Choice { mutation_ids } => mutation_ids
                .iter()
                .any(|id| self.progress.locked_mutation_ids.contains(id)),
            RaceMutationSelectionDefinition::CastingAttribute {
                default_mutation_id,
                mutation_ids_by_attribute,
            } => std::iter::once(default_mutation_id)
                .chain(mutation_ids_by_attribute.values())
                .any(|id| self.progress.locked_mutation_ids.contains(id)),
        }
    }

    fn gain_and_lock_race_mutation(
        &mut self,
        mutation_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        if !self.progress.active_mutation_ids.contains(mutation_id)
            && !self.gain_mutation(mutation_id, events)
        {
            return false;
        }
        self.progress
            .locked_mutation_ids
            .insert(mutation_id.to_owned());
        true
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
        let lost_levels = self.progress.lose_experience(
            amount,
            self.character_experience_percent(),
            self.victory_level_cap_unlocked(),
        );
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
    fn experience_factor_uses_original_sequential_truncation() {
        let game = Game::new(83);
        let (build, race, class, personality) = game.character_definitions().unwrap();
        let mut race = race.clone();
        let mut class = class.clone();
        let mut personality = personality.clone();
        race.experience_percent = 111;
        class.experience_percent = 111;
        personality.experience_percent = 111;
        // floor(floor(111 * 111 / 100) * 111 / 100), not a rounded product.
        assert_eq!(
            character_experience_percent(Some((build, &race, &class, &personality))),
            136
        );
    }

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
        let steps = || {
            [
                (AttributeSourceKindDto::Race, stat_modifiers_dto(&character)),
                (AttributeSourceKindDto::Class, StatModifiersDto::default()),
                (
                    AttributeSourceKindDto::Personality,
                    StatModifiersDto::default(),
                ),
                (
                    AttributeSourceKindDto::Mutation,
                    stat_modifiers_dto(&mutation),
                ),
                (AttributeSourceKindDto::Equipment, equipment),
                (AttributeSourceKindDto::TemporaryEffect, status),
            ]
            .map(|(kind, modifiers)| AttributeStep {
                kind,
                source_id: None,
                name_key: None,
                modifiers,
            })
        };

        let attributes = effective_attributes(base, steps(), Some(18), 118, |_, _, _| {});

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
            steps(),
            Some(8),
            118,
            |_, _, _| {},
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
            LifeForceChangeOutcome {
                before: 900,
                after: 1_000,
            }
        );
        assert_eq!(
            apply_life_force_restoration(&mut progress, LifeForceRestorationRequest::at_least(700),),
            LifeForceChangeOutcome {
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
