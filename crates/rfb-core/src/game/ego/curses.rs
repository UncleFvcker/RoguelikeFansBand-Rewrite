// SPDX-License-Identifier: MPL-2.0
//! RFB master artifact.c one_biff/curse_object and mspells1.c get_curse.

use super::*;
use crate::game::{item_value, loot::GeneratedItemDraft};
use rfb_content::RfbPvalFlagDefinition;

#[cfg(test)]
mod tests;

pub(in crate::game) const CURSE_EFFECTS: [Option<ItemCurseEffectDto>; 28] = {
    use ItemCurseEffectDto::*;
    [
        Some(TyCurse),
        Some(Aggravate),
        Some(DrainExperience),
        Some(SlowRegeneration),
        Some(AddLightCurse),
        Some(AddHeavyCurse),
        Some(CallAnimal),
        Some(CallDemon),
        Some(CallDragon),
        Some(Cowardice),
        Some(Teleport),
        Some(LowMelee),
        Some(LowArmor),
        Some(LowMagic),
        Some(FastDigest),
        Some(DrainHp),
        Some(DrainMana),
        None,
        None,
        Some(ByCurse),
        Some(Danger),
        Some(Catlike),
        Some(DrainPack),
        Some(CrappyMutation),
        Some(Allergy),
        Some(OpenWounds),
        Some(Normality),
        Some(LowDevice),
    ]
};

pub(in crate::game) fn get_curse(rng: &mut RfbRng, power: u8, tval: u16) -> ItemCurseEffectDto {
    loop {
        let index = rng.bounded(28) as usize;
        let Some(effect) = CURSE_EFFECTS[index] else {
            continue;
        };
        let heavy = matches!(index, 0 | 1 | 2 | 5 | 7 | 8 | 10 | 19 | 20 | 23);
        if (power == 2 && !heavy)
            || (power == 1 && index <= 1)
            || (power == 0 && heavy)
            || (index == 11 && !(19..=23).contains(&tval))
            || (index == 12 && !(30..=38).contains(&tval))
        {
            continue;
        }
        return effect;
    }
}

fn add_bad_flag(
    flags: &BTreeSet<String>,
    added: &mut BTreeSet<String>,
    bad: &str,
    good: &str,
) -> bool {
    if flags.contains(good) {
        return false;
    }
    // An already-present bad flag still succeeds in the source.
    added.insert(bad.to_owned());
    true
}

fn one_vulnerability(
    rng: &mut RfbRng,
    flags: &BTreeSet<String>,
    added: &mut BTreeSet<String>,
) -> bool {
    let elements: &[&str] = if one_in(rng, 3) {
        &["ACID", "ELEC", "COLD", "FIRE"]
    } else {
        &[
            "POIS", "LITE", "DARK", "SHARDS", "BLIND", "CONF", "SOUND", "NETHER", "NEXUS", "CHAOS",
            "DISEN", "FEAR",
        ]
    };
    for _ in 0..100 {
        let element = elements[rng.bounded(elements.len() as u64) as usize];
        if add_bad_flag(
            flags,
            added,
            &format!("VULN_{element}"),
            &format!("RES_{element}"),
        ) {
            return true;
        }
    }
    false
}

fn one_biff(rng: &mut RfbRng, flags: &BTreeSet<String>, added: &mut BTreeSet<String>) -> bool {
    for _ in 0..100 {
        let flag = match rng.bounded(100) {
            0..=9 => "STEALTH",
            10..=12 => "SPEED",
            13..=16 => "LIFE",
            17..=21 => "MAGIC_MASTERY",
            22..=29 => "SPELL_CAP",
            30..=34 => "SPELL_POWER",
            35..=66 => {
                for _ in 0..100 {
                    let stat = ["STR", "INT", "WIS", "DEX", "CON", "CHR"][rng.bounded(6) as usize];
                    if add_bad_flag(flags, added, &format!("DEC_{stat}"), stat) {
                        return true;
                    }
                }
                continue;
            }
            _ => {
                if one_vulnerability(rng, flags, added) {
                    return true;
                }
                continue;
            }
        };
        if add_bad_flag(flags, added, &format!("DEC_{flag}"), flag) {
            return true;
        }
    }
    false
}

#[derive(Debug)]
pub(in crate::game) struct CurseRoll {
    pub(in crate::game) heavy: bool,
    pub(in crate::game) flags: BTreeSet<String>,
    pub(in crate::game) effects: BTreeSet<ItemCurseEffectDto>,
    pub(in crate::game) severity: ItemCurseSeverityDto,
}

