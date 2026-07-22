// SPDX-License-Identifier: MPL-2.0

use std::{fs, path::PathBuf};

use rfb_contract::{ContractFixture, validate_fixture_set, verify};

#[test]
fn committed_contract_fixtures_pass() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/contract-v47/scenarios");
    let mut paths = fs::read_dir(&root)
        .expect("contract fixture directory should exist")
        .map(|entry| entry.expect("fixture entry should be readable").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    paths.sort();
    assert!(
        paths.len() >= 92,
        "the active contract baseline requires at least 92 committed fixtures"
    );

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

    for fixture in &fixtures {
        verify(fixture).unwrap_or_else(|error| panic!("{error}"));
    }
}
