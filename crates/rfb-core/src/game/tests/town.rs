// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::game::tests::support::{dispatch_next, test_caster_game};
use rfb_protocol::{BountyOfficeActionDto, FacilityMembershipDto, FacilityServiceKindDto};

const GENERAL_STORE_ID: &str = "demo.shop.outpost-general-store";
const ARMOURY_ID: &str = "demo.shop.outpost-armoury";
const WEAPONSMITH_ID: &str = "demo.shop.outpost-weaponsmith";
const TEMPLE_ID: &str = "demo.shop.outpost-temple";
const ALCHEMIST_ID: &str = "demo.shop.outpost-alchemist";
const MAGIC_SHOP_ID: &str = "demo.shop.outpost-magic-shop";
const BLACK_MARKET_ID: &str = "demo.shop.outpost-black-market";
const BOOKSTORE_ID: &str = "demo.shop.outpost-bookstore";
const SHROOMERY_ID: &str = "demo.shop.outpost-shroomery";
const WHITE_HORSE_INN_ID: &str = "demo.shop.outpost-white-horse";
const HOME_ID: &str = "demo.town-facility.outpost-home";
const ANAMBAR_HOME_ID: &str = "demo.town-facility.anambar-home";
const ANAMBAR_MUSEUM_ID: &str = "demo.town-facility.anambar-museum";
const ANAMBAR_INN_ID: &str = "demo.shop.anambar-inn";
const THALOS_INN_ID: &str = "demo.shop.thalos-inn";
const THALOS_MUSEUM_ID: &str = "demo.town-facility.thalos-museum";
const OUTPOST_MUSEUM_ID: &str = "demo.town-facility.outpost-museum";
const ANAMBAR_LIBRARY_ID: &str = "demo.town-facility.anambar-library";
const ANAMBAR_WEAPON_MASTER_ID: &str = "demo.town-facility.anambar-weapon-master";
const ANAMBAR_WARRIOR_GUILD_ID: &str = "demo.town-facility.anambar-warrior-guild";
const ANAMBAR_MAMMON_TEMPLE_ID: &str = "demo.town-facility.anambar-mammon-temple";
const ANAMBAR_ARCHER_GUILD_ID: &str = "demo.town-facility.anambar-archer-guild";
const ANAMBAR_TRUMP_TOWER_ID: &str = "demo.town-facility.anambar-trump-tower";
const OUTPOST_COUNT_ID: &str = "demo.town-facility.outpost-count";
const OUTPOST_BOUNTY_OFFICE_ID: &str = "demo.town-facility.outpost-bounty-office";
const MORIVANT_TOWN_ID: &str = "demo.town.morivant";
const MORIVANT_INN_ID: &str = "demo.shop.morivant-inn";
const MORIVANT_HOME_ID: &str = "demo.town-facility.morivant-home";
const MORIVANT_SORCERY_TOWER_ID: &str = "demo.town-facility.morivant-sorcery-tower";
const MORIVANT_THIEVES_GUILD_ID: &str = "demo.town-facility.morivant-thieves-guild";

fn at1_game(edit: impl FnOnce(&mut rfb_content::CompiledContentV1)) -> Game {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    let mut content = rfb_content::compile_pack_dir(&root).unwrap().content;
    edit(&mut content);
    let catalog = Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(content).unwrap(),
    ));
    Game::from_content(42, catalog, DEFAULT_WORLD_ID).unwrap()
}

#[test]
fn zul_towers_project_realm_owners_beastman_members_and_public_prices() {
    let sorcery_id = "demo.town-facility.zul-sorcery-tower";
    let chaos_id = "demo.town-facility.zul-chaos-tower";
    let nature_id = "demo.town-facility.zul-nature-tower";
    for (build, race, realm_owner, chaos_member) in [
        (
            "demo.build.mage-sorcery-nature",
            "demo.race.rfb-human",
            true,
            false,
        ),
        (
            "demo.build.mage-nature-sorcery",
            "demo.race.rfb-human",
            true,
            false,
        ),
        (
            "demo.build.warrior",
            "rfb-legacy.race.beastman",
            false,
            true,
        ),
        ("demo.build.warrior", "demo.race.rfb-human", false, false),
    ] {
        let mut game = Game::new_with_build_race_and_name(42, build, race, "Zul").unwrap();
        enter_town(&mut game, "demo.town.zul", Position { x: 77, y: 6 });
        for id in [sorcery_id, chaos_id, nature_id] {
            let facility = game.content.town_facility(id).unwrap();
            game.player.position = game
                .town_local_to_active_position(
                    "demo.town.zul",
                    position_from_content(facility.entrance_position),
                )
                .unwrap();
            let snapshot = game.snapshot();
            let service = snapshot
                .task_services
                .iter()
                .find(|service| service.id == id)
                .unwrap();
            assert!(service.player_at_entrance);
            if id == sorcery_id {
                assert_eq!(service.tasks.len(), 2);
                let eddies = service
                    .tasks
                    .iter()
                    .find(|task| task.task_id == "demo.task.zul-eddies")
                    .unwrap();
                assert_eq!(eddies.status, TaskStatusKindDto::Available);
                assert!(eddies.unavailable_reason.is_none());
            } else {
                assert_eq!(service.tasks.len(), 1);
            }
            let expected = if realm_owner && id != chaos_id {
                FacilityMembershipDto::Owner
            } else if chaos_member && id == chaos_id {
                FacilityMembershipDto::Member
            } else {
                FacilityMembershipDto::Visitor
            };
            assert_eq!(service.membership, expected, "{build}/{race}/{id}");
            let node = service
                .tasks
                .iter()
                .find(|task| task.task_id.ends_with("-node"))
                .unwrap();
            assert_eq!(
                node.unavailable_reason.as_deref(),
                (expected == FacilityMembershipDto::Visitor).then_some("task-membership-required")
            );
            let base = if id == sorcery_id {
                if realm_owner { 100 } else { 800 }
            } else if id == nature_id {
                if realm_owner { 2_000 } else { 10_000 }
            } else {
                5_000
            };
            let cost = if id == sorcery_id {
                service.identify_all_items_cost.unwrap()
            } else {
                service.service_actions[0].cost
            };
            assert_eq!(cost, game.town_service_price(base));
        }
        assert!(
            game.teleport_town_targets()
                .iter()
                .all(|target| target.town_id != "demo.town.zul")
        );
    }
}

#[test]
fn zul_sorcery_identifies_all_for_visitors_and_rejects_unpaid_or_empty_work() {
    let id = "demo.town-facility.zul-sorcery-tower";
    let mut game = Game::new_with_build(42, "demo.build.warrior").unwrap();
    enter_town_facility(&mut game, id);
    support::give_inventory_item(&mut game, "test.zul.identify", "demo.item.dagger");
    let cost = game.town_service_price(800);
    game.gold = cost - 1;
    let before = game.to_save();
    assert_eq!(game.identify_all_at_facility(id), Err("insufficient-gold"));
    assert_eq!(game.to_save(), before);
    game.gold = cost;
    let outcome = game.identify_all_at_facility(id).unwrap();
    assert!(outcome.identified_count > 0);
    assert_eq!(game.gold, 0);
    let knowledge = &game.item_property_knowledge["test.zul.identify"];
    assert!(knowledge.appraised || knowledge.identified);
    game.reveal_current_visibility();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    let before = restored.to_save();
    assert_eq!(
        restored.identify_all_at_facility(id),
        Err("nothing-to-identify")
    );
    assert_eq!(restored.to_save(), before);
}

#[test]
fn zul_chaos_cures_only_unlocked_mutations_and_charges_members_the_public_price() {
    let id = "demo.town-facility.zul-chaos-tower";
    let mutation = "rfb.mutation.alcohol";
    let mut game = Game::new_with_build_race_and_name(
        42,
        "demo.build.warrior",
        "rfb-legacy.race.beastman",
        "Zul",
    )
    .unwrap();
    enter_town_facility(&mut game, id);
    for mutation_id in game.progress.active_mutation_ids.clone() {
        assert!(game.lose_mutation(&mutation_id, &mut Vec::new()));
    }
    game.gold = game.town_service_price(5_000);
    for locked in [false, true] {
        if locked {
            assert!(game.gain_mutation(mutation, &mut Vec::new()));
            game.progress
                .locked_mutation_ids
                .insert(mutation.to_owned());
        }
        let before = game.to_save();
        assert_eq!(
            game.use_town_facility_service(
                id,
                FacilityServiceKindDto::CureMutation,
                None,
                None,
                &mut Vec::new(),
            ),
            Err("no-curable-mutation")
        );
        assert_eq!(game.to_save(), before);
    }
    game.progress.locked_mutation_ids.clear();
    game.reveal_current_visibility();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    for run in [&mut game, &mut restored] {
        let update = dispatch_next(
            run,
            GameCommand::UseFacilityService {
                facility_id: id.to_owned(),
                service: FacilityServiceKindDto::CureMutation,
                item_id: None,
                enchantment_steps: None,
            },
        );
        assert!(
            update
                .events
                .iter()
                .any(|event| event.kind == "facility.mutation-cured")
        );
        assert!(run.progress.active_mutation_ids.is_empty());
        assert_eq!(run.gold, 0);
    }
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn zul_special_shops_buy_sell_restore_restock_and_reject_without_mutation() {
    let mut game = Game::new(42);
    enter_town(&mut game, "demo.town.zul", Position { x: 77, y: 6 });
    game.gold = 10_000_000;
    for id in ["demo.shop.zul-jeweler", "demo.shop.zul-dragonskin"] {
        let entrance = game.content.shop(id).unwrap().entrance_position;
        game.player.position = game
            .town_local_to_active_position("demo.town.zul", position_from_content(entrance))
            .unwrap();
        game.mark_shop_visited_at_player().unwrap();
        let snapshot = game.snapshot();
        let stock = projected_shop(&snapshot.shops, id).stock[0].clone();
        let before = game.state_hash();
        assert_eq!(
            game.buy_from_shop(id, &stock.id, 0),
            Err("invalid-quantity")
        );
        assert_eq!(game.state_hash(), before);
        let purchase = game.buy_from_shop(id, &stock.id, 1).unwrap();
        assert_eq!(purchase.unit_price, stock.unit_price);
        let acquired = game
            .items
            .iter()
            .find(|item| item.id == purchase.item_id)
            .unwrap()
            .clone();
        let mut restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        let bought_index = game
            .items
            .iter()
            .position(|item| item.id == acquired.id)
            .unwrap();
        game.items[bought_index].discount_percent = 77;
        assert!(
            Game::from_save(game.to_save()).is_err(),
            "unsupported shop discount must be rejected"
        );
        game.items[bought_index].discount_percent = acquired.discount_percent;
        let sale = game.sell_to_shop(id, &acquired.id, 1).unwrap();
        assert_eq!(restored.sell_to_shop(id, &acquired.id, 1).unwrap(), sale);
        assert_eq!(restored.state_hash(), game.state_hash());
        let stock = game
            .snapshot()
            .shops
            .into_iter()
            .find(|shop| shop.id == id)
            .unwrap()
            .stock[0]
            .clone();
        assert_eq!(
            game.buy_from_shop(id, &stock.id, 1).unwrap(),
            restored.buy_from_shop(id, &stock.id, 1).unwrap()
        );
        assert_eq!(restored.state_hash(), game.state_hash());
        game.gold = 0;
        let stock_id = game.shop_states[id].inventory[0].id.clone();
        let before = game.state_hash();
        assert_eq!(
            game.buy_from_shop(id, &stock_id, 1),
            Err("insufficient-gold")
        );
        assert_eq!(game.state_hash(), before);
        game.gold = 10_000_000;
        game.shop_states.get_mut(id).unwrap().inventory.clear();
        game.world_tick += 10_000;
        game.maintain_shop_at_player().unwrap();
        assert!(!game.shop_states[id].inventory.is_empty());
    }
}

#[test]
fn zul_desktop_preparation_uses_physical_arrival_and_preserves_save_boundaries() {
    let mut game = Game::new_with_build_race_and_name(
        42,
        "demo.build.mage-sorcery-nature",
        "rfb-legacy.race.beastman",
        "Zul desktop",
    )
    .unwrap();
    let before = game.to_save();
    assert!(game.debug_prepare_zul_e2e(false).is_err());
    assert!(game.debug_prepare_zul_e2e(true).is_err());
    assert_eq!(game.to_save(), before);
    game.debug_prepare_town_map_e2e("demo.town.zul").unwrap();
    assert_eq!(game.player.position, Position { x: 53, y: 32 });
    assert_eq!(game.current_town().unwrap().id, "demo.town.zul");
    game.debug_prepare_zul_e2e(false).unwrap();
    assert_eq!(game.player_incoming_damage_percent(), 0);
    assert!(
        game.facility_town_travel_destinations("demo.town-facility.zul-sorcery-tower")
            .is_empty()
    );
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn zul_high_level_generated_stock_restores_across_source_rolls() {
    let mut base = Game::new_with_build_race_and_name(
        42,
        "demo.build.mage-sorcery-nature",
        "rfb-legacy.race.beastman",
        "Zul stock",
    )
    .unwrap();
    base.debug_prepare_town_map_e2e("demo.town.zul").unwrap();
    base.debug_prepare_zul_e2e(false).unwrap();
    for seed in 0..64 {
        let mut game = base.clone();
        game.rng = RfbRng::seeded(seed);
        for id in ["demo.shop.zul-jeweler", "demo.shop.zul-dragonskin"] {
            game.player.position =
                position_from_content(game.content.shop(id).unwrap().entrance_position);
            game.mark_shop_visited_at_player().unwrap();
            let bytes = rfb_protocol::to_msgpack(&game.to_save()).unwrap();
            let payload = rfb_protocol::from_msgpack(&bytes).unwrap();
            let restored = Game::from_save(payload).unwrap_or_else(|error| {
                panic!(
                    "{id} seed {seed}: {error:?}; stock {:?}",
                    game.shop_states[id].inventory
                )
            });
            assert_eq!(restored.state_hash(), game.state_hash());
        }
    }
}

#[test]
fn zul_ordinary_shops_trade_independently_and_save_without_unlocking_teleport() {
    let mut game = Game::new(42);
    enter_town(&mut game, "demo.town.zul", Position { x: 77, y: 6 });
    assert_eq!(game.player.position, Position { x: 53, y: 32 });
    assert!(game.town_states["demo.town.zul"].visited);
    // Prepare funds and direct door positions; this is a business-state test, not a route run.
    game.gold = 1_000_000;
    for category in [
        "general-store",
        "weaponsmith",
        "temple",
        "alchemist",
        "magic-shop",
        "black-market",
        "bookstore",
    ] {
        let id = format!("demo.shop.zul-{category}");
        let shop = game.content.shop(&id).unwrap();
        game.player.position = game
            .town_local_to_active_position(
                "demo.town.zul",
                position_from_content(shop.entrance_position),
            )
            .unwrap();
        game.mark_shop_visited_at_player().unwrap();
        let before = game.shop_states.clone();
        let snapshot = game.snapshot();
        let item_id = projected_shop(&snapshot.shops, &id).stock[0].id.clone();
        let update = dispatch_next(
            &mut game,
            GameCommand::BuyFromShop {
                shop_id: id.clone(),
                item_id,
                quantity: 1,
            },
        );
        assert!(
            update
                .events
                .iter()
                .any(|event| event.kind == "shop.purchase")
        );
        for (other_id, state) in &before {
            if other_id != &id {
                assert_eq!(&game.shop_states[other_id], state);
            }
        }
    }
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    // Physical visitation must not grant the quest-77 teleport qualification.
    game.teleport_to_town("demo.town.outpost").unwrap();
    assert!(
        game.teleport_town_targets()
            .iter()
            .all(|target| target.town_id != "demo.town.zul")
    );
}

#[test]
fn at1_shop_doors_share_stock_transactions_projection_and_save() {
    let id = "demo.shop.anambar-general-store";
    let second = rfb_content::ContentPosition { x: 92, y: 46 };
    let mut game = Game::new(42);
    enter_town(&mut game, "demo.town.anambar", Position { x: 26, y: 39 });
    game.player.position = game
        .town_local_to_active_position("demo.town.anambar", position_from_content(second))
        .unwrap();
    game.mark_shop_visited_at_player().unwrap();
    game.gold = 10_000;
    let snapshot = game.snapshot();
    let shop = projected_shop(&snapshot.shops, id);
    assert!(shop.player_at_entrance);
    assert_eq!(shop.entrance_position, game.player.position);
    let item = shop.stock[0].clone();
    let update = dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: id.into(),
            item_id: item.id,
            quantity: 1,
        },
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "shop.purchase")
    );
    let stock = game.shop_states[id].clone();
    let rng = game.rng_draw_counter();
    game.player.position = game
        .town_local_to_active_position("demo.town.anambar", Position { x: 92, y: 45 })
        .unwrap();
    game.mark_shop_visited_at_player().unwrap();
    assert_eq!(game.shop_states[id], stock);
    assert_eq!(game.rng_draw_counter(), rng);
    assert_eq!(
        game.snapshot()
            .shops
            .iter()
            .filter(|shop| shop.id == id)
            .count(),
        1
    );
    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    let item = projected_shop(&game.snapshot().shops, id).stock[0]
        .id
        .clone();
    for current in [&mut game, &mut restored] {
        dispatch_next(
            current,
            GameCommand::BuyFromShop {
                shop_id: id.into(),
                item_id: item.clone(),
                quantity: 1,
            },
        );
    }
    assert_eq!(game.state_hash(), restored.state_hash());
    game.player.position = game
        .town_local_to_active_position("demo.town.anambar", position_from_content(second))
        .unwrap();
    game.shop_states.get_mut(id).unwrap().visited = false;
    assert!(Game::from_save_with_content(game.to_save(), game.content.clone()).is_err());
}

#[test]
fn at1_task_home_shared_cell_updates_service_and_preserves_town_across_save() {
    use rfb_content::{
        DungeonEntryTaskStatus as Status, TownTaskTerrainCaseDefinition as Case,
        TownTaskTerrainOverrideDefinition as Rule,
    };
    let task_id = "demo.task.anambar-cop-quest";
    let service_id = "demo.town-facility.anambar-police-station";
    let home = rfb_content::ContentPosition { x: 105, y: 57 };
    let mut game = at1_game(|content| {
        let floor = content.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.anambar")
            .unwrap();
        let map = floor.inline_map.as_mut().unwrap();
        map.task_terrain_overrides
            .retain(|rule| !rule.positions.contains(&home));
        map.task_terrain_overrides.push(Rule {
            positions: vec![home],
            default_terrain_id: "demo.terrain.permanent-wall".into(),
            cases: vec![
                Case {
                    task_id: task_id.into(),
                    statuses: vec![Status::Available],
                    terrain_id: "demo.terrain.anambar-cop-quest-entry-available".into(),
                },
                Case {
                    task_id: task_id.into(),
                    statuses: vec![Status::Taken, Status::Active],
                    terrain_id: "demo.terrain.anambar-cop-quest-entry".into(),
                },
                Case {
                    task_id: task_id.into(),
                    statuses: vec![Status::RewardAvailable],
                    terrain_id: "demo.terrain.permanent-wall".into(),
                },
                Case {
                    task_id: task_id.into(),
                    statuses: vec![Status::Completed],
                    terrain_id: "demo.terrain.home-entrance".into(),
                },
            ],
        });
        map.task_terrain_overrides.push(Rule {
            positions: vec![
                rfb_content::ContentPosition { x: 3, y: 5 },
                rfb_content::ContentPosition { x: 4, y: 5 },
            ],
            default_terrain_id: "demo.terrain.surface-grass".into(),
            cases: vec![Case {
                task_id: task_id.into(),
                statuses: vec![Status::Taken, Status::Active],
                terrain_id: "demo.terrain.permanent-wall".into(),
            }],
        });
    });
    enter_town(&mut game, "demo.town.anambar", Position { x: 26, y: 39 });
    let home_position = game
        .town_local_to_active_position("demo.town.anambar", position_from_content(home))
        .unwrap();
    game.player.position = home_position;
    assert!(!game.town_facility_accessible(ANAMBAR_HOME_ID));
    assert!(
        !game
            .snapshot()
            .homes
            .iter()
            .find(|home| home.id == ANAMBAR_HOME_ID)
            .unwrap()
            .player_at_entrance
    );
    let carried = game
        .items
        .iter()
        .find(|item| item.location == ItemLocation::Inventory)
        .unwrap()
        .id
        .clone();
    assert!(game.deposit_at_home(ANAMBAR_HOME_ID, &carried, 1).is_err());
    let unchanged_position = game
        .town_local_to_active_position("demo.town.anambar", Position { x: 5, y: 5 })
        .unwrap();
    let unchanged_index = game.index(unchanged_position).unwrap();
    game.terrain[unchanged_index] = "demo.terrain.door-open".into();
    game.player.position = game
        .town_facility_entrance_position(game.content.town_facility(service_id).unwrap())
        .unwrap();
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: service_id.into(),
            task_id: task_id.into(),
        },
    );
    assert_eq!(
        game.terrain_at(unchanged_position),
        "demo.terrain.door-open"
    );
    for x in [3, 4] {
        let position = game
            .town_local_to_active_position("demo.town.anambar", Position { x, y: 5 })
            .unwrap();
        assert_eq!(game.terrain_at(position), "demo.terrain.permanent-wall");
    }
    game.player.position = home_position;
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, "demo.floor.anambar-cop-quest");
    // This test exercises town state transitions; combat is covered by the existing task tests.
    super::support::clear_monsters(&mut game);
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(
        game.task_states[task_id].status,
        TaskStatusKindDto::RewardAvailable
    );
    let exit = game
        .terrain
        .iter()
        .position(|terrain| terrain == "demo.terrain.stairs-up")
        .unwrap();
    game.player.position = Position {
        x: (exit % usize::from(game.width)) as i32,
        y: (exit / usize::from(game.width)) as i32,
    };
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.player.position, home_position);
    assert_eq!(game.terrain_at(home_position), "demo.terrain.surface-grass");
    assert!(!game.town_facility_accessible(ANAMBAR_HOME_ID));
    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    for current in [&mut game, &mut restored] {
        current.player.position = current
            .town_facility_entrance_position(current.content.town_facility(service_id).unwrap())
            .unwrap();
        dispatch_next(
            current,
            GameCommand::ClaimTaskReward {
                facility_id: service_id.into(),
                task_id: task_id.into(),
            },
        );
        current.player.position = home_position;
        current.mark_shop_visited_at_player().unwrap();
        assert!(current.town_facility_accessible(ANAMBAR_HOME_ID));
        assert!(
            current
                .snapshot()
                .homes
                .iter()
                .find(|home| home.id == ANAMBAR_HOME_ID)
                .unwrap()
                .player_at_entrance
        );
    }
    assert_eq!(game.state_hash(), restored.state_hash());
    game.deposit_at_home(ANAMBAR_HOME_ID, &carried, 1).unwrap();
    assert_eq!(
        game.terrain_at(unchanged_position),
        "demo.terrain.door-open"
    );
    enter_town(&mut game, "demo.town.outpost", Position { x: 28, y: 52 });
    enter_town(&mut game, "demo.town.anambar", Position { x: 26, y: 39 });
    assert_eq!(
        game.terrain_at(unchanged_position),
        "demo.terrain.door-open"
    );
    assert_eq!(game.terrain_at(home_position), "demo.terrain.home-entrance");
    assert!(Game::from_save_with_content(game.to_save(), game.content.clone()).is_ok());
}

fn enter_morivant(game: &mut Game) {
    enter_town(game, MORIVANT_TOWN_ID, Position { x: 47, y: 50 });
}

#[test]
fn at2_anambar_far_task_returns_to_scrolled_entrance_and_preserves_ground_item() {
    let mut game = Game::new(202);
    enter_town(&mut game, "demo.town.anambar", Position { x: 26, y: 39 });
    let mayor_id = "demo.town-facility.anambar-mayor-office";
    let task_id = "demo.task.anambar-orc-camp";
    game.player.position = Position { x: 110, y: 28 }; // Fourth source door.
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: mayor_id.into(),
            task_id: task_id.into(),
        },
    );
    assert_eq!(game.task_states[task_id].status, TaskStatusKindDto::Taken);
    game.player.position = Position { x: 100, y: 41 };
    support::give_inventory_item(&mut game, "test.anambar.floor-item", "demo.item.dagger");
    game.drop_inventory_quantity("test.anambar.floor-item", 1)
        .unwrap()
        .unwrap();
    let item = game
        .items
        .iter()
        .find(|item| item.location == ItemLocation::Ground(game.player.position))
        .unwrap()
        .clone();
    // Cross the real east scroll boundary on the source road, then approach the far entrance.
    game.player.position = Position { x: 131, y: 41 };
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_ne!(game.wilderness_view_offset, Position { x: 0, y: 0 });
    let entry = game
        .town_local_to_active_position("demo.town.anambar", Position { x: 183, y: 62 })
        .unwrap();
    assert_eq!(
        game.terrain_at(entry),
        "demo.terrain.anambar-orc-camp-entry"
    );
    game.player.position = entry;
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, "demo.floor.anambar-orc-camp");
    dispatch_next(&mut game, GameCommand::AbandonTask);
    assert_eq!(
        game.task_states[task_id].status,
        TaskStatusKindDto::Abandoned
    );
    assert_eq!(
        game.player.position,
        game.town_local_to_active_position("demo.town.anambar", Position { x: 183, y: 62 })
            .unwrap()
    );
    assert_eq!(game.terrain_at(game.player.position), "demo.terrain.dirt");
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    restored.teleport_to_town("demo.town.outpost").unwrap();
    restored.teleport_to_town("demo.town.anambar").unwrap();
    let mut expected = item;
    expected.location = ItemLocation::Ground(
        restored
            .town_local_to_active_position("demo.town.anambar", Position { x: 100, y: 41 })
            .unwrap(),
    );
    assert!(restored.items.contains(&expected));
    assert_eq!(
        restored.terrain_at(Position { x: 183, y: 62 }),
        "demo.terrain.dirt"
    );
    assert!(Game::from_save(restored.to_save()).is_ok());
}

#[test]
fn at2_anambar_dinosaur_failure_rolls_once_and_keeps_the_actor_after_save_and_travel() {
    let task_id = "demo.task.anambar-dinosaur-quest";
    let mayor_id = "demo.town-facility.anambar-mayor-office";
    let mut game = Game::new(203);
    enter_town(&mut game, "demo.town.anambar", Position { x: 26, y: 39 });
    // Explicit prerequisite setup; the test concerns the actual failure return.
    for id in [
        "demo.task.anambar-orc-camp",
        "demo.task.anambar-clear-tunnels",
        "demo.task.anambar-scary-rock-treasure",
    ] {
        if let Some(state) = game.task_states.get_mut(id) {
            state.status = TaskStatusKindDto::Completed;
            state.current = state.required;
        }
    }
    game.player.position = Position { x: 107, y: 28 };
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: mayor_id.into(),
            task_id: task_id.into(),
        },
    );
    game.player.position = Position { x: 78, y: 25 };
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, "demo.floor.anambar-dinosaur-quest");
    let before = game.to_save();
    for should_spawn in [false, true] {
        let mut current = Game::from_save(before.clone()).unwrap();
        // Select either side of the real 33% draw without changing formal content.
        let seed = (0..1000)
            .find(|seed| (RfbRng::seeded(*seed).bounded(100) < 33) == should_spawn)
            .unwrap();
        current.rng = RfbRng::seeded(seed);
        dispatch_next(&mut current, GameCommand::AbandonTask);
        assert_eq!(
            current.task_states[task_id].status,
            TaskStatusKindDto::Abandoned
        );
        let actor_id = format!("{task_id}.failure-return");
        let actor = current
            .entities
            .iter()
            .find(|actor| actor.id == actor_id)
            .cloned();
        assert_eq!(actor.is_some(), should_spawn);
        if let Some(actor) = &actor {
            assert_eq!(actor.kind_id, "demo.actor.triceratops");
            assert_eq!(actor.position, Position { x: 93, y: 26 });
        }
        let rng = current.rng_draw_counter();
        let _ = current.snapshot();
        let _ = current.snapshot();
        assert_eq!(current.rng_draw_counter(), rng);
        let mut restored = Game::from_save(current.to_save()).unwrap();
        assert_eq!(restored.state_hash(), current.state_hash());
        for copy in [&mut current, &mut restored] {
            copy.teleport_to_town("demo.town.outpost").unwrap();
            copy.teleport_to_town("demo.town.anambar").unwrap();
            assert_eq!(
                copy.entities
                    .iter()
                    .find(|candidate| candidate.id == actor_id),
                actor.as_ref()
            );
        }
        assert_eq!(restored.state_hash(), current.state_hash());
    }
}

