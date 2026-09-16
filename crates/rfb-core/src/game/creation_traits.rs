// SPDX-License-Identifier: MPL-2.0
// Birth descriptions of implemented rules. Never creates or advances a Game.
use super::progression::CharacterDefinitions;
use rfb_content::{
    ActorResistanceLevel, CastingStudyMode, ContentCatalog, RaceMutationSelectionDefinition,
};
use rfb_protocol::CreationFeatureDto;

fn feature(source: &str, name: &str, level: u16) -> CreationFeatureDto {
    CreationFeatureDto {
        source_name_key: source.into(),
        name_key: name.into(),
        name: None,
        description: None,
        detail_key: None,
        value: None,
        minimum_level: level,
        maximum_level: None,
        acquisition_key: "creation-acquire-automatic".into(),
        negative: false,
    }
}

// Enum wire names are also the existing localization suffixes (fire, hold-life, ...).
fn suffix(value: impl serde::Serialize) -> String {
    serde_json::to_value(value)
        .expect("serializable content enum")
        .as_str()
        .expect("content enum has a string wire name")
        .to_owned()
}

pub(super) fn creation_features(
    content: &ContentCatalog,
    (build, race, class, _): CharacterDefinitions<'_>,
) -> Vec<CreationFeatureDto> {
    let mut rows = Vec::new();
    for (name, level, resistances) in std::iter::once((&race.name_key, 1, &race.resistances))
        .chain(
            race.level_resistances
                .iter()
                .map(|entry| (&race.name_key, entry.minimum_level, &entry.resistances)),
        )
        .chain(
            class
                .level_resistances
                .iter()
                .map(|entry| (&class.name_key, entry.minimum_level, &entry.resistances)),
        )
    {
        for (damage, resistance) in resistances {
            let mut row = feature(name, &format!("damage-type-{}-name", suffix(damage)), level);
            row.detail_key = Some(format!("resistance-level-{}", suffix(resistance)));
            row.negative = *resistance == ActorResistanceLevel::Vulnerable;
            rows.push(row);
        }
    }
    for status in &race.status_immunities {
        let name = status
            .strip_prefix("rfb.status.")
            .expect("formal status immunity id");
        let mut row = feature(&race.name_key, &format!("status-{name}-name"), 1);
        row.detail_key = Some("creation-status-immune".into());
        rows.push(row);
    }
    for (level, name) in [
        (race.levitation.then_some(1), "levitation"),
        (
            if race.see_invisible {
                Some(1)
            } else {
                race.see_invisible_minimum_level
            },
            "see-invisible",
        ),
        (race.telepathy_minimum_level, "telepathy"),
        (race.hold_life_minimum_level, "hold-life"),
        (race.reflects_bolts_minimum_level, "reflects-bolts"),
        (
            race.tags
                .iter()
                .any(|tag| tag == "slow-digestion")
                .then_some(1),
            "slow-digestion",
        ),
    ] {
        if let Some(level) = level {
            rows.push(feature(
                &race.name_key,
                &format!("item-passive-{name}"),
                level,
            ));
        }
    }
    for attribute in &race.attribute_sustains {
        rows.push(feature(
            &race.name_key,
            &format!("item-passive-sustain-{}", suffix(attribute)),
            1,
        ));
    }
    if race.infravision > 0 {
        let mut row = feature(&race.name_key, "creation-infravision", 1);
        row.value = Some(race.infravision);
        rows.push(row);
    }
    for (value, key, negative) in [
        (
            race.regeneration_rate_modifier_percent,
            "creation-regeneration",
            race.regeneration_rate_modifier_percent < 0,
        ),
        (
            i32::from(race.healing_received_percent) - 100,
            "creation-healing",
            race.healing_received_percent < 100,
        ),
        (
            i32::from(race.melee_damage_percent) - 100,
            "creation-melee-damage",
            race.melee_damage_percent < 100,
        ),
    ] {
        if value != 0 {
            let mut row = feature(&race.name_key, key, 1);
            row.value = Some(value);
            row.negative = negative;
            rows.push(row);
        }
    }
    for (level, passive) in super::player_stats::class_passive_unlocks(&class.id) {
        rows.push(feature(
            &class.name_key,
            &format!("item-passive-{}", suffix(passive)),
            *level,
        ));
    }
    for (level, status) in super::player_stats::class_immunity_unlocks(&class.id) {
        let mut row = feature(
            &class.name_key,
            &format!(
                "status-{}-name",
                status.strip_prefix("rfb.status.").expect("formal immunity")
            ),
            *level,
        );
        row.detail_key = Some("creation-status-immune".into());
        rows.push(row);
    }
    if race.tags.iter().any(|tag| tag == "nonliving") {
        rows.push(feature(&race.name_key, "creation-nonliving", 1));
    }
    if race.id == "rfb-legacy.race.spectre" {
        rows.push(feature(&race.name_key, "trait-spectre-rule-passage", 1));
    }
    if race.id == "rfb-legacy.race.maia" {
        rows.push(feature(&race.name_key, "trait-maia-rule-birth", 1));
    }
    for activation in &race.abilities {
        let ability = content
            .ability(&activation.ability_id)
            .expect("validated race power");
        let mut row = feature(&race.name_key, &ability.name_key, activation.minimum_level);
        row.detail_key = Some(ability.description_key.clone());
        row.acquisition_key = "creation-acquire-power".into();
        if race.id == "rfb-legacy.race.android" {
            row.maximum_level = race
                .abilities
                .iter()
                .filter(|next| next.minimum_level > activation.minimum_level)
                .map(|next| next.minimum_level - 1)
                .min();
        }
        rows.push(row);
    }
    for activation in &class.abilities {
        // Stop actions depend on an active song/hex, not a newly granted character power.
        if activation.ability_id.starts_with("demo.ability.hex-stop")
            || !super::player_abilities::birth_class_power_matches_realm(
                &class.id,
                build.first_realm_id.as_deref(),
                &activation.ability_id,
            )
        {
            continue;
        }
        let ability = content
            .ability(&activation.ability_id)
            .expect("validated class power");
        let mut row = feature(&class.name_key, &ability.name_key, activation.minimum_level);
        row.detail_key = Some(ability.description_key.clone());
        row.acquisition_key = "creation-acquire-power".into();
        if activation.minimum_concentration > 0 {
            row.acquisition_key = "creation-acquire-concentration".into();
            row.value = Some(i32::from(activation.minimum_concentration));
        }
        rows.push(row);
    }
    for reward in &race.level_mutation_rewards {
        match &reward.selection {
            RaceMutationSelectionDefinition::Choice { .. } => {
                let mut row = feature(
                    &race.name_key,
                    "creation-mutation-choice",
                    reward.minimum_level,
                );
                row.acquisition_key = "creation-acquire-choice".into();
                rows.push(row);
            }
            RaceMutationSelectionDefinition::CastingAttribute {
                default_mutation_id,
                mutation_ids_by_attribute,
            } => {
                let id = class
                    .casting_profile
                    .as_ref()
                    .and_then(|profile| mutation_ids_by_attribute.get(&profile.casting_attribute))
                    .unwrap_or(default_mutation_id);
                let mutation = content.mutation(id).expect("validated race reward");
                let mut row = feature(
                    &race.name_key,
                    "creation-mutation-reward",
                    reward.minimum_level,
                );
                row.name = Some(mutation.name.clone());
                row.description = Some(mutation.description.clone());
                rows.push(row);
            }
        }
    }
    if let Some(profile) = &class.casting_profile {
        if class.uses_spell_scrolls {
            let mut row = feature(
                &class.name_key,
                "creation-book-learning",
                profile.first_spell_level,
            );
            row.acquisition_key = match profile.study_mode {
                CastingStudyMode::Chosen => "creation-acquire-study",
                CastingStudyMode::DivineRandom => "creation-acquire-prayer",
            }
            .into();
            row.detail_key = Some("creation-book-learning-help".into());
            rows.push(row);
        }
        if let Some(encumbrance) = &profile.encumbrance {
            let mut row = feature(&class.name_key, "creation-casting-encumbrance", 1);
            row.negative = true;
            row.value = Some(
                i32::try_from(encumbrance.maximum_weight_tenths_pound)
                    .expect("casting weight fits i32"),
            );
            rows.push(row);
        }
    }
    // Existing audited descriptions cover specialized rules not expressible as a flag.
    let rules: &[&str] = match race.id.as_str() {
        "rfb-legacy.race.android" => &[
            "trait-android-rule-experience",
            "trait-android-rule-diet",
            "trait-android-rule-birth",
        ],
        "rfb-legacy.race.centaur" => &["trait-centaur-rule-armor", "trait-centaur-rule-birth"],
        "rfb-legacy.race.ent" => &["trait-ent-rule-fire", "trait-ent-rule-diet"],
        "rfb-legacy.race.balrog" => &["trait-balrog-rule-diet"],
        "rfb-legacy.race.maia" => &["trait-maia-rule-choice", "trait-maia-rule-realms"],
        "rfb-legacy.race.tomte" => &["trait-tomte-headgear-rule"],
        "rfb-legacy.race.spectre" => &["trait-spectre-rule-density", "trait-spectre-rule-diet"],
        "rfb-legacy.race.vampire" => &["trait-vampire-rule-light", "trait-vampire-rule-diet"],
        "rfb-legacy.race.tonberry" => &[
            "trait-tonberry-rule-speed",
            "trait-tonberry-rule-attacks",
            "trait-tonberry-rule-confusion",
        ],
        _ => &[],
    };
    for key in rules {
        let mut row = feature(&race.name_key, key, 1);
        row.negative = true;
        rows.push(row);
    }
    if race.food_nutrition_divisor > 1 && rules.is_empty() {
        let mut row = feature(&race.name_key, "creation-food-divisor", 1);
        row.value = Some(i32::from(race.food_nutrition_divisor));
        row.negative = true;
        rows.push(row);
    }
    for slot in &class.icky_equipment_slots {
        let mut row = feature(&class.name_key, &format!("equipment-slot-{slot}"), 1);
        row.detail_key = Some("creation-icky-equipment".into());
        row.negative = true;
        rows.push(row);
    }
    if matches!(
        class.id.as_str(),
        "demo.class.berserker" | "demo.class.rage-mage"
    ) {
        let mut row = feature(&class.name_key, &class.description_key, 1);
        row.negative = true;
        rows.push(row);
    }
    if class.id == "demo.class.priest"
        && matches!(build.first_realm_id.as_deref(), Some("life" | "crusade"))
    {
        let mut row = feature(&class.name_key, "trait-priest-blade-rule", 1);
        row.negative = true;
        rows.push(row);
    }
    rows.sort_by_key(|row| row.minimum_level);
    rows
}
