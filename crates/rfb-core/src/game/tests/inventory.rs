// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

#[test]
fn ring_slots_fill_in_body_order_and_replace_deterministically() {
    let mut game = Game::new(42);
    for ordinal in 1..=3 {
        game.items.push(ItemInstance {
            id: format!("test.item.band.{ordinal}"),
            kind_id: "demo.item.resonant-band".to_owned(),
            quantity: 1,
            quality: ItemQualityDto::Ordinary,
            affix_ids: Vec::new(),
            rolled_affixes: Vec::new(),
            enchantments: Default::default(),
            curse: None,
            activation: None,
            charges: None,
            fuel: None,
            device_recovery_progress: 0,
            location: ItemLocation::Inventory,
        });
    }
    let slot_of = |game: &Game, id: &str| {
        game.items
            .iter()
            .find(|item| item.id == id)
            .map(|item| match &item.location {
                ItemLocation::Equipped { slot_id } => slot_id.clone(),
                _ => "unequipped".to_owned(),
            })
            .expect("test band should exist")
    };

    game.dispatch(command(
        1,
        0,
        GameCommand::Equip {
            item_id: "test.item.band.1".to_owned(),
            slot_id: None,
        },
    ))
    .expect("first ring should equip");
    game.dispatch(command(
        2,
        1,
        GameCommand::Equip {
            item_id: "test.item.band.2".to_owned(),
            slot_id: None,
        },
    ))
    .expect("second ring should equip");
    assert_eq!(slot_of(&game, "test.item.band.1"), "ring-1");
    assert_eq!(slot_of(&game, "test.item.band.2"), "ring-2");
    let snapshot = game.snapshot();
    assert_eq!(snapshot.player.equipment_modifiers.defense, 2);
    assert_eq!(snapshot.body_slots.len(), 15);
    assert!(
        snapshot
            .body_slots
            .iter()
            .any(|slot| slot.id == "light" && slot.slot_type == "light")
    );

    // All ring instances occupied: the next equip replaces the first
    // instance in body order, returning its occupant to the inventory.
    game.dispatch(command(
        3,
        2,
        GameCommand::Equip {
            item_id: "test.item.band.3".to_owned(),
            slot_id: None,
        },
    ))
    .expect("third ring should replace the first instance");
    assert_eq!(slot_of(&game, "test.item.band.3"), "ring-1");
    assert_eq!(slot_of(&game, "test.item.band.1"), "unequipped");
    assert_eq!(slot_of(&game, "test.item.band.2"), "ring-2");

    let restored = Game::from_save(game.to_save()).expect("body slots should round trip");
    assert_eq!(restored.body_slots.len(), 15);
    assert_eq!(slot_of(&restored, "test.item.band.3"), "ring-1");
}

#[test]
fn pickup_moves_the_ground_stack_into_inventory() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    game.dispatch(command(
        1,
        0,
        GameCommand::Move {
            direction: Direction::East,
        },
    ))
    .expect("move should execute");
    let update = game
        .dispatch(command(2, 1, GameCommand::PickUp))
        .expect("pickup should execute");

    assert_eq!(update.items.len(), 4);
    assert_eq!(update.inventory.len(), 1);
    assert_eq!(update.inventory[0].id, "demo.item.luminous-shard.1");
    assert_eq!(update.inventory[0].quantity, 5);
    assert_eq!(update.player.carried_weight_tenths_pound, 50);
    assert_eq!(update.player.carry_capacity_tenths_pound, 1_000);
    assert_eq!(update.changed_cells.len(), 1);
    assert_eq!(update.changed_cells[0].position, Position { x: 4, y: 3 });
    assert_eq!(update.changed_cells[0].item_id, None);
    assert_eq!(update.events[0].message_key, "item-pickup-success");
}

#[test]
fn pickup_can_make_the_player_overburdened() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    game.player.position = Position { x: 6, y: 4 };
    support::give_inventory_item(&mut game, "test.heavy-stack", "demo.item.burdened-mail");
    game.items
        .iter_mut()
        .find(|item| item.id == "test.heavy-stack")
        .expect("fixture item should exist")
        .quantity = 9;
    assert_eq!(game.carried_weight_tenths_pound(), 1_260);

    let update = game
        .dispatch(command(1, 0, GameCommand::PickUp))
        .expect("overburdened pickup should resolve as an action");

    let event = &update.events[0];
    assert_eq!(event.kind, "item.pickup");
    assert_eq!(update.player.carried_weight_tenths_pound, 1_272);
    assert_eq!(update.player.encumbrance_speed_penalty, 1);
    assert_eq!(update.player.speed, 109);
    assert!(
        update
            .inventory
            .iter()
            .any(|item| { item.id == "demo.item.resonance-pellet.1" && item.quantity == 6 })
    );
}

#[test]
fn twenty_seventh_inventory_stack_is_rejected() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    game.items.clear();
    for index in 0..26 {
        give_inventory_item(
            &mut game,
            &format!("test.inventory.band.{index}"),
            "demo.item.resonant-band",
        );
    }
    game.items.push(ItemInstance {
        id: "test.ground.band".to_owned(),
        kind_id: "demo.item.resonant-band".to_owned(),
        quantity: 1,
        quality: ItemQualityDto::Ordinary,
        affix_ids: Vec::new(),
        rolled_affixes: Vec::new(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        fuel: None,
        device_recovery_progress: 0,
        location: ItemLocation::Ground(game.player.position),
    });

    let update = dispatch_next(&mut game, GameCommand::PickUp);

    assert_eq!(update.player.inventory_used_slots, 26);
    assert_eq!(update.player.inventory_slot_capacity, 26);
    assert!(update.events.iter().any(|event| {
        event.kind == "item.pickup.inventory-full"
            && event.args["usedSlots"] == "26"
            && event.args["requiredSlots"] == "1"
            && event.args["capacity"] == "26"
    }));
    assert!(game.items.iter().any(|item| {
        item.id == "test.ground.band" && item.location == ItemLocation::Ground(game.player.position)
    }));
}

#[test]
fn full_inventory_can_merge_into_an_existing_stack() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    game.items.clear();
    for index in 0..25 {
        give_inventory_item(
            &mut game,
            &format!("test.inventory.band.{index}"),
            "demo.item.resonant-band",
        );
    }
    give_inventory_item(&mut game, "test.inventory.arrow", "demo.item.arrow");
    let ground = ItemInstance {
        id: "test.ground.arrow".to_owned(),
        kind_id: "demo.item.arrow".to_owned(),
        quantity: 1,
        quality: ItemQualityDto::Ordinary,
        affix_ids: Vec::new(),
        rolled_affixes: Vec::new(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        fuel: None,
        device_recovery_progress: 0,
        location: ItemLocation::Ground(game.player.position),
    };
    game.items.push(ground);

    let update = dispatch_next(&mut game, GameCommand::PickUp);

    assert_eq!(update.player.inventory_used_slots, 26);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.pickup")
    );
    assert!(game.items.iter().any(|item| {
        item.id == "test.inventory.arrow"
            && item.location == ItemLocation::Inventory
            && item.quantity == 2
    }));
    assert!(!game.items.iter().any(|item| item.id == "test.ground.arrow"));
}

