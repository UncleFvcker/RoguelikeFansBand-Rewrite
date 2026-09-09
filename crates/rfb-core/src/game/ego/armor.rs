// SPDX-License-Identifier: MPL-2.0

use super::*;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Pval {
    Strength,
    Intelligence,
    Wisdom,
    Dexterity,
    Constitution,
    Charisma,
    LessStrength,
    LessIntelligence,
    LessWisdom,
    LessDexterity,
    LessConstitution,
    LessCharisma,
    Stealth,
    LessStealth,
    Speed,
    LessSpeed,
    Life,
    LessLife,
    Infra,
    Digging,
    SpellPower,
    DevicePower,
    MagicResistance,
    Might,
    Search,
    Mastery,
    Capacity,
    Blows,
    Shots,
    WeaponMastery,
}

pub(super) fn can_apply(index: u32, tval: u16, sval: u16) -> bool {
    if tval == 32 && sval == 10 && !matches!(index, 56 | 110 | 121 | 122 | 126) {
        return false;
    }
    match index {
        50 => matches!(tval, 30..=32 | 34..=37),
        51 => matches!(tval, 34..=37),
        52 => matches!(tval, 33 | 34 | 36 | 37),
        53 => matches!(tval, 34 | 36 | 37),
        54 => matches!(tval, 30 | 35),
        55 => matches!(tval, 30 | 31),
        56 => matches!(tval, 32 | 33),
        60 => tval == 34 && !matches!(sval, 2 | 4 | 9 | 10),
        61 => tval == 34 && !matches!(sval, 9 | 10),
        62 => tval == 34 && sval != 10,
        63 | 64 => tval == 34,
        70 => tval == 37 && sval != 1,
        71..=74 => tval == 37,
        75 => tval == 36,
        76 => matches!(tval, 36 | 37),
        77 => tval == 36 && sval == 1,
        80..=82 => tval == 36 && sval == 2,
        85..=90 => tval == 38,
        91 => tval == 38 && matches!(sval, 12 | 16),
        92 => tval == 38 && sval == 18,
        95..=104 => tval == 35,
        118 => tval == 32 && !matches!(sval, 2 | 9 | 10),
        121 => tval == 32 && matches!(sval, 1 | 10),
        122 => tval == 32 && sval == 10,
        110..=120 => tval == 32,
        126 => tval == 33 || (tval == 32 && sval == 10),
        125..=130 => tval == 33,
        135..=142 => tval == 31,
        147 => tval == 30 && matches!(sval, 5 | 6),
        145..=152 => tval == 30,
        _ => false,
    }
}

