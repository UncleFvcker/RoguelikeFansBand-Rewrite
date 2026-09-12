// SPDX-License-Identifier: MPL-2.0
// Ability effect and target projections for the client.

use rfb_content::{
    AbilityDefinition, AbilityEffectDefinition, AbilityRandomTargetDefinition,
    AbilitySpellPowerField, AbilityTargetDefinition, AbilityTargetModeDefinition,
    AbilityTerrainBeamOperationDefinition, SniperShotModeDefinition,
};
use rfb_protocol::{
    AbilityEffectSpecDto, AbilityRandomBranchSpecDto, AbilityRandomTargetDto,
    AbilityStatusStackingDto, AbilitySummonCandidateSpecDto, AbilityTerrainBeamOperationDto,
    ResistanceDto, SniperShotModeDto, TargetModeDto, TargetSpecDto,
};

use super::ability_scaling::{ability_has_spell_power_field, spell_power_value};
use super::{
    CRUSADE_ARREST_ABILITY_ID, ability_detect_subject_dto, ability_genocide_scope_dto,
    ability_status_stacking_dto, equipment_bonuses_dto, stat_modifiers_dto, weapon_brand_dto,
};
use crate::{
    effect::{STATUS_PARALYSIS, STATUS_SLEEP},
    resistance::{DamageType, ResistanceLevel},
};

