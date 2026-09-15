// SPDX-License-Identifier: MPL-2.0
//! RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c:
//! object1.c::obj_flags_known, obj_learn_flag, obj_learn_activation and identification.

use super::*;
use rfb_content::{AffixDefinition, AffixPropertyBundleDefinition, ItemDefinition};
#[cfg(test)]
mod tests;

pub(super) fn base_properties(kind: &ItemDefinition) -> AffixPropertyBundleDefinition {
    AffixPropertyBundleDefinition {
        rfb_flags: kind
            .rfb_value
            .as_ref()
            .map(|v| v.flags.clone())
            .unwrap_or_default(),
        modifiers: kind.modifiers.clone(),
        equipment_bonuses: kind.equipment_bonuses.clone(),
        resistances: kind.resistances.clone(),
        status_immunities: kind.status_immunities.clone(),
        slays: kind.slays.clone(),
        brands: kind.brands.clone(),
        passives: kind.passives.clone(),
        ..Default::default()
    }
}

pub(super) fn affix_properties(affix: &AffixDefinition) -> AffixPropertyBundleDefinition {
    AffixPropertyBundleDefinition {
        rfb_flags: affix
            .rfb_ego
            .as_ref()
            .map(|e| e.flags.clone())
            .unwrap_or_default(),
        modifiers: affix.modifiers.clone(),
        equipment_bonuses: affix.equipment_bonuses.clone(),
        resistances: affix.resistances.clone(),
        status_immunities: affix.status_immunities.clone(),
        slays: affix.slays.clone(),
        brands: affix.brands.clone(),
        passives: affix.passives.clone(),
        ..Default::default()
    }
}

// The same field-to-flag mapping serves learning and visible numeric properties.
// Non-flag combat bonuses remain visible after ordinary identification.
fn numeric_flags(
    properties: &mut AffixPropertyBundleDefinition,
    mut visit: impl FnMut(&mut i32, &str),
) {
    let pval_flags = properties.rfb_pval.as_ref().map(|pval| &pval.flags);
    macro_rules! signed {
        ($source:expr; $( $field:ident => $positive:literal, $negative:literal );* $(;)?) => {
            $(let value = &mut $source.$field;
              let declared = |flag: &str| pval_flags.is_some_and(|flags|
                  flags.iter().any(|pval| pval.source_flag() == flag));
              if $positive != $negative && declared($positive) && declared($negative) {
                  // A net modifier containing opposing powers needs both flags.
                  visit(value, $positive);
                  visit(value, $negative);
              } else {
                  let flag = if declared($positive) { $positive }
                      else if declared($negative) || *value < 0 { $negative } else { $positive };
                  visit(value, flag);
              })*
        };
    }
    signed!(properties.modifiers;
        strength => "STR", "DEC_STR"; intelligence => "INT", "DEC_INT";
        wisdom => "WIS", "DEC_WIS"; dexterity => "DEX", "DEC_DEX";
        constitution => "CON", "DEC_CON"; charisma => "CHR", "DEC_CHR";
        speed => "SPEED", "DEC_SPEED"; max_hp => "LIFE", "DEC_LIFE";
        spell_power_bonus => "SPELL_POWER", "DEC_SPELL_POWER";
        device_power_bonus => "DEVICE_POWER", "DEVICE_POWER";
    );
    signed!(properties.equipment_bonuses;
        weapon_dice_bonus => "WEAPONMASTERY", "WEAPONMASTERY";
        life_percent => "LIFE", "DEC_LIFE";
        launcher_multiplier_delta_percent => "XTRA_MIGHT", "XTRA_MIGHT";
        base_shot_delta_percent => "XTRA_SHOTS", "XTRA_SHOTS";
        melee_attacks_delta_percent => "BLOWS", "BLOWS";
        melee_attacks => "BLOWS", "BLOWS";
        spell_capacity_bonus => "SPELL_CAP", "DEC_SPELL_CAP";
        magic_resistance_percent => "MAGIC_RESISTANCE", "MAGIC_RESISTANCE";
        device_skill => "MAGIC_MASTERY", "DEC_MAGIC_MASTERY";
        stealth_skill => "STEALTH", "DEC_STEALTH";
        search_skill => "SEARCH", "SEARCH"; perception_skill => "SEARCH", "SEARCH";
        digging_skill => "TUNNEL", "TUNNEL"; infravision => "INFRA", "INFRA";
        light_radius => "LITE", "DARKNESS";
    );
}

