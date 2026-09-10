// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::state::ItemInstance;
use rfb_content::*;
use rfb_protocol::{ItemQualityDto, MeleeDamageDiceDto, WeaponTraitDto};

/// artifact.c::create_replacement_art, used when a named quest reward already exists.
/// Candidate rejection still consumes RNG and interns names, exactly like creation.
pub(in crate::game) fn materialize_replacement(
    content: &ContentCatalog,
    rng: &mut RfbRng,
    fixed: &ItemInstance,
    class_id: &str,
    quarks: &mut BTreeSet<String>,
) -> Option<ItemInstance> {
    let definition = content.item(&fixed.kind_id)?;
    let base = content.item(&definition.artifact_generation.as_ref()?.base_item_kind_id)?;
    let data = content.random_artifact_generation()?;
    let mut original_value = crate::game::item_value::instance::value_object(content, fixed)?;
    if matches!(original_value.tval, 16..=18 | 21..=23) {
        original_value.to_h = original_value.to_h.max(10);
        original_value.to_d = original_value.to_d.max(10);
    } else if (30..=38).contains(&original_value.tval) {
        original_value.to_a = original_value.to_a.max(10);
    }
    let mut base_power = crate::game::item_value::object_value(original_value)?;
    if !(16..=18).contains(&base.rfb_base_kind?.tval) {
        base_power = base_power.max(7500);
    }
    let minimum = (base_power * 4 / 5).max(base_power - 10_000);
    let maximum = (base_power * 6 / 5).min(base_power + 10_000);
    let mut original = fixed.clone();
    original.kind_id = base.id.clone();
    original.activation = None;
    original.charges = None;
    original.device_recovery_progress = 0;
    original.fuel = crate::game::initial_item_fuel(content, &base.id);
    let mut object = crate::game::item_value::instance::value_object(content, &original)?;
    object.flags.retain(|flag| valid_rfb_runtime_flag(flag));
    let creation = Creation {
        level: i32::from(definition.generation_level),
        class_id,
        good: true,
        base_flags: Some(&base.rfb_value.as_ref()?.flags),
        ..Default::default()
    };
    let mut best = None;
    let mut worst = None;
    let mut best_power = i32::MIN;
    let mut worst_power = i32::MAX;
    for _ in 0..10_000 {
        let roll = create_artifact(
            rng,
            data,
            object.clone(),
            creation,
            quarks,
            None,
            false,
            i32::from(base.weight_tenths_pound),
        )?;
        let power = score(&roll.object, creation.base_flags);
        let item = apply_roll(base, &original, roll);
        if (minimum..=maximum).contains(&power) {
            let mut item = item;
            item.intrinsic_weight_tenths_pound = Some(definition.weight_tenths_pound);
            return Some(item);
        }
        if power > best_power {
            best_power = power;
            best = Some(item.clone());
        }
        if power < worst_power {
            worst_power = power;
            worst = Some(item);
        }
    }
    let mut item = if worst_power > base_power {
        worst?
    } else {
        best?
    };
    item.intrinsic_weight_tenths_pound = Some(definition.weight_tenths_pound);
    Some(item)
}

/// E8.5b's item factory. The caller supplies the name table shared by consecutive
/// generations and commits only the returned instance; IDs are never allocated here.
#[allow(clippy::too_many_arguments)] // Mirrors the caller's existing generation state.
pub(in crate::game) fn materialize(
    content: &ContentCatalog,
    rng: &mut RfbRng,
    original: &ItemInstance,
    creation: Creation<'_>,
    quarks: &mut BTreeSet<String>,
    level: i32,
    power: i16,
    jewelry_level_adjusted: &mut bool,
    bad_luck: bool,
) -> Option<(ItemInstance, usize)> {
    let definition = content.item(&original.kind_id)?;
    let data = content.random_artifact_generation()?;
    let mut object = crate::game::item_value::instance::value_object(content, original)?;
    // k_info also has allocation directives (TOWN, INSTA_ART); they are not OFs.
    object.flags.retain(|flag| valid_rfb_runtime_flag(flag));
    let raw = definition.rfb_value.as_ref()?;
    let creation = Creation {
        base_flags: Some(&raw.flags),
        ..creation
    };
    let base_ac = definition.modifiers.defense - i32::from(raw.to_armor);
    let (roll, attempts) = art_create_random(
        rng,
        data,
        &object,
        creation,
        quarks,
        level,
        power,
        jewelry_level_adjusted,
        bad_luck,
        original.curse,
        original.intrinsic_properties.rfb_heavy_curse
            || original
                .rolled_affixes
                .iter()
                .any(|roll| roll.properties.rfb_heavy_curse),
        base_ac,
        i32::from(definition.weight_tenths_pound),
    )?;
    Some((apply_roll(definition, original, roll), attempts))
}