pub(super) fn materialize(
    rng: &mut RfbRng,
    item: &ItemDefinition,
    affix: &AffixDefinition,
    level: u16,
    intrinsic_properties: Option<&AffixPropertyBundleDefinition>,
) -> Option<EgoMaterialization> {
    use EquipmentPassive as Passive;
    use Pval::*;
    let base = item.rfb_base_kind?;
    let index = affix.rfb_ego.as_ref()?.source_index;
    if !can_apply(index, base.tval, base.sval) {
        return None;
    }
    if index == 77 && rng.bounded(2) != 0 {
        return None;
    }
    if index == 147 && base.sval == 6 && one_in(rng, 2) {
        return None;
    }
    let body = matches!(base.tval, 36 | 37);
    let mut state = RolledAffixState {
        affix_id: affix.id.clone(),
        ..Default::default()
    };
    let mut flags = BTreeSet::new();
    let mut pval = 0;
    if base.tval == 35 && base.sval == 2 {
        pval = intrinsic_properties.map_or_else(
            || randint1(rng, 4),
            |intrinsic| {
                (item.equipment_bonuses.stealth_skill + intrinsic.equipment_bonuses.stealth_skill)
                    as u16
            },
        );
        flags.extend([Stealth, Search]);
        state.properties.equipment_bonuses.stealth_skill -= item.equipment_bonuses.stealth_skill;
        state.properties.equipment_bonuses.search_skill -= item.equipment_bonuses.search_skill;
        state.properties.equipment_bonuses.perception_skill -=
            item.equipment_bonuses.perception_skill;
        if let Some(intrinsic) = intrinsic_properties {
            state.properties.equipment_bonuses.stealth_skill -=
                intrinsic.equipment_bonuses.stealth_skill;
            state.properties.equipment_bonuses.search_skill -=
                intrinsic.equipment_bonuses.search_skill;
            state.properties.equipment_bonuses.perception_skill -=
                intrinsic.equipment_bonuses.perception_skill;
        }
    }
    let mut activation = None;
    let mut curse = None;
    let properties = &mut state.properties;
    let to_a = &mut state.enchantment_delta.to_armor;
    match index {
        50 => {
            if !matches!(base.tval, 30 | 31) && one_in(rng, 3) {
                *to_a += rfb_m_bonus(rng, 10, level) as i16;
            }
        }
        51 => {
            for _ in 0..1 + rfb_m_bonus(rng, if body { 6 } else { 5 }, level) {
                add_one_elemental_resistance(rng, properties);
            }
            if level > 20 && one_in(rng, 4) {
                add_resistance(properties, ActorDamageType::Poison);
            }
            if one_in(rng, 5) {
                activation = random_activation(rng, affix, level, false);
            }
        }
        52 => celestial(rng, item, properties, to_a, level, body),
        53 => {
            flags.insert(Stealth);
            if one_in(rng, 4) {
                flags.insert(Dexterity);
                if one_in(rng, 3) {
                    flags.insert(LessStrength);
                }
            }
            if body && level > 60 && one_in(rng, 7) {
                flags.insert(Speed);
            }
        }
        54 => {
            flags.insert(Stealth);
        }
        55 => {}
        56 => {
            flags.insert(Search);
            if one_in(rng, if base.tval == 33 { 3 } else { 7 }) {
                if one_in(rng, 2) {
                    add_esp_strong(rng, properties);
                } else {
                    add_esp_weak(rng, properties, false);
                }
            }
        }
        60 => {
            state.weight_tenths_pound = Some(item.weight_tenths_pound * 2 / 3);
            properties.modifiers.defense = 4;
            if one_in(rng, 4) {
                properties.passives.insert(Passive::SustainConstitution);
            }
        }
        61 => {
            flags.extend([Strength, LessIntelligence, LessStealth]);
        }
        62 | 63 => {}
        64 => {
            flags.insert(Constitution);
            if one_in(rng, 3) {
                properties.passives.insert(Passive::SustainConstitution);
            }
        }
        70 => {
            flags.insert(Strength);
            state.weight_tenths_pound = Some(item.weight_tenths_pound * 2 / 3);
            properties.modifiers.defense = 5;
            if level <= 40 {
                if one_in(rng, 4) {
                    flags.insert(LessDexterity);
                }
                if one_in(rng, 4) {
                    flags.insert(LessStealth);
                }
            } else if one_in(rng, 8) {
                flags.insert(Stealth);
                if one_in(rng, 1200 / level.min(60)) {
                    flags.insert(Speed);
                }
            }
            if level > 30 && one_in(rng, 300 / level.min(60)) {
                state.enchantment_delta.to_hit +=
                    (randint1(rng, 3) + rfb_m_bonus(rng, 4, level)) as i16;
                state.enchantment_delta.to_damage +=
                    (randint1(rng, 3) + rfb_m_bonus(rng, 3, level)) as i16;
            }
            if one_in(rng, 300 / level.clamp(20, 60)) {
                pval += 1;
            }
            if level > 60 && one_in(rng, 10) {
                activation = random_activation(rng, affix, level, false);
            }
            let mut rolls = 1;
            while randint1(rng, 150 + level) > 150 {
                rolls += 1;
            }
            for _ in 0..rolls {
                if one_in(rng, 4) {
                    flags.insert(Constitution);
                    if one_in(rng, 4) {
                        properties.passives.insert(Passive::SustainConstitution);
                    }
                }
                if one_in(rng, 4) {
                    properties.passives.insert(Passive::SustainStrength);
                }
                if one_in(rng, 2) {
                    properties.passives.insert(Passive::Regeneration);
                }
                if one_in(rng, 10) {
                    flags.insert(Search);
                }
                if one_in(rng, 3) {
                    add_resistance(properties, ActorDamageType::Dark);
                }
                if one_in(rng, 600 / level.clamp(10, 100)) {
                    add_resistance(properties, ActorDamageType::Disenchant);
                }
                if one_in(rng, 300 / level.clamp(20, 75)) {
                    add_status_immunity(properties, "rfb.status.blindness");
                }
                if one_in(rng, 300 / level.clamp(20, 75)) {
                    add_status_immunity(properties, "rfb.status.paralysis");
                }
                if one_in(rng, 1200 / level.clamp(20, 75)) {
                    properties.passives.insert(Passive::ReflectsBolts);
                }
                if one_in(rng, 800 / level.clamp(20, 75)) {
                    add_one_elemental_resistance(rng, properties);
                }
                if one_in(rng, 3200 / level.clamp(20, 75)) {
                    add_one_resistance(rng, properties);
                }
                if one_in(rng, 3200 / level.clamp(20, 75)) {
                    state.weight_tenths_pound = Some(item.weight_tenths_pound / 2);
                }
                if one_in(rng, 4) {
                    *to_a += (randint1(rng, 4) + rfb_m_bonus(rng, 4, level)) as i16;
                }
            }
        }
        71 | 72 => {
            flags.extend([Strength, LessIntelligence]);
            if index == 72 {
                if one_in(rng, 4) {
                    flags.insert(Constitution);
                }
                activation = fixed_activation_profile_index(affix);
            }
            if one_in(rng, 4) {
                flags.insert(LessStealth);
            }
        }
        73 | 74 => {
            flags.extend([Strength, Intelligence, LessWisdom]);
            if index == 73 {
                flags.insert(LessStealth);
            } else {
                flags.insert(Constitution);
                state
                    .curse_effects
                    .extend([ItemCurseEffectDto::Aggravate, ItemCurseEffectDto::TyCurse]);
            }
            if level > 66 && one_in(rng, 6) {
                flags.insert(Speed);
            }
            if one_in(rng, 5) {
                activation = random_activation(rng, affix, level, false);
            }
        }
        75 => {
            loop {
                for (flag, odds) in [
                    (Dexterity, 2),
                    (LessWisdom, 2),
                    (Intelligence, 7),
                    (LessStrength, 7),
                ] {
                    if one_in(rng, odds) {
                        flags.insert(flag);
                    }
                }
                if !flags.is_empty() {
                    if one_in(rng, 9) {
                        flags.insert(Stealth);
                    }
                    break;
                }
            }
            if level > 55 && one_in(rng, 5) {
                add_resistance(properties, ActorDamageType::Nether);
            }
            if (level > 36 || one_in(rng, 5)) && one_in(rng, 3) {
                flags.insert(Speed);
            }
            if one_in(rng, 10) {
                activation = random_activation(rng, affix, level, false);
            }
            if one_in(rng, 2) {
                state.enchantment_delta.to_hit += 2;
            }
        }
        76 => {
            flags.extend([
                Strength,
                Intelligence,
                Wisdom,
                Dexterity,
                Constitution,
                Charisma,
            ]);
        }
        77 => {
            let rolls = 1 + rfb_m_bonus(rng, 12, level);
            *to_a += rfb_m_bonus(rng, 32, level) as i16;
            if randint1(rng, 120) < level {
                state.enchantment_delta.to_hit += rfb_m_bonus(rng, 8, level) as i16;
                state.enchantment_delta.to_damage += rfb_m_bonus(rng, 8, level) as i16;
            }
            for _ in 0..rolls {
                if one_in(rng, 2) {
                    add_one_elemental_resistance(rng, properties);
                } else {
                    add_one_high_resistance(rng, properties);
                }
            }
            for _ in 0..1 + rfb_m_bonus(rng, 3, level) {
                add_one_low_esp(rng, properties);
            }
            for (passive, bound) in [
                (Passive::Regeneration, 150),
                (Passive::SlowDigestion, 150),
                (Passive::ReflectsBolts, 250),
            ] {
                if randint1(rng, bound) < level {
                    properties.passives.insert(passive);
                }
            }
            for flag in [
                Strength,
                Intelligence,
                Wisdom,
                Dexterity,
                Constitution,
                Charisma,
            ] {
                if randint1(rng, level) > 20 {
                    flags.insert(flag);
                }
            }
            for passive in [
                Passive::SustainStrength,
                Passive::SustainIntelligence,
                Passive::SustainWisdom,
                Passive::SustainDexterity,
                Passive::SustainConstitution,
                Passive::SustainCharisma,
            ] {
                if randint1(rng, level) > 40 {
                    properties.passives.insert(passive);
                }
            }
            if randint1(rng, level) > 40 {
                flags.insert(Stealth);
            }
            if randint1(rng, level) > 50 {
                properties.passives.insert(Passive::HoldLife);
            }
            if randint1(rng, level) > 50 {
                flags.insert(Speed);
            }
            if randint1(rng, level) > 60 {
                properties.passives.insert(Passive::Levitation);
            }
            if randint1(rng, level) > 80 {
                add_resistance(properties, ActorDamageType::Time);
            }
        }
        80 | 81 => {}
        82 => {
            flags.extend([Intelligence, LessConstitution, Capacity]);
            for _ in 0..3 {
                add_one_high_resistance(rng, properties);
            }
        }
        85 => {
            flags.insert(Intelligence);
            if one_in(rng, 3) {
                if one_in(rng, 2) {
                    add_esp_strong(rng, properties);
                } else {
                    add_esp_weak(rng, properties, false);
                }
            }
            if one_in(rng, 7) {
                flags.insert(Mastery);
            }
            if one_in(rng, 5) {
                properties.passives.insert(Passive::AutoIdentify);
            }
            if one_in(rng, 5) {
                activation = random_activation(rng, affix, level, true);
            }
        }
        86 => {
            flags.insert(Constitution);
            let suffix = format!("-{}", base.source_index);
            activation = affix.device_generation.as_ref().and_then(|generation| {
                generation
                    .activations
                    .iter()
                    .position(|profile| profile.id.ends_with(&suffix))
            });
        }
        87 => {
            flags.insert(Strength);
            state.enchantment_delta.to_hit += 3;
            state.enchantment_delta.to_damage += 3;
            if one_in(rng, 3) {
                add_status_immunity(properties, "rfb.status.fear");
            }
        }
        88 => {
            flags.extend([Wisdom, Mastery]);
        }
        89 => {
            flags.insert(Dexterity);
            *to_a += 5;
            if one_in(rng, 3) {
                *to_a += rfb_m_bonus(rng, 10, level) as i16;
            }
            while one_in(rng, 2) {
                add_one_sustain(rng, properties);
            }
            if one_in(rng, 7) {
                properties.passives.insert(Passive::ReflectsBolts);
            }
            if one_in(rng, 7) {
                properties.passives.insert(Passive::ShardsAura);
            }
        }
        90 => {
            flags.insert(Charisma);
        }
        91 => {
            flags.extend([Strength, Charisma]);
            if one_in(rng, 7) {
                flags.insert(Blows);
            }
        }
        92 => {
            flags.insert(Strength);
            if one_in(rng, 6) {
                properties.passives.insert(Passive::Vampiric);
            }
            if one_in(rng, 5) {
                activation = random_activation(rng, affix, level, true);
            }
        }
        95 => {
            flags.extend([Speed, LessCharisma]);
            activation = fixed_activation_profile_index(affix);
        }
        96..=98 => {
            if one_in(rng, 5) {
                activation = random_activation(rng, affix, level, false);
            }
        }
        99 => {
            for (passive, odds) in [
                (Passive::FireAura, 2),
                (Passive::ColdAura, 2),
                (Passive::ElectricityAura, 2),
                (Passive::ShardsAura, 7),
            ] {
                if one_in(rng, odds) {
                    properties.passives.insert(passive);
                }
            }
        }
        100 => {
            flags.insert(Stealth);
            if one_in(rng, 3) {
                add_vulnerability(properties, ActorDamageType::Light);
            }
        }
        101 => {
            flags.extend([Stealth, Speed]);
        }
        102 => {
            flags.extend([Speed, Infra, Stealth]);
            state.enchantment_delta.to_hit -= 6;
            state.enchantment_delta.to_damage -= 6;
            if one_in(rng, 6) {
                properties.equipment_bonuses.light_radius -= 1;
            }
            if one_in(rng, 3) {
                add_vulnerability(properties, ActorDamageType::Light);
                add_one_high_resistance(rng, properties);
                if one_in(rng, 3) {
                    flags.insert(LessStrength);
                }
            }
            if one_in(rng, 12) {
                properties.passives.insert(Passive::NightVision);
            }
        }
        103 => {
            flags.extend([Stealth, Speed, LessWisdom, LessLife]);
            state.enchantment_delta.to_hit += 6;
            state.enchantment_delta.to_damage += 6;
            curse = Some(if one_in(rng, 6) {
                ItemCurseSeverityDto::Permanent
            } else {
                ItemCurseSeverityDto::Heavy
            });
            if one_in(rng, 66) {
                properties
                    .resistances
                    .insert(ActorDamageType::Cold, ActorResistanceLevel::Immune);
            }
            while one_in(rng, 6) {
                add_one_high_resistance(rng, properties);
            }
            if one_in(rng, 5) {
                activation = random_activation(rng, affix, level, false);
            }
        }
        104 => {
            flags.insert(Strength);
            state.enchantment_delta.to_hit += 3;
            state.enchantment_delta.to_damage += 3;
            for (passive, odds) in [
                (Passive::SustainDexterity, 2),
                (Passive::Regeneration, 3),
                (Passive::SeeInvisible, 5),
            ] {
                if one_in(rng, odds) {
                    properties.passives.insert(passive);
                }
            }
            for (flag, odds) in [(Dexterity, 5), (Constitution, 5), (Life, 10), (Speed, 10)] {
                if one_in(rng, odds) {
                    flags.insert(flag);
                }
            }
            if one_in(rng, 10) {
                properties.passives.insert(Passive::Levitation);
            }
            if one_in(rng, 20) {
                flags.insert(Charisma);
            } else if one_in(rng, 5) {
                flags.insert(LessStealth);
            }
            if one_in(rng, 12) {
                add_one_high_resistance(rng, properties);
            }
            if one_in(rng, 3) && one_in(rng, 5) {
                activation = random_activation(rng, affix, level, false);
            }
        }
        110 | 111 => {
            flags.insert(if index == 110 { Intelligence } else { Wisdom });
            if one_in(rng, 7) {
                flags.insert(if index == 110 { Mastery } else { Capacity });
            }
            if one_in(rng, 5) {
                activation = random_activation(rng, affix, level, true);
            }
        }
        112 => {
            flags.insert(Charisma);
        }
        113 => {
            flags.insert(Constitution);
        }
        114 => {
            flags.extend([Strength, LessCharisma]);
        }
        115 => {
            flags.extend([Strength, LessIntelligence]);
        }
        116 => {
            flags.extend([Strength, Charisma, Stealth, LessWisdom]);
            for element in [ActorDamageType::Cold, ActorDamageType::Nether] {
                if one_in(rng, 2) {
                    add_resistance(properties, element);
                }
            }
            if one_in(rng, 3) {
                add_vulnerability(properties, ActorDamageType::Light);
            }
            if one_in(rng, 6) {
                properties.passives.insert(Passive::Vampiric);
            }
        }
        117 => {
            if one_in(rng, 3) {
                add_vulnerability(properties, ActorDamageType::Dark);
            }
            if one_in(rng, 5) {
                activation = random_activation(rng, affix, level, true);
            }
        }
        118 => {
            flags.insert(Infra);
            state.weight_tenths_pound = Some(item.weight_tenths_pound * 2 / 3);
            properties.modifiers.defense = 3;
            if one_in(rng, 4) {
                flags.insert(Digging);
            }
        }
        119 => {
            flags.extend([Speed, Strength, Charisma]);
            activation = fixed_activation_profile_index(affix);
        }
        120 => {
            flags.extend([Strength, LessIntelligence, LessWisdom]);
            if one_in(rng, 6) {
                flags.insert(LessStealth);
            }
            if one_in(rng, 3) {
                add_vulnerability(properties, ActorDamageType::Confusion);
            }
            state.enchantment_delta.to_damage += 3 + rfb_m_bonus(rng, 7, level) as i16;
            activation = fixed_activation_profile_index(affix);
        }
        121 => {
            flags.insert(Stealth);
            state.weight_tenths_pound = Some(8);
            if one_in(rng, 2) && rng.bounded(75) < u64::from(level) {
                flags.insert(Speed);
            }
            if one_in(rng, 2) && rng.bounded(42) < u64::from(level) {
                add_resistance(properties, ActorDamageType::Cold);
            }
            if one_in(rng, 4) && rng.bounded(75) < u64::from(level) {
                flags.insert(Dexterity);
                if one_in(rng, 2) {
                    properties.passives.insert(Passive::SustainDexterity);
                }
            }
        }
        122 => {
            flags.insert(Intelligence);
            if witch(rng, properties, &mut flags, level, affix, &mut activation) {
                use rfb_content::ItemDestructionElement::*;
                state
                    .elemental_destruction_immunities
                    .extend([Acid, Electricity, Fire, Cold]);
            }
        }
        125 => {
            add_esp_strong(rng, properties);
            let extra = properties.passives.contains(&Passive::EspNonliving);
            add_esp_weak(rng, properties, extra);
        }
        126 => {
            flags.insert(Intelligence);
            if one_in(rng, 3) {
                add_one_high_resistance(rng, properties);
            } else {
                for _ in 0..4 {
                    add_one_elemental_resistance(rng, properties);
                }
            }
            if one_in(rng, 7) {
                properties.passives.insert(Passive::EasySpell);
            }
            if one_in(rng, 3) {
                flags.insert(LessStrength);
            }
            if one_in(rng, 30) {
                flags.extend([SpellPower, LessConstitution]);
            } else if one_in(rng, 3) {
                state.enchantment_delta.to_damage += 4 + randint1(rng, 11) as i16;
                while one_in(rng, 2) {
                    state.enchantment_delta.to_damage += 1;
                }
            } else if base.tval == 33 && level > 70 && one_in(rng, 30) {
                properties.passives.insert(Passive::ManaRegeneration);
            }
            if level > 70 && one_in(rng, 10) {
                flags.insert(Speed);
            }
            if one_in(rng, 5) {
                activation = random_activation(rng, affix, level, false);
            }
        }
        127 => {
            flags.extend([Strength, Dexterity, Constitution, LessIntelligence]);
            if one_in(rng, 5) {
                state.enchantment_delta.to_hit += randint1(rng, 7) as i16;
                state.enchantment_delta.to_damage += randint1(rng, 7) as i16;
            }
            if one_in(rng, 3) {
                add_status_immunity(properties, "rfb.status.fear");
            } else {
                add_one_high_resistance(rng, properties);
            }
            if level > 70 && one_in(rng, 10) {
                flags.insert(Speed);
            }
            if one_in(rng, 5) {
                activation = random_activation(rng, affix, level, false);
            }
        }
        128 => {
            flags.extend([Wisdom, Charisma]);
            if one_in(rng, 5) {
                flags.insert(Capacity);
            }
            for _ in 0..2 {
                if one_in(rng, 5) {
                    add_one_high_resistance(rng, properties);
                }
            }
            if level > 70 && one_in(rng, 5) {
                flags.insert(Speed);
            }
            if one_in(rng, 5) {
                activation = random_activation(rng, affix, level, false);
            }
        }
        129 => {
            flags.extend([Stealth, LessConstitution]);
            state.curse_effects.insert(ItemCurseEffectDto::TyCurse);
        }
        130 => {
            flags.extend([
                Strength,
                Constitution,
                LessIntelligence,
                LessWisdom,
                MagicResistance,
            ]);
        }
        135 => {
            if one_in(rng, 4) {
                gloves_slaying(rng, properties, level);
            }
        }
        136 => {
            flags.extend([Dexterity, Stealth]);
            if one_in(rng, 20) {
                flags.insert(Speed);
            }
        }
        137 => {
            flags.extend([Strength, Constitution, LessIntelligence]);
            if one_in(rng, 4) {
                let element = [
                    ActorDamageType::Sound,
                    ActorDamageType::Shards,
                    ActorDamageType::Chaos,
                ][rng.bounded(3) as usize];
                add_resistance(properties, element);
            }
            if one_in(rng, 3) {
                add_vulnerability(properties, ActorDamageType::Confusion);
            }
            if one_in(rng, 2) {
                flags.insert(LessStealth);
            }
            if one_in(rng, 2) {
                flags.insert(LessDexterity);
            }
        }
        138 => {
            flags.extend([Intelligence, Mastery]);
            if one_in(rng, 4) {
                match rng.bounded(3) {
                    0 => add_resistance(properties, ActorDamageType::Confusion),
                    1 => add_status_immunity(properties, "rfb.status.blindness"),
                    _ => add_resistance(properties, ActorDamageType::Light),
                }
            }
            if one_in(rng, 2) {
                flags.insert(LessStrength);
            }
            if one_in(rng, 3) {
                flags.insert(LessConstitution);
            }
            if one_in(rng, 30) {
                flags.insert(DevicePower);
            }
        }
        139 => {
            flags.extend([
                LessStrength,
                LessDexterity,
                LessConstitution,
                LessCharisma,
                Stealth,
            ]);
            if one_in(rng, 10) {
                properties
                    .resistances
                    .insert(ActorDamageType::Acid, ActorResistanceLevel::Immune);
            }
        }
        140 => {
            flags.insert(Dexterity);
        }
        141 => {
            flags.extend([Might, Stealth]);
            state.enchantment_delta.to_hit = 5 + randint1(rng, 10) as i16;
        }
        142 => {
            flags.extend([Blows, LessStealth, LessIntelligence]);
            state.enchantment_delta.to_hit = -10;
            state.enchantment_delta.to_damage = 10;
            *to_a = -10;
            activation = fixed_activation_profile_index(affix);
        }
        145 | 151 => {
            if index == 151 {
                flags.insert(Speed);
            }
            if one_in(rng, 2) {
                add_one_high_resistance(rng, properties);
            }
        }
        146 => {
            activation = fixed_activation_profile_index(affix);
        }
        147 => {
            flags.extend([Constitution, LessStealth]);
            state.weight_tenths_pound = Some(item.weight_tenths_pound * 2 / 3);
            properties.modifiers.defense = 4;
            if one_in(rng, 4) {
                properties.passives.insert(Passive::SustainConstitution);
            }
        }
        148 => {
            flags.insert(Speed);
            let amount = 3 + level.min(90).saturating_sub(30) / 10;
            pval = 1 + rfb_m_bonus(rng, amount, level);
        }
        149 => {
            flags.extend([Stealth, Speed]);
            if one_in(rng, 2) {
                add_one_high_resistance(rng, properties);
            }
            if one_in(rng, 2) {
                properties.passives.insert(Passive::Levitation);
            }
        }
        150 => {
            flags.insert(Speed);
            pval = 6 + rfb_m_bonus(rng, 9, level);
            activation = fixed_activation_profile_index(affix);
        }
        152 => {
            flags.extend([LessSpeed, Strength, Constitution, LessDexterity, Life]);
        }
        _ => return None,
    }
    if index == 126 {
        add_one_ability(rng, properties);
    }
    if index == 103 {
        state.curse_effects.extend([
            ItemCurseEffectDto::DrainExperience,
            roll_rfb_heavy_curse_effect(rng),
        ]);
    }
    if matches!(index, 53 | 71..=75 | 80 | 101 | 103 | 126 | 128 | 130 | 152) {
        add_one_high_resistance(rng, properties);
        if randint1(rng, level) > 60 {
            add_one_high_resistance(rng, properties);
        }
    }
    if index == 61 {
        add_one_elemental_resistance(rng, properties);
    }
    let (hit, damage, armor, maximum_pval) = maxima(index);
    state.enchantment_delta.to_hit += roll_signed(rng, hit);
    state.enchantment_delta.to_damage += roll_signed(rng, damage);
    *to_a += roll_signed(rng, armor);
    if index == 142 {
        pval = randint1(rng, 2);
        if one_in(rng, 15) {
            pval += 1;
        }
    } else if index == 121 && flags.contains(&Speed) {
        pval = rng.bounded(3) as u16;
        loop {
            pval += 1;
            if !one_in(rng, (100_u16.saturating_sub(level) / 6).max(7)) {
                break;
            }
        }
    } else if matches!(index, 95 | 101 | 102) {
        pval = randint1(rng, maximum_pval);
        if base.sval == 2 {
            pval += rng.bounded(2) as u16;
        }
    } else if maximum_pval > 0 {
        pval += randint1(rng, maximum_pval);
    }
    if index == 104 {
        pval = pval.min(3);
    }
    if index == 101 && level > 80 {
        while one_in(rng, 4) {
            pval += 1;
        }
    }
    if matches!(index, 126..=128) && level > 80 && one_in(rng, 5) {
        pval += 1;
    }
    if flags.contains(&DevicePower) && pval >= 3 {
        pval = 2;
        if one_in(rng, 30) {
            pval += 1;
        }
    }
    if index == 149 && level > 70 {
        *to_a += randint1(rng, 5) as i16;
        while one_in(rng, 3) {
            pval += 1;
        }
    }
    for flag in flags {
        apply_pval(properties, flag, i32::from(pval));
    }
    properties.status_immunities.sort();
    let profile = activation
        .and_then(|i| affix.device_generation.as_ref()?.activations.get(i))
        .or_else(|| {
            (base.tval == 38)
                .then(|| item.device_generation.as_ref()?.activations.first())
                .flatten()
        });
    let (activation, charges) = profile.map(materialize_rfb_activation).unzip();
    let rolled = state
        .has_instance_state()
        .then_some(state)
        .into_iter()
        .collect();
    let mut result = EgoMaterialization::new(
        vec![affix.id.clone()],
        rolled,
        None,
        curse,
        activation,
        charges,
    );
    if index == 81 {
        result.kind_id_override = Some("demo.item.yoiyami-robe".to_owned());
        result.clear_armor_enchantment = true;
    }
    if index == 141 {
        result.clear_hit_enchantment = true;
    }
    if index == 142 {
        result.clear_hit_enchantment = true;
        result.clear_damage_enchantment = true;
        result.clear_armor_enchantment = true;
    }
    Some(result)
}

