// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

use rfb_content::ItemMountUseDefinition;
use rfb_protocol::{EquipmentBonusesDto, StatModifiersDto, TargetSelection};

use crate::{
    effect::{STATUS_HASTE, StatusApplication, StatusInstance, StatusStacking},
    event::DomainEvent,
    resistance::definition_resistance_profile,
    state::{Actor, RidingBond},
};

use super::{Game, actor_spawn_max_hp, apply_status_application};

pub(super) const RIDING_BOND_MAX: u16 = 10_000;

impl Game {
    fn mount_use_minimum_bond(mount_use: &ItemMountUseDefinition) -> u16 {
        match mount_use {
            ItemMountUseDefinition::Heal { minimum_bond, .. }
            | ItemMountUseDefinition::Haste { minimum_bond, .. } => *minimum_bond,
        }
    }

    pub(super) fn mount_item_is_usable(&self, item_kind_id: &str) -> bool {
        let Some(mount_use) = self
            .content
            .item(item_kind_id)
            .and_then(|definition| definition.mount_use.as_ref())
        else {
            return false;
        };
        self.active_riding_bond_value()
            .is_some_and(|value| value >= Self::mount_use_minimum_bond(mount_use))
    }

    pub(super) fn mount_item_target_is_valid(
        &self,
        item_id: &str,
        target: Option<&TargetSelection>,
    ) -> Option<bool> {
        let Some(TargetSelection::Entity { entity_id }) = target else {
            return None;
        };
        let item = self.items.iter().find(|item| {
            item.id == item_id
                && item.location == crate::state::ItemLocation::Inventory
                && item.quantity > 0
        })?;
        self.content
            .item(&item.kind_id)
            .and_then(|definition| definition.mount_use.as_ref())?;
        Some(
            self.riding_actor_id.as_deref() == Some(entity_id.as_str())
                && self.mount_item_is_usable(&item.kind_id),
        )
    }

    pub(super) fn use_inventory_mount_item(
        &mut self,
        item_index: usize,
        item_kind_id: &str,
        mount_use: &ItemMountUseDefinition,
        target: Option<&TargetSelection>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<rfb_protocol::Position>,
    ) {
        let item_id = self.items[item_index].id.clone();
        if self.mount_item_target_is_valid(&item_id, target) != Some(true) {
            events.push(DomainEvent::ItemUseUnavailable);
            return;
        }
        let target_id = self
            .riding_actor_id
            .clone()
            .expect("validated mount item target must be the current mount");
        let target_index = self
            .entities
            .iter()
            .position(|actor| actor.id == target_id)
            .expect("active mount must remain present");

        self.mark_item_tried(item_kind_id);
        self.mark_item_aware(item_kind_id);
        if self.items[item_index].quantity == 1 {
            let removed = self.items.remove(item_index);
            self.item_property_knowledge.remove(&removed.id);
        } else {
            self.items[item_index].quantity -= 1;
        }

        match mount_use {
            ItemMountUseDefinition::Heal {
                dice,
                sides,
                amount,
                full,
                clear_statuses,
                ..
            } => {
                let rolled = if *dice > 0 {
                    self.roll_damage(*dice, *sides)
                } else {
                    0
                };
                let healing = i32::try_from(*amount)
                    .unwrap_or(i32::MAX)
                    .saturating_add(rolled);
                let maximum = self.entities[target_index].max_hp;
                self.entities[target_index].hp = if *full {
                    maximum
                } else {
                    self.entities[target_index]
                        .hp
                        .saturating_add(healing)
                        .min(maximum)
                };
                self.entities[target_index]
                    .statuses
                    .retain(|status| !clear_statuses.contains(&status.kind_id));
            }
            ItemMountUseDefinition::Haste {
                duration_dice,
                duration_sides,
                duration_bonus,
                extension,
                ..
            } => {
                let already_hasted = self.entities[target_index]
                    .statuses
                    .iter()
                    .any(|status| status.kind_id == STATUS_HASTE);
                let duration = if already_hasted {
                    u32::from(*extension)
                } else {
                    u32::try_from(self.roll_damage(*duration_dice, *duration_sides))
                        .unwrap_or(u32::MAX)
                        .saturating_add(u32::from(*duration_bonus))
                };
                apply_status_application(
                    &mut self.entities[target_index].statuses,
                    StatusApplication {
                        status: StatusInstance {
                            kind_id: STATUS_HASTE.to_owned(),
                            intensity: 1,
                            remaining_ticks: duration,
                            source_id: Some(item_kind_id.to_owned()),
                            granted_resistances: BTreeMap::new(),
                            granted_brands: BTreeSet::new(),
                            granted_modifiers: StatModifiersDto::default(),
                            granted_equipment_bonuses: EquipmentBonusesDto::default(),
                            granted_status_immunities: BTreeSet::new(),
                            granted_race_id: None,
                            grants_wall_passage: false,
                            incoming_damage_percent: 100,
                        },
                        stacking: StatusStacking::Extend,
                    },
                );
            }
        }
        changed.insert(self.entities[target_index].position);
        events.push(DomainEvent::MountPotionUsed {
            item_kind_id: item_kind_id.to_owned(),
            target_kind_id: self.entities[target_index].kind_id.clone(),
        });
    }

