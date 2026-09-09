// SPDX-License-Identifier: MPL-2.0

use super::*;
use EquipmentPassive as Passive;
use Pval::*;
use armor::{Pval, apply_pval};

fn pval(rng: &mut RfbRng, maximum: u16, level: u16) -> u16 {
    let maximum = 1 + rfb_m_bonus(rng, maximum - 1, level);
    randint1(rng, maximum)
}

fn bonus(rng: &mut RfbRng, maximum: u16, level: u16) -> i16 {
    (randint1(rng, maximum) + rfb_m_bonus(rng, maximum, level)) as i16
}

fn level_check(rng: &mut RfbRng, power: u16, level: i32) -> bool {
    level > 0 && rng.bounded((i32::from(power) * 100 / level).max(1) as u64) < 100
}

pub(in crate::game) fn roll(
    content: &ContentCatalog,
    rng: &mut RfbRng,
    item: &ItemDefinition,
    level: u16,
    power: i16,
) -> Option<EgoMaterialization> {
    let category = match item.rfb_base_kind?.tval {
        40 => RfbEgoTypeDefinition::Amulet,
        45 => RfbEgoTypeDefinition::Ring,
        _ => return None,
    };
    let id = roll_rfb_ego_from_affixes(content.affix_definitions(), rng, level, &[category])?;
    materialize(rng, item, content.affix(id)?, level, power)
}

pub(super) fn can_apply(index: u32, tval: u16) -> bool {
    match index {
        200..=201 => matches!(tval, 40 | 45),
        205..=211 => tval == 45,
        220..=227 => tval == 40,
        _ => false,
    }
}

