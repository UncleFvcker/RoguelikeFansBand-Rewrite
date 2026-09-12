// SPDX-License-Identifier: MPL-2.0
use super::materialization::materialize;
use super::*;
use crate::game::{DomainEvent, ItemLocation, Position};
use crate::game::{Game, tests::support::give_inventory_item};
use rfb_protocol::{Direction, TargetSelection};

fn content() -> rfb_content::ContentCatalog {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    rfb_content::ContentCatalog::from_artifact(rfb_content::compile_pack_dir(&path).unwrap())
}

fn dagger() -> ValueObject {
    ValueObject {
        tval: 23,
        sval: 4,
        dd: 1,
        ds: 4,
        base_dd: 1,
        base_ds: 4,
        weight: 12,
        ..Default::default()
    }
}

#[test]
fn random_artifact_value_bounds_and_soft_rejection_short_circuit() {
    let mut rng = RfbRng::seeded(85);
    let mut adjusted = false;
    let sword = ValueObject {
        base_dd: 2,
        base_ds: 6,
        ..dagger()
    };
    for (level, minimum, maximum, soft_maximum) in [
        (0, 0, 5000, 5000),
        (20, 0, 20000, 17500),
        (70, 50000, 100000, 80000),
        (100, 50000, 150000, 80000),
    ] {
        assert_eq!(
            value_limits(
                &mut rng,
                &sword,
                "demo.class.warrior",
                level,
                &mut adjusted,
                false,
                0
            ),
            ValueLimits {
                minimum,
                maximum,
                soft_maximum
            }
        );
    }
    let limits = ValueLimits {
        minimum: 10,
        maximum: 100,
        soft_maximum: 80,
    };
    let before = rng.clone();
    assert!(!accepts_score(&mut rng, 9, limits));
    assert!(!accepts_score(&mut rng, 101, limits));
    assert!(accepts_score(&mut rng, 10, limits));
    assert!(accepts_score(&mut rng, 80, limits));
    assert_eq!(rng, before);
    let mut both = BTreeSet::new();
    for seed in 0..16 {
        let mut rng = RfbRng::seeded(seed);
        both.insert(accepts_score(&mut rng, 81, limits));
        assert_eq!(rng.draw_counter, 1);
    }
    assert_eq!(both, BTreeSet::from([false, true]));
}

#[test]
fn random_artifact_exhaustion_constructs_fresh_candidate_1001_without_mutating_input() {
    let content = content();
    let data = content.random_artifact_generation().unwrap();
    let original = ValueObject {
        activation_value: 1_000_000,
        activation_timeout: 1,
        ..dagger()
    };
    let creation = Creation {
        level: 50,
        class_id: "demo.class.warrior",
        ..Default::default()
    };
    let mut rng = RfbRng::seeded(7);
    let mut quarks = BTreeSet::new();
    let (actual, attempts) = art_create_random(
        &mut rng,
        data,
        &original,
        creation,
        &mut quarks,
        50,
        2,
        &mut false,
        false,
        None,
        false,
        0,
        12,
    )
    .unwrap();
    assert_eq!(attempts, 1001);
    let mut replay_rng = RfbRng::seeded(7);
    let mut replay_quarks = BTreeSet::new();
    let mut expected = None;
    for _ in 0..1001 {
        expected = create_artifact(
            &mut replay_rng,
            data,
            original.clone(),
            creation,
            &mut replay_quarks,
            None,
            false,
            12,
        );
    }
    let expected = expected.unwrap();
    assert_eq!(actual.object, expected.object);
    assert_eq!(actual.name, expected.name);
    assert_eq!(rng, replay_rng);
    assert_eq!(quarks, replay_quarks);
    assert_eq!(
        original,
        ValueObject {
            activation_value: 1_000_000,
            activation_timeout: 1,
            ..dagger()
        }
    );
}

#[test]
fn random_artifact_name_tables_accept_source_whitespace_and_aliases() {
    let table = "\u{feff}N: 1:BIAS_ELEC\nN:2\nFirst\n\nN:3\nWrong\n";
    for entry in [1, 2] {
        assert_eq!(
            names::random_line(&mut RfbRng::seeded(85), table, entry),
            "First"
        );
    }
    let content = content();
    let table = &content.random_artifact_generation().unwrap().name_tables["a_high.txt"];
    assert!(!names::random_line(&mut RfbRng::seeded(85), table, 1).is_empty());
}