pub(super) fn ability_effect_spec_dto(effect: &AbilityEffectDefinition) -> AbilityEffectSpecDto {
    match effect {
        AbilityEffectDefinition::CraftEnchant {
            maximum, increment, ..
        } => AbilityEffectSpecDto::CraftEnchant {
            maximum: *maximum,
            increment: *increment,
        },
        AbilityEffectDefinition::CraftItem => AbilityEffectSpecDto::CraftItem,
        AbilityEffectDefinition::PolishShield => AbilityEffectSpecDto::PolishShield,
        AbilityEffectDefinition::Mundanity => AbilityEffectSpecDto::Mundanity,
        AbilityEffectDefinition::ElementalBrand => AbilityEffectSpecDto::ElementalBrand,
        AbilityEffectDefinition::ElementalImmunity { duration_base } => {
            AbilityEffectSpecDto::ElementalImmunity {
                duration_base: *duration_base,
            }
        }
        AbilityEffectDefinition::LivingTrump => AbilityEffectSpecDto::LivingTrump,
        AbilityEffectDefinition::JumpDamage { .. }
        | AbilityEffectDefinition::BirdDrop
        | AbilityEffectDefinition::DraconianBreathDamage { .. } => {
            unreachable!("non-projected effects are resolved before player ability projection")
        }
        AbilityEffectDefinition::BlinkSelf { radius, .. } => {
            AbilityEffectSpecDto::BlinkSelf { radius: *radius }
        }
        AbilityEffectDefinition::BlinkTarget { radius } => {
            AbilityEffectSpecDto::BlinkTarget { radius: *radius }
        }
        AbilityEffectDefinition::TeleportSelf { minimum_distance } => {
            AbilityEffectSpecDto::TeleportSelf {
                minimum_distance: *minimum_distance,
            }
        }
        AbilityEffectDefinition::TeleportTarget => AbilityEffectSpecDto::TeleportTarget,
        AbilityEffectDefinition::TeleportLevel => AbilityEffectSpecDto::TeleportLevel,
        AbilityEffectDefinition::CreateStair {
            up_terrain_id,
            down_terrain_id,
        } => AbilityEffectSpecDto::CreateStair {
            up_terrain_id: up_terrain_id.clone(),
            down_terrain_id: down_terrain_id.clone(),
        },
        AbilityEffectDefinition::TeleportTown => AbilityEffectSpecDto::TeleportTown,
        AbilityEffectDefinition::SelfKnowledge => AbilityEffectSpecDto::SelfKnowledge,
        AbilityEffectDefinition::DimensionDoor { range } => {
            AbilityEffectSpecDto::DimensionDoor { range: *range }
        }
        AbilityEffectDefinition::Damage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
        } => AbilityEffectSpecDto::Damage {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            damage_type: DamageType::from(*damage_type).into(),
            final_damage_spell_power_bonus: None,
        },
        AbilityEffectDefinition::Malediction {
            damage_dice,
            damage_sides,
            damage_bonus,
        } => AbilityEffectSpecDto::Damage {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            damage_type: DamageType::HellFire.into(),
            final_damage_spell_power_bonus: None,
        },
        AbilityEffectDefinition::AreaDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            radius,
            target_category,
        } => AbilityEffectSpecDto::AreaDamage {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            damage_type: DamageType::from(*damage_type).into(),
            radius: *radius,
            target_category: target_category.clone(),
            final_damage_spell_power_bonus: None,
        },
        AbilityEffectDefinition::BeamDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            ..
        } => AbilityEffectSpecDto::BeamDamage {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            damage_type: DamageType::from(*damage_type).into(),
            final_damage_spell_power_bonus: None,
        },
        AbilityEffectDefinition::LightLine {
            damage_dice,
            damage_sides,
        } => AbilityEffectSpecDto::LightLine {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
        },
        AbilityEffectDefinition::LightArea {
            damage_dice,
            damage_sides,
            radius,
            ..
        } => AbilityEffectSpecDto::LightArea {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            radius: *radius,
            final_damage_spell_power_bonus: None,
        },
        AbilityEffectDefinition::BoltOrBeamDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            beam_chance_percent,
            ball_when_not_beam,
            ..
        } => AbilityEffectSpecDto::BoltOrBeamDamage {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            damage_type: DamageType::from(*damage_type).into(),
            beam_chance_percent: *beam_chance_percent,
            ball_when_not_beam: *ball_when_not_beam,
            final_damage_spell_power_bonus: None,
        },
        AbilityEffectDefinition::Stardust {
            damage_dice,
            damage_sides,
            count,
            deviation,
        } => AbilityEffectSpecDto::Stardust {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            count: *count,
            deviation: *deviation,
        },
        AbilityEffectDefinition::BoltOrAreaDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            area_from_level,
            radius,
            ..
        } => AbilityEffectSpecDto::BoltOrAreaDamage {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            damage_type: DamageType::from(*damage_type).into(),
            area_from_level: *area_from_level,
            radius: *radius,
            final_damage_spell_power_bonus: None,
        },
        AbilityEffectDefinition::ConeDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            radius,
        } => AbilityEffectSpecDto::ConeDamage {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            damage_type: DamageType::from(*damage_type).into(),
            radius: *radius,
            final_damage_spell_power_bonus: None,
        },
        AbilityEffectDefinition::BreathDamage {
            hp_percent,
            max_damage,
            damage_type,
            radius,
        } => AbilityEffectSpecDto::BreathDamage {
            hp_percent: *hp_percent,
            max_damage: *max_damage,
            damage_type: DamageType::from(*damage_type).into(),
            radius: *radius,
        },
        AbilityEffectDefinition::CurseDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_is_current_hp_percent,
            nonlethal,
        } => AbilityEffectSpecDto::CurseDamage {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            damage_is_current_hp_percent: *damage_is_current_hp_percent,
            nonlethal: *nonlethal,
        },
        AbilityEffectDefinition::DeathRay { power } => {
            AbilityEffectSpecDto::DeathRay { power: *power }
        }
        AbilityEffectDefinition::TeleportAway {
            minimum_distance,
            power,
            stop_at_actor,
            target_category,
        } => AbilityEffectSpecDto::TeleportAway {
            minimum_distance: *minimum_distance,
            power: *power,
            stop_at_actor: *stop_at_actor,
            target_category: target_category.clone(),
        },
        AbilityEffectDefinition::RechargeFromPlayer { power } => {
            AbilityEffectSpecDto::RechargeFromPlayer { power: *power }
        }
        AbilityEffectDefinition::Clairvoyance {
            telepathy_duration_ticks,
            telepathy_duration_dice,
            telepathy_duration_sides,
            ..
        } => AbilityEffectSpecDto::Clairvoyance {
            telepathy_duration_ticks: *telepathy_duration_ticks,
            telepathy_duration_dice: *telepathy_duration_dice,
            telepathy_duration_sides: *telepathy_duration_sides,
        },
        AbilityEffectDefinition::CallSunlight { vampire_damage } => {
            AbilityEffectSpecDto::CallSunlight {
                vampire_damage: *vampire_damage,
            }
        }
        AbilityEffectDefinition::NatureWrath => AbilityEffectSpecDto::NatureWrath,
        AbilityEffectDefinition::Probe => AbilityEffectSpecDto::Probe,
        AbilityEffectDefinition::CreateDoor { terrain_id } => AbilityEffectSpecDto::CreateDoor {
            terrain_id: terrain_id.clone(),
        },
        AbilityEffectDefinition::DeviceMastery {
            duration_base,
            device_power_bonus,
        } => AbilityEffectSpecDto::DeviceMastery {
            duration_base: *duration_base,
            device_power_bonus: *device_power_bonus,
        },
        AbilityEffectDefinition::Banish { maximum_distance } => AbilityEffectSpecDto::Banish {
            maximum_distance: *maximum_distance,
        },
        AbilityEffectDefinition::Invulnerability {
            duration_dice,
            duration_sides,
            duration_bonus,
        } => AbilityEffectSpecDto::Invulnerability {
            duration_dice: *duration_dice,
            duration_sides: *duration_sides,
            duration_bonus: *duration_bonus,
            duration_spell_power_bonus: None,
        },
        AbilityEffectDefinition::DrainResource { amount } => {
            AbilityEffectSpecDto::DrainResource { amount: *amount }
        }
        AbilityEffectDefinition::Amnesia => AbilityEffectSpecDto::Amnesia,
        AbilityEffectDefinition::DarkenRoom => AbilityEffectSpecDto::DarkenRoom,
        AbilityEffectDefinition::AggravateMonsters => AbilityEffectSpecDto::AggravateMonsters,
        AbilityEffectDefinition::Teleport => AbilityEffectSpecDto::Teleport,
        AbilityEffectDefinition::FetchItem {
            maximum_weight_tenths_pound,
        } => AbilityEffectSpecDto::FetchItem {
            maximum_weight_tenths_pound: *maximum_weight_tenths_pound,
        },
        AbilityEffectDefinition::ConsumeTerrain { nutrition } => {
            AbilityEffectSpecDto::ConsumeTerrain {
                nutrition: *nutrition,
            }
        }
        AbilityEffectDefinition::CreateItem {
            item_kind_id,
            quantity,
        } => AbilityEffectSpecDto::CreateItem {
            item_kind_id: item_kind_id.clone(),
            quantity: *quantity,
        },
        AbilityEffectDefinition::CreateAmmunition {
            item_kind_ids,
            quantity_minimum,
            quantity_maximum,
            source_item_tags,
            source_terrain_tags,
        } => AbilityEffectSpecDto::CreateAmmunition {
            item_kind_ids: item_kind_ids.clone(),
            quantity_minimum: *quantity_minimum,
            quantity_maximum: *quantity_maximum,
            source_item_tags: source_item_tags.clone(),
            source_terrain_tags: source_terrain_tags.clone(),
        },
        AbilityEffectDefinition::TransmuteItemToGold {
            value_divisor,
            unit_value_cap,
        } => AbilityEffectSpecDto::TransmuteItemToGold {
            value_divisor: *value_divisor,
            unit_value_cap: *unit_value_cap,
        },
        AbilityEffectDefinition::DrainItemMagic {
            base_power,
            level_multiplier,
            level_divisor,
        } => AbilityEffectSpecDto::DrainItemMagic {
            base_power: *base_power,
            level_multiplier: *level_multiplier,
            level_divisor: *level_divisor,
        },
        AbilityEffectDefinition::ReportMagic => AbilityEffectSpecDto::ReportMagic,
        AbilityEffectDefinition::Earthquake {
            radius,
            affect_chance_percent,
            floor_terrain_id,
            wall_terrain_ids,
        } => AbilityEffectSpecDto::Earthquake {
            radius: *radius,
            affect_chance_percent: *affect_chance_percent,
            floor_terrain_id: floor_terrain_id.clone(),
            wall_terrain_ids: wall_terrain_ids.clone(),
        },
        AbilityEffectDefinition::AreaDestruction {
            minimum_radius,
            maximum_radius,
            floor_terrain_id,
            wall_terrain_id,
            quartz_terrain_id,
            magma_terrain_id,
        } => AbilityEffectSpecDto::AreaDestruction {
            minimum_radius: *minimum_radius,
            maximum_radius: *maximum_radius,
            floor_terrain_id: floor_terrain_id.clone(),
            wall_terrain_id: wall_terrain_id.clone(),
            quartz_terrain_id: quartz_terrain_id.clone(),
            magma_terrain_id: magma_terrain_id.clone(),
        },
        AbilityEffectDefinition::SuppressMonsterReproduction {
            damage_dice,
            damage_sides,
            damage_bonus,
        } => AbilityEffectSpecDto::SuppressMonsterReproduction {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
        },
        AbilityEffectDefinition::MeleeThenTeleport {
            radius,
            failure_threshold,
        } => AbilityEffectSpecDto::MeleeThenTeleport {
            radius: *radius,
            failure_threshold: *failure_threshold,
        },
        AbilityEffectDefinition::PolymorphSelf => AbilityEffectSpecDto::PolymorphSelf,
        AbilityEffectDefinition::PolymorphTarget => AbilityEffectSpecDto::PolymorphTarget,
        AbilityEffectDefinition::SwapPosition => AbilityEffectSpecDto::SwapPosition,
        AbilityEffectDefinition::Recall {
            delay_dice,
            delay_sides,
            delay_bonus,
        } => AbilityEffectSpecDto::Recall {
            delay_dice: *delay_dice,
            delay_sides: *delay_sides,
            delay_bonus: *delay_bonus,
        },
        AbilityEffectDefinition::ResistElements {
            duration_dice,
            duration_sides,
            duration_bonus,
        } => AbilityEffectSpecDto::ResistElements {
            duration_dice: *duration_dice,
            duration_sides: *duration_sides,
            duration_bonus: *duration_bonus,
        },
        AbilityEffectDefinition::Summon {
            actor_kind_id,
            count,
            radius,
            duration_turns,
            hostile,
        } => AbilityEffectSpecDto::Summon {
            actor_kind_id: actor_kind_id.clone(),
            count: *count,
            radius: *radius,
            duration_turns: *duration_turns,
            hostile: *hostile,
        },
        AbilityEffectDefinition::SummonCategory {
            category,
            upgraded_category,
            upgrade_at_level,
            maximum_level,
            count_dice,
            count_sides,
            count_bonus,
            maximum_count,
            batch_candidates,
            hostile_chance_percent,
            friendly_group_chance_percent,
            hostile_group_chance_percent,
            group_count_dice,
            group_count_sides,
            group_count_bonus,
            allow_unique_hostile,
            radius,
            duration_turns,
        } => AbilityEffectSpecDto::SummonCategory {
            category: category.clone(),
            upgraded_category: upgraded_category.clone(),
            upgrade_at_level: *upgrade_at_level,
            maximum_level: *maximum_level,
            count_dice: *count_dice,
            count_sides: *count_sides,
            count_bonus: *count_bonus,
            maximum_count: *maximum_count,
            batch_candidates: batch_candidates
                .iter()
                .map(|candidate| AbilitySummonCandidateSpecDto {
                    actor_kind_id: candidate.actor_kind_id.clone(),
                    weight: candidate.weight,
                })
                .collect(),
            hostile_chance_percent: *hostile_chance_percent,
            friendly_group_chance_percent: *friendly_group_chance_percent,
            hostile_group_chance_percent: *hostile_group_chance_percent,
            group_count_dice: *group_count_dice,
            group_count_sides: *group_count_sides,
            group_count_bonus: *group_count_bonus,
            allow_unique_hostile: *allow_unique_hostile,
            radius: *radius,
            duration_turns: *duration_turns,
        },
        AbilityEffectDefinition::NatureGate {
            animal_category,
            hound_category,
            hydra_category,
            ent_actor_kind_id,
            radius,
            duration_turns,
        } => AbilityEffectSpecDto::NatureGate {
            animal_category: animal_category.clone(),
            hound_category: hound_category.clone(),
            hydra_category: hydra_category.clone(),
            ent_actor_kind_id: ent_actor_kind_id.clone(),
            radius: *radius,
            duration_turns: *duration_turns,
        },
        AbilityEffectDefinition::DemonSummoning => AbilityEffectSpecDto::DemonSummoning,
        AbilityEffectDefinition::AngelSummoning => AbilityEffectSpecDto::AngelSummoning,
        AbilityEffectDefinition::BanishEvil => AbilityEffectSpecDto::BanishEvil { power: 0 },
        AbilityEffectDefinition::Evocation => AbilityEffectSpecDto::Evocation {
            damage: 0,
            power: 0,
        },
        AbilityEffectDefinition::BlessWeapon => AbilityEffectSpecDto::BlessWeapon,
        AbilityEffectDefinition::WrathOfGod { damage } => AbilityEffectSpecDto::WrathOfGod {
            damage: damage.unwrap_or(0),
            radius: 2,
            minimum_count: 11,
            maximum_count: 20,
        },
        AbilityEffectDefinition::DivineIntervention => AbilityEffectSpecDto::DivineIntervention {
            adjacent_damage: 0,
            visible_damage: 0,
            healing: 0,
            control_power: 0,
            stun_duration_ticks: 0,
        },
        AbilityEffectDefinition::Crusade => AbilityEffectSpecDto::Crusade {
            charm_power: 0,
            summon_attempts: 12,
        },
        AbilityEffectDefinition::InsanityCircle {
            damage_bonus,
            control_power,
            radius,
        } => AbilityEffectSpecDto::InsanityCircle {
            damage_bonus: *damage_bonus,
            control_power: *control_power,
            radius: *radius,
        },
        AbilityEffectDefinition::ExplodePets => AbilityEffectSpecDto::ExplodePets,
        AbilityEffectDefinition::SummonGreaterDemon {
            corpse_item_kind_id,
            radius,
        } => AbilityEffectSpecDto::SummonGreaterDemon {
            corpse_item_kind_id: corpse_item_kind_id.clone(),
            radius: *radius,
        },
        AbilityEffectDefinition::Hellfire {
            damage_bonus,
            radius,
            backlash_dice,
            backlash_sides,
            backlash_bonus,
        } => AbilityEffectSpecDto::Hellfire {
            damage_bonus: *damage_bonus,
            radius: *radius,
            backlash_dice: *backlash_dice,
            backlash_sides: *backlash_sides,
            backlash_bonus: *backlash_bonus,
        },
        AbilityEffectDefinition::LavaFlow {
            damage_dice,
            damage_sides,
            damage_bonus,
            radius,
            target_terrain_id,
        } => AbilityEffectSpecDto::LavaFlow {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            radius: *radius,
            target_terrain_id: target_terrain_id.clone(),
            final_damage_spell_power_bonus: None,
        },
        AbilityEffectDefinition::DoomHand => AbilityEffectSpecDto::DoomHand,
        AbilityEffectDefinition::Sanctuary { power, radius } => AbilityEffectSpecDto::Sanctuary {
            power: *power,
            radius: *radius,
        },
        AbilityEffectDefinition::Detect {
            subject,
            category,
            radius,
            persistent,
            through_walls,
        } => AbilityEffectSpecDto::Detect {
            subject: ability_detect_subject_dto(*subject),
            category: category.clone(),
            radius: *radius,
            persistent: *persistent,
            through_walls: *through_walls,
        },
        AbilityEffectDefinition::RefuelEquippedLight {
            maximum_fraction_divisor,
        } => AbilityEffectSpecDto::RefuelEquippedLight {
            maximum_fraction_divisor: *maximum_fraction_divisor,
        },
        AbilityEffectDefinition::TransformTerrain {
            source_terrain_ids,
            target_terrain_id,
            radius,
        } => AbilityEffectSpecDto::TransformTerrain {
            source_terrain_ids: source_terrain_ids.clone(),
            target_terrain_id: target_terrain_id.clone(),
            radius: *radius,
        },
        AbilityEffectDefinition::CreateAdjacentTerrain {
            source_terrain_ids,
            target_terrain_id,
        } => AbilityEffectSpecDto::CreateAdjacentTerrain {
            source_terrain_ids: source_terrain_ids.clone(),
            target_terrain_id: target_terrain_id.clone(),
        },
        AbilityEffectDefinition::CreateCurrentTerrain {
            source_terrain_ids,
            target_terrain_id,
        } => AbilityEffectSpecDto::CreateCurrentTerrain {
            source_terrain_ids: source_terrain_ids.clone(),
            target_terrain_id: target_terrain_id.clone(),
        },
        AbilityEffectDefinition::TerrainBeam { operation } => AbilityEffectSpecDto::TerrainBeam {
            operation: match operation {
                AbilityTerrainBeamOperationDefinition::JamDoors => {
                    AbilityTerrainBeamOperationDto::JamDoors
                }
                AbilityTerrainBeamOperationDefinition::DestroyTrapsAndDoors => {
                    AbilityTerrainBeamOperationDto::DestroyTrapsAndDoors
                }
                AbilityTerrainBeamOperationDefinition::StoneToMud => {
                    AbilityTerrainBeamOperationDto::StoneToMud
                }
            },
        },
        AbilityEffectDefinition::ApplyStatus {
            status_kind_id,
            intensity,
            duration_ticks,
            duration_dice,
            duration_sides,
            stacking,
            resistance_type,
            power,
            granted_resistances,
            granted_brands,
            granted_modifiers,
            granted_equipment_bonuses,
            granted_status_immunities,
            granted_race_id,
            grants_wall_passage,
            incoming_damage_percent,
        } => AbilityEffectSpecDto::ApplyStatus {
            status_kind_id: status_kind_id.clone(),
            intensity: *intensity,
            duration_ticks: *duration_ticks,
            duration_dice: *duration_dice,
            duration_sides: *duration_sides,
            stacking: ability_status_stacking_dto(*stacking),
            resistance_type: resistance_type.map(DamageType::from).map(Into::into),
            power: *power,
            granted_resistances: granted_resistances
                .iter()
                .map(|(damage_type, level)| ResistanceDto {
                    damage_type: DamageType::from(*damage_type).into(),
                    level: ResistanceLevel::from(*level).into(),
                })
                .collect(),
            granted_modifiers: stat_modifiers_dto(granted_modifiers),
            granted_equipment_bonuses: equipment_bonuses_dto(granted_equipment_bonuses),
            granted_status_immunities: granted_status_immunities.iter().cloned().collect(),
            granted_race_id: granted_race_id.clone(),
            grants_wall_passage: *grants_wall_passage,
            incoming_damage_percent: *incoming_damage_percent,
            granted_brands: granted_brands
                .iter()
                .copied()
                .map(weapon_brand_dto)
                .collect(),
        },
        AbilityEffectDefinition::RemoveStatus { status_kind_id } => {
            AbilityEffectSpecDto::RemoveStatus {
                status_kind_id: status_kind_id.clone(),
            }
        }
        AbilityEffectDefinition::Control { category, power } => AbilityEffectSpecDto::Control {
            category: category.clone(),
            power: *power,
        },
        AbilityEffectDefinition::SniperShot { mode } => AbilityEffectSpecDto::SniperShot {
            mode: match mode {
                SniperShotModeDefinition::Shining => SniperShotModeDto::Shining,
                SniperShotModeDefinition::Retreat => SniperShotModeDto::Retreat,
                SniperShotModeDefinition::Disarm => SniperShotModeDto::Disarm,
                SniperShotModeDefinition::Burning => SniperShotModeDto::Burning,
                SniperShotModeDefinition::Shatter => SniperShotModeDto::Shatter,
                SniperShotModeDefinition::Freezing => SniperShotModeDto::Freezing,
                SniperShotModeDefinition::Knockback => SniperShotModeDto::Knockback,
                SniperShotModeDefinition::Piercing => SniperShotModeDto::Piercing,
                SniperShotModeDefinition::Evil => SniperShotModeDto::Evil,
                SniperShotModeDefinition::Holy => SniperShotModeDto::Holy,
                SniperShotModeDefinition::Exploding => SniperShotModeDto::Exploding,
                SniperShotModeDefinition::Double => SniperShotModeDto::Double,
                SniperShotModeDefinition::Thunder => SniperShotModeDto::Thunder,
                SniperShotModeDefinition::Needle => SniperShotModeDto::Needle,
                SniperShotModeDefinition::Final => SniperShotModeDto::Final,
            },
        },
        AbilityEffectDefinition::MeleeAdjacent => AbilityEffectSpecDto::MeleeAdjacent,
        AbilityEffectDefinition::ChargeThrough => AbilityEffectSpecDto::ChargeThrough,
        AbilityEffectDefinition::DuelistChallenge => AbilityEffectSpecDto::DuelistChallenge,
        AbilityEffectDefinition::DuelistCharge => AbilityEffectSpecDto::DuelistCharge,
        AbilityEffectDefinition::DuelistAcrobaticCharge => {
            AbilityEffectSpecDto::DuelistAcrobaticCharge
        }
        AbilityEffectDefinition::DuelistPhaseCharge => AbilityEffectSpecDto::DuelistPhaseCharge,
        AbilityEffectDefinition::DuelistDartingDuel => AbilityEffectSpecDto::DuelistDartingDuel,
        AbilityEffectDefinition::Strafing => AbilityEffectSpecDto::Strafing,
        AbilityEffectDefinition::DuelistDisengage => AbilityEffectSpecDto::DuelistDisengage,
        AbilityEffectDefinition::DuelistIsolation => AbilityEffectSpecDto::DuelistIsolation,

        AbilityEffectDefinition::SmashTrap => AbilityEffectSpecDto::SmashTrap,
        AbilityEffectDefinition::DraconianStrike { .. } => AbilityEffectSpecDto::MeleeAdjacent,
        AbilityEffectDefinition::ProbeMonsters => AbilityEffectSpecDto::ProbeMonsters,
        AbilityEffectDefinition::Concentrate => AbilityEffectSpecDto::Concentrate,
        AbilityEffectDefinition::Rodeo => AbilityEffectSpecDto::Rodeo,
        AbilityEffectDefinition::DrainLife {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            target_category,
            repeat,
            feeds,
        } => AbilityEffectSpecDto::DrainLife {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            damage_type: DamageType::from(*damage_type).into(),
            target_category: target_category.clone(),
            repeat: *repeat,
            feeds: *feeds,
            final_damage_spell_power_bonus: None,
        },
        AbilityEffectDefinition::Genocide {
            scope,
            power,
            radius,
            target_category,
            fatigue,
            ..
        } => AbilityEffectSpecDto::Genocide {
            scope: ability_genocide_scope_dto(*scope),
            power: *power,
            radius: *radius,
            target_category: target_category.clone(),
            fatigue: *fatigue,
        },
        AbilityEffectDefinition::IdentifyItem {
            full_identify_power,
            full_identify_roll_sides,
        } => AbilityEffectSpecDto::IdentifyItem {
            full_identify_power: *full_identify_power,
            full_identify_roll_sides: *full_identify_roll_sides,
        },
        AbilityEffectDefinition::IdentifyOrMassIdentify { mass, .. } => {
            if *mass {
                AbilityEffectSpecDto::MassIdentify
            } else {
                AbilityEffectSpecDto::IdentifyItem {
                    full_identify_power: 0,
                    full_identify_roll_sides: 0,
                }
            }
        }
        AbilityEffectDefinition::RestoreVitality { life_force, .. } => {
            AbilityEffectSpecDto::RestoreVitality {
                life_force: *life_force,
            }
        }
        AbilityEffectDefinition::HealthToMana => AbilityEffectSpecDto::HealthToMana {
            hit_point_cost: 0,
            mana_divisor: 5,
        },
        AbilityEffectDefinition::ManaToHealth => AbilityEffectSpecDto::ManaToHealth {
            mana_cost: 0,
            healing: 0,
        },
        AbilityEffectDefinition::ClearMind => AbilityEffectSpecDto::ClearMind { amount: 2 },
        AbilityEffectDefinition::Precognition => AbilityEffectSpecDto::Precognition {
            detect_invisible: false,
            detect_traps_and_doors: false,
            detect_objects_and_stairs: false,
            maps_area: false,
            illuminates_floor: false,
            telepathy_minimum_ticks: 0,
            telepathy_maximum_ticks: 0,
        },
        AbilityEffectDefinition::Psychometry => AbilityEffectSpecDto::Psychometry,
        AbilityEffectDefinition::MindArmor => AbilityEffectSpecDto::MindArmor {
            minimum_duration_ticks: 0,
            maximum_duration_ticks: 0,
            armor_class: 50,
            resistances: Vec::new(),
        },
        AbilityEffectDefinition::Adrenaline => AbilityEffectSpecDto::Adrenaline {
            minimum_duration_ticks: 0,
            maximum_duration_ticks: 0,
            healing_if_not_already_hasted_and_heroic: 0,
        },
        AbilityEffectDefinition::Domination { power, mass } => AbilityEffectSpecDto::Domination {
            power: *power,
            mass: *mass,
        },
        AbilityEffectDefinition::AlterReality => AbilityEffectSpecDto::AlterReality,
        AbilityEffectDefinition::AnimateDead {
            actor_kind_id,
            corpse_item_kind_id,
            radius,
            count,
            ..
        } => AbilityEffectSpecDto::AnimateDead {
            actor_kind_id: actor_kind_id.clone(),
            corpse_item_kind_id: corpse_item_kind_id.clone(),
            radius: *radius,
            count: *count,
        },
        AbilityEffectDefinition::Heal { amount } => AbilityEffectSpecDto::Heal { amount: *amount },
        AbilityEffectDefinition::HealDice { dice, sides } => AbilityEffectSpecDto::HealDice {
            dice: *dice,
            sides: *sides,
            final_healing_spell_power_bonus: None,
        },
        AbilityEffectDefinition::RemoveEquippedCurses { include_heavy } => {
            AbilityEffectSpecDto::RemoveEquippedCurses {
                include_heavy: *include_heavy,
            }
        }
        AbilityEffectDefinition::BeginFasting => AbilityEffectSpecDto::BeginFasting,
        AbilityEffectDefinition::TurnUndead { power } => {
            AbilityEffectSpecDto::TurnUndead { power: *power }
        }
        AbilityEffectDefinition::SustainAttributes { duration_ticks } => {
            AbilityEffectSpecDto::SustainAttributes {
                duration_ticks: *duration_ticks,
            }
        }
        AbilityEffectDefinition::CureMutation => AbilityEffectSpecDto::CureMutation,
        AbilityEffectDefinition::ReduceStatus {
            status_kind_id,
            amount,
            current_divisor,
            remaining_divisor,
        } => AbilityEffectSpecDto::ReduceStatus {
            status_kind_id: status_kind_id.clone(),
            amount: *amount,
            current_divisor: *current_divisor,
            remaining_divisor: *remaining_divisor,
        },
        AbilityEffectDefinition::SatisfyHunger => AbilityEffectSpecDto::SatisfyHunger,
        AbilityEffectDefinition::DevourFlesh {
            maximum_hp_divisor,
            bleeding_amount,
        } => AbilityEffectSpecDto::DevourFlesh {
            maximum_hp_divisor: *maximum_hp_divisor,
            bleeding_amount: *bleeding_amount,
        },
        AbilityEffectDefinition::Vomit => AbilityEffectSpecDto::Vomit,
        AbilityEffectDefinition::VisibleDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            target_category,
            unlife_change_on_hit,
        } => AbilityEffectSpecDto::VisibleDamage {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            damage_type: DamageType::from(*damage_type).into(),
            target_category: target_category.clone(),
            unlife_change_on_hit: *unlife_change_on_hit,
            final_damage_spell_power_bonus: None,
        },
        AbilityEffectDefinition::VisibleApplyStatus {
            status_kind_id,
            intensity,
            duration_ticks,
            duration_dice,
            duration_sides,
            stacking,
            resistance_type,
            power,
            target_category,
        } => AbilityEffectSpecDto::VisibleApplyStatus {
            status_kind_id: status_kind_id.clone(),
            intensity: *intensity,
            duration_ticks: *duration_ticks,
            duration_dice: *duration_dice,
            duration_sides: *duration_sides,
            stacking: ability_status_stacking_dto(*stacking),
            resistance_type: resistance_type.map(DamageType::from).map(Into::into),
            power: *power,
            target_category: target_category.clone(),
        },
        AbilityEffectDefinition::Entangle {
            power,
            duration_ticks,
        } => AbilityEffectSpecDto::Entangle {
            power: *power,
            duration_ticks: *duration_ticks,
        },
        AbilityEffectDefinition::MassSleepOrStasis { stasis, power, .. } => {
            AbilityEffectSpecDto::VisibleApplyStatus {
                status_kind_id: if *stasis {
                    STATUS_PARALYSIS.to_owned()
                } else {
                    STATUS_SLEEP.to_owned()
                },
                intensity: 1,
                duration_ticks: if *stasis { 20 } else { 500 },
                duration_dice: 0,
                duration_sides: 0,
                stacking: if *stasis {
                    AbilityStatusStackingDto::Extend
                } else {
                    AbilityStatusStackingDto::KeepStrongest
                },
                resistance_type: None,
                power: Some(*power),
                target_category: None,
            }
        }
        AbilityEffectDefinition::SleepingDust { .. } => {
            unreachable!("sleeping dust must be level-scaled before projection")
        }
        AbilityEffectDefinition::BrandWeapon {
            affix_id,
            brand,
            resistance,
        } => AbilityEffectSpecDto::BrandWeapon {
            affix_id: affix_id.clone(),
            brand: brand.map(weapon_brand_dto),
            resistance: resistance.map(DamageType::from).map(Into::into),
        },
        AbilityEffectDefinition::ProtectFromCorrosion => AbilityEffectSpecDto::ProtectFromCorrosion,
        AbilityEffectDefinition::RandomChoice {
            roll_sides,
            level_bonus_divisor,
            branches,
        } => AbilityEffectSpecDto::RandomChoice {
            roll_sides: *roll_sides,
            level_bonus_divisor: *level_bonus_divisor,
            branches: branches
                .iter()
                .map(|branch| AbilityRandomBranchSpecDto {
                    maximum_roll: branch.maximum_roll,
                    target: match branch.target {
                        AbilityRandomTargetDefinition::CastTarget => {
                            AbilityRandomTargetDto::CastTarget
                        }
                        AbilityRandomTargetDefinition::SelfTarget => {
                            AbilityRandomTargetDto::SelfTarget
                        }
                    },
                    effect: Box::new(ability_effect_spec_dto(&branch.effect)),
                })
                .collect(),
            roll_spell_power_bonus: None,
        },
        AbilityEffectDefinition::NoOp { reason } => AbilityEffectSpecDto::NoOp {
            reason: reason.clone(),
        },
        AbilityEffectDefinition::Sequence { effects } => AbilityEffectSpecDto::Sequence {
            effects: effects.iter().map(ability_effect_spec_dto).collect(),
        },
    }
}

