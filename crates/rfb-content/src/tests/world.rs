use std::collections::BTreeSet;

use super::*;

#[test]
fn outpost_has_walls_inner_shops_and_an_exterior_warrens_entrance() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let world = artifact
        .content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.warrens-journey")
        .expect("fixture should contain Warrens");
    assert_eq!(world.town_id.as_deref(), Some("demo.town.outpost"));
    let entrances = [
        (
            "demo.terrain.general-store-entrance",
            ContentPosition { x: 32, y: 13 },
        ),
        (
            "demo.terrain.temple-entrance",
            ContentPosition { x: 45, y: 19 },
        ),
        (
            "demo.terrain.alchemist-entrance",
            ContentPosition { x: 53, y: 13 },
        ),
        (
            "demo.terrain.magic-shop-entrance",
            ContentPosition { x: 57, y: 13 },
        ),
        (
            "demo.terrain.bookstore-entrance",
            ContentPosition { x: 55, y: 13 },
        ),
        (
            "demo.terrain.armoury-entrance",
            ContentPosition { x: 30, y: 19 },
        ),
        (
            "demo.terrain.weaponsmith-entrance",
            ContentPosition { x: 34, y: 19 },
        ),
        (
            "demo.terrain.black-market-entrance",
            ContentPosition { x: 55, y: 19 },
        ),
    ];
    for (terrain_id, entrance) in entrances {
        assert!(world.terrain_overrides.iter().any(|terrain| {
            terrain.terrain_id == terrain_id && terrain.positions == [entrance]
        }));
    }

    let fortifications = world
        .terrain_overrides
        .iter()
        .find(|terrain| terrain.terrain_id == "demo.terrain.outpost-fortification")
        .expect("fixture should contain town fortifications");
    assert_eq!((world.width, world.height), (96, 32));
    assert!(
        world
            .procedural_floors
            .iter()
            .all(|floor| (floor.width, floor.height) == (66, 22))
    );
    let expected_fortifications = (22..=66)
        .flat_map(|x| [ContentPosition { x, y: 6 }, ContentPosition { x, y: 25 }])
        .chain((7..=24).flat_map(|y| [ContentPosition { x: 22, y }, ContentPosition { x: 66, y }]))
        .filter(|position| position.y != 16)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        fortifications
            .positions
            .iter()
            .copied()
            .collect::<BTreeSet<_>>(),
        expected_fortifications,
        "the Outpost should have one continuous perimeter interrupted only by its gates"
    );
    let gates = world
        .terrain_overrides
        .iter()
        .find(|terrain| terrain.terrain_id == "demo.terrain.outpost-gate")
        .expect("fixture should contain town gates");
    assert_eq!(
        gates.positions,
        [
            ContentPosition { x: 22, y: 16 },
            ContentPosition { x: 66, y: 16 }
        ]
    );
    assert!(entrances.iter().all(|(_, position)| {
        position.x > 22 && position.x < 66 && position.y > 6 && position.y < 25
    }));
    let warrens_entrance = world
        .terrain_overrides
        .iter()
        .find(|terrain| terrain.terrain_id == "demo.terrain.stairs-down")
        .expect("fixture should contain the Warrens entrance");
    assert_eq!(
        warrens_entrance.positions,
        [ContentPosition { x: 74, y: 16 }]
    );
    assert!(warrens_entrance.positions[0].x > 66);

    let mut wrong_entrance = artifact.content.clone();
    wrong_entrance
        .shops
        .iter_mut()
        .find(|shop| shop.id == "demo.shop.outpost-general-store")
        .unwrap()
        .entrance_position = ContentPosition { x: 18, y: 8 };
    assert!(matches!(
        validate_and_normalize(&mut wrong_entrance),
        Err(ContentError::InvalidShop(id)) if id == "demo.shop.outpost-general-store"
    ));

    let mut unowned_shop = artifact.content.clone();
    unowned_shop.towns[0]
        .shop_ids
        .retain(|shop_id| shop_id != "demo.shop.outpost-general-store");
    assert!(matches!(
        validate_and_normalize(&mut unowned_shop),
        Err(ContentError::InvalidShop(id)) if id == "demo.shop.outpost-general-store"
    ));
}

#[test]
fn general_store_economy_content_enforces_generic_stock_rules() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let store_id = "demo.shop.outpost-general-store";

    let mutations: [fn(&mut CompiledContentV1); 4] = [
        |content: &mut CompiledContentV1| {
            content
                .shops
                .iter_mut()
                .find(|shop| shop.id == "demo.shop.outpost-general-store")
                .unwrap()
                .owner
                .greed_percent = 99;
        },
        |content: &mut CompiledContentV1| {
            let store = content
                .shops
                .iter_mut()
                .find(|shop| shop.id == "demo.shop.outpost-general-store")
                .unwrap();
            store.stock.push(store.stock[0].clone());
        },
        |content: &mut CompiledContentV1| {
            let shop = content
                .shops
                .iter_mut()
                .find(|shop| shop.id == "demo.shop.outpost-general-store")
                .unwrap();
            shop.stock[0].initial_minimum = shop.stock[0].initial_maximum + 1;
        },
        |content: &mut CompiledContentV1| {
            let kind_id = content
                .shops
                .iter()
                .find(|shop| shop.id == "demo.shop.outpost-general-store")
                .unwrap()
                .stock[0]
                .item_kind_id
                .clone();
            content
                .items
                .iter_mut()
                .find(|item| item.id == kind_id)
                .expect("stock kind should exist")
                .base_value = 0;
        },
    ];
    for mutate in mutations {
        let mut invalid = artifact.content.clone();
        mutate(&mut invalid);
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidShop(id)) if id == store_id
        ));
    }

    let mut extended = artifact.content.clone();
    let template = extended
        .shops
        .iter()
        .find(|shop| shop.id == store_id)
        .unwrap()
        .stock[0]
        .clone();
    let mut added = template;
    added.item_kind_id = "demo.item.leather-pouch".to_owned();
    extended
        .shops
        .iter_mut()
        .find(|shop| shop.id == store_id)
        .unwrap()
        .stock
        .push(added);
    validate_and_normalize(&mut extended).expect("a valid new stock item should be data-only");

    let mut invalid_owner = artifact.content.clone();
    invalid_owner
        .shops
        .iter_mut()
        .find(|shop| shop.id == store_id)
        .unwrap()
        .owner
        .race_id = "demo.race.missing".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_owner),
        Err(ContentError::InvalidShop(id)) if id == store_id
    ));

    let mut invalid_value = artifact.content.clone();
    invalid_value
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.ration-of-food")
        .expect("ration should exist")
        .base_value = 1_000_000_000;
    assert!(matches!(
        validate_and_normalize(&mut invalid_value),
        Err(ContentError::InvalidItemValue(id)) if id == "demo.item.ration-of-food"
    ));

    let mut invalid_race_factor = artifact.content.clone();
    invalid_race_factor
        .races
        .iter_mut()
        .find(|race| race.id == "demo.race.rfb-human")
        .expect("Human race should exist")
        .shop_adjust_percent = 49;
    assert!(matches!(
        validate_and_normalize(&mut invalid_race_factor),
        Err(ContentError::InvalidCharacterSource(id)) if id == "demo.race.rfb-human"
    ));
}

