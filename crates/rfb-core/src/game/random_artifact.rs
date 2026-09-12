// Adapted from RFB master; upstream copyright and terms are preserved in NOTICE.
//! RFB master artifact.c / ego.c random-artifact construction.
//! Source: a0d92b6378d148c5262cc236b8fa6ed2ca06a54c (see NOTICE).

mod construction;
mod materialization;
mod names;
mod powers;
pub(super) mod scheduling;
#[cfg(test)]
mod tests;
pub(super) use materialization::materialize;
pub(super) use materialization::materialize_replacement;
pub(super) use materialization::materialize_scroll;
pub(super) use materialization::resistance_elements;
pub(super) use names::intern as intern_name;

pub(super) fn names_are_valid(names: &BTreeSet<String>) -> bool {
    names.len() <= names::QUARK_CAPACITY
        && (names.is_empty() || names.contains(""))
        && names.iter().all(|name| {
            (name.is_empty() || !name.trim().is_empty())
                && name.len() < 1024
                && !name.chars().any(char::is_control)
        })
}

use super::item_value::ValueObject;
use crate::rng::{RfbRng, rfb_m_bonus};
use rfb_content::{RandomArtifactGenerationDefinition, RfbPvalFlagDefinition};
use rfb_protocol::{ItemCurseEffectDto, ItemCurseSeverityDto};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Default)]
pub(super) struct Creation<'a> {
    pub level: i32,
    pub class_id: &'a str,
    pub theme: &'a str,
    pub good: bool,
    pub cursed: bool,
    pub scroll: bool,
    pub requested_name: Option<&'a str>,
    pub no_artifacts: bool,
    pub base_flags: Option<&'a BTreeSet<String>>,
}

pub(super) struct ArtifactRoll<'a> {
    pub value: i32,
    pub object: ValueObject,
    pub name: String,
    pub activation: Option<&'a rfb_content::ItemDeviceActivationDefinition>,
    pub curse: Option<ItemCurseSeverityDto>,
    pub heavy_curse: bool,
    pub curse_effects: BTreeSet<ItemCurseEffectDto>,
}

