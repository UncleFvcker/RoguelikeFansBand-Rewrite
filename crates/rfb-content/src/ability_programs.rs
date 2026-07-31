// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeMap;

#[cfg(feature = "schemas")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    ABILITY_PROGRAM_SCHEMA, AbilityCooldownDefinition, AbilityDefinition, AbilityEffectDefinition,
    AbilityGenocideScopeDefinition, AbilityLevelScalingDefinition, AbilityProficiencyDefinition,
    AbilityTargetDefinition, AbilityTargetModeDefinition, ContentError, require_format_version,
    require_schema, validate_definition_id,
};

pub type AbilityProgramStepDefinition = AbilityEffectDefinition;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum AbilityProgramInputDefinition {
    #[serde(rename = "self")]
    SelfTarget,
    CastTarget,
    Item,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AbilityProgramDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub input: AbilityProgramInputDefinition,
    pub steps: Vec<AbilityProgramStepDefinition>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[cfg_attr(feature = "schemas", schemars(title = "AbilityDefinition"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SourceAbilityDefinition {
    #[serde(rename = "$schema")]
    schema: String,
    format_version: u16,
    id: String,
    name_key: String,
    description_key: String,
    minimum_level: u16,
    resource_id: String,
    resource_cost: u32,
    base_failure_percent: u8,
    target: AbilityTargetDefinition,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    effect: Option<AbilityEffectDefinition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ability_program_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    level_scaling: Vec<AbilityLevelScalingDefinition>,
    #[serde(default)]
    proficiency: AbilityProficiencyDefinition,
    #[serde(default)]
    cooldown: Option<AbilityCooldownDefinition>,
    tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ResolvedAbilityProgram {
    input: AbilityProgramInputDefinition,
    effect: AbilityEffectDefinition,
}

pub(super) fn compile_ability_program_catalog(
    definitions: Vec<AbilityProgramDefinition>,
) -> Result<BTreeMap<String, ResolvedAbilityProgram>, ContentError> {
    let mut programs = BTreeMap::new();
    for definition in definitions {
        require_schema(&definition.schema, ABILITY_PROGRAM_SCHEMA, &definition.id)?;
        require_format_version(definition.format_version, &definition.id)?;
        validate_definition_id(&definition.id, "ability-program")?;
        if !(1..=8).contains(&definition.steps.len())
            || definition
                .steps
                .iter()
                .any(|step| !ability_program_input_accepts_step(definition.input, step))
            || (definition.steps.len() > 1
                && definition
                    .steps
                    .iter()
                    .any(|step| !ability_program_step_is_composable(definition.input, step)))
        {
            return Err(ContentError::InvalidAbilityProgram(definition.id));
        }

        let effect = if definition.steps.len() == 1 {
            definition
                .steps
                .into_iter()
                .next()
                .ok_or_else(|| ContentError::InvalidAbilityProgram(definition.id.clone()))?
        } else {
            AbilityEffectDefinition::Sequence {
                effects: definition.steps,
            }
        };
        let id = definition.id;
        if programs
            .insert(
                id.clone(),
                ResolvedAbilityProgram {
                    input: definition.input,
                    effect,
                },
            )
            .is_some()
        {
            return Err(ContentError::DuplicateDefinitionId(id));
        }
    }
    Ok(programs)
}

fn ability_program_input_accepts_step(
    input: AbilityProgramInputDefinition,
    effect: &AbilityEffectDefinition,
) -> bool {
    match input {
        AbilityProgramInputDefinition::SelfTarget => {
            matches!(
                effect,
                AbilityEffectDefinition::AreaDamage { .. }
                    | AbilityEffectDefinition::Summon { .. }
                    | AbilityEffectDefinition::SummonCategory { .. }
                    | AbilityEffectDefinition::Detect { .. }
                    | AbilityEffectDefinition::ApplyStatus { .. }
                    | AbilityEffectDefinition::RemoveStatus { .. }
                    | AbilityEffectDefinition::AnimateDead { .. }
                    | AbilityEffectDefinition::Heal { .. }
                    | AbilityEffectDefinition::RestoreVitality { .. }
                    | AbilityEffectDefinition::VisibleDamage { .. }
                    | AbilityEffectDefinition::VisibleApplyStatus { .. }
                    | AbilityEffectDefinition::EnchantEquippedWeapon { .. }
                    | AbilityEffectDefinition::BlinkSelf { .. }
                    | AbilityEffectDefinition::TeleportSelf { .. }
                    | AbilityEffectDefinition::NoOp { .. }
            ) || matches!(
                effect,
                AbilityEffectDefinition::Genocide {
                    scope: AbilityGenocideScopeDefinition::Nearby,
                    ..
                }
            )
        }
        AbilityProgramInputDefinition::CastTarget => {
            matches!(
                effect,
                AbilityEffectDefinition::Damage { .. }
                    | AbilityEffectDefinition::AreaDamage { .. }
                    | AbilityEffectDefinition::BeamDamage { .. }
                    | AbilityEffectDefinition::BoltOrBeamDamage { .. }
                    | AbilityEffectDefinition::ConeDamage { .. }
                    | AbilityEffectDefinition::BreathDamage { .. }
                    | AbilityEffectDefinition::CurseDamage { .. }
                    | AbilityEffectDefinition::DeathRay { .. }
                    | AbilityEffectDefinition::TeleportAway { .. }
                    | AbilityEffectDefinition::DrainResource { .. }
                    | AbilityEffectDefinition::Amnesia
                    | AbilityEffectDefinition::Teleport
                    | AbilityEffectDefinition::TransformTerrain { .. }
                    | AbilityEffectDefinition::ApplyStatus { .. }
                    | AbilityEffectDefinition::RemoveStatus { .. }
                    | AbilityEffectDefinition::Control { .. }
                    | AbilityEffectDefinition::DrainLife { .. }
                    | AbilityEffectDefinition::TeleportTarget
                    | AbilityEffectDefinition::NoOp { .. }
            ) || matches!(
                effect,
                AbilityEffectDefinition::Genocide {
                    scope: AbilityGenocideScopeDefinition::Single
                        | AbilityGenocideScopeDefinition::Glyph,
                    ..
                }
            )
        }
        AbilityProgramInputDefinition::Item => {
            matches!(effect, AbilityEffectDefinition::IdentifyItem { .. })
        }
    }
}

fn ability_program_step_is_composable(
    input: AbilityProgramInputDefinition,
    effect: &AbilityEffectDefinition,
) -> bool {
    match input {
        AbilityProgramInputDefinition::SelfTarget => matches!(
            effect,
            AbilityEffectDefinition::Heal { .. }
                | AbilityEffectDefinition::ApplyStatus { .. }
                | AbilityEffectDefinition::RemoveStatus { .. }
                | AbilityEffectDefinition::VisibleDamage { .. }
                | AbilityEffectDefinition::VisibleApplyStatus { .. }
                | AbilityEffectDefinition::NoOp { .. }
        ),
        AbilityProgramInputDefinition::CastTarget => matches!(
            effect,
            AbilityEffectDefinition::Damage { .. }
                | AbilityEffectDefinition::ApplyStatus { .. }
                | AbilityEffectDefinition::RemoveStatus { .. }
        ),
        AbilityProgramInputDefinition::Item => false,
    }
}

fn resolve_source_ability_program(
    owner_id: &str,
    ability_program_id: String,
    programs: &BTreeMap<String, ResolvedAbilityProgram>,
) -> Result<ResolvedAbilityProgram, ContentError> {
    validate_definition_id(&ability_program_id, "ability-program")?;
    programs
        .get(&ability_program_id)
        .cloned()
        .ok_or_else(|| ContentError::DanglingReference {
            owner: owner_id.to_owned(),
            target: ability_program_id,
        })
}

fn ability_program_input_matches_target(
    input: AbilityProgramInputDefinition,
    target: &AbilityTargetDefinition,
) -> bool {
    match input {
        AbilityProgramInputDefinition::SelfTarget => {
            target.modes.as_slice() == [AbilityTargetModeDefinition::SelfTarget]
                && target.range == 0
                && !target.requires_line_of_effect
        }
        AbilityProgramInputDefinition::CastTarget => {
            !target.modes.is_empty()
                && !target
                    .modes
                    .contains(&AbilityTargetModeDefinition::SelfTarget)
                && !target.modes.contains(&AbilityTargetModeDefinition::Item)
                && (1..=64).contains(&target.range)
                && target.requires_line_of_effect
        }
        AbilityProgramInputDefinition::Item => {
            target.modes.as_slice() == [AbilityTargetModeDefinition::Item]
                && target.range == 0
                && !target.requires_line_of_effect
        }
    }
}

impl SourceAbilityDefinition {
    pub(super) fn into_compiled(
        self,
        programs: &BTreeMap<String, ResolvedAbilityProgram>,
    ) -> Result<AbilityDefinition, ContentError> {
        let effect = match (self.effect, self.ability_program_id) {
            (Some(effect), None) => effect,
            (None, Some(program_id)) => {
                let program = resolve_source_ability_program(&self.id, program_id, programs)?;
                if !ability_program_input_matches_target(program.input, &self.target) {
                    return Err(ContentError::InvalidAbility(self.id));
                }
                program.effect
            }
            (Some(_), Some(_)) | (None, None) => {
                return Err(ContentError::InvalidAbility(self.id));
            }
        };
        Ok(AbilityDefinition {
            schema: self.schema,
            format_version: self.format_version,
            id: self.id,
            name_key: self.name_key,
            description_key: self.description_key,
            minimum_level: self.minimum_level,
            resource_id: self.resource_id,
            resource_cost: self.resource_cost,
            base_failure_percent: self.base_failure_percent,
            target: self.target,
            effect,
            level_scaling: self.level_scaling,
            proficiency: self.proficiency,
            cooldown: self.cooldown,
            tags: self.tags,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ABILITY_SCHEMA, ActorDamageType, CONTENT_FORMAT_VERSION};

    fn ability_program(
        id: &str,
        input: AbilityProgramInputDefinition,
        steps: Vec<AbilityEffectDefinition>,
    ) -> AbilityProgramDefinition {
        AbilityProgramDefinition {
            schema: ABILITY_PROGRAM_SCHEMA.to_owned(),
            format_version: CONTENT_FORMAT_VERSION,
            id: id.to_owned(),
            input,
            steps,
        }
    }

    fn source_ability(
        target: AbilityTargetDefinition,
        effect: Option<AbilityEffectDefinition>,
        ability_program_id: Option<&str>,
    ) -> SourceAbilityDefinition {
        SourceAbilityDefinition {
            schema: ABILITY_SCHEMA.to_owned(),
            format_version: CONTENT_FORMAT_VERSION,
            id: "demo.ability.test".to_owned(),
            name_key: "ability-demo-test-name".to_owned(),
            description_key: "ability-demo-test-description".to_owned(),
            minimum_level: 1,
            resource_id: "demo.resource.mana".to_owned(),
            resource_cost: 1,
            base_failure_percent: 0,
            target,
            effect,
            ability_program_id: ability_program_id.map(str::to_owned),
            level_scaling: Vec::new(),
            proficiency: AbilityProficiencyDefinition::default(),
            cooldown: None,
            tags: vec!["test".to_owned()],
        }
    }

    #[test]
    fn ability_program_catalog_requires_unique_flat_typed_programs() {
        let healing = ability_program(
            "demo.ability-program.healing",
            AbilityProgramInputDefinition::SelfTarget,
            vec![AbilityEffectDefinition::Heal { amount: 4 }],
        );
        let damage = ability_program(
            "demo.ability-program.damage",
            AbilityProgramInputDefinition::CastTarget,
            vec![AbilityEffectDefinition::Damage {
                damage_dice: 1,
                damage_sides: 4,
                damage_bonus: 0,
                damage_type: ActorDamageType::Physical,
            }],
        );
        let programs = compile_ability_program_catalog(vec![healing.clone(), damage])
            .expect("ability programs should compile");
        assert_eq!(
            programs.keys().map(String::as_str).collect::<Vec<_>>(),
            vec![
                "demo.ability-program.damage",
                "demo.ability-program.healing"
            ]
        );
        assert_eq!(
            programs.get("demo.ability-program.healing"),
            Some(&ResolvedAbilityProgram {
                input: AbilityProgramInputDefinition::SelfTarget,
                effect: AbilityEffectDefinition::Heal { amount: 4 },
            })
        );

        assert!(matches!(
            compile_ability_program_catalog(vec![healing.clone(), healing]),
            Err(ContentError::DuplicateDefinitionId(id))
                if id == "demo.ability-program.healing"
        ));
        assert!(matches!(
            compile_ability_program_catalog(vec![ability_program(
                "demo.ability-program.wrong-input",
                AbilityProgramInputDefinition::CastTarget,
                vec![AbilityEffectDefinition::Heal { amount: 4 }],
            )]),
            Err(ContentError::InvalidAbilityProgram(id))
                if id == "demo.ability-program.wrong-input"
        ));
        assert!(matches!(
            compile_ability_program_catalog(vec![ability_program(
                "demo.ability-program.nested",
                AbilityProgramInputDefinition::SelfTarget,
                vec![AbilityEffectDefinition::Sequence {
                    effects: vec![
                        AbilityEffectDefinition::Heal { amount: 1 },
                        AbilityEffectDefinition::Heal { amount: 1 },
                    ],
                }],
            )]),
            Err(ContentError::InvalidAbilityProgram(id))
                if id == "demo.ability-program.nested"
        ));
        assert!(matches!(
            compile_ability_program_catalog(vec![ability_program(
                "demo.ability-program.invalid-composition",
                AbilityProgramInputDefinition::SelfTarget,
                vec![
                    AbilityEffectDefinition::Summon {
                        actor_kind_id: "demo.actor.test".to_owned(),
                        count: 1,
                        radius: 1,
                        duration_turns: 1,
                        hostile: false,
                    },
                    AbilityEffectDefinition::Heal { amount: 1 },
                ],
            )]),
            Err(ContentError::InvalidAbilityProgram(id))
                if id == "demo.ability-program.invalid-composition"
        ));
    }

    #[test]
    fn ability_program_bindings_lower_without_changing_runtime_definitions() {
        let programs = compile_ability_program_catalog(vec![ability_program(
            "demo.ability-program.healing",
            AbilityProgramInputDefinition::SelfTarget,
            vec![AbilityEffectDefinition::Heal { amount: 4 }],
        )])
        .expect("ability program should compile");
        let self_target = AbilityTargetDefinition {
            modes: vec![AbilityTargetModeDefinition::SelfTarget],
            range: 0,
            requires_line_of_effect: false,
        };

        let referenced_source = source_ability(
            self_target.clone(),
            None,
            Some("demo.ability-program.healing"),
        );
        let referenced_json = serde_json::to_value(referenced_source)
            .expect("referenced source ability should serialize");
        assert!(referenced_json.get("effect").is_none());
        let referenced = serde_json::from_value::<SourceAbilityDefinition>(referenced_json)
            .expect("referenced source ability should deserialize")
            .into_compiled(&programs)
            .expect("ability program reference should lower");
        assert_eq!(
            referenced.effect,
            AbilityEffectDefinition::Heal { amount: 4 }
        );

        let inline = source_ability(
            self_target.clone(),
            Some(AbilityEffectDefinition::Heal { amount: 4 }),
            None,
        )
        .into_compiled(&programs)
        .expect("inline compatibility path should lower");
        assert_eq!(referenced, inline);

        assert!(matches!(
            source_ability(
                self_target.clone(),
                Some(AbilityEffectDefinition::Heal { amount: 4 }),
                Some("demo.ability-program.healing"),
            )
            .into_compiled(&programs),
            Err(ContentError::InvalidAbility(id)) if id == "demo.ability.test"
        ));
        assert!(matches!(
            source_ability(self_target.clone(), None, None).into_compiled(&programs),
            Err(ContentError::InvalidAbility(id)) if id == "demo.ability.test"
        ));
        assert!(matches!(
            source_ability(
                self_target.clone(),
                None,
                Some("demo.ability-program.missing"),
            )
            .into_compiled(&programs),
            Err(ContentError::DanglingReference { owner, target })
                if owner == "demo.ability.test"
                    && target == "demo.ability-program.missing"
        ));

        let cast_target = AbilityTargetDefinition {
            modes: vec![AbilityTargetModeDefinition::Entity],
            range: 6,
            requires_line_of_effect: true,
        };
        assert!(matches!(
            source_ability(
                cast_target,
                None,
                Some("demo.ability-program.healing"),
            )
            .into_compiled(&programs),
            Err(ContentError::InvalidAbility(id)) if id == "demo.ability.test"
        ));
    }
}
