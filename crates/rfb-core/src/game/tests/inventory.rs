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
        permanent_destruction_immunities: Default::default(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        fuel: None,
        device_recovery_progress: 0,
        captured_actor: None,
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
        permanent_destruction_immunities: Default::default(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        fuel: None,
        device_recovery_progress: 0,
        captured_actor: None,
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
    assert_eq!(equipped.dig_skill.value, baseline.dig_skill.value + 46);
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
fn original_diggers_use_weight_and_tunneling_pval_without_stacking_with_weapons() {
    for (kind_id, expected) in [
        ("demo.item.shovel", 46),
        ("demo.item.pick", 55),
        ("demo.item.gnomish-shovel", 66),
        ("demo.item.orcish-pick", 75),
    ] {
        let mut game = Game::new(42);
        game.items.clear();
        let baseline = game.player_derived_stats().dig_skill.value;
        give_inventory_item(&mut game, "test.digger", kind_id);
        dispatch_next(
            &mut game,
            GameCommand::Equip {
                item_id: "test.digger".to_owned(),
                slot_id: Some("tool".to_owned()),
            },
        );
        assert_eq!(
            game.player_derived_stats().dig_skill.value,
            baseline + expected
        );
    }

    let mut game = Game::new(42);
    game.items.clear();
    let baseline = game.player_derived_stats().dig_skill.value;
    give_inventory_item(&mut game, "test.weapon", "demo.item.broad-sword");
    dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: "test.weapon".to_owned(),
            slot_id: Some("right-hand".to_owned()),
        },
    );
    give_inventory_item(&mut game, "test.tool", "demo.item.orcish-pick");
    dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: "test.tool".to_owned(),
            slot_id: Some("tool".to_owned()),
        },
    );
    assert_eq!(game.player_derived_stats().dig_skill.value, baseline + 75);
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
        permanent_destruction_immunities: Default::default(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        fuel: None,
        device_recovery_progress: 0,
        captured_actor: None,
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
        permanent_destruction_immunities: Default::default(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        fuel: None,
        device_recovery_progress: 0,
        captured_actor: None,
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

fn p88b_add_item(
    game: &mut Game,
    id: &str,
    kind_id: &str,
    quantity: u32,
    location: ItemLocation,
    affix_ids: &[&str],
) {
    give_inventory_item(game, id, kind_id);
    let item = game
        .items
        .iter_mut()
        .find(|item| item.id == id)
        .expect("P88B item should exist");
    item.quantity = quantity;
    item.location = location;
    item.affix_ids = affix_ids.iter().map(|id| (*id).to_owned()).collect();
}

#[test]
fn p88b_protection_quiver_skips_quivered_ammunition_without_rng() {
    let mut game = Game::new(0);
    game.items.clear();
    p88b_add_item(
        &mut game,
        "p88b.quiver",
        "demo.item.quiver",
        1,
        ItemLocation::Equipped {
            slot_id: "quiver".to_owned(),
        },
        &["rfb-legacy.affix.quiver-protection"],
    );
    p88b_add_item(
        &mut game,
        "p88b.arrows",
        "demo.item.arrow",
        60,
        ItemLocation::Inventory,
        &[],
    );
    assert_eq!(game.inventory_used_slots(), 0);

    let draws = game.rng_draw_counter();
    let mut events = Vec::new();
    game.damage_player_inventory("test.acid", DamageType::Acid, false, 1, &mut events);

    assert_eq!(game.rng_draw_counter(), draws);
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "p88b.arrows")
            .unwrap()
            .quantity,
        60
    );
    assert!(events.is_empty());

    let mut expected = game.clone();
    let _nuke_resistance_roll = expected.rng.bounded(55);
    game.damage_player_inventory("test.nuke", DamageType::Nuke, false, 1, &mut events);
    assert_eq!(game.rng_draw_counter(), draws + 1);
    assert_eq!(game.rng.bounded(10_000), expected.rng.bounded(10_000));
    assert!(events.is_empty());
}

#[test]
fn p88b_quiver_overflow_remains_vulnerable_and_emits_partial_destruction() {
    let mut game = Game::new(0);
    game.items.clear();
    p88b_add_item(
        &mut game,
        "p88b.quiver",
        "demo.item.quiver",
        1,
        ItemLocation::Equipped {
            slot_id: "quiver".to_owned(),
        },
        &["rfb-legacy.affix.quiver-protection"],
    );
    for id in ["p88b.arrows-a", "p88b.arrows-b"] {
        p88b_add_item(
            &mut game,
            id,
            "demo.item.arrow",
            60,
            ItemLocation::Inventory,
            &[],
        );
    }
    assert_eq!(game.inventory_used_slots(), 1);

    let mut events = Vec::new();
    game.resolve_monster_damage_to_player(
        "test.monster",
        "test.monster-kind",
        "test.acid-bolt",
        0,
        1,
        1,
        DamageType::Acid,
        &mut events,
    );

    let protected = game
        .items
        .iter()
        .find(|item| item.id == "p88b.arrows-a")
        .expect("quivered arrows should remain");
    let overflow = game
        .items
        .iter()
        .find(|item| item.id == "p88b.arrows-b")
        .expect("partially destroyed overflow should remain");
    assert_eq!(protected.quantity, 60);
    assert!(overflow.quantity < 60);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::InventoryItemDestroyedByDamage {
            source_kind_id,
            target_kind_id,
            quantity,
        } if source_kind_id == "test.monster-kind"
            && target_kind_id == "demo.item.arrow"
            && *quantity == 60 - overflow.quantity
    )));
}