#[test]
fn at3_thalos_museum_closes_for_dark_academy_and_restores_its_collection_on_success() {
    let task_id = "demo.task.thalos-dark-academy";
    let academy = "demo.town-facility.thalos-royal-academy";
    let mut game = [10, 11]
        .into_iter()
        .map(thalos_game)
        .find(|game| game.task_states.contains_key(task_id))
        .unwrap();
    game.player.position = Position { x: 86, y: 50 };
    game.mark_shop_visited_at_player().unwrap();
    support::give_inventory_item(&mut game, "test.thalos.museum-item", "demo.item.dagger");
    dispatch_next(
        &mut game,
        GameCommand::DepositAtHome {
            facility_id: THALOS_MUSEUM_ID.into(),
            item_id: "test.thalos.museum-item".into(),
            quantity: 1,
        },
    );
    let collection = game.home_states[THALOS_MUSEUM_ID].clone();
    let stored_item = game
        .snapshot()
        .homes
        .into_iter()
        .find(|home| home.id == THALOS_MUSEUM_ID)
        .unwrap()
        .stored_items[0]
        .id
        .clone();
    // Prepare only the prior academy quests; entrance, completion and rewards use real commands.
    prepare_thalos_completed_tasks(
        &mut game,
        &["mushrooms", "tidy-laboratory", "staff-recovery-first"],
    );
    game.player.position = Position { x: 55, y: 31 };
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: academy.into(),
            task_id: task_id.into(),
        },
    );
    assert_eq!(game.task_states[task_id].status, TaskStatusKindDto::Taken);
    game.player.position = Position { x: 86, y: 50 };
    assert_eq!(
        game.terrain_at(game.player.position),
        "demo.terrain.thalos-dark-academy-entry"
    );
    assert!(!game.town_facility_accessible(THALOS_MUSEUM_ID));
    assert!(
        !game
            .snapshot()
            .homes
            .into_iter()
            .find(|home| home.id == THALOS_MUSEUM_ID)
            .unwrap()
            .player_at_entrance
    );
    let rejected = dispatch_next(
        &mut game,
        GameCommand::WithdrawFromHome {
            facility_id: THALOS_MUSEUM_ID.into(),
            item_id: stored_item.clone(),
            quantity: 1,
        },
    );
    assert_eq!(rejected.events[0].kind, "home.transfer-unavailable");
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, "demo.floor.thalos-dark-academy");
    let inside = game.to_save();
    // The persistent museum remains closed after either unsuccessful departure.
    for abandon in [false, true] {
        let mut failed = Game::from_save(inside.clone()).unwrap();
        if abandon {
            dispatch_next(&mut failed, GameCommand::AbandonTask);
        } else {
            support::place_player_on_terrain(&mut failed, "demo.terrain.stairs-up");
            dispatch_next(&mut failed, GameCommand::TraverseStairs);
        }
        assert_eq!(
            failed.task_states[task_id].status,
            if abandon {
                TaskStatusKindDto::Abandoned
            } else {
                TaskStatusKindDto::Failed
            }
        );
        failed.player.position = Position { x: 87, y: 50 };
        dispatch_next(&mut failed, GameCommand::Wait);
        assert_eq!(
            failed.terrain_at(Position { x: 86, y: 50 }),
            "demo.terrain.permanent-wall"
        );
        assert!(!failed.town_facility_accessible(THALOS_MUSEUM_ID));
        failed = Game::from_save(failed.to_save()).unwrap();
        failed.teleport_to_town("demo.town.outpost").unwrap();
        failed.teleport_to_town("demo.town.thalos").unwrap();
        assert_eq!(
            failed.terrain_at(Position { x: 86, y: 50 }),
            "demo.terrain.permanent-wall"
        );
        assert_eq!(failed.home_states[THALOS_MUSEUM_ID], collection);
    }
    support::clear_monsters(&mut game);
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(
        game.task_states[task_id].status,
        TaskStatusKindDto::RewardAvailable
    );
    support::place_player_on_terrain(&mut game, "demo.terrain.stairs-up");
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.player.position, Position { x: 86, y: 50 });
    assert!(!game.town_facility_accessible(THALOS_MUSEUM_ID));
    game.player.position = Position { x: 55, y: 31 };
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(
        game.terrain_at(Position { x: 86, y: 50 }),
        "demo.terrain.permanent-wall"
    );
    game = Game::from_save(game.to_save()).unwrap();
    dispatch_next(
        &mut game,
        GameCommand::ClaimTaskReward {
            facility_id: academy.into(),
            task_id: task_id.into(),
        },
    );
    assert_eq!(
        game.task_states[task_id].status,
        TaskStatusKindDto::Completed
    );
    assert_eq!(
        game.terrain_at(Position { x: 86, y: 50 }),
        "demo.terrain.museum-entrance"
    );
    game.player.position = Position { x: 86, y: 50 };
    assert!(game.town_facility_accessible(THALOS_MUSEUM_ID));
    assert_eq!(game.home_states[THALOS_MUSEUM_ID], collection);
    dispatch_next(
        &mut game,
        GameCommand::WithdrawFromHome {
            facility_id: THALOS_MUSEUM_ID.into(),
            item_id: stored_item,
            quantity: 1,
        },
    );
    assert!(game.home_states[THALOS_MUSEUM_ID].inventory.is_empty());
    assert!(Game::from_save(game.to_save()).is_ok());
}

#[test]
fn at3_thalos_staff_variants_share_one_door_without_closing_the_other_branch_museum() {
    let mut variants = BTreeSet::new();
    for seed in [10, 11] {
        let mut game = thalos_game(seed);
        let task_id = if game
            .task_states
            .contains_key("demo.task.thalos-staff-recovery")
        {
            "demo.task.thalos-staff-recovery"
        } else {
            "demo.task.thalos-staff-recovery-first"
        };
        variants.insert(task_id);
        prepare_thalos_completed_tasks(
            &mut game,
            &["mushrooms", "tidy-laboratory", "basilisk-cave"],
        );
        game.player.position = Position { x: 55, y: 31 };
        dispatch_next(
            &mut game,
            GameCommand::AcceptTask {
                facility_id: "demo.town-facility.thalos-royal-academy".into(),
                task_id: task_id.into(),
            },
        );
        assert_eq!(
            game.terrain_at(Position { x: 86, y: 50 }),
            "demo.terrain.museum-entrance"
        );
        game.player.position = Position { x: 60, y: 25 };
        assert_eq!(
            game.terrain_at(game.player.position),
            task_id.replace("demo.task.", "demo.terrain.") + "-entry"
        );
        game = Game::from_save(game.to_save()).unwrap();
        dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(
            game.current_floor_id,
            task_id.replace("demo.task.", "demo.floor.")
        );
        dispatch_next(&mut game, GameCommand::AbandonTask);
        assert_eq!(game.player.position, Position { x: 60, y: 25 });
        assert_eq!(game.terrain_at(game.player.position), "demo.terrain.floor");
    }
    assert_eq!(variants.len(), 2);
}

#[test]
fn at3_thalos_tower_cells_follow_palace_conclusion_and_sorcerer_return() {
    let mut game = thalos_game(10);
    let fairy = "demo.task.thalos-shadow-fairies";
    let sorcerer = "demo.task.thalos-renegade-sorcerer";
    let palace = "demo.town-facility.thalos-palace";
    let academy = "demo.town-facility.thalos-royal-academy";
    let tower = [
        (
            "surface-grass",
            vec![
                (156, 57),
                (157, 57),
                (155, 58),
                (156, 58),
                (157, 58),
                (158, 58),
                (157, 59),
                (158, 59),
            ],
        ),
        ("surface-brake", vec![(155, 59), (156, 59), (156, 60)]),
        ("surface-flower", vec![(157, 60)]),
    ];
    let assert_tower = |game: &Game, natural: bool| {
        for (terrain, cells) in &tower {
            for &(x, y) in cells {
                let position = game
                    .town_local_to_active_position("demo.town.thalos", Position { x, y })
                    .unwrap();
                assert_eq!(
                    game.terrain_at(position),
                    if natural {
                        format!("demo.terrain.{terrain}")
                    } else {
                        "demo.terrain.permanent-wall".into()
                    },
                    "{x},{y}"
                );
            }
        }
    };
    assert_tower(&game, true);
    game.player.position = Position { x: 21, y: 41 }; // Fourth palace door.
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: palace.into(),
            task_id: fairy.into(),
        },
    );
    assert_eq!(game.task_states[fairy].status, TaskStatusKindDto::Taken);
    assert_tower(&game, true);
    game.player.position = Position { x: 131, y: 39 };
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(game.wilderness_view_offset, Position { x: 1, y: 0 });
    game.player.position = game
        .town_local_to_active_position("demo.town.thalos", Position { x: 141, y: 23 })
        .unwrap();
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, "demo.floor.thalos-shadow-fairies");
    support::clear_monsters(&mut game);
    dispatch_next(&mut game, GameCommand::Wait);
    support::place_player_on_terrain(&mut game, "demo.terrain.stairs-up");
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(
        game.player.position,
        game.town_local_to_active_position("demo.town.thalos", Position { x: 141, y: 23 })
            .unwrap()
    );
    assert_tower(&game, true);
    game.teleport_to_town("demo.town.outpost").unwrap();
    game.teleport_to_town("demo.town.thalos").unwrap();
    game.player.position = Position { x: 21, y: 38 };
    dispatch_next(
        &mut game,
        GameCommand::ClaimTaskReward {
            facility_id: palace.into(),
            task_id: fairy.into(),
        },
    );
    assert_eq!(game.task_states[fairy].status, TaskStatusKindDto::Completed);
    assert_tower(&game, false);
    // Academy prerequisite battles are outside this map-state test.
    prepare_thalos_completed_tasks(
        &mut game,
        &[
            "mushrooms",
            "tidy-laboratory",
            "basilisk-cave",
            "staff-recovery-first",
            "staff-recovery",
            "dark-academy",
        ],
    );
    game.player.position = Position { x: 55, y: 31 };
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: academy.into(),
            task_id: sorcerer.into(),
        },
    );
    game.player.position = Position { x: 157, y: 60 };
    assert_eq!(
        game.terrain_at(game.player.position),
        "demo.terrain.thalos-renegade-sorcerer-entry"
    );
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, "demo.floor.thalos-renegade-sorcerer");
    let mut failed = Game::from_save(game.to_save()).unwrap();
    dispatch_next(&mut failed, GameCommand::AbandonTask);
    failed.player.position = Position { x: 156, y: 61 };
    dispatch_next(&mut failed, GameCommand::Wait);
    assert_tower(&failed, false);
    failed = Game::from_save(failed.to_save()).unwrap();
    assert_tower(&failed, false);
    // Prepare the sorcerer kill result; this test verifies return and conclusion geometry.
    support::clear_monsters(&mut game);
    let state = game.task_states.get_mut(sorcerer).unwrap();
    state.current = state.required;
    state.status = TaskStatusKindDto::RewardAvailable;
    state.active_floor_id = None;
    dispatch_next(&mut game, GameCommand::Wait);
    support::place_player_on_terrain(&mut game, "demo.terrain.stairs-up");
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    game.player.position = Position { x: 55, y: 31 };
    dispatch_next(
        &mut game,
        GameCommand::ClaimTaskReward {
            facility_id: academy.into(),
            task_id: sorcerer.into(),
        },
    );
    assert_eq!(
        game.task_states[sorcerer].status,
        TaskStatusKindDto::Completed
    );
    assert_tower(&game, true);
    let mut restored = Game::from_save(game.to_save()).unwrap();
    for current in [&mut game, &mut restored] {
        current.teleport_to_town("demo.town.outpost").unwrap();
        current.teleport_to_town("demo.town.thalos").unwrap();
        assert_tower(current, true);
    }
    assert_eq!(game.state_hash(), restored.state_hash());
}

fn prepare_thalos_completed_tasks(game: &mut Game, slugs: &[&str]) {
    for slug in slugs {
        let id = format!("demo.task.thalos-{slug}");
        if let Some(mut state) = crate::game::tasks::projected_task_state(
            game.content.world(&game.world_id).unwrap(),
            &game.task_states,
            &id,
        ) {
            state.status = TaskStatusKindDto::Completed;
            state.current = state.required;
            game.task_states.insert(id, state);
        }
    }
}

#[test]
fn zul_eddies_entry_loot_return_reward_and_town_travel_survive_save() {
    let tower = "demo.town-facility.zul-sorcery-tower";
    let task = "demo.task.zul-eddies";
    let mut game = town_facility_game(42, "demo.build.mage-sorcery-nature", tower);
    game.debug_prepare_spell_learning_e2e(50).unwrap();
    support::choose_human_talent_if_pending(&mut game);
    support::clear_monsters(&mut game);
    game.apply_player_melee_status(STATUS_INVULNERABILITY, 1000, "test.task-lifecycle");
    let entry = game
        .town_local_to_active_position("demo.town.zul", Position { x: 91, y: 32 })
        .unwrap();
    assert_eq!(game.terrain_at(entry), "demo.terrain.surface-grass");
    assert!(game.facility_town_travel_destinations(tower).is_empty());
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: tower.into(),
            task_id: task.into(),
        },
    );
    assert_eq!(game.terrain_at(entry), "demo.terrain.zul-eddies-entry");
    game.player.position = entry;
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, "demo.floor.zul-eddies");
    assert_eq!((game.width, game.height), (29, 37));
    assert_eq!(game.entities.len(), 25);
    assert_eq!(game.vault_cells.iter().filter(|cell| **cell).count(), 7);
    let ground = game
        .items
        .iter()
        .filter(|item| matches!(item.location, ItemLocation::Ground(_)))
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(ground.len(), 2);
    for item in &ground {
        assert_eq!(item.affix_ids, ["rfb-legacy.affix.the-bat"]);
        assert_eq!(
            game.content
                .item(&item.kind_id)
                .unwrap()
                .rfb_base_kind
                .unwrap()
                .tval,
            35
        );
        assert!(matches!(
            item.location,
            ItemLocation::Ground(Position { x: 6 | 22, y: 32 })
        ));
        assert!(item.artifact_name.is_none());
        assert!(!item.rolled_affixes.is_empty());
    }
    game = Game::from_save(game.to_save()).unwrap();
    assert_eq!(
        game.items
            .iter()
            .filter(|item| matches!(item.location, ItemLocation::Ground(_)))
            .cloned()
            .collect::<Vec<_>>(),
        ground
    );
    // Prepare the result explicitly: this tests lifecycle/settlement, not natural combat.
    game.entities.clear();
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(
        game.task_states[task].status,
        TaskStatusKindDto::RewardAvailable
    );
    let ItemLocation::Ground(cloak_position) = ground[0].location else {
        panic!("ground cloak");
    };
    game.player.position = cloak_position;
    dispatch_next(&mut game, GameCommand::PickUp);
    game.player.position = Position { x: 14, y: 35 };
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_town().unwrap().id, "demo.town.zul");
    assert_eq!(game.player.position, entry);
    assert_eq!(game.terrain_at(entry), "demo.terrain.surface-grass");
    assert!(!game.stored_floors.contains_key("demo.floor.zul-eddies"));
    assert!(
        game.items
            .iter()
            .any(|item| item.id == ground[0].id && item.location == ItemLocation::Inventory)
    );
    assert!(!game.items.iter().any(|item| item.id == ground[1].id));
    game.player.position = game
        .town_local_to_active_position("demo.town.zul", Position { x: 65, y: 16 })
        .unwrap();
    assert!(game.facility_town_travel_destinations(tower).is_empty());
    dispatch_next(
        &mut game,
        GameCommand::ClaimTaskReward {
            facility_id: tower.into(),
            task_id: task.into(),
        },
    );
    assert_eq!(game.task_states[task].status, TaskStatusKindDto::Completed);
    let reward = game
        .items
        .iter()
        .find(|item| item.id == "demo.task.zul-eddies.reward.1")
        .unwrap();
    assert!(game.generated_artifact_ids.contains(&reward.kind_id));
    let destinations = game.facility_town_travel_destinations(tower);
    let outpost = destinations
        .iter()
        .find(|destination| destination.town_id == "demo.town.outpost")
        .unwrap();
    assert_eq!(outpost.cost, game.town_service_price(200));
    game.gold = outpost.cost - 1;
    let before = game.to_save();
    assert_eq!(
        game.inn_travel_unavailable_reason(tower, "demo.town.outpost"),
        Some("insufficient-gold")
    );
    assert_eq!(game.to_save(), before);
    game.gold = 10_000;
    dispatch_next(
        &mut game,
        GameCommand::TravelFromInn {
            facility_id: tower.into(),
            destination_town_id: "demo.town.outpost".into(),
        },
    );
    assert!(projected_shop(&game.snapshot().shops, WHITE_HORSE_INN_ID).player_at_entrance);
    assert!(game.teleport_town_target_available("demo.town.zul"));
    let snapshot = game.snapshot();
    assert!(
        projected_shop(&snapshot.shops, WHITE_HORSE_INN_ID)
            .inn_travel_destinations
            .iter()
            .any(|destination| destination.town_id == "demo.town.zul")
    );
    game = Game::from_save(game.to_save()).unwrap();
    dispatch_next(
        &mut game,
        GameCommand::TravelFromInn {
            facility_id: WHITE_HORSE_INN_ID.into(),
            destination_town_id: "demo.town.zul".into(),
        },
    );
    assert_eq!(
        game.player.position,
        game.town_local_to_active_position("demo.town.zul", Position { x: 65, y: 16 })
            .unwrap()
    );
    assert!(
        game.current_task_service_dtos()
            .iter()
            .any(|service| service.id == tower && service.player_at_entrance)
    );
    let before = game.to_save();
    assert_eq!(
        game.claim_task_reward(tower, task),
        Err("reward-unavailable")
    );
    assert_eq!(game.to_save(), before);
}

#[test]
fn zul_eddies_failure_and_abandonment_never_unlock_town_teleport() {
    let tower = "demo.town-facility.zul-sorcery-tower";
    let task = "demo.task.zul-eddies";
    for abandon in [false, true] {
        let mut game = town_facility_game(42, "demo.build.warrior", tower);
        support::clear_monsters(&mut game);
        dispatch_next(
            &mut game,
            GameCommand::AcceptTask {
                facility_id: tower.into(),
                task_id: task.into(),
            },
        );
        let entry = game
            .town_local_to_active_position("demo.town.zul", Position { x: 91, y: 32 })
            .unwrap();
        game.player.position = entry;
        dispatch_next(&mut game, GameCommand::TraverseStairs);
        support::clear_monsters(&mut game);
        dispatch_next(
            &mut game,
            if abandon {
                GameCommand::AbandonTask
            } else {
                GameCommand::TraverseStairs
            },
        );
        assert_eq!(
            game.task_states[task].status,
            if abandon {
                TaskStatusKindDto::Abandoned
            } else {
                TaskStatusKindDto::Failed
            }
        );
        assert_eq!(game.player.position, entry);
        assert_eq!(game.terrain_at(entry), "demo.terrain.surface-grass");
        game = Game::from_save(game.to_save()).unwrap();
        game.player.position = game
            .town_local_to_active_position("demo.town.zul", Position { x: 65, y: 16 })
            .unwrap();
        assert!(game.facility_town_travel_destinations(tower).is_empty());
        assert_eq!(game.accept_task(tower, task), Err("task-already-taken"));
        game.teleport_to_town("demo.town.outpost").unwrap();
        assert!(!game.teleport_town_target_available("demo.town.zul"));
        assert_eq!(
            game.inn_travel_unavailable_reason(WHITE_HORSE_INN_ID, "demo.town.zul"),
            Some("town-unvisited")
        );
    }
}

#[test]
fn zul_eddies_reward_choice_is_durable_and_generated_artifacts_are_replaced() {
    let tower = "demo.town-facility.zul-sorcery-tower";
    let task = "demo.task.zul-eddies";
    let mut seen = BTreeSet::new();
    for seed in 0..24 {
        let mut game = town_facility_game(seed, "demo.build.warrior", tower);
        support::clear_monsters(&mut game);
        game.task_states.insert(
            task.into(),
            TaskState {
                status: TaskStatusKindDto::RewardAvailable,
                stage_index: 0,
                current: 1,
                required: 1,
                active_floor_id: None,
                retakes_used: 0,
            },
        );
        game.reveal_current_visibility();
        let save = game.to_save();
        game.claim_task_reward(tower, task).unwrap();
        let kind = game
            .items
            .iter()
            .find(|item| item.id == "demo.task.zul-eddies.reward.1")
            .unwrap()
            .kind_id
            .clone();
        if kind == "demo.item.visiting-team" {
            let reward = game
                .items
                .iter()
                .find(|item| item.id == "demo.task.zul-eddies.reward.1")
                .unwrap();
            assert!(game.item_has_weapon_trait(reward, rfb_protocol::WeaponTraitDto::Stun));
        }
        seen.insert(kind.clone());
        assert_eq!(
            game.facility_town_travel_destinations(tower)[0].cost,
            game.town_service_price(500)
        );
        let mut restored = Game::from_save(save.clone()).unwrap();
        let original_items = restored.items.clone();
        for index in 0..restored.inventory_slot_capacity() {
            support::give_inventory_item(
                &mut restored,
                &format!("test.zul.full.{index}"),
                "demo.item.dagger",
            );
        }
        let full = restored.to_save();
        assert_eq!(
            restored.claim_task_reward(tower, task),
            Err("inventory-full")
        );
        assert_eq!(restored.to_save(), full);
        restored.items = original_items;
        for _ in 0..10 {
            restored.rng.bounded(100);
        }
        restored.claim_task_reward(tower, task).unwrap();
        assert_eq!(
            restored
                .items
                .iter()
                .find(|item| item.id == "demo.task.zul-eddies.reward.1")
                .unwrap()
                .kind_id,
            kind
        );
        let mut duplicate = Game::from_save(save).unwrap();
        duplicate.generated_artifact_ids.insert(kind);
        duplicate.claim_task_reward(tower, task).unwrap();
        let replacement = duplicate
            .items
            .iter()
            .find(|item| item.id == "demo.task.zul-eddies.reward.1")
            .unwrap();
        assert_eq!(replacement.kind_id, "demo.item.baseball-bat");
        assert!(replacement.artifact_name.is_some());
        assert!(
            duplicate
                .content
                .item(&replacement.kind_id)
                .unwrap()
                .artifact_generation
                .is_none()
        );
        if seen.len() == 3 {
            break;
        }
    }
    assert_eq!(
        seen,
        BTreeSet::from([
            "demo.item.sotkamo".to_owned(),
            "demo.item.visiting-team".to_owned(),
            "demo.item.superbat".to_owned()
        ])
    );
}

fn prepare_zul_task_status(game: &mut Game, task_id: &str, status: TaskStatusKindDto) {
    let mut state = crate::game::tasks::projected_task_state(
        game.content.world(&game.world_id).unwrap(),
        &game.task_states,
        task_id,
    )
    .unwrap();
    state.status = status;
    if matches!(
        status,
        TaskStatusKindDto::Completed | TaskStatusKindDto::RewardAvailable
    ) {
        state.current = state.required;
    }
    game.task_states.insert(task_id.to_owned(), state);
}

#[test]
fn zul_node_membership_and_eddies_terminal_prerequisite_are_independent() {
    let tower = "demo.town-facility.zul-sorcery-tower";
    let node = "demo.task.zul-sorcery-node";
    for prerequisite in [
        TaskStatusKindDto::Available,
        TaskStatusKindDto::Taken,
        TaskStatusKindDto::RewardAvailable,
        TaskStatusKindDto::Completed,
        TaskStatusKindDto::Failed,
        TaskStatusKindDto::Abandoned,
    ] {
        let mut game = town_facility_game(42, "demo.build.mage-sorcery-nature", tower);
        prepare_zul_task_status(&mut game, "demo.task.zul-eddies", prerequisite);
        let allowed = matches!(
            prerequisite,
            TaskStatusKindDto::Completed | TaskStatusKindDto::Failed | TaskStatusKindDto::Abandoned
        );
        let before = game.to_save();
        assert_eq!(
            game.accept_task(tower, node).is_ok(),
            allowed,
            "{prerequisite:?}"
        );
        if !allowed {
            assert_eq!(game.to_save(), before);
        }
        game.teleport_to_town("demo.town.outpost").unwrap();
        assert_eq!(
            game.teleport_town_target_available("demo.town.zul"),
            prerequisite == TaskStatusKindDto::Completed
        );
    }
    for realm in ["sorcery", "chaos", "nature"] {
        let tower = format!("demo.town-facility.zul-{realm}-tower");
        let task = format!("demo.task.zul-{realm}-node");
        let mut visitor = town_facility_game(42, "demo.build.warrior", &tower);
        prepare_zul_task_status(
            &mut visitor,
            "demo.task.zul-eddies",
            TaskStatusKindDto::Completed,
        );
        let before = visitor.to_save();
        assert_eq!(
            visitor.accept_task(&tower, &task),
            Err("task-membership-required")
        );
        assert_eq!(visitor.to_save(), before);
        prepare_zul_task_status(&mut visitor, &task, TaskStatusKindDto::RewardAvailable);
        let before = visitor.to_save();
        assert_eq!(
            visitor.claim_task_reward(&tower, &task),
            Err("task-membership-required")
        );
        assert_eq!(visitor.to_save(), before);
    }
}

#[test]
fn zul_nodes_enter_save_complete_return_and_deliver_each_source_book_once() {
    for (realm, entry, start, count, book) in [
        (
            "sorcery",
            Position { x: 78, y: 44 },
            Position { x: 1, y: 23 },
            85,
            "grimoire-of-power",
        ),
        (
            "chaos",
            Position { x: 9, y: 1 },
            Position { x: 1, y: 23 },
            97,
            "armageddon-tome",
        ),
        (
            "nature",
            Position { x: 61, y: 4 },
            Position { x: 2, y: 24 },
            88,
            "natures-wrath",
        ),
    ] {
        let tower = format!("demo.town-facility.zul-{realm}-tower");
        let task = format!("demo.task.zul-{realm}-node");
        // Beastman is a Chaos Member; the current Mage realms own the other two towers.
        let mut game = Game::new_with_build_race_and_name(
            42,
            "demo.build.mage-sorcery-nature",
            "rfb-legacy.race.beastman",
            "Nodes",
        )
        .unwrap();
        enter_town_facility(&mut game, &tower);
        support::clear_monsters(&mut game);
        prepare_zul_task_status(&mut game, "demo.task.zul-eddies", TaskStatusKindDto::Failed);
        game.apply_player_melee_status(STATUS_INVULNERABILITY, 1000, "test.node-lifecycle");
        dispatch_next(
            &mut game,
            GameCommand::AcceptTask {
                facility_id: tower.clone(),
                task_id: task.clone(),
            },
        );
        let entry = game
            .town_local_to_active_position("demo.town.zul", entry)
            .unwrap();
        assert_eq!(
            game.terrain_at(entry),
            format!("demo.terrain.zul-{realm}-node-entry")
        );
        game.player.position = entry;
        dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(
            game.current_floor_id,
            format!("demo.floor.zul-{realm}-node")
        );
        let companions = game
            .entities
            .iter()
            .filter(|actor| actor.id.contains(".companion."))
            .count();
        assert_eq!(game.entities.len() - companions, count);
        if realm == "nature" {
            assert!(companions > 0, "source war bears allow FRIENDS(1d7)");
            assert!(
                game.entities
                    .iter()
                    .filter(|actor| actor.id.contains(".companion."))
                    .all(|actor| actor.kind_id == "demo.actor.war-bear" && actor.pack.is_some())
            );
        } else {
            assert_eq!(
                companions, 0,
                "source phantom warriors and hell hounds have NO_GROUP"
            );
        }
        game.reveal_current_visibility();
        let mut restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        let mut expected_entities = game.entities.clone();
        expected_entities.sort_by(|left, right| left.id.cmp(&right.id));
        assert_eq!(restored.entities, expected_entities);
        // Explicitly prepare success; this is lifecycle coverage, not natural combat acceptance.
        support::clear_monsters(&mut restored);
        dispatch_next(&mut restored, GameCommand::Wait);
        assert_eq!(
            restored.task_states[&task].status,
            TaskStatusKindDto::RewardAvailable
        );
        restored.player.position = start;
        dispatch_next(&mut restored, GameCommand::TraverseStairs);
        assert_eq!(restored.player.position, entry);
        assert_eq!(restored.current_town().unwrap().id, "demo.town.zul");
        let facility_position = restored
            .content
            .town_facility(&tower)
            .unwrap()
            .entrance_position;
        restored.player.position = restored
            .town_local_to_active_position(
                "demo.town.zul",
                position_from_content(facility_position),
            )
            .unwrap();
        restored.refresh_town_task_terrain(&mut BTreeSet::new());
        let natural = match realm {
            "sorcery" => "surface-water-shallow",
            "chaos" => "surface-mountain",
            _ => "surface-tree",
        };
        assert_eq!(
            restored.terrain_at(entry),
            format!("demo.terrain.{natural}")
        );
        let inventory = restored.items.clone();
        for index in 0..30 {
            support::give_inventory_item(
                &mut restored,
                &format!("test.full.{index}"),
                "demo.item.short-sword",
            );
        }
        let full = restored.to_save();
        assert_eq!(
            restored.claim_task_reward(&tower, &task),
            Err("inventory-full")
        );
        assert_eq!(restored.to_save(), full);
        restored.items = inventory;
        dispatch_next(
            &mut restored,
            GameCommand::ClaimTaskReward {
                facility_id: tower.clone(),
                task_id: task.clone(),
            },
        );
        assert_eq!(
            restored.task_states[&task].status,
            TaskStatusKindDto::Completed
        );
        let reward = restored
            .items
            .iter()
            .find(|item| item.id == format!("{task}.reward.1"))
            .unwrap();
        assert_eq!(reward.kind_id, format!("demo.item.{book}"));
        assert_eq!(reward.quantity, 1);
        assert_eq!(reward.location, ItemLocation::Inventory);
        assert!(
            !restored.teleport_town_target_available("demo.town.zul"),
            "node success cannot replace failed quest 77"
        );
        let before = restored.to_save();
        assert_eq!(
            restored.claim_task_reward(&tower, &task),
            Err("reward-unavailable")
        );
        assert_eq!(restored.to_save(), before);
        restored = Game::from_save(before).unwrap();
        // Separately prepare 77 settlement to exercise the unlocked cross-town/save path.
        prepare_zul_task_status(
            &mut restored,
            "demo.task.zul-eddies",
            TaskStatusKindDto::Completed,
        );
        restored.teleport_to_town("demo.town.outpost").unwrap();
        restored.teleport_to_town("demo.town.zul").unwrap();
        assert_eq!(
            restored.task_states[&task].status,
            TaskStatusKindDto::Completed
        );
        assert_eq!(
            restored
                .items
                .iter()
                .filter(|item| item.id == format!("{task}.reward.1"))
                .count(),
            1
        );
    }
}

