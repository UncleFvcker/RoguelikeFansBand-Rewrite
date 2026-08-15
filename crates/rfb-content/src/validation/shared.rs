// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

use super::*;

pub(crate) fn validate_definition_id(id: &str, category: &str) -> Result<(), ContentError> {
    validate_id(id)?;
    if id.split('.').nth(1) != Some(category) {
        return Err(ContentError::WrongIdCategory {
            id: id.to_owned(),
            expected: category.to_owned(),
        });
    }
    Ok(())
}

pub(crate) fn validate_id(id: &str) -> Result<(), ContentError> {
    if id.is_empty()
        || id.len() > 128
        || id.split('.').count() < 3
        || id.split('.').any(str::is_empty)
        || !id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
    {
        return Err(ContentError::InvalidStableId(id.to_owned()));
    }
    Ok(())
}

pub(crate) fn validate_semver(version: &str) -> Result<(), ContentError> {
    if version.is_empty() || version.len() > 64 || !version.is_ascii() {
        return Err(ContentError::InvalidPackVersion(version.to_owned()));
    }
    let (core_and_prerelease, build) = version
        .split_once('+')
        .map_or((version, None), |(core, build)| (core, Some(build)));
    if version.matches('+').count() > 1
        || build.is_some_and(|value| !valid_semver_identifiers(value, false))
    {
        return Err(ContentError::InvalidPackVersion(version.to_owned()));
    }
    let (core, prerelease) = core_and_prerelease
        .split_once('-')
        .map_or((core_and_prerelease, None), |(core, prerelease)| {
            (core, Some(prerelease))
        });
    if prerelease.is_some_and(|value| !valid_semver_identifiers(value, true)) {
        return Err(ContentError::InvalidPackVersion(version.to_owned()));
    }
    let parts = core.split('.').collect::<Vec<_>>();
    if parts.len() != 3
        || parts.iter().any(|part| {
            part.is_empty()
                || !part.bytes().all(|byte| byte.is_ascii_digit())
                || (part.len() > 1 && part.starts_with('0'))
        })
    {
        return Err(ContentError::InvalidPackVersion(version.to_owned()));
    }
    Ok(())
}

fn valid_semver_identifiers(value: &str, reject_numeric_leading_zero: bool) -> bool {
    !value.is_empty()
        && value.split('.').all(|identifier| {
            !identifier.is_empty()
                && identifier
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && !(reject_numeric_leading_zero
                    && identifier.len() > 1
                    && identifier.starts_with('0')
                    && identifier.bytes().all(|byte| byte.is_ascii_digit()))
        })
}

pub(crate) fn validate_pack_relations(
    pack_id: &str,
    dependencies: &[PackDependency],
    load_after_entries: &[String],
) -> Result<(), ContentError> {
    let mut dependency_ids = BTreeSet::new();
    for dependency in dependencies {
        validate_id(&dependency.id)?;
        if dependency.id == pack_id || !dependency_ids.insert(&dependency.id) {
            return Err(ContentError::InvalidDependency(dependency.id.clone()));
        }
        if dependency.version_requirement.trim().is_empty()
            || dependency.version_requirement.len() > 64
        {
            return Err(ContentError::InvalidVersionRequirement(
                dependency.version_requirement.clone(),
            ));
        }
    }
    let mut load_after = BTreeSet::new();
    for id in load_after_entries {
        validate_id(id)?;
        if id == pack_id || !load_after.insert(id) {
            return Err(ContentError::InvalidLoadAfter(id.clone()));
        }
    }
    Ok(())
}

pub(crate) fn validate_message_key(key: &str) -> Result<(), ContentError> {
    if key.is_empty()
        || key.len() > 128
        || !key.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
    {
        return Err(ContentError::InvalidMessageKey(key.to_owned()));
    }
    Ok(())
}

/// Status immunity lists carry engine status kind ids: normalized to a
/// sorted, unique list with a small size budget.
pub(super) fn validate_status_immunities(
    owner_id: &str,
    immunities: &mut Vec<String>,
) -> Result<(), ContentError> {
    immunities.sort();
    immunities.dedup();
    if immunities.len() > 16
        || immunities.iter().any(|kind_id| {
            kind_id.is_empty()
                || kind_id.len() > 64
                || !kind_id.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'-' | b'.')
                })
        })
    {
        return Err(ContentError::InvalidStatusImmunities(owner_id.to_owned()));
    }
    Ok(())
}

pub(super) fn validate_equipment_slot(slot: &str) -> Result<(), ContentError> {
    if slot.is_empty()
        || slot.len() > 64
        || !slot.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
    {
        return Err(ContentError::InvalidEquipmentSlot(slot.to_owned()));
    }
    Ok(())
}

pub(super) fn attribute_modifiers_out_of_range(modifiers: &StatModifiers) -> bool {
    [
        modifiers.strength,
        modifiers.intelligence,
        modifiers.wisdom,
        modifiers.dexterity,
        modifiers.constitution,
        modifiers.charisma,
        modifiers.spell_power_bonus,
        modifiers.device_power_bonus,
    ]
    .into_iter()
    .any(|value| !(-100..=100).contains(&value))
}

