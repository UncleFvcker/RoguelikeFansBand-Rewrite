// SPDX-License-Identifier: MPL-2.0
// Equipment scoring for bldg.c::enchant_item's curse-free valuation copy.
// Source: object3.c at a0d92b6378d148c5262cc236b8fa6ed2ca06a54c.

use super::{Game, ItemInstance};
use rfb_content::{
    ActorDamageType, ActorResistanceLevel, AffixPropertyBundleDefinition, AmmunitionTypeDefinition,
    EquipmentPassive, ItemDefinition, ItemDestructionElement, SlayLevel, SlayTarget, WeaponBrand,
};
use rfb_protocol::{ItemEnchantmentsDto, WeaponTraitDto};

fn interpolate(value: i64, points: &[(i64, i64)]) -> i64 {
    if value <= points[0].0 {
        return points[0].1;
    }
    for pair in points.windows(2) {
        let [(x0, y0), (x1, y1)] = pair else {
            unreachable!()
        };
        if value <= *x1 {
            return y0 + (value - x0) * (y1 - y0) / (x1 - x0);
        }
    }
    points.last().unwrap().1
}

fn scored_group(entries: impl IntoIterator<Item = (bool, i64)>) -> i64 {
    let mut count = 0;
    let mut total = 0;
    for (present, score) in entries {
        if present {
            let c = f64::from(count);
            total += (score as f64 * (1.0 + c / 10.0 + c * c / 50.0)) as i64;
            count += 1;
        }
    }
    total
}

impl Game {
    fn item_scoring_properties(&self, item: &ItemInstance) -> AffixPropertyBundleDefinition {
        let d = self
            .content
            .item(&item.kind_id)
            .expect("item definition must exist");
        let mut properties = AffixPropertyBundleDefinition {
            modifiers: d.modifiers.clone(),
            equipment_bonuses: d.equipment_bonuses.clone(),
            resistances: d.resistances.clone(),
            status_immunities: d.status_immunities.clone(),
            slays: d.slays.clone(),
            brands: d.brands.clone(),
            passives: d.passives.clone(),
        };
        for id in &item.affix_ids {
            let a = self.content.affix(id).expect("item affix must exist");
            super::ego::merge_affix_properties(
                &mut properties,
                &AffixPropertyBundleDefinition {
                    modifiers: a.modifiers.clone(),
                    equipment_bonuses: a.equipment_bonuses.clone(),
                    resistances: a.resistances.clone(),
                    status_immunities: a.status_immunities.clone(),
                    slays: a.slays.clone(),
                    brands: a.brands.clone(),
                    passives: a.passives.clone(),
                },
            );
        }
        super::ego::merge_affix_properties(&mut properties, &item.intrinsic_properties);
        for rolled in &item.rolled_affixes {
            super::ego::merge_affix_properties(&mut properties, &rolled.properties);
        }
        properties
    }

    fn item_scoring_base<'a>(&'a self, definition: &'a ItemDefinition) -> &'a ItemDefinition {
        definition
            .artifact_generation
            .as_ref()
            .map_or(definition, |artifact| {
                self.content
                    .item(&artifact.base_item_kind_id)
                    .expect("artifact base kind must exist")
            })
    }

    pub(super) fn item_total_enchantments(&self, item: &ItemInstance) -> ItemEnchantmentsDto {
        let d = self
            .content
            .item(&item.kind_id)
            .expect("item definition must exist");
        let (hit, damage) = if let Some(p) = &d.melee_profile {
            (p.to_hit, p.to_damage)
        } else if let Some(p) = &d.projectile_profile {
            (p.to_hit, p.to_damage)
        } else if let Some(p) = &d.ammunition_profile {
            (p.to_hit, p.to_damage)
        } else {
            (0, 0)
        };
        let modifiers = self.item_scoring_properties(item).modifiers;
        let intrinsic_armor = if d.tags.iter().any(|tag| tag == "armor") {
            i32::from(d.armor_enchantment) - d.modifiers.defense
        } else {
            0
        };
        ItemEnchantmentsDto {
            to_hit: (hit + i32::from(item.enchantments.to_hit))
                .clamp(i16::MIN as i32, i16::MAX as i32) as i16,
            to_damage: (damage + i32::from(item.enchantments.to_damage))
                .clamp(i16::MIN as i32, i16::MAX as i32) as i16,
            to_armor: (modifiers.defense + intrinsic_armor + i32::from(item.enchantments.to_armor))
                .clamp(i16::MIN as i32, i16::MAX as i32) as i16,
        }
    }

