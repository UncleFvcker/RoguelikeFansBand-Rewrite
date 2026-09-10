// SPDX-License-Identifier: MPL-2.0

use super::*;

#[test]
fn cost_real_matches_independent_rfb_c_reference() {
    #[derive(serde::Deserialize)]
    struct Case {
        name: String,
        object: ValueObject,
        expected: i32,
    }
    #[derive(serde::Deserialize)]
    struct Reference {
        cases: Vec<Case>,
    }
    let reference: Reference = serde_json::from_str(include_str!("reference.json")).unwrap();
    let mut failures = Vec::new();
    for case in reference.cases {
        let actual = object_value(case.object);
        if actual != Some(case.expected) {
            failures.push(format!(
                "{}: Rust {actual:?}, C {}",
                case.name, case.expected
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn unsupported_types_are_not_scored_as_plain_equipment() {
    for tval in [0, 55, 65, 66, 70, 80] {
        assert_eq!(
            object_value(ValueObject {
                tval,
                ..Default::default()
            }),
            None
        );
    }
}

#[test]
fn actual_base_and_fixed_artifact_instances_match_source_costs() {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Case {
        kind_id: String,
        object: ValueObject,
        expected: i32,
    }
    #[derive(serde::Deserialize)]
    struct Reference {
        instances: Vec<Case>,
    }
    let reference: Reference = serde_json::from_str(include_str!("source-instances.json")).unwrap();
    let mut game = crate::game::Game::new(81);
    let mut failures = Vec::new();
    for case in reference.instances {
        game.debug_add_generated_inventory_item("test.value", &case.kind_id, 80)
            .unwrap();
        let item = game.items.pop().unwrap();
        let before = item.clone();
        let rng = game.rng.clone();
        let actual = obj_value_real(&game.content, &item);
        if actual != Some(case.expected) {
            failures.push(format!(
                "{}: Rust {actual:?}, C {}\n  actual {:?}\n  source {:?}",
                case.kind_id,
                case.expected,
                instance::value_object(&game.content, &item),
                case.object
            ));
        }
        assert_eq!(item, before);
        assert_eq!(game.rng, rng);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn real_value_ignores_knowledge_and_round_trips_without_rng() {
    use crate::game::{Game, ego::materialize_ego_with_rng, inventory::ItemIdentificationRequest};
    let mut game = Game::new(81);
    game.debug_add_generated_inventory_item("test.value", "demo.item.ring", 80)
        .unwrap();
    let materialized = materialize_ego_with_rng(
        false,
        &game.content,
        &mut game.rng,
        "demo.item.ring",
        vec!["rfb-legacy.affix.combat-ring".to_owned()],
        |_| 80,
        80,
        2,
    );
    materialized.apply_to(game.items.last_mut().unwrap());
    game.items.last_mut().unwrap().quality = rfb_protocol::ItemQualityDto::Exceptional;
    let item = game.items.last().unwrap().clone();
    let rng = game.rng.clone();
    let hash = game.state_hash();
    let value = obj_value_real(&game.content, &item).unwrap();
    assert_eq!(hash, game.state_hash());
    assert_eq!(rng, game.rng);
    game.identify_item_instance("test.value", ItemIdentificationRequest::new(true));
    assert_eq!(
        obj_value_real(&game.content, game.items.last().unwrap()),
        Some(value)
    );
    assert_eq!(item, *game.items.last().unwrap());
    let restored = Game::from_save(game.to_save()).unwrap();
    let saved_item = restored
        .items
        .iter()
        .find(|item| item.id == "test.value")
        .unwrap();
    assert_eq!(obj_value_real(&restored.content, saved_item), Some(value));
    assert_eq!(rng, restored.rng);
    assert_eq!(game.state_hash(), restored.state_hash());
}

#[test]
fn object_flags_and_random_curse_effects_keep_distinct_real_values() {
    let mut game = crate::game::Game::new(81);
    game.debug_add_generated_inventory_item("test.value", "demo.item.ring", 80)
        .unwrap();
    let materialized = crate::game::ego::materialize_ego_with_rng(
        false,
        &game.content,
        &mut game.rng,
        "demo.item.ring",
        vec!["rfb-legacy.affix.combat-ring".to_owned()],
        |_| 80,
        80,
        2,
    );
    materialized.apply_to(game.items.last_mut().unwrap());
    let mut item = game.items.pop().unwrap();
    let ordinary = obj_value_real(&game.content, &item).unwrap();
    item.rolled_affixes[0]
        .curse_effects
        .insert(rfb_protocol::ItemCurseEffectDto::Aggravate);
    assert_eq!(obj_value_real(&game.content, &item), Some(ordinary));
    item.intrinsic_properties
        .rfb_flags
        .insert("AGGRAVATE".to_owned());
    assert!(obj_value_real(&game.content, &item).unwrap() < ordinary);
}

#[test]
fn negative_one_subtracts_but_negative_two_keeps_positive_enchantment_rolls() {
    use crate::game::ego::{roll_rfb_armor_enchantment, roll_rfb_weapon_enchantment};
    let game = crate::game::Game::new(81);
    let item = game.content.item("demo.item.long-sword").unwrap();
    for power in [1, 2] {
        let mut positive = game.rng.clone();
        let mut negative = positive.clone();
        let good = roll_rfb_weapon_enchantment(&mut positive, item, 80, power).unwrap();
        let bad = roll_rfb_weapon_enchantment(&mut negative, item, 80, -power).unwrap();
        let sign = if power == 1 { -1 } else { 1 };
        assert_eq!(
            (bad.to_hit, bad.to_damage),
            (sign * good.to_hit, sign * good.to_damage)
        );
        assert_eq!(positive, negative);
        let good = roll_rfb_armor_enchantment(&mut positive, 80, power);
        let bad = roll_rfb_armor_enchantment(&mut negative, 80, -power);
        assert_eq!(bad, sign * good);
        assert_eq!(positive, negative);
    }
}
