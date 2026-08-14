// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeSet;

use rfb_content::{
    ActorDamageType, ActorResistanceLevel, AffixDefinition, AffixPropertyBundleDefinition,
    ContentCatalog, EquipmentPassive, ItemDefinition, RfbEgoTypeDefinition, SlayLevel, SlayTarget,
    StatModifiers, WeaponBrand,
};
use rfb_protocol::{
    ItemActivationDto, ItemChargesDto, ItemCurseEffectDto, ItemEnchantmentsDto, MeleeDamageDiceDto,
    WeaponTraitDto,
};

use crate::{
    rng::{RfbRng, rfb_m_bonus},
    state::{ItemInstance, RolledAffixState},
};

use super::{initial_item_runtime_state, merge_equipment_bonuses, roll_weighted_index_with_rng};

/// Complete generated affix state shared by content-driven consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EgoMaterialization {
    pub(super) affix_ids: Vec<String>,
    pub(super) rolled_affixes: Vec<RolledAffixState>,
    pub(super) enchantment_delta: ItemEnchantmentsDto,
    pub(super) melee_damage_dice: Option<MeleeDamageDiceDto>,
    pub(super) weapon_traits: BTreeSet<WeaponTraitDto>,
    pub(super) curse_effects: BTreeSet<ItemCurseEffectDto>,
    pub(super) activation: Option<ItemActivationDto>,
    pub(super) charges: Option<ItemChargesDto>,
}

impl EgoMaterialization {
    pub(super) fn new(
        affix_ids: Vec<String>,
        rolled_affixes: Vec<RolledAffixState>,
        activation: Option<ItemActivationDto>,
        charges: Option<ItemChargesDto>,
    ) -> Self {
        debug_assert!(
            rolled_affixes
                .iter()
                .filter(|rolled| rolled.melee_damage_dice.is_some())
                .count()
                <= 1,
            "one weapon ego may replace melee dice"
        );
        let enchantment_delta =
            rolled_affixes
                .iter()
                .fold(ItemEnchantmentsDto::default(), |mut total, rolled| {
                    total.to_hit = total.to_hit.saturating_add(rolled.enchantment_delta.to_hit);
                    total.to_damage = total
                        .to_damage
                        .saturating_add(rolled.enchantment_delta.to_damage);
                    total.to_armor = total
                        .to_armor
                        .saturating_add(rolled.enchantment_delta.to_armor);
                    total
                });
        let melee_damage_dice = rolled_affixes
            .iter()
            .find_map(|rolled| rolled.melee_damage_dice);
        let weapon_traits = rolled_affixes
            .iter()
            .flat_map(|rolled| rolled.weapon_traits.iter().copied())
            .collect();
        let curse_effects = rolled_affixes
            .iter()
            .flat_map(|rolled| rolled.curse_effects.iter().copied())
            .collect();
        Self {
            affix_ids,
            rolled_affixes,
            enchantment_delta,
            melee_damage_dice,
            weapon_traits,
            curse_effects,
            activation,
            charges,
        }
    }

    /// Commits a fully prepared materialization to an existing item in one step.
    pub(super) fn apply_to(self, item: &mut ItemInstance) {
        let enchantments = ItemEnchantmentsDto {
            to_hit: item
                .enchantments
                .to_hit
                .saturating_add(self.enchantment_delta.to_hit),
            to_damage: item
                .enchantments
                .to_damage
                .saturating_add(self.enchantment_delta.to_damage),
            to_armor: item
                .enchantments
                .to_armor
                .saturating_add(self.enchantment_delta.to_armor),
        };
        item.affix_ids = self.affix_ids;
        item.rolled_affixes = self.rolled_affixes;
        item.enchantments = enchantments;
        item.activation = self.activation;
        item.charges = self.charges;
        item.device_recovery_progress = 0;
    }
}

/// Materializes static affix identities, dynamic roll groups, and activation
/// state before a caller commits the result to a generated or existing item.
pub(super) fn materialize_ego_with_rng(
    content: &ContentCatalog,
    rng: &mut RfbRng,
    kind_id: &str,
    mut affix_ids: Vec<String>,
    roll_depth: impl Fn(&AffixDefinition) -> u16,
    activation_depth: u16,
) -> EgoMaterialization {
    affix_ids.sort();
    debug_assert!(affix_ids.windows(2).all(|pair| pair[0] != pair[1]));
    let rolled_affixes = roll_affix_properties_with_rng(content, rng, &affix_ids, roll_depth);
    let (activation, charges) =
        initial_item_runtime_state(content, rng, kind_id, &affix_ids, activation_depth);
    EgoMaterialization::new(affix_ids, rolled_affixes, activation, charges)
}

/// Selects one authoritative RFB ego without changing any item state.
/// Callers own the exact item-to-ego-type policy and the later materialization.
#[allow(dead_code)]
pub(crate) fn roll_rfb_ego_affix_id(
    content: &ContentCatalog,
    rng: &mut RfbRng,
    generation_level: u16,
    allowed_types: &[RfbEgoTypeDefinition],
) -> Option<String> {
    roll_rfb_ego_from_affixes(
        content.affix_definitions(),
        rng,
        generation_level,
        allowed_types,
    )
    .map(str::to_owned)
}

fn roll_affix_properties_with_rng(
    content: &ContentCatalog,
    rng: &mut RfbRng,
    affix_ids: &[String],
    roll_depth: impl Fn(&AffixDefinition) -> u16,
) -> Vec<RolledAffixState> {
    let mut rolled_affixes = Vec::new();
    for affix_id in affix_ids {
        let affix = content
            .affix(affix_id)
            .expect("selected affix must remain available");
        let depth = roll_depth(affix);
        let mut properties = AffixPropertyBundleDefinition::default();
        for group in &affix.roll_groups {
            let eligible = group
                .candidates
                .iter()
                .filter(|candidate| candidate.min_depth <= depth && depth <= candidate.max_depth)
                .collect::<Vec<_>>();
            if eligible.is_empty() {
                continue;
            }
            let weights = eligible
                .iter()
                .map(|candidate| candidate.weight)
                .collect::<Vec<_>>();
            for _ in 0..group.rolls {
                let selected = eligible[roll_weighted_index_with_rng(rng, &weights)];
                merge_affix_properties(&mut properties, &selected.properties);
            }
        }
        if properties != AffixPropertyBundleDefinition::default() {
            rolled_affixes.push(RolledAffixState {
                affix_id: affix_id.clone(),
                properties,
                ..RolledAffixState::default()
            });
        }
    }
    rolled_affixes
}

