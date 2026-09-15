// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::ability_scaling::device_power_value;

const N2: &[u32] = &[
    4, 8, 9, 10, 11, 12, 18, 23, 25, 40, 44, 48, 61, 82, 83, 84, 93, 98, 108, 115, 116, 119, 122,
    127, 128, 131, 133, 149, 159, 170, 184, 188, 202, 205, 207, 209, 211, 214, 218, 225, 252, 277,
    333, 336, 361, 363, 367,
];

#[test]
#[ignore = "explicit N2 preparation for ordinary standalone UI acceptance"]
fn export_n2_desktop_saves() {
    let input = std::path::PathBuf::from(std::env::var("ORDINARY_EQUIPMENT_INPUT").unwrap());
    let directory = input.parent().unwrap();
    let (header, payload) = rfb_save::decode(&std::fs::read(&input).unwrap()).unwrap();
    assert!(header.museum_binding.is_some());
    let mut base = Game::from_save(payload, Game::default_behavior_preferences()).unwrap();
    choose_human_talent_if_pending(&mut base);
    base.apply_player_experience(base.experience_required_for_level(50), &mut Vec::new());
    choose_human_talent_if_pending(&mut base);
    descend_one_floor(&mut base);
    base.apply_player_melee_status(STATUS_INVULNERABILITY, 200_000, "test.n2.desktop");
    let mut scenarios = Vec::new();
    for slug in ["carlammas", "narya", "frakir", "werewindle", "taikobo"] {
        let mut game = base.clone();
        clear_monsters(&mut game);
        game.items.clear();
        game.player.position = Position { x: 10, y: 10 };
        for x in 10..=15 {
            replace_terrain(&mut game, Position { x, y: 10 }, "demo.terrain.floor");
        }
        give_inventory_item(&mut game, "test.n2.desktop-light", "demo.item.wooden-torch");
        game.equip_inventory_item("test.n2.desktop-light", None)
            .unwrap();
        let id = generate(&mut game, &format!("demo.item.{slug}"));
        game.items.iter_mut().find(|i| i.id == id).unwrap().location =
            ItemLocation::Ground(game.player.position);
        if matches!(slug, "narya" | "frakir") {
            game.push_generated_actor(
                "test.n2.desktop-target".into(),
                "demo.actor.war-bear",
                Position { x: 11, y: 10 },
            );
            game.entities[0]
                .statuses
                .push(monster_combat::melee_status(STATUS_SLEEP, 10_000, "test.sleep").status);
        }
        if slug == "taikobo" {
            replace_terrain(
                &mut game,
                Position { x: 11, y: 10 },
                "demo.terrain.surface-water-shallow",
            );
        }
        game.refresh_player_resource_maxima();
        game.reveal_current_visibility();
        let target = matches!(slug, "narya" | "frakir" | "taikobo").then(east);
        let commands = vec![
            GameCommand::PickUp,
            GameCommand::Equip {
                item_id: id.clone(),
                slot_id: None,
            },
            GameCommand::UseItem {
                item_id: id.clone(),
                target,
            },
            GameCommand::Wait,
        ];
        let seed = (0..5000)
            .find(|seed| {
                let mut trial = game.clone();
                trial.rng = RfbRng::seeded(*seed);
                let mut activated = false;
                for command in &commands {
                    if matches!(command, GameCommand::UseItem { .. })
                        && matches!(slug, "narya" | "frakir")
                        && !trial.entities.iter().any(|a| {
                            a.id == "test.n2.desktop-target"
                                && a.position == Position { x: 11, y: 10 }
                        })
                    {
                        return false;
                    }
                    dispatch_next(&mut trial, command.clone());
                    if matches!(command, GameCommand::UseItem { .. }) {
                        activated = if slug == "taikobo" {
                            trial.fishing_direction.is_some()
                        } else {
                            charges(&trial, &id) == 0
                        };
                    }
                }
                activated
            })
            .unwrap();
        game.rng = RfbRng::seeded(seed);
        let start = game.clone();
        let mut steps = Vec::new();
        for command in commands {
            let activation_slot =
                game.items
                    .iter()
                    .find(|i| i.id == id)
                    .and_then(|i| match &i.location {
                        ItemLocation::Equipped { slot_id } => Some(slot_id.clone()),
                        _ => None,
                    });
            dispatch_next(&mut game, command.clone());
            assert_eq!(
                game.state_hash(),
                Game::from_save(game.to_save(), game.behavior_preferences())
                    .unwrap()
                    .state_hash()
            );
            steps.push(serde_json::json!({"command":command,"hash":game.state_hash(),"activationSlot":activation_slot}));
        }
        std::fs::write(
            directory.join(format!("{slug}.rfbsave")),
            rfb_save::encode(&header, &start.to_save()).unwrap(),
        )
        .unwrap();
        scenarios
            .push(serde_json::json!({"name":slug,"initialHash":start.state_hash(),"steps":steps}));
    }
    std::fs::write(
        directory.join("scenarios.json"),
        serde_json::to_vec_pretty(&scenarios).unwrap(),
    )
    .unwrap();
}

