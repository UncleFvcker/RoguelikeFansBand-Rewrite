// SPDX-License-Identifier: MPL-2.0

use rfb_content::{MutationDefinition, MutationRatingDefinition};

use super::*;

const GOOD_LUCK_MUTATION_ID: &str = "rfb.mutation.good-luck";
const BAD_LUCK_MUTATION_ID: &str = "rfb.mutation.bad-luck";

fn scale_by_ratio(value: u64, ratio: rfb_content::MutationRatioDefinition) -> u64 {
    value
        .saturating_mul(u64::from(ratio.numerator))
        .saturating_div(u64::from(ratio.denominator))
}

#[derive(Clone, Copy)]
enum RandomMutationOperation {
    Gain,
    Lose,
}

impl Game {
    pub(super) fn mutation_activation_for_ability(
        &self,
        ability_id: &str,
    ) -> Option<&MutationActivationDefinition> {
        self.content
            .mutations()
            .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
            .filter_map(|mutation| mutation.activation.as_ref())
            .find(|activation| activation.ability_id == ability_id)
    }

    pub(super) fn gain_mutation(
        &mut self,
        mutation_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        if !self.gain_mutation_without_refresh(mutation_id, events) {
            return false;
        }
        self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        true
    }

    pub(super) fn gain_mutation_without_refresh(
        &mut self,
        mutation_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let Some(definition) = self.content.mutation(mutation_id).cloned() else {
            return false;
        };
        if self.progress.active_mutation_ids.contains(mutation_id) {
            return false;
        }
        for removed_id in &definition.removes_on_gain {
            if self.progress.active_mutation_ids.contains(removed_id)
                && !self.progress.locked_mutation_ids.contains(removed_id)
            {
                self.progress.active_mutation_ids.remove(removed_id);
                let removed = self
                    .content
                    .mutation(removed_id)
                    .expect("validated mutation removal must remain available");
                events.push(DomainEvent::MutationLost {
                    mutation_id: removed.id.clone(),
                    name: removed.name.clone(),
                });
            }
        }
        self.progress
            .active_mutation_ids
            .insert(definition.id.clone());
        events.push(DomainEvent::MutationGained {
            mutation_id: definition.id,
            name: definition.name,
        });
        if definition.auto_identify_items {
            let count = self.identify_carried_items();
            if count > 0 {
                events.push(DomainEvent::ItemAutoIdentified { count });
            }
        }
        true
    }

    pub(super) fn lose_mutation(
        &mut self,
        mutation_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        if !self.lose_mutation_without_refresh(mutation_id, events) {
            return false;
        }
        self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        true
    }

    pub(super) fn lose_mutation_without_refresh(
        &mut self,
        mutation_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let Some(definition) = self.content.mutation(mutation_id).cloned() else {
            return false;
        };
        if !self.progress.active_mutation_ids.contains(mutation_id)
            || self.progress.locked_mutation_ids.contains(mutation_id)
        {
            return false;
        }
        self.progress.active_mutation_ids.remove(mutation_id);
        events.push(DomainEvent::MutationLost {
            mutation_id: definition.id,
            name: definition.name,
        });
        true
    }

    pub(super) fn lose_all_unlocked_mutations(&mut self, events: &mut Vec<DomainEvent>) -> usize {
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let removed = self.remove_all_unlocked_mutations_without_refresh();
        if removed.is_empty() {
            return 0;
        }

        self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        for (mutation_id, name) in &removed {
            events.push(DomainEvent::MutationLost {
                mutation_id: mutation_id.clone(),
                name: name.clone(),
            });
        }
        removed.len()
    }

    pub(super) fn remove_all_unlocked_mutations_without_refresh(
        &mut self,
    ) -> Vec<(String, String)> {
        let mut removed = self
            .content
            .mutations()
            .filter(|definition| {
                self.progress.active_mutation_ids.contains(&definition.id)
                    && !self.progress.locked_mutation_ids.contains(&definition.id)
            })
            .map(|definition| {
                (
                    definition.source_index,
                    definition.id.clone(),
                    definition.name.clone(),
                )
            })
            .collect::<Vec<_>>();
        removed.sort_by_key(|(source_index, _, _)| *source_index);
        for (_, mutation_id, _) in &removed {
            self.progress.active_mutation_ids.remove(mutation_id);
        }
        removed
            .into_iter()
            .map(|(_, mutation_id, name)| (mutation_id, name))
            .collect()
    }

