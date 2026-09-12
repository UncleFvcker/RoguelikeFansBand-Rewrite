// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::tests::support::{dispatch_next, reward_ready};
use crate::game::tests::town::enter_town_facility;
use rfb_protocol::{AutoGetModeDto, FacilityMembershipDto, FacilityServiceKindDto, LocaleDto};

#[test]
fn rewards_keep_birth_selection_replace_unique_bows_and_fail_atomically() {
    for realm in ["sorcery", "death", "arcane", "daemon"] {
        let build = format!("demo.build.ranger-nature-{realm}");
        let (mut thieves, task, facility, id) = reward_ready(925, &build, "thieves-hideout");
        thieves.claim_task_reward(&facility, &task).unwrap();
        assert_eq!(
            thieves
                .items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .kind_id,
            "demo.item.long-sword"
        );
        let mut seen = BTreeSet::new();
        for seed in 0..64 {
            let (mut game, task, facility, id) = reward_ready(seed, &build, "old-castle");
            let before = game.clone();
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
                let mut duplicate = before;
                duplicate.generated_artifact_ids.insert(kind);
                let mut full = duplicate.clone();
                while full.inventory_used_slots() < full.inventory_slot_capacity() {
                    let filler = format!("test.full.{}", full.items.len());
                    give_inventory_item(&mut full, &filler, "demo.item.dagger");
                }
                let full_save = full.to_save();
                assert_eq!(
                    full.claim_task_reward(&facility, &task),
                    Err("inventory-full")
                );
                assert_eq!(full.to_save(), full_save);
                let mut restored = Game::from_save(duplicate.to_save()).unwrap();
                for run in [&mut duplicate, &mut restored] {
                    run.claim_task_reward(&facility, &task).unwrap();
                    let item = run.items.iter().find(|item| item.id == id).unwrap();
                    assert_eq!(item.kind_id, "demo.item.long-bow");
                    assert!(item.artifact_name.is_some());
                    assert_eq!(
                        run.claim_task_reward(&facility, &task),
                        Err("reward-unavailable")
                    );
                    Game::from_save(run.to_save()).unwrap();
                    run.rng.bounded(1000);
                }
                assert_eq!(duplicate.to_save(), restored.to_save());
            }
            if seen.len() == 2 {
                break;
            }
        }
        assert_eq!(
            seen,
            ["demo.item.belthronding", "demo.item.yoichi"]
                .map(str::to_owned)
                .into()
        );
    }
}

#[test]
fn fixed_bows_generate_equip_fire_and_keep_uniqueness_after_loading() {
    for (slug, multiplier, shots) in [("belthronding", 300, 60), ("yoichi", 400, 0)] {
        let mut game = at_level(BUILD, 25);
        let context = LootContext {
            table_id: "demo.loot-table.base-items".into(),
            floor_id: game.current_floor_id.clone(),
            depth: 80,
            source: LootSource::ItemUse {
                item_id: "test.fixed-generation".into(),
            },
        };
        let kind = format!("demo.item.{slug}");
        assert!((0..5000).any(|seed| {
            game.rng = RfbRng::seeded(seed);
            game.roll_fixed_artifact_kind_id(&context, Some("demo.item.long-bow"), false)
                == Some(kind.clone())
        }));
        let draft = game.fixed_item_draft(&context, kind.clone());
        let item = game
            .commit_generated_item_draft(draft, ItemLocation::Inventory)
            .unwrap();
        let id = item.id.clone();
        game.items.push(item);
        let dexterity = game.effective_player_attributes().dexterity;
        assert!(game.equip_inventory_item(&id, None).is_some());
        game.refresh_player_resource_maxima();
        game.refresh_player_ability_state();
        game.reveal_current_visibility();
        assert_eq!(game.effective_player_attributes().dexterity, dexterity + 4);
        game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
        let item = game.items.iter().find(|item| item.id == id).unwrap();
        assert_eq!(
            game.item_equipment_bonuses(item).base_shot_delta_percent,
            shots
        );
        assert!(!game.inventory_item_dto(item).usable);
        if slug == "belthronding" {
            assert_eq!(game.item_equipment_bonuses(item).stealth_skill, 4);
            assert_eq!(
                game.effective_player_resistances()
                    .level(DamageType::Disenchant),
                ResistanceLevel::Resistant
            );
        } else {
            assert_eq!(game.player_see_invisible_sources(), 1);
        }
        let profile = game.player_projectile_profile().unwrap();
        assert_eq!(profile.damage_multiplier_percent, multiplier);
        assert_eq!(
            profile.launcher_to_damage,
            if slug == "belthronding" { 33 } else { 23 }
        );
        assert_eq!(
            profile.base_shot,
            game.player_derived_stats().ranged_skill.value.max(100) + shots
        );
        let ammunition = profile.ammo_item_id.clone().unwrap();
        let quantity = game
            .items
            .iter()
            .find(|item| item.id == ammunition)
            .unwrap()
            .quantity;
        let mut restored = Game::from_save(game.to_save()).unwrap();
        let tick = game.world_tick;
        let gain = energy_gain(derived_speed(&game.player_derived_stats().speed));
        let ticks = u32::try_from((profile.energy_cost + gain - 1) / gain).unwrap();
        for run in [&mut game, &mut restored] {
            dispatch_next(
                run,
                GameCommand::Fire {
                    direction: Direction::East,
                },
            );
            assert_eq!(
                run.items
                    .iter()
                    .find(|item| item.id == ammunition)
                    .unwrap()
                    .quantity,
                quantity - 1
            );
            for _ in 0..128 {
                assert_ne!(
                    run.roll_fixed_artifact_kind_id(&context, Some("demo.item.long-bow"), false),
                    Some(kind.clone())
                );
            }
        }
        assert_eq!(game.world_tick - tick, ticks);
        assert_eq!(game.to_save(), restored.to_save());
    }
}

