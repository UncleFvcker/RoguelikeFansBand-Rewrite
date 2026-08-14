// SPDX-License-Identifier: MPL-2.0

use rfb_content::{AffixDefinition, ContentCatalog, RfbEgoTypeDefinition};

use crate::rng::RfbRng;

use super::roll_weighted_index_with_rng;

/// Selects one authoritative RFB ego without changing any item state.
/// Callers own the exact item-to-ego-type policy and the later materialization.
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

    use rfb_content::{
        AffixDefinition, EquipmentBonuses, RfbEgoGenerationDefinition, StatModifiers,
    };

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

    #[test]
    fn ego_weight_matches_below_in_range_and_above_max_penalties() {
        assert_eq!(rfb_ego_weight(4, 10, 20, 5), 416);
        assert_eq!(rfb_ego_weight(4, 10, 20, 15), 2_500);
        assert_eq!(rfb_ego_weight(4, 10, 20, 25), 156);
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
