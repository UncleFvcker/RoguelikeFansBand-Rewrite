// SPDX-License-Identifier: MPL-2.0
use std::collections::{BTreeMap, BTreeSet};

use rfb_content::ContentCatalog;
use rfb_protocol::{
    ItemCurseSeverityDto, ItemEnchantmentsDto, ItemKnowledgeDto, ItemQualityDto, Position,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ItemEnchantmentRequest {
    to_hit_attempts: u16,
    to_damage_attempts: u16,
    to_armor_attempts: u16,
}

impl ItemEnchantmentRequest {
    pub(super) const fn new(
        to_hit_attempts: u16,
        to_damage_attempts: u16,
        to_armor_attempts: u16,
    ) -> Self {
        Self {
            to_hit_attempts,
            to_damage_attempts,
            to_armor_attempts,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ItemEnchantmentComponentOutcome {
    pub(super) attempts: u16,
    pub(super) successes: u16,
    pub(super) before: u16,
    pub(super) after: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ItemEnchantmentOutcome {
    pub(super) item_id: String,
    pub(super) item_kind_id: String,
    pub(super) to_hit: ItemEnchantmentComponentOutcome,
    pub(super) to_damage: ItemEnchantmentComponentOutcome,
    pub(super) to_armor: ItemEnchantmentComponentOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EquippedItemCurseTarget {
    Weapon,
    Armor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CurseEquippedItemRequest {
    target: EquippedItemCurseTarget,
}

impl CurseEquippedItemRequest {
    pub(super) const fn new(target: EquippedItemCurseTarget) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CurseEquippedItemOutcome {
    pub(super) item_id: Option<String>,
    pub(super) item_kind_id: Option<String>,
    pub(super) before: Option<ItemCurseSeverityDto>,
    pub(super) after: Option<ItemCurseSeverityDto>,
    pub(super) resisted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RemoveEquippedCursesRequest {
    include_heavy: bool,
}

impl RemoveEquippedCursesRequest {
    pub(super) const fn new(include_heavy: bool) -> Self {
        Self { include_heavy }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RemoveEquippedCursesOutcome {
    pub(super) include_heavy: bool,
    pub(super) removed_item_ids: Vec<String>,
    pub(super) retained_permanent_item_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct InventoryItemRechargeRequest {
    attempted: u32,
    power: u32,
}

impl InventoryItemRechargeRequest {
    pub(super) const fn new(attempted: u32, power: u32) -> Self {
        Self { attempted, power }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DeviceRechargeRequest {
    power: u32,
    source_destruction_one_in: u16,
}

impl DeviceRechargeRequest {
    pub(super) const fn new(power: u32, source_destruction_one_in: u16) -> Self {
        Self {
            power,
            source_destruction_one_in,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct InventoryItemRechargeOutcome {
    pub(super) target_item_id: String,
    pub(super) target_kind_id: String,
    pub(super) attempted: u32,
    pub(super) target_before: u32,
    pub(super) target_after: u32,
    pub(super) succeeded: bool,
    pub(super) failure_one_in: u32,
    pub(super) failure_roll: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeviceRechargeOutcome {
    pub(super) source_kind_id: String,
    pub(super) source_destroyed: bool,
    pub(super) target: InventoryItemRechargeOutcome,
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

    pub(super) fn enchant_item_instance(
        &mut self,
        item_id: &str,
        request: ItemEnchantmentRequest,
    ) -> ItemEnchantmentOutcome {
        let item = self
            .items
            .iter()
            .find(|item| item.id == item_id)
            .expect("planned enchantment target must remain available");
        let item_kind_id = item.kind_id.clone();
        let quantity = item.quantity;
        let definition = self
            .content
            .item(&item_kind_id)
            .expect("planned enchantment kind must remain available");
        let artifact = definition.tags.iter().any(|tag| tag == "artifact");
        let ammunition = definition.tags.iter().any(|tag| tag == "ammunition");
        let before = item.enchantments;

        let to_hit = self.resolve_item_enchantment_component(
            before.to_hit,
            request.to_hit_attempts,
            quantity,
            ammunition,
            artifact,
        );
        let to_damage = self.resolve_item_enchantment_component(
            before.to_damage,
            request.to_damage_attempts,
            quantity,
            ammunition,
            artifact,
        );
        let to_armor = self.resolve_item_enchantment_component(
            before.to_armor,
            request.to_armor_attempts,
            quantity,
            ammunition,
            artifact,
        );
        self.items
            .iter_mut()
            .find(|item| item.id == item_id)
            .expect("planned enchantment target must remain available")
            .enchantments = ItemEnchantmentsDto {
            to_hit: to_hit.after,
            to_damage: to_damage.after,
            to_armor: to_armor.after,
        };
        ItemEnchantmentOutcome {
            item_id: item_id.to_owned(),
            item_kind_id,
            to_hit,
            to_damage,
            to_armor,
        }
    }

    pub(super) fn resolve_item_enchantment_component(
        &mut self,
        before: u16,
        attempts: u16,
        quantity: u32,
        ammunition: bool,
        artifact: bool,
    ) -> ItemEnchantmentComponentOutcome {
        const FAILURE_PER_THOUSAND: [u16; 16] = [
            5, 10, 50, 100, 200, 300, 400, 500, 650, 800, 950, 987, 993, 995, 998, 1000,
        ];
        let mut after = before;
        let pile_probability = if ammunition {
            u64::from(quantity).saturating_mul(100) / 20
        } else {
            u64::from(quantity).saturating_mul(100)
        }
        .max(1);
        for _ in 0..attempts {
            if self.rng.bounded(pile_probability) >= 100 {
                continue;
            }
            let failure = FAILURE_PER_THOUSAND
                .get(usize::from(after))
                .copied()
                .unwrap_or(1000);
            if self.rng.bounded(1000).saturating_add(1) <= u64::from(failure) {
                continue;
            }
            if artifact && self.rng.bounded(100) >= 50 {
                continue;
            }
            after = after.saturating_add(1).min(15);
        }
        ItemEnchantmentComponentOutcome {
            attempts,
            successes: after.saturating_sub(before),
            before,
            after,
        }
    }

    pub(super) fn curse_equipped_item(
        &mut self,
        request: CurseEquippedItemRequest,
    ) -> CurseEquippedItemOutcome {
        let mut candidates = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let ItemLocation::Equipped { slot_id } = &item.location else {
                    return None;
                };
                let definition = self.content.item(&item.kind_id)?;
                let matches_target = match request.target {
                    EquippedItemCurseTarget::Weapon => {
                        definition.tags.iter().any(|tag| tag == "weapon")
                    }
                    EquippedItemCurseTarget::Armor => {
                        definition.tags.iter().any(|tag| tag == "armor")
                    }
                };
                matches_target.then(|| (slot_id.clone(), item.id.clone(), index))
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
        if candidates.is_empty() {
            return CurseEquippedItemOutcome {
                item_id: None,
                item_kind_id: None,
                before: None,
                after: None,
                resisted: false,
            };
        }
        let candidate_index = if candidates.len() == 1 {
            0
        } else {
            usize::try_from(self.rng.bounded(candidates.len() as u64))
                .expect("curse target index must fit usize")
        };
        let item_index = candidates[candidate_index].2;
        let item_id = self.items[item_index].id.clone();
        let item_kind_id = self.items[item_index].kind_id.clone();
        let artifact = self
            .content
            .item(&item_kind_id)
            .is_some_and(|definition| definition.tags.iter().any(|tag| tag == "artifact"));
        let resisted = artifact
            && if self.debug_item_curses_land {
                false
            } else if self.debug_item_curses_resisted {
                true
            } else {
                self.rng.bounded(100) < 50
            };
        let before = self.items[item_index].curse;
        if !resisted {
            self.items[item_index].curse =
                Some(before.map_or(ItemCurseSeverityDto::Normal, |severity| {
                    severity.max(ItemCurseSeverityDto::Normal)
                }));
        }
        CurseEquippedItemOutcome {
            item_id: Some(item_id),
            item_kind_id: Some(item_kind_id),
            before,
            after: self.items[item_index].curse,
            resisted,
        }
    }

    pub(super) fn remove_equipped_curses(
        &mut self,
        request: RemoveEquippedCursesRequest,
    ) -> RemoveEquippedCursesOutcome {
        let mut removed_item_ids = Vec::new();
        let mut retained_permanent_item_ids = Vec::new();
        for item in &mut self.items {
            if !matches!(item.location, ItemLocation::Equipped { .. }) {
                continue;
            }
            match item.curse {
                Some(ItemCurseSeverityDto::Normal) => {
                    item.curse = None;
                    removed_item_ids.push(item.id.clone());
                }
                Some(ItemCurseSeverityDto::Heavy) if request.include_heavy => {
                    item.curse = None;
                    removed_item_ids.push(item.id.clone());
                }
                Some(ItemCurseSeverityDto::Permanent) => {
                    retained_permanent_item_ids.push(item.id.clone());
                }
                Some(ItemCurseSeverityDto::Heavy) | None => {}
            }
        }
        removed_item_ids.sort();
        retained_permanent_item_ids.sort();
        RemoveEquippedCursesOutcome {
            include_heavy: request.include_heavy,
            removed_item_ids,
            retained_permanent_item_ids,
        }
    }

    pub(super) fn item_can_receive_recharge(&self, item: &ItemInstance) -> bool {
        item.location == ItemLocation::Inventory
            && item.activation.is_some()
            && self
                .content
                .item(&item.kind_id)
                .is_some_and(|definition| definition.device_generation.is_some())
            && item
                .charges
                .is_some_and(|charges| charges.current < charges.maximum)
    }

    pub(super) fn item_can_supply_recharge(&self, item: &ItemInstance) -> bool {
        item.location == ItemLocation::Inventory
            && self
                .content
                .item(&item.kind_id)
                .is_some_and(|definition| definition.tags.iter().any(|tag| tag == "device"))
            && item.charges.is_some_and(|charges| charges.current > 0)
    }

    pub(super) fn recharge_inventory_item_from_resource(
        &mut self,
        target_item_id: &str,
        request: InventoryItemRechargeRequest,
    ) -> InventoryItemRechargeOutcome {
        self.recharge_inventory_item_target(target_item_id, request, true)
    }

    pub(super) fn recharge_inventory_item_from_device(
        &mut self,
        target_item_id: &str,
        source_item_id: &str,
        request: DeviceRechargeRequest,
    ) -> DeviceRechargeOutcome {
        let target_charges = self
            .items
            .iter()
            .find(|item| item.id == target_item_id)
            .and_then(|item| item.charges)
            .expect("preflighted recharge target must carry energy");
        let missing = target_charges
            .maximum
            .saturating_sub(target_charges.current);
        let source_index = self
            .items
            .iter()
            .position(|item| item.id == source_item_id)
            .expect("preflighted recharge source must remain available");
        let source_kind_id = self.items[source_index].kind_id.clone();
        let source_current = self.items[source_index]
            .charges
            .expect("recharge source must carry energy")
            .current;
        let attempted = request.power.min(source_current).min(missing);
        let destruction_roll = (!self.debug_recharge_sources_survive).then(|| {
            self.rng
                .bounded(u64::from(request.source_destruction_one_in))
        });
        let destroy = destruction_roll == Some(0);
        let artifact = self
            .content
            .item(&source_kind_id)
            .is_some_and(|definition| definition.tags.iter().any(|tag| tag == "artifact"));
        let source_destroyed = destroy && !artifact;
        if source_destroyed {
            let removed = self.items.remove(source_index);
            self.item_property_knowledge.remove(&removed.id);
        } else {
            let source = self
                .items
                .iter_mut()
                .find(|item| item.id == source_item_id)
                .expect("surviving recharge source must remain available");
            source
                .charges
                .as_mut()
                .expect("recharge source must carry energy")
                .current -= attempted;
        }
        let target = self.recharge_inventory_item_target(
            target_item_id,
            InventoryItemRechargeRequest::new(attempted, request.power),
            false,
        );
        DeviceRechargeOutcome {
            source_kind_id,
            source_destroyed,
            target,
        }
    }

    fn recharge_inventory_item_target(
        &mut self,
        target_item_id: &str,
        request: InventoryItemRechargeRequest,
        deplete_on_failure: bool,
    ) -> InventoryItemRechargeOutcome {
        let target = self
            .items
            .iter()
            .find(|item| item.id == target_item_id)
            .expect("preflighted recharge target must remain available");
        let target_kind_id = target.kind_id.clone();
        let target_before = target
            .charges
            .expect("recharge target must carry energy")
            .current;
        let difficulty = u32::try_from(
            target
                .activation
                .as_ref()
                .expect("recharge target must retain dynamic activation")
                .device_check_difficulty,
        )
        .expect("validated device difficulty must be positive");
        let half_difficulty = difficulty / 2;
        let failure_one_in = request.power.saturating_sub(half_difficulty) / 15;
        let (failure_roll, succeeded) = if self.debug_recharge_attempts_succeed {
            (None, true)
        } else if self.debug_recharge_attempts_fail
            || request.power <= half_difficulty
            || failure_one_in == 0
        {
            (None, false)
        } else {
            let roll = u32::try_from(self.rng.bounded(u64::from(failure_one_in)))
                .expect("recharge failure roll must fit u32");
            (Some(roll), roll != 0)
        };

        let target = self
            .items
            .iter_mut()
            .find(|item| item.id == target_item_id)
            .expect("recharge target must remain available");
        let charges = target
            .charges
            .as_mut()
            .expect("recharge target must carry energy");
        if succeeded {
            charges.current = charges
                .current
                .saturating_add(request.attempted)
                .min(charges.maximum);
            if charges.current == charges.maximum {
                target.device_recovery_progress = 0;
            }
        } else if deplete_on_failure {
            charges.current = 0;
            target.device_recovery_progress = 0;
        }
        InventoryItemRechargeOutcome {
            target_item_id: target_item_id.to_owned(),
            target_kind_id,
            attempted: request.attempted,
            target_before,
            target_after: charges.current,
            succeeded,
            failure_one_in,
            failure_roll,
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
