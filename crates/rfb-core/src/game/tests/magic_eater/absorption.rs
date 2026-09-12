// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::tests::support::{command, dispatch_next};
use rfb_protocol::AbsorbedDeviceCategoryDto as Category;

const ABSORB: &str = "demo.ability.magic-eater-absorb-magic";

fn begin(item_id: &str) -> GameCommand {
    GameCommand::CastAbility {
        ability_id: ABSORB.to_owned(),
        target: TargetSelection::Item {
            item_id: item_id.to_owned(),
        },
    }
}

fn resolve(confirm: bool, inherit_inscription: bool) -> GameCommand {
    GameCommand::ResolveMagicAbsorption {
        confirm,
        inherit_inscription,
    }
}

pub(super) fn absorb(game: &mut Game, id: &str, slot: u8) {
    dispatch_next(game, begin(id));
    dispatch_next(game, GameCommand::SelectMagicAbsorptionSlot { slot });
}

fn item(game: &Game, id: &str) -> ItemInstance {
    game.items
        .iter()
        .find(|item| item.id == id)
        .unwrap()
        .clone()
}

#[test]
fn selection_is_free_cancellable_and_saved_without_identifying_or_spending_rng() {
    let mut game = at_level(1);
    give_inventory_item(&mut game, "test.absorb", "demo.item.magic-missile-wand");
    game.items.reverse();
    game.reveal_current_visibility();
    let source = item(&game, "test.absorb");
    let knowledge = game.item_property_knowledge.clone();
    let before = (
        game.turn,
        game.world_tick,
        game.player.energy_need,
        game.rng.clone(),
    );
    let projected = game.snapshot().player;
    let slots = projected.magic_eater.unwrap().slots;
    assert_eq!(slots.len(), 30);
    assert!(slots.iter().all(|slot| slot.item.is_none()));
    let power = projected
        .abilities
        .iter()
        .find(|ability| ability.id == ABSORB)
        .unwrap();
    assert_eq!(power.source, AbilitySourceDto::Class);
    assert!(
        power
            .item_targets
            .as_ref()
            .unwrap()
            .iter()
            .any(|target| target.item_id == source.id)
    );
    dispatch_next(&mut game, begin(&source.id));
    assert_eq!(item(&game, &source.id), source);
    assert_eq!(game.item_property_knowledge, knowledge);
    assert_eq!(
        (
            game.turn,
            game.world_tick,
            game.player.energy_need,
            game.rng.clone()
        ),
        before
    );
    let saved = game.to_save();
    let mut restored = Game::from_save(saved.clone()).unwrap();
    for invalid in [
        GameCommand::Wait,
        GameCommand::SelectMagicAbsorptionSlot { slot: 10 },
        resolve(true, false),
    ] {
        assert!(matches!(
            game.dispatch(command(game.last_command_seq + 1, game.revision, invalid)),
            Err(CoreError::MagicAbsorptionUnavailable(_))
        ));
        assert_eq!(game.to_save(), saved);
    }
    assert_eq!(
        dispatch_next(&mut game, resolve(false, false)),
        dispatch_next(&mut restored, resolve(false, false))
    );
    assert!(game.pending_magic_absorption.is_none());
    assert_eq!(item(&game, &source.id), source);
    assert_eq!(game.item_property_knowledge, knowledge);
    assert_eq!(
        (
            game.turn,
            game.world_tick,
            game.player.energy_need,
            game.rng.clone()
        ),
        before
    );
    assert_eq!(game.to_save(), restored.to_save());
}