#[test]
fn fabric_bag_adds_four_shared_inventory_slots() {
    let mut game = Game::new(42);
    game.items.clear();
    give_inventory_item(
        &mut game,
        "test.inventory.fabric-bag",
        "demo.item.fabric-bag",
    );

    let update = dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: "test.inventory.fabric-bag".to_owned(),
            slot_id: None,
        },
    );

    assert_eq!(update.player.inventory_used_slots, 0);
    assert_eq!(update.player.inventory_slot_capacity, 30);
    assert!(
        update
            .equipment
            .iter()
            .any(|item| { item.id == "test.inventory.fabric-bag" && item.slot_id == "container" })
    );
}

#[test]
fn container_cannot_be_unequipped_while_its_slots_are_in_use() {
    let mut game = Game::new(42);
    game.items.clear();
    give_inventory_item(
        &mut game,
        "test.inventory.fabric-bag",
        "demo.item.fabric-bag",
    );
    dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: "test.inventory.fabric-bag".to_owned(),
            slot_id: None,
        },
    );
    for index in 0..27 {
        give_inventory_item(
            &mut game,
            &format!("test.inventory.band.{index}"),
            "demo.item.resonant-band",
        );
    }

    let update = dispatch_next(
        &mut game,
        GameCommand::Unequip {
            slot_id: "container".to_owned(),
        },
    );

    assert_eq!(update.player.inventory_used_slots, 27);
    assert_eq!(update.player.inventory_slot_capacity, 30);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.unequip.none")
    );
    assert!(
        update
            .equipment
            .iter()
            .any(|item| item.id == "test.inventory.fabric-bag")
    );
}

#[test]
fn equipping_and_unequipping_moves_an_item_between_authoritative_lists() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    collect_both_demo_items(&mut game);
    let carried = game.snapshot();
    let charm = carried
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.echo-charm")
        .expect("collected charm should be in inventory");
    assert_eq!(charm.modifiers.attack, 1);
    assert_eq!(charm.identification, ItemIdentificationDto::Unexamined);
    assert_eq!(charm.quality, None);
    assert!(charm.known_properties.is_empty());
    assert!(
        game.to_save()
            .item_property_knowledge
            .iter()
            .all(|knowledge| knowledge.discovered && !knowledge.appraised)
    );
    let equipped = game
        .dispatch(command(
            5,
            4,
            GameCommand::Equip {
                item_id: "demo.item.echo-charm.1".to_owned(),
                slot_id: None,
            },
        ))
        .expect("equipping should execute");

    assert_eq!(equipped.inventory.len(), 1);
    assert_eq!(equipped.equipment.len(), 1);
    assert_eq!(equipped.equipment[0].slot_id, "charm");
    assert_eq!(equipped.equipment[0].modifiers.attack, 2);
    assert_eq!(equipped.equipment[0].modifiers.defense, 1);
    assert_eq!(equipped.equipment[0].modifiers.max_hp, 4);
    assert_eq!(equipped.player.base_max_hp, 10);
    assert_eq!(equipped.player.max_hp, 14);
    assert_eq!(equipped.player.base_attack, 2);
    assert_eq!(equipped.player.attack, 4);
    assert_eq!(equipped.player.base_defense, 1);
    assert_eq!(equipped.player.defense, 2);
    assert_eq!(equipped.player.equipment_modifiers.attack, 2);
    assert_eq!(equipped.player.equipment_modifiers.defense, 1);
    assert_eq!(equipped.player.equipment_modifiers.max_hp, 4);
    assert_eq!(equipped.player.carried_weight_tenths_pound, 55);
    assert_eq!(equipped.events[0].message_key, "item-equip-success");
    assert_eq!(equipped.events[1].message_key, "item-property-discovered");
    assert_eq!(equipped.equipment[0].known_properties.len(), 1);
    assert_eq!(
        equipped.equipment[0].identification,
        ItemIdentificationDto::Identified
    );
    assert_eq!(equipped.equipment[0].quality, Some(ItemQualityDto::Fine));
    assert_eq!(
        equipped.equipment[0].known_properties[0].affix_id,
        "demo.affix.harmonic-edge"
    );
    let saved = game.to_save();
    let charm_knowledge = saved
        .item_property_knowledge
        .iter()
        .find(|knowledge| knowledge.item_id == "demo.item.echo-charm.1")
        .expect("equipped charm knowledge should be saved");
    assert!(charm_knowledge.discovered);
    assert!(charm_knowledge.appraised);
    assert!(charm_knowledge.identified);
    let restored = Game::from_save(saved.clone()).expect("affix knowledge should round trip");
    assert_eq!(restored.state_hash(), game.state_hash());
    let mut invalid = saved;
    invalid
        .item_property_knowledge
        .iter_mut()
        .find(|knowledge| knowledge.item_id == "demo.item.echo-charm.1")
        .expect("equipped charm knowledge should be saved")
        .known_affix_ids = vec!["demo.affix.missing".to_owned()];
    assert!(matches!(
        Game::from_save(invalid),
        Err(CoreError::InvalidSave(
            "item property knowledge state is invalid"
        ))
    ));

    game.player.hp = 14;

    let unequipped = game
        .dispatch(command(
            6,
            5,
            GameCommand::Unequip {
                slot_id: "charm".to_owned(),
            },
        ))
        .expect("unequipping should execute");
    assert_eq!(unequipped.inventory.len(), 2);
    assert!(unequipped.equipment.is_empty());
    assert_eq!(unequipped.player.carried_weight_tenths_pound, 55);
    assert_eq!(unequipped.player.hp, 10);
    assert_eq!(unequipped.player.max_hp, 10);
    assert_eq!(unequipped.player.attack, 2);
    assert_eq!(unequipped.player.defense, 1);
    assert_eq!(unequipped.events[0].message_key, "item-unequip-success");
}

