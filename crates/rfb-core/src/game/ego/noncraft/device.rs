// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::game::loot::ItemGenerationMode;
use rfb_content::{RfbDeviceEffectDefinition, RfbDeviceEffectFlagDefinition as Flag};

// RFB master a0d92b6378: devices.c::_rand_normal, device_init[_fixed],
// _effect_rarity, _device_adjust_activation_difficulty; ego.c::obj_create_device.
fn normal(rng: &mut RfbRng, mean: i32, percent: i32) -> i32 {
    let scaled = mean * 10;
    let value = crate::rng::rfb_randnor(rng, scaled, (scaled * percent / 100) as u16);
    let fraction = rng.bounded(10) as i32;
    value / 10 + i32::from(fraction < value % 10)
}

fn normal_bounds(mean: u32, percent: u32) -> (u32, u32) {
    let scaled = mean * 10;
    let offset = (scaled * percent / 100) * 255 / 64;
    ((scaled - offset) / 10, (scaled + offset).div_ceil(10))
}

pub(crate) fn valid_runtime(
    profile: &ItemDeviceActivationDefinition,
    row: &RfbDeviceEffectDefinition,
    activation: &ItemActivationDto,
    charges: ItemChargesDto,
    rod: bool,
    ego: Option<(u32, u16)>,
) -> bool {
    if !(profile.min_depth..=profile.max_depth).contains(&activation.power) {
        return false;
    }
    let base = profile.device_check_difficulty;
    let delta = 10 * (i32::from(activation.power) - base);
    let bonus = i32::from(row.difficulty_base) * delta / 100;
    let extra = (i32::from(row.difficulty_extra) * delta / 100 - 1).max(0);
    let low = base + bonus / 10;
    let high = (base + (bonus + extra + 9) / 10).min(i32::from(activation.power));
    let ease = |difficulty| {
        ego.filter(|(index, _)| *index == 253)
            .map_or(difficulty, |(_, pval)| device_difficulty(difficulty, pval))
    };
    let (min_cost, max_cost) = normal_bounds(profile.charges.cost, 5);
    if !(min_cost.max(1)..=max_cost.min(1000)).contains(&activation.cost)
        || !(ease(low)..=ease(high)).contains(&activation.device_check_difficulty)
    {
        return false;
    }
    let (minimum, maximum) = normal_bounds(
        3 * u32::from(activation.power) / if rod { 2 } else { 1 },
        15,
    );
    let floor = profile.charges.minimum / profile.charges.cost * activation.cost;
    let bound = |value: u32| {
        let capacity = value.max(floor).min(profile.charges.maximum);
        ego.filter(|(index, _)| *index == 251)
            .map_or(capacity, |(_, pval)| device_capacity(capacity, pval))
    };
    (bound(minimum)..=bound(maximum)).contains(&charges.maximum)
}

fn difficulty(
    rng: &mut RfbRng,
    profile: &ItemDeviceActivationDefinition,
    row: &RfbDeviceEffectDefinition,
    power: u16,
) -> i32 {
    let base = profile.device_check_difficulty;
    if i32::from(power) <= base {
        return base;
    }
    let delta = 10 * (i32::from(power) - base);
    let extra = i32::from(row.difficulty_extra) * delta / 100;
    let bonus = i32::from(row.difficulty_base) * delta / 100
        + if extra > 0 {
            rng.bounded(extra as u64) as i32
        } else {
            0
        };
    (base + bonus / 10 + i32::from((rng.bounded(10) as i32) < bonus % 10)).min(i32::from(power))
}

