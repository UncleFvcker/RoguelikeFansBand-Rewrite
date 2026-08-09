// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    AbilityBookDefinition, AbilityDefinition, ActorDefinition, BUILD_SCHEMA, CLASS_SCHEMA,
    CharacterBuildDefinition, ClassDefinition, ContentError, ItemDefinition, PERSONALITY_SCHEMA,
    PersonalityDefinition, RACE_SCHEMA, RaceDefinition, SKILL_SCHEMA, SKILL_SET_SCHEMA,
    SkillDefinition, SkillKind, SkillSetDefinition, StartingItemDefinition, StatModifiers,
    TerrainDefinition, valid_ability_level_scaling,
};

use super::shared::{
    attribute_modifiers_out_of_range, insert_definition_id, normalize_tags, require_format_version,
    require_reference, require_schema, validate_definition_id, validate_definition_text,
    validate_equipment_slot, validate_status_immunities,
};

pub(super) struct CharacterDefinitions<'a> {
    pub(super) skills: &'a mut [SkillDefinition],
    pub(super) skill_sets: &'a mut [SkillSetDefinition],
    pub(super) races: &'a mut [RaceDefinition],
    pub(super) classes: &'a mut [ClassDefinition],
    pub(super) personalities: &'a mut [PersonalityDefinition],
    pub(super) builds: &'a mut [CharacterBuildDefinition],
}

pub(super) struct CharacterValidationRefs<'a> {
    pub(super) items: &'a [ItemDefinition],
    pub(super) terrain: &'a [TerrainDefinition],
    pub(super) actors: &'a [ActorDefinition],
    pub(super) actor_tag_values: &'a BTreeSet<String>,
    pub(super) ability_race_ids: Vec<(String, String)>,
    pub(super) resource_ids: &'a BTreeSet<String>,
    pub(super) ability_book_ids: &'a BTreeSet<String>,
    pub(super) ability_books_by_id: &'a BTreeMap<String, AbilityBookDefinition>,
    pub(super) ability_resources: &'a BTreeMap<String, String>,
    pub(super) ability_ids: &'a BTreeSet<String>,
    pub(super) abilities: &'a [AbilityDefinition],
}

