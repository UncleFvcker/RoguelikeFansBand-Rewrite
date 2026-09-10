// SPDX-License-Identifier: MPL-2.0
pub(super) mod generation;
pub(super) mod geometry;

use super::*;

const PANTHEON_TAGS: [(u8, &str, &str); 4] = [
    (1, "olympian", "olympian2"),
    (2, "egyptian", "egyptian2"),
    (3, "norse", "norse2"),
    (4, "hindu", "hindu2"),
];

impl Game {
    pub(super) fn actor_is_pantheon_suppressed(
        &self,
        actor: &rfb_content::ActorDefinition,
    ) -> bool {
        let mut member = false;
        for (id, primary, _) in PANTHEON_TAGS {
            if actor.tags.iter().any(|tag| tag == primary) {
                if self.active_pantheons & (1 << id) != 0 {
                    return false;
                }
                member = true;
            }
        }
        member
            || actor.allocation.as_ref().is_some_and(|allocation| {
                !allocation.legacy_dungeon_indices.is_empty()
                    && self.content.world(&self.world_id).is_some_and(|world| {
                        world.dungeons.iter().any(|dungeon| {
                            dungeon
                                .pantheon
                                .is_some_and(|id| self.active_pantheons & (1 << id) == 0)
                                && dungeon.legacy_index.is_some_and(|index| {
                                    allocation.legacy_dungeon_indices.contains(&index)
                                })
                        })
                    })
            })
    }

    pub(super) fn pantheon_allows_allocation(
        &self,
        floor_id: &str,
        actor: &rfb_content::ActorDefinition,
    ) -> bool {
        if self.actor_is_pantheon_suppressed(actor) {
            return false;
        }
        self.pantheon_allows_location(floor_id, actor)
    }

    pub(super) fn pantheon_allows_location(
        &self,
        floor_id: &str,
        actor: &rfb_content::ActorDefinition,
    ) -> bool {
        let Some((id, _, secondary)) = PANTHEON_TAGS
            .iter()
            .find(|(_, primary, _)| actor.tags.iter().any(|tag| tag == primary))
        else {
            return true;
        };
        // monster2.c: get_mon_num. Secondary tags alone do not confer membership.
        self.floor_dungeon(floor_id).is_some_and(|dungeon| {
            dungeon.pantheon == Some(*id)
                || (dungeon.pantheon.is_none() && actor.tags.iter().any(|tag| tag == secondary))
        })
    }

    pub(super) fn actor_matches_summon_category(
        &self,
        actor: &rfb_content::ActorDefinition,
        category: &str,
    ) -> bool {
        if let Some((id, _, _)) = PANTHEON_TAGS
            .iter()
            .find(|(_, primary, _)| *primary == category)
        {
            // mon_is_type(SUMMON_PANTHEON): an inactive requested pantheon
            // uses the ordinary unique category. Suppression is checked separately.
            return actor.tags.iter().any(|tag| tag == "unique")
                && actor.finite_lifetime_instance_limit() == Some(1)
                && (self.active_pantheons & (1 << id) == 0
                    || actor_matches_category(actor, category));
        }
        actor_matches_category(actor, category)
    }

    pub(super) fn floor_uses_arena_rooms(&self, floor_id: &str) -> bool {
        self.content.world(&self.world_id).is_some_and(|world| {
            world
                .procedural_floors
                .iter()
                .find(|floor| floor.id == floor_id)
                .and_then(|floor| floor.layout.as_ref())
                .is_some_and(|layout| layout.mode == ProceduralLayoutMode::ArenaRooms)
        })
    }

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

    pub(super) fn dungeon_has_darkness(&self) -> bool {
        self.floor_dungeon(&self.current_floor_id)
            .is_some_and(|dungeon| dungeon.darkness)
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
