// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeMap;

use rfb_protocol::{
    AbilityAreaDamageResolutionDto, AbilityBeamDamageResolutionDto, AbilityCastResolutionDto,
    AbilityConeDamageResolutionDto, AbilityDetectResolutionDto, AbilityEffectsResolutionDto,
    AbilityMonsterProbeResolutionDto, AbilityProbeAlignmentDto, AbilityProbeTargetDto,
    AbilitySummonResolutionDto, AbilityTeleportResolutionDto, AbilityTerrainTransformResolutionDto,
    AbilityVisibleDamageResolutionDto, CheckResolutionDto, Direction, GameEventDto,
    GameEventOutcomeDto, HealingResolutionDto, ItemCurseRemovalResolutionDto,
    ItemCurseResolutionDto, ItemCurseSeverityDto, ItemEnchantmentResolutionDto,
    ItemIdentifyResolutionDto, ItemQualityDto, MonsterAbilityCastResolutionDto,
    MonsterAbilityDecisionResolutionDto, MonsterDisplacementResolutionDto, Position,
    ProjectileTraceDto, ResourceRecoveryResolutionDto, RestResolutionDto, RestStopReasonDto,
    SummonCommandModeDto, SummonCommandResolutionDto,
};

use crate::{
    effect::DamageOutcome,
    game::town::{
        FacilityIdentifyOutcome, FacilityRenameOutcome, InnStayOutcome, InnTravelOutcome,
        ShopTransactionOutcome,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ItemAttributeChange {
    Drained,
    Restored,
    Increased,
    Sustained,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectileTrace {
    pub(crate) origin: Position,
    pub(crate) impact: Position,
    pub(crate) landing: Position,
    pub(crate) traversed: Vec<Position>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelfKnowledgeReport {
    pub(crate) level: u16,
    pub(crate) hp: i32,
    pub(crate) max_hp: i32,
    pub(crate) gold: u32,
    pub(crate) nutrition: u16,
    pub(crate) attack: i32,
    pub(crate) defense: i32,
    pub(crate) melee_skill: i32,
    pub(crate) armor_class: i32,
    pub(crate) speed: u16,
    pub(crate) attributes: [String; 6],
    pub(crate) statuses: String,
    pub(crate) resistances: String,
    pub(crate) resources: String,
}

impl From<ProjectileTrace> for ProjectileTraceDto {
    fn from(trace: ProjectileTrace) -> Self {
        Self {
            origin: trace.origin,
            impact: trace.impact,
            landing: trace.landing,
            traversed: trace.traversed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BoltReflectionOutcome {
    Landed,
    Hit {
        target_kind_id: String,
        damage: DamageOutcome,
        fatal: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DomainEvent {
    #[allow(dead_code)]
    MutationGained {
        mutation_id: String,
        name: String,
    },
    #[allow(dead_code)]
    MutationLost {
        mutation_id: String,
        name: String,
    },
    MutationAllCured,
    MutationPeriodicTriggered {
        mutation_id: String,
        name: String,
    },
    MutationFumbled {
        damage: DamageOutcome,
        dropped_item_kind_id: Option<String>,
    },
    MutationWarning {
        danger_amount: u32,
    },
    ChaosPatronReward {
        patron_id: String,
        patron_name: String,
    },
    RealityChangeResolved {
        regenerated: bool,
    },
    AbilityStudied {
        ability_id: String,
    },
    AbilityForgotten {
        ability_id: String,
    },
    AbilityForgetUnavailable {
        ability_id: String,
        reason: String,
    },
    AbilityStudyUnavailable {
        target_id: String,
        reason: String,
    },
    AbilityCastUnavailable {
        ability_id: String,
        reason: String,
    },
    AbilityCastFailed {
        resolution: AbilityCastResolutionDto,
    },
    AbilityCastSucceeded {
        resolution: AbilityCastResolutionDto,
    },
    AbilityMonstersProbed {
        ability_id: String,
        resolution: AbilityMonsterProbeResolutionDto,
    },
    AbilityTargetUnavailable {
        ability_id: String,
    },
    AbilityAreaDamage {
        ability_id: String,
        resolution: AbilityAreaDamageResolutionDto,
        trace: ProjectileTrace,
    },
    AbilityVisibleDamage {
        ability_id: String,
        resolution: AbilityVisibleDamageResolutionDto,
    },
    AbilityBeamDamage {
        ability_id: String,
        resolution: AbilityBeamDamageResolutionDto,
        trace: ProjectileTrace,
    },
    AbilityConeDamage {
        ability_id: String,
        resolution: AbilityConeDamageResolutionDto,
        trace: ProjectileTrace,
    },
    AbilityTeleported {
        ability_id: String,
        resolution: AbilityTeleportResolutionDto,
    },
    AbilitySummoned {
        ability_id: String,
        resolution: AbilitySummonResolutionDto,
    },
    AbilityDetected {
        ability_id: String,
        resolution: AbilityDetectResolutionDto,
    },
    AbilityTerrainTransformed {
        ability_id: String,
        resolution: AbilityTerrainTransformResolutionDto,
    },
    AbilityEffectsResolved {
        ability_id: String,
        resolution: AbilityEffectsResolutionDto,
        trace: Option<ProjectileTrace>,
    },
    AbilitySelfKnowledge {
        ability_id: String,
        name_key: String,
        report: SelfKnowledgeReport,
    },
    AbilityProbed {
        ability_id: String,
        report: AbilityProbeTargetDto,
    },
    MonsterAbilityDecision {
        resolution: MonsterAbilityDecisionResolutionDto,
    },
    MonsterAbilityCast {
        resolution: Box<MonsterAbilityCastResolutionDto>,
        trace: Option<ProjectileTrace>,
    },
    SummonExpired {
        entity_id: String,
        target_kind_id: String,
    },
    SummonCommandChanged {
        resolution: SummonCommandResolutionDto,
    },
    PetsDismissed {
        count: u16,
        upkeep_percent: u16,
    },
    PetUpkeepManaLost {
        resource_id: String,
        amount: u32,
        upkeep_percent: u16,
    },
    PetUpkeepDismissalRequired {
        upkeep_percent: u16,
    },
    PetNeglected {
        entity_id: String,
        target_kind_id: String,
        disappeared: bool,
    },
    SummonFollowedFloor {
        entity_id: String,
        target_kind_id: String,
    },
    SummonCouldNotFollow {
        entity_id: String,
        target_kind_id: String,
    },
    AbilityLanded {
        ability_id: String,
        trace: ProjectileTrace,
    },
    GroundItemDestroyedByAbility {
        ability_id: String,
        item_id: String,
        target_kind_id: String,
        quantity: u32,
        position: Position,
    },
    InventoryItemDestroyedByDamage {
        source_kind_id: String,
        target_kind_id: String,
        quantity: u32,
    },
    AbilityHit {
        ability_id: String,
        target_kind_id: String,
        damage: DamageOutcome,
        trace: ProjectileTrace,
    },
    AbilitySlew {
        ability_id: String,
        target_kind_id: String,
        damage: DamageOutcome,
        trace: ProjectileTrace,
    },
    BoltReflected {
        reflector_kind_id: String,
        source_kind_id: String,
        outcome: BoltReflectionOutcome,
        trace: ProjectileTrace,
    },
    AbilityHealed {
        ability_id: String,
        resolution: HealingResolutionDto,
    },
    PlayerVampiricHealed {
        resolution: HealingResolutionDto,
    },
    EquipmentRegenerated {
        resolution: HealingResolutionDto,
    },
    ResourceRecovered {
        resolution: ResourceRecoveryResolutionDto,
    },
    MonsterBlinked {
        source_kind_id: String,
        resolution: MonsterDisplacementResolutionDto,
    },
    MonsterQuantumVanished {
        source_kind_id: String,
    },
    MonsterEarthquakeResolved {
        source_kind_id: String,
        resolution: AbilityEffectsResolutionDto,
    },
    PlayerWeaponEarthquakeResolved {
        source_item_id: String,
        resolution: AbilityEffectsResolutionDto,
    },
    PlayerWeaponEarthquakeHit {
        source_item_id: String,
        damage: DamageOutcome,
    },
    PlayerWeaponEarthquakeSlew {
        source_item_id: String,
        target_kind_id: String,
        damage: DamageOutcome,
    },
    MonsterMeleeAmnesia {
        source_kind_id: String,
        cleared_cells: u32,
    },
    MonsterTimeRavaged {
        source_kind_id: String,
        attribute_count: u8,
    },
    MonsterBlinkedTarget {
        source_kind_id: String,
        target_kind_id: String,
        resolution: MonsterDisplacementResolutionDto,
    },
    MonsterTeleported {
        source_kind_id: String,
        resolution: MonsterDisplacementResolutionDto,
    },
    EldritchHorror {
        source_entity_id: String,
        source_kind_id: String,
        power: u16,
        outcome: &'static str,
    },
    MonsterDraggedTarget {
        source_kind_id: String,
        target_kind_id: String,
        resolution: MonsterDisplacementResolutionDto,
    },
    MonsterBanishedTarget {
        source_kind_id: String,
        target_kind_id: String,
        resolution: MonsterDisplacementResolutionDto,
    },
    RestCompleted {
        resolution: RestResolutionDto,
    },
    RestInterrupted {
        resolution: RestResolutionDto,
    },
    ItemAppraised {
        target_kind_id: String,
        quality: ItemQualityDto,
    },
    ItemAppraiseUnavailable,
    ItemDestroyed {
        target_kind_id: String,
        quantity: u32,
        rule_line: Option<u32>,
    },
    ItemDestroyUnavailable {
        item_id: String,
        reason: String,
        rule_line: Option<u32>,
    },
    ItemInscribed {
        target_kind_id: String,
        inscription: Option<String>,
        rule_line: Option<u32>,
    },
    ItemInscribeUnavailable {
        item_id: String,
        reason: String,
    },
    ItemsDropped {
        stacks: usize,
        quantity: u64,
    },
    NoItemsDropped,
    CaptureBallCaptured {
        target_kind_id: String,
    },
    CaptureBallCaptureFailed {
        target_kind_id: String,
        reason: String,
    },
    CaptureBallReleased {
        target_kind_id: String,
        hostile: bool,
    },
    CaptureBallReleaseFailed {
        target_kind_id: String,
    },
    ItemEquipped {
        target_kind_id: String,
        slot_id: String,
        replaced_kind_id: Option<String>,
    },
    ItemEquipUnavailable,
    ItemPropertyDiscovered {
        target_kind_id: String,
        property_name_key: String,
    },
    LootDropped {
        source_kind_id: String,
        target_kind_id: String,
        quantity: u32,
    },
    GoldDropped {
        source_kind_id: String,
        amount: u32,
    },
    ExperienceGained {
        amount: u64,
        total: u64,
    },
    ExperienceDrained {
        source_kind_id: String,
        amount: u64,
        total: u64,
    },
    MonsterUnlifeDrained {
        source_kind_id: String,
        amount: u16,
        life_force_before: u16,
        life_force_after: u16,
        power_before: u16,
        power_after: u16,
    },
    MonsterUnlifeWeakened {
        source_kind_id: String,
        target_kind_id: String,
        amount: u16,
        power_before: u16,
        power_after: u16,
    },
    PlayerLevelGained {
        level: u16,
        max_hp: i32,
        pending_attribute_increases: u16,
        reached_new_maximum: bool,
    },
    PlayerLevelLost {
        level: u16,
        max_hp: i32,
    },
    PlayerLevelCapUnlocked {
        level_cap: u16,
        attribute_index_cap: u8,
    },
    PlayerAttributeIncreased {
        attribute: crate::stats::AttributeKind,
        natural: u16,
        effective: u16,
        index: u8,
        pending_attribute_increases: u16,
    },
    PlayerAttributeIncreaseUnavailable {
        attribute: crate::stats::AttributeKind,
    },
    FloorTransitioned {
        from_floor_id: String,
        to_floor_id: String,
    },
    FloorTransitionUnavailable,
    DungeonExpeditionEnded,
    DungeonGuardianDefeated {
        dungeon_id: String,
        floor_id: String,
        target_kind_id: String,
    },
    DungeonEntranceGuardianDefeated {
        dungeon_id: String,
        target_kind_id: String,
    },
    CampaignVictorious {
        score: u64,
    },
    CampaignRetired {
        score: u64,
    },
    CampaignRetireUnavailable,
    OneShotFloorClosed {
        floor_id: String,
    },
    TaskCompleted {
        floor_id: String,
    },
    TaskRewardAvailable {
        floor_id: String,
    },
    TaskExitRevealed {
        floor_id: String,
        position: Position,
    },
    TaskAccepted {
        task_id: String,
    },
    TaskAcceptUnavailable {
        facility_id: String,
        task_id: String,
        reason: String,
    },
    TaskFailed {
        floor_id: String,
    },
    TaskAbandoned {
        floor_id: String,
    },
    TaskAbandonUnavailable,
    TaskPaused {
        floor_id: String,
    },
    TaskResumed {
        floor_id: String,
    },
    TaskRewarded {
        item_kind_id: String,
        quantity: u32,
    },
    TaskRewardClaimUnavailable {
        facility_id: String,
        task_id: String,
        reason: String,
    },
    DoorOpened {
        position: Position,
    },
    DoorUnlocked {
        position: Position,
    },
    DoorUnlockFailed {
        position: Position,
    },
    DoorOpenUnavailable,
    DoorBashedOpen {
        position: Position,
    },
    DoorBashFailed {
        position: Position,
    },
    DoorBashUnavailable,
    SecretTerrainDiscovered {
        position: Position,
    },
    SearchFoundNothing,
    TrapTriggered {
        position: Position,
        damage: DamageOutcome,
    },
    ActorTrapTriggered {
        position: Position,
        target_kind_id: String,
        damage: DamageOutcome,
    },
    TrapDisarmed {
        position: Position,
    },
    TrapDisarmFailed {
        position: Position,
    },
    TrapDisarmUnavailable,
    DeviceSkillChecked {
        source_kind_id: String,
        succeeded: bool,
        resolution: CheckResolutionDto,
    },
    DeviceEnergyRecovered {
        target_item_id: String,
        target_kind_id: String,
        amount: u32,
        current: u32,
        maximum: u32,
    },
    DeviceRechargeResolved {
        target_item_id: String,
        target_kind_id: String,
        source_id: String,
        source_is_item: bool,
        attempted: u32,
        target_before: u32,
        target_after: u32,
        succeeded: bool,
        failure_one_in: u32,
        failure_roll: Option<u32>,
        source_destroyed: bool,
    },
    LightRefuelUnavailable {
        target_item_id: String,
        source_item_id: String,
        reason: String,
    },
    LightRefueled {
        target_item_id: String,
        target_kind_id: String,
        source_kind_id: String,
        amount: u16,
        current: u16,
        maximum: u16,
    },
    LightExtinguished {
        target_item_id: String,
        target_kind_id: String,
    },
    SavingThrowChecked {
        source_kind_id: String,
        position: Position,
        succeeded: bool,
        resolution: CheckResolutionDto,
    },
    StealthChecked {
        source_kind_id: String,
        succeeded: bool,
        resolution: CheckResolutionDto,
    },
    PerceptionChecked {
        position: Position,
        succeeded: bool,
        resolution: CheckResolutionDto,
    },
    ItemWarnedOfTrap {
        position: Position,
    },
    TerrainDug {
        position: Position,
    },
    TerrainDigFailed {
        position: Position,
        retryable: bool,
    },
    TerrainDigUnavailable,
    DoorClosed {
        position: Position,
    },
    DoorCloseUnavailable,
    Waited,
    ItemPickedUp {
        target_kind_id: String,
        quantity: u32,
    },
    GoldPickedUp {
        amount: u32,
        balance: u32,
    },
    ShopPurchaseCompleted {
        outcome: ShopTransactionOutcome,
    },
    ShopSaleCompleted {
        outcome: ShopTransactionOutcome,
    },
    ShopTransactionUnavailable {
        shop_id: String,
        item_id: String,
        reason: String,
    },
    FacilityIdentifyUnavailable {
        facility_id: String,
        item_id: String,
        reason: String,
    },
    FacilityItemIdentified {
        outcome: FacilityIdentifyOutcome,
    },
    FacilityRenameUnavailable {
        facility_id: String,
        reason: String,
    },
    FacilityPlayerRenamed {
        outcome: FacilityRenameOutcome,
    },
    InnStayUnavailable {
        facility_id: String,
        reason: String,
    },
    InnStayCompleted {
        outcome: InnStayOutcome,
    },
    InnTravelUnavailable {
        facility_id: String,
        destination_town_id: String,
        reason: String,
    },
    InnTravelCompleted {
        outcome: InnTravelOutcome,
    },
    HomeItemDeposited {
        outcome: crate::game::town::HomeTransferOutcome,
    },
    HomeItemWithdrawn {
        outcome: crate::game::town::HomeTransferOutcome,
    },
    HomeTransferUnavailable {
        facility_id: String,
        item_id: String,
        reason: String,
    },
    ItemPickupInventoryFull {
        target_kind_id: String,
        quantity: u32,
        used_slots: u16,
        required_slots: u16,
        capacity: u16,
    },
    NothingToPickUp,
    ItemUnequipped {
        target_kind_id: String,
        slot_id: String,
    },
    ItemUnequipUnavailable {
        slot_id: String,
    },
    ItemUnequipCursed {
        target_kind_id: String,
        slot_id: String,
        severity: ItemCurseSeverityDto,
    },
    MoveBlocked,
    WildernessAmbushed,
    WildernessInterestingDiscovery,
    WildernessTerrainDamaged {
        terrain_id: String,
        damage: DamageOutcome,
    },
    RidingMounted {
        target_kind_id: String,
    },
    RidingDismounted {
        target_kind_id: String,
    },
    RidingFailed {
        target_kind_id: String,
    },
    RidingNotPet {
        target_kind_id: String,
    },
    RidingFell {
        target_kind_id: String,
        damage: DamageOutcome,
    },
    RidingCollided {
        target_kind_id: String,
        damage: DamageOutcome,
    },
    RodeoAlreadyRiding,
    RodeoUntameable {
        target_kind_id: String,
    },
    RodeoTooWeak {
        target_kind_id: String,
    },
    RodeoTamed {
        target_kind_id: String,
    },
    RodeoThrownOff {
        target_kind_id: String,
    },
    RidingBondMaxed {
        target_kind_id: String,
    },
    PetEvolved {
        previous_kind_id: String,
        target_kind_id: String,
    },
    MountPotionUsed {
        item_kind_id: String,
        target_kind_id: String,
    },
    RidingUnavailable,
    SheepRidingRefused {
        response: u8,
    },
    ProjectileUnavailable,
    ProjectileAmmoUnavailable {
        ammo_kind_id: String,
    },
    ProjectileTargetUnavailable,
    ProjectileLanded {
        trace: ProjectileTrace,
    },
    ProjectileMissed {
        target_kind_id: String,
        trace: ProjectileTrace,
    },
    ProjectileHit {
        target_kind_id: String,
        damage: DamageOutcome,
        trace: ProjectileTrace,
    },
    ProjectileSlew {
        target_kind_id: String,
        damage: DamageOutcome,
        trace: ProjectileTrace,
    },
    ProjectileAmmoRecovered {
        ammo_kind_id: String,
    },
    ProjectileAmmoBroken {
        ammo_kind_id: String,
    },
    ItemThrown {
        target_kind_id: String,
        trace: ProjectileTrace,
    },
    ItemThrowMissed {
        source_kind_id: String,
        target_kind_id: String,
        trace: ProjectileTrace,
    },
    ItemThrowHit {
        source_kind_id: String,
        target_kind_id: String,
        damage: DamageOutcome,
        trace: ProjectileTrace,
    },
    ItemThrowSlew {
        source_kind_id: String,
        target_kind_id: String,
        damage: DamageOutcome,
        trace: ProjectileTrace,
    },
    ItemThrowUnavailable,
    DeviceAbsorbed {
        item_id: String,
        item_kind_id: String,
        charges_before: u32,
        charges_after: u32,
        drained: u32,
        nutrition_before: u16,
        nutrition_after: u16,
    },
    DeviceAbsorptionUnavailable {
        item_id: String,
    },
    ItemUsed {
        source_kind_id: String,
        display_name_key: String,
        requested: i32,
        applied: i32,
    },
    ItemNutritionIncreased {
        source_kind_id: String,
        display_name_key: String,
        amount: u16,
        nutrition: u16,
    },
    ItemNutritionSatisfied {
        source_kind_id: String,
        display_name_key: String,
        nutrition: u16,
        noticed: bool,
    },
    ItemExperienceLost {
        source_kind_id: String,
        display_name_key: String,
        amount: u64,
        remaining: u64,
    },
    ItemStatusResolved {
        source_kind_id: String,
        display_name_key: String,
        status_kind_id: String,
        duration: Option<u32>,
        noticed: bool,
    },
    ItemStatusRemoved {
        source_kind_id: String,
        display_name_key: String,
        status_kind_id: String,
        removed: bool,
    },
    ItemStatusReduced {
        source_kind_id: String,
        display_name_key: String,
        status_kind_id: String,
        before: u32,
        after: u32,
    },
    ItemBlessed {
        source_kind_id: String,
        display_name_key: String,
        duration: u32,
        resolution: AbilityEffectsResolutionDto,
    },
    ItemSlownessResolved {
        source_kind_id: String,
        display_name_key: String,
        duration: u32,
        noticed: bool,
    },
    ItemSpeedResolved {
        source_kind_id: String,
        display_name_key: String,
        duration: u32,
    },
    ItemHeroismResolved {
        source_kind_id: String,
        display_name_key: String,
        duration: u32,
        noticed: bool,
    },
    ItemBerserkStrengthResolved {
        source_kind_id: String,
        display_name_key: String,
        duration: u32,
        noticed: bool,
    },
    ItemPoeticInspirationResolved {
        source_kind_id: String,
        display_name_key: String,
        duration: u32,
        noticed: bool,
    },
    ItemStoneSkinResolved {
        source_kind_id: String,
        display_name_key: String,
        duration: u32,
        noticed: bool,
    },
    ItemRestoreLifeLevelsResolved {
        source_kind_id: String,
        display_name_key: String,
        noticed: bool,
    },
    ItemRestorationResolved {
        source_kind_id: String,
        display_name_key: String,
        noticed: bool,
    },
    ItemAttributeChanged {
        source_kind_id: String,
        display_name_key: String,
        attribute: crate::stats::AttributeKind,
        change: ItemAttributeChange,
        before: u16,
        after: u16,
        maximum: u16,
        noticed: bool,
    },
    ItemThermalResistanceResolved {
        source_kind_id: String,
        display_name_key: String,
        duration: u32,
        noticed: bool,
    },
    ItemBasicResistanceApplied {
        source_kind_id: String,
        display_name_key: String,
        duration: u32,
    },
    ItemPoisonResolved {
        source_kind_id: String,
        display_name_key: String,
        duration: Option<u32>,
    },
    ItemBlindnessResolved {
        source_kind_id: String,
        display_name_key: String,
        duration: Option<u32>,
        noticed: bool,
    },
    ItemResourceDrained {
        source_kind_id: String,
        display_name_key: String,
        resource_id: String,
        drained: u32,
    },
    ItemDetonation {
        source_kind_id: String,
        display_name_key: String,
        damage: DamageOutcome,
        fatal: bool,
    },
    ItemLifeLost {
        source_kind_id: String,
        display_name_key: String,
        amount: i32,
        fatal: bool,
    },
    ItemVengeanceActivated {
        source_kind_id: String,
        display_name_key: String,
        duration: u32,
        resolution: AbilityEffectsResolutionDto,
    },
    ItemProtectionFromEvil {
        source_kind_id: String,
        display_name_key: String,
        duration: u32,
        resolution: AbilityEffectsResolutionDto,
    },
    ItemConfusingStrikePrepared {
        source_kind_id: String,
        display_name_key: String,
    },
    ItemSpellLearningCapacityChanged {
        source_kind_id: String,
        display_name_key: String,
        before: u16,
        after: u16,
    },
    ItemElementalBlast {
        source_kind_id: String,
        display_name_key: String,
        target_count: usize,
    },
    ItemElementalBlastHit {
        source_kind_id: String,
        target_kind_id: String,
        damage: DamageOutcome,
    },
    ItemElementalBlastSlew {
        source_kind_id: String,
        target_kind_id: String,
        damage: DamageOutcome,
    },
    ItemElementalBlastBacklash {
        source_kind_id: String,
        damage: DamageOutcome,
        fatal: bool,
    },
    ItemAggravated {
        source_kind_id: String,
        display_name_key: String,
    },
    ItemMassGenocide {
        source_kind_id: String,
        display_name_key: String,
        removed_count: usize,
        resisted_count: usize,
        fatigue_damage: i32,
    },
    ItemGenocide {
        source_kind_id: String,
        display_name_key: String,
        glyph: String,
        removed_count: usize,
        resisted_count: usize,
        fatigue_damage: i32,
    },
    ItemCreatedAdjacentTerrain {
        source_kind_id: String,
        display_name_key: String,
        affected_positions: Vec<Position>,
    },
    ItemCreatedCurrentTerrain {
        source_kind_id: String,
        display_name_key: String,
        affected_position: Option<Position>,
    },
    ItemFloorGlowChanged {
        source_kind_id: String,
        display_name_key: String,
        glow: bool,
        affected_positions: Vec<Position>,
    },
    ItemAreaDestruction {
        source_kind_id: String,
        display_name_key: String,
        protected_floor: bool,
        affected_positions: Vec<Position>,
        removed_entities: usize,
        removed_items: usize,
        removed_gold_piles: usize,
    },
    ItemDestroyedAdjacentTrapsAndDoors {
        source_kind_id: String,
        display_name_key: String,
        affected_positions: Vec<Position>,
    },
    ItemResourceRestored {
        source_kind_id: String,
        display_name_key: String,
        resolution: ResourceRecoveryResolutionDto,
    },
    ItemIdentified {
        source_kind_id: String,
        display_name_key: String,
        resolution: ItemIdentifyResolutionDto,
    },
    ItemInventoryIdentified {
        source_kind_id: String,
        display_name_key: String,
        count: usize,
    },
    ItemAutoIdentified {
        count: usize,
    },
    ItemSelfKnowledge {
        source_kind_id: String,
        display_name_key: String,
        report: SelfKnowledgeReport,
    },
    ItemAcquirement {
        source_kind_id: String,
        display_name_key: String,
        generated_item_ids: Vec<String>,
        generated_kind_ids: Vec<String>,
        position: Position,
    },
    ItemMundanified {
        source_kind_id: String,
        display_name_key: String,
        target_item_id: String,
        target_kind_id: String,
        split: bool,
    },
    ItemCrafted {
        source_kind_id: String,
        display_name_key: String,
        target_item_id: String,
        target_kind_id: String,
        affix_id: String,
        split: bool,
    },
    ItemRumour {
        source_kind_id: String,
        display_name_key: String,
        message_key: String,
    },
    ItemEnchanted {
        source_kind_id: String,
        resolution: ItemEnchantmentResolutionDto,
    },
    ItemCursed {
        source_kind_id: String,
        resolution: ItemCurseResolutionDto,
    },
    ItemCursesRemoved {
        source_kind_id: String,
        resolution: ItemCurseRemovalResolutionDto,
    },
    ItemActivationLanded {
        source_kind_id: String,
        profile_id: String,
        trace: ProjectileTrace,
    },
    ItemActivationHit {
        source_kind_id: String,
        profile_id: String,
        target_kind_id: String,
        damage: DamageOutcome,
        trace: ProjectileTrace,
    },
    ItemActivationSlew {
        source_kind_id: String,
        profile_id: String,
        target_kind_id: String,
        damage: DamageOutcome,
        trace: ProjectileTrace,
    },
    ItemDispelHit {
        source_kind_id: String,
        target_kind_id: String,
        damage: DamageOutcome,
    },
    ItemDispelSlew {
        source_kind_id: String,
        target_kind_id: String,
        damage: DamageOutcome,
    },
    ItemDispelNoEffect {
        source_kind_id: String,
    },
    ItemBanishedActor {
        source_kind_id: String,
        target_kind_id: String,
        resolution: MonsterDisplacementResolutionDto,
    },
    ItemBanishmentResisted {
        source_kind_id: String,
        target_kind_id: String,
    },
    ItemBanishmentNoSpace {
        source_kind_id: String,
        target_kind_id: String,
    },
    ItemBanishmentNoEffect {
        source_kind_id: String,
    },
    ItemActivationDetected {
        source_kind_id: String,
        profile_id: String,
        resolution: AbilityDetectResolutionDto,
    },
    ItemDetected {
        source_kind_id: String,
        resolution: AbilityDetectResolutionDto,
    },
    ItemSummoned {
        source_kind_id: String,
        profile_id: Option<String>,
        resolution: AbilitySummonResolutionDto,
    },
    ItemTeleported {
        source_kind_id: String,
        profile_id: Option<String>,
        resolution: AbilityTeleportResolutionDto,
    },
    ItemTeleportedLevel {
        source_kind_id: String,
        from_floor_id: String,
        to_floor_id: String,
    },
    ItemRecallStarted {
        source_kind_id: String,
        dungeon_id: String,
        floor_id: String,
        turns: u16,
    },
    ItemRecallCancelled {
        source_kind_id: String,
    },
    ItemRecallReset {
        source_kind_id: String,
        dungeon_id: String,
        floor_id: String,
    },
    RecallTriggered {
        from_floor_id: String,
        to_floor_id: String,
    },
    ItemUseUnavailable,
    WeaponProficiencyImproved {
        item_kind_id: String,
    },
    RidingProficiencyImproved {
        current: u16,
    },
    MiningProficiencyImproved,
    TerrainFoundSomething,
    PlayerMeleeMissed {
        target_kind_id: String,
    },
    MutationMeleeMissed {
        mutation_id: String,
        attack_name: String,
        target_kind_id: String,
    },
    ConfusingStrikeImmune {
        target_kind_id: String,
    },
    ConfusingStrikeResisted {
        target_kind_id: String,
    },
    ConfusingStrikeApplied {
        target_kind_id: String,
        duration: u32,
    },
    PlayerFearBlocked {
        status_kind_id: String,
    },
    PlayerConfusedMove {
        intended: Direction,
        actual: Direction,
    },
    PlayerParalyzed {
        status_kind_id: String,
    },
    MonsterSlept {
        target_kind_id: String,
    },
    EntityAwakened {
        target_kind_id: String,
    },
    PlayerMeleeHit {
        target_kind_id: String,
        damage: DamageOutcome,
    },
    MutationMeleeHit {
        mutation_id: String,
        attack_name: String,
        target_kind_id: String,
        damage: DamageOutcome,
    },
    MonsterContactAuraApplied {
        source_kind_id: String,
        status_kind_id: String,
        duration: u32,
    },
    MonsterFearAuraApplied {
        source_kind_id: String,
        trigger: &'static str,
        duration: u32,
    },
    PlayerSlew {
        target_kind_id: String,
        damage: DamageOutcome,
    },
    MutationMeleeSlew {
        mutation_id: String,
        attack_name: String,
        target_kind_id: String,
        damage: DamageOutcome,
    },
    SummonMeleeMissed {
        source_kind_id: String,
        target_kind_id: String,
        method_id: Option<String>,
    },
    SummonMeleeHit {
        source_kind_id: String,
        target_kind_id: String,
        method_id: Option<String>,
        damage: DamageOutcome,
    },
    SummonSlew {
        source_kind_id: String,
        target_kind_id: String,
        method_id: Option<String>,
        damage: DamageOutcome,
    },
    MonsterMeleeMissed {
        source_kind_id: String,
        method_id: Option<String>,
    },
    MonsterMeleeRepelled {
        source_kind_id: String,
        method_id: Option<String>,
    },
    MonsterMeleeHit {
        source_kind_id: String,
        method_id: Option<String>,
        damage: DamageOutcome,
    },
    MonsterBegged {
        source_kind_id: String,
    },
    MonsterSelfDestructed {
        source_kind_id: String,
    },
    MonsterDeathExplosionHit {
        source_kind_id: String,
        target_kind_id: String,
        damage: DamageOutcome,
    },
    MonsterDeathExplosionSlew {
        source_kind_id: String,
        target_kind_id: String,
        damage: DamageOutcome,
    },
    MonsterTerrainDestroyed {
        source_kind_id: String,
        terrain_kind_id: String,
        replacement_terrain_kind_id: String,
        position: Position,
    },
    WardingGlyphHeld {
        source_kind_id: String,
    },
    WardingGlyphBroken {
        source_kind_id: String,
        position: Position,
    },
    MonsterItemDestroyed {
        source_kind_id: String,
        target_kind_id: String,
        quantity: u32,
        position: Position,
    },
    MonsterItemPickedUp {
        source_kind_id: String,
        target_kind_id: String,
        quantity: u32,
        position: Position,
    },
    MonsterGoldTheftPrevented {
        source_kind_id: String,
    },
    MonsterItemTheftPrevented {
        source_kind_id: String,
    },
    MonsterGoldStolen {
        source_kind_id: String,
        amount: u32,
    },
    MonsterItemStolen {
        source_kind_id: String,
        target_kind_id: String,
        item_id: String,
    },
    MonsterFoodEaten {
        source_kind_id: String,
        target_kind_id: String,
    },
    MonsterLightEaten {
        source_kind_id: String,
        target_kind_id: String,
        amount: u16,
    },
    MonsterChargesDrained {
        source_kind_id: String,
        target_kind_id: String,
        amount: u32,
    },
    MonsterNutritionDrained {
        source_kind_id: String,
        amount: u16,
    },
    MutationAuraHit {
        target_kind_id: String,
        damage: DamageOutcome,
    },
    MutationAuraSlew {
        target_kind_id: String,
        damage: DamageOutcome,
    },
    VengeanceHit {
        target_kind_id: String,
        damage: DamageOutcome,
    },
    VengeanceSlew {
        target_kind_id: String,
        damage: DamageOutcome,
    },
    MonsterMeleeEntityMissed {
        source_kind_id: String,
        target_kind_id: String,
        method_id: Option<String>,
    },
    MonsterMeleeEntityHit {
        source_kind_id: String,
        target_kind_id: String,
        method_id: Option<String>,
        damage: DamageOutcome,
    },
    MonsterMeleeEntitySlew {
        source_kind_id: String,
        target_kind_id: String,
        method_id: Option<String>,
        damage: DamageOutcome,
    },
    MonsterBeggedEntity {
        source_kind_id: String,
        target_kind_id: String,
    },
    MonsterFled {
        source_kind_id: String,
        target_kind_id: String,
    },
    MonsterKeptDistance {
        source_kind_id: String,
        target_kind_id: String,
    },
    PlayerDied {
        source_kind_id: String,
        method_id: Option<String>,
        damage: DamageOutcome,
    },
    NutritionStateChanged {
        from: rfb_protocol::NutritionStateDto,
        to: rfb_protocol::NutritionStateDto,
        nutrition: u16,
    },
    PlayerFaintedFromHunger {
        duration: u32,
    },
    PlayerDamagedByStarvation {
        damage: DamageOutcome,
    },
    PlayerDiedFromStarvation {
        damage: DamageOutcome,
    },
    PlayerStatusDamaged {
        status_kind_id: String,
        damage: DamageOutcome,
    },
    EntityStatusDamaged {
        target_kind_id: String,
        status_kind_id: String,
        damage: DamageOutcome,
    },
    PlayerStatusExpired {
        status_kind_id: String,
    },
    EntityStatusExpired {
        target_kind_id: String,
        status_kind_id: String,
    },
    PlayerDiedFromStatus {
        status_kind_id: String,
        damage: DamageOutcome,
    },
    EntityDiedFromStatus {
        target_kind_id: String,
        status_kind_id: String,
        damage: DamageOutcome,
    },
}

impl DomainEvent {
    pub(crate) fn into_dto(self) -> GameEventDto {
        match self {
            Self::MutationGained { mutation_id, name } => dto(
                "mutation.gained",
                "mutation-gained",
                [("target", mutation_id), ("name", name)],
            ),
            Self::MutationLost { mutation_id, name } => dto(
                "mutation.lost",
                "mutation-lost",
                [("target", mutation_id), ("name", name)],
            ),
            Self::MutationAllCured => dto_without_args("mutation.all-cured", "mutation-all-cured"),
            Self::MutationPeriodicTriggered { mutation_id, name } => dto(
                "mutation.periodic-triggered",
                "mutation-periodic-triggered",
                [("target", mutation_id), ("name", name)],
            ),
            Self::MutationWarning { danger_amount } => {
                let (kind, message_key) = if danger_amount > 100 {
                    ("mutation.warning.extreme", "mutation-warning-extreme")
                } else if danger_amount > 50 {
                    ("mutation.warning.afraid", "mutation-warning-afraid")
                } else if danger_amount > 20 {
                    ("mutation.warning.worried", "mutation-warning-worried")
                } else if danger_amount > 10 {
                    ("mutation.warning.paranoid", "mutation-warning-paranoid")
                } else if danger_amount > 5 {
                    ("mutation.warning.safe", "mutation-warning-safe")
                } else {
                    ("mutation.warning.lonely", "mutation-warning-lonely")
                };
                dto(kind, message_key, [("danger", danger_amount.to_string())])
            }
            Self::ChaosPatronReward {
                patron_id,
                patron_name,
            } => dto(
                "mutation.chaos-patron-reward",
                "mutation-chaos-patron-reward",
                [("target", patron_id), ("patron", patron_name)],
            ),
            Self::MutationFumbled {
                damage,
                dropped_item_kind_id: Some(target_kind_id),
            } => dto_with_outcome(
                "mutation.fumbled-drop",
                "mutation-fumbled-drop",
                [
                    ("target", target_kind_id),
                    ("damage", damage.applied.to_string()),
                ],
                GameEventOutcomeDto::Damage {
                    resolution: damage.into(),
                },
            ),
            Self::MutationFumbled {
                damage,
                dropped_item_kind_id: None,
            } => dto_with_outcome(
                "mutation.fumbled",
                "mutation-fumbled",
                [("damage", damage.applied.to_string())],
                GameEventOutcomeDto::Damage {
                    resolution: damage.into(),
                },
            ),
            Self::RealityChangeResolved { regenerated: true } => {
                dto_without_args("mutation.reality-changed", "mutation-reality-changed")
            }
            Self::RealityChangeResolved { regenerated: false } => {
                dto_without_args("mutation.reality-unchanged", "mutation-reality-unchanged")
            }
            Self::AbilityStudied { ability_id } => dto(
                "ability.studied",
                "ability-studied",
                [("target", ability_id)],
            ),
            Self::AbilityForgotten { ability_id } => dto(
                "ability.forgotten",
                "ability-forgotten",
                [("target", ability_id)],
            ),
            Self::AbilityForgetUnavailable { ability_id, reason } => dto(
                "ability.forget-unavailable",
                "ability-forget-unavailable",
                [("target", ability_id), ("reason", reason)],
            ),
            Self::AbilityStudyUnavailable { target_id, reason } => dto(
                "ability.study-unavailable",
                "ability-study-unavailable",
                [("target", target_id), ("reason", reason)],
            ),
            Self::AbilityCastUnavailable { ability_id, reason } => dto(
                "ability.cast-unavailable",
                "ability-cast-unavailable",
                [("target", ability_id), ("reason", reason)],
            ),
            Self::AbilityCastFailed { resolution } => dto_with_outcome(
                "ability.cast-failure",
                "ability-cast-failure",
                [("target", resolution.ability_id.clone())],
                GameEventOutcomeDto::AbilityCast { resolution },
            ),
            Self::AbilityCastSucceeded { resolution } => dto_with_outcome(
                "ability.cast-success",
                "ability-cast-success",
                [("target", resolution.ability_id.clone())],
                GameEventOutcomeDto::AbilityCast { resolution },
            ),
            Self::AbilityMonstersProbed {
                ability_id,
                resolution,
            } => dto_with_outcome(
                "ability.monsters-probed",
                "ability-monsters-probed",
                [
                    ("target", ability_id),
                    ("count", resolution.monsters.len().to_string()),
                ],
                GameEventOutcomeDto::AbilityMonsterProbe { resolution },
            ),
            Self::AbilityTargetUnavailable { ability_id } => dto(
                "ability.target-unavailable",
                "ability-target-unavailable",
                [("target", ability_id)],
            ),
            Self::AbilityAreaDamage {
                ability_id,
                resolution,
                trace,
            } => with_trace(
                dto_with_outcome(
                    "ability.area-damage",
                    "ability-area-damage",
                    [
                        ("target", ability_id),
                        ("radius", resolution.radius.to_string()),
                        ("targets", resolution.target_count.to_string()),
                    ],
                    GameEventOutcomeDto::AbilityAreaDamage { resolution },
                ),
                trace,
            ),
            Self::AbilityVisibleDamage {
                ability_id,
                resolution,
            } => dto_with_outcome(
                "ability.visible-damage",
                "ability-visible-damage",
                [
                    ("target", ability_id),
                    ("targets", resolution.target_count.to_string()),
                ],
                GameEventOutcomeDto::AbilityVisibleDamage { resolution },
            ),
            Self::AbilityBeamDamage {
                ability_id,
                resolution,
                trace,
            } => with_trace(
                dto_with_outcome(
                    "ability.beam-damage",
                    "ability-beam-damage",
                    [
                        ("target", ability_id),
                        ("targets", resolution.target_count.to_string()),
                    ],
                    GameEventOutcomeDto::AbilityBeamDamage { resolution },
                ),
                trace,
            ),
            Self::AbilityConeDamage {
                ability_id,
                resolution,
                trace,
            } => with_trace(
                dto_with_outcome(
                    "ability.cone-damage",
                    "ability-cone-damage",
                    [
                        ("target", ability_id),
                        ("radius", resolution.radius.to_string()),
                        ("targets", resolution.target_count.to_string()),
                    ],
                    GameEventOutcomeDto::AbilityConeDamage { resolution },
                ),
                trace,
            ),
            Self::AbilityTeleported {
                ability_id,
                resolution,
            } => dto_with_outcome(
                "ability.teleport",
                "ability-teleport",
                [
                    ("target", ability_id),
                    ("fromX", resolution.from.x.to_string()),
                    ("fromY", resolution.from.y.to_string()),
                    ("toX", resolution.to.x.to_string()),
                    ("toY", resolution.to.y.to_string()),
                ],
                GameEventOutcomeDto::AbilityTeleport { resolution },
            ),
            Self::AbilitySummoned {
                ability_id,
                resolution,
            } => dto_with_outcome(
                "ability.summon",
                "ability-summon",
                [
                    ("target", ability_id),
                    ("actor", resolution.actor_kind_id.clone()),
                    ("count", resolution.entity_ids.len().to_string()),
                ],
                GameEventOutcomeDto::AbilitySummon { resolution },
            ),
            Self::AbilityDetected {
                ability_id,
                resolution,
            } => dto_with_outcome(
                "ability.detect",
                "ability-detect",
                [
                    ("target", ability_id),
                    ("category", resolution.category.clone()),
                    ("count", resolution.detected_positions.len().to_string()),
                ],
                GameEventOutcomeDto::AbilityDetect { resolution },
            ),
            Self::AbilityTerrainTransformed {
                ability_id,
                resolution,
            } => dto_with_outcome(
                "ability.terrain-transform",
                "ability-terrain-transform",
                [
                    ("target", ability_id),
                    ("terrain", resolution.target_terrain_id.clone()),
                    ("count", resolution.transformed_positions.len().to_string()),
                ],
                GameEventOutcomeDto::AbilityTerrainTransform { resolution },
            ),
            Self::AbilityEffectsResolved {
                ability_id,
                resolution,
                trace,
            } => {
                let event = dto_with_outcome(
                    "ability.effects",
                    "ability-effects",
                    [
                        ("target", ability_id),
                        ("count", resolution.effects.len().to_string()),
                    ],
                    GameEventOutcomeDto::AbilityEffects { resolution },
                );
                match trace {
                    Some(trace) => with_trace(event, trace),
                    None => event,
                }
            }
            Self::AbilitySelfKnowledge {
                ability_id,
                name_key,
                report,
            } => dto(
                "ability.self-knowledge",
                "item-use-self-knowledge",
                [
                    ("source", ability_id),
                    ("nameKey", name_key),
                    ("level", report.level.to_string()),
                    ("hp", report.hp.to_string()),
                    ("maxHp", report.max_hp.to_string()),
                    ("gold", report.gold.to_string()),
                    ("nutrition", report.nutrition.to_string()),
                    ("attack", report.attack.to_string()),
                    ("defense", report.defense.to_string()),
                    ("meleeSkill", report.melee_skill.to_string()),
                    ("armorClass", report.armor_class.to_string()),
                    ("speed", report.speed.to_string()),
                    ("strength", report.attributes[0].clone()),
                    ("intelligence", report.attributes[1].clone()),
                    ("wisdom", report.attributes[2].clone()),
                    ("dexterity", report.attributes[3].clone()),
                    ("constitution", report.attributes[4].clone()),
                    ("charisma", report.attributes[5].clone()),
                    ("statuses", report.statuses),
                    ("resistances", report.resistances),
                    ("resources", report.resources),
                ],
            ),
            Self::AbilityProbed { ability_id, report } => {
                let alignment = match report.alignment {
                    AbilityProbeAlignmentDto::Neutral => "neutral",
                    AbilityProbeAlignmentDto::Good => "good",
                    AbilityProbeAlignmentDto::Evil => "evil",
                    AbilityProbeAlignmentDto::GoodAndEvil => "good-and-evil",
                };
                dto(
                    "ability.probe",
                    "ability-probe",
                    [
                        ("source", ability_id),
                        ("target", report.target_kind_id),
                        ("hp", report.hp.to_string()),
                        ("maxHp", report.max_hp.to_string()),
                        ("speed", report.speed.to_string()),
                        ("alignment", alignment.to_owned()),
                        ("faction", format!("{:?}", report.faction).to_lowercase()),
                    ],
                )
            }
            Self::MonsterAbilityDecision { resolution } => {
                let target = resolution
                    .selected_ability_id
                    .clone()
                    .unwrap_or_else(|| "none".to_owned());
                dto_with_outcome(
                    "monster.ability-decision",
                    "monster-ability-decision",
                    [
                        ("source", resolution.source_kind_id.clone()),
                        ("target", target),
                        ("roll", resolution.frequency_roll.to_string()),
                        ("frequency", resolution.frequency_percent.to_string()),
                    ],
                    GameEventOutcomeDto::MonsterAbilityDecision { resolution },
                )
            }
            Self::MonsterAbilityCast { resolution, trace } => {
                let event = dto_with_outcome(
                    "monster.ability-cast",
                    "monster-ability-cast",
                    [
                        ("source", resolution.source_kind_id.clone()),
                        ("target", resolution.ability_id.clone()),
                        ("count", resolution.effects.len().to_string()),
                    ],
                    GameEventOutcomeDto::MonsterAbilityCast {
                        resolution: *resolution,
                    },
                );
                match trace {
                    Some(trace) => with_trace(event, trace),
                    None => event,
                }
            }
            Self::SummonExpired {
                entity_id,
                target_kind_id,
            } => dto(
                "summon.expired",
                "summon-expired",
                [("target", entity_id), ("actor", target_kind_id)],
            ),
            Self::SummonCommandChanged { resolution } => dto_with_outcome(
                "summon.command-changed",
                "summon-command-changed",
                [
                    (
                        "mode",
                        summon_command_mode_id(resolution.command.mode).to_owned(),
                    ),
                    ("count", resolution.affected_summons.to_string()),
                ],
                GameEventOutcomeDto::SummonCommand { resolution },
            ),
            Self::PetsDismissed {
                count,
                upkeep_percent,
            } => dto(
                "summon.dismissed",
                "pets-dismissed",
                [
                    ("count", count.to_string()),
                    ("upkeep", upkeep_percent.to_string()),
                ],
            ),
            Self::PetUpkeepManaLost {
                resource_id,
                amount,
                upkeep_percent,
            } => dto(
                "summon.upkeep-mana-lost",
                "pet-upkeep-mana-lost",
                [
                    ("resource", resource_id),
                    ("amount", amount.to_string()),
                    ("upkeep", upkeep_percent.to_string()),
                ],
            ),
            Self::PetUpkeepDismissalRequired { upkeep_percent } => dto(
                "summon.upkeep-dismissal-required",
                "pet-upkeep-dismissal-required",
                [("upkeep", upkeep_percent.to_string())],
            ),
            Self::PetNeglected {
                entity_id,
                target_kind_id,
                disappeared,
            } => dto(
                if disappeared {
                    "summon.neglected-disappeared"
                } else {
                    "summon.neglected-hostile"
                },
                if disappeared {
                    "pet-neglected-disappeared"
                } else {
                    "pet-neglected-hostile"
                },
                [("target", entity_id), ("actor", target_kind_id)],
            ),
            Self::SummonFollowedFloor {
                entity_id,
                target_kind_id,
            } => dto(
                "summon.followed-floor",
                "summon-followed-floor",
                [("target", entity_id), ("actor", target_kind_id)],
            ),
            Self::SummonCouldNotFollow {
                entity_id,
                target_kind_id,
            } => dto(
                "summon.could-not-follow",
                "summon-could-not-follow",
                [("target", entity_id), ("actor", target_kind_id)],
            ),
            Self::AbilityLanded { ability_id, trace } => with_trace(
                dto("ability.landed", "ability-landed", [("target", ability_id)]),
                trace,
            ),
            Self::GroundItemDestroyedByAbility {
                ability_id,
                item_id,
                target_kind_id,
                quantity,
                position,
            } => dto(
                "ability.item-destroyed",
                "ability-ground-item-destroyed",
                [
                    ("source", ability_id),
                    ("itemId", item_id),
                    ("target", target_kind_id),
                    ("quantity", quantity.to_string()),
                    ("x", position.x.to_string()),
                    ("y", position.y.to_string()),
                ],
            ),
            Self::InventoryItemDestroyedByDamage {
                source_kind_id,
                target_kind_id,
                quantity,
            } => dto(
                "inventory.item-destroyed",
                "inventory-item-destroyed-by-damage",
                [
                    ("source", source_kind_id),
                    ("target", target_kind_id),
                    ("quantity", quantity.to_string()),
                ],
            ),
            Self::AbilityHit {
                ability_id,
                target_kind_id,
                damage,
                trace,
            } => with_trace(
                dto_with_outcome(
                    "ability.hit",
                    "ability-hit",
                    [
                        ("source", ability_id),
                        ("target", target_kind_id),
                        ("damage", damage.applied.to_string()),
                    ],
                    GameEventOutcomeDto::Damage {
                        resolution: damage.into(),
                    },
                ),
                trace,
            ),
            Self::AbilitySlew {
                ability_id,
                target_kind_id,
                damage,
                trace,
            } => with_trace(
                dto_with_outcome(
                    "ability.slay",
                    "ability-slay",
                    [("source", ability_id), ("target", target_kind_id)],
                    GameEventOutcomeDto::Death {
                        resolution: damage.into(),
                    },
                ),
                trace,
            ),
            Self::BoltReflected {
                reflector_kind_id,
                source_kind_id,
                outcome,
                trace,
            } => match outcome {
                BoltReflectionOutcome::Landed => with_trace(
                    dto(
                        "combat.bolt-reflected",
                        "combat-bolt-reflected",
                        [("reflector", reflector_kind_id), ("source", source_kind_id)],
                    ),
                    trace,
                ),
                BoltReflectionOutcome::Hit {
                    target_kind_id,
                    damage,
                    fatal,
                } => with_trace(
                    dto_with_outcome(
                        if fatal {
                            "combat.bolt-reflected-slay"
                        } else {
                            "combat.bolt-reflected-hit"
                        },
                        if fatal {
                            "combat-bolt-reflected-slay"
                        } else {
                            "combat-bolt-reflected-hit"
                        },
                        [
                            ("reflector", reflector_kind_id),
                            ("source", source_kind_id),
                            ("target", target_kind_id),
                            ("damage", damage.applied.to_string()),
                        ],
                        if fatal {
                            GameEventOutcomeDto::Death {
                                resolution: damage.into(),
                            }
                        } else {
                            GameEventOutcomeDto::Damage {
                                resolution: damage.into(),
                            }
                        },
                    ),
                    trace,
                ),
            },
            Self::AbilityHealed {
                ability_id,
                resolution,
            } => dto_with_outcome(
                "ability.healed",
                "ability-healed",
                [
                    ("source", ability_id),
                    ("amount", resolution.applied.to_string()),
                ],
                GameEventOutcomeDto::Heal { resolution },
            ),
            Self::PlayerVampiricHealed { resolution } => dto_with_outcome(
                "player.vampiric-heal",
                "player-vampiric-heal",
                [
                    ("requested", resolution.requested.to_string()),
                    ("applied", resolution.applied.to_string()),
                ],
                GameEventOutcomeDto::Heal { resolution },
            ),
            Self::EquipmentRegenerated { resolution } => dto_with_outcome(
                "equipment.regenerated",
                "equipment-regenerated",
                [("amount", resolution.applied.to_string())],
                GameEventOutcomeDto::Heal { resolution },
            ),
            Self::ResourceRecovered { resolution } => dto_with_outcome(
                "resource.recovered",
                "resource-recovered",
                [
                    ("target", resolution.resource_id.clone()),
                    ("amount", resolution.recovered.to_string()),
                ],
                GameEventOutcomeDto::ResourceRecovery { resolution },
            ),
            Self::MonsterBlinked {
                source_kind_id,
                resolution,
            } => dto_with_outcome(
                "monster.blinked",
                "monster-blinked",
                [("source", source_kind_id)],
                GameEventOutcomeDto::MonsterDisplacement { resolution },
            ),
            Self::MonsterQuantumVanished { source_kind_id } => dto(
                "monster.quantum-vanished",
                "monster-quantum-vanished",
                [("source", source_kind_id)],
            ),
            Self::MonsterEarthquakeResolved {
                source_kind_id,
                resolution,
            } => dto_with_outcome(
                "monster.earthquake",
                "monster-earthquake",
                [("source", source_kind_id)],
                GameEventOutcomeDto::AbilityEffects { resolution },
            ),
            Self::PlayerWeaponEarthquakeResolved {
                source_item_id,
                resolution,
            } => dto_with_outcome(
                "weapon.impact-earthquake",
                "weapon-impact-earthquake",
                [("source", source_item_id)],
                GameEventOutcomeDto::AbilityEffects { resolution },
            ),
            Self::PlayerWeaponEarthquakeHit {
                source_item_id,
                damage,
            } => dto_with_outcome(
                "weapon.impact-earthquake-hit",
                "weapon-impact-earthquake-hit",
                [("source", source_item_id)],
                GameEventOutcomeDto::Damage {
                    resolution: damage.into(),
                },
            ),
            Self::PlayerWeaponEarthquakeSlew {
                source_item_id,
                target_kind_id,
                damage,
            } => dto_with_outcome(
                "weapon.impact-earthquake-slew",
                "weapon-impact-earthquake-slew",
                [("source", source_item_id), ("target", target_kind_id)],
                GameEventOutcomeDto::Damage {
                    resolution: damage.into(),
                },
            ),
            Self::MonsterMeleeAmnesia {
                source_kind_id,
                cleared_cells,
            } => dto(
                "monster.melee-amnesia",
                "monster-melee-amnesia",
                [
                    ("source", source_kind_id),
                    ("count", cleared_cells.to_string()),
                ],
            ),
            Self::MonsterTimeRavaged {
                source_kind_id,
                attribute_count,
            } => dto(
                "monster.time-ravaged",
                "monster-time-ravaged",
                [
                    ("source", source_kind_id),
                    ("count", attribute_count.to_string()),
                ],
            ),
            Self::MonsterBlinkedTarget {
                source_kind_id,
                target_kind_id,
                resolution,
            } => dto_with_outcome(
                "monster.blinked-target",
                "monster-blinked-target",
                [("source", source_kind_id), ("target", target_kind_id)],
                GameEventOutcomeDto::MonsterDisplacement { resolution },
            ),
            Self::MonsterTeleported {
                source_kind_id,
                resolution,
            } => dto_with_outcome(
                "monster.teleported",
                "monster-teleported",
                [("source", source_kind_id)],
                GameEventOutcomeDto::MonsterDisplacement { resolution },
            ),
            Self::EldritchHorror {
                source_entity_id,
                source_kind_id,
                power,
                outcome,
            } => dto(
                "monster.eldritch-horror",
                "monster-eldritch-horror",
                [
                    ("sourceEntity", source_entity_id),
                    ("source", source_kind_id),
                    ("power", power.to_string()),
                    ("outcome", outcome.to_string()),
                ],
            ),
            Self::MonsterDraggedTarget {
                source_kind_id,
                target_kind_id,
                resolution,
            } => dto_with_outcome(
                "monster.dragged-target",
                "monster-dragged-target",
                [("source", source_kind_id), ("target", target_kind_id)],
                GameEventOutcomeDto::MonsterDisplacement { resolution },
            ),
            Self::MonsterBanishedTarget {
                source_kind_id,
                target_kind_id,
                resolution,
            } => dto_with_outcome(
                "monster.banished-target",
                "monster-banished-target",
                [("source", source_kind_id), ("target", target_kind_id)],
                GameEventOutcomeDto::MonsterDisplacement { resolution },
            ),
            Self::RestCompleted { resolution } => dto_with_outcome(
                "rest.completed",
                "rest-completed",
                [
                    ("turns", resolution.completed_turns.to_string()),
                    ("reason", rest_stop_reason(&resolution.stop_reason)),
                ],
                GameEventOutcomeDto::Rest { resolution },
            ),
            Self::RestInterrupted { resolution } => dto_with_outcome(
                "rest.interrupted",
                "rest-interrupted",
                [
                    ("turns", resolution.completed_turns.to_string()),
                    ("reason", rest_stop_reason(&resolution.stop_reason)),
                ],
                GameEventOutcomeDto::Rest { resolution },
            ),
            Self::ItemAppraised {
                target_kind_id,
                quality,
            } => dto(
                "item.appraise",
                "item-appraise-success",
                [
                    ("target", target_kind_id),
                    ("quality", item_quality_id(quality).to_owned()),
                ],
            ),
            Self::ItemAppraiseUnavailable => {
                dto_without_args("item.appraise.none", "item-appraise-unavailable")
            }
            Self::ItemDestroyed {
                target_kind_id,
                quantity,
                rule_line,
            } => dto(
                "item.destroy",
                "item-destroy-success",
                [
                    ("target", target_kind_id),
                    ("quantity", quantity.to_string()),
                    ("ruleLine", rule_line.unwrap_or(0).to_string()),
                ],
            ),
            Self::ItemDestroyUnavailable {
                item_id,
                reason,
                rule_line,
            } => dto(
                "item.destroy.unavailable",
                "item-destroy-unavailable",
                [
                    ("item", item_id),
                    ("reason", reason),
                    ("ruleLine", rule_line.unwrap_or(0).to_string()),
                ],
            ),
            Self::ItemInscribed {
                target_kind_id,
                inscription: Some(inscription),
                rule_line,
            } => dto(
                "item.inscribe",
                "item-inscribe-success",
                [
                    ("target", target_kind_id),
                    ("inscription", inscription),
                    ("ruleLine", rule_line.unwrap_or(0).to_string()),
                ],
            ),
            Self::ItemInscribed {
                target_kind_id,
                inscription: None,
                rule_line,
            } => dto(
                "item.inscribe.cleared",
                "item-inscribe-cleared",
                [
                    ("target", target_kind_id),
                    ("ruleLine", rule_line.unwrap_or(0).to_string()),
                ],
            ),
            Self::ItemInscribeUnavailable { item_id, reason } => dto(
                "item.inscribe.unavailable",
                "item-inscribe-unavailable",
                [("item", item_id), ("reason", reason)],
            ),
            Self::ItemsDropped { stacks, quantity } => dto(
                "item.drop",
                "item-drop-success",
                [
                    ("stacks", stacks.to_string()),
                    ("quantity", quantity.to_string()),
                ],
            ),
            Self::NoItemsDropped => dto_without_args("item.drop.none", "item-drop-none"),
            Self::CaptureBallCaptured { target_kind_id } => dto(
                "capture-ball.captured",
                "capture-ball-captured",
                [("target", target_kind_id)],
            ),
            Self::CaptureBallCaptureFailed {
                target_kind_id,
                reason,
            } => dto(
                "capture-ball.capture-failed",
                "capture-ball-capture-failed",
                [("target", target_kind_id), ("reason", reason)],
            ),
            Self::CaptureBallReleased {
                target_kind_id,
                hostile,
            } => dto(
                "capture-ball.released",
                if hostile {
                    "capture-ball-released-hostile"
                } else {
                    "capture-ball-released"
                },
                [("target", target_kind_id)],
            ),
            Self::CaptureBallReleaseFailed { target_kind_id } => dto(
                "capture-ball.release-failed",
                "capture-ball-release-failed",
                [("target", target_kind_id)],
            ),
            Self::ItemEquipped {
                target_kind_id,
                slot_id,
                replaced_kind_id: Some(replaced_kind_id),
            } => dto(
                "item.equip.swap",
                "item-equip-swap",
                [
                    ("target", target_kind_id),
                    ("replaced", replaced_kind_id),
                    ("slot", slot_id),
                ],
            ),
            Self::ItemEquipped {
                target_kind_id,
                slot_id,
                replaced_kind_id: None,
            } => dto(
                "item.equip",
                "item-equip-success",
                [("target", target_kind_id), ("slot", slot_id)],
            ),
            Self::ItemEquipUnavailable => {
                dto_without_args("item.equip.none", "item-equip-unavailable")
            }
            Self::ItemPropertyDiscovered {
                target_kind_id,
                property_name_key,
            } => dto(
                "item.property-discovered",
                "item-property-discovered",
                [
                    ("target", target_kind_id),
                    ("propertyNameKey", property_name_key),
                ],
            ),
            Self::LootDropped {
                source_kind_id,
                target_kind_id,
                quantity,
            } => dto(
                "loot.drop",
                "loot-drop",
                [
                    ("source", source_kind_id),
                    ("target", target_kind_id),
                    ("quantity", quantity.to_string()),
                ],
            ),
            Self::GoldDropped {
                source_kind_id,
                amount,
            } => dto(
                "gold.drop",
                "gold-drop",
                [("source", source_kind_id), ("amount", amount.to_string())],
            ),
            Self::ExperienceGained { amount, total } => dto(
                "player.experience-gained",
                "player-experience-gained",
                [("amount", amount.to_string()), ("total", total.to_string())],
            ),
            Self::ExperienceDrained {
                source_kind_id,
                amount,
                total,
            } => dto(
                "player.experience-drained",
                "player-experience-drained",
                [
                    ("source", source_kind_id),
                    ("amount", amount.to_string()),
                    ("total", total.to_string()),
                ],
            ),
            Self::MonsterUnlifeDrained {
                source_kind_id,
                amount,
                life_force_before,
                life_force_after,
                power_before,
                power_after,
            } => dto(
                "combat.monster-unlife-drained",
                "monster-unlife-drained",
                [
                    ("source", source_kind_id),
                    ("amount", amount.to_string()),
                    ("lifeForceBefore", life_force_before.to_string()),
                    ("lifeForceAfter", life_force_after.to_string()),
                    ("powerBefore", power_before.to_string()),
                    ("powerAfter", power_after.to_string()),
                ],
            ),
            Self::MonsterUnlifeWeakened {
                source_kind_id,
                target_kind_id,
                amount,
                power_before,
                power_after,
            } => dto(
                "combat.monster-unlife-weakened",
                "monster-unlife-weakened",
                [
                    ("source", source_kind_id),
                    ("target", target_kind_id),
                    ("amount", amount.to_string()),
                    ("powerBefore", power_before.to_string()),
                    ("powerAfter", power_after.to_string()),
                ],
            ),
            Self::PlayerLevelGained {
                level,
                max_hp,
                pending_attribute_increases,
                reached_new_maximum: _,
            } => dto(
                "player.level-gained",
                "player-level-gained",
                [
                    ("level", level.to_string()),
                    ("maxHp", max_hp.to_string()),
                    (
                        "pendingAttributeIncreases",
                        pending_attribute_increases.to_string(),
                    ),
                ],
            ),
            Self::PlayerLevelLost { level, max_hp } => dto(
                "player.level-lost",
                "player-level-lost",
                [("level", level.to_string()), ("maxHp", max_hp.to_string())],
            ),
            Self::PlayerLevelCapUnlocked {
                level_cap,
                attribute_index_cap,
            } => dto(
                "player.level-cap-unlocked",
                "player-level-cap-unlocked",
                [
                    ("levelCap", level_cap.to_string()),
                    ("attributeIndexCap", attribute_index_cap.to_string()),
                ],
            ),
            Self::PlayerAttributeIncreased {
                attribute,
                natural,
                effective,
                index,
                pending_attribute_increases,
            } => dto(
                "player.attribute-increased",
                "player-attribute-increased",
                [
                    ("attribute", attribute_kind_id(attribute).to_owned()),
                    ("natural", natural.to_string()),
                    ("effective", effective.to_string()),
                    ("index", index.to_string()),
                    (
                        "pendingAttributeIncreases",
                        pending_attribute_increases.to_string(),
                    ),
                ],
            ),
            Self::PlayerAttributeIncreaseUnavailable { attribute } => dto(
                "player.attribute-increase-unavailable",
                "player-attribute-increase-unavailable",
                [("attribute", attribute_kind_id(attribute).to_owned())],
            ),
            Self::FloorTransitioned {
                from_floor_id,
                to_floor_id,
            } => dto(
                "floor.transition",
                "floor-transition",
                [("from", from_floor_id), ("to", to_floor_id)],
            ),
            Self::FloorTransitionUnavailable => dto_without_args(
                "floor.transition-unavailable",
                "floor-transition-unavailable",
            ),
            Self::DungeonExpeditionEnded => {
                dto_without_args("floor.expedition-ended", "floor-expedition-ended")
            }
            Self::DungeonGuardianDefeated {
                dungeon_id,
                floor_id,
                target_kind_id,
            } => dto(
                "dungeon.guardian-defeated",
                "dungeon-guardian-defeated",
                [
                    ("dungeon", dungeon_id),
                    ("floor", floor_id),
                    ("target", target_kind_id),
                ],
            ),
            Self::DungeonEntranceGuardianDefeated {
                dungeon_id,
                target_kind_id,
            } => dto(
                "dungeon.entrance-guardian-defeated",
                "dungeon-entrance-guardian-defeated",
                [("dungeon", dungeon_id), ("target", target_kind_id)],
            ),
            Self::CampaignVictorious { score } => dto(
                "campaign.victorious",
                "campaign-victorious",
                [("score", score.to_string())],
            ),
            Self::CampaignRetired { score } => dto(
                "campaign.retired",
                "campaign-retired",
                [("score", score.to_string())],
            ),
            Self::CampaignRetireUnavailable => {
                dto_without_args("campaign.retire-unavailable", "campaign-retire-unavailable")
            }
            Self::OneShotFloorClosed { floor_id } => dto(
                "floor.one-shot-closed",
                "floor-one-shot-closed",
                [("floor", floor_id)],
            ),
            Self::TaskCompleted { floor_id } => {
                dto("task.completed", "task-completed", [("floor", floor_id)])
            }
            Self::TaskRewardAvailable { floor_id } => dto(
                "task.reward-available",
                "task-reward-available",
                [("floor", floor_id)],
            ),
            Self::TaskExitRevealed { floor_id, position } => dto(
                "task.exit-revealed",
                "task-exit-revealed",
                [
                    ("floor", floor_id),
                    ("x", position.x.to_string()),
                    ("y", position.y.to_string()),
                ],
            ),
            Self::TaskAccepted { task_id } => {
                dto("task.accepted", "task-accepted", [("task", task_id)])
            }
            Self::TaskAcceptUnavailable {
                facility_id,
                task_id,
                reason,
            } => dto(
                "task.accept-unavailable",
                "task-accept-unavailable",
                [
                    ("facility", facility_id),
                    ("task", task_id),
                    ("reason", reason),
                ],
            ),
            Self::TaskFailed { floor_id } => {
                dto("task.failed", "task-failed", [("floor", floor_id)])
            }
            Self::TaskAbandoned { floor_id } => {
                dto("task.abandoned", "task-abandoned", [("floor", floor_id)])
            }
            Self::TaskAbandonUnavailable => {
                dto_without_args("task.abandon-unavailable", "task-abandon-unavailable")
            }
            Self::TaskPaused { floor_id } => {
                dto("task.paused", "task-paused", [("floor", floor_id)])
            }
            Self::TaskResumed { floor_id } => {
                dto("task.resumed", "task-resumed", [("floor", floor_id)])
            }
            Self::TaskRewarded {
                item_kind_id,
                quantity,
            } => dto(
                "task.rewarded",
                "task-rewarded",
                [("target", item_kind_id), ("quantity", quantity.to_string())],
            ),
            Self::TaskRewardClaimUnavailable {
                facility_id,
                task_id,
                reason,
            } => dto(
                "task.reward-claim-unavailable",
                "task-reward-claim-unavailable",
                [
                    ("facility", facility_id),
                    ("task", task_id),
                    ("reason", reason),
                ],
            ),
            Self::DoorOpened { position } => dto(
                "terrain.door-opened",
                "door-opened",
                [("x", position.x.to_string()), ("y", position.y.to_string())],
            ),
            Self::DoorUnlocked { position } => dto(
                "terrain.door-unlocked",
                "door-unlocked",
                [("x", position.x.to_string()), ("y", position.y.to_string())],
            ),
            Self::DoorUnlockFailed { position } => dto(
                "terrain.door-unlock-failed",
                "door-unlock-failed",
                [("x", position.x.to_string()), ("y", position.y.to_string())],
            ),
            Self::DoorOpenUnavailable => {
                dto_without_args("terrain.door-open-unavailable", "door-open-unavailable")
            }
            Self::DoorBashedOpen { position } => dto(
                "terrain.door-bashed-open",
                "door-bashed-open",
                [("x", position.x.to_string()), ("y", position.y.to_string())],
            ),
            Self::DoorBashFailed { position } => dto(
                "terrain.door-bash-failed",
                "door-bash-failed",
                [("x", position.x.to_string()), ("y", position.y.to_string())],
            ),
            Self::DoorBashUnavailable => {
                dto_without_args("terrain.door-bash-unavailable", "door-bash-unavailable")
            }
            Self::SecretTerrainDiscovered { position } => dto(
                "terrain.secret-discovered",
                "terrain-secret-discovered",
                [("x", position.x.to_string()), ("y", position.y.to_string())],
            ),
            Self::SearchFoundNothing => {
                dto_without_args("terrain.search-empty", "terrain-search-empty")
            }
            Self::TrapTriggered { position, damage } => dto_with_outcome(
                "terrain.trap-triggered",
                "terrain-trap-triggered",
                [("x", position.x.to_string()), ("y", position.y.to_string())],
                GameEventOutcomeDto::Damage {
                    resolution: damage.into(),
                },
            ),
            Self::ActorTrapTriggered {
                position,
                target_kind_id,
                damage,
            } => dto_with_outcome(
                "terrain.actor-trap-triggered",
                "terrain-trap-triggered",
                [
                    ("target", target_kind_id.clone()),
                    ("x", position.x.to_string()),
                    ("y", position.y.to_string()),
                ],
                GameEventOutcomeDto::Damage {
                    resolution: damage.into(),
                },
            ),
            Self::TrapDisarmed { position } => dto(
                "terrain.trap-disarmed",
                "terrain-trap-disarmed",
                [("x", position.x.to_string()), ("y", position.y.to_string())],
            ),
            Self::TrapDisarmFailed { position } => dto(
                "terrain.trap-disarm-failed",
                "terrain-trap-disarm-failed",
                [("x", position.x.to_string()), ("y", position.y.to_string())],
            ),
            Self::TrapDisarmUnavailable => dto_without_args(
                "terrain.trap-disarm-unavailable",
                "terrain-trap-disarm-unavailable",
            ),
            Self::DeviceSkillChecked {
                source_kind_id,
                succeeded,
                resolution,
            } => dto_with_outcome(
                if succeeded {
                    "skill.device-success"
                } else {
                    "skill.device-failure"
                },
                if succeeded {
                    "skill-check-device-success"
                } else {
                    "skill-check-device-failure"
                },
                [("target", source_kind_id)],
                GameEventOutcomeDto::Check { resolution },
            ),
            Self::DeviceEnergyRecovered {
                target_item_id,
                target_kind_id,
                amount,
                current,
                maximum,
            } => dto(
                "device.energy-recovered",
                "device-energy-recovered",
                [
                    ("target", target_kind_id),
                    ("targetItem", target_item_id),
                    ("amount", amount.to_string()),
                    ("current", current.to_string()),
                    ("maximum", maximum.to_string()),
                ],
            ),
            Self::DeviceRechargeResolved {
                target_item_id,
                target_kind_id,
                source_id,
                source_is_item,
                attempted,
                target_before,
                target_after,
                succeeded,
                failure_one_in,
                failure_roll,
                source_destroyed,
            } => dto(
                if succeeded {
                    "device.recharge-success"
                } else {
                    "device.recharge-failure"
                },
                if succeeded {
                    "device-recharge-success"
                } else {
                    "device-recharge-failure"
                },
                [
                    ("target", target_kind_id),
                    ("targetItem", target_item_id),
                    ("source", source_id),
                    (
                        "sourceType",
                        if source_is_item { "item" } else { "resource" }.to_owned(),
                    ),
                    ("attempted", attempted.to_string()),
                    ("before", target_before.to_string()),
                    ("after", target_after.to_string()),
                    ("failureOneIn", failure_one_in.to_string()),
                    (
                        "failureRoll",
                        failure_roll
                            .map_or_else(|| "automatic".to_owned(), |roll| roll.to_string()),
                    ),
                    ("sourceDestroyed", source_destroyed.to_string()),
                ],
            ),
            Self::LightRefuelUnavailable {
                target_item_id,
                source_item_id,
                reason,
            } => dto(
                "light.refuel-unavailable",
                "light-refuel-unavailable",
                [
                    ("targetItem", target_item_id),
                    ("sourceItem", source_item_id),
                    ("reason", reason),
                ],
            ),
            Self::LightRefueled {
                target_item_id,
                target_kind_id,
                source_kind_id,
                amount,
                current,
                maximum,
            } => dto(
                "light.refueled",
                "light-refueled",
                [
                    ("targetItem", target_item_id),
                    ("target", target_kind_id),
                    ("source", source_kind_id),
                    ("amount", amount.to_string()),
                    ("current", current.to_string()),
                    ("maximum", maximum.to_string()),
                ],
            ),
            Self::LightExtinguished {
                target_item_id,
                target_kind_id,
            } => dto(
                "light.extinguished",
                "light-extinguished",
                [("targetItem", target_item_id), ("target", target_kind_id)],
            ),
            Self::SavingThrowChecked {
                source_kind_id,
                position,
                succeeded,
                resolution,
            } => dto_with_outcome(
                if succeeded {
                    "skill.saving-throw-success"
                } else {
                    "skill.saving-throw-failure"
                },
                if succeeded {
                    "skill-check-saving-throw-success"
                } else {
                    "skill-check-saving-throw-failure"
                },
                [
                    ("source", source_kind_id),
                    ("x", position.x.to_string()),
                    ("y", position.y.to_string()),
                ],
                GameEventOutcomeDto::Check { resolution },
            ),
            Self::StealthChecked {
                source_kind_id,
                succeeded,
                resolution,
            } => dto_with_outcome(
                if succeeded {
                    "skill.stealth-success"
                } else {
                    "skill.stealth-failure"
                },
                if succeeded {
                    "skill-check-stealth-success"
                } else {
                    "skill-check-stealth-failure"
                },
                [("source", source_kind_id)],
                GameEventOutcomeDto::Check { resolution },
            ),
            Self::PerceptionChecked {
                position,
                succeeded: true,
                resolution,
            } => dto_with_outcome(
                "skill.perception-success",
                "skill-check-perception-success",
                [("x", position.x.to_string()), ("y", position.y.to_string())],
                GameEventOutcomeDto::Check { resolution },
            ),
            Self::PerceptionChecked {
                succeeded: false,
                resolution,
                ..
            } => dto_with_outcome(
                "skill.perception-failure",
                "skill-check-perception-failure",
                [],
                GameEventOutcomeDto::Check { resolution },
            ),
            Self::ItemWarnedOfTrap { position } => dto(
                "item.warning-trap",
                "item-warning-trap",
                [("x", position.x.to_string()), ("y", position.y.to_string())],
            ),
            Self::TerrainDug { position } => dto(
                "terrain.dug",
                "terrain-dug",
                [("x", position.x.to_string()), ("y", position.y.to_string())],
            ),
            Self::TerrainDigFailed {
                position,
                retryable,
            } => dto(
                "terrain.dig-failed",
                "terrain-dig-failed",
                [
                    ("x", position.x.to_string()),
                    ("y", position.y.to_string()),
                    ("retryable", retryable.to_string()),
                ],
            ),
            Self::TerrainDigUnavailable => {
                dto_without_args("terrain.dig-unavailable", "terrain-dig-unavailable")
            }
            Self::DoorClosed { position } => dto(
                "terrain.door-closed",
                "door-closed",
                [("x", position.x.to_string()), ("y", position.y.to_string())],
            ),
            Self::DoorCloseUnavailable => {
                dto_without_args("terrain.door-close-unavailable", "door-close-unavailable")
            }
            Self::Waited => dto_without_args("turn.wait", "game-wait"),
            Self::ItemPickedUp {
                target_kind_id,
                quantity,
            } => dto(
                "item.pickup",
                "item-pickup-success",
                [
                    ("target", target_kind_id),
                    ("quantity", quantity.to_string()),
                ],
            ),
            Self::GoldPickedUp { amount, balance } => dto(
                "gold.pickup",
                "gold-pickup-success",
                [
                    ("amount", amount.to_string()),
                    ("balance", balance.to_string()),
                ],
            ),
            Self::ShopPurchaseCompleted { outcome } => dto(
                "shop.purchase",
                "shop-purchase-success",
                [
                    ("shop", outcome.shop_id.clone()),
                    ("item", outcome.item_id.clone()),
                    ("target", outcome.item_kind_id.clone()),
                    ("quantity", outcome.quantity.to_string()),
                    ("unitPrice", outcome.unit_price.to_string()),
                    ("totalPrice", outcome.total_price.to_string()),
                    ("balance", outcome.gold_balance.to_string()),
                ],
            ),
            Self::ShopSaleCompleted { outcome } => dto(
                "shop.sale",
                "shop-sale-success",
                [
                    ("shop", outcome.shop_id.clone()),
                    ("item", outcome.item_id.clone()),
                    ("target", outcome.item_kind_id.clone()),
                    ("quantity", outcome.quantity.to_string()),
                    ("unitPrice", outcome.unit_price.to_string()),
                    ("totalPrice", outcome.total_price.to_string()),
                    ("balance", outcome.gold_balance.to_string()),
                ],
            ),
            Self::ShopTransactionUnavailable {
                shop_id,
                item_id,
                reason,
            } => dto(
                "shop.transaction-unavailable",
                "shop-transaction-unavailable",
                [
                    ("shop", shop_id.clone()),
                    ("item", item_id.clone()),
                    ("reason", reason.clone()),
                ],
            ),
            Self::FacilityIdentifyUnavailable {
                facility_id,
                item_id,
                reason,
            } => dto(
                "facility.identify-unavailable",
                "facility-identify-unavailable",
                [
                    ("facility", facility_id.clone()),
                    ("item", item_id.clone()),
                    ("reason", reason.clone()),
                ],
            ),
            Self::FacilityItemIdentified { outcome } => dto_with_outcome(
                "facility.identified",
                "facility-identify-completed",
                [
                    ("facility", outcome.facility_id.clone()),
                    ("target", outcome.resolution.item_kind_id.clone()),
                    ("cost", outcome.cost.to_string()),
                    ("balance", outcome.gold_balance.to_string()),
                ],
                GameEventOutcomeDto::ItemIdentify {
                    resolution: outcome.resolution.clone(),
                },
            ),
            Self::FacilityRenameUnavailable {
                facility_id,
                reason,
            } => dto(
                "facility.rename-unavailable",
                "facility-rename-unavailable",
                [
                    ("facility", facility_id.clone()),
                    ("reason", reason.clone()),
                ],
            ),
            Self::FacilityPlayerRenamed { outcome } => dto(
                "facility.renamed",
                "facility-rename-completed",
                [
                    ("facility", outcome.facility_id.clone()),
                    ("previousName", outcome.previous_name.clone()),
                    ("name", outcome.name.clone()),
                    ("cost", outcome.cost.to_string()),
                    ("balance", outcome.gold_balance.to_string()),
                ],
            ),
            Self::InnStayUnavailable {
                facility_id,
                reason,
            } => dto(
                "inn.stay-unavailable",
                "inn-stay-unavailable",
                [
                    ("facility", facility_id.clone()),
                    ("reason", reason.clone()),
                ],
            ),
            Self::InnStayCompleted { outcome } => dto(
                "inn.stay",
                "inn-stay-completed",
                [
                    ("facility", outcome.facility_id.clone()),
                    ("cost", outcome.cost.to_string()),
                    ("balance", outcome.gold_balance.to_string()),
                    ("elapsedTicks", outcome.elapsed_ticks.to_string()),
                    ("worldTick", outcome.world_tick.to_string()),
                ],
            ),
            Self::InnTravelUnavailable {
                facility_id,
                destination_town_id,
                reason,
            } => dto(
                "inn.travel-unavailable",
                "inn-travel-unavailable",
                [
                    ("facility", facility_id.clone()),
                    ("destinationTown", destination_town_id.clone()),
                    ("reason", reason.clone()),
                ],
            ),
            Self::InnTravelCompleted { outcome } => dto(
                "inn.travel",
                "inn-travel-completed",
                [
                    ("facility", outcome.facility_id.clone()),
                    ("destinationTown", outcome.destination_town_id.clone()),
                    ("cost", outcome.cost.to_string()),
                    ("balance", outcome.gold_balance.to_string()),
                ],
            ),
            Self::HomeItemDeposited { outcome } => dto(
                "home.deposit",
                "home-deposit-success",
                [
                    ("facility", outcome.facility_id.clone()),
                    ("item", outcome.item_id.clone()),
                    ("target", outcome.item_kind_id.clone()),
                    ("quantity", outcome.quantity.to_string()),
                ],
            ),
            Self::HomeItemWithdrawn { outcome } => dto(
                "home.withdraw",
                "home-withdraw-success",
                [
                    ("facility", outcome.facility_id.clone()),
                    ("item", outcome.item_id.clone()),
                    ("target", outcome.item_kind_id.clone()),
                    ("quantity", outcome.quantity.to_string()),
                ],
            ),
            Self::HomeTransferUnavailable {
                facility_id,
                item_id,
                reason,
            } => dto(
                "home.transfer-unavailable",
                "home-transfer-unavailable",
                [
                    ("facility", facility_id.clone()),
                    ("item", item_id.clone()),
                    ("reason", reason.clone()),
                ],
            ),
            Self::ItemPickupInventoryFull {
                target_kind_id,
                quantity,
                used_slots,
                required_slots,
                capacity,
            } => dto(
                "item.pickup.inventory-full",
                "item-pickup-inventory-full",
                [
                    ("target", target_kind_id),
                    ("quantity", quantity.to_string()),
                    ("usedSlots", used_slots.to_string()),
                    ("requiredSlots", required_slots.to_string()),
                    ("capacity", capacity.to_string()),
                ],
            ),
            Self::NothingToPickUp => dto_without_args("item.pickup.none", "item-pickup-none"),
            Self::ItemUnequipped {
                target_kind_id,
                slot_id,
            } => dto(
                "item.unequip",
                "item-unequip-success",
                [("target", target_kind_id), ("slot", slot_id)],
            ),
            Self::ItemUnequipUnavailable { slot_id } => dto(
                "item.unequip.none",
                "item-unequip-none",
                [("slot", slot_id)],
            ),
            Self::ItemUnequipCursed {
                target_kind_id,
                slot_id,
                severity,
            } => dto(
                "item.unequip.cursed",
                "item-unequip-cursed",
                [
                    ("target", target_kind_id),
                    ("slot", slot_id),
                    ("severity", curse_severity_arg(severity).to_owned()),
                ],
            ),
            Self::MoveBlocked => dto_without_args("move.blocked", "game-move-blocked"),
            Self::WildernessAmbushed => {
                dto_without_args("wilderness.ambushed", "wilderness-ambushed")
            }
            Self::WildernessInterestingDiscovery => dto_without_args(
                "wilderness.interesting-discovery",
                "wilderness-interesting-discovery",
            ),
            Self::WildernessTerrainDamaged { terrain_id, damage } => dto_with_outcome(
                "wilderness.terrain-damaged",
                "wilderness-terrain-damaged",
                [("terrain", terrain_id)],
                GameEventOutcomeDto::Damage {
                    resolution: damage.into(),
                },
            ),
            Self::RidingMounted { target_kind_id } => dto(
                "riding.mounted",
                "riding-mounted",
                [("target", target_kind_id)],
            ),
            Self::RidingDismounted { target_kind_id } => dto(
                "riding.dismounted",
                "riding-dismounted",
                [("target", target_kind_id)],
            ),
            Self::RidingFailed { target_kind_id } => dto(
                "riding.failed",
                "riding-failed",
                [("target", target_kind_id)],
            ),
            Self::RidingNotPet { target_kind_id } => dto(
                "riding.not-pet",
                "riding-not-pet",
                [("target", target_kind_id)],
            ),
            Self::RidingFell {
                target_kind_id,
                damage,
            } => dto_with_outcome(
                "riding.fell",
                "riding-fell",
                [("target", target_kind_id)],
                GameEventOutcomeDto::Damage {
                    resolution: damage.into(),
                },
            ),
            Self::RidingCollided {
                target_kind_id,
                damage,
            } => dto_with_outcome(
                "riding.collided",
                "riding-collided",
                [("target", target_kind_id)],
                GameEventOutcomeDto::Damage {
                    resolution: damage.into(),
                },
            ),
            Self::RodeoAlreadyRiding => {
                dto_without_args("rodeo.already-riding", "rodeo-already-riding")
            }
            Self::RodeoUntameable { target_kind_id } => dto(
                "rodeo.untameable",
                "rodeo-untameable",
                [("target", target_kind_id)],
            ),
            Self::RodeoTooWeak { target_kind_id } => dto(
                "rodeo.too-weak",
                "rodeo-too-weak",
                [("target", target_kind_id)],
            ),
            Self::RodeoTamed { target_kind_id } => {
                dto("rodeo.tamed", "rodeo-tamed", [("target", target_kind_id)])
            }
            Self::RodeoThrownOff { target_kind_id } => dto(
                "rodeo.thrown-off",
                "rodeo-thrown-off",
                [("target", target_kind_id)],
            ),
            Self::RidingBondMaxed { target_kind_id } => dto(
                "riding.bond-maxed",
                "riding-bond-maxed",
                [("target", target_kind_id)],
            ),
            Self::PetEvolved {
                previous_kind_id,
                target_kind_id,
            } => dto(
                "pet.evolved",
                "pet-evolved",
                [("source", previous_kind_id), ("target", target_kind_id)],
            ),
            Self::MountPotionUsed {
                item_kind_id,
                target_kind_id,
            } => dto(
                "riding.mount-potion-used",
                "riding-mount-potion-used",
                [("item", item_kind_id), ("target", target_kind_id)],
            ),
            Self::RidingUnavailable => dto_without_args("riding.unavailable", "riding-unavailable"),
            Self::SheepRidingRefused { response } => dto_without_args(
                "riding.sheep-refused",
                match response {
                    0 => "riding-sheep-refused-0",
                    1 => "riding-sheep-refused-1",
                    _ => "riding-sheep-refused-2",
                },
            ),
            Self::ProjectileUnavailable => {
                dto_without_args("combat.projectile-unavailable", "projectile-unavailable")
            }
            Self::ProjectileAmmoUnavailable { ammo_kind_id } => dto(
                "combat.projectile-ammo-unavailable",
                "projectile-ammo-unavailable",
                [("target", ammo_kind_id)],
            ),
            Self::ProjectileTargetUnavailable => dto_without_args(
                "combat.projectile-target-unavailable",
                "projectile-target-unavailable",
            ),
            Self::ProjectileLanded { trace } => with_trace(
                dto_without_args("combat.projectile-landed", "projectile-landed"),
                trace,
            ),
            Self::ProjectileMissed {
                target_kind_id,
                trace,
            } => with_trace(
                dto(
                    "combat.projectile-miss",
                    "projectile-miss",
                    [("target", target_kind_id)],
                ),
                trace,
            ),
            Self::ProjectileHit {
                target_kind_id,
                damage,
                trace,
            } => with_trace(
                dto_with_outcome(
                    "combat.projectile-hit",
                    "projectile-hit",
                    [
                        ("target", target_kind_id),
                        ("damage", damage.applied.to_string()),
                    ],
                    GameEventOutcomeDto::Damage {
                        resolution: damage.into(),
                    },
                ),
                trace,
            ),
            Self::ProjectileSlew {
                target_kind_id,
                damage,
                trace,
            } => with_trace(
                dto_with_outcome(
                    "combat.projectile-slay",
                    "projectile-slay",
                    [("target", target_kind_id)],
                    GameEventOutcomeDto::Death {
                        resolution: damage.into(),
                    },
                ),
                trace,
            ),
            Self::ProjectileAmmoRecovered { ammo_kind_id } => dto(
                "combat.projectile-ammo-recovered",
                "projectile-ammo-recovered",
                [("target", ammo_kind_id)],
            ),
            Self::ProjectileAmmoBroken { ammo_kind_id } => dto(
                "combat.projectile-ammo-broken",
                "projectile-ammo-broken",
                [("target", ammo_kind_id)],
            ),
            Self::ItemThrown {
                target_kind_id,
                trace,
            } => with_trace(
                dto("item.thrown", "item-thrown", [("target", target_kind_id)]),
                trace,
            ),
            Self::ItemThrowMissed {
                source_kind_id,
                target_kind_id,
                trace,
            } => with_trace(
                dto(
                    "combat.throw-miss",
                    "throw-miss",
                    [("source", source_kind_id), ("target", target_kind_id)],
                ),
                trace,
            ),
            Self::ItemThrowHit {
                source_kind_id,
                target_kind_id,
                damage,
                trace,
            } => with_trace(
                dto_with_outcome(
                    "combat.throw-hit",
                    "throw-hit",
                    [
                        ("source", source_kind_id),
                        ("target", target_kind_id),
                        ("damage", damage.applied.to_string()),
                    ],
                    GameEventOutcomeDto::Damage {
                        resolution: damage.into(),
                    },
                ),
                trace,
            ),
            Self::ItemThrowSlew {
                source_kind_id,
                target_kind_id,
                damage,
                trace,
            } => with_trace(
                dto_with_outcome(
                    "combat.throw-slay",
                    "throw-slay",
                    [("source", source_kind_id), ("target", target_kind_id)],
                    GameEventOutcomeDto::Death {
                        resolution: damage.into(),
                    },
                ),
                trace,
            ),
            Self::ItemThrowUnavailable => {
                dto_without_args("item.throw-unavailable", "item-throw-unavailable")
            }
            Self::DeviceAbsorbed {
                item_id,
                item_kind_id,
                charges_before,
                charges_after,
                drained,
                nutrition_before,
                nutrition_after,
            } => dto(
                "item.device-absorbed",
                if drained == 0 {
                    "item-device-empty"
                } else {
                    "item-device-absorbed"
                },
                [
                    ("item", item_id),
                    ("target", item_kind_id),
                    ("amount", drained.to_string()),
                    ("chargesBefore", charges_before.to_string()),
                    ("chargesAfter", charges_after.to_string()),
                    ("nutritionBefore", nutrition_before.to_string()),
                    ("nutritionAfter", nutrition_after.to_string()),
                ],
            ),
            Self::DeviceAbsorptionUnavailable { item_id } => dto(
                "item.device-absorb-unavailable",
                "item-device-absorb-unavailable",
                [("item", item_id)],
            ),
            Self::ItemUsed {
                source_kind_id,
                display_name_key,
                requested,
                applied,
            } => dto_with_outcome(
                if applied > 0 {
                    "item.use-heal"
                } else {
                    "item.use-no-effect"
                },
                if applied > 0 {
                    "item-use-heal"
                } else {
                    "item-use-no-effect"
                },
                [
                    ("target", source_kind_id),
                    ("nameKey", display_name_key),
                    ("amount", applied.to_string()),
                ],
                GameEventOutcomeDto::Heal {
                    resolution: HealingResolutionDto { requested, applied },
                },
            ),
            Self::ItemNutritionIncreased {
                source_kind_id,
                display_name_key,
                amount,
                nutrition,
            } => dto(
                "item.use-food",
                "item-use-food",
                [
                    ("target", source_kind_id),
                    ("nameKey", display_name_key),
                    ("amount", amount.to_string()),
                    ("nutrition", nutrition.to_string()),
                ],
            ),
            Self::ItemNutritionSatisfied {
                source_kind_id,
                display_name_key,
                nutrition,
                noticed,
            } => dto(
                if noticed {
                    "item.use-hunger-satisfied"
                } else {
                    "item.use-hunger-no-effect"
                },
                if noticed {
                    "item-use-hunger-satisfied"
                } else {
                    "item-use-hunger-no-effect"
                },
                [
                    ("target", source_kind_id),
                    ("nameKey", display_name_key),
                    ("nutrition", nutrition.to_string()),
                ],
            ),
            Self::ItemExperienceLost {
                source_kind_id,
                display_name_key,
                amount,
                remaining,
            } => dto(
                "item.experience-lost",
                "item-experience-lost",
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("amount", amount.to_string()),
                    ("remaining", remaining.to_string()),
                ],
            ),
            Self::ItemStatusResolved {
                source_kind_id,
                display_name_key,
                status_kind_id,
                duration,
                noticed,
            } => dto(
                if duration.is_none() {
                    "item.use-status-resisted"
                } else if noticed {
                    "item.use-status-applied"
                } else {
                    "item.use-status-no-new-effect"
                },
                if duration.is_none() {
                    "item-use-status-resisted"
                } else if noticed {
                    "item-use-status-applied"
                } else {
                    "item-use-status-no-new-effect"
                },
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("status", status_kind_id),
                    ("duration", duration.unwrap_or_default().to_string()),
                ],
            ),
            Self::ItemStatusRemoved {
                source_kind_id,
                display_name_key,
                status_kind_id,
                removed,
            } => dto(
                if removed {
                    "item.use-status-removed"
                } else {
                    "item.use-status-no-effect"
                },
                if removed {
                    "item-use-status-removed"
                } else {
                    "item-use-status-no-effect"
                },
                [
                    ("target", source_kind_id),
                    ("nameKey", display_name_key),
                    ("status", status_kind_id),
                ],
            ),
            Self::ItemStatusReduced {
                source_kind_id,
                display_name_key,
                status_kind_id,
                before,
                after,
            } => dto(
                if after < before {
                    "item.use-status-reduced"
                } else {
                    "item.use-status-no-effect"
                },
                if after < before {
                    "item-use-status-reduced"
                } else {
                    "item-use-status-no-effect"
                },
                [
                    ("target", source_kind_id),
                    ("nameKey", display_name_key),
                    ("status", status_kind_id),
                    ("before", before.to_string()),
                    ("after", after.to_string()),
                ],
            ),
            Self::ItemBlessed {
                source_kind_id,
                display_name_key,
                duration,
                resolution,
            } => dto_with_outcome(
                "item.use-blessed",
                "item-use-blessed",
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("duration", duration.to_string()),
                ],
                GameEventOutcomeDto::AbilityEffects { resolution },
            ),
            Self::ItemSlownessResolved {
                source_kind_id,
                display_name_key,
                duration,
                noticed,
            } => dto(
                if noticed {
                    "item.use-slowness-applied"
                } else {
                    "item.use-slowness-no-effect"
                },
                if noticed {
                    "item-use-slowness-applied"
                } else {
                    "item-use-slowness-no-effect"
                },
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("duration", duration.to_string()),
                ],
            ),
            Self::ItemSpeedResolved {
                source_kind_id,
                display_name_key,
                duration,
            } => dto(
                "item.use-speed",
                "item-use-speed",
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("duration", duration.to_string()),
                ],
            ),
            Self::ItemHeroismResolved {
                source_kind_id,
                display_name_key,
                duration,
                noticed,
            } => dto(
                if noticed {
                    "item.use-heroism-applied"
                } else {
                    "item.use-heroism-no-new-effect"
                },
                if noticed {
                    "item-use-heroism-applied"
                } else {
                    "item-use-heroism-no-new-effect"
                },
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("duration", duration.to_string()),
                ],
            ),
            Self::ItemBerserkStrengthResolved {
                source_kind_id,
                display_name_key,
                duration,
                noticed,
            } => dto(
                if noticed {
                    "item.use-berserk-strength-applied"
                } else {
                    "item.use-berserk-strength-no-new-effect"
                },
                if noticed {
                    "item-use-berserk-strength-applied"
                } else {
                    "item-use-berserk-strength-no-new-effect"
                },
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("duration", duration.to_string()),
                ],
            ),
            Self::ItemPoeticInspirationResolved {
                source_kind_id,
                display_name_key,
                duration,
                noticed,
            } => dto(
                if noticed {
                    "item.use-poetic-inspiration-applied"
                } else {
                    "item.use-poetic-inspiration-no-new-effect"
                },
                if noticed {
                    "item-use-poetic-inspiration-applied"
                } else {
                    "item-use-poetic-inspiration-no-new-effect"
                },
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("duration", duration.to_string()),
                ],
            ),
            Self::ItemStoneSkinResolved {
                source_kind_id,
                display_name_key,
                duration,
                noticed,
            } => dto(
                if noticed {
                    "item.use-stone-skin-applied"
                } else {
                    "item.use-stone-skin-no-new-effect"
                },
                if noticed {
                    "item-use-stone-skin-applied"
                } else {
                    "item-use-stone-skin-no-new-effect"
                },
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("duration", duration.to_string()),
                ],
            ),
            Self::ItemRestoreLifeLevelsResolved {
                source_kind_id,
                display_name_key,
                noticed,
            } => dto(
                if noticed {
                    "item.use-restore-life-levels"
                } else {
                    "item.use-restore-life-levels-no-effect"
                },
                if noticed {
                    "item-use-restore-life-levels"
                } else {
                    "item-use-restore-life-levels-no-effect"
                },
                [("source", source_kind_id), ("nameKey", display_name_key)],
            ),
            Self::ItemRestorationResolved {
                source_kind_id,
                display_name_key,
                noticed,
            } => dto(
                if noticed {
                    "item.use-restoration"
                } else {
                    "item.use-restoration-no-effect"
                },
                if noticed {
                    "item-use-restoration"
                } else {
                    "item-use-restoration-no-effect"
                },
                [("source", source_kind_id), ("nameKey", display_name_key)],
            ),
            Self::ItemAttributeChanged {
                source_kind_id,
                display_name_key,
                attribute,
                change,
                before,
                after,
                maximum,
                noticed,
            } => {
                let (message_key, event_key, change_key) = match (change, noticed) {
                    (ItemAttributeChange::Drained, true) => (
                        "item.use-attribute-drained",
                        "item-use-attribute-drained",
                        "drained",
                    ),
                    (ItemAttributeChange::Drained, false) => (
                        "item.use-attribute-drain-no-effect",
                        "item-use-attribute-drain-no-effect",
                        "drained",
                    ),
                    (ItemAttributeChange::Restored, true) => (
                        "item.use-attribute-restored",
                        "item-use-attribute-restored",
                        "restored",
                    ),
                    (ItemAttributeChange::Restored, false) => (
                        "item.use-attribute-restore-no-effect",
                        "item-use-attribute-restore-no-effect",
                        "restored",
                    ),
                    (ItemAttributeChange::Increased, true) => (
                        "item.use-attribute-increased",
                        "item-use-attribute-increased",
                        "increased",
                    ),
                    (ItemAttributeChange::Increased, false) => (
                        "item.use-attribute-increase-no-effect",
                        "item-use-attribute-increase-no-effect",
                        "increased",
                    ),
                    (ItemAttributeChange::Sustained, _) => (
                        "item.use-attribute-sustained",
                        "item-use-attribute-sustained",
                        "sustained",
                    ),
                };
                dto(
                    message_key,
                    event_key,
                    [
                        ("source", source_kind_id),
                        ("nameKey", display_name_key),
                        ("attribute", attribute_kind_id(attribute).to_owned()),
                        ("change", change_key.to_owned()),
                        ("before", before.to_string()),
                        ("after", after.to_string()),
                        ("maximum", maximum.to_string()),
                    ],
                )
            }
            Self::ItemThermalResistanceResolved {
                source_kind_id,
                display_name_key,
                duration,
                noticed,
            } => dto(
                if noticed {
                    "item.use-thermal-resistance-applied"
                } else {
                    "item.use-thermal-resistance-no-effect"
                },
                if noticed {
                    "item-use-thermal-resistance-applied"
                } else {
                    "item-use-thermal-resistance-no-effect"
                },
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("duration", duration.to_string()),
                ],
            ),
            Self::ItemBasicResistanceApplied {
                source_kind_id,
                display_name_key,
                duration,
            } => dto(
                "item.use-basic-resistance",
                "item-use-basic-resistance",
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("duration", duration.to_string()),
                ],
            ),
            Self::ItemPoisonResolved {
                source_kind_id,
                display_name_key,
                duration,
            } => match duration {
                Some(duration) => dto(
                    "item.use-poison-applied",
                    "item-use-poison-applied",
                    [
                        ("source", source_kind_id),
                        ("nameKey", display_name_key),
                        ("duration", duration.to_string()),
                    ],
                ),
                None => dto(
                    "item.use-poison-resisted",
                    "item-use-poison-resisted",
                    [("source", source_kind_id), ("nameKey", display_name_key)],
                ),
            },
            Self::ItemBlindnessResolved {
                source_kind_id,
                display_name_key,
                duration,
                noticed,
            } => match (duration, noticed) {
                (Some(duration), true) => dto(
                    "item.use-blindness-applied",
                    "item-use-blindness-applied",
                    [
                        ("source", source_kind_id),
                        ("nameKey", display_name_key),
                        ("duration", duration.to_string()),
                    ],
                ),
                (Some(duration), false) => dto(
                    "item.use-blindness-no-new-effect",
                    "item-use-blindness-no-new-effect",
                    [
                        ("source", source_kind_id),
                        ("nameKey", display_name_key),
                        ("duration", duration.to_string()),
                    ],
                ),
                (None, _) => dto(
                    "item.use-blindness-resisted",
                    "item-use-blindness-resisted",
                    [("source", source_kind_id), ("nameKey", display_name_key)],
                ),
            },
            Self::ItemResourceDrained {
                source_kind_id,
                display_name_key,
                resource_id,
                drained,
            } => dto(
                if drained > 0 {
                    "item.use-resource-drained"
                } else {
                    "item.use-resource-drain-no-effect"
                },
                if drained > 0 {
                    "item-use-resource-drained"
                } else {
                    "item-use-resource-drain-no-effect"
                },
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("resource", resource_id),
                    ("amount", drained.to_string()),
                ],
            ),
            Self::ItemDetonation {
                source_kind_id,
                display_name_key,
                damage,
                fatal,
            } => dto_with_outcome(
                if fatal {
                    "item.use-detonation-death"
                } else {
                    "item.use-detonation"
                },
                if fatal {
                    "item-use-detonation-death"
                } else {
                    "item-use-detonation"
                },
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("damage", damage.applied.to_string()),
                ],
                if fatal {
                    GameEventOutcomeDto::Death {
                        resolution: damage.into(),
                    }
                } else {
                    GameEventOutcomeDto::Damage {
                        resolution: damage.into(),
                    }
                },
            ),
            Self::ItemLifeLost {
                source_kind_id,
                display_name_key,
                amount,
                fatal,
            } => dto(
                if fatal {
                    "item.use-life-loss-death"
                } else {
                    "item.use-life-loss"
                },
                if fatal {
                    "item-use-life-loss-death"
                } else {
                    "item-use-life-loss"
                },
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("amount", amount.to_string()),
                ],
            ),
            Self::ItemVengeanceActivated {
                source_kind_id,
                display_name_key,
                duration,
                resolution,
            } => dto_with_outcome(
                "item.use-vengeance",
                "item-use-vengeance",
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("duration", duration.to_string()),
                ],
                GameEventOutcomeDto::AbilityEffects { resolution },
            ),
            Self::ItemProtectionFromEvil {
                source_kind_id,
                display_name_key,
                duration,
                resolution,
            } => dto_with_outcome(
                "item.use-protection-from-evil",
                "item-use-protection-from-evil",
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("duration", duration.to_string()),
                ],
                GameEventOutcomeDto::AbilityEffects { resolution },
            ),
            Self::ItemConfusingStrikePrepared {
                source_kind_id,
                display_name_key,
            } => dto(
                "item.use-confusing-strike-prepared",
                "item-use-confusing-strike-prepared",
                [("source", source_kind_id), ("nameKey", display_name_key)],
            ),
            Self::ItemSpellLearningCapacityChanged {
                source_kind_id,
                display_name_key,
                before,
                after,
            } => dto(
                if after > before {
                    "item.use-spell-learning-capacity-increased"
                } else {
                    "item.use-spell-learning-capacity-no-effect"
                },
                if after > before {
                    "item-use-spell-learning-capacity-increased"
                } else {
                    "item-use-spell-learning-capacity-no-effect"
                },
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("before", before.to_string()),
                    ("after", after.to_string()),
                ],
            ),
            Self::ItemElementalBlast {
                source_kind_id,
                display_name_key,
                target_count,
            } => dto(
                "item.use-elemental-blast",
                "item-use-elemental-blast",
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("count", target_count.to_string()),
                ],
            ),
            Self::ItemElementalBlastHit {
                source_kind_id,
                target_kind_id,
                damage,
            } => dto_with_outcome(
                "item.use-elemental-blast-hit",
                "item-use-elemental-blast-hit",
                [
                    ("source", source_kind_id),
                    ("target", target_kind_id),
                    ("damage", damage.applied.to_string()),
                ],
                GameEventOutcomeDto::Damage {
                    resolution: damage.into(),
                },
            ),
            Self::ItemElementalBlastSlew {
                source_kind_id,
                target_kind_id,
                damage,
            } => dto_with_outcome(
                "item.use-elemental-blast-slay",
                "item-use-elemental-blast-slay",
                [("source", source_kind_id), ("target", target_kind_id)],
                GameEventOutcomeDto::Death {
                    resolution: damage.into(),
                },
            ),
            Self::ItemElementalBlastBacklash {
                source_kind_id,
                damage,
                fatal,
            } => dto_with_outcome(
                if fatal {
                    "item.use-elemental-backlash-death"
                } else {
                    "item.use-elemental-backlash"
                },
                if fatal {
                    "item-use-elemental-backlash-death"
                } else {
                    "item-use-elemental-backlash"
                },
                [
                    ("source", source_kind_id),
                    ("damage", damage.applied.to_string()),
                ],
                if fatal {
                    GameEventOutcomeDto::Death {
                        resolution: damage.into(),
                    }
                } else {
                    GameEventOutcomeDto::Damage {
                        resolution: damage.into(),
                    }
                },
            ),
            Self::ItemAggravated {
                source_kind_id,
                display_name_key,
            } => dto(
                "item.use-aggravate",
                "item-use-aggravate",
                [("source", source_kind_id), ("nameKey", display_name_key)],
            ),
            Self::ItemMassGenocide {
                source_kind_id,
                display_name_key,
                removed_count,
                resisted_count,
                fatigue_damage,
            } => dto(
                "item.use-mass-genocide",
                "item-use-mass-genocide",
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("removed", removed_count.to_string()),
                    ("resisted", resisted_count.to_string()),
                    ("fatigue", fatigue_damage.to_string()),
                ],
            ),
            Self::ItemGenocide {
                source_kind_id,
                display_name_key,
                glyph,
                removed_count,
                resisted_count,
                fatigue_damage,
            } => dto(
                "item.use-genocide",
                "item-use-genocide",
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("glyph", glyph),
                    ("removed", removed_count.to_string()),
                    ("resisted", resisted_count.to_string()),
                    ("fatigue", fatigue_damage.to_string()),
                ],
            ),
            Self::ItemCreatedAdjacentTerrain {
                source_kind_id,
                display_name_key,
                affected_positions,
            } => dto(
                if affected_positions.is_empty() {
                    "item.use-create-adjacent-terrain-no-effect"
                } else {
                    "item.use-create-adjacent-terrain"
                },
                if affected_positions.is_empty() {
                    "item-use-create-adjacent-terrain-no-effect"
                } else {
                    "item-use-create-adjacent-terrain"
                },
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("count", affected_positions.len().to_string()),
                ],
            ),
            Self::ItemCreatedCurrentTerrain {
                source_kind_id,
                display_name_key,
                affected_position,
            } => dto(
                if affected_position.is_some() {
                    "item.use-create-current-terrain"
                } else {
                    "item.use-create-current-terrain-no-effect"
                },
                if affected_position.is_some() {
                    "item-use-create-current-terrain"
                } else {
                    "item-use-create-current-terrain-no-effect"
                },
                [("source", source_kind_id), ("nameKey", display_name_key)],
            ),
            Self::ItemFloorGlowChanged {
                source_kind_id,
                display_name_key,
                glow,
                affected_positions,
            } => dto(
                if affected_positions.is_empty() {
                    "item.use-floor-glow-no-effect"
                } else if glow {
                    "item.use-floor-light"
                } else {
                    "item.use-floor-darkness"
                },
                if affected_positions.is_empty() {
                    "item-use-floor-glow-no-effect"
                } else if glow {
                    "item-use-floor-light"
                } else {
                    "item-use-floor-darkness"
                },
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("count", affected_positions.len().to_string()),
                ],
            ),
            Self::ItemAreaDestruction {
                source_kind_id,
                display_name_key,
                protected_floor,
                affected_positions,
                removed_entities,
                removed_items,
                removed_gold_piles,
            } => dto(
                if protected_floor {
                    "item.use-area-destruction-protected"
                } else {
                    "item.use-area-destruction"
                },
                if protected_floor {
                    "item-use-area-destruction-protected"
                } else {
                    "item-use-area-destruction"
                },
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("count", affected_positions.len().to_string()),
                    ("entities", removed_entities.to_string()),
                    ("items", removed_items.to_string()),
                    ("gold", removed_gold_piles.to_string()),
                ],
            ),
            Self::ItemDestroyedAdjacentTrapsAndDoors {
                source_kind_id,
                display_name_key,
                affected_positions,
            } => dto(
                if affected_positions.is_empty() {
                    "item.use-destroy-adjacent-traps-doors-no-effect"
                } else {
                    "item.use-destroy-adjacent-traps-doors"
                },
                if affected_positions.is_empty() {
                    "item-use-destroy-adjacent-traps-doors-no-effect"
                } else {
                    "item-use-destroy-adjacent-traps-doors"
                },
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("count", affected_positions.len().to_string()),
                ],
            ),
            Self::ItemResourceRestored {
                source_kind_id,
                display_name_key,
                resolution,
            } => dto_with_outcome(
                if resolution.recovered > 0 {
                    "item.use-resource-restored"
                } else {
                    "item.use-resource-no-effect"
                },
                if resolution.recovered > 0 {
                    "item-use-resource-restored"
                } else {
                    "item-use-resource-no-effect"
                },
                [
                    ("target", source_kind_id),
                    ("nameKey", display_name_key),
                    ("resource", resolution.resource_id.clone()),
                    ("amount", resolution.recovered.to_string()),
                ],
                GameEventOutcomeDto::ResourceRecovery { resolution },
            ),
            Self::ItemIdentified {
                source_kind_id,
                display_name_key,
                resolution,
            } => dto_with_outcome(
                if resolution.full {
                    "item.use-fully-identified"
                } else {
                    "item.use-identified"
                },
                if resolution.full {
                    "item-use-fully-identified"
                } else {
                    "item-use-identified"
                },
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("target", resolution.item_kind_id.clone()),
                    ("changed", resolution.changed.to_string()),
                ],
                GameEventOutcomeDto::ItemIdentify { resolution },
            ),
            Self::ItemInventoryIdentified {
                source_kind_id,
                display_name_key,
                count,
            } => dto(
                "item.use-inventory-identified",
                "item-use-inventory-identified",
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("count", count.to_string()),
                ],
            ),
            Self::ItemAutoIdentified { count } => dto(
                "item.auto-identified",
                "item-auto-identified",
                [("count", count.to_string())],
            ),
            Self::ItemSelfKnowledge {
                source_kind_id,
                display_name_key,
                report,
            } => dto(
                "item.use-self-knowledge",
                "item-use-self-knowledge",
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("level", report.level.to_string()),
                    ("hp", report.hp.to_string()),
                    ("maxHp", report.max_hp.to_string()),
                    ("gold", report.gold.to_string()),
                    ("nutrition", report.nutrition.to_string()),
                    ("attack", report.attack.to_string()),
                    ("defense", report.defense.to_string()),
                    ("meleeSkill", report.melee_skill.to_string()),
                    ("armorClass", report.armor_class.to_string()),
                    ("speed", report.speed.to_string()),
                    ("strength", report.attributes[0].clone()),
                    ("intelligence", report.attributes[1].clone()),
                    ("wisdom", report.attributes[2].clone()),
                    ("dexterity", report.attributes[3].clone()),
                    ("constitution", report.attributes[4].clone()),
                    ("charisma", report.attributes[5].clone()),
                    ("statuses", report.statuses),
                    ("resistances", report.resistances),
                    ("resources", report.resources),
                ],
            ),
            Self::ItemAcquirement {
                source_kind_id,
                display_name_key,
                generated_item_ids,
                generated_kind_ids,
                position,
            } => dto(
                "item.use-acquirement",
                "item-use-acquirement",
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("count", generated_item_ids.len().to_string()),
                    ("itemIds", generated_item_ids.join(",")),
                    ("items", generated_kind_ids.join(",")),
                    ("x", position.x.to_string()),
                    ("y", position.y.to_string()),
                ],
            ),
            Self::ItemMundanified {
                source_kind_id,
                display_name_key,
                target_item_id,
                target_kind_id,
                split,
            } => dto(
                "item.use-mundanity",
                "item-use-mundanity",
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("targetId", target_item_id),
                    ("target", target_kind_id),
                    ("split", split.to_string()),
                ],
            ),
            Self::ItemCrafted {
                source_kind_id,
                display_name_key,
                target_item_id,
                target_kind_id,
                affix_id,
                split,
            } => dto(
                "item.use-crafting",
                "item-use-crafting",
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("targetId", target_item_id),
                    ("target", target_kind_id),
                    ("affix", affix_id),
                    ("split", split.to_string()),
                ],
            ),
            Self::ItemRumour {
                source_kind_id,
                display_name_key,
                message_key,
            } => dto(
                "item.use-rumour",
                "item-use-rumour",
                [
                    ("source", source_kind_id),
                    ("nameKey", display_name_key),
                    ("rumourKey", message_key),
                ],
            ),
            Self::ItemEnchanted {
                source_kind_id,
                resolution,
            } => dto_with_outcome(
                if resolution.to_hit.successes > 0
                    || resolution.to_damage.successes > 0
                    || resolution.to_armor.successes > 0
                {
                    "item.use-enchanted"
                } else {
                    "item.use-enchantment-failed"
                },
                if resolution.to_hit.successes > 0
                    || resolution.to_damage.successes > 0
                    || resolution.to_armor.successes > 0
                {
                    "item-use-enchanted"
                } else {
                    "item-use-enchantment-failed"
                },
                [
                    ("source", source_kind_id),
                    ("target", resolution.item_kind_id.clone()),
                    ("toHit", resolution.to_hit.after.to_string()),
                    ("toDamage", resolution.to_damage.after.to_string()),
                    ("toArmor", resolution.to_armor.after.to_string()),
                ],
                GameEventOutcomeDto::ItemEnchantment { resolution },
            ),
            Self::ItemCursed {
                source_kind_id,
                resolution,
            } => {
                let (kind, message_key) = if resolution.item_id.is_none() {
                    ("item.use-curse-no-target", "item-use-curse-no-target")
                } else if resolution.resisted {
                    ("item.use-curse-resisted", "item-use-curse-resisted")
                } else {
                    ("item.use-cursed", "item-use-cursed")
                };
                dto_with_outcome(
                    kind,
                    message_key,
                    [
                        ("source", source_kind_id),
                        (
                            "target",
                            resolution.item_kind_id.clone().unwrap_or_default(),
                        ),
                    ],
                    GameEventOutcomeDto::ItemCurse { resolution },
                )
            }
            Self::ItemCursesRemoved {
                source_kind_id,
                resolution,
            } => {
                let removed = resolution.removed_item_ids.len();
                dto_with_outcome(
                    if removed == 0 {
                        "item.use-curse-removal-no-effect"
                    } else {
                        "item.use-curses-removed"
                    },
                    if removed == 0 {
                        "item-use-curse-removal-no-effect"
                    } else {
                        "item-use-curses-removed"
                    },
                    [("source", source_kind_id), ("count", removed.to_string())],
                    GameEventOutcomeDto::ItemCurseRemoval { resolution },
                )
            }
            Self::ItemActivationLanded {
                source_kind_id,
                profile_id,
                trace,
            } => with_trace(
                dto(
                    "item.activation-landed",
                    "item-activation-landed",
                    [("source", source_kind_id), ("profile", profile_id)],
                ),
                trace,
            ),
            Self::ItemActivationHit {
                source_kind_id,
                profile_id,
                target_kind_id,
                damage,
                trace,
            } => with_trace(
                dto_with_outcome(
                    "item.activation-hit",
                    "item-activation-hit",
                    [
                        ("source", source_kind_id),
                        ("profile", profile_id),
                        ("target", target_kind_id),
                    ],
                    GameEventOutcomeDto::Damage {
                        resolution: damage.into(),
                    },
                ),
                trace,
            ),
            Self::ItemActivationSlew {
                source_kind_id,
                profile_id,
                target_kind_id,
                damage,
                trace,
            } => with_trace(
                dto_with_outcome(
                    "item.activation-slay",
                    "item-activation-slay",
                    [
                        ("source", source_kind_id),
                        ("profile", profile_id),
                        ("target", target_kind_id),
                    ],
                    GameEventOutcomeDto::Death {
                        resolution: damage.into(),
                    },
                ),
                trace,
            ),
            Self::ItemDispelHit {
                source_kind_id,
                target_kind_id,
                damage,
            } => dto_with_outcome(
                "item.use-dispel-hit",
                "item-use-dispel-hit",
                [
                    ("source", source_kind_id),
                    ("target", target_kind_id),
                    ("damage", damage.applied.to_string()),
                ],
                GameEventOutcomeDto::Damage {
                    resolution: damage.into(),
                },
            ),
            Self::ItemDispelSlew {
                source_kind_id,
                target_kind_id,
                damage,
            } => dto_with_outcome(
                "item.use-dispel-slay",
                "item-use-dispel-slay",
                [("source", source_kind_id), ("target", target_kind_id)],
                GameEventOutcomeDto::Death {
                    resolution: damage.into(),
                },
            ),
            Self::ItemDispelNoEffect { source_kind_id } => dto(
                "item.use-dispel-no-effect",
                "item-use-dispel-no-effect",
                [("source", source_kind_id)],
            ),
            Self::ItemBanishedActor {
                source_kind_id,
                target_kind_id,
                resolution,
            } => dto_with_outcome(
                "item.use-banished",
                "item-use-banished",
                [("source", source_kind_id), ("target", target_kind_id)],
                GameEventOutcomeDto::MonsterDisplacement { resolution },
            ),
            Self::ItemBanishmentResisted {
                source_kind_id,
                target_kind_id,
            } => dto(
                "item.use-banishment-resisted",
                "item-use-banishment-resisted",
                [("source", source_kind_id), ("target", target_kind_id)],
            ),
            Self::ItemBanishmentNoSpace {
                source_kind_id,
                target_kind_id,
            } => dto(
                "item.use-banishment-no-space",
                "item-use-banishment-no-space",
                [("source", source_kind_id), ("target", target_kind_id)],
            ),
            Self::ItemBanishmentNoEffect { source_kind_id } => dto(
                "item.use-banishment-no-effect",
                "item-use-banishment-no-effect",
                [("source", source_kind_id)],
            ),
            Self::ItemActivationDetected {
                source_kind_id,
                profile_id,
                resolution,
            } => dto_with_outcome(
                "item.activation-detected",
                "item-activation-detected",
                [
                    ("source", source_kind_id),
                    ("profile", profile_id),
                    ("count", resolution.detected_positions.len().to_string()),
                ],
                GameEventOutcomeDto::AbilityDetect { resolution },
            ),
            Self::ItemDetected {
                source_kind_id,
                resolution,
            } => dto_with_outcome(
                "item.use-detected",
                "item-use-detected",
                [
                    ("source", source_kind_id),
                    ("count", resolution.detected_positions.len().to_string()),
                ],
                GameEventOutcomeDto::AbilityDetect { resolution },
            ),
            Self::ItemSummoned {
                source_kind_id,
                profile_id,
                resolution,
            } => {
                let summoned = resolution.entity_ids.len();
                let activation = profile_id.is_some();
                dto_with_outcome(
                    match (activation, summoned == 0) {
                        (false, false) => "item.use-summoned",
                        (false, true) => "item.use-summon-no-effect",
                        (true, false) => "item.activation-summoned",
                        (true, true) => "item.activation-summon-no-effect",
                    },
                    match (activation, summoned == 0) {
                        (false, false) => "item-use-summoned",
                        (false, true) => "item-use-summon-no-effect",
                        (true, false) => "item-activation-summoned",
                        (true, true) => "item-activation-summon-no-effect",
                    },
                    [
                        ("source", source_kind_id),
                        ("actor", resolution.actor_kind_id.clone()),
                        ("count", summoned.to_string()),
                    ],
                    GameEventOutcomeDto::ItemSummon { resolution },
                )
            }
            Self::ItemTeleported {
                source_kind_id,
                profile_id,
                resolution,
            } => dto_with_outcome(
                if profile_id.is_some() {
                    "item.activation-teleported"
                } else {
                    "item.use-teleported"
                },
                if profile_id.is_some() {
                    "item-activation-teleported"
                } else {
                    "item-use-teleported"
                },
                [
                    ("source", source_kind_id),
                    ("fromX", resolution.from.x.to_string()),
                    ("fromY", resolution.from.y.to_string()),
                    ("toX", resolution.to.x.to_string()),
                    ("toY", resolution.to.y.to_string()),
                ],
                GameEventOutcomeDto::AbilityTeleport { resolution },
            ),
            Self::ItemTeleportedLevel {
                source_kind_id,
                from_floor_id,
                to_floor_id,
            } => dto(
                "item.use-teleported-level",
                "item-use-teleported-level",
                [
                    ("source", source_kind_id),
                    ("from", from_floor_id),
                    ("to", to_floor_id),
                ],
            ),
            Self::ItemRecallStarted {
                source_kind_id,
                dungeon_id,
                floor_id,
                turns,
            } => dto(
                "item.recall-started",
                "item-recall-started",
                [
                    ("source", source_kind_id),
                    ("dungeon", dungeon_id),
                    ("floor", floor_id),
                    ("turns", turns.to_string()),
                ],
            ),
            Self::ItemRecallCancelled { source_kind_id } => dto(
                "item.recall-cancelled",
                "item-recall-cancelled",
                [("source", source_kind_id)],
            ),
            Self::ItemRecallReset {
                source_kind_id,
                dungeon_id,
                floor_id,
            } => dto(
                "item.recall-reset",
                "item-recall-reset",
                [
                    ("source", source_kind_id),
                    ("dungeon", dungeon_id),
                    ("floor", floor_id),
                ],
            ),
            Self::RecallTriggered {
                from_floor_id,
                to_floor_id,
            } => dto(
                "item.recall-triggered",
                "item-recall-triggered",
                [("from", from_floor_id), ("to", to_floor_id)],
            ),
            Self::ItemUseUnavailable => {
                dto_without_args("item.use-unavailable", "item-use-unavailable")
            }
            Self::WeaponProficiencyImproved { item_kind_id } => dto(
                "progress.weapon-proficiency-improved",
                "weapon-proficiency-improved",
                [("target", item_kind_id)],
            ),
            Self::RidingProficiencyImproved { current } => dto_without_args(
                "progress.riding-proficiency-improved",
                match current {
                    ..=500 => "riding-proficiency-improved-novice",
                    501..=1_000 => "riding-proficiency-improved-comfortable",
                    1_001..=2_000 => "riding-proficiency-improved-technique",
                    2_001..=5_000 => "riding-proficiency-improved-good",
                    _ => "riding-proficiency-improved-master",
                },
            ),
            Self::MiningProficiencyImproved => dto_without_args(
                "progress.mining-proficiency-improved",
                "mining-proficiency-improved",
            ),
            Self::TerrainFoundSomething => {
                dto_without_args("terrain.found-something", "terrain-found-something")
            }
            Self::PlayerMeleeMissed { target_kind_id } => dto(
                "combat.miss",
                "combat-player-miss",
                [("target", target_kind_id)],
            ),
            Self::MutationMeleeMissed {
                mutation_id,
                attack_name,
                target_kind_id,
            } => dto(
                "mutation.melee-miss",
                "mutation-melee-miss",
                [
                    ("source", mutation_id),
                    ("attack", attack_name),
                    ("target", target_kind_id),
                ],
            ),
            Self::ConfusingStrikeImmune { target_kind_id } => dto(
                "combat.confusing-strike-immune",
                "combat-confusing-strike-immune",
                [("target", target_kind_id)],
            ),
            Self::ConfusingStrikeResisted { target_kind_id } => dto(
                "combat.confusing-strike-resisted",
                "combat-confusing-strike-resisted",
                [("target", target_kind_id)],
            ),
            Self::ConfusingStrikeApplied {
                target_kind_id,
                duration,
            } => dto(
                "combat.confusing-strike-applied",
                "combat-confusing-strike-applied",
                [
                    ("target", target_kind_id),
                    ("duration", duration.to_string()),
                ],
            ),
            Self::PlayerFearBlocked { status_kind_id } => dto(
                "status.fear-blocked",
                "status-fear-blocked",
                [("status", status_kind_id)],
            ),
            Self::PlayerConfusedMove { intended, actual } => dto(
                "status.confused-move",
                "status-confused-move",
                [
                    ("intended", direction_name(intended).to_owned()),
                    ("actual", direction_name(actual).to_owned()),
                ],
            ),
            Self::PlayerParalyzed { status_kind_id } => dto(
                "status.paralyzed",
                "status-paralyzed",
                [("status", status_kind_id)],
            ),
            Self::MonsterSlept { target_kind_id } => dto(
                "status.monster-slept",
                "status-monster-slept",
                [("target", target_kind_id)],
            ),
            Self::EntityAwakened { target_kind_id } => dto(
                "status.entity-awakened",
                "status-entity-awakened",
                [("target", target_kind_id)],
            ),
            Self::PlayerMeleeHit {
                target_kind_id,
                damage,
            } => dto_with_outcome(
                "combat.hit",
                "combat-player-hit",
                [
                    ("target", target_kind_id),
                    ("damage", damage.applied.to_string()),
                ],
                GameEventOutcomeDto::Damage {
                    resolution: damage.into(),
                },
            ),
            Self::MutationMeleeHit {
                mutation_id,
                attack_name,
                target_kind_id,
                damage,
            } => dto_with_outcome(
                "mutation.melee-hit",
                "mutation-melee-hit",
                [
                    ("source", mutation_id),
                    ("attack", attack_name),
                    ("target", target_kind_id),
                    ("damage", damage.applied.to_string()),
                ],
                GameEventOutcomeDto::Damage {
                    resolution: damage.into(),
                },
            ),
            Self::MonsterContactAuraApplied {
                source_kind_id,
                status_kind_id,
                duration,
            } => dto(
                "combat.monster-contact-aura",
                "combat-monster-contact-aura",
                [
                    ("source", source_kind_id),
                    ("status", status_kind_id),
                    ("duration", duration.to_string()),
                ],
            ),
            Self::MonsterFearAuraApplied {
                source_kind_id,
                trigger,
                duration,
            } => dto(
                "combat.monster-fear-aura",
                "combat-monster-fear-aura",
                [
                    ("source", source_kind_id),
                    ("trigger", trigger.to_string()),
                    ("duration", duration.to_string()),
                ],
            ),
            Self::PlayerSlew {
                target_kind_id,
                damage,
            } => dto_with_outcome(
                "combat.slay",
                "combat-player-slay",
                [("target", target_kind_id)],
                GameEventOutcomeDto::Death {
                    resolution: damage.into(),
                },
            ),
            Self::MutationMeleeSlew {
                mutation_id,
                attack_name,
                target_kind_id,
                damage,
            } => dto_with_outcome(
                "mutation.melee-slay",
                "mutation-melee-slay",
                [
                    ("source", mutation_id),
                    ("attack", attack_name),
                    ("target", target_kind_id),
                ],
                GameEventOutcomeDto::Death {
                    resolution: damage.into(),
                },
            ),
            Self::SummonMeleeMissed {
                source_kind_id,
                target_kind_id,
                method_id,
            } => with_method(
                dto(
                    "combat.summon-miss",
                    "combat-summon-miss",
                    [("source", source_kind_id), ("target", target_kind_id)],
                ),
                method_id,
            ),
            Self::SummonMeleeHit {
                source_kind_id,
                target_kind_id,
                method_id,
                damage,
            } => with_method(
                dto_with_outcome(
                    "combat.summon-hit",
                    "combat-summon-hit",
                    [
                        ("source", source_kind_id),
                        ("target", target_kind_id),
                        ("damage", damage.applied.to_string()),
                    ],
                    GameEventOutcomeDto::Damage {
                        resolution: damage.into(),
                    },
                ),
                method_id,
            ),
            Self::SummonSlew {
                source_kind_id,
                target_kind_id,
                method_id,
                damage,
            } => with_method(
                dto_with_outcome(
                    "combat.summon-slay",
                    "combat-summon-slay",
                    [("source", source_kind_id), ("target", target_kind_id)],
                    GameEventOutcomeDto::Death {
                        resolution: damage.into(),
                    },
                ),
                method_id,
            ),
            Self::MonsterMeleeMissed {
                source_kind_id,
                method_id,
            } => with_method(
                dto(
                    "combat.monster-miss",
                    "combat-monster-miss",
                    [("source", source_kind_id)],
                ),
                method_id,
            ),
            Self::MonsterMeleeRepelled {
                source_kind_id,
                method_id,
            } => with_method(
                dto(
                    "combat.monster-repelled",
                    "combat-monster-repelled",
                    [("source", source_kind_id)],
                ),
                method_id,
            ),
            Self::MonsterMeleeHit {
                source_kind_id,
                method_id,
                damage,
            } => with_method(
                dto_with_outcome(
                    "combat.monster-hit",
                    "combat-monster-hit",
                    [
                        ("source", source_kind_id),
                        ("damage", damage.applied.to_string()),
                    ],
                    GameEventOutcomeDto::Damage {
                        resolution: damage.into(),
                    },
                ),
                method_id,
            ),
            Self::MonsterBegged { source_kind_id } => dto(
                "combat.monster-beg",
                "combat-monster-beg",
                [("source", source_kind_id)],
            ),
            Self::MonsterSelfDestructed { source_kind_id } => dto(
                "combat.monster-self-destructed",
                "combat-monster-self-destructed",
                [("source", source_kind_id)],
            ),
            Self::MonsterDeathExplosionHit {
                source_kind_id,
                target_kind_id,
                damage,
            } => dto_with_outcome(
                "combat.monster-death-explosion-hit",
                "combat-monster-death-explosion-hit",
                [
                    ("source", source_kind_id),
                    ("target", target_kind_id),
                    ("damage", damage.applied.to_string()),
                ],
                GameEventOutcomeDto::Damage {
                    resolution: damage.into(),
                },
            ),
            Self::MonsterDeathExplosionSlew {
                source_kind_id,
                target_kind_id,
                damage,
            } => dto_with_outcome(
                "combat.monster-death-explosion-slew",
                "combat-monster-death-explosion-slew",
                [
                    ("source", source_kind_id),
                    ("target", target_kind_id),
                    ("damage", damage.applied.to_string()),
                ],
                GameEventOutcomeDto::Death {
                    resolution: damage.into(),
                },
            ),
            Self::MonsterTerrainDestroyed {
                source_kind_id,
                terrain_kind_id,
                replacement_terrain_kind_id,
                position,
            } => dto(
                "monster.terrain-destroyed",
                "monster-terrain-destroyed",
                [
                    ("source", source_kind_id),
                    ("terrain", terrain_kind_id),
                    ("replacement", replacement_terrain_kind_id),
                    ("x", position.x.to_string()),
                    ("y", position.y.to_string()),
                ],
            ),
            Self::WardingGlyphHeld { source_kind_id } => dto(
                "monster.warding-glyph-held",
                "monster-warding-glyph-held",
                [("source", source_kind_id)],
            ),
            Self::WardingGlyphBroken {
                source_kind_id,
                position,
            } => dto(
                "monster.warding-glyph-broken",
                "monster-warding-glyph-broken",
                [
                    ("source", source_kind_id),
                    ("x", position.x.to_string()),
                    ("y", position.y.to_string()),
                ],
            ),
            Self::MonsterItemDestroyed {
                source_kind_id,
                target_kind_id,
                quantity,
                position,
            } => dto(
                "monster.item-destroyed",
                "monster-item-destroyed",
                [
                    ("source", source_kind_id),
                    ("target", target_kind_id),
                    ("quantity", quantity.to_string()),
                    ("x", position.x.to_string()),
                    ("y", position.y.to_string()),
                ],
            ),
            Self::MonsterItemPickedUp {
                source_kind_id,
                target_kind_id,
                quantity,
                position,
            } => dto(
                "monster.item-picked-up",
                "monster-item-picked-up",
                [
                    ("source", source_kind_id),
                    ("target", target_kind_id),
                    ("quantity", quantity.to_string()),
                    ("x", position.x.to_string()),
                    ("y", position.y.to_string()),
                ],
            ),
            Self::MonsterGoldTheftPrevented { source_kind_id } => dto(
                "monster.gold-theft-prevented",
                "monster-gold-theft-prevented",
                [("source", source_kind_id)],
            ),
            Self::MonsterItemTheftPrevented { source_kind_id } => dto(
                "monster.item-theft-prevented",
                "monster-item-theft-prevented",
                [("source", source_kind_id)],
            ),
            Self::MonsterGoldStolen {
                source_kind_id,
                amount,
            } => dto(
                "monster.gold-stolen",
                "monster-gold-stolen",
                [("source", source_kind_id), ("amount", amount.to_string())],
            ),
            Self::MonsterItemStolen {
                source_kind_id,
                target_kind_id,
                item_id,
            } => dto(
                "monster.item-stolen",
                "monster-item-stolen",
                [
                    ("source", source_kind_id),
                    ("target", target_kind_id),
                    ("itemId", item_id),
                ],
            ),
            Self::MonsterFoodEaten {
                source_kind_id,
                target_kind_id,
            } => dto(
                "monster.food-eaten",
                "monster-food-eaten",
                [("source", source_kind_id), ("target", target_kind_id)],
            ),
            Self::MonsterLightEaten {
                source_kind_id,
                target_kind_id,
                amount,
            } => dto(
                "monster.light-eaten",
                "monster-light-eaten",
                [
                    ("source", source_kind_id),
                    ("target", target_kind_id),
                    ("amount", amount.to_string()),
                ],
            ),
            Self::MonsterChargesDrained {
                source_kind_id,
                target_kind_id,
                amount,
            } => dto(
                "monster.charges-drained",
                "monster-charges-drained",
                [
                    ("source", source_kind_id),
                    ("target", target_kind_id),
                    ("amount", amount.to_string()),
                ],
            ),
            Self::MonsterNutritionDrained {
                source_kind_id,
                amount,
            } => dto(
                "monster.nutrition-drained",
                "monster-nutrition-drained",
                [("source", source_kind_id), ("amount", amount.to_string())],
            ),
            Self::MutationAuraHit {
                target_kind_id,
                damage,
            } => dto_with_outcome(
                "mutation.aura-hit",
                "mutation-aura-hit",
                [
                    ("target", target_kind_id),
                    ("damage", damage.applied.to_string()),
                ],
                GameEventOutcomeDto::Damage {
                    resolution: damage.into(),
                },
            ),
            Self::MutationAuraSlew {
                target_kind_id,
                damage,
            } => dto_with_outcome(
                "mutation.aura-slay",
                "mutation-aura-slay",
                [("target", target_kind_id)],
                GameEventOutcomeDto::Death {
                    resolution: damage.into(),
                },
            ),
            Self::VengeanceHit {
                target_kind_id,
                damage,
            } => dto_with_outcome(
                "combat.vengeance-hit",
                "combat-vengeance-hit",
                [
                    ("target", target_kind_id),
                    ("damage", damage.applied.to_string()),
                ],
                GameEventOutcomeDto::Damage {
                    resolution: damage.into(),
                },
            ),
            Self::VengeanceSlew {
                target_kind_id,
                damage,
            } => dto_with_outcome(
                "combat.vengeance-slay",
                "combat-vengeance-slay",
                [("target", target_kind_id)],
                GameEventOutcomeDto::Death {
                    resolution: damage.into(),
                },
            ),
            Self::MonsterMeleeEntityMissed {
                source_kind_id,
                target_kind_id,
                method_id,
            } => with_method(
                dto(
                    "combat.monster-entity-miss",
                    "combat-monster-entity-miss",
                    [("source", source_kind_id), ("target", target_kind_id)],
                ),
                method_id,
            ),
            Self::MonsterMeleeEntityHit {
                source_kind_id,
                target_kind_id,
                method_id,
                damage,
            } => with_method(
                dto_with_outcome(
                    "combat.monster-entity-hit",
                    "combat-monster-entity-hit",
                    [
                        ("source", source_kind_id),
                        ("target", target_kind_id),
                        ("damage", damage.applied.to_string()),
                    ],
                    GameEventOutcomeDto::Damage {
                        resolution: damage.into(),
                    },
                ),
                method_id,
            ),
            Self::MonsterMeleeEntitySlew {
                source_kind_id,
                target_kind_id,
                method_id,
                damage,
            } => with_method(
                dto_with_outcome(
                    "combat.monster-entity-slew",
                    "combat-monster-entity-slew",
                    [
                        ("source", source_kind_id),
                        ("target", target_kind_id),
                        ("damage", damage.applied.to_string()),
                    ],
                    GameEventOutcomeDto::Death {
                        resolution: damage.into(),
                    },
                ),
                method_id,
            ),
            Self::MonsterBeggedEntity {
                source_kind_id,
                target_kind_id,
            } => dto(
                "combat.monster-entity-beg",
                "combat-monster-entity-beg",
                [("source", source_kind_id), ("target", target_kind_id)],
            ),
            Self::MonsterFled {
                source_kind_id,
                target_kind_id,
            } => dto(
                "combat.monster-fled",
                "combat-monster-fled",
                [("source", source_kind_id), ("target", target_kind_id)],
            ),
            Self::MonsterKeptDistance {
                source_kind_id,
                target_kind_id,
            } => dto(
                "combat.monster-kept-distance",
                "combat-monster-kept-distance",
                [("source", source_kind_id), ("target", target_kind_id)],
            ),
            Self::PlayerDied {
                source_kind_id,
                method_id,
                damage,
            } => with_method(
                dto_with_outcome(
                    "combat.player-death",
                    "combat-player-death",
                    [("source", source_kind_id)],
                    GameEventOutcomeDto::Death {
                        resolution: damage.into(),
                    },
                ),
                method_id,
            ),
            Self::NutritionStateChanged {
                from,
                to,
                nutrition,
            } => dto(
                "hunger.state-changed",
                "hunger-state-changed",
                [
                    ("from", nutrition_state_id(from).to_owned()),
                    ("to", nutrition_state_id(to).to_owned()),
                    ("nutrition", nutrition.to_string()),
                ],
            ),
            Self::PlayerFaintedFromHunger { duration } => dto(
                "hunger.fainted",
                "hunger-fainted",
                [("duration", duration.to_string())],
            ),
            Self::PlayerDamagedByStarvation { damage } => dto_with_outcome(
                "hunger.starvation-damage",
                "hunger-starvation-damage",
                [("damage", damage.applied.to_string())],
                GameEventOutcomeDto::Damage {
                    resolution: damage.into(),
                },
            ),
            Self::PlayerDiedFromStarvation { damage } => dto_with_outcome(
                "hunger.starvation-death",
                "hunger-starvation-death",
                [],
                GameEventOutcomeDto::Death {
                    resolution: damage.into(),
                },
            ),
            Self::PlayerStatusDamaged {
                status_kind_id,
                damage,
            } => dto_with_outcome(
                "status.player-damage",
                "status-player-damage",
                [
                    ("status", status_kind_id),
                    ("damage", damage.applied.to_string()),
                ],
                GameEventOutcomeDto::Damage {
                    resolution: damage.into(),
                },
            ),
            Self::EntityStatusDamaged {
                target_kind_id,
                status_kind_id,
                damage,
            } => dto_with_outcome(
                "status.entity-damage",
                "status-entity-damage",
                [
                    ("target", target_kind_id),
                    ("status", status_kind_id),
                    ("damage", damage.applied.to_string()),
                ],
                GameEventOutcomeDto::Damage {
                    resolution: damage.into(),
                },
            ),
            Self::PlayerStatusExpired { status_kind_id } => dto(
                "status.player-expired",
                "status-player-expired",
                [("status", status_kind_id)],
            ),
            Self::EntityStatusExpired {
                target_kind_id,
                status_kind_id,
            } => dto(
                "status.entity-expired",
                "status-entity-expired",
                [("target", target_kind_id), ("status", status_kind_id)],
            ),
            Self::PlayerDiedFromStatus {
                status_kind_id,
                damage,
            } => dto_with_outcome(
                "status.player-death",
                "status-player-death",
                [("status", status_kind_id)],
                GameEventOutcomeDto::Death {
                    resolution: damage.into(),
                },
            ),
            Self::EntityDiedFromStatus {
                target_kind_id,
                status_kind_id,
                damage,
            } => dto_with_outcome(
                "status.entity-death",
                "status-entity-death",
                [("target", target_kind_id), ("status", status_kind_id)],
                GameEventOutcomeDto::Death {
                    resolution: damage.into(),
                },
            ),
        }
    }
}