#[test]
fn appraising_reveals_quality_without_revealing_affixes() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    collect_both_demo_items(&mut game);

    let before = game.snapshot();
    let charm = before
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.echo-charm")
        .expect("collected charm should be in inventory");
    assert_eq!(charm.identification, ItemIdentificationDto::Unexamined);
    assert_eq!(charm.quality, None);
    assert!(charm.known_properties.is_empty());

    let appraised = game
        .dispatch(command(
            5,
            4,
            GameCommand::Appraise {
                item_id: "demo.item.echo-charm.1".to_owned(),
            },
        ))
        .expect("appraisal should execute");
    let charm = appraised
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.echo-charm")
        .expect("appraised charm should remain in inventory");
    assert_eq!(charm.identification, ItemIdentificationDto::Appraised);
    assert_eq!(charm.quality, Some(ItemQualityDto::Fine));
    assert_eq!(charm.modifiers.attack, 1);
    assert!(charm.known_properties.is_empty());
    assert_eq!(appraised.player.attack, 2);
    assert_eq!(appraised.events[0].message_key, "item-appraise-success");
    assert_eq!(appraised.events[0].args["quality"], "fine");

    let saved = game.to_save();
    let charm_knowledge = saved
        .item_property_knowledge
        .iter()
        .find(|knowledge| knowledge.item_id == "demo.item.echo-charm.1")
        .expect("appraised charm knowledge should be saved");
    assert!(charm_knowledge.discovered);
    assert!(charm_knowledge.appraised);
    assert!(!charm_knowledge.identified);
    assert!(charm_knowledge.known_affix_ids.is_empty());
    let restored = Game::from_save(saved).expect("appraisal knowledge should round trip");
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn player_derived_stats_retain_equipment_and_status_sources() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    collect_both_demo_items(&mut game);
    game.dispatch(command(
        5,
        4,
        GameCommand::Equip {
            item_id: "demo.item.echo-charm.1".to_owned(),
            slot_id: None,
        },
    ))
    .expect("equipping should execute");
    game.player.statuses.push(StatusInstance {
        kind_id: STATUS_HASTE.to_owned(),
        intensity: 2,
        remaining_ticks: 3,
        source_id: Some("demo.item.temporary-tonic.1".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    game.player.statuses.push(StatusInstance {
        kind_id: STATUS_STUN.to_owned(),
        intensity: 2,
        remaining_ticks: 3,
        source_id: Some("demo.monster.impact.1".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    game.player
        .statuses
        .sort_by(|left, right| left.kind_id.cmp(&right.kind_id));

    let stats = game.player_derived_stats();

    assert_eq!(stats.attack.value, 4);
    assert_eq!(stats.speed.value, 130);
    assert_eq!(stats.melee_skill.value, 60);
    assert!(stats.attack.contributions.iter().any(|contribution| {
        contribution.layer == StatLayer::Equipment
            && contribution.source_id == "demo.item.echo-charm.1"
            && contribution.amount == 2
    }));
    assert!(stats.speed.contributions.iter().any(|contribution| {
        contribution.layer == StatLayer::Status
            && contribution.source_id == STATUS_HASTE
            && contribution.origin_id.as_deref() == Some("demo.item.temporary-tonic.1")
            && contribution.amount == 20
    }));
    assert!(stats.melee_skill.contributions.iter().any(|contribution| {
        contribution.layer == StatLayer::Status
            && contribution.source_id == STATUS_STUN
            && contribution.origin_id.as_deref() == Some("demo.monster.impact.1")
            && contribution.amount == -20
    }));
}

#[test]
fn armor_hit_modifier_only_changes_melee_skill() {
    let mut game = Game::new(42);
    game.items.clear();
    let baseline = game.player_derived_stats();
    game.items.push(ItemInstance {
        id: "test.item.hard-leather-armour".to_owned(),
        kind_id: "demo.item.hard-leather-armour".to_owned(),
        quantity: 1,
        quality: ItemQualityDto::Ordinary,
        affix_ids: Vec::new(),
        rolled_affixes: Vec::new(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        fuel: None,
        device_recovery_progress: 0,
        location: ItemLocation::Equipped {
            slot_id: "body".to_owned(),
        },
    });

    let equipped = game.player_derived_stats();
    assert_eq!(equipped.melee_skill.value, baseline.melee_skill.value - 1);
    assert_eq!(
        equipped.melee_damage_bonus.value,
        baseline.melee_damage_bonus.value
    );
    assert_eq!(equipped.ranged_skill.value, baseline.ranged_skill.value);
}

#[test]
fn gauntlets_add_their_hit_and_damage_modifiers_to_melee() {
    let mut game = Game::new(42);
    game.items.clear();
    let baseline = game.player_derived_stats();
    game.items.push(ItemInstance {
        id: "test.item.set-of-gauntlets".to_owned(),
        kind_id: "demo.item.set-of-gauntlets".to_owned(),
        quantity: 1,
        quality: ItemQualityDto::Ordinary,
        affix_ids: Vec::new(),
        rolled_affixes: Vec::new(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        fuel: None,
        device_recovery_progress: 0,
        location: ItemLocation::Equipped {
            slot_id: "hands".to_owned(),
        },
    });

    let equipped = game.player_derived_stats();
    assert_eq!(equipped.melee_skill.value, baseline.melee_skill.value + 1);
    assert_eq!(
        equipped.melee_damage_bonus.value,
        baseline.melee_damage_bonus.value + 1
    );
    assert_eq!(equipped.ranged_skill.value, baseline.ranged_skill.value);
}

#[test]
fn shovel_equips_as_a_tool_without_replacing_the_melee_weapon_profile() {
    let mut game = Game::new(42);
    game.items.clear();
    let baseline = game.player_derived_stats();
    give_inventory_item(&mut game, "test.item.shovel", "demo.item.shovel");

    let update = dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: "test.item.shovel".to_owned(),
            slot_id: Some("tool".to_owned()),
        },
    );

    let equipped = game.player_derived_stats();
    let profile = game.player_melee_profile(&equipped);
    assert_eq!(update.equipment[0].slot_id, "tool");
    assert_eq!(equipped.dig_skill.value, baseline.dig_skill.value + 2);
    assert_eq!(equipped.attack.value, baseline.attack.value);
    assert_eq!(equipped.defense.value, baseline.defense.value);
    assert_eq!(equipped.melee_skill.value, baseline.melee_skill.value);
    assert_eq!(
        equipped.melee_damage_bonus.value,
        baseline.melee_damage_bonus.value
    );
    assert_eq!((profile.damage_dice, profile.damage_sides), (1, 2));
    assert_eq!(profile.source_item_id, None);
}

#[test]
fn shovel_equips_as_a_weapon_with_its_full_melee_profile() {
    let mut game = Game::new(42);
    game.items.clear();
    let baseline = game.player_derived_stats();
    give_inventory_item(&mut game, "test.item.shovel", "demo.item.shovel");

    let update = dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: "test.item.shovel".to_owned(),
            slot_id: Some("weapon".to_owned()),
        },
    );

    let equipped = game.player_derived_stats();
    let profile = game.player_melee_profile(&equipped);
    assert_eq!(update.equipment[0].slot_id, "weapon");
    assert_eq!(equipped.dig_skill.value, baseline.dig_skill.value + 2);
    assert_eq!((profile.damage_dice, profile.damage_sides), (1, 3));
    assert_eq!(profile.source_item_id.as_deref(), Some("test.item.shovel"));

    let restored = Game::from_save(game.to_save()).expect("weapon-slot tool should reload");
    let restored_stats = restored.player_derived_stats();
    let restored_profile = restored.player_melee_profile(&restored_stats);
    assert_eq!(restored.snapshot().equipment[0].slot_id, "weapon");
    assert_eq!(
        (restored_profile.damage_dice, restored_profile.damage_sides),
        (1, 3)
    );
}

#[test]
fn tool_rejects_an_unrelated_target_slot_without_changing_inventory() {
    let mut game = Game::new(42);
    game.items.clear();
    give_inventory_item(&mut game, "test.item.shovel", "demo.item.shovel");

    let update = dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: "test.item.shovel".to_owned(),
            slot_id: Some("body".to_owned()),
        },
    );

    assert!(update.equipment.is_empty());
    assert_eq!(update.inventory.len(), 1);
    assert_eq!(update.inventory[0].id, "test.item.shovel");
    assert_eq!(update.events[0].kind, "item.equip.none");
}

#[test]
fn fear_check_can_consume_a_melee_action_without_attacking() {
    let mut game = Game::new(0);
    game.rng = RfbRng::seeded(0);
    game.entities[0].position = Position { x: 4, y: 3 };
    game.entities[0].statuses.push(StatusInstance {
        kind_id: STATUS_SLOW.to_owned(),
        intensity: 10,
        remaining_ticks: 20,
        source_id: None,
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    game.player.statuses.push(StatusInstance {
        kind_id: STATUS_FEAR.to_owned(),
        intensity: 2,
        remaining_ticks: 20,
        source_id: Some("demo.monster.ember-mote.1".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::Move {
                direction: Direction::East,
            },
        ))
        .expect("fear-blocked action should still execute");

    assert_eq!(update.player.position, Position { x: 3, y: 3 });
    assert_eq!(update.entities[0].hp, 3);
    assert_eq!(update.turn, 1);
    assert_eq!(update.player.statuses[0].kind_id, STATUS_FEAR);
    assert_eq!(update.player.statuses[0].remaining_ticks, 10);
    assert_eq!(game.rng.draw_counter, 2);
    assert_eq!(update.events.len(), 1);
    assert_eq!(update.events[0].message_key, "status-fear-blocked");
}

#[test]
fn item_instance_identity_survives_location_transitions() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    let original_instance_count = game.items.len();
    collect_both_demo_items(&mut game);

    let charm_id = "demo.item.echo-charm.1";
    assert_eq!(game.items.len(), original_instance_count);
    assert!(game.items.iter().any(|item| {
        item.id == charm_id && item.location == ItemLocation::Inventory && item.quantity == 1
    }));

    game.dispatch(command(
        5,
        4,
        GameCommand::Equip {
            item_id: charm_id.to_owned(),
            slot_id: None,
        },
    ))
    .expect("equip should execute");
    assert!(game.items.iter().any(|item| {
        item.id == charm_id
            && item.location
                == ItemLocation::Equipped {
                    slot_id: "charm".to_owned(),
                }
    }));

    game.dispatch(command(
        6,
        5,
        GameCommand::Unequip {
            slot_id: "charm".to_owned(),
        },
    ))
    .expect("unequip should execute");
    game.dispatch(command(
        7,
        6,
        GameCommand::Drop {
            item_ids: vec![charm_id.to_owned()],
        },
    ))
    .expect("drop should execute");

    assert_eq!(game.items.len(), original_instance_count);
    assert!(game.items.iter().any(|item| {
        item.id == charm_id
            && item.location == ItemLocation::Ground(game.player.position)
            && item.quantity == 1
    }));
}

#[test]
fn equipped_attack_modifier_changes_authoritative_melee_skill() {
    let mut game = Game::new(42);
    collect_both_demo_items(&mut game);
    game.dispatch(command(
        5,
        4,
        GameCommand::Equip {
            item_id: "demo.item.echo-charm.1".to_owned(),
            slot_id: None,
        },
    ))
    .expect("equip should execute");
    game.entities[0].position = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    game.entities[0].energy_need = STANDARD_ACTION_COST;
    game.rng = RfbRng::seeded(42);
    let update = game
        .dispatch(command(
            6,
            5,
            GameCommand::Move {
                direction: Direction::East,
            },
        ))
        .expect("equipped attack should execute");

    assert_eq!(update.events[0].message_key, "combat-player-hit");
    assert_eq!(update.player.melee_skill, 80);
    assert_eq!(update.events[0].args["damage"], "2");
    assert_eq!(update.entities[0].hp, 1);
}

#[test]
fn equipped_weapon_profile_drives_two_stable_player_attacks() {
    let mut game = Game::new(42);
    game.rng = RfbRng::seeded(42);
    let weapon = game
        .items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.echo-blade")
        .expect("demo weapon should exist");
    weapon.location = ItemLocation::Equipped {
        slot_id: "weapon".to_owned(),
    };
    let snapshot = game.snapshot();
    let profile = snapshot.player.melee_profile;

    assert_eq!(profile.attacks, 2);
    assert_eq!(profile.to_hit, 10);
    assert_eq!(profile.to_damage, 1);
    assert_eq!(profile.damage.dice, 1);
    assert_eq!(profile.damage.sides, 2);
    assert_eq!(
        profile.source_item_id.as_deref(),
        Some("demo.item.echo-blade.1")
    );
    assert_eq!(snapshot.equipment[0].melee_profile, Some(profile));

    let mut events = Vec::new();
    let mut changed = BTreeSet::new();
    let mut removed = Vec::new();
    game.resolve_player_melee(0, &mut events, &mut changed, &mut removed)
        .expect("melee resolution should succeed");

    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                DomainEvent::PlayerMeleeHit { .. } | DomainEvent::PlayerMeleeMissed { .. }
            ))
            .count(),
        2
    );
    assert!(removed.is_empty());
}

