// SPDX-License-Identifier: MPL-2.0
use crate::game::monster_ecology::OriginalGroupRole;
use crate::rng::RfbRng;

use super::support::*;
use super::*;

fn enter_warrens(seed: u64) -> Game {
    let mut game = Game::new_warrens_journey_with_build(seed, "demo.build.warrior")
        .expect("Warrens journey should create");
    place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
    game.traverse_stairs(false)
        .expect("Warrens entry should resolve")
        .expect("Warrens entry should transition");
    game
}

fn first_seed_for(mut predicate: impl FnMut(&mut RfbRng) -> bool) -> u64 {
    (0..1_000_000)
        .find(|seed| predicate(&mut RfbRng::seeded(*seed)))
        .expect("bounded deterministic seed search should find a match")
}

#[test]
fn depth_nine_two_stage_out_of_depth_roll_reaches_level_fourteen() {
    let seed = first_seed_for(|rng| rng.bounded(40) == 0 && rng.bounded(40) == 0);
    let mut game = enter_warrens(1);
    game.rng = RfbRng::seeded(seed);

    assert_eq!(game.original_allocation_level(9), 14);
}

#[test]
fn non_preferred_glyph_uses_original_monster_div_sixteen_weight() {
    let mut game = enter_warrens(2);
    let policy = game
        .content
        .encounter_table("demo.encounter-table.warrens")
        .and_then(|table| table.global_allocation.as_ref())
        .expect("Warrens global allocation policy")
        .clone();
    let mut actor = game
        .content
        .actor("demo.actor.newt")
        .expect("Newt definition")
        .clone();
    actor.glyph = "o".to_owned();
    game.rng = RfbRng::seeded(3);
    let draws_before = game.rng.draw_counter;

    assert_eq!(game.original_dungeon_weight(&actor, &policy), 25);
    assert_eq!(game.rng.draw_counter, draws_before + 1);
    assert_eq!(game.original_dungeon_weight(&actor, &policy), 25);
    assert_eq!(game.rng.draw_counter, draws_before + 1);
}

#[test]
fn warg_friend_count_uses_three_d_three_including_the_leader() {
    let mut game = enter_warrens(4);
    let warg = game
        .content
        .actor("demo.actor.warg")
        .expect("Warg definition")
        .clone();
    game.rng = RfbRng::seeded(5);

    let total = game.original_friend_total(&warg, 9);

    assert!((3..=9).contains(&total));
    assert_eq!(game.rng.draw_counter, 3);
}

#[test]
fn mughash_escort_uses_lower_level_kobolds() {
    let mut game = enter_warrens(6);
    let policy = game
        .content
        .encounter_table("demo.encounter-table.warrens")
        .and_then(|table| table.global_allocation.as_ref())
        .expect("Warrens global allocation policy")
        .clone();
    let leader_position = game.player.position;
    let mut occupied = BTreeSet::from([leader_position]);
    let terrain = game.terrain.clone();

    let members = game.plan_original_group(
        &policy,
        "demo.actor.warrens-keeper",
        leader_position,
        9,
        &terrain,
        game.width,
        game.height,
        &mut occupied,
    );

    assert!(!members.is_empty());
    assert!(members.iter().all(|member| {
        member.role == OriginalGroupRole::Escort
            && matches!(
                member.kind_id.as_str(),
                "demo.actor.small-kobold" | "demo.actor.kobold" | "demo.actor.large-kobold"
            )
    }));
}

#[test]
fn giant_white_mouse_reproduction_adds_one_adjacent_mouse() {
    let mut game = enter_warrens(7);
    game.entities.clear();
    let origin = game.player.position;
    game.player.position = Position {
        x: origin.x.saturating_sub(5),
        y: origin.y,
    };
    game.push_generated_actor(
        "test.mouse".to_owned(),
        "demo.actor.giant-white-mouse",
        origin,
    );
    let seed = first_seed_for(|rng| {
        let _harmony = rng.bounded(375);
        rng.bounded(8) == 0
    });
    game.rng = RfbRng::seeded(seed);
    let mut changed = BTreeSet::new();

    assert!(game.try_original_reproduction(0, &mut changed));
    assert_eq!(game.entities.len(), 2);
    assert_eq!(game.entities[1].kind_id, "demo.actor.giant-white-mouse");
    assert!(adjacent(origin, game.entities[1].position));
}