pub(super) fn player_ability_effect_spec_dto(
    ability: &AbilityDefinition,
    effect_index: u8,
    effect: &AbilityEffectDefinition,
    level: u16,
    spell_damage_bonus: u16,
) -> AbilityEffectSpecDto {
    let mut spec = ability_effect_spec_dto(effect);
    super::abilities::mindcraft::project_mindcraft_effect(
        &mut spec,
        level,
        ability.spell_power_bonus,
    );
    match &mut spec {
        AbilityEffectSpecDto::Evocation { damage, power } => {
            *power = spell_power_value(u64::from(level) * 4, ability.spell_power_bonus)
                .min(u64::from(u16::MAX)) as u16;
            *damage = spell_power_value(
                u64::from(level) * 4 + u64::from(spell_damage_bonus),
                ability.spell_power_bonus,
            )
            .min(u64::from(u16::MAX)) as u16;
        }
        AbilityEffectSpecDto::HealthToMana { hit_point_cost, .. } => {
            *hit_point_cost = u32::from(level);
        }
        AbilityEffectSpecDto::ManaToHealth { mana_cost, healing } => {
            *mana_cost = u32::from(level / 5);
            *healing = u32::from(level);
        }
        AbilityEffectSpecDto::ClearMind { amount } => {
            *amount = super::player_abilities::clear_mind_recovery_amount(level);
        }
        AbilityEffectSpecDto::BanishEvil { power } => {
            *power =
                spell_power_value(100, ability.spell_power_bonus).min(u64::from(u16::MAX)) as u16;
        }
        AbilityEffectSpecDto::WrathOfGod { damage, .. } => {
            let raw = if let AbilityEffectDefinition::WrathOfGod {
                damage: Some(damage),
            } = effect
            {
                *damage
            } else {
                level
                    .saturating_mul(3)
                    .saturating_add(25)
                    .saturating_add(spell_damage_bonus)
            };
            *damage = spell_power_value(u64::from(raw), ability.spell_power_bonus)
                .min(u64::from(u16::MAX)) as u16;
        }
        AbilityEffectSpecDto::DivineIntervention {
            adjacent_damage,
            visible_damage,
            healing,
            control_power,
            stun_duration_ticks,
        } => {
            *adjacent_damage = spell_power_value(
                u64::from(level.saturating_mul(11)),
                ability.spell_power_bonus,
            )
            .min(u64::from(u16::MAX)) as u16;
            *visible_damage = spell_power_value(
                u64::from(level.saturating_mul(4).saturating_add(spell_damage_bonus)),
                ability.spell_power_bonus,
            )
            .min(u64::from(u16::MAX)) as u16;
            *healing =
                spell_power_value(100, ability.spell_power_bonus).min(u64::from(u16::MAX)) as u16;
            *control_power = spell_power_value(
                u64::from(level.saturating_mul(4)),
                ability.spell_power_bonus,
            )
            .min(u64::from(u16::MAX)) as u16;
            *stun_duration_ticks = u32::from(5_u16.saturating_add(level / 5));
        }
        AbilityEffectSpecDto::Crusade { charm_power, .. } => {
            *charm_power = level.saturating_mul(4);
        }
        _ => {}
    }
    if ability.id == CRUSADE_ARREST_ABILITY_ID
        && let AbilityEffectSpecDto::ApplyStatus {
            power: Some(power), ..
        } = &mut spec
    {
        *power = u16::try_from(spell_power_value(
            u64::from(*power),
            ability.spell_power_bonus,
        ))
        .expect("validated Arrest display power must fit u16");
    }
    if ability.spell_power_bonus == 0 {
        return spec;
    }
    if ability_has_spell_power_field(ability, effect_index, AbilitySpellPowerField::FinalDamage) {
        let bonus = Some(ability.spell_power_bonus);
        match &mut spec {
            AbilityEffectSpecDto::Damage {
                final_damage_spell_power_bonus,
                ..
            }
            | AbilityEffectSpecDto::AreaDamage {
                final_damage_spell_power_bonus,
                ..
            }
            | AbilityEffectSpecDto::LavaFlow {
                final_damage_spell_power_bonus,
                ..
            }
            | AbilityEffectSpecDto::BeamDamage {
                final_damage_spell_power_bonus,
                ..
            }
            | AbilityEffectSpecDto::LightArea {
                final_damage_spell_power_bonus,
                ..
            }
            | AbilityEffectSpecDto::BoltOrBeamDamage {
                final_damage_spell_power_bonus,
                ..
            }
            | AbilityEffectSpecDto::BoltOrAreaDamage {
                final_damage_spell_power_bonus,
                ..
            }
            | AbilityEffectSpecDto::ConeDamage {
                final_damage_spell_power_bonus,
                ..
            }
            | AbilityEffectSpecDto::DrainLife {
                final_damage_spell_power_bonus,
                ..
            }
            | AbilityEffectSpecDto::VisibleDamage {
                final_damage_spell_power_bonus,
                ..
            } => *final_damage_spell_power_bonus = bonus,
            AbilityEffectSpecDto::RandomChoice { branches, .. } => {
                for branch in branches {
                    match branch.effect.as_mut() {
                        AbilityEffectSpecDto::Damage {
                            final_damage_spell_power_bonus,
                            ..
                        }
                        | AbilityEffectSpecDto::AreaDamage {
                            final_damage_spell_power_bonus,
                            ..
                        }
                        | AbilityEffectSpecDto::BeamDamage {
                            final_damage_spell_power_bonus,
                            ..
                        }
                        | AbilityEffectSpecDto::LightArea {
                            final_damage_spell_power_bonus,
                            ..
                        }
                        | AbilityEffectSpecDto::BoltOrBeamDamage {
                            final_damage_spell_power_bonus,
                            ..
                        }
                        | AbilityEffectSpecDto::BoltOrAreaDamage {
                            final_damage_spell_power_bonus,
                            ..
                        }
                        | AbilityEffectSpecDto::ConeDamage {
                            final_damage_spell_power_bonus,
                            ..
                        }
                        | AbilityEffectSpecDto::DrainLife {
                            final_damage_spell_power_bonus,
                            ..
                        }
                        | AbilityEffectSpecDto::VisibleDamage {
                            final_damage_spell_power_bonus,
                            ..
                        } => *final_damage_spell_power_bonus = bonus,
                        _ => unreachable!("validated random damage branch must project damage"),
                    }
                }
            }
            _ => unreachable!("validated final damage marker must project a damage effect"),
        }
    }
    if ability_has_spell_power_field(ability, effect_index, AbilitySpellPowerField::FinalHealing) {
        let AbilityEffectSpecDto::HealDice {
            final_healing_spell_power_bonus,
            ..
        } = &mut spec
        else {
            unreachable!("validated final healing marker must project a healing-dice effect");
        };
        *final_healing_spell_power_bonus = Some(ability.spell_power_bonus);
    }
    if ability_has_spell_power_field(
        ability,
        effect_index,
        AbilitySpellPowerField::RandomChoiceRoll,
    ) {
        let AbilityEffectSpecDto::RandomChoice {
            roll_spell_power_bonus,
            ..
        } = &mut spec
        else {
            unreachable!("validated random roll marker must project a random choice effect");
        };
        *roll_spell_power_bonus = Some(ability.spell_power_bonus);
    }
    if ability_has_spell_power_field(
        ability,
        effect_index,
        AbilitySpellPowerField::InvulnerabilityDuration,
    ) {
        let AbilityEffectSpecDto::Invulnerability {
            duration_spell_power_bonus,
            ..
        } = &mut spec
        else {
            unreachable!("validated invulnerability marker must project invulnerability");
        };
        *duration_spell_power_bonus = Some(ability.spell_power_bonus);
    }
    spec
}

pub(super) fn ability_target_spec_dto(ability: &AbilityDefinition) -> TargetSpecDto {
    target_spec_dto(&ability.target)
}

pub(super) fn target_spec_dto(target: &AbilityTargetDefinition) -> TargetSpecDto {
    TargetSpecDto {
        modes: target
            .modes
            .iter()
            .map(|mode| match mode {
                AbilityTargetModeDefinition::Direction => TargetModeDto::Direction,
                AbilityTargetModeDefinition::Position => TargetModeDto::Position,
                AbilityTargetModeDefinition::Entity => TargetModeDto::Entity,
                AbilityTargetModeDefinition::Item => TargetModeDto::Item,
                AbilityTargetModeDefinition::Element => TargetModeDto::Element,
                AbilityTargetModeDefinition::Town => TargetModeDto::Town,
                AbilityTargetModeDefinition::SelfTarget => TargetModeDto::SelfTarget,
            })
            .collect(),
        range: target.range,
        requires_line_of_effect: target.requires_line_of_effect,
    }
}