pub(in crate::game) fn roll_curse(
    rng: &mut RfbRng,
    value: i32,
    tval: u16,
    flags: &BTreeSet<String>,
) -> CurseRoll {
    let mut count = i32::from(randint1(rng, 2));
    let value = value / 10_000;
    let mut result = CurseRoll {
        heavy: false,
        flags: BTreeSet::new(),
        effects: BTreeSet::new(),
        severity: ItemCurseSeverityDto::Normal,
    };
    one_biff(rng, flags, &mut result.flags);
    count -= 1;
    let nested = if value <= 1 {
        1
    } else {
        rng.bounded(value as u64) as i32 + 1
    };
    count += if nested <= 1 {
        1
    } else {
        rng.bounded(nested as u64) as i32 + 1
    };
    for _ in 0..count {
        let bound = 70_i32.wrapping_add(value.wrapping_mul(value)) as u32;
        let roll = if bound <= 1 {
            0
        } else {
            rng.bounded(u64::from(bound))
        };
        match roll {
            0..=24 => loop {
                result.effects.insert(get_curse(rng, 0, tval));
                if !one_in(rng, 2) {
                    break;
                }
            },
            25..=34 => {
                result.effects.insert(get_curse(rng, 1, tval));
            }
            35..=39 => {
                result.heavy = true;
                result.severity = result.severity.max(ItemCurseSeverityDto::Heavy);
            }
            40..=44 => {
                result.effects.insert(get_curse(rng, 2, tval));
            }
            45..=99 | 108..=249 => {
                one_biff(rng, flags, &mut result.flags);
            }
            100..=104 => {
                result.flags.insert("AGGRAVATE".to_owned());
            }
            105..=106 => {
                result.flags.insert("TY_CURSE".to_owned());
            }
            107 => result.severity = ItemCurseSeverityDto::Permanent,
            _ => {
                if one_in(rng, 2) {
                    result.severity = ItemCurseSeverityDto::Permanent;
                    for flag in ["TY_CURSE", "NO_TELE", "AGGRAVATE"] {
                        if one_in(rng, 2) {
                            result.flags.insert(flag.to_owned());
                        }
                    }
                } else {
                    one_biff(rng, flags, &mut result.flags);
                }
            }
        }
    }
    result
}

pub(in crate::game) fn curse_object(
    content: &ContentCatalog,
    rng: &mut RfbRng,
    item: &mut ItemInstance,
) {
    let object = item_value::instance::value_object(content, item)
        .expect("generated RFB equipment retains its complete original value inputs");
    let value = item_value::object_value(object.clone()).expect("equipment COST_REAL is supported");
    let roll = roll_curse(rng, value, object.tval, &object.flags);
    item.intrinsic_properties.rfb_heavy_curse = roll.heavy
        || item.curse == Some(ItemCurseSeverityDto::Heavy)
        || (item.curse == Some(ItemCurseSeverityDto::Permanent)
            && item.intrinsic_properties.rfb_heavy_curse);
    item.curse = Some(
        item.curse
            .map_or(roll.severity, |old| old.max(roll.severity)),
    );
    if !roll.effects.is_empty() {
        if item.affix_ids.is_empty() {
            item.intrinsic_curse_effects.extend(roll.effects);
        } else {
            if item.rolled_affixes.is_empty() {
                item.rolled_affixes.push(RolledAffixState {
                    affix_id: item.affix_ids[0].clone(),
                    ..Default::default()
                });
            }
            item.rolled_affixes[0].curse_effects.extend(roll.effects);
        }
    }
    let mut pval = object.pval;
    let dynamic_pval = item
        .intrinsic_properties
        .rfb_pval
        .iter()
        .chain(
            item.rolled_affixes
                .iter()
                .filter_map(|roll| roll.properties.rfb_pval.as_ref()),
        )
        .any(|pval| !pval.flags.is_empty());
    if pval == 0
        && (dynamic_pval
            || RfbPvalFlagDefinition::ALL
                .iter()
                .any(|flag| roll.flags.contains(flag.source_flag())))
    {
        pval = i32::from(randint1(rng, 3));
        for raw in item.intrinsic_properties.rfb_pval.iter_mut().chain(
            item.rolled_affixes
                .iter_mut()
                .filter_map(|roll| roll.properties.rfb_pval.as_mut()),
        ) {
            raw.value = pval as i16;
        }
    }
    // Only the shared pval delta and newly-added flags change the gameplay projection.
    for flag in RfbPvalFlagDefinition::ALL {
        let old = object.flags.contains(flag.source_flag());
        if old && pval != object.pval {
            armor::apply_pval(&mut item.intrinsic_properties, flag, pval - object.pval);
        } else if !old && roll.flags.contains(flag.source_flag()) {
            armor::apply_pval(&mut item.intrinsic_properties, flag, pval);
        }
        if roll.flags.contains(flag.source_flag()) {
            super::remember_rfb_pval(&mut item.intrinsic_properties, [flag], pval);
        }
    }
    for flag in roll.flags {
        if RfbPvalFlagDefinition::ALL
            .iter()
            .any(|pval| pval.source_flag() == flag)
        {
            continue;
        }
        item.intrinsic_properties.rfb_flags.insert(flag.clone());
        if flag == "NO_TELE" {
            item.intrinsic_properties
                .passives
                .insert(EquipmentPassive::AntiTeleport);
        }
        if let Some(element) = flag.strip_prefix("VULN_") {
            let damage = match element {
                "ACID" => ActorDamageType::Acid,
                "ELEC" => ActorDamageType::Electricity,
                "FIRE" => ActorDamageType::Fire,
                "COLD" => ActorDamageType::Cold,
                "POIS" => ActorDamageType::Poison,
                "LITE" => ActorDamageType::Light,
                "DARK" => ActorDamageType::Dark,
                "SHARDS" => ActorDamageType::Shards,
                "BLIND" => ActorDamageType::Blindness,
                "CONF" => ActorDamageType::Confusion,
                "SOUND" => ActorDamageType::Sound,
                "NETHER" => ActorDamageType::Nether,
                "NEXUS" => ActorDamageType::Nexus,
                "CHAOS" => ActorDamageType::Chaos,
                "DISEN" => ActorDamageType::Disenchant,
                "FEAR" => ActorDamageType::Fear,
                _ => unreachable!(),
            };
            item.intrinsic_properties
                .resistances
                .entry(damage)
                .or_insert(ActorResistanceLevel::Vulnerable);
        }
    }
}

