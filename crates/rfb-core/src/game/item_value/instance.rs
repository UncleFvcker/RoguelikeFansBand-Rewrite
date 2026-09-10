// SPDX-License-Identifier: MPL-2.0

use super::ValueObject;
use crate::state::ItemInstance;
use rfb_content::*;
use rfb_protocol::{ItemCurseSeverityDto, WeaponTraitDto};

/// Full instance input, independent of player knowledge, pricing, and RNG.
/// Missing authoritative metadata is unsupported, never an approximate price.
pub(in crate::game) fn value_object(
    content: &ContentCatalog,
    item: &ItemInstance,
) -> Option<ValueObject> {
    let definition = content.item(&item.kind_id)?;
    let base = match &definition.artifact_generation {
        Some(artifact) => content.item(&artifact.base_item_kind_id)?,
        None => definition,
    };
    let kind = base.rfb_base_kind?;
    let raw = definition.rfb_value.as_ref()?;
    let mut object = ValueObject {
        tval: kind.tval,
        sval: kind.sval,
        // obj_create_lite moves the kind's fuel pval to xtra4, then clears pval.
        pval: if kind.tval == 39 && kind.sval <= 1 {
            0
        } else {
            i32::from(raw.pval)
        },
        flags: base
            .rfb_value
            .as_ref()?
            .flags
            .union(&raw.flags)
            .cloned()
            .collect(),
        ac: definition.modifiers.defense - i32::from(raw.to_armor),
        to_a: i32::from(raw.to_armor) + i32::from(item.enchantments.to_armor),
        to_h: definition.equipment_bonuses.melee_skill + i32::from(item.enchantments.to_hit),
        to_d: definition.equipment_bonuses.melee_damage + i32::from(item.enchantments.to_damage),
        base_to_h: base.equipment_bonuses.melee_skill,
        weight: i32::from(
            item.weight_override()
                .unwrap_or(definition.weight_tenths_pound),
        ),
        capacity: i32::from(
            crate::game::ego::base_bag_capacity(definition)
                .unwrap_or(definition.ammunition_capacity),
        ),
        fixed_artifact: definition
            .artifact_generation
            .as_ref()
            .map_or(0, |art| art.source_index),
        artifact: item.is_artifact(content),
        permanent_curse: item.curse == Some(ItemCurseSeverityDto::Permanent),
        ..Default::default()
    };
    if let Some(profile) = &definition.melee_profile {
        let base = base.melee_profile.as_ref()?;
        object.dd = i32::from(profile.damage_dice);
        object.ds = i32::from(profile.damage_sides);
        object.base_dd = i32::from(base.damage_dice);
        object.base_ds = i32::from(base.damage_sides);
        object.to_h += profile.to_hit;
        object.to_d += profile.to_damage;
    } else if let Some(profile) = &definition.ammunition_profile {
        let base = base.ammunition_profile.as_ref()?;
        object.dd = i32::from(item.damage_dice_override.unwrap_or(profile.damage_dice));
        object.ds = i32::from(profile.damage_sides);
        object.base_dd = i32::from(base.damage_dice);
        object.base_ds = i32::from(base.damage_sides);
        object.to_h += profile.to_hit;
        object.to_d += profile.to_damage;
    }
    if let Some(profile) = &definition.projectile_profile {
        object.mult = i32::from(profile.damage_multiplier_percent);
        object.base_mult = i32::from(base.projectile_profile.as_ref()?.damage_multiplier_percent);
        object.to_h += profile.to_hit;
        object.to_d += profile.to_damage;
    }
    for id in &item.affix_ids {
        let affix = content.affix(id)?;
        if let Some(ego) = &affix.rfb_ego {
            object.ego = ego.source_index;
            // obj_flags omits these static light egos when the fuel is empty.
            if !(kind.tval == 39
                && kind.sval <= 1
                && item.fuel.as_ref().is_some_and(|fuel| fuel.current == 0)
                && matches!(ego.source_index, 238 | 239 | 241))
            {
                object.flags.extend(ego.flags.iter().cloned());
            }
        }
        object.ac += affix.modifiers.defense;
        object.to_h += affix.equipment_bonuses.melee_skill;
        object.to_d += affix.equipment_bonuses.melee_damage;
        if affix.rfb_ego.is_none() {
            object.properties(&AffixPropertyBundleDefinition {
                resistances: affix.resistances.clone(),
                status_immunities: affix.status_immunities.clone(),
                slays: affix.slays.clone(),
                brands: affix.brands.clone(),
                passives: affix.passives.clone(),
                ..Default::default()
            });
        }
    }
    object.properties(&item.intrinsic_properties);
    if let Some(dice) = item.melee_damage_dice() {
        object.dd = i32::from(dice.dice);
        object.ds = i32::from(dice.sides);
    }
    for rolled in &item.rolled_affixes {
        object.properties(&rolled.properties);
    }
    for trait_ in item.intrinsic_weapon_traits.iter().chain(
        item.rolled_affixes
            .iter()
            .flat_map(|roll| &roll.weapon_traits),
    ) {
        object.flags.insert(
            match trait_ {
                WeaponTraitDto::ManaBrand => "BRAND_MANA",
                WeaponTraitDto::Vorpal => "VORPAL",
                WeaponTraitDto::Vorpal2 => "VORPAL2",
                WeaponTraitDto::Order => "ORDER",
                WeaponTraitDto::Wild => "BRAND_WILD",
                WeaponTraitDto::Impact => "IMPACT",
                WeaponTraitDto::Stun => "STUN",
                WeaponTraitDto::Blessed => "BLESSED",
            }
            .to_owned(),
        );
    }
    if definition.vorpal {
        object.flags.insert("VORPAL".to_owned());
    }
    for element in item.permanent_destruction_immunities.iter().chain(
        item.rolled_affixes
            .iter()
            .flat_map(|roll| &roll.elemental_destruction_immunities),
    ) {
        object.flags.insert(
            match element {
                ItemDestructionElement::Acid => "IGNORE_ACID",
                ItemDestructionElement::Electricity => "IGNORE_ELEC",
                ItemDestructionElement::Fire => "IGNORE_FIRE",
                ItemDestructionElement::Cold => "IGNORE_COLD",
            }
            .to_owned(),
        );
    }
    if let Some(activation) = &item.activation {
        let generation = crate::game::item_device_generation(
            content,
            &item.kind_id,
            &item.affix_ids,
            Some(&activation.profile_id),
            item.artifact_name.is_some(),
        )?;
        let profile = generation
            .activations
            .iter()
            .find(|profile| profile.id == activation.profile_id)?;
        if i32::from(activation.power) != profile.device_check_difficulty {
            return None;
        }
        object.activation_value = profile.rfb_value?;
        object.activation_timeout = profile
            .recovery
            .or(generation.recovery)
            .map_or(0, |recovery| i32::from(recovery.interval_ticks) / 10);
    }
    Some(object)
}

