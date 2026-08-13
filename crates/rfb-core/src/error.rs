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
    #[error("a periodic mutation is waiting for a direction")]
    MutationDirectionRequired,
    #[error("no periodic mutation is waiting for a direction")]
    MutationDirectionUnavailable,
    #[error("a race mutation reward is waiting for a choice")]
    RaceMutationChoiceRequired,
    #[error("the requested race mutation choice is unavailable")]
    RaceMutationChoiceUnavailable,
    #[error("world map transition is unavailable from the current state")]
    WorldMapTransitionUnavailable,
    #[error("the command is unavailable while viewing the world map")]
    WorldMapActionUnavailable,
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
    #[error("content set does not define character build {0}")]
    UnknownCharacterBuild(String),
    #[error("content set does not define race {0}")]
    UnknownCharacterRace(String),
    #[error("race {0} is not available for character creation")]
    CharacterRaceUnavailable(String),
    #[error("player name must contain 1 to 32 printable characters")]
    InvalidPlayerName,
    #[error("generated item instance ID space is exhausted")]
    ItemIdExhausted,
    #[error("generated gold pile ID space is exhausted")]
    GoldPileIdExhausted,
    #[error("internal invariant failed: {0}")]
    Invariant(String),
    #[error("invalid save: {0}")]
    InvalidSave(&'static str),
    #[error(transparent)]
    Content(#[from] ContentError),
}
