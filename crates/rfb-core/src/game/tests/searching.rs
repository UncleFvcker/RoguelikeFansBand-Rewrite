// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

fn at(x: i32, y: i32) -> Position {
    Position {
        x: 90 + x,
        y: 30 + y,
    }
}

fn arena() -> Game {
    let mut game = Game::new_with_build(509, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.gold_piles.clear();
    game.terrain.fill("demo.terrain.wall".into());
    game.explored.fill(true);
    for x in 1..=20 {
        replace_terrain(&mut game, at(x, 1), "demo.terrain.floor");
    }
    game.player.position = at(1, 1);
    game.mogaminator.enabled = false;
    game.reveal_current_visibility();
    game
}

fn trapped_chest(game: &mut Game, position: Position) {
    give_inventory_item(game, "test.search.chest", "demo.item.large-wooden-chest");
    let chest = game.items.last_mut().unwrap();
    chest.location = ItemLocation::Ground(position);
    chest.chest = Some(rfb_protocol::ChestSaveDto {
        difficulty: 1,
        opening_depth: 1,
    });
    game.item_property_knowledge
        .entry("test.search.chest".into())
        .or_default()
        .discovered = true;
    game.glow.fill(true);
    game.rng = RfbRng::seeded(
        (0..1000)
            .find(|seed| {
                RfbRng::seeded(*seed).bounded(100)
                    < game.player_derived_stats().search_skill.value.max(0) as u64
            })
            .unwrap(),
    );
}

#[test]
fn toggle_is_free_preserves_manual_search_and_round_trips() {
    let mut game = arena();
    let before = (
        game.turn,
        game.world_tick,
        game.player.energy_need,
        game.rng.clone(),
    );
    let old_hash = game.state_hash();
    let speed = game.player_derived_stats().speed.value;
    let update = dispatch_next(&mut game, GameCommand::ToggleSearch);
    assert!(update.player.searching);
    assert_eq!(
        before,
        (
            game.turn,
            game.world_tick,
            game.player.energy_need,
            game.rng.clone()
        )
    );
    assert_eq!(game.player_derived_stats().speed.value, speed - 10);
    assert_ne!(old_hash, game.state_hash());
    let mut restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    for command in [
        GameCommand::Search,
        GameCommand::CancelRun,
        GameCommand::CancelAutoExplore,
        GameCommand::CancelFishing,
    ] {
        assert_eq!(
            dispatch_next(&mut game, command.clone()),
            dispatch_next(&mut restored, command)
        );
        assert!(game.searching);
    }
    assert_eq!(game.turn, before.0 + 1, "only manual search spends a turn");
    dispatch_next(&mut game, GameCommand::ToggleSearch);
    assert!(!game.searching);
    assert_eq!(game.player_derived_stats().speed.value, speed);
}

#[test]
fn real_step_searches_once_but_off_failed_steps_and_non_energy_displacement_do_not() {
    for mode in ["on", "off", "blocked", "teleport", "charge"] {
        let mut game = arena();
        trapped_chest(&mut game, at(2, 2));
        game.searching = mode != "off";
        let mut events = vec![];
        let mut changed = BTreeSet::new();
        let before = game.rng.clone();
        match mode {
            "teleport" => {
                events.extend(game.relocate_player(at(2, 1), &mut changed));
            }
            "charge" => {
                game.enter_player_position(
                    at(2, 1),
                    false,
                    false,
                    &mut events,
                    &mut changed,
                    &mut vec![],
                )
                .unwrap();
            }
            _ => {
                game.resolve_local_player_step(
                    if mode == "blocked" {
                        Direction::North
                    } else {
                        Direction::East
                    },
                    false,
                    &mut events,
                    &mut changed,
                    &mut vec![],
                )
                .unwrap();
            }
        }
        let mut expected = before;
        if mode == "on" {
            expected.bounded(100);
        }
        assert_eq!(game.rng, expected, "{mode}: exactly one chest search roll");
        assert_eq!(events.iter().filter(|event| matches!(event, DomainEvent::ChestInteracted { message_key } if message_key == "chest-trap-found")).count(), usize::from(mode == "on"));
    }
}

#[test]
fn stay_searches_and_discoveries_stop_run_and_exploration_without_switching_off() {
    for command in [
        GameCommand::Wait,
        GameCommand::Run {
            max_steps: None,
            direction: Direction::East,
        },
        GameCommand::AutoExplore,
        GameCommand::TravelLocal {
            destination: at(10, 1),
        },
    ] {
        let mut game = arena();
        trapped_chest(&mut game, at(2, 2));
        game.searching = true;
        if command == GameCommand::AutoExplore {
            for x in 10..=20 {
                let index = game.index(at(x, 1)).unwrap();
                game.explored[index] = false;
            }
        }
        let update = dispatch_next(&mut game, command.clone());
        assert!(
            update
                .events
                .iter()
                .any(|event| event.message_key == "chest-trap-found"),
            "{command:?}"
        );
        assert!(game.searching);
        assert!(game.running.is_none());
        assert!(game.auto_explore.is_none());
        assert_eq!(game.turn, 1);
    }
}

#[test]
fn terrain_search_reuses_the_manual_checks_and_reports_hidden_doors_and_traps() {
    let mut game = arena();
    replace_terrain(&mut game, at(2, 0), "demo.terrain.door-secret");
    replace_terrain(&mut game, at(2, 2), "demo.terrain.created-trap");
    let seed = (0..1000)
        .find(|seed| {
            let mut trial = game.clone();
            trial.rng = RfbRng::seeded(*seed);
            trial.search_hidden_terrain().len() == 2
        })
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    let mut expected = game.clone();
    let discovered = expected.search_hidden_terrain();
    assert_eq!(discovered.len(), 2);
    let mut events = vec![];
    let mut changed = BTreeSet::new();
    game.search_surroundings(&mut events, &mut changed);
    assert_eq!(game.rng, expected.rng);
    assert_eq!(game.revealed_terrain, expected.revealed_terrain);
    assert_eq!(events.len(), discovered.len());
}

#[test]
fn searching_slows_world_and_monster_energy_but_light_speed_overrides_on_foot_only() {
    let mut normal = arena();
    normal.push_generated_actor("test.sleeper".into(), "demo.actor.war-bear", at(15, 1));
    normal.entities[0].speed = 93;
    let mut searching = normal.clone();
    searching.searching = true;
    dispatch_next(
        &mut normal,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    dispatch_next(
        &mut searching,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert!(searching.world_tick > normal.world_tick);
    assert_ne!(
        searching.entities[0].energy_need,
        normal.entities[0].energy_need
    );
    let mut game = arena();
    game.progress.level = 50;
    for _ in 0..1000 {
        game.resolve_wild_weapon_strike("test.light-speed", &mut vec![]);
        if game.player_has_status_kind(STATUS_LIGHT_SPEED) {
            break;
        }
    }
    let light = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == crate::effect::STATUS_LIGHT_SPEED)
        .unwrap()
        .clone();
    game.player.statuses = vec![light];
    let speed = game.player_derived_stats().speed.value;
    game.searching = true;
    assert_eq!(game.player_derived_stats().speed.value, speed);
    game.push_generated_actor(
        "test.mount".into(),
        "demo.actor.war-bear",
        game.player.position,
    );
    game.riding_actor_id = Some("test.mount".into());
    let searching_speed = game.player_derived_stats().speed.value;
    game.searching = false;
    assert_eq!(
        game.player_derived_stats().speed.value,
        searching_speed + 10
    );
}

#[test]
fn rest_and_damage_close_search() {
    let mut game = arena();
    game.searching = true;
    dispatch_next(&mut game, GameCommand::Rest { turns: 1 });
    assert!(!game.searching);
    game.searching = true;
    let damage = DamageOutcome {
        raw: 1,
        requested: 1,
        applied: 1,
        armor_reduction: 0,
        resistance_delta: 0,
        damage_type: DamageType::Physical,
        resistance: ResistanceLevel::Normal,
    };
    game.apply_final_player_damage(damage, super::super::damage::FatalityPolicy::BelowZero);
    assert!(!game.searching);
}

#[test]
fn periodic_damage_closes_search_at_the_damage_tick() {
    let mut game = arena();
    game.searching = true;
    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_BLEEDING, 2, "test.bleeding").status);
    let before = game.player.hp;
    dispatch_next(&mut game, GameCommand::Wait);
    assert!(game.player.hp < before);
    assert!(!game.searching);
}