fn weight(
    profile: &ItemDeviceActivationDefinition,
    row: &RfbDeviceEffectDefinition,
    power: u16,
    mode: ItemGenerationMode,
) -> u64 {
    if row.rarity == 0
        || power < profile.min_depth
        || power > profile.max_depth
        || (matches!(
            mode,
            ItemGenerationMode::Good
                | ItemGenerationMode::Great
                | ItemGenerationMode::TailoredGreat
                | ItemGenerationMode::Artifact { .. }
        ) && !row.flags.contains(&Flag::DropGood))
        || (matches!(
            mode,
            ItemGenerationMode::GreatOnly
                | ItemGenerationMode::Great
                | ItemGenerationMode::TailoredGreat
                | ItemGenerationMode::Artifact { .. }
        ) && !row.flags.contains(&Flag::DropGreat))
    {
        return 0;
    }
    let mut rarity = u64::from(row.rarity);
    let mut delta = power - profile.min_depth;
    if delta > 0 {
        let spread = profile.max_depth - profile.min_depth;
        let step = if row.flags.contains(&Flag::Common) {
            spread / 2
        } else {
            spread * 2 / 7
        };
        // Content validation rejects a zero step for source tables.
        while delta >= step {
            rarity *= 2;
            delta -= step;
            if rarity > 64 {
                return 0;
            }
        }
        rarity += u64::from(delta) * rarity / u64::from(step);
    }
    64 / rarity
}

fn state(
    profile: &ItemDeviceActivationDefinition,
    power: u16,
    difficulty: i32,
    cost: u32,
    current: u32,
    maximum: u32,
) -> (ItemActivationDto, ItemChargesDto) {
    (
        ItemActivationDto {
            profile_id: profile.id.clone(),
            name_key: profile.name_key.clone(),
            power,
            cost,
            device_check_difficulty: difficulty,
            target_spec: target_spec_dto(&profile.target),
        },
        ItemChargesDto { current, maximum },
    )
}

pub(in crate::game) fn fixed(
    rng: &mut RfbRng,
    generation: &ItemDeviceGenerationDefinition,
    rod: bool,
) -> (ItemActivationDto, ItemChargesDto) {
    let source = generation.rfb_device.as_ref().unwrap();
    let profile = generation
        .activations
        .iter()
        .find(|p| p.id == source.fixed_activation_id)
        .unwrap();
    let row = source
        .effects
        .iter()
        .find(|r| r.activation_id == profile.id)
        .unwrap();
    let power = profile.min_depth.max(7);
    let difficulty = difficulty(rng, profile, row, power);
    let maximum = (3 * u32::from(power) / if rod { 2 } else { 1 })
        .clamp(profile.charges.minimum, profile.charges.maximum);
    state(
        profile,
        power,
        difficulty,
        profile.charges.cost,
        maximum,
        maximum,
    )
}

fn attempt(
    rng: &mut RfbRng,
    generation: &ItemDeviceGenerationDefinition,
    level: u16,
    mode: ItemGenerationMode,
    rod: bool,
) -> Option<(ItemActivationDto, ItemChargesDto)> {
    let mut level = i32::from(level);
    if rng.bounded(8) == 0 {
        let boost = level.max(20);
        level += boost / 4 + rng.bounded((boost / 2 - boost / 4 + 1) as u64) as i32;
    }
    let power = normal(rng, level.min(100) * 95 / 100, 10).clamp(1, 100) as u16;
    let source = generation.rfb_device.as_ref().unwrap();
    let candidates = source
        .effects
        .iter()
        .map(|row| {
            let profile = generation
                .activations
                .iter()
                .find(|p| p.id == row.activation_id)
                .unwrap();
            (profile, row, weight(profile, row, power, mode))
        })
        .collect::<Vec<_>>();
    let total = candidates.iter().map(|(_, _, weight)| weight).sum();
    if total == 0 {
        return None;
    }
    let mut roll = rng.bounded(total);
    let (profile, row, _) = candidates
        .into_iter()
        .find(|(_, _, weight)| {
            if roll < *weight {
                true
            } else {
                roll -= weight;
                false
            }
        })
        .unwrap();
    let difficulty = difficulty(rng, profile, row, power);
    // This first source table contains power-independent detection/identification
    // effects only; their SPELL_COST_EXTRA is zero (validated at import).
    let cost = normal(rng, profile.charges.cost as i32, 5).clamp(1, 1000) as u32;
    let minimum = profile.charges.minimum / profile.charges.cost * cost;
    let maximum = (normal(rng, 3 * i32::from(power) / if rod { 2 } else { 1 }, 15) as i64)
        .max(i64::from(minimum))
        .min(i64::from(profile.charges.maximum)) as u32;
    let current = normal(rng, (maximum / 2) as i32, 25)
        .max(cost as i32)
        .min(maximum as i32) as u32;
    Some(state(profile, power, difficulty, cost, current, maximum))
}