#[test]
fn zul_node_failed_or_abandoned_return_restores_gate_and_cannot_be_reaccepted() {
    for (realm, local) in [
        ("sorcery", Position { x: 78, y: 44 }),
        ("chaos", Position { x: 9, y: 1 }),
        ("nature", Position { x: 61, y: 4 }),
    ] {
        for abandon in [false, true] {
            let tower = format!("demo.town-facility.zul-{realm}-tower");
            let task = format!("demo.task.zul-{realm}-node");
            let mut game = Game::new_with_build_race_and_name(
                42,
                "demo.build.mage-sorcery-nature",
                "rfb-legacy.race.beastman",
                "Nodes",
            )
            .unwrap();
            enter_town_facility(&mut game, &tower);
            support::clear_monsters(&mut game);
            prepare_zul_task_status(
                &mut game,
                "demo.task.zul-eddies",
                TaskStatusKindDto::Abandoned,
            );
            game.apply_player_melee_status(STATUS_INVULNERABILITY, 1000, "test.node-lifecycle");
            dispatch_next(
                &mut game,
                GameCommand::AcceptTask {
                    facility_id: tower.clone(),
                    task_id: task.clone(),
                },
            );
            let entry = game
                .town_local_to_active_position("demo.town.zul", local)
                .unwrap();
            game.player.position = entry;
            dispatch_next(&mut game, GameCommand::TraverseStairs);
            assert!(game.entities.len() > 1);
            dispatch_next(
                &mut game,
                if abandon {
                    GameCommand::AbandonTask
                } else {
                    GameCommand::TraverseStairs
                },
            );
            assert_eq!(
                game.task_states[&task].status,
                if abandon {
                    TaskStatusKindDto::Abandoned
                } else {
                    TaskStatusKindDto::Failed
                }
            );
            assert_eq!(game.player.position, entry);
            // The shared town rule temporarily protects blocked return squares until departure.
            assert!(
                game.content
                    .terrain(game.terrain_at(entry))
                    .unwrap()
                    .walkable
            );
            game = Game::from_save(game.to_save()).unwrap();
            let door = game
                .content
                .town_facility(&tower)
                .unwrap()
                .entrance_position;
            game.player.position = game
                .town_local_to_active_position("demo.town.zul", position_from_content(door))
                .unwrap();
            game.refresh_town_task_terrain(&mut BTreeSet::new());
            let before = game.to_save();
            assert_eq!(game.accept_task(&tower, &task), Err("task-already-taken"));
            assert_eq!(
                game.claim_task_reward(&tower, &task),
                Err("reward-unavailable")
            );
            assert_eq!(game.to_save(), before);
            assert!(
                !game
                    .items
                    .iter()
                    .any(|item| item.id == format!("{task}.reward.1"))
            );
        }
    }
}

#[test]
fn node_interior_lava_and_water_use_periodic_exposure_flight_and_resistance() {
    let tower = "demo.town-facility.zul-nature-tower";
    let mut game = town_facility_game(42, "demo.build.mage-sorcery-nature", tower);
    game.debug_prepare_spell_learning_e2e(50).unwrap();
    support::choose_human_talent_if_pending(&mut game);
    support::clear_monsters(&mut game);
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: tower.into(),
            task_id: "demo.task.zul-nature-node".into(),
        },
    );
    game.player.position = game
        .town_local_to_active_position("demo.town.zul", Position { x: 61, y: 4 })
        .unwrap();
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    support::clear_monsters(&mut game);
    let position = game.player.position;
    game.player.statuses.clear();
    game.player.hp = 1000;
    game.world_tick = 10;
    support::replace_terrain(&mut game, position, "demo.terrain.surface-lava-deep");
    game.rng = RfbRng::seeded(42);
    let mut expected = game.rng.clone();
    let raw = 6000 + expected.bounded(4000) as i32;
    let amount = raw / 100 + i32::from(expected.bounded(100) < (raw % 100) as u64);
    let mut events = Vec::new();
    assert!(game.process_player_interior_water_lava_damage(&mut events));
    assert_eq!(game.player.hp, 1000 - amount);
    assert_eq!(game.rng, expected);
    assert!(
        matches!(events.as_slice(), [DomainEvent::WildernessTerrainDamaged { damage, .. }] if damage.applied == amount)
    );
    game.apply_player_melee_status(STATUS_LEVITATION, 100, "test.flight");
    support::replace_terrain(&mut game, position, "demo.terrain.surface-lava-shallow");
    let before = game.rng.clone();
    assert!(!game.process_player_interior_water_lava_damage(&mut Vec::new()));
    assert_eq!(game.rng, before);
    support::replace_terrain(&mut game, position, "demo.terrain.surface-lava-deep");
    let before = game.player.hp;
    assert!(game.process_player_interior_water_lava_damage(&mut Vec::new()));
    assert!(
        (12..=20).contains(&(before - game.player.hp)),
        "flying above deep lava still burns"
    );
    game.player
        .resistances
        .set(DamageType::Fire, ResistanceLevel::Immune);
    let before = game.player.hp;
    assert!(!game.process_player_interior_water_lava_damage(&mut Vec::new()));
    assert_eq!(game.player.hp, before);
    game.player
        .resistances
        .set(DamageType::Fire, ResistanceLevel::Normal);
    game.player.statuses.clear();
    game.apply_player_melee_status(STATUS_INVULNERABILITY, 1, "test.invulnerability");
    game.world_tick = 9;
    // Expiration spends one action; isolate the protected tick from later exposed ticks.
    game.player.energy_need = 1 - crate::scheduler::STANDARD_ACTION_COST;
    let mut events = Vec::new();
    game.advance_until_player_ready(
        false,
        true,
        false,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::WildernessTerrainDamaged { .. })),
        "last invulnerability tick protects before expiration"
    );
    game.world_tick = 19;
    game.player.energy_need = 1;
    events.clear();
    game.advance_until_player_ready(
        false,
        true,
        false,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::WildernessTerrainDamaged { .. })),
        "standing still takes periodic lava damage"
    );
    support::replace_terrain(&mut game, position, "demo.terrain.surface-water-deep");
    for index in 0..100 {
        support::give_inventory_item(
            &mut game,
            &format!("test.weight.{index}"),
            "demo.item.short-sword",
        );
    }
    assert!(game.carried_weight_tenths_pound() > game.player_carry_capacity_tenths_pound());
    let before = game.player.hp;
    assert!(game.process_player_interior_water_lava_damage(&mut Vec::new()));
    assert!(game.player.hp < before);
    game.apply_player_melee_status(STATUS_LEVITATION, 100, "test.flight");
    assert!(!game.process_player_interior_water_lava_damage(&mut Vec::new()));
}

fn enter_town(game: &mut Game, town_id: &str, position: Position) {
    dispatch_next(
        game,
        GameCommand::EnterWorldMap {
            leave_pets: false,
            cancel_recall: false,
        },
    );
    game.wilderness_position = Some(position);
    dispatch_next(game, GameCommand::LeaveWorldMap);
    assert_eq!(game.current_town().unwrap().id, town_id);
}

fn town_facility_game(seed: u64, build_id: &str, facility_id: &str) -> Game {
    let mut game = Game::new_with_build(seed, build_id).unwrap();
    enter_town_facility(&mut game, facility_id);
    game
}

pub(super) fn enter_town_facility(game: &mut Game, facility_id: &str) {
    let facility = game.content.town_facility(facility_id).unwrap();
    let town_id = facility.town_id.clone();
    let entrance = facility.entrance_position;
    let position = game
        .wilderness()
        .locations
        .iter()
        .find_map(|location| match location {
            rfb_content::WildernessLocationDefinition::Town {
                town_id: id,
                position,
                ..
            } if id == &town_id => Some(position_from_content(*position)),
            _ => None,
        })
        .unwrap();
    enter_town(game, &town_id, position);
    game.player.position = game
        .town_local_to_wilderness_view_position(
            &town_id,
            Position {
                x: i32::from(entrance.x),
                y: i32::from(entrance.y),
            },
        )
        .unwrap();
}

#[test]
fn casino_poker_pays_once_resumes_the_deck_and_settles_chance_on_exit() {
    casino_poker_round_trip("demo.town-facility.morivant-casino");
}

#[test]
fn telmora_casino_poker_resumes_at_its_own_facility() {
    casino_poker_round_trip("demo.town-facility.telmora-casino");
}

#[test]
fn angwil_casino_poker_resumes_at_its_own_facility() {
    casino_poker_round_trip("demo.town-facility.angwil-casino");
}

fn casino_poker_round_trip(id: &str) {
    use rfb_protocol::{
        CasinoActionDto as Action, CasinoGameDto, CasinoRoundSaveDto, VirtueKindDto,
    };
    let mut game = town_facility_game(61, "demo.build.warrior", id);
    game.virtues[0] = rfb_protocol::VirtueDto {
        kind: VirtueKindDto::Chance,
        value: 0,
    };
    game.gold = 10_000;
    let before = game.state_hash();
    let mut events = Vec::new();
    assert!(
        game.casino_action(
            id,
            Action::Start {
                game: CasinoGameDto::Poker,
                wager: 0,
                roulette_choice: None
            },
            &mut events
        )
        .is_err()
    );
    assert_eq!(game.state_hash(), before);
    let tick = game.world_tick;
    let chance = game.virtue_current(VirtueKindDto::Chance);
    game.casino_action(
        id,
        Action::Start {
            game: CasinoGameDto::Poker,
            wager: 100,
            roulette_choice: None,
        },
        &mut events,
    )
    .unwrap();
    assert_eq!(game.gold, 9900);
    assert_eq!(game.virtue_current(VirtueKindDto::Chance), chance);
    let rng = game.rng.clone();
    let deck = match &game.casino.as_ref().unwrap().round {
        CasinoRoundSaveDto::Poker { deck } => deck.clone(),
        _ => panic!("expected poker"),
    };
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.casino.as_ref().unwrap().facility_id, id);
    let other_casino = if id == "demo.town-facility.morivant-casino" {
        "demo.town-facility.telmora-casino"
    } else {
        "demo.town-facility.morivant-casino"
    };
    let before = restored.state_hash();
    assert!(
        restored
            .casino_action(other_casino, Action::Draw { replace_mask: 0 }, &mut events)
            .is_err()
    );
    assert_eq!(restored.state_hash(), before);
    let mut invalid = game.to_save();
    if let CasinoRoundSaveDto::Poker { deck } = &mut invalid.casino.as_mut().unwrap().round {
        deck[1] = deck[0];
    }
    assert!(Game::from_save(invalid).is_err());
    for target in [&mut game, &mut restored] {
        let before = target.state_hash();
        assert!(
            target
                .casino_action(id, Action::Leave, &mut events)
                .is_err()
        );
        assert!(
            target
                .casino_action(id, Action::Draw { replace_mask: 32 }, &mut events)
                .is_err()
        );
        assert_eq!(target.state_hash(), before);
        target
            .casino_action(id, Action::Draw { replace_mask: 31 }, &mut events)
            .unwrap();
        assert_eq!(target.rng, rng);
        let CasinoRoundSaveDto::Finished { values, odds } = &target.casino.as_ref().unwrap().round
        else {
            panic!("expected settlement")
        };
        assert_eq!(*values, deck[5..10]);
        assert_eq!(target.gold, 9900 + u32::from(*odds) * 100);
        let before = target.state_hash();
        assert!(
            target
                .casino_action(id, Action::Draw { replace_mask: 31 }, &mut events)
                .is_err()
        );
        assert_eq!(target.state_hash(), before);
        target
            .casino_action(id, Action::Leave, &mut events)
            .unwrap();
        assert!(target.casino.is_none());
        assert_eq!(
            target.virtue_current(VirtueKindDto::Chance),
            if target.gold >= 10_000 { 3 } else { -3 }
        );
        assert_eq!(target.world_tick, tick);
    }
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn casino_games_repeat_deterministically_and_craps_resumes_its_point() {
    use rfb_protocol::{CasinoActionDto as Action, CasinoGameDto::*, CasinoRoundSaveDto};
    let id = "demo.town-facility.morivant-casino";
    let mut template = town_facility_game(62, "demo.build.warrior", id);
    template.gold = 10_000;
    for kind in [InBetween, Roulette, DiceSlots, Craps] {
        let mut game = Game::from_save(template.to_save()).unwrap();
        let mut replay = Game::from_save(template.to_save()).unwrap();
        for target in [&mut game, &mut replay] {
            let choice = (kind == Roulette).then_some(7);
            dispatch_next(
                target,
                GameCommand::Casino {
                    facility_id: id.to_owned(),
                    action: Action::Start {
                        game: kind,
                        wager: 100,
                        roulette_choice: choice,
                    },
                },
            );
            let before = target.state_hash();
            assert!(matches!(
                target.dispatch(crate::game::tests::support::command(
                    target.last_command_seq + 1,
                    target.revision,
                    GameCommand::Wait
                )),
                Err(CoreError::CasinoInProgress)
            ));
            assert_eq!(target.state_hash(), before);
            // Finish at most twenty rounds, including an actual saved point in Craps.
            let mut saved_point = false;
            for _ in 0..20 {
                if matches!(
                    target.casino.as_ref().unwrap().round,
                    CasinoRoundSaveDto::Craps { .. }
                ) {
                    let restored = Game::from_save(target.to_save()).unwrap();
                    assert_eq!(restored.state_hash(), target.state_hash());
                    *target = restored;
                    saved_point = true;
                }
                while matches!(
                    target.casino.as_ref().unwrap().round,
                    CasinoRoundSaveDto::Craps { .. }
                ) {
                    dispatch_next(
                        target,
                        GameCommand::Casino {
                            facility_id: id.to_owned(),
                            action: Action::Roll,
                        },
                    );
                }
                let CasinoRoundSaveDto::Finished { odds, .. } =
                    target.casino.as_ref().unwrap().round
                else {
                    panic!("round must settle")
                };
                assert!(match kind {
                    InBetween => [0, 4].contains(&odds),
                    Roulette => [0, 9].contains(&odds),
                    DiceSlots => [0, 2, 5, 10, 20, 50, 200, 1000].contains(&odds),
                    Craps => [0, 2].contains(&odds),
                    Poker => unreachable!(),
                });
                if kind != Craps || saved_point {
                    break;
                }
                dispatch_next(
                    target,
                    GameCommand::Casino {
                        facility_id: id.to_owned(),
                        action: Action::Again {
                            roulette_choice: choice,
                        },
                    },
                );
            }
            assert!(kind != Craps || saved_point);
            let gold = target.gold;
            let restored = Game::from_save(target.to_save()).unwrap();
            assert_eq!(restored.gold, gold);
            let before = target.state_hash();
            assert!(
                target
                    .casino_action(
                        id,
                        Action::Again {
                            roulette_choice: Some(10)
                        },
                        &mut Vec::new()
                    )
                    .is_err()
            );
            assert_eq!(target.state_hash(), before);
            dispatch_next(
                target,
                GameCommand::Casino {
                    facility_id: id.to_owned(),
                    action: Action::Leave,
                },
            );
            assert_eq!(target.gold, gold);
            assert_eq!(target.world_tick, template.world_tick);
        }
        assert_eq!(game.state_hash(), replay.state_hash());
    }
}

#[test]
fn reputation_is_paid_uses_original_bands_and_survives_save() {
    let mut game = white_horse_inn_game(51);
    assert_eq!(game.fame, 0);
    let tick = game.world_tick;
    let rng = game.rng.clone();
    for (fame, key) in [
        (0, "unknown"),
        (1, "unheard"),
        (19, "unheard"),
        (20, "noticed"),
        (39, "noticed"),
        (40, "talked"),
        (59, "talked"),
        (60, "honored"),
        (79, "honored"),
        (80, "hero"),
        (99, "hero"),
        (100, "legend"),
        (149, "legend"),
        (150, "ballads"),
    ] {
        game.fame = fame;
        let snapshot = game.snapshot();
        let cost = projected_shop(&snapshot.shops, WHITE_HORSE_INN_ID)
            .inn_reputation_cost
            .unwrap();
        assert_eq!(snapshot.player.fame, fame);
        game.gold = cost - 1;
        let before = game.state_hash();
        assert_eq!(
            game.ask_reputation_at_inn(WHITE_HORSE_INN_ID, &mut Vec::new()),
            Err("insufficient-gold")
        );
        assert_eq!(game.state_hash(), before);
        game.gold = cost;
        let report = dispatch_next(
            &mut game,
            GameCommand::AskReputationAtInn {
                facility_id: WHITE_HORSE_INN_ID.to_owned(),
            },
        );
        assert_eq!(
            report.events[0].message_key,
            format!("inn-reputation-{key}")
        );
        assert_eq!(game.gold, 0);
        assert_eq!(game.fame, fame);
    }
    assert_eq!(game.world_tick, tick);
    assert_eq!(game.rng, rng);
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.fame, 150);
    assert_eq!(restored.state_hash(), game.state_hash());
    game.player.position.x -= 1;
    let before = game.state_hash();
    assert_eq!(
        game.ask_reputation_at_inn(WHITE_HORSE_INN_ID, &mut Vec::new()),
        Err("inn-unreachable")
    );
    assert_eq!(game.state_hash(), before);
}

#[test]
fn town_prices_apply_rfb_fame_charisma_race_and_rounding_in_order() {
    let mut game = Game::new_with_build(51, "demo.build.warrior").unwrap();
    // Human warrior CHR 14: race 100, charisma 104, then fame, then greed.
    assert_eq!(game.effective_player_attributes().charisma, 14);
    for (fame, price) in [
        (0, 2100),
        (3, 2100),
        (4, 2090),
        (140, 1560),
        (200, 1500),
        (u16::MAX, 1500),
    ] {
        game.fame = fame;
        assert_eq!(game.town_service_price(1500), price, "fame={fame}");
        assert_eq!(game.town_service_price(0), 0);
    }
    game.fame = 0;
    game.progress.attributes.charisma = 3;
    // The warrior's +1 CHR produces index 1 (125%), then 135% fame => 169%.
    assert_eq!(game.effective_player_attributes().charisma, 4);
    assert_eq!(game.town_service_price(1500), 2540);
    let mut form = monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 100, "test.price").status;
    form.granted_race_id = Some("rfb-legacy.race.skeleton".to_owned());
    game.player.statuses.push(form);
    // The effective race supplies the same trade modifier used by shops.
    assert_ne!(game.town_service_price(1500), 2540);
    assert_eq!(
        super::super::town::buy_unit_price(1_000_099, 140),
        1_400_000
    );
    assert_eq!(
        super::super::town::sell_unit_price(1_000_099, 140, u32::MAX),
        714_000
    );
}

#[test]
fn building_enchantment_quotes_tiers_forces_only_selected_steps_and_preserves_save() {
    let mut game = anambar_facility_game(51, "demo.build.warrior", None, ANAMBAR_WARRIOR_GUILD_ID);
    game.apply_player_experience(4_500_000, &mut Vec::new());
    support::choose_human_talent_if_pending(&mut game);
    support::give_inventory_item(&mut game, "test.enchant.broken", "demo.item.broken-sword");
    let item = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.enchant.broken")
        .unwrap();
    item.enchantments.to_hit = -15;
    item.enchantments.to_damage = -15;
    let targets = |game: &Game, kind| {
        game.town_facility_service_dtos(
            game.content
                .town_facility(ANAMBAR_WARRIOR_GUILD_ID)
                .unwrap(),
        )
        .into_iter()
        .find(|s| s.kind == kind)
        .unwrap()
        .targets
    };
    let quotes = targets(&game, FacilityServiceKindDto::EnchantWeapon);
    let target = quotes
        .iter()
        .find(|t| t.item_id == "test.enchant.broken")
        .unwrap();
    assert_eq!(target.choices.len(), 25);
    assert_eq!(
        (
            target.choices[24].result.to_hit,
            target.choices[24].result.to_damage
        ),
        (8, 6)
    );
    game.gold = target.choices[24].cost;
    let before = game.state_hash();
    for steps in [None, Some(0), Some(26)] {
        assert_eq!(
            game.use_town_facility_service(
                ANAMBAR_WARRIOR_GUILD_ID,
                FacilityServiceKindDto::EnchantWeapon,
                Some("test.enchant.broken"),
                steps,
                &mut Vec::new()
            ),
            Err("enchantment-steps-unavailable")
        );
        assert_eq!(game.state_hash(), before);
    }
    game.gold -= 1;
    let before = game.state_hash();
    assert_eq!(
        game.use_town_facility_service(
            ANAMBAR_WARRIOR_GUILD_ID,
            FacilityServiceKindDto::EnchantWeapon,
            Some("test.enchant.broken"),
            Some(25),
            &mut Vec::new()
        ),
        Err("insufficient-gold")
    );
    assert_eq!(game.state_hash(), before);
    game.gold += 1;
    let tick = game.world_tick;
    let rng = game.rng.clone();
    for steps in [25, 9] {
        let quotes = targets(&game, FacilityServiceKindDto::EnchantWeapon);
        game.gold = quotes
            .iter()
            .find(|t| t.item_id == "test.enchant.broken")
            .unwrap()
            .choices
            .iter()
            .find(|c| c.steps == steps)
            .unwrap()
            .cost;
        let result = dispatch_next(
            &mut game,
            GameCommand::UseFacilityService {
                facility_id: ANAMBAR_WARRIOR_GUILD_ID.to_owned(),
                service: FacilityServiceKindDto::EnchantWeapon,
                item_id: Some("test.enchant.broken".to_owned()),
                enchantment_steps: Some(steps),
            },
        );
        assert!(
            result
                .events
                .iter()
                .any(|e| e.kind == "facility.item-enchanted")
        );
        assert_eq!(game.gold, 0);
    }
    let item = game
        .items
        .iter()
        .find(|i| i.id == "test.enchant.broken")
        .unwrap();
    assert_eq!(
        (item.enchantments.to_hit, item.enchantments.to_damage),
        (17, 19)
    );
    assert_eq!(game.world_tick, tick);
    assert_eq!(game.rng, rng);
    assert!(
        !targets(&game, FacilityServiceKindDto::EnchantWeapon)
            .iter()
            .any(|t| t.item_id == "test.enchant.broken")
    );
    let mut invalid = game.to_save();
    invalid
        .inventory
        .iter_mut()
        .find(|i| i.id == "test.enchant.broken")
        .unwrap()
        .enchantments
        .to_hit = 256;
    assert!(Game::from_save(invalid).is_err());
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    support::give_inventory_item(
        &mut restored,
        "test.enchant.artifact",
        "demo.item.set-of-gauntlets-paurnimmen",
    );
    restored.register_generated_artifact("demo.item.set-of-gauntlets-paurnimmen");
    support::give_inventory_item(
        &mut restored,
        "test.enchant.dragon",
        "demo.item.multi-hued-dragon-scale-mail",
    );
    let quotes = targets(&restored, FacilityServiceKindDto::EnchantArmor);
    let artifact = quotes
        .iter()
        .find(|t| t.item_id == "test.enchant.artifact")
        .unwrap();
    assert_eq!(artifact.choices.len(), 8); // Intrinsic +7, owner limit +15.
    assert_eq!(artifact.choices[0].cost, 5250); // 500 value * 5 * 3 * 140% / 2.
    assert_eq!(artifact.choices[7].cost, 214000); // Original repeated 5/3 armor multiplier.
    let dragon = quotes
        .iter()
        .find(|t| t.item_id == "test.enchant.dragon")
        .unwrap();
    assert_eq!(dragon.choices.len(), 5); // Intrinsic +10.
    assert_eq!(dragon.choices[0].cost, 2940); // Non-artifact valuation keeps the original 3/4.
    restored.gold = artifact.choices[0].cost;
    let rng = restored.rng.clone();
    restored
        .use_town_facility_service(
            ANAMBAR_WARRIOR_GUILD_ID,
            FacilityServiceKindDto::EnchantArmor,
            Some("test.enchant.artifact"),
            Some(1),
            &mut Vec::new(),
        )
        .unwrap();
    assert_eq!(restored.gold, 0);
    assert_eq!(restored.rng, rng); // Artifacts do not resist ENCH_FORCE.
    support::give_inventory_item(&mut restored, "test.enchant.cursed", "demo.item.long-sword");
    restored
        .items
        .iter_mut()
        .find(|i| i.id == "test.enchant.cursed")
        .unwrap()
        .curse = Some(ItemCurseSeverityDto::Normal);
    let seed = (0..100)
        .find(|s| RfbRng::seeded(*s).bounded(100) < 25)
        .unwrap();
    restored.rng = RfbRng::seeded(seed);
    restored.gold = 1_000_000;
    restored
        .use_town_facility_service(
            ANAMBAR_WARRIOR_GUILD_ID,
            FacilityServiceKindDto::EnchantWeapon,
            Some("test.enchant.cursed"),
            Some(1),
            &mut Vec::new(),
        )
        .unwrap();
    assert_eq!(
        restored
            .items
            .iter()
            .find(|i| i.id == "test.enchant.cursed")
            .unwrap()
            .curse,
        None
    );
    assert_eq!(restored.rng.draw_counter, 1);
    restored
        .items
        .iter_mut()
        .find(|i| i.id == "test.enchant.cursed")
        .unwrap()
        .curse = Some(ItemCurseSeverityDto::Heavy);
    restored
        .use_town_facility_service(
            ANAMBAR_WARRIOR_GUILD_ID,
            FacilityServiceKindDto::EnchantWeapon,
            Some("test.enchant.cursed"),
            Some(1),
            &mut Vec::new(),
        )
        .unwrap();
    assert_eq!(
        restored
            .items
            .iter()
            .find(|i| i.id == "test.enchant.cursed")
            .unwrap()
            .curse,
        Some(ItemCurseSeverityDto::Heavy)
    );
    assert_eq!(restored.rng.draw_counter, 1);
    assert_eq!(
        Game::from_save(restored.to_save()).unwrap().state_hash(),
        restored.state_hash()
    );
}

#[test]
fn morivant_monster_research_reveals_unseen_kinds_only_after_paid_confirmation() {
    let facility_id = "demo.town-facility.morivant-beastmaster";
    let mut game = town_facility_game(62, "demo.build.warrior", facility_id);
    game.reveal_current_visibility();
    let before_browsing = game.state_hash();
    let snapshot = game.snapshot();
    let service = snapshot
        .task_services
        .iter()
        .find(|s| s.id == facility_id)
        .unwrap();
    assert_eq!(
        service.research_monster_cost,
        Some(game.town_service_price(1500))
    );
    let candidate = service
        .research_monsters
        .iter()
        .find(|m| m.kind_id == "demo.actor.sheep")
        .unwrap();
    assert!(candidate.knowledge.is_none());
    assert!(service.research_monsters.iter().any(|m| m.unique));
    assert!(
        !game
            .entities
            .iter()
            .any(|actor| actor.kind_id == candidate.kind_id)
    );
    assert_eq!(game.state_hash(), before_browsing);
    let command = GameCommand::ResearchMonsterAtFacility {
        facility_id: facility_id.to_owned(),
        actor_kind_id: candidate.kind_id.clone(),
    };
    game.gold = service.research_monster_cost.unwrap() - 1;
    let before = game.state_hash();
    assert_eq!(
        game.research_monster_at_facility(facility_id, &candidate.kind_id, &mut Vec::new()),
        Err("insufficient-gold")
    );
    assert_eq!(game.state_hash(), before);
    let business_before = (
        game.gold,
        game.world_tick,
        game.rng.clone(),
        game.probed_actor_kind_ids.clone(),
    );
    let rejected = dispatch_next(&mut game, command.clone());
    assert_eq!(rejected.events[0].args["reason"], "insufficient-gold");
    assert_eq!(
        (
            game.gold,
            game.world_tick,
            game.rng.clone(),
            game.probed_actor_kind_ids.clone()
        ),
        business_before
    );
    game.gold = service.research_monster_cost.unwrap();
    for kind in ["missing.actor", game.player.kind_id.as_str()].map(str::to_owned) {
        let before = game.state_hash();
        assert_eq!(
            game.research_monster_at_facility(facility_id, &kind, &mut Vec::new()),
            Err("monster-unavailable")
        );
        assert_eq!(game.state_hash(), before);
    }
    let entrance = game.player.position;
    game.player.position.x -= 1;
    game.reveal_current_visibility();
    let before = game.state_hash();
    assert!(
        game.snapshot()
            .task_services
            .iter()
            .find(|s| s.id == facility_id)
            .unwrap()
            .research_monsters
            .is_empty()
    );
    assert_eq!(
        game.research_monster_at_facility(facility_id, &candidate.kind_id, &mut Vec::new()),
        Err("facility-unreachable")
    );
    assert_eq!(game.state_hash(), before);
    game.player.position = entrance;
    let rng = game.rng.clone();
    let tick = game.world_tick;
    let update = dispatch_next(&mut game, command);
    assert_eq!(update.events[0].kind, "facility.monster-researched");
    assert_eq!(game.gold, 0);
    assert_eq!(game.rng, rng);
    assert_eq!(game.world_tick, tick);
    let knowledge = game
        .research_monster_dtos()
        .into_iter()
        .find(|m| m.kind_id == "demo.actor.sheep")
        .unwrap()
        .knowledge
        .unwrap();
    let actor = game.content.actor("demo.actor.sheep").unwrap();
    assert_eq!(knowledge.max_hp, actor.max_hp);
    assert_eq!(knowledge.armor_class, rating_to_armor_class(actor.defense));
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(
        restored.research_monster_dtos(),
        game.research_monster_dtos()
    );
}

