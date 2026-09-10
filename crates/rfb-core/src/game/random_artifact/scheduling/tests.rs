// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::loot::LootSource;
use rfb_content::{CompiledArtifact, ContentCatalog};
use std::sync::Arc;

fn base(tval: u16, sval: u16) -> RfbBaseKindDefinition {
    RfbBaseKindDefinition {
        source_index: 0,
        tval,
        sval,
    }
}

fn context(game: &Game) -> LootContext {
    LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: game.current_floor_id.clone(),
        depth: 80,
        source: LootSource::MonsterDeath {
            actor_id: "test.drop".into(),
            themed: false,
        },
    }
}

fn source() -> CompiledArtifact {
    rfb_content::compile_pack_dir(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original"),
    )
    .unwrap()
}

fn narrow(game: &mut Game, artifact: &CompiledArtifact, kind: &str) {
    let mut artifact = artifact.clone();
    let table = artifact
        .content
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.base-items")
        .unwrap();
    // This test fixes the base kind and exercises materialization.
    table.kind_selection = None;
    table.entries.retain(|entry| entry.item_kind_id == kind);
    table.entries.truncate(1);
    assert_eq!(table.entries.len(), 1, "{kind}");
    game.content = Arc::new(ContentCatalog::from_artifact(artifact));
}

#[test]
fn random_artifact_bad_luck_keeps_creation_depth_separate_from_value_level() {
    let mut game = Game::new_with_build(85, "demo.build.warrior").unwrap();
    game.progress
        .active_mutation_ids
        .insert("rfb.mutation.bad-luck".into());
    let context = context(&game);
    let draft = game.fixed_item_draft(&context, "demo.item.dagger".into());
    let original = draft
        .clone()
        .into_item_instance(String::new(), ItemLocation::Inventory);
    let mut expected_rng = game.rng.clone();
    let mut expected_names = game.random_artifact_names.clone();
    let (expected, _) = materialize(
        &game.content,
        &mut expected_rng,
        &original,
        Creation {
            level: 80,
            class_id: "demo.class.warrior",
            ..Default::default()
        },
        &mut expected_names,
        60,
        3,
        &mut false,
        true,
    )
    .unwrap();
    let actual = game.materialize_random_artifact_draft(draft, &context, 60, 3, false);
    assert_eq!(actual, expected.into());
    assert_eq!(game.rng, expected_rng);
    assert_eq!(game.random_artifact_names, expected_names);
}

#[test]
fn random_artifact_schedule_uses_slot_chance_depth_and_power_short_circuits() {
    // Source denominators at level 80, after base -= base * level / 200.
    for (tval, denominator) in [(19, 12), (20, 18), (21, 24), (31, 12), (38, 30)] {
        let seed = (0..1000)
            .find(|seed| RfbRng::seeded(*seed).bounded(denominator) == 0)
            .unwrap();
        let mut rng = RfbRng::seeded(seed);
        let mut expected = rng.clone();
        expected.bounded(denominator);
        assert_eq!(
            select(
                &mut rng,
                base(tval, 0),
                80,
                -2,
                ItemGenerationMode::Ordinary
            ),
            Some((80, false))
        );
        assert_eq!(rng, expected);
        let before = rng.clone();
        assert_eq!(
            select(&mut rng, base(tval, 0), 80, 3, ItemGenerationMode::Ordinary),
            Some((80, false))
        );
        assert_eq!(rng, before);
        for power in [-1, 0, 1] {
            assert_eq!(
                select(
                    &mut rng,
                    base(tval, 0),
                    80,
                    power,
                    ItemGenerationMode::Ordinary
                ),
                None
            );
            assert_eq!(rng, before);
        }
    }
    let mut rng = RfbRng::seeded(85);
    let before = rng.clone();
    for tval in [16, 17, 18, 46] {
        assert!(select(&mut rng, base(tval, 0), 80, 3, ItemGenerationMode::Ordinary).is_none());
    }
    assert_eq!(rng, before);
}

