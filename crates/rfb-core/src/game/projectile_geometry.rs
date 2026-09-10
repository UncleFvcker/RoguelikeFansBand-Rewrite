// SPDX-License-Identifier: MPL-2.0
// Shared projectile paths, area geometry, and line-of-effect queries.

use rfb_content::{AbilityDefinition, AbilityTargetModeDefinition};
use rfb_protocol::{Direction, Position, TargetSelection};

use super::{Game, actor_matches_category};
use crate::{event::ProjectileTrace, resistance::DamageType};

impl Game {
    pub(super) fn ability_path(
        &self,
        ability: &AbilityDefinition,
        target: &TargetSelection,
    ) -> Option<Vec<Position>> {
        let mode = match target {
            TargetSelection::Direction { .. } => AbilityTargetModeDefinition::Direction,
            TargetSelection::Position { .. } => AbilityTargetModeDefinition::Position,
            TargetSelection::Entity { .. } => AbilityTargetModeDefinition::Entity,
            TargetSelection::Item { .. } => AbilityTargetModeDefinition::Item,
            TargetSelection::Town { .. } => AbilityTargetModeDefinition::Town,
            TargetSelection::CraftingItem { .. } => return None,
            TargetSelection::SelfTarget => AbilityTargetModeDefinition::SelfTarget,
        };
        if !ability.target.modes.contains(&mode) {
            return None;
        }
        self.projectile_path(target, ability.target.range)
    }

    pub(super) fn beam_ability_path(
        &self,
        ability: &AbilityDefinition,
        target: &TargetSelection,
    ) -> Option<Vec<Position>> {
        let mode = match target {
            TargetSelection::Direction { .. } => AbilityTargetModeDefinition::Direction,
            TargetSelection::Position { .. } => AbilityTargetModeDefinition::Position,
            TargetSelection::Entity { .. } => AbilityTargetModeDefinition::Entity,
            TargetSelection::Item { .. } => AbilityTargetModeDefinition::Item,
            TargetSelection::Town { .. } => AbilityTargetModeDefinition::Town,
            TargetSelection::CraftingItem { .. } => return None,
            TargetSelection::SelfTarget => AbilityTargetModeDefinition::SelfTarget,
        };
        if !ability.target.modes.contains(&mode) {
            return None;
        }
        match target {
            TargetSelection::Direction { .. } => self.projectile_path(target, ability.target.range),
            TargetSelection::Position { position } => {
                self.targeted_projectile_path_through_target(*position, ability.target.range)
            }
            TargetSelection::Entity { entity_id } => {
                let position = self
                    .entities
                    .iter()
                    .find(|entity| {
                        entity.id == *entity_id && self.entity_is_visible_to_player(entity)
                    })
                    .map(|entity| entity.position)?;
                self.targeted_projectile_path_through_target(position, ability.target.range)
            }
            TargetSelection::SelfTarget => None,
            TargetSelection::Item { .. } => None,
            TargetSelection::Town { .. } | TargetSelection::CraftingItem { .. } => None,
        }
    }

    pub(super) fn projectile_path(
        &self,
        target: &TargetSelection,
        range: u16,
    ) -> Option<Vec<Position>> {
        let origin = self.player.position;
        match target {
            TargetSelection::Direction { direction } => {
                let (dx, dy) = direction.delta();
                Some(
                    (1..=range)
                        .map(|step| Position {
                            x: origin.x + dx * i32::from(step),
                            y: origin.y + dy * i32::from(step),
                        })
                        .collect(),
                )
            }
            TargetSelection::Position { position } => {
                self.targeted_projectile_path(*position, range)
            }
            TargetSelection::Entity { entity_id } => {
                let position = self
                    .entities
                    .iter()
                    .find(|entity| {
                        entity.id == *entity_id && self.entity_is_visible_to_player(entity)
                    })
                    .map(|entity| entity.position)?;
                self.targeted_projectile_path(position, range)
            }
            TargetSelection::SelfTarget => None,
            TargetSelection::Item { .. } => None,
            TargetSelection::Town { .. } | TargetSelection::CraftingItem { .. } => None,
        }
    }

    fn targeted_projectile_path(&self, target: Position, range: u16) -> Option<Vec<Position>> {
        self.projectile_path_to_position(target, range, false, true)
    }

    pub(super) fn targeted_projectile_path_through_target(
        &self,
        target: Position,
        range: u16,
    ) -> Option<Vec<Position>> {
        self.projectile_path_to_position(target, range, true, true)
    }

    pub(super) fn untargeted_projectile_path(
        &self,
        target: Position,
        range: u16,
    ) -> Option<Vec<Position>> {
        self.projectile_path_to_position(target, range, false, false)
    }

