// SPDX-License-Identifier: MPL-2.0

use super::item_use::VisibleBanishmentOutcome;
use super::terrain::TerrainChangeSource;
use super::*;

const DEATH_INVOKE_SPIRITS_ABILITY_ID: &str = "demo.ability.death-invoke-spirits";
const DEATH_POISON_BRANDING_ABILITY_ID: &str = "demo.ability.death-poison-branding";
const DEATH_RAISE_DEAD_ABILITY_ID: &str = "demo.ability.death-raise-dead";
const DEATH_VAMPIRIC_DRAIN_ABILITY_ID: &str = "demo.ability.death-vampiric-drain";
const DEATH_VAMPIRIC_BRANDING_ABILITY_ID: &str = "demo.ability.death-vampiric-branding";
const DEATH_VAMPIRISM_TRUE_ABILITY_ID: &str = "demo.ability.death-vampirism-true";

enum EarthquakeSource {
    Ability(String),
    Monster(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum AbilityTargetPlan {
    SelfTarget,
    Detect,
    TerrainTransform {
        center: Position,
        positions: Vec<Position>,
    },
    Teleport {
        destination: Position,
    },
    RandomTeleport {
        candidates: Vec<Position>,
    },
    DimensionDoor {
        requested: Position,
        destination_valid: bool,
        fallback_candidates: Vec<Position>,
    },
    Town {
        town_id: String,
    },
    FetchItem {
        path: Vec<Position>,
    },
    ConsumeTerrain {
        position: Position,
        source_terrain_id: String,
        target_terrain_id: String,
    },
    CreateAmmunitionFromTerrain {
        position: Position,
        source_terrain_id: String,
        target_terrain_id: String,
    },
    CreateAmmunitionFromItem {
        item_id: String,
    },
    MeleeThenTeleport {
        target_entity_id: String,
        teleport_candidates: Vec<Position>,
    },
    Recall {
        action: RecallUseAction,
    },
    TeleportLevel {
        upward_targets: Vec<FloorTransitionTarget>,
        downward_targets: Vec<FloorTransitionTarget>,
    },
    Projectile {
        path: Vec<Position>,
        stop_at_actor: bool,
    },
    SniperShot {
        target: TargetSelection,
    },
    Cone {
        path: Vec<Position>,
        direction: Direction,
        radius: u8,
    },
    Summon {
        positions: Vec<Position>,
    },
    SummonCategory {
        friendly_candidate_kind_ids: Vec<String>,
        hostile_candidate_kind_ids: Vec<String>,
        positions: Vec<Position>,
    },
    Rodeo {
        direction: Direction,
        target_entity_id: String,
    },
    Item {
        item_id: String,
    },
}

fn attribute_kind_dto(kind: AttributeKind) -> rfb_protocol::AttributeKindDto {
    match kind {
        AttributeKind::Strength => rfb_protocol::AttributeKindDto::Strength,
        AttributeKind::Intelligence => rfb_protocol::AttributeKindDto::Intelligence,
        AttributeKind::Wisdom => rfb_protocol::AttributeKindDto::Wisdom,
        AttributeKind::Dexterity => rfb_protocol::AttributeKindDto::Dexterity,
        AttributeKind::Constitution => rfb_protocol::AttributeKindDto::Constitution,
        AttributeKind::Charisma => rfb_protocol::AttributeKindDto::Charisma,
    }
}

fn set_attribute_value(attributes: &mut AttributeSet, kind: AttributeKind, value: u16) {
    match kind {
        AttributeKind::Strength => attributes.strength = value,
        AttributeKind::Intelligence => attributes.intelligence = value,
        AttributeKind::Wisdom => attributes.wisdom = value,
        AttributeKind::Dexterity => attributes.dexterity = value,
        AttributeKind::Constitution => attributes.constitution = value,
        AttributeKind::Charisma => attributes.charisma = value,
    }
}

impl Game {
    pub(super) fn resolve_player_ability(
        &mut self,
        ability_id: &str,
        target: TargetSelection,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        if self.player_has_status_kind(STATUS_CONFUSION) {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "confused".to_owned(),
            });
            return Ok(());
        }
        let ability = self.content.ability(ability_id).cloned();
        let mutation_activation = self.mutation_activation_for_ability(ability_id).cloned();
        let race_activation = self.race_ability_activation(ability_id).cloned();
        let class_activation = self.class_ability_activation(ability_id).cloned();
        let casting_profile = self.casting_profile().cloned();
        if mutation_activation.is_none()
            && race_activation.is_none()
            && class_activation.is_none()
            && casting_profile.is_none()
        {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "no-casting-profile".to_owned(),
            });
            return Ok(());
        }
        let Some(ability) = ability else {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "unknown-ability".to_owned(),
            });
            return Ok(());
        };
        if ability.tags.iter().any(|tag| tag == "requires-sight")
            && self.player_has_status_kind(STATUS_BLINDNESS)
        {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "blind".to_owned(),
            });
            return Ok(());
        }
        let source = if mutation_activation.is_some() {
            AbilitySourceDto::Mutation
        } else if race_activation.is_some() {
            AbilitySourceDto::Race
        } else if class_activation.is_some() {
            AbilitySourceDto::Class
        } else if casting_profile.is_some() {
            AbilitySourceDto::Learned
        } else {
            unreachable!("at least one validated ability source must be available")
        };
        let innate_power = matches!(source, AbilitySourceDto::Mutation | AbilitySourceDto::Race);
        let innate_activation = match source {
            AbilitySourceDto::Mutation => mutation_activation.as_ref(),
            AbilitySourceDto::Race => race_activation.as_ref(),
            AbilitySourceDto::Class | AbilitySourceDto::Learned => None,
        };
        if source != AbilitySourceDto::Learned && self.player_has_status_kind(STATUS_FEAR) {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "afraid".to_owned(),
            });
            return Ok(());
        }
        if source == AbilitySourceDto::Learned && self.player_has_status_kind(STATUS_ANTI_MAGIC) {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "anti-magic".to_owned(),
            });
            return Ok(());
        }
        if source == AbilitySourceDto::Learned && self.player_has_status_kind(STATUS_BERSERK) {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "berserk".to_owned(),
            });
            return Ok(());
        }
        let mut ability = match source {
            AbilitySourceDto::Learned => self.effective_casting_ability(
                casting_profile
                    .as_ref()
                    .expect("learned ability source requires a casting profile"),
                &ability,
            ),
            AbilitySourceDto::Class | AbilitySourceDto::Mutation | AbilitySourceDto::Race => {
                ability
            }
        };
        Self::apply_player_level_scaling(&mut ability, self.progress.level);
        if source == AbilitySourceDto::Learned {
            Self::apply_casting_profile_effect_scaling(
                casting_profile
                    .as_ref()
                    .expect("learned ability source requires a casting profile"),
                &mut ability,
                self.progress.level,
            );
        }
        if !innate_power && let Some(profile) = casting_profile.as_ref() {
            Self::apply_casting_profile_damage_bonus(profile, &mut ability, self.progress.level);
        }
        Self::apply_player_spell_power(&mut ability, self.effective_player_spell_power_bonus());
        let unavailable_reason = match source {
            AbilitySourceDto::Mutation | AbilitySourceDto::Race => {
                let activation =
                    innate_activation.expect("innate ability source requires an activation");
                (self.progress.level < activation.minimum_level).then_some("level-too-low")
            }
            AbilitySourceDto::Class => {
                let activation = class_activation
                    .as_ref()
                    .expect("class ability source requires an activation");
                if self.progress.level < activation.minimum_level {
                    Some("level-too-low")
                } else if self.sniper_concentration < activation.minimum_concentration {
                    Some("concentration-too-low")
                } else {
                    None
                }
            }
            AbilitySourceDto::Learned => {
                let player = Self::player_ability_parameters(&ability);
                let profile = casting_profile
                    .as_ref()
                    .expect("learned ability source requires a casting profile");
                if !self.learned_abilities.contains(ability_id) {
                    Some("not-learned")
                } else if self.progress.level < player.minimum_level {
                    Some("level-too-low")
                } else if !self.profile_supports_ability(profile, ability_id) {
                    Some("ability-not-supported")
                } else if self.ability_book_item_id(profile, ability_id).is_none() {
                    Some("book-unavailable")
                } else if self.ability_cooldown_remaining(&ability) > 0 {
                    Some("cooldown")
                } else {
                    None
                }
            }
        };
        if let Some(reason) = unavailable_reason {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: reason.to_owned(),
            });
            return Ok(());
        }
        if matches!(ability.effect, AbilityEffectDefinition::Rodeo)
            && self.riding_actor_id.is_some()
        {
            events.push(DomainEvent::RodeoAlreadyRiding);
            return Ok(());
        }

        // Validate the target before charging resources/HP or drawing the
        // failure/damage RNG. The command remains a normal scheduled action,
        // but an impossible target cannot consume resources or proficiency.
        let Some(mut target_plan) = self.ability_target_plan(&ability, &target) else {
            events.push(DomainEvent::AbilityTargetUnavailable {
                ability_id: ability.id,
            });
            return Ok(());
        };

        let mutation_progress = AbilityProgress {
            proficiency: 0,
            proficiency_cap: 0,
            cast_count: 0,
            fail_count: 0,
            cooldown_remaining: 0,
        };
        let progress_before = if source != AbilitySourceDto::Learned {
            mutation_progress
        } else {
            self.ability_progress_value(&ability)
        };
        let cooldown_before = if source != AbilitySourceDto::Learned {
            0
        } else {
            self.ability_cooldown_remaining(&ability)
        };
        let (base_resource_cost, resource_cost, resource_id) = match source {
            AbilitySourceDto::Mutation | AbilitySourceDto::Race => {
                let activation =
                    innate_activation.expect("innate ability source requires an activation");
                let cost = self.innate_power_resource_cost(activation);
                (
                    activation.cost,
                    cost,
                    casting_profile
                        .as_ref()
                        .map(|profile| profile.resource_id.clone()),
                )
            }
            AbilitySourceDto::Class => {
                let activation = class_activation
                    .as_ref()
                    .expect("class ability source requires an activation");
                (
                    activation.resource_cost,
                    activation.resource_cost,
                    activation.resource_id.clone(),
                )
            }
            AbilitySourceDto::Learned => {
                let player = Self::player_ability_parameters(&ability);
                (
                    player.resource_cost,
                    self.ability_effective_resource_cost(&ability, progress_before),
                    Some(player.resource_id.clone()),
                )
            }
        };
        let failure_percent = if self.debug_ability_casts_succeed {
            0
        } else {
            match source {
                AbilitySourceDto::Mutation | AbilitySourceDto::Race => self
                    .innate_power_failure_percent(
                        innate_activation.expect("innate ability source requires an activation"),
                    ),
                AbilitySourceDto::Class => self.class_ability_failure_percent(
                    class_activation
                        .as_ref()
                        .expect("class ability source requires an activation"),
                ),
                AbilitySourceDto::Learned => self.ability_failure_percent(
                    casting_profile
                        .as_ref()
                        .expect("learned ability source requires a casting profile"),
                    &ability,
                ),
            }
        };
        let resource_before = resource_id
            .as_deref()
            .and_then(|id| self.resources.get(id))
            .map_or(0, |pool| pool.current);
        let class_hit_point_cost = if source == AbilitySourceDto::Class {
            class_activation
                .as_ref()
                .map_or(0, |activation| activation.hit_point_cost)
        } else {
            0
        };
        if !innate_power
            && resource_cost > 0
            && resource_id
                .as_deref()
                .is_none_or(|id| !self.resources.contains_key(id))
        {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "resource-unavailable".to_owned(),
            });
            return Ok(());
        }
        let resource_paid = if innate_power {
            resource_before.min(resource_cost)
        } else {
            resource_cost
        };
        let hp_paid = if innate_power {
            resource_cost.saturating_sub(resource_paid)
        } else {
            class_hit_point_cost
        };
        let affordable = if innate_power {
            hp_paid <= u32::try_from(self.player.hp.max(0)).unwrap_or(0)
        } else {
            resource_before >= resource_cost
                && hp_paid <= u32::try_from(self.player.hp.max(0)).unwrap_or(0)
        };
        if !affordable {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "insufficient-resource".to_owned(),
            });
            return Ok(());
        }
        if matches!(
            ability.effect,
            AbilityEffectDefinition::RechargeFromPlayer { .. }
        ) && resource_before <= resource_cost
        {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "insufficient-recharge-resource".to_owned(),
            });
            return Ok(());
        }
        if resource_paid > 0 {
            let id = resource_id
                .as_ref()
                .expect("positive resource payment requires a resource id");
            let pool = self
                .resources
                .get_mut(id)
                .expect("positive resource payment requires an available pool");
            pool.current -= resource_paid;
        }
        if hp_paid > 0 {
            self.player.hp = self.player.hp.saturating_sub(
                i32::try_from(hp_paid).expect("validated innate power cost must fit i32"),
            );
        }
        if !matches!(
            ability.effect,
            AbilityEffectDefinition::Concentrate | AbilityEffectDefinition::SniperShot { .. }
        ) {
            self.sniper_concentration = 0;
        }
        let resource_after = resource_before.saturating_sub(resource_paid);
        let percentile_roll =
            u8::try_from(self.rng.bounded(100)).expect("percentile ability roll must fit u8");
        let succeeded = percentile_roll >= failure_percent;
        let progress_after = if source != AbilitySourceDto::Learned {
            mutation_progress
        } else {
            self.record_ability_cast(&ability, succeeded)
        };
        let resolution = AbilityCastResolutionDto {
            ability_id: ability.id.clone(),
            resource_id,
            base_resource_cost,
            resource_cost,
            resource_before,
            resource_after,
            resource_paid,
            hp_paid,
            failure_percent,
            percentile_roll,
            succeeded,
            proficiency_before: progress_before.proficiency,
            proficiency_after: progress_after.proficiency,
            proficiency_rank: Self::ability_proficiency_rank(progress_after.proficiency),
            cast_count: progress_after.cast_count,
            fail_count: progress_after.fail_count,
            cooldown_before,
            cooldown_after: if source != AbilitySourceDto::Learned {
                0
            } else {
                self.ability_cooldown_remaining(&ability)
            },
        };
        if !succeeded {
            events.push(DomainEvent::AbilityCastFailed { resolution });
            return Ok(());
        }
        events.push(DomainEvent::AbilityCastSucceeded {
            resolution: resolution.clone(),
        });
        let first_success_experience =
            if source == AbilitySourceDto::Learned && progress_before.cast_count == 0 {
                Self::player_ability_parameters(&ability).first_success_experience
            } else {
                0
            };

        let random_branch_index = if matches!(
            &ability.effect,
            AbilityEffectDefinition::RandomChoice { .. }
        ) {
            Some(self.select_player_random_choice_branch(
                &mut ability,
                &target,
                &mut target_plan,
                events,
            ))
        } else {
            None
        };

        let result = self.resolve_player_ability_effect(
            ability,
            target_plan,
            events,
            changed,
            removed_entities,
        );
        if result.is_ok()
            && ability_id == DEATH_INVOKE_SPIRITS_ABILITY_ID
            && random_branch_index == Some(0)
        {
            self.add_virtue(VirtueKindDto::Unlife, 1);
        }
        if result.is_ok() && first_success_experience > 0 {
            self.apply_player_experience(u64::from(first_success_experience), events);
        }
        result
    }

    fn select_player_random_choice_branch(
        &mut self,
        ability: &mut AbilityDefinition,
        target: &TargetSelection,
        target_plan: &mut AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
    ) -> u16 {
        let AbilityEffectDefinition::RandomChoice {
            roll_sides,
            level_bonus_divisor,
            branches,
        } = ability.effect.clone()
        else {
            unreachable!("random choice selector requires a random choice effect");
        };
        let base_roll = u16::try_from(self.rng.bounded(u64::from(roll_sides)) + 1)
            .expect("random ability roll must fit u16");
        let level_bonus = self
            .progress
            .level
            .checked_div(level_bonus_divisor)
            .unwrap_or(0);
        let mut roll = i32::try_from(spell_powered_ability_value(
            ability,
            0,
            AbilitySpellPowerField::RandomChoiceRoll,
            u64::from(base_roll.saturating_add(level_bonus)),
        ))
        .expect("spell-powered random ability roll must fit i32");
        if ability.id == DEATH_INVOKE_SPIRITS_ABILITY_ID {
            roll = self.adjust_roll_by_chance_virtue(roll);
            if roll < 26 {
                self.add_virtue(VirtueKindDto::Chance, 1);
            }
        }
        let (branch_index, branch) = branches
            .iter()
            .enumerate()
            .find(|(_, branch)| roll <= i32::from(branch.maximum_roll))
            .or_else(|| {
                (ability.id == DEATH_INVOKE_SPIRITS_ABILITY_ID)
                    .then(|| branches.iter().enumerate().next_back())
                    .flatten()
            })
            .expect("validated random ability branches must cover every roll");
        let branch_index =
            u16::try_from(branch_index).expect("validated random branch index must fit u16");
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::RandomChoice {
                    effect_index: 0,
                    roll,
                    branch_index,
                    maximum_roll: branch.maximum_roll,
                }],
            },
            trace: None,
        });
        ability.effect = (*branch.effect).clone();
        match branch.target {
            AbilityRandomTargetDefinition::CastTarget => {
                if !matches!(ability.effect, AbilityEffectDefinition::NoOp { .. }) {
                    *target_plan = self
                        .ability_target_plan(ability, target)
                        .expect("validated random branch must accept the cast target");
                }
            }
            AbilityRandomTargetDefinition::SelfTarget => {
                ability.target.modes = vec![AbilityTargetModeDefinition::SelfTarget];
                ability.target.range = 0;
                ability.target.requires_line_of_effect = false;
                *target_plan = self
                    .ability_target_plan(ability, &TargetSelection::SelfTarget)
                    .expect("validated random branch must accept a self target");
            }
        }
        branch_index
    }
}

