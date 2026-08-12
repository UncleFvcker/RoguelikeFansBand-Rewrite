// SPDX-License-Identifier: MPL-2.0
use crate::game::movement::actor_can_cross_terrain;

use super::support::{game_with_actor_definition, replace_terrain};
use super::*;

#[test]
fn climber_crosses_mountain_and_glacier_terrain() {
    let game = Game::new(2);
    let brumby = game
        .content
        .actor("demo.actor.brumby")
        .expect("Brumby definition");
    let walker = game
        .content
        .actor("demo.actor.small-kobold")
        .expect("walker definition");

    for terrain_id in [
        "demo.terrain.surface-mountain",
        "demo.terrain.surface-glacier",
    ] {
        let terrain = game.content.terrain(terrain_id).expect("climb terrain");
        assert!(actor_can_cross_terrain(brumby, terrain));
        assert!(!actor_can_cross_terrain(walker, terrain));
    }
}

#[test]
fn aquatic_and_wall_passing_domains_remain_distinct() {
    let game = Game::new(11);
    let floor = game
        .content
        .terrain("demo.terrain.floor")
        .expect("floor definition");
    let wall = game
        .content
        .terrain("demo.terrain.wall")
        .expect("ordinary wall definition");
    let permanent_wall = game
        .content
        .terrain("demo.terrain.permanent-wall")
        .expect("permanent wall definition");
    let shallow_water = game
        .content
        .terrain("demo.terrain.surface-water-shallow")
        .expect("surface water definition");
    let aquatic = game
        .content
        .actor("demo.actor.piranha")
        .expect("aquatic actor definition");
    let wall_passer = game
        .content
        .actor("demo.actor.poltergeist")
        .expect("wall-passing actor definition");

    assert!(actor_can_cross_terrain(aquatic, shallow_water));
    assert!(!actor_can_cross_terrain(aquatic, floor));
    assert!(actor_can_cross_terrain(wall_passer, wall));
    assert!(!actor_can_cross_terrain(wall_passer, permanent_wall));
}

#[test]
fn kill_body_attacks_a_weaker_actor_blocking_the_next_step() {
    let mut game = Game::new(42);
    game.entities.clear();
    let origin = Position { x: 4, y: 3 };
    let blocker = Position { x: 5, y: 3 };
    for position in [origin, blocker] {
        replace_terrain(&mut game, position, "demo.terrain.floor");
    }
    game.push_generated_actor("test.slug".to_owned(), "demo.actor.giant-slug", origin);
    game.push_generated_actor("test.blocker".to_owned(), "demo.actor.sheep", blocker);
    let mut events = Vec::new();

    let outcome = game
        .move_entity(
            0,
            blocker,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("KILL_BODY interaction should resolve");

    assert_eq!(outcome, ActorStepOutcome::Interacted);
    assert_eq!(game.entities[0].position, origin);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MonsterMeleeEntityMissed { .. }
            | DomainEvent::MonsterMeleeEntityHit { .. }
            | DomainEvent::MonsterMeleeEntitySlew { .. }
    )));
}

#[test]
fn move_body_swaps_with_a_weaker_actor_and_wakes_it() {
    let mut game = game_with_actor_definition(42, "demo.actor.small-kobold", |actor| {
        actor.moves_weaker_bodies = true;
        actor.experience_value = 1_000;
    });
    game.entities.clear();
    let origin = Position { x: 4, y: 3 };
    let blocker = Position { x: 5, y: 3 };
    for position in [origin, blocker] {
        replace_terrain(&mut game, position, "demo.terrain.floor");
    }
    game.push_generated_actor("test.mover".to_owned(), "demo.actor.small-kobold", origin);
    game.push_generated_actor("test.blocker".to_owned(), "demo.actor.sheep", blocker);
    game.entities[1].statuses.push(StatusInstance {
        kind_id: STATUS_SLEEP.to_owned(),
        intensity: 1,
        remaining_ticks: 25,
        source_id: None,
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();

    let outcome = game
        .move_entity(0, blocker, &mut events, &mut changed, &mut Vec::new())
        .expect("MOVE_BODY swap should resolve");

    assert_eq!(outcome, ActorStepOutcome::Moved);
    assert_eq!(game.entities[0].position, blocker);
    assert_eq!(game.entities[1].position, origin);
    assert!(
        game.entities[1]
            .statuses
            .iter()
            .all(|status| status.kind_id != STATUS_SLEEP)
    );
    assert_eq!(changed, BTreeSet::from([origin, blocker]));
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::EntityAwakened { target_kind_id }
            if target_kind_id == "demo.actor.sheep"
    )));
}

