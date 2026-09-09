// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::game::Game;

#[test]
#[ignore = "exports the real save used by the explicit E8 desktop acceptance pass"]
fn export_ego_desktop_acceptance_save() {
    use crate::game::inventory::ItemIdentificationRequest;
    use crate::state::ItemLocation;
    use rfb_protocol::{
        CharacterSummary, PROTOCOL_VERSION, SAVE_HEADER_SCHEMA_VERSION, SaveHeaderV1,
    };

    let mut game = Game::new_with_build(808, "demo.build.high-mage-death").unwrap();
    game.entities.clear();
    game.items
        .retain(|item| !matches!(item.location, ItemLocation::CarriedBy { .. }));
    for (id, kind, affix) in [
        (
            "e8.light",
            "demo.item.brass-lantern",
            Some("rfb-legacy.affix.illumination-light"),
        ),
        (
            "e8.ring",
            "demo.item.ring",
            Some("rfb-legacy.affix.combat-ring"),
        ),
        (
            "e8.quiver",
            "demo.item.quiver",
            Some("rfb-legacy.affix.endless-quiver"),
        ),
        ("e8.dagger", "demo.item.dagger", None),
        ("e8.identify", "demo.item.revelation-scroll", None),
        ("e8.craft", "demo.item.crafting-scroll", None),
    ] {
        game.debug_add_generated_inventory_item(id, kind, 20)
            .unwrap();
        if let Some(affix) = affix {
            let materialized = materialize_ego_with_rng(
                &game.content,
                &mut game.rng,
                kind,
                vec![affix.to_owned()],
                |_| 20,
                20,
            );
            let item = game.items.last_mut().unwrap();
            materialized.apply_to(item);
            item.quality = rfb_protocol::ItemQualityDto::Exceptional;
            item.location = ItemLocation::Ground(game.player.position);
        } else {
            game.identify_item_instance(id, ItemIdentificationRequest::new(true));
            if id == "e8.identify" {
                game.items.last_mut().unwrap().quantity = 3;
            }
        }
    }
    // Rebuild derived visibility after preparing ground items and clearing actors.
    let game = Game::from_save(game.to_save()).unwrap();
    let snapshot = game.snapshot();
    let header = SaveHeaderV1 {
        format: "rfb-save".into(),
        save_schema_version: SAVE_HEADER_SCHEMA_VERSION,
        game_version: env!("CARGO_PKG_VERSION").into(),
        protocol_version: PROTOCOL_VERSION.into(),
        slot_name: "E8 desktop fixture".into(),
        created_at: "2026-09-09T00:00:00Z".into(),
        saved_at: "2026-09-09T00:00:00Z".into(),
        character_summary: CharacterSummary {
            display_name: snapshot.player.name,
            level: snapshot.player.progress.level.into(),
            location_key: game.location_key().into(),
            turn: snapshot.turn,
        },
        content_id: snapshot.content_id,
        content_hash: snapshot.content_hash,
        payload_encoding: "messagepack".into(),
    };
    let bytes = rfb_save::encode(&header, &game.to_save()).unwrap();
    let (_, payload) = rfb_save::decode(&bytes).unwrap();
    assert_eq!(
        Game::from_save(payload).unwrap().state_hash(),
        game.state_hash()
    );
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-results");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("ego-desktop.rfbsave"), bytes).unwrap();
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
                    &content,
                    &mut game.rng,
                    &item.id,
                    vec![affix.id.clone()],
                    |_| level,
                    level,
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
        &game.content,
        &mut game.rng,
        &staff.id,
        vec![affix.id.clone()],
        |_| 50,
        50,
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
