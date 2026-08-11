// SPDX-License-Identifier: MPL-2.0
use crate::game::monster_ecology::{
    OriginalGroupRole, actor_allocation_matches_legacy_dungeon, actor_allocation_matches_task,
};
use crate::rng::RfbRng;

use super::support::*;
use super::*;

fn enter_warrens(seed: u64) -> Game {
    let mut game =
        Game::new_with_build(seed, "demo.build.warrior").expect("Warrens journey should create");
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

fn eldritch_seed(saving_throw_skill: i32, consequence_saves: &[bool]) -> u64 {
    let threshold = u64::try_from(saving_throw_skill.saturating_sub(9).clamp(0, 100))
        .expect("clamped saving throw must fit u64");
    first_seed_for(|rng| {
        if rng.bounded(100) >= 9 || rng.bounded(100) < threshold {
            return false;
        }
        consequence_saves
            .iter()
            .all(|expected| (rng.bounded(100) < threshold) == *expected)
    })
}

fn game_with_ghast() -> (Game, usize) {
    let mut game =
        Game::new_with_build(1, "demo.build.warrior").expect("Warrens journey should create");
    let position = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    game.push_generated_actor("test.ghast".to_owned(), "demo.actor.ghast", position);
    let index = game.entities.len() - 1;
    (game, index)
}

#[test]
fn compost_monsters_allocate_only_in_the_sewer_task() {
    let game = Game::new(3);
    for actor_id in ["demo.actor.plague-rat", "demo.actor.nizukil-prince-of-rats"] {
        let allocation = game
            .content
            .actor(actor_id)
            .and_then(|actor| actor.allocation.as_ref())
            .expect("compost actor allocation");
        assert!(!actor_allocation_matches_task(allocation, None));
        assert!(!actor_allocation_matches_task(
            allocation,
            Some("demo.task.pest-control")
        ));
        assert!(actor_allocation_matches_task(
            allocation,
            Some("demo.task.the-sewer")
        ));
    }
}

#[test]
fn shapechanger_projects_another_monster_and_rerolls_each_action() {
    let mut game =
        Game::new_with_build(1, "demo.build.warrior").expect("Warrens journey should create");
    game.push_generated_actor(
        "test.shapechanger".to_owned(),
        "demo.actor.chaos-shapechanger",
        Position { x: 5, y: 3 },
    );
    let index = game.entities.len() - 1;
    let mut actor = game.entities.pop().expect("shapechanger should exist");
    game.maybe_apply_shadower_appearance(&mut actor);
    game.entities.push(actor);

    let appearance = game.entities[index]
        .appearance_kind_id
        .as_deref()
        .expect("shapechanger should project another monster");
    assert_ne!(appearance, "demo.actor.chaos-shapechanger");
    assert!(game.content.actor(appearance).is_some_and(|definition| {
        definition.role == ActorRole::Monster
            && !definition
                .tags
                .iter()
                .any(|tag| tag == "shadower-appearance")
    }));

    let draws_before = game.rng.draw_counter;
    game.reroll_shapechanger_appearance(index);
    assert_eq!(game.rng.draw_counter, draws_before + 1);
    assert!(game.entities[index].appearance_kind_id.is_some());
}

#[test]
fn tanuki_keeps_true_runtime_stats_behind_one_persistent_disguise() {
    let mut game = Game::new(1);
    clear_monsters(&mut game);
    game.push_generated_actor(
        "test.tanuki".to_owned(),
        "demo.actor.tanuki",
        Position { x: 5, y: 3 },
    );
    let mut actor = game.entities.pop().expect("tanuki should exist");
    game.maybe_apply_shadower_appearance(&mut actor);
    game.entities.push(actor);

    let appearance = game.entities[0]
        .appearance_kind_id
        .clone()
        .expect("tanuki should receive an initial disguise");
    assert_ne!(appearance, "demo.actor.tanuki");
    assert_eq!(
        game.actor_runtime_definition(&game.entities[0])
            .expect("tanuki runtime definition")
            .id,
        "demo.actor.tanuki"
    );
    assert_eq!(
        game.actor_apparent_definition(&game.entities[0])
            .expect("tanuki apparent definition")
            .id,
        appearance
    );
    let draws_before = game.rng.draw_counter;
    game.reroll_shapechanger_appearance(0);
    assert_eq!(game.rng.draw_counter, draws_before);
    assert_eq!(
        game.entities[0].appearance_kind_id.as_deref(),
        Some(appearance.as_str())
    );

    let restored = Game::from_save(game.to_save()).expect("tanuki disguise should round-trip");
    assert_eq!(
        restored.entities[0].appearance_kind_id.as_deref(),
        Some(appearance.as_str())
    );
}

#[test]
fn fear_aura_uses_apparent_level_and_only_lands_once_per_tick_at_range() {
    let mut game = game_with_actor_definition(0, "demo.actor.fearmaster", |actor| {
        actor.level = 500;
    });
    clear_monsters(&mut game);
    game.player.position = Position { x: 3, y: 3 };
    game.push_generated_actor(
        "test.fearmaster".to_owned(),
        "demo.actor.fearmaster",
        Position { x: 5, y: 3 },
    );
    let seed = first_seed_for(|rng| rng.bounded(100) >= 95);
    game.rng = RfbRng::seeded(seed);
    let mut events = Vec::new();

    assert!(game.resolve_monster_fear_aura(0, "hurt", true, &mut events));
    game.rng = RfbRng::seeded(seed);
    assert!(!game.resolve_monster_fear_aura(0, "hurt", true, &mut events));
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, DomainEvent::MonsterFearAuraApplied { .. }))
            .count(),
        1
    );
    assert!(game.player_has_status_kind(STATUS_FEAR));
}