fn item_quality_id(quality: ItemQualityDto) -> &'static str {
    match quality {
        ItemQualityDto::Ordinary => "ordinary",
        ItemQualityDto::Fine => "fine",
        ItemQualityDto::Exceptional => "exceptional",
    }
}

const fn nutrition_state_id(state: rfb_protocol::NutritionStateDto) -> &'static str {
    use rfb_protocol::NutritionStateDto;

    match state {
        NutritionStateDto::Bloated => "bloated",
        NutritionStateDto::Full => "full",
        NutritionStateDto::Normal => "normal",
        NutritionStateDto::Hungry => "hungry",
        NutritionStateDto::Weak => "weak",
        NutritionStateDto::Faint => "faint",
        NutritionStateDto::Starving => "starving",
    }
}

fn attribute_kind_id(attribute: crate::stats::AttributeKind) -> &'static str {
    match attribute {
        crate::stats::AttributeKind::Strength => "strength",
        crate::stats::AttributeKind::Intelligence => "intelligence",
        crate::stats::AttributeKind::Wisdom => "wisdom",
        crate::stats::AttributeKind::Dexterity => "dexterity",
        crate::stats::AttributeKind::Constitution => "constitution",
        crate::stats::AttributeKind::Charisma => "charisma",
    }
}

fn rest_stop_reason(reason: &RestStopReasonDto) -> String {
    match reason {
        RestStopReasonDto::Damaged => "damaged",
        RestStopReasonDto::EnemyVisible => "enemy-visible",
        RestStopReasonDto::FullResources => "full-resources",
        RestStopReasonDto::InvalidTurns => "invalid-turns",
        RestStopReasonDto::MutationDirectionRequired => "mutation-direction-required",
        RestStopReasonDto::PetDismissalRequired => "pet-dismissal-required",
        RestStopReasonDto::PlayerDied => "player-died",
        RestStopReasonDto::TurnLimit => "turn-limit",
    }
    .to_owned()
}

