// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use crate::game::tasks::{
    TaskResolution, TaskRewardOutcome, TaskServiceCompletionOutcome, task_resolution_for_departure,
};

const SNAKES_TASK: &str = "demo.task.morivant-snakes";
const SNAKES_FLOOR: &str = "demo.floor.morivant-snakes";
const JONES_WHIP: &str = "demo.item.dr-jones-whip";
const MORIVANT_CASTLE: &str = "demo.town-facility.morivant-castle";

fn morivant_snakes_game() -> Game {
    let mut game = test_caster_game(51);
    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    dispatch_next(
        &mut game,
        GameCommand::EnterWorldMap {
            leave_pets: false,
            cancel_recall: false,
        },
    );
    game.wilderness_position = Some(Position { x: 47, y: 50 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    clear_monsters(&mut game);
    let entry = game
        .town_local_to_wilderness_view_position("demo.town.morivant", Position { x: 14, y: 46 })
        .unwrap();
    game.player.position = entry;
    assert!(!game.task_states.contains_key(SNAKES_TASK));
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    game.player.position = game
        .town_local_to_wilderness_view_position("demo.town.morivant", Position { x: 153, y: 18 })
        .unwrap();
    let service = game
        .snapshot()
        .task_services
        .into_iter()
        .find(|service| service.id == MORIVANT_CASTLE)
        .unwrap();
    assert_eq!(service.tasks.len(), 1);
    assert_eq!(service.tasks[0].task_id, SNAKES_TASK);
    assert_eq!(service.tasks[0].status, TaskStatusKindDto::Available);
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: MORIVANT_CASTLE.to_owned(),
            task_id: SNAKES_TASK.to_owned(),
        },
    );
    assert_eq!(
        game.task_states[SNAKES_TASK].status,
        TaskStatusKindDto::Taken
    );
    game.player.position = game
        .town_local_to_wilderness_view_position("demo.town.morivant", Position { x: 14, y: 46 })
        .unwrap();
    assert_eq!(
        game.terrain_at(game.player.position),
        "demo.terrain.morivant-snakes-entry"
    );
    game
}

#[test]
fn morivant_snakes_fetch_pickup_save_and_exit_keep_one_artifact() {
    let accepted = morivant_snakes_game();
    let mut game =
        Game::from_save_with_content(accepted.to_save(), accepted.content.clone()).unwrap();
    assert_eq!(game.state_hash(), accepted.state_hash());
    let entry = game.player.position;
    let entered = dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, SNAKES_FLOOR, "{:?}", entered.events);
    assert_eq!(game.entities.len(), 44);
    assert!(game.generated_artifact_ids.contains(JONES_WHIP));
    let whip_id = game
        .items
        .iter()
        .find(|item| item.kind_id == JONES_WHIP)
        .unwrap()
        .id
        .clone();
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == whip_id)
            .unwrap()
            .location,
        ItemLocation::Ground(Position { x: 19, y: 4 })
    );
    clear_monsters(&mut game);
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(
        game.task_states[SNAKES_TASK].current, 0,
        "clearing snakes is not FIND_ART"
    );

    game.player.position = Position { x: 18, y: 4 };
    give_inventory_item(&mut game, "test.snakes.plain-whip", "demo.item.whip");
    dispatch_next(
        &mut game,
        GameCommand::Drop {
            item_ids: vec!["test.snakes.plain-whip".to_owned()],
        },
    );
    dispatch_next(&mut game, GameCommand::PickUp);
    assert_eq!(game.task_states[SNAKES_TASK].current, 0);
    game.progress.level = 9;
    game.progress.max_level = 9;
    game.refresh_character_skills();
    assert!(game.gain_mutation("rfb.mutation.telekinesis", &mut Vec::new()));
    game.debug_set_ability_casts_succeed(true);
    let mana = game.resources.get_mut("demo.resource.mana").unwrap();
    mana.current = mana.maximum;
    game.reveal_current_visibility();
    dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: "rfb.ability.mutation.telekinesis".to_owned(),
            target: TargetSelection::Direction {
                direction: Direction::East,
            },
        },
    );
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == whip_id)
            .unwrap()
            .location,
        ItemLocation::Ground(game.player.position)
    );
    assert_eq!(
        game.task_states[SNAKES_TASK].current, 0,
        "fetch is not pickup"
    );
    let mut game = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert_eq!(game.task_states[SNAKES_TASK].current, 0);
    dispatch_next(&mut game, GameCommand::PickUp);
    assert_eq!(game.task_states[SNAKES_TASK].current, 1);
    assert_eq!(
        game.items
            .iter()
            .filter(|item| item.kind_id == JONES_WHIP)
            .count(),
        1
    );
    dispatch_next(
        &mut game,
        GameCommand::Drop {
            item_ids: vec![whip_id.clone()],
        },
    );
    dispatch_next(&mut game, GameCommand::PickUp);
    assert_eq!(game.task_states[SNAKES_TASK].current, 1);
    game.items
        .iter_mut()
        .find(|item| item.id == "test.snakes.plain-whip")
        .unwrap()
        .location = ItemLocation::Ground(Position { x: 19, y: 4 });
    game.rng = RfbRng::seeded(
        (0..1_000)
            .find(|seed| RfbRng::seeded(*seed).bounded(100) < 5)
            .unwrap(),
    );
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: whip_id.clone(),
            target: Some(TargetSelection::Position {
                position: Position { x: 19, y: 4 },
            }),
        },
    );
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == whip_id)
            .unwrap()
            .charges
            .unwrap()
            .current,
        0
    );
    game.player.position = Position { x: 1, y: 10 };
    let exited = dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.wilderness_position, Some(Position { x: 47, y: 50 }));
    assert_eq!(game.player.position, entry);
    assert_eq!(
        game.task_states[SNAKES_TASK].status,
        TaskStatusKindDto::RewardAvailable
    );
    let mut game = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    game.player.position = game
        .town_local_to_wilderness_view_position("demo.town.morivant", Position { x: 153, y: 18 })
        .unwrap();
    let gold = game.gold;
    let draws = game.rng_draw_counter();
    for _ in 0..2 {
        dispatch_next(
            &mut game,
            GameCommand::ClaimTaskReward {
                facility_id: MORIVANT_CASTLE.to_owned(),
                task_id: SNAKES_TASK.to_owned(),
            },
        );
    }
    assert_eq!(game.gold, gold);
    assert_eq!(game.rng_draw_counter(), draws);
    assert_eq!(
        game.task_states[SNAKES_TASK].status,
        TaskStatusKindDto::Completed
    );
    assert!(
        exited
            .events
            .iter()
            .all(|event| event.kind != "task.rewarded")
    );
    assert_eq!(
        game.items
            .iter()
            .filter(|item| item.kind_id == JONES_WHIP)
            .count(),
        1
    );
    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(
        restored
            .items
            .iter()
            .find(|item| item.id == whip_id)
            .unwrap()
            .charges
            .unwrap()
            .current,
        0
    );
    restored.player.position = entry;
    let before = restored.current_floor_id.clone();
    dispatch_next(&mut restored, GameCommand::TraverseStairs);
    assert_eq!(restored.current_floor_id, before);
    assert_eq!(
        restored.task_states[SNAKES_TASK].status,
        TaskStatusKindDto::Completed
    );
    assert_eq!(
        restored
            .items
            .iter()
            .filter(|item| item.kind_id == JONES_WHIP)
            .count(),
        1
    );
}

#[test]
fn morivant_snakes_failed_and_abandoned_floors_stay_closed_after_save() {
    for abandon in [false, true] {
        let mut game = morivant_snakes_game();
        let entry = game.player.position;
        dispatch_next(&mut game, GameCommand::TraverseStairs);
        game.fame = if abandon { 20 } else { 90 };
        clear_monsters(&mut game);
        let action = if abandon {
            GameCommand::AbandonTask
        } else {
            GameCommand::TraverseStairs
        };
        dispatch_next(&mut game, action);
        let expected = if abandon {
            TaskStatusKindDto::Abandoned
        } else {
            TaskStatusKindDto::Failed
        };
        assert_eq!(game.task_states[SNAKES_TASK].status, expected);
        assert_eq!(game.fame, if abandon { 10 } else { 60 });
        assert_eq!(game.wilderness_position, Some(Position { x: 47, y: 50 }));
        assert_eq!(game.player.position, entry);
        assert!(game.generated_artifact_ids.contains(JONES_WHIP));
        assert!(game.items.iter().all(|item| item.kind_id != JONES_WHIP));
        let mut restored =
            Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
        assert_eq!(restored.task_states[SNAKES_TASK].status, expected);
        restored.player.position = restored
            .town_local_to_wilderness_view_position(
                "demo.town.morivant",
                Position { x: 153, y: 18 },
            )
            .unwrap();
        let service = restored
            .snapshot()
            .task_services
            .into_iter()
            .find(|service| service.id == MORIVANT_CASTLE)
            .unwrap();
        assert_eq!(service.tasks.len(), 1);
        assert_eq!(service.tasks[0].status, expected);
        restored.player.position = entry;
        let before = restored.current_floor_id.clone();
        dispatch_next(&mut restored, GameCommand::TraverseStairs);
        assert_eq!(restored.current_floor_id, before);
        assert_eq!(restored.task_states[SNAKES_TASK].status, expected);
    }
}

#[test]
fn morivant_snakes_preexisting_artifact_rejects_entry_without_mutation() {
    let mut game = morivant_snakes_game();
    game.generated_artifact_ids.insert(JONES_WHIP.to_owned());
    let before = game.state_hash();
    let draws = game.rng_draw_counter();
    assert!(
        game.transition_floor(SNAKES_FLOOR.to_owned(), None, None, false)
            .unwrap()
            .is_none()
    );
    assert_eq!(game.state_hash(), before);
    assert_eq!(game.rng_draw_counter(), draws);
    let definition = game
        .content
        .world(&game.world_id)
        .unwrap()
        .procedural_floors
        .iter()
        .find(|floor| floor.id == SNAKES_FLOOR)
        .unwrap()
        .clone();
    assert!(game.generate_procedural_floor(&definition, None).is_err());
    assert_eq!(game.state_hash(), before);
}

fn item_reward(outcome: TaskServiceCompletionOutcome) -> TaskRewardOutcome {
    match outcome {
        TaskServiceCompletionOutcome::Rewarded(reward) => reward,
        TaskServiceCompletionOutcome::Concluded { .. } => {
            panic!("task should grant an item reward")
        }
    }
}

fn direct_warrens_death_drops(
    actor_kind_id: &str,
    seed: u64,
) -> (Vec<ItemInstance>, Vec<GoldPile>) {
    let mut game =
        Game::new_with_build(1, "demo.build.warrior").expect("Warrens journey should create");
    game.current_floor_id = "demo.floor.warrens-depth-1".to_owned();
    // This helper isolates normal death drops from the separate conquest reward.
    game.dungeon_states
        .get_mut("demo.dungeon.warrens")
        .unwrap()
        .guardian_defeated = true;
    game.rng = RfbRng::seeded(seed);
    let existing_item_ids = game
        .items
        .iter()
        .map(|item| item.id.clone())
        .collect::<BTreeSet<_>>();
    let existing_gold_ids = game
        .gold_piles
        .iter()
        .map(|pile| pile.id.clone())
        .collect::<BTreeSet<_>>();
    let actor_id = format!("test.{actor_kind_id}.{seed}");
    let actor = game.generated_actor(actor_id, actor_kind_id, game.player.position);
    game.entities.push(actor);
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();
    let mut removed = Vec::new();
    game.resolve_actor_death(
        game.entities.len() - 1,
        DomainEvent::Waited,
        &mut events,
        &mut changed,
        &mut removed,
    )
    .expect("direct monster death should resolve");
    (
        game.items
            .into_iter()
            .filter(|item| !existing_item_ids.contains(&item.id))
            .collect(),
        game.gold_piles
            .into_iter()
            .filter(|pile| !existing_gold_ids.contains(&pile.id))
            .collect(),
    )
}

#[test]
fn forced_base_ammunition_damage_dice_survive_generation_and_save() {
    let mut game = Game::new(67);
    let original = game.content.clone();
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&path).unwrap();
    let table = artifact
        .content
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.base-items")
        .unwrap();
    // This test fixes the base kind and exercises materialization.
    table.kind_selection = None;
    table
        .entries
        .retain(|entry| entry.item_kind_id == "demo.item.sheaf-arrow");
    game.content = Arc::new(rfb_content::ContentCatalog::from_artifact(artifact));
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: "demo.floor.orc-cave-depth-32".into(),
        depth: 80,
        source: LootSource::MonsterDeath {
            actor_id: "test.ammo-dice".into(),
        },
    };
    game.rng = RfbRng::seeded(41);
    let drops = game
        .generate_loot_instances(&context, ItemLocation::Inventory)
        .unwrap();
    game.content = original;
    let expected = drops[0].clone();
    assert_eq!(expected.kind_id, "demo.item.sheaf-arrow");
    assert_eq!(expected.damage_dice_override, Some(5));
    game.items.extend(drops);
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(
        restored.items.iter().find(|item| item.id == expected.id),
        Some(&expected)
    );
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.rng, game.rng);
}

