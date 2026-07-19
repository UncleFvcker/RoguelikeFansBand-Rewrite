// SPDX-License-Identifier: MPL-2.0

use rfb_protocol::{Direction, GameCommand, TargetSelection};

use crate::scheduler::STANDARD_ACTION_COST;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GameAction {
    Appraise {
        item_id: String,
    },
    Move {
        direction: Direction,
    },
    Wait,
    PickUp,
    Equip {
        item_id: String,
    },
    Fire {
        direction: Direction,
    },
    FireTarget {
        target: TargetSelection,
    },
    Throw {
        item_id: String,
        direction: Direction,
    },
    UseItem {
        item_id: String,
    },
    Unequip {
        slot_id: String,
    },
    Drop {
        item_ids: Vec<String>,
    },
    DropQuantity {
        item_id: String,
        quantity: u32,
    },
}

impl GameAction {
    pub(crate) const fn energy_cost(&self) -> i32 {
        STANDARD_ACTION_COST
    }
}

impl From<GameCommand> for GameAction {
    fn from(command: GameCommand) -> Self {
        match command {
            GameCommand::Appraise { item_id } => Self::Appraise { item_id },
            GameCommand::Move { direction } => Self::Move { direction },
            GameCommand::Wait => Self::Wait,
            GameCommand::PickUp => Self::PickUp,
            GameCommand::Equip { item_id } => Self::Equip { item_id },
            GameCommand::Fire { direction } => Self::Fire { direction },
            GameCommand::FireTarget { target } => Self::FireTarget { target },
            GameCommand::Throw { item_id, direction } => Self::Throw { item_id, direction },
            GameCommand::UseItem { item_id } => Self::UseItem { item_id },
            GameCommand::Unequip { slot_id } => Self::Unequip { slot_id },
            GameCommand::Drop { item_ids } => Self::Drop { item_ids },
            GameCommand::DropQuantity { item_id, quantity } => {
                Self::DropQuantity { item_id, quantity }
            }
        }
    }
}