    pub(super) fn gain_random_mutation(&mut self, events: &mut Vec<DomainEvent>) -> Option<String> {
        let mutation_id = self.select_random_mutation(RandomMutationOperation::Gain)?;
        let gained = self.gain_mutation(&mutation_id, events);
        debug_assert!(gained, "selected mutation must remain gainable");
        gained.then_some(mutation_id)
    }

    pub(super) fn gain_random_mutation_without_refresh(
        &mut self,
        events: &mut Vec<DomainEvent>,
    ) -> Option<String> {
        let mutation_id = self.select_random_mutation(RandomMutationOperation::Gain)?;
        let gained = self.gain_mutation_without_refresh(&mutation_id, events);
        debug_assert!(gained, "selected mutation must remain gainable");
        gained.then_some(mutation_id)
    }

    pub(super) fn lose_random_mutation(&mut self, events: &mut Vec<DomainEvent>) -> Option<String> {
        let mutation_id = self.select_random_mutation(RandomMutationOperation::Lose)?;
        let lost = self.lose_mutation(&mutation_id, events);
        debug_assert!(lost, "selected mutation must remain removable");
        lost.then_some(mutation_id)
    }

    pub(super) fn lose_random_mutation_without_refresh(
        &mut self,
        events: &mut Vec<DomainEvent>,
    ) -> Option<String> {
        let mutation_id = self.select_random_mutation(RandomMutationOperation::Lose)?;
        let lost = self.lose_mutation_without_refresh(&mutation_id, events);
        debug_assert!(lost, "selected mutation must remain removable");
        lost.then_some(mutation_id)
    }

