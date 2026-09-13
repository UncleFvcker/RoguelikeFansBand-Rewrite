// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

const REWARDS: [(&str, &str, u16); 4] = [
    ("fang-farmer-maggots-dog", "dog-collar-of-fang", 2),
    ("wolf-farmer-maggots-dog", "dog-collar-of-wolf", 2),
    ("grip-farmer-maggots-dog", "dog-collar-of-grip", 2),
    ("the-multi-hued-centipede", "multi-hued-centipede", 30),
];

#[test]
#[ignore = "explicit Q1 combat preparation for ordinary Tauri standalone acceptance"]
fn export_q1_desktop_save() {
    let input = std::path::PathBuf::from(std::env::var("ORDINARY_EQUIPMENT_INPUT").unwrap());
    let directory = input.parent().unwrap();
    let (header, payload) = rfb_save::decode(&std::fs::read(&input).unwrap()).unwrap();
    assert!(header.museum_binding.is_some());
    let mut game = Game::from_save(payload).unwrap();
    choose_human_talent_if_pending(&mut game);
    prepare_combat(&mut game, "demo.actor.fang-farmer-maggots-dog");
    let start = successful_kill_start(&game, "demo.item.dog-collar-of-fang");
    let mut game = start.clone();
    let mut steps = Vec::new();
    let attack = GameCommand::Move {
        direction: Direction::East,
    };
    dispatch_next(&mut game, attack.clone());
    steps.push(serde_json::json!({"command":attack,"hash":game.state_hash()}));
    assert!(
        !game
            .entities
            .iter()
            .any(|a| a.kind_id == "demo.actor.fang-farmer-maggots-dog")
    );
    let reward = game
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.dog-collar-of-fang")
        .unwrap()
        .id
        .clone();
    let move_to_reward = GameCommand::Move {
        direction: Direction::East,
    };
    dispatch_next(&mut game, move_to_reward.clone());
    steps.push(serde_json::json!({"command":move_to_reward,"hash":game.state_hash()}));
    if matches!(
        game.items
            .iter()
            .find(|item| item.id == reward)
            .unwrap()
            .location,
        ItemLocation::Ground(_)
    ) {
        dispatch_next(&mut game, GameCommand::PickUp);
        steps.push(serde_json::json!({"command":GameCommand::PickUp,"hash":game.state_hash()}));
    }
    for command in [
        GameCommand::Equip {
            item_id: reward.clone(),
            slot_id: None,
        },
        GameCommand::Wait,
    ] {
        dispatch_next(&mut game, command.clone());
        steps.push(serde_json::json!({"command":command,"hash":game.state_hash()}));
    }
    assert!(matches!(
        game.items
            .iter()
            .find(|item| item.id == reward)
            .unwrap()
            .location,
        ItemLocation::Equipped { .. }
    ));
    assert_eq!(game.equipment_modifiers().strength, 1);
    assert_eq!(
        Game::from_save(start.to_save()).unwrap().state_hash(),
        start.state_hash()
    );
    std::fs::write(
        directory.join("q1-fang.rfbsave"),
        rfb_save::encode(&header, &start.to_save()).unwrap(),
    )
    .unwrap();
    std::fs::write(
        directory.join("scenarios.json"),
        serde_json::to_vec_pretty(&serde_json::json!([
            {"name":"q1-fang","initialHash":start.state_hash(),"steps":steps}
        ]))
        .unwrap(),
    )
    .unwrap();
}

