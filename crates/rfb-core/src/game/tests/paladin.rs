// SPDX-License-Identifier: MPL-2.0

use super::*;

const PALADIN_BUILD_ID: &str = "demo.build.paladin-death";

fn paladin_game(seed: u64) -> Game {
    Game::new_with_build(seed, PALADIN_BUILD_ID).expect("Death Paladin build should create")
}

#[test]
fn death_paladin_birth_uses_the_original_class_identity_skills_and_kit() {
    let game = paladin_game(0x5041_4c41_4449_4e00);
    let snapshot = game.snapshot();
    let build = snapshot
        .player
        .build
        .expect("Paladin should project its build");

    assert_eq!(build.build_id, PALADIN_BUILD_ID);
    assert_eq!(build.class_id, "demo.class.paladin");
    assert_eq!(build.life_percent, 110);
    assert_eq!(build.experience_percent, 135);
    assert_eq!(snapshot.player.kind_id, "demo.actor.paladin-player");
    assert_eq!(snapshot.player.progress.attributes.strength.effective, 15);
    assert_eq!(
        snapshot.player.progress.attributes.intelligence.effective,
        10
    );
    assert_eq!(snapshot.player.progress.attributes.wisdom.effective, 14);
    assert_eq!(
        snapshot.player.progress.attributes.constitution.effective,
        15
    );
    assert_eq!(snapshot.player.progress.attributes.charisma.effective, 15);

    for kind_id in [
        "demo.item.broad-sword",
        "demo.item.ring-mail",
        "demo.item.black-prayers",
    ] {
        assert!(
            game.items.iter().any(|item| item.kind_id == kind_id),
            "birth kit should contain {kind_id}"
        );
    }
    for kind_id in ["demo.item.broad-sword", "demo.item.ring-mail"] {
        assert!(game.items.iter().any(|item| {
            item.kind_id == kind_id && matches!(item.location, ItemLocation::Equipped { .. })
        }));
    }

    let skill = |id: &str| {
        snapshot
            .player
            .progress
            .skills
            .iter()
            .find(|skill| skill.id == id)
            .expect("original Paladin skill should be projected")
    };
    for (id, base, growth) in [
        ("demo.skill.disarming", 20, 7),
        ("demo.skill.device", 24, 10),
        ("demo.skill.saving-throw", 34, 11),
        ("demo.skill.stealth", 1, 0),
        ("demo.skill.search", 12, 0),
        ("demo.skill.perception", 2, 0),
        ("demo.skill.melee", 68, 21),
        ("demo.skill.ranged", 40, 18),
    ] {
        assert_eq!(
            (skill(id).base, skill(id).growth_per_ten_levels),
            (base, growth)
        );
    }
}

#[test]
fn death_paladin_projects_divine_study_mana_and_the_original_spell_table() {
    let game = paladin_game(7);
    let snapshot = game.snapshot();
    let mana = snapshot
        .player
        .resources
        .iter()
        .find(|resource| resource.id == "demo.resource.mana")
        .expect("Paladin should have Mana");
    assert_eq!((mana.current, mana.maximum), (7, 7));
    assert_eq!(
        snapshot.player.ability_learning,
        Some(rfb_protocol::AbilityLearningDto {
            learned_count: 0,
            capacity: 1,
            remaining_slots: 1,
            study_mode: rfb_protocol::AbilityStudyModeDto::DivineRandom,
        })
    );

    let learned = snapshot
        .player
        .abilities
        .iter()
        .filter(|ability| ability.source == AbilitySourceDto::Learned)
        .collect::<Vec<_>>();
    assert_eq!(learned.len(), 32);
    assert!(
        learned
            .iter()
            .all(|ability| ability.book_name_key.is_some())
    );

    let detect_unlife = learned
        .iter()
        .find(|ability| ability.id == "demo.ability.death-detect-unlife")
        .expect("first Death prayer should be projected");
    assert_eq!(detect_unlife.minimum_level, 1);
    assert_eq!(detect_unlife.base_resource_cost, 1);
    assert_eq!(detect_unlife.failure_percent, 25);
    assert!(detect_unlife.can_study);

    let malediction = learned
        .iter()
        .find(|ability| ability.id == "demo.ability.death-malediction")
        .expect("second Death prayer should be projected");
    assert_eq!(
        (
            malediction.minimum_level,
            malediction.base_resource_cost,
            malediction.failure_percent
        ),
        (3, 2, 25)
    );
    assert!(!malediction.can_study);

    let wraithform = learned
        .iter()
        .find(|ability| ability.id == "demo.ability.death-wraithform")
        .expect("last Death prayer should be projected");
    assert_eq!(
        (
            wraithform.minimum_level,
            wraithform.base_resource_cost,
            wraithform.failure_percent
        ),
        (50, 111, 95)
    );
    assert_eq!(
        wraithform.book_name_key.as_deref(),
        Some("ability-book-demo-necronomicon-name")
    );
    assert_eq!(wraithform.book_rank, Some(4));
}

