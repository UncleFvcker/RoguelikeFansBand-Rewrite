// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use crate::game::player_combat::ProjectileMode;

fn context() -> LootContext {
    LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: "test.floor.depth-75".into(),
        depth: 75,
        source: LootSource::ItemUse {
            item_id: "test.c3-generation".into(),
        },
    }
}

fn combat_game(build: &str) -> Game {
    let mut game = Game::new_with_build(493, build).unwrap();
    choose_human_talent_if_pending(&mut game);
    descend_one_floor(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.player.position = Position { x: 10, y: 10 };
    for x in 8..=29 {
        for y in 8..=12 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game.glow.fill(true);
    game.reveal_current_visibility();
    game
}

fn give_artifact(game: &mut Game, slug: &str) -> String {
    let kind = format!("demo.item.{slug}");
    let base = game
        .content
        .item(&kind)
        .unwrap()
        .artifact_generation
        .as_ref()
        .unwrap()
        .base_item_kind_id
        .clone();
    let selected = (0..30_000)
        .find_map(|_| {
            game.roll_fixed_artifact_kind_id(&context(), Some(&base), false)
                .filter(|candidate| candidate == &kind)
        })
        .expect("real base/level/rarity/uniqueness gates admit the artifact");
    let draft = game.fixed_item_draft(&context(), selected);
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Ground(game.player.position))
        .unwrap();
    let id = item.id.clone();
    game.items.push(item);
    game.pick_up_item_at_player(Some(&id)).unwrap();
    game.equip_inventory_item(&id, None).unwrap();
    id
}

fn activate(
    game: &mut Game,
    id: &str,
    target: Option<&TargetSelection>,
) -> (Option<i32>, Vec<DomainEvent>) {
    let mut events = Vec::new();
    let cost = game
        .use_inventory_item(
            id,
            target,
            None,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    (cost, events)
}

fn east() -> TargetSelection {
    TargetSelection::Direction {
        direction: Direction::East,
    }
}

fn charge(game: &Game, id: &str) -> u32 {
    game.items
        .iter()
        .find(|item| item.id == id)
        .unwrap()
        .charges
        .unwrap()
        .current
}

fn restore(game: &Game) -> Game {
    Game::from_save(game.to_save()).unwrap()
}

fn hits(events: &[DomainEvent]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, DomainEvent::ProjectileHit { .. }))
        .count()
}