#[test]
fn equipped_launcher_traces_to_first_target_and_resolves_damage() {
    let mut game = Game::new(0);
    game.rng = RfbRng::seeded(0);
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.resonance-sling")
        .expect("demo launcher should exist")
        .location = ItemLocation::Equipped {
        slot_id: "launcher".to_owned(),
    };
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.resonance-pellet")
        .expect("demo ammunition should exist")
        .location = ItemLocation::Inventory;
    game.entities[0].position = Position { x: 7, y: 3 };
    game.entities[0].hp = 10;

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::Fire {
                direction: Direction::East,
            },
        ))
        .expect("projectile action should execute");

    let projectile = update
        .events
        .iter()
        .find(|event| event.kind.starts_with("combat.projectile-"))
        .expect("projectile event should be emitted");
    let trace = projectile
        .trace
        .as_ref()
        .expect("projectile trace should exist");
    assert_eq!(trace.origin, Position { x: 3, y: 3 });
    assert_eq!(trace.impact, Position { x: 7, y: 3 });
    assert_eq!(trace.landing, Position { x: 7, y: 3 });
    assert_eq!(trace.traversed.len(), 4);
    assert_eq!(projectile.kind, "combat.projectile-hit");
    assert!(update.entities[0].hp < 10);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "combat.projectile-ammo-recovered")
    );
    assert_eq!(
        update
            .inventory
            .iter()
            .find(|item| item.kind_id == "demo.item.resonance-pellet")
            .map(|item| item.quantity),
        Some(5)
    );
    assert!(update.items.iter().any(|item| {
        item.id == "generated.item.2"
            && item.kind_id == "demo.item.resonance-pellet"
            && item.quantity == 1
            && item.position == Position { x: 7, y: 3 }
    }));
}

#[test]
fn sling_bow_and_crossbow_resolve_their_compatible_ammunition_profiles() {
    let cases = [
        (
            "demo.item.sling",
            "demo.item.mithril-shot",
            15,
            200,
            5,
            3,
            10,
        ),
        (
            "demo.item.long-bow",
            "demo.item.sheaf-arrow",
            16,
            300,
            4,
            4,
            20,
        ),
        (
            "demo.item.heavy-crossbow",
            "demo.item.adamantine-bolt",
            18,
            400,
            7,
            5,
            10,
        ),
    ];

    for (launcher_kind, ammo_kind, range, multiplier, dice, sides, break_chance) in cases {
        let mut game = Game::new(0);
        for item in &mut game.items {
            if matches!(item.location, ItemLocation::Equipped { ref slot_id } if slot_id == "launcher")
            {
                item.location = ItemLocation::Inventory;
            }
        }
        give_inventory_item(&mut game, "test.item.launcher", launcher_kind);
        give_inventory_item(&mut game, "test.item.ammunition", ammo_kind);
        game.items
            .iter_mut()
            .find(|item| item.id == "test.item.launcher")
            .expect("test launcher should exist")
            .location = ItemLocation::Equipped {
            slot_id: "launcher".to_owned(),
        };

        let profile = game
            .player_projectile_profile()
            .expect("compatible ammunition should resolve a projectile profile");
        assert_eq!(profile.range, range);
        assert_eq!(profile.damage_multiplier_percent, multiplier);
        assert_eq!((profile.damage_dice, profile.damage_sides), (dice, sides));
        assert_eq!(profile.ammo_kind_id, ammo_kind);
        assert_eq!(profile.ammo_break_chance_percent, break_chance);
    }
}

