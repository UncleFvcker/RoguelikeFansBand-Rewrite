// SPDX-License-Identifier: MPL-2.0

use super::*;

const RACE_PROBE_MONSTERS_ABILITY_ID: &str = "rfb.ability.race.probe-monsters";

const RACE_STONE_TO_MUD_ABILITY_ID: &str = "rfb.ability.race.stone-to-mud";

const RACE_WOOD_ELF_NATURE_AWARENESS_ABILITY_ID: &str =
    "rfb.ability.race.wood-elf-nature-awareness";

const RACE_OGRE_EXPLOSIVE_RUNE_ABILITY_ID: &str = "rfb.ability.race.explosive-rune";

const EXPLOSIVE_RUNE_TERRAIN_ID: &str = "demo.terrain.explosive-rune";

const ENT_TREE_POWER: &str = "rfb.ability.race.summon-tree";
const ENT_TREE_TERRAIN: &str = "demo.terrain.surface-tree";

fn ent_tree_game(level: u16, build: &str) -> Game {
    let mut game = crate::game::tests::hunger::ent_birth(440, build);
    clear_monsters(&mut game);
    game.items.clear();
    game.gold_piles.clear();
    game.player.position = Position { x: 10, y: 10 };
    for y in 8..=12 {
        for x in 8..=12 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game.progress.level = level;
    game.progress.max_level = level;
    game.refresh_character_skills();
    game.refresh_player_resource_maxima();
    game.player.hp = game.effective_player_max_hp();
    game
}

fn cast_ent_trees(game: &mut Game) -> (Vec<DomainEvent>, BTreeSet<Position>) {
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();
    game.resolve_player_ability(
        ENT_TREE_POWER,
        TargetSelection::SelfTarget,
        &mut events,
        &mut changed,
        &mut Vec::new(),
    )
    .unwrap();
    (events, changed)
}

fn no_trees_answer(events: &[DomainEvent]) -> bool {
    events.iter().any(|event| matches!(event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if resolution.effects.iter().any(|effect| matches!(effect,
                rfb_protocol::AbilityEffectResolutionDto::NoOp { reason, .. } if reason == "no-trees-answer"))
    ))
}

#[test]
fn ent_tree_power_unlock_cost_wisdom_failure_and_current_form_use_innate_pipeline() {
    for (level, cost) in [(9, 20), (10, 20), (44, 20), (45, 50), (50, 50)] {
        let mut game = ent_tree_game(level, "demo.build.warrior");
        assert_eq!(game.virtues[2].kind, VirtueKindDto::Nature);
        let power = game
            .snapshot()
            .player
            .abilities
            .into_iter()
            .find(|power| power.id == ENT_TREE_POWER)
            .unwrap();
        assert_eq!(power.minimum_level, 10);
        assert_eq!(power.source, AbilitySourceDto::Race);
        assert_eq!(
            power.governing_attribute,
            Some(rfb_protocol::AttributeKindDto::Wisdom)
        );
        assert_eq!((power.base_resource_cost, power.resource_cost), (20, cost));
        assert_eq!(power.can_cast, level >= 10);
        game.debug_set_ability_casts_succeed(true);
        let hp = game.player.hp;
        let (events, changed) = cast_ent_trees(&mut game);
        assert_eq!(
            game.player.hp,
            hp - if level >= 10 { cost as i32 } else { 0 }
        );
        if level == 9 {
            assert!(changed.is_empty());
            assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityCastUnavailable { reason, .. } if reason == "level-too-low")));
        }
    }
    let mut game = ent_tree_game(10, "demo.build.high-mage-death");
    let activation = game.content.race("rfb-legacy.race.ent").unwrap().abilities[0].clone();
    assert_eq!(activation.base_failure_percent, 70);
    game.progress.attributes.wisdom = 3;
    let poor = game.innate_power_failure_percent(&activation);
    game.progress.attributes.wisdom = 18;
    assert!(game.innate_power_failure_percent(&activation) < poor);
    let failure = game.innate_power_failure_percent(&activation);
    let seed = (0..1000)
        .find(|seed| RfbRng::seeded(*seed).bounded(100) < u64::from(failure))
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    game.resources
        .get_mut("demo.resource.mana")
        .unwrap()
        .current = 7;
    let hp = game.player.hp;
    let (events, changed) = cast_ent_trees(&mut game);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::AbilityCastFailed { .. }))
    );
    assert!(changed.is_empty());
    assert_eq!(game.rng.draw_counter, 1);
    assert_eq!(game.resources["demo.resource.mana"].current, 0);
    assert_eq!(game.player.hp, hp - 13);

    let virtues = game.virtues;
    assert_eq!(virtues[3].kind, VirtueKindDto::Nature);
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 1000, "test.ent-form").status;
    form.granted_race_id = Some("demo.race.rfb-human".to_owned());
    game.player.statuses.push(form);
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != ENT_TREE_POWER)
    );
    game.build.as_mut().unwrap().race_id = "demo.race.rfb-human".to_owned();
    game.player.statuses.last_mut().unwrap().granted_race_id =
        Some("rfb-legacy.race.ent".to_owned());
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .any(|ability| ability.id == ENT_TREE_POWER)
    );
    assert_eq!(
        game.virtues, virtues,
        "temporary form must not redraw birth virtues"
    );
    game.debug_set_ability_casts_succeed(true);
    let (events, _) = cast_ent_trees(&mut game);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::AbilityCastSucceeded { .. }))
    );
    game.player.statuses.clear();
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != ENT_TREE_POWER)
    );
}

