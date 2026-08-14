// SPDX-License-Identifier: MPL-2.0

use super::support::*;
use super::*;
use crate::game::{
    gold::starting_gold, hunger::starting_ration_quantity, lighting::starting_torch_supply,
};

const RATION_KIND_ID: &str = "demo.item.ration-of-food";

#[test]
fn warrior_birth_rolls_five_to_nine_rations_after_gold() {
    let content = Game::new(0).content;
    for seed in 0..32 {
        let mut expected_rng = RfbRng::seeded(seed);
        let build = CharacterBuildIdentity {
            build_id: RFB_WARRIOR_BUILD_ID.to_owned(),
            race_id: "demo.race.rfb-human".to_owned(),
            class_id: "demo.class.warrior".to_owned(),
            personality_id: "demo.personality.ordinary".to_owned(),
        };
        let _ = crate::game::virtues::initial_virtues(&content, Some(&build), &mut expected_rng);
        let expected_gold = starting_gold(Some(&build), &mut expected_rng);
        let expected_quantity = starting_ration_quantity(Some(&build), &mut expected_rng)
            .expect("Warrior should receive birth rations");
        let _ = starting_torch_supply(Some(&build), &mut expected_rng)
            .expect("Warrior should receive birth torches after rations");
        let shop_draws_before = expected_rng.draw_counter;

        let game = Game::new_with_build(seed, RFB_WARRIOR_BUILD_ID)
            .expect("Warrens Warrior should create");
        let ration = game
            .items
            .iter()
            .find(|item| item.kind_id == RATION_KIND_ID && item.location == ItemLocation::Inventory)
            .expect("Warrior should carry a ration stack");

        assert_eq!(game.gold, expected_gold);
        assert_eq!(ration.id, "generated.item.1");
        assert_eq!(ration.quantity, expected_quantity);
        assert!((5..=9).contains(&ration.quantity));
        assert_eq!(game.nutrition, rfb_protocol::PLAYER_NUTRITION_BIRTH);
        assert!(game.rng_draw_counter() > shop_draws_before);
    }
}

#[test]
fn formal_golem_birth_replaces_rations_with_a_full_nothing_staff_and_keeps_torches() {
    let game = golem_game(366);
    assert!(
        game.items.iter().all(|item| item.kind_id != RATION_KIND_ID),
        "Golem birth should not create ordinary rations"
    );
    assert!(game.items.iter().any(|item| {
        item.kind_id == "demo.item.wooden-torch" && item.location == ItemLocation::Inventory
    }));
    let staff = game
        .items
        .iter()
        .find(|item| {
            item.kind_id == "demo.item.staff-of-nothing" && item.location == ItemLocation::Inventory
        })
        .expect("Golem should carry its Staff of Nothing");
    assert_eq!(
        staff.charges,
        Some(ItemChargesDto {
            current: 21,
            maximum: 21,
        })
    );
    let activation = staff.activation.as_ref().expect("birth staff activation");
    assert_eq!(activation.profile_id, "demo.device-activation.nothing");
    assert_eq!(activation.cost, 1);
    assert_eq!(staff.device_recovery_progress, 0);
}

#[test]
fn formal_zombie_birth_starts_at_night_without_rations_and_round_trips() {
    let mut game = zombie_game(370);
    assert_eq!(game.world_tick, wilderness::WILDERNESS_NIGHT_START_TICK);
    assert!(!game.wilderness_is_daytime());
    assert!(
        game.items.iter().all(|item| item.kind_id != RATION_KIND_ID),
        "Zombie birth should not create ordinary rations"
    );
    assert!(game.items.iter().any(|item| {
        item.kind_id == "demo.item.wooden-torch" && item.location == ItemLocation::Inventory
    }));
    let staff = game
        .items
        .iter()
        .find(|item| {
            item.kind_id == "demo.item.staff-of-nothing" && item.location == ItemLocation::Inventory
        })
        .expect("Zombie should carry its Staff of Nothing");
    assert_eq!(
        staff.charges,
        Some(ItemChargesDto {
            current: 21,
            maximum: 21,
        })
    );

    let saved = game.to_save();
    let restored = Game::from_save(saved.clone()).expect("night-start Zombie should restore");
    assert_eq!(restored.to_save(), saved);
    assert_eq!(restored.state_hash(), game.state_hash());

    clear_monsters(&mut game);
    let mut replay = game.clone();
    let update = dispatch_next(&mut game, GameCommand::Wait);
    let replay_update = dispatch_next(&mut replay, GameCommand::Wait);
    assert_eq!(replay_update.events, update.events);
    assert_eq!(replay.state_hash(), game.state_hash());
}

