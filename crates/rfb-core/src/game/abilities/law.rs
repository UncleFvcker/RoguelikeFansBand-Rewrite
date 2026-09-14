// SPDX-License-Identifier: MPL-2.0
// RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c: lawyer.c, tables.c, gf.c.
mod traps;
use super::AbilityTargetPlan;
use crate::effect::{
    STATUS_CONFUSION, STATUS_FEAR, STATUS_PARALYSIS, STATUS_SLOW, STATUS_STUN, StatusStacking,
    apply_status,
};
use crate::error::CoreError;
use crate::event::DomainEvent;
use crate::game::Game;
use crate::game::ability_scaling::spell_power_value;
use crate::game::monster_combat::melee_status;
use crate::resistance::{DamageType, ResistanceLevel};
use rfb_content::{
    AbilityDefinition, AbilityDetectSubjectDefinition as Subject, AbilityEffectDefinition as E,
    AbilityStatusStackingDefinition as Stacking,
    AbilityTerrainBeamOperationDefinition as TerrainOp, ActorDamageType as D,
};
use rfb_protocol::{Position, TargetSelection, VirtueKindDto};
use std::collections::BTreeSet;

fn detect(subject: Subject, category: &str, radius: u8, persistent: bool) -> E {
    E::Detect {
        subject,
        category: category.into(),
        radius,
        persistent,
        through_walls: true,
    }
}

fn control_status(kind: &str, power: u16, visible: bool) -> E {
    if visible {
        E::VisibleApplyStatus {
            status_kind_id: kind.into(),
            intensity: 1,
            duration_ticks: 25,
            duration_dice: 0,
            duration_sides: 0,
            stacking: Stacking::Extend,
            resistance_type: None,
            power: Some(power),
            target_category: None,
        }
    } else {
        E::ApplyStatus {
            status_kind_id: kind.into(),
            intensity: 1,
            duration_ticks: 25,
            duration_dice: 0,
            duration_sides: 0,
            stacking: Stacking::Extend,
            resistance_type: None,
            power: Some(power),
            granted_resistances: Default::default(),
            granted_brands: Default::default(),
            granted_modifiers: Default::default(),
            granted_equipment_bonuses: Default::default(),
            granted_status_immunities: Default::default(),
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        }
    }
}

impl Game {
    pub(in crate::game) fn player_has_law_aptitude(&self) -> bool {
        self.player_is_vampire()
            || self
                .character_definitions()
                .is_some_and(|(_, race, _, _)| race.id == "rfb-legacy.race.ent")
    }

    pub(in crate::game) fn law_dig_range(&self, bonus: i32) -> u16 {
        (spell_power_value(u64::from(self.progress.level / 4 + 1), bonus)
            + if self.player_has_law_aptitude() { 4 } else { 0 })
        .min(u64::from(u16::MAX)) as u16
    }