#[test]
fn morivant_inn_meals_are_atomic_and_feed_skeletons_without_creating_items() {
    let mut game = Game::new_with_build_race_and_name(
        62,
        "demo.build.warrior",
        "rfb-legacy.race.skeleton",
        Game::DEFAULT_PLAYER_NAME,
    )
    .unwrap();
    enter_morivant(&mut game);
    game.player.position = game
        .town_local_to_wilderness_view_position(MORIVANT_TOWN_ID, Position { x: 92, y: 43 })
        .unwrap();
    game.mark_shop_visited_at_player().unwrap();
    game.reveal_current_visibility();
    assert_eq!(
        projected_shop(&game.snapshot().shops, MORIVANT_INN_ID).inn_food_cost,
        Some(3)
    );
    let command = GameCommand::EatAtInn {
        facility_id: MORIVANT_INN_ID.to_owned(),
    };
    game.gold = 1;
    let before = game.state_hash();
    assert_eq!(
        game.eat_at_inn(MORIVANT_INN_ID, &mut Vec::new()),
        Err("insufficient-gold")
    );
    assert_eq!(game.state_hash(), before);
    let business_before = (game.gold, game.world_tick, game.rng.clone(), game.nutrition);
    let rejected = dispatch_next(&mut game, command.clone());
    assert_eq!(rejected.events[0].args["reason"], "insufficient-gold");
    assert_eq!(
        (game.gold, game.world_tick, game.rng.clone(), game.nutrition),
        business_before
    );
    game.gold = 6;
    let entrance = game.player.position;
    game.player.position.x -= 1;
    game.reveal_current_visibility();
    let before = game.state_hash();
    assert_eq!(
        game.eat_at_inn(MORIVANT_INN_ID, &mut Vec::new()),
        Err("inn-unreachable")
    );
    assert_eq!(game.state_hash(), before);
    game.player.position = entrance;
    let items = game.items.clone();
    let rng = game.rng.clone();
    let tick = game.world_tick;
    for nutrition in [100, rfb_protocol::PLAYER_NUTRITION_MAXIMUM] {
        game.nutrition = nutrition;
        let update = dispatch_next(&mut game, command.clone());
        let meal = update
            .events
            .iter()
            .find(|event| event.kind == "inn.food")
            .unwrap();
        assert_eq!(meal.args["foodKey"], "inn-food-empty-staff");
        assert_eq!(game.nutrition, rfb_protocol::PLAYER_NUTRITION_MAXIMUM - 1);
    }
    assert_eq!(game.gold, 0);
    assert_eq!(game.items, items);
    assert_eq!(game.rng, rng);
    assert_eq!(game.world_tick, tick);
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn inn_meals_use_effective_race_and_the_source_flavour_rng_branches() {
    let mut game = Game::new_with_build(62, "demo.build.warrior").unwrap();
    let mut events = Vec::new();
    let rng = game.rng.clone();
    assert_eq!(game.consume_inn_meal(&mut events), "inn-food-porridge");
    assert_eq!(game.rng, rng);
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 100, "test.inn-form").status;
    form.granted_race_id = Some("rfb-legacy.race.balrog".to_owned());
    game.player.statuses.push(form);
    assert_eq!(game.consume_inn_meal(&mut events), "inn-food-meat");
    assert_eq!(game.rng, rng);
    for (race, seed, expected, draws) in [
        ("rfb-legacy.race.ent", 0, "inn-food-water", 1),
        ("rfb-legacy.race.vampire", 0, "inn-food-blood", 1),
        ("rfb-legacy.race.android", 0, "inn-food-oil", 1),
        ("demo.race.vampire-lord", 0, "inn-food-empty-staff", 2),
        ("rfb-legacy.race.ent", 7, "inn-food-buffet", 1),
        ("rfb-legacy.race.golem", 5, "inn-food-speed-staff", 2),
        ("rfb-legacy.race.spectre", 0, "inn-food-empty-staff", 2),
        ("rfb-legacy.race.einheri", 0, "inn-food-porridge", 0),
    ] {
        game.player.statuses[0].granted_race_id = Some(race.to_owned());
        game.rng = RfbRng::seeded(seed);
        assert_eq!(game.consume_inn_meal(&mut events), expected, "{race}");
        assert_eq!(game.rng.draw_counter, draws, "{race}");
    }
}

#[test]
fn morivant_identification_uses_the_projected_membership_price() {
    for (facility_id, build_id, membership, cost) in [
        (
            MORIVANT_SORCERY_TOWER_ID,
            "demo.build.high-mage-sorcery",
            FacilityMembershipDto::Owner,
            100,
        ),
        (
            MORIVANT_SORCERY_TOWER_ID,
            "demo.build.warrior",
            FacilityMembershipDto::Visitor,
            500,
        ),
        (
            MORIVANT_THIEVES_GUILD_ID,
            "demo.build.warrior",
            FacilityMembershipDto::Visitor,
            600,
        ),
        (
            "demo.town-facility.telmora-thieves-guild",
            "demo.build.warrior",
            FacilityMembershipDto::Visitor,
            2000,
        ),
        (
            "demo.town-facility.angwil-mage-tower",
            "demo.build.high-mage-death",
            FacilityMembershipDto::Owner,
            200,
        ),
        (
            "demo.town-facility.angwil-mage-tower",
            "demo.build.mage-death-sorcery",
            FacilityMembershipDto::Owner,
            200,
        ),
        (
            "demo.town-facility.thalos-sorcery-tower",
            "demo.build.mage-death-sorcery",
            FacilityMembershipDto::Owner,
            200,
        ),
        (
            "demo.town-facility.thalos-sorcery-tower",
            "demo.build.high-mage-death",
            FacilityMembershipDto::Owner,
            200,
        ),
        (
            "demo.town-facility.thalos-sorcery-tower",
            "demo.build.warrior",
            FacilityMembershipDto::Visitor,
            1000,
        ),
        (
            "demo.town-facility.angwil-mage-tower",
            "demo.build.warrior",
            FacilityMembershipDto::Visitor,
            1000,
        ),
        (
            "demo.town-facility.angwil-thieves-guild",
            "demo.build.warrior",
            FacilityMembershipDto::Visitor,
            800,
        ),
    ] {
        let mut game = town_facility_game(51, build_id, facility_id);
        let cost = game.town_service_price(cost);
        let service = game
            .snapshot()
            .task_services
            .into_iter()
            .find(|service| service.id == facility_id)
            .unwrap();
        assert!(service.player_at_entrance);
        assert_eq!(service.membership, membership);
        assert_eq!(service.identify_all_items_cost, Some(cost));
        assert!(service.tasks.is_empty());
        if facility_id == MORIVANT_THIEVES_GUILD_ID {
            assert_eq!(service.inn_stay_cost, Some(game.town_service_price(50)));
        }
        let inventory_ids = game
            .items
            .iter()
            .filter(|item| item.location == ItemLocation::Inventory)
            .map(|item| item.id.clone())
            .collect::<Vec<_>>();
        for id in &inventory_ids {
            game.item_property_knowledge.remove(id);
        }
        game.gold = cost - 1;
        let before = game.state_hash();
        assert_eq!(
            game.identify_all_at_facility(facility_id).unwrap_err(),
            "insufficient-gold"
        );
        assert_eq!(game.state_hash(), before);
        game.gold = cost;
        let entrance = game.player.position;
        game.player.position.x -= 1;
        let before = game.state_hash();
        assert_eq!(
            game.identify_all_at_facility(facility_id).unwrap_err(),
            "facility-unreachable"
        );
        assert_eq!(game.state_hash(), before);
        game.player.position = entrance;
        let update = dispatch_next(
            &mut game,
            GameCommand::IdentifyAllAtFacility {
                facility_id: facility_id.to_owned(),
            },
        );
        assert_eq!(game.gold, 0);
        assert_eq!(update.events[0].args["cost"], cost.to_string());
        assert!(
            inventory_ids
                .iter()
                .all(|id| game.item_property_knowledge[id].appraised)
        );
        let restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
    }
}

#[test]
fn morivant_nine_shops_trade_and_save() {
    nine_shops_trade_and_save(
        MORIVANT_TOWN_ID,
        MORIVANT_INN_ID,
        Position { x: 47, y: 50 },
        12,
    );
}

#[test]
fn telmora_nine_shops_trade_and_save() {
    nine_shops_trade_and_save(
        "demo.town.telmora",
        "demo.shop.telmora-inn",
        Position { x: 87, y: 49 },
        9,
    );
}

#[test]
fn angwil_nine_shops_trade_and_save() {
    nine_shops_trade_and_save(
        "demo.town.angwil",
        "demo.shop.angwil-inn",
        Position { x: 74, y: 23 },
        11,
    );
}

fn nine_shops_trade_and_save(
    town_id: &str,
    inn_id: &str,
    position: Position,
    service_count: usize,
) {
    let mut game = Game::new_with_build(51, "demo.build.warrior").unwrap();
    let town = game.content.town(town_id).unwrap().clone();
    assert!(!game.shop_states.keys().any(|id| town.shop_ids.contains(id)));
    let entrance = game
        .content
        .shop(GENERAL_STORE_ID)
        .unwrap()
        .entrance_position;
    game.player.position = game
        .town_local_to_wilderness_view_position(
            "demo.town.outpost",
            Position {
                x: i32::from(entrance.x),
                y: i32::from(entrance.y),
            },
        )
        .unwrap();
    game.mark_shop_visited_at_player().unwrap();
    let outpost_stock = game.shop_states[GENERAL_STORE_ID].inventory.clone();
    enter_town(&mut game, town_id, position);
    let snapshot = game.snapshot();
    assert_eq!(snapshot.shops.len(), 10);
    assert!(
        snapshot
            .shops
            .iter()
            .all(|shop| !shop.visited && shop.stock.is_empty())
    );
    assert_eq!(snapshot.homes.len(), 2);
    assert_eq!(snapshot.task_services.len(), service_count);
    game.gold = 1_000_000;
    for shop_id in town.shop_ids.iter().filter(|id| id.as_str() != inn_id) {
        let definition = game.content.shop(shop_id).unwrap();
        let local = Position {
            x: i32::from(definition.entrance_position.x),
            y: i32::from(definition.entrance_position.y),
        };
        game.player.position = game
            .town_local_to_wilderness_view_position(town_id, local)
            .unwrap();
        game.mark_shop_visited_at_player().unwrap();
        let shop = projected_shop(&game.snapshot().shops, shop_id).clone();
        assert!(shop.visited && shop.player_at_entrance);
        let stock = shop.stock.first().unwrap();
        let purchase = dispatch_next(
            &mut game,
            GameCommand::BuyFromShop {
                shop_id: shop_id.clone(),
                item_id: stock.id.clone(),
                quantity: 1,
            },
        );
        assert!(
            purchase
                .events
                .iter()
                .any(|event| event.kind == "shop.purchase"),
            "{shop_id}: {:?}",
            purchase.events
        );
        let item_id = game
            .items
            .iter()
            .find(|item| item.kind_id == stock.kind_id && item.location == ItemLocation::Inventory)
            .unwrap()
            .id
            .clone();
        let sale = dispatch_next(
            &mut game,
            GameCommand::SellToShop {
                shop_id: shop_id.clone(),
                item_id,
                quantity: 1,
            },
        );
        assert!(
            sale.events.iter().any(|event| event.kind == "shop.sale"),
            "{shop_id}: {:?}",
            sale.events
        );
        assert_eq!(game.shop_states[GENERAL_STORE_ID].inventory, outpost_stock);
    }
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn morivant_shares_home_rests_and_revisits_through_inns() {
    let mut game = Game::new_with_build(51, "demo.build.warrior").unwrap();
    game.player.position = Position { x: 110, y: 44 };
    let item = game.snapshot().homes[0].deposit_items[0].clone();
    dispatch_next(
        &mut game,
        GameCommand::DepositAtHome {
            facility_id: HOME_ID.to_owned(),
            item_id: item.id,
            quantity: 1,
        },
    );
    assert_eq!(game.home_states[HOME_ID].inventory.len(), 1);
    enter_morivant(&mut game);
    game.player.position = game
        .town_local_to_wilderness_view_position(MORIVANT_TOWN_ID, Position { x: 131, y: 41 })
        .unwrap();
    let home = game
        .snapshot()
        .homes
        .into_iter()
        .find(|home| home.id == MORIVANT_HOME_ID)
        .unwrap();
    let stored = home.stored_items[0].clone();
    dispatch_next(
        &mut game,
        GameCommand::WithdrawFromHome {
            facility_id: MORIVANT_HOME_ID.to_owned(),
            item_id: stored.id,
            quantity: 1,
        },
    );
    assert!(game.home_states[HOME_ID].inventory.is_empty());
    assert_eq!(game.home_states.len(), 2);
    game.player.position = game
        .town_local_to_wilderness_view_position(MORIVANT_TOWN_ID, Position { x: 92, y: 43 })
        .unwrap();
    game.mark_shop_visited_at_player().unwrap();
    game.gold = game.town_service_price(20) + 2 * game.town_service_price(500);
    game.player.hp = 1;
    game.world_tick = 12_345;
    let stay = dispatch_next(
        &mut game,
        GameCommand::StayAtInn {
            facility_id: MORIVANT_INN_ID.to_owned(),
        },
    );
    assert!(
        stay.events
            .iter()
            .any(|event| event.kind == "inn.stay" && event.args["cost"] == "28")
    );
    assert_eq!(game.world_tick, 50_000);
    assert_eq!(game.player.hp, game.effective_player_max_hp());
    assert_eq!(game.gold, 1_400);
    let stock = game.shop_states[MORIVANT_INN_ID].inventory.clone();
    let mut game = Game::from_save(game.to_save()).unwrap();
    dispatch_next(
        &mut game,
        GameCommand::TravelFromInn {
            facility_id: MORIVANT_INN_ID.to_owned(),
            destination_town_id: "demo.town.outpost".to_owned(),
        },
    );
    assert_eq!(game.current_town().unwrap().id, "demo.town.outpost");
    assert_eq!(game.gold, 700);
    dispatch_next(
        &mut game,
        GameCommand::TravelFromInn {
            facility_id: WHITE_HORSE_INN_ID.to_owned(),
            destination_town_id: MORIVANT_TOWN_ID.to_owned(),
        },
    );
    assert_eq!(game.current_town().unwrap().id, MORIVANT_TOWN_ID);
    assert_eq!(game.gold, 0);
    assert!(projected_shop(&game.snapshot().shops, MORIVANT_INN_ID).player_at_entrance);
    assert_eq!(game.shop_states[MORIVANT_INN_ID].inventory, stock);
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn telmora_inn_services_and_visited_travel_survive_save() {
    inn_services_and_visited_travel_survive_save(
        "demo.town.telmora",
        "demo.shop.telmora-inn",
        Position { x: 87, y: 49 },
        1,
        20,
    );
}

#[test]
fn angwil_inn_services_and_visited_travel_survive_save() {
    inn_services_and_visited_travel_survive_save(
        "demo.town.angwil",
        "demo.shop.angwil-inn",
        Position { x: 74, y: 23 },
        3,
        25,
    );
}

fn inn_services_and_visited_travel_survive_save(
    town: &str,
    inn: &str,
    world_position: Position,
    food: u32,
    stay: u32,
) {
    let mut game = white_horse_inn_game(51);
    assert!(
        projected_shop(&game.snapshot().shops, WHITE_HORSE_INN_ID)
            .inn_travel_destinations
            .iter()
            .all(|destination| destination.town_id != town)
    );
    enter_town(&mut game, town, world_position);
    assert!(game.town_states[town].visited);
    assert_eq!(game.player.position, Position { x: 99, y: 33 });
    let entrance = game.content.shop(inn).unwrap().entrance_position;
    game.player.position = game
        .town_local_to_wilderness_view_position(
            town,
            Position {
                x: i32::from(entrance.x),
                y: i32::from(entrance.y),
            },
        )
        .unwrap();
    game.mark_shop_visited_at_player().unwrap();
    let projected = projected_shop(&game.snapshot().shops, inn).clone();
    assert_eq!(projected.inn_food_cost, Some(game.town_service_price(food)));
    assert_eq!(projected.inn_stay_cost, Some(game.town_service_price(stay)));
    let destination = projected
        .inn_travel_destinations
        .iter()
        .find(|destination| destination.town_id == "demo.town.outpost")
        .unwrap();
    assert_eq!(destination.cost, game.town_service_price(500));
    game.gold = projected.inn_food_cost.unwrap()
        + projected.inn_stay_cost.unwrap()
        + projected.inn_reputation_cost.unwrap()
        + 2 * destination.cost;
    game.nutrition = 100;
    dispatch_next(
        &mut game,
        GameCommand::EatAtInn {
            facility_id: inn.to_owned(),
        },
    );
    assert_eq!(game.nutrition, rfb_protocol::PLAYER_NUTRITION_MAXIMUM - 1);
    let reputation = dispatch_next(
        &mut game,
        GameCommand::AskReputationAtInn {
            facility_id: inn.to_owned(),
        },
    );
    assert_eq!(reputation.events[0].message_key, "inn-reputation-unknown");
    game.player.hp = 1;
    game.world_tick = 12_345;
    dispatch_next(
        &mut game,
        GameCommand::StayAtInn {
            facility_id: inn.to_owned(),
        },
    );
    assert_eq!(game.player.hp, game.effective_player_max_hp());
    assert_eq!(game.world_tick, 50_000);
    let stock = game.shop_states[inn].inventory.clone();
    let mut game = Game::from_save(game.to_save()).unwrap();
    dispatch_next(
        &mut game,
        GameCommand::TravelFromInn {
            facility_id: inn.to_owned(),
            destination_town_id: "demo.town.outpost".to_owned(),
        },
    );
    assert_eq!(game.current_town().unwrap().id, "demo.town.outpost");
    assert!(
        projected_shop(&game.snapshot().shops, WHITE_HORSE_INN_ID)
            .inn_travel_destinations
            .iter()
            .any(|destination| destination.town_id == town)
    );
    dispatch_next(
        &mut game,
        GameCommand::TravelFromInn {
            facility_id: WHITE_HORSE_INN_ID.to_owned(),
            destination_town_id: town.to_owned(),
        },
    );
    assert_eq!(game.current_town().unwrap().id, town);
    assert_eq!(game.gold, 0);
    assert!(projected_shop(&game.snapshot().shops, inn).player_at_entrance);
    assert_eq!(game.shop_states[inn].inventory, stock);
    assert_eq!(
        Game::from_save(game.to_save()).unwrap().state_hash(),
        game.state_hash()
    );
}

#[test]
fn telmora_and_angwil_home_and_museum_use_existing_storage() {
    for (town, world_position, facility, storage) in [
        (
            "demo.town.telmora",
            Position { x: 87, y: 49 },
            "demo.town-facility.telmora-home",
            HOME_ID,
        ),
        (
            "demo.town.telmora",
            Position { x: 87, y: 49 },
            "demo.town-facility.telmora-museum",
            THALOS_MUSEUM_ID,
        ),
        (
            "demo.town.angwil",
            Position { x: 74, y: 23 },
            "demo.town-facility.angwil-home",
            HOME_ID,
        ),
        (
            "demo.town.angwil",
            Position { x: 74, y: 23 },
            "demo.town-facility.angwil-museum",
            THALOS_MUSEUM_ID,
        ),
    ] {
        let mut game = Game::new_with_build(51, "demo.build.warrior").unwrap();
        enter_town(&mut game, town, world_position);
        let entrance = game
            .content
            .town_facility(facility)
            .unwrap()
            .entrance_position;
        let position = Position {
            x: i32::from(entrance.x),
            y: i32::from(entrance.y),
        };
        game.player.position = game
            .town_local_to_wilderness_view_position(town, position)
            .unwrap();
        game.mark_shop_visited_at_player().unwrap();
        let home = game
            .snapshot()
            .homes
            .into_iter()
            .find(|home| home.id == facility)
            .unwrap();
        assert!(home.player_at_entrance);
        let item = home.deposit_items[0].clone();
        dispatch_next(
            &mut game,
            GameCommand::DepositAtHome {
                facility_id: facility.to_owned(),
                item_id: item.id,
                quantity: 1,
            },
        );
        assert_eq!(game.home_states.len(), 2);
        assert_eq!(game.home_states[storage].inventory.len(), 1);
        assert!(!game.home_states.contains_key(facility));
        let mut game = Game::from_save(game.to_save()).unwrap();
        let home = game
            .snapshot()
            .homes
            .into_iter()
            .find(|home| home.id == facility)
            .unwrap();
        let stored = &home.stored_items[0];
        assert!(stored.details.is_some());
        dispatch_next(
            &mut game,
            GameCommand::WithdrawFromHome {
                facility_id: facility.to_owned(),
                item_id: stored.id.clone(),
                quantity: 1,
            },
        );
        assert!(game.home_states[storage].inventory.is_empty());
        assert_eq!(
            Game::from_save(game.to_save()).unwrap().state_hash(),
            game.state_hash()
        );
    }
}

#[test]
fn telmora_and_angwil_research_and_assessment_charge_the_source_quotes() {
    for (town, suffix, base_cost) in [
        ("telmora", "library", 2000),
        ("telmora", "beastmaster", 10000),
        ("telmora", "weapon-master", 1000),
        ("angwil", "library", 1500),
        ("angwil", "beastmaster", 1500),
        ("angwil", "weapon-master", 400),
    ] {
        let id = format!("demo.town-facility.{town}-{suffix}");
        let mut game = town_facility_game(51, "demo.build.warrior", &id);
        let item_id = game
            .items
            .iter()
            .find(|item| item.location == ItemLocation::Inventory)
            .unwrap()
            .id
            .clone();
        game.item_property_knowledge.remove(&item_id);
        let projection = game
            .snapshot()
            .task_services
            .into_iter()
            .find(|service| service.id == id)
            .unwrap();
        assert!(projection.player_at_entrance && projection.tasks.is_empty());
        let cost = match suffix {
            "library" => projection.research_item_cost.unwrap(),
            "beastmaster" => projection.research_monster_cost.unwrap(),
            _ => {
                projection
                    .service_actions
                    .iter()
                    .find(|service| service.kind == FacilityServiceKindDto::AssessArmor)
                    .unwrap()
                    .cost
            }
        };
        assert_eq!(cost, game.town_service_price(base_cost));
        game.gold = cost - 1;
        let before = game.state_hash();
        let rejected = match suffix {
            "library" => game.research_item_at_facility(&id, &item_id).map(|_| ()),
            "beastmaster" => {
                game.research_monster_at_facility(&id, "demo.actor.sheep", &mut Vec::new())
            }
            _ => game
                .use_town_facility_service(
                    &id,
                    FacilityServiceKindDto::AssessArmor,
                    None,
                    None,
                    &mut Vec::new(),
                )
                .map(|_| ()),
        };
        assert_eq!(rejected, Err("insufficient-gold"));
        assert_eq!(game.state_hash(), before);
        game.gold = cost;
        let tick = game.world_tick;
        let rng = game.rng.clone();
        let command = match suffix {
            "library" => GameCommand::ResearchItemAtFacility {
                facility_id: id.clone(),
                item_id: item_id.clone(),
            },
            "beastmaster" => GameCommand::ResearchMonsterAtFacility {
                facility_id: id.clone(),
                actor_kind_id: "demo.actor.sheep".to_owned(),
            },
            _ => GameCommand::UseFacilityService {
                facility_id: id.clone(),
                service: FacilityServiceKindDto::AssessArmor,
                item_id: None,
                enchantment_steps: None,
            },
        };
        dispatch_next(&mut game, command);
        assert_eq!(game.gold, 0, "{suffix}");
        assert_eq!(game.world_tick, tick);
        assert_eq!(game.rng, rng);
        if suffix == "library" {
            assert!(game.item_property_knowledge[&item_id].identified);
        }
        if suffix == "beastmaster" {
            assert!(game.probed_actor_kind_ids.contains("demo.actor.sheep"));
        }
        assert_eq!(
            Game::from_save(game.to_save()).unwrap().state_hash(),
            game.state_hash()
        );
    }
}

#[test]
fn telmora_and_angwil_paladin_guild_enchants_equipped_armor_at_the_selected_membership_tier() {
    for (id, build, membership, base_cost) in [
        (
            "demo.town-facility.telmora-paladin-guild",
            "demo.build.paladin-death",
            FacilityMembershipDto::Owner,
            300,
        ),
        (
            "demo.town-facility.telmora-paladin-guild",
            "demo.build.warrior",
            FacilityMembershipDto::Visitor,
            600,
        ),
        (
            "demo.town-facility.angwil-paladin-guild",
            "demo.build.paladin-death",
            FacilityMembershipDto::Owner,
            240,
        ),
        (
            "demo.town-facility.angwil-paladin-guild",
            "demo.build.warrior",
            FacilityMembershipDto::Visitor,
            440,
        ),
    ] {
        let mut game = town_facility_game(51, build, id);
        support::give_inventory_item(&mut game, "test.telmora.food", "demo.item.ration-of-food");
        let guild = game
            .snapshot()
            .task_services
            .into_iter()
            .find(|service| service.id == id)
            .unwrap();
        assert_eq!(guild.membership, membership);
        let action = guild
            .service_actions
            .iter()
            .find(|action| action.kind == FacilityServiceKindDto::EnchantArmor)
            .unwrap();
        assert_eq!(action.cost, game.town_service_price(base_cost));
        let target = action
            .targets
            .iter()
            .find(|target| {
                game.items.iter().any(|item| {
                    item.id == target.item_id
                        && matches!(item.location, ItemLocation::Equipped { .. })
                })
            })
            .unwrap();
        let choice = &target.choices[1];
        assert_eq!(choice.steps, 2);
        game.gold = choice.cost;
        let before = game.state_hash();
        assert_eq!(
            game.use_town_facility_service(
                id,
                FacilityServiceKindDto::EnchantArmor,
                Some("test.telmora.food"),
                Some(2),
                &mut Vec::new()
            )
            .unwrap_err(),
            "item-unavailable"
        );
        assert_eq!(game.state_hash(), before);
        game.gold -= 1;
        let before = game.state_hash();
        assert_eq!(
            game.use_town_facility_service(
                id,
                FacilityServiceKindDto::EnchantArmor,
                Some(&target.item_id),
                Some(2),
                &mut Vec::new()
            )
            .unwrap_err(),
            "insufficient-gold"
        );
        assert_eq!(game.state_hash(), before);
        game.gold += 1;
        let result = dispatch_next(
            &mut game,
            GameCommand::UseFacilityService {
                facility_id: id.to_owned(),
                service: FacilityServiceKindDto::EnchantArmor,
                item_id: Some(target.item_id.clone()),
                enchantment_steps: Some(2),
            },
        );
        assert!(
            result
                .events
                .iter()
                .any(|event| event.kind == "facility.item-enchanted")
        );
        assert_eq!(game.gold, 0);
        assert_eq!(
            game.items
                .iter()
                .find(|item| item.id == target.item_id)
                .unwrap()
                .enchantments
                .to_armor,
            choice.result.to_armor
        );
        assert_eq!(
            Game::from_save(game.to_save()).unwrap().state_hash(),
            game.state_hash()
        );
    }
}

#[test]
fn telmora_life_temple_heals_owner_and_visitor_for_the_projected_cost() {
    let id = "demo.town-facility.telmora-life-temple";
    for (build, membership, base_cost) in [
        ("demo.build.high-mage-life", FacilityMembershipDto::Owner, 0),
        (
            "demo.build.paladin-death",
            FacilityMembershipDto::Visitor,
            150,
        ),
    ] {
        let mut game = town_facility_game(51, build, id);
        let temple = game
            .snapshot()
            .task_services
            .into_iter()
            .find(|service| service.id == id)
            .unwrap();
        assert_eq!(temple.membership, membership);
        let heal = temple
            .service_actions
            .iter()
            .find(|action| action.kind == FacilityServiceKindDto::Heal)
            .unwrap();
        assert_eq!(heal.cost, game.town_service_price(base_cost));
        game.gold = heal.cost;
        game.player.hp = 1;
        dispatch_next(
            &mut game,
            GameCommand::UseFacilityService {
                facility_id: id.to_owned(),
                service: FacilityServiceKindDto::Heal,
                item_id: None,
                enchantment_steps: None,
            },
        );
        assert_eq!(game.gold, 0);
        assert_eq!(game.player.hp, game.effective_player_max_hp());
    }
}

#[test]
fn angwil_inner_temple_uses_class_membership_for_healing_and_restoration() {
    let id = "demo.town-facility.angwil-inner-temple";
    for (build, membership) in [
        ("demo.build.paladin-death", FacilityMembershipDto::Member),
        ("demo.build.high-mage-life", FacilityMembershipDto::Visitor),
    ] {
        for (kind, base_cost) in [
            (FacilityServiceKindDto::Heal, 150),
            (FacilityServiceKindDto::RestoreVitality, 1000),
        ] {
            let mut game = town_facility_game(51, build, id);
            game.player.hp = 1;
            game.progress.attributes.strength =
                game.progress.maximum_attributes.strength.saturating_sub(1);
            game.progress.life_force = 900;
            let projection = game
                .snapshot()
                .task_services
                .into_iter()
                .find(|s| s.id == id)
                .unwrap();
            assert_eq!(projection.membership, membership);
            let cost = projection
                .service_actions
                .iter()
                .find(|action| action.kind == kind)
                .unwrap()
                .cost;
            assert_eq!(cost, game.town_service_price(base_cost));
            game.gold = cost - 1;
            let before = game.state_hash();
            assert_eq!(
                game.use_town_facility_service(id, kind, None, None, &mut Vec::new())
                    .unwrap_err(),
                "insufficient-gold"
            );
            assert_eq!(game.state_hash(), before);
            game.gold = cost;
            let tick = game.world_tick;
            let rng = game.rng.clone();
            dispatch_next(
                &mut game,
                GameCommand::UseFacilityService {
                    facility_id: id.to_owned(),
                    service: kind,
                    item_id: None,
                    enchantment_steps: None,
                },
            );
            assert_eq!(game.gold, 0);
            assert_eq!(game.world_tick, tick);
            assert_eq!(game.rng, rng);
            if kind == FacilityServiceKindDto::Heal {
                assert_eq!(game.player.hp, game.effective_player_max_hp());
            } else {
                assert_eq!(game.progress.attributes, game.progress.maximum_attributes);
                assert_eq!(game.progress.life_force, 1000);
            }
            let restored = Game::from_save(game.to_save()).unwrap();
            assert_eq!(restored.state_hash(), game.state_hash());
            assert_eq!(
                restored
                    .snapshot()
                    .task_services
                    .iter()
                    .find(|s| s.id == id)
                    .unwrap()
                    .membership,
                membership
            );
        }
    }
}

#[test]
fn angwil_trump_tower_prices_and_recall_survive_save_and_return() {
    let id = "demo.town-facility.angwil-trump-tower";
    // Trump is not a formal player build yet. This validated fixture supplies
    // only a realm identity to exercise the real membership and recall paths.
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&root).unwrap();
    // New realm identities need both high-book reward ranks required by the world.
    for (rank, source) in [(3, "black-prayers"), (4, "necronomicon")] {
        let book_id = format!("test.ability-book.trump-{rank}");
        let mut book = artifact
            .content
            .ability_books
            .iter()
            .find(|book| book.id == format!("demo.ability-book.{source}"))
            .unwrap()
            .clone();
        book.id = book_id.clone();
        book.realm_id = Some("trump".to_owned());
        book.rank = Some(rank);
        book.ability_ids.truncate(1);
        artifact.content.ability_books.push(book);
        let mut item = artifact
            .content
            .items
            .iter()
            .find(|item| item.id == format!("demo.item.{source}"))
            .unwrap()
            .clone();
        item.id = format!("test.item.trump-book-{rank}");
        item.rfb_base_kind = None;
        item.ability_book_id = Some(book_id);
        artifact.content.items.push(item);
    }
    let profile = artifact
        .content
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.high-mage")
        .unwrap()
        .casting_profile
        .as_mut()
        .unwrap();
    profile
        .realm_profiles
        .push(rfb_content::CastingRealmProfileDefinition {
            realm_id: "trump".to_owned(),
            ability_book_ids: vec![
                "test.ability-book.trump-3".to_owned(),
                "test.ability-book.trump-4".to_owned(),
            ],
            learning_capacity_bonus: 0,
            ability_overrides: Vec::new(),
        });
    let mut build = artifact
        .content
        .builds
        .iter()
        .find(|build| build.id == "demo.build.high-mage-death")
        .unwrap()
        .clone();
    build.id = "test.build.trump".to_owned();
    build.first_realm_id = Some("trump".to_owned());
    build.starting_items.clear();
    artifact.content.builds.push(build);
    let content = Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content).unwrap(),
    ));
    for (build, amberite, membership, base_cost) in [
        ("test.build.trump", false, FacilityMembershipDto::Owner, 0),
        (
            "demo.build.warrior",
            false,
            FacilityMembershipDto::Visitor,
            150,
        ),
        (
            "demo.build.warrior",
            true,
            FacilityMembershipDto::Member,
            150,
        ),
    ] {
        let mut game = if build == "test.build.trump" {
            let mut game =
                Game::from_content_with_build(51, content.clone(), DEFAULT_WORLD_ID, build)
                    .unwrap();
            enter_town_facility(&mut game, id);
            game
        } else {
            town_facility_game(51, build, id)
        };
        if amberite {
            let mut form =
                monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 20, "test.angwil").status;
            form.granted_race_id = Some("rfb-legacy.race.amberite".to_owned());
            game.player.statuses.push(form);
        }
        let tower = game
            .snapshot()
            .task_services
            .into_iter()
            .find(|s| s.id == id)
            .unwrap();
        assert_eq!(tower.membership, membership);
        assert_eq!(tower.service_actions.len(), 1);
        assert_eq!(
            tower.service_actions[0].kind,
            FacilityServiceKindDto::Recall
        );
        let cost = tower.service_actions[0].cost;
        assert_eq!(cost, game.town_service_price(base_cost));
        let departure = game.player.position;
        game.recall = Some(RecallStateDto {
            destination: Some(rfb_protocol::RecallDestinationDto {
                dungeon_id: "demo.dungeon.tidal-cave".to_owned(),
                floor_id: "demo.floor.tidal-cave-depth-15".to_owned(),
            }),
            remaining_turns: None,
        });
        if cost > 0 {
            game.gold = cost - 1;
            let before = game.state_hash();
            assert_eq!(
                game.use_town_facility_service(
                    id,
                    FacilityServiceKindDto::Recall,
                    None,
                    None,
                    &mut Vec::new()
                )
                .unwrap_err(),
                "insufficient-gold"
            );
            assert_eq!(game.state_hash(), before);
        }
        game.gold = cost;
        let update = dispatch_next(
            &mut game,
            GameCommand::UseFacilityService {
                facility_id: id.to_owned(),
                service: FacilityServiceKindDto::Recall,
                item_id: None,
                enchantment_steps: None,
            },
        );
        assert!(
            update
                .events
                .iter()
                .any(|event| event.kind == "facility.recall-started")
        );
        assert_eq!(game.gold, 0);
        assert_eq!(game.recall.as_ref().unwrap().remaining_turns, Some(2));
        let mut game = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
        assert_eq!(
            game.snapshot()
                .task_services
                .iter()
                .find(|s| s.id == id)
                .unwrap()
                .membership,
            membership
        );
        dispatch_next(&mut game, GameCommand::Wait);
        dispatch_next(&mut game, GameCommand::Wait);
        assert_eq!(game.current_floor_id, "demo.floor.tidal-cave-depth-15");
        let mut game = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
        game.entities.clear();
        game.start_recall(0);
        dispatch_next(&mut game, GameCommand::Wait);
        assert_eq!(game.current_town().unwrap().id, "demo.town.angwil");
        assert_eq!(game.player.position, departure);
        assert_eq!(
            Game::from_save_with_content(game.to_save(), game.content.clone())
                .unwrap()
                .state_hash(),
            game.state_hash()
        );
    }
}