#[test]
fn ent_tree_attempts_match_original_keypad_rng_including_zero_five_and_missing_nine() {
    let mut base = ent_tree_game(44, "demo.build.warrior");
    base.debug_set_ability_casts_succeed(true);
    let center = base.player.position;
    let mut saw_five = false;
    let mut saw_zero = false;
    for only_north_east in [false, true] {
        for seed in 0..64 {
            let mut game = base.clone();
            let allowed: BTreeSet<_> = TERRAIN_INTERACTION_DIRECTIONS
                .iter()
                .map(|direction| game.position_in_direction(*direction))
                .filter(|position| !only_north_east || *position == Position { x: 11, y: 9 })
                .collect();
            for direction in TERRAIN_INTERACTION_DIRECTIONS {
                let position = game.position_in_direction(direction);
                if !allowed.contains(&position) {
                    replace_terrain(&mut game, position, "demo.terrain.wall");
                }
            }
            let mut oracle = RfbRng::seeded(seed);
            oracle.bounded(100); // innate cast failure roll
            let mut attempts = 0;
            let mut expected = None;
            while attempts < 5 {
                let dir = oracle.bounded(9) as usize;
                saw_five |= dir == 5;
                if dir == 5 {
                    continue;
                }
                attempts += 1;
                saw_zero |= dir == 0;
                let x = center.x + [0, -1, 0, 1, -1, 0, 1, -1, 0, 1][dir];
                let y = center.y + [0, 1, 1, 1, 0, 0, 0, -1, -1, -1][dir];
                let position = Position { x, y };
                if position != center && allowed.contains(&position) {
                    expected = Some(position);
                    break;
                }
            }
            game.rng = RfbRng::seeded(seed);
            let hp = game.player.hp;
            let (events, changed) = cast_ent_trees(&mut game);
            assert_eq!(
                changed,
                expected.into_iter().collect(),
                "seed {seed}, NE only {only_north_east}"
            );
            assert_eq!(game.rng, oracle);
            assert_eq!(game.player.hp, hp - 20);
            assert_eq!(no_trees_answer(&events), expected.is_none());
            assert!(
                game.entities.is_empty(),
                "the power creates terrain, not actors"
            );
            assert_eq!(game.terrain_at(center), "demo.terrain.floor");
        }
    }
    assert!(saw_five && saw_zero);
}

#[test]
fn ent_level_45_creates_all_adjacent_trees_on_original_floor_types_without_target_rng() {
    let base = ent_tree_game(45, "demo.build.warrior");
    let AbilityEffectDefinition::CreateAdjacentTerrain {
        source_terrain_ids, ..
    } = &base.content.ability(ENT_TREE_POWER).unwrap().effect
    else {
        panic!("tree creation effect")
    };
    assert_eq!(source_terrain_ids.len(), 12);
    for terrain in source_terrain_ids {
        let mut game = base.clone();
        game.debug_set_ability_casts_succeed(true);
        let positions: BTreeSet<_> = TERRAIN_INTERACTION_DIRECTIONS
            .iter()
            .map(|direction| game.position_in_direction(*direction))
            .collect();
        for position in &positions {
            replace_terrain(&mut game, *position, terrain);
        }
        game.rng = RfbRng::seeded(17);
        let (events, changed) = cast_ent_trees(&mut game);
        assert_eq!(changed, positions, "{terrain}");
        assert_eq!(game.rng.draw_counter, 1);
        assert!(!no_trees_answer(&events));
        assert!(
            positions
                .iter()
                .all(|position| game.terrain_at(*position) == ENT_TREE_TERRAIN)
        );
    }
}

