// SPDX-License-Identifier: MPL-2.0

use crate::effect::{DamagePacket, STATUS_HALLUCINATION, STATUS_TELEPATHY, resolve_damage};
use crate::error::CoreError;
use crate::event::{DomainEvent, ProjectileTrace};
use crate::game::ability_scaling::spell_powered_ability_value;
use crate::game::damage::{FatalityPolicy, commit_damage_application, plan_damage_application};
use crate::game::inventory::ItemIdentificationRequest;
use crate::game::player_stats::{actor_melee_routine_dto, derived_speed};
use crate::game::projectile_geometry::{has_line_of_effect, rfb_area_damage};
use crate::game::status_effects::apply_ability_status_effect;
use crate::game::terrain::TerrainChangeSource;
use crate::game::{Game, TERRAIN_INTERACTION_DIRECTIONS, ability_detect_subject_dto};
use crate::resistance::{DamageType, ResistanceLevel};
use crate::state::ItemLocation;
use rfb_content::{
    AbilityDefinition, AbilityDetectSubjectDefinition, AbilityEffectDefinition,
    AbilitySpellPowerField, AbilityStatusStackingDefinition, AbilityTerrainBeamOperationDefinition,
    EquipmentBonuses, StatModifiers, TerrainDiggingResolution,
};
use rfb_protocol::{
    AbilityAreaDamageResolutionDto, AbilityBeamDamageResolutionDto, AbilityDetectResolutionDto,
    AbilityDetectSubjectDto, AbilityEffectResolutionDto, AbilityEffectSkipReasonDto,
    AbilityEffectsResolutionDto, AbilityMonsterProbeResolutionDto, AbilityProbeAlignmentDto,
    AbilityProbeTargetDto, AbilityTerrainTransformResolutionDto, EntityFactionDto,
    MonsterAlignmentDto, Position, ProbedMonsterDto, VirtueKindDto,
};
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::game) enum EarthquakeSource {
    Ability(String),
    Monster(String),
    Weapon(String),
}

