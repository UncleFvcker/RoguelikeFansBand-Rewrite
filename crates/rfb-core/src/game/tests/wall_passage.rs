// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use rfb_content::ActorMovementMode;

const SPECTRE: &str = "rfb-legacy.race.spectre";
const HUMAN: &str = "demo.race.rfb-human";
const WALL: &str = "demo.terrain.wall";
const FLOOR: &str = "demo.terrain.floor";
const START: Position = Position { x: 48, y: 16 };
const EAST: Position = Position { x: 49, y: 16 };

fn form(race: &str, ticks: u32) -> StatusInstance {
    let mut status =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, ticks, "test.wall-form").status;
    status.granted_race_id = Some(race.to_owned());
    status
}

fn prepare(game: &mut Game, native: bool) {
    clear_monsters(game);
    game.items.clear();
    game.gold_piles.clear();
    game.player.statuses.clear();
    game.player.position = START;
    for y in 14..=18 {
        for x in 46..=52 {
            replace_terrain(game, Position { x, y }, FLOOR);
        }
    }
    // Share terrain preconditions between native and temporary body cases.
    if native {
        game.build.as_mut().unwrap().race_id = SPECTRE.to_owned();
    }
    game.refresh_player_resource_maxima();
    game.player.hp = game.effective_player_max_hp();
    game.world_tick = 0;
    game.player.energy_need = 0;
}

fn game(native: bool) -> Game {
    let mut game = Game::new_with_build(83, "demo.build.high-mage-death").unwrap();
    prepare(&mut game, native);
    game
}