pub(super) fn materialize(
    rng: &mut RfbRng,
    item: &ItemDefinition,
    affix: &AffixDefinition,
    level: u16,
    mut power: i16,
) -> Option<EgoMaterialization> {
    let index = affix.rfb_ego.as_ref()?.source_index;
    let tval = item.rfb_base_kind?.tval;
    if !can_apply(index, tval) {
        return None;
    }
    let mut state = RolledAffixState {
        affix_id: affix.id.clone(),
        ..Default::default()
    };
    let properties = &mut state.properties;
    let delta = &mut state.enchantment_delta;
    let mut flags = BTreeSet::new();
    let mut value = 0;
    let mut curse = None;
    let mut activation = None;
    // The object flag set shares one pval, including when a later power replaces it.
    match index {
        200 => defender(rng, properties, &mut delta.to_armor, level, power),
        201 => elemental(
            rng,
            properties,
            &mut delta.to_armor,
            level,
            power,
            tval,
            affix,
            &mut activation,
        ),
        210 | 211 => {
            delta.to_damage += if index == 210 { 6 } else { 5 };
            if index == 210 {
                delta.to_hit += 6;
                flags.extend([
                    Stealth,
                    Speed,
                    Strength,
                    Intelligence,
                    LessWisdom,
                    Dexterity,
                    LessConstitution,
                    Charisma,
                    LessLife,
                ]);
            } else {
                flags.extend([Strength, Constitution, Digging, LessStealth]);
                if one_in(rng, 3) {
                    flags.insert(LessDexterity);
                }
                if one_in(rng, 3) {
                    flags.insert(Wisdom);
                }
            }
            curse = Some(if one_in(rng, 6) {
                ItemCurseSeverityDto::Permanent
            } else {
                ItemCurseSeverityDto::Heavy
            });
            if index == 210 {
                if one_in(rng, 66) {
                    properties
                        .resistances
                        .insert(ActorDamageType::Cold, ActorResistanceLevel::Immune);
                }
                if one_in(rng, 6) {
                    add_slay(properties, SlayTarget::Good, SlayLevel::Slay);
                }
                if one_in(rng, 6) {
                    properties.brands.insert(WeaponBrand::Cold);
                }
                if one_in(rng, 6) {
                    add_slay(properties, SlayTarget::Human, SlayLevel::Slay);
                }
            } else {
                if one_in(rng, 6) {
                    state.curse_effects.insert(ItemCurseEffectDto::Aggravate);
                }
                if one_in(rng, 2) {
                    add_resistance(properties, ActorDamageType::Dark);
                }
                if one_in(rng, 3) {
                    add_resistance(properties, ActorDamageType::Disenchant);
                }
                if one_in(rng, 3) {
                    properties.passives.insert(Passive::SustainStrength);
                }
            }
            if one_in(rng, 6) {
                add_one_high_resistance(rng, properties);
            }
        }
        206 | 207 => {
            let archery = index == 207;
            let div = if power.abs() >= 2 { 2 } else { 1 };
            let mut powers = power.abs() + rfb_m_bonus(rng, 5, level) as i16;
            while powers > 0 {
                let roll = randint1(rng, 10);
                if (!archery && roll <= 3) || (archery && roll == 1) {
                    let (flag, sustain) = match roll {
                        1 if !archery => (Constitution, Passive::SustainConstitution),
                        3 => (Strength, Passive::SustainStrength),
                        _ => (Dexterity, Passive::SustainDexterity),
                    };
                    if flags.insert(flag) {
                        if value == 0 {
                            value = pval(rng, 5, level);
                        }
                        if !archery && one_in(rng, 6) {
                            properties.passives.insert(sustain);
                        }
                        powers -= 1;
                    }
                } else if archery && roll == 2 {
                    let flag = if !flags.contains(&Speed)
                        && level_check(rng, 200 / div, i32::from(level) - 40)
                    {
                        Speed
                    } else {
                        Stealth
                    };
                    if flags.insert(flag) {
                        if value == 0 {
                            value = pval(rng, if flag == Speed { 3 } else { 5 }, level);
                        }
                        powers -= 1;
                    }
                } else if (!archery && (4..=7).contains(&roll))
                    || (archery && (3..=6).contains(&roll))
                {
                    let hit = roll <= if archery { 4 } else { 5 };
                    let maximum = if archery && !hit { 7 } else { 5 };
                    let target = if hit {
                        &mut delta.to_hit
                    } else {
                        &mut delta.to_damage
                    };
                    *target += bonus(rng, maximum, level);
                    powers -= 1;
                    while one_in(rng, 2) && powers > 0 {
                        *target += bonus(rng, maximum, level);
                        powers -= 1;
                    }
                } else if (!archery && (8..=9).contains(&roll))
                    || (archery && (7..=8).contains(&roll))
                {
                    delta.to_hit += bonus(rng, 4, level);
                    delta.to_damage += if archery {
                        (randint1(rng, 6) + rfb_m_bonus(rng, 5, level)) as i16
                    } else {
                        bonus(rng, 4, level)
                    };
                    powers -= 1;
                } else if archery {
                    let (flag, other, chance) = if roll == 9 {
                        (Shots, Might, 100)
                    } else {
                        (Might, Shots, 200)
                    };
                    if !flags.contains(&flag)
                        && level_check(rng, chance / div, i32::from(level))
                        && (!flags.contains(&other) || one_in(rng, 7))
                    {
                        flags.insert(flag);
                        if value == 0 {
                            value = pval(rng, 5, level);
                        }
                        powers -= 1;
                    }
                } else if power.abs() >= 2 && one_in(rng, 30) && level >= 50 {
                    flags.insert(WeaponMastery);
                    value = pval(rng, 3, level);
                    powers -= 1;
                    if one_in(rng, 30) && powers > 0 {
                        properties.brands.insert(
                            [
                                WeaponBrand::Acid,
                                WeaponBrand::Cold,
                                WeaponBrand::Fire,
                                WeaponBrand::Electricity,
                                WeaponBrand::Poison,
                            ][rng.bounded(5) as usize],
                        );
                        powers -= 1;
                    }
                } else if power.abs() >= 2 && one_in(rng, 15) && level >= 50 {
                    flags.insert(Blows);
                    value = pval(rng, 3, level);
                    powers = 0;
                } else if !properties
                    .status_immunities
                    .iter()
                    .any(|id| id == "rfb.status.fear")
                {
                    add_status_immunity(properties, "rfb.status.fear");
                    powers -= 1;
                }
            }
            delta.to_hit = delta.to_hit.min(if archery { 30 } else { 25 });
            delta.to_damage = delta.to_damage.min(if archery && !flags.contains(&Shots) {
                40
            } else {
                20
            });
            if archery
                && value > 3
                && (flags.contains(&Shots) || flags.contains(&Might))
                && !one_in(rng, 10)
            {
                value = 3;
            }
        }
        205 => {
            let mut powers = power.abs() + rfb_m_bonus(rng, 5, level) as i16;
            while powers > 0 {
                match randint1(rng, 7) {
                    1 => {
                        add_one_high_resistance(rng, properties);
                        if power.abs() >= 2 {
                            let mut count = 0;
                            loop {
                                add_one_high_resistance(rng, properties);
                                power -= 1;
                                count += 1;
                                if !one_in(rng, 2 + count) {
                                    break;
                                }
                            }
                        }
                    }
                    roll => {
                        // Original switch fallthrough tries the later abilities in order.
                        if roll <= 2
                            && !properties
                                .status_immunities
                                .iter()
                                .any(|id| id == "rfb.status.paralysis")
                        {
                            add_status_immunity(properties, "rfb.status.paralysis");
                        } else if roll <= 3 && properties.passives.insert(Passive::SeeInvisible) {
                        } else if roll <= 4 && one_in(rng, 2) {
                            properties.passives.insert(Passive::Warning);
                            if one_in(rng, 3) {
                                add_one_low_esp(rng, properties);
                            }
                        } else if roll <= 5 && one_in(rng, 2) {
                            add_one_sustain(rng, properties);
                            if power.abs() >= 2 {
                                loop {
                                    add_one_sustain(rng, properties);
                                    power -= 1;
                                    if !one_in(rng, 2) {
                                        break;
                                    }
                                }
                            }
                        } else {
                            delta.to_armor += randint1(rng, 10) as i16;
                        }
                    }
                }
                powers -= 1;
            }
            delta.to_armor = delta.to_armor.min(35);
        }
        209 => {
            let amount = 5 + if level >= 30 {
                (level.min(80) - 30) / 10
            } else {
                0
            };
            value = 1 + rfb_m_bonus(rng, amount, level);
            if (rng.bounded(20) as i32) < i32::from(level) - 50 {
                while one_in(rng, 2) {
                    value += 1;
                }
            }
            flags.insert(Speed);
            if level >= 50 && one_in(rng, 10) {
                let token = if one_in(rng, 777) {
                    "light-speed"
                } else if one_in(rng, 77) {
                    "speed-hero"
                } else {
                    "speed"
                };
                activation = activation_token(affix, token);
            }
        }
        208 | 224 => {
            let magi = index == 224;
            if magi {
                flags.insert(Search);
            }
            let mut powers = power.abs() + rfb_m_bonus(rng, if magi { 5 } else { 4 }, level) as i16;
            while powers > 0 {
                let roll = randint1(rng, 7);
                match roll {
                    1 if magi => {
                        add_status_immunity(properties, "rfb.status.paralysis");
                        properties.passives.insert(Passive::SeeInvisible);
                    }
                    1 => {
                        flags.insert(Intelligence);
                        if value == 0 {
                            value = pval(rng, 5, level);
                        }
                    }
                    2 => {
                        properties.passives.insert(Passive::SustainIntelligence);
                    }
                    3 if magi => {
                        properties.passives.insert(
                            if power.abs() >= 2 && one_in(rng, 10) && level >= 70 {
                                Passive::ManaRegeneration
                            } else {
                                Passive::EasySpell
                            },
                        );
                    }
                    3 => {
                        flags.insert(Capacity);
                        if value == 0 {
                            value = pval(rng, 3, level);
                        } else {
                            value = value.min(3);
                        }
                    }
                    4 if magi => {
                        if power.abs() >= 2 && one_in(rng, 2) {
                            properties.passives.insert(Passive::Telepathy);
                        } else {
                            add_one_low_esp(rng, properties);
                        }
                    }
                    4 => {
                        properties.passives.insert(Passive::EasySpell);
                    }
                    5 if power.abs() >= 2 => {
                        properties.passives.insert(Passive::ReducedManaCost);
                    }
                    5 if magi && one_in(rng, 2) => {
                        flags.insert(Mastery);
                        if value == 0 {
                            value = pval(rng, 5, level);
                        }
                    }
                    5 if one_in(rng, 3) => {
                        add_resistance(properties, ActorDamageType::Confusion);
                    }
                    5 if one_in(rng, 3) => {
                        add_status_immunity(properties, "rfb.status.blindness");
                    }
                    5 if !magi => {
                        properties.passives.insert(Passive::Levitation);
                    }
                    5 | 6 => {
                        if power.abs() >= 2 && one_in(rng, if magi { 15 } else { 30 }) {
                            flags.extend([
                                SpellPower,
                                LessStrength,
                                LessDexterity,
                                LessConstitution,
                            ]);
                            value = pval(rng, 2, level);
                        } else {
                            delta.to_damage += bonus(rng, 5, level);
                            while one_in(rng, 2) && powers > 0 {
                                delta.to_damage += bonus(rng, 5, level);
                                powers -= 1;
                            }
                        }
                    }
                    _ if magi => {
                        flags.insert(Intelligence);
                        if value == 0 {
                            value = pval(rng, 5, level);
                        }
                    }
                    _ => {
                        if power.abs() >= 2 && one_in(rng, 15) {
                            properties.passives.insert(Passive::Telepathy);
                        } else {
                            add_one_low_esp(rng, properties);
                        }
                    }
                }
                powers -= 1;
            }
            if magi {
                if value == 0 {
                    value = randint1(rng, 8);
                }
                if flags.contains(&Mastery) && value > 1 {
                    value -= 1;
                }
            }
            delta.to_damage = delta.to_damage.min(20);
        }
        225 => {
            delta.to_armor = bonus(rng, 5, level);
            delta.to_hit = (randint1(rng, 3) + rfb_m_bonus(rng, 5, level)) as i16;
            delta.to_damage = (randint1(rng, 3) + rfb_m_bonus(rng, 5, level)) as i16;
            for passive in [
                Passive::SlowDigestion,
                Passive::SustainConstitution,
                Passive::SustainStrength,
                Passive::SustainDexterity,
            ] {
                if one_in(rng, 3) {
                    properties.passives.insert(passive);
                }
            }
        }
        220..=223 | 226..=227 => amulet(
            rng,
            index,
            level,
            power,
            properties,
            delta,
            &mut flags,
            &mut value,
            &mut curse,
            &mut state.curse_effects,
        ),
        _ => unreachable!(),
    }
    if !matches!(index, 209 | 223) && one_in(rng, if index == 220 { 10 } else { 5 }) {
        activation = armor::random_activation(rng, affix, level, index == 220);
    }
    for (element, destruction) in [
        (
            ActorDamageType::Acid,
            rfb_content::ItemDestructionElement::Acid,
        ),
        (
            ActorDamageType::Electricity,
            rfb_content::ItemDestructionElement::Electricity,
        ),
        (
            ActorDamageType::Fire,
            rfb_content::ItemDestructionElement::Fire,
        ),
        (
            ActorDamageType::Cold,
            rfb_content::ItemDestructionElement::Cold,
        ),
    ] {
        if properties.resistances.contains_key(&element) {
            state.elemental_destruction_immunities.insert(destruction);
        }
    }
    if properties.brands.contains(&WeaponBrand::Fire) {
        add_light(properties);
    }
    if matches!(index, 210 | 211) {
        state
            .curse_effects
            .insert(ItemCurseEffectDto::DrainExperience);
        state.curse_effects.insert(roll_rfb_heavy_curse_effect(rng));
        add_one_high_resistance(rng, properties);
        if randint1(rng, level) > 60 {
            add_one_high_resistance(rng, properties);
        }
        delta.to_hit += roll_signed(rng, if index == 210 { 13 } else { -10 });
        delta.to_damage += roll_signed(rng, 13);
        value += randint1(rng, 2);
    }
    for flag in flags {
        apply_pval(properties, flag, i32::from(value));
    }
    properties.status_immunities.sort();
    let profile = activation.and_then(|i| affix.device_generation.as_ref()?.activations.get(i));
    let (activation, charges) = profile.map(materialize_rfb_activation).unzip();
    Some(EgoMaterialization::new(
        vec![affix.id.clone()],
        state
            .has_instance_state()
            .then_some(state)
            .into_iter()
            .collect(),
        None,
        curse,
        activation,
        charges,
    ))
}