#[test]
fn monster_object_level_and_theme_reach_real_jewelry_generation() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&root).unwrap();
    let mut table = artifact
        .content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .unwrap()
        .clone();
    table.id = "test.loot-table.forced-jewelry".into();
    // Keep an actual source theme while isolating jewelry's object level.
    table
        .entries
        .retain(|entry| entry.item_kind_id == "demo.item.ring");
    table.entries[0].min_depth = 0;
    table.quality_policy = Some(rfb_content::LootQualityPolicyDefinition::RfbDepth {
        good_cap_percent: 0,
        great_cap_percent: 0,
    });
    let mut pool = table.clone();
    pool.id = "test.loot-table.jewelry-base".into();
    table.entries.clear();
    table.kind_selection = Some(rfb_content::LootKindSelectionDefinition::RfbTheme {
        pool_id: pool.id.clone(),
        theme: rfb_content::RfbDropTheme::Mage,
    });
    artifact.content.loot_tables.push(pool);
    let actor = artifact
        .content
        .actors
        .iter_mut()
        .find(|actor| actor.death_drop.is_some())
        .unwrap();
    actor.level = 80;
    let actor_kind = actor.id.clone();
    actor.death_drop = Some(rfb_content::MonsterDropDefinition {
        great_only: false,
        kind: rfb_content::MonsterDropKindDefinition::Items,
        item_table_id: Some(table.id.clone()),
        theme_table_id: Some(table.id.clone()),
        theme_chance_percent: 100,
        base_rolls: 1,
        chance_rolls: vec![],
        count_dice: vec![],
        minimum_quality: rfb_content::ItemQuality::Ordinary,
    });
    artifact.content.loot_tables.push(table);
    let mut actual = Game::new_with_build(81, "demo.build.warrior").unwrap();
    actual.content = Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content).unwrap(),
    ));
    actual.current_floor_id = "demo.floor.warrens-depth-1".into();
    let actor = actual.generated_actor(
        "test.object-level".into(),
        &actor_kind,
        actual.player.position,
    );
    actual.rng = RfbRng::seeded(81);
    let mut expected = actual.clone();
    expected.rng.bounded(100); // The real monster theme gate precedes make_object.
    let context = LootContext {
        table_id: "test.loot-table.forced-jewelry".into(),
        floor_id: actual.current_floor_id.clone(),
        depth: 80, // _mon_drop_lvl(1, 80), independently from the floor depth.
        source: LootSource::MonsterDeath {
            actor_id: actor.id.clone(),
        },
    };
    let expected_items = expected
        .generate_loot_instances_internal(
            &context,
            ItemLocation::Ground(actor.position),
            false,
            Some(1),
            ItemGenerationMode::Ordinary,
        )
        .unwrap();
    let (items, gold) = actual.generate_death_loot(&actor).unwrap();
    assert_eq!(items, expected_items);
    assert_eq!(actual.rng, expected.rng);
    assert!(gold.is_empty());
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind_id, "demo.item.ring");
    assert!(
        !items[0].affix_ids.is_empty(),
        "theme promotes power zero jewelry to power one"
    );
}

#[test]
fn warrior_shoot_monster_death_keeps_theme_through_pickup_equipment_and_save() {
    let mut game = Game::new_with_build(421, RFB_WARRIOR_BUILD_ID).unwrap();
    clear_monsters(&mut game);
    let actor_kind = "demo.actor.orc-warlord";
    let drop = game
        .content
        .actor(actor_kind)
        .unwrap()
        .death_drop
        .as_ref()
        .unwrap();
    assert_eq!(
        drop.theme_table_id.as_deref(),
        Some("demo.loot-table.warrior-shoot")
    );
    let actor = game.generated_actor("test.themed-orc".into(), actor_kind, game.player.position);
    game.entities.push(actor);
    let initial_ids = game
        .items
        .iter()
        .map(|item| item.id.clone())
        .collect::<BTreeSet<_>>();
    game.rng = RfbRng::seeded(30);
    let mut probe = game.rng.clone();
    // Source DROP_90, then theme, then the gold/item choice.
    assert!(probe.bounded(100) < 90);
    assert!(probe.bounded(100) < 50);
    assert!(probe.bounded(100) >= 20);
    let mut events = Vec::new();
    game.resolve_actor_death(
        0,
        DomainEvent::Waited,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    let item = game
        .items
        .iter()
        .find(|item| !initial_ids.contains(&item.id) && !item.affix_ids.is_empty())
        .unwrap()
        .clone();
    // Warrior-shoot can produce armor, which the former Archer mapping rejects.
    assert!(matches!(
        game.content
            .item(&item.kind_id)
            .unwrap()
            .rfb_base_kind
            .unwrap()
            .tval,
        30..=38
    ));
    assert!(events.iter().any(|event| matches!(event, DomainEvent::LootDropped { source_kind_id, target_kind_id, .. } if source_kind_id == actor_kind && target_kind_id == &item.kind_id)));
    assert_eq!(item.location, ItemLocation::Ground(game.player.position));
    game.pick_up_item_at_player(Some(&item.id)).unwrap();
    assert!(game.equip_inventory_item(&item.id, None).is_some());
    game.refresh_player_resource_maxima();
    game.reveal_current_visibility();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.rng, game.rng);
    dispatch_next(&mut game, GameCommand::Wait);
    dispatch_next(&mut restored, GameCommand::Wait);
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn base_item_natural_egos_cover_all_equipment_types() {
    let base =
        Game::new_with_build(67, RFB_WARRIOR_BUILD_ID).expect("Orc Cave loot test should create");
    let context = LootContext {
        table_id: "demo.loot-table.base-items".to_owned(),
        floor_id: "demo.floor.orc-cave-depth-32".to_owned(),
        depth: 30,
        source: LootSource::MonsterDeath {
            actor_id: "test.orc-cave.loot-source".to_owned(),
        },
    };
    let mut seen = BTreeSet::new();
    // Fixed representatives exercise the real shared pool without a large seed sweep.
    for seed in [3, 11, 27, 38, 176, 241, 429, 513, 1207, 2489, 4957] {
        let mut game = base.clone();
        game.rng = RfbRng::seeded(seed);
        let drops = game
            .generate_loot_instances(&context, ItemLocation::Ground(game.player.position))
            .expect("Orc Cave loot should generate");
        assert_eq!(drops.len(), 1, "seed {seed}");
        for item in &drops {
            let definition = game
                .content
                .item(&item.kind_id)
                .expect("generated item definition should exist");
            let rfb_ego = item
                .affix_ids
                .first()
                .and_then(|affix_id| game.content.affix(affix_id))
                .and_then(|affix| affix.rfb_ego.as_ref());
            if let Some(rfb_ego) = rfb_ego {
                assert!(rfb_ego.rarity > 0);
                if matches!(rfb_ego.source_index, 200..=227) {
                    assert_ne!(item.quality, ItemQualityDto::Ordinary);
                } else {
                    assert_eq!(item.quality, ItemQualityDto::Exceptional);
                }
                assert_eq!(item.affix_ids.len(), 1);
                assert!(item.rolled_affixes.len() <= 1);
                if let Some(rolled) = item.rolled_affixes.first() {
                    seen.insert("rolled");
                    assert_eq!(rolled.affix_id, item.affix_ids[0]);
                }
                if (250..=256).contains(&rfb_ego.source_index) {
                    seen.insert("device");
                    assert!(definition.tags.iter().any(|tag| tag == "device"));
                    assert!(item.activation.is_some());
                    continue;
                }
                let base_kind = definition
                    .rfb_base_kind
                    .expect("RFB ego target should retain source base identity");
                match rfb_ego.source_index {
                    50..=152 => {
                        seen.insert("armor");
                        assert!(matches!(base_kind.tval, 30..=38));
                    }
                    1..=27 | 40..=42 => {
                        seen.insert("weapon-or-digger");
                        assert!(matches!(base_kind.tval, 20..=23));
                        assert_ne!(rfb_ego.source_index, 6, "Arcane requires a Wizardstaff");
                        assert_ne!(rfb_ego.source_index, 42, "Disruption requires a Mattock");
                    }
                    160..=167 => {
                        seen.insert("launcher");
                        assert_eq!(base_kind.tval, 19);
                        assert_ne!(base_kind.sval, 70, "Harp must not enter the BOW pool");
                        match rfb_ego.source_index {
                            164 => assert_eq!(base_kind.sval, 13),
                            165 => assert_eq!(base_kind.sval, 24),
                            166 => assert_eq!(base_kind.sval, 2),
                            _ => {}
                        }
                    }
                    180..=185 => {
                        seen.insert("ammunition");
                        assert!(matches!(base_kind.tval, 16..=18));
                    }
                    195 | 196 => {
                        seen.insert("harp");
                        assert_eq!((base_kind.tval, base_kind.sval), (19, 70));
                    }
                    200..=209 | 220..=227 => {
                        seen.insert("jewelry");
                        assert!(matches!(base_kind.tval, 40 | 45));
                    }
                    235..=243 => {
                        seen.insert("light");
                        assert_eq!(base_kind.tval, 39);
                    }
                    265..=268 => {
                        assert_eq!(base_kind.tval, 46);
                        if base_kind.sval == 0 {
                            seen.insert("quiver");
                            assert!(item.intrinsic_properties.ammunition_capacity.is_some());
                        } else {
                            seen.insert("bag");
                            assert!(item.intrinsic_properties.bag_capacity.is_some());
                        }
                    }
                    index => panic!("unexpected natural RFB ego source index {index}"),
                }
            } else if item.quality == ItemQualityDto::Fine
                && definition.equipment_slot.as_deref() != Some("weapon")
            {
                seen.insert("fine-without-ego");
                assert!(item.affix_ids.is_empty());
            }
        }
        let generated = drops[0].clone();
        game.items.extend(drops);
        game.reveal_current_visibility();
        let restored =
            Game::from_save(game.to_save()).expect("natural equipment must load without rerolling");
        assert_eq!(
            restored.items.iter().find(|item| item.id == generated.id),
            Some(&generated),
            "seed {seed}"
        );
        assert_eq!(restored.rng, game.rng, "seed {seed}");
        assert_eq!(restored.state_hash(), game.state_hash(), "seed {seed}");
        if seen.len() == 12 {
            break;
        }
    }
    assert_eq!(seen.len(), 12, "{seen:?}");
}

#[test]
fn shared_base_and_warrior_loot_use_depth_instead_of_dungeon_identity() {
    let catalog =
        Game::new_with_build(1, RFB_WARRIOR_BUILD_ID).expect("shared loot catalog should create");
    let depth_nine_kinds = catalog
        .content
        .loot_table("demo.loot-table.base-items")
        .expect("base item pool should exist")
        .entries
        .iter()
        .filter(|entry| entry.min_depth <= 9 && 9 <= entry.max_depth && entry.weight > 0)
        .map(|entry| entry.item_kind_id.as_str())
        .collect::<BTreeSet<_>>();
    assert!(!depth_nine_kinds.contains("demo.item.bastard-sword"));

    let roll = |table_id: &str, floor_id: &str, depth: u16, seed: u64| {
        let mut game = catalog.clone();
        game.rng = RfbRng::seeded(seed);
        game.generate_loot_instances(
            &LootContext {
                table_id: table_id.to_owned(),
                floor_id: floor_id.to_owned(),
                depth,
                source: LootSource::MonsterDeath {
                    actor_id: "test.shared-loot.actor".to_owned(),
                },
            },
            ItemLocation::Ground(game.player.position),
        )
        .expect("shared loot should resolve")
        .into_iter()
        .map(|item| (item.kind_id, item.quality, item.affix_ids))
        .collect::<Vec<_>>()
    };

    for seed in [0, 1, 42, 255] {
        let depth_nine = roll(
            "demo.loot-table.base-items",
            "demo.floor.warrens-depth-9",
            9,
            seed,
        );
        assert_eq!(depth_nine.len(), 1);
        assert!(depth_nine_kinds.contains(depth_nine[0].0.as_str()));

        let warrens = roll(
            "demo.loot-table.base-items",
            "demo.floor.warrens-depth-15",
            15,
            seed,
        );
        let orc_cave = roll(
            "demo.loot-table.base-items",
            "demo.floor.orc-cave-depth-15",
            15,
            seed,
        );
        assert_eq!(warrens.len(), 1);
        assert_eq!(warrens, orc_cave);

        assert_eq!(
            roll(
                "demo.loot-table.warrior",
                "demo.floor.warrens-depth-20",
                20,
                seed,
            ),
            roll(
                "demo.loot-table.warrior",
                "demo.floor.orc-cave-depth-20",
                20,
                seed,
            )
        );
    }
}

#[test]
fn warrens_keeper_drop_count_is_one_d_two_and_items_only() {
    let content = Game::new(0).content;
    let mut saw_one = false;
    let mut saw_two = false;
    for seed in 0..64 {
        let (drops, gold) = direct_warrens_death_drops("demo.actor.warrens-keeper", seed);
        let equipment = drops
            .iter()
            .filter(|item| {
                !matches!(
                    item.kind_id.as_str(),
                    "demo.item.corpse-remains" | "demo.item.skeleton-remains"
                )
            })
            .collect::<Vec<_>>();
        assert!(gold.is_empty());
        assert!(
            matches!(equipment.len(), 1 | 2),
            "seed={seed}: {equipment:?}"
        );
        assert!(
            equipment
                .iter()
                .filter(|item| {
                    let kind = content.item(&item.kind_id).unwrap();
                    kind.equipment_slot.is_some()
                        || kind.ammunition_profile.is_some()
                        || kind.tags.iter().any(|tag| tag == "device")
                })
                .all(|item| matches!(
                    item.quality,
                    ItemQualityDto::Fine | ItemQualityDto::Exceptional
                )),
            "seed={seed}: {equipment:?}"
        );
        saw_one |= equipment.len() == 1;
        saw_two |= equipment.len() == 2;
    }
    assert!(saw_one && saw_two);
}

#[test]
fn warrens_monster_drops_follow_original_probability_and_remains_profiles() {
    let is_remains = |item: &ItemInstance| {
        matches!(
            item.kind_id.as_str(),
            "demo.item.corpse-remains" | "demo.item.skeleton-remains"
        )
    };
    let mut saw_kobold_drop = false;
    let mut saw_kobold_gold = false;
    let mut saw_kobold_no_drop = false;
    let mut saw_no_remains = false;
    let mut saw_corpse = false;
    let mut saw_skeleton = false;

    for seed in 0..128 {
        let (drops, gold) = direct_warrens_death_drops("demo.actor.small-kobold", seed);
        let ordinary_drop_count = drops.iter().filter(|item| !is_remains(item)).count();
        assert!(ordinary_drop_count <= 1);
        assert!(gold.len() <= 1);
        assert!(ordinary_drop_count == 0 || gold.is_empty());
        saw_kobold_drop |= ordinary_drop_count == 1;
        saw_kobold_gold |= gold.len() == 1;
        saw_kobold_no_drop |= ordinary_drop_count == 0;
        saw_no_remains |= drops.iter().all(|item| !is_remains(item));
        saw_corpse |= drops
            .iter()
            .any(|item| item.kind_id == "demo.item.corpse-remains");
        saw_skeleton |= drops
            .iter()
            .any(|item| item.kind_id == "demo.item.skeleton-remains");
    }

    assert!(saw_kobold_drop && saw_kobold_gold && saw_kobold_no_drop);
    assert!(saw_no_remains && saw_corpse && saw_skeleton);
    assert_eq!(
        direct_warrens_death_drops("demo.actor.small-kobold", 42),
        direct_warrens_death_drops("demo.actor.small-kobold", 42)
    );

    for actor_kind_id in ["demo.actor.giant-white-mouse", "demo.actor.warg"] {
        for seed in 0..32 {
            let (drops, gold) = direct_warrens_death_drops(actor_kind_id, seed);
            assert!(gold.is_empty());
            assert!(drops.iter().all(is_remains));
            if actor_kind_id == "demo.actor.giant-white-mouse" {
                assert!(
                    drops
                        .iter()
                        .all(|item| item.kind_id != "demo.item.skeleton-remains")
                );
            }
        }
    }

    let mut surface =
        Game::new_with_build(1, "demo.build.warrior").expect("Warrens journey should create");
    surface.rng = RfbRng::seeded(42);
    let draws_before = surface.rng_draw_counter();
    let outside_depth = surface
        .generate_loot_instances(
            &LootContext {
                table_id: "demo.loot-table.warrior".to_owned(),
                floor_id: "demo.floor.surface".to_owned(),
                depth: 0,
                source: LootSource::MonsterDeath {
                    actor_id: "test.small-kobold.surface".to_owned(),
                },
            },
            ItemLocation::Ground(surface.player.position),
        )
        .expect("an out-of-depth loot table should resolve without candidates");
    assert!(outside_depth.is_empty());
    // Instant-artifact gate, then both ring/amulet hook rolls, even at depth zero.
    assert_eq!(surface.rng_draw_counter(), draws_before + 3);
}

#[test]
fn campaign_victory_plan_commits_ordered_events_once_without_rng() {
    let mut game = Game::new(42);
    let victory_dungeon_ids = game
        .campaign_definition()
        .expect("demo world should define a campaign")
        .victory_dungeon_ids
        .clone();
    for dungeon_id in victory_dungeon_ids {
        game.dungeon_states
            .get_mut(&dungeon_id)
            .expect("victory dungeon state should exist")
            .guardian_defeated = true;
    }
    let draws_before = game.rng_draw_counter();
    let victory_turn = game.turn.saturating_add(1);
    let mut events = Vec::new();

    game.apply_campaign_events(&mut events);

    assert_eq!(game.campaign_state.status, CampaignStatusDto::Victorious);
    assert_eq!(game.campaign_state.victory_turn, Some(victory_turn));
    assert!(matches!(
        events.as_slice(),
        [
            DomainEvent::CampaignVictorious { .. },
            DomainEvent::PlayerLevelCapUnlocked { .. }
        ]
    ));
    assert_eq!(game.rng_draw_counter(), draws_before);

    game.apply_campaign_events(&mut events);
    assert_eq!(events.len(), 2);
    assert_eq!(game.rng_draw_counter(), draws_before);
}

#[test]
fn external_task_service_projects_sparse_available_state_and_accepts_at_entrance() {
    let mut game = task_service_game(42);
    let task_id = "demo.task.test-warrens-depth";
    assert!(!game.task_states.contains_key(task_id));
    assert_eq!(
        game.snapshot()
            .tasks
            .iter()
            .find(|task| task.task_id == task_id)
            .expect("external task should be projected")
            .status,
        TaskStatusKindDto::Available
    );
    assert!(
        game.snapshot()
            .task_services
            .iter()
            .find(|service| service.id == "demo.town-facility.outpost-count")
            .expect("test task service should be projected")
            .tasks
            .is_empty()
    );

    game.player.position = Position { x: 98, y: 23 };
    let before_draws = game.rng_draw_counter();
    let snapshot = game.snapshot();
    let service = snapshot
        .task_services
        .iter()
        .find(|service| service.id == "demo.town-facility.outpost-count")
        .expect("test task service should be projected");
    assert!(service.player_at_entrance);
    assert_eq!(
        service
            .tasks
            .iter()
            .find(|task| task.task_id == task_id)
            .expect("external task should be projected at the service")
            .status,
        TaskStatusKindDto::Available
    );
    let update = dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-count".to_owned(),
            task_id: task_id.to_owned(),
        },
    );
    assert_eq!(game.task_states[task_id].status, TaskStatusKindDto::Taken);
    assert_eq!(game.rng_draw_counter(), before_draws);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "task.accepted")
    );
    assert_eq!(
        game.snapshot()
            .task_services
            .iter()
            .find(|service| service.id == "demo.town-facility.outpost-count")
            .expect("test task service should be projected")
            .tasks
            .iter()
            .find(|task| task.task_id == task_id)
            .expect("accepted task should remain projected")
            .status,
        TaskStatusKindDto::Taken
    );
}