#[test]
fn chameleon_keeps_its_identity_while_its_form_drives_runtime_behavior() {
    let mut game =
        Game::new_with_build(1, "demo.build.warrior").expect("Warrens journey should create");
    game.push_generated_actor(
        "test.chameleon".to_owned(),
        "demo.actor.chameleon",
        Position { x: 5, y: 3 },
    );
    let index = game.entities.len() - 1;
    game.apply_chameleon_form(index, "demo.actor.illusionist");

    let actor = &game.entities[index];
    let form = game
        .content
        .actor("demo.actor.illusionist")
        .expect("illusionist form should exist");
    assert_eq!(actor.kind_id, "demo.actor.chameleon");
    assert_eq!(
        actor.appearance_kind_id.as_deref(),
        Some("demo.actor.illusionist")
    );
    assert_eq!(actor.speed, form.speed);
    assert!(actor_max_hp_is_valid(form, actor.max_hp));
    assert_eq!(actor.resistances, definition_resistance_profile(form));
    let runtime = game
        .actor_runtime_definition(actor)
        .expect("chameleon form should be its runtime definition");
    assert_eq!(runtime.id, form.id);
    assert!(!resolved_melee_blows(runtime).is_empty());
    assert!(runtime.monster_casting.is_some());

    game.entities[index].casting_cooldown_remaining = 3;
    game.entities[index]
        .observed_player_resistances
        .insert(DamageType::Fire, ResistanceLevel::Resistant);
    game.apply_chameleon_form(index, "demo.actor.earth-spirit");
    assert_eq!(game.entities[index].casting_cooldown_remaining, 0);
    assert!(game.entities[index].observed_player_resistances.is_empty());
    let wall = Position { x: 6, y: 3 };
    replace_terrain(&mut game, wall, "demo.terrain.wall");
    assert!(game.actor_can_enter_position(index, wall));

    let expected_hash = game.state_hash();
    let save = game.to_save();
    let saved = save
        .entities
        .iter()
        .find(|entity| entity.id == "test.chameleon")
        .expect("chameleon should be saved");
    assert_eq!(saved.kind_id, "demo.actor.chameleon");
    assert_eq!(
        saved.appearance_kind_id.as_deref(),
        Some("demo.actor.earth-spirit")
    );
    let restored = Game::from_save(save).expect("chameleon form should round-trip");
    assert_eq!(restored.state_hash(), expected_hash);
    let restored_actor = restored
        .entities
        .iter()
        .find(|actor| actor.id == "test.chameleon")
        .expect("restored chameleon should exist");
    assert_eq!(
        restored
            .actor_runtime_definition(restored_actor)
            .expect("restored form should remain active")
            .id,
        "demo.actor.earth-spirit"
    );
}