#[test]
fn p88b_protection_boundaries_preserve_other_destruction_rules() {
    let mut touch = Game::new(0);
    touch.items.clear();
    p88b_add_item(
        &mut touch,
        "p88b.touch-arrows",
        "demo.item.arrow",
        60,
        ItemLocation::Inventory,
        &[],
    );
    let draws = touch.rng_draw_counter();
    touch.damage_player_inventory("test.plasma", DamageType::Plasma, true, 1, &mut Vec::new());
    assert_eq!(touch.rng_draw_counter(), draws);

    let mut enduring = Game::new(0);
    enduring.items.clear();
    p88b_add_item(
        &mut enduring,
        "p88b.enduring-arrows",
        "demo.item.arrow",
        60,
        ItemLocation::Inventory,
        &["rfb-legacy.affix.endurance"],
    );
    let draws = enduring.rng_draw_counter();
    enduring.damage_player_inventory("test.acid", DamageType::Acid, false, 1, &mut Vec::new());
    assert_eq!(enduring.rng_draw_counter(), draws);

    let mut protected = Game::new(0);
    protected.items.clear();
    p88b_add_item(
        &mut protected,
        "p88b.protected-status-arrows",
        "demo.item.arrow",
        60,
        ItemLocation::Inventory,
        &[],
    );
    protected
        .player
        .statuses
        .push(monster_combat::melee_status(STATUS_INVENTORY_PROTECTION, 10, "test.status").status);
    let draws = protected.rng_draw_counter();
    protected.damage_player_inventory("test.acid", DamageType::Acid, false, 1, &mut Vec::new());
    assert!(protected.rng_draw_counter() > draws);
    assert_eq!(protected.items[0].quantity, 60);

    let mut manual = Game::new(0);
    manual.items.clear();
    p88b_add_item(
        &mut manual,
        "p88b.manual-quiver",
        "demo.item.quiver",
        1,
        ItemLocation::Equipped {
            slot_id: "quiver".to_owned(),
        },
        &["rfb-legacy.affix.quiver-protection"],
    );
    p88b_add_item(
        &mut manual,
        "p88b.manual-arrows",
        "demo.item.arrow",
        10,
        ItemLocation::Inventory,
        &[],
    );
    assert_eq!(
        manual
            .destroy_item("p88b.manual-arrows", 1)
            .unwrap()
            .quantity,
        1
    );
}

#[test]
fn p88e_protection_quiver_does_not_cover_unequipped_fired_or_ground_ammunition() {
    let mut unequipped = Game::new(0);
    unequipped.items.clear();
    p88b_add_item(
        &mut unequipped,
        "p88e.unequipped-quiver",
        "demo.item.quiver",
        1,
        ItemLocation::Inventory,
        &["rfb-legacy.affix.quiver-protection"],
    );
    p88b_add_item(
        &mut unequipped,
        "p88e.unequipped-arrows",
        "demo.item.arrow",
        60,
        ItemLocation::Inventory,
        &[],
    );
    let draws = unequipped.rng_draw_counter();
    unequipped.damage_player_inventory("test.acid", DamageType::Acid, false, 1, &mut Vec::new());
    assert!(unequipped.rng_draw_counter() > draws);
    assert!(
        unequipped
            .items
            .iter()
            .find(|item| item.id == "p88e.unequipped-arrows")
            .is_none_or(|item| item.quantity < 60)
    );

    let mut fired = Game::new(0);
    fired.items.clear();
    p88b_add_item(
        &mut fired,
        "p88e.fired-quiver",
        "demo.item.quiver",
        1,
        ItemLocation::Equipped {
            slot_id: "quiver".to_owned(),
        },
        &["rfb-legacy.affix.quiver-protection"],
    );
    p88b_add_item(
        &mut fired,
        "p88e.fired-arrows",
        "demo.item.arrow",
        10,
        ItemLocation::Inventory,
        &[],
    );
    let ammunition = fired
        .take_inventory_item("p88e.fired-arrows")
        .expect("taking ammunition should succeed")
        .expect("ammunition should exist");
    let landing = fired.player.position;
    let mut fired_events = Vec::new();
    fired.settle_projectile_ammunition(
        ammunition,
        landing,
        true,
        100,
        &mut fired_events,
        &mut BTreeSet::new(),
    );
    assert_eq!(
        fired
            .items
            .iter()
            .find(|item| item.id == "p88e.fired-arrows")
            .expect("the remaining stack should exist")
            .quantity,
        9
    );
    assert!(fired_events.iter().any(|event| matches!(
        event,
        DomainEvent::ProjectileAmmoBroken { ammo_kind_id }
            if ammo_kind_id == "demo.item.arrow"
    )));

    let mut ground = Game::new(0);
    ground.items.clear();
    p88b_add_item(
        &mut ground,
        "p88e.ground-quiver",
        "demo.item.quiver",
        1,
        ItemLocation::Equipped {
            slot_id: "quiver".to_owned(),
        },
        &["rfb-legacy.affix.quiver-protection"],
    );
    let ground_position = ground.player.position;
    p88b_add_item(
        &mut ground,
        "p88e.ground-arrows",
        "demo.item.arrow",
        10,
        ItemLocation::Ground(ground_position),
        &[],
    );
    let mut ground_events = Vec::new();
    ground.resolve_ground_item_projectile_effects(
        "test.fire",
        &[ground_position],
        DamageType::Fire,
        true,
        &mut ground_events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    assert!(
        ground
            .items
            .iter()
            .all(|item| item.id != "p88e.ground-arrows")
    );
    assert!(ground_events.iter().any(|event| matches!(
        event,
        DomainEvent::GroundItemDestroyedByAbility {
            item_id,
            target_kind_id,
            quantity: 10,
            ..
        } if item_id == "p88e.ground-arrows" && target_kind_id == "demo.item.arrow"
    )));
}
