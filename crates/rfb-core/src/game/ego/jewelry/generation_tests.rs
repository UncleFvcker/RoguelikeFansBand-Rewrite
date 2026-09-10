// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::loot::LootSource;
use rfb_protocol::ItemQualityDto;

fn context(game: &Game, depth: u16) -> LootContext {
    LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: game.current_floor_id.clone(),
        depth,
        source: LootSource::ItemUse {
            item_id: "test.jewelry".into(),
        },
    }
}

fn value(game: &Game, draft: &GeneratedItemDraft) -> i32 {
    item_value::obj_value_real(
        &game.content,
        &draft
            .clone()
            .into_item_instance(String::new(), ItemLocation::Inventory),
    )
    .unwrap()
}

fn seed_for_upgrade(upgrade: bool) -> u64 {
    (0..100)
        .find(|seed| (RfbRng::seeded(*seed).bounded(8) == 0) == upgrade)
        .unwrap()
}

#[test]
fn jewelry_limits_use_strict_level_boundaries_and_one_upgrade_draw() {
    let seed = seed_for_upgrade(false);
    for (level, expected) in [
        (0, (0, 5_000)),
        (9, (0, 5_000)),
        (10, (0, 7_000)),
        (19, (0, 7_000)),
        (20, (1_000, 15_000)),
        (29, (1_000, 15_000)),
        (30, (2_500, 20_000)),
        (39, (2_500, 20_000)),
        (40, (5_000, 30_000)),
        (49, (5_000, 30_000)),
        (50, (7_500, 60_000)),
        (59, (7_500, 60_000)),
        (60, (10_000, 0)),
        (69, (10_000, 0)),
        (70, (12_500, 0)),
        (79, (12_500, 0)),
        (80, (15_000, 0)),
        (89, (15_000, 0)),
        (90, (15_000, 0)),
        (127, (15_000, 0)),
    ] {
        let mut rng = RfbRng::seeded(seed);
        let mut expected_rng = rng.clone();
        expected_rng.bounded(8);
        assert_eq!(power_limits(&mut rng, level, 0), expected, "level {level}");
        assert_eq!(rng, expected_rng);
    }
    for upgrade in [false, true] {
        for mode in [
            0,
            AM_GOOD,
            AM_GREAT,
            AM_FORCE_EGO,
            AM_QUEST,
            AM_GOOD | AM_GREAT,
            AM_GOOD | AM_FORCE_EGO,
            AM_GOOD | AM_QUEST,
            AM_GOOD | AM_GREAT | AM_FORCE_EGO | AM_QUEST,
        ] {
            let mut rng = RfbRng::seeded(seed_for_upgrade(upgrade));
            let mut expected_rng = rng.clone();
            expected_rng.bounded(8);
            let strong = mode & (AM_GREAT | AM_FORCE_EGO | AM_QUEST) != 0;
            let minimum = if strong || upgrade {
                5_000
            } else if mode == AM_GOOD {
                2_500
            } else {
                0
            };
            let maximum = if strong {
                10_000
            } else if mode == AM_GOOD {
                7_500
            } else {
                5_000
            };
            assert_eq!(
                power_limits(&mut rng, 9, mode),
                (minimum, maximum * if upgrade { 2 } else { 1 })
            );
            assert_eq!(rng, expected_rng);
            let mut rng = RfbRng::seeded(seed_for_upgrade(upgrade));
            assert_eq!(power_limits(&mut rng, 60, mode), (10_000, 0));
            assert_eq!(
                rng, expected_rng,
                "unbounded still consumes the upgrade draw"
            );
        }
    }
}

#[test]
fn jewelry_value_limits_are_inclusive_and_zero_disables_that_bound() {
    for (score, limits, accepted) in [
        (999, (1_000, 15_000), false),
        (1_000, (1_000, 15_000), true),
        (15_000, (1_000, 15_000), true),
        (15_001, (1_000, 15_000), false),
        (-1, (0, 5_000), true),
        (i32::MAX, (10_000, 0), true),
        (9_999, (10_000, 0), false),
        (i32::MIN, (0, 0), true),
    ] {
        assert_eq!(accepts_value(score, limits), accepted);
    }
}

