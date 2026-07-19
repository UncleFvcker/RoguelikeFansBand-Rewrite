// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeMap;

use rfb_protocol::{GameEventDto, GameEventOutcomeDto, Position, ProjectileTraceDto};

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
    Waited,
    ItemPickedUp {
        target_kind_id: String,
        quantity: u32,
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
    ItemThrown {
        target_kind_id: String,
        trace: ProjectileTrace,
    },
    ItemThrowUnavailable,
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
            Self::ItemThrown {
                target_kind_id,
                trace,
            } => with_trace(
                dto("item.thrown", "item-thrown", [("target", target_kind_id)]),
                trace,
            ),
            Self::ItemThrowUnavailable => {
                dto_without_args("item.throw-unavailable", "item-throw-unavailable")
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
