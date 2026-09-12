// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::tests::support::{command, dispatch_next, replace_terrain, rest_resolution};

fn body(game: &mut Game, id: &str, kind: &str, slot: u8) {
    give_inventory_item(game, id, kind);
    let category = game
        .absorbed_device_category(game.items.last().unwrap())
        .unwrap();
    game.items.last_mut().unwrap().location = ItemLocation::Absorbed { category, slot };
    game.identify_item_instance(id, ItemIdentificationRequest::new(true));
    let item = game.items.last_mut().unwrap();
    let charges = item.charges.as_mut().unwrap();
    charges.current = charges.maximum;
}

fn item<'a>(game: &'a Game, id: &str) -> &'a ItemInstance {
    game.items.iter().find(|item| item.id == id).unwrap()
}

fn sp(game: &mut Game, id: &str, current: u32) {
    game.items
        .iter_mut()
        .find(|item| item.id == id)
        .unwrap()
        .charges
        .as_mut()
        .unwrap()
        .current = current;
}

fn check_seed(game: &Game, id: &str, succeeds: bool) -> (RfbRng, RfbRng) {
    let item = item(game, id);
    let activation = item.activation.as_ref().unwrap();
    let definition = game.content.item(&item.kind_id).unwrap();
    let profile = item_device_generation(
        &game.content,
        &item.kind_id,
        &item.affix_ids,
        Some(&activation.profile_id),
        false,
    )
    .unwrap()
    .activations
    .iter()
    .find(|profile| profile.id == activation.profile_id)
    .unwrap();
    let context = game.item_device_check_context(
        item,
        definition,
        &profile.effect,
        activation.device_check_difficulty,
    );
    (0..1000)
        .find_map(|seed| {
            let before = RfbRng::seeded(seed);
            let mut after = before.clone();
            (resolve_check(&mut after, context.clone()).succeeded() == succeeds)
                .then_some((before, after))
        })
        .unwrap()
}