fn east() -> TargetSelection {
    TargetSelection::Direction {
        direction: Direction::East,
    }
}

fn activate(
    game: &mut Game,
    id: &str,
    target: Option<&TargetSelection>,
    glyph: Option<&str>,
) -> (Option<i32>, Vec<DomainEvent>) {
    let mut events = Vec::new();
    let cost = game
        .use_inventory_item(
            id,
            target,
            glyph,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    (cost, events)
}

fn success_seed() -> u64 {
    (0..1000)
        .find(|s| RfbRng::seeded(*s).bounded(100) < 5)
        .unwrap()
}

fn charged(slug: &str) -> (Game, String) {
    let mut game = prepared_mage();
    give_inventory_item(&mut game, "test.n2.light", "demo.item.wooden-torch");
    game.equip_inventory_item("test.n2.light", None).unwrap();
    let id = generate(&mut game, &format!("demo.item.{slug}"));
    game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
    game.equip_inventory_item(&id, None).unwrap();
    game.refresh_player_resource_maxima();
    game.reveal_current_visibility();
    game.rng = RfbRng::seeded(success_seed());
    (game, id)
}

fn charges(game: &Game, id: &str) -> u32 {
    game.items
        .iter()
        .find(|i| i.id == id)
        .unwrap()
        .charges
        .unwrap()
        .current
}

#[test]
fn n2_all_artifacts_generate_activate_and_resume_equipped_cooldowns() {
    let prepared = prepared_mage();
    let definitions: Vec<_> = prepared
        .content
        .item_definitions()
        .filter(|i| {
            i.artifact_generation
                .as_ref()
                .is_some_and(|g| N2.contains(&g.source_index))
        })
        .cloned()
        .collect();
    assert_eq!(definitions.len(), 47);
    for definition in definitions {
        let mut game = prepared.clone();
        give_inventory_item(&mut game, "test.n2.light", "demo.item.wooden-torch");
        game.equip_inventory_item("test.n2.light", None).unwrap();
        let kind = &definition.id;
        let source = definition
            .artifact_generation
            .as_ref()
            .unwrap()
            .source_index;
        let id = generate(&mut game, kind);
        assert!(
            !game
                .item_property_knowledge
                .get(&id)
                .is_some_and(|k| k.appraised)
        );
        game.reveal_current_visibility();
        game = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
        game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
        game.equip_inventory_item(&id, None).unwrap();
        game.refresh_player_resource_maxima();
        let mut target = None;
        let glyph = (source == 23).then_some("o");
        if matches!(
            source,
            8 | 10 | 11 | 12 | 82 | 83 | 93 | 108 | 115 | 122 | 131 | 184 | 188 | 202 | 207 | 218
        ) {
            game.push_generated_actor(
                "test.n2.target".into(),
                "demo.actor.war-bear",
                Position { x: 11, y: 10 },
            );
            target = Some(east());
        }
        if matches!(source, 98 | 211) {
            replace_terrain(&mut game, Position { x: 11, y: 10 }, "demo.terrain.wall");
            target = Some(east());
        }
        if source == 159 {
            replace_terrain(
                &mut game,
                Position { x: 11, y: 10 },
                "demo.terrain.surface-water-shallow",
            );
            target = Some(east());
        }
        if source == 119 {
            give_inventory_item(&mut game, "test.n2.identify", "demo.item.dagger");
            target = Some(TargetSelection::Item {
                item_id: "test.n2.identify".into(),
            });
        }
        if source == 25 {
            replace_terrain(
                &mut game,
                Position { x: 11, y: 10 },
                "demo.terrain.door-closed",
            );
        }
        game.reveal_current_visibility();
        let initial_charge = charges(&game, &id);
        assert!(source == 159 || initial_charge == 1);
        let failure = (0..1000)
            .find(|s| (5..10).contains(&RfbRng::seeded(*s).bounded(100)))
            .unwrap();
        let mut failed = game.clone();
        failed.rng = RfbRng::seeded(failure);
        let (_, events) = activate(&mut failed, &id, target.as_ref(), glyph);
        assert!(
            events.iter().any(|e| matches!(
                e,
                DomainEvent::DeviceSkillChecked {
                    succeeded: false,
                    ..
                }
            )),
            "{kind}: failure gate {events:?}"
        );
        assert_eq!(charges(&failed, &id), initial_charge, "{kind}");
        if target.is_some() || glyph.is_some() {
            let mut cancelled = game.clone();
            cancelled.rng = RfbRng::seeded(success_seed());
            let mut expected_rng = cancelled.rng.clone();
            expected_rng.bounded(100);
            activate(&mut cancelled, &id, None, None);
            assert_eq!(
                charges(&cancelled, &id),
                initial_charge,
                "{kind}: cancelled charge"
            );
            assert_eq!(
                cancelled.rng, expected_rng,
                "{kind}: cancel only checks device"
            );
        }
        game.rng = RfbRng::seeded(success_seed());
        let mut restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
        let left = activate(&mut game, &id, target.as_ref(), glyph);
        assert_eq!(
            left,
            activate(&mut restored, &id, target.as_ref(), glyph),
            "{kind}"
        );
        assert!(
            left.1.iter().any(|e| matches!(
                e,
                DomainEvent::DeviceSkillChecked {
                    succeeded: true,
                    ..
                }
            )),
            "{kind}: {left:?}"
        );
        assert_eq!(game.state_hash(), restored.state_hash(), "{kind}");
        assert_eq!(
            charges(&game, &id),
            if source == 159 { initial_charge } else { 0 },
            "{kind}"
        );
        match source {
            98 | 211 => assert_eq!(
                game.terrain_at(Position { x: 11, y: 10 }),
                "demo.terrain.floor"
            ),
            25 => assert_eq!(
                game.terrain_at(Position { x: 11, y: 10 }),
                "demo.terrain.door-broken"
            ),
            159 => assert_eq!(game.fishing_direction, Some(Direction::East)),
            119 => assert!(
                game.item_property_knowledge
                    .get("test.n2.identify")
                    .unwrap()
                    .appraised
            ),
            149 => assert!(game.recall.as_ref().unwrap().remaining_turns.is_some()),
            _ => {}
        }
        game.reveal_current_visibility();
        restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
        if let Some(recovery) = definition
            .device_generation
            .as_ref()
            .unwrap()
            .recovery
            .as_ref()
        {
            let interval = recovery.interval_ticks;
            let start = game.world_tick;
            for offset in 1..=interval {
                for g in [&mut game, &mut restored] {
                    g.world_tick = start + u32::from(offset);
                    g.process_inventory_device_recovery(&mut Vec::new());
                }
                if offset == interval / 2 {
                    restored = Game::from_save(restored.to_save(), restored.behavior_preferences())
                        .unwrap();
                }
                if offset == interval - 1 {
                    assert_eq!(charges(&restored, &id), 0, "{kind}: early recovery");
                }
            }
            assert_eq!(charges(&game, &id), 1, "{kind}: recovery");
            assert_eq!(
                game.state_hash(),
                restored.state_hash(),
                "{kind}: saved recovery"
            );
        }
        let slot = match &game.items.iter().find(|i| i.id == id).unwrap().location {
            ItemLocation::Equipped { slot_id } => slot_id.clone(),
            _ => panic!(),
        };
        game.unequip_slot(&slot).unwrap();
        let before = game.rng.clone();
        activate(&mut game, &id, target.as_ref(), glyph);
        assert_eq!(
            before, game.rng,
            "{kind}: inventory cannot activate equipment"
        );
    }
}

#[test]
fn n2_device_drain_ignores_nether_resistance_and_restores_life_before_hp() {
    for (slug, damage) in [("frakir", 100), ("theoden", 120), ("turmil", 90)] {
        for (actor, eligible) in [
            ("demo.actor.war-bear", true),
            ("demo.actor.skeleton-orc", false),
        ] {
            let (mut game, id) = charged(slug);
            game.push_generated_actor("test.drain".into(), actor, Position { x: 11, y: 10 });
            game.entities[0].hp = 1000;
            game.entities[0].max_hp = 1000;
            game.entities[0]
                .resistances
                .set(DamageType::Nether, ResistanceLevel::Immune);
            game.progress.life_force = 950;
            game.player.hp = 1;
            let life = game.progress.life_force;
            let hp = game.player.hp;
            let mut expected = game.clone();
            expected.restore_player_life_force(LifeForceRestorationRequest::add(50));
            expected.apply_player_healing(damage - 50);
            activate(&mut game, &id, Some(&east()), None);
            assert_eq!(
                game.entities[0].hp,
                if eligible { 1000 - damage } else { 1000 },
                "{slug}"
            );
            assert_eq!(
                game.progress.life_force,
                if eligible { 1000 } else { life },
                "{slug}"
            );
            assert_eq!(
                game.player.hp,
                if eligible { expected.player.hp } else { hp },
                "{slug}"
            );
        }
    }
}

#[test]
fn n2_balls_bolts_apply_source_damage_and_immunity_to_real_targets() {
    for (slug, element, amount) in [
        ("narya", DamageType::Fire, 300),
        ("nenya", DamageType::Cold, 400),
        ("vilya", DamageType::Electricity, 500),
        ("ringil", DamageType::Cold, 100),
        ("anduril", DamageType::Fire, 72),
        ("firestar", DamageType::Fire, 72),
        ("dragonic-sword", DamageType::Electricity, 500),
        ("ama-no-numahoko", DamageType::Water, 200),
        ("harness-of-the-hell", DamageType::Dark, 250),
        ("aranruth", DamageType::Cold, 0),
        ("incanus", DamageType::Mana, 0),
    ] {
        let (mut base, id) = charged(slug);
        base.push_generated_actor(
            "test.ball".into(),
            "demo.actor.war-bear",
            Position { x: 11, y: 10 },
        );
        base.entities[0].hp = 2000;
        base.entities[0].max_hp = 2000;
        for immune in [false, true] {
            let mut game = base.clone();
            game.entities[0].resistances.set(
                element,
                if immune {
                    ResistanceLevel::Immune
                } else {
                    ResistanceLevel::Normal
                },
            );
            activate(&mut game, &id, Some(&east()), None);
            let damage = 2000 - game.entities[0].hp;
            if immune {
                assert_eq!(damage, 0, "{slug}");
            } else if amount > 0 {
                assert_eq!(damage, amount, "{slug}");
            } else {
                assert!(
                    (if slug == "incanus" { 51..=250 } else { 12..=96 }).contains(&damage),
                    "{slug}: {damage}"
                );
            }
        }
    }
}

#[test]
fn n2_timed_activations_boost_one_roll_and_keep_the_stronger_timer() {
    for (slug, status, power) in [
        ("tulkas", STATUS_HASTE, 75),
        ("hurin", STATUS_HASTE, 50),
        ("taratol", STATUS_HASTE, 20),
        ("bubo", STATUS_HASTE, 20),
        ("efki", STATUS_HASTE, 30),
        ("matoi", "rfb.status.hero", 25),
        ("sword-of-tengri", "rfb.status.hero", 25),
        ("defender-of-the-crown", "rfb.status.stone-skin", 20),
        ("colluin", STATUS_BASIC_RESISTANCE, 20),
        ("mook", "rfb.status.resist-cold", 20),
        ("carlammas", STATUS_PROTECTION_FROM_EVIL, 100),
        ("himring", STATUS_PROTECTION_FROM_EVIL, 100),
        ("hermits-purple", STATUS_PROTECTION_FROM_EVIL, 100),
    ] {
        let (mut game, id) = charged(slug);
        let mut rng = game.rng.clone();
        rng.bounded(100);
        let roll = rng.bounded(if power == 100 { 25 } else { power }) + 1;
        let generic = matches!(
            slug,
            "mook" | "carlammas" | "himring" | "hermits-purple" | "matoi" | "sword-of-tengri"
        );
        activate(&mut game, &id, None, None);
        let duration = game
            .player
            .statuses
            .iter()
            .find(|s| s.kind_id == status)
            .unwrap()
            .remaining_ticks;
        assert_eq!(
            duration,
            ((power + roll + u64::from(generic)) * 10) as u32,
            "{slug}"
        );
        assert_eq!(game.rng, rng, "{slug}: one duration roll");
        let (mut boosted, boosted_id) = charged(slug);
        boosted
            .items
            .iter_mut()
            .find(|i| i.id == boosted_id)
            .unwrap()
            .intrinsic_properties
            .modifiers
            .device_power_bonus = 3;
        activate(&mut boosted, &boosted_id, None, None);
        let turns = device_power_value(power + roll, 3);
        assert_eq!(
            boosted
                .player
                .statuses
                .iter()
                .find(|s| s.kind_id == status)
                .unwrap()
                .remaining_ticks,
            ((turns + u64::from(generic)) * 10) as u32,
            "{slug}: boosted duration"
        );
        if matches!(slug, "tulkas" | "hurin") {
            assert_eq!(
                duration,
                game.player
                    .statuses
                    .iter()
                    .find(|s| s.kind_id == "rfb.status.hero")
                    .unwrap()
                    .remaining_ticks
            );
        }
        let mut expected = game.clone();
        expected
            .player
            .statuses
            .iter_mut()
            .find(|s| s.kind_id == status)
            .unwrap()
            .remaining_ticks = 10_000;
        expected
            .items
            .iter_mut()
            .find(|i| i.id == id)
            .unwrap()
            .charges
            .as_mut()
            .unwrap()
            .current = 1;
        expected.rng = RfbRng::seeded(success_seed());
        activate(&mut expected, &id, None, None);
        assert_eq!(
            expected
                .player
                .statuses
                .iter()
                .find(|s| s.kind_id == status)
                .unwrap()
                .remaining_ticks,
            10_000,
            "{slug}: no shortened timer"
        );
    }
}

#[test]
fn n2_escape_exercises_all_four_branches_from_equipped_artifacts() {
    for slug in ["werewindle", "kusanagi-no-tsurugi"] {
        let (base, id) = charged(slug);
        for wanted in [0, 5, 10, 12] {
            let seed = (0..100_000)
                .find(|seed| {
                    let mut r = RfbRng::seeded(*seed);
                    r.bounded(100) < 5 && r.bounded(13) == wanted
                })
                .unwrap();
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            let origin = game.player.position;
            let floor = game.current_floor_id.clone();
            let (_, events) = activate(&mut game, &id, None, None);
            assert_eq!(charges(&game, &id), 0);
            match wanted {
                0 | 5 => assert_ne!(game.player.position, origin, "{slug}: {events:?}"),
                10 => assert!(
                    matches!(
                        game.terrain_at(origin),
                        "demo.terrain.stairs-up" | "demo.terrain.stairs-down"
                    ),
                    "{slug}: {events:?}"
                ),
                _ => assert_ne!(game.current_floor_id, floor, "{slug}: {events:?}"),
            }
            game.reveal_current_visibility();
            assert_eq!(
                game.state_hash(),
                Game::from_save(game.to_save(), game.behavior_preferences())
                    .unwrap()
                    .state_hash()
            );
        }
    }
}

#[test]
fn n2_rings_change_actual_weapon_damage_and_shot_cost() {
    for (slug, pval) in [("narya", 1), ("nenya", 2), ("vilya", 3)] {
        let (mut game, id) = charged(slug);
        give_inventory_item(&mut game, "test.sword", "demo.item.long-sword");
        game.equip_inventory_item("test.sword", Some("right-hand"))
            .unwrap();
        give_inventory_item(&mut game, "test.bow", "demo.item.short-bow");
        game.equip_inventory_item("test.bow", None).unwrap();
        give_inventory_item(&mut game, "test.arrow", "demo.item.arrow");
        game.push_generated_actor(
            "test.ring".into(),
            "demo.actor.war-bear",
            Position { x: 11, y: 10 },
        );
        game.entities[0].hp = 10_000;
        game.entities[0].max_hp = 10_000;
        let mut plain = game.clone();
        let bonuses = &mut plain
            .items
            .iter_mut()
            .find(|i| i.id == id)
            .unwrap()
            .intrinsic_properties
            .equipment_bonuses;
        bonuses.weapon_dice_bonus = -pval;
        bonuses.base_shot_delta_percent = -15 * pval;
        let raw = |events: Vec<DomainEvent>| {
            events.into_iter().find_map(|e| match e.into_dto().outcome {
                Some(GameEventOutcomeDto::Damage { resolution }) => Some(resolution.raw_damage),
                _ => None,
            })
        };
        let seed = (0..1000)
            .find(|seed| {
                let mut a = game.clone();
                let mut b = plain.clone();
                a.rng = RfbRng::seeded(*seed);
                b.rng = a.rng.clone();
                matches!((raw(strike(&mut a)),raw(strike(&mut b))),(Some(a),Some(b)) if a>b)
            })
            .expect("ring weapon mastery increases actual damage");
        let mut a = game.clone();
        let mut b = plain.clone();
        a.rng = RfbRng::seeded(seed);
        b.rng = a.rng.clone();
        assert!(
            raw(strike(&mut a)).unwrap() > raw(strike(&mut b)).unwrap(),
            "{slug}"
        );
        let mut costs = Vec::new();
        for shooter in [&mut plain, &mut game] {
            shooter.rng = RfbRng::seeded(seed);
            shooter.reveal_current_visibility();
            let tick = shooter.world_tick;
            dispatch_next(
                shooter,
                GameCommand::Fire {
                    direction: Direction::East,
                },
            );
            costs.push(shooter.world_tick - tick);
        }
        assert!(costs[1] < costs[0], "{slug}: real shot cost {costs:?}");
    }
}

#[test]
fn n2_support_activations_reach_real_consumers() {
    let (mut flora, id) = charged("flora");
    flora.apply_player_melee_status(STATUS_POISON, 5000, "test.poison");
    flora.apply_player_melee_status(STATUS_FEAR, 500, "test.fear");
    activate(&mut flora, &id, None, None);
    assert!(!flora.player_has_status_kind(STATUS_FEAR));
    assert_eq!(
        flora
            .player
            .statuses
            .iter()
            .find(|s| s.kind_id == STATUS_POISON)
            .unwrap()
            .remaining_ticks,
        4000
    );

    let (mut restoring, id) = charged("asclepius");
    restoring.progress.life_force = 500;
    restoring.progress.attributes.strength -= 2;
    activate(&mut restoring, &id, None, None);
    assert_eq!(restoring.progress.life_force, 1000);
    assert_eq!(
        restoring.progress.attributes,
        restoring.progress.maximum_attributes
    );

    for (slug, damage) in [("faramir", 4), ("fundin-bluecloak", 250)] {
        let (mut game, id) = charged(slug);
        game.push_generated_actor(
            "test.visible".into(),
            "demo.actor.cave-orc",
            Position { x: 11, y: 10 },
        );
        game.push_generated_actor(
            "test.hidden".into(),
            "demo.actor.cave-orc",
            Position { x: 10, y: 13 },
        );
        for a in &mut game.entities {
            a.hp = 1000;
            a.max_hp = 1000;
        }
        replace_terrain(&mut game, Position { x: 10, y: 11 }, "demo.terrain.wall");
        game.reveal_current_visibility();
        game.rng = RfbRng::seeded(success_seed());
        let (_, events) = activate(&mut game, &id, None, None);
        assert_eq!(
            game.entities[0].hp,
            1000 - damage,
            "{slug}: visible {events:?}"
        );
        assert_eq!(game.entities[1].hp, 1000, "{slug}: obstructed");
    }
    let (mut charm, id) = charged("bolshoi");
    charm.push_generated_actor(
        "test.animal".into(),
        "demo.actor.war-bear",
        Position { x: 11, y: 10 },
    );
    let seed = (0..1000)
        .find(|s| {
            let mut t = charm.clone();
            t.rng = RfbRng::seeded(*s);
            activate(&mut t, &id, Some(&east()), None);
            t.entities[0].controller_id.as_deref() == Some(t.player.id.as_str())
        })
        .unwrap();
    charm.rng = RfbRng::seeded(seed);
    activate(&mut charm, &id, Some(&east()), None);
    assert_eq!(
        charm.entities[0].controller_id.as_deref(),
        Some(charm.player.id.as_str())
    );

    let (mut probe, id) = charged("surveillance");
    probe.push_generated_actor(
        "test.probe".into(),
        "demo.actor.war-bear",
        Position { x: 11, y: 10 },
    );
    probe.reveal_current_visibility();
    probe.rng = RfbRng::seeded(success_seed());
    let (_, events) = activate(&mut probe, &id, None, None);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, DomainEvent::AbilityMonstersProbed { .. })),
        "{events:?}"
    );
}