#[test]
fn c3_ballista_full_pool_activation_uses_real_shot_and_saved_ammunition() {
    let mut game = combat_game("demo.build.warrior");
    let item = (0..300_000)
        .find_map(|_| {
            game.generate_loot_instances(&context(), ItemLocation::Ground(game.player.position))
                .unwrap()
                .into_iter()
                .find(|item| item.kind_id == "demo.item.ballista")
        })
        .expect("Ballista must occur in the complete ordinary pool");
    let id = item.id.clone();
    let origin = (item.origin_kind, item.origin_actor_kind_id.clone());
    game.items.push(item);
    game.pick_up_item_at_player(Some(&id)).unwrap();
    game.equip_inventory_item(&id, None).unwrap();
    assert_eq!(game.equipment_modifiers().strength, 3);
    assert_eq!(
        game.player_projectile_profile()
            .unwrap()
            .damage_multiplier_percent,
        500
    );
    give_inventory_item(&mut game, "test.z-bolt", "demo.item.bolt");
    give_inventory_item(&mut game, "test.a-bolt", "demo.item.bolt");
    give_inventory_item(&mut game, "test.0-arrow", "demo.item.arrow");
    for item in game
        .items
        .iter_mut()
        .filter(|item| item.location == ItemLocation::Inventory)
    {
        item.quantity = 10;
    }
    game.items
        .iter_mut()
        .find(|item| item.id == "test.a-bolt")
        .unwrap()
        .intrinsic_properties
        .brands
        .insert(WeaponBrand::Fire);
    game.push_generated_actor(
        "test.target".into(),
        "demo.actor.great-hell-wyrm",
        Position { x: 12, y: 10 },
    );
    game.reveal_current_visibility();
    let profile = game.player_projectile_profile().unwrap();
    assert_eq!(profile.ammo_item_id.as_deref(), Some("test.a-bolt"));
    assert_eq!(
        game.inventory_item_dto(game.items.iter().find(|item| item.id == id).unwrap())
            .use_target_spec
            .unwrap()
            .range,
        profile.range
    );
    let successful = (0..1000)
        .find(|seed| {
            let mut trial = game.clone();
            trial.rng = RfbRng::seeded(*seed);
            let (_, events) = activate(&mut trial, &id, Some(&east()));
            hits(&events) == 1
        })
        .unwrap();
    game.rng = RfbRng::seeded(successful);
    let mut saved = restore(&game);
    let mut ordinary = game.clone();
    // Only the activation check precedes the real shot. With a single target,
    // brands (including this target's fire immunity), criticals and breakage
    // must agree with ordinary shooting from the same post-check RNG.
    let activation = game
        .items
        .iter()
        .find(|item| item.id == id)
        .unwrap()
        .activation
        .as_ref()
        .unwrap();
    let mut difficulty = DerivedStatsPipeline::new();
    difficulty.add(
        StatKind::ActionDifficulty,
        StatLayer::Environment,
        "demo.item.ballista",
        activation.device_check_difficulty,
    );
    let ability = ordinary.player_derived_stats().device_skill;
    assert!(
        resolve_check(
            &mut ordinary.rng,
            CheckContext {
                kind: CheckKind::UseDevice,
                actor_id: ordinary.player.id.clone(),
                target_id: Some(id.clone()),
                ability,
                difficulty: difficulty
                    .resolve(StatKind::ActionDifficulty, StatBounds::NON_NEGATIVE),
            }
        )
        .succeeded()
    );
    let mut ordinary_events = Vec::new();
    ordinary
        .resolve_player_projectile(
            east(),
            ProjectileMode::Normal,
            &mut ordinary_events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    let (cost, events) = activate(&mut game, &id, Some(&east()));
    let (saved_cost, saved_events) = activate(&mut saved, &id, Some(&east()));
    assert_eq!(cost, Some(profile.energy_cost));
    assert_eq!((cost, &events), (saved_cost, &saved_events));
    assert_eq!(game.state_hash(), saved.state_hash());
    assert_eq!(game.entities[0].hp, ordinary.entities[0].hp);
    assert_eq!(game.rng, ordinary.rng);
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.a-bolt")
            .unwrap()
            .quantity,
        9
    );
    for ammo in ["test.z-bolt", "test.0-arrow"] {
        assert_eq!(
            game.items
                .iter()
                .find(|item| item.id == ammo)
                .unwrap()
                .quantity,
            10
        );
    }
    let weapon = game.items.iter().find(|item| item.id == id).unwrap();
    assert_eq!(
        (weapon.origin_kind, weapon.origin_actor_kind_id.clone()),
        origin
    );
    assert_eq!(charge(&game, &id), 0);
    let rng = game.rng.clone();
    activate(&mut game, &id, Some(&east()));
    assert_eq!(
        game.rng, rng,
        "cooldown rejects without another check or shot"
    );
    clear_monsters(&mut game);
    for tick in 1..=499 {
        game.world_tick = tick;
        game.process_inventory_device_recovery(&mut Vec::new());
    }
    let mut saved = restore(&game);
    for tick in 500..1000 {
        saved.world_tick = tick;
        saved.process_inventory_device_recovery(&mut Vec::new());
    }
    assert_eq!(charge(&saved, &id), 0);
    saved.world_tick = 1000;
    saved.process_inventory_device_recovery(&mut Vec::new());
    assert_eq!(charge(&saved, &id), 1);
}