fn activation_token(affix: &AffixDefinition, token: &str) -> Option<usize> {
    affix
        .device_generation
        .as_ref()?
        .activations
        .iter()
        .position(|profile| {
            profile.id
                == format!(
                    "rfb.device-activation.ego-{}-{token}",
                    affix.rfb_ego.as_ref().unwrap().source_index
                )
        })
}

#[allow(clippy::too_many_arguments)]
fn amulet(
    rng: &mut RfbRng,
    index: u32,
    level: u16,
    power: i16,
    properties: &mut AffixPropertyBundleDefinition,
    delta: &mut ItemEnchantmentsDto,
    flags: &mut BTreeSet<Pval>,
    value: &mut u16,
    curse: &mut Option<ItemCurseSeverityDto>,
    curse_effects: &mut BTreeSet<ItemCurseEffectDto>,
) {
    if index == 221 {
        properties.passives.insert(Passive::Blessed);
        delta.to_armor = randint1(rng, 5) as i16;
        if one_in(rng, 2) {
            add_light(properties);
        }
    }
    if index == 222 {
        *curse = Some(ItemCurseSeverityDto::Normal);
        delta.to_armor = -5;
    }
    if index == 223 {
        flags.insert(Infra);
        if one_in(rng, 2) {
            add_light(properties);
        }
    }
    if index == 227 {
        flags.insert(Search);
    }
    let mut powers = power.abs() + rfb_m_bonus(rng, 5, level) as i16;
    while powers > 0 {
        let roll = randint1(
            rng,
            match index {
                220 => 6,
                221 => 8,
                223 => 9,
                _ => 7,
            },
        );
        let mut attribute = None;
        match (index, roll) {
            (220 | 221 | 223, 1) => attribute = Some((Strength, 4)),
            (220, 2) => attribute = Some((Dexterity, 4)),
            (220, 3) => {
                if one_in(rng, 3) {
                    add_status_immunity(properties, "rfb.status.paralysis");
                } else if level > 30 {
                    properties.passives.insert(Passive::AntiMagic);
                    if power.abs() >= 2 && one_in(rng, 10) && level >= 70 {
                        flags.insert(MagicResistance);
                        *value = pval(rng, 3, level);
                    }
                } else if one_in(rng, 2) {
                    delta.to_armor += 1 + rfb_m_bonus(rng, 4, level) as i16;
                } else {
                    attribute = Some((LessIntelligence, 4));
                }
            }
            (220, 4) => {
                if power.abs() >= 2 && one_in(rng, 10) && level >= 70 {
                    properties.passives.insert(Passive::AntiSummoning);
                } else if one_in(rng, 6) && level > 30 {
                    properties.passives.insert(Passive::AntiTeleport);
                } else {
                    add_status_immunity(properties, "rfb.status.fear");
                }
            }
            (220, 5) => attribute = Some((LessIntelligence, 4)),
            (220, _) => delta.to_armor += 1 + rfb_m_bonus(rng, 4, level) as i16,
            (221, 2) => attribute = Some((Wisdom, 4)),
            (221, 3) => {
                add_status_immunity(properties, "rfb.status.paralysis");
                if one_in(rng, 2) {
                    properties.passives.insert(Passive::SeeInvisible);
                }
            }
            (221, 4) => {
                if power.abs() >= 2 && one_in(rng, 10) && level >= 50 {
                    properties.passives.insert(Passive::ReflectsBolts);
                } else if one_in(rng, 5) {
                    add_resistance(properties, ActorDamageType::Chaos);
                } else if one_in(rng, 3) {
                    add_resistance(properties, ActorDamageType::Confusion);
                } else if one_in(rng, 3) {
                    add_resistance(properties, ActorDamageType::Nether);
                } else {
                    add_status_immunity(properties, "rfb.status.fear");
                }
            }
            (221, 5) => {
                if power.abs() >= 2 && one_in(rng, 20) && level >= 70 {
                    flags.insert(Speed);
                    *value = pval(rng, 3, level);
                } else if one_in(rng, 7) && level >= 50 {
                    attribute = Some((Life, 4));
                } else {
                    properties.passives.insert(if one_in(rng, 2) {
                        Passive::Regeneration
                    } else {
                        Passive::HoldLife
                    });
                }
            }
            (221, _) => delta.to_armor += 1 + rfb_m_bonus(rng, 5, level) as i16,
            (222, 1) => {
                if one_in(rng, 3) {
                    curse_effects.insert(ItemCurseEffectDto::Aggravate);
                    add_one_demon_resistance(rng, properties);
                } else {
                    flags.extend([LessStealth, LessWisdom]);
                    if *value == 0 {
                        *value = randint1(rng, 7);
                    }
                }
            }
            (222, 2) => {
                delta.to_armor -= bonus(rng, 5, level);
                add_one_demon_resistance(rng, properties);
            }
            (222, 3) => {
                add_status_immunity(properties, "rfb.status.paralysis");
                if one_in(rng, 2) {
                    properties.passives.insert(Passive::SeeInvisible);
                }
                if one_in(rng, 6) {
                    properties
                        .resistances
                        .insert(ActorDamageType::Cold, ActorResistanceLevel::Vulnerable);
                    delta.to_hit += randint1(rng, 3) as i16;
                    delta.to_damage += randint1(rng, 5) as i16;
                    delta.to_armor -= randint1(rng, 5) as i16;
                    add_one_demon_resistance(rng, properties);
                }
            }
            (222, 4) => delta.to_hit += bonus(rng, 5, level),
            (222, 5) => {
                delta.to_damage += bonus(rng, 5, level);
                if one_in(rng, 6) {
                    curse_effects.insert(ItemCurseEffectDto::DrainExperience);
                    add_one_demon_resistance(rng, properties);
                }
            }
            (222, 6) if power.abs() >= 2 && one_in(rng, 66) && level >= 66 => {
                curse_effects.insert(ItemCurseEffectDto::TyCurse);
                properties
                    .resistances
                    .insert(ActorDamageType::Fire, ActorResistanceLevel::Immune);
                delta.to_hit += randint1(rng, 6) as i16;
                delta.to_damage += randint1(rng, 6) as i16;
                delta.to_armor -= randint1(rng, 20) as i16;
            }
            (222, roll) => {
                if roll == 6 && one_in(rng, 3) {
                    flags.insert(LessSpeed);
                    *value = randint1(rng, 3);
                    delta.to_hit += randint1(rng, 3) as i16;
                    delta.to_damage += randint1(rng, 5) as i16;
                    delta.to_armor -= randint1(rng, 5) as i16;
                    add_one_demon_resistance(rng, properties);
                } else {
                    delta.to_hit += randint1(rng, 3) as i16;
                    delta.to_damage += randint1(rng, 5) as i16;
                }
            }
            (223, 2) => {
                flags.insert(if one_in(rng, 3) {
                    LessDexterity
                } else {
                    LessStealth
                });
                if one_in(rng, 5) {
                    flags.insert(Wisdom);
                }
                if *value == 0 {
                    *value = pval(rng, 4, level);
                }
            }
            (223, 3) => attribute = Some((Constitution, 4)),
            (223, 4) => add_status_immunity(properties, "rfb.status.blindness"),
            (223, 5) => add_resistance(properties, ActorDamageType::Dark),
            (223, 6) => add_resistance(properties, ActorDamageType::Disenchant),
            (223, 7) => add_status_immunity(properties, "rfb.status.paralysis"),
            (223, _) => {
                properties.passives.insert(Passive::Regeneration);
            }
            (226, 1) => attribute = Some((Charisma, 5)),
            (226, 2) => {
                properties.passives.insert(Passive::ReflectsBolts);
            }
            (226, 3) => {
                if power.abs() >= 2 && one_in(rng, 2) && level >= 30 {
                    flags.insert(Capacity);
                    *value = pval(rng, 3, level);
                } else {
                    properties.passives.insert(Passive::HoldLife);
                    if one_in(rng, 2) {
                        add_status_immunity(properties, "rfb.status.paralysis");
                    }
                    if one_in(rng, 2) {
                        properties.passives.insert(Passive::SeeInvisible);
                    }
                }
            }
            (226, 4) => add_one_high_resistance(rng, properties),
            (226, 5) if power.abs() >= 2 && one_in(rng, 2) && level >= 30 => {
                let mut count = 0;
                loop {
                    add_one_high_resistance(rng, properties);
                    powers -= 1;
                    count += 1;
                    if !one_in(rng, 2 + count) {
                        break;
                    }
                }
            }
            (226, _) => attribute = Some((Wisdom, 5)),
            (227, 1) => attribute = Some((Dexterity, 5)),
            (227, 2) => {
                properties.passives.insert(Passive::SustainDexterity);
            }
            (227, 3) => {
                add_resistance(
                    properties,
                    if one_in(rng, 2) {
                        ActorDamageType::Poison
                    } else {
                        ActorDamageType::Dark
                    },
                );
            }
            (227, 4) => {
                add_resistance(
                    properties,
                    if one_in(rng, 2) {
                        ActorDamageType::Nexus
                    } else {
                        ActorDamageType::Confusion
                    },
                );
            }
            (227, roll) => {
                if roll == 5 && power.abs() >= 2 && one_in(rng, 2) && level >= 50 {
                    properties.passives.insert(Passive::Telepathy);
                } else if roll <= 6 && power.abs() >= 2 && one_in(rng, 2) && level >= 50 {
                    flags.insert(Speed);
                    *value = pval(rng, 3, level);
                } else {
                    flags.insert(Stealth);
                }
            }
            _ => unreachable!(),
        }
        if let Some((flag, maximum)) = attribute {
            flags.insert(flag);
            if *value == 0 {
                *value = pval(rng, maximum, level);
            }
        }
        powers -= 1;
    }
    match index {
        220 => delta.to_armor = delta.to_armor.min(12),
        221 => delta.to_armor = delta.to_armor.min(15),
        222 => {
            delta.to_armor = delta.to_armor.max(-20);
            delta.to_hit = delta.to_hit.min(20);
            if delta.to_damage > 16 {
                curse_effects.insert(ItemCurseEffectDto::Aggravate);
                delta.to_damage = 16;
            }
        }
        223 if *value == 0 => *value = 2 + randint1(rng, 6),
        227 if *value == 0 => *value = randint1(rng, 5),
        _ => {}
    }
}

