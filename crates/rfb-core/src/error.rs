// SPDX-License-Identifier: MPL-2.0

use rfb_content::ContentError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("revision mismatch: core is at {expected}, command expected {received}")]
    RevisionMismatch { expected: u32, received: u32 },
    #[error("command sequence mismatch: expected {expected}, received {received}")]
    CommandSequence { expected: u32, received: u32 },
    #[error("the player is dead and cannot act")]
    PlayerDead,
    #[error("the campaign has ended and cannot accept more commands")]
    CampaignEnded,
    #[error("unsupported save schema version {0}")]
    UnsupportedSaveVersion(u16),
    #[error("save uses unsupported RNG algorithm {0}")]
    UnsupportedRng(String),
    #[error("save content set does not match the demo content set")]
    ContentMismatch,
    #[error("content set does not define world {0}")]
    UnknownWorld(String),
    #[error("save contains unknown terrain ID {0}")]
    UnknownTerrain(String),
    #[error("content set does not define actor {0}")]
    UnknownActor(String),
    #[error("content set does not define item {0}")]
    UnknownItem(String),
    #[error("generated item instance ID space is exhausted")]
    ItemIdExhausted,
    #[error("invalid save: {0}")]
    InvalidSave(&'static str),
    #[error(transparent)]
    Content(#[from] ContentError),
}
