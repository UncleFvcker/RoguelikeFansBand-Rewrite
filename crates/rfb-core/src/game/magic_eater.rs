// SPDX-License-Identifier: MPL-2.0
// Adapted from RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c,
// src/magic_eater.c: gain_magic, _choose and _birth.

use std::collections::BTreeSet;

use rfb_protocol::{
    AbsorbedDeviceCategoryDto, AbsorbedDeviceSlotDto, MagicAbsorptionReplacementDto, MagicEaterDto,
    MapScaleDto, PendingMagicAbsorptionDto,
};

use super::inventory::ItemIdentificationRequest;
use super::{
    CoreError, DomainEvent, Game, GameAction, ItemInstance, ItemLocation, STATUS_CONFUSION,
    STATUS_FEAR, STATUS_PARALYSIS,
};

const SLOTS_PER_CATEGORY: u8 = 10;
mod automatic;
mod usage;
pub(super) use automatic::AutoDeviceEffect;
const CATEGORIES: [(AbsorbedDeviceCategoryDto, &str); 3] = [
    (AbsorbedDeviceCategoryDto::Wand, "wand"),
    (AbsorbedDeviceCategoryDto::Staff, "staff"),
    (AbsorbedDeviceCategoryDto::Rod, "rod"),
];

impl Game {
    /// Explicit desktop preparation: thirty generated/absorbed devices and a floor replacement.
    /// This is not a claim of natural acquisition; the UI performs subsequent replacement/use.
    #[doc(hidden)]
    pub fn debug_prepare_magic_eater_e2e(&mut self) -> Result<(), CoreError> {
        if !self.player_is_magic_eater() || self.map_scale != MapScaleDto::Local {
            return Err(CoreError::InvalidSave(
                "Magic-Eater E2E requires a local Magic-Eater",
            ));
        }
        self.relocate_to_town("demo.town.outpost")?;
        self.pending_magic_absorption = None;
        self.entities.clear();
        self.items.retain(|item| {
            !matches!(
                item.location,
                ItemLocation::CarriedBy { .. } | ItemLocation::Absorbed { .. }
            ) && !item.id.starts_with("e2e.magic-eater.")
        });
        self.player.statuses.clear();
        self.player.position = super::Position {
            x: i32::from(self.width / 2),
            y: i32::from(self.height / 2),
        };
        for dy in -1..=1 {
            for dx in -1..=1 {
                let pos = super::Position {
                    x: self.player.position.x + dx,
                    y: self.player.position.y + dy,
                };
                let index = self.index(pos).expect("centered E2E fixture");
                self.terrain[index] = "demo.terrain.floor".to_owned();
            }
        }
        self.glow.fill(true);
        self.apply_player_experience(
            self.experience_required_for_level(25)
                .saturating_sub(self.progress.experience),
            &mut Vec::new(),
        );
        self.player.hp = self.effective_player_max_hp();
        for (category, kind) in [
            ("wand", "demo.item.magic-missile-wand"),
            ("staff", "demo.item.enlightenment-staff"),
            ("rod", "demo.item.detection-rod"),
        ] {
            for slot in 0..SLOTS_PER_CATEGORY {
                let id = format!("e2e.magic-eater.{category}.{slot}");
                self.debug_add_generated_inventory_item(&id, kind, 1)?;
                self.begin_magic_absorption(&id, &mut Vec::new())?;
                self.select_magic_absorption_slot(slot, &mut Vec::new());
                if slot == 0 {
                    let item = self.items.iter_mut().find(|item| item.id == id).unwrap();
                    item.charges.as_mut().unwrap().current = item.activation.as_ref().unwrap().cost;
                    item.device_recovery_progress = 0;
                }
            }
        }
        self.debug_add_generated_inventory_item(
            "e2e.magic-eater.replacement",
            "demo.item.magic-missile-wand",
            1,
        )?;
        self.items
            .iter_mut()
            .find(|item| item.id == "e2e.magic-eater.replacement")
            .expect("fixture replacement")
            .location = ItemLocation::Ground(self.player.position);
        self.reveal_current_visibility();
        Ok(())
    }

    pub(super) fn absorbed_device_category(
        &self,
        item: &ItemInstance,
    ) -> Option<AbsorbedDeviceCategoryDto> {
        let definition = self.content.item(&item.kind_id)?;
        if !self.item_is_device(item) {
            return None;
        }
        CATEGORIES.iter().find_map(|(category, tag)| {
            definition
                .tags
                .iter()
                .any(|value| value == *tag)
                .then_some(*category)
        })
    }