#[test]
fn absorbs_pack_and_floor_instances_with_zero_sp_and_retains_all_instance_fields() {
    for at_feet in [false, true] {
        let mut game = at_level(1);
        give_inventory_item(&mut game, "test.absorb", "demo.item.magic-missile-wand");
        let ego = game
            .content
            .affix_definitions()
            .find(|affix| {
                affix
                    .rfb_ego
                    .as_ref()
                    .is_some_and(|ego| ego.source_index == 251)
            })
            .unwrap();
        let materialized = crate::game::ego::materialize_device(
            &game.content,
            &mut game.rng,
            game.content.item("demo.item.magic-missile-wand").unwrap(),
            50,
            true,
            crate::game::loot::ItemGenerationMode::Ordinary,
            Some(ego),
        )
        .unwrap();
        let base_device_skill = game.player_derived_stats().device_skill.value;
        let position = game.player.position;
        let source = game
            .items
            .iter_mut()
            .find(|item| item.id == "test.absorb")
            .unwrap();
        materialized.apply_to(source);
        source.quality = ItemQualityDto::Exceptional;
        source.intrinsic_properties.equipment_bonuses.device_skill = -19;
        if at_feet {
            source.location = ItemLocation::Ground(position);
        }
        source.charges.as_mut().unwrap().current = 0;
        source.device_recovery_progress = 370;
        source.inscription = Some("@ma".to_owned());
        let mut expected = source.clone();
        expected.location = ItemLocation::Absorbed {
            category: Category::Wand,
            slot: 9,
        };
        let before_turn = game.turn;
        let before_world = game.world_tick;
        let update = {
            dispatch_next(&mut game, begin("test.absorb"));
            // The commit itself preserves SP/fraction; the ensuing world tick may recover it.
            let mut commit_only = game.clone();
            commit_only.select_magic_absorption_slot(9, &mut Vec::new());
            assert_eq!(item(&commit_only, "test.absorb"), expected);
            dispatch_next(
                &mut game,
                GameCommand::SelectMagicAbsorptionSlot { slot: 9 },
            )
        };
        assert!(
            update
                .events
                .iter()
                .any(|event| event.kind == "magic-eater.absorbed")
        );
        assert_eq!(game.turn, before_turn + 1);
        assert!(game.world_tick > before_world);
        expected.charges = item(&game, "test.absorb").charges;
        expected.device_recovery_progress = item(&game, "test.absorb").device_recovery_progress;
        assert_eq!(item(&game, "test.absorb"), expected);
        assert_eq!(
            game.player_derived_stats().device_skill.value,
            base_device_skill
        );
        assert!(!game.item_can_receive_recharge(&expected));
        assert!(!game.item_can_receive_player_recharge(&expected));
        assert!(!game.item_can_supply_recharge(&expected));
        assert!(!game.item_can_be_absorbed(&expected));
        assert!(
            game.drop_inventory_items(std::slice::from_ref(&expected.id))
                .is_none()
        );
        assert!(game.destroy_item(&expected.id, 1).is_err());
        assert_eq!(item(&game, "test.absorb"), expected);
        assert_eq!(
            game.item_identification(&expected),
            ItemIdentificationDto::Identified
        );
        assert!(
            game.snapshot()
                .inventory
                .iter()
                .all(|item| item.id != expected.id)
        );
        let restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(item(&restored, "test.absorb"), expected);
    }
}

#[test]
fn occupied_slot_needs_identity_bound_confirmation_and_optional_inscription_inheritance() {
    for inherit in [false, true] {
        let mut game = at_level(1);
        give_inventory_item(&mut game, "test.old", "demo.item.identify-staff");
        absorb(&mut game, "test.old", 0);
        dispatch_next(
            &mut game,
            GameCommand::InscribeItem {
                item_id: "test.old".to_owned(),
                inscription: Some("@mb".to_owned()),
            },
        );
        give_inventory_item(&mut game, "test.new", "demo.item.identify-staff");
        game.items
            .iter_mut()
            .find(|item| item.id == "test.new")
            .unwrap()
            .inscription = Some("@mc".to_owned());
        let before = (game.world_tick, game.rng.clone());
        absorb(&mut game, "test.new", 0);
        assert_eq!((game.world_tick, game.rng.clone()), before);
        let pending = game.pending_magic_absorption.as_ref().unwrap();
        assert_eq!(pending.replacement.as_ref().unwrap().item_id, "test.old");
        let saved = game.to_save();
        let mut cancelled = Game::from_save(saved.clone()).unwrap();
        dispatch_next(&mut cancelled, resolve(false, false));
        assert_eq!(item(&cancelled, "test.old"), item(&game, "test.old"));
        assert_eq!(item(&cancelled, "test.new"), item(&game, "test.new"));
        let mut restored = Game::from_save(saved).unwrap();
        assert_eq!(
            dispatch_next(&mut game, resolve(true, inherit)),
            dispatch_next(&mut restored, resolve(true, inherit))
        );
        assert!(!game.items.iter().any(|item| item.id == "test.old"));
        assert!(!game.item_property_knowledge.contains_key("test.old"));
        let absorbed = game.absorbed_device(Category::Staff, 0).unwrap();
        assert_eq!(absorbed.id, "test.new");
        assert_eq!(
            absorbed.inscription.as_deref(),
            Some(if inherit { "@mb" } else { "@mc" })
        );
        assert!(game.pending_magic_absorption.is_none());
        assert_eq!(game.to_save(), restored.to_save());
    }
}