fn defender(
    rng: &mut RfbRng,
    properties: &mut AffixPropertyBundleDefinition,
    armor: &mut i16,
    level: u16,
    power: i16,
) {
    add_status_immunity(properties, "rfb.status.paralysis");
    properties.passives.insert(Passive::SeeInvisible);
    if power.abs() >= 2 && level > 50 {
        if one_in(rng, 2) {
            properties.passives.insert(Passive::Levitation);
        }
        while one_in(rng, 2) {
            add_one_sustain(rng, properties);
        }
        *armor = bonus(rng, 7, level);
        match randint1(rng, 4) {
            1 => {
                for element in [
                    ActorDamageType::Acid,
                    ActorDamageType::Electricity,
                    ActorDamageType::Fire,
                    ActorDamageType::Cold,
                ] {
                    add_resistance(properties, element);
                }
                if one_in(rng, 3) {
                    add_resistance(properties, ActorDamageType::Poison);
                } else {
                    add_one_high_resistance(rng, properties);
                }
            }
            2 => {
                add_one_high_resistance(rng, properties);
                let mut count = 0;
                loop {
                    add_one_high_resistance(rng, properties);
                    count += 1;
                    if !one_in(rng, 2 + count) {
                        break;
                    }
                }
            }
            3 => {
                *armor += 5;
                add_resistance(properties, ActorDamageType::Poison);
                add_resistance(properties, ActorDamageType::Disenchant);
                properties.passives.insert(Passive::HoldLife);
                loop {
                    add_lordly_resistance(rng, properties);
                    if !one_in(rng, 4) {
                        break;
                    }
                }
            }
            _ => {
                properties.passives.extend([
                    Passive::ColdAura,
                    Passive::ElectricityAura,
                    Passive::FireAura,
                ]);
                if one_in(rng, 2) {
                    properties.passives.insert(Passive::ShardsAura);
                }
                if one_in(rng, 7) {
                    properties.passives.insert(Passive::RevengeAura);
                }
            }
        }
    } else {
        if one_in(rng, 5) {
            properties.passives.insert(Passive::Levitation);
        }
        if one_in(rng, 5) {
            add_one_sustain(rng, properties);
        }
        *armor = bonus(rng, 5, level);
        if one_in(rng, 3) {
            add_one_high_resistance(rng, properties);
            add_one_high_resistance(rng, properties);
        } else {
            for _ in 0..7 {
                add_one_elemental_resistance(rng, properties);
            }
        }
    }
}