pub(super) fn property_flags(properties: &AffixPropertyBundleDefinition) -> BTreeSet<String> {
    let mut object = super::super::item_value::ValueObject::default();
    object.properties(properties);
    let mut numeric = properties.clone();
    numeric_flags(&mut numeric, |value, flag| {
        if *value != 0 {
            object.flags.insert(flag.to_owned());
        }
    });
    object
        .flags
        .retain(|flag| rfb_content::valid_rfb_runtime_flag(flag));
    object.flags
}

fn rolled_flags(roll: &crate::state::RolledAffixState) -> BTreeSet<String> {
    let mut flags = property_flags(&roll.properties);
    flags.extend(
        roll.weapon_traits
            .iter()
            .map(|trait_| weapon_trait_flag(*trait_).to_owned()),
    );
    flags.extend(
        roll.elemental_destruction_immunities
            .iter()
            .map(|element| destruction_flag(*element).to_owned()),
    );
    flags
}

fn destruction_flag(element: rfb_content::ItemDestructionElement) -> &'static str {
    use rfb_content::ItemDestructionElement::*;
    match element {
        Acid => "IGNORE_ACID",
        Electricity => "IGNORE_ELEC",
        Fire => "IGNORE_FIRE",
        Cold => "IGNORE_COLD",
    }
}

pub(super) fn filter_properties(
    mut properties: AffixPropertyBundleDefinition,
    flags: &BTreeSet<String>,
    identified: bool,
) -> AffixPropertyBundleDefinition {
    numeric_flags(&mut properties, |value, flag| {
        if !flags.contains(flag) {
            *value = 0;
        }
    });
    if !identified {
        properties.modifiers.attack = 0;
        properties.modifiers.defense = 0;
        properties.equipment_bonuses.melee_skill = 0;
        properties.equipment_bonuses.melee_damage = 0;
        properties.equipment_bonuses.ranged_skill = 0;
        properties.equipment_bonuses.throwing_skill = 0;
    }
    // These project-specific fields have no source object flag; full ID bypasses filtering.
    properties.equipment_bonuses.saving_throw_skill = 0;
    properties.equipment_bonuses.saving_throw_skill_override = None;
    properties.equipment_bonuses.disarming_skill = 0;
    properties.rfb_flags.retain(|flag| flags.contains(flag));
    if let Some(pval) = &mut properties.rfb_pval {
        pval.flags.retain(|flag| flags.contains(flag.source_flag()));
    }
    properties.rfb_heavy_curse = false;
    if !identified {
        properties.ammunition_capacity = None;
        properties.bag_capacity = None;
    }
    // Reuse the canonical resistance/slay/brand/status mapping from item valuation.
    // Each probe has one property, so learning a flag cannot expose its neighbours.
    let known = |probe: AffixPropertyBundleDefinition| {
        let mut object = super::super::item_value::ValueObject::default();
        object.properties(&probe);
        !object.flags.is_empty() && object.flags.is_subset(flags)
    };
    properties.resistances.retain(|damage, level| {
        known(AffixPropertyBundleDefinition {
            resistances: BTreeMap::from([(*damage, *level)]),
            ..Default::default()
        })
    });
    properties.slays.retain(|target, level| {
        known(AffixPropertyBundleDefinition {
            slays: BTreeMap::from([(*target, *level)]),
            ..Default::default()
        })
    });
    properties.brands.retain(|brand| {
        known(AffixPropertyBundleDefinition {
            brands: BTreeSet::from([*brand]),
            ..Default::default()
        })
    });
    properties.status_immunities.retain(|status| {
        known(AffixPropertyBundleDefinition {
            status_immunities: vec![status.clone()],
            ..Default::default()
        })
    });
    properties.passives.retain(|passive| {
        super::super::item_value::instance::passive_flag(*passive)
            .is_some_and(|flag| flags.contains(flag))
    });
    properties
}