fn merge_affix_properties(
    total: &mut AffixPropertyBundleDefinition,
    addition: &AffixPropertyBundleDefinition,
) {
    merge_stat_modifiers(&mut total.modifiers, &addition.modifiers);
    merge_equipment_bonuses(&mut total.equipment_bonuses, &addition.equipment_bonuses);
    for (damage_type, level) in &addition.resistances {
        let current = total.resistances.entry(*damage_type).or_insert(*level);
        if actor_resistance_rank(*level) > actor_resistance_rank(*current) {
            *current = *level;
        }
    }
    let status_immunities = total
        .status_immunities
        .iter()
        .chain(&addition.status_immunities)
        .cloned()
        .collect::<BTreeSet<_>>();
    total.status_immunities = status_immunities.into_iter().collect();
    for (target, level) in &addition.slays {
        let current = total.slays.entry(*target).or_insert(*level);
        if *level > *current {
            *current = *level;
        }
    }
    total.brands.extend(&addition.brands);
    total.passives.extend(&addition.passives);
}

fn merge_stat_modifiers(total: &mut StatModifiers, addition: &StatModifiers) {
    total.attack = total.attack.saturating_add(addition.attack);
    total.defense = total.defense.saturating_add(addition.defense);
    total.max_hp = total.max_hp.saturating_add(addition.max_hp);
    total.strength = total.strength.saturating_add(addition.strength);
    total.intelligence = total.intelligence.saturating_add(addition.intelligence);
    total.wisdom = total.wisdom.saturating_add(addition.wisdom);
    total.dexterity = total.dexterity.saturating_add(addition.dexterity);
    total.constitution = total.constitution.saturating_add(addition.constitution);
    total.charisma = total.charisma.saturating_add(addition.charisma);
    total.speed = total.speed.saturating_add(addition.speed);
    total.spell_power_bonus = total
        .spell_power_bonus
        .saturating_add(addition.spell_power_bonus);
}

const TV_DIGGING: u16 = 20;
const TV_HAFTED: u16 = 21;
const TV_POLEARM: u16 = 22;
const TV_SWORD: u16 = 23;
const SV_WHIP: u16 = 2;

#[derive(Debug, Default)]
struct RfbWeaponRoll {
    state: RolledAffixState,
    charisma_pval: bool,
    dexterity_pval: bool,
    blows_pval: bool,
}

/// Materializes one selected RFB weapon/digger ego without changing item state.
/// A rejected base-kind combination returns `None`; callers may then reselect.
pub(crate) fn materialize_rfb_weapon_ego_with_rng(
    rng: &mut RfbRng,
    item: &ItemDefinition,
    affix: &AffixDefinition,
    generation_level: u16,
) -> Option<EgoMaterialization> {
    let source_index = affix.rfb_ego.as_ref()?.source_index;
    let base_kind = item.rfb_base_kind?;
    let profile = item.melee_profile.as_ref()?;
    let base_dice = MeleeDamageDiceDto {
        dice: profile.damage_dice,
        sides: profile.damage_sides,
    };
    let is_digger = base_kind.tval == TV_DIGGING;
    let is_weapon = matches!(base_kind.tval, TV_HAFTED | TV_POLEARM | TV_SWORD);
    if !is_digger && !is_weapon {
        return None;
    }

    let mut roll = RfbWeaponRoll {
        state: RolledAffixState {
            affix_id: affix.id.clone(),
            ..RolledAffixState::default()
        },
        ..RfbWeaponRoll::default()
    };
    let mut dice = base_dice;

    match source_index {
        1 if is_weapon => {
            roll_rfb_slaying(rng, &mut roll.state.properties, generation_level, false)
        }
        3 if is_weapon => {
            roll.state.weapon_traits.insert(WeaponTraitDto::ManaBrand);
        }
        4 => {
            roll.state.weapon_traits.insert(WeaponTraitDto::Blessed);
            if is_weapon {
                if one_in(rng, 2) {
                    roll.state
                        .properties
                        .passives
                        .insert(EquipmentPassive::EspGood);
                }
                if one_in(rng, 5) {
                    add_light(&mut roll.state.properties);
                }
                roll_rfb_slaying(rng, &mut roll.state.properties, generation_level, false);
            }
        }
        5 => {}
        8 if is_weapon => {}
        9 if is_weapon => {
            roll_rfb_craft(rng, &mut roll.state, generation_level, false);
        }
        10 if is_weapon => {
            roll.state.weapon_traits.insert(WeaponTraitDto::Blessed);
            if one_in(rng, 4) {
                add_light(&mut roll.state.properties);
            }
            if one_in(rng, 4) && generation_level > 40 {
                roll.blows_pval = true;
            } else if one_in(rng, 777) && generation_level > 80 {
                roll.state.weapon_traits.insert(WeaponTraitDto::ManaBrand);
            }
        }
        13 if is_weapon => {
            roll.state.weapon_traits.insert(WeaponTraitDto::Blessed);
        }
        14 if is_weapon => {
            roll_rfb_nature(
                rng,
                &mut roll.state,
                &mut dice,
                base_kind.tval,
                base_kind.sval,
            );
        }
        15 if is_weapon => {
            if one_in(rng, 3) {
                roll.charisma_pval = true;
            }
            if one_in(rng, 5) {
                add_slay(
                    &mut roll.state.properties,
                    SlayTarget::Demon,
                    SlayLevel::Slay,
                );
            }
            if one_in(rng, 7) {
                add_one_ability(rng, &mut roll.state.properties);
            }
        }
        18 if is_weapon => {
            roll.state.enchantment_delta.to_armor = 5;
            roll_rfb_defender(rng, &mut roll.state.properties);
        }
        19 if is_weapon => {
            if one_in(rng, 3) {
                add_status_immunity(&mut roll.state.properties, "rfb.status.fear");
            }
        }
        20 if is_weapon => {
            if one_in(rng, 44) {
                add_slay(
                    &mut roll.state.properties,
                    SlayTarget::Demon,
                    SlayLevel::Kill,
                );
            } else if one_in(rng, 12) {
                add_status_immunity(&mut roll.state.properties, "rfb.status.fear");
            }
            if randint1(rng, 60_u16.saturating_add(generation_level / 10)) > 56 {
                add_slay(
                    &mut roll.state.properties,
                    SlayTarget::Evil,
                    SlayLevel::Slay,
                );
            }
        }
        22 if is_weapon => {
            if one_in(rng, 3) {
                roll.state
                    .properties
                    .passives
                    .insert(EquipmentPassive::HoldLife);
            }
            if one_in(rng, 3) {
                roll.dexterity_pval = true;
            }
            if one_in(rng, 5) {
                add_status_immunity(&mut roll.state.properties, "rfb.status.fear");
            }
        }
        _ => return None,
    }

    if is_weapon
        && dice == base_dice
        && !matches!(source_index, 5 | 16 | 17)
        && dice.dice.saturating_mul(dice.sides) > 0
        && one_in(rng, 5_u16.saturating_add(200 / generation_level.max(1)))
    {
        loop {
            dice.dice = dice.dice.saturating_add(1);
            let odds = dice.dice.saturating_mul(dice.sides) / 2;
            if odds == 0 || !one_in(rng, odds) {
                break;
            }
        }
    }

    finalize_rfb_weapon_ego(
        rng,
        &mut roll,
        affix,
        source_index,
        generation_level,
        base_kind.tval,
        base_kind.sval,
        dice,
    );
    if dice != base_dice {
        roll.state.melee_damage_dice = Some(dice);
    }
    let rolled_affixes = roll
        .state
        .has_instance_state()
        .then_some(roll.state)
        .into_iter()
        .collect();
    Some(EgoMaterialization::new(
        vec![affix.id.clone()],
        rolled_affixes,
        None,
        None,
    ))
}

