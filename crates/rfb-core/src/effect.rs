// SPDX-License-Identifier: MPL-2.0

use crate::resistance::{DamageType, ResistanceLevel, ResistanceProfile};
use rfb_protocol::{DamageResolutionDto, StatusDto, StatusSaveDto};

pub const STATUS_HASTE: &str = "rfb.status.haste";
pub const STATUS_SLOW: &str = "rfb.status.slow";
pub const STATUS_POISON: &str = "rfb.status.poison";
pub const STATUS_BLEEDING: &str = "rfb.status.bleeding";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamagePacket {
    pub amount: i32,
    pub armor_reduction: i32,
    pub damage_type: DamageType,
}

impl DamagePacket {
    #[must_use]
    pub const fn new(amount: i32, damage_type: DamageType) -> Self {
        Self {
            amount,
            armor_reduction: 0,
            damage_type,
        }
    }

    #[must_use]
    pub fn after_armor(amount: i32, prepared_amount: i32, damage_type: DamageType) -> Self {
        let raw_damage = amount.max(0);
        let prepared_damage = prepared_amount.clamp(0, raw_damage);
        Self {
            amount: raw_damage,
            armor_reduction: raw_damage.saturating_sub(prepared_damage),
            damage_type,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageOutcome {
    pub raw: i32,
    pub armor_reduction: i32,
    pub requested: i32,
    pub applied: i32,
    /// Positive values were prevented; negative values were added by vulnerability.
    pub resistance_delta: i32,
    pub damage_type: DamageType,
    pub resistance: ResistanceLevel,
}

impl From<DamageOutcome> for DamageResolutionDto {
    fn from(value: DamageOutcome) -> Self {
        Self {
            raw_damage: value.raw,
            armor_reduction: value.armor_reduction,
            resistance_adjustment: value.resistance_delta,
            final_damage: value.applied,
            damage_type: value.damage_type.into(),
            resistance: value.resistance.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusInstance {
    pub kind_id: String,
    pub intensity: u16,
    pub remaining_ticks: u32,
    pub source_id: Option<String>,
}

impl StatusInstance {
    #[must_use]
    pub fn to_dto(&self) -> StatusDto {
        StatusDto {
            kind_id: self.kind_id.clone(),
            intensity: self.intensity,
            remaining_ticks: self.remaining_ticks,
        }
    }

    #[must_use]
    pub fn to_save_dto(&self) -> StatusSaveDto {
        StatusSaveDto {
            kind_id: self.kind_id.clone(),
            intensity: self.intensity,
            remaining_ticks: self.remaining_ticks,
            source_id: self.source_id.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusStacking {
    Replace,
    Extend,
    KeepStrongest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusApplication {
    pub status: StatusInstance,
    pub stacking: StatusStacking,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusChange {
    Added,
    Replaced,
    Extended,
    Strengthened,
    Unchanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectSpec {
    Damage(DamagePacket),
    Heal { amount: i32 },
    ApplyStatus(StatusApplication),
    RemoveStatus { kind_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectOutcome {
    Damage(DamageOutcome),
    Healed {
        requested: i32,
        applied: i32,
    },
    StatusApplied {
        kind_id: String,
        change: StatusChange,
    },
    StatusRemoved {
        kind_id: String,
        removed: bool,
    },
}

pub struct EffectTarget<'a> {
    pub hp: &'a mut i32,
    pub max_hp: i32,
    pub resistances: &'a ResistanceProfile,
    pub statuses: &'a mut Vec<StatusInstance>,
}

#[must_use]
pub fn resolve_damage(packet: DamagePacket, resistance: ResistanceLevel) -> DamageOutcome {
    let raw = packet.amount.max(0);
    let armor_reduction = packet.armor_reduction.clamp(0, raw);
    let requested = raw.saturating_sub(armor_reduction);
    let reduction = i64::from(resistance.reduction_percent());
    let prevented = i64::from(requested).saturating_mul(reduction) / 100;
    let applied = i64::from(requested)
        .saturating_sub(prevented)
        .clamp(0, i64::from(i32::MAX)) as i32;

    DamageOutcome {
        raw,
        armor_reduction,
        requested,
        applied,
        resistance_delta: requested.saturating_sub(applied),
        damage_type: packet.damage_type,
        resistance,
    }
}

pub fn apply_effect(target: &mut EffectTarget<'_>, effect: EffectSpec) -> EffectOutcome {
    match effect {
        EffectSpec::Damage(packet) => {
            let outcome = resolve_damage(packet, target.resistances.level(packet.damage_type));
            *target.hp = target.hp.saturating_sub(outcome.applied);
            EffectOutcome::Damage(outcome)
        }
        EffectSpec::Heal { amount } => {
            let requested = amount.max(0);
            let before = *target.hp;
            *target.hp = target.hp.saturating_add(requested).min(target.max_hp);
            EffectOutcome::Healed {
                requested,
                applied: target.hp.saturating_sub(before),
            }
        }
        EffectSpec::ApplyStatus(application) => {
            let kind_id = application.status.kind_id.clone();
            let change = apply_status(target.statuses, application);
            EffectOutcome::StatusApplied { kind_id, change }
        }
        EffectSpec::RemoveStatus { kind_id } => {
            let before = target.statuses.len();
            target.statuses.retain(|status| status.kind_id != kind_id);
            EffectOutcome::StatusRemoved {
                kind_id,
                removed: target.statuses.len() != before,
            }
        }
    }
}

pub fn apply_status(
    statuses: &mut Vec<StatusInstance>,
    application: StatusApplication,
) -> StatusChange {
    let Some(index) = statuses
        .iter()
        .position(|status| status.kind_id == application.status.kind_id)
    else {
        statuses.push(application.status);
        statuses.sort_by(|left, right| left.kind_id.cmp(&right.kind_id));
        return StatusChange::Added;
    };

    let existing = &mut statuses[index];
    match application.stacking {
        StatusStacking::Replace => {
            if *existing == application.status {
                StatusChange::Unchanged
            } else {
                *existing = application.status;
                StatusChange::Replaced
            }
        }
        StatusStacking::Extend => {
            existing.remaining_ticks = existing
                .remaining_ticks
                .saturating_add(application.status.remaining_ticks);
            if application.status.intensity > existing.intensity {
                existing.intensity = application.status.intensity;
                existing.source_id = application.status.source_id;
            }
            StatusChange::Extended
        }
        StatusStacking::KeepStrongest => {
            let stronger = application.status.intensity > existing.intensity;
            let longer = application.status.remaining_ticks > existing.remaining_ticks;
            if stronger {
                existing.intensity = application.status.intensity;
                existing.source_id = application.status.source_id.clone();
            }
            if longer {
                existing.remaining_ticks = application.status.remaining_ticks;
            }
            if stronger {
                StatusChange::Strengthened
            } else if longer {
                StatusChange::Extended
            } else {
                StatusChange::Unchanged
            }
        }
    }
}

/// Advances status durations in stable kind-id order and returns the expired IDs.
pub fn advance_status_ticks(statuses: &mut Vec<StatusInstance>, elapsed_ticks: u32) -> Vec<String> {
    if elapsed_ticks == 0 {
        return Vec::new();
    }

    statuses.sort_by(|left, right| left.kind_id.cmp(&right.kind_id));
    let mut expired = Vec::new();
    for status in statuses.iter_mut() {
        status.remaining_ticks = status.remaining_ticks.saturating_sub(elapsed_ticks);
        if status.remaining_ticks == 0 {
            expired.push(status.kind_id.clone());
        }
    }
    statuses.retain(|status| status.remaining_ticks > 0);
    expired
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(kind_id: &str, intensity: u16, remaining_ticks: u32) -> StatusInstance {
        StatusInstance {
            kind_id: kind_id.to_owned(),
            intensity,
            remaining_ticks,
            source_id: Some("actor.source".to_owned()),
        }
    }

    #[test]
    fn elemental_resistance_uses_deterministic_integer_reduction() {
        let packet = DamagePacket {
            amount: 10,
            armor_reduction: 0,
            damage_type: DamageType::Fire,
        };

        assert_eq!(
            resolve_damage(packet, ResistanceLevel::Vulnerable).applied,
            15
        );
        assert_eq!(resolve_damage(packet, ResistanceLevel::Normal).applied, 10);
        assert_eq!(
            resolve_damage(packet, ResistanceLevel::Resistant).applied,
            5
        );
        assert_eq!(resolve_damage(packet, ResistanceLevel::Strong).applied, 4);
        assert_eq!(resolve_damage(packet, ResistanceLevel::Immune).applied, 0);
        assert_eq!(
            resolve_damage(
                DamagePacket {
                    amount: 1,
                    armor_reduction: 0,
                    damage_type: DamageType::Fire,
                },
                ResistanceLevel::Resistant,
            )
            .applied,
            1
        );
    }

    #[test]
    fn armor_and_resistance_reductions_remain_separate_in_the_outcome() {
        let outcome = resolve_damage(
            DamagePacket::after_armor(10, 7, DamageType::Physical),
            ResistanceLevel::Resistant,
        );

        assert_eq!(outcome.raw, 10);
        assert_eq!(outcome.armor_reduction, 3);
        assert_eq!(outcome.requested, 7);
        assert_eq!(outcome.resistance_delta, 3);
        assert_eq!(outcome.applied, 4);
    }

    #[test]
    fn status_application_is_sorted_and_obeys_explicit_stacking() {
        let mut statuses = vec![status("status.poison", 2, 10)];
        assert_eq!(
            apply_status(
                &mut statuses,
                StatusApplication {
                    status: status("status.haste", 1, 5),
                    stacking: StatusStacking::Replace,
                },
            ),
            StatusChange::Added
        );
        assert_eq!(statuses[0].kind_id, "status.haste");

        assert_eq!(
            apply_status(
                &mut statuses,
                StatusApplication {
                    status: status("status.poison", 3, 4),
                    stacking: StatusStacking::Extend,
                },
            ),
            StatusChange::Extended
        );
        assert_eq!(statuses[1].intensity, 3);
        assert_eq!(statuses[1].remaining_ticks, 14);
    }

    #[test]
    fn effect_pipeline_mutates_only_the_supplied_authoritative_target() {
        let mut hp = 12;
        let mut statuses = Vec::new();
        let mut resistances = ResistanceProfile::default();
        resistances.set(DamageType::Poison, ResistanceLevel::Resistant);
        let mut target = EffectTarget {
            hp: &mut hp,
            max_hp: 12,
            resistances: &resistances,
            statuses: &mut statuses,
        };

        let outcome = apply_effect(
            &mut target,
            EffectSpec::Damage(DamagePacket {
                amount: 7,
                armor_reduction: 0,
                damage_type: DamageType::Poison,
            }),
        );
        assert_eq!(
            outcome,
            EffectOutcome::Damage(DamageOutcome {
                raw: 7,
                armor_reduction: 0,
                requested: 7,
                applied: 4,
                resistance_delta: 3,
                damage_type: DamageType::Poison,
                resistance: ResistanceLevel::Resistant,
            })
        );
        assert_eq!(*target.hp, 8);

        assert_eq!(
            apply_effect(&mut target, EffectSpec::Heal { amount: 10 }),
            EffectOutcome::Healed {
                requested: 10,
                applied: 4,
            }
        );
        assert_eq!(*target.hp, 12);
    }

    #[test]
    fn status_ticks_expire_in_stable_kind_order() {
        let mut statuses = vec![
            status("status.poison", 1, 3),
            status("status.haste", 1, 1),
            status("status.fear", 1, 1),
        ];

        assert_eq!(
            advance_status_ticks(&mut statuses, 1),
            vec!["status.fear", "status.haste"]
        );
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].kind_id, "status.poison");
        assert_eq!(statuses[0].remaining_ticks, 2);
    }
}
