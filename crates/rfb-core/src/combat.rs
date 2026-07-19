// SPDX-License-Identifier: MPL-2.0

use rfb_protocol::Position;

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

pub(crate) fn apply_melee_armor_reduction(damage: i32, armor_class: i32) -> i32 {
    let armor_class = armor_class.clamp(0, MELEE_MAXIMUM_ARMOR_CLASS);
    let reduction =
        MELEE_MAXIMUM_DAMAGE_REDUCTION.saturating_mul(armor_class) / MELEE_MAXIMUM_ARMOR_CLASS;
    damage.saturating_mul(100 - reduction) / 100
}