#[test]
fn move_body_requires_the_displaced_actor_to_cross_the_origin_terrain() {
    let mut game = game_with_actor_definition(42, "demo.actor.small-kobold", |actor| {
        actor.moves_weaker_bodies = true;
        actor.experience_value = 1_000;
        actor
            .movement
            .modes
            .push(rfb_content::ActorMovementMode::Swim);
    });
    game.entities.clear();
    let origin = Position { x: 4, y: 3 };
    let blocker = Position { x: 5, y: 3 };
    replace_terrain(&mut game, origin, "demo.terrain.floor");
    replace_terrain(&mut game, blocker, "demo.terrain.surface-water-shallow");
    game.push_generated_actor("test.mover".to_owned(), "demo.actor.small-kobold", origin);
    game.push_generated_actor("test.blocker".to_owned(), "demo.actor.piranha", blocker);

    assert!(game.actor_can_enter_position(0, blocker));
    assert!(!game.actor_kind_can_enter_position("demo.actor.piranha", origin));
    assert!(!game.actor_can_move_body_blocker(0, 1));
}

#[test]
fn move_body_requires_strictly_greater_experience_value() {
    let mut game = game_with_actor_definition(42, "demo.actor.small-kobold", |actor| {
        actor.moves_weaker_bodies = true;
        actor.experience_value = 0;
    });
    game.entities.clear();
    let origin = Position { x: 4, y: 3 };
    let blocker = Position { x: 5, y: 3 };
    for position in [origin, blocker] {
        replace_terrain(&mut game, position, "demo.terrain.floor");
    }
    game.push_generated_actor("test.mover".to_owned(), "demo.actor.small-kobold", origin);
    game.push_generated_actor("test.blocker".to_owned(), "demo.actor.sheep", blocker);

    assert!(!game.actor_can_move_body_blocker(0, 1));
}

#[test]
fn monster_regeneration_is_shared_doubled_and_capped() {
    let mut ordinary = game_with_actor_definition(42, "demo.actor.small-kobold", |actor| {
        actor.regenerates = false;
    });
    ordinary.entities.clear();
    ordinary.push_generated_actor(
        "test.ordinary".to_owned(),
        "demo.actor.small-kobold",
        Position { x: 4, y: 3 },
    );
    ordinary.entities[0].hp = 1;
    ordinary.entities[0].max_hp = 250;
    ordinary.world_tick = MONSTER_REGENERATION_INTERVAL_TICKS - 1;
    ordinary.process_monster_regeneration();
    assert_eq!(ordinary.entities[0].hp, 1);
    ordinary.world_tick = MONSTER_REGENERATION_INTERVAL_TICKS;
    ordinary.process_monster_regeneration();
    assert_eq!(ordinary.entities[0].hp, 3);

    let mut fast = game_with_actor_definition(42, "demo.actor.small-kobold", |actor| {
        actor.regenerates = true;
    });
    fast.entities.clear();
    fast.push_generated_actor(
        "test.fast".to_owned(),
        "demo.actor.small-kobold",
        Position { x: 4, y: 3 },
    );
    fast.push_generated_actor(
        "test.capped".to_owned(),
        "demo.actor.small-kobold",
        Position { x: 5, y: 3 },
    );
    fast.entities[0].hp = 1;
    fast.entities[0].max_hp = 250;
    fast.entities[1].hp = 1;
    fast.entities[1].max_hp = 50_000;
    fast.world_tick = MONSTER_REGENERATION_INTERVAL_TICKS;
    let draws = fast.rng.draw_counter;
    fast.process_monster_regeneration();
    assert_eq!(fast.entities[0].hp, 5);
    assert_eq!(fast.entities[1].hp, 401);
    assert_eq!(fast.rng.draw_counter, draws);
}

