// SPDX-License-Identifier: MPL-2.0
use super::*;

impl Game {
    pub(in crate::game) fn hermes_range(&self) -> u16 {
        device_power_value(
            u64::from(self.progress.level / 2 + 10),
            self.effective_player_device_power_bonus(),
        ) as u16
    }

    pub(super) fn resolve_item_monster_summon(
        &mut self,
        source: &str,
        profile: Option<&str>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let count = self.rng.bounded(3) + 1;
        let hostile = self.rng.bounded(10) == 0;
        for _ in 0..count {
            // devices.c retries the pet branch when a hostile summon cannot be placed.
            for hostile in if hostile {
                vec![true, false]
            } else {
                vec![false]
            } {
                let depth = self.floor_depth(&self.current_floor_id);
                let level = depth.saturating_add(if hostile { 5 } else { 0 });
                let candidates = self
                    .summon_category_candidate_kind_ids("any-monster", None, level, false, true)
                    .into_iter()
                    .filter(|id| {
                        hostile
                            || !self
                                .original_pack_spell_flags(self.content.actor(id).unwrap())
                                .1
                    })
                    .collect::<Vec<_>>();
                if candidates.is_empty() {
                    continue;
                }
                let kind = candidates[self.rng.bounded(candidates.len() as u64) as usize].clone();
                let definition = self.content.actor(&kind).unwrap().clone();
                let group = !self.floor_uses_arena_rooms(&self.current_floor_id)
                    && definition
                        .allocation
                        .as_ref()
                        .is_some_and(|a| a.friends.is_some());
                let total = if group {
                    self.original_friend_total(&definition, depth)
                } else {
                    1
                };
                let positions = self
                    .open_positions_around_for_actor_kind(self.player.position, 2, &kind)
                    .into_iter()
                    .take(usize::from(total))
                    .collect();
                let owner = self.player.id.clone();
                let resolution = self.resolve_category_summon(
                    CategorySummonSpec {
                        is_spell: true,
                        source_id: source,
                        owner_id: &owner,
                        category: "any-monster",
                        count_dice: 0,
                        count_sides: 0,
                        count_bonus: 1,
                        maximum_count: None,
                        hostile,
                        group_chance_percent: if group { 100 } else { 0 },
                        group_count_dice: 0,
                        group_count_sides: 0,
                        group_count_bonus: total as u8,
                        duration_turns: 0,
                    },
                    vec![kind],
                    positions,
                    changed,
                );
                let summoned = !resolution.entity_ids.is_empty();
                events.push(DomainEvent::ItemSummoned {
                    source_kind_id: source.into(),
                    profile_id: profile.map(str::to_owned),
                    resolution,
                });
                if summoned {
                    self.mark_item_aware(source);
                    break;
                }
            }
        }
    }

    pub(super) fn resolve_item_starlight(
        &mut self,
        source: &str,
        dice: u16,
        bonus: i32,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let count = self.roll_damage(5, 3);
        let terrain = self.terrain.clone();
        for _ in 0..count {
            let mut target = self.player.position;
            for _ in 0..1000 {
                target = self.original_scatter_position(
                    &terrain,
                    self.width,
                    self.height,
                    self.player.position,
                    4,
                );
                if target != self.player.position
                    && self.index(target).is_some_and(|i| {
                        self.content
                            .terrain(&self.terrain[i])
                            .is_some_and(|terrain| {
                                terrain.walkable
                                    || terrain.tags.iter().any(|tag| tag == "projectable")
                            })
                    })
                {
                    break;
                }
            }
            let damage = self.roll_damage(dice, 10);
            let damage = device_power_value(damage as u64, bonus) as i32;
            if let Some(path) = super::super::projectile_geometry::projectile_path_through_target(
                self.player.position,
                target,
                self.width.max(self.height),
            ) {
                self.resolve_weak_light_line(source, path, damage, events, changed, removed)?;
            }
        }
        self.mark_item_aware(source);
        Ok(())
    }

    pub(super) fn resolve_item_listing(
        &mut self,
        source: &str,
        uniques: bool,
        events: &mut Vec<DomainEvent>,
    ) {
        let entries: Vec<_> = if uniques {
            self.entities
                .iter()
                .rev()
                .filter(|entity| {
                    entity.hp > 0
                        && self
                            .content
                            .actor(&entity.kind_id)
                            .is_some_and(|actor| actor.tags.iter().any(|tag| tag == "unique"))
                })
                .map(|entity| (entity.kind_id.clone(), None))
                .collect()
        } else {
            self.items
                .iter()
                .filter(|item| {
                    item.quantity > 0
                        && matches!(item.location, ItemLocation::Ground(_))
                        && item.is_artifact(&self.content)
                })
                .map(|item| (item.kind_id.clone(), item.artifact_name.clone()))
                .collect()
        };
        if !entries.is_empty() {
            self.mark_item_aware(source);
        }
        for (kind_id, artifact_name) in entries {
            events.push(DomainEvent::ItemListed {
                kind_id,
                artifact_name,
            });
        }
    }

