// SPDX-License-Identifier: MPL-2.0
// Visibility, exploration memory, and player senses.

use std::collections::{BTreeMap, BTreeSet};

use rfb_content::{ActorDamageType, EquipmentPassive};
use rfb_protocol::Position;

use super::{Game, HUMAN_WIS_MUTATION_ID, squared_distance};
use crate::{
    effect::STATUS_BLINDNESS,
    state::{Actor, ItemLocation},
};

pub(super) const VISIBILITY_RADIUS: i32 = 8;

impl Game {
    pub(super) fn player_has_night_vision(&self) -> bool {
        self.player_equipment_passives()
            .contains(&EquipmentPassive::NightVision)
    }

    pub(super) fn player_monster_sight_radius(&self) -> i32 {
        // monster2.c:update_mon halves monster sight, not terrain FOV or ESP.
        // Keep the existing eight-cell FOV scale used by this engine.
        if self.dungeon_has_darkness() && !self.player_has_night_vision() {
            VISIBILITY_RADIUS / 2
        } else {
            VISIBILITY_RADIUS
        }
    }

    pub(super) fn dungeon_detection_radius(&self, radius: u8) -> u8 {
        if self.dungeon_has_darkness() {
            radius / 3
        } else {
            radius
        }
    }

    pub(super) fn reveal_current_visibility(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                let position = Position {
                    x: i32::from(x),
                    y: i32::from(y),
                };
                if self.is_visible(position) {
                    let index = self.index(position).expect("visibility position is valid");
                    self.explored[index] = true;
                }
            }
        }
        let discovered_item_ids = self
            .items
            .iter()
            .filter_map(|item| match item.location {
                ItemLocation::Inventory | ItemLocation::Equipped { .. } => Some(item.id.clone()),
                ItemLocation::Ground(position) if self.is_visible(position) => {
                    Some(item.id.clone())
                }
                ItemLocation::Ground(_)
                | ItemLocation::CarriedBy { .. }
                | ItemLocation::Shop { .. }
                | ItemLocation::Home { .. } => None,
            })
            .collect::<Vec<_>>();
        self.mark_item_instances_discovered(&discovered_item_ids);
        let discovered_gold_ids = self
            .gold_piles
            .iter()
            .filter(|pile| self.is_visible(pile.position))
            .map(|pile| pile.id.clone())
            .collect::<Vec<_>>();
        self.mark_gold_piles_discovered(&discovered_gold_ids);
    }

    pub(super) fn clear_current_floor_memory(&mut self, changed: &mut BTreeSet<Position>) -> u32 {
        let width = usize::from(self.width);
        let mut cleared_cells = 0_u32;
        for (index, explored) in self.explored.iter_mut().enumerate() {
            if *explored {
                *explored = false;
                cleared_cells += 1;
                changed.insert(Position {
                    x: i32::try_from(index % width).expect("explored x must fit i32"),
                    y: i32::try_from(index / width).expect("explored y must fit i32"),
                });
            }
        }
        cleared_cells += u32::try_from(self.revealed_terrain.len()).unwrap_or(0);
        self.revealed_terrain.clear();
        cleared_cells
    }

    pub(super) fn mark_item_instances_discovered(&mut self, item_ids: &[String]) {
        for item_id in item_ids {
            self.item_property_knowledge
                .entry(item_id.clone())
                .or_default()
                .discovered = true;
        }
    }

    pub(super) fn item_is_discovered(&self, item_id: &str) -> bool {
        self.item_property_knowledge
            .get(item_id)
            .is_some_and(|knowledge| knowledge.discovered)
    }

    pub(super) fn is_visible(&self, position: Position) -> bool {
        // Blindness suppresses the whole player FOV except the occupied cell:
        // visuals fall back to remembered knowledge, visibility-gated targeting
        // rejects, and rest no longer interrupts on enemies the player cannot
        // see. Monster senses do not route through this helper.
        if self.player_has_status_kind(STATUS_BLINDNESS) {
            return position == self.player.position;
        }
        if squared_distance(self.player.position, position) > VISIBILITY_RADIUS * VISIBILITY_RADIUS
        {
            return false;
        }
        has_line_of_sight(self, self.player.position, position)
            && (self.floor_has_environment_light()
                || self.player_has_night_vision()
                || position == self.player.position
                || self.position_is_lit(position))
    }

    fn actor_is_invisible(&self, entity: &Actor) -> bool {
        self.actor_runtime_definition(entity)
            .is_some_and(|definition| definition.tags.iter().any(|tag| tag == "invisible"))
    }

    pub(super) fn entity_is_visible_to_player(&self, entity: &Actor) -> bool {
        self.entity_is_visually_visible_to_player(entity)
            || self.entity_is_visible_by_telepathy(entity)
    }

    pub(super) fn entity_is_visually_visible_to_player(&self, entity: &Actor) -> bool {
        let sight = self.player_monster_sight_radius();
        (!self.dungeon_has_darkness()
            || squared_distance(self.player.position, entity.position) <= sight * sight)
            && (self.is_visible(entity.position) || self.entity_is_visible_by_infravision(entity))
            && (!self.actor_is_invisible(entity) || entity.visible_invisible)
    }

    pub(super) fn entity_is_visible_by_telepathy(&self, entity: &Actor) -> bool {
        let Some(definition) = self.actor_runtime_definition(entity) else {
            return false;
        };
        let full_telepathy = self.player_has_telepathy();
        let targeted_esp = self.player_has_targeted_esp(definition);
        if (!full_telepathy && !targeted_esp)
            || squared_distance(self.player.position, entity.position)
                > VISIBILITY_RADIUS * VISIBILITY_RADIUS
        {
            return false;
        }
        if targeted_esp {
            return true;
        }
        if self.player_has_mutation(HUMAN_WIS_MUTATION_ID)
            && !self.actor_is_player_side(entity)
            && definition.tags.iter().any(|tag| tag == "evil")
        {
            return false;
        }
        if definition.tags.iter().any(|tag| tag == "empty-mind") {
            false
        } else if definition.tags.iter().any(|tag| tag == "weird-mind") {
            entity.visible_weird_mind
        } else {
            true
        }
    }

    pub(super) fn entity_is_fuzzy_to_player(&self, entity: &Actor) -> bool {
        !self.actor_is_player_aligned(entity)
            && !self.entity_is_visually_visible_to_player(entity)
            && self.entity_is_visible_by_telepathy(entity)
    }

    fn entity_is_visible_by_infravision(&self, entity: &Actor) -> bool {
        if self.player_has_status_kind(STATUS_BLINDNESS) {
            return false;
        }
        let range = self.player_infravision_range();
        range > 0
            && squared_distance(self.player.position, entity.position) <= range * range
            && has_line_of_sight(self, self.player.position, entity.position)
            && self
                .actor_runtime_definition(entity)
                .is_some_and(|definition| {
                    !definition.tags.iter().any(|tag| tag == "cold-blooded")
                        || definition
                            .contact_auras
                            .iter()
                            .any(|aura| aura.damage_type == ActorDamageType::Fire)
                })
    }

    pub(super) fn refresh_invisible_visibility(
        &mut self,
        full: bool,
        previous_positions: &BTreeMap<String, Position>,
    ) {
        let sources = self.player_see_invisible_sources();
        let search_skill = self.player_derived_stats().search_skill.value.max(0) as u64;
        let candidates = self
            .entities
            .iter()
            .enumerate()
            .filter_map(|(index, entity)| {
                let definition = self.actor_runtime_definition(entity)?;
                definition
                    .tags
                    .iter()
                    .any(|tag| tag == "invisible")
                    .then_some((
                        index,
                        entity.id.clone(),
                        entity.position,
                        definition.level,
                        entity.visible_invisible,
                    ))
            })
            .collect::<Vec<_>>();
        for (index, id, position, level, was_visible) in candidates {
            let sight = self.player_monster_sight_radius();
            if !self.is_visible(position)
                || sources == 0
                || (self.dungeon_has_darkness()
                    && squared_distance(self.player.position, position) > sight * sight)
            {
                self.entities[index].visible_invisible = false;
                continue;
            }
            let moved = previous_positions
                .get(&id)
                .is_none_or(|before| *before != position);
            if !full && !moved {
                self.entities[index].visible_invisible = was_visible;
                continue;
            }
            let difficulty = u64::from(50_u32.saturating_add(level / 2));
            self.entities[index].visible_invisible =
                (0..sources).any(|_| self.rng.bounded(difficulty) < search_skill);
        }
    }

    pub(super) fn refresh_weird_mind_visibility(
        &mut self,
        full: bool,
        previous_positions: &BTreeMap<String, Position>,
    ) {
        let has_telepathy = self.player_has_telepathy();
        let candidates = self
            .entities
            .iter()
            .enumerate()
            .filter_map(|(index, entity)| {
                self.actor_runtime_definition(entity)?
                    .tags
                    .iter()
                    .any(|tag| tag == "weird-mind")
                    .then_some((
                        index,
                        entity.id.clone(),
                        entity.position,
                        entity.visible_weird_mind,
                    ))
            })
            .collect::<Vec<_>>();
        for (index, id, position, was_visible) in candidates {
            if !has_telepathy
                || squared_distance(self.player.position, position)
                    > VISIBILITY_RADIUS * VISIBILITY_RADIUS
            {
                self.entities[index].visible_weird_mind = false;
                continue;
            }
            let moved = previous_positions
                .get(&id)
                .is_none_or(|before| *before != position);
            if !full && !moved {
                self.entities[index].visible_weird_mind = was_visible;
                continue;
            }
            self.entities[index].visible_weird_mind = self.rng.bounded(10) == 0;
        }
    }
}

pub(super) fn has_line_of_sight(game: &Game, from: Position, to: Position) -> bool {
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
        if !(x == from.x && y == from.y)
            && game
                .index(Position { x, y })
                .and_then(|index| game.content.terrain(&game.terrain[index]))
                .is_some_and(|terrain| terrain.blocks_sight)
        {
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
    }
}