fn prepare_combat(game: &mut Game, actor_kind: &str) {
    clear_monsters(game);
    game.items.clear();
    game.player.position = Position { x: 10, y: 10 };
    for y in 9..=11 {
        for x in 9..=13 {
            replace_terrain(game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game.push_generated_actor(
        "test.q1.target".into(),
        actor_kind,
        Position { x: 11, y: 10 },
    );
    // Shortened real melee: preserve source maximum HP and all actor rules.
    game.entities[0].hp = 1;
    game.entities[0].nice = true;
    game.entities[0].energy_need = STANDARD_ACTION_COST;
    game.player.hp = game.effective_player_max_hp();
    game.reveal_current_visibility();
}

fn successful_kill_start(base: &Game, item_kind: &str) -> Game {
    (0..1024)
        .find_map(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            let before = game.clone();
            dispatch_next(
                &mut game,
                GameCommand::Move {
                    direction: Direction::East,
                },
            );
            game.items
                .iter()
                .any(|item| item.kind_id == item_kind)
                .then_some(before)
        })
        .expect("a real melee death with the source reward probability")
}

#[test]
fn q1_global_allocation_death_equipment_and_saved_continuation() {
    let mut fresh = Game::new_with_build(521, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut fresh);
    let surface = fresh.clone();
    place_player_on_terrain(&mut fresh, "demo.terrain.stairs-down");
    dispatch_next(&mut fresh, GameCommand::TraverseStairs);
    assert_eq!(fresh.current_floor_id, "demo.floor.warrens-depth-1");
    for (actor, slug, depth) in REWARDS {
        let mut game = if depth == 30 {
            surface.clone()
        } else {
            fresh.clone()
        };
        let dungeon = if depth == 2 { "warrens" } else { "orc-cave" };
        let floor_id = format!("demo.floor.{dungeon}-depth-{depth}");
        // Explicit depth preparation; selection retains the complete formal pool.
        assert!(
            game.transition_floor(floor_id.clone(), None, None, false)
                .unwrap()
                .is_some()
        );
        clear_monsters(&mut game);
        let policy = game
            .content
            .encounter_table(&format!("demo.encounter-table.{dungeon}"))
            .unwrap()
            .global_allocation
            .clone()
            .unwrap();
        let kind = format!("demo.actor.{actor}");
        let selected = (0..20_000)
            .find_map(|_| {
                game.select_original_allocated_monster(
                    &floor_id,
                    &policy,
                    depth,
                    depth,
                    None,
                    &[],
                    None,
                    None,
                )
                .filter(|id| id == &kind)
            })
            .expect("Q1 actor must be reachable in the unchanged global allocation");
        prepare_combat(&mut game, &selected);
        let item_kind = format!("demo.item.{slug}");
        game = successful_kill_start(&game, &item_kind);
        let mut restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(game.state_hash(), restored.state_hash());
        // Loading publishes a full snapshot; its subsequent visual deltas need
        // not match a session whose last publication predates test preparation.
        assert_eq!(
            dispatch_next(
                &mut game,
                GameCommand::Move {
                    direction: Direction::East
                }
            )
            .events,
            dispatch_next(
                &mut restored,
                GameCommand::Move {
                    direction: Direction::East
                }
            )
            .events
        );
        assert_eq!(game.state_hash(), restored.state_hash());
        assert_eq!(game.actor_kind_available_instance_count(&kind), 0);
        let reward = game
            .items
            .iter()
            .find(|item| item.kind_id == item_kind)
            .unwrap()
            .clone();
        let ItemLocation::Ground(position) = reward.location else {
            panic!("ground reward")
        };
        game.player.position = position;
        game.pick_up_item_at_player(Some(&reward.id)).unwrap();
        assert!(
            !game
                .item_property_knowledge
                .get(&reward.id)
                .is_some_and(|knowledge| knowledge.identified)
        );
        game.reveal_current_visibility();
        game = Game::from_save(game.to_save()).unwrap();
        game.identify_item_instance(&reward.id, ItemIdentificationRequest::new(true));
        game.equip_inventory_item(&reward.id, None).unwrap();
        let properties = game.equipment_modifiers();
        let bonuses = game.player_equipment_bonuses();
        match slug {
            "dog-collar-of-fang" => {
                assert_eq!(properties.strength, 1);
                assert_eq!((bonuses.melee_skill, bonuses.melee_damage), (2, 3));
                assert!(game.player_status_immunities().contains("rfb.status.fear"));
            }
            "dog-collar-of-wolf" => {
                assert_eq!((properties.constitution, properties.defense), (1, 7));
                assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));
            }
            "dog-collar-of-grip" => assert_eq!((properties.dexterity, properties.speed), (1, 1)),
            _ => {
                assert_eq!(
                    (
                        properties.strength,
                        properties.dexterity,
                        properties.constitution,
                        properties.speed,
                        properties.defense
                    ),
                    (1, 1, 1, 1, 12)
                );
                assert_eq!(
                    (
                        bonuses.melee_skill,
                        bonuses.melee_damage,
                        bonuses.stealth_skill
                    ),
                    (2, 3, 1)
                );
                assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));
                for element in [
                    DamageType::Acid,
                    DamageType::Electricity,
                    DamageType::Fire,
                    DamageType::Cold,
                    DamageType::Poison,
                ] {
                    assert_eq!(game.player_resistance_percent(element), 50);
                }
            }
        }
        game.reveal_current_visibility();
        restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(
            dispatch_next(&mut restored, GameCommand::Wait).events,
            dispatch_next(&mut game, GameCommand::Wait).events
        );
        assert_eq!(game.state_hash(), restored.state_hash());
        assert_eq!(game.rng, restored.rng);
        assert!(restored.generated_artifact_ids.contains(&item_kind));
        let ItemLocation::Equipped { slot_id } = restored
            .items
            .iter()
            .find(|item| item.id == reward.id)
            .unwrap()
            .location
            .clone()
        else {
            panic!("equipped reward")
        };
        assert!(restored.unequip_slot(&slot_id).is_some());
        assert_eq!(restored.equipment_modifiers(), Default::default());
        assert_eq!(restored.player_equipment_bonuses(), Default::default());
    }
}

