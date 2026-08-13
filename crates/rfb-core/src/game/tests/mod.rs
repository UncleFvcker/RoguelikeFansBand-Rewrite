// SPDX-License-Identifier: MPL-2.0
use crate::effect::StatusInstance;
use crate::resistance::ResistanceLevel;
use rfb_protocol::{
    CellVisualDto, DamageTypeDto, Direction, GameCommand, GameCommandEnvelope, GameEventOutcomeDto,
    GameSnapshot, ResistanceLevelDto, ShopCategoryDto, ShopDto, StatusSaveDto,
    TerrainInteractionKindDto, VisibilityState,
};

use super::*;

mod abilities;
mod archer;
mod capture_ball;
mod cavalry;
mod combat;
mod deterministic_replay;
mod generation;
mod gold;
mod high_mage;
mod hunger;
mod inventory;
mod items;
mod lighting;
mod mining_progress;
mod monster_ai;
mod monster_doors;
mod monster_ecology;
mod monster_hit_points;
mod monster_movement;
mod mutations;
mod paladin;
mod persistence;
mod pet_upkeep;
mod prayer_study;
mod progression;
mod riding;
mod riding_bond;
mod snapshots;
mod sniper;
pub(crate) mod support;
mod tasks;
mod town;
mod virtue_state;
mod weapon_proficiency;
mod world;
