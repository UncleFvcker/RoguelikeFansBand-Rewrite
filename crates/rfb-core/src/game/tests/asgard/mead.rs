// SPDX-License-Identifier: MPL-2.0
use super::*;

const MEAD: &str = "demo.item.muse-tonic";
const POET: &str = "rfb.status.poetic-inspiration";

fn remaining(game: &Game) -> u32 {
    game.player
        .statuses
        .iter()
        .find(|status| status.kind_id == POET)
        .unwrap()
        .remaining_ticks
}

#[test]
fn asgard_mead_actual_use_extends_caps_and_resumes_without_stacking_attributes() {
    let mut game = game();
    let before = game.effective_player_attributes();
    let first = artifact(&mut game, "muse-tonic");
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: first.clone(),
            target: None,
        },
    );
    assert!(!game.items.iter().any(|item| item.id == first));
    assert!(
        (1010..=2000).contains(&remaining(&game)),
        "100 + 1d100 source turns after the item action"
    );
    assert!(game.effective_player_attributes().wisdom > before.wisdom);
    assert!(game.effective_player_attributes().charisma > before.charisma);
    let boosted = game.effective_player_attributes();
    let second = artifact(&mut game, "muse-tonic");
    let mut restored = Game::from_save(game.to_save()).unwrap();
    let old = remaining(&game);
    for game in [&mut game, &mut restored] {
        dispatch_next(
            game,
            GameCommand::UseItem {
                item_id: second.clone(),
                target: None,
            },
        );
        assert!((1010..=2000).contains(&(remaining(game) - old)));
        assert_eq!(game.effective_player_attributes(), boosted);
        assert_eq!(
            game.player
                .statuses
                .iter()
                .filter(|status| status.kind_id == POET)
                .count(),
            1
        );
    }
    assert_eq!(game.state_hash(), restored.state_hash());
    game.player
        .statuses
        .iter_mut()
        .find(|status| status.kind_id == POET)
        .unwrap()
        .remaining_ticks = 99_999;
    let third = artifact(&mut game, "muse-tonic");
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: third,
            target: None,
        },
    );
    assert_eq!(remaining(&game), 100_000);
    let mut restored = Game::from_save(game.to_save()).unwrap();
    restored
        .player
        .statuses
        .iter_mut()
        .find(|status| status.kind_id == POET)
        .unwrap()
        .remaining_ticks = 1;
    restored
        .process_status_tick(
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
            false,
        )
        .unwrap();
    assert!(!restored.player_has_status_kind(POET));
    assert_eq!(restored.effective_player_attributes(), before);
}

#[test]
fn asgard_mead_uses_cast_dice_and_ignores_ordinary_device_power() {
    let mut ordinary = game();
    let mut powered = ordinary.clone();
    let mut status = monster_combat::melee_status(STATUS_HASTE, 1000, "test.device-power").status;
    status.granted_modifiers.device_power_bonus = 20;
    powered.player.statuses.push(status);
    assert!(
        powered.effective_player_device_power_bonus()
            > ordinary.effective_player_device_power_bonus()
    );
    for seed in [0, 12, 88] {
        for game in [&mut ordinary, &mut powered] {
            game.player.statuses.retain(|status| status.kind_id != POET);
            game.rng = RfbRng::seeded(seed);
            let mut expected = game.rng.clone();
            let duration = (102 + expected.bounded(100)) as u32 * 10;
            assert!(game.resolve_item_poetic_inspiration(MEAD, 1, 100, 100, &mut Vec::new()));
            assert_eq!(remaining(game), duration);
            assert_eq!(game.rng, expected);
        }
        assert_eq!(remaining(&ordinary), remaining(&powered));
    }
}

