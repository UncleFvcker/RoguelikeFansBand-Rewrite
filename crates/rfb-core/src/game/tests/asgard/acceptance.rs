// SPDX-License-Identifier: MPL-2.0
use super::*;

const DUNGEON: &str = "demo.dungeon.asgard";
const HEIMDALL: &str = "demo.actor.heimdall-guardian-of-bifrost";
const ODIN: &str = "demo.actor.odin-the-all-father";
const VIDARR: &str = "demo.actor.vidarr-the-silent-avenger";

fn warrior() -> Game {
    let mut game = (0..32)
        .map(|seed| Game::new_with_build(seed, "demo.build.warrior").unwrap())
        .find(|game| game.active_pantheons & 8 != 0)
        .unwrap();
    choose_human_talent_if_pending(&mut game);
    game
}

fn prepared() -> Game {
    let mut game = warrior();
    game.debug_prepare_asgard_e2e("arrival", None).unwrap();
    choose_human_talent_if_pending(&mut game);
    game
}

fn battle(game: &mut Game, kind: &str, level: u16) {
    let actor = game
        .entities
        .iter()
        .find(|actor| actor.kind_id == kind)
        .unwrap();
    let id = actor.id.clone();
    let hp = actor.hp;
    let max_hp = actor.max_hp;
    let definition = game.actor_runtime_definition(actor).unwrap().clone();
    assert_eq!(definition.level, level);
    assert_eq!(hp, max_hp, "full source HP before prepared combat");
    game.debug_prepare_asgard_e2e("battle", Some(&id)).unwrap();
    let prepared = game.entities.iter().find(|actor| actor.id == id).unwrap();
    assert_eq!((prepared.hp, prepared.max_hp), (hp, max_hp));
    assert_eq!(
        game.actor_runtime_definition(prepared).unwrap(),
        &definition
    );
    let mut hits = 0;
    for _ in 0..512 {
        let Some(actor) = game.entities.iter().find(|actor| actor.id == id) else {
            assert!(hits > 0, "victory needs player attack events");
            assert!(!game.unique_actor_kind_is_available(kind));
            return;
        };
        let mut delta = (
            actor.position.x - game.player.position.x,
            actor.position.y - game.player.position.y,
        );
        if delta.0.abs().max(delta.1.abs()) > 1 {
            game.debug_prepare_asgard_e2e("battle", Some(&id)).unwrap();
            let actor = game.entities.iter().find(|actor| actor.id == id).unwrap();
            delta = (
                actor.position.x - game.player.position.x,
                actor.position.y - game.player.position.y,
            );
        }
        let direction = [
            Direction::North,
            Direction::NorthEast,
            Direction::East,
            Direction::SouthEast,
            Direction::South,
            Direction::SouthWest,
            Direction::West,
            Direction::NorthWest,
        ]
        .into_iter()
        .find(|direction| direction.delta() == delta)
        .unwrap();
        let update = dispatch_next(game, GameCommand::Move { direction });
        hits += update
            .events
            .iter()
            .filter(|event| event.kind == "combat.hit")
            .count();
        assert!(!game.player_is_dead());
        choose_human_talent_if_pending(game);
    }
    panic!("full-HP {kind} survived 512 prepared melee actions");
}

fn stairs(game: &mut Game, terrain: &str, expected: &str) {
    game.debug_prepare_asgard_e2e("route", None).unwrap();
    place_player_on_terrain(game, terrain);
    assert_eq!(
        dispatch_next(game, GameCommand::TraverseStairs).floor_id,
        expected
    );
    game.debug_prepare_asgard_e2e("route", None).unwrap();
}

fn pickup(game: &mut Game, kind: &str) -> String {
    let item = game.items.iter().find(|item| item.kind_id == kind).unwrap();
    let id = item.id.clone();
    let ItemLocation::Ground(position) = item.location else {
        panic!("reward must be on ground");
    };
    game.player.position = position;
    game.pick_up_item_at_player(Some(&id)).unwrap();
    id
}