    pub(super) fn resolve_equipment_enchantment(
        &mut self,
        source: &str,
        target: &str,
        events: &mut Vec<DomainEvent>,
    ) {
        let index = self
            .items
            .iter()
            .position(|item| item.id == target)
            .expect("planned enchantment target");
        let item = &self.items[index];
        let total = self.item_total_enchantments(item);
        let definition = self.content.item(&item.kind_id).unwrap();
        let base = match &definition.artifact_generation {
            Some(artifact) => self.content.item(&artifact.base_item_kind_id).unwrap(),
            None => definition,
        };
        let weapon = base
            .rfb_base_kind
            .is_some_and(|kind| (16..=23).contains(&kind.tval));
        let before = item.enchantments;
        let nameless = item.affix_ids.is_empty() && !item.is_artifact(&self.content);
        let mut curse = item.curse;
        let mut component = |before: i16, total: i16, enabled: bool| {
            let gain = if enabled {
                (15_i32 - i32::from(total)).clamp(0, 3) as u16
            } else {
                0
            };
            if gain > 0
                && i32::from(total) + i32::from(gain) >= 0
                && curse == Some(ItemCurseSeverityDto::Normal)
                && self.rng.bounded(100) < 25
            {
                curse = None;
            }
            ItemEnchantmentComponentResolutionDto {
                attempts: gain,
                successes: gain,
                before,
                after: before + gain as i16,
            }
        };
        let to_hit = component(before.to_hit, total.to_hit, weapon);
        let to_damage = component(before.to_damage, total.to_damage, weapon);
        let to_armor = component(before.to_armor, total.to_armor, !weapon);
        let success = to_hit.successes + to_damage.successes + to_armor.successes > 0;
        let item = &mut self.items[index];
        if item.curse.is_some() && curse.is_none() {
            item.intrinsic_properties.rfb_heavy_curse = false;
            item.intrinsic_curse_effects.clear();
            for roll in &mut item.rolled_affixes {
                roll.curse_effects.clear();
                roll.properties.rfb_heavy_curse = false;
            }
        }
        item.curse = curse;
        item.enchantments = ItemEnchantmentsDto {
            to_hit: to_hit.after,
            to_damage: to_damage.after,
            to_armor: to_armor.after,
        };
        if success && nameless {
            item.discount_percent = 99;
        }
        let item_kind_id = item.kind_id.clone();
        if success {
            self.add_virtue(VirtueKindDto::Enchantment, 1);
        } else if self.rng.bounded(3) == 0 && self.virtue_current(VirtueKindDto::Enchantment) < 100
        {
            self.add_virtue(VirtueKindDto::Enchantment, -1);
        }
        self.mark_item_aware(source);
        events.push(DomainEvent::ItemEnchanted {
            source_kind_id: source.to_owned(),
            resolution: ItemEnchantmentResolutionDto {
                item_id: target.to_owned(),
                item_kind_id,
                to_hit,
                to_damage,
                to_armor,
            },
        });
    }

