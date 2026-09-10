// SPDX-License-Identifier: MPL-2.0
pub(super) mod generation;
pub(super) mod geometry;

use super::*;

impl Game {
    pub(super) fn floor_dungeon(&self, floor_id: &str) -> Option<&rfb_content::DungeonDefinition> {
        let world = self.content.world(&self.world_id)?;
        let dungeon_id = world
            .procedural_floors
            .iter()
            .find(|floor| floor.id == floor_id)?
            .dungeon_id
            .as_deref()?;
        world
            .dungeons
            .iter()
            .find(|dungeon| dungeon.id == dungeon_id)
    }

    pub(super) fn dungeon_blocks_magic(&self) -> bool {
        self.floor_dungeon(&self.current_floor_id)
            .is_some_and(|dungeon| dungeon.no_magic)
    }

    pub(super) fn dungeon_allows_monster(
        &self,
        floor_id: &str,
        actor: &rfb_content::ActorDefinition,
        player_summon: bool,
    ) -> bool {
        let Some(dungeon) = self.floor_dungeon(floor_id) else {
            return true;
        };
        if actor
            .allocation
            .as_ref()
            .is_some_and(|allocation| allocation.legacy_index == 1040)
        {
            return true;
        }
        // monster2.c: restrict_monster_to_dungeon. Pets are enabled in this game;
        // the exception follows SUMMON_WHO_PLAYER, even when its result is hostile.
        (!dungeon.no_magic || actor.tags.iter().any(|tag| tag == "innate-spell"))
            && (!dungeon.no_melee
                || player_summon
                || actor.tags.iter().any(|tag| tag == "attack-spell"))
    }

    pub(super) fn dungeon_blocks_melee(&self) -> bool {
        self.floor_dungeon(&self.current_floor_id)
            .is_some_and(|dungeon| dungeon.no_melee)
    }
}
