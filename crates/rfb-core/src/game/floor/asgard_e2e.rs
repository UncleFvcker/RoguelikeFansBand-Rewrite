// SPDX-License-Identifier: MPL-2.0
use crate::game::*;

const DUNGEON: &str = "demo.dungeon.asgard";
const TARGETS: [&str; 3] = [
    "demo.actor.heimdall-guardian-of-bifrost",
    "demo.actor.odin-the-all-father",
    "demo.actor.vidarr-the-silent-avenger",
];

impl Game {
    /// AS6 fixture, exposed by the native app only in its WebDriver build.
    /// Retains source guardian HP/defenses and resolves victories through combat.
    #[doc(hidden)]
    pub fn debug_prepare_asgard_e2e(
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
            || !self.dungeon_is_active(DUNGEON)
        {
            return Err(CoreError::InvalidSave(
                "Asgard fixture requires an active Norse Warrior",
            ));
        }
        let local = self
            .content
            .world(&self.world_id)
            .unwrap()
            .procedural_floors
            .iter()
            .any(|floor| {
                floor.id == self.current_floor_id && floor.dungeon_id.as_deref() == Some(DUNGEON)
            })
            || (self.is_wilderness_floor()
                && self.wilderness_position == Some(Position { x: 94, y: 11 }));
        match phase {
            "arrival"
                if target_id.is_none()
                    && self.is_wilderness_floor()
                    && self.progress.level == 1 =>
            {
                // Physical surface placement, not a transition directly into a dungeon.
                self.store_visible_town_states();
                self.wilderness_position = Some(Position { x: 94, y: 11 });
                self.wilderness_view_offset = Position::default();
                self.activate_wilderness_position(None, false)?;
                let experience = self
                    .experience_required_for_level(50)
                    .saturating_sub(self.progress.experience);
                self.apply_player_experience(experience, &mut Vec::new());
                self.debug_add_generated_inventory_item(
                    "e2e.asgard.sword",
                    "demo.item.broad-sword",
                    80,
                )?;
                let sword = self
                    .items
                    .iter_mut()
                    .find(|item| item.id == "e2e.asgard.sword")
                    .unwrap();
                sword.enchantments.to_hit = 100;
                sword.enchantments.to_damage = 100;
                self.equip_inventory_item("e2e.asgard.sword", None).ok_or(
                    CoreError::InvalidSave("Asgard fixture could not equip its sword"),
                )?;
                for (id, kind) in [
                    ("e2e.asgard.recall.1", "demo.item.homeward-scroll"),
                    ("e2e.asgard.recall.2", "demo.item.homeward-scroll"),
                ] {
                    self.debug_add_generated_inventory_item(id, kind, 80)?;
                }
                for status in [
                    STATUS_LEVITATION,
                    STATUS_INVULNERABILITY,
                    STATUS_SEE_INVISIBLE,
                ] {
                    self.apply_player_melee_status(status, 200_000, "e2e.asgard.traversal");
                }
                self.player
                    .statuses
                    .iter_mut()
                    .find(|status| status.kind_id == STATUS_INVULNERABILITY)
                    .unwrap()
                    .incoming_damage_percent = 0;
                self.refresh_player_resource_maxima();
                self.player.hp = self.player.max_hp;
                self.mogaminator.enabled = false;
            }
            "route" if local && target_id.is_none() => {}
            "battle" if local && target_id.is_some() => {
                let actor = self
                    .entities
                    .iter()
                    .find(|actor| {
                        Some(actor.id.as_str()) == target_id
                            && TARGETS.contains(&actor.kind_id.as_str())
                            && actor.hp > 0
                    })
                    .ok_or(CoreError::InvalidSave(
                        "Asgard battle fixture requires a living source guardian or avenger",
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
                    "Asgard battle fixture needs a free adjacent tile",
                ))?;
                self.player.hp = self.player.max_hp;
            }
            _ => {
                return Err(CoreError::InvalidSave(
                    "Asgard fixture phase or location is invalid",
                ));
            }
        }
        // Route clearing must retain ALL three protected kinds, including an
        // avenger that spawned before the UI could observe its first frame.
        self.entities
            .retain(|actor| TARGETS.contains(&actor.kind_id.as_str()));
        self.items.retain(|item| match &item.location {
            ItemLocation::CarriedBy { actor_id } => {
                self.entities.iter().any(|actor| &actor.id == actor_id)
            }
            _ => true,
        });
        // Keep source actor energy and statuses valid for native save/load.
        // Their ordinary AI continues during this prepared player scenario.
        self.explored.fill(true);
        self.glow.fill(true);
        // Reveal concealed doors/traps for route planning; retain their terrain
        // and use ordinary open/bash/move commands to cross them.
        for y in 0..self.height {
            for x in 0..self.width {
                let position = Position {
                    x: i32::from(x),
                    y: i32::from(y),
                };
                if self
                    .content
                    .terrain(self.terrain_at(position))
                    .is_some_and(|terrain| terrain.concealed_as_terrain_id.is_some())
                {
                    self.revealed_terrain.insert(position);
                }
            }
        }
        self.reveal_current_visibility();
        Ok(())
    }
}
