// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeMap;

use rfb_protocol::{
    GameEventDto, GameEventOutcomeDto, HealingResolutionDto, ItemQualityDto, Position,
    ProjectileTraceDto,
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
    PlayerMeleeHit {
        target_kind_id: String,
        damage: DamageOutcome,
    },
    PlayerSlew {
        target_kind_id: String,
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

pub(crate) fn project_events(events: Vec<DomainEvent>) -> Vec<GameEventDto> {
    events.into_iter().map(DomainEvent::into_dto).collect()
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
