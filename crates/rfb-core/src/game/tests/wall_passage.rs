// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use rfb_content::ActorMovementMode;

const SPECTRE: &str = "rfb-legacy.race.spectre";
const HUMAN: &str = "demo.race.rfb-human";
const WALL: &str = "demo.terrain.wall";
const FLOOR: &str = "demo.terrain.floor";
const START: Position = Position { x: 99, y: 33 };
const EAST: Position = Position { x: 100, y: 33 };

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
    for y in 31..=35 {
        for x in 97..=103 {
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
fn c4c_spectral_ordinary_equipment_inherited_breath_and_wall_lifecycle_survive_save() {
    let mut game = Game::new_with_build(509, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut game);
    prepare(&mut game, false);
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: "test.floor.depth-80".into(),
        depth: 80,
        source: LootSource::MonsterDeath {
            actor_id: "test.c4c".into(),
        },
    };
    let base = "demo.item.white-dragon-scale-mail";
    (0..100_000)
        .find_map(|_| {
            game.generate_loot_instances(&context, ItemLocation::Inventory)
                .unwrap()
                .into_iter()
                .find(|item| item.kind_id == base && item.artifact_name.is_none())
        })
        .expect("white scales must be reached through the complete ordinary pool");
    let kind = (0..30_000)
        .find_map(|_| {
            game.roll_fixed_artifact_kind_id(&context, Some(base), false)
                .filter(|id| id == "demo.item.spectral-dragon-scale-mail")
        })
        .expect("the observed base must reach spectral scales with all artifact gates intact");
    let draft = game.fixed_item_draft(&context, kind);
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Ground(START))
        .unwrap();
    let id = item.id.clone();
    game.items.push(item);
    game.pick_up_item_at_player(Some(&id)).unwrap();
    assert!(game.visible_item_passives(&game.items[0]).is_empty());
    assert!(!game.player_has_wall_passage());
    let original_max_hp = game.effective_player_max_hp();
    game.equip_inventory_item(&id, None).unwrap();
    game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
    assert_eq!(game.items[0].curse, Some(ItemCurseSeverityDto::Heavy));
    assert_eq!(game.carried_weight_tenths_pound(), 100);
    let stats = game.item_base_modifiers("demo.item.spectral-dragon-scale-mail");
    assert_eq!(
        (
            stats.defense,
            stats.strength,
            stats.dexterity,
            stats.constitution
        ),
        (60, -3, -3, -3)
    );
    let bonuses = game.player_equipment_bonuses();
    assert_eq!(
        (
            bonuses.melee_skill,
            bonuses.stealth_skill,
            bonuses.life_percent
        ),
        (-2, 3, -9)
    );
    assert!(game.effective_player_max_hp() < original_max_hp);
    assert_eq!(game.player_hold_life_sources(), 1);
    assert_eq!(game.player_see_invisible_sources(), 1);
    assert!(
        game.player_equipment_passives()
            .contains(&EquipmentPassive::ColdAura)
    );
    assert!(game.player_levitates());
    assert!(game.player_can_pass_walls());
    assert_eq!(game.player_incoming_damage_percent(), 100);
    let mut hit = game.clone();
    let hp = hit.player.hp;
    hit.resolve_monster_damage_to_player(
        "test.fire",
        "demo.actor.ogre",
        "test.fire",
        0,
        10,
        10,
        DamageType::Fire,
        &mut Vec::new(),
    );
    assert_eq!(
        hp - hit.player.hp,
        10,
        "passwall armor must not grant Wraithform damage reduction"
    );
    let traits = game.character_trait_details(&game.player_derived_stats());
    assert!(traits.passes_walls);
    let source = traits.sources.iter().find(|s| s.source_id == id).unwrap();
    assert!(source.passes_walls);
    assert!(
        source
            .passives
            .contains(&EquipmentPassiveDto::NoPasswallDamage)
    );
    let north = Position {
        x: EAST.x,
        y: EAST.y - 1,
    };
    replace_terrain(&mut game, EAST, WALL);
    replace_terrain(&mut game, north, "demo.terrain.permanent-wall");
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(game.player.position, EAST);
    assert_eq!(game.terrain[game.index(EAST).unwrap()], WALL);
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::North,
        },
    );
    assert_eq!(game.player.position, EAST);
    assert!(!game.player_can_enter_position(north));
    // With no artifact E line, the profile and program are exactly the base's.
    let base_profile = &game
        .content
        .item(base)
        .unwrap()
        .device_generation
        .as_ref()
        .unwrap()
        .activations[0];
    assert_eq!(
        game.items[0].activation.as_ref().unwrap().profile_id,
        base_profile.id
    );
    assert_eq!(
        game.content
            .item("demo.item.spectral-dragon-scale-mail")
            .unwrap()
            .device_generation,
        game.content.item(base).unwrap().device_generation
    );
    game.push_generated_actor(
        "test.breath-target".into(),
        "demo.actor.great-hell-wyrm",
        Position { x: 102, y: 33 },
    );
    let hp = game.entities[0].hp;
    let seed = (0..1000)
        .find(|seed| RfbRng::seeded(*seed).bounded(100) < 5)
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    game.reveal_current_visibility();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    let activate = |g: &mut Game| {
        let mut events = Vec::new();
        g.use_inventory_item(
            &id,
            Some(&TargetSelection::Direction {
                direction: Direction::East,
            }),
            None,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        events
    };
    let ready_tick = game.world_tick + 300;
    let events = activate(&mut game);
    assert_eq!(activate(&mut restored), events);
    assert!(events.iter().any(|e| matches!(e, DomainEvent::AbilityConeDamage { resolution, .. }
        if resolution.base_raw_damage == 150 && resolution.radius == 2 && resolution.damage_type == DamageTypeDto::Cold)));
    assert_eq!(hp - game.entities[0].hp, 150);
    assert_eq!(game.state_hash(), restored.state_hash());
    assert_eq!(game.items[0].charges.unwrap().current, 0);
    clear_monsters(&mut game);
    // Wait/rest in a wall must neither damage the wearer nor interrupt recovery.
    let hp = game.player.hp;
    dispatch_next(&mut game, GameCommand::Wait);
    assert!(game.player.hp >= hp);
    game.player.hp = game.effective_player_max_hp() / 2;
    let rest = game
        .resolve_player_rest(3, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .unwrap();
    assert_eq!(rest.completed_turns, 3);
    assert_ne!(rest.stop_reason, RestStopReasonDto::Damaged);
    assert_eq!(game.player.position, EAST);
    let slot = match &game.items[0].location {
        ItemLocation::Equipped { slot_id } => slot_id.clone(),
        _ => unreachable!(),
    };
    assert!(game.unequip_slot(&slot).is_none());
    give_inventory_item(&mut game, "test.replacement", base);
    assert!(
        game.equip_inventory_item("test.replacement", None)
            .is_none()
    );
    assert!(game.player_can_pass_walls());
    give_inventory_item(
        &mut game,
        "test.remove-curse",
        "demo.item.greater-cleansing-scroll",
    );
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.remove-curse".into(),
            target: None,
        },
    );
    assert_eq!(game.items[0].curse, None);
    let mut unequipped = game.clone();
    assert!(unequipped.unequip_slot(&slot).is_some());
    assert!(!unequipped.player_can_pass_walls());
    assert_eq!(unequipped.player.position, EAST);
    unequipped.player.statuses.push(form(SPECTRE, 100));
    assert!(unequipped.player_can_pass_walls());
    unequipped.world_tick = unequipped.world_tick.next_multiple_of(10);
    let mut density = Vec::new();
    assert!(unequipped.process_player_wall_damage(&mut density));
    assert!(density.iter().any(|e| matches!(
        e,
        DomainEvent::PlayerWallDamaged {
            crushing: false,
            ..
        }
    )));
    // A legal equipment replacement revokes the capability immediately in-wall.
    game.equip_inventory_item("test.replacement", None).unwrap();
    assert_eq!(game.player.position, EAST);
    assert!(!game.player_can_pass_walls());
    assert!(
        !game
            .character_trait_details(&game.player_derived_stats())
            .passes_walls
    );
    game.reveal_current_visibility();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    for run in [&mut game, &mut restored] {
        let hp = run.player.hp;
        while !run.world_tick.is_multiple_of(10) {
            run.world_tick += 1;
            run.process_inventory_device_recovery(&mut Vec::new());
        }
        let mut events = Vec::new();
        assert!(run.process_player_wall_damage(&mut events));
        assert!(run.player.hp < hp);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, DomainEvent::PlayerWallDamaged { crushing: true, .. }))
        );
        dispatch_next(
            run,
            GameCommand::Move {
                direction: Direction::West,
            },
        );
        assert_eq!(run.player.position, START);
        // Device recovery stays attached to the same saved item after replacement.
        let index = run.items.iter().position(|item| item.id == id).unwrap();
        while run.world_tick < ready_tick - 1 {
            run.world_tick += 1;
            run.process_inventory_device_recovery(&mut Vec::new());
        }
        assert_eq!(run.items[index].charges.unwrap().current, 0);
        run.world_tick += 1;
        run.process_inventory_device_recovery(&mut Vec::new());
        assert_eq!(run.items[index].charges.unwrap().current, 1);
    }
    assert_eq!(game.state_hash(), restored.state_hash());
    assert_eq!(
        game.generate_loot_instances(&context, ItemLocation::Inventory)
            .unwrap(),
        restored
            .generate_loot_instances(&context, ItemLocation::Inventory)
            .unwrap()
    );
    assert_eq!(game.rng, restored.rng);
}

