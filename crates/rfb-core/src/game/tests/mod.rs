// SPDX-License-Identifier: MPL-2.0
use crate::effect::StatusInstance;
use crate::game::loot::{ItemGenerationMode, LootContext, LootSource};
use crate::resistance::ResistanceLevel;
use rfb_content::{
    AbilityLevelScalingCurveDefinition, AbilityLevelScalingDefinition, AbilityLevelScalingField,
    AbilitySpellPowerDefinition,
};
use rfb_protocol::{
    AbilityEffectSpecDto, AbilitySummonCandidateSpecDto, AbilityTerrainBeamOperationDto,
    CellVisualDto, DamageTypeDto, Direction, GameCommand, GameCommandEnvelope, GameEventOutcomeDto,
    GameSnapshot, ResistanceLevelDto, ShopCategoryDto, ShopDto, SniperShotModeDto, StatusSaveDto,
    TerrainInteractionKindDto, VisibilityState,
};

use super::*;

mod abilities;
mod acquirement;
mod archer;
mod artifact_identity;
mod attribute_sources;
mod berserker;

mod book_discovery;
mod capture_ball;
mod cavalry;
mod combat;
mod experience;
mod generation;
mod gold;
mod high_mage;
mod hunger;
mod inventory;
mod items;
mod lighting;
mod mindcrafter;
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
mod race_attribute_sustains;
mod riding;
mod riding_bond;
mod snapshots;
mod sniper;
mod snow;
mod spectre_rules;
mod spectre_supplies;
pub(crate) mod support;
mod tasks;
mod town;
mod trait_details;
mod vampire;
mod virtue_state;
mod wall_passage;
mod weapon_ego_activations;
mod weapon_proficiency;
mod weapon_traits;
mod world;
