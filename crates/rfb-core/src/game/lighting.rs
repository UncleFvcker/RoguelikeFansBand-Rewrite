// SPDX-License-Identifier: MPL-2.0

use rfb_protocol::ItemFuelKindDto;

use super::*;

pub(super) const WOODEN_TORCH_ITEM_KIND_ID: &str = "demo.item.wooden-torch";
const LIGHT_FUEL_INTERVAL_TICKS: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StartingTorchSupply {
    pub(super) quantity: u32,
    pub(super) fuel: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LightRefuelOutcome {
    pub(super) target_item_id: String,
    pub(super) target_kind_id: String,
    pub(super) source_kind_id: String,
    pub(super) amount: u16,
    pub(super) current: u16,
    pub(super) maximum: u16,
}

pub(super) fn starting_torch_supply(
    build: Option<&CharacterBuildIdentity>,
    rng: &mut RfbRng,
) -> Option<StartingTorchSupply> {
    build.is_some().then(|| StartingTorchSupply {
        quantity: u32::try_from(rng.bounded(5) + 3).expect("birth torch quantity must fit u32"),
        fuel: u16::try_from((rng.bounded(5) + 3) * 500).expect("birth torch fuel must fit u16"),
    })
}

impl Game {
    pub(super) fn extinguish_area(&mut self, origin: Position, radius: u8) -> Vec<Position> {
        let darkened = self
            .area_damage_cells(origin, radius)
            .into_iter()
            .map(|(_, position)| position)
            .filter(|position| self.index(*position).is_some_and(|index| self.glow[index]))
            .collect::<Vec<_>>();
        for position in &darkened {
            let index = self
                .index(*position)
                .expect("area light position must remain in bounds");
            self.glow[index] = false;
        }
        darkened
    }

    pub(super) fn refuel_light_unavailable_reason(
        &self,
        target_item_id: &str,
        source_item_id: &str,
    ) -> Option<&'static str> {
        if target_item_id == source_item_id {
            return Some("same-item");
        }
        let Some(target) = self.items.iter().find(|item| item.id == target_item_id) else {
            return Some("target-missing");
        };
        if !matches!(&target.location, ItemLocation::Equipped { slot_id } if slot_id == "light") {
            return Some("target-not-equipped");
        }
        let Some(target_fuel) = target.fuel else {
            return Some("target-not-refillable");
        };
        if !matches!(
            target_fuel.kind,
            ItemFuelKindDto::Torch | ItemFuelKindDto::Lantern
        ) {
            return Some("target-not-refillable");
        }
        if target_fuel.current >= target_fuel.maximum {
            return Some("target-full");
        }
        let Some(source) = self.items.iter().find(|item| item.id == source_item_id) else {
            return Some("source-missing");
        };
        if source.location != ItemLocation::Inventory {
            return Some("source-not-carried");
        }
        let Some(source_fuel) = source.fuel else {
            return Some("source-incompatible");
        };
        if source_fuel.current == 0 {
            return Some("source-empty");
        }
        let compatible = match target_fuel.kind {
            ItemFuelKindDto::Torch => source_fuel.kind == ItemFuelKindDto::Torch,
            ItemFuelKindDto::Lantern => {
                matches!(
                    source_fuel.kind,
                    ItemFuelKindDto::Lantern | ItemFuelKindDto::Oil
                )
            }
            ItemFuelKindDto::Oil => false,
        };
        (!compatible).then_some("source-incompatible")
    }

    pub(super) fn refuel_equipped_light(
        &mut self,
        target_item_id: &str,
        source_item_id: &str,
    ) -> Option<LightRefuelOutcome> {
        if self
            .refuel_light_unavailable_reason(target_item_id, source_item_id)
            .is_some()
        {
            return None;
        }
        Some(self.apply_light_refuel(target_item_id, source_item_id))
    }

    pub(super) fn apply_light_refuel(
        &mut self,
        target_item_id: &str,
        source_item_id: &str,
    ) -> LightRefuelOutcome {
        debug_assert!(
            self.refuel_light_unavailable_reason(target_item_id, source_item_id)
                .is_none()
        );
        let source_index = self
            .items
            .iter()
            .position(|item| item.id == source_item_id)
            .expect("preflighted fuel source must remain available");
        let source_kind_id = self.items[source_index].kind_id.clone();
        let source_fuel = self.items[source_index]
            .fuel
            .expect("preflighted fuel source must retain fuel");
        let target_kind = self
            .items
            .iter()
            .find(|item| item.id == target_item_id)
            .and_then(|item| item.fuel)
            .expect("preflighted light target must retain fuel")
            .kind;
        let requested = if target_kind == ItemFuelKindDto::Torch {
            source_fuel.current.saturating_add(5)
        } else {
            source_fuel.current
        };
        if self.items[source_index].quantity > 1 {
            self.items[source_index].quantity -= 1;
        } else {
            let removed = self.items.remove(source_index);
            self.item_property_knowledge.remove(&removed.id);
        }
        let target = self
            .items
            .iter_mut()
            .find(|item| item.id == target_item_id)
            .expect("preflighted light target must remain available");
        let target_kind_id = target.kind_id.clone();
        let fuel = target
            .fuel
            .as_mut()
            .expect("preflighted light target must retain fuel");
        let before = fuel.current;
        fuel.current = fuel.current.saturating_add(requested).min(fuel.maximum);
        LightRefuelOutcome {
            target_item_id: target_item_id.to_owned(),
            target_kind_id,
            source_kind_id,
            amount: fuel.current - before,
            current: fuel.current,
            maximum: fuel.maximum,
        }
    }

    pub(super) fn process_equipped_light_fuel(&mut self, events: &mut Vec<DomainEvent>) {
        if !self.world_tick.is_multiple_of(LIGHT_FUEL_INTERVAL_TICKS) {
            return;
        }
        let Some(item) = self.items.iter_mut().find(|item| {
            matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == "light")
                && item
                    .fuel
                    .is_some_and(|fuel| fuel.light_radius > 0 && fuel.current > 0)
        }) else {
            return;
        };
        let fuel = item.fuel.as_mut().expect("selected light must have fuel");
        fuel.current -= 1;
        if fuel.current == 0 {
            events.push(DomainEvent::LightExtinguished {
                target_item_id: item.id.clone(),
                target_kind_id: item.kind_id.clone(),
            });
        }
    }

    pub(super) fn player_light_radius(&self) -> Option<i32> {
        let equipment = self
            .items
            .iter()
            .find(|item| {
                matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == "light")
            })
            .and_then(|item| item.fuel)
            .filter(|fuel| fuel.current > 0 && fuel.light_radius > 0)
            .map_or(0, |fuel| i32::from(fuel.light_radius));
        let status = self
            .player
            .statuses
            .iter()
            .map(|status| status.granted_equipment_bonuses.light_radius)
            .max()
            .unwrap_or_default();
        let radius = equipment
            .max(self.player_mutation_light_radius())
            .max(status);
        (radius > 0).then_some(radius)
    }
}
