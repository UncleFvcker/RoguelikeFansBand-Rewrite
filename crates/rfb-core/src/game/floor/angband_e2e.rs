// SPDX-License-Identifier: MPL-2.0
use crate::game::*;

impl Game {
    /// Player/route preparation only. The Tauri command is WebDriver-only.
    #[doc(hidden)]
    pub fn debug_prepare_angband_e2e(
        &mut self,
        phase: &str,
        target_id: Option<&str>,
    ) -> Result<(), CoreError> {
        if self.world_id != DEFAULT_WORLD_ID
            || self.map_scale != MapScaleDto::Local
            || self
                .build
                .as_ref()
                .is_none_or(|build| build.class_id != "demo.class.warrior")
        {
            return Err(CoreError::InvalidSave(
                "Angband fixture requires a local Warrior",
            ));
        }
        let in_dungeon = self
            .content
            .world(&self.world_id)
            .unwrap()
            .procedural_floors
            .iter()
            .any(|floor| {
                floor.id == self.current_floor_id
                    && floor.dungeon_id.as_deref() == Some("demo.dungeon.angband")
            });
        let at_entrance = self.is_wilderness_floor()
            && self.wilderness_position == Some(Position { x: 57, y: 40 });
        if phase == "arrival"
            && target_id.is_none()
            && self.is_wilderness_floor()
            && self.progress.level == 1
        {
            self.store_visible_town_states();
            self.wilderness_position = Some(Position { x: 57, y: 40 });
            self.wilderness_view_offset = Position::default();
            self.activate_wilderness_position(None, false)?;
            self.apply_player_experience(self.experience_required_for_level(51), &mut Vec::new());
            self.debug_add_generated_inventory_item(
                "e2e.angband.sword",
                "demo.item.broad-sword",
                100,
            )?;
            self.equip_inventory_item("e2e.angband.sword", None)
                .ok_or(CoreError::InvalidSave("Angband fixture cannot equip sword"))?;
            self.debug_add_generated_inventory_item(
                "e2e.angband.recall",
                "demo.item.homeward-scroll",
                100,
            )?;
            for status in [
                STATUS_LEVITATION,
                STATUS_INVULNERABILITY,
                STATUS_SEE_INVISIBLE,
            ] {
                self.apply_player_melee_status(status, 200_000, "e2e.angband");
            }
            let protection = self
                .player
                .statuses
                .iter_mut()
                .find(|status| status.kind_id == STATUS_INVULNERABILITY)
                .unwrap();
            protection.incoming_damage_percent = 0;
            protection.granted_modifiers.max_hp = 2000;
            protection.granted_equipment_bonuses.melee_skill = 1000;
            protection.granted_equipment_bonuses.melee_damage = 1000;
            protection.granted_status_immunities.extend(
                [
                    STATUS_BLEEDING,
                    STATUS_BLINDNESS,
                    STATUS_CONFUSION,
                    STATUS_FEAR,
                    STATUS_PARALYSIS,
                    STATUS_STUN,
                ]
                .into_iter()
                .map(str::to_owned),
            );
            self.refresh_player_resource_maxima();
            self.mogaminator.enabled = false;
        } else if !(in_dungeon || at_entrance)
            || !matches!(phase, "route" | "battle" | "stairs-up" | "stairs-down")
        {
            return Err(CoreError::InvalidSave(
                "Angband fixture phase or location is invalid",
            ));
        }
        let mut protected = self
            .task_states
            .values()
            .filter_map(|state| {
                state
                    .random_assignment
                    .as_ref()
                    .map(|assignment| assignment.actor_kind_id.clone())
            })
            .collect::<BTreeSet<_>>();
        protected.extend(
            [
                "demo.actor.oberon-king-of-amber",
                "demo.actor.the-serpent-of-chaos",
            ]
            .into_iter()
            .map(str::to_owned),
        );
        self.entities
            .retain(|actor| protected.contains(&actor.kind_id));
        self.items.retain(|item| match &item.location {
            ItemLocation::CarriedBy { actor_id } => {
                self.entities.iter().any(|actor| &actor.id == actor_id)
            }
            _ => true,
        });
        if phase == "battle" {
            let actor = self
                .entities
                .iter()
                .find(|actor| {
                    Some(actor.id.as_str()) == target_id
                        && protected.contains(&actor.kind_id)
                        && actor.hp > 0
                })
                .ok_or(CoreError::InvalidSave(
                    "Angband battle requires a living quest target",
                ))?;
            let target = actor.position;
            self.player.position = [
                Direction::West,
                Direction::East,
                Direction::South,
                Direction::North,
                Direction::SouthWest,
                Direction::SouthEast,
                Direction::NorthWest,
                Direction::NorthEast,
            ]
            .into_iter()
            .find_map(|direction| {
                let (dx, dy) = direction.delta();
                let position = Position {
                    x: target.x + dx,
                    y: target.y + dy,
                };
                (self.index(position).is_some()
                    && self
                        .content
                        .terrain(self.terrain_at(position))
                        .is_some_and(|terrain| {
                            self.player_can_cross_terrain(terrain) && terrain.trap.is_none()
                        })
                    && !self.entities.iter().any(|actor| actor.position == position))
                .then_some(position)
            })
            .ok_or(CoreError::InvalidSave(
                "Angband battle needs a free adjacent tile",
            ))?;
        } else if phase == "arrival" || phase.starts_with("stairs-") {
            let index = self
                .terrain
                .iter()
                .position(|id| {
                    if phase == "arrival" {
                        id == "demo.terrain.angband-entrance"
                    } else {
                        id == &format!("demo.terrain.{phase}")
                    }
                })
                .ok_or(CoreError::InvalidSave(
                    "Angband requested stairs are unavailable",
                ))?;
            self.player.position = Position {
                x: (index % usize::from(self.width)) as i32,
                y: (index / usize::from(self.width)) as i32,
            };
        } else if target_id.is_some() {
            return Err(CoreError::InvalidSave("Angband route cannot name a target"));
        }
        self.player.hp = self.effective_player_max_hp();
        self.explored.fill(true);
        self.glow.fill(true);
        self.reveal_current_visibility();
        Ok(())
    }
}
