// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::game::abilities::terrain::EarthquakeSource;
use crate::game::projectile_geometry::rfb_area_damage;
use crate::game::terrain::TerrainChangeSource;
use rfb_content::AbilityTerrainBeamOperationDefinition;

impl Game {
    // RFB master spells2.c activate_ty_curse: cases deliberately fall through.
    pub(super) fn resolve_equipped_ty_curse(
        &mut self,
        source: &str,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let depth = self.floor_depth(&self.current_floor_id);
        let mut count = 0;
        let mut stop = false;
        let mut stat_index = 0;
        let stats = [
            AttributeKind::Strength,
            AttributeKind::Intelligence,
            AttributeKind::Wisdom,
            AttributeKind::Dexterity,
            AttributeKind::Constitution,
            AttributeKind::Charisma,
        ];
        loop {
            let mut branch = match self.rng.bounded(34) + 1 {
                28 | 29 => 0,
                30 | 31 => 1,
                32 | 33 => 2,
                34 => 3,
                1 | 2 | 3 | 16 | 17 => 4,
                4..=6 => 5,
                7..=9 | 18 => 6,
                10..=12 => 7,
                13..=15 | 19 | 20 => 8,
                21..=23 => 9,
                24 => 10,
                25 => 11,
                _ => 12,
            };
            loop {
                if branch <= 2 && count != 0 {
                    branch += 1;
                    continue;
                }
                match branch {
                    0 => {
                        let radius = 5 + self.rng.bounded(10) as u8;
                        self.ty_curse_earthquake(source, radius, events, changed, removed)?;
                    }
                    1 => {
                        let damage = self.roll_damage(10, 10);
                        self.ty_curse_blast(source, 8, damage, false, events, changed, removed)?;
                        self.resolve_item_life_loss(source, damage as u32, events);
                    }
                    2 => {
                        let distance = self.roll_damage(10, 10) as u16;
                        let positions = self.random_teleport_candidates(distance);
                        if !positions.is_empty() {
                            let position =
                                positions[self.rng.bounded(positions.len() as u64) as usize];
                            events.extend(self.relocate_player(position, changed));
                        }
                        if self.rng.bounded(13) != 0 {
                            count += self.ty_curse_high_summon(source, depth, events, changed);
                        }
                    }
                    3 => {
                        self.ty_curse_wall_breaker(source, events, changed, removed)?;
                        if self.rng.bounded(7) == 0 {
                            self.ty_curse_blast(source, 7, 50, true, events, changed, removed)?;
                            self.resolve_item_life_loss(source, 50, events);
                        }
                    }
                    4 => {
                        self.resolve_item_aggravation(source, events, changed);
                    }
                    5 => {
                        count += self.ty_curse_high_summon(source, depth, events, changed);
                    }
                    6 => {
                        count += self.ty_curse_summon(
                            source,
                            "any-monster",
                            depth,
                            true,
                            events,
                            changed,
                        );
                    }
                    7 => {
                        self.resolve_item_experience_loss(source, 16, events);
                    }
                    8 => {
                        let free_action =
                            self.player_status_immunities().contains(STATUS_PARALYSIS);
                        if !(stop
                            || free_action
                                && (self.rng.bounded(125) + 1)
                                    < self.player_derived_stats().saving_throw_skill.value.max(0)
                                        as u64)
                        {
                            let duration =
                                1 + self.rng.bounded(if free_action { 2 } else { 13 }) as u32;
                            let mut paralysis =
                                monster_combat::melee_status(STATUS_PARALYSIS, duration, source);
                            paralysis.stacking = crate::effect::StatusStacking::KeepStrongest;
                            apply_status(&mut self.player.statuses, paralysis);
                            changed.insert(self.player.position);
                            stop = true;
                        }
                    }
                    9 => {
                        let stat = stats[self.rng.bounded(6) as usize];
                        self.resolve_item_drain_attribute(source, stat, events);
                    }
                    10 => {
                        self.clear_current_floor_memory(changed);
                        for knowledge in self.item_property_knowledge.values_mut() {
                            knowledge.appraised = false;
                            knowledge.identified = false;
                            knowledge.known_affix_ids.clear();
                        }
                    }
                    11 => {
                        if depth > 65 && !stop {
                            let number = (depth / 50 + 1 + self.rng.bounded(2) as u16).min(4);
                            for _ in 0..number {
                                count += self
                                    .ty_curse_summon(source, "cyber", 100, false, events, changed);
                            }
                            stop = true;
                            break;
                        }
                    }
                    _ => {
                        while stat_index < stats.len() {
                            loop {
                                self.resolve_item_drain_attribute(
                                    source,
                                    stats[stat_index],
                                    events,
                                );
                                if self.rng.bounded(2) != 0 {
                                    break;
                                }
                            }
                            stat_index += 1;
                        }
                        break;
                    }
                }
                if self.player_is_dead() {
                    return Ok(());
                }
                if self.rng.bounded(6) != 0 {
                    break;
                }
                branch += 1;
            }
            if self.rng.bounded(3) != 0 || stop {
                break;
            }
        }
        Ok(())
    }