pub(super) fn finalize_materialization(
    content: &ContentCatalog,
    rng: &mut RfbRng,
    kind: &str,
    result: &mut EgoMaterialization,
) {
    let mut item = GeneratedItemDraft {
        artifact_name: None,
        intrinsic_melee_damage_dice: None,
        intrinsic_weight_tenths_pound: None,
        intrinsic_weapon_traits: Default::default(),
        intrinsic_curse_effects: Default::default(),
        permanent_destruction_immunities: Default::default(),
        kind_id: kind.to_owned(),
        quantity: 1,
        origin_kind: None,
        quality: rfb_protocol::ItemQualityDto::Ordinary,
        affix_ids: result.affix_ids.clone(),
        rolled_affixes: result.rolled_affixes.clone(),
        intrinsic_properties: result.intrinsic_properties.clone().unwrap_or_default(),
        enchantments: result.enchantment_delta,
        damage_dice_override: result.ammunition_damage_dice,
        curse: result.curse,
        activation: result.activation.clone(),
        charges: result.charges,
        fuel: crate::save::initial_item_fuel(content, kind),
    }
    .into_item_instance(String::new(), crate::state::ItemLocation::Inventory);
    curse_object(content, rng, &mut item);
    result.rolled_affixes = item.rolled_affixes;
    result.intrinsic_properties = Some(item.intrinsic_properties);
    result.curse = item.curse;
    result.curse_effects = result
        .rolled_affixes
        .iter()
        .flat_map(|roll| roll.curse_effects.iter().copied())
        .collect();
    result.curse_on_finalize = false;
}

pub(in crate::game) fn finalize_draft(
    content: &ContentCatalog,
    rng: &mut RfbRng,
    draft: &mut GeneratedItemDraft,
    final_curse: bool,
) {
    let mut item = draft
        .clone()
        .into_item_instance(String::new(), crate::state::ItemLocation::Inventory);
    if final_curse {
        curse_object(content, rng, &mut item);
    } else {
        let object = item_value::instance::value_object(content, &item).unwrap();
        if ((16..=23).contains(&object.tval) && object.to_h + object.to_d < 0)
            || ((30..=38).contains(&object.tval) && object.to_a < 0)
        {
            item.curse = Some(item.curse.unwrap_or(ItemCurseSeverityDto::Normal));
        }
    }
    draft.intrinsic_properties = item.intrinsic_properties;
    draft.rolled_affixes = item.rolled_affixes;
    draft.curse = item.curse;
}

pub(in crate::game) fn decrease_shared_pval(content: &ContentCatalog, item: &mut ItemInstance) {
    let object = item_value::instance::value_object(content, item).unwrap();
    let value = object.pval - 1;
    for flag in RfbPvalFlagDefinition::ALL {
        if object.flags.contains(flag.source_flag()) {
            armor::apply_pval(&mut item.intrinsic_properties, flag, -1);
            remember_rfb_pval(&mut item.intrinsic_properties, [flag], value);
        }
    }
    for raw in item
        .rolled_affixes
        .iter_mut()
        .filter_map(|roll| roll.properties.rfb_pval.as_mut())
    {
        raw.value = value as i16;
    }
}
