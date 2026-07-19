// SPDX-License-Identifier: MPL-2.0

mod action;
pub mod check;
mod combat;
pub mod effect;
mod error;
mod event;
mod game;
pub mod resistance;
mod rng;
mod save;
mod scheduler;
mod state;
pub mod stats;

pub use error::CoreError;
pub use game::{BUILT_IN_WORLD_ID, Game, load_built_in_content};
pub use rng::{RNG_ALGORITHM, RfbRng};
