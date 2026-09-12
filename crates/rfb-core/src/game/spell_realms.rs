// SPDX-License-Identifier: MPL-2.0
use super::*;
use rfb_protocol::{RealmChangeBookDto, SpellRealmsDto};

impl Game {
    /// Desktop fixtures: level 1 adds only a wand; other levels prepare a quiet map.
    /// Level 0 drains XP; the other levels exercise learning and class power boundaries.
    #[doc(hidden)]
    pub fn debug_prepare_spell_learning_e2e(&mut self, level: u16) -> Result<(), CoreError> {
        if !matches!(
            level,
            0 | 1 | 2 | 3 | 5 | 15 | 20 | 24 | 25 | 34 | 35 | 41 | 42 | 50
        ) {
            return Err(CoreError::InvalidSave(
                "unsupported spell learning E2E level",
            ));
        }
        if level == 1 {
            self.debug_add_generated_inventory_item(
                "e2e.mage-wand",
                "demo.item.magic-missile-wand",
                1,
            )?;
            self.mark_item_aware("demo.item.magic-missile-wand");
            return Ok(());
        }
        self.entities.clear();
        self.items
            .retain(|item| !matches!(item.location, ItemLocation::CarriedBy { .. }));
        self.glow.fill(true);
        if level == 0 {
            self.apply_player_experience_drain(
                self.progress.experience,
                "e2e.mage-drain",
                &mut Vec::new(),
            );
        } else {
            let experience = self
                .experience_required_for_level(level)
                .saturating_sub(self.progress.experience);
            self.apply_player_experience(experience, &mut Vec::new());
        }
        let ranger = self
            .build
            .as_ref()
            .is_some_and(|build| build.class_id == "demo.class.ranger");
        let (book_id, change_realm) = if ranger {
            (
                "e2e.ranger-change-book",
                if self.current_second_realm_id() == Some("death") {
                    "sorcery"
                } else {
                    "death"
                },
            )
        } else if self.player_is_priest() {
            (
                "e2e.priest-change-book",
                if self.current_second_realm_id() == Some("craft") {
                    "arcane"
                } else {
                    "craft"
                },
            )
        } else {
            ("e2e.mage-life-book", "life")
        };
        if !self.items.iter().any(|item| item.id == book_id) {
            let kind = self
                .content
                .item_definitions()
                .find(|item| {
                    item.ability_book_id
                        .as_deref()
                        .and_then(|id| self.content.ability_book(id))
                        .is_some_and(|book| {
                            book.realm_id.as_deref() == Some(change_realm) && book.rank == Some(1)
                        })
                })
                .expect("formal realm change book")
                .id
                .clone();
            self.debug_add_generated_inventory_item(book_id, &kind, 1)?;
        }
        if level == 25 && !self.player_is_warrior_mage() {
            self.debug_add_generated_inventory_item(
                "e2e.mage-acquirement",
                "demo.item.acquirement-scroll",
                1,
            )?;
            self.mark_item_aware("demo.item.acquirement-scroll");
        }
        if self.player_is_priest()
            && level >= 34
            && !self.items.iter().any(|item| item.id == "e2e.priest-dagger")
        {
            self.debug_add_generated_inventory_item("e2e.priest-dagger", "demo.item.dagger", 1)?;
        }
        if self.player_is_priest() && matches!(level, 35 | 42 | 50) {
            // Explicit desktop preparation: current-realm high books and a melee/power target.
            if level == 50 {
                let first_realm = self
                    .character_definitions()
                    .expect("Priest build")
                    .0
                    .first_realm_id
                    .as_deref();
                let kinds: Vec<_> = self
                    .content
                    .item_definitions()
                    .filter(|item| {
                        item.ability_book_id
                            .as_deref()
                            .and_then(|id| self.content.ability_book(id))
                            .is_some_and(|book| {
                                book.rank == Some(4)
                                    && (book.realm_id.as_deref() == first_realm
                                        || book.realm_id.as_deref()
                                            == self.current_second_realm_id())
                            })
                    })
                    .map(|item| item.id.clone())
                    .collect();
                for kind in kinds {
                    let id = format!("e2e.priest.{kind}");
                    if !self.items.iter().any(|item| item.id == id) {
                        self.debug_add_generated_inventory_item(&id, &kind, 1)?;
                    }
                }
            }
            let origin = self.player.position;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    self.replace_terrain_from_source(
                        Position {
                            x: origin.x + dx,
                            y: origin.y + dy,
                        },
                        "demo.terrain.floor",
                        terrain::TerrainChangeSource::Magic,
                        &mut Vec::new(),
                        &mut BTreeSet::new(),
                    );
                }
            }
            let actor = self.generated_actor(
                "e2e.priest-power-target".to_owned(),
                "demo.actor.sheep",
                Position {
                    x: origin.x - 1,
                    y: origin.y,
                },
            );
            self.entities.push(actor);
        }
        if ranger && level == 50 {
            // Explicit desktop fixtures: high books and a small tree/probing scene.
            // Keep learned spells, attributes and RNG outcomes from actual play.
            let kinds: Vec<_> = self
                .content
                .item_definitions()
                .filter(|item| {
                    item.ability_book_id
                        .as_deref()
                        .and_then(|id| self.content.ability_book(id))
                        .is_some_and(|book| {
                            book.rank == Some(4)
                                && matches!(book.realm_id.as_deref(), Some("nature" | "sorcery"))
                        })
                })
                .map(|item| item.id.clone())
                .collect();
            for kind in kinds {
                self.debug_add_generated_inventory_item(&format!("e2e.ranger.{kind}"), &kind, 1)?;
            }
            let origin = self.player.position;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    self.replace_terrain_from_source(
                        Position {
                            x: origin.x + dx,
                            y: origin.y + dy,
                        },
                        if dx == 1 && dy == 0 {
                            "demo.terrain.surface-tree"
                        } else {
                            "demo.terrain.floor"
                        },
                        terrain::TerrainChangeSource::Magic,
                        &mut Vec::new(),
                        &mut BTreeSet::new(),
                    );
                }
            }
            let actor = self.generated_actor(
                "e2e.ranger-probe-target".to_owned(),
                "demo.actor.sheep",
                Position {
                    x: origin.x - 1,
                    y: origin.y,
                },
            );
            self.entities.push(actor);
        }
        self.refresh_player_resource_maxima();
        self.refresh_player_ability_state();
        self.player.hp = self.effective_player_max_hp();
        for pool in self.resources.values_mut() {
            pool.current = pool.maximum;
        }
        self.reveal_current_visibility();
        Ok(())
    }

    pub(super) fn current_second_realm_id(&self) -> Option<&str> {
        if self.player_uses_dual_realm_learning() {
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
        if !self.player_uses_dual_realm_learning() {
            return Err(unavailable("class-unavailable"));
        }
        if self.map_scale != MapScaleDto::Local {
            return Err(unavailable("local-map-required"));
        }
        if let Some(reason) = self.ability_study_unavailable_reason() {
            return Err(unavailable(reason));
        }
        let (build, _, class, _) = self.character_definitions().expect("dual-realm build");
        let profile = class.casting_profile.as_ref().expect("dual-realm caster");
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
        if !class.allows_second_realm(
            build.first_realm_id.as_deref().expect("dual-realm primary"),
            realm_id,
        ) || !profile.realm_profiles.iter().any(|realm| {
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
    ) -> Result<bool, CoreError> {
        let book_id = self
            .pending_realm_change_book()
            .ok_or(CoreError::RealmChangeUnavailable("no-pending-change"))?
            .to_owned();
        if !confirm {
            self.mage_realms
                .as_mut()
                .expect("pending realm change")
                .pending_change_book_item_id = None;
            return Ok(false);
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
        let realms = self.mage_realms.as_mut().expect("pending realm change");
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
        let resolutions = self.apply_mogaminator_to_items(vec![book_id.clone()], false, false)?;
        self.record_mogaminator_resolutions(resolutions, events, changed);
        Ok(self
            .casting_profile()
            .is_some_and(|profile| profile.study_mode == CastingStudyMode::DivineRandom)
            && self.resolve_prayer_study(&book_id, events))
    }

    pub(super) fn validate_spell_realms(&self) -> Result<(), CoreError> {
        let invalid = CoreError::InvalidSave("spell realms are invalid");
        if !self.player_uses_dual_realm_learning() {
            return if self.mage_realms.is_none() {
                Ok(())
            } else {
                Err(invalid)
            };
        }
        let Some(realms) = &self.mage_realms else {
            return Err(invalid);
        };
        let (build, _, class, _) = self.character_definitions().expect("dual-realm build");
        let first = build.first_realm_id.as_deref().expect("dual-realm primary");
        let supported = |id: &str| class.allows_second_realm(first, id);
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
