// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::{
    Game,
    loot::{GeneratedItemDraft, ItemGenerationMode, LootContext, LootSource},
};
use crate::state::ItemLocation;
use rfb_protocol::{GameCommand, GameCommandEnvelope, GameUpdate, ItemQualityDto, TargetSelection};
use std::sync::Arc;

const KINDS: [&str; 6] = [
    "dragon-fang",
    "dragon-wings",
    "dragon-boots",
    "dragon-helm",
    "dragon-gloves",
    "dragon-shield",
];

#[derive(serde::Deserialize)]
struct Case {
    seed: u64,
    tval: u16,
    sval: u16,
    power: i16,
    mode: u16,
    expected: Expected,
}
#[derive(serde::Deserialize)]
struct Expected {
    power: i16,
    draws: u64,
    state: [u64; 4],
    flags: BTreeSet<String>,
}
#[derive(serde::Deserialize)]
struct Reference {
    cases: Vec<Case>,
}

#[test]
fn six_dragon_bases_match_authoritative_kind_data_and_chinese_names() {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("reference.json")).unwrap();
    let game = Game::new_with_build(83, "demo.build.warrior").unwrap();
    let en = include_str!("../../../../../../locales/en-US/content.ftl");
    let zh = include_str!("../../../../../../locales/zh-CN/content.ftl");
    for source in reference["bases"].as_array().unwrap() {
        let index = source["sourceIndex"].as_u64().unwrap() as u32;
        let item = game
            .content
            .item_definitions()
            .find(|item| {
                item.rfb_base_kind
                    .is_some_and(|base| base.source_index == index)
            })
            .unwrap();
        let lines = source["lines"].as_array().unwrap();
        let field = |prefix: &str| {
            lines
                .iter()
                .filter_map(|line| line.as_str().unwrap().strip_prefix(prefix))
                .collect::<Vec<_>>()
                .join("|")
        };
        let numbers = |prefix: &str| {
            field(prefix)
                .split(':')
                .map(|number| number.parse::<i32>().unwrap())
                .collect::<Vec<_>>()
        };
        let identity = numbers("I:");
        let weights = numbers("W:");
        let base = item.rfb_base_kind.unwrap();
        assert_eq!(
            (base.tval, base.sval),
            (identity[0] as u16, identity[1] as u16)
        );
        assert_eq!(i32::from(item.generation_level), weights[0]);
        assert_eq!(i32::from(item.weight_tenths_pound), weights[3]);
        assert_eq!(item.base_value as i32, weights[4]);
        let raw = item.rfb_value.as_ref().unwrap();
        assert_eq!(i32::from(raw.pval), identity[2]);
        assert_eq!(
            raw.flags,
            field("F:")
                .split('|')
                .map(|flag| flag.trim().to_owned())
                .collect()
        );
        let combat = field("P:");
        let combat = combat.split(':').collect::<Vec<_>>();
        assert_eq!(i32::from(raw.to_armor), combat[4].parse::<i32>().unwrap());
        if let Some(melee) = &item.melee_profile {
            assert_eq!(
                format!("{}d{}", melee.damage_dice, melee.damage_sides),
                combat[1]
            );
            assert_eq!(melee.to_hit, combat[2].parse::<i32>().unwrap());
            assert_eq!(melee.to_damage, combat[3].parse::<i32>().unwrap());
        } else {
            assert_eq!(
                item.modifiers.defense,
                combat[0].parse::<i32>().unwrap() + i32::from(raw.to_armor)
            );
        }
        assert!(item.mogaminator_rare);
        assert_eq!(item.elemental_destruction_immunities.len(), 4);
        assert_eq!(
            item.passives.contains(&EquipmentPassive::Levitation),
            base.tval == 35
        );
        let allocation = field("A:")
            .split('/')
            .map(|number| number.parse::<u32>().unwrap())
            .collect::<Vec<_>>();
        let entry = game
            .content
            .loot_table("demo.loot-table.base-items")
            .unwrap()
            .entries
            .iter()
            .find(|entry| entry.item_kind_id == item.id)
            .unwrap();
        assert_eq!(
            (u32::from(entry.min_depth), entry.weight, entry.quantity),
            (allocation[0], 100 / allocation[1], 1)
        );
        let english = source["name"]
            .as_str()
            .unwrap()
            .trim_start_matches("& ")
            .replace('~', "");
        let chinese = source["chineseName"]
            .as_str()
            .unwrap()
            .split_once('~')
            .unwrap()
            .1;
        assert!(
            en.lines()
                .any(|line| line == format!("{} = {english}", item.name_key))
        );
        assert!(
            zh.lines()
                .any(|line| line == format!("{} = {chinese}", item.name_key))
        );
    }
}

