// SPDX-License-Identifier: MPL-2.0

use super::support::*;
use super::*;
use crate::game::{
    gold::starting_gold, hunger::starting_ration_quantity, lighting::starting_torch_supply,
};

const RATION_KIND_ID: &str = "demo.item.ration-of-food";

#[test]
fn warrior_birth_rolls_five_to_nine_rations_after_gold() {
    for seed in 0..32 {
        let mut expected_rng = RfbRng::seeded(seed);
        let build = CharacterBuildIdentity {
            build_id: RFB_WARRIOR_BUILD_ID.to_owned(),
            race_id: String::new(),
            class_id: String::new(),
            personality_id: String::new(),
        };
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
fn historical_builds_do_not_gain_rations_or_birth_rng_draws() {
    let game =
        Game::new_with_build(11, "demo.build.vanguard").expect("historical build should create");
    assert_eq!(game.gold, 0);
    assert!(game.items.iter().all(|item| item.kind_id != RATION_KIND_ID));
    let build = CharacterBuildIdentity {
        build_id: "demo.build.vanguard".to_owned(),
        race_id: String::new(),
        class_id: String::new(),
        personality_id: String::new(),
    };
    let mut rng = RfbRng::seeded(11);
    let draws_before = rng.draw_counter;
    assert_eq!(starting_gold(Some(&build), &mut rng), 0);
    assert_eq!(starting_ration_quantity(Some(&build), &mut rng), None);
    assert_eq!(rng.draw_counter, draws_before);
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
    let mut game =
        Game::new_with_build(37, "demo.build.scholar").expect("scholar should create with mana");
    clear_monsters(&mut game);
    game.nutrition = 0;
    game.resources
        .get_mut("demo.resource.mana")
        .expect("scholar should have mana")
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
    assert!((5..=33).contains(&normal.player.hp));
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