pub(super) fn validate_characters(
    definitions: CharacterDefinitions<'_>,
    refs: CharacterValidationRefs<'_>,
    all_ids: &mut BTreeSet<String>,
) -> Result<BTreeSet<String>, ContentError> {
    let CharacterValidationRefs {
        items,
        terrain,
        actors,
        actor_tag_values,
        ability_race_ids,
        resource_ids,
        ability_book_ids,
        ability_books_by_id,
        ability_resources,
        ability_ids,
        abilities,
    } = refs;
    let item_starting_metadata = items
        .iter()
        .map(|item| {
            (
                item.id.clone(),
                (item.max_stack, item.equipment_slot.clone()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut skill_maxima = BTreeMap::new();
    let mut skill_kinds = BTreeSet::new();
    for skill in definitions.skills.iter_mut() {
        require_schema(&skill.schema, SKILL_SCHEMA, &skill.id)?;
        require_format_version(skill.format_version, &skill.id)?;
        validate_definition_id(&skill.id, "skill")?;
        validate_definition_text(&skill.id, &skill.name_key, &skill.description_key)?;
        if !(1..=1_000_000).contains(&skill.maximum) || !skill_kinds.insert(skill.kind) {
            return Err(ContentError::InvalidSkill(skill.id.clone()));
        }
        normalize_tags(&skill.id, &mut skill.tags)?;
        insert_definition_id(all_ids, &skill.id)?;
        skill_maxima.insert(skill.id.clone(), skill.maximum);
    }
    for (required, kind, name) in [
        (
            items.iter().any(|item| {
                item.use_action
                    .as_ref()
                    .is_some_and(|action| action.device_check_difficulty.is_some())
            }),
            SkillKind::Device,
            "device",
        ),
        (
            terrain.iter().any(|terrain| {
                terrain
                    .trap
                    .as_ref()
                    .is_some_and(|trap| trap.saving_throw_difficulty.is_some())
            }),
            SkillKind::SavingThrow,
            "saving-throw",
        ),
        (
            actors.iter().any(|actor| actor.awareness.is_some()),
            SkillKind::Stealth,
            "stealth",
        ),
        (
            terrain
                .iter()
                .any(|terrain| terrain.perception_check_difficulty.is_some()),
            SkillKind::Perception,
            "perception",
        ),
    ] {
        if required && !skill_kinds.contains(&kind) {
            return Err(ContentError::MissingRequiredSkillKind(name.to_owned()));
        }
    }

    let mut skill_sets_by_id = BTreeMap::new();
    for skill_set in definitions.skill_sets.iter_mut() {
        require_schema(&skill_set.schema, SKILL_SET_SCHEMA, &skill_set.id)?;
        require_format_version(skill_set.format_version, &skill_set.id)?;
        validate_definition_id(&skill_set.id, "skill-set")?;
        skill_set
            .entries
            .sort_by(|left, right| left.skill_id.cmp(&right.skill_id));
        if skill_set.entries.len() > 64 {
            return Err(ContentError::InvalidSkillSet(skill_set.id.clone()));
        }
        let mut skill_ids = BTreeSet::new();
        for entry in &skill_set.entries {
            let Some(maximum) = skill_maxima.get(&entry.skill_id) else {
                return Err(ContentError::DanglingReference {
                    owner: skill_set.id.clone(),
                    target: entry.skill_id.clone(),
                });
            };
            if !skill_ids.insert(entry.skill_id.clone())
                || !(-1_000_000..=1_000_000).contains(&entry.base)
                || !(-1_000_000..=1_000_000).contains(&entry.growth_per_ten_levels)
                || entry.base > *maximum
            {
                return Err(ContentError::InvalidSkillSet(skill_set.id.clone()));
            }
        }
        insert_definition_id(all_ids, &skill_set.id)?;
        skill_sets_by_id.insert(skill_set.id.clone(), skill_set.clone());
    }

    let mut race_ids = BTreeSet::new();
    for race in definitions.races.iter_mut() {
        require_schema(&race.schema, RACE_SCHEMA, &race.id)?;
        require_format_version(race.format_version, &race.id)?;
        validate_definition_id(&race.id, "race")?;
        validate_definition_text(&race.id, &race.name_key, &race.description_key)?;
        if !(50..=200).contains(&race.shop_adjust_percent) {
            return Err(ContentError::InvalidCharacterSource(race.id.clone()));
        }
        validate_character_source(
            &race.id,
            CharacterSourceValidation {
                modifiers: &race.modifiers,
                life_percent: race.life_percent,
                experience_percent: race.experience_percent,
                base_hp: race.base_hp,
                skill_set_id: &race.skill_set_id,
                starting_items: &mut race.starting_items,
            },
            &skill_sets_by_id,
            &item_starting_metadata,
        )?;
        if race.body_slots.len() > 64 {
            return Err(ContentError::InvalidBodySlots(race.id.clone()));
        }
        validate_status_immunities(&race.id, &mut race.status_immunities)?;
        if let Some(category) = &race.kin_category
            && (category.is_empty()
                || category.len() > 64
                || !category.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'-' | b'_')
                })
                || !actor_tag_values.contains(category))
        {
            return Err(ContentError::InvalidCharacterSource(race.id.clone()));
        }
        let mut body_slot_ids = BTreeSet::new();
        for slot in &race.body_slots {
            if validate_equipment_slot(&slot.id).is_err()
                || validate_equipment_slot(&slot.slot_type).is_err()
                || !body_slot_ids.insert(slot.id.as_str())
            {
                return Err(ContentError::InvalidBodySlots(race.id.clone()));
            }
        }
        normalize_tags(&race.id, &mut race.tags)?;
        insert_definition_id(all_ids, &race.id)?;
        race_ids.insert(race.id.clone());
    }
    for (owner, race_id) in ability_race_ids {
        require_reference(&race_ids, &race_id, &owner)?;
    }

    let mut class_ids = BTreeSet::new();
    for class in definitions.classes.iter_mut() {
        require_schema(&class.schema, CLASS_SCHEMA, &class.id)?;
        require_format_version(class.format_version, &class.id)?;
        validate_definition_id(&class.id, "class")?;
        validate_definition_text(&class.id, &class.name_key, &class.description_key)?;
        validate_character_source(
            &class.id,
            CharacterSourceValidation {
                modifiers: &class.modifiers,
                life_percent: class.life_percent,
                experience_percent: class.experience_percent,
                base_hp: class.base_hp,
                skill_set_id: &class.skill_set_id,
                starting_items: &mut class.starting_items,
            },
            &skill_sets_by_id,
            &item_starting_metadata,
        )?;
        if let Some(profile) = &mut class.casting_profile {
            profile.ability_book_ids.sort();
            profile
                .ability_overrides
                .sort_by(|left, right| left.ability_id.cmp(&right.ability_id));
            let mut books = BTreeSet::new();
            let mut overrides = BTreeSet::new();
            let maximum_capacity = u64::from(profile.base_capacity)
                .saturating_add(u64::from(profile.capacity_per_level).saturating_mul(100))
                .saturating_add(
                    u64::from(profile.capacity_per_attribute_index).saturating_mul(100),
                );
            let maximum_learning_capacity = u64::from(profile.base_learning_capacity)
                .saturating_add(u64::from(profile.learning_capacity_per_level).saturating_mul(100))
                .saturating_add(
                    u64::from(profile.learning_capacity_per_attribute_index).saturating_mul(100),
                );
            if profile.minimum_failure_percent > 95
                || profile.beam_chance_level_divisor == 0
                || profile.beam_chance_level_multiplier > 4
                || !(-100..=100).contains(&profile.beam_chance_bonus)
                || profile.ability_book_ids.is_empty()
                || profile.ability_book_ids.len() > 16
                || profile
                    .ability_book_ids
                    .iter()
                    .any(|book_id| !books.insert(book_id.clone()))
                || profile.ability_overrides.len() > 1_024
                || profile.ability_overrides.iter().any(|override_| {
                    !overrides.insert(override_.ability_id.clone())
                        || !(1..=100).contains(&override_.minimum_level)
                        || !(1..=1_000_000).contains(&override_.resource_cost)
                        || override_.base_failure_percent > 95
                })
                || maximum_capacity == 0
                || maximum_capacity > 1_000_000_000
                || profile.learning_capacity_cap == 0
                || profile.base_learning_capacity > profile.learning_capacity_cap
                || maximum_learning_capacity > u64::from(u16::MAX)
            {
                return Err(ContentError::InvalidCastingProfile(class.id.clone()));
            }
            require_reference(resource_ids, &profile.resource_id, &class.id)?;
            let mut supported_ability_ids = BTreeSet::new();
            for book_id in &profile.ability_book_ids {
                require_reference(ability_book_ids, book_id, &class.id)?;
                let book = ability_books_by_id
                    .get(book_id)
                    .expect("validated ability book must remain available");
                if book.ability_ids.iter().any(|ability_id| {
                    ability_resources.get(ability_id) != Some(&profile.resource_id)
                }) {
                    return Err(ContentError::InvalidCastingProfile(class.id.clone()));
                }
                supported_ability_ids.extend(book.ability_ids.iter().cloned());
            }
            if profile
                .ability_overrides
                .iter()
                .any(|override_| !supported_ability_ids.contains(&override_.ability_id))
            {
                return Err(ContentError::InvalidCastingProfile(class.id.clone()));
            }
            for override_ in &profile.ability_overrides {
                if override_.level_scaling.is_empty() {
                    continue;
                }
                let ability = abilities
                    .iter()
                    .find(|ability| ability.id == override_.ability_id)
                    .expect("supported casting ability must remain available");
                if !valid_ability_level_scaling(&ability.effect, &override_.level_scaling) {
                    return Err(ContentError::InvalidCastingProfile(class.id.clone()));
                }
            }
        }
        let mut technique_resource_ids = class
            .casting_profile
            .as_ref()
            .map(|profile| profile.resource_id.clone())
            .into_iter()
            .collect::<BTreeSet<_>>();
        if class.technique_profiles.len() > 8 {
            return Err(ContentError::InvalidTechniqueProfile(class.id.clone()));
        }
        for profile in &mut class.technique_profiles {
            profile.innate_ability_ids.sort();
            let mut innate = BTreeSet::new();
            let maximum_capacity = u64::from(profile.base_capacity)
                .saturating_add(u64::from(profile.capacity_per_level).saturating_mul(100))
                .saturating_add(
                    u64::from(profile.capacity_per_attribute_index).saturating_mul(100),
                );
            if profile.minimum_failure_percent > 95
                || profile.innate_ability_ids.is_empty()
                || profile.innate_ability_ids.len() > 16
                || profile
                    .innate_ability_ids
                    .iter()
                    .any(|ability_id| !innate.insert(ability_id.clone()))
                || maximum_capacity == 0
                || maximum_capacity > 1_000_000_000
                || !technique_resource_ids.insert(profile.resource_id.clone())
            {
                return Err(ContentError::InvalidTechniqueProfile(class.id.clone()));
            }
            require_reference(resource_ids, &profile.resource_id, &class.id)?;
            for ability_id in &profile.innate_ability_ids {
                require_reference(ability_ids, ability_id, &class.id)?;
                if ability_resources.get(ability_id) != Some(&profile.resource_id) {
                    return Err(ContentError::InvalidTechniqueProfile(class.id.clone()));
                }
            }
        }
        if let Some(profile) = &class.device_recharge_profile {
            let maximum_capacity = u64::from(profile.base_capacity)
                .saturating_add(u64::from(profile.capacity_per_level).saturating_mul(100))
                .saturating_add(
                    u64::from(profile.capacity_per_attribute_index).saturating_mul(100),
                );
            if maximum_capacity == 0
                || maximum_capacity > 1_000_000_000
                || !(1..=u16::MAX).contains(&profile.power)
                || !(2..=u16::MAX).contains(&profile.source_item_destruction_one_in)
                || !technique_resource_ids.insert(profile.resource_id.clone())
            {
                return Err(ContentError::InvalidDeviceRechargeProfile(class.id.clone()));
            }
            require_reference(resource_ids, &profile.resource_id, &class.id)?;
        }
        normalize_tags(&class.id, &mut class.favorite_weapon_tags)?;
        normalize_tags(&class.id, &mut class.special_item_tags)?;
        class.icky_equipment_slots.sort();
        if class
            .icky_equipment_slots
            .iter()
            .any(|slot| validate_equipment_slot(slot).is_err())
            || class
                .icky_equipment_slots
                .windows(2)
                .any(|pair| pair[0] == pair[1])
        {
            return Err(ContentError::InvalidCharacterSource(class.id.clone()));
        }
        normalize_tags(&class.id, &mut class.tags)?;
        insert_definition_id(all_ids, &class.id)?;
        class_ids.insert(class.id.clone());
    }

    let mut personality_ids = BTreeSet::new();
    for personality in definitions.personalities.iter_mut() {
        require_schema(&personality.schema, PERSONALITY_SCHEMA, &personality.id)?;
        require_format_version(personality.format_version, &personality.id)?;
        validate_definition_id(&personality.id, "personality")?;
        validate_definition_text(
            &personality.id,
            &personality.name_key,
            &personality.description_key,
        )?;
        validate_character_source(
            &personality.id,
            CharacterSourceValidation {
                modifiers: &personality.modifiers,
                life_percent: personality.life_percent,
                experience_percent: personality.experience_percent,
                base_hp: personality.base_hp,
                skill_set_id: &personality.skill_set_id,
                starting_items: &mut personality.starting_items,
            },
            &skill_sets_by_id,
            &item_starting_metadata,
        )?;
        normalize_tags(&personality.id, &mut personality.tags)?;
        insert_definition_id(all_ids, &personality.id)?;
        personality_ids.insert(personality.id.clone());
    }

    let races_by_id = definitions
        .races
        .iter()
        .map(|definition| (definition.id.as_str(), definition))
        .collect::<BTreeMap<_, _>>();
    let classes_by_id = definitions
        .classes
        .iter()
        .map(|definition| (definition.id.as_str(), definition))
        .collect::<BTreeMap<_, _>>();
    let personalities_by_id = definitions
        .personalities
        .iter()
        .map(|definition| (definition.id.as_str(), definition))
        .collect::<BTreeMap<_, _>>();
    let mut build_ids = BTreeSet::new();
    for build in definitions.builds.iter_mut() {
        require_schema(&build.schema, BUILD_SCHEMA, &build.id)?;
        require_format_version(build.format_version, &build.id)?;
        validate_definition_id(&build.id, "build")?;
        validate_definition_text(&build.id, &build.name_key, &build.description_key)?;
        let Some(race) = races_by_id.get(build.race_id.as_str()) else {
            return Err(ContentError::DanglingReference {
                owner: build.id.clone(),
                target: build.race_id.clone(),
            });
        };
        let Some(class) = classes_by_id.get(build.class_id.as_str()) else {
            return Err(ContentError::DanglingReference {
                owner: build.id.clone(),
                target: build.class_id.clone(),
            });
        };
        let Some(personality) = personalities_by_id.get(build.personality_id.as_str()) else {
            return Err(ContentError::DanglingReference {
                owner: build.id.clone(),
                target: build.personality_id.clone(),
            });
        };
        if [
            build.attributes.strength,
            build.attributes.intelligence,
            build.attributes.wisdom,
            build.attributes.dexterity,
            build.attributes.constitution,
            build.attributes.charisma,
        ]
        .into_iter()
        .any(|value| !(3..=18).contains(&value))
        {
            return Err(ContentError::InvalidCharacterBuild(build.id.clone()));
        }
        if build.first_realm_id.as_deref().is_some_and(str::is_empty)
            || build.second_realm_id.as_deref().is_some_and(str::is_empty)
            || build.first_realm_id.is_some() && build.first_realm_id == build.second_realm_id
        {
            return Err(ContentError::InvalidCharacterBuild(build.id.clone()));
        }
        validate_starting_items(
            &build.id,
            &mut build.starting_items,
            &item_starting_metadata,
        )?;
        validate_combined_starting_items(
            &build.id,
            race.starting_items
                .iter()
                .chain(class.starting_items.iter())
                .chain(personality.starting_items.iter())
                .chain(build.starting_items.iter()),
            &item_starting_metadata,
        )?;
        normalize_tags(&build.id, &mut build.tags)?;
        insert_definition_id(all_ids, &build.id)?;
        build_ids.insert(build.id.clone());
    }
    Ok(build_ids)
}

struct CharacterSourceValidation<'a> {
    modifiers: &'a StatModifiers,
    life_percent: u16,
    experience_percent: u16,
    base_hp: i32,
    skill_set_id: &'a str,
    starting_items: &'a mut Vec<StartingItemDefinition>,
}