fn finalize_rfb_weapon_ego(
    rng: &mut RfbRng,
    roll: &mut RfbWeaponRoll,
    affix: &AffixDefinition,
    source_index: u32,
    generation_level: u16,
    tval: u16,
    sval: u16,
    dice: MeleeDamageDiceDto,
) {
    if matches!(source_index, 10 | 18) {
        add_one_sustain(rng, &mut roll.state.properties);
    }
    if matches!(source_index, 4 | 16) {
        add_one_ability(rng, &mut roll.state.properties);
    }
    if matches!(source_index, 15 | 16 | 22) {
        add_one_high_resistance(rng, &mut roll.state.properties);
        if generation_level > 0 && randint1(rng, generation_level) > 60 {
            add_one_high_resistance(rng, &mut roll.state.properties);
        }
    }
    if matches!(source_index, 8 | 16) {
        add_one_resistance(rng, &mut roll.state.properties);
    }

    let (max_to_hit, max_to_damage, max_to_armor, max_pval) = rfb_ego_maxima(source_index);
    roll.state.enchantment_delta.to_hit = roll
        .state
        .enchantment_delta
        .to_hit
        .saturating_add(roll_signed(rng, max_to_hit));
    roll.state.enchantment_delta.to_damage = roll
        .state
        .enchantment_delta
        .to_damage
        .saturating_add(roll_signed(rng, max_to_damage));
    roll.state.enchantment_delta.to_armor = roll
        .state
        .enchantment_delta
        .to_armor
        .saturating_add(roll_signed(rng, max_to_armor));

    if max_pval > 0 {
        let mut pval = match source_index {
            5 => roll_extra_attacks_pval(rng, max_pval, generation_level, dice, tval, sval),
            10 if roll.blows_pval => {
                if dice.dice.saturating_mul(dice.sides) > 30 {
                    roll.blows_pval = false;
                    randint1(rng, max_pval)
                } else {
                    let mut pval = randint1(rng, 2);
                    if tval == TV_SWORD && sval == 33 {
                        pval = pval.saturating_add(randint1(rng, 2));
                    }
                    if generation_level > 60
                        && one_in(rng, 3)
                        && dice.dice.saturating_mul(dice.sides.saturating_add(1)) < 15
                    {
                        pval = pval.saturating_add(randint1(rng, 2));
                    }
                    pval
                }
            }
            _ => randint1(rng, max_pval),
        };
        if tval == TV_SWORD && sval == 33 && pval > 2 && source_index != 5 {
            pval = 2;
        }
        apply_rfb_pval(
            &mut roll.state.properties,
            source_index,
            pval,
            roll.charisma_pval,
            roll.dexterity_pval,
            roll.blows_pval,
        );
    }

    let has_fire_brand = affix.brands.contains(&WeaponBrand::Fire)
        || roll.state.properties.brands.contains(&WeaponBrand::Fire);
    if has_fire_brand && affix.equipment_bonuses.light_radius == 0 {
        add_light(&mut roll.state.properties);
    }
    if affix.equipment_bonuses.light_radius > 0 {
        roll.state.properties.equipment_bonuses.light_radius = 0;
    }
}

fn apply_rfb_pval(
    properties: &mut AffixPropertyBundleDefinition,
    source_index: u32,
    pval: u16,
    charisma_pval: bool,
    dexterity_pval: bool,
    blows_pval: bool,
) {
    let pval_i32 = i32::from(pval);
    match source_index {
        2 | 40 | 41 => properties.equipment_bonuses.digging_skill = pval_i32,
        3 => {
            properties.modifiers.intelligence = pval_i32;
            properties.modifiers.wisdom = pval_i32;
        }
        4 => properties.modifiers.wisdom = pval_i32,
        5 => properties.equipment_bonuses.melee_attacks = pval_i32,
        10 => {
            properties.modifiers.wisdom = pval_i32;
            if blows_pval {
                properties.equipment_bonuses.melee_attacks = pval_i32;
            }
        }
        11 => {
            properties.equipment_bonuses.melee_attacks = pval_i32;
            properties.modifiers.strength = pval_i32;
            properties.modifiers.dexterity = pval_i32;
            properties.modifiers.wisdom = -pval_i32;
        }
        13 => properties.equipment_bonuses.life_percent = pval_i32.saturating_mul(3),
        14 => properties.modifiers.intelligence = pval_i32,
        15 => {
            properties.equipment_bonuses.search_skill = pval_i32;
            if charisma_pval {
                properties.modifiers.charisma = pval_i32;
            }
        }
        19 => {
            properties.modifiers.strength = pval_i32;
            properties.modifiers.dexterity = pval_i32;
            properties.modifiers.constitution = pval_i32;
        }
        22 => {
            properties.modifiers.strength = pval_i32;
            properties.modifiers.constitution = pval_i32;
            if dexterity_pval {
                properties.modifiers.dexterity = pval_i32;
            }
        }
        23 => {
            properties.modifiers.charisma = pval_i32;
            properties.modifiers.speed = pval_i32;
        }
        42 => {
            properties.equipment_bonuses.digging_skill = pval_i32;
            properties.modifiers.strength = pval_i32;
        }
        _ => {}
    }
}

