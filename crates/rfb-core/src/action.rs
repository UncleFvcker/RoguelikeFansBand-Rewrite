// SPDX-License-Identifier: MPL-2.0

use rfb_protocol::{
    AttributeKindDto, DeviceRechargeSourceDto, Direction, GameCommand, SummonCommandModeDto,
    TargetSelection,
};

use crate::{scheduler::STANDARD_ACTION_COST, stats::AttributeKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GameAction {
    AbandonTask,
    AbandonPausedTask {
        task_id: String,
    },
    IncreaseAttribute {
        attribute: AttributeKind,
    },
    Appraise {
        item_id: String,
    },
    BashDoor {
        direction: Direction,
    },
    CastAbility {
        ability_id: String,
        target: TargetSelection,
    },
    CloseDoor {
        direction: Direction,
    },
    DisarmTrap {
        direction: Direction,
    },
    DigTerrain {
        direction: Direction,
    },
    Move {
        direction: Direction,
    },
    OpenDoor {
        direction: Direction,
    },
    /// Internal action substituted when paralysis wastes the player's turn.
    /// No command maps to it; it advances world time at standard cost.
    ParalyzedIdle,
    Wait,
    PickUp,
    Retire,
    Rest {
        turns: u16,
    },
    RechargeItem {
        target_item_id: String,
        source: DeviceRechargeSourceDto,
    },
    Search,
    SetSummonCommand {
        mode: SummonCommandModeDto,
    },
    ForgetAbility {
        ability_id: String,
    },
    StudyAbility {
        book_item_id: String,
        ability_id: String,
    },
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
    TraverseStairs,
    UseItem {
        item_id: String,
        target: Option<TargetSelection>,
        target_glyph: Option<String>,
    },
    UseItemForRecharge {
        item_id: String,
        source_item_id: String,
        target_item_id: String,
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
        match self {
            Self::IncreaseAttribute { .. } | Self::Retire | Self::SetSummonCommand { .. } => 0,
            _ => STANDARD_ACTION_COST,
        }
    }
}

impl From<GameCommand> for GameAction {
    fn from(command: GameCommand) -> Self {
        match command {
            GameCommand::AbandonTask => Self::AbandonTask,
            GameCommand::AbandonPausedTask { task_id } => Self::AbandonPausedTask { task_id },
            GameCommand::IncreaseAttribute { attribute } => Self::IncreaseAttribute {
                attribute: match attribute {
                    AttributeKindDto::Strength => AttributeKind::Strength,
                    AttributeKindDto::Intelligence => AttributeKind::Intelligence,
                    AttributeKindDto::Wisdom => AttributeKind::Wisdom,
                    AttributeKindDto::Dexterity => AttributeKind::Dexterity,
                    AttributeKindDto::Constitution => AttributeKind::Constitution,
                    AttributeKindDto::Charisma => AttributeKind::Charisma,
                },
            },
            GameCommand::Appraise { item_id } => Self::Appraise { item_id },
            GameCommand::BashDoor { direction } => Self::BashDoor { direction },
            GameCommand::CastAbility { ability_id, target } => {
                Self::CastAbility { ability_id, target }
            }
            GameCommand::CloseDoor { direction } => Self::CloseDoor { direction },
            GameCommand::DisarmTrap { direction } => Self::DisarmTrap { direction },
            GameCommand::DigTerrain { direction } => Self::DigTerrain { direction },
            GameCommand::Move { direction } => Self::Move { direction },
            GameCommand::OpenDoor { direction } => Self::OpenDoor { direction },
            GameCommand::Wait => Self::Wait,
            GameCommand::PickUp => Self::PickUp,
            GameCommand::Retire => Self::Retire,
            GameCommand::Rest { turns } => Self::Rest { turns },
            GameCommand::RechargeItem {
                target_item_id,
                source,
            } => Self::RechargeItem {
                target_item_id,
                source,
            },
            GameCommand::Search => Self::Search,
            GameCommand::SetSummonCommand { mode } => Self::SetSummonCommand { mode },
            GameCommand::ForgetAbility { ability_id } => Self::ForgetAbility { ability_id },
            GameCommand::StudyAbility {
                book_item_id,
                ability_id,
            } => Self::StudyAbility {
                book_item_id,
                ability_id,
            },
            GameCommand::Equip { item_id } => Self::Equip { item_id },
            GameCommand::Fire { direction } => Self::Fire { direction },
            GameCommand::FireTarget { target } => Self::FireTarget { target },
            GameCommand::Throw { item_id, direction } => Self::Throw { item_id, direction },
            GameCommand::TraverseStairs => Self::TraverseStairs,
            GameCommand::UseItem { item_id, target } => Self::UseItem {
                item_id,
                target,
                target_glyph: None,
            },
            GameCommand::UseItemByGlyph { item_id, glyph } => Self::UseItem {
                item_id,
                target: None,
                target_glyph: Some(glyph),
            },
            GameCommand::UseItemForRecharge {
                item_id,
                source_item_id,
                target_item_id,
            } => Self::UseItemForRecharge {
                item_id,
                source_item_id,
                target_item_id,
            },
            GameCommand::Unequip { slot_id } => Self::Unequip { slot_id },
            GameCommand::Drop { item_ids } => Self::Drop { item_ids },
            GameCommand::DropQuantity { item_id, quantity } => {
                Self::DropQuantity { item_id, quantity }
            }
        }
    }
}
