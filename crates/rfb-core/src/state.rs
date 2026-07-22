// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeSet;

use rfb_protocol::{ItemQualityDto, MonsterPackBehaviorDto, MonsterPackRoleDto, Position};
use serde::{Deserialize, Serialize};

use crate::{effect::StatusInstance, resistance::ResistanceProfile};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Actor {
    pub(crate) id: String,
    pub(crate) kind_id: String,
    pub(crate) position: Position,
    pub(crate) hp: i32,
    pub(crate) max_hp: i32,
    pub(crate) speed: u16,
    pub(crate) energy_need: i32,
    pub(crate) statuses: Vec<StatusInstance>,
    pub(crate) resistances: ResistanceProfile,
    pub(crate) pack: Option<MonsterPackIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MonsterPackIdentity {
    pub(crate) id: String,
    pub(crate) leader_id: String,
    pub(crate) role: MonsterPackRoleDto,
    pub(crate) behavior: MonsterPackBehaviorDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ItemLocation {
    Ground(Position),
    Inventory,
    Equipped { slot_id: String },
    CarriedBy { actor_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ItemInstance {
    pub(crate) id: String,
    pub(crate) kind_id: String,
    pub(crate) quantity: u32,
    pub(crate) quality: ItemQualityDto,
    pub(crate) affix_ids: Vec<String>,
    pub(crate) location: ItemLocation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FloorState {
    pub(crate) id: String,
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) terrain: Vec<String>,
    pub(crate) player_position: Position,
    pub(crate) entities: Vec<Actor>,
    pub(crate) items: Vec<ItemInstance>,
    pub(crate) explored: Vec<bool>,
    pub(crate) revealed_terrain: BTreeSet<Position>,
    pub(crate) connections: Vec<FloorConnectionState>,
    pub(crate) regions: Vec<FloorRegionState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FloorConnectionState {
    pub(crate) id: String,
    pub(crate) position: Position,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FloorRegionState {
    pub(crate) region_id: String,
    pub(crate) theme_id: String,
    pub(crate) encounter_table_id: String,
    pub(crate) loot_table_id: String,
    pub(crate) cells: Vec<Position>,
}

pub(crate) struct EquipOutcome {
    pub(crate) kind_id: String,
    pub(crate) slot_id: String,
    pub(crate) replaced_kind_id: Option<String>,
    pub(crate) discovered_affix_ids: Vec<String>,
}
