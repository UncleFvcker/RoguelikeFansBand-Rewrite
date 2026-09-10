// SPDX-License-Identifier: MPL-2.0

use super::support::{clear_monsters, give_inventory_item, replace_terrain};
use super::*;
use crate::game::ability_projection::target_spec_dto;

const AFFIX_ID: &str = "test.affix.riding-charge";
const ACTIVATION_ID: &str = "test.device-activation.riding-charge";
const ITEM_ID: &str = "test.item.riding-charge";
const ABILITY_EFFECT_AFFIX_ID: &str = "test.affix.ability-effect";
const ABILITY_EFFECT_ACTIVATION_ID: &str = "test.device-activation.ability-effect";
const ABILITY_EFFECT_ITEM_ID: &str = "test.item.ability-effect";

#[test]
fn mattock_forced_base_disruption_activation_round_trips() {
    let mut game = Game::new_with_build(67, RFB_WARRIOR_BUILD_ID).unwrap();
    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack_root).unwrap();
    let mut table = artifact
        .content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .unwrap()
        .clone();
    table.id = "test.loot-table.forced-mattock".into();
    // This test fixes the base kind and exercises materialization.
    table.kind_selection = None;
    table
        .entries
        .retain(|entry| entry.item_kind_id == "demo.item.mattock");
    table.entries[0].min_depth = 0;
    table.affix_weights.retain(|entry| entry.affix_id.is_none());
    artifact.content.loot_tables.push(table);
    game.content = std::sync::Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content).unwrap(),
    ));
    clear_monsters(&mut game);
    let context = LootContext {
        table_id: "test.loot-table.forced-mattock".to_owned(),
        floor_id: game.current_floor_id.clone(),
        depth: 50,
        source: LootSource::MonsterDeath {
            actor_id: "test.loot-source".to_owned(),
        },
    };
    // This seed reaches Disruption after the natural artifact gate.
    game.rng = RfbRng::seeded(136);
    let mut drops = game
        .generate_loot_instances(&context, ItemLocation::Inventory)
        .unwrap();
    assert_eq!(drops.len(), 1);
    let item = drops.remove(0);
    assert_eq!(item.kind_id, "demo.item.mattock");
    assert_eq!(item.quality, ItemQualityDto::Exceptional);
    assert_eq!(item.affix_ids, ["rfb-legacy.affix.disruption"]);
    assert_eq!(item.rolled_affixes.len(), 1);
    let rolled = &item.rolled_affixes[0];
    assert_eq!(
        rolled.melee_damage_dice,
        Some(rfb_protocol::MeleeDamageDiceDto { dice: 3, sides: 9 })
    );
    assert_eq!(rolled.properties.modifiers.strength, 3);
    assert_eq!(rolled.properties.equipment_bonuses.digging_skill, 3);
    assert!(item.enchantments.to_damage > 0);
    assert_eq!(
        item.activation.as_ref().unwrap().profile_id,
        "rfb.device-activation.ego-42-stone-to-mud"
    );
    assert_eq!(item.charges.unwrap().current, 1);
    let item_id = item.id.clone();
    game.items.push(item);

    let save = game.to_save();
    let mut game = Game::from_save_with_content(save.clone(), game.content.clone())
        .expect("natural ego rolls and activation should survive a save round-trip");
    assert_eq!(game.to_save(), save);
    game.use_inventory_item(
        &item_id,
        None,
        None,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(
        game.to_save(),
        save,
        "missing direction must preserve item, resources, and RNG"
    );

    let target = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    replace_terrain(&mut game, target, "demo.terrain.quartz-vein");
    game.rng = RfbRng::seeded(0);
    let mut events = Vec::new();
    game.use_inventory_item(
        &item_id,
        Some(&TargetSelection::Direction {
            direction: Direction::East,
        }),
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(
        events.iter().any(|event| matches!(
            event,
            DomainEvent::DeviceSkillChecked {
                succeeded: true,
                ..
            }
        )),
        "{events:?}"
    );
    assert_eq!(
        game.terrain[game.index(target).unwrap()],
        "demo.terrain.floor"
    );
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == item_id)
            .unwrap()
            .charges
            .unwrap()
            .current,
        0
    );
    for _ in 0..50 {
        game.world_tick += 1;
        game.process_inventory_device_recovery(&mut events);
    }
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == item_id)
            .unwrap()
            .charges
            .unwrap()
            .current,
        1
    );
}