#[test]
fn formal_zombie_uses_undead_food_and_device_metabolism() {
    let mut digestion = zombie_game(371);
    digestion.nutrition = 9_000;
    digestion.world_tick = 75_050;
    digestion.process_hunger(&mut Vec::new());
    assert_eq!(digestion.nutrition, 8_995);

    digestion.nutrition = rfb_protocol::PLAYER_NUTRITION_MAXIMUM;
    digestion.world_tick = 75_060;
    digestion.process_hunger(&mut Vec::new());
    assert_eq!(digestion.nutrition, 14_900);

    let mut mushroom = zombie_game(372);
    clear_monsters(&mut mushroom);
    mushroom.nutrition = 9_000;
    mushroom.player.hp = 1;
    give_inventory_item(
        &mut mushroom,
        "test.item.zombie-mushroom",
        "demo.item.fast-recovery-mushroom",
    );
    dispatch_next(
        &mut mushroom,
        GameCommand::UseItem {
            item_id: "test.item.zombie-mushroom".to_owned(),
            target: None,
        },
    );
    assert_eq!(mushroom.nutrition, 9_025);
    assert!(mushroom.player.hp > 1);
    assert!(mushroom.player_has_status_kind(STATUS_REGENERATION));

    let mut absorption = zombie_game(373);
    clear_monsters(&mut absorption);
    absorption.nutrition = 1_000;
    give_inventory_item(
        &mut absorption,
        "test.item.zombie-device",
        "demo.item.detect-objects-staff",
    );
    absorption
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.zombie-device")
        .and_then(|item| item.charges.as_mut())
        .expect("Zombie device charges")
        .current = 2;
    assert!(
        absorption
            .snapshot()
            .inventory
            .iter()
            .find(|item| item.id == "test.item.zombie-device")
            .expect("projected Zombie device")
            .absorbable
    );
    dispatch_next(
        &mut absorption,
        GameCommand::AbsorbDevice {
            item_id: "test.item.zombie-device".to_owned(),
        },
    );
    assert_eq!(absorption.nutrition, 6_000);
    assert_eq!(
        absorption
            .items
            .iter()
            .find(|item| item.id == "test.item.zombie-device")
            .and_then(|item| item.charges)
            .expect("absorbed Zombie device remains")
            .current,
        0
    );
}

#[test]
fn formal_skeleton_birth_starts_at_night_without_rations_and_round_trips() {
    let mut game = skeleton_game(378);
    assert_eq!(game.world_tick, wilderness::WILDERNESS_NIGHT_START_TICK);
    assert!(!game.wilderness_is_daytime());
    assert!(game.items.iter().all(|item| item.kind_id != RATION_KIND_ID));
    assert!(game.items.iter().any(|item| {
        item.kind_id == "demo.item.wooden-torch" && item.location == ItemLocation::Inventory
    }));
    let staff = game
        .items
        .iter()
        .find(|item| {
            item.kind_id == "demo.item.staff-of-nothing" && item.location == ItemLocation::Inventory
        })
        .expect("Skeleton should carry its Staff of Nothing");
    assert_eq!(
        staff.charges,
        Some(ItemChargesDto {
            current: 21,
            maximum: 21,
        })
    );

    let saved = game.to_save();
    let restored = Game::from_save(saved.clone()).expect("night-start Skeleton should restore");
    assert_eq!(restored.to_save(), saved);
    assert_eq!(restored.state_hash(), game.state_hash());

    clear_monsters(&mut game);
    let mut replay = game.clone();
    let update = dispatch_next(&mut game, GameCommand::Wait);
    let replay_update = dispatch_next(&mut replay, GameCommand::Wait);
    assert_eq!(replay_update.events, update.events);
    assert_eq!(replay.state_hash(), game.state_hash());
}