#[test]
fn c3_piercing_stops_on_miss_or_wall_and_caps_five_penetrations_without_forced_breakage() {
    let mut game = combat_game("demo.build.warrior");
    let id = give_artifact(&mut game, "ballista");
    give_inventory_item(&mut game, "test.bolt", "demo.item.bolt");
    game.items
        .iter_mut()
        .find(|item| item.id == "test.bolt")
        .unwrap()
        .quantity = 100;
    // Controlled hit bonus isolates the six-target limit from low-level accuracy.
    game.items
        .iter_mut()
        .find(|item| item.id == id)
        .unwrap()
        .intrinsic_properties
        .equipment_bonuses
        .ranged_skill = 1000;
    for x in 11..=17 {
        game.push_generated_actor(
            format!("test.target-{x}"),
            "demo.actor.great-hell-wyrm",
            Position { x, y: 10 },
        );
    }
    for actor in &mut game.entities {
        actor.alerted = false;
    }
    game.reveal_current_visibility();
    let fire = |game: &mut Game| {
        let mut events = Vec::new();
        game.resolve_player_projectile(
            TargetSelection::Entity {
                entity_id: "test.target-11".into(),
            },
            ProjectileMode::Piercing,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        events
    };
    let seed = (0..1000)
        .find(|seed| {
            let mut trial = game.clone();
            trial.rng = RfbRng::seeded(*seed);
            let events = fire(&mut trial);
            hits(&events) == 6
                && !events
                    .iter()
                    .any(|event| matches!(event, DomainEvent::ProjectileAmmoBroken { .. }))
        })
        .expect("six hits and an intact bolt are reachable");
    game.rng = RfbRng::seeded(seed);
    let mut blast = game.clone();
    blast
        .items
        .iter_mut()
        .find(|item| item.id == "test.bolt")
        .unwrap()
        .affix_ids = vec!["rfb-legacy.affix.exploding".into()];
    let outside_hp = blast
        .entities
        .iter()
        .find(|actor| actor.id == "test.target-15")
        .unwrap()
        .hp;
    let blast_events = fire(&mut blast);
    assert_eq!(
        blast
            .entities
            .iter()
            .find(|actor| actor.id == "test.target-15")
            .unwrap()
            .hp,
        outside_hp,
        "exploding ammunition ends the flight at the first hit"
    );
    assert!(
        blast_events
            .iter()
            .any(|event| matches!(event, DomainEvent::ProjectileAmmoBroken { .. }))
    );
    let mut wall = game.clone();
    replace_terrain(&mut wall, Position { x: 13, y: 10 }, "demo.terrain.wall");
    let wall_events = fire(&mut wall);
    assert_eq!(hits(&wall_events), 2);
    assert!(
        !wall
            .entities
            .iter()
            .find(|actor| actor.id == "test.target-14")
            .unwrap()
            .alerted
    );
    let mut miss = game.clone();
    let miss_seed = (0..1000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            (5..10).contains(&rng.bounded(100))
        })
        .unwrap();
    miss.rng = RfbRng::seeded(miss_seed);
    let miss_events = fire(&mut miss);
    assert_eq!(hits(&miss_events), 0);
    assert_eq!(
        miss_events
            .iter()
            .filter(|event| matches!(event, DomainEvent::ProjectileMissed { .. }))
            .count(),
        1
    );
    assert!(!miss.entities[1].alerted);
    let events = fire(&mut game);
    assert_eq!(hits(&events), 6);
    assert!(
        !game
            .entities
            .iter()
            .find(|actor| actor.id == "test.target-17")
            .unwrap()
            .alerted
    );
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.bolt")
            .unwrap()
            .quantity,
        99
    );
    assert!(
        game.items
            .iter()
            .any(|item| item.kind_id == "demo.item.bolt"
                && matches!(item.location, ItemLocation::Ground(_)))
    );
    assert_eq!(restore(&game).rng, game.rng);
}

