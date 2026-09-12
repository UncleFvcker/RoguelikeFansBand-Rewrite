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
mod android;
mod archer;
mod arena;
mod artifact_identity;
mod asgard;
mod attribute_sources;
mod berserker;

mod balrog;
mod book_discovery;
mod capture_ball;
mod cavalry;
mod centaur;
mod combat;
mod craft;
mod death_scythe;
mod devices;
mod disaster_area;
mod dragon_scale_artifacts;
mod duelist;
mod dungeon_anti_magic;
mod dungeon_anti_melee;
mod experience;
mod generation;
mod gold;
mod high_mage;
mod hunger;
mod inventory;
mod item_combat_activations;
mod items;
mod lighting;
mod mage;
mod magic_eater;
mod maia;
mod mindcrafter;
mod mining_progress;
mod monster_ai;
mod monster_doors;
mod monster_ecology;
mod monster_hit_points;
mod monster_movement;
mod monster_status_projection;
mod mount_olympus;
mod mutations;
mod paladin;
mod pantheons;
mod persistence;
mod pet_upkeep;
mod poison_needle;
mod prayer_study;
mod priest;
mod progression;
mod pyramidal_mound;
mod race_attribute_sustains;
mod random_dungeons;
mod ranger;
mod riding;
mod riding_bond;
mod ring_of_power;
mod snapshots;
mod sniper;
mod snow;
mod spectre_rules;
mod spectre_supplies;
pub(crate) mod support;
mod tasks;
mod thingol;
mod town;
mod trait_details;
mod vampire;
mod virtue_state;
mod wall_passage;
mod warrior_mage;
mod waste;
mod weapon_ego_activations;
mod weapon_proficiency;
mod weapon_traits;
mod world;