#[test]
fn p107_task_substitutions_are_correlated_persisted_and_hide_losing_variants() {
    let projected_ids = |game: &mut Game| {
        game.player.position = Position { x: 98, y: 23 };
        game.snapshot()
            .task_services
            .iter()
            .find(|service| service.id == "demo.town-facility.outpost-count")
            .expect("P107 test task service should be projected")
            .tasks
            .iter()
            .map(|task| task.task_id.clone())
            .filter(|task_id| {
                task_id.contains("test-warrens") || task_id.contains("test-prerequisite")
            })
            .collect::<BTreeSet<_>>()
    };
    let primary_ids = BTreeSet::from([
        "demo.task.test-warrens-depth".to_owned(),
        "demo.task.test-prerequisite".to_owned(),
    ]);
    let alternate_ids = BTreeSet::from([
        "demo.task.test-warrens-depth-alternate".to_owned(),
        "demo.task.test-prerequisite-alternate".to_owned(),
    ]);

    let mut first = p107_task_service_game(10);
    let mut second = p107_task_service_game(11);
    let first_ids = projected_ids(&mut first);
    let second_ids = projected_ids(&mut second);
    assert!(first_ids == primary_ids || first_ids == alternate_ids);
    assert!(second_ids == primary_ids || second_ids == alternate_ids);
    assert_ne!(first_ids, second_ids);

    let losing_id = if first_ids == primary_ids {
        "demo.task.test-warrens-depth-alternate"
    } else {
        "demo.task.test-warrens-depth"
    };
    let unavailable = dispatch_next(
        &mut first,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-count".to_owned(),
            task_id: losing_id.to_owned(),
        },
    );
    assert!(
        unavailable
            .events
            .iter()
            .any(|event| event.kind == "task.accept-unavailable")
    );

    first.advance_wilderness_generation();
    assert_eq!(projected_ids(&mut first), first_ids);
    let content = first.content.clone();
    let mut restored = Game::from_save_with_content(first.to_save(), content)
        .expect("P107 task substitution should round-trip through existing task state");
    assert_eq!(projected_ids(&mut restored), first_ids);
}

#[test]
fn p107_failed_prerequisites_unlock_and_optional_status_descriptions_project() {
    let mut game = p107_task_service_game(10);
    game.player.position = Position { x: 98, y: 23 };
    let root_id = if game
        .task_states
        .contains_key("demo.task.test-warrens-depth")
    {
        "demo.task.test-warrens-depth"
    } else {
        "demo.task.test-warrens-depth-alternate"
    };
    let followup_id = if root_id.ends_with("-alternate") {
        "demo.task.test-prerequisite-alternate"
    } else {
        "demo.task.test-prerequisite"
    };
    game.task_states.get_mut(root_id).unwrap().status = TaskStatusKindDto::Failed;
    let failed = game.snapshot();
    let tasks = &failed
        .task_services
        .iter()
        .find(|service| service.id == "demo.town-facility.outpost-count")
        .unwrap()
        .tasks;
    assert_eq!(
        tasks
            .iter()
            .find(|task| task.task_id == root_id)
            .unwrap()
            .description_key
            .as_deref(),
        Some("test-task-failed-description")
    );
    assert_eq!(
        tasks
            .iter()
            .find(|task| task.task_id == followup_id)
            .unwrap()
            .status,
        TaskStatusKindDto::Available
    );

    let root = game.task_states.get_mut(root_id).unwrap();
    root.status = TaskStatusKindDto::Completed;
    root.current = root.required;
    assert_eq!(
        game.snapshot()
            .tasks
            .iter()
            .find(|task| task.task_id == root_id)
            .unwrap()
            .description_key
            .as_deref(),
        Some("test-task-completed-description")
    );
    assert!(matches!(
        task_resolution_for_departure(Some(false), false, true, false),
        Some(TaskResolution::Completed)
    ));
    assert_eq!(
        game.claim_task_reward("demo.town-facility.outpost-count", root_id),
        Err("reward-unavailable")
    );
}

#[test]
fn p107j_rewardless_service_task_waits_for_conclusion_without_creating_an_item() {
    let mut game = p107_task_service_game(10);
    game.player.position = Position { x: 98, y: 23 };
    let root_id = if game
        .task_states
        .contains_key("demo.task.test-warrens-depth")
    {
        "demo.task.test-warrens-depth"
    } else {
        "demo.task.test-warrens-depth-alternate"
    };
    let followup_id = if root_id.ends_with("-alternate") {
        "demo.task.test-prerequisite-alternate"
    } else {
        "demo.task.test-prerequisite"
    };
    let state = game.task_states.get_mut(root_id).unwrap();
    state.status = TaskStatusKindDto::RewardAvailable;
    state.current = state.required;
    state.active_floor_id = None;

    let projected = game
        .snapshot()
        .task_services
        .into_iter()
        .find(|service| service.id == "demo.town-facility.outpost-count")
        .and_then(|service| {
            service
                .tasks
                .into_iter()
                .find(|task| task.task_id == root_id)
        })
        .expect("rewardless task should remain projected for conclusion");
    assert_eq!(projected.status, TaskStatusKindDto::RewardAvailable);
    assert!(!projected.has_item_reward);

    let item_count = game.items.len();
    let before_draws = game.rng_draw_counter();
    let update = dispatch_next(
        &mut game,
        GameCommand::ClaimTaskReward {
            facility_id: "demo.town-facility.outpost-count".to_owned(),
            task_id: root_id.to_owned(),
        },
    );
    assert_eq!(
        game.task_states[root_id].status,
        TaskStatusKindDto::Completed
    );
    assert_eq!(game.items.len(), item_count);
    assert_eq!(game.rng_draw_counter(), before_draws);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "task.completed")
    );
    assert!(
        update
            .events
            .iter()
            .all(|event| event.kind != "task.rewarded")
    );
    assert_eq!(
        update
            .task_services
            .iter()
            .find(|service| service.id == "demo.town-facility.outpost-count")
            .and_then(|service| {
                service
                    .tasks
                    .iter()
                    .find(|task| task.task_id == followup_id)
            })
            .map(|task| task.status),
        Some(TaskStatusKindDto::Available)
    );

    let content = game.content.clone();
    let restored = Game::from_save_with_content(game.to_save(), content)
        .expect("rewardless task conclusion should use the existing task state save");
    assert_eq!(
        restored.task_states[root_id].status,
        TaskStatusKindDto::Completed
    );
}

#[test]
fn p110_thalos_projects_five_correlated_tasks_from_each_quest_line() {
    let projected = |seed: u64, facility_id: &str, position: Position| {
        let mut game = Game::new_with_build(seed, "demo.build.warrior")
            .expect("Middle-earth game should start");
        dispatch_next(
            &mut game,
            GameCommand::EnterWorldMap {
                leave_pets: false,
                cancel_recall: false,
            },
        );
        game.wilderness_position = Some(Position { x: 17, y: 29 });
        dispatch_next(&mut game, GameCommand::LeaveWorldMap);
        game.player.position = position;
        game.snapshot()
            .task_services
            .into_iter()
            .find(|service| service.id == facility_id)
            .unwrap_or_else(|| panic!("{facility_id} should be projected"))
            .tasks
            .into_iter()
            .map(|task| (task.task_id, task.status))
            .collect::<BTreeMap<_, _>>()
    };

    let first_palace = projected(
        10,
        "demo.town-facility.thalos-palace",
        Position { x: 89, y: 32 },
    );
    let second_palace = projected(
        11,
        "demo.town-facility.thalos-palace",
        Position { x: 89, y: 32 },
    );
    for palace in [&first_palace, &second_palace] {
        assert_eq!(palace.len(), 5);
        assert_eq!(
            palace["demo.task.thalos-shadow-fairies"],
            TaskStatusKindDto::Available
        );
        assert_eq!(
            palace.contains_key("demo.task.thalos-djinnis-cavern"),
            !palace.contains_key("demo.task.thalos-cyclops-lair")
        );
    }
    assert_ne!(first_palace, second_palace);

    let first_academy = projected(
        10,
        "demo.town-facility.thalos-royal-academy",
        Position { x: 109, y: 32 },
    );
    let second_academy = projected(
        11,
        "demo.town-facility.thalos-royal-academy",
        Position { x: 109, y: 32 },
    );
    for academy in [&first_academy, &second_academy] {
        assert_eq!(academy.len(), 5);
        assert_eq!(
            academy["demo.task.thalos-mushrooms"],
            TaskStatusKindDto::Available
        );
        let basilisk_first = academy.contains_key("demo.task.thalos-basilisk-cave");
        assert_eq!(
            basilisk_first,
            academy.contains_key("demo.task.thalos-staff-recovery")
        );
        assert_eq!(
            !basilisk_first,
            academy.contains_key("demo.task.thalos-staff-recovery-first")
        );
        assert_eq!(
            !basilisk_first,
            academy.contains_key("demo.task.thalos-dark-academy")
        );
    }
    assert_ne!(first_academy, second_academy);
}

