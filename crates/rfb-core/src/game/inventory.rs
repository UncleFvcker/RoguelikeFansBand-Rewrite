// SPDX-License-Identifier: MPL-2.0
use std::collections::{BTreeMap, BTreeSet};

use rfb_content::ContentCatalog;
use rfb_protocol::{ItemCurseSeverityDto, ItemKnowledgeDto, ItemQualityDto, Position};

use crate::{
    error::CoreError,
    state::{EquipOutcome, ItemInstance, ItemLocation},
};

use super::{BodySlot, Game, body_slot_instance_for_type};

#[derive(Debug, Clone, Default)]
pub(super) struct ItemKnowledgeState {
    pub(super) tried: bool,
    pub(super) aware: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ItemPropertyKnowledgeState {
    pub(super) appraised: bool,
    pub(super) identified: bool,
    pub(super) known_affix_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ItemIdentificationRequest {
    full: bool,
}

impl ItemIdentificationRequest {
    pub(super) const fn new(full: bool) -> Self {
        Self { full }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ItemIdentificationOutcome {
    pub(super) item_id: String,
    pub(super) item_kind_id: String,
    pub(super) full: bool,
    pub(super) changed: bool,
}

pub(super) enum PickUpOutcome {
    Picked {
        kind_id: String,
        quantity: u32,
    },
    OverCapacity {
        kind_id: String,
        quantity: u32,
        current_weight: u32,
        pickup_weight: u32,
        capacity: u32,
    },
    Nothing,
}

struct BatchDropPlan {
    item_indices: Vec<usize>,
    quantity: u64,
}

struct DropQuantityPlan {
    item_index: usize,
    quantity: u32,
    split_stack: bool,
}

struct EquipPlan {
    inventory_index: usize,
    slot_id: String,
    replaced_index: Option<usize>,
}

struct UnequipPlan {
    item_index: usize,
    kind_id: String,
    curse: Option<ItemCurseSeverityDto>,
}

struct PickUpCommitPlan {
    ground_index: usize,
    kind_id: String,
    original_quantity: u32,
    stack_transfers: Vec<(usize, u32)>,
    remaining: u32,
}

enum PickUpPlan {
    Picked(PickUpCommitPlan),
    OverCapacity {
        kind_id: String,
        quantity: u32,
        current_weight: u32,
        pickup_weight: u32,
        capacity: u32,
    },
    Nothing,
}

fn plan_batch_drop(items: &[ItemInstance], item_ids: &[String]) -> Option<BatchDropPlan> {
    let selected = item_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
    if selected.is_empty() {
        return None;
    }
    let item_indices = items
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            item.location == ItemLocation::Inventory && selected.contains(item.id.as_str())
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if item_indices.is_empty() {
        return None;
    }
    let quantity = item_indices.iter().fold(0_u64, |quantity, index| {
        quantity.saturating_add(u64::from(items[*index].quantity))
    });
    Some(BatchDropPlan {
        item_indices,
        quantity,
    })
}

fn plan_drop_quantity(
    items: &[ItemInstance],
    item_id: &str,
    quantity: u32,
) -> Option<DropQuantityPlan> {
    let item_index = items
        .iter()
        .position(|item| item.id == item_id && item.location == ItemLocation::Inventory)?;
    if quantity == 0 || quantity > items[item_index].quantity {
        return None;
    }
    Some(DropQuantityPlan {
        item_index,
        quantity,
        split_stack: quantity != items[item_index].quantity,
    })
}

fn plan_equip(
    content: &ContentCatalog,
    body_slots: &[BodySlot],
    items: &[ItemInstance],
    item_id: &str,
) -> Option<EquipPlan> {
    let inventory_index = items
        .iter()
        .position(|item| item.id == item_id && item.location == ItemLocation::Inventory)?;
    let carried = &items[inventory_index];
    let slot_type = content.item(&carried.kind_id)?.equipment_slot.as_ref()?;
    let slot_id = body_slot_instance_for_type(body_slots, slot_type, |slot_id| {
        items.iter().any(|item| {
            matches!(
                &item.location,
                ItemLocation::Equipped { slot_id: equipped } if equipped == slot_id
            )
        })
    })?
    .id
    .clone();
    if carried.quantity != 1 {
        return None;
    }
    let replaced_index = items.iter().position(|equipped| {
        matches!(
            &equipped.location,
            ItemLocation::Equipped { slot_id: equipped_slot } if equipped_slot == &slot_id
        )
    });
    Some(EquipPlan {
        inventory_index,
        slot_id,
        replaced_index,
    })
}

fn plan_unequip(items: &[ItemInstance], slot_id: &str) -> Option<UnequipPlan> {
    let item_index = items.iter().position(|item| {
        matches!(
            &item.location,
            ItemLocation::Equipped { slot_id: equipped_slot } if equipped_slot == slot_id
        )
    })?;
    Some(UnequipPlan {
        item_index,
        kind_id: items[item_index].kind_id.clone(),
        curse: items[item_index].curse,
    })
}

fn plan_pick_up(
    content: &ContentCatalog,
    items: &[ItemInstance],
    item_property_knowledge: &BTreeMap<String, ItemPropertyKnowledgeState>,
    player_position: Position,
    player_kind_id: &str,
    current_weight: u32,
) -> Result<PickUpPlan, CoreError> {
    let Some(ground_index) = items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.location == ItemLocation::Ground(player_position))
        .min_by(|(_, left), (_, right)| left.id.cmp(&right.id))
        .map(|(index, _)| index)
    else {
        return Ok(PickUpPlan::Nothing);
    };

    let pickup_item = &items[ground_index];
    let kind_id = pickup_item.kind_id.clone();
    let definition = content
        .item(&kind_id)
        .ok_or_else(|| CoreError::UnknownItem(kind_id.clone()))?;
    let original_quantity = pickup_item.quantity;
    let pickup_weight = u32::from(definition.weight_tenths_pound).saturating_mul(original_quantity);
    let capacity = content
        .actor(player_kind_id)
        .expect("player actor definition must remain available")
        .carry_capacity_tenths_pound;
    if current_weight.saturating_add(pickup_weight) > capacity {
        return Ok(PickUpPlan::OverCapacity {
            kind_id,
            quantity: original_quantity,
            current_weight,
            pickup_weight,
            capacity,
        });
    }

    let pickup_knowledge = item_property_knowledge.get(&pickup_item.id);
    let mut stack_indices = items
        .iter()
        .enumerate()
        .filter(|(_, carried)| {
            carried.location == ItemLocation::Inventory
                && carried.kind_id == kind_id
                && carried.quantity < definition.max_stack
                && item_instances_stack_compatible(carried, pickup_item)
                && item_property_knowledge.get(&carried.id) == pickup_knowledge
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    stack_indices.sort_by(|left, right| items[*left].id.cmp(&items[*right].id));

    let mut remaining = original_quantity;
    let mut stack_transfers = Vec::new();
    for stack_index in stack_indices {
        let transferred = remaining.min(definition.max_stack - items[stack_index].quantity);
        stack_transfers.push((stack_index, transferred));
        remaining -= transferred;
        if remaining == 0 {
            break;
        }
    }
    Ok(PickUpPlan::Picked(PickUpCommitPlan {
        ground_index,
        kind_id,
        original_quantity,
        stack_transfers,
        remaining,
    }))
}

fn item_instances_stack_compatible(left: &ItemInstance, right: &ItemInstance) -> bool {
    left.kind_id == right.kind_id
        && left.quality == right.quality
        && left.affix_ids == right.affix_ids
        && left.rolled_affixes == right.rolled_affixes
        && left.enchantments == right.enchantments
        && left.curse == right.curse
        && left.activation == right.activation
        && left.charges == right.charges
        && left.device_recovery_progress == right.device_recovery_progress
}

impl Game {
    pub(super) fn drop_inventory_items(&mut self, item_ids: &[String]) -> Option<(usize, u64)> {
        let plan = plan_batch_drop(&self.items, item_ids)?;
        for index in &plan.item_indices {
            self.items[*index].location = ItemLocation::Ground(self.player.position);
        }
        Some((plan.item_indices.len(), plan.quantity))
    }

    pub(super) fn appraise_inventory_item(
        &mut self,
        item_id: &str,
    ) -> Option<(String, ItemQualityDto)> {
        let item = self
            .items
            .iter()
            .find(|item| item.id == item_id && item.location == ItemLocation::Inventory)?;
        let item_instance_id = item.id.clone();
        let kind_id = item.kind_id.clone();
        let quality = item.quality;
        let knowledge = self.item_property_knowledge.get(&item_instance_id);
        if knowledge.is_some_and(|knowledge| knowledge.appraised || knowledge.identified) {
            return None;
        }
        self.item_property_knowledge
            .entry(item_instance_id)
            .or_default()
            .appraised = true;
        Some((kind_id, quality))
    }

    pub(super) fn identify_item_instance(
        &mut self,
        item_id: &str,
        request: ItemIdentificationRequest,
    ) -> ItemIdentificationOutcome {
        let item = self
            .items
            .iter()
            .find(|item| item.id == item_id)
            .expect("planned identify target must remain available");
        let item_kind_id = item.kind_id.clone();
        let affix_ids = item
            .affix_ids
            .iter()
            .cloned()
            .chain(
                item.rolled_affixes
                    .iter()
                    .map(|rolled| rolled.affix_id.clone()),
            )
            .collect::<BTreeSet<_>>();
        let awareness_before = self.item_knowledge_dto(&item_kind_id);
        let property_before = self.item_property_knowledge.get(item_id).cloned();
        self.mark_item_aware(&item_kind_id);
        let knowledge = self
            .item_property_knowledge
            .entry(item_id.to_owned())
            .or_default();
        knowledge.appraised = true;
        if request.full {
            knowledge.identified = true;
            knowledge.known_affix_ids.extend(affix_ids);
        }
        let changed = awareness_before != self.item_knowledge_dto(&item_kind_id)
            || property_before.as_ref() != self.item_property_knowledge.get(item_id);
        ItemIdentificationOutcome {
            item_id: item_id.to_owned(),
            item_kind_id,
            full: request.full,
            changed,
        }
    }

    pub(super) fn drop_inventory_quantity(
        &mut self,
        item_id: &str,
        quantity: u32,
    ) -> Result<Option<(usize, u64)>, CoreError> {
        let Some(plan) = plan_drop_quantity(&self.items, item_id, quantity) else {
            return Ok(None);
        };
        if !plan.split_stack {
            self.items[plan.item_index].location = ItemLocation::Ground(self.player.position);
        } else {
            let id = self.allocate_item_instance_id()?;
            let mut split = self.items[plan.item_index].clone();
            let knowledge = self.item_property_knowledge.get(&split.id).cloned();
            self.items[plan.item_index].quantity -= plan.quantity;
            split.id = id.clone();
            split.quantity = plan.quantity;
            split.location = ItemLocation::Ground(self.player.position);
            self.items.push(split);
            if let Some(knowledge) = knowledge {
                self.item_property_knowledge.insert(id, knowledge);
            }
        }
        Ok(Some((1, u64::from(plan.quantity))))
    }

    pub(super) fn equip_inventory_item(&mut self, item_id: &str) -> Option<EquipOutcome> {
        let plan = plan_equip(&self.content, &self.body_slots, &self.items, item_id)?;
        if plan
            .replaced_index
            .is_some_and(|index| self.items[index].curse.is_some())
        {
            return None;
        }
        let replaced_kind_id = plan.replaced_index.map(|index| {
            let kind_id = self.items[index].kind_id.clone();
            self.items[index].location = ItemLocation::Inventory;
            kind_id
        });
        let kind_id = self.items[plan.inventory_index].kind_id.clone();
        let item_instance_id = self.items[plan.inventory_index].id.clone();
        let affix_ids = self.items[plan.inventory_index].affix_ids.clone();
        self.items[plan.inventory_index].location = ItemLocation::Equipped {
            slot_id: plan.slot_id.clone(),
        };
        self.clamp_player_hp_to_effective_max();
        let knowledge = self
            .item_property_knowledge
            .entry(item_instance_id)
            .or_default();
        knowledge.appraised = true;
        knowledge.identified = true;
        let discovered_affix_ids = affix_ids
            .into_iter()
            .filter(|affix_id| knowledge.known_affix_ids.insert(affix_id.clone()))
            .collect();
        Some(EquipOutcome {
            kind_id,
            slot_id: plan.slot_id,
            replaced_kind_id,
            discovered_affix_ids,
        })
    }

    pub(super) fn cursed_equipment_replaced_by(
        &self,
        item_id: &str,
    ) -> Option<(String, String, ItemCurseSeverityDto)> {
        let plan = plan_equip(&self.content, &self.body_slots, &self.items, item_id)?;
        let replaced = &self.items[plan.replaced_index?];
        Some((replaced.kind_id.clone(), plan.slot_id, replaced.curse?))
    }

    pub(super) fn unequip_slot(&mut self, slot_id: &str) -> Option<String> {
        let plan = plan_unequip(&self.items, slot_id)?;
        if plan.curse.is_some() {
            return None;
        }
        self.items[plan.item_index].location = ItemLocation::Inventory;
        self.clamp_player_hp_to_effective_max();
        Some(plan.kind_id)
    }

    pub(super) fn cursed_equipment_in_slot(
        &self,
        slot_id: &str,
    ) -> Option<(String, ItemCurseSeverityDto)> {
        let plan = plan_unequip(&self.items, slot_id)?;
        Some((plan.kind_id, plan.curse?))
    }

    pub(super) fn pick_up_at_player(&mut self) -> Result<PickUpOutcome, CoreError> {
        let plan = plan_pick_up(
            &self.content,
            &self.items,
            &self.item_property_knowledge,
            self.player.position,
            &self.player.kind_id,
            self.carried_weight_tenths_pound(),
        )?;
        match plan {
            PickUpPlan::Nothing => Ok(PickUpOutcome::Nothing),
            PickUpPlan::OverCapacity {
                kind_id,
                quantity,
                current_weight,
                pickup_weight,
                capacity,
            } => Ok(PickUpOutcome::OverCapacity {
                kind_id,
                quantity,
                current_weight,
                pickup_weight,
                capacity,
            }),
            PickUpPlan::Picked(plan) => {
                for (stack_index, transferred) in plan.stack_transfers {
                    self.items[stack_index].quantity += transferred;
                }
                if plan.remaining == 0 {
                    let removed = self.items.remove(plan.ground_index);
                    self.item_property_knowledge.remove(&removed.id);
                } else {
                    self.items[plan.ground_index].quantity = plan.remaining;
                    self.items[plan.ground_index].location = ItemLocation::Inventory;
                }
                Ok(PickUpOutcome::Picked {
                    kind_id: plan.kind_id,
                    quantity: plan.original_quantity,
                })
            }
        }
    }

    pub(super) fn item_knowledge_dto(&self, kind_id: &str) -> ItemKnowledgeDto {
        let Some(definition) = self.content.item(kind_id) else {
            return ItemKnowledgeDto::Unknown;
        };
        if definition.appearance_name_key.is_none() {
            return ItemKnowledgeDto::Aware;
        }
        self.item_knowledge
            .get(kind_id)
            .map_or(ItemKnowledgeDto::Unknown, |knowledge| {
                if knowledge.aware {
                    ItemKnowledgeDto::Aware
                } else if knowledge.tried {
                    ItemKnowledgeDto::Tried
                } else {
                    ItemKnowledgeDto::Unknown
                }
            })
    }

    pub(super) fn item_display_name_key(&self, kind_id: &str) -> String {
        let Some(definition) = self.content.item(kind_id) else {
            return "item-unknown-name".to_owned();
        };
        if self.item_knowledge_dto(kind_id) == ItemKnowledgeDto::Aware {
            definition.name_key.clone()
        } else {
            definition
                .appearance_name_key
                .clone()
                .unwrap_or_else(|| definition.name_key.clone())
        }
    }

    pub(super) fn mark_item_tried(&mut self, kind_id: &str) {
        if self
            .content
            .item(kind_id)
            .is_some_and(|definition| definition.appearance_name_key.is_some())
        {
            self.item_knowledge
                .entry(kind_id.to_owned())
                .or_default()
                .tried = true;
        }
    }

    pub(super) fn mark_item_aware(&mut self, kind_id: &str) {
        if self
            .content
            .item(kind_id)
            .is_some_and(|definition| definition.appearance_name_key.is_some())
        {
            let knowledge = self.item_knowledge.entry(kind_id.to_owned()).or_default();
            knowledge.tried = true;
            knowledge.aware = true;
        }
    }
}
