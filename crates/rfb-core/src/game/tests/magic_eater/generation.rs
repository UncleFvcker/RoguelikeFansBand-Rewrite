// SPDX-License-Identifier: MPL-2.0
use super::usage::{body, check_seed, item, use_body};
use super::*;
use crate::game::tests::support::{dispatch_next, reward_ready};
use crate::game::tests::town::enter_town_facility;
use rfb_protocol::FacilityMembershipDto;

#[test]
fn natural_three_device_categories_pick_up_absorb_use_recover_and_replay_generation() {
    let mut game = at_level(25);
    game.terrain.fill("demo.terrain.floor".into());
    game.glow.fill(true);
    game.player.position = Position { x: 10, y: 10 };
    let mut ids = Vec::new();
    for profile in [
        "demo.device-activation.magic-missile",
        "demo.device-activation.detect-objects",
        "demo.device-activation.trap-sense",
    ] {
        let id = crate::game::tests::devices::pick_utility(&mut game, profile);
        let original = item(&game, &id).clone();
        super::absorption::absorb(&mut game, &id, 0);
        let absorbed = item(&game, &id);
        assert_eq!(absorbed.id, original.id);
        assert_eq!(absorbed.activation, original.activation);
        assert!(matches!(
            absorbed.location,
            ItemLocation::Absorbed { slot: 0, .. }
        ));
        game.rng = check_seed(&game, &id, true).0;
        let before = item(&game, &id).charges.unwrap().current;
        let cost = item(&game, &id).activation.as_ref().unwrap().cost;
        let targets = if profile.ends_with("magic-missile") {
            game.push_generated_actor(
                "test.target".into(),
                "demo.actor.sheep",
                Position { x: 11, y: 10 },
            );
            vec![TargetSelection::Direction {
                direction: Direction::East,
            }]
        } else {
            Vec::new()
        };
        let (energy, _) = use_body(&mut game, &id, &targets);
        assert_eq!(energy, 100);
        assert_eq!(item(&game, &id).charges.unwrap().current, before - cost);
        ids.push(id);
    }
    clear_monsters(&mut game);
    let spent = ids
        .iter()
        .map(|id| item(&game, id).charges.unwrap().current)
        .collect::<Vec<_>>();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    for _ in 0..100 {
        assert_eq!(
            dispatch_next(&mut game, GameCommand::Wait),
            dispatch_next(&mut restored, GameCommand::Wait)
        );
    }
    for (id, before) in ids.iter().zip(spent) {
        assert!(item(&game, id).charges.unwrap().current > before);
    }
    assert_eq!(game.to_save(), restored.to_save());
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: game.current_floor_id.clone(),
        depth: 30,
        source: LootSource::MonsterDeath {
            actor_id: "test.next-drop".into(),
        },
    };
    assert_eq!(
        game.generate_loot_instances(&context, ItemLocation::Inventory)
            .unwrap(),
        restored
            .generate_loot_instances(&context, ItemLocation::Inventory)
            .unwrap()
    );
    assert_eq!(game.rng, restored.rng);
    assert_eq!(game.state_hash(), restored.state_hash());
}

#[test]
fn thieves_wargs_and_orc_rewards_use_real_items_and_remain_single_claims() {
    for (task, kind) in [
        ("thieves-hideout", "demo.item.long-sword"),
        ("pest-control", "demo.item.frost-bolt-wand"),
        ("anambar-orc-camp", "demo.item.frost-ball-wand"),
    ] {
        let (mut game, task, facility, id) = reward_ready(925, BUILD, task);
        let mut restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(
            game.claim_task_reward(&facility, &task),
            restored.claim_task_reward(&facility, &task)
        );
        assert_eq!(item(&game, &id).kind_id, kind);
        assert_eq!(game.to_save(), restored.to_save());
        if kind.ends_with("wand") {
            super::absorption::absorb(&mut game, &id, 0);
            game.rng = check_seed(&game, &id, true).0;
            let before = item(&game, &id).charges.unwrap().current;
            let (energy, _) = use_body(
                &mut game,
                &id,
                &[TargetSelection::Direction {
                    direction: Direction::East,
                }],
            );
            assert_eq!(energy, 100);
            assert!(item(&game, &id).charges.unwrap().current < before);
        } else {
            assert!(game.equip_inventory_item(&id, None).is_some());
        }
        let before = game.to_save();
        assert_eq!(
            game.claim_task_reward(&facility, &task),
            Err("reward-unavailable")
        );
        assert_eq!(game.to_save(), before);
        Game::from_save(before).unwrap();
    }
}

