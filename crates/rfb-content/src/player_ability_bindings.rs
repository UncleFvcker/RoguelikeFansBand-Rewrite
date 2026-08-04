// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "schemas")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    AbilityCooldownDefinition, AbilityDefinition, AbilityProficiencyDefinition, ContentError,
    PLAYER_ABILITY_BINDING_SCHEMA, PlayerAbilityDefinition,
};
use crate::validation::{
    require_format_version, require_schema, validate_definition_id, validate_id,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlayerAbilityBindingDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub ability_id: String,
    pub minimum_level: u16,
    pub resource_id: String,
    pub resource_cost: u32,
    pub base_failure_percent: u8,
    #[serde(default)]
    pub proficiency: AbilityProficiencyDefinition,
    #[serde(default)]
    pub cooldown: Option<AbilityCooldownDefinition>,
}

pub(super) type ResolvedPlayerAbilityBinding = PlayerAbilityDefinition;

pub(super) fn compile_player_ability_binding_catalog(
    definitions: Vec<PlayerAbilityBindingDefinition>,
) -> Result<BTreeMap<String, ResolvedPlayerAbilityBinding>, ContentError> {
    let mut bindings = BTreeMap::new();
    for definition in definitions {
        require_schema(
            &definition.schema,
            PLAYER_ABILITY_BINDING_SCHEMA,
            &definition.ability_id,
        )?;
        require_format_version(definition.format_version, &definition.ability_id)?;
        validate_definition_id(&definition.ability_id, "ability")?;
        validate_definition_id(&definition.resource_id, "resource")?;
        if !valid_player_ability_binding(&definition) {
            return Err(ContentError::InvalidPlayerAbilityBinding(
                definition.ability_id,
            ));
        }

        let ability_id = definition.ability_id;
        let binding = PlayerAbilityDefinition {
            minimum_level: definition.minimum_level,
            resource_id: definition.resource_id,
            resource_cost: definition.resource_cost,
            base_failure_percent: definition.base_failure_percent,
            proficiency: definition.proficiency,
            cooldown: definition.cooldown,
        };
        if bindings.insert(ability_id.clone(), binding).is_some() {
            return Err(ContentError::DuplicatePlayerAbilityBinding(ability_id));
        }
    }
    Ok(bindings)
}

pub(super) fn validate_player_ability_binding_references(
    bindings: &BTreeMap<String, ResolvedPlayerAbilityBinding>,
    abilities: &[AbilityDefinition],
) -> Result<(), ContentError> {
    let ability_ids = abilities
        .iter()
        .map(|ability| ability.id.as_str())
        .collect::<BTreeSet<_>>();
    if let Some(ability_id) = bindings
        .keys()
        .find(|ability_id| !ability_ids.contains(ability_id.as_str()))
    {
        return Err(ContentError::InvalidPlayerAbilityBinding(
            ability_id.clone(),
        ));
    }
    Ok(())
}

fn valid_player_ability_binding(definition: &PlayerAbilityBindingDefinition) -> bool {
    (1..=100).contains(&definition.minimum_level)
        && (1..=1_000_000).contains(&definition.resource_cost)
        && definition.base_failure_percent <= 95
        && definition.proficiency.initial <= definition.proficiency.cap
        && definition.proficiency.cap <= 1600
        && definition
            .proficiency
            .success_gain
            .saturating_add(definition.proficiency.failure_gain)
            <= 10_000
        && definition
            .cooldown
            .as_ref()
            .is_none_or(|cooldown| cooldown.turns > 0)
        && definition
            .cooldown
            .as_ref()
            .and_then(|cooldown| cooldown.group_id.as_deref())
            .is_none_or(|group_id| validate_id(group_id).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CONTENT_FORMAT_VERSION;

    fn binding(ability_id: &str) -> PlayerAbilityBindingDefinition {
        PlayerAbilityBindingDefinition {
            schema: PLAYER_ABILITY_BINDING_SCHEMA.to_owned(),
            format_version: CONTENT_FORMAT_VERSION,
            ability_id: ability_id.to_owned(),
            minimum_level: 3,
            resource_id: "demo.resource.mana".to_owned(),
            resource_cost: 5,
            base_failure_percent: 20,
            proficiency: AbilityProficiencyDefinition::default(),
            cooldown: None,
        }
    }

    #[test]
    fn player_ability_binding_catalog_is_unique_and_bounded() {
        let first = binding("demo.ability.first");
        let second = binding("demo.ability.second");
        let bindings = compile_player_ability_binding_catalog(vec![second, first.clone()])
            .expect("valid player bindings should compile");
        assert_eq!(
            bindings.keys().map(String::as_str).collect::<Vec<_>>(),
            vec!["demo.ability.first", "demo.ability.second"]
        );

        assert!(matches!(
            compile_player_ability_binding_catalog(vec![first.clone(), first]),
            Err(ContentError::DuplicatePlayerAbilityBinding(id))
                if id == "demo.ability.first"
        ));

        let mut invalid = binding("demo.ability.invalid");
        invalid.minimum_level = 0;
        assert!(matches!(
            compile_player_ability_binding_catalog(vec![invalid]),
            Err(ContentError::InvalidPlayerAbilityBinding(id))
                if id == "demo.ability.invalid"
        ));
    }
}
