// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

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
fn armor_hit_modifier_only_changes_melee_skill() {
    let mut game = Game::new(42);
    game.items.clear();
    let baseline = game.player_derived_stats();
    game.items.push(ItemInstance {
        id: "test.item.hard-leather-armour".to_owned(),
        kind_id: "demo.item.hard-leather-armour".to_owned(),
        quantity: 1,
        inscription: None,
        origin_actor_kind_id: None,
        origin_kind: None,
        damage_dice_override: None,
        discount_percent: 0,
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
        inscription: None,
        origin_actor_kind_id: None,
        origin_kind: None,
        damage_dice_override: None,
        discount_percent: 0,
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
        game.items.clear();
        give_inventory_item(&mut game, "test.item.launcher", launcher_kind);
        give_inventory_item(&mut game, "test.item.ammunition", ammo_kind);
        game.items
            .iter_mut()
            .find(|item| item.id == "test.item.launcher")
            .expect("test launcher should exist")
            .location = ItemLocation::Equipped {
            slot_id: "shooting".to_owned(),
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
fn pickup_on_empty_ground_is_zero_time() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    game.items.clear();
    let before = game.state_hash();
    let world_tick = game.world_tick;
    let update = game
        .dispatch(command(1, 0, GameCommand::PickUp))
        .expect("empty pickup should still execute");

    assert_eq!(update.turn, 1);
    assert_eq!(update.world_tick, world_tick);
    assert!(update.changed_cells.is_empty());
    assert!(update.inventory.is_empty());
    assert_eq!(update.events[0].message_key, "item-pickup-none");
    assert_ne!(update.state_hash, before);
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
    let mut game = skill_check_game(1, "demo.build.warrior");
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
fn elemental_brand_is_suppressed_only_by_matching_immunity() {
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    game.items.push(ItemInstance {
        id: "test.item.ember-edge".to_owned(),
        kind_id: "demo.item.ember-edge".to_owned(),
        quantity: 1,
        inscription: None,
        origin_actor_kind_id: None,
        origin_kind: None,
        damage_dice_override: None,
        discount_percent: 0,
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
        inscription: None,
        origin_actor_kind_id: None,
        origin_kind: None,
        damage_dice_override: None,
        discount_percent: 0,
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
