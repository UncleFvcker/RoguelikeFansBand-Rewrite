// SPDX-License-Identifier: MPL-2.0

use super::*;

pub fn sync_demo_armor_egos(source: &Path, pack_root: &Path) -> Result<usize, LegacyImportError> {
    let commit = resolve_legacy_content_commit(source)?;
    let profiles = parse_s_info(&read_legacy_object_at(source, &commit, S_INFO_SOURCE)?)?;
    let classes = parse_class_registrations(
        &read_legacy_object_at(source, &commit, "src/defines.h")?,
        &read_legacy_object_at(source, &commit, "src/classes.c")?,
    );
    for class in classes {
        let path = pack_root.join(format!("classes/{}.json", class.id));
        if !path.exists() {
            continue;
        }
        let profile = profiles
            .iter()
            .find(|profile| profile.class_index == class.index)
            .unwrap();
        let dual = profile
            .skill_entries
            .iter()
            .find(|skill| skill.skill_index == 1)
            .unwrap();
        assert_eq!(dual.initial, 0);
        let text = fs::read_to_string(&path)?;
        let key = "  \"dualWieldingMaximum\": ";
        let text = if let Some(start) = text.find(key) {
            let end = start + text[start..].find(',').unwrap();
            format!("{}{}{}{}", &text[..start], key, dual.maximum, &text[end..])
        } else {
            text.replacen(
                "  \"formatVersion\": 1,",
                &format!("  \"formatVersion\": 1,\n{key}{},", dual.maximum),
                1,
            )
        };
        fs::write(path, text)?;
    }
    let egos = parse_e_info(&read_legacy_object_at(source, &commit, E_INFO_SOURCE)?)?;
    let names = parse_chinese_name_table(
        &read_legacy_object_at(source, &commit, E_NAME_ZH_SOURCE)?,
        E_NAME_ZH_SOURCE,
    )?;
    validate_armor_ego_contract(&egos, &names)?;
    let devices = read_legacy_object_at(source, &commit, DEVICES_C_SOURCE)?;
    let candidates = parse_effect_info_activation_candidates(&devices)?;
    let mut activation_names = BTreeMap::new();
    let mut tokens = Vec::new();
    for line in devices.lines().map(str::trim) {
        if let Some(token) = line
            .strip_prefix("case EFFECT_")
            .and_then(|s| s.split_once(':').map(|(token, _)| token))
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
    for language in ["en-US", "zh-CN"] {
        let path = root.join(format!("locales/{language}/content.ftl"));
        let mut text = fs::read_to_string(&path)?;
        for label in ["bases", "egos", "activations"] {
            let start_marker = format!("# E5 armor {label} (generated)");
            let end_marker = format!("# /E5 armor {label}");
            if let Some(start) = text.find(&start_marker) {
                let end = start + text[start..].find(&end_marker).unwrap() + end_marker.len();
                let start = text[..start].trim_end().len();
                text.replace_range(start..end, "");
            }
        }
        fs::write(path, text.trim_end().to_owned() + "\n")?;
    }
    let breath_profiles = sync_armor_bases(source, &commit, pack_root)?;
    let contract: Vec<ArmorEgoContract> =
        serde_json::from_str(include_str!("armor-ego-contract.json"))?;
    let mut locale = Vec::new();
    let mut report = ContentImportReport::default();
    let mut used_activations = BTreeSet::from([
        "BREATHE_ONE_MULTIHUED".to_owned(),
        "BREATHE_ONE_LAW".to_owned(),
        "BREATHE_SOUND".to_owned(),
        "BREATHE_ONE_CHAOS".to_owned(),
    ]);
    for expected in &contract {
        let entry = egos
            .iter()
            .find(|entry| entry.index == expected.source_index)
            .unwrap();
        let id = expected.affix_id.strip_prefix("rfb-legacy.affix.").unwrap();
        let mut value = ego_json(entry, id, &mut report);
        if entry.index != 50 {
            value.as_object_mut().unwrap().remove("rollGroups");
        }
        if let Some(activations) = armor_activations(entry, id, &candidates) {
            for activation in &activations {
                used_activations.insert(
                    activation["nameKey"]
                        .as_str()
                        .unwrap()
                        .strip_prefix("device-activation-e5-")
                        .unwrap()
                        .strip_suffix("-name")
                        .unwrap()
                        .to_ascii_uppercase()
                        .replace('-', "_"),
                );
            }
            value["deviceGeneration"] = serde_json::json!({"activationOptional": entry.activation.is_none(), "activations": activations});
        }
        if entry.index == 86 {
            value["deviceGeneration"] = serde_json::json!({"activations": breath_profiles});
        }
        fs::write(
            pack_root.join("affixes").join(format!("{id}.json")),
            serde_json::to_string_pretty(&value)? + "\n",
        )?;
        locale.push((
            id.to_owned(),
            entry.name.clone(),
            expected.chinese_name.clone(),
        ));
    }
    let root = pack_root
        .parent()
        .and_then(Path::parent)
        .unwrap_or(pack_root);
    for (language, chinese) in [("en-US", false), ("zh-CN", true)] {
        write_ego_locale_block(
            &root.join(format!("locales/{language}/content.ftl")),
            &locale,
            chinese,
            "# E5 armor egos (generated)",
            "# /E5 armor egos",
            "An RFB armor ego.",
            "RFB 护甲 Ego。",
        )?;
        let path = root.join(format!("locales/{language}/content.ftl"));
        let mut text = fs::read_to_string(&path)?;
        text.push_str("\n# E5 armor activations (generated)\n");
        for token in &used_activations {
            let name = activation_names.get(token).ok_or_else(|| {
                LegacyImportError::InvalidEgoAudit(format!(
                    "unresolved Chinese activation name: {token}"
                ))
            })?;
            let english = token.to_ascii_lowercase().replace('_', " ");
            text.push_str(&format!(
                "device-activation-e5-{}-name = {}\n",
                token.to_ascii_lowercase().replace('_', "-"),
                if chinese { name } else { &english }
            ));
        }
        text.push_str("# /E5 armor activations\n");
        fs::write(path, text)?;
    }
    Ok(locale.len())
}

fn sync_armor_bases(
    source: &Path,
    commit: &str,
    pack_root: &Path,
) -> Result<Vec<serde_json::Value>, LegacyImportError> {
    let entries = parse_k_info(&read_legacy_object_at(source, commit, K_INFO_SOURCE)?)?;
    let names = parse_chinese_name_table(
        &read_legacy_object_at(source, commit, K_NAME_ZH_SOURCE)?,
        K_NAME_ZH_SOURCE,
    )?;
    let ammo = launcher_ammo_index(&entries);
    let selection_path = pack_root.join("legacy-item-selection.json");
    let selection: serde_json::Value = serde_json::from_slice(&fs::read(&selection_path)?)?;
    let mut new_selection = Vec::new();
    let mut locale = Vec::new();
    let mut breaths = Vec::new();
    for (index, id, tval, sval, power, recovery, damage, elements) in [
        (209, "mithril-shod-boots", 30, 6, 0, 0, 0, &[][..]),
        (220, "iron-crown", 33, 10, 0, 0, 0, &[][..]),
        (224, "knit-cap", 32, 1, 0, 0, 0, &[][..]),
        (225, "pointy-hat", 32, 10, 0, 0, 0, &[][..]),
        (261, "yoiyami-robe", 36, 60, 0, 0, 0, &[][..]),
        (
            297,
            "multi-hued-dragon-scale-mail",
            38,
            6,
            40,
            70,
            250,
            &["acid", "electricity", "fire", "cold", "poison"][..],
        ),
        (
            299,
            "law-dragon-scale-mail",
            38,
            12,
            50,
            100,
            230,
            &["sound", "shards"][..],
        ),
        (
            302,
            "gold-dragon-scale-mail",
            38,
            16,
            30,
            40,
            130,
            &["sound"][..],
        ),
        (
            303,
            "chaos-dragon-scale-mail",
            38,
            18,
            50,
            100,
            220,
            &["chaos", "disenchant"][..],
        ),
    ] {
        let entry = entries
            .iter()
            .find(|entry| entry.index == index)
            .ok_or_else(|| {
                LegacyImportError::InvalidDemoItemSelection(format!("missing armor base {index}"))
            })?;
        if (entry.tval, entry.sval) != (tval, sval) {
            return Err(LegacyImportError::InvalidDemoItemSelection(format!(
                "armor base {index} changed"
            )));
        }
        let chinese = names
            .get(index as usize)
            .and_then(Option::as_deref)
            .ok_or_else(|| {
                LegacyImportError::InvalidDemoItemSelection(format!(
                    "armor base {index} has no Chinese name"
                ))
            })?;
        let mut report = ContentImportReport::default();
        let mut value = finalize_demo_item_json(
            item_json_with_terrain(entry, id, &ammo, None, None, &mut report),
            id,
        );
        value["rfbBaseKind"] = serde_json::json!({"sourceIndex": index,"tval":tval,"sval":sval});
        if tval == 38 {
            value["tags"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!("activatable"));
            let effect = |damage| {
                if elements.len() == 1 {
                    device_ability_effect(
                        serde_json::json!({"type":"cone-damage","damageDice":0,"damageSides":0,"damageBonus":damage,"damageType":elements[0],"radius":2}),
                    )
                } else {
                    serde_json::json!({"type":"random-element-cone-damage","damage":damage,"damageTypes":elements,"radius":2})
                }
            };
            let profile = |enhanced| {
                serde_json::json!({
                    "id":format!("rfb.device-activation.{}-{index}", if enhanced {"ego-breath"} else {"dragon-breath"}),
                    "nameKey":format!("device-activation-e5-{}-name", match index {297 => "breathe-one-multihued", 299 => "breathe-one-law", 302 => "breathe-sound", _ => "breathe-one-chaos"}),"weight":1,"minDepth":1,"maxDepth":100,
                    "deviceCheckDifficulty":power,"charges":{"minimum":1,"maximum":1,"cost":1},
                    "recovery":{"intervalTicks":recovery * if enhanced {5} else {10},"energyPerMille":1000},
                    "target":{"modes":["direction"],"range":18,"requiresLineOfEffect":true},
                    "effect":effect(damage * if enhanced {2} else {1})
                })
            };
            breaths.push(profile(true));
            // Keep the existing multi-hued item's stable activation identity and program.
            if index != 297 {
                let mut base_profile = profile(false);
                let effect = base_profile
                    .as_object_mut()
                    .unwrap()
                    .remove("effect")
                    .unwrap();
                let program_id = format!("rfb.effect.dragon-breath-{index}");
                let mut program = effect_program_from_inline(&program_id, effect)
                    .map_err(LegacyImportError::InvalidDemoItemSelection)?;
                program["input"] = serde_json::json!("actor");
                fs::write(
                    pack_root.join(format!("effectPrograms/dragon-breath-{index}.json")),
                    serde_json::to_string_pretty(&program)? + "\n",
                )?;
                base_profile["effectProgramId"] = serde_json::json!(program_id);
                value["deviceGeneration"] = serde_json::json!({"activations":[base_profile]});
            }
        }
        if index != 297 {
            fs::write(
                pack_root.join(format!("items/{id}.json")),
                serde_json::to_string_pretty(&value)? + "\n",
            )?;
            if !selection["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["id"] == id)
            {
                new_selection.push(
                    serde_json::json!({"sourceIndex":index,"sourceId":kebab(&entry.name),"id":id}),
                );
            }
            locale.push((
                id,
                singular_english_kind_name(&entry.name),
                singular_chinese_kind_name(chinese),
            ));
        }
    }
    append_source_array_entries(&selection_path, "items", &new_selection)?;
    let loot_path = pack_root.join("lootTables/base-items.json");
    let loot: serde_json::Value = serde_json::from_slice(&fs::read(&loot_path)?)?;
    let mut new_loot = Vec::new();
    for (id, weight, depth) in [
        ("mithril-shod-boots", 50, 50),
        ("iron-crown", 100, 30),
        ("knit-cap", 33, 3),
        ("pointy-hat", 20, 10),
        ("law-dragon-scale-mail", 12, 60),
        ("gold-dragon-scale-mail", 20, 50),
        ("chaos-dragon-scale-mail", 10, 65),
    ] {
        let id = format!("demo.item.{id}");
        if !loot["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["itemKindId"] == id)
        {
            new_loot.push(
                serde_json::json!({"itemKindId":id,"weight":weight,"quantity":1,"minDepth":depth}),
            );
        }
    }
    append_source_array_entries(&loot_path, "entries", &new_loot)?;
    let root = pack_root
        .parent()
        .and_then(Path::parent)
        .unwrap_or(pack_root);
    for (language, chinese) in [("en-US", false), ("zh-CN", true)] {
        let path = root.join(format!("locales/{language}/content.ftl"));
        let mut source = fs::read_to_string(&path)?;
        if let Some(start) = source.find("# E5 armor bases (generated)") {
            let end = source[start..].find("# /E5 armor bases").unwrap()
                + start
                + "# /E5 armor bases".len();
            source.replace_range(start..end, "");
        }
        let mut source = source
            .lines()
            .filter(|line| {
                !locale.iter().any(|(id, _, _)| {
                    line.starts_with(&format!("item-demo-{id}-name ="))
                        || line.starts_with(&format!("item-demo-{id}-description ="))
                })
            })
            .collect::<Vec<_>>()
            .join("\n")
            .trim_end()
            .to_owned();
        source.push_str("\n\n# E5 armor bases (generated)");
        for (id, en, zh) in &locale {
            let name = if chinese { zh } else { en };
            source.push_str(&format!(
                "\nitem-demo-{id}-name = {name}\nitem-demo-{id}-description = {name}\n"
            ));
        }
        source.push_str("# /E5 armor bases\n");
        fs::write(path, source)?;
    }
    Ok(breaths)
}

pub(super) fn append_source_array_entries(
    path: &Path,
    key: &str,
    entries: &[serde_json::Value],
) -> Result<(), LegacyImportError> {
    if entries.is_empty() {
        return Ok(());
    }
    let mut source = fs::read_to_string(path)?;
    let start = source.find(&format!("\"{key}\": [")).unwrap();
    let end = source[start..].find("\n  ]").unwrap() + start;
    let mut addition = String::new();
    for entry in entries {
        addition.push_str(",\n    ");
        addition.push_str(&serde_json::to_string(entry)?);
    }
    source.insert_str(end, &addition);
    fs::write(path, source)?;
    Ok(())
}

pub(super) fn armor_activations(
    entry: &LegacyEgoEntry,
    _id: &str,
    candidates: &[LegacyEgoActivationCandidate],
) -> Option<Vec<serde_json::Value>> {
    let eligible = |candidate: &&LegacyEgoActivationCandidate| {
        if candidate.rarity == 0 {
            return false;
        }
        if let Some(activation) = &entry.activation {
            return candidate.token == activation.token;
        }
        match entry.index {
            209 => matches!(
                candidate.token.as_str(),
                "SPEED" | "SPEED_HERO" | "LIGHT_SPEED"
            ),
            220 => candidate.token == "BERSERK",
            200..=201 | 205..=208 | 210..=211 | 221..=222 | 224..=227 => {
                let biases: &[&str] = match entry.index {
                    200 | 205 => &["BIAS_PROTECTION"],
                    201 => &[
                        "BIAS_ACID",
                        "BIAS_ELEC",
                        "BIAS_FIRE",
                        "BIAS_COLD",
                        "BIAS_POIS",
                    ],
                    206 => &["BIAS_WARRIOR", "BIAS_STR"],
                    207 => &["BIAS_ARCHER"],
                    208 | 224 => &["BIAS_MAGE"],
                    210 => &["BIAS_NECROMANTIC"],
                    211 | 226 => &["BIAS_PRIESTLY"],
                    221 => &["BIAS_LAW"],
                    222 => &["BIAS_DEMON"],
                    225 => &["BIAS_WARRIOR"],
                    _ => &["BIAS_ROGUE"],
                };
                candidate.level >= entry.level / 3
                    && candidate
                        .biases
                        .iter()
                        .any(|bias| biases.contains(&bias.as_str()))
            }
            242 => matches!(
                candidate.token.as_str(),
                "LITE_AREA" | "LITE_MAP_AREA" | "ENLIGHTENMENT" | "CLAIRVOYANCE"
            ),
            51 => candidate.biases.iter().any(|bias| {
                matches!(
                    bias.as_str(),
                    "BIAS_ACID" | "BIAS_ELEC" | "BIAS_FIRE" | "BIAS_COLD" | "BIAS_POIS"
                )
            }),
            70 => {
                candidate.level >= entry.level / 3
                    && candidate.biases.iter().any(|bias| bias == "BIAS_WARRIOR")
            }
            72 => candidate.token == "BERSERK",
            146 => candidate.token == "PHASE_DOOR",
            150 => candidate.token == "SPEED",
            73..=75 => {
                candidate.level >= entry.level / 3
                    && candidate.biases.iter().any(|bias| bias == "BIAS_DEMON")
            }
            85 => matches!(
                candidate.token.as_str(),
                "IDENTIFY_FULL"
                    | "DETECT_ALL"
                    | "ENLIGHTENMENT"
                    | "CLAIRVOYANCE"
                    | "SELF_KNOWLEDGE"
            ),
            92 => matches!(
                candidate.token.as_str(),
                "GENOCIDE" | "MASS_GENOCIDE" | "WRAITHFORM" | "DARKNESS_STORM"
            ),
            96..=98 | 103 | 104 | 122 | 126..=128 => {
                let bias = match entry.index {
                    96 => "BIAS_FIRE",
                    97 => "BIAS_ELEC",
                    98 => "BIAS_COLD",
                    103 => "BIAS_NECROMANTIC",
                    104 | 127 => "BIAS_WARRIOR",
                    122 => "BIAS_DEMON",
                    126 => "BIAS_MAGE",
                    _ => "BIAS_PRIESTLY",
                };
                candidate.level >= entry.level / 3
                    && candidate.biases.iter().any(|value| value == bias)
            }
            110 => matches!(
                candidate.token.as_str(),
                "IDENTIFY"
                    | "IDENTIFY_FULL"
                    | "PROBING"
                    | "DETECT_TRAPS"
                    | "DETECT_MONSTERS"
                    | "DETECT_OBJECTS"
                    | "DETECT_ALL"
                    | "ENLIGHTENMENT"
                    | "CLAIRVOYANCE"
                    | "SELF_KNOWLEDGE"
            ),
            111 => matches!(
                candidate.token.as_str(),
                "HEAL"
                    | "CURING"
                    | "RESTORE_STATS"
                    | "RESTORE_EXP"
                    | "HEAL_CURING"
                    | "CURE_POIS"
                    | "CURE_FEAR"
                    | "REMOVE_CURSE"
                    | "REMOVE_ALL_CURSE"
                    | "CLARITY"
            ),
            117 => matches!(
                candidate.token.as_str(),
                "LITE_AREA"
                    | "LITE_MAP_AREA"
                    | "BOLT_LITE"
                    | "BEAM_LITE_WEAK"
                    | "BEAM_LITE"
                    | "BALL_LITE"
                    | "BREATHE_LITE"
                    | "CONFUSING_LITE"
            ),
            _ => false,
        }
    };
    let profiles: Vec<_> = candidates.iter().filter(eligible).map(|candidate| {
        let (effect, target, ground) = match candidate.token.as_str() {
            "STAR_BALL" => (serde_json::json!({"type":"star-ball"}), device_self_target(), false),
            "ESCAPE" => (serde_json::json!({"type":"escape"}), device_self_target(), false),
            "DESTROY_TRAPS" => (device_ability_effect(serde_json::json!({"type":"terrain-beam", "operation":"destroy-traps-and-doors"})), device_projectile_target(), false),
            "STARBURST" => (serde_json::json!({"type":"starburst", "damage":375 + device_power_curve(200, candidate.level, 80)}), device_self_target(), false),
            "WRATH_OF_GOD" => (device_ability_effect(serde_json::json!({"type":"wrath-of-god", "damage":25 + candidate.level * 3 / 2})), device_projectile_target(), true),
            "EARTHQUAKE" => (device_ability_effect(serde_json::json!({"type":"earthquake","radius":10,"affectChancePercent":15,"floorTerrainId":"demo.terrain.floor","wallTerrainIds":["demo.terrain.wall","demo.terrain.quartz-vein","demo.terrain.magma-vein"]})), device_self_target(), false),
            "LIGHT_SPEED" => (device_status_effect("rfb.status.light-speed", 0, 0, 16), device_self_target(), false),
            "DETECT_ALL" => (device_ability_effect(serde_json::json!({"type":"sequence", "effects": [
                {"type":"detect","subject":"terrain","category":"trap","radius":30,"persistent":true,"throughWalls":true},
                {"type":"detect","subject":"terrain","category":"passage","radius":30,"persistent":true,"throughWalls":true},
                {"type":"detect","subject":"gold","category":"gold","radius":30,"persistent":false,"throughWalls":true},
                {"type":"detect","subject":"item","category":"item","radius":30,"persistent":false,"throughWalls":true},
                {"type":"detect","subject":"actor","category":"any-monster","radius":30,"persistent":false,"throughWalls":true}
            ]})), device_self_target(), false),
            "WHIRLWIND_ATTACK" => (device_ability_effect(serde_json::json!({"type":"melee-adjacent"})), device_self_target(), false),
            "PHASE_DOOR" | "TELEPORT" => (serde_json::json!({"type":"random-teleport", "maximumDistance":if candidate.token == "PHASE_DOOR" {10} else {100}}), device_self_target(), false),
            "STRAFING" => (device_ability_effect(serde_json::json!({"type":"blink-self", "radius":10, "lineOfSight":true})), device_self_target(), false),
            "SLEEP_MONSTERS" => (serde_json::json!({"type":"visible-apply-status", "statusKindId":"rfb.status.sleep", "intensity":1, "durationTicks":500, "stacking":"replace", "power":candidate.level * 3}), device_self_target(), false),
            "IDENTIFY" => (serde_json::json!({"type":"identify-item","full":false}), device_item_target(), false),
            "DETECT_MONSTERS" | "DETECT_OBJECTS" | "DETECT_TRAPS" => {
                let (subject, category, persistent) = match candidate.token.as_str() {
                    "DETECT_MONSTERS" => ("actor", "normal-monster", false),
                    "DETECT_OBJECTS" => ("item", "item", false),
                    _ => ("terrain", "trap", true),
                };
                (device_ability_effect(serde_json::json!({"type":"detect", "subject":subject,"category":category,"radius":30,"persistent":persistent,"throughWalls":true})), device_self_target(), false)
            }
            "LITE_AREA" | "LITE_MAP_AREA" => {
                let light = serde_json::json!({"type":"light-area","damageDice":2+candidate.level/20,"damageSides":15,"radius":3});
                let effect = if candidate.token == "LITE_MAP_AREA" { serde_json::json!({"type":"sequence","effects":[{"type":"detect","subject":"terrain","category":"all","radius":30,"persistent":true,"throughWalls":true}, light]}) } else { light };
                (device_ability_effect(effect), device_self_target(), false)
            }
            "BOLT_MISSILE" => (device_damage_effect("damage", "missile", 2 + candidate.level / 10, 6, 0, 0), device_projectile_target(), false),
            "BOLT_SOUND" => (device_damage_effect("damage", "sound", 7 + candidate.level / 6, 8, 0, 0), device_projectile_target(), false),
            "BEAM_SOUND" => (device_damage_effect("beam-damage", "sound", 7 + candidate.level / 6, 8, 0, 0), device_projectile_target(), false),
            "BALL_SOUND" => (device_damage_effect("area-damage", "sound", 0, 0, 70 + device_power_curve(280, candidate.level, 40), 3), device_projectile_target(), false),
            "BALL_SHARDS" => (device_damage_effect("area-damage", "shards", 0, 0, 175 + device_power_curve(325, candidate.level, 75), 2), device_projectile_target(), false),
            "BREATHE_SOUND" => (device_damage_effect("cone-damage", "sound", 0, 0, 50 + candidate.level * 2, 2), device_projectile_target(), false),
            "BREATHE_SHARDS" => (device_damage_effect("cone-damage", "shards", 0, 0, 100 + candidate.level * 2, 2), device_projectile_target(), false),
            "BOLT_SHARDS" => (device_damage_effect("damage", "shards", 7 + candidate.level / 5, 8, 0, 0), device_projectile_target(), false),
            "BOLT_LITE" => (device_damage_effect("damage", "light", 5 + candidate.level / 8, 8, 0, 0), device_projectile_target(), false),
            "BEAM_LITE_WEAK" => (device_ability_effect(serde_json::json!({"type":"light-line","damageDice":6,"damageSides":8})), device_projectile_target(), false),
            "BEAM_LITE" => (device_damage_effect("beam-damage", "light", 0, 0, 10 + device_power_curve(275, candidate.level, 0), 0), device_projectile_target(), false),
            "BALL_LITE" => (device_damage_effect("area-damage", "light", 0, 0, 200 + device_power_curve(350, candidate.level, 80), 4), device_projectile_target(), false),
            "BREATHE_LITE" => (device_damage_effect("cone-damage", "light", 0, 0, 50 + candidate.level * 2, 2), device_projectile_target(), false),
            "STONE_SKIN" => (serde_json::json!({"type":"apply-stone-skin","durationDice":1,"durationSides":20,"durationBonus":20}), device_self_target(), false),
            "BERSERK" => (serde_json::json!({"type":"apply-berserk-strength","durationDice":1,"durationSides":25,"durationBonus":25}), device_self_target(), false),
            "SPEED_HERO" => (serde_json::json!({"type":"apply-heroic-speed","durationDice":1,"durationSides":candidate.level / 2,"durationBonus":candidate.level / 2}), device_self_target(), false),
            _ => legacy_device_item_effect(candidate).unwrap_or_else(|| panic!("front armor activation {} requires an implemented effect", candidate.token)),
        };
        let mut effect = effect;
        if ground { effect["affectsGroundItems"] = serde_json::json!(true); }
        let fixed = entry.activation.as_ref();
        let biases: Vec<_> = if fixed.is_some() { Vec::new() } else { candidate.biases.iter().filter_map(|bias| match bias.as_str() {
            "BIAS_WARRIOR" => Some("warrior"), "BIAS_MAGE" => Some("mage"), "BIAS_CHAOS" => Some("chaos"),
            "BIAS_ACID" => Some("acid"), "BIAS_ELEC" => Some("electricity"), "BIAS_FIRE" => Some("fire"),
            "BIAS_COLD" => Some("cold"), "BIAS_POIS" => Some("poison"), "BIAS_PRIESTLY" => Some("priestly"),
            "BIAS_DEMON" => Some("demon"), "BIAS_NECROMANTIC" => Some("necromantic"), "BIAS_RANGER" => Some("ranger"), _ => None,
        }).collect() };
        serde_json::json!({
            "id": format!("rfb.device-activation.ego-{}-{}",entry.index,candidate.token.to_ascii_lowercase().replace('_',"-")),
            "nameKey": format!("device-activation-e5-{}-name", candidate.token.to_ascii_lowercase().replace('_', "-")),
            "weight": if fixed.is_some() {1} else {(255 / u32::from(candidate.rarity)).max(1)},
            "minDepth": 1,
            "maxDepth": if fixed.is_some() || matches!(entry.index,85|92|110|111|117|209|220|242) {100} else {candidate.level.saturating_mul(3).saturating_add(2).min(100)},
            "deviceCheckDifficulty": fixed.map_or(candidate.level, |activation| activation.power),
            "rfbBiases": biases,
            "charges": {"minimum":1,"maximum":1,"cost":1},
            "recovery": {"intervalTicks": fixed.map_or(candidate.recovery_turns, |activation| activation.recovery_turns).saturating_mul(10),"energyPerMille":1000},
            "target":target,"effect":effect
        })
    }).collect();
    (!profiles.is_empty()).then_some(profiles)
}

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
        assert_eq!(contract.len(), 76);
        for expected in contract {
            let matches = affixes
                .iter()
                .filter(|affix| affix["id"] == expected.affix_id)
                .collect::<Vec<_>>();
            assert_eq!(matches.len(), 1, "{}", expected.affix_id);
            assert_eq!(matches[0]["rfbEgo"]["sourceIndex"], expected.source_index);
        }
    }
}
