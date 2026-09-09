// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use crate::game::{hunger, lighting};
use std::sync::OnceLock;

const SPECTRE: &str = "rfb-legacy.race.spectre";
const HUMAN: &str = "demo.race.rfb-human";
const STAFF: &str = "demo.item.staff-of-nothing";

fn birth(seed: u64, build: &str) -> Game {
    static CONTENT: OnceLock<Arc<rfb_content::ContentCatalog>> = OnceLock::new();
    let content = CONTENT.get_or_init(|| {
        let root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
        let mut artifact = rfb_content::compile_pack_dir(&root).unwrap();
        // Exercise the real initialization pipeline before step five opens creation.
        artifact
            .content
            .races
            .iter_mut()
            .find(|race| race.id == SPECTRE)
            .unwrap()
            .tags
            .push("rfb-compatibility".to_owned());
        Arc::new(rfb_content::ContentCatalog::from_artifact(
            rfb_content::encode_content(artifact.content).unwrap(),
        ))
    });
    Game::from_content_internal(
        seed,
        content.clone(),
        DEFAULT_WORLD_ID,
        Some(build),
        Some(SPECTRE),
        Game::DEFAULT_PLAYER_NAME,
    )
    .unwrap()
}

fn form(game: &mut Game, race: &str) {
    let mut status =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 100_000, "test.spectre").status;
    status.granted_race_id = Some(race.to_owned());
    game.player.statuses.push(status);
    game.refresh_player_resource_maxima();
}

fn use_food(game: &mut Game, kind: &str) {
    give_inventory_item(game, "test.spectre.food", kind);
    dispatch_next(
        game,
        GameCommand::UseItem {
            item_id: "test.spectre.food".to_owned(),
            target: None,
        },
    );
    assert!(game.items.iter().all(|item| item.id != "test.spectre.food"));
}

#[test]
fn spectre_night_birth_preserves_all_six_class_kits_and_supplies_full_staff_and_light() {
    for build_id in [
        "demo.build.warrior",
        "demo.build.archer",
        "demo.build.high-mage-death",
        "demo.build.paladin-death",
        "demo.build.cavalry",
        "demo.build.sniper",
    ] {
        let game = birth(83, build_id);
        assert_eq!(game.world_tick, wilderness::WILDERNESS_NIGHT_START_TICK);
        assert!(!game.wilderness_is_daytime());
        let carried: Vec<_> = game
            .items
            .iter()
            .filter(|item| !matches!(item.location, ItemLocation::Ground(_)))
            .collect();
        assert!(
            carried
                .iter()
                .all(|item| item.kind_id != hunger::RATION_ITEM_KIND_ID)
        );
        let staves: Vec<_> = carried
            .iter()
            .filter(|item| item.kind_id == STAFF)
            .collect();
        assert_eq!(staves.len(), 1);
        assert_eq!(staves[0].quantity, 1);
        assert_eq!(staves[0].location, ItemLocation::Inventory);
        assert_eq!(
            staves[0].charges,
            Some(ItemChargesDto {
                current: 21,
                maximum: 21
            })
        );
        let torches: Vec<_> = carried
            .iter()
            .filter(|item| item.kind_id == lighting::WOODEN_TORCH_ITEM_KIND_ID)
            .collect();
        assert!((3..=7).contains(&torches.len()));
        assert!(torches.iter().all(|item| item.quantity == 1
            && item.location == ItemLocation::Inventory
            && item.fuel == torches[0].fuel));
        let fuel = torches[0].fuel.unwrap().current;
        assert!((1500..=3500).contains(&fuel) && fuel % 500 == 0);
        let (build, _, class, personality) = game.character_definitions().unwrap();
        let kit = class
            .starting_items
            .iter()
            .chain(&personality.starting_items)
            .chain(&build.starting_items);
        assert_eq!(carried.len(), 1 + torches.len() + kit.clone().count());
        for expected in kit {
            let items: Vec<_> = carried
                .iter()
                .filter(|item| item.kind_id == expected.item_kind_id)
                .collect();
            assert_eq!(items.len(), 1, "{build_id}: {}", expected.item_kind_id);
            assert!(
                (expected.quantity..=expected.maximum_quantity.unwrap_or(expected.quantity))
                    .contains(&items[0].quantity)
            );
            assert_eq!(
                matches!(items[0].location, ItemLocation::Equipped { .. }),
                expected.equipped
            );
        }
        let restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
        assert_eq!(restored.snapshot(), game.snapshot());
    }
    assert!(
        Game::new_with_build_race_and_name(
            83,
            "demo.build.warrior",
            SPECTRE,
            Game::DEFAULT_PLAYER_NAME
        )
        .is_err()
    );
}

