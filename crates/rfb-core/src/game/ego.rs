// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeSet;

use rfb_content::{
    ActorDamageType, ActorResistanceLevel, AffixDefinition, AffixPropertyBundleDefinition,
    ContentCatalog, EquipmentPassive, ItemDefinition, ItemDeviceActivationDefinition,
    ItemDeviceGenerationDefinition, RfbActivationBiasDefinition, RfbEgoTypeDefinition, SlayLevel,
    SlayTarget, StatModifiers, WeaponBrand,
};
use rfb_protocol::{
    ItemActivationDto, ItemChargesDto, ItemCurseEffectDto, ItemCurseSeverityDto,
    ItemEnchantmentsDto, MeleeDamageDiceDto, WeaponTraitDto,
};

use crate::{
    rng::{RfbRng, rfb_m_bonus},
    state::{ItemInstance, RolledAffixState},
};

use super::{
    initial_item_runtime_state, merge_equipment_bonuses, roll_weighted_index_with_rng,
    target_spec_dto,
};

/// Complete generated affix state shared by content-driven consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EgoMaterialization {
    pub(super) affix_ids: Vec<String>,
    pub(super) rolled_affixes: Vec<RolledAffixState>,
    pub(super) intrinsic_properties: Option<AffixPropertyBundleDefinition>,
    pub(super) enchantment_delta: ItemEnchantmentsDto,
    pub(super) melee_damage_dice: Option<MeleeDamageDiceDto>,
    pub(super) ammunition_damage_dice: Option<u16>,
    pub(super) weapon_traits: BTreeSet<WeaponTraitDto>,
    pub(super) curse_effects: BTreeSet<ItemCurseEffectDto>,
    pub(super) curse: Option<ItemCurseSeverityDto>,
    pub(super) activation: Option<ItemActivationDto>,
    pub(super) charges: Option<ItemChargesDto>,
}

impl EgoMaterialization {
    pub(super) fn new(
        affix_ids: Vec<String>,
        rolled_affixes: Vec<RolledAffixState>,
        intrinsic_properties: Option<AffixPropertyBundleDefinition>,
        curse: Option<ItemCurseSeverityDto>,
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
            intrinsic_properties,
            enchantment_delta,
            melee_damage_dice,
            ammunition_damage_dice: None,
            weapon_traits,
            curse_effects,
            curse,
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
        if let Some(properties) = self.intrinsic_properties {
            item.intrinsic_properties = properties;
        }
        item.enchantments = enchantments;
        item.damage_dice_override = self.ammunition_damage_dice;
        if let Some(curse) = self.curse {
            item.curse = Some(item.curse.map_or(curse, |current| current.max(curse)));
        }
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
    EgoMaterialization::new(affix_ids, rolled_affixes, None, None, activation, charges)
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

pub(super) fn merge_affix_properties(
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

const TV_SHOT: u16 = 16;
const TV_ARROW: u16 = 17;
const TV_BOLT: u16 = 18;
const TV_DIGGING: u16 = 20;
const TV_HAFTED: u16 = 21;
const TV_POLEARM: u16 = 22;
const TV_SWORD: u16 = 23;
const TV_BOW: u16 = 19;
const SV_WHIP: u16 = 2;
const SV_WAR_HAMMER: u16 = 8;
const SV_WIZSTAFF: u16 = 21;
const SV_LANCE: u16 = 20;
const SV_HEAVY_LANCE: u16 = 29;
const SV_LONG_SWORD: u16 = 17;
const SV_KATANA: u16 = 20;
const SV_BLADE_OF_CHAOS: u16 = 30;
const SV_DIAMOND_EDGE: u16 = 31;
const SV_FALCON_SWORD: u16 = 33;
const SV_MATTOCK: u16 = 7;
const SV_SLING: u16 = 2;
const SV_LONG_BOW: u16 = 13;
const SV_HEAVY_XBOW: u16 = 24;
const SV_HARP: u16 = 70;

/// Rolls the generation-time property carried by every ordinary RFB Harp.
pub(super) fn materialize_rfb_harp_intrinsic_with_rng(
    rng: &mut RfbRng,
    item: &ItemDefinition,
    generation_level: u16,
) -> Option<AffixPropertyBundleDefinition> {
    let base_kind = item.rfb_base_kind?;
    if base_kind.tval != TV_BOW || base_kind.sval != SV_HARP {
        return None;
    }
    Some(AffixPropertyBundleDefinition {
        modifiers: StatModifiers {
            charisma: i32::from(1_u16.saturating_add(rfb_m_bonus(rng, 1, generation_level))),
            ..StatModifiers::default()
        },
        ..AffixPropertyBundleDefinition::default()
    })
}

/// Materializes one selected RFB Harp ego from the already rolled base pval.
pub(crate) fn materialize_rfb_harp_ego(
    item: &ItemDefinition,
    affix: &AffixDefinition,
    intrinsic_properties: &AffixPropertyBundleDefinition,
) -> Option<EgoMaterialization> {
    let source_index = affix.rfb_ego.as_ref()?.source_index;
    let base_kind = item.rfb_base_kind?;
    if base_kind.tval != TV_BOW || base_kind.sval != SV_HARP {
        return None;
    }
    let pval = intrinsic_properties.modifiers.charisma;
    if pval <= 0 {
        return None;
    }

    let mut state = RolledAffixState {
        affix_id: affix.id.clone(),
        ..RolledAffixState::default()
    };
    match source_index {
        195 => {
            state.properties.modifiers.wisdom = pval;
            state.properties.passives.extend([
                EquipmentPassive::SustainCharisma,
                EquipmentPassive::SustainWisdom,
            ]);
            add_resistance(&mut state.properties, ActorDamageType::Dark);
        }
        196 => {
            state.properties.passives.extend([
                EquipmentPassive::SustainCharisma,
                EquipmentPassive::SustainStrength,
                EquipmentPassive::SustainConstitution,
            ]);
            add_status_immunity(&mut state.properties, "rfb.status.fear");
            add_status_immunity(&mut state.properties, "rfb.status.blindness");
        }
        _ => return None,
    }
    Some(EgoMaterialization::new(
        vec![affix.id.clone()],
        vec![state],
        None,
        None,
        None,
        None,
    ))
}

/// Materializes one selected RFB ammunition ego and the common post-ego dice
/// super-charge. The returned state is committed atomically by `apply_to`.
pub(crate) fn materialize_rfb_ammunition_ego_with_rng(
    rng: &mut RfbRng,
    item: &ItemDefinition,
    affix: &AffixDefinition,
    generation_level: u16,
) -> Option<EgoMaterialization> {
    let source_index = affix.rfb_ego.as_ref()?.source_index;
    let base_kind = item.rfb_base_kind?;
    if !matches!(base_kind.tval, TV_SHOT | TV_ARROW | TV_BOLT) {
        return None;
    }
    let ammunition = item.ammunition_profile.as_ref()?;
    let mut state = RolledAffixState {
        affix_id: affix.id.clone(),
        ..RolledAffixState::default()
    };
    match source_index {
        180 => roll_rfb_slaying(rng, &mut state.properties, generation_level, true),
        181 => roll_rfb_craft(rng, &mut state, generation_level, true),
        182 => {
            state.weapon_traits.insert(WeaponTraitDto::Blessed);
        }
        183..=185 => {}
        _ => return None,
    }

    let rolled_affixes = state
        .has_instance_state()
        .then_some(state)
        .into_iter()
        .collect();
    let mut materialized = EgoMaterialization::new(
        vec![affix.id.clone()],
        rolled_affixes,
        None,
        None,
        None,
        None,
    );
    let mut dice = ammunition.damage_dice;
    if one_in(rng, 5_u16.saturating_add(200 / generation_level.max(1))) {
        loop {
            dice = dice.saturating_add(1);
            let odds = dice
                .saturating_mul(ammunition.damage_sides)
                .saturating_div(2)
                .max(1);
            if !one_in(rng, odds) {
                break;
            }
        }
        dice = dice.min(9);
    }
    materialized.ammunition_damage_dice = (dice != ammunition.damage_dice).then_some(dice);
    Some(materialized)
}

#[derive(Debug, Default)]
struct RfbLauncherRoll {
    state: RolledAffixState,
    extra_shots: bool,
}

/// Materializes one selected RFB launcher ego without changing item state.
/// Restricted launcher egos return `None` so a caller can preserve the
/// original choose-reject-retry RNG sequence.
pub(crate) fn materialize_rfb_launcher_ego_with_rng(
    rng: &mut RfbRng,
    item: &ItemDefinition,
    affix: &AffixDefinition,
    generation_level: u16,
) -> Option<EgoMaterialization> {
    let source_index = affix.rfb_ego.as_ref()?.source_index;
    let base_kind = item.rfb_base_kind?;
    let profile = item.projectile_profile.as_ref()?;
    if base_kind.tval != TV_BOW {
        return None;
    }

    let mut roll = RfbLauncherRoll {
        state: RolledAffixState {
            affix_id: affix.id.clone(),
            ..RolledAffixState::default()
        },
        extra_shots: false,
    };
    match source_index {
        160 => roll.state.enchantment_delta.to_hit = 10,
        161 => {
            add_launcher_multiplier(
                &mut roll.state.properties,
                5_u16.saturating_add(rfb_m_bonus(rng, 20, generation_level)),
                profile.shot_energy,
            );
            roll.state.enchantment_delta.to_damage = 5;
        }
        162 => add_launcher_multiplier(
            &mut roll.state.properties,
            25_u16
                .saturating_add(randint1(rng, 25))
                .saturating_add(rfb_m_bonus(rng, 50, generation_level)),
            profile.shot_energy,
        ),
        163 => {
            let pval = 1_u16.saturating_add(rfb_m_bonus(rng, 4, generation_level));
            apply_rfb_launcher_pval(&mut roll.state.properties, source_index, pval, true);
        }
        164 => {
            if base_kind.sval != SV_LONG_BOW {
                return None;
            }
            add_launcher_multiplier(
                &mut roll.state.properties,
                25_u16
                    .saturating_add(randint1(rng, 35))
                    .saturating_add(rfb_m_bonus(rng, 50, generation_level)),
                profile.shot_energy,
            );
            if one_in(rng, 3) {
                roll.extra_shots = true;
            } else {
                add_one_high_resistance(rng, &mut roll.state.properties);
            }
        }
        165 => {
            if base_kind.sval != SV_HEAVY_XBOW {
                return None;
            }
            add_launcher_multiplier(
                &mut roll.state.properties,
                25_u16
                    .saturating_add(randint1(rng, 35))
                    .saturating_add(rfb_m_bonus(rng, 50, generation_level)),
                profile.shot_energy,
            );
            if one_in(rng, 6) {
                roll.extra_shots = true;
            } else {
                add_one_high_resistance(rng, &mut roll.state.properties);
            }
        }
        166 => {
            if base_kind.sval != SV_SLING {
                return None;
            }
            roll.extra_shots = true;
            if one_in(rng, 3) {
                add_launcher_multiplier(
                    &mut roll.state.properties,
                    25_u16
                        .saturating_add(randint1(rng, 25))
                        .saturating_add(rfb_m_bonus(rng, 50, generation_level)),
                    profile.shot_energy,
                );
            } else {
                add_one_high_resistance(rng, &mut roll.state.properties);
            }
        }
        167 => {
            if one_in(rng, 5) {
                add_esp_strong(rng, &mut roll.state.properties);
            } else {
                add_esp_weak(rng, &mut roll.state.properties, false);
            }
            if one_in(rng, 30) {
                add_slay(
                    &mut roll.state.properties,
                    SlayTarget::Animal,
                    SlayLevel::Slay,
                );
            }
        }
        _ => return None,
    }

    finalize_rfb_launcher_ego(rng, &mut roll, source_index, profile.shot_energy);
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
        None,
        None,
    ))
}

fn add_launcher_multiplier(
    properties: &mut AffixPropertyBundleDefinition,
    multiplier_percent: u16,
    shot_energy: u16,
) {
    properties
        .equipment_bonuses
        .launcher_multiplier_delta_percent = properties
        .equipment_bonuses
        .launcher_multiplier_delta_percent
        .saturating_add(
            i32::from(multiplier_percent).saturating_mul(i32::from(shot_energy)) / 10_000,
        );
}

fn finalize_rfb_launcher_ego(
    rng: &mut RfbRng,
    roll: &mut RfbLauncherRoll,
    source_index: u32,
    shot_energy: u16,
) {
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
        apply_rfb_launcher_pval(
            &mut roll.state.properties,
            source_index,
            randint1(rng, max_pval),
            roll.extra_shots,
        );
    }
    roll.state.enchantment_delta.to_damage = i16::try_from(
        i32::from(roll.state.enchantment_delta.to_damage).saturating_mul(i32::from(shot_energy))
            / 7_150,
    )
    .expect("launcher ego damage bonus fits i16");
}

