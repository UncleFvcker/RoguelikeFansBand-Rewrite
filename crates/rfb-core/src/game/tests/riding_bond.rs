// SPDX-License-Identifier: MPL-2.0

use super::support::{clear_monsters, give_inventory_item};
use super::*;

fn bonded_horse(seed: u64, bond: u16) -> Game {
    let mut game =
        Game::new_with_build(seed, "demo.build.cavalry").expect("Cavalry build should create");
    clear_monsters(&mut game);
    game.push_generated_actor(
        "test.mount".to_owned(),
        "demo.actor.horse",
        game.player.position,
    );
    game.entities[0].controller_id = Some(game.player.id.clone());
    game.riding_actor_id = Some("test.mount".to_owned());
    game.riding_bond = Some(RidingBond {
        actor_id: "test.mount".to_owned(),
        actor_kind_id: "demo.actor.horse".to_owned(),
        value: bond,
    });
    game
}

#[test]
fn pet_experience_evolves_in_place_and_resets_the_bond() {
    let mut game = bonded_horse(0x4556_4f4c_5645, 7_500);
    game.entities[0].experience = 69;
    game.entities[0].hp = game.entities[0].max_hp / 2;
    let previous_max_hp = game.entities[0].max_hp;
    let mut events = Vec::new();

    game.grant_pet_experience("test.mount", 1, &mut events);

    let horse = &game.entities[0];
    assert_eq!(horse.id, "test.mount");
    assert_eq!(horse.kind_id, "demo.actor.unruly-horse");
    assert_eq!(horse.experience, 0);
    assert_eq!(
        horse.controller_id.as_deref(),
        Some(game.player.id.as_str())
    );
    assert_eq!(game.riding_actor_id.as_deref(), Some("test.mount"));
    assert!(horse.max_hp != previous_max_hp);
    assert!(horse.hp > 0 && horse.hp <= horse.max_hp);
    assert_eq!(
        game.riding_bond,
        Some(RidingBond {
            actor_id: "test.mount".to_owned(),
            actor_kind_id: "demo.actor.unruly-horse".to_owned(),
            value: 0,
        })
    );
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::PetEvolved { .. }]
    ));

    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("evolved mount should round-trip");
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn a_kill_that_completes_the_bond_uses_full_experience_sharing() {
    let mut game = bonded_horse(0x424f_4e44, 9_995);
    let mut ordinary = bonded_horse(0x424f_4e44, 9_994);
    let mut target = game.entities[0].clone();
    target.id = "test.target".to_owned();
    target.controller_id = None;
    let pet_level = game
        .actor_runtime_definition(&game.entities[0])
        .expect("Horse definition should exist")
        .level;
    let expected = game.pet_experience_reward(pet_level, &target);
    assert!(expected > 0);
    let mut events = Vec::new();

    game.reward_controlled_actor_kill("test.mount", &target, &mut events);
    ordinary.reward_controlled_actor_kill("test.mount", &target, &mut Vec::new());

    assert_eq!(
        game.riding_bond.as_ref().map(|bond| bond.value),
        Some(10_000)
    );
    assert_eq!(game.entities[0].experience, expected);
    assert_eq!(
        ordinary.entities[0].experience,
        expected.saturating_sub(expected / 5)
    );
    assert!(game.progress.experience > ordinary.progress.experience);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::RidingBondMaxed { .. }))
    );
}

#[test]
fn riding_bond_potions_use_the_current_mount_and_keep_haste_rng_exact() {
    let mut game = bonded_horse(0x504f_5449_4f4e, 2_499);
    game.entities[0].hp = 1;
    give_inventory_item(
        &mut game,
        "test.light-healing",
        "demo.item.light-healing-potion",
    );
    let hidden = game
        .inventory_dto()
        .into_iter()
        .find(|item| item.id == "test.light-healing")
        .expect("healing potion should project");
    assert!(!hidden.mount_usable);

    game.riding_bond.as_mut().expect("bond").value = 2_500;
    let target = TargetSelection::Entity {
        entity_id: "test.mount".to_owned(),
    };
    let mut events = Vec::new();
    game.use_inventory_item(
        "test.light-healing",
        Some(&target),
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("mount healing should resolve");
    assert!(game.entities[0].hp > 1);
    assert!(
        !game
            .items
            .iter()
            .any(|item| item.id == "test.light-healing")
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::MountPotionUsed { .. }))
    );

    game.riding_bond.as_mut().expect("bond").value = 5_000;
    give_inventory_item(&mut game, "test.speed-one", "demo.item.swiftstep-tonic");
    game.use_inventory_item(
        "test.speed-one",
        Some(&target),
        None,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("first mount speed potion should resolve");
    let first_duration = game.entities[0]
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_HASTE)
        .expect("mount should be hasted")
        .remaining_ticks;
    assert!((16..=40).contains(&first_duration));

    give_inventory_item(&mut game, "test.speed-two", "demo.item.swiftstep-tonic");
    let rng_before = game.rng.clone();
    game.use_inventory_item(
        "test.speed-two",
        Some(&target),
        None,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("second mount speed potion should resolve");
    assert_eq!(game.rng, rng_before);
    let second_duration = game.entities[0]
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_HASTE)
        .expect("mount should remain hasted")
        .remaining_ticks;
    assert_eq!(second_duration, first_duration + 5);
}

#[test]
fn mount_enabled_potions_remain_usable_by_the_player_without_a_target() {
    let mut game = bonded_horse(0x504c_4159_4552, 5_000);
    game.player.hp = 1;
    give_inventory_item(
        &mut game,
        "test.player-healing",
        "demo.item.light-healing-potion",
    );

    game.use_inventory_item(
        "test.player-healing",
        None,
        None,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("ordinary potion use should still resolve");

    assert!(game.player.hp > 1);
    assert!(
        !game
            .items
            .iter()
            .any(|item| item.id == "test.player-healing")
    );
}