fn use_body(game: &mut Game, id: &str, targets: &[TargetSelection]) -> (i32, Vec<DomainEvent>) {
    let mut events = Vec::new();
    let energy = game
        .use_absorbed_device(
            id,
            targets,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    (energy, events)
}

fn status(game: &mut Game, kind: &str) {
    game.player.statuses.push(StatusInstance {
        kind_id: kind.to_owned(),
        intensity: 1,
        remaining_ticks: 1000,
        source_id: None,
        granted_resistances: Default::default(),
        granted_brands: Default::default(),
        granted_modifiers: Default::default(),
        granted_equipment_bonuses: Default::default(),
        granted_status_immunities: Default::default(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
}

#[test]
fn three_categories_use_the_existing_effects_and_instance_sp() {
    for (kind, target) in [
        (
            "demo.item.magic-missile-wand",
            Some(TargetSelection::Direction {
                direction: Direction::East,
            }),
        ),
        ("demo.item.detect-objects-staff", None),
        ("demo.item.detection-rod", None),
    ] {
        let mut absorbed = at_level(1);
        body(&mut absorbed, "test.device", kind, 0);
        if kind == "demo.item.magic-missile-wand" {
            let ego = absorbed
                .content
                .affix_definitions()
                .find(|affix| {
                    affix
                        .rfb_ego
                        .as_ref()
                        .is_some_and(|ego| ego.source_index == 254)
                })
                .unwrap();
            let materialized = crate::game::ego::materialize_device(
                &absorbed.content,
                &mut absorbed.rng,
                absorbed.content.item(kind).unwrap(),
                100,
                true,
                crate::game::loot::ItemGenerationMode::Ordinary,
                Some(ego),
            )
            .unwrap();
            materialized.apply_to(
                absorbed
                    .items
                    .iter_mut()
                    .find(|item| item.id == "test.device")
                    .unwrap(),
            );
            absorbed.identify_item_instance("test.device", ItemIdentificationRequest::new(true));
        }
        absorbed.player.position = Position { x: 10, y: 10 };
        for x in 10..=18 {
            replace_terrain(&mut absorbed, Position { x, y: 10 }, "demo.terrain.floor");
        }
        absorbed.push_generated_actor(
            "test.target".into(),
            "demo.actor.sheep",
            Position { x: 11, y: 10 },
        );
        give_inventory_item(&mut absorbed, "test.loot", "demo.item.ration-of-food");
        absorbed.items.last_mut().unwrap().location =
            ItemLocation::Ground(Position { x: 12, y: 10 });
        absorbed.rng = check_seed(&absorbed, "test.device", true).0;
        let mut ordinary = absorbed.clone();
        ordinary
            .items
            .iter_mut()
            .find(|item| item.id == "test.device")
            .unwrap()
            .location = ItemLocation::Inventory;
        let before = item(&absorbed, "test.device").charges.unwrap().current;
        let cost = item(&absorbed, "test.device")
            .activation
            .as_ref()
            .unwrap()
            .cost;
        let targets = target.clone().into_iter().collect::<Vec<_>>();
        let (energy, events) = use_body(&mut absorbed, "test.device", &targets);
        let mut ordinary_events = Vec::new();
        ordinary
            .use_inventory_item(
                "test.device",
                target.as_ref(),
                None,
                &mut ordinary_events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        assert_eq!(energy, 100);
        assert_eq!(events, ordinary_events, "{kind}");
        assert_eq!(absorbed.entities, ordinary.entities);
        assert_eq!(absorbed.rng, ordinary.rng);
        assert_eq!(
            item(&absorbed, "test.device").charges.unwrap().current,
            before - cost
        );
        assert!(absorbed.resources.is_empty());
    }
}

#[test]
fn empty_sp_is_checked_after_failure_and_cancellation_refunds_time_without_rng_rollback() {
    let mut base = at_level(1);
    body(&mut base, "test.wand", "demo.item.magic-missile-wand", 0);
    for succeeds in [false, true] {
        let mut game = base.clone();
        sp(&mut game, "test.wand", 0);
        let (before, after) = check_seed(&game, "test.wand", succeeds);
        game.rng = before;
        let (energy, events) = use_body(&mut game, "test.wand", &[]);
        assert_eq!(energy, if succeeds { 0 } else { 100 });
        assert_eq!(game.rng, after);
        assert_eq!(item(&game, "test.wand").charges.unwrap().current, 0);
        assert!(events.iter().any(|event| matches!(event, DomainEvent::DeviceSkillChecked { succeeded, .. } if *succeeded == succeeds)));
    }
    let (before_rng, after_rng) = check_seed(&base, "test.wand", true);
    base.rng = before_rng;
    base.sniper_concentration = 3;
    let before = (
        base.turn,
        base.world_tick,
        base.player.energy_need,
        item(&base, "test.wand").clone(),
    );
    dispatch_next(
        &mut base,
        GameCommand::UseAbsorbedDevice {
            item_id: "test.wand".into(),
            targets: vec![],
        },
    );
    assert_eq!(
        (
            base.turn,
            base.world_tick,
            base.player.energy_need,
            item(&base, "test.wand").clone()
        ),
        before
    );
    assert_eq!(base.sniper_concentration, 3);
    assert_eq!(base.rng, after_rng);
}

#[test]
fn identification_checks_once_and_pays_only_for_actual_targets_up_to_available_sp() {
    let mut game = at_level(1);
    body(
        &mut game,
        "test.identify",
        "demo.item.detect-objects-staff",
        0,
    );
    let profile = game
        .content
        .item("demo.item.detect-objects-staff")
        .unwrap()
        .device_generation
        .as_ref()
        .unwrap()
        .activations
        .iter()
        .find(|profile| profile.id == "rfb.device-activation.staff.identify")
        .unwrap()
        .clone();
    let activation = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.identify")
        .unwrap()
        .activation
        .as_mut()
        .unwrap();
    activation.profile_id = profile.id;
    activation.name_key = profile.name_key;
    activation.cost = profile.charges.cost;
    activation.device_check_difficulty = profile.device_check_difficulty;
    activation.power = profile.min_depth;
    activation.target_spec = target_spec_dto(&profile.target);
    let cost = activation.cost;
    sp(&mut game, "test.identify", 2 * cost);
    for id in ["test.first", "test.second", "test.third"] {
        give_inventory_item(&mut game, id, "demo.item.ration-of-food");
    }
    let (before, after) = check_seed(&game, "test.identify", true);
    game.rng = before;
    let targets = ["test.first", "test.first", "test.second", "test.third"].map(|item_id| {
        TargetSelection::Item {
            item_id: item_id.into(),
        }
    });
    let (energy, events) = use_body(&mut game, "test.identify", &targets);
    assert_eq!(energy, 100);
    assert_eq!(item(&game, "test.identify").charges.unwrap().current, 0);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, DomainEvent::ItemIdentified { .. }))
            .count(),
        2
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, DomainEvent::DeviceSkillChecked { .. }))
            .count(),
        1
    );
    assert!(
        game.item_property_knowledge
            .get("test.second")
            .unwrap()
            .appraised
    );
    assert!(
        game.item_property_knowledge
            .get("test.third")
            .is_none_or(|knowledge| !knowledge.appraised)
    );
    assert_eq!(game.rng, after);
}

