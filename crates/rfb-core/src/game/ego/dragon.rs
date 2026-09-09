// SPDX-License-Identifier: MPL-2.0
use super::*;

/// object2.c:dragon_resist and the type-specific apply_magic suppression.
/// Craft calls the Ego materializer directly, preserving these base properties.
pub(in crate::game) fn materialize(
    rng: &mut RfbRng,
    item: &ItemDefinition,
    power: &mut i16,
    special_no_fixed_artifact: bool,
) -> Option<AffixPropertyBundleDefinition> {
    let base = item.rfb_base_kind?;
    let fang = (base.tval, base.sval) == (23, 35);
    if !fang
        && !matches!(
            (base.tval, base.sval),
            (32, 8) | (35, 7) | (34, 6) | (31, 6) | (30, 4)
        )
    {
        return None;
    }
    let mut properties = AffixPropertyBundleDefinition::default();
    let mut count = 0;
    loop {
        if fang && one_in(rng, 3) {
            properties.brands.insert(match rng.bounded(5) {
                0 => WeaponBrand::Acid,
                1 => WeaponBrand::Cold,
                2 => WeaponBrand::Fire,
                3 => WeaponBrand::Electricity,
                _ => WeaponBrand::Poison,
            });
        } else if one_in(rng, 4) {
            if one_in(rng, 7) {
                add_resistance(&mut properties, ActorDamageType::Poison);
            } else {
                add_one_elemental_resistance(rng, &mut properties);
            }
        } else {
            add_one_high_resistance(rng, &mut properties);
        }
        count += 1;
        if !one_in(rng, 1 + count) {
            break;
        }
    }
    if (fang || !special_no_fixed_artifact) && !one_in(rng, 3) {
        *power = 0;
    }
    Some(properties)
}

#[cfg(test)]
mod tests;
