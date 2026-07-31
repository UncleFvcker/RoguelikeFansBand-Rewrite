// SPDX-License-Identifier: MPL-2.0

mod healing;
mod resources;

pub(super) use healing::{HealingRequest, apply_healing};
pub(super) use resources::{ResourceRestorationRequest, apply_resource_restoration};