#[test]
fn morivant_level_teleport_validates_choices_before_charging_and_resumes_after_save() {
    let facility_id = "demo.town-facility.morivant-trump-tower";
    let dungeon_id = "demo.dungeon.tidal-cave";
    let mut game = town_facility_game(51, "demo.build.warrior", facility_id);
    let departure = game.player.position;
    game.gold = game.town_service_price(100_000);
    assert!(game.teleport_dungeon_dtos().is_empty());
    let before = game.state_hash();
    assert_eq!(
        game.teleport_to_dungeon_level_at_facility(facility_id, dungeon_id, 20),
        Err("recall-unavailable")
    );
    assert_eq!(game.state_hash(), before);
    game.reset_recall(super::super::floor::RecallDestination {
        dungeon_id: dungeon_id.to_owned(),
        floor_id: "demo.floor.tidal-cave-depth-15".to_owned(),
    });
    let before = game.state_hash();
    let service = game
        .snapshot()
        .task_services
        .into_iter()
        .find(|service| service.id == facility_id)
        .unwrap();
    assert_eq!(service.teleport_level_cost, Some(140_000));
    let dungeon = &service.teleport_dungeons[0];
    assert_eq!(dungeon.recall_depth, 15);
    assert_eq!(dungeon.depths, (15..=27).collect::<Vec<_>>());
    // Browsing/closing the chooser never dispatches a paid action.
    assert_eq!(game.state_hash(), before);
    for depth in [0, 14, 28, u16::MAX] {
        assert_eq!(
            game.teleport_to_dungeon_level_at_facility(facility_id, dungeon_id, depth),
            Err("recall-unavailable")
        );
        assert_eq!(game.state_hash(), before);
    }
    game.dungeon_states.get_mut(dungeon_id).unwrap().suppressed = true;
    assert!(game.teleport_dungeon_dtos().is_empty());
    assert_eq!(
        game.teleport_to_dungeon_level_at_facility(facility_id, dungeon_id, 20),
        Err("recall-unavailable")
    );
    game.dungeon_states.get_mut(dungeon_id).unwrap().suppressed = false;
    game.gold -= 1;
    assert_eq!(
        game.teleport_to_dungeon_level_at_facility(facility_id, dungeon_id, 20),
        Err("insufficient-gold")
    );
    game.gold += 1;
    game.player.position.x += 1;
    assert_eq!(
        game.teleport_to_dungeon_level_at_facility(facility_id, dungeon_id, 20),
        Err("facility-unreachable")
    );
    game.player.position = departure;
    game.start_recall(25);
    dispatch_next(
        &mut game,
        GameCommand::TeleportToDungeonLevelAtFacility {
            facility_id: facility_id.to_owned(),
            dungeon_id: dungeon_id.to_owned(),
            depth: 20,
        },
    );
    assert_eq!(game.gold, 0);
    assert_eq!(game.recall.as_ref().unwrap().remaining_turns, Some(1));
    assert_eq!(
        game.dungeon_states[dungeon_id].recall_floor_id.as_deref(),
        Some("demo.floor.tidal-cave-depth-20")
    );
    let mut game = Game::from_save(game.to_save()).unwrap();
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(game.current_floor_id, "demo.floor.tidal-cave-depth-20");
    let mut game = Game::from_save(game.to_save()).unwrap();
    game.entities.clear();
    game.start_recall(0);
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(game.current_town().unwrap().id, MORIVANT_TOWN_ID);
    assert_eq!(game.player.position, departure);
    assert_eq!(game.wilderness_position, Some(Position { x: 47, y: 50 }));
    assert_eq!(
        Game::from_save(game.to_save()).unwrap().state_hash(),
        game.state_hash()
    );
}

#[test]
fn dungeon_recall_records_survive_switching_dungeons_and_allow_explicit_lowering() {
    let mut game = Game::new_with_build(51, "demo.build.warrior").unwrap();
    let surface = game.current_floor_id.clone();
    for floor_id in [
        "demo.floor.tidal-cave-depth-20",
        "demo.floor.warrens-depth-1",
        "demo.floor.tidal-cave-depth-15",
    ] {
        game.current_floor_id = floor_id.to_owned();
        game.update_recall_destination_for_current_floor();
    }
    assert_eq!(
        game.recall
            .as_ref()
            .unwrap()
            .destination
            .as_ref()
            .unwrap()
            .floor_id,
        "demo.floor.tidal-cave-depth-20"
    );
    game.reset_recall(game.recall_reset_plan().unwrap());
    assert_eq!(
        game.dungeon_states["demo.dungeon.tidal-cave"]
            .recall_floor_id
            .as_deref(),
        Some("demo.floor.tidal-cave-depth-15")
    );
    game.current_floor_id = surface;
    let mut saved = game.to_save();
    assert_eq!(
        Game::from_save(saved.clone()).unwrap().state_hash(),
        game.state_hash()
    );
    saved
        .dungeon_states
        .iter_mut()
        .find(|state| state.dungeon_id == "demo.dungeon.tidal-cave")
        .unwrap()
        .recall_floor_id = Some("demo.floor.warrens-depth-1".to_owned());
    assert!(matches!(
        Game::from_save(saved),
        Err(CoreError::InvalidSave("dungeon recall floor is invalid"))
    ));
}

#[test]
fn morivant_recall_resumes_after_save_and_returns_to_the_departure_position() {
    let mut game = Game::new_with_build(51, "demo.build.warrior").unwrap();
    enter_morivant(&mut game);
    game.player.position = game
        .town_local_to_wilderness_view_position(MORIVANT_TOWN_ID, Position { x: 55, y: 15 })
        .unwrap();
    let departure = game.player.position;
    game.recall = Some(RecallStateDto {
        destination: Some(rfb_protocol::RecallDestinationDto {
            dungeon_id: "demo.dungeon.tidal-cave".to_owned(),
            floor_id: "demo.floor.tidal-cave-depth-15".to_owned(),
        }),
        remaining_turns: None,
    });
    game.gold = game.town_service_price(50);
    dispatch_next(
        &mut game,
        GameCommand::UseFacilityService {
            facility_id: "demo.town-facility.morivant-trump-tower".to_owned(),
            service: FacilityServiceKindDto::Recall,
            item_id: None,
            enchantment_steps: None,
        },
    );
    assert_eq!(game.gold, 0);
    let mut game = Game::from_save(game.to_save()).unwrap();
    dispatch_next(&mut game, GameCommand::Wait);
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(game.current_floor_id, "demo.floor.tidal-cave-depth-15");
    let mut game = Game::from_save(game.to_save()).unwrap();
    game.entities.clear();
    game.start_recall(0);
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(game.current_town().unwrap().id, MORIVANT_TOWN_ID);
    assert_eq!(game.wilderness_position, Some(Position { x: 47, y: 50 }));
    assert_eq!(game.player.position, departure);
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
}

fn projected_shop<'a>(shops: &'a [ShopDto], shop_id: &str) -> &'a ShopDto {
    shops
        .iter()
        .find(|shop| shop.id == shop_id)
        .expect("requested shop should be projected")
}

fn store_game(seed: u64) -> Game {
    let mut game =
        Game::new_with_build(seed, "demo.build.warrior").expect("Warrens game should start");
    game.player.position = Position { x: 70, y: 39 };
    game.mark_shop_visited_at_player().unwrap();
    game
}

#[test]
fn i6_shop_quotes_keep_discounts_separate_and_purchase_blends_only_the_transfer() {
    let mut game = store_game(606);
    game.items.clear();
    game.item_property_knowledge.clear();
    game.gold = 10_000;
    for id in ["test.full", "test.discount", "test.carried"] {
        super::support::give_inventory_item(&mut game, id, "demo.item.mithril-arrow");
    }
    game.items[0].quantity = 2;
    game.items[1].quantity = 3;
    game.items[1].origin_kind = Some(ItemOriginKindDto::PlayerMade);
    game.items[1].discount_percent = 99;
    game.items[2].origin_kind = Some(ItemOriginKindDto::Acquire);
    game.items[2].inscription = Some("keep".into());
    game.shop_states
        .get_mut(GENERAL_STORE_ID)
        .unwrap()
        .inventory = game.items.drain(..2).collect();
    let before = game.snapshot();
    let stock = &projected_shop(&before.shops, GENERAL_STORE_ID).stock;
    assert_eq!(stock.len(), 2);
    let full = stock.iter().find(|item| item.id == "test.full").unwrap();
    let discount = stock
        .iter()
        .find(|item| item.id == "test.discount")
        .unwrap();
    assert_eq!(full.quantity, 2);
    assert_eq!(discount.quantity, 3);
    assert!(full.unit_price > discount.unit_price);
    let gold_before = game.gold;
    let update = dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: GENERAL_STORE_ID.into(),
            item_id: discount.id.clone(),
            quantity: 2,
        },
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "shop.purchase")
    );
    assert_eq!(game.gold, gold_before - 2 * discount.unit_price);
    assert_eq!(game.items.len(), 1);
    let merged = &game.items[0];
    assert_eq!(merged.id, "test.carried");
    assert_eq!(merged.quantity, 3);
    assert_eq!(merged.origin_kind, Some(ItemOriginKindDto::Mixed));
    assert_eq!(merged.discount_percent, 99);
    assert_eq!(merged.inscription.as_deref(), Some("keep"));
    let after = game.snapshot();
    let shop = projected_shop(&after.shops, GENERAL_STORE_ID);
    assert_eq!(shop.stock.len(), 2);
    assert_eq!(
        shop.stock
            .iter()
            .find(|item| item.id == "test.full")
            .unwrap()
            .quantity,
        2
    );
    assert_eq!(
        shop.stock
            .iter()
            .find(|item| item.id == "test.discount")
            .unwrap()
            .quantity,
        1
    );
    assert_eq!(
        shop.sell_quotes
            .iter()
            .find(|quote| quote.item_id == "test.carried")
            .unwrap()
            .unit_price,
        1
    );
    let mut restored = Game::from_save(game.to_save()).unwrap();
    for current in [&mut game, &mut restored] {
        let update = dispatch_next(
            current,
            GameCommand::SellToShop {
                shop_id: GENERAL_STORE_ID.into(),
                item_id: "test.carried".into(),
                quantity: 1,
            },
        );
        assert!(update.events.iter().any(|event| event.kind == "shop.sale"));
    }
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(game.items[0].quantity, 2);
}

#[test]
fn i6_home_withdrawal_merges_into_a_full_pack_and_preserves_saved_metadata() {
    let mut game = Game::new_with_build(606, "demo.build.warrior").unwrap();
    game.player.position = Position { x: 110, y: 44 };
    game.mark_shop_visited_at_player().unwrap();
    game.items.clear();
    game.item_property_knowledge.clear();
    for id in ["test.stored", "test.carried"] {
        super::support::give_inventory_item(&mut game, id, "demo.item.ration-of-food");
    }
    game.items[0].quantity = 2;
    game.items[0].origin_kind = Some(ItemOriginKindDto::Acquire);
    game.items[0].inscription = Some("keep".into());
    let deposit = dispatch_next(
        &mut game,
        GameCommand::DepositAtHome {
            facility_id: HOME_ID.into(),
            item_id: "test.stored".into(),
            quantity: 2,
        },
    );
    assert!(
        deposit
            .events
            .iter()
            .any(|event| event.kind == "home.deposit")
    );
    for index in 0..25 {
        super::support::give_inventory_item(
            &mut game,
            &format!("test.filler.{index}"),
            "demo.item.dagger",
        );
    }
    assert_eq!(game.inventory_used_slots(), game.inventory_slot_capacity());
    let mut restored = Game::from_save(game.to_save()).unwrap();
    for current in [&mut game, &mut restored] {
        let update = dispatch_next(
            current,
            GameCommand::WithdrawFromHome {
                facility_id: HOME_ID.into(),
                item_id: "test.stored".into(),
                quantity: 2,
            },
        );
        assert!(
            update
                .events
                .iter()
                .any(|event| event.kind == "home.withdraw")
        );
        let merged = current
            .items
            .iter()
            .find(|item| item.id == "test.carried")
            .unwrap();
        assert_eq!(merged.quantity, 3);
        assert_eq!(merged.origin_kind, Some(ItemOriginKindDto::Mixed));
        assert_eq!(merged.inscription.as_deref(), Some("keep"));
        assert_eq!(
            current.inventory_used_slots(),
            current.inventory_slot_capacity()
        );
        assert!(current.snapshot().homes[0].stored_items.is_empty());
    }
    assert_eq!(restored.state_hash(), game.state_hash());
    assert!(Game::from_save(game.to_save()).is_ok());
}

