// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::tests::support::{dispatch_next, reward_ready, warrior_mage_build_ids};
use crate::game::tests::town::enter_town_facility;
use rfb_protocol::{AutoGetModeDto, FacilityMembershipDto, LocaleDto};

fn use_item(game: &mut Game, id: &str) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.use_inventory_item(
        id,
        None,
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    events
}

fn charges(game: &Game, id: &str) -> u32 {
    game.items
        .iter()
        .find(|item| item.id == id)
        .unwrap()
        .charges
        .unwrap()
        .current
}

fn successful_device_seed(game: &mut Game) {
    // All ordinary device checks retain the source minimum success chance.
    game.rng = (0..1000)
        .map(RfbRng::seeded)
        .find(|rng| rng.clone().bounded(100) < 5)
        .unwrap();
}

#[test]
fn every_build_claims_and_uses_the_source_thieves_and_orc_rewards() {
    let builds = warrior_mage_build_ids();
    assert_eq!(builds.len(), 8);
    for build in builds {
        for (slug, kind) in [
            ("thieves-hideout", "demo.item.long-sword"),
            ("anambar-orc-camp", "demo.item.frost-ball-wand"),
        ] {
            let (mut game, task, facility, id) = reward_ready(925, &build, slug);
            let mut restored = Game::from_save(game.to_save()).unwrap();
            assert_eq!(
                game.claim_task_reward(&facility, &task),
                restored.claim_task_reward(&facility, &task)
            );
            assert_eq!(
                game.items
                    .iter()
                    .find(|item| item.id == id)
                    .unwrap()
                    .kind_id,
                kind
            );
            assert_eq!(game.to_save(), restored.to_save());
            if slug == "thieves-hideout" {
                assert!(game.equip_inventory_item(&id, None).is_some());
                game.refresh_player_ability_state();
            } else {
                successful_device_seed(&mut game);
                let before = charges(&game, &id);
                game.use_inventory_item(
                    &id,
                    Some(&TargetSelection::Direction {
                        direction: Direction::East,
                    }),
                    None,
                    &mut Vec::new(),
                    &mut BTreeSet::new(),
                    &mut Vec::new(),
                )
                .unwrap();
                assert!(charges(&game, &id) < before);
            }
            Game::from_save(game.to_save()).unwrap();
            let before = game.to_save();
            assert_eq!(
                game.claim_task_reward(&facility, &task),
                Err("reward-unavailable")
            );
            assert_eq!(game.to_save(), before);
        }
    }
}

#[test]
fn castle_choice_is_one_to_four_durable_and_duplicate_replacement_is_atomic() {
    let mut seen = BTreeSet::new();
    for seed in 0..64 {
        let (mut game, task, facility, id) = reward_ready(seed, BUILD, "old-castle");
        let task_definition = game
            .content
            .world(&game.world_id)
            .unwrap()
            .tasks
            .iter()
            .find(|entry| entry.id == task)
            .unwrap();
        let entries = &task_definition
            .reward
            .as_ref()
            .unwrap()
            .class_overrides
            .iter()
            .find(|entry| entry.class_id == "demo.class.warrior-mage")
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
        let mut advanced = game.clone();
        advanced.rng.bounded(12345);
        let mut restored = Game::from_save(game.to_save()).unwrap();
        game.claim_task_reward(&facility, &task).unwrap();
        restored.claim_task_reward(&facility, &task).unwrap();
        assert_eq!(game.to_save(), restored.to_save());
        let kind = game
            .items
            .iter()
            .find(|item| item.id == id)
            .unwrap()
            .kind_id
            .clone();
        assert!(game.generated_artifact_ids.contains(&kind));
        advanced.claim_task_reward(&facility, &task).unwrap();
        assert_eq!(
            advanced
                .items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .kind_id,
            kind
        );
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
                let item = run.items.iter().find(|item| item.id == id).unwrap();
                assert_eq!(
                    item.kind_id,
                    if kind == "demo.item.lohengrin" {
                        "demo.item.mithril-chain-mail"
                    } else {
                        "demo.item.amulet"
                    }
                );
                assert!(item.artifact_name.is_some());
                assert_eq!(
                    run.claim_task_reward(&facility, &task),
                    Err("reward-unavailable")
                );
                Game::from_save(run.to_save()).unwrap();
            }
            assert_eq!(duplicate.to_save(), restored.to_save());
        }
        if seen.len() == 2 {
            break;
        }
    }
    assert_eq!(
        seen,
        [
            "demo.item.lohengrin".to_owned(),
            "demo.item.charmed-pendant".to_owned()
        ]
        .into()
    );
}