impl Game {
    pub(super) fn resolve_player_light_line_effect(
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
            if !self.dungeon_has_darkness()
                && let Some(index) = self.index(*position)
            {
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

    pub(super) fn resolve_player_light_area_effect(
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
            sunlight_burn_damage_dice,
            sunlight_burn_damage_sides,
        } = ability.effect
        else {
            unreachable!("light-area executor requires a light-area effect");
        };
        // The caller rolls damage before spells2.c:lite_area can reject projection.
        let base_raw_damage = self.roll_damage(damage_dice, damage_sides).max(0);
        if self.dungeon_has_darkness() {
            events.push(DomainEvent::DungeonDarknessAbsorbedLight);
        } else {
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
            let base_raw_damage = i32::try_from(spell_powered_ability_value(
                ability,
                0,
                AbilitySpellPowerField::FinalDamage,
                u64::try_from(base_raw_damage).expect("light damage must be non-negative"),
            ))
            .expect("spell-powered light damage must fit i32");

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
        }
        // Nature Daylight's vampire backlash is outside lite_area in do-spell.c.
        if sunlight_burn_damage_dice > 0 && self.player_fails_sunlight_save() {
            let raw_damage = self
                .roll_damage(sunlight_burn_damage_dice, sunlight_burn_damage_sides)
                .max(0);
            self.apply_player_sunlight_damage(&ability.id, raw_damage, events);
        }
        Ok(())
    }

    fn player_fails_sunlight_save(&mut self) -> bool {
        if !self.player_is_vampire() {
            return false;
        }
        let light_resistance = self.player_resistance_percent(DamageType::Light).max(0);
        self.rng.bounded(33) >= u64::try_from(light_resistance).unwrap_or(0)
    }

    fn apply_player_sunlight_damage(
        &mut self,
        ability_id: &str,
        raw_damage: i32,
        events: &mut Vec<DomainEvent>,
    ) -> i32 {
        let damage = resolve_damage(
            DamagePacket::new(raw_damage, DamageType::Light),
            ResistanceLevel::Normal,
        );
        let application = self.apply_final_player_damage(damage, FatalityPolicy::BelowZero);
        let damage = application.damage;
        let center = self.player.position;
        events.push(DomainEvent::AbilityHit {
            ability_id: ability_id.to_owned(),
            target_kind_id: self.player.kind_id.clone(),
            damage,
            trace: ProjectileTrace {
                origin: center,
                impact: center,
                landing: center,
                traversed: vec![center],
            },
        });
        if application.fatal {
            events.push(DomainEvent::PlayerDied {
                source_kind_id: self.player.kind_id.clone(),
                method_id: Some(ability_id.to_owned()),
                damage,
            });
        }
        damage.applied
    }

    pub(super) fn resolve_player_terrain_beam_effect(
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
        self.resolve_terrain_beam_effect(
            &ability.id,
            operation,
            path,
            events,
            changed,
            removed_entities,
        )
    }

    pub(in crate::game) fn resolve_terrain_beam_effect(
        &mut self,
        source_id: &str,
        operation: AbilityTerrainBeamOperationDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
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
                ability_id: source_id.to_owned(),
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
                ability_id: source_id.to_owned(),
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
                    source_id,
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

    pub(super) fn resolve_player_create_current_terrain_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::CreateCurrentTerrain {
            source_terrain_ids,
            target_terrain_id,
        } = &ability.effect
        else {
            unreachable!("current terrain executor requires a current terrain effect");
        };
        let position = self.player.position;
        let terrain_id = self
            .current_terrain_creation_replacement(source_terrain_ids, target_terrain_id)
            .map(|(position, terrain_id)| {
                let index = self
                    .index(position)
                    .expect("planned current terrain creation must remain in bounds");
                self.terrain[index] = terrain_id.clone();
                self.revealed_terrain.remove(&position);
                changed.insert(position);
                terrain_id
            });
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::CreateCurrentTerrain {
                    effect_index: 0,
                    position,
                    terrain_id,
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_create_stair_effect(
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

    pub(super) fn resolve_player_probe_effect(
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

    pub(super) fn resolve_player_create_door_effect(
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

    pub(super) fn resolve_player_consume_terrain_effect(
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

    pub(super) fn resolve_player_earthquake_effect(
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

    pub(super) fn resolve_player_area_destruction_effect(
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

    pub(in crate::game) fn resolve_monster_shatter_earthquake(
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

    pub(in crate::game) fn resolve_player_impact_earthquake(
        &mut self,
        source_item_id: String,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        self.resolve_earthquake(
            self.player.position,
            10,
            15,
            "demo.terrain.floor",
            &[
                "demo.terrain.wall".to_owned(),
                "demo.terrain.quartz-vein".to_owned(),
                "demo.terrain.magma-vein".to_owned(),
            ],
            EarthquakeSource::Weapon(source_item_id),
            events,
            changed,
            removed_entities,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::game) fn resolve_earthquake(
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
            EarthquakeSource::Weapon(_) => TerrainChangeSource::Magic,
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
                let application = self.apply_final_player_damage(damage, FatalityPolicy::BelowZero);
                let damage = application.damage;
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
                    EarthquakeSource::Weapon(source_item_id) => {
                        events.push(DomainEvent::PlayerWeaponEarthquakeHit {
                            source_item_id: source_item_id.clone(),
                            damage,
                        });
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
                        EarthquakeSource::Weapon(source_item_id) => self.resolve_actor_death(
                            actor_index,
                            DomainEvent::PlayerWeaponEarthquakeSlew {
                                source_item_id: source_item_id.clone(),
                                target_kind_id,
                                damage,
                            },
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
                } else if let EarthquakeSource::Weapon(source_item_id) = &source {
                    events.push(DomainEvent::PlayerWeaponEarthquakeHit {
                        source_item_id: source_item_id.clone(),
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
            EarthquakeSource::Weapon(source_item_id) => {
                events.push(DomainEvent::PlayerWeaponEarthquakeResolved {
                    source_item_id,
                    resolution,
                });
            }
        }
        Ok(())
    }

    pub(super) fn resolve_player_clairvoyance_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::Clairvoyance {
            telepathy_duration_ticks,
            telepathy_duration_dice,
            telepathy_duration_sides,
            grants_virtues,
            grants_telepathy,
        } = ability.effect
        else {
            unreachable!("clairvoyance executor requires a clairvoyance effect");
        };

        if grants_virtues {
            self.add_virtue(VirtueKindDto::Knowledge, 1);
            self.add_virtue(VirtueKindDto::Enlightenment, 1);
        }

        self.reveal_and_light_floor(&ability.id, events, changed);

        let telepathy_resolution = if !grants_telepathy || self.player_has_permanent_telepathy() {
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

    fn reveal_and_light_floor(
        &mut self,
        ability_id: &str,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let mut mapped_positions = Vec::with_capacity(self.terrain.len());
        let illuminate = !self.dungeon_has_darkness();
        for y in 0..self.height {
            for x in 0..self.width {
                let position = Position {
                    x: i32::from(x),
                    y: i32::from(y),
                };
                let index = self.index(position).expect("floor position must be valid");
                if !self.explored[index] || (illuminate && !self.glow[index]) {
                    changed.insert(position);
                }
                self.explored[index] = true;
                if illuminate {
                    self.glow[index] = true;
                }
                mapped_positions.push(position);
            }
        }
        events.push(DomainEvent::AbilityDetected {
            ability_id: ability_id.to_owned(),
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
            ability_id: ability_id.to_owned(),
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
    }

    pub(super) fn resolve_player_call_sunlight_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::CallSunlight { vampire_damage } = ability.effect else {
            unreachable!("call sunlight executor requires a call sunlight effect");
        };
        self.add_virtue(VirtueKindDto::Knowledge, 1);
        self.add_virtue(VirtueKindDto::Enlightenment, 1);
        self.reveal_and_light_floor(&ability.id, events, changed);
        let applied_vampire_damage = if self.player_fails_sunlight_save() {
            self.apply_player_sunlight_damage(&ability.id, i32::from(vampire_damage), events)
        } else {
            0
        };
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::CallSunlight {
                    effect_index: 0,
                    vampire_damage: applied_vampire_damage,
                }],
            },
            trace: None,
        });
    }

    pub(in crate::game) fn resolve_player_probe_monsters_effect(
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

    pub(in crate::game) fn resolve_player_detection_effect(
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
                radius: self.dungeon_detection_radius(*radius),
                persistent: *persistent,
                through_walls: *through_walls,
                detected_positions,
                detected_entity_ids,
            },
        });
    }

    pub(in crate::game) fn resolve_terrain_transform_effect(
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

    pub(super) fn resolve_player_adjacent_terrain_creation_effect(
        &mut self,
        ability: &AbilityDefinition,
        mut replacements: Vec<(Position, String)>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::CreateAdjacentTerrain {
            source_terrain_ids,
            target_terrain_id,
        } = &ability.effect
        else {
            unreachable!("adjacent terrain executor requires an adjacent terrain effect");
        };
        if ability.id == "rfb.ability.race.summon-tree" && self.progress.level < 45 {
            // Low-level placement uses in_bounds (excluding the outer wall);
            // level 45's projection uses in_bounds2, as the shared planner does.
            replacements.retain(|(position, _)| {
                position.x > 0
                    && position.y > 0
                    && position.x < i32::from(self.width) - 1
                    && position.y < i32::from(self.height) - 1
            });
            let mut selected = None;
            for _ in 0..5 {
                // summon_tree_spell draws randint0(9): 5 retries without
                // spending an attempt, 0 wastes one, and 9 is never drawn.
                let direction = loop {
                    let roll = self.rng.bounded(9);
                    if roll != 5 {
                        break roll;
                    }
                };
                let (dx, dy) = [
                    (0, 0),
                    (-1, 1),
                    (0, 1),
                    (1, 1),
                    (-1, 0),
                    (0, 0),
                    (1, 0),
                    (-1, -1),
                    (0, -1),
                ][direction as usize];
                let position = Position {
                    x: self.player.position.x + dx,
                    y: self.player.position.y + dy,
                };
                selected = replacements
                    .iter()
                    .find(|(candidate, _)| *candidate == position)
                    .cloned();
                if selected.is_some() {
                    break;
                }
            }
            replacements = selected.into_iter().collect();
            if replacements.is_empty() {
                events.push(DomainEvent::AbilityEffectsResolved {
                    ability_id: ability.id.clone(),
                    resolution: AbilityEffectsResolutionDto {
                        target_entity_id: None,
                        target_kind_id: None,
                        effects: vec![AbilityEffectResolutionDto::NoOp {
                            effect_index: 0,
                            reason: "no-trees-answer".to_owned(),
                        }],
                    },
                    trace: None,
                });
                return;
            }
        }
        let transformed_positions = self.apply_adjacent_terrain_creation(replacements, changed);
        events.push(DomainEvent::AbilityTerrainTransformed {
            ability_id: ability.id.clone(),
            resolution: AbilityTerrainTransformResolutionDto {
                center: self.player.position,
                radius: 1,
                source_terrain_ids: source_terrain_ids.clone(),
                target_terrain_id: target_terrain_id.clone(),
                transformed_positions,
            },
        });
    }
}