    pub(super) fn ensure_riding_bond(&mut self, actor_index: usize) {
        let actor = &self.entities[actor_index];
        if self
            .riding_bond
            .as_ref()
            .is_some_and(|bond| bond.actor_id == actor.id && bond.actor_kind_id == actor.kind_id)
        {
            return;
        }
        self.riding_bond = Some(RidingBond {
            actor_id: actor.id.clone(),
            actor_kind_id: actor.kind_id.clone(),
            value: 0,
        });
    }

    pub(super) fn clear_riding_bond_for(&mut self, actor_id: &str) {
        if self
            .riding_bond
            .as_ref()
            .is_some_and(|bond| bond.actor_id == actor_id)
        {
            self.riding_bond = None;
        }
    }

    pub(super) fn active_riding_bond_value(&self) -> Option<u16> {
        let riding_actor_id = self.riding_actor_id.as_deref()?;
        let bond = self.riding_bond.as_ref()?;
        let actor = self.entities.iter().find(|actor| {
            actor.id == riding_actor_id
                && actor.id == bond.actor_id
                && actor.kind_id == bond.actor_kind_id
                && self.actor_is_player_aligned(actor)
        })?;
        (actor.hp > 0).then_some(bond.value)
    }

    fn gain_active_riding_bond(&mut self, amount: u32, events: &mut Vec<DomainEvent>) {
        if self.active_riding_bond_value().is_none() {
            return;
        }
        let bond = self
            .riding_bond
            .as_mut()
            .expect("active riding bond must exist");
        let previous = bond.value;
        bond.value = u32::from(bond.value)
            .saturating_add(amount)
            .min(u32::from(RIDING_BOND_MAX))
            .try_into()
            .expect("bounded riding bond must fit u16");
        if previous < RIDING_BOND_MAX && bond.value == RIDING_BOND_MAX {
            events.push(DomainEvent::RidingBondMaxed {
                target_kind_id: bond.actor_kind_id.clone(),
            });
        }
    }

    pub(super) fn pet_experience_reward(&self, pet_level: u32, target: &Actor) -> u64 {
        let Some(target_definition) = self.actor_runtime_definition(target) else {
            return 0;
        };
        let mut amount = target_definition
            .experience_value
            .saturating_mul(u64::from(target_definition.level))
            .saturating_div(u64::from(pet_level).saturating_add(2));
        if self.floor_depth(&self.current_floor_id) == 0 {
            amount /= 5;
        }
        amount
    }