#[test]
fn random_artifact_jewelry_and_feanor_keep_their_distinct_gates() {
    for tval in [40, 45] {
        let mut rng = RfbRng::seeded(85);
        let before = rng.clone();
        assert_eq!(
            select(
                &mut rng,
                base(tval, 0),
                80,
                3,
                ItemGenerationMode::Artifact {
                    no_fixed_artifact: true
                }
            ),
            Some((80, false))
        );
        assert_eq!(rng, before, "AM_SPECIAL skips the amulet and level gates");
    }
    for seed in 0..40 {
        let mut rng = RfbRng::seeded(seed);
        let mut expected = rng.clone();
        // Level 20 needs no trim draws. Forced power without AM_SPECIAL still
        // has jewelry's 1/3 gate; amulets also have the earlier 5/6 gate.
        let accepted = expected.bounded(3) == 0;
        assert_eq!(
            select(&mut rng, base(45, 0), 20, 3, ItemGenerationMode::Ordinary),
            accepted.then_some((20, true))
        );
        assert_eq!(rng, expected);
        rng = RfbRng::seeded(seed);
        expected = rng.clone();
        let accepted = expected.bounded(6) != 0 && expected.bounded(3) == 0;
        assert_eq!(
            select(&mut rng, base(40, 0), 20, 3, ItemGenerationMode::Ordinary),
            accepted.then_some((20, true))
        );
        assert_eq!(rng, expected);
        rng = RfbRng::seeded(seed);
        expected = rng.clone();
        expected.bounded(7);
        assert_eq!(
            select(&mut rng, base(39, 2), 80, 3, ItemGenerationMode::Ordinary),
            Some((80, false))
        );
        assert_eq!(rng, expected, "Feanor rolls before checking forced power");
        let before = rng.clone();
        for sval in [0, 1] {
            assert!(
                select(
                    &mut rng,
                    base(39, sval),
                    80,
                    3,
                    ItemGenerationMode::Ordinary
                )
                .is_none()
            );
        }
        assert_eq!(rng, before);
    }
    let mut adjusted = BTreeSet::new();
    for seed in 0..300 {
        if let Some((level, true)) = select(
            &mut RfbRng::seeded(seed),
            base(45, 0),
            100,
            2,
            ItemGenerationMode::Ordinary,
        ) {
            assert!((20..100).contains(&level));
            adjusted.insert(level);
        }
    }
    assert!(adjusted.len() > 1, "jewelry level uses stochastic trim");
}

#[test]
fn random_artifact_forced_base_pipeline_covers_slots_and_special_robe_and_light() {
    let source = source();
    let mut game = Game::new_with_build(85, "demo.build.warrior").unwrap();
    let mut slots = BTreeSet::new();
    for kind in [
        "long-bow",
        "harp",
        "mattock",
        "mace",
        "spear",
        "dagger",
        "pair-of-hard-leather-boots",
        "leather-gloves",
        "iron-helm",
        "iron-crown",
        "small-leather-shield",
        "elven-cloak",
        "robe",
        "chain-mail",
        "multi-hued-dragon-scale-mail",
        "ring",
        "amulet",
        "feanorian-lamp",
    ] {
        let kind = format!("demo.item.{kind}");
        narrow(&mut game, &source, &kind);
        let mut context = context(&game);
        if kind == "demo.item.feanorian-lamp" {
            context.depth = 30;
        }
        let mut found = None;
        let mut robe_ego = false;
        for seed in 0..40 {
            game.rng = RfbRng::seeded(seed);
            let serial = game.next_item_instance_serial;
            let draft = game
                .generate_one_loot_draft(
                    &context,
                    ItemGenerationMode::Artifact {
                        no_fixed_artifact: true,
                    },
                )
                .unwrap_or_else(|| panic!("no eligible drop for {kind}"));
            assert_eq!(game.next_item_instance_serial, serial);
            assert_eq!(draft.kind_id, kind, "no fixed artifact is allowed");
            if draft.artifact_name.is_some() {
                found = Some(draft);
            } else if kind == "demo.item.robe" {
                robe_ego |= draft.affix_ids.iter().any(|id| {
                    game.content
                        .affix(id)
                        .unwrap()
                        .rfb_ego
                        .as_ref()
                        .unwrap()
                        .types
                        .contains(&rfb_content::RfbEgoTypeDefinition::Robe)
                });
            }
            if found.is_some() && (kind != "demo.item.robe" || robe_ego) {
                break;
            }
        }
        let draft = found.unwrap_or_else(|| panic!("no artifact for {kind}"));
        slots.insert(
            game.content
                .item(&kind)
                .unwrap()
                .rfb_base_kind
                .unwrap()
                .tval,
        );
        assert!(draft.affix_ids.is_empty() && draft.rolled_affixes.is_empty());
        let item = game
            .commit_generated_item_draft(draft, ItemLocation::Ground(game.player.position))
            .unwrap();
        assert!(item.is_artifact(&game.content));
        if kind == "demo.item.robe" {
            assert!(robe_ego);
        }
    }
    assert_eq!(slots.len(), 17);
}