#[test]
fn temple_and_alchemist_stock_are_strictly_separated() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let expected = [
        (
            "demo.shop.outpost-temple",
            BTreeSet::from([
                "demo.item.light-healing-potion",
                "demo.item.valor-tonic",
                "demo.item.homeward-scroll",
                "demo.item.cleansing-scroll",
            ]),
        ),
        (
            "demo.shop.outpost-alchemist",
            BTreeSet::from([
                "demo.item.flicker-scroll",
                "demo.item.farstep-scroll",
                "demo.item.seeking-scroll",
                "demo.item.trapfinding-scroll",
                "demo.item.temperate-tonic",
            ]),
        ),
    ];
    for (shop_id, item_ids) in expected {
        let shop = artifact
            .content
            .shops
            .iter()
            .find(|shop| shop.id == shop_id)
            .expect("shop should exist");
        assert_eq!(
            shop.stock
                .iter()
                .map(|stock| stock.item_kind_id.as_str())
                .collect::<BTreeSet<_>>(),
            item_ids
        );
    }
}

#[test]
fn selected_legacy_equipment_is_exposed_by_its_shop_and_warrens_depth() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let shop_stock = |id: &str| {
        artifact
            .content
            .shops
            .iter()
            .find(|shop| shop.id == id)
            .expect("shop should exist")
            .stock
            .iter()
            .map(|entry| entry.item_kind_id.as_str())
            .collect::<BTreeSet<_>>()
    };
    assert!(
        BTreeSet::from([
            "demo.item.club",
            "demo.item.dagger",
            "demo.item.main-gauche",
            "demo.item.tanto",
            "demo.item.whip",
            "demo.item.rapier",
            "demo.item.small-sword",
            "demo.item.cutlass",
            "demo.item.mace",
            "demo.item.shovel",
            "demo.item.pick",
        ])
        .is_subset(&shop_stock("demo.shop.outpost-weaponsmith"))
    );
    assert!(
        BTreeSet::from([
            "demo.item.cloak",
            "demo.item.robe",
            "demo.item.padded-armour",
            "demo.item.knit-cap",
            "demo.item.soft-leather-armour",
            "demo.item.soft-studded-leather",
            "demo.item.hard-leather-armour",
            "demo.item.hard-studded-leather",
            "demo.item.pair-of-hard-leather-boots",
            "demo.item.cord-armour",
            "demo.item.metal-cap",
            "demo.item.small-metal-shield",
            "demo.item.large-leather-shield",
            "demo.item.set-of-studded-leather-gloves",
            "demo.item.set-of-gauntlets",
        ])
        .is_subset(&shop_stock("demo.shop.outpost-armoury"))
    );

    let warrens = artifact
        .content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.warrens")
        .expect("Warrens loot table should exist");
    let depth = |id: &str| {
        warrens
            .entries
            .iter()
            .find(|entry| entry.item_kind_id == id)
            .map(|entry| (entry.min_depth, entry.max_depth))
    };
    assert_eq!(depth("demo.item.club"), Some((0, 9)));
    assert_eq!(depth("demo.item.dagger"), Some((0, 9)));
    assert_eq!(depth("demo.item.cloak"), Some((1, 9)));
    assert_eq!(depth("demo.item.robe"), Some((1, 9)));
    assert_eq!(depth("demo.item.shovel"), Some((1, 9)));
    assert_eq!(depth("demo.item.padded-armour"), Some((2, 9)));
    assert_eq!(depth("demo.item.knit-cap"), Some((3, 9)));
    assert_eq!(depth("demo.item.main-gauche"), Some((3, 9)));
    assert_eq!(depth("demo.item.soft-leather-armour"), Some((3, 9)));
    assert_eq!(depth("demo.item.soft-studded-leather"), Some((3, 9)));
    assert_eq!(depth("demo.item.tanto"), Some((3, 9)));
    assert_eq!(depth("demo.item.whip"), Some((3, 9)));
    assert_eq!(depth("demo.item.cord-armour"), Some((5, 9)));
    assert_eq!(depth("demo.item.cutlass"), Some((5, 9)));
    assert_eq!(depth("demo.item.hard-leather-armour"), Some((5, 9)));
    assert_eq!(depth("demo.item.rapier"), Some((5, 9)));
    assert_eq!(depth("demo.item.mace"), Some((5, 9)));
    assert_eq!(depth("demo.item.pair-of-hard-leather-boots"), Some((5, 9)));
    assert_eq!(depth("demo.item.pick"), Some((5, 9)));
    assert_eq!(depth("demo.item.small-sword"), Some((5, 9)));
    assert_eq!(
        depth("demo.item.set-of-studded-leather-gloves"),
        Some((5, 9))
    );
    assert_eq!(depth("demo.item.metal-cap"), None);
    assert_eq!(depth("demo.item.small-metal-shield"), None);
    assert_eq!(depth("demo.item.large-leather-shield"), None);
    assert_eq!(depth("demo.item.hard-studded-leather"), None);
    assert_eq!(depth("demo.item.set-of-gauntlets"), None);
}

