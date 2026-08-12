// SPDX-License-Identifier: MPL-2.0

mod abilities;
mod actors;
mod characters;
mod items;
mod mutations;
mod pack;
mod tables;
mod towns;
mod worlds;

pub use abilities::*;
pub(crate) use abilities::{valid_ability_level_scaling, valid_ability_spell_power};
pub use actors::*;
pub use characters::*;
pub use items::*;
pub use mutations::*;
pub use pack::*;
pub use tables::*;
pub use towns::*;
pub use worlds::*;

const fn default_percent() -> u16 {
    100
}