fn slot_weight(object: &ValueObject, class_id: &str) -> i32 {
    if object.tval == 19 && object.sval > 24 && object.sval != 63 {
        return if object.sval == 70 && class_id == "demo.class.bard" {
            50
        } else {
            40
        };
    }
    match object.tval {
        34 => 52,
        19 => 60,
        45 => 55,
        40 => 40,
        39 => 36,
        35 => 43,
        32 | 33 | 30 => 50,
        31 => 45,
        _ => 80,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ValueLimits {
    minimum: i32,
    maximum: i32,
    soft_maximum: i32,
}

fn value_limits(
    rng: &mut RfbRng,
    object: &ValueObject,
    class_id: &str,
    level: i32,
    jewelry_level_adjusted: &mut bool,
    bad_luck: bool,
    base_ac: i32,
) -> ValueLimits {
    use super::item_value::interpolate;
    let mut minimum = interpolate(level, &[(20, 0), (70, 50_000)]);
    let mut maximum = interpolate(
        level,
        &[
            (0, 5_000),
            (10, 10_000),
            (30, 30_000),
            (70, 100_000),
            (100, 150_000),
        ],
    );
    let mut weight = slot_weight(object, class_id);
    if (20..=23).contains(&object.tval) && object.base_dd * object.base_ds < 12 {
        weight = weight * object.base_dd * object.base_ds / 12;
    } else if (36..=38).contains(&object.tval) && base_ac < 16 {
        weight = weight * (base_ac + 20) / 36;
    }
    let percent = weight * 100 / 80;
    if *jewelry_level_adjusted && object.tval == 40 && (bad_luck || rng.bounded(14) != 0) {
        minimum = minimum.min(43_560);
        if maximum > 58_564 {
            maximum = (maximum + 58_564) / 2;
        }
    }
    *jewelry_level_adjusted = false;
    minimum = minimum * percent / 100;
    maximum = maximum * percent / 100;
    ValueLimits {
        minimum,
        maximum,
        soft_maximum: (800 * percent).min(5_000.max(maximum * 7 / 8)),
    }
}

/// ego.c:_art_create_random. Each attempt copies exactly the same input; only
/// source RNG and name quarks survive rejection. There is no item-ID allocation.
#[allow(clippy::too_many_arguments)] // Keep source inputs explicit at the candidate boundary.
pub(super) fn art_create_random<'a>(
    rng: &mut RfbRng,
    data: &'a RandomArtifactGenerationDefinition,
    object: &ValueObject,
    mut creation: Creation<'a>,
    quarks: &mut BTreeSet<String>,
    level: i32,
    power: i16,
    jewelry_level_adjusted: &mut bool,
    bad_luck: bool,
    curse: Option<ItemCurseSeverityDto>,
    heavy_curse: bool,
    base_ac: i32,
    base_weight: i32,
) -> Option<(ArtifactRoll<'a>, usize)> {
    let limits = value_limits(
        rng,
        object,
        creation.class_id,
        level,
        jewelry_level_adjusted,
        bad_luck,
        base_ac,
    );
    creation.good = false;
    creation.scroll = false;
    creation.cursed = power < 0;
    for attempt in 1..=1001 {
        let roll = create_artifact(
            rng,
            data,
            object.clone(),
            creation,
            quarks,
            curse,
            heavy_curse,
            base_weight,
        )?;
        if attempt == 1001 {
            return Some((roll, attempt));
        }
        let score = score(&roll.object, creation.base_flags);
        if !accepts_score(rng, score, limits) {
            continue;
        }
        return Some((roll, attempt));
    }
    unreachable!("attempt 1001 is unconditional")
}

fn accepts_score(rng: &mut RfbRng, score: i32, limits: ValueLimits) -> bool {
    score >= limits.minimum
        && score <= limits.maximum
        && (score <= limits.soft_maximum || rng.bounded(2) != 0)
}

/// Construct one candidate. The caller retains the original item identity and
/// owns the name quarks; rejected candidates still intern their generated names.
#[allow(clippy::too_many_arguments)] // Base item state and generation mode are independent inputs.
pub(super) fn create_artifact<'a>(
    rng: &mut RfbRng,
    data: &'a RandomArtifactGenerationDefinition,
    mut object: ValueObject,
    creation: Creation<'a>,
    quarks: &mut BTreeSet<String>,
    curse: Option<ItemCurseSeverityDto>,
    heavy_curse: bool,
    base_weight: i32,
) -> Option<ArtifactRoll<'a>> {
    if creation.no_artifacts
        || object.flags.contains("NO_REMOVE")
        || !matches!(object.tval, 16..=23 | 30..=40 | 45)
    {
        return None;
    }
    quarks.insert(String::new()); // quark_init reserves the empty emergency string.
    let has_activation = object.activation_value != 0 || object.flags.contains("ACTIVATE");
    if object.tval == 39 {
        object.pval = 0;
    }
    object.fixed_artifact = 0;
    object.ego = 0;
    if let Some(flags) = creation.base_flags {
        object.flags.extend(
            flags
                .iter()
                .filter(|flag| rfb_content::valid_rfb_runtime_flag(flag))
                .cloned(),
        );
    }
    let mut generator = Generator {
        rng,
        data,
        has_pval: object.pval != 0 || (object.tval, object.sval) == (19, 70),
        object,
        level: creation.level,
        class_id: creation.class_id,
        bias: Bias::None,
        immunity_replaced: false,
        slaying: 0,
        activation: None,
        has_activation,
    };
    generator.initial_bias(creation);
    let cursed = (!creation.good && generator.one(13))
        || creation.cursed
        || (matches!(generator.object.tval, 40 | 45) && curse.is_some());
    let level = creation.level.clamp(1, 127);
    let powers = generator.roll_powers(level);
    generator.construct(powers, level);
    generator.finish(level, creation.scroll);
    if generator.object.tval == 39 {
        names::intern(quarks, "临时".to_owned());
        generator.object.artifact = true;
    }
    if generator.melee() && creation.class_id == "demo.class.mauler" {
        let original = generator.object.base_dd * generator.object.base_ds;
        let mut multiplier =
            (generator.object.dd * generator.object.ds - original) * 100 / original;
        let mut extra = base_weight;
        while multiplier >= 100 {
            extra = extra * 3 / 4;
            generator.object.weight += extra;
            multiplier -= 100;
        }
        if multiplier > 0 {
            generator.object.weight += extra * 3 / 4 * multiplier / 100;
        }
    }
    generator.sanitize();
    let cost = score(&generator.object, creation.base_flags);
    let power = generator.name_power(cost);
    let name = if creation.scroll && creation.requested_name.is_some_and(|name| !name.is_empty()) {
        format!("'{}'", creation.requested_name.unwrap())
    } else {
        generator.random_name(quarks, power)
    };
    let mut severity = curse;
    let mut heavy = heavy_curse;
    let mut effects = BTreeSet::new();
    if !creation.scroll && cursed {
        let flags = effective_flags(&generator.object, creation.base_flags);
        let roll = super::ego::curses::roll_curse(
            generator.rng,
            score(&generator.object, creation.base_flags),
            generator.object.tval,
            &flags,
        );
        generator.has_pval |= RfbPvalFlagDefinition::ALL
            .iter()
            .any(|flag| roll.flags.contains(flag.source_flag()));
        generator.object.flags.extend(roll.flags);
        severity = Some(curse.map_or(roll.severity, |old| old.max(roll.severity)));
        heavy |= roll.heavy;
        effects = roll.effects;
        generator.object.permanent_curse = severity == Some(ItemCurseSeverityDto::Permanent);
        if generator.has_pval && generator.object.pval == 0 {
            generator.object.pval = generator.roll(5);
        }
    }
    let name = names::intern(quarks, name);
    generator.object.artifact = true;
    Some(ArtifactRoll {
        value: cost,
        object: generator.object,
        name,
        activation: generator.activation,
        curse: severity,
        heavy_curse: heavy,
        curse_effects: effects,
    })
}