fn tick(game: &mut Game) -> Vec<DomainEvent> {
    game.player.energy_need = 1;
    let mut events = Vec::new();
    game.advance_until_player_ready(
        false,
        true,
        false,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    events
}

#[test]
fn spectre_passives_follow_current_form_and_consume_existing_stat_and_hunger_rules() {
    for native in [false, true] {
        let mut game = game(native);
        if !native {
            game.player.statuses.push(form(SPECTRE, 100));
        }
        assert!(game.player_can_pass_walls());
        assert!(game.player_levitates());
        assert!(game.player_slow_digestion());
        assert_eq!(game.player_see_invisible_sources(), 1);
        assert_eq!(game.player_hold_life_sources(), 1);
        assert_eq!(game.player_infravision_range(), 5);
        for kind in [DamageType::Cold, DamageType::Poison, DamageType::Nether] {
            assert_eq!(
                game.effective_player_resistances().level(kind),
                ResistanceLevel::Resistant
            );
        }
        let traits = game.character_trait_details(&game.player_derived_stats());
        assert!(
            traits
                .sources
                .iter()
                .find(|source| source.source_id == SPECTRE)
                .unwrap()
                .passes_walls
        );
        game.nutrition = 5000;
        game.world_tick = 1000;
        game.process_hunger(&mut Vec::new());
        assert_eq!(game.nutrition, 4995);
        game.player.statuses.clear();
        if native {
            game.player.statuses.push(form(HUMAN, 100));
        }
        assert!(!game.player_can_pass_walls());
        assert!(!game.player_levitates());
        assert!(!game.player_slow_digestion());
        assert_eq!(game.player_see_invisible_sources(), 0);
        assert_eq!(game.player_hold_life_sources(), 0);
        game.nutrition = 5000;
        game.process_hunger(&mut Vec::new());
        assert_eq!(game.nutrition, 4990);
    }
}

#[test]
fn spectre_movement_checks_wall_metadata_flight_occupancy_and_original_energy() {
    for (terrain, allowed, cost, damage) in [
        (WALL, true, 150, true),
        ("demo.terrain.door-closed", true, 150, true),
        ("demo.terrain.permanent-wall", false, 100, false),
        ("demo.terrain.surface-tree", true, 100, false),
        ("demo.terrain.curtain-closed", true, 100, false),
        ("demo.terrain.surface-mountain", true, 100, false),
    ] {
        let mut game = game(true);
        replace_terrain(&mut game, EAST, terrain);
        game.explored.fill(true);
        assert_eq!(game.player_can_enter_position(EAST), allowed, "{terrain}");
        assert_eq!(
            game.next_local_travel_direction(EAST),
            allowed.then_some(Direction::East),
            "{terrain}"
        );
        let before_hp = game.player.hp;
        let update = dispatch_next(
            &mut game,
            GameCommand::Move {
                direction: Direction::East,
            },
        );
        assert_eq!(
            game.player.position,
            if allowed { EAST } else { START },
            "{terrain}"
        );
        assert_eq!(game.world_tick, cost / 10, "{terrain}");
        assert_eq!(game.player.hp < before_hp, damage, "{terrain}");
        assert_eq!(
            update
                .events
                .iter()
                .any(|event| event.message_key == "player-wall-density"),
            damage
        );
        if allowed {
            assert_eq!(
                game.terrain[game.index(EAST).unwrap()],
                terrain,
                "passage must preserve terrain"
            );
        }
    }
    let mut game = game(true);
    assert!(!game.player_can_enter_position(Position { x: -1, y: 16 }));
    assert!(!game.player_can_enter_position(Position {
        x: i32::from(game.width),
        y: 16
    }));
    replace_terrain(&mut game, EAST, WALL);
    assert!(!projectile_geometry::has_line_of_effect(
        &game,
        START,
        Position { x: 50, y: 16 }
    ));
    game.push_generated_actor("test.wall-blocker".to_owned(), "demo.actor.horse", EAST);
    game.entities[0].controller_id = Some(game.player.id.clone());
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(game.player.position, START);
}

#[test]
fn spectre_density_caps_only_native_body_before_shared_damage_and_never_uses_one_hp_floor() {
    for native in [false, true] {
        for (level, hp) in [(4, 9), (5, 9), (5, 2), (5, 1), (5, 0)] {
            let mut game = game(native);
            if !native {
                game.player.statuses.push(form(SPECTRE, 100));
            }
            game.progress.level = level;
            game.player.hp = hp;
            replace_terrain(&mut game, START, WALL);
            game.world_tick = 10;
            let rng = game.rng.clone();
            let mut events = Vec::new();
            let blocked = game.process_player_wall_damage(&mut events);
            let raw = 1 + i32::from(level / 5);
            let expected = if native { raw.min(hp) } else { raw };
            assert_eq!(game.player.hp, hp - expected);
            assert_eq!(blocked, expected != 0);
            assert_eq!(game.player_is_dead(), hp - expected < 0);
            assert_eq!(game.rng, rng);
        }
    }
    let mut game = game(true);
    replace_terrain(&mut game, START, WALL);
    game.player.hp = 1;
    game.world_tick = 10;
    game.player.statuses.push(form(HUMAN, 100));
    assert!(game.process_player_wall_damage(&mut Vec::new()));
    assert_eq!(game.player.hp, 0); // Human form takes ordinary crushing damage, not a race cap.
    game.world_tick = 20;
    assert!(game.process_player_wall_damage(&mut Vec::new()));
    assert_eq!(game.player.hp, -1);
}

#[test]
fn native_spectre_cap_survives_other_form_only_while_another_source_grants_passage() {
    let mut game = game(true);
    let mut human = form(HUMAN, 100);
    human.grants_wall_passage = true;
    game.player.statuses.push(human);
    game.progress.level = 5;
    game.player.hp = 1;
    game.world_tick = 10;
    replace_terrain(&mut game, START, WALL);
    assert!(game.process_player_wall_damage(&mut Vec::new()));
    assert_eq!(game.player.hp, 0);
    assert!(!game.player_is_dead());
}

#[test]
fn wall_immunity_last_tick_precedes_expiry_without_relocation_and_crushing_follows() {
    for kind in [STATUS_WRAITHFORM, STATUS_INVULNERABILITY] {
        let mut game = game(false);
        replace_terrain(&mut game, START, WALL);
        let mut status = monster_combat::melee_status(kind, 1, "test.wall-immunity").status;
        status.grants_wall_passage = kind == STATUS_WRAITHFORM;
        game.player.statuses.push(status);
        game.world_tick = 9;
        let events = tick(&mut game);
        let wall_event = events
            .iter()
            .position(|event| matches!(event, DomainEvent::PlayerWallDamaged { .. }));
        if kind == STATUS_INVULNERABILITY {
            // Expiration itself spends 100 energy, reaching the next, unprotected wall tick.
            let expiry = events.iter().position(|event| matches!(event, DomainEvent::PlayerStatusExpired { status_kind_id } if status_kind_id == kind)).unwrap();
            assert!(expiry < wall_event.unwrap());
            assert_eq!(game.world_tick, 20);
        } else {
            assert_eq!(wall_event, None);
            assert_eq!(game.world_tick, 10);
        }
        assert!(!game.player_has_status_kind(kind));
        assert_eq!(game.player.position, START);
        game.world_tick = 19;
        game.player.hp = 0;
        let events = tick(&mut game);
        assert!(
            events.iter().any(|event| matches!(
                event,
                DomainEvent::PlayerWallDamaged { crushing: true, .. }
            ))
        );
        assert!(game.player_is_dead());
        assert_eq!(game.player.position, START);
    }
    let mut game = game(false);
    game.player.statuses.push(form(SPECTRE, 1));
    replace_terrain(&mut game, START, WALL);
    game.world_tick = 9;
    assert!(tick(&mut game).iter().any(|event| matches!(
        event,
        DomainEvent::PlayerWallDamaged {
            crushing: false,
            ..
        }
    )));
    assert!(!game.player_can_pass_walls());
    assert_eq!(game.player.position, START);
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(game.player.position, EAST);
}

#[test]
fn density_blocks_hp_regeneration_but_preserves_zero_damage_and_transcendence_boundaries() {
    let mut bare = game(true);
    replace_terrain(&mut bare, START, WALL);
    bare.player.hp = 0;
    for _ in 0..1000 {
        tick(&mut bare);
        assert!(!bare.player_is_dead());
        if bare.player.hp > 0 {
            break;
        }
    }
    assert!(
        bare.player.hp > 0,
        "a zero-density tick must permit natural HP recovery without equipment"
    );
    let mut game = game(true);
    give_inventory_item(&mut game, "test.wall-regeneration", "demo.item.cloak");
    let cloak = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.wall-regeneration")
        .unwrap();
    cloak.affix_ids.push("demo.affix.regeneration".to_owned());
    cloak.location = ItemLocation::Equipped {
        slot_id: "cloak".to_owned(),
    };
    replace_terrain(&mut game, START, WALL);
    game.world_tick = 9;
    game.player.hp = 1;
    let events = tick(&mut game);
    assert_eq!(game.player.hp, 0);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::EquipmentRegenerated { .. }))
    );
    game.world_tick = 19;
    let events = tick(&mut game);
    assert!(game.player.hp >= 1);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::EquipmentRegenerated { .. }))
    );

    game.player.hp = 1;
    game.world_tick = 29;
    game.resources
        .get_mut("demo.resource.mana")
        .unwrap()
        .current = 1;
    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_TRANSCENDENCE, 100, "test.wall").status);
    let events = tick(&mut game);
    assert_eq!(game.player.hp, 1);
    assert_eq!(game.resources["demo.resource.mana"].current, 0);
    assert!(events.iter().any(|event| matches!(event, DomainEvent::PlayerWallDamaged { damage, .. } if damage.applied == 0)));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::EquipmentRegenerated { .. }))
    );
}