    pub(super) fn item_enchantment_value(&self, item: &ItemInstance) -> i64 {
        let d = self
            .content
            .item(&item.kind_id)
            .expect("item definition must exist");
        let base = self.item_scoring_base(d);
        let p = self.item_scoring_properties(item);
        let bonuses = &p.equipment_bonuses;
        let ench = self.item_total_enchantments(item);
        let h = i64::from(ench.to_hit);
        let damage = i64::from(ench.to_damage);
        let armor = i64::from(ench.to_armor);
        let artifact = d.tags.iter().any(|tag| tag == "artifact");
        let mut value;
        if let Some(profile) = &d.melee_profile {
            if base
                .rfb_base_kind
                .is_some_and(|kind| kind.tval == 23 && kind.sval == 32)
            {
                return 2500;
            }
            if base
                .rfb_base_kind
                .is_some_and(|kind| kind.tval == 23 && kind.sval == 34)
            {
                return 1;
            }
            let base_profile = base
                .melee_profile
                .as_ref()
                .expect("weapon base must be a weapon");
            let average = |dice: u16, sides: u16| f64::from(dice) * (f64::from(sides) + 1.0) / 2.0;
            let b = average(base_profile.damage_dice, base_profile.damage_sides);
            let mut dice = (profile.damage_dice, profile.damage_sides);
            if let Some(roll) = item
                .rolled_affixes
                .iter()
                .find_map(|roll| roll.melee_damage_dice)
            {
                dice = (roll.dice, roll.sides);
            }
            let mut multiplier = 1.0;
            let mut weight = 1.0;
            let mut add = |amount: f64| {
                multiplier += amount * weight;
                weight *= 0.7;
            };
            let slay = |target, normal, kill| match p.slays.get(&target) {
                Some(SlayLevel::Kill) => kill,
                Some(SlayLevel::Slay) => normal,
                None => 0.0,
            };
            for (target, normal, kill) in [
                (SlayTarget::Evil, 0.8, 2.0),
                (SlayTarget::Undead, 0.2, 0.4),
                (SlayTarget::Demon, 0.3, 0.6),
                (SlayTarget::Living, 0.7, 1.75),
                (SlayTarget::Good, 0.1, 0.25),
            ] {
                let amount = slay(target, normal, kill);
                if amount > 0.0 {
                    add(amount);
                }
            }
            for (brand, amount) in [
                (WeaponBrand::Acid, 0.225),
                (WeaponBrand::Electricity, 0.225),
                (WeaponBrand::Fire, 0.15),
                (WeaponBrand::Cold, 0.15),
            ] {
                if p.brands.contains(&brand) {
                    add(amount);
                }
            }
            for (target, normal, kill) in [
                (SlayTarget::Dragon, 0.2, 0.4),
                (SlayTarget::Human, 0.15, 0.3),
                (SlayTarget::Giant, 0.15, 0.3),
            ] {
                let amount = slay(target, normal, kill);
                if amount > 0.0 {
                    add(amount);
                }
            }
            if p.brands.contains(&WeaponBrand::Poison) {
                add(0.1125);
            }
            for (target, normal, kill) in [
                (SlayTarget::Orc, 0.02, 0.04),
                (SlayTarget::Troll, 0.2, 0.4),
                (SlayTarget::Animal, 0.3, 0.6),
            ] {
                let amount = slay(target, normal, kill);
                if amount > 0.0 {
                    add(amount);
                }
            }
            if p.brands.contains(&WeaponBrand::Chaos) {
                multiplier += 0.2;
            }
            if p.passives.contains(&EquipmentPassive::Vampiric) {
                multiplier += 0.1;
            }
            if Self::item_has_weapon_trait(item, WeaponTraitDto::ManaBrand) {
                multiplier = (multiplier * 1.5 + 1.0) * 0.25 + multiplier * 0.75;
            }
            if Self::item_has_weapon_trait(item, WeaponTraitDto::Vorpal2) {
                multiplier *= 1.67;
            } else if d.vorpal || Self::item_has_weapon_trait(item, WeaponTraitDto::Vorpal) {
                multiplier *= 1.22;
            }
            if Self::item_has_weapon_trait(item, WeaponTraitDto::Stun) {
                multiplier *= 1.18;
            }
            let mut output = (average(dice.0, dice.1) * multiplier + damage as f64).max(1.0);
            if bonuses.melee_attacks > 0 {
                output += (output + 40.0) * f64::from(bonuses.melee_attacks) / 10.0;
            }
            let x = output - b;
            value =
                (x * 100.0) as i64 + (x * x * 5.0) as i64 + (output * output * output * 0.2) as i64;
            value += if h <= 10 { 100 * h } else { 10 * h * h };
            value += 500 * armor + i64::from(d.weight_tenths_pound.saturating_sub(20));
            if d.artifact_generation
                .as_ref()
                .is_none_or(|a| a.source_index != 136)
                && (d.weight_tenths_pound > 99
                    || base.rfb_base_kind.is_some_and(|kind| {
                        kind.tval == 22 || (kind.tval == 21 && kind.sval == 51)
                    }))
            {
                value += 75;
            }
            value += resistance_value(&p, d.reflects_bolts) * 7 / 10;
            if p.passives.contains(&EquipmentPassive::Vampiric) {
                value += 3000;
            }
            if Self::item_has_weapon_trait(item, WeaponTraitDto::Impact) {
                value += 250;
            }
            if Self::item_has_weapon_trait(item, WeaponTraitDto::Wild) {
                value += 10000;
            }
        } else if let Some(profile) = &d.projectile_profile {
            let unit = match profile.ammunition_type {
                AmmunitionTypeDefinition::Shot => 22,
                AmmunitionTypeDefinition::Arrow => 27,
                AmmunitionTypeDefinition::Bolt => 30,
            };
            let base_profile = base
                .projectile_profile
                .as_ref()
                .expect("launcher base must be a launcher");
            let base_damage = i64::from(base_profile.damage_multiplier_percent) * unit / 10;
            let output = (i64::from(profile.damage_multiplier_percent)
                + i64::from(bonuses.launcher_multiplier_delta_percent))
            .max(0)
                * unit
                / 10;
            let extra = (output - base_damage).max(0);
            value = base_damage / 20
                + (base_damage - 440).pow(2) / 600
                + 150 * extra / 10
                + extra * extra * 25 / 100;
            for brand in [
                WeaponBrand::Poison,
                WeaponBrand::Acid,
                WeaponBrand::Electricity,
                WeaponBrand::Fire,
                WeaponBrand::Cold,
            ] {
                if p.brands.contains(&brand) {
                    value = value * 5 / 4;
                }
            }
            value += 150 * damage;
            if damage > 10 {
                value += (damage - 10).pow(2) * 15 * 10000 / i64::from(profile.shot_energy);
            }
            value += value * i64::from(bonuses.base_shot_delta_percent) / 100;
            value = value * 10000 / i64::from(profile.shot_energy);
            value += 100 * h
                + 10 * h * h.abs()
                + 500 * armor
                + 30 * armor * armor.abs()
                + i64::from(d.weight_tenths_pound);
            value += resistance_value(&p, d.reflects_bolts);
        } else {
            let ac = i64::from(d.modifiers.defense) - i64::from(d.armor_enchantment);
            value = if ac < 10 {
                ac.pow(3)
            } else {
                1000 + 200 * (ac - 10) + 20 * (ac - 10).pow(2)
            };
            if let Some(kind) = base.rfb_base_kind
                && kind.tval == 33
            {
                value += match kind.sval {
                    11 => 1000,
                    12 => 2000,
                    _ => 0,
                };
            }
            if armor != 0 {
                let mut boost = interpolate(
                    armor.abs(),
                    &[
                        (1, 200),
                        (5, 1500),
                        (10, 4000),
                        (15, 7500),
                        (20, 11000),
                        (25, 15000),
                        (30, 20000),
                        (100, 90000),
                    ],
                );
                let acid_proof = artifact
                    || d.elemental_destruction_immunities
                        .contains(&ItemDestructionElement::Acid)
                    || item
                        .permanent_destruction_immunities
                        .contains(&ItemDestructionElement::Acid)
                    || item.affix_ids.iter().any(|id| {
                        self.content
                            .affix(id)
                            .unwrap()
                            .elemental_destruction_immunities
                            .contains(&ItemDestructionElement::Acid)
                    });
                if !acid_proof {
                    boost /= 2;
                }
                value += boost * armor.signum();
            }
            value += resistance_value(&p, d.reflects_bolts) + off_weapon_brand_value(&p);
            value += 15000 * i64::from(bonuses.melee_attacks);
            value += 250 * h + 25 * h * h.abs();
            value += if damage > 20 {
                35000 + (damage - 20) * 1000
            } else {
                750 * damage + 50 * damage * damage.abs()
            };
        }
        value += common_value(&p, d.reflects_bolts);
        let digging = i64::from(bonuses.digging_skill) + i64::from(d.tunneling_pval);
        value += if d.melee_profile.is_some() {
            digging
                * if base.rfb_base_kind.is_some_and(|kind| kind.tval == 20) && digging <= 2 {
                    150
                } else {
                    1000
                }
        } else if d.projectile_profile.is_some() {
            0
        } else {
            digging * 800
        };
        if let Some(activation) = &item.activation {
            let profile =
                super::item_device_generation(&self.content, &item.kind_id, &item.affix_ids)
                    .and_then(|g| g.activations.iter().find(|p| p.id == activation.profile_id))
                    .expect("equipment activation must have a profile");
            value += i64::from(
                profile
                    .equipment_value
                    .expect("equipment activation must have a valuation"),
            );
        }
        if !artifact {
            value = (value + 1) * 3 / 4;
        }
        value.max(if artifact || !item.affix_ids.is_empty() {
            1
        } else {
            0
        })
    }
}

