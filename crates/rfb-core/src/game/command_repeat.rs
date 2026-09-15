// SPDX-License-Identifier: MPL-2.0
use super::*;

// RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c, cmd2.c command_rep/more.
// The UI owns the count; Core decides whether another identical command is useful and safe.
#[derive(Clone, Copy)]
pub(super) enum CommandRepeatKind {
    None,
    Move,
    Wait,
    Search,
    Terrain,
}

impl CommandRepeatKind {
    pub(super) fn for_action(action: &GameAction) -> Self {
        match action {
            GameAction::Move { .. } => Self::Move,
            GameAction::Wait | GameAction::Stay => Self::Wait,
            GameAction::Search => Self::Search,
            GameAction::Alter { .. }
            | GameAction::OpenDoor { .. }
            | GameAction::BashDoor { .. }
            | GameAction::DigTerrain { .. }
            | GameAction::DisarmTrap { .. }
            | GameAction::OpenChest { .. }
            | GameAction::DisarmChest { .. } => Self::Terrain,
            _ => Self::None,
        }
    }

    pub(super) fn can_repeat(
        self,
        game: &Game,
        events: &[rfb_protocol::GameEventDto],
        moved: bool,
    ) -> bool {
        if matches!(self, Self::None)
            || game.player_is_dead()
            || game.visible_hostile_exists()
            || game.player_has_status_kind(STATUS_PARALYSIS)
            || game.player_has_status_kind(STATUS_CONFUSION)
            || game.mogaminator.pending_query.is_some()
            || game.pending_duelist.is_some()
            || game.pending_mutation_direction.is_some()
            || game.pending_ability_direction.is_some()
            || events.iter().any(|event| {
                event.kind.starts_with("combat.")
                    || event.kind == "status.player-damage"
                    || matches!(
                        event.kind.as_str(),
                        "riding.moved" | "riding.direction-changed" | "riding.control-unavailable"
                    )
                    || event.kind == "hunger.starvation-damage"
                    || event.kind.ends_with("-damaged")
                    || event.kind == "terrain.secret-discovered"
                    || event.kind == "terrain.trap-triggered"
                    || event.kind == "item.pickup.inventory-full"
                    || event.message_key == "chest-trap-found"
            })
        {
            return false;
        }
        match self {
            Self::None => false,
            Self::Move | Self::Wait => {
                let at_service = game
                    .content
                    .terrain(game.terrain_at(game.player.position))
                    .is_some_and(|terrain| {
                        terrain.tags.iter().any(|tag| {
                            matches!(
                                tag.as_str(),
                                "shop-entrance" | "town-facility-entrance" | "task-entry"
                            )
                        })
                    });
                (!matches!(self, Self::Move) || moved) && !at_service
            }
            Self::Search => true,
            Self::Terrain => events.iter().any(|event| {
                matches!(
                    event.kind.as_str(),
                    "terrain.door-unlock-failed"
                        | "terrain.door-bash-failed"
                        | "terrain.trap-disarm-failed"
                ) || (event.kind == "terrain.dig-failed"
                    && event
                        .args
                        .get("retryable")
                        .is_some_and(|value| value == "true"))
                    || matches!(
                        event.message_key.as_str(),
                        "chest-unlock-failed" | "chest-disarm-failed"
                    )
            }),
        }
    }
}