#[test]
fn low_hp_monster_regeneration_uses_one_minimum_recovery_draw() {
    let mut game = game_with_actor_definition(42, "demo.actor.small-kobold", |actor| {
        actor.regenerates = false;
    });
    game.entities.clear();
    game.push_generated_actor(
        "test.low-hp".to_owned(),
        "demo.actor.small-kobold",
        Position { x: 4, y: 3 },
    );
    game.entities[0].hp = 1;
    game.entities[0].max_hp = 50;
    game.world_tick = MONSTER_REGENERATION_INTERVAL_TICKS;
    let draws = game.rng.draw_counter;

    game.process_monster_regeneration();

    assert!(matches!(game.entities[0].hp, 1 | 2));
    assert_eq!(game.rng.draw_counter, draws + 1);
}

#[test]
fn ranged_melee_uses_the_melee_routine_at_rfb_two_grid_reach() {
    let mut game = game_with_actor_definition(42, "demo.actor.small-kobold", |actor| {
        actor.ranged_melee = true;
    });
    game.entities.clear();
    let origin = Position { x: 4, y: 3 };
    game.player.position = Position { x: 6, y: 4 };
    for position in [origin, Position { x: 5, y: 4 }, game.player.position] {
        replace_terrain(&mut game, position, "demo.terrain.floor");
    }
    game.push_generated_actor(
        "test.ranged-melee".to_owned(),
        "demo.actor.small-kobold",
        origin,
    );
    game.entities[0].alerted = true;
    let mut events = Vec::new();

    game.resolve_monster_action(
        0,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
        &mut BTreeSet::new(),
    )
    .expect("ranged melee action should resolve");

    assert_eq!(game.entities[0].position, origin);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MonsterMeleeHit { .. } | DomainEvent::MonsterMeleeMissed { .. }
    )));
}

#[test]
fn living_trump_blinks_for_free_before_its_action() {
    let mut game = Game::new(0);
    game.entities.clear();
    game.push_generated_actor(
        "test.trump".to_owned(),
        "demo.actor.jurt-the-living-trump",
        Position { x: 4, y: 3 },
    );
    let origin = game.entities[0].position;
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();

    let seed = (0..100)
        .find(|seed| {
            let mut trial = game.clone();
            trial.rng = RfbRng::seeded(*seed);
            trial.try_trump_blink(0, &mut Vec::new(), &mut BTreeSet::new());
            trial.entities[0].position != origin
        })
        .expect("a deterministic seed should trigger the trump blink");
    game.rng = RfbRng::seeded(seed);
    game.try_trump_blink(0, &mut events, &mut changed);

    let destination = game.entities[0].position;
    assert_ne!(destination, origin);
    assert_eq!(changed, BTreeSet::from([origin, destination]));
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MonsterBlinked { source_kind_id, resolution }
            if source_kind_id == "demo.actor.jurt-the-living-trump"
                && resolution.from == origin
                && resolution.to == destination
    )));
}

#[test]
fn quantum_turn_uses_the_stable_entity_id_and_can_naturally_vanish() {
    assert_eq!(
        Game::quantum_slot_denominator("stable.quantum.1"),
        Game::quantum_slot_denominator("stable.quantum.1")
    );

    let mut game = Game::new(0);
    game.entities.clear();
    game.push_generated_actor(
        "stable.quantum.1".to_owned(),
        "demo.actor.quantum-dot",
        Position { x: 4, y: 3 },
    );
    let seed = (0..10_000)
        .find(|seed| {
            let mut trial = game.clone();
            trial.rng = RfbRng::seeded(*seed);
            let mut events = Vec::new();
            trial
                .try_quantum_turn(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
                .expect("quantum turn should resolve");
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::MonsterQuantumVanished { .. }))
        })
        .expect("a deterministic seed should trigger natural quantum disappearance");
    game.rng = RfbRng::seeded(seed);
    let mut events = Vec::new();
    let mut removed = Vec::new();

    assert!(
        game.try_quantum_turn(0, &mut events, &mut BTreeSet::new(), &mut removed)
            .expect("quantum turn should resolve")
    );
    assert!(game.entities.is_empty());
    assert_eq!(removed, ["stable.quantum.1"]);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MonsterQuantumVanished { source_kind_id }
            if source_kind_id == "demo.actor.quantum-dot"
    )));
}

