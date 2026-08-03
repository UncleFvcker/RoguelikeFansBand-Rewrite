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

        let game = Game::new_warrens_journey_with_build(seed, RFB_WARRIOR_BUILD_ID)
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
    let mut game = Game::new_warrens_journey_with_build(7, RFB_WARRIOR_BUILD_ID)
        .expect("Warrens Warrior should create");
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
    let mut game = Game::new_warrens_journey_with_build(9, RFB_WARRIOR_BUILD_ID)
        .expect("Warrens Warrior should create");
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
fn nutrition_round_trips_and_v157_missing_field_migrates_without_side_effects() {
    let mut game = Game::new_warrens_journey_with_build(17, RFB_WARRIOR_BUILD_ID)
        .expect("Warrens Warrior should create");
    game.nutrition = 321;
    let restored = Game::from_save(game.to_save()).expect("nutrition should round trip");
    assert_eq!(restored.nutrition, 321);
    assert_eq!(restored.state_hash(), game.state_hash());

    let mut payload = game.to_save();
    payload.content_hash =
        "70f21e8d8f28a2102a8b28e5c6cabf83137afb4532e5c2868d10fb7c1e5e5012".to_owned();
    payload.items.retain(|item| item.kind_id != RATION_KIND_ID);
    payload
        .inventory
        .retain(|item| item.kind_id != RATION_KIND_ID);
    payload
        .equipment
        .retain(|item| item.kind_id != RATION_KIND_ID);
    payload
        .carried_items
        .retain(|item| item.kind_id != RATION_KIND_ID);
    let revision_before = payload.revision;
    let turn_before = payload.turn;
    let tick_before = payload.world_tick;
    let draws_before = payload.rng.draw_counter;
    let mut json = serde_json::to_value(payload).expect("save should encode as JSON");
    json["player"]
        .as_object_mut()
        .expect("player save should be an object")
        .remove("nutrition");
    let legacy: SavePayloadV1 = serde_json::from_value(json).expect("legacy save should decode");

    let migrated = Game::from_save(legacy).expect("v157 save should migrate nutrition");
    assert_eq!(migrated.nutrition, rfb_protocol::PLAYER_NUTRITION_BIRTH);
    assert!(
        migrated
            .items
            .iter()
            .all(|item| item.kind_id != RATION_KIND_ID)
    );
    assert_eq!(migrated.revision, revision_before);
    assert_eq!(migrated.turn, turn_before);
    assert_eq!(migrated.world_tick, tick_before);
    assert_eq!(migrated.rng.draw_counter, draws_before);
}

#[test]
fn warrens_ration_attempts_are_deterministic_walkable_and_persistent() {
    let mut saw_guaranteed_tail = false;
    let mut saw_other_tail = false;
    for seed in 1..=32 {
        let mut left = Game::new_warrens_journey_with_build(seed, RFB_WARRIOR_BUILD_ID)
            .expect("Warrens Warrior should create");
        let mut right = Game::new_warrens_journey_with_build(seed, RFB_WARRIOR_BUILD_ID)
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
