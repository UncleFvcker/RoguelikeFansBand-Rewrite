// SPDX-License-Identifier: MPL-2.0

use super::*;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Pval {
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
    Stealth,
    LessStealth,
    Speed,
    Search,
    Mastery,
    Capacity,
    Blows,
}

pub(super) fn can_apply(index: u32, tval: u16, sval: u16) -> bool {
    match index {
        50..=53 => matches!(tval, 34 | 36 | 37),
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
        _ => false,
    }
}

pub(super) fn materialize(
    rng: &mut RfbRng,
    item: &ItemDefinition,
    affix: &AffixDefinition,
    level: u16,
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
    let body = matches!(base.tval, 36 | 37);
    let mut state = RolledAffixState {
        affix_id: affix.id.clone(),
        ..Default::default()
    };
    let mut flags = BTreeSet::new();
    let mut pval = 0;
    let mut activation = None;
    let properties = &mut state.properties;
    let to_a = &mut state.enchantment_delta.to_armor;
    match index {
        50 => {
            if one_in(rng, 3) {
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
        _ => return None,
    }
    if matches!(index, 53 | 71..=75 | 80) {
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
    if maximum_pval > 0 {
        pval += randint1(rng, maximum_pval);
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
        None,
        activation,
        charges,
    );
    if index == 81 {
        result.kind_id_override = Some("demo.item.yoiyami-robe".to_owned());
        result.clear_armor_enchantment = true;
    }
    Some(result)
}

fn random_activation(
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

fn apply_pval(properties: &mut AffixPropertyBundleDefinition, flag: Pval, value: i32) {
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
        Speed => properties.modifiers.speed += value,
        Stealth => properties.equipment_bonuses.stealth_skill += value,
        LessStealth => properties.equipment_bonuses.stealth_skill -= value,
        Search => {
            properties.equipment_bonuses.search_skill += 5 * value;
            properties.equipment_bonuses.perception_skill += 5 * value;
        }
        Mastery => properties.equipment_bonuses.device_skill += 8 * value,
        Capacity => properties.equipment_bonuses.spell_capacity_bonus += value,
        Blows => properties.equipment_bonuses.melee_attacks_delta_percent += 50 * value,
    }
}

fn maxima(index: u32) -> (i16, i16, i16, u16) {
    match index {
        50 | 52 | 80 => (0, 0, 10, 0),
        51 => (0, 0, 8, 0),
        53 => (0, 0, 10, 3),
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
        _ => unreachable!("validated front armor ego"),
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
                materialize(&mut RfbRng::seeded(seed), definition, affix, 90).filter(&predicate)
            })
            .expect("requested generated armor property");
        let mut item = item_for(game, &definition.id);
        item.id = format!("test.armor.{source_index}");
        item.location = ItemLocation::Equipped {
            slot_id: definition.equipment_slot.clone().unwrap(),
        };
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
    fn all_front_armor_egos_generate_and_round_trip() {
        let game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        let affixes: Vec<_> = game.content.affix_definitions().filter(|affix| affix.rfb_ego.as_ref()
            .is_some_and(|ego| matches!(ego.source_index, 50..=53 | 60..=64 | 70..=77 | 80..=82 | 85..=92))).collect();
        assert_eq!(affixes.len(), 28);
        for affix in affixes {
            let index = affix.rfb_ego.as_ref().unwrap().source_index;
            let definition = game.content.item(base_id(index)).unwrap();
            for seed in 1..=48 {
                let mut rng = RfbRng::seeded(seed);
                let result = loop {
                    if let Some(result) = materialize(&mut rng, definition, affix, 90) {
                        break result;
                    }
                };
                let mut item = item_for(&game, &definition.id);
                item.enchantments.to_armor = 12;
                result.apply_to(&mut item);
                assert_eq!(item.affix_ids, [affix.id.clone()]);
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
        materialize(&mut rng, definition, affix, 30)
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
            let result = materialize(&mut rng, definition, breath, 90).unwrap();
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
