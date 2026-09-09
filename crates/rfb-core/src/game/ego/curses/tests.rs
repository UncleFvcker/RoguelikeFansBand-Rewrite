// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::{Game, loot::ItemGenerationMode};
use std::sync::Arc;

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Case {
    kind: String,
    seed: u64,
    level: u16,
    tval: u16,
    good: u8,
    great: u8,
    mode: u32,
    no_egos: bool,
    luck: i8,
    chance: i16,
    value: i32,
    power: u8,
    flags: BTreeSet<String>,
    expected: Expected,
}

#[derive(serde::Deserialize, Default)]
struct Expected {
    power: i16,
    picked: u32,
    curse: u32,
    draws: u64,
    state: [u64; 4],
    flags: BTreeSet<String>,
}

#[derive(serde::Deserialize)]
struct Reference {
    cases: Vec<Case>,
}

fn curse_bit(effect: ItemCurseEffectDto) -> u32 {
    1_u32
        << (CURSE_EFFECTS
            .iter()
            .position(|entry| *entry == Some(effect))
            .unwrap()
            + 4)
}

#[test]
fn negative_generation_matches_1216_independent_c_cases_and_rng_states() {
    let reference: Reference = serde_json::from_str(include_str!("reference.json")).unwrap();
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&path).unwrap();
    let ordinary = Arc::new(ContentCatalog::from_artifact(artifact.clone()));
    for world in &mut artifact.content.worlds {
        world.no_egos = true;
    }
    let no_egos = Arc::new(ContentCatalog::from_artifact(artifact));
    let mut game = Game::new(0);
    for (index, case) in reference.cases.into_iter().enumerate() {
        game.content = if case.no_egos {
            no_egos.clone()
        } else {
            ordinary.clone()
        };
        game.progress.active_mutation_ids.clear();
        if case.luck != 0 {
            game.progress.active_mutation_ids.insert(
                if case.luck < 0 {
                    "rfb.mutation.bad-luck"
                } else {
                    "rfb.mutation.good-luck"
                }
                .to_owned(),
            );
        }
        for virtue in &mut game.virtues {
            virtue.value = 0;
        }
        if let Some(virtue) = game
            .virtues
            .iter_mut()
            .find(|virtue| virtue.kind == rfb_protocol::VirtueKindDto::Chance)
        {
            virtue.value = case.chance;
        } else {
            game.virtues[0] = rfb_protocol::VirtueDto {
                kind: rfb_protocol::VirtueKindDto::Chance,
                value: case.chance,
            };
        }
        game.rng = RfbRng::seeded(case.seed);
        match case.kind.as_str() {
            "power" => {
                let level = game.luck_adjusted_item_generation_depth(case.level, case.tval == 55);
                let mode = match case.mode {
                    0 => ItemGenerationMode::Ordinary,
                    2 => ItemGenerationMode::Good,
                    4 => ItemGenerationMode::GreatOnly,
                    6 => ItemGenerationMode::Great,
                    14 => ItemGenerationMode::Artifact {
                        no_fixed_artifact: false,
                    },
                    _ => unreachable!(),
                };
                let power = game.roll_rfb_depth_loot_power(
                    rfb_content::LootQualityPolicyDefinition::RfbDepth {
                        good_cap_percent: case.good,
                        great_cap_percent: case.great,
                    },
                    level,
                    matches!(case.tval, 40 | 45),
                    matches!(case.tval, 55 | 65 | 66),
                    mode,
                );
                assert_eq!(power, case.expected.power, "power case {index}");
            }
            "curse" => {
                let result = roll_curse(&mut game.rng, case.value, case.tval, &case.flags);
                let bits = result.effects.into_iter().fold(
                    1 | if result.heavy { 2 } else { 0 }
                        | if result.severity == ItemCurseSeverityDto::Permanent {
                            4
                        } else {
                            0
                        },
                    |bits, effect| bits | curse_bit(effect),
                );
                assert_eq!(bits, case.expected.curse, "curse bits case {index}");
                assert_eq!(
                    result.flags, case.expected.flags,
                    "curse flags case {index}"
                );
            }
            "get-curse" => assert_eq!(
                curse_bit(get_curse(&mut game.rng, case.power, case.tval)),
                case.expected.picked,
                "get_curse case {index}"
            ),
            _ => unreachable!(),
        }
        assert_eq!(
            game.rng.draw_counter, case.expected.draws,
            "draws case {index}"
        );
        assert_eq!(
            game.rng.state, case.expected.state,
            "RNG state case {index}"
        );
    }
}