#[test]
fn spectre_absorbs_birth_staff_and_partial_floor_device_then_pays_for_empty_attempt() {
    let mut game = birth(83, "demo.build.warrior");
    clear_monsters(&mut game);
    game.nutrition = 1000;
    let staff_id = game
        .items
        .iter()
        .find(|item| item.kind_id == STAFF)
        .unwrap()
        .id
        .clone();
    assert!(
        game.snapshot()
            .inventory
            .iter()
            .find(|item| item.id == staff_id)
            .unwrap()
            .absorbable
    );
    let before_tick = game.world_tick;
    let update = dispatch_next(
        &mut game,
        GameCommand::AbsorbDevice {
            item_id: staff_id.clone(),
        },
    );
    assert_eq!(game.nutrition, 6000);
    assert_eq!(game.world_tick, before_tick + 10);
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == staff_id)
            .unwrap()
            .charges
            .unwrap()
            .current,
        20
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.device-absorbed")
    );

    give_inventory_item(&mut game, "test.spectre.rod", "demo.item.detection-rod");
    let position = game.player.position;
    let rod = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.spectre.rod")
        .unwrap();
    assert!(rod.activation.as_ref().unwrap().cost > 2);
    rod.charges.as_mut().unwrap().current = 2;
    rod.location = ItemLocation::Ground(position);
    game.item_property_knowledge
        .entry("test.spectre.rod".to_owned())
        .or_default()
        .discovered = true;
    assert!(
        game.snapshot()
            .items
            .iter()
            .find(|item| item.id == "test.spectre.rod")
            .unwrap()
            .absorbable
    );
    dispatch_next(
        &mut game,
        GameCommand::AbsorbDevice {
            item_id: "test.spectre.rod".to_owned(),
        },
    );
    assert_eq!(game.nutrition, 11000);
    let rod = game
        .items
        .iter()
        .find(|item| item.id == "test.spectre.rod")
        .unwrap();
    assert_eq!(rod.quantity, 1);
    assert_eq!(rod.charges.unwrap().current, 0);
    let before_tick = game.world_tick;
    let update = dispatch_next(
        &mut game,
        GameCommand::AbsorbDevice {
            item_id: "test.spectre.rod".to_owned(),
        },
    );
    assert_eq!(game.world_tick, before_tick + 10);
    assert_eq!(game.nutrition, 11000);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.message_key == "item-device-empty")
    );

    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    let command = GameCommand::AbsorbDevice { item_id: staff_id };
    assert_eq!(
        dispatch_next(&mut restored, command.clone()).events,
        dispatch_next(&mut game, command).events
    );
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn spectre_food_uses_current_form_divisor_without_losing_waybread_and_mushroom_effects() {
    for temporary in [false, true] {
        for (kind, gain) in [
            (hunger::RATION_ITEM_KIND_ID, 250),
            ("demo.item.piece-of-elvish-waybread", 375),
            ("demo.item.fast-recovery-mushroom", 25),
        ] {
            let mut game = birth(83, "demo.build.warrior");
            clear_monsters(&mut game);
            if temporary {
                game.build.as_mut().unwrap().race_id = HUMAN.to_owned();
                form(&mut game, SPECTRE);
            }
            game.nutrition = 5000;
            game.player.hp = 1;
            if kind.contains("waybread") {
                game.player
                    .statuses
                    .push(monster_combat::melee_status(STATUS_POISON, 100, "test.food").status);
            }
            use_food(&mut game, kind);
            assert_eq!(game.nutrition, 5000 + gain, "{kind}, temporary={temporary}");
            if kind != hunger::RATION_ITEM_KIND_ID {
                assert!(game.player.hp > 1);
            }
            if kind.contains("waybread") {
                assert!(!game.player_has_status_kind(STATUS_POISON));
            }
            if kind.contains("mushroom") {
                assert!(game.player_has_status_kind(STATUS_REGENERATION));
            }
        }
    }
    let mut game = birth(83, "demo.build.warrior");
    clear_monsters(&mut game);
    form(&mut game, HUMAN);
    game.nutrition = 5000;
    use_food(&mut game, hunger::RATION_ITEM_KIND_ID);
    assert_eq!(game.nutrition, 10000);
}

