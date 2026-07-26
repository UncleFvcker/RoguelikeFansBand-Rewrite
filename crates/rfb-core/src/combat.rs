// SPDX-License-Identifier: MPL-2.0

use rfb_protocol::Position;

use crate::{
    effect::{DamageOutcome, DamagePacket, resolve_damage},
    resistance::{DamageType, ResistanceLevel},
};

const ATTACK_SKILL_PER_RATING: i32 = 20;
const ARMOR_CLASS_PER_RATING: i32 = 10;
const MONSTER_MINIMUM_LEVEL: i32 = 4;
const MONSTER_LEVEL_SKILL_MULTIPLIER: i32 = 3;
const MELEE_MAXIMUM_ARMOR_CLASS: i32 = 180;
const MELEE_MAXIMUM_DAMAGE_REDUCTION: i32 = 60;

pub(crate) fn adjacent(left: Position, right: Position) -> bool {
    let dx = (left.x - right.x).abs();
    let dy = (left.y - right.y).abs();
    dx <= 1 && dy <= 1 && (dx != 0 || dy != 0)
}

pub(crate) fn rating_to_combat_value(rating: i32) -> i32 {
    rating.saturating_mul(ATTACK_SKILL_PER_RATING)
}

pub(crate) fn rating_to_armor_class(rating: i32) -> i32 {
    rating.saturating_mul(ARMOR_CLASS_PER_RATING)
}

pub(crate) fn monster_melee_skill(attack: i32, level: u32) -> i32 {
    let level = i32::try_from(level)
        .unwrap_or(i32::MAX)
        .max(MONSTER_MINIMUM_LEVEL);
    rating_to_combat_value(attack)
        .saturating_add(level.saturating_mul(MONSTER_LEVEL_SKILL_MULTIPLIER))
}

/// Physical damage is reduced by armor before resistance scaling; every
/// non-physical type skips the armor step. All melee-family resolution paths
/// share this exact sequence.
pub(crate) fn resolve_armored_damage(
    raw_damage: i32,
    damage_type: DamageType,
    armor_class: i32,
    resistance: ResistanceLevel,
) -> DamageOutcome {
    let prepared = if damage_type == DamageType::Physical {
        apply_melee_armor_reduction(raw_damage, armor_class)
    } else {
        raw_damage
    };
    resolve_damage(
        DamagePacket::after_armor(raw_damage, prepared, damage_type),
        resistance,
    )
}

pub(crate) fn apply_melee_armor_reduction(damage: i32, armor_class: i32) -> i32 {
    let armor_class = armor_class.clamp(0, MELEE_MAXIMUM_ARMOR_CLASS);
    let reduction =
        MELEE_MAXIMUM_DAMAGE_REDUCTION.saturating_mul(armor_class) / MELEE_MAXIMUM_ARMOR_CLASS;
    damage.saturating_mul(100 - reduction) / 100
}
