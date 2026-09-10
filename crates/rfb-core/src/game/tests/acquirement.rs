// SPDX-License-Identifier: MPL-2.0

use super::{support::*, *};
use crate::game::loot::GeneratedItemDraft;

#[test]
#[ignore = "prepares scrolls, not generated rewards, for standalone B6 desktop acceptance"]
fn export_acquirement_desktop_save() {
    // Start from the desktop's actual new-game export so its museum binding,
    // identity and RNG remain intact. Only prepare consumables and clear actors.
    let input = std::env::var("B6_DESKTOP_INPUT").expect("path to the new-game desktop export");
    let (header, payload) = rfb_save::decode(&std::fs::read(input).unwrap()).unwrap();
    assert!(header.museum_binding.is_some());
    let mut game = Game::from_save(payload).unwrap();
    let rng = game.rng.clone();
    clear_monsters(&mut game);
    give_inventory_item(&mut game, "b6.acquirement", "demo.item.acquirement-scroll");
    give_inventory_item(&mut game, "b6.identify", "demo.item.revelation-scroll");
    game.items
        .iter_mut()
        .find(|item| item.id == "b6.identify")
        .unwrap()
        .quantity = 3;
    assert_eq!(game.rng, rng);
    let game = Game::from_save(game.to_save()).unwrap();
    let bytes = rfb_save::encode(&header, &game.to_save()).unwrap();
    let (_, payload) = rfb_save::decode(&bytes).unwrap();
    assert_eq!(
        Game::from_save(payload).unwrap().state_hash(),
        game.state_hash()
    );
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-results");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("b6-acquirement.rfbsave"), bytes).unwrap();
}

fn context(game: &Game) -> LootContext {
    LootContext {
        table_id: "test.loot-table.acquirement".into(),
        floor_id: game.current_floor_id.clone(),
        depth: game.floor_depth(&game.current_floor_id),
        source: LootSource::ItemUse {
            item_id: "test.acquirement".into(),
        },
    }
}

fn source_with_pool(source_pool: bool, available: bool) -> rfb_content::CompiledContentV1 {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    let mut source = rfb_content::compile_pack_dir(&path).unwrap().content;
    let mut table = source
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.base-items")
        .unwrap()
        .clone();
    table.id = "test.loot-table.acquirement".into();
    table
        .entries
        .retain(|entry| entry.item_kind_id == "demo.item.dagger");
    table.entries[0].min_depth = if available { 0 } else { 127 };
    if !source_pool {
        table.kind_selection = None;
    }
    source.loot_tables.push(table);
    for item in &mut source.items {
        if let Some(action) = &mut item.use_action
            && let ItemUseEffectDefinition::Acquirement { loot_table_id, .. } = &mut action.effect
        {
            *loot_table_id = "test.loot-table.acquirement".into();
        }
    }
    source
}

fn catalog_with_pool(source_pool: bool, available: bool) -> Arc<ContentCatalog> {
    Arc::new(ContentCatalog::from_artifact(
        rfb_content::encode_content(source_with_pool(source_pool, available)).unwrap(),
    ))
}

fn prepare(content: Arc<ContentCatalog>) -> Game {
    let mut game = Game::new_with_build(503, "demo.build.warrior").unwrap();
    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    game.items.clear();
    game.gold_piles.clear();
    game.content = content;
    game.rng = RfbRng::seeded(0);
    game
}

fn plain_draft(game: &mut Game, kind: &str) -> GeneratedItemDraft {
    game.fixed_item_draft(&context(game), kind.into())
}

#[test]
fn make_object_caps_follow_mode_and_explicit_hook_and_preserve_failed_rng() {
    for source_pool in [false, true] {
        let template = prepare(catalog_with_pool(source_pool, false));
        for (mode, attempts) in [
            (ItemGenerationMode::Ordinary, 1),
            (ItemGenerationMode::Good, if source_pool { 100 } else { 1 }),
            (
                ItemGenerationMode::GreatOnly,
                if source_pool { 100 } else { 1 },
            ),
            (ItemGenerationMode::Great, if source_pool { 100 } else { 1 }),
            (ItemGenerationMode::TailoredGreat, 1000),
        ] {
            let mut game = template.clone();
            let mut reference = template.clone();
            let context = context(&game);
            for _ in 0..attempts {
                assert!(
                    reference
                        .generate_loot_draft_attempt(&context, mode)
                        .is_none()
                );
            }
            assert!(game.generate_one_loot_draft(&context, mode).is_none());
            assert_eq!(
                game.rng, reference.rng,
                "{mode:?}, source pool {source_pool}"
            );
            assert!(game.rng.draw_counter >= attempts);
            assert_eq!(
                game.next_item_instance_serial,
                template.next_item_instance_serial
            );
            assert_eq!(game.item_knowledge, template.item_knowledge);
            assert!(game.items.is_empty());
        }
    }
}