#[test]
fn formal_skeleton_food_magic_drop_rules_and_potion_shatter_match_rfb() {
    let mut game = skeleton_game(379);
    clear_monsters(&mut game);
    game.nutrition = 9_000;

    give_inventory_item(&mut game, "test.item.skeleton-ration", RATION_KIND_ID);
    game.items
        .iter_mut()
        .find(|item| item.id == "test.item.skeleton-ration")
        .expect("Skeleton ration")
        .quantity = 2;
    let ration_update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.skeleton-ration".to_owned(),
            target: None,
        },
    );
    assert_eq!(game.nutrition, 9_000);
    assert!(ration_update.events.iter().any(|event| {
        event.kind == "item.use-food"
            && event.args.get("amount").is_some_and(|amount| amount == "0")
    }));
    assert!(game.items.iter().any(|item| {
        item.kind_id == RATION_KIND_ID
            && item.quantity == 1
            && item.location == ItemLocation::Inventory
    }));
    assert!(game.items.iter().any(|item| {
        item.kind_id == RATION_KIND_ID
            && item.quantity == 1
            && item.location == ItemLocation::Ground(game.player.position)
    }));

    game.player.hp = 1;
    give_inventory_item(
        &mut game,
        "test.item.skeleton-feast",
        "demo.item.sunlit-feast",
    );
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.skeleton-feast".to_owned(),
            target: None,
        },
    );
    assert!(game.player.hp > 1);
    assert_eq!(game.nutrition, 9_000);
    assert!(game.items.iter().any(|item| {
        item.id == "test.item.skeleton-feast"
            && item.location == ItemLocation::Ground(game.player.position)
    }));

    game.player.hp = 1;
    give_inventory_item(
        &mut game,
        "test.item.skeleton-mushroom",
        "demo.item.fast-recovery-mushroom",
    );
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.skeleton-mushroom".to_owned(),
            target: None,
        },
    );
    assert!(game.player.hp > 1);
    assert!(game.player_has_status_kind(STATUS_REGENERATION));
    assert_eq!(game.nutrition, 9_000);
    assert!(
        game.items
            .iter()
            .all(|item| item.id != "test.item.skeleton-mushroom")
    );

    game.player.hp = 1;
    give_inventory_item(
        &mut game,
        "test.item.skeleton-waybread",
        "demo.item.piece-of-elvish-waybread",
    );
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.skeleton-waybread".to_owned(),
            target: None,
        },
    );
    assert!(game.player.hp > 1);
    assert_eq!(game.nutrition, 9_000);
    assert!(
        game.items
            .iter()
            .all(|item| item.id != "test.item.skeleton-waybread")
    );

    game.player.hp = game.effective_player_max_hp();
    game.world_tick = 75_052;
    let hp_before = game.player.hp;
    give_inventory_item(
        &mut game,
        "test.item.skeleton-potion",
        "demo.item.venom-draught",
    );
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.skeleton-potion".to_owned(),
            target: None,
        },
    );
    assert!(game.player.hp < hp_before, "spilled potion should shatter");
    assert_eq!(game.nutrition, 9_000);
    assert!(
        game.items
            .iter()
            .all(|item| item.id != "test.item.skeleton-potion")
    );
}

#[test]
fn formal_skeleton_temporary_form_controls_food_fallthrough() {
    let mut human = Game::new_with_build_race_and_name(
        380,
        "demo.build.warrior",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Human warrior should create");
    clear_monsters(&mut human);
    human.nutrition = 9_000;
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.skeleton-form").status;
    form.granted_race_id = Some("rfb-legacy.race.skeleton".to_owned());
    human.player.statuses.push(form);
    give_inventory_item(&mut human, "test.item.skeleton-form-ration", RATION_KIND_ID);
    dispatch_next(
        &mut human,
        GameCommand::UseItem {
            item_id: "test.item.skeleton-form-ration".to_owned(),
            target: None,
        },
    );
    assert_eq!(human.nutrition, 9_000);
    assert!(human.items.iter().any(|item| {
        item.id == "test.item.skeleton-form-ration"
            && item.location == ItemLocation::Ground(human.player.position)
    }));

    human
        .player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    give_inventory_item(&mut human, "test.item.human-ration", RATION_KIND_ID);
    dispatch_next(
        &mut human,
        GameCommand::UseItem {
            item_id: "test.item.human-ration".to_owned(),
            target: None,
        },
    );
    assert_eq!(human.nutrition, 14_000);
    assert!(
        human
            .items
            .iter()
            .all(|item| item.id != "test.item.human-ration")
    );
}

#[test]
fn ration_use_consumes_one_restores_food_and_pays_normal_action_cost() {
    let mut game =
        Game::new_with_build(7, RFB_WARRIOR_BUILD_ID).expect("Warrens Warrior should create");
    game.entities.clear();
    game.nutrition = rfb_protocol::PLAYER_NUTRITION_BIRTH;
    let ration = game
        .items
        .iter()
        .find(|item| item.kind_id == RATION_KIND_ID)
        .expect("Warrior should carry rations");
    let ration_id = ration.id.clone();
    let quantity_before = ration.quantity;

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: ration_id,
            target: None,
        },
    );

    assert_eq!(game.nutrition, 14_999);
    assert_eq!(game.world_tick, 10);
    assert_eq!(game.turn, 1);
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.kind_id == RATION_KIND_ID)
            .map(|item| item.quantity),
        Some(quantity_before - 1)
    );
    let event = update
        .events
        .iter()
        .find(|event| event.kind == "item.use-food")
        .expect("eating should report nutrition");
    assert_eq!(event.args["amount"], "5000");
    assert_eq!(event.args["nutrition"], "14999");
}