#[test]
fn n2_protection_uses_evil_target_and_source_level_resistance() {
    let (mut game, id) = charged("carlammas");
    activate(&mut game, &id, None, None);
    let neutral = game.content.actor("demo.actor.war-bear").unwrap().clone();
    let evil = game.content.actor("demo.actor.cave-orc").unwrap().clone();
    let rng = game.rng.clone();
    assert!(!game.protection_from_evil_repels(&neutral));
    assert_eq!(rng, game.rng);
    let mut outcomes = BTreeSet::new();
    for seed in 0..200 {
        game.rng = RfbRng::seeded(seed);
        outcomes.insert(game.protection_from_evil_repels(&evil));
    }
    assert_eq!(outcomes, BTreeSet::from([false, true]));
    game.player
        .statuses
        .retain(|s| s.kind_id != STATUS_PROTECTION_FROM_EVIL);
    let rng = game.rng.clone();
    assert!(!game.protection_from_evil_repels(&evil));
    assert_eq!(rng, game.rng);
}

#[test]
fn n2_julian_genocide_uses_selected_glyph_without_kill_rewards() {
    let (mut game, id) = charged("julian");
    for (id, kind, x) in [
        ("test.orc", "demo.actor.cave-orc", 11),
        ("test.other", "demo.actor.war-bear", 12),
        ("test.unique", "demo.actor.golfimbul-the-hill-orc-chief", 13),
    ] {
        game.push_generated_actor(id.into(), kind, Position { x, y: 10 });
    }
    game.reveal_current_visibility();
    let xp = game.progress.experience;
    let count = game.items.len();
    let seed = (0..1000)
        .find(|seed| {
            let mut t = game.clone();
            t.rng = RfbRng::seeded(*seed);
            activate(&mut t, &id, None, Some("o"));
            !t.entities.iter().any(|a| a.id == "test.orc")
        })
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    let hp = game.player.hp;
    let mut restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert_eq!(
        activate(&mut game, &id, None, Some("o")),
        activate(&mut restored, &id, None, Some("o"))
    );
    assert_eq!(game.state_hash(), restored.state_hash());
    assert!(!game.entities.iter().any(|a| a.id == "test.orc"));
    assert!(game.entities.iter().any(|a| a.id == "test.other"));
    assert!(game.entities.iter().any(|a| a.id == "test.unique"));
    assert!(game.player.hp < hp);
    assert_eq!(game.progress.experience, xp);
    assert_eq!(game.items.len(), count);
}
