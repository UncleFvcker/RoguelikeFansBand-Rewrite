// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::{
    game::{
        Game,
        inventory::PickUpOutcome,
        loot::{ItemGenerationMode, LootContext, LootSource},
    },
    resistance::DamageType,
    state::ItemLocation,
};
use rfb_protocol::ItemQualityDto;
use std::{collections::BTreeMap, sync::Arc};

const BAGS: [&str; 3] = ["fabric-bag", "leather-pouch", "dwarven-backpack"];

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Case {
    source_index: u32,
    power: i16,
    level: u16,
    seed: u64,
    forced_ego: u32,
    expected: Expected,
}

#[derive(serde::Deserialize)]
struct Expected {
    capacity: u16,
    weight: u16,
    ego: u32,
    draws: u64,
    state: [u64; 4],
    value: i32,
}

#[test]
fn containers_match_972_original_c_capacity_ego_rng_and_value_results() {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("bag-reference.json")).unwrap();
    let cases: Vec<Case> = serde_json::from_value(reference["cases"].clone()).unwrap();
    assert_eq!(cases.len(), 972);
    let mut game = blank();
    let mut seen = BTreeSet::new();
    for (number, case) in cases.into_iter().enumerate() {
        let definition = game
            .content
            .item_definitions()
            .find(|item| {
                item.rfb_base_kind
                    .is_some_and(|base| base.source_index == case.source_index)
            })
            .unwrap()
            .clone();
        game.items.clear();
        add(&mut game, "test.container", &definition.id, 1);
        let mut item = game.items[0].clone();
        let mut rng = RfbRng::seeded(case.seed);
        item.intrinsic_properties =
            roll_container_capacity(&mut rng, &definition, case.power).unwrap();
        if case.power > 1 {
            let result = if case.forced_ego == 0 {
                roll_and_materialize_rfb_ego_from_affixes_with_rng(
                    false,
                    Default::default(),
                    &mut rng,
                    &definition,
                    game.content.affix_definitions(),
                    case.level,
                    Some(&item.intrinsic_properties),
                )
                .unwrap()
            } else {
                let affix = game
                    .content
                    .affix_definitions()
                    .find(|affix| {
                        affix
                            .rfb_ego
                            .as_ref()
                            .is_some_and(|ego| ego.source_index == case.forced_ego)
                    })
                    .unwrap();
                materialize_ego_with_rng(
                    false,
                    &game.content,
                    &mut rng,
                    &definition.id,
                    vec![affix.id.clone()],
                    |_| case.level,
                    case.level,
                    case.power,
                )
            };
            result.apply_to(&mut item);
        }
        let ego = item.affix_ids.first().map_or(0, |id| {
            game.content
                .affix(id)
                .unwrap()
                .rfb_ego
                .as_ref()
                .unwrap()
                .source_index
        });
        assert_eq!(ego, case.expected.ego, "ego case {number}");
        assert_eq!(
            item.intrinsic_properties
                .bag_capacity
                .or(item.intrinsic_properties.ammunition_capacity),
            Some(case.expected.capacity),
            "capacity case {number}"
        );
        assert_eq!(
            game.item_instance_weight(&item),
            case.expected.weight,
            "weight case {number}"
        );
        assert_eq!(rng.draw_counter, case.expected.draws, "draws case {number}");
        assert_eq!(rng.state, case.expected.state, "RNG case {number}");
        assert_eq!(
            crate::game::item_value::obj_value_real(&game.content, &item),
            Some(case.expected.value),
            "value case {number}"
        );
        if case.source_index != 721 && ego != 0 {
            seen.insert((case.source_index, ego));
        }
    }
    for index in 722..=724 {
        for ego in 265..=268 {
            assert!(seen.contains(&(index, ego)));
        }
    }
}

