// SPDX-License-Identifier: MPL-2.0
use super::*;
use rfb_protocol::{RealmChangeBookDto, SpellRealmsDto};

impl Game {
    pub(super) fn current_second_realm_id(&self) -> Option<&str> {
        if self.player_is_mage() {
            self.mage_realms
                .as_ref()
                .map(|realms| realms.second_realm_id.as_str())
        } else {
            self.character_definitions()
                .and_then(|(build, _, _, _)| build.second_realm_id.as_deref())
        }
    }

    pub(super) fn pending_realm_change_book(&self) -> Option<&str> {
        self.mage_realms
            .as_ref()?
            .pending_change_book_item_id
            .as_deref()
    }

    pub(super) fn realm_change_book(&self, item_id: &str) -> Result<RealmChangeBookDto, CoreError> {
        let unavailable = CoreError::RealmChangeUnavailable;
        if !self.player_is_mage() {
            return Err(unavailable("class-unavailable"));
        }
        if self.map_scale != MapScaleDto::Local {
            return Err(unavailable("local-map-required"));
        }
        if let Some(reason) = self.ability_study_unavailable_reason() {
            return Err(unavailable(reason));
        }
        let profile = self.casting_profile().expect("Mage caster");
        if self.ability_learning_remaining(profile) == 0 {
            return Err(unavailable("learning-capacity-full"));
        }
        let book_id = self
            .study_book_id(item_id)
            .ok_or(unavailable("book-unavailable"))?;
        let book = self.content.ability_book(book_id).expect("validated book");
        let realm_id = book
            .realm_id
            .as_deref()
            .ok_or(unavailable("unsupported-realm"))?;
        if self
            .active_casting_realm_profiles()
            .iter()
            .any(|realm| realm.realm_id == realm_id)
        {
            return Err(unavailable("already-active-realm"));
        }
        if !profile.realm_profiles.iter().any(|realm| {
            realm.realm_id == realm_id && realm.ability_book_ids.iter().any(|id| id == book_id)
        }) {
            return Err(unavailable("unsupported-realm"));
        }
        Ok(RealmChangeBookDto {
            book_item_id: item_id.to_owned(),
            realm_id: realm_id.to_owned(),
        })
    }

    pub(super) fn spell_realms_dto(&self) -> Option<SpellRealmsDto> {
        let realms = self.mage_realms.as_ref()?;
        let first_realm_id = self.character_definitions()?.0.first_realm_id.clone()?;
        let mut change_books: Vec<_> = self
            .items
            .iter()
            .filter_map(|item| self.realm_change_book(&item.id).ok())
            .collect();
        change_books.sort_by(|a, b| a.book_item_id.cmp(&b.book_item_id));
        Some(SpellRealmsDto {
            first_realm_id,
            second_realm_id: realms.second_realm_id.clone(),
            previous_realm_ids: realms.previous_realm_ids.clone(),
            pending_change: realms.pending_change_book_item_id.as_ref().and_then(|id| {
                change_books
                    .iter()
                    .find(|book| &book.book_item_id == id)
                    .cloned()
            }),
            change_books,
        })
    }

    pub(super) fn resolve_realm_change(
        &mut self,
        confirm: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Result<(), CoreError> {
        let book_id = self
            .pending_realm_change_book()
            .ok_or(CoreError::RealmChangeUnavailable("no-pending-change"))?
            .to_owned();
        if !confirm {
            self.mage_realms
                .as_mut()
                .expect("pending Mage")
                .pending_change_book_item_id = None;
            return Ok(());
        }
        let choice = self.realm_change_book(&book_id)?;
        let old_abilities: BTreeSet<_> = self.active_casting_realm_profiles()[1]
            .ability_book_ids
            .iter()
            .flat_map(|id| {
                self.content
                    .ability_book(id)
                    .expect("active book")
                    .ability_ids
                    .iter()
                    .cloned()
            })
            .collect();
        self.ability_learning_order
            .retain(|id| !old_abilities.contains(id));
        let realms = self.mage_realms.as_mut().expect("pending Mage");
        if !realms.previous_realm_ids.contains(&realms.second_realm_id) {
            realms
                .previous_realm_ids
                .push(realms.second_realm_id.clone());
            realms.previous_realm_ids.sort();
        }
        realms.second_realm_id = choice.realm_id;
        realms.pending_change_book_item_id = None;
        // Refresh drops old secondary progress and starts the new books at zero.
        // Paid study and primary progress survive, including after cancelling spell selection.
        self.refresh_player_ability_state();
        let resolutions = self.apply_mogaminator_to_items(vec![book_id], false, false)?;
        self.record_mogaminator_resolutions(resolutions, events, changed);
        Ok(())
    }

    pub(super) fn validate_mage_realms(&self) -> Result<(), CoreError> {
        let invalid = CoreError::InvalidSave("Mage realms are invalid");
        if !self.player_is_mage() {
            return if self.mage_realms.is_none() {
                Ok(())
            } else {
                Err(invalid)
            };
        }
        let Some(realms) = &self.mage_realms else {
            return Err(invalid);
        };
        let (build, _, class, _) = self.character_definitions().expect("Mage build");
        let profile = class.casting_profile.as_ref().expect("Mage caster");
        let supported = |id: &str| {
            Some(id) != build.first_realm_id.as_deref()
                && profile
                    .realm_profiles
                    .iter()
                    .any(|realm| realm.realm_id == id)
        };
        if !supported(&realms.second_realm_id)
            || !realms.previous_realm_ids.iter().all(|id| supported(id))
            || !realms
                .previous_realm_ids
                .windows(2)
                .all(|ids| ids[0] < ids[1])
            || (realms.previous_realm_ids.is_empty()
                && Some(realms.second_realm_id.as_str()) != build.second_realm_id.as_deref())
            || (!realms.previous_realm_ids.is_empty()
                && !realms
                    .previous_realm_ids
                    .iter()
                    .any(|id| Some(id.as_str()) == build.second_realm_id.as_deref()))
        {
            return Err(invalid);
        }
        Ok(())
    }
}