    pub(super) fn resolve_item_sea_summon(
        &mut self,
        source: String,
        profile_id: Option<String>,
        kraken: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let count = self.rng.bounded(3);
        if kraken {
            let origin = self.player.position;
            let mut replacements = Vec::new();
            for y in origin.y - 3..=origin.y + 3 {
                for x in origin.x - 3..=origin.x + 3 {
                    let position = Position { x, y };
                    let distance = rfb_distance(origin, position);
                    let Some(terrain) = self
                        .index(position)
                        .and_then(|i| self.content.terrain(&self.terrain[i]))
                    else {
                        continue;
                    };
                    if distance > 3
                        || terrain.tags.iter().any(|tag| tag == "permanent")
                        || !has_line_of_effect(self, origin, position)
                    {
                        continue;
                    }
                    let damage = (3 + distance) / (distance + 1);
                    if damage > 1 {
                        replacements.push((position, "demo.terrain.surface-water-deep"));
                    } else if terrain.walkable {
                        replacements.push((position, "demo.terrain.surface-water-shallow"));
                    }
                }
            }
            for (position, terrain) in replacements {
                self.replace_terrain_from_source(
                    position,
                    terrain,
                    super::super::terrain::TerrainChangeSource::Magic,
                    events,
                    changed,
                );
            }
            self.mark_item_aware(&source);
        }
        let candidates: Vec<String> = if kraken {
            vec!["demo.actor.lesser-kraken", "demo.actor.greater-kraken"]
        } else {
            vec!["demo.actor.octopus-of-kshitigarbha"]
        }
        .into_iter()
        .map(str::to_owned)
        .collect();
        let owner = self.player.id.clone();
        for _ in 0..count {
            let positions =
                self.open_positions_around_for_actor_kinds(self.player.position, 2, &candidates);
            let group_count = if !kraken
                && !positions.is_empty()
                && !self.equipment_blocks_summoning()
            {
                let definition = self
                    .content
                    .actor(&candidates[0])
                    .expect("source octopus")
                    .clone();
                self.original_friend_total(&definition, self.floor_depth(&self.current_floor_id))
                    as u8
            } else {
                1
            };
            let resolution = self.resolve_category_summon(
                CategorySummonSpec {
                    is_spell: true,
                    source_id: &source,
                    owner_id: &owner,
                    category: if kraken { "kraken" } else { "octopus" },
                    count_dice: 0,
                    count_sides: 0,
                    count_bonus: group_count,
                    maximum_count: None,
                    hostile: false,
                    group_chance_percent: 0,
                    group_count_dice: 0,
                    group_count_sides: 0,
                    group_count_bonus: 0,
                    duration_turns: 0,
                },
                candidates.clone(),
                positions,
                changed,
            );
            if !resolution.entity_ids.is_empty() {
                self.mark_item_aware(&source);
            }
            events.push(DomainEvent::ItemSummoned {
                source_kind_id: source.clone(),
                profile_id: profile_id.clone(),
                resolution,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_artifact_scheduling_never_adds_draws_to_craft() {
        for kind in ["demo.item.long-sword", "demo.item.robe"] {
            let mut game = Game::new_with_build(85, "demo.build.warrior").unwrap();
            game.items.clear();
            game.progress.level = 40;
            crate::game::tests::support::give_inventory_item(&mut game, "test.craft-target", kind);
            game.rng = RfbRng::seeded(85);
            let mut expected_rng = game.rng.clone();
            let names_before = game.random_artifact_names.clone();
            let expected = crate::game::ego::roll_and_materialize_rfb_ego_from_affixes_with_rng(
                false,
                ItemEnchantmentsDto::default(),
                &mut expected_rng,
                game.content.item(kind).unwrap(),
                game.content.affix_definitions(),
                40,
                Some(&game.items[0].intrinsic_properties),
            )
            .unwrap();
            game.resolve_item_crafting(
                "demo.item.crafting-scroll",
                "test.craft-target",
                &mut Vec::new(),
            )
            .unwrap();
            assert_eq!(game.rng, expected_rng);
            assert_eq!(game.items[0].affix_ids, expected.affix_ids);
            assert!(game.items[0].artifact_name.is_none());
            assert_eq!(game.random_artifact_names, names_before);
        }
    }

    #[test]
    fn random_artifact_kraken_water_flow_occurs_even_when_zero_are_summoned() {
        let mut game = Game::new_with_build(85, "demo.build.warrior").unwrap();
        game.entities.clear();
        game.terrain.fill("demo.terrain.floor".to_owned());
        game.player.position = Position { x: 4, y: 4 };
        let seed = (0..100)
            .find(|seed| RfbRng::seeded(*seed).bounded(3) == 0)
            .unwrap();
        game.rng = RfbRng::seeded(seed);
        let mut changed = BTreeSet::new();
        game.resolve_item_sea_summon(
            "demo.item.dagger".to_owned(),
            None,
            true,
            &mut Vec::new(),
            &mut changed,
        );
        assert!(game.entities.is_empty());
        assert!(!changed.is_empty());
        assert_eq!(
            game.terrain_at(game.player.position),
            "demo.terrain.surface-water-deep"
        );
        assert_eq!(
            game.terrain_at(Position { x: 7, y: 4 }),
            "demo.terrain.surface-water-shallow"
        );
    }

    #[test]
    fn random_artifact_sea_summons_create_pets_and_obey_anti_summoning() {
        for blocked in [false, true] {
            let mut game = Game::new_with_build(85, "demo.build.warrior").unwrap();
            game.entities.clear();
            game.items.clear();
            game.terrain.fill("demo.terrain.floor".to_owned());
            game.player.position = Position { x: 4, y: 4 };
            crate::game::tests::support::give_inventory_item(
                &mut game,
                "test.no-summon",
                "demo.item.dagger",
            );
            game.items[0]
                .intrinsic_properties
                .passives
                .insert(rfb_content::EquipmentPassive::AntiSummoning);
            if blocked {
                assert!(game.equip_inventory_item("test.no-summon", None).is_some());
            }
            let seed = (0..100)
                .find(|seed| {
                    let mut rng = RfbRng::seeded(*seed);
                    rng.bounded(3) == 1 && (!blocked || rng.bounded(3) != 0)
                })
                .unwrap();
            game.rng = RfbRng::seeded(seed);
            game.resolve_item_sea_summon(
                "demo.item.dagger".to_owned(),
                None,
                true,
                &mut Vec::new(),
                &mut BTreeSet::new(),
            );
            if blocked {
                assert!(game.entities.is_empty());
            } else {
                assert!(!game.entities.is_empty());
                assert!(
                    game.entities
                        .iter()
                        .all(|e| e.controller_id.as_deref() == Some(&game.player.id))
                );
            }
        }
    }
}
