// SPDX-License-Identifier: MPL-2.0

use super::support::{clear_monsters, give_inventory_item, replace_terrain};
use super::*;

const AFFIX_ID: &str = "test.affix.riding-charge";
const ACTIVATION_ID: &str = "test.device-activation.riding-charge";
const ITEM_ID: &str = "test.item.riding-charge";
const ABILITY_EFFECT_AFFIX_ID: &str = "test.affix.ability-effect";
const ABILITY_EFFECT_ACTIVATION_ID: &str = "test.device-activation.ability-effect";
const ABILITY_EFFECT_ITEM_ID: &str = "test.item.ability-effect";

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
        .find(|affix| affix.id == "rfb-legacy.affix.combat")
        .expect("test source affix should exist")
        .clone();
    affix.id = AFFIX_ID.to_owned();
    affix.name_key = "test-affix-riding-charge-name".to_owned();
    affix.description_key = "test-affix-riding-charge-description".to_owned();
    affix.rfb_ego = None;
    affix.device_generation = Some(rfb_content::ItemDeviceGenerationDefinition {
        activations: vec![rfb_content::ItemDeviceActivationDefinition {
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
        .find(|affix| affix.id == "rfb-legacy.affix.craft")
        .expect("Craft should expose biased activation candidates")
        .clone();
    let mut activation = affix
        .device_generation
        .as_ref()
        .expect("Craft activation candidates")
        .activations
        .iter()
        .find(|activation| activation.id.ends_with("resist-fire"))
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
        activations: vec![activation.clone()],
        recovery: None,
    });
    artifact.content.affixes.push(affix);
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

    game.world_tick = 999;
    game.process_inventory_device_recovery(&mut events);
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
