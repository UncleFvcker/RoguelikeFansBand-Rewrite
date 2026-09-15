// SPDX-License-Identifier: MPL-2.0

use rfb_protocol::{
    AttributeKindDto, AutoGetModeDto, BountyOfficeActionDto, Direction, FacilityServiceKindDto,
    GameCommand, LocaleDto, SummonCommandModeDto, TargetSelection,
};

use crate::{scheduler::STANDARD_ACTION_COST, stats::AttributeKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RestMode {
    Turns,
    Resources,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GameAction {
    Casino {
        facility_id: String,
        action: rfb_protocol::CasinoActionDto,
    },
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
    ChooseMaiaPath {
        path: rfb_protocol::MaiaPathDto,
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
    ClearDuelistChallenge,
    ResolveDuelistChoice {
        choice: rfb_protocol::DuelistChoiceDto,
    },
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
    DismissPet {
        actor_id: String,
    },
    SetPetTarget {
        actor_id: Option<String>,
    },
    SetPetOption {
        option: rfb_protocol::PetOptionDto,
        enabled: bool,
    },
    SetPetName {
        actor_id: String,
        name: Option<String>,
    },
    ResolveMutationDirection {
        direction: Direction,
    },
    ResolveAbilityDirection {
        direction: Direction,
    },
    Move {
        direction: Direction,
        flip_pickup: bool,
    },
    Ride {
        direction: Direction,
    },
    OpenChest {
        item_id: String,
    },
    DisarmChest {
        item_id: String,
    },
    OpenDoor {
        direction: Direction,
    },
    InscribeItem {
        item_id: String,
        inscription: Option<String>,
    },
    SelectMagicAbsorptionSlot {
        slot: u8,
    },
    ResolveMagicAbsorption {
        confirm: bool,
        inherit_inscription: bool,
    },
    SwapAbsorbedDevices {
        category: rfb_protocol::AbsorbedDeviceCategoryDto,
        first_slot: u8,
        second_slot: u8,
    },
    /// Internal action substituted when paralysis wastes the player's turn.
    /// No command maps to it; it advances world time at standard cost.
    ParalyzedIdle,
    Wait,
    Stay,
    SwapRings {
        first_slot_id: String,
        second_slot_id: String,
    },
    ContinueFishing,
    CancelFishing,
    PickUp,
    Retire,
    EndCharacter,
    Rest {
        turns: u16,
        mode: RestMode,
    },
    RefuelLight {
        target_item_id: String,
        source_item_id: String,
    },
    Search,
    ToggleSearch,
    Alter {
        direction: Direction,
    },
    SpikeDoor {
        direction: Direction,
    },
    // Internal selection by directional interactions; attacks without walking or confusing direction twice.
    AttackAdjacent {
        direction: Direction,
    },
    SellToShop {
        shop_id: String,
        item_id: String,
        quantity: u32,
    },
    IdentifyAtFacility {
        facility_id: String,
        item_id: String,
    },
    ResearchItemAtFacility {
        facility_id: String,
        item_id: String,
    },
    ResearchMonsterAtFacility {
        facility_id: String,
        actor_kind_id: String,
    },
    TeleportToDungeonLevelAtFacility {
        facility_id: String,
        dungeon_id: String,
        depth: u16,
    },
    EatAtInn {
        facility_id: String,
    },
    AskReputationAtInn {
        facility_id: String,
    },
    IdentifyAllAtFacility {
        facility_id: String,
    },
    UseFacilityService {
        facility_id: String,
        service: FacilityServiceKindDto,
        item_id: Option<String>,
        enchantment_steps: Option<u8>,
    },
    UseBountyOffice {
        facility_id: String,
        action: BountyOfficeActionDto,
        item_id: Option<String>,
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
    ConfigureTravel {
        options: rfb_protocol::TravelOptionsDto,
    },
    ConfigureMogaminatorPreferences {
        preferences: rfb_protocol::MogaminatorPreferencesDto,
    },
    ConfigurePreferences {
        preferences: rfb_protocol::BehaviorPreferencesDto,
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
    BeginRealmChange {
        book_item_id: String,
    },
    ResolveRealmChange {
        confirm: bool,
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
        cancel_recall: bool,
    },
    LeaveWorldMap,
    TravelWorld {
        destination: rfb_protocol::Position,
    },
    Run {
        direction: Direction,
        max_steps: u16,
    },
    ContinueRun,
    CancelRun,
    AutoExplore,
    ContinueAutoExplore,
    CancelAutoExplore,
    TravelLocal {
        destination: rfb_protocol::Position,
    },
    FindNearestUnknownItem,
    TravelUnknownItem {
        object_id: String,
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
    UseAbsorbedDevice {
        item_id: String,
        targets: Vec<TargetSelection>,
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
            | Self::BeginRealmChange { .. }
            | Self::ResolveRealmChange { .. }
            | Self::AcceptTask { .. }
            | Self::ClaimTaskReward { .. }
            | Self::DepositAtHome { .. }
            | Self::DismissPets
            | Self::DismissPet { .. }
            | Self::SetPetTarget { .. }
            | Self::SetPetOption { .. }
            | Self::SetPetName { .. }
            | Self::IncreaseAttribute { .. }
            | Self::ChooseRaceMutation { .. }
            | Self::ChooseMaiaPath { .. }
            | Self::EnterWorldMap { .. }
            | Self::LeaveWorldMap
            | Self::Retire
            | Self::EndCharacter
            | Self::SellToShop { .. }
            | Self::IdentifyAtFacility { .. }
            | Self::ResearchItemAtFacility { .. }
            | Self::ResearchMonsterAtFacility { .. }
            | Self::TeleportToDungeonLevelAtFacility { .. }
            | Self::EatAtInn { .. }
            | Self::AskReputationAtInn { .. }
            | Self::IdentifyAllAtFacility { .. }
            | Self::Casino { .. }
            | Self::UseFacilityService { .. }
            | Self::UseBountyOffice { .. }
            | Self::RenameAtFacility { .. }
            | Self::StayAtInn { .. }
            | Self::TravelFromInn { .. }
            | Self::WithdrawFromHome { .. }
            | Self::SetSummonCommand { .. }
            | Self::ConfigureMogaminator { .. }
            | Self::AutoGet { .. }
            | Self::PickUp
            | Self::SwapRings { .. }
            | Self::ResolveMogaminatorQuery { .. }
            | Self::ResolveMutationDirection { .. }
            | Self::CancelAbilityDirection
            | Self::ToggleSearch
            | Self::CancelFishing
            | Self::ClearDuelistChallenge
            | Self::ResolveDuelistChoice { .. }
            | Self::InscribeItem { .. }
            | Self::SwapAbsorbedDevices { .. }
            | Self::SetInterfaceLocale { .. }
            | Self::ConfigureTravel { .. }
            | Self::ConfigurePreferences { .. }
            | Self::ConfigureMogaminatorPreferences { .. } => 0,
            Self::TravelLocal { .. }
            | Self::FindNearestUnknownItem
            | Self::TravelUnknownItem { .. }
            | Self::Run { .. }
            | Self::ContinueRun
            | Self::CancelRun
            | Self::AutoExplore
            | Self::ContinueAutoExplore
            | Self::CancelAutoExplore => 0,
            Self::RefuelLight { .. } => STANDARD_ACTION_COST / 2,
            _ => STANDARD_ACTION_COST,
        }
    }
}

impl From<GameCommand> for GameAction {
    fn from(command: GameCommand) -> Self {
        match command {
            GameCommand::SelectMagicAbsorptionSlot { slot } => {
                Self::SelectMagicAbsorptionSlot { slot }
            }
            GameCommand::ResolveMagicAbsorption {
                confirm,
                inherit_inscription,
            } => Self::ResolveMagicAbsorption {
                confirm,
                inherit_inscription,
            },
            GameCommand::SwapAbsorbedDevices {
                category,
                first_slot,
                second_slot,
            } => Self::SwapAbsorbedDevices {
                category,
                first_slot,
                second_slot,
            },
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
            GameCommand::ChooseMaiaPath { path } => Self::ChooseMaiaPath { path },
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
            GameCommand::ClearDuelistChallenge => Self::ClearDuelistChallenge,
            GameCommand::ResolveDuelistChoice { choice } => Self::ResolveDuelistChoice { choice },
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
            GameCommand::DismissPet { actor_id } => Self::DismissPet { actor_id },
            GameCommand::SetPetTarget { actor_id } => Self::SetPetTarget { actor_id },
            GameCommand::SetPetOption { option, enabled } => Self::SetPetOption { option, enabled },
            GameCommand::SetPetName { actor_id, name } => Self::SetPetName { actor_id, name },
            GameCommand::ResolveMutationDirection { direction } => {
                Self::ResolveMutationDirection { direction }
            }
            GameCommand::ResolveAbilityDirection { direction } => {
                Self::ResolveAbilityDirection { direction }
            }
            GameCommand::EnterWorldMap { cancel_recall } => Self::EnterWorldMap { cancel_recall },
            GameCommand::LeaveWorldMap => Self::LeaveWorldMap,
            GameCommand::TravelWorld { destination } => Self::TravelWorld { destination },
            GameCommand::Run {
                direction,
                max_steps,
            } => Self::Run {
                direction,
                max_steps: max_steps.unwrap_or(1000),
            },
            GameCommand::ContinueRun => Self::ContinueRun,
            GameCommand::CancelRun => Self::CancelRun,
            GameCommand::AutoExplore => Self::AutoExplore,
            GameCommand::ContinueAutoExplore => Self::ContinueAutoExplore,
            GameCommand::CancelAutoExplore => Self::CancelAutoExplore,
            GameCommand::TravelLocal { destination } => Self::TravelLocal { destination },
            GameCommand::FindNearestUnknownItem => Self::FindNearestUnknownItem,
            GameCommand::TravelUnknownItem {
                object_id,
                destination,
            } => Self::TravelUnknownItem {
                object_id,
                destination,
            },
            GameCommand::Move { direction } => Self::Move {
                direction,
                flip_pickup: false,
            },
            GameCommand::WalkSpecial { direction } => Self::Move {
                direction,
                flip_pickup: true,
            },
            GameCommand::Ride { direction } => Self::Ride { direction },
            GameCommand::OpenChest { item_id } => Self::OpenChest { item_id },
            GameCommand::DisarmChest { item_id } => Self::DisarmChest { item_id },
            GameCommand::OpenDoor { direction } => Self::OpenDoor { direction },
            GameCommand::InscribeItem {
                item_id,
                inscription,
            } => Self::InscribeItem {
                item_id,
                inscription,
            },
            GameCommand::Wait => Self::Wait,
            GameCommand::Stay => Self::Stay,
            GameCommand::SwapRings {
                first_slot_id,
                second_slot_id,
            } => Self::SwapRings {
                first_slot_id,
                second_slot_id,
            },
            GameCommand::ContinueFishing => Self::ContinueFishing,
            GameCommand::CancelFishing => Self::CancelFishing,
            GameCommand::PickUp => Self::PickUp,
            GameCommand::Retire => Self::Retire,
            GameCommand::EndCharacter => Self::EndCharacter,
            GameCommand::Rest { turns } => Self::Rest {
                turns,
                mode: RestMode::Complete,
            },
            GameCommand::RestForTurns { turns } => Self::Rest {
                turns,
                mode: RestMode::Turns,
            },
            GameCommand::RestUntilResources { turns } => Self::Rest {
                turns,
                mode: RestMode::Resources,
            },
            GameCommand::RefuelLight {
                target_item_id,
                source_item_id,
            } => Self::RefuelLight {
                target_item_id,
                source_item_id,
            },
            GameCommand::Search => Self::Search,
            GameCommand::ToggleSearch => Self::ToggleSearch,
            GameCommand::Alter { direction } => Self::Alter { direction },
            GameCommand::SpikeDoor { direction } => Self::SpikeDoor { direction },
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
            GameCommand::ResearchItemAtFacility {
                facility_id,
                item_id,
            } => Self::ResearchItemAtFacility {
                facility_id,
                item_id,
            },
            GameCommand::IdentifyAllAtFacility { facility_id } => {
                Self::IdentifyAllAtFacility { facility_id }
            }
            GameCommand::Casino {
                facility_id,
                action,
            } => Self::Casino {
                facility_id,
                action,
            },
            GameCommand::UseFacilityService {
                facility_id,
                service,
                item_id,
                enchantment_steps,
            } => Self::UseFacilityService {
                facility_id,
                service,
                item_id,
                enchantment_steps,
            },
            GameCommand::UseBountyOffice {
                facility_id,
                action,
                item_id,
            } => Self::UseBountyOffice {
                facility_id,
                action,
                item_id,
            },
            GameCommand::RenameAtFacility { facility_id, name } => {
                Self::RenameAtFacility { facility_id, name }
            }
            GameCommand::StayAtInn { facility_id } => Self::StayAtInn { facility_id },
            GameCommand::EatAtInn { facility_id } => Self::EatAtInn { facility_id },
            GameCommand::AskReputationAtInn { facility_id } => {
                Self::AskReputationAtInn { facility_id }
            }
            GameCommand::ResearchMonsterAtFacility {
                facility_id,
                actor_kind_id,
            } => Self::ResearchMonsterAtFacility {
                facility_id,
                actor_kind_id,
            },
            GameCommand::TeleportToDungeonLevelAtFacility {
                facility_id,
                dungeon_id,
                depth,
            } => Self::TeleportToDungeonLevelAtFacility {
                facility_id,
                dungeon_id,
                depth,
            },
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
            GameCommand::ConfigureTravel { options } => Self::ConfigureTravel { options },
            GameCommand::ConfigureMogaminatorPreferences { preferences } => {
                Self::ConfigureMogaminatorPreferences { preferences }
            }
            GameCommand::ConfigurePreferences { preferences } => {
                Self::ConfigurePreferences { preferences }
            }
            GameCommand::ForgetAbility { ability_id } => Self::ForgetAbility { ability_id },
            GameCommand::StudyAbility {
                book_item_id,
                ability_id,
            } => Self::StudyAbility {
                book_item_id,
                ability_id,
            },
            GameCommand::StudyPrayer { book_item_id } => Self::StudyPrayer { book_item_id },
            GameCommand::BeginRealmChange { book_item_id } => {
                Self::BeginRealmChange { book_item_id }
            }
            GameCommand::ResolveRealmChange { confirm } => Self::ResolveRealmChange { confirm },
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
            GameCommand::UseAbsorbedDevice { item_id, targets } => {
                Self::UseAbsorbedDevice { item_id, targets }
            }
            GameCommand::UseItemForRecharge {
                item_id,
                source_item_id,
                target_item_id,
            } => Self::UseItem {
                item_id,
                target: Some(TargetSelection::RechargeItems {
                    source_item_id,
                    target_item_id,
                }),
                target_glyph: None,
            },
            GameCommand::Unequip { slot_id } => Self::Unequip { slot_id },
            GameCommand::Drop { item_ids } => Self::Drop { item_ids },
            GameCommand::DropQuantity { item_id, quantity } => {
                Self::DropQuantity { item_id, quantity }
            }
        }
    }
}
