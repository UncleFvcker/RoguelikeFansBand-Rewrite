// SPDX-License-Identifier: MPL-2.0

use std::path::PathBuf;

use rfb_contract::approval::validate_policy_file;

#[test]
fn committed_baseline_policy_and_waivers_are_valid() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    for version in ["v1", "v2", "v3", "v4", "v5", "v6", "v7", "v8", "v9", "v10"] {
        let policy = root.join(format!("contract-{version}/baseline-policy.json"));
        let report = validate_policy_file(&policy).expect("baseline policy should validate");
        assert_eq!(report.policy_id, format!("rfb-contract-baseline-{version}"));
        let minimum = match version {
            "v10" => 39,
            "v9" => 36,
            "v7" | "v8" => 32,
            "v6" => 29,
            "v5" => 28,
            "v4" => 26,
            "v3" => 22,
            _ => 20,
        };
        assert!(report.fixture_count >= minimum);
        assert_eq!(report.waiver_count, 0);
    }
}