impl Game {
    fn resolve_player_ability_effect(
        &mut self,
        ability: AbilityDefinition,
        target_plan: AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        match (ability.effect.clone(), target_plan) {
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
            (AbilityEffectDefinition::FetchItem { .. }, AbilityTargetPlan::FetchItem { path }) => {
                self.resolve_player_fetch_item_effect(&ability, path, events, changed)
            }
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
            (AbilityEffectDefinition::CreateItem { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_create_item_effect(&ability, events, changed)?;
            }
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
                AbilityTargetPlan::Projectile { path, .. },
            ) => {
                self.resolve_player_bolt_or_beam_damage_effect(
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
            (AbilityEffectDefinition::ReduceStatus { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_status_reduction_effect(&ability, events);
            }
            (AbilityEffectDefinition::SatisfyHunger, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_satisfy_hunger_effect(&ability, events);
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
            (AbilityEffectDefinition::MassSleepOrStasis { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_mass_sleep_or_stasis_effect(&ability, events, changed)
            }
            (AbilityEffectDefinition::AggravateMonsters, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_aggravate_monsters_effect(&ability, events, changed);
            }
            (AbilityEffectDefinition::BrandWeapon { .. }, AbilityTargetPlan::Item { item_id }) => {
                self.resolve_player_brand_weapon_effect(&ability, &item_id, events);
            }
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
        Ok(())
    }

    pub(super) fn resolve_player_projectile_damage_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::Damage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
        } = &ability.effect
        else {
            unreachable!("player projectile damage executor requires a damage effect");
        };
        let (trace, target_index) = self.trace_projectile_path(path);
        self.resolve_projectile_terrain_effects(
            &[trace.impact],
            DamageType::from(*damage_type),
            changed,
        );
        if ability.affects_ground_items {
            self.resolve_ground_item_projectile_effects(
                &ability.id,
                &[trace.landing],
                DamageType::from(*damage_type),
                true,
                events,
                changed,
                removed_entities,
            );
        }
        let Some(index) = target_index else {
            events.push(DomainEvent::AbilityLanded {
                ability_id: ability.id.clone(),
                trace,
            });
            return Ok(());
        };
        let raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        let raw_damage = i32::try_from(spell_powered_ability_value(
            ability,
            0,
            AbilitySpellPowerField::FinalDamage,
            u64::try_from(raw_damage).expect("ability damage must be non-negative"),
        ))
        .expect("spell-powered ability damage must fit i32");
        if self.try_reflect_player_bolt(
            index,
            &ability.id,
            raw_damage,
            DamageType::from(*damage_type),
            ability.affects_ground_items,
            events,
            changed,
            removed_entities,
        )? {
            return Ok(());
        }
        self.resolve_ability_damage_to_entity(
            index,
            &ability.id,
            DamageType::from(*damage_type),
            raw_damage,
            trace,
            events,
            changed,
            removed_entities,
        )?;
        Ok(())
    }

    fn resolve_player_malediction_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::Malediction {
            damage_dice,
            damage_sides,
            damage_bonus,
        } = ability.effect
        else {
            unreachable!("Malediction executor requires a Malediction effect");
        };
        let raw_damage = self
            .roll_damage(damage_dice, damage_sides)
            .saturating_add(i32::from(damage_bonus))
            .max(0);
        let raw_damage = i32::try_from(spell_powered_ability_value(
            ability,
            0,
            AbilitySpellPowerField::FinalDamage,
            u64::try_from(raw_damage).expect("Malediction damage must be non-negative"),
        ))
        .expect("spell-powered Malediction damage must fit i32");
        let (trace, target_index) = self.trace_projectile_path(path.clone());
        self.resolve_projectile_terrain_effects(&[trace.impact], DamageType::HellFire, changed);
        if ability.affects_ground_items {
            self.resolve_ground_item_projectile_effects(
                &ability.id,
                &[trace.landing],
                DamageType::HellFire,
                true,
                events,
                changed,
                removed_entities,
            );
        }
        if let Some(target_index) = target_index {
            self.resolve_ability_damage_to_entity(
                target_index,
                &ability.id,
                DamageType::HellFire,
                raw_damage,
                trace.clone(),
                events,
                changed,
                removed_entities,
            )?;
        } else {
            events.push(DomainEvent::AbilityLanded {
                ability_id: ability.id.clone(),
                trace: trace.clone(),
            });
        }

        let trigger_roll =
            u16::try_from(self.rng.bounded(5) + 1).expect("Malediction trigger roll must fit u16");
        let mut choices = vec![AbilityEffectResolutionDto::RandomChoice {
            effect_index: 0,
            roll: i32::from(trigger_roll),
            branch_index: u16::from(trigger_roll == 1),
            maximum_roll: 5,
        }];
        if trigger_roll != 1 {
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability.id.clone(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: None,
                    target_kind_id: None,
                    effects: choices,
                },
                trace: Some(trace),
            });
            return Ok(());
        }

        let rider_roll = u16::try_from(self.rng.bounded(1_000) + 1)
            .expect("Malediction rider roll must fit u16");
        let branch_index = if rider_roll == 666 {
            0
        } else if rider_roll < 500 {
            1
        } else if rider_roll < 800 {
            2
        } else {
            3
        };
        choices.push(AbilityEffectResolutionDto::RandomChoice {
            effect_index: 0,
            roll: i32::from(rider_roll),
            branch_index,
            maximum_roll: 1_000,
        });
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: choices,
            },
            trace: Some(trace),
        });

        let level = self.progress.level;
        if rider_roll == 666 {
            let mut rider = ability.clone();
            rider.effect = AbilityEffectDefinition::DeathRay {
                power: u32::try_from(spell_powered_ability_value(
                    ability,
                    0,
                    AbilitySpellPowerField::MaledictionDeathRayPower,
                    u64::from(level) * 200,
                ))
                .expect("spell-powered Malediction death ray must fit u32"),
            };
            return self.resolve_player_death_ray_effect(
                &rider,
                path,
                events,
                changed,
                removed_entities,
            );
        }

        let (status_kind_id, duration_ticks, power) = if rider_roll < 500 {
            let duration_sides = level / 2;
            let duration_ticks = if duration_sides == 0 {
                1
            } else {
                u32::try_from(self.rng.bounded(u64::from(duration_sides)) + 1)
                    .expect("Malediction fear duration roll must fit u32")
                    .saturating_mul(3)
                    .saturating_add(1)
            };
            let power = u16::try_from(spell_powered_ability_value(
                ability,
                0,
                AbilitySpellPowerField::MaledictionFearPower,
                u64::from(level),
            ))
            .expect("spell-powered Malediction fear power must fit u16");
            (STATUS_FEAR, duration_ticks, Some(power))
        } else if rider_roll < 800 {
            let power = (level / 2)
                .max(u16::try_from(raw_damage.min(100)).expect("Malediction damage must fit u16"));
            let duration_sides = power / 2;
            let duration_ticks = u32::try_from(self.rng.bounded(u64::from(duration_sides)) + 1)
                .expect("Malediction confusion duration roll must fit u32")
                .saturating_mul(3)
                .saturating_add(1);
            (STATUS_CONFUSION, duration_ticks, Some(power))
        } else {
            (
                STATUS_STUN,
                u32::try_from(raw_damage).expect("Malediction damage must be non-negative"),
                None,
            )
        };
        self.resolve_player_malediction_status_rider(
            &ability.id,
            path,
            status_kind_id,
            duration_ticks,
            power,
            events,
            changed,
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_player_malediction_status_rider(
        &mut self,
        ability_id: &str,
        path: Vec<Position>,
        status_kind_id: &str,
        duration_ticks: u32,
        power: Option<u16>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let (trace, target_index) = self.trace_projectile_path(path);
        let Some(target_index) = target_index else {
            events.push(DomainEvent::AbilityLanded {
                ability_id: ability_id.to_owned(),
                trace: trace.clone(),
            });
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability_id.to_owned(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: None,
                    target_kind_id: None,
                    effects: vec![AbilityEffectResolutionDto::Skipped {
                        effect_index: 0,
                        reason: AbilityEffectSkipReasonDto::NoTarget,
                    }],
                },
                trace: Some(trace),
            });
            return;
        };

        let target_entity_id = self.entities[target_index].id.clone();
        let target_kind_id = self.entities[target_index].kind_id.clone();
        let definition = self
            .actor_runtime_definition(&self.entities[target_index])
            .expect("Malediction target definition must remain available");
        let target_level =
            definition
                .level
                .saturating_add(if definition.tags.iter().any(|tag| tag == "unique") {
                    3
                } else {
                    0
                });
        let immunities = if self.actor_has_status_immunity(target_index, status_kind_id) {
            BTreeSet::from([status_kind_id.to_owned()])
        } else {
            BTreeSet::new()
        };
        let resistances = ResistanceProfile::default();
        self.entities[target_index].alerted = true;
        changed.insert(self.entities[target_index].position);
        let resolution = apply_ability_status_effect(
            &mut self.entities[target_index],
            ability_id,
            0,
            status_kind_id,
            1,
            duration_ticks,
            0,
            0,
            AbilityStatusStackingDefinition::Extend,
            None,
            power,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers::default(),
            &EquipmentBonuses::default(),
            &BTreeSet::new(),
            None,
            false,
            100,
            Some(target_level),
            Some((&resistances, &immunities)),
            &mut self.rng,
        );
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability_id.to_owned(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(target_entity_id),
                target_kind_id: Some(target_kind_id),
                effects: vec![resolution],
            },
            trace: Some(trace),
        });
    }

    pub(super) fn resolve_player_area_damage_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        stop_at_actor: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::AreaDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            radius,
            target_category,
        } = &ability.effect
        else {
            unreachable!("player area damage executor requires an area damage effect");
        };
        let base_raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        let base_raw_damage = i32::try_from(spell_powered_ability_value(
            ability,
            0,
            AbilitySpellPowerField::FinalDamage,
            u64::try_from(base_raw_damage).expect("area damage must be non-negative"),
        ))
        .expect("spell-powered area damage must fit i32");
        self.resolve_player_area_damage_with_base(
            &ability.id,
            path,
            stop_at_actor,
            DamageType::from(*damage_type),
            *radius,
            target_category.as_deref(),
            base_raw_damage,
            ability.affects_ground_items,
            events,
            changed,
            removed_entities,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_player_area_damage_with_base(
        &mut self,
        source_id: &str,
        path: Vec<Position>,
        stop_at_actor: bool,
        damage_type: DamageType,
        radius: u8,
        target_category: Option<&str>,
        base_raw_damage: i32,
        affects_ground_items: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let (trace, _) = self.trace_projectile_path_with_actor_policy(path, stop_at_actor);
        let center = trace.landing;
        let (affected_positions, targets) =
            self.area_damage_targets(center, radius, target_category);
        self.resolve_projectile_terrain_effects(&affected_positions, damage_type, changed);
        if affects_ground_items {
            self.resolve_ground_item_projectile_effects(
                source_id,
                &affected_positions,
                damage_type,
                true,
                events,
                changed,
                removed_entities,
            );
        }
        changed.extend(affected_positions.iter().copied());
        events.push(DomainEvent::AbilityAreaDamage {
            ability_id: source_id.to_owned(),
            resolution: AbilityAreaDamageResolutionDto {
                center,
                radius,
                base_raw_damage,
                damage_type: damage_type.into(),
                affected_positions,
                target_count: u16::try_from(targets.len()).unwrap_or(u16::MAX),
            },
            trace: trace.clone(),
        });
        for (entity_id, distance) in targets {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            let falloff_damage = rfb_area_damage(base_raw_damage, distance);
            self.resolve_ability_damage_to_entity(
                index,
                source_id,
                damage_type,
                falloff_damage,
                trace.clone(),
                events,
                changed,
                removed_entities,
            )?;
        }
        Ok(())
    }

    pub(super) fn resolve_player_beam_damage_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::BeamDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
        } = &ability.effect
        else {
            unreachable!("player beam damage executor requires a beam damage effect");
        };
        let base_raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        let base_raw_damage = i32::try_from(spell_powered_ability_value(
            ability,
            0,
            AbilitySpellPowerField::FinalDamage,
            u64::try_from(base_raw_damage).expect("beam damage must be non-negative"),
        ))
        .expect("spell-powered beam damage must fit i32");
        self.resolve_player_beam_damage_with_base(
            &ability.id,
            path,
            DamageType::from(*damage_type),
            base_raw_damage,
            ability.affects_ground_items,
            events,
            changed,
            removed_entities,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_player_beam_damage_with_base(
        &mut self,
        source_id: &str,
        path: Vec<Position>,
        damage_type: DamageType,
        base_raw_damage: i32,
        affects_ground_items: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let (trace, _) = self.trace_projectile_path_with_actor_policy(path, false);
        let affected_positions = trace.traversed.clone();
        self.resolve_projectile_terrain_effects(&affected_positions, damage_type, changed);
        self.resolve_projectile_terrain_effects(&[trace.impact], damage_type, changed);
        if affects_ground_items {
            self.resolve_ground_item_projectile_effects(
                source_id,
                &affected_positions,
                damage_type,
                true,
                events,
                changed,
                removed_entities,
            );
        }
        let targets = self.beam_damage_targets(&affected_positions);
        changed.extend(affected_positions.iter().copied());
        events.push(DomainEvent::AbilityBeamDamage {
            ability_id: source_id.to_owned(),
            resolution: AbilityBeamDamageResolutionDto {
                base_raw_damage,
                damage_type: damage_type.into(),
                affected_positions,
                target_count: u16::try_from(targets.len()).unwrap_or(u16::MAX),
            },
            trace: trace.clone(),
        });
        for entity_id in targets {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            self.resolve_ability_damage_to_entity(
                index,
                source_id,
                damage_type,
                base_raw_damage,
                trace.clone(),
                events,
                changed,
                removed_entities,
            )?;
        }
        Ok(())
    }

    fn resolve_player_light_line_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::LightLine {
            damage_dice,
            damage_sides,
        } = ability.effect
        else {
            unreachable!("light-line executor requires a light-line effect");
        };
        let (trace, _) = self.trace_projectile_path_with_actor_policy(path.clone(), false);
        let affected_positions = trace.traversed.clone();
        for position in &affected_positions {
            if let Some(index) = self.index(*position) {
                self.glow[index] = true;
                changed.insert(*position);
            }
        }
        let base_raw_damage = self.roll_damage(damage_dice, damage_sides).max(0);
        let targets = self
            .beam_damage_targets(&affected_positions)
            .into_iter()
            .filter(|entity_id| {
                self.entities.iter().any(|entity| {
                    entity.id == *entity_id
                        && entity.resistances.level(DamageType::Light)
                            == ResistanceLevel::Vulnerable
                })
            })
            .collect::<Vec<_>>();
        events.push(DomainEvent::AbilityBeamDamage {
            ability_id: ability.id.clone(),
            resolution: AbilityBeamDamageResolutionDto {
                base_raw_damage,
                damage_type: DamageType::Light.into(),
                affected_positions,
                target_count: u16::try_from(targets.len()).unwrap_or(u16::MAX),
            },
            trace: trace.clone(),
        });
        for entity_id in targets {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            self.resolve_weak_light_damage_to_entity(
                index,
                &ability.id,
                base_raw_damage,
                trace.clone(),
                events,
                changed,
                removed_entities,
            )?;
        }
        Ok(())
    }

    fn resolve_player_light_area_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::LightArea {
            damage_dice,
            damage_sides,
            radius,
        } = ability.effect
        else {
            unreachable!("light-area executor requires a light-area effect");
        };
        let (trace, _) = self.trace_projectile_path_with_actor_policy(Vec::new(), false);
        let center = self.player.position;
        let (affected_positions, targets) = self.area_damage_targets(center, radius, None);
        let targets = targets
            .into_iter()
            .filter(|(entity_id, _)| {
                self.entities.iter().any(|entity| {
                    entity.id == *entity_id
                        && entity.resistances.level(DamageType::Light)
                            == ResistanceLevel::Vulnerable
                })
            })
            .collect::<Vec<_>>();
        let base_raw_damage = self.roll_damage(damage_dice, damage_sides).max(0);

        let mut glow_positions = affected_positions.iter().copied().collect::<BTreeSet<_>>();
        glow_positions.extend(self.connected_glow_positions(center));
        for position in glow_positions {
            let Some(index) = self.index(position) else {
                continue;
            };
            if !self.glow[index] {
                self.glow[index] = true;
                changed.insert(position);
            }
        }

        events.push(DomainEvent::AbilityAreaDamage {
            ability_id: ability.id.clone(),
            resolution: AbilityAreaDamageResolutionDto {
                center,
                radius,
                base_raw_damage,
                damage_type: DamageType::Light.into(),
                affected_positions,
                target_count: u16::try_from(targets.len()).unwrap_or(u16::MAX),
            },
            trace: trace.clone(),
        });
        for (entity_id, distance) in targets {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            self.resolve_weak_light_damage_to_entity(
                index,
                &ability.id,
                rfb_area_damage(base_raw_damage, distance),
                trace.clone(),
                events,
                changed,
                removed_entities,
            )?;
        }
        Ok(())
    }

    fn resolve_player_terrain_beam_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::TerrainBeam { operation } = ability.effect else {
            unreachable!("terrain-beam executor requires a terrain-beam effect");
        };
        let stone_to_mud_power = (operation == AbilityTerrainBeamOperationDefinition::StoneToMud)
            .then(|| {
                u16::try_from(21 + self.rng.bounded(30)).expect("stone-to-mud power must fit u16")
            });
        if operation == AbilityTerrainBeamOperationDefinition::JamDoors {
            let _ = self.rng.bounded(30);
        }
        let (trace, _) = self.trace_projectile_path_with_actor_policy(path, false);
        let mut affected_positions = trace.traversed.clone();
        if trace.impact != trace.landing && self.index(trace.impact).is_some() {
            affected_positions.push(trace.impact);
        }
        let mut replacements = Vec::new();
        for position in affected_positions {
            let Some(index) = self.index(position) else {
                continue;
            };
            let Some(terrain) = self.content.terrain(&self.terrain[index]) else {
                continue;
            };
            let target_id = match operation {
                AbilityTerrainBeamOperationDefinition::JamDoors => {
                    terrain.jam_to_terrain_id.as_ref()
                }
                AbilityTerrainBeamOperationDefinition::DestroyTrapsAndDoors => terrain
                    .trap
                    .as_ref()
                    .map(|trap| &trap.disarm_to_terrain_id)
                    .or_else(|| {
                        terrain
                            .tags
                            .iter()
                            .any(|tag| tag == "door")
                            .then_some(terrain.bash_to_terrain_id.as_ref())
                            .flatten()
                    }),
                AbilityTerrainBeamOperationDefinition::StoneToMud => terrain
                    .digging
                    .as_ref()
                    .filter(|digging| digging.resolution != TerrainDiggingResolution::Permanent)
                    .and_then(|digging| digging.result_terrain_id.as_ref()),
            };
            if let Some(target_id) = target_id
                && target_id != &terrain.id
            {
                replacements.push((position, terrain.id.clone(), target_id.clone()));
            }
        }

        let mut groups = BTreeMap::<(String, String), Vec<Position>>::new();
        for (position, source_id, target_id) in replacements {
            self.replace_terrain_from_source(
                position,
                &target_id,
                TerrainChangeSource::Magic,
                events,
                changed,
            );
            groups
                .entry((source_id, target_id))
                .or_default()
                .push(position);
        }
        for ((source_id, target_id), positions) in groups {
            events.push(DomainEvent::AbilityTerrainTransformed {
                ability_id: ability.id.clone(),
                resolution: AbilityTerrainTransformResolutionDto {
                    center: self.player.position,
                    radius: 0,
                    source_terrain_ids: vec![source_id],
                    target_terrain_id: target_id,
                    transformed_positions: positions,
                },
            });
        }
        if let Some(power) = stone_to_mud_power {
            let targets = self
                .beam_damage_targets(&trace.traversed)
                .into_iter()
                .filter(|entity_id| {
                    self.entities.iter().any(|entity| {
                        entity.id == *entity_id
                            && entity.resistances.level(DamageType::Disintegrate)
                                == ResistanceLevel::Vulnerable
                    })
                })
                .collect::<Vec<_>>();
            events.push(DomainEvent::AbilityBeamDamage {
                ability_id: ability.id.clone(),
                resolution: AbilityBeamDamageResolutionDto {
                    base_raw_damage: i32::from(power),
                    damage_type: DamageType::Disintegrate.into(),
                    affected_positions: trace.traversed.clone(),
                    target_count: u16::try_from(targets.len()).unwrap_or(u16::MAX),
                },
                trace: trace.clone(),
            });
            for entity_id in targets {
                let Some(index) = self
                    .entities
                    .iter()
                    .position(|entity| entity.id == entity_id && entity.hp > 0)
                else {
                    continue;
                };
                self.resolve_stone_to_mud_damage_to_entity(
                    index,
                    &ability.id,
                    i32::from(power),
                    trace.clone(),
                    events,
                    changed,
                    removed_entities,
                )?;
            }
        }
        Ok(())
    }

    pub(super) fn resolve_player_bolt_or_beam_damage_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::BoltOrBeamDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            beam_chance_percent,
            ..
        } = &ability.effect
        else {
            unreachable!("bolt-or-beam executor requires a bolt-or-beam damage effect");
        };
        let damage_type = DamageType::from(*damage_type);
        let beam = self.rng.bounded(100) < u64::from(*beam_chance_percent);
        let base_raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        let base_raw_damage = i32::try_from(spell_powered_ability_value(
            ability,
            0,
            AbilitySpellPowerField::FinalDamage,
            u64::try_from(base_raw_damage).expect("bolt or beam damage must be non-negative"),
        ))
        .expect("spell-powered bolt or beam damage must fit i32");
        if beam {
            let (trace, _) = self.trace_projectile_path_with_actor_policy(path, false);
            let affected_positions = trace.traversed.clone();
            self.resolve_projectile_terrain_effects(&affected_positions, damage_type, changed);
            self.resolve_projectile_terrain_effects(&[trace.impact], damage_type, changed);
            if ability.affects_ground_items {
                self.resolve_ground_item_projectile_effects(
                    &ability.id,
                    &affected_positions,
                    damage_type,
                    true,
                    events,
                    changed,
                    removed_entities,
                );
            }
            let targets = self.beam_damage_targets(&affected_positions);
            changed.extend(affected_positions.iter().copied());
            events.push(DomainEvent::AbilityBeamDamage {
                ability_id: ability.id.clone(),
                resolution: AbilityBeamDamageResolutionDto {
                    base_raw_damage,
                    damage_type: damage_type.into(),
                    affected_positions,
                    target_count: u16::try_from(targets.len()).unwrap_or(u16::MAX),
                },
                trace: trace.clone(),
            });
            for entity_id in targets {
                let Some(index) = self
                    .entities
                    .iter()
                    .position(|entity| entity.id == entity_id && entity.hp > 0)
                else {
                    continue;
                };
                self.resolve_ability_damage_to_entity(
                    index,
                    &ability.id,
                    damage_type,
                    base_raw_damage,
                    trace.clone(),
                    events,
                    changed,
                    removed_entities,
                )?;
            }
        } else {
            let (trace, target_index) = self.trace_projectile_path_with_actor_policy(path, true);
            self.resolve_projectile_terrain_effects(&[trace.impact], damage_type, changed);
            if ability.affects_ground_items {
                self.resolve_ground_item_projectile_effects(
                    &ability.id,
                    &[trace.landing],
                    damage_type,
                    true,
                    events,
                    changed,
                    removed_entities,
                );
            }
            let Some(index) = target_index else {
                events.push(DomainEvent::AbilityLanded {
                    ability_id: ability.id.clone(),
                    trace,
                });
                return Ok(());
            };
            if self.try_reflect_player_bolt(
                index,
                &ability.id,
                base_raw_damage,
                damage_type,
                ability.affects_ground_items,
                events,
                changed,
                removed_entities,
            )? {
                return Ok(());
            }
            self.resolve_ability_damage_to_entity(
                index,
                &ability.id,
                damage_type,
                base_raw_damage,
                trace,
                events,
                changed,
                removed_entities,
            )?;
        }
        Ok(())
    }

    fn resolve_player_bolt_or_area_damage_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        stop_at_actor: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::BoltOrAreaDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            area_from_level,
            radius,
        } = ability.effect
        else {
            unreachable!("bolt-or-area executor requires a matching effect");
        };
        let mut resolved = ability.clone();
        if self.progress.level < area_from_level {
            resolved.effect = AbilityEffectDefinition::Damage {
                damage_dice,
                damage_sides,
                damage_bonus,
                damage_type,
            };
            self.resolve_player_projectile_damage_effect(
                &resolved,
                path,
                events,
                changed,
                removed_entities,
            )
        } else {
            resolved.effect = AbilityEffectDefinition::AreaDamage {
                damage_dice,
                damage_sides,
                damage_bonus,
                damage_type,
                radius,
                target_category: None,
            };
            self.resolve_player_area_damage_effect(
                &resolved,
                path,
                stop_at_actor,
                events,
                changed,
                removed_entities,
            )
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn try_reflect_player_bolt(
        &mut self,
        reflector_index: usize,
        source_kind_id: &str,
        raw_damage: i32,
        damage_type: DamageType,
        affects_ground_items: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<bool, CoreError> {
        let reflector_kind_id = self.entities[reflector_index].kind_id.clone();
        if !self
            .actor_runtime_definition(&self.entities[reflector_index])
            .is_some_and(|definition| definition.reflects_bolts)
            || self.rng.bounded(4) == 0
        {
            return Ok(false);
        }

        let origin = self.entities[reflector_index].position;
        let range = self.width.max(self.height);
        let mut reflected_path = None;
        for _ in 0..10 {
            let y = self.player.position.y
                + i32::try_from(self.rng.bounded(5)).expect("bounded draw fits i32")
                - 2;
            let x = self.player.position.x
                + i32::try_from(self.rng.bounded(5)).expect("bounded draw fits i32")
                - 2;
            let destination = Position { x, y };
            let Some(path) = projectile_path_between(origin, destination, range) else {
                continue;
            };
            if path
                .iter()
                .all(|position| self.index(*position).is_some() && self.is_walkable(*position))
            {
                reflected_path = Some(path);
                break;
            }
        }
        let path = reflected_path
            .or_else(|| projectile_path_between(origin, self.player.position, range))
            .expect("an incoming bolt must retain a reverse reflection path");
        let can_hit_player = self.rng.bounded(2) != 0;
        let mut impact = origin;
        let mut landing = origin;
        let mut traversed = Vec::new();
        let mut hit_player = false;
        let mut hit_actor_index = None;
        for position in path {
            impact = position;
            if self.index(position).is_none() || !self.is_walkable(position) {
                break;
            }
            landing = position;
            traversed.push(position);
            if can_hit_player && position == self.player.position {
                hit_player = true;
                break;
            }
            if let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.hp > 0 && entity.position == position)
            {
                hit_actor_index = Some(index);
                break;
            }
        }
        let trace = ProjectileTrace {
            origin,
            impact,
            landing,
            traversed,
        };
        self.resolve_projectile_terrain_effects(&[trace.impact], damage_type, changed);
        if affects_ground_items {
            self.resolve_ground_item_projectile_effects(
                source_kind_id,
                &[trace.landing],
                damage_type,
                true,
                events,
                changed,
                removed_entities,
            );
        }

        if hit_player {
            let target = self.player_derived_stats();
            let resistance = self.effective_player_resistances().level(damage_type);
            let damage = self.reduce_player_damage(resolve_armored_damage(
                raw_damage,
                damage_type,
                target.armor_class.value,
                resistance,
            ));
            let application =
                plan_damage_application(&self.player, damage, FatalityPolicy::BelowZero);
            commit_damage_application(&mut self.player, &application);
            events.push(DomainEvent::BoltReflected {
                reflector_kind_id: reflector_kind_id.clone(),
                source_kind_id: source_kind_id.to_owned(),
                outcome: BoltReflectionOutcome::Hit {
                    target_kind_id: self.player.kind_id.clone(),
                    damage,
                    fatal: application.fatal,
                },
                trace,
            });
            if application.fatal {
                events.push(DomainEvent::PlayerDied {
                    source_kind_id: reflector_kind_id,
                    method_id: Some(source_kind_id.to_owned()),
                    damage,
                });
            }
            return Ok(true);
        }

        if let Some(index) = hit_actor_index {
            let definition = self
                .content
                .actor(&self.entities[index].kind_id)
                .expect("reflected bolt target definition must remain available")
                .clone();
            let target_kind_id = definition.id.clone();
            let target = self.actor_derived_stats(&self.entities[index], &definition, false);
            let resistance = self.entities[index].resistances.level(damage_type);
            let damage = resolve_armored_damage(
                raw_damage,
                damage_type,
                target.armor_class.value,
                resistance,
            );
            let application = plan_damage_application(
                &self.entities[index],
                damage,
                FatalityPolicy::AtOrBelowZero,
            );
            commit_damage_application(&mut self.entities[index], &application);
            self.entities[index].alerted = true;
            changed.insert(self.entities[index].position);
            self.wake_entity_after_damage(index, damage.applied, events);
            events.push(DomainEvent::BoltReflected {
                reflector_kind_id,
                source_kind_id: source_kind_id.to_owned(),
                outcome: BoltReflectionOutcome::Hit {
                    target_kind_id,
                    damage,
                    fatal: application.fatal,
                },
                trace,
            });
            if application.fatal {
                self.resolve_actor_death_without_rewards(
                    index,
                    None,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            return Ok(true);
        }

        events.push(DomainEvent::BoltReflected {
            reflector_kind_id,
            source_kind_id: source_kind_id.to_owned(),
            outcome: BoltReflectionOutcome::Landed,
            trace,
        });
        Ok(true)
    }

    pub(super) fn resolve_player_cone_damage_effect(
        &mut self,
        ability: &AbilityDefinition,
        target_plan: AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::ConeDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            radius,
        } = &ability.effect
        else {
            unreachable!("player cone damage executor requires a cone damage effect");
        };
        let AbilityTargetPlan::Cone {
            path,
            direction,
            radius: planned_radius,
        } = target_plan
        else {
            unreachable!("player cone damage executor requires a cone target plan");
        };
        debug_assert_eq!(*radius, planned_radius);
        let damage_type = DamageType::from(*damage_type);
        let (trace, _) =
            self.trace_projectile_path_with_damage_policy(path, false, Some(damage_type));
        let (affected_positions, targets) =
            self.cone_damage_targets(&trace.traversed, direction, *radius, damage_type);
        self.resolve_projectile_terrain_effects(&affected_positions, damage_type, changed);
        if ability.affects_ground_items {
            self.resolve_ground_item_projectile_effects(
                &ability.id,
                &affected_positions,
                damage_type,
                true,
                events,
                changed,
                removed_entities,
            );
        }
        changed.extend(affected_positions.iter().copied());
        let base_raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        let base_raw_damage = i32::try_from(spell_powered_ability_value(
            ability,
            0,
            AbilitySpellPowerField::FinalDamage,
            u64::try_from(base_raw_damage).expect("cone damage must be non-negative"),
        ))
        .expect("spell-powered cone damage must fit i32");
        events.push(DomainEvent::AbilityConeDamage {
            ability_id: ability.id.clone(),
            resolution: AbilityConeDamageResolutionDto {
                radius: *radius,
                base_raw_damage,
                damage_type: damage_type.into(),
                affected_positions,
                target_count: u16::try_from(targets.len()).unwrap_or(u16::MAX),
            },
            trace: trace.clone(),
        });
        for (entity_id, lateral_distance) in targets {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            let falloff_damage = rfb_area_damage(base_raw_damage, lateral_distance);
            self.resolve_ability_damage_to_entity(
                index,
                &ability.id,
                damage_type,
                falloff_damage,
                trace.clone(),
                events,
                changed,
                removed_entities,
            )?;
        }
        Ok(())
    }

    pub(super) fn resolve_player_visible_damage_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::VisibleDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            target_category,
        } = &ability.effect
        else {
            unreachable!("visible damage executor requires a visible damage effect");
        };
        let target_ids = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && self.entity_is_visible_to_player(entity)
                    && target_category.as_ref().is_none_or(|category| {
                        self.content
                            .actor(&entity.kind_id)
                            .is_some_and(|definition| actor_matches_category(definition, category))
                    })
            })
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        let affected_positions = target_ids
            .iter()
            .filter_map(|id| self.entities.iter().find(|entity| &entity.id == id))
            .map(|entity| entity.position)
            .collect::<Vec<_>>();
        let base_raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        let base_raw_damage = i32::try_from(spell_powered_ability_value(
            ability,
            0,
            AbilitySpellPowerField::FinalDamage,
            u64::try_from(base_raw_damage).expect("visible damage must be non-negative"),
        ))
        .expect("spell-powered visible damage must fit i32");
        events.push(DomainEvent::AbilityVisibleDamage {
            ability_id: ability.id.clone(),
            resolution: AbilityVisibleDamageResolutionDto {
                base_raw_damage,
                damage_type: DamageType::from(*damage_type).into(),
                affected_positions,
                target_count: u16::try_from(target_ids.len()).unwrap_or(u16::MAX),
            },
        });
        let trace = ProjectileTrace {
            origin: self.player.position,
            impact: self.player.position,
            landing: self.player.position,
            traversed: Vec::new(),
        };
        for entity_id in target_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            self.resolve_ability_damage_to_entity(
                index,
                &ability.id,
                DamageType::from(*damage_type),
                base_raw_damage,
                trace.clone(),
                events,
                changed,
                removed_entities,
            )?;
        }
        Ok(())
    }

    pub(super) fn resolve_player_death_ray_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::DeathRay { power } = ability.effect else {
            unreachable!("death ray executor requires a death ray effect");
        };
        let (trace, target_index) = self.trace_projectile_path(path);
        let Some(target_index) = target_index else {
            events.push(DomainEvent::AbilityLanded {
                ability_id: ability.id.clone(),
                trace,
            });
            return Ok(());
        };
        let target_entity_id = self.entities[target_index].id.clone();
        let target_kind_id = self.entities[target_index].kind_id.clone();
        let definition = self
            .content
            .actor(&target_kind_id)
            .expect("death ray target definition must remain available")
            .clone();
        let living = actor_matches_category(&definition, "living");
        let unique = definition.tags.iter().any(|tag| tag == "unique");
        let unique_roll = if living && unique {
            Some(
                u16::try_from(self.rng.bounded(888) + 1)
                    .expect("death ray unique roll must fit u16"),
            )
        } else {
            None
        };
        let unique_resisted = unique_roll.is_some_and(|roll| roll != 666);
        let (target_level_roll, caster_level_roll) = if living && !unique_resisted {
            (
                Some(
                    u16::try_from(self.rng.bounded(20) + 1)
                        .expect("death ray target roll must fit u16"),
                ),
                Some(
                    u32::try_from(self.rng.bounded(u64::from(power.max(1))) + 1)
                        .expect("validated death ray caster roll must fit u32"),
                ),
            )
        } else {
            (None, None)
        };
        let resisted = !living
            || unique_resisted
            || target_level_roll.zip(caster_level_roll).is_some_and(
                |(target_roll, caster_roll)| {
                    definition.level.saturating_add(u32::from(target_roll)) > caster_roll
                },
            );
        let damage = if resisted {
            None
        } else {
            let raw_damage = i32::from(self.progress.level).saturating_mul(200);
            let damage = resolve_damage(
                DamagePacket::new(raw_damage, DamageType::Curse),
                ResistanceLevel::Normal,
            );
            self.entities[target_index].alerted = true;
            let application = plan_damage_application(
                &self.entities[target_index],
                damage,
                FatalityPolicy::AtOrBelowZero,
            );
            commit_damage_application(&mut self.entities[target_index], &application);
            changed.insert(application.position);
            events.push(DomainEvent::AbilityHit {
                ability_id: ability.id.clone(),
                target_kind_id: target_kind_id.clone(),
                damage,
                trace: trace.clone(),
            });
            self.wake_entity_after_damage(target_index, damage.applied, events);
            if !application.fatal {
                self.resolve_monster_fear_aura(target_index, "hurt", true, events);
            }
            if application.fatal {
                self.resolve_actor_death(
                    target_index,
                    DomainEvent::AbilitySlew {
                        ability_id: ability.id.clone(),
                        target_kind_id: target_kind_id.clone(),
                        damage,
                        trace: trace.clone(),
                    },
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            Some(damage.into())
        };
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(target_entity_id),
                target_kind_id: Some(target_kind_id),
                effects: vec![AbilityEffectResolutionDto::DeathRay {
                    effect_index: 0,
                    power,
                    target_level: definition.level,
                    living,
                    unique,
                    unique_roll,
                    target_level_roll,
                    caster_level_roll,
                    resisted,
                    resolution: damage,
                }],
            },
            trace: Some(trace),
        });
        Ok(())
    }

    pub(super) fn resolve_player_drain_life_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::DrainLife {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            target_category,
            repeat,
            feeds,
        } = &ability.effect
        else {
            unreachable!("drain life executor requires a drain life effect");
        };
        if ability.id == DEATH_VAMPIRISM_TRUE_ABILITY_ID {
            self.add_virtue(VirtueKindDto::Sacrifice, -1);
            self.add_virtue(VirtueKindDto::Vitality, -1);
        }
        for _ in 0..*repeat {
            let (trace, target_index) = self.trace_projectile_path(path.clone());
            let Some(target_index) = target_index else {
                events.push(DomainEvent::AbilityLanded {
                    ability_id: ability.id.clone(),
                    trace: trace.clone(),
                });
                events.push(DomainEvent::AbilityEffectsResolved {
                    ability_id: ability.id.clone(),
                    resolution: AbilityEffectsResolutionDto {
                        target_entity_id: None,
                        target_kind_id: None,
                        effects: vec![AbilityEffectResolutionDto::Skipped {
                            effect_index: 0,
                            reason: AbilityEffectSkipReasonDto::NoTarget,
                        }],
                    },
                    trace: Some(trace),
                });
                continue;
            };
            let target_entity_id = self.entities[target_index].id.clone();
            let target_kind_id = self.entities[target_index].kind_id.clone();
            let eligible = self
                .content
                .actor(&target_kind_id)
                .is_some_and(|definition| actor_matches_category(definition, target_category));
            if !eligible {
                events.push(DomainEvent::AbilityEffectsResolved {
                    ability_id: ability.id.clone(),
                    resolution: AbilityEffectsResolutionDto {
                        target_entity_id: Some(target_entity_id),
                        target_kind_id: Some(target_kind_id),
                        effects: vec![AbilityEffectResolutionDto::Skipped {
                            effect_index: 0,
                            reason: AbilityEffectSkipReasonDto::Ineligible,
                        }],
                    },
                    trace: Some(trace),
                });
                continue;
            }
            let hp_before = self.entities[target_index].hp.max(0);
            let raw_damage = self
                .roll_damage(*damage_dice, *damage_sides)
                .saturating_add(i32::from(*damage_bonus))
                .max(0);
            let raw_damage = i32::try_from(spell_powered_ability_value(
                ability,
                0,
                AbilitySpellPowerField::FinalDamage,
                u64::try_from(raw_damage).expect("drain life damage must be non-negative"),
            ))
            .expect("spell-powered drain life damage must fit i32");
            let damage = self.resolve_ability_damage_to_entity(
                target_index,
                &ability.id,
                DamageType::from(*damage_type),
                raw_damage,
                trace.clone(),
                events,
                changed,
                removed_entities,
            )?;
            if ability.id == DEATH_VAMPIRIC_DRAIN_ABILITY_ID && damage.applied > 0 {
                self.add_virtue(VirtueKindDto::Sacrifice, -1);
                self.add_virtue(VirtueKindDto::Vitality, -1);
            }
            let requested = if !*feeds || self.nutrition < hunger::NUTRITION_FULL {
                damage.applied.min(hp_before)
            } else {
                0
            };
            let outcome = self.apply_player_healing(requested);
            let requested = outcome.requested;
            let applied = outcome.applied;
            if *feeds && damage.applied > 0 {
                let nutrition = u16::try_from(raw_damage.saturating_mul(100).min(5_000))
                    .expect("bounded vampiric nutrition must fit u16");
                if self.nutrition < rfb_protocol::PLAYER_NUTRITION_MAXIMUM {
                    self.nutrition = self
                        .nutrition
                        .saturating_add(nutrition)
                        .min(rfb_protocol::PLAYER_NUTRITION_MAXIMUM - 1);
                }
            }
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability.id.clone(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: Some(target_entity_id),
                    target_kind_id: Some(target_kind_id),
                    effects: vec![AbilityEffectResolutionDto::DrainLife {
                        effect_index: 0,
                        resolution: damage.into(),
                        healing: HealingResolutionDto { requested, applied },
                    }],
                },
                trace: Some(trace),
            });
        }
        Ok(())
    }

    pub(super) fn resolve_ability_control(
        &mut self,
        target_index: usize,
        effect_index: u8,
        category: &str,
        power: u16,
    ) -> AbilityEffectResolutionDto {
        let target_entity_id = self.entities[target_index].id.clone();
        let target_kind_id = self.entities[target_index].kind_id.clone();
        let definition = self
            .content
            .actor(&target_kind_id)
            .expect("controlled actor definition must remain available");
        let target_level = definition.level;
        let eligible = definition.tags.iter().any(|tag| tag == category);
        let already_controlled = self.entity_is_player_aligned(target_index);
        let (roll, outcome) = if already_controlled {
            (None, AbilityControlOutcomeDto::AlreadyControlled)
        } else if !eligible {
            (None, AbilityControlOutcomeDto::Ineligible)
        } else {
            let range = power.saturating_sub(10).max(1);
            let roll = u16::try_from(self.rng.bounded(u64::from(range)) + 1)
                .expect("validated control power roll must fit u16");
            if target_level > u32::from(roll).saturating_add(10) {
                (Some(roll), AbilityControlOutcomeDto::Resisted)
            } else {
                let pack = self.entities[target_index].pack.clone();
                if let Some(pack) = pack {
                    if pack.role == MonsterPackRoleDto::Leader || pack.leader_id == target_entity_id
                    {
                        for entity in &mut self.entities {
                            if entity
                                .pack
                                .as_ref()
                                .is_some_and(|identity| identity.id == pack.id)
                            {
                                entity.pack = None;
                            }
                        }
                    } else {
                        self.entities[target_index].pack = None;
                    }
                }
                self.entities[target_index].controller_id = Some(self.player.id.clone());
                (Some(roll), AbilityControlOutcomeDto::Controlled)
            }
        };
        AbilityEffectResolutionDto::Control {
            effect_index,
            category: category.to_owned(),
            power,
            target_entity_id,
            target_kind_id,
            target_level,
            roll,
            outcome,
        }
    }

    pub(super) fn resolve_player_control_effect(
        &mut self,
        ability: &AbilityDefinition,
        target_plan: AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::Control { category, power } = &ability.effect else {
            unreachable!("control executor requires a control effect");
        };
        let AbilityTargetPlan::Projectile { path, .. } = target_plan else {
            unreachable!("control effects require a projectile target plan");
        };
        let (trace, target_index) = self.trace_projectile_path(path);
        let Some(target_index) = target_index else {
            events.push(DomainEvent::AbilityLanded {
                ability_id: ability.id.clone(),
                trace: trace.clone(),
            });
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability.id.clone(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: None,
                    target_kind_id: None,
                    effects: vec![AbilityEffectResolutionDto::Skipped {
                        effect_index: 0,
                        reason: AbilityEffectSkipReasonDto::NoTarget,
                    }],
                },
                trace: Some(trace),
            });
            return;
        };
        let target_entity_id = self.entities[target_index].id.clone();
        let target_kind_id = self.entities[target_index].kind_id.clone();
        self.entities[target_index].alerted = true;
        changed.insert(self.entities[target_index].position);
        let resolution = self.resolve_ability_control(target_index, 0, category, *power);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(target_entity_id),
                target_kind_id: Some(target_kind_id),
                effects: vec![resolution],
            },
            trace: Some(trace),
        });
    }

    pub(super) fn resolve_player_actor_status_effect(
        &mut self,
        ability: &AbilityDefinition,
        target_plan: AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        debug_assert!(matches!(
            ability.effect,
            AbilityEffectDefinition::ApplyStatus { .. }
                | AbilityEffectDefinition::RemoveStatus { .. }
        ));
        match target_plan {
            AbilityTargetPlan::SelfTarget => {
                let resolution = match &ability.effect {
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
                    } => apply_ability_status_effect(
                        &mut self.player,
                        &ability.id,
                        0,
                        status_kind_id,
                        *intensity,
                        *duration_ticks,
                        *duration_dice,
                        *duration_sides,
                        *stacking,
                        *resistance_type,
                        *power,
                        granted_resistances,
                        granted_brands,
                        granted_modifiers,
                        granted_equipment_bonuses,
                        granted_status_immunities,
                        granted_race_id.as_deref(),
                        *grants_wall_passage,
                        *incoming_damage_percent,
                        None,
                        None,
                        &mut self.rng,
                    ),
                    AbilityEffectDefinition::RemoveStatus { status_kind_id } => {
                        remove_ability_status_effect(&mut self.player, 0, status_kind_id)
                    }
                    _ => unreachable!("actor status executor requires a status effect"),
                };
                events.push(DomainEvent::AbilityEffectsResolved {
                    ability_id: ability.id.clone(),
                    resolution: AbilityEffectsResolutionDto {
                        target_entity_id: Some(self.player.id.clone()),
                        target_kind_id: Some(self.player.kind_id.clone()),
                        effects: vec![resolution],
                    },
                    trace: None,
                });
                self.refresh_player_resource_maxima();
            }
            AbilityTargetPlan::Projectile { path, .. } => {
                let (trace, target_index) = self.trace_projectile_path(path);
                let Some(target_index) = target_index else {
                    events.push(DomainEvent::AbilityLanded {
                        ability_id: ability.id.clone(),
                        trace: trace.clone(),
                    });
                    events.push(DomainEvent::AbilityEffectsResolved {
                        ability_id: ability.id.clone(),
                        resolution: AbilityEffectsResolutionDto {
                            target_entity_id: None,
                            target_kind_id: None,
                            effects: vec![AbilityEffectResolutionDto::Skipped {
                                effect_index: 0,
                                reason: AbilityEffectSkipReasonDto::NoTarget,
                            }],
                        },
                        trace: Some(trace),
                    });
                    return;
                };
                let target_entity_id = self.entities[target_index].id.clone();
                let target_kind_id = self.entities[target_index].kind_id.clone();
                let resolution = match &ability.effect {
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
                    } => {
                        let target_level = self
                            .content
                            .actor(&target_kind_id)
                            .map(|definition| definition.level);
                        self.entities[target_index].alerted = true;
                        changed.insert(self.entities[target_index].position);
                        apply_ability_status_effect(
                            &mut self.entities[target_index],
                            &ability.id,
                            0,
                            status_kind_id,
                            *intensity,
                            *duration_ticks,
                            *duration_dice,
                            *duration_sides,
                            *stacking,
                            *resistance_type,
                            *power,
                            granted_resistances,
                            granted_brands,
                            granted_modifiers,
                            granted_equipment_bonuses,
                            granted_status_immunities,
                            granted_race_id.as_deref(),
                            *grants_wall_passage,
                            *incoming_damage_percent,
                            target_level,
                            None,
                            &mut self.rng,
                        )
                    }
                    AbilityEffectDefinition::RemoveStatus { status_kind_id } => {
                        self.entities[target_index].alerted = true;
                        changed.insert(self.entities[target_index].position);
                        remove_ability_status_effect(
                            &mut self.entities[target_index],
                            0,
                            status_kind_id,
                        )
                    }
                    _ => unreachable!("actor status executor requires a status effect"),
                };
                events.push(DomainEvent::AbilityEffectsResolved {
                    ability_id: ability.id.clone(),
                    resolution: AbilityEffectsResolutionDto {
                        target_entity_id: Some(target_entity_id),
                        target_kind_id: Some(target_kind_id),
                        effects: vec![resolution],
                    },
                    trace: Some(trace),
                });
            }
            _ => unreachable!("actor status effects require a self or projectile target plan"),
        }
    }

    pub(super) fn resolve_player_ordered_sequence_effect(
        &mut self,
        ability: &AbilityDefinition,
        target_plan: AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::Sequence { effects } = &ability.effect else {
            unreachable!("ordered sequence executor requires a sequence effect");
        };
        if matches!(target_plan, AbilityTargetPlan::SelfTarget)
            && effects.iter().any(|effect| {
                !matches!(
                    effect,
                    AbilityEffectDefinition::Heal { .. }
                        | AbilityEffectDefinition::HealDice { .. }
                        | AbilityEffectDefinition::ReduceStatus { .. }
                        | AbilityEffectDefinition::ApplyStatus { .. }
                        | AbilityEffectDefinition::RemoveStatus { .. }
                )
            })
        {
            for effect in effects {
                let mut step = ability.clone();
                step.effect = effect.clone();
                let plan = match effect {
                    AbilityEffectDefinition::AreaDamage { .. } => AbilityTargetPlan::Projectile {
                        path: Vec::new(),
                        stop_at_actor: false,
                    },
                    AbilityEffectDefinition::Detect { .. } => AbilityTargetPlan::Detect,
                    AbilityEffectDefinition::Heal { .. }
                    | AbilityEffectDefinition::HealDice { .. }
                    | AbilityEffectDefinition::ReduceStatus { .. }
                    | AbilityEffectDefinition::LightArea { .. }
                    | AbilityEffectDefinition::ApplyStatus { .. }
                    | AbilityEffectDefinition::RemoveStatus { .. }
                    | AbilityEffectDefinition::VisibleDamage { .. }
                    | AbilityEffectDefinition::VisibleApplyStatus { .. }
                    | AbilityEffectDefinition::AggravateMonsters
                    | AbilityEffectDefinition::NoOp { .. } => AbilityTargetPlan::SelfTarget,
                    _ => unreachable!("validated self sequence must remain self-targeted"),
                };
                self.resolve_player_ability_effect(step, plan, events, changed, removed_entities)?;
            }
            return Ok(());
        }
        match target_plan {
            AbilityTargetPlan::SelfTarget => {
                let target_entity_id = self.player.id.clone();
                let target_kind_id = self.player.kind_id.clone();
                let mut resolutions = Vec::with_capacity(effects.len());
                for (index, effect) in effects.iter().enumerate() {
                    let effect_index =
                        u8::try_from(index).expect("validated ability effect index must fit u8");
                    let resolution = match effect {
                        AbilityEffectDefinition::Heal { amount } => {
                            let amount = i32::try_from(*amount)
                                .expect("validated healing amount must fit i32");
                            let outcome = self.apply_player_healing(amount);
                            AbilityEffectResolutionDto::Heal {
                                effect_index,
                                resolution: HealingResolutionDto {
                                    requested: outcome.requested,
                                    applied: outcome.applied,
                                },
                            }
                        }
                        AbilityEffectDefinition::HealDice { dice, sides } => {
                            let amount = self.roll_damage(*dice, *sides).max(0);
                            let outcome = self.apply_player_healing(amount);
                            AbilityEffectResolutionDto::Heal {
                                effect_index,
                                resolution: HealingResolutionDto {
                                    requested: outcome.requested,
                                    applied: outcome.applied,
                                },
                            }
                        }
                        AbilityEffectDefinition::ReduceStatus {
                            status_kind_id,
                            amount,
                            current_divisor,
                            remaining_divisor,
                        } => {
                            let (before, after) = self.reduce_player_status(
                                status_kind_id,
                                *amount,
                                *current_divisor,
                                *remaining_divisor,
                            );
                            AbilityEffectResolutionDto::ReduceStatus {
                                effect_index,
                                status_kind_id: status_kind_id.clone(),
                                before,
                                after,
                            }
                        }
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
                        } => apply_ability_status_effect(
                            &mut self.player,
                            &ability.id,
                            effect_index,
                            status_kind_id,
                            *intensity,
                            *duration_ticks,
                            *duration_dice,
                            *duration_sides,
                            *stacking,
                            *resistance_type,
                            *power,
                            granted_resistances,
                            granted_brands,
                            granted_modifiers,
                            granted_equipment_bonuses,
                            granted_status_immunities,
                            granted_race_id.as_deref(),
                            *grants_wall_passage,
                            *incoming_damage_percent,
                            None,
                            None,
                            &mut self.rng,
                        ),
                        AbilityEffectDefinition::RemoveStatus { status_kind_id } => {
                            remove_ability_status_effect(
                                &mut self.player,
                                effect_index,
                                status_kind_id,
                            )
                        }
                        _ => unreachable!(
                            "validated self-target effect sequences contain only actor effects"
                        ),
                    };
                    resolutions.push(resolution);
                }
                events.push(DomainEvent::AbilityEffectsResolved {
                    ability_id: ability.id.clone(),
                    resolution: AbilityEffectsResolutionDto {
                        target_entity_id: Some(target_entity_id),
                        target_kind_id: Some(target_kind_id),
                        effects: resolutions,
                    },
                    trace: None,
                });
                self.refresh_player_resource_maxima();
            }
            AbilityTargetPlan::Projectile { path, .. } => {
                let (trace, target_index) = self.trace_projectile_path(path);
                let Some(target_index) = target_index else {
                    let resolutions = effects
                        .iter()
                        .enumerate()
                        .map(|(index, _)| AbilityEffectResolutionDto::Skipped {
                            effect_index: u8::try_from(index)
                                .expect("validated ability effect index must fit u8"),
                            reason: AbilityEffectSkipReasonDto::NoTarget,
                        })
                        .collect();
                    events.push(DomainEvent::AbilityLanded {
                        ability_id: ability.id.clone(),
                        trace: trace.clone(),
                    });
                    events.push(DomainEvent::AbilityEffectsResolved {
                        ability_id: ability.id.clone(),
                        resolution: AbilityEffectsResolutionDto {
                            target_entity_id: None,
                            target_kind_id: None,
                            effects: resolutions,
                        },
                        trace: Some(trace),
                    });
                    return Ok(());
                };

                let target_entity_id = self.entities[target_index].id.clone();
                let target_kind_id = self.entities[target_index].kind_id.clone();
                let mut resolutions = Vec::with_capacity(effects.len());
                for (index, effect) in effects.iter().enumerate() {
                    let effect_index =
                        u8::try_from(index).expect("validated ability effect index must fit u8");
                    let Some(current_index) = self
                        .entities
                        .iter()
                        .position(|entity| entity.id == target_entity_id && entity.hp > 0)
                    else {
                        resolutions.push(AbilityEffectResolutionDto::Skipped {
                            effect_index,
                            reason: AbilityEffectSkipReasonDto::TargetDead,
                        });
                        continue;
                    };
                    let resolution = match effect {
                        AbilityEffectDefinition::Damage {
                            damage_dice,
                            damage_sides,
                            damage_bonus,
                            damage_type,
                        } => {
                            let raw_damage = self
                                .roll_damage(*damage_dice, *damage_sides)
                                .saturating_add(i32::from(*damage_bonus))
                                .max(0);
                            let damage = self.resolve_ability_damage_to_entity(
                                current_index,
                                &ability.id,
                                DamageType::from(*damage_type),
                                raw_damage,
                                trace.clone(),
                                events,
                                changed,
                                removed_entities,
                            )?;
                            AbilityEffectResolutionDto::Damage {
                                effect_index,
                                resolution: damage.into(),
                            }
                        }
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
                        } => {
                            let target_level = self
                                .content
                                .actor(&self.entities[current_index].kind_id)
                                .map(|definition| definition.level);
                            self.entities[current_index].alerted = true;
                            changed.insert(self.entities[current_index].position);
                            apply_ability_status_effect(
                                &mut self.entities[current_index],
                                &ability.id,
                                effect_index,
                                status_kind_id,
                                *intensity,
                                *duration_ticks,
                                *duration_dice,
                                *duration_sides,
                                *stacking,
                                *resistance_type,
                                *power,
                                granted_resistances,
                                granted_brands,
                                granted_modifiers,
                                granted_equipment_bonuses,
                                granted_status_immunities,
                                granted_race_id.as_deref(),
                                *grants_wall_passage,
                                *incoming_damage_percent,
                                target_level,
                                None,
                                &mut self.rng,
                            )
                        }
                        AbilityEffectDefinition::RemoveStatus { status_kind_id } => {
                            self.entities[current_index].alerted = true;
                            changed.insert(self.entities[current_index].position);
                            remove_ability_status_effect(
                                &mut self.entities[current_index],
                                effect_index,
                                status_kind_id,
                            )
                        }
                        AbilityEffectDefinition::Control { category, power } => {
                            self.entities[current_index].alerted = true;
                            changed.insert(self.entities[current_index].position);
                            self.resolve_ability_control(
                                current_index,
                                effect_index,
                                category,
                                *power,
                            )
                        }
                        _ => unreachable!(
                            "validated projectile effect sequences contain only actor effects"
                        ),
                    };
                    resolutions.push(resolution);
                }
                events.push(DomainEvent::AbilityEffectsResolved {
                    ability_id: ability.id.clone(),
                    resolution: AbilityEffectsResolutionDto {
                        target_entity_id: Some(target_entity_id),
                        target_kind_id: Some(target_kind_id),
                        effects: resolutions,
                    },
                    trace: Some(trace),
                });
            }
            _ => unreachable!("effect sequences require a self or projectile target plan"),
        }
        Ok(())
    }

    pub(super) fn resolve_player_visible_status_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::VisibleApplyStatus {
            status_kind_id,
            intensity,
            duration_ticks,
            stacking,
            resistance_type,
            power,
            target_category,
        } = &ability.effect
        else {
            unreachable!("visible status executor requires a visible status effect");
        };
        let target_ids = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && self.entity_is_visible_to_player(entity)
                    && target_category.as_ref().is_none_or(|category| {
                        self.content
                            .actor(&entity.kind_id)
                            .is_some_and(|definition| actor_matches_category(definition, category))
                    })
            })
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        let empty_resistances = BTreeMap::new();
        let empty_brands = BTreeSet::new();
        let empty_immunities = BTreeSet::new();
        for entity_id in target_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            let target_kind_id = self.entities[index].kind_id.clone();
            let target_level = self
                .content
                .actor(&target_kind_id)
                .map(|definition| definition.level);
            let resolution = apply_ability_status_effect(
                &mut self.entities[index],
                &ability.id,
                0,
                status_kind_id,
                *intensity,
                *duration_ticks,
                0,
                0,
                *stacking,
                *resistance_type,
                *power,
                &empty_resistances,
                &empty_brands,
                &StatModifiers::default(),
                &EquipmentBonuses::default(),
                &empty_immunities,
                None,
                false,
                100,
                target_level,
                None,
                &mut self.rng,
            );
            changed.insert(self.entities[index].position);
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability.id.clone(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: Some(entity_id),
                    target_kind_id: Some(target_kind_id),
                    effects: vec![resolution],
                },
                trace: None,
            });
        }
    }

    pub(super) fn resolve_player_mass_sleep_or_stasis_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::MassSleepOrStasis { stasis, power, .. } = ability.effect
        else {
            unreachable!("mass sleep executor requires a mass sleep effect");
        };
        let (status_kind_id, duration_ticks, stacking) = if stasis {
            (
                STATUS_PARALYSIS,
                20,
                AbilityStatusStackingDefinition::Extend,
            )
        } else {
            (
                STATUS_SLEEP,
                500,
                AbilityStatusStackingDefinition::KeepStrongest,
            )
        };
        let target_ids = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && self.entity_is_visible_to_player(entity)
                    && (!stasis
                        || self
                            .content
                            .actor(&entity.kind_id)
                            .is_some_and(|definition| {
                                !definition
                                    .tags
                                    .iter()
                                    .any(|tag| matches!(tag.as_str(), "unique" | "unique2"))
                            }))
            })
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        let empty_resistances = BTreeMap::new();
        let empty_brands = BTreeSet::new();
        let empty_immunities = BTreeSet::new();
        for entity_id in target_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            let target_kind_id = self.entities[index].kind_id.clone();
            let target_level = self
                .content
                .actor(&target_kind_id)
                .map(|definition| definition.level);
            let mut resolution = apply_ability_status_effect(
                &mut self.entities[index],
                &ability.id,
                0,
                status_kind_id,
                1,
                duration_ticks,
                0,
                0,
                stacking,
                None,
                Some(power),
                &empty_resistances,
                &empty_brands,
                &StatModifiers::default(),
                &EquipmentBonuses::default(),
                &empty_immunities,
                None,
                false,
                100,
                target_level,
                None,
                &mut self.rng,
            );
            if stasis
                && let AbilityEffectResolutionDto::ApplyStatus {
                    requested_duration_ticks,
                    applied_duration_ticks,
                    change,
                    ..
                } = &mut resolution
                && !matches!(
                    change,
                    AbilityStatusChangeDto::Immune | AbilityStatusChangeDto::Resisted
                )
                && self.rng.bounded(15) == 0
            {
                *requested_duration_ticks += 10;
                *applied_duration_ticks += 10;
                if let Some(status) = self.entities[index]
                    .statuses
                    .iter_mut()
                    .find(|status| status.kind_id == STATUS_PARALYSIS)
                {
                    status.remaining_ticks = status.remaining_ticks.saturating_add(10);
                }
            }
            changed.insert(self.entities[index].position);
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability.id.clone(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: Some(entity_id),
                    target_kind_id: Some(target_kind_id),
                    effects: vec![resolution],
                },
                trace: None,
            });
        }
    }

    pub(super) fn resolve_player_healing_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::Heal { amount } = ability.effect else {
            unreachable!("player healing executor requires a healing effect");
        };
        let amount = i32::try_from(amount).expect("validated healing amount must fit i32");
        let outcome = self.apply_player_healing(amount);
        events.push(DomainEvent::AbilityHealed {
            ability_id: ability.id.clone(),
            resolution: HealingResolutionDto {
                requested: outcome.requested,
                applied: outcome.applied,
            },
        });
    }

    fn resolve_player_healing_dice_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::HealDice { dice, sides } = ability.effect else {
            unreachable!("player healing-dice executor requires a healing-dice effect");
        };
        let amount = self.roll_damage(dice, sides).max(0);
        let outcome = self.apply_player_healing(amount);
        events.push(DomainEvent::AbilityHealed {
            ability_id: ability.id.clone(),
            resolution: HealingResolutionDto {
                requested: outcome.requested,
                applied: outcome.applied,
            },
        });
    }

    fn resolve_player_status_reduction_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::ReduceStatus {
            ref status_kind_id,
            amount,
            current_divisor,
            remaining_divisor,
        } = ability.effect
        else {
            unreachable!("status reduction executor requires a reduce-status effect");
        };
        let (before, after) =
            self.reduce_player_status(status_kind_id, amount, current_divisor, remaining_divisor);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::ReduceStatus {
                    effect_index: 0,
                    status_kind_id: status_kind_id.clone(),
                    before,
                    after,
                }],
            },
            trace: None,
        });
    }

    fn reduce_player_status(
        &mut self,
        status_kind_id: &str,
        minimum_amount: u32,
        current_divisor: Option<u32>,
        remaining_divisor: Option<u32>,
    ) -> (u32, u32) {
        self.player
            .statuses
            .iter()
            .position(|status| status.kind_id == status_kind_id)
            .map_or((0, 0), |index| {
                let before = self.player.statuses[index].remaining_ticks;
                let after = remaining_divisor.map_or_else(
                    || {
                        let amount = current_divisor.map_or(minimum_amount, |divisor| {
                            minimum_amount.max(before / divisor)
                        });
                        before.saturating_sub(amount)
                    },
                    |divisor| (before / divisor).saturating_sub(minimum_amount),
                );
                if after == 0 {
                    self.player.statuses.remove(index);
                } else {
                    self.player.statuses[index].remaining_ticks = after;
                }
                (before, after)
            })
    }

    fn resolve_player_satisfy_hunger_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let before_state = self.nutrition_state();
        let nutrition_before = self.nutrition;
        self.nutrition = rfb_protocol::PLAYER_NUTRITION_MAXIMUM - 1;
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::SatisfyHunger {
                    effect_index: 0,
                    nutrition_before,
                    nutrition_after: self.nutrition,
                }],
            },
            trace: None,
        });
        let after_state = self.nutrition_state();
        if after_state != before_state {
            events.push(DomainEvent::NutritionStateChanged {
                from: before_state,
                to: after_state,
                nutrition: self.nutrition,
            });
        }
    }

    fn resolve_player_refuel_equipped_light_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::RefuelEquippedLight {
            maximum_fraction_divisor,
        } = ability.effect
        else {
            unreachable!("light refuel executor requires a light refuel effect");
        };
        let mut item_id = None;
        let mut before = 0;
        let mut after = 0;
        if let Some(item) = self.items.iter_mut().find(|item| {
            matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == "light")
                && item.fuel.is_some_and(|fuel| {
                    matches!(
                        fuel.kind,
                        rfb_protocol::ItemFuelKindDto::Torch
                            | rfb_protocol::ItemFuelKindDto::Lantern
                    )
                })
        }) {
            item_id = Some(item.id.clone());
            let fuel = item.fuel.as_mut().expect("selected light must retain fuel");
            before = fuel.current;
            fuel.current = fuel
                .current
                .saturating_add(fuel.maximum / maximum_fraction_divisor)
                .min(fuel.maximum);
            after = fuel.current;
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::RefuelEquippedLight {
                    effect_index: 0,
                    item_id,
                    before,
                    after,
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_identify_item_effect(
        &mut self,
        ability: &AbilityDefinition,
        item_id: &str,
        events: &mut Vec<DomainEvent>,
    ) {
        let (full_identify_power, full_identify_roll_sides) = match ability.effect {
            AbilityEffectDefinition::IdentifyItem {
                full_identify_power,
                full_identify_roll_sides,
            } => (full_identify_power, full_identify_roll_sides),
            AbilityEffectDefinition::IdentifyOrMassIdentify { mass: false, .. } => (0, 0),
            _ => unreachable!("item identification executor requires an identify item effect"),
        };
        let roll = if full_identify_roll_sides == 0 {
            0
        } else {
            u16::try_from(self.rng.bounded(u64::from(full_identify_roll_sides)) + 1)
                .expect("validated identify roll must fit u16")
        };
        let full = full_identify_roll_sides > 0 && roll <= full_identify_power;
        let identification =
            self.identify_item_instance(item_id, ItemIdentificationRequest::new(full));
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::IdentifyItem {
                    effect_index: 0,
                    item_id: identification.item_id,
                    item_kind_id: identification.item_kind_id,
                    full_identify_power,
                    full_identify_roll_sides,
                    roll,
                    full,
                    changed: identification.changed,
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_mass_identify_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::IdentifyOrMassIdentify { mass: true, .. } = ability.effect
        else {
            unreachable!("mass identification executor requires the upgraded identify effect");
        };
        let mut item_ids = self
            .items
            .iter()
            .filter(|item| {
                item.quantity > 0
                    && matches!(
                        item.location,
                        ItemLocation::Inventory | ItemLocation::Equipped { .. }
                    )
            })
            .map(|item| item.id.clone())
            .collect::<Vec<_>>();
        item_ids.sort();
        let effects = item_ids
            .into_iter()
            .map(|item_id| {
                let identification =
                    self.identify_item_instance(&item_id, ItemIdentificationRequest::new(false));
                AbilityEffectResolutionDto::IdentifyItem {
                    effect_index: 0,
                    item_id: identification.item_id,
                    item_kind_id: identification.item_kind_id,
                    full_identify_power: 0,
                    full_identify_roll_sides: 0,
                    roll: 0,
                    full: false,
                    changed: identification.changed,
                }
            })
            .collect();
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects,
            },
            trace: None,
        });
    }

    fn item_is_brandable_weapon(&self, item: &ItemInstance) -> bool {
        (matches!(
            &item.location,
            ItemLocation::Inventory | ItemLocation::Equipped { .. }
        ) || item.location == ItemLocation::Ground(self.player.position))
            && item.affix_ids.is_empty()
            && item.rolled_affixes.is_empty()
            && self.content.item(&item.kind_id).is_some_and(|definition| {
                definition.melee_profile.is_some()
                    && !definition
                        .tags
                        .iter()
                        .any(|tag| matches!(tag.as_str(), "artifact" | "unbrandable"))
            })
    }

    pub(super) fn resolve_player_brand_weapon_effect(
        &mut self,
        ability: &AbilityDefinition,
        item_id: &str,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::BrandWeapon {
            affix_id,
            brand,
            resistance,
        } = &ability.effect
        else {
            unreachable!("weapon branding executor requires a weapon branding effect");
        };
        let item_index = self
            .items
            .iter()
            .position(|item| item.id == item_id)
            .expect("planned branding target must remain available");
        let item_kind_id = self.items[item_index].kind_id.clone();
        let mut properties = AffixPropertyBundleDefinition::default();
        if let Some(brand) = brand {
            properties.brands.insert(*brand);
        }
        if let Some(resistance) = resistance {
            properties
                .resistances
                .insert(*resistance, ActorResistanceLevel::Resistant);
        }
        let item = &mut self.items[item_index];
        item.affix_ids.push(affix_id.clone());
        item.affix_ids.sort();
        if properties != AffixPropertyBundleDefinition::default() {
            item.rolled_affixes.push(RolledAffixState {
                affix_id: affix_id.clone(),
                properties,
            });
            item.rolled_affixes
                .sort_by(|left, right| left.affix_id.cmp(&right.affix_id));
        }
        if item.quality == ItemQualityDto::Ordinary {
            item.quality = ItemQualityDto::Fine;
        }
        item.origin_kind = Some(ItemOriginKindDto::PlayerMade);
        item.discount_percent = 99;

        let enchantment_attempts = u16::try_from(self.rng.bounded(3) + 4)
            .expect("branding enchantment attempts must fit u16");
        let enchantment = self.enchant_item_instance(
            item_id,
            ItemEnchantmentRequest::new(enchantment_attempts, enchantment_attempts, 0),
        );
        if matches!(
            ability.id.as_str(),
            DEATH_POISON_BRANDING_ABILITY_ID | DEATH_VAMPIRIC_BRANDING_ABILITY_ID
        ) {
            self.add_virtue(VirtueKindDto::Enchantment, 2);
        }
        self.identify_item_instance(item_id, ItemIdentificationRequest::new(true));
        self.clamp_player_hp_to_effective_max();
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::BrandWeapon {
                    effect_index: 0,
                    item_id: item_id.to_owned(),
                    item_kind_id,
                    affix_id: affix_id.clone(),
                    brand: brand.map(weapon_brand_dto),
                    resistance: resistance.map(DamageType::from).map(Into::into),
                    to_hit: ItemEnchantmentComponentResolutionDto {
                        attempts: enchantment.to_hit.attempts,
                        successes: enchantment.to_hit.successes,
                        before: enchantment.to_hit.before,
                        after: enchantment.to_hit.after,
                    },
                    to_damage: ItemEnchantmentComponentResolutionDto {
                        attempts: enchantment.to_damage.attempts,
                        successes: enchantment.to_damage.successes,
                        before: enchantment.to_damage.before,
                        after: enchantment.to_damage.after,
                    },
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_no_op_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::NoOp { reason } = &ability.effect else {
            unreachable!("no-op executor requires a no-op effect");
        };
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::NoOp {
                    effect_index: 0,
                    reason: reason.clone(),
                }],
            },
            trace: None,
        });
    }

    fn resolve_player_concentrate_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let maximum = self
            .sniper_max_concentration()
            .expect("validated concentrate ability requires a sniping profile");
        let before = self.sniper_concentration;
        self.sniper_concentration = before.saturating_add(1).min(maximum);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::Concentrate {
                    effect_index: 0,
                    before,
                    after: self.sniper_concentration,
                    maximum,
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_restore_vitality_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::RestoreVitality { life_force } = ability.effect else {
            unreachable!("vitality executor requires a restore vitality effect");
        };
        let experience = apply_experience_restoration(&mut self.progress);
        let life_force =
            self.restore_player_life_force(LifeForceRestorationRequest::at_least(life_force));
        self.apply_player_experience(0, events);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::RestoreVitality {
                    effect_index: 0,
                    experience_before: experience.before,
                    experience_after: experience.after,
                    life_force_before: life_force.before,
                    life_force_after: life_force.after,
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_teleport_effect(
        &mut self,
        ability: &AbilityDefinition,
        destination: Position,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let from = self.player.position;
        events.push(DomainEvent::AbilityTeleported {
            ability_id: ability.id.clone(),
            resolution: AbilityTeleportResolutionDto {
                from,
                to: destination,
            },
        });
        events.extend(self.relocate_player(destination, changed));
    }

    fn resolve_player_random_teleport_effect(
        &mut self,
        ability: &AbilityDefinition,
        candidates: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let index = usize::try_from(self.rng.bounded(candidates.len() as u64))
            .expect("bounded teleport candidate index must fit usize");
        self.resolve_player_teleport_effect(ability, candidates[index], events, changed);
    }

    fn resolve_player_dimension_door_effect(
        &mut self,
        ability: &AbilityDefinition,
        requested: Position,
        destination_valid: bool,
        fallback_candidates: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let failure_sides = u64::from(self.progress.level / 10 + 10);
        let failed = !destination_valid || self.rng.bounded(failure_sides) == 0;
        let destination = if failed {
            (!fallback_candidates.is_empty()).then(|| {
                let index = usize::try_from(self.rng.bounded(fallback_candidates.len() as u64))
                    .expect("bounded dimension door fallback index must fit usize");
                fallback_candidates[index]
            })
        } else {
            Some(requested)
        };
        if let Some(destination) = destination {
            self.resolve_player_teleport_effect(ability, destination, events, changed);
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::DimensionDoor {
                    effect_index: 0,
                    requested,
                    destination,
                    failed,
                }],
            },
            trace: None,
        });
    }

    fn resolve_player_teleport_town_effect(
        &mut self,
        ability: &AbilityDefinition,
        town_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> Result<(), CoreError> {
        let from_town_id = self.current_town().map(|town| town.id.clone());
        self.teleport_to_town(town_id)?;
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::TeleportTown {
                    effect_index: 0,
                    from_town_id,
                    to_town_id: town_id.to_owned(),
                }],
            },
            trace: None,
        });
        Ok(())
    }

    fn resolve_player_create_stair_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::CreateStair {
            up_terrain_id,
            down_terrain_id,
        } = &ability.effect
        else {
            unreachable!("create stair executor requires a create stair effect");
        };
        let position = self.player.position;
        let terrain_is_permanent = self
            .index(position)
            .and_then(|index| self.content.terrain(&self.terrain[index]))
            .is_some_and(|terrain| terrain.tags.iter().any(|tag| tag == "permanent"));
        let blocked = self.is_wilderness_floor()
            || self.current_floor_task_id().is_some()
            || terrain_is_permanent
            || self
                .floor_connections
                .iter()
                .any(|connection| connection.position == position);
        let (can_create_up, can_create_down) = if blocked {
            (false, false)
        } else {
            self.content
                .world(&self.world_id)
                .and_then(|world| {
                    world
                        .procedural_floors
                        .iter()
                        .find(|floor| floor.id == self.current_floor_id)
                })
                .map_or((false, false), |floor| {
                    (true, floor.next_floor_id.is_some())
                })
        };
        let terrain_id = match (can_create_up, can_create_down) {
            (true, true) if self.rng.bounded(100) < 50 => Some(up_terrain_id.clone()),
            (true, true) => Some(down_terrain_id.clone()),
            (true, false) => Some(up_terrain_id.clone()),
            (false, true) => Some(down_terrain_id.clone()),
            (false, false) => None,
        };
        if let Some(terrain_id) = terrain_id.as_deref() {
            self.replace_terrain_from_source(
                position,
                terrain_id,
                TerrainChangeSource::Magic,
                events,
                changed,
            );
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::CreateStair {
                    effect_index: 0,
                    position,
                    terrain_id,
                }],
            },
            trace: None,
        });
    }

    fn resolve_player_self_knowledge_effect(
        &self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        events.push(DomainEvent::AbilitySelfKnowledge {
            ability_id: ability.id.clone(),
            name_key: ability.name_key.clone(),
            report: self.self_knowledge_report(),
        });
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::SelfKnowledge { effect_index: 0 }],
            },
            trace: None,
        });
    }

    fn resolve_player_probe_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let mut entity_ids = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && self.entity_is_visible_to_player(entity)
                    && !self.entity_is_fuzzy_to_player(entity)
                    && has_line_of_effect(self, self.player.position, entity.position)
            })
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        entity_ids.sort();

        let mut targets = Vec::with_capacity(entity_ids.len());
        for entity_id in entity_ids {
            let index = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id)
                .expect("probed entity must remain available");
            let entity = self.entities[index].clone();
            let definition = self
                .actor_runtime_definition(&entity)
                .expect("probed actor definition must remain available")
                .clone();
            let stats = self.actor_derived_stats(&entity, &definition, false);
            let good = definition.tags.iter().any(|tag| tag == "good");
            let evil = definition.tags.iter().any(|tag| tag == "evil");
            let alignment = match (good, evil) {
                (true, true) => AbilityProbeAlignmentDto::GoodAndEvil,
                (true, false) => AbilityProbeAlignmentDto::Good,
                (false, true) => AbilityProbeAlignmentDto::Evil,
                (false, false) => AbilityProbeAlignmentDto::Neutral,
            };
            let faction = if self.actor_is_player_aligned(&entity) {
                EntityFactionDto::Player
            } else if self.actor_is_friendly(&entity) {
                EntityFactionDto::Friendly
            } else {
                EntityFactionDto::Hostile
            };
            let report = AbilityProbeTargetDto {
                entity_id,
                target_kind_id: entity.kind_id,
                hp: entity.hp,
                max_hp: entity.max_hp,
                speed: derived_speed(&stats.speed),
                alignment,
                faction,
            };
            if self.entities[index].appearance_kind_id.take().is_some() {
                changed.insert(entity.position);
            }
            events.push(DomainEvent::AbilityProbed {
                ability_id: ability.id.clone(),
                report: report.clone(),
            });
            targets.push(report);
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::Probe {
                    effect_index: 0,
                    targets,
                }],
            },
            trace: None,
        });
    }

    fn resolve_player_create_door_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::CreateDoor { terrain_id } = &ability.effect else {
            unreachable!("create-door executor requires a create-door effect");
        };
        let occupied = self
            .entities
            .iter()
            .filter(|entity| entity.hp > 0)
            .map(|entity| entity.position)
            .chain(self.items.iter().filter_map(|item| match item.location {
                ItemLocation::Ground(position) => Some(position),
                _ => None,
            }))
            .chain(self.gold_piles.iter().map(|pile| pile.position))
            .chain(
                self.floor_connections
                    .iter()
                    .map(|connection| connection.position),
            )
            .collect::<BTreeSet<_>>();
        let mut positions = Vec::new();
        for direction in TERRAIN_INTERACTION_DIRECTIONS {
            let position = self.position_in_direction(direction);
            let Some(index) = self.index(position) else {
                continue;
            };
            let Some(terrain) = self.content.terrain(&self.terrain[index]) else {
                continue;
            };
            let blocked_feature = terrain.trap.is_some()
                || terrain.tags.iter().any(|tag| {
                    matches!(
                        tag.as_str(),
                        "door"
                            | "passage"
                            | "permanent"
                            | "stairs-up"
                            | "stairs-down"
                            | "shaft"
                            | "dungeon-entry"
                            | "task-entry"
                            | "shop-entrance"
                    )
                });
            if !terrain.walkable || blocked_feature || occupied.contains(&position) {
                continue;
            }
            self.replace_terrain_from_source(
                position,
                terrain_id,
                TerrainChangeSource::Magic,
                events,
                changed,
            );
            positions.push(position);
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::CreateDoor {
                    effect_index: 0,
                    positions,
                }],
            },
            trace: None,
        });
    }

    fn resolve_player_device_mastery_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::DeviceMastery {
            duration_base,
            device_power_bonus,
        } = ability.effect
        else {
            unreachable!("device-mastery executor requires a device-mastery effect");
        };
        let resolution = apply_ability_status_effect(
            &mut self.player,
            &ability.id,
            0,
            STATUS_DEVICE_MASTERY,
            1,
            u32::from(duration_base),
            1,
            u32::from(duration_base),
            AbilityStatusStackingDefinition::KeepStrongest,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers {
                device_power_bonus,
                ..StatModifiers::default()
            },
            &EquipmentBonuses::default(),
            &BTreeSet::new(),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        let AbilityEffectResolutionDto::ApplyStatus {
            applied_duration_ticks,
            change,
            ..
        } = resolution
        else {
            unreachable!("device mastery must resolve as a status");
        };
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::DeviceMastery {
                    effect_index: 0,
                    duration_base,
                    duration_ticks: applied_duration_ticks,
                    device_power_bonus,
                    change,
                }],
            },
            trace: None,
        });
    }

    fn resolve_player_banish_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::Banish { maximum_distance } = ability.effect else {
            unreachable!("banish executor requires a banish effect");
        };
        let mut actor_ids = self.item_visible_actor_ids();
        actor_ids.sort();
        let before = actor_ids
            .iter()
            .filter_map(|actor_id| {
                self.entities
                    .iter()
                    .find(|entity| entity.id == *actor_id && entity.hp > 0)
                    .map(|entity| (actor_id.clone(), entity.position))
            })
            .collect::<Vec<_>>();
        let outcomes = self.banish_visible_actors(maximum_distance, actor_ids, changed);
        let targets = before
            .into_iter()
            .zip(outcomes)
            .map(|((entity_id, from), outcome)| match outcome {
                VisibleBanishmentOutcome::Resisted { target_kind_id } => AbilityBanishTargetDto {
                    entity_id,
                    target_kind_id,
                    resisted: true,
                    from,
                    to: None,
                },
                VisibleBanishmentOutcome::NoSpace { target_kind_id } => AbilityBanishTargetDto {
                    entity_id,
                    target_kind_id,
                    resisted: false,
                    from,
                    to: None,
                },
                VisibleBanishmentOutcome::Banished {
                    target_kind_id,
                    resolution,
                } => AbilityBanishTargetDto {
                    entity_id,
                    target_kind_id,
                    resisted: false,
                    from,
                    to: Some(resolution.to),
                },
            })
            .collect();
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::Banish {
                    effect_index: 0,
                    maximum_distance,
                    targets,
                }],
            },
            trace: None,
        });
    }

    fn resolve_player_invulnerability_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::Invulnerability {
            duration_dice,
            duration_sides,
            duration_bonus,
        } = ability.effect
        else {
            unreachable!("invulnerability executor requires an invulnerability effect");
        };
        let rolled = u64::try_from(self.roll_damage(duration_dice, duration_sides))
            .expect("validated invulnerability duration must be non-negative")
            .saturating_add(u64::from(duration_bonus));
        let duration_ticks = u32::try_from(spell_powered_ability_value(
            ability,
            0,
            AbilitySpellPowerField::InvulnerabilityDuration,
            rolled,
        ))
        .expect("spell-powered invulnerability duration must fit u32");
        let was_invulnerable = self.player_has_status_kind(STATUS_INVULNERABILITY);
        let resolution = apply_ability_status_effect(
            &mut self.player,
            &ability.id,
            0,
            STATUS_INVULNERABILITY,
            1,
            duration_ticks,
            0,
            0,
            AbilityStatusStackingDefinition::KeepStrongest,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers::default(),
            &EquipmentBonuses::default(),
            &BTreeSet::new(),
            None,
            false,
            0,
            None,
            None,
            &mut self.rng,
        );
        let AbilityEffectResolutionDto::ApplyStatus {
            applied_duration_ticks,
            change,
            ..
        } = resolution
        else {
            unreachable!("invulnerability must resolve as a status");
        };
        if !was_invulnerable && applied_duration_ticks > 0 {
            self.apply_invulnerability_opening_virtues();
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::Invulnerability {
                    effect_index: 0,
                    duration_ticks: applied_duration_ticks,
                    change,
                }],
            },
            trace: None,
        });
    }

    fn resolve_player_fetch_item_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::FetchItem {
            maximum_weight_tenths_pound,
        } = ability.effect
        else {
            unreachable!("fetch item executor requires a fetch item effect");
        };
        let candidate = path.iter().find_map(|position| {
            self.items
                .iter()
                .enumerate()
                .filter(|(_, item)| matches!(item.location, ItemLocation::Ground(at) if at == *position))
                .min_by(|left, right| left.1.id.cmp(&right.1.id))
                .map(|(index, item)| (index, item.id.clone(), *position))
        });
        let mut item_id = None;
        let mut from = None;
        let mut moved = false;
        if let Some((index, id, position)) = candidate {
            let weight = u32::from(self.item_weight_tenths_pound(&self.items[index].kind_id))
                .saturating_mul(self.items[index].quantity);
            item_id = Some(id);
            from = Some(position);
            if weight <= maximum_weight_tenths_pound {
                self.items[index].location = ItemLocation::Ground(self.player.position);
                changed.insert(position);
                changed.insert(self.player.position);
                moved = true;
            }
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::FetchItem {
                    effect_index: 0,
                    item_id,
                    from,
                    to: self.player.position,
                    moved,
                }],
            },
            trace: None,
        });
    }

    fn resolve_player_consume_terrain_effect(
        &mut self,
        ability: &AbilityDefinition,
        position: Position,
        source_terrain_id: String,
        target_terrain_id: String,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::ConsumeTerrain {
            nutrition: base_nutrition,
        } = ability.effect
        else {
            unreachable!("terrain consumption executor requires a consume-terrain effect");
        };
        let source = self
            .content
            .terrain(&source_terrain_id)
            .expect("planned consumed terrain must remain available");
        let nutrition = if source.tags.iter().any(|tag| tag == "vein") {
            base_nutrition.max(5_000)
        } else if source
            .tags
            .iter()
            .any(|tag| matches!(tag.as_str(), "diggable" | "door"))
        {
            base_nutrition
        } else {
            base_nutrition.max(10_000)
        };
        let nutrition_before = self.nutrition;
        self.increase_nutrition(nutrition);
        self.replace_terrain_from_source(
            position,
            &target_terrain_id,
            TerrainChangeSource::Magic,
            events,
            changed,
        );
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::ConsumeTerrain {
                    effect_index: 0,
                    position,
                    source_terrain_id,
                    target_terrain_id,
                    nutrition_before,
                    nutrition_after: self.nutrition,
                }],
            },
            trace: None,
        });
        events.extend(self.relocate_player(position, changed));
    }

    fn roll_rfb_m_bonus(&mut self, maximum: u16) -> u16 {
        let level = self.progress.level.min(127);
        let product = maximum.saturating_mul(level);
        let mut mean = i32::from(product / 128);
        if self.rng.bounded(128) < u64::from(product % 128) {
            mean += 1;
        }
        let mut deviation = maximum / 4;
        if self.rng.bounded(4) < u64::from(maximum % 4) {
            deviation += 1;
        }
        let value = if deviation == 0 {
            mean
        } else {
            let roll = self.rng.bounded(32_768);
            // Only deviations 1..=3 are reachable by Create Ammo's m_bonus
            // calls. These are the exact category boundaries from RFB's
            // 256-entry randnor table after scaling by RANDNOR_STD (64).
            let thresholds: &[u64] = match deviation {
                1 => &[22_245, 31_249, 32_677],
                2 => &[12_367, 22_245, 28_323, 31_249, 32_352, 32_677, 32_752],
                3 => &[
                    8_621, 16_166, 22_245, 26_818, 29_619, 31_249, 32_129, 32_515, 32_677, 32_740,
                    32_760,
                ],
                _ => unreachable!("Create Ammo m_bonus deviation is bounded by three"),
            };
            let offset = i32::try_from(thresholds.partition_point(|threshold| *threshold < roll))
                .expect("randnor category count fits i32");
            if self.rng.bounded(100) < 50 {
                mean.saturating_sub(offset)
            } else {
                mean.saturating_add(offset)
            }
        };
        u16::try_from(value.clamp(0, i32::from(maximum))).expect("bounded RFB bonus must fit u16")
    }

    fn roll_rfb_ammunition_magic_power(&mut self) -> i8 {
        let level = self.progress.level.min(127);
        let good_chance = level.saturating_add(10).min(75);
        let great_chance = good_chance.saturating_mul(2).saturating_div(3).min(20);
        if self.rng.bounded(100) < u64::from(good_chance) {
            return if self.rng.bounded(100) < u64::from(great_chance) {
                2
            } else {
                1
            };
        }
        if self.rng.bounded(100) >= u64::from(good_chance.saturating_add(2) / 3) {
            return 0;
        }
        if self.rng.bounded(100) < u64::from(great_chance) {
            return -2;
        }
        if self.rng.bounded(u64::from(level.max(1))).saturating_add(1) > 10 {
            0
        } else {
            -1
        }
    }

    fn roll_rfb_ammunition_ego(&mut self, level: u16) -> RolledAffixState {
        const SLAYING_AFFIX_ID: &str = "rfb-legacy.affix.slaying";
        const ELEMENTAL_AFFIX_ID: &str = "demo.affix.ammo-elemental";

        let rarity_weight = |rarity: u32, minimum_level: u16| {
            let effective = if level < minimum_level {
                rarity.saturating_add(rarity.saturating_mul(u32::from(minimum_level - level)))
            } else {
                rarity
            };
            (10_000 / effective).max(1)
        };
        let slaying_weight = rarity_weight(2, 10);
        let elemental_weight = rarity_weight(3, 20);
        if self
            .rng
            .bounded(u64::from(slaying_weight + elemental_weight))
            < u64::from(slaying_weight)
        {
            let mut properties = AffixPropertyBundleDefinition::default();
            let slays = [
                (SlayTarget::Orc, 2_u32, 20_u16),
                (SlayTarget::Troll, 2, 30),
                (SlayTarget::Giant, 2, 40),
                (SlayTarget::Dragon, 3, 80),
                (SlayTarget::Demon, 3, 90),
                (SlayTarget::Undead, 3, 95),
                (SlayTarget::Animal, 2, 60),
                (SlayTarget::Human, 3, 50),
                (SlayTarget::Evil, 5, 0),
                (SlayTarget::Good, 5, 0),
                (SlayTarget::Living, 20, 0),
            ];
            let eligible = slays
                .iter()
                .copied()
                .filter(|(_, _, maximum_level)| *maximum_level == 0 || level <= *maximum_level)
                .collect::<Vec<_>>();
            let total = eligible
                .iter()
                .map(|(_, rarity, _)| (255 / rarity).max(1))
                .sum::<u32>();
            let mut rolls = 1_u16.saturating_add(self.roll_rfb_m_bonus(4));
            if self.rng.bounded(8) == 0 {
                rolls = rolls.saturating_mul(2);
            }
            rolls = rolls.saturating_add(1) / 2;
            for _ in 0..rolls {
                let mut choice = u32::try_from(self.rng.bounded(u64::from(total)))
                    .expect("slaying choice fits u32");
                let (target, rarity, _) = eligible
                    .iter()
                    .copied()
                    .find(|(_, rarity, _)| {
                        let weight = (255 / rarity).max(1);
                        if choice < weight {
                            true
                        } else {
                            choice -= weight;
                            false
                        }
                    })
                    .expect("positive slaying weight must select a target");
                let level = if self.rng.bounded(u64::from(rarity.pow(3))) == 0 {
                    SlayLevel::Kill
                } else {
                    SlayLevel::Slay
                };
                properties
                    .slays
                    .entry(target)
                    .and_modify(|current| *current = (*current).max(level))
                    .or_insert(level);
            }
            RolledAffixState {
                affix_id: SLAYING_AFFIX_ID.to_owned(),
                properties,
            }
        } else {
            let mut properties = AffixPropertyBundleDefinition::default();
            let mut rolls = 1_u16.saturating_add(self.roll_rfb_m_bonus(4));
            if self.rng.bounded(8) == 0 {
                rolls = rolls.saturating_mul(2);
            }
            rolls = rolls.saturating_add(1) / 2;
            for _ in 0..rolls {
                properties.brands.insert(match self.rng.bounded(5) {
                    0 => WeaponBrand::Acid,
                    1 => WeaponBrand::Electricity,
                    2 => WeaponBrand::Fire,
                    3 => WeaponBrand::Cold,
                    _ => WeaponBrand::Poison,
                });
            }
            RolledAffixState {
                affix_id: ELEMENTAL_AFFIX_ID.to_owned(),
                properties,
            }
        }
    }

    pub(super) fn apply_rfb_ammunition_magic(&mut self, item: &mut ItemInstance) {
        let level = self.progress.level.min(127);
        let power = self.roll_rfb_ammunition_magic_power();
        item.origin_kind = Some(ItemOriginKindDto::PlayerMade);
        item.discount_percent = 99;
        item.quality = match power {
            1 => ItemQualityDto::Fine,
            2 => ItemQualityDto::Exceptional,
            _ => ItemQualityDto::Ordinary,
        };
        if power == 0 {
            return;
        }

        let primary_to_hit = 1_u16
            .saturating_add(u16::try_from(self.rng.bounded(5)).expect("d5 roll fits u16"))
            .saturating_add(self.roll_rfb_m_bonus(5));
        let primary_to_damage = 1_u16
            .saturating_add(u16::try_from(self.rng.bounded(5)).expect("d5 roll fits u16"))
            .saturating_add(self.roll_rfb_m_bonus(5));
        let extra_to_hit = self.roll_rfb_m_bonus(10).saturating_add(1) / 2;
        let extra_to_damage = self.roll_rfb_m_bonus(10).saturating_add(1) / 2;
        let sign = if power < 0 { -1_i16 } else { 1_i16 };
        item.enchantments.to_hit = sign.saturating_mul(
            i16::try_from(primary_to_hit.saturating_add(if power.abs() > 1 {
                extra_to_hit
            } else {
                0
            }))
            .expect("bounded ammunition enchantment fits i16"),
        );
        item.enchantments.to_damage = sign.saturating_mul(
            i16::try_from(primary_to_damage.saturating_add(if power.abs() > 1 {
                extra_to_damage
            } else {
                0
            }))
            .expect("bounded ammunition enchantment fits i16"),
        );
        if power < 0 {
            item.curse = Some(if power < -1 {
                ItemCurseSeverityDto::Heavy
            } else {
                ItemCurseSeverityDto::Normal
            });
            return;
        }
        if power < 2 {
            return;
        }

        let rolled_affix = self.roll_rfb_ammunition_ego(level);
        item.affix_ids = vec![rolled_affix.affix_id.clone()];
        item.rolled_affixes = vec![rolled_affix];
        let (base_dice, sides) = self
            .content
            .item(&item.kind_id)
            .and_then(|definition| definition.ammunition_profile.as_ref())
            .map(|profile| (profile.damage_dice, profile.damage_sides))
            .expect("created ammunition kind must remain ammunition");
        let mut dice = base_dice;
        if self
            .rng
            .bounded(u64::from(5_u16.saturating_add(200 / level.max(1))))
            == 0
        {
            loop {
                dice = dice.saturating_add(1);
                let odds = dice.saturating_mul(sides).saturating_div(2).max(1);
                if self.rng.bounded(u64::from(odds)) != 0 {
                    break;
                }
            }
            dice = dice.min(9);
        }
        if dice != base_dice {
            item.damage_dice_override = Some(dice);
        }
    }

    fn resolve_player_create_item_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::CreateItem {
            item_kind_id,
            quantity,
        } = &ability.effect
        else {
            unreachable!("item creation executor requires a create-item effect");
        };
        let draft = GeneratedItemDraft {
            kind_id: item_kind_id.clone(),
            quantity: *quantity,
            origin_kind: Some(ItemOriginKindDto::Acquire),
            quality: ItemQualityDto::Ordinary,
            affix_ids: Vec::new(),
            rolled_affixes: Vec::new(),
            curse: None,
            activation: None,
            charges: None,
            fuel: None,
        };
        let mut item =
            draft.into_item_instance(String::new(), ItemLocation::Ground(self.player.position));
        let position = self.created_item_drop_position(&item);
        item.location = ItemLocation::Ground(position);
        let maximum_stack = self
            .content
            .item(item_kind_id)
            .expect("validated created item must remain available")
            .max_stack;
        let mut stack_indices = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, existing)| {
                existing.location == ItemLocation::Ground(position)
                    && existing.quantity < maximum_stack
                    && super::inventory::item_instances_stack_compatible(existing, &item)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        stack_indices.sort_by(|left, right| self.items[*left].id.cmp(&self.items[*right].id));

        let mut destination_item_ids = Vec::new();
        for index in stack_indices {
            let transferred = item
                .quantity
                .min(maximum_stack - self.items[index].quantity);
            self.items[index].quantity += transferred;
            item.quantity -= transferred;
            destination_item_ids.push(self.items[index].id.clone());
            if item.quantity == 0 {
                break;
            }
        }
        if item.quantity > 0 {
            item.id = self.allocate_item_instance_id()?;
            destination_item_ids.push(item.id.clone());
            self.items.push(item);
        }
        changed.insert(position);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::CreateItem {
                    effect_index: 0,
                    item_kind_id: item_kind_id.clone(),
                    quantity: *quantity,
                    position,
                    destination_item_ids,
                }],
            },
            trace: None,
        });
        Ok(())
    }

    fn created_item_drop_position(&mut self, item: &ItemInstance) -> Position {
        let origin = self.player.position;
        let maximum_stack = self
            .content
            .item(&item.kind_id)
            .expect("validated created item must remain available")
            .max_stack;
        let mut best = None;
        let mut ties = 0_u64;
        for dy in -3..=3 {
            for dx in -3..=3 {
                let distance_squared = dx * dx + dy * dy;
                if distance_squared > 10 {
                    continue;
                }
                let position = Position {
                    x: origin.x + dx,
                    y: origin.y + dy,
                };
                if !self.is_walkable(position) || !has_line_of_effect(self, origin, position) {
                    continue;
                }
                let (pile_count, combines) = self
                    .items
                    .iter()
                    .filter(|existing| existing.location == ItemLocation::Ground(position))
                    .fold((0_usize, false), |(count, combines), existing| {
                        (
                            count + 1,
                            combines
                                || (existing.quantity < maximum_stack
                                    && super::inventory::item_instances_stack_compatible(
                                        existing, item,
                                    )),
                        )
                    });
                let pile_count = pile_count + usize::from(!combines);
                let score = 1_000_i64 - i64::from(distance_squared) - pile_count as i64 * 5;
                match best {
                    None => {
                        best = Some((score, position));
                        ties = 1;
                    }
                    Some((best_score, _)) if score > best_score => {
                        best = Some((score, position));
                        ties = 1;
                    }
                    Some((best_score, _)) if score == best_score => {
                        ties += 1;
                        if self.rng.bounded(ties) == 0 {
                            best = Some((score, position));
                        }
                    }
                    _ => {}
                }
            }
        }
        best.expect("the player's walkable grid must accept a created item")
            .1
    }

    fn resolve_player_create_ammunition_effect(
        &mut self,
        ability: &AbilityDefinition,
        source_item_id: Option<String>,
        source_terrain: Option<(Position, String, String)>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::CreateAmmunition {
            item_kind_ids,
            quantity_minimum,
            quantity_maximum,
            ..
        } = &ability.effect
        else {
            unreachable!("ammunition creation executor requires a create-ammunition effect");
        };
        let maximum_tier = u16::try_from(item_kind_ids.len() - 1)
            .expect("validated ammunition tier count must fit u16");
        let tier = usize::from(self.roll_rfb_m_bonus(maximum_tier));
        let item_kind_id = item_kind_ids[tier].clone();
        let quantity = quantity_minimum.saturating_add(
            u32::try_from(
                self.rng
                    .bounded(u64::from(quantity_maximum - quantity_minimum + 1)),
            )
            .expect("validated ammunition quantity must fit u32"),
        );

        let item_id = self.allocate_item_instance_id()?;
        let mut item = ItemInstance {
            id: item_id.clone(),
            kind_id: item_kind_id.clone(),
            quantity,
            inscription: None,
            origin_actor_kind_id: None,
            origin_kind: None,
            damage_dice_override: None,
            discount_percent: 0,
            quality: ItemQualityDto::Ordinary,
            affix_ids: Vec::new(),
            rolled_affixes: Vec::new(),
            enchantments: ItemEnchantmentsDto::default(),
            curse: None,
            activation: None,
            charges: None,
            fuel: None,
            device_recovery_progress: 0,
            captured_actor: None,
            location: ItemLocation::Inventory,
        };
        self.apply_rfb_ammunition_magic(&mut item);

        if let Some(item_id) = source_item_id.as_deref() {
            self.destroy_item(item_id, 1)
                .expect("planned ammunition material must remain destroyable");
        }
        if let Some((position, _, target_terrain_id)) = &source_terrain {
            self.replace_terrain_from_source(
                *position,
                target_terrain_id,
                TerrainChangeSource::Magic,
                events,
                changed,
            );
        }
        let destination_item_ids = if self.inventory_quantity_capacity_for(&item, false) >= quantity
        {
            self.carry_shop_purchase_item(item)
        } else {
            item.location = ItemLocation::Ground(self.player.position);
            self.items.push(item);
            changed.insert(self.player.position);
            vec![item_id]
        };
        self.mark_item_aware(&item_kind_id);
        for destination_id in &destination_item_ids {
            self.identify_item_instance(destination_id, ItemIdentificationRequest::new(true));
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::CreateAmmunition {
                    effect_index: 0,
                    source_item_id,
                    source_position: source_terrain.as_ref().map(|(position, _, _)| *position),
                    item_kind_id,
                    quantity,
                    destination_item_ids,
                }],
            },
            trace: None,
        });
        Ok(())
    }

    fn resolve_player_transmute_item_effect(
        &mut self,
        ability: &AbilityDefinition,
        item_id: &str,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::TransmuteItemToGold {
            value_divisor,
            unit_value_cap,
        } = ability.effect
        else {
            unreachable!("item transmutation executor requires a transmute-item-to-gold effect");
        };
        let item = self
            .items
            .iter()
            .find(|item| item.id == item_id)
            .expect("planned transmutation item must remain available")
            .clone();
        let unit_value = self
            .content
            .item(&item.kind_id)
            .expect("planned transmutation item definition must remain available")
            .base_value
            .saturating_div(u32::from(value_divisor))
            .min(unit_value_cap);
        let requested = unit_value.saturating_mul(item.quantity);
        self.destroy_item(item_id, item.quantity)
            .expect("planned transmutation must remain valid");
        let before = self.gold;
        self.gold = self
            .gold
            .saturating_add(requested)
            .min(super::gold::MAX_PLAYER_GOLD);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: Some(item.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::TransmuteItemToGold {
                    effect_index: 0,
                    item_id: item.id,
                    item_kind_id: item.kind_id,
                    quantity: item.quantity,
                    gold_gained: self.gold.saturating_sub(before),
                    gold_balance: self.gold,
                }],
            },
            trace: None,
        });
    }

    fn resolve_player_drain_item_magic_effect(
        &mut self,
        ability: &AbilityDefinition,
        item_id: &str,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::DrainItemMagic {
            base_power,
            level_multiplier,
            level_divisor,
        } = ability.effect
        else {
            unreachable!("magic drain executor requires a drain-item-magic effect");
        };
        let index = self
            .items
            .iter()
            .position(|item| item.id == item_id)
            .expect("planned magic drain item must remain available");
        let item_kind_id = self.items[index].kind_id.clone();
        let artifact = self
            .content
            .item(&item_kind_id)
            .is_some_and(|definition| definition.tags.iter().any(|tag| tag == "artifact"));
        let difficulty = self.items[index]
            .activation
            .as_ref()
            .map_or(0, |activation| {
                u32::try_from(activation.device_check_difficulty.max(0)).unwrap_or(0)
            });
        let charges_before = self.items[index]
            .charges
            .expect("planned magic drain item must retain charges")
            .current;
        let drained = difficulty.min(charges_before);
        let power = u32::from(base_power).saturating_add(
            u32::from(self.progress.level).saturating_mul(u32::from(level_multiplier))
                / u32::from(level_divisor),
        );
        let failure_odds = power.saturating_sub(difficulty / 2) / 5;
        let failed = failure_odds > 0 && self.rng.bounded(u64::from(failure_odds)) == 0;
        let mut destroyed = false;
        if failed && !artifact && self.rng.bounded(10) == 0 {
            if self.items[index].quantity == 1 {
                let removed = self.items.remove(index);
                self.item_property_knowledge.remove(&removed.id);
            } else {
                self.items[index].quantity -= 1;
            }
            destroyed = true;
        } else {
            let charges = self.items[index]
                .charges
                .as_mut()
                .expect("planned magic drain item must retain charges");
            charges.current = if failed {
                0
            } else {
                charges.current.saturating_sub(drained)
            };
        }
        let resource_id = self
            .casting_profile()
            .map(|profile| profile.resource_id.clone());
        let resource_before = resource_id
            .as_deref()
            .and_then(|id| self.resources.get(id))
            .map_or(0, |pool| pool.current);
        if !failed
            && let Some(resource_id) = resource_id.as_deref()
            && let Some(pool) = self.resources.get_mut(resource_id)
        {
            pool.current = pool.current.saturating_add(drained).min(pool.maximum);
        }
        let resource_after = resource_id
            .as_deref()
            .and_then(|id| self.resources.get(id))
            .map_or(0, |pool| pool.current);
        let charges_after = if destroyed {
            0
        } else {
            self.items
                .iter()
                .find(|item| item.id == item_id)
                .and_then(|item| item.charges)
                .map_or(0, |charges| charges.current)
        };
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: Some(item_kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::DrainItemMagic {
                    effect_index: 0,
                    item_id: item_id.to_owned(),
                    item_kind_id,
                    charges_before,
                    charges_after,
                    drained: if failed { charges_before } else { drained },
                    destroyed,
                    failed,
                    resource_id,
                    resource_before,
                    resource_after,
                }],
            },
            trace: None,
        });
    }

    fn resolve_player_report_magic_effect(
        &self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let mut statuses = self
            .player
            .statuses
            .iter()
            .map(StatusInstance::to_dto)
            .collect::<Vec<_>>();
        statuses.sort_by(|left, right| left.kind_id.cmp(&right.kind_id));
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::ReportMagic {
                    effect_index: 0,
                    statuses,
                    recall: self.recall.clone(),
                }],
            },
            trace: None,
        });
    }

    fn resolve_player_earthquake_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::Earthquake {
            radius,
            affect_chance_percent,
            ref floor_terrain_id,
            ref wall_terrain_ids,
        } = ability.effect
        else {
            unreachable!("earthquake executor requires an earthquake effect");
        };
        self.resolve_earthquake(
            self.player.position,
            radius,
            affect_chance_percent,
            floor_terrain_id,
            wall_terrain_ids,
            EarthquakeSource::Ability(ability.id.clone()),
            events,
            changed,
            removed_entities,
        )
    }

    fn resolve_player_area_destruction_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) {
        let AbilityEffectDefinition::AreaDestruction {
            minimum_radius,
            maximum_radius,
            ref floor_terrain_id,
            ref wall_terrain_id,
            ref quartz_terrain_id,
            ref magma_terrain_id,
        } = ability.effect
        else {
            unreachable!("area-destruction executor requires an area-destruction effect");
        };
        let (
            protected_floor,
            affected_positions,
            removed_entity_count,
            removed_items,
            removed_gold_piles,
        ) = if self.area_destruction_allowed() {
            let plan = self.plan_area_destruction(
                minimum_radius,
                maximum_radius,
                floor_terrain_id,
                wall_terrain_id,
                quartz_terrain_id,
                magma_terrain_id,
            );
            let outcome = self.apply_area_destruction_plan(plan, events, changed, removed_entities);
            (
                false,
                outcome.affected_positions,
                outcome.removed_entities,
                outcome.removed_items,
                outcome.removed_gold_piles,
            )
        } else {
            (true, Vec::new(), 0, 0, 0)
        };
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::AreaDestruction {
                    effect_index: 0,
                    protected_floor,
                    affected_positions,
                    removed_entities: u32::try_from(removed_entity_count).unwrap_or(u32::MAX),
                    removed_items: u32::try_from(removed_items).unwrap_or(u32::MAX),
                    removed_gold_piles: u32::try_from(removed_gold_piles).unwrap_or(u32::MAX),
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_monster_shatter_earthquake(
        &mut self,
        center: Position,
        source_kind_id: String,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        self.resolve_earthquake(
            center,
            8,
            15,
            "demo.terrain.floor",
            &[
                "demo.terrain.wall".to_owned(),
                "demo.terrain.quartz-vein".to_owned(),
                "demo.terrain.magma-vein".to_owned(),
            ],
            EarthquakeSource::Monster(source_kind_id),
            events,
            changed,
            removed_entities,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_earthquake(
        &mut self,
        center: Position,
        radius: u8,
        affect_chance_percent: u8,
        floor_terrain_id: &str,
        wall_terrain_ids: &[String],
        source: EarthquakeSource,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let radius_squared = i32::from(radius).pow(2);
        let mut affected_positions = Vec::new();
        for y in center.y - i32::from(radius)..=center.y + i32::from(radius) {
            for x in center.x - i32::from(radius)..=center.x + i32::from(radius) {
                let position = Position { x, y };
                let dx = x - center.x;
                let dy = y - center.y;
                if position == center
                    || dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy)) > radius_squared
                    || x <= 0
                    || y <= 0
                    || x >= i32::from(self.width) - 1
                    || y >= i32::from(self.height) - 1
                    || self
                        .floor_connections
                        .iter()
                        .any(|connection| connection.position == position)
                {
                    continue;
                }
                if self.rng.bounded(100) < u64::from(affect_chance_percent) {
                    affected_positions.push(position);
                }
            }
        }
        let affected = affected_positions.iter().copied().collect::<BTreeSet<_>>();
        let terrain_change_source = match &source {
            EarthquakeSource::Ability(_) => TerrainChangeSource::Magic,
            EarthquakeSource::Monster(_) => TerrainChangeSource::Monster,
        };
        let mut captured_balls = self
            .items
            .iter()
            .filter_map(|item| match item.location {
                ItemLocation::Ground(position)
                    if affected.contains(&position) && item.captured_actor.is_some() =>
                {
                    Some((item.id.clone(), position))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        captured_balls.sort_by(|left, right| left.0.cmp(&right.0));
        for (item_id, position) in captured_balls {
            self.force_open_capture_ball(&item_id, position, false, events, changed);
        }
        let removed_items = self
            .items
            .iter()
            .filter(|item| matches!(item.location, ItemLocation::Ground(position) if affected.contains(&position)))
            .count();
        self.items.retain(|item| {
            !matches!(item.location, ItemLocation::Ground(position) if affected.contains(&position))
        });
        let removed_gold_piles = self
            .gold_piles
            .iter()
            .filter(|pile| affected.contains(&pile.position))
            .count();
        self.gold_piles
            .retain(|pile| !affected.contains(&pile.position));

        let mut wall_positions = Vec::new();
        let mut floor_positions = Vec::new();
        for position in &affected_positions {
            if self.player.position == *position && !self.player_is_dead() {
                if self.player_evades_innate_monster_attacks() && self.rng.bounded(2) != 0 {
                    self.replace_terrain_from_source(
                        *position,
                        floor_terrain_id,
                        terrain_change_source,
                        events,
                        changed,
                    );
                    floor_positions.push(*position);
                    continue;
                }
                let raw_damage = self.roll_damage(4, 8);
                let damage = self.reduce_player_damage(resolve_damage(
                    DamagePacket::new(raw_damage, DamageType::Physical),
                    self.effective_player_resistances()
                        .level(DamageType::Physical),
                ));
                let application =
                    plan_damage_application(&self.player, damage, FatalityPolicy::BelowZero);
                commit_damage_application(&mut self.player, &application);
                match &source {
                    EarthquakeSource::Ability(ability_id) => {
                        events.push(DomainEvent::AbilityHit {
                            ability_id: ability_id.clone(),
                            target_kind_id: self.player.kind_id.clone(),
                            damage,
                            trace: ProjectileTrace {
                                origin: center,
                                impact: *position,
                                landing: *position,
                                traversed: vec![*position],
                            },
                        });
                    }
                    EarthquakeSource::Monster(source_kind_id) => {
                        events.push(DomainEvent::MonsterMeleeHit {
                            source_kind_id: source_kind_id.clone(),
                            method_id: Some("rfb.blow.shatter".to_owned()),
                            damage,
                        });
                        if application.fatal {
                            events.push(DomainEvent::PlayerDied {
                                source_kind_id: source_kind_id.clone(),
                                method_id: Some("rfb.blow.shatter".to_owned()),
                                damage,
                            });
                        }
                    }
                }
                self.replace_terrain_from_source(
                    *position,
                    floor_terrain_id,
                    terrain_change_source,
                    events,
                    changed,
                );
                floor_positions.push(*position);
                continue;
            }
            let actor_index = self
                .entities
                .iter()
                .position(|entity| entity.position == *position);
            if let Some(actor_index) = actor_index {
                let target_kind_id = self.entities[actor_index].kind_id.clone();
                let damage = resolve_damage(
                    DamagePacket::new(self.roll_damage(4, 8), DamageType::Physical),
                    self.entities[actor_index]
                        .resistances
                        .level(DamageType::Physical),
                );
                let application = plan_damage_application(
                    &self.entities[actor_index],
                    damage,
                    FatalityPolicy::AtOrBelowZero,
                );
                commit_damage_application(&mut self.entities[actor_index], &application);
                self.entities[actor_index].alerted = true;
                let trace = ProjectileTrace {
                    origin: center,
                    impact: *position,
                    landing: *position,
                    traversed: vec![*position],
                };
                if let EarthquakeSource::Ability(ability_id) = &source {
                    events.push(DomainEvent::AbilityHit {
                        ability_id: ability_id.clone(),
                        target_kind_id: target_kind_id.clone(),
                        damage,
                        trace: trace.clone(),
                    });
                }
                self.wake_entity_after_damage(actor_index, damage.applied, events);
                if !application.fatal {
                    self.resolve_monster_fear_aura(actor_index, "hurt", true, events);
                }
                if application.fatal {
                    match &source {
                        EarthquakeSource::Ability(ability_id) => self.resolve_actor_death(
                            actor_index,
                            DomainEvent::AbilitySlew {
                                ability_id: ability_id.clone(),
                                target_kind_id,
                                damage,
                                trace,
                            },
                            events,
                            changed,
                            removed_entities,
                        )?,
                        EarthquakeSource::Monster(source_kind_id) => self
                            .resolve_actor_death_without_rewards(
                                actor_index,
                                Some(DomainEvent::MonsterMeleeEntitySlew {
                                    source_kind_id: source_kind_id.clone(),
                                    target_kind_id,
                                    method_id: Some("rfb.blow.shatter".to_owned()),
                                    damage,
                                }),
                                events,
                                changed,
                                removed_entities,
                            )?,
                    }
                } else if let EarthquakeSource::Monster(source_kind_id) = &source {
                    events.push(DomainEvent::MonsterMeleeEntityHit {
                        source_kind_id: source_kind_id.clone(),
                        target_kind_id,
                        method_id: Some("rfb.blow.shatter".to_owned()),
                        damage,
                    });
                }
                self.replace_terrain_from_source(
                    *position,
                    floor_terrain_id,
                    terrain_change_source,
                    events,
                    changed,
                );
                floor_positions.push(*position);
            } else if self.is_walkable(*position) {
                let roll = self.rng.bounded(100);
                let wall_index = if wall_terrain_ids.len() == 1 || roll < 20 {
                    0
                } else if wall_terrain_ids.len() == 2 || roll < 70 {
                    1
                } else {
                    2
                };
                self.replace_terrain_from_source(
                    *position,
                    &wall_terrain_ids[wall_index],
                    terrain_change_source,
                    events,
                    changed,
                );
                wall_positions.push(*position);
            } else {
                self.replace_terrain_from_source(
                    *position,
                    floor_terrain_id,
                    terrain_change_source,
                    events,
                    changed,
                );
                floor_positions.push(*position);
            }
        }
        let resolution = AbilityEffectsResolutionDto {
            target_entity_id: None,
            target_kind_id: None,
            effects: vec![AbilityEffectResolutionDto::Earthquake {
                effect_index: 0,
                radius,
                affected_positions,
                wall_positions,
                floor_positions,
                removed_items: u32::try_from(removed_items).unwrap_or(u32::MAX),
                removed_gold_piles: u32::try_from(removed_gold_piles).unwrap_or(u32::MAX),
            }],
        };
        match source {
            EarthquakeSource::Ability(ability_id) => {
                events.push(DomainEvent::AbilityEffectsResolved {
                    ability_id,
                    resolution,
                    trace: None,
                });
            }
            EarthquakeSource::Monster(source_kind_id) => {
                events.push(DomainEvent::MonsterEarthquakeResolved {
                    source_kind_id,
                    resolution,
                });
            }
        }
        Ok(())
    }

    fn resolve_player_suppress_reproduction_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::SuppressMonsterReproduction {
            damage_dice,
            damage_sides,
            damage_bonus,
        } = ability.effect
        else {
            unreachable!("reproduction suppression executor requires its matching effect");
        };
        let damage = self
            .roll_damage(damage_dice, damage_sides)
            .saturating_add(i32::from(damage_bonus));
        self.player.hp = self.player.hp.saturating_sub(damage);
        let already_suppressed = self.reproduction_suppressed;
        self.reproduction_suppressed = true;
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::SuppressMonsterReproduction {
                    effect_index: 0,
                    damage,
                    fatal: self.player_is_dead(),
                    already_suppressed,
                }],
            },
            trace: None,
        });
    }

    fn resolve_player_melee_then_teleport_effect(
        &mut self,
        ability: &AbilityDefinition,
        target_entity_id: &str,
        teleport_candidates: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::MeleeThenTeleport {
            failure_threshold, ..
        } = ability.effect
        else {
            unreachable!("panic melee executor requires a melee-then-teleport effect");
        };
        let index = self
            .entities
            .iter()
            .position(|entity| entity.id == target_entity_id)
            .expect("planned panic-hit target must remain available");
        let target_kind_id = self.entities[index].kind_id.clone();
        let player_from = self.player.position;
        self.resolve_player_melee(index, false, events, changed, removed_entities)?;
        let skill =
            u64::try_from(self.player_derived_stats().disarm_skill.value.max(1)).unwrap_or(1);
        let teleport_attempted = self.rng.bounded(skill) >= u64::from(failure_threshold);
        let candidates = teleport_candidates
            .into_iter()
            .filter(|position| {
                self.is_walkable(*position)
                    && self
                        .entities
                        .iter()
                        .all(|entity| entity.position != *position)
            })
            .collect::<Vec<_>>();
        let teleported = teleport_attempted && !candidates.is_empty() && !self.player_is_dead();
        if teleported {
            let destination_index = usize::try_from(
                self.rng
                    .bounded(u64::try_from(candidates.len()).unwrap_or(u64::MAX)),
            )
            .expect("panic teleport candidate index must fit usize");
            self.resolve_player_teleport_effect(
                ability,
                candidates[destination_index],
                events,
                changed,
            );
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(target_entity_id.to_owned()),
                target_kind_id: Some(target_kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::MeleeThenTeleport {
                    effect_index: 0,
                    target_entity_id: target_entity_id.to_owned(),
                    target_kind_id,
                    player_from,
                    player_to: self.player.position,
                    teleport_attempted,
                    teleported,
                }],
            },
            trace: None,
        });
        Ok(())
    }

    fn resolve_player_polymorph_target_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let (trace, target_index) = self.trace_projectile_path(path);
        let Some(target_index) = target_index else {
            events.push(DomainEvent::AbilityLanded {
                ability_id: ability.id.clone(),
                trace: trace.clone(),
            });
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability.id.clone(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: None,
                    target_kind_id: None,
                    effects: vec![AbilityEffectResolutionDto::Skipped {
                        effect_index: 0,
                        reason: AbilityEffectSkipReasonDto::NoTarget,
                    }],
                },
                trace: Some(trace),
            });
            return;
        };
        let target_entity_id = self.entities[target_index].id.clone();
        let target_kind_id = self.entities[target_index].kind_id.clone();
        let resolution = self.resolve_actor_polymorph_target(
            target_index,
            u32::from(self.progress.level),
            0,
            events,
            changed,
        );
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(target_entity_id),
                target_kind_id: Some(target_kind_id),
                effects: vec![resolution],
            },
            trace: Some(trace),
        });
    }

    pub(super) fn resolve_player_polymorph_self_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        const ATTRIBUTES: [AttributeKind; 6] = [
            AttributeKind::Strength,
            AttributeKind::Intelligence,
            AttributeKind::Wisdom,
            AttributeKind::Dexterity,
            AttributeKind::Constitution,
            AttributeKind::Charisma,
        ];

        let active_before = self.progress.active_mutation_ids.clone();
        let hp_before = self.player.hp;
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let mut power = i32::from(self.progress.level);

        if power > i32::try_from(self.rng.bounded(30)).expect("polymorph roll must fit i32")
            && self.rng.bounded(6) == 0
        {
            power -= 20;
            for attribute in ATTRIBUTES {
                let amount = u8::try_from(self.rng.bounded(6) + 7)
                    .expect("polymorph attribute drain must fit u8");
                self.progress
                    .permanently_drain_attribute(attribute, amount, &mut self.rng);
            }
            if self.rng.bounded(6) == 0 {
                let dice = u16::try_from(self.rng.bounded(10) + 1)
                    .expect("polymorph life-loss dice must fit u16");
                self.player.hp = self
                    .player
                    .hp
                    .saturating_sub(self.roll_damage(dice, self.progress.level.max(1)));
                power -= 10;
            }
        }

        if power > i32::try_from(self.rng.bounded(20)).expect("polymorph roll must fit i32")
            && self.rng.bounded(4) == 0
        {
            power -= 10;
            let base_max_hp = self
                .progress
                .hp_progression
                .first()
                .copied()
                .unwrap_or(self.player.max_hp);
            self.progress.hp_progression =
                CharacterProgress::roll_hp_progression(base_max_hp, &mut self.rng);
        }

        while power > i32::try_from(self.rng.bounded(15)).expect("polymorph roll must fit i32")
            && self.rng.bounded(3) == 0
        {
            power -= 7;
            if self.gain_random_mutation_without_refresh(events).is_none() {
                break;
            }
        }

        if power > i32::try_from(self.rng.bounded(5)).expect("polymorph roll must fit i32") {
            power -= 5;
            self.resolve_polymorph_wounds(&ability.id);
        }

        let mut swapped_attributes = Vec::new();
        while power > 0 {
            let left_index = usize::try_from(self.rng.bounded(6))
                .expect("polymorph attribute index must fit usize");
            let mut right_index = usize::try_from(self.rng.bounded(5))
                .expect("polymorph attribute index must fit usize");
            if right_index >= left_index {
                right_index += 1;
            }
            let left = ATTRIBUTES[left_index];
            let right = ATTRIBUTES[right_index];
            let left_current = self.progress.attributes.value(left);
            let right_current = self.progress.attributes.value(right);
            let left_maximum = self.progress.maximum_attributes.value(left);
            let right_maximum = self.progress.maximum_attributes.value(right);
            let left_cap = self.progress.attribute_potentials.value(left);
            let right_cap = self.progress.attribute_potentials.value(right);
            let next_left_maximum = right_maximum.min(left_cap);
            let next_right_maximum = left_maximum.min(right_cap);
            set_attribute_value(
                &mut self.progress.maximum_attributes,
                left,
                next_left_maximum,
            );
            set_attribute_value(
                &mut self.progress.maximum_attributes,
                right,
                next_right_maximum,
            );
            set_attribute_value(
                &mut self.progress.attributes,
                left,
                right_current.min(next_left_maximum),
            );
            set_attribute_value(
                &mut self.progress.attributes,
                right,
                left_current.min(next_right_maximum),
            );
            swapped_attributes.push(attribute_kind_dto(left));
            swapped_attributes.push(attribute_kind_dto(right));
            power -= 1;
        }

        self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        let active_after = &self.progress.active_mutation_ids;
        let gained_mutation_ids = active_after
            .difference(&active_before)
            .cloned()
            .collect::<Vec<_>>();
        let lost_mutation_ids = active_before
            .difference(active_after)
            .cloned()
            .collect::<Vec<_>>();
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::PolymorphSelf {
                    effect_index: 0,
                    gained_mutation_ids,
                    lost_mutation_ids,
                    swapped_attributes,
                    hp_before,
                    hp_after: self.player.hp,
                }],
            },
            trace: None,
        });
    }

    fn resolve_player_swap_position_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let (trace, target_index) = self.trace_projectile_path(path);
        let player_from = self.player.position;
        let mut target_entity_id = None;
        let mut target_from = None;
        if let Some(index) = target_index {
            let position = self.entities[index].position;
            target_entity_id = Some(self.entities[index].id.clone());
            target_from = Some(position);
            self.entities[index].position = player_from;
            self.player.position = position;
            changed.insert(player_from);
            changed.insert(position);
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: target_entity_id.clone(),
                target_kind_id: target_entity_id.as_ref().and_then(|id| {
                    self.entities
                        .iter()
                        .find(|entity| &entity.id == id)
                        .map(|entity| entity.kind_id.clone())
                }),
                effects: vec![AbilityEffectResolutionDto::SwapPosition {
                    effect_index: 0,
                    target_entity_id,
                    player_from,
                    target_from,
                    swapped: target_from.is_some(),
                }],
            },
            trace: Some(trace),
        });
    }

    fn resolve_player_recall_effect(
        &mut self,
        ability: &AbilityDefinition,
        action: RecallUseAction,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::Recall {
            delay_dice,
            delay_sides,
            delay_bonus,
        } = ability.effect
        else {
            unreachable!("recall executor requires a recall effect");
        };
        let recall = self
            .recall
            .as_ref()
            .expect("planned recall must retain its destination")
            .clone();
        let (action_dto, delay) = match action {
            RecallUseAction::Start => {
                let rolled = self
                    .roll_damage(delay_dice, delay_sides)
                    .saturating_add(i32::from(delay_bonus));
                let rolled = u16::try_from(rolled.max(1)).expect("validated recall delay fits u16");
                let delay = self.debug_recall_delay_turns.unwrap_or(rolled).max(1);
                self.start_recall(delay);
                (AbilityRecallActionDto::Start, Some(delay))
            }
            RecallUseAction::Cancel => {
                self.cancel_recall();
                (AbilityRecallActionDto::Cancel, None)
            }
        };
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::Recall {
                    effect_index: 0,
                    action: action_dto,
                    delay,
                    dungeon_id: recall.dungeon_id,
                    floor_id: recall.floor_id,
                }],
            },
            trace: None,
        });
    }

    fn resolve_player_level_teleport_effect(
        &mut self,
        ability: &AbilityDefinition,
        upward_targets: Vec<FloorTransitionTarget>,
        downward_targets: Vec<FloorTransitionTarget>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Result<(), CoreError> {
        let prefer_upward = self.rng.bounded(2) == 0;
        let targets = if prefer_upward {
            if upward_targets.is_empty() {
                downward_targets
            } else {
                upward_targets
            }
        } else if downward_targets.is_empty() {
            upward_targets
        } else {
            downward_targets
        };
        let target_index = if targets.len() == 1 {
            0
        } else {
            usize::try_from(self.rng.bounded(targets.len() as u64))
                .expect("bounded floor target index must fit usize")
        };
        let target = targets[target_index].clone();
        let from_floor_id = self.current_floor_id.clone();
        let transition = self
            .transition_floor(
                target.floor_id,
                target.arrival_connection_id,
                target.departure_connection_id,
                false,
            )?
            .expect("planned ability floor teleport must remain available");
        let to_floor_id = transition.to_floor_id.clone();
        self.record_floor_transition(transition, events, changed);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::TeleportLevel {
                    effect_index: 0,
                    from_floor_id,
                    to_floor_id,
                }],
            },
            trace: None,
        });
        Ok(())
    }

    fn resolve_player_teleport_away_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::TeleportAway {
            minimum_distance,
            power,
        } = ability.effect
        else {
            unreachable!("teleport-away executor requires a teleport-away effect");
        };
        let (trace, _) = self.trace_projectile_path_with_actor_policy(path, false);
        let target_ids = self.beam_damage_targets(&trace.traversed);
        let mut resolutions = Vec::new();
        for target_entity_id in target_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == target_entity_id && entity.hp > 0)
            else {
                continue;
            };
            let from = self.entities[index].position;
            let definition = self
                .actor_runtime_definition(&self.entities[index])
                .expect("teleport-away target definition must remain available");
            let has_tag = |tag: &str| definition.tags.iter().any(|candidate| candidate == tag);
            let target_level = definition.level;
            let resistant = has_tag("resist-teleport");
            let always_resisted =
                has_tag("guardian") || (resistant && (has_tag("unique") || has_tag("resist-all")));
            let resistance_roll = if resistant && !always_resisted {
                Some(
                    u8::try_from(self.rng.bounded(100) + 1)
                        .expect("teleport resistance roll must fit u8"),
                )
            } else {
                None
            };
            let resisted = always_resisted
                || resistance_roll.is_some_and(|roll| target_level > u32::from(roll));
            let mut to = None;
            if !resisted {
                let distance = u32::from(power.max(u16::from(minimum_distance)));
                let mut minimum = distance / 2;
                let mut maximum = distance.max(1);
                for _ in 0..8 {
                    let candidates =
                        (0..self.height)
                            .flat_map(|y| {
                                (0..self.width).map(move |x| Position {
                                    x: i32::from(x),
                                    y: i32::from(y),
                                })
                            })
                            .filter(|position| {
                                let distance = rfb_distance(from, *position);
                                distance >= minimum
                                    && distance <= maximum
                                    && *position != self.player.position
                                    && self.actor_can_enter_position(index, *position)
                                    && !self.entities.iter().enumerate().any(
                                        |(other_index, entity)| {
                                            other_index != index
                                                && entity.hp > 0
                                                && entity.position == *position
                                        },
                                    )
                            })
                            .collect::<Vec<_>>();
                    if !candidates.is_empty() {
                        let candidate_index = if candidates.len() == 1 {
                            0
                        } else {
                            usize::try_from(self.rng.bounded(candidates.len() as u64))
                                .expect("bounded teleport destination index must fit usize")
                        };
                        to = Some(candidates[candidate_index]);
                        break;
                    }
                    minimum /= 2;
                    maximum = maximum.saturating_mul(2);
                }
            }
            if let Some(destination) = to {
                self.entities[index].position = destination;
                changed.insert(from);
                changed.insert(destination);
            }
            resolutions.push(AbilityEffectResolutionDto::TeleportAway {
                effect_index: 0,
                target_entity_id,
                power,
                resistance_roll,
                resisted,
                from,
                to,
            });
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: resolutions,
            },
            trace: Some(trace),
        });
    }

    fn resolve_player_recharge_effect(
        &mut self,
        ability: &AbilityDefinition,
        item_id: &str,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::RechargeFromPlayer { power } = ability.effect else {
            unreachable!("recharge executor requires a player recharge effect");
        };
        let resource_id = Self::player_ability_parameters(ability).resource_id.clone();
        let available = self
            .resources
            .get(&resource_id)
            .expect("validated recharge resource must remain available")
            .current;
        let missing = self
            .items
            .iter()
            .find(|item| item.id == item_id)
            .and_then(|item| item.charges)
            .map(|charges| charges.maximum.saturating_sub(charges.current))
            .expect("preflighted recharge target must retain charge capacity");
        let attempted = u32::from(power).min(available).min(missing);
        self.resources
            .get_mut(&resource_id)
            .expect("validated recharge resource must remain available")
            .current -= attempted;
        let outcome =
            self.recharge_inventory_item_from_player(item_id, attempted, u32::from(power));
        events.push(device_recharge_resolved_event(
            outcome,
            ability.id.clone(),
            false,
            false,
        ));
    }

    fn resolve_player_clairvoyance_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::Clairvoyance {
            telepathy_duration_ticks,
            telepathy_duration_dice,
            telepathy_duration_sides,
        } = ability.effect
        else {
            unreachable!("clairvoyance executor requires a clairvoyance effect");
        };

        self.add_virtue(VirtueKindDto::Knowledge, 1);
        self.add_virtue(VirtueKindDto::Enlightenment, 1);

        let mut mapped_positions = Vec::with_capacity(self.terrain.len());
        for y in 0..self.height {
            for x in 0..self.width {
                let position = Position {
                    x: i32::from(x),
                    y: i32::from(y),
                };
                let index = self.index(position).expect("floor position must be valid");
                if !self.explored[index] || !self.glow[index] {
                    changed.insert(position);
                }
                self.explored[index] = true;
                self.glow[index] = true;
                mapped_positions.push(position);
            }
        }
        events.push(DomainEvent::AbilityDetected {
            ability_id: ability.id.clone(),
            resolution: AbilityDetectResolutionDto {
                subject: AbilityDetectSubjectDto::Terrain,
                category: "map".to_owned(),
                radius: u8::MAX,
                persistent: true,
                through_walls: true,
                detected_positions: mapped_positions,
                detected_entity_ids: Vec::new(),
            },
        });

        let mut ground_items = self
            .items
            .iter()
            .filter_map(|item| match item.location {
                ItemLocation::Ground(position) => {
                    Some((position.y, position.x, item.id.clone(), position))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        ground_items.sort_by(|left, right| {
            (left.0, left.1, left.2.as_str()).cmp(&(right.0, right.1, right.2.as_str()))
        });
        let item_ids = ground_items
            .iter()
            .map(|(_, _, item_id, _)| item_id.clone())
            .collect::<Vec<_>>();
        let item_positions = ground_items
            .into_iter()
            .map(|(_, _, _, position)| position)
            .collect::<Vec<_>>();
        self.mark_item_instances_discovered(&item_ids);
        changed.extend(item_positions.iter().copied());
        events.push(DomainEvent::AbilityDetected {
            ability_id: ability.id.clone(),
            resolution: AbilityDetectResolutionDto {
                subject: AbilityDetectSubjectDto::Item,
                category: "item".to_owned(),
                radius: u8::MAX,
                persistent: false,
                through_walls: true,
                detected_positions: item_positions,
                detected_entity_ids: item_ids,
            },
        });

        let telepathy_resolution = if self.player_has_permanent_telepathy() {
            AbilityEffectResolutionDto::Skipped {
                effect_index: 0,
                reason: AbilityEffectSkipReasonDto::Ineligible,
            }
        } else {
            apply_ability_status_effect(
                &mut self.player,
                &ability.id,
                0,
                STATUS_TELEPATHY,
                1,
                u32::from(telepathy_duration_ticks),
                u16::from(telepathy_duration_dice),
                u32::from(telepathy_duration_sides),
                AbilityStatusStackingDefinition::KeepStrongest,
                None,
                None,
                &BTreeMap::new(),
                &BTreeSet::new(),
                &StatModifiers::default(),
                &EquipmentBonuses::default(),
                &BTreeSet::new(),
                None,
                false,
                100,
                None,
                None,
                &mut self.rng,
            )
        };
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![telepathy_resolution],
            },
            trace: None,
        });
    }

    fn resolve_player_resist_elements_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::ResistElements {
            duration_dice,
            duration_sides,
            duration_bonus,
        } = ability.effect
        else {
            unreachable!("resist elements executor requires a resist elements effect");
        };
        let rolled_duration = (0..duration_dice).fold(duration_bonus, |total, _| {
            total.saturating_add(
                u32::try_from(self.rng.bounded(u64::from(duration_sides)) + 1)
                    .expect("validated resistance duration roll must fit u32"),
            )
        });
        let mut remaining = self.progress.level / 10;
        let candidates = [
            (5_u16, ActorDamageType::Acid, "rfb.status.resist-acid"),
            (
                4,
                ActorDamageType::Electricity,
                "rfb.status.resist-electricity",
            ),
            (3, ActorDamageType::Fire, "rfb.status.resist-fire"),
            (2, ActorDamageType::Cold, "rfb.status.resist-cold"),
            (1, ActorDamageType::Poison, "rfb.status.resist-poison"),
        ];
        let empty_brands = BTreeSet::new();
        let empty_immunities = BTreeSet::new();
        let mut resolutions = Vec::new();
        for (denominator, damage_type, status_kind_id) in candidates {
            if remaining == 0 || self.rng.bounded(u64::from(denominator)) >= u64::from(remaining) {
                continue;
            }
            remaining -= 1;
            let mut resistances = BTreeMap::new();
            resistances.insert(damage_type, ActorResistanceLevel::Resistant);
            let effect_index = u8::try_from(resolutions.len())
                .expect("elemental resistance effect count must fit u8");
            resolutions.push(apply_ability_status_effect(
                &mut self.player,
                &ability.id,
                effect_index,
                status_kind_id,
                1,
                rolled_duration,
                0,
                0,
                AbilityStatusStackingDefinition::Replace,
                None,
                None,
                &resistances,
                &empty_brands,
                &StatModifiers::default(),
                &EquipmentBonuses::default(),
                &empty_immunities,
                None,
                false,
                100,
                None,
                None,
                &mut self.rng,
            ));
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: resolutions,
            },
            trace: None,
        });
    }

    fn resolve_player_aggravate_monsters_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let (awakened, hastened, _) = self.aggravate_monsters(None, &ability.id, changed);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::AggravateMonsters {
                    effect_index: 0,
                    awakened,
                    hastened,
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_summon_effect(
        &mut self,
        ability: &AbilityDefinition,
        positions: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::Summon {
            actor_kind_id,
            count,
            duration_turns,
            hostile,
            ..
        } = &ability.effect
        else {
            unreachable!("summon executor requires a fixed summon effect");
        };
        debug_assert!(positions.len() <= usize::from(*count));
        let definition = self
            .content
            .actor(actor_kind_id)
            .expect("validated summon actor must remain available")
            .clone();
        let mut entity_ids = Vec::with_capacity(positions.len());
        for (ordinal, position) in positions.iter().copied().enumerate() {
            let id = self.summon_entity_id(&ability.id, ordinal);
            let mut entity = spawn_actor_from_definition(
                &mut self.rng,
                &definition,
                &id,
                position,
                INITIAL_MONSTER_ENERGY_NEED,
                true,
            );
            self.maybe_initialize_chameleon_form(&mut entity);
            if !hostile {
                entity.summon = Some(SummonIdentity {
                    owner_id: self.player.id.clone(),
                    source_ability_id: ability.id.clone(),
                    remaining_turns: *duration_turns,
                });
            }
            changed.insert(position);
            entity_ids.push(id);
            self.entities.push(entity);
        }
        events.push(DomainEvent::AbilitySummoned {
            ability_id: ability.id.clone(),
            resolution: AbilitySummonResolutionDto {
                owner_id: self.player.id.clone(),
                actor_kind_id: actor_kind_id.clone(),
                entity_ids,
                positions,
                duration_turns: *duration_turns,
                hostile: *hostile,
                group: false,
                summoned_kind_ids: Vec::new(),
            },
        });
    }

    pub(super) fn resolve_player_category_summon_effect(
        &mut self,
        ability: &AbilityDefinition,
        friendly_candidate_kind_ids: Vec<String>,
        hostile_candidate_kind_ids: Vec<String>,
        positions: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::SummonCategory {
            category,
            upgraded_category,
            upgrade_at_level,
            count_dice,
            count_sides,
            count_bonus,
            maximum_count,
            hostile_chance_percent,
            friendly_group_chance_percent,
            hostile_group_chance_percent,
            group_count_dice,
            group_count_sides,
            group_count_bonus,
            duration_turns,
            ..
        } = &ability.effect
        else {
            unreachable!("category summon executor requires a category summon effect");
        };
        let hostile = match *hostile_chance_percent {
            0 => false,
            100 => true,
            chance => self.rng.bounded(100) < u64::from(chance),
        };
        let group_chance = if hostile {
            *hostile_group_chance_percent
        } else {
            *friendly_group_chance_percent
        };
        let candidates = if hostile {
            hostile_candidate_kind_ids
        } else {
            friendly_candidate_kind_ids
        };
        let selected_category = upgraded_category
            .as_deref()
            .zip(*upgrade_at_level)
            .filter(|(_, level)| self.progress.level >= *level)
            .map_or(category.as_str(), |(category, _)| category);
        let owner_id = self.player.id.clone();
        let resolution = self.resolve_category_summon(
            CategorySummonSpec {
                source_id: &ability.id,
                owner_id: &owner_id,
                category: selected_category,
                count_dice: *count_dice,
                count_sides: *count_sides,
                count_bonus: *count_bonus,
                maximum_count: *maximum_count,
                hostile,
                group_chance_percent: group_chance,
                group_count_dice: *group_count_dice,
                group_count_sides: *group_count_sides,
                group_count_bonus: *group_count_bonus,
                duration_turns: *duration_turns,
            },
            candidates,
            positions,
            changed,
        );
        if ability.id == DEATH_RAISE_DEAD_ABILITY_ID && !resolution.entity_ids.is_empty() {
            self.add_virtue(VirtueKindDto::Unlife, 1);
        }
        events.push(DomainEvent::AbilitySummoned {
            ability_id: ability.id.clone(),
            resolution,
        });
    }

    pub(super) fn resolve_player_genocide_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Option<Vec<Position>>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) {
        let AbilityEffectDefinition::Genocide {
            scope,
            power,
            radius,
            target_category,
            fatigue,
        } = &ability.effect
        else {
            unreachable!("genocide executor requires a genocide effect");
        };
        let (trace, target_entity_id, target_kind_id, glyph) =
            if *scope == AbilityGenocideScopeDefinition::Nearby {
                (None, None, None, None)
            } else {
                let (trace, target_index) =
                    self.trace_projectile_path(path.expect("targeted genocide must retain a path"));
                let Some(target_index) = target_index else {
                    events.push(DomainEvent::AbilityLanded {
                        ability_id: ability.id.clone(),
                        trace: trace.clone(),
                    });
                    events.push(DomainEvent::AbilityEffectsResolved {
                        ability_id: ability.id.clone(),
                        resolution: AbilityEffectsResolutionDto {
                            target_entity_id: None,
                            target_kind_id: None,
                            effects: vec![AbilityEffectResolutionDto::Skipped {
                                effect_index: 0,
                                reason: AbilityEffectSkipReasonDto::NoTarget,
                            }],
                        },
                        trace: Some(trace),
                    });
                    return;
                };
                let target_entity_id = self.entities[target_index].id.clone();
                let target_kind_id = self.entities[target_index].kind_id.clone();
                let glyph = self
                    .content
                    .actor(&target_kind_id)
                    .map(|definition| definition.glyph.clone());
                (
                    Some(trace),
                    Some(target_entity_id),
                    Some(target_kind_id),
                    glyph,
                )
            };
        let mut candidate_ids = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && target_category.as_ref().is_none_or(|category| {
                        self.content
                            .actor(&entity.kind_id)
                            .is_some_and(|definition| actor_matches_category(definition, category))
                    })
                    && match scope {
                        AbilityGenocideScopeDefinition::Single => {
                            target_entity_id.as_deref() == Some(entity.id.as_str())
                        }
                        AbilityGenocideScopeDefinition::Glyph => self
                            .content
                            .actor(&entity.kind_id)
                            .zip(glyph.as_ref())
                            .is_some_and(|(definition, glyph)| &definition.glyph == glyph),
                        AbilityGenocideScopeDefinition::Nearby => {
                            chebyshev_distance(self.player.position, entity.position)
                                <= u32::from(*radius)
                        }
                    }
            })
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        candidate_ids.sort();
        let resolution = self.resolve_genocide_candidates(
            candidate_ids,
            *scope,
            *power,
            *fatigue,
            changed,
            removed_entities,
        );
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id,
                target_kind_id,
                effects: vec![AbilityEffectResolutionDto::Genocide {
                    effect_index: 0,
                    scope: ability_genocide_scope_dto(*scope),
                    power: *power,
                    radius: *radius,
                    glyph: matches!(scope, AbilityGenocideScopeDefinition::Glyph)
                        .then_some(glyph)
                        .flatten(),
                    removed_entity_ids: resolution.removed_entity_ids,
                    resisted_entity_ids: resolution.resisted_entity_ids,
                    fatigue_damage: resolution.fatigue_damage,
                }],
            },
            trace,
        });
    }

    pub(super) fn animate_dead_candidates(
        &self,
        origin: Position,
        actor_kind_id: &str,
        corpse_item_kind_id: &str,
        radius: u8,
        count: u8,
    ) -> Vec<(String, Position)> {
        let mut corpses = self
            .items
            .iter()
            .filter_map(|item| match item.location {
                ItemLocation::Ground(position)
                    if item.kind_id == corpse_item_kind_id
                        && chebyshev_distance(origin, position) <= u32::from(radius)
                        && self.actor_kind_can_enter_position(actor_kind_id, position) =>
                {
                    Some((
                        chebyshev_distance(origin, position),
                        position.y,
                        position.x,
                        item.id.clone(),
                        position,
                    ))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        corpses.sort();
        corpses.truncate(usize::from(count));
        corpses
            .into_iter()
            .map(|(_, _, _, item_id, position)| (item_id, position))
            .collect()
    }

    pub(super) fn resolve_player_animate_dead_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::AnimateDead {
            actor_kind_id,
            corpse_item_kind_id,
            radius,
            count,
            failure_chance_percent,
        } = &ability.effect
        else {
            unreachable!("animate dead executor requires an animate dead effect");
        };
        let origin = self.player.position;
        let definition = self
            .content
            .actor(actor_kind_id)
            .expect("validated animated actor must remain available")
            .clone();
        let corpses = self.animate_dead_candidates(
            origin,
            actor_kind_id,
            corpse_item_kind_id,
            *radius,
            *count,
        );
        let consumed_corpse_item_ids = corpses
            .iter()
            .map(|corpse| corpse.0.clone())
            .collect::<Vec<_>>();
        self.items
            .retain(|item| !consumed_corpse_item_ids.contains(&item.id));
        for item_id in &consumed_corpse_item_ids {
            self.item_property_knowledge.remove(item_id);
        }
        let mut entity_ids = Vec::with_capacity(corpses.len());
        let mut positions = Vec::with_capacity(corpses.len());
        for (ordinal, (_, position)) in corpses.into_iter().enumerate() {
            changed.insert(position);
            if *failure_chance_percent > 0
                && self.rng.bounded(100) < u64::from(*failure_chance_percent)
            {
                continue;
            }
            let id = self.summon_entity_id(&ability.id, ordinal);
            let mut entity = spawn_actor_from_definition(
                &mut self.rng,
                &definition,
                &id,
                position,
                INITIAL_MONSTER_ENERGY_NEED,
                true,
            );
            entity.controller_id = Some(self.player.id.clone());
            self.entities.push(entity);
            entity_ids.push(id);
            positions.push(position);
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::AnimateDead {
                    effect_index: 0,
                    actor_kind_id: actor_kind_id.clone(),
                    consumed_corpse_item_ids,
                    entity_ids,
                    positions,
                }],
            },
            trace: None,
        });
        Ok(())
    }

    pub(super) fn resolve_player_probe_monsters_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let mut monsters = Vec::new();
        if !self.player_has_status_kind(STATUS_HALLUCINATION) {
            let indices = self
                .entities
                .iter()
                .enumerate()
                .filter(|(_, entity)| {
                    entity.hp > 0
                        && self.entity_is_visible_to_player(entity)
                        && !self.entity_is_fuzzy_to_player(entity)
                        && has_line_of_effect(self, self.player.position, entity.position)
                })
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            for index in indices {
                if self.entities[index].appearance_kind_id.take().is_some() {
                    changed.insert(self.entities[index].position);
                }
                let entity = self.entities[index].clone();
                let definition = self
                    .actor_runtime_definition(&entity)
                    .expect("probed actor definition must remain available")
                    .clone();
                self.probed_actor_kind_ids.insert(definition.id.clone());
                let stats = self.actor_derived_stats(&entity, &definition, false);
                let good = definition.tags.iter().any(|tag| tag == "good");
                let evil = definition.tags.iter().any(|tag| tag == "evil");
                let alignment = match (good, evil) {
                    (true, true) => MonsterAlignmentDto::GoodAndEvil,
                    (true, false) => MonsterAlignmentDto::Good,
                    (false, true) => MonsterAlignmentDto::Evil,
                    (false, false) => MonsterAlignmentDto::Neutral,
                };
                let faction = if self.actor_is_player_aligned(&entity) {
                    rfb_protocol::EntityFactionDto::Player
                } else if self.actor_is_friendly(&entity) {
                    rfb_protocol::EntityFactionDto::Friendly
                } else {
                    rfb_protocol::EntityFactionDto::Hostile
                };
                let mut status_immunities = definition
                    .status_immunities
                    .iter()
                    .cloned()
                    .collect::<BTreeSet<_>>();
                for status in &entity.statuses {
                    status_immunities.extend(status.granted_status_immunities.iter().cloned());
                }
                let mut ability_ids = definition
                    .monster_casting
                    .as_ref()
                    .map(|casting| {
                        casting
                            .abilities
                            .iter()
                            .map(|candidate| candidate.ability_id.clone())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                ability_ids.sort();
                ability_ids.dedup();
                monsters.push(ProbedMonsterDto {
                    entity_id: entity.id,
                    kind_id: definition.id.clone(),
                    glyph: definition.glyph.clone(),
                    position: entity.position,
                    hp: entity.hp,
                    max_hp: entity.max_hp,
                    speed: derived_speed(&stats.speed),
                    armor_class: stats.armor_class.value,
                    alignment,
                    faction,
                    resistances: entity.resistances.to_dtos(),
                    status_immunities: status_immunities.into_iter().collect(),
                    melee_routine: actor_melee_routine_dto(&definition),
                    ability_ids,
                });
            }
        }
        events.push(DomainEvent::AbilityMonstersProbed {
            ability_id: ability.id.clone(),
            resolution: AbilityMonsterProbeResolutionDto { monsters },
        });
    }

    pub(super) fn resolve_player_detection_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::Detect {
            subject,
            category,
            radius,
            persistent,
            through_walls,
        } = &ability.effect
        else {
            unreachable!("detection executor requires a detection effect");
        };
        let (detected_positions, detected_entity_ids) = match subject {
            AbilityDetectSubjectDefinition::Terrain => (
                self.detect_terrain_positions(category, *radius, *persistent, *through_walls),
                Vec::new(),
            ),
            AbilityDetectSubjectDefinition::Actor => self.detect_actor_positions(category, *radius),
            AbilityDetectSubjectDefinition::Item => {
                let detected = self.detect_item_positions(category, *radius, *through_walls);
                self.mark_item_instances_discovered(&detected.1);
                detected
            }
            AbilityDetectSubjectDefinition::Gold => {
                let detected = self.detect_gold_positions(*radius, *through_walls);
                self.mark_gold_piles_discovered(&detected.1);
                detected
            }
            AbilityDetectSubjectDefinition::Curse => {
                let mut item_ids = self
                    .items
                    .iter()
                    .filter(|item| {
                        item.curse.is_some()
                            && matches!(
                                item.location,
                                ItemLocation::Inventory | ItemLocation::Equipped { .. }
                            )
                    })
                    .map(|item| item.id.clone())
                    .collect::<Vec<_>>();
                item_ids.sort();
                for item_id in &item_ids {
                    self.identify_item_instance(item_id, ItemIdentificationRequest::new(false));
                }
                (
                    (!item_ids.is_empty())
                        .then_some(self.player.position)
                        .into_iter()
                        .collect(),
                    item_ids,
                )
            }
        };
        if *persistent
            || matches!(
                subject,
                AbilityDetectSubjectDefinition::Item
                    | AbilityDetectSubjectDefinition::Gold
                    | AbilityDetectSubjectDefinition::Curse
            )
        {
            changed.extend(detected_positions.iter().copied());
        }
        events.push(DomainEvent::AbilityDetected {
            ability_id: ability.id.clone(),
            resolution: AbilityDetectResolutionDto {
                subject: ability_detect_subject_dto(*subject),
                category: category.clone(),
                radius: *radius,
                persistent: *persistent,
                through_walls: *through_walls,
                detected_positions,
                detected_entity_ids,
            },
        });
    }

    pub(super) fn resolve_terrain_transform_effect(
        &mut self,
        ability: &AbilityDefinition,
        center: Position,
        positions: Vec<Position>,
        source: TerrainChangeSource,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::TransformTerrain {
            source_terrain_ids,
            target_terrain_id,
            radius,
        } = &ability.effect
        else {
            unreachable!("terrain executor requires a terrain transform effect");
        };
        for position in &positions {
            let index = self
                .index(*position)
                .expect("planned terrain transformation must remain in bounds");
            debug_assert!(source_terrain_ids.contains(&self.terrain[index]));
            self.replace_terrain_from_source(*position, target_terrain_id, source, events, changed);
        }
        events.push(DomainEvent::AbilityTerrainTransformed {
            ability_id: ability.id.clone(),
            resolution: AbilityTerrainTransformResolutionDto {
                center,
                radius: *radius,
                source_terrain_ids: source_terrain_ids.clone(),
                target_terrain_id: target_terrain_id.clone(),
                transformed_positions: positions,
            },
        });
    }
}