#[test]
fn random_artifact_names_precede_curses_and_scrolls_accept_user_names() {
    let content = content();
    let data = content.random_artifact_generation().unwrap();
    let creation = Creation {
        level: 70,
        class_id: "demo.class.paladin",
        good: true,
        ..Default::default()
    };
    let normal = create_artifact(
        &mut RfbRng::seeded(13),
        data,
        dagger(),
        creation,
        &mut BTreeSet::new(),
        None,
        false,
        12,
    )
    .unwrap();
    let cursed = create_artifact(
        &mut RfbRng::seeded(13),
        data,
        dagger(),
        Creation {
            cursed: true,
            ..creation
        },
        &mut BTreeSet::new(),
        None,
        false,
        12,
    )
    .unwrap();
    assert_eq!(normal.name, cursed.name);
    assert!(normal.curse.is_none());
    assert!(cursed.curse.is_some());
    assert!(normal.object.flags.is_subset(&cursed.object.flags));
    let scroll = create_artifact(
        &mut RfbRng::seeded(13),
        data,
        dagger(),
        Creation {
            scroll: true,
            cursed: true,
            requested_name: Some("验收"),
            ..creation
        },
        &mut BTreeSet::new(),
        None,
        false,
        12,
    )
    .unwrap();
    assert_eq!(scroll.name, "'验收'");
    assert!(scroll.curse.is_none());
    let mut quarks = BTreeSet::from([String::new()]);
    quarks.extend((1..names::QUARK_CAPACITY).map(|n| n.to_string()));
    assert_eq!(names::intern(&mut quarks, "new name".to_owned()), "");
    assert_eq!(quarks.len(), names::QUARK_CAPACITY);
}

#[test]
fn random_artifact_factory_covers_slots_and_preserves_item_identity_and_save_integrity() {
    let mut game = Game::new_with_build(85, "demo.build.warrior").unwrap();
    game.items.clear();
    game.entities.clear();
    let kinds: Vec<_> = game
        .content
        .item_definitions()
        .filter(|d| d.artifact_generation.is_none())
        .filter_map(|d| {
            d.rfb_base_kind
                .filter(|k| matches!(k.tval,19..=23|30..=40|45))
                .map(|k| (k.tval, d.id.clone()))
        })
        .collect::<std::collections::BTreeMap<_, _>>()
        .into_iter()
        .collect();
    let mut seen = BTreeSet::new();
    for (slot, kind) in kinds {
        give_inventory_item(&mut game, "test.generated-artifact", &kind);
        let original = game.items.pop().unwrap();
        let before = original.clone();
        let mut quarks = BTreeSet::new();
        let (item, attempts) = materialize(
            &game.content,
            &mut game.rng,
            &original,
            Creation {
                level: 30,
                class_id: "demo.class.warrior",
                ..Default::default()
            },
            &mut quarks,
            30,
            2,
            &mut false,
            false,
        )
        .unwrap();
        assert!((1..=1001).contains(&attempts));
        assert_eq!(original, before);
        assert_eq!(item.id, original.id);
        assert_eq!(item.location, original.location);
        assert!(item.artifact_name.is_some());
        assert!(item.affix_ids.is_empty());
        assert!(item.rolled_affixes.is_empty());
        assert_eq!(item.quantity, 1);
        assert!(super::super::item_value::obj_value_real(&game.content, &item).is_some());
        game.items.push(item);
        let restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        game.items.clear();
        seen.insert(slot);
    }
    assert!(
        BTreeSet::from([
            19, 20, 21, 22, 23, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 45
        ])
        .is_subset(&seen)
    );
}