#[test]
fn q1_named_drop_boundaries_pet_unique_and_normal_pool_exclusion() {
    let mut base = Game::new_with_build(522, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut base);
    for (actor, slug, depth) in REWARDS {
        let mut prepared = base.clone();
        prepare_combat(&mut prepared, &format!("demo.actor.{actor}"));
        let target = prepared.entities[0].clone();
        let item_kind = format!("demo.item.{slug}");
        let chance = if depth == 2 { 10 } else { 5 };
        for bad_luck in [false, true] {
            let threshold = if bad_luck {
                chance - chance / 4
            } else {
                chance
            };
            for roll in [threshold - 1, threshold] {
                let mut game = prepared.clone();
                if bad_luck {
                    game.progress
                        .active_mutation_ids
                        .insert("rfb.mutation.bad-luck".into());
                }
                let seed = (0..10_000)
                    .find(|seed| RfbRng::seeded(*seed).bounded(100) == roll)
                    .unwrap();
                game.rng = RfbRng::seeded(seed);
                let (drops, _) = game.generate_death_loot(&target).unwrap();
                assert_eq!(
                    drops.iter().any(|item| item.kind_id == item_kind),
                    roll < threshold
                );
                if roll < threshold {
                    let mut already = prepared.clone();
                    already.generated_artifact_ids.insert(item_kind.clone());
                    already.rng = RfbRng::seeded(seed);
                    let draws = already.rng.draw_counter;
                    assert!(
                        already
                            .generate_death_loot(&target)
                            .unwrap()
                            .0
                            .iter()
                            .all(|item| item.kind_id != item_kind)
                    );
                    assert!(
                        already.rng.draw_counter > draws,
                        "already generated still rolls"
                    );
                }
            }
        }
        let mut pet = target;
        pet.controller_id = Some(prepared.player.id.clone());
        assert!(
            prepared
                .generate_death_loot(&pet)
                .unwrap()
                .0
                .iter()
                .all(|item| item.kind_id != item_kind)
        );
        if slug == "dog-collar-of-fang" {
            pet.controller_id = None;
            prepared.terrain.fill("demo.terrain.wall".into());
            let seed = (0..10_000)
                .find(|seed| RfbRng::seeded(*seed).bounded(100) == 0)
                .unwrap();
            prepared.rng = RfbRng::seeded(seed);
            assert!(
                prepared
                    .generate_death_loot(&pet)
                    .unwrap()
                    .0
                    .iter()
                    .all(|item| item.kind_id != item_kind)
            );
            assert!(
                !prepared.generated_artifact_ids.contains(&item_kind),
                "no legal floor must not reserve uniqueness"
            );
        }
    }
    // Remove all ordinary candidates via the existing uniqueness gate, leaving
    // the four new QUESTITEM definitions ungenerated. Neither base can roll them.
    base.generated_artifact_ids = base
        .content
        .item_definitions()
        .filter(|item| {
            item.artifact_generation.is_some()
                && !REWARDS
                    .iter()
                    .any(|(_, slug, _)| item.id == format!("demo.item.{slug}"))
        })
        .map(|item| item.id.clone())
        .collect();
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: base.current_floor_id.clone(),
        depth: 100,
        source: LootSource::MonsterDeath {
            actor_id: "test.pool".into(),
        },
    };
    for kind in ["demo.item.amulet", "demo.item.soft-leather-boots"] {
        assert_eq!(
            base.roll_fixed_artifact_kind_id(&context, Some(kind), false),
            None
        );
    }
}
