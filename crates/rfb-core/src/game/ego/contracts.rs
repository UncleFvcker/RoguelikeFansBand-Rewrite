// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::game::Game;
use std::collections::BTreeMap;

#[test]
#[ignore = "exports museum-bound saves for the explicit E8.8 standalone acceptance"]
fn export_ego_desktop_acceptance_save() {
    use crate::game::{
        inventory::ItemIdentificationRequest,
        loot::{ItemGenerationMode, LootContext, LootSource},
    };
    use crate::state::ItemLocation;
    use rfb_protocol::ItemCurseSeverityDto;
    use std::sync::Arc;

    let input = std::path::PathBuf::from(
        std::env::var("E88_DESKTOP_INPUT").expect("real desktop new-game export required"),
    );
    let directory = input.parent().unwrap();
    let (header, payload) = rfb_save::decode(&std::fs::read(&input).unwrap()).unwrap();
    assert!(header.museum_binding.is_some());
    let mut base = Game::from_save(payload).unwrap();
    base.entities.clear();
    base.items.clear();
    let original = base.content.clone();
    let artifact = rfb_content::compile_pack_dir(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original"),
    )
    .unwrap();
    let mut report = Vec::new();
    for (case, kind, mode) in [
        (
            "negative",
            "demo.item.chain-mail",
            ItemGenerationMode::Ordinary,
        ),
        (
            "randart",
            "demo.item.feanorian-lamp",
            ItemGenerationMode::Artifact {
                no_fixed_artifact: true,
            },
        ),
        (
            "dragon",
            "demo.item.dragon-shield",
            ItemGenerationMode::Great,
        ),
        ("bag", "demo.item.fabric-bag", ItemGenerationMode::Great),
    ] {
        let mut game = base.clone();
        let mut narrowed = artifact.clone();
        let table = narrowed
            .content
            .loot_tables
            .iter_mut()
            .find(|table| table.id == "demo.loot-table.base-items")
            .unwrap();
        // Fix only the base allocation; all quality, Ego, dragon and artifact rolls use the shared owner.
        table.kind_selection = None;
        table.entries.retain(|entry| entry.item_kind_id == kind);
        table.entries.truncate(1);
        assert_eq!(table.entries.len(), 1);
        table.entries[0].min_depth = 0;
        table.entries[0].max_depth = 127;
        game.content = Arc::new(ContentCatalog::from_artifact(narrowed));
        let context = LootContext {
            table_id: "demo.loot-table.base-items".into(),
            floor_id: game.current_floor_id.clone(),
            depth: if case == "randart" { 30 } else { 50 },
            source: LootSource::ItemUse {
                item_id: "e88.source".into(),
            },
        };
        let (seed, draft) = (0..5000)
            .find_map(|seed| {
                game.rng = RfbRng::seeded(seed);
                let draft = game.generate_one_loot_draft(&context, mode)?;
                let matches = draft.kind_id == kind
                    && match case {
                        "negative" => {
                            draft
                                .curse
                                .is_some_and(|curse| curse != ItemCurseSeverityDto::Permanent)
                                && !draft.affix_ids.is_empty()
                                && draft.intrinsic_properties.modifiers.speed < 0
                        }
                        "randart" => {
                            draft.artifact_name.is_some()
                                && draft.curse.is_none()
                                && draft.activation.as_ref().is_some_and(|activation| {
                                    activation.profile_id.contains(".detect-")
                                })
                        }
                        "dragon" => {
                            !draft.intrinsic_properties.resistances.is_empty()
                                && !draft.affix_ids.is_empty()
                                && draft.enchantments.to_armor > 0
                                && draft.curse.is_none()
                        }
                        "bag" => {
                            draft.affix_ids == ["rfb-legacy.affix.holding-quiver"]
                                && draft.curse.is_none()
                        }
                        _ => unreachable!(),
                    };
                matches.then_some((seed, draft))
            })
            .unwrap_or_else(|| panic!("no acceptance candidate for {case}"));
        game.content = original.clone();
        let mut item = game
            .commit_generated_item_draft(draft, ItemLocation::Ground(game.player.position))
            .unwrap();
        item.id = format!("e88.{case}");
        let id = item.id.clone();
        game.items.push(item);
        game.debug_add_generated_inventory_item("e88.identify", "demo.item.revelation-scroll", 1)
            .unwrap();
        game.items.last_mut().unwrap().quantity = if case == "bag" { 1 } else { 20 };
        game.identify_item_instance("e88.identify", ItemIdentificationRequest::new(true));
        if case == "bag" {
            // Distinct inscriptions prevent merging; reach the extra bag slots through real pickup.
            for index in 0..35 {
                game.debug_add_generated_inventory_item(
                    &format!("e88.cargo-{index:02}"),
                    "demo.item.ration-of-food",
                    1,
                )
                .unwrap();
                let cargo = game.items.last_mut().unwrap();
                cargo.inscription = Some(format!("cargo-{index:02}"));
                cargo.location = if index < 24 {
                    ItemLocation::Inventory
                } else {
                    ItemLocation::Ground(game.player.position)
                };
            }
        }
        let game = Game::from_save(game.to_save()).unwrap();
        let mut equipped = game.clone();
        equipped
            .items
            .iter_mut()
            .find(|item| item.id == id)
            .unwrap()
            .location = ItemLocation::Inventory;
        assert!(equipped.equip_inventory_item(&id, None).is_some());
        equipped.identify_item_instance(&id, ItemIdentificationRequest::new(true));
        let instance = equipped.items.iter().find(|item| item.id == id).unwrap();
        let details = equipped
            .equipment_dto()
            .into_iter()
            .find(|item| item.id == id)
            .unwrap();
        let player = equipped.snapshot().player;
        let mut plain = equipped.clone();
        let plain_item = plain.items.iter_mut().find(|item| item.id == id).unwrap();
        plain_item.enchantments = Default::default();
        plain_item.intrinsic_properties.modifiers.speed = 0;
        let plain_armor = plain.snapshot().player.armor_class;
        let plain_speed = plain.snapshot().player.speed;
        if case == "negative" {
            assert!(player.speed < plain_speed);
        }
        if case == "dragon" {
            assert!(player.armor_class > plain_armor);
        }
        report.push(serde_json::json!({ "case": case, "seed": seed, "id": id, "details": details,
            "intrinsic": instance.intrinsic_properties, "player": player, "plainArmor": plain_armor, "plainSpeed": plain_speed,
            "initialWeight": game.carried_weight_tenths_pound(), "initialSlots": game.inventory_used_slots() }));
        let bytes = rfb_save::encode(&header, &game.to_save()).unwrap();
        let (_, saved) = rfb_save::decode(&bytes).unwrap();
        assert_eq!(
            Game::from_save(saved).unwrap().state_hash(),
            game.state_hash()
        );
        std::fs::write(directory.join(format!("e88-{case}.rfbsave")), bytes).unwrap();
    }
    std::fs::write(
        directory.join("e88-expectations.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
fn all_160_source_egos_have_an_effect_and_save_stable_instances() {
    let base = Game::new_with_build(7, "demo.build.warrior").unwrap();
    let content = base.content.clone();
    let mut affixes = content
        .affix_definitions()
        .filter(|affix| affix.rfb_ego.is_some())
        .collect::<Vec<_>>();
    affixes.sort_by_key(|affix| affix.rfb_ego.as_ref().unwrap().source_index);
    assert_eq!(affixes.len(), 160);
    assert_eq!(
        affixes
            .iter()
            .map(|affix| affix.rfb_ego.as_ref().unwrap().source_index)
            .collect::<BTreeSet<_>>()
            .len(),
        160
    );
    assert_eq!(
        affixes
            .iter()
            .filter(|affix| affix.rfb_ego.as_ref().unwrap().rarity == 0)
            .map(|affix| affix.rfb_ego.as_ref().unwrap().source_index)
            .collect::<Vec<_>>(),
        [103, 210, 211, 260]
    );
    for affix in affixes {
        let source = affix.rfb_ego.as_ref().unwrap().source_index;
        for level in [20, 80] {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(u64::from(source));
            if source == 260 {
                use crate::game::inventory::{CurseEquippedItemRequest, EquippedItemCurseTarget};
                game.debug_item_curses_land = true;
                game.curse_equipped_item(
                    CurseEquippedItemRequest::new(EquippedItemCurseTarget::Weapon).blasting(),
                );
                assert!(
                    game.items
                        .iter()
                        .any(|item| item.affix_ids == [affix.id.clone()])
                );
            } else {
                let item = content
                    .item_definitions()
                    .find(|item| {
                        item.artifact_generation.is_none()
                            && if (250..=256).contains(&source) {
                                item.id == "demo.item.magic-missile-wand"
                            } else {
                                item.rfb_base_kind.is_some_and(|base| {
                                    rfb_ego_can_apply_to_base(source, base.tval, base.sval, item)
                                })
                            }
                    })
                    .unwrap_or_else(|| panic!("source {source} lacks a compatible formal base"));
                game.debug_add_generated_inventory_item("test.ego.contract", &item.id, level)
                    .unwrap();
                let materialized = materialize_ego_with_rng(
                    false,
                    &content,
                    &mut game.rng,
                    &item.id,
                    vec![affix.id.clone()],
                    |_| level,
                    level,
                    2,
                );
                let empty = EgoMaterialization::new(
                    vec![affix.id.clone()],
                    Vec::new(),
                    None,
                    None,
                    None,
                    None,
                );
                let static_effect = affix.modifiers != Default::default()
                    || affix.equipment_bonuses != Default::default()
                    || !affix.resistances.is_empty()
                    || !affix.status_immunities.is_empty()
                    || !affix.slays.is_empty()
                    || !affix.brands.is_empty()
                    || !affix.passives.is_empty()
                    || !affix.elemental_destruction_immunities.is_empty()
                    || affix.resists_projection_destruction
                    || affix.resists_monster_destruction
                    || affix.protects_quiver_ammunition
                    || affix.ammunition_behavior.is_some();
                assert!(
                    static_effect || materialized != empty || source == 237,
                    "source {source}, level {level}: no runtime effect"
                );
                let target = game.items.last_mut().unwrap();
                materialized.apply_to(target);
                target.quality = rfb_protocol::ItemQualityDto::Exceptional;
                assert_eq!(target.affix_ids.as_slice(), std::slice::from_ref(&affix.id));
                assert!(
                    target
                        .rolled_affixes
                        .iter()
                        .all(|rolled| rolled.affix_id == affix.id)
                );
                if source == 237 {
                    // Duration is consumed directly by the light-fuel owner.
                    let mut probe = game.clone();
                    probe
                        .equip_inventory_item("test.ego.contract", None)
                        .unwrap();
                    let index = probe
                        .items
                        .iter()
                        .position(|item| item.id == "test.ego.contract")
                        .unwrap();
                    probe.items[index].fuel.as_mut().unwrap().current = 10;
                    for tick in [10, 20] {
                        probe.world_tick = tick;
                        probe.process_equipped_light_fuel(&mut Vec::new());
                    }
                    assert_eq!(probe.items[index].fuel.unwrap().current, 9);
                }
            }
            let values = game
                .items
                .iter()
                .map(|item| {
                    let value = crate::game::item_value::obj_value_real(&content, item);
                    if item.affix_ids.contains(&affix.id) && !(250..=256).contains(&source) {
                        assert!(
                            value.is_some(),
                            "source {source}, level {level}: missing real equipment value"
                        );
                    }
                    (item.id.clone(), value)
                })
                .collect::<BTreeMap<_, _>>();
            let restored = Game::from_save(game.to_save())
                .unwrap_or_else(|error| panic!("source {source}, level {level}: {error}"));
            assert_eq!(restored.rng, game.rng, "source {source}, level {level}");
            assert_eq!(
                restored.state_hash(),
                game.state_hash(),
                "source {source}, level {level}"
            );
            let mut actual = restored.items.clone();
            let mut expected = game.items.clone();
            actual.sort_by(|left, right| left.id.cmp(&right.id));
            expected.sort_by(|left, right| left.id.cmp(&right.id));
            assert_eq!(actual, expected, "source {source}, level {level}");
            for item in &restored.items {
                assert_eq!(
                    crate::game::item_value::obj_value_real(&content, item),
                    values[&item.id]
                );
            }
        }
    }
}

#[test]
fn adapted_weapon_bases_and_wizardstaff_use_the_shared_owner() {
    let mut game = Game::new_with_build(7, "demo.build.high-mage-death").unwrap();
    for (kind, source, tval, sval) in [
        ("broad-sword", 59, 23, 16),
        ("sabre", 54, 23, 11),
        ("spear", 86, 22, 2),
        ("diamond-edge", 74, 23, 31),
        ("wizardstaff", 137, 21, 21),
    ] {
        let id = format!("demo.item.{kind}");
        let item = game.content.item(&id).unwrap();
        let base = item.rfb_base_kind.unwrap();
        assert_eq!(
            (base.source_index, base.tval, base.sval),
            (source, tval, sval)
        );
        assert!(
            game.content
                .loot_table("demo.loot-table.base-items")
                .unwrap()
                .entries
                .iter()
                .any(|entry| entry.item_kind_id == id)
        );
    }
    let staff = game.content.item("demo.item.wizardstaff").unwrap();
    assert!(staff.passives.contains(&EquipmentPassive::ReducedManaCost));
    let affix = game.content.affix("rfb-legacy.affix.arcane").unwrap();
    let materialized = materialize_ego_with_rng(
        false,
        &game.content,
        &mut game.rng,
        &staff.id,
        vec![affix.id.clone()],
        |_| 50,
        50,
        2,
    );
    assert_eq!(
        materialized.affix_ids.as_slice(),
        std::slice::from_ref(&affix.id)
    );
    assert_eq!(materialized.rolled_affixes.len(), 1);
    let ability = game
        .content
        .abilities()
        .find(|ability| {
            ability
                .player
                .as_ref()
                .is_some_and(|player| player.resource_cost >= 8)
        })
        .unwrap()
        .clone();
    let progress = game.ability_progress_value(&ability);
    let cost_before = game.ability_effective_resource_cost(&ability, progress);
    game.debug_add_generated_inventory_item("test.ego.wizardstaff", "demo.item.wizardstaff", 50)
        .unwrap();
    game.equip_inventory_item("test.ego.wizardstaff", None)
        .unwrap();
    assert!(
        game.player_equipment_passives()
            .contains(&EquipmentPassive::ReducedManaCost)
    );
    assert_eq!(
        game.ability_effective_resource_cost(&ability, progress),
        (cost_before * 3 / 4).max(1)
    );
}