#[test]
fn castle_one_to_four_choice_is_birth_seeded_and_duplicate_replacement_is_atomic() {
    let mut seen = BTreeSet::new();
    for seed in 0..64 {
        let (mut game, task, facility, id) = reward_ready(seed, BUILD, "old-castle");
        let definition = game
            .content
            .world(&game.world_id)
            .unwrap()
            .tasks
            .iter()
            .find(|t| t.id == task)
            .unwrap();
        let entries = &definition
            .reward
            .as_ref()
            .unwrap()
            .class_overrides
            .iter()
            .find(|entry| entry.class_id == "demo.class.magic-eater")
            .unwrap()
            .entries;
        assert_eq!(
            entries
                .iter()
                .map(|entry| (entry.item_kind_id.as_str(), entry.weight))
                .collect::<Vec<_>>(),
            [("demo.item.lohengrin", 1), ("demo.item.charmed-pendant", 4)]
        );
        let original = game.clone();
        let mut advanced = Game::from_save(game.to_save()).unwrap();
        advanced.rng.bounded(12345);
        game.claim_task_reward(&facility, &task).unwrap();
        advanced.claim_task_reward(&facility, &task).unwrap();
        let kind = item(&game, &id).kind_id.clone();
        assert!(item(&game, &id).activation.is_some());
        assert!(
            game.absorbed_device_category(item(&game, &id)).is_none(),
            "activated armor and jewelry are not body devices"
        );
        assert_eq!(item(&advanced, &id).kind_id, kind);
        if seen.insert(kind.clone()) {
            let mut duplicate = original;
            duplicate.generated_artifact_ids.insert(kind.clone());
            let mut full = duplicate.clone();
            while full.inventory_used_slots() < full.inventory_slot_capacity() {
                let filler = format!("test.full.{}", full.items.len());
                give_inventory_item(&mut full, &filler, "demo.item.dagger");
            }
            let before = full.to_save();
            assert_eq!(
                full.claim_task_reward(&facility, &task),
                Err("inventory-full")
            );
            assert_eq!(full.to_save(), before);
            let mut restored = Game::from_save(duplicate.to_save()).unwrap();
            for run in [&mut duplicate, &mut restored] {
                run.claim_task_reward(&facility, &task).unwrap();
                assert_eq!(
                    item(run, &id).kind_id,
                    if kind == "demo.item.lohengrin" {
                        "demo.item.mithril-chain-mail"
                    } else {
                        "demo.item.amulet"
                    }
                );
                assert!(item(run, &id).artifact_name.is_some());
                assert_eq!(
                    run.claim_task_reward(&facility, &task),
                    Err("reward-unavailable")
                );
            }
            assert_eq!(duplicate.to_save(), restored.to_save());
        }
        if seen.len() == 2 {
            break;
        }
    }
    assert_eq!(
        seen,
        BTreeSet::from([
            "demo.item.lohengrin".to_owned(),
            "demo.item.charmed-pendant".to_owned()
        ])
    );
}

