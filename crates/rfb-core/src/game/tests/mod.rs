// SPDX-License-Identifier: MPL-2.0
use crate::effect::StatusInstance;
use crate::resistance::ResistanceLevel;
use crate::save::{actors_to_save, items_to_save};
use rfb_protocol::{
    AbilityLearningDto, CellVisualDto, CheckOutcomeDto, CheckResolutionDto, DamageResolutionDto,
    DamageTypeDto, DeviceRechargeSourceDto, Direction, EntityFactionDto, FloorConnectionSaveDto,
    GameCommand, GameCommandEnvelope, GameEventOutcomeDto, GameSnapshot, ItemKnowledgeSaveDto,
    ResistanceLevelDto, ResistanceSaveDto, SavePayloadV1, StatusSaveDto, TerrainInteractionKindDto,
    VisibilityState,
};

use super::*;

mod abilities;
mod combat;
mod deterministic_replay;
mod generation;
mod inventory;
mod items;
mod monster_ai;
mod movement;
mod persistence;
mod progression;
mod snapshots;
mod summons;
mod support;
mod tasks;
mod world;