#[test]
fn guild_enchantments_and_tower_identification_use_ranger_roles_and_saved_continuation() {
    for realm in ["sorcery", "death", "arcane", "daemon"] {
        let build = format!("demo.build.ranger-nature-{realm}");
        // Thalos defines building 11 in the source but has no corresponding map door.
        for town in ["anambar", "angwil", "morivant", "telmora"] {
            let mut game = at_level(&build, 3);
            let facility = format!("demo.town-facility.{town}-archer-guild");
            enter_town_facility(&mut game, &facility);
            game.reveal_current_visibility();
            for service in [
                FacilityServiceKindDto::EnchantAmmunition,
                FacilityServiceKindDto::EnchantBow,
            ] {
                let projection = game
                    .snapshot()
                    .task_services
                    .into_iter()
                    .find(|entry| entry.id == facility)
                    .unwrap();
                assert_eq!(projection.membership, FacilityMembershipDto::Owner);
                let target = projection
                    .service_actions
                    .iter()
                    .find(|entry| entry.kind == service)
                    .unwrap()
                    .targets
                    .first()
                    .unwrap();
                let cost = target.choices[0].cost;
                assert!(cost > 0);
                if service == FacilityServiceKindDto::EnchantAmmunition {
                    let quantity = game
                        .items
                        .iter()
                        .find(|item| item.id == target.item_id)
                        .unwrap()
                        .quantity;
                    assert_eq!(
                        cost,
                        game.town_service_price(if matches!(town, "anambar" | "angwil") {
                            22
                        } else {
                            20
                        }) * quantity
                    );
                }
                game.gold = cost;
                let mut restored = Game::from_save(game.to_save()).unwrap();
                let before = (game.world_tick, game.rng.clone());
                for run in [&mut game, &mut restored] {
                    let result = dispatch_next(
                        run,
                        GameCommand::UseFacilityService {
                            facility_id: facility.clone(),
                            service,
                            item_id: Some(target.item_id.clone()),
                            enchantment_steps: Some(1),
                        },
                    );
                    assert!(
                        result
                            .events
                            .iter()
                            .any(|event| event.kind == "facility.item-enchanted")
                    );
                    assert_eq!(run.gold, 0);
                    assert_eq!((run.world_tick, run.rng.clone()), before);
                }
                assert_eq!(game.to_save(), restored.to_save());
            }
        }
        for tower in ["thalos-sorcery-tower", "angwil-mage-tower"] {
            let mut game = at_level(&build, 3);
            let facility = format!("demo.town-facility.{tower}");
            enter_town_facility(&mut game, &facility);
            game.reveal_current_visibility();
            let definition = game.content.town_facility(&facility).unwrap();
            assert_eq!(
                game.town_facility_membership(definition),
                FacilityMembershipDto::Member
            );
            let price =
                game.town_facility_price(definition, definition.identify_all_items_cost.unwrap());
            assert_eq!(price, game.town_service_price(1000));
            give_inventory_item(&mut game, "test.unidentified", "demo.item.long-bow");
            game.gold = price;
            let mut restored = Game::from_save(game.to_save()).unwrap();
            for run in [&mut game, &mut restored] {
                run.identify_all_at_facility(&facility).unwrap();
                assert_eq!(run.gold, 0);
                assert!(run.item_property_knowledge["test.unidentified"].appraised);
                assert!(!run.item_property_knowledge["test.unidentified"].identified);
            }
            assert_eq!(game.to_save(), restored.to_save());
        }
    }
}

#[test]
fn changing_realm_updates_autopick_and_realm_guild_but_keeps_class_membership() {
    let mut game = super::learning::prepared("demo.build.ranger-nature-sorcery", 10);
    let book = super::learning::book(&mut game, "death", 1);
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
    let old_price = game.town_facility_price(&tower, tower.identify_all_items_cost.unwrap());
    dispatch_next(
        &mut game,
        GameCommand::BeginRealmChange {
            book_item_id: book.clone(),
        },
    );
    dispatch_next(&mut game, GameCommand::ResolveRealmChange { confirm: true });
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
    for (id, role) in [
        ("morivant-archer-guild", FacilityMembershipDto::Owner),
        ("thalos-sorcery-tower", FacilityMembershipDto::Member),
        ("angwil-mage-tower", FacilityMembershipDto::Member),
    ] {
        assert_eq!(
            game.town_facility_membership(
                game.content
                    .town_facility(&format!("demo.town-facility.{id}"))
                    .unwrap()
            ),
            role
        );
    }
    enter_town_facility(&mut game, &tower.id);
    game.reveal_current_visibility();
    game.gold = old_price;
    let mut restored = Game::from_save(game.to_save()).unwrap();
    for run in [&mut game, &mut restored] {
        let before = run.to_save();
        assert_eq!(
            run.identify_all_at_facility(&tower.id),
            Err("insufficient-gold")
        );
        assert_eq!(run.to_save(), before);
    }
    assert_eq!(game.to_save(), restored.to_save());
}
