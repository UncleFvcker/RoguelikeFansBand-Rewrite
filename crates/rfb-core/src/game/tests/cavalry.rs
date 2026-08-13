// SPDX-License-Identifier: MPL-2.0

use super::support::{clear_monsters, game_with_actor_definition, replace_terrain};
use super::*;

const CAVALRY_BUILD_ID: &str = "demo.build.cavalry";
const RODEO_ABILITY_ID: &str = "demo.ability.cavalry-rodeo";

fn cavalry_game(seed: u64) -> Game {
    Game::new_with_build(seed, CAVALRY_BUILD_ID).expect("Cavalry build should create")
}

fn cavalry_game_with_horse(
    seed: u64,
    update: impl FnOnce(&mut rfb_content::ActorDefinition),
) -> Game {
    let prepared = game_with_actor_definition(seed, "demo.actor.horse", update);
    Game::from_content_with_build(seed, prepared.content, DEFAULT_WORLD_ID, CAVALRY_BUILD_ID)
        .expect("custom Cavalry game should create")
}

fn place_wild_horse(game: &mut Game) {
    clear_monsters(game);
    let target = game.position_in_direction(Direction::East);
    replace_terrain(game, game.player.position, "demo.terrain.floor");
    replace_terrain(game, target, "demo.terrain.floor");
    game.push_generated_actor("test.rodeo-horse".to_owned(), "demo.actor.horse", target);
}

fn cast_rodeo(game: &mut Game) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.resolve_player_ability(
        RODEO_ABILITY_ID,
        TargetSelection::Direction {
            direction: Direction::East,
        },
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Rodeo should resolve");
    events
}

#[test]
fn cavalry_birth_uses_original_identity_skills_proficiencies_and_kit() {
    let game = cavalry_game(0x0043_4156_414c_5259);
    let snapshot = game.snapshot();
    let build = snapshot
        .player
        .build
        .expect("Cavalry should project its build");

    assert_eq!(build.build_id, CAVALRY_BUILD_ID);
    assert_eq!(build.class_id, "demo.class.cavalry");
    assert_eq!((build.life_percent, build.experience_percent), (111, 120));
    assert_eq!(snapshot.player.kind_id, "demo.actor.cavalry-player");
    let attributes = snapshot.player.progress.attributes;
    assert_eq!(attributes.strength.effective, 15);
    assert_eq!(attributes.intelligence.effective, 11);
    assert_eq!(attributes.wisdom.effective, 11);
    assert_eq!(attributes.dexterity.effective, 15);
    assert_eq!(attributes.constitution.effective, 15);
    assert_eq!(attributes.charisma.effective, 14);

    let skill = |id: &str| {
        snapshot
            .player
            .progress
            .skills
            .iter()
            .find(|skill| skill.id == id)
            .expect("original Cavalry skill should be projected")
    };
    for (id, base, growth) in [
        ("demo.skill.disarming", 20, 10),
        ("demo.skill.device", 18, 7),
        ("demo.skill.saving-throw", 32, 10),
        ("demo.skill.stealth", 1, 0),
        ("demo.skill.search", 16, 0),
        ("demo.skill.perception", 10, 0),
        ("demo.skill.melee", 60, 22),
        ("demo.skill.ranged", 66, 26),
    ] {
        assert_eq!(
            (skill(id).base, skill(id).growth_per_ten_levels),
            (base, growth)
        );
    }

    assert_eq!(snapshot.player.progress.riding_proficiency.current, 2_000);
    assert_eq!(snapshot.player.progress.riding_proficiency.maximum, 8_000);
    let short_bow = snapshot
        .player
        .progress
        .weapon_proficiencies
        .iter()
        .find(|entry| entry.item_kind_id == "demo.item.short-bow")
        .expect("Cavalry short-bow proficiency");
    assert_eq!((short_bow.current, short_bow.maximum), (4_000, 8_000));

    for kind_id in [
        "demo.item.broad-spear",
        "demo.item.leather-scale-mail",
        "demo.item.short-bow",
    ] {
        assert!(game.items.iter().any(|item| {
            item.kind_id == kind_id && matches!(item.location, ItemLocation::Equipped { .. })
        }));
    }
    let arrows = game
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.arrow")
        .expect("Cavalry should start with arrows");
    assert!((15..=25).contains(&arrows.quantity));

    let rodeo = snapshot
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == RODEO_ABILITY_ID)
        .expect("Rodeo should be projected");
    assert_eq!(rodeo.source, AbilitySourceDto::Class);
    assert_eq!(rodeo.minimum_level, 10);
    assert_eq!(rodeo.target_spec.range, 1);
    assert_eq!(
        rodeo.target_spec.modes,
        [rfb_protocol::TargetModeDto::Direction]
    );
    assert!(matches!(
        rodeo.effects.as_slice(),
        [rfb_protocol::AbilityEffectSpecDto::Rodeo]
    ));
    assert!(!rodeo.can_cast);

    let class = game
        .content
        .class("demo.class.cavalry")
        .expect("Cavalry class should exist");
    assert_eq!(class.base_hp, 10);
    assert_eq!(class.pet_upkeep_divisor, 35);
    assert!(class.riding_combat_expert);
    assert_eq!(class.mounted_non_arrow_base_shot_cap, Some(100));
}

#[test]
fn rodeo_mounts_and_tames_a_wild_adjacent_monster() {
    let mut game = cavalry_game(1);
    game.progress.level = 50;
    game.progress.max_level = 50;
    game.refresh_character_skills();
    game.progress.riding_proficiency = 8_000;
    game.debug_ability_casts_succeed = true;
    place_wild_horse(&mut game);

    let events = cast_rodeo(&mut game);

    assert_eq!(game.riding_actor_id.as_deref(), Some("test.rodeo-horse"));
    assert_eq!(
        game.entities[0].controller_id.as_deref(),
        Some(game.player.id.as_str())
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::RidingMounted { .. }))
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::RodeoTamed { .. }))
    );
    Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("tamed riding state should remain valid");

    let rng_before = game.rng.clone();
    let repeated = cast_rodeo(&mut game);
    assert_eq!(game.rng, rng_before);
    assert!(matches!(
        repeated.as_slice(),
        [DomainEvent::RodeoAlreadyRiding]
    ));
}

#[test]
fn guardian_and_questor_mounts_are_thrown_off_without_becoming_pets() {
    for protected_tag in ["guardian", "questor"] {
        let mut game = cavalry_game_with_horse(2, |horse| {
            horse.level = 1;
            horse.tags.push(protected_tag.to_owned());
        });
        game.progress.level = 50;
        game.progress.max_level = 50;
        game.refresh_character_skills();
        game.progress.riding_proficiency = 8_000;
        game.debug_ability_casts_succeed = true;
        place_wild_horse(&mut game);

        let events = cast_rodeo(&mut game);

        assert_eq!(game.riding_actor_id, None, "{protected_tag}");
        assert_eq!(game.entities[0].controller_id, None, "{protected_tag}");
        assert!(
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::RodeoUntameable { .. }))
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::RodeoThrownOff { .. }))
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::RidingFell { .. }))
        );
        Game::from_save_with_content(game.to_save(), game.content.clone())
            .unwrap_or_else(|error| panic!("{protected_tag} failure should remain valid: {error}"));
    }
}