pub(super) fn apply_roll(
    definition: &ItemDefinition,
    original: &ItemInstance,
    roll: ArtifactRoll<'_>,
) -> ItemInstance {
    let object = &roll.object;
    let raw = definition.rfb_value.as_ref().expect("RFB kind value");
    let flags = effective_flags(object, Some(&raw.flags));
    let mut result = original.clone();
    result.artifact_name = Some(roll.name);
    result.quantity = 1;
    result.quality = ItemQualityDto::Exceptional;
    result.affix_ids.clear();
    result.rolled_affixes.clear();
    result.intrinsic_properties = AffixPropertyBundleDefinition::default();
    result.intrinsic_weapon_traits.clear();
    result.intrinsic_curse_effects.extend(roll.curse_effects);
    for rolled in &original.rolled_affixes {
        result
            .intrinsic_curse_effects
            .extend(rolled.curse_effects.iter().copied());
    }
    result.curse = roll.curse;
    result.intrinsic_weight_tenths_pound =
        Some(object.weight.try_into().expect("source artifact weight"));
    result.intrinsic_melee_damage_dice =
        definition
            .melee_profile
            .as_ref()
            .map(|_| MeleeDamageDiceDto {
                dice: object.dd.try_into().expect("source artifact dice"),
                sides: object.ds.try_into().expect("source artifact sides"),
            });
    let (base_h, base_d) = definition
        .melee_profile
        .as_ref()
        .map(|p| (p.to_hit, p.to_damage))
        .or_else(|| {
            definition
                .projectile_profile
                .as_ref()
                .map(|p| (p.to_hit, p.to_damage))
        })
        .unwrap_or_default();
    result.enchantments = rfb_protocol::ItemEnchantmentsDto {
        to_hit: (object.to_h - base_h - definition.equipment_bonuses.melee_skill)
            .try_into()
            .expect("source artifact hit"),
        to_damage: (object.to_d - base_d - definition.equipment_bonuses.melee_damage)
            .try_into()
            .expect("source artifact damage"),
        to_armor: (object.to_a - i32::from(raw.to_armor))
            .try_into()
            .expect("source artifact armor"),
    };
    let properties = &mut result.intrinsic_properties;
    properties.modifiers.defense =
        object.ac - (definition.modifiers.defense - i32::from(raw.to_armor));
    // Current kinds project cloak search and falcon blows in older units. Replace
    // those base values with the final shared pval projection, without counting twice.
    properties.equipment_bonuses.stealth_skill = -definition.equipment_bonuses.stealth_skill;
    properties.equipment_bonuses.search_skill = -definition.equipment_bonuses.search_skill;
    properties.equipment_bonuses.perception_skill = -definition.equipment_bonuses.perception_skill;
    properties.equipment_bonuses.melee_attacks = -definition.equipment_bonuses.melee_attacks;
    let pval_flags: BTreeSet<_> = RfbPvalFlagDefinition::ALL
        .into_iter()
        .filter(|flag| flags.contains(flag.source_flag()))
        .collect();
    for flag in &pval_flags {
        crate::game::ego::armor::apply_pval(properties, *flag, object.pval);
    }
    properties.rfb_pval = Some(RfbPvalDefinition {
        value: object.pval.try_into().expect("source artifact pval"),
        flags: pval_flags,
    });
    properties.rfb_flags = object.flags.clone();
    properties.rfb_heavy_curse = roll.heavy_curse;
    if object.tval == 19 {
        properties
            .equipment_bonuses
            .launcher_multiplier_delta_percent = object.mult - object.base_mult;
    }
    properties.equipment_bonuses.light_radius =
        i32::from(flags.contains("LITE")) - i32::from(flags.contains("DARKNESS"));
    for (element, token) in resistance_elements() {
        let resistant = flags.contains(&format!("RES_{token}"));
        let vulnerable = flags.contains(&format!("VULN_{token}"));
        let tier = if flags.contains(&format!("IM_{token}")) {
            Some(ActorResistanceLevel::Immune)
        } else if resistant && !vulnerable {
            Some(ActorResistanceLevel::Resistant)
        } else if vulnerable && !resistant {
            Some(ActorResistanceLevel::Vulnerable)
        } else {
            None
        };
        if let Some(tier) = tier {
            properties.resistances.insert(element, tier);
        }
    }
    for (flag, status) in [
        ("FREE_ACT", crate::effect::STATUS_PARALYSIS),
        ("RES_BLIND", crate::effect::STATUS_BLINDNESS),
        ("RES_FEAR", crate::effect::STATUS_FEAR),
    ] {
        if flags.contains(flag) {
            properties.status_immunities.push(status.to_owned());
        }
    }
    use EquipmentPassive::*;
    for passive in [
        Regeneration,
        SeeInvisible,
        Vampiric,
        HoldLife,
        Levitation,
        Warning,
        SlowDigestion,
        ReflectsBolts,
        FireAura,
        ColdAura,
        ElectricityAura,
        RevengeAura,
        ManaRegeneration,
        AntiMagic,
        AntiTeleport,
        AntiSummoning,
        NightVision,
        DualWielding,
        NoEnchant,
        ShardsAura,
        ReducedManaCost,
        EasySpell,
        AutoIdentify,
        Blessed,
        EspAnimal,
        EspUndead,
        EspDemon,
        EspOrc,
        EspTroll,
        EspGiant,
        EspDragon,
        EspHuman,
        EspGood,
        EspEvil,
        EspLiving,
        EspNonliving,
        Telepathy,
        SustainStrength,
        SustainIntelligence,
        SustainWisdom,
        SustainDexterity,
        SustainConstitution,
        SustainCharisma,
    ] {
        if flags.contains(crate::game::item_value::instance::passive_flag(passive)) {
            properties.passives.insert(passive);
        }
    }
    for (target, token) in [
        (SlayTarget::Animal, "ANIMAL"),
        (SlayTarget::Evil, "EVIL"),
        (SlayTarget::Good, "GOOD"),
        (SlayTarget::Living, "LIVING"),
        (SlayTarget::Human, "HUMAN"),
        (SlayTarget::Undead, "UNDEAD"),
        (SlayTarget::Demon, "DEMON"),
        (SlayTarget::Orc, "ORC"),
        (SlayTarget::Troll, "TROLL"),
        (SlayTarget::Giant, "GIANT"),
        (SlayTarget::Dragon, "DRAGON"),
    ] {
        let tier = if flags.contains(&format!("KILL_{token}")) {
            Some(SlayLevel::Kill)
        } else if flags.contains(&format!("SLAY_{token}")) {
            Some(SlayLevel::Slay)
        } else {
            None
        };
        if let Some(tier) = tier {
            properties.slays.insert(target, tier);
        }
    }
    for (brand, flag) in [
        (WeaponBrand::Acid, "BRAND_ACID"),
        (WeaponBrand::Electricity, "BRAND_ELEC"),
        (WeaponBrand::Fire, "BRAND_FIRE"),
        (WeaponBrand::Cold, "BRAND_COLD"),
        (WeaponBrand::Poison, "BRAND_POIS"),
        (WeaponBrand::Chaos, "BRAND_CHAOS"),
    ] {
        if flags.contains(flag) {
            properties.brands.insert(brand);
        }
    }
    for (trait_, flag) in [
        (WeaponTraitDto::ManaBrand, "BRAND_MANA"),
        (WeaponTraitDto::Vorpal, "VORPAL"),
        (WeaponTraitDto::Vorpal2, "VORPAL2"),
        (WeaponTraitDto::Order, "ORDER"),
        (WeaponTraitDto::Wild, "BRAND_WILD"),
        (WeaponTraitDto::Impact, "IMPACT"),
        (WeaponTraitDto::Stun, "STUN"),
        (WeaponTraitDto::Blessed, "BLESSED"),
    ] {
        if flags.contains(flag) {
            result.intrinsic_weapon_traits.insert(trait_);
        }
    }
    result.permanent_destruction_immunities = BTreeSet::from([
        ItemDestructionElement::Acid,
        ItemDestructionElement::Electricity,
        ItemDestructionElement::Fire,
        ItemDestructionElement::Cold,
    ]);
    if let Some(profile) = roll.activation {
        let (activation, charges) = crate::game::ego::materialize_rfb_activation(profile);
        result.activation = Some(activation);
        result.charges = Some(charges);
        result.device_recovery_progress = 0;
    }
    result
}

pub(in crate::game) fn resistance_elements() -> [(ActorDamageType, &'static str); 17] {
    use ActorDamageType::*;
    [
        Acid,
        Electricity,
        Fire,
        Cold,
        Poison,
        Light,
        Dark,
        Blindness,
        Fear,
        Confusion,
        Nether,
        Nexus,
        Sound,
        Shards,
        Chaos,
        Disenchant,
        Time,
    ]
    .map(|element| (element, crate::game::ego::rfb_resistance_element(element)))
}