    pub(super) fn absorbed_device(
        &self,
        category: AbsorbedDeviceCategoryDto,
        slot: u8,
    ) -> Option<&ItemInstance> {
        self.items
            .iter()
            .find(|item| item.location == ItemLocation::Absorbed { category, slot })
    }

    pub(super) fn magic_absorption_source(
        &self,
        item_id: &str,
    ) -> Result<AbsorbedDeviceCategoryDto, CoreError> {
        if !self.player_is_magic_eater() || self.map_scale != MapScaleDto::Local {
            return Err(CoreError::MagicAbsorptionUnavailable("class-or-map"));
        }
        if [STATUS_CONFUSION, STATUS_FEAR, STATUS_PARALYSIS]
            .into_iter()
            .any(|status| self.player_has_status_kind(status))
        {
            return Err(CoreError::MagicAbsorptionUnavailable("player-status"));
        }
        self.items
            .iter()
            .find(|item| {
                item.id == item_id && item.quantity == 1 && self.item_is_in_pack_or_at_feet(item)
            })
            .and_then(|item| self.absorbed_device_category(item))
            .ok_or(CoreError::MagicAbsorptionUnavailable("source-item"))
    }

    fn validate_pending_magic_absorption(&self) -> Result<&PendingMagicAbsorptionDto, CoreError> {
        let pending =
            self.pending_magic_absorption
                .as_ref()
                .ok_or(CoreError::MagicAbsorptionUnavailable(
                    "no-pending-absorption",
                ))?;
        if self.magic_absorption_source(&pending.source_item_id)? != pending.category {
            return Err(CoreError::MagicAbsorptionUnavailable("source-category"));
        }
        if let Some(replacement) = &pending.replacement
            && (replacement.slot >= SLOTS_PER_CATEGORY
                || self
                    .absorbed_device(pending.category, replacement.slot)
                    .is_none_or(|item| item.id != replacement.item_id))
        {
            return Err(CoreError::MagicAbsorptionUnavailable("replacement-changed"));
        }
        Ok(pending)
    }

    /// Some(false) is a free selection/cancellation; only an actual absorption advances time.
    pub(super) fn magic_absorption_action_time(
        &self,
        action: &GameAction,
    ) -> Result<Option<bool>, CoreError> {
        if self.pending_magic_absorption.is_some()
            && !matches!(
                action,
                GameAction::SelectMagicAbsorptionSlot { .. }
                    | GameAction::ResolveMagicAbsorption { .. }
                    | GameAction::SetInterfaceLocale { .. }
            )
        {
            return Err(CoreError::MagicAbsorptionUnavailable(
                "finish-pending-absorption",
            ));
        }
        match action {
            GameAction::SetInterfaceLocale { .. } if self.pending_magic_absorption.is_some() => {
                Ok(Some(false))
            }
            GameAction::InscribeItem { item_id, .. }
                if self.items.iter().any(|item| {
                    item.id == *item_id && matches!(item.location, ItemLocation::Absorbed { .. })
                }) =>
            {
                Ok(Some(false))
            }
            GameAction::CastAbility { ability_id, .. }
                if self.content.ability(ability_id).is_some_and(|ability| {
                    matches!(
                        ability.effect,
                        rfb_content::AbilityEffectDefinition::MagicEaterAbsorb
                    )
                }) =>
            {
                Ok(Some(false))
            }
            GameAction::SelectMagicAbsorptionSlot { slot } => {
                let pending = self.validate_pending_magic_absorption()?;
                if *slot >= SLOTS_PER_CATEGORY || pending.replacement.is_some() {
                    return Err(CoreError::MagicAbsorptionUnavailable("slot-selection"));
                }
                Ok(Some(
                    self.absorbed_device(pending.category, *slot).is_none(),
                ))
            }
            GameAction::ResolveMagicAbsorption { confirm, .. } => {
                if !confirm {
                    return self
                        .pending_magic_absorption
                        .as_ref()
                        .map(|_| Some(false))
                        .ok_or(CoreError::MagicAbsorptionUnavailable(
                            "no-pending-absorption",
                        ));
                }
                let pending = self.validate_pending_magic_absorption()?;
                if pending.replacement.is_none() {
                    return Err(CoreError::MagicAbsorptionUnavailable(
                        "no-pending-replacement",
                    ));
                }
                Ok(Some(true))
            }
            GameAction::SwapAbsorbedDevices {
                first_slot,
                second_slot,
                ..
            } => {
                if !self.player_is_magic_eater()
                    || *first_slot >= SLOTS_PER_CATEGORY
                    || *second_slot >= SLOTS_PER_CATEGORY
                {
                    return Err(CoreError::MagicAbsorptionUnavailable("swap-slots"));
                }
                Ok(Some(false))
            }
            _ => Ok(None),
        }
    }

