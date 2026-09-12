// SPDX-License-Identifier: MPL-2.0

use super::*;
use serde_json::{Value, json};

fn invalid(message: impl Into<String>) -> LegacyImportError {
    LegacyImportError::InvalidDemoItemAudit(message.into())
}

fn allocation_rows(entry: &LegacyItemEntry, id: &str) -> Vec<Value> {
    entry
        .allocations
        .iter()
        .map(|row| {
            json!({
                "itemKindId": id, "quantity": 1, "weight": 100 / row.chance,
                "minDepth": row.level,
                "maxDepth": if entry.max_level == 0 { u16::MAX } else { entry.max_level },
            })
        })
        .collect()
}

fn chinese_display_name(entry: &LegacyItemEntry, template: &str) -> String {
    let name = singular_chinese_kind_name(template);
    // flavor.c::object_desc, aware/OD_NO_FLAVOR branch. The Chinese table
    // supplies a stem for flavored consumables; retain its verbatim spelling.
    match entry.tval {
        70 => format!("{name}卷轴"),
        // Mead of Poetry has a complete source display name, not a potion stem.
        75 if entry.sval == 14 => name,
        75 => format!("{name}药水"),
        80 if entry.name.contains(':') => format!("{name}蘑菇"),
        11 => name.trim_end_matches('#').to_owned(),
        _ => name,
    }
}