#[test]
fn ent_tree_creation_respects_occupied_special_and_boundary_squares() {
    let base = ent_tree_game(45, "demo.build.warrior");
    for blocked in [
        "item",
        "gold",
        "monster",
        "stairs",
        "connection",
        "rune",
        "tree",
        "wall",
        "deep-water",
        "border",
    ] {
        for level in [44, 45] {
            let mut game = base.clone();
            game.progress.level = level;
            game.debug_set_ability_casts_succeed(true);
            if blocked == "border" {
                game.player.position = Position {
                    x: i32::from(game.width) - 2,
                    y: 10,
                };
            }
            for direction in TERRAIN_INTERACTION_DIRECTIONS {
                let position = game.position_in_direction(direction);
                replace_terrain(&mut game, position, "demo.terrain.wall");
            }
            let target = game.position_in_direction(Direction::East);
            replace_terrain(&mut game, target, "demo.terrain.floor");
            match blocked {
                "item" => {
                    give_inventory_item(&mut game, "test.ent.blocker", "demo.item.ration-of-food");
                    game.items.last_mut().unwrap().location = ItemLocation::Ground(target);
                }
                "gold" => game.gold_piles.push(GoldPile {
                    id: "test.ent.gold".to_owned(),
                    position: target,
                    amount: 1,
                    appearance: GoldAppearanceDto::Gold,
                    discovered: false,
                }),
                "monster" => game.push_generated_actor(
                    "test.ent.blocker".to_owned(),
                    "demo.actor.newt",
                    target,
                ),
                "connection" => {
                    game.floor_connections
                        .push(crate::state::FloorConnectionState {
                            id: "test.ent.connection".to_owned(),
                            position: target,
                            target_floor_id: None,
                            target_connection_id: None,
                        });
                }
                "stairs" => replace_terrain(&mut game, target, "demo.terrain.stairs-down"),
                "rune" => replace_terrain(&mut game, target, "demo.terrain.warding-glyph"),
                "tree" => replace_terrain(&mut game, target, ENT_TREE_TERRAIN),
                "wall" => replace_terrain(&mut game, target, "demo.terrain.wall"),
                "deep-water" => {
                    replace_terrain(&mut game, target, "demo.terrain.surface-water-deep")
                }
                "border" => (),
                _ => unreachable!(),
            }
            let before = game.terrain.clone();
            let hp = game.player.hp;
            let (events, changed) = cast_ent_trees(&mut game);
            if blocked == "border" && level == 45 {
                assert_eq!(changed, BTreeSet::from([target]));
                assert_eq!(game.terrain_at(target), ENT_TREE_TERRAIN);
            } else {
                assert!(changed.is_empty(), "{blocked} at {level}");
                assert_eq!(game.terrain, before);
            }
            assert_eq!(game.player.hp, hp - if level == 44 { 20 } else { 50 });
            assert_eq!(no_trees_answer(&events), level == 44);
        }
    }
}