    pub(super) fn begin_magic_absorption(
        &mut self,
        item_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> Result<(), CoreError> {
        let category = self.magic_absorption_source(item_id)?;
        self.pending_magic_absorption = Some(PendingMagicAbsorptionDto {
            source_item_id: item_id.to_owned(),
            category,
            replacement: None,
        });
        events.push(DomainEvent::MagicAbsorptionPending {
            item_id: item_id.to_owned(),
        });
        Ok(())
    }

    pub(super) fn select_magic_absorption_slot(&mut self, slot: u8, events: &mut Vec<DomainEvent>) {
        let category = self
            .pending_magic_absorption
            .as_ref()
            .expect("preflight validated pending absorption")
            .category;
        if let Some(item) = self.absorbed_device(category, slot) {
            let replacement = MagicAbsorptionReplacementDto {
                slot,
                item_id: item.id.clone(),
            };
            let pending = self
                .pending_magic_absorption
                .as_mut()
                .expect("pending absorption");
            pending.replacement = Some(replacement);
            events.push(DomainEvent::MagicAbsorptionPending {
                item_id: pending.source_item_id.clone(),
            });
        } else {
            self.commit_magic_absorption(slot, false, events);
        }
    }

    pub(super) fn resolve_magic_absorption(
        &mut self,
        confirm: bool,
        inherit_inscription: bool,
        events: &mut Vec<DomainEvent>,
    ) {
        if !confirm {
            self.pending_magic_absorption = None;
            events.push(DomainEvent::MagicAbsorptionCancelled);
            return;
        }
        let slot = self
            .pending_magic_absorption
            .as_ref()
            .and_then(|pending| pending.replacement.as_ref())
            .expect("preflight validated replacement")
            .slot;
        self.commit_magic_absorption(slot, inherit_inscription, events);
    }

    fn commit_magic_absorption(
        &mut self,
        slot: u8,
        inherit_inscription: bool,
        events: &mut Vec<DomainEvent>,
    ) {
        let pending = self
            .pending_magic_absorption
            .take()
            .expect("preflight validated absorption");
        let old_inscription = inherit_inscription
            .then(|| {
                self.absorbed_device(pending.category, slot)
                    .and_then(|item| item.inscription.clone())
            })
            .flatten();
        let replaced_item_id = pending.replacement.map(|replacement| replacement.item_id);
        if let Some(old_id) = &replaced_item_id {
            self.items.retain(|item| &item.id != old_id);
            self.item_property_knowledge.remove(old_id);
        }
        self.identify_item_instance(
            &pending.source_item_id,
            ItemIdentificationRequest::new(true),
        );
        let item = self
            .items
            .iter_mut()
            .find(|item| item.id == pending.source_item_id)
            .expect("preflight validated source");
        item.location = ItemLocation::Absorbed {
            category: pending.category,
            slot,
        };
        if let Some(inscription) = old_inscription {
            item.inscription = Some(inscription);
        }
        events.push(DomainEvent::MagicDeviceAbsorbed {
            item_id: item.id.clone(),
            kind_id: item.kind_id.clone(),
            slot,
            replaced_item_id,
        });
    }

    pub(super) fn swap_absorbed_devices(
        &mut self,
        category: AbsorbedDeviceCategoryDto,
        first_slot: u8,
        second_slot: u8,
        events: &mut Vec<DomainEvent>,
    ) {
        for item in &mut self.items {
            if let ItemLocation::Absorbed {
                category: item_category,
                slot,
            } = &mut item.location
                && *item_category == category
            {
                if *slot == first_slot {
                    *slot = second_slot;
                } else if *slot == second_slot {
                    *slot = first_slot;
                }
            }
        }
        events.push(DomainEvent::AbsorbedDevicesSwapped {
            first_slot,
            second_slot,
        });
    }

    pub(super) fn magic_eater_dto(&self) -> Option<MagicEaterDto> {
        self.player_is_magic_eater().then(|| MagicEaterDto {
            slots: CATEGORIES
                .iter()
                .flat_map(|(category, _)| {
                    (0..SLOTS_PER_CATEGORY)
                        .map(move |slot| self.absorbed_device_slot_dto(*category, slot))
                })
                .collect(),
            pending_absorption: self.pending_magic_absorption.clone(),
            device_commands: CATEGORIES
                .iter()
                .map(|(category, _)| {
                    let mut items = self
                        .items
                        .iter()
                        .filter(|item| {
                            self.item_is_in_pack_or_at_feet(item)
                                && self.absorbed_device_category(item) == Some(*category)
                        })
                        .map(|item| self.inventory_item_dto(item))
                        .collect::<Vec<_>>();
                    items.sort_by(|left, right| left.id.cmp(&right.id));
                    rfb_protocol::DeviceCommandDto {
                        category: *category,
                        items,
                    }
                })
                .collect(),
        })
    }

    // RFB magic_eater.c::_magic_eater_calculate_labels and obj.c::obj_label.
    fn absorbed_device_labels(
        &self,
        category: AbsorbedDeviceCategoryDto,
        command: u8,
    ) -> [char; 10] {
        let mut labels = std::array::from_fn(|slot| char::from(b'a' + slot as u8));
        for slot in 0..SLOTS_PER_CATEGORY {
            let Some(inscription) = self
                .absorbed_device(category, slot)
                .and_then(|item| item.inscription.as_deref())
            else {
                continue;
            };
            let bytes = inscription.as_bytes();
            let label = bytes
                .iter()
                .enumerate()
                .filter(|(_, byte)| **byte == b'@')
                .find_map(|(index, _)| {
                    let next = *bytes.get(index + 1)?;
                    if next == command {
                        bytes
                            .get(index + 2)
                            .copied()
                            .filter(u8::is_ascii_alphanumeric)
                    } else {
                        next.is_ascii_digit().then_some(next)
                    }
                });
            if let Some(label) = label {
                let label = char::from(if b"XWSRZ".contains(&label) {
                    label.to_ascii_lowercase()
                } else {
                    label
                });
                if let Some(previous) = labels.iter().position(|value| *value == label) {
                    labels[previous] = ' ';
                }
                labels[usize::from(slot)] = label;
            }
        }
        for slot in 0..labels.len() {
            if labels[slot] == ' ' {
                labels[slot] = ('a'..='z')
                    .find(|label| !labels.contains(label))
                    .expect("ten slots fit alphabet");
            }
        }
        labels
    }

    pub(super) fn magic_absorption_item_targets(&self) -> Vec<rfb_protocol::AbilityItemTargetDto> {
        let mut targets = self
            .items
            .iter()
            .filter(|item| self.magic_absorption_source(&item.id).is_ok())
            .map(|item| rfb_protocol::AbilityItemTargetDto {
                item_id: item.id.clone(),
                target: rfb_protocol::TargetSelection::Item {
                    item_id: item.id.clone(),
                },
                confirmation_key: None,
            })
            .collect::<Vec<_>>();
        targets.sort_by(|left, right| left.item_id.cmp(&right.item_id));
        targets
    }

    pub(super) fn magic_eater_state_is_valid(&self) -> bool {
        let mut occupied = BTreeSet::new();
        self.items.iter().all(|item| {
            let ItemLocation::Absorbed { category, slot } = item.location else {
                return true;
            };
            self.player_is_magic_eater()
                && slot < SLOTS_PER_CATEGORY
                && item.quantity == 1
                && occupied.insert((category, slot))
                && self.absorbed_device_category(item) == Some(category)
                && self
                    .item_property_knowledge
                    .get(&item.id)
                    .is_some_and(|knowledge| {
                        knowledge.identified
                            && item
                                .affix_ids
                                .iter()
                                .chain(item.rolled_affixes.iter().map(|affix| &affix.affix_id))
                                .all(|id| knowledge.known_affix_ids.contains(id))
                    })
        }) && (self.pending_magic_absorption.is_none()
            || (self.validate_pending_magic_absorption().is_ok()
                && self.pending_realm_change_book().is_none()
                && self.pending_ability_direction.is_none()
                && self.pending_mutation_direction.is_none()
                && self.pending_duelist.is_none()
                && self.casino.is_none()
                && self.pending_race_mutation_choice().is_none()))
    }
}
