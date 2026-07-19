// SPDX-License-Identifier: MPL-2.0

use rfb_protocol::{ItemQualityDto, Position};
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ItemLocation {
    Ground(Position),
    Inventory,
    Equipped { slot_id: String },
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

pub(crate) struct EquipOutcome {
    pub(crate) kind_id: String,
    pub(crate) slot_id: String,
    pub(crate) replaced_kind_id: Option<String>,
    pub(crate) discovered_affix_ids: Vec<String>,
}
