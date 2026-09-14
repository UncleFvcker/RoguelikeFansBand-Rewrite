// SPDX-License-Identifier: MPL-2.0
// RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c: cave.c:hit_mon_trap.
use super::*;
use crate::game::terrain::TerrainChangeSource;

impl Game {
    pub(in crate::game) fn trigger_law_trap(
        &mut self,
        index: usize,
        position: Position,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<bool, CoreError> {
        let terrain_index = self.index(position).unwrap();
        let trap = self.terrain[terrain_index].clone();
        let id = self.entities[index].id.clone();
        let actor = self
            .actor_runtime_definition(&self.entities[index])
            .unwrap()
            .clone();
        let level = self.progress.level;
        self.replace_terrain_from_source(
            position,
            "demo.terrain.floor",
            TerrainChangeSource::Magic,
            events,
            changed,
        );
        if self.rng.bounded(299 * u64::from(level) / 50) < u64::from(actor.level) {
            return Ok(true);
        }
        let mut ability = self
            .content
            .ability("demo.ability.law-basic-trap")
            .unwrap()
            .clone();
        ability.id = match trap.as_str() {
            "demo.terrain.law-expert-trap" => "demo.ability.law-expert-trap",
            "demo.terrain.law-semicolon" => "demo.ability.law-semicolon-of-punishment",
            _ => "demo.ability.law-basic-trap",
        }
        .into();
        if trap == "demo.terrain.law-semicolon" {
            self.law_trap_damage(
                &ability.id,
                position,
                0,
                DamageType::Force,
                i32::from(level + 32),
                events,
                changed,
                removed,
            )?;
        } else if trap == "demo.terrain.law-basic-trap" {
            match self.rng.bounded(9) {
                n @ 0..=2 => {
                    let (kind, radius) = match n {
                        0 => (crate::effect::STATUS_SLEEP, 1),
                        1 => (STATUS_CONFUSION, 3),
                        _ => (STATUS_SLOW, 1),
                    };
                    self.law_trap_status(
                        &ability, position, radius, kind, level, events, changed, removed,
                    )?;
                }
                3 => {
                    if !actor.tags.iter().any(|t| t == "resist-teleport") {
                        let effect = self.resolve_teleport_away_target(index, 0, 0, 100, changed);
                        events.push(DomainEvent::AbilityEffectsResolved {
                            ability_id: ability.id.clone(),
                            resolution: rfb_protocol::AbilityEffectsResolutionDto {
                                target_entity_id: Some(id.clone()),
                                target_kind_id: Some(actor.id.clone()),
                                effects: vec![effect],
                            },
                            trace: None,
                        });
                    }
                }
                4 if self.law_trapdoor_available() => {
                    if !actor
                        .movement
                        .modes
                        .contains(&rfb_content::ActorMovementMode::Fly)
                    {
                        self.erase_monsters_without_death(
                            std::slice::from_ref(&id),
                            changed,
                            removed,
                        );
                    }
                }
                _ => {
                    let damage = i32::from(level / 2) + self.roll_damage(5, 5);
                    self.law_trap_damage(
                        &ability.id,
                        position,
                        1,
                        DamageType::Physical,
                        damage,
                        events,
                        changed,
                        removed,
                    )?;
                }
            }
        } else {
            match self.rng.bounded(7) {
                0 => {
                    let damage = 2 * (self.roll_damage(6, 5) + i32::from(level / 3));
                    self.law_trap_damage(
                        &ability.id,
                        position,
                        1,
                        DamageType::Physical,
                        damage,
                        events,
                        changed,
                        removed,
                    )?;
                }
                1 => {
                    let damage = self.roll_damage(8, 7) + i32::from(level / 2);
                    self.law_trap_damage(
                        &ability.id,
                        position,
                        2,
                        DamageType::Sound,
                        damage,
                        events,
                        changed,
                        removed,
                    )?;
                }
                2 => {
                    let radius = (self.rng.bounded(2) + 1) as u8;
                    let damage = self.roll_damage(8, 8) + i32::from(level);
                    self.law_trap_damage(
                        &ability.id,
                        position,
                        radius,
                        DamageType::Shards,
                        damage,
                        events,
                        changed,
                        removed,
                    )?;
                }
                3 => {
                    for kind in [
                        DamageType::Fire,
                        DamageType::Cold,
                        DamageType::Electricity,
                        DamageType::Acid,
                        DamageType::Poison,
                    ] {
                        let radius = (self.rng.bounded(2) + 1) as u8;
                        let damage = self.roll_damage(4, 6) + i32::from(level / 5);
                        self.law_trap_damage(
                            &ability.id,
                            position,
                            radius,
                            kind,
                            damage,
                            events,
                            changed,
                            removed,
                        )?;
                    }
                }
                4 => {
                    let damage = self.roll_damage(7, 7) + i32::from(level);
                    self.law_trap_damage(
                        &ability.id,
                        position,
                        5,
                        DamageType::Disintegrate,
                        damage,
                        events,
                        changed,
                        removed,
                    )?;
                }
                5 => self.law_trap_status(
                    &ability,
                    position,
                    1,
                    STATUS_PARALYSIS,
                    3 * level / 2,
                    events,
                    changed,
                    removed,
                )?,
                _ => {
                    self.law_trap_damage(
                        &ability.id,
                        position,
                        10,
                        DamageType::Disintegrate,
                        50,
                        events,
                        changed,
                        removed,
                    )?;
                    let cells = self
                        .area_damage_cells(position, 10)
                        .into_iter()
                        .map(|(_, p)| p)
                        .collect::<Vec<_>>();
                    for cell in cells {
                        if self.is_walkable(cell) {
                            self.replace_terrain_from_source(
                                cell,
                                "demo.terrain.surface-water-shallow",
                                TerrainChangeSource::Magic,
                                events,
                                changed,
                            );
                        }
                    }
                    let hostile = self.rng.bounded(3) != 0;
                    self.trump_summon_batch(
                        &ability,
                        "piranha",
                        position,
                        level * 2,
                        1 + level / 10,
                        hostile,
                        false,
                        true,
                        events,
                        changed,
                    );
                }
            }
        }
        Ok(self.entities.iter().any(|a| a.id == id && a.hp > 0))
    }

    fn law_trapdoor_available(&self) -> bool {
        let world = self.content.world(&self.world_id).unwrap();
        world
            .procedural_floors
            .iter()
            .find(|f| f.id == self.current_floor_id)
            .is_some_and(|f| {
                f.depth > 0
                    && self.current_floor_task_id().is_none()
                    && world.procedural_floors.iter().any(|next| {
                        next.depth == f.depth + 1
                            && crate::game::floor_dungeon_id(world, &next.id)
                                == crate::game::floor_dungeon_id(world, &f.id)
                    })
            })
    }

    #[allow(clippy::too_many_arguments)]
    fn law_trap_damage(
        &mut self,
        source: &str,
        center: Position,
        radius: u8,
        kind: DamageType,
        damage: i32,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        self.resolve_player_area_damage_with_base_policy(
            source,
            vec![center],
            false,
            kind,
            radius,
            None,
            damage,
            true,
            true,
            kind == DamageType::Physical,
            events,
            changed,
            removed,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn law_trap_status(
        &mut self,
        ability: &AbilityDefinition,
        center: Position,
        radius: u8,
        kind: &str,
        power: u16,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let positions = self
            .area_damage_cells(center, radius)
            .into_iter()
            .collect::<Vec<_>>();
        for (distance, position) in positions {
            let Some(actor) = self
                .entities
                .iter()
                .find(|a| a.hp > 0 && a.position == position)
            else {
                continue;
            };
            let power = power.div_ceil(1 + distance as u16);
            if kind != STATUS_SLOW {
                let projection = match kind {
                    crate::effect::STATUS_SLEEP => {
                        rfb_content::MonsterStatusProjectionDefinition::Sleep
                    }
                    STATUS_CONFUSION => rfb_content::MonsterStatusProjectionDefinition::Confusion,
                    _ => rfb_content::MonsterStatusProjectionDefinition::Stasis,
                };
                self.resolve_monster_status_targets(
                    &ability.id,
                    projection,
                    power,
                    vec![actor.id.clone()],
                    events,
                    changed,
                );
                continue;
            }
            let mut branch = ability.clone();
            branch.effect = control_status(kind, power, false);
            self.resolve_player_ability_effect(
                branch,
                AbilityTargetPlan::Projectile {
                    path: vec![position],
                    stop_at_actor: true,
                },
                events,
                changed,
                removed,
            )?;
        }
        Ok(())
    }
}