fn rfb_ego_maxima(source_index: u32) -> (i16, i16, i16, u16) {
    match source_index {
        2 => (0, 0, 0, 5),
        3 => (3, 3, 0, 2),
        4 => (0, 0, 0, 3),
        5 => (0, 0, 0, 6),
        7 => (0, 10, 0, 0),
        10 => (6, 6, 0, 4),
        11 => (0, 0, 0, 3),
        13 => (0, 0, 0, 4),
        14 => (0, 0, 0, 2),
        15 => (4, 4, 0, 2),
        18 => (4, 4, 8, 0),
        19 => (5, 5, 0, 2),
        20 => (8, 8, 0, 0),
        21 => (20, 20, 10, 0),
        22 => (6, 6, 0, 3),
        23 => (10, 10, 0, 5),
        24 => (5, 5, 0, 0),
        25 => (6, 6, 0, 0),
        26 => (7, 7, 0, 0),
        27 => (10, 10, 0, 0),
        40 => (0, 0, 0, 5),
        41 => (0, 3, 0, 5),
        42 => (0, 7, 0, 5),
        _ => (0, 0, 0, 0),
    }
}

fn roll_signed(rng: &mut RfbRng, maximum: i16) -> i16 {
    if maximum == 0 {
        0
    } else if maximum < 0 {
        -i16::try_from(randint1(rng, maximum.unsigned_abs())).expect("ego penalty fits i16")
    } else {
        i16::try_from(randint1(rng, maximum as u16)).expect("ego bonus fits i16")
    }
}

fn roll_extra_attacks_pval(
    rng: &mut RfbRng,
    maximum: u16,
    generation_level: u16,
    dice: MeleeDamageDiceDto,
    tval: u16,
    sval: u16,
) -> u16 {
    let odds = 3_u16.saturating_add(dice.dice.saturating_mul(dice.sides) / 3);
    let bound = maximum
        .saturating_mul(generation_level)
        .saturating_div(100)
        .saturating_add(1);
    let mut pval = randint1(rng, bound);
    if pval > 4 && !one_in(rng, odds) {
        pval = 4;
    } else if pval > 5 && !one_in(rng, odds) {
        pval = 5;
    } else if pval > 6 {
        pval = 6;
    }
    if tval == TV_SWORD && sval == 33 {
        pval = pval.saturating_add(randint1(rng, 2));
    }
    if dice.dice.saturating_mul(dice.sides) > 30 {
        pval = pval.max(3);
    }
    pval
}

pub(super) fn roll_rfb_slaying(
    rng: &mut RfbRng,
    properties: &mut AffixPropertyBundleDefinition,
    generation_level: u16,
    is_ammunition: bool,
) {
    const SLAYS: [(SlayTarget, EquipmentPassive, u16, u16); 11] = [
        (SlayTarget::Orc, EquipmentPassive::EspOrc, 2, 20),
        (SlayTarget::Troll, EquipmentPassive::EspTroll, 2, 30),
        (SlayTarget::Giant, EquipmentPassive::EspGiant, 2, 40),
        (SlayTarget::Dragon, EquipmentPassive::EspDragon, 3, 80),
        (SlayTarget::Demon, EquipmentPassive::EspDemon, 3, 90),
        (SlayTarget::Undead, EquipmentPassive::EspUndead, 3, 95),
        (SlayTarget::Animal, EquipmentPassive::EspAnimal, 2, 60),
        (SlayTarget::Human, EquipmentPassive::EspHuman, 3, 50),
        (SlayTarget::Evil, EquipmentPassive::EspEvil, 5, 0),
        (SlayTarget::Good, EquipmentPassive::EspGood, 5, 0),
        (SlayTarget::Living, EquipmentPassive::EspLiving, 20, 0),
    ];
    let eligible = SLAYS
        .iter()
        .copied()
        .filter(|(_, _, _, maximum)| *maximum == 0 || generation_level <= *maximum)
        .collect::<Vec<_>>();
    let total = eligible
        .iter()
        .map(|(_, _, rarity, _)| (255 / u32::from(*rarity)).max(1))
        .sum::<u32>();
    let mut rolls = 1_u16.saturating_add(rfb_m_bonus(rng, 4, generation_level));
    if one_in(rng, 8) {
        rolls = rolls.saturating_mul(2);
    }
    if is_ammunition {
        rolls = rolls.saturating_add(1) / 2;
    }
    for _ in 0..rolls {
        let mut choice = u32::try_from(rng.bounded(u64::from(total))).expect("slay roll fits u32");
        let (target, esp, rarity, _) = eligible
            .iter()
            .copied()
            .find(|(_, _, rarity, _)| {
                let weight = (255 / u32::from(*rarity)).max(1);
                if choice < weight {
                    true
                } else {
                    choice -= weight;
                    false
                }
            })
            .expect("positive slaying weight selects a target");
        let kill_odds = rarity
            .saturating_mul(rarity)
            .saturating_mul(if is_ammunition { rarity } else { 1 });
        if one_in(rng, kill_odds) {
            add_slay(properties, target, SlayLevel::Kill);
            if !is_ammunition {
                properties.passives.insert(esp);
            }
        } else {
            add_slay(properties, target, SlayLevel::Slay);
            if !is_ammunition && one_in(rng, 6) {
                properties.passives.insert(esp);
            }
        }
    }
}