#[test]
fn death_paladin_unlocks_hell_lance_and_fear_resistance_at_original_levels() {
    const HELL_LANCE_ID: &str = "demo.ability.paladin-hell-lance";

    let mut game = paladin_game(0x4845_4c4c_4c41_4e43);
    let activation = game
        .content
        .class("demo.class.paladin")
        .expect("Paladin class should exist")
        .abilities
        .iter()
        .find(|activation| activation.ability_id == HELL_LANCE_ID)
        .expect("Paladin should own Hell Lance");
    assert_eq!(activation.minimum_level, 30);
    assert_eq!(activation.resource_cost, 30);
    assert_eq!(activation.base_failure_percent, 70);

    game.progress.level = 29;
    game.progress.max_level = 29;
    let before_unlock = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == HELL_LANCE_ID)
        .expect("Hell Lance should be projected for the class UI");
    assert_eq!(before_unlock.source, AbilitySourceDto::Class);
    assert_eq!(before_unlock.minimum_level, 30);
    assert_eq!(before_unlock.failure_percent, 100);
    assert!(!before_unlock.can_cast);

    game.progress.level = 30;
    game.progress.max_level = 30;
    game.refresh_player_resource_maxima();
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("level 30 Paladin should retain Mana");
    mana.current = mana.maximum;
    let at_unlock = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == HELL_LANCE_ID)
        .expect("Hell Lance should remain projected at level 30");
    assert!(at_unlock.can_cast);
    assert!(at_unlock.beam_damage);
    assert!(matches!(
        at_unlock.effects.as_slice(),
        [rfb_protocol::AbilityEffectSpecDto::BeamDamage {
            damage_dice: 1,
            damage_sides: 1,
            damage_bonus: 89,
            damage_type: DamageTypeDto::HellFire,
        }]
    ));

    game.entities.clear();
    game.debug_ability_casts_succeed = true;
    let mut events = Vec::new();
    game.resolve_player_ability(
        HELL_LANCE_ID,
        TargetSelection::Direction {
            direction: Direction::East,
        },
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Hell Lance should resolve");
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityBeamDamage { resolution, .. }
            if resolution.base_raw_damage == 90
                && resolution.damage_type == DamageTypeDto::HellFire
    )));

    game.progress.level = 39;
    game.progress.max_level = 39;
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Fear),
        ResistanceLevel::Normal
    );
    game.progress.level = 40;
    game.progress.max_level = 40;
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Fear),
        ResistanceLevel::Resistant
    );

    let mut restorable = paladin_game(0x5041_4c41_4449_4e40);
    restorable.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(40),
        &mut Vec::new(),
    );
    assert_eq!(restorable.progress.level, 40);
    assert_eq!(
        restorable
            .effective_player_resistances()
            .level(DamageType::Fear),
        ResistanceLevel::Resistant
    );
    let restored = Game::from_save(restorable.to_save()).expect("level 40 Paladin should reload");
    assert_eq!(restored.progress.level, 40);
    assert_eq!(
        restored
            .effective_player_resistances()
            .level(DamageType::Fear),
        ResistanceLevel::Resistant
    );
    let restored_lance = restored
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == HELL_LANCE_ID)
        .expect("reloaded Paladin should retain Hell Lance");
    assert_eq!(restored_lance.source, AbilitySourceDto::Class);
    assert!(matches!(
        restored_lance.effects.as_slice(),
        [rfb_protocol::AbilityEffectSpecDto::BeamDamage {
            damage_dice: 1,
            damage_sides: 1,
            damage_bonus: 119,
            damage_type: DamageTypeDto::HellFire,
        }]
    ));
}