#[test]
fn make_object_stops_on_first_success_after_zero_or_more_failed_categories() {
    let template = prepare(catalog_with_pool(true, true));
    for needs_retry in [false, true] {
        let (seed, draft, expected_rng) = (0..100)
            .find_map(|seed| {
                let mut reference = template.clone();
                reference.rng = RfbRng::seeded(seed);
                let context = context(&reference);
                for attempt in 0..100 {
                    if let Some(draft) =
                        reference.generate_loot_draft_attempt(&context, ItemGenerationMode::Great)
                    {
                        return ((attempt > 0) == needs_retry).then_some((
                            seed,
                            draft,
                            reference.rng,
                        ));
                    }
                }
                None
            })
            .unwrap();
        let mut game = template.clone();
        game.rng = RfbRng::seeded(seed);
        assert_eq!(
            game.generate_one_loot_draft(&context(&game), ItemGenerationMode::Great),
            Some(draft)
        );
        assert_eq!(game.rng, expected_rng);
        assert!(game.items.is_empty());
    }
}

#[test]
fn empty_acquirement_consumes_scroll_and_runs_both_caps_without_allocating_or_revealing() {
    let mut game = prepare(catalog_with_pool(false, false));
    assert_eq!(context(&game).depth, 0);
    give_inventory_item(
        &mut game,
        "test.acquirement",
        "demo.item.star-acquirement-scroll",
    );
    game.rng = RfbRng::seeded(71);
    let before_serial = game.next_item_instance_serial;
    let before_properties = game.item_property_knowledge.clone();
    let mut expected_rng = game.rng.clone();
    expected_rng.bounded(2); // Target count is rolled exactly once.
    for _ in 0..1_000_000 {
        expected_rng.bounded(10);
    }
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.acquirement".into(),
            target: None,
        },
    );
    assert!(game.items.is_empty());
    assert_eq!(game.next_item_instance_serial, before_serial);
    assert_eq!(game.rng, expected_rng);
    assert!(game.item_knowledge["demo.item.star-acquirement-scroll"].aware);
    assert_eq!(game.item_property_knowledge, before_properties);
    assert!(
        game.item_knowledge
            .values()
            .all(|knowledge| knowledge.found_count == 0)
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.use-acquirement" && event.args["count"] == "0")
    );
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.rng, game.rng);
}

#[test]
fn acquirement_id_exhaustion_is_an_error_before_consumption_or_rng() {
    let mut game = Game::new(503);
    clear_monsters(&mut game);
    give_inventory_item(
        &mut game,
        "test.acquirement",
        "demo.item.star-acquirement-scroll",
    );
    game.next_item_instance_serial = u64::MAX - 2;
    let mut invalid_target = game.clone();
    dispatch_next(
        &mut invalid_target,
        GameCommand::UseItem {
            item_id: "test.acquirement".into(),
            target: Some(TargetSelection::Item {
                item_id: "test.acquirement".into(),
            }),
        },
    );
    assert_eq!(invalid_target.items, game.items);
    assert_eq!(invalid_target.rng, game.rng);
    let before = game.to_save();
    let snapshot = game.snapshot();
    let result = game.dispatch(command(
        snapshot.last_command_seq + 1,
        snapshot.revision,
        GameCommand::UseItem {
            item_id: "test.acquirement".into(),
            target: None,
        },
    ));
    assert!(matches!(result, Err(CoreError::ItemIdExhausted)));
    assert_eq!(game.to_save(), before);
}