#[test]
fn random_artifact_special_bases_keep_their_constraints_and_base_flags() {
    let content = content();
    let data = content.random_artifact_generation().unwrap();
    for (tval, sval, flags) in [
        (23, 32, vec!["NO_ENCHANT"]),
        (23, 33, vec!["BLOWS"]),
        (19, 70, vec!["CHR"]),
        (22, 50, vec![]),
    ] {
        let base_flags: BTreeSet<_> = flags.into_iter().map(str::to_owned).collect();
        for seed in 0..24 {
            let input = ValueObject {
                tval,
                sval,
                pval: if sval == 70 {
                    2
                } else if sval == 33 {
                    3
                } else {
                    0
                },
                ..dagger()
            };
            let roll = create_artifact(
                &mut RfbRng::seeded(seed),
                data,
                input,
                Creation {
                    level: 80,
                    class_id: "demo.class.warrior",
                    base_flags: Some(&base_flags),
                    good: true,
                    ..Default::default()
                },
                &mut BTreeSet::new(),
                None,
                false,
                12,
            )
            .unwrap();
            if sval == 32 {
                assert_eq!((roll.object.to_h, roll.object.to_d), (0, 0));
                assert!(!roll.object.flags.contains("BLOWS"));
            }
            if sval == 33 {
                assert!(roll.object.flags.contains("BLOWS"));
                assert!(roll.object.pval > 0);
            }
            if tval == 19 {
                assert_eq!((roll.object.to_h, roll.object.to_d), (0, 0));
                assert!(!roll.object.flags.contains("SHOW_MODS"));
                assert!(effective_flags(&roll.object, Some(&base_flags)).contains("CHR"));
            }
            if tval == 22 {
                assert_eq!((roll.object.dd, roll.object.ds), (1, 4));
            }
        }
    }
    let mut rng = RfbRng::seeded(0);
    let before = rng.clone();
    assert!(
        create_artifact(
            &mut rng,
            data,
            ValueObject {
                tval: 15,
                ..dagger()
            },
            Creation::default(),
            &mut BTreeSet::new(),
            None,
            false,
            12
        )
        .is_none()
    );
    assert!(
        create_artifact(
            &mut rng,
            data,
            dagger(),
            Creation {
                no_artifacts: true,
                ..Default::default()
            },
            &mut BTreeSet::new(),
            None,
            false,
            12
        )
        .is_none()
    );
    assert_eq!(rng, before);
}

fn generator<'a, 'r>(
    data: &'a RandomArtifactGenerationDefinition,
    rng: &'r mut RfbRng,
) -> Generator<'r, 'a> {
    Generator {
        rng,
        data,
        object: dagger(),
        level: 70,
        class_id: "demo.class.warrior",
        bias: Bias::None,
        has_pval: false,
        immunity_replaced: false,
        slaying: 0,
        activation: None,
        has_activation: false,
    }
}

#[test]
fn random_artifact_current_classes_and_themes_select_eligible_biases_and_activations() {
    let content = content();
    let data = content.random_artifact_generation().unwrap();
    for (theme, bias) in [
        ("warrior", Bias::Warrior),
        ("warrior-shoot", Bias::Warrior),
        ("samurai", Bias::Warrior),
        ("dwarf", Bias::Warrior),
        ("archer", Bias::Archer),
        ("mage", Bias::Mage),
        ("priest", Bias::Priestly),
        ("priest-evil", Bias::Necromantic),
        ("paladin", Bias::Law),
        ("paladin-evil", Bias::Necromantic),
        ("ninja", Bias::Rogue),
        ("hobbit", Bias::Rogue),
        ("rogue", Bias::Rogue),
    ] {
        let mut rng = RfbRng::seeded(0);
        let mut gen_ = generator(data, &mut rng);
        gen_.initial_bias(Creation {
            theme,
            ..Default::default()
        });
        assert_eq!(gen_.bias, bias);
        gen_.random_activation();
        if let Some(profile) = gen_.activation {
            let index = data
                .device_generation
                .activations
                .iter()
                .position(|p| p.id == profile.id)
                .unwrap();
            assert!(profile.device_check_difficulty >= 70 / 3);
            assert_ne!(data.activation_biases[index] & (1 << (bias as u8 - 1)), 0);
        }
    }
    for (class, bias) in [
        ("warrior", Bias::Warrior),
        ("duelist", Bias::Warrior),
        ("berserker", Bias::Warrior),
        ("mindcrafter", Bias::Priestly),
        ("archer", Bias::Warrior),
        ("cavalry", Bias::Warrior),
        ("high-mage", Bias::Mage),
        ("mage", Bias::Mage),
        ("sniper", Bias::Ranger),
        ("ranger", Bias::Ranger),
        ("paladin", Bias::Priestly),
    ] {
        let class_id = format!("demo.class.{class}");
        let mut seen = false;
        for seed in 0..64 {
            let mut rng = RfbRng::seeded(seed);
            let mut gen_ = generator(data, &mut rng);
            gen_.class_id = &class_id;
            gen_.initial_bias(Creation {
                class_id: &class_id,
                scroll: true,
                ..Default::default()
            });
            assert!([Bias::None, Bias::Warrior, bias].contains(&gen_.bias));
            seen |= gen_.bias == bias;
        }
        assert!(seen, "{class}");
    }
}

