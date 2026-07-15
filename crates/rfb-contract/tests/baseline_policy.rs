// SPDX-License-Identifier: MPL-2.0

use std::path::PathBuf;

use rfb_contract::approval::validate_policy_file;

#[test]
fn committed_baseline_policy_and_waivers_are_valid() {
    let policy = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/contract-v1/baseline-policy.json");
    let report = validate_policy_file(&policy).expect("baseline policy should validate");
    assert_eq!(report.policy_id, "rfb-contract-baseline-v1");
    assert!(report.fixture_count >= 20);
    assert_eq!(report.waiver_count, 0);
}