    fn law_effect(&self, ability: &AbilityDefinition, spell: u8) -> E {
        let level = self.progress.level;
        let power = |v: u16| {
            spell_power_value(u64::from(v), ability.spell_power_bonus).min(u64::from(u16::MAX))
                as u16
        };
        let radius = if level == 50 { 255 } else { (30 + level) as u8 };
        match spell {
            0 => detect(Subject::Terrain, "treasure", radius, true),
            1 => detect(Subject::Terrain, "trap", radius, true),
            2 => E::SatisfyHunger,
            3 => detect(Subject::Item, "item", radius, false),
            4 | 12 | 15 | 17 => E::CreateCurrentTerrain {
                source_terrain_ids: vec!["demo.terrain.floor".into()],
                target_terrain_id: format!(
                    "demo.terrain.{}",
                    match spell {
                        4 => "law-basic-trap",
                        12 => "law-semicolon",
                        15 => "warding-glyph",
                        _ => "law-expert-trap",
                    }
                ),
            },
            5 => E::TerrainBeam {
                operation: TerrainOp::DestroyTrapsAndDoors,
            },
            6 => E::IdentifyItem {
                full_identify_power: u16::from(level >= 45 && self.player_has_law_aptitude()),
                full_identify_roll_sides: 1,
            },
            7 => E::TerrainBeam {
                operation: TerrainOp::StoneToMud,
            },
            8 => detect(
                Subject::Actor,
                if level >= 25 {
                    "any-monster"
                } else {
                    "normal-monster"
                },
                30,
                false,
            ),
            9 => control_status(STATUS_SLOW, power(2 * level), false),
            10 => control_status(STATUS_CONFUSION, power(2 * level), false),
            11 => control_status(STATUS_FEAR, power(3 * level / 2), false),
            13 => control_status(STATUS_CONFUSION, power((2 * level).saturating_sub(5)), true),
            14 => E::CreateAdjacentTerrain {
                source_terrain_ids: vec!["demo.terrain.floor".into()],
                target_terrain_id: "demo.terrain.door-closed".into(),
            },
            16 => E::Control {
                category: "any-monster".into(),
                power: level / 2,
            },
            19 => E::VisibleDamage {
                damage_dice: 0,
                damage_sides: 0,
                damage_bonus: power(
                    (50 + i32::from(level) + i32::from(self.casting_spell_damage_bonus())).max(0)
                        as u16,
                ),
                damage_type: D::Physical,
                target_category: Some("undead".into()),
                unlife_change_on_hit: 0,
            },
            20 => E::ProbeMonsters,
            22 => {
                let damage = power(
                    (50 + 2 * i32::from(level) / 3
                        + i32::from(self.casting_spell_damage_bonus()) / 3)
                        .max(0) as u16,
                );
                E::DrainLife {
                    damage_dice: 0,
                    damage_sides: 0,
                    damage_bonus: if self.player_is_vampire() {
                        damage * 3 / 2
                    } else {
                        damage
                    },
                    damage_type: D::Physical,
                    target_category: "living".into(),
                    repeat: 1,
                    feeds: true,
                }
            }
            24 => E::BlinkSelf {
                radius: 10,
                line_of_sight: false,
            },
            26 => detect(Subject::Terrain, "map", 30, true),
            27 => E::BeamDamage {
                damage_dice: 0,
                damage_sides: 0,
                damage_bonus: power(30),
                damage_type: D::Disintegrate,
                maximum_range: Some(self.law_dig_range(ability.spell_power_bonus)),
            },
            30 => E::BlinkSelf {
                radius: (level * 5) as u8,
                line_of_sight: false,
            },
            18 | 21 | 23 | 25 | 28 | 29 | 31 => E::SatisfyHunger, // Special slots plan self.
            _ => unreachable!("validated Law slot"),
        }
    }

    pub(super) fn law_target_plan(
        &self,
        ability: &AbilityDefinition,
        spell: u8,
        target: &TargetSelection,
    ) -> Option<AbilityTargetPlan> {
        if spell == 29 {
            let TargetSelection::Entity { entity_id } = target else {
                return None;
            };
            let actor = self
                .entities
                .iter()
                .find(|a| a.id == *entity_id && a.hp > 0)?;
            return (self.riding_actor_id.as_deref() != Some(entity_id)
                && self.entity_is_visible_to_player(actor)
                && crate::game::projectile_geometry::has_line_of_effect(
                    self,
                    self.player.position,
                    actor.position,
                )
                && crate::game::projectile_geometry::rfb_distance(
                    self.player.position,
                    actor.position,
                ) <= u32::from(ability.target.range))
            .then(|| AbilityTargetPlan::LawSubpoena {
                target_entity_id: entity_id.clone(),
            });
        }
        let mut branch = ability.clone();
        branch.effect = self.law_effect(ability, spell);
        self.ability_target_plan(&branch, target)
    }

