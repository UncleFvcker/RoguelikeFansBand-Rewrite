// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

use rfb_protocol::{
    GameCommand, InventoryItemSaveDto, ItemKnowledgeSaveDto, ItemPropertyKnowledgeSaveDto,
};
use serde::{Deserialize, Serialize};

use super::Game;
use crate::CoreError;

/// Serialized collection at the local profile boundary. Character saves keep their
/// own projection; the desktop commits ownership together with a character checkpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SharedMuseum {
    pub inventory: Vec<InventoryItemSaveDto>,
    pub item_knowledge: Vec<ItemKnowledgeSaveDto>,
    pub item_property_knowledge: Vec<ItemPropertyKnowledgeSaveDto>,
}

impl Game {
    #[must_use]
    pub fn has_shared_museum(&self) -> bool {
        self.museum_storage_id().is_some()
    }

    fn museum_storage_id(&self) -> Option<&str> {
        self.home_states.keys().find_map(|id| {
            self.content
                .town_facility(id)
                .filter(|facility| facility.reject_artifact_deposits)
                .map(|_| id.as_str())
        })
    }

    #[must_use]
    pub fn is_museum_transfer(&self, command: &GameCommand) -> bool {
        let (GameCommand::DepositAtHome { facility_id, .. }
        | GameCommand::WithdrawFromHome { facility_id, .. }) = command
        else {
            return false;
        };
        self.content
            .town_facility(facility_id)
            .is_some_and(|facility| facility.reject_artifact_deposits)
    }

    #[must_use]
    pub fn shared_museum(&self) -> Option<SharedMuseum> {
        let storage_id = self.museum_storage_id()?;
        let save = self.to_save();
        let inventory = save
            .home_states
            .into_iter()
            .find(|home| home.facility_id == storage_id)?
            .inventory;
        let item_knowledge = save
            .item_knowledge
            .into_iter()
            // A shared collection transfers awareness, not another character's
            // cumulative discoveries. OM_COUNTED remains on each stored book.
            .filter(|entry| entry.tried)
            .map(|mut entry| {
                entry.found_count = 0;
                entry
            })
            .filter(|entry| inventory.iter().any(|item| item.kind_id == entry.kind_id))
            .collect();
        let item_property_knowledge = save
            .item_property_knowledge
            .into_iter()
            .filter(|entry| inventory.iter().any(|item| item.id == entry.item_id))
            .collect();
        Some(SharedMuseum {
            inventory,
            item_knowledge,
            item_property_knowledge,
        })
    }

    /// Import only after the museum's town state exists. All incoming IDs are
    /// allocated by Game so another character's local IDs cannot collide here.
    pub fn with_shared_museum(&self, museum: &SharedMuseum) -> Result<Option<Self>, CoreError> {
        let Some(storage_id) = self.museum_storage_id() else {
            return Ok(None);
        };
        let invalid = || CoreError::InvalidSave("shared museum state is invalid");
        let mut allocator = self.clone();
        let mut save = self.to_save();
        let home = save
            .home_states
            .iter_mut()
            .find(|home| home.facility_id == storage_id)
            .unwrap();
        let old_ids = home
            .inventory
            .iter()
            .map(|item| item.id.clone())
            .collect::<BTreeSet<_>>();
        let mut ids = BTreeMap::new();
        let mut inventory = museum.inventory.clone();
        for item in &mut inventory {
            let definition = self.content.item(&item.kind_id).ok_or_else(invalid)?;
            if definition.artifact_generation.is_some() || ids.contains_key(&item.id) {
                return Err(invalid());
            }
            let id = allocator.allocate_item_instance_id()?;
            ids.insert(item.id.clone(), id.clone());
            item.id = id;
        }
        home.inventory = inventory;
        save.next_item_instance_serial = allocator.next_item_instance_serial;
        save.item_property_knowledge
            .retain(|entry| !old_ids.contains(&entry.item_id));
        for entry in &museum.item_property_knowledge {
            let mut entry = entry.clone();
            entry.item_id = ids.get(&entry.item_id).ok_or_else(invalid)?.clone();
            save.item_property_knowledge.push(entry);
        }
        let mut kinds = BTreeSet::new();
        for entry in &museum.item_knowledge {
            if !entry.tried
                || entry.found_count != 0
                || !kinds.insert(&entry.kind_id)
                || !museum
                    .inventory
                    .iter()
                    .any(|item| item.kind_id == entry.kind_id)
            {
                return Err(invalid());
            }
            if let Some(current) = save
                .item_knowledge
                .iter_mut()
                .find(|current| current.kind_id == entry.kind_id)
            {
                current.tried |= entry.tried;
                current.aware |= entry.aware;
            } else {
                save.item_knowledge.push(entry.clone());
            }
        }
        // Reuse the complete content, item, knowledge and uniqueness validation.
        Self::from_save(save).map(Some)
    }
}
