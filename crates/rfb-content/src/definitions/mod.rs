// SPDX-License-Identifier: MPL-2.0

mod abilities;
mod actors;
mod characters;
mod items;
mod pack;
mod tables;
mod worlds;

pub(crate) use abilities::valid_ability_level_scaling;
pub use abilities::*;
pub use actors::*;
pub use characters::*;
pub use items::*;
pub use pack::*;
pub use tables::*;
pub use worlds::*;

const fn default_percent() -> u16 {
    100
}