#[test]
fn external_task_prerequisite_stays_locked_without_materializing_state() {
    let mut game = task_service_game(42);
    let task_id = "demo.task.test-prerequisite";
    game.player.position = Position { x: 98, y: 23 };
    let before_draws = game.rng_draw_counter();
    assert_eq!(
        game.snapshot()
            .task_services
            .iter()
            .find(|service| service.id == "demo.town-facility.outpost-count")
            .expect("test task service should be projected")
            .tasks
            .iter()
            .find(|task| task.task_id == task_id)
            .expect("dependent task should be projected")
            .status,
        TaskStatusKindDto::Locked
    );

    let update = dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-count".to_owned(),
            task_id: task_id.to_owned(),
        },
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "task.accept-unavailable")
    );
    assert!(!game.task_states.contains_key(task_id));
    assert_eq!(game.rng_draw_counter(), before_draws);
}

#[test]
fn accepted_external_task_binds_while_inside_its_dungeon_depth() {
    let mut game = task_service_game(42);
    let task_id = "demo.task.test-warrens-depth";
    game.player.position = Position { x: 98, y: 23 };
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-count".to_owned(),
            task_id: task_id.to_owned(),
        },
    );
    place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, "demo.floor.warrens-depth-1");
    assert_eq!(game.task_states[task_id].status, TaskStatusKindDto::Active);
    assert_eq!(
        game.task_states[task_id].active_floor_id.as_deref(),
        Some("demo.floor.warrens-depth-1")
    );

    place_player_on_terrain(&mut game, "demo.terrain.stairs-up");
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(game.task_states[task_id].status, TaskStatusKindDto::Failed);
    assert!(game.task_states[task_id].active_floor_id.is_none());
}

#[test]
fn external_task_service_rejects_unavailable_commands_without_rng_or_state_changes() {
    let mut game = task_service_game(42);
    let task_id = "demo.task.test-warrens-depth";
    let before_draws = game.rng_draw_counter();
    let accept = dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-count".to_owned(),
            task_id: task_id.to_owned(),
        },
    );
    assert!(
        accept
            .events
            .iter()
            .any(|event| event.kind == "task.accept-unavailable")
    );
    assert!(!game.task_states.contains_key(task_id));
    assert_eq!(game.rng_draw_counter(), before_draws);

    game.player.position = Position { x: 98, y: 23 };
    let claim = dispatch_next(
        &mut game,
        GameCommand::ClaimTaskReward {
            facility_id: "demo.town-facility.outpost-count".to_owned(),
            task_id: task_id.to_owned(),
        },
    );
    assert!(
        claim
            .events
            .iter()
            .any(|event| event.kind == "task.reward-claim-unavailable")
    );
    assert!(!game.task_states.contains_key(task_id));
    assert_eq!(game.rng.draw_counter, before_draws);
}

#[test]
fn task_rewards_use_one_weighted_default_choice_and_class_affix_overrides() {
    let template = task_service_game(42);
    let mut saw_food = false;
    let mut saw_water = false;
    for seed in 0..32 {
        let mut game = template.clone();
        game.rng = RfbRng::seeded(seed);
        game.player.position = Position { x: 98, y: 23 };
        game.task_states.insert(
            "demo.task.test-warrens-depth".to_owned(),
            TaskState {
                status: TaskStatusKindDto::RewardAvailable,
                stage_index: 0,
                current: 1,
                required: 1,
                active_floor_id: None,
                retakes_used: 0,
            },
        );
        let before_draws = game.rng_draw_counter();
        let reward = item_reward(
            game.claim_task_reward(
                "demo.town-facility.outpost-count",
                "demo.task.test-warrens-depth",
            )
            .expect("weighted task reward should resolve"),
        );
        assert_eq!(game.rng_draw_counter(), before_draws + 1);
        saw_food |= reward.item_kind_id == "demo.item.ration-of-food";
        saw_water |= reward.item_kind_id == "demo.item.water-potion";
    }
    assert!(saw_food && saw_water);

    let mut game = template;
    game.player.position = Position { x: 98, y: 23 };
    game.task_states.insert(
        "demo.task.test-prerequisite".to_owned(),
        TaskState {
            status: TaskStatusKindDto::RewardAvailable,
            stage_index: 0,
            current: 1,
            required: 1,
            active_floor_id: None,
            retakes_used: 0,
        },
    );
    let reward = item_reward(
        game.claim_task_reward(
            "demo.town-facility.outpost-count",
            "demo.task.test-prerequisite",
        )
        .expect("Warrior reward override should resolve"),
    );
    assert_eq!(reward.item_kind_id, "demo.item.broad-sword");
    let item = game
        .items
        .iter()
        .find(|item| item.id == "demo.task.test-prerequisite.reward.1")
        .expect("fixed reward instance should enter inventory");
    assert_eq!(item.quality, ItemQualityDto::Fine);
    assert_eq!(item.affix_ids, ["rfb-legacy.affix.slaying"]);
    assert_eq!(item.rolled_affixes.len(), 1);
}

#[test]
fn accepting_thieves_hideout_at_the_count_opens_its_count_district_entry() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
    let entry = Position { x: 125, y: 28 };
    assert_eq!(
        game.terrain_at(entry),
        "demo.terrain.thieves-hideout-entry-available"
    );
    game.player.position = Position { x: 98, y: 23 };
    let before_draws = game.rng_draw_counter();
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-count".to_owned(),
            task_id: "demo.task.thieves-hideout".to_owned(),
        },
    );

    assert_eq!(
        game.task_states["demo.task.thieves-hideout"].status,
        TaskStatusKindDto::Taken
    );
    assert_eq!(game.terrain_at(entry), "demo.terrain.thieves-hideout-entry");
    assert_eq!(game.rng_draw_counter(), before_draws);
}

#[test]
fn trouble_at_home_runs_from_white_horse_targets_only_mercenaries_and_rewards_warrior() {
    let mut game =
        Game::new_with_build(142, "demo.build.warrior").expect("Warrens journey should create");
    let entry = Position { x: 121, y: 36 };
    assert_eq!(
        game.terrain_at(entry),
        "demo.terrain.trouble-at-home-entry-available"
    );
    game.player.position = Position { x: 124, y: 35 };
    let service = game
        .snapshot()
        .task_services
        .into_iter()
        .find(|service| service.id == "demo.town-facility.outpost-white-horse")
        .expect("White Horse should expose its task service at the inn entrance");
    assert!(service.player_at_entrance);
    assert_eq!(
        service.tasks.first().map(|task| task.task_id.as_str()),
        Some("demo.task.trouble-at-home")
    );
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-white-horse".to_owned(),
            task_id: "demo.task.trouble-at-home".to_owned(),
        },
    );
    assert_eq!(
        game.task_states["demo.task.trouble-at-home"].status,
        TaskStatusKindDto::Taken
    );
    assert_eq!(game.terrain_at(entry), "demo.terrain.trouble-at-home-entry");

    game.player.position = entry;
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, "demo.floor.trouble-at-home");
    assert_eq!(game.entities.len(), 13);
    assert_eq!(
        game.entities
            .iter()
            .filter(|actor| actor.kind_id == "demo.actor.mean-looking-mercenary")
            .count(),
        5
    );
    assert_eq!(
        game.entities
            .iter()
            .filter(|actor| actor.kind_id == "demo.actor.singing-happy-drunk")
            .count(),
        7
    );
    assert_eq!(
        game.items
            .iter()
            .filter(|item| item.id.starts_with("demo.item.trouble-at-home.waybread."))
            .count(),
        4
    );
    let scrambled_positions = game
        .items
        .iter()
        .filter(|item| {
            matches!(
                item.id.as_str(),
                "demo.item.trouble-at-home.boldness.1" | "demo.item.trouble-at-home.booze.1"
            )
        })
        .filter_map(|item| match item.location {
            ItemLocation::Ground(position) => Some(position),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        scrambled_positions,
        BTreeSet::from([Position { x: 25, y: 1 }, Position { x: 25, y: 2 }])
    );

    let targets = game
        .entities
        .iter()
        .filter(|actor| actor.kind_id == "demo.actor.mean-looking-mercenary")
        .map(|actor| ActorDeathRecord {
            actor_id: actor.id.clone(),
            actor_kind_id: actor.kind_id.clone(),
            position: actor.position,
            credit_player: true,
        })
        .collect::<Vec<_>>();
    game.entities
        .retain(|actor| actor.kind_id != "demo.actor.mean-looking-mercenary");
    game.command_actor_deaths.extend(targets);
    let mut events = Vec::new();
    game.apply_deferred_task_events(None, &mut events)
        .expect("five mercenary deaths should complete Trouble at Home");
    assert_eq!(
        game.task_states["demo.task.trouble-at-home"].status,
        TaskStatusKindDto::RewardAvailable
    );
    assert_eq!(game.entities.len(), 8);

    game.player.position = Position { x: 25, y: 15 };
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(
        game.terrain_at(entry),
        "demo.terrain.trouble-at-home-entry-completed"
    );
    game.player.position = Position { x: 124, y: 35 };
    let before_draws = game.rng_draw_counter();
    dispatch_next(
        &mut game,
        GameCommand::ClaimTaskReward {
            facility_id: "demo.town-facility.outpost-white-horse".to_owned(),
            task_id: "demo.task.trouble-at-home".to_owned(),
        },
    );
    assert_eq!(
        game.task_states["demo.task.trouble-at-home"].status,
        TaskStatusKindDto::Completed
    );
    assert_eq!(game.rng_draw_counter(), before_draws);
    assert!(game.items.iter().any(|item| {
        item.id == "demo.task.trouble-at-home.reward.1"
            && item.kind_id == "demo.item.set-of-studded-leather-gloves"
            && item.location == ItemLocation::Inventory
    }));
}

#[test]
fn crows_nest_unlocks_after_trouble_at_home_clears_all_birds_and_rewards_a_staff() {
    let mut game =
        Game::new_with_build(143, "demo.build.warrior").expect("Warrens journey should create");
    let task_id = "demo.task.crows-nest";
    let entry = Position { x: 181, y: 59 };
    assert_eq!(
        game.terrain_at(entry),
        "demo.terrain.crows-nest-entry-available"
    );
    game.player.position = Position { x: 124, y: 35 };
    assert_eq!(
        game.accept_task("demo.town-facility.outpost-white-horse", task_id),
        Err("task-locked")
    );

    game.task_states.insert(
        "demo.task.trouble-at-home".to_owned(),
        TaskState {
            status: TaskStatusKindDto::Completed,
            stage_index: 0,
            current: 5,
            required: 5,
            active_floor_id: None,
            retakes_used: 0,
        },
    );
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-white-horse".to_owned(),
            task_id: task_id.to_owned(),
        },
    );
    assert_eq!(game.task_states[task_id].status, TaskStatusKindDto::Taken);
    assert_eq!(game.terrain_at(entry), "demo.terrain.crows-nest-entry");

    game.player.position = entry;
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, "demo.floor.crows-nest");
    assert_eq!(game.entities.len(), 9);
    game.entities.clear();
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(
        game.task_states[task_id].status,
        TaskStatusKindDto::RewardAvailable
    );

    game.player.position = Position { x: 2, y: 14 };
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(
        game.terrain_at(entry),
        "demo.terrain.crows-nest-entry-completed"
    );
    game.player.position = Position { x: 124, y: 35 };
    dispatch_next(
        &mut game,
        GameCommand::ClaimTaskReward {
            facility_id: "demo.town-facility.outpost-white-horse".to_owned(),
            task_id: task_id.to_owned(),
        },
    );
    assert_eq!(
        game.task_states[task_id].status,
        TaskStatusKindDto::Completed
    );
    let reward = game
        .items
        .iter()
        .find(|item| item.id == "demo.task.crows-nest.reward.1")
        .expect("Crow's Nest should grant its fixed reward");
    assert_eq!(reward.kind_id, "demo.item.enlightenment-staff");
    assert_eq!(reward.location, ItemLocation::Inventory);
    assert_eq!(
        reward
            .activation
            .as_ref()
            .map(|activation| activation.profile_id.as_str()),
        Some("demo.device-activation.enlightenment")
    );
    assert_eq!(
        reward
            .charges
            .map(|charges| (charges.current, charges.maximum)),
        Some((60, 60))
    );
}