#[test]
fn naturally_generated_negative_equipment_can_be_equipped_uncursed_and_saved_without_revealing_unknown_properties()
 {
    use crate::game::inventory::{ItemIdentificationRequest, RemoveEquippedCursesRequest};
    use crate::game::loot::{LootContext, LootSource};
    use crate::state::ItemLocation;
    use rfb_protocol::{ItemIdentificationDto, ItemQualityDto};
    let mut game = Game::new_with_build(82, "demo.build.warrior").unwrap();
    let original = game.content.clone();
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    let artifact = rfb_content::compile_pack_dir(&path).unwrap();
    for kind in [
        "demo.item.long-sword",
        "demo.item.chain-mail",
        "demo.item.wooden-torch",
        "demo.item.ring",
        "demo.item.amulet",
    ] {
        let mut narrowed = artifact.clone();
        let table = narrowed
            .content
            .loot_tables
            .iter_mut()
            .find(|table| table.id == "demo.loot-table.base-items")
            .unwrap();
        table.entries.retain(|entry| entry.item_kind_id == kind);
        assert_eq!(table.entries.len(), 1, "{kind}");
        table.entries[0].min_depth = 0;
        table.entries[0].max_depth = 127;
        game.content = Arc::new(ContentCatalog::from_artifact(narrowed));
        let context = LootContext {
            table_id: "demo.loot-table.base-items".to_owned(),
            floor_id: game.current_floor_id.clone(),
            depth: 50,
            source: LootSource::ItemUse {
                item_id: "test.natural-equipment".to_owned(),
            },
        };
        let draft = (1..=5000)
            .find_map(|seed| {
                game.rng = RfbRng::seeded(seed);
                let draft = game.generate_one_loot_draft(&context, ItemGenerationMode::Ordinary)?;
                (draft.kind_id == kind
                    && draft.quality == ItemQualityDto::Ordinary
                    && draft
                        .curse
                        .is_some_and(|curse| curse != ItemCurseSeverityDto::Permanent)
                    && !draft.affix_ids.is_empty())
                .then_some(draft)
            })
            .unwrap_or_else(|| panic!("negative Ego for {kind}"));
        game.content = original.clone();
        let item = game
            .commit_generated_item_draft(draft, ItemLocation::Inventory)
            .unwrap();
        let id = item.id.clone();
        let mut properties = item.intrinsic_properties.clone();
        properties.rfb_heavy_curse = false;
        game.items.retain(|item| {
            item.id != id && !matches!(item.location, ItemLocation::Equipped { .. })
        });
        game.items.push(item);
        assert_eq!(
            game.item_identification(game.items.last().unwrap()),
            ItemIdentificationDto::Unexamined
        );
        let projected = game
            .inventory_dto()
            .into_iter()
            .find(|item| item.id == id)
            .unwrap();
        assert!(projected.known_properties.is_empty());
        assert!(projected.enchantments.is_empty());
        assert!(
            game.equip_inventory_item(&id, None).is_some(),
            "equip {kind}"
        );
        assert_eq!(
            game.item_identification(game.items.iter().find(|item| item.id == id).unwrap()),
            ItemIdentificationDto::Identified,
            "the existing equip rule identifies the worn item"
        );
        let restored =
            Game::from_save(game.to_save()).unwrap_or_else(|error| panic!("{kind}: {error:?}"));
        assert_eq!(restored.state_hash(), game.state_hash());
        game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
        game.remove_equipped_curses(RemoveEquippedCursesRequest::new(true));
        let item = game.items.iter().find(|item| item.id == id).unwrap();
        assert_eq!(item.curse, None);
        assert_eq!(
            item.intrinsic_properties, properties,
            "uncursing retains the generated negative flags and pval"
        );
        assert_eq!(
            Game::from_save(game.to_save())
                .unwrap_or_else(|error| panic!("uncursed {kind}: {error:?}"))
                .state_hash(),
            game.state_hash()
        );
    }
}

#[test]
fn negative_ammunition_devices_and_quivers_keep_their_type_specific_results() {
    use crate::game::loot::{LootContext, LootSource};
    use rfb_protocol::ItemQualityDto;
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    let artifact = rfb_content::compile_pack_dir(&path).unwrap();
    let mut game = Game::new_with_build(82, "demo.build.warrior").unwrap();
    for kind in [
        "demo.item.sheaf-arrow",
        "demo.item.detect-objects-staff",
        "demo.item.quiver",
    ] {
        let mut narrowed = artifact.clone();
        let table = narrowed
            .content
            .loot_tables
            .iter_mut()
            .find(|table| table.id == "demo.loot-table.base-items")
            .unwrap();
        table.entries.retain(|entry| entry.item_kind_id == kind);
        assert!(!table.entries.is_empty(), "{kind}");
        game.content = Arc::new(ContentCatalog::from_artifact(narrowed));
        let context = LootContext {
            table_id: "demo.loot-table.base-items".into(),
            floor_id: game.current_floor_id.clone(),
            depth: 50,
            source: LootSource::ItemUse {
                item_id: "test.negative-types".into(),
            },
        };
        let mut observed = false;
        for seed in 0..500 {
            game.rng = RfbRng::seeded(seed);
            let Some(item) = game.generate_one_loot_draft(&context, ItemGenerationMode::Ordinary)
            else {
                continue;
            };
            if item.kind_id != kind || item.quality != ItemQualityDto::Ordinary {
                continue;
            }
            match kind {
                "demo.item.sheaf-arrow"
                    if item.enchantments.to_hit > 0 && item.enchantments.to_damage > 0 =>
                {
                    // Positive enchantments with Ordinary quality identify the -2 branch.
                    assert!(item.affix_ids.is_empty());
                    assert_eq!(item.curse, None);
                    observed = true;
                }
                "demo.item.detect-objects-staff" if !item.affix_ids.is_empty() => {
                    // Ordinary device Ego requires negative exceptional power.
                    assert_eq!(item.curse, None);
                    assert!(item.activation.is_some() && item.charges.is_some());
                    observed = true;
                }
                "demo.item.quiver" => {
                    assert!(item.affix_ids.is_empty());
                    assert_eq!(item.curse, None);
                    assert!(item.intrinsic_properties.ammunition_capacity.is_some());
                    observed = true;
                }
                _ => {}
            }
        }
        assert!(observed, "{kind}");
    }
}
