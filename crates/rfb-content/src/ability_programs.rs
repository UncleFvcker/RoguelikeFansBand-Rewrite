// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeMap;

#[cfg(feature = "schemas")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    ABILITY_PROGRAM_SCHEMA, AbilityDefinition, AbilityEffectDefinition,
    AbilityGenocideScopeDefinition, AbilityLevelScalingDefinition, AbilityRandomTargetDefinition,
    AbilitySpellPowerDefinition, AbilityTargetDefinition, AbilityTargetModeDefinition,
    ContentError,
};
use crate::player_ability_bindings::ResolvedPlayerAbilityBinding;
use crate::valid_ability_level_scaling;
use crate::validation::{require_format_version, require_schema, validate_definition_id};

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
    target: AbilityTargetDefinition,
    ability_program_id: String,
    #[serde(default)]
    affects_ground_items: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    level_scaling: Vec<AbilityLevelScalingDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    spell_power_fields: Vec<AbilitySpellPowerDefinition>,
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
        let valid_steps = match definition.steps.as_slice() {
            [random_choice @ AbilityEffectDefinition::RandomChoice { .. }] => {
                ability_program_top_level_random_choice_is_valid(definition.input, random_choice)
            }
            steps => {
                (1..=8).contains(&steps.len())
                    && steps
                        .iter()
                        .all(|step| ability_program_input_accepts_step(definition.input, step))
                    && (steps.len() == 1
                        || steps
                            .iter()
                            .all(|step| ability_program_step_is_composable(definition.input, step)))
            }
        };
        if !valid_steps {
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

fn ability_program_top_level_random_choice_is_valid(
    input: AbilityProgramInputDefinition,
    effect: &AbilityEffectDefinition,
) -> bool {
    let AbilityEffectDefinition::RandomChoice {
        roll_sides,
        level_bonus_divisor,
        branches,
    } = effect
    else {
        return false;
    };
    if input != AbilityProgramInputDefinition::CastTarget {
        return false;
    }

    let maximum_roll = u32::from(*roll_sides)
        + if *level_bonus_divisor == 0 {
            0
        } else {
            100 / u32::from(*level_bonus_divisor)
        };
    (2..=10_000).contains(roll_sides)
        && (*level_bonus_divisor == 0 || *level_bonus_divisor <= 100)
        && (2..=64).contains(&branches.len())
        && branches.iter().all(|branch| {
            valid_ability_level_scaling(&branch.effect, &branch.level_scaling)
                && match branch.target {
                    AbilityRandomTargetDefinition::SelfTarget => match branch.effect.as_ref() {
                        AbilityEffectDefinition::Sequence { effects } => {
                            (2..=8).contains(&effects.len())
                                && effects.iter().all(|effect| {
                                    matches!(
                                        effect,
                                        AbilityEffectDefinition::Heal { .. }
                                            | AbilityEffectDefinition::VisibleDamage { .. }
                                            | AbilityEffectDefinition::VisibleApplyStatus { .. }
                                    )
                                })
                        }
                        effect => matches!(
                            effect,
                            AbilityEffectDefinition::Heal { .. }
                                | AbilityEffectDefinition::ApplyStatus { .. }
                                | AbilityEffectDefinition::Summon { .. }
                                | AbilityEffectDefinition::VisibleDamage { .. }
                                | AbilityEffectDefinition::VisibleApplyStatus { .. }
                                | AbilityEffectDefinition::Earthquake { .. }
                                | AbilityEffectDefinition::AreaDestruction { .. }
                                | AbilityEffectDefinition::NoOp { .. }
                        ),
                    },
                    AbilityRandomTargetDefinition::CastTarget => matches!(
                        branch.effect.as_ref(),
                        AbilityEffectDefinition::Damage { .. }
                            | AbilityEffectDefinition::AreaDamage { .. }
                            | AbilityEffectDefinition::BeamDamage { .. }
                            | AbilityEffectDefinition::LightLine { .. }
                            | AbilityEffectDefinition::BoltOrBeamDamage { .. }
                            | AbilityEffectDefinition::ApplyStatus { .. }
                            | AbilityEffectDefinition::DrainLife { .. }
                            | AbilityEffectDefinition::Genocide { .. }
                            | AbilityEffectDefinition::PolymorphTarget
                            | AbilityEffectDefinition::NoOp { .. }
                    ),
                }
        })
        && branches
            .windows(2)
            .all(|pair| pair[0].maximum_roll < pair[1].maximum_roll)
        && branches
            .last()
            .is_some_and(|branch| u32::from(branch.maximum_roll) >= maximum_roll)
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
                    | AbilityEffectDefinition::JumpDamage { .. }
                    | AbilityEffectDefinition::AggravateMonsters
                    | AbilityEffectDefinition::Recall { .. }
                    | AbilityEffectDefinition::ResistElements { .. }
                    | AbilityEffectDefinition::Summon { .. }
                    | AbilityEffectDefinition::SummonCategory { .. }
                    | AbilityEffectDefinition::Detect { .. }
                    | AbilityEffectDefinition::RefuelEquippedLight { .. }
                    | AbilityEffectDefinition::LightArea { .. }
                    | AbilityEffectDefinition::ApplyStatus { .. }
                    | AbilityEffectDefinition::RemoveStatus { .. }
                    | AbilityEffectDefinition::AnimateDead { .. }
                    | AbilityEffectDefinition::Heal { .. }
                    | AbilityEffectDefinition::HealDice { .. }
                    | AbilityEffectDefinition::ReduceStatus { .. }
                    | AbilityEffectDefinition::SatisfyHunger
                    | AbilityEffectDefinition::RestoreVitality { .. }
                    | AbilityEffectDefinition::VisibleDamage { .. }
                    | AbilityEffectDefinition::VisibleApplyStatus { .. }
                    | AbilityEffectDefinition::MassSleepOrStasis { .. }
                    | AbilityEffectDefinition::BlinkSelf { .. }
                    | AbilityEffectDefinition::TeleportSelf { .. }
                    | AbilityEffectDefinition::ReportMagic
                    | AbilityEffectDefinition::Earthquake { .. }
                    | AbilityEffectDefinition::AreaDestruction { .. }
                    | AbilityEffectDefinition::SuppressMonsterReproduction { .. }
                    | AbilityEffectDefinition::PolymorphSelf
                    | AbilityEffectDefinition::TeleportLevel
                    | AbilityEffectDefinition::Clairvoyance { .. }
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
                    | AbilityEffectDefinition::Malediction { .. }
                    | AbilityEffectDefinition::AreaDamage { .. }
                    | AbilityEffectDefinition::BeamDamage { .. }
                    | AbilityEffectDefinition::LightLine { .. }
                    | AbilityEffectDefinition::BoltOrBeamDamage { .. }
                    | AbilityEffectDefinition::BoltOrAreaDamage { .. }
                    | AbilityEffectDefinition::ConeDamage { .. }
                    | AbilityEffectDefinition::BreathDamage { .. }
                    | AbilityEffectDefinition::CurseDamage { .. }
                    | AbilityEffectDefinition::DeathRay { .. }
                    | AbilityEffectDefinition::TeleportAway { .. }
                    | AbilityEffectDefinition::BirdDrop
                    | AbilityEffectDefinition::DrainResource { .. }
                    | AbilityEffectDefinition::Amnesia
                    | AbilityEffectDefinition::DarkenRoom
                    | AbilityEffectDefinition::Teleport
                    | AbilityEffectDefinition::FetchItem { .. }
                    | AbilityEffectDefinition::ConsumeTerrain { .. }
                    | AbilityEffectDefinition::MeleeThenTeleport { .. }
                    | AbilityEffectDefinition::SwapPosition
                    | AbilityEffectDefinition::TransformTerrain { .. }
                    | AbilityEffectDefinition::TerrainBeam { .. }
                    | AbilityEffectDefinition::ApplyStatus { .. }
                    | AbilityEffectDefinition::RemoveStatus { .. }
                    | AbilityEffectDefinition::Control { .. }
                    | AbilityEffectDefinition::DrainLife { .. }
                    | AbilityEffectDefinition::BlinkTarget { .. }
                    | AbilityEffectDefinition::TeleportTarget
                    | AbilityEffectDefinition::TeleportLevel
                    | AbilityEffectDefinition::PolymorphTarget
                    | AbilityEffectDefinition::Rodeo
                    | AbilityEffectDefinition::NoOp { .. }
            ) || matches!(
                effect,
                AbilityEffectDefinition::CreateAmmunition {
                    source_terrain_tags,
                    ..
                } if !source_terrain_tags.is_empty()
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
            matches!(
                effect,
                AbilityEffectDefinition::IdentifyItem { .. }
                    | AbilityEffectDefinition::IdentifyOrMassIdentify { .. }
                    | AbilityEffectDefinition::BrandWeapon { .. }
                    | AbilityEffectDefinition::TransmuteItemToGold { .. }
                    | AbilityEffectDefinition::DrainItemMagic { .. }
                    | AbilityEffectDefinition::RechargeFromPlayer { .. }
            ) || matches!(
                effect,
                AbilityEffectDefinition::CreateAmmunition {
                    source_item_tags,
                    ..
                } if !source_item_tags.is_empty()
            )
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
                | AbilityEffectDefinition::HealDice { .. }
                | AbilityEffectDefinition::ReduceStatus { .. }
                | AbilityEffectDefinition::ApplyStatus { .. }
                | AbilityEffectDefinition::RemoveStatus { .. }
                | AbilityEffectDefinition::AnimateDead { .. }
                | AbilityEffectDefinition::AreaDamage { .. }
                | AbilityEffectDefinition::AggravateMonsters
                | AbilityEffectDefinition::Detect { .. }
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
    effect: &AbilityEffectDefinition,
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
                && (target.requires_line_of_effect
                    || matches!(effect, AbilityEffectDefinition::DarkenRoom))
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
        player_bindings: &BTreeMap<String, ResolvedPlayerAbilityBinding>,
    ) -> Result<AbilityDefinition, ContentError> {
        let player = player_bindings.get(&self.id).cloned();
        let program = resolve_source_ability_program(&self.id, self.ability_program_id, programs)?;
        if !ability_program_input_matches_target(program.input, &self.target, &program.effect) {
            return Err(ContentError::InvalidAbility(self.id));
        }
        Ok(AbilityDefinition {
            schema: self.schema,
            format_version: self.format_version,
            id: self.id,
            name_key: self.name_key,
            description_key: self.description_key,
            target: self.target,
            effect: program.effect,
            affects_ground_items: self.affects_ground_items,
            level_scaling: self.level_scaling,
            spell_power_fields: self.spell_power_fields,
            spell_power_bonus: 0,
            player,
            tags: self.tags,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ABILITY_SCHEMA, AbilityProficiencyDefinition, AbilityRandomBranchDefinition,
        ActorDamageType, CONTENT_FORMAT_VERSION,
    };

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
        ability_program_id: &str,
    ) -> SourceAbilityDefinition {
        SourceAbilityDefinition {
            schema: ABILITY_SCHEMA.to_owned(),
            format_version: CONTENT_FORMAT_VERSION,
            id: "demo.ability.test".to_owned(),
            name_key: "ability-demo-test-name".to_owned(),
            description_key: "ability-demo-test-description".to_owned(),
            target,
            ability_program_id: ability_program_id.to_owned(),
            affects_ground_items: false,
            level_scaling: Vec::new(),
            spell_power_fields: Vec::new(),
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

        let random_choice = AbilityEffectDefinition::RandomChoice {
            roll_sides: 2,
            level_bonus_divisor: 0,
            branches: vec![
                AbilityRandomBranchDefinition {
                    maximum_roll: 1,
                    target: AbilityRandomTargetDefinition::SelfTarget,
                    effect: Box::new(AbilityEffectDefinition::Heal { amount: 1 }),
                    level_scaling: Vec::new(),
                },
                AbilityRandomBranchDefinition {
                    maximum_roll: 2,
                    target: AbilityRandomTargetDefinition::CastTarget,
                    effect: Box::new(AbilityEffectDefinition::Damage {
                        damage_dice: 1,
                        damage_sides: 2,
                        damage_bonus: 0,
                        damage_type: ActorDamageType::Physical,
                    }),
                    level_scaling: Vec::new(),
                },
            ],
        };
        let random_programs = compile_ability_program_catalog(vec![ability_program(
            "demo.ability-program.random",
            AbilityProgramInputDefinition::CastTarget,
            vec![random_choice.clone()],
        )])
        .expect("one top-level random choice should compile");
        assert_eq!(
            random_programs
                .get("demo.ability-program.random")
                .map(|program| &program.effect),
            Some(&random_choice)
        );

        let mut self_sequence_random_choice = random_choice.clone();
        let AbilityEffectDefinition::RandomChoice { branches, .. } =
            &mut self_sequence_random_choice
        else {
            unreachable!("test effect should remain random choice");
        };
        *branches[0].effect = AbilityEffectDefinition::Sequence {
            effects: vec![
                AbilityEffectDefinition::Heal { amount: 1 },
                AbilityEffectDefinition::Heal { amount: 1 },
            ],
        };
        compile_ability_program_catalog(vec![ability_program(
            "demo.ability-program.self-sequence-random",
            AbilityProgramInputDefinition::CastTarget,
            vec![self_sequence_random_choice.clone()],
        )])
        .expect("one non-nested self sequence should compile inside a random branch");

        let AbilityEffectDefinition::RandomChoice { branches, .. } =
            &mut self_sequence_random_choice
        else {
            unreachable!("test effect should remain random choice");
        };
        let AbilityEffectDefinition::Sequence { effects } = branches[0].effect.as_mut() else {
            unreachable!("test branch should remain a sequence");
        };
        effects[0] = AbilityEffectDefinition::Sequence {
            effects: vec![
                AbilityEffectDefinition::Heal { amount: 1 },
                AbilityEffectDefinition::Heal { amount: 1 },
            ],
        };
        assert!(matches!(
            compile_ability_program_catalog(vec![ability_program(
                "demo.ability-program.nested-random",
                AbilityProgramInputDefinition::CastTarget,
                vec![self_sequence_random_choice],
            )]),
            Err(ContentError::InvalidAbilityProgram(id))
                if id == "demo.ability-program.nested-random"
        ));
        assert!(matches!(
            compile_ability_program_catalog(vec![ability_program(
                "demo.ability-program.mixed-random",
                AbilityProgramInputDefinition::CastTarget,
                vec![
                    random_choice,
                    AbilityEffectDefinition::Damage {
                        damage_dice: 1,
                        damage_sides: 2,
                        damage_bonus: 0,
                        damage_type: ActorDamageType::Physical,
                    },
                ],
            )]),
            Err(ContentError::InvalidAbilityProgram(id))
                if id == "demo.ability-program.mixed-random"
        ));
    }

    #[test]
    fn ability_program_references_are_required_and_player_bindings_are_optional() {
        let programs = compile_ability_program_catalog(vec![ability_program(
            "demo.ability-program.healing",
            AbilityProgramInputDefinition::SelfTarget,
            vec![AbilityEffectDefinition::Heal { amount: 4 }],
        )])
        .expect("ability program should compile");
        let player_bindings = BTreeMap::from([(
            "demo.ability.test".to_owned(),
            ResolvedPlayerAbilityBinding {
                minimum_level: 1,
                resource_id: "demo.resource.mana".to_owned(),
                resource_cost: 1,
                base_failure_percent: 0,
                first_success_experience: 0,
                proficiency: AbilityProficiencyDefinition::default(),
                cooldown: None,
            },
        )]);
        let self_target = AbilityTargetDefinition {
            modes: vec![AbilityTargetModeDefinition::SelfTarget],
            range: 0,
            requires_line_of_effect: false,
        };

        let referenced_source = source_ability(self_target.clone(), "demo.ability-program.healing");
        let referenced_json = serde_json::to_value(referenced_source)
            .expect("referenced source ability should serialize");
        assert!(referenced_json.get("effect").is_none());
        assert!(referenced_json.get("minimumLevel").is_none());
        let referenced = serde_json::from_value::<SourceAbilityDefinition>(referenced_json)
            .expect("referenced source ability should deserialize")
            .into_compiled(&programs, &player_bindings)
            .expect("Program and casting binding should lower");
        assert_eq!(
            referenced.effect,
            AbilityEffectDefinition::Heal { amount: 4 }
        );
        let player = referenced
            .player
            .expect("matching player binding should be lowered");
        assert_eq!(player.minimum_level, 1);
        assert_eq!(player.resource_id, "demo.resource.mana");

        let monster_only = source_ability(self_target.clone(), "demo.ability-program.healing")
            .into_compiled(&programs, &BTreeMap::new())
            .expect("an ability without a player binding should compile");
        assert!(monster_only.player.is_none());
        assert!(matches!(
            source_ability(self_target.clone(), "demo.ability-program.missing")
                .into_compiled(&programs, &player_bindings),
            Err(ContentError::DanglingReference { owner, target })
                if owner == "demo.ability.test"
                    && target == "demo.ability-program.missing"
        ));

        let serialized = serde_json::to_value(source_ability(
            self_target.clone(),
            "demo.ability-program.healing",
        ))
        .expect("source ability should serialize");
        let mut missing_program = serialized.clone();
        missing_program
            .as_object_mut()
            .expect("source ability should be an object")
            .remove("abilityProgramId");
        assert!(serde_json::from_value::<SourceAbilityDefinition>(missing_program).is_err());
        let mut inline_effect = serialized;
        inline_effect
            .as_object_mut()
            .expect("source ability should be an object")
            .insert(
                "effect".to_owned(),
                serde_json::json!({ "type": "heal", "amount": 4 }),
            );
        assert!(serde_json::from_value::<SourceAbilityDefinition>(inline_effect).is_err());

        let cast_target = AbilityTargetDefinition {
            modes: vec![AbilityTargetModeDefinition::Entity],
            range: 6,
            requires_line_of_effect: true,
        };
        assert!(matches!(
            source_ability(cast_target, "demo.ability-program.healing")
                .into_compiled(&programs, &player_bindings),
            Err(ContentError::InvalidAbility(id)) if id == "demo.ability.test"
        ));
    }
}