    fn select_random_mutation(&mut self, operation: RandomMutationOperation) -> Option<String> {
        let mut candidates = self
            .content
            .mutations()
            .filter_map(|definition| {
                if !definition.random_selection_enabled {
                    return None;
                }
                let eligible = match operation {
                    RandomMutationOperation::Gain => {
                        !self.progress.active_mutation_ids.contains(&definition.id)
                    }
                    RandomMutationOperation::Lose => {
                        self.progress.active_mutation_ids.contains(&definition.id)
                            && !self.progress.locked_mutation_ids.contains(&definition.id)
                    }
                };
                let weight =
                    eligible.then(|| self.mutation_random_weight(definition, operation))?;
                (weight > 0).then(|| (definition.source_index, definition.id.clone(), weight))
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|(source_index, _, _)| *source_index);
        let total = candidates.iter().map(|(_, _, weight)| *weight).sum::<u64>();
        if total == 0 {
            return None;
        }
        let roll = self.rng.bounded(total);
        let mut cumulative = 0_u64;
        candidates.into_iter().find_map(|(_, mutation_id, weight)| {
            cumulative = cumulative.saturating_add(weight);
            (roll < cumulative).then_some(mutation_id)
        })
    }

    fn mutation_random_weight(
        &self,
        definition: &MutationDefinition,
        operation: RandomMutationOperation,
    ) -> u64 {
        let base = u64::from(definition.random_weight);
        if base == 0 {
            return 0;
        }
        let good_luck = self
            .progress
            .active_mutation_ids
            .contains(GOOD_LUCK_MUTATION_ID);
        let bad_luck = self
            .progress
            .active_mutation_ids
            .contains(BAD_LUCK_MUTATION_ID);
        let positive = matches!(
            definition.rating,
            MutationRatingDefinition::Good | MutationRatingDefinition::Great
        );
        let negative = matches!(
            definition.rating,
            MutationRatingDefinition::Awful | MutationRatingDefinition::Bad
        );
        let reduced = match operation {
            RandomMutationOperation::Gain => (good_luck && negative) || (bad_luck && positive),
            RandomMutationOperation::Lose => (good_luck && positive) || (bad_luck && negative),
        };
        if reduced { 1 } else { base }
    }

    pub(super) fn mutation_regeneration_percent(&self) -> u64 {
        let unlocked = self
            .progress
            .active_mutation_ids
            .len()
            .saturating_sub(self.progress.locked_mutation_ids.len());
        100_u64
            .saturating_sub(
                u64::try_from(unlocked)
                    .unwrap_or(u64::MAX)
                    .saturating_mul(10),
            )
            .max(10)
    }

    pub(super) fn player_mutation_action_energy_cost(&self, action: &GameAction, cost: i32) -> i32 {
        let mut mutations = self
            .content
            .mutations()
            .filter(|definition| self.progress.active_mutation_ids.contains(&definition.id))
            .collect::<Vec<_>>();
        let walking = matches!(
            action,
            GameAction::Move { .. } | GameAction::TravelWorld { .. }
        );
        let scroll_use = match action {
            GameAction::UseItem { item_id, .. } => self
                .items
                .iter()
                .find(|item| item.id == *item_id)
                .and_then(|item| self.content.item(&item.kind_id))
                .is_some_and(|definition| definition.tags.iter().any(|tag| tag == "scroll")),
            _ => false,
        };
        if walking {
            // RFB applies Limp before Fleet of Foot; descending source order
            // preserves that integer-rounding order without hard-coded IDs.
            mutations.sort_by_key(|definition| std::cmp::Reverse(definition.source_index));
        }
        let scaled = mutations.into_iter().fold(
            u64::try_from(cost.max(0)).unwrap_or(0),
            |value, mutation| {
                if walking {
                    mutation
                        .movement_energy_multiplier
                        .map_or(value, |ratio| scale_by_ratio(value, ratio))
                } else if scroll_use {
                    mutation
                        .scroll_energy_multiplier
                        .map_or(value, |ratio| scale_by_ratio(value, ratio))
                } else {
                    value
                }
            },
        );
        i32::try_from(scaled).unwrap_or(i32::MAX)
    }

    pub(super) fn player_kill_experience_reward(&self, amount: u64) -> u64 {
        self.content
            .mutations()
            .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
            .fold(amount, |value, mutation| {
                value
                    .saturating_mul(
                        100_u64.saturating_add(u64::from(mutation.kill_experience_bonus_percent)),
                    )
                    .saturating_div(100)
            })
    }

    pub(super) fn player_relative_experience_reward(&self, amount: u64) -> u64 {
        self.content
            .mutations()
            .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
            .filter_map(|mutation| mutation.relative_experience_multiplier)
            .fold(amount, scale_by_ratio)
    }

    pub(super) fn player_spell_failure_modifier_percent(&self) -> i32 {
        self.content
            .mutations()
            .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
            .fold(0_i32, |total, mutation| {
                total.saturating_add(mutation.spell_failure_modifier_percent)
            })
    }

    pub(super) fn player_auto_identifies_items(&self) -> bool {
        self.content.mutations().any(|mutation| {
            mutation.auto_identify_items && self.progress.active_mutation_ids.contains(&mutation.id)
        })
    }

    pub(super) fn player_has_black_market_standard_prices(&self) -> bool {
        self.content.mutations().any(|mutation| {
            mutation.black_market_standard_prices
                && self.progress.active_mutation_ids.contains(&mutation.id)
        })
    }

    pub(super) fn player_resists_dispel(&mut self) -> bool {
        let chance = self
            .content
            .mutations()
            .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
            .fold(0_u8, |total, mutation| {
                total.saturating_add(mutation.dispel_resistance_percent)
            })
            .min(100);
        chance > 0 && self.rng.bounded(100) < u64::from(chance)
    }

    pub(super) fn player_has_resource_drain_immunity(&self) -> bool {
        self.content.mutations().any(|mutation| {
            mutation.resource_drain_immunity
                && self.progress.active_mutation_ids.contains(&mutation.id)
        })
    }
}