fn gloves_slaying(rng: &mut RfbRng, properties: &mut AffixPropertyBundleDefinition, level: u16) {
    let candidates: Vec<_> = SLAYS
        .iter()
        .filter(|(_, _, _, maximum)| *maximum == 0 || level <= *maximum)
        .collect();
    let total: u64 = candidates
        .iter()
        .map(|(_, _, rarity, _)| u64::from(255 / rarity))
        .sum();
    let mut rolls = 1 + rfb_m_bonus(rng, 4, level);
    if one_in(rng, 8) {
        rolls *= 2;
    }
    for _ in 0..rolls {
        let mut choice = rng.bounded(total);
        for (target, _, rarity, _) in &candidates {
            let weight = u64::from(255 / rarity);
            if choice < weight {
                add_slay(properties, *target, SlayLevel::Slay);
                break;
            }
            choice -= weight;
        }
    }
}

fn add_vulnerability(properties: &mut AffixPropertyBundleDefinition, element: ActorDamageType) {
    properties
        .resistances
        .insert(element, ActorResistanceLevel::Vulnerable);
}

fn witch(
    rng: &mut RfbRng,
    properties: &mut AffixPropertyBundleDefinition,
    flags: &mut BTreeSet<Pval>,
    level: u16,
    affix: &AffixDefinition,
    activation: &mut Option<usize>,
) -> bool {
    use EquipmentPassive as P;
    use Pval::*;
    let mut strength = i32::from(level.min(100));
    let mut ignores_elements = false;
    for i in 0..1 + rfb_m_bonus(rng, 5, level) {
        if one_in(rng, 10) && !properties.resistances.contains_key(&ActorDamageType::Dark) {
            add_resistance(properties, ActorDamageType::Dark);
            strength -= 2;
        } else if one_in(rng, 12)
            && !properties
                .resistances
                .contains_key(&ActorDamageType::Nether)
        {
            add_resistance(properties, ActorDamageType::Nether);
            strength -= 2;
        } else if strength > 25
            && rng.bounded((192 - strength) as u64) < 7
            && !properties.resistances.contains_key(&ActorDamageType::Chaos)
        {
            add_resistance(properties, ActorDamageType::Chaos);
            strength -= 2;
        }
        if strength > 25
            && !flags.contains(&Mastery)
            && rng.bounded((172 - strength) as u64) < u64::from(i)
        {
            flags.insert(Mastery);
            strength -= 4;
        } else if strength > 25
            && !properties.passives.contains(&P::EasySpell)
            && rng.bounded((256 - strength) as u64) < u64::from(i)
        {
            properties.passives.insert(P::EasySpell);
            if one_in(rng, 3) {
                properties.passives.insert(P::ReducedManaCost);
            }
            strength -= 4;
        } else if strength > 25
            && !flags.contains(&Capacity)
            && rng.bounded((384 - strength) as u64) < u64::from(i)
        {
            flags.insert(Capacity);
            strength -= 4;
        } else if strength > 50
            && !properties.passives.contains(&P::ManaRegeneration)
            && rng.bounded((512 - strength) as u64) < u64::from(i)
        {
            properties.passives.insert(P::ManaRegeneration);
            strength -= 4;
        }
        if one_in(rng, 22) {
            add_one_resistance(rng, properties);
            strength -= 2;
        }
        if !properties.passives.contains(&P::SustainIntelligence) && one_in(rng, 10) {
            properties.passives.insert(P::SustainIntelligence);
            strength -= 2;
        }
        if !flags.contains(&Charisma) && one_in(rng, 20) {
            flags.insert(Charisma);
            strength -= 2;
        }
        for (passive, odds) in [
            (P::Levitation, 20),
            (P::AutoIdentify, 100),
            (P::SeeInvisible, 16),
        ] {
            if !properties.passives.contains(&passive) && one_in(rng, odds) {
                properties.passives.insert(passive);
                strength -= 2;
            }
        }
        if one_in(rng, 12) {
            add_one_low_esp(rng, properties);
            strength -= 2;
        }
        if !ignores_elements && one_in(rng, 4) {
            ignores_elements = true;
            strength -= 2;
        }
        if i == 0 {
            if one_in(rng, 8) {
                flags.insert(LessWisdom);
            }
            if one_in(rng, 8) {
                add_vulnerability(properties, ActorDamageType::Light);
            }
            if one_in(rng, 5) {
                *activation = random_activation(rng, affix, level, false);
            }
        }
    }
    ignores_elements
}