#[test]
fn ration_caps_at_maximum_before_bloated_world_processing() {
    let mut game =
        Game::new_with_build(9, RFB_WARRIOR_BUILD_ID).expect("Warrens Warrior should create");
    game.entities.clear();
    game.nutrition = 14_000;
    let ration_id = game
        .items
        .iter()
        .find(|item| item.kind_id == RATION_KIND_ID)
        .expect("Warrior should carry rations")
        .id
        .clone();
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: ration_id,
            target: None,
        },
    );
    let food = update
        .events
        .iter()
        .find(|event| event.kind == "item.use-food")
        .expect("eating should report capped nutrition");
    assert_eq!(food.args["amount"], "1000");
    assert_eq!(food.args["nutrition"], "15000");
    assert_eq!(game.nutrition, 14_900);
}

#[test]
fn satisfy_hunger_sets_food_to_the_original_maximum_minus_one() {
    let mut game = Game::new(31);
    clear_monsters(&mut game);
    game.nutrition = 123;
    give_inventory_item(
        &mut game,
        "test.item.satisfy-hunger.1",
        "demo.item.satisfy-hunger-scroll",
    );

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.satisfy-hunger.1".to_owned(),
            target: None,
        },
    );

    assert_eq!(game.nutrition, rfb_protocol::PLAYER_NUTRITION_MAXIMUM - 1);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.use-hunger-satisfied")
    );
}

#[test]
fn hallucination_food_applies_status_drains_mana_then_adds_nutrition() {
    let mut game = test_caster_game(37);
    clear_monsters(&mut game);
    game.nutrition = 0;
    game.resources
        .get_mut("demo.resource.mana")
        .expect("test caster should have mana")
        .current = 50;
    give_inventory_item(
        &mut game,
        "test.item.hallucination.1",
        "demo.item.hallucination-mushroom",
    );

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.hallucination.1".to_owned(),
            target: None,
        },
    );

    assert!(game.player_has_status_kind(crate::effect::STATUS_HALLUCINATION));
    assert_eq!(game.resources["demo.resource.mana"].current, 0);
    assert_eq!(game.nutrition, 500);
    let effects = update
        .events
        .iter()
        .filter(|event| {
            matches!(
                event.kind.as_str(),
                "item.use-status-applied" | "item.use-resource-drained" | "item.use-food"
            )
        })
        .map(|event| event.kind.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        effects,
        [
            "item.use-status-applied",
            "item.use-resource-drained",
            "item.use-food"
        ]
    );
}

#[test]
fn sleep_potion_uses_the_paralysis_status() {
    let mut game = Game::new(41);
    clear_monsters(&mut game);
    give_inventory_item(&mut game, "test.item.sleep.1", "demo.item.sleep-potion");

    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.sleep.1".to_owned(),
            target: None,
        },
    );

    assert!(game.player_has_status_kind(STATUS_PARALYSIS));
    assert!(!game.player_has_status_kind(STATUS_SLEEP));
    let remaining = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_PARALYSIS)
        .expect("sleep should leave paralysis active")
        .remaining_ticks;
    assert!((10..=40).contains(&remaining));
    assert_eq!(remaining % 10, 0);
}

