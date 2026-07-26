// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeMap;

use rfb_protocol::{
    AbilityAreaDamageResolutionDto, AbilityBeamDamageResolutionDto, AbilityCastResolutionDto,
    AbilityConeDamageResolutionDto, AbilityDetectResolutionDto, AbilityEffectsResolutionDto,
    AbilitySummonResolutionDto, AbilityTeleportResolutionDto, AbilityTerrainTransformResolutionDto,
    CheckResolutionDto, Direction, GameEventDto, GameEventOutcomeDto, HealingResolutionDto,
    ItemQualityDto, MonsterAbilityCastResolutionDto, MonsterAbilityDecisionResolutionDto,
    MonsterDisplacementResolutionDto, Position, ProjectileTraceDto, ResourceGainResolutionDto,
    ResourceGainSourceDto, ResourceRecoveryResolutionDto, RestResolutionDto, RestStopReasonDto,
    SummonCommandModeDto, SummonCommandResolutionDto,
};

use crate::effect::DamageOutcome;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectileTrace {
    pub(crate) origin: Position,
    pub(crate) impact: Position,
    pub(crate) landing: Position,
    pub(crate) traversed: Vec<Position>,
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
pub(crate) enum DomainEvent {
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
        ability_id: String,
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
    AbilityTargetUnavailable {
        ability_id: String,
    },
    AbilityAreaDamage {
        ability_id: String,
        resolution: AbilityAreaDamageResolutionDto,
        trace: ProjectileTrace,
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
    AbilityHealed {
        ability_id: String,
        resolution: HealingResolutionDto,
    },
    ResourceRecovered {
        resolution: ResourceRecoveryResolutionDto,
    },
    ResourceGained {
        resolution: ResourceGainResolutionDto,
    },
    MonsterBlinked {
        source_kind_id: String,
        resolution: MonsterDisplacementResolutionDto,
    },
    MonsterTeleported {
        source_kind_id: String,
        resolution: MonsterDisplacementResolutionDto,
    },
    MonsterDraggedTarget {
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
    ItemsDropped {
        stacks: usize,
        quantity: u64,
    },
    NoItemsDropped,
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
    ExperienceGained {
        amount: u64,
        total: u64,
    },
    PlayerLevelGained {
        level: u16,
        max_hp: i32,
        pending_attribute_increases: u16,
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
    TerrainDug {
        position: Position,
    },
    TerrainDigFailed {
        position: Position,
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
    ItemPickupOverCapacity {
        target_kind_id: String,
        quantity: u32,
        current_weight: u32,
        pickup_weight: u32,
        capacity: u32,
    },
    NothingToPickUp,
    ItemUnequipped {
        target_kind_id: String,
        slot_id: String,
    },
    ItemUnequipUnavailable {
        slot_id: String,
    },
    MoveBlocked,
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
    ItemUsed {
        source_kind_id: String,
        display_name_key: String,
        requested: i32,
        applied: i32,
    },
    ItemUseUnavailable,
    PlayerMeleeMissed {
        target_kind_id: String,
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
    PlayerMeleeHit {
        target_kind_id: String,
        damage: DamageOutcome,
    },
    PlayerSlew {
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
    MonsterMeleeHit {
        source_kind_id: String,
        method_id: Option<String>,
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
            Self::AbilityStudyUnavailable { ability_id, reason } => dto(
                "ability.study-unavailable",
                "ability-study-unavailable",
                [("target", ability_id), ("reason", reason)],
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
            Self::MonsterTeleported {
                source_kind_id,
                resolution,
            } => dto_with_outcome(
                "monster.teleported",
                "monster-teleported",
                [("source", source_kind_id)],
                GameEventOutcomeDto::MonsterDisplacement { resolution },
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
            Self::ResourceGained { resolution } => dto_with_outcome(
                "resource.gained",
                "resource-gained",
                [
                    ("target", resolution.resource_id.clone()),
                    ("amount", resolution.gained.to_string()),
                    (
                        "source",
                        match resolution.source {
                            ResourceGainSourceDto::MeleeHit => "melee-hit".to_owned(),
                            ResourceGainSourceDto::MeleeKill => "melee-kill".to_owned(),
                        },
                    ),
                ],
                GameEventOutcomeDto::ResourceGain { resolution },
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
            Self::ItemsDropped { stacks, quantity } => dto(
                "item.drop",
                "item-drop-success",
                [
                    ("stacks", stacks.to_string()),
                    ("quantity", quantity.to_string()),
                ],
            ),
            Self::NoItemsDropped => dto_without_args("item.drop.none", "item-drop-none"),
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
            Self::ExperienceGained { amount, total } => dto(
                "player.experience-gained",
                "player-experience-gained",
                [("amount", amount.to_string()), ("total", total.to_string())],
            ),
            Self::PlayerLevelGained {
                level,
                max_hp,
                pending_attribute_increases,
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
            Self::TerrainDug { position } => dto(
                "terrain.dug",
                "terrain-dug",
                [("x", position.x.to_string()), ("y", position.y.to_string())],
            ),
            Self::TerrainDigFailed { position } => dto(
                "terrain.dig-failed",
                "terrain-dig-failed",
                [("x", position.x.to_string()), ("y", position.y.to_string())],
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
            Self::ItemPickupOverCapacity {
                target_kind_id,
                quantity,
                current_weight,
                pickup_weight,
                capacity,
            } => dto(
                "item.pickup.over-capacity",
                "item-pickup-over-capacity",
                [
                    ("target", target_kind_id),
                    ("quantity", quantity.to_string()),
                    ("currentWeight", current_weight.to_string()),
                    ("pickupWeight", pickup_weight.to_string()),
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
            Self::MoveBlocked => dto_without_args("move.blocked", "game-move-blocked"),
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
            Self::ItemUseUnavailable => {
                dto_without_args("item.use-unavailable", "item-use-unavailable")
            }
            Self::PlayerMeleeMissed { target_kind_id } => dto(
                "combat.miss",
                "combat-player-miss",
                [("target", target_kind_id)],
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