impl ValueObject {
    pub(in crate::game) fn properties(&mut self, properties: &AffixPropertyBundleDefinition) {
        self.flags.extend(properties.rfb_flags.iter().cloned());
        if let Some(pval) = &properties.rfb_pval {
            self.pval = i32::from(pval.value);
            self.flags
                .extend(pval.flags.iter().map(|flag| flag.source_flag().to_owned()));
        }
        self.ac += properties.modifiers.defense;
        self.to_h += properties.equipment_bonuses.melee_skill;
        self.to_d += properties.equipment_bonuses.melee_damage;
        self.mult += properties
            .equipment_bonuses
            .launcher_multiplier_delta_percent;
        if let Some(capacity) = properties.bag_capacity.or(properties.ammunition_capacity) {
            self.capacity = i32::from(capacity);
        }
        for (element, tier) in &properties.resistances {
            let element = match element {
                ActorDamageType::Acid => "ACID",
                ActorDamageType::Electricity => "ELEC",
                ActorDamageType::Fire => "FIRE",
                ActorDamageType::Cold => "COLD",
                ActorDamageType::Poison => "POIS",
                ActorDamageType::Light => "LITE",
                ActorDamageType::Dark => "DARK",
                ActorDamageType::Blindness => "BLIND",
                ActorDamageType::Fear => "FEAR",
                ActorDamageType::Confusion => "CONF",
                ActorDamageType::Nether => "NETHER",
                ActorDamageType::Nexus => "NEXUS",
                ActorDamageType::Sound => "SOUND",
                ActorDamageType::Shards => "SHARDS",
                ActorDamageType::Chaos => "CHAOS",
                ActorDamageType::Disenchant => "DISEN",
                ActorDamageType::Time => "TIME",
                _ => continue,
            };
            let prefix = match tier {
                ActorResistanceLevel::Vulnerable => "VULN",
                ActorResistanceLevel::Resistant | ActorResistanceLevel::Strong => "RES",
                ActorResistanceLevel::Immune => "IM",
            };
            self.flags.insert(format!("{prefix}_{element}"));
        }
        for (target, level) in &properties.slays {
            let target = match target {
                SlayTarget::Animal => "ANIMAL",
                SlayTarget::Evil => "EVIL",
                SlayTarget::Good => "GOOD",
                SlayTarget::Living => "LIVING",
                SlayTarget::Human => "HUMAN",
                SlayTarget::Undead => "UNDEAD",
                SlayTarget::Demon => "DEMON",
                SlayTarget::Orc => "ORC",
                SlayTarget::Troll => "TROLL",
                SlayTarget::Giant => "GIANT",
                SlayTarget::Dragon => "DRAGON",
            };
            let prefix = match level {
                SlayLevel::Slay => "SLAY",
                SlayLevel::Kill => "KILL",
            };
            self.flags.insert(format!("{prefix}_{target}"));
        }
        for brand in &properties.brands {
            self.flags.insert(
                match brand {
                    WeaponBrand::Acid => "BRAND_ACID",
                    WeaponBrand::Electricity => "BRAND_ELEC",
                    WeaponBrand::Fire => "BRAND_FIRE",
                    WeaponBrand::Cold => "BRAND_COLD",
                    WeaponBrand::Poison => "BRAND_POIS",
                    WeaponBrand::Chaos => "BRAND_CHAOS",
                }
                .to_owned(),
            );
        }
        for status in &properties.status_immunities {
            let flag = match status.as_str() {
                crate::effect::STATUS_PARALYSIS => "FREE_ACT",
                crate::effect::STATUS_BLINDNESS => "RES_BLIND",
                crate::effect::STATUS_FEAR => "RES_FEAR",
                _ => continue,
            };
            self.flags.insert(flag.to_owned());
        }
        for passive in &properties.passives {
            self.flags.insert(passive_flag(*passive).to_owned());
        }
    }
}