#[test]
fn drop_near_respects_terrain_pile_limit_merges_and_id_allocation() {
    let mut game = Game::new(503);
    game.items.clear();
    game.gold_piles.clear();
    let origin = Position { x: 5, y: 5 };
    let destination = Position { x: 6, y: 5 };
    game.terrain.fill("demo.terrain.wall".into());
    replace_terrain(&mut game, origin, "demo.terrain.stairs-down");
    replace_terrain(&mut game, destination, "demo.terrain.floor");
    let draft = plain_draft(&mut game, "demo.item.arrow");
    let serial = game.next_item_instance_serial;
    let (_, ids) = game
        .drop_generated_item_near(draft.clone(), origin)
        .unwrap()
        .unwrap();
    assert_eq!(
        game.items.last().unwrap().location,
        ItemLocation::Ground(destination)
    );
    assert_eq!(game.next_item_instance_serial, serial + 1);
    // The existing stack accepts the new item even at the 99-pile limit.
    for index in 0..98 {
        game.gold_piles.push(crate::state::GoldPile {
            id: format!("test.gold.{index}"),
            amount: 1,
            appearance: rfb_protocol::GoldAppearanceDto::Copper,
            discovered: false,
            position: destination,
        });
    }
    game.next_item_instance_serial = u64::MAX;
    assert_eq!(
        game.drop_generated_item_near(draft.clone(), origin)
            .unwrap(),
        Some((destination, ids))
    );
    assert_eq!(game.items.last().unwrap().quantity, 2);
    assert_eq!(game.next_item_instance_serial, u64::MAX);
    game.items.last_mut().unwrap().quantity =
        game.content.item("demo.item.arrow").unwrap().max_stack;
    let before = game.items.clone();
    assert!(matches!(
        game.drop_generated_item_near(draft, origin),
        Err(CoreError::ItemIdExhausted)
    ));
    let incompatible = plain_draft(&mut game, "demo.item.dagger");
    assert_eq!(
        game.drop_generated_item_near(incompatible, origin).unwrap(),
        None
    );
    assert_eq!(game.items, before);
}

#[test]
fn artifact_drop_searches_beyond_local_space_and_preserves_unknown_fixed_on_total_failure() {
    let mut game = Game::new(503);
    game.items.clear();
    game.gold_piles.clear();
    let origin = Position { x: 5, y: 5 };
    let destination = Position {
        x: i32::from(game.width) - 2,
        y: i32::from(game.height) - 2,
    };
    game.terrain.fill("demo.terrain.wall".into());
    replace_terrain(&mut game, destination, "demo.terrain.floor");
    let draft = plain_draft(&mut game, "demo.item.crisdurian");
    assert!(game.generated_artifact_ids.contains(&draft.kind_id));
    let placed = game
        .drop_generated_item_near(draft, origin)
        .unwrap()
        .unwrap();
    assert_eq!(placed.0, destination);
    assert_eq!(
        game.items.last().unwrap().location,
        ItemLocation::Ground(destination)
    );
    game.items.clear();
    replace_terrain(&mut game, destination, "demo.terrain.wall");
    let draft = plain_draft(&mut game, "demo.item.crisdurian");
    let serial = game.next_item_instance_serial;
    let draws = game.rng.draw_counter;
    assert!(
        game.drop_generated_item_near(draft, origin)
            .unwrap()
            .is_none()
    );
    assert!(!game.generated_artifact_ids.contains("demo.item.crisdurian"));
    assert_eq!(game.next_item_instance_serial, serial);
    assert_eq!(game.rng.draw_counter, draws + 2000);
}

#[test]
fn drop_near_projects_through_closed_curtains_without_requiring_walkability() {
    let mut game = Game::new(503);
    game.items.clear();
    game.gold_piles.clear();
    let origin = Position { x: 5, y: 5 };
    let curtain = Position { x: 6, y: 5 };
    let destination = Position { x: 7, y: 5 };
    game.terrain.fill("demo.terrain.wall".into());
    replace_terrain(&mut game, origin, "demo.terrain.stairs-down");
    replace_terrain(&mut game, curtain, "demo.terrain.curtain-closed");
    replace_terrain(&mut game, destination, "demo.terrain.floor");
    assert!(!game.is_walkable(curtain));
    assert!(game.can_drop_item_at(curtain));
    for index in 0..99 {
        game.gold_piles.push(crate::state::GoldPile {
            id: format!("test.gold.{index}"),
            amount: 1,
            position: curtain,
            appearance: rfb_protocol::GoldAppearanceDto::Copper,
            discovered: false,
        });
    }
    let draft = plain_draft(&mut game, "demo.item.arrow");
    assert_eq!(
        game.drop_generated_item_near(draft, origin)
            .unwrap()
            .unwrap()
            .0,
        destination
    );
}

