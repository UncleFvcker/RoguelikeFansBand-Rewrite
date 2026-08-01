// SPDX-License-Identifier: MPL-2.0

#[cfg(feature = "schemas")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackManifest {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub version: String,
    pub title_key: String,
    pub dependencies: Vec<PackDependency>,
    pub load_after: Vec<String>,
    pub content_roots: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackDependency {
    pub id: String,
    pub version_requirement: String,
}