pub(in crate::game) fn passive_flag(passive: EquipmentPassive) -> &'static str {
    use EquipmentPassive::*;
    match passive {
        Regeneration => "REGEN",
        SeeInvisible => "SEE_INVIS",
        Vampiric => "BRAND_VAMP",
        HoldLife => "HOLD_LIFE",
        Levitation => "LEVITATION",
        Warning => "WARNING",
        SlowDigestion => "SLOW_DIGEST",
        ReflectsBolts => "REFLECT",
        FireAura => "AURA_FIRE",
        ColdAura => "AURA_COLD",
        ElectricityAura => "AURA_ELEC",
        RevengeAura => "AURA_REVENGE",
        ManaRegeneration => "REGEN_MANA",
        AntiMagic => "NO_MAGIC",
        AntiTeleport => "NO_TELE",
        AntiSummoning => "NO_SUMMON",
        NightVision => "NIGHT_VISION",
        DualWielding => "DUAL_WIELDING",
        NoEnchant => "NO_ENCHANT",
        ShardsAura => "AURA_SHARDS",
        ReducedManaCost => "DEC_MANA",
        EasySpell => "EASY_SPELL",
        AutoIdentify => "LORE2",
        Blessed => "BLESSED",
        EspAnimal => "ESP_ANIMAL",
        EspUndead => "ESP_UNDEAD",
        EspDemon => "ESP_DEMON",
        EspOrc => "ESP_ORC",
        EspTroll => "ESP_TROLL",
        EspGiant => "ESP_GIANT",
        EspDragon => "ESP_DRAGON",
        EspHuman => "ESP_HUMAN",
        EspGood => "ESP_GOOD",
        EspEvil => "ESP_EVIL",
        EspLiving => "ESP_LIVING",
        EspNonliving => "ESP_NONLIVING",
        Telepathy => "TELEPATHY",
        SustainStrength => "SUST_STR",
        SustainIntelligence => "SUST_INT",
        SustainWisdom => "SUST_WIS",
        SustainDexterity => "SUST_DEX",
        SustainConstitution => "SUST_CON",
        SustainCharisma => "SUST_CHR",
    }
}
