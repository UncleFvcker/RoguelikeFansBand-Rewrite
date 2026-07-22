// SPDX-License-Identifier: MPL-2.0

use std::path::PathBuf;

use rfb_contract::approval::validate_policy_file;

#[test]
fn committed_baseline_policy_and_waivers_are_valid() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    for version in [
        "v1", "v2", "v3", "v4", "v5", "v6", "v7", "v8", "v9", "v10", "v11", "v12", "v13", "v14",
        "v15", "v16", "v17", "v18", "v19", "v20", "v21", "v22", "v23", "v24", "v25", "v26", "v27",
        "v28", "v29", "v30", "v31", "v32", "v33", "v34", "v35", "v36", "v37", "v38", "v39", "v40",
        "v41", "v42", "v43", "v44", "v45", "v46", "v47", "v48", "v49", "v50", "v51", "v52", "v53",
        "v54", "v55", "v56", "v57", "v58", "v59", "v60", "v61", "v62",
    ] {
        let policy = root.join(format!("contract-{version}/baseline-policy.json"));
        let report = validate_policy_file(&policy).expect("baseline policy should validate");
        assert_eq!(report.policy_id, format!("rfb-contract-baseline-{version}"));
        let minimum = match version {
            "v62" => 125,
            "v61" => 121,
            "v60" => 119,
            "v59" => 117,
            "v58" => 117,
            "v57" => 114,
            "v56" => 112,
            "v55" => 110,
            "v54" => 108,
            "v53" => 106,
            "v52" => 104,
            "v51" => 102,
            "v50" => 100,
            "v49" => 99,
            "v48" => 96,
            "v47" => 92,
            "v46" => 91,
            "v45" => 88,
            "v44" => 86,
            "v43" => 85,
            "v42" => 83,
            "v41" => 81,
            "v40" => 79,
            "v39" => 77,
            "v38" => 76,
            "v37" => 75,
            "v36" => 74,
            "v35" => 73,
            "v34" => 72,
            "v33" => 71,
            "v32" => 70,
            "v31" => 68,
            "v30" => 67,
            "v29" => 66,
            "v28" => 65,
            "v27" => 64,
            "v26" => 63,
            "v25" => 62,
            "v24" => 61,
            "v23" => 60,
            "v22" => 59,
            "v21" => 58,
            "v20" => 57,
            "v19" => 56,
            "v18" => 55,
            "v17" => 54,
            "v16" => 53,
            "v15" => 52,
            "v14" => 50,
            "v13" => 49,
            "v12" => 48,
            "v11" => 47,
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