#[test]
fn old_man_willow_unlocks_after_crows_nest_and_rewards_an_elemental_ring() {
    let mut game =
        Game::new_with_build(149, "demo.build.warrior").expect("Warrens journey should create");
    let task_id = "demo.task.old-man-willow";
    let entry = Position { x: 176, y: 19 };
    assert_eq!(
        game.terrain_at(entry),
        "demo.terrain.old-man-willow-entry-available"
    );
    game.player.position = Position { x: 124, y: 35 };
    assert_eq!(
        game.accept_task("demo.town-facility.outpost-white-horse", task_id),
        Err("task-locked")
    );

    game.task_states.insert(
        "demo.task.crows-nest".to_owned(),
        TaskState {
            status: TaskStatusKindDto::Completed,
            stage_index: 0,
            current: 1,
            required: 1,
            active_floor_id: None,
            retakes_used: 0,
        },
    );
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-white-horse".to_owned(),
            task_id: task_id.to_owned(),
        },
    );
    assert_eq!(game.task_states[task_id].status, TaskStatusKindDto::Taken);
    assert_eq!(game.terrain_at(entry), "demo.terrain.old-man-willow-entry");

    game.player.position = entry;
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, "demo.floor.old-man-willow");
    assert_eq!(game.entities.len(), 23);
    let target = game
        .entities
        .iter()
        .find(|actor| actor.kind_id == "demo.actor.old-man-willow")
        .expect("Old Man Willow should occupy the original fixed position");
    let death = ActorDeathRecord {
        actor_id: target.id.clone(),
        actor_kind_id: target.kind_id.clone(),
        position: target.position,
        credit_player: true,
    };
    game.entities
        .retain(|actor| actor.kind_id != "demo.actor.old-man-willow");
    game.command_actor_deaths.push(death);
    let mut events = Vec::new();
    game.apply_deferred_task_events(None, &mut events)
        .expect("Old Man Willow's death should complete the objective");
    assert_eq!(
        game.task_states[task_id].status,
        TaskStatusKindDto::RewardAvailable
    );
    assert_eq!(game.entities.len(), 22);

    game.player.position = Position { x: 1, y: 18 };
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(
        game.terrain_at(entry),
        "demo.terrain.old-man-willow-entry-completed"
    );
    game.player.position = Position { x: 124, y: 35 };
    let before_draws = game.rng_draw_counter();
    dispatch_next(
        &mut game,
        GameCommand::ClaimTaskReward {
            facility_id: "demo.town-facility.outpost-white-horse".to_owned(),
            task_id: task_id.to_owned(),
        },
    );
    assert_eq!(
        game.task_states[task_id].status,
        TaskStatusKindDto::Completed
    );
    assert_eq!(game.rng_draw_counter(), before_draws + 10);
    let reward = game
        .items
        .iter()
        .find(|item| item.id == "demo.task.old-man-willow.reward.1")
        .expect("Old Man Willow should grant its fixed ring reward");
    assert_eq!(reward.kind_id, "demo.item.ring");
    assert_eq!(reward.location, ItemLocation::Inventory);
    assert_eq!(reward.quality, ItemQualityDto::Fine);
    assert_eq!(reward.affix_ids, ["rfb-legacy.affix.elemental-jewelry"]);
    assert_eq!(reward.rolled_affixes.len(), 1);
    let resistances = &reward.rolled_affixes[0].properties.resistances;
    assert!((1..=4).contains(&resistances.len()));
    assert!(resistances.keys().all(|damage_type| matches!(
        damage_type,
        ActorDamageType::Acid
            | ActorDamageType::Cold
            | ActorDamageType::Electricity
            | ActorDamageType::Fire
    )));
}

#[test]
fn vapor_quest_unlocks_after_old_man_willow_clears_the_cellar_and_rewards_detection() {
    let mut game =
        Game::new_with_build(150, "demo.build.warrior").expect("Warrens journey should create");
    let task_id = "demo.task.vapor-quest";
    let entry = Position { x: 127, y: 41 };
    assert_eq!(
        game.terrain_at(entry),
        "demo.terrain.vapor-quest-entry-available"
    );
    game.player.position = Position { x: 124, y: 35 };
    assert_eq!(
        game.accept_task("demo.town-facility.outpost-white-horse", task_id),
        Err("task-locked")
    );

    game.task_states.insert(
        "demo.task.old-man-willow".to_owned(),
        TaskState {
            status: TaskStatusKindDto::Completed,
            stage_index: 0,
            current: 1,
            required: 1,
            active_floor_id: None,
            retakes_used: 0,
        },
    );
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-white-horse".to_owned(),
            task_id: task_id.to_owned(),
        },
    );
    assert_eq!(game.task_states[task_id].status, TaskStatusKindDto::Taken);
    assert_eq!(game.terrain_at(entry), "demo.terrain.vapor-quest-entry");

    game.player.position = entry;
    // Inspect the initial cellar before monster turns can destroy its ground items.
    game.transition_floor("demo.floor.vapor-quest".into(), None, None, false)
        .unwrap()
        .expect("accepted quest should admit the player");
    assert_eq!(game.current_floor_id, "demo.floor.vapor-quest");
    assert_eq!(game.entities.len(), 18);
    assert_eq!(
        game.items
            .iter()
            .filter(|item| item.id.starts_with("demo.item.vapor-quest."))
            .count(),
        12
    );
    game.entities.clear();
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(
        game.task_states[task_id].status,
        TaskStatusKindDto::RewardAvailable
    );

    game.player.position = Position { x: 12, y: 20 };
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(
        game.terrain_at(entry),
        "demo.terrain.vapor-quest-entry-completed"
    );
    game.player.position = Position { x: 124, y: 35 };
    dispatch_next(
        &mut game,
        GameCommand::ClaimTaskReward {
            facility_id: "demo.town-facility.outpost-white-horse".to_owned(),
            task_id: task_id.to_owned(),
        },
    );
    assert_eq!(
        game.task_states[task_id].status,
        TaskStatusKindDto::Completed
    );
    let reward = game
        .items
        .iter()
        .find(|item| item.id == "demo.task.vapor-quest.reward.1")
        .expect("Vapor Quest should grant its fixed reward");
    assert_eq!(reward.kind_id, "demo.item.detection-rod");
    assert_eq!(reward.location, ItemLocation::Inventory);
    assert_eq!(
        reward
            .activation
            .as_ref()
            .map(|activation| activation.profile_id.as_str()),
        Some("demo.device-activation.detection")
    );
    assert_eq!(
        reward
            .charges
            .map(|charges| (charges.current, charges.maximum)),
        Some((45, 45))
    );
}

#[test]
fn old_castle_unlocks_after_vapor_quest_and_rewards_the_warrior_artifact_pool() {
    let mut game =
        Game::new_with_build(271, "demo.build.warrior").expect("Warrens journey should create");
    let task_id = "demo.task.old-castle";
    let entry = Position { x: 31, y: 6 };
    assert_eq!(
        game.terrain_at(entry),
        "demo.terrain.old-castle-entry-available"
    );
    game.player.position = Position { x: 124, y: 35 };
    assert_eq!(
        game.accept_task("demo.town-facility.outpost-white-horse", task_id),
        Err("task-locked")
    );

    game.task_states.insert(
        "demo.task.vapor-quest".to_owned(),
        TaskState {
            status: TaskStatusKindDto::Completed,
            stage_index: 0,
            current: 1,
            required: 1,
            active_floor_id: None,
            retakes_used: 0,
        },
    );
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-white-horse".to_owned(),
            task_id: task_id.to_owned(),
        },
    );
    assert_eq!(game.task_states[task_id].status, TaskStatusKindDto::Taken);
    assert_eq!(game.terrain_at(entry), "demo.terrain.old-castle-entry");

    game.player.position = Position { x: 66, y: 33 };
    game.scroll_wilderness_for_player_entry(Position { x: 65, y: 33 }, &mut Vec::new())
        .unwrap();
    let entry = Position { x: 97, y: 6 };
    assert_eq!(game.wilderness_view_offset, Position { x: -1, y: 0 });
    assert_eq!(game.terrain_at(entry), "demo.terrain.old-castle-entry");
    game.player.position = entry;
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, "demo.floor.old-castle");
    game = Game::from_save(game.to_save()).unwrap();
    assert!(game.entities.len() >= 75);
    game.entities.clear();
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(
        game.task_states[task_id].status,
        TaskStatusKindDto::RewardAvailable
    );

    game.player.position = Position { x: 31, y: 1 };
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.player.position, entry);
    assert_eq!(game.wilderness_view_offset, Position { x: -1, y: 0 });
    assert_eq!(
        game.terrain_at(entry),
        "demo.terrain.old-castle-entry-completed"
    );
    game.player.position = Position { x: 131, y: 33 };
    game.scroll_wilderness_for_player_entry(Position { x: 132, y: 33 }, &mut Vec::new())
        .unwrap();
    assert_eq!(game.wilderness_view_offset, Position::default());
    assert_eq!(
        game.terrain_at(Position { x: 31, y: 6 }),
        "demo.terrain.old-castle-entry-completed"
    );
    game.player.position = Position { x: 124, y: 35 };
    dispatch_next(
        &mut game,
        GameCommand::ClaimTaskReward {
            facility_id: "demo.town-facility.outpost-white-horse".to_owned(),
            task_id: task_id.to_owned(),
        },
    );
    let reward = game
        .items
        .iter()
        .find(|item| item.id == "demo.task.old-castle.reward.1")
        .expect("Old Castle should grant an artifact");
    assert!(matches!(
        reward.kind_id.as_str(),
        "demo.item.slayer" | "demo.item.pain"
    ));
    assert_eq!(reward.location, ItemLocation::Inventory);
    assert!(game.generated_artifact_ids.contains(&reward.kind_id));
    assert!(
        game.to_save()
            .generated_artifact_ids
            .contains(&reward.kind_id)
    );
}

#[test]
fn old_castle_reward_is_forced_even_when_the_artifact_was_generated_before_claim() {
    let mut game =
        Game::new_with_build(271, "demo.build.warrior").expect("Warrens journey should create");
    game.player.position = Position { x: 124, y: 35 };
    game.task_states.insert(
        "demo.task.old-castle".to_owned(),
        TaskState {
            status: TaskStatusKindDto::RewardAvailable,
            stage_index: 0,
            current: 1,
            required: 1,
            active_floor_id: None,
            retakes_used: 0,
        },
    );
    game.generated_artifact_ids
        .extend(["demo.item.slayer".to_owned(), "demo.item.pain".to_owned()]);

    let reward = item_reward(
        game.claim_task_reward(
            "demo.town-facility.outpost-white-horse",
            "demo.task.old-castle",
        )
        .expect("explicit artifact reward should remain forced"),
    );
    assert!(matches!(
        reward.item_kind_id.as_str(),
        "demo.item.slayer" | "demo.item.pain"
    ));
    assert!(game.items.iter().any(|item| {
        item.id == "demo.task.old-castle.reward.1" && item.kind_id == reward.item_kind_id
    }));
}

#[test]
fn thieves_hideout_departure_closes_the_entry_and_keeps_reward_and_failure_distinct() {
    let mut base = Game::new_with_build(43, "demo.build.warrior").expect("task character");
    base.player.position = Position { x: 98, y: 23 };
    dispatch_next(
        &mut base,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-count".to_owned(),
            task_id: "demo.task.thieves-hideout".to_owned(),
        },
    );
    base.player.position = Position { x: 125, y: 28 };
    dispatch_next(&mut base, GameCommand::TraverseStairs);
    assert_eq!(base.current_floor_id, "demo.floor.thieves-hideout");
    for (cleared, expected_status, entry_terrain) in [
        (
            true,
            TaskStatusKindDto::RewardAvailable,
            "demo.terrain.thieves-hideout-entry-completed",
        ),
        (
            false,
            TaskStatusKindDto::Failed,
            "demo.terrain.thieves-hideout-entry-failed",
        ),
    ] {
        let mut game = base.clone();
        if cleared {
            game.entities.clear();
            dispatch_next(&mut game, GameCommand::Wait);
            assert!((1..=2).contains(&game.fame));
            assert_eq!(
                game.task_states["demo.task.thieves-hideout"].status,
                TaskStatusKindDto::RewardAvailable
            );
        }
        game.player.position = Position { x: 1, y: 4 };
        let fame = game.fame;
        let returned = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(
            game.fame, fame,
            "completion must not pay fame again on departure"
        );
        assert_eq!(
            game.current_floor_id,
            wilderness::WILDERNESS_FLOOR_ID,
            "cleared={cleared}"
        );
        assert_eq!(
            game.task_states["demo.task.thieves-hideout"].status, expected_status,
            "cleared={cleared}"
        );
        assert_eq!(
            game.terrain_at(Position { x: 125, y: 28 }),
            entry_terrain,
            "cleared={cleared}"
        );
        if cleared {
            assert!(
                returned
                    .events
                    .iter()
                    .any(|event| event.kind == "task.reward-available")
            );
        }
        assert!(
            !game
                .items
                .iter()
                .any(|item| item.id == "demo.task.thieves-hideout.reward.1"),
            "cleared={cleared}"
        );
    }
}