#[test]
fn bag_base_identity_pval_and_chinese_names_match_master() {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("bag-reference.json")).unwrap();
    let game = blank();
    let zh = include_str!("../../../../../../locales/zh-CN/content.ftl");
    for source in reference["bases"].as_array().unwrap().iter().take(3) {
        let index = source["sourceIndex"].as_u64().unwrap() as u32;
        let item = game
            .content
            .item_definitions()
            .find(|item| {
                item.rfb_base_kind
                    .is_some_and(|base| base.source_index == index)
            })
            .unwrap();
        let base = item.rfb_base_kind.unwrap();
        assert_eq!((base.tval, base.sval), (46, 1));
        assert_eq!(
            i64::from(item.rfb_value.as_ref().unwrap().pval),
            source["identity"][2].as_i64().unwrap()
        );
        assert_eq!(
            item.generation_level.to_string(),
            source["allocation"][0].as_str().unwrap()
        );
        assert_eq!(
            item.weight_tenths_pound.to_string(),
            source["allocation"][3].as_str().unwrap()
        );
        assert_eq!(
            item.base_value.to_string(),
            source["allocation"][4].as_str().unwrap()
        );
        let chinese = source["chineseName"]
            .as_str()
            .unwrap()
            .split_once('~')
            .unwrap()
            .1
            .replace('~', "");
        assert!(
            zh.lines()
                .any(|line| line == format!("{} = {chinese}", item.name_key))
        );
    }
}

fn blank() -> Game {
    let mut game = Game::new_with_build(84, "demo.build.warrior").unwrap();
    game.items.clear();
    game.entities.clear();
    game
}

fn add(game: &mut Game, id: &str, kind: &str, quantity: u32) {
    game.debug_add_generated_inventory_item(id, kind, 50)
        .unwrap();
    game.items.last_mut().unwrap().quantity = quantity;
}

fn equip_bag(game: &mut Game, suffix: &str, ego: Option<&str>) {
    let kind = format!("demo.item.{suffix}");
    add(game, "test.bag", &kind, 1);
    if let Some(ego) = ego {
        let result = materialize_ego_with_rng(
            false,
            &game.content,
            &mut game.rng,
            &kind,
            vec![format!("rfb-legacy.affix.{ego}")],
            |_| 50,
            50,
            2,
        );
        result.apply_to(game.items.last_mut().unwrap());
    }
    assert!(game.equip_inventory_item("test.bag", None).is_some());
}

#[test]
fn forced_base_bags_cover_ordinary_good_great_all_egos_known_capacity_and_save() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    let artifact = rfb_content::compile_pack_dir(&path).unwrap();
    for suffix in BAGS {
        let kind = format!("demo.item.{suffix}");
        for (mode, quality) in [
            (ItemGenerationMode::Ordinary, ItemQualityDto::Ordinary),
            (ItemGenerationMode::Good, ItemQualityDto::Fine),
            (ItemGenerationMode::Great, ItemQualityDto::Exceptional),
        ] {
            let mut game = blank();
            let original = game.content.clone();
            let base = base_bag_capacity(game.content.item(&kind).unwrap()).unwrap();
            let mut narrowed = artifact.clone();
            let table = narrowed
                .content
                .loot_tables
                .iter_mut()
                .find(|table| table.id == "demo.loot-table.base-items")
                .unwrap();
            // This test fixes the base kind and exercises materialization.
            table.kind_selection = None;
            table.entries.retain(|entry| entry.item_kind_id == kind);
            assert_eq!(table.entries.len(), 1);
            let context = LootContext {
                table_id: table.id.clone(),
                floor_id: game.current_floor_id.clone(),
                depth: table.entries[0].min_depth,
                source: LootSource::ItemUse {
                    item_id: "test.bag-generation".into(),
                },
            };
            game.content = Arc::new(ContentCatalog::from_artifact(narrowed));
            let mut drafts = BTreeMap::new();
            for seed in 0..1000 {
                game.rng = RfbRng::seeded(seed);
                let draft = game.generate_one_loot_draft(&context, mode).unwrap();
                if draft.kind_id == kind && draft.quality == quality {
                    drafts.entry(draft.affix_ids.clone()).or_insert(draft);
                }
                if drafts.len()
                    == if mode == ItemGenerationMode::Great {
                        4
                    } else {
                        1
                    }
                {
                    break;
                }
            }
            assert_eq!(
                drafts.len(),
                if mode == ItemGenerationMode::Great {
                    4
                } else {
                    1
                },
                "{suffix} {mode:?}"
            );
            game.content = original;
            for draft in drafts.into_values() {
                let holding = draft
                    .affix_ids
                    .iter()
                    .any(|id| id == "rfb-legacy.affix.holding-quiver");
                let expected = if holding {
                    base * 2
                } else if quality == ItemQualityDto::Fine {
                    base + 2
                } else {
                    base
                };
                assert_eq!(draft.intrinsic_properties.bag_capacity, Some(expected));
                assert_eq!(draft.intrinsic_properties.ammunition_capacity, None);
                game.items.clear();
                let item = game
                    .commit_generated_item_draft(draft, ItemLocation::Inventory)
                    .unwrap();
                let id = item.id.clone();
                game.items.push(item);
                assert_eq!(game.inventory_dto()[0].bag_capacity, None);
                assert!(game.equip_inventory_item(&id, None).is_some());
                assert_eq!(game.equipment_dto()[0].bag_capacity, Some(expected));
                assert_eq!(game.inventory_slot_capacity(), 26 + expected);
                assert_eq!(
                    game.equipment_dto()[0].known_properties.len(),
                    usize::from(quality == ItemQualityDto::Exceptional)
                );
                let restored = Game::from_save(game.to_save()).unwrap();
                assert_eq!(restored.rng, game.rng);
                assert_eq!(restored.state_hash(), game.state_hash());
                assert_eq!(restored.equipment_dto()[0].bag_capacity, Some(expected));
                assert!(game.unequip_slot("container").is_some());
                assert_eq!(game.inventory_slot_capacity(), 26);
                assert_eq!(game.inventory_dto()[0].bag_capacity, Some(expected));
            }
        }
    }
}