fn riding_charge_game(seed: u64) -> Game {
    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should be inside the workspace")
        .join("packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
    let mut affix = artifact
        .content
        .affixes
        .iter()
        .find(|affix| affix.id == "rfb-legacy.affix.combat-ring")
        .expect("test source affix should exist")
        .clone();
    affix.id = AFFIX_ID.to_owned();
    affix.name_key = "test-affix-riding-charge-name".to_owned();
    affix.description_key = "test-affix-riding-charge-description".to_owned();
    affix.rfb_ego = None;
    affix.device_generation = Some(rfb_content::ItemDeviceGenerationDefinition {
        activation_optional: false,
        activations: vec![rfb_content::ItemDeviceActivationDefinition {
            rfb_value: Some(0),
            id: ACTIVATION_ID.to_owned(),
            name_key: "test-device-activation-riding-charge-name".to_owned(),
            weight: 1,
            min_depth: 1,
            max_depth: 100,
            device_check_difficulty: 1,
            rfb_biases: BTreeSet::new(),
            charges: rfb_content::ItemDeviceChargeRangeDefinition {
                minimum: 1,
                maximum: 1,
                cost: 1,
            },
            recovery: Some(rfb_content::ItemDeviceRecoveryDefinition {
                interval_ticks: 1_000,
                energy_per_mille: 1_000,
            }),
            target: AbilityTargetDefinition {
                modes: vec![
                    AbilityTargetModeDefinition::Direction,
                    AbilityTargetModeDefinition::Entity,
                ],
                range: 7,
                requires_line_of_effect: true,
            },
            effect_program_id: None,
            effect: ItemUseEffectDefinition::RidingCharge,
        }],
        recovery: None,
    });
    artifact.content.affixes.push(affix);
    artifact.content.worlds[0]
        .dungeons
        .iter_mut()
        .find(|d| d.id == "demo.dungeon.castle")
        .unwrap()
        .no_melee = true;
    let content = Arc::new(ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content)
            .expect("riding-charge test content should remain valid"),
    ));
    let mut game =
        Game::from_content_with_build(seed, content, DEFAULT_WORLD_ID, "demo.build.cavalry")
            .expect("riding-charge test game should create");
    clear_monsters(&mut game);
    give_inventory_item(&mut game, ITEM_ID, "demo.item.long-sword");
    let target_spec = target_spec_dto(
        &game
            .content
            .affix(AFFIX_ID)
            .expect("test affix should exist")
            .device_generation
            .as_ref()
            .expect("test affix should activate")
            .activations[0]
            .target,
    );
    let item = game
        .items
        .iter_mut()
        .find(|item| item.id == ITEM_ID)
        .expect("test weapon should exist");
    item.affix_ids = vec![AFFIX_ID.to_owned()];
    item.activation = Some(ItemActivationDto {
        profile_id: ACTIVATION_ID.to_owned(),
        name_key: "test-device-activation-riding-charge-name".to_owned(),
        power: 1,
        cost: 1,
        device_check_difficulty: 1,
        target_spec,
    });
    item.charges = Some(ItemChargesDto {
        current: 1,
        maximum: 1,
    });
    game
}

fn place_charge_target(game: &mut Game) -> Position {
    let origin = game.player.position;
    let target = Position {
        x: origin.x + 3,
        y: origin.y,
    };
    for offset in 0..=3 {
        replace_terrain(
            game,
            Position {
                x: origin.x + offset,
                y: origin.y,
            },
            "demo.terrain.floor",
        );
    }
    game.push_generated_actor(
        "test.charge-target".to_owned(),
        "demo.actor.dread-vampire",
        target,
    );
    target
}

fn ability_effect_game(seed: u64) -> Game {
    activation_effect_game(seed, "rfb-legacy.affix.craft", "resist-fire", 12)
}

