// SPDX-License-Identifier: MPL-2.0

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ItemUsePlan {
    SelfTarget,
    GlyphGenocide {
        glyph: String,
    },
    CreateAdjacentTerrain {
        replacements: Vec<(Position, String)>,
    },
    DestroyAdjacentTrapsAndDoors {
        replacements: Vec<(Position, String)>,
    },
    VisibleActors {
        actor_ids: Vec<String>,
    },
    Projectile {
        path: Vec<Position>,
    },
    Detect,
    SummonCategory {
        category: String,
        candidate_kind_ids: Vec<String>,
        positions: Vec<Position>,
    },
    Item {
        item_id: String,
    },
    RandomTeleport {
        candidates: Vec<Position>,
    },
    TeleportLevel {
        upward_targets: Vec<FloorTransitionTarget>,
        downward_targets: Vec<FloorTransitionTarget>,
    },
    Recall(RecallUseAction),
    ResetRecall(floor::RecallDestination),
}

pub(super) struct SettledItemUse {
    pub(super) kind_id: String,
    pub(super) profile_id: Option<String>,
    pub(super) effect: ItemUseEffectDefinition,
    pub(super) plan: ItemUsePlan,
}

impl Game {
    pub(super) fn resolve_item_recall(
        &mut self,
        source_kind_id: String,
        effect: ItemUseEffectDefinition,
        plan: ItemUsePlan,
        events: &mut Vec<DomainEvent>,
    ) {
        match (effect, plan) {
            (
                ItemUseEffectDefinition::Recall {
                    delay_dice,
                    delay_sides,
                    delay_bonus,
                },
                ItemUsePlan::Recall(action),
            ) => {
                self.mark_item_aware(&source_kind_id);
                match action {
                    RecallUseAction::Cancel => {
                        self.cancel_recall();
                        events.push(DomainEvent::ItemRecallCancelled { source_kind_id });
                    }
                    RecallUseAction::Start => {
                        let rolled_delay = u16::try_from(self.roll_damage(delay_dice, delay_sides))
                            .expect("validated recall delay roll must fit u16")
                            .saturating_add(delay_bonus);
                        let delay = self.debug_recall_delay_turns.unwrap_or(rolled_delay).max(1);
                        let destination = self.start_recall(delay);
                        events.push(DomainEvent::ItemRecallStarted {
                            source_kind_id,
                            dungeon_id: destination.dungeon_id,
                            floor_id: destination.floor_id,
                            turns: delay,
                        });
                    }
                }
            }
            (ItemUseEffectDefinition::ResetRecall, ItemUsePlan::ResetRecall(destination)) => {
                let dungeon_id = destination.dungeon_id.clone();
                let floor_id = destination.floor_id.clone();
                self.reset_recall(destination);
                self.mark_item_aware(&source_kind_id);
                events.push(DomainEvent::ItemRecallReset {
                    source_kind_id,
                    dungeon_id,
                    floor_id,
                });
            }
            _ => unreachable!("item recall executor requires a matching recall plan"),
        }
    }

    pub(super) fn resolve_item_level_teleport(
        &mut self,
        source_kind_id: String,
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
            .expect("planned floor teleport must remain available");
        self.mark_item_aware(&source_kind_id);
        events.push(DomainEvent::ItemTeleportedLevel {
            source_kind_id,
            from_floor_id,
            to_floor_id: transition.to_floor_id.clone(),
        });
        self.record_floor_transition(transition, events, changed);
        Ok(())
    }