#[test]
fn bookstore_stocks_original_town_books() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let shop_id = "demo.shop.outpost-bookstore";
    let shop = artifact
        .content
        .shops
        .iter()
        .find(|shop| shop.id == shop_id)
        .expect("bookstore should exist");
    assert_eq!(shop.category, ShopCategory::Bookstore);
    assert_eq!(
        shop.stock
            .iter()
            .map(|stock| stock.item_kind_id.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["demo.item.stench-of-death", "demo.item.sepulchral-ways",])
    );
    let values = artifact
        .content
        .items
        .iter()
        .filter(|item| shop.stock.iter().any(|stock| stock.item_kind_id == item.id))
        .map(|item| (item.id.as_str(), item.base_value))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(values["demo.item.stench-of-death"], 100);
    assert_eq!(values["demo.item.sepulchral-ways"], 1_000);
}

#[test]
fn black_market_stocks_original_non_town_books() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let shop_id = "demo.shop.outpost-black-market";
    let shop = artifact
        .content
        .shops
        .iter()
        .find(|shop| shop.id == shop_id)
        .expect("Black Market should exist");
    assert_eq!(shop.category, ShopCategory::BlackMarket);
    assert_eq!(shop.owner.greed_percent, 150);
    assert_eq!(shop.owner.purchase_price_cap, 30_000);
    assert_eq!(
        shop.stock
            .iter()
            .map(|stock| stock.item_kind_id.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["demo.item.black-channels", "demo.item.necronomicon"])
    );
    let values = artifact
        .content
        .items
        .iter()
        .filter(|item| shop.stock.iter().any(|stock| stock.item_kind_id == item.id))
        .map(|item| (item.id.as_str(), item.base_value))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(values["demo.item.black-channels"], 15_000);
    assert_eq!(values["demo.item.necronomicon"], 100_000);
}

#[test]
fn guaranteed_floor_supplies_require_rfb_chance_and_supported_items() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let warrens = artifact
        .content
        .worlds
        .iter()
        .find(|world| world.id == "demo.world.warrens-journey")
        .expect("fixture should contain Warrens");
    assert!(warrens.procedural_floors.iter().all(|floor| {
        floor.guaranteed_items.as_slice()
            == [
                ProceduralGuaranteedItemDefinition {
                    id: "demo.guaranteed.warrens-food".to_owned(),
                    chance_one_in: 2,
                    entries: vec![ProceduralGuaranteedItemEntryDefinition {
                        item_kind_id: "demo.item.ration-of-food".to_owned(),
                        weight: 1,
                    }],
                },
                ProceduralGuaranteedItemDefinition {
                    id: "demo.guaranteed.warrens-light".to_owned(),
                    chance_one_in: 2,
                    entries: vec![
                        ProceduralGuaranteedItemEntryDefinition {
                            item_kind_id: "demo.item.flask-of-oil".to_owned(),
                            weight: 1,
                        },
                        ProceduralGuaranteedItemEntryDefinition {
                            item_kind_id: "demo.item.brass-lantern".to_owned(),
                            weight: 2,
                        },
                    ],
                },
            ]
    }));

    let mut invalid_chance = artifact.content.clone();
    invalid_chance
        .worlds
        .iter_mut()
        .find(|world| world.id == "demo.world.warrens-journey")
        .expect("fixture should contain Warrens")
        .procedural_floors[0]
        .guaranteed_items[0]
        .chance_one_in = 1;
    assert!(matches!(
        validate_and_normalize(&mut invalid_chance),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut invalid_item = artifact.content.clone();
    invalid_item
        .worlds
        .iter_mut()
        .find(|world| world.id == "demo.world.warrens-journey")
        .expect("fixture should contain Warrens")
        .procedural_floors[0]
        .guaranteed_items[0]
        .entries[0]
        .item_kind_id = "demo.item.broad-sword".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_item),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn loot_tables_require_valid_weights_references_and_instance_shapes() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut zero_weight = artifact.content.clone();
    zero_weight
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.ember-mote")
        .expect("fixture should contain the death loot table")
        .entries[0]
        .weight = 0;
    assert!(matches!(
        validate_and_normalize(&mut zero_weight),
        Err(ContentError::InvalidLootTable(_))
    ));

    let mut dangling_affix = artifact.content.clone();
    dangling_affix
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.ember-mote")
        .expect("fixture should contain the death loot table")
        .affix_weights[1]
        .affix_id = Some("demo.affix.missing".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut dangling_affix),
        Err(ContentError::DanglingReference { .. })
    ));

    let mut stackable_quality = artifact.content.clone();
    stackable_quality
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.ember-mote")
        .expect("fixture should contain the death loot table")
        .entries[0]
        .item_kind_id = "demo.item.luminous-shard".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut stackable_quality),
        Err(ContentError::InvalidLootTable(_))
    ));

    let mut player_drop = artifact.content.clone();
    let player = player_drop
        .actors
        .iter_mut()
        .find(|actor| actor.role == ActorRole::Player)
        .expect("fixture should contain the player");
    player.loot_table_id = Some("demo.loot-table.ember-mote".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut player_drop),
        Err(ContentError::InvalidActorLootTable(_))
    ));

    let mut player_carry = artifact.content.clone();
    let player = player_carry
        .actors
        .iter_mut()
        .find(|actor| actor.role == ActorRole::Player)
        .expect("fixture should contain the player");
    player.carried_loot_table_id = Some("demo.loot-table.ember-mote-carried".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut player_carry),
        Err(ContentError::InvalidActorLootTable(_))
    ));

    let mut invalid_chance = artifact.content.clone();
    invalid_chance
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.small-kobold")
        .expect("fixture should contain the probabilistic loot table")
        .roll_chance_percent = Some(0);
    assert!(matches!(
        validate_and_normalize(&mut invalid_chance),
        Err(ContentError::InvalidLootTable(_))
    ));

    let mut invalid_dice = artifact.content.clone();
    invalid_dice
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.warrens-keeper")
        .expect("fixture should contain the diced loot table")
        .roll_dice
        .as_mut()
        .expect("Warrens keeper should use a drop-count die")
        .sides = 0;
    assert!(matches!(
        validate_and_normalize(&mut invalid_dice),
        Err(ContentError::InvalidLootTable(_))
    ));

    let mut inverted_depth = artifact.content.clone();
    let entry = &mut inverted_depth
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.warrens")
        .expect("fixture should contain the floor loot table")
        .entries[0];
    entry.min_depth = 2;
    entry.max_depth = 1;
    assert!(matches!(
        validate_and_normalize(&mut inverted_depth),
        Err(ContentError::InvalidLootTable(_))
    ));

    let mut dangling_guardian_reward = artifact.content.clone();
    dangling_guardian_reward
        .worlds
        .iter_mut()
        .find(|world| world.id == "demo.world.warrens-journey")
        .expect("fixture should contain the Warrens journey")
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.final_floor)
        .and_then(|floor| floor.guardian.as_mut())
        .expect("Warrens should contain a final guardian")
        .reward_loot_table_id = Some("demo.loot-table.missing".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut dangling_guardian_reward),
        Err(ContentError::DanglingReference { .. })
    ));
}

