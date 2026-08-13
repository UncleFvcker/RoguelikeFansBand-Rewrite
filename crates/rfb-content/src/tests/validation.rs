use std::collections::BTreeMap;

use super::*;

#[test]
fn dynamic_devices_require_the_device_skill() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let mut invalid = artifact.content.clone();
    invalid
        .skills
        .retain(|skill| skill.kind != SkillKind::Device);
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::MissingRequiredSkillKind(actual)) if actual == "device"
    ));
}

#[test]
fn dangling_references_and_checksum_corruption_are_rejected() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let mut invalid = artifact.content.clone();
    invalid.worlds[0].fill_terrain_id = "demo.terrain.missing".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::DanglingReference { .. })
    ));

    let mut corrupted = artifact.bytes;
    let last = corrupted.len() - 1;
    corrupted[last] ^= 0x01;
    assert!(matches!(
        decode_content(&corrupted),
        Err(ContentError::ChecksumMismatch)
    ));
}

#[test]
fn semantic_versions_are_checked_strictly() {
    assert!(validate_semver("1.2.3-alpha.1+build.5").is_ok());
    for invalid in ["01.2.3", "1.2", "1.2.3-", "1.2.3+", "1.2.3-alpha..1"] {
        assert!(matches!(
            validate_semver(invalid),
            Err(ContentError::InvalidPackVersion(_))
        ));
    }
}

#[test]
fn class_level_resistance_thresholds_are_strict() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let mut duplicate = artifact.content.clone();
    let paladin = duplicate
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.paladin")
        .expect("Paladin class should exist");
    paladin
        .level_resistances
        .push(paladin.level_resistances[0].clone());
    assert!(matches!(
        validate_and_normalize(&mut duplicate),
        Err(ContentError::InvalidCharacterSource(id)) if id == "demo.class.paladin"
    ));

    let mut empty = artifact.content;
    empty
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.paladin")
        .expect("Paladin class should exist")
        .level_resistances[0]
        .resistances
        .clear();
    assert!(matches!(
        validate_and_normalize(&mut empty),
        Err(ContentError::InvalidCharacterSource(id)) if id == "demo.class.paladin"
    ));
}

#[test]
fn sniping_profiles_and_concentration_requirements_are_strict() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let profile = SnipingProfileDefinition {
        preferred_ammunition_type: AmmunitionTypeDefinition::Bolt,
        preferred_ammunition_to_hit_base: 10,
        preferred_ammunition_to_hit_level_divisor: 5,
        base_shot_excess_percent: 50,
        preferred_ammunition_critical_chance_percent: 150,
        base_concentration_maximum: 2,
        concentration_level_offset: 5,
        concentration_level_divisor: 10,
        concentration_bonus_percent_per_level: 10,
    };
    let mut valid = artifact.content.clone();
    let mut shot = valid
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.archer-create-shots")
        .expect("Archer power should exist")
        .clone();
    shot.id = "test.ability.sniper-shot".to_owned();
    shot.target = AbilityTargetDefinition {
        modes: vec![AbilityTargetModeDefinition::Direction],
        range: 20,
        requires_line_of_effect: true,
    };
    shot.effect = AbilityEffectDefinition::SniperShot {
        mode: SniperShotModeDefinition::Shining,
    };
    let mut probe = shot.clone();
    probe.id = "test.ability.sniper-probe".to_owned();
    probe.target = AbilityTargetDefinition {
        modes: vec![AbilityTargetModeDefinition::SelfTarget],
        range: 0,
        requires_line_of_effect: false,
    };
    probe.effect = AbilityEffectDefinition::ProbeMonsters;
    valid.abilities.push(shot);
    valid.abilities.push(probe);
    let archer = valid
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.archer")
        .expect("Archer class should exist");
    archer.sniping_profile = Some(profile);
    archer.abilities[0].ability_id = "test.ability.sniper-shot".to_owned();
    archer.abilities[0].minimum_concentration = 1;
    archer.abilities[0].hit_point_cost = 1;
    let mut probe_binding = archer.abilities[0].clone();
    probe_binding.ability_id = "test.ability.sniper-probe".to_owned();
    probe_binding.minimum_level = 15;
    probe_binding.governing_attribute = Some(TechniqueAttribute::Intelligence);
    probe_binding.minimum_concentration = 0;
    probe_binding.hit_point_cost = 20;
    probe_binding.base_failure_percent = 80;
    archer.abilities.push(probe_binding);
    assert!(validate_and_normalize(&mut valid).is_ok());

    let mut no_profile = artifact.content.clone();
    no_profile
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.archer")
        .expect("Archer class should exist")
        .abilities[0]
        .minimum_concentration = 1;
    assert!(matches!(
        validate_and_normalize(&mut no_profile),
        Err(ContentError::InvalidCharacterSource(id)) if id == "demo.class.archer"
    ));

    let mut invalid_divisor = artifact.content;
    let archer = invalid_divisor
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.archer")
        .expect("Archer class should exist");
    archer.sniping_profile = Some(SnipingProfileDefinition {
        concentration_level_divisor: 0,
        ..profile
    });
    assert!(matches!(
        validate_and_normalize(&mut invalid_divisor),
        Err(ContentError::InvalidCharacterSource(id)) if id == "demo.class.archer"
    ));
}