#[test]
fn c4c_spectral_rider_still_requires_a_wall_passing_mount() {
    for mount_passes in [false, true] {
        let content_game = game_with_actor_definition(510, "demo.actor.horse", |actor| {
            actor.movement.modes = if mount_passes {
                vec![ActorMovementMode::PassWall]
            } else {
                Vec::new()
            };
        });
        let mut game = Game::from_content_with_build(
            510,
            content_game.content.clone(),
            DEFAULT_WORLD_ID,
            "demo.build.warrior",
        )
        .unwrap();
        choose_human_talent_if_pending(&mut game);
        prepare(&mut game, false);
        let context = LootContext {
            table_id: "demo.loot-table.base-items".into(),
            floor_id: "test.c4c-mounted".into(),
            depth: 80,
            source: LootSource::ItemUse {
                item_id: "test.acquirement".into(),
            },
        };
        let draft = game.fixed_item_draft(&context, "demo.item.spectral-dragon-scale-mail".into());
        let item = game
            .commit_generated_item_draft(draft, ItemLocation::Inventory)
            .unwrap();
        let id = item.id.clone();
        game.items.push(item);
        game.equip_inventory_item(&id, None).unwrap();
        game.push_generated_actor("test.mount".into(), "demo.actor.horse", START);
        game.entities[0].controller_id = Some(game.player.id.clone());
        game.riding_actor_id = Some("test.mount".into());
        replace_terrain(&mut game, EAST, WALL);
        assert_eq!(game.player_can_pass_walls(), mount_passes);
        assert_eq!(game.actor_can_enter_position(0, EAST), mount_passes);
        assert_eq!(
            game.character_trait_details(&game.player_derived_stats())
                .passes_walls,
            mount_passes
        );
        game.reveal_current_visibility();
        let mut restored =
            Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
        for run in [&mut game, &mut restored] {
            dispatch_next(
                run,
                GameCommand::Move {
                    direction: Direction::East,
                },
            );
            assert_eq!(run.player.position, if mount_passes { EAST } else { START });
            // An ordinary mount remains vulnerable to crushing if displaced into a wall.
            run.player.position = EAST;
            run.entities[0].position = EAST;
            run.world_tick = 10;
            assert_eq!(
                run.process_player_wall_damage(&mut Vec::new()),
                !mount_passes
            );
        }
        assert_eq!(game.state_hash(), restored.state_hash());
    }
}