#[test]
fn ent_created_trees_block_sight_allow_tree_movement_and_persist_through_save() {
    let mut game = ent_tree_game(45, "demo.build.warrior");
    game.debug_set_ability_casts_succeed(true);
    game.player.position = Position { x: 48, y: 16 };
    for y in 14..=18 {
        for x in 46..=50 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    let beyond = Position { x: 50, y: 16 };
    assert!(crate::game::visibility::has_line_of_sight(
        &game,
        game.player.position,
        beyond
    ));
    dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: ENT_TREE_POWER.to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    assert!(!crate::game::visibility::has_line_of_sight(
        &game,
        game.player.position,
        beyond
    ));
    assert_eq!(game.terrain_at(Position { x: 49, y: 16 }), ENT_TREE_TERRAIN);
    let before_tick = game.world_tick;
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(game.player.position, Position { x: 49, y: 16 });
    assert!(game.world_tick > before_tick);
    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert_eq!(restored.snapshot(), game.snapshot());
    assert_eq!(restored.state_hash(), game.state_hash());
    for state in [&mut game, &mut restored] {
        dispatch_next(
            state,
            GameCommand::Move {
                direction: Direction::East,
            },
        );
    }
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn formal_ogre_sustains_intelligence_and_places_capped_explosive_runes() {
    let mut game = ogre_game(423);
    clear_monsters(&mut game);
    assert!(game.player_sustains_attribute(AttributeKind::Intelligence));
    assert!(
        game.virtues
            .iter()
            .any(|virtue| virtue.kind == VirtueKindDto::Temperance)
    );

    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 20, "test.ogre-form").status;
    form.granted_race_id = Some("rfb-legacy.race.small-kobold".to_owned());
    game.player.statuses.push(form);
    assert!(!game.player_sustains_attribute(AttributeKind::Intelligence));
    game.player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);

    game.progress.level = 24;
    let locked = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_OGRE_EXPLOSIVE_RUNE_ABILITY_ID)
        .expect("Explosive Rune should project before unlocking");
    assert!(!locked.can_cast);
    assert_eq!(locked.minimum_level, 25);
    assert_eq!((locked.base_resource_cost, locked.resource_cost), (35, 35));
    assert_eq!(
        locked.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Intelligence)
    );
    assert!(matches!(
        locked.effects.as_slice(),
        [AbilityEffectSpecDto::CreateCurrentTerrain {
            target_terrain_id,
            ..
        }] if target_terrain_id == EXPLOSIVE_RUNE_TERRAIN_ID
    ));

    game.progress.level = 25;
    game.progress.max_level = 25;
    game.refresh_character_skills();
    game.player.hp = game.effective_player_max_hp();
    let position = game.player.position;
    replace_terrain(&mut game, position, "demo.terrain.floor");
    game.floor_connections
        .retain(|connection| connection.position != position);
    game.items.retain(
        |item| !matches!(item.location, ItemLocation::Ground(ground) if ground == position),
    );
    game.gold_piles.retain(|pile| pile.position != position);

    let available = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_OGRE_EXPLOSIVE_RUNE_ABILITY_ID)
        .expect("level-twenty-five Explosive Rune");
    assert!(available.can_cast);
    let failure_seed = (0..1_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(100) < u64::from(available.failure_percent)
        })
        .expect("Explosive Rune should have a failing percentile seed");
    let mut failed = game.clone();
    failed.rng = RfbRng::seeded(failure_seed);
    let failed_hp = failed.player.hp;
    let mut failed_events = Vec::new();
    failed
        .resolve_player_ability(
            RACE_OGRE_EXPLOSIVE_RUNE_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut failed_events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("failed Explosive Rune should resolve");
    assert_eq!(failed.player.hp, failed_hp - 35);
    assert_eq!(failed.terrain_at(position), "demo.terrain.floor");
    assert!(matches!(
        failed_events.first(),
        Some(DomainEvent::AbilityCastFailed { .. })
    ));

    game.debug_set_ability_casts_succeed(true);
    let hp_before = game.player.hp;
    game.resolve_player_ability(
        RACE_OGRE_EXPLOSIVE_RUNE_ABILITY_ID,
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Explosive Rune should resolve");
    assert_eq!(game.player.hp, hp_before - 35);
    assert_eq!(game.terrain_at(position), EXPLOSIVE_RUNE_TERRAIN_ID);
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("placed Explosive Rune should restore");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.terrain_at(position), EXPLOSIVE_RUNE_TERRAIN_ID);

    let mut capped = ogre_game(424);
    let player_index = capped
        .index(capped.player.position)
        .expect("player position");
    let mut rune_count = 0;
    for (index, terrain_id) in capped.terrain.iter_mut().enumerate() {
        if rune_count < 11 && index != player_index {
            *terrain_id = EXPLOSIVE_RUNE_TERRAIN_ID.to_owned();
            rune_count += 1;
        }
    }
    capped.terrain[player_index] = "demo.terrain.floor".to_owned();
    assert_eq!(rune_count, 11);
    assert!(
        capped
            .current_terrain_creation_replacement(
                &["demo.terrain.floor".to_owned()],
                EXPLOSIVE_RUNE_TERRAIN_ID,
            )
            .is_none()
    );
}