impl Game {
    // Identity is separate from flag completeness. Lore lookup must not call
    // item_identification, which derives completeness from this very knowledge.
    pub(in crate::game) fn item_identity_is_known(&self, item: &ItemInstance) -> bool {
        self.item_property_knowledge
            .get(&item.id)
            .is_some_and(|k| k.appraised || k.identified)
    }

    pub(in crate::game) fn unknown_item_flags(&self, item: &ItemInstance) -> BTreeSet<String> {
        let known = self.known_item_flags(item);
        self.actual_item_flags(item)
            .difference(&known)
            // object1.c::_obj_flags_purify: presentation metadata is not a power.
            .filter(|flag| {
                !matches!(
                    flag.as_str(),
                    "HIDE_TYPE" | "SHOW_MODS" | "FULL_NAME" | "FIXED_FLAVOR"
                )
            })
            .cloned()
            .collect()
    }

    pub(super) fn item_unflagged_properties_known(&self, item: &ItemInstance) -> bool {
        let knowledge = self.item_property_knowledge.get(&item.id);
        if knowledge.is_some_and(|k| k.identified) {
            return true;
        }
        // Some project properties (e.g. disarming and passwall) have no OF flag.
        // An empty source-flag difference must not reveal those hidden properties.
        let needs_full_id = |properties: AffixPropertyBundleDefinition| {
            let visible = filter_properties(properties.clone(), &property_flags(&properties), true);
            visible.modifiers != properties.modifiers
                || visible.equipment_bonuses != properties.equipment_bonuses
                || visible.resistances != properties.resistances
                || visible.status_immunities != properties.status_immunities
                || visible.passives != properties.passives
        };
        let kind = self.content.item(&item.kind_id).expect("item kind exists");
        !(needs_full_id(item.intrinsic_properties.clone())
            || kind.artifact_generation.is_some() && needs_full_id(base_properties(kind)))
            && item.affix_ids.iter().all(|id| {
                knowledge.is_some_and(|k| k.known_affix_ids.contains(id))
                    || !needs_full_id(affix_properties(
                        self.content.affix(id).expect("affix exists"),
                    ))
            })
            && item.rolled_affixes.iter().all(|roll| {
                knowledge.is_some_and(|k| k.known_affix_ids.contains(&roll.affix_id))
                    || !needs_full_id(roll.properties.clone())
            })
    }

    fn fixed_artifact_flags(&self, item: &ItemInstance) -> Option<BTreeSet<String>> {
        let kind = self.content.item(&item.kind_id).expect("item kind exists");
        kind.artifact_generation.as_ref()?;
        (item.artifact_name.is_none()).then(|| property_flags(&base_properties(kind)))
    }

    pub(in crate::game) fn actual_item_flags(&self, item: &ItemInstance) -> BTreeSet<String> {
        let kind = self.content.item(&item.kind_id).expect("item kind exists");
        let mut flags = property_flags(&base_properties(kind));
        if let Some(artifact) = &kind.artifact_generation {
            flags.extend(property_flags(&base_properties(
                self.content
                    .item(&artifact.base_item_kind_id)
                    .expect("artifact base exists"),
            )));
        }
        flags.extend(property_flags(&item.intrinsic_properties));
        for id in &item.affix_ids {
            flags.extend(property_flags(&affix_properties(
                self.content.affix(id).expect("affix exists"),
            )));
        }
        for roll in &item.rolled_affixes {
            flags.extend(rolled_flags(roll));
        }
        for trait_ in &item.intrinsic_weapon_traits {
            flags.insert(weapon_trait_flag(*trait_).to_owned());
        }
        if kind.vorpal {
            flags.insert("VORPAL".to_owned());
        }
        if item.is_artifact(&self.content) {
            flags.extend(
                ["IGNORE_ACID", "IGNORE_ELEC", "IGNORE_FIRE", "IGNORE_COLD"].map(str::to_owned),
            );
        }
        for element in &item.permanent_destruction_immunities {
            flags.insert(destruction_flag(*element).to_owned());
        }
        if item.activation.is_some() {
            flags.insert("ACTIVATE".to_owned());
        }
        flags
    }

