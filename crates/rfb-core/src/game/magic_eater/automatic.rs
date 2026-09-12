// SPDX-License-Identifier: MPL-2.0
// RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c, magic_eater_auto_*.
use super::*;

#[derive(Clone, Copy)]
pub(in crate::game) enum AutoDeviceEffect {
    Identify,
    Traps,
    Mapping,
}

impl AutoDeviceEffect {
    pub(in crate::game) fn matches(self, profile: &str) -> bool {
        match self {
            Self::Identify => profile == "rfb.device-activation.staff.identify",
            Self::Traps => matches!(
                profile,
                "rfb.device-activation.staff.detect-traps"
                    | "demo.device-activation.trap-sense"
                    | "rfb.device-activation.rod.detect-all"
            ),
            Self::Mapping => matches!(
                profile,
                "demo.device-activation.enlightenment" | "rfb.device-activation.rod.enlightenment"
            ),
        }
    }
}

impl Game {
    pub(in crate::game) fn magic_eater_auto_device(
        &self,
        effect: AutoDeviceEffect,
    ) -> Option<usize> {
        if !self.player_is_magic_eater() {
            return None;
        }
        // Source visits staff then rod at each slot, not the whole staff category first.
        self.items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let ItemLocation::Absorbed { category, slot } = item.location else {
                    return None;
                };
                if category == AbsorbedDeviceCategoryDto::Wand {
                    return None;
                }
                let activation = item.activation.as_ref()?;
                let charges = item.charges?;
                (effect.matches(&activation.profile_id) && charges.current > activation.cost)
                    .then_some(((slot, category), index))
            })
            .min_by_key(|(order, _)| *order)
            .map(|(_, index)| index)
    }
}