#[test]
fn ammunition_breakage_is_checked_after_hitting_a_body() {
    let mut game = Game::new(16);
    game.rng = RfbRng::seeded(16);
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.resonance-sling")
        .expect("demo launcher should exist")
        .location = ItemLocation::Equipped {
        slot_id: "launcher".to_owned(),
    };
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.resonance-pellet")
        .expect("demo ammunition should exist")
        .location = ItemLocation::Inventory;
    game.entities[0].position = Position { x: 7, y: 3 };
    game.entities[0].hp = 10;

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::Fire {
                direction: Direction::East,
            },
        ))
        .expect("projectile action should execute");

    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "combat.projectile-ammo-broken")
    );
    assert_eq!(update.inventory[0].quantity, 5);
    assert!(!update.items.iter().any(|item| {
        item.kind_id == "demo.item.resonance-pellet" && item.position == Position { x: 7, y: 3 }
    }));
    assert_eq!(game.next_item_instance_serial, 3);
}

#[test]
fn ammunition_that_hits_no_body_lands_without_a_breakage_roll() {
    let mut game = Game::new(0);
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.resonance-sling")
        .expect("demo launcher should exist")
        .location = ItemLocation::Equipped {
        slot_id: "launcher".to_owned(),
    };
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.resonance-pellet")
        .expect("demo ammunition should exist")
        .location = ItemLocation::Inventory;
    let rng_draws = game.rng_draw_counter();

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::Fire {
                direction: Direction::North,
            },
        ))
        .expect("projectile action should execute");

    assert_eq!(game.rng_draw_counter(), rng_draws);
    assert_eq!(update.events[0].kind, "combat.projectile-landed");
    assert_eq!(update.events[1].kind, "combat.projectile-ammo-recovered");
    assert!(update.items.iter().any(|item| {
        item.kind_id == "demo.item.resonance-pellet" && item.position == Position { x: 3, y: 1 }
    }));
}

#[test]
fn launcher_without_inventory_ammunition_does_not_advance_rng() {
    let mut game = Game::new(0);
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.resonance-sling")
        .expect("demo launcher should exist")
        .location = ItemLocation::Equipped {
        slot_id: "launcher".to_owned(),
    };
    let rng_draws = game.rng_draw_counter();

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::Fire {
                direction: Direction::East,
            },
        ))
        .expect("unavailable fire action should execute deterministically");

    assert_eq!(update.events[0].kind, "combat.projectile-ammo-unavailable");
    assert_eq!(game.rng_draw_counter(), rng_draws);
    assert!(update.inventory.is_empty());
    assert!(
        update
            .items
            .iter()
            .any(|item| { item.kind_id == "demo.item.resonance-pellet" && item.quantity == 6 })
    );
}

#[test]
fn entity_targeting_uses_a_stable_off_axis_line() {
    let mut game = Game::new(0);
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.resonance-sling")
        .expect("demo launcher should exist")
        .location = ItemLocation::Equipped {
        slot_id: "launcher".to_owned(),
    };
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.resonance-pellet")
        .expect("demo ammunition should exist")
        .location = ItemLocation::Inventory;
    game.entities[0].position = Position { x: 9, y: 5 };
    game.entities[0].hp = 10;
    let expected_path = vec![
        Position { x: 4, y: 3 },
        Position { x: 5, y: 4 },
        Position { x: 6, y: 4 },
        Position { x: 7, y: 4 },
        Position { x: 8, y: 5 },
        Position { x: 9, y: 5 },
    ];
    assert_eq!(
        game.projectile_path(
            &TargetSelection::Position {
                position: Position { x: 9, y: 5 },
            },
            6,
        ),
        Some(expected_path.clone())
    );

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::FireTarget {
                target: TargetSelection::Entity {
                    entity_id: "demo.monster.ember-mote.1".to_owned(),
                },
            },
        ))
        .expect("targeted projectile action should execute");

    let projectile = update
        .events
        .iter()
        .find(|event| event.kind == "combat.projectile-hit")
        .expect("targeted projectile should hit");
    let trace = projectile.trace.as_ref().expect("trace should exist");
    assert_eq!(trace.impact, Position { x: 9, y: 5 });
    assert_eq!(trace.traversed, expected_path);
    let target_spec = update
        .player
        .projectile_profile
        .as_ref()
        .expect("equipped launcher profile should exist")
        .target_spec
        .clone();
    assert_eq!(target_spec.range, 6);
    assert_eq!(
        target_spec.modes,
        [
            TargetModeDto::Direction,
            TargetModeDto::Position,
            TargetModeDto::Entity,
        ]
    );
}

#[test]
fn invalid_entity_target_preserves_ammunition_and_rng() {
    let mut game = Game::new(0);
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.resonance-sling")
        .expect("demo launcher should exist")
        .location = ItemLocation::Equipped {
        slot_id: "launcher".to_owned(),
    };
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.resonance-pellet")
        .expect("demo ammunition should exist")
        .location = ItemLocation::Inventory;
    let rng_draws = game.rng_draw_counter();

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::FireTarget {
                target: TargetSelection::Entity {
                    entity_id: "demo.monster.missing.1".to_owned(),
                },
            },
        ))
        .expect("invalid target should produce a deterministic event");

    assert_eq!(
        update.events[0].kind,
        "combat.projectile-target-unavailable"
    );
    assert_eq!(game.rng_draw_counter(), rng_draws);
    assert_eq!(update.inventory[0].quantity, 6);
}

#[test]
fn throwing_one_item_splits_the_stack_and_lands_before_a_wall() {
    let mut game = Game::new(0);
    game.rng = RfbRng::seeded(0);
    game.player.position = Position { x: 10, y: 3 };
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("demo throwable stack should exist")
        .location = ItemLocation::Inventory;

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::Throw {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                direction: Direction::East,
            },
        ))
        .expect("throw action should execute");

    let thrown = update
        .events
        .iter()
        .find(|event| event.kind == "item.thrown")
        .expect("throw event should be emitted");
    let trace = thrown.trace.as_ref().expect("throw trace should exist");
    assert_eq!(trace.origin, Position { x: 10, y: 3 });
    assert_eq!(trace.impact, Position { x: 11, y: 3 });
    assert_eq!(trace.landing, Position { x: 10, y: 3 });
    assert!(trace.traversed.is_empty());
    assert_eq!(update.inventory[0].quantity, 4);
    assert!(update.items.iter().any(|item| {
        item.id == "generated.item.2"
            && item.kind_id == "demo.item.luminous-shard"
            && item.quantity == 1
            && item.position == Position { x: 10, y: 3 }
    }));
}