#[test]
fn real_build_misc_warning_and_no_tele_follow_source_boundaries() {
    // master artifact.c random_misc case 31: magik(10) for Berserker,
    // magik(90) otherwise. Fix the two source draws, not generated flags.
    for (build, threshold) in [("berserker", 10), ("warrior", 90)] {
        let game = Game::new_with_build(85, &format!("demo.build.{build}")).unwrap();
        let class_id = &game.build.as_ref().unwrap().class_id;
        for roll in [9, 10, 89, 90] {
            let seed = (0..100_000)
                .find(|seed| {
                    let mut rng = RfbRng::seeded(*seed);
                    rng.bounded(33) == 30 && rng.bounded(100) == roll
                })
                .expect("misc branch and probability boundary seed");
            let mut expected = RfbRng::seeded(seed);
            assert_eq!(expected.bounded(33), 30);
            assert_eq!(expected.bounded(100), roll);
            let mut rng = RfbRng::seeded(seed);
            let mut gen_ = generator(game.content.random_artifact_generation().unwrap(), &mut rng);
            gen_.class_id = class_id;
            gen_.misc();
            assert_eq!(
                gen_.object.flags,
                BTreeSet::from([if roll < threshold {
                    "WARNING"
                } else {
                    "NO_TELE"
                }
                .to_owned()]),
                "{build}: {roll}"
            );
            assert_eq!(rng, expected);
        }
    }
}

#[test]
fn real_mindcrafter_bias_is_scroll_only_and_uses_source_conversion_boundary() {
    for (build, class_bias, warrior_chance) in [
        ("mindcrafter", Bias::Priestly, 20),
        ("mage-death-sorcery", Bias::Mage, 20),
        ("warrior-mage-arcane-sorcery", Bias::Mage, 20),
        ("ranger-nature-sorcery", Bias::Ranger, 30),
        ("priest-life-sorcery", Bias::Priestly, 30),
        ("priest-death-sorcery", Bias::Priestly, 30),
    ] {
        let game = Game::new_with_build(85, &format!("demo.build.{build}")).unwrap();
        let class_id = &game.build.as_ref().unwrap().class_id;
        let data = game.content.random_artifact_generation().unwrap();
        for (theme, bias) in [
            ("", Bias::None),
            ("mage", Bias::Mage),
            ("priest", Bias::Priestly),
        ] {
            let mut rng = RfbRng::seeded(85);
            let before = rng.clone();
            let mut gen_ = generator(data, &mut rng);
            gen_.class_id = class_id;
            gen_.initial_bias(Creation {
                class_id,
                theme,
                ..Default::default()
            });
            assert_eq!(gen_.bias, bias);
            assert_eq!(rng, before, "natural mode must not draw for class bias");
        }
        // Factory boundary evidence complements the actual artifact-scroll command tests.
        for (gate, roll, bias) in [
            (0, warrior_chance - 1, Bias::Warrior),
            (0, warrior_chance, class_bias),
            (1, warrior_chance - 1, Bias::None),
        ] {
            let seed = (0..100_000)
                .find(|seed| {
                    let mut rng = RfbRng::seeded(*seed);
                    rng.bounded(4) == gate && rng.bounded(100) == roll
                })
                .unwrap();
            let mut expected = RfbRng::seeded(seed);
            expected.bounded(4);
            expected.bounded(100);
            let mut rng = RfbRng::seeded(seed);
            let mut gen_ = generator(data, &mut rng);
            gen_.class_id = class_id;
            gen_.initial_bias(Creation {
                class_id,
                scroll: true,
                ..Default::default()
            });
            assert_eq!(gen_.bias, bias);
            assert_eq!(rng, expected);
        }
    }
}

