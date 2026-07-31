// SPDX-License-Identifier: MPL-2.0

use crate::effect::{
    StatusApplication, StatusChange, StatusInstance, apply_status as apply_effect_status,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::game) struct StatusApplicationOutcome {
    pub(in crate::game) kind_id: String,
    pub(in crate::game) change: StatusChange,
}

pub(in crate::game) fn apply_status_application(
    statuses: &mut Vec<StatusInstance>,
    request: StatusApplication,
) -> StatusApplicationOutcome {
    let kind_id = request.status.kind_id.clone();
    let change = apply_effect_status(statuses, request);
    StatusApplicationOutcome { kind_id, change }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::game) struct StatusRemovalRequest<'a> {
    kind_id: &'a str,
}

impl<'a> StatusRemovalRequest<'a> {
    pub(in crate::game) const fn new(kind_id: &'a str) -> Self {
        Self { kind_id }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::game) struct StatusRemovalOutcome {
    pub(in crate::game) kind_id: String,
    pub(in crate::game) removed: bool,
}

pub(in crate::game) fn apply_status_removal(
    statuses: &mut Vec<StatusInstance>,
    request: StatusRemovalRequest<'_>,
) -> StatusRemovalOutcome {
    let before = statuses.len();
    statuses.retain(|status| status.kind_id != request.kind_id);
    StatusRemovalOutcome {
        kind_id: request.kind_id.to_owned(),
        removed: statuses.len() != before,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use rfb_protocol::{EquipmentBonusesDto, StatModifiersDto};

    use super::*;
    use crate::effect::StatusStacking;

    fn status(kind_id: &str, remaining_ticks: u32) -> StatusInstance {
        StatusInstance {
            kind_id: kind_id.to_owned(),
            intensity: 1,
            remaining_ticks,
            source_id: None,
            granted_resistances: BTreeMap::new(),
            granted_brands: BTreeSet::new(),
            granted_modifiers: StatModifiersDto::default(),
            granted_equipment_bonuses: EquipmentBonusesDto::default(),
            granted_status_immunities: BTreeSet::new(),
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        }
    }

    #[test]
    fn status_application_and_removal_report_source_neutral_outcomes() {
        let mut statuses = vec![status("test.status.slow", 3)];
        let application = apply_status_application(
            &mut statuses,
            StatusApplication {
                status: status("test.status.haste", 5),
                stacking: StatusStacking::Extend,
            },
        );
        assert_eq!(application.kind_id, "test.status.haste");
        assert_eq!(application.change, StatusChange::Added);
        assert_eq!(statuses[0].kind_id, "test.status.haste");

        let removed = apply_status_removal(
            &mut statuses,
            StatusRemovalRequest::new("test.status.haste"),
        );
        assert_eq!(removed.kind_id, "test.status.haste");
        assert!(removed.removed);
        assert!(
            !apply_status_removal(
                &mut statuses,
                StatusRemovalRequest::new("test.status.missing"),
            )
            .removed
        );
    }
}