#[test]
fn chameleon_change_check_uses_one_in_thirteen_before_selecting_a_form() {
    let mut game =
        Game::new_with_build(1, "demo.build.warrior").expect("Warrens journey should create");
    game.push_generated_actor(
        "test.chameleon".to_owned(),
        "demo.actor.chameleon",
        Position { x: 5, y: 3 },
    );
    let index = game.entities.len() - 1;
    game.apply_chameleon_form(index, "demo.actor.small-kobold");

    let miss_seed = first_seed_for(|rng| rng.bounded(13) != 0);
    game.rng = RfbRng::seeded(miss_seed);
    let appearance = game.entities[index].appearance_kind_id.clone();
    assert!(!game.maybe_change_chameleon_form(index));
    assert_eq!(game.rng.draw_counter, 1);
    assert_eq!(game.entities[index].appearance_kind_id, appearance);

    let change_seed = first_seed_for(|rng| rng.bounded(13) == 0);
    game.rng = RfbRng::seeded(change_seed);
    assert!(game.maybe_change_chameleon_form(index));
    assert!(game.rng.draw_counter >= 2);
    assert_eq!(game.entities[index].kind_id, "demo.actor.chameleon");
    assert!(game.entities[index].appearance_kind_id.is_some());
}

#[test]
fn eldritch_horror_triggers_on_fresh_sight_and_persists_its_repeat_gate() {
    let (mut game, index) = game_with_ghast();
    let visible = game.visible_monster_aura_entity_ids();
    let saving_throw = game.player_derived_stats().saving_throw_skill.value;
    let seed = eldritch_seed(saving_throw, &[false]);
    game.rng = RfbRng::seeded(seed);
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();

    game.resolve_newly_visible_monster_auras(&visible, &mut events, &mut changed);
    assert_eq!(game.rng.draw_counter, 0, "continuous sight must be safe");
    game.resolve_newly_visible_monster_auras(&BTreeSet::new(), &mut events, &mut changed);
    assert!(game.entities[index].eldritch_horror_triggered);
    assert!(game.player_has_status_kind(STATUS_CONFUSION));
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::EldritchHorror {
            outcome: "mind-blast",
            ..
        }
    )));

    let expected_hash = game.state_hash();
    let save = game.to_save();
    assert!(
        save.entities
            .iter()
            .find(|actor| actor.id == "test.ghast")
            .expect("Ghast should be saved")
            .eldritch_horror_triggered
    );
    let restored = Game::from_save(save).expect("Eldritch trigger state should round-trip");
    assert_eq!(restored.state_hash(), expected_hash);

    let repeat_miss_seed = first_seed_for(|rng| rng.bounded(100) < 9 && rng.bounded(5) != 0);
    game.rng = RfbRng::seeded(repeat_miss_seed);
    let event_count = events.len();
    game.resolve_eldritch_horror(index, &mut events, &mut changed);
    assert_eq!(game.rng.draw_counter, 2);
    assert_eq!(events.len(), event_count);
}

#[test]
fn eldritch_horror_reuses_attribute_amnesia_and_weird_mind_contracts() {
    let (mut drained, drained_index) = game_with_ghast();
    let saving_throw = drained.player_derived_stats().saving_throw_skill.value;
    drained.rng = RfbRng::seeded(eldritch_seed(saving_throw, &[true, false]));
    let attributes_before = drained.progress.attributes;
    drained.resolve_eldritch_horror(drained_index, &mut Vec::new(), &mut BTreeSet::new());
    assert!(
        drained.progress.attributes.intelligence < attributes_before.intelligence
            || drained.progress.attributes.wisdom < attributes_before.wisdom
            || drained.progress.attributes.charisma < attributes_before.charisma
    );

    let (mut amnesia, amnesia_index) = game_with_ghast();
    let saving_throw = amnesia.player_derived_stats().saving_throw_skill.value;
    amnesia.explored.fill(true);
    amnesia.rng = RfbRng::seeded(eldritch_seed(saving_throw, &[true, true, true, false]));
    amnesia.resolve_eldritch_horror(amnesia_index, &mut Vec::new(), &mut BTreeSet::new());
    assert!(amnesia.explored.iter().all(|explored| !explored));

    let (mut immune, immune_index) = game_with_ghast();
    assert!(immune.gain_mutation("rfb.mutation.weird-mind", &mut Vec::new()));
    let draws_before = immune.rng.draw_counter;
    immune.resolve_eldritch_horror(immune_index, &mut Vec::new(), &mut BTreeSet::new());
    assert_eq!(immune.rng.draw_counter, draws_before);
    assert!(!immune.entities[immune_index].eldritch_horror_triggered);
}