    fn projectile_path_to_position(
        &self,
        target: Position,
        range: u16,
        continue_through_target: bool,
        require_visible: bool,
    ) -> Option<Vec<Position>> {
        let origin = self.player.position;
        if target == origin
            || self.index(target).is_none()
            || (require_visible && !self.is_visible(target))
            || origin.x.abs_diff(target.x).max(origin.y.abs_diff(target.y)) > u32::from(range)
        {
            return None;
        }

        let mut x = origin.x;
        let mut y = origin.y;
        let dx = (target.x - x).abs();
        let sx = if x < target.x { 1 } else { -1 };
        let dy = -(target.y - y).abs();
        let sy = if y < target.y { 1 } else { -1 };
        let mut error = dx + dy;
        let mut path = Vec::new();
        let max_steps = usize::from(range);
        while path.len() < max_steps {
            if !continue_through_target && x == target.x && y == target.y {
                break;
            }
            let doubled = error.saturating_mul(2);
            if doubled >= dy {
                error += dy;
                x += sx;
            }
            if doubled <= dx {
                error += dx;
                y += sy;
            }
            path.push(Position { x, y });
            if !continue_through_target && (x == target.x && y == target.y) {
                break;
            }
            if path.len() >= max_steps {
                break;
            }
        }
        Some(path)
    }

    pub(super) fn trace_projectile_path(
        &self,
        path: Vec<Position>,
    ) -> (ProjectileTrace, Option<usize>) {
        self.trace_projectile_path_with_actor_policy(path, true)
    }

    pub(super) fn trace_projectile_path_with_actor_policy(
        &self,
        path: Vec<Position>,
        stop_at_actor: bool,
    ) -> (ProjectileTrace, Option<usize>) {
        self.trace_projectile_path_with_damage_policy(path, stop_at_actor, None)
    }