fn summon_command_mode_id(mode: SummonCommandModeDto) -> &'static str {
    match mode {
        SummonCommandModeDto::Follow => "follow",
        SummonCommandModeDto::Attack => "attack",
        SummonCommandModeDto::KeepDistance => "keep-distance",
        SummonCommandModeDto::Guard => "guard",
    }
}

pub(crate) fn project_events(events: Vec<DomainEvent>) -> Vec<GameEventDto> {
    events.into_iter().map(DomainEvent::into_dto).collect()
}

fn direction_name(direction: Direction) -> &'static str {
    match direction {
        Direction::North => "north",
        Direction::NorthEast => "north-east",
        Direction::East => "east",
        Direction::SouthEast => "south-east",
        Direction::South => "south",
        Direction::SouthWest => "south-west",
        Direction::West => "west",
        Direction::NorthWest => "north-west",
    }
}

fn dto_without_args(kind: &str, message_key: &str) -> GameEventDto {
    GameEventDto {
        kind: kind.to_owned(),
        message_key: message_key.to_owned(),
        args: BTreeMap::new(),
        outcome: None,
        trace: None,
    }
}

fn dto<const N: usize>(kind: &str, message_key: &str, args: [(&str, String); N]) -> GameEventDto {
    GameEventDto {
        kind: kind.to_owned(),
        message_key: message_key.to_owned(),
        args: args
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
        outcome: None,
        trace: None,
    }
}