pub(super) fn roll_rfb_craft(
    rng: &mut RfbRng,
    state: &mut RolledAffixState,
    generation_level: u16,
    is_ammunition: bool,
) {
    let mut rolls = 1_u16.saturating_add(rfb_m_bonus(rng, 4, generation_level));
    if one_in(rng, 8) {
        rolls = rolls.saturating_mul(2);
    }
    if is_ammunition {
        rolls = rolls.saturating_add(1) / 2;
    }
    for roll_index in 0..rolls {
        let (brand, resistance) = match rng.bounded(5) {
            0 => (WeaponBrand::Acid, ActorDamageType::Acid),
            1 => (WeaponBrand::Electricity, ActorDamageType::Electricity),
            2 => (WeaponBrand::Fire, ActorDamageType::Fire),
            3 => (WeaponBrand::Cold, ActorDamageType::Cold),
            _ => (WeaponBrand::Poison, ActorDamageType::Poison),
        };
        state.properties.brands.insert(brand);
        if roll_index == 0 && one_in(rng, 2) && !is_ammunition {
            add_resistance(&mut state.properties, resistance);
            break;
        }
        if one_in(rng, 3) && !is_ammunition {
            add_resistance(&mut state.properties, resistance);
        }
    }
    if one_in(rng, 6) && generation_level > 60 && !is_ammunition {
        state.weapon_traits.insert(WeaponTraitDto::ManaBrand);
    }
}

fn roll_rfb_nature(
    rng: &mut RfbRng,
    state: &mut RolledAffixState,
    dice: &mut MeleeDamageDiceDto,
    tval: u16,
    sval: u16,
) {
    if one_in(rng, 5) {
        add_slay(&mut state.properties, SlayTarget::Animal, SlayLevel::Kill);
    }
    if one_in(rng, 3) {
        state.properties.brands.insert(WeaponBrand::Electricity);
    }
    if one_in(rng, 3) {
        state.properties.brands.insert(WeaponBrand::Fire);
        if tval == TV_HAFTED && sval == SV_WHIP {
            dice.dice = dice.dice.saturating_add(1);
        }
    }
    if one_in(rng, 3) {
        state.properties.brands.insert(WeaponBrand::Cold);
    }
    for damage_type in [
        ActorDamageType::Electricity,
        ActorDamageType::Fire,
        ActorDamageType::Cold,
    ] {
        if one_in(rng, 3) {
            add_resistance(&mut state.properties, damage_type);
        }
    }
}

fn roll_rfb_defender(rng: &mut RfbRng, properties: &mut AffixPropertyBundleDefinition) {
    if one_in(rng, 4) {
        let mut count = 2_u16;
        while one_in(rng, 3) {
            count = count.saturating_add(1);
        }
        for _ in 0..count {
            add_one_high_resistance(rng, properties);
        }
    } else {
        let mut count = 4_u16;
        while one_in(rng, 2) {
            count = count.saturating_add(1);
        }
        for _ in 0..count {
            add_one_elemental_resistance(rng, properties);
        }
    }
    if one_in(rng, 3) {
        properties.passives.insert(EquipmentPassive::Warning);
    }
    if one_in(rng, 3) {
        properties.passives.insert(EquipmentPassive::Levitation);
    }
    if one_in(rng, 5) {
        properties.passives.insert(EquipmentPassive::Regeneration);
    }
}

fn add_one_sustain(rng: &mut RfbRng, properties: &mut AffixPropertyBundleDefinition) {
    properties.passives.insert(match rng.bounded(6) {
        0 => EquipmentPassive::SustainStrength,
        1 => EquipmentPassive::SustainIntelligence,
        2 => EquipmentPassive::SustainWisdom,
        3 => EquipmentPassive::SustainDexterity,
        4 => EquipmentPassive::SustainConstitution,
        _ => EquipmentPassive::SustainCharisma,
    });
}

fn add_one_high_resistance(rng: &mut RfbRng, properties: &mut AffixPropertyBundleDefinition) {
    match rng.bounded(12) {
        0 => add_resistance(properties, ActorDamageType::Poison),
        1 => add_resistance(properties, ActorDamageType::Light),
        2 => add_resistance(properties, ActorDamageType::Dark),
        3 => add_resistance(properties, ActorDamageType::Shards),
        4 => add_status_immunity(properties, "rfb.status.blindness"),
        5 => add_resistance(properties, ActorDamageType::Confusion),
        6 => add_resistance(properties, ActorDamageType::Sound),
        7 => add_resistance(properties, ActorDamageType::Nether),
        8 => add_resistance(properties, ActorDamageType::Nexus),
        9 => add_resistance(properties, ActorDamageType::Chaos),
        10 => add_resistance(properties, ActorDamageType::Disenchant),
        _ => add_status_immunity(properties, "rfb.status.fear"),
    }
}

fn add_one_elemental_resistance(rng: &mut RfbRng, properties: &mut AffixPropertyBundleDefinition) {
    let damage_type = match rng.bounded(4) {
        0 => ActorDamageType::Acid,
        1 => ActorDamageType::Electricity,
        2 => ActorDamageType::Cold,
        _ => ActorDamageType::Fire,
    };
    add_resistance(properties, damage_type);
}

fn add_one_resistance(rng: &mut RfbRng, properties: &mut AffixPropertyBundleDefinition) {
    if one_in(rng, 3) {
        add_one_elemental_resistance(rng, properties);
    } else {
        add_one_high_resistance(rng, properties);
    }
}

fn add_one_ability(rng: &mut RfbRng, properties: &mut AffixPropertyBundleDefinition) {
    match rng.bounded(10) {
        0 => {
            properties.passives.insert(EquipmentPassive::Levitation);
        }
        1 => add_light(properties),
        2 => {
            properties.passives.insert(EquipmentPassive::SeeInvisible);
        }
        3 => {
            properties.passives.insert(EquipmentPassive::Warning);
        }
        4 => {
            properties.passives.insert(EquipmentPassive::SlowDigestion);
        }
        5 => {
            properties.passives.insert(EquipmentPassive::Regeneration);
        }
        6 => add_status_immunity(properties, "rfb.status.paralysis"),
        7 => {
            properties.passives.insert(EquipmentPassive::HoldLife);
        }
        _ => add_one_low_esp(rng, properties),
    }
}

