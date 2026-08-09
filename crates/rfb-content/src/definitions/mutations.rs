// SPDX-License-Identifier: MPL-2.0

#[cfg(feature = "schemas")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::StatModifiers;

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
    pub source_index: u16,
    pub random_weight: u8,
    #[serde(default)]
    pub modifiers: StatModifiers,
    /// Direct armor-class adjustment from the original mutation bonus.
    #[serde(default)]
    pub armor_class: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removes_on_gain: Vec<String>,
}
