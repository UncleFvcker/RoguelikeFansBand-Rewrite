// SPDX-License-Identifier: MPL-2.0

use std::path::PathBuf;

use rfb_contract::approval::validate_policy_file;

#[test]
fn committed_baseline_policy_and_waivers_are_valid() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    for version in ["v1", "v2", "v3"] {
        let policy = root.join(format!("contract-{version}/baseline-policy.json"));
        let report = validate_policy_file(&policy).expect("baseline policy should validate");
        assert_eq!(report.policy_id, format!("rfb-contract-baseline-{version}"));
        let minimum = if version == "v3" { 22 } else { 20 };
        assert!(report.fixture_count >= minimum);
        assert_eq!(report.waiver_count, 0);
    }
}