fn apply_rfb_launcher_pval(
    properties: &mut AffixPropertyBundleDefinition,
    source_index: u32,
    pval: u16,
    extra_shots: bool,
) {
    let pval = i32::from(pval);
    match source_index {
        162 => properties.modifiers.strength = pval,
        163 => properties.equipment_bonuses.base_shot_delta_percent = pval.saturating_mul(15),
        164 => {
            properties.modifiers.dexterity = pval;
            properties.equipment_bonuses.stealth_skill = pval;
            if extra_shots {
                properties.equipment_bonuses.base_shot_delta_percent = pval.saturating_mul(15);
            }
        }
        165 => {
            properties.modifiers.strength = pval;
            if extra_shots {
                properties.modifiers.speed = -pval;
                properties.equipment_bonuses.stealth_skill = -pval;
                properties.equipment_bonuses.base_shot_delta_percent = pval.saturating_mul(15);
            }
        }
        166 => {
            properties.modifiers.speed = pval;
            properties.equipment_bonuses.base_shot_delta_percent = pval.saturating_mul(15);
        }
        167 => properties.equipment_bonuses.stealth_skill = pval,
        _ => {}
    }
}

#[derive(Debug, Default)]
struct RfbWeaponRoll {
    state: RolledAffixState,
    curse: Option<ItemCurseSeverityDto>,
    charisma_pval: bool,
    dexterity_pval: bool,
    blows_pval: bool,
    stealth_penalty_pval: bool,
    explicit_pval: Option<u16>,
    activation_profile_index: Option<usize>,
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
    roll.activation_profile_index = matches!(source_index, 11 | 15 | 24 | 42)
        .then(|| fixed_activation_profile_index(affix))
        .flatten();
    let mut dice = base_dice;

