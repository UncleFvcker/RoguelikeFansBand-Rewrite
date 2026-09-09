// SPDX-License-Identifier: MPL-2.0

use crate::game::player_combat::ProjectileMode;
use crate::game::projectile_geometry::rfb_distance;
use crate::game::{
    AbilityDefinition, AbilityEffectDefinition, AbilityGenocideScopeDefinition,
    AbilityTargetModeDefinition, Direction, FloorTransitionTarget, Game, ItemLocation, Position,
    RecallUseAction, TargetSelection, floor_dungeon_id,
};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::game) enum AbilityTargetPlan {
    SelfTarget,
    Detect,
    TerrainTransform {
        center: Position,
        positions: Vec<Position>,
    },
    AdjacentTerrain {
        replacements: Vec<(Position, String)>,
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
        target: TargetSelection,
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
    GreaterDemonSacrifice {
        item_id: String,
        candidate_kind_ids: Vec<String>,
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

impl Game {
    pub(in crate::game) fn ability_target_plan(
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
            | AbilityEffectDefinition::DraconianBreathDamage { .. }
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
            AbilityEffectDefinition::FetchItem { .. } => {
                let mode = match target {
                    TargetSelection::Direction { .. } => AbilityTargetModeDefinition::Direction,
                    TargetSelection::Position { position } if self.index(*position).is_some() => {
                        AbilityTargetModeDefinition::Position
                    }
                    TargetSelection::Entity { entity_id }
                        if self.entities.iter().any(|entity| {
                            entity.id == *entity_id && self.entity_is_visible_to_player(entity)
                        }) =>
                    {
                        AbilityTargetModeDefinition::Entity
                    }
                    _ => return None,
                };
                ability
                    .target
                    .modes
                    .contains(&mode)
                    .then(|| AbilityTargetPlan::FetchItem {
                        target: target.clone(),
                    })
            }
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
                            && self.item_is_in_pack_or_at_feet(item)
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
            AbilityEffectDefinition::DraconianStrike { .. } => {
                let TargetSelection::Direction { direction } = target else {
                    return None;
                };
                let position = self.position_in_direction(*direction);
                let target_entity_id = self
                    .entities
                    .iter()
                    .find(|entity| {
                        entity.hp > 0
                            && entity.position == position
                            && !self.actor_is_player_side(entity)
                    })?
                    .id
                    .clone();
                Some(AbilityTargetPlan::MeleeThenTeleport {
                    target_entity_id,
                    teleport_candidates: Vec::new(),
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
                    ProjectileMode::Sniper(mode),
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
                Some(AbilityTargetPlan::SummonCategory {
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
            AbilityEffectDefinition::SummonGreaterDemon {
                ref corpse_item_kind_id,
                radius,
            } => {
                let TargetSelection::Item { item_id } = target else {
                    return None;
                };
                let corpse = self.items.iter().find(|item| {
                    item.id == *item_id
                        && item.kind_id == *corpse_item_kind_id
                        && item.quantity > 0
                        && match item.location {
                            ItemLocation::Inventory => true,
                            ItemLocation::Ground(position) => position == self.player.position,
                            _ => false,
                        }
                })?;
                let corpse_actor = self
                    .content
                    .actor(corpse.origin_actor_kind_id.as_deref()?)?;
                if !matches!(corpse_actor.glyph.as_str(), "p" | "h" | "t") {
                    return None;
                }
                let maximum_level = u16::try_from(corpse_actor.level)
                    .unwrap_or(u16::MAX)
                    .saturating_add(self.progress.level.saturating_mul(2) / 3);
                let candidate_kind_ids = self
                    .summon_category_candidate_kind_ids("demon", None, maximum_level, false)
                    .into_iter()
                    .filter(|kind_id| {
                        self.content
                            .actor(kind_id)
                            .is_some_and(|actor| matches!(actor.glyph.as_str(), "U" | "H" | "B"))
                    })
                    .collect::<Vec<_>>();
                let positions = self.open_positions_around_for_actor_kinds(
                    self.player.position,
                    radius,
                    &candidate_kind_ids,
                );
                Some(AbilityTargetPlan::GreaterDemonSacrifice {
                    item_id: item_id.clone(),
                    candidate_kind_ids,
                    positions,
                })
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
            AbilityEffectDefinition::ProtectFromCorrosion => {
                let TargetSelection::Item { item_id } = target else {
                    return None;
                };
                self.items
                    .iter()
                    .find(|item| {
                        item.id == *item_id && self.item_can_receive_corrosion_protection(item)
                    })
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
            AbilityEffectDefinition::CreateAdjacentTerrain {
                ref source_terrain_ids,
                ref target_terrain_id,
            } => (matches!(target, TargetSelection::SelfTarget)
                && ability
                    .target
                    .modes
                    .contains(&AbilityTargetModeDefinition::SelfTarget))
            .then(|| AbilityTargetPlan::AdjacentTerrain {
                replacements: self
                    .adjacent_terrain_creation_replacements(source_terrain_ids, target_terrain_id),
            }),
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
            | AbilityEffectDefinition::RemoveEquippedCurses { .. }
            | AbilityEffectDefinition::BeginFasting
            | AbilityEffectDefinition::TurnUndead { .. }
            | AbilityEffectDefinition::SustainAttributes { .. }
            | AbilityEffectDefinition::CureMutation
            | AbilityEffectDefinition::CreateCurrentTerrain { .. }
            | AbilityEffectDefinition::NatureGate { .. }
            | AbilityEffectDefinition::DemonSummoning
            | AbilityEffectDefinition::AngelSummoning
            | AbilityEffectDefinition::BanishEvil
            | AbilityEffectDefinition::DivineIntervention
            | AbilityEffectDefinition::Crusade
            | AbilityEffectDefinition::InsanityCircle { .. }
            | AbilityEffectDefinition::ExplodePets
            | AbilityEffectDefinition::ReduceStatus { .. }
            | AbilityEffectDefinition::SatisfyHunger
            | AbilityEffectDefinition::DevourFlesh { .. }
            | AbilityEffectDefinition::Vomit
            | AbilityEffectDefinition::CreateItem { .. }
            | AbilityEffectDefinition::CreateStair { .. }
            | AbilityEffectDefinition::SelfKnowledge
            | AbilityEffectDefinition::Clairvoyance { .. }
            | AbilityEffectDefinition::CallSunlight { .. }
            | AbilityEffectDefinition::NatureWrath
            | AbilityEffectDefinition::Probe
            | AbilityEffectDefinition::CreateDoor { .. }
            | AbilityEffectDefinition::DeviceMastery { .. }
            | AbilityEffectDefinition::Banish { .. }
            | AbilityEffectDefinition::Invulnerability { .. }
            | AbilityEffectDefinition::LightArea { .. }
            | AbilityEffectDefinition::MassSleepOrStasis { .. }
            | AbilityEffectDefinition::SleepingDust { .. }
            | AbilityEffectDefinition::Sanctuary { .. } => {
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
            AbilityEffectDefinition::AlterReality => {
                (matches!(target, TargetSelection::SelfTarget)
                    && ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget))
                .then_some(AbilityTargetPlan::SelfTarget)
            }
            AbilityEffectDefinition::VisibleDamage { .. }
            | AbilityEffectDefinition::VisibleApplyStatus { .. }
            | AbilityEffectDefinition::Entangle { .. }
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
            | AbilityEffectDefinition::DoomHand
            | AbilityEffectDefinition::Hellfire { .. }
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
            AbilityEffectDefinition::WrathOfGod => {
                self.ability_path(ability, target)
                    .map(|path| AbilityTargetPlan::Projectile {
                        path,
                        stop_at_actor: matches!(target, TargetSelection::Direction { .. }),
                    })
            }
            AbilityEffectDefinition::LavaFlow { .. } => {
                (matches!(target, TargetSelection::SelfTarget)
                    && ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget))
                .then_some(AbilityTargetPlan::SelfTarget)
            }
            AbilityEffectDefinition::BeamDamage { .. }
            | AbilityEffectDefinition::LightLine { .. }
            | AbilityEffectDefinition::TerrainBeam { .. }
            | AbilityEffectDefinition::BoltOrBeamDamage { .. }
            | AbilityEffectDefinition::Stardust { .. } => self
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
}
