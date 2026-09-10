// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeSet, VecDeque};

use rfb_protocol::{CellLightDto, ItemFuelKindDto, Position};

use super::Game;
use super::projectile_geometry::rfb_distance;
use crate::{
    effect::STATUS_SLEEP, event::DomainEvent, rng::RfbRng, state::ItemLocation,
    stats::CharacterBuildIdentity,
};

pub(super) const WOODEN_TORCH_ITEM_KIND_ID: &str = "demo.item.wooden-torch";
const LIGHT_FUEL_INTERVAL_TICKS: u32 = 10;
pub(super) const SURFACE_AMBIENT_LIGHT: u8 = 48;
pub(super) const DUNGEON_AMBIENT_LIGHT: u8 = 0;
const ROOM_GLOW_LIGHT: u8 = 48;
const ITEM_LIGHT_RADIUS: i32 = 4;
const PLAYER_LIGHT_COLOR: u32 = 0xffd7a3;
const ACTOR_LIGHT_COLOR: u32 = 0xff8a4c;
const ITEM_LIGHT_COLOR: u32 = 0x8ad9ff;

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

fn source_intensity(source: Position, target: Position, radius: i32, maximum: u8) -> u8 {
    let distance = rfb_distance(source, target);
    let radius = u32::try_from(radius).expect("validated light radius must be non-negative");
    if distance > radius {
        return 0;
    }

    // RFB treats the source and all eight adjacent grids as the same inner
    // light band. Every included outer band remains lit at reduced strength.
    let remaining = radius.saturating_sub(distance.saturating_sub(1));
    u8::try_from(u32::from(maximum).saturating_mul(remaining) / radius.max(1))
        .expect("scaled light intensity must fit u8")
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LightSource {
    position: Position,
    radius: i32,
    maximum: u8,
    color: u32,
    darkness: bool,
}

impl LightSource {
    fn contains(self, position: Position) -> bool {
        rfb_distance(self.position, position)
            <= u32::try_from(self.radius).expect("validated light radius must be non-negative")
    }
}

pub(super) fn light_from_sources(
    sources: &[LightSource],
    position: Position,
    ambient_light: u8,
) -> CellLightDto {
    let mut strongest = (0_u8, PLAYER_LIGHT_COLOR);
    for source in sources.iter().filter(|source| !source.darkness) {
        let boost = source_intensity(source.position, position, source.radius, source.maximum);
        if boost > strongest.0 {
            strongest = (boost, source.color);
        }
    }
    CellLightDto {
        color: strongest.1,
        intensity: ambient_light.saturating_add(strongest.0),
    }
}

impl Game {
    pub(super) fn floor_has_environment_light(&self) -> bool {
        self.is_wilderness_floor()
            || self.current_town().is_some()
            || self
                .content
                .world(&self.world_id)
                .is_some_and(|world| self.current_floor_id == world.initial_floor_id)
    }

    pub(super) fn ambient_light(&self, position: Position, sources: &[LightSource]) -> u8 {
        if sources
            .iter()
            .any(|source| source.darkness && source.contains(position))
        {
            return DUNGEON_AMBIENT_LIGHT;
        }
        let Some(index) = self.index(position) else {
            return DUNGEON_AMBIENT_LIGHT;
        };
        if self.floor_has_environment_light()
            && self.wilderness_is_daytime()
            && (!self.daylight_suppressed[index] || self.glow[index])
        {
            SURFACE_AMBIENT_LIGHT
        } else if self.glow[index] {
            ROOM_GLOW_LIGHT
        } else {
            DUNGEON_AMBIENT_LIGHT
        }
    }

    pub(super) fn set_floor_glow_at(&mut self, position: Position, glow: bool) -> bool {
        if glow && self.dungeon_has_darkness() {
            return false;
        }
        let Some(index) = self.index(position) else {
            return false;
        };
        let suppress_daylight = !glow
            && self.floor_has_environment_light()
            && (self.wilderness_is_daytime() || self.daylight_suppressed[index]);
        let changed =
            self.glow[index] != glow || self.daylight_suppressed[index] != suppress_daylight;
        self.glow[index] = glow;
        self.daylight_suppressed[index] = suppress_daylight;
        changed
    }

    pub(super) fn clear_daylight_suppression_at_dawn(&mut self) {
        if !self
            .world_tick
            .is_multiple_of(super::wilderness::WILDERNESS_DAY_TICKS)
        {
            return;
        }
        self.daylight_suppressed.fill(false);
        for floor in self.stored_floors.values_mut() {
            floor.daylight_suppressed.fill(false);
        }
    }

    pub(super) fn position_is_lit(&self, position: Position) -> bool {
        let sources = self.collect_light_sources();
        sources
            .iter()
            .any(|source| !source.darkness && source.contains(position))
            || self.ambient_light(position, &sources) > 0
    }

    pub(super) fn connected_glow_positions(&self, origin: Position) -> Vec<Position> {
        let Some(origin_index) = self.index(origin) else {
            return Vec::new();
        };
        if !self.glow[origin_index] {
            return Vec::new();
        }

        let mut visited = BTreeSet::from([origin]);
        let mut queue = VecDeque::from([origin]);
        let mut positions = Vec::new();
        while let Some(position) = queue.pop_front() {
            positions.push(position);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let neighbor = Position {
                        x: position.x + dx,
                        y: position.y + dy,
                    };
                    let Some(index) = self.index(neighbor) else {
                        continue;
                    };
                    if self.glow[index] && visited.insert(neighbor) {
                        queue.push_back(neighbor);
                    }
                }
            }
        }
        positions
    }

    pub(super) fn darken_room(&mut self, origin: Position) -> Vec<Position> {
        let darkened = self.connected_glow_positions(origin);
        for position in &darkened {
            self.set_floor_glow_at(*position, false);
        }
        darkened
    }

    pub(super) fn collect_light_sources(&self) -> Vec<LightSource> {
        // The source order mirrors the original per-cell scan (player, then
        // entities, then ground items) so strict-greater comparisons keep
        // resolving ties identically.
        let mut sources = Vec::new();
        let darkness = self.dungeon_has_darkness();
        let monster_distance_limit = (darkness && !self.player_has_night_vision())
            .then(|| (self.player_monster_sight_radius() + 1) as u32);
        if let Some(radius) = self.player_light_radius() {
            sources.push(LightSource {
                position: self.player.position,
                radius,
                maximum: 72,
                color: PLAYER_LIGHT_COLOR,
                darkness: false,
            });
        }
        for entity in &self.entities {
            let Some(definition) = self.actor_runtime_definition(entity) else {
                continue;
            };
            let Some(light) = definition.light else {
                continue;
            };
            // cave.c:update_mon_lite limits both positive and negative sources.
            if monster_distance_limit
                .is_some_and(|limit| rfb_distance(self.player.position, entity.position) > limit)
            {
                continue;
            }
            if !light.intrinsic
                && entity
                    .statuses
                    .iter()
                    .any(|status| status.kind_id == STATUS_SLEEP)
            {
                continue;
            }
            sources.push(LightSource {
                position: entity.position,
                radius: if darkness && !light.darkness {
                    1
                } else {
                    i32::from(light.radius)
                },
                maximum: 64,
                color: ACTOR_LIGHT_COLOR,
                darkness: light.darkness,
            });
        }
        for item in &self.items {
            let ItemLocation::Ground(item_position) = &item.location else {
                continue;
            };
            let Some(definition) = self.content.item(&item.kind_id) else {
                continue;
            };
            if definition.fuel.is_some() || !definition.tags.iter().any(|tag| tag == "light-source")
            {
                continue;
            }
            sources.push(LightSource {
                position: *item_position,
                radius: ITEM_LIGHT_RADIUS,
                maximum: 52,
                color: ITEM_LIGHT_COLOR,
                darkness: false,
            });
        }
        sources
    }

    pub(super) fn extinguish_area(&mut self, origin: Position, radius: u8) -> Vec<Position> {
        let positions = self
            .area_damage_cells(origin, radius)
            .into_iter()
            .map(|(_, position)| position)
            .collect::<Vec<_>>();
        positions
            .into_iter()
            .filter(|position| self.set_floor_glow_at(*position, false))
            .collect()
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
        if super::ego::item_has_ego(&self.content, item, 237)
            && self
                .world_tick
                .is_multiple_of(LIGHT_FUEL_INTERVAL_TICKS * 2)
        {
            return;
        }
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
            .filter_map(|item| match &item.location {
                ItemLocation::Equipped { slot_id } => {
                    let bonus = self.item_equipment_bonuses(item).light_radius;
                    if slot_id == "light" {
                        if self.item_has_darkness(item) {
                            return Some(
                                match self
                                    .content
                                    .item(&item.kind_id)
                                    .and_then(|definition| definition.rfb_base_kind)
                                    .map(|base| base.sval)
                                {
                                    Some(0) => -1,
                                    Some(1) => -2,
                                    _ => -3,
                                },
                            );
                        }
                        let fuel = item
                            .fuel
                            .filter(|fuel| fuel.current > 0)
                            .map_or(0, |fuel| i32::from(fuel.light_radius));
                        if self
                            .content
                            .item(&item.kind_id)
                            .and_then(|definition| definition.rfb_base_kind)
                            .is_some_and(|base| base.tval == 39)
                        {
                            Some(fuel + bonus)
                        } else {
                            Some(fuel.max(bonus))
                        }
                    } else {
                        Some(bonus)
                    }
                }
                _ => None,
            })
            .fold(0_i32, i32::saturating_add);
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
        // xtra1.c:calc_torch caps positive light before the intrinsic weak glow.
        let radius = if self.dungeon_has_darkness() {
            radius.min(1)
        } else {
            radius
        };
        let radius = if radius <= 0 && self.player_is_vampire() {
            equipment.saturating_add(1)
        } else {
            radius
        };
        (radius > 0).then_some(radius)
    }

    pub(super) fn item_has_darkness(&self, item: &crate::state::ItemInstance) -> bool {
        self.content
            .item(&item.kind_id)
            .is_some_and(|definition| definition.equipment_bonuses.light_radius < 0)
            || item
                .affix_ids
                .iter()
                .filter_map(|id| self.content.affix(id))
                .any(|affix| affix.equipment_bonuses.light_radius < 0)
            || item.intrinsic_properties.equipment_bonuses.light_radius < 0
            || item
                .rolled_affixes
                .iter()
                .any(|rolled| rolled.properties.equipment_bonuses.light_radius < 0)
    }
}