    pub(in crate::game) fn known_item_flags(&self, item: &ItemInstance) -> BTreeSet<String> {
        let actual = self.actual_item_flags(item);
        let knowledge = self.item_property_knowledge.get(&item.id);
        if knowledge.is_some_and(|k| k.identified) {
            return actual;
        }
        let mut known = knowledge.map(|k| k.known_flags.clone()).unwrap_or_default();
        if knowledge.is_some_and(|k| k.known_blessed) {
            known.insert("BLESSED".into());
        }
        for id in &item.affix_ids {
            if knowledge.is_some_and(|k| k.known_affix_ids.contains(id)) {
                known.extend(property_flags(&affix_properties(
                    self.content.affix(id).expect("affix exists"),
                )));
                for roll in item
                    .rolled_affixes
                    .iter()
                    .filter(|roll| &roll.affix_id == id)
                {
                    known.extend(rolled_flags(roll));
                }
            }
        }
        if self.item_identity_is_known(item) {
            let kind = self.content.item(&item.kind_id).expect("item kind exists");
            let base = kind.artifact_generation.as_ref().map_or(kind, |artifact| {
                self.content
                    .item(&artifact.base_item_kind_id)
                    .expect("artifact base exists")
            });
            if self.item_knowledge_dto(&base.id) == ItemKnowledgeDto::Aware {
                known.extend(property_flags(&base_properties(base)));
                if item.activation.as_ref().is_some_and(|activation| {
                    base.device_generation.as_ref().is_some_and(|generation| {
                        generation
                            .activations
                            .iter()
                            .any(|profile| profile.id == activation.profile_id)
                    })
                }) {
                    known.insert("ACTIVATE".into());
                }
            }
            if self.fixed_artifact_flags(item).is_some() {
                if let Some(flags) = self.item_lore.artifacts.get(&item.kind_id) {
                    known.extend(flags.clone());
                }
            } else if item.artifact_name.is_none() {
                for id in &item.affix_ids {
                    if let Some(flags) = self.item_lore.egos.get(id) {
                        known.extend(flags.clone());
                    }
                }
            }
            if self.activation_is_instance_specific(item) {
                // An unknown override must not inherit knowledge of the default activation.
                if !knowledge.is_some_and(|k| k.known_flags.contains("ACTIVATE")) {
                    known.remove("ACTIVATE");
                }
            }
        }
        known.retain(|flag| actual.contains(flag));
        known
    }

    pub(in crate::game) fn learn_item_flag(&mut self, item_id: &str, flag: &str) -> bool {
        let item = self
            .items
            .iter()
            .find(|item| item.id == item_id)
            .expect("learned item exists")
            .clone();
        self.learn_item_flag_on(&item, flag)
    }

    /// Projectiles temporarily leave `items` while flying; their lore still belongs
    /// to the same instance and is settled with it on recovery or destruction.
    pub(in crate::game) fn learn_item_flag_on(&mut self, item: &ItemInstance, flag: &str) -> bool {
        let item_id = item.id.as_str();
        if !self.actual_item_flags(item).contains(flag) {
            return false;
        }
        if matches!(
            item.location,
            ItemLocation::Inventory | ItemLocation::Equipped { .. }
        ) {
            self.item_property_knowledge
                .entry(item_id.to_owned())
                .or_default()
                .discovered = true;
        }
        if self.item_identity_is_known(item) && flag != "ACTIVATE" {
            if let Some(fixed) = self.fixed_artifact_flags(item) {
                if fixed.contains(flag) {
                    let shared = self
                        .item_lore
                        .artifacts
                        .entry(item.kind_id.clone())
                        .or_default()
                        .insert(flag.to_owned());
                    self.item_property_knowledge
                        .entry(item_id.to_owned())
                        .or_default()
                        .known_flags
                        .remove(flag);
                    return shared;
                }
            } else if item.artifact_name.is_none() {
                for id in &item.affix_ids {
                    let base = property_flags(&affix_properties(
                        self.content.affix(id).expect("affix exists"),
                    ));
                    // Materialized ego extras are separate from intrinsic curse biffs.
                    let extra = item
                        .rolled_affixes
                        .iter()
                        .filter(|roll| &roll.affix_id == id)
                        .any(|roll| rolled_flags(roll).contains(flag));
                    if base.contains(flag) || extra {
                        let shared = self
                            .item_lore
                            .egos
                            .entry(id.clone())
                            .or_default()
                            .insert(flag.to_owned());
                        self.item_property_knowledge
                            .entry(item_id.to_owned())
                            .or_default()
                            .known_flags
                            .remove(flag);
                        return shared;
                    }
                }
            }
        }
        self.item_property_knowledge
            .entry(item_id.to_owned())
            .or_default()
            .known_flags
            .insert(flag.to_owned())
    }