#[test]
fn asgard_mead_source_dispel_clears_both_statuses_with_one_resistance_roll() {
    let mut base = game();
    base.resolve_item_speed("demo.item.swiftstep-tonic", 0, 1, 10, &mut Vec::new());
    base.resolve_item_poetic_inspiration(MEAD, 1, 100, 100, &mut Vec::new());
    assert!(base.gain_mutation("rfb.mutation.one-with-magic", &mut Vec::new()));
    let ability = base
        .content
        .ability("rfb-legacy.ability.dispel")
        .unwrap()
        .clone();
    for resisted in [true, false] {
        let mut game = base.clone();
        let seed = (0..1000)
            .find(|seed| (RfbRng::seeded(*seed).bounded(100) < 77) == resisted)
            .unwrap();
        game.rng = RfbRng::seeded(seed);
        let mut expected = game.rng.clone();
        expected.bounded(100);
        let results = game.resolve_monster_player_effects(
            "test.dispel",
            "demo.actor.loki-the-trickster",
            &ability,
            &mut Vec::new(),
            &mut BTreeSet::new(),
        );
        assert_eq!(results.len(), 2);
        assert_eq!(game.rng, expected);
        assert_eq!(game.player_has_status_kind(POET), resisted);
        assert_eq!(game.player_has_status_kind(STATUS_HASTE), resisted);
    }
}

#[test]
fn asgard_mead_source_allocation_reaches_real_factory_only_in_local_dungeon() {
    let pack = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack).unwrap();
    // Keep the production row and category/quality factory; restrict only the
    // candidate pool so the test does not depend on finding a rare lucky seed.
    let mut table = artifact
        .content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .unwrap()
        .clone();
    table.id = "test.loot-table.mead".into();
    table.entries.retain(|entry| entry.item_kind_id == MEAD);
    assert_eq!(table.entries.len(), 1);
    assert_eq!(table.entries[0].min_depth, 75);
    assert_eq!(table.entries[0].weight, 16);
    let entries = table.entries.clone();
    artifact.content.loot_tables.push(table);
    let content = Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content).unwrap(),
    ));
    let mut game = Game::from_content(493, content, DEFAULT_WORLD_ID).unwrap();
    for source in [
        LootSource::FloorRoom {
            room_id: "test.room".into(),
            spawn_id: "test.floor-item".into(),
        },
        LootSource::MonsterDeath {
            actor_id: "test.drop".into(),
        },
        LootSource::ItemUse {
            item_id: "test.create-item".into(),
        },
        LootSource::Chest {
            item_id: "test.chest".into(),
        },
    ] {
        let mut context = LootContext {
            table_id: "test.loot-table.mead".into(),
            floor_id: "demo.floor.asgard-depth-80".into(),
            depth: 80,
            source,
        };
        assert_eq!(
            crate::game::loot::allocation::select_filtered_entry(&mut game, &context, &entries),
            Some(0)
        );
        let draft = (0..500)
            .find_map(|_| {
                game.generate_loot_draft_attempt(&context, ItemGenerationMode::Ordinary)
                    .filter(|draft| draft.kind_id == MEAD)
            })
            .unwrap();
        let item = game
            .commit_generated_item_draft(draft, ItemLocation::Inventory)
            .unwrap();
        assert_eq!(item.kind_id, MEAD);
        // Good/Great source hooks reject Mead even locally; acquirement/chest
        // callers using those modes must not be advertised as a Mead source.
        for mode in [ItemGenerationMode::Good, ItemGenerationMode::Great] {
            for _ in 0..16 {
                assert!(
                    game.generate_loot_draft_attempt(&context, mode)
                        .is_none_or(|draft| draft.kind_id != MEAD)
                );
            }
        }
        for floor in [
            "demo.floor.castle-depth-40",
            "demo.floor.pyramidal-mound-depth-80",
        ] {
            context.floor_id = floor.into();
            assert_eq!(
                crate::game::loot::allocation::select_filtered_entry(&mut game, &context, &entries),
                None
            );
            for _ in 0..16 {
                assert!(
                    game.generate_loot_draft_attempt(&context, ItemGenerationMode::Ordinary)
                        .is_none_or(|draft| draft.kind_id != MEAD)
                );
            }
        }
    }
}