fn common_value(p: &AffixPropertyBundleDefinition, reflection: bool) -> i64 {
    use EquipmentPassive::*;
    let has = |flag| p.passives.contains(&flag);
    let mut value = 0;
    for group in [
        &[
            (EspOrc, 500),
            (EspTroll, 500),
            (EspGiant, 500),
            (EspGood, 500),
            (EspAnimal, 600),
            (EspUndead, 600),
            (EspDemon, 600),
            (EspDragon, 700),
            (EspHuman, 700),
            (EspNonliving, 1500),
            (EspLiving, 1500),
        ][..],
        &[
            (SustainStrength, 1000),
            (SustainIntelligence, 1000),
            (SustainWisdom, 1000),
            (SustainDexterity, 1000),
            (SustainCharisma, 1000),
            (SustainConstitution, 1000),
        ],
        &[(Levitation, 1800), (HoldLife, 1000), (Regeneration, 1000)],
    ] {
        value += scored_group(group.iter().map(|(flag, score)| (has(*flag), *score)));
    }
    value += scored_group([
        (has(Warning), 100),
        (p.equipment_bonuses.light_radius != 0, 100),
        (has(SlowDigestion), 100),
        (has(SeeInvisible), 500),
        (
            p.status_immunities
                .iter()
                .any(|id| id == crate::effect::STATUS_PARALYSIS),
            750,
        ),
    ]);
    value += scored_group([
        (reflection, 5000),
        (has(EspEvil), 3000),
        (has(Telepathy), 10000),
    ]);
    let m = &p.modifiers;
    for positive in [true, false] {
        value += scored_group(
            [
                (m.strength, 400),
                (m.intelligence, 333),
                (m.wisdom, 333),
                (m.dexterity, 367),
                (m.constitution, 433),
                (m.charisma, 267),
            ]
            .map(|(v, score)| {
                let n = i64::from(v.abs().min(10));
                ((v > 0) == positive && v != 0, score * (n * (n + 1) / 2 + n))
            }),
        ) * if positive { 1 } else { -1 };
    }
    let speed = i64::from(m.speed);
    if speed > 0 {
        value += 1000 * speed + 500 * speed * speed;
    }
    let b = &p.equipment_bonuses;
    for positive in [true, false] {
        value += scored_group(
            [
                (b.device_skill / 5, 1000),
                (b.stealth_skill, 300),
                (m.spell_power_bonus, 1666),
                (b.life_percent / 3, 1000),
            ]
            .map(|(v, score)| {
                let n = i64::from(v).abs().min(10);
                ((v > 0) == positive && v != 0, score * (n * (n + 1) / 2 + n))
            })
            .into_iter()
            .chain([(!positive && speed < 0, 1000 * speed.clamp(-10, 0).pow(2))]),
        ) * if positive { 1 } else { -1 };
    }
    value += i64::from(b.search_skill) * 200 + i64::from(b.infravision) * 400;
    value
}