#[test]
fn all_thirty_slots_swap_inscribe_and_round_trip_without_pack_capacity_or_weight() {
    let mut game = at_level(1);
    let pack_slots = game.inventory_used_slots();
    let carried_weight = game.carried_weight_tenths_pound();
    for (category, kind) in [
        (Category::Wand, "demo.item.magic-missile-wand"),
        (Category::Staff, "demo.item.identify-staff"),
        (Category::Rod, "demo.item.detection-rod"),
    ] {
        for slot in 0..10 {
            let id = format!("test.{}.{slot}", kind.rsplit('.').next().unwrap());
            give_inventory_item(&mut game, &id, kind);
            absorb(&mut game, &id, slot);
            assert_eq!(game.absorbed_device(category, slot).unwrap().id, id);
        }
    }
    assert_eq!(game.inventory_used_slots(), pack_slots);
    assert_eq!(game.carried_weight_tenths_pound(), carried_weight);
    assert_eq!(game.to_save().absorbed_devices.len(), 30);
    let ordinary = game.snapshot().player.magic_eater.unwrap().device_commands;
    assert_eq!(
        ordinary.iter().flat_map(|command| &command.items).count(),
        1,
        "only the unabsorbed birth wand remains in the ordinary command"
    );
    assert!(
        ordinary
            .iter()
            .flat_map(|command| &command.items)
            .all(|item| game
                .items
                .iter()
                .any(|instance| instance.id == item.id
                    && instance.location == ItemLocation::Inventory))
    );
    let before = (game.turn, game.world_tick, game.rng.clone());
    dispatch_next(
        &mut game,
        GameCommand::SwapAbsorbedDevices {
            category: Category::Rod,
            first_slot: 0,
            second_slot: 9,
        },
    );
    assert_eq!(
        game.absorbed_device(Category::Rod, 9).unwrap().id,
        "test.detection-rod.0"
    );
    dispatch_next(
        &mut game,
        GameCommand::InscribeItem {
            item_id: "test.detection-rod.0".to_owned(),
            inscription: Some("@mz".to_owned()),
        },
    );
    assert_eq!((game.turn, game.world_tick, game.rng.clone()), before);
    for (id, inscription) in [
        ("test.detection-rod.1", "@ma @zB"),
        ("test.detection-rod.2", "@ma @zR"),
        ("test.detection-rod.3", "@8"),
    ] {
        dispatch_next(
            &mut game,
            GameCommand::InscribeItem {
                item_id: id.to_owned(),
                inscription: Some(inscription.to_owned()),
            },
        );
    }
    let projection = game.snapshot().player.magic_eater.unwrap();
    let rods = projection
        .slots
        .iter()
        .filter(|slot| slot.category == Category::Rod)
        .collect::<Vec<_>>();
    assert_eq!(rods[2].use_label, "a");
    assert_ne!(rods[1].use_label, "a");
    assert_ne!(rods[0].use_label, "a");
    assert_eq!(rods[1].device_label, "B");
    assert_eq!(rods[2].device_label, "r");
    assert_eq!(rods[3].use_label, "8");
    assert_eq!(rods[3].device_label, "8");
    assert_eq!(
        rods.iter()
            .map(|slot| &slot.use_label)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        10
    );
    assert_eq!(
        rods.iter()
            .map(|slot| &slot.device_label)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        10
    );
    assert_eq!((game.turn, game.world_tick, game.rng.clone()), before);
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.snapshot().player.magic_eater, Some(projection));
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(
        dispatch_next(&mut game, GameCommand::Wait),
        dispatch_next(&mut restored, GameCommand::Wait)
    );
    assert_eq!(game.to_save(), restored.to_save());
    let body_before = game.to_save().absorbed_devices;
    let stairs = game
        .snapshot()
        .cells
        .iter()
        .find(|cell| cell.terrain_id == "demo.terrain.stairs-down")
        .unwrap()
        .position;
    game.player.position = stairs;
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, "demo.floor.warrens-depth-1");
    assert_eq!(game.to_save().absorbed_devices, body_before);
    assert!(game.stored_floors.values().all(|floor| {
        floor
            .items
            .iter()
            .all(|item| !matches!(item.location, ItemLocation::Absorbed { .. }))
    }));
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(game.state_hash(), restored.state_hash());
}

#[test]
fn absorbed_devices_cannot_use_inventory_commands_or_be_sold() {
    let mut game = at_level(1);
    give_inventory_item(&mut game, "test.body", "demo.item.magic-missile-wand");
    absorb(&mut game, "test.body", 0);
    let saved = game.to_save();
    assert!(
        game.dispatch(command(
            game.last_command_seq + 1,
            game.revision,
            GameCommand::UseItem {
                item_id: "test.body".to_owned(),
                target: None
            }
        ))
        .is_err()
    );
    assert_eq!(game.to_save(), saved);
    let shop = game.snapshot().shops.into_iter().next().unwrap();
    game.player.position = shop.entrance_position;
    assert!(matches!(
        game.sell_to_shop(&shop.id, "test.body", 1),
        Err("item-unavailable")
    ));
    assert_eq!(game.to_save().absorbed_devices, saved.absorbed_devices);
}