#[test]
fn asgard_prepared_full_source_guardians_route_rewards_return_and_recall_resume() {
    // Shared with desktop preparation: new Human Warrior, source XP to 50,
    // +100/+100 broad sword, long levitation/invulnerability/see-invisible,
    // full player HP and explicit route/position prep; normal enemy AI continues.
    // No guardian HP/definition changes and no assignment of conquest flags.
    let mut game = prepared();
    let arrival_hash = game.state_hash();
    game = Game::from_save(game.to_save()).unwrap();
    assert_eq!(game.state_hash(), arrival_hash);
    assert_eq!(game.wilderness_position, Some(Position { x: 94, y: 11 }));
    assert_eq!(game.progress.level, 50);
    assert_eq!(
        game.entities
            .iter()
            .find(|actor| actor.kind_id == HEIMDALL)
            .unwrap()
            .id,
        "demo.guardian.asgard-entrance.1"
    );
    battle(&mut game, HEIMDALL, 77);
    assert!(game.dungeon_states[DUNGEON].entrance_guardian_defeated);
    assert!(!game.dungeon_states[DUNGEON].guardian_defeated);
    place_player_on_terrain(&mut game, "demo.terrain.asgard-entrance");
    let departure = game.player.position;
    stairs(
        &mut game,
        "demo.terrain.asgard-entrance",
        "demo.floor.asgard-depth-64",
    );
    for depth in [68, 72, 76, 80, 82, 84, 86, 88] {
        stairs(
            &mut game,
            "demo.terrain.shaft-down",
            &format!("demo.floor.asgard-depth-{depth}"),
        );
        if depth == 80 {
            game = Game::from_save(game.to_save()).unwrap();
        }
    }
    battle(&mut game, ODIN, 90);
    assert!(game.dungeon_states[DUNGEON].guardian_defeated);
    let vidarr = game
        .entities
        .iter()
        .find(|actor| actor.kind_id == VIDARR)
        .unwrap()
        .clone();
    // Save immediately after Odin, then exercise the same route-clear used by
    // the desktop. It must preserve the actual avenger and its full HP.
    game = Game::from_save(game.to_save()).unwrap();
    game.debug_prepare_asgard_e2e("route", None).unwrap();
    let retained = game
        .entities
        .iter()
        .find(|actor| actor.id == vidarr.id)
        .unwrap();
    assert_eq!(
        (retained.kind_id.as_str(), retained.hp, retained.max_hp),
        (VIDARR, vidarr.hp, vidarr.max_hp)
    );
    battle(&mut game, VIDARR, 87);
    assert_eq!(game.defeated_limited_actor_counts[VIDARR], 1);
    assert_eq!(
        game.items
            .iter()
            .filter(|item| item.kind_id == "demo.item.acquirement-scroll")
            .count(),
        1
    );
    let rune = pickup(&mut game, "demo.item.runespear");
    let scroll = pickup(&mut game, "demo.item.acquirement-scroll");
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: scroll.clone(),
            target: None,
        },
    );
    assert!(!game.items.iter().any(|item| item.id == scroll));
    assert!(
        game.items
            .iter()
            .any(|item| item.origin_kind == Some(rfb_protocol::ItemOriginKindDto::Acquire))
    );
    game = Game::from_save(game.to_save()).unwrap();
    for depth in [86, 84, 82, 80, 76, 72, 68, 64] {
        stairs(
            &mut game,
            "demo.terrain.shaft-up",
            &format!("demo.floor.asgard-depth-{depth}"),
        );
    }
    stairs(
        &mut game,
        "demo.terrain.stairs-up",
        wilderness::WILDERNESS_FLOOR_ID,
    );
    assert_eq!(game.wilderness_position, Some(Position { x: 94, y: 11 }));
    assert_eq!(game.player.position, departure);
    for (recall, expected) in [
        ("e2e.asgard.recall.1", "demo.floor.asgard-depth-88"),
        ("e2e.asgard.recall.2", wilderness::WILDERNESS_FLOOR_ID),
    ] {
        dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: recall.into(),
                target: None,
            },
        );
        let mut restored = Game::from_save(game.to_save()).unwrap();
        for current in [&mut game, &mut restored] {
            for _ in 0..40 {
                if current.current_floor_id == expected {
                    break;
                }
                current.debug_prepare_asgard_e2e("route", None).unwrap();
                dispatch_next(current, GameCommand::Wait);
            }
            assert_eq!(current.current_floor_id, expected);
            assert!(
                !current
                    .entities
                    .iter()
                    .any(|actor| [HEIMDALL, ODIN, VIDARR].contains(&actor.kind_id.as_str()))
            );
            assert!(
                !current
                    .items
                    .iter()
                    .any(|item| item.kind_id == "demo.item.acquirement-scroll")
            );
            assert_eq!(
                current.items.iter().filter(|item| item.id == rune).count(),
                1
            );
        }
        assert_eq!(game.state_hash(), restored.state_hash());
    }
    assert_eq!(game.player.position, departure);
}

