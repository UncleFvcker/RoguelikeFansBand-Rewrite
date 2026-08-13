// SPDX-License-Identifier: MPL-2.0

use super::*;

const HIGH_MAGE_BUILD_ID: &str = "demo.build.high-mage-death";

fn high_mage_game(seed: u64) -> Game {
    let mut game =
        Game::new_with_build(seed, HIGH_MAGE_BUILD_ID).expect("High-Mage build should create");
    game.entities.clear();
    game
}

fn add_actor(game: &mut Game, kind_id: &str, ordinal: usize, controlled: bool) {
    let id = format!("test.pet.{ordinal}");
    game.push_generated_actor(
        id.clone(),
        kind_id,
        Position {
            x: i32::try_from(ordinal).unwrap_or(i32::MAX),
            y: 1,
        },
    );
    if controlled {
        let player_id = game.player.id.clone();
        game.entities
            .iter_mut()
            .find(|actor| actor.id == id)
            .expect("spawned test actor should remain available")
            .controller_id = Some(player_id);
    }
}

#[test]
fn upkeep_uses_the_class_divisor_unique_cost_and_strict_control() {
    let mut game = high_mage_game(1);
    assert_eq!(
        game.character_definitions()
            .expect("High-Mage build should resolve")
            .2
            .pet_upkeep_divisor,
        25
    );

    add_actor(&mut game, "demo.actor.coatl", 0, false);
    assert_eq!(
        game.pet_upkeep().percent,
        0,
        "ordinary friendly actors do not count"
    );
    game.entities[0].summon = Some(SummonIdentity {
        owner_id: game.player.id.clone(),
        source_ability_id: "test.ability.summon".to_owned(),
        remaining_turns: 5,
    });
    assert_eq!(game.pet_upkeep().controlled_pets, 1);
    assert_eq!(game.pet_upkeep().percent, 26);

    game.entities.clear();
    add_actor(&mut game, "demo.actor.ugluk-the-uruk", 0, true);
    assert_eq!(game.pet_upkeep().total_levels, 250);
    assert_eq!(game.pet_upkeep().percent, 320);

    game.entities.clear();
    for ordinal in 0..13 {
        add_actor(&mut game, "demo.actor.coatl", ordinal, true);
    }
    assert_eq!(game.pet_upkeep().total_levels, 377);
    assert_eq!(game.pet_upkeep().percent, 511);
    assert!(game.pet_upkeep().unsafe_warning());
}

#[test]
fn upkeep_scales_positive_mana_recovery_and_drains_above_one_hundred_percent() {
    let mut game = high_mage_game(2);
    assert_eq!(
        game.player_resource_recovery_change("demo.resource.mana", false),
        2
    );

    add_actor(&mut game, "demo.actor.coatl", 0, true);
    assert_eq!(game.pet_upkeep().percent, 26);
    assert_eq!(
        game.player_resource_recovery_change("demo.resource.mana", false),
        1
    );

    for ordinal in 1..5 {
        add_actor(&mut game, "demo.actor.coatl", ordinal, true);
    }
    assert_eq!(game.pet_upkeep().percent, 163);
    assert_eq!(
        game.player_resource_recovery_change("demo.resource.mana", false),
        -1
    );
    game.resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana")
        .current = 1;
    let mut events = Vec::new();
    game.apply_pet_upkeep_mana_loss(&mut events);
    assert_eq!(game.resources["demo.resource.mana"].current, 0);
    assert!(game.pet_upkeep_dto().dismissal_required);
    assert!(matches!(
        events.as_slice(),
        [
            DomainEvent::PetUpkeepManaLost { amount: 1, .. },
            DomainEvent::PetUpkeepDismissalRequired {
                upkeep_percent: 163
            }
        ]
    ));
}

#[test]
fn dismiss_pets_is_zero_time_and_removes_every_controlled_actor() {
    let mut game = high_mage_game(3);
    add_actor(&mut game, "demo.actor.coatl", 0, true);
    add_actor(&mut game, "demo.actor.grey-mold", 1, true);
    let before = game.snapshot();
    let update = super::support::dispatch_next(&mut game, GameCommand::DismissPets);

    assert_eq!(update.world_tick, before.world_tick);
    assert_eq!(game.pet_upkeep().controlled_pets, 0);
    assert!(game.entities.is_empty());
    assert!(update.events.iter().any(|event| {
        event.message_key == "pets-dismissed"
            && event.args.get("count").map(String::as_str) == Some("2")
    }));
}

#[test]
fn advancing_actions_drain_excess_upkeep_and_zero_mana_interrupts_rest() {
    let mut game = high_mage_game(4);
    for ordinal in 0..5 {
        add_actor(&mut game, "demo.actor.coatl", ordinal, true);
    }
    game.resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana")
        .current = 2;
    let update = super::support::dispatch_next(&mut game, GameCommand::Search);
    assert_eq!(game.resources["demo.resource.mana"].current, 1);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.message_key == "pet-upkeep-mana-lost")
    );

    game.resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana")
        .current = 0;
    game.player.hp = game.player.hp.saturating_sub(1);
    let update = super::support::dispatch_next(&mut game, GameCommand::Rest { turns: 10 });
    assert_eq!(
        super::support::rest_resolution(&update).stop_reason,
        RestStopReasonDto::PetDismissalRequired
    );
}

#[test]
fn exactly_one_hundred_percent_upkeep_is_not_a_recoverable_rest_need() {
    let mut game = high_mage_game(5);
    for ordinal in 0..103 {
        add_actor(&mut game, "demo.actor.grey-mold", ordinal, true);
    }
    assert_eq!(game.pet_upkeep().percent, 100);
    game.resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana")
        .current = 0;
    let update = super::support::dispatch_next(&mut game, GameCommand::Rest { turns: 10 });
    let resolution = super::support::rest_resolution(&update);
    assert_eq!(resolution.completed_turns, 0);
    assert_eq!(resolution.stop_reason, RestStopReasonDto::FullResources);
}

#[test]
fn neglected_pet_checks_preserve_the_original_rng_gate_order() {
    let mut game = high_mage_game(195);
    add_actor(&mut game, "demo.actor.ugluk-the-uruk", 0, true);
    add_actor(&mut game, "demo.actor.ugluk-the-uruk", 1, true);
    game.resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana")
        .current = 0;
    game.rng = RfbRng::seeded(81);
    let draws_before = game.rng_draw_counter();
    let mut events = Vec::new();
    let disappeared =
        game.resolve_neglected_pet(0, true, &mut events, &mut BTreeSet::new(), &mut Vec::new());
    assert!(!disappeared);
    assert_eq!(game.rng_draw_counter() - draws_before, 7);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::PetNeglected {
            disappeared: false,
            ..
        }]
    ));
    assert!(
        game.entities
            .iter()
            .any(|actor| actor.id == "test.pet.0" && actor.controller_id.is_none())
    );
}