    pub(super) fn trace_projectile_path_with_damage_policy(
        &self,
        path: Vec<Position>,
        stop_at_actor: bool,
        damage_type: Option<DamageType>,
    ) -> (ProjectileTrace, Option<usize>) {
        let origin = self.player.position;
        let mut impact = origin;
        let mut landing = origin;
        let mut traversed = Vec::new();
        let mut target_index = None;
        for position in path {
            impact = position;
            let traversable = self.index(position).is_some_and(|index| {
                if damage_type == Some(DamageType::Disintegrate) {
                    !self
                        .content
                        .terrain(&self.terrain[index])
                        .is_some_and(|terrain| terrain.tags.iter().any(|tag| tag == "permanent"))
                } else {
                    self.is_walkable(position)
                }
            });
            if !traversable {
                break;
            }
            landing = position;
            traversed.push(position);
            if let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.position == position)
            {
                target_index = Some(index);
                if stop_at_actor {
                    break;
                }
            }
        }
        (
            ProjectileTrace {
                origin,
                impact,
                landing,
                traversed,
            },
            target_index,
        )
    }

    pub(super) fn area_damage_targets(
        &self,
        center: Position,
        radius: u8,
        target_category: Option<&str>,
    ) -> (Vec<Position>, Vec<(String, u32)>) {
        self.area_damage_targets_for_type(center, radius, target_category, DamageType::Physical)
    }

    pub(super) fn area_damage_targets_for_type(
        &self,
        center: Position,
        radius: u8,
        target_category: Option<&str>,
        damage_type: DamageType,
    ) -> (Vec<Position>, Vec<(String, u32)>) {
        let cells = self.area_damage_cells_for_type(center, radius, damage_type);
        let affected_positions = cells.iter().map(|(_, position)| *position).collect();
        let targets = cells
            .iter()
            .flat_map(|(distance, position)| {
                self.entities
                    .iter()
                    .filter(move |entity| {
                        entity.hp > 0
                            && entity.position == *position
                            && target_category.is_none_or(|category| {
                                self.content
                                    .actor(&entity.kind_id)
                                    .is_some_and(|definition| {
                                        actor_matches_category(definition, category)
                                    })
                            })
                    })
                    .map(move |entity| (entity.id.clone(), *distance))
            })
            .collect();
        (affected_positions, targets)
    }

    pub(super) fn area_damage_cells(&self, center: Position, radius: u8) -> Vec<(u32, Position)> {
        self.area_damage_cells_for_type(center, radius, DamageType::Physical)
    }

    fn area_damage_cells_for_type(
        &self,
        center: Position,
        radius: u8,
        damage_type: DamageType,
    ) -> Vec<(u32, Position)> {
        let mut cells = Vec::new();
        let radius_limit = u32::from(radius);
        let radius = i32::from(radius);
        for y in center.y - radius..=center.y + radius {
            for x in center.x - radius..=center.x + radius {
                let position = Position { x, y };
                let distance = rfb_distance(center, position);
                if distance > radius_limit
                    || !self.index(position).is_some_and(|_| {
                        if damage_type == DamageType::Disintegrate {
                            has_disintegration_line_of_effect(self, center, position)
                        } else {
                            has_line_of_effect(self, center, position)
                        }
                    })
                {
                    continue;
                }
                cells.push((distance, position));
            }
        }
        cells.sort_by_key(|(distance, position)| (*distance, position.y, position.x));
        cells
    }

    pub(super) fn beam_damage_targets(&self, path: &[Position]) -> Vec<String> {
        path.iter()
            .flat_map(|position| {
                self.entities
                    .iter()
                    .filter(move |entity| entity.hp > 0 && entity.position == *position)
                    .map(|entity| entity.id.clone())
            })
            .collect()
    }

    pub(super) fn cone_damage_targets(
        &self,
        centerline: &[Position],
        direction: Direction,
        radius: u8,
        damage_type: DamageType,
    ) -> (Vec<Position>, Vec<(String, u32)>) {
        let cells = self.cone_damage_cells(
            self.player.position,
            centerline,
            direction,
            radius,
            damage_type,
        );
        let affected_positions = cells.iter().map(|(_, _, position)| *position).collect();
        let targets = cells
            .iter()
            .flat_map(|(_, lateral_distance, position)| {
                self.entities
                    .iter()
                    .filter(move |entity| entity.hp > 0 && entity.position == *position)
                    .map(move |entity| (entity.id.clone(), *lateral_distance))
            })
            .collect();
        (affected_positions, targets)
    }

    pub(super) fn cone_damage_cells(
        &self,
        origin: Position,
        centerline: &[Position],
        direction: Direction,
        radius: u8,
        damage_type: DamageType,
    ) -> Vec<(i32, u32, Position)> {
        let depth = i32::try_from(centerline.len()).unwrap_or(i32::MAX);
        if depth == 0 {
            return Vec::new();
        }
        let (dx, dy) = direction.delta();
        let width_denominator = (depth - 1).max(1);
        let mut cells = Vec::new();
        for (index, center) in centerline.iter().enumerate() {
            let layer = i32::try_from(index + 1).unwrap_or(i32::MAX);
            debug_assert_eq!(
                *center,
                Position {
                    x: origin.x + dx * layer,
                    y: origin.y + dy * layer,
                }
            );
            let width = if depth == 1 {
                0
            } else {
                i32::from(radius).saturating_mul(layer - 1) / width_denominator
            };
            for y in center.y - width..=center.y + width {
                for x in center.x - width..=center.x + width {
                    let position = Position { x, y };
                    let position_layer = origin
                        .x
                        .abs_diff(position.x)
                        .max(origin.y.abs_diff(position.y));
                    let offset_x = position.x - origin.x;
                    let offset_y = position.y - origin.y;
                    let forward = offset_x * dx + offset_y * dy;
                    let lateral = (offset_x * dy - offset_y * dx).abs();
                    if position_layer != u32::try_from(layer).unwrap_or(u32::MAX)
                        || forward <= 0
                        || lateral > forward
                        || self.index(position).is_none()
                        || if damage_type == DamageType::Disintegrate {
                            !has_disintegration_line_of_effect(self, origin, position)
                        } else {
                            !has_line_of_effect(self, origin, position)
                        }
                    {
                        continue;
                    }
                    let lateral_distance = center
                        .x
                        .abs_diff(position.x)
                        .max(center.y.abs_diff(position.y));
                    cells.push((layer, lateral_distance, position));
                }
            }
        }
        cells.sort_by_key(|(layer, lateral_distance, position)| {
            (*layer, *lateral_distance, position.y, position.x)
        });
        cells
    }
}

/// Angband/RFB's integer distance approximation: a rounded Euclidean
/// distance with the familiar max + min/2 fast path.  Keeping this separate
/// from the UI's Chebyshev targeting distance makes ball falloff and rings
/// match the original projection routine.
pub(super) fn rfb_distance(from: Position, to: Position) -> u32 {
    let dy = from.y.abs_diff(to.y);
    let dx = from.x.abs_diff(to.x);
    let target = dy.saturating_mul(dy).saturating_add(dx.saturating_mul(dx));
    let mut distance = if dy > dx {
        dy + (dx >> 1)
    } else {
        dx + (dy >> 1)
    };
    if dy == 0 || dx == 0 {
        return distance;
    }
    loop {
        let denominator = distance.saturating_mul(2).max(1);
        let error = (target as i64 - distance.saturating_mul(distance) as i64) / denominator as i64;
        if error == 0 {
            return distance;
        }
        let next = (distance as i64 + error).max(0);
        distance = u32::try_from(next).unwrap_or(u32::MAX);
    }
}