#[test]
fn blocked_and_stale_body_commands_preserve_state_and_projection_explains_the_gate() {
    for kind in [
        STATUS_CONFUSION,
        STATUS_BERSERK,
        crate::effect::STATUS_ANTI_MAGIC,
    ] {
        let mut game = at_level(1);
        body(&mut game, "test.wand", "demo.item.magic-missile-wand", 0);
        status(&mut game, kind);
        let projected = game.snapshot().player.magic_eater.unwrap().slots[0]
            .item
            .clone()
            .unwrap();
        assert!(!projected.usable);
        assert!(projected.use_unavailable_reason.is_some());
        let before = game.to_save();
        assert!(
            game.dispatch(command(
                game.last_command_seq + 1,
                game.revision,
                GameCommand::UseAbsorbedDevice {
                    item_id: "test.wand".into(),
                    targets: vec![]
                }
            ))
            .is_err()
        );
        assert_eq!(game.to_save(), before);
    }
    let mut game = at_level(1);
    let before = game.to_save();
    assert!(
        game.dispatch(command(
            game.last_command_seq + 1,
            game.revision,
            GameCommand::UseAbsorbedDevice {
                item_id: "missing".into(),
                targets: vec![]
            }
        ))
        .is_err()
    );
    assert_eq!(game.to_save(), before);
}

#[test]
fn fear_failure_precedes_the_device_check_and_keeps_sp() {
    let mut game = at_level(1);
    body(&mut game, "test.wand", "demo.item.magic-missile-wand", 0);
    status(&mut game, STATUS_FEAR);
    let seed = (0..1000)
        .find(|seed| (5..10).contains(&RfbRng::seeded(*seed).bounded(100)))
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    let mut expected_rng = game.rng.clone();
    expected_rng.bounded(100);
    let before = item(&game, "test.wand").clone();
    let (energy, events) = use_body(&mut game, "test.wand", &[]);
    assert_eq!(energy, 100);
    assert_eq!(item(&game, "test.wand"), &before);
    assert_eq!(game.rng, expected_rng);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::PlayerFearBlocked { .. }))
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::DeviceSkillChecked { .. }))
    );
}

#[test]
fn recovery_uses_source_rounding_stable_slots_and_saved_fraction_even_outside_rest() {
    let mut game = at_level(1);
    body(&mut game, "test.wand", "demo.item.magic-missile-wand", 8);
    body(&mut game, "test.staff", "demo.item.detect-objects-staff", 2);
    body(&mut game, "test.rod", "demo.item.resonance-rod", 9);
    for item in &mut game.items {
        if matches!(item.location, ItemLocation::Absorbed { .. }) {
            let charges = item.charges.as_mut().unwrap();
            charges.current = 0;
        } else if let Some(charges) = item.charges.as_mut() {
            charges.current = charges.maximum;
        }
    }
    game.items.reverse();
    game.world_tick = 9;
    let mut expected_rng = game.rng.clone();
    game.process_inventory_device_recovery(&mut Vec::new());
    assert_eq!(game.rng, expected_rng);
    game.world_tick = 10;
    let wand_fraction = if expected_rng.bounded(1000) < 500 {
        140
    } else {
        130
    };
    let staff_fraction = if expected_rng.bounded(1000) < 300 {
        70
    } else {
        60
    };
    // The 18-SP rod gains exactly 27 hundredths; it still consumes the rounding draw.
    expected_rng.bounded(1000);
    game.process_inventory_device_recovery(&mut Vec::new());
    assert_eq!(
        item(&game, "test.wand").device_recovery_progress,
        wand_fraction
    );
    assert_eq!(
        item(&game, "test.staff").device_recovery_progress,
        staff_fraction
    );
    assert_eq!(
        (
            item(&game, "test.rod").charges.unwrap().current,
            item(&game, "test.rod").device_recovery_progress
        ),
        (0, 270)
    );
    assert_eq!(game.rng, expected_rng);
    game.player.position = Position { x: 10, y: 10 };
    for x in 10..=11 {
        replace_terrain(&mut game, Position { x, y: 10 }, "demo.terrain.floor");
    }
    game.reveal_current_visibility();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    let before = item(&game, "test.rod").charges.unwrap().current;
    for step in 0..20 {
        let command = match step % 3 {
            0 => GameCommand::Move {
                direction: Direction::East,
            },
            1 => GameCommand::Move {
                direction: Direction::West,
            },
            _ => GameCommand::Wait,
        };
        assert_eq!(
            dispatch_next(&mut game, command.clone()),
            dispatch_next(&mut restored, command)
        );
    }
    assert!(item(&game, "test.rod").charges.unwrap().current > before);
    assert_eq!(game.to_save(), restored.to_save());
    assert_eq!(game.state_hash(), restored.state_hash());
}

