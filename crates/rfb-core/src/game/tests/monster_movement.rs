// SPDX-License-Identifier: MPL-2.0
use crate::game::movement::{actor_avoids_terrain_trap, actor_can_cross_terrain};

use super::support::{game_with_actor_definition, give_inventory_item, replace_terrain};
use super::*;

#[test]
fn movement_profile_controls_non_walkable_terrain_entry() {
    let game = Game::new(1);
    let deep_water = game
        .content
        .terrain("demo.terrain.resonance-water-deep")
        .expect("deep water definition");
    let walker = game
        .content
        .actor("demo.actor.small-kobold")
        .expect("walking actor definition");
    let swimmer = game
        .content
        .actor("demo.actor.newt")
        .expect("swimming actor definition");
    let flyer = game
        .content
        .actor("demo.actor.fruit-bat")
        .expect("flying actor definition");

    assert!(!actor_can_cross_terrain(walker, deep_water));
    assert!(actor_can_cross_terrain(swimmer, deep_water));
    assert!(actor_can_cross_terrain(flyer, deep_water));
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
fn trap_avoidance_requires_an_explicit_matching_movement_mode() {
    let game = Game::new(2);
    let mut trap = game
        .content
        .terrain("demo.terrain.trap-echo-snare")
        .expect("trap definition")
        .clone();
    trap.trap
        .as_mut()
        .expect("trap behavior")
        .avoided_by_movement_modes = vec![rfb_content::ActorMovementMode::Fly];
    let flyer = game
        .content
        .actor("demo.actor.fruit-bat")
        .expect("flying actor definition");
    let swimmer = game
        .content
        .actor("demo.actor.newt")
        .expect("swimming actor definition");

    assert!(actor_avoids_terrain_trap(flyer, &trap));
    assert!(!actor_avoids_terrain_trap(swimmer, &trap));
}

#[test]
fn entering_a_non_avoided_trap_applies_damage_to_the_monster() {
    let mut game = Game::new(3);
    game.entities.clear();
    let origin = Position { x: 4, y: 3 };
    let trap = Position { x: 5, y: 3 };
    replace_terrain(&mut game, origin, "demo.terrain.floor");
    replace_terrain(&mut game, trap, "demo.terrain.trap-echo-snare");
    game.push_generated_actor(
        "test.trap-actor".to_owned(),
        "demo.actor.small-kobold",
        origin,
    );
    game.entities[0].hp = 10;
    game.entities[0].max_hp = 10;
    let mut events = Vec::new();

    let outcome = game
        .move_entity(0, trap, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("trap movement should resolve");

    assert_eq!(outcome, ActorStepOutcome::Moved);
    assert_eq!(game.entities[0].position, trap);
    assert_eq!(game.entities[0].hp, 8);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::ActorTrapTriggered { .. }]
    ));
}

#[test]
fn wall_destroyer_transforms_the_wall_and_enters_the_cell() {
    let mut game = game_with_actor_definition(4, "demo.actor.echo-hound", |actor| {
        actor.terrain_interaction.destroys_walls = true;
    });
    game.entities.clear();
    let origin = Position { x: 4, y: 3 };
    let wall = Position { x: 5, y: 3 };
    replace_terrain(&mut game, origin, "demo.terrain.floor");
    replace_terrain(&mut game, wall, "demo.terrain.wall");
    game.push_generated_actor(
        "test.wall-destroyer".to_owned(),
        "demo.actor.echo-hound",
        origin,
    );
    let mut events = Vec::new();

    let outcome = game
        .move_entity(0, wall, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("wall destruction should resolve");

    assert_eq!(outcome, ActorStepOutcome::Moved);
    assert_eq!(game.entities[0].position, wall);
    assert_eq!(game.terrain_at(wall), "demo.terrain.floor");
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::MonsterTerrainDestroyed { position, .. }] if *position == wall
    ));
}

#[test]
fn item_destroyer_removes_an_ordinary_ground_item() {
    let mut game = game_with_actor_definition(5, "demo.actor.echo-hound", |actor| {
        actor.terrain_interaction.destroys_items = true;
    });
    game.entities[0].kind_id = "demo.actor.echo-hound".to_owned();
    let position = game.entities[0].position;
    game.items.push(ItemInstance {
        id: "test.destroyed-item".to_owned(),
        kind_id: "demo.item.echo-charm".to_owned(),
        quantity: 2,
        inscription: None,
        origin_actor_kind_id: None,
        quality: ItemQualityDto::Ordinary,
        affix_ids: Vec::new(),
        rolled_affixes: Vec::new(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        fuel: None,
        device_recovery_progress: 0,
        location: ItemLocation::Ground(position),
    });
    let mut events = Vec::new();

    game.destroy_items_under_monster(0, position, &mut events, &mut BTreeSet::new());

    assert!(
        game.items
            .iter()
            .all(|item| item.id != "test.destroyed-item")
    );
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::MonsterItemDestroyed { target_kind_id, quantity: 2, .. }]
            if target_kind_id == "demo.item.echo-charm"
    ));
}

#[test]
fn item_destroyer_removes_a_ground_gold_pile() {
    let mut game = game_with_actor_definition(6, "demo.actor.echo-hound", |actor| {
        actor.terrain_interaction.destroys_items = true;
    });
    game.entities[0].kind_id = "demo.actor.echo-hound".to_owned();
    let position = game.entities[0].position;
    game.gold_piles.push(GoldPile {
        id: "test.destroyed-gold".to_owned(),
        position,
        amount: 17,
        appearance: GoldAppearanceDto::Silver,
        discovered: true,
    });
    let mut events = Vec::new();

    game.destroy_items_under_monster(0, position, &mut events, &mut BTreeSet::new());

    assert!(game.gold_piles.is_empty());
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::MonsterItemDestroyed { target_kind_id, quantity: 17, .. }]
            if target_kind_id == "core.gold.silver"
    ));
}

#[test]
fn item_picker_carries_ordinary_ground_items_and_leaves_protected_kinds() {
    let mut game = game_with_actor_definition(6, "demo.actor.echo-hound", |actor| {
        actor.terrain_interaction.picks_up_items = true;
    });
    game.entities[0].kind_id = "demo.actor.echo-hound".to_owned();
    let position = game.entities[0].position;
    for (id, kind_id) in [
        ("test.ordinary", "demo.item.echo-charm"),
        ("test.corpse", "demo.item.corpse-remains"),
        ("test.artifact", "demo.item.relic-blade"),
    ] {
        give_inventory_item(&mut game, id, kind_id);
        game.items
            .last_mut()
            .expect("test item should exist")
            .location = ItemLocation::Ground(position);
    }
    let mut events = Vec::new();

    game.pick_up_items_under_monster(0, position, &mut events, &mut BTreeSet::new());

    assert!(matches!(
        &game.items
            .iter()
            .find(|item| item.id == "test.ordinary")
            .expect("ordinary item should remain")
            .location,
        ItemLocation::CarriedBy { actor_id } if actor_id == &game.entities[0].id
    ));
    for item_id in ["test.corpse", "test.artifact"] {
        assert!(matches!(
            game.items
                .iter()
                .find(|item| item.id == item_id)
                .expect("protected item should remain")
                .location,
            ItemLocation::Ground(item_position) if item_position == position
        ));
    }
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::MonsterItemPickedUp { target_kind_id, .. }]
            if target_kind_id == "demo.item.echo-charm"
    ));
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
