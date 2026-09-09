// SPDX-License-Identifier: MPL-2.0

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
};

use rfb_contract::{
    ACTIVE_FIXTURE_DIRECTORY, CONTRACT_SCHEMA_VERSION, ContractError, ContractFixture, observe,
    observe_assertions, validate_fixture_set, verify,
};
use rfb_protocol::Position;
use serde_json::json;

#[test]
#[ignore = "full contract replay is reserved for milestone or global contract validation"]
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
fn committed_contract_fixture_metadata_is_valid() {
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

    validate_fixture_set(&fixtures).expect("fixture metadata should be valid");
    let mut obsolete = fixtures[0].clone();
    obsolete.schema_version = CONTRACT_SCHEMA_VERSION - 1;
    assert!(matches!(
        validate_fixture_set(&[obsolete]),
        Err(ContractError::UnsupportedSchema(_))
    ));
}

fn minimal_default_fixture(
    preconditions: serde_json::Value,
    commands: serde_json::Value,
) -> ContractFixture {
    serde_json::from_value(json!({
        "schemaVersion": CONTRACT_SCHEMA_VERSION,
        "id": "town.minimal-contract-helper",
        "category": "town",
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
    let fixture = minimal_default_fixture(
        json!({
            "world": "demo.world.middle-earth",
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
fn player_position_precondition_rejects_out_of_bounds_cells() {
    let fixture = minimal_default_fixture(
        json!({
            "world": "demo.world.middle-earth",
            "debugClearEntities": true,
            "playerPosition": { "x": -1, "y": 0 }
        }),
        json!([]),
    );

    assert!(matches!(
        observe(&fixture),
        Err(ContractError::InvalidPlayerPositionPrecondition(Position {
            x: -1,
            y: 0
        }))
    ));
}

#[test]
fn equipment_precondition_relocates_and_identifies_an_existing_item() {
    let fixture = minimal_default_fixture(
        json!({
            "world": "demo.world.middle-earth",
            "debugClearEntities": true,
            "equipmentItems": [{
                "id": "demo.item.warding-band.1",
                "kindId": "demo.item.warding-band",
                "quantity": 1,
                "slotId": "right-ring",
                "quality": "fine",
                "affixIds": ["demo.affix.regeneration"],
                "permanentDestructionImmunities": []
            }]
        }),
        json!([]),
    );

    let observed = observe(&fixture).expect("equipment precondition should be accepted");

    assert!(
        observed
            .final_state
            .equipment
            .iter()
            .any(|item| item.id == "demo.item.warding-band.1")
    );
    assert_eq!(
        observed
            .final_state
            .item_property_knowledge
            .iter()
            .find(|knowledge| knowledge.item_id == "demo.item.warding-band.1")
            .expect("equipped ring knowledge should be projected")
            .known_affix_ids,
        ["demo.affix.regeneration"]
    );
}

#[test]
fn buy_first_from_shop_resolves_projected_stock_without_movement() {
    let fixture = minimal_default_fixture(
        json!({
            "world": "demo.world.middle-earth",
            "debugClearEntities": true,
            "playerPosition": { "x": 83, "y": 30 },
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
        Position { x: 83, y: 30 }
    );
    assert_eq!(observed.events.len(), 1);
    assert_eq!(observed.events[0].kind, "shop.purchase");
    assert!(observed.changed_cells.is_empty());
}

#[test]
fn focused_assertions_still_require_exact_values_events_and_state_hashes() {
    let mut fixture = minimal_default_fixture(
        json!({"world": "demo.world.middle-earth", "debugClearEntities": true}),
        json!([{"command": {"type": "wait"}}]),
    );
    let mut expected = observe_assertions(&fixture).expect("fixture should be observable");
    expected
        .final_state
        .retain(|key, _| key == "stateHash" || key == "turn");
    fixture.assertions = Some(expected.clone());
    verify(&fixture).expect("unselected projections should not be required");
    assert_eq!(
        observe_assertions(&fixture).unwrap(),
        expected,
        "refresh should preserve the scope"
    );

    for field in ["turn", "stateHash"] {
        let mut incorrect = fixture.clone();
        incorrect.assertions.as_mut().unwrap().final_state[field] = if field == "turn" {
            json!(999)
        } else {
            json!("wrong-hash")
        };
        assert!(matches!(
            verify(&incorrect),
            Err(ContractError::AssertionMismatch { .. })
        ));
    }
    let mut missing_event = fixture.clone();
    missing_event.assertions.as_mut().unwrap().events.clear();
    assert!(matches!(
        verify(&missing_event),
        Err(ContractError::AssertionMismatch { .. })
    ));

    fixture
        .assertions
        .as_mut()
        .unwrap()
        .final_state
        .remove("stateHash");
    assert!(matches!(
        validate_fixture_set(&[fixture]),
        Err(ContractError::MissingStateHash(_))
    ));
}