fn anambar_inn_game(seed: u64) -> Game {
    let mut game =
        Game::new_with_build(seed, "demo.build.warrior").expect("Middle-earth game should start");
    dispatch_next(
        &mut game,
        GameCommand::EnterWorldMap {
            leave_pets: false,
            cancel_recall: false,
        },
    );
    game.wilderness_position = Some(Position { x: 26, y: 39 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    game.player.position = game
        .shop_entrance_position(game.content.shop(ANAMBAR_INN_ID).unwrap())
        .unwrap();
    game.mark_shop_visited_at_player().unwrap();
    assert!(projected_shop(&game.snapshot().shops, ANAMBAR_INN_ID).player_at_entrance);
    game
}

fn thalos_game(seed: u64) -> Game {
    let mut game =
        Game::new_with_build(seed, "demo.build.warrior").expect("Middle-earth game should start");
    dispatch_next(
        &mut game,
        GameCommand::EnterWorldMap {
            leave_pets: false,
            cancel_recall: false,
        },
    );
    game.wilderness_position = Some(Position { x: 17, y: 29 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    assert_eq!(
        game.current_town().map(|town| town.id.as_str()),
        Some("demo.town.thalos")
    );
    game
}

fn anambar_library_game(seed: u64) -> Game {
    let mut game =
        Game::new_with_build(seed, "demo.build.warrior").expect("Middle-earth game should start");
    dispatch_next(
        &mut game,
        GameCommand::EnterWorldMap {
            leave_pets: false,
            cancel_recall: false,
        },
    );
    game.wilderness_position = Some(Position { x: 26, y: 39 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    game.player.position = game
        .town_facility_entrance_position(game.content.town_facility(ANAMBAR_LIBRARY_ID).unwrap())
        .unwrap();
    assert!(
        game.snapshot()
            .task_services
            .iter()
            .any(|service| service.id == ANAMBAR_LIBRARY_ID && service.player_at_entrance)
    );
    game
}

fn anambar_facility_game(
    seed: u64,
    build_id: &str,
    race_id: Option<&str>,
    facility_id: &str,
) -> Game {
    let mut game = match race_id {
        Some(race_id) => {
            Game::new_with_build_race_and_name(seed, build_id, race_id, Game::DEFAULT_PLAYER_NAME)
        }
        None => Game::new_with_build(seed, build_id),
    }
    .expect("Middle-earth game should start");
    dispatch_next(
        &mut game,
        GameCommand::EnterWorldMap {
            leave_pets: false,
            cancel_recall: false,
        },
    );
    game.wilderness_position = Some(Position { x: 26, y: 39 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    game.player.position = game
        .town_facility_entrance_position(game.content.town_facility(facility_id).unwrap())
        .unwrap();
    assert!(
        game.snapshot()
            .task_services
            .iter()
            .any(|service| service.id == facility_id && service.player_at_entrance)
    );
    game
}

fn add_bounty_remains(game: &mut Game, id: &str, actor_kind_id: &str) {
    let mut remains = game
        .items
        .first()
        .expect("a new character should carry an item")
        .clone();
    remains.id = id.to_owned();
    remains.kind_id = "demo.item.corpse-remains".to_owned();
    remains.quantity = 1;
    remains.origin_actor_kind_id = Some(actor_kind_id.to_owned());
    remains.origin_kind = None;
    remains.affix_ids.clear();
    remains.rolled_affixes.clear();
    remains.activation = None;
    remains.charges = None;
    remains.fuel = None;
    remains.captured_actor = None;
    remains.location = ItemLocation::Inventory;
    game.items.push(remains);
}

fn white_horse_inn_game(seed: u64) -> Game {
    let mut game =
        Game::new_with_build(seed, "demo.build.warrior").expect("Middle-earth game should start");
    game.player.position = Position { x: 124, y: 35 };
    game.mark_shop_visited_at_player().unwrap();
    assert!(projected_shop(&game.snapshot().shops, WHITE_HORSE_INN_ID).player_at_entrance);
    game
}

fn outpost_count_game(seed: u64) -> Game {
    let mut game =
        Game::new_with_build(seed, "demo.build.warrior").expect("Middle-earth game should start");
    game.player.position = Position { x: 98, y: 23 };
    game
}

fn stock_item_id(game: &Game, kind_id: &str) -> String {
    game.shop_states[GENERAL_STORE_ID]
        .inventory
        .iter()
        .find(|item| item.kind_id == kind_id)
        .expect("store should stock requested kind")
        .id
        .clone()
}

#[test]
fn outpost_count_three_doors_share_one_task_service_and_accepted_task() {
    let mut game = outpost_count_game(42);
    game.player.position = Position { x: 97, y: 23 };
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: OUTPOST_COUNT_ID.to_owned(),
            task_id: "demo.task.thieves-hideout".to_owned(),
        },
    );
    let tasks = game.task_states.clone();
    let draws = game.rng_draw_counter();
    for x in [97, 98, 99] {
        game.player.position = Position { x, y: 23 };
        let services = game
            .snapshot()
            .task_services
            .into_iter()
            .filter(|service| service.id == OUTPOST_COUNT_ID)
            .collect::<Vec<_>>();
        assert_eq!(services.len(), 1);
        assert!(services[0].player_at_entrance);
        assert_eq!(
            services[0]
                .tasks
                .iter()
                .find(|task| task.task_id == "demo.task.thieves-hideout")
                .unwrap()
                .status,
            TaskStatusKindDto::Taken
        );
        assert_eq!(game.task_states, tasks);
        assert_eq!(game.rng_draw_counter(), draws);
    }
}

#[test]
fn outpost_shops_are_projected_from_authoritative_content() {
    let game = Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    let snapshot = game.snapshot();
    let town = snapshot.town.expect("surface should project the Outpost");
    assert_eq!(town.id, "demo.town.outpost");
    assert_eq!(town.floor_id, "demo.floor.surface");
    assert!(town.visited);
    assert_eq!(snapshot.shops.len(), 10);
    assert_eq!(snapshot.homes.len(), 2);
    assert_eq!(snapshot.homes[0].id, HOME_ID);
    assert_eq!(
        snapshot.homes[0].entrance_position,
        Position { x: 110, y: 44 }
    );
    assert!(!snapshot.homes[0].visited);
    let museum = snapshot
        .homes
        .iter()
        .find(|home| home.id == OUTPOST_MUSEUM_ID)
        .unwrap();
    assert_eq!(museum.entrance_position, Position { x: 97, y: 46 });
    assert!(museum.museum);
    assert!(game.has_shared_museum());
    assert_eq!(game.home_states.len(), 2);
    assert!(!game.home_states.contains_key(OUTPOST_MUSEUM_ID));
    let general_store = projected_shop(&snapshot.shops, GENERAL_STORE_ID);
    assert_eq!(general_store.entrance_position, Position { x: 70, y: 39 });
    assert_eq!(
        general_store.entrance_terrain_id,
        "demo.terrain.general-store-entrance"
    );
    assert_eq!(general_store.category, ShopCategoryDto::GeneralStore);
    let temple = projected_shop(&snapshot.shops, TEMPLE_ID);
    assert_eq!(temple.entrance_position, Position { x: 70, y: 29 });
    assert_eq!(temple.category, ShopCategoryDto::Temple);
    let alchemist = projected_shop(&snapshot.shops, ALCHEMIST_ID);
    assert_eq!(alchemist.entrance_position, Position { x: 74, y: 43 });
    assert_eq!(alchemist.category, ShopCategoryDto::Alchemist);
    let magic_shop = projected_shop(&snapshot.shops, MAGIC_SHOP_ID);
    assert_eq!(magic_shop.entrance_position, Position { x: 84, y: 43 });
    assert_eq!(magic_shop.category, ShopCategoryDto::MagicShop);
    let bookstore = projected_shop(&snapshot.shops, BOOKSTORE_ID);
    assert_eq!(bookstore.entrance_position, Position { x: 89, y: 44 });
    assert_eq!(bookstore.category, ShopCategoryDto::Bookstore);
    let armoury = projected_shop(&snapshot.shops, ARMOURY_ID);
    assert_eq!(armoury.entrance_position, Position { x: 115, y: 28 });
    assert_eq!(armoury.category, ShopCategoryDto::Armoury);
    let weaponsmith = projected_shop(&snapshot.shops, WEAPONSMITH_ID);
    assert_eq!(weaponsmith.entrance_position, Position { x: 126, y: 31 });
    assert_eq!(weaponsmith.category, ShopCategoryDto::Weaponsmith);
    let black_market = projected_shop(&snapshot.shops, BLACK_MARKET_ID);
    assert_eq!(black_market.entrance_position, Position { x: 115, y: 43 });
    assert_eq!(black_market.category, ShopCategoryDto::BlackMarket);
    let shroomery = projected_shop(&snapshot.shops, SHROOMERY_ID);
    assert_eq!(shroomery.entrance_position, Position { x: 78, y: 26 });
    assert_eq!(shroomery.category, ShopCategoryDto::Shroomery);
    let white_horse = projected_shop(&snapshot.shops, WHITE_HORSE_INN_ID);
    assert_eq!(white_horse.entrance_position, Position { x: 124, y: 35 });
    assert_eq!(white_horse.inn_stay_cost, Some(28));
    assert!(white_horse.inn_travel_destinations.is_empty());
    assert!(
        snapshot
            .shops
            .iter()
            .all(|shop| !shop.visited && !shop.player_at_entrance)
    );
}

#[test]
fn p108c_thalos_projects_its_embedded_icky_cave_and_returns_to_town() {
    let mut game =
        Game::new_with_build(108, "demo.build.warrior").expect("Middle-earth game should start");
    dispatch_next(
        &mut game,
        GameCommand::EnterWorldMap {
            leave_pets: false,
            cancel_recall: false,
        },
    );
    let thalos_position = Position { x: 17, y: 29 };
    game.wilderness_position = Some(thalos_position);
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);

    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(game.wilderness_position, Some(thalos_position));
    assert_eq!(
        game.current_town().map(|town| town.id.as_str()),
        Some("demo.town.thalos")
    );
    assert!(game.town_states["demo.town.thalos"].visited);
    let entrance = Position { x: 164, y: 47 };
    assert_eq!(game.terrain_at(entrance), "demo.terrain.icky-cave-entrance");
    assert_eq!(
        game.terrain
            .iter()
            .filter(|terrain| terrain.as_str() == "demo.terrain.icky-cave-entrance")
            .count(),
        1
    );

    // Walk across the east scroll boundary on the source road before entering the lake cave.
    game.player.position = Position { x: 131, y: 39 };
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(game.wilderness_view_offset, Position { x: 1, y: 0 });
    game.player.position = game
        .town_local_to_active_position("demo.town.thalos", entrance)
        .unwrap();
    let entered = dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(entered.floor_id, "demo.floor.icky-cave-depth-10");
    assert_eq!(game.current_floor_id, "demo.floor.icky-cave-depth-10");
    game = Game::from_save(game.to_save()).unwrap();

    let upstairs_index = game
        .terrain
        .iter()
        .position(|terrain| terrain == "demo.terrain.stairs-up")
        .expect("Icky Cave root should have an upstairs");
    game.player.position = Position {
        x: (upstairs_index % usize::from(game.width)) as i32,
        y: (upstairs_index / usize::from(game.width)) as i32,
    };
    let returned = dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(returned.floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(game.wilderness_position, Some(thalos_position));
    assert_eq!(
        game.current_town().map(|town| town.id.as_str()),
        Some("demo.town.thalos")
    );
    let returned_entrance = game
        .town_local_to_active_position("demo.town.thalos", entrance)
        .unwrap();
    assert_eq!(game.player.position, returned_entrance);
    assert_eq!(
        game.terrain_at(returned_entrance),
        "demo.terrain.icky-cave-entrance"
    );
    assert!(Game::from_save(game.to_save()).is_ok());
}

#[test]
fn p109c_thalos_inn_travels_to_a_visited_town_for_the_projected_price() {
    let mut game = thalos_game(109);
    game.player.position = game
        .shop_entrance_position(game.content.shop(THALOS_INN_ID).unwrap())
        .unwrap();
    game.mark_shop_visited_at_player().unwrap();
    game.gold = game.town_service_price(500);

    let inn = projected_shop(&game.snapshot().shops, THALOS_INN_ID).clone();
    assert!(inn.player_at_entrance);
    assert_eq!(inn.inn_stay_cost, Some(28));
    assert_eq!(inn.inn_travel_destinations.len(), 1);
    assert_eq!(inn.inn_travel_destinations[0].town_id, "demo.town.outpost");
    assert_eq!(inn.inn_travel_destinations[0].cost, 700);

    let update = dispatch_next(
        &mut game,
        GameCommand::TravelFromInn {
            facility_id: THALOS_INN_ID.to_owned(),
            destination_town_id: "demo.town.outpost".to_owned(),
        },
    );
    assert_eq!(update.events[0].kind, "inn.travel");
    assert_eq!(update.events[0].args["cost"], "700");
    assert_eq!(game.gold, 0);
    assert_eq!(game.wilderness_position, Some(Position { x: 28, y: 52 }));
    assert_eq!(game.player.position, Position { x: 124, y: 35 });
}

#[test]
fn museums_share_ordinary_items_across_towns_and_reject_true_artifacts() {
    let mut game = Game::new_with_build(109, "demo.build.warrior").unwrap();
    enter_town_facility(&mut game, ANAMBAR_MUSEUM_ID);
    game.mark_shop_visited_at_player().unwrap();
    support::give_inventory_item(&mut game, "test.museum.dagger", "demo.item.dagger");
    game.items
        .iter_mut()
        .find(|item| item.id == "test.museum.dagger")
        .unwrap()
        .inscription = Some("keep".to_owned());
    let sacrifice = game
        .virtues
        .iter()
        .position(|virtue| virtue.kind == rfb_protocol::VirtueKindDto::Sacrifice)
        .unwrap_or(0);
    game.virtues[sacrifice].kind = rfb_protocol::VirtueKindDto::Sacrifice;
    game.virtues[sacrifice].value = 0;
    support::give_inventory_item(
        &mut game,
        "test.museum.arkenstone",
        "demo.item.arkenstone-of-thrain",
    );
    game.generated_artifact_ids
        .insert("demo.item.arkenstone-of-thrain".to_owned());

    let museum = game
        .snapshot()
        .homes
        .into_iter()
        .find(|home| home.id == ANAMBAR_MUSEUM_ID)
        .expect("Anambar Museum should be projected");
    assert!(museum.player_at_entrance);
    assert!(
        museum
            .deposit_items
            .iter()
            .any(|item| item.id == "test.museum.dagger")
    );
    assert!(
        museum
            .deposit_items
            .iter()
            .all(|item| item.id != "test.museum.arkenstone")
    );

    let rejected = dispatch_next(
        &mut game,
        GameCommand::DepositAtHome {
            facility_id: ANAMBAR_MUSEUM_ID.to_owned(),
            item_id: "test.museum.arkenstone".to_owned(),
            quantity: 1,
        },
    );
    assert_eq!(rejected.events[0].kind, "home.transfer-unavailable");
    assert_eq!(rejected.events[0].args["reason"], "artifact-rejected");

    let deposited = dispatch_next(
        &mut game,
        GameCommand::DepositAtHome {
            facility_id: ANAMBAR_MUSEUM_ID.to_owned(),
            item_id: "test.museum.dagger".to_owned(),
            quantity: 1,
        },
    );
    assert_eq!(deposited.events[0].kind, "home.deposit");
    assert_eq!(
        game.virtue_current(rfb_protocol::VirtueKindDto::Sacrifice),
        1
    );
    let stored = game
        .snapshot()
        .homes
        .into_iter()
        .find(|home| home.id == ANAMBAR_MUSEUM_ID)
        .expect("Anambar Museum should remain projected")
        .stored_items
        .into_iter()
        .find(|item| item.kind_id == "demo.item.dagger")
        .expect("donated dagger should be displayed");
    assert!(stored.inscription.is_none());
    assert!(stored.details.as_ref().unwrap().inscription.is_none());
    game.player.position = game
        .town_local_to_active_position("demo.town.anambar", Position { x: 60, y: 49 })
        .unwrap();
    assert!(game.town_facility_accessible(ANAMBAR_MUSEUM_ID));
    let second_door = game
        .snapshot()
        .homes
        .into_iter()
        .find(|home| home.id == ANAMBAR_MUSEUM_ID)
        .unwrap();
    assert_eq!(second_door.stored_items, vec![stored.clone()]);
    game = Game::from_save(game.to_save()).unwrap();
    let home_before = game.home_states["demo.town-facility.outpost-home"].clone();
    enter_town_facility(&mut game, THALOS_MUSEUM_ID);
    let thalos_collection = game
        .snapshot()
        .homes
        .into_iter()
        .find(|home| home.id == THALOS_MUSEUM_ID)
        .unwrap();
    assert_eq!(thalos_collection.stored_items, vec![stored.clone()]);
    enter_morivant(&mut game);
    game.player.position = game
        .town_local_to_wilderness_view_position(MORIVANT_TOWN_ID, Position { x: 98, y: 17 })
        .unwrap();
    let same_collection = game
        .snapshot()
        .homes
        .into_iter()
        .find(|home| home.id == "demo.town-facility.morivant-museum")
        .unwrap();
    assert_eq!(same_collection.stored_items, vec![stored.clone()]);
    let withdrawn = dispatch_next(
        &mut game,
        GameCommand::WithdrawFromHome {
            facility_id: "demo.town-facility.morivant-museum".to_owned(),
            item_id: stored.id,
            quantity: 1,
        },
    );
    assert_eq!(withdrawn.events[0].kind, "home.withdraw");
    assert!(game.home_states[THALOS_MUSEUM_ID].inventory.is_empty());
    assert_eq!(
        game.home_states["demo.town-facility.outpost-home"],
        home_before
    );
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn stored_blood_sours_and_only_museum_donations_clear_inscriptions() {
    for (facility, museum) in [
        (MORIVANT_HOME_ID, false),
        ("demo.town-facility.morivant-museum", true),
    ] {
        let mut game = town_facility_game(109, "demo.build.warrior", facility);
        game.mark_shop_visited_at_player().unwrap();
        game.reveal_current_visibility();
        support::give_inventory_item(&mut game, "test.blood", "demo.item.blood-potion");
        let blood = game
            .items
            .iter_mut()
            .find(|item| item.id == "test.blood")
            .unwrap();
        blood.quantity = 2;
        blood.inscription = Some("sample".to_owned());
        let outcome = game.deposit_at_home(facility, "test.blood", 1).unwrap();
        assert_eq!(outcome.item_kind_id, "demo.item.salt-water");
        let home = game
            .snapshot()
            .homes
            .into_iter()
            .find(|home| home.id == facility)
            .unwrap();
        assert_eq!(home.stored_items[0].kind_id, "demo.item.salt-water");
        assert_eq!(home.stored_items[0].inscription.is_none(), museum);
        let retained = game
            .items
            .iter()
            .find(|item| item.id == "test.blood")
            .unwrap();
        assert_eq!(retained.kind_id, "demo.item.blood-potion");
        assert_eq!(retained.quantity, 1);
        assert_eq!(retained.inscription.as_deref(), Some("sample"));
        assert_eq!(
            Game::from_save(game.to_save()).unwrap().state_hash(),
            game.state_hash()
        );
    }
}

#[test]
fn shared_museum_import_preserves_instances_and_knowledge_without_id_collisions() {
    let facility = "demo.town-facility.morivant-museum";
    let mut donor = town_facility_game(109, "demo.build.warrior", facility);
    donor.mark_shop_visited_at_player().unwrap();
    donor.reveal_current_visibility();
    support::give_inventory_item(&mut donor, "test.device", "demo.item.magic-missile-wand");
    support::give_inventory_item(&mut donor, "test.ball", "demo.item.capture-ball");
    support::give_inventory_item(&mut donor, "test.unknown", "demo.item.healing-potion");
    let device = donor
        .items
        .iter_mut()
        .find(|item| item.id == "test.device")
        .unwrap();
    device.charges.as_mut().unwrap().current -= 1;
    device.device_recovery_progress = 1;
    donor.identify_item_instance("test.device", ItemIdentificationRequest::new(true));
    donor
        .items
        .iter_mut()
        .find(|item| item.id == "test.ball")
        .unwrap()
        .captured_actor = Some(CapturedActor {
        kind_id: "demo.actor.horse".to_owned(),
        speed: 117,
        hp: 3,
        max_hp: 8,
        experience: 19,
    });
    support::give_inventory_item(&mut donor, "test.randart", "demo.item.dagger");
    let artifact = donor
        .items
        .iter_mut()
        .find(|item| item.id == "test.randart")
        .unwrap();
    artifact.artifact_name = Some("Shared artifact".to_owned());
    artifact.intrinsic_weight_tenths_pound = Some(9);
    donor.identify_item_instance("test.randart", ItemIdentificationRequest::new(true));
    for id in ["test.device", "test.ball", "test.unknown", "test.randart"] {
        donor.deposit_at_home(facility, id, 1).unwrap();
    }
    let museum = donor.shared_museum().unwrap();
    let mut recipient = town_facility_game(110, "demo.build.warrior", facility);
    recipient.mark_shop_visited_at_player().unwrap();
    recipient.reveal_current_visibility();
    support::give_inventory_item(&mut recipient, "test.device", "demo.item.dagger");
    let unchanged = recipient.state_hash();
    let imported = recipient.with_shared_museum(&museum).unwrap().unwrap();
    assert_eq!(recipient.state_hash(), unchanged);
    let result = imported.shared_museum().unwrap();
    for original in &museum.inventory {
        let incoming = result
            .inventory
            .iter()
            .find(|item| item.kind_id == original.kind_id)
            .unwrap();
        assert_ne!(incoming.id, original.id);
        let mut normalized = incoming.clone();
        normalized.id = original.id.clone();
        assert_eq!(&normalized, original);
        let before = museum
            .item_property_knowledge
            .iter()
            .find(|entry| entry.item_id == original.id);
        let after = result
            .item_property_knowledge
            .iter()
            .find(|entry| entry.item_id == incoming.id);
        assert_eq!(
            before.map(|entry| (entry.appraised, entry.identified, &entry.known_affix_ids)),
            after.map(|entry| (entry.appraised, entry.identified, &entry.known_affix_ids))
        );
    }
    let projection = imported.snapshot();
    let home = projection
        .homes
        .iter()
        .find(|home| home.id == facility)
        .unwrap();
    assert_eq!(
        home.stored_items
            .iter()
            .find(|item| item.kind_id == "demo.item.healing-potion")
            .unwrap()
            .details
            .as_ref()
            .unwrap()
            .knowledge,
        rfb_protocol::ItemKnowledgeDto::Unknown
    );
    assert_eq!(
        Game::from_save(imported.to_save()).unwrap().state_hash(),
        imported.state_hash()
    );
    let mut corrupt = museum.clone();
    corrupt.inventory.push(corrupt.inventory[0].clone());
    assert!(recipient.with_shared_museum(&corrupt).is_err());
    corrupt = museum;
    corrupt.inventory[0].kind_id = "demo.item.arkenstone-of-thrain".to_owned();
    assert!(recipient.with_shared_museum(&corrupt).is_err());
    assert_eq!(recipient.state_hash(), unchanged);
}

#[test]
fn shroomery_trade_maintenance_and_save_round_trip_use_existing_shop_state() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Outpost game should start");
    game.gold = 10_000;
    game.player.position = Position { x: 78, y: 26 };
    game.mark_shop_visited_at_player().unwrap();

    let shop = projected_shop(&game.snapshot().shops, SHROOMERY_ID).clone();
    assert!(shop.visited && shop.player_at_entrance);
    assert_eq!(shop.owner.name_key, "shop-owner-demo-outpost-martin-name");
    assert_eq!(
        shop.stock
            .iter()
            .map(|item| item.kind_id.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from([
            "demo.item.cure-blindness-mushroom",
            "demo.item.cure-confusion-mushroom",
            "demo.item.cure-paranoia-mushroom",
            "demo.item.cure-poison-mushroom",
            "demo.item.fast-recovery-mushroom",
        ])
    );
    let mushroom = shop
        .stock
        .iter()
        .find(|item| item.kind_id == "demo.item.fast-recovery-mushroom")
        .expect("Shroomery should stock Fast Recovery")
        .clone();
    dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: SHROOMERY_ID.to_owned(),
            item_id: mushroom.id,
            quantity: 1,
        },
    );
    assert!(game.items.iter().any(|item| {
        item.kind_id == "demo.item.fast-recovery-mushroom"
            && item.location == ItemLocation::Inventory
    }));

    game.shop_states
        .get_mut(SHROOMERY_ID)
        .expect("Shroomery state should exist")
        .inventory
        .clear();
    game.world_tick = 10_000;
    game.maintain_shop_at_player().unwrap();
    assert_eq!(
        game.shop_states[SHROOMERY_ID]
            .inventory
            .iter()
            .map(|item| item.kind_id.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        5
    );

    let restored = Game::from_save(game.to_save()).expect("Shroomery state should round-trip");
    assert_eq!(restored.shop_states, game.shop_states);
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn shroomery_refuses_formal_and_temporary_snotlings() {
    let prepare = |game: &mut Game| {
        game.gold = 10_000;
        game.player.position = Position { x: 78, y: 26 };
        game.mark_shop_visited_at_player().unwrap();
        game.shop_states[SHROOMERY_ID]
            .inventory
            .first()
            .expect("Shroomery stock")
            .id
            .clone()
    };

    let mut formal = Game::new_with_build_race_and_name(
        399,
        "demo.build.warrior",
        "rfb-legacy.race.snotling",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("formal Snotling should create");
    let item_id = prepare(&mut formal);
    assert_eq!(
        formal.buy_from_shop(SHROOMERY_ID, &item_id, 1),
        Err("race-refused"),
    );

    let mut temporary =
        Game::new_with_build(400, "demo.build.warrior").expect("Human Warrior should create");
    let item_id = prepare(&mut temporary);
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 100, "test.snotling-form").status;
    form.granted_race_id = Some("rfb-legacy.race.snotling".to_owned());
    temporary.player.statuses.push(form);
    assert_eq!(
        temporary.buy_from_shop(SHROOMERY_ID, &item_id, 1),
        Err("race-refused"),
    );
}

#[test]
fn inn_stays_use_content_prices_and_restore_the_player_at_half_day() {
    for (facility_id, starting_gold, cost) in [
        (ANAMBAR_INN_ID, 100, 25),
        (WHITE_HORSE_INN_ID, 20, 20),
        (MORIVANT_THIEVES_GUILD_ID, 50, 50),
        ("demo.town-facility.telmora-thieves-guild", 100, 100),
        ("demo.town-facility.angwil-thieves-guild", 50, 50),
    ] {
        let mut game = if facility_id == ANAMBAR_INN_ID {
            anambar_inn_game(42)
        } else if matches!(
            facility_id,
            MORIVANT_THIEVES_GUILD_ID
                | "demo.town-facility.telmora-thieves-guild"
                | "demo.town-facility.angwil-thieves-guild"
        ) {
            town_facility_game(42, "demo.build.warrior", facility_id)
        } else {
            white_horse_inn_game(42)
        };
        let cost = game.town_service_price(cost);
        let starting_gold = starting_gold + cost;
        game.world_tick = 12_345;
        game.gold = starting_gold;
        game.player.hp = 1;
        game.player
            .statuses
            .push(monster_combat::melee_status(STATUS_HASTE, 20, "test.inn-rest").status);
        game.minor_slow = 3;
        game.minor_slow_energy = 41;
        game.reality_change_ticks = 20;
        game.recall = Some(RecallStateDto {
            destination: Some(rfb_protocol::RecallDestinationDto {
                dungeon_id: "demo.dungeon.warrens".to_owned(),
                floor_id: "demo.floor.warrens-depth-1".to_owned(),
            }),
            remaining_turns: Some(2),
        });
        game.resources
            .values_mut()
            .for_each(|pool| pool.current = 0);
        support::give_inventory_item(
            &mut game,
            "test.inn.device",
            "demo.item.detect-objects-staff",
        );
        let device = game
            .items
            .iter_mut()
            .find(|item| item.id == "test.inn.device")
            .expect("test device should exist");
        device
            .charges
            .as_mut()
            .expect("test staff should have charges")
            .current = 0;
        device.device_recovery_progress = 500;
        let nutrition = game.nutrition;
        let draws = game.rng_draw_counter();

        let update = dispatch_next(
            &mut game,
            GameCommand::StayAtInn {
                facility_id: facility_id.to_owned(),
            },
        );

        let event = update
            .events
            .iter()
            .find(|event| event.kind == "inn.stay")
            .expect("successful inn stay should be explicit");
        assert_eq!(event.args["cost"], cost.to_string(), "{facility_id}");
        assert_eq!(
            event.args["balance"],
            (starting_gold - cost).to_string(),
            "{facility_id}"
        );
        assert_eq!(event.args["elapsedTicks"], "37655");
        assert_eq!(game.world_tick, 50_000);
        assert_eq!(game.gold, starting_gold - cost, "{facility_id}");
        assert_eq!(game.player.hp, game.effective_player_max_hp());
        assert!(game.player.statuses.is_empty());
        assert_eq!((game.minor_slow, game.minor_slow_energy), (0, 0));
        assert_eq!(game.reality_change_ticks, 0);
        assert!(!game.recall_is_active());
        assert!(
            game.resources
                .values()
                .all(|pool| pool.current == pool.maximum)
        );
        let device = game
            .items
            .iter()
            .find(|item| item.id == "test.inn.device")
            .expect("test device should remain carried");
        let charges = device.charges.expect("test staff should retain charges");
        assert_eq!(charges.current, charges.maximum);
        assert_eq!(device.device_recovery_progress, 0);
        assert_eq!(game.nutrition, nutrition);
        assert_eq!(game.rng_draw_counter(), draws);

        let restored = Game::from_save(game.to_save()).expect("inn result should round-trip");
        assert_eq!(restored.state_hash(), game.state_hash());
    }
}

#[test]
fn inn_and_guild_rejections_do_not_charge_or_advance_time() {
    for (facility_id, base, cost) in [
        (ANAMBAR_INN_ID, anambar_inn_game(42), 25),
        (
            MORIVANT_THIEVES_GUILD_ID,
            town_facility_game(42, "demo.build.warrior", MORIVANT_THIEVES_GUILD_ID),
            50,
        ),
    ] {
        let cost = base.town_service_price(cost);
        for status_kind_id in [STATUS_POISON, STATUS_BLEEDING] {
            let mut game = base.clone();
            game.gold = 100;
            game.world_tick = 12_345;
            game.player
                .statuses
                .push(monster_combat::melee_status(status_kind_id, 20, "test.inn-rest").status);
            let draws = game.rng_draw_counter();

            let update = dispatch_next(
                &mut game,
                GameCommand::StayAtInn {
                    facility_id: facility_id.to_owned(),
                },
            );

            let event = update
                .events
                .iter()
                .find(|event| event.kind == "inn.stay-unavailable")
                .expect("unsafe inn stay should be rejected");
            assert_eq!(event.args["reason"], "needs-healer");
            assert_eq!(game.gold, 100);
            assert_eq!(game.world_tick, 12_345);
            assert_eq!(game.rng_draw_counter(), draws);
            assert!(game.player_has_status_kind(status_kind_id));
        }

        let mut poor = base.clone();
        poor.gold = cost - 1;
        let tick = poor.world_tick;
        let update = dispatch_next(
            &mut poor,
            GameCommand::StayAtInn {
                facility_id: facility_id.to_owned(),
            },
        );
        assert_eq!(update.events[0].args["reason"], "insufficient-gold");
        assert_eq!(poor.gold, cost - 1);
        assert_eq!(poor.world_tick, tick);
        let mut outside = base;
        outside.player.position.x -= 1;
        let before = outside.state_hash();
        assert_eq!(
            outside.stay_at_inn(facility_id).unwrap_err(),
            "inn-unreachable"
        );
        assert_eq!(outside.state_hash(), before);
    }
}

#[test]
fn inn_travel_requires_a_visited_town_and_arrives_at_its_inn() {
    let mut unvisited = white_horse_inn_game(42);
    unvisited.gold = 500;
    let rejected = dispatch_next(
        &mut unvisited,
        GameCommand::TravelFromInn {
            facility_id: WHITE_HORSE_INN_ID.to_owned(),
            destination_town_id: "demo.town.anambar".to_owned(),
        },
    );
    assert_eq!(rejected.events[0].kind, "inn.travel-unavailable");
    assert_eq!(rejected.events[0].args["reason"], "town-unvisited");
    assert_eq!(unvisited.gold, 500);
    assert_eq!(
        unvisited.wilderness_position,
        Some(Position { x: 28, y: 52 })
    );

    let mut game = anambar_inn_game(42);
    game.gold = 2 * game.town_service_price(500);
    let to_outpost = dispatch_next(
        &mut game,
        GameCommand::TravelFromInn {
            facility_id: ANAMBAR_INN_ID.to_owned(),
            destination_town_id: "demo.town.outpost".to_owned(),
        },
    );
    assert_eq!(to_outpost.events[0].kind, "inn.travel");
    assert_eq!(to_outpost.events[0].args["cost"], "700");
    assert_eq!(game.gold, 700);
    assert_eq!(game.wilderness_position, Some(Position { x: 28, y: 52 }));
    assert_eq!(game.player.position, Position { x: 124, y: 35 });
    let white_horse = projected_shop(&game.snapshot().shops, WHITE_HORSE_INN_ID).clone();
    assert!(white_horse.player_at_entrance);
    assert_eq!(
        white_horse.inn_travel_destinations[0].town_id,
        "demo.town.anambar"
    );

    let to_anambar = dispatch_next(
        &mut game,
        GameCommand::TravelFromInn {
            facility_id: WHITE_HORSE_INN_ID.to_owned(),
            destination_town_id: "demo.town.anambar".to_owned(),
        },
    );
    assert_eq!(to_anambar.events[0].kind, "inn.travel");
    assert_eq!(game.gold, 0);
    assert_eq!(game.wilderness_position, Some(Position { x: 26, y: 39 }));
    assert_eq!(
        game.player.position,
        game.shop_entrance_position(game.content.shop(ANAMBAR_INN_ID).unwrap())
            .unwrap()
    );
    assert!(projected_shop(&game.snapshot().shops, ANAMBAR_INN_ID).player_at_entrance);
}

#[test]
fn outpost_count_identifies_carried_items_for_the_projected_price() {
    let mut game = outpost_count_game(42);
    game.gold = 100;
    let item_id = game
        .items
        .iter()
        .find(|item| item.location == ItemLocation::Inventory)
        .expect("warrior should start with a carried item")
        .id
        .clone();
    game.item_property_knowledge.remove(&item_id);

    let before_tick = game.world_tick;
    let update = dispatch_next(
        &mut game,
        GameCommand::IdentifyAtFacility {
            facility_id: OUTPOST_COUNT_ID.to_owned(),
            item_id: item_id.clone(),
        },
    );
    assert_eq!(game.gold, 30);
    assert_eq!(game.world_tick, before_tick);
    assert!(game.item_property_knowledge[&item_id].appraised);
    assert!(update.events.iter().any(|event| {
        event.kind == "facility.identified"
            && event.args.get("cost").is_some_and(|cost| cost == "70")
    }));

    let rejected = dispatch_next(
        &mut game,
        GameCommand::IdentifyAtFacility {
            facility_id: OUTPOST_COUNT_ID.to_owned(),
            item_id,
        },
    );
    assert_eq!(game.gold, 30);
    assert_eq!(rejected.events[0].kind, "facility.identify-unavailable");
}

#[test]
fn outpost_count_legal_name_change_is_validated_saved_and_projected() {
    let mut game = outpost_count_game(42);
    game.gold = 20;
    let service = game
        .snapshot()
        .task_services
        .into_iter()
        .find(|service| service.id == OUTPOST_COUNT_ID)
        .expect("Count service should be projected at its entrance");
    assert_eq!(service.identify_item_cost, Some(70));
    assert_eq!(service.legal_name_change_cost, Some(14));

    let update = dispatch_next(
        &mut game,
        GameCommand::RenameAtFacility {
            facility_id: OUTPOST_COUNT_ID.to_owned(),
            name: "  Elessar  ".to_owned(),
        },
    );
    assert_eq!(game.gold, 6);
    assert_eq!(game.snapshot().player.name, "Elessar");
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "facility.renamed")
    );

    let restored = Game::from_save(game.to_save()).expect("renamed player should round-trip");
    assert_eq!(restored.snapshot().player.name, "Elessar");
    assert_eq!(restored.gold, 6);

    let mut invalid = outpost_count_game(43);
    invalid.gold = 20;
    let rejected = dispatch_next(
        &mut invalid,
        GameCommand::RenameAtFacility {
            facility_id: OUTPOST_COUNT_ID.to_owned(),
            name: "\n".to_owned(),
        },
    );
    assert_eq!(invalid.gold, 20);
    assert_eq!(rejected.events[0].kind, "facility.rename-unavailable");
}

#[test]
fn p104c_anambar_library_identifies_researches_and_identifies_all_without_time_or_rng() {
    let mut single = anambar_library_game(104);
    let service = single
        .snapshot()
        .task_services
        .into_iter()
        .find(|service| service.id == ANAMBAR_LIBRARY_ID)
        .expect("library service should be projected");
    assert_eq!(service.identify_item_cost, Some(70));
    assert_eq!(service.research_item_cost, Some(1_820));
    assert_eq!(service.identify_all_items_cost, Some(490));
    assert_eq!(
        service.overview_message_key.as_deref(),
        Some("town-facility-demo-anambar-library-overview")
    );
    assert!(service.tasks.is_empty());
    let item_id = single
        .items
        .iter()
        .find(|item| item.location == ItemLocation::Inventory)
        .expect("warrior should carry an item")
        .id
        .clone();
    single.item_property_knowledge.remove(&item_id);
    single.gold = service.identify_item_cost.unwrap();
    let tick = single.world_tick;
    let draws = single.rng_draw_counter();
    let identified = dispatch_next(
        &mut single,
        GameCommand::IdentifyAtFacility {
            facility_id: ANAMBAR_LIBRARY_ID.to_owned(),
            item_id: item_id.clone(),
        },
    );
    assert_eq!(single.gold, 0);
    assert!(single.item_property_knowledge[&item_id].appraised);
    assert!(!single.item_property_knowledge[&item_id].identified);
    assert_eq!(single.world_tick, tick);
    assert_eq!(single.rng_draw_counter(), draws);
    assert_eq!(
        identified.events[0].message_key,
        "facility-identify-completed"
    );

    let mut research = anambar_library_game(105);
    let item_id = research
        .items
        .iter()
        .find(|item| item.location == ItemLocation::Inventory)
        .expect("warrior should carry an item")
        .id
        .clone();
    let knowledge = research
        .item_property_knowledge
        .entry(item_id.clone())
        .or_default();
    knowledge.appraised = true;
    knowledge.identified = false;
    knowledge.known_affix_ids.clear();
    research.gold = service.research_item_cost.unwrap();
    let tick = research.world_tick;
    let draws = research.rng_draw_counter();
    let researched = dispatch_next(
        &mut research,
        GameCommand::ResearchItemAtFacility {
            facility_id: ANAMBAR_LIBRARY_ID.to_owned(),
            item_id: item_id.clone(),
        },
    );
    assert_eq!(research.gold, 0);
    assert!(research.item_property_knowledge[&item_id].identified);
    assert_eq!(research.world_tick, tick);
    assert_eq!(research.rng_draw_counter(), draws);
    assert_eq!(
        researched.events[0].message_key,
        "facility-research-completed"
    );

    let mut all = anambar_library_game(106);
    let carried_ids = all
        .items
        .iter()
        .filter(|item| {
            matches!(
                item.location,
                ItemLocation::Inventory | ItemLocation::Equipped { .. }
            )
        })
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    for item_id in &carried_ids {
        all.item_property_knowledge.remove(item_id);
    }
    all.gold = service.identify_all_items_cost.unwrap();
    let tick = all.world_tick;
    let draws = all.rng_draw_counter();
    let identified_all = dispatch_next(
        &mut all,
        GameCommand::IdentifyAllAtFacility {
            facility_id: ANAMBAR_LIBRARY_ID.to_owned(),
        },
    );
    assert_eq!(all.gold, 0);
    assert!(carried_ids.iter().all(|item_id| {
        all.item_property_knowledge
            .get(item_id)
            .is_some_and(|knowledge| knowledge.appraised)
    }));
    assert_eq!(all.world_tick, tick);
    assert_eq!(all.rng_draw_counter(), draws);
    assert_eq!(identified_all.events[0].kind, "facility.identified-all");
    assert_eq!(
        identified_all.events[0].args["count"],
        carried_ids.len().to_string()
    );

    all.gold = service.identify_all_items_cost.unwrap();
    let rejected = dispatch_next(
        &mut all,
        GameCommand::IdentifyAllAtFacility {
            facility_id: ANAMBAR_LIBRARY_ID.to_owned(),
        },
    );
    assert_eq!(all.gold, 490);
    assert_eq!(rejected.events[0].kind, "facility.identify-all-unavailable");
    assert_eq!(rejected.events[0].args["reason"], "nothing-to-identify");
}

#[test]
fn p105c_anambar_facilities_apply_roles_prices_recovery_enchantment_assessment_and_recall() {
    let mut warrior =
        anambar_facility_game(105, "demo.build.warrior", None, ANAMBAR_WARRIOR_GUILD_ID);
    let guild = warrior
        .snapshot()
        .task_services
        .into_iter()
        .find(|service| service.id == ANAMBAR_WARRIOR_GUILD_ID)
        .expect("warrior guild should be projected");
    assert_eq!(guild.membership, FacilityMembershipDto::Owner);
    let weapon_service = guild
        .service_actions
        .iter()
        .find(|service| service.kind == FacilityServiceKindDto::EnchantWeapon)
        .expect("warrior guild should enchant weapons");
    assert_eq!(weapon_service.cost, 0);
    let weapon_id = weapon_service
        .targets
        .first()
        .expect("warrior should carry an enchantable weapon")
        .item_id
        .clone();
    warrior.gold = weapon_service.targets[0].choices[0].cost;
    let tick = warrior.world_tick;
    let draws = warrior.rng_draw_counter();
    let enchanted = dispatch_next(
        &mut warrior,
        GameCommand::UseFacilityService {
            facility_id: ANAMBAR_WARRIOR_GUILD_ID.to_owned(),
            service: FacilityServiceKindDto::EnchantWeapon,
            item_id: Some(weapon_id),
            enchantment_steps: Some(1),
        },
    );
    assert_eq!(warrior.gold, 0);
    assert_eq!(warrior.world_tick, tick);
    assert_eq!(warrior.rng_draw_counter(), draws);
    assert!(
        enchanted
            .events
            .iter()
            .any(|event| event.kind == "facility.item-enchanted")
    );

    let mut archer = anambar_facility_game(106, "demo.build.archer", None, ANAMBAR_ARCHER_GUILD_ID);
    let archer_guild = archer
        .snapshot()
        .task_services
        .into_iter()
        .find(|service| service.id == ANAMBAR_ARCHER_GUILD_ID)
        .expect("archer guild should be projected");
    assert_eq!(archer_guild.membership, FacilityMembershipDto::Owner);
    let ammunition = archer_guild
        .service_actions
        .iter()
        .find(|service| service.kind == FacilityServiceKindDto::EnchantAmmunition)
        .and_then(|service| service.targets.first())
        .expect("archer should carry enchantable ammunition");
    let quantity = archer
        .items
        .iter()
        .find(|item| item.id == ammunition.item_id)
        .expect("projected ammunition should exist")
        .quantity;
    assert_eq!(
        ammunition.choices[0].cost,
        archer.town_service_price(22) * quantity
    );
    let choice = &ammunition.choices[2];
    assert_eq!(choice.steps, 3);
    assert_eq!(choice.cost, 3 * archer.town_service_price(22) * quantity);
    archer.gold = choice.cost;
    let rng = archer.rng.clone();
    archer
        .use_town_facility_service(
            ANAMBAR_ARCHER_GUILD_ID,
            FacilityServiceKindDto::EnchantAmmunition,
            Some(&ammunition.item_id),
            Some(3),
            &mut Vec::new(),
        )
        .unwrap();
    assert_eq!(archer.gold, 0);
    assert_eq!(archer.rng, rng);

    let mut healing = anambar_facility_game(
        107,
        "demo.build.paladin-death",
        None,
        ANAMBAR_MAMMON_TEMPLE_ID,
    );
    let temple = healing
        .snapshot()
        .task_services
        .into_iter()
        .find(|service| service.id == ANAMBAR_MAMMON_TEMPLE_ID)
        .expect("Mammon temple should be projected");
    assert_eq!(temple.membership, FacilityMembershipDto::Member);
    assert_eq!(
        temple
            .service_actions
            .iter()
            .find(|service| service.kind == FacilityServiceKindDto::Heal)
            .map(|service| service.cost),
        Some(healing.town_service_price(500))
    );
    healing.player.hp = 1;
    healing
        .player
        .statuses
        .push(monster_combat::melee_status(STATUS_POISON, 20, "test.p105").status);
    healing.gold = healing.town_service_price(500);
    let tick = healing.world_tick;
    let draws = healing.rng_draw_counter();
    let healed = dispatch_next(
        &mut healing,
        GameCommand::UseFacilityService {
            facility_id: ANAMBAR_MAMMON_TEMPLE_ID.to_owned(),
            service: FacilityServiceKindDto::Heal,
            item_id: None,
            enchantment_steps: None,
        },
    );
    assert_eq!(
        healing.player.hp,
        201.min(healing.effective_player_max_hp())
    );
    assert!(!healing.player_has_status_kind(STATUS_POISON));
    assert_eq!(healing.gold, 0);
    assert_eq!(healing.world_tick, tick);
    assert_eq!(healing.rng_draw_counter(), draws);
    assert!(
        healed
            .events
            .iter()
            .any(|event| event.kind == "facility.healed")
    );

    let mut restored =
        anambar_facility_game(108, "demo.build.warrior", None, ANAMBAR_MAMMON_TEMPLE_ID);
    restored.progress.attributes.strength = restored
        .progress
        .maximum_attributes
        .strength
        .saturating_sub(1);
    restored.progress.maximum_experience = restored.progress.experience.saturating_add(10);
    restored.progress.life_force = 900;
    restored.gold = restored.town_service_price(2_500);
    let update = dispatch_next(
        &mut restored,
        GameCommand::UseFacilityService {
            facility_id: ANAMBAR_MAMMON_TEMPLE_ID.to_owned(),
            service: FacilityServiceKindDto::RestoreVitality,
            item_id: None,
            enchantment_steps: None,
        },
    );
    assert_eq!(
        restored.progress.attributes,
        restored.progress.maximum_attributes
    );
    assert_eq!(
        restored.progress.experience,
        restored.progress.maximum_experience
    );
    assert_eq!(restored.progress.life_force, 1_000);
    assert_eq!(restored.gold, 0);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "facility.vitality-restored")
    );

    let mut mutated =
        anambar_facility_game(109, "demo.build.warrior", None, ANAMBAR_MAMMON_TEMPLE_ID);
    mutated.progress.active_mutation_ids.clear();
    mutated.progress.locked_mutation_ids.clear();
    mutated
        .progress
        .active_mutation_ids
        .insert("rfb.mutation.alcohol".to_owned());
    mutated.gold = mutated.town_service_price(100_000);
    let update = dispatch_next(
        &mut mutated,
        GameCommand::UseFacilityService {
            facility_id: ANAMBAR_MAMMON_TEMPLE_ID.to_owned(),
            service: FacilityServiceKindDto::CureMutation,
            item_id: None,
            enchantment_steps: None,
        },
    );
    assert!(mutated.progress.active_mutation_ids.is_empty());
    assert_eq!(mutated.gold, 0);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "facility.mutation-cured")
    );

    let mut assessed =
        anambar_facility_game(110, "demo.build.warrior", None, ANAMBAR_WEAPON_MASTER_ID);
    assessed.gold = assessed.town_service_price(400);
    let update = dispatch_next(
        &mut assessed,
        GameCommand::UseFacilityService {
            facility_id: ANAMBAR_WEAPON_MASTER_ID.to_owned(),
            service: FacilityServiceKindDto::AssessArmor,
            item_id: None,
            enchantment_steps: None,
        },
    );
    assert_eq!(assessed.gold, 0);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "facility.armor-assessed")
    );

    let mut recall = anambar_facility_game(111, "demo.build.warrior", None, ANAMBAR_TRUMP_TOWER_ID);
    let mut amberite_form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 20, "test.p105").status;
    amberite_form.granted_race_id = Some("rfb-legacy.race.amberite".to_owned());
    recall.player.statuses.push(amberite_form);
    let tower = recall
        .snapshot()
        .task_services
        .into_iter()
        .find(|service| service.id == ANAMBAR_TRUMP_TOWER_ID)
        .expect("Trump tower should be projected");
    assert_eq!(tower.membership, FacilityMembershipDto::Member);
    assert_eq!(
        tower.service_actions[0].cost,
        recall.town_service_price(150)
    );
    recall.recall = Some(RecallStateDto {
        destination: Some(rfb_protocol::RecallDestinationDto {
            dungeon_id: "demo.dungeon.warrens".to_owned(),
            floor_id: "demo.floor.warrens-depth-1".to_owned(),
        }),
        remaining_turns: None,
    });
    recall.gold = tower.service_actions[0].cost;
    let update = dispatch_next(
        &mut recall,
        GameCommand::UseFacilityService {
            facility_id: ANAMBAR_TRUMP_TOWER_ID.to_owned(),
            service: FacilityServiceKindDto::Recall,
            item_id: None,
            enchantment_steps: None,
        },
    );
    assert_eq!(recall.gold, 0, "{:?}", update.events);
    assert_eq!(
        recall
            .recall
            .as_ref()
            .and_then(|recall| recall.remaining_turns),
        Some(2)
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "facility.recall-started")
    );
}