#[test]
fn dragon_base_matches_2048_independent_c_results_and_rng_states() {
    let reference: Reference = serde_json::from_str(include_str!("reference.json")).unwrap();
    assert_eq!(reference.cases.len(), 2048);
    let game = Game::new_with_build(83, "demo.build.warrior").unwrap();
    let mut observed = BTreeSet::new();
    for (index, case) in reference.cases.into_iter().enumerate() {
        let definition = game
            .content
            .item_definitions()
            .find(|item| {
                item.artifact_generation.is_none()
                    && item
                        .rfb_base_kind
                        .is_some_and(|base| (base.tval, base.sval) == (case.tval, case.sval))
            })
            .unwrap();
        let mut rng = RfbRng::seeded(case.seed);
        let mut power = case.power;
        let properties = materialize(&mut rng, definition, &mut power, case.mode == 9);
        let flags = properties.map_or_else(BTreeSet::new, |properties| {
            let mut item = game.items[0].clone();
            item.kind_id = definition.id.clone();
            item.affix_ids.clear();
            item.rolled_affixes.clear();
            item.intrinsic_properties = properties;
            crate::game::item_value::instance::value_object(&game.content, &item)
                .unwrap()
                .flags
                .into_iter()
                .filter(|flag| flag.starts_with("RES_") || flag.starts_with("BRAND_"))
                .collect()
        });
        assert_eq!(flags, case.expected.flags, "flags case {index}");
        assert_eq!(power, case.expected.power, "power case {index}");
        assert_eq!(rng.draw_counter, case.expected.draws, "draws case {index}");
        assert_eq!(rng.state, case.expected.state, "state case {index}");
        observed.extend(flags);
    }
    assert_eq!(observed.len(), 21, "five brands and sixteen resistances");
}

fn source_artifact() -> rfb_content::CompiledArtifact {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    rfb_content::compile_pack_dir(&path).unwrap()
}

fn narrow(game: &mut Game, artifact: &rfb_content::CompiledArtifact, kind: &str) {
    let mut artifact = artifact.clone();
    let table = artifact
        .content
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.base-items")
        .unwrap();
    table.entries.retain(|entry| entry.item_kind_id == kind);
    assert_eq!(table.entries.len(), 1);
    game.content = Arc::new(ContentCatalog::from_artifact(artifact));
}

fn context(game: &Game) -> LootContext {
    LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: game.current_floor_id.clone(),
        depth: 80,
        source: LootSource::ItemUse {
            item_id: "test.dragon-base".into(),
        },
    }
}

fn has_base_properties(item: &GeneratedItemDraft) -> bool {
    let properties = &item.intrinsic_properties;
    !properties.resistances.is_empty()
        || !properties.status_immunities.is_empty()
        || !properties.brands.is_empty()
}

#[test]
fn natural_dragon_bases_keep_properties_when_great_power_is_suppressed_and_round_trip() {
    let mut game = Game::new_with_build(83, "demo.build.warrior").unwrap();
    let original = game.content.clone();
    let artifact = source_artifact();
    for suffix in KINDS {
        let kind = format!("demo.item.{suffix}");
        narrow(&mut game, &artifact, &kind);
        let context = context(&game);
        let mut suppressed = None;
        let mut ego = None;
        for seed in 0..200 {
            game.rng = RfbRng::seeded(seed);
            let draft = game
                .generate_one_loot_draft(&context, ItemGenerationMode::Great)
                .unwrap();
            if draft.kind_id != kind {
                continue;
            }
            assert!(has_base_properties(&draft), "{kind} seed {seed}");
            if draft.quality == ItemQualityDto::Ordinary {
                assert!(draft.affix_ids.is_empty());
                assert!(draft.enchantments.is_empty());
                suppressed.get_or_insert(draft);
            } else if !draft.affix_ids.is_empty() {
                ego.get_or_insert(draft);
            }
            if suppressed.is_some() && ego.is_some() {
                break;
            }
        }
        game.content = original.clone();
        for draft in [suppressed.unwrap(), ego.unwrap()] {
            game.items.clear();
            let properties = draft.intrinsic_properties.clone();
            let item = game
                .commit_generated_item_draft(draft, ItemLocation::Inventory)
                .unwrap();
            let id = item.id.clone();
            game.items.push(item);
            assert!(game.equip_inventory_item(&id, None).is_some());
            assert_eq!(game.items[0].intrinsic_properties, properties);
            let restored = Game::from_save(game.to_save()).unwrap();
            assert_eq!(restored.state_hash(), game.state_hash());
            assert_eq!(restored.rng, game.rng);
            assert_eq!(restored.items[0].intrinsic_properties, properties);
            if game.items[0].quality == ItemQualityDto::Ordinary {
                for immunity in &properties.status_immunities {
                    assert!(game.player_status_immunities().contains(immunity));
                }
                for element in properties.resistances.keys() {
                    let mut protected = game.clone();
                    let mut plain = game.clone();
                    plain.items[0].intrinsic_properties = Default::default();
                    for target in [&mut protected, &mut plain] {
                        target.player.hp = 1000;
                        target.resolve_monster_damage_to_player(
                            "test.caster",
                            "test.caster",
                            "test.spell",
                            0,
                            100,
                            100,
                            (*element).into(),
                            &mut Vec::new(),
                        );
                    }
                    assert!(protected.player.hp > plain.player.hp, "{kind} {element:?}");
                }
                if !properties.brands.is_empty() {
                    let mut target = game.entities[0].clone();
                    target.resistances = Default::default();
                    let definition = game.content.actor(&target.kind_id).unwrap();
                    let profile = game.player_melee_profile(&game.player_derived_stats());
                    assert_eq!(
                        game.player_melee_damage_multiplier(&profile, &target, definition),
                        24
                    );
                    let mut plain = game.clone();
                    plain.items[0].intrinsic_properties = Default::default();
                    assert_eq!(
                        plain.player_melee_damage_multiplier(&profile, &target, definition),
                        10
                    );
                }
            }
        }
    }
}