fn activation_effect_game(seed: u64, affix_id: &str, effect: &str, weight: u16) -> Game {
    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should be inside the workspace")
        .join("packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
    let mut affix = artifact
        .content
        .affixes
        .iter()
        .find(|affix| affix.id == affix_id)
        .expect("Craft should expose biased activation candidates")
        .clone();
    let mut activation = affix
        .device_generation
        .as_ref()
        .expect("Craft activation candidates")
        .activations
        .iter()
        .find(|activation| activation.id.ends_with(effect))
        .expect("Craft should include Resist Fire")
        .clone();
    assert!(matches!(
        activation.effect,
        ItemUseEffectDefinition::AbilityEffect { .. }
    ));
    activation.id = ABILITY_EFFECT_ACTIVATION_ID.to_owned();
    activation.device_check_difficulty = 1;
    activation.min_depth = 1;
    activation.max_depth = 100;
    affix.id = ABILITY_EFFECT_AFFIX_ID.to_owned();
    affix.name_key = "test-affix-ability-effect-name".to_owned();
    affix.description_key = "test-affix-ability-effect-description".to_owned();
    affix.rfb_ego = None;
    affix.device_generation = Some(rfb_content::ItemDeviceGenerationDefinition {
        activation_optional: false,
        activations: vec![activation.clone()],
        recovery: None,
    });
    artifact.content.affixes.push(affix);
    artifact
        .content
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.iron-shot")
        .expect("iron shot must exist")
        .weight_tenths_pound = weight;
    let content = Arc::new(ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content)
            .expect("ability-effect test content should remain valid"),
    ));
    let mut game =
        Game::from_content_with_build(seed, content, DEFAULT_WORLD_ID, RFB_WARRIOR_BUILD_ID)
            .expect("ability-effect test game should create");
    give_inventory_item(&mut game, ABILITY_EFFECT_ITEM_ID, "demo.item.dagger");
    let item = game
        .items
        .iter_mut()
        .find(|item| item.id == ABILITY_EFFECT_ITEM_ID)
        .expect("test weapon should exist");
    item.affix_ids = vec![ABILITY_EFFECT_AFFIX_ID.to_owned()];
    item.quality = ItemQualityDto::Fine;
    item.activation = Some(ItemActivationDto {
        profile_id: activation.id,
        name_key: activation.name_key,
        power: 1,
        cost: activation.charges.cost,
        device_check_difficulty: activation.device_check_difficulty,
        target_spec: target_spec_dto(&activation.target),
    });
    item.charges = Some(ItemChargesDto {
        current: 1,
        maximum: 1,
    });
    game
}

#[test]
fn riding_charge_cancellation_preserves_charge_and_rng() {
    let mut game = riding_charge_game(0xE3_6001);
    place_charge_target(&mut game);
    let rng_before = game.rng.clone();
    let mut events = Vec::new();
    game.use_inventory_item(
        ITEM_ID,
        Some(&TargetSelection::Entity {
            entity_id: "test.charge-target".to_owned(),
        }),
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("unmounted riding charge should be rejected cleanly");

    assert_eq!(game.rng, rng_before);
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == ITEM_ID)
            .and_then(|item| item.charges)
            .map(|charges| charges.current),
        Some(1)
    );
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::ItemUseUnavailable]
    ));
}

#[test]
fn riding_charge_moves_mount_attacks_and_uses_profile_recovery() {
    let mut game = riding_charge_game(0xE3_6002);
    let target = place_charge_target(&mut game);
    game.push_generated_actor(
        "test.charge-mount".to_owned(),
        "demo.actor.horse",
        game.player.position,
    );
    game.entities
        .iter_mut()
        .find(|entity| entity.id == "test.charge-mount")
        .expect("test mount should exist")
        .controller_id = Some(game.player.id.clone());
    game.riding_actor_id = Some("test.charge-mount".to_owned());
    let destination = Position {
        x: target.x - 1,
        y: target.y,
    };
    let mut events = Vec::new();
    game.use_inventory_item(
        ITEM_ID,
        Some(&TargetSelection::Entity {
            entity_id: "test.charge-target".to_owned(),
        }),
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("mounted riding charge should resolve");

    assert_eq!(game.player.position, destination);
    assert_eq!(
        game.entities
            .iter()
            .find(|entity| entity.id == "test.charge-mount")
            .map(|entity| entity.position),
        Some(destination)
    );
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::PlayerMeleeHit { .. } | DomainEvent::PlayerMeleeMissed { .. }
    )));
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == ITEM_ID)
            .and_then(|item| item.charges)
            .map(|charges| charges.current),
        Some(0)
    );

    for tick in 1..1_000 {
        game.world_tick = tick;
        game.process_inventory_device_recovery(&mut events);
    }
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == ITEM_ID)
            .and_then(|item| item.charges)
            .map(|charges| charges.current),
        Some(0)
    );
    game.world_tick = 1_000;
    game.process_inventory_device_recovery(&mut events);
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == ITEM_ID)
            .and_then(|item| item.charges)
            .map(|charges| charges.current),
        Some(1)
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::DeviceEnergyRecovered { amount: 1, .. }))
    );
}