#[test]
fn same_kind_reproduction_stops_at_one_hundred_living_monsters() {
    let mut game = enter_warrens(70);
    game.entities.clear();
    let origin = game.player.position;
    game.player.position = Position {
        x: origin.x.saturating_sub(5),
        y: origin.y,
    };
    for ordinal in 0..100 {
        let mut actor = game.generated_actor(
            format!("test.mouse.{ordinal}"),
            "demo.actor.giant-white-mouse",
            origin,
        );
        actor.position = Position {
            x: origin.x + i32::try_from(ordinal % 10).expect("small x offset"),
            y: origin.y + i32::try_from(ordinal / 10).expect("small y offset"),
        };
        game.entities.push(actor);
    }

    let draws_before = game.rng.draw_counter;
    assert!(!game.try_original_reproduction(0, &mut BTreeSet::new()));
    assert_eq!(game.entities.len(), 100);
    assert_eq!(game.rng.draw_counter, draws_before);
}

#[test]
fn original_pack_members_share_one_selected_behavior() {
    let mut game = enter_warrens(71);
    for _ in 2..=9 {
        place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        game.traverse_stairs(false)
            .expect("Warrens descent should resolve")
            .expect("Warrens descent should transition");
    }
    let guardian = game
        .entities
        .iter()
        .find(|actor| actor.id == "demo.guardian.warrens.1")
        .expect("Warrens guardian should be generated");
    let pack_id = guardian
        .pack
        .as_ref()
        .expect("Warrens guardian should lead an escort pack")
        .id
        .clone();
    let pack = game
        .entities
        .iter()
        .filter(|actor| actor.pack.as_ref().is_some_and(|pack| pack.id == pack_id))
        .collect::<Vec<_>>();

    assert!(pack.len() > 1);
    assert!(pack.iter().all(|actor| {
        actor.pack.as_ref().expect("pack member identity").behavior
            == guardian
                .pack
                .as_ref()
                .expect("guardian pack identity")
                .behavior
    }));
}

#[test]
fn fixed_guardian_without_allocation_generates_on_global_allocation_floor() {
    let mut game = enter_warrens(72);
    let mut floor = game
        .content
        .world(WARRENS_JOURNEY_WORLD_ID)
        .expect("Warrens world definition")
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.warrens-depth-9")
        .expect("Warrens final floor definition")
        .clone();
    let guardian = floor
        .guardian
        .as_mut()
        .expect("Warrens final floor guardian");
    guardian.instance_id = "test.fixed-guardian".to_owned();
    guardian.actor_kind_id = "demo.actor.serpent-of-chaos".to_owned();

    let generated = game
        .generate_procedural_floor(&floor, None)
        .expect("fixed Guardian without allocation should generate safely");

    assert!(generated.entities.iter().any(|actor| {
        actor.id == "test.fixed-guardian" && actor.kind_id == "demo.actor.serpent-of-chaos"
    }));
}

