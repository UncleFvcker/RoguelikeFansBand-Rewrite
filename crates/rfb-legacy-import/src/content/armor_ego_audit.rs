// SPDX-License-Identifier: MPL-2.0

use super::*;

const ARMOR_TYPES: &[&str] = &[
    "BODY_ARMOR",
    "DRAGON_ARMOR",
    "ROBE",
    "SHIELD",
    "CROWN",
    "HELMET",
    "CLOAK",
    "GLOVES",
    "BOOTS",
];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ArmorEgoContract {
    source_index: u32,
    english_name: String,
    chinese_name: String,
    types: Vec<String>,
    level: u16,
    max_level: Option<u16>,
    rarity: u16,
    flags: Vec<String>,
    combat_maxima: [i32; 4],
    activation: Option<(String, u16, u16)>,
    affix_id: String,
}

pub(super) fn validate_armor_ego_contract(
    egos: &[LegacyEgoEntry],
    chinese_names: &[Option<String>],
) -> Result<(), LegacyImportError> {
    let contract: Vec<ArmorEgoContract> =
        serde_json::from_str(include_str!("armor-ego-contract.json"))?;
    let armor = egos
        .iter()
        .filter(|ego| {
            ego.slots
                .iter()
                .any(|slot| ARMOR_TYPES.contains(&slot.as_str()))
        })
        .collect::<Vec<_>>();
    if armor.len() != 76
        || armor.iter().map(|ego| ego.index).collect::<Vec<_>>()
            != contract
                .iter()
                .map(|ego| ego.source_index)
                .collect::<Vec<_>>()
        || contract
            .iter()
            .map(|ego| &ego.affix_id)
            .collect::<BTreeSet<_>>()
            .len()
            != 76
    {
        return Err(LegacyImportError::InvalidEgoAudit(
            "armor ego source indices or stable IDs changed".to_owned(),
        ));
    }
    for (ego, expected) in armor.into_iter().zip(contract) {
        let activation = ego
            .activation
            .as_ref()
            .map(|value| (value.token.clone(), value.power, value.recovery_turns));
        if ego.name != expected.english_name
            || chinese_names
                .get(ego.index as usize)
                .and_then(Option::as_ref)
                != Some(&expected.chinese_name)
            || ego.slots != expected.types
            || ego.level != expected.level
            || ego.max_level != expected.max_level
            || ego.rarity != expected.rarity
            || ego.flags != expected.flags
            || [
                ego.max_to_hit,
                ego.max_to_damage,
                ego.max_to_armor,
                ego.max_pval,
            ] != expected.combat_maxima
            || activation != expected.activation
            || ego.has_activation != expected.activation.is_some()
        {
            return Err(LegacyImportError::InvalidEgoAudit(format!(
                "armor ego {} no longer matches its authoritative expectation",
                ego.index
            )));
        }
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ArmorBaseContract {
    source_index: u32,
    source_name: String,
    tval: u16,
    sval: u16,
    item_id: String,
}

fn armor_identity_updates(
    entries: &[LegacyItemEntry],
    pack_root: &Path,
) -> Result<Vec<(PathBuf, String)>, LegacyImportError> {
    let contract: Vec<ArmorBaseContract> =
        serde_json::from_str(include_str!("armor-base-contract.json"))?;
    let selection: DemoItemSelection =
        serde_json::from_slice(&fs::read(pack_root.join("legacy-item-selection.json"))?)?;
    let adaptations: DemoItemAdaptationLedger =
        serde_json::from_slice(&fs::read(pack_root.join("legacy-item-adaptations.json"))?)?;
    let mut updates = Vec::new();
    for expected in contract {
        let selected = selection.items.iter().any(|item| {
            item.source_index == expected.source_index
                && format!("demo.item.{}", item.id) == expected.item_id
        }) || adaptations.items.iter().any(|item| {
            item.status == DemoItemCoverageStatus::Active
                && item.source_index == expected.source_index
                && item.item_id == expected.item_id
        });
        if !selected
            || !entries.iter().any(|entry| {
                entry.index == expected.source_index
                    && entry.name == expected.source_name
                    && entry.tval == expected.tval
                    && entry.sval == expected.sval
            })
        {
            return Err(LegacyImportError::InvalidEgoAudit(format!(
                "armor base {} source or selection identity changed",
                expected.source_index
            )));
        }
        let id = expected
            .item_id
            .strip_prefix("demo.item.")
            .expect("demo armor ID");
        let path = pack_root.join("items").join(format!("{id}.json"));
        let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)?;
        let identity = serde_json::json!({
            "sourceIndex": expected.source_index, "tval": expected.tval, "sval": expected.sval,
        });
        if value["id"] != expected.item_id
            || value
                .get("rfbBaseKind")
                .is_some_and(|existing| existing != &identity)
        {
            return Err(LegacyImportError::InvalidEgoAudit(format!(
                "{} has a conflicting armor identity",
                path.display()
            )));
        }
        value["rfbBaseKind"] = identity;
        updates.push((path, serde_json::to_string_pretty(&value)? + "\n"));
    }
    Ok(updates)
}

/// Backfills only the E5.0 existing armor identities after validating the full batch.
pub fn sync_demo_armor_ego_identities(
    source: &Path,
    pack_root: &Path,
) -> Result<usize, LegacyImportError> {
    let commit = resolve_legacy_content_commit(source)?;
    let egos = parse_e_info(&read_legacy_object_at(source, &commit, E_INFO_SOURCE)?)?;
    let names = parse_chinese_name_table(
        &read_legacy_object_at(source, &commit, E_NAME_ZH_SOURCE)?,
        E_NAME_ZH_SOURCE,
    )?;
    validate_armor_ego_contract(&egos, &names)?;
    let entries = parse_k_info(&read_legacy_object_at(source, &commit, K_INFO_SOURCE)?)?;
    let updates = armor_identity_updates(&entries, pack_root)?;
    for (path, contents) in &updates {
        fs::write(path, contents)?;
    }
    Ok(updates.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn armor_ego_contract_rejects_source_and_chinese_name_drift() {
        let contract: Vec<ArmorEgoContract> =
            serde_json::from_str(include_str!("armor-ego-contract.json")).unwrap();
        let mut names = vec![None; 153];
        let egos = contract
            .iter()
            .map(|expected| {
                names[expected.source_index as usize] = Some(expected.chinese_name.clone());
                LegacyEgoEntry {
                    index: expected.source_index,
                    name: expected.english_name.clone(),
                    slots: expected.types.clone(),
                    level: expected.level,
                    max_level: expected.max_level,
                    rarity: expected.rarity,
                    max_to_hit: expected.combat_maxima[0],
                    max_to_damage: expected.combat_maxima[1],
                    max_to_armor: expected.combat_maxima[2],
                    max_pval: expected.combat_maxima[3],
                    flags: expected.flags.clone(),
                    has_activation: expected.activation.is_some(),
                    activation: expected
                        .activation
                        .as_ref()
                        .map(|(token, power, recovery)| LegacyEgoActivation {
                            token: token.clone(),
                            power: *power,
                            recovery_turns: *recovery,
                        }),
                }
            })
            .collect::<Vec<_>>();
        validate_armor_ego_contract(&egos, &names).unwrap();
        for index in 0..egos.len() {
            let mut changed = egos.clone();
            changed[index].rarity += 1;
            assert!(validate_armor_ego_contract(&changed, &names).is_err());
            let mut changed_names = names.clone();
            changed_names[egos[index].index as usize] = None;
            assert!(validate_armor_ego_contract(&egos, &changed_names).is_err());
        }
        let mut changed = egos.clone();
        changed
            .iter_mut()
            .find(|ego| ego.index == 72)
            .unwrap()
            .activation
            .as_mut()
            .unwrap()
            .recovery_turns += 1;
        assert!(validate_armor_ego_contract(&changed, &names).is_err());
        assert!(validate_armor_ego_contract(&egos[1..], &names).is_err());
    }

    #[test]
    fn committed_armor_base_contract_is_current_and_rejects_drift() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
        let contract: Vec<ArmorBaseContract> =
            serde_json::from_str(include_str!("armor-base-contract.json")).unwrap();
        assert_eq!(contract.len(), 38);
        assert_eq!(
            contract
                .iter()
                .map(|entry| (entry.tval, entry.sval))
                .collect::<BTreeSet<_>>()
                .len(),
            38
        );
        let mut entries = contract
            .iter()
            .map(|expected| LegacyItemEntry {
                index: expected.source_index,
                name: expected.source_name.clone(),
                tval: expected.tval,
                sval: expected.sval,
                ..Default::default()
            })
            .collect::<Vec<_>>();
        let updates = armor_identity_updates(&entries, &root).unwrap();
        for (path, contents) in updates {
            let before: serde_json::Value =
                serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
            let after: serde_json::Value = serde_json::from_str(&contents).unwrap();
            assert_eq!(before, after, "committed armor identities must be current");
        }
        entries.last_mut().unwrap().sval += 1;
        assert!(armor_identity_updates(&entries, &root).is_err());
    }

    #[test]
    fn armor_stable_ids_reuse_existing_affixes_without_collisions() {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original/affixes");
        let affixes = fs::read_dir(root)
            .unwrap()
            .map(|entry| {
                serde_json::from_slice::<serde_json::Value>(
                    &fs::read(entry.unwrap().path()).unwrap(),
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let contract: Vec<ArmorEgoContract> =
            serde_json::from_str(include_str!("armor-ego-contract.json")).unwrap();
        let mut existing = Vec::new();
        for expected in contract {
            if let Some(affix) = affixes
                .iter()
                .find(|affix| affix["id"] == expected.affix_id)
            {
                if let Some(ego) = affix.get("rfbEgo") {
                    assert_eq!(ego["sourceIndex"], expected.source_index);
                }
                existing.push(expected.source_index);
            }
        }
        assert_eq!(existing, [50, 56, 72, 126, 138]);
    }
}