#[test]
fn fixed_rewards_generate_equip_activate_and_resume_source_cooldowns() {
    for (slug, base, cooldown) in [
        ("lohengrin", "mithril-chain-mail", 3000),
        ("charmed-pendant", "amulet", 7770),
    ] {
        let mut game = at_level(BUILD, 25);
        let context = LootContext {
            table_id: "demo.loot-table.base-items".into(),
            floor_id: game.current_floor_id.clone(),
            depth: 80,
            source: LootSource::ItemUse {
                item_id: "test.natural-artifact".into(),
            },
        };
        let kind = format!("demo.item.{slug}");
        let base = format!("demo.item.{base}");
        assert!((0..5000).any(|seed| {
            game.rng = RfbRng::seeded(seed);
            game.roll_fixed_artifact_kind_id(&context, Some(&base), false) == Some(kind.clone())
        }));
        let draft = game.fixed_item_draft(&context, kind.clone());
        let item = game
            .commit_generated_item_draft(draft, ItemLocation::Inventory)
            .unwrap();
        let id = item.id.clone();
        game.items.push(item);
        assert!(game.equip_inventory_item(&id, None).is_some());
        game.refresh_player_ability_state();
        game.reveal_current_visibility();
        game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
        let passives = game.player_equipment_passives();
        assert!(passives.contains(&EquipmentPassive::SeeInvisible));
        if slug == "lohengrin" {
            assert!(passives.contains(&EquipmentPassive::HoldLife));
            let item = game.items.iter().find(|item| item.id == id).unwrap();
            assert_eq!(game.item_equipment_bonuses(item).stealth_skill, 4);
            assert_eq!(game.content.item(&kind).unwrap().modifiers.defense, 48);
            assert_eq!(game.content.item(&base).unwrap().modifiers.defense, 35);
            game.player.hp = 1;
            for status in [
                STATUS_BLINDNESS,
                STATUS_BLEEDING,
                STATUS_CONFUSION,
                STATUS_STUN,
                STATUS_BERSERK,
            ] {
                game.player.statuses.push(
                    super::super::monster_combat::melee_status(status, 1000, "test.curing").status,
                );
            }
            game.player.statuses.push(
                super::super::monster_combat::melee_status(STATUS_POISON, 8000, "test.curing")
                    .status,
            );
            game.minor_slow = 3;
        } else {
            assert!(passives.contains(&EquipmentPassive::EasySpell));
            assert!(passives.contains(&EquipmentPassive::Warning));
            assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));
            let item = game.items.iter().find(|item| item.id == id).unwrap();
            assert_eq!(game.item_equipment_bonuses(item).device_skill, 16);
            // Removing only EasySpell isolates its effect from the pendant's INT bonus.
            let failure = game
                .snapshot()
                .player
                .abilities
                .into_iter()
                .find(|a| a.id == "demo.ability.warrior-mage-hp-to-sp")
                .unwrap()
                .failure_percent;
            let mut ordinary = game.clone();
            let item = ordinary
                .items
                .iter_mut()
                .find(|item| item.id == id)
                .unwrap();
            item.kind_id = "demo.item.amulet".into();
            item.activation = None;
            item.charges = None;
            item.intrinsic_properties.modifiers.intelligence = 2;
            assert_eq!(
                failure + 5,
                ordinary
                    .snapshot()
                    .player
                    .abilities
                    .into_iter()
                    .find(|a| a.id == "demo.ability.warrior-mage-hp-to-sp")
                    .unwrap()
                    .failure_percent
            );
            game.resources.get_mut(MANA).unwrap().current = 0;
            game.apply_player_mental_status(STATUS_BERSERK, 100, "test.restore");
            for kind in [
                "demo.item.detect-objects-staff",
                "demo.item.magic-missile-wand",
                "demo.item.detection-rod",
            ] {
                let device = format!("test.{kind}");
                give_inventory_item(&mut game, &device, kind);
                game.items
                    .last_mut()
                    .unwrap()
                    .charges
                    .as_mut()
                    .unwrap()
                    .current = 0;
            }
        }
        successful_device_seed(&mut game);
        let mut restored = Game::from_save(game.to_save()).unwrap();
        let events = use_item(&mut game, &id);
        assert_eq!(events, use_item(&mut restored, &id));
        assert_eq!(game.to_save(), restored.to_save());
        assert_eq!(charges(&game, &id), 0);
        assert!(!game.player_has_status_kind(STATUS_BERSERK));
        if slug == "lohengrin" {
            assert!(
                events
                    .iter()
                    .any(|event| matches!(event, DomainEvent::ItemUsed { requested: 932, .. }))
            );
            for status in [
                STATUS_BLINDNESS,
                STATUS_BLEEDING,
                STATUS_CONFUSION,
                STATUS_STUN,
            ] {
                assert!(!game.player_has_status_kind(status));
            }
            assert_eq!(
                game.player
                    .statuses
                    .iter()
                    .find(|s| s.kind_id == STATUS_POISON)
                    .unwrap()
                    .remaining_ticks,
                4000
            );
            assert_eq!(game.minor_slow, 2);
            assert!(
                (26..=50).contains(
                    &game
                        .player
                        .statuses
                        .iter()
                        .find(|s| s.kind_id == "rfb.status.hero")
                        .unwrap()
                        .remaining_ticks
                )
            );
        } else {
            assert_eq!(game.resources[MANA].current, game.resources[MANA].maximum);
            for item in game
                .items
                .iter()
                .filter(|item| item.id.starts_with("test.demo.item."))
            {
                let maximum = item.charges.unwrap().maximum;
                let per_mille = if item.kind_id.ends_with("-rod") {
                    500
                } else {
                    250
                };
                assert_eq!(item.charges.unwrap().current, maximum * per_mille / 1000);
                assert_eq!(
                    item.device_recovery_progress,
                    ((maximum * per_mille) % 1000) as u16
                );
            }
        }
        for run in [&mut game, &mut restored] {
            let before = run.to_save();
            let events = use_item(run, &id);
            assert!(
                events
                    .iter()
                    .any(|event| matches!(event, DomainEvent::ItemUseUnavailable))
            );
            assert_eq!(run.to_save(), before);
            for _ in 0..cooldown - 1 {
                run.world_tick += 1;
                run.process_inventory_device_recovery(&mut Vec::new());
            }
            assert_eq!(charges(run, &id), 0);
            run.world_tick += 1;
            run.process_inventory_device_recovery(&mut Vec::new());
            assert_eq!(charges(run, &id), 1);
        }
        assert_eq!(game.to_save(), restored.to_save());
        if slug == "lohengrin" {
            game.player
                .statuses
                .iter_mut()
                .find(|s| s.kind_id == "rfb.status.hero")
                .unwrap()
                .remaining_ticks = 100;
            game.player
                .statuses
                .iter_mut()
                .find(|s| s.kind_id == STATUS_POISON)
                .unwrap()
                .remaining_ticks = 1000;
            game.minor_slow = 1;
            successful_device_seed(&mut game);
            let mut restored = Game::from_save(game.to_save()).unwrap();
            assert_eq!(use_item(&mut game, &id), use_item(&mut restored, &id));
            assert_eq!(game.to_save(), restored.to_save());
            assert_eq!(
                game.player
                    .statuses
                    .iter()
                    .find(|s| s.kind_id == "rfb.status.hero")
                    .unwrap()
                    .remaining_ticks,
                100
            );
            assert!(!game.player_has_status_kind(STATUS_POISON));
            assert_eq!(game.minor_slow, 0);
        }
        for _ in 0..128 {
            assert_ne!(
                game.roll_fixed_artifact_kind_id(&context, Some(&base), false),
                Some(kind.clone())
            );
        }
        Game::from_save(game.to_save()).unwrap();
    }
}