#[test]
fn clear_head_has_a_one_in_four_chance_to_remove_monster_confusion() {
    let mut game = game_with_actor_definition(0, "demo.actor.small-kobold", |actor| {
        actor.tags.push("clear-head".to_owned());
    });
    game.entities.clear();
    game.push_generated_actor(
        "test.clear-head".to_owned(),
        "demo.actor.small-kobold",
        Position { x: 4, y: 3 },
    );
    game.entities[0].statuses.push(StatusInstance {
        kind_id: STATUS_CONFUSION.to_owned(),
        intensity: 1,
        remaining_ticks: 25,
        source_id: None,
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    let seed = (0..100)
        .find(|seed| {
            let mut trial = game.clone();
            trial.rng = RfbRng::seeded(*seed);
            trial.try_clear_monster_confusion(0, &mut Vec::new())
        })
        .expect("a deterministic seed should hit the one-in-four clear roll");
    game.rng = RfbRng::seeded(seed);
    let mut events = Vec::new();

    assert!(game.try_clear_monster_confusion(0, &mut events));
    assert!(game.entities[0].statuses.is_empty());
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::EntityStatusExpired { target_kind_id, status_kind_id }
            if target_kind_id == "demo.actor.small-kobold"
                && status_kind_id == STATUS_CONFUSION
    )));
}

#[test]
fn never_move_monster_waits_at_range_and_can_attack_adjacent() {
    let mut game = game_with_actor_definition(7, "demo.actor.grey-mold", |actor| {
        actor.attack = 1_000_000;
    });
    game.entities.clear();
    let origin = Position { x: 5, y: 3 };
    replace_terrain(&mut game, origin, "demo.terrain.floor");
    let distant_player = Position { x: 8, y: 3 };
    game.player.position = distant_player;
    replace_terrain(&mut game, distant_player, "demo.terrain.floor");
    game.push_generated_actor("test.grey-mold".to_owned(), "demo.actor.grey-mold", origin);
    game.entities[0].alerted = true;

    game.resolve_monster_action(
        0,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
        &mut BTreeSet::new(),
    )
    .expect("stationary monster action should resolve");
    assert_eq!(game.entities[0].position, origin);

    let adjacent_player = Position { x: 6, y: 3 };
    game.player.position = adjacent_player;
    replace_terrain(&mut game, adjacent_player, "demo.terrain.floor");
    let mut events = Vec::new();
    game.resolve_monster_action(
        0,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
        &mut BTreeSet::new(),
    )
    .expect("adjacent stationary monster action should resolve");

    assert_eq!(game.entities[0].position, origin);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MonsterMeleeHit { .. } | DomainEvent::MonsterMeleeMissed { .. }
    )));
}

#[test]
fn never_move_monster_can_still_blink() {
    let mut game = game_with_actor_definition(8, "demo.actor.blinking-dot", |actor| {
        actor
            .monster_casting
            .as_mut()
            .expect("Blinking Dot casting profile")
            .frequency_percent = 100;
    });
    game.entities.clear();
    for y in 2..=6 {
        for x in 2..=12 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    let origin = Position { x: 5, y: 3 };
    game.player.position = Position { x: 12, y: 3 };
    game.push_generated_actor(
        "test.blinking-dot".to_owned(),
        "demo.actor.blinking-dot",
        origin,
    );
    game.entities[0].alerted = true;
    let mut events = Vec::new();

    game.resolve_monster_action(
        0,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
        &mut BTreeSet::new(),
    )
    .expect("stationary caster should blink");

    assert_ne!(game.entities[0].position, origin);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::MonsterBlinked { .. }))
    );
}
