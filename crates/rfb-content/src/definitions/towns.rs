// SPDX-License-Identifier: MPL-2.0

#[cfg(feature = "schemas")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::ContentPosition;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TownDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub name_key: String,
    pub description_key: String,
    pub floor_id: String,
    #[serde(default)]
    pub facility_ids: Vec<String>,
    pub shop_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TownFacilityDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub name_key: String,
    pub description_key: String,
    pub town_id: String,
    pub category: TownFacilityCategory,
    #[serde(default)]
    pub storage_id: Option<String>,
    #[serde(default)]
    pub owner_name_key: Option<String>,
    #[serde(default)]
    pub task_ids: Vec<String>,
    pub entrance_position: ContentPosition,
    pub entrance_terrain_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum TownFacilityCategory {
    Home,
    QuestGiver,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShopDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub name_key: String,
    pub description_key: String,
    pub town_id: String,
    pub category: ShopCategory,
    pub entrance_position: ContentPosition,
    pub entrance_terrain_id: String,
    pub owner: ShopOwnerDefinition,
    pub stock: Vec<ShopStockDefinition>,
    pub maintenance: ShopMaintenanceDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShopOwnerDefinition {
    pub id: String,
    pub name_key: String,
    pub race_id: String,
    pub greed_percent: u16,
    /// Maximum amount the owner will offer per unit. This is an RFB-style
    /// purse cap, not a diminishing shop wallet.
    pub purchase_price_cap: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShopStockDefinition {
    pub item_kind_id: String,
    pub initial_minimum: u32,
    pub initial_maximum: u32,
    pub maintenance_minimum: u32,
    pub maintenance_maximum: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShopMaintenanceDefinition {
    pub interval_world_ticks: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ShopCategory {
    Shroomery,
    GeneralStore,
    Armoury,
    Weaponsmith,
    Temple,
    Alchemist,
    MagicShop,
    BlackMarket,
    Bookstore,
}