    match source_index {
        1 if is_weapon => {
            roll_rfb_slaying(rng, &mut roll.state.properties, generation_level, false)
        }
        2 if is_weapon => {
            if !matches!(base_kind.tval, TV_POLEARM | TV_SWORD) {
                return None;
            }
            if base_kind.tval == TV_SWORD && base_kind.sval == SV_DIAMOND_EDGE {
                if one_in(rng, 8) {
                    roll.state.weapon_traits.insert(WeaponTraitDto::Vorpal2);
                } else {
                    return None;
                }
            } else {
                if one_in(rng, 2) {
                    loop {
                        dice.dice = dice.dice.saturating_add(1);
                        if !one_in(rng, dice.dice) {
                            break;
                        }
                    }
                }
                roll.state.weapon_traits.insert(if one_in(rng, 7) {
                    WeaponTraitDto::Vorpal2
                } else {
                    WeaponTraitDto::Vorpal
                });
            }
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
        6 if is_weapon => {
            if base_kind.tval != TV_HAFTED || base_kind.sval != SV_WIZSTAFF {
                return None;
            }
            let mut pval = randint1(rng, 2);
            if one_in(rng, 30) {
                pval = pval.saturating_add(1);
            }
            roll.explicit_pval = Some(pval);
            roll.state.enchantment_delta.to_hit = -10;
            roll.state.enchantment_delta.to_damage = -10;
            roll.state.weapon_traits.insert(WeaponTraitDto::ManaBrand);
            if one_in(rng, 5) {
                roll.activation_profile_index = roll_biased_activation_profile_index(
                    rng,
                    affix,
                    RfbActivationBiasDefinition::Mage,
                    generation_level,
                );
            }
        }
        7 if is_weapon => roll_rfb_armageddon(
            rng,
            &mut roll.state,
            &mut dice,
            base_kind.tval,
            base_kind.sval,
        ),
        8 if is_weapon => {
            if one_in(rng, 5) {
                roll.activation_profile_index = roll_biased_activation_profile_index(
                    rng,
                    affix,
                    RfbActivationBiasDefinition::Chaos,
                    generation_level,
                );
            }
        }
        9 if is_weapon => {
            roll.activation_profile_index = roll_rfb_craft_with_activation(
                rng,
                &mut roll.state,
                generation_level,
                false,
                affix.device_generation.as_ref(),
            );
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
            if one_in(rng, 5) {
                roll.activation_profile_index = roll_biased_activation_profile_index(
                    rng,
                    affix,
                    RfbActivationBiasDefinition::Priestly,
                    generation_level,
                );
            }
        }
        11 if is_weapon => {
            if one_in(rng, 3) {
                add_slay(
                    &mut roll.state.properties,
                    SlayTarget::Good,
                    SlayLevel::Slay,
                );
            }
            if one_in(rng, 3) {
                roll.state.properties.brands.insert(WeaponBrand::Fire);
            }
            if one_in(rng, 6) {
                add_slay(
                    &mut roll.state.properties,
                    SlayTarget::Human,
                    SlayLevel::Slay,
                );
            }
            if one_in(rng, 5) {
                roll.state
                    .curse_effects
                    .insert(ItemCurseEffectDto::Aggravate);
            } else {
                roll.stealth_penalty_pval = true;
            }
            if one_in(rng, 5) {
                roll.activation_profile_index = roll_biased_activation_profile_index(
                    rng,
                    affix,
                    RfbActivationBiasDefinition::Demon,
                    generation_level,
                )
                .or(roll.activation_profile_index);
            }
        }
        12 if is_weapon => {
            roll_rfb_death(rng, &mut roll, &mut dice);
            if one_in(rng, 5) {
                roll.activation_profile_index = roll_biased_activation_profile_index(
                    rng,
                    affix,
                    RfbActivationBiasDefinition::Necromantic,
                    generation_level,
                );
            }
        }
        12 if is_digger => {}
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
            if one_in(rng, 5) {
                roll.activation_profile_index = roll_biased_activation_profile_index(
                    rng,
                    affix,
                    RfbActivationBiasDefinition::Ranger,
                    generation_level,
                );
            }
        }
        15 if is_weapon => {
            roll.state
                .curse_effects
                .insert(ItemCurseEffectDto::Teleport);
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
        16 if is_weapon => {
            dice.sides = dice.dice.saturating_mul(dice.sides);
            dice.dice = 1;
            roll.state.weapon_traits.insert(WeaponTraitDto::Wild);
        }
        17 if is_weapon => {
            dice.dice = dice.dice.saturating_mul(dice.sides);
            dice.sides = 1;
            roll.state.weapon_traits.insert(WeaponTraitDto::Order);
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
        21 if is_weapon => {
            roll.curse = Some(ItemCurseSeverityDto::Heavy);
            roll.state
                .curse_effects
                .insert(ItemCurseEffectDto::Aggravate);
            roll.state
                .curse_effects
                .insert(roll_rfb_heavy_curse_effect(rng));
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
        23 if is_weapon => {
            if base_kind.tval != TV_SWORD
                || base_kind.sval == SV_BLADE_OF_CHAOS
                || dice.dice.saturating_mul(dice.sides) < 10
            {
                return None;
            }
            dice.dice = dice.dice.saturating_add(1);
        }
        24 if is_weapon => {
            if !is_lance(base_kind.tval, base_kind.sval) {
                return None;
            }
            while one_in(rng, dice.dice.saturating_mul(3)) {
                dice.dice = dice.dice.saturating_add(1);
            }
            if one_in(rng, 3) {
                add_slay(
                    &mut roll.state.properties,
                    SlayTarget::Human,
                    SlayLevel::Slay,
                );
            }
        }
        25 if is_weapon => {
            if !is_lance(base_kind.tval, base_kind.sval) {
                return None;
            }
            while one_in(rng, dice.dice.saturating_mul(4)) {
                dice.dice = dice.dice.saturating_add(1);
            }
            add_one_demon_resistance(rng, &mut roll.state.properties);
            if one_in(rng, 16) {
                roll.state
                    .properties
                    .passives
                    .insert(EquipmentPassive::Vampiric);
            }
            if one_in(rng, 5) {
                roll.activation_profile_index = roll_biased_activation_profile_index(
                    rng,
                    affix,
                    RfbActivationBiasDefinition::Demon,
                    generation_level,
                );
            }
        }
        26 if is_weapon => {
            if !is_lance(base_kind.tval, base_kind.sval) {
                return None;
            }
            while one_in(rng, dice.dice.saturating_mul(5)) {
                dice.dice = dice.dice.saturating_add(1);
            }
            add_one_holy_resistance(rng, &mut roll.state.properties);
            roll.state.weapon_traits.insert(WeaponTraitDto::Blessed);
            if one_in(rng, 77) {
                dice.dice = dice.dice.saturating_mul(dice.sides);
                dice.sides = 1;
                roll.state.weapon_traits.insert(WeaponTraitDto::Order);
            }
            if one_in(rng, 5) {
                roll.activation_profile_index = roll_biased_activation_profile_index(
                    rng,
                    affix,
                    RfbActivationBiasDefinition::Priestly,
                    generation_level,
                );
            }
        }
        27 if is_weapon => {
            if base_kind.tval != TV_SWORD {
                return None;
            }
            roll_rfb_troika(rng, &mut roll, &mut dice, generation_level);
        }
        40 if is_digger => {}
        41 if is_digger => {
            dice.dice = dice.dice.saturating_add(1);
        }
        42 if is_digger => {
            if base_kind.sval != SV_MATTOCK {
                return None;
            }
            dice.dice = dice.dice.saturating_add(2);
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
    let (activation, charges) = roll
        .activation_profile_index
        .and_then(|index| affix.device_generation.as_ref()?.activations.get(index))
        .map(materialize_rfb_activation)
        .unzip();
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
        roll.curse,
        activation,
        charges,
    ))
}

fn fixed_activation_profile_index(affix: &AffixDefinition) -> Option<usize> {
    affix
        .device_generation
        .as_ref()?
        .activations
        .iter()
        .position(|activation| activation.rfb_biases.is_empty())
}

fn roll_biased_activation_profile_index(
    rng: &mut RfbRng,
    affix: &AffixDefinition,
    bias: RfbActivationBiasDefinition,
    generation_level: u16,
) -> Option<usize> {
    let generation = affix.device_generation.as_ref()?;
    roll_biased_activation_index(rng, generation, bias, generation_level)
}

fn roll_biased_activation_index(
    rng: &mut RfbRng,
    generation: &ItemDeviceGenerationDefinition,
    bias: RfbActivationBiasDefinition,
    generation_level: u16,
) -> Option<usize> {
    let candidates = generation
        .activations
        .iter()
        .enumerate()
        .filter(|(_, activation)| {
            activation.rfb_biases.contains(&bias)
                && activation.min_depth <= generation_level
                && generation_level <= activation.max_depth
        })
        .collect::<Vec<_>>();
    let total_weight = candidates
        .iter()
        .map(|(_, activation)| u64::from(activation.weight))
        .sum::<u64>();
    if total_weight == 0 {
        return None;
    }
    let mut selection = rng.bounded(total_weight);
    candidates
        .into_iter()
        .find(|(_, activation)| {
            if selection < u64::from(activation.weight) {
                true
            } else {
                selection -= u64::from(activation.weight);
                false
            }
        })
        .map(|(index, _)| index)
}

fn materialize_rfb_activation(
    profile: &ItemDeviceActivationDefinition,
) -> (ItemActivationDto, ItemChargesDto) {
    let power = u16::try_from(profile.device_check_difficulty)
        .expect("validated RFB effect level must fit u16");
    (
        ItemActivationDto {
            profile_id: profile.id.clone(),
            name_key: profile.name_key.clone(),
            power,
            cost: profile.charges.cost,
            device_check_difficulty: profile.device_check_difficulty,
            target_spec: target_spec_dto(&profile.target),
        },
        ItemChargesDto {
            current: profile.charges.maximum,
            maximum: profile.charges.maximum,
        },
    )
}

pub(super) fn roll_and_materialize_rfb_ego_from_affixes_with_rng<'a>(
    rng: &mut RfbRng,
    item: &ItemDefinition,
    affixes: impl Iterator<Item = &'a AffixDefinition> + Clone,
    generation_level: u16,
    intrinsic_properties: Option<&AffixPropertyBundleDefinition>,
) -> Option<EgoMaterialization> {
    let base_kind = item.rfb_base_kind?;
    let allowed_type = if matches!(base_kind.tval, TV_SHOT | TV_ARROW | TV_BOLT) {
        RfbEgoTypeDefinition::Ammo
    } else if base_kind.tval == TV_DIGGING {
        RfbEgoTypeDefinition::Digger
    } else if matches!(base_kind.tval, TV_HAFTED | TV_POLEARM | TV_SWORD) {
        RfbEgoTypeDefinition::Weapon
    } else if base_kind.tval == TV_BOW && base_kind.sval == SV_HARP {
        RfbEgoTypeDefinition::Harp
    } else if base_kind.tval == TV_BOW {
        RfbEgoTypeDefinition::Bow
    } else {
        return None;
    };
    if allowed_type == RfbEgoTypeDefinition::Harp
        && intrinsic_properties.is_none_or(|properties| properties.modifiers.charisma <= 0)
    {
        return None;
    }
    if !affixes.clone().any(|affix| {
        affix.rfb_ego.as_ref().is_some_and(|ego| {
            ego.types.contains(&allowed_type)
                && rfb_ego_can_apply_to_base(ego.source_index, base_kind.tval, base_kind.sval, item)
        })
    }) {
        return None;
    }

    loop {
        let affix_id = roll_rfb_ego_from_affixes(
            affixes.clone(),
            rng,
            generation_level,
            std::slice::from_ref(&allowed_type),
        )?;
        let affix = affixes
            .clone()
            .find(|affix| affix.id == affix_id)
            .expect("selected ego affix remains available");
        let materialized = match allowed_type {
            RfbEgoTypeDefinition::Ammo => {
                materialize_rfb_ammunition_ego_with_rng(rng, item, affix, generation_level)
            }
            RfbEgoTypeDefinition::Bow => {
                materialize_rfb_launcher_ego_with_rng(rng, item, affix, generation_level)
            }
            RfbEgoTypeDefinition::Harp => intrinsic_properties
                .and_then(|properties| materialize_rfb_harp_ego(item, affix, properties)),
            _ => materialize_rfb_weapon_ego_with_rng(rng, item, affix, generation_level),
        };
        if let Some(materialized) = materialized {
            return Some(materialized);
        }
    }
}

fn rfb_ego_can_apply_to_base(
    source_index: u32,
    tval: u16,
    sval: u16,
    item: &ItemDefinition,
) -> bool {
    let dice_product = item
        .melee_profile
        .as_ref()
        .map(|profile| profile.damage_dice.saturating_mul(profile.damage_sides))
        .unwrap_or_default();
    match source_index {
        2 => matches!(tval, TV_POLEARM | TV_SWORD),
        6 => tval == TV_HAFTED && sval == SV_WIZSTAFF,
        23 => tval == TV_SWORD && sval != SV_BLADE_OF_CHAOS && dice_product >= 10,
        24..=26 => is_lance(tval, sval),
        27 => tval == TV_SWORD,
        42 => tval == TV_DIGGING && sval == SV_MATTOCK,
        1..=27 => matches!(tval, TV_HAFTED | TV_POLEARM | TV_SWORD) || tval == TV_DIGGING,
        40 | 41 => tval == TV_DIGGING,
        160..=163 | 167 => tval == TV_BOW,
        164 => tval == TV_BOW && sval == SV_LONG_BOW,
        165 => tval == TV_BOW && sval == SV_HEAVY_XBOW,
        166 => tval == TV_BOW && sval == SV_SLING,
        180..=185 => matches!(tval, TV_SHOT | TV_ARROW | TV_BOLT),
        195 | 196 => tval == TV_BOW && sval == SV_HARP,
        _ => false,
    }
}

#[allow(clippy::too_many_arguments)]
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
                    if tval == TV_SWORD && sval == SV_FALCON_SWORD {
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
        if tval == TV_SWORD && sval == SV_FALCON_SWORD && pval > 2 && source_index != 5 {
            pval = 2;
        }
        apply_rfb_pval(
            &mut roll.state.properties,
            source_index,
            pval,
            roll.charisma_pval,
            roll.dexterity_pval,
            roll.blows_pval,
            roll.stealth_penalty_pval,
        );
    }
    if let Some(pval) = roll.explicit_pval {
        apply_rfb_pval(
            &mut roll.state.properties,
            source_index,
            pval,
            roll.charisma_pval,
            roll.dexterity_pval,
            roll.blows_pval,
            roll.stealth_penalty_pval,
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
    stealth_penalty_pval: bool,
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
            if stealth_penalty_pval {
                properties.equipment_bonuses.stealth_skill = -pval_i32;
            }
        }
        6 => {
            properties.modifiers.spell_power_bonus = pval_i32;
            properties.modifiers.strength = -pval_i32;
            properties.modifiers.dexterity = -pval_i32;
            properties.modifiers.constitution = -pval_i32;
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
        27 => {
            if blows_pval {
                properties.equipment_bonuses.melee_attacks = pval_i32;
            }
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
        160 => (10, 5, 0, 0),
        161 => (5, 5, 0, 0),
        162 => (2, 4, 0, 3),
        163 => (4, 2, 0, 0),
        164 => (10, 10, 0, 3),
        165 => (5, 10, 0, 3),
        166 => (10, 5, 0, 3),
        167 => (10, 5, 0, 4),
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
    if tval == TV_SWORD && sval == SV_FALCON_SWORD {
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
            let sensed = one_in(rng, 6);
            if sensed && !is_ammunition {
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
    let _ = roll_rfb_craft_with_activation(rng, state, generation_level, is_ammunition, None);
}

fn roll_rfb_craft_with_activation(
    rng: &mut RfbRng,
    state: &mut RolledAffixState,
    generation_level: u16,
    is_ammunition: bool,
    generation: Option<&ItemDeviceGenerationDefinition>,
) -> Option<usize> {
    let mut activation_profile_index = None;
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
            if one_in(rng, 5) {
                activation_profile_index = generation.and_then(|generation| {
                    roll_biased_activation_index(
                        rng,
                        generation,
                        match brand {
                            WeaponBrand::Acid => RfbActivationBiasDefinition::Acid,
                            WeaponBrand::Electricity => RfbActivationBiasDefinition::Electricity,
                            WeaponBrand::Fire => RfbActivationBiasDefinition::Fire,
                            WeaponBrand::Cold => RfbActivationBiasDefinition::Cold,
                            WeaponBrand::Poison => RfbActivationBiasDefinition::Poison,
                            _ => unreachable!("craft only rolls five elemental brands"),
                        },
                        generation_level,
                    )
                });
            }
            break;
        }
        if one_in(rng, 3) && !is_ammunition {
            add_resistance(&mut state.properties, resistance);
        }
    }
    if one_in(rng, 6) && generation_level > 60 && !is_ammunition {
        state.weapon_traits.insert(WeaponTraitDto::ManaBrand);
    }
    activation_profile_index
}

fn roll_rfb_armageddon(
    rng: &mut RfbRng,
    state: &mut RolledAffixState,
    dice: &mut MeleeDamageDiceDto,
    tval: u16,
    sval: u16,
) {
    let odds = (dice.dice.saturating_mul(dice.sides) / 2).max(3);
    if one_in(rng, odds) {
        dice.dice = dice.dice.saturating_mul(2);
        if tval == TV_SWORD && sval == SV_LONG_SWORD && one_in(rng, 2) {
            dice.dice = 5;
        }
        if tval == TV_SWORD && sval == SV_KATANA && one_in(rng, 3) {
            dice.dice = 8;
            dice.sides = 5;
            if one_in(rng, 100) {
                dice.dice = 10;
                dice.sides = 6;
            }
        }
        if tval == TV_HAFTED && sval == SV_WAR_HAMMER && one_in(rng, 2) {
            dice.dice = 9;
        }
    } else {
        loop {
            dice.dice = dice.dice.saturating_add(1);
            if !one_in(rng, dice.dice) {
                break;
            }
        }
        loop {
            dice.sides = dice.sides.saturating_add(1);
            if !one_in(rng, dice.sides) {
                break;
            }
        }
    }
    if one_in(rng, 5) {
        state.properties.brands.insert(match randint1(rng, 5) {
            1 => WeaponBrand::Electricity,
            2 => WeaponBrand::Fire,
            3 => WeaponBrand::Cold,
            4 => WeaponBrand::Acid,
            _ => WeaponBrand::Poison,
        });
    }
    if tval == TV_SWORD {
        if one_in(rng, 3) {
            state.weapon_traits.insert(WeaponTraitDto::Vorpal);
        } else if one_in(rng, 777) {
            state.weapon_traits.insert(WeaponTraitDto::Vorpal2);
        }
    }
    if tval == TV_HAFTED {
        if one_in(rng, 7) {
            state.weapon_traits.insert(WeaponTraitDto::Impact);
        } else if one_in(rng, 7) {
            state.weapon_traits.insert(WeaponTraitDto::Stun);
        }
    }
    if one_in(rng, 666) {
        state.weapon_traits.insert(WeaponTraitDto::ManaBrand);
    }
}

fn roll_rfb_death(rng: &mut RfbRng, roll: &mut RfbWeaponRoll, dice: &mut MeleeDamageDiceDto) {
    let state = &mut roll.state;
    if one_in(rng, 16) {
        state.properties.equipment_bonuses.light_radius = -1;
        add_resistance(&mut state.properties, ActorDamageType::Dark);
        if one_in(rng, 6) {
            state
                .properties
                .resistances
                .insert(ActorDamageType::Light, ActorResistanceLevel::Vulnerable);
        }
    }
    if one_in(rng, 3) {
        add_slay(&mut state.properties, SlayTarget::Good, SlayLevel::Slay);
    } else if one_in(rng, 27) {
        add_slay(&mut state.properties, SlayTarget::Good, SlayLevel::Kill);
    }
    if one_in(rng, 3) {
        state.properties.brands.insert(WeaponBrand::Poison);
    }
    if one_in(rng, 3) {
        add_resistance(&mut state.properties, ActorDamageType::Nether);
    }
    if one_in(rng, 3) {
        add_resistance(&mut state.properties, ActorDamageType::Poison);
    }
    if one_in(rng, 6) {
        add_slay(&mut state.properties, SlayTarget::Human, SlayLevel::Slay);
    } else if one_in(rng, 36) {
        add_slay(&mut state.properties, SlayTarget::Human, SlayLevel::Kill);
    } else if one_in(rng, 13) {
        add_slay(&mut state.properties, SlayTarget::Living, SlayLevel::Slay);
        dice.dice = dice.dice.saturating_add(1);
        roll.curse = Some(ItemCurseSeverityDto::Heavy);
        state.curse_effects.insert(roll_rfb_heavy_curse_effect(rng));
    } else if one_in(rng, 78) {
        add_slay(&mut state.properties, SlayTarget::Living, SlayLevel::Kill);
        if one_in(rng, 2) {
            dice.dice = dice.dice.saturating_add(1);
        }
        roll.curse = Some(ItemCurseSeverityDto::Heavy);
        state.curse_effects.insert(roll_rfb_heavy_curse_effect(rng));
    }
}

fn roll_rfb_heavy_curse_effect(rng: &mut RfbRng) -> ItemCurseEffectDto {
    loop {
        let effect = match rng.bounded(28) {
            0 => ItemCurseEffectDto::TyCurse,
            1 => ItemCurseEffectDto::Aggravate,
            2 => ItemCurseEffectDto::DrainExperience,
            5 => ItemCurseEffectDto::AddHeavyCurse,
            7 => ItemCurseEffectDto::CallDemon,
            8 => ItemCurseEffectDto::CallDragon,
            10 => ItemCurseEffectDto::Teleport,
            19 => ItemCurseEffectDto::ByCurse,
            20 => ItemCurseEffectDto::Danger,
            23 => ItemCurseEffectDto::CrappyMutation,
            _ => continue,
        };
        return effect;
    }
}

fn roll_rfb_troika(
    rng: &mut RfbRng,
    roll: &mut RfbWeaponRoll,
    dice: &mut MeleeDamageDiceDto,
    generation_level: u16,
) {
    let random = i32::try_from(rng.bounded(33)).expect("d33 roll fits i32");
    let level = i32::from(generation_level);
    let lva =
        u16::try_from((6 - ((level + 6 - random) / 25)).clamp(2, 6)).expect("Troika odds fit u16");
    let mut gained_power = true;
    if one_in(rng, lva) {
        roll_rfb_craft(rng, &mut roll.state, generation_level, false);
    } else if one_in(rng, lva) {
        roll_rfb_slaying(rng, &mut roll.state.properties, generation_level, false);
    } else {
        gained_power = false;
    }
    if one_in(rng, lva) {
        let mut extra = 0;
        dice.dice = dice.dice.saturating_add(1);
        while one_in(rng, lva) && extra < 4 {
            extra += 1;
        }
        dice.dice = dice.dice.saturating_add(extra);
        if one_in(rng, lva.saturating_mul(lva)) {
            dice.sides = dice.sides.saturating_add(1);
            if one_in(rng, lva.saturating_mul(lva)) {
                dice.sides = dice.sides.saturating_add(1);
            }
        }
        gained_power = true;
    }
    if one_in(rng, lva.saturating_mul(2)) {
        roll.state
            .properties
            .passives
            .insert(EquipmentPassive::Vampiric);
        gained_power = true;
    } else if one_in(rng, lva.saturating_mul(lva)) {
        roll.state.properties.brands.insert(WeaponBrand::Chaos);
        gained_power = true;
    }
    if one_in(rng, lva.saturating_mul(88)) {
        roll.state.weapon_traits.insert(WeaponTraitDto::Vorpal2);
        gained_power = true;
    } else if one_in(rng, lva) {
        roll.state.weapon_traits.insert(WeaponTraitDto::Vorpal);
        gained_power = true;
    }
    if one_in(rng, lva) || roll.state.weapon_traits.contains(&WeaponTraitDto::Vorpal2) {
        roll.state
            .curse_effects
            .insert(ItemCurseEffectDto::Aggravate);
        gained_power = true;
    }
    if one_in(rng, 22) {
        add_one_elemental_resistance(rng, &mut roll.state.properties);
        gained_power = true;
    }
    if one_in(rng, 22) {
        add_one_resistance(rng, &mut roll.state.properties);
        gained_power = true;
    }
    if one_in(rng, 22) {
        add_one_sustain(rng, &mut roll.state.properties);
        gained_power = true;
    }
    if one_in(rng, 22) {
        add_one_ability(rng, &mut roll.state.properties);
        gained_power = true;
    }
    if one_in(rng, 7) {
        roll.state.weapon_traits.insert(WeaponTraitDto::Blessed);
        gained_power = true;
    }
    if one_in(rng, lva) {
        let armor_roll = i16::try_from(randint1(rng, (generation_level / 3).max(1)))
            .expect("Troika armor roll fits i16");
        roll.state.enchantment_delta.to_armor = roll
            .state
            .enchantment_delta
            .to_armor
            .saturating_add(8_i16.saturating_sub(armor_roll));
        gained_power = true;
    }
    if one_in(rng, 22_u16.saturating_mul(lva)) || !gained_power {
        roll.blows_pval = true;
        roll.explicit_pval = Some(if one_in(rng, 22_u16.saturating_mul(lva)) {
            2
        } else {
            1
        });
    }
    if generation_level < 30 {
        let penalty =
            i16::try_from((34 - generation_level) / 5).expect("Troika low-level penalty fits i16");
        roll.state.enchantment_delta.to_hit =
            roll.state.enchantment_delta.to_hit.saturating_sub(penalty);
        roll.state.enchantment_delta.to_damage = roll
            .state
            .enchantment_delta
            .to_damage
            .saturating_sub(penalty);
    }
}

fn is_lance(tval: u16, sval: u16) -> bool {
    tval == TV_POLEARM && matches!(sval, SV_LANCE | SV_HEAVY_LANCE)
}

fn add_one_demon_resistance(rng: &mut RfbRng, properties: &mut AffixPropertyBundleDefinition) {
    match randint1(rng, 6) {
        1 => add_resistance(properties, ActorDamageType::Fire),
        2 => add_resistance(properties, ActorDamageType::Confusion),
        3 => add_resistance(properties, ActorDamageType::Nexus),
        4 => add_resistance(properties, ActorDamageType::Chaos),
        5 => add_resistance(properties, ActorDamageType::Disenchant),
        _ => add_status_immunity(properties, "rfb.status.fear"),
    }
}

fn add_one_holy_resistance(rng: &mut RfbRng, properties: &mut AffixPropertyBundleDefinition) {
    let damage_type = match randint1(rng, 4) {
        1 => ActorDamageType::Light,
        2 => ActorDamageType::Sound,
        3 => ActorDamageType::Shards,
        _ => ActorDamageType::Disenchant,
    };
    add_resistance(properties, damage_type);
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
    properties.passives.insert(weak_esp(rng.bounded(9)));
}

fn add_esp_strong(rng: &mut RfbRng, properties: &mut AffixPropertyBundleDefinition) {
    properties.passives.insert(match rng.bounded(4) {
        0 => EquipmentPassive::EspEvil,
        1 => EquipmentPassive::Telepathy,
        2 => EquipmentPassive::EspLiving,
        _ => EquipmentPassive::EspNonliving,
    });
}

fn add_esp_weak(rng: &mut RfbRng, properties: &mut AffixPropertyBundleDefinition, extra: bool) {
    let count = if extra {
        let maximum = randint1(rng, 6);
        3_u16.saturating_add(randint1(rng, maximum))
    } else {
        randint1(rng, 3)
    };
    let mut available = (0..9_u64).collect::<Vec<_>>();
    for _ in 0..count {
        let index = usize::try_from(rng.bounded(available.len() as u64))
            .expect("weak ESP index fits usize");
        properties
            .passives
            .insert(weak_esp(available.remove(index)));
    }
}

const fn weak_esp(index: u64) -> EquipmentPassive {
    match index {
        0 => EquipmentPassive::EspAnimal,
        1 => EquipmentPassive::EspUndead,
        2 => EquipmentPassive::EspDemon,
        3 => EquipmentPassive::EspOrc,
        4 => EquipmentPassive::EspTroll,
        5 => EquipmentPassive::EspGiant,
        6 => EquipmentPassive::EspDragon,
        7 => EquipmentPassive::EspHuman,
        _ => EquipmentPassive::EspGood,
    }
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

    use crate::{STATE_HASH_SCHEMA_VERSION, game::Game};
    use rfb_content::{
        AbilityTargetDefinition, AbilityTargetModeDefinition, AffixDefinition, EquipmentBonuses,
        ItemDefinition, ItemDeviceActivationDefinition, ItemDeviceChargeRangeDefinition,
        ItemDeviceGenerationDefinition, ItemDeviceRecoveryDefinition, ItemUseEffectDefinition,
        RfbActivationBiasDefinition, RfbBaseKindDefinition, RfbEgoGenerationDefinition,
        StatModifiers,
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
            ammunition_behavior: None,
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

    fn rfb_launcher_item(kind_id: &str) -> ItemDefinition {
        Game::new(1)
            .content
            .item(kind_id)
            .expect("test launcher exists")
            .clone()
    }

    fn launcher_instance(kind_id: &str) -> ItemInstance {
        ItemInstance {
            id: "test.item.launcher".to_owned(),
            kind_id: kind_id.to_owned(),
            quantity: 1,
            inscription: None,
            origin_actor_kind_id: None,
            origin_kind: None,
            damage_dice_override: None,
            discount_percent: 0,
            quality: ItemQualityDto::Fine,
            affix_ids: Vec::new(),
            rolled_affixes: Vec::new(),
            intrinsic_properties: Default::default(),
            enchantments: ItemEnchantmentsDto::default(),
            curse: None,
            permanent_destruction_immunities: BTreeSet::new(),
            activation: None,
            charges: None,
            fuel: None,
            device_recovery_progress: 0,
            captured_actor: None,
            location: ItemLocation::Equipped {
                slot_id: "launcher".to_owned(),
            },
        }
    }

    fn roll_launcher_ego(source_index: u32, seed: u64, kind_id: &str) -> (EgoMaterialization, u64) {
        let affix = ego_affix(
            &format!("test.affix.launcher-{source_index}"),
            source_index,
            1,
            0,
            u16::MAX,
            vec![RfbEgoTypeDefinition::Bow],
        );
        let mut rng = RfbRng::seeded(seed);
        let materialization = materialize_rfb_launcher_ego_with_rng(
            &mut rng,
            &rfb_launcher_item(kind_id),
            &affix,
            80,
        )
        .expect("compatible launcher ego should materialize");
        (materialization, rng.draw_counter)
    }

    fn ego_activation_profile(
        id: &str,
        weight: u32,
        biases: BTreeSet<RfbActivationBiasDefinition>,
        difficulty: i32,
    ) -> ItemDeviceActivationDefinition {
        ItemDeviceActivationDefinition {
            id: id.to_owned(),
            name_key: format!("{id}-name"),
            weight,
            min_depth: 1,
            max_depth: 100,
            device_check_difficulty: difficulty,
            rfb_biases: biases,
            charges: ItemDeviceChargeRangeDefinition {
                minimum: 1,
                maximum: 1,
                cost: 1,
            },
            recovery: Some(ItemDeviceRecoveryDefinition {
                interval_ticks: 100,
                energy_per_mille: 1_000,
            }),
            target: AbilityTargetDefinition {
                modes: vec![AbilityTargetModeDefinition::SelfTarget],
                range: 0,
                requires_line_of_effect: false,
            },
            effect_program_id: None,
            effect: ItemUseEffectDefinition::NoNumericEffect,
        }
    }

    #[test]
    fn fixed_weapon_ego_activations_materialize_one_full_charge() {
        for (source_index, item, profile_id, power) in [
            (
                11,
                rfb_weapon_item(TV_SWORD, SV_LONG_SWORD),
                "test.activation.destruction",
                50,
            ),
            (
                15,
                rfb_weapon_item(TV_SWORD, SV_LONG_SWORD),
                "test.activation.teleport",
                15,
            ),
            (
                24,
                rfb_weapon_item(TV_POLEARM, SV_LANCE),
                "test.activation.charge",
                10,
            ),
            (
                42,
                rfb_weapon_item(TV_DIGGING, SV_MATTOCK),
                "test.activation.stone-to-mud",
                10,
            ),
        ] {
            let mut affix = ego_affix(
                &format!("test.affix.{source_index}"),
                source_index,
                1,
                0,
                u16::MAX,
                vec![if source_index == 42 {
                    RfbEgoTypeDefinition::Digger
                } else {
                    RfbEgoTypeDefinition::Weapon
                }],
            );
            affix.device_generation = Some(ItemDeviceGenerationDefinition {
                activations: vec![ego_activation_profile(
                    profile_id,
                    1,
                    BTreeSet::new(),
                    power,
                )],
                recovery: None,
            });
            let mut rng = RfbRng::seeded(0xE3_6000 + u64::from(source_index));
            let materialized = materialize_rfb_weapon_ego_with_rng(&mut rng, &item, &affix, 80)
                .expect("fixed activation ego should materialize");
            let activation = materialized
                .activation
                .expect("fixed activation should be selected");
            assert_eq!(activation.profile_id, profile_id);
            assert_eq!(activation.power, u16::try_from(power).unwrap());
            assert_eq!(activation.device_check_difficulty, power);
            assert_eq!(
                materialized.charges,
                Some(ItemChargesDto {
                    current: 1,
                    maximum: 1,
                })
            );
        }
    }

    #[test]
    fn biased_activation_selection_uses_source_order_weights_and_depth() {
        let mut excluded = ego_activation_profile(
            "test.activation.excluded",
            10_000,
            BTreeSet::from([RfbActivationBiasDefinition::Mage]),
            1,
        );
        excluded.max_depth = 49;
        let generation = ItemDeviceGenerationDefinition {
            activations: vec![
                ego_activation_profile("test.activation.fixed", 1, BTreeSet::new(), 1),
                excluded,
                ego_activation_profile(
                    "test.activation.common",
                    127,
                    BTreeSet::from([RfbActivationBiasDefinition::Mage]),
                    16,
                ),
                ego_activation_profile(
                    "test.activation.rare",
                    1,
                    BTreeSet::from([RfbActivationBiasDefinition::Mage]),
                    30,
                ),
            ],
            recovery: None,
        };
        let mut rng = RfbRng::seeded(0xE3_6003);
        assert_eq!(
            roll_biased_activation_index(
                &mut rng,
                &generation,
                RfbActivationBiasDefinition::Mage,
                50,
            ),
            Some(2)
        );
        assert_eq!(rng.draw_counter, 1);
    }

    #[test]
    fn daemon_bias_activation_overrides_fixed_destruction_in_exact_rng_order() {
        let item = rfb_weapon_item(TV_SWORD, SV_LONG_SWORD);
        let mut affix = ego_affix(
            "test.affix.daemon",
            11,
            1,
            0,
            u16::MAX,
            vec![RfbEgoTypeDefinition::Weapon],
        );
        affix.device_generation = Some(ItemDeviceGenerationDefinition {
            activations: vec![
                ego_activation_profile("test.activation.destruction", 1, BTreeSet::new(), 50),
                ego_activation_profile(
                    "test.activation.demon",
                    255,
                    BTreeSet::from([RfbActivationBiasDefinition::Demon]),
                    20,
                ),
            ],
            recovery: None,
        });
        let outcomes = [0_u64, 4]
            .into_iter()
            .map(|seed| {
                let mut rng = RfbRng::seeded(seed);
                let materialized = materialize_rfb_weapon_ego_with_rng(&mut rng, &item, &affix, 80)
                    .expect("Daemon ego should materialize");
                (
                    materialized
                        .activation
                        .expect("Daemon keeps either fixed or biased activation")
                        .profile_id,
                    rng.draw_counter,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            outcomes,
            [
                ("test.activation.destruction".to_owned(), 7),
                ("test.activation.demon".to_owned(), 8),
            ]
        );
    }

    #[test]
    fn mage_bias_activation_belongs_to_arcane_not_mana() {
        let activation = ego_activation_profile(
            "test.activation.mage",
            255,
            BTreeSet::from([RfbActivationBiasDefinition::Mage]),
            20,
        );
        let mut mana_affix = ego_affix(
            "test.affix.mana",
            3,
            1,
            0,
            u16::MAX,
            vec![RfbEgoTypeDefinition::Weapon],
        );
        mana_affix.device_generation = Some(ItemDeviceGenerationDefinition {
            activations: vec![activation.clone()],
            recovery: None,
        });
        let mut mana_rng = RfbRng::seeded(1);
        let mana = materialize_rfb_weapon_ego_with_rng(
            &mut mana_rng,
            &rfb_weapon_item(TV_SWORD, SV_LONG_SWORD),
            &mana_affix,
            50,
        )
        .expect("Mana ego should materialize");
        assert_eq!(mana.activation, None);
        assert_eq!(mana_rng.draw_counter, 4);

        let mut arcane_affix = ego_affix(
            "test.affix.arcane",
            6,
            1,
            0,
            u16::MAX,
            vec![RfbEgoTypeDefinition::Weapon],
        );
        arcane_affix.device_generation = Some(ItemDeviceGenerationDefinition {
            activations: vec![activation],
            recovery: None,
        });
        let mut arcane_rng = RfbRng::seeded(1);
        let arcane = materialize_rfb_weapon_ego_with_rng(
            &mut arcane_rng,
            &rfb_weapon_item(TV_HAFTED, SV_WIZSTAFF),
            &arcane_affix,
            50,
        )
        .expect("Arcane ego should materialize");
        assert_eq!(
            arcane.activation.map(|activation| activation.profile_id),
            Some("test.activation.mage".to_owned())
        );
        assert_eq!(arcane_rng.draw_counter, 5);
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
    fn special_weapon_egos_materialize_fixed_seed_results() {
        let item = rfb_weapon_item(TV_SWORD, SV_LONG_SWORD);
        for (source_index, seed) in [(2, 13), (7, 23), (27, 41)] {
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
                .expect("special weapon ego should materialize");
            let rolled = &materialized.rolled_affixes[0];
            match source_index {
                2 => {
                    assert_eq!(
                        rolled.melee_damage_dice,
                        Some(MeleeDamageDiceDto { dice: 3, sides: 6 })
                    );
                    assert_eq!(
                        rolled.weapon_traits,
                        BTreeSet::from([WeaponTraitDto::Vorpal])
                    );
                    assert_eq!(rolled.properties.equipment_bonuses.digging_skill, 1);
                    assert_eq!(rng.draw_counter, 4);
                }
                7 => {
                    assert_eq!(
                        rolled.melee_damage_dice,
                        Some(MeleeDamageDiceDto { dice: 3, sides: 7 })
                    );
                    assert_eq!(rolled.enchantment_delta.to_damage, 7);
                    assert_eq!(rng.draw_counter, 8);
                }
                27 => {
                    assert_eq!(
                        rolled.melee_damage_dice,
                        Some(MeleeDamageDiceDto { dice: 4, sides: 6 })
                    );
                    assert_eq!(
                        rolled.enchantment_delta,
                        ItemEnchantmentsDto {
                            to_hit: 6,
                            to_damage: 9,
                            to_armor: -11,
                        }
                    );
                    assert_eq!(
                        rolled.properties.brands,
                        BTreeSet::from([
                            WeaponBrand::Acid,
                            WeaponBrand::Electricity,
                            WeaponBrand::Fire,
                            WeaponBrand::Poison,
                        ])
                    );
                    assert_eq!(
                        rolled.properties.resistances,
                        BTreeMap::from([
                            (
                                ActorDamageType::Electricity,
                                ActorResistanceLevel::Resistant,
                            ),
                            (ActorDamageType::Poison, ActorResistanceLevel::Resistant),
                        ])
                    );
                    assert_eq!(rolled.properties.equipment_bonuses.light_radius, 1);
                    assert_eq!(rng.draw_counter, 36);
                }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn morgul_and_death_materialize_concrete_heavy_curses_and_darkness() {
        let item = rfb_weapon_item(TV_SWORD, SV_LONG_SWORD);
        let morgul = ego_affix(
            "test.affix.morgul",
            21,
            1,
            0,
            u16::MAX,
            vec![RfbEgoTypeDefinition::Weapon],
        );
        let mut morgul_rng = RfbRng::seeded(1);
        let morgul = materialize_rfb_weapon_ego_with_rng(&mut morgul_rng, &item, &morgul, 70)
            .expect("Morgul should materialize");
        assert_eq!(morgul.curse, Some(ItemCurseSeverityDto::Heavy));
        assert!(
            morgul
                .curse_effects
                .contains(&ItemCurseEffectDto::Aggravate),
            "Morgul aggravation is an intrinsic drawback"
        );
        assert!(!morgul.curse_effects.is_empty());

        let death = ego_affix(
            "test.affix.death",
            12,
            1,
            0,
            u16::MAX,
            vec![RfbEgoTypeDefinition::Weapon],
        );
        let dark = (0..10_000_u64)
            .find_map(|seed| {
                let mut rng = RfbRng::seeded(seed);
                let materialized =
                    materialize_rfb_weapon_ego_with_rng(&mut rng, &item, &death, 70)?;
                materialized
                    .rolled_affixes
                    .first()
                    .is_some_and(|rolled| rolled.properties.equipment_bonuses.light_radius == -1)
                    .then_some(materialized)
            })
            .expect("a deterministic Death darkness seed should exist");
        assert_eq!(
            dark.rolled_affixes[0]
                .properties
                .equipment_bonuses
                .light_radius,
            -1
        );

        let cursed = (0..100_000_u64)
            .find_map(|seed| {
                let mut rng = RfbRng::seeded(seed);
                let materialized =
                    materialize_rfb_weapon_ego_with_rng(&mut rng, &item, &death, 70)?;
                materialized.curse.is_some().then_some(materialized)
            })
            .expect("a deterministic cursed Death seed should exist");
        assert_eq!(cursed.curse, Some(ItemCurseSeverityDto::Heavy));
        assert_eq!(cursed.curse_effects.len(), 1);
    }

    #[test]
    fn authoritative_heavy_mask_contains_exactly_ten_effects() {
        let mut effects = BTreeSet::new();
        for seed in 0..10_000_u64 {
            let mut rng = RfbRng::seeded(seed);
            effects.insert(roll_rfb_heavy_curse_effect(&mut rng));
        }
        assert_eq!(
            effects,
            BTreeSet::from([
                ItemCurseEffectDto::TyCurse,
                ItemCurseEffectDto::Aggravate,
                ItemCurseEffectDto::DrainExperience,
                ItemCurseEffectDto::AddHeavyCurse,
                ItemCurseEffectDto::CallDemon,
                ItemCurseEffectDto::CallDragon,
                ItemCurseEffectDto::Teleport,
                ItemCurseEffectDto::ByCurse,
                ItemCurseEffectDto::Danger,
                ItemCurseEffectDto::CrappyMutation,
            ])
        );
    }

    #[test]
    fn special_weapon_ego_restrictions_reject_without_partial_state() {
        let hafted = rfb_weapon_item(TV_HAFTED, SV_WAR_HAMMER);
        let sharpness = ego_affix(
            "test.affix.sharpness",
            2,
            1,
            0,
            u16::MAX,
            vec![RfbEgoTypeDefinition::Weapon],
        );
        let mut rng = RfbRng::seeded(7);
        let before = rng.clone();
        assert!(materialize_rfb_weapon_ego_with_rng(&mut rng, &hafted, &sharpness, 80).is_none());
        assert_eq!(rng, before);

        let sword = rfb_weapon_item(TV_SWORD, SV_LONG_SWORD);
        let jousting = ego_affix(
            "test.affix.jousting",
            24,
            1,
            0,
            u16::MAX,
            vec![RfbEgoTypeDefinition::Weapon],
        );
        assert!(materialize_rfb_weapon_ego_with_rng(&mut rng, &sword, &jousting, 80).is_none());
        assert_eq!(rng, before);
    }

    #[test]
    fn shared_slaying_helper_keeps_ammunition_rng_contract() {
        let mut rng = RfbRng::seeded(31);
        let mut properties = AffixPropertyBundleDefinition::default();
        roll_rfb_slaying(&mut rng, &mut properties, 50, true);
        assert_eq!(
            properties.slays,
            BTreeMap::from([
                (SlayTarget::Evil, SlayLevel::Slay),
                (SlayTarget::Good, SlayLevel::Slay),
                (SlayTarget::Undead, SlayLevel::Slay),
                (SlayTarget::Demon, SlayLevel::Slay),
            ])
        );
        assert!(properties.passives.is_empty());
        assert_eq!(rng.draw_counter, 17);
    }

    #[test]
    fn digger_egos_materialize_fixed_seed_results() {
        for (source_index, seed, sval) in [(40, 17, 1), (41, 29, 1), (42, 43, SV_MATTOCK)] {
            let item = rfb_weapon_item(TV_DIGGING, sval);
            let affix = ego_affix(
                &format!("test.affix.{source_index}"),
                source_index,
                1,
                0,
                u16::MAX,
                vec![RfbEgoTypeDefinition::Digger],
            );
            let mut rng = RfbRng::seeded(seed);
            let materialized = materialize_rfb_weapon_ego_with_rng(&mut rng, &item, &affix, 70)
                .expect("digger ego should materialize");
            let rolled = &materialized.rolled_affixes[0];
            match source_index {
                40 => {
                    assert_eq!(rolled.melee_damage_dice, None);
                    assert_eq!(rolled.properties.equipment_bonuses.digging_skill, 5);
                    assert_eq!(rng.draw_counter, 1);
                }
                41 => {
                    assert_eq!(
                        rolled.melee_damage_dice,
                        Some(MeleeDamageDiceDto { dice: 3, sides: 6 })
                    );
                    assert_eq!(rolled.enchantment_delta.to_damage, 3);
                    assert_eq!(rolled.properties.equipment_bonuses.digging_skill, 4);
                    assert_eq!(rng.draw_counter, 2);
                }
                42 => {
                    assert_eq!(
                        rolled.melee_damage_dice,
                        Some(MeleeDamageDiceDto { dice: 4, sides: 6 })
                    );
                    assert_eq!(rolled.enchantment_delta.to_damage, 3);
                    assert_eq!(rolled.properties.equipment_bonuses.digging_skill, 4);
                    assert_eq!(rolled.properties.modifiers.strength, 4);
                    assert_eq!(rng.draw_counter, 2);
                }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn incompatible_digger_ego_retries_before_atomic_materialization() {
        let item = rfb_weapon_item(TV_DIGGING, 1);
        let mut disruption = ego_affix(
            "test.affix.disruption",
            42,
            1,
            0,
            u16::MAX,
            vec![RfbEgoTypeDefinition::Digger],
        );
        disruption.rfb_ego.as_mut().unwrap().rarity = 1;
        let digging = ego_affix(
            "test.affix.digging",
            40,
            1,
            0,
            u16::MAX,
            vec![RfbEgoTypeDefinition::Digger],
        );
        let mut rng = RfbRng::seeded(1);
        let materialized = roll_and_materialize_rfb_ego_from_affixes_with_rng(
            &mut rng,
            &item,
            [disruption, digging].iter(),
            70,
            None,
        )
        .expect("compatible fallback ego should eventually materialize");
        assert_eq!(materialized.affix_ids, ["test.affix.digging"]);
        assert_eq!(
            materialized.rolled_affixes[0]
                .properties
                .equipment_bonuses
                .digging_skill,
            1
        );
        assert_eq!(rng.draw_counter, 3);
    }

    #[test]
    fn all_weapon_and_digger_source_indices_have_a_materialization_branch() {
        for source_index in (1..=27).chain(40..=42) {
            let (item, ego_type) = match source_index {
                6 => (
                    rfb_weapon_item(TV_HAFTED, SV_WIZSTAFF),
                    RfbEgoTypeDefinition::Weapon,
                ),
                24..=26 => (
                    rfb_weapon_item(TV_POLEARM, SV_LANCE),
                    RfbEgoTypeDefinition::Weapon,
                ),
                40 | 41 => (rfb_weapon_item(TV_DIGGING, 1), RfbEgoTypeDefinition::Digger),
                42 => (
                    rfb_weapon_item(TV_DIGGING, SV_MATTOCK),
                    RfbEgoTypeDefinition::Digger,
                ),
                _ => (
                    rfb_weapon_item(TV_SWORD, SV_LONG_SWORD),
                    RfbEgoTypeDefinition::Weapon,
                ),
            };
            let affix = ego_affix(
                &format!("test.affix.{source_index}"),
                source_index,
                1,
                0,
                u16::MAX,
                vec![ego_type],
            );
            let mut rng = RfbRng::seeded(u64::from(source_index));
            assert!(
                materialize_rfb_weapon_ego_with_rng(&mut rng, &item, &affix, 80).is_some(),
                "source {source_index} must materialize on its compatible base"
            );
        }
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
            intrinsic_properties: Default::default(),
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
            intrinsic_properties: Default::default(),
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
    fn ranged_materialization_state_is_atomic_projected_and_save_stable() {
        assert_eq!(STATE_HASH_SCHEMA_VERSION, 108);
        let intrinsic_properties = AffixPropertyBundleDefinition {
            modifiers: StatModifiers {
                charisma: 2,
                ..StatModifiers::default()
            },
            equipment_bonuses: EquipmentBonuses {
                launcher_multiplier_delta_percent: 25,
                base_shot_delta_percent: 15,
                ..EquipmentBonuses::default()
            },
            ..AffixPropertyBundleDefinition::default()
        };
        let mut item = ItemInstance {
            id: "test.item.harp".to_owned(),
            kind_id: "demo.item.harp".to_owned(),
            quantity: 1,
            inscription: None,
            origin_actor_kind_id: None,
            origin_kind: None,
            damage_dice_override: None,
            discount_percent: 0,
            quality: ItemQualityDto::Fine,
            affix_ids: Vec::new(),
            rolled_affixes: Vec::new(),
            intrinsic_properties: Default::default(),
            enchantments: ItemEnchantmentsDto::default(),
            curse: None,
            permanent_destruction_immunities: BTreeSet::new(),
            activation: None,
            charges: None,
            fuel: None,
            device_recovery_progress: 0,
            captured_actor: None,
            location: ItemLocation::Equipped {
                slot_id: "launcher".to_owned(),
            },
        };
        let before_commit = item.clone();
        let materialization = EgoMaterialization::new(
            Vec::new(),
            Vec::new(),
            Some(intrinsic_properties.clone()),
            None,
            None,
            None,
        );
        assert_eq!(item, before_commit, "preparation must not mutate the item");
        materialization.apply_to(&mut item);
        assert_eq!(item.intrinsic_properties, intrinsic_properties);

        let mut game = Game::new(57);
        for equipped in &mut game.items {
            if matches!(
                &equipped.location,
                ItemLocation::Equipped { slot_id } if slot_id == "launcher"
            ) {
                equipped.location = ItemLocation::Inventory;
            }
        }
        let base_charisma = game.effective_player_attributes().charisma;
        game.items.push(item);
        let knowledge = game
            .item_property_knowledge
            .entry("test.item.harp".to_owned())
            .or_default();
        knowledge.discovered = true;
        knowledge.appraised = true;
        knowledge.identified = true;
        assert_eq!(
            game.effective_player_attributes().charisma,
            base_charisma + 2
        );
        let harp = game
            .items
            .iter()
            .find(|item| item.id == "test.item.harp")
            .expect("harp should remain equipped");
        let bonuses = game.item_equipment_bonuses(harp);
        assert_eq!(bonuses.launcher_multiplier_delta_percent, 25);
        assert_eq!(bonuses.base_shot_delta_percent, 15);
        let projected = game
            .equipment_dto()
            .into_iter()
            .find(|item| item.id == "test.item.harp")
            .expect("equipped harp should be projected");
        assert_eq!(projected.modifiers.charisma, 2);
        assert_eq!(
            projected
                .equipment_bonuses
                .launcher_multiplier_delta_percent,
            25
        );
        assert_eq!(projected.equipment_bonuses.base_shot_delta_percent, 15);
        game.items
            .iter_mut()
            .find(|item| item.id == "test.item.harp")
            .expect("harp should remain present")
            .location = ItemLocation::Inventory;

        let mut without_intrinsic = game.clone();
        without_intrinsic
            .items
            .iter_mut()
            .find(|item| item.id == "test.item.harp")
            .expect("harp should remain present")
            .intrinsic_properties = Default::default();
        assert_ne!(game.state_hash(), without_intrinsic.state_hash());
        let rng_before = game.rng.clone();
        let saved = game.to_save();
        assert_eq!(
            saved
                .inventory
                .iter()
                .find(|item| item.id == "test.item.harp")
                .expect("harp should be saved")
                .intrinsic_properties
                .modifiers
                .charisma,
            2
        );
        let restored = Game::from_save(saved).expect("ranged item state should round-trip");
        assert_eq!(
            restored
                .items
                .iter()
                .find(|item| item.id == "test.item.harp")
                .expect("harp should restore")
                .intrinsic_properties,
            intrinsic_properties
        );
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(
            restored.rng, rng_before,
            "loading must not reroll properties"
        );
    }

    #[test]
    fn ordinary_harp_rolls_intrinsic_charisma_and_is_not_a_projectile_launcher() {
        let definition = rfb_launcher_item("demo.item.harp");
        assert_eq!(definition.equipment_slot.as_deref(), Some("launcher"));
        assert!(definition.projectile_profile.is_none());
        assert!(definition.resists_enchantment);

        let mut rng = RfbRng::seeded(0xE4_4001);
        let intrinsic = materialize_rfb_harp_intrinsic_with_rng(&mut rng, &definition, 80)
            .expect("the authoritative Harp base should roll intrinsic charisma");
        assert_eq!((intrinsic.modifiers.charisma, rng.draw_counter), (2, 4));

        let mut game = Game::new(57);
        for item in &mut game.items {
            if matches!(&item.location, ItemLocation::Equipped { .. }) {
                item.location = ItemLocation::Inventory;
            }
        }
        let base_charisma = game.effective_player_attributes().charisma;
        let mut harp = launcher_instance("demo.item.harp");
        harp.quality = ItemQualityDto::Ordinary;
        harp.intrinsic_properties = intrinsic.clone();
        assert!(harp.affix_ids.is_empty());
        assert_eq!(harp.enchantments, ItemEnchantmentsDto::default());
        game.items.push(harp);

        assert_eq!(
            game.effective_player_attributes().charisma,
            base_charisma
                + u16::try_from(intrinsic.modifiers.charisma)
                    .expect("Harp charisma bonus should be positive")
        );
        assert!(game.player_projectile_profile().is_none());
        game.items
            .iter_mut()
            .find(|item| item.id == "test.item.launcher")
            .expect("ordinary Harp should remain present")
            .location = ItemLocation::Inventory;
        assert_eq!(game.effective_player_attributes().charisma, base_charisma);
    }

    #[test]
    fn harp_egos_reuse_base_pval_and_round_trip_without_rerolling() {
        let definition = rfb_launcher_item("demo.item.harp");
        let mut rng = RfbRng::seeded(0xE4_4195);
        let intrinsic = materialize_rfb_harp_intrinsic_with_rng(&mut rng, &definition, 80)
            .expect("Harp base should roll before its ego");
        let base_draws = rng.draw_counter;
        let vanyar = ego_affix(
            "test.affix.vanyar",
            195,
            1,
            0,
            u16::MAX,
            vec![RfbEgoTypeDefinition::Harp],
        );
        let mut missing_base_rng = RfbRng::seeded(0xE4_4195);
        assert!(
            roll_and_materialize_rfb_ego_from_affixes_with_rng(
                &mut missing_base_rng,
                &definition,
                std::iter::once(&vanyar),
                80,
                None,
            )
            .is_none()
        );
        assert_eq!(missing_base_rng.draw_counter, 0);
        let materialization = roll_and_materialize_rfb_ego_from_affixes_with_rng(
            &mut rng,
            &definition,
            std::iter::once(&vanyar),
            80,
            Some(&intrinsic),
        )
        .expect("Vanyar should materialize on a Harp");
        assert_eq!(rng.draw_counter, base_draws + 1, "ego must not reroll pval");
        assert_eq!(
            materialization.enchantment_delta,
            ItemEnchantmentsDto::default()
        );
        let rolled = &materialization.rolled_affixes[0];
        assert_eq!(rolled.properties.modifiers.charisma, 0);
        assert_eq!(
            rolled.properties.modifiers.wisdom,
            intrinsic.modifiers.charisma
        );
        assert_eq!(
            rolled.properties.passives,
            BTreeSet::from([
                EquipmentPassive::SustainCharisma,
                EquipmentPassive::SustainWisdom,
            ])
        );
        assert_eq!(
            rolled.properties.resistances.get(&ActorDamageType::Dark),
            Some(&ActorResistanceLevel::Resistant)
        );

        let mut vanyar_item = launcher_instance("demo.item.harp");
        vanyar_item.intrinsic_properties = intrinsic.clone();
        materialization.apply_to(&mut vanyar_item);
        assert_eq!(
            vanyar_item.intrinsic_properties.modifiers.charisma,
            intrinsic.modifiers.charisma
        );
        vanyar_item.location = ItemLocation::Inventory;
        let content = Game::new(1).content;
        let saved = crate::save::inventory_to_save(std::slice::from_ref(&vanyar_item));
        let restored = crate::save::inventory_item_from_dto(saved[0].clone(), &content)
            .expect("Vanyar Harp should round-trip");
        assert_eq!(
            restored.intrinsic_properties,
            vanyar_item.intrinsic_properties
        );
        assert_eq!(restored.rolled_affixes, vanyar_item.rolled_affixes);

        let erebor = ego_affix(
            "test.affix.erebor",
            196,
            1,
            0,
            u16::MAX,
            vec![RfbEgoTypeDefinition::Harp],
        );
        let erebor = materialize_rfb_harp_ego(&definition, &erebor, &intrinsic)
            .expect("Erebor should materialize on a Harp");
        assert_eq!(erebor.enchantment_delta, ItemEnchantmentsDto::default());
        let rolled = &erebor.rolled_affixes[0];
        assert_eq!(rolled.properties.modifiers.charisma, 0);
        assert_eq!(
            rolled.properties.passives,
            BTreeSet::from([
                EquipmentPassive::SustainCharisma,
                EquipmentPassive::SustainStrength,
                EquipmentPassive::SustainConstitution,
            ])
        );
        assert_eq!(
            rolled.properties.status_immunities,
            ["rfb.status.fear", "rfb.status.blindness"]
        );
    }

    #[test]
    fn ammunition_egos_share_dynamic_helpers_typed_behaviors_and_supercharge() {
        use rfb_content::AmmunitionBehaviorDefinition;

        let content = Game::new(1).content;
        let definition = content
            .item("demo.item.arrow")
            .expect("test ammunition exists")
            .clone();
        for (source_index, affix_id) in [
            (180, "rfb-legacy.affix.slaying-180"),
            (181, "rfb-legacy.affix.elemental"),
            (182, "rfb-legacy.affix.holy-might"),
            (183, "rfb-legacy.affix.returning"),
            (184, "rfb-legacy.affix.endurance"),
            (185, "rfb-legacy.affix.exploding"),
        ] {
            let affix = content
                .affix(affix_id)
                .expect("formal ammunition ego exists");
            assert_eq!(
                affix.rfb_ego.as_ref().map(|ego| ego.source_index),
                Some(source_index)
            );
            let mut rng = RfbRng::seeded(0xE4_5000 + u64::from(source_index));
            let materialized =
                materialize_rfb_ammunition_ego_with_rng(&mut rng, &definition, affix, 50)
                    .expect("all six ammunition egos should materialize");
            assert_eq!(materialized.affix_ids, [affix.id.clone()]);
            match source_index {
                180 => assert!(!materialized.rolled_affixes[0].properties.slays.is_empty()),
                181 => {
                    assert!(!materialized.rolled_affixes[0].properties.brands.is_empty());
                    assert_eq!(affix.elemental_destruction_immunities.len(), 4);
                }
                182 => {
                    assert!(
                        materialized
                            .weapon_traits
                            .contains(&WeaponTraitDto::Blessed)
                    );
                    assert_eq!(affix.slays.len(), 3);
                    assert!(affix.brands.contains(&WeaponBrand::Fire));
                    assert_eq!(affix.elemental_destruction_immunities.len(), 4);
                }
                183 => assert_eq!(
                    affix.ammunition_behavior,
                    Some(AmmunitionBehaviorDefinition::Returning)
                ),
                184 => {
                    assert!(affix.ammunition_behavior.is_none());
                    assert_eq!(affix.elemental_destruction_immunities.len(), 4);
                    assert!(affix.resists_projection_destruction);
                    assert!(affix.resists_monster_destruction);
                }
                185 => assert_eq!(
                    affix.ammunition_behavior,
                    Some(AmmunitionBehaviorDefinition::Exploding)
                ),
                _ => unreachable!(),
            }
        }

        let endurance = content
            .affix("rfb-legacy.affix.endurance")
            .expect("formal Endurance ego exists");
        let mut rng = RfbRng::seeded(18);
        let materialized =
            materialize_rfb_ammunition_ego_with_rng(&mut rng, &definition, endurance, 50)
                .expect("Endurance ammunition should materialize");
        assert_eq!(rng.draw_counter, 3);
        assert_eq!(materialized.ammunition_damage_dice, Some(5));
        let mut stack = launcher_instance("demo.item.arrow");
        stack.quantity = 20;
        materialized.apply_to(&mut stack);
        assert_eq!(stack.damage_dice_override, Some(5));
    }

    #[test]
    fn basic_launcher_egos_follow_authoritative_rng_and_profile_order() {
        let (accuracy, draws) = roll_launcher_ego(160, 0xE4_3160, "demo.item.long-bow");
        assert_eq!(
            (
                draws,
                accuracy.enchantment_delta.to_hit,
                accuracy.enchantment_delta.to_damage
            ),
            (2, 12, 5)
        );

        let (velocity, draws) = roll_launcher_ego(161, 0xE4_3161, "demo.item.long-bow");
        let velocity = &velocity.rolled_affixes[0];
        assert_eq!(
            (
                draws,
                velocity.enchantment_delta.to_hit,
                velocity.enchantment_delta.to_damage
            ),
            (6, 4, 9)
        );
        assert_eq!(
            velocity
                .properties
                .equipment_bonuses
                .launcher_multiplier_delta_percent,
            17
        );

        let (might, draws) = roll_launcher_ego(162, 0xE4_3162, "demo.item.long-bow");
        let might = &might.rolled_affixes[0];
        assert_eq!(
            (
                draws,
                might.enchantment_delta.to_hit,
                might.enchantment_delta.to_damage
            ),
            (8, 2, 1)
        );
        assert_eq!(
            might
                .properties
                .equipment_bonuses
                .launcher_multiplier_delta_percent,
            80
        );
        assert_eq!(might.properties.modifiers.strength, 1);

        let (shots, draws) = roll_launcher_ego(163, 0xE4_3163, "demo.item.long-bow");
        let shots = &shots.rolled_affixes[0];
        assert_eq!(
            (
                draws,
                shots.enchantment_delta.to_hit,
                shots.enchantment_delta.to_damage
            ),
            (6, 3, 1)
        );
        assert_eq!(
            shots.properties.equipment_bonuses.base_shot_delta_percent,
            60
        );

        let (strong_hunter, draws) = roll_launcher_ego(167, 0xE4_3167, "demo.item.long-bow");
        let strong_hunter = &strong_hunter.rolled_affixes[0];
        assert_eq!(
            (
                draws,
                strong_hunter.enchantment_delta.to_hit,
                strong_hunter.enchantment_delta.to_damage
            ),
            (6, 4, 4)
        );
        assert_eq!(
            strong_hunter.properties.passives,
            BTreeSet::from([EquipmentPassive::EspNonliving])
        );
        assert_eq!(
            strong_hunter.properties.slays.get(&SlayTarget::Animal),
            Some(&SlayLevel::Slay)
        );
        assert_eq!(strong_hunter.properties.equipment_bonuses.stealth_skill, 3);

        let (weak_hunter, draws) = roll_launcher_ego(167, 1, "demo.item.long-bow");
        let weak_hunter = &weak_hunter.rolled_affixes[0];
        assert_eq!(
            (
                draws,
                weak_hunter.enchantment_delta.to_hit,
                weak_hunter.enchantment_delta.to_damage
            ),
            (8, 3, 2)
        );
        assert_eq!(
            weak_hunter.properties.passives,
            BTreeSet::from([EquipmentPassive::EspGiant, EquipmentPassive::EspGood])
        );
        assert_eq!(weak_hunter.properties.equipment_bonuses.stealth_skill, 2);
    }

    #[test]
    fn launcher_ego_profile_uses_final_multiplier_range_and_shot_rate() {
        let mut game = Game::new(57);
        let launcher_slot_id = game
            .items
            .iter()
            .find_map(|item| {
                let ItemLocation::Equipped { slot_id } = &item.location else {
                    return None;
                };
                game.content
                    .item(&item.kind_id)
                    .is_some_and(|definition| definition.projectile_profile.is_some())
                    .then(|| slot_id.clone())
            })
            .expect("test game should begin with a launcher slot");
        let equipped_launchers = game
            .items
            .iter()
            .filter(|item| {
                matches!(item.location, ItemLocation::Equipped { .. })
                    && game
                        .content
                        .item(&item.kind_id)
                        .is_some_and(|definition| definition.projectile_profile.is_some())
            })
            .map(|item| item.id.clone())
            .collect::<BTreeSet<_>>();
        for item in &mut game.items {
            if equipped_launchers.contains(&item.id) {
                item.location = ItemLocation::Inventory;
            }
        }
        let mut launcher = launcher_instance("demo.item.long-bow");
        launcher.location = ItemLocation::Equipped {
            slot_id: launcher_slot_id,
        };
        game.items.push(launcher);
        let base = game
            .player_projectile_profile()
            .expect("test long bow should resolve");

        let mut might_game = game.clone();
        let (might, _) = roll_launcher_ego(162, 0xE4_3162, "demo.item.long-bow");
        let might_item = might_game
            .items
            .iter_mut()
            .find(|item| item.id == "test.item.launcher")
            .expect("test launcher should remain equipped");
        might.apply_to(might_item);
        might_item.affix_ids.clear();
        let might_profile = might_game
            .player_projectile_profile()
            .expect("extra might bow should resolve");
        assert_eq!(might_profile.damage_multiplier_percent, 380);
        assert_eq!(might_profile.range, 17);
        assert_eq!(
            might_profile.to_damage,
            might_profile.ammunition_to_damage.saturating_mul(380) / 100
                + might_profile.launcher_to_damage
        );
        let might_item = might_game
            .items
            .iter()
            .find(|item| item.id == "test.item.launcher")
            .expect("test launcher should remain equipped");
        assert_eq!(
            might_game
                .item_projectile_profile(might_item)
                .expect("item profile should resolve")
                .range,
            17
        );

        let mut shots_game = game.clone();
        let (shots, _) = roll_launcher_ego(163, 0xE4_3163, "demo.item.long-bow");
        let shots_item = shots_game
            .items
            .iter_mut()
            .find(|item| item.id == "test.item.launcher")
            .expect("test launcher should remain equipped");
        shots.apply_to(shots_item);
        shots_item.affix_ids.clear();
        let shots_profile = shots_game
            .player_projectile_profile()
            .expect("extra shots bow should resolve");
        assert_eq!(shots_profile.base_shot, base.base_shot + 60);
        assert_eq!(shots_profile.energy_cost, 10_000 / shots_profile.base_shot);
    }

    #[test]
    fn restricted_launcher_egos_retry_without_partial_rolls() {
        let affixes = [164, 165, 166].map(|source_index| {
            ego_affix(
                &format!("test.affix.launcher-{source_index}"),
                source_index,
                1,
                0,
                u16::MAX,
                vec![RfbEgoTypeDefinition::Bow],
            )
        });

        for (source_index, wrong_kind) in [
            (164, "demo.item.short-bow"),
            (165, "demo.item.long-bow"),
            (166, "demo.item.long-bow"),
        ] {
            let mut rng = RfbRng::seeded(11);
            let before = rng.clone();
            assert!(
                materialize_rfb_launcher_ego_with_rng(
                    &mut rng,
                    &rfb_launcher_item(wrong_kind),
                    affixes
                        .iter()
                        .find(|affix| affix.rfb_ego.as_ref().unwrap().source_index == source_index)
                        .unwrap(),
                    80,
                )
                .is_none()
            );
            assert_eq!(
                rng, before,
                "source {source_index} rejection must be atomic"
            );
        }

        for (kind_id, seed, expected_affix, selection_draws, total_draws) in [
            ("demo.item.long-bow", 1, "test.affix.launcher-164", 5, 15),
            (
                "demo.item.heavy-crossbow",
                1,
                "test.affix.launcher-165",
                2,
                12,
            ),
            ("demo.item.sling", 2, "test.affix.launcher-166", 3, 8),
        ] {
            let mut rng = RfbRng::seeded(seed);
            let materialization = roll_and_materialize_rfb_ego_from_affixes_with_rng(
                &mut rng,
                &rfb_launcher_item(kind_id),
                affixes.iter(),
                80,
                None,
            )
            .expect("compatible restricted ego should eventually be selected");
            assert_eq!(materialization.affix_ids, [expected_affix]);
            assert_eq!(rng.draw_counter, total_draws);

            let mut selection_rng = RfbRng::seeded(seed);
            while roll_rfb_ego_from_affixes(
                affixes.iter(),
                &mut selection_rng,
                80,
                &[RfbEgoTypeDefinition::Bow],
            ) != Some(expected_affix)
            {}
            assert_eq!(selection_rng.draw_counter, selection_draws);
        }

        for (source_index, seed, kind_id, expected) in [
            (164, 2, "demo.item.long-bow", (9, 7, 11, 0, 2, 0, 2, 71, 30)),
            (
                165,
                2,
                "demo.item.heavy-crossbow",
                (9, 2, 14, 2, 0, -2, -2, 94, 30),
            ),
            (166, 7, "demo.item.sling", (9, 7, 2, 0, 0, 2, 0, 58, 30)),
        ] {
            let (materialization, draws) = roll_launcher_ego(source_index, seed, kind_id);
            let state = &materialization.rolled_affixes[0];
            let modifiers = &state.properties.modifiers;
            let bonuses = &state.properties.equipment_bonuses;
            assert_eq!(
                (
                    draws,
                    state.enchantment_delta.to_hit,
                    state.enchantment_delta.to_damage,
                    modifiers.strength,
                    modifiers.dexterity,
                    modifiers.speed,
                    bonuses.stealth_skill,
                    bonuses.launcher_multiplier_delta_percent,
                    bonuses.base_shot_delta_percent,
                ),
                expected,
                "source {source_index}"
            );
        }

        for (source_index, seed, kind_id, damage_type) in [
            (
                164,
                0xE4_3164,
                "demo.item.long-bow",
                ActorDamageType::Nether,
            ),
            (
                165,
                0xE4_3165,
                "demo.item.heavy-crossbow",
                ActorDamageType::Chaos,
            ),
            (
                166,
                0xE4_3166,
                "demo.item.sling",
                ActorDamageType::Disenchant,
            ),
        ] {
            let (materialization, _) = roll_launcher_ego(source_index, seed, kind_id);
            assert_eq!(
                materialization.rolled_affixes[0]
                    .properties
                    .resistances
                    .get(&damage_type),
                Some(&ActorResistanceLevel::Resistant)
            );
        }
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