pub(in crate::game) fn natural(
    rng: &mut RfbRng,
    generation: &ItemDeviceGenerationDefinition,
    level: u16,
    mode: ItemGenerationMode,
    rod: bool,
) -> Option<(ItemActivationDto, ItemChargesDto, u16)> {
    // The source explicitly retries once without quality flags at level + 2.
    attempt(rng, generation, level, mode, rod)
        .map(|(activation, charges)| (activation, charges, level))
        .or_else(|| {
            let level = level.saturating_add(2);
            attempt(rng, generation, level, ItemGenerationMode::Ordinary, rod)
                .map(|(activation, charges)| (activation, charges, level))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Game;

    #[test]
    fn source_rarity_depth_quality_and_single_retry_keep_their_boundaries() {
        let game = Game::new(410);
        let generation = game
            .content
            .item("demo.item.detect-objects-staff")
            .unwrap()
            .device_generation
            .as_ref()
            .unwrap();
        let source = generation.rfb_device.as_ref().unwrap();
        let profile = &generation.activations[0];
        let row = source
            .effects
            .iter()
            .find(|row| row.activation_id == profile.id)
            .unwrap();
        // DETECT_GOLD: level 5, max 30, rarity 1, non-COMMON spread step 7.
        for (power, expected) in [(4, 0), (5, 64), (12, 32), (19, 16), (26, 8), (31, 0)] {
            assert_eq!(
                weight(profile, row, power, ItemGenerationMode::Ordinary),
                expected
            );
        }
        assert_eq!(weight(profile, row, 5, ItemGenerationMode::Good), 0);
        let mut quality_row = row.clone();
        quality_row.flags.insert(Flag::DropGood);
        assert_eq!(
            weight(profile, &quality_row, 5, ItemGenerationMode::Good),
            64
        );
        assert_eq!(
            weight(profile, &quality_row, 5, ItemGenerationMode::Great),
            0
        );
        quality_row.flags.insert(Flag::DropGreat);
        assert_eq!(
            weight(profile, &quality_row, 5, ItemGenerationMode::Great),
            64
        );
        quality_row.rarity = 0;
        assert_eq!(
            weight(profile, &quality_row, 5, ItemGenerationMode::Ordinary),
            0
        );

        // This utility table has no DROP_GOOD entries: retry ordinary at +2,
        // with the RNG left by the failed source attempt.
        let mut rng = RfbRng::seeded(13);
        let mut expected_rng = rng.clone();
        assert!(
            attempt(
                &mut expected_rng,
                generation,
                15,
                ItemGenerationMode::Good,
                false
            )
            .is_none()
        );
        let (activation, charges) = attempt(
            &mut expected_rng,
            generation,
            17,
            ItemGenerationMode::Ordinary,
            false,
        )
        .unwrap();
        assert_eq!(
            natural(&mut rng, generation, 15, ItemGenerationMode::Good, false),
            Some((activation, charges, 17))
        );
        assert_eq!(rng, expected_rng);
        let mut unavailable = generation.clone();
        for row in &mut unavailable.rfb_device.as_mut().unwrap().effects {
            row.rarity = 0;
        }
        let mut expected_rng = rng.clone();
        assert!(
            attempt(
                &mut expected_rng,
                &unavailable,
                15,
                ItemGenerationMode::Ordinary,
                false
            )
            .is_none()
        );
        assert!(
            attempt(
                &mut expected_rng,
                &unavailable,
                17,
                ItemGenerationMode::Ordinary,
                false
            )
            .is_none()
        );
        assert!(
            natural(
                &mut rng,
                &unavailable,
                15,
                ItemGenerationMode::Ordinary,
                false
            )
            .is_none()
        );
        assert_eq!(rng, expected_rng);
    }
}