#[test]
fn procedural_floor_tables_require_valid_depth_roles_and_references() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut zero_depth = artifact.content.clone();
    zero_depth.worlds[0].procedural_floors[0].depth = 0;
    assert!(matches!(
        validate_and_normalize(&mut zero_depth),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut player_candidate = artifact.content.clone();
    player_candidate.encounter_tables[0].entries[0].actor_kind_id =
        "demo.actor.explorer".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut player_candidate),
        Err(ContentError::WrongActorRole(_))
    ));

    let mut dangling_loot = artifact.content.clone();
    dangling_loot.worlds[0].procedural_floors[0].loot_table_id =
        Some("demo.loot-table.missing".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut dangling_loot),
        Err(ContentError::DanglingReference { .. })
    ));

    let mut duplicate_actor = artifact.content.clone();
    duplicate_actor.worlds[0].procedural_floors[0].encounter_table_id = None;
    duplicate_actor.worlds[0].procedural_floors[0].generation_budget = None;
    duplicate_actor.worlds[0].procedural_floors[0].nest = None;
    duplicate_actor.worlds[0].procedural_floors[0]
        .actor_spawns
        .push(ProceduralActorSpawnDefinition {
            instance_id: "demo.monster.ember-mote.1".to_owned(),
            room_id: "remote".to_owned(),
            actor_kind_ids: vec!["demo.actor.echo-hound".to_owned()],
        });
    assert!(matches!(
        validate_and_normalize(&mut duplicate_actor),
        Err(ContentError::DuplicateInstanceId(_))
    ));

    let mut zero_weight = artifact.content.clone();
    zero_weight.encounter_tables[0].entries[0].weight = 0;
    assert!(matches!(
        validate_and_normalize(&mut zero_weight),
        Err(ContentError::InvalidEncounterTable(_))
    ));

    let mut missing_theme = artifact.content.clone();
    missing_theme.worlds[0].procedural_floors[0].theme_table_id =
        Some("demo.theme-table.missing".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut missing_theme),
        Err(ContentError::DanglingReference { .. })
    ));

    let mut exhausted_actor_budget = artifact.content.clone();
    exhausted_actor_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-1")
        .expect("fixture should contain the nest floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .actor_slots = 3;
    assert!(matches!(
        validate_and_normalize(&mut exhausted_actor_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut exhausted_loot_budget = artifact.content.clone();
    exhausted_loot_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-2")
        .expect("fixture should contain the vault floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .loot_placements = 1;
    assert!(matches!(
        validate_and_normalize(&mut exhausted_loot_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut incomplete_spatial_budget = artifact.content.clone();
    incomplete_spatial_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-8")
        .expect("fixture should contain the spatial Vault floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .vault_area_tiles = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_spatial_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut incomplete_group_budget = artifact.content.clone();
    incomplete_group_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-6")
        .expect("fixture should contain the dynamic group floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .group_actor_slots = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_group_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut undersized_group_budget = artifact.content.clone();
    undersized_group_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-6")
        .expect("fixture should contain the dynamic group floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .group_actor_slots = Some(1);
    assert!(matches!(
        validate_and_normalize(&mut undersized_group_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut player_escort = artifact.content.clone();
    player_escort
        .encounter_tables
        .iter_mut()
        .find(|table| table.id == "demo.encounter-table.resonance-formations")
        .expect("fixture should contain the formation encounter table")
        .entries
        .iter_mut()
        .find_map(|entry| entry.group.as_mut())
        .and_then(|group| group.escort.as_mut())
        .expect("fixture should contain an escort table")
        .entries[0]
        .actor_kind_id = "demo.actor.explorer".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut player_escort),
        Err(ContentError::WrongActorRole(_))
    ));

    let mut self_guarding_leader = artifact.content.clone();
    self_guarding_leader
        .encounter_tables
        .iter_mut()
        .find(|table| table.id == "demo.encounter-table.resonance-formations")
        .expect("fixture should contain the formation encounter table")
        .entries
        .iter_mut()
        .find_map(|entry| entry.group.as_mut())
        .expect("fixture should contain a dynamic group")
        .pack_ai
        .leader = MonsterPackBehavior::GuardLeader;
    assert!(matches!(
        validate_and_normalize(&mut self_guarding_leader),
        Err(ContentError::InvalidEncounterTable(_))
    ));

    let mut invalid_feature_terrain = artifact.content.clone();
    invalid_feature_terrain.terrain_feature_tables[0].entries[0].terrain_id =
        "demo.terrain.floor".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_feature_terrain),
        Err(ContentError::InvalidTerrainFeatureTable(_))
    ));

    let mut incomplete_feature_budget = artifact.content.clone();
    incomplete_feature_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-3")
        .expect("fixture should contain the feature-budget floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .feature_placements = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_feature_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut oversized_feature_budget = artifact.content.clone();
    oversized_feature_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-3")
        .expect("fixture should contain the feature-budget floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .feature_placements = Some(5);
    assert!(matches!(
        validate_and_normalize(&mut oversized_feature_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut incomplete_room_budget = artifact.content.clone();
    incomplete_room_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the room-budget floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .room_area_tiles = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_room_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut undersized_room_budget = artifact.content.clone();
    undersized_room_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the room-budget floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .room_area_tiles = Some(35);
    assert!(matches!(
        validate_and_normalize(&mut undersized_room_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut blocked_cavern = artifact.content.clone();
    blocked_cavern.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the cavern floor")
        .layout
        .as_mut()
        .expect("fixture should contain a layout")
        .cavern
        .as_mut()
        .expect("fixture should contain a cavern")
        .terrain_id = "demo.terrain.wall".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut blocked_cavern),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut incomplete_cavern_budget = artifact.content.clone();
    incomplete_cavern_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the cavern floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .cavern_area_tiles = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_cavern_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut incomplete_lake_budget = artifact.content.clone();
    incomplete_lake_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the lake floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .lake_deep_area_tiles = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_lake_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut walkable_deep_water = artifact.content.clone();
    walkable_deep_water
        .terrain
        .iter_mut()
        .find(|terrain| terrain.id == "demo.terrain.resonance-water-deep")
        .expect("fixture should contain deep water")
        .walkable = true;
    assert!(matches!(
        validate_and_normalize(&mut walkable_deep_water),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut incompatible_river = artifact.content.clone();
    incompatible_river.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the river floor")
        .layout
        .as_mut()
        .expect("fixture should contain a layout")
        .river
        .as_mut()
        .expect("fixture should contain a river")
        .shallow_terrain_id = "demo.terrain.floor".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut incompatible_river),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut mismatched_maze_budget = artifact.content.clone();
    mismatched_maze_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-9")
        .expect("fixture should contain the maze floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .maze_floor_tiles = Some(126);
    assert!(matches!(
        validate_and_normalize(&mut mismatched_maze_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut maze_with_rooms = artifact.content.clone();
    let room_geometry = maze_with_rooms.worlds[0]
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .and_then(|floor| floor.layout.as_ref())
        .and_then(|layout| layout.rooms.clone())
        .expect("fixture should contain room geometry");
    maze_with_rooms.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-9")
        .and_then(|floor| floor.layout.as_mut())
        .expect("fixture should contain the maze-only layout")
        .rooms = Some(room_geometry);
    assert!(matches!(
        validate_and_normalize(&mut maze_with_rooms),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut room_overlay_maze = artifact.content.clone();
    let final_floor = room_overlay_maze.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the rooms floor");
    final_floor
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .maze_floor_tiles = Some(127);
    final_floor
        .layout
        .as_mut()
        .expect("fixture should contain a layout")
        .maze = Some(ProceduralMazeDefinition {
        width: 15,
        height: 15,
    });
    assert!(matches!(
        validate_and_normalize(&mut room_overlay_maze),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut mismatched_pit_budget = artifact.content.clone();
    mismatched_pit_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the pit floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .pit_actor_slots = Some(24);
    assert!(matches!(
        validate_and_normalize(&mut mismatched_pit_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut dangling_pit_table = artifact.content.clone();
    dangling_pit_table.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the pit floor")
        .layout
        .as_mut()
        .and_then(|layout| layout.pit.as_mut())
        .expect("fixture should contain a pit")
        .encounter_table_id = "demo.encounter-table.missing".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut dangling_pit_table),
        Err(ContentError::DanglingReference { .. })
    ));

    let mut incomplete_destroyed_budget = artifact.content.clone();
    incomplete_destroyed_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the destroyed floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .destruction_centers = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_destroyed_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut walkable_streamer = artifact.content.clone();
    walkable_streamer
        .terrain
        .iter_mut()
        .find(|terrain| terrain.id == "demo.terrain.resonance-vein")
        .expect("fixture should contain the streamer terrain")
        .walkable = true;
    assert!(validate_and_normalize(&mut walkable_streamer).is_err());

    let mut duplicate_room_shape = artifact.content.clone();
    let shapes = &mut duplicate_room_shape.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the room-layout floor")
        .layout
        .as_mut()
        .expect("fixture should contain a layout")
        .rooms
        .as_mut()
        .expect("fixture should contain room geometry")
        .shapes;
    shapes[1].shape = shapes[0].shape;
    assert!(matches!(
        validate_and_normalize(&mut duplicate_room_shape),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn warrens_stair_ranges_match_floor_topology_and_stay_bounded() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut zero_up = artifact.content.clone();
    zero_up
        .worlds
        .iter_mut()
        .find(|world| world.id == "demo.world.warrens-journey")
        .and_then(|world| world.procedural_floors.first_mut())
        .and_then(|floor| floor.layout.as_mut())
        .and_then(|layout| layout.stairs.as_mut())
        .expect("Warrens first floor should retain stair ranges")
        .up
        .minimum = 0;
    assert!(matches!(
        validate_and_normalize(&mut zero_up),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut missing_down = artifact.content.clone();
    missing_down
        .worlds
        .iter_mut()
        .find(|world| world.id == "demo.world.warrens-journey")
        .and_then(|world| world.procedural_floors.first_mut())
        .and_then(|floor| floor.layout.as_mut())
        .and_then(|layout| layout.stairs.as_mut())
        .expect("Warrens first floor should retain stair ranges")
        .down = None;
    assert!(matches!(
        validate_and_normalize(&mut missing_down),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut final_down = artifact.content.clone();
    final_down
        .worlds
        .iter_mut()
        .find(|world| world.id == "demo.world.warrens-journey")
        .and_then(|world| world.procedural_floors.last_mut())
        .and_then(|floor| floor.layout.as_mut())
        .and_then(|layout| layout.stairs.as_mut())
        .expect("Warrens final floor should retain its up stair range")
        .down = Some(ProceduralCountRangeDefinition {
        minimum: 4,
        maximum: 5,
    });
    assert!(matches!(
        validate_and_normalize(&mut final_down),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn region_tables_require_depth_eligible_candidates_and_composable_budgets() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    fn regional_floor(content: &mut CompiledContentV1) -> &mut ProceduralFloorDefinition {
        content.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-2")
            .expect("fixture should contain the regional floor")
    }

    let mut exhausted_depth = artifact.content.clone();
    regional_floor(&mut exhausted_depth).depth = 11;
    assert!(matches!(
        validate_and_normalize(&mut exhausted_depth),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut missing_budget = artifact.content.clone();
    regional_floor(&mut missing_budget)
        .generation_budget
        .as_mut()
        .expect("regional floor should retain a generation budget")
        .region_placements = None;
    assert!(matches!(
        validate_and_normalize(&mut missing_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut oversized_budget = artifact.content.clone();
    regional_floor(&mut oversized_budget)
        .generation_budget
        .as_mut()
        .expect("regional floor should retain a generation budget")
        .region_placements = Some(3);
    assert!(matches!(
        validate_and_normalize(&mut oversized_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut mixed_floor_tables = artifact.content.clone();
    regional_floor(&mut mixed_floor_tables).encounter_table_id =
        Some("demo.encounter-table.resonance-descent".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut mixed_floor_tables),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut composable_features = artifact.content.clone();
    composable_features.terrain_feature_tables[0].entries[0].min_depth = 2;
    let floor = regional_floor(&mut composable_features);
    floor.terrain_feature_table_id =
        Some("demo.terrain-feature-table.resonance-hazards".to_owned());
    floor
        .generation_budget
        .as_mut()
        .expect("regional floor should retain a generation budget")
        .feature_placements = Some(1);
    validate_and_normalize(&mut composable_features)
        .expect("regional feature, theme, vault, and connections should compose");

    let mut missing_theme = artifact.content.clone();
    missing_theme.region_tables[0].entries[0].theme_id = "demo.theme.resonance-missing".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut missing_theme),
        Err(ContentError::InvalidRegionTable(_))
    ));

    let mut incomplete_group_budget = artifact.content.clone();
    let budget = incomplete_group_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-6")
        .and_then(|floor| floor.generation_budget.as_mut())
        .expect("fixture should contain the regional group budget");
    budget.group_actor_slots = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_group_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut exhausted_special_actor_budget = artifact.content.clone();
    exhausted_special_actor_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .and_then(|floor| floor.generation_budget.as_mut())
        .expect("fixture should contain the regional pit budget")
        .actor_slots = 27;
    assert!(matches!(
        validate_and_normalize(&mut exhausted_special_actor_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut pit_consumes_too_many_rooms = artifact.content.clone();
    pit_consumes_too_many_rooms.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .and_then(|floor| floor.generation_budget.as_mut())
        .expect("fixture should contain the regional pit budget")
        .room_placements = Some(2);
    assert!(matches!(
        validate_and_normalize(&mut pit_consumes_too_many_rooms),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn vaults_require_walkable_unique_positions_and_depth_eligible_encounters() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut blocked_member = artifact.content.clone();
    blocked_member.vaults[0].encounter_groups[0].member_positions[0] =
        ContentPosition { x: 0, y: 0 };
    assert!(matches!(
        validate_and_normalize(&mut blocked_member),
        Err(ContentError::InvalidVault(_))
    ));

    let mut duplicate_transform = artifact.content.clone();
    let transform = duplicate_transform.vaults[0]
        .transforms
        .first()
        .copied()
        .unwrap_or(VaultTransform::Identity);
    duplicate_transform.vaults[0].transforms = vec![transform, transform];
    assert!(matches!(
        validate_and_normalize(&mut duplicate_transform),
        Err(ContentError::InvalidVault(_))
    ));

    let mut interior_entrance = artifact.content.clone();
    let vault = interior_entrance
        .vaults
        .iter_mut()
        .find(|vault| vault.width >= 4 && vault.height >= 4)
        .expect("fixture should contain a large Vault");
    vault.entrance_positions = vec![ContentPosition { x: 1, y: 1 }];
    assert!(matches!(
        validate_and_normalize(&mut interior_entrance),
        Err(ContentError::InvalidVault(_))
    ));

    let mut duplicate_entrance = artifact.content.clone();
    let entrance = duplicate_entrance.vaults[0].entrance_positions[0];
    duplicate_entrance.vaults[0].entrance_positions = vec![entrance, entrance];
    assert!(matches!(
        validate_and_normalize(&mut duplicate_entrance),
        Err(ContentError::InvalidVault(_))
    ));

    let mut disconnected_interior = artifact.content.clone();
    let vault = disconnected_interior
        .vaults
        .iter_mut()
        .find(|vault| vault.id == "demo.vault.harmonic-sepulcher")
        .expect("fixture should contain the sepulcher Vault");
    vault
        .terrain_overrides
        .iter_mut()
        .find(|terrain| terrain.terrain_id == "demo.terrain.wall")
        .expect("fixture should contain Vault walls")
        .positions
        .extend((1..5).map(|x| ContentPosition { x, y: 2 }));
    assert!(matches!(
        validate_and_normalize(&mut disconnected_interior),
        Err(ContentError::InvalidVault(_))
    ));

    let mut legacy_entrance = artifact.content.clone();
    let entrance = legacy_entrance.vaults[0].entrance_positions[0];
    legacy_entrance.vaults[0].entrance_positions.clear();
    legacy_entrance.vaults[0].entrance_position = Some(entrance);
    validate_and_normalize(&mut legacy_entrance)
        .expect("legacy single Vault entrance should normalize");
    assert_eq!(legacy_entrance.vaults[0].entrance_position, None);
    assert_eq!(legacy_entrance.vaults[0].entrance_positions, [entrance]);

    let mut theme_mismatch = artifact.content.clone();
    theme_mismatch.vaults[0].theme_id = "demo.theme.other".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut theme_mismatch),
        Err(ContentError::InvalidThemeTable(_))
    ));

    let mut no_depth_candidate = artifact.content.clone();
    for entry in &mut no_depth_candidate.vaults[0].encounter_groups[0].entries {
        entry.min_depth = 1;
        entry.max_depth = 1;
    }
    assert!(matches!(
        validate_and_normalize(&mut no_depth_candidate),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn staged_tasks_require_ordered_member_floor_objectives() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut outside_member = artifact.content.clone();
    outside_member.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-chain-rift")
        .expect("fixture should contain the staged task")
        .task_stages[1]
        .floor_id = Some("demo.floor.echo-bounty-rift".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut outside_member),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut duplicate_action_floor = artifact.content.clone();
    duplicate_action_floor.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-chain-rift")
        .expect("fixture should contain the staged task")
        .task_stages[2]
        .floor_id = Some("demo.floor.echo-chain-rift".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut duplicate_action_floor),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut non_retakeable = artifact.content.clone();
    for floor in non_retakeable.worlds[0]
        .procedural_floors
        .iter_mut()
        .filter(|floor| floor.task_id.as_deref() == Some("demo.task.echo-chain"))
    {
        floor.retakeable = false;
    }
    assert!(matches!(
        validate_and_normalize(&mut non_retakeable),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut zero_limit = artifact.content.clone();
    zero_limit.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-bounty-rift")
        .expect("fixture should contain the retakeable bounty")
        .max_retakes = Some(0);
    assert!(matches!(
        validate_and_normalize(&mut zero_limit),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut mismatched_policy = artifact.content.clone();
    mismatched_policy.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-bounty-annex-rift")
        .expect("fixture should contain the shared bounty member")
        .retake_floor_policy = RetakeFloorPolicy::PreserveFloor;
    assert!(matches!(
        validate_and_normalize(&mut mismatched_policy),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn dungeon_trees_require_shared_guardian_mirrors() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut missing_guardian = artifact.content.clone();
    missing_guardian.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-3")
        .expect("fixture should contain the final floor")
        .guardian = None;
    assert!(matches!(
        validate_and_normalize(&mut missing_guardian),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut broken_chain = artifact.content.clone();
    broken_chain.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-3")
        .expect("fixture should contain the final floor")
        .dungeon_id = Some("demo.dungeon.other".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut broken_chain),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut final_with_descent = artifact.content.clone();
    let final_floor = final_with_descent.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-3")
        .expect("fixture should contain the final floor");
    final_floor.next_floor_id = Some("demo.floor.echo-depth-1".to_owned());
    final_floor.down_stair_terrain_id = Some("demo.terrain.stairs-down".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut final_with_descent),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut mismatched_guardian = artifact.content.clone();
    mismatched_guardian.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-3-mirror")
        .expect("fixture should contain a guardian mirror")
        .guardian
        .as_mut()
        .expect("mirror should retain a guardian")
        .actor_kind_id = "demo.actor.echo-hound".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut mismatched_guardian),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut converging_tree = artifact.content.clone();
    let child_parent = converging_tree.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-2-mirror")
        .expect("fixture should contain the mirror branch");
    child_parent
        .connections
        .push(ProceduralFloorConnectionDefinition {
            id: "demo.connection.test.second-parent-down".to_owned(),
            kind: FloorConnectionKind::Stairs,
            terrain_id: "demo.terrain.stairs-down".to_owned(),
            target_floor_id: "demo.floor.echo-depth-3-mirror".to_owned(),
            target_connection_id: Some("demo.connection.test.second-parent-up".to_owned()),
            target_candidates: Vec::new(),
        });
    let child = converging_tree.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-3-mirror")
        .expect("fixture should contain the existing mirror final");
    child.connections.push(ProceduralFloorConnectionDefinition {
        id: "demo.connection.test.second-parent-up".to_owned(),
        kind: FloorConnectionKind::Stairs,
        terrain_id: "demo.terrain.stairs-up".to_owned(),
        target_floor_id: "demo.floor.echo-depth-2-mirror".to_owned(),
        target_connection_id: Some("demo.connection.test.second-parent-down".to_owned()),
        target_candidates: Vec::new(),
    });
    assert!(matches!(
        validate_and_normalize(&mut converging_tree),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn dungeon_entrance_guardians_and_entry_requirements_are_validated() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let world = &artifact.content.worlds[0];
    let resonance = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.resonance-descent")
        .expect("demo should contain the resonance dungeon");
    let entrance = resonance
        .entrance_guardian
        .as_ref()
        .expect("resonance should declare an entrance guardian");
    assert_eq!(entrance.position, ContentPosition { x: 2, y: 1 });
    assert!(resonance.entry_requirements.is_empty());

    let mut zero_ttl = artifact.content.clone();
    zero_ttl.worlds[0]
        .dungeons
        .iter_mut()
        .find(|dungeon| dungeon.id == "demo.dungeon.archive-depths")
        .expect("archive dungeon should remain available")
        .instance_lifecycle = DungeonInstanceLifecycle::TurnTtl { ttl_turns: 0 };
    assert!(matches!(
        validate_and_normalize(&mut zero_ttl),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut blocked_guardian = artifact.content.clone();
    blocked_guardian.worlds[0]
        .dungeons
        .iter_mut()
        .find(|dungeon| dungeon.id == resonance.id)
        .expect("resonance should remain available")
        .entrance_guardian
        .as_mut()
        .expect("entrance guardian should remain available")
        .position = ContentPosition { x: 3, y: 2 };
    assert!(matches!(
        validate_and_normalize(&mut blocked_guardian),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut duplicate_requirement = artifact.content.clone();
    let dungeon = duplicate_requirement.worlds[0]
        .dungeons
        .iter_mut()
        .find(|dungeon| dungeon.id == "demo.dungeon.echo-depths")
        .expect("echo dungeon should remain available");
    let requirement = DungeonEntryRequirementDefinition::CarriedItem {
        item_kind_id: "demo.item.luminous-shard".to_owned(),
        quantity: 1,
    };
    dungeon.entry_requirements = vec![requirement.clone(), requirement];
    assert!(matches!(
        validate_and_normalize(&mut duplicate_requirement),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut dangling_requirement = artifact.content.clone();
    dangling_requirement.worlds[0]
        .dungeons
        .iter_mut()
        .find(|dungeon| dungeon.id == "demo.dungeon.echo-depths")
        .expect("echo dungeon should remain available")
        .entry_requirements = vec![DungeonEntryRequirementDefinition::TaskStatus {
        task_id: "demo.task.missing".to_owned(),
        status: DungeonEntryTaskStatus::Completed,
    }];
    assert!(matches!(
        validate_and_normalize(&mut dangling_requirement),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn floor_connections_require_reciprocal_targets_and_matching_terrain_roles() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut broken_pair = artifact.content.clone();
    broken_pair.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-1")
        .expect("fixture should contain echo depth one")
        .connections
        .iter_mut()
        .find(|connection| connection.id == "demo.connection.echo-depth-1.down-a")
        .expect("fixture should contain the first downward connection")
        .target_connection_id = Some("demo.connection.echo-depth-2.up-b".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut broken_pair),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut wrong_shaft_kind = artifact.content.clone();
    wrong_shaft_kind.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-1")
        .expect("fixture should contain echo depth one")
        .connections
        .iter_mut()
        .find(|connection| connection.id == "demo.connection.echo-depth-1.shaft-down")
        .expect("fixture should contain the downward shaft")
        .kind = FloorConnectionKind::Stairs;
    assert!(matches!(
        validate_and_normalize(&mut wrong_shaft_kind),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut missing_entry = artifact.content.clone();
    missing_entry.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-1")
        .expect("fixture should contain echo depth one")
        .entry_connection_id = None;
    assert!(matches!(
        validate_and_normalize(&mut missing_entry),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn door_terrain_transitions_are_reciprocal_and_match_collision() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut missing_reciprocal = artifact.content.clone();
    missing_reciprocal
        .terrain
        .iter_mut()
        .find(|terrain| terrain.id == "demo.terrain.door-closed")
        .expect("fixture should contain the closed door")
        .open_to_terrain_id = None;
    assert!(matches!(
        validate_and_normalize(&mut missing_reciprocal),
        Err(ContentError::InvalidTerrainTransition(_))
    ));

    let mut blocked_open_door = artifact.content.clone();
    blocked_open_door
        .terrain
        .iter_mut()
        .find(|terrain| terrain.id == "demo.terrain.door-open")
        .expect("fixture should contain the open door")
        .walkable = false;
    assert!(matches!(
        validate_and_normalize(&mut blocked_open_door),
        Err(ContentError::InvalidTerrainTransition(_))
    ));

    let mut incomplete_bash = artifact.content.clone();
    incomplete_bash
        .terrain
        .iter_mut()
        .find(|terrain| terrain.id == "demo.terrain.door-locked")
        .expect("fixture should contain the locked door")
        .bash_check_difficulty = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_bash),
        Err(ContentError::InvalidTerrainTransition(_))
    ));

    let mut invalid_lock = artifact.content.clone();
    invalid_lock
        .terrain
        .iter_mut()
        .find(|terrain| terrain.id == "demo.terrain.door-locked")
        .expect("fixture should contain the locked door")
        .open_check_difficulty = Some(0);
    assert!(matches!(
        validate_and_normalize(&mut invalid_lock),
        Err(ContentError::InvalidTerrainTransition(_))
    ));

    let mut incomplete_concealment = artifact.content.clone();
    incomplete_concealment
        .terrain
        .iter_mut()
        .find(|terrain| terrain.id == "demo.terrain.door-secret")
        .expect("fixture should contain the secret door")
        .search_check_difficulty = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_concealment),
        Err(ContentError::InvalidTerrainTransition(_))
    ));

    let mut non_door_generator = artifact.content.clone();
    non_door_generator.worlds[0].procedural_floors[0].closed_door_terrain_id =
        "demo.terrain.wall".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut non_door_generator),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn terrain_glyphs_do_not_consume_letters_reserved_for_actors() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    assert!(artifact.content.terrain.iter().all(|terrain| {
        !terrain
            .glyph
            .chars()
            .next()
            .is_some_and(|glyph| glyph.is_ascii_alphabetic())
    }));

    let mut letter_terrain = artifact.content.clone();
    letter_terrain.terrain[0].glyph = "T".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut letter_terrain),
        Err(ContentError::InvalidTerrainGlyph(_))
    ));
}