fn resistance_value(p: &AffixPropertyBundleDefinition, reflection: bool) -> i64 {
    use ActorDamageType::*;
    use ActorResistanceLevel::*;
    let resists = |element| matches!(p.resistances.get(&element), Some(Resistant | Strong));
    let immune = |element| p.resistances.get(&element) == Some(&Immune);
    let vulnerable = |element| p.resistances.get(&element) == Some(&Vulnerable);
    let low = [
        (Acid, 3000),
        (Electricity, 3000),
        (Fire, 3500),
        (Cold, 3000),
        (Poison, 2500),
    ];
    let high = [
        (Light, 2500),
        (Dark, 4000),
        (Confusion, 4500),
        (Nether, 5500),
        (Nexus, 3500),
        (Chaos, 6000),
        (Sound, 4000),
        (Shards, 7000),
        (Disenchant, 5500),
        (Time, 9001),
    ];
    let immunities = [
        (Acid, 12000),
        (Electricity, 14000),
        (Fire, 15000),
        (Cold, 14000),
    ];
    let vulnerabilities = [
        Acid,
        Electricity,
        Fire,
        Cold,
        Poison,
        Light,
        Dark,
        Blindness,
        Confusion,
        Nether,
        Nexus,
        Chaos,
        Sound,
        Shards,
        Disenchant,
    ];
    let mut value = scored_group(low.map(|(e, c)| (resists(e), c)))
        + scored_group(high.map(|(e, c)| (resists(e), c)))
        + scored_group([(resists(Blindness), 1000), (resists(Fear), 2500)])
        + scored_group(immunities.map(|(e, c)| (immune(e), c)))
        - scored_group(
            vulnerabilities.map(|e| (vulnerable(e), if e == Blindness { 2000 } else { 5000 })),
        );
    let count_low = low.iter().filter(|(e, _)| resists(*e)).count() as i64;
    let count_high =
        high.iter().filter(|(e, _)| resists(*e)).count() as i64 + i64::from(resists(Fear));
    let count_immune = immunities.iter().filter(|(e, _)| immune(*e)).count() as i64;
    let count_vulnerable = vulnerabilities.iter().filter(|e| vulnerable(**e)).count() as i64;
    let light = i64::from(resists(Light));
    let honorary = i64::from(p.modifiers.speed > 0)
        + i64::from(p.passives.contains(&EquipmentPassive::Telepathy));
    let big = (if count_low + light >= 5 {
        2
    } else if count_low + light >= 2 {
        1
    } else {
        0
    }) - light
        + count_high
        + count_immune
        - (count_vulnerable + 1) / 2
        + honorary
        + i64::from(reflection);
    let balanced = (count_high - count_vulnerable).min(count_low + count_immune * 2) + honorary;
    for count in [big, balanced] {
        if count >= 3 {
            value += 300 * (count - 1) * (count - 2);
        }
    }
    value
}