#[test]
fn random_artifact_save_preserves_rejected_names_and_continued_generation() {
    let mut game = Game::new_with_build(85, "demo.build.warrior").unwrap();
    game.items.clear();
    game.entities.clear();
    let context = context(&game);
    let mode = ItemGenerationMode::Artifact {
        no_fixed_artifact: true,
    };
    for _ in 0..6 {
        let items = game
            .generate_loot_instances_internal(
                &context,
                ItemLocation::Inventory,
                false,
                Some(1),
                mode,
            )
            .unwrap();
        game.items.extend(items);
    }
    assert!(game.items.iter().any(|item| item.artifact_name.is_some()));
    let produced: BTreeSet<_> = game
        .items
        .iter()
        .filter_map(|item| item.artifact_name.as_ref())
        .collect();
    assert!(
        game.random_artifact_names
            .iter()
            .any(|name| !name.is_empty() && !produced.contains(name))
    );
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(game.state_hash(), restored.state_hash());
    for _ in 0..6 {
        let actual = game
            .generate_loot_instances_internal(
                &context,
                ItemLocation::Inventory,
                false,
                Some(1),
                mode,
            )
            .unwrap();
        let replay = restored
            .generate_loot_instances_internal(
                &context,
                ItemLocation::Inventory,
                false,
                Some(1),
                mode,
            )
            .unwrap();
        assert_eq!(actual, replay);
        assert_eq!(game.rng, restored.rng);
        assert_eq!(game.random_artifact_names, restored.random_artifact_names);
        game.items.extend(actual);
        restored.items.extend(replay);
        assert_eq!(game.state_hash(), restored.state_hash());
    }
    for names in [
        vec!["missing empty quark".into()],
        vec!["".into(), "".into()],
        vec!["".into(), "bad\nname".into()],
    ] {
        let mut invalid = game.to_save();
        invalid.random_artifact_names = names;
        assert!(Game::from_save(invalid).is_err());
    }
    let mut altered = game.clone();
    altered
        .random_artifact_names
        .insert("another rejected name".into());
    assert_ne!(altered.state_hash(), game.state_hash());
}

#[test]
fn random_artifact_negative_power_reaches_a_cursed_equippable_instance() {
    let artifact = source();
    let mut game = Game::new_with_build(85, "demo.build.warrior").unwrap();
    game.items.clear();
    game.entities.clear();
    let original_content = game.content.clone();
    narrow(&mut game, &artifact, "demo.item.dagger");
    let context = context(&game);
    let table = game.content.loot_table(&context.table_id).unwrap().clone();
    let mut found = None;
    for seed in 0..5000 {
        game.rng = RfbRng::seeded(seed);
        let mut prefix = game.clone();
        if prefix
            .roll_instant_fixed_artifact_kind_id(&context, 1000)
            .is_some()
        {
            continue;
        }
        prefix.roll_weighted_index(&[table.entries[0].weight]);
        if prefix.roll_rfb_depth_loot_power(
            table.quality_policy.unwrap(),
            80,
            false,
            false,
            ItemGenerationMode::Ordinary,
        ) != -2
        {
            continue;
        }
        let draft = game
            .generate_one_loot_draft(&context, ItemGenerationMode::Ordinary)
            .unwrap();
        if draft.artifact_name.is_some() {
            found = Some(draft);
            break;
        }
    }
    let draft = found.expect("natural -2 artifact branch");
    assert_eq!(draft.kind_id, "demo.item.dagger");
    assert!(draft.curse.is_some());
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Inventory)
        .unwrap();
    let id = item.id.clone();
    game.items.push(item);
    game.content = original_content;
    assert!(game.equip_inventory_item(&id, None).is_some());
    assert!(game.items[0].curse.is_some());
    assert_eq!(
        Game::from_save(game.to_save()).unwrap().state_hash(),
        game.state_hash()
    );
}

#[test]
fn random_artifact_is_not_sampled_after_either_fixed_artifact_success() {
    let artifact = source();
    let mut game = Game::new_with_build(85, "demo.build.warrior").unwrap();
    narrow(&mut game, &artifact, "demo.item.whip");
    let context = context(&game);
    let weight = game.content.loot_table(&context.table_id).unwrap().entries[0].weight;
    let mut seen = BTreeSet::new();
    for seed in 0..1000 {
        game.rng = RfbRng::seeded(seed);
        let mut expected = game.clone();
        let (instant, kind) =
            if let Some(kind) = expected.roll_instant_fixed_artifact_kind_id(&context, 10) {
                (true, kind)
            } else {
                expected.roll_weighted_index(&[weight]);
                let Some(kind) = (0..4).find_map(|_| {
                    expected.roll_fixed_artifact_kind_id(&context, Some("demo.item.whip"), false)
                }) else {
                    continue;
                };
                (false, kind)
            };
        let draft = expected.fixed_item_draft(&context, kind);
        let actual = game
            .generate_one_loot_draft(
                &context,
                ItemGenerationMode::Artifact {
                    no_fixed_artifact: false,
                },
            )
            .unwrap();
        assert_eq!(actual, draft);
        assert_eq!(game.rng, expected.rng);
        assert_eq!(game.random_artifact_names, expected.random_artifact_names);
        seen.insert(instant);
        if seen.len() == 2 {
            break;
        }
    }
    assert_eq!(seen.len(), 2);
}
