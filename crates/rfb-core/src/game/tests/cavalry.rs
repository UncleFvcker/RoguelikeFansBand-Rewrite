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
fn cavalry_birth_projects_proficiencies_and_rodeo() {
    let game = cavalry_game(0x0043_4156_414c_5259);
    let snapshot = game.snapshot();
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
    game.rng = RfbRng::seeded(0);

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
