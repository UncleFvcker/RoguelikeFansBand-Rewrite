// SPDX-License-Identifier: MPL-2.0

use crate::{
    effect::{
        DamageOutcome, DamagePacket, STATUS_BLEEDING, STATUS_POISON, STATUS_SLEEP,
        advance_status_ticks, resolve_damage,
    },
    resistance::DamageType,
    state::Actor,
};
use rfb_protocol::Position;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FatalityPolicy {
    BelowZero,
    AtOrBelowZero,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DamageApplicationPlan {
    target_id: String,
    pub(super) position: Position,
    pub(super) hp_before: i32,
    pub(super) hp_after: i32,
    pub(super) damage: DamageOutcome,
    pub(super) fatal: bool,
    pub(super) wakes_sleeping_target: bool,
}

pub(super) fn plan_damage_application(
    target: &Actor,
    damage: DamageOutcome,
    fatality_policy: FatalityPolicy,
) -> DamageApplicationPlan {
    let hp_after = target.hp.saturating_sub(damage.applied);
    let fatal = match fatality_policy {
        FatalityPolicy::BelowZero => hp_after < 0,
        FatalityPolicy::AtOrBelowZero => hp_after <= 0,
    };
    DamageApplicationPlan {
        target_id: target.id.clone(),
        position: target.position,
        hp_before: target.hp,
        hp_after,
        damage,
        fatal,
        wakes_sleeping_target: damage.applied > 0 && hp_after > 0,
    }
}

pub(super) fn commit_damage_application(target: &mut Actor, plan: &DamageApplicationPlan) {
    debug_assert_eq!(target.id, plan.target_id);
    debug_assert_eq!(target.position, plan.position);
    debug_assert_eq!(target.hp, plan.hp_before);
    target.hp = plan.hp_after;
}

pub(super) struct ActorStatusTick {
    pub(super) damage: Vec<StatusDamageTick>,
    pub(super) expired: Vec<String>,
    pub(super) fatal_damage: Option<StatusDamageTick>,
    pub(super) awakened: bool,
}

#[derive(Clone)]
pub(super) struct StatusDamageTick {
    pub(super) status_kind_id: String,
    pub(super) outcome: DamageOutcome,
}

pub(super) fn process_actor_status_tick(
    actor: &mut Actor,
    lethal_at_zero: bool,
    incoming_damage_percent: u8,
) -> ActorStatusTick {
    let periodic = actor
        .statuses
        .iter()
        .filter_map(|status| {
            let damage_type = match status.kind_id.as_str() {
                STATUS_BLEEDING => DamageType::Physical,
                STATUS_POISON => DamageType::Poison,
                _ => return None,
            };
            Some((
                status.kind_id.clone(),
                i32::from(status.intensity),
                damage_type,
            ))
        })
        .collect::<Vec<_>>();
    let mut damage = Vec::new();
    let mut fatal_damage = None;
    let mut awakened = false;
    for (status_kind_id, amount, damage_type) in periodic {
        let outcome = scale_damage_outcome(
            resolve_damage(
                DamagePacket::new(amount, damage_type),
                actor.resistances.level(damage_type),
            ),
            incoming_damage_percent,
        );
        let fatality_policy = if lethal_at_zero {
            FatalityPolicy::AtOrBelowZero
        } else {
            FatalityPolicy::BelowZero
        };
        let application = plan_damage_application(actor, outcome, fatality_policy);
        commit_damage_application(actor, &application);
        let damage_tick = StatusDamageTick {
            status_kind_id: status_kind_id.clone(),
            outcome,
        };
        if application.wakes_sleeping_target {
            let before = actor.statuses.len();
            actor
                .statuses
                .retain(|status| status.kind_id != STATUS_SLEEP);
            awakened |= actor.statuses.len() != before;
        }
        damage.push(damage_tick.clone());
        if application.fatal {
            fatal_damage = Some(damage_tick);
            break;
        }
    }
    let expired = advance_status_ticks(&mut actor.statuses, 1);
    ActorStatusTick {
        damage,
        expired,
        fatal_damage,
        awakened,
    }
}

pub(super) fn scale_damage_outcome(mut damage: DamageOutcome, percent: u8) -> DamageOutcome {
    let original_applied = damage.applied;
    damage.applied = i32::try_from(
        i64::from(original_applied)
            .saturating_mul(i64::from(percent))
            .saturating_add(99)
            .saturating_div(100),
    )
    .unwrap_or(i32::MAX);
    damage.resistance_delta = damage
        .resistance_delta
        .saturating_add(original_applied.saturating_sub(damage.applied));
    damage
}

#[cfg(test)]
mod tests {
    use crate::{
        effect::{DamageOutcome, DamagePacket, resolve_damage},
        resistance::{DamageType, ResistanceLevel},
    };

    use super::*;

    fn target_with_hp(hp: i32) -> Actor {
        let mut target = crate::game::Game::new(42).player;
        target.id = "damage-contract-target".to_owned();
        target.hp = hp;
        target
    }

    fn physical_damage(amount: i32) -> DamageOutcome {
        resolve_damage(
            DamagePacket::new(amount, DamageType::Physical),
            ResistanceLevel::Normal,
        )
    }

    #[test]
    fn fatality_policy_preserves_player_and_actor_zero_hp_distinction() {
        let target = target_with_hp(1);
        let damage = physical_damage(1);

        let player = plan_damage_application(&target, damage, FatalityPolicy::BelowZero);
        let actor = plan_damage_application(&target, damage, FatalityPolicy::AtOrBelowZero);

        assert_eq!(player.hp_after, 0);
        assert!(!player.fatal);
        assert!(actor.fatal);
    }

    #[test]
    fn application_saturates_hp_and_only_wakes_surviving_damaged_targets() {
        let mut target = target_with_hp(i32::MIN + 1);
        let plan = plan_damage_application(
            &target,
            physical_damage(i32::MAX),
            FatalityPolicy::AtOrBelowZero,
        );

        assert_eq!(plan.hp_after, i32::MIN);
        assert!(plan.fatal);
        assert!(!plan.wakes_sleeping_target);
        commit_damage_application(&mut target, &plan);
        assert_eq!(target.hp, i32::MIN);

        let target = target_with_hp(5);
        let zero =
            plan_damage_application(&target, physical_damage(0), FatalityPolicy::AtOrBelowZero);
        let surviving =
            plan_damage_application(&target, physical_damage(1), FatalityPolicy::AtOrBelowZero);
        assert!(!zero.wakes_sleeping_target);
        assert!(surviving.wakes_sleeping_target);
    }
}