#[test]
fn wall_damage_interrupts_rest_after_mana_recovery_and_wait_also_recovers_mana() {
    for resting in [false, true] {
        let mut game = game(true);
        replace_terrain(&mut game, START, WALL);
        game.player.hp = game.effective_player_max_hp() - 1;
        game.resources
            .get_mut("demo.resource.mana")
            .unwrap()
            .current = 0;
        if resting {
            let resolution = game
                .resolve_player_rest(20, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
                .unwrap();
            assert_eq!(resolution.completed_turns, 1);
            assert_eq!(resolution.stop_reason, RestStopReasonDto::Damaged);
        } else {
            dispatch_next(&mut game, GameCommand::Wait);
        }
        assert!(game.resources["demo.resource.mana"].current > 0);
    }
}

#[test]
fn mounted_wall_passage_requires_both_participants_and_dismount_uses_the_riders_body() {
    for mount_passes in [false, true] {
        let content_game = game_with_actor_definition(83, "demo.actor.horse", |actor| {
            actor.movement.modes = if mount_passes {
                vec![ActorMovementMode::PassWall]
            } else {
                Vec::new()
            };
        });
        for native in [false, true] {
            let mut game = Game::from_content_with_build(
                83,
                content_game.content.clone(),
                DEFAULT_WORLD_ID,
                "demo.build.high-mage-death",
            )
            .unwrap();
            prepare(&mut game, native);
            game.push_generated_actor("test.mount".to_owned(), "demo.actor.horse", START);
            game.entities[0].controller_id = Some(game.player.id.clone());
            game.riding_actor_id = Some("test.mount".to_owned());
            replace_terrain(&mut game, EAST, WALL);
            assert_eq!(game.player_can_pass_walls(), mount_passes && native);
            assert_eq!(
                game.actor_can_enter_position(0, EAST),
                mount_passes && native
            );
            assert_eq!(game.player_can_enter_position(EAST), mount_passes && native);
            dispatch_next(
                &mut game,
                GameCommand::Move {
                    direction: Direction::East,
                },
            );
            assert_eq!(
                game.player.position,
                if native && mount_passes { EAST } else { START }
            );
            let target = Position {
                x: game.player.position.x,
                y: game.player.position.y - 1,
            };
            replace_terrain(&mut game, target, WALL);
            game.resolve_riding(Direction::North, &mut Vec::new(), &mut BTreeSet::new());
            assert_eq!(game.riding_actor_id.is_none(), native);
            assert_eq!(game.player.position == target, native);
        }
    }
}

#[test]
fn wall_positions_round_trip_after_form_expiry_and_in_departed_floor_cache() {
    let mut game = game(false);
    game.player.statuses.push(form(SPECTRE, 1));
    replace_terrain(&mut game, START, WALL);
    game.world_tick = 0;
    tick(&mut game);
    assert!(!game.player_can_pass_walls());
    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    dispatch_next(
        &mut restored,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(restored.state_hash(), game.state_hash());

    game.player.position = Position { x: 93, y: 29 };
    game.traverse_stairs(false).unwrap().unwrap();
    clear_monsters(&mut game);
    game.items.clear();
    game.gold_piles.clear();
    let next_floor = game
        .content
        .world(&game.world_id)
        .unwrap()
        .procedural_floors
        .iter()
        .find(|floor| floor.id == game.current_floor_id)
        .unwrap()
        .next_floor_id
        .clone()
        .unwrap();
    let position = (0..game.terrain.len())
        .map(|index| Position {
            x: (index % usize::from(game.width)) as i32,
            y: (index / usize::from(game.width)) as i32,
        })
        .find(|position| {
            game.is_walkable(*position)
                && !game
                    .floor_connections
                    .iter()
                    .any(|connection| connection.position == *position)
        })
        .unwrap();
    game.player.position = position;
    replace_terrain(&mut game, position, WALL);
    let departed = game.current_floor_id.clone();
    game.transition_floor(next_floor, None, None, false)
        .unwrap_or_else(|error| panic!("transition from {departed}: {error:?}"))
        .unwrap();
    let cached = game
        .stored_floors
        .values()
        .find(|floor| floor.id == departed)
        .unwrap();
    assert_eq!(cached.player_position, position);
    let saved = game.to_save();
    let mut restored = Game::from_save_with_content(saved.clone(), game.content.clone()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    for instance in [&mut game, &mut restored] {
        instance
            .transition_floor(departed.clone(), None, None, false)
            .unwrap()
            .unwrap();
        assert_eq!(instance.player.position, position);
        dispatch_next(instance, GameCommand::Wait);
    }
    assert_eq!(restored.state_hash(), game.state_hash());
    let mut invalid = saved;
    let cached = invalid
        .stored_floors
        .iter_mut()
        .find(|floor| floor.id == departed)
        .unwrap();
    cached.player_position.x = -1;
    assert!(Game::from_save_with_content(invalid, game.content.clone()).is_err());
}

#[test]
fn demonlord_breaks_source_destructible_terrain_without_density_energy_or_mining_rewards() {
    for (terrain, replacement) in [
        (WALL, Some(FLOOR)),
        ("demo.terrain.door-closed", Some(FLOOR)),
        ("demo.terrain.door-jammed-7", Some(FLOOR)),
        ("demo.terrain.glass-wall", Some(FLOOR)),
        ("demo.terrain.glass-door-closed", Some(FLOOR)),
        ("demo.terrain.curtain-closed", Some(FLOOR)),
        (
            "demo.terrain.surface-tree",
            Some("demo.terrain.surface-grass"),
        ),
        ("demo.terrain.magma-treasure", Some(FLOOR)),
        ("demo.terrain.permanent-wall", None),
    ] {
        let mut game = high_mage::daemon_high_mage_game(83, 50);
        prepare(&mut game, false);
        choose_human_talent_if_pending(&mut game);
        give_inventory_item(&mut game, "test.hellfire-tome", "demo.item.hellfire-tome");
        game.resources
            .get_mut("demo.resource.mana")
            .unwrap()
            .current = 1000;
        let mut cast_events = Vec::new();
        game.resolve_player_ability(
            "demo.ability.daemon-polymorph-demonlord",
            TargetSelection::SelfTarget,
            &mut cast_events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert!(
            cast_events
                .iter()
                .any(|event| matches!(event, DomainEvent::AbilityCastSucceeded { .. }))
        );
        assert!(game.player_has_status_kind(STATUS_DEMON_LORD_TRANSFORMATION));
        game.items.clear();
        replace_terrain(&mut game, EAST, terrain);
        assert!(!game.player_can_pass_walls());
        let expected_ticks = (STANDARD_ACTION_COST as u32).div_ceil(
            u32::try_from(energy_gain(derived_speed(
                &game.player_derived_stats().speed,
            )))
            .unwrap(),
        );
        let update = dispatch_next(
            &mut game,
            GameCommand::Move {
                direction: Direction::East,
            },
        );
        assert_eq!(
            game.player.position,
            if replacement.is_some() { EAST } else { START },
            "{terrain}"
        );
        assert_eq!(
            game.terrain[game.index(EAST).unwrap()],
            replacement.unwrap_or(terrain)
        );
        assert_eq!(game.world_tick, expected_ticks, "{terrain}");
        assert!(!update.events.iter().any(|event| matches!(
            event.message_key.as_str(),
            "player-wall-density" | "player-wall-crushed"
        )));
        assert!(game.gold_piles.is_empty());
        assert!(game.items.is_empty());
        assert_eq!(game.progress.mining_proficiency, 0);
    }
}

#[test]
fn actual_wraith_spell_exempts_density_and_preserves_transparent_walls_with_demonlord() {
    let mut game = game(false);
    game.progress.level = 50;
    game.progress.max_level = 50;
    choose_human_talent_if_pending(&mut game);
    game.ability_learning_order
        .push("demo.ability.death-wraithform".to_owned());
    game.bonus_spell_learning_capacity = 32;
    give_inventory_item(&mut game, "test.necronomicon", "demo.item.necronomicon");
    game.debug_ability_casts_succeed = true;
    // The simultaneous demon body is a form precondition, not a second learned realm.
    game.player.statuses.push(form("demo.race.demon-lord", 100));
    game.refresh_player_resource_maxima();
    let mana = game.resources.get_mut("demo.resource.mana").unwrap();
    mana.current = mana.maximum;
    let mut cast_events = Vec::new();
    game.resolve_player_ability(
        "demo.ability.death-wraithform",
        TargetSelection::SelfTarget,
        &mut cast_events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(
        cast_events
            .iter()
            .any(|event| matches!(event, DomainEvent::AbilityCastSucceeded { .. }))
    );
    replace_terrain(&mut game, EAST, "demo.terrain.glass-wall");
    assert!(game.player_can_pass_walls());
    assert_eq!(game.player_wall_destruction_target(EAST), None);
    game.player.hp = 1;
    let update = dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(game.player.position, EAST);
    assert_eq!(
        game.terrain[game.index(EAST).unwrap()],
        "demo.terrain.glass-wall"
    );
    assert!(!update.events.iter().any(|event| matches!(
        event.message_key.as_str(),
        "player-wall-density" | "player-wall-crushed"
    )));
    assert!(!game.player_is_dead());
}

#[test]
fn forced_fall_uses_unmounted_passage_and_rejects_permanent_wall_candidates() {
    for spectre in [false, true] {
        let mut game = game(false);
        if spectre {
            game.player.statuses.push(form(SPECTRE, 100));
        }
        game.push_generated_actor("test.mount".to_owned(), "demo.actor.horse", START);
        game.entities[0].controller_id = Some(game.player.id.clone());
        game.riding_actor_id = Some("test.mount".to_owned());
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx != 0 || dy != 0 {
                    replace_terrain(
                        &mut game,
                        Position {
                            x: START.x + dx,
                            y: START.y + dy,
                        },
                        "demo.terrain.permanent-wall",
                    );
                }
            }
        }
        replace_terrain(&mut game, EAST, WALL);
        assert_eq!(
            game.resolve_riding_fall(1, true, &mut Vec::new(), &mut BTreeSet::new()),
            spectre
        );
        assert_eq!(game.player.position, if spectre { EAST } else { START });
        assert_eq!(game.riding_actor_id.is_none(), spectre);
    }
}