#[test]
fn count_task_rewards_complete_only_on_claim_with_the_expected_inventory_item() {
    let mut base = Game::new_with_build(45, "demo.build.warrior").expect("task reward character");
    base.player.position = Position { x: 98, y: 23 };
    for (task_id, required, kind_id) in [
        ("demo.task.thieves-hideout", 1, "demo.item.broad-sword"),
        ("demo.task.pest-control", 8, "demo.item.fur-cloak"),
    ] {
        let mut game = base.clone();
        if task_id == "demo.task.pest-control" {
            game.task_states.insert(
                "demo.task.thieves-hideout".to_owned(),
                TaskState {
                    status: TaskStatusKindDto::Completed,
                    stage_index: 0,
                    current: 1,
                    required: 1,
                    active_floor_id: None,
                    retakes_used: 0,
                },
            );
        }
        game.task_states.insert(
            task_id.to_owned(),
            TaskState {
                status: TaskStatusKindDto::RewardAvailable,
                stage_index: 0,
                current: required,
                required,
                active_floor_id: None,
                retakes_used: 0,
            },
        );
        let reward_id = format!("{task_id}.reward.1");
        assert!(
            !game.items.iter().any(|item| item.id == reward_id),
            "{task_id}"
        );
        let before_draws = game.rng_draw_counter();
        dispatch_next(
            &mut game,
            GameCommand::ClaimTaskReward {
                facility_id: "demo.town-facility.outpost-count".to_owned(),
                task_id: task_id.to_owned(),
            },
        );
        assert_eq!(
            game.task_states[task_id].status,
            TaskStatusKindDto::Completed,
            "{task_id}"
        );
        assert_eq!(game.rng_draw_counter(), before_draws, "{task_id}");
        assert!(
            game.items.iter().any(|item| item.id == reward_id
                && item.kind_id == kind_id
                && item.location == ItemLocation::Inventory),
            "{task_id}"
        );
    }
}

fn pest_control_state(status: TaskStatusKindDto, current: u32) -> TaskState {
    TaskState {
        status,
        stage_index: 0,
        current,
        required: 8,
        active_floor_id: (status == TaskStatusKindDto::Active)
            .then(|| "demo.floor.warrens-depth-5".to_owned()),
        retakes_used: 0,
    }
}

fn generate_pest_control_floor(game: &mut Game) -> FloorState {
    let definition = game
        .content
        .world(&game.world_id)
        .expect("Middle-earth world should remain available")
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.warrens-depth-5")
        .expect("Warrens depth 5 should remain available")
        .clone();
    game.generate_procedural_floor(&definition, None)
        .expect("Pest Control floor should generate")
}

#[test]
fn pest_control_unlocks_only_after_the_thieves_reward_is_claimed() {
    let mut game =
        Game::new_with_build(50, "demo.build.warrior").expect("Warrens journey should create");
    game.player.position = Position { x: 98, y: 23 };
    let task_id = "demo.task.pest-control";
    assert_eq!(
        game.snapshot()
            .task_services
            .into_iter()
            .find(|service| service.id == "demo.town-facility.outpost-count")
            .expect("Count should project task service")
            .tasks
            .into_iter()
            .find(|task| task.task_id == task_id)
            .expect("Pest Control should be projected")
            .status,
        TaskStatusKindDto::Locked
    );

    game.task_states.insert(
        "demo.task.thieves-hideout".to_owned(),
        TaskState {
            status: TaskStatusKindDto::Completed,
            stage_index: 0,
            current: 1,
            required: 1,
            active_floor_id: None,
            retakes_used: 0,
        },
    );
    assert_eq!(
        game.snapshot()
            .task_services
            .into_iter()
            .find(|service| service.id == "demo.town-facility.outpost-count")
            .expect("Count should project task service")
            .tasks
            .into_iter()
            .find(|task| task.task_id == task_id)
            .expect("Pest Control should be projected")
            .status,
        TaskStatusKindDto::Available
    );
}

#[test]
fn count_accepts_pest_control_without_advancing_rng() {
    let mut game =
        Game::new_with_build(51, "demo.build.warrior").expect("Warrens journey should create");
    game.player.position = Position { x: 98, y: 23 };
    game.task_states.insert(
        "demo.task.thieves-hideout".to_owned(),
        TaskState {
            status: TaskStatusKindDto::Completed,
            stage_index: 0,
            current: 1,
            required: 1,
            active_floor_id: None,
            retakes_used: 0,
        },
    );
    let before_draws = game.rng_draw_counter();
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-count".to_owned(),
            task_id: "demo.task.pest-control".to_owned(),
        },
    );

    assert_eq!(
        game.task_states["demo.task.pest-control"].status,
        TaskStatusKindDto::Taken
    );
    assert_eq!(game.rng_draw_counter(), before_draws);
}

#[test]
fn pest_control_floor_places_the_remaining_wargs_and_hides_downstairs() {
    for (current, expected_wargs) in [(0, 8), (3, 5)] {
        let mut game =
            Game::new_with_build(52, "demo.build.warrior").expect("Warrens journey should create");
        game.task_states.insert(
            "demo.task.pest-control".to_owned(),
            pest_control_state(TaskStatusKindDto::Taken, current),
        );
        let floor = generate_pest_control_floor(&mut game);
        let wargs = floor
            .entities
            .iter()
            .filter(|entity| entity.kind_id == "demo.actor.warg")
            .collect::<Vec<_>>();

        assert_eq!(wargs.len(), expected_wargs);
        assert!(
            wargs
                .iter()
                .all(|warg| { chebyshev_distance(floor.player_position, warg.position) >= 10 })
        );
        assert!(
            !floor
                .terrain
                .iter()
                .any(|terrain_id| terrain_id == "demo.terrain.stairs-down")
        );
    }
}

#[test]
fn final_pest_control_kill_reveals_a_magic_stair_and_draws_only_fame() {
    let mut game =
        Game::new_with_build(53, "demo.build.warrior").expect("Warrens journey should create");
    game.task_states.insert(
        "demo.task.pest-control".to_owned(),
        pest_control_state(TaskStatusKindDto::Active, 7),
    );
    let floor = generate_pest_control_floor(&mut game);
    game.current_floor_id = floor.id;
    game.width = floor.width;
    game.height = floor.height;
    game.terrain = floor.terrain;
    game.player.position = floor.player_position;
    game.entities = floor.entities;
    game.items.extend(floor.items);
    game.gold_piles = floor.gold_piles;
    game.floor_connections = floor.connections;
    game.floor_regions = floor.regions;
    let death_position = game
        .terrain
        .iter()
        .enumerate()
        .filter_map(|(index, terrain_id)| {
            let position = Position {
                x: i32::try_from(index % usize::from(game.width)).ok()?,
                y: i32::try_from(index / usize::from(game.width)).ok()?,
            };
            (terrain_id == "demo.terrain.floor"
                && !game.entities.iter().any(|actor| actor.position == position)
                && !game.items.iter().any(
                    |item| matches!(item.location, ItemLocation::Ground(ground) if ground == position),
                )
                && !game.gold_piles.iter().any(|pile| pile.position == position))
            .then_some(position)
        })
        .max_by_key(|position| chebyshev_distance(game.player.position, *position))
        .expect("Pest Control floor should retain a remote empty floor tile");
    let mut expected_rng = game.rng.clone();
    let expected_fame = expected_rng.bounded(2) as u16 + 1;
    game.command_actor_deaths.push(ActorDeathRecord {
        actor_id: "task-target.warg".to_owned(),
        actor_kind_id: "demo.actor.warg".to_owned(),
        position: death_position,
        credit_player: true,
    });
    let mut events = Vec::new();
    game.apply_deferred_task_events(None, &mut events)
        .expect("final Warg kill should complete Pest Control");

    assert_eq!(
        game.task_states["demo.task.pest-control"].status,
        TaskStatusKindDto::RewardAvailable
    );
    assert_eq!(
        game.terrain[usize::try_from(death_position.y).expect("non-negative y")
            * usize::from(game.width)
            + usize::try_from(death_position.x).expect("non-negative x")],
        "demo.terrain.stairs-down"
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::TaskExitRevealed { .. }))
    );
    assert_eq!(game.rng, expected_rng);
    assert_eq!(game.fame, expected_fame);
}

#[test]
fn leaving_pest_control_incomplete_fails_and_discards_the_blocked_floor() {
    let mut game =
        Game::new_with_build(54, "demo.build.warrior").expect("Warrens journey should create");
    game.task_states.insert(
        "demo.task.pest-control".to_owned(),
        pest_control_state(TaskStatusKindDto::Active, 3),
    );
    let floor = generate_pest_control_floor(&mut game);
    game.stored_floors
        .insert("test.warrens.5".to_owned(), floor);
    game.current_floor_id = "demo.floor.warrens-depth-4".to_owned();
    let mut events = vec![DomainEvent::FloorTransitioned {
        from_floor_id: "demo.floor.warrens-depth-5".to_owned(),
        to_floor_id: "demo.floor.warrens-depth-4".to_owned(),
    }];
    game.apply_deferred_task_events(None, &mut events)
        .expect("early departure should resolve");

    assert_eq!(
        game.task_states["demo.task.pest-control"].status,
        TaskStatusKindDto::Failed
    );
    assert_eq!(
        game.stored_floors
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["demo.floor.surface"]
    );
}

#[test]
fn count_follow_up_tasks_unlock_in_the_original_order() {
    let mut game =
        Game::new_with_build(56, "demo.build.warrior").expect("Warrens journey should create");
    game.player.position = Position { x: 98, y: 23 };
    let completed = |required| TaskState {
        status: TaskStatusKindDto::Completed,
        stage_index: 0,
        current: required,
        required,
        active_floor_id: None,
        retakes_used: 0,
    };
    let sequence = [
        ("demo.task.thieves-hideout", "demo.task.pest-control", 1),
        ("demo.task.pest-control", "demo.task.the-sewer", 8),
        ("demo.task.the-sewer", "demo.task.haunted-house", 1),
        ("demo.task.haunted-house", "demo.task.royal-crypt", 1),
    ];
    assert_eq!(
        game.snapshot()
            .task_services
            .into_iter()
            .find(|service| service.id == "demo.town-facility.outpost-count")
            .expect("Count should project its task service")
            .tasks
            .first()
            .map(|task| task.task_id.as_str()),
        Some("demo.task.thieves-hideout")
    );

    for (prerequisite, unlocked, required) in sequence {
        game.task_states
            .insert(prerequisite.to_owned(), completed(required));
        let service = game
            .snapshot()
            .task_services
            .into_iter()
            .find(|service| service.id == "demo.town-facility.outpost-count")
            .expect("Count should project its task service");
        assert_eq!(
            service
                .tasks
                .into_iter()
                .find(|task| task.task_id == unlocked)
                .expect("next Count task should be projected")
                .status,
            TaskStatusKindDto::Available
        );
    }
}

#[test]
fn royal_crypt_places_five_archliches_on_its_level_seventy_fixed_floor() {
    let mut game =
        Game::new_with_build(57, "demo.build.warrior").expect("Warrens journey should create");
    game.player.position = Position { x: 98, y: 23 };
    game.task_states.insert(
        "demo.task.haunted-house".to_owned(),
        TaskState {
            status: TaskStatusKindDto::Completed,
            stage_index: 0,
            current: 1,
            required: 1,
            active_floor_id: None,
            retakes_used: 0,
        },
    );
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-count".to_owned(),
            task_id: "demo.task.royal-crypt".to_owned(),
        },
    );
    game.player.position = Position { x: 120, y: 16 };
    dispatch_next(&mut game, GameCommand::TraverseStairs);

    assert_eq!(game.current_floor_id, "demo.floor.outpost-royal-crypt");
    assert_eq!(game.floor_depth(&game.current_floor_id), 70);
    assert_eq!(
        game.entities
            .iter()
            .filter(|actor| actor.kind_id == "demo.actor.archlich")
            .count(),
        5
    );
    assert_eq!(
        game.task_states["demo.task.royal-crypt"].status,
        TaskStatusKindDto::Active
    );
}

