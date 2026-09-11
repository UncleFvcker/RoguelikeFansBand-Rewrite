// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::tests::support::reward_ready;
use rfb_protocol::WeaponTraitDto;

fn same_save(game: &Game, restored: &Game) {
    let a = serde_json::to_value(game.to_save()).unwrap();
    let b = serde_json::to_value(restored.to_save()).unwrap();
    let differences: Vec<_> = a
        .as_object()
        .unwrap()
        .iter()
        .filter(|(key, value)| b.get(*key) != Some(*value))
        .map(|(key, _)| key)
        .collect();
    assert!(
        differences.is_empty(),
        "save fields differ: {differences:?}"
    );
}

#[test]
fn mage_task_rewards_keep_birth_selection_and_replace_previously_generated_artifacts() {
    let (mut game, task, facility, id) = reward_ready(925, BUILD, "thieves-hideout");
    game.claim_task_reward(&facility, &task).unwrap();
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == id)
            .unwrap()
            .kind_id,
        "demo.item.long-sword"
    );
    for build in [BUILD, "demo.build.high-mage-death"] {
        let mut seen = BTreeSet::new();
        for seed in 0..64 {
            let (mut game, task, facility, id) = reward_ready(seed, build, "old-castle");
            let before = game.clone();
            let mut changed_rng = game.clone();
            changed_rng.rng.bounded(12345);
            let mut restored = Game::from_save(game.to_save()).unwrap();
            assert_eq!(
                game.claim_task_reward(&facility, &task),
                restored.claim_task_reward(&facility, &task)
            );
            same_save(&game, &restored);
            let item = game.items.iter().find(|item| item.id == id).unwrap();
            let kind = item.kind_id.clone();
            assert!(game.generated_artifact_ids.contains(&kind));
            changed_rng.claim_task_reward(&facility, &task).unwrap();
            assert_eq!(
                changed_rng
                    .items
                    .iter()
                    .find(|item| item.id == id)
                    .unwrap()
                    .kind_id,
                kind
            );
            if seen.insert(kind.clone()) {
                let mut repeated = before;
                repeated.generated_artifact_ids.insert(kind.clone());
                let mut full = repeated.clone();
                while full.inventory_used_slots() < full.inventory_slot_capacity() {
                    let filler = format!("test.full.{}", full.items.len());
                    give_inventory_item(&mut full, &filler, "demo.item.dagger");
                }
                let full_save = full.to_save();
                assert_eq!(
                    full.claim_task_reward(&facility, &task),
                    Err("inventory-full")
                );
                assert!(full.to_save() == full_save);
                let mut restored = Game::from_save(repeated.to_save()).unwrap();
                repeated.claim_task_reward(&facility, &task).unwrap();
                restored.claim_task_reward(&facility, &task).unwrap();
                same_save(&repeated, &restored);
                let replacement = repeated.items.iter().find(|item| item.id == id).unwrap();
                let base = if kind == "demo.item.indra" {
                    "demo.item.hard-leather-cap"
                } else {
                    "demo.item.wizardstaff"
                };
                assert_eq!(replacement.kind_id, base);
                assert!(replacement.artifact_name.is_some());
                Game::from_save(repeated.to_save()).unwrap();
                assert_eq!(
                    repeated.claim_task_reward(&facility, &task),
                    Err("reward-unavailable")
                );
            }
            Game::from_save(game.to_save()).unwrap();
            if seen.len() == 3 {
                break;
            }
        }
        assert_eq!(
            seen,
            ["demo.item.gandalf", "demo.item.saruman", "demo.item.indra"]
                .map(str::to_owned)
                .into()
        );
    }
}