#[test]
fn bag_slots_exclude_ammunition_and_full_bags_still_accept_compatible_stacks() {
    let mut game = blank();
    equip_bag(&mut game, "fabric-bag", None);
    for index in 0..26 {
        add(
            &mut game,
            &format!("test.ammo-{index}"),
            "demo.item.arrow",
            1,
        );
        game.items.last_mut().unwrap().inscription = Some(index.to_string());
    }
    add(&mut game, "test.blocked-ammo", "demo.item.arrow", 1);
    game.items.last_mut().unwrap().location = ItemLocation::Ground(game.player.position);
    let incoming = game.items.last().unwrap().clone();
    assert_eq!(game.inventory_used_slots(), 26);
    assert_eq!(game.inventory_slot_capacity(), 30);
    assert_eq!(game.inventory_quantity_capacity_for(&incoming, true), 0);
    let before = game.items.clone();
    assert!(matches!(
        game.pick_up_item_at_player(Some(&incoming.id)).unwrap(),
        PickUpOutcome::InventoryFull { .. }
    ));
    assert_eq!(game.items, before);
    let mut overloaded = game.clone();
    overloaded
        .items
        .iter_mut()
        .find(|item| item.id == incoming.id)
        .unwrap()
        .location = ItemLocation::Inventory;
    assert!(matches!(
        Game::from_save(overloaded.to_save()),
        Err(crate::error::CoreError::InvalidSave(
            "inventory exceeds slot capacity"
        ))
    ));
    for index in 0..4 {
        add(
            &mut game,
            &format!("test.food-{index}"),
            "demo.item.ration-of-food",
            1,
        );
        let item = game.items.last_mut().unwrap();
        item.inscription = Some(format!("food-{index}"));
        item.location = ItemLocation::Ground(game.player.position);
        let id = item.id.clone();
        assert!(matches!(
            game.pick_up_item_at_player(Some(&id)).unwrap(),
            PickUpOutcome::Picked { .. }
        ));
    }
    assert_eq!(game.inventory_used_slots(), 30);
    add(&mut game, "test.merge", "demo.item.ration-of-food", 3);
    let incoming = game.items.last_mut().unwrap();
    incoming.inscription = Some("food-0".into());
    incoming.location = ItemLocation::Ground(game.player.position);
    let incoming = incoming.clone();
    assert!(game.inventory_quantity_capacity_for(&incoming, true) >= 3);
    assert!(matches!(
        game.pick_up_item_at_player(Some(&incoming.id)).unwrap(),
        PickUpOutcome::Picked { quantity: 3, .. }
    ));
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.food-0")
            .unwrap()
            .quantity,
        4
    );
    assert_eq!(game.inventory_used_slots(), 30);
    add(&mut game, "test.overflow", "demo.item.dagger", 1);
    game.items.last_mut().unwrap().location = ItemLocation::Ground(game.player.position);
    let before = game.items.clone();
    assert!(matches!(
        game.pick_up_item_at_player(Some("test.overflow")).unwrap(),
        PickUpOutcome::InventoryFull { .. }
    ));
    assert_eq!(game.items, before);
    game.reveal_current_visibility();
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn smaller_bag_or_removal_rejects_overflow_without_losing_items() {
    let mut game = blank();
    equip_bag(&mut game, "dwarven-backpack", Some("holding-quiver"));
    add(&mut game, "test.small", "demo.item.fabric-bag", 1);
    for index in 0..31 {
        add(
            &mut game,
            &format!("test.supply-{index}"),
            "demo.item.dagger",
            1,
        );
    }
    assert_eq!(game.inventory_slot_capacity(), 50);
    let before = game.items.clone();
    let rng = game.rng.clone();
    assert!(game.equip_inventory_item("test.small", None).is_none());
    assert!(game.unequip_slot("container").is_none());
    assert_eq!(game.items, before);
    assert_eq!(game.rng, rng);
    assert_eq!(
        Game::from_save(game.to_save()).unwrap().state_hash(),
        game.state_hash()
    );
    game.items
        .retain(|item| !item.id.starts_with("test.supply-"));
    assert!(game.equip_inventory_item("test.small", None).is_some());
    assert_eq!(game.inventory_slot_capacity(), 30);
    assert!(game.unequip_slot("container").is_some());
    assert_eq!(game.items.len(), 2);
}

