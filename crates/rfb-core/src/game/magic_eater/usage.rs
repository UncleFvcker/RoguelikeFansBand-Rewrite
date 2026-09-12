// SPDX-License-Identifier: MPL-2.0
// Adapted from RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c,
// src/magic_eater.c: _use_object, magic_eater_cast, magic_eater_regen_amt.

use super::*;
use crate::game::item_use::{ItemUsePlan, SettledItemUse};
use crate::game::{EquipmentPassive, Position, STATUS_BERSERK, item_device_generation};
use rfb_content::{ItemDeviceActivationDefinition, ItemUseEffectDefinition};
use rfb_protocol::{ItemCurseEffectDto, TargetSelection};

impl Game {
    fn absorbed_device_profile(
        &self,
        item: &ItemInstance,
    ) -> Option<&ItemDeviceActivationDefinition> {
        let activation = item.activation.as_ref()?;
        item_device_generation(
            &self.content,
            &item.kind_id,
            &item.affix_ids,
            Some(&activation.profile_id),
            item.artifact_name.is_some(),
        )?
        .activations
        .iter()
        .find(|profile| profile.id == activation.profile_id)
    }

    pub(in crate::game) fn validate_absorbed_device_use(
        &self,
        item_id: &str,
        targets: &[TargetSelection],
    ) -> Result<(), CoreError> {
        let item = self
            .items
            .iter()
            .find(|item| {
                item.id == item_id && matches!(item.location, ItemLocation::Absorbed { .. })
            })
            .filter(|_| self.player_is_magic_eater())
            .ok_or(CoreError::AbsorbedDeviceUnavailable("source-item"))?;
        if let Some(reason) = self.absorbed_device_state_unavailable_reason(item) {
            return Err(CoreError::AbsorbedDeviceUnavailable(reason));
        }
        if targets.len() > 1
            && !self.absorbed_device_profile(item).is_some_and(|profile| {
                matches!(
                    profile.effect,
                    ItemUseEffectDefinition::IdentifyItem { full: false }
                )
            })
        {
            return Err(CoreError::AbsorbedDeviceUnavailable(
                "single-target-required",
            ));
        }
        Ok(())
    }

