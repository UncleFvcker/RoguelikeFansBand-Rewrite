// SPDX-License-Identifier: MPL-2.0

use rfb_protocol::{
    AttributeKindDto, AutoGetModeDto, Direction, GameCommand, LocaleDto, SummonCommandModeDto,
    TargetSelection,
};

use crate::{scheduler::STANDARD_ACTION_COST, stats::AttributeKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GameAction {
    AbsorbDevice {
        item_id: String,
    },
    AcceptTask {
        facility_id: String,
        task_id: String,
    },
    AbandonTask,
    AbandonPausedTask {
        task_id: String,
    },
    IncreaseAttribute {
        attribute: AttributeKind,
    },
    ChooseRaceMutation {
        reward_id: String,
        mutation_id: String,
    },
    Appraise {
        item_id: String,
    },
    BashDoor {
        direction: Direction,
    },
    BuyFromShop {
        shop_id: String,
        item_id: String,
        quantity: u32,
    },
    ClaimTaskReward {
        facility_id: String,
        task_id: String,
    },
    DepositAtHome {
        facility_id: String,
        item_id: String,
        quantity: u32,
    },
    CastAbility {
        ability_id: String,
        target: TargetSelection,
    },
    CancelAbilityDirection,
    CloseDoor {
        direction: Direction,
    },
    ConfigureMogaminator {
        enabled: bool,
        leave_destroyed_items: bool,
        auto_get_mode: AutoGetModeDto,
        locale: LocaleDto,
        source: String,
    },
    AutoGet {
        object_id: String,
    },
    ResolveMogaminatorQuery {
        item_id: String,
        pick_up: bool,
    },
    DestroyItem {
        item_id: String,
        quantity: u32,
    },
    DisarmTrap {
        direction: Direction,
    },
    DigTerrain {
        direction: Direction,
    },
    DismissPets,
    ResolveMutationDirection {
        direction: Direction,
    },
    ResolveAbilityDirection {
        direction: Direction,
    },
    Move {
        direction: Direction,
    },
    Ride {
        direction: Direction,
    },
    OpenDoor {
        direction: Direction,
    },
    InscribeItem {
        item_id: String,
        inscription: Option<String>,
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
    RefuelLight {
        target_item_id: String,
        source_item_id: String,
    },
    Search,
    SellToShop {
        shop_id: String,
        item_id: String,
        quantity: u32,
    },
    IdentifyAtFacility {
        facility_id: String,
        item_id: String,
    },
    RenameAtFacility {
        facility_id: String,
        name: String,
    },
    StayAtInn {
        facility_id: String,
    },
    TravelFromInn {
        facility_id: String,
        destination_town_id: String,
    },
    WithdrawFromHome {
        facility_id: String,
        item_id: String,
        quantity: u32,
    },
    SetSummonCommand {
        mode: SummonCommandModeDto,
    },
    SetInterfaceLocale {
        locale: LocaleDto,
    },
    ForgetAbility {
        ability_id: String,
    },
    StudyAbility {
        book_item_id: String,
        ability_id: String,
    },
    StudyPrayer {
        book_item_id: String,
    },
    Equip {
        item_id: String,
        slot_id: Option<String>,
    },
    Fire {
        direction: Direction,
    },
    FireTarget {
        target: TargetSelection,
    },
    EnterWorldMap {
        leave_pets: bool,
        cancel_recall: bool,
    },
    LeaveWorldMap,
    TravelWorld {
        destination: rfb_protocol::Position,
    },
    TravelLocal {
        destination: rfb_protocol::Position,
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
            Self::BuyFromShop { .. }
            | Self::AcceptTask { .. }
            | Self::ClaimTaskReward { .. }
            | Self::DepositAtHome { .. }
            | Self::DismissPets
            | Self::IncreaseAttribute { .. }
            | Self::ChooseRaceMutation { .. }
            | Self::EnterWorldMap { .. }
            | Self::LeaveWorldMap
            | Self::Retire
            | Self::SellToShop { .. }
            | Self::IdentifyAtFacility { .. }
            | Self::RenameAtFacility { .. }
            | Self::StayAtInn { .. }
            | Self::TravelFromInn { .. }
            | Self::WithdrawFromHome { .. }
            | Self::SetSummonCommand { .. }
            | Self::ConfigureMogaminator { .. }
            | Self::AutoGet { .. }
            | Self::PickUp
            | Self::ResolveMogaminatorQuery { .. }
            | Self::ResolveMutationDirection { .. }
            | Self::CancelAbilityDirection
            | Self::InscribeItem { .. }
            | Self::SetInterfaceLocale { .. } => 0,
            Self::TravelLocal { .. } => 0,
            Self::RefuelLight { .. } => STANDARD_ACTION_COST / 2,
            _ => STANDARD_ACTION_COST,
        }
    }
}

impl From<GameCommand> for GameAction {
    fn from(command: GameCommand) -> Self {
        match command {
            GameCommand::AbsorbDevice { item_id } => Self::AbsorbDevice { item_id },
            GameCommand::AcceptTask {
                facility_id,
                task_id,
            } => Self::AcceptTask {
                facility_id,
                task_id,
            },
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
            GameCommand::ChooseRaceMutation {
                reward_id,
                mutation_id,
            } => Self::ChooseRaceMutation {
                reward_id,
                mutation_id,
            },
            GameCommand::Appraise { item_id } => Self::Appraise { item_id },
            GameCommand::BashDoor { direction } => Self::BashDoor { direction },
            GameCommand::BuyFromShop {
                shop_id,
                item_id,
                quantity,
            } => Self::BuyFromShop {
                shop_id,
                item_id,
                quantity,
            },
            GameCommand::ClaimTaskReward {
                facility_id,
                task_id,
            } => Self::ClaimTaskReward {
                facility_id,
                task_id,
            },
            GameCommand::DepositAtHome {
                facility_id,
                item_id,
                quantity,
            } => Self::DepositAtHome {
                facility_id,
                item_id,
                quantity,
            },
            GameCommand::CastAbility { ability_id, target } => {
                Self::CastAbility { ability_id, target }
            }
            GameCommand::CancelAbilityDirection => Self::CancelAbilityDirection,
            GameCommand::CloseDoor { direction } => Self::CloseDoor { direction },
            GameCommand::ConfigureMogaminator {
                enabled,
                leave_destroyed_items,
                auto_get_mode,
                locale,
                source,
            } => Self::ConfigureMogaminator {
                enabled,
                leave_destroyed_items,
                auto_get_mode,
                locale,
                source,
            },
            GameCommand::AutoGet { object_id } => Self::AutoGet { object_id },
            GameCommand::ResolveMogaminatorQuery { item_id, pick_up } => {
                Self::ResolveMogaminatorQuery { item_id, pick_up }
            }
            GameCommand::DestroyItem { item_id, quantity } => {
                Self::DestroyItem { item_id, quantity }
            }
            GameCommand::DisarmTrap { direction } => Self::DisarmTrap { direction },
            GameCommand::DigTerrain { direction } => Self::DigTerrain { direction },
            GameCommand::DismissPets => Self::DismissPets,
            GameCommand::ResolveMutationDirection { direction } => {
                Self::ResolveMutationDirection { direction }
            }
            GameCommand::ResolveAbilityDirection { direction } => {
                Self::ResolveAbilityDirection { direction }
            }
            GameCommand::EnterWorldMap {
                leave_pets,
                cancel_recall,
            } => Self::EnterWorldMap {
                leave_pets,
                cancel_recall,
            },
            GameCommand::LeaveWorldMap => Self::LeaveWorldMap,
            GameCommand::TravelWorld { destination } => Self::TravelWorld { destination },
            GameCommand::TravelLocal { destination } => Self::TravelLocal { destination },
            GameCommand::Move { direction } => Self::Move { direction },
            GameCommand::Ride { direction } => Self::Ride { direction },
            GameCommand::OpenDoor { direction } => Self::OpenDoor { direction },
            GameCommand::InscribeItem {
                item_id,
                inscription,
            } => Self::InscribeItem {
                item_id,
                inscription,
            },
            GameCommand::Wait => Self::Wait,
            GameCommand::PickUp => Self::PickUp,
            GameCommand::Retire => Self::Retire,
            GameCommand::Rest { turns } => Self::Rest { turns },
            GameCommand::RefuelLight {
                target_item_id,
                source_item_id,
            } => Self::RefuelLight {
                target_item_id,
                source_item_id,
            },
            GameCommand::Search => Self::Search,
            GameCommand::SellToShop {
                shop_id,
                item_id,
                quantity,
            } => Self::SellToShop {
                shop_id,
                item_id,
                quantity,
            },
            GameCommand::IdentifyAtFacility {
                facility_id,
                item_id,
            } => Self::IdentifyAtFacility {
                facility_id,
                item_id,
            },
            GameCommand::RenameAtFacility { facility_id, name } => {
                Self::RenameAtFacility { facility_id, name }
            }
            GameCommand::StayAtInn { facility_id } => Self::StayAtInn { facility_id },
            GameCommand::TravelFromInn {
                facility_id,
                destination_town_id,
            } => Self::TravelFromInn {
                facility_id,
                destination_town_id,
            },
            GameCommand::WithdrawFromHome {
                facility_id,
                item_id,
                quantity,
            } => Self::WithdrawFromHome {
                facility_id,
                item_id,
                quantity,
            },
            GameCommand::SetSummonCommand { mode } => Self::SetSummonCommand { mode },
            GameCommand::SetInterfaceLocale { locale } => Self::SetInterfaceLocale { locale },
            GameCommand::ForgetAbility { ability_id } => Self::ForgetAbility { ability_id },
            GameCommand::StudyAbility {
                book_item_id,
                ability_id,
            } => Self::StudyAbility {
                book_item_id,
                ability_id,
            },
            GameCommand::StudyPrayer { book_item_id } => Self::StudyPrayer { book_item_id },
            GameCommand::Equip { item_id, slot_id } => Self::Equip { item_id, slot_id },
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
