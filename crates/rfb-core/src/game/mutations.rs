// SPDX-License-Identifier: MPL-2.0

use rfb_content::{MutationDefinition, MutationRatingDefinition};

use super::*;

const GOOD_LUCK_MUTATION_ID: &str = "rfb.mutation.good-luck";
const BAD_LUCK_MUTATION_ID: &str = "rfb.mutation.bad-luck";

#[derive(Clone, Copy)]
enum RandomMutationOperation {
    Gain,
    Lose,
}

impl Game {
    pub(super) fn gain_mutation(
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

        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
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
        self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        events.push(DomainEvent::MutationGained {
            mutation_id: definition.id,
            name: definition.name,
        });
        true
    }

    pub(super) fn lose_mutation(
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

        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        self.progress.active_mutation_ids.remove(mutation_id);
        self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        events.push(DomainEvent::MutationLost {
            mutation_id: definition.id,
            name: definition.name,
        });
        true
    }

    pub(super) fn lose_all_unlocked_mutations(&mut self, events: &mut Vec<DomainEvent>) -> usize {
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
        if removed.is_empty() {
            return 0;
        }

        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        for (_, mutation_id, _) in &removed {
            self.progress.active_mutation_ids.remove(mutation_id);
        }
        self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        for (_, mutation_id, name) in &removed {
            events.push(DomainEvent::MutationLost {
                mutation_id: mutation_id.clone(),
                name: name.clone(),
            });
        }
        removed.len()
    }

    pub(super) fn gain_random_mutation(&mut self, events: &mut Vec<DomainEvent>) -> Option<String> {
        let mutation_id = self.select_random_mutation(RandomMutationOperation::Gain)?;
        let gained = self.gain_mutation(&mutation_id, events);
        debug_assert!(gained, "selected mutation must remain gainable");
        gained.then_some(mutation_id)
    }

    pub(super) fn lose_random_mutation(&mut self, events: &mut Vec<DomainEvent>) -> Option<String> {
        let mutation_id = self.select_random_mutation(RandomMutationOperation::Lose)?;
        let lost = self.lose_mutation(&mutation_id, events);
        debug_assert!(lost, "selected mutation must remain removable");
        lost.then_some(mutation_id)
    }

    fn select_random_mutation(&mut self, operation: RandomMutationOperation) -> Option<String> {
        let mut candidates = self
            .content
            .mutations()
            .filter_map(|definition| {
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
}