#[test]
fn dungeon_anti_melee_riding_charge_moves_and_spends_charge_without_attacking() {
    let mut game = riding_charge_game(0xE3_6002);
    game.current_floor_id = "demo.floor.castle-depth-40".to_owned();
    let target = place_charge_target(&mut game);
    let target_hp = game.entities[0].hp;
    game.push_generated_actor(
        "test.mount".to_owned(),
        "demo.actor.horse",
        game.player.position,
    );
    game.entities[1].controller_id = Some(game.player.id.clone());
    game.riding_actor_id = Some("test.mount".to_owned());
    let mut events = Vec::new();
    game.use_inventory_item(
        ITEM_ID,
        Some(&TargetSelection::Direction {
            direction: Direction::East,
        }),
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    let destination = Position {
        x: target.x - 1,
        y: target.y,
    };
    assert_eq!(game.player.position, destination, "{events:?}");
    assert_eq!(game.entities[1].position, destination);
    assert_eq!(game.entities[0].hp, target_hp);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::PlayerMeleeBlocked))
    );
    assert!(!events.iter().any(|event| matches!(
        event,
        DomainEvent::PlayerMeleeHit { .. } | DomainEvent::PlayerMeleeMissed { .. }
    )));
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == ITEM_ID)
            .unwrap()
            .charges
            .unwrap()
            .current,
        0
    );
}

#[test]
fn biased_ego_activation_reuses_the_ability_effect_resolver() {
    let mut game = ability_effect_game(0xE3_7001);
    let mut events = Vec::new();
    game.use_inventory_item(
        ABILITY_EFFECT_ITEM_ID,
        Some(&TargetSelection::SelfTarget),
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("biased ego activation should resolve");

    assert!(
        game.player
            .statuses
            .iter()
            .any(|status| status.kind_id == "rfb.status.resist-fire")
    );
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == ABILITY_EFFECT_ITEM_ID)
            .and_then(|item| item.charges)
            .map(|charges| charges.current),
        Some(0)
    );
}

fn fetch_game(weight: u16) -> (Game, Position) {
    let mut game = activation_effect_game(17, "rfb-legacy.affix.arcane", "telekinesis", weight);
    clear_monsters(&mut game);
    super::support::choose_human_talent_if_pending(&mut game);
    game.items
        .retain(|item| !matches!(item.location, ItemLocation::Ground(_)));
    game.gold_piles.clear();
    let origin = game.player.position;
    let target = Position {
        x: origin.x + 3,
        y: origin.y,
    };
    for dx in 0..=4 {
        replace_terrain(
            &mut game,
            Position {
                x: origin.x + dx,
                y: origin.y,
            },
            "demo.terrain.floor",
        );
    }
    give_inventory_item(&mut game, "test.fetch-stack", "demo.item.iron-shot");
    let item = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.fetch-stack")
        .unwrap();
    item.location = ItemLocation::Ground(target);
    item.quantity = 9;
    game.reveal_current_visibility();
    game.rng = fetch_check_rng(true);
    (game, target)
}

fn fetch_check_rng(success: bool) -> RfbRng {
    RfbRng::seeded(
        (0..1000)
            .find(|seed| {
                let roll = RfbRng::seeded(*seed).bounded(100);
                if success {
                    roll < 5
                } else {
                    (5..10).contains(&roll)
                }
            })
            .unwrap(),
    )
}

fn use_fetch(game: &mut Game, target: Option<TargetSelection>) -> GameUpdate {
    super::support::dispatch_next(
        game,
        GameCommand::UseItem {
            item_id: ABILITY_EFFECT_ITEM_ID.to_owned(),
            target,
        },
    )
}