#[test]
fn two_towers_keep_visitor_membership_and_charge_the_source_ordinary_price() {
    for slug in ["angwil-mage-tower", "thalos-sorcery-tower"] {
        let mut game = at_level(3);
        let facility = format!("demo.town-facility.{slug}");
        enter_town_facility(&mut game, &facility);
        let definition = game.content.town_facility(&facility).unwrap();
        assert_eq!(
            game.town_facility_membership(definition),
            FacilityMembershipDto::Visitor
        );
        let cost =
            game.town_facility_price(definition, definition.identify_all_items_cost.unwrap());
        assert_eq!(cost, game.town_service_price(1000));
        give_inventory_item(&mut game, "test.unidentified", "demo.item.long-bow");
        game.gold = cost - 1;
        let before = game.to_save();
        assert_eq!(
            game.identify_all_at_facility(&facility),
            Err("insufficient-gold")
        );
        assert_eq!(game.to_save(), before);
        game.gold = cost;
        let mut restored = Game::from_save(game.to_save()).unwrap();
        for run in [&mut game, &mut restored] {
            run.identify_all_at_facility(&facility).unwrap();
            assert_eq!(run.gold, 0);
            assert!(run.item_property_knowledge["test.unidentified"].appraised);
        }
        assert_eq!(game.to_save(), restored.to_save());
    }
}

#[test]
fn ordinary_frost_bolt_scales_stored_power_and_replays_after_absorption() {
    let mut base = at_level(25);
    base.terrain.fill("demo.terrain.floor".into());
    base.player.position = Position { x: 10, y: 10 };
    body(&mut base, "test.frost", "demo.item.frost-bolt-wand", 0);
    let mut dice = BTreeSet::new();
    for power in [12, 16, 24] {
        let mut game = base.clone();
        // Source generation chooses the stored power; force its real materializer at each depth.
        for _ in 0..10000 {
            let Some(generated) = crate::game::ego::materialize_device(
                &game.content,
                &mut game.rng,
                game.content.item("demo.item.frost-bolt-wand").unwrap(),
                power,
                false,
                crate::game::loot::ItemGenerationMode::Ordinary,
                None,
            ) else {
                continue;
            };
            generated.apply_to(
                game.items
                    .iter_mut()
                    .find(|i| i.id == "test.frost")
                    .unwrap(),
            );
            if item(&game, "test.frost").activation.as_ref().unwrap().power == power {
                break;
            }
        }
        assert_eq!(
            item(&game, "test.frost").activation.as_ref().unwrap().power,
            power
        );
        dice.insert(5 + power / 8);
        game.push_generated_actor(
            "test.target".into(),
            "demo.actor.sheep",
            Position { x: 11, y: 10 },
        );
        let (start_rng, mut damage_rng) = check_seed(&game, "test.frost", true);
        let expected_damage = (0..5 + power / 8)
            .map(|_| damage_rng.bounded(8) as i32 + 1)
            .sum::<i32>();
        game.rng = start_rng;
        let mut ordinary = game.clone();
        ordinary
            .items
            .iter_mut()
            .find(|i| i.id == "test.frost")
            .unwrap()
            .location = ItemLocation::Inventory;
        let mut restored = Game::from_save(game.to_save()).unwrap();
        let target = TargetSelection::Direction {
            direction: Direction::East,
        };
        let (_, events) = use_body(&mut game, "test.frost", &[target.clone()]);
        let actual_damage = events
            .iter()
            .find_map(|event| match event {
                DomainEvent::ItemActivationHit { damage, .. }
                | DomainEvent::ItemActivationSlew { damage, .. } => Some(damage.raw),
                _ => None,
            })
            .expect("cold bolt hit the adjacent target");
        assert_eq!(actual_damage, expected_damage, "source (5 + power / 8)d8");
        assert_eq!(
            use_body(&mut restored, "test.frost", &[target.clone()]).1,
            events
        );
        let mut normal_events = Vec::new();
        ordinary
            .use_inventory_item(
                "test.frost",
                Some(&target),
                None,
                &mut normal_events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        assert_eq!(normal_events, events);
        assert_eq!(ordinary.entities, game.entities);
        assert_eq!(ordinary.rng, game.rng);
        assert_eq!(game.to_save(), restored.to_save());
    }
    assert!(
        dice.len() > 1,
        "stored power must reach different source dice counts"
    );
}