pub(super) fn random_activation(
    rng: &mut RfbRng,
    affix: &AffixDefinition,
    level: u16,
    list: bool,
) -> Option<usize> {
    let generation = affix.device_generation.as_ref()?;
    let candidates: Vec<_> = generation
        .activations
        .iter()
        .enumerate()
        .filter(|(_, profile)| list || (profile.min_depth <= level && level <= profile.max_depth))
        .collect();
    let total: u64 = candidates
        .iter()
        .map(|(_, profile)| u64::from(profile.weight))
        .sum();
    if total == 0 {
        return None;
    }
    let mut choice = rng.bounded(total);
    candidates.into_iter().find_map(|(index, profile)| {
        if choice < u64::from(profile.weight) {
            Some(index)
        } else {
            choice -= u64::from(profile.weight);
            None
        }
    })
}

pub(super) fn apply_pval(properties: &mut AffixPropertyBundleDefinition, flag: Pval, value: i32) {
    use Pval::*;
    match flag {
        Strength => properties.modifiers.strength += value,
        Intelligence => properties.modifiers.intelligence += value,
        Wisdom => properties.modifiers.wisdom += value,
        Dexterity => properties.modifiers.dexterity += value,
        Constitution => properties.modifiers.constitution += value,
        Charisma => properties.modifiers.charisma += value,
        LessStrength => properties.modifiers.strength -= value,
        LessIntelligence => properties.modifiers.intelligence -= value,
        LessWisdom => properties.modifiers.wisdom -= value,
        LessDexterity => properties.modifiers.dexterity -= value,
        LessConstitution => properties.modifiers.constitution -= value,
        LessCharisma => properties.modifiers.charisma -= value,
        Speed => properties.modifiers.speed += value,
        LessSpeed => properties.modifiers.speed -= value,
        Life => properties.equipment_bonuses.life_percent += 3 * value,
        LessLife => properties.equipment_bonuses.life_percent -= 3 * value,
        Infra => properties.equipment_bonuses.infravision += value,
        Digging => properties.equipment_bonuses.digging_skill += 20 * value,
        SpellPower => properties.modifiers.spell_power_bonus += value,
        DevicePower => properties.modifiers.device_power_bonus += value,
        MagicResistance => properties.equipment_bonuses.magic_resistance_percent += 5 * value,
        Might => {
            properties
                .equipment_bonuses
                .launcher_multiplier_delta_percent += 20 * value
        }
        Stealth => properties.equipment_bonuses.stealth_skill += value,
        LessStealth => properties.equipment_bonuses.stealth_skill -= value,
        Search => {
            properties.equipment_bonuses.search_skill += 5 * value;
            properties.equipment_bonuses.perception_skill += 5 * value;
        }
        Mastery => properties.equipment_bonuses.device_skill += 8 * value,
        Capacity => properties.equipment_bonuses.spell_capacity_bonus += value,
        Blows => properties.equipment_bonuses.melee_attacks_delta_percent += 50 * value,
        Shots => properties.equipment_bonuses.base_shot_delta_percent += 15 * value,
        WeaponMastery => properties.equipment_bonuses.weapon_dice_bonus += value,
    }
}