#[test]
fn jewelry_retry_accepts_first_or_later_fresh_candidate_with_identical_rng() {
    let base = Game::new_with_build(86, "demo.build.warrior").unwrap();
    for kind in ["demo.item.ring", "demo.item.amulet"] {
        for power in [1, -1] {
            let mut initial = base.clone();
            let context = context(&initial, 20);
            let mut original = initial.fixed_item_draft(&context, kind.into());
            original.enchantments.to_hit = 3;
            original.quality = if power > 0 {
                ItemQualityDto::Fine
            } else {
                ItemQualityDto::Ordinary
            };
            initial.rng = RfbRng::seeded(86);
            let before = original.clone();
            let mut expected = initial.clone();
            let first = expected.create_jewelry_candidate(
                &original,
                &context,
                20,
                power,
                ItemGenerationMode::Ordinary,
            );
            let mut actual = initial.clone();
            let (draft, attempts) = create_with_limits(
                &mut actual,
                &original,
                &context,
                20,
                power,
                ItemGenerationMode::Ordinary,
                (0, 0),
            );
            assert_eq!((draft, attempts), (first.clone(), 1));
            assert_eq!(actual.rng, expected.rng);
            assert_eq!(actual.random_artifact_names, expected.random_artifact_names);
            let first_value = value(&expected, &first);
            let mut later = None;
            for attempt in 2..=20 {
                let candidate = expected.create_jewelry_candidate(
                    &original,
                    &context,
                    20,
                    power,
                    ItemGenerationMode::Ordinary,
                );
                let score = value(&expected, &candidate);
                if score > 0 && score != first_value {
                    later = Some((candidate, score, attempt));
                    break;
                }
            }
            let (candidate, score, attempt) = later.expect("distinct later value");
            let mut actual = initial.clone();
            let result = create_with_limits(
                &mut actual,
                &original,
                &context,
                20,
                power,
                ItemGenerationMode::Ordinary,
                (score, score),
            );
            assert_eq!(result, (candidate, attempt));
            assert_eq!(actual.rng, expected.rng);
            assert_eq!(actual.random_artifact_names, expected.random_artifact_names);
            assert_eq!(
                actual.next_item_instance_serial,
                initial.next_item_instance_serial
            );
            assert_eq!(original, before);
        }
    }
}

#[test]
fn jewelry_exhaustion_uses_a_fresh_1001st_candidate_including_inner_artifact_rng() {
    let mut initial = Game::new_with_build(86, "demo.build.warrior").unwrap();
    let context = context(&initial, 20);
    let original = initial.fixed_item_draft(&context, "demo.item.ring".into());
    initial.rng = RfbRng::seeded(86);
    let mut expected = initial.clone();
    let mut last = None;
    let mut artifacts = 0;
    for _ in 0..1001 {
        let candidate = expected.create_jewelry_candidate(
            &original,
            &context,
            20,
            1,
            ItemGenerationMode::Ordinary,
        );
        assert!(value(&expected, &candidate) < i32::MAX);
        artifacts += usize::from(candidate.artifact_name.is_some());
        last = Some(candidate);
    }
    assert!(
        artifacts > 0,
        "outer retries must include the full inner artifact loop"
    );
    let mut actual = initial.clone();
    let result = create_with_limits(
        &mut actual,
        &original,
        &context,
        20,
        1,
        ItemGenerationMode::Ordinary,
        (i32::MAX, 0),
    );
    assert_eq!(result, (last.unwrap(), 1001));
    assert_eq!(actual.rng, expected.rng);
    assert_eq!(actual.random_artifact_names, expected.random_artifact_names);
    assert_eq!(
        actual.next_item_instance_serial,
        initial.next_item_instance_serial
    );
}

#[test]
fn jewelry_natural_pipeline_uses_one_outer_limit_roll_and_saves_complete_candidates() {
    use std::sync::Arc;
    let artifact = rfb_content::compile_pack_dir(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original"),
    )
    .unwrap();
    let base = Game::new_with_build(86, "demo.build.warrior").unwrap();
    for kind in ["demo.item.ring", "demo.item.amulet"] {
        let mut source = artifact.clone();
        let table = source
            .content
            .loot_tables
            .iter_mut()
            .find(|table| table.id == "demo.loot-table.base-items")
            .unwrap();
        table.entries.retain(|entry| entry.item_kind_id == kind);
        table.entries.truncate(1);
        assert_eq!(table.entries.len(), 1);
        let weight = table.entries[0].weight;
        let policy = table.quality_policy.unwrap();
        let mut initial = base.clone();
        initial.content = Arc::new(ContentCatalog::from_artifact(source));
        let context = context(&initial, 30);
        let original = initial.fixed_item_draft(&context, kind.into());
        for (mode, flags) in [
            (ItemGenerationMode::Ordinary, 0),
            (ItemGenerationMode::Good, AM_GOOD),
            (ItemGenerationMode::GreatOnly, AM_GREAT),
            (ItemGenerationMode::Great, AM_GOOD | AM_GREAT),
            (ItemGenerationMode::TailoredGreat, AM_GOOD | AM_GREAT),
            (
                ItemGenerationMode::Artifact {
                    no_fixed_artifact: true,
                },
                AM_GOOD | AM_GREAT,
            ),
        ] {
            let mut powers = BTreeSet::new();
            for seed in 0..1500 {
                let mut expected = initial.clone();
                expected.rng = RfbRng::seeded(seed);
                let mut actual = expected.clone();
                let special = matches!(mode, ItemGenerationMode::Artifact { .. });
                let one_in = if matches!(
                    mode,
                    ItemGenerationMode::Ordinary | ItemGenerationMode::GreatOnly
                ) {
                    1000
                } else {
                    10
                };
                if !special
                    && expected
                        .roll_instant_fixed_artifact_kind_id(&context, one_in)
                        .is_some()
                {
                    continue;
                }
                expected.roll_weighted_index(&[weight]);
                let power = expected.roll_rfb_depth_loot_power(policy, 30, true, false, mode);
                let fixed_rolls = if special {
                    0
                } else if flags & AM_GREAT != 0 {
                    4
                } else {
                    usize::from(power >= 2)
                };
                if (0..fixed_rolls).any(|_| {
                    expected
                        .roll_fixed_artifact_kind_id(&context, Some(kind), false)
                        .is_some()
                }) {
                    continue;
                }
                if powers.contains(&power) {
                    continue;
                }
                let mut input = original.clone();
                input.quality = match power {
                    1 => ItemQualityDto::Fine,
                    2.. => ItemQualityDto::Exceptional,
                    _ => ItemQualityDto::Ordinary,
                };
                let expected_draft = if power == 0 {
                    input
                } else {
                    let limits = power_limits(&mut expected.rng, 30, flags);
                    let (draft, attempts) = create_with_limits(
                        &mut expected,
                        &input,
                        &context,
                        30,
                        power,
                        mode,
                        limits,
                    );
                    assert!(attempts == 1001 || accepts_value(value(&expected, &draft), limits));
                    draft
                };
                let actual_draft = actual.generate_one_loot_draft(&context, mode).unwrap();
                assert_eq!(
                    actual_draft, expected_draft,
                    "{kind}/{mode:?}/{seed}/power {power}"
                );
                assert_eq!(actual.rng, expected.rng);
                assert_eq!(actual.random_artifact_names, expected.random_artifact_names);
                assert_eq!(
                    actual.next_item_instance_serial,
                    initial.next_item_instance_serial
                );
                if special {
                    assert!(actual_draft.artifact_name.is_some());
                }
                let item = actual
                    .commit_generated_item_draft(actual_draft, ItemLocation::Inventory)
                    .unwrap();
                actual.items.push(item);
                actual.content = base.content.clone();
                assert_eq!(
                    Game::from_save(actual.to_save()).unwrap().state_hash(),
                    actual.state_hash()
                );
                powers.insert(power);
                if mode != ItemGenerationMode::Ordinary || powers.len() == 5 {
                    break;
                }
            }
            if mode == ItemGenerationMode::Ordinary {
                assert_eq!(powers, [-2, -1, 0, 1, 2].into_iter().collect());
            } else {
                assert!(!powers.is_empty(), "{kind}/{mode:?}");
            }
        }
    }
}

