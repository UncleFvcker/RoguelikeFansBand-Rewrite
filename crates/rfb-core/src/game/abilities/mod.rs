// SPDX-License-Identifier: MPL-2.0

mod berserker;
mod book_magic;
mod casting;
mod compound;
mod control;
mod damage;
mod duelist;
mod duelist_choices;
mod items;
pub(in crate::game) mod mindcraft;
mod restoration;
mod summoning;
mod targeting;
pub(in crate::game) mod terrain;
mod travel;

pub(super) use casting::nature_wrath_direction_roll;
pub(super) use targeting::AbilityTargetPlan;

use crate::error::CoreError;
use crate::event::DomainEvent;
use crate::game::Game;
use crate::game::terrain::TerrainChangeSource;
use rfb_content::{AbilityDefinition, AbilityEffectDefinition, AbilityGenocideScopeDefinition};
use rfb_protocol::Position;
use std::collections::BTreeSet;

impl Game {
    pub(super) fn resolve_player_ability_effect(
        &mut self,
        ability: AbilityDefinition,
        target_plan: AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<Option<Position>, CoreError> {
        match (ability.effect.clone(), target_plan) {
            (
                AbilityEffectDefinition::ElementalBrand
                | AbilityEffectDefinition::ElementalImmunity { .. },
                AbilityTargetPlan::Element { element },
            ) => self.resolve_player_elemental_enchantment(&ability, element, events),
            (AbilityEffectDefinition::LivingTrump, AbilityTargetPlan::SelfTarget) => {
                let controlled =
                    self.rng.bounded(7) == 0 || self.floor_depth(&self.current_floor_id) == 0;
                self.gain_mutation(
                    if controlled {
                        "rfb.mutation.teleport"
                    } else {
                        "rfb.mutation.teleport-rnd"
                    },
                    events,
                );
            }
            (
                AbilityEffectDefinition::CraftEnchant { .. }
                | AbilityEffectDefinition::BlessWeapon
                | AbilityEffectDefinition::CraftItem
                | AbilityEffectDefinition::PolishShield
                | AbilityEffectDefinition::Mundanity,
                AbilityTargetPlan::Item { item_id },
            ) => self.resolve_player_craft_item_effect(&ability, &item_id, events)?,
            (
                AbilityEffectDefinition::DuelistChallenge,
                AbilityTargetPlan::DuelistChallenge { target_entity_id },
            ) => {
                self.resolve_duelist_challenge(target_entity_id, events);
            }
            (AbilityEffectDefinition::Strafing, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_teleport_with_range(
                    &ability.id,
                    10,
                    true,
                    false,
                    None,
                    events,
                    changed,
                );
            }
            (AbilityEffectDefinition::DuelistDisengage, AbilityTargetPlan::SelfTarget) => {
                let excluded = self.duelist_target_id.clone();
                self.resolve_player_teleport_with_range(
                    &ability.id,
                    100,
                    false,
                    false,
                    excluded.as_deref(),
                    events,
                    changed,
                );
                self.duelist_target_id = None;
                events.push(DomainEvent::DuelistChallengeCleared);
            }
            (AbilityEffectDefinition::DuelistIsolation, AbilityTargetPlan::SelfTarget) => {
                self.resolve_duelist_isolation(&ability.id, events, changed);
            }
            (
                AbilityEffectDefinition::DuelistCharge
                | AbilityEffectDefinition::DuelistAcrobaticCharge
                | AbilityEffectDefinition::DuelistPhaseCharge
                | AbilityEffectDefinition::DuelistDartingDuel,
                AbilityTargetPlan::SelfTarget,
            ) => {
                return self.resolve_duelist_charge(&ability, events, changed, removed_entities);
            }
            (AbilityEffectDefinition::ChargeThrough, AbilityTargetPlan::Step { direction }) => {
                return self.resolve_player_charge_through(
                    direction,
                    events,
                    changed,
                    removed_entities,
                );
            }
            (AbilityEffectDefinition::SmashTrap, AbilityTargetPlan::Step { direction }) => {
                let energy = self.player.energy_need;
                let step = self.resolve_local_player_step(
                    direction,
                    true,
                    events,
                    changed,
                    removed_entities,
                )?;
                self.player.energy_need = energy;
                return Ok(step.map_translation);
            }
            (AbilityEffectDefinition::Teleport, AbilityTargetPlan::Teleport { destination }) => {
                self.resolve_player_teleport_effect(&ability, destination, events, changed);
            }
            (
                AbilityEffectDefinition::BlinkSelf { .. },
                AbilityTargetPlan::RandomTeleport { candidates },
            ) => {
                self.resolve_player_random_teleport_effect(&ability, candidates, events, changed);
            }
            (
                AbilityEffectDefinition::DimensionDoor { .. },
                AbilityTargetPlan::DimensionDoor {
                    requested,
                    destination_valid,
                    fallback_candidates,
                },
            ) => self.resolve_player_dimension_door_effect(
                &ability,
                requested,
                destination_valid,
                fallback_candidates,
                events,
                changed,
            ),
            (AbilityEffectDefinition::TeleportTown, AbilityTargetPlan::Town { town_id }) => {
                self.resolve_player_teleport_town_effect(&ability, &town_id, events)?
            }
            (AbilityEffectDefinition::CreateStair { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_create_stair_effect(&ability, events, changed);
            }
            (AbilityEffectDefinition::CreateItem { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_create_item_effect(&ability, events, changed)?;
            }
            (AbilityEffectDefinition::SelfKnowledge, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_self_knowledge_effect(&ability, events);
            }
            (AbilityEffectDefinition::Probe, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_probe_effect(&ability, events, changed);
            }
            (AbilityEffectDefinition::CreateDoor { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_create_door_effect(&ability, events, changed);
            }
            (AbilityEffectDefinition::DeviceMastery { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_device_mastery_effect(&ability, events);
            }
            (AbilityEffectDefinition::Banish { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_banish_effect(&ability, events, changed);
            }
            (AbilityEffectDefinition::Invulnerability { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_invulnerability_effect(&ability, events);
            }
            (AbilityEffectDefinition::LightArea { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_light_area_effect(&ability, events, changed, removed_entities)?;
            }
            (
                AbilityEffectDefinition::TerrainBeam { .. },
                AbilityTargetPlan::Projectile { path, .. },
            ) => self.resolve_player_terrain_beam_effect(
                &ability,
                path,
                events,
                changed,
                removed_entities,
            )?,
            (
                AbilityEffectDefinition::FetchItem { .. },
                AbilityTargetPlan::FetchItem { target },
            ) => self.resolve_player_fetch_item_effect(&ability, target, events, changed),
            (
                AbilityEffectDefinition::ConsumeTerrain { .. },
                AbilityTargetPlan::ConsumeTerrain {
                    position,
                    source_terrain_id,
                    target_terrain_id,
                },
            ) => self.resolve_player_consume_terrain_effect(
                &ability,
                position,
                source_terrain_id,
                target_terrain_id,
                events,
                changed,
            ),
            (
                AbilityEffectDefinition::CreateAmmunition { .. },
                AbilityTargetPlan::CreateAmmunitionFromTerrain {
                    position,
                    source_terrain_id,
                    target_terrain_id,
                },
            ) => self.resolve_player_create_ammunition_effect(
                &ability,
                None,
                Some((position, source_terrain_id, target_terrain_id)),
                events,
                changed,
            )?,
            (
                AbilityEffectDefinition::CreateAmmunition { .. },
                AbilityTargetPlan::CreateAmmunitionFromItem { item_id },
            ) => self.resolve_player_create_ammunition_effect(
                &ability,
                Some(item_id),
                None,
                events,
                changed,
            )?,
            (
                AbilityEffectDefinition::TransmuteItemToGold { .. },
                AbilityTargetPlan::Item { item_id },
            ) => self.resolve_player_transmute_item_effect(&ability, &item_id, events),
            (
                AbilityEffectDefinition::DrainItemMagic { .. },
                AbilityTargetPlan::Item { item_id },
            ) => self.resolve_player_drain_item_magic_effect(&ability, &item_id, events),
            (AbilityEffectDefinition::ReportMagic, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_report_magic_effect(&ability, events)
            }
            (AbilityEffectDefinition::Concentrate, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_concentrate_effect(&ability, events)
            }
            (AbilityEffectDefinition::MeleeAdjacent, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_melee_adjacent_effect(events, changed, removed_entities)?;
            }
            (
                AbilityEffectDefinition::SniperShot { mode },
                AbilityTargetPlan::SniperShot { target },
            ) => self.resolve_player_projectile(
                target,
                super::player_combat::ProjectileMode::Sniper(mode),
                events,
                changed,
                removed_entities,
            )?,
            (AbilityEffectDefinition::ProbeMonsters, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_probe_monsters_effect(&ability, events, changed);
            }
            (AbilityEffectDefinition::Earthquake { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_earthquake_effect(&ability, events, changed, removed_entities)?;
            }
            (AbilityEffectDefinition::AreaDestruction { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_area_destruction_effect(
                    &ability,
                    events,
                    changed,
                    removed_entities,
                );
            }
            (
                AbilityEffectDefinition::SuppressMonsterReproduction { .. },
                AbilityTargetPlan::SelfTarget,
            ) => self.resolve_player_suppress_reproduction_effect(&ability, events),
            (
                AbilityEffectDefinition::MeleeThenTeleport { .. },
                AbilityTargetPlan::MeleeThenTeleport {
                    target_entity_id,
                    teleport_candidates,
                },
            ) => self.resolve_player_melee_then_teleport_effect(
                &ability,
                &target_entity_id,
                teleport_candidates,
                events,
                changed,
                removed_entities,
            )?,
            (
                AbilityEffectDefinition::DraconianStrike { mode },
                AbilityTargetPlan::MeleeThenTeleport {
                    target_entity_id, ..
                },
            ) => self.resolve_player_draconian_strike_effect(
                mode,
                &target_entity_id,
                events,
                changed,
                removed_entities,
            )?,
            (AbilityEffectDefinition::PolymorphSelf, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_polymorph_self_effect(&ability, events)
            }
            (
                AbilityEffectDefinition::Rodeo,
                AbilityTargetPlan::Rodeo {
                    direction,
                    target_entity_id,
                },
            ) => self.resolve_player_rodeo_effect(direction, &target_entity_id, events, changed),
            (
                AbilityEffectDefinition::PolymorphTarget,
                AbilityTargetPlan::Projectile { path, .. },
            ) => self.resolve_player_polymorph_target_effect(&ability, path, events, changed),
            (AbilityEffectDefinition::SwapPosition, AbilityTargetPlan::Projectile { path, .. }) => {
                self.resolve_player_swap_position_effect(&ability, path, events, changed)
            }
            (AbilityEffectDefinition::Recall { .. }, AbilityTargetPlan::Recall { action }) => {
                self.resolve_player_recall_effect(&ability, action, events)
            }
            (
                AbilityEffectDefinition::TeleportLevel,
                AbilityTargetPlan::TeleportLevel {
                    upward_targets,
                    downward_targets,
                },
            ) => self.resolve_player_level_teleport_effect(
                &ability,
                upward_targets,
                downward_targets,
                events,
                changed,
            )?,
            (
                AbilityEffectDefinition::TeleportAway { .. },
                AbilityTargetPlan::Projectile { path, .. },
            ) => self.resolve_player_teleport_away_effect(&ability, path, events, changed),
            (
                AbilityEffectDefinition::RechargeFromPlayer { .. },
                AbilityTargetPlan::Item { item_id },
            ) => self.resolve_player_recharge_effect(&ability, &item_id, events),
            (AbilityEffectDefinition::Clairvoyance { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_clairvoyance_effect(&ability, events, changed)
            }
            (AbilityEffectDefinition::CallSunlight { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_call_sunlight_effect(&ability, events, changed);
            }
            (AbilityEffectDefinition::NatureWrath, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_nature_wrath_effect(
                    &ability,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (AbilityEffectDefinition::ResistElements { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_resist_elements_effect(&ability, events)
            }
            (AbilityEffectDefinition::Summon { .. }, AbilityTargetPlan::Summon { positions }) => {
                self.resolve_player_summon_effect(&ability, positions, events, changed);
            }
            (
                AbilityEffectDefinition::SummonCategory { .. },
                AbilityTargetPlan::SummonCategory {
                    friendly_candidate_kind_ids,
                    hostile_candidate_kind_ids,
                    positions,
                },
            ) => {
                self.resolve_player_category_summon_effect(
                    &ability,
                    friendly_candidate_kind_ids,
                    hostile_candidate_kind_ids,
                    positions,
                    events,
                    changed,
                );
            }
            (AbilityEffectDefinition::NatureGate { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_nature_gate_effect(&ability, events, changed);
            }
            (AbilityEffectDefinition::DemonSummoning, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_demon_summoning_effect(&ability, events, changed);
            }
            (AbilityEffectDefinition::AngelSummoning, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_angel_summoning_effect(&ability, events, changed);
            }
            (AbilityEffectDefinition::BanishEvil, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_banish_evil_effect(&ability, events, changed);
            }
            (AbilityEffectDefinition::Evocation, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_evocation_effect(&ability, events, changed, removed_entities)?;
            }
            (
                AbilityEffectDefinition::WrathOfGod { .. },
                AbilityTargetPlan::Projectile {
                    path,
                    stop_at_actor,
                },
            ) => self.resolve_player_wrath_of_god_effect(
                &ability,
                path,
                stop_at_actor,
                events,
                changed,
                removed_entities,
            )?,
            (AbilityEffectDefinition::DivineIntervention, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_divine_intervention_effect(
                    &ability,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (AbilityEffectDefinition::Crusade, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_crusade_effect(&ability, events, changed);
            }
            (AbilityEffectDefinition::InsanityCircle { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_insanity_circle_effect(
                    &ability,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (AbilityEffectDefinition::ExplodePets, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_explode_pets_effect(
                    &ability,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                AbilityEffectDefinition::SummonGreaterDemon { .. },
                AbilityTargetPlan::GreaterDemonSacrifice {
                    item_id,
                    candidate_kind_ids,
                    positions,
                },
            ) => self.resolve_player_greater_demon_effect(
                &ability,
                &item_id,
                candidate_kind_ids,
                positions,
                events,
                changed,
            ),
            (AbilityEffectDefinition::Detect { .. }, AbilityTargetPlan::Detect) => {
                self.resolve_player_detection_effect(&ability, events, changed);
            }
            (
                AbilityEffectDefinition::RefuelEquippedLight { .. },
                AbilityTargetPlan::SelfTarget,
            ) => self.resolve_player_refuel_equipped_light_effect(&ability, events),
            (
                AbilityEffectDefinition::TransformTerrain { .. },
                AbilityTargetPlan::TerrainTransform { center, positions },
            ) => {
                self.resolve_terrain_transform_effect(
                    &ability,
                    center,
                    positions,
                    TerrainChangeSource::Magic,
                    events,
                    changed,
                );
            }
            (
                AbilityEffectDefinition::CreateAdjacentTerrain { .. },
                AbilityTargetPlan::AdjacentTerrain { replacements },
            ) => self.resolve_player_adjacent_terrain_creation_effect(
                &ability,
                replacements,
                events,
                changed,
            ),
            (
                AbilityEffectDefinition::ApplyStatus { .. }
                | AbilityEffectDefinition::RemoveStatus { .. },
                target_plan,
            ) => {
                self.resolve_player_actor_status_effect(&ability, target_plan, events, changed);
                self.clamp_player_hp_to_effective_max();
            }
            (AbilityEffectDefinition::Control { .. }, target_plan) => {
                self.resolve_player_control_effect(&ability, target_plan, events, changed);
                self.clamp_player_hp_to_effective_max();
            }
            (AbilityEffectDefinition::Sequence { .. }, target_plan) => {
                self.resolve_player_ordered_sequence_effect(
                    &ability,
                    target_plan,
                    events,
                    changed,
                    removed_entities,
                )?;
                self.clamp_player_hp_to_effective_max();
            }
            (
                AbilityEffectDefinition::Damage { .. },
                AbilityTargetPlan::Projectile { path, .. },
            ) => {
                self.resolve_player_projectile_damage_effect(
                    &ability,
                    path,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                AbilityEffectDefinition::Malediction { .. },
                AbilityTargetPlan::Projectile { path, .. },
            ) => {
                self.resolve_player_malediction_effect(
                    &ability,
                    path,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                AbilityEffectDefinition::DeathRay { .. },
                AbilityTargetPlan::Projectile { path, .. },
            ) => {
                self.resolve_player_death_ray_effect(
                    &ability,
                    path,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                AbilityEffectDefinition::AreaDamage { .. },
                AbilityTargetPlan::Projectile {
                    path,
                    stop_at_actor,
                },
            ) => {
                self.resolve_player_area_damage_effect(
                    &ability,
                    path,
                    stop_at_actor,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (AbilityEffectDefinition::LavaFlow { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_lava_flow_effect(&ability, events, changed, removed_entities)?;
            }
            (AbilityEffectDefinition::DoomHand, AbilityTargetPlan::Projectile { path, .. }) => {
                self.resolve_player_doom_hand_effect(
                    &ability,
                    path,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                AbilityEffectDefinition::Hellfire { .. },
                AbilityTargetPlan::Projectile {
                    path,
                    stop_at_actor,
                },
            ) => self.resolve_player_hellfire_effect(
                &ability,
                path,
                stop_at_actor,
                events,
                changed,
                removed_entities,
            )?,
            (
                AbilityEffectDefinition::BeamDamage { .. },
                AbilityTargetPlan::Projectile { path, .. },
            ) => {
                self.resolve_player_beam_damage_effect(
                    &ability,
                    path,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                AbilityEffectDefinition::LightLine { .. },
                AbilityTargetPlan::Projectile { path, .. },
            ) => {
                self.resolve_player_light_line_effect(
                    &ability,
                    path,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                AbilityEffectDefinition::BoltOrBeamDamage { .. },
                AbilityTargetPlan::BoltOrBeam {
                    path,
                    ball_landing,
                    stop_at_actor,
                },
            ) => {
                self.resolve_player_bolt_or_beam_damage_effect(
                    &ability,
                    path,
                    ball_landing,
                    stop_at_actor,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                AbilityEffectDefinition::Stardust { .. },
                AbilityTargetPlan::Projectile { path, .. },
            ) => {
                self.resolve_player_stardust_effect(
                    &ability,
                    path,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                AbilityEffectDefinition::BoltOrAreaDamage { .. },
                AbilityTargetPlan::Projectile {
                    path,
                    stop_at_actor,
                },
            ) => {
                self.resolve_player_bolt_or_area_damage_effect(
                    &ability,
                    path,
                    stop_at_actor,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                AbilityEffectDefinition::ConeDamage { .. },
                target_plan @ AbilityTargetPlan::Cone { .. },
            ) => {
                self.resolve_player_cone_damage_effect(
                    &ability,
                    target_plan,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (AbilityEffectDefinition::Heal { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_healing_effect(&ability, events);
            }
            (AbilityEffectDefinition::HealDice { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_healing_dice_effect(&ability, events);
            }
            (
                AbilityEffectDefinition::RemoveEquippedCurses { .. },
                AbilityTargetPlan::SelfTarget,
            ) => self.resolve_player_remove_equipped_curses_effect(&ability, events),
            (AbilityEffectDefinition::BeginFasting, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_begin_fasting_effect(&ability, events);
            }
            (AbilityEffectDefinition::TurnUndead { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_turn_undead_effect(&ability, events, changed);
            }
            (AbilityEffectDefinition::SustainAttributes { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_sustain_attributes_effect(&ability, events)
            }
            (AbilityEffectDefinition::CureMutation, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_cure_mutation_effect(&ability, events);
            }
            (
                AbilityEffectDefinition::CreateCurrentTerrain { .. },
                AbilityTargetPlan::SelfTarget,
            ) => self.resolve_player_create_current_terrain_effect(&ability, events, changed),
            (AbilityEffectDefinition::ReduceStatus { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_status_reduction_effect(&ability, events);
            }
            (AbilityEffectDefinition::SatisfyHunger, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_satisfy_hunger_effect(&ability, events);
            }
            (AbilityEffectDefinition::DevourFlesh { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_devour_flesh_effect(&ability, events);
            }
            (AbilityEffectDefinition::Vomit, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_vomit_effect(&ability, events, changed, removed_entities)?;
            }
            (AbilityEffectDefinition::IdentifyItem { .. }, AbilityTargetPlan::Item { item_id }) => {
                self.resolve_player_identify_item_effect(&ability, &item_id, events);
            }
            (
                AbilityEffectDefinition::IdentifyOrMassIdentify { mass: false, .. },
                AbilityTargetPlan::Item { item_id },
            ) => self.resolve_player_identify_item_effect(&ability, &item_id, events),
            (
                AbilityEffectDefinition::IdentifyOrMassIdentify { mass: true, .. },
                AbilityTargetPlan::SelfTarget,
            ) => self.resolve_player_mass_identify_effect(&ability, events),
            (AbilityEffectDefinition::RestoreVitality { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_restore_vitality_effect(&ability, events);
            }
            (
                AbilityEffectDefinition::HealthToMana | AbilityEffectDefinition::ManaToHealth,
                AbilityTargetPlan::SelfTarget,
            ) => {
                self.resolve_player_resource_conversion(&ability, events);
            }
            (AbilityEffectDefinition::ClearMind, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_clear_mind(events);
            }
            (
                AbilityEffectDefinition::Precognition
                | AbilityEffectDefinition::Psychometry
                | AbilityEffectDefinition::MindArmor
                | AbilityEffectDefinition::Adrenaline,
                target,
            ) => {
                self.resolve_player_mindcraft_effect(&ability, target, events, changed);
            }
            (AbilityEffectDefinition::Domination { .. }, target) => {
                self.resolve_player_domination_effect(&ability, target, events, changed)
            }
            (AbilityEffectDefinition::AlterReality, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_alter_reality_effect(&ability, events);
            }
            (AbilityEffectDefinition::VisibleDamage { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_visible_damage_effect(
                    &ability,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (AbilityEffectDefinition::VisibleApplyStatus { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_visible_status_effect(&ability, events, changed);
            }
            (AbilityEffectDefinition::Entangle { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_entangle_effect(&ability, events, changed);
            }
            (AbilityEffectDefinition::MassSleepOrStasis { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_mass_sleep_or_stasis_effect(&ability, events, changed)
            }
            (AbilityEffectDefinition::Sanctuary { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_sanctuary_effect(&ability, events, changed)
            }
            (AbilityEffectDefinition::AggravateMonsters, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_aggravate_monsters_effect(&ability, events, changed);
            }
            (AbilityEffectDefinition::BrandWeapon { .. }, AbilityTargetPlan::Item { item_id }) => {
                self.resolve_player_brand_weapon_effect(&ability, &item_id, events);
            }
            (
                AbilityEffectDefinition::ProtectFromCorrosion,
                AbilityTargetPlan::Item { item_id },
            ) => self.resolve_player_corrosion_protection_effect(&ability, &item_id, events),
            (AbilityEffectDefinition::NoOp { .. }, _) => {
                self.resolve_player_no_op_effect(&ability, events);
            }
            (
                AbilityEffectDefinition::DrainLife { .. },
                AbilityTargetPlan::Projectile { path, .. },
            ) => {
                self.resolve_player_drain_life_effect(
                    &ability,
                    path,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                AbilityEffectDefinition::Genocide { .. },
                AbilityTargetPlan::Projectile { path, .. },
            ) => {
                self.resolve_player_genocide_effect(
                    &ability,
                    Some(path),
                    events,
                    changed,
                    removed_entities,
                );
            }
            (
                AbilityEffectDefinition::Genocide {
                    scope: AbilityGenocideScopeDefinition::Nearby,
                    ..
                },
                AbilityTargetPlan::SelfTarget,
            ) => {
                self.resolve_player_genocide_effect(
                    &ability,
                    None,
                    events,
                    changed,
                    removed_entities,
                );
            }
            (AbilityEffectDefinition::AnimateDead { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_animate_dead_effect(&ability, events, changed)?;
            }
            _ => unreachable!("validated ability target plan must match its effect"),
        }
        Ok(None)
    }
}