#[test]
fn legacy_dungeon_restrictions_match_only_the_declared_region() {
    let game =
        Game::new_with_build(1, "demo.build.warrior").expect("Warrens journey should create");
    let allocation = |actor_id: &str| {
        game.content
            .actor(actor_id)
            .and_then(|actor| actor.allocation.as_ref())
            .unwrap_or_else(|| panic!("{actor_id} should retain allocation"))
    };

    let duosi = allocation("demo.actor.king-duosi-the-chief-of-southerings");
    assert!(actor_allocation_matches_legacy_dungeon(duosi, Some(31)));
    assert!(!actor_allocation_matches_legacy_dungeon(duosi, Some(30)));
    let wallaby = allocation("demo.actor.wallaby");
    assert!(actor_allocation_matches_legacy_dungeon(wallaby, Some(35)));
    assert!(!actor_allocation_matches_legacy_dungeon(wallaby, None));
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
fn preferred_glyph_or_tag_uses_full_original_weight_without_rng() {
    let mut game = enter_warrens(2);
    let mut policy = game
        .content
        .encounter_table("demo.encounter-table.warrens")
        .and_then(|table| table.global_allocation.as_ref())
        .expect("Warrens global allocation policy")
        .clone();
    policy.preferred_tags = vec!["animal".to_owned()];
    let mut actor = game
        .content
        .actor("demo.actor.newt")
        .expect("Newt definition")
        .clone();
    actor.glyph = "o".to_owned();
    let draws_before = game.rng.draw_counter;

    assert_eq!(game.original_dungeon_weight(&actor, &policy), 100);
    assert_eq!(game.rng.draw_counter, draws_before);

    actor.glyph = "R".to_owned();
    actor.tags.clear();
    assert_eq!(game.original_dungeon_weight(&actor, &policy), 100);
    assert_eq!(game.rng.draw_counter, draws_before);
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
        None,
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
fn sterility_suppresses_reproduction_without_spending_rng() {
    let mut game = enter_warrens(69);
    game.entities.clear();
    let origin = game.player.position;
    game.push_generated_actor(
        "test.mouse".to_owned(),
        "demo.actor.giant-white-mouse",
        origin,
    );
    game.reproduction_suppressed = true;
    let draws_before = game.rng.draw_counter;

    assert!(!game.try_original_reproduction(0, &mut BTreeSet::new()));
    assert_eq!(game.entities.len(), 1);
    assert_eq!(game.rng.draw_counter, draws_before);
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
            x: origin.x + ordinal % 10,
            y: origin.y + ordinal / 10,
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
        .world(DEFAULT_WORLD_ID)
        .expect("Middle-earth world definition")
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
fn unique2_allows_one_living_instance_but_returns_after_death() {
    let mut game = enter_warrens(10);
    game.entities.clear();
    game.push_generated_actor(
        "test.unique2".to_owned(),
        "demo.actor.silver-angel",
        game.player.position,
    );
    assert!(!game.unique_actor_kind_is_available("demo.actor.silver-angel"));

    game.resolve_actor_death(
        0,
        DomainEvent::EntityDiedFromStatus {
            target_kind_id: "demo.actor.silver-angel".to_owned(),
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
    .expect("UNIQUE2 death should resolve");

    assert!(game.unique_actor_kind_is_available("demo.actor.silver-angel"));
    assert!(
        !game
            .defeated_unique_actor_kind_ids
            .contains("demo.actor.silver-angel")
    );
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