#[test]
#[ignore = "auxiliary natural jewelry value sampling; not a branch acceptance test"]
fn jewelry_value_distribution_report() {
    use std::sync::Arc;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let artifact = rfb_content::compile_pack_dir(&root.join("packs/rfb-demo-original")).unwrap();
    let base = Game::new_with_build(86, "demo.build.warrior").unwrap();
    let mut rows = Vec::new();
    for kind in ["demo.item.ring", "demo.item.amulet"] {
        let mut source = artifact.clone();
        let table = source
            .content
            .loot_tables
            .iter_mut()
            .find(|table| table.id == "demo.loot-table.base-items")
            .unwrap();
        table.entries.retain(|entry| entry.item_kind_id == kind);
        table.entries.truncate(1);
        assert_eq!(table.entries.len(), 1);
        let content = Arc::new(ContentCatalog::from_artifact(source));
        for level in [10, 29, 59, 80] {
            for mode in [ItemGenerationMode::Good, ItemGenerationMode::Great] {
                let mut game = base.clone();
                game.content = content.clone();
                let seed = 86_000 + u64::from(level);
                game.rng = RfbRng::seeded(seed);
                let context = context(&game, level);
                let mut values = Vec::new();
                let mut random_artifacts = 0;
                let mut fixed_artifacts = 0;
                for _ in 0..128 {
                    let draft = game.generate_one_loot_draft(&context, mode).unwrap();
                    let item = game
                        .commit_generated_item_draft(draft, ItemLocation::Inventory)
                        .unwrap();
                    if item.kind_id != kind {
                        fixed_artifacts += 1;
                        continue;
                    }
                    random_artifacts += usize::from(item.artifact_name.is_some());
                    values.push(item_value::obj_value_real(&game.content, &item).unwrap());
                }
                values.sort_unstable();
                let n = values.len();
                rows.push(serde_json::json!({
                    "kind": kind, "level": level, "mode": format!("{mode:?}"), "seed": seed,
                    "draws": 128, "jewelryCount": n, "fixedArtifactsExcluded": fixed_artifacts,
                    "randomArtifacts": random_artifacts, "minimum": values[0],
                    "p10": values[n / 10], "median": values[n / 2], "p90": values[n * 9 / 10],
                    "maximum": values[n - 1], "rngDraws": game.rng_draw_counter(),
                }));
            }
        }
    }
    let report = serde_json::json!({
        "sourceCommit": "a0d92b6378d148c5262cc236b8fa6ed2ca06a54c",
        "method": "128 consecutive natural drafts per group from the formal allocation narrowed to one jewelry base; fixed artifacts committed and excluded; project-local RNG; supporting statistics only",
        "groups": rows,
    });
    let path = root.join("target/e86-jewelry-value-distribution.json");
    std::fs::write(&path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("{}", path.display());
}