    fn absorbed_device_state_unavailable_reason(
        &self,
        item: &ItemInstance,
    ) -> Option<&'static str> {
        if self.map_scale != MapScaleDto::Local {
            return Some("world-map");
        }
        if self.dungeon_blocks_magic() || self.player_has_anti_magic() {
            return Some("anti-magic");
        }
        if self.player_has_status_kind(STATUS_BERSERK) {
            return Some("berserk");
        }
        if self.player_has_status_kind(STATUS_CONFUSION) {
            return Some("confused");
        }
        if item.activation.is_none() {
            return Some("no-activation");
        }
        None
    }

    fn absorbed_device_energy_cost(&self, item: &ItemInstance) -> i32 {
        if crate::game::ego::item_has_ego(&self.content, item, 256) {
            100 - 10 * i32::from(crate::game::ego::device_pval(item))
        } else {
            100
        }
    }

    pub(in crate::game) fn absorbed_device_slot_dto(
        &self,
        category: AbsorbedDeviceCategoryDto,
        slot: u8,
    ) -> AbsorbedDeviceSlotDto {
        let item = self.absorbed_device(category, slot);
        AbsorbedDeviceSlotDto {
            category,
            slot,
            failure_per_mille: item.and_then(|item| {
                let profile = self.absorbed_device_profile(item)?;
                let activation = item.activation.as_ref()?;
                Some(
                    self.item_device_check_context(
                        item,
                        self.content
                            .item(&item.kind_id)
                            .expect("absorbed item definition"),
                        &profile.effect,
                        activation.device_check_difficulty,
                    )
                    .failure_per_mille(),
                )
            }),
            energy_cost: item.map(|item| self.absorbed_device_energy_cost(item)),
            recovery_per_mille: item.map(|item| self.absorbed_device_recovery_per_mille(item)),
            item: item.map(|item| {
                let mut dto = self.inventory_item_dto(item);
                let reason = self
                    .absorbed_device_state_unavailable_reason(item)
                    .or_else(|| {
                        item.activation.as_ref().and_then(|activation| {
                            item.charges
                                .is_none_or(|sp| sp.current < activation.cost)
                                .then_some("insufficient-energy")
                        })
                    });
                dto.usable = reason.is_none();
                dto.use_unavailable_reason = reason.map(str::to_owned);
                dto
            }),
        }
    }

    pub(in crate::game) fn use_absorbed_device(
        &mut self,
        item_id: &str,
        targets: &[TargetSelection],
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<i32, CoreError> {
        let index = self
            .items
            .iter()
            .position(|item| item.id == item_id)
            .expect("validated absorbed item");
        let item = self.items[index].clone();
        let energy = self.absorbed_device_energy_cost(&item);
        if let Some(fear) = self
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_FEAR)
            && !self.monster_fear_saving_throw(
                None,
                &item.kind_id,
                u32::from(fear.intensity) * 40,
                events,
            )
        {
            events.push(DomainEvent::PlayerFearBlocked {
                status_kind_id: STATUS_FEAR.to_owned(),
            });
            return Ok(energy);
        }
        let definition = self
            .content
            .item(&item.kind_id)
            .expect("absorbed item definition")
            .clone();
        let profile = self
            .absorbed_device_profile(&item)
            .expect("validated absorbed activation")
            .clone();
        let activation = item
            .activation
            .as_ref()
            .expect("usable absorbed activation");
        if !self.check_item_device(
            index,
            &definition,
            &profile.effect,
            activation.device_check_difficulty,
            events,
        ) {
            return Ok(energy);
        }
        if item.charges.expect("activated device SP").current < activation.cost {
            events.push(DomainEvent::ItemUseUnavailable);
            return Ok(0);
        }
        let power = self.effective_player_device_power_bonus()
            + if crate::game::ego::item_has_ego(&self.content, &item, 254) {
                i32::from(crate::game::ego::device_pval(&item))
            } else {
                0
            };
        let mut used = false;
        // Empty selection still executes a self effect; directional/item cancellation refunds time.
        for target in targets
            .iter()
            .map(Some)
            .chain(targets.is_empty().then_some(None))
        {
            let current = self
                .items
                .iter()
                .find(|device| device.id == item_id)
                .expect("body device retained");
            if current.charges.expect("device SP").current < activation.cost {
                break;
            }
            let Some(plan) = self.item_use_plan(
                item_id,
                &profile.effect,
                Some(&profile.target),
                target,
                None,
            ) else {
                break;
            };
            if matches!(plan, ItemUsePlan::CancelledActivation) {
                break;
            }
            if let (ItemUseEffectDefinition::IdentifyItem { full }, ItemUsePlan::Item { item_id }) =
                (&profile.effect, &plan)
                && self
                    .item_property_knowledge
                    .get(item_id)
                    .is_some_and(|knowledge| {
                        if *full {
                            knowledge.identified
                        } else {
                            knowledge.appraised
                        }
                    })
            {
                continue;
            }
            self.resolve_inventory_item_effect(
                SettledItemUse {
                    kind_id: item.kind_id.clone(),
                    profile_id: Some(activation.profile_id.clone()),
                    effect: profile.effect.clone(),
                    plan,
                    device_power_bonus: power,
                },
                events,
                changed,
                removed_entities,
            )?;
            // A detection effect with no findings is still a successful use.
            self.items
                .iter_mut()
                .find(|device| device.id == item_id)
                .expect("body device retained")
                .charges
                .as_mut()
                .expect("device SP")
                .current -= activation.cost;
            used = true;
        }
        Ok(if used { energy } else { 0 })
    }

    pub(in crate::game) fn magic_eater_can_regen(&self) -> bool {
        self.player_is_magic_eater()
            && self.items.iter().any(|item| {
                matches!(item.location, ItemLocation::Absorbed { .. })
                    && item.charges.is_some_and(|sp| sp.current < sp.maximum)
            })
    }

    pub(in crate::game) fn absorbed_device_recovery_per_mille(&self, item: &ItemInstance) -> u16 {
        // Equipment contributes to source p_ptr->regen before the slow-regeneration curse.
        let slow = self.player_has_equipped_curse_effect(ItemCurseEffectDto::SlowRegeneration);
        let equipment = self.items.iter().filter(|item|
            matches!(&item.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) != Some("tool"))
            && self.item_passives(item).contains(&EquipmentPassive::Regeneration)).count() as u64 * 100;
        let regen = self.player_regeneration_rate_percent() + equipment / if slow { 5 } else { 1 };
        let mut base = 3 + regen.saturating_sub(100) / 100;
        if matches!(
            item.location,
            ItemLocation::Absorbed {
                category: AbsorbedDeviceCategoryDto::Rod,
                ..
            }
        ) {
            base *= 5;
        }
        if crate::game::ego::item_has_ego(&self.content, item, 252) {
            base += u64::from(crate::game::ego::device_pval(item)) * base / 5;
        }
        u16::try_from(base).expect("body recovery rate fits u16")
    }
}