#[test]
fn speed_and_regeneration_egos_keep_their_body_semantics() {
    let mut game = at_level(1);
    body(&mut game, "test.wand", "demo.item.magic-missile-wand", 0);
    let ego = game
        .content
        .affix_definitions()
        .find(|affix| {
            affix
                .rfb_ego
                .as_ref()
                .is_some_and(|ego| ego.source_index == 256)
        })
        .unwrap();
    let materialized = crate::game::ego::materialize_device(
        &game.content,
        &mut game.rng,
        game.content.item("demo.item.magic-missile-wand").unwrap(),
        100,
        true,
        crate::game::loot::ItemGenerationMode::Ordinary,
        Some(ego),
    )
    .unwrap();
    materialized.apply_to(
        game.items
            .iter_mut()
            .find(|item| item.id == "test.wand")
            .unwrap(),
    );
    game.identify_item_instance("test.wand", ItemIdentificationRequest::new(true));
    let pval = crate::game::ego::device_pval(item(&game, "test.wand"));
    game.rng = check_seed(&game, "test.wand", false).0;
    assert_eq!(
        use_body(&mut game, "test.wand", &[]).0,
        100 - 10 * i32::from(pval)
    );
    let slot = &game.snapshot().player.magic_eater.unwrap().slots[0];
    assert_eq!(slot.energy_cost, Some(100 - 10 * i32::from(pval)));
    assert!(
        slot.failure_per_mille
            .is_some_and(|rate| (50..=950).contains(&rate))
    );

    body(&mut game, "test.rod", "demo.item.resonance-rod", 0);
    let ego = game
        .content
        .affix("rfb-legacy.affix.regeneration-device")
        .unwrap();
    let materialized = crate::game::ego::materialize_device(
        &game.content,
        &mut game.rng,
        game.content.item("demo.item.resonance-rod").unwrap(),
        100,
        true,
        crate::game::loot::ItemGenerationMode::Ordinary,
        Some(ego),
    )
    .unwrap();
    materialized.apply_to(
        game.items
            .iter_mut()
            .find(|item| item.id == "test.rod")
            .unwrap(),
    );
    let pval = crate::game::ego::device_pval(item(&game, "test.rod"));
    assert_eq!(
        game.absorbed_device_recovery_per_mille(item(&game, "test.rod")),
        15 + pval * 3
    );
    status(&mut game, crate::effect::STATUS_REGENERATION);
    assert_eq!(
        game.absorbed_device_recovery_per_mille(item(&game, "test.rod")),
        20 + pval * 4
    );
    assert_eq!(
        game.absorbed_device_recovery_per_mille(item(&game, "test.wand")),
        4
    );
}

#[test]
fn rest_includes_body_sp_and_stops_at_full_or_visible_danger() {
    let mut game = at_level(1);
    body(&mut game, "test.rod", "demo.item.detection-rod", 0);
    let maximum = item(&game, "test.rod").charges.unwrap().maximum;
    sp(&mut game, "test.rod", maximum - 1);
    assert_eq!(game.player.hp, game.effective_player_max_hp());
    assert!(game.resources.is_empty());
    let update = dispatch_next(&mut game, GameCommand::Rest { turns: 100 });
    assert_eq!(
        rest_resolution(&update).stop_reason,
        RestStopReasonDto::FullResources
    );
    assert!(rest_resolution(&update).completed_turns > 0);
    assert_eq!(item(&game, "test.rod").charges.unwrap().current, maximum);
    assert_eq!(item(&game, "test.rod").device_recovery_progress, 0);
    let rng = game.rng.clone();
    game.process_inventory_device_recovery(&mut Vec::new());
    assert_eq!(game.rng, rng);
    sp(&mut game, "test.rod", 0);
    game.player.position = Position { x: 10, y: 10 };
    replace_terrain(&mut game, Position { x: 10, y: 10 }, "demo.terrain.floor");
    replace_terrain(&mut game, Position { x: 11, y: 10 }, "demo.terrain.floor");
    game.push_generated_actor(
        "test.danger".into(),
        "demo.actor.sheep",
        Position { x: 11, y: 10 },
    );
    game.reveal_current_visibility();
    let update = dispatch_next(&mut game, GameCommand::Rest { turns: 100 });
    assert_eq!(
        rest_resolution(&update).stop_reason,
        RestStopReasonDto::EnemyVisible
    );
    assert_eq!(rest_resolution(&update).completed_turns, 0);
}