#[test]
fn outpost_temple_entrance_opens_onto_the_source_street() {
    let mut game = Game::new_with_build(42, "demo.build.warrior").unwrap();
    game.player.position = Position { x: 70, y: 30 };
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::North,
        },
    );
    assert_eq!(game.player.position, Position { x: 70, y: 29 });
    assert!(projected_shop(&game.snapshot().shops, TEMPLE_ID).player_at_entrance);
}

#[test]
fn home_deposit_withdraw_grouping_and_save_are_authoritative() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    game.player.position = Position { x: 110, y: 44 };
    game.mark_shop_visited_at_player().unwrap();
    let home = game.snapshot().homes[0].clone();
    assert!(home.visited);
    assert!(home.player_at_entrance);
    assert!(home.stored_items.is_empty());
    let ration = home
        .deposit_items
        .iter()
        .find(|item| item.kind_id == "demo.item.ration-of-food")
        .expect("Warrior should carry rations")
        .clone();
    let initial_ration_quantity = ration.quantity;
    game.item_property_knowledge.insert(
        ration.id.clone(),
        ItemPropertyKnowledgeState {
            known_blessed: false,
            discovered: true,
            appraised: true,
            identified: true,
            feeling: None,
            known_affix_ids: BTreeSet::new(),
        },
    );
    let gold_before = game.gold;
    let tick_before = game.world_tick;
    let draws_before = game.rng_draw_counter();
    let deposit = dispatch_next(
        &mut game,
        GameCommand::DepositAtHome {
            facility_id: HOME_ID.to_owned(),
            item_id: ration.id,
            quantity: 1,
        },
    );
    assert!(
        deposit
            .events
            .iter()
            .any(|event| event.kind == "home.deposit")
    );
    assert_eq!(game.gold, gold_before);
    assert_eq!(game.world_tick, tick_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
    let stored = game.snapshot().homes[0]
        .stored_items
        .iter()
        .find(|item| item.kind_id == "demo.item.ration-of-food")
        .expect("deposited ration should be stored")
        .clone();
    assert_eq!(stored.quantity, 1);

    let restored = Game::from_save(game.to_save()).expect("home inventory should round-trip");
    assert_eq!(restored.home_states, game.home_states);
    let mut game = restored;
    let withdrawal = dispatch_next(
        &mut game,
        GameCommand::WithdrawFromHome {
            facility_id: HOME_ID.to_owned(),
            item_id: stored.id,
            quantity: 1,
        },
    );
    assert!(
        withdrawal
            .events
            .iter()
            .any(|event| event.kind == "home.withdraw")
    );
    assert!(game.snapshot().homes[0].stored_items.is_empty());
    let carried_rations = game
        .items
        .iter()
        .filter(|item| {
            item.kind_id == "demo.item.ration-of-food" && item.location == ItemLocation::Inventory
        })
        .collect::<Vec<_>>();
    assert_eq!(carried_rations.len(), 1);
    assert_eq!(carried_rations[0].quantity, initial_ration_quantity);
    assert_eq!(game.gold, gold_before);
    assert_eq!(game.world_tick, tick_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
}

#[test]
fn anambar_home_uses_the_outpost_home_inventory() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    game.player.position = Position { x: 110, y: 44 };
    game.mark_shop_visited_at_player().unwrap();
    let ration = game.snapshot().homes[0]
        .deposit_items
        .iter()
        .find(|item| item.kind_id == "demo.item.ration-of-food")
        .expect("Warrior should carry rations")
        .clone();
    dispatch_next(
        &mut game,
        GameCommand::DepositAtHome {
            facility_id: HOME_ID.to_owned(),
            item_id: ration.id,
            quantity: 1,
        },
    );

    dispatch_next(
        &mut game,
        GameCommand::EnterWorldMap {
            leave_pets: false,
            cancel_recall: false,
        },
    );
    game.wilderness_position = Some(Position { x: 26, y: 39 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(game.home_states.len(), 2);
    assert!(game.home_states.contains_key(HOME_ID));
    let town_snapshot = game.snapshot();
    assert_eq!(town_snapshot.shops.len(), 10);
    assert!(
        town_snapshot
            .shops
            .iter()
            .all(|shop| !shop.visited && shop.stock.is_empty())
    );
    assert!(
        !game
            .shop_states
            .keys()
            .any(|shop_id| shop_id.starts_with("demo.shop.anambar-"))
    );

    game.player.position = game
        .shop_entrance_position(game.content.shop(ANAMBAR_INN_ID).unwrap())
        .unwrap();
    game.mark_shop_visited_at_player().unwrap();
    let inn = projected_shop(&game.snapshot().shops, ANAMBAR_INN_ID).clone();
    assert!(inn.visited && inn.player_at_entrance && !inn.stock.is_empty());
    assert!(inn.stock.iter().all(|item| {
        [
            "demo.item.ration-of-food",
            "demo.item.water-potion",
            "demo.item.apple-juice",
            "demo.item.pint-of-fine-ale",
            "demo.item.pint-of-fine-wine",
        ]
        .contains(&item.kind_id.as_str())
    }));

    let home_position = game
        .town_facility_entrance_position(game.content.town_facility(ANAMBAR_HOME_ID).unwrap())
        .unwrap();
    game.player.position = home_position;
    assert!(!game.town_facility_accessible(ANAMBAR_HOME_ID));
    assert!(
        game.deposit_at_home(ANAMBAR_HOME_ID, "closed-home", 1)
            .is_err()
    );
    let police_id = "demo.town-facility.anambar-police-station";
    let task_id = "demo.task.anambar-cop-quest";
    game.player.position = game
        .town_facility_entrance_position(game.content.town_facility(police_id).unwrap())
        .unwrap();
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: police_id.into(),
            task_id: task_id.into(),
        },
    );
    game.player.position = home_position;
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, "demo.floor.anambar-cop-quest");
    support::clear_monsters(&mut game); // Explicit completion setup; this is a town access test.
    dispatch_next(&mut game, GameCommand::Wait);
    support::place_player_on_terrain(&mut game, "demo.terrain.stairs-up");
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.player.position, home_position);
    assert!(!game.town_facility_accessible(ANAMBAR_HOME_ID));
    game = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    game.player.position = game
        .town_facility_entrance_position(game.content.town_facility(police_id).unwrap())
        .unwrap();
    dispatch_next(
        &mut game,
        GameCommand::ClaimTaskReward {
            facility_id: police_id.into(),
            task_id: task_id.into(),
        },
    );
    game.player.position = home_position;
    game.mark_shop_visited_at_player().unwrap();
    let home = game
        .snapshot()
        .homes
        .into_iter()
        .find(|home| home.id == ANAMBAR_HOME_ID)
        .expect("Anambar Home should be projected");
    let stored = home
        .stored_items
        .iter()
        .find(|item| item.kind_id == "demo.item.ration-of-food")
        .expect("Outpost deposit should be visible in Anambar");
    dispatch_next(
        &mut game,
        GameCommand::WithdrawFromHome {
            facility_id: ANAMBAR_HOME_ID.to_owned(),
            item_id: stored.id.clone(),
            quantity: 1,
        },
    );

    assert!(game.home_states[HOME_ID].inventory.is_empty());
    let restored = Game::from_save(game.to_save()).expect("shared Home should round-trip");
    assert_eq!(restored.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(restored.home_states, game.home_states);
}

#[test]
fn overburdened_player_can_withdraw_from_home() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    game.player.position = Position { x: 110, y: 44 };
    game.mark_shop_visited_at_player().unwrap();
    support::give_inventory_item(&mut game, "test.heavy-stack", "demo.item.burdened-mail");
    game.items
        .iter_mut()
        .find(|item| item.id == "test.heavy-stack")
        .expect("fixture item should exist")
        .quantity = 11;
    let mut stored = game
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.ration-of-food")
        .expect("Warrior should carry rations")
        .clone();
    stored.id = "test.home-ration".to_owned();
    stored.quantity = 1;
    stored.location = ItemLocation::Home {
        facility_id: HOME_ID.to_owned(),
    };
    let item_id = stored.id.clone();
    game.home_states
        .get_mut(HOME_ID)
        .expect("Home should have authoritative state")
        .inventory
        .push(stored);

    let update = dispatch_next(
        &mut game,
        GameCommand::WithdrawFromHome {
            facility_id: HOME_ID.to_owned(),
            item_id,
            quantity: 1,
        },
    );

    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "home.withdraw")
    );
    assert!(update.player.carried_weight_tenths_pound > update.player.carry_capacity_tenths_pound);
}

#[test]
fn home_inventory_ids_are_reserved_by_the_global_allocator() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    let reserved_id = format!("{GENERATED_ITEM_ID_PREFIX}9000");
    let mut stored = game
        .items
        .iter()
        .find(|item| {
            item.kind_id == "demo.item.ration-of-food" && item.location == ItemLocation::Inventory
        })
        .expect("Warrior should carry rations")
        .clone();
    stored.id = reserved_id.clone();
    stored.quantity = 1;
    stored.location = ItemLocation::Home {
        facility_id: HOME_ID.to_owned(),
    };
    game.home_states
        .get_mut(HOME_ID)
        .expect("Home should have authoritative state")
        .inventory
        .push(stored);
    game.next_item_instance_serial = 9000;

    assert_eq!(
        game.allocate_item_instance_id()
            .expect("allocator should skip Home inventory IDs"),
        format!("{GENERATED_ITEM_ID_PREFIX}9001")
    );
}

#[test]
fn entering_general_store_entrance_marks_persistent_shop_visit() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    game.player.position = Position { x: 71, y: 39 };

    let update = dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::West,
        },
    );
    let general_store = projected_shop(&update.shops, GENERAL_STORE_ID);
    assert!(general_store.visited);
    assert!(general_store.player_at_entrance);
    assert!(
        update
            .shops
            .iter()
            .filter(|shop| shop.player_at_entrance)
            .all(|shop| shop.id == GENERAL_STORE_ID)
    );

    let restored = Game::from_save(game.to_save()).expect("shop visit should round-trip");
    assert!(projected_shop(&restored.snapshot().shops, GENERAL_STORE_ID).visited);
}

#[test]
fn malformed_town_state_is_rejected() {
    let game = Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    let mut payload = game.to_save();
    payload.town_states[0].visited = false;
    assert!(matches!(
        Game::from_save(payload),
        Err(CoreError::InvalidSave("town state is invalid"))
    ));
}

#[test]
fn runtime_town_validation_requires_complete_home_state() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    game.home_states.clear();
    assert!(matches!(
        game.validate_loaded_state(),
        Err(CoreError::InvalidSave("home state is invalid"))
    ));
}

#[test]
fn missing_unentered_shop_state_is_created_on_first_entry() {
    let game = Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    let mut payload = game.to_save();
    payload.shop_states.clear();
    let mut restored = Game::from_save(payload).expect("unentered shop state may remain sparse");
    assert!(restored.shop_states.is_empty());

    restored.player.position = Position { x: 71, y: 39 };
    let update = dispatch_next(
        &mut restored,
        GameCommand::Move {
            direction: Direction::West,
        },
    );
    assert!(restored.shop_states.contains_key(GENERAL_STORE_ID));
    assert!(projected_shop(&update.shops, GENERAL_STORE_ID).visited);
}

#[test]
fn initial_shop_stock_is_seeded_independent_and_persistent() {
    let left = Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    let right = Game::new_with_build(42, "demo.build.warrior").expect("same seed should start");
    assert_eq!(left.shop_states, right.shop_states);
    let town = left
        .content
        .town("demo.town.outpost")
        .expect("Outpost should exist");
    for shop_id in &town.shop_ids {
        let expected_kinds = left
            .content
            .shop(shop_id)
            .expect("town shop should exist")
            .stock
            .iter()
            .map(|stock| stock.item_kind_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let actual_kinds = left.shop_states[shop_id.as_str()]
            .inventory
            .iter()
            .map(|item| item.kind_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        assert!(actual_kinds.is_subset(&expected_kinds));
        for guaranteed in left
            .content
            .shop(shop_id)
            .expect("town shop should exist")
            .stock
            .iter()
            .filter(|stock| stock.availability_percent == 100)
        {
            assert!(actual_kinds.contains(guaranteed.item_kind_id.as_str()));
        }
    }
    let restored = Game::from_save(left.to_save()).expect("store stock should round-trip");
    assert_eq!(restored.shop_states, left.shop_states);
    assert_eq!(restored.state_hash(), left.state_hash());
}

#[test]
fn current_warrior_uses_rfb_price_factor_and_trade_values() {
    let game = store_game(42);
    let snapshot = game.snapshot();
    let shop = projected_shop(&snapshot.shops, GENERAL_STORE_ID);
    assert_eq!(shop.owner.price_factor_percent, 135);
    let buy_prices = shop
        .stock
        .iter()
        .map(|item| (item.kind_id.as_str(), item.unit_price))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(buy_prices["demo.item.ration-of-food"], 4);
    assert_eq!(buy_prices["demo.item.wooden-torch"], 1);
    assert_eq!(buy_prices["demo.item.brass-lantern"], 41);
    assert_eq!(buy_prices["demo.item.flask-of-oil"], 4);
    assert_eq!(super::super::town::sell_unit_price(3, 100, 500), 2);
    assert_eq!(super::super::town::sell_unit_price(1, 100, 500), 1);
    assert_eq!(super::super::town::sell_unit_price(30, 100, 500), 28);
    assert_eq!(super::super::town::sell_unit_price(1_000, 100, 7), 7);
}

#[test]
fn player_made_ammunition_keeps_its_ninety_nine_percent_shop_discount() {
    let mut game = store_game(44);
    let ammunition = game
        .items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.arrow")
        .expect("Warrior should carry arrows");
    ammunition.kind_id = "demo.item.mithril-arrow".to_owned();
    ammunition.quantity = 1;
    ammunition.origin_kind = Some(ItemOriginKindDto::PlayerMade);
    ammunition.discount_percent = 99;
    let item_id = ammunition.id.clone();

    let before_sale = game.snapshot();
    let quote = projected_shop(&before_sale.shops, GENERAL_STORE_ID)
        .sell_quotes
        .iter()
        .find(|quote| quote.item_id == item_id)
        .expect("player-made ammunition should be legal shop stock");
    assert_eq!(quote.unit_price, 1);
    let gold_before = game.gold;
    let sale = dispatch_next(
        &mut game,
        GameCommand::SellToShop {
            shop_id: GENERAL_STORE_ID.to_owned(),
            item_id,
            quantity: 1,
        },
    );
    assert!(sale.events.iter().any(|event| event.kind == "shop.sale"));
    assert_eq!(game.gold, gold_before + 1);
    let after_sale = game.snapshot();
    let stock = projected_shop(&after_sale.shops, GENERAL_STORE_ID)
        .stock
        .iter()
        .find(|item| item.kind_id == "demo.item.mithril-arrow")
        .expect("sold player-made ammunition should remain discounted in stock");
    assert_eq!(stock.unit_price, 1);
}

#[test]
fn black_market_uses_original_warrior_markup_and_markdown() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    game.gold = 1_000_000;
    game.player.position = Position { x: 115, y: 43 };
    game.mark_shop_visited_at_player().unwrap();
    let snapshot = game.snapshot();
    let shop = projected_shop(&snapshot.shops, BLACK_MARKET_ID);
    assert!(shop.visited);
    assert!(shop.player_at_entrance);
    assert_eq!(shop.owner.greed_percent, 150);
    assert_eq!(shop.owner.purchase_price_cap, 30_000);
    assert_eq!(shop.owner.price_factor_percent, 189);
    let black_channels = shop
        .stock
        .iter()
        .find(|item| item.kind_id == "demo.item.black-channels")
        .expect("Black Market should stock Black Channels");
    assert_eq!(black_channels.unit_price, 56_800);
    let ration = shop
        .sell_quotes
        .iter()
        .find(|item| item.kind_id == "demo.item.ration-of-food")
        .expect("Black Market should buy ordinary legal inventory");
    assert_eq!(ration.unit_price, 1);

    assert!(game.gain_mutation("rfb.mutation.black-marketeer", &mut Vec::new()));
    let discounted = game.snapshot();
    let shop = projected_shop(&discounted.shops, BLACK_MARKET_ID);
    assert_eq!(
        shop.stock
            .iter()
            .find(|item| item.kind_id == "demo.item.black-channels")
            .expect("Black Market should retain Black Channels")
            .unit_price,
        28_400
    );
    assert_eq!(
        shop.sell_quotes
            .iter()
            .find(|item| item.kind_id == "demo.item.ration-of-food")
            .expect("Black Market should retain the ration quote")
            .unit_price,
        1
    );

    let restored = Game::from_save(game.to_save()).expect("Black Market should round-trip");
    assert_eq!(restored.shop_states, game.shop_states);
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn temple_purchase_and_alchemist_visit_use_independent_shop_state() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    game.gold = 1_000;
    game.player.position = Position { x: 70, y: 29 };
    game.mark_shop_visited_at_player().unwrap();
    let temple_snapshot = game.snapshot();
    let temple = projected_shop(&temple_snapshot.shops, TEMPLE_ID);
    assert!(temple.visited);
    assert!(temple.player_at_entrance);
    assert_eq!(temple.owner.price_factor_percent, 137);
    let healing = temple
        .stock
        .iter()
        .find(|item| item.kind_id == "demo.item.light-healing-potion")
        .expect("Temple should stock light healing")
        .clone();
    assert_eq!(
        healing.display_name_key, "item-demo-light-healing-potion-name",
        "shop stock should use its known item name without revealing it globally"
    );
    assert_ne!(
        game.item_display_name_key("demo.item.light-healing-potion"),
        healing.display_name_key
    );
    let alchemist_before = game.shop_states[ALCHEMIST_ID].clone();
    let update = dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: TEMPLE_ID.to_owned(),
            item_id: healing.id,
            quantity: 1,
        },
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "shop.purchase")
    );
    assert_eq!(game.gold, 1_000 - healing.unit_price);
    assert_eq!(game.shop_states[ALCHEMIST_ID], alchemist_before);

    game.player.position = Position { x: 74, y: 43 };
    game.mark_shop_visited_at_player().unwrap();
    let snapshot = game.snapshot();
    let alchemist = projected_shop(&snapshot.shops, ALCHEMIST_ID);
    assert!(alchemist.visited);
    assert!(alchemist.player_at_entrance);
    assert_eq!(alchemist.owner.price_factor_percent, 139);
    assert!(
        alchemist
            .stock
            .iter()
            .any(|item| item.kind_id == "demo.item.flicker-scroll")
    );
    assert!(!projected_shop(&snapshot.shops, TEMPLE_ID).player_at_entrance);

    let restored = Game::from_save(game.to_save()).expect("seven shops should round-trip");
    assert_eq!(restored.shop_states, game.shop_states);
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn book_discovery_shop_groups_and_repurchase_do_not_count_as_found() {
    let mut game = Game::new_with_build(42, "demo.build.warrior").unwrap();
    game.gold = 10_000;
    game.player.position =
        position_from_content(game.content.shop(BOOKSTORE_ID).unwrap().entrance_position);
    game.mark_shop_visited_at_player().unwrap();
    let first = projected_shop(&game.snapshot().shops, BOOKSTORE_ID).stock[0].clone();
    let mut another = game.shop_states[BOOKSTORE_ID]
        .inventory
        .iter()
        .find(|item| item.id == first.id)
        .unwrap()
        .clone();
    another.id = game.allocate_item_instance_id().unwrap();
    game.shop_states
        .get_mut(BOOKSTORE_ID)
        .unwrap()
        .inventory
        .push(another);
    let book = projected_shop(&game.snapshot().shops, BOOKSTORE_ID).stock[0].clone();
    assert!(book.quantity >= 2);
    let before = game.state_hash();
    assert!(
        game.buy_from_shop(BOOKSTORE_ID, &book.id, book.quantity + 1)
            .is_err()
    );
    assert_eq!(game.state_hash(), before);
    let purchase = game.buy_from_shop(BOOKSTORE_ID, &book.id, 2).unwrap();
    assert!(!game.item_knowledge.contains_key(&book.kind_id));
    assert_eq!(
        game.items
            .iter()
            .filter(|item| item.kind_id == book.kind_id
                && item.location == ItemLocation::Inventory
                && item.book_counted)
            .count(),
        2
    );
    let sale = game
        .sell_to_shop(BOOKSTORE_ID, &purchase.item_id, 1)
        .unwrap();
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    game = restored;
    let repurchase = game.buy_from_shop(BOOKSTORE_ID, &sale.item_id, 1).unwrap();
    game.drop_inventory_quantity(&repurchase.item_id, 1)
        .unwrap()
        .unwrap();
    game.pick_up_item_at_player(Some(&repurchase.item_id))
        .unwrap();
    game.destroy_item(&repurchase.item_id, 1).unwrap();
    assert!(!game.item_knowledge.contains_key(&book.kind_id));

    // Source shop.c uses stats_on_purchase in both trade directions, including
    // an uncounted item entering the shop. It must not create a found count.
    support::give_inventory_item(&mut game, "test.uncounted-book", &book.kind_id);
    let sale = game
        .sell_to_shop(BOOKSTORE_ID, "test.uncounted-book", 1)
        .unwrap();
    assert!(
        game.shop_states[BOOKSTORE_ID]
            .inventory
            .iter()
            .find(|item| item.id == sale.item_id)
            .unwrap()
            .book_counted
    );
    assert!(!game.item_knowledge.contains_key(&book.kind_id));
}