impl Game {
    pub(super) fn ability_target_plan(
        &self,
        ability: &AbilityDefinition,
        target: &TargetSelection,
    ) -> Option<AbilityTargetPlan> {
        match ability.effect {
            // These forms are monster-casting-only. The player cast path
            // never produces a target plan for them.
            AbilityEffectDefinition::BlinkTarget { .. }
            | AbilityEffectDefinition::TeleportSelf { .. }
            | AbilityEffectDefinition::TeleportTarget
            | AbilityEffectDefinition::BreathDamage { .. }
            | AbilityEffectDefinition::CurseDamage { .. }
            | AbilityEffectDefinition::BirdDrop
            | AbilityEffectDefinition::DrainResource { .. }
            | AbilityEffectDefinition::Amnesia
            | AbilityEffectDefinition::DarkenRoom
            | AbilityEffectDefinition::JumpDamage { .. } => None,
            AbilityEffectDefinition::Rodeo => {
                let TargetSelection::Direction { direction } = target else {
                    return None;
                };
                if !ability
                    .target
                    .modes
                    .contains(&AbilityTargetModeDefinition::Direction)
                    || self.riding_actor_id.is_some()
                {
                    return None;
                }
                let position = self.position_in_direction(*direction);
                let target_entity_id = self
                    .entities
                    .iter()
                    .find(|entity| {
                        entity.hp > 0
                            && entity.position == position
                            && self
                                .actor_runtime_definition(entity)
                                .is_some_and(|definition| definition.rideable)
                    })?
                    .id
                    .clone();
                Some(AbilityTargetPlan::Rodeo {
                    direction: *direction,
                    target_entity_id,
                })
            }
            AbilityEffectDefinition::TeleportLevel => {
                if !matches!(target, TargetSelection::SelfTarget)
                    || !ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget)
                {
                    return None;
                }
                let (upward_targets, downward_targets) = self.teleport_level_targets();
                (!upward_targets.is_empty() || !downward_targets.is_empty()).then_some(
                    AbilityTargetPlan::TeleportLevel {
                        upward_targets,
                        downward_targets,
                    },
                )
            }
            AbilityEffectDefinition::TeleportAway { power, .. } => (power > 0)
                .then(|| self.beam_ability_path(ability, target))
                .flatten()
                .map(|path| AbilityTargetPlan::Projectile {
                    path,
                    stop_at_actor: false,
                }),
            AbilityEffectDefinition::Teleport => {
                let TargetSelection::Position { position } = target else {
                    return None;
                };
                self.teleport_destination(ability, *position)
                    .map(|destination| AbilityTargetPlan::Teleport { destination })
            }
            AbilityEffectDefinition::BlinkSelf { radius } => {
                if !matches!(target, TargetSelection::SelfTarget)
                    || !ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget)
                {
                    return None;
                }
                let candidates = self.random_teleport_candidates(u16::from(radius));
                (!candidates.is_empty()).then_some(AbilityTargetPlan::RandomTeleport { candidates })
            }
            AbilityEffectDefinition::DimensionDoor { range } => {
                let TargetSelection::Position { position } = target else {
                    return None;
                };
                if ability.target.modes.as_slice() != [AbilityTargetModeDefinition::Position] {
                    return None;
                }
                let destination_valid = self.index(*position).is_some()
                    && rfb_distance(self.player.position, *position) <= u32::from(range)
                    && self.is_walkable(*position)
                    && !self
                        .entities
                        .iter()
                        .any(|entity| entity.hp > 0 && entity.position == *position);
                Some(AbilityTargetPlan::DimensionDoor {
                    requested: *position,
                    destination_valid,
                    fallback_candidates: self.random_teleport_candidates(
                        self.progress.level.saturating_add(2).saturating_mul(2),
                    ),
                })
            }
            AbilityEffectDefinition::TeleportTown => {
                let TargetSelection::Town { town_id } = target else {
                    return None;
                };
                self.teleport_town_target_available(town_id)
                    .then(|| AbilityTargetPlan::Town {
                        town_id: town_id.clone(),
                    })
            }
            AbilityEffectDefinition::FetchItem { .. } => self
                .ability_path(ability, target)
                .map(|path| AbilityTargetPlan::FetchItem { path }),
            AbilityEffectDefinition::ConsumeTerrain { .. } => {
                let TargetSelection::Direction { direction } = target else {
                    return None;
                };
                if !ability
                    .target
                    .modes
                    .contains(&AbilityTargetModeDefinition::Direction)
                {
                    return None;
                }
                let position = self.position_in_direction(*direction);
                let index = self.index(position)?;
                if self
                    .entities
                    .iter()
                    .any(|entity| entity.position == position)
                    || self
                        .floor_connections
                        .iter()
                        .any(|connection| connection.position == position)
                {
                    return None;
                }
                let terrain = self.content.terrain(&self.terrain[index])?;
                if terrain
                    .tags
                    .iter()
                    .any(|tag| matches!(tag.as_str(), "permanent" | "tree" | "glass"))
                {
                    return None;
                }
                let target_terrain_id = terrain
                    .digging
                    .as_ref()
                    .and_then(|digging| digging.result_terrain_id.as_ref())
                    .or(terrain.monster_destroy_to_terrain_id.as_ref())?
                    .clone();
                Some(AbilityTargetPlan::ConsumeTerrain {
                    position,
                    source_terrain_id: terrain.id.clone(),
                    target_terrain_id,
                })
            }
            AbilityEffectDefinition::CreateAmmunition {
                ref source_item_tags,
                ref source_terrain_tags,
                ..
            } => {
                if !source_item_tags.is_empty() {
                    let TargetSelection::Item { item_id } = target else {
                        return None;
                    };
                    return self
                        .items
                        .iter()
                        .find(|item| {
                            item.id == *item_id
                                && (item.location == ItemLocation::Inventory
                                    || item.location == ItemLocation::Ground(self.player.position))
                                && self.can_destroy_item(item).is_ok()
                                && self.content.item(&item.kind_id).is_some_and(|definition| {
                                    source_item_tags.iter().any(|tag| {
                                        if tag == "corpse" {
                                            definition.tags.contains(tag)
                                                && item.origin_actor_kind_id.as_ref().is_some_and(
                                                    |actor_id| {
                                                        self.content.actor(actor_id).is_some_and(
                                                            |actor| {
                                                                actor
                                                                    .tags
                                                                    .iter()
                                                                    .any(|tag| tag == "skeleton")
                                                            },
                                                        )
                                                    },
                                                )
                                        } else {
                                            definition.tags.contains(tag)
                                        }
                                    })
                                })
                        })
                        .map(|_| AbilityTargetPlan::CreateAmmunitionFromItem {
                            item_id: item_id.clone(),
                        });
                }
                let TargetSelection::Direction { direction } = target else {
                    return None;
                };
                let position = self.position_in_direction(*direction);
                let index = self.index(position)?;
                if self
                    .entities
                    .iter()
                    .any(|entity| entity.position == position)
                    || self
                        .floor_connections
                        .iter()
                        .any(|connection| connection.position == position)
                {
                    return None;
                }
                let terrain = self.content.terrain(&self.terrain[index])?;
                if !source_terrain_tags
                    .iter()
                    .any(|tag| terrain.tags.contains(tag))
                {
                    return None;
                }
                let target_terrain_id = terrain
                    .digging
                    .as_ref()
                    .and_then(|digging| digging.result_terrain_id.as_ref())
                    .or(terrain.monster_destroy_to_terrain_id.as_ref())?
                    .clone();
                Some(AbilityTargetPlan::CreateAmmunitionFromTerrain {
                    position,
                    source_terrain_id: terrain.id.clone(),
                    target_terrain_id,
                })
            }
            AbilityEffectDefinition::TransmuteItemToGold { .. } => {
                let TargetSelection::Item { item_id } = target else {
                    return None;
                };
                self.items
                    .iter()
                    .find(|item| {
                        item.id == *item_id
                            && (item.location == ItemLocation::Inventory
                                || item.location == ItemLocation::Ground(self.player.position))
                            && item.captured_actor.is_none()
                            && self.can_destroy_item(item).is_ok()
                    })
                    .map(|_| AbilityTargetPlan::Item {
                        item_id: item_id.clone(),
                    })
            }
            AbilityEffectDefinition::DrainItemMagic { .. } => {
                let TargetSelection::Item { item_id } = target else {
                    return None;
                };
                self.items
                    .iter()
                    .find(|item| {
                        item.id == *item_id
                            && (item.location == ItemLocation::Inventory
                                || item.location == ItemLocation::Ground(self.player.position))
                            && item.charges.is_some_and(|charges| charges.current > 0)
                    })
                    .map(|_| AbilityTargetPlan::Item {
                        item_id: item_id.clone(),
                    })
            }
            AbilityEffectDefinition::RechargeFromPlayer { .. } => {
                let TargetSelection::Item { item_id } = target else {
                    return None;
                };
                self.items
                    .iter()
                    .find(|item| item.id == *item_id && self.item_can_receive_player_recharge(item))
                    .map(|_| AbilityTargetPlan::Item {
                        item_id: item_id.clone(),
                    })
            }
            AbilityEffectDefinition::MeleeThenTeleport { radius, .. } => {
                let TargetSelection::Direction { direction } = target else {
                    return None;
                };
                let position = self.position_in_direction(*direction);
                let target_entity_id = self
                    .entities
                    .iter()
                    .find(|entity| {
                        entity.position == position && !self.actor_is_player_side(entity)
                    })?
                    .id
                    .clone();
                Some(AbilityTargetPlan::MeleeThenTeleport {
                    target_entity_id,
                    teleport_candidates: self.random_teleport_candidates(u16::from(radius)),
                })
            }
            AbilityEffectDefinition::SwapPosition => {
                self.ability_path(ability, target)
                    .map(|path| AbilityTargetPlan::Projectile {
                        path,
                        stop_at_actor: true,
                    })
            }
            AbilityEffectDefinition::SniperShot { mode } => {
                self.ability_path(ability, target)?;
                let profile = self.player_projectile_profile()?;
                profile.ammo_item_id.as_ref()?;
                self.player_projectile_path_for_mode(
                    target,
                    profile.range,
                    super::player_combat::ProjectileMode::Sniper(mode),
                )?;
                Some(AbilityTargetPlan::SniperShot {
                    target: target.clone(),
                })
            }
            AbilityEffectDefinition::ProbeMonsters => matches!(target, TargetSelection::SelfTarget)
                .then_some(AbilityTargetPlan::SelfTarget),
            AbilityEffectDefinition::Recall { .. } => {
                (matches!(target, TargetSelection::SelfTarget)
                    && ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget))
                .then(|| self.recall_use_plan())
                .flatten()
                .map(|action| AbilityTargetPlan::Recall { action })
            }
            AbilityEffectDefinition::MeleeAdjacent
            | AbilityEffectDefinition::ResistElements { .. }
            | AbilityEffectDefinition::ReportMagic
            | AbilityEffectDefinition::AreaDestruction { .. }
            | AbilityEffectDefinition::SuppressMonsterReproduction { .. }
            | AbilityEffectDefinition::PolymorphSelf => {
                (matches!(target, TargetSelection::SelfTarget)
                    && ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget))
                .then_some(AbilityTargetPlan::SelfTarget)
            }
            AbilityEffectDefinition::Earthquake { .. } => {
                let world = self.content.world(&self.world_id)?;
                (matches!(target, TargetSelection::SelfTarget)
                    && ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget)
                    && floor_dungeon_id(world, &self.current_floor_id).is_some())
                .then_some(AbilityTargetPlan::SelfTarget)
            }
            AbilityEffectDefinition::Summon {
                ref actor_kind_id,
                count,
                radius,
                ..
            } => {
                let available_count = self
                    .actor_kind_available_instance_count(actor_kind_id)
                    .min(usize::from(count));
                (matches!(target, TargetSelection::SelfTarget)
                    && ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget)
                    && available_count > 0)
                    .then(|| {
                        self.summon_positions_around(
                            self.player.position,
                            u8::try_from(available_count)
                                .expect("summon count is bounded by its u8 content field"),
                            radius,
                            actor_kind_id,
                        )
                    })
                    .flatten()
                    .map(|positions| AbilityTargetPlan::Summon { positions })
            }
            AbilityEffectDefinition::SummonCategory {
                ref category,
                ref upgraded_category,
                upgrade_at_level,
                maximum_level,
                count_dice,
                count_sides,
                count_bonus,
                maximum_count,
                hostile_chance_percent,
                group_count_dice,
                group_count_sides,
                group_count_bonus,
                allow_unique_hostile,
                radius,
                ..
            } => {
                if !matches!(target, TargetSelection::SelfTarget)
                    || !ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget)
                {
                    return None;
                }
                let selected_category = upgraded_category
                    .as_deref()
                    .zip(upgrade_at_level)
                    .filter(|(_, level)| self.progress.level >= *level)
                    .map_or(category.as_str(), |(category, _)| category);
                let excluded_upgrade_category = upgraded_category
                    .as_deref()
                    .filter(|category| *category != selected_category);
                let friendly_candidate_kind_ids = self.summon_category_candidate_kind_ids(
                    selected_category,
                    excluded_upgrade_category,
                    maximum_level,
                    false,
                );
                let hostile_candidate_kind_ids = self.summon_category_candidate_kind_ids(
                    selected_category,
                    excluded_upgrade_category,
                    maximum_level,
                    allow_unique_hostile,
                );
                if (hostile_chance_percent < 100 && friendly_candidate_kind_ids.is_empty())
                    || (hostile_chance_percent > 0 && hostile_candidate_kind_ids.is_empty())
                {
                    return None;
                }
                let normal_maximum = (usize::from(count_dice) * usize::from(count_sides)
                    + usize::from(count_bonus))
                .min(maximum_count.map_or(usize::MAX, usize::from));
                let group_maximum = (usize::from(group_count_dice)
                    * usize::from(group_count_sides)
                    + usize::from(group_count_bonus))
                .min(maximum_count.map_or(usize::MAX, usize::from));
                let position_candidate_kind_ids = friendly_candidate_kind_ids
                    .iter()
                    .chain(&hostile_candidate_kind_ids)
                    .cloned()
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();
                let positions = self
                    .open_positions_around_for_actor_kinds(
                        self.player.position,
                        radius,
                        &position_candidate_kind_ids,
                    )
                    .into_iter()
                    .take(normal_maximum.max(group_maximum))
                    .collect::<Vec<_>>();
                (!positions.is_empty()).then_some(AbilityTargetPlan::SummonCategory {
                    friendly_candidate_kind_ids,
                    hostile_candidate_kind_ids,
                    positions,
                })
            }
            AbilityEffectDefinition::AnimateDead { .. } => {
                (matches!(target, TargetSelection::SelfTarget)
                    && ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget))
                .then_some(AbilityTargetPlan::SelfTarget)
            }
            AbilityEffectDefinition::IdentifyItem { .. } => {
                let TargetSelection::Item { item_id } = target else {
                    return None;
                };
                (ability
                    .target
                    .modes
                    .contains(&AbilityTargetModeDefinition::Item)
                    && self.items.iter().any(|item| {
                        item.id == *item_id
                            && match &item.location {
                                ItemLocation::Inventory | ItemLocation::Equipped { .. } => true,
                                ItemLocation::Ground(position) => *position == self.player.position,
                                ItemLocation::CarriedBy { .. }
                                | ItemLocation::Shop { .. }
                                | ItemLocation::Home { .. } => false,
                            }
                    }))
                .then(|| AbilityTargetPlan::Item {
                    item_id: item_id.clone(),
                })
            }
            AbilityEffectDefinition::IdentifyOrMassIdentify { mass, .. } => {
                if mass {
                    (matches!(target, TargetSelection::SelfTarget)
                        && ability
                            .target
                            .modes
                            .contains(&AbilityTargetModeDefinition::SelfTarget))
                    .then_some(AbilityTargetPlan::SelfTarget)
                } else {
                    let TargetSelection::Item { item_id } = target else {
                        return None;
                    };
                    (ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::Item)
                        && self.items.iter().any(|item| {
                            item.id == *item_id
                                && match &item.location {
                                    ItemLocation::Inventory | ItemLocation::Equipped { .. } => true,
                                    ItemLocation::Ground(position) => {
                                        *position == self.player.position
                                    }
                                    ItemLocation::CarriedBy { .. }
                                    | ItemLocation::Shop { .. }
                                    | ItemLocation::Home { .. } => false,
                                }
                        }))
                    .then(|| AbilityTargetPlan::Item {
                        item_id: item_id.clone(),
                    })
                }
            }
            AbilityEffectDefinition::BrandWeapon { .. } => {
                let TargetSelection::Item { item_id } = target else {
                    return None;
                };
                self.items
                    .iter()
                    .find(|item| item.id == *item_id && self.item_is_brandable_weapon(item))
                    .map(|_| AbilityTargetPlan::Item {
                        item_id: item_id.clone(),
                    })
            }
            AbilityEffectDefinition::Detect { .. } => {
                (matches!(target, TargetSelection::SelfTarget)
                    && ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget))
                .then_some(AbilityTargetPlan::Detect)
            }
            AbilityEffectDefinition::RefuelEquippedLight { .. } => {
                (matches!(target, TargetSelection::SelfTarget)
                    && ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget))
                .then_some(AbilityTargetPlan::SelfTarget)
            }
            AbilityEffectDefinition::TransformTerrain {
                ref source_terrain_ids,
                ref target_terrain_id,
                radius,
            } => {
                let TargetSelection::Position { position } = target else {
                    return None;
                };
                self.terrain_transform_positions(
                    ability,
                    *position,
                    source_terrain_ids,
                    target_terrain_id,
                    radius,
                )
                .map(|positions| AbilityTargetPlan::TerrainTransform {
                    center: *position,
                    positions,
                })
            }
            AbilityEffectDefinition::ApplyStatus { .. }
            | AbilityEffectDefinition::RemoveStatus { .. }
            | AbilityEffectDefinition::Control { .. }
            | AbilityEffectDefinition::Sequence { .. } => {
                if ability
                    .target
                    .modes
                    .contains(&AbilityTargetModeDefinition::SelfTarget)
                {
                    (matches!(target, TargetSelection::SelfTarget))
                        .then_some(AbilityTargetPlan::SelfTarget)
                } else {
                    self.ability_path(ability, target)
                        .map(|path| AbilityTargetPlan::Projectile {
                            path,
                            stop_at_actor: true,
                        })
                }
            }
            AbilityEffectDefinition::Heal { .. }
            | AbilityEffectDefinition::HealDice { .. }
            | AbilityEffectDefinition::ReduceStatus { .. }
            | AbilityEffectDefinition::SatisfyHunger
            | AbilityEffectDefinition::CreateItem { .. }
            | AbilityEffectDefinition::CreateStair { .. }
            | AbilityEffectDefinition::SelfKnowledge
            | AbilityEffectDefinition::Clairvoyance { .. }
            | AbilityEffectDefinition::Probe
            | AbilityEffectDefinition::CreateDoor { .. }
            | AbilityEffectDefinition::DeviceMastery { .. }
            | AbilityEffectDefinition::Banish { .. }
            | AbilityEffectDefinition::Invulnerability { .. }
            | AbilityEffectDefinition::LightArea { .. }
            | AbilityEffectDefinition::MassSleepOrStasis { .. } => {
                (matches!(target, TargetSelection::SelfTarget)
                    && ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget))
                .then_some(AbilityTargetPlan::SelfTarget)
            }
            AbilityEffectDefinition::RestoreVitality { .. } => {
                (matches!(target, TargetSelection::SelfTarget)
                    && ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget))
                .then_some(AbilityTargetPlan::SelfTarget)
            }
            AbilityEffectDefinition::VisibleDamage { .. }
            | AbilityEffectDefinition::VisibleApplyStatus { .. }
            | AbilityEffectDefinition::AggravateMonsters
            | AbilityEffectDefinition::Concentrate
            | AbilityEffectDefinition::NoOp { .. } => {
                (matches!(target, TargetSelection::SelfTarget)
                    && ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget))
                .then_some(AbilityTargetPlan::SelfTarget)
            }
            AbilityEffectDefinition::RandomChoice { .. } => {
                self.ability_path(ability, target)
                    .map(|path| AbilityTargetPlan::Projectile {
                        path,
                        stop_at_actor: true,
                    })
            }
            AbilityEffectDefinition::Damage { .. }
            | AbilityEffectDefinition::Malediction { .. }
            | AbilityEffectDefinition::DeathRay { .. }
            | AbilityEffectDefinition::PolymorphTarget => {
                self.ability_path(ability, target)
                    .map(|path| AbilityTargetPlan::Projectile {
                        path,
                        stop_at_actor: true,
                    })
            }
            AbilityEffectDefinition::AreaDamage { .. } => {
                if matches!(target, TargetSelection::SelfTarget)
                    && ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget)
                {
                    Some(AbilityTargetPlan::Projectile {
                        path: Vec::new(),
                        stop_at_actor: false,
                    })
                } else {
                    self.ability_path(ability, target)
                        .map(|path| AbilityTargetPlan::Projectile {
                            path,
                            stop_at_actor: matches!(target, TargetSelection::Direction { .. }),
                        })
                }
            }
            AbilityEffectDefinition::BeamDamage { .. }
            | AbilityEffectDefinition::LightLine { .. }
            | AbilityEffectDefinition::TerrainBeam { .. }
            | AbilityEffectDefinition::BoltOrBeamDamage { .. } => self
                .beam_ability_path(ability, target)
                .map(|path| AbilityTargetPlan::Projectile {
                    path,
                    stop_at_actor: false,
                }),
            AbilityEffectDefinition::BoltOrAreaDamage { .. } => self
                .ability_path(ability, target)
                .map(|path| AbilityTargetPlan::Projectile {
                    path,
                    stop_at_actor: matches!(target, TargetSelection::Direction { .. }),
                }),
            AbilityEffectDefinition::Genocide {
                scope: AbilityGenocideScopeDefinition::Nearby,
                ..
            } => (matches!(target, TargetSelection::SelfTarget)
                && ability
                    .target
                    .modes
                    .contains(&AbilityTargetModeDefinition::SelfTarget))
            .then_some(AbilityTargetPlan::SelfTarget),
            AbilityEffectDefinition::DrainLife { .. }
            | AbilityEffectDefinition::Genocide { .. } => {
                self.ability_path(ability, target)
                    .map(|path| AbilityTargetPlan::Projectile {
                        path,
                        stop_at_actor: true,
                    })
            }
            AbilityEffectDefinition::ConeDamage { radius, .. } => {
                let TargetSelection::Direction { direction } = target else {
                    return None;
                };
                self.ability_path(ability, target)
                    .map(|path| AbilityTargetPlan::Cone {
                        path,
                        direction: *direction,
                        radius,
                    })
            }
        }
    }

    fn resolve_player_melee_adjacent_effect(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let target_ids = TERRAIN_INTERACTION_DIRECTIONS
            .iter()
            .filter_map(|direction| {
                let position = self.position_in_direction(*direction);
                self.entities
                    .iter()
                    .find(|entity| {
                        entity.hp > 0
                            && entity.position == position
                            && !self.actor_is_player_side(entity)
                    })
                    .map(|entity| entity.id.clone())
            })
            .collect::<Vec<_>>();
        for target_id in target_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == target_id && entity.hp > 0)
            else {
                continue;
            };
            self.resolve_player_melee(index, false, events, changed, removed_entities)?;
            if self.player_is_dead() {
                break;
            }
        }
        Ok(())
    }
}