#[test]
fn asgard_fixture_rejects_wrong_context_and_preserves_live_avenger_during_clear() {
    let mut game = warrior();
    let before = game.state_hash();
    assert!(game.debug_prepare_asgard_e2e("route", None).is_err());
    assert!(
        game.debug_prepare_asgard_e2e("battle", Some("test.missing"))
            .is_err()
    );
    assert_eq!(game.state_hash(), before);
    game.debug_prepare_asgard_e2e("arrival", None).unwrap();
    let before = game.state_hash();
    assert!(game.debug_prepare_asgard_e2e("arrival", None).is_err());
    assert!(
        game.debug_prepare_asgard_e2e("battle", Some("test.missing"))
            .is_err()
    );
    assert_eq!(game.state_hash(), before);
    game.push_generated_actor("test.avenger".into(), VIDARR, Position { x: 101, y: 33 });
    game.push_generated_actor(
        "test.unrelated".into(),
        "demo.actor.newt",
        Position { x: 102, y: 33 },
    );
    let avenger = game
        .entities
        .iter()
        .find(|actor| actor.id == "test.avenger")
        .unwrap()
        .clone();
    game.debug_prepare_asgard_e2e("route", None).unwrap();
    assert!(
        !game
            .entities
            .iter()
            .any(|actor| actor.id == "test.unrelated")
    );
    let retained = game
        .entities
        .iter()
        .find(|actor| actor.id == "test.avenger")
        .unwrap();
    assert_eq!(
        (retained.hp, retained.max_hp, &retained.kind_id),
        (avenger.hp, avenger.max_hp, &avenger.kind_id)
    );
    assert_eq!(retained.energy_need, avenger.energy_need);
    assert_eq!(retained.statuses, avenger.statuses);
}

#[test]
fn asgard_source_guardians_and_avenger_execute_their_real_melee_blows() {
    let base = prepared();
    for kind in [HEIMDALL, ODIN, VIDARR] {
        let mut game = base.clone();
        clear_monsters(&mut game);
        game.player.statuses.clear();
        // Isolate incoming source blows from the desktop's invulnerability.
        // Excess player HP is only for this unsaved combat fixture.
        game.player.hp = 100_000;
        game.push_generated_actor(
            "test.asgard.attacker".into(),
            kind,
            Position {
                x: game.player.position.x + 1,
                y: game.player.position.y,
            },
        );
        let target = MonsterHostileTarget::Player {
            entity_id: game.player.id.clone(),
            kind_id: game.player.kind_id.clone(),
            position: game.player.position,
        };
        for _ in 0..8 {
            game.resolve_monster_melee_target(
                0,
                &target,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        }
        assert!(
            game.player.hp < 100_000,
            "source {kind} must damage the unprotected player"
        );
    }
}