fn off_weapon_brand_value(p: &AffixPropertyBundleDefinition) -> i64 {
    use SlayLevel::*;
    use SlayTarget::*;
    use WeaponBrand::*;
    let slay = |target, level| p.slays.get(&target) == Some(&level);
    let brand = |b| p.brands.contains(&b);
    let entries = [
        (slay(Evil, Kill), 22000),
        (slay(Living, Kill), 15000),
        (slay(Demon, Kill), 14000),
        (slay(Undead, Kill), 14000),
        (slay(Human, Kill), 14000),
        (slay(Dragon, Kill), 12500),
        (p.passives.contains(&EquipmentPassive::Vampiric), 12500),
        (slay(Good, Kill), 10000),
        (slay(Evil, Slay), 10000),
        (brand(Poison), 8500),
        (slay(Living, Slay), 8500),
        (slay(Demon, Slay), 7500),
        (slay(Undead, Slay), 7500),
        (slay(Human, Slay), 7500),
        (brand(Acid), 7000),
        (brand(Electricity), 7000),
        (slay(Animal, Kill), 6500),
        (slay(Dragon, Slay), 6500),
        (brand(Fire), 6000),
        (brand(Cold), 6000),
        (slay(Giant, Kill), 5500),
        (slay(Good, Slay), 5000),
        (slay(Troll, Kill), 4000),
        (slay(Orc, Kill), 4000),
        (slay(Animal, Slay), 3250),
        (slay(Giant, Slay), 2750),
        (slay(Troll, Slay), 2000),
        (slay(Orc, Slay), 2000),
    ];
    entries
        .into_iter()
        .filter_map(|(present, score)| present.then_some(score))
        .enumerate()
        .map(|(index, score)| score * 3 / (index as i64 * 2 + 3))
        .sum()
}
