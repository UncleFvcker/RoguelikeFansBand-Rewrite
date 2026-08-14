// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeSet;

use rfb_content::{
    ActorResistanceLevel, AffixDefinition, AffixPropertyBundleDefinition, ContentCatalog,
    RfbEgoTypeDefinition, StatModifiers,
};
use rfb_protocol::{ItemActivationDto, ItemChargesDto};

use crate::{
    rng::RfbRng,
    state::{ItemInstance, RolledAffixState},
};

use super::{initial_item_runtime_state, merge_equipment_bonuses, roll_weighted_index_with_rng};

/// Complete generated affix state shared by content-driven consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EgoMaterialization {
    pub(super) affix_ids: Vec<String>,
    pub(super) rolled_affixes: Vec<RolledAffixState>,
    pub(super) activation: Option<ItemActivationDto>,
    pub(super) charges: Option<ItemChargesDto>,
}

impl EgoMaterialization {
    /// Commits a fully prepared materialization to an existing item in one step.
    pub(super) fn apply_to(self, item: &mut ItemInstance) {
        item.affix_ids = self.affix_ids;
        item.rolled_affixes = self.rolled_affixes;
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
    EgoMaterialization {
        affix_ids,
        rolled_affixes,
        activation,
        charges,
    }
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
            });
        }
    }
    rolled_affixes
}

fn merge_affix_properties(
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

    use crate::game::Game;
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
