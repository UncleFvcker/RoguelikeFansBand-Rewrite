// SPDX-License-Identifier: MPL-2.0

use super::*;

pub(super) const SAFE_UPKEEP_PERCENT: u16 = 484;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PetUpkeep {
    pub(super) controlled_pets: u16,
    pub(super) total_levels: u32,
    pub(super) percent: u16,
}

impl PetUpkeep {
    pub(super) fn unsafe_warning(self) -> bool {
        self.percent > SAFE_UPKEEP_PERCENT
    }
}

impl Game {
    pub(super) fn pet_upkeep(&self) -> PetUpkeep {
        let mut controlled_pets = 0_u16;
        let total_levels = self
            .entities
            .iter()
            .filter(|actor| actor.hp > 0 && self.actor_is_player_aligned(actor))
            .fold(0_u32, |total, actor| {
                controlled_pets = controlled_pets.saturating_add(1);
                let definition = self
                    .actor_runtime_definition(actor)
                    .expect("controlled actor definition must remain available");
                let level = definition.level;
                let cost = if definition.tags.iter().any(|tag| tag == "unique") {
                    level.saturating_add(5).saturating_mul(10)
                } else {
                    level
                };
                total.saturating_add(cost)
            });
        if controlled_pets == 0 {
            return PetUpkeep {
                controlled_pets,
                total_levels: 0,
                percent: 0,
            };
        }
        let divisor = self
            .character_definitions()
            .map_or(40, |(_, _, class, _)| class.pet_upkeep_divisor)
            .max(1);
        let free_levels = u32::from(self.progress.level)
            .saturating_mul(80)
            .saturating_div(u32::from(divisor));
        let mut percent = total_levels.saturating_sub(free_levels);
        if percent > 100 {
            percent = percent.saturating_add(percent.saturating_sub(100) / 2);
        }
        PetUpkeep {
            controlled_pets,
            total_levels,
            percent: u16::try_from(percent.min(1_500)).expect("capped upkeep must fit u16"),
        }
    }

    pub(super) fn pet_upkeep_dto(&self) -> rfb_protocol::PetUpkeepDto {
        let upkeep = self.pet_upkeep();
        let mana_current = self.pet_mana_pool().map_or(0, |pool| pool.current);
        rfb_protocol::PetUpkeepDto {
            controlled_pets: upkeep.controlled_pets,
            total_levels: upkeep.total_levels,
            upkeep_percent: upkeep.percent,
            unsafe_warning: upkeep.unsafe_warning(),
            dismissal_required: upkeep.percent > 100 && mana_current == 0,
        }
    }

    pub(super) fn player_resource_recovery_change(&self, id: &str, resting: bool) -> i64 {
        let Some(definition) = self.content.resource(id) else {
            return 0;
        };
        let base = if resting {
            definition.rest_recovery_amount
        } else {
            definition.wait_recovery_amount
        };
        let Some(profile) = self
            .casting_profile()
            .filter(|profile| profile.resource_id == id)
        else {
            return i64::from(base);
        };
        let upkeep = self.pet_upkeep().percent;
        if upkeep <= 100 {
            let class_recovery =
                base.saturating_mul(u32::from(profile.resource_recovery_percent)) / 100;
            return i64::from(
                class_recovery.saturating_mul(u32::from(100_u16.saturating_sub(upkeep))) / 100,
            );
        }

        // RFB deliberately uses normal regeneration rather than boosted rest
        // regeneration for negative upkeep. Rewrite resources are integral, so
        // round a non-zero loss up to keep every over-100% upkeep observable.
        let loss_percent = u32::from(upkeep - 100);
        let loss = definition
            .wait_recovery_amount
            .saturating_mul(loss_percent)
            .div_ceil(100);
        -i64::from(loss)
    }