#[test]
fn acquirement_keeps_partial_success_when_the_only_instant_artifact_is_used_up() {
    let mut source = source_with_pool(false, false);
    for item in &mut source.items {
        if matches!(
            item.id.as_str(),
            "demo.item.arkenstone" | "demo.item.arkenstone-of-thrain"
        ) {
            item.generation_level = 1;
            if let Some(artifact) = &mut item.artifact_generation {
                artifact.rarity_one_in = 1;
            }
        }
    }
    let content = Arc::new(ContentCatalog::from_artifact(
        rfb_content::encode_content(source).unwrap(),
    ));
    let mut game = Game::new_with_build(503, "demo.build.warrior").unwrap();
    descend_one_floor(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.gold_piles.clear();
    game.content = content;
    give_inventory_item(
        &mut game,
        "test.acquirement",
        "demo.item.star-acquirement-scroll",
    );
    let seed = (0..100)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(2);
            rng.bounded(10) == 0
        })
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    // Isolate the only instant artifact; newly imported artifacts must not
    // add unrelated rarity draws to the exhaustion reference.
    game.generated_artifact_ids.extend(
        game.content
            .item_definitions()
            .filter(|item| {
                item.artifact_generation.is_some() && item.id != "demo.item.arkenstone-of-thrain"
            })
            .map(|item| item.id.clone()),
    );
    let serial = game.next_item_instance_serial;
    let mut reference = game.clone();
    reference.rng.bounded(2);
    let mut draft = reference
        .generate_loot_draft_attempt(&context(&reference), ItemGenerationMode::TailoredGreat)
        .unwrap();
    assert_eq!(draft.kind_id, "demo.item.arkenstone-of-thrain");
    draft.origin_kind = Some(ItemOriginKindDto::Acquire);
    reference
        .drop_generated_item_near(draft, reference.player.position)
        .unwrap()
        .unwrap();
    // One successful make_object, then 999 calls exhausting their 1000 attempts.
    for _ in 0..999_000 {
        reference.rng.bounded(10);
    }
    // The dungeon also rolls ambient allocation during the scroll's ten ticks.
    for _ in 0..10 {
        reference
            .process_ambient_monster_allocation(&mut BTreeSet::new())
            .unwrap();
    }
    assert!(reference.entities.is_empty());
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.acquirement".into(),
            target: None,
        },
    );
    assert_eq!(game.items.len(), 1);
    let item = &game.items[0];
    assert_eq!(item.kind_id, "demo.item.arkenstone-of-thrain");
    assert_eq!(item.origin_kind, Some(ItemOriginKindDto::Acquire));
    assert_eq!(
        game.item_identification(item),
        ItemIdentificationDto::Unexamined
    );
    assert_eq!(game.next_item_instance_serial, serial + 1);
    assert_eq!(game.rng, reference.rng);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.use-acquirement" && event.args["count"] == "1")
    );
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn rejected_fixed_artifact_retains_generation_registration_without_identity_or_knowledge() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    let mut source = rfb_content::compile_pack_dir(&path).unwrap().content;
    let mut glove = source
        .items
        .iter()
        .find(|item| item.id == "demo.item.leather-gloves")
        .unwrap()
        .clone();
    glove.id = "test.item.instant-glove".into();
    glove.rfb_base_kind = None;
    glove.tags.push("artifact".into());
    glove.artifact_generation = Some(rfb_content::ArtifactGenerationDefinition {
        source_index: 10_000,
        base_item_kind_id: "demo.item.leather-gloves".into(),
        rarity_one_in: 1,
        instant: true,
        affix_ids: Vec::new(),
    });
    source.items.push(glove);
    let mut game = Game::new_with_build(503, "demo.build.high-mage-death").unwrap();
    game.content = Arc::new(ContentCatalog::from_artifact(
        rfb_content::encode_content(source).unwrap(),
    ));
    game.generated_artifact_ids
        .insert("demo.item.arkenstone-of-thrain".into());
    let mut context = context(&game);
    context.table_id = "demo.loot-table.base-items".into();
    context.floor_id = "test.floor".into();
    context.depth = 30;
    let seed = (0..100)
        .find(|seed| RfbRng::seeded(*seed).bounded(10) == 0)
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    let before_items = game.items.clone();
    let before_knowledge = game.item_knowledge.clone();
    let serial = game.next_item_instance_serial;
    let mut control = game.clone();
    let draft = control
        .generate_loot_draft_attempt(&context, ItemGenerationMode::Great)
        .unwrap();
    assert_eq!(draft.kind_id, "test.item.instant-glove");
    assert!(
        game.generate_loot_draft_attempt(&context, ItemGenerationMode::TailoredGreat)
            .is_none()
    );
    assert!(
        game.generated_artifact_ids
            .contains("test.item.instant-glove")
    );
    assert_eq!(game.rng, control.rng);
    assert_eq!(game.items, before_items);
    assert_eq!(game.item_knowledge, before_knowledge);
    assert_eq!(game.next_item_instance_serial, serial);
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert_eq!(restored.generated_artifact_ids, game.generated_artifact_ids);
}