#[test]
fn empty_slots_and_mundane_devices_are_valid_but_other_equipment_is_not_absorbable() {
    let mut game = at_level(1);
    let before = (game.turn, game.world_tick, game.rng.clone());
    dispatch_next(
        &mut game,
        GameCommand::SwapAbsorbedDevices {
            category: Category::Staff,
            first_slot: 0,
            second_slot: 9,
        },
    );
    assert_eq!((game.turn, game.world_tick, game.rng.clone()), before);
    give_inventory_item(&mut game, "test.mundane", "demo.item.identify-staff");
    let source = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.mundane")
        .unwrap();
    source.origin_kind = Some(rfb_protocol::ItemOriginKindDto::Mundanity);
    source.activation = None;
    source.charges = None;
    absorb(&mut game, "test.mundane", 0);
    dispatch_next(
        &mut game,
        GameCommand::SwapAbsorbedDevices {
            category: Category::Staff,
            first_slot: 0,
            second_slot: 9,
        },
    );
    assert!(game.absorbed_device(Category::Staff, 0).is_none());
    assert_eq!(
        game.absorbed_device(Category::Staff, 9).unwrap().id,
        "test.mundane"
    );
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(game.state_hash(), restored.state_hash());
    let sword = game
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.short-sword")
        .unwrap()
        .id
        .clone();
    let before = (game.world_tick, game.rng.clone());
    dispatch_next(&mut game, begin(&sword));
    assert!(game.pending_magic_absorption.is_none());
    assert_eq!((game.world_tick, game.rng.clone()), before);
}

#[test]
fn corrupt_body_containers_and_stale_pending_targets_are_rejected() {
    let mut game = at_level(1);
    give_inventory_item(&mut game, "test.body", "demo.item.magic-missile-wand");
    absorb(&mut game, "test.body", 0);
    let saved = game.to_save();
    for corrupt in 0..9 {
        let mut invalid = saved.clone();
        match corrupt {
            0 => invalid.absorbed_devices[0].slot = 10,
            1 => invalid.absorbed_devices[0].category = Category::Rod,
            2 => {
                let mut duplicate = invalid.absorbed_devices[0].clone();
                duplicate.item.id = "test.other-body".to_owned();
                let mut knowledge = invalid
                    .item_property_knowledge
                    .iter()
                    .find(|knowledge| knowledge.item_id == "test.body")
                    .unwrap()
                    .clone();
                knowledge.item_id = duplicate.item.id.clone();
                invalid.item_property_knowledge.push(knowledge);
                invalid.absorbed_devices.push(duplicate);
            }
            3 => invalid
                .inventory
                .push(invalid.absorbed_devices[0].item.clone()),
            4 => invalid.absorbed_devices[0].item.quantity = 2,
            5 => invalid.absorbed_devices[0].item.device_recovery_progress = 1_000,
            6 => {
                invalid.absorbed_devices[0]
                    .item
                    .activation
                    .as_mut()
                    .unwrap()
                    .profile_id = "unknown.profile".to_owned()
            }
            7 => {
                invalid
                    .item_property_knowledge
                    .retain(|knowledge| knowledge.item_id != "test.body");
            }
            8 => {
                invalid.absorbed_devices[0]
                    .item
                    .charges
                    .as_mut()
                    .unwrap()
                    .current = u32::MAX
            }
            _ => unreachable!(),
        }
        assert!(Game::from_save(invalid).is_err(), "corruption {corrupt}");
    }
    let mut other_class = Game::new_with_build(925, "demo.build.warrior")
        .unwrap()
        .to_save();
    other_class.absorbed_devices = saved.absorbed_devices.clone();
    other_class.item_property_knowledge.extend(
        saved
            .item_property_knowledge
            .iter()
            .filter(|knowledge| knowledge.item_id == "test.body")
            .cloned(),
    );
    assert!(matches!(
        Game::from_save(other_class),
        Err(CoreError::InvalidSave("magic eater state is invalid"))
    ));
    give_inventory_item(&mut game, "test.next", "demo.item.magic-missile-wand");
    absorb(&mut game, "test.next", 0);
    game.pending_magic_absorption
        .as_mut()
        .unwrap()
        .replacement
        .as_mut()
        .unwrap()
        .item_id = "test.wrong".to_owned();
    let stale = game.to_save();
    assert!(Game::from_save(stale.clone()).is_err());
    assert!(matches!(
        game.dispatch(command(
            game.last_command_seq + 1,
            game.revision,
            resolve(true, false)
        )),
        Err(CoreError::MagicAbsorptionUnavailable("replacement-changed"))
    ));
    assert_eq!(game.to_save(), stale);
}
