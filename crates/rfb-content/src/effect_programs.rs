// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "schemas")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    AbilityTargetDefinition, AbilityTargetModeDefinition, CompiledContentV1, ContentError,
    EFFECT_PROGRAM_SCHEMA, ItemUseEffectDefinition,
};
use crate::validation::{
    require_format_version, require_schema, valid_item_effect, validate_definition_id,
};

pub type EffectProgramStepDefinition = ItemUseEffectDefinition;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum EffectProgramInputDefinition {
    #[serde(rename = "self")]
    SelfTarget,
    Actor,
    Area,
    Item,
    Glyph,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectProgramDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub input: EffectProgramInputDefinition,
    pub steps: Vec<EffectProgramStepDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ResolvedEffectProgram {
    input: EffectProgramInputDefinition,
    effect: ItemUseEffectDefinition,
}

pub(super) fn compile_effect_program_catalog(
    definitions: Vec<EffectProgramDefinition>,
) -> Result<BTreeMap<String, ResolvedEffectProgram>, ContentError> {
    let mut programs = BTreeMap::new();
    for definition in definitions {
        require_schema(&definition.schema, EFFECT_PROGRAM_SCHEMA, &definition.id)?;
        require_format_version(definition.format_version, &definition.id)?;
        validate_definition_id(&definition.id, "effect")?;
        if !(1..=12).contains(&definition.steps.len())
            || definition
                .steps
                .iter()
                .any(|step| !effect_program_step_accepts_input(step, definition.input))
        {
            return Err(ContentError::InvalidEffectProgram(definition.id));
        }

        let effect = if definition.steps.len() == 1 {
            definition
                .steps
                .into_iter()
                .next()
                .ok_or_else(|| ContentError::InvalidEffectProgram(definition.id.clone()))?
        } else {
            ItemUseEffectDefinition::Sequence {
                effects: definition.steps,
            }
        };
        let id = definition.id;
        if programs
            .insert(
                id.clone(),
                ResolvedEffectProgram {
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

fn effect_program_input_for_step(
    effect: &ItemUseEffectDefinition,
) -> Option<EffectProgramInputDefinition> {
    match effect {
        ItemUseEffectDefinition::Sequence { .. } => None,
        ItemUseEffectDefinition::Damage { .. }
        | ItemUseEffectDefinition::BeamDamage { .. }
        | ItemUseEffectDefinition::RandomElementConeDamage { .. } => {
            Some(EffectProgramInputDefinition::Actor)
        }
        ItemUseEffectDefinition::IdentifyItem { .. }
        | ItemUseEffectDefinition::EnchantItem { .. }
        | ItemUseEffectDefinition::MundanifyItem
        | ItemUseEffectDefinition::CraftItem { .. }
        | ItemUseEffectDefinition::RechargeFromDevice { .. } => {
            Some(EffectProgramInputDefinition::Item)
        }
        ItemUseEffectDefinition::Genocide { .. } => Some(EffectProgramInputDefinition::Glyph),
        _ => Some(EffectProgramInputDefinition::SelfTarget),
    }
}

fn effect_program_step_accepts_input(
    effect: &ItemUseEffectDefinition,
    input: EffectProgramInputDefinition,
) -> bool {
    if input == EffectProgramInputDefinition::Area {
        return matches!(
            effect,
            ItemUseEffectDefinition::Damage { .. }
                | ItemUseEffectDefinition::Heal { .. }
                | ItemUseEffectDefinition::HealDice { .. }
        );
    }
    effect_program_input_for_step(effect) == Some(input)
}

pub(super) fn resolve_source_item_effect(
    owner_id: &str,
    effect_program_id: String,
    programs: &BTreeMap<String, ResolvedEffectProgram>,
) -> Result<(ItemUseEffectDefinition, EffectProgramInputDefinition), ContentError> {
    validate_definition_id(&effect_program_id, "effect")?;
    programs
        .get(&effect_program_id)
        .map(|program| (program.effect.clone(), program.input))
        .ok_or_else(|| ContentError::DanglingReference {
            owner: owner_id.to_owned(),
            target: effect_program_id,
        })
}

pub(super) fn effect_program_input_matches_device_target(
    input: EffectProgramInputDefinition,
    target: &AbilityTargetDefinition,
) -> bool {
    match input {
        EffectProgramInputDefinition::SelfTarget => {
            target.modes.as_slice() == [AbilityTargetModeDefinition::SelfTarget]
                && target.range == 0
                && !target.requires_line_of_effect
        }
        EffectProgramInputDefinition::Actor => {
            !target.modes.is_empty()
                && !target
                    .modes
                    .contains(&AbilityTargetModeDefinition::SelfTarget)
                && target.modes.iter().all(|mode| {
                    matches!(
                        mode,
                        AbilityTargetModeDefinition::Direction
                            | AbilityTargetModeDefinition::Position
                            | AbilityTargetModeDefinition::Entity
                    )
                })
                && (1..=64).contains(&target.range)
                && target.requires_line_of_effect
        }
        EffectProgramInputDefinition::Area => false,
        EffectProgramInputDefinition::Item => {
            target.modes.as_slice() == [AbilityTargetModeDefinition::Item]
                && target.range == 0
                && !target.requires_line_of_effect
        }
        EffectProgramInputDefinition::Glyph => false,
    }
}

pub(super) fn validate_effect_program_catalog(
    programs: &BTreeMap<String, ResolvedEffectProgram>,
    content: &CompiledContentV1,
) -> Result<(), ContentError> {
    let terrain_tags = content
        .terrain
        .iter()
        .map(|terrain| {
            (
                terrain.id.clone(),
                terrain.tags.iter().cloned().collect::<BTreeSet<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let actor_tag_values = content
        .actors
        .iter()
        .flat_map(|actor| actor.tags.iter().cloned())
        .collect::<BTreeSet<_>>();
    let item_tag_values = content
        .items
        .iter()
        .flat_map(|item| item.tags.iter().cloned())
        .collect::<BTreeSet<_>>();
    let resource_ids = content
        .resources
        .iter()
        .map(|resource| resource.id.clone())
        .collect::<BTreeSet<_>>();
    let affix_ids = content
        .affixes
        .iter()
        .map(|affix| affix.id.clone())
        .collect::<BTreeSet<_>>();
    let loot_table_ids = content
        .loot_tables
        .iter()
        .map(|table| table.id.clone())
        .collect::<BTreeSet<_>>();

    for (id, program) in programs {
        if !valid_item_effect(
            &program.effect,
            &terrain_tags,
            &actor_tag_values,
            &item_tag_values,
            &resource_ids,
            &affix_ids,
            &loot_table_ids,
        ) {
            return Err(ContentError::InvalidEffectProgram(id.clone()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::{CONTENT_FORMAT_VERSION, compile_pack_dir, source::SourceItemUseActionDefinition};

    fn original_pack_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("crate should be inside the workspace")
            .join("packs/rfb-demo-original")
    }

    fn effect_program(
        id: &str,
        input: EffectProgramInputDefinition,
        steps: Vec<ItemUseEffectDefinition>,
    ) -> EffectProgramDefinition {
        EffectProgramDefinition {
            schema: EFFECT_PROGRAM_SCHEMA.to_owned(),
            format_version: CONTENT_FORMAT_VERSION,
            id: id.to_owned(),
            input,
            steps,
        }
    }

    #[test]
    fn effect_program_catalog_requires_unique_flat_typed_programs() {
        let healing = effect_program(
            "demo.effect.healing",
            EffectProgramInputDefinition::SelfTarget,
            vec![ItemUseEffectDefinition::Heal { amount: 4 }],
        );
        let programs =
            compile_effect_program_catalog(vec![healing.clone()]).expect("program should compile");
        assert_eq!(
            programs.get("demo.effect.healing"),
            Some(&ResolvedEffectProgram {
                input: EffectProgramInputDefinition::SelfTarget,
                effect: ItemUseEffectDefinition::Heal { amount: 4 },
            })
        );

        assert!(matches!(
            compile_effect_program_catalog(vec![healing.clone(), healing]),
            Err(ContentError::DuplicateDefinitionId(id)) if id == "demo.effect.healing"
        ));
        assert!(matches!(
            compile_effect_program_catalog(vec![effect_program(
                "demo.effect.wrong-input",
                EffectProgramInputDefinition::Actor,
                vec![ItemUseEffectDefinition::Heal { amount: 4 }],
            )]),
            Err(ContentError::InvalidEffectProgram(id)) if id == "demo.effect.wrong-input"
        ));
        assert!(matches!(
            compile_effect_program_catalog(vec![effect_program(
                "demo.effect.nested",
                EffectProgramInputDefinition::SelfTarget,
                vec![ItemUseEffectDefinition::Sequence {
                    effects: vec![
                        ItemUseEffectDefinition::Heal { amount: 1 },
                        ItemUseEffectDefinition::Heal { amount: 1 },
                    ],
                }],
            )]),
            Err(ContentError::InvalidEffectProgram(id)) if id == "demo.effect.nested"
        ));
    }

    #[test]
    fn effect_program_bindings_require_a_resolvable_program_reference() {
        let programs = compile_effect_program_catalog(vec![effect_program(
            "demo.effect.healing",
            EffectProgramInputDefinition::SelfTarget,
            vec![ItemUseEffectDefinition::Heal { amount: 4 }],
        )])
        .expect("program should compile");

        let (effect, input) = resolve_source_item_effect(
            "demo.item.test",
            "demo.effect.healing".to_owned(),
            &programs,
        )
        .expect("reference should resolve");
        assert_eq!(effect, ItemUseEffectDefinition::Heal { amount: 4 });
        assert_eq!(input, EffectProgramInputDefinition::SelfTarget);

        assert!(matches!(
            resolve_source_item_effect(
                "demo.item.test",
                "demo.effect.missing".to_owned(),
                &programs,
            ),
            Err(ContentError::DanglingReference { owner, target })
                if owner == "demo.item.test" && target == "demo.effect.missing"
        ));

        assert!(
            serde_json::from_value::<SourceItemUseActionDefinition>(serde_json::json!({
                "effectProgramId": "demo.effect.healing"
            }))
            .is_ok()
        );
        assert!(
            serde_json::from_value::<SourceItemUseActionDefinition>(serde_json::json!({})).is_err()
        );
        assert!(
            serde_json::from_value::<SourceItemUseActionDefinition>(serde_json::json!({
                "effect": { "type": "heal", "amount": 1 }
            }))
            .is_err()
        );
    }

    #[test]
    fn effect_program_catalog_validates_unreferenced_effect_parameters() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");
        let programs = compile_effect_program_catalog(vec![effect_program(
            "demo.effect.invalid-healing",
            EffectProgramInputDefinition::SelfTarget,
            vec![ItemUseEffectDefinition::Heal { amount: 0 }],
        )])
        .expect("structural program contract should compile");

        assert!(matches!(
            validate_effect_program_catalog(&programs, &artifact.content),
            Err(ContentError::InvalidEffectProgram(id)) if id == "demo.effect.invalid-healing"
        ));
    }

    #[test]
    fn effect_program_input_must_match_device_target_policy() {
        let self_target = AbilityTargetDefinition {
            modes: vec![AbilityTargetModeDefinition::SelfTarget],
            range: 0,
            requires_line_of_effect: false,
        };
        let actor_target = AbilityTargetDefinition {
            modes: vec![
                AbilityTargetModeDefinition::Direction,
                AbilityTargetModeDefinition::Position,
                AbilityTargetModeDefinition::Entity,
            ],
            range: 8,
            requires_line_of_effect: true,
        };

        assert!(effect_program_input_matches_device_target(
            EffectProgramInputDefinition::SelfTarget,
            &self_target,
        ));
        assert!(effect_program_input_matches_device_target(
            EffectProgramInputDefinition::Actor,
            &actor_target,
        ));
        assert!(!effect_program_input_matches_device_target(
            EffectProgramInputDefinition::Actor,
            &self_target,
        ));
        assert!(!effect_program_input_matches_device_target(
            EffectProgramInputDefinition::Glyph,
            &self_target,
        ));
    }
}
