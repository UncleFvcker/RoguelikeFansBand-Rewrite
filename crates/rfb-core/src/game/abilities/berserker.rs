// SPDX-License-Identifier: MPL-2.0

use crate::game::*;

impl Game {
    /// Desktop acceptance: real level gains, a quiet floor, and explicit item/HP fixtures.
    #[doc(hidden)]
    pub fn debug_prepare_berserker_e2e(
        &mut self,
        level: u16,
        wounded: bool,
        with_target: bool,
    ) -> Result<(), CoreError> {
        self.entities.clear();
        self.items
            .retain(|item| !matches!(item.location, ItemLocation::CarriedBy { .. }));
        let experience = self
            .experience_required_for_level(level)
            .saturating_sub(self.progress.experience);
        self.apply_player_experience(experience, &mut Vec::new());
        self.player.hp = if wounded {
            1
        } else {
            self.effective_player_max_hp()
        };
        for (id, kind) in [
            ("e2e.scroll", "demo.item.appraisal-scroll"),
            ("e2e.wand", "demo.item.magic-missile-wand"),
            ("e2e.activation", "demo.item.dr-jones-whip"),
        ] {
            if !self.items.iter().any(|item| item.id == id) {
                self.debug_add_generated_inventory_item(id, kind, 1)?;
                self.mark_item_aware(kind);
            }
        }
        self.generated_artifact_ids
            .insert("demo.item.dr-jones-whip".to_owned());
        if with_target {
            let position = [
                Direction::East,
                Direction::South,
                Direction::West,
                Direction::North,
            ]
            .into_iter()
            .find_map(|direction| {
                let (dx, dy) = direction.delta();
                (1..=2)
                    .all(|step| {
                        let position = Position {
                            x: self.player.position.x + dx * step,
                            y: self.player.position.y + dy * step,
                        };
                        self.index(position).is_some()
                            && self.content.terrain(self.terrain_at(position)).is_some_and(
                                |terrain| {
                                    self.player_can_cross_terrain(terrain) && terrain.trap.is_none()
                                },
                            )
                    })
                    .then(|| self.position_in_direction(direction))
            })
            .ok_or(CoreError::InvalidSave(
                "Berserker E2E needs two open adjacent cells",
            ))?;
            let actor = self.generated_actor(
                "e2e.charge-target".to_owned(),
                "demo.actor.stone-troll",
                position,
            );
            self.entities.push(actor);
        }
        Ok(())
    }

    pub(in crate::game) fn berserker_cast_is_zero_time_unavailable(
        &self,
        ability_id: &str,
        target: &TargetSelection,
    ) -> bool {
        if !self.player_is_berserker() {
            return false;
        }
        let Some(activation) = self.class_ability_activation(ability_id) else {
            return false;
        };
        self.progress.level < activation.minimum_level
            || self.player_has_status_kind(STATUS_CONFUSION)
            || self.player_has_status_kind(STATUS_FEAR)
            || i64::from(self.player.hp) < i64::from(activation.hit_point_cost)
            || self
                .content
                .ability(ability_id)
                .is_none_or(|ability| self.ability_target_plan(ability, target).is_none())
    }

    pub(super) fn resolve_player_charge_through(
        &mut self,
        direction: Direction,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<Option<Position>, CoreError> {
        let target = self.position_in_direction(direction);
        let Some(index) = self
            .entities
            .iter()
            .position(|actor| actor.position == target && actor.hp > 0)
        else {
            // berserker.c: a blind swing into empty space still pays the full cost.
            return Ok(None);
        };
        let origin = self.player.position;
        let floor_id = self.current_floor_id.clone();
        let energy = self.player.energy_need;
        self.resolve_player_melee(index, true, events, changed, removed_entities)?;
        // The outer technique always takes 100 energy, including after a kill or human STR critical.
        self.player.energy_need = energy;
        if self.player_is_dead()
            || self.current_floor_id != floor_id
            || self.player.position != origin
        {
            return Ok(None);
        }
        let (dx, dy) = direction.delta();
        let destination = Position {
            x: target.x + dx,
            y: target.y + dy,
        };
        let can_cross = |position| {
            self.index(position).is_some()
                && self
                    .content
                    .terrain(self.terrain_at(position))
                    .is_some_and(|terrain| {
                        self.player_can_cross_terrain(terrain) && terrain.trap.is_none()
                    })
        };
        if can_cross(target)
            && can_cross(destination)
            && !self
                .entities
                .iter()
                .any(|actor| actor.position == destination)
        {
            return self
                .enter_player_position(destination, false, events, changed, removed_entities)
                .map(|step| step.map_translation);
        }
        Ok(None)
    }
}