#[test]
fn warrens_dungeon_conquest_returns_retires_and_round_trips() {
    let mut game =
        Game::new_with_build(49, "demo.build.warrior").expect("Warrens journey should create");
    game.player
        .resistances
        .set(DamageType::Physical, ResistanceLevel::Immune);

    assert_eq!(game.world_id, DEFAULT_WORLD_ID);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(game.campaign_state.status, CampaignStatusDto::Active);

    for depth in 1..=9 {
        place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        let update = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(update.floor_id, format!("demo.floor.warrens-depth-{depth}"));
        assert_eq!(update.campaign.status, CampaignStatusDto::Active);
    }

    let guardian_index = game
        .entities
        .iter()
        .position(|entity| entity.id == "demo.guardian.warrens.1")
        .expect("Warrens depth 9 should spawn its guardian");
    assert_eq!(
        game.entities[guardian_index].kind_id,
        "demo.actor.warrens-keeper"
    );
    let guardian_position = game.entities[guardian_index].position;
    let item_ids_before_guardian = game
        .items
        .iter()
        .map(|item| item.id.clone())
        .collect::<BTreeSet<_>>();
    game.entities[guardian_index].hp = 1;
    game.entities[guardian_index].statuses = vec![StatusInstance {
        kind_id: STATUS_POISON.to_owned(),
        intensity: 3,
        remaining_ticks: 1,
        source_id: Some(game.player.id.clone()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    }];

    let victory = dispatch_next(&mut game, GameCommand::Wait);
    let guardian_event = victory
        .events
        .iter()
        .position(|event| event.kind == "dungeon.guardian-defeated")
        .expect("guardian death should conquer the Warrens");
    let victory_event = victory
        .events
        .iter()
        .position(|event| event.kind == "campaign.victorious")
        .expect("Warrens conquest should win the journey");
    assert!(guardian_event < victory_event);
    assert_eq!(victory.campaign.status, CampaignStatusDto::Victorious);
    assert_eq!(victory.campaign.conquered_dungeons, 1);
    assert_eq!(victory.campaign.score, 60_000);
    let guardian_drops = game
        .items
        .iter()
        .filter(|item| !item_ids_before_guardian.contains(&item.id))
        .collect::<Vec<_>>();
    assert!(
        guardian_drops
            .iter()
            .all(|item| { item.location == ItemLocation::Ground(guardian_position) })
    );
    assert_eq!(
        guardian_drops
            .iter()
            .filter(|item| item.kind_id == "demo.item.swiftstep-tonic")
            .count(),
        1
    );
    let high_quality_equipment = guardian_drops
        .iter()
        .filter(|item| {
            matches!(
                item.quality,
                ItemQualityDto::Fine | ItemQualityDto::Exceptional
            ) || game
                .content
                .item(&item.kind_id)
                .is_some_and(|definition| definition.artifact_generation.is_some())
        })
        .collect::<Vec<_>>();
    assert!(
        (1..=2).contains(&high_quality_equipment.len()),
        "unexpected guardian drops: {:?}",
        guardian_drops
            .iter()
            .map(|item| (&item.kind_id, &item.quality, &item.affix_ids))
            .collect::<Vec<_>>()
    );
    let allowed_guardian_drop_kinds = ["demo.loot-table.base-items", "demo.loot-table.warrior"]
        .into_iter()
        .flat_map(|table_id| {
            game.content
                .loot_table(table_id)
                .expect("guardian drop table should remain available")
                .entries
                .iter()
                .map(|entry| entry.item_kind_id.clone())
        })
        .collect::<BTreeSet<_>>();
    assert!(high_quality_equipment.iter().all(|item| {
        allowed_guardian_drop_kinds.contains(&item.kind_id)
            || game
                .content
                .item(&item.kind_id)
                .is_some_and(|definition| definition.artifact_generation.is_some())
    }));

    let victorious_hash = game.state_hash();
    let mut restored = Game::from_save(game.to_save()).expect("victory should round-trip");
    assert_eq!(restored.world_id, DEFAULT_WORLD_ID);
    assert_eq!(restored.state_hash(), victorious_hash);

    for expected_depth in (1..=8).rev() {
        place_player_on_terrain(&mut restored, "demo.terrain.stairs-up");
        let update = dispatch_next(&mut restored, GameCommand::TraverseStairs);
        assert_eq!(
            update.floor_id,
            format!("demo.floor.warrens-depth-{expected_depth}")
        );
        assert_eq!(update.campaign.status, CampaignStatusDto::Victorious);
    }
    place_player_on_terrain(&mut restored, "demo.terrain.stairs-up");
    let surface = dispatch_next(&mut restored, GameCommand::TraverseStairs);
    assert_eq!(surface.floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(surface.campaign.status, CampaignStatusDto::Victorious);

    let retirement = dispatch_next(&mut restored, GameCommand::Retire);
    assert_eq!(retirement.campaign.status, CampaignStatusDto::Retired);
    assert!(
        retirement
            .events
            .iter()
            .any(|event| event.kind == "campaign.retired")
    );
    let retired_hash = restored.state_hash();
    let retired = Game::from_save(restored.to_save()).expect("retirement should round-trip");
    assert_eq!(retired.state_hash(), retired_hash);
    assert_eq!(
        retired.snapshot().campaign.status,
        CampaignStatusDto::Retired
    );
}

#[test]
fn orc_cave_guardian_conquest_reward_and_surface_return_round_trip() {
    let mut game =
        Game::new_with_build(1185, "demo.build.warrior").expect("Middle-earth should create");
    game.player
        .resistances
        .set(DamageType::Physical, ResistanceLevel::Immune);

    dispatch_next(
        &mut game,
        GameCommand::EnterWorldMap {
            leave_pets: false,
            cancel_recall: false,
        },
    );
    game.wilderness_position = Some(Position { x: 30, y: 45 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    place_player_on_terrain(&mut game, "demo.terrain.orc-cave-entrance");

    for depth in 15..=32 {
        let update = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(
            update.floor_id,
            format!("demo.floor.orc-cave-depth-{depth}")
        );
        assert_eq!(update.campaign.status, CampaignStatusDto::Active);
        if depth < 32 {
            game.entities.clear();
            place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        }
    }

    let guardian_index = game
        .entities
        .iter()
        .position(|entity| entity.id == "demo.guardian.orc-cave.1")
        .expect("Orc Cave depth 32 should spawn Othrod");
    assert_eq!(
        game.entities[guardian_index].kind_id,
        "demo.actor.othrod-lord-of-the-orcs"
    );
    let guardian_position = game.entities[guardian_index].position;
    let item_ids_before_guardian = game
        .items
        .iter()
        .map(|item| item.id.clone())
        .collect::<BTreeSet<_>>();
    game.entities[guardian_index].hp = 1;
    game.entities[guardian_index].statuses = vec![StatusInstance {
        kind_id: STATUS_POISON.to_owned(),
        intensity: 3,
        remaining_ticks: 1,
        source_id: Some(game.player.id.clone()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    }];

    let conquered = dispatch_next(&mut game, GameCommand::Wait);
    assert!(
        (1..=4).contains(&game.fame),
        "guardian fame plus the unique kill chance"
    );
    assert!(
        conquered
            .events
            .iter()
            .any(|event| event.kind == "dungeon.guardian-defeated")
    );
    assert!(
        conquered
            .events
            .iter()
            .all(|event| event.kind != "campaign.victorious")
    );
    assert_eq!(conquered.campaign.status, CampaignStatusDto::Active);
    assert_eq!(conquered.campaign.conquered_dungeons, 1);
    assert_eq!(conquered.campaign.score, 10_000);
    assert!(game.dungeon_states["demo.dungeon.orc-cave"].guardian_defeated);

    let combat_ring = game
        .items
        .iter()
        .filter(|item| !item_ids_before_guardian.contains(&item.id))
        .find(|item| {
            item.kind_id == "demo.item.ring" && item.affix_ids == ["rfb-legacy.affix.combat-ring"]
        })
        .expect("Othrod should drop the fixed Combat ring");
    assert_eq!(
        combat_ring.location,
        ItemLocation::Ground(guardian_position)
    );
    assert_eq!(combat_ring.quality, ItemQualityDto::Fine);
    assert_eq!(combat_ring.affix_ids, ["rfb-legacy.affix.combat-ring"]);
    assert_eq!(combat_ring.rolled_affixes.len(), 1);
    assert_eq!(
        combat_ring.rolled_affixes[0].affix_id,
        "rfb-legacy.affix.combat-ring"
    );

    let conquered_hash = game.state_hash();
    let mut restored = Game::from_save(game.to_save()).expect("Orc Cave should round-trip");
    assert_eq!(restored.state_hash(), conquered_hash);
    assert!(restored.dungeon_states["demo.dungeon.orc-cave"].guardian_defeated);

    restored.entities.clear();
    for expected_depth in (15..=31).rev() {
        place_player_on_terrain(&mut restored, "demo.terrain.stairs-up");
        let update = dispatch_next(&mut restored, GameCommand::TraverseStairs);
        assert_eq!(
            update.floor_id,
            format!("demo.floor.orc-cave-depth-{expected_depth}")
        );
        restored.entities.clear();
    }
    place_player_on_terrain(&mut restored, "demo.terrain.stairs-up");
    let surface = dispatch_next(&mut restored, GameCommand::TraverseStairs);
    assert_eq!(surface.floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(
        restored.wilderness_position,
        Some(Position { x: 30, y: 45 })
    );
    assert_eq!(surface.campaign.status, CampaignStatusDto::Active);
}

#[test]
fn p86d_camelot_entrance_recall_conquest_and_reward_round_trip() {
    let mut game =
        Game::new_with_build(1111, "demo.build.warrior").expect("Middle-earth should create");
    game.player
        .resistances
        .set(DamageType::Physical, ResistanceLevel::Immune);

    dispatch_next(
        &mut game,
        GameCommand::EnterWorldMap {
            leave_pets: false,
            cancel_recall: false,
        },
    );
    game.wilderness_position = Some(Position { x: 7, y: 59 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(game.wilderness_position, Some(Position { x: 7, y: 59 }));
    assert_eq!(
        game.terrain
            .iter()
            .filter(|terrain| terrain.as_str() == "demo.terrain.camelot-entrance")
            .count(),
        1
    );
    place_player_on_terrain(&mut game, "demo.terrain.camelot-entrance");

    for depth in 20..=27 {
        let update = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(update.floor_id, format!("demo.floor.camelot-depth-{depth}"));
        assert_eq!(update.campaign.status, CampaignStatusDto::Active);
        if depth < 27 {
            support::clear_monsters(&mut game);
            place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        }
    }

    let mid_depth_hash = game.state_hash();
    game = Game::from_save(game.to_save()).expect("Camelot depth 27 should round-trip");
    assert_eq!(game.state_hash(), mid_depth_hash);
    assert_eq!(game.current_floor_id, "demo.floor.camelot-depth-27");
    support::clear_monsters(&mut game);
    game.recall = Some(RecallStateDto {
        dungeon_id: "demo.dungeon.camelot".to_owned(),
        floor_id: "demo.floor.camelot-depth-27".to_owned(),
        remaining_turns: Some(1),
    });
    let recalled = dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(recalled.floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(game.wilderness_position, Some(Position { x: 7, y: 59 }));
    assert!(
        recalled
            .events
            .iter()
            .any(|event| event.kind == "item.recall-triggered")
    );

    place_player_on_terrain(&mut game, "demo.terrain.camelot-entrance");
    for depth in 20..=35 {
        let update = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(update.floor_id, format!("demo.floor.camelot-depth-{depth}"));
        assert_eq!(update.campaign.status, CampaignStatusDto::Active);
        if depth < 35 {
            support::clear_monsters(&mut game);
            place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        }
    }

    let guardian_index = game
        .entities
        .iter()
        .position(|entity| entity.id == "demo.guardian.camelot.1")
        .expect("Camelot depth 35 should spawn Arthur");
    assert_eq!(
        game.entities[guardian_index].kind_id,
        "demo.actor.arthur-pendragon"
    );
    let guardian_position = game.entities[guardian_index].position;
    assert_eq!(
        game.items
            .iter()
            .filter(|item| item.kind_id == "demo.item.mirror-shield")
            .count(),
        0
    );
    game.entities[guardian_index].hp = 1;
    game.entities[guardian_index].statuses = vec![StatusInstance {
        kind_id: STATUS_POISON.to_owned(),
        intensity: 3,
        remaining_ticks: 1,
        source_id: Some(game.player.id.clone()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    }];

    let conquered = dispatch_next(&mut game, GameCommand::Wait);
    assert!(
        conquered
            .events
            .iter()
            .any(|event| event.kind == "dungeon.guardian-defeated")
    );
    assert!(
        conquered
            .events
            .iter()
            .all(|event| event.kind != "campaign.victorious")
    );
    assert_eq!(conquered.campaign.status, CampaignStatusDto::Active);
    assert_eq!(conquered.campaign.conquered_dungeons, 1);
    assert_eq!(conquered.campaign.score, 10_000);
    assert!(game.dungeon_states["demo.dungeon.camelot"].guardian_defeated);
    let mirror_shield = game
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.mirror-shield")
        .expect("Arthur should drop the fixed Mirror Shield");
    assert_eq!(
        mirror_shield.location,
        ItemLocation::Ground(guardian_position)
    );
    assert_eq!(mirror_shield.quality, ItemQualityDto::Ordinary);
    assert!(mirror_shield.affix_ids.is_empty());

    support::clear_monsters(&mut game);
    let after_conquest = dispatch_next(&mut game, GameCommand::Wait);
    assert!(
        after_conquest
            .events
            .iter()
            .all(|event| event.kind != "dungeon.guardian-defeated")
    );
    assert_eq!(after_conquest.campaign.conquered_dungeons, 1);
    assert_eq!(after_conquest.campaign.score, 10_000);
    assert_eq!(
        game.items
            .iter()
            .filter(|item| item.kind_id == "demo.item.mirror-shield")
            .count(),
        1
    );

    let conquered_hash = game.state_hash();
    let mut restored = Game::from_save(game.to_save()).expect("Camelot conquest should round-trip");
    assert_eq!(restored.state_hash(), conquered_hash);
    assert!(restored.dungeon_states["demo.dungeon.camelot"].guardian_defeated);
    assert_eq!(
        restored
            .defeated_limited_actor_counts
            .get("demo.actor.arthur-pendragon"),
        Some(&1)
    );
    let final_floor = restored
        .content
        .world(&restored.world_id)
        .expect("Middle-earth should remain available")
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.camelot-depth-35")
        .expect("Camelot final floor should remain available")
        .clone();
    let dungeon_instance_id = restored.current_dungeon_instance_id.clone();
    let regenerated = restored
        .generate_procedural_floor(&final_floor, dungeon_instance_id)
        .expect("conquered Camelot final floor should regenerate");
    assert!(
        regenerated
            .entities
            .iter()
            .all(|entity| entity.kind_id != "demo.actor.arthur-pendragon")
    );

    for expected_depth in (20..=34).rev() {
        place_player_on_terrain(&mut restored, "demo.terrain.stairs-up");
        let update = dispatch_next(&mut restored, GameCommand::TraverseStairs);
        assert_eq!(
            update.floor_id,
            format!("demo.floor.camelot-depth-{expected_depth}")
        );
        support::clear_monsters(&mut restored);
    }
    place_player_on_terrain(&mut restored, "demo.terrain.stairs-up");
    let surface = dispatch_next(&mut restored, GameCommand::TraverseStairs);
    assert_eq!(surface.floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(restored.wilderness_position, Some(Position { x: 7, y: 59 }));
    assert_eq!(surface.campaign.status, CampaignStatusDto::Active);
}

#[test]
fn p87d_tidal_cave_entrance_recall_conquest_and_reward_round_trip() {
    let mut game =
        Game::new_with_build(431, "demo.build.warrior").expect("Middle-earth should create");
    game.player
        .resistances
        .set(DamageType::Physical, ResistanceLevel::Immune);

    dispatch_next(
        &mut game,
        GameCommand::EnterWorldMap {
            leave_pets: false,
            cancel_recall: false,
        },
    );
    game.wilderness_position = Some(Position { x: 47, y: 53 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(game.wilderness_position, Some(Position { x: 47, y: 53 }));
    assert_eq!(
        game.terrain
            .iter()
            .filter(|terrain| terrain.as_str() == "demo.terrain.tidal-cave-entrance")
            .count(),
        1
    );
    place_player_on_terrain(&mut game, "demo.terrain.tidal-cave-entrance");

    for depth in 15..=20 {
        let update = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(
            update.floor_id,
            format!("demo.floor.tidal-cave-depth-{depth}")
        );
        assert_eq!(update.campaign.status, CampaignStatusDto::Active);
        if depth < 20 {
            game.entities.clear();
            place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        }
    }

    let mid_depth_hash = game.state_hash();
    game = Game::from_save(game.to_save()).expect("Tidal Cave depth 20 should round-trip");
    assert_eq!(game.state_hash(), mid_depth_hash);
    game.entities.clear();
    game.recall = Some(RecallStateDto {
        dungeon_id: "demo.dungeon.tidal-cave".to_owned(),
        floor_id: "demo.floor.tidal-cave-depth-20".to_owned(),
        remaining_turns: Some(1),
    });
    let recalled = dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(recalled.floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(game.wilderness_position, Some(Position { x: 47, y: 53 }));
    assert!(
        recalled
            .events
            .iter()
            .any(|event| event.kind == "item.recall-triggered")
    );

    assert!(game.recall_use_plan().is_some());
    let mut pending = Game::from_save(game.to_save()).unwrap();
    pending.start_recall(0);
    let mut pending = Game::from_save(pending.to_save()).unwrap();
    dispatch_next(&mut pending, GameCommand::Wait);
    assert_eq!(pending.current_floor_id, "demo.floor.tidal-cave-depth-20");

    place_player_on_terrain(&mut game, "demo.terrain.tidal-cave-entrance");
    for depth in 15..=27 {
        let update = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(
            update.floor_id,
            format!("demo.floor.tidal-cave-depth-{depth}")
        );
        assert_eq!(update.campaign.status, CampaignStatusDto::Active);
        if depth < 27 {
            game.entities.clear();
            place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        }
    }

    let guardian_index = game
        .entities
        .iter()
        .position(|entity| entity.id == "demo.guardian.tidal-cave.1")
        .expect("Tidal Cave depth 27 should spawn Grendel");
    assert_eq!(game.entities[guardian_index].kind_id, "demo.actor.grendel");
    let guardian_position = game.entities[guardian_index].position;
    let reward_count_before = game
        .items
        .iter()
        .filter(|item| item.kind_id == "demo.item.giant-strength-potion")
        .count();
    game.entities[guardian_index].hp = 1;
    game.entities[guardian_index].statuses = vec![StatusInstance {
        kind_id: STATUS_POISON.to_owned(),
        intensity: 3,
        remaining_ticks: 1,
        source_id: Some(game.player.id.clone()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    }];

    let conquered = dispatch_next(&mut game, GameCommand::Wait);
    assert!(
        conquered
            .events
            .iter()
            .any(|event| event.kind == "dungeon.guardian-defeated")
    );
    assert!(
        conquered
            .events
            .iter()
            .all(|event| event.kind != "campaign.victorious")
    );
    assert_eq!(conquered.campaign.status, CampaignStatusDto::Active);
    assert_eq!(conquered.campaign.conquered_dungeons, 1);
    assert_eq!(conquered.campaign.score, 10_000);
    assert!(game.dungeon_states["demo.dungeon.tidal-cave"].guardian_defeated);
    let reward = game
        .items
        .iter()
        .filter(|item| item.kind_id == "demo.item.giant-strength-potion")
        .find(|item| item.location == ItemLocation::Ground(guardian_position))
        .expect("Grendel should drop the fixed Potion of Giant Strength");
    assert_eq!(reward.quality, ItemQualityDto::Ordinary);
    assert!(reward.affix_ids.is_empty());
    assert_eq!(
        game.items
            .iter()
            .filter(|item| item.kind_id == "demo.item.giant-strength-potion")
            .count(),
        reward_count_before + 1
    );

    game.entities.clear();
    let after_conquest = dispatch_next(&mut game, GameCommand::Wait);
    assert!(
        after_conquest
            .events
            .iter()
            .all(|event| event.kind != "dungeon.guardian-defeated")
    );
    assert_eq!(after_conquest.campaign.conquered_dungeons, 1);
    assert_eq!(after_conquest.campaign.score, 10_000);
    assert_eq!(
        game.items
            .iter()
            .filter(|item| item.kind_id == "demo.item.giant-strength-potion")
            .count(),
        reward_count_before + 1
    );

    let conquered_hash = game.state_hash();
    let mut restored =
        Game::from_save(game.to_save()).expect("Tidal Cave conquest should round-trip");
    assert_eq!(restored.state_hash(), conquered_hash);
    assert!(restored.dungeon_states["demo.dungeon.tidal-cave"].guardian_defeated);
    assert_eq!(
        restored
            .defeated_limited_actor_counts
            .get("demo.actor.grendel"),
        Some(&1)
    );
    let final_floor = restored
        .content
        .world(&restored.world_id)
        .expect("Middle-earth should remain available")
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.tidal-cave-depth-27")
        .expect("Tidal Cave final floor should remain available")
        .clone();
    let dungeon_instance_id = restored.current_dungeon_instance_id.clone();
    let regenerated = restored
        .generate_procedural_floor(&final_floor, dungeon_instance_id)
        .expect("conquered Tidal Cave final floor should regenerate");
    assert!(
        regenerated
            .entities
            .iter()
            .all(|entity| entity.kind_id != "demo.actor.grendel")
    );

    for expected_depth in (15..=26).rev() {
        place_player_on_terrain(&mut restored, "demo.terrain.stairs-up");
        let update = dispatch_next(&mut restored, GameCommand::TraverseStairs);
        assert_eq!(
            update.floor_id,
            format!("demo.floor.tidal-cave-depth-{expected_depth}")
        );
        restored.entities.clear();
    }
    place_player_on_terrain(&mut restored, "demo.terrain.stairs-up");
    let surface = dispatch_next(&mut restored, GameCommand::TraverseStairs);
    assert_eq!(surface.floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(
        restored.wilderness_position,
        Some(Position { x: 47, y: 53 })
    );
    assert_eq!(surface.campaign.status, CampaignStatusDto::Active);
}

#[test]
fn p88d_icky_cave_entrance_recall_conquest_and_reward_round_trip() {
    let mut game =
        Game::new_with_build(909, "demo.build.warrior").expect("Middle-earth should create");
    game.player
        .resistances
        .set(DamageType::Physical, ResistanceLevel::Immune);

    dispatch_next(
        &mut game,
        GameCommand::EnterWorldMap {
            leave_pets: false,
            cancel_recall: false,
        },
    );
    game.wilderness_position = Some(Position { x: 17, y: 29 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(game.wilderness_position, Some(Position { x: 17, y: 29 }));
    assert_eq!(
        game.terrain
            .iter()
            .filter(|terrain| terrain.as_str() == "demo.terrain.icky-cave-entrance")
            .count(),
        1
    );
    place_player_on_terrain(&mut game, "demo.terrain.icky-cave-entrance");

    for depth in 10..=14 {
        let update = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(
            update.floor_id,
            format!("demo.floor.icky-cave-depth-{depth}")
        );
        assert_eq!(update.campaign.status, CampaignStatusDto::Active);
        if depth < 14 {
            game.entities.clear();
            place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        }
    }

    let mid_depth_hash = game.state_hash();
    game = Game::from_save(game.to_save()).expect("Icky Cave depth 14 should round-trip");
    assert_eq!(game.state_hash(), mid_depth_hash);
    game.entities.clear();
    game.recall = Some(RecallStateDto {
        dungeon_id: "demo.dungeon.icky-cave".to_owned(),
        floor_id: "demo.floor.icky-cave-depth-14".to_owned(),
        remaining_turns: Some(1),
    });
    let recalled = dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(recalled.floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(game.wilderness_position, Some(Position { x: 17, y: 29 }));
    assert!(
        recalled
            .events
            .iter()
            .any(|event| event.kind == "item.recall-triggered")
    );

    place_player_on_terrain(&mut game, "demo.terrain.icky-cave-entrance");
    for depth in 10..=20 {
        let update = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(
            update.floor_id,
            format!("demo.floor.icky-cave-depth-{depth}")
        );
        assert_eq!(update.campaign.status, CampaignStatusDto::Active);
        if depth < 20 {
            game.entities.clear();
            place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        }
    }

    let guardian_index = game
        .entities
        .iter()
        .position(|entity| entity.id == "demo.guardian.icky-cave.1")
        .expect("Icky Cave depth 20 should spawn The Icky Queen");
    let guardian = game.entities[guardian_index].clone();
    // Reward acceptance does not include other monsters picking the drop up.
    game.entities = vec![guardian];
    let guardian_index = 0;
    assert_eq!(
        game.entities[guardian_index].kind_id,
        "demo.actor.the-icky-queen"
    );
    let guardian_position = game.entities[guardian_index].position;
    let reward_count_before = game
        .items
        .iter()
        .filter(|item| {
            item.kind_id == "demo.item.quiver"
                && item.affix_ids == ["rfb-legacy.affix.quiver-protection"]
        })
        .count();
    game.entities[guardian_index].hp = 1;
    game.entities[guardian_index].statuses = vec![StatusInstance {
        kind_id: STATUS_POISON.to_owned(),
        intensity: 3,
        remaining_ticks: 1,
        source_id: Some(game.player.id.clone()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    }];

    let conquered = dispatch_next(&mut game, GameCommand::Wait);
    assert!(
        conquered
            .events
            .iter()
            .any(|event| event.kind == "dungeon.guardian-defeated")
    );
    assert!(
        conquered
            .events
            .iter()
            .all(|event| event.kind != "campaign.victorious")
    );
    assert_eq!(conquered.campaign.status, CampaignStatusDto::Active);
    assert_eq!(conquered.campaign.conquered_dungeons, 1);
    assert_eq!(conquered.campaign.score, 10_000);
    assert!(game.dungeon_states["demo.dungeon.icky-cave"].guardian_defeated);
    let reward = game
        .items
        .iter()
        .filter(|item| {
            item.kind_id == "demo.item.quiver"
                && item.affix_ids == ["rfb-legacy.affix.quiver-protection"]
        })
        .find(|item| item.location == ItemLocation::Ground(guardian_position))
        .expect("The Icky Queen should drop the fixed Protection quiver");
    assert_eq!(reward.quality, ItemQualityDto::Ordinary);
    assert_eq!(
        game.items
            .iter()
            .filter(|item| {
                item.kind_id == "demo.item.quiver"
                    && item.affix_ids == ["rfb-legacy.affix.quiver-protection"]
            })
            .count(),
        reward_count_before + 1
    );

    clear_monsters(&mut game);
    let after_conquest = dispatch_next(&mut game, GameCommand::Wait);
    assert!(
        after_conquest
            .events
            .iter()
            .all(|event| event.kind != "dungeon.guardian-defeated")
    );
    assert_eq!(after_conquest.campaign.conquered_dungeons, 1);
    assert_eq!(after_conquest.campaign.score, 10_000);
    assert_eq!(
        game.items
            .iter()
            .filter(|item| {
                item.kind_id == "demo.item.quiver"
                    && item.affix_ids == ["rfb-legacy.affix.quiver-protection"]
            })
            .count(),
        reward_count_before + 1
    );

    let conquered_hash = game.state_hash();
    let mut restored =
        Game::from_save(game.to_save()).expect("Icky Cave conquest should round-trip");
    assert_eq!(restored.state_hash(), conquered_hash);
    assert!(restored.dungeon_states["demo.dungeon.icky-cave"].guardian_defeated);
    assert_eq!(
        restored
            .defeated_limited_actor_counts
            .get("demo.actor.the-icky-queen"),
        Some(&1)
    );
    let final_floor = restored
        .content
        .world(&restored.world_id)
        .expect("Middle-earth should remain available")
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.icky-cave-depth-20")
        .expect("Icky Cave final floor should remain available")
        .clone();
    let dungeon_instance_id = restored.current_dungeon_instance_id.clone();
    let regenerated = restored
        .generate_procedural_floor(&final_floor, dungeon_instance_id)
        .expect("conquered Icky Cave final floor should regenerate");
    assert!(
        regenerated
            .entities
            .iter()
            .all(|entity| entity.kind_id != "demo.actor.the-icky-queen")
    );

    for expected_depth in (10..=19).rev() {
        place_player_on_terrain(&mut restored, "demo.terrain.stairs-up");
        let update = dispatch_next(&mut restored, GameCommand::TraverseStairs);
        assert_eq!(
            update.floor_id,
            format!("demo.floor.icky-cave-depth-{expected_depth}")
        );
        restored.entities.clear();
    }
    place_player_on_terrain(&mut restored, "demo.terrain.stairs-up");
    let surface = dispatch_next(&mut restored, GameCommand::TraverseStairs);
    assert_eq!(surface.floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(
        restored.wilderness_position,
        Some(Position { x: 17, y: 29 })
    );
    assert_eq!(surface.campaign.status, CampaignStatusDto::Active);
}