    fn activation_is_instance_specific(&self, item: &ItemInstance) -> bool {
        let Some(activation) = &item.activation else {
            return false;
        };
        if item.artifact_name.is_some() {
            return true;
        }
        let kind = self.content.item(&item.kind_id).expect("item kind exists");
        let fixed = |generation: &rfb_content::ItemDeviceGenerationDefinition| {
            generation
                .activations
                .iter()
                .find(|profile| profile.rfb_biases.is_empty())
                .is_some_and(|profile| profile.id == activation.profile_id)
        };
        !kind.device_generation.as_ref().is_some_and(fixed)
            && !item.affix_ids.iter().any(|id| {
                self.content
                    .affix(id)
                    .and_then(|affix| affix.device_generation.as_ref())
                    .is_some_and(fixed)
            })
    }

    pub(in crate::game) fn learn_item_activation(&mut self, item_id: &str) {
        let item = self
            .items
            .iter()
            .find(|item| item.id == item_id)
            .expect("activated item exists");
        let Some(activation) = &item.activation else {
            return;
        };
        if self.item_is_device(item) {
            return;
        }
        if matches!(
            item.location,
            ItemLocation::Inventory | ItemLocation::Equipped { .. }
        ) {
            self.item_property_knowledge
                .entry(item_id.to_owned())
                .or_default()
                .discovered = true;
        }
        if !self.item_identity_is_known(item) {
            self.item_property_knowledge
                .entry(item_id.to_owned())
                .or_default()
                .known_flags
                .insert("ACTIVATE".into());
        } else if self.activation_is_instance_specific(item) {
            self.item_lore
                .activation_profiles
                .insert(activation.profile_id.clone());
            self.item_property_knowledge
                .entry(item_id.to_owned())
                .or_default()
                .known_flags
                .insert("ACTIVATE".into());
        } else if self.fixed_artifact_flags(item).is_some() {
            self.item_lore
                .artifacts
                .entry(item.kind_id.clone())
                .or_default()
                .insert("ACTIVATE".into());
            self.item_property_knowledge
                .entry(item_id.to_owned())
                .or_default()
                .known_flags
                .remove("ACTIVATE");
        } else if let Some(id) = item.affix_ids.iter().find(|id| {
            self.content
                .affix(id)
                .and_then(|a| a.device_generation.as_ref())
                .is_some_and(|g| {
                    g.activations
                        .iter()
                        .any(|p| p.id == activation.profile_id && p.rfb_biases.is_empty())
                })
        }) {
            self.item_lore
                .egos
                .entry(id.clone())
                .or_default()
                .insert("ACTIVATE".into());
            self.item_property_knowledge
                .entry(item_id.to_owned())
                .or_default()
                .known_flags
                .remove("ACTIVATE");
        } else {
            self.item_property_knowledge
                .entry(item_id.to_owned())
                .or_default()
                .known_flags
                .insert("ACTIVATE".into());
        }
    }

