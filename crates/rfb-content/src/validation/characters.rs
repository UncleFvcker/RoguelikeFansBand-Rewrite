// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    AbilityBookDefinition, AbilityDefinition, ActorDefinition, ActorRole, BUILD_SCHEMA,
    CLASS_SCHEMA, CharacterBuildDefinition, ClassDefinition, ContentError,
    InnatePowerCostScalingCurveDefinition, ItemDefinition, LevelResistanceDefinition,
    MutationDefinition, PERSONALITY_SCHEMA, PersonalityDefinition, RACE_SCHEMA, RaceDefinition,
    RaceMutationSelectionDefinition, SKILL_SCHEMA, SKILL_SET_SCHEMA, SkillDefinition, SkillKind,
    SkillSetDefinition, StartingItemDefinition, StatModifiers, TerrainDefinition,
    valid_ability_level_scaling,
};

use super::shared::{
    attribute_modifiers_out_of_range, insert_definition_id, normalize_tags, require_format_version,
    require_reference, require_schema, validate_definition_id, validate_definition_text,
    validate_equipment_slot, validate_message_key, validate_status_immunities,
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
    pub(super) abilities: &'a [AbilityDefinition],
    pub(super) mutations: &'a [MutationDefinition],
}

fn level_resistances_are_valid(entries: &mut Vec<LevelResistanceDefinition>) -> bool {
    entries.sort_by_key(|entry| entry.minimum_level);
    entries.len() <= 32
        && entries.iter().all(|entry| {
            (1..=100).contains(&entry.minimum_level)
                && !entry.resistances.is_empty()
                && entry.resistances.len() <= 32
        })
        && entries
            .windows(2)
            .all(|pair| pair[0].minimum_level != pair[1].minimum_level)
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
        abilities,
        mutations,
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
    let weapon_proficiency_base_item_ids = items
        .iter()
        .filter(|item| {
            item.weapon_proficiency_base_item_id.is_none()
                && (item.melee_profile.is_some() || item.projectile_profile.is_some())
        })
        .map(|item| item.id.as_str())
        .collect::<BTreeSet<_>>();
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
                item.device_generation.is_some()
                    || item
                        .use_action
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
    let mut legacy_race_indices = BTreeSet::new();
    let mutation_random_weights = mutations
        .iter()
        .map(|mutation| (mutation.id.as_str(), mutation.random_weight))
        .collect::<BTreeMap<_, _>>();
    let unavailable_race_ability_ids = ability_books_by_id
        .values()
        .flat_map(|book| book.ability_ids.iter())
        .chain(
            mutations
                .iter()
                .filter_map(|mutation| mutation.activation.as_ref())
                .map(|activation| &activation.ability_id),
        )
        .chain(
            definitions
                .classes
                .iter()
                .flat_map(|class| class.abilities.iter())
                .map(|activation| &activation.ability_id),
        )
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for race in definitions.races.iter_mut() {
        require_schema(&race.schema, RACE_SCHEMA, &race.id)?;
        require_format_version(race.format_version, &race.id)?;
        validate_definition_id(&race.id, "race")?;
        validate_definition_text(&race.id, &race.name_key, &race.description_key)?;
        if !(50..=200).contains(&race.shop_adjust_percent)
            || !(0..=64).contains(&race.infravision)
            || !(-1_000..=1_000).contains(&race.regeneration_rate_modifier_percent)
            || !(-100..=100).contains(&race.speed_per_ten_levels)
            || !level_resistances_are_valid(&mut race.level_resistances)
        {
            return Err(ContentError::InvalidCharacterSource(race.id.clone()));
        }
        if race
            .legacy_index
            .is_some_and(|index| !legacy_race_indices.insert(index))
        {
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
        validate_race_level_mutation_rewards(race, &mutation_random_weights)?;
        let mut race_ability_ids = BTreeSet::new();
        if race.abilities.iter().any(|activation| {
            !race_ability_ids.insert(activation.ability_id.as_str())
                || !(1..=100).contains(&activation.minimum_level)
                || activation.cost > 1_000_000
                || activation.cost_scaling.is_some_and(|scaling| {
                    !(1..=100).contains(&scaling.start_level)
                        || scaling.level_interval == 0
                        || scaling.level_interval > 100
                        || scaling.amount == 0
                        || scaling.amount > 1_000_000
                        || scaling.divisor == 0
                        || scaling.divisor > 1_000_000
                        || match scaling.curve {
                            InnatePowerCostScalingCurveDefinition::Step => {
                                scaling.divisor != 1
                                    || scaling.round_up
                                    || scaling.linear_weight != 1
                                    || scaling.quadratic_weight != 0
                                    || scaling.cubic_weight != 0
                            }
                            InnatePowerCostScalingCurveDefinition::Prorated => {
                                scaling.start_level != 1
                                    || scaling.level_interval != 1
                                    || !(1..=100).contains(&scaling.linear_weight)
                                    || scaling.quadratic_weight > 100
                                    || scaling.cubic_weight > 100
                            }
                        }
                })
                || activation.base_failure_percent > 95
                || activation
                    .minimum_failure_percent
                    .is_some_and(|minimum| minimum > activation.base_failure_percent)
                || unavailable_race_ability_ids.contains(activation.ability_id.as_str())
                || abilities
                    .iter()
                    .find(|ability| ability.id == activation.ability_id)
                    .is_none_or(|ability| ability.player.is_some())
        }) {
            return Err(ContentError::InvalidCharacterSource(race.id.clone()));
        }
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
        if let Some(profile) = &class.weapon_proficiency {
            let valid_bounds = |initial: u16, maximum: u16| initial <= maximum && maximum <= 8_000;
            if !valid_bounds(profile.default_initial, profile.default_maximum)
                || profile.overrides.len() > 1_024
                || profile.overrides.iter().any(|(item_id, bounds)| {
                    !weapon_proficiency_base_item_ids.contains(item_id.as_str())
                        || !valid_bounds(bounds.initial, bounds.maximum)
                })
            {
                return Err(ContentError::InvalidWeaponProficiency(class.id.clone()));
            }
        }
        if class.riding_proficiency.initial > class.riding_proficiency.maximum
            || class.riding_proficiency.maximum > 8_000
            || class
                .mounted_non_arrow_base_shot_cap
                .is_some_and(|cap| cap == 0)
        {
            return Err(ContentError::InvalidRidingProficiency(class.id.clone()));
        }
        if class.sniping_profile.is_some_and(|profile| {
            !(-100..=100).contains(&profile.preferred_ammunition_to_hit_base)
                || profile.preferred_ammunition_to_hit_level_divisor == 0
                || !(1..=100).contains(&profile.base_shot_excess_percent)
                || !(100..=500).contains(&profile.preferred_ammunition_critical_chance_percent)
                || profile.base_concentration_maximum == 0
                || profile.concentration_level_divisor == 0
                || profile.concentration_bonus_percent_per_level == 0
                || profile.concentration_bonus_percent_per_level > 20
                || u16::from(profile.base_concentration_maximum).saturating_add(
                    (50_u16.saturating_add(profile.concentration_level_offset))
                        / profile.concentration_level_divisor,
                ) > 10
        }) {
            return Err(ContentError::InvalidCharacterSource(class.id.clone()));
        }
        if let Some(profile) = &mut class.casting_profile {
            profile
                .realm_profiles
                .sort_by(|left, right| left.realm_id.cmp(&right.realm_id));
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
                || profile.spell_damage_bonus_level_divisor == 0
                || !(1..=400).contains(&profile.capacity_percent)
                || !(1..=400).contains(&profile.resource_recovery_percent)
                || profile.realm_profiles.is_empty()
                || profile.realm_profiles.len() > 16
                || (profile.capacity_formula == crate::CastingCapacityFormula::Linear
                    && maximum_capacity == 0)
                || maximum_capacity > 1_000_000_000
                || profile.learning_capacity_cap == 0
                || profile.base_learning_capacity > profile.learning_capacity_cap
                || maximum_learning_capacity > u64::from(u16::MAX)
                || profile.encumbrance.as_ref().is_some_and(|encumbrance| {
                    encumbrance.maximum_weight_tenths_pound > 1_000_000
                        || encumbrance.weapon_weight_percent > 1_000
                        || encumbrance.penalty_weight_tenths_pound == 0
                })
            {
                return Err(ContentError::InvalidCastingProfile(class.id.clone()));
            }
            require_reference(resource_ids, &profile.resource_id, &class.id)?;
            let mut realm_ids = BTreeSet::new();
            for realm in &mut profile.realm_profiles {
                realm.ability_book_ids.sort();
                realm
                    .ability_overrides
                    .sort_by(|left, right| left.ability_id.cmp(&right.ability_id));
                let valid_realm_id = !realm.realm_id.is_empty()
                    && realm.realm_id.len() <= 64
                    && realm.realm_id.bytes().all(|byte| {
                        byte.is_ascii_lowercase()
                            || byte.is_ascii_digit()
                            || matches!(byte, b'-' | b'_')
                    });
                let mut books = BTreeSet::new();
                let mut overrides = BTreeSet::new();
                if !valid_realm_id
                    || !realm_ids.insert(realm.realm_id.clone())
                    || realm.ability_book_ids.is_empty()
                    || realm.ability_book_ids.len() > 16
                    || realm
                        .ability_book_ids
                        .iter()
                        .any(|book_id| !books.insert(book_id.clone()))
                    || realm.ability_overrides.len() > 1_024
                    || realm.ability_overrides.iter().any(|override_| {
                        !overrides.insert(override_.ability_id.clone())
                            || !(1..=100).contains(&override_.minimum_level)
                            || !(1..=1_000_000).contains(&override_.resource_cost)
                            || override_.base_failure_percent > 95
                    })
                {
                    return Err(ContentError::InvalidCastingProfile(class.id.clone()));
                }
                let mut supported_ability_ids = BTreeSet::new();
                for book_id in &realm.ability_book_ids {
                    require_reference(ability_book_ids, book_id, &class.id)?;
                    let book = ability_books_by_id
                        .get(book_id)
                        .expect("validated ability book must remain available");
                    if book.realm_id.as_deref() != Some(realm.realm_id.as_str())
                        || book.ability_ids.iter().any(|ability_id| {
                            ability_resources.get(ability_id) != Some(&profile.resource_id)
                        })
                    {
                        return Err(ContentError::InvalidCastingProfile(class.id.clone()));
                    }
                    supported_ability_ids.extend(book.ability_ids.iter().cloned());
                }
                if realm
                    .ability_overrides
                    .iter()
                    .any(|override_| !supported_ability_ids.contains(&override_.ability_id))
                {
                    return Err(ContentError::InvalidCastingProfile(class.id.clone()));
                }
                for override_ in &realm.ability_overrides {
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
        }
        class
            .abilities
            .sort_by(|left, right| left.ability_id.cmp(&right.ability_id));
        if !(-100..=100).contains(&class.ammunition_breakage_factor_modifier)
            || class.projectile_critical_chance_bonus_percent_per_level > 10
            || !level_resistances_are_valid(&mut class.level_resistances)
            || class.pet_upkeep_divisor == 0
        {
            return Err(ContentError::InvalidCharacterSource(class.id.clone()));
        }
        let mut class_ability_ids = BTreeSet::new();
        for activation in &class.abilities {
            if !class_ability_ids.insert(activation.ability_id.clone())
                || !(1..=100).contains(&activation.minimum_level)
                || activation.resource_cost > 1_000_000
                || activation.minimum_concentration > 10
                || activation.hit_point_cost > 1_000_000
                || activation.base_failure_percent > 95
                || activation.minimum_failure_percent > 95
                || (activation.resource_id.is_none() && activation.resource_cost != 0)
                || (activation.governing_attribute.is_none()
                    && activation.base_failure_percent != 0)
                || abilities
                    .iter()
                    .find(|ability| ability.id == activation.ability_id)
                    .is_none_or(|ability| {
                        ability.player.is_some()
                            || (matches!(
                                ability.effect,
                                crate::AbilityEffectDefinition::Concentrate
                                    | crate::AbilityEffectDefinition::SniperShot { .. }
                                    | crate::AbilityEffectDefinition::ProbeMonsters
                            ) && class.sniping_profile.is_none())
                    })
                || (activation.minimum_concentration != 0 && class.sniping_profile.is_none())
            {
                return Err(ContentError::InvalidCharacterSource(class.id.clone()));
            }
            if let Some(key) = &activation.ui_group_name_key {
                validate_message_key(key)
                    .map_err(|_| ContentError::InvalidCharacterSource(class.id.clone()))?;
            }
            if let Some(resource_id) = &activation.resource_id {
                require_reference(resource_ids, resource_id, &class.id)?;
            }
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
        let selected_realms = [
            build.first_realm_id.as_deref(),
            build.second_realm_id.as_deref(),
        ];
        match &class.casting_profile {
            Some(profile)
                if build.first_realm_id.is_none()
                    || selected_realms.into_iter().flatten().any(|realm_id| {
                        !profile
                            .realm_profiles
                            .iter()
                            .any(|profile| profile.realm_id == realm_id)
                    }) =>
            {
                return Err(ContentError::InvalidCharacterBuild(build.id.clone()));
            }
            None if build.first_realm_id.is_some() || build.second_realm_id.is_some() => {
                return Err(ContentError::InvalidCharacterBuild(build.id.clone()));
            }
            _ => {}
        }
        if let Some(actor_id) = &build.player_actor_id
            && actors
                .iter()
                .find(|actor| actor.id == *actor_id)
                .is_none_or(|actor| actor.role != ActorRole::Player)
        {
            return Err(ContentError::DanglingReference {
                owner: build.id.clone(),
                target: actor_id.clone(),
            });
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

fn validate_race_level_mutation_rewards(
    race: &mut RaceDefinition,
    mutation_random_weights: &BTreeMap<&str, u8>,
) -> Result<(), ContentError> {
    race.level_mutation_rewards.sort_by(|left, right| {
        (left.minimum_level, &left.id).cmp(&(right.minimum_level, &right.id))
    });
    let mut reward_ids = BTreeSet::new();
    let mut claimed_mutation_ids = BTreeSet::new();
    for reward in &race.level_mutation_rewards {
        let valid_reward_id = !reward.id.is_empty()
            && reward.id.len() <= 64
            && reward.id.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
            });
        if reward.minimum_level == 0 || !valid_reward_id || !reward_ids.insert(reward.id.as_str()) {
            return Err(ContentError::InvalidCharacterSource(race.id.clone()));
        }
        let mutation_ids = match &reward.selection {
            RaceMutationSelectionDefinition::Choice { mutation_ids } => {
                if mutation_ids.is_empty() {
                    return Err(ContentError::InvalidCharacterSource(race.id.clone()));
                }
                mutation_ids.iter().map(String::as_str).collect::<Vec<_>>()
            }
            RaceMutationSelectionDefinition::CastingAttribute {
                default_mutation_id,
                mutation_ids_by_attribute,
            } => std::iter::once(default_mutation_id.as_str())
                .chain(mutation_ids_by_attribute.values().map(String::as_str))
                .collect(),
        };
        for mutation_id in mutation_ids {
            let Some(random_weight) = mutation_random_weights.get(mutation_id) else {
                return Err(ContentError::DanglingReference {
                    owner: race.id.clone(),
                    target: mutation_id.to_owned(),
                });
            };
            if *random_weight != 0 || !claimed_mutation_ids.insert(mutation_id) {
                return Err(ContentError::InvalidCharacterSource(race.id.clone()));
            }
        }
    }
    Ok(())
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
        let maximum_quantity = item.maximum_quantity.unwrap_or(item.quantity);
        if item.quantity == 0
            || maximum_quantity < item.quantity
            || maximum_quantity > *max_stack
            || !item_ids.insert(item.item_kind_id.clone())
            || (item.equipped
                && (item.quantity != 1
                    || maximum_quantity != 1
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
        *quantity = quantity.saturating_add(item.maximum_quantity.unwrap_or(item.quantity));
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