    pub(super) fn grant_pet_experience(
        &mut self,
        actor_id: &str,
        amount: u64,
        events: &mut Vec<DomainEvent>,
    ) {
        if amount == 0 {
            return;
        }
        let Some(index) = self.entities.iter().position(|actor| actor.id == actor_id) else {
            return;
        };
        let kind_id = self.entities[index].kind_id.clone();
        let Some(evolution) = self
            .content
            .actor(&kind_id)
            .and_then(|definition| definition.evolution.clone())
        else {
            return;
        };
        self.entities[index].experience = self.entities[index].experience.saturating_add(amount);
        if self.entities[index].experience < evolution.required_experience {
            return;
        }

        let Some(next_definition) = self.content.actor(&evolution.next_actor_kind_id).cloned()
        else {
            return;
        };
        let previous_max_hp = self.entities[index].max_hp.max(1);
        let previous_hp = self.entities[index].hp.max(0);
        let next_max_hp = actor_spawn_max_hp(&mut self.rng, &next_definition).max(1);
        let next_hp = i64::from(previous_hp)
            .saturating_mul(i64::from(next_max_hp))
            .saturating_div(i64::from(previous_max_hp));
        let actor_id = self.entities[index].id.clone();
        let reset_bond = self
            .riding_bond
            .as_ref()
            .is_some_and(|bond| bond.actor_id == actor_id && bond.actor_kind_id == kind_id);
        self.entities[index].kind_id = next_definition.id.clone();
        self.entities[index].appearance_kind_id = None;
        self.entities[index].experience = 0;
        self.entities[index].hp = i32::try_from(next_hp).unwrap_or(i32::MAX).max(1);
        self.entities[index].max_hp = next_max_hp;
        self.entities[index].speed = next_definition.speed;
        self.entities[index].resistances = definition_resistance_profile(&next_definition);
        self.clear_riding_bond_for(&actor_id);
        if reset_bond {
            self.riding_bond = Some(RidingBond {
                actor_id,
                actor_kind_id: next_definition.id.clone(),
                value: 0,
            });
        }
        events.push(DomainEvent::PetEvolved {
            previous_kind_id: kind_id,
            target_kind_id: next_definition.id,
        });
    }

    pub(super) fn reward_player_kill_riding_bond(
        &mut self,
        target: &Actor,
        events: &mut Vec<DomainEvent>,
    ) {
        if self.actor_is_player_side(target) {
            return;
        }
        let Some(target_definition) = self.actor_runtime_definition(target) else {
            return;
        };
        self.gain_active_riding_bond(target_definition.level, events);
        if self.active_riding_bond_value() != Some(RIDING_BOND_MAX) {
            return;
        }
        let Some(actor_id) = self.riding_actor_id.clone() else {
            return;
        };
        let Some(pet_level) = self
            .entities
            .iter()
            .find(|actor| actor.id == actor_id)
            .and_then(|actor| self.actor_runtime_definition(actor))
            .map(|definition| definition.level)
        else {
            return;
        };
        let amount = self.pet_experience_reward(pet_level, target) / 10;
        self.grant_pet_experience(&actor_id, amount, events);
    }

    pub(super) fn reward_controlled_actor_kill(
        &mut self,
        source_actor_id: &str,
        target: &Actor,
        events: &mut Vec<DomainEvent>,
    ) {
        if self.actor_is_player_side(target) {
            return;
        }
        let Some(source_index) = self
            .entities
            .iter()
            .position(|actor| actor.id == source_actor_id && self.actor_is_player_aligned(actor))
        else {
            return;
        };
        let Some(pet_level) = self
            .actor_runtime_definition(&self.entities[source_index])
            .map(|definition| definition.level)
        else {
            return;
        };
        let pet_amount = self.pet_experience_reward(pet_level, target);
        if self.riding_actor_id.as_deref() == Some(source_actor_id) {
            let target_level = self
                .actor_runtime_definition(target)
                .map_or(0, |definition| definition.level);
            self.gain_active_riding_bond(target_level, events);
        }
        let full_bond = self.riding_actor_id.as_deref() == Some(source_actor_id)
            && self.active_riding_bond_value() == Some(RIDING_BOND_MAX);
        let player_amount = if full_bond {
            pet_amount
        } else {
            pet_amount / 5
        };
        let actor_amount = if full_bond {
            pet_amount
        } else {
            pet_amount.saturating_sub(player_amount)
        };
        self.apply_player_experience(player_amount, events);
        self.grant_pet_experience(source_actor_id, actor_amount, events);
    }
}