    pub(in crate::game) fn identify_item_lore(&mut self, item_id: &str, full: bool) {
        if full {
            self.learn_all_item_curses(item_id);
        }
        let item = self
            .items
            .iter()
            .find(|item| item.id == item_id)
            .expect("identified item exists");
        let mut flags = if full {
            self.actual_item_flags(item)
        } else {
            self.item_property_knowledge
                .get(item_id)
                .map(|k| k.known_flags.clone())
                .unwrap_or_default()
        };
        if item.is_artifact(&self.content) {
            flags.extend(
                ["IGNORE_ACID", "IGNORE_ELEC", "IGNORE_FIRE", "IGNORE_COLD"].map(str::to_owned),
            );
        }
        let learn_activation = full
            || flags.contains("ACTIVATE")
            || (self.activation_is_instance_specific(item)
                && item
                    .activation
                    .as_ref()
                    .is_some_and(|activation| self.activation_effect_is_known(activation)));
        if full
            && item.curse.is_none()
            && item.artifact_name.is_none()
            && self.fixed_artifact_flags(item).is_none()
            && !self.item_is_device(item)
        {
            // _obj_identify_fully_aux also remembers observed random ego powers.
            let mut extras = property_flags(&item.intrinsic_properties);
            extras.extend(
                item.intrinsic_weapon_traits
                    .iter()
                    .map(|trait_| weapon_trait_flag(*trait_).to_owned()),
            );
            for roll in &item.rolled_affixes {
                extras.extend(rolled_flags(roll));
            }
            extras.remove("ACTIVATE");
            for id in &item.affix_ids {
                self.item_lore
                    .egos
                    .entry(id.clone())
                    .or_default()
                    .extend(extras.clone());
            }
        }
        for flag in flags {
            if flag != "ACTIVATE" {
                self.learn_item_flag(item_id, &flag);
            }
        }
        if learn_activation {
            self.learn_item_activation(item_id);
        }
    }

    fn activation_effect_is_known(&self, activation: &rfb_protocol::ItemActivationDto) -> bool {
        // Importers give one source effect the same stable name key across ego/artifact
        // profiles, while IDs and power values belong to the individual profiles.
        let known = |generation: &rfb_content::ItemDeviceGenerationDefinition| {
            generation.activations.iter().any(|profile| {
                profile.name_key == activation.name_key
                    && self.item_lore.activation_profiles.contains(&profile.id)
            })
        };
        self.content
            .item_definitions()
            .filter_map(|kind| kind.device_generation.as_ref())
            .any(known)
            || self
                .content
                .affix_definitions()
                .filter_map(|affix| affix.device_generation.as_ref())
                .any(known)
            || self
                .content
                .random_artifact_generation()
                .is_some_and(|source| known(&source.device_generation))
    }

    pub(in crate::game) fn validate_item_lore(&self) -> Result<(), CoreError> {
        let valid_flags = |flags: &BTreeSet<String>| {
            flags
                .iter()
                .all(|flag| rfb_content::valid_rfb_runtime_flag(flag))
        };
        if self
            .item_lore
            .egos
            .iter()
            .any(|(id, flags)| self.content.affix(id).is_none() || !valid_flags(flags))
            || self.item_lore.artifacts.iter().any(|(id, flags)| {
                self.content.item(id).is_none_or(|kind| {
                    let mut actual = property_flags(&base_properties(kind));
                    if kind.device_generation.is_some() {
                        actual.insert("ACTIVATE".into());
                    }
                    kind.artifact_generation.is_none() || !flags.is_subset(&actual)
                }) || !valid_flags(flags)
            })
            || self
                .item_property_knowledge
                .values()
                .any(|k| !valid_flags(&k.known_flags))
        {
            return Err(CoreError::InvalidSave("item lore state is invalid"));
        }
        for id in &self.item_lore.activation_profiles {
            let has_profile = |generation: &rfb_content::ItemDeviceGenerationDefinition| {
                generation.activations.iter().any(|p| &p.id == id)
            };
            if !self
                .content
                .item_definitions()
                .filter_map(|kind| kind.device_generation.as_ref())
                .any(has_profile)
                && !self
                    .content
                    .affix_definitions()
                    .filter_map(|affix| affix.device_generation.as_ref())
                    .any(has_profile)
                && !self
                    .content
                    .random_artifact_generation()
                    .is_some_and(|source| has_profile(&source.device_generation))
            {
                return Err(CoreError::InvalidSave(
                    "item activation lore reference is invalid",
                ));
            }
        }
        Ok(())
    }
}

pub(super) fn weapon_trait_flag(trait_: rfb_protocol::WeaponTraitDto) -> &'static str {
    use rfb_protocol::WeaponTraitDto::*;
    match trait_ {
        ManaBrand => "BRAND_MANA",
        Vorpal => "VORPAL",
        Vorpal2 => "VORPAL2",
        Order => "ORDER",
        Wild => "BRAND_WILD",
        Impact => "IMPACT",
        Stun => "STUN",
        Blessed => "BLESSED",
    }
}