#[test]
fn water_allows_player_and_monster_bolts_and_ammunition_stays_on_shore() {
    let mut game = game(false);
    let shallow = Position { x: 101, y: 33 };
    let target = Position { x: 102, y: 33 };
    replace_terrain(&mut game, EAST, "demo.terrain.surface-water-deep");
    replace_terrain(&mut game, shallow, "demo.terrain.surface-water-shallow");
    game.push_generated_actor(
        "test.water-target".to_owned(),
        "demo.actor.small-kobold",
        target,
    );
    let mut ability = game
        .content
        .ability("rfb-legacy.ability.bolt-physical-1d4")
        .unwrap()
        .clone();
    ability.effect = rfb_content::AbilityEffectDefinition::Damage {
        damage_dice: 1,
        damage_sides: 1,
        damage_bonus: 0,
        damage_type: rfb_content::ActorDamageType::Fire,
    };
    let plan = game
        .monster_ability_target_plan(0, ability.clone(), 1)
        .expect("monster bolt should cross shallow and deep water");
    let hp = game.player.hp;
    game.resolve_monster_ability_plan(
        0,
        "demo.actor.small-kobold",
        &plan,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    assert!(game.player.hp < hp);
    let hp = game.entities[0].hp;
    game.resolve_player_projectile_damage_effect(
        &ability,
        vec![EAST, shallow, target],
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(game.entities.first().map_or(0, |actor| actor.hp) < hp);

    for wall in ["demo.terrain.glass-wall", "demo.terrain.permanent-wall"] {
        replace_terrain(&mut game, EAST, wall);
        assert!(!projectile_geometry::has_line_of_effect(
            &game, START, target
        ));
        assert!(
            game.trace_projectile_path(vec![EAST, shallow, target])
                .0
                .traversed
                .is_empty()
        );
    }
    replace_terrain(&mut game, EAST, "demo.terrain.surface-water-deep");
    assert_eq!(game.trace_projectile_path(vec![EAST]).0.landing, EAST);
    give_inventory_item(&mut game, "test.water-ammunition", "demo.item.arrow");
    let ammunition = game.items.pop().unwrap();
    game.settle_projectile_ammunition(
        ammunition,
        EAST,
        false,
        0,
        &mut Vec::new(),
        &mut BTreeSet::new(),
    );
    let item = game
        .items
        .iter()
        .find(|item| item.id == "test.water-ammunition")
        .unwrap();
    let ItemLocation::Ground(landing) = item.location else {
        panic!("surviving ammunition should land");
    };
    assert_ne!(landing, EAST);
    assert!(game.is_walkable(landing));
    assert!(!game.is_walkable(EAST));
    game.reveal_current_visibility();
    assert_eq!(
        Game::from_save(game.to_save()).unwrap().state_hash(),
        game.state_hash()
    );
}

#[test]
fn dark_pit_requires_flight_but_allows_projectiles_and_keeps_drops_on_floor() {
    let pit = "demo.terrain.dark-pit";
    let mut game = game(false);
    replace_terrain(&mut game, EAST, pit);
    assert!(!game.player_can_enter_position(EAST));
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(game.player.position, START);
    assert!(projectile_geometry::has_line_of_effect(
        &game,
        START,
        Position { x: 101, y: 33 }
    ));
    let (trace, _) = game.trace_projectile_path(vec![EAST, Position { x: 101, y: 33 }]);
    assert_eq!(trace.traversed, vec![EAST, Position { x: 101, y: 33 }]);
    assert_eq!(game.trace_projectile_path(vec![EAST]).0.landing, EAST);
    give_inventory_item(&mut game, "test.pit-ammunition", "demo.item.arrow");
    let ammunition = game.items.pop().unwrap();
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();
    game.settle_projectile_ammunition(ammunition, EAST, false, 0, &mut events, &mut changed);
    let item = game
        .items
        .iter()
        .find(|item| item.id == "test.pit-ammunition")
        .unwrap();
    let ItemLocation::Ground(position) = item.location else {
        panic!("ammunition should land");
    };
    assert_ne!(position, EAST);
    assert!(game.is_walkable(position));
    assert!(changed.contains(&position));
    game.player.statuses.push(form(SPECTRE, 100));
    game.refresh_player_resource_maxima();
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(game.player.position, EAST);
    give_inventory_item(&mut game, "test.pit-drop", "demo.item.short-sword");
    let (_, _, position) = game
        .drop_inventory_quantity("test.pit-drop", 1)
        .unwrap()
        .unwrap();
    assert!(game.is_walkable(position));
    assert_ne!(position, EAST);
    game.reveal_current_visibility();
    assert_eq!(
        Game::from_save(game.to_save()).unwrap().state_hash(),
        game.state_hash()
    );
    replace_terrain(&mut game, EAST, "demo.terrain.glass-wall");
    assert!(!game.projectile_can_cross(EAST));
}

#[test]
fn a_flying_monster_killed_above_a_pit_drops_its_carried_item_on_nearby_floor() {
    let mut game = game(false);
    replace_terrain(&mut game, EAST, "demo.terrain.dark-pit");
    game.push_generated_actor("test.pit-bat".to_owned(), "demo.actor.fruit-bat", EAST);
    give_inventory_item(&mut game, "test.pit-loot", "demo.item.short-sword");
    game.items[0].location = ItemLocation::CarriedBy {
        actor_id: "test.pit-bat".to_owned(),
    };
    let mut changed = BTreeSet::new();
    game.resolve_actor_death(
        0,
        DomainEvent::PlayerSlew {
            target_kind_id: "demo.actor.fruit-bat".to_owned(),
            damage: DamageOutcome {
                raw: 1,
                armor_reduction: 0,
                requested: 1,
                applied: 1,
                resistance_delta: 0,
                damage_type: DamageType::Physical,
                resistance: ResistanceLevel::Normal,
            },
        },
        &mut Vec::new(),
        &mut changed,
        &mut Vec::new(),
    )
    .unwrap();
    let item = game
        .items
        .iter()
        .find(|item| item.id == "test.pit-loot")
        .unwrap();
    let ItemLocation::Ground(position) = item.location else {
        panic!("carried loot should drop");
    };
    assert!(game.is_walkable(position));
    assert_ne!(position, EAST);
    assert!(changed.contains(&position));
    game.reveal_current_visibility();
    assert_eq!(
        Game::from_save(game.to_save()).unwrap().state_hash(),
        game.state_hash()
    );
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
        Position { x: 101, y: 33 }
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

    game.player.position = Position { x: 188, y: 58 };
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
