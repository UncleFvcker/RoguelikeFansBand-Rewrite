// SPDX-License-Identifier: MPL-2.0

use std::path::PathBuf;

use rfb_contract::{ACTIVE_BASELINE, ACTIVE_FIXTURE_DIRECTORY, policy::validate_policy_file};

#[test]
fn shared_constants_stay_in_sync_across_crates() {
    // These constants are duplicated in intentionally isolated crates; this
    // assertion is the guard that keeps the copies from drifting apart.
    assert_eq!(
        rfb_contract::LEGACY_BASELINE_COMMIT,
        rfb_legacy_import::LEGACY_BASELINE_COMMIT
    );
}

#[test]
fn committed_active_baseline_policy_is_valid() {
    let policy = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../tests/fixtures/{ACTIVE_FIXTURE_DIRECTORY}/baseline-policy.json"
    ));
    let report = validate_policy_file(&policy).expect("active baseline policy should validate");
    assert_eq!(report.baseline, ACTIVE_BASELINE);
    assert_eq!(report.waiver_count, 0);
}