/// Original bias bit positions also select sections in the name files.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u8)]
enum Bias {
    #[default]
    None = 0,
    Electricity,
    Poison,
    Fire,
    Cold,
    Acid,
    Strength,
    Intelligence,
    Wisdom,
    Dexterity,
    Constitution,
    Charisma,
    Chaos,
    Priestly,
    Necromantic,
    Law,
    Rogue,
    Mage,
    Warrior,
    Ranger,
    Archer = 22,
}

struct Generator<'rng, 'a> {
    rng: &'rng mut RfbRng,
    object: ValueObject,
    level: i32,
    class_id: &'a str,
    bias: Bias,
    has_pval: bool,
    immunity_replaced: bool,
    slaying: i32,
    data: &'a rfb_content::RandomArtifactGenerationDefinition,
    activation: Option<&'a rfb_content::ItemDeviceActivationDefinition>,
    has_activation: bool,
}

fn effective_flags(object: &ValueObject, base: Option<&BTreeSet<String>>) -> BTreeSet<String> {
    let mut flags = object.flags.clone();
    if let Some(base) = base {
        flags.extend(base.iter().cloned());
    }
    flags
}

fn score(object: &ValueObject, base: Option<&BTreeSet<String>>) -> i32 {
    let mut effective = object.clone();
    effective.flags = effective_flags(object, base);
    super::item_value::object_value(effective).expect("supported artifact slot")
}

fn trim(rng: &mut RfbRng, start: i32, high: i32, very_high: i32, level: i32) -> i32 {
    let high = high.min(very_high);
    if start <= high {
        return start;
    }
    let mut reduction = 0;
    for _ in 0..(very_high - high).min(start - high) {
        if rng.bounded(2) == 0 || rng.bounded((100 + level) as u64) < 80 {
            reduction += 1;
        }
    }
    for _ in 0..start - reduction - very_high {
        if rng.bounded(4) != 0 || rng.bounded((100 + level) as u64) < 120 {
            reduction += 1;
        }
    }
    start - reduction
}