#[test]
fn rejected_random_artifact_retains_its_name_and_rng_without_allocating() {
    let mut source = source_with_pool(false, true);
    source
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "test.loot-table.acquirement")
        .unwrap()
        .entries[0]
        .item_kind_id = "demo.item.leather-gloves".into();
    let mut template = Game::new_with_build(503, "demo.build.high-mage-death").unwrap();
    template.content = Arc::new(ContentCatalog::from_artifact(
        rfb_content::encode_content(source).unwrap(),
    ));
    let mut context = context(&template);
    context.depth = 80;
    let (seed, draft, expected_rng, names) = (0..1000)
        .find_map(|seed| {
            let mut game = template.clone();
            game.rng = RfbRng::seeded(seed);
            let draft = game.generate_loot_draft_attempt(&context, ItemGenerationMode::Great)?;
            if draft.artifact_name.is_none()
                || !game.item_is_icky(
                    &draft
                        .clone()
                        .into_item_instance(String::new(), ItemLocation::Inventory),
                    true,
                )
            {
                return None;
            }
            Some((seed, draft, game.rng, game.random_artifact_names))
        })
        .expect("natural random gloves without casting exemption must be reachable");
    let mut game = template.clone();
    game.rng = RfbRng::seeded(seed);
    assert!(
        game.generate_loot_draft_attempt(&context, ItemGenerationMode::TailoredGreat)
            .is_none()
    );
    assert_eq!(game.random_artifact_names, names);
    assert!(
        game.random_artifact_names
            .contains(draft.artifact_name.as_ref().unwrap())
    );
    assert_eq!(game.rng, expected_rng);
    assert_eq!(game.items, template.items);
    assert_eq!(game.item_knowledge, template.item_knowledge);
    assert_eq!(
        game.item_property_knowledge,
        template.item_property_knowledge
    );
    assert_eq!(
        game.next_item_instance_serial,
        template.next_item_instance_serial
    );
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert_eq!(restored.random_artifact_names, names);
    // Total placement failure preserves the name too; fixed preservation is separate.
    game.terrain.fill("demo.terrain.wall".into());
    assert!(
        game.drop_generated_item_near(draft, game.player.position)
            .unwrap()
            .is_none()
    );
    assert_eq!(game.random_artifact_names, names);
}

#[test]
fn acquirement_success_without_drop_space_consumes_scroll_and_does_not_retry_placement() {
    let mut game = prepare(catalog_with_pool(false, true));
    give_inventory_item(
        &mut game,
        "test.acquirement",
        "demo.item.acquirement-scroll",
    );
    let origin = game.player.position;
    // Walkable stairs keep this a valid player position but reject dropped objects.
    game.terrain.fill("demo.terrain.stairs-down".into());
    let mut reference = game.clone();
    let draft = reference
        .generate_one_loot_draft(&context(&reference), ItemGenerationMode::TailoredGreat)
        .unwrap();
    assert!(
        reference
            .drop_generated_item_near(draft, origin)
            .unwrap()
            .is_none()
    );
    let serial = game.next_item_instance_serial;
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.acquirement".into(),
            target: None,
        },
    );
    assert!(game.items.is_empty());
    assert_eq!(game.next_item_instance_serial, serial);
    assert_eq!(game.rng, reference.rng);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.use-acquirement" && event.args["count"] == "0")
    );
}