#[test]
fn fetch_activation_checks_unit_weight_moves_the_whole_instance_and_consumes_empty_casts() {
    for weight in [174, 175, 176] {
        let (mut game, target) = fetch_game(weight);
        let before = game
            .items
            .iter()
            .find(|item| item.id == "test.fetch-stack")
            .unwrap()
            .clone();
        let tick = game.world_tick;
        let update = use_fetch(
            &mut game,
            Some(TargetSelection::Position { position: target }),
        );
        let mut expected = before;
        if weight <= 175 {
            expected.location = ItemLocation::Ground(game.player.position);
        }
        assert_eq!(
            *game
                .items
                .iter()
                .find(|item| item.id == expected.id)
                .unwrap(),
            expected
        );
        assert_eq!(
            game.items
                .iter()
                .find(|item| item.id == ABILITY_EFFECT_ITEM_ID)
                .unwrap()
                .charges
                .unwrap()
                .current,
            0
        );
        assert_eq!(game.world_tick, tick + 10);
        assert!(update.events.iter().any(|event| matches!(&event.outcome,
            Some(GameEventOutcomeDto::AbilityEffects { resolution, .. })
            if matches!(resolution.effects.as_slice(), [AbilityEffectResolutionDto::FetchItem { moved, .. }] if *moved == (weight <= 175)))));
    }
}

#[test]
fn fetch_activation_target_branches_keep_original_blocking_rules() {
    let (base, target) = fetch_game(175);
    for (case, directional, should_move) in [
        ("clear", true, true),
        ("wall", true, false),
        ("wall", false, true),
        ("vault", false, false),
        ("vault", true, true),
        ("occupied", false, false),
        ("glyph", false, false),
        ("deep-water", false, false),
        ("water-path", true, true),
        ("empty-target", false, false),
        ("intermediate-pile", false, true),
        ("stairs", false, false),
        ("open-door", false, false),
        ("curtain-path", true, true),
    ] {
        let mut game = base.clone();
        let intermediate = Position {
            x: target.x - 1,
            y: target.y,
        };
        match case {
            "wall" => replace_terrain(&mut game, intermediate, "demo.terrain.wall"),
            "water-path" => {
                replace_terrain(&mut game, intermediate, "demo.terrain.surface-water-deep")
            }
            "curtain-path" => {
                replace_terrain(&mut game, intermediate, "demo.terrain.curtain-closed")
            }
            "stairs" | "open-door" => {
                let origin = game.player.position;
                replace_terrain(
                    &mut game,
                    origin,
                    if case == "stairs" {
                        "demo.terrain.stairs-up"
                    } else {
                        "demo.terrain.door-open"
                    },
                );
            }
            "vault" => {
                let index = game.index(target).unwrap();
                game.vault_cells[index] = true;
            }
            "occupied" => {
                give_inventory_item(&mut game, "test.obstruction", "demo.item.dagger");
                game.items
                    .iter_mut()
                    .find(|item| item.id == "test.obstruction")
                    .unwrap()
                    .location = ItemLocation::Ground(game.player.position);
            }
            "intermediate-pile" => {
                give_inventory_item(&mut game, "test.intermediate", "demo.item.dagger");
                game.items
                    .iter_mut()
                    .find(|item| item.id == "test.intermediate")
                    .unwrap()
                    .location = ItemLocation::Ground(intermediate);
            }
            "glyph" | "deep-water" => {
                let origin = game.player.position;
                replace_terrain(
                    &mut game,
                    origin,
                    if case == "glyph" {
                        "demo.terrain.warding-glyph"
                    } else {
                        "demo.terrain.surface-water-deep"
                    },
                );
            }
            _ => {}
        }
        let selection = if directional {
            TargetSelection::Direction {
                direction: Direction::East,
            }
        } else {
            TargetSelection::Position {
                position: if case == "empty-target" {
                    intermediate
                } else {
                    target
                },
            }
        };
        use_fetch(&mut game, Some(selection));
        if case == "intermediate-pile" {
            assert_eq!(
                game.items
                    .iter()
                    .find(|item| item.id == "test.intermediate")
                    .unwrap()
                    .location,
                ItemLocation::Ground(intermediate)
            );
        }
        assert_eq!(
            game.items
                .iter()
                .find(|item| item.id == "test.fetch-stack")
                .unwrap()
                .location,
            ItemLocation::Ground(if should_move {
                game.player.position
            } else {
                target
            }),
            "{case}/{directional}"
        );
        assert_eq!(
            game.items
                .iter()
                .find(|item| item.id == ABILITY_EFFECT_ITEM_ID)
                .unwrap()
                .charges
                .unwrap()
                .current,
            0,
            "{case}"
        );
    }
}