fn maxima(index: u32) -> (i16, i16, i16, u16) {
    match index {
        50 | 52 | 80 => (0, 0, 10, 0),
        51 => (0, 0, 8, 0),
        53 => (0, 0, 10, 3),
        54 => (0, 0, 0, 3),
        55 | 145 | 146 | 148 | 150 => (0, 0, 0, 0),
        147 => (0, 0, 10, 3),
        149 | 151 => (0, 0, 0, 3),
        152 => (8, 8, 15, 3),
        60 => (5, 5, 12, 0),
        61 => (3, 6, 10, 3),
        62 | 63 => (0, 0, 5, 0),
        64 => (0, 0, 0, 5),
        70 => (0, 0, 15, 3),
        71 => (5, 5, 5, 3),
        72 | 73 => (0, 7, 10, 4),
        74 => (0, 15, 15, 5),
        75 => (-5, -5, 5, 3),
        76 => (0, 0, 15, 4),
        77 => (0, 0, 0, 4),
        81 => (0, 0, 0, 0),
        82 => (-25, -25, 0, 5),
        87 => (5, 5, 0, 3),
        89 => (0, 0, 5, 3),
        91 => (5, 5, 0, 1),
        85 | 86 | 88 | 90 | 92 => (0, 0, 0, 3),
        56 => (0, 0, 0, 6),
        95 => (-10, -10, -10, 3),
        96 => (0, 0, 0, 0),
        97 => (0, 0, 0, 0),
        98 => (0, 0, 0, 0),
        99 => (0, 0, -20, 0),
        100 => (0, 0, 0, 7),
        101 => (0, 0, 10, 3),
        102 => (-7, -7, -5, 5),
        103 => (6, 6, 6, 3),
        104 => (4, 4, 2, 2),
        110 => (0, 0, 0, 3),
        111 => (0, 0, 0, 3),
        112 => (0, 0, 0, 3),
        113 => (0, 0, 0, 5),
        114 => (0, 5, 0, 2),
        115 => (0, 8, 5, 2),
        116 => (0, 0, 0, 3),
        117 => (0, 0, 0, 0),
        118 => (0, 0, 12, 3),
        119 => (5, 5, 0, 2),
        120 => (-10, 10, -10, 3),
        121 => (8, -8, 3, 5),
        122 => (0, 0, 0, 3),
        125 => (0, 0, 0, 0),
        126 => (0, 0, 0, 3),
        127 => (0, 0, 0, 3),
        128 => (0, 0, 0, 3),
        129 => (10, 10, 0, 3),
        130 => (0, 0, 0, 3),
        135 => (8, 8, 0, 0),
        136 => (5, -5, 0, 4),
        137 => (-5, 10, 0, 4),
        138 => (-10, -10, -20, 3),
        139 => (-10, -10, 0, 7),
        140 => (8, 0, 0, 3),
        141 => (0, 5, 0, 3),
        142 => (-15, 8, -15, 3),
        _ => unreachable!("validated armor ego"),
    }
}