    pub(super) fn dismiss_controlled_pets(
        &mut self,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> u16 {
        let mut ids = self
            .entities
            .iter()
            .filter(|actor| actor.hp > 0 && self.actor_is_player_aligned(actor))
            .map(|actor| actor.id.clone())
            .collect::<Vec<_>>();
        ids.sort();
        let mut dismissed = 0_u16;
        for id in ids {
            let Some(index) = self.entities.iter().position(|actor| actor.id == id) else {
                continue;
            };
            self.remove_pet_at(index, changed, removed_entities);
            dismissed = dismissed.saturating_add(1);
        }
        dismissed
    }

    pub(super) fn apply_pet_upkeep_mana_loss(&mut self, events: &mut Vec<DomainEvent>) {
        let Some(profile) = self.casting_profile() else {
            return;
        };
        let resource_id = profile.resource_id.clone();
        let change = self.player_resource_recovery_change(&resource_id, false);
        if change >= 0 {
            return;
        }
        let upkeep_percent = self.pet_upkeep().percent;
        let Some(pool) = self.resources.get_mut(&resource_id) else {
            return;
        };
        let before = pool.current;
        pool.current = pool
            .current
            .saturating_sub(u32::try_from(-change).unwrap_or(u32::MAX));
        if pool.current < before {
            events.push(DomainEvent::PetUpkeepManaLost {
                resource_id,
                amount: before - pool.current,
                upkeep_percent,
            });
        }
        if self.pet_upkeep_dto().dismissal_required {
            events.push(DomainEvent::PetUpkeepDismissalRequired { upkeep_percent });
        }
    }

    pub(super) fn resolve_neglected_pet(
        &mut self,
        index: usize,
        neglect_allowed: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> bool {
        if !neglect_allowed || self.entities[index].hp <= 0 || !self.entity_is_player_aligned(index)
        {
            return false;
        }
        let upkeep = self.pet_upkeep().percent;
        if upkeep <= SAFE_UPKEEP_PERCENT {
            return false;
        }
        let (mana, maximum_mana) = self
            .pet_mana_pool()
            .map_or((0, 0), |pool| (pool.current, pool.maximum));
        if u64::from(mana).saturating_mul(15) > u64::from(maximum_mana).saturating_mul(14) {
            return false;
        }
        let definition = self
            .actor_runtime_definition(&self.entities[index])
            .expect("controlled actor definition must remain available");
        let level = definition.level;
        let unique = definition.tags.iter().any(|tag| tag == "unique");
        let aligned = definition
            .tags
            .iter()
            .any(|tag| matches!(tag.as_str(), "good" | "evil"));
        let hydra = definition.glyph == "M";

        if mana.saturating_mul(3) > maximum_mana
            && self.rng.bounded(u64::from(maximum_mana)) < u64::from(mana)
        {
            return false;
        }
        if self.rng.bounded(if aligned { 2 } else { 4 }) != 0 {
            return false;
        }
        if self.rng.bounded(1_500) + 1 > u64::from(upkeep) {
            return false;
        }
        if unique && (self.rng.bounded(3) != 0 || self.rng.bounded(1_500) + 1 > u64::from(upkeep)) {
            return false;
        }
        if self.rng.bounded(125) + 1 < u64::from(level)
            && self.rng.bounded(2_000) + 1 > u64::from(upkeep)
        {
            return false;
        }
        // Current player-controlled actors have no monster parent identity, so
        // they follow RFB's parent_m_idx == 0 branch.
        if self.rng.bounded(2) == 0 {
            return false;
        }

        let disappears = (unique && self.rng.bounded(2) == 0)
            || hydra
            || (mana > 0
                && self.rng.bounded(u64::from(maximum_mana)) < u64::from(mana)
                && self.rng.bounded(125) + 1 > u64::from(level));
        let entity_id = self.entities[index].id.clone();
        let target_kind_id = self.entities[index].kind_id.clone();
        if disappears {
            self.remove_pet_at(index, changed, removed_entities);
        } else {
            self.clear_riding_bond_for(&entity_id);
            self.entities[index].controller_id = None;
            self.entities[index].summon = None;
        }
        events.push(DomainEvent::PetNeglected {
            entity_id,
            target_kind_id,
            disappeared: disappears,
        });
        disappears
    }

    fn pet_mana_pool(&self) -> Option<&ResourcePool> {
        let profile = self.casting_profile()?;
        self.resources.get(&profile.resource_id)
    }

    fn remove_pet_at(
        &mut self,
        index: usize,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) {
        let removed = self.entities.remove(index);
        if self.riding_actor_id.as_deref() == Some(removed.id.as_str()) {
            self.riding_actor_id = None;
        }
        self.clear_riding_bond_for(&removed.id);
        let carried_item_ids = self
            .items
            .iter()
            .filter_map(|item| match &item.location {
                ItemLocation::CarriedBy { actor_id } if actor_id == &removed.id => {
                    Some(item.id.clone())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        self.items
            .retain(|item| !carried_item_ids.contains(&item.id));
        for item_id in carried_item_ids {
            self.item_property_knowledge.remove(&item_id);
        }
        changed.insert(removed.position);
        removed_entities.push(removed.id);
    }
}
