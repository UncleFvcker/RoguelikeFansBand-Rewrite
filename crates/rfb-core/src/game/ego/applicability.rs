// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::{
    Game,
    loot::{ItemGenerationMode, LootContext, LootSource},
};
use crate::state::ItemLocation;

fn context(game: &Game, table: &str) -> LootContext {
    LootContext {
        table_id: format!("demo.loot-table.{table}"),
        floor_id: game.current_floor_id.clone(),
        depth: 80,
        source: LootSource::MonsterDeath {
            actor_id: "test.theme".into(),
            themed: true,
        },
    }
}

#[test]
fn themed_ego_selection_filters_before_weighting_and_empty_pool_falls_back() {
    let game = Game::new_with_build(87, "demo.build.warrior").unwrap();
    let content = &game.content;
    for (theme, category, index) in [
        ("archer", RfbEgoTypeDefinition::Ring, 207),
        ("mage", RfbEgoTypeDefinition::Gloves, 138),
        ("dwarf", RfbEgoTypeDefinition::Helmet, 118),
    ] {
        let affix = content
            .affix_definitions()
            .find(|affix| {
                affix
                    .rfb_ego
                    .as_ref()
                    .is_some_and(|ego| ego.source_index == index)
            })
            .unwrap();
        let mut rng = RfbRng::seeded(87);
        let mut expected = rng.clone();
        expected.bounded(u64::from(rfb_ego_weight(
            affix.rfb_ego.as_ref().unwrap().rarity,
            affix.generation_level,
            affix.generation_max_level,
            80,
        )));
        assert_eq!(
            roll_rfb_ego_from_affixes(
                theme,
                content.affix_definitions(),
                &mut rng,
                80,
                &[category]
            ),
            Some(affix.id.as_str())
        );
        assert_eq!(rng, expected);
        let remaining = content
            .affix_definitions()
            .filter(|candidate| candidate.id != affix.id);
        let mut themed_rng = RfbRng::seeded(87);
        let mut plain_rng = themed_rng.clone();
        assert_eq!(
            roll_rfb_ego_from_affixes(theme, remaining.clone(), &mut themed_rng, 80, &[category]),
            roll_rfb_ego_from_affixes("", remaining, &mut plain_rng, 80, &[category])
        );
        assert_eq!(themed_rng, plain_rng);
    }
}

#[test]
fn real_warrior_gets_mage_and_dwarf_themed_equipment_then_equips_and_restores() {
    let mut game = Game::new_with_build(87, "demo.build.warrior").unwrap();

    let mut seen = BTreeSet::new();
    for attempt in 0..200 {
        let context = context(&game, if attempt % 2 == 0 { "mage" } else { "dwarf" });
        let Some(draft) = game.generate_one_loot_draft(&context, ItemGenerationMode::Great) else {
            continue;
        };
        let Some(base) = game.content.item(&draft.kind_id).unwrap().rfb_base_kind else {
            continue;
        };
        let Some(id) = draft.affix_ids.first() else {
            continue;
        };
        let index = game
            .content
            .affix(id)
            .unwrap()
            .rfb_ego
            .as_ref()
            .unwrap()
            .source_index;
        match base.tval {
            30 => assert_eq!(index, 147),
            32 if context.drop_theme() == "dwarf" => assert_eq!(index, 118),
            34 => assert_eq!(index, 60),

            45 => assert!(matches!(index, 200 | 201 | 205 | 208 | 209)),
            _ => continue,
        }
        if seen.insert(base.tval) {
            let item = game
                .commit_generated_item_draft(draft, ItemLocation::Inventory)
                .unwrap();
            let id = item.id.clone();
            game.items.push(item);
            assert!(game.equip_inventory_item(&id, None).is_some());
        }
        if seen.len() == 4 {
            break;
        }
    }
    assert_eq!(seen, BTreeSet::from([30, 32, 34, 45]));
    game.refresh_player_resource_maxima();
    let restored = Game::from_save(game.to_save()).unwrap();
    for item in &game.items {
        assert_eq!(
            restored.items.iter().find(|saved| saved.id == item.id),
            Some(item)
        );
    }
    assert_eq!(
        restored.player_derived_stats().speed.value,
        game.player_derived_stats().speed.value
    );
    assert_eq!(restored.rng, game.rng);
}