#[test]
fn c3_ballista_cancel_failure_and_missing_ammo_keep_cooldown_and_success_uses_one_shot_energy() {
    let mut game = combat_game("demo.build.sniper");
    let id = give_artifact(&mut game, "ballista");
    give_inventory_item(&mut game, "test.bolt", "demo.item.bolt");
    game.items
        .iter_mut()
        .find(|item| item.id == "test.bolt")
        .unwrap()
        .quantity = 10;
    give_inventory_item(&mut game, "test.arrow", "demo.item.arrow");
    let success = (0..1000)
        .find(|seed| {
            let mut trial = game.clone();
            trial.rng = RfbRng::seeded(*seed);
            activate(&mut trial, &id, Some(&east())).0.is_some()
        })
        .unwrap();
    let failure = (0..1000)
        .find(|seed| {
            let mut trial = game.clone();
            trial.rng = RfbRng::seeded(*seed);
            activate(&mut trial, &id, Some(&east()))
                .1
                .iter()
                .any(|event| {
                    matches!(
                        event,
                        DomainEvent::DeviceSkillChecked {
                            succeeded: false,
                            ..
                        }
                    )
                })
        })
        .unwrap();
    game.rng = RfbRng::seeded(success);
    let mut cancelled = game.clone();
    let (cost, events) = activate(&mut cancelled, &id, None);
    assert_eq!(cost, None);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::DeviceSkillChecked {
            succeeded: true,
            ..
        }
    )));
    assert_eq!(charge(&cancelled, &id), 1);
    assert_eq!(
        cancelled
            .items
            .iter()
            .find(|item| item.id == "test.bolt")
            .unwrap()
            .quantity,
        10
    );
    let mut missing = game.clone();
    missing.items.retain(|item| item.id != "test.bolt");
    let (cost, events) = activate(&mut missing, &id, Some(&east()));
    assert_eq!(cost, None);
    assert_eq!(missing.rng, cancelled.rng);
    assert_eq!(charge(&missing, &id), 1);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::ProjectileLanded { .. }))
    );
    assert_eq!(
        missing
            .items
            .iter()
            .find(|item| item.id == "test.arrow")
            .unwrap()
            .quantity,
        1
    );
    let mut failed = game.clone();
    failed.rng = RfbRng::seeded(failure);
    let (cost, events) = activate(&mut failed, &id, Some(&east()));
    assert_eq!(cost, None);
    assert_eq!(charge(&failed, &id), 1);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::ProjectileLanded { .. }))
    );
    game.sniper_concentration = 2;
    let mut expected = game.clone();
    // Consume only the successful activation check before comparing the two
    // real commands. An empty lane isolates energy and ammunition settlement.
    activate(&mut expected, &id, None);
    dispatch_next(
        &mut expected,
        GameCommand::Fire {
            direction: Direction::East,
        },
    );
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: id.clone(),
            target: Some(east()),
        },
    );
    assert_eq!(
        (game.world_tick, game.player.energy_need),
        (expected.world_tick, expected.player.energy_need)
    );
    assert_eq!(game.rng, expected.rng);
    assert_eq!(game.sniper_concentration, 0);
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.bolt")
            .unwrap()
            .quantity,
        9
    );
    assert_eq!(charge(&game, &id), 0);
    assert_eq!(restore(&game).state_hash(), game.state_hash());
}