fn add_lordly_resistance(rng: &mut RfbRng, properties: &mut AffixPropertyBundleDefinition) {
    match rng.bounded(10) {
        3 => add_status_immunity(properties, "rfb.status.blindness"),
        9 => add_status_immunity(properties, "rfb.status.fear"),
        index => add_resistance(
            properties,
            [
                ActorDamageType::Light,
                ActorDamageType::Dark,
                ActorDamageType::Shards,
                ActorDamageType::Blindness,
                ActorDamageType::Confusion,
                ActorDamageType::Sound,
                ActorDamageType::Nether,
                ActorDamageType::Nexus,
                ActorDamageType::Chaos,
            ][index as usize],
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn elemental(
    rng: &mut RfbRng,
    properties: &mut AffixPropertyBundleDefinition,
    armor: &mut i16,
    level: u16,
    power: i16,
    tval: u16,
    affix: &AffixDefinition,
    activation: &mut Option<usize>,
) {
    if tval == 40 && power.abs() >= 2 && randint1(rng, level) > 30 {
        for element in [
            ActorDamageType::Cold,
            ActorDamageType::Fire,
            ActorDamageType::Electricity,
        ] {
            add_resistance(properties, element);
        }
        if one_in(rng, 3) {
            add_resistance(properties, ActorDamageType::Acid);
        }
        if one_in(rng, 5) {
            add_resistance(properties, ActorDamageType::Poison);
        } else if one_in(rng, 5) {
            add_resistance(properties, ActorDamageType::Shards);
        }
    } else if tval == 45 && power.abs() >= 2 {
        match randint1(rng, 6) {
            1 => {
                for element in [
                    ActorDamageType::Cold,
                    ActorDamageType::Fire,
                    ActorDamageType::Electricity,
                ] {
                    add_resistance(properties, element);
                }
                if one_in(rng, 3) {
                    add_resistance(properties, ActorDamageType::Acid);
                }
                if one_in(rng, 5) {
                    add_resistance(properties, ActorDamageType::Poison);
                }
            }
            6 => {
                *armor = 5 + randint1(rng, 5) as i16 + rfb_m_bonus(rng, 10, level) as i16;
                add_resistance(properties, ActorDamageType::Shards);
                if one_in(rng, 3) {
                    properties.passives.insert(Passive::ShardsAura);
                }
            }
            roll => {
                *armor = 5 + randint1(rng, 5) as i16 + rfb_m_bonus(rng, 10, level) as i16;
                let (element, brand, aura, threshold, bias) = match roll {
                    2 => (
                        ActorDamageType::Fire,
                        WeaponBrand::Fire,
                        Some(Passive::FireAura),
                        70,
                        RfbActivationBiasDefinition::Fire,
                    ),
                    3 => (
                        ActorDamageType::Cold,
                        WeaponBrand::Cold,
                        Some(Passive::ColdAura),
                        70,
                        RfbActivationBiasDefinition::Cold,
                    ),
                    4 => (
                        ActorDamageType::Electricity,
                        WeaponBrand::Electricity,
                        Some(Passive::ElectricityAura),
                        75,
                        RfbActivationBiasDefinition::Electricity,
                    ),
                    _ => (
                        ActorDamageType::Acid,
                        WeaponBrand::Acid,
                        None,
                        65,
                        RfbActivationBiasDefinition::Acid,
                    ),
                };
                add_resistance(properties, element);
                if let Some(aura) = aura
                    && one_in(rng, 3)
                {
                    properties.passives.insert(aura);
                }
                if one_in(rng, 7) {
                    properties.brands.insert(brand);
                } else if randint1(rng, level) >= threshold {
                    properties
                        .resistances
                        .insert(element, ActorResistanceLevel::Immune);
                }
                if one_in(rng, 5) {
                    *activation = roll_biased_activation_profile_index(rng, affix, bias, level);
                }
            }
        }
    } else {
        add_one_elemental_resistance(rng, properties);
        if one_in(rng, 3) {
            add_one_elemental_resistance(rng, properties);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{game::Game, state::ItemLocation};

    #[test]
    fn jewelry_all_seventeen_egos_materialize_and_round_trip_both_power_levels() {
        let mut game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        let mut seen = BTreeSet::new();
        for kind in ["demo.item.ring", "demo.item.amulet"] {
            game.debug_add_generated_inventory_item(kind, kind, 50)
                .unwrap();
            let template = game.items.last().unwrap().clone();
            let definition = game.content.item(kind).unwrap();
            for affix in game.content.affix_definitions().filter(|affix| {
                affix.rfb_ego.as_ref().is_some_and(|ego| {
                    can_apply(ego.source_index, definition.rfb_base_kind.unwrap().tval)
                })
            }) {
                let source = affix.rfb_ego.as_ref().unwrap().source_index;
                seen.insert(source);
                for seed in 1..=80 {
                    for power in [1, 2] {
                        let result = materialize(
                            &mut RfbRng::seeded(seed),
                            definition,
                            affix,
                            (seed % 100 + 1) as u16,
                            power,
                        )
                        .unwrap();
                        let mut item = template.clone();
                        item.quality = rfb_protocol::ItemQualityDto::Exceptional;
                        result.apply_to(&mut item);
                        assert!(
                            item.rolled_affixes
                                .iter()
                                .any(RolledAffixState::has_instance_state),
                            "{source}"
                        );
                        let dto =
                            crate::save::inventory_to_save(std::slice::from_ref(&item)).remove(0);
                        assert_eq!(
                            crate::save::inventory_item_from_dto(dto, &game.content)
                                .unwrap_or_else(|error| panic!("{source}/{seed}/{power}: {error}")),
                            item
                        );
                    }
                }
            }
        }
        assert_eq!(
            seen,
            [200, 201]
                .into_iter()
                .chain(205..=211)
                .chain(220..=227)
                .collect()
        );
    }

    #[test]
    fn jewelry_natural_selection_includes_fifteen_egos_and_excludes_forced_rings() {
        let game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        let mut seen = BTreeSet::new();
        for kind in ["demo.item.ring", "demo.item.amulet"] {
            let definition = game.content.item(kind).unwrap();
            for seed in 1..=1500 {
                let result = roll(
                    &game.content,
                    &mut RfbRng::seeded(seed),
                    definition,
                    (seed % 100 + 1) as u16,
                    1,
                )
                .unwrap();
                seen.insert(
                    game.content
                        .affix(&result.affix_ids[0])
                        .unwrap()
                        .rfb_ego
                        .as_ref()
                        .unwrap()
                        .source_index,
                );
            }
        }
        assert_eq!(
            seen,
            [200, 201]
                .into_iter()
                .chain(205..=209)
                .chain(220..=227)
                .collect()
        );
    }

    #[test]
    fn jewelry_spell_damage_and_weapon_mastery_reach_equipped_consumers() {
        let mut game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        game.debug_add_generated_inventory_item("test.jewelry", "demo.item.ring", 50)
            .unwrap();
        let item = game.items.last_mut().unwrap();
        item.location = ItemLocation::Equipped {
            slot_id: game
                .body_slots
                .iter()
                .find(|slot| slot.slot_type == "ring")
                .unwrap()
                .id
                .clone(),
        };
        item.affix_ids = vec!["rfb-legacy.affix.wizardry-ring".to_owned()];
        item.enchantments.to_damage = 15;
        assert_eq!(game.armor_spell_damage_bonus(), 15);
        let before = game.player_melee_profile(&game.player_derived_stats());
        let item = game.items.last_mut().unwrap();
        item.affix_ids = vec!["rfb-legacy.affix.combat-ring".to_owned()];
        item.rolled_affixes = vec![RolledAffixState {
            affix_id: item.affix_ids[0].clone(),
            properties: AffixPropertyBundleDefinition {
                equipment_bonuses: rfb_content::EquipmentBonuses {
                    weapon_dice_bonus: 2,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        }];
        let after = game.player_melee_profile(&game.player_derived_stats());
        assert_eq!(game.armor_spell_damage_bonus(), 0);
        assert_eq!(after.damage_dice, before.damage_dice + 2);
        assert_eq!(after.to_damage, before.to_damage + 15);
    }

    #[test]
    fn jewelry_ring_bonuses_follow_the_weapon_hand_and_two_handed_grip() {
        let mut game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        let weapon_id = game.equipped_melee_weapons()[0].id.clone();
        let offhand_slot = game
            .body_slots
            .iter()
            .find(|slot| slot.slot_type == "shield")
            .unwrap()
            .id
            .clone();
        game.items.retain(|item| !matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == &offhand_slot));
        let weapon = game
            .items
            .iter_mut()
            .find(|item| item.id == weapon_id)
            .unwrap();
        weapon.kind_id = "demo.item.dagger".to_owned();
        game.debug_add_generated_inventory_item("test.ring", "demo.item.ring", 50)
            .unwrap();
        let ring = game.items.last_mut().unwrap();
        ring.location = ItemLocation::Equipped {
            slot_id: game
                .body_slots
                .iter()
                .filter(|slot| slot.slot_type == "ring")
                .nth(1)
                .unwrap()
                .id
                .clone(),
        };
        ring.affix_ids = vec!["rfb-legacy.affix.combat-ring".to_owned()];
        ring.enchantments = ItemEnchantmentsDto {
            to_hit: 9,
            to_damage: 12,
            to_armor: 0,
        };
        ring.rolled_affixes = vec![RolledAffixState {
            affix_id: ring.affix_ids[0].clone(),
            properties: AffixPropertyBundleDefinition {
                equipment_bonuses: rfb_content::EquipmentBonuses {
                    weapon_dice_bonus: 2,
                    melee_attacks_delta_percent: 50,
                    ..Default::default()
                },
                brands: [WeaponBrand::Fire].into_iter().collect(),
                ..Default::default()
            },
            ..Default::default()
        }];
        let dagger = game.player_melee_profile(&game.player_derived_stats());
        game.items
            .iter_mut()
            .find(|item| item.id == weapon_id)
            .unwrap()
            .kind_id = "demo.item.long-sword".to_owned();
        let two_handed = game.player_melee_profile(&game.player_derived_stats());
        assert_eq!(two_handed.damage_dice, 4);
        assert_eq!(two_handed.to_damage, dagger.to_damage + 12);
        assert_eq!(two_handed.extra_attack_chance_percent, 50);
        game.debug_add_generated_inventory_item("test.shield", "demo.item.small-metal-shield", 1)
            .unwrap();
        let shield = game.items.last_mut().unwrap();
        shield.affix_ids.clear();
        shield.rolled_affixes.clear();
        shield.location = ItemLocation::Equipped {
            slot_id: offhand_slot,
        };
        let shielded = game.player_melee_profile(&game.player_derived_stats());
        assert_eq!(shielded.damage_dice, 2);
        assert_eq!(shielded.extra_attack_chance_percent, 0);
        assert_eq!(shielded.to_damage, two_handed.to_damage - 12);
        game.items.last_mut().unwrap().kind_id = "demo.item.dagger".to_owned();
        let dual = game.player_melee_profiles(&game.player_derived_stats());
        assert_eq!(dual[0].damage_dice, 2);
        assert_eq!(dual[0].extra_attack_chance_percent, 0);
        assert_eq!(dual[1].damage_dice, 3);
        assert_eq!(dual[1].extra_attack_chance_percent, 50);
    }

    #[test]
    fn jewelry_anti_teleport_stops_actual_item_travel_and_round_trips() {
        let mut game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        game.debug_add_generated_inventory_item("test.amulet", "demo.item.amulet", 70)
            .unwrap();
        let item = game.items.last_mut().unwrap();
        item.location = ItemLocation::Equipped {
            slot_id: game
                .body_slots
                .iter()
                .find(|slot| slot.slot_type == "amulet")
                .unwrap()
                .id
                .clone(),
        };
        item.quality = rfb_protocol::ItemQualityDto::Exceptional;
        item.activation = None;
        item.charges = None;
        item.device_recovery_progress = 0;
        item.affix_ids = vec!["rfb-legacy.affix.barbarian-talisman".to_owned()];
        item.rolled_affixes = vec![RolledAffixState {
            affix_id: item.affix_ids[0].clone(),
            properties: AffixPropertyBundleDefinition {
                passives: [Passive::AntiTeleport, Passive::AntiSummoning]
                    .into_iter()
                    .collect(),
                ..Default::default()
            },
            ..Default::default()
        }];
        let before = game.player.position;
        let destination = game.random_teleport_candidates(10)[0];
        game.resolve_item_random_teleport(
            "test".to_owned(),
            None,
            vec![destination],
            &mut Vec::new(),
            &mut BTreeSet::new(),
        );
        assert_eq!(game.player.position, before);
        assert!(game.player_has_anti_teleport());
        let blocked = (0..300)
            .filter(|_| game.equipment_blocks_summoning())
            .count();
        assert!((150..=250).contains(&blocked));
        let candidate = game
            .content
            .actor_definitions()
            .find(|actor| {
                actor.role == rfb_content::ActorRole::Monster
                    && actor.level == 1
                    && !actor.tags.iter().any(|tag| tag == "unique")
            })
            .unwrap()
            .id
            .clone();
        let blocked_seed = (1..)
            .find(|seed| RfbRng::seeded(*seed).bounded(3) != 0)
            .unwrap();
        for is_spell in [false, true] {
            game.rng = RfbRng::seeded(blocked_seed);
            let resolution = game.resolve_category_summon(
                crate::game::CategorySummonSpec {
                    is_spell,
                    source_id: "test.summon",
                    owner_id: "test",
                    category: "any-monster",
                    count_dice: 0,
                    count_sides: 0,
                    count_bonus: 1,
                    maximum_count: Some(1),
                    hostile: true,
                    group_chance_percent: 0,
                    group_count_dice: 0,
                    group_count_sides: 0,
                    group_count_bonus: 1,
                    duration_turns: 0,
                },
                vec![candidate.clone()],
                vec![destination],
                &mut BTreeSet::new(),
            );
            assert_eq!(resolution.entity_ids.len(), usize::from(!is_spell));
        }
        let restored = Game::from_save(game.to_save()).unwrap();
        assert!(restored.player_has_anti_teleport());
        game.items.last_mut().unwrap().location = ItemLocation::Inventory;
        assert!(!game.equipment_blocks_summoning());
        game.resolve_item_random_teleport(
            "test".to_owned(),
            None,
            vec![destination],
            &mut Vec::new(),
            &mut BTreeSet::new(),
        );
        assert_eq!(game.player.position, destination);
    }

    #[test]
    fn jewelry_sacred_activations_execute_holiness_star_ball_and_starburst() {
        for token in ["holiness", "star-ball", "starburst"] {
            let mut game = Game::new_with_build(7, "demo.build.warrior").unwrap();
            game.entities.clear();
            game.debug_add_generated_inventory_item("test.amulet", "demo.item.amulet", 70)
                .unwrap();
            let affix = game
                .content
                .affix("rfb-legacy.affix.sacred-pendant")
                .unwrap();
            let profile = affix
                .device_generation
                .as_ref()
                .unwrap()
                .activations
                .iter()
                .find(|profile| profile.id.ends_with(&format!("-{token}")))
                .unwrap();
            let (activation, charges) = materialize_rfb_activation(profile);
            let item = game.items.last_mut().unwrap();
            item.affix_ids = vec![affix.id.clone()];
            item.rolled_affixes.clear();
            item.intrinsic_properties = Default::default();
            item.location = ItemLocation::Equipped {
                slot_id: game
                    .body_slots
                    .iter()
                    .find(|slot| slot.slot_type == "amulet")
                    .unwrap()
                    .id
                    .clone(),
            };
            item.activation = Some(activation);
            item.charges = Some(charges);
            game.player.hp = 1;
            let mut events = Vec::new();
            let mut changed = BTreeSet::new();
            for _ in 0..100 {
                game.use_inventory_item(
                    "test.amulet",
                    None,
                    None,
                    &mut events,
                    &mut changed,
                    &mut Vec::new(),
                )
                .unwrap();
                if game.items.last().unwrap().charges.unwrap().current == 0 {
                    break;
                }
            }
            assert_eq!(
                game.items.last().unwrap().charges.unwrap().current,
                0,
                "{token}"
            );
            if token == "holiness" {
                assert!(game.player.hp > 1);
                assert!(
                    game.player
                        .statuses
                        .iter()
                        .any(|status| status.kind_id == crate::effect::STATUS_PROTECTION_FROM_EVIL)
                );
            } else {
                let blasts = events
                    .iter()
                    .filter(|event| {
                        matches!(event, crate::event::DomainEvent::AbilityAreaDamage { .. })
                    })
                    .count();
                assert!(
                    blasts >= if token == "star-ball" { 5 } else { 1 },
                    "{token}: {blasts}"
                );
                assert!(!changed.is_empty());
            }
            if token == "starburst" {
                assert!(
                    game.player
                        .statuses
                        .iter()
                        .any(|status| status.kind_id == crate::effect::STATUS_BLINDNESS)
                );
            }
        }
    }
}