fn celestial(
    rng: &mut RfbRng,
    item: &ItemDefinition,
    properties: &mut AffixPropertyBundleDefinition,
    armor: &mut i16,
    level: u16,
    body: bool,
) {
    use EquipmentPassive as P;
    let rolls = (if body { 2 } else { 1 }) + rfb_m_bonus(rng, 3, level);
    for _ in 0..rolls {
        add_one_high_resistance(rng, properties);
    }
    if one_in(rng, 7) {
        properties.passives.insert(P::HoldLife);
    }
    if !body {
        if one_in(rng, 5) {
            properties.passives.insert(P::ReflectsBolts);
        }
        return;
    }
    while one_in(rng, 3) {
        *armor += randint1(rng, 3) as i16;
    }
    if one_in(rng, 3) {
        add_status_immunity(properties, "rfb.status.paralysis");
    }
    if one_in(rng, 3) {
        properties.passives.insert(P::SlowDigestion);
    }
    if one_in(rng, 17) {
        properties.passives.insert(P::ReflectsBolts);
    }
    let bound = if rolls == 4 { 242 } else { 160 };
    if rolls < 4
        || !one_in(rng, 2)
        || randint1(rng, bound) >= level
        || randint1(rng, bound) >= level
        || randint1(rng, item.modifiers.defense.max(1) as u16) >= 16
    {
        return;
    }
    add_one_high_resistance(rng, properties);
    if one_in(rng, 7) {
        properties.passives.insert(P::HoldLife);
    }
    if one_in(rng, 7) {
        properties.passives.insert(P::Warning);
    }
    if one_in(rng, 3) {
        add_one_sustain(rng, properties);
    }
    if one_in(rng, 15) {
        properties.passives.insert(P::ReflectsBolts);
    }
    if one_in(rng, 99) {
        add_resistance(properties, ActorDamageType::Time);
    }
    if one_in(rng, if rolls > 4 { 10 } else { 2 }) {
        return;
    }
    match rng.bounded(6) {
        0 => add_one_high_resistance(rng, properties),
        1 => add_one_elemental_resistance(rng, properties),
        2 => add_one_sustain(rng, properties),
        3 => {
            if !properties
                .status_immunities
                .iter()
                .any(|id| id == "rfb.status.paralysis")
                && !item
                    .status_immunities
                    .iter()
                    .any(|id| id == "rfb.status.paralysis")
            {
                add_status_immunity(properties, "rfb.status.paralysis");
            } else if !properties.passives.contains(&P::HoldLife)
                && !item.passives.contains(&P::HoldLife)
            {
                properties.passives.insert(P::HoldLife);
            } else if !properties.passives.contains(&P::ReflectsBolts) && !item.reflects_bolts {
                properties.passives.insert(P::ReflectsBolts);
            } else {
                properties.passives.insert(P::Blessed);
                if one_in(rng, 2) {
                    properties.passives.insert(P::EspEvil);
                }
            }
        }
        _ => {
            let mut remaining = 6;
            loop {
                *armor += 4;
                remaining -= 1;
                if !one_in(rng, 2) || remaining == 0 {
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{game::Game, resistance::DamageType, state::ItemLocation};

    fn base_id(index: u32) -> &'static str {
        match index {
            54 | 55 | 145..=152 => "demo.item.pair-of-metal-shod-boots",
            56 | 110..=120 => "demo.item.iron-helm",
            95..=104 => "demo.item.cloak",
            121 | 122 => "demo.item.pointy-hat",
            125..=130 => "demo.item.iron-crown",
            135..=142 => "demo.item.leather-gloves",
            50..=64 => "demo.item.small-metal-shield",
            70..=74 | 76 => "demo.item.chain-mail",
            75 => "demo.item.soft-leather-armour",
            77 => "demo.item.filthy-rag",
            80..=82 => "demo.item.robe",
            91 => "demo.item.gold-dragon-scale-mail",
            92 => "demo.item.chaos-dragon-scale-mail",
            _ => "demo.item.multi-hued-dragon-scale-mail",
        }
    }

    #[test]
    fn boots_pool_generates_all_eleven_egos_and_preserves_instance_state() {
        let game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        let definition = game
            .content
            .item("demo.item.pair-of-metal-shod-boots")
            .unwrap();
        let mut seen = BTreeSet::new();
        for seed in 1..=12_000 {
            let result = roll_and_materialize_rfb_ego_from_affixes_with_rng(
                rfb_protocol::ItemEnchantmentsDto::default(),
                &mut RfbRng::seeded(seed),
                definition,
                game.content.affix_definitions(),
                100,
                None,
            )
            .unwrap();
            let index = game
                .content
                .affix(&result.affix_ids[0])
                .unwrap()
                .rfb_ego
                .as_ref()
                .unwrap()
                .source_index;
            if !seen.insert(index) {
                continue;
            }
            let mut item = item_for(&game, &definition.id);
            result.apply_to(&mut item);
            assert!(crate::game::validation::rolled_affixes_are_valid(&item));
            let dto = crate::save::inventory_to_save(std::slice::from_ref(&item)).remove(0);
            assert_eq!(
                crate::save::inventory_item_from_dto(dto, &game.content).unwrap(),
                item
            );
            match index {
                146 | 150 => {
                    assert!(item.activation.is_some());
                    assert_eq!(item.charges.as_ref().unwrap().current, 1);
                    if index == 150 {
                        assert!(
                            (6..=15).contains(&item.rolled_affixes[0].properties.modifiers.speed)
                        );
                    }
                }
                147 => assert_eq!(
                    game.item_instance_weight(&item),
                    definition.weight_tenths_pound * 2 / 3
                ),
                148 => {
                    assert!((1..=10).contains(&item.rolled_affixes[0].properties.modifiers.speed))
                }
                152 => {
                    let properties = &item.rolled_affixes[0].properties;
                    assert!(properties.modifiers.speed < 0);
                    assert!(properties.modifiers.dexterity < 0);
                    assert_eq!(
                        properties.equipment_bonuses.life_percent,
                        3 * properties.modifiers.strength
                    );
                }
                _ => {}
            }
        }
        assert_eq!(seen, [50, 54, 55].into_iter().chain(145..=152).collect());
        assert!(!can_apply(147, 30, 2));
        assert!(can_apply(147, 30, 5));
        assert!(can_apply(147, 30, 6));
    }

    fn item_for(game: &Game, kind_id: &str) -> ItemInstance {
        let mut item = game.items[0].clone();
        item.kind_id = kind_id.to_owned();
        item.quantity = 1;
        item.affix_ids.clear();
        item.rolled_affixes.clear();
        item.intrinsic_properties = Default::default();
        item.enchantments = Default::default();
        item.activation = None;
        item.charges = None;
        item.fuel = None;
        item.location = ItemLocation::Inventory;
        item
    }

    fn equip_ego(
        game: &mut Game,
        source_index: u32,
        predicate: impl Fn(&EgoMaterialization) -> bool,
    ) -> usize {
        let definition = game.content.item(base_id(source_index)).unwrap();
        let affix = game
            .content
            .affix_definitions()
            .find(|affix| {
                affix
                    .rfb_ego
                    .as_ref()
                    .is_some_and(|ego| ego.source_index == source_index)
            })
            .unwrap();
        let result = (1..10_000)
            .find_map(|seed| {
                materialize(&mut RfbRng::seeded(seed), definition, affix, 90, None)
                    .filter(&predicate)
            })
            .expect("requested generated armor property");
        let mut item = item_for(game, &definition.id);
        item.id = format!("test.armor.{source_index}");
        item.quality = rfb_protocol::ItemQualityDto::Exceptional;
        let slot_id = game
            .body_slots
            .iter()
            .find(|slot| Some(&slot.slot_type) == definition.equipment_slot.as_ref())
            .unwrap()
            .id
            .clone();
        game.items.retain(|item| !matches!(&item.location, ItemLocation::Equipped { slot_id: occupied } if occupied == &slot_id));
        item.location = ItemLocation::Equipped { slot_id };
        result.apply_to(&mut item);
        game.items.push(item);
        game.items.len() - 1
    }

    #[test]
    fn complete_front_armor_pools_are_reachable_through_natural_selection() {
        let game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        let mut seen = BTreeSet::new();
        for id in [
            "small-metal-shield",
            "large-metal-shield",
            "chain-mail",
            "soft-leather-armour",
            "filthy-rag",
            "robe",
            "multi-hued-dragon-scale-mail",
            "law-dragon-scale-mail",
            "gold-dragon-scale-mail",
            "chaos-dragon-scale-mail",
        ] {
            let definition = game.content.item(&format!("demo.item.{id}")).unwrap();
            for seed in 1..=3000 {
                let result = roll_and_materialize_rfb_ego_from_affixes_with_rng(
                    rfb_protocol::ItemEnchantmentsDto::default(),
                    &mut RfbRng::seeded(seed),
                    definition,
                    game.content.affix_definitions(),
                    90,
                    None,
                )
                .unwrap();
                let affix = game.content.affix(&result.affix_ids[0]).unwrap();
                seen.insert(affix.rfb_ego.as_ref().unwrap().source_index);
            }
        }
        let expected: BTreeSet<_> = (50..=53)
            .chain(60..=64)
            .chain(70..=77)
            .chain(80..=82)
            .chain(85..=92)
            .collect();
        assert_eq!(seen, expected);
    }

    #[test]
    fn armor_passives_and_fractional_blows_reach_equipped_consumers() {
        let mut game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        let reflection = equip_ego(&mut game, 62, |_| true);
        assert!(game.player_reflects_bolts());
        game.items[reflection].location = ItemLocation::Inventory;
        assert!(!game.player_reflects_bolts());
        equip_ego(&mut game, 74, |_| true);
        assert_eq!(
            game.player_elemental_contact_aura_sources(DamageType::Fire)
                .len(),
            1
        );
        equip_ego(&mut game, 89, |result| {
            result.rolled_affixes.iter().any(|rolled| {
                rolled
                    .properties
                    .passives
                    .contains(&EquipmentPassive::ShardsAura)
            })
        });
        assert_eq!(
            game.player_elemental_contact_aura_sources(DamageType::Shards)
                .len(),
            1
        );
        equip_ego(&mut game, 85, |result| {
            result.rolled_affixes.iter().any(|rolled| {
                rolled
                    .properties
                    .passives
                    .contains(&EquipmentPassive::AutoIdentify)
            })
        });
        assert!(game.player_auto_identifies_items());
        let index = equip_ego(&mut game, 91, |result| {
            result.rolled_affixes.iter().any(|rolled| {
                rolled
                    .properties
                    .equipment_bonuses
                    .melee_attacks_delta_percent
                    % 100
                    == 50
            })
        });
        let bonus = game
            .item_equipment_bonuses(&game.items[index])
            .melee_attacks_delta_percent;
        let stats = game.player_derived_stats();
        let profile = game.player_melee_profile(&stats);
        assert_eq!(
            profile.attacks,
            (stats.melee_attacks.value + bonus / 100) as u16
        );
        assert_eq!(profile.extra_attack_chance_percent, 50);
    }

    #[test]
    fn sorcerer_robe_changes_spell_cost_capacity_and_failure() {
        let mut game = Game::new_with_build(7, "demo.build.high-mage-sorcery").unwrap();
        game.progress.level = 30;
        let index = equip_ego(&mut game, 82, |_| true);
        let ability = game
            .content
            .abilities()
            .find(|ability| {
                ability
                    .player
                    .as_ref()
                    .is_some_and(|player| player.resource_cost >= 8)
            })
            .unwrap()
            .clone();
        let progress = game.ability_progress_value(&ability);
        let discounted = game.ability_effective_resource_cost(&ability, progress);
        let fail_with = game.ability_failure_percent(game.casting_profile().unwrap(), &ability);
        let capacity = game.player_ability_baseline().0;
        // Keep the robe's attribute and weight changes; isolate its casting flags.
        game.items[index].affix_ids.clear();
        let capacity_bonus = game.items[index].rolled_affixes[0]
            .properties
            .equipment_bonuses
            .spell_capacity_bonus;
        game.items[index].rolled_affixes[0]
            .properties
            .equipment_bonuses
            .spell_capacity_bonus = 0;
        let ordinary = game.ability_effective_resource_cost(&ability, progress);
        assert_eq!(discounted, (ordinary * 3 / 4).max(1));
        assert!(
            fail_with <= game.ability_failure_percent(game.casting_profile().unwrap(), &ability)
        );
        let base_capacity = game.player_ability_baseline().0;
        assert!(capacity_bonus > 0);
        assert!(
            capacity
                .iter()
                .any(|(id, maximum)| *maximum > base_capacity[id])
        );
    }

    #[test]
    fn heroic_speed_refreshes_both_timers_from_one_roll() {
        let mut game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        let effect = rfb_content::ItemUseEffectDefinition::ApplyHeroicSpeed {
            duration_dice: 0,
            duration_sides: 0,
            duration_bonus: 30,
        };
        for _ in 0..2 {
            game.resolve_item_self_effect("demo.item.chain-mail", &effect, &mut Vec::new());
        }
        for id in ["rfb.status.haste", "rfb.status.hero"] {
            assert_eq!(
                game.player
                    .statuses
                    .iter()
                    .find(|status| status.kind_id == id)
                    .unwrap()
                    .remaining_ticks,
                30
            );
        }
    }

    #[test]
    fn death_dragon_armor_grants_vampiric_healing_to_weapon_hits() {
        let mut game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        game.entities.clear();
        game.push_generated_actor(
            "test.vampiric-target".to_owned(),
            "demo.actor.warrens-keeper",
            rfb_protocol::Position { x: 4, y: 3 },
        );
        game.entities[0].hp = 100_000;
        game.entities[0].max_hp = 100_000;
        game.player.position = rfb_protocol::Position { x: 3, y: 3 };
        game.player.hp = 1;
        equip_ego(&mut game, 92, |result| {
            result.rolled_affixes.iter().any(|rolled| {
                rolled
                    .properties
                    .passives
                    .contains(&EquipmentPassive::Vampiric)
            })
        });
        let mut events = Vec::new();
        for _ in 0..20 {
            game.resolve_player_melee(0, false, &mut events, &mut BTreeSet::new(), &mut Vec::new())
                .unwrap();
            if events.iter().any(|event| {
                matches!(
                    event,
                    crate::event::DomainEvent::PlayerVampiricHealed { .. }
                )
            }) {
                break;
            }
        }
        assert!(game.player.hp > 1);
        assert!(events.iter().any(|event| matches!(
            event,
            crate::event::DomainEvent::PlayerVampiricHealed { .. }
        )));
    }

    #[test]
    fn all_armor_egos_generate_and_round_trip() {
        let game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        let affixes: Vec<_> = game
            .content
            .affix_definitions()
            .filter(|affix| {
                affix
                    .rfb_ego
                    .as_ref()
                    .is_some_and(|ego| matches!(ego.source_index, 50..=152))
            })
            .collect();
        assert_eq!(affixes.len(), 76);
        for affix in affixes {
            let index = affix.rfb_ego.as_ref().unwrap().source_index;
            let definition = game.content.item(base_id(index)).unwrap();
            for seed in 1..=48 {
                let mut rng = RfbRng::seeded(seed);
                let result = loop {
                    if let Some(result) = materialize(&mut rng, definition, affix, 90, None) {
                        break result;
                    }
                };
                let mut item = item_for(&game, &definition.id);
                item.enchantments.to_armor = 12;
                result.apply_to(&mut item);
                assert_eq!(item.affix_ids.as_slice(), std::slice::from_ref(&affix.id));
                assert!(
                    crate::game::validation::rolled_affixes_are_valid(&item),
                    "ego {index}, seed {seed}"
                );
                if index == 81 {
                    assert_eq!(item.kind_id, "demo.item.yoiyami-robe");
                    assert_eq!(item.enchantments.to_armor, 0);
                }
                let dto = crate::save::inventory_to_save(std::slice::from_ref(&item)).remove(0);
                let restored = crate::save::inventory_item_from_dto(dto, &game.content)
                    .unwrap_or_else(|error| panic!("ego {index}, seed {seed}: {error:?}"));
                assert_eq!(restored, item, "ego {index}, seed {seed}");
            }
        }
    }

    #[test]
    fn all_back_armor_pools_are_naturally_reachable_except_the_nazgul() {
        let game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        let mut seen = BTreeSet::new();
        for id in [
            "iron-helm",
            "pointy-hat",
            "iron-crown",
            "cloak",
            "leather-gloves",
            "pair-of-metal-shod-boots",
        ] {
            let item = game.content.item(&format!("demo.item.{id}")).unwrap();
            for seed in 1..=6000 {
                let result = roll_and_materialize_rfb_ego_from_affixes_with_rng(
                    rfb_protocol::ItemEnchantmentsDto::default(),
                    &mut RfbRng::seeded(seed),
                    item,
                    game.content.affix_definitions(),
                    100,
                    None,
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
        let expected = (50..=52)
            .chain(54..=56)
            .chain(95..=104)
            .chain(110..=122)
            .chain(125..=130)
            .chain(135..=142)
            .chain(145..=152)
            .filter(|index| *index != 103)
            .collect();
        assert_eq!(seen, expected);
    }

    #[test]
    fn elven_cloak_uses_one_pval_for_its_intrinsic_and_ego_flags() {
        let game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        let definition = game.content.item("demo.item.elven-cloak").unwrap();
        for index in [50, 95, 101, 102, 104] {
            let affix = game
                .content
                .affix_definitions()
                .find(|affix| {
                    affix
                        .rfb_ego
                        .as_ref()
                        .is_some_and(|ego| ego.source_index == index)
                })
                .unwrap();
            for seed in 1..=32 {
                let mut item = item_for(&game, &definition.id);
                materialize(&mut RfbRng::seeded(seed), definition, affix, 90, None)
                    .unwrap()
                    .apply_to(&mut item);
                let bonuses = game.item_equipment_bonuses(&item);
                let pval = bonuses.search_skill / 5;
                assert!(pval > 0);
                assert!(
                    bonuses.stealth_skill == pval || (index == 104 && bonuses.stealth_skill == 0)
                );
                assert_eq!(bonuses.perception_skill, bonuses.search_skill);
                if index == 50 {
                    assert!((1..=4).contains(&bonuses.stealth_skill));
                }
                if index == 104 {
                    assert!(pval <= 3);
                }
            }
        }
    }

    #[test]
    fn armor_mana_regeneration_doubles_normal_recovery_without_stacking_with_high_mage() {
        for build in ["demo.build.paladin-death", "demo.build.high-mage-arcane"] {
            let mut game = Game::new_with_build(7, build).unwrap();
            let id = game.casting_profile().unwrap().resource_id.clone();
            let before = game.player_resource_recovery_change(&id, true);
            let class_rate = game.casting_profile().unwrap().resource_recovery_percent;
            equip_ego(&mut game, 126, |result| {
                result.rolled_affixes.iter().any(|rolled| {
                    rolled
                        .properties
                        .passives
                        .contains(&EquipmentPassive::ManaRegeneration)
                })
            });
            let after = game.player_resource_recovery_change(&id, true);
            assert_eq!(
                after,
                if class_rate >= 200 {
                    before
                } else {
                    before * 2
                }
            );
        }
    }

    #[test]
    fn berserker_gloves_reject_enchantment_and_elemental_auras_use_equipment_sources() {
        let mut game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        let index = equip_ego(&mut game, 142, |_| true);
        let item = game.items[index].clone();
        let draws = game.rng_draw_counter();
        let outcome = game.enchant_item_instance(
            &item.id,
            crate::game::inventory::ItemEnchantmentRequest::new(20, 20, 20),
        );
        assert_eq!(
            outcome.to_hit.successes + outcome.to_damage.successes + outcome.to_armor.successes,
            0
        );
        assert_eq!(game.items[index].enchantments, item.enchantments);
        assert_eq!(game.rng_draw_counter(), draws);
        for (ego, damage_type) in [(97, DamageType::Electricity), (98, DamageType::Cold)] {
            let index = equip_ego(&mut game, ego, |_| true);
            assert!(
                game.player_elemental_contact_aura_sources(damage_type)
                    .contains(&vec![game.items[index].id.clone()])
            );
            game.items[index].location = ItemLocation::Inventory;
            assert!(
                game.player_elemental_contact_aura_sources(damage_type)
                    .is_empty()
            );
        }
    }

    #[test]
    fn genji_improves_both_real_weapon_attacks_and_dual_training_survives_save() {
        let mut game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        let mut offhand = item_for(&game, "demo.item.dagger");
        offhand.id = "test.offhand".to_owned();
        offhand.location = ItemLocation::Equipped {
            slot_id: game
                .body_slots
                .iter()
                .find(|slot| slot.slot_type == "shield")
                .unwrap()
                .id
                .clone(),
        };
        offhand.enchantments.to_damage = 17;
        game.items.push(offhand);
        game.progress.dual_wielding_proficiency = 4000;
        let before = game.player_melee_profiles(&game.player_derived_stats());
        assert_eq!(before.len(), 2);
        equip_ego(&mut game, 140, |_| true);
        let after = game.player_melee_profiles(&game.player_derived_stats());
        for (before, after) in before.iter().zip(&after) {
            assert!(after.melee_skill.value > before.melee_skill.value);
        }
        assert_eq!(after[1].source_item_id.as_deref(), Some("test.offhand"));
        assert!(after[1].to_damage >= 17);
        assert!(after[0].to_damage < after[1].to_damage);
        game.train_dual_wielding(90);
        assert_eq!(game.progress.dual_wielding_proficiency, 4004);
        game.refresh_player_resource_maxima();
        game.identify_carried_items();
        let saved = game.to_save();
        let restored = Game::from_save(saved).unwrap();
        assert_eq!(restored.progress.dual_wielding_proficiency, 4004);
        assert_eq!(restored.equipped_melee_weapons().len(), 2);
    }

    #[test]
    fn bat_night_vision_sees_dark_cells_without_lighting_them_and_respects_blindness() {
        let mut game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        game.current_floor_id = "test.dark-floor".to_owned();
        game.items.clear();
        game.entities.clear();
        game.glow.fill(false);
        // Keep a base instance for the shared materialization helper.
        game.items.push(item_for(
            &Game::new_with_build(7, "demo.build.warrior").unwrap(),
            "demo.item.cloak",
        ));
        let target = (0..i32::from(game.height))
            .flat_map(|y| (0..i32::from(game.width)).map(move |x| rfb_protocol::Position { x, y }))
            .find(|position| {
                *position != game.player.position
                    && crate::game::squared_distance(game.player.position, *position) <= 16
                    && crate::game::visibility::has_line_of_sight(
                        &game,
                        game.player.position,
                        *position,
                    )
            })
            .unwrap();
        assert!(!game.is_visible(target));
        equip_ego(&mut game, 102, |result| {
            result.rolled_affixes.iter().any(|rolled| {
                rolled
                    .properties
                    .passives
                    .contains(&EquipmentPassive::NightVision)
            })
        });
        assert!(game.is_visible(target));
        assert!(!game.position_is_lit(target));
        game.player.statuses.push(
            crate::game::monster_combat::melee_status("rfb.status.blindness", 10, "test.blindness")
                .status,
        );
        assert!(!game.is_visible(target));
    }

    #[test]
    fn crown_magic_resistance_reduces_spells_but_not_innate_breaths() {
        let mut game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        game.items.clear();
        game.items.push(item_for(
            &Game::new_with_build(7, "demo.build.warrior").unwrap(),
            "demo.item.iron-crown",
        ));
        equip_ego(&mut game, 130, |_| true);
        assert!(game.player_equipment_bonuses().magic_resistance_percent >= 15);
        for (ability, expected) in [
            ("demo.ability.armageddon-fire-bolt", 85),
            ("rfb-legacy.ability.breath-acid-20-900-r2", 100),
        ] {
            assert!(game.content.ability(ability).is_some());
            game.player.hp = 1000;
            let result = game.resolve_monster_damage_to_player(
                "test.caster",
                "test.caster",
                ability,
                0,
                100,
                100,
                DamageType::Mana,
                &mut Vec::new(),
            );
            let rfb_protocol::AbilityEffectResolutionDto::Damage { resolution, .. } = result else {
                panic!("damage expected");
            };
            assert_eq!(resolution.final_damage, expected, "{ability}");
        }
    }

    #[test]
    fn sniper_gloves_scale_extra_might_by_the_equipped_launcher_energy() {
        let mut game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        let before = game
            .player_projectile_profile()
            .unwrap()
            .damage_multiplier_percent;
        let index = equip_ego(&mut game, 141, |_| true);
        let might = game
            .item_equipment_bonuses(&game.items[index])
            .launcher_multiplier_delta_percent;
        let profile = game.player_projectile_profile().unwrap();
        let energy = game
            .content
            .item(
                &game
                    .items
                    .iter()
                    .find(|item| item.id == profile.source_item_id)
                    .unwrap()
                    .kind_id,
            )
            .unwrap()
            .projectile_profile
            .as_ref()
            .unwrap()
            .shot_energy;
        assert_eq!(
            i32::from(profile.damage_multiplier_percent - before),
            might * i32::from(energy) / 10_000
        );
    }

    #[test]
    fn armor_hit_and_damage_enchantments_follow_melee_shooting_and_spell_scopes() {
        for index in [135, 141, 126] {
            let mut game = Game::new_with_build(7, "demo.build.warrior").unwrap();
            let melee = game.player_melee_profile(&game.player_derived_stats());
            let shot = game.player_projectile_profile().unwrap();
            let i = equip_ego(&mut game, index, |result| {
                result.enchantment_delta.to_damage > 0 || index == 141
            });
            let enchantments = game.items[i].enchantments;
            let after_melee = game.player_melee_profile(&game.player_derived_stats());
            let after_shot = game.player_projectile_profile().unwrap();
            if index == 135 {
                assert_eq!(
                    after_melee.to_hit - melee.to_hit,
                    i32::from(enchantments.to_hit)
                );
                assert_eq!(
                    after_melee.to_damage - melee.to_damage,
                    i32::from(enchantments.to_damage)
                );
                assert_eq!(after_shot.to_hit, shot.to_hit);
                assert_eq!(after_shot.launcher_to_damage, shot.launcher_to_damage);
            } else if index == 141 {
                assert_eq!(after_melee.to_hit, melee.to_hit);
                assert_eq!(after_melee.to_damage, melee.to_damage);
                assert_eq!(
                    after_shot.to_hit - shot.to_hit,
                    i32::from(enchantments.to_hit)
                );
            } else {
                assert_eq!(after_melee.to_damage, melee.to_damage);
                assert_eq!(after_shot.launcher_to_damage, shot.launcher_to_damage);
                assert_eq!(
                    game.casting_spell_damage_bonus(),
                    enchantments.to_damage as u16
                );
                let mut spell = game
                    .content
                    .ability("demo.ability.armageddon-fire-bolt")
                    .unwrap()
                    .clone();
                let class = game
                    .content
                    .class("demo.class.high-mage")
                    .unwrap()
                    .casting_profile
                    .as_ref()
                    .unwrap();
                let mut without = spell.clone();
                game.items[i].location = ItemLocation::Inventory;
                game.apply_casting_profile_damage_bonus(class, &mut without, 1);
                game.items[i].location = ItemLocation::Equipped {
                    slot_id: game
                        .body_slots
                        .iter()
                        .find(|slot| slot.slot_type == "head")
                        .unwrap()
                        .id
                        .clone(),
                };
                game.apply_casting_profile_damage_bonus(class, &mut spell, 1);
                let bonus = |spell: &rfb_content::AbilityDefinition| match &spell.effect {
                    rfb_content::AbilityEffectDefinition::BoltOrBeamDamage {
                        damage_bonus, ..
                    } => *damage_bonus,
                    other => panic!("unexpected bolt effect: {other:?}"),
                };
                assert_eq!(
                    bonus(&spell) - bonus(&without),
                    enchantments.to_damage as u16
                );
            }
        }
    }

    #[test]
    fn revenge_attack_is_one_blow_and_does_not_trigger_monster_revenge_again() {
        let mut game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        game.entities.clear();
        game.push_generated_actor(
            "test.revenge-target".to_owned(),
            "demo.actor.ebony-monk",
            rfb_protocol::Position { x: 4, y: 3 },
        );
        game.entities[0].hp = 100_000;
        game.entities[0].max_hp = 100_000;
        game.player.position = rfb_protocol::Position { x: 3, y: 3 };
        equip_ego(&mut game, 99, |_| true);
        for _ in 0..20 {
            let hp = game.player.hp;
            let result = game
                .resolve_player_revenge_blow(
                    0,
                    &mut Vec::new(),
                    &mut BTreeSet::new(),
                    &mut Vec::new(),
                )
                .unwrap();
            assert_eq!(result.attacks_used, 1);
            assert_eq!(result.attacks_available, 1);
            assert_eq!(game.player.hp, hp);
        }
    }

    #[test]
    fn armor_base_restrictions_are_enforced_before_generation() {
        for (index, tval, sval) in [
            (60, 34, 2),
            (60, 34, 4),
            (60, 34, 9),
            (60, 34, 10),
            (61, 34, 9),
            (61, 34, 10),
            (62, 34, 10),
            (70, 37, 1),
            (70, 36, 2),
            (71, 36, 2),
            (72, 36, 2),
            (73, 36, 2),
            (74, 36, 2),
            (75, 37, 4),
            (77, 36, 2),
            (80, 37, 4),
            (81, 36, 3),
            (82, 36, 60),
            (91, 38, 6),
            (92, 38, 16),
        ] {
            assert!(!can_apply(index, tval, sval), "{index} on {tval}/{sval}");
        }
        assert!(can_apply(91, 38, 12));
        assert!(can_apply(91, 38, 16));
        assert!(can_apply(92, 38, 18));
    }

    #[test]
    fn dwarven_weight_and_breath_activation_use_instance_state() {
        let game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        let mut rng = RfbRng::seeded(19);
        let definition = game.content.item("demo.item.small-metal-shield").unwrap();
        let affix = game.content.affix("rfb-legacy.affix.dwarven").unwrap();
        let mut item = item_for(&game, &definition.id);
        materialize(&mut rng, definition, affix, 30, None)
            .unwrap()
            .apply_to(&mut item);
        assert_eq!(
            game.item_instance_weight(&item),
            definition.weight_tenths_pound * 2 / 3
        );
        assert_eq!(item.rolled_affixes[0].properties.modifiers.defense, 4);
        let breath = game.content.affix("rfb-legacy.affix.breath").unwrap();
        for id in ["multi-hued", "gold", "law", "chaos"] {
            let definition = game
                .content
                .item(&format!("demo.item.{id}-dragon-scale-mail"))
                .unwrap();
            let result = materialize(&mut rng, definition, breath, 90, None).unwrap();
            let activation = result.activation.unwrap();
            assert!(activation.profile_id.ends_with(&format!(
                "-{}",
                definition.rfb_base_kind.unwrap().source_index
            )));
            let enhanced = breath
                .device_generation
                .as_ref()
                .unwrap()
                .activations
                .iter()
                .find(|profile| profile.id == activation.profile_id)
                .unwrap();
            let base = definition.device_generation.as_ref().unwrap();
            let recovery = base.activations[0].recovery.or(base.recovery).unwrap();
            assert_eq!(
                enhanced.recovery.unwrap().interval_ticks * 2,
                recovery.interval_ticks
            );
        }
    }
}