#[test]
fn c3_bloodrip_backlash_is_once_per_hand_after_misses_and_extends_cut_with_saved_rng() {
    let mut game = combat_game("demo.build.warrior");
    let id = give_artifact(&mut game, "bloodrip");
    assert_eq!(game.equipment_modifiers().constitution, 2);
    assert_eq!(game.player_equipment_bonuses().stealth_skill, -2);
    game.items
        .iter_mut()
        .find(|item| item.id == id)
        .unwrap()
        .intrinsic_properties
        .equipment_bonuses
        .melee_skill = -1000;
    game.push_generated_actor(
        "test.target".into(),
        "demo.actor.great-hell-wyrm",
        Position { x: 11, y: 10 },
    );
    game.apply_player_melee_status(STATUS_BLEEDING, 9998, "test.prior-cut");
    let profiles = game.player_melee_profiles(&game.player_derived_stats());
    assert_eq!(profiles.len(), 1);
    assert!(profiles[0].melee_skill.value <= 0);
    let mut seen = [false; 2];
    for seed in 0..30 {
        let mut trial = game.clone();
        trial.rng = RfbRng::seeded(seed);
        let mut expected = trial.rng.clone();
        if profiles[0].extra_attack_chance_percent > 0 {
            expected.bounded(100);
        }
        let backlash = expected.bounded(2) == 0;
        seen[usize::from(backlash)] = true;
        let amount = if backlash {
            2 + expected.bounded(3) as u32 + 1 + expected.bounded(3) as u32 + 1
        } else {
            0
        };
        let mut saved = restore(&trial);
        let attack = |game: &mut Game| {
            let mut events = Vec::new();
            game.resolve_player_melee(0, false, &mut events, &mut BTreeSet::new(), &mut Vec::new())
                .unwrap();
            events
        };
        let events = attack(&mut trial);
        assert_eq!(events, attack(&mut saved));
        assert_eq!(trial.rng, expected);
        let mut immune = game.clone();
        immune.rng = RfbRng::seeded(seed);
        immune.player.statuses.clear();
        immune
            .items
            .iter_mut()
            .find(|item| item.id == id)
            .unwrap()
            .intrinsic_properties
            .status_immunities
            .push(STATUS_BLEEDING.into());
        attack(&mut immune);
        assert!(!immune.player_has_status_kind(STATUS_BLEEDING));
        assert_eq!(
            immune.rng, expected,
            "cut immunity does not skip the backlash dice"
        );
        assert_eq!(trial.state_hash(), saved.state_hash());
        assert_eq!(
            trial
                .player
                .statuses
                .iter()
                .find(|status| status.kind_id == STATUS_BLEEDING)
                .unwrap()
                .remaining_ticks,
            (9998 + amount).min(10_000)
        );
        assert_eq!(events.iter().filter(|event| matches!(event, DomainEvent::ItemStatusResolved { source_kind_id, .. } if source_kind_id == "demo.item.bloodrip")).count(), usize::from(backlash));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, DomainEvent::PlayerMeleeHit { .. }))
        );
    }
    assert_eq!(seen, [true, true]);
    let mut killing = game;
    killing.player.statuses.clear();
    killing
        .items
        .iter_mut()
        .find(|item| item.id == id)
        .unwrap()
        .intrinsic_properties
        .equipment_bonuses
        .melee_skill = 0;
    clear_monsters(&mut killing);
    killing.push_generated_actor(
        "test.target".into(),
        "demo.actor.small-kobold",
        Position { x: 11, y: 10 },
    );
    let after_kill = (0..1000)
        .find_map(|seed| {
            let mut trial = killing.clone();
            trial.rng = RfbRng::seeded(seed);
            let mut events = Vec::new();
            let outcome = trial
                .resolve_player_melee(0, false, &mut events, &mut BTreeSet::new(), &mut Vec::new())
                .unwrap();
            (outcome.killed && trial.player_has_status_kind(STATUS_BLEEDING)).then_some(trial)
        })
        .expect("the killing hand still rolls Bloodrip backlash");
    assert!(
        (4..=8).contains(
            &after_kill
                .player
                .statuses
                .iter()
                .find(|status| status.kind_id == STATUS_BLEEDING)
                .unwrap()
                .remaining_ticks
        )
    );
    assert_eq!(restore(&after_kill).rng, after_kill.rng);
}

