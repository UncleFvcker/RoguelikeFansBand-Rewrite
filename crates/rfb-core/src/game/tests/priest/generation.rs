// SPDX-License-Identifier: MPL-2.0
use super::learning::{book, prepared};
use super::powers::EVIL;
use super::*;
use crate::game::tests::support::{dispatch_next, priest_build_ids, reward_ready};
use crate::game::tests::town::enter_town_facility;
use rfb_protocol::{AutoGetModeDto, FacilityMembershipDto, FacilityServiceKindDto, LocaleDto};

#[test]
fn every_priest_build_claims_the_source_thieves_and_orc_rewards_and_resumes() {
    let builds = priest_build_ids();
    assert_eq!(builds.len(), 24);
    for build in builds {
        for task_slug in ["thieves-hideout", "anambar-orc-camp"] {
            let (mut game, task, facility, id) = reward_ready(925, &build, task_slug);
            let mut restored = Game::from_save(game.to_save()).unwrap();
            game.claim_task_reward(&facility, &task).unwrap();
            restored.claim_task_reward(&facility, &task).unwrap();
            assert_eq!(game.to_save(), restored.to_save());
            let item = game.items.iter().find(|item| item.id == id).unwrap();
            if task_slug == "thieves-hideout" {
                assert_eq!(item.kind_id, "demo.item.war-hammer");
            } else {
                assert_eq!(
                    game.content
                        .item(&item.kind_id)
                        .unwrap()
                        .rfb_base_kind
                        .unwrap()
                        .tval,
                    21
                );
                assert!(
                    item.affix_ids
                        .iter()
                        .any(|id| id == "rfb-legacy.affix.slaying")
                );
                assert!(!item.rolled_affixes.is_empty());
            }
            assert!(game.equip_inventory_item(&id, None).is_some());
            game.refresh_player_resource_maxima();
            game.refresh_player_ability_state();
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
fn castle_rewards_keep_one_to_four_birth_selection_and_duplicate_replacement_atomic() {
    for primary in ["life", "crusade", "death", "daemon"] {
        let build = format!("demo.build.priest-{primary}-sorcery");
        let mut seen = BTreeSet::new();
        for seed in 0..64 {
            let (mut game, task, facility, id) = reward_ready(seed, &build, "old-castle");
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
                .find(|entry| entry.class_id == "demo.class.priest")
                .unwrap()
                .entries;
            assert_eq!(
                entries
                    .iter()
                    .map(|entry| (entry.item_kind_id.as_str(), entry.weight))
                    .collect::<Vec<_>>(),
                [
                    ("demo.item.aule", 1),
                    ("demo.item.palantir-of-westernesse", 4)
                ]
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
            assert!(game.generated_artifact_ids.contains(&kind));
            if seen.insert(kind.clone()) {
                assert!(game.equip_inventory_item(&id, None).is_some());
                game.refresh_player_resource_maxima();
                game.refresh_player_ability_state();
                if kind == "demo.item.aule" {
                    let profile = game.player_melee_profile(&game.player_derived_stats());
                    assert_eq!((profile.damage_dice, profile.damage_sides), (5, 7));
                    assert!(!game.priest_weapon_is_unblessed_blade(
                        game.items.iter().find(|item| item.id == id).unwrap()
                    ));
                } else {
                    game.rng = (0..1000)
                        .map(RfbRng::seeded)
                        .find(|rng| rng.clone().bounded(100) < 5)
                        .unwrap();
                    let mut events = Vec::new();
                    game.use_inventory_item(
                        &id,
                        None,
                        None,
                        &mut events,
                        &mut BTreeSet::new(),
                        &mut Vec::new(),
                    )
                    .unwrap();
                    assert!(
                        events.iter().any(|event| matches!(
                            event,
                            DomainEvent::ItemUniqueMonsterListed { .. }
                        ))
                    );
                }
                Game::from_save(game.to_save()).unwrap();
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
                        if kind == "demo.item.aule" {
                            "demo.item.great-hammer"
                        } else {
                            "demo.item.crystal-ball"
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
        assert_eq!(seen.len(), 2);
    }
}

#[test]
fn all_priests_receive_both_temple_owner_services_at_projected_prices() {
    for build in priest_build_ids() {
        for (slug, restore_price) in [("anambar-mammon-temple", 500), ("angwil-inner-temple", 300)]
        {
            let facility = format!("demo.town-facility.{slug}");
            let mut game = prepared(&build, 10);
            enter_town_facility(&mut game, &facility);
            game.reveal_current_visibility();
            game.player.hp = 1;
            game.progress.attributes.strength = game.progress.maximum_attributes.strength - 1;
            game.progress.life_force = 900;
            let projection = game
                .snapshot()
                .task_services
                .into_iter()
                .find(|entry| entry.id == facility)
                .unwrap();
            assert_eq!(projection.membership, FacilityMembershipDto::Owner);
            for (service, base_price) in [
                (FacilityServiceKindDto::Heal, 0),
                (FacilityServiceKindDto::RestoreVitality, restore_price),
            ] {
                let price = projection
                    .service_actions
                    .iter()
                    .find(|entry| entry.kind == service)
                    .unwrap()
                    .cost;
                assert_eq!(price, game.town_service_price(base_price));
                if price > 0 {
                    game.gold = price - 1;
                    let before = game.to_save();
                    assert_eq!(
                        game.use_town_facility_service(
                            &facility,
                            service,
                            None,
                            None,
                            &mut Vec::new()
                        )
                        .unwrap_err(),
                        "insufficient-gold"
                    );
                    assert_eq!(game.to_save(), before);
                }
                game.gold = price;
                let mut restored = Game::from_save(game.to_save()).unwrap();
                for run in [&mut game, &mut restored] {
                    run.use_town_facility_service(&facility, service, None, None, &mut Vec::new())
                        .unwrap();
                    assert_eq!(run.gold, 0);
                    if service == FacilityServiceKindDto::Heal {
                        assert_eq!(run.player.hp, run.effective_player_max_hp());
                    } else {
                        assert_eq!(run.progress.attributes, run.progress.maximum_attributes);
                        assert_eq!(run.progress.life_force, 1000);
                    }
                }
                assert_eq!(game.to_save(), restored.to_save());
            }
            if slug == "anambar-mammon-temple" {
                let price = projection
                    .service_actions
                    .iter()
                    .find(|entry| entry.kind == FacilityServiceKindDto::CureMutation)
                    .unwrap()
                    .cost;
                assert_eq!(price, game.town_service_price(10000));
                assert!(game.gain_mutation("rfb.mutation.alcohol", &mut Vec::new()));
                game.gold = price;
                let mut restored = Game::from_save(game.to_save()).unwrap();
                for run in [&mut game, &mut restored] {
                    run.use_town_facility_service(
                        &facility,
                        FacilityServiceKindDto::CureMutation,
                        None,
                        None,
                        &mut Vec::new(),
                    )
                    .unwrap();
                    assert!(
                        !run.progress
                            .active_mutation_ids
                            .contains("rfb.mutation.alcohol")
                    );
                    assert_eq!(run.gold, 0);
                }
                assert_eq!(game.to_save(), restored.to_save());
            }
        }
    }
}

#[test]
fn realm_changes_update_inscriptions_and_realm_services_without_extra_guild_membership() {
    for (build, next, life_owner) in [
        ("demo.build.priest-crusade-sorcery", "life", true),
        (EVIL, "craft", false),
    ] {
        let mut game = prepared(build, 10);
        let next_book = book(&mut game, next, 1);
        game.interface_locale = LocaleDto::EnUs;
        assert!(
            game.configure_mogaminator(
                true,
                false,
                AutoGetModeDto::Off,
                LocaleDto::EnUs,
                "second realm's spellbooks#changed".to_owned()
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
                book_item_id: next_book.clone(),
            },
        );
        dispatch_next(&mut game, GameCommand::ResolveRealmChange { confirm: true });
        assert_eq!(game.current_second_realm_id(), Some(next));
        assert_eq!(
            game.items
                .iter()
                .find(|item| item.id == next_book)
                .unwrap()
                .inscription
                .as_deref(),
            Some("changed")
        );
        assert_eq!(
            game.town_facility_membership(&tower),
            FacilityMembershipDto::Visitor
        );
        for slug in ["telmora-life-temple", "thalos-life-temple"] {
            let definition = game
                .content
                .town_facility(&format!("demo.town-facility.{slug}"))
                .unwrap();
            assert_eq!(
                game.town_facility_membership(definition),
                if life_owner {
                    FacilityMembershipDto::Owner
                } else {
                    FacilityMembershipDto::Visitor
                }
            );
        }
        for slug in ["angwil-mage-tower", "thalos-sorcery-tower"] {
            let definition = game
                .content
                .town_facility(&format!("demo.town-facility.{slug}"))
                .unwrap();
            assert_eq!(
                game.town_facility_membership(definition),
                FacilityMembershipDto::Visitor
            );
        }
        let restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.to_save(), game.to_save());
        assert_eq!(
            restored.town_facility_membership(&tower),
            FacilityMembershipDto::Visitor
        );
    }
}