#[test]
fn random_artifact_throwing_flag_changes_real_throw_range_damage_and_instance_dice() {
    let mut game = Game::new_with_build(85, "demo.build.warrior").unwrap();
    game.items.clear();
    game.entities.clear();
    game.player.position = Position { x: 4, y: 4 };
    for x in 4..=6 {
        let i = game.index(Position { x, y: 4 }).unwrap();
        game.terrain[i] = "demo.terrain.floor".to_owned();
    }
    give_inventory_item(&mut game, "test.throwing", "demo.item.dagger");
    game.items[0].intrinsic_melee_damage_dice =
        Some(rfb_protocol::MeleeDamageDiceDto { dice: 2, sides: 1 });
    game.items[0].enchantments.to_hit = 1000;
    let normal = game.item_throw_parameters(&game.items[0]);
    game.items[0]
        .intrinsic_properties
        .rfb_flags
        .insert("THROWING".to_owned());
    let throwing = game.item_throw_parameters(&game.items[0]);
    assert!(throwing.0 > normal.0);
    assert!(throwing.1 > normal.1);
    assert_eq!(
        (
            game.item_throw_profile(&game.items[0]).unwrap().damage.dice,
            game.item_throw_profile(&game.items[0])
                .unwrap()
                .damage
                .sides
        ),
        (2, 1)
    );
    let actor = game
        .content
        .actor_definitions()
        .find(|a| {
            a.role == rfb_content::ActorRole::Monster && a.defense == 0 && a.resistances.is_empty()
        })
        .unwrap()
        .clone();
    game.entities.push(crate::game::spawn_actor_from_definition(
        &mut game.rng,
        &actor,
        "test.target",
        Position { x: 5, y: 4 },
        0,
        true,
    ));
    let mut events = Vec::new();
    game.throw_inventory_item(
        "test.throwing",
        Direction::East,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(events.iter().any(|event|matches!(event,DomainEvent::ItemThrowHit {damage,..} if damage.raw==2*throwing.1/100)),"{events:?}");
}

fn give_activation(game: &mut Game, token: &str) {
    give_inventory_item(game, "test.activation", "demo.item.dagger");
    let id = format!("rfb.device-activation.random-artifact.{token}");
    let profile = game
        .content
        .random_artifact_generation()
        .unwrap()
        .device_generation
        .activations
        .iter()
        .find(|p| p.id == id)
        .unwrap();
    let (activation, charges) = crate::game::ego::materialize_rfb_activation(profile);
    let item = game.items.last_mut().unwrap();
    item.artifact_name = Some("'验收'".to_owned());
    item.activation = Some(activation);
    item.charges = Some(charges);
}

fn use_activation(game: &mut Game, target: Option<&TargetSelection>) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    for _ in 0..100 {
        game.use_inventory_item(
            "test.activation",
            target,
            None,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        if game
            .items
            .iter()
            .find(|i| i.id == "test.activation")
            .unwrap()
            .charges
            .unwrap()
            .current
            == 0
        {
            return events;
        }
    }
    panic!("activation did not execute: {events:?}");
}

#[test]
fn random_artifact_enchantment_activation_requires_target_and_caps_each_component() {
    let mut game = Game::new_with_build(85, "demo.build.warrior").unwrap();
    game.items.clear();
    game.entities.clear();
    give_activation(&mut game, "enchantment");
    give_inventory_item(&mut game, "test.target", "demo.item.dagger");
    game.items[1].enchantments.to_hit = 13;
    game.items[1].enchantments.to_damage = 14;
    let before = game.rng.clone();
    game.use_inventory_item(
        "test.activation",
        None,
        None,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(game.rng, before);
    assert_eq!(game.items[0].charges.unwrap().current, 1);
    let events = use_activation(
        &mut game,
        Some(&TargetSelection::Item {
            item_id: "test.target".to_owned(),
        }),
    );
    assert_eq!(
        (
            game.items[1].enchantments.to_hit,
            game.items[1].enchantments.to_damage
        ),
        (15, 15)
    );
    assert_eq!(game.items[1].discount_percent, 99);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, DomainEvent::ItemEnchanted { .. }))
    );
}

#[test]
fn random_artifact_list_activation_reports_ground_artifacts_without_identifying_inventory() {
    let mut game = Game::new_with_build(85, "demo.build.warrior").unwrap();
    game.items.clear();
    game.entities.clear();
    give_activation(&mut game, "list-artifacts");
    give_inventory_item(&mut game, "test.ground", "demo.item.dagger");
    game.items[1].artifact_name = Some("'地面验收'".to_owned());
    game.items[1].location = ItemLocation::Ground(game.player.position);
    let events = use_activation(&mut game, None);
    let listed: Vec<_> = events
        .iter()
        .filter_map(|event| {
            if let DomainEvent::ItemListed { artifact_name, .. } = event {
                artifact_name.as_deref()
            } else {
                None
            }
        })
        .collect();
    assert_eq!(listed, vec!["'地面验收'"]);
    assert_eq!(game.snapshot().inventory[0].artifact_name, None);
}

#[test]
fn random_artifact_starlight_activation_casts_multiple_weak_light_beams() {
    let mut game = Game::new_with_build(85, "demo.build.warrior").unwrap();
    game.items.clear();
    game.entities.clear();
    game.terrain.fill("demo.terrain.floor".to_owned());
    game.glow.fill(false);
    give_activation(&mut game, "starlite");
    let events = use_activation(&mut game, None);
    let beams = events
        .iter()
        .filter(|event| matches!(event, DomainEvent::AbilityBeamDamage { .. }))
        .count();
    assert!((5..=15).contains(&beams));
    assert!(game.glow.iter().any(|glow| *glow));
}