#[test]
fn throwable_profile_uses_weight_range_and_resolves_damage() {
    let mut game = Game::new(0);
    game.rng = RfbRng::seeded(0);
    game.item_knowledge.insert(
        "demo.item.luminous-shard".to_owned(),
        ItemKnowledgeState {
            tried: true,
            aware: true,
        },
    );
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("demo throwable stack should exist")
        .location = ItemLocation::Inventory;
    game.entities[0].position = Position { x: 6, y: 3 };
    game.entities[0].hp = 10;
    let inventory = game.snapshot().inventory;
    let shard = inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("throwable should be projected into inventory");
    assert_eq!(shard.weight_tenths_pound, 10);
    assert_eq!(
        shard
            .throw_profile
            .as_ref()
            .expect("shard should expose its throw profile")
            .range,
        5
    );

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::Throw {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                direction: Direction::East,
            },
        ))
        .expect("throw attack should execute");

    let hit = update
        .events
        .iter()
        .find(|event| event.kind == "combat.throw-hit")
        .expect("throw hit should be emitted");
    assert_eq!(hit.args["source"], "demo.item.luminous-shard");
    assert_eq!(hit.args["target"], "demo.actor.ember-mote");
    assert_eq!(hit.args["damage"], "1");
    assert_eq!(update.entities[0].hp, 9);
    assert_eq!(update.inventory[0].quantity, 4);
    assert!(update.items.iter().any(|item| {
        item.id == "generated.item.2"
            && item.kind_id == "demo.item.luminous-shard"
            && item.position == Position { x: 6, y: 3 }
    }));
}

#[test]
fn throwing_an_unknown_item_marks_the_kind_tried_and_preserves_its_appearance() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("demo unknown stack should exist")
        .location = ItemLocation::Inventory;
    let before = game.snapshot();
    let shard = before
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("unknown shard should be projected");
    assert_eq!(shard.knowledge, ItemKnowledgeDto::Unknown);
    assert_eq!(shard.display_name_key, "item-demo-unfamiliar-shard-name");
    assert!(shard.throw_profile.is_none());

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::Throw {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                direction: Direction::North,
            },
        ))
        .expect("throwing an unknown item should execute");

    let remaining = update
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("remaining stack should stay carried");
    assert_eq!(remaining.knowledge, ItemKnowledgeDto::Tried);
    assert_eq!(
        remaining.display_name_key,
        "item-demo-unfamiliar-shard-name"
    );
    assert!(remaining.throw_profile.is_none());
    assert_eq!(game.to_save().item_knowledge.len(), 1);
    let restored = Game::from_save(game.to_save()).expect("tried knowledge should reload");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn aware_item_knowledge_reveals_the_true_name_and_profile_after_reload() {
    let mut game = Game::new(7);
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("demo unknown stack should exist")
        .location = ItemLocation::Inventory;
    let mut payload = game.to_save();
    payload.item_knowledge = vec![ItemKnowledgeSaveDto {
        kind_id: "demo.item.luminous-shard".to_owned(),
        tried: true,
        aware: true,
    }];

    let restored = Game::from_save(payload).expect("aware knowledge should load");
    let shard = restored
        .snapshot()
        .inventory
        .into_iter()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("aware shard should be projected");
    assert_eq!(shard.knowledge, ItemKnowledgeDto::Aware);
    assert_eq!(shard.display_name_key, "item-demo-luminous-shard-name");
    assert!(shard.throw_profile.is_some());

    let mut invalid = restored.to_save();
    invalid.item_knowledge[0].tried = false;
    assert!(matches!(
        Game::from_save(invalid),
        Err(CoreError::InvalidSave("item knowledge state is invalid"))
    ));
}

#[test]
fn observable_item_use_consumes_one_heals_and_marks_the_kind_aware() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    game.player.hp = 3;
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("demo usable stack should exist")
        .location = ItemLocation::Inventory;

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::UseItem {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                target: None,
            },
        ))
        .expect("using a healing item should execute");

    assert_eq!(update.player.hp, 7);
    let shard = update
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("the remaining stack should stay carried");
    assert_eq!(shard.quantity, 4);
    assert!(shard.usable);
    assert_eq!(shard.knowledge, ItemKnowledgeDto::Aware);
    assert_eq!(shard.display_name_key, "item-demo-luminous-shard-name");
    assert!(shard.throw_profile.is_some());
    assert_eq!(update.events[0].kind, "item.use-heal");
    assert_eq!(
        update.events[0].args["nameKey"],
        "item-demo-luminous-shard-name"
    );
    assert!(matches!(
        update.events[0].outcome,
        Some(GameEventOutcomeDto::Heal { resolution })
            if resolution.requested == 4 && resolution.applied == 4
    ));
    let restored = Game::from_save(game.to_save()).expect("aware use result should reload");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn unobservable_item_use_consumes_one_but_only_marks_the_kind_tried() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("demo usable stack should exist")
        .location = ItemLocation::Inventory;

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::UseItem {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                target: None,
            },
        ))
        .expect("using an item at full health should execute");

    assert_eq!(update.player.hp, 10);
    let shard = update
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("the remaining stack should stay carried");
    assert_eq!(shard.quantity, 4);
    assert_eq!(shard.knowledge, ItemKnowledgeDto::Tried);
    assert_eq!(shard.display_name_key, "item-demo-unfamiliar-shard-name");
    assert!(shard.throw_profile.is_none());
    assert_eq!(update.events[0].kind, "item.use-no-effect");
    assert_eq!(
        update.events[0].args["nameKey"],
        "item-demo-unfamiliar-shard-name"
    );
    assert!(matches!(
        update.events[0].outcome,
        Some(GameEventOutcomeDto::Heal { resolution })
            if resolution.requested == 4 && resolution.applied == 0
    ));
}

#[test]
fn unusable_inventory_item_is_not_consumed_or_added_to_knowledge() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.echo-charm")
        .expect("demo non-consumable should exist")
        .location = ItemLocation::Inventory;

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::UseItem {
                item_id: "demo.item.echo-charm.1".to_owned(),
                target: None,
            },
        ))
        .expect("an unavailable use attempt should remain a valid action");

    assert_eq!(update.events[0].kind, "item.use-unavailable");
    assert!(
        update
            .inventory
            .iter()
            .any(|item| item.id == "demo.item.echo-charm.1" && item.quantity == 1)
    );
    assert!(game.to_save().item_knowledge.is_empty());
}

#[test]
fn missed_throw_still_lands_at_the_collided_target() {
    let mut game = Game::new(3);
    game.rng = RfbRng::seeded(3);
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("demo throwable stack should exist")
        .location = ItemLocation::Inventory;
    game.entities[0].position = Position { x: 6, y: 3 };
    game.entities[0].hp = 10;

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::Throw {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                direction: Direction::East,
            },
        ))
        .expect("missed throw should execute");

    assert_eq!(update.events[0].kind, "combat.throw-miss");
    assert_eq!(update.entities[0].hp, 10);
    assert!(update.items.iter().any(|item| {
        item.kind_id == "demo.item.luminous-shard" && item.position == Position { x: 6, y: 3 }
    }));
}