pub(super) fn equipment_bonuses_out_of_range(bonuses: &EquipmentBonuses) -> bool {
    !(-100..=100).contains(&bonuses.life_percent)
        || !(-1_000..=1_000).contains(&bonuses.launcher_multiplier_delta_percent)
        || !(-1_000..=1_000).contains(&bonuses.base_shot_delta_percent)
        || !(-8..=8).contains(&bonuses.melee_attacks)
        || [
            bonuses.melee_skill,
            bonuses.melee_damage,
            bonuses.ranged_skill,
            bonuses.throwing_skill,
            bonuses.device_skill,
            bonuses.saving_throw_skill,
            bonuses.stealth_skill,
            bonuses.search_skill,
            bonuses.perception_skill,
            bonuses.disarming_skill,
            bonuses.digging_skill,
        ]
        .into_iter()
        .any(|value| !(-1_000_000..=1_000_000).contains(&value))
        || bonuses
            .saving_throw_skill_override
            .is_some_and(|value| !(0..=1_000_000).contains(&value))
        || !(-64..=64).contains(&bonuses.infravision)
        || !(-8..=8).contains(&bonuses.light_radius)
}

pub(super) fn affix_property_bundle_out_of_range(bundle: &AffixPropertyBundleDefinition) -> bool {
    bundle.modifiers.max_hp < -1_000_000
        || bundle.modifiers.max_hp > 1_000_000
        || bundle.modifiers.attack < -1_000_000
        || bundle.modifiers.attack > 1_000_000
        || bundle.modifiers.defense < -1_000_000
        || bundle.modifiers.defense > 1_000_000
        || !(-100..=100).contains(&bundle.modifiers.speed)
        || attribute_modifiers_out_of_range(&bundle.modifiers)
        || equipment_bonuses_out_of_range(&bundle.equipment_bonuses)
}

pub(super) fn validate_definition_text(
    id: &str,
    name_key: &str,
    description_key: &str,
) -> Result<(), ContentError> {
    validate_message_key(name_key)
        .map_err(|_| ContentError::InvalidDefinitionText(id.to_owned()))?;
    validate_message_key(description_key)
        .map_err(|_| ContentError::InvalidDefinitionText(id.to_owned()))?;
    Ok(())
}

pub(super) fn validate_glyph(id: &str, glyph: &str) -> Result<(), ContentError> {
    let mut characters = glyph.chars();
    if characters.next().is_none_or(char::is_control) || characters.next().is_some() {
        return Err(ContentError::InvalidGlyph(id.to_owned()));
    }
    Ok(())
}

pub(super) fn normalize_tags(id: &str, tags: &mut [String]) -> Result<(), ContentError> {
    for tag in tags.iter() {
        if tag.is_empty()
            || tag.len() > 64
            || !tag.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
            })
        {
            return Err(ContentError::InvalidTag {
                id: id.to_owned(),
                tag: tag.clone(),
            });
        }
    }
    tags.sort();
    if tags.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ContentError::DuplicateTag(id.to_owned()));
    }
    Ok(())
}

pub(super) fn insert_definition_id(
    ids: &mut BTreeSet<String>,
    id: &str,
) -> Result<(), ContentError> {
    if !ids.insert(id.to_owned()) {
        return Err(ContentError::DuplicateDefinitionId(id.to_owned()));
    }
    Ok(())
}

pub(crate) fn require_schema(
    actual: &str,
    expected: &str,
    owner: &str,
) -> Result<(), ContentError> {
    if actual != expected {
        return Err(ContentError::SchemaMismatch(owner.to_owned()));
    }
    Ok(())
}

pub(crate) fn require_format_version(actual: u16, owner: &str) -> Result<(), ContentError> {
    if actual != CONTENT_FORMAT_VERSION {
        return Err(ContentError::UnsupportedSourceVersion {
            owner: owner.to_owned(),
            version: actual,
        });
    }
    Ok(())
}

pub(super) fn require_reference(
    ids: &BTreeSet<String>,
    target: &str,
    owner: &str,
) -> Result<(), ContentError> {
    if !ids.contains(target) {
        return Err(ContentError::DanglingReference {
            owner: owner.to_owned(),
            target: target.to_owned(),
        });
    }
    Ok(())
}

pub(super) fn require_actor_role(
    roles: &BTreeMap<String, ActorRole>,
    target: &str,
    expected: ActorRole,
    owner: &str,
) -> Result<(), ContentError> {
    match roles.get(target) {
        Some(actual) if *actual == expected => Ok(()),
        Some(_) => Err(ContentError::WrongActorRole(target.to_owned())),
        None => Err(ContentError::DanglingReference {
            owner: owner.to_owned(),
            target: target.to_owned(),
        }),
    }
}

pub(super) fn validate_position(
    position: ContentPosition,
    width: u16,
    height: u16,
    owner: &str,
) -> Result<(), ContentError> {
    if position.x >= width || position.y >= height {
        return Err(ContentError::PositionOutOfBounds(owner.to_owned()));
    }
    Ok(())
}