fn validate_character_source(
    owner_id: &str,
    source: CharacterSourceValidation<'_>,
    skill_sets: &BTreeMap<String, SkillSetDefinition>,
    item_metadata: &BTreeMap<String, (u32, Option<String>)>,
) -> Result<(), ContentError> {
    if source.modifiers.max_hp < -1_000_000
        || source.modifiers.max_hp > 1_000_000
        || source.modifiers.attack < -1_000_000
        || source.modifiers.attack > 1_000_000
        || source.modifiers.defense < -1_000_000
        || source.modifiers.defense > 1_000_000
        || !(-100..=100).contains(&source.modifiers.speed)
        || attribute_modifiers_out_of_range(source.modifiers)
        || !(25..=400).contains(&source.life_percent)
        || !(25..=500).contains(&source.experience_percent)
        || !(-1_000..=1_000).contains(&source.base_hp)
    {
        return Err(ContentError::InvalidCharacterSource(owner_id.to_owned()));
    }
    if !skill_sets.contains_key(source.skill_set_id) {
        return Err(ContentError::DanglingReference {
            owner: owner_id.to_owned(),
            target: source.skill_set_id.to_owned(),
        });
    }
    validate_starting_items(owner_id, source.starting_items, item_metadata)
}

fn validate_starting_items(
    owner_id: &str,
    starting_items: &mut Vec<StartingItemDefinition>,
    item_metadata: &BTreeMap<String, (u32, Option<String>)>,
) -> Result<(), ContentError> {
    starting_items.sort_by(|left, right| {
        left.item_kind_id
            .cmp(&right.item_kind_id)
            .then(left.equipped.cmp(&right.equipped))
    });
    if starting_items.len() > 32 {
        return Err(ContentError::InvalidStartingItems(owner_id.to_owned()));
    }
    let mut item_ids = BTreeSet::new();
    let mut equipment_slots = BTreeSet::new();
    for item in starting_items {
        let Some((max_stack, slot)) = item_metadata.get(&item.item_kind_id) else {
            return Err(ContentError::DanglingReference {
                owner: owner_id.to_owned(),
                target: item.item_kind_id.clone(),
            });
        };
        if item.quantity == 0
            || item.quantity > *max_stack
            || !item_ids.insert(item.item_kind_id.clone())
            || (item.equipped
                && (item.quantity != 1
                    || slot
                        .as_ref()
                        .is_none_or(|slot| !equipment_slots.insert(slot.clone()))))
        {
            return Err(ContentError::InvalidStartingItems(owner_id.to_owned()));
        }
    }
    Ok(())
}

fn validate_combined_starting_items<'a>(
    owner_id: &str,
    items: impl Iterator<Item = &'a StartingItemDefinition>,
    item_metadata: &BTreeMap<String, (u32, Option<String>)>,
) -> Result<(), ContentError> {
    let mut quantities = BTreeMap::<&str, u32>::new();
    let mut equipment_slots = BTreeSet::new();
    let mut count = 0_usize;
    for item in items {
        count += 1;
        let Some((max_stack, slot)) = item_metadata.get(&item.item_kind_id) else {
            return Err(ContentError::DanglingReference {
                owner: owner_id.to_owned(),
                target: item.item_kind_id.clone(),
            });
        };
        let quantity = quantities.entry(item.item_kind_id.as_str()).or_default();
        *quantity = quantity.saturating_add(item.quantity);
        if *quantity > *max_stack
            || (item.equipped
                && slot
                    .as_ref()
                    .is_none_or(|slot| !equipment_slots.insert(slot.clone())))
        {
            return Err(ContentError::InvalidCharacterBuild(owner_id.to_owned()));
        }
    }
    if count > 32 {
        return Err(ContentError::InvalidCharacterBuild(owner_id.to_owned()));
    }
    Ok(())
}