impl Generator<'_, '_> {
    fn initial_bias(&mut self, creation: Creation<'_>) {
        use Bias::*;
        let (bias, mut warrior_chance) = match creation.theme {
            "warrior" | "warrior-shoot" | "samurai" | "dwarf" => (Warrior, 0),
            "archer" => (Archer, 0),
            "mage" => (Mage, 0),
            "priest" => (Priestly, 0),
            "priest-evil" => (Necromantic, 0),
            "paladin" => (Law, 60),
            "paladin-evil" => (Necromantic, 60),
            "ninja" | "hobbit" => (Rogue, 0),
            "rogue" => (Rogue, 50),
            _ => (None, 0),
        };
        self.bias = bias;
        if self.bias == None && creation.scroll && self.one(4) {
            let (bias, chance) = match self.class_id {
                "demo.class.mage"
                | "demo.class.high-mage"
                | "demo.class.warrior-mage"
                | "demo.class.magic-eater" => (Mage, 20),
                "demo.class.mindcrafter" => (Priestly, 20),
                "demo.class.priest" => (Priestly, 30),
                "demo.class.sniper" | "demo.class.ranger" => (Ranger, 30),
                "demo.class.paladin" => (Priestly, 60),
                _ => (Warrior, 0),
            };
            self.bias = bias;
            warrior_chance = chance;
        }
        if (creation.scroll || creation.good) && self.roll(100) <= warrior_chance {
            self.bias = Warrior;
        }
    }

    fn name_power(&mut self, cost: i32) -> usize {
        if cost <= 0 {
            return 0;
        }
        let (low, high) = match self.object.tval {
            45 => (15_000, 60_000),
            40 => (8_000, 25_000),
            _ if self.weapon() => (50_000, 100_000),
            _ => (30_000, 70_000),
        };
        if cost < low {
            return 1;
        }
        if cost < high {
            return 2;
        }
        if !matches!(self.object.tval, 40 | 45) {
            if self.one(17) {
                self.add("AGGRAVATE");
            }
            if self.weapon() {
                if self.one(5) {
                    self.add("FREE_ACT");
                }
                if self.one(5) {
                    self.add("SEE_INVIS");
                }
            } else if self.one(5) {
                self.add("HOLD_LIFE");
            }
        }
        3
    }
    fn zero(&mut self, n: i32) -> i32 {
        if n <= 1 {
            0
        } else {
            self.rng.bounded(n as u64) as i32
        }
    }
    fn roll(&mut self, n: i32) -> i32 {
        self.zero(n) + 1
    }
    fn one(&mut self, n: i32) -> bool {
        self.zero(n) == 0
    }
    fn has(&self, flag: &str) -> bool {
        self.object.flags.contains(flag)
    }
    fn add(&mut self, flag: &str) {
        self.object.flags.insert(flag.to_owned());
    }
    fn remove(&mut self, flag: &str) {
        self.object.flags.remove(flag);
    }
    fn add_all(&mut self, flags: &[&str]) {
        for flag in flags {
            self.add(flag);
        }
    }
    fn melee(&self) -> bool {
        (20..=23).contains(&self.object.tval)
    }
    fn armor(&self) -> bool {
        (30..=38).contains(&self.object.tval)
    }
    fn body(&self) -> bool {
        (36..=38).contains(&self.object.tval)
    }
    fn weapon(&self) -> bool {
        (16..=23).contains(&self.object.tval)
    }
    fn harp(&self) -> bool {
        self.object.tval == 19 && self.object.sval == 70
    }
    fn bonus(&mut self, maximum: u16, level: i32) -> i32 {
        i32::from(rfb_m_bonus(self.rng, maximum, level as u16))
    }
    fn free_biased_flag(&mut self, flag: &str) -> bool {
        if self.has(flag) {
            return false;
        }
        self.add(flag);
        self.one(2)
    }
}