#[test]
fn explosive_rune_step_explodes_or_is_destroyed_by_the_authoritative_roll() {
    let mut base = ogre_game(425);
    clear_monsters(&mut base);
    base.progress.level = 25;
    base.progress.max_level = 25;
    base.refresh_character_skills();
    let rune = Position { x: 10, y: 10 };
    let start = Position { x: 11, y: 10 };
    let bystander = Position { x: 10, y: 11 };
    let safe = Position { x: 5, y: 5 };
    for position in [rune, start, bystander, safe] {
        replace_terrain(&mut base, position, "demo.terrain.floor");
    }
    replace_terrain(&mut base, rune, EXPLOSIVE_RUNE_TERRAIN_ID);
    base.player.position = safe;
    base.push_generated_actor(
        "test.ogre-rune-stepper".to_owned(),
        "demo.actor.newt",
        start,
    );
    let monster_level = base
        .content
        .actor("demo.actor.newt")
        .expect("Newt definition")
        .level;
    let break_roll_sides = 299_u64 * u64::from(base.progress.level) / 50;
    let explode_seed = (0..10_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(break_roll_sides) + 1 > u64::from(monster_level)
        })
        .expect("Explosive Rune should have a triggering seed");
    let disarm_seed = (0..10_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(break_roll_sides) < u64::from(monster_level)
        })
        .expect("Explosive Rune should have a destruction seed");

    let restored = Game::from_save_with_content(base.to_save(), base.content.clone())
        .expect("armed Explosive Rune should restore");
    assert_eq!(restored.state_hash(), base.state_hash());

    let mut exploded = base.clone();
    exploded.push_generated_actor(
        "test.ogre-rune-bystander".to_owned(),
        "demo.actor.newt",
        bystander,
    );
    exploded.rng = RfbRng::seeded(explode_seed);
    let mut events = Vec::new();
    let mut removed = Vec::new();
    assert_eq!(
        exploded
            .move_entity(0, rune, &mut events, &mut BTreeSet::new(), &mut removed)
            .expect("monster rune step should resolve"),
        ActorStepOutcome::Removed
    );
    assert_eq!(exploded.terrain_at(rune), "demo.terrain.floor");
    assert!(removed.contains(&"test.ogre-rune-stepper".to_owned()));
    assert!(removed.contains(&"test.ogre-rune-bystander".to_owned()));
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityAreaDamage {
            ability_id,
            resolution,
            ..
        } if ability_id == RACE_OGRE_EXPLOSIVE_RUNE_ABILITY_ID
            && resolution.center == rune
            && resolution.radius == 2
            && (64..=148).contains(&resolution.base_raw_damage)
            && resolution.target_count == 2
    )));

    let mut disarmed = base;
    disarmed.rng = RfbRng::seeded(disarm_seed);
    let hp_before = disarmed.entities[0].hp;
    let mut events = Vec::new();
    assert_eq!(
        disarmed
            .move_entity(0, rune, &mut events, &mut BTreeSet::new(), &mut Vec::new(),)
            .expect("monster rune destruction should resolve"),
        ActorStepOutcome::Moved
    );
    assert_eq!(disarmed.terrain_at(rune), "demo.terrain.floor");
    assert_eq!(disarmed.entities[0].hp, hp_before);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MonsterTerrainDestroyed {
            terrain_kind_id,
            replacement_terrain_kind_id,
            position,
            ..
        } if terrain_kind_id == EXPLOSIVE_RUNE_TERRAIN_ID
            && replacement_terrain_kind_id == "demo.terrain.floor"
            && *position == rune
    )));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::AbilityAreaDamage { .. }))
    );
}

