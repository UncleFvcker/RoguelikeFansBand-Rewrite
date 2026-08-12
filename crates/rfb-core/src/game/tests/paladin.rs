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
