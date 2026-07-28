// SPDX-License-Identifier: MPL-2.0

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
};

use rfb_contract::{ACTIVE_FIXTURE_DIRECTORY, ContractFixture, validate_fixture_set, verify};

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