#[test]
fn special_no_fixed_artifact_skips_only_dragon_armor_power_suppression() {
    let artifact = source_artifact();
    for suffix in KINDS {
        let mut game = Game::new_with_build(83, "demo.build.warrior").unwrap();
        let kind = format!("demo.item.{suffix}");
        narrow(&mut game, &artifact, &kind);
        let context = context(&game);
        let mut suppressed = 0;
        let mut preserved = 0;
        for seed in 0..40 {
            game.rng = RfbRng::seeded(seed);
            let draft = game
                .generate_one_loot_draft(
                    &context,
                    ItemGenerationMode::Artifact {
                        no_fixed_artifact: true,
                    },
                )
                .unwrap();
            if draft.kind_id != kind {
                continue;
            }
            assert!(has_base_properties(&draft));
            if draft.quality == ItemQualityDto::Ordinary {
                suppressed += 1;
                assert_eq!(suffix, "dragon-fang");
            } else {
                assert_eq!(draft.quality, ItemQualityDto::Exceptional);
                assert!(!draft.affix_ids.is_empty());
                preserved += 1;
            }
        }
        assert!(preserved > 0);
        assert_eq!(suppressed > 0, suffix == "dragon-fang");
    }
}

fn dispatch(game: &mut Game, command: GameCommand) -> GameUpdate {
    let snapshot = game.snapshot();
    game.dispatch(GameCommandEnvelope {
        command_seq: snapshot.last_command_seq + 1,
        expected_revision: snapshot.revision,
        command,
    })
    .unwrap()
}

#[test]
fn crafting_preserves_dragon_base_properties_without_any_base_generation_draws() {
    for suffix in KINDS {
        let mut game = Game::new_with_build(83, "demo.build.warrior").unwrap();
        game.entities.clear();
        game.items.clear();
        let kind = format!("demo.item.{suffix}");
        game.debug_add_generated_inventory_item("test.dragon", &kind, 80)
            .unwrap();
        let properties = materialize(
            &mut RfbRng::seeded(83),
            game.content.item(&kind).unwrap(),
            &mut 0,
            false,
        )
        .unwrap();
        game.items[0].intrinsic_properties = properties.clone();
        game.debug_add_generated_inventory_item("test.craft", "demo.item.crafting-scroll", 80)
            .unwrap();
        game.rng = RfbRng::seeded(83);
        let mut without_properties = game.clone();
        without_properties.items[0].intrinsic_properties = Default::default();
        let command = GameCommand::UseItem {
            item_id: "test.craft".into(),
            target: Some(TargetSelection::Item {
                item_id: "test.dragon".into(),
            }),
        };
        let update = dispatch(&mut game, command.clone());
        dispatch(&mut without_properties, command);
        assert!(
            update
                .events
                .iter()
                .any(|event| event.kind == "item.use-crafting"),
            "{kind}"
        );
        assert_eq!(
            game.rng, without_properties.rng,
            "Craft must not roll dragon_resist or the suppression gate: {kind}"
        );
        assert_eq!(game.items[0].intrinsic_properties, properties);
        assert_eq!(
            game.items[0].affix_ids,
            without_properties.items[0].affix_ids
        );
        assert!(without_properties.items[0].intrinsic_properties == Default::default());
        assert_eq!(
            Game::from_save(game.to_save()).unwrap().state_hash(),
            game.state_hash()
        );
    }
}