pub(super) fn rfb_area_damage(base_damage: i32, distance: u32) -> i32 {
    let numerator = i64::from(base_damage.max(0)).saturating_add(i64::from(distance));
    i32::try_from(numerator / i64::from(distance.saturating_add(1))).unwrap_or(i32::MAX)
}

pub(super) fn projectile_path_between(
    origin: Position,
    target: Position,
    range: u16,
) -> Option<Vec<Position>> {
    if target == origin
        || origin.x.abs_diff(target.x).max(origin.y.abs_diff(target.y)) > u32::from(range)
    {
        return None;
    }
    let mut x = origin.x;
    let mut y = origin.y;
    let dx = (target.x - x).abs();
    let sx = if x < target.x { 1 } else { -1 };
    let dy = -(target.y - y).abs();
    let sy = if y < target.y { 1 } else { -1 };
    let mut error = dx + dy;
    let mut path = Vec::new();
    while path.len() < usize::from(range) {
        let doubled = error.saturating_mul(2);
        if doubled >= dy {
            error += dy;
            x += sx;
        }
        if doubled <= dx {
            error += dx;
            y += sy;
        }
        let position = Position { x, y };
        path.push(position);
        if position == target {
            return Some(path);
        }
    }
    None
}

pub(super) fn projectile_path_through_target(
    origin: Position,
    target: Position,
    range: u16,
) -> Option<Vec<Position>> {
    if target == origin
        || origin.x.abs_diff(target.x).max(origin.y.abs_diff(target.y)) > u32::from(range)
    {
        return None;
    }
    let mut x = origin.x;
    let mut y = origin.y;
    let dx = (target.x - x).abs();
    let sx = if x < target.x { 1 } else { -1 };
    let dy = -(target.y - y).abs();
    let sy = if y < target.y { 1 } else { -1 };
    let mut error = dx + dy;
    let mut path = Vec::with_capacity(usize::from(range));
    while path.len() < usize::from(range) {
        let doubled = error.saturating_mul(2);
        if doubled >= dy {
            error += dy;
            x += sx;
        }
        if doubled <= dx {
            error += dx;
            y += sy;
        }
        path.push(Position { x, y });
    }
    path.contains(&target).then_some(path)
}

pub(super) fn has_line_of_effect(game: &Game, from: Position, to: Position) -> bool {
    let mut x = from.x;
    let mut y = from.y;
    let dx = (to.x - from.x).abs();
    let dy = (to.y - from.y).abs();
    let step_x = if from.x < to.x { 1 } else { -1 };
    let step_y = if from.y < to.y { 1 } else { -1 };
    let mut error = dx - dy;

    loop {
        if x == to.x && y == to.y {
            return true;
        }
        if !(game.is_walkable(Position { x, y }) || (x == from.x && y == from.y)) {
            return false;
        }
        let double_error = error * 2;
        if double_error > -dy {
            error -= dy;
            x += step_x;
        }
        if double_error < dx {
            error += dx;
            y += step_y;
        }
        if game.index(Position { x, y }).is_none() {
            return false;
        }
    }
}

pub(super) fn has_disintegration_line_of_effect(game: &Game, from: Position, to: Position) -> bool {
    let mut x = from.x;
    let mut y = from.y;
    let dx = (to.x - from.x).abs();
    let dy = (to.y - from.y).abs();
    let step_x = if from.x < to.x { 1 } else { -1 };
    let step_y = if from.y < to.y { 1 } else { -1 };
    let mut error = dx - dy;

    loop {
        let position = Position { x, y };
        let Some(index) = game.index(position) else {
            return false;
        };
        if position != from
            && game
                .content
                .terrain(&game.terrain[index])
                .is_some_and(|terrain| terrain.tags.iter().any(|tag| tag == "permanent"))
        {
            return false;
        }
        if position == to {
            return true;
        }
        let double_error = error * 2;
        if double_error > -dy {
            error -= dy;
            x += step_x;
        }
        if double_error < dx {
            error += dx;
            y += step_y;
        }
    }
}

impl Game {
    pub(super) fn terrain_is_projectable(&self, position: Position) -> bool {
        self.index(position).is_some_and(|index| {
            let terrain = self
                .content
                .terrain(&self.terrain[index])
                .expect("validated terrain");
            (terrain.walkable || terrain.tags.iter().any(|tag| tag == "projectable"))
                && !terrain.tags.iter().any(|tag| tag == "blocks-projectiles")
        })
    }
}
