// SPDX-License-Identifier: MPL-2.0

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
};

use rfb_contract::{
    ACTIVE_FIXTURE_DIRECTORY, ContractError, ContractFixture, observe, validate_fixture_set, verify,
};
use rfb_protocol::Position;
use serde_json::json;

#[test]
fn committed_contract_fixtures_pass() {
    let baseline_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../tests/fixtures/{ACTIVE_FIXTURE_DIRECTORY}"));

    let mut paths = fs::read_dir(baseline_root.join("scenarios"))
        .expect("contract fixture directory should exist")
        .map(|entry| entry.expect("fixture entry should be readable").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    paths.sort();

    let fixtures = paths
        .iter()
        .map(|path| {
            serde_json::from_slice::<ContractFixture>(
                &fs::read(path).expect("fixture should be readable"),
            )
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
        })
        .collect::<Vec<_>>();
    validate_fixture_set(&fixtures).expect("fixture set should be valid");

    // Each verify run is independent and deterministic, so the fixtures can be
    // checked concurrently; failures are re-sorted to keep the report stable.
    let worker_count = thread::available_parallelism()
        .map_or(1, std::num::NonZeroUsize::get)
        .min(fixtures.len().max(1));
    let next_fixture = AtomicUsize::new(0);
    let mut failures = thread::scope(|scope| {
        let workers = (0..worker_count)
            .map(|_| {
                scope.spawn(|| {
                    let mut worker_failures = Vec::new();
                    loop {
                        let index = next_fixture.fetch_add(1, Ordering::Relaxed);
                        let Some(fixture) = fixtures.get(index) else {
                            break;
                        };
                        if let Err(error) = verify(fixture) {
                            worker_failures.push(format!("{}: {error}", fixture.id));
                        }
                    }
                    worker_failures
                })
            })
            .collect::<Vec<_>>();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().expect("fixture worker should not panic"))
            .collect::<Vec<_>>()
    });
    failures.sort();
    assert!(
        failures.is_empty(),
        "contract fixtures failed:\n{}",
        failures.join("\n")
    );
}

#[test]
fn legacy_attribute_projection_migration_is_schema_bounded() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/active/scenarios/450-potion-attribute-history.json");
    let fixture: ContractFixture =
        serde_json::from_slice(&fs::read(path).expect("attribute fixture should be readable"))
            .expect("attribute fixture should parse");

    let legacy = |schema_version| {
        let mut fixture = fixture.clone();
        fixture.schema_version = schema_version;
        let attributes = &mut fixture
            .assertions
            .as_mut()
            .expect("fixture should have assertions")
            .final_state
            .player_attributes
            .as_mut()
            .expect("fixture should project player attributes")
            .attributes;
        for value in [
            &mut attributes.strength,
            &mut attributes.intelligence,
            &mut attributes.wisdom,
            &mut attributes.dexterity,
            &mut attributes.constitution,
            &mut attributes.charisma,
        ] {
            value.maximum_natural = 0;
        }
        fixture
    };

    verify(&legacy(1)).expect("schema 1 should migrate the complete legacy projection");

    let mut partial = legacy(1);
    partial
        .assertions
        .as_mut()
        .unwrap()
        .final_state
        .player_attributes
        .as_mut()
        .unwrap()
        .attributes
        .strength
        .maximum_natural = 13;
    assert!(matches!(
        verify(&partial),
        Err(ContractError::IncompleteLegacyAttributeProjection(_))
    ));
    assert!(matches!(
        verify(&legacy(2)),
        Err(ContractError::AssertionMismatch { .. })
    ));
}

fn minimal_warrens_fixture(
    preconditions: serde_json::Value,
    commands: serde_json::Value,
) -> ContractFixture {
    serde_json::from_value(json!({
        "schemaVersion": 2,
        "id": "town.minimal-contract-helper",
        "legacyCommit": "191f48c3fd1cdbc81a3d3395a88cd6758402b4d9",
        "determinism": "exact",
        "seed": "42",
        "preconditions": preconditions,
        "commands": commands
    }))
    .expect("minimal fixture should parse")
}

#[test]
fn player_position_precondition_does_not_simulate_movement() {
    let fixture = minimal_warrens_fixture(
        json!({
            "world": "demo.world.warrens-journey",
            "debugClearEntities": true,
            "playerPosition": { "x": 32, "y": 13 }
        }),
        json!([]),
    );

    let observed = observe(&fixture).expect("walkable player position should be accepted");

    assert_eq!(
        observed.final_state.player_position,
        Position { x: 32, y: 13 }
    );
    assert_eq!(observed.final_state.revision, 0);
    assert_eq!(observed.final_state.turn, 0);
    assert!(observed.changed_cells.is_empty());
    assert!(observed.events.is_empty());
}

#[test]
fn player_position_precondition_rejects_unwalkable_cells() {
    let fixture = minimal_warrens_fixture(
        json!({
            "world": "demo.world.warrens-journey",
            "debugClearEntities": true,
            "playerPosition": { "x": 22, "y": 6 }
        }),
        json!([]),
    );

    assert!(matches!(
        observe(&fixture),
        Err(ContractError::InvalidPlayerPositionPrecondition(Position {
            x: 22,
            y: 6
        }))
    ));
}

#[test]
fn equipment_precondition_relocates_and_identifies_an_existing_item() {
    let fixture = minimal_warrens_fixture(
        json!({
            "world": "demo.world.original-v1",
            "debugClearEntities": true,
            "equipmentItems": [{
                "id": "demo.item.echo-charm.1",
                "kindId": "demo.item.echo-charm",
                "quantity": 1,
                "slotId": "charm",
                "quality": "fine",
                "affixIds": ["demo.affix.harmonic-edge"]
            }]
        }),
        json!([]),
    );

    let observed = observe(&fixture).expect("equipment precondition should be accepted");

    assert_eq!(observed.final_state.ground_item_count, 4);
    assert_eq!(observed.final_state.equipment.len(), 1);
    assert_eq!(
        observed.final_state.equipment[0].id,
        "demo.item.echo-charm.1"
    );
    assert_eq!(
        observed.final_state.item_property_knowledge[0].known_affix_ids,
        ["demo.affix.harmonic-edge"]
    );
}

#[test]
fn buy_first_from_shop_resolves_projected_stock_without_movement() {
    let fixture = minimal_warrens_fixture(
        json!({
            "world": "demo.world.warrens-journey",
            "debugClearEntities": true,
            "playerPosition": { "x": 32, "y": 13 },
            "playerGold": 1000000
        }),
        json!([{
            "command": {
                "type": "buy-first-from-shop",
                "shopId": "demo.shop.outpost-general-store",
                "quantity": 1
            }
        }]),
    );

    let observed = observe(&fixture).expect("first projected shop item should be purchasable");

    assert_eq!(
        observed.final_state.player_position,
        Position { x: 32, y: 13 }
    );
    assert_eq!(observed.events.len(), 1);
    assert_eq!(observed.events[0].kind, "shop.purchase");
    assert!(observed.changed_cells.is_empty());
}