#[test]
fn c3_bloodrip_whirlwind_uses_sequential_melee_and_stops_after_lethal_contact() {
    let mut game = combat_game("demo.build.warrior");
    let id = give_artifact(&mut game, "bloodrip");
    for (target, x, y, kind) in [
        ("test.south", 10, 11, "demo.actor.small-kobold"),
        ("test.north", 10, 9, "demo.actor.great-hell-wyrm"),
        ("test.east", 11, 10, "demo.actor.small-kobold"),
        ("test.outside", 12, 10, "demo.actor.small-kobold"),
    ] {
        game.push_generated_actor(target.into(), kind, Position { x, y });
    }
    game.reveal_current_visibility();
    let seed = (0..1000)
        .find(|seed| {
            let mut trial = game.clone();
            trial.rng = RfbRng::seeded(*seed);
            let events = activate(&mut trial, &id, None).1;
            events.iter().any(|event| {
                matches!(
                    event,
                    DomainEvent::DeviceSkillChecked {
                        succeeded: true,
                        ..
                    }
                )
            }) && events
                .iter()
                .any(|event| matches!(event, DomainEvent::PlayerMeleeHit { .. }))
        })
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    let mut expected = game.clone();
    let ability = expected.player_derived_stats().device_skill;
    let mut difficulty = DerivedStatsPipeline::new();
    difficulty.add(
        StatKind::ActionDifficulty,
        StatLayer::Environment,
        "demo.item.bloodrip",
        40,
    );
    assert!(
        resolve_check(
            &mut expected.rng,
            CheckContext {
                kind: CheckKind::UseDevice,
                actor_id: expected.player.id.clone(),
                target_id: Some(id.clone()),
                ability,
                difficulty: difficulty
                    .resolve(StatKind::ActionDifficulty, StatBounds::NON_NEGATIVE)
            }
        )
        .succeeded()
    );
    let mut melee_events = Vec::new();
    for target in ["test.south", "test.north", "test.east"] {
        let index = expected
            .entities
            .iter()
            .position(|actor| actor.id == target)
            .unwrap();
        expected
            .resolve_player_melee(
                index,
                false,
                &mut melee_events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        if expected.player_is_dead() {
            break;
        }
    }
    let mut saved = restore(&game);
    let (_, events) = activate(&mut game, &id, None);
    assert_eq!(events, activate(&mut saved, &id, None).1);
    assert_eq!(game.entities, expected.entities);
    assert_eq!(game.player.hp, expected.player.hp);
    assert_eq!(game.player.statuses, expected.player.statuses);
    assert_eq!(game.rng, expected.rng);
    assert_eq!(game.state_hash(), saved.state_hash());
    assert_eq!(charge(&game, &id), 0);
    // A real fire aura can kill the wielder before the next adjacent target.
    let mut lethal = combat_game("demo.build.warrior");
    let id = give_artifact(&mut lethal, "bloodrip");
    lethal.player.hp = 1;
    lethal.push_generated_actor(
        "test.south".into(),
        "demo.actor.great-hell-wyrm",
        Position { x: 10, y: 11 },
    );
    lethal.push_generated_actor(
        "test.north".into(),
        "demo.actor.small-kobold",
        Position { x: 10, y: 9 },
    );
    lethal.entities[1].alerted = false;
    let (trial, events) = (0..1000)
        .find_map(|seed| {
            let mut trial = lethal.clone();
            trial.rng = RfbRng::seeded(seed);
            let events = activate(&mut trial, &id, None).1;
            trial.player_is_dead().then_some((trial, events))
        })
        .expect("contact aura can interrupt the whirlwind");
    assert!(
        !trial
            .entities
            .iter()
            .find(|actor| actor.id == "test.north")
            .unwrap()
            .alerted
    );
    assert_eq!(charge(&trial, &id), 0);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, DomainEvent::DeviceSkillChecked { .. }))
            .count(),
        1
    );
}