#[test]
fn warg_random_movement_replaces_normal_tracking() {
    let mut game = enter_warrens(8);
    game.entities.clear();
    let origin = game.player.position;
    game.player.position = Position {
        x: origin.x.saturating_sub(5),
        y: origin.y,
    };
    game.push_generated_actor("test.warg".to_owned(), "demo.actor.warg", origin);
    let seed = first_seed_for(|rng| rng.bounded(100) < 25);
    game.rng = RfbRng::seeded(seed);

    assert!(
        game.resolve_original_random_movement(
            0,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("random movement should resolve")
    );
    assert!(adjacent(origin, game.entities[0].position));
}

#[test]
fn ambient_allocation_adds_a_distant_warrens_monster() {
    let mut game = enter_warrens(9);
    game.entities.clear();
    game.items
        .retain(|item| !matches!(item.location, ItemLocation::CarriedBy { .. }));
    let chance = 160_u64 * 101 / 100;
    let seed = first_seed_for(|rng| rng.bounded(chance) == 0);
    game.rng = RfbRng::seeded(seed);

    game.process_ambient_monster_allocation(&mut BTreeSet::new())
        .expect("ambient Warrens allocation should resolve");

    assert!(!game.entities.is_empty());
    assert!(
        game.entities
            .iter()
            .all(|actor| rfb_distance(actor.position, game.player.position) > 25)
    );
}

#[test]
fn defeated_unique_state_round_trips_after_normal_unique_death() {
    let mut game = enter_warrens(10);
    game.entities.clear();
    game.push_generated_actor(
        "test.unique".to_owned(),
        "demo.actor.dread-vampire",
        game.player.position,
    );
    game.resolve_actor_death(
        0,
        DomainEvent::EntityDiedFromStatus {
            target_kind_id: "demo.actor.dread-vampire".to_owned(),
            status_kind_id: STATUS_POISON.to_owned(),
            damage: DamageOutcome {
                raw: 1,
                armor_reduction: 0,
                requested: 1,
                applied: 1,
                resistance_delta: 0,
                damage_type: DamageType::Poison,
                resistance: ResistanceLevel::Normal,
            },
        },
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("normal unique death should resolve");

    let restored = Game::from_save(game.to_save()).expect("unique state should round-trip");

    assert!(
        restored
            .defeated_unique_actor_kind_ids
            .contains("demo.actor.dread-vampire")
    );
    assert!(!restored.unique_actor_kind_is_available("demo.actor.dread-vampire"));
}

#[test]
fn fixed_unique_summon_plans_only_one_available_instance() {
    let mut game = enter_warrens(11);
    game.entities.clear();
    let mut ability = game
        .content
        .abilities()
        .find(|ability| matches!(ability.effect, AbilityEffectDefinition::Summon { .. }))
        .expect("demo should retain a fixed summon ability")
        .clone();
    let AbilityEffectDefinition::Summon {
        actor_kind_id,
        count,
        ..
    } = &mut ability.effect
    else {
        unreachable!("selected ability must remain a fixed summon")
    };
    *actor_kind_id = "demo.actor.dread-vampire".to_owned();
    *count = 2;

    let plan = game
        .ability_target_plan(&ability, &TargetSelection::SelfTarget)
        .expect("available Unique should produce a summon plan");
    let AbilityTargetPlan::Summon { positions } = plan else {
        panic!("fixed summon should retain its target plan kind");
    };
    assert_eq!(positions.len(), 1);

    game.push_generated_actor(
        "test.unique".to_owned(),
        "demo.actor.dread-vampire",
        positions[0],
    );
    assert!(
        game.ability_target_plan(&ability, &TargetSelection::SelfTarget)
            .is_none()
    );
}

#[test]
fn save_rejects_duplicate_living_normal_unique_instances() {
    let mut game = enter_warrens(12);
    game.entities.clear();
    let first = game.player.position;
    let second = Position {
        x: first.x.saturating_add(1),
        y: first.y,
    };
    game.push_generated_actor(
        "test.unique.1".to_owned(),
        "demo.actor.dread-vampire",
        first,
    );
    game.push_generated_actor(
        "test.unique.2".to_owned(),
        "demo.actor.dread-vampire",
        second,
    );

    let error = Game::from_save(game.to_save()).expect_err("duplicate Unique save must fail");
    assert!(matches!(
        error,
        CoreError::InvalidSave("living unique actor state is duplicated")
    ));
}