#[test]
fn dropping_multiple_selected_stacks_is_atomic_and_deterministic() {
    let mut game = Game::new(42);
    collect_both_demo_items(&mut game);
    let update = game
        .dispatch(command(
            5,
            4,
            GameCommand::Drop {
                item_ids: vec![
                    "demo.item.luminous-shard.1".to_owned(),
                    "demo.item.echo-charm.1".to_owned(),
                ],
            },
        ))
        .expect("batch drop should execute");

    assert!(update.inventory.is_empty());
    assert_eq!(update.items.len(), 5);
    assert!(
        update
            .items
            .iter()
            .filter(|item| {
                item.kind_id != "demo.item.echo-blade"
                    && item.kind_id != "demo.item.resonance-sling"
                    && item.kind_id != "demo.item.resonance-pellet"
            })
            .all(|item| item.position == Position { x: 5, y: 3 })
    );
    assert_eq!(update.changed_cells.len(), 1);
    assert_eq!(update.events[0].message_key, "item-drop-success");
    assert_eq!(update.events[0].args["stacks"], "2");
    assert_eq!(update.events[0].args["quantity"], "6");
}

#[test]
fn pickup_on_empty_ground_is_a_deterministic_turn() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    let before = game.state_hash();
    let update = game
        .dispatch(command(1, 0, GameCommand::PickUp))
        .expect("empty pickup should still execute");

    assert_eq!(update.turn, 1);
    assert!(update.changed_cells.is_empty());
    assert!(update.inventory.is_empty());
    assert_eq!(update.events[0].message_key, "item-pickup-none");
    assert_ne!(update.state_hash, before);
}

#[test]
fn pickup_merges_into_the_lowest_id_compatible_stack() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    game.items.push(ItemInstance {
        id: "demo.inventory.resonance-pellet.1".to_owned(),
        kind_id: "demo.item.resonance-pellet".to_owned(),
        quantity: 19,
        quality: ItemQualityDto::Ordinary,
        affix_ids: Vec::new(),
        rolled_affixes: Vec::new(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        fuel: None,
        device_recovery_progress: 0,
        location: ItemLocation::Inventory,
    });
    game.player.position = Position { x: 6, y: 4 };
    let update = game
        .dispatch(command(1, 0, GameCommand::PickUp))
        .expect("pickup should execute");

    assert_eq!(update.inventory.len(), 2);
    assert_eq!(update.inventory[0].id, "demo.inventory.resonance-pellet.1");
    assert_eq!(update.inventory[0].quantity, 20);
    assert_eq!(update.inventory[1].id, "demo.item.resonance-pellet.1");
    assert_eq!(update.inventory[1].quantity, 5);
}

#[test]
fn partial_drop_allocates_stable_ids_and_survives_save_round_trip() {
    let mut game = Game::new(42);
    game.dispatch(command(
        1,
        0,
        GameCommand::Move {
            direction: Direction::East,
        },
    ))
    .expect("move should execute");
    game.dispatch(command(2, 1, GameCommand::PickUp))
        .expect("pickup should execute");
    let first_drop = game
        .dispatch(command(
            3,
            2,
            GameCommand::DropQuantity {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                quantity: 2,
            },
        ))
        .expect("partial drop should execute");

    assert_eq!(first_drop.inventory[0].quantity, 3);
    assert!(first_drop.items.iter().any(|item| {
        item.id == "generated.item.2"
            && item.quantity == 2
            && item.position == Position { x: 4, y: 3 }
    }));
    assert_eq!(game.next_item_instance_serial, 3);

    let mut restored = Game::from_save(game.to_save()).expect("save should preserve allocator");
    let second_drop = restored
        .dispatch(command(
            4,
            3,
            GameCommand::DropQuantity {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                quantity: 1,
            },
        ))
        .expect("second partial drop should execute");
    assert!(
        second_drop
            .items
            .iter()
            .any(|item| item.id == "generated.item.3" && item.quantity == 1)
    );
    assert_eq!(restored.next_item_instance_serial, 4);
}

#[test]
fn stale_revision_is_rejected_without_mutation() {
    let mut game = Game::new(1);
    let before = game.state_hash();
    let error = game
        .dispatch(command(1, 99, GameCommand::Wait))
        .expect_err("stale command should fail");
    assert!(matches!(error, CoreError::RevisionMismatch { .. }));
    assert_eq!(game.state_hash(), before);
}

#[test]
fn inventory_item_missing_its_kind_is_an_invariant_error() {
    const ITEM_ID: &str = "test.item.missing-kind";
    let mut game = skill_check_game(1, "demo.build.vanguard");
    give_inventory_item(&mut game, ITEM_ID, "demo.item.clarity-draught");
    game.items
        .iter_mut()
        .find(|item| item.id == ITEM_ID)
        .expect("test item should exist")
        .kind_id = "test.item-kind.missing".to_owned();

    assert_invariant_error_without_mutation(
        &mut game,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
        "inventory item test.item.missing-kind references missing kind test.item-kind.missing",
    );
}

#[test]
fn dynamic_item_missing_its_activation_profile_is_an_invariant_error() {
    const ITEM_ID: &str = "test.item.missing-activation";
    let mut game = skill_check_game(1, "demo.build.tinkerer");
    give_inventory_item(&mut game, ITEM_ID, "demo.item.resonance-wand");
    game.items
        .iter_mut()
        .find(|item| item.id == ITEM_ID)
        .and_then(|item| item.activation.as_mut())
        .expect("dynamic test item should carry an activation")
        .profile_id = "test.activation.missing".to_owned();

    assert_invariant_error_without_mutation(
        &mut game,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: Some(TargetSelection::Direction {
                direction: Direction::East,
            }),
        },
        "dynamic item test.item.missing-activation references missing activation profile test.activation.missing",
    );
}

#[test]
fn active_task_missing_its_objective_is_an_invariant_error() {
    let mut game = skill_check_game(1, "demo.build.vanguard");
    let state = game
        .task_states
        .get_mut("demo.task.echo-chain")
        .expect("staged task should exist");
    state.status = TaskStatusKindDto::Active;
    state.active_floor_id = Some(game.current_floor_id.clone());
    state.stage_index = 99;

    assert_invariant_error_without_mutation(
        &mut game,
        GameCommand::Wait,
        "active task demo.task.echo-chain references missing objective stage 99",
    );
}

#[test]
fn offensive_flag_multipliers_and_living_predicate_match_original_tiers() {
    assert_eq!(slay_multiplier(SlayTarget::Evil, SlayLevel::Slay), 19);
    assert_eq!(slay_multiplier(SlayTarget::Animal, SlayLevel::Kill), 46);
    assert_eq!(slay_multiplier(SlayTarget::Dragon, SlayLevel::Slay), 28);
    assert_eq!(slay_multiplier(SlayTarget::Dragon, SlayLevel::Kill), 56);

    let game = Game::new(0);
    let dragon = game
        .content
        .actor("demo.actor.ash-drake")
        .expect("demo dragon");
    let construct = game
        .content
        .actor("demo.actor.resonant-warden")
        .expect("demo construct");
    assert!(slay_target_matches(SlayTarget::Dragon, dragon));
    assert!(slay_target_matches(SlayTarget::Living, dragon));
    assert!(!slay_target_matches(SlayTarget::Living, construct));
}

