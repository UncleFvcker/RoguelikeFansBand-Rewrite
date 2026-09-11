// SPDX-License-Identifier: MPL-2.0

use super::{Game, GeneratedItemDraft, ItemInstance, ItemLocation, Position};
use crate::{CoreError, game::inventory::item_instances_stack_compatible};

impl Game {
    pub(in crate::game) fn can_drop_item_at(&self, position: Position) -> bool {
        self.index(position).is_some_and(|index| {
            let terrain = self
                .content
                .terrain(&self.terrain[index])
                .expect("validated terrain");
            terrain.allows_items()
                && !terrain.tags.iter().any(|tag| {
                    matches!(
                        tag.as_str(),
                        "no-item-drop"
                            | "warding-glyph"
                            | "explosive-rune"
                            | "door"
                            | "stairs-up"
                            | "stairs-down"
                            | "task-entry"
                            | "shop-entrance"
                            | "town-facility-entrance"
                            | "building"
                    )
                })
        })
    }

    fn drop_interior(&self, at: Position) -> bool {
        at.x > 0
            && at.y > 0
            && at.x < i32::from(self.width) - 1
            && at.y < i32::from(self.height) - 1
    }

    // RFB master a0d92b6378, object2.c::drop_near with chance=-1.
    fn generated_item_drop_position(
        &mut self,
        item: &ItemInstance,
        origin: Position,
    ) -> Option<Position> {
        let artifact = item.is_artifact(&self.content);
        if !artifact {
            // randint0(100) is evaluated even when breakage is disabled.
            self.rng.bounded(100);
        }
        let maximum_stack = self
            .content
            .item(&item.kind_id)
            .expect("generated kind")
            .max_stack;
        let mut best = None;
        let mut ties = 0;
        for dy in -3..=3 {
            for dx in -3..=3 {
                let distance = dx * dx + dy * dy;
                let at = Position {
                    x: origin.x + dx,
                    y: origin.y + dy,
                };
                if distance > 10
                    || !self.drop_interior(at)
                    || !self.can_drop_item_at(at)
                    || !(at == origin
                        || super::super::projectile_geometry::projectile_path_between(
                            origin, at, 3,
                        )
                        .is_some_and(|path| {
                            path.into_iter()
                                .all(|point| self.terrain_is_projectable(point))
                        }))
                {
                    continue;
                }
                let mut piles = self
                    .gold_piles
                    .iter()
                    .filter(|pile| pile.position == at)
                    .count();
                let mut combines = false;
                for existing in self
                    .items
                    .iter()
                    .filter(|existing| existing.location == ItemLocation::Ground(at))
                {
                    piles += 1;
                    // obj_can_combine does not reject a full stack. The source
                    // score can therefore admit a new pile beside a full one.
                    combines |= maximum_stack > 1
                        && item_instances_stack_compatible(&self.content, existing, item);
                }
                piles += usize::from(!combines);
                if piles > 99 {
                    continue;
                }
                let score = 1000 - distance - 5 * piles as i32;
                if best.is_some_and(|(previous, _)| score < previous) {
                    continue;
                }
                if best.is_none_or(|(previous, _)| score > previous) {
                    ties = 0;
                }
                ties += 1;
                if ties >= 2 && self.rng.bounded(ties) != 0 {
                    continue;
                }
                best = Some((score, at));
            }
        }
        if let Some((_, at)) = best {
            return Some(at);
        }
        if !artifact {
            return None;
        }
        let mut at = origin;
        for _ in 0..1000 {
            let y = at.y + self.rng.bounded(3) as i32 - 1;
            let x = at.x + self.rng.bounded(3) as i32 - 1;
            let next = Position { x, y };
            if !self.drop_interior(next) {
                continue;
            }
            at = next;
            if self.can_drop_item_at(at) {
                return Some(at);
            }
        }
        let candidates = (1..i32::from(self.height) - 1)
            .flat_map(|y| (1..i32::from(self.width) - 1).map(move |x| Position { x, y }))
            .filter(|at| self.can_drop_item_at(*at))
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            None
        } else {
            Some(candidates[self.rng.bounded(candidates.len() as u64) as usize])
        }
    }

    /// Returns actual destination IDs, including existing stacks. Drafts are
    /// unknown: failed placement releases the fixed-artifact generation mark,
    /// while random-artifact name registrations remain reserved.
    pub(in crate::game) fn drop_generated_item_near(
        &mut self,
        draft: GeneratedItemDraft,
        origin: Position,
    ) -> Result<Option<(Position, Vec<String>)>, CoreError> {
        let mut item = draft.into_item_instance(String::new(), ItemLocation::Ground(origin));
        let Some(position) = self.generated_item_drop_position(&item, origin) else {
            self.generated_artifact_ids.remove(&item.kind_id);
            return Ok(None);
        };
        item.location = ItemLocation::Ground(position);
        let maximum_stack = self
            .content
            .item(&item.kind_id)
            .expect("generated kind")
            .max_stack;
        let mut indices = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, existing)| {
                existing.location == item.location
                    && existing.quantity < maximum_stack
                    && item_instances_stack_compatible(&self.content, existing, &item)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        indices.sort_by(|a, b| self.items[*a].id.cmp(&self.items[*b].id));
        let capacity: u64 = indices
            .iter()
            .map(|index| u64::from(maximum_stack - self.items[*index].quantity))
            .sum();
        // Reserve before changing any existing stack; full merges consume no ID.
        let new_id = if capacity < u64::from(item.quantity) {
            Some(self.allocate_item_instance_id()?)
        } else {
            None
        };
        let mut ids = Vec::new();
        for index in indices {
            let transferred = item
                .quantity
                .min(maximum_stack - self.items[index].quantity);
            super::super::inventory::merge_item_stack(&mut self.items[index], &item, transferred);
            item.quantity -= transferred;
            ids.push(self.items[index].id.clone());
            if item.quantity == 0 {
                break;
            }
        }
        if let Some(id) = new_id {
            item.id = id.clone();
            self.register_generated_artifact(&item.kind_id);
            self.items.push(item);
            ids.push(id);
        }
        Ok(Some((position, ids)))
    }
}