    fn ty_curse_earthquake(
        &mut self,
        source: &str,
        radius: u8,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        self.resolve_earthquake(
            self.player.position,
            radius,
            15,
            "demo.terrain.floor",
            &[
                "demo.terrain.wall".to_owned(),
                "demo.terrain.quartz-vein".to_owned(),
                "demo.terrain.magma-vein".to_owned(),
            ],
            EarthquakeSource::Weapon(source.to_owned()),
            events,
            changed,
            removed,
        )
    }

    pub(super) fn ty_curse_summon(
        &mut self,
        source: &str,
        category: &str,
        level: u16,
        unique: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> usize {
        let level = self.curse_danger_level(level, false);
        let base_category = match category {
            "high-undead" => "undead",
            "high-dragon" => "dragon",
            other => other,
        };
        let mut candidates = self.summon_category_candidate_kind_ids(
            base_category,
            None,
            level.max(1),
            unique,
            false,
        );
        if matches!(category, "high-undead" | "high-dragon") {
            candidates.retain(|id| {
                self.content.actor(id).is_some_and(|actor| match category {
                    "high-undead" => matches!(actor.glyph.as_str(), "L" | "V" | "W"),
                    _ => actor.glyph == "D",
                })
            });
        }
        let positions =
            self.open_positions_around_for_actor_kinds(self.player.position, 3, &candidates);
        let owner = self.player.id.clone();
        let mut resolution = self.resolve_category_summon(
            CategorySummonSpec {
                is_spell: false,
                source_id: source,
                owner_id: &owner,
                category,
                count_dice: 0,
                count_sides: 0,
                count_bonus: 1,
                maximum_count: None,
                hostile: true,
                group_chance_percent: 0,
                group_count_dice: 0,
                group_count_sides: 0,
                group_count_bonus: 1,
                duration_turns: 0,
            },
            candidates,
            positions,
            changed,
        );
        let count = resolution.entity_ids.len();
        if let Some(kind_id) = resolution.summoned_kind_ids.first().cloned()
            && self
                .content
                .actor(&kind_id)
                .is_some_and(|actor| actor.allocation.is_some())
        {
            let policy = rfb_content::GlobalMonsterAllocationDefinition {
                preferred_glyphs: Vec::new(),
                preferred_tags: Vec::new(),
                preferred_movement_modes: Vec::new(),
                preferred_habitats: Vec::new(),
                preferred_damage_immunities: Vec::new(),
                special_div: 64,
                ambient_chance_one_in: 1,
            };
            let mut occupied = self
                .entities
                .iter()
                .filter(|entity| entity.hp > 0)
                .map(|entity| entity.position)
                .chain(std::iter::once(self.player.position))
                .collect();
            let terrain = self.terrain.clone();
            let task = self.current_floor_task_id().map(str::to_owned);
            let members = self.plan_original_group(
                &self.current_floor_id.clone(),
                &policy,
                &kind_id,
                resolution.positions[0],
                level,
                task.as_deref(),
                &terrain,
                self.width,
                self.height,
                &mut occupied,
            );
            for member in members {
                let id = self.summon_entity_id(source, resolution.entity_ids.len());
                let mut entity = self.generated_actor(id.clone(), &member.kind_id, member.position);
                entity.alerted = true;
                self.entities.push(entity);
                changed.insert(member.position);
                resolution.entity_ids.push(id);
                resolution.positions.push(member.position);
                resolution.summoned_kind_ids.push(member.kind_id);
            }
            resolution.group = resolution.entity_ids.len() > 1;
        }
        events.push(DomainEvent::AbilitySummoned {
            ability_id: source.to_owned(),
            resolution,
        });
        count
    }

    fn ty_curse_high_summon(
        &mut self,
        source: &str,
        depth: u16,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> usize {
        let mut count = 0;
        let mut i = 0;
        let mut unique = false;
        // The source rolls the loop bound again on each iteration.
        while i < 1 + self.rng.bounded(7) as u16 + depth / 40 {
            for _ in 0..1000 {
                let roll = 1 + self.rng.bounded(25) as u16 + depth / 20;
                let category = match roll {
                    1 | 2 => "ant",
                    3 | 4 => "spider",
                    5 | 6 => "hound",
                    7 | 8 => "hydra",
                    9 | 10 => "angel",
                    11 | 12 => "undead",
                    13 | 14 => "dragon",
                    15 | 16 => "demon",
                    17 => "amberite",
                    18 | 19 => "unique",
                    20 | 21 => "high-undead",
                    22 | 23 => "high-dragon",
                    24 => "cyber",
                    _ => "any-monster",
                };
                if matches!(roll, 20..=23 | 25..) {
                    unique = true;
                }
                let level = match roll {
                    24 => 100,
                    25.. => depth * 3 / 2 + 5,
                    _ => depth,
                };
                let spawned = self.ty_curse_summon(
                    source,
                    category,
                    level,
                    unique || matches!(roll, 17..=19),
                    events,
                    changed,
                );
                count += spawned;
                if spawned > 0 {
                    break;
                }
            }
            i += 1;
        }
        count
    }

    fn ty_curse_wall_breaker(
        &mut self,
        source: &str,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let single = 1 + self.rng.bounded(80 + u64::from(self.progress.level)) < 70;
        if !single && 1 + self.rng.bounded(100) > 30 {
            return self.ty_curse_earthquake(source, 1, events, changed, removed);
        }
        let rays = if single { 1 } else { self.roll_damage(5, 3) };
        let terrain = self.terrain.clone();
        for _ in 0..rays {
            let mut target = self.player.position;
            for _ in 0..1000 {
                target = self.original_scatter_position(
                    &terrain,
                    self.width,
                    self.height,
                    self.player.position,
                    if single { 4 } else { 10 },
                );
                if target != self.player.position
                    && (!single
                        || self
                            .index(target)
                            .and_then(|i| self.content.terrain(&self.terrain[i]))
                            .is_some_and(|t| !t.blocks_sight))
                {
                    break;
                }
            }
            if let Some(path) = self.targeted_projectile_path_through_target(target, 18) {
                self.resolve_terrain_beam_effect(
                    source,
                    AbilityTerrainBeamOperationDefinition::StoneToMud,
                    path,
                    events,
                    changed,
                    removed,
                )?;
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn ty_curse_blast(
        &mut self,
        source: &str,
        radius: u8,
        damage: i32,
        walls: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let center = self.player.position;
        let cells = self.area_damage_cells(center, radius);
        let positions = cells
            .iter()
            .map(|(_, position)| *position)
            .collect::<Vec<_>>();
        if walls {
            for position in &positions {
                let replacement = self
                    .index(*position)
                    .and_then(|i| self.content.terrain(&self.terrain[i]))
                    .and_then(|t| t.digging.as_ref())
                    .filter(|d| d.resolution != TerrainDiggingResolution::Permanent)
                    .and_then(|d| d.result_terrain_id.clone());
                if let Some(id) = replacement {
                    self.replace_terrain_from_source(
                        *position,
                        &id,
                        TerrainChangeSource::Magic,
                        events,
                        changed,
                    );
                }
            }
        } else {
            self.resolve_projectile_terrain_effects(&positions, DamageType::Mana, changed);
            self.resolve_ground_item_projectile_effects(
                source,
                &positions,
                DamageType::Mana,
                true,
                events,
                changed,
                removed,
            );
        }
        let targets = self
            .entities
            .iter()
            .filter(|e| e.hp > 0)
            .filter_map(|e| {
                cells
                    .iter()
                    .find(|(_, p)| *p == e.position)
                    .map(|(distance, _)| (e.id.clone(), *distance))
            })
            .collect::<Vec<_>>();
        for (id, distance) in targets {
            let Some(index) = self.entities.iter().position(|e| e.id == id && e.hp > 0) else {
                continue;
            };
            let position = self.entities[index].position;
            let trace = ProjectileTrace {
                origin: center,
                impact: position,
                landing: position,
                traversed: vec![position],
            };
            let raw = rfb_area_damage(damage, distance);
            if walls {
                if self.entities[index]
                    .resistances
                    .level(DamageType::Disintegrate)
                    == ResistanceLevel::Vulnerable
                {
                    self.resolve_stone_to_mud_damage_to_entity(
                        index, source, raw, trace, events, changed, removed,
                    )?;
                }
            } else {
                self.resolve_ability_damage_to_entity(
                    index,
                    source,
                    DamageType::Mana,
                    raw,
                    trace,
                    events,
                    changed,
                    removed,
                )?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intrinsic_ty_curse_ticks_once_globally_and_mana_can_kill() {
        let mut game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        game.entities.clear();
        let mut item = game.items[0].clone();
        item.kind_id = "demo.item.chain-mail".to_owned();
        item.location = ItemLocation::Equipped {
            slot_id: "body".to_owned(),
        };
        item.curse = None;
        item.intrinsic_properties
            .rfb_flags
            .insert("TY_CURSE".to_owned());
        item.rolled_affixes = vec![RolledAffixState {
            affix_id: "rfb-legacy.affix.the-demon-lord".to_owned(),
            curse_effects: BTreeSet::from([ItemCurseEffectDto::TyCurse]),
            ..Default::default()
        }];
        game.items = vec![item.clone(), item];
        let seed = (1..100_000)
            .find(|seed| {
                let mut rng = RfbRng::seeded(*seed);
                rng.bounded(200) == 0 && matches!(rng.bounded(34) + 1, 30 | 31)
            })
            .unwrap();
        game.rng = RfbRng::seeded(seed);
        game.player.hp = 1;
        game.world_tick = 9;
        let draws = game.rng_draw_counter();
        game.process_equipped_curse_effects(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
            .unwrap();
        assert_eq!(draws, game.rng_draw_counter());
        game.world_tick = 10;
        let mut events = Vec::new();
        game.process_equipped_curse_effects(&mut events, &mut BTreeSet::new(), &mut Vec::new())
            .unwrap();
        assert!(game.player_is_dead());
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, DomainEvent::ItemLifeLost { fatal: true, .. }))
                .count(),
            1
        );
    }

    #[test]
    fn ty_curse_amnesia_clears_map_and_instance_identification() {
        let mut game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        let seed = (1..10_000)
            .find(|seed| {
                let mut rng = RfbRng::seeded(*seed);
                rng.bounded(34) + 1 == 24 && rng.bounded(6) != 0 && rng.bounded(3) != 0
            })
            .unwrap();
        game.explored.fill(true);
        game.item_property_knowledge.insert(
            game.items[0].id.clone(),
            inventory::ItemPropertyKnowledgeState {
                discovered: true,
                appraised: true,
                identified: true,
                feeling: None,
                known_affix_ids: BTreeSet::from(["rfb-legacy.affix.protection".to_owned()]),
            },
        );
        game.rng = RfbRng::seeded(seed);
        game.resolve_equipped_ty_curse(
            "demo.item.chain-mail",
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert!(game.explored.iter().all(|cell| !cell));
        assert!(
            game.item_property_knowledge
                .values()
                .all(|knowledge| !knowledge.identified
                    && !knowledge.appraised
                    && knowledge.known_affix_ids.is_empty())
        );
    }

    #[test]
    fn ty_curse_free_action_uses_a_save_instead_of_absolute_immunity() {
        let mut game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        let weapon = game
            .items
            .iter_mut()
            .find(|item| matches!(item.location, ItemLocation::Equipped { .. }))
            .unwrap();
        weapon
            .intrinsic_properties
            .status_immunities
            .push(STATUS_PARALYSIS.to_owned());
        weapon
            .intrinsic_properties
            .equipment_bonuses
            .saving_throw_skill = -1000;
        let seed = (1..10_000)
            .find(|seed| {
                let mut rng = RfbRng::seeded(*seed);
                matches!(rng.bounded(34) + 1, 13..=15 | 19 | 20)
            })
            .unwrap();
        game.rng = RfbRng::seeded(seed);
        game.resolve_equipped_ty_curse(
            "demo.item.chain-mail",
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        let paralysis = game
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_PARALYSIS)
            .unwrap();
        assert!((1..=2).contains(&paralysis.remaining_ticks));
    }

    #[test]
    fn high_summoning_creates_hostile_monsters() {
        let mut game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        game.entities.clear();
        game.terrain.fill("demo.terrain.floor".to_owned());
        let count = game.ty_curse_high_summon(
            "demo.item.chain-mail",
            80,
            &mut Vec::new(),
            &mut BTreeSet::new(),
        );
        assert!(count > 0);
        assert!(
            game.entities
                .iter()
                .all(|entity| entity.controller_id.is_none())
        );
        assert_eq!(
            game.entities
                .iter()
                .map(|entity| &entity.id)
                .collect::<BTreeSet<_>>()
                .len(),
            game.entities.len()
        );
    }
}