#[test]
fn race_level_mutation_rewards_are_strict_and_canonical() {
    let mut artifact =
        compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    human_level_mutation_rewards(&mut artifact.content).clear();
    let choice = RaceLevelMutationRewardDefinition {
        id: "talent".to_owned(),
        minimum_level: 20,
        selection: RaceMutationSelectionDefinition::Choice {
            mutation_ids: vec![
                "rfb.mutation.ambidextrous".to_owned(),
                "rfb.mutation.evasion".to_owned(),
            ],
        },
    };
    let weakness = RaceLevelMutationRewardDefinition {
        id: "weakness".to_owned(),
        minimum_level: 35,
        selection: RaceMutationSelectionDefinition::CastingAttribute {
            default_mutation_id: "rfb.mutation.black-marketeer".to_owned(),
            mutation_ids_by_attribute: BTreeMap::from([(
                CastingAttribute::Intelligence,
                "rfb.mutation.astral-guide".to_owned(),
            )]),
        },
    };

    let mut valid = artifact.content.clone();
    human_level_mutation_rewards(&mut valid).extend([weakness.clone(), choice.clone()]);
    validate_and_normalize(&mut valid).expect("valid rewards should normalize");
    let rewards = &valid
        .races
        .iter()
        .find(|race| race.id == "demo.race.rfb-human")
        .expect("Human race should exist")
        .level_mutation_rewards;
    assert_eq!(rewards[0].id, "talent");
    assert_eq!(rewards[1].id, "weakness");

    let mut duplicate_reward = artifact.content.clone();
    human_level_mutation_rewards(&mut duplicate_reward).extend([choice.clone(), choice.clone()]);
    assert!(matches!(
        validate_and_normalize(&mut duplicate_reward),
        Err(ContentError::InvalidCharacterSource(id)) if id == "demo.race.rfb-human"
    ));

    let mut zero_level = artifact.content.clone();
    human_level_mutation_rewards(&mut zero_level).push(RaceLevelMutationRewardDefinition {
        minimum_level: 0,
        ..choice.clone()
    });
    assert!(matches!(
        validate_and_normalize(&mut zero_level),
        Err(ContentError::InvalidCharacterSource(id)) if id == "demo.race.rfb-human"
    ));

    let mut empty_choice = artifact.content.clone();
    human_level_mutation_rewards(&mut empty_choice).push(RaceLevelMutationRewardDefinition {
        selection: RaceMutationSelectionDefinition::Choice {
            mutation_ids: Vec::new(),
        },
        ..choice.clone()
    });
    assert!(matches!(
        validate_and_normalize(&mut empty_choice),
        Err(ContentError::InvalidCharacterSource(id)) if id == "demo.race.rfb-human"
    ));

    let mut duplicate_mutation = artifact.content.clone();
    let mut repeated = weakness.clone();
    repeated.selection = RaceMutationSelectionDefinition::CastingAttribute {
        default_mutation_id: "rfb.mutation.ambidextrous".to_owned(),
        mutation_ids_by_attribute: BTreeMap::new(),
    };
    human_level_mutation_rewards(&mut duplicate_mutation).extend([choice.clone(), repeated]);
    assert!(matches!(
        validate_and_normalize(&mut duplicate_mutation),
        Err(ContentError::InvalidCharacterSource(id)) if id == "demo.race.rfb-human"
    ));

    let mut random_candidate = artifact.content;
    human_level_mutation_rewards(&mut random_candidate).push(RaceLevelMutationRewardDefinition {
        selection: RaceMutationSelectionDefinition::Choice {
            mutation_ids: vec!["rfb.mutation.spit-acid".to_owned()],
        },
        ..choice
    });
    assert!(matches!(
        validate_and_normalize(&mut random_candidate),
        Err(ContentError::InvalidCharacterSource(id)) if id == "demo.race.rfb-human"
    ));
}

#[test]
fn race_infravision_is_nonnegative_and_bounded() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    for invalid in [-1, 65] {
        let mut content = artifact.content.clone();
        let human = content
            .races
            .iter_mut()
            .find(|race| race.id == "demo.race.rfb-human")
            .expect("Human race should exist");
        human.infravision = invalid;
        assert!(matches!(
            validate_and_normalize(&mut content),
            Err(ContentError::InvalidCharacterSource(id)) if id == "demo.race.rfb-human"
        ));
    }
}

fn human_level_mutation_rewards(
    content: &mut CompiledContentV1,
) -> &mut Vec<RaceLevelMutationRewardDefinition> {
    &mut content
        .races
        .iter_mut()
        .find(|race| race.id == "demo.race.rfb-human")
        .expect("Human race should exist")
        .level_mutation_rewards
}