fn add_one_low_esp(rng: &mut RfbRng, properties: &mut AffixPropertyBundleDefinition) {
    properties.passives.insert(match rng.bounded(9) {
        0 => EquipmentPassive::EspAnimal,
        1 => EquipmentPassive::EspUndead,
        2 => EquipmentPassive::EspDemon,
        3 => EquipmentPassive::EspOrc,
        4 => EquipmentPassive::EspTroll,
        5 => EquipmentPassive::EspGiant,
        6 => EquipmentPassive::EspDragon,
        7 => EquipmentPassive::EspHuman,
        _ => EquipmentPassive::EspGood,
    });
}

fn add_slay(properties: &mut AffixPropertyBundleDefinition, target: SlayTarget, level: SlayLevel) {
    properties
        .slays
        .entry(target)
        .and_modify(|current| *current = (*current).max(level))
        .or_insert(level);
}

fn add_resistance(properties: &mut AffixPropertyBundleDefinition, damage_type: ActorDamageType) {
    properties
        .resistances
        .entry(damage_type)
        .or_insert(ActorResistanceLevel::Resistant);
}

fn add_status_immunity(properties: &mut AffixPropertyBundleDefinition, status_id: &str) {
    if !properties
        .status_immunities
        .iter()
        .any(|id| id == status_id)
    {
        properties.status_immunities.push(status_id.to_owned());
    }
}

fn add_light(properties: &mut AffixPropertyBundleDefinition) {
    properties.equipment_bonuses.light_radius = properties.equipment_bonuses.light_radius.max(1);
}

fn one_in(rng: &mut RfbRng, odds: u16) -> bool {
    debug_assert!(odds > 0);
    rng.bounded(u64::from(odds)) == 0
}

fn randint1(rng: &mut RfbRng, maximum: u16) -> u16 {
    debug_assert!(maximum > 0);
    u16::try_from(rng.bounded(u64::from(maximum))).expect("bounded roll fits u16") + 1
}

const fn actor_resistance_rank(level: ActorResistanceLevel) -> u8 {
    match level {
        ActorResistanceLevel::Vulnerable => 0,
        ActorResistanceLevel::Resistant => 1,
        ActorResistanceLevel::Strong => 2,
        ActorResistanceLevel::Immune => 3,
    }
}

fn roll_rfb_ego_from_affixes<'a>(
    affixes: impl IntoIterator<Item = &'a AffixDefinition>,
    rng: &mut RfbRng,
    generation_level: u16,
    allowed_types: &[RfbEgoTypeDefinition],
) -> Option<&'a str> {
    let mut candidates = affixes
        .into_iter()
        .filter_map(|affix| {
            let ego = affix.rfb_ego.as_ref()?;
            if ego.rarity == 0
                || !ego
                    .types
                    .iter()
                    .any(|ego_type| allowed_types.contains(ego_type))
            {
                return None;
            }
            Some((
                ego.source_index,
                affix.id.as_str(),
                rfb_ego_weight(
                    ego.rarity,
                    affix.generation_level,
                    affix.generation_max_level,
                    generation_level,
                ),
            ))
        })
        .collect::<Vec<_>>();
    candidates.sort_unstable_by_key(|candidate| candidate.0);
    if candidates.is_empty() {
        return None;
    }

    let weights = candidates
        .iter()
        .map(|candidate| candidate.2)
        .collect::<Vec<_>>();
    let selected = roll_weighted_index_with_rng(rng, &weights);
    Some(candidates[selected].1)
}

