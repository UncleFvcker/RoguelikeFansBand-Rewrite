// SPDX-License-Identifier: MPL-2.0
use super::*;

pub fn sync_demo_random_artifacts(
    source: &Path,
    pack_root: &Path,
) -> Result<usize, LegacyImportError> {
    let commit = resolve_legacy_content_commit(source)?;
    let devices = read_legacy_object_at(source, &commit, DEVICES_C_SOURCE)?;
    let candidates: Vec<_> = parse_effect_info_activation_candidates(&devices)?
        .into_iter()
        .filter(|candidate| candidate.rarity > 0)
        .collect();
    let missing: Vec<_> = candidates
        .iter()
        .filter(|candidate| armor_ego_audit::activation_effect(candidate).is_none())
        .map(|candidate| candidate.token.clone())
        .collect();
    if !missing.is_empty() {
        return Err(LegacyImportError::InvalidEgoAudit(format!(
            "random artifact effects lack consumers: {}",
            missing.join(", ")
        )));
    }
    let bias_names = [
        "ELEC",
        "POIS",
        "FIRE",
        "COLD",
        "ACID",
        "STR",
        "INT",
        "WIS",
        "DEX",
        "CON",
        "CHR",
        "CHAOS",
        "PRIESTLY",
        "NECROMANTIC",
        "LAW",
        "ROGUE",
        "MAGE",
        "WARRIOR",
        "RANGER",
        "DEMON",
        "PROTECTION",
        "ARCHER",
    ];
    let mut activations = Vec::new();
    let mut masks = Vec::new();
    for candidate in &candidates {
        let (mut effect, target, ground) = armor_ego_audit::activation_effect(candidate).unwrap();
        if ground {
            effect["affectsGroundItems"] = serde_json::json!(true);
        }
        let token = candidate.token.to_ascii_lowercase().replace('_', "-");
        activations.push(serde_json::json!({
            "id": format!("rfb.device-activation.random-artifact.{token}"),
            "nameKey": format!("device-activation-e5-{token}-name"),
            "weight": (255 / u32::from(candidate.rarity)).max(1), "minDepth": 1, "maxDepth": 100,
            "deviceCheckDifficulty": candidate.level,
            "charges": {"minimum": 1, "maximum": 1, "cost": 1},
            "recovery": {"intervalTicks": candidate.recovery_turns * 10, "energyPerMille": 1000},
            "target": target, "effect": effect
        }));
        let mut mask = 0_u32;
        for bias in &candidate.biases {
            let bit = bias_names
                .iter()
                .position(|name| bias == &format!("BIAS_{name}"))
                .ok_or_else(|| {
                    LegacyImportError::InvalidEgoAudit(format!(
                        "unknown source activation bias {bias}"
                    ))
                })?;
            mask |= 1 << bit;
        }
        masks.push(mask);
    }
    let mut name_tables = BTreeMap::new();
    let mut unresolved = BTreeSet::new();
    let has_chinese = |text: &str| text.chars().any(|c| ('\u{3400}'..='\u{9fff}').contains(&c));
    for name in [
        "lite_drk",
        "lite_cursed",
        "lite_low",
        "lite_med",
        "lite_high",
        "ring_cursed",
        "ring_low",
        "ring_med",
        "ring_high",
        "amu_cursed",
        "amu_low",
        "amu_med",
        "amu_high",
        "ranged",
        "a_cursed",
        "a_med",
        "a_high",
        "aa_med",
        "ab_med",
        "ac_med",
        "ag_med",
        "ah_med",
        "as_med",
        "w_types",
        "w_sword",
        "w_hafted",
        "w_pole",
        "w_cursed",
        "w_med",
        "w_high",
    ] {
        let file = format!("{name}.txt");
        let table = read_legacy_object_at(source, &commit, &format!("lib/file/{file}"))?;
        for line in table.trim_start_matches('\u{feff}').lines() {
            if !line.is_empty()
                && !line.starts_with('#')
                && !line.starts_with("N:")
                && !has_chinese(line)
            {
                unresolved.insert(line.to_owned());
            }
        }
        name_tables.insert(file, table);
    }
    let mut activation_names = BTreeMap::new();
    let mut tokens = Vec::new();
    for line in devices.lines().map(str::trim) {
        if let Some((token, _)) = line
            .strip_prefix("case EFFECT_")
            .and_then(|s| s.split_once(':'))
        {
            tokens.push(token.to_owned());
        }
        if let Some(name) = line
            .strip_prefix("if (name) return \"")
            .and_then(|s| s.strip_suffix("\";"))
        {
            for token in tokens.drain(..) {
                activation_names.insert(token, name.to_owned());
            }
        }
    }
    let root = pack_root
        .parent()
        .and_then(Path::parent)
        .unwrap_or(pack_root);
    let mut locale_updates = Vec::new();
    for language in ["en-US", "zh-CN"] {
        let path = root.join(format!("locales/{language}/content.ftl"));
        let mut text = fs::read_to_string(&path)?;
        let start_marker = "# Random artifact activations (generated)";
        let end_marker = "# /Random artifact activations";
        if let Some(start) = text.find(start_marker) {
            let end = start + text[start..].find(end_marker).unwrap() + end_marker.len();
            text.replace_range(start..end, "");
        }
        text = text.trim_end().to_owned();
        let existing: BTreeSet<_> = text
            .lines()
            .filter_map(|line| line.split_once(" = ").map(|(key, _)| key.to_owned()))
            .collect();
        text.push_str(&format!("\n\n{start_marker}\n"));
        for candidate in &candidates {
            let token = candidate.token.to_ascii_lowercase().replace('_', "-");
            let key = format!("device-activation-e5-{token}-name");
            let name = activation_names.get(&candidate.token).ok_or_else(|| {
                LegacyImportError::InvalidEgoAudit(format!(
                    "missing source activation name {}",
                    candidate.token
                ))
            })?;
            if !has_chinese(name) {
                unresolved.insert(key.clone());
            }
            if !existing.contains(&key) {
                let english = candidate.token.to_ascii_lowercase().replace('_', " ");
                text.push_str(&format!(
                    "{key} = {}\n",
                    if language == "zh-CN" { name } else { &english }
                ));
            }
        }
        text.push_str(&format!("{end_marker}\n"));
        locale_updates.push((path, text));
    }
    let path = pack_root.join("randomArtifacts/source.json");
    let mut value = serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/random-artifact.schema.json"), "formatVersion": 1,
        "id": "rfb.random-artifact.generation", "sourceCommit": commit,
        "nameTables": name_tables, "unresolvedChineseNames": unresolved,
        "deviceGeneration": {"activations": activations}, "activationBiases": masks
    });
    // COST_REAL values are supplied by the source C oracle; preserve them on a
    // repeat import only when the profile's source power has not changed.
    if path.exists() {
        let previous: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)?;
        let same_source = previous["sourceCommit"] == value["sourceCommit"];
        for profile in value["deviceGeneration"]["activations"]
            .as_array_mut()
            .unwrap()
        {
            if let Some(old) = previous["deviceGeneration"]["activations"]
                .as_array()
                .unwrap()
                .iter()
                .find(|old| {
                    same_source
                        && old["id"] == profile["id"]
                        && old["deviceCheckDifficulty"] == profile["deviceCheckDifficulty"]
                })
                .filter(|old| old["rfbValue"].is_number())
            {
                profile["rfbValue"] = old["rfbValue"].clone();
            }
        }
    }
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(path, serde_json::to_string_pretty(&value)? + "\n")?;
    for (path, text) in locale_updates {
        fs::write(path, text)?;
    }
    Ok(candidates.len())
}