#[test]
fn fetch_activation_cancel_failure_and_success_have_distinct_costs() {
    let (base, target) = fetch_game(175);
    for (selection, success, ticks, charges, draws) in [
        (Some(TargetSelection::SelfTarget), true, 0, 1, 0),
        (None, true, 10, 1, 1),
        (None, false, 10, 1, 1),
        (
            Some(TargetSelection::Position { position: target }),
            false,
            10,
            1,
            1,
        ),
        (
            Some(TargetSelection::Position { position: target }),
            true,
            10,
            0,
            1,
        ),
    ] {
        let mut game = base.clone();
        game.rng = fetch_check_rng(success);
        let mut expected_rng = game.rng.clone();
        for _ in 0..draws {
            expected_rng.bounded(100);
        }
        let tick = game.world_tick;
        use_fetch(&mut game, selection);
        assert_eq!(game.world_tick, tick + ticks);
        assert_eq!(game.rng, expected_rng);
        assert_eq!(
            game.items
                .iter()
                .find(|item| item.id == ABILITY_EFFECT_ITEM_ID)
                .unwrap()
                .charges
                .unwrap()
                .current,
            charges
        );
    }
}

#[test]
fn fetch_vault_protection_is_hashed_and_survives_save_restore() {
    let (mut game, target) = fetch_game(175);
    let unprotected = game.state_hash();
    let index = game.index(target).unwrap();
    game.vault_cells[index] = true;
    assert_ne!(game.state_hash(), unprotected);
    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("vault cells round trip");
    assert_eq!(restored.state_hash(), game.state_hash());
    use_fetch(
        &mut restored,
        Some(TargetSelection::Position { position: target }),
    );
    assert_eq!(
        restored
            .items
            .iter()
            .find(|item| item.id == "test.fetch-stack")
            .unwrap()
            .location,
        ItemLocation::Ground(target)
    );
    let mut invalid = game.to_save();
    invalid.terrain.vault_cells.pop();
    assert!(Game::from_save_with_content(invalid, game.content.clone()).is_err());
}

#[test]
fn fetch_activation_device_bonus_uses_original_percentage_rounding() {
    for (bonus, weight, moved) in [
        (5, 218, true),
        (5, 219, false),
        (-1, 166, true),
        (-1, 167, false),
        (-20, 1, false),
    ] {
        let (mut game, target) = fetch_game(weight);
        let mut status = monster_combat::melee_status(STATUS_BERSERK, 20, "test").status;
        status.granted_modifiers.device_power_bonus = bonus;
        game.player.statuses.push(status);
        use_fetch(
            &mut game,
            Some(TargetSelection::Position { position: target }),
        );
        assert_eq!(
            game.items
                .iter()
                .find(|item| item.id == "test.fetch-stack")
                .unwrap()
                .location,
            ItemLocation::Ground(if moved { game.player.position } else { target }),
            "bonus {bonus}, weight {weight}"
        );
    }
}

#[test]
fn fetch_position_range_and_los_failures_are_resolved_without_extra_rng() {
    let (base, target) = fetch_game(175);
    let source = base
        .content
        .ability("rfb.ability.mutation.telekinesis")
        .unwrap()
        .clone();
    for (range, requires_los, should_move) in
        [(18, true, false), (18, false, true), (2, false, false)]
    {
        let mut game = base.clone();
        replace_terrain(
            &mut game,
            Position {
                x: target.x - 1,
                y: target.y,
            },
            "demo.terrain.wall",
        );
        let mut ability = source.clone();
        ability.target.range = range;
        ability.target.requires_line_of_effect = requires_los;
        ability.effect = AbilityEffectDefinition::FetchItem {
            maximum_weight_tenths_pound: 175,
        };
        let plan = game
            .ability_target_plan(&ability, &TargetSelection::Position { position: target })
            .unwrap();
        let rng = game.rng.clone();
        game.resolve_player_ability_effect(
            ability,
            plan,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert_eq!(game.rng, rng);
        assert_eq!(
            game.items
                .iter()
                .find(|item| item.id == "test.fetch-stack")
                .unwrap()
                .location,
            ItemLocation::Ground(if should_move {
                game.player.position
            } else {
                target
            })
        );
    }
}