fn rfb_ego_weight(rarity: u16, min_level: u16, max_level: u16, level: u16) -> u32 {
    debug_assert!(rarity > 0);
    let rarity = u64::from(rarity);
    let adjusted_rarity = if level > max_level {
        rarity + 3 * rarity * u64::from(level - max_level)
    } else if level < min_level {
        rarity + rarity * u64::from(min_level - level)
    } else {
        rarity
    };
    u32::try_from((10_000 / adjusted_rarity).max(1))
        .expect("authoritative ego weight never exceeds 10000")
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use crate::game::Game;
    use rfb_content::{
        AffixDefinition, EquipmentBonuses, ItemDefinition, RfbBaseKindDefinition,
        RfbEgoGenerationDefinition, StatModifiers,
    };
    use rfb_protocol::{ItemCurseEffectDto, ItemQualityDto, MeleeDamageDiceDto, WeaponTraitDto};

    use crate::state::ItemLocation;

    use super::*;

    fn ego_affix(
        id: &str,
        source_index: u32,
        rarity: u16,
        min_level: u16,
        max_level: u16,
        types: Vec<RfbEgoTypeDefinition>,
    ) -> AffixDefinition {
        AffixDefinition {
            schema: String::new(),
            format_version: 1,
            id: id.to_owned(),
            name_key: "duplicate-english-name".to_owned(),
            description_key: "test-description".to_owned(),
            generation_level: min_level,
            generation_max_level: max_level,
            rfb_ego: Some(RfbEgoGenerationDefinition {
                source_index,
                rarity,
                types,
            }),
            name_placement: Default::default(),
            modifiers: StatModifiers::default(),
            equipment_bonuses: EquipmentBonuses::default(),
            resistances: BTreeMap::new(),
            status_immunities: Vec::new(),
            slays: BTreeMap::new(),
            brands: BTreeSet::new(),
            passives: BTreeSet::new(),
            elemental_destruction_vulnerabilities: BTreeSet::new(),
            elemental_destruction_immunities: BTreeSet::new(),
            resists_projection_destruction: false,
            resists_monster_destruction: false,
            protects_quiver_ammunition: false,
            device_generation: None,
            preserves_ordinary_quality: false,
            roll_groups: Vec::new(),
            tags: Vec::new(),
        }
    }

    fn rfb_weapon_item(tval: u16, sval: u16) -> ItemDefinition {
        let game = Game::new(1);
        let mut item = game
            .content
            .item("demo.item.long-sword")
            .expect("test weapon exists")
            .clone();
        item.rfb_base_kind = Some(RfbBaseKindDefinition {
            source_index: 60,
            tval,
            sval,
        });
        item
    }

    #[test]
    fn basic_weapon_egos_materialize_fixed_seed_results() {
        let item = rfb_weapon_item(TV_SWORD, 17);
        for (source_index, seed) in [(1, 37), (5, 81), (9, 53), (18, 67)] {
            let affix = ego_affix(
                &format!("test.affix.{source_index}"),
                source_index,
                1,
                0,
                u16::MAX,
                vec![RfbEgoTypeDefinition::Weapon],
            );
            let mut rng = RfbRng::seeded(seed);
            let materialized = materialize_rfb_weapon_ego_with_rng(&mut rng, &item, &affix, 80)
                .expect("basic weapon ego should materialize");
            let rolled = &materialized.rolled_affixes[0];
            match source_index {
                1 => {
                    assert_eq!(
                        rolled.properties.slays,
                        BTreeMap::from([
                            (SlayTarget::Undead, SlayLevel::Slay),
                            (SlayTarget::Demon, SlayLevel::Slay),
                            (SlayTarget::Dragon, SlayLevel::Kill),
                        ])
                    );
                    assert_eq!(
                        rolled.properties.passives,
                        BTreeSet::from([EquipmentPassive::EspDemon, EquipmentPassive::EspDragon])
                    );
                    assert_eq!(rng.draw_counter, 20);
                }
                5 => {
                    assert_eq!(rolled.properties.equipment_bonuses.melee_attacks, 3);
                    assert_eq!(rng.draw_counter, 1);
                }
                9 => {
                    assert_eq!(
                        rolled.properties.brands,
                        BTreeSet::from([
                            WeaponBrand::Electricity,
                            WeaponBrand::Cold,
                            WeaponBrand::Poison,
                        ])
                    );
                    assert_eq!(
                        rolled.properties.resistances,
                        BTreeMap::from([
                            (
                                ActorDamageType::Electricity,
                                ActorResistanceLevel::Resistant
                            ),
                            (ActorDamageType::Cold, ActorResistanceLevel::Resistant),
                        ])
                    );
                    assert_eq!(rng.draw_counter, 18);
                }
                18 => {
                    assert_eq!(
                        rolled.enchantment_delta,
                        ItemEnchantmentsDto {
                            to_hit: 1,
                            to_damage: 1,
                            to_armor: 12,
                        }
                    );
                    assert_eq!(
                        rolled.properties.passives,
                        BTreeSet::from([
                            EquipmentPassive::Warning,
                            EquipmentPassive::SustainIntelligence,
                        ])
                    );
                    assert_eq!(
                        rolled.properties.status_immunities,
                        ["rfb.status.blindness"]
                    );
                    assert_eq!(rng.draw_counter, 12);
                }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn fire_brand_adds_light_and_c_rolls_are_independent() {
        let item = rfb_weapon_item(TV_SWORD, 17);
        let mut affix = ego_affix(
            "test.affix.force",
            3,
            1,
            0,
            u16::MAX,
            vec![RfbEgoTypeDefinition::Weapon],
        );
        affix.brands.insert(WeaponBrand::Fire);
        let mut rng = RfbRng::seeded(29);
        let materialized =
            materialize_rfb_weapon_ego_with_rng(&mut rng, &item, &affix, 60).unwrap();
        let rolled = &materialized.rolled_affixes[0];

        assert_eq!(
            rolled.enchantment_delta,
            ItemEnchantmentsDto {
                to_hit: 3,
                to_damage: 2,
                to_armor: 0,
            }
        );
        assert_eq!(rolled.properties.modifiers.intelligence, 2);
        assert_eq!(rolled.properties.modifiers.wisdom, 2);
        assert_eq!(rolled.properties.equipment_bonuses.light_radius, 1);
        assert_eq!(
            rolled.weapon_traits,
            BTreeSet::from([WeaponTraitDto::ManaBrand])
        );
    }

    #[test]
    fn ego_weight_matches_below_in_range_and_above_max_penalties() {
        assert_eq!(rfb_ego_weight(4, 10, 20, 5), 416);
        assert_eq!(rfb_ego_weight(4, 10, 20, 15), 2_500);
        assert_eq!(rfb_ego_weight(4, 10, 20, 25), 156);
    }

    #[test]
    fn ego_materialization_preserves_roll_then_activation_rng_order() {
        let game = Game::new(91);
        let affix_ids = vec!["rfb-legacy.affix.olog-hai".to_owned()];
        let mut expected_rng = RfbRng::seeded(91);
        let expected_rolls =
            roll_affix_properties_with_rng(&game.content, &mut expected_rng, &affix_ids, |_| 36);
        let (expected_activation, expected_charges) = initial_item_runtime_state(
            &game.content,
            &mut expected_rng,
            "demo.item.metal-lamellar-armour",
            &affix_ids,
            36,
        );

        let mut rng = RfbRng::seeded(91);
        let materialized = materialize_ego_with_rng(
            &game.content,
            &mut rng,
            "demo.item.metal-lamellar-armour",
            affix_ids.clone(),
            |_| 36,
            36,
        );

        assert_eq!(materialized.affix_ids, affix_ids);
        assert_eq!(materialized.rolled_affixes, expected_rolls);
        assert_eq!(materialized.activation, expected_activation);
        assert_eq!(materialized.charges, expected_charges);
        assert_eq!(rng, expected_rng);
    }

    #[test]
    fn ego_materialization_commits_complete_instance_state_only_after_success() {
        let mut item = ItemInstance {
            id: "test.item.weapon".to_owned(),
            kind_id: "demo.item.long-sword".to_owned(),
            quantity: 1,
            inscription: None,
            origin_actor_kind_id: None,
            origin_kind: None,
            damage_dice_override: None,
            discount_percent: 0,
            quality: ItemQualityDto::Fine,
            affix_ids: Vec::new(),
            rolled_affixes: Vec::new(),
            enchantments: ItemEnchantmentsDto {
                to_hit: 1,
                to_damage: 2,
                to_armor: 0,
            },
            curse: None,
            permanent_destruction_immunities: BTreeSet::new(),
            activation: None,
            charges: None,
            fuel: None,
            device_recovery_progress: 0,
            captured_actor: None,
            location: ItemLocation::Inventory,
        };
        let before_failed_selection = item.clone();
        let mut failed_rng = RfbRng::seeded(33);
        let rng_before = failed_rng.clone();
        assert_eq!(
            roll_rfb_ego_from_affixes(
                std::iter::empty::<&AffixDefinition>(),
                &mut failed_rng,
                30,
                &[RfbEgoTypeDefinition::Weapon],
            ),
            None
        );
        assert_eq!(item, before_failed_selection);
        assert_eq!(failed_rng, rng_before);

        let rolled = RolledAffixState {
            affix_id: "rfb-legacy.affix.slaying".to_owned(),
            enchantment_delta: ItemEnchantmentsDto {
                to_hit: 3,
                to_damage: -1,
                to_armor: 0,
            },
            melee_damage_dice: Some(MeleeDamageDiceDto { dice: 4, sides: 6 }),
            weapon_traits: BTreeSet::from([WeaponTraitDto::Vorpal, WeaponTraitDto::Impact]),
            curse_effects: BTreeSet::from([ItemCurseEffectDto::Aggravate]),
            ..RolledAffixState::default()
        };
        let materialization = EgoMaterialization::new(
            vec![rolled.affix_id.clone()],
            vec![rolled.clone()],
            None,
            None,
        );
        assert_eq!(materialization.enchantment_delta, rolled.enchantment_delta);
        assert_eq!(materialization.melee_damage_dice, rolled.melee_damage_dice);
        assert_eq!(materialization.weapon_traits, rolled.weapon_traits);
        assert_eq!(materialization.curse_effects, rolled.curse_effects);

        materialization.apply_to(&mut item);
        assert_eq!(
            item.affix_ids.as_slice(),
            std::slice::from_ref(&rolled.affix_id)
        );
        assert_eq!(item.rolled_affixes, [rolled]);
        assert_eq!(
            item.enchantments,
            ItemEnchantmentsDto {
                to_hit: 4,
                to_damage: 1,
                to_armor: 0,
            }
        );
    }

    #[test]
    fn rolled_weapon_ego_state_round_trips_without_rng_draws() {
        let game = Game::new(57);
        let mut item = ItemInstance {
            id: "test.item.weapon".to_owned(),
            kind_id: "demo.item.long-sword".to_owned(),
            quantity: 1,
            inscription: None,
            origin_actor_kind_id: None,
            origin_kind: None,
            damage_dice_override: None,
            discount_percent: 0,
            quality: ItemQualityDto::Fine,
            affix_ids: vec!["rfb-legacy.affix.slaying".to_owned()],
            rolled_affixes: vec![RolledAffixState {
                affix_id: "rfb-legacy.affix.slaying".to_owned(),
                enchantment_delta: ItemEnchantmentsDto {
                    to_hit: 2,
                    to_damage: 5,
                    to_armor: 0,
                },
                melee_damage_dice: Some(MeleeDamageDiceDto { dice: 5, sides: 5 }),
                weapon_traits: BTreeSet::from([WeaponTraitDto::ManaBrand, WeaponTraitDto::Order]),
                curse_effects: BTreeSet::from([
                    ItemCurseEffectDto::DrainExperience,
                    ItemCurseEffectDto::Teleport,
                ]),
                ..RolledAffixState::default()
            }],
            enchantments: ItemEnchantmentsDto {
                to_hit: 2,
                to_damage: 5,
                to_armor: 0,
            },
            curse: None,
            permanent_destruction_immunities: BTreeSet::new(),
            activation: None,
            charges: None,
            fuel: None,
            device_recovery_progress: 0,
            captured_actor: None,
            location: ItemLocation::Inventory,
        };
        let rng_before = game.rng.clone();
        let saved = crate::save::inventory_to_save(std::slice::from_ref(&item));
        let restored = crate::save::inventory_item_from_dto(saved[0].clone(), &game.content)
            .expect("weapon ego instance state should round-trip");
        assert_eq!(restored.rolled_affixes, item.rolled_affixes);
        assert_eq!(restored.enchantments, item.enchantments);
        assert_eq!(game.rng, rng_before);

        item.rolled_affixes[0].melee_damage_dice = Some(MeleeDamageDiceDto { dice: 0, sides: 5 });
        let invalid = crate::save::inventory_to_save(std::slice::from_ref(&item));
        assert!(crate::save::inventory_item_from_dto(invalid[0].clone(), &game.content).is_err());
    }

    #[test]
    fn ego_selection_uses_source_order_for_duplicate_names_and_one_draw() {
        let affixes = [
            ego_affix(
                "test.affix.source-20",
                20,
                10,
                0,
                u16::MAX,
                vec![RfbEgoTypeDefinition::Weapon],
            ),
            ego_affix(
                "test.affix.source-10",
                10,
                10,
                0,
                u16::MAX,
                vec![RfbEgoTypeDefinition::Weapon],
            ),
        ];
        let mut rng = RfbRng::seeded(7);
        let mut expected_rng = rng.clone();
        let expected = if expected_rng.bounded(2_000) < 1_000 {
            "test.affix.source-10"
        } else {
            "test.affix.source-20"
        };

        assert_eq!(
            roll_rfb_ego_from_affixes(
                affixes.iter(),
                &mut rng,
                10,
                &[RfbEgoTypeDefinition::Weapon],
            ),
            Some(expected)
        );
        assert_eq!(rng, expected_rng);
    }

    #[test]
    fn ego_selection_matches_any_type_and_excludes_zero_rarity_or_missing_metadata() {
        let mut custom_affix = ego_affix(
            "test.affix.custom",
            99,
            1,
            0,
            u16::MAX,
            vec![RfbEgoTypeDefinition::Digger],
        );
        custom_affix.rfb_ego = None;
        let affixes = [
            custom_affix,
            ego_affix(
                "test.affix.forced-only",
                1,
                0,
                0,
                u16::MAX,
                vec![RfbEgoTypeDefinition::Digger],
            ),
            ego_affix(
                "test.affix.multi-type",
                2,
                4,
                0,
                u16::MAX,
                vec![RfbEgoTypeDefinition::Weapon, RfbEgoTypeDefinition::Digger],
            ),
        ];
        let mut rng = RfbRng::seeded(11);

        assert_eq!(
            roll_rfb_ego_from_affixes(
                affixes.iter(),
                &mut rng,
                10,
                &[RfbEgoTypeDefinition::Digger],
            ),
            Some("test.affix.multi-type")
        );
        assert_eq!(rng.draw_counter, 1);

        let mut zero_only_rng = RfbRng::seeded(11);
        let unchanged = zero_only_rng.clone();
        assert_eq!(
            roll_rfb_ego_from_affixes(
                affixes[..2].iter(),
                &mut zero_only_rng,
                10,
                &[RfbEgoTypeDefinition::Digger],
            ),
            None
        );
        assert_eq!(zero_only_rng, unchanged);
    }
}
