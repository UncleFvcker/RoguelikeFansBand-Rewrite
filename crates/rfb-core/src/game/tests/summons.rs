// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

#[test]
fn summon_commands_are_zero_world_time_persistent_and_guard_the_issue_position() {
    let mut game = Game::new(89);
    clear_monsters(&mut game);
    add_player_summon(
        &mut game,
        "test.summon.echo-companion.1",
        Position { x: 4, y: 3 },
        5,
    );
    let before = game.to_save();
    let update = dispatch_next(
        &mut game,
        GameCommand::SetSummonCommand {
            mode: SummonCommandModeDto::Guard,
        },
    );

    assert_eq!(update.world_tick, before.world_tick);
    assert_eq!(update.player.energy_need, before.player.energy_need);
    assert_eq!(game.rng.draw_counter, before.rng.draw_counter);
    assert_eq!(
        update.player.summon_command,
        SummonCommandDto {
            mode: SummonCommandModeDto::Guard,
            guard_position: Some(Position { x: 3, y: 3 }),
        }
    );
    let resolution = update
        .events
        .iter()
        .find_map(|event| match &event.outcome {
            Some(GameEventOutcomeDto::SummonCommand { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("summon command should have a structured outcome");
    assert_eq!(resolution.affected_summons, 1);
    let restored = Game::from_save(game.to_save()).expect("summon command should round-trip");
    assert_eq!(restored.summon_command, game.summon_command);

    let mut malformed = game.to_save();
    malformed.player.summon_command.mode = SummonCommandModeDto::Follow;
    assert!(matches!(
        Game::from_save(malformed),
        Err(CoreError::InvalidSave(
            "non-guard summon command retains a guard position"
        ))
    ));
}

#[test]
fn player_summons_follow_attack_keep_distance_and_guard_deterministically() {
    let resolve = |mode: SummonCommandModeDto,
                   summon_position: Position,
                   guard_position: Option<Position>| {
        let mut game = Game::new(89);
        clear_monsters(&mut game);
        add_player_summon(
            &mut game,
            "test.summon.echo-companion.1",
            summon_position,
            5,
        );
        game.push_generated_actor(
            "test.monster.ember-mote.1".to_owned(),
            "demo.actor.ember-mote",
            Position { x: 10, y: 3 },
        );
        game.summon_command = SummonCommandDto {
            mode,
            guard_position,
        };
        let rng_before = game.rng.draw_counter;
        let mut changed = BTreeSet::new();
        game.resolve_player_summon_action(0, &mut Vec::new(), &mut changed, &mut Vec::new())
            .expect("summon action should resolve");
        (
            game.entities[0].position,
            changed,
            game.rng.draw_counter - rng_before,
        )
    };

    let (follow, _, follow_rng) =
        resolve(SummonCommandModeDto::Follow, Position { x: 7, y: 3 }, None);
    assert_eq!(follow, Position { x: 6, y: 3 });
    assert_eq!(follow_rng, 0);

    let (attack, _, attack_rng) =
        resolve(SummonCommandModeDto::Attack, Position { x: 7, y: 3 }, None);
    assert_eq!(attack, Position { x: 8, y: 3 });
    assert_eq!(attack_rng, 0);

    let (keep_distance, _, keep_distance_rng) = resolve(
        SummonCommandModeDto::KeepDistance,
        Position { x: 4, y: 3 },
        None,
    );
    assert_eq!(keep_distance, Position { x: 5, y: 2 });
    assert_eq!(keep_distance_rng, 0);

    let (guard, _, guard_rng) = resolve(
        SummonCommandModeDto::Guard,
        Position { x: 7, y: 3 },
        Some(Position { x: 3, y: 3 }),
    );
    assert_eq!(guard, Position { x: 6, y: 3 });
    assert_eq!(guard_rng, 0);
}

#[test]
fn attacking_summon_uses_actor_melee_and_player_owned_death_credit() {
    let seed = (1..=256)
        .find(|seed| {
            let mut game = Game::new(*seed);
            clear_monsters(&mut game);
            add_player_summon(
                &mut game,
                "test.summon.echo-companion.1",
                Position { x: 4, y: 3 },
                5,
            );
            let mut target = game.generated_actor(
                "test.monster.ember-mote.1".to_owned(),
                "demo.actor.ember-mote",
                Position { x: 5, y: 3 },
            );
            target.hp = 1;
            game.entities.push(target);
            game.summon_command.mode = SummonCommandModeDto::Attack;
            let mut events = Vec::new();
            game.resolve_player_summon_action(
                0,
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .expect("summon melee should resolve");
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::SummonSlew { .. }))
        })
        .expect("a bounded deterministic seed should let the summon hit");
    let mut game = Game::new(seed);
    clear_monsters(&mut game);
    add_player_summon(
        &mut game,
        "test.summon.echo-companion.1",
        Position { x: 4, y: 3 },
        5,
    );
    let mut target = game.generated_actor(
        "test.monster.ember-mote.1".to_owned(),
        "demo.actor.ember-mote",
        Position { x: 5, y: 3 },
    );
    target.hp = 1;
    game.entities.push(target);
    game.summon_command.mode = SummonCommandModeDto::Attack;
    let experience_before = game.progress.experience;
    let mut events = Vec::new();
    let mut removed = Vec::new();
    game.resolve_player_summon_action(0, &mut events, &mut BTreeSet::new(), &mut removed)
        .expect("summon melee should resolve");

    assert_eq!(removed, ["test.monster.ember-mote.1"]);
    assert!(game.progress.experience > experience_before);
    assert!(events.iter().any(|event| {
        matches!(
            event,
            DomainEvent::SummonSlew {
                source_kind_id,
                target_kind_id,
                ..
            } if source_kind_id == "demo.actor.echo-companion"
                && target_kind_id == "demo.actor.ember-mote"
        )
    }));
}

#[test]
fn nearby_player_summons_follow_across_floors_while_distant_summons_stay() {
    let mut game = Game::new(89);
    clear_monsters(&mut game);
    game.player.position = Position { x: 3, y: 4 };
    add_player_summon(&mut game, "test.summon.near", Position { x: 4, y: 4 }, 5);
    add_player_summon(
        &mut game,
        "test.summon.distant",
        Position { x: 10, y: 10 },
        5,
    );

    let transition = game
        .traverse_stairs(false)
        .expect("floor traversal should resolve")
        .expect("entrance should transition");
    assert_eq!(
        transition.summons_followed,
        [(
            "test.summon.near".to_owned(),
            "demo.actor.echo-companion".to_owned()
        )]
    );
    assert!(transition.summons_could_not_follow.is_empty());
    assert!(game.entities.iter().any(|entity| {
        entity.id == "test.summon.near"
            && chebyshev_distance(entity.position, game.player.position) <= 5
    }));
    assert!(
        stored_floor(&game, "demo.floor.surface")
            .entities
            .iter()
            .any(|entity| entity.id == "test.summon.distant")
    );
    assert!(
        stored_floor(&game, "demo.floor.surface")
            .entities
            .iter()
            .all(|entity| entity.id != "test.summon.near")
    );
}