/// Synchronize only source identities, allocation rows and theme references.
/// Existing effect/device/book adaptations remain authoritative for execution.
pub fn sync_demo_base_allocation(source: &Path, pack: &Path) -> Result<usize, LegacyImportError> {
    let commit = resolve_legacy_content_commit(source)?;
    let entries = parse_k_info(&read_legacy_object_at(source, &commit, K_INFO_SOURCE)?)?;
    let names = parse_chinese_name_table(
        &read_legacy_object_at(source, &commit, K_NAME_ZH_SOURCE)?,
        K_NAME_ZH_SOURCE,
    )?;
    let selection: DemoItemSelection =
        serde_json::from_slice(&fs::read(pack.join("legacy-item-selection.json"))?)?;
    let adaptations: DemoItemAdaptationLedger =
        serde_json::from_slice(&fs::read(pack.join("legacy-item-adaptations.json"))?)?;
    let mut identities = BTreeMap::new();
    for (id, index) in selection
        .items
        .iter()
        .map(|item| (format!("demo.item.{}", item.id), item.source_index))
        .chain(
            adaptations
                .items
                .iter()
                .filter(|item| item.status == DemoItemCoverageStatus::Active)
                .map(|item| (item.item_id.clone(), item.source_index)),
        )
    {
        if identities
            .insert(id.clone(), index)
            .is_some_and(|previous| previous != index)
        {
            return Err(invalid(format!("conflicting source identity for {id}")));
        }
    }
    let mut items = BTreeMap::new();
    for file in fs::read_dir(pack.join("items"))? {
        let path = file?.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let value: Value = serde_json::from_slice(&fs::read(&path)?)?;
        let id = value["id"]
            .as_str()
            .ok_or_else(|| invalid("item has no id"))?
            .to_owned();
        if let Some(index) = value["rfbBaseKind"]["sourceIndex"].as_u64() {
            let index = u32::try_from(index).map_err(|_| invalid("source index overflow"))?;
            if identities
                .insert(id.clone(), index)
                .is_some_and(|previous| previous != index)
            {
                return Err(invalid(format!("conflicting source identity for {id}")));
            }
        }
        items.insert(id, (path, value));
    }
    let base_path = pack.join("lootTables/base-items.json");
    let mut base: Value = serde_json::from_slice(&fs::read(&base_path)?)?;
    let previous_ids = base["entries"]
        .as_array()
        .ok_or_else(|| invalid("base pool has no entries"))?
        .iter()
        .map(|row| row["itemKindId"].as_str().unwrap().to_owned())
        .collect::<BTreeSet<_>>();
    // Existing natural candidates disambiguate adapted explicit device aliases.
    let mut canonical = BTreeMap::new();
    for id in &previous_ids {
        let index = *identities
            .get(id)
            .ok_or_else(|| invalid(format!("unmapped natural candidate {id}")))?;
        if canonical.insert(index, id.clone()).is_some() {
            return Err(invalid(format!(
                "duplicate natural source identity {index}"
            )));
        }
    }
    for (id, (_, value)) in &items {
        if value.get("rfbBaseKind").is_some() {
            let index = identities[id];
            if canonical.get(&index).is_some_and(|existing| existing != id) {
                return Err(invalid(format!(
                    "duplicate canonical source identity {index}"
                )));
            }
            canonical.insert(index, id.clone());
        }
    }
    for (id, index) in &identities {
        if !items.contains_key(id) {
            return Err(invalid(format!("active item missing: {id}")));
        }
        canonical.entry(*index).or_insert_with(|| id.clone());
    }
    let source_by_index = entries
        .iter()
        .map(|entry| (entry.index, entry))
        .collect::<BTreeMap<_, _>>();
    for index in canonical.keys() {
        if !source_by_index.contains_key(index) {
            return Err(invalid(format!("source kind missing: {index}")));
        }
    }
    let mut writes = Vec::new();
    let mut rows = Vec::new();
    let mut coverage = Vec::new();
    let mut unresolved_names = Vec::new();
    let zh_path = pack.join("../../locales/zh-CN/content.ftl");
    let mut zh = fs::read_to_string(&zh_path)?;
    for entry in &entries {
        let id = canonical.get(&entry.index);
        let explicit =
            entry.flags.iter().any(|flag| flag == "INSTA_ART") || entry.allocations.is_empty();
        if let Some(id) = id {
            let (path, item) = items.get_mut(id).unwrap();
            let before = item.clone();
            let identity =
                json!({"sourceIndex": entry.index, "tval": entry.tval, "sval": entry.sval});
            if item.get("rfbBaseKind").is_some_and(|old| old != &identity) {
                return Err(invalid(format!("source identity drift: {id}")));
            }
            item["rfbBaseKind"] = identity;
            if entry.tval == 23 && item["weightTenthsPound"] != entry.weight_tenths_pound {
                return Err(invalid(format!(
                    "source sword weight required for rogue theme: {id}"
                )));
            }
            if let Some(name) = names
                .get(entry.index as usize)
                .and_then(Option::as_deref)
                .map(|name| chinese_display_name(entry, name))
                .filter(|name| !name.is_empty())
            {
                let key = item["nameKey"]
                    .as_str()
                    .ok_or_else(|| invalid("missing nameKey"))?;
                let actual = ftl_message_value(&zh, key)?;
                if actual != name {
                    let old = format!("{key} = {actual}");
                    if !zh.lines().any(|line| line == old) {
                        return Err(invalid(format!("multiline name {key}")));
                    }
                    zh = zh
                        .lines()
                        .map(|line| {
                            if line == old {
                                format!("{key} = {name}")
                            } else {
                                line.to_owned()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                        + "\n";
                }
            } else {
                unresolved_names.push(id.clone());
            }
            if *item != before {
                writes.push((path.clone(), serde_json::to_string_pretty(item)? + "\n"));
            }
            if !explicit {
                rows.extend(allocation_rows(entry, id));
            }
        }
        let blocker = adaptations
            .items
            .iter()
            .find(|item| {
                item.source_index == entry.index && item.status != DemoItemCoverageStatus::Active
            })
            .and_then(|item| item.blocker.as_deref());
        coverage.push(json!({
            "sourceIndex": entry.index, "sourceName": entry.name, "tval": entry.tval, "sval": entry.sval,
            "itemId": id, "status": if id.is_none() { "not-imported" }
                else if entry.flags.iter().any(|flag| flag == "INSTA_ART") { "explicit-only" }
                else if entry.allocations.is_empty() { "no-natural-allocation" } else { "mapped" },
            "hasNaturalAllocationRows": !entry.allocations.is_empty(),
            "reason": if id.is_none() { blocker.unwrap_or("source kind has no active formal implementation/mapping") }
                else if entry.flags.iter().any(|flag| flag == "INSTA_ART") { "INSTA_ART base; explicit artifact creation only" }
                else if entry.allocations.is_empty() { "source kind has no natural allocation rows" } else { "source allocation rows mapped without promotion" },
            "maxDepth": if entry.max_level == 0 { u16::MAX } else { entry.max_level },
            "allocations": entry.allocations.iter().enumerate().map(|(ordinal, row)| json!({
                "ordinal": ordinal, "minDepth": row.level, "chance": row.chance, "weight": 100 / row.chance,
            })).collect::<Vec<_>>(),
        }));
    }
    rows.sort_by_key(|row| {
        (
            row["minDepth"].as_u64().unwrap(),
            identities[row["itemKindId"].as_str().unwrap()],
        )
    });
    let count = rows.len();
    base["entries"] = json!(rows);
    base["kindSelection"] = json!({"kind": "rfb-base"});
    writes.push((base_path, serde_json::to_string_pretty(&base)? + "\n"));
    for name in [
        "warrior",
        "warrior-shoot",
        "archer",
        "mage",
        "priest",
        "evil-priest",
        "paladin",
        "evil-paladin",
        "samurai",
        "ninja",
        "rogue",
        "hobbit",
        "dwarf",
    ] {
        let path = pack.join(format!("lootTables/{name}.json"));
        let mut value: Value = serde_json::from_slice(&fs::read(&path)?)?;
        value.as_object_mut().unwrap().remove("entries");
        value["kindSelection"] = json!({"kind": "rfb-theme", "poolId": base["id"], "theme": match name {
            "evil-priest" => "priest-evil", "evil-paladin" => "paladin-evil", _ => name,
        }});
        writes.push((path, serde_json::to_string_pretty(&value)? + "\n"));
    }
    let explicit_items = items.keys().filter(|id| !canonical.values().any(|mapped| mapped == *id))
        .map(|id| json!({"itemId": id, "sourceIndex": identities.get(id), "status": "explicit-only",
            "reason": "fixed artifact, original content or adapted effect/device alias; no canonical source kind mapping"})).collect::<Vec<_>>();
    let report = json!({"schemaVersion": 1, "sourceCommit": commit, "sourceRef": "master",
        "source": K_INFO_SOURCE, "nameSource": K_NAME_ZH_SOURCE,
        "nameFormatSource": "src/flavor.c::object_desc (aware, no flavor; consumable suffixes)", "allocationRowCount": count,
        "unresolvedChineseNames": unresolved_names, "kinds": coverage, "explicitItems": explicit_items,
        "remainingRules": [],
        "acceptanceScope": "B0-B6 current playable builds and imported canonical pool; see design/base-allocation-acquirement-plan.md for evidence and object-representation limits"});
    writes.push((
        pack.join("legacy-base-allocation-audit.json"),
        serde_json::to_string_pretty(&report)? + "\n",
    ));
    writes.push((zh_path, zh));
    for (path, text) in writes {
        if fs::read_to_string(&path).ok().as_deref() != Some(&text) {
            fs::write(path, text)?;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formal_source_spellbooks_keep_their_executable_realm_and_rank() {
        let pack = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
        for file in fs::read_dir(pack.join("items")).unwrap() {
            let item: Value =
                serde_json::from_slice(&fs::read(file.unwrap().path()).unwrap()).unwrap();
            let Some(book_id) = item["abilityBookId"].as_str() else {
                continue;
            };
            let book_path = pack.join("abilityBooks").join(format!(
                "{}.json",
                book_id.strip_prefix("demo.ability-book.").unwrap()
            ));
            let book: Value = serde_json::from_slice(&fs::read(book_path).unwrap()).unwrap();
            let base = &item["rfbBaseKind"];
            // Craft is authored in the formal pack; the bulk legacy importer
            // does not emit its executable books. Source TV_CRAFT_BOOK is 97.
            if base["tval"] == 97 {
                assert_eq!(book["realmId"], "craft");
                assert!(base["sval"].as_u64().unwrap() < 4);
            } else {
                let source_book = player_ability_book_for_item(&LegacyItemEntry {
                    tval: base["tval"].as_u64().unwrap() as u16,
                    sval: base["sval"].as_u64().unwrap() as u16,
                    ..Default::default()
                })
                .unwrap_or_else(|| panic!("{} maps to an unimplemented source realm", item["id"]));
                assert!(
                    source_book.starts_with(&format!(
                        "rfb-legacy.ability-book.{}-",
                        book["realmId"].as_str().unwrap()
                    )),
                    "{} source realm differs from its executable book",
                    item["id"]
                );
            }
            assert_eq!(
                base["sval"].as_u64().unwrap() + 1,
                book["rank"].as_u64().unwrap(),
                "{} source rank",
                item["id"]
            );
        }
    }

    #[test]
    fn source_consumable_stems_keep_the_original_display_suffix() {
        let mut entry = LegacyItemEntry {
            tval: 75,
            ..Default::default()
        };
        assert_eq!(chinese_display_name(&entry, "治愈"), "治愈药水");
        entry.sval = 14;
        assert_eq!(chinese_display_name(&entry, "诗歌蜜酒"), "诗歌蜜酒");
        entry.tval = 70;
        assert_eq!(chinese_display_name(&entry, "*鉴定*"), "*鉴定*卷轴");
        entry.tval = 80;
        entry.name = "Restoring:Red".into();
        assert_eq!(chinese_display_name(&entry, "完全恢复"), "完全恢复蘑菇");
        entry.name = "Ration of Food".into();
        assert_eq!(chinese_display_name(&entry, "食物口粮"), "食物口粮");
    }

    #[test]
    fn allocation_rows_preserve_integer_zero_repetitions_and_maximum_depth() {
        let entry = LegacyItemEntry {
            max_level: 45,
            allocations: vec![
                LegacyItemAllocation {
                    level: 3,
                    chance: 255,
                },
                LegacyItemAllocation {
                    level: 9,
                    chance: 3,
                },
                LegacyItemAllocation {
                    level: 9,
                    chance: 3,
                },
            ],
            ..Default::default()
        };
        let rows = allocation_rows(&entry, "test.item.source");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0]["weight"], 0);
        assert_eq!(rows[1]["weight"], 33);
        assert_eq!(rows[1], rows[2]);
        assert_eq!(rows[1]["maxDepth"], 45);
        assert_eq!(
            allocation_rows(
                &LegacyItemEntry {
                    max_level: 0,
                    ..entry
                },
                "test.item.source"
            )[0]["maxDepth"],
            u16::MAX
        );
    }
}
