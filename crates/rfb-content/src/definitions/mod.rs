// SPDX-License-Identifier: MPL-2.0

mod abilities;
mod actors;
mod characters;
mod items;
mod pack;

pub(crate) use abilities::valid_ability_level_scaling;
pub use abilities::*;
pub use actors::*;
pub use characters::*;
pub use items::*;
pub use pack::*;

const fn default_percent() -> u16 {
    100
}
