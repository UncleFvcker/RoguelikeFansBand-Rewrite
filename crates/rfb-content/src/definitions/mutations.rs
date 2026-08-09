// SPDX-License-Identifier: MPL-2.0

#[cfg(feature = "schemas")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum MutationRatingDefinition {
    Awful,
    Bad,
    Average,
    Good,
    Great,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MutationDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub name: String,
    pub description: String,
    pub rating: MutationRatingDefinition,
}