#[test]
fn fixed_mage_artifacts_generate_extra_power_equip_and_resume_activation_cooldowns() {
    for (slug, cooldown) in [("gandalf", 7770), ("saruman", 1110), ("indra", 0)] {
        // Level one is deliberate: Saruman grants all five resistances regardless of level.
        let mut game = Game::new_with_build(925, BUILD).unwrap();
        choose_human_talent_if_pending(&mut game);
        clear_monsters(&mut game);
        let context = LootContext {
            table_id: "demo.loot-table.base-items".into(),
            floor_id: game.current_floor_id.clone(),
            depth: 80,
            source: LootSource::ItemUse {
                item_id: "test.fixed-generation".into(),
            },
        };
        let kind = format!("demo.item.{slug}");
        let base = if slug == "indra" {
            "demo.item.hard-leather-cap"
        } else {
            "demo.item.wizardstaff"
        };
        let natural = (0..5000).find_map(|seed| {
            game.rng = RfbRng::seeded(seed);
            game.roll_fixed_artifact_kind_id(&context, Some(base), false)
                .filter(|id| id == &kind)
        });
        assert_eq!(
            natural.as_ref(),
            Some(&kind),
            "natural source pool includes {slug}"
        );
        let draft = game.fixed_item_draft(&context, kind.clone());
        if slug == "gandalf" {
            assert_ne!(draft.intrinsic_properties, Default::default());
        }
        let item = game
            .commit_generated_item_draft(draft, ItemLocation::Inventory)
            .unwrap();
        let id = item.id.clone();
        game.items.push(item);
        assert!(game.equip_inventory_item(&id, None).is_some());
        game.refresh_player_resource_maxima();
        game.refresh_player_ability_state();
        game.reveal_current_visibility();
        game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
        let item = game.items.iter().find(|item| item.id == id).unwrap();
        if slug == "indra" {
            assert_eq!(
                game.effective_player_resistances()
                    .level(DamageType::Electricity),
                ResistanceLevel::Immune
            );
            assert!(game.player_status_immunities().contains(STATUS_BLINDNESS));
            assert!(!game.inventory_item_dto(item).usable);
        } else {
            assert!(game.item_has_weapon_trait(item, WeaponTraitDto::ManaBrand));
            assert_eq!(
                game.item_equipment_bonuses(item).spell_capacity_bonus,
                if slug == "gandalf" { 4 } else { 3 }
            );
            // Find a successful real device check, then replay that exact attempt after loading.
            let mut successful = None;
            for seed in 0..1000 {
                let mut trial = game.clone();
                trial.rng = RfbRng::seeded(seed);
                let before = trial.clone();
                trial
                    .use_inventory_item(
                        &id,
                        None,
                        None,
                        &mut Vec::new(),
                        &mut BTreeSet::new(),
                        &mut Vec::new(),
                    )
                    .unwrap();
                if trial
                    .items
                    .iter()
                    .find(|item| item.id == id)
                    .unwrap()
                    .charges
                    .unwrap()
                    .current
                    == 0
                {
                    successful = Some((before, trial));
                    break;
                }
            }
            let (before, after) =
                successful.expect("artifact activation succeeds for a low-level Mage");
            let mut restored = Game::from_save(before.to_save()).unwrap();
            restored
                .use_inventory_item(
                    &id,
                    None,
                    None,
                    &mut Vec::new(),
                    &mut BTreeSet::new(),
                    &mut Vec::new(),
                )
                .unwrap();
            same_save(&after, &restored);
            game = after;
            if slug == "gandalf" {
                assert!(game.player_has_status_kind(STATUS_INVULNERABILITY));
            } else {
                let status = game
                    .player
                    .statuses
                    .iter()
                    .find(|status| status.kind_id == STATUS_BASIC_RESISTANCE)
                    .unwrap();
                assert_eq!(status.granted_resistances.len(), 5);
                assert!((21..=40).contains(&status.remaining_ticks));
            }
            let mut restored = Game::from_save(game.to_save()).unwrap();
            for run in [&mut game, &mut restored] {
                for _ in 0..cooldown - 1 {
                    run.world_tick += 1;
                    run.process_inventory_device_recovery(&mut Vec::new());
                }
                assert_eq!(
                    run.items
                        .iter()
                        .find(|item| item.id == id)
                        .unwrap()
                        .charges
                        .unwrap()
                        .current,
                    0
                );
                run.world_tick += 1;
                run.process_inventory_device_recovery(&mut Vec::new());
                assert_eq!(
                    run.items
                        .iter()
                        .find(|item| item.id == id)
                        .unwrap()
                        .charges
                        .unwrap()
                        .current,
                    1
                );
            }
            same_save(&game, &restored);
        }
        // Both forced rewards and natural candidates share the fixed-artifact registry.
        for _ in 0..128 {
            assert_ne!(
                game.roll_fixed_artifact_kind_id(&context, Some(base), false),
                Some(kind.clone())
            );
        }
        Game::from_save(game.to_save()).unwrap();
    }
}

#[test]
fn mage_can_buy_both_early_volumes_of_each_realm_and_resume_the_purchase() {
    for realm in REALMS {
        let first = if realm == "death" { "life" } else { "death" };
        let mut game =
            Game::new_with_build(925, &format!("demo.build.mage-{first}-{realm}")).unwrap();
        choose_human_talent_if_pending(&mut game);
        clear_monsters(&mut game);
        let shop = game
            .content
            .shop("demo.shop.outpost-bookstore")
            .unwrap()
            .clone();
        game.player.position = game
            .town_local_to_wilderness_view_position(
                &shop.town_id,
                position_from_content(shop.entrance_position),
            )
            .unwrap();
        game.mark_shop_visited_at_player().unwrap();
        game.reveal_current_visibility();
        game.gold = 100_000;
        for rank in [1, 2] {
            let kind = game
                .content
                .item_definitions()
                .find(|item| {
                    item.ability_book_id
                        .as_deref()
                        .and_then(|id| game.content.ability_book(id))
                        .is_some_and(|book| {
                            book.realm_id.as_deref() == Some(realm) && book.rank == Some(rank)
                        })
                })
                .unwrap()
                .id
                .clone();
            let stock = game
                .snapshot()
                .shops
                .into_iter()
                .find(|entry| entry.id == shop.id)
                .unwrap()
                .stock
                .into_iter()
                .find(|item| item.kind_id == kind)
                .unwrap();
            let mut restored = Game::from_save(game.to_save()).unwrap();
            assert_eq!(
                game.buy_from_shop(&shop.id, &stock.id, 1),
                restored.buy_from_shop(&shop.id, &stock.id, 1)
            );
            same_save(&game, &restored);
            assert!(
                game.items
                    .iter()
                    .any(|item| item.kind_id == kind && item.location == ItemLocation::Inventory)
            );
        }
    }
}