    pub(super) fn resolve_law(
        &mut self,
        ability: &AbilityDefinition,
        spell: u8,
        plan: AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<Option<Position>, CoreError> {
        let mut branch = ability.clone();
        branch.effect = self.law_effect(ability, spell);
        branch.spell_power_bonus = 0;
        let scaled = |v| spell_power_value(v, ability.spell_power_bonus) as u16;
        match spell {
            5 => {
                let AbilityTargetPlan::Projectile { path, .. } = plan else {
                    unreachable!()
                };
                let (trace, _) = self.trace_projectile_path_with_actor_policy(path, false);
                for position in trace
                    .traversed
                    .into_iter()
                    .chain(std::iter::once(trace.impact))
                {
                    let Some(terrain) = self
                        .index(position)
                        .and_then(|i| self.content.terrain(&self.terrain[i]))
                    else {
                        continue;
                    };
                    let replacement = if let Some(trap) = &terrain.trap {
                        Some(trap.disarm_to_terrain_id.clone())
                    } else if terrain.tags.iter().any(|t| t == "monster-trap") {
                        Some("demo.terrain.floor".into())
                    } else if terrain.open_to_terrain_id.is_some() {
                        Some("demo.terrain.door-closed".into())
                    } else {
                        None
                    };
                    if let Some(id) = replacement {
                        self.replace_terrain_from_source(
                            position,
                            &id,
                            crate::game::terrain::TerrainChangeSource::Magic,
                            events,
                            changed,
                        );
                    }
                }
                return Ok(None);
            }
            18 => {
                branch.effect = match self.rng.bounded(13) {
                    0..=4 => E::BlinkSelf {
                        radius: 10,
                        line_of_sight: false,
                    },
                    5..=9 => E::BlinkSelf {
                        radius: 222,
                        line_of_sight: false,
                    },
                    10..=11 => E::CreateStair {
                        up_terrain_id: "demo.terrain.stairs-up".into(),
                        down_terrain_id: "demo.terrain.stairs-down".into(),
                    },
                    _ => E::TeleportLevel,
                };
                if matches!(branch.effect, E::TeleportLevel) {
                    if self.law_escape_plan().is_some() {
                        self.begin_duelist_choice(rfb_protocol::DuelistPromptDto::LawEscape);
                    }
                    return Ok(None);
                }
                if let Some(plan) = self.ability_target_plan(&branch, &TargetSelection::SelfTarget)
                {
                    return self
                        .resolve_player_ability_effect(branch, plan, events, changed, removed);
                }
                return Ok(None);
            }
            21 | 25 => {
                let base = scaled(if spell == 21 { 20 } else { 50 });
                let duration =
                    u32::from(base) + self.rng.bounded(u64::from(base).max(1)) as u32 + 1;
                let mut status = melee_status(
                    if spell == 21 {
                        "rfb.status.law-spin"
                    } else {
                        "rfb.status.law-tread-softly"
                    },
                    duration,
                    &ability.id,
                );
                status.stacking = StatusStacking::KeepStrongest;
                if spell == 21 {
                    status
                        .status
                        .granted_resistances
                        .insert(DamageType::Nether, ResistanceLevel::Resistant);
                } else {
                    status.status.granted_equipment_bonuses.stealth_skill =
                        i32::from(3 + self.progress.level / 5);
                }
                apply_status(&mut self.player.statuses, status);
                return Ok(None);
            }
            23 => {
                self.reality_change_ticks = 2;
                self.advance_reality_change(events, changed, removed)?;
                return Ok(None);
            }
            28 => {
                let base = scaled(20);
                let duration =
                    u32::from(base) + self.rng.bounded(u64::from(base).max(1)) as u32 + 1;
                branch = self
                    .content
                    .ability("demo.ability.craft-berserk")
                    .expect("formal berserk")
                    .clone();
                branch.id = ability.id.clone();
                Self::apply_player_level_scaling(&mut branch, self.progress.level);
                let E::Sequence { effects } = &mut branch.effect else {
                    unreachable!("formal berserk sequence")
                };
                for effect in effects {
                    match effect {
                        E::ApplyStatus {
                            duration_ticks,
                            duration_dice,
                            duration_sides,
                            ..
                        } => {
                            *duration_ticks = duration;
                            *duration_dice = 0;
                            *duration_sides = 0;
                        }
                        E::Heal { amount } => *amount = 75,
                        _ => {}
                    }
                }
                return self.resolve_player_ability_effect(
                    branch,
                    AbilityTargetPlan::SelfTarget,
                    events,
                    changed,
                    removed,
                );
            }
            29 => {
                let AbilityTargetPlan::LawSubpoena { target_entity_id } = plan else {
                    unreachable!()
                };
                self.law_subpoena(ability, &target_entity_id, events, changed);
                return Ok(None);
            }
            31 => {
                let p = scaled(u64::from(60 + self.progress.level));
                for (kind, power) in [
                    (STATUS_SLOW, p),
                    (STATUS_STUN, 5 + p / 10),
                    (STATUS_CONFUSION, p),
                    (STATUS_FEAR, p),
                    (STATUS_PARALYSIS, p / 3),
                ] {
                    if kind == STATUS_PARALYSIS {
                        self.resolve_projected_monster_status(
                            &ability.id,
                            rfb_content::MonsterStatusProjectionDefinition::Stasis,
                            power,
                            events,
                            changed,
                        );
                        continue;
                    }
                    branch.effect = control_status(kind, power, true);
                    self.resolve_player_ability_effect(
                        branch.clone(),
                        AbilityTargetPlan::SelfTarget,
                        events,
                        changed,
                        removed,
                    )?;
                }
                return Ok(None);
            }
            16 => {
                if let E::Control { power, .. } = &mut branch.effect {
                    *power += self.roll_damage(5, 7) as u16;
                }
            }
            22 => {
                self.add_virtue(VirtueKindDto::Sacrifice, -1);
                self.add_virtue(VirtueKindDto::Vitality, -1);
            }
            _ => {}
        }
        self.resolve_player_ability_effect(branch.clone(), plan, events, changed, removed)?;
        let mut extra = Vec::new();
        if spell == 0 {
            extra.push(detect(
                Subject::Gold,
                "gold",
                if self.progress.level == 50 {
                    255
                } else {
                    (30 + self.progress.level) as u8
                },
                false,
            ));
        }
        if spell == 26 && self.player_has_law_aptitude() && self.progress.level >= 48 {
            extra.extend([
                detect(Subject::Terrain, "trap", 30, true),
                detect(Subject::Terrain, "door", 30, true),
                detect(Subject::Terrain, "stairs-up", 30, true),
                detect(Subject::Terrain, "stairs-down", 30, true),
                detect(Subject::Terrain, "treasure", 30, true),
                detect(Subject::Gold, "gold", 30, false),
                detect(Subject::Item, "item", 30, false),
                detect(Subject::Actor, "any-monster", 30, false),
            ]);
        }
        for effect in extra {
            branch.effect = effect;
            self.resolve_player_ability_effect(
                branch.clone(),
                AbilityTargetPlan::Detect,
                events,
                changed,
                removed,
            )?;
        }
        Ok(None)
    }

    fn law_subpoena(
        &mut self,
        ability: &AbilityDefinition,
        id: &str,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let index = self
            .entities
            .iter()
            .position(|a| a.id == id)
            .expect("planned subpoena target");
        let actor = self
            .actor_runtime_definition(&self.entities[index])
            .unwrap();
        let resistant = actor.tags.iter().any(|t| t == "resist-teleport");
        let immune = resistant && actor.tags.iter().any(|t| t == "resist-all");
        let level = actor.level;
        let roll = (resistant && !immune).then(|| (self.rng.bounded(100) + 1) as u8);
        let resisted = immune || roll.is_some_and(|r| level > u32::from(r));
        let from = self.entities[index].position;
        let mut to = None;
        if !resisted {
            self.entities[index]
                .statuses
                .retain(|s| s.kind_id != crate::effect::STATUS_SLEEP);
            // Shared map uses a finite candidate set; widen only when the adjacent ring is full.
            for radius in 1..=18 {
                let candidates = self
                    .area_damage_cells(self.player.position, radius)
                    .into_iter()
                    .map(|(_, p)| p)
                    .filter(|p| {
                        *p != self.player.position
                            && self.actor_can_enter_position(index, *p)
                            && self
                                .content
                                .terrain(&self.terrain[self.index(*p).unwrap()])
                                .is_some_and(Game::terrain_allows_passive_monster_displacement)
                            && !self.entities.iter().any(|a| a.hp > 0 && a.position == *p)
                    })
                    .collect::<Vec<_>>();
                if !candidates.is_empty() {
                    to = Some(candidates[self.rng.bounded(candidates.len() as u64) as usize]);
                    break;
                }
            }
            if let Some(p) = to {
                self.entities[index].position = p;
                changed.extend([from, p]);
            }
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: rfb_protocol::AbilityEffectsResolutionDto {
                target_entity_id: Some(id.into()),
                target_kind_id: Some(self.entities[index].kind_id.clone()),
                effects: vec![rfb_protocol::AbilityEffectResolutionDto::TeleportAway {
                    effect_index: 0,
                    target_entity_id: id.into(),
                    power: 100,
                    resistance_roll: roll,
                    resisted,
                    from,
                    to,
                }],
            },
            trace: None,
        });
    }

    pub(in crate::game) fn law_escape_plan(
        &self,
    ) -> Option<(AbilityDefinition, AbilityTargetPlan)> {
        let mut ability = self.content.ability("demo.ability.law-getaway")?.clone();
        ability.effect = E::TeleportLevel;
        let plan = self.ability_target_plan(&ability, &TargetSelection::SelfTarget)?;
        Some((ability, plan))
    }
}