#[test]
fn elemental_brand_is_suppressed_only_by_matching_immunity() {
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    game.items.push(ItemInstance {
        id: "test.item.ember-edge".to_owned(),
        kind_id: "demo.item.ember-edge".to_owned(),
        quantity: 1,
        quality: ItemQualityDto::Ordinary,
        affix_ids: Vec::new(),
        rolled_affixes: Vec::new(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        fuel: None,
        device_recovery_progress: 0,
        location: ItemLocation::Equipped {
            slot_id: "weapon".to_owned(),
        },
    });
    let profile = game.player_melee_profile(&game.player_derived_stats());
    let definition = game
        .content
        .actor("demo.actor.ash-drake")
        .expect("demo target")
        .clone();
    let mut target = actor_from_runtime_spawn(
        "test.actor.brand-target",
        &definition.id,
        Position { x: 4, y: 3 },
        definition.max_hp,
        definition.speed,
        0,
        true,
    );

    target
        .resistances
        .set(DamageType::Fire, ResistanceLevel::Resistant);
    assert_eq!(
        game.player_melee_damage_multiplier(&profile, &target, &definition),
        24
    );
    target
        .resistances
        .set(DamageType::Fire, ResistanceLevel::Immune);
    assert_eq!(
        game.player_melee_damage_multiplier(&profile, &target, &definition),
        10
    );
}

#[test]
fn offensive_flag_dto_hides_unknown_affix_contributions() {
    let mut game = Game::new(0);
    let item_id = "test.item.known-offense".to_owned();
    game.items.push(ItemInstance {
        id: item_id.clone(),
        kind_id: "demo.item.ember-edge".to_owned(),
        quantity: 1,
        quality: ItemQualityDto::Fine,
        affix_ids: vec!["demo.affix.frost-hunter".to_owned()],
        rolled_affixes: Vec::new(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        fuel: None,
        device_recovery_progress: 0,
        location: ItemLocation::Inventory,
    });

    let hidden = game
        .inventory_dto()
        .into_iter()
        .find(|item| item.id == item_id)
        .expect("test item");
    assert_eq!(hidden.brands, vec![WeaponBrandDto::Fire]);
    assert!(hidden.slays.is_empty());

    game.item_property_knowledge.insert(
        item_id.clone(),
        ItemPropertyKnowledgeState {
            discovered: true,
            appraised: true,
            identified: true,
            known_affix_ids: BTreeSet::from(["demo.affix.frost-hunter".to_owned()]),
        },
    );
    let visible = game
        .inventory_dto()
        .into_iter()
        .find(|item| item.id == item_id)
        .expect("test item");
    assert_eq!(
        visible.brands,
        vec![WeaponBrandDto::Fire, WeaponBrandDto::Cold]
    );
    assert_eq!(
        visible.slays,
        vec![SlayDto {
            target: SlayTargetDto::Animal,
            level: SlayLevelDto::Slay,
        }]
    );
}

#[test]
fn dynamic_affix_rolls_are_seeded_depth_filtered_and_materialized() {
    let roll = |seed, depth| {
        let mut game = Game::new(seed);
        game.roll_affix_properties(&["demo.affix.adaptive-echo".to_owned()], depth)
    };
    let shallow = roll(17, 1);
    assert_eq!(shallow, roll(17, 1));
    assert_eq!(shallow.len(), 1);
    assert!(
        shallow[0].properties.equipment_bonuses.melee_skill == 12
            || shallow[0].properties.equipment_bonuses.melee_attacks == 1
    );
    let deep = roll(17, 10);
    assert_eq!(deep.len(), 1);
    assert!(
        deep[0].properties.equipment_bonuses.device_skill == 8
            || deep[0].properties.equipment_bonuses.melee_attacks == 2
    );
    assert_eq!(
        deep[0].properties.equipment_bonuses.melee_skill, 0,
        "shallow candidates must not leak into deep rolls"
    );
}

#[test]
fn rolled_affix_save_round_trip_does_not_redraw_rng() {
    let mut game = Game::new(23);
    let before_roll = game.rng.draw_counter;
    let rolled = game.roll_affix_properties(&["demo.affix.adaptive-echo".to_owned()], 1);
    assert!(game.rng.draw_counter > before_roll);
    game.items.push(ItemInstance {
        id: "test.item.dynamic-save".to_owned(),
        kind_id: "demo.item.adaptive-glaive".to_owned(),
        quantity: 1,
        quality: ItemQualityDto::Fine,
        affix_ids: vec!["demo.affix.adaptive-echo".to_owned()],
        rolled_affixes: rolled.clone(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        fuel: None,
        device_recovery_progress: 0,
        location: ItemLocation::Inventory,
    });
    let saved = game.to_save();
    let saved_draws = saved.rng.draw_counter;
    let restored = Game::from_save(saved).expect("rolled affix payload should reload");
    let restored_item = restored
        .items
        .iter()
        .find(|item| item.id == "test.item.dynamic-save")
        .expect("dynamic item should survive reload");
    assert_eq!(restored_item.rolled_affixes, rolled);
    assert_eq!(restored.rng.draw_counter, saved_draws);

    let mut legacy = restored.to_save();
    legacy
        .inventory
        .iter_mut()
        .find(|item| item.id == "test.item.dynamic-save")
        .expect("dynamic inventory item")
        .rolled_affixes
        .clear();
    let migrated = Game::from_save(legacy).expect("missing rolled payload is a zero-RNG migration");
    assert_eq!(migrated.rng.draw_counter, saved_draws);
    assert!(
        migrated
            .items
            .iter()
            .find(|item| item.id == "test.item.dynamic-save")
            .expect("legacy dynamic item")
            .rolled_affixes
            .is_empty()
    );
}

#[test]
fn rolled_equipment_bonuses_and_regeneration_are_authoritative() {
    let mut game = Game::new(31);
    clear_monsters(&mut game);
    let properties = AffixPropertyBundleDefinition {
        equipment_bonuses: EquipmentBonuses {
            melee_attacks: 2,
            melee_skill: 11,
            digging_skill: 7,
            ..EquipmentBonuses::default()
        },
        ..AffixPropertyBundleDefinition::default()
    };
    game.items.push(ItemInstance {
        id: "test.item.dynamic-equipped".to_owned(),
        kind_id: "demo.item.adaptive-glaive".to_owned(),
        quantity: 1,
        quality: ItemQualityDto::Fine,
        affix_ids: vec!["demo.affix.adaptive-echo".to_owned()],
        rolled_affixes: vec![RolledAffixState {
            affix_id: "demo.affix.adaptive-echo".to_owned(),
            properties,
        }],
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        fuel: None,
        device_recovery_progress: 0,
        location: ItemLocation::Equipped {
            slot_id: "weapon".to_owned(),
        },
    });
    let item_id = "test.item.dynamic-equipped".to_owned();
    game.item_property_knowledge.insert(
        item_id.clone(),
        ItemPropertyKnowledgeState {
            discovered: true,
            appraised: true,
            identified: true,
            known_affix_ids: BTreeSet::from(["demo.affix.adaptive-echo".to_owned()]),
        },
    );
    let stats = game.player_derived_stats();
    assert_eq!(stats.melee_attacks.value, 3);
    assert!(stats.melee_skill.value >= 11);
    assert!(stats.dig_skill.value >= 7);
    let equipped = game
        .equipment_dto()
        .into_iter()
        .find(|item| item.id == item_id)
        .expect("dynamic item should be visible");
    assert_eq!(equipped.equipment_bonuses.melee_attacks, 2);
    assert_eq!(equipped.passives, vec![EquipmentPassiveDto::Regeneration]);

    game.player.hp = game.effective_player_max_hp() - 2;
    game.world_tick = EQUIPMENT_REGENERATION_INTERVAL_TICKS - 1;
    let before = game.player.hp;
    let update = dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(game.player.hp, before + 1);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "equipment.regenerated")
    );
}