#[test]
fn bad_luck_fixed_artifact_attempts_reduce_reference_depth_and_consume_town_roll() {
    let mut game = Game::new_with_build(87, "demo.build.warrior").unwrap();
    assert!(game.gain_mutation("rfb.mutation.bad-luck", &mut Vec::new()));
    let mut town = context(&game, "base-items");
    town.floor_id = "test.town".into();
    town.depth = 0;
    for instant in [false, true] {
        let mut expected = game.rng.clone();
        expected.bounded(4);
        assert_eq!(
            game.roll_fixed_artifact_kind_id(&town, Some("demo.item.whip"), instant),
            None
        );
        assert_eq!(game.rng, expected);
    }
    // Isolate the make_artifact reference-level contract from floor creation.
    let mut dungeon = town;
    dungeon.floor_id = "test.reference-depth".into();
    dungeon.depth = 40;
    for seed in 0..100 {
        game.rng = RfbRng::seeded(seed);
        let mut expected = game.clone();
        expected
            .progress
            .active_mutation_ids
            .remove("rfb.mutation.bad-luck");
        let mut adjusted = dungeon.clone();
        adjusted.depth -= adjusted.depth / (4 * (1 + expected.rng.bounded(4) as u16));
        assert_eq!(
            game.roll_fixed_artifact_kind_id(&dungeon, Some("demo.item.whip"), false),
            expected.roll_fixed_artifact_kind_id(&adjusted, Some("demo.item.whip"), false)
        );
        assert_eq!(game.rng, expected.rng);
    }
}

#[test]
fn real_tomte_bad_luck_hat_limits_speed_and_survives_equipping_and_save() {
    let mut game = Game::new_with_build_race_and_name(
        87,
        "demo.build.warrior",
        "rfb-legacy.race.tomte",
        "Tomte",
    )
    .unwrap();
    assert!(game.gain_mutation("rfb.mutation.bad-luck", &mut Vec::new()));
    let affix = game
        .content
        .affix_definitions()
        .find(|affix| {
            affix
                .rfb_ego
                .as_ref()
                .is_some_and(|ego| ego.source_index == 121)
        })
        .unwrap();
    let ids = vec![affix.id.clone()];
    let mut found = None;
    for seed in 0..1000 {
        let mut normal_rng = RfbRng::seeded(seed);
        let normal = materialize_ego_with_rng(
            false,
            &game.content,
            &mut normal_rng,
            "demo.item.knit-cap",
            ids.clone(),
            |_| 90,
            90,
            2,
        );
        let speed = normal.rolled_affixes[0].properties.modifiers.speed;
        if speed <= 3 {
            continue;
        }
        game.rng = RfbRng::seeded(seed);
        let bad = materialize_ego_with_rng(
            game.progress
                .active_mutation_ids
                .contains("rfb.mutation.bad-luck"),
            &game.content,
            &mut game.rng,
            "demo.item.knit-cap",
            ids.clone(),
            |_| 90,
            90,
            2,
        );
        let bad_speed = bad.rolled_affixes[0].properties.modifiers.speed;
        assert!((1..=3).contains(&bad_speed));
        // Bad Luck has already consumed the first successful continuation die.
        // Replaying only the remaining source loop must recover the normal result.
        let mut replay = game.rng.clone();
        let mut expected_speed = bad_speed;
        loop {
            expected_speed += 1;
            if !one_in(&mut replay, 7) {
                break;
            }
        }
        assert_eq!(expected_speed, speed);
        assert_eq!(replay, normal_rng);
        found = Some(bad);
        break;
    }
    let materialization =
        found.expect("source continuation roll can exceed three without Bad Luck");
    let context = context(&game, "base-items");
    let draft = game.fixed_item_draft(&context, "demo.item.knit-cap".into());
    let mut item = game
        .commit_generated_item_draft(draft, ItemLocation::Inventory)
        .unwrap();
    materialization.apply_to(&mut item);
    let speed = item.rolled_affixes[0].properties.modifiers.speed;
    assert!((1..=3).contains(&speed));
    let id = item.id.clone();
    game.items.push(item);
    let before_speed = game.player_derived_stats().speed.value;
    assert!(game.equip_inventory_item(&id, None).is_some());
    assert!(game.player_derived_stats().speed.value > before_speed);
    game.refresh_player_resource_maxima();
    let restored = Game::from_save(game.to_save()).unwrap();
    for item in &game.items {
        assert_eq!(
            restored.items.iter().find(|saved| saved.id == item.id),
            Some(item)
        );
    }
    assert_eq!(
        restored.player_derived_stats().speed.value,
        game.player_derived_stats().speed.value
    );
    assert_eq!(restored.rng, game.rng);
}