#[test]
fn formal_wood_elf_nature_awareness_unlocks_at_twenty_and_reuses_full_detection() {
    let mut game = wood_elf_game(385);
    clear_monsters(&mut game);
    game.progress.level = 19;
    let locked = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_WOOD_ELF_NATURE_AWARENESS_ABILITY_ID)
        .expect("Wood-Elf Nature Awareness should be projected before it unlocks");
    assert_eq!(locked.source, AbilitySourceDto::Race);
    assert_eq!(
        locked.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Wisdom)
    );
    assert_eq!(locked.minimum_level, 20);
    assert_eq!((locked.base_resource_cost, locked.resource_cost), (15, 15));
    assert!(!locked.can_cast);

    game.progress.level = 20;
    game.progress.max_level = 20;
    game.refresh_character_skills();
    game.player.hp = game.effective_player_max_hp();
    let available = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_WOOD_ELF_NATURE_AWARENESS_ABILITY_ID)
        .expect("level-twenty Wood-Elf Nature Awareness");
    assert!(available.can_cast);
    assert!(available.failure_percent >= 50);
    assert_eq!(available.effects.len(), 6);
    assert!(
        available
            .effects
            .iter()
            .all(|effect| matches!(effect, AbilityEffectSpecDto::Detect { radius: 30, .. }))
    );

    game.debug_set_ability_casts_succeed(true);
    game.player.position = Position { x: 3, y: 3 };
    let trap = Position { x: 4, y: 3 };
    let door = Position { x: 5, y: 3 };
    let stairs_down = Position { x: 6, y: 3 };
    let stairs_up = Position { x: 7, y: 3 };
    let monster = Position { x: 3, y: 4 };
    for (position, terrain_id) in [
        (game.player.position, "demo.terrain.floor"),
        (trap, "demo.terrain.created-trap"),
        (door, "demo.terrain.door-secret"),
        (stairs_down, "demo.terrain.stairs-down"),
        (stairs_up, "demo.terrain.stairs-up"),
        (monster, "demo.terrain.floor"),
    ] {
        replace_terrain(&mut game, position, terrain_id);
    }
    for position in [trap, door, stairs_down, stairs_up] {
        let index = game.index(position).expect("detection target should exist");
        game.explored[index] = false;
        game.revealed_terrain.remove(&position);
    }
    game.push_generated_actor(
        "test.wood-elf-detection".to_owned(),
        "demo.actor.sheep",
        monster,
    );

    let hp_before = game.player.hp;
    let mut replay = game.clone();
    for cast in [&mut game, &mut replay] {
        let mut events = Vec::new();
        cast.resolve_player_ability(
            RACE_WOOD_ELF_NATURE_AWARENESS_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Wood-Elf Nature Awareness should resolve");

        let detections = events
            .iter()
            .filter_map(|event| match event {
                DomainEvent::AbilityDetected { resolution, .. } => Some(resolution),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(detections.len(), 6);
        for category in ["map", "trap", "door", "stairs-down", "stairs-up"] {
            assert!(
                detections
                    .iter()
                    .any(|detection| detection.category == category)
            );
        }
        assert!(detections.iter().any(|detection| {
            detection.category == "normal-monster"
                && detection
                    .detected_entity_ids
                    .iter()
                    .any(|id| id == "test.wood-elf-detection")
        }));
        assert!(cast.revealed_terrain.contains(&trap));
        assert!(cast.revealed_terrain.contains(&door));
        assert!(cast.explored[cast.index(stairs_down).expect("stairs should exist")]);
        assert!(cast.explored[cast.index(stairs_up).expect("stairs should exist")]);
    }
    assert_eq!(game.player.hp, hp_before - 15);
    assert_eq!(game.state_hash(), replay.state_hash());
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("Wood-Elf detection knowledge should restore");
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn formal_dwarf_detection_powers_reveal_original_terrain_categories_only() {
    let mut game = Game::new_with_build_race_and_name(
        94,
        "demo.build.high-mage-death",
        "rfb-legacy.race.dwarf",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Dwarf High-Mage should create");
    clear_monsters(&mut game);
    assert_eq!(game.player_infravision_range(), 5);
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Blindness),
        ResistanceLevel::Resistant
    );

    let level_four_experience = crate::stats::experience_required_for_level(4);
    game.apply_unscaled_player_experience(level_four_experience, &mut Vec::new());
    let snapshot = game.snapshot();
    let doors = snapshot
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == RACE_DETECT_DOORS_ABILITY_ID)
        .expect("Dwarf door detection should be projected before it unlocks");
    assert_eq!(doors.source, AbilitySourceDto::Race);
    assert_eq!(
        doors.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Wisdom)
    );
    assert_eq!(doors.minimum_level, 5);
    assert_eq!(doors.base_resource_cost, 5);
    assert!(!doors.can_cast);
    let treasure = snapshot
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == RACE_DETECT_TREASURE_ABILITY_ID)
        .expect("Dwarf treasure detection should be projected before it unlocks");
    assert_eq!(treasure.source, AbilitySourceDto::Race);
    assert_eq!(
        treasure.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Charisma)
    );
    assert_eq!(treasure.minimum_level, 10);
    assert_eq!(treasure.base_resource_cost, 5);
    assert!(!treasure.can_cast);

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(5) - level_four_experience,
        &mut Vec::new(),
    );
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana");
    mana.current = mana.maximum;
    let snapshot = game.snapshot();
    assert!(
        snapshot
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == RACE_DETECT_DOORS_ABILITY_ID)
            .expect("Dwarf door detection should remain projected")
            .can_cast
    );
    assert!(
        !snapshot
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == RACE_DETECT_TREASURE_ABILITY_ID)
            .expect("Dwarf treasure detection should remain projected")
            .can_cast
    );

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(9)
            - crate::stats::experience_required_for_level(5),
        &mut Vec::new(),
    );
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should retain mana");
    mana.current = mana.maximum;
    assert!(
        !game
            .snapshot()
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == RACE_DETECT_TREASURE_ABILITY_ID)
            .expect("Dwarf treasure detection should remain projected")
            .can_cast
    );

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(10)
            - crate::stats::experience_required_for_level(9),
        &mut Vec::new(),
    );
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should retain mana");
    mana.current = mana.maximum;
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == RACE_DETECT_TREASURE_ABILITY_ID)
            .expect("Dwarf treasure detection should unlock at level ten")
            .can_cast
    );

    game.debug_set_ability_casts_succeed(true);
    game.player.position = Position { x: 3, y: 3 };
    let player = game.player.position;
    let blocker = Position { x: 4, y: 3 };
    let trap = Position { x: 5, y: 3 };
    let door = Position { x: 6, y: 3 };
    let stairs_down = Position { x: 7, y: 3 };
    let stairs_up = Position { x: 8, y: 3 };
    let magma = Position { x: 5, y: 4 };
    let quartz = Position { x: 6, y: 4 };
    let gold_blocker = Position { x: 4, y: 2 };
    let gold = Position { x: 5, y: 2 };
    for (position, terrain_id) in [
        (player, "demo.terrain.floor"),
        (blocker, "demo.terrain.wall"),
        (trap, "demo.terrain.created-trap"),
        (door, "demo.terrain.door-secret"),
        (stairs_down, "demo.terrain.stairs-down"),
        (stairs_up, "demo.terrain.stairs-up"),
        (magma, "demo.terrain.magma-hidden-treasure"),
        (quartz, "demo.terrain.quartz-hidden-treasure"),
        (gold_blocker, "demo.terrain.wall"),
        (gold, "demo.terrain.floor"),
    ] {
        replace_terrain(&mut game, position, terrain_id);
    }
    for position in [trap, door, stairs_down, stairs_up, magma, quartz] {
        let index = game.index(position).expect("detection target should exist");
        game.explored[index] = false;
        game.revealed_terrain.remove(&position);
    }
    game.gold_piles = vec![GoldPile {
        id: "generated.gold.1".to_owned(),
        position: gold,
        amount: 25,
        appearance: GoldAppearanceDto::Gold,
        discovered: false,
    }];
    game.next_gold_pile_serial = 2;
    let mana_before = game.resources["demo.resource.mana"].current;
    let mut replay = game.clone();

    for cast in [&mut game, &mut replay] {
        cast.resolve_player_ability(
            RACE_DETECT_DOORS_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Dwarf door detection should resolve");
        assert!(cast.revealed_terrain.contains(&trap));
        assert!(cast.revealed_terrain.contains(&door));
        assert!(cast.explored[cast.index(stairs_down).expect("stairs should exist")]);
        assert!(cast.explored[cast.index(stairs_up).expect("stairs should exist")]);
        assert!(!cast.revealed_terrain.contains(&magma));
        assert!(!cast.revealed_terrain.contains(&quartz));

        cast.resolve_player_ability(
            RACE_DETECT_TREASURE_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Dwarf treasure detection should resolve");
        assert!(cast.revealed_terrain.contains(&magma));
        assert!(cast.revealed_terrain.contains(&quartz));
        assert!(!cast.gold_piles[0].discovered);
    }

    assert_eq!(
        game.resources["demo.resource.mana"].current,
        mana_before - 10
    );
    assert_eq!(game.state_hash(), replay.state_hash());
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("Dwarf detection knowledge should restore");
    for position in [trap, door, magma, quartz] {
        assert!(restored.revealed_terrain.contains(&position));
    }
    assert!(
        restored.explored[restored
            .index(stairs_down)
            .expect("restored stairs should exist")]
    );
    assert!(
        restored.explored[restored
            .index(stairs_up)
            .expect("restored stairs should exist")]
    );
    assert!(!restored.gold_piles[0].discovered);
}

#[test]
fn formal_nibelung_intrinsics_and_detection_powers_unlock_at_level_ten() {
    let mut game = Game::new_with_build_race_and_name(
        97,
        "demo.build.high-mage-death",
        "rfb-legacy.race.nibelung",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Nibelung High-Mage should create");
    assert_eq!(game.player_infravision_range(), 5);
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Dark),
        ResistanceLevel::Resistant
    );
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Disenchant),
        ResistanceLevel::Resistant
    );

    let level_nine_experience = crate::stats::experience_required_for_level(9);
    game.apply_unscaled_player_experience(level_nine_experience, &mut Vec::new());
    let snapshot = game.snapshot();
    for (ability_id, attribute) in [
        (
            RACE_DETECT_DOORS_ABILITY_ID,
            rfb_protocol::AttributeKindDto::Wisdom,
        ),
        (
            RACE_DETECT_TREASURE_ABILITY_ID,
            rfb_protocol::AttributeKindDto::Charisma,
        ),
    ] {
        let ability = snapshot
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == ability_id)
            .expect("Nibelung detection power should be projected");
        assert_eq!(ability.source, AbilitySourceDto::Race);
        assert_eq!(ability.governing_attribute, Some(attribute));
        assert_eq!(ability.minimum_level, 10);
        assert_eq!(ability.base_resource_cost, 5);
        assert!(!ability.can_cast);
    }

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(10) - level_nine_experience,
        &mut Vec::new(),
    );
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana");
    mana.current = mana.maximum;
    let snapshot = game.snapshot();
    for ability_id in [
        RACE_DETECT_DOORS_ABILITY_ID,
        RACE_DETECT_TREASURE_ABILITY_ID,
    ] {
        assert!(
            snapshot
                .player
                .abilities
                .iter()
                .find(|ability| ability.id == ability_id)
                .expect("Nibelung detection power should remain projected")
                .can_cast
        );
    }
}