fn dto_with_outcome<const N: usize>(
    kind: &str,
    message_key: &str,
    args: [(&str, String); N],
    outcome: GameEventOutcomeDto,
) -> GameEventDto {
    let mut event = dto(kind, message_key, args);
    event.outcome = Some(outcome);
    event
}

fn with_method(mut event: GameEventDto, method_id: Option<String>) -> GameEventDto {
    if let Some(method_id) = method_id {
        event.args.insert("method".to_owned(), method_id);
    }
    event
}

fn with_trace(mut event: GameEventDto, trace: ProjectileTrace) -> GameEventDto {
    event.trace = Some(trace.into());
    event
}

const fn curse_severity_arg(severity: ItemCurseSeverityDto) -> &'static str {
    match severity {
        ItemCurseSeverityDto::Normal => "normal",
        ItemCurseSeverityDto::Heavy => "heavy",
        ItemCurseSeverityDto::Permanent => "permanent",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resistance::{DamageType, ResistanceLevel};

    fn damage(applied: i32) -> DamageOutcome {
        DamageOutcome {
            raw: applied,
            armor_reduction: 0,
            requested: applied,
            applied,
            resistance_delta: 0,
            damage_type: DamageType::Physical,
            resistance: ResistanceLevel::Normal,
        }
    }

    #[test]
    fn typed_events_project_to_the_existing_protocol_contract() {
        let event = DomainEvent::ItemEquipped {
            target_kind_id: "demo.item.charm".to_owned(),
            slot_id: "charm".to_owned(),
            replaced_kind_id: Some("demo.item.old-charm".to_owned()),
        }
        .into_dto();

        assert_eq!(event.kind, "item.equip.swap");
        assert_eq!(event.message_key, "item-equip-swap");
        assert_eq!(event.args["target"], "demo.item.charm");
        assert_eq!(event.args["replaced"], "demo.item.old-charm");
        assert_eq!(event.args["slot"], "charm");
    }

    #[test]
    fn numeric_domain_values_are_formatted_only_at_the_dto_boundary() {
        let event = DomainEvent::MonsterMeleeHit {
            source_kind_id: "demo.actor.monster".to_owned(),
            method_id: None,
            damage: damage(7),
        }
        .into_dto();

        assert_eq!(event.args["source"], "demo.actor.monster");
        assert_eq!(event.args["damage"], "7");
        let Some(GameEventOutcomeDto::Damage { resolution }) = event.outcome else {
            panic!("damage events should preserve their structured resolution");
        };
        assert_eq!(resolution.raw_damage, 7);
        assert_eq!(resolution.final_damage, 7);
    }

    #[test]
    fn digging_failure_projects_its_repeat_decision() {
        let event = DomainEvent::TerrainDigFailed {
            position: Position { x: 4, y: 7 },
            retryable: true,
        }
        .into_dto();

        assert_eq!(event.kind, "terrain.dig-failed");
        assert_eq!(event.args["retryable"], "true");
    }

    #[test]
    fn batch_projection_preserves_authoritative_event_order() {
        let events = project_events(vec![
            DomainEvent::Waited,
            DomainEvent::MoveBlocked,
            DomainEvent::PlayerDied {
                source_kind_id: "demo.actor.monster".to_owned(),
                method_id: None,
                damage: damage(7),
            },
        ]);

        assert_eq!(events[0].kind, "turn.wait");
        assert_eq!(events[1].kind, "move.blocked");
        assert_eq!(events[2].kind, "combat.player-death");
    }
}