#[test]
fn elvish_waybread_uses_normal_and_intolerant_branches() {
    const ITEM_KIND_ID: &str = "demo.item.piece-of-elvish-waybread";

    let mut normal = Game::new(43);
    clear_monsters(&mut normal);
    normal.nutrition = 123;
    normal.player.hp = 1;
    normal.player.statuses.push(StatusInstance {
        kind_id: STATUS_POISON.to_owned(),
        intensity: 1,
        remaining_ticks: 2_000,
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
    give_inventory_item(&mut normal, "test.item.waybread.normal", ITEM_KIND_ID);

    let update = dispatch_next(
        &mut normal,
        GameCommand::UseItem {
            item_id: "test.item.waybread.normal".to_owned(),
            target: None,
        },
    );

    assert_eq!(normal.nutrition, rfb_protocol::PLAYER_NUTRITION_MAXIMUM - 1);
    assert!(
        (4..=33).contains(&normal.player.hp),
        "Waybread left the player at {} HP",
        normal.player.hp
    );
    let poison_reduction = update
        .events
        .iter()
        .find(|event| event.kind == "item.use-status-reduced")
        .expect("normal Waybread should reduce poison");
    assert_eq!(poison_reduction.args["before"], "2000");
    assert_eq!(poison_reduction.args["after"], "1000");
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.use-heal")
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.use-hunger-satisfied")
    );

    let mut intolerant = Game::new(47);
    clear_monsters(&mut intolerant);
    assert!(intolerant.gain_mutation("rfb.mutation.waybread-into", &mut Vec::new()));
    assert!(intolerant.player_levitates());
    intolerant.nutrition = rfb_protocol::PLAYER_NUTRITION_BIRTH;
    intolerant.player.statuses.push(StatusInstance {
        kind_id: STATUS_POISON.to_owned(),
        intensity: 1,
        remaining_ticks: 2_000,
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
    let hp_before = intolerant.player.hp;
    give_inventory_item(
        &mut intolerant,
        "test.item.waybread.intolerant",
        ITEM_KIND_ID,
    );

    let update = dispatch_next(
        &mut intolerant,
        GameCommand::UseItem {
            item_id: "test.item.waybread.intolerant".to_owned(),
            target: None,
        },
    );

    assert_eq!(
        intolerant.nutrition,
        crate::game::hunger::NUTRITION_STARVING - 1
    );
    assert_eq!(intolerant.player.hp, hp_before);
    assert!(!intolerant.player_has_status_kind(STATUS_POISON));
    assert!(intolerant.player_has_status_kind(STATUS_PARALYSIS));
    assert!(
        !update
            .events
            .iter()
            .any(|event| event.kind == "item.use-heal")
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.use-status-removed")
    );
}

#[test]
fn salt_water_affects_living_players_but_is_inert_for_nonliving_players() {
    const ITEM_KIND_ID: &str = "demo.item.salt-water";
    assert!(!Game::salt_water_affects_race(
        "rfb-legacy.race.mon-jelly",
        &[]
    ));
    assert!(Game::salt_water_affects_race(
        "rfb-legacy.race.einheri",
        &["nonliving".to_owned()]
    ));
    let poison = || StatusInstance {
        kind_id: STATUS_POISON.to_owned(),
        intensity: 1,
        remaining_ticks: 2_000,
        source_id: None,
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    };

    let mut living = Game::new_with_build(59, RFB_WARRIOR_BUILD_ID)
        .expect("living Salt Water test should create");
    clear_monsters(&mut living);
    living.nutrition = rfb_protocol::PLAYER_NUTRITION_BIRTH;
    living.player.statuses.push(poison());
    give_inventory_item(&mut living, "test.item.salt-water.living", ITEM_KIND_ID);
    dispatch_next(
        &mut living,
        GameCommand::UseItem {
            item_id: "test.item.salt-water.living".to_owned(),
            target: None,
        },
    );
    assert_eq!(
        living.nutrition,
        crate::game::hunger::NUTRITION_STARVING - 1
    );
    assert!(living.player_has_status_kind(STATUS_PARALYSIS));
    assert!(!living.player_has_status_kind(STATUS_POISON));
    assert!(
        living
            .items
            .iter()
            .all(|item| item.id != "test.item.salt-water.living")
    );

    let mut nonliving = Game::new_with_build(61, RFB_WARRIOR_BUILD_ID)
        .expect("nonliving Salt Water test should create");
    clear_monsters(&mut nonliving);
    nonliving
        .build
        .as_mut()
        .expect("test build should exist")
        .race_id = "demo.race.vampire-lord".to_owned();
    nonliving.nutrition = rfb_protocol::PLAYER_NUTRITION_BIRTH;
    nonliving.player.statuses.push(poison());
    give_inventory_item(
        &mut nonliving,
        "test.item.salt-water.nonliving",
        ITEM_KIND_ID,
    );
    dispatch_next(
        &mut nonliving,
        GameCommand::UseItem {
            item_id: "test.item.salt-water.nonliving".to_owned(),
            target: None,
        },
    );
    assert_eq!(nonliving.nutrition, rfb_protocol::PLAYER_NUTRITION_BIRTH);
    assert!(!nonliving.player_has_status_kind(STATUS_PARALYSIS));
    assert!(nonliving.player_has_status_kind(STATUS_POISON));
    assert!(
        nonliving
            .items
            .iter()
            .all(|item| item.id != "test.item.salt-water.nonliving")
    );
}

#[test]
fn fast_recovery_mushroom_heals_eases_bleeding_and_grants_timed_regeneration() {
    const ITEM_KIND_ID: &str = "demo.item.fast-recovery-mushroom";
    const ITEM_ID: &str = "test.item.fast-recovery-mushroom";
    let mut game = Game::new(53);
    clear_monsters(&mut game);
    game.player.hp = 1;
    game.player.statuses.push(StatusInstance {
        kind_id: STATUS_BLEEDING.to_owned(),
        intensity: 1,
        remaining_ticks: 100,
        source_id: Some("test.bleeding".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    give_inventory_item(&mut game, ITEM_ID, ITEM_KIND_ID);
    let draws_before = game.rng_draw_counter();

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );

    assert!((3..=17).contains(&game.player.hp));
    assert!(!game.player_has_status_kind(STATUS_BLEEDING));
    let regeneration = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_REGENERATION)
        .expect("Fast Recovery should grant regeneration");
    assert!((1_010..=2_000).contains(&regeneration.remaining_ticks));
    assert_eq!(game.player_regeneration_rate_percent(), 200);
    assert_eq!(game.rng_draw_counter(), draws_before + 3);
    assert!(update.events.iter().any(|event| {
        event.kind == "item.use-status-reduced"
            && event.args.get("before").is_some_and(|value| value == "100")
            && event.args.get("after").is_some_and(|value| value == "0")
    }));
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.use-status-applied")
    );

    let restored = Game::from_save(game.to_save()).expect("regeneration should round-trip");
    assert_eq!(restored.snapshot(), game.snapshot());
    assert_eq!(restored.player_regeneration_rate_percent(), 200);
}

#[test]
fn digestion_uses_world_tick_and_current_scheduler_speed() {
    let mut normal = Game::new(42);
    normal.nutrition = 9_000;
    normal.world_tick = 50;
    normal.process_hunger(&mut Vec::new());
    assert_eq!(normal.nutrition, 8_990);

    let mut fast = Game::new(42);
    fast.nutrition = 9_000;
    fast.player.speed = 120;
    fast.world_tick = 50;
    fast.process_hunger(&mut Vec::new());
    assert_eq!(fast.nutrition, 8_980);

    let mut bloated = Game::new(42);
    bloated.nutrition = rfb_protocol::PLAYER_NUTRITION_MAXIMUM;
    bloated.world_tick = 10;
    bloated.process_hunger(&mut Vec::new());
    assert_eq!(bloated.nutrition, 14_900);
}

#[test]
fn golem_slow_digestion_and_food_magic_follow_construct_metabolism() {
    let mut digestion = golem_game(360);
    digestion.nutrition = 9_000;
    digestion.world_tick = 50;
    digestion.process_hunger(&mut Vec::new());
    assert_eq!(digestion.nutrition, 8_995);

    digestion.nutrition = rfb_protocol::PLAYER_NUTRITION_MAXIMUM;
    digestion.world_tick = 60;
    digestion.process_hunger(&mut Vec::new());
    assert_eq!(digestion.nutrition, 14_900);

    let mut mushroom = golem_game(361);
    clear_monsters(&mut mushroom);
    mushroom.nutrition = 9_000;
    mushroom.player.hp = 1;
    give_inventory_item(
        &mut mushroom,
        "test.item.golem-mushroom",
        "demo.item.fast-recovery-mushroom",
    );
    let update = dispatch_next(
        &mut mushroom,
        GameCommand::UseItem {
            item_id: "test.item.golem-mushroom".to_owned(),
            target: None,
        },
    );
    assert_eq!(mushroom.nutrition, 9_025);
    assert!(mushroom.player.hp > 1);
    assert!(mushroom.player_has_status_kind(STATUS_REGENERATION));
    assert!(update.events.iter().any(|event| {
        event.kind == "item.use-food"
            && event
                .args
                .get("amount")
                .is_some_and(|amount| amount == "25")
    }));

    let mut waybread = golem_game(362);
    clear_monsters(&mut waybread);
    waybread.nutrition = 9_000;
    waybread.player.hp = 1;
    waybread.player.statuses.push(StatusInstance {
        kind_id: STATUS_POISON.to_owned(),
        intensity: 1,
        remaining_ticks: 2_000,
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
    give_inventory_item(
        &mut waybread,
        "test.item.golem-waybread",
        "demo.item.piece-of-elvish-waybread",
    );
    dispatch_next(
        &mut waybread,
        GameCommand::UseItem {
            item_id: "test.item.golem-waybread".to_owned(),
            target: None,
        },
    );
    assert_eq!(waybread.nutrition, 9_375);
    assert!(waybread.player.hp > 1);
    assert_eq!(
        waybread
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_POISON)
            .expect("Waybread should reduce rather than remove this poison")
            .remaining_ticks,
        990
    );
}

#[test]
fn golem_absorbs_inventory_and_floor_devices_without_consuming_them() {
    let mut human =
        Game::new_with_build(363, RFB_WARRIOR_BUILD_ID).expect("living Warrior should create");
    give_inventory_item(
        &mut human,
        "test.item.human-device",
        "demo.item.detect-objects-staff",
    );
    assert!(
        !human
            .snapshot()
            .inventory
            .iter()
            .find(|item| item.id == "test.item.human-device")
            .expect("projected human device")
            .absorbable
    );

    let mut game = golem_game(363);
    clear_monsters(&mut game);
    game.nutrition = 1_000;
    give_inventory_item(
        &mut game,
        "test.item.golem-device.pack",
        "demo.item.detect-objects-staff",
    );
    let pack = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.golem-device.pack")
        .expect("pack device");
    pack.charges.as_mut().expect("device charges").current = 2;
    assert!(
        game.snapshot()
            .inventory
            .iter()
            .find(|item| item.id == "test.item.golem-device.pack")
            .expect("projected pack device")
            .absorbable
    );

    let update = dispatch_next(
        &mut game,
        GameCommand::AbsorbDevice {
            item_id: "test.item.golem-device.pack".to_owned(),
        },
    );
    assert_eq!(game.nutrition, 6_000);
    assert_eq!(game.world_tick, 10);
    assert_eq!(game.turn, 1);
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.item.golem-device.pack")
            .and_then(|item| item.charges)
            .expect("absorbed device remains")
            .current,
        0
    );
    assert!(update.events.iter().any(|event| {
        event.kind == "item.device-absorbed"
            && event.args.get("amount").is_some_and(|amount| amount == "2")
            && event
                .args
                .get("chargesAfter")
                .is_some_and(|charges| charges == "0")
    }));

    give_inventory_item(
        &mut game,
        "test.item.golem-device.floor",
        "demo.item.detection-rod",
    );
    let floor = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.golem-device.floor")
        .expect("floor device");
    let cost = floor.activation.as_ref().expect("device activation").cost;
    floor.charges.as_mut().expect("device charges").current = cost + 3;
    floor.location = ItemLocation::Ground(game.player.position);
    game.item_property_knowledge
        .entry("test.item.golem-device.floor".to_owned())
        .or_default()
        .discovered = true;
    assert!(
        game.snapshot()
            .items
            .iter()
            .find(|item| item.id == "test.item.golem-device.floor")
            .expect("projected floor device")
            .absorbable
    );
    game.nutrition = rfb_protocol::PLAYER_NUTRITION_MAXIMUM - 100;
    let capped = dispatch_next(
        &mut game,
        GameCommand::AbsorbDevice {
            item_id: "test.item.golem-device.floor".to_owned(),
        },
    );
    assert!(capped.events.iter().any(|event| {
        event.kind == "item.device-absorbed"
            && event.args.get("nutritionAfter").is_some_and(|nutrition| {
                nutrition == &rfb_protocol::PLAYER_NUTRITION_MAXIMUM.to_string()
            })
    }));
    assert_eq!(game.nutrition, rfb_protocol::PLAYER_NUTRITION_MAXIMUM - 100);
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.item.golem-device.floor")
            .and_then(|item| item.charges)
            .expect("floor device remains")
            .current,
        3
    );
}

#[test]
fn golem_device_absorption_round_trips_and_replays_deterministically() {
    let mut game = golem_game(369);
    clear_monsters(&mut game);
    game.nutrition = 1_000;
    give_inventory_item(
        &mut game,
        "test.item.golem-device.replay",
        "demo.item.detect-objects-staff",
    );
    game.items
        .iter_mut()
        .find(|item| item.id == "test.item.golem-device.replay")
        .and_then(|item| item.charges.as_mut())
        .expect("replay device charges")
        .current = 2;

    let hash_before = game.state_hash();
    let mut replay = game.clone();
    let command = GameCommand::AbsorbDevice {
        item_id: "test.item.golem-device.replay".to_owned(),
    };
    let update = dispatch_next(&mut game, command.clone());
    let replay_update = dispatch_next(&mut replay, command);
    assert_eq!(replay_update.events, update.events);
    assert_ne!(game.state_hash(), hash_before);
    assert_eq!(replay.state_hash(), game.state_hash());

    let saved = game.to_save();
    let restored = Game::from_save(saved.clone()).expect("absorbed Golem device should restore");
    assert_eq!(restored.to_save(), saved);
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn empty_device_absorption_spends_a_turn_without_changing_energy_or_food() {
    let mut game = golem_game(364);
    clear_monsters(&mut game);
    game.nutrition = 9_000;
    give_inventory_item(
        &mut game,
        "test.item.golem-device.empty",
        "demo.item.detect-objects-staff",
    );
    game.items
        .iter_mut()
        .find(|item| item.id == "test.item.golem-device.empty")
        .and_then(|item| item.charges.as_mut())
        .expect("empty device charges")
        .current = 0;

    let update = dispatch_next(
        &mut game,
        GameCommand::AbsorbDevice {
            item_id: "test.item.golem-device.empty".to_owned(),
        },
    );
    assert_eq!(game.nutrition, 9_000);
    assert_eq!(game.world_tick, 10);
    assert_eq!(game.turn, 1);
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.item.golem-device.empty")
            .and_then(|item| item.charges)
            .expect("empty device remains")
            .current,
        0
    );
    assert!(update.events.iter().any(|event| {
        event.kind == "item.device-absorbed"
            && event.args.get("amount").is_some_and(|amount| amount == "0")
            && event.message_key == "item-device-empty"
    }));
}

#[test]
fn ultimate_resistance_slow_digestion_halves_normal_food_use() {
    let mut game = Game::new(42);
    game.player.statuses.push(
        monster_combat::melee_status(STATUS_ULTIMATE_RESISTANCE, 10, "test.ultimate-resistance")
            .status,
    );
    game.nutrition = 9_000;
    game.world_tick = 50;
    game.process_hunger(&mut Vec::new());
    assert_eq!(game.nutrition, 8_995);
}

#[test]
fn wait_and_rest_share_the_hunger_world_clock() {
    let mut waiting = Game::new(42);
    waiting.entities.clear();
    waiting.nutrition = 9_000;
    waiting.world_tick = 40;
    dispatch_next(&mut waiting, GameCommand::Wait);
    assert_eq!(waiting.world_tick, 50);
    assert_eq!(waiting.nutrition, 8_990);

    let mut resting = Game::new(42);
    resting.entities.clear();
    resting.nutrition = 9_000;
    resting.player.hp = 1;
    dispatch_next(&mut resting, GameCommand::Rest { turns: 20 });
    assert!(resting.world_tick >= 50);
    assert!(resting.nutrition < 9_000);
}

#[test]
fn nutrition_thresholds_apply_rfb_regeneration_factors() {
    let mut game = Game::new(42);
    for (nutrition, expected) in [
        (1_000, 197),
        (999, 98),
        (500, 98),
        (499, 33),
        (100, 33),
        (99, 0),
    ] {
        game.nutrition = nutrition;
        assert_eq!(game.nutrition_regeneration_factor(), expected);
    }
}

#[test]
fn fainting_rolls_once_and_skips_rng_while_already_paralyzed() {
    let mut fainted = None;
    for seed in 0..1_000 {
        let mut game = Game::new(42);
        game.nutrition = 499;
        game.world_tick = 10;
        game.rng = RfbRng::seeded(seed);
        let mut events = Vec::new();
        game.process_hunger(&mut events);
        if events
            .iter()
            .any(|event| matches!(event, DomainEvent::PlayerFaintedFromHunger { .. }))
        {
            fainted = Some(game);
            break;
        }
    }
    let mut game = fainted.expect("a bounded seed should trigger the ten-percent faint roll");
    assert!(game.player_has_status_kind(STATUS_PARALYSIS));
    assert_eq!(game.rng.draw_counter, 2);

    game.world_tick = 20;
    let draws_before = game.rng.draw_counter;
    game.process_hunger(&mut Vec::new());
    assert_eq!(game.rng.draw_counter, draws_before);
}

#[test]
fn starvation_damage_precedes_recovery_and_can_kill() {
    let mut game = Game::new(42);
    game.nutrition = 0;
    game.player.hp = 4;
    game.world_tick = 10;
    game.rng = RfbRng::seeded(1);
    let mut events = Vec::new();
    game.process_hunger(&mut events);

    assert_eq!(game.player.hp, -6);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::PlayerDamagedByStarvation { damage } if damage.applied == 10
    )));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::PlayerDiedFromStarvation { .. }))
    );
}