#[test]
fn formal_half_giant_stone_to_mud_does_not_grant_mining_rewards() {
    let mut game = Game::new_with_build_race_and_name(
        99,
        "demo.build.high-mage-death",
        "rfb-legacy.race.half-giant",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Half-Giant High-Mage should create");
    clear_monsters(&mut game);
    game.items.clear();
    game.gold_piles.clear();
    assert_eq!(game.player_infravision_range(), 3);
    assert!(game.player_sustains_attribute(AttributeKind::Strength));
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Shards),
        ResistanceLevel::Resistant
    );

    let level_nineteen_experience = crate::stats::experience_required_for_level(19);
    game.apply_unscaled_player_experience(level_nineteen_experience, &mut Vec::new());
    let racial = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_STONE_TO_MUD_ABILITY_ID)
        .expect("Half-Giant Stone to Mud should be projected");
    assert_eq!(racial.source, AbilitySourceDto::Race);
    assert_eq!(
        racial.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Strength)
    );
    assert_eq!(racial.minimum_level, 20);
    assert_eq!(racial.base_resource_cost, 10);
    assert!(!racial.can_cast);

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(20) - level_nineteen_experience,
        &mut Vec::new(),
    );
    game.debug_set_ability_casts_succeed(true);
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana");
    mana.current = mana.maximum;
    let mana_before = mana.current;
    let target = game.position_in_direction(Direction::East);
    replace_terrain(&mut game, target, "demo.terrain.quartz-vein");
    game.progress.mining_proficiency = 3_999;
    let materials_before = game.progress.materials.clone();
    let item_serial_before = game.next_item_instance_serial;

    game.resolve_player_ability(
        RACE_STONE_TO_MUD_ABILITY_ID,
        TargetSelection::Direction {
            direction: Direction::East,
        },
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Half-Giant Stone to Mud should resolve");

    assert_eq!(game.terrain_at(target), "demo.terrain.floor");
    assert_eq!(
        game.resources["demo.resource.mana"].current,
        mana_before - 10
    );
    assert_eq!(game.progress.mining_proficiency, 3_999);
    assert_eq!(game.progress.materials, materials_before);
    assert!(game.gold_piles.is_empty());
    assert!(game.items.is_empty());
    assert_eq!(game.next_item_instance_serial, item_serial_before);
}