#[test]
fn c3_impaler_generates_charges_with_mount_and_locks_duelist_to_current_opponent() {
    for build in ["demo.build.cavalry", "demo.build.duelist"] {
        let mut game = combat_game(build);
        let id = give_artifact(&mut game, "impaler");
        let item = game.items.iter().find(|item| item.id == id).unwrap();
        assert_eq!(
            game.item_melee_profile(item).map(|profile| (
                profile.damage.dice,
                profile.damage.sides,
                profile.to_hit,
                profile.to_damage
            )),
            Some((5, 10, 17, 23))
        );
        let seed = (0..1000)
            .find(|seed| {
                let mut trial = game.clone();
                trial.rng = RfbRng::seeded(*seed);
                activate(&mut trial, &id, None).1.iter().any(|event| {
                    matches!(
                        event,
                        DomainEvent::DeviceSkillChecked {
                            succeeded: true,
                            ..
                        }
                    )
                })
            })
            .unwrap();
        game.rng = RfbRng::seeded(seed);
        let mut unmounted = game.clone();
        let (_, events) = activate(&mut unmounted, &id, Some(&east()));
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::DeviceSkillChecked {
                succeeded: true,
                ..
            }
        )));
        assert_eq!(charge(&unmounted, &id), 1);
        assert_eq!(unmounted.player.position, game.player.position);
        game.push_generated_actor(
            "test.mount".into(),
            "demo.actor.horse",
            game.player.position,
        );
        game.entities[0].controller_id = Some(game.player.id.clone());
        game.riding_actor_id = Some("test.mount".into());
        game.push_generated_actor(
            "test.target".into(),
            "demo.actor.great-hell-wyrm",
            Position { x: 16, y: 10 },
        );
        game.reveal_current_visibility();
        if game.player_is_duelist() {
            game.duelist_target_id = Some("test.target".into());
        }
        game.rng = RfbRng::seeded(seed);
        assert_eq!(
            game.player_melee_profiles(&game.player_derived_stats())[0].damage_dice,
            7
        );
        let target = if game.player_is_duelist() {
            TargetSelection::Direction {
                direction: Direction::North,
            }
        } else {
            east()
        };
        let mut blocked = game.clone();
        replace_terrain(&mut blocked, Position { x: 13, y: 10 }, "demo.terrain.wall");
        let (_, events) = activate(&mut blocked, &id, Some(&target));
        assert!(!events.iter().any(|event| matches!(
            event,
            DomainEvent::PlayerMeleeHit { .. } | DomainEvent::PlayerMeleeMissed { .. }
        )));
        assert_eq!(
            blocked.player.position.x,
            if game.player_is_duelist() { 10 } else { 12 }
        );
        if game.player_is_duelist() {
            assert_eq!(charge(&blocked, &id), 1);
        }
        let mut saved = restore(&game);
        let (_, events) = activate(&mut game, &id, Some(&target));
        assert_eq!(events, activate(&mut saved, &id, Some(&target)).1);
        assert_eq!(game.state_hash(), saved.state_hash());
        assert_eq!(game.player.position, Position { x: 15, y: 10 });
        assert_eq!(
            game.entities
                .iter()
                .find(|actor| actor.id == "test.mount")
                .unwrap()
                .position,
            game.player.position
        );
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::PlayerMeleeHit { .. } | DomainEvent::PlayerMeleeMissed { .. }
        )));
        assert_eq!(charge(&game, &id), 0);
        // Empty directions still use the source seven-square movement limit.
        if !game.player_is_duelist() {
            clear_monsters(&mut saved);
            saved.riding_actor_id = None;
            saved.player.position = Position { x: 10, y: 10 };
            saved.push_generated_actor(
                "test.mount".into(),
                "demo.actor.horse",
                saved.player.position,
            );
            saved.entities[0].controller_id = Some(saved.player.id.clone());
            saved.riding_actor_id = Some("test.mount".into());
            saved
                .items
                .iter_mut()
                .find(|item| item.id == id)
                .unwrap()
                .charges
                .as_mut()
                .unwrap()
                .current = 1;
            saved.rng = RfbRng::seeded(seed);
            activate(&mut saved, &id, Some(&east()));
            assert_eq!(saved.player.position, Position { x: 17, y: 10 });
            assert_eq!(charge(&saved, &id), 0);
        }
    }
}
