// SPDX-License-Identifier: MPL-2.0
use std::collections::{BTreeMap, BTreeSet};

use rfb_content::{ContentCatalog, ItemDestructionElement, TaskObjectiveKind};
use rfb_protocol::{
    ItemCurseSeverityDto, ItemEnchantmentsDto, ItemKnowledgeDto, ItemQualityDto, Position,
};

use crate::{
    error::CoreError,
    event::DomainEvent,
    resistance::{DamageType, ResistanceLevel},
    state::{EquipOutcome, ItemInstance, ItemLocation},
};

use super::{
    BodySlot, Game, STATUS_INVENTORY_PROTECTION, body_slot_instance_for_type,
    item_can_occupy_slot_type, item_device_generation,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ItemKnowledgeState {
    pub(super) tried: bool,
    pub(super) aware: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ItemPropertyKnowledgeState {
    pub(super) discovered: bool,
    pub(super) appraised: bool,
    pub(super) identified: bool,
    pub(super) known_affix_ids: BTreeSet<String>,
}

pub(super) fn item_properties_match(
    left: Option<&ItemPropertyKnowledgeState>,
    right: Option<&ItemPropertyKnowledgeState>,
) -> bool {
    let empty = ItemPropertyKnowledgeState::default();
    let left = left.unwrap_or(&empty);
    let right = right.unwrap_or(&empty);
    left.appraised == right.appraised
        && left.identified == right.identified
        && left.known_affix_ids == right.known_affix_ids
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
    pub(super) before: i16,
    pub(super) after: i16,
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
    Any,
    Weapon,
    Armor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CurseEquippedItemRequest {
    target: EquippedItemCurseTarget,
    heavy_chance_percent: u8,
}

impl CurseEquippedItemRequest {
    pub(super) const fn new(target: EquippedItemCurseTarget) -> Self {
        Self {
            target,
            heavy_chance_percent: 0,
        }
    }

    pub(super) const fn with_heavy_chance(mut self, chance_percent: u8) -> Self {
        self.heavy_chance_percent = chance_percent;
        self
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
    drain_on_failure: bool,
}

impl InventoryItemRechargeRequest {
    pub(super) const fn new(attempted: u32, power: u32) -> Self {
        Self {
            attempted,
            power,
            drain_on_failure: false,
        }
    }

    pub(super) const fn from_player(attempted: u32, power: u32) -> Self {
        Self {
            attempted,
            power,
            drain_on_failure: true,
        }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeviceAbsorptionOutcome {
    pub(super) item_id: String,
    pub(super) item_kind_id: String,
    pub(super) charges_before: u32,
    pub(super) charges_after: u32,
    pub(super) drained: u32,
    pub(super) nutrition_before: u16,
    pub(super) nutrition_after: u16,
}

pub(super) enum PickUpOutcome {
    Picked {
        kind_id: String,
        quantity: u32,
    },
    InventoryFull {
        kind_id: String,
        quantity: u32,
        used_slots: u16,
        required_slots: u16,
        capacity: u16,
    },
    Nothing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DestroyItemOutcome {
    pub(super) kind_id: String,
    pub(super) quantity: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DestroyItemFailure {
    Artifact,
    Indestructible,
    InvalidQuantity,
    NotFound,
    NotOwned,
    ProtectedInscription,
    TaskItem,
}

impl DestroyItemFailure {
    pub(super) const fn reason(self) -> &'static str {
        match self {
            Self::Artifact => "artifact",
            Self::Indestructible => "indestructible",
            Self::InvalidQuantity => "invalid-quantity",
            Self::NotFound => "not-found",
            Self::NotOwned => "not-owned",
            Self::ProtectedInscription => "protected-inscription",
            Self::TaskItem => "task-item",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct InscribeItemOutcome {
    pub(super) kind_id: String,
    pub(super) inscription: Option<String>,
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
    InventoryFull {
        kind_id: String,
        quantity: u32,
        used_slots: u16,
        required_slots: u16,
        capacity: u16,
    },
    Nothing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InventoryDamageProfile {
    element: ItemDestructionElement,
    chance_percent: u8,
    resistance: DamageType,
}

fn inventory_damage_profiles(
    damage_type: DamageType,
    touch: bool,
) -> &'static [InventoryDamageProfile] {
    use DamageType as Damage;
    use ItemDestructionElement as Element;

    match damage_type {
        Damage::Acid => &[InventoryDamageProfile {
            element: Element::Acid,
            chance_percent: 3,
            resistance: Damage::Acid,
        }],
        Damage::Electricity => &[InventoryDamageProfile {
            element: Element::Electricity,
            chance_percent: 3,
            resistance: Damage::Electricity,
        }],
        Damage::Fire => &[InventoryDamageProfile {
            element: Element::Fire,
            chance_percent: 3,
            resistance: Damage::Fire,
        }],
        Damage::Cold => &[InventoryDamageProfile {
            element: Element::Cold,
            chance_percent: 3,
            resistance: Damage::Cold,
        }],
        Damage::Water => &[InventoryDamageProfile {
            element: Element::Cold,
            chance_percent: 3,
            resistance: Damage::Sound,
        }],
        Damage::Plasma if !touch => &[InventoryDamageProfile {
            element: Element::Acid,
            chance_percent: 3,
            resistance: Damage::Fire,
        }],
        Damage::Chaos if !touch => &[
            InventoryDamageProfile {
                element: Element::Electricity,
                chance_percent: 2,
                resistance: Damage::Chaos,
            },
            InventoryDamageProfile {
                element: Element::Fire,
                chance_percent: 2,
                resistance: Damage::Chaos,
            },
        ],
        Damage::Shards if !touch => &[InventoryDamageProfile {
            element: Element::Cold,
            chance_percent: 2,
            resistance: Damage::Shards,
        }],
        Damage::Sound if !touch => &[InventoryDamageProfile {
            element: Element::Cold,
            chance_percent: 2,
            resistance: Damage::Sound,
        }],
        Damage::Nuke => &[InventoryDamageProfile {
            element: Element::Acid,
            chance_percent: 2,
            resistance: Damage::Poison,
        }],
        Damage::Meteor => &[
            InventoryDamageProfile {
                element: Element::Fire,
                chance_percent: 2,
                resistance: Damage::Fire,
            },
            InventoryDamageProfile {
                element: Element::Cold,
                chance_percent: 2,
                resistance: Damage::Shards,
            },
        ],
        Damage::Ice => &[
            InventoryDamageProfile {
                element: Element::Cold,
                chance_percent: 3,
                resistance: Damage::Cold,
            },
            InventoryDamageProfile {
                element: Element::Cold,
                chance_percent: 3,
                resistance: Damage::Cold,
            },
        ],
        Damage::Rocket => &[InventoryDamageProfile {
            element: Element::Cold,
            chance_percent: 3,
            resistance: Damage::Shards,
        }],
        _ => &[],
    }
}

fn inventory_resistance_power(damage_type: DamageType) -> u64 {
    if matches!(
        damage_type,
        DamageType::Acid
            | DamageType::Electricity
            | DamageType::Fire
            | DamageType::Cold
            | DamageType::Poison
    ) {
        66
    } else {
        41
    }
}

fn inventory_resistance_save(
    rng: &mut crate::rng::RfbRng,
    resistance: ResistanceLevel,
    damage_type: DamageType,
) -> bool {
    let power = inventory_resistance_power(damage_type);
    let resistance = u64::try_from(resistance.reduction_percent().max(0)).unwrap_or(0);
    rng.bounded(power) < resistance
}

fn equipped_ammunition_capacity(content: &ContentCatalog, items: &[ItemInstance]) -> u32 {
    items
        .iter()
        .filter(|item| matches!(item.location, ItemLocation::Equipped { .. }))
        .filter_map(|item| content.item(&item.kind_id))
        .fold(0_u32, |capacity, definition| {
            capacity.saturating_add(u32::from(definition.ammunition_capacity))
        })
}

fn quivered_ammunition_item_ids<'a>(
    content: &ContentCatalog,
    items: &'a [ItemInstance],
) -> BTreeSet<&'a str> {
    let mut ammunition_stacks = items
        .iter()
        .filter(|item| item.location == ItemLocation::Inventory)
        .filter(|item| {
            content
                .item(&item.kind_id)
                .is_some_and(|definition| definition.ammunition_profile.is_some())
        })
        .map(|item| (item.quantity, item.id.as_str()))
        .collect::<Vec<_>>();
    ammunition_stacks.sort_unstable();
    let mut capacity = equipped_ammunition_capacity(content, items);
    ammunition_stacks
        .into_iter()
        .filter_map(|(quantity, item_id)| {
            if quantity > capacity {
                return None;
            }
            capacity -= quantity;
            Some(item_id)
        })
        .collect()
}

fn inventory_used_slots(content: &ContentCatalog, items: &[ItemInstance]) -> u16 {
    let quivered = quivered_ammunition_item_ids(content, items);
    items
        .iter()
        .filter(|item| item.location == ItemLocation::Inventory)
        .filter(|item| !quivered.contains(item.id.as_str()))
        .fold(0_u16, |slots, _| slots.saturating_add(1))
}

fn compatible_inventory_space(
    content: &ContentCatalog,
    items: &[ItemInstance],
    item_property_knowledge: &BTreeMap<String, ItemPropertyKnowledgeState>,
    incoming: &ItemInstance,
    match_knowledge: bool,
) -> u32 {
    let Some(definition) = content.item(&incoming.kind_id) else {
        return 0;
    };
    let incoming_knowledge = item_property_knowledge.get(&incoming.id);
    items
        .iter()
        .filter(|carried| {
            carried.location == ItemLocation::Inventory
                && carried.quantity < definition.max_stack
                && item_instances_stack_compatible(carried, incoming)
                && (!match_knowledge
                    || item_properties_match(
                        item_property_knowledge.get(&carried.id),
                        incoming_knowledge,
                    ))
        })
        .fold(0_u32, |space, carried| {
            space.saturating_add(definition.max_stack - carried.quantity)
        })
}

pub(super) fn additional_inventory_slots(
    content: &ContentCatalog,
    items: &[ItemInstance],
    item_property_knowledge: &BTreeMap<String, ItemPropertyKnowledgeState>,
    incoming: &ItemInstance,
    quantity: u32,
    match_knowledge: bool,
) -> u16 {
    let Some(definition) = content.item(&incoming.kind_id) else {
        return u16::MAX;
    };
    let incoming_knowledge = item_property_knowledge.get(&incoming.id);
    let mut projected = items.to_vec();
    let mut stack_indices = projected
        .iter()
        .enumerate()
        .filter(|(_, carried)| {
            carried.location == ItemLocation::Inventory
                && carried.quantity < definition.max_stack
                && item_instances_stack_compatible(carried, incoming)
                && (!match_knowledge
                    || item_properties_match(
                        item_property_knowledge.get(&carried.id),
                        incoming_knowledge,
                    ))
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    stack_indices.sort_by(|left, right| projected[*left].id.cmp(&projected[*right].id));
    let mut remaining = quantity;
    for stack_index in stack_indices {
        let transferred = remaining.min(definition.max_stack - projected[stack_index].quantity);
        projected[stack_index].quantity += transferred;
        remaining -= transferred;
        if remaining == 0 {
            break;
        }
    }
    while remaining > 0 {
        let mut stack = incoming.clone();
        stack.id = format!("projected-inventory-{}", projected.len());
        stack.quantity = remaining.min(definition.max_stack);
        stack.location = ItemLocation::Inventory;
        remaining -= stack.quantity;
        projected.push(stack);
    }
    inventory_used_slots(content, &projected).saturating_sub(inventory_used_slots(content, items))
}

pub(super) fn inventory_quantity_capacity(
    content: &ContentCatalog,
    items: &[ItemInstance],
    item_property_knowledge: &BTreeMap<String, ItemPropertyKnowledgeState>,
    incoming: &ItemInstance,
    slot_capacity: u16,
    match_knowledge: bool,
) -> u32 {
    let Some(definition) = content.item(&incoming.kind_id) else {
        return 0;
    };
    let current_used = inventory_used_slots(content, items);
    if current_used > slot_capacity {
        return 0;
    }
    let stack_space = compatible_inventory_space(
        content,
        items,
        item_property_knowledge,
        incoming,
        match_knowledge,
    );
    let carried_ammunition = items
        .iter()
        .filter(|item| item.location == ItemLocation::Inventory)
        .filter(|item| {
            content
                .item(&item.kind_id)
                .is_some_and(|definition| definition.ammunition_profile.is_some())
        })
        .fold(0_u32, |quantity, item| {
            quantity.saturating_add(item.quantity)
        });
    let free_quiver_capacity = if definition.ammunition_profile.is_some() {
        equipped_ammunition_capacity(content, items).saturating_sub(carried_ammunition)
    } else {
        0
    };
    let free_slots = slot_capacity.saturating_sub(current_used);
    let mut low = 0_u32;
    let mut high = stack_space
        .saturating_add(u32::from(free_slots).saturating_mul(definition.max_stack))
        .saturating_add(free_quiver_capacity);
    while low < high {
        let middle = low.saturating_add(high).saturating_add(1) / 2;
        let required = additional_inventory_slots(
            content,
            items,
            item_property_knowledge,
            incoming,
            middle,
            match_knowledge,
        );
        if current_used.saturating_add(required) <= slot_capacity {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    low
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
    requested_slot_id: Option<&str>,
) -> Option<EquipPlan> {
    let inventory_index = items
        .iter()
        .position(|item| item.id == item_id && item.location == ItemLocation::Inventory)?;
    let carried = &items[inventory_index];
    let definition = content.item(&carried.kind_id)?;
    let declared_slot_type = definition.equipment_slot.as_deref()?;
    let slot_id = if let Some(requested_slot_id) = requested_slot_id {
        let requested_slot = body_slots
            .iter()
            .find(|slot| slot.id == requested_slot_id)?;
        if !item_can_occupy_slot_type(declared_slot_type, &requested_slot.slot_type) {
            return None;
        }
        requested_slot.id.clone()
    } else {
        body_slot_instance_for_type(body_slots, declared_slot_type, |slot_id| {
            items.iter().any(|item| {
                matches!(
                    &item.location,
                    ItemLocation::Equipped { slot_id: equipped } if equipped == slot_id
                )
            })
        })?
        .id
        .clone()
    };
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
    inventory_slot_capacity: u16,
    item_id: Option<&str>,
) -> Result<PickUpPlan, CoreError> {
    let Some(ground_index) = items
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            item.location == ItemLocation::Ground(player_position)
                && item_id.is_none_or(|item_id| item.id == item_id)
        })
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

    let used_slots = inventory_used_slots(content, items);
    let required_slots = additional_inventory_slots(
        content,
        items,
        item_property_knowledge,
        pickup_item,
        original_quantity,
        true,
    );
    if used_slots.saturating_add(required_slots) > inventory_slot_capacity {
        return Ok(PickUpPlan::InventoryFull {
            kind_id,
            quantity: original_quantity,
            used_slots,
            required_slots,
            capacity: inventory_slot_capacity,
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
                && item_properties_match(item_property_knowledge.get(&carried.id), pickup_knowledge)
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

pub(super) fn item_instances_stack_compatible(left: &ItemInstance, right: &ItemInstance) -> bool {
    left.kind_id == right.kind_id
        && left.inscription == right.inscription
        && left.origin_actor_kind_id == right.origin_actor_kind_id
        && left.origin_kind == right.origin_kind
        && left.damage_dice_override == right.damage_dice_override
        && left.discount_percent == right.discount_percent
        && left.quality == right.quality
        && left.affix_ids == right.affix_ids
        && left.rolled_affixes == right.rolled_affixes
        && left.enchantments == right.enchantments
        && left.curse == right.curse
        && left.activation == right.activation
        && left.charges == right.charges
        && left.fuel == right.fuel
        && left.device_recovery_progress == right.device_recovery_progress
        && left.captured_actor == right.captured_actor
}

impl Game {
    fn equipped_quiver_protects_ammunition(&self) -> bool {
        self.items
            .iter()
            .filter(|item| matches!(item.location, ItemLocation::Equipped { .. }))
            .filter(|item| {
                self.content
                    .item(&item.kind_id)
                    .is_some_and(|definition| definition.ammunition_capacity > 0)
            })
            .any(|item| {
                item.affix_ids.iter().any(|affix_id| {
                    self.content
                        .affix(affix_id)
                        .is_some_and(|affix| affix.protects_quiver_ammunition)
                }) || item.rolled_affixes.iter().any(|rolled| {
                    self.content
                        .affix(&rolled.affix_id)
                        .is_some_and(|affix| affix.protects_quiver_ammunition)
                })
            })
    }

    pub(super) fn damage_player_inventory(
        &mut self,
        source_kind_id: &str,
        damage_type: DamageType,
        touch: bool,
        damage_applied: i32,
        events: &mut Vec<DomainEvent>,
    ) {
        if damage_applied <= 0 {
            return;
        }
        if damage_type == DamageType::Nuke {
            let poison_resistance = self
                .effective_player_resistances()
                .level(DamageType::Poison);
            let poison_threshold =
                u64::try_from(poison_resistance.reduction_percent().max(0)).unwrap_or(0);
            if self.rng.bounded(55) < poison_threshold {
                return;
            }
        }
        let protected_ammunition = if self.equipped_quiver_protects_ammunition() {
            quivered_ammunition_item_ids(&self.content, &self.items)
                .into_iter()
                .map(str::to_owned)
                .collect::<BTreeSet<_>>()
        } else {
            BTreeSet::new()
        };
        let inventory_protected = self.player_has_status_kind(STATUS_INVENTORY_PROTECTION);

        for profile in inventory_damage_profiles(damage_type, touch) {
            let resistance = self
                .effective_player_resistances()
                .level(profile.resistance);
            let candidates = self
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| item.location == ItemLocation::Inventory && item.quantity > 0)
                .filter(|(_, item)| !protected_ammunition.contains(item.id.as_str()))
                .filter(|(_, item)| {
                    self.content.item(&item.kind_id).is_some_and(|definition| {
                        !definition.tags.iter().any(|tag| tag == "artifact")
                    })
                })
                .filter(|(_, item)| self.element_destroys_item(item, profile.element, true))
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            let mut removed_item_ids = Vec::new();
            for index in candidates {
                let mut destroyed = 0;
                for _ in 0..self.items[index].quantity {
                    if self.rng.bounded(100) >= u64::from(profile.chance_percent) {
                        continue;
                    }
                    if inventory_protected {
                        let _protection_roll = self.rng.bounded(100);
                        continue;
                    }
                    if inventory_resistance_save(&mut self.rng, resistance, profile.resistance) {
                        continue;
                    }
                    destroyed += 1;
                }
                if destroyed == 0 {
                    continue;
                }
                let item = &mut self.items[index];
                item.quantity -= destroyed;
                events.push(DomainEvent::InventoryItemDestroyedByDamage {
                    source_kind_id: source_kind_id.to_owned(),
                    target_kind_id: item.kind_id.clone(),
                    quantity: destroyed,
                });
                if item.quantity == 0 {
                    removed_item_ids.push(item.id.clone());
                }
            }
            if !removed_item_ids.is_empty() {
                self.items
                    .retain(|item| !removed_item_ids.contains(&item.id));
                for item_id in removed_item_ids {
                    self.item_property_knowledge.remove(&item_id);
                }
            }
        }
    }

    pub(super) fn can_destroy_item(&self, item: &ItemInstance) -> Result<(), DestroyItemFailure> {
        let definition = self
            .content
            .item(&item.kind_id)
            .ok_or(DestroyItemFailure::Indestructible)?;
        if definition.tags.iter().any(|tag| tag == "artifact") {
            return Err(DestroyItemFailure::Artifact);
        }
        if definition.tags.iter().any(|tag| tag == "indestructible") {
            return Err(DestroyItemFailure::Indestructible);
        }
        if item
            .inscription
            .as_deref()
            .is_some_and(inscription_protects_from_destruction)
        {
            return Err(DestroyItemFailure::ProtectedInscription);
        }
        let task_item = self.content.world(&self.world_id).is_some_and(|world| {
            world
                .tasks
                .iter()
                .flat_map(|task| &task.objectives)
                .any(|objective| {
                    objective.kind == TaskObjectiveKind::CollectItem
                        && objective.item_instance_id.as_deref().map_or_else(
                            || objective.item_kind_id.as_deref() == Some(item.kind_id.as_str()),
                            |instance_id| instance_id == item.id,
                        )
                })
        });
        if task_item {
            return Err(DestroyItemFailure::TaskItem);
        }
        Ok(())
    }

    pub(super) fn destroy_item(
        &mut self,
        item_id: &str,
        quantity: u32,
    ) -> Result<DestroyItemOutcome, DestroyItemFailure> {
        let index = self
            .items
            .iter()
            .position(|item| item.id == item_id)
            .ok_or(DestroyItemFailure::NotFound)?;
        if !matches!(self.items[index].location, ItemLocation::Inventory)
            && self.items[index].location != ItemLocation::Ground(self.player.position)
        {
            return Err(DestroyItemFailure::NotOwned);
        }
        self.can_destroy_item(&self.items[index])?;
        if quantity == 0 || quantity > self.items[index].quantity {
            return Err(DestroyItemFailure::InvalidQuantity);
        }
        let kind_id = self.items[index].kind_id.clone();
        if quantity == self.items[index].quantity {
            let removed = self.items.remove(index);
            self.item_property_knowledge.remove(&removed.id);
        } else {
            self.items[index].quantity -= quantity;
        }
        Ok(DestroyItemOutcome { kind_id, quantity })
    }

    pub(super) fn inscribe_item(
        &mut self,
        item_id: &str,
        inscription: Option<String>,
    ) -> Result<InscribeItemOutcome, &'static str> {
        let item = self
            .items
            .iter_mut()
            .find(|item| item.id == item_id)
            .ok_or("not-found")?;
        if !matches!(
            item.location,
            ItemLocation::Inventory | ItemLocation::Equipped { .. }
        ) && item.location != ItemLocation::Ground(self.player.position)
        {
            return Err("not-owned");
        }
        item.inscription = inscription.filter(|value| !value.is_empty());
        Ok(InscribeItemOutcome {
            kind_id: item.kind_id.clone(),
            inscription: item.inscription.clone(),
        })
    }

    pub(super) fn inventory_used_slots(&self) -> u16 {
        inventory_used_slots(&self.content, &self.items)
    }

    pub(super) fn inventory_slot_capacity(&self) -> u16 {
        let base = self
            .content
            .actor(&self.player.kind_id)
            .expect("player actor definition must remain available")
            .inventory_slot_capacity;
        self.items
            .iter()
            .filter_map(|item| {
                if !matches!(item.location, ItemLocation::Equipped { .. }) {
                    return None;
                }
                self.content
                    .item(&item.kind_id)
                    .map(|definition| definition.inventory_slot_bonus)
            })
            .fold(base, u16::saturating_add)
    }

    pub(super) fn inventory_quantity_capacity_for(
        &self,
        incoming: &ItemInstance,
        match_knowledge: bool,
    ) -> u32 {
        inventory_quantity_capacity(
            &self.content,
            &self.items,
            &self.item_property_knowledge,
            incoming,
            self.inventory_slot_capacity(),
            match_knowledge,
        )
    }

    pub(super) fn carry_shop_purchase_item(&mut self, mut item: ItemInstance) -> Vec<String> {
        let definition = self
            .content
            .item(&item.kind_id)
            .expect("purchased item kind must remain available");
        let mut stack_indices = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, carried)| {
                carried.location == ItemLocation::Inventory
                    && carried.quantity < definition.max_stack
                    && item_instances_stack_compatible(carried, &item)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        stack_indices.sort_by(|left, right| self.items[*left].id.cmp(&self.items[*right].id));

        let mut destination_ids = Vec::new();
        for stack_index in stack_indices {
            let transferred = item
                .quantity
                .min(definition.max_stack - self.items[stack_index].quantity);
            if transferred == 0 {
                continue;
            }
            self.items[stack_index].quantity += transferred;
            item.quantity -= transferred;
            destination_ids.push(self.items[stack_index].id.clone());
            if item.quantity == 0 {
                break;
            }
        }
        if item.quantity > 0 {
            destination_ids.push(item.id.clone());
            self.items.push(item);
        }
        destination_ids
    }

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
        let knowledge = self
            .item_property_knowledge
            .entry(item_instance_id)
            .or_default();
        knowledge.discovered = true;
        knowledge.appraised = true;
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
        knowledge.discovered = true;
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

    pub(super) fn identify_carried_items(&mut self) -> usize {
        let mut item_ids = self
            .items
            .iter()
            .filter(|item| {
                item.quantity > 0
                    && matches!(
                        item.location,
                        ItemLocation::Inventory | ItemLocation::Equipped { .. }
                    )
            })
            .map(|item| item.id.clone())
            .collect::<Vec<_>>();
        item_ids.sort();
        item_ids
            .into_iter()
            .filter(|item_id| {
                self.identify_item_instance(item_id, ItemIdentificationRequest::new(false))
                    .changed
            })
            .count()
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
        before: i16,
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
                .get(usize::try_from(after.max(0)).expect("non-negative enchantment fits usize"))
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
            successes: u16::try_from(after.saturating_sub(before)).unwrap_or_default(),
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
                    EquippedItemCurseTarget::Any => true,
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
            let severity = if request.heavy_chance_percent > 0
                && self.rng.bounded(100) < u64::from(request.heavy_chance_percent)
            {
                ItemCurseSeverityDto::Heavy
            } else {
                ItemCurseSeverityDto::Normal
            };
            self.items[item_index].curse =
                Some(before.map_or(severity, |current| current.max(severity)));
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
        item.location == ItemLocation::Inventory && self.item_has_recharge_capacity(item)
    }

    pub(super) fn item_can_receive_player_recharge(&self, item: &ItemInstance) -> bool {
        self.item_is_in_pack_or_at_feet(item) && self.item_has_recharge_capacity(item)
    }

    pub(super) fn item_can_be_absorbed(&self, item: &ItemInstance) -> bool {
        self.character_definitions()
            .is_some_and(|(_, race, _, _)| race.tags.iter().any(|tag| tag == "device-eater"))
            && self.item_is_in_pack_or_at_feet(item)
            && self.item_is_device(item)
            && item.charges.is_some()
            && self.item_device_charge_cost(item).is_some()
    }

    pub(super) fn absorb_device(&mut self, item_id: &str) -> Option<DeviceAbsorptionOutcome> {
        let index = self
            .items
            .iter()
            .position(|item| item.id == item_id && self.item_can_be_absorbed(item))?;
        let item_kind_id = self.items[index].kind_id.clone();
        let cost = self.item_device_charge_cost(&self.items[index])?;
        let nutrition_before = self.nutrition;
        let (charges_before, charges_after, drained) = self.decrease_item_charges(index, cost);
        if drained > 0 {
            self.increase_nutrition(5_000);
        }
        Some(DeviceAbsorptionOutcome {
            item_id: item_id.to_owned(),
            item_kind_id,
            charges_before,
            charges_after,
            drained,
            nutrition_before,
            nutrition_after: self.nutrition,
        })
    }

    pub(super) fn decrease_item_charges(
        &mut self,
        item_index: usize,
        requested: u32,
    ) -> (u32, u32, u32) {
        let charges = self.items[item_index]
            .charges
            .as_mut()
            .expect("preflighted charged item must retain energy state");
        let before = charges.current;
        let drained = requested.min(before);
        charges.current -= drained;
        (before, charges.current, drained)
    }

    pub(super) fn item_is_in_pack_or_at_feet(&self, item: &ItemInstance) -> bool {
        item.location == ItemLocation::Inventory
            || item.location == ItemLocation::Ground(self.player.position)
    }

    fn item_is_device(&self, item: &ItemInstance) -> bool {
        self.content
            .item(&item.kind_id)
            .is_some_and(|definition| definition.tags.iter().any(|tag| tag == "device"))
    }

    fn item_device_charge_cost(&self, item: &ItemInstance) -> Option<u32> {
        item.activation
            .as_ref()
            .map(|activation| activation.cost)
            .or_else(|| {
                self.content
                    .item(&item.kind_id)
                    .and_then(|definition| definition.use_action.as_ref())
                    .and_then(|action| action.charges)
                    .map(|charges| charges.cost)
            })
    }

    fn item_has_recharge_capacity(&self, item: &ItemInstance) -> bool {
        item.activation.is_some()
            && item_device_generation(&self.content, &item.kind_id, &item.affix_ids).is_some()
            && item
                .charges
                .is_some_and(|charges| charges.current < charges.maximum)
    }

    pub(super) fn recharge_inventory_item_from_player(
        &mut self,
        target_item_id: &str,
        attempted: u32,
        power: u32,
    ) -> InventoryItemRechargeOutcome {
        self.recharge_inventory_item_target(
            target_item_id,
            InventoryItemRechargeRequest::from_player(attempted, power),
        )
    }

    pub(super) fn item_can_supply_recharge(&self, item: &ItemInstance) -> bool {
        item.location == ItemLocation::Inventory
            && self.item_is_device(item)
            && item.charges.is_some_and(|charges| charges.current > 0)
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
        let source_destroyed =
            destroy && !artifact && !self.player_has_status_kind(STATUS_INVENTORY_PROTECTION);
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
        } else if request.drain_on_failure {
            charges.current = 0;
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

    pub(super) fn equip_inventory_item(
        &mut self,
        item_id: &str,
        slot_id: Option<&str>,
    ) -> Option<EquipOutcome> {
        let plan = plan_equip(
            &self.content,
            &self.body_slots,
            &self.items,
            item_id,
            slot_id,
        )?;
        if plan
            .replaced_index
            .is_some_and(|index| self.items[index].curse.is_some())
        {
            return None;
        }
        let current_capacity = self.inventory_slot_capacity();
        let equipped_bonus = self
            .content
            .item(&self.items[plan.inventory_index].kind_id)?
            .inventory_slot_bonus;
        let replaced_bonus = plan
            .replaced_index
            .and_then(|index| self.content.item(&self.items[index].kind_id))
            .map_or(0, |definition| definition.inventory_slot_bonus);
        let projected_capacity = current_capacity
            .saturating_sub(replaced_bonus)
            .saturating_add(equipped_bonus);
        let mut projected_items = self.items.clone();
        if let Some(index) = plan.replaced_index {
            projected_items[index].location = ItemLocation::Inventory;
        }
        projected_items[plan.inventory_index].location = ItemLocation::Equipped {
            slot_id: plan.slot_id.clone(),
        };
        let projected_used = inventory_used_slots(&self.content, &projected_items);
        if projected_used > projected_capacity {
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
        knowledge.discovered = true;
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
        slot_id: Option<&str>,
    ) -> Option<(String, String, ItemCurseSeverityDto)> {
        let plan = plan_equip(
            &self.content,
            &self.body_slots,
            &self.items,
            item_id,
            slot_id,
        )?;
        let replaced = &self.items[plan.replaced_index?];
        Some((replaced.kind_id.clone(), plan.slot_id, replaced.curse?))
    }

    pub(super) fn unequip_slot(&mut self, slot_id: &str) -> Option<String> {
        let plan = plan_unequip(&self.items, slot_id)?;
        if plan.curse.is_some() {
            return None;
        }
        let removed_bonus = self.content.item(&plan.kind_id)?.inventory_slot_bonus;
        let projected_capacity = self.inventory_slot_capacity().saturating_sub(removed_bonus);
        let mut projected_items = self.items.clone();
        projected_items[plan.item_index].location = ItemLocation::Inventory;
        if inventory_used_slots(&self.content, &projected_items) > projected_capacity {
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
        self.pick_up_item_at_player(None)
    }

    pub(super) fn pick_up_item_at_player(
        &mut self,
        item_id: Option<&str>,
    ) -> Result<PickUpOutcome, CoreError> {
        let plan = plan_pick_up(
            &self.content,
            &self.items,
            &self.item_property_knowledge,
            self.player.position,
            self.inventory_slot_capacity(),
            item_id,
        )?;
        match plan {
            PickUpPlan::Nothing => Ok(PickUpOutcome::Nothing),
            PickUpPlan::InventoryFull {
                kind_id,
                quantity,
                used_slots,
                required_slots,
                capacity,
            } => Ok(PickUpOutcome::InventoryFull {
                kind_id,
                quantity,
                used_slots,
                required_slots,
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

    pub(super) fn record_pick_up_outcome(
        &self,
        outcome: PickUpOutcome,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> bool {
        match outcome {
            PickUpOutcome::Picked { kind_id, quantity } => {
                changed.insert(self.player.position);
                events.push(DomainEvent::ItemPickedUp {
                    target_kind_id: kind_id,
                    quantity,
                });
                true
            }
            PickUpOutcome::InventoryFull {
                kind_id,
                quantity,
                used_slots,
                required_slots,
                capacity,
            } => {
                events.push(DomainEvent::ItemPickupInventoryFull {
                    target_kind_id: kind_id,
                    quantity,
                    used_slots,
                    required_slots,
                    capacity,
                });
                false
            }
            PickUpOutcome::Nothing => false,
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

fn inscription_protects_from_destruction(inscription: &str) -> bool {
    inscription.split('!').skip(1).any(|commands| {
        commands
            .chars()
            .take_while(|command| command.is_ascii_alphabetic() || *command == '*')
            .any(|command| command == 'k' || command == '*')
    })
}