#[test]
fn half_titan_probe_knowledge_survives_losing_the_race_power_and_reloading() {
    let mut game = Game::new_with_build_race_and_name(
        102,
        "demo.build.high-mage-death",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Human High-Mage should create");
    clear_monsters(&mut game);
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.half-titan-form").status;
    form.granted_race_id = Some("rfb-legacy.race.half-titan".to_owned());
    game.player.statuses.push(form);
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Chaos),
        ResistanceLevel::Resistant
    );

    let level_fourteen_experience = crate::stats::experience_required_for_level(14);
    game.apply_unscaled_player_experience(level_fourteen_experience, &mut Vec::new());
    let racial = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_PROBE_MONSTERS_ABILITY_ID)
        .expect("temporary Half-Titan form should grant monster probing");
    assert_eq!(racial.source, AbilitySourceDto::Race);
    assert_eq!(
        racial.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Intelligence)
    );
    assert_eq!(racial.minimum_level, 15);
    assert_eq!(racial.base_resource_cost, 10);
    assert!(!racial.can_cast);

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(15) - level_fourteen_experience,
        &mut Vec::new(),
    );
    game.debug_set_ability_casts_succeed(true);
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana");
    mana.current = mana.maximum;
    let mana_before = mana.current;
    let target = game.position_in_direction(Direction::East);
    replace_terrain(&mut game, target, "demo.terrain.floor");
    let target_index = game.index(target).expect("probe target should exist");
    game.glow[target_index] = true;
    game.push_generated_actor(
        "test.half-titan-probe".to_owned(),
        "demo.actor.sheep",
        target,
    );
    let mut events = Vec::new();
    game.resolve_player_ability(
        RACE_PROBE_MONSTERS_ABILITY_ID,
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Half-Titan monster probing should resolve");
    assert_eq!(
        game.resources["demo.resource.mana"].current,
        mana_before - 10
    );
    assert!(game.probed_actor_kind_ids.contains("demo.actor.sheep"));
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityMonstersProbed { resolution, .. }
            if resolution.monsters.iter().any(|monster| monster.kind_id == "demo.actor.sheep")
    )));

    game.player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    game.refresh_player_resource_maxima();
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != RACE_PROBE_MONSTERS_ABILITY_ID)
    );
    assert!(game.probed_actor_kind_ids.contains("demo.actor.sheep"));
    let hash = game.state_hash();
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("probe knowledge should not require a current Sniper or Half-Titan source");
    assert!(restored.probed_actor_kind_ids.contains("demo.actor.sheep"));
    assert_eq!(restored.state_hash(), hash);
}
