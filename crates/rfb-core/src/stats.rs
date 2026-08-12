// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

use crate::rng::RfbRng;

pub const MAX_LEVEL: u16 = 100;
pub const PRE_VICTORY_LEVEL_CAP: u16 = 50;
pub const PRE_VICTORY_ATTRIBUTE_CAP: u16 = 238;
pub const VICTORY_ATTRIBUTE_CAP: u16 = 838;
pub const PRE_VICTORY_ATTRIBUTE_INDEX_CAP: u8 = 37;
pub const VICTORY_ATTRIBUTE_INDEX_CAP: u8 = 97;
pub const MAX_EXPERIENCE: u64 = 999_999_999;
const HP_SEQUENCE_SEED_SALT: u64 = 0x5246_425f_4850_7637;
const ATTRIBUTE_POTENTIAL_SEED_SALT: u64 = 0x5246_425f_4150_7631;
const ATTRIBUTE_POTENTIAL_DIE_BASE: u16 = 78;
const ATTRIBUTE_POTENTIAL_DIE_SCALE: u16 = 10;
const ATTRIBUTE_POTENTIAL_DIE_TOTAL: u16 = 24;
const HP_RATING_MINIMUM_PERCENT: u16 = 87;
const HP_RATING_MAXIMUM_PERCENT: u16 = 117;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StatKind {
    MaxHp,
    Attack,
    Defense,
    Speed,
    MeleeSkill,
    MeleeAttacks,
    MeleeDamageBonus,
    RangedSkill,
    ThrowingSkill,
    DoorSkill,
    BashPower,
    SearchSkill,
    DeviceSkill,
    SavingThrowSkill,
    StealthSkill,
    PerceptionSkill,
    DisarmSkill,
    DigSkill,
    ArmorClass,
    ActionDifficulty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AttributeKind {
    Strength,
    Intelligence,
    Wisdom,
    Dexterity,
    Constitution,
    Charisma,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttributeSet {
    pub strength: u16,
    pub intelligence: u16,
    pub wisdom: u16,
    pub dexterity: u16,
    pub constitution: u16,
    pub charisma: u16,
}

impl Default for AttributeSet {
    fn default() -> Self {
        Self {
            strength: 13,
            intelligence: 13,
            wisdom: 13,
            dexterity: 13,
            constitution: 13,
            charisma: 13,
        }
    }
}

impl AttributeSet {
    #[must_use]
    pub const fn value(self, kind: AttributeKind) -> u16 {
        match kind {
            AttributeKind::Strength => self.strength,
            AttributeKind::Intelligence => self.intelligence,
            AttributeKind::Wisdom => self.wisdom,
            AttributeKind::Dexterity => self.dexterity,
            AttributeKind::Constitution => self.constitution,
            AttributeKind::Charisma => self.charisma,
        }
    }

    #[must_use]
    pub const fn index(self, kind: AttributeKind) -> u8 {
        stat_index(self.value(kind))
    }

    #[must_use]
    pub const fn constitution_hp_percent(self) -> u16 {
        constitution_hp_percent(stat_index(self.constitution))
    }
}

/// RFB's 3..18/220 representation uses 38 non-linear buckets. The rewrite
/// extends the same ten-point progression through 18/820 after victory.
#[must_use]
pub const fn stat_index(value: u16) -> u8 {
    let index = if value <= 18 {
        value.saturating_sub(3)
    } else if value <= 237 {
        15 + (value - 18) / 10
    } else {
        37 + (value - 238) / 10
    };
    if index > VICTORY_ATTRIBUTE_INDEX_CAP as u16 {
        VICTORY_ATTRIBUTE_INDEX_CAP
    } else {
        index as u8
    }
}

const ORIGINAL_STRENGTH_CARRY_CAPACITY_DECA_POUNDS: [u16; 38] = [
    10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 31, 32,
    32, 33, 33, 34, 34, 35, 35, 36, 36, 37, 37, 38, 38, 39,
];

const ORIGINAL_STRENGTH_HOLD_POUNDS: [u16; 38] = [
    4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
    30, 31, 32, 33, 34, 35, 37, 40, 44, 48, 50, 50, 50,
];

const ORIGINAL_STRENGTH_DIGGING_BONUS: [u16; 38] = [
    0, 0, 1, 2, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 10, 12, 15, 20, 25, 30, 35, 40, 45, 50, 55, 60,
    65, 70, 75, 80, 85, 90, 95, 100, 100, 100,
];

#[must_use]
pub fn carry_capacity_tenths_pound(strength: u16) -> u32 {
    let index = stat_index(strength).min(PRE_VICTORY_ATTRIBUTE_INDEX_CAP);
    u32::from(ORIGINAL_STRENGTH_CARRY_CAPACITY_DECA_POUNDS[index as usize]) * 50
}

#[must_use]
pub fn strength_hold_pounds(strength: u16) -> u16 {
    let index = stat_index(strength).min(PRE_VICTORY_ATTRIBUTE_INDEX_CAP);
    ORIGINAL_STRENGTH_HOLD_POUNDS[index as usize]
}

#[must_use]
pub fn strength_digging_bonus(strength: u16) -> u16 {
    let index = stat_index(strength).min(PRE_VICTORY_ATTRIBUTE_INDEX_CAP);
    ORIGINAL_STRENGTH_DIGGING_BONUS[index as usize]
}

#[must_use]
pub fn encumbrance_speed_penalty(weight: u32, capacity: u32) -> i32 {
    if weight <= capacity || capacity < 5 {
        return 0;
    }
    let penalty = (weight - capacity) / (capacity / 5);
    i32::try_from(penalty).unwrap_or(i32::MAX)
}

#[must_use]
pub fn modify_attribute_value(value: u16, modifier: i32, cap: u16) -> u16 {
    let mut value = value.clamp(3, cap);
    if modifier > 0 {
        for _ in 0..modifier {
            value = if value < 18 {
                value.saturating_add(1)
            } else {
                value.saturating_add(10)
            }
            .min(cap);
        }
    } else {
        for _ in modifier..0 {
            value = if value >= 28 {
                value - 10
            } else if value > 18 {
                18
            } else {
                value.saturating_sub(1).max(3)
            };
        }
    }
    value
}

const ORIGINAL_CONSTITUTION_HP_PERCENT: [u16; 38] = [
    80, 84, 87, 90, 92, 94, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 109, 111, 113,
    115, 117, 119, 121, 124, 127, 130, 133, 136, 139, 142, 145, 148, 151, 154, 157, 160,
];

const fn constitution_hp_percent(index: u8) -> u16 {
    if index <= PRE_VICTORY_ATTRIBUTE_INDEX_CAP {
        ORIGINAL_CONSTITUTION_HP_PERCENT[index as usize]
    } else {
        160 + (index as u16 - PRE_VICTORY_ATTRIBUTE_INDEX_CAP as u16) * 3
    }
}

const ORIGINAL_EXPERIENCE_THRESHOLDS: [u64; 50] = [
    10, 25, 45, 70, 100, 140, 200, 280, 380, 500, 650, 850, 1_100, 1_400, 1_800, 2_300, 2_900,
    3_600, 4_400, 5_400, 6_800, 8_400, 10_200, 12_500, 17_500, 25_000, 35_000, 50_000, 75_000,
    100_000, 150_000, 200_000, 275_000, 350_000, 450_000, 550_000, 700_000, 850_000, 1_000_000,
    1_250_000, 1_500_000, 1_800_000, 2_100_000, 2_400_000, 2_700_000, 3_000_000, 3_500_000,
    4_000_000, 4_500_000, 5_000_000,
];

#[must_use]
pub fn experience_to_next_level(level: u16) -> Option<u64> {
    if level == 0 || level >= MAX_LEVEL {
        return None;
    }
    if level <= 50 {
        return Some(ORIGINAL_EXPERIENCE_THRESHOLDS[usize::from(level - 1)]);
    }
    Some(5_500_000 + u64::from(level - 51) * 500_000)
}

#[must_use]
pub fn experience_required_for_level(level: u16) -> u64 {
    level
        .checked_sub(1)
        .and_then(experience_to_next_level)
        .unwrap_or(0)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterProgress {
    pub attributes: AttributeSet,
    pub maximum_attributes: AttributeSet,
    pub attribute_potentials: AttributeSet,
    pub experience: u64,
    pub maximum_experience: u64,
    pub life_force: u16,
    pub level: u16,
    pub max_level: u16,
    pub pending_attribute_increases: u16,
    pub hp_progression: Vec<i32>,
    pub skills: BTreeMap<String, SkillProgress>,
    /// Trained values above the active class's birth proficiency, keyed by
    /// canonical base item kind ID.
    pub weapon_proficiencies: BTreeMap<String, u16>,
    pub mining_proficiency: u16,
    pub materials: BTreeMap<String, u32>,
    pub active_mutation_ids: BTreeSet<String>,
    pub locked_mutation_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillProgress {
    pub current: i32,
    pub maximum: i32,
    pub base: i32,
    pub growth_per_ten_levels: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterBuildIdentity {
    pub build_id: String,
    pub race_id: String,
    pub class_id: String,
    pub personality_id: String,
}

impl SkillProgress {
    #[must_use]
    pub fn at_level(base: i32, growth_per_ten_levels: i32, maximum: i32, level: u16) -> Self {
        let growth = growth_per_ten_levels
            .saturating_mul(i32::from(level))
            .saturating_div(10);
        Self {
            current: base.saturating_add(growth).clamp(0, maximum),
            maximum,
            base,
            growth_per_ten_levels,
        }
    }
}

impl CharacterProgress {
    #[must_use]
    pub fn new(seed: u64, base_max_hp: i32) -> Self {
        let mut hp_rng = RfbRng::seeded(seed ^ HP_SEQUENCE_SEED_SALT);
        let hp_progression = Self::roll_hp_progression(base_max_hp, &mut hp_rng);
        let mut potential_rng = RfbRng::seeded(seed ^ ATTRIBUTE_POTENTIAL_SEED_SALT);
        Self {
            attributes: AttributeSet::default(),
            maximum_attributes: AttributeSet::default(),
            attribute_potentials: Self::roll_attribute_potentials(&mut potential_rng),
            experience: 0,
            maximum_experience: 0,
            life_force: 1_000,
            level: 1,
            max_level: 1,
            pending_attribute_increases: 0,
            hp_progression,
            skills: BTreeMap::new(),
            weapon_proficiencies: BTreeMap::new(),
            mining_proficiency: 0,
            materials: BTreeMap::new(),
            active_mutation_ids: BTreeSet::new(),
            locked_mutation_ids: BTreeSet::new(),
        }
    }

    #[must_use]
    pub fn legacy(base_max_hp: i32) -> Self {
        let mut progress = Self::new(0, base_max_hp);
        let mut hp = base_max_hp.max(1);
        progress.hp_progression.clear();
        progress.hp_progression.push(hp);
        for _ in 1..MAX_LEVEL {
            hp = hp.saturating_add(6);
            progress.hp_progression.push(hp);
        }
        progress
    }

    #[must_use]
    pub const fn level_cap(victorious: bool) -> u16 {
        if victorious {
            MAX_LEVEL
        } else {
            PRE_VICTORY_LEVEL_CAP
        }
    }

    #[must_use]
    pub const fn attribute_cap(victorious: bool) -> u16 {
        if victorious {
            VICTORY_ATTRIBUTE_CAP
        } else {
            PRE_VICTORY_ATTRIBUTE_CAP
        }
    }

    #[must_use]
    pub const fn attribute_index_cap(victorious: bool) -> u8 {
        if victorious {
            VICTORY_ATTRIBUTE_INDEX_CAP
        } else {
            PRE_VICTORY_ATTRIBUTE_INDEX_CAP
        }
    }

    #[must_use]
    pub fn personal_attribute_cap(&self, kind: AttributeKind, victorious: bool) -> u16 {
        Self::attribute_cap(victorious).min(self.attribute_potentials.value(kind))
    }

    pub(crate) fn roll_attribute_potentials(rng: &mut RfbRng) -> AttributeSet {
        loop {
            let dice = std::array::from_fn::<_, 6, _>(|_| {
                u16::try_from(rng.bounded(7)).expect("attribute potential roll must fit u16") + 1
            });
            if dice.into_iter().sum::<u16>() != ATTRIBUTE_POTENTIAL_DIE_TOTAL {
                continue;
            }
            let potential = |die: u16| {
                ATTRIBUTE_POTENTIAL_DIE_BASE
                    .saturating_add(die.saturating_mul(ATTRIBUTE_POTENTIAL_DIE_SCALE))
            };
            return AttributeSet {
                strength: potential(dice[0]),
                intelligence: potential(dice[1]),
                wisdom: potential(dice[2]),
                dexterity: potential(dice[3]),
                constitution: potential(dice[4]),
                charisma: potential(dice[5]),
            };
        }
    }

    pub(crate) fn roll_hp_progression(base_max_hp: i32, rng: &mut RfbRng) -> Vec<i32> {
        loop {
            let mut progression = Vec::with_capacity(usize::from(MAX_LEVEL));
            let mut hp = base_max_hp.max(1);
            progression.push(hp);
            for _ in 1..MAX_LEVEL {
                hp = hp.saturating_add(
                    i32::try_from(rng.bounded(10)).expect("HP roll must fit i32") + 1,
                );
                progression.push(hp);
            }
            if Self::hp_progression_rating_is_accepted(&progression) {
                return progression;
            }
        }
    }

    #[must_use]
    pub(crate) fn hp_progression_rating_is_accepted(progression: &[i32]) -> bool {
        let rating = |level: u16| {
            let first = progression.first().copied()?;
            let current = progression
                .get(usize::from(level.checked_sub(1)?))
                .copied()?;
            let gain = u64::try_from(current.checked_sub(first)?).ok()?;
            let rolls = u64::from(level.checked_sub(1)?);
            u16::try_from(
                gain.saturating_mul(200)
                    .saturating_div(rolls.saturating_mul(11)),
            )
            .ok()
        };
        [5, 10, 25]
            .into_iter()
            .all(|level| rating(level).is_some_and(|value| value >= HP_RATING_MINIMUM_PERCENT))
            && rating(MAX_LEVEL).is_some_and(|value| {
                (HP_RATING_MINIMUM_PERCENT..=HP_RATING_MAXIMUM_PERCENT).contains(&value)
            })
    }

    pub(crate) fn clamp_attributes_to_potentials(&mut self) {
        self.maximum_attributes.strength = self
            .maximum_attributes
            .strength
            .min(self.attribute_potentials.strength);
        self.attributes.strength = self
            .attributes
            .strength
            .min(self.maximum_attributes.strength);
        self.maximum_attributes.intelligence = self
            .maximum_attributes
            .intelligence
            .min(self.attribute_potentials.intelligence);
        self.attributes.intelligence = self
            .attributes
            .intelligence
            .min(self.maximum_attributes.intelligence);
        self.maximum_attributes.wisdom = self
            .maximum_attributes
            .wisdom
            .min(self.attribute_potentials.wisdom);
        self.attributes.wisdom = self.attributes.wisdom.min(self.maximum_attributes.wisdom);
        self.maximum_attributes.dexterity = self
            .maximum_attributes
            .dexterity
            .min(self.attribute_potentials.dexterity);
        self.attributes.dexterity = self
            .attributes
            .dexterity
            .min(self.maximum_attributes.dexterity);
        self.maximum_attributes.constitution = self
            .maximum_attributes
            .constitution
            .min(self.attribute_potentials.constitution);
        self.attributes.constitution = self
            .attributes
            .constitution
            .min(self.maximum_attributes.constitution);
        self.maximum_attributes.charisma = self
            .maximum_attributes
            .charisma
            .min(self.attribute_potentials.charisma);
        self.attributes.charisma = self
            .attributes
            .charisma
            .min(self.maximum_attributes.charisma);
    }

    #[must_use]
    pub fn base_max_hp(&self) -> i32 {
        self.hp_progression
            .get(usize::from(self.level.saturating_sub(1)))
            .copied()
            .unwrap_or(1)
    }

    #[must_use]
    pub fn effective_base_max_hp(&self) -> i32 {
        let base = self.base_max_hp();
        let percent = i32::from(self.attributes.constitution_hp_percent());
        base.saturating_mul(percent).saturating_add(50) / 100
    }

    pub fn gain_experience(&mut self, amount: u64, victorious: bool) -> Vec<u16> {
        self.experience = self.experience.saturating_add(amount).min(MAX_EXPERIENCE);
        self.maximum_experience = self.maximum_experience.max(self.experience);
        let cap = Self::level_cap(victorious);
        let mut gained = Vec::new();
        while self.level < cap
            && self.experience >= experience_required_for_level(self.level.saturating_add(1))
        {
            self.level += 1;
            let reached_new_maximum = self.level > self.max_level;
            self.max_level = self.max_level.max(self.level);
            if reached_new_maximum && self.level.is_multiple_of(5) {
                self.pending_attribute_increases =
                    self.pending_attribute_increases.saturating_add(1);
            }
            gained.push(self.level);
        }
        gained
    }

    pub fn lose_experience(&mut self, amount: u64) -> Vec<u16> {
        self.experience = self.experience.saturating_sub(amount);
        let mut lost = Vec::new();
        while self.level > 1 && self.experience < experience_required_for_level(self.level) {
            self.level -= 1;
            lost.push(self.level);
        }
        lost
    }

    pub fn replace_skills(&mut self, skills: BTreeMap<String, SkillProgress>) {
        self.skills = skills;
    }

    #[must_use]
    pub fn skill(&self, id: &str) -> Option<&SkillProgress> {
        self.skills.get(id)
    }

    /// Spend one earned attribute increase using the original RFB bucket
    /// progression: values below 18 advance by one, while 18/xx buckets
    /// advance by ten. Equipment modifiers never consume an increase.
    pub fn increase_attribute(&mut self, kind: AttributeKind, victorious: bool) -> Option<u16> {
        if self.pending_attribute_increases == 0 {
            return None;
        }
        let cap = self.personal_attribute_cap(kind, victorious);
        let value = self.maximum_attributes.value(kind);
        if value >= cap {
            return None;
        }
        let next = modify_attribute_value(value, 1, cap);
        if next == value {
            return None;
        }
        match kind {
            AttributeKind::Strength => {
                self.attributes.strength = next;
                self.maximum_attributes.strength = next;
            }
            AttributeKind::Intelligence => {
                self.attributes.intelligence = next;
                self.maximum_attributes.intelligence = next;
            }
            AttributeKind::Wisdom => {
                self.attributes.wisdom = next;
                self.maximum_attributes.wisdom = next;
            }
            AttributeKind::Dexterity => {
                self.attributes.dexterity = next;
                self.maximum_attributes.dexterity = next;
            }
            AttributeKind::Constitution => {
                self.attributes.constitution = next;
                self.maximum_attributes.constitution = next;
            }
            AttributeKind::Charisma => {
                self.attributes.charisma = next;
                self.maximum_attributes.charisma = next;
            }
        }
        self.pending_attribute_increases -= 1;
        Some(next)
    }

    pub fn drain_attribute(&mut self, kind: AttributeKind, rng: &mut RfbRng) -> bool {
        self.drain_attribute_by(kind, 10, rng)
    }

    pub fn drain_attribute_by(
        &mut self,
        kind: AttributeKind,
        amount: u8,
        rng: &mut RfbRng,
    ) -> bool {
        let current = self.attributes.value(kind);
        let next = drain_attribute_value(current, amount, rng);
        self.set_current_attribute(kind, next)
    }

    pub fn permanently_drain_attribute(
        &mut self,
        kind: AttributeKind,
        amount: u8,
        rng: &mut RfbRng,
    ) -> bool {
        let current = self.attributes.value(kind);
        let maximum = self.maximum_attributes.value(kind);
        let next_current = drain_attribute_value(current, amount, rng);
        let mut next_maximum = drain_attribute_value(maximum, amount, rng);
        if current == maximum || next_maximum < next_current {
            next_maximum = next_current;
        }
        if next_current == current && next_maximum == maximum {
            return false;
        }
        match kind {
            AttributeKind::Strength => {
                self.attributes.strength = next_current;
                self.maximum_attributes.strength = next_maximum;
            }
            AttributeKind::Intelligence => {
                self.attributes.intelligence = next_current;
                self.maximum_attributes.intelligence = next_maximum;
            }
            AttributeKind::Wisdom => {
                self.attributes.wisdom = next_current;
                self.maximum_attributes.wisdom = next_maximum;
            }
            AttributeKind::Dexterity => {
                self.attributes.dexterity = next_current;
                self.maximum_attributes.dexterity = next_maximum;
            }
            AttributeKind::Constitution => {
                self.attributes.constitution = next_current;
                self.maximum_attributes.constitution = next_maximum;
            }
            AttributeKind::Charisma => {
                self.attributes.charisma = next_current;
                self.maximum_attributes.charisma = next_maximum;
            }
        }
        true
    }

    pub fn restore_attribute(&mut self, kind: AttributeKind) -> bool {
        self.set_current_attribute(kind, self.maximum_attributes.value(kind))
    }

    pub fn increase_attribute_permanently(
        &mut self,
        kind: AttributeKind,
        victorious: bool,
        below_eighteen_threshold: u64,
        rng: &mut RfbRng,
    ) -> bool {
        let restored = self.restore_attribute(kind);
        let cap = self.personal_attribute_cap(kind, victorious);
        let value = self.maximum_attributes.value(kind);
        if value >= cap {
            return restored;
        }

        let next = if value < 18 {
            let threshold = if value == 17 {
                58
            } else {
                below_eighteen_threshold
            };
            let gain = if rng.bounded(100) < threshold { 1 } else { 2 };
            value + gain
        } else if value < cap - 2 {
            let delta = u32::from(cap - value) * 10;
            let percentage = u32::try_from(rng.bounded(151) + 200)
                .expect("attribute growth percentage must fit u32");
            let gain = ((delta * percentage / 1_000 + 5) / 10).max(2);
            u16::try_from(u32::from(value) + gain)
                .expect("validated attribute growth must fit u16")
                .min(cap - 1)
        } else {
            value + 1
        };

        match kind {
            AttributeKind::Strength => {
                self.attributes.strength = next;
                self.maximum_attributes.strength = next;
            }
            AttributeKind::Intelligence => {
                self.attributes.intelligence = next;
                self.maximum_attributes.intelligence = next;
            }
            AttributeKind::Wisdom => {
                self.attributes.wisdom = next;
                self.maximum_attributes.wisdom = next;
            }
            AttributeKind::Dexterity => {
                self.attributes.dexterity = next;
                self.maximum_attributes.dexterity = next;
            }
            AttributeKind::Constitution => {
                self.attributes.constitution = next;
                self.maximum_attributes.constitution = next;
            }
            AttributeKind::Charisma => {
                self.attributes.charisma = next;
                self.maximum_attributes.charisma = next;
            }
        }
        true
    }

    fn set_current_attribute(&mut self, kind: AttributeKind, next: u16) -> bool {
        let current = self.attributes.value(kind);
        if current == next || next > self.maximum_attributes.value(kind) {
            return false;
        }
        match kind {
            AttributeKind::Strength => self.attributes.strength = next,
            AttributeKind::Intelligence => self.attributes.intelligence = next,
            AttributeKind::Wisdom => self.attributes.wisdom = next,
            AttributeKind::Dexterity => self.attributes.dexterity = next,
            AttributeKind::Constitution => self.attributes.constitution = next,
            AttributeKind::Charisma => self.attributes.charisma = next,
        }
        true
    }

    pub fn validate(&self, victorious: bool) -> bool {
        self.level >= 1
            && self.level <= Self::level_cap(victorious)
            && self.max_level >= self.level
            && self.max_level <= MAX_LEVEL
            && self.experience <= MAX_EXPERIENCE
            && self.maximum_experience >= self.experience
            && self.maximum_experience <= MAX_EXPERIENCE
            && self.life_force <= 1_000
            && self.hp_progression.len() == usize::from(MAX_LEVEL)
            && self.hp_progression.iter().all(|hp| *hp > 0)
            && Self::hp_progression_rating_is_accepted(&self.hp_progression)
            && [
                self.attributes.strength,
                self.attributes.intelligence,
                self.attributes.wisdom,
                self.attributes.dexterity,
                self.attributes.constitution,
                self.attributes.charisma,
            ]
            .into_iter()
            .zip([
                self.maximum_attributes.strength,
                self.maximum_attributes.intelligence,
                self.maximum_attributes.wisdom,
                self.maximum_attributes.dexterity,
                self.maximum_attributes.constitution,
                self.maximum_attributes.charisma,
            ])
            .zip([
                self.attribute_potentials.strength,
                self.attribute_potentials.intelligence,
                self.attribute_potentials.wisdom,
                self.attribute_potentials.dexterity,
                self.attribute_potentials.constitution,
                self.attribute_potentials.charisma,
            ])
            .all(|((value, maximum), potential)| {
                (88..=148).contains(&potential)
                    && (potential - ATTRIBUTE_POTENTIAL_DIE_BASE)
                        .is_multiple_of(ATTRIBUTE_POTENTIAL_DIE_SCALE)
                    && maximum <= potential
                    && (3..=Self::attribute_cap(victorious)).contains(&value)
                    && (3..=Self::attribute_cap(victorious)).contains(&maximum)
                    && value <= maximum
            })
            && [
                self.attribute_potentials.strength,
                self.attribute_potentials.intelligence,
                self.attribute_potentials.wisdom,
                self.attribute_potentials.dexterity,
                self.attribute_potentials.constitution,
                self.attribute_potentials.charisma,
            ]
            .into_iter()
            .sum::<u16>()
                == 6 * ATTRIBUTE_POTENTIAL_DIE_BASE
                    + ATTRIBUTE_POTENTIAL_DIE_TOTAL * ATTRIBUTE_POTENTIAL_DIE_SCALE
            && self.pending_attribute_increases <= self.max_level / 5
            && self.skills.iter().all(|(id, skill)| {
                !id.is_empty()
                    && skill.maximum > 0
                    && skill.current >= 0
                    && skill.current <= skill.maximum
                    && (-1_000_000..=1_000_000).contains(&skill.base)
                    && (-1_000_000..=1_000_000).contains(&skill.growth_per_ten_levels)
            })
            && self
                .weapon_proficiencies
                .iter()
                .all(|(id, current)| !id.is_empty() && *current <= 8_000)
            && (self.level == Self::level_cap(victorious)
                || self.experience < experience_required_for_level(self.level + 1))
    }
}

fn drain_attribute_value(value: u16, amount: u8, rng: &mut RfbRng) -> u16 {
    if value <= 3 {
        return value;
    }
    if value <= 18 {
        return value
            .saturating_sub(
                1 + u16::from(amount > 20) + u16::from(amount > 50) + u16::from(amount > 90),
            )
            .max(3);
    }
    let loss_base = (((value - 18) / 2).div_ceil(2) + 1).max(1);
    let roll = rng.bounded(u64::from(loss_base)) + 1;
    let loss = ((roll + u64::from(loss_base)) * u64::from(amount) / 100).max(u64::from(amount / 2));
    let reduced = u16::try_from(u64::from(value).saturating_sub(loss)).unwrap_or(3);
    if reduced < 18 {
        if amount <= 20 { 18 } else { 17 }
    } else {
        reduced
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StatLayer {
    Base,
    Species,
    Class,
    Personality,
    Equipment,
    Status,
    Stance,
    Environment,
}

impl StatLayer {
    const fn priority(self) -> i16 {
        match self {
            Self::Base => 0,
            Self::Species => 100,
            Self::Class => 200,
            Self::Personality => 300,
            Self::Equipment => 400,
            Self::Status => 500,
            Self::Stance => 600,
            Self::Environment => 700,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatContribution {
    pub source_id: String,
    pub origin_id: Option<String>,
    pub layer: StatLayer,
    pub priority: i16,
    pub amount: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedStat {
    pub kind: StatKind,
    pub value: i32,
    pub contributions: Vec<StatContribution>,
}

impl DerivedStat {
    #[must_use]
    pub fn with_modifier(
        &self,
        layer: StatLayer,
        source_id: impl Into<String>,
        amount: i32,
        bounds: StatBounds,
    ) -> Self {
        let mut contributions = self.contributions.clone();
        contributions.push(StatContribution {
            source_id: source_id.into(),
            origin_id: None,
            layer,
            priority: layer.priority(),
            amount,
        });
        contributions.sort_by(|left, right| {
            (
                left.priority,
                left.layer,
                left.source_id.as_str(),
                left.origin_id.as_deref(),
            )
                .cmp(&(
                    right.priority,
                    right.layer,
                    right.source_id.as_str(),
                    right.origin_id.as_deref(),
                ))
        });
        Self {
            kind: self.kind,
            value: self
                .value
                .saturating_add(amount)
                .clamp(bounds.minimum, bounds.maximum),
            contributions,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatBounds {
    minimum: i32,
    maximum: i32,
}

impl StatBounds {
    pub const UNBOUNDED: Self = Self {
        minimum: i32::MIN,
        maximum: i32::MAX,
    };
    pub const NON_NEGATIVE: Self = Self {
        minimum: 0,
        maximum: i32::MAX,
    };
    pub const ACTOR_SPEED: Self = Self {
        minimum: 0,
        maximum: 199,
    };
}

#[derive(Debug, Clone, Default)]
pub struct DerivedStatsPipeline {
    entries: Vec<(StatKind, StatContribution)>,
}

impl DerivedStatsPipeline {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add(
        &mut self,
        kind: StatKind,
        layer: StatLayer,
        source_id: impl Into<String>,
        amount: i32,
    ) {
        self.add_detailed(kind, layer, layer.priority(), source_id, None, amount);
    }

    pub fn add_with_priority(
        &mut self,
        kind: StatKind,
        layer: StatLayer,
        priority: i16,
        source_id: impl Into<String>,
        amount: i32,
    ) {
        self.add_detailed(kind, layer, priority, source_id, None, amount);
    }

    pub fn add_with_origin(
        &mut self,
        kind: StatKind,
        layer: StatLayer,
        source_id: impl Into<String>,
        origin_id: Option<String>,
        amount: i32,
    ) {
        self.add_detailed(kind, layer, layer.priority(), source_id, origin_id, amount);
    }

    fn add_detailed(
        &mut self,
        kind: StatKind,
        layer: StatLayer,
        priority: i16,
        source_id: impl Into<String>,
        origin_id: Option<String>,
        amount: i32,
    ) {
        self.entries.push((
            kind,
            StatContribution {
                source_id: source_id.into(),
                origin_id,
                layer,
                priority,
                amount,
            },
        ));
    }

    #[must_use]
    pub fn resolve(&self, kind: StatKind, bounds: StatBounds) -> DerivedStat {
        let mut contributions = self
            .entries
            .iter()
            .filter(|(entry_kind, _)| *entry_kind == kind)
            .map(|(_, contribution)| contribution.clone())
            .collect::<Vec<_>>();
        contributions.sort_by(|left, right| {
            (
                left.priority,
                left.layer,
                left.source_id.as_str(),
                left.origin_id.as_deref(),
            )
                .cmp(&(
                    right.priority,
                    right.layer,
                    right.source_id.as_str(),
                    right.origin_id.as_deref(),
                ))
        });
        let value = contributions
            .iter()
            .fold(0_i32, |total, contribution| {
                total.saturating_add(contribution.amount)
            })
            .clamp(bounds.minimum, bounds.maximum);
        DerivedStat {
            kind,
            value,
            contributions,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_orders_sources_by_layer_priority_and_id() {
        let mut pipeline = DerivedStatsPipeline::new();
        pipeline.add(
            StatKind::Attack,
            StatLayer::Equipment,
            "demo.item.zeta.1",
            2,
        );
        pipeline.add(StatKind::Attack, StatLayer::Base, "demo.actor.explorer", 3);
        pipeline.add(
            StatKind::Attack,
            StatLayer::Equipment,
            "demo.item.alpha.1",
            -1,
        );

        let result = pipeline.resolve(StatKind::Attack, StatBounds::NON_NEGATIVE);

        assert_eq!(result.value, 4);
        assert_eq!(
            result
                .contributions
                .iter()
                .map(|contribution| contribution.source_id.as_str())
                .collect::<Vec<_>>(),
            [
                "demo.actor.explorer",
                "demo.item.alpha.1",
                "demo.item.zeta.1"
            ]
        );
    }

    #[test]
    fn pipeline_clamps_only_the_final_saturating_total() {
        let mut pipeline = DerivedStatsPipeline::new();
        pipeline.add(StatKind::Speed, StatLayer::Base, "demo.actor.explorer", 190);
        pipeline.add(StatKind::Speed, StatLayer::Status, "rfb.status.haste", 20);

        let result = pipeline.resolve(StatKind::Speed, StatBounds::ACTOR_SPEED);

        assert_eq!(result.value, 199);
        assert_eq!(result.contributions.len(), 2);
    }

    #[test]
    fn rfb_attribute_buckets_extend_from_18_220_to_18_820_after_victory() {
        assert_eq!(stat_index(3), 0);
        assert_eq!(stat_index(18), 15);
        assert_eq!(stat_index(PRE_VICTORY_ATTRIBUTE_CAP), 37);
        assert_eq!(stat_index(VICTORY_ATTRIBUTE_CAP), 97);
        assert_eq!(stat_index(u16::MAX), 97);

        assert_eq!(
            modify_attribute_value(PRE_VICTORY_ATTRIBUTE_CAP, 60, PRE_VICTORY_ATTRIBUTE_CAP),
            PRE_VICTORY_ATTRIBUTE_CAP
        );
        assert_eq!(
            modify_attribute_value(PRE_VICTORY_ATTRIBUTE_CAP, 60, VICTORY_ATTRIBUTE_CAP),
            VICTORY_ATTRIBUTE_CAP
        );
        assert_eq!(
            modify_attribute_value(VICTORY_ATTRIBUTE_CAP, -60, VICTORY_ATTRIBUTE_CAP),
            PRE_VICTORY_ATTRIBUTE_CAP
        );
    }

    #[test]
    fn strength_digging_uses_the_original_38_bucket_table() {
        assert_eq!(strength_digging_bonus(3), 0);
        assert_eq!(strength_digging_bonus(5), 1);
        assert_eq!(strength_digging_bonus(18), 9);
        assert_eq!(strength_digging_bonus(28), 10);
        assert_eq!(strength_digging_bonus(PRE_VICTORY_ATTRIBUTE_CAP), 100);
        assert_eq!(strength_digging_bonus(VICTORY_ATTRIBUTE_CAP), 100);
    }

    #[test]
    fn carrying_capacity_uses_the_original_strength_table_and_caps_at_195_pounds() {
        assert_eq!(carry_capacity_tenths_pound(3), 500);
        assert_eq!(carry_capacity_tenths_pound(13), 1_000);
        assert_eq!(carry_capacity_tenths_pound(17), 1_200);
        assert_eq!(carry_capacity_tenths_pound(18), 1_250);
        assert_eq!(
            carry_capacity_tenths_pound(PRE_VICTORY_ATTRIBUTE_CAP),
            1_950
        );
        assert_eq!(carry_capacity_tenths_pound(VICTORY_ATTRIBUTE_CAP), 1_950);

        assert_eq!(strength_hold_pounds(3), 4);
        assert_eq!(strength_hold_pounds(15), 16);
        assert_eq!(strength_hold_pounds(18), 19);
        assert_eq!(strength_hold_pounds(118), 30);
        assert_eq!(strength_hold_pounds(PRE_VICTORY_ATTRIBUTE_CAP), 50);
        assert_eq!(strength_hold_pounds(VICTORY_ATTRIBUTE_CAP), 50);
    }

    #[test]
    fn encumbrance_penalty_starts_at_twenty_percent_over_capacity() {
        assert_eq!(encumbrance_speed_penalty(1_000, 1_000), 0);
        assert_eq!(encumbrance_speed_penalty(1_199, 1_000), 0);
        assert_eq!(encumbrance_speed_penalty(1_200, 1_000), 1);
        assert_eq!(encumbrance_speed_penalty(1_400, 1_000), 2);
    }

    #[test]
    fn experience_thresholds_preserve_rfb_then_extend_to_level_100() {
        assert_eq!(experience_required_for_level(1), 0);
        assert_eq!(experience_required_for_level(2), 10);
        assert_eq!(experience_required_for_level(50), 4_500_000);
        assert_eq!(experience_required_for_level(51), 5_000_000);
        assert_eq!(experience_required_for_level(52), 5_500_000);
        assert_eq!(experience_required_for_level(100), 29_500_000);
    }

    #[test]
    fn victory_unlocks_banked_experience_through_level_100() {
        let mut progress = CharacterProgress::new(7, 10);
        let capped = progress.gain_experience(29_500_000, false);
        assert_eq!(capped.last(), Some(&50));
        assert_eq!(progress.level, 50);
        assert_eq!(progress.pending_attribute_increases, 10);

        let unlocked = progress.gain_experience(0, true);
        assert_eq!(unlocked.first(), Some(&51));
        assert_eq!(unlocked.last(), Some(&100));
        assert_eq!(progress.level, 100);
        assert_eq!(progress.pending_attribute_increases, 20);
        assert!(progress.validate(true));
    }

    #[test]
    fn regaining_drained_levels_does_not_repeat_attribute_rewards() {
        let mut progress = CharacterProgress::new(7, 10);
        progress.gain_experience(experience_required_for_level(5), false);
        let pending = progress.pending_attribute_increases;

        progress.lose_experience(progress.experience);
        assert_eq!(progress.level, 1);
        assert_eq!(progress.max_level, 5);
        assert_eq!(
            progress.maximum_experience,
            experience_required_for_level(5)
        );

        progress.gain_experience(progress.maximum_experience, false);
        assert_eq!(progress.level, 5);
        assert_eq!(progress.pending_attribute_increases, pending);
    }

    #[test]
    fn hp_progression_is_seeded_without_using_the_simulation_rng() {
        let first = CharacterProgress::new(17, 10);
        let repeated = CharacterProgress::new(17, 10);
        let other = CharacterProgress::new(18, 10);

        assert_eq!(first.hp_progression.len(), usize::from(MAX_LEVEL));
        assert_eq!(first.hp_progression, repeated.hp_progression);
        assert_ne!(first.hp_progression, other.hp_progression);
        assert!(
            first
                .hp_progression
                .windows(2)
                .all(|window| (1..=10).contains(&(window[1] - window[0])))
        );
        assert!(CharacterProgress::hp_progression_rating_is_accepted(
            &first.hp_progression
        ));
    }

    #[test]
    fn birth_attribute_potentials_are_seeded_balanced_and_source_encoded() {
        let first = CharacterProgress::new(17, 10);
        let repeated = CharacterProgress::new(17, 10);
        let potentials = first.attribute_potentials;

        assert_eq!(potentials, repeated.attribute_potentials);
        let values = [
            potentials.strength,
            potentials.intelligence,
            potentials.wisdom,
            potentials.dexterity,
            potentials.constitution,
            potentials.charisma,
        ];
        assert_eq!(values.into_iter().sum::<u16>(), 708);
        assert!(
            values
                .into_iter()
                .all(|value| (88..=148).contains(&value) && (value - 78).is_multiple_of(10))
        );
    }

    #[test]
    fn hp_rating_filter_uses_early_lower_bounds_and_final_upper_bound() {
        let progression = |gain: i32| {
            (0..MAX_LEVEL)
                .map(|level| 10 + i32::from(level) * gain)
                .collect::<Vec<_>>()
        };
        assert!(!CharacterProgress::hp_progression_rating_is_accepted(
            &progression(4)
        ));
        assert!(CharacterProgress::hp_progression_rating_is_accepted(
            &progression(5)
        ));
        assert!(!CharacterProgress::hp_progression_rating_is_accepted(
            &progression(7)
        ));
    }

    #[test]
    fn skill_growth_uses_deterministic_per_ten_level_proration() {
        let level_one = SkillProgress::at_level(70, 30, 1000, 1);
        let level_ten = SkillProgress::at_level(70, 30, 1000, 10);
        let capped = SkillProgress::at_level(900, 30, 1000, 100);

        assert_eq!(level_one.current, 73);
        assert_eq!(level_ten.current, 100);
        assert_eq!(capped.current, 1000);
    }

    #[test]
    fn attribute_increases_spend_points_and_respect_stage_caps() {
        let mut progress = CharacterProgress::new(1, 10);
        progress.pending_attribute_increases = 3;
        assert_eq!(
            progress.increase_attribute(AttributeKind::Strength, false),
            Some(14)
        );
        progress.attributes.strength = 18;
        progress.maximum_attributes.strength = 18;
        assert_eq!(
            progress.increase_attribute(AttributeKind::Strength, false),
            Some(28)
        );
        progress.attributes.strength = PRE_VICTORY_ATTRIBUTE_CAP;
        progress.maximum_attributes.strength = PRE_VICTORY_ATTRIBUTE_CAP;
        assert_eq!(
            progress.increase_attribute(AttributeKind::Strength, false),
            None
        );
        assert_eq!(progress.pending_attribute_increases, 1);
        assert_eq!(
            progress.increase_attribute(AttributeKind::Constitution, false),
            Some(14)
        );
        assert_eq!(progress.pending_attribute_increases, 0);
    }

    #[test]
    fn attribute_drain_preserves_floor_and_only_rolls_above_18() {
        for (current, expected, draws) in [(3, 3, 0), (13, 12, 0), (38, 33, 1)] {
            let mut progress = CharacterProgress::new(1, 10);
            progress.attributes.strength = current;
            progress.maximum_attributes.strength = 100;
            let mut rng = RfbRng::seeded(99);
            assert_eq!(
                progress.drain_attribute(AttributeKind::Strength, &mut rng),
                current != 3
            );
            assert_eq!(progress.attributes.strength, expected);
            assert_eq!(rng.draw_counter, draws);
        }
    }

    #[test]
    fn permanent_attribute_increase_uses_potion_bands_without_spending_level_points() {
        for (current, maximum, seed, expected, changed, draws) in [
            (13, 13, 0, 14, true, 1),
            (17, 17, 2, 19, true, 1),
            (146, 146, 0, 147, true, 0),
            (147, 148, 0, 148, true, 0),
            (148, 148, 0, 148, false, 0),
        ] {
            let mut progress = CharacterProgress::new(1, 10);
            progress.attribute_potentials.strength = 148;
            progress.attributes.strength = current;
            progress.maximum_attributes.strength = maximum;
            progress.pending_attribute_increases = 1;
            let mut rng = RfbRng::seeded(seed);

            assert_eq!(
                progress.increase_attribute_permanently(
                    AttributeKind::Strength,
                    false,
                    75,
                    &mut rng,
                ),
                changed
            );
            assert_eq!(progress.attributes.strength, expected);
            assert_eq!(progress.maximum_attributes.strength, expected);
            assert_eq!(progress.pending_attribute_increases, 1);
            assert_eq!(rng.draw_counter, draws);
        }
    }
}