#[test]
fn spectre_potion_nutrition_uses_native_untransformed_body_and_keeps_main_effects() {
    for (kind, native_gain, transformed_gain) in [
        ("water-potion", 10, 200),
        ("light-healing-potion", 2, 50),
        ("invulnerability-potion", -125, -2500),
    ] {
        for variant in 0..4 {
            let mut game = birth(83, "demo.build.warrior");
            clear_monsters(&mut game);
            match variant {
                1 => {
                    game.build.as_mut().unwrap().race_id = HUMAN.to_owned();
                    form(&mut game, SPECTRE);
                }
                2 => form(&mut game, HUMAN),
                3 => form(&mut game, SPECTRE),
                _ => {}
            }
            game.nutrition = 5000;
            game.player.hp = 1;
            use_food(&mut game, &format!("demo.item.{kind}"));
            assert_eq!(
                i32::from(game.nutrition),
                5000 + if variant == 0 {
                    native_gain
                } else {
                    transformed_gain
                },
                "{kind}, variant={variant}"
            );
            if kind == "light-healing-potion" {
                assert!(game.player.hp > 1);
            }
            if kind == "invulnerability-potion" {
                assert!(game.player_has_status_kind(STATUS_INVULNERABILITY));
            }
        }
    }
}

#[test]
fn spectre_device_absorption_follows_current_form_and_rejects_remote_items() {
    let mut game = Game::new_with_build(83, "demo.build.warrior").unwrap();
    clear_monsters(&mut game);
    give_inventory_item(&mut game, "test.spectre.device", STAFF);
    assert!(
        !game.item_can_be_absorbed(
            game.items
                .iter()
                .find(|item| item.id == "test.spectre.device")
                .unwrap()
        )
    );
    let birth_tick = game.world_tick;
    let before_items = game.items.clone();
    form(&mut game, SPECTRE);
    assert_eq!(game.world_tick, birth_tick);
    assert_eq!(game.items, before_items);
    game.nutrition = 1000;
    dispatch_next(
        &mut game,
        GameCommand::AbsorbDevice {
            item_id: "test.spectre.device".to_owned(),
        },
    );
    assert_eq!(game.nutrition, 6000);
    let index = game
        .items
        .iter()
        .position(|item| item.id == "test.spectre.device")
        .unwrap();
    game.items[index].location = ItemLocation::Ground(Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    });
    assert!(!game.item_can_be_absorbed(&game.items[index]));
    game.items[index].location = ItemLocation::Inventory;
    game.player.statuses.clear();
    assert!(!game.item_can_be_absorbed(&game.items[index]));
}

#[test]
fn spectre_can_buy_projected_magic_shop_device_and_absorb_its_energy() {
    let mut game = birth(83, "demo.build.warrior");
    clear_monsters(&mut game);
    game.player.position = Position { x: 57, y: 13 };
    game.gold = 100_000;
    game.mark_shop_visited_at_player().unwrap();
    let snapshot = game.snapshot();
    let shop = snapshot
        .shops
        .iter()
        .find(|shop| shop.id == "demo.shop.outpost-magic-shop")
        .unwrap();
    // This case needs a device; use the first projected one, with no generated ID fixture.
    let stock = shop
        .stock
        .iter()
        .find(|item| {
            game.content
                .item(&item.kind_id)
                .unwrap()
                .tags
                .iter()
                .any(|tag| tag == "device")
        })
        .unwrap()
        .clone();
    let old_ids: BTreeSet<_> = game.items.iter().map(|item| item.id.clone()).collect();
    let purchase = dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: shop.id.clone(),
            item_id: stock.id,
            quantity: 1,
        },
    );
    assert!(
        purchase
            .events
            .iter()
            .any(|event| event.kind == "shop.purchase")
    );
    assert!(game.gold < 100_000);
    let item = game
        .items
        .iter()
        .find(|item| !old_ids.contains(&item.id) && item.location == ItemLocation::Inventory)
        .unwrap();
    let item_id = item.id.clone();
    let before = item.charges.unwrap().current;
    assert!(before > 0);
    assert!(
        game.snapshot()
            .inventory
            .iter()
            .find(|item| item.id == item_id)
            .unwrap()
            .absorbable
    );
    game.nutrition = 1000;
    let update = dispatch_next(
        &mut game,
        GameCommand::AbsorbDevice {
            item_id: item_id.clone(),
        },
    );
    assert_eq!(game.nutrition, 6000);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.device-absorbed")
    );
    let item = game.items.iter().find(|item| item.id == item_id).unwrap();
    assert_eq!(item.quantity, 1);
    assert!(item.charges.unwrap().current < before);
}
