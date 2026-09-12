// SPDX-License-Identifier: MPL-2.0
use super::*;

impl Game {
    /// RD6 preparation: native IPC exposes this only in WebDriver builds.
    /// Uses verified source encounter seeds, then generates the surface once.
    #[doc(hidden)]
    pub fn debug_prepare_random_dungeon_e2e(
        &mut self,
        kind: &str,
        phase: &str,
    ) -> Result<(), CoreError> {
        let (encounter_id, seed) = match kind {
            "forest" => ("demo.wilderness-encounter.trees-random-forest-level", 1),
            "volcano" => ("demo.wilderness-encounter.lava-random-volcano-level", 6),
            "mountain" => (
                "demo.wilderness-encounter.mountain-random-mountain-level",
                301,
            ),
            "sea" => ("demo.wilderness-encounter.water-random-sea-level", 0),
            _ => return Err(CoreError::InvalidSave("unknown random dungeon fixture")),
        };
        if self.world_id != DEFAULT_WORLD_ID
            || self.map_scale != MapScaleDto::Local
            || self
                .build
                .as_ref()
                .is_none_or(|build| build.class_id != "demo.class.warrior")
        {
            return Err(CoreError::InvalidSave(
                "random dungeon fixture requires a local Warrior",
            ));
        }
        let dungeon_id = format!("demo.dungeon.random-{kind}");
        let in_dungeon = self
            .content
            .world(&self.world_id)
            .unwrap()
            .procedural_floors
            .iter()
            .any(|floor| {
                floor.id == self.current_floor_id
                    && floor.dungeon_id.as_deref() == Some(dungeon_id.as_str())
            });
        match phase {
            "arrival"
                if self.is_wilderness_floor()
                    && self.progress.level == 1
                    && self.current_dungeon_instance_id.is_none() =>
            {
                let encounter = self
                    .wilderness()
                    .encounters
                    .iter()
                    .find(|entry| entry.id == encounter_id)
                    .unwrap();
                let position = (0..self.wilderness().height)
                    .flat_map(|y| {
                        (0..self.wilderness().width).map(move |x| Position {
                            x: i32::from(x),
                            y: i32::from(y),
                        })
                    })
                    .find(|position| {
                        let entry = wilderness_legend_at(self.wilderness(), *position).unwrap();
                        !entry.road
                            && !wilderness_has_location(self.wilderness(), *position)
                            && self.visible_towns(*position).is_empty()
                            && encounter_is_eligible(
                                encounter,
                                entry.terrain,
                                self.wilderness_danger_level(*position),
                                self.wilderness_is_daytime(),
                                true,
                            )
                    })
                    .ok_or(CoreError::InvalidSave(
                        "no eligible random dungeon fixture site",
                    ))?;
                self.store_visible_town_states();
                self.wilderness_position = Some(position);
                self.wilderness_view_offset = Position::default();
                let chunk = wilderness_view_center_chunk(position, Position::default());
                // Verified by the four-type core acceptance. No injected entrance,
                // altered rarity or repeated map generation.
                self.wilderness_seed = seed;
                self.wilderness_terrain_cache.clear();
                self.activate_wilderness_position(None, false)?;
                self.player.position = self
                    .floor_connections
                    .iter()
                    .find(|connection| {
                        connection
                            .wilderness_entrance
                            .as_ref()
                            .is_some_and(|binding| {
                                binding.chunk == chunk
                                    && binding.placement.encounter_id == encounter_id
                            })
                    })
                    .ok_or(CoreError::InvalidSave(
                        "source encounter did not generate its entrance",
                    ))?
                    .position;
                self.debug_add_generated_inventory_item(
                    "e2e.random.recall",
                    "demo.item.homeward-scroll",
                    1,
                )?;
                for status in [STATUS_LEVITATION, STATUS_INVULNERABILITY] {
                    self.apply_player_melee_status(status, 200_000, "e2e.random.traversal");
                }
                self.mogaminator.enabled = false;
            }
            "route" if in_dungeon => {}
            "stairs" if in_dungeon => {
                let index = self
                    .terrain
                    .iter()
                    .position(|id| {
                        self.content.terrain(id).is_some_and(|terrain| {
                            terrain.tags.iter().any(|tag| tag == "stairs-up")
                        })
                    })
                    .ok_or(CoreError::InvalidSave(
                        "random dungeon has no actual up stairs",
                    ))?;
                self.player.position = Position {
                    x: (index % usize::from(self.width)) as i32,
                    y: (index / usize::from(self.width)) as i32,
                };
            }
            _ => {
                return Err(CoreError::InvalidSave(
                    "random dungeon fixture phase or location is invalid",
                ));
            }
        }
        self.entities.clear();
        self.items
            .retain(|item| !matches!(item.location, ItemLocation::CarriedBy { .. }));
        self.explored.fill(true);
        self.glow.fill(true);
        self.reveal_current_visibility();
        Ok(())
    }
}