#[test]
fn phase_bag_removes_only_its_own_weight_even_with_a_separate_quiver() {
    let mut game = blank();
    add(&mut game, "test.quiver", "demo.item.quiver", 1);
    assert!(game.equip_inventory_item("test.quiver", None).is_some());
    add(&mut game, "test.ammo", "demo.item.arrow", 60);
    add(&mut game, "test.food", "demo.item.ration-of-food", 10);
    let before = game.carried_weight_tenths_pound();
    equip_bag(&mut game, "fabric-bag", Some("phase-quiver"));
    assert_eq!(game.carried_weight_tenths_pound(), before);
    assert_eq!(game.item_instance_weight(game.items.last().unwrap()), 0);
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(
        restored.carried_weight_tenths_pound(),
        game.carried_weight_tenths_pound()
    );
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn protection_bag_does_not_protect_contents_or_a_separate_quiver() {
    let mut game = blank();
    equip_bag(&mut game, "fabric-bag", Some("quiver-protection"));
    add(&mut game, "test.scrolls", "demo.item.crafting-scroll", 99);
    add(&mut game, "test.arrows", "demo.item.arrow", 60);
    add(&mut game, "test.quiver", "demo.item.quiver", 1);
    assert!(game.equip_inventory_item("test.quiver", None).is_some());
    let mut plain = game.clone();
    plain
        .items
        .iter_mut()
        .find(|item| item.id == "test.bag")
        .unwrap()
        .affix_ids
        .clear();
    for target in [&mut game, &mut plain] {
        target.rng = RfbRng::seeded(0);
        target.damage_player_inventory("test.fire", DamageType::Fire, false, 100, &mut Vec::new());
    }
    for id in ["test.scrolls", "test.arrows"] {
        let quantity = |game: &Game| {
            game.items
                .iter()
                .find(|item| item.id == id)
                .map_or(0, |item| item.quantity)
        };
        assert_eq!(quantity(&game), quantity(&plain));
    }
    assert!(
        game.items
            .iter()
            .find(|item| item.id == "test.scrolls")
            .unwrap()
            .quantity
            < 99
    );
    assert_eq!(game.rng, plain.rng);
}

#[test]
fn endless_bag_activation_refills_only_an_equipped_quiver() {
    for has_quiver in [false, true] {
        let mut game = blank();
        equip_bag(&mut game, "fabric-bag", Some("endless-quiver"));
        if has_quiver {
            add(&mut game, "test.quiver", "demo.item.quiver", 1);
            assert!(game.equip_inventory_item("test.quiver", None).is_some());
        }
        for _ in 0..100 {
            game.use_inventory_item(
                "test.bag",
                None,
                None,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
            if game.items[0].charges.as_ref().unwrap().current == 0 {
                break;
            }
        }
        assert_eq!(game.items[0].charges.as_ref().unwrap().current, 0);
        let arrows: u32 = game
            .items
            .iter()
            .filter(|item| item.kind_id == "demo.item.arrow")
            .map(|item| item.quantity)
            .sum();
        assert_eq!(arrows, if has_quiver { 50 } else { 0 });
        assert_eq!(game.items[0].intrinsic_properties.bag_capacity, Some(4));
        assert_eq!(game.items[0].intrinsic_properties.ammunition_capacity, None);
        assert_eq!(
            Game::from_save(game.to_save()).unwrap().state_hash(),
            game.state_hash()
        );
    }
}