    fn random_teleport_candidates(&self, maximum_distance: u16) -> Vec<Position> {
        let origin = self.player.position;
        let occupied = self
            .entities
            .iter()
            .filter(|entity| entity.hp > 0)
            .map(|entity| entity.position)
            .collect::<BTreeSet<_>>();
        let mut candidates = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let position = Position {
                    x: i32::from(x),
                    y: i32::from(y),
                };
                let distance = chebyshev_distance(origin, position);
                if distance > 0
                    && distance <= u32::from(maximum_distance)
                    && self.is_walkable(position)
                    && !occupied.contains(&position)
                {
                    candidates.push((
                        std::cmp::Reverse(distance),
                        position.y,
                        position.x,
                        position,
                    ));
                }
            }
        }
        candidates.sort_unstable();
        candidates.truncate(candidates.len().div_ceil(2));
        candidates.into_iter().map(|entry| entry.3).collect()
    }

    pub(super) fn resolve_item_random_teleport(
        &mut self,
        source_kind_id: String,
        profile_id: Option<String>,
        candidates: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let candidate_index = usize::try_from(self.rng.bounded(candidates.len() as u64))
            .expect("bounded teleport candidate index must fit usize");
        let destination = candidates[candidate_index];
        let origin = self.player.position;
        self.mark_item_aware(&source_kind_id);
        events.push(DomainEvent::ItemTeleported {
            source_kind_id,
            profile_id,
            resolution: AbilityTeleportResolutionDto {
                from: origin,
                to: destination,
            },
        });
        events.extend(self.relocate_player(destination, changed));
    }

    pub(super) fn item_category_summon_plan(
        &self,
        effect: &ItemUseEffectDefinition,
    ) -> ItemUsePlan {
        let ItemUseEffectDefinition::SummonCategory {
            selector,
            maximum_level_source,
            count_dice,
            count_sides,
            count_bonus,
            group_chance_percent,
            group_count_dice,
            group_count_sides,
            group_count_bonus,
            allow_unique,
            radius,
            ..
        } = effect
        else {
            unreachable!("item summon planning requires a category summon effect");
        };
        let resolved_kin_category = self
            .character_definitions()
            .and_then(|(_, race, _, _)| race.kin_category.as_deref());
        let category = match selector {
            ItemSummonSelectorDefinition::AnyMonster => "any-monster",
            ItemSummonSelectorDefinition::Category { category } => category,
            ItemSummonSelectorDefinition::PlayerKin => {
                resolved_kin_category.unwrap_or("player-kin")
            }
        };
        let maximum_level = match maximum_level_source {
            ItemSummonLevelSourceDefinition::DungeonDepth => {
                self.floor_depth(&self.current_floor_id).max(1)
            }
            ItemSummonLevelSourceDefinition::PlayerLevel => self.progress.level.max(1),
        };
        let candidate_kind_ids = if category == "player-kin" {
            Vec::new()
        } else {
            self.summon_category_candidate_kind_ids(category, None, maximum_level, *allow_unique)
        };
        let normal_maximum =
            usize::from(*count_dice) * usize::from(*count_sides) + usize::from(*count_bonus);
        let group_maximum = if *group_chance_percent == 0 {
            0
        } else {
            usize::from(*group_count_dice) * usize::from(*group_count_sides)
                + usize::from(*group_count_bonus)
        };
        let positions = self
            .open_positions_around(self.player.position, *radius)
            .into_iter()
            .take(normal_maximum.max(group_maximum))
            .collect();
        ItemUsePlan::SummonCategory {
            category: category.to_owned(),
            candidate_kind_ids,
            positions,
        }
    }

    pub(super) fn resolve_item_category_summon(
        &mut self,
        source_kind_id: String,
        profile_id: Option<String>,
        effect: ItemUseEffectDefinition,
        plan: ItemUsePlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let ItemUseEffectDefinition::SummonCategory {
            count_dice,
            count_sides,
            count_bonus,
            hostile,
            group_chance_percent,
            group_count_dice,
            group_count_sides,
            group_count_bonus,
            duration_turns,
            ..
        } = effect
        else {
            unreachable!("item summon executor requires a category summon effect")
        };
        let ItemUsePlan::SummonCategory {
            category,
            candidate_kind_ids,
            positions,
        } = plan
        else {
            unreachable!("item summon executor requires a category summon plan")
        };
        let owner_id = self.player.id.clone();
        let resolution = self.resolve_category_summon(
            CategorySummonSpec {
                source_id: &source_kind_id,
                owner_id: &owner_id,
                category: &category,
                count_dice,
                count_sides,
                count_bonus,
                hostile,
                group_chance_percent,
                group_count_dice,
                group_count_sides,
                group_count_bonus,
                duration_turns,
            },
            candidate_kind_ids,
            positions,
            changed,
        );
        if !resolution.entity_ids.is_empty() {
            self.mark_item_aware(&source_kind_id);
        }
        events.push(DomainEvent::ItemSummoned {
            source_kind_id,
            profile_id,
            resolution,
        });
    }

    fn item_visible_actor_ids(&self) -> Vec<String> {
        self.entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && self.entity_is_visible_to_player(entity)
                    && has_line_of_effect(self, self.player.position, entity.position)
            })
            .map(|entity| entity.id.clone())
            .collect()
    }

    pub(super) fn resolve_item_banish_visible(
        &mut self,
        source_kind_id: &str,
        maximum_distance: u16,
        actor_ids: Vec<String>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        if actor_ids.is_empty() {
            events.push(DomainEvent::ItemBanishmentNoEffect {
                source_kind_id: source_kind_id.to_owned(),
            });
            return;
        }

        let mut noticed = false;
        for actor_id in actor_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == actor_id && entity.hp > 0)
            else {
                continue;
            };
            let definition = self
                .content
                .actor(&self.entities[index].kind_id)
                .expect("item banishment target definition must remain available")
                .clone();
            let guardian = definition.tags.iter().any(|tag| tag == "guardian");
            let teleport_resistance = definition.tags.iter().any(|tag| tag == "resist-teleport");
            let protected_resistance = definition
                .tags
                .iter()
                .any(|tag| matches!(tag.as_str(), "unique" | "resist-all"));
            let resisted = guardian
                || (teleport_resistance
                    && (protected_resistance
                        || definition.level
                            > u32::try_from(self.rng.bounded(100) + 1)
                                .expect("bounded teleport resistance roll must fit u32")));
            if resisted {
                events.push(DomainEvent::ItemBanishmentResisted {
                    source_kind_id: source_kind_id.to_owned(),
                    target_kind_id: definition.id,
                });
                continue;
            }

            noticed = true;
            let destinations = self.item_banishment_destinations(index, maximum_distance);
            if destinations.is_empty() {
                events.push(DomainEvent::ItemBanishmentNoSpace {
                    source_kind_id: source_kind_id.to_owned(),
                    target_kind_id: definition.id,
                });
                continue;
            }
            let choice = usize::try_from(self.rng.bounded(
                u64::try_from(destinations.len()).expect("banishment candidate count must fit u64"),
            ))
            .expect("bounded banishment candidate index must fit usize");
            let from = self.entities[index].position;
            let to = destinations[choice];
            self.entities[index].position = to;
            changed.insert(from);
            changed.insert(to);
            events.push(DomainEvent::ItemBanishedActor {
                source_kind_id: source_kind_id.to_owned(),
                target_kind_id: definition.id,
                resolution: MonsterDisplacementResolutionDto { actor_id, from, to },
            });
        }
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
    }

    fn item_banishment_destinations(
        &self,
        source_index: usize,
        maximum_distance: u16,
    ) -> Vec<Position> {
        let origin = self.entities[source_index].position;
        let mut maximum = u32::from(maximum_distance).min(200);
        let mut minimum = maximum / 2;
        loop {
            let candidates = self.displacement_destinations(source_index, |position| {
                let distance = origin
                    .x
                    .abs_diff(position.x)
                    .max(origin.y.abs_diff(position.y));
                (minimum..=maximum).contains(&distance)
            });
            if !candidates.is_empty() || (maximum == 200 && minimum == 0) {
                return candidates;
            }
            maximum = maximum.saturating_mul(2).min(200);
            minimum /= 2;
        }
    }

    pub(super) fn aggravate_monsters(
        &mut self,
        source_entity_id: Option<&str>,
        source_id: &str,
        changed: &mut BTreeSet<Position>,
    ) -> (u32, u32, Vec<Position>) {
        let mut awakened = 0_u32;
        let mut hastened = 0_u32;
        let mut affected_positions = Vec::new();
        let origin = self.player.position;
        let sight_radius =
            u32::try_from(VISIBILITY_RADIUS).expect("positive visibility radius must fit u32");
        for index in 0..self.entities.len() {
            if self.entities[index].hp <= 0
                || source_entity_id.is_some_and(|source| self.entities[index].id == source)
            {
                continue;
            }
            let position = self.entities[index].position;
            let distance = rfb_distance(origin, position);
            let nearby = distance < sight_radius.saturating_mul(2);
            let hostile_in_los = distance <= sight_radius
                && !self.actor_is_player_side(&self.entities[index])
                && has_line_of_sight(self, origin, position);
            if !nearby && !hostile_in_los {
                continue;
            }
            if nearby {
                self.entities[index].alerted = true;
                let status_count = self.entities[index].statuses.len();
                self.entities[index]
                    .statuses
                    .retain(|status| status.kind_id != STATUS_SLEEP);
                if self.entities[index].statuses.len() < status_count {
                    awakened = awakened.saturating_add(1);
                }
            }
            if hostile_in_los {
                apply_status_application(
                    &mut self.entities[index].statuses,
                    StatusApplication {
                        status: StatusInstance {
                            kind_id: STATUS_HASTE.to_owned(),
                            intensity: 1,
                            remaining_ticks: 100,
                            source_id: Some(source_id.to_owned()),
                            granted_resistances: BTreeMap::new(),
                            granted_brands: BTreeSet::new(),
                            granted_modifiers: StatModifiersDto::default(),
                            granted_equipment_bonuses: EquipmentBonusesDto::default(),
                            granted_status_immunities: BTreeSet::new(),
                            granted_race_id: None,
                            grants_wall_passage: false,
                            incoming_damage_percent: 100,
                        },
                        stacking: StatusStacking::Extend,
                    },
                );
                hastened = hastened.saturating_add(1);
            }
            changed.insert(position);
            affected_positions.push(position);
        }
        (awakened, hastened, affected_positions)
    }

    pub(super) fn resolve_item_aggravation(
        &mut self,
        source_kind_id: &str,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        self.aggravate_monsters(None, source_kind_id, changed);
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemAggravated {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
        });
    }

    pub(super) fn resolve_item_mass_genocide(
        &mut self,
        source_kind_id: &str,
        power: u16,
        radius: u8,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) {
        let mut candidate_ids = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && chebyshev_distance(self.player.position, entity.position)
                        <= u32::from(radius)
            })
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        candidate_ids.sort();
        let resolution = self.resolve_genocide_candidates(
            candidate_ids,
            AbilityGenocideScopeDefinition::Nearby,
            power,
            changed,
            removed_entities,
        );
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemMassGenocide {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            removed_count: resolution.removed_entity_ids.len(),
            resisted_count: resolution.resisted_entity_ids.len(),
            fatigue_damage: resolution.fatigue_damage,
        });
    }

    pub(super) fn resolve_item_genocide(
        &mut self,
        source_kind_id: &str,
        glyph: &str,
        power: u16,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) {
        let mut candidate_ids = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && self
                        .content
                        .actor(&entity.kind_id)
                        .is_some_and(|definition| definition.glyph == glyph)
            })
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        candidate_ids.sort();
        let resolution = self.resolve_genocide_candidates(
            candidate_ids,
            AbilityGenocideScopeDefinition::Glyph,
            power,
            changed,
            removed_entities,
        );
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemGenocide {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            glyph: glyph.to_owned(),
            removed_count: resolution.removed_entity_ids.len(),
            resisted_count: resolution.resisted_entity_ids.len(),
            fatigue_damage: resolution.fatigue_damage,
        });
    }

    fn adjacent_terrain_creation_replacements(
        &self,
        source_terrain_ids: &[String],
        target_terrain_id: &str,
    ) -> Vec<(Position, String)> {
        let occupied = self
            .entities
            .iter()
            .filter(|entity| entity.hp > 0)
            .map(|entity| entity.position)
            .chain(self.items.iter().filter_map(|item| match item.location {
                ItemLocation::Ground(position) => Some(position),
                ItemLocation::Inventory
                | ItemLocation::Equipped { .. }
                | ItemLocation::CarriedBy { .. }
                | ItemLocation::Shop { .. }
                | ItemLocation::Home { .. } => None,
            }))
            .collect::<BTreeSet<_>>();
        let connections = self
            .floor_connections
            .iter()
            .map(|connection| connection.position)
            .collect::<BTreeSet<_>>();
        TERRAIN_INTERACTION_DIRECTIONS
            .iter()
            .filter_map(|direction| {
                let position = self.position_in_direction(*direction);
                let index = self.index(position)?;
                (!occupied.contains(&position)
                    && !connections.contains(&position)
                    && source_terrain_ids.contains(&self.terrain[index]))
                .then(|| (position, target_terrain_id.to_owned()))
            })
            .collect()
    }

    fn adjacent_trap_door_replacements(&self) -> Vec<(Position, String)> {
        TERRAIN_INTERACTION_DIRECTIONS
            .iter()
            .filter_map(|direction| {
                let position = self.position_in_direction(*direction);
                let terrain = self
                    .index(position)
                    .and_then(|index| self.content.terrain(&self.terrain[index]))?;
                let target_terrain_id = if let Some(trap) = &terrain.trap {
                    Some(trap.disarm_to_terrain_id.clone())
                } else if terrain.tags.iter().any(|tag| tag == "door") {
                    terrain.bash_to_terrain_id.clone()
                } else {
                    None
                }?;
                Some((position, target_terrain_id))
            })
            .collect()
    }

    pub(super) fn resolve_item_adjacent_terrain_creation(
        &mut self,
        source_kind_id: &str,
        replacements: Vec<(Position, String)>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let affected_positions = replacements
            .into_iter()
            .map(|(position, target_terrain_id)| {
                let index = self
                    .index(position)
                    .expect("planned terrain creation must remain in bounds");
                self.terrain[index] = target_terrain_id;
                self.revealed_terrain.remove(&position);
                changed.insert(position);
                position
            })
            .collect::<Vec<_>>();
        if !affected_positions.is_empty() {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemCreatedAdjacentTerrain {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            affected_positions,
        });
    }

    pub(super) fn resolve_item_adjacent_trap_door_destruction(
        &mut self,
        source_kind_id: &str,
        replacements: Vec<(Position, String)>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let affected_positions = replacements
            .into_iter()
            .map(|(position, target_terrain_id)| {
                let index = self
                    .index(position)
                    .expect("planned terrain replacement must remain in bounds");
                self.terrain[index] = target_terrain_id;
                self.revealed_terrain.remove(&position);
                changed.insert(position);
                position
            })
            .collect();
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemDestroyedAdjacentTrapsAndDoors {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            affected_positions,
        });
    }

    pub(super) fn resolve_item_detection(
        &mut self,
        source_kind_id: String,
        profile_id: Option<String>,
        effect: ItemUseEffectDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let ItemUseEffectDefinition::Detect {
            subject,
            category,
            radius,
            persistent,
            through_walls,
        } = effect
        else {
            unreachable!("item detection executor requires a detection effect")
        };
        let (detected_positions, detected_entity_ids) = match subject {
            AbilityDetectSubjectDefinition::Terrain => (
                self.detect_terrain_positions(&category, radius, persistent, through_walls),
                Vec::new(),
            ),
            AbilityDetectSubjectDefinition::Actor => self.detect_actor_positions(&category, radius),
            AbilityDetectSubjectDefinition::Item => {
                self.detect_item_positions(&category, radius, through_walls)
            }
        };
        if persistent {
            changed.extend(detected_positions.iter().copied());
        }
        self.mark_item_aware(&source_kind_id);
        let resolution = AbilityDetectResolutionDto {
            subject: ability_detect_subject_dto(subject),
            category,
            radius,
            persistent,
            detected_positions,
            detected_entity_ids,
        };
        if let Some(profile_id) = profile_id {
            events.push(DomainEvent::ItemActivationDetected {
                source_kind_id,
                profile_id,
                resolution,
            });
        } else {
            events.push(DomainEvent::ItemDetected {
                source_kind_id,
                resolution,
            });
        }
    }

    pub(super) fn recharging_item_unavailable_reason(
        &self,
        item_id: &str,
        source_item_id: &str,
        target_item_id: &str,
    ) -> Option<&'static str> {
        if item_id == source_item_id || item_id == target_item_id {
            return Some("recharging-item-is-device");
        }
        if source_item_id == target_item_id {
            return Some("source-is-target");
        }
        if self.recharging_item_power(item_id).is_none() {
            return Some("item-unavailable");
        }
        let source = self.items.iter().find(|item| {
            item.id == source_item_id
                && item.location == ItemLocation::Inventory
                && item.quantity > 0
        });
        if source.is_none_or(|item| !self.item_can_supply_recharge(item)) {
            return Some("source-unavailable");
        }
        let target = self.items.iter().find(|item| {
            item.id == target_item_id
                && item.location == ItemLocation::Inventory
                && item.quantity > 0
        });
        if target.is_none_or(|item| !self.item_can_receive_recharge(item)) {
            return Some("target-not-rechargeable");
        }
        None
    }

    pub(super) fn use_recharging_item(
        &mut self,
        item_id: &str,
        source_item_id: &str,
        target_item_id: &str,
        events: &mut Vec<DomainEvent>,
    ) {
        if self
            .recharging_item_unavailable_reason(item_id, source_item_id, target_item_id)
            .is_some()
        {
            events.push(DomainEvent::ItemUseUnavailable);
            return;
        }
        let power = u32::from(
            self.recharging_item_power(item_id)
                .expect("preflighted recharging item must retain its power"),
        );
        let index = self
            .items
            .iter()
            .position(|item| item.id == item_id)
            .expect("preflighted recharging item must remain available");
        let kind_id = self.items[index].kind_id.clone();
        self.mark_item_tried(&kind_id);
        if self.items[index].quantity == 1 {
            let removed = self.items.remove(index);
            self.item_property_knowledge.remove(&removed.id);
        } else {
            self.items[index].quantity -= 1;
        }
        let outcome = self.recharge_inventory_item_from_device(
            target_item_id,
            source_item_id,
            DeviceRechargeRequest::new(power, RECHARGING_ITEM_SOURCE_DESTRUCTION_ONE_IN),
        );
        events.push(device_recharge_resolved_event(
            outcome.target,
            outcome.source_kind_id,
            true,
            outcome.source_destroyed,
        ));
        self.mark_item_aware(&kind_id);
    }

    fn recharging_item_power(&self, item_id: &str) -> Option<u16> {
        let item = self.items.iter().find(|item| {
            item.id == item_id
                && item.location == ItemLocation::Inventory
                && item.quantity > 0
                && item.activation.is_none()
        })?;
        let action = self.content.item(&item.kind_id)?.use_action.as_ref()?;
        match action.effect {
            ItemUseEffectDefinition::RechargeFromDevice { power } => Some(power),
            _ => None,
        }
    }

    pub(super) fn resolve_item_curse(
        &mut self,
        source_kind_id: &str,
        target: ItemCurseTargetDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let target = match target {
            ItemCurseTargetDefinition::Weapon => EquippedItemCurseTarget::Weapon,
            ItemCurseTargetDefinition::Armor => EquippedItemCurseTarget::Armor,
        };
        let outcome = self.curse_equipped_item(CurseEquippedItemRequest::new(target));
        if outcome.item_id.is_some() {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemCursed {
            source_kind_id: source_kind_id.to_owned(),
            resolution: ItemCurseResolutionDto {
                item_id: outcome.item_id,
                item_kind_id: outcome.item_kind_id,
                before: outcome.before,
                after: outcome.after,
                resisted: outcome.resisted,
            },
        });
    }

    pub(super) fn resolve_item_curse_removal(
        &mut self,
        source_kind_id: &str,
        include_heavy: bool,
        events: &mut Vec<DomainEvent>,
    ) {
        let outcome = self.remove_equipped_curses(RemoveEquippedCursesRequest::new(include_heavy));
        if include_heavy || !outcome.removed_item_ids.is_empty() {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemCursesRemoved {
            source_kind_id: source_kind_id.to_owned(),
            resolution: ItemCurseRemovalResolutionDto {
                include_heavy: outcome.include_heavy,
                removed_item_ids: outcome.removed_item_ids,
                retained_permanent_item_ids: outcome.retained_permanent_item_ids,
            },
        });
    }

    fn item_is_valid_enchant_target(
        &self,
        source_item_id: &str,
        target_item_id: &str,
        effect: &ItemUseEffectDefinition,
    ) -> bool {
        if source_item_id == target_item_id {
            return false;
        }
        let ItemUseEffectDefinition::EnchantItem {
            to_hit,
            to_damage,
            to_armor,
        } = effect
        else {
            return false;
        };
        self.items.iter().any(|item| {
            if item.id != target_item_id
                || item.quantity == 0
                || !matches!(
                    &item.location,
                    ItemLocation::Inventory | ItemLocation::Equipped { .. }
                ) && !matches!(
                    &item.location,
                    ItemLocation::Ground(position) if *position == self.player.position
                )
            {
                return false;
            }
            let Some(definition) = self.content.item(&item.kind_id) else {
                return false;
            };
            if definition.tags.iter().any(|tag| tag == "no-enchant") {
                return false;
            }
            if to_armor.is_some() {
                definition.tags.iter().any(|tag| tag == "armor")
            } else if to_hit.is_some() || to_damage.is_some() {
                definition
                    .tags
                    .iter()
                    .any(|tag| matches!(tag.as_str(), "weapon" | "launcher" | "ammunition"))
            } else {
                false
            }
        })
    }

    pub(super) fn resolve_item_enchantment(
        &mut self,
        source_kind_id: &str,
        target_item_id: &str,
        to_hit: Option<ItemEnchantmentRollDefinition>,
        to_damage: Option<ItemEnchantmentRollDefinition>,
        to_armor: Option<ItemEnchantmentRollDefinition>,
        events: &mut Vec<DomainEvent>,
    ) {
        self.mark_item_aware(source_kind_id);
        let hit_attempts = self.roll_item_enchantment_attempts(to_hit);
        let damage_attempts = self.roll_item_enchantment_attempts(to_damage);
        let armor_attempts = self.roll_item_enchantment_attempts(to_armor);
        let outcome = self.enchant_item_instance(
            target_item_id,
            ItemEnchantmentRequest::new(hit_attempts, damage_attempts, armor_attempts),
        );
        events.push(DomainEvent::ItemEnchanted {
            source_kind_id: source_kind_id.to_owned(),
            resolution: ItemEnchantmentResolutionDto {
                item_id: outcome.item_id,
                item_kind_id: outcome.item_kind_id,
                to_hit: ItemEnchantmentComponentResolutionDto {
                    attempts: outcome.to_hit.attempts,
                    successes: outcome.to_hit.successes,
                    before: outcome.to_hit.before,
                    after: outcome.to_hit.after,
                },
                to_damage: ItemEnchantmentComponentResolutionDto {
                    attempts: outcome.to_damage.attempts,
                    successes: outcome.to_damage.successes,
                    before: outcome.to_damage.before,
                    after: outcome.to_damage.after,
                },
                to_armor: ItemEnchantmentComponentResolutionDto {
                    attempts: outcome.to_armor.attempts,
                    successes: outcome.to_armor.successes,
                    before: outcome.to_armor.before,
                    after: outcome.to_armor.after,
                },
            },
        });
    }

    fn roll_item_enchantment_attempts(
        &mut self,
        roll: Option<ItemEnchantmentRollDefinition>,
    ) -> u16 {
        let Some(roll) = roll else {
            return 0;
        };
        let rolled = if roll.dice == 0 {
            0
        } else {
            u16::try_from(self.roll_damage(roll.dice, roll.sides))
                .expect("validated enchantment roll must fit u16")
        };
        rolled.saturating_add(roll.bonus)
    }

    fn item_is_valid_identify_target(&self, source_item_id: &str, target_item_id: &str) -> bool {
        source_item_id != target_item_id
            && self.items.iter().any(|item| {
                item.id == target_item_id
                    && item.quantity > 0
                    && match &item.location {
                        ItemLocation::Inventory | ItemLocation::Equipped { .. } => true,
                        ItemLocation::Ground(position) => *position == self.player.position,
                        ItemLocation::CarriedBy { .. }
                        | ItemLocation::Shop { .. }
                        | ItemLocation::Home { .. } => false,
                    }
            })
    }

    pub(super) fn resolve_item_identification(
        &mut self,
        source_kind_id: &str,
        target_item_id: &str,
        full: bool,
        events: &mut Vec<DomainEvent>,
    ) {
        self.mark_item_aware(source_kind_id);
        let outcome =
            self.identify_item_instance(target_item_id, ItemIdentificationRequest::new(full));
        events.push(DomainEvent::ItemIdentified {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            resolution: ItemIdentifyResolutionDto {
                item_id: outcome.item_id,
                item_kind_id: outcome.item_kind_id,
                full: outcome.full,
                changed: outcome.changed,
            },
        });
    }

    pub(super) fn item_attribute_kind(attribute: &ItemAttributeDefinition) -> AttributeKind {
        match attribute {
            ItemAttributeDefinition::Strength => AttributeKind::Strength,
            ItemAttributeDefinition::Intelligence => AttributeKind::Intelligence,
            ItemAttributeDefinition::Wisdom => AttributeKind::Wisdom,
            ItemAttributeDefinition::Dexterity => AttributeKind::Dexterity,
            ItemAttributeDefinition::Constitution => AttributeKind::Constitution,
            ItemAttributeDefinition::Charisma => AttributeKind::Charisma,
        }
    }

    pub(super) fn resolve_item_drain_attribute(
        &mut self,
        source_kind_id: &str,
        attribute: AttributeKind,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        if self
            .player_equipment_passives()
            .contains(&attribute_sustain_passive(attribute))
        {
            let value = self.progress.attributes.value(attribute);
            self.mark_item_aware(source_kind_id);
            events.push(DomainEvent::ItemAttributeChanged {
                source_kind_id: source_kind_id.to_owned(),
                display_name_key: self.item_display_name_key(source_kind_id),
                attribute,
                change: ItemAttributeChange::Sustained,
                before: value,
                after: value,
                maximum: self.progress.maximum_attributes.value(attribute),
                noticed: true,
            });
            return true;
        }
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let outcome = apply_attribute_drain(&mut self.progress, attribute, &mut self.rng);
        let noticed = outcome.changed;
        if noticed {
            self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemAttributeChanged {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            attribute: outcome.attribute,
            change: ItemAttributeChange::Drained,
            before: outcome.before,
            after: outcome.after,
            maximum: outcome.maximum_after,
            noticed,
        });
        noticed
    }

    pub(super) fn resolve_item_restore_attribute(
        &mut self,
        source_kind_id: &str,
        attribute: AttributeKind,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let outcome = apply_attribute_restoration(&mut self.progress, attribute);
        let noticed = outcome.changed;
        if noticed {
            self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemAttributeChanged {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            attribute: outcome.attribute,
            change: ItemAttributeChange::Restored,
            before: outcome.before,
            after: outcome.after,
            maximum: outcome.maximum_after,
            noticed,
        });
        noticed
    }

    pub(super) fn resolve_item_increase_attributes(
        &mut self,
        source_kind_id: &str,
        attributes: &[AttributeKind],
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let victorious = self.victory_level_cap_unlocked();
        let mut noticed = false;
        let mut resolutions = Vec::with_capacity(attributes.len());

        for &attribute in attributes {
            let outcome = apply_permanent_attribute_increase(
                &mut self.progress,
                attribute,
                victorious,
                &mut self.rng,
            );
            let change = if outcome.maximum_after > outcome.maximum_before {
                ItemAttributeChange::Increased
            } else if outcome.after > outcome.before {
                ItemAttributeChange::Restored
            } else {
                ItemAttributeChange::Increased
            };
            resolutions.push((outcome, change));
            noticed = outcome.changed || noticed;
        }

        if noticed {
            self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
            self.mark_item_aware(source_kind_id);
        }
        let display_name_key = self.item_display_name_key(source_kind_id);
        for (outcome, change) in resolutions {
            events.push(DomainEvent::ItemAttributeChanged {
                source_kind_id: source_kind_id.to_owned(),
                display_name_key: display_name_key.clone(),
                attribute: outcome.attribute,
                change,
                before: outcome.before,
                after: outcome.after,
                maximum: outcome.maximum_after,
                noticed: outcome.changed,
            });
        }
        noticed
    }

    pub(super) fn use_inventory_item(
        &mut self,
        item_id: &str,
        target: Option<&TargetSelection>,
        target_glyph: Option<&str>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let Some((index, definition)) = self.inventory_item_use_context(item_id)? else {
            events.push(DomainEvent::ItemUseUnavailable);
            return Ok(());
        };
        let kind_id = self.items[index].kind_id.clone();
        let activation = self.items[index].activation.clone();
        let (profile_id, difficulty, cost, effect, plan) =
            if let Some(activation) = activation.as_ref() {
                let profile = definition
                    .device_generation
                    .as_ref()
                    .and_then(|generation| {
                        generation
                            .activations
                            .iter()
                            .find(|candidate| candidate.id == activation.profile_id)
                    })
                    .expect("validated dynamic item activation profile must remain available");
                let Some(plan) = self.item_use_plan(
                    item_id,
                    &profile.effect,
                    Some(&profile.target),
                    target,
                    target_glyph,
                ) else {
                    events.push(DomainEvent::ItemUseUnavailable);
                    return Ok(());
                };
                (
                    Some(activation.profile_id.clone()),
                    Some(activation.device_check_difficulty),
                    Some(activation.cost),
                    profile.effect.clone(),
                    plan,
                )
            } else if let Some(action) = definition.use_action {
                let Some(plan) =
                    self.item_use_plan(item_id, &action.effect, None, target, target_glyph)
                else {
                    events.push(DomainEvent::ItemUseUnavailable);
                    return Ok(());
                };
                (
                    None,
                    action.device_check_difficulty,
                    action.charges.map(|charges| charges.cost),
                    action.effect,
                    plan,
                )
            } else {
                events.push(DomainEvent::ItemUseUnavailable);
                return Ok(());
            };
        if cost.is_some_and(|cost| {
            self.items[index]
                .charges
                .is_none_or(|state| state.current < cost)
        }) {
            events.push(DomainEvent::ItemUseUnavailable);
            return Ok(());
        }

        self.mark_item_tried(&kind_id);
        if let Some(difficulty) = difficulty {
            let ability = self.player_derived_stats().device_skill;
            let mut difficulty_pipeline = DerivedStatsPipeline::new();
            difficulty_pipeline.add(
                StatKind::ActionDifficulty,
                StatLayer::Environment,
                &kind_id,
                difficulty,
            );
            let check = resolve_check(
                &mut self.rng,
                CheckContext {
                    kind: CheckKind::UseDevice,
                    actor_id: self.player.id.clone(),
                    target_id: Some(item_id.to_owned()),
                    ability,
                    difficulty: difficulty_pipeline
                        .resolve(StatKind::ActionDifficulty, StatBounds::NON_NEGATIVE),
                },
            );
            let succeeded = check.succeeded();
            let skill_id = self
                .content
                .skill_by_kind(SkillKind::Device)
                .expect("validated device skill must remain available")
                .id
                .clone();
            events.push(DomainEvent::DeviceSkillChecked {
                source_kind_id: kind_id.clone(),
                succeeded,
                resolution: check.to_dto(skill_id),
            });
            if !succeeded {
                return Ok(());
            }
        }

        if let Some(cost) = cost {
            self.items[index]
                .charges
                .as_mut()
                .expect("validated charged item must carry charge state")
                .current -= cost;
        } else if self.items[index].quantity == 1 {
            let removed = self.items.remove(index);
            self.item_property_knowledge.remove(&removed.id);
        } else {
            self.items[index].quantity -= 1;
        }
        self.resolve_inventory_item_effect(
            SettledItemUse {
                kind_id,
                profile_id,
                effect,
                plan,
            },
            events,
            changed,
            removed_entities,
        )
    }

    fn resolve_inventory_item_effect(
        &mut self,
        settled: SettledItemUse,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let SettledItemUse {
            kind_id,
            profile_id,
            effect,
            plan,
        } = settled;
        match (effect, plan) {
            (
                effect @ (ItemUseEffectDefinition::Heal { .. }
                | ItemUseEffectDefinition::NoNumericEffect
                | ItemUseEffectDefinition::IncreaseNutrition { .. }
                | ItemUseEffectDefinition::SatisfyHunger
                | ItemUseEffectDefinition::HealDice { .. }
                | ItemUseEffectDefinition::Bless { .. }
                | ItemUseEffectDefinition::ApplySlowness { .. }
                | ItemUseEffectDefinition::ApplySpeed { .. }
                | ItemUseEffectDefinition::ApplyHeroism { .. }
                | ItemUseEffectDefinition::ApplyBerserkStrength { .. }
                | ItemUseEffectDefinition::ApplyPoeticInspiration { .. }
                | ItemUseEffectDefinition::ApplyStoneSkin { .. }
                | ItemUseEffectDefinition::RestoreLifeLevels { .. }
                | ItemUseEffectDefinition::RestoreAllAttributes
                | ItemUseEffectDefinition::RestoreAllVitality { .. }
                | ItemUseEffectDefinition::ApplyRestorativeFeast { .. }
                | ItemUseEffectDefinition::ApplyLifeRestoration { .. }
                | ItemUseEffectDefinition::ApplyThermalResistance { .. }
                | ItemUseEffectDefinition::ApplyBasicResistance { .. }
                | ItemUseEffectDefinition::ApplyPoison { .. }
                | ItemUseEffectDefinition::ApplyBlindness { .. }
                | ItemUseEffectDefinition::ApplyStatus { .. }
                | ItemUseEffectDefinition::ApplyGiantStrength { .. }
                | ItemUseEffectDefinition::SelfDamage { .. }
                | ItemUseEffectDefinition::LoseExperienceFraction { .. }
                | ItemUseEffectDefinition::DrainAttribute { .. }
                | ItemUseEffectDefinition::RestoreAttribute { .. }
                | ItemUseEffectDefinition::IncreaseAttribute { .. }
                | ItemUseEffectDefinition::AugmentAttributes
                | ItemUseEffectDefinition::Vengeance { .. }
                | ItemUseEffectDefinition::ProtectionFromEvil
                | ItemUseEffectDefinition::PrepareConfusingStrike
                | ItemUseEffectDefinition::IncreaseSpellLearningCapacity
                | ItemUseEffectDefinition::RemoveStatus { .. }
                | ItemUseEffectDefinition::ReduceStatus { .. }
                | ItemUseEffectDefinition::RestoreResource { .. }
                | ItemUseEffectDefinition::RestoreResourceDice { .. }
                | ItemUseEffectDefinition::RestoreResourceFull { .. }
                | ItemUseEffectDefinition::DrainResourceFull { .. }
                | ItemUseEffectDefinition::Sequence { .. }),
                ItemUsePlan::SelfTarget,
            ) => {
                self.resolve_item_self_effect(&kind_id, &effect, events);
            }
            (
                ItemUseEffectDefinition::SelfCenteredElementalBlast {
                    base_damage,
                    damage_type,
                    radius,
                    backlash_sides,
                    backlash_bonus,
                    backlash_damage_type,
                    backlash_uses_resistance,
                },
                ItemUsePlan::SelfTarget,
            ) => {
                self.resolve_item_elemental_blast(
                    &kind_id,
                    base_damage,
                    damage_type.into(),
                    radius,
                    backlash_sides,
                    backlash_bonus,
                    backlash_damage_type.into(),
                    backlash_uses_resistance,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                ItemUseEffectDefinition::ApplyDetonation {
                    damage_dice,
                    damage_sides,
                    stun_ticks,
                    bleeding_ticks,
                },
                ItemUsePlan::SelfTarget,
            ) => self.resolve_item_detonation(
                &kind_id,
                damage_dice,
                damage_sides,
                stun_ticks,
                bleeding_ticks,
                events,
            ),
            (ItemUseEffectDefinition::SelfLifeLoss { amount }, ItemUsePlan::SelfTarget) => {
                self.resolve_item_life_loss(&kind_id, amount, events);
            }
            (ItemUseEffectDefinition::AggravateMonsters, ItemUsePlan::SelfTarget) => {
                self.resolve_item_aggravation(&kind_id, events, changed);
            }
            (ItemUseEffectDefinition::MassGenocide { power, radius }, ItemUsePlan::SelfTarget) => {
                self.resolve_item_mass_genocide(
                    &kind_id,
                    power,
                    radius,
                    events,
                    changed,
                    removed_entities,
                );
            }
            (ItemUseEffectDefinition::Genocide { power }, ItemUsePlan::GlyphGenocide { glyph }) => {
                self.resolve_item_genocide(
                    &kind_id,
                    &glyph,
                    power,
                    events,
                    changed,
                    removed_entities,
                );
            }
            (
                ItemUseEffectDefinition::CreateAdjacentTerrain { .. },
                ItemUsePlan::CreateAdjacentTerrain { replacements },
            ) => {
                self.resolve_item_adjacent_terrain_creation(
                    &kind_id,
                    replacements,
                    events,
                    changed,
                );
            }
            (
                ItemUseEffectDefinition::DestroyAdjacentTrapsAndDoors,
                ItemUsePlan::DestroyAdjacentTrapsAndDoors { replacements },
            ) => {
                self.resolve_item_adjacent_trap_door_destruction(
                    &kind_id,
                    replacements,
                    events,
                    changed,
                );
            }
            (
                effect @ ItemUseEffectDefinition::Damage { .. },
                plan @ ItemUsePlan::Projectile { .. },
            ) => self.resolve_item_activation_damage(
                kind_id,
                profile_id,
                effect,
                plan,
                events,
                changed,
                removed_entities,
            )?,
            (
                ItemUseEffectDefinition::DispelCategory { category, damage },
                ItemUsePlan::VisibleActors { actor_ids },
            ) => {
                self.resolve_item_dispel_category(
                    &kind_id,
                    &category,
                    damage,
                    actor_ids,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                ItemUseEffectDefinition::BanishVisible { maximum_distance },
                ItemUsePlan::VisibleActors { actor_ids },
            ) => {
                self.resolve_item_banish_visible(
                    &kind_id,
                    maximum_distance,
                    actor_ids,
                    events,
                    changed,
                );
            }
            (effect @ ItemUseEffectDefinition::Detect { .. }, ItemUsePlan::Detect) => {
                self.resolve_item_detection(kind_id, profile_id, effect, events, changed);
            }
            (
                effect @ ItemUseEffectDefinition::SummonCategory { .. },
                plan @ ItemUsePlan::SummonCategory { .. },
            ) => self
                .resolve_item_category_summon(kind_id, profile_id, effect, plan, events, changed),
            (ItemUseEffectDefinition::IdentifyItem { full }, ItemUsePlan::Item { item_id }) => {
                self.resolve_item_identification(&kind_id, &item_id, full, events);
            }
            (
                ItemUseEffectDefinition::EnchantItem {
                    to_hit,
                    to_damage,
                    to_armor,
                },
                ItemUsePlan::Item { item_id },
            ) => {
                self.resolve_item_enchantment(
                    &kind_id, &item_id, to_hit, to_damage, to_armor, events,
                );
            }
            (ItemUseEffectDefinition::CurseEquippedItem { target }, ItemUsePlan::SelfTarget) => {
                self.resolve_item_curse(&kind_id, target, events);
            }
            (
                ItemUseEffectDefinition::RemoveEquippedCurses { include_heavy },
                ItemUsePlan::SelfTarget,
            ) => {
                self.resolve_item_curse_removal(&kind_id, include_heavy, events);
            }
            (
                ItemUseEffectDefinition::RandomTeleport { .. },
                ItemUsePlan::RandomTeleport { candidates },
            ) => {
                self.resolve_item_random_teleport(kind_id, profile_id, candidates, events, changed);
            }
            (
                ItemUseEffectDefinition::TeleportLevel,
                ItemUsePlan::TeleportLevel {
                    upward_targets,
                    downward_targets,
                },
            ) => {
                self.resolve_item_level_teleport(
                    kind_id,
                    upward_targets,
                    downward_targets,
                    events,
                    changed,
                )?;
            }
            (
                effect @ (ItemUseEffectDefinition::Recall { .. }
                | ItemUseEffectDefinition::ResetRecall),
                plan @ (ItemUsePlan::Recall(_) | ItemUsePlan::ResetRecall(_)),
            ) => self.resolve_item_recall(kind_id, effect, plan, events),
            _ => unreachable!("validated item effect and target plan must remain compatible"),
        }
        Ok(())
    }

    pub(super) fn item_use_plan(
        &self,
        source_item_id: &str,
        effect: &ItemUseEffectDefinition,
        target_definition: Option<&AbilityTargetDefinition>,
        target: Option<&TargetSelection>,
        target_glyph: Option<&str>,
    ) -> Option<ItemUsePlan> {
        if target_glyph.is_some() && !matches!(effect, ItemUseEffectDefinition::Genocide { .. }) {
            return None;
        }
        let self_target = target.is_none_or(|target| matches!(target, TargetSelection::SelfTarget));
        match effect {
            ItemUseEffectDefinition::NoNumericEffect
            | ItemUseEffectDefinition::IncreaseNutrition { .. }
            | ItemUseEffectDefinition::SatisfyHunger
            | ItemUseEffectDefinition::Heal { .. }
            | ItemUseEffectDefinition::HealDice { .. }
            | ItemUseEffectDefinition::Bless { .. }
            | ItemUseEffectDefinition::ApplySlowness { .. }
            | ItemUseEffectDefinition::ApplySpeed { .. }
            | ItemUseEffectDefinition::ApplyHeroism { .. }
            | ItemUseEffectDefinition::ApplyBerserkStrength { .. }
            | ItemUseEffectDefinition::ApplyPoeticInspiration { .. }
            | ItemUseEffectDefinition::ApplyStoneSkin { .. }
            | ItemUseEffectDefinition::RestoreLifeLevels { .. }
            | ItemUseEffectDefinition::RestoreAllAttributes
            | ItemUseEffectDefinition::RestoreAllVitality { .. }
            | ItemUseEffectDefinition::ApplyRestorativeFeast { .. }
            | ItemUseEffectDefinition::ApplyLifeRestoration { .. }
            | ItemUseEffectDefinition::DrainAttribute { .. }
            | ItemUseEffectDefinition::RestoreAttribute { .. }
            | ItemUseEffectDefinition::IncreaseAttribute { .. }
            | ItemUseEffectDefinition::AugmentAttributes
            | ItemUseEffectDefinition::ApplyThermalResistance { .. }
            | ItemUseEffectDefinition::ApplyBasicResistance { .. }
            | ItemUseEffectDefinition::ApplyPoison { .. }
            | ItemUseEffectDefinition::ApplyBlindness { .. }
            | ItemUseEffectDefinition::ApplyStatus { .. }
            | ItemUseEffectDefinition::ApplyGiantStrength { .. }
            | ItemUseEffectDefinition::ApplyDetonation { .. }
            | ItemUseEffectDefinition::SelfLifeLoss { .. }
            | ItemUseEffectDefinition::SelfDamage { .. }
            | ItemUseEffectDefinition::LoseExperienceFraction { .. }
            | ItemUseEffectDefinition::Vengeance { .. }
            | ItemUseEffectDefinition::ProtectionFromEvil
            | ItemUseEffectDefinition::PrepareConfusingStrike
            | ItemUseEffectDefinition::IncreaseSpellLearningCapacity
            | ItemUseEffectDefinition::SelfCenteredElementalBlast { .. }
            | ItemUseEffectDefinition::AggravateMonsters
            | ItemUseEffectDefinition::MassGenocide { .. }
            | ItemUseEffectDefinition::RemoveStatus { .. }
            | ItemUseEffectDefinition::ReduceStatus { .. }
            | ItemUseEffectDefinition::RestoreResource { .. }
            | ItemUseEffectDefinition::RestoreResourceDice { .. }
            | ItemUseEffectDefinition::RestoreResourceFull { .. }
            | ItemUseEffectDefinition::DrainResourceFull { .. }
            | ItemUseEffectDefinition::Sequence { .. }
            | ItemUseEffectDefinition::CurseEquippedItem { .. }
            | ItemUseEffectDefinition::RemoveEquippedCurses { .. } => {
                self_target.then_some(ItemUsePlan::SelfTarget)
            }
            ItemUseEffectDefinition::Genocide { .. } => {
                if target.is_some() {
                    return None;
                }
                let glyph = target_glyph?;
                let mut characters = glyph.chars();
                let character = characters.next()?;
                (!character.is_control() && characters.next().is_none()).then(|| {
                    ItemUsePlan::GlyphGenocide {
                        glyph: glyph.to_owned(),
                    }
                })
            }
            ItemUseEffectDefinition::RechargeFromDevice { .. } => None,
            ItemUseEffectDefinition::CreateAdjacentTerrain {
                source_terrain_ids,
                target_terrain_id,
            } => self_target.then(|| ItemUsePlan::CreateAdjacentTerrain {
                replacements: self
                    .adjacent_terrain_creation_replacements(source_terrain_ids, target_terrain_id),
            }),
            ItemUseEffectDefinition::DestroyAdjacentTrapsAndDoors => {
                self_target.then(|| ItemUsePlan::DestroyAdjacentTrapsAndDoors {
                    replacements: self.adjacent_trap_door_replacements(),
                })
            }
            ItemUseEffectDefinition::Damage { .. } => {
                let path = target_definition.and_then(|definition| {
                    target.and_then(|target| self.item_effect_path(definition, target))
                })?;
                Some(ItemUsePlan::Projectile { path })
            }
            ItemUseEffectDefinition::DispelCategory { .. }
            | ItemUseEffectDefinition::BanishVisible { .. } => {
                self_target.then(|| ItemUsePlan::VisibleActors {
                    actor_ids: self.item_visible_actor_ids(),
                })
            }
            ItemUseEffectDefinition::Detect { .. } => self_target.then_some(ItemUsePlan::Detect),
            effect @ ItemUseEffectDefinition::SummonCategory { .. } => {
                self_target.then(|| self.item_category_summon_plan(effect))
            }
            ItemUseEffectDefinition::IdentifyItem { .. } => {
                let TargetSelection::Item {
                    item_id: target_item_id,
                } = target?
                else {
                    return None;
                };
                self.item_is_valid_identify_target(source_item_id, target_item_id)
                    .then(|| ItemUsePlan::Item {
                        item_id: target_item_id.clone(),
                    })
            }
            effect @ ItemUseEffectDefinition::EnchantItem { .. } => {
                let TargetSelection::Item {
                    item_id: target_item_id,
                } = target?
                else {
                    return None;
                };
                self.item_is_valid_enchant_target(source_item_id, target_item_id, effect)
                    .then(|| ItemUsePlan::Item {
                        item_id: target_item_id.clone(),
                    })
            }
            ItemUseEffectDefinition::RandomTeleport { maximum_distance } => {
                if !self_target {
                    return None;
                }
                let candidates = self.random_teleport_candidates(*maximum_distance);
                (!candidates.is_empty()).then_some(ItemUsePlan::RandomTeleport { candidates })
            }
            ItemUseEffectDefinition::TeleportLevel => {
                if !self_target {
                    return None;
                }
                let (upward_targets, downward_targets) = self.teleport_level_targets();
                (!upward_targets.is_empty() || !downward_targets.is_empty()).then_some(
                    ItemUsePlan::TeleportLevel {
                        upward_targets,
                        downward_targets,
                    },
                )
            }
            ItemUseEffectDefinition::Recall { .. } => self_target
                .then(|| self.recall_use_plan())
                .flatten()
                .map(ItemUsePlan::Recall),
            ItemUseEffectDefinition::ResetRecall => self_target
                .then(|| self.recall_reset_plan())
                .flatten()
                .map(ItemUsePlan::ResetRecall),
        }
    }
}

impl Game {
    pub(super) fn resolve_item_restorative_resource_effect(
        &mut self,
        source_kind_id: &str,
        effect: &ItemUseEffectDefinition,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        match effect {
            ItemUseEffectDefinition::Heal { amount } => {
                let amount = i32::try_from(*amount).expect("validated healing amount must fit i32");
                self.resolve_item_healing(source_kind_id, amount, events)
            }
            ItemUseEffectDefinition::HealDice { dice, sides } => {
                let amount = self.roll_damage(*dice, *sides);
                self.resolve_item_healing(source_kind_id, amount, events)
            }
            ItemUseEffectDefinition::RestoreResource {
                resource_id,
                amount,
            } => self.resolve_item_resource_restoration(
                source_kind_id,
                ResourceRestorationRequest::amount(resource_id, *amount),
                events,
            ),
            ItemUseEffectDefinition::RestoreResourceDice {
                resource_id,
                dice,
                sides,
                bonus,
            } => {
                let rolled = u32::try_from(self.roll_damage(*dice, *sides))
                    .expect("validated resource restoration roll must fit u32")
                    .saturating_add(*bonus);
                self.resolve_item_resource_restoration(
                    source_kind_id,
                    ResourceRestorationRequest::amount(resource_id, rolled),
                    events,
                )
            }
            ItemUseEffectDefinition::RestoreResourceFull { resource_id } => self
                .resolve_item_resource_restoration(
                    source_kind_id,
                    ResourceRestorationRequest::full(resource_id),
                    events,
                ),
            _ => {
                unreachable!("restorative resource executor requires healing or resource recovery")
            }
        }
    }

    pub(super) fn resolve_item_vitality_restoration_effect(
        &mut self,
        source_kind_id: &str,
        effect: &ItemUseEffectDefinition,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        match effect {
            ItemUseEffectDefinition::RestoreLifeLevels { life_force_amount } => {
                self.resolve_item_restore_life_levels(source_kind_id, *life_force_amount, events)
            }
            ItemUseEffectDefinition::RestoreAllAttributes => {
                let noticed = self.restore_all_player_attributes();
                if noticed {
                    self.mark_item_aware(source_kind_id);
                }
                events.push(DomainEvent::ItemRestorationResolved {
                    source_kind_id: source_kind_id.to_owned(),
                    display_name_key: self.item_display_name_key(source_kind_id),
                    noticed,
                });
                noticed
            }
            ItemUseEffectDefinition::RestoreAllVitality { life_force_amount } => {
                let attributes_restored = self.restore_all_player_attributes();
                let vitality_restored =
                    self.restore_player_experience_and_life_force(*life_force_amount, events);
                let noticed = attributes_restored || vitality_restored;
                if noticed {
                    self.mark_item_aware(source_kind_id);
                }
                events.push(DomainEvent::ItemRestorationResolved {
                    source_kind_id: source_kind_id.to_owned(),
                    display_name_key: self.item_display_name_key(source_kind_id),
                    noticed,
                });
                noticed
            }
            ItemUseEffectDefinition::ApplyRestorativeFeast {
                healing_dice,
                healing_sides,
            } => {
                if let Some(index) = self
                    .player
                    .statuses
                    .iter()
                    .position(|status| status.kind_id == STATUS_POISON)
                {
                    let before = self.player.statuses[index].remaining_ticks;
                    let reduction = (before / 5).max(100);
                    let after = before.saturating_sub(reduction);
                    if after == 0 {
                        self.player.statuses.remove(index);
                    } else {
                        self.player.statuses[index].remaining_ticks = after;
                    }
                }
                let healing = self.roll_damage(*healing_dice, *healing_sides);
                let max_hp = self.effective_player_max_hp();
                let player = &mut self.player;
                apply_effect(
                    &mut EffectTarget {
                        hp: &mut player.hp,
                        max_hp,
                        resistances: &player.resistances,
                        statuses: &mut player.statuses,
                    },
                    EffectSpec::Heal { amount: healing },
                );
                self.restore_all_player_attributes();
                self.restore_player_experience_and_life_force(0, events);
                self.mark_item_aware(source_kind_id);
                events.push(DomainEvent::ItemRestorationResolved {
                    source_kind_id: source_kind_id.to_owned(),
                    display_name_key: self.item_display_name_key(source_kind_id),
                    noticed: true,
                });
                true
            }
            ItemUseEffectDefinition::ApplyLifeRestoration {
                healing_amount,
                life_force_amount,
            } => {
                self.restore_player_experience_and_life_force(*life_force_amount, events);
                self.player.statuses.retain(|status| {
                    !matches!(
                        status.kind_id.as_str(),
                        STATUS_POISON
                            | STATUS_BLINDNESS
                            | STATUS_CONFUSION
                            | STATUS_STUN
                            | STATUS_BLEEDING
                            | STATUS_SLOW
                            | "rfb.status.berserk"
                    )
                });
                self.restore_all_player_attributes();
                let amount = i32::try_from(*healing_amount)
                    .expect("validated life restoration amount must fit i32");
                let max_hp = self.effective_player_max_hp();
                let player = &mut self.player;
                apply_effect(
                    &mut EffectTarget {
                        hp: &mut player.hp,
                        max_hp,
                        resistances: &player.resistances,
                        statuses: &mut player.statuses,
                    },
                    EffectSpec::Heal { amount },
                );
                self.mark_item_aware(source_kind_id);
                events.push(DomainEvent::ItemRestorationResolved {
                    source_kind_id: source_kind_id.to_owned(),
                    display_name_key: self.item_display_name_key(source_kind_id),
                    noticed: true,
                });
                true
            }
            _ => unreachable!("vitality restoration executor requires a restoration effect"),
        }
    }

    fn resolve_item_restore_life_levels(
        &mut self,
        source_kind_id: &str,
        life_force_amount: u16,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let noticed = self.restore_player_experience_and_life_force(life_force_amount, events);
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemRestoreLifeLevelsResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            noticed,
        });
        noticed
    }

    fn restore_all_player_attributes(&mut self) -> bool {
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let mut restored = false;
        for attribute in [
            AttributeKind::Strength,
            AttributeKind::Intelligence,
            AttributeKind::Wisdom,
            AttributeKind::Dexterity,
            AttributeKind::Constitution,
            AttributeKind::Charisma,
        ] {
            restored =
                apply_attribute_restoration(&mut self.progress, attribute).changed || restored;
        }
        if restored {
            self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        }
        restored
    }

    fn restore_player_experience_and_life_force(
        &mut self,
        life_force_amount: u16,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let experience = apply_experience_restoration(&mut self.progress);
        self.apply_player_experience(0, events);
        let life_force = apply_life_force_restoration(
            &mut self.progress,
            LifeForceRestorationRequest::add(life_force_amount),
        );
        experience.after != experience.before || life_force.after != life_force.before
    }

    pub(super) fn resolve_item_healing(
        &mut self,
        source_kind_id: &str,
        amount: i32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let max_hp = self.effective_player_max_hp();
        let outcome = apply_healing(&mut self.player.hp, max_hp, HealingRequest::amount(amount));
        let requested = outcome.requested;
        let applied = outcome.applied;
        if applied > 0 {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemUsed {
            display_name_key: self.item_display_name_key(source_kind_id),
            source_kind_id: source_kind_id.to_owned(),
            requested,
            applied,
        });
        applied > 0
    }

    fn resolve_item_resource_restoration(
        &mut self,
        source_kind_id: &str,
        request: ResourceRestorationRequest<'_>,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let outcome =
            apply_resource_restoration(&mut self.resources, &mut self.resources_touched, request);
        if outcome.recovered > 0 {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemResourceRestored {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            resolution: ResourceRecoveryResolutionDto {
                resource_id: outcome.resource_id,
                before: outcome.before,
                after: outcome.after,
                recovered: outcome.recovered,
            },
        });
        outcome.recovered > 0
    }

    pub(super) fn resolve_item_confusing_strike_preparation(
        &mut self,
        source_kind_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        self.confusing_strike_ready = true;
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemConfusingStrikePrepared {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
        });
        true
    }

    pub(super) fn resolve_item_status_removal(
        &mut self,
        source_kind_id: &str,
        status_kind_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let outcome = apply_status_removal(
            &mut self.player.statuses,
            StatusRemovalRequest::new(status_kind_id),
        );
        let removed = outcome.removed;
        if removed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemStatusRemoved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            status_kind_id: outcome.kind_id,
            removed,
        });
        removed
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_item_status(
        &mut self,
        source_kind_id: &str,
        status_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        stacking: AbilityStatusStackingDefinition,
        resistance_type: Option<rfb_content::ActorDamageType>,
        granted_resistances: &BTreeMap<
            rfb_content::ActorDamageType,
            rfb_content::ActorResistanceLevel,
        >,
        granted_modifiers: &StatModifiers,
        granted_equipment_bonuses: &EquipmentBonuses,
        incoming_damage_percent: u8,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let immunity = self.player_status_immunities().contains(status_kind_id);
        let resistance_threshold = resistance_type.map_or(0, |damage_type| {
            u64::try_from(
                self.effective_player_resistances()
                    .level(damage_type.into())
                    .reduction_percent()
                    .max(0),
            )
            .expect("status resistance threshold must be non-negative")
        });
        let resisted =
            immunity || (resistance_type.is_some() && self.rng.bounded(55) < resistance_threshold);
        let (duration, noticed) = if resisted {
            (None, false)
        } else {
            let duration_sides =
                u16::try_from(duration_sides).expect("validated status die sides must fit u16");
            let source_turns = u32::try_from(self.roll_damage(duration_dice, duration_sides))
                .expect("validated status duration must fit u32")
                .saturating_add(duration_bonus);
            // RFB timed food and potion effects count standard player turns. The
            // Rewrite scheduler advances ten world ticks per standard-speed action;
            // one extra action window is consumed immediately after item resolution.
            let duration = source_turns.saturating_add(1).saturating_mul(10);
            let stacking = match stacking {
                AbilityStatusStackingDefinition::Replace => StatusStacking::Replace,
                AbilityStatusStackingDefinition::Extend => StatusStacking::Extend,
                AbilityStatusStackingDefinition::KeepStrongest => StatusStacking::KeepStrongest,
            };
            let change = apply_status_application(
                &mut self.player.statuses,
                StatusApplication {
                    status: StatusInstance {
                        kind_id: status_kind_id.to_owned(),
                        intensity: 1,
                        remaining_ticks: duration,
                        source_id: Some(source_kind_id.to_owned()),
                        granted_resistances: granted_resistances
                            .iter()
                            .map(|(damage_type, level)| ((*damage_type).into(), (*level).into()))
                            .collect(),
                        granted_brands: BTreeSet::new(),
                        granted_modifiers: stat_modifiers_dto(granted_modifiers),
                        granted_equipment_bonuses: equipment_bonuses_dto(granted_equipment_bonuses),
                        granted_status_immunities: BTreeSet::new(),
                        granted_race_id: None,
                        grants_wall_passage: false,
                        incoming_damage_percent,
                    },
                    stacking,
                },
            )
            .change;
            (Some(duration), !matches!(change, StatusChange::Unchanged))
        };
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemStatusResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            status_kind_id: status_kind_id.to_owned(),
            duration,
            noticed,
        });
        noticed
    }

    fn resolve_item_giant_strength(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let duration_sides =
            u16::try_from(duration_sides).expect("validated status die sides must fit u16");
        let source_turns = u32::try_from(self.roll_damage(duration_dice, duration_sides))
            .expect("validated status duration must fit u32")
            .saturating_add(duration_bonus);
        let duration = source_turns.saturating_add(1).saturating_mul(10);
        let level = i32::from(self.progress.level);
        let change = apply_status_application(
            &mut self.player.statuses,
            StatusApplication {
                status: StatusInstance {
                    kind_id: STATUS_GIANT_STRENGTH.to_owned(),
                    intensity: 1,
                    remaining_ticks: duration,
                    source_id: Some(source_kind_id.to_owned()),
                    granted_resistances: BTreeMap::new(),
                    granted_brands: BTreeSet::new(),
                    granted_modifiers: StatModifiersDto {
                        max_hp: 10 + level / 2,
                        ..StatModifiersDto::default()
                    },
                    granted_equipment_bonuses: EquipmentBonusesDto {
                        melee_skill: 60 * level / 50,
                        ..EquipmentBonusesDto::default()
                    },
                    granted_status_immunities: BTreeSet::new(),
                    granted_race_id: None,
                    grants_wall_passage: false,
                    incoming_damage_percent: 100,
                },
                stacking: StatusStacking::Extend,
            },
        )
        .change;
        let noticed = !matches!(change, StatusChange::Unchanged);
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemStatusResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            status_kind_id: STATUS_GIANT_STRENGTH.to_owned(),
            duration: Some(duration),
            noticed,
        });
        noticed
    }

    fn resolve_item_experience_loss(
        &mut self,
        source_kind_id: &str,
        divisor: u8,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let amount = self.progress.experience / u64::from(divisor);
        self.progress.experience = self.progress.experience.saturating_sub(amount);
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemExperienceLost {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            amount,
            remaining: self.progress.experience,
        });
        true
    }

    fn resolve_item_status_reduction(
        &mut self,
        source_kind_id: &str,
        status_kind_id: &str,
        minimum_reduction: u32,
        reduction_divisor: u8,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let Some(index) = self
            .player
            .statuses
            .iter()
            .position(|status| status.kind_id == status_kind_id)
        else {
            events.push(DomainEvent::ItemStatusReduced {
                source_kind_id: source_kind_id.to_owned(),
                display_name_key: self.item_display_name_key(source_kind_id),
                status_kind_id: status_kind_id.to_owned(),
                before: 0,
                after: 0,
            });
            return false;
        };
        let before = self.player.statuses[index].remaining_ticks;
        let reduction = (before / u32::from(reduction_divisor)).max(minimum_reduction);
        let after = before.saturating_sub(reduction);
        if after == 0 {
            self.player.statuses.remove(index);
        } else {
            self.player.statuses[index].remaining_ticks = after;
        }
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemStatusReduced {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            status_kind_id: status_kind_id.to_owned(),
            before,
            after,
        });
        true
    }

    pub(super) fn resolve_item_resource_drain(
        &mut self,
        source_kind_id: &str,
        resource_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let drained = self.resources.get_mut(resource_id).map_or(0, |pool| {
            let drained = pool.current;
            pool.current = 0;
            drained
        });
        if drained > 0 {
            self.resources_touched.insert(resource_id.to_owned());
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemResourceDrained {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            resource_id: resource_id.to_owned(),
            drained,
        });
        drained > 0
    }

    pub(super) fn resolve_item_protection_from_evil(
        &mut self,
        source_kind_id: &str,
        events: &mut Vec<DomainEvent>,
    ) {
        let duration = u32::from(self.progress.level)
            .saturating_mul(3)
            .saturating_add(
                u32::try_from(self.roll_damage(1, 25))
                    .expect("protection from evil duration must fit u32"),
            );
        let resolution = apply_ability_status_effect(
            &mut self.player,
            source_kind_id,
            0,
            STATUS_PROTECTION_FROM_EVIL,
            1,
            duration,
            0,
            1,
            AbilityStatusStackingDefinition::Extend,
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
        );
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemProtectionFromEvil {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![resolution],
            },
        });
    }

    pub(super) fn resolve_item_blessing(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) {
        let resolution = apply_ability_status_effect(
            &mut self.player,
            source_kind_id,
            0,
            "rfb.status.blessed",
            1,
            duration_bonus,
            duration_dice,
            duration_sides,
            AbilityStatusStackingDefinition::Extend,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers {
                defense: 5,
                ..StatModifiers::default()
            },
            &EquipmentBonuses {
                melee_skill: 10,
                ranged_skill: 10,
                ..EquipmentBonuses::default()
            },
            &BTreeSet::new(),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        let duration = match &resolution {
            AbilityEffectResolutionDto::ApplyStatus {
                applied_duration_ticks,
                ..
            } => *applied_duration_ticks,
            _ => unreachable!("blessing must produce a status application resolution"),
        };
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemBlessed {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![resolution],
            },
        });
    }

    pub(super) fn resolve_item_slowness(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let duration_sides =
            u16::try_from(duration_sides).expect("validated slowness die sides must fit u16");
        let duration = u32::try_from(self.roll_damage(duration_dice, duration_sides))
            .expect("validated slowness duration must fit u32")
            .saturating_add(duration_bonus);
        let change = if self.player_status_immunities().contains(STATUS_SLOW) {
            StatusChange::Unchanged
        } else {
            apply_status_application(
                &mut self.player.statuses,
                StatusApplication {
                    status: StatusInstance {
                        kind_id: STATUS_SLOW.to_owned(),
                        intensity: 1,
                        remaining_ticks: duration,
                        source_id: Some(source_kind_id.to_owned()),
                        granted_resistances: BTreeMap::new(),
                        granted_brands: BTreeSet::new(),
                        granted_modifiers: StatModifiersDto::default(),
                        granted_equipment_bonuses: EquipmentBonusesDto::default(),
                        granted_status_immunities: BTreeSet::new(),
                        granted_race_id: None,
                        grants_wall_passage: false,
                        incoming_damage_percent: 100,
                    },
                    stacking: StatusStacking::KeepStrongest,
                },
            )
            .change
        };
        let noticed = matches!(change, StatusChange::Added);
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemSlownessResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            noticed,
        });
        noticed
    }

    pub(super) fn resolve_item_speed(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let already_hasted = self
            .player
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_HASTE);
        let duration = if already_hasted {
            5
        } else {
            let duration_sides =
                u16::try_from(duration_sides).expect("validated speed die sides must fit u16");
            u32::try_from(self.roll_damage(duration_dice, duration_sides))
                .expect("validated speed duration must fit u32")
                .saturating_add(duration_bonus)
        };
        let change = apply_status_application(
            &mut self.player.statuses,
            StatusApplication {
                status: StatusInstance {
                    kind_id: STATUS_HASTE.to_owned(),
                    intensity: 1,
                    remaining_ticks: duration,
                    source_id: Some(source_kind_id.to_owned()),
                    granted_resistances: BTreeMap::new(),
                    granted_brands: BTreeSet::new(),
                    granted_modifiers: StatModifiersDto::default(),
                    granted_equipment_bonuses: EquipmentBonusesDto::default(),
                    granted_status_immunities: BTreeSet::new(),
                    granted_race_id: None,
                    grants_wall_passage: false,
                    incoming_damage_percent: 100,
                },
                stacking: StatusStacking::Extend,
            },
        )
        .change;
        let noticed = matches!(change, StatusChange::Added);
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemSpeedResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
        });
        noticed
    }

    pub(super) fn resolve_item_heroism(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let resolution = apply_ability_status_effect(
            &mut self.player,
            source_kind_id,
            0,
            "rfb.status.hero",
            1,
            duration_bonus,
            duration_dice,
            duration_sides,
            AbilityStatusStackingDefinition::Extend,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers {
                max_hp: 10,
                ..StatModifiers::default()
            },
            &EquipmentBonuses {
                melee_skill: 12,
                ranged_skill: 12,
                ..EquipmentBonuses::default()
            },
            &BTreeSet::from([STATUS_FEAR.to_owned()]),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        let (duration, noticed) = match resolution {
            AbilityEffectResolutionDto::ApplyStatus {
                applied_duration_ticks,
                change,
                ..
            } => (
                applied_duration_ticks,
                matches!(change, AbilityStatusChangeDto::Added),
            ),
            _ => unreachable!("heroism must produce a status application resolution"),
        };
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemHeroismResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            noticed,
        });
        noticed
    }

    pub(super) fn resolve_item_berserk_strength(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let resolution = apply_ability_status_effect(
            &mut self.player,
            source_kind_id,
            0,
            "rfb.status.berserk",
            1,
            duration_bonus,
            duration_dice,
            duration_sides,
            AbilityStatusStackingDefinition::Extend,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers {
                defense: -10,
                max_hp: 30,
                ..StatModifiers::default()
            },
            &EquipmentBonuses {
                melee_skill: 12,
                melee_damage: 3 + i32::from(self.progress.level / 5),
                ranged_skill: -12,
                throwing_skill: -20,
                device_skill: -20,
                saving_throw_skill: -30,
                stealth_skill: -7,
                search_skill: -15,
                perception_skill: -15,
                digging_skill: 30,
                ..EquipmentBonuses::default()
            },
            &BTreeSet::from([STATUS_FEAR.to_owned()]),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        let (duration, status_noticed) = match resolution {
            AbilityEffectResolutionDto::ApplyStatus {
                applied_duration_ticks,
                change,
                ..
            } => (
                applied_duration_ticks,
                matches!(change, AbilityStatusChangeDto::Added),
            ),
            _ => unreachable!("berserk strength must produce a status application resolution"),
        };
        if status_noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemBerserkStrengthResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            noticed: status_noticed,
        });
        let healed = self.resolve_item_healing(source_kind_id, 30, events);
        status_noticed || healed
    }

    pub(super) fn resolve_item_poetic_inspiration(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let resolution = apply_ability_status_effect(
            &mut self.player,
            source_kind_id,
            0,
            "rfb.status.poetic-inspiration",
            1,
            duration_bonus,
            duration_dice,
            duration_sides,
            AbilityStatusStackingDefinition::Extend,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers {
                wisdom: 5,
                charisma: 5,
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
        let (duration, noticed) = match resolution {
            AbilityEffectResolutionDto::ApplyStatus {
                applied_duration_ticks,
                change,
                ..
            } => (
                applied_duration_ticks,
                matches!(change, AbilityStatusChangeDto::Added),
            ),
            _ => unreachable!("poetic inspiration must produce a status application resolution"),
        };
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemPoeticInspirationResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            noticed,
        });
        noticed
    }

    pub(super) fn resolve_item_stone_skin(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let defense = 10 + 40 * i32::from(self.progress.level) / 50;
        let resolution = apply_ability_status_effect(
            &mut self.player,
            source_kind_id,
            0,
            "rfb.status.stone-skin",
            1,
            duration_bonus,
            duration_dice,
            duration_sides,
            AbilityStatusStackingDefinition::KeepStrongest,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers {
                defense,
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
        let (duration, noticed) = match resolution {
            AbilityEffectResolutionDto::ApplyStatus {
                applied_duration_ticks,
                change,
                ..
            } => (
                applied_duration_ticks,
                matches!(change, AbilityStatusChangeDto::Added),
            ),
            _ => unreachable!("stone skin must produce a status application resolution"),
        };
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemStoneSkinResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            noticed,
        });
        noticed
    }

    pub(super) fn resolve_item_thermal_resistance(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let duration_sides =
            u16::try_from(duration_sides).expect("validated thermal die sides must fit u16");
        let duration = u32::try_from(self.roll_damage(duration_dice, duration_sides))
            .expect("validated thermal duration must fit u32")
            .saturating_add(duration_bonus);
        let change = apply_status_application(
            &mut self.player.statuses,
            StatusApplication {
                status: StatusInstance {
                    kind_id: STATUS_THERMAL_RESISTANCE.to_owned(),
                    intensity: 1,
                    remaining_ticks: duration,
                    source_id: Some(source_kind_id.to_owned()),
                    granted_resistances: BTreeMap::from([
                        (DamageType::Fire, ResistanceLevel::Resistant),
                        (DamageType::Cold, ResistanceLevel::Resistant),
                    ]),
                    granted_brands: BTreeSet::new(),
                    granted_modifiers: StatModifiersDto::default(),
                    granted_equipment_bonuses: EquipmentBonusesDto::default(),
                    granted_status_immunities: BTreeSet::new(),
                    granted_race_id: None,
                    grants_wall_passage: false,
                    incoming_damage_percent: 100,
                },
                stacking: StatusStacking::Extend,
            },
        )
        .change;
        let noticed = matches!(change, StatusChange::Added);
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemThermalResistanceResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            noticed,
        });
        noticed
    }

    pub(super) fn resolve_item_basic_resistance(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) {
        let duration_sides =
            u16::try_from(duration_sides).expect("validated resistance die sides must fit u16");
        let duration = u32::try_from(self.roll_damage(duration_dice, duration_sides))
            .expect("validated resistance duration must fit u32")
            .saturating_add(duration_bonus);
        apply_status_application(
            &mut self.player.statuses,
            StatusApplication {
                status: StatusInstance {
                    kind_id: STATUS_BASIC_RESISTANCE.to_owned(),
                    intensity: 1,
                    remaining_ticks: duration,
                    source_id: Some(source_kind_id.to_owned()),
                    granted_resistances: BTreeMap::from([
                        (DamageType::Acid, ResistanceLevel::Resistant),
                        (DamageType::Electricity, ResistanceLevel::Resistant),
                        (DamageType::Fire, ResistanceLevel::Resistant),
                        (DamageType::Cold, ResistanceLevel::Resistant),
                        (DamageType::Poison, ResistanceLevel::Resistant),
                    ]),
                    granted_brands: BTreeSet::new(),
                    granted_modifiers: StatModifiersDto::default(),
                    granted_equipment_bonuses: EquipmentBonusesDto::default(),
                    granted_status_immunities: BTreeSet::new(),
                    granted_race_id: None,
                    grants_wall_passage: false,
                    incoming_damage_percent: 100,
                },
                stacking: StatusStacking::KeepStrongest,
            },
        );
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemBasicResistanceApplied {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
        });
    }

    pub(super) fn resolve_item_poison(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let resistance = self
            .effective_player_resistances()
            .level(DamageType::Poison);
        let resistance_threshold = u64::try_from(resistance.reduction_percent().max(0))
            .expect("threshold is non-negative");
        let resisted = self.rng.bounded(55) < resistance_threshold;
        let duration = if resisted {
            None
        } else {
            let duration_sides =
                u16::try_from(duration_sides).expect("validated poison die sides must fit u16");
            let duration = u32::try_from(self.roll_damage(duration_dice, duration_sides))
                .expect("validated poison duration must fit u32")
                .saturating_add(duration_bonus);
            apply_status_application(
                &mut self.player.statuses,
                StatusApplication {
                    status: StatusInstance {
                        kind_id: STATUS_POISON.to_owned(),
                        intensity: 1,
                        remaining_ticks: duration,
                        source_id: Some(source_kind_id.to_owned()),
                        granted_resistances: BTreeMap::new(),
                        granted_brands: BTreeSet::new(),
                        granted_modifiers: StatModifiersDto::default(),
                        granted_equipment_bonuses: EquipmentBonusesDto::default(),
                        granted_status_immunities: BTreeSet::new(),
                        granted_race_id: None,
                        grants_wall_passage: false,
                        incoming_damage_percent: 100,
                    },
                    stacking: StatusStacking::Extend,
                },
            );
            self.mark_item_aware(source_kind_id);
            Some(duration)
        };
        events.push(DomainEvent::ItemPoisonResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
        });
        !resisted
    }

    pub(super) fn resolve_item_blindness(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let resistance_threshold = if self.player_status_immunities().contains(STATUS_BLINDNESS) {
            55
        } else {
            0
        };
        let resisted = self.rng.bounded(55) < resistance_threshold;
        let (duration, noticed) = if resisted {
            (None, false)
        } else {
            let duration_sides =
                u16::try_from(duration_sides).expect("validated blindness die sides must fit u16");
            let duration = u32::try_from(self.roll_damage(duration_dice, duration_sides))
                .expect("validated blindness duration must fit u32")
                .saturating_add(duration_bonus);
            let change = apply_status_application(
                &mut self.player.statuses,
                StatusApplication {
                    status: StatusInstance {
                        kind_id: STATUS_BLINDNESS.to_owned(),
                        intensity: 1,
                        remaining_ticks: duration,
                        source_id: Some(source_kind_id.to_owned()),
                        granted_resistances: BTreeMap::new(),
                        granted_brands: BTreeSet::new(),
                        granted_modifiers: StatModifiersDto::default(),
                        granted_equipment_bonuses: EquipmentBonusesDto::default(),
                        granted_status_immunities: BTreeSet::new(),
                        granted_race_id: None,
                        grants_wall_passage: false,
                        incoming_damage_percent: 100,
                    },
                    stacking: StatusStacking::Extend,
                },
            )
            .change;
            let noticed = matches!(change, StatusChange::Added);
            if noticed {
                self.mark_item_aware(source_kind_id);
            }
            (Some(duration), noticed)
        };
        events.push(DomainEvent::ItemBlindnessResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            noticed,
        });
        noticed
    }

    pub(super) fn resolve_item_vengeance(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) {
        let resolution = apply_ability_status_effect(
            &mut self.player,
            source_kind_id,
            0,
            STATUS_VENGEANCE,
            1,
            duration_bonus,
            duration_dice,
            duration_sides,
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
        );
        let duration = match &resolution {
            AbilityEffectResolutionDto::ApplyStatus {
                applied_duration_ticks,
                ..
            } => *applied_duration_ticks,
            _ => unreachable!("vengeance must produce a status application resolution"),
        };
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemVengeanceActivated {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![resolution],
            },
        });
    }

    pub(super) fn resolve_item_spell_learning_capacity(
        &mut self,
        source_kind_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let before = self
            .casting_profile()
            .map_or(0, |profile| self.ability_learning_capacity(profile));
        let eligible = self.uses_spell_scrolls();
        let bonus =
            apply_learning_capacity_increase(&mut self.bonus_spell_learning_capacity, eligible);
        debug_assert_eq!(bonus.after, self.bonus_spell_learning_capacity);
        debug_assert!(bonus.after >= bonus.before);
        let after = self
            .casting_profile()
            .map_or(0, |profile| self.ability_learning_capacity(profile));
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemSpellLearningCapacityChanged {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            before,
            after,
        });
        true
    }

    pub(super) fn resolve_item_self_effect(
        &mut self,
        source_kind_id: &str,
        effect: &ItemUseEffectDefinition,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        match effect {
            ItemUseEffectDefinition::NoNumericEffect => {
                self.mark_item_aware(source_kind_id);
                events.push(DomainEvent::ItemUsed {
                    source_kind_id: source_kind_id.to_owned(),
                    display_name_key: self.item_display_name_key(source_kind_id),
                    requested: 0,
                    applied: 0,
                });
                true
            }
            ItemUseEffectDefinition::IncreaseNutrition { amount } => {
                let before_state = self.nutrition_state();
                let applied = self.increase_nutrition(*amount);
                self.mark_item_aware(source_kind_id);
                events.push(DomainEvent::ItemNutritionIncreased {
                    source_kind_id: source_kind_id.to_owned(),
                    display_name_key: self.item_display_name_key(source_kind_id),
                    amount: applied,
                    nutrition: self.nutrition,
                });
                let after_state = self.nutrition_state();
                if after_state != before_state {
                    events.push(DomainEvent::NutritionStateChanged {
                        from: before_state,
                        to: after_state,
                        nutrition: self.nutrition,
                    });
                }
                true
            }
            ItemUseEffectDefinition::SatisfyHunger => {
                let before_state = self.nutrition_state();
                let target = rfb_protocol::PLAYER_NUTRITION_MAXIMUM - 1;
                let noticed = self.nutrition != target;
                self.nutrition = target;
                self.mark_item_aware(source_kind_id);
                events.push(DomainEvent::ItemNutritionSatisfied {
                    source_kind_id: source_kind_id.to_owned(),
                    display_name_key: self.item_display_name_key(source_kind_id),
                    nutrition: self.nutrition,
                    noticed,
                });
                let after_state = self.nutrition_state();
                if after_state != before_state {
                    events.push(DomainEvent::NutritionStateChanged {
                        from: before_state,
                        to: after_state,
                        nutrition: self.nutrition,
                    });
                }
                noticed
            }
            effect @ (ItemUseEffectDefinition::Heal { .. }
            | ItemUseEffectDefinition::HealDice { .. }
            | ItemUseEffectDefinition::RestoreResource { .. }
            | ItemUseEffectDefinition::RestoreResourceDice { .. }
            | ItemUseEffectDefinition::RestoreResourceFull { .. }) => {
                self.resolve_item_restorative_resource_effect(source_kind_id, effect, events)
            }
            ItemUseEffectDefinition::Bless {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => {
                self.resolve_item_blessing(
                    source_kind_id,
                    *duration_dice,
                    *duration_sides,
                    *duration_bonus,
                    events,
                );
                true
            }
            ItemUseEffectDefinition::ApplySlowness {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => self.resolve_item_slowness(
                source_kind_id,
                *duration_dice,
                *duration_sides,
                *duration_bonus,
                events,
            ),
            ItemUseEffectDefinition::ApplySpeed {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => self.resolve_item_speed(
                source_kind_id,
                *duration_dice,
                *duration_sides,
                *duration_bonus,
                events,
            ),
            ItemUseEffectDefinition::ApplyHeroism {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => self.resolve_item_heroism(
                source_kind_id,
                *duration_dice,
                *duration_sides,
                *duration_bonus,
                events,
            ),
            ItemUseEffectDefinition::ApplyBerserkStrength {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => self.resolve_item_berserk_strength(
                source_kind_id,
                *duration_dice,
                *duration_sides,
                *duration_bonus,
                events,
            ),
            ItemUseEffectDefinition::ApplyPoeticInspiration {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => self.resolve_item_poetic_inspiration(
                source_kind_id,
                *duration_dice,
                *duration_sides,
                *duration_bonus,
                events,
            ),
            ItemUseEffectDefinition::ApplyStoneSkin {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => self.resolve_item_stone_skin(
                source_kind_id,
                *duration_dice,
                *duration_sides,
                *duration_bonus,
                events,
            ),
            effect @ (ItemUseEffectDefinition::RestoreLifeLevels { .. }
            | ItemUseEffectDefinition::RestoreAllAttributes
            | ItemUseEffectDefinition::RestoreAllVitality { .. }
            | ItemUseEffectDefinition::ApplyRestorativeFeast { .. }
            | ItemUseEffectDefinition::ApplyLifeRestoration { .. }) => {
                self.resolve_item_vitality_restoration_effect(source_kind_id, effect, events)
            }
            ItemUseEffectDefinition::DrainAttribute { attribute } => self
                .resolve_item_drain_attribute(
                    source_kind_id,
                    Self::item_attribute_kind(attribute),
                    events,
                ),
            ItemUseEffectDefinition::RestoreAttribute { attribute } => self
                .resolve_item_restore_attribute(
                    source_kind_id,
                    Self::item_attribute_kind(attribute),
                    events,
                ),
            ItemUseEffectDefinition::IncreaseAttribute { attribute } => self
                .resolve_item_increase_attributes(
                    source_kind_id,
                    &[Self::item_attribute_kind(attribute)],
                    events,
                ),
            ItemUseEffectDefinition::AugmentAttributes => self.resolve_item_increase_attributes(
                source_kind_id,
                &[
                    AttributeKind::Strength,
                    AttributeKind::Intelligence,
                    AttributeKind::Wisdom,
                    AttributeKind::Dexterity,
                    AttributeKind::Constitution,
                    AttributeKind::Charisma,
                ],
                events,
            ),
            ItemUseEffectDefinition::ApplyThermalResistance {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => self.resolve_item_thermal_resistance(
                source_kind_id,
                *duration_dice,
                *duration_sides,
                *duration_bonus,
                events,
            ),
            ItemUseEffectDefinition::ApplyBasicResistance {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => {
                self.resolve_item_basic_resistance(
                    source_kind_id,
                    *duration_dice,
                    *duration_sides,
                    *duration_bonus,
                    events,
                );
                true
            }
            ItemUseEffectDefinition::ApplyPoison {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => self.resolve_item_poison(
                source_kind_id,
                *duration_dice,
                *duration_sides,
                *duration_bonus,
                events,
            ),
            ItemUseEffectDefinition::ApplyBlindness {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => self.resolve_item_blindness(
                source_kind_id,
                *duration_dice,
                *duration_sides,
                *duration_bonus,
                events,
            ),
            ItemUseEffectDefinition::ApplyStatus {
                status_kind_id,
                duration_dice,
                duration_sides,
                duration_bonus,
                stacking,
                resistance_type,
                granted_resistances,
                granted_modifiers,
                granted_equipment_bonuses,
                incoming_damage_percent,
            } => self.resolve_item_status(
                source_kind_id,
                status_kind_id,
                *duration_dice,
                *duration_sides,
                *duration_bonus,
                *stacking,
                *resistance_type,
                granted_resistances,
                granted_modifiers,
                granted_equipment_bonuses,
                *incoming_damage_percent,
                events,
            ),
            ItemUseEffectDefinition::ApplyGiantStrength {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => self.resolve_item_giant_strength(
                source_kind_id,
                *duration_dice,
                *duration_sides,
                *duration_bonus,
                events,
            ),
            ItemUseEffectDefinition::ApplyDetonation {
                damage_dice,
                damage_sides,
                stun_ticks,
                bleeding_ticks,
            } => {
                self.resolve_item_detonation(
                    source_kind_id,
                    *damage_dice,
                    *damage_sides,
                    *stun_ticks,
                    *bleeding_ticks,
                    events,
                );
                true
            }
            ItemUseEffectDefinition::SelfLifeLoss { amount } => {
                self.resolve_item_life_loss(source_kind_id, *amount, events);
                true
            }
            ItemUseEffectDefinition::SelfDamage {
                damage_dice,
                damage_sides,
                damage_bonus,
            } => {
                let amount = u32::try_from(self.roll_damage(*damage_dice, *damage_sides))
                    .expect("validated self damage must fit u32")
                    .saturating_add(u32::from(*damage_bonus));
                self.resolve_item_life_loss(source_kind_id, amount, events);
                true
            }
            ItemUseEffectDefinition::LoseExperienceFraction { divisor } => {
                self.resolve_item_experience_loss(source_kind_id, *divisor, events)
            }
            ItemUseEffectDefinition::Vengeance {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => {
                self.resolve_item_vengeance(
                    source_kind_id,
                    *duration_dice,
                    *duration_sides,
                    *duration_bonus,
                    events,
                );
                true
            }
            ItemUseEffectDefinition::ProtectionFromEvil => {
                self.resolve_item_protection_from_evil(source_kind_id, events);
                true
            }
            ItemUseEffectDefinition::PrepareConfusingStrike => {
                self.resolve_item_confusing_strike_preparation(source_kind_id, events)
            }
            ItemUseEffectDefinition::IncreaseSpellLearningCapacity => {
                self.resolve_item_spell_learning_capacity(source_kind_id, events)
            }
            ItemUseEffectDefinition::RemoveStatus { status_kind_id } => {
                self.resolve_item_status_removal(source_kind_id, status_kind_id, events)
            }
            ItemUseEffectDefinition::ReduceStatus {
                status_kind_id,
                minimum_reduction,
                reduction_divisor,
            } => self.resolve_item_status_reduction(
                source_kind_id,
                status_kind_id,
                *minimum_reduction,
                *reduction_divisor,
                events,
            ),
            ItemUseEffectDefinition::DrainResourceFull { resource_id } => {
                self.resolve_item_resource_drain(source_kind_id, resource_id, events)
            }
            ItemUseEffectDefinition::Sequence { effects } => {
                let mut noticed = false;
                for effect in effects {
                    noticed =
                        self.resolve_item_self_effect(source_kind_id, effect, events) || noticed;
                }
                noticed
            }
            ItemUseEffectDefinition::Damage { .. }
            | ItemUseEffectDefinition::SelfCenteredElementalBlast { .. }
            | ItemUseEffectDefinition::AggravateMonsters
            | ItemUseEffectDefinition::MassGenocide { .. }
            | ItemUseEffectDefinition::Genocide { .. }
            | ItemUseEffectDefinition::RechargeFromDevice { .. }
            | ItemUseEffectDefinition::CreateAdjacentTerrain { .. }
            | ItemUseEffectDefinition::DestroyAdjacentTrapsAndDoors
            | ItemUseEffectDefinition::DispelCategory { .. }
            | ItemUseEffectDefinition::BanishVisible { .. }
            | ItemUseEffectDefinition::Detect { .. }
            | ItemUseEffectDefinition::IdentifyItem { .. }
            | ItemUseEffectDefinition::EnchantItem { .. }
            | ItemUseEffectDefinition::CurseEquippedItem { .. }
            | ItemUseEffectDefinition::RemoveEquippedCurses { .. }
            | ItemUseEffectDefinition::SummonCategory { .. }
            | ItemUseEffectDefinition::RandomTeleport { .. }
            | ItemUseEffectDefinition::TeleportLevel
            | ItemUseEffectDefinition::Recall { .. }
            | ItemUseEffectDefinition::ResetRecall => {
                unreachable!("projected item effects cannot resolve as self restoration")
            }
        }
    }
}