#[test]
fn every_build_pays_the_member_identification_price_and_restores_the_purchase() {
    for build in warrior_mage_build_ids() {
        for slug in ["angwil-mage-tower", "thalos-sorcery-tower"] {
            let mut game = at_level(&build, 3);
            let facility = format!("demo.town-facility.{slug}");
            enter_town_facility(&mut game, &facility);
            game.reveal_current_visibility();
            let definition = game.content.town_facility(&facility).unwrap();
            assert_eq!(
                game.town_facility_membership(definition),
                FacilityMembershipDto::Member
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
}

#[test]
fn realm_change_updates_book_inscriptions_and_realm_services_but_keeps_class_membership() {
    let mut game = at_level(BUILD, 10);
    let book = "test.life-book";
    give_inventory_item(&mut game, book, "demo.item.book-of-common-prayer");
    game.interface_locale = LocaleDto::EnUs;
    assert!(
        game.configure_mogaminator(
            true,
            false,
            AutoGetModeDto::Off,
            LocaleDto::EnUs,
            "second realm's spellbooks#changed".into()
        )
        .is_empty()
    );
    let tower = game
        .content
        .town_facility("demo.town-facility.morivant-sorcery-tower")
        .unwrap()
        .clone();
    assert_eq!(
        game.town_facility_membership(&tower),
        FacilityMembershipDto::Owner
    );
    dispatch_next(
        &mut game,
        GameCommand::BeginRealmChange {
            book_item_id: book.into(),
        },
    );
    dispatch_next(&mut game, GameCommand::ResolveRealmChange { confirm: true });
    assert_eq!(game.current_second_realm_id(), Some("life"));
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == book)
            .unwrap()
            .inscription
            .as_deref(),
        Some("changed")
    );
    assert_eq!(
        game.town_facility_membership(&tower),
        FacilityMembershipDto::Visitor
    );
    for (slug, role) in [
        ("telmora-life-temple", FacilityMembershipDto::Owner),
        ("thalos-life-temple", FacilityMembershipDto::Owner),
        ("angwil-mage-tower", FacilityMembershipDto::Member),
        ("thalos-sorcery-tower", FacilityMembershipDto::Member),
    ] {
        assert_eq!(
            game.town_facility_membership(
                game.content
                    .town_facility(&format!("demo.town-facility.{slug}"))
                    .unwrap()
            ),
            role
        );
    }
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.to_save(), game.to_save());
}