#[test]
fn book_discovery_home_partial_groups_preserve_each_instance_through_save() {
    let mut game = Game::new_with_build(406, "demo.build.warrior").unwrap();
    game.player.position = position_from_content(
        game.content
            .town_facility(HOME_ID)
            .unwrap()
            .entrance_position,
    );
    game.mark_shop_visited_at_player().unwrap();
    let kind = "demo.item.black-prayers";
    for id in ["test.book-one", "test.book-two"] {
        support::give_inventory_item(&mut game, id, kind);
        game.identify_item_instance(id, ItemIdentificationRequest::new(true));
    }
    let deposit = game.deposit_at_home(HOME_ID, "test.book-one", 2).unwrap();
    assert_eq!(game.item_knowledge[kind].found_count, 2);
    assert_eq!(game.home_states[HOME_ID].inventory.len(), 2);
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    game = restored;
    game.withdraw_from_home(HOME_ID, &deposit.item_id, 1)
        .unwrap();
    assert_eq!(game.home_states[HOME_ID].inventory.len(), 1);
    assert!(game.home_states[HOME_ID].inventory[0].book_counted);
    assert!(
        game.items
            .iter()
            .filter(|item| item.kind_id == kind)
            .all(|item| item.book_counted)
    );
    assert_eq!(game.item_knowledge[kind].found_count, 2);
}

#[test]
fn book_discovery_is_character_history_not_shared_museum_knowledge() {
    let facility = "demo.town-facility.morivant-museum";
    let kind = "demo.item.black-prayers";
    let mut donor = town_facility_game(407, "demo.build.warrior", facility);
    donor.mark_shop_visited_at_player().unwrap();
    donor.reveal_current_visibility();
    support::give_inventory_item(&mut donor, "test.book", kind);
    donor.identify_item_instance("test.book", ItemIdentificationRequest::new(true));
    donor.deposit_at_home(facility, "test.book", 1).unwrap();
    let museum = donor.shared_museum().unwrap();
    assert!(museum.item_knowledge.is_empty());
    assert!(museum.inventory[0].book_counted);
    assert_eq!(donor.item_knowledge[kind].found_count, 1);
    let mut recipient = town_facility_game(408, "demo.build.warrior", facility);
    recipient.mark_shop_visited_at_player().unwrap();
    recipient.reveal_current_visibility();
    let mut recipient = recipient.with_shared_museum(&museum).unwrap().unwrap();
    let imported = recipient.shared_museum().unwrap();
    recipient
        .withdraw_from_home(facility, &imported.inventory[0].id, 1)
        .unwrap();
    assert!(!recipient.item_knowledge.contains_key(kind));
    let mut corrupt = museum;
    corrupt
        .item_knowledge
        .push(rfb_protocol::ItemKnowledgeSaveDto {
            kind_id: kind.to_owned(),
            tried: false,
            aware: false,
            found_count: 1,
        });
    assert!(recipient.with_shared_museum(&corrupt).is_err());
}

#[test]
fn bookstore_purchase_can_supply_an_original_spellbook_for_study() {
    let mut game = test_caster_game(42);
    game.gold = 10_000;
    game.items
        .retain(|item| item.location != ItemLocation::Inventory);
    game.player.position = Position { x: 89, y: 44 };
    game.mark_shop_visited_at_player().unwrap();

    let shop = projected_shop(&game.snapshot().shops, BOOKSTORE_ID).clone();
    assert!(shop.visited);
    assert!(shop.player_at_entrance);
    assert_eq!(shop.category, ShopCategoryDto::Bookstore);
    assert_eq!(shop.owner.greed_percent, 108);
    assert_eq!(shop.owner.purchase_price_cap, 10_000);
    assert_eq!(
        shop.stock
            .iter()
            .map(|item| (item.kind_id.as_str(), item.unit_price))
            .collect::<std::collections::BTreeMap<_, _>>(),
        std::collections::BTreeMap::from([
            ("demo.item.black-prayers", 135),
            ("demo.item.black-mass", 1_350),
            ("demo.item.cantrips-for-beginners", 135),
            ("demo.item.minor-arcana", 338),
            ("demo.item.major-arcana", 1_350),
            ("demo.item.manual-of-mastery", 3_380),
            ("demo.item.beginners-handbook", 135),
            ("demo.item.master-sorcerers-handbook", 1_350),
            ("demo.item.book-of-elements", 135),
            ("demo.item.earth-wind-and-fire", 1_350),
            ("demo.item.call-of-the-wild", 135),
            ("demo.item.nature-mastery", 1_350),
            ("demo.item.book-of-common-prayer", 135),
            ("demo.item.high-mass", 1_350),
            ("demo.item.dark-incantations", 135),
            ("demo.item.immortal-rituals", 1_350),
            ("demo.item.rites-of-initiation", 135),
            ("demo.item.ways-of-war", 1_350),
        ])
    );
    let book = shop
        .stock
        .iter()
        .find(|item| item.kind_id == "demo.item.black-prayers")
        .expect("Bookstore should stock Black Prayers")
        .clone();

    let purchase = dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: BOOKSTORE_ID.to_owned(),
            item_id: book.id,
            quantity: 1,
        },
    );
    let book_item_id = purchase
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.black-prayers")
        .expect("purchased book should be carried")
        .id
        .clone();
    let studied = dispatch_next(
        &mut game,
        GameCommand::StudyAbility {
            book_item_id,
            ability_id: "demo.ability.death-detect-evil".to_owned(),
        },
    );
    assert!(
        studied
            .player
            .abilities
            .iter()
            .any(|ability| { ability.id == "demo.ability.death-detect-evil" && ability.learned })
    );

    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("bookstore trade should round-trip");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn shared_forge_shops_group_stock_and_sell_equipment_that_can_be_used() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    game.gold = 10_000;
    let mut extra_arrows = game.shop_states[WEAPONSMITH_ID]
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.arrow")
        .expect("Weaponsmith should stock arrows")
        .clone();
    extra_arrows.id = game
        .allocate_item_instance_id()
        .expect("test arrow stock should allocate a unique item ID");
    extra_arrows.quantity = 99;
    game.shop_states
        .get_mut(WEAPONSMITH_ID)
        .expect("weaponsmith state should exist")
        .inventory
        .push(extra_arrows);
    game.player.position = Position { x: 126, y: 31 };
    game.mark_shop_visited_at_player().unwrap();

    let weaponsmith = projected_shop(&game.snapshot().shops, WEAPONSMITH_ID).clone();
    assert!(weaponsmith.visited);
    assert!(weaponsmith.player_at_entrance);
    assert_eq!(weaponsmith.owner.greed_percent, 110);
    assert_eq!(weaponsmith.owner.purchase_price_cap, 20_000);
    assert_eq!(
        weaponsmith
            .stock
            .iter()
            .filter(|item| item.kind_id == "demo.item.arrow")
            .count(),
        1,
        "compatible arrow stacks should project as one shop entry"
    );
    assert!(
        weaponsmith
            .stock
            .iter()
            .find(|item| item.kind_id == "demo.item.arrow")
            .is_some_and(|item| item.quantity > 99),
        "compatible arrow instances should group across the stack limit"
    );

    game.player.position = Position { x: 115, y: 28 };
    game.mark_shop_visited_at_player().unwrap();
    let armoury = projected_shop(&game.snapshot().shops, ARMOURY_ID).clone();
    assert!(armoury.visited);
    assert!(armoury.player_at_entrance);
    assert_eq!(armoury.owner.greed_percent, 111);
    assert_eq!(armoury.owner.purchase_price_cap, 20_000);
    let gloves = armoury
        .stock
        .iter()
        .find(|item| item.kind_id == "demo.item.leather-gloves")
        .expect("Armoury should stock RFB Leather Gloves")
        .clone();
    assert_eq!(gloves.unit_price, 4);

    let purchase = dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: ARMOURY_ID.to_owned(),
            item_id: gloves.id,
            quantity: 1,
        },
    );
    let glove_id = purchase
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.leather-gloves")
        .expect("purchased gloves should be carried")
        .id
        .clone();
    let equipped = dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: glove_id,
            slot_id: None,
        },
    );
    assert!(
        equipped
            .equipment
            .iter()
            .any(|item| { item.kind_id == "demo.item.leather-gloves" && item.slot_id == "hands" })
    );

    let restored = Game::from_save(game.to_save()).expect("forge trade should round-trip");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn magic_shop_purchase_device_use_and_save_are_authoritative() {
    let mut game =
        Game::new_with_build(43, "demo.build.warrior").expect("Warrens game should start");
    game.gold = 10_000;
    game.player.position = Position { x: 84, y: 43 };
    game.mark_shop_visited_at_player().unwrap();

    let shop = projected_shop(&game.snapshot().shops, MAGIC_SHOP_ID).clone();
    assert!(shop.visited);
    assert!(shop.player_at_entrance);
    assert_eq!(shop.category, ShopCategoryDto::MagicShop);
    assert_eq!(shop.owner.price_factor_percent, 138);
    assert_eq!(
        shop.stock
            .iter()
            .map(|item| item.kind_id.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from([
            "demo.item.magic-missile-wand",
            "demo.item.detect-objects-staff",
            "demo.item.identify-staff",
        ])
    );
    let staff = shop
        .stock
        .iter()
        .find(|item| item.kind_id == "demo.item.detect-objects-staff")
        .expect("Magic Shop should stock the RFB Detect Objects Staff")
        .clone();
    assert_eq!(
        staff.display_name_key,
        "item-demo-detect-objects-staff-name"
    );
    assert_eq!(staff.unit_price, 2_070);

    let purchase = dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: MAGIC_SHOP_ID.to_owned(),
            item_id: staff.id,
            quantity: 1,
        },
    );
    assert!(
        purchase
            .events
            .iter()
            .any(|event| event.kind == "shop.purchase")
    );
    let bought = purchase
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.detect-objects-staff")
        .expect("purchased staff should be carried");
    let charges_before = bought
        .charges
        .expect("purchased staff should retain its generated energy");
    // Fixed DETECT_OBJECTS: power max(source level 5, 7), capacity 3 * 7.
    assert_eq!(charges_before.maximum, 21);
    assert!(charges_before.current >= 4);
    let staff_id = bought.id.clone();

    game.rng = RfbRng::seeded(32);
    let used = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: staff_id.clone(),
            target: Some(TargetSelection::SelfTarget),
        },
    );
    assert!(
        used.events
            .iter()
            .any(|event| event.kind == "skill.device-success")
    );
    assert_eq!(
        used.inventory
            .iter()
            .find(|item| item.id == staff_id)
            .and_then(|item| item.charges),
        Some(ItemChargesDto {
            current: charges_before.current - 4,
            maximum: charges_before.maximum,
        })
    );

    let restored = Game::from_save(game.to_save()).expect("Magic Shop device state should reload");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn quantity_purchase_is_atomic_zero_time_and_identified() {
    let mut game = store_game(43);
    game.gold = 100;
    let item_id = stock_item_id(&game, "demo.item.ration-of-food");
    game.shop_states
        .get_mut(GENERAL_STORE_ID)
        .expect("general store should exist")
        .inventory
        .iter_mut()
        .find(|item| item.id == item_id)
        .expect("selected ration stock should remain available")
        .quantity = 2;
    let ration_before = game
        .items
        .iter()
        .find(|item| {
            item.kind_id == "demo.item.ration-of-food" && item.location == ItemLocation::Inventory
        })
        .expect("warrior should start with rations")
        .clone();
    let stack_count_before = game
        .items
        .iter()
        .filter(|item| {
            item.kind_id == ration_before.kind_id && item.location == ItemLocation::Inventory
        })
        .count();
    let before_tick = game.world_tick;
    let before_draws = game.rng_draw_counter();
    let update = dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: GENERAL_STORE_ID.to_owned(),
            item_id,
            quantity: 2,
        },
    );
    assert_eq!(game.gold, 92);
    assert_eq!(game.world_tick, before_tick);
    assert_eq!(game.rng_draw_counter(), before_draws);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "shop.purchase")
    );
    let bought = game
        .items
        .iter()
        .find(|item| item.id == ration_before.id)
        .expect("purchase should merge into the existing ration stack");
    assert_eq!(bought.quantity, ration_before.quantity + 2);
    assert_eq!(
        game.items
            .iter()
            .filter(|item| {
                item.kind_id == ration_before.kind_id && item.location == ItemLocation::Inventory
            })
            .count(),
        stack_count_before
    );
    assert_eq!(
        game.item_knowledge_dto(&bought.kind_id),
        ItemKnowledgeDto::Aware
    );
    assert!(game.item_property_knowledge[&bought.id].identified);
}

#[test]
fn rejected_purchase_preserves_rng_and_business_state() {
    let mut game = store_game(42);
    game.gold = 0;
    let item_id = stock_item_id(&game, "demo.item.brass-lantern");
    let business_before = game.shop_states.clone();
    let items_before = game.items.clone();
    let gold_before = game.gold;
    let tick_before = game.world_tick;
    let draws_before = game.rng_draw_counter();
    let update = dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: GENERAL_STORE_ID.to_owned(),
            item_id,
            quantity: 1,
        },
    );
    assert!(update.events.iter().any(|event| {
        event
            .args
            .get("reason")
            .is_some_and(|reason| reason == "insufficient-gold")
    }));
    assert_eq!(game.shop_states, business_before);
    assert_eq!(game.items, items_before);
    assert_eq!(game.gold, gold_before);
    assert_eq!(game.world_tick, tick_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
}

#[test]
fn overburdened_player_can_purchase() {
    let mut game = store_game(42);
    game.gold = 100;
    for item in &mut game.items {
        if item.location == ItemLocation::Inventory {
            item.quantity = game.content.item(&item.kind_id).unwrap().max_stack;
        }
    }
    let item_id = stock_item_id(&game, "demo.item.brass-lantern");
    let update = dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: GENERAL_STORE_ID.to_owned(),
            item_id,
            quantity: 1,
        },
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "shop.purchase")
    );
    assert!(game.items.iter().any(|item| {
        item.kind_id == "demo.item.brass-lantern" && item.location == ItemLocation::Inventory
    }));
}

#[test]
fn corpse_sale_is_rejected() {
    let mut game = store_game(42);
    support::give_inventory_item(&mut game, "test.corpse", "demo.item.corpse-remains");
    let draws_before = game.rng_draw_counter();
    let update = dispatch_next(
        &mut game,
        GameCommand::SellToShop {
            shop_id: GENERAL_STORE_ID.to_owned(),
            item_id: "test.corpse".to_owned(),
            quantity: 1,
        },
    );
    assert!(update.events.iter().any(|event| {
        event
            .args
            .get("reason")
            .is_some_and(|reason| reason == "item-illegal")
    }));
    assert!(game.items.iter().any(|item| item.id == "test.corpse"));
    assert_eq!(game.rng_draw_counter(), draws_before);
}

#[test]
fn sold_item_can_be_bought_back_with_full_instance_state() {
    let mut game = store_game(42);
    game.gold = 100;
    let original = game
        .items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.wooden-torch")
        .map(|item| {
            item.fuel
                .as_mut()
                .expect("starting torch must have fuel")
                .current = 1_234;
            item.inscription = Some("@m1".to_owned());
            item.clone()
        })
        .expect("warrior should start with torches")
        .clone();
    let sale = dispatch_next(
        &mut game,
        GameCommand::SellToShop {
            shop_id: GENERAL_STORE_ID.to_owned(),
            item_id: original.id.clone(),
            quantity: 1,
        },
    );
    assert!(sale.events.iter().any(|event| event.kind == "shop.sale"));
    assert_eq!(
        sale.shops
            .iter()
            .find(|shop| shop.id == GENERAL_STORE_ID)
            .and_then(|shop| shop.stock.iter().find(|item| item.id == original.id))
            .and_then(|item| item.inscription.as_deref()),
        Some("@m1")
    );
    let sold = game.shop_states[GENERAL_STORE_ID]
        .inventory
        .iter()
        .find(|item| item.id == original.id)
        .expect("sold item should enter store")
        .clone();
    let purchase = dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: GENERAL_STORE_ID.to_owned(),
            item_id: sold.id.clone(),
            quantity: 1,
        },
    );
    assert!(
        purchase
            .events
            .iter()
            .any(|event| event.kind == "shop.purchase")
    );
    let repurchased = game
        .items
        .iter()
        .find(|item| item.id == sold.id)
        .expect("repurchased item should retain full instance");
    assert_eq!(repurchased.kind_id, original.kind_id);
    assert_eq!(repurchased.fuel, original.fuel);
    assert_eq!(repurchased.charges, original.charges);
    assert_eq!(repurchased.inscription, original.inscription);
    assert!(game.item_property_knowledge[&repurchased.id].identified);
}

#[test]
fn compatible_shop_instances_project_and_trade_as_one_row() {
    let mut game = store_game(42);
    game.gold = 100;
    let mut extra_ration = game.shop_states[GENERAL_STORE_ID]
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.ration-of-food")
        .expect("store should stock rations")
        .clone();
    extra_ration.id = "demo.shop.outpost-general-store.test-ration".to_owned();
    extra_ration.quantity = 1;
    game.shop_states
        .get_mut(GENERAL_STORE_ID)
        .expect("general store state should exist")
        .inventory
        .push(extra_ration);
    let shop_before = projected_shop(&game.snapshot().shops, GENERAL_STORE_ID).clone();
    for kind_id in [
        "demo.item.ration-of-food",
        "demo.item.wooden-torch",
        "demo.item.brass-lantern",
        "demo.item.flask-of-oil",
    ] {
        assert_eq!(
            shop_before
                .stock
                .iter()
                .filter(|item| item.kind_id == kind_id)
                .count(),
            1,
            "compatible {kind_id} stock should use one row"
        );
    }

    let ration = shop_before
        .stock
        .iter()
        .find(|item| item.kind_id == "demo.item.ration-of-food")
        .expect("store should stock rations");
    assert!(ration.maximum_quantity >= 2);
    let ration_item_id = ration.id.clone();
    let shop_rations_before = game.shop_states[GENERAL_STORE_ID]
        .inventory
        .iter()
        .filter(|item| item.kind_id == "demo.item.ration-of-food")
        .map(|item| item.quantity)
        .sum::<u32>();
    let carried_rations_before = game
        .items
        .iter()
        .filter(|item| {
            item.kind_id == "demo.item.ration-of-food" && item.location == ItemLocation::Inventory
        })
        .map(|item| item.quantity)
        .sum::<u32>();
    let purchase = dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: GENERAL_STORE_ID.to_owned(),
            item_id: ration_item_id,
            quantity: 2,
        },
    );
    assert!(
        purchase
            .events
            .iter()
            .any(|event| event.kind == "shop.purchase")
    );
    assert_eq!(
        game.shop_states[GENERAL_STORE_ID]
            .inventory
            .iter()
            .filter(|item| item.kind_id == "demo.item.ration-of-food")
            .map(|item| item.quantity)
            .sum::<u32>(),
        shop_rations_before - 2
    );
    assert_eq!(
        game.items
            .iter()
            .filter(|item| {
                item.kind_id == "demo.item.ration-of-food"
                    && item.location == ItemLocation::Inventory
            })
            .map(|item| item.quantity)
            .sum::<u32>(),
        carried_rations_before + 2
    );

    let shop_after_purchase = projected_shop(&game.snapshot().shops, GENERAL_STORE_ID).clone();
    let quote = shop_after_purchase
        .sell_quotes
        .iter()
        .find(|quote| quote.kind_id == "demo.item.ration-of-food" && quote.maximum_quantity >= 2)
        .expect("compatible carried rations should have one grouped quote");
    let sale = dispatch_next(
        &mut game,
        GameCommand::SellToShop {
            shop_id: GENERAL_STORE_ID.to_owned(),
            item_id: quote.item_id.clone(),
            quantity: 2,
        },
    );
    assert!(sale.events.iter().any(|event| event.kind == "shop.sale"));
    assert_eq!(
        game.items
            .iter()
            .filter(|item| {
                item.kind_id == "demo.item.ration-of-food"
                    && item.location == ItemLocation::Inventory
            })
            .map(|item| item.quantity)
            .sum::<u32>(),
        carried_rations_before
    );
    assert_eq!(
        projected_shop(&game.snapshot().shops, GENERAL_STORE_ID)
            .stock
            .iter()
            .filter(|item| item.kind_id == "demo.item.ration-of-food")
            .count(),
        1
    );
}

#[test]
fn maintenance_refills_only_after_interval_at_entrance() {
    let mut game = store_game(42);
    game.shop_states
        .get_mut(GENERAL_STORE_ID)
        .unwrap()
        .inventory
        .clear();
    game.world_tick = 9_999;
    let draws_before = game.rng_draw_counter();
    game.maintain_shop_at_player().unwrap();
    assert!(game.shop_states[GENERAL_STORE_ID].inventory.is_empty());
    assert_eq!(game.rng_draw_counter(), draws_before);
    game.world_tick = 10_000;
    game.maintain_shop_at_player().unwrap();
    let state = &game.shop_states[GENERAL_STORE_ID];
    let guaranteed_stock = game
        .content
        .shop(GENERAL_STORE_ID)
        .expect("General Store should exist")
        .stock
        .iter()
        .filter(|stock| stock.availability_percent == 100)
        .map(|stock| stock.item_kind_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(state.last_maintenance_world_tick, 10_000);
    let stocked = state
        .inventory
        .iter()
        .map(|item| item.kind_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(guaranteed_stock.is_subset(&stocked));
    assert!(game.rng_draw_counter() > draws_before);
}

#[test]
fn p106_bounty_office_projects_and_redeems_daily_and_wanted_remains() {
    let mut game =
        Game::new_with_build(106, "demo.build.warrior").expect("Middle-earth game should start");
    game.player.position = Position { x: 84, y: 26 };
    let office = game
        .snapshot()
        .task_services
        .into_iter()
        .find(|service| service.id == OUTPOST_BOUNTY_OFFICE_ID)
        .expect("Outpost bounty office should be projected");
    let bounty = office
        .bounty_office
        .expect("bounty details should be visible at the entrance");
    assert_eq!(bounty.wanted_targets.len(), 20);

    let daily = bounty.daily_target;
    add_bounty_remains(&mut game, "test.bounty.daily", &daily.actor_kind_id);
    let gold_before = game.gold;
    let daily_update = dispatch_next(
        &mut game,
        GameCommand::UseBountyOffice {
            facility_id: OUTPOST_BOUNTY_OFFICE_ID.to_owned(),
            action: BountyOfficeActionDto::TurnIn,
            item_id: Some("test.bounty.daily".to_owned()),
        },
    );
    assert!(
        daily_update
            .events
            .iter()
            .any(|event| event.kind == "bounty.daily-turned-in")
    );
    assert_eq!(game.gold, gold_before + daily.corpse_reward);
    assert_eq!(game.fame, 0, "daily bounties do not grant fame");
    assert!(!game.items.iter().any(|item| item.id == "test.bounty.daily"));

    let wanted = bounty
        .wanted_targets
        .into_iter()
        .find(|target| !target.completed)
        .expect("a new character should have an open wanted target");
    let reward_before = game
        .items
        .iter()
        .filter(|item| item.kind_id == wanted.reward_item_kind_id)
        .map(|item| item.quantity)
        .sum::<u32>();
    add_bounty_remains(&mut game, "test.bounty.wanted", &wanted.actor_kind_id);
    let wanted_update = dispatch_next(
        &mut game,
        GameCommand::UseBountyOffice {
            facility_id: OUTPOST_BOUNTY_OFFICE_ID.to_owned(),
            action: BountyOfficeActionDto::TurnIn,
            item_id: Some("test.bounty.wanted".to_owned()),
        },
    );
    assert_eq!(game.fame, 1);
    assert!(
        wanted_update
            .events
            .iter()
            .any(|event| event.kind == "bounty.wanted-turned-in")
    );
    assert!(
        game.bounty_state
            .completed_wanted_actor_kind_ids
            .contains(&wanted.actor_kind_id)
    );
    assert_eq!(
        game.items
            .iter()
            .filter(|item| item.kind_id == wanted.reward_item_kind_id)
            .map(|item| item.quantity)
            .sum::<u32>(),
        reward_before + 1
    );
}

#[test]
fn p106_dynamic_bounty_spawns_only_counted_targets_and_round_trips() {
    let mut game =
        Game::new_with_build(206, "demo.build.warrior").expect("Middle-earth game should start");
    game.player.position = Position { x: 84, y: 26 };
    let update = dispatch_next(
        &mut game,
        GameCommand::UseBountyOffice {
            facility_id: OUTPOST_BOUNTY_OFFICE_ID.to_owned(),
            action: BountyOfficeActionDto::RequestMission,
            item_id: None,
        },
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "bounty.mission-requested")
    );
    let mission = game
        .bounty_state
        .mission
        .clone()
        .expect("requesting a bounty should create a mission");
    let definition = game
        .content
        .world(&game.world_id)
        .and_then(|world| {
            world
                .procedural_floors
                .iter()
                .find(|floor| floor.id == mission.floor_id)
        })
        .expect("bounty floor should exist")
        .clone();
    let floor = game
        .generate_procedural_floor(&definition, None)
        .expect("bounty floor should generate");
    let targets = floor
        .entities
        .iter()
        .filter(|actor| actor.id.contains(".bounty-target."))
        .collect::<Vec<_>>();
    assert_eq!(targets.len(), usize::from(mission.total));
    assert!(
        targets
            .iter()
            .all(|actor| actor.kind_id == mission.actor_kind_id)
    );

    game.current_floor_id.clone_from(&mission.floor_id);
    game.command_actor_deaths.push(ActorDeathRecord {
        actor_id: "ordinary.same-kind".to_owned(),
        actor_kind_id: mission.actor_kind_id.clone(),
        position: Position::default(),
        credit_player: true,
    });
    assert_eq!(game.apply_bounty_deaths(), None);
    assert_eq!(
        game.bounty_state.mission.as_ref().unwrap().remaining,
        mission.total
    );
    game.command_actor_deaths = targets
        .iter()
        .map(|actor| ActorDeathRecord {
            actor_id: actor.id.clone(),
            actor_kind_id: actor.kind_id.clone(),
            position: actor.position,
            credit_player: true,
        })
        .collect();
    assert_eq!(
        game.apply_bounty_deaths(),
        Some(mission.actor_kind_id.clone())
    );
    assert_eq!(game.bounty_state.mission.as_ref().unwrap().remaining, 0);

    let mut restored = Game::from_save(game.to_save()).expect("bounty mission should round-trip");
    assert_eq!(restored.bounty_state, game.bounty_state);
    restored.current_floor_id = "demo.floor.surface".to_owned();
    restored.current_dungeon_instance_id = None;
    restored.player.position = Position { x: 84, y: 26 };
    let reward_kind_id = restored
        .snapshot()
        .task_services
        .into_iter()
        .find(|service| service.id == OUTPOST_BOUNTY_OFFICE_ID)
        .and_then(|service| service.bounty_office)
        .and_then(|bounty| bounty.mission)
        .expect("completed mission should be projected")
        .reward_item_kind_id;
    let claim = dispatch_next(
        &mut restored,
        GameCommand::UseBountyOffice {
            facility_id: OUTPOST_BOUNTY_OFFICE_ID.to_owned(),
            action: BountyOfficeActionDto::ClaimMissionReward,
            item_id: None,
        },
    );
    assert!(
        claim
            .events
            .iter()
            .any(|event| event.kind == "bounty.mission-rewarded")
    );
    assert!(restored.bounty_state.mission.is_none());
    assert_eq!(restored.fame, 1);
    assert!(
        restored
            .items
            .iter()
            .any(|item| item.kind_id == reward_kind_id && item.location == ItemLocation::Inventory)
    );
}