#[test]
fn nutrition_round_trips() {
    let mut game =
        Game::new_with_build(17, RFB_WARRIOR_BUILD_ID).expect("Warrens Warrior should create");
    game.nutrition = 321;
    let restored = Game::from_save(game.to_save()).expect("nutrition should round trip");
    assert_eq!(restored.nutrition, 321);
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn warrens_ration_attempts_are_deterministic_walkable_and_persistent() {
    let mut saw_guaranteed_tail = false;
    let mut saw_other_tail = false;
    for seed in 1..=32 {
        let mut left = Game::new_with_build(seed, RFB_WARRIOR_BUILD_ID)
            .expect("Warrens Warrior should create");
        let mut right = Game::new_with_build(seed, RFB_WARRIOR_BUILD_ID)
            .expect("matching Warrens Warrior should create");
        descend_one_floor(&mut left);
        descend_one_floor(&mut right);
        assert_eq!(left.items, right.items);
        let ground = left
            .items
            .iter()
            .filter(|item| matches!(item.location, ItemLocation::Ground(_)))
            .collect::<Vec<_>>();
        for ration in ground.iter().filter(|item| item.kind_id == RATION_KIND_ID) {
            let ItemLocation::Ground(position) = ration.location else {
                unreachable!()
            };
            assert!(left.is_walkable(position));
        }
        if let Some(last) = ground.iter().max_by_key(|item| generated_serial(&item.id)) {
            if last.kind_id == RATION_KIND_ID {
                saw_guaranteed_tail = true;
            } else {
                saw_other_tail = true;
            }
        }
        let restored = Game::from_save(left.to_save()).expect("generated rations should reload");
        assert_eq!(restored.state_hash(), left.state_hash());
    }
    assert!(saw_guaranteed_tail && saw_other_tail);
}

fn generated_serial(id: &str) -> Option<u64> {
    id.strip_prefix(GENERATED_ITEM_ID_PREFIX)?.parse().ok()
}
