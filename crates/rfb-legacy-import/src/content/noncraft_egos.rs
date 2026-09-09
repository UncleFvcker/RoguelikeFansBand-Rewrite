// SPDX-License-Identifier: MPL-2.0

use super::*;

pub fn sync_demo_noncraft_egos(
    source: &Path,
    pack_root: &Path,
) -> Result<usize, LegacyImportError> {
    let commit = resolve_legacy_content_commit(source)?;
    let egos = parse_e_info(&read_legacy_object_at(source, &commit, E_INFO_SOURCE)?)?;
    let names = parse_chinese_name_table(
        &read_legacy_object_at(source, &commit, E_NAME_ZH_SOURCE)?,
        E_NAME_ZH_SOURCE,
    )?;
    let devices = read_legacy_object_at(source, &commit, DEVICES_C_SOURCE)?;
    let candidates = parse_effect_info_activation_candidates(&devices)?;
    let mut activation_names = BTreeMap::new();
    let mut tokens = Vec::new();
    for line in devices.lines().map(str::trim) {
        if let Some(token) = line
            .strip_prefix("case EFFECT_")
            .and_then(|line| line.split_once(':').map(|(token, _)| token))
        {
            tokens.push(token.to_owned());
        }
        if let Some(name) = line
            .strip_prefix("if (name) return \"")
            .and_then(|line| line.strip_suffix("\";"))
        {
            for token in tokens.drain(..) {
                activation_names.insert(token, name.to_owned());
            }
        }
    }
    let mut used_activations = BTreeSet::new();
    let mut locale = Vec::new();
    for (index, id) in [
        (200, "defender-jewelry"),
        (201, "elemental-jewelry"),
        (205, "protection-ring"),
        (206, "combat-ring"),
        (207, "archery-ring"),
        (208, "wizardry-ring"),
        (209, "speed-ring"),
        (210, "nazgul-ring"),
        (211, "dwarves-ring"),
        (220, "barbarian-talisman"),
        (221, "sacred-pendant"),
        (222, "hell-harness"),
        (223, "dwarven-necklace"),
        (224, "magi-amulet"),
        (225, "hero-torc"),
        (226, "devotion-amulet"),
        (227, "trickery-amulet"),
        (260, "blasted"),
        (235, "extra-light-light"),
        (236, "illumination-light"),
        (237, "duration-light"),
        (238, "infravision-light"),
        (239, "immolation-light"),
        (240, "darkness-light"),
        (241, "immortal-eye-light"),
        (242, "valinor-light"),
        (243, "scrying-light"),
        (265, "holding-quiver"),
        (266, "quiver-protection"),
        (267, "endless-quiver"),
        (268, "phase-quiver"),
        (250, "resistance-device"),
        (251, "capacity-device"),
        (252, "regeneration-device"),
        (253, "simplicity-device"),
        (254, "power-device"),
        (255, "holding-device"),
        (256, "quickness-device"),
    ] {
        let entry = egos.iter().find(|entry| entry.index == index).unwrap();
        let name = names
            .get(index as usize)
            .and_then(Option::as_ref)
            .ok_or_else(|| {
                LegacyImportError::InvalidEgoAudit(format!("unresolved Chinese ego name: {index}"))
            })?;
        let mut value = ego_json(entry, id, &mut ContentImportReport::default());
        value.as_object_mut().unwrap().remove("rollGroups");
        if index == 256 {
            value["tags"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!("quickness"));
        }
        if index == 266 {
            value["protectsQuiverAmmunition"] = serde_json::json!(true);
            value["preservesOrdinaryQuality"] = serde_json::json!(true);
        }
        if let Some(activations) = armor_ego_audit::armor_activations(entry, id, &candidates) {
            for activation in &activations {
                used_activations.insert(activation["nameKey"].as_str().unwrap().to_owned());
            }
            value["deviceGeneration"] = serde_json::json!({
                "activationOptional": entry.activation.is_none(), "activations": activations,
            });
        }
        if index == 267 {
            used_activations.insert("device-activation-e5-endless-quiver-name".to_owned());
            let activation = entry.activation.as_ref().unwrap();
            value["deviceGeneration"] = serde_json::json!({"activations":[{
                "id":"rfb.device-activation.endless-quiver", "nameKey":"device-activation-e5-endless-quiver-name",
                "weight":1,"minDepth":1,"maxDepth":100,"deviceCheckDifficulty":activation.power,
                "charges":{"minimum":1,"maximum":1,"cost":1},
                "recovery":{"intervalTicks":activation.recovery_turns * 10,"energyPerMille":1000},
                "target":device_self_target(),"effect":{"type":"refill-quiver"}
            }]});
        }
        fs::write(
            pack_root.join("affixes").join(format!("{id}.json")),
            serde_json::to_string_pretty(&value)? + "\n",
        )?;
        locale.push((id.to_owned(), entry.name.clone(), name.clone()));
    }
    let kinds = parse_k_info(&read_legacy_object_at(source, &commit, K_INFO_SOURCE)?)?;
    for (id, tval) in [("ring", 45), ("amulet", 40)] {
        let kind = kinds
            .iter()
            .find(|kind| kind.tval == tval && kind.sval == 0)
            .unwrap();
        let path = pack_root.join("items").join(format!("{id}.json"));
        let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)?;
        value["rfbBaseKind"] = serde_json::json!({"sourceIndex":kind.index,"tval":tval,"sval":0});
        fs::write(path, serde_json::to_string_pretty(&value)? + "\n")?;
    }
    let quiver = kinds
        .iter()
        .find(|kind| kind.tval == 46 && kind.sval == 0)
        .unwrap();
    let quiver_path = pack_root.join("items/quiver.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&quiver_path)?)?;
    value["rfbBaseKind"] = serde_json::json!({"sourceIndex":quiver.index,"tval":46,"sval":0});
    fs::write(quiver_path, serde_json::to_string_pretty(&value)? + "\n")?;
    for (index, id, sval) in [
        (600, "wooden-torch", 0),
        (601, "brass-lantern", 1),
        (603, "feanorian-lamp", 2),
    ] {
        let kind = kinds.iter().find(|kind| kind.index == index).unwrap();
        assert_eq!((kind.tval, kind.sval), (39, sval));
        let path = pack_root.join("items").join(format!("{id}.json"));
        let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)?;
        value["rfbBaseKind"] = serde_json::json!({"sourceIndex":index,"tval":39,"sval":sval});
        if sval == 2 {
            value["equipmentBonuses"]["lightRadius"] = serde_json::json!(2);
        }
        fs::write(path, serde_json::to_string_pretty(&value)? + "\n")?;
    }
    let root = pack_root
        .parent()
        .and_then(Path::parent)
        .unwrap_or(pack_root);
    let table_path = pack_root.join("lootTables/base-items.json");
    let table: serde_json::Value = serde_json::from_slice(&fs::read(&table_path)?)?;
    let entries = table["entries"].as_array().unwrap();
    let mut additions = Vec::new();
    if !entries
        .iter()
        .any(|entry| entry["itemKindId"] == "demo.item.quiver")
    {
        additions.push(serde_json::json!({"itemKindId":"demo.item.quiver", "weight":100, "quantity":1, "minDepth":20}));
    }
    if !entries
        .iter()
        .any(|entry| entry["itemKindId"] == "demo.item.feanorian-lamp")
    {
        additions.push(serde_json::json!({"itemKindId":"demo.item.feanorian-lamp", "weight":100, "quantity":1, "minDepth":15, "maxDepth":40}));
    }
    armor_ego_audit::append_source_array_entries(&table_path, "entries", &additions)?;
    for (language, chinese) in [("en-US", false), ("zh-CN", true)] {
        write_ego_locale_block(
            &root.join(format!("locales/{language}/content.ftl")),
            &locale,
            chinese,
            "# E7 non-Craft egos (generated)",
            "# /E7 non-Craft egos",
            "An RFB ego.",
            "RFB Ego。",
        )?;
        let path = root.join(format!("locales/{language}/content.ftl"));
        let mut text = fs::read_to_string(&path)?;
        for key in &used_activations {
            if text
                .lines()
                .any(|line| line.starts_with(&format!("{key} =")))
            {
                continue;
            }
            let token = key
                .strip_prefix("device-activation-e5-")
                .unwrap()
                .strip_suffix("-name")
                .unwrap()
                .to_ascii_uppercase()
                .replace('-', "_");
            let name = activation_names.get(&token).ok_or_else(|| {
                LegacyImportError::InvalidEgoAudit(format!(
                    "unresolved Chinese activation name: {token}"
                ))
            })?;
            text.push_str(&format!(
                "{key} = {}\n",
                if chinese {
                    name.clone()
                } else {
                    token.to_ascii_lowercase().replace('_', " ")
                }
            ));
        }
        fs::write(path, text)?;
    }
    Ok(locale.len())
}
