// SPDX-License-Identifier: MPL-2.0
// RFB master a0d92b6378: duelist.c. Challenge identity belongs to Game, not the aim cursor.

use crate::game::*;

impl Game {
    pub(in crate::game) fn player_is_duelist(&self) -> bool {
        self.build
            .as_ref()
            .is_some_and(|build| build.class_id == "demo.class.duelist")
    }

    pub(in crate::game) fn duelist_equipment_error(&self) -> Option<&'static str> {
        let equipped = || {
            self.items
                .iter()
                .filter(|item| matches!(item.location, ItemLocation::Equipped { .. }))
        };
        let kind = |item: &ItemInstance| {
            self.content
                .item(&item.kind_id)
                .and_then(|definition| definition.rfb_base_kind)
        };
        let armor_weight: u32 = equipped()
            .filter(|item| kind(item).is_some_and(|kind| (30..=38).contains(&kind.tval)))
            .map(|item| u32::from(self.item_instance_weight(item)))
            .sum();
        if armor_weight > 120 + 3 * u32::from(self.progress.level) {
            Some("duelist-heavy-armor")
        } else if equipped().any(|item| kind(item).is_some_and(|kind| matches!(kind.tval, 11 | 34)))
        {
            Some("duelist-shield")
        } else if self.equipped_melee_weapons().len() > 1 {
            Some("duelist-multiple-weapons")
        } else if equipped()
            .any(|item| kind(item).is_some_and(|kind| (kind.tval, kind.sval) == (23, 32)))
        {
            Some("duelist-poison-needle")
        } else if self
            .player_equipment_passives()
            .contains(&EquipmentPassive::AntiMagic)
        {
            // The timed anti-magic status represents tim_no_spells, not an equipment error.
            Some("anti-magic")
        } else {
            None
        }
    }

    pub(in crate::game) fn duelist_challenge_is_valid(&self) -> bool {
        self.duelist_target_id.as_ref().is_none_or(|id| {
            self.player_is_duelist()
                && self.duelist_equipment_error().is_none()
                && self
                    .entities
                    .iter()
                    .any(|entity| entity.id == *id && entity.hp > 0)
        })
    }

    pub(in crate::game) fn refresh_duelist_challenge(&mut self) {
        if !self.duelist_challenge_is_valid() {
            self.duelist_target_id = None;
        }
    }

    pub(in crate::game) fn clear_duelist_challenge_for(&mut self, entity_id: &str) {
        if self.duelist_target_id.as_deref() == Some(entity_id) {
            self.duelist_target_id = None;
        }
    }

    pub(in crate::game) fn duelist_challenge_target(
        &self,
        target: &TargetSelection,
    ) -> Option<String> {
        let TargetSelection::Entity { entity_id } = target else {
            return None;
        };
        self.entities
            .iter()
            .find(|entity| {
                entity.id == *entity_id
                    && entity.hp > 0
                    // Mounted actors must remain controlled in the current riding model.
                    // Dismount before making this actor hostile; never leave overlapping actors.
                    && self.riding_actor_id.as_ref() != Some(entity_id)
                    && self.duelist_target_id.as_ref() != Some(entity_id)
                    && self.entity_is_visible_to_player(entity)
            })
            .map(|entity| entity.id.clone())
    }

    pub(in crate::game) fn duelist_cast_is_zero_time_unavailable(
        &self,
        ability_id: &str,
        target: &TargetSelection,
    ) -> bool {
        self.player_is_duelist()
            && ability_id == "demo.ability.duelist-mark-target"
            && (self.player_has_status_kind(STATUS_CONFUSION)
                || self.player_has_status_kind(STATUS_FEAR)
                || self.duelist_challenge_target(target).is_none())
    }

    pub(super) fn resolve_duelist_challenge(
        &mut self,
        target_entity_id: String,
        events: &mut Vec<DomainEvent>,
    ) {
        let index = self
            .entities
            .iter()
            .position(|entity| entity.id == target_entity_id)
            .expect("validated challenge target must remain available");
        self.wake_entity(index, events);
        self.entities[index].alerted = true;
        self.entities[index].friendly = false;
        self.entities[index].controller_id = None;
        self.clear_riding_bond_for(&target_entity_id);
        self.duelist_target_id = Some(target_entity_id);
        events.push(DomainEvent::DuelistChallengeIssued {
            target_kind_id: self.entities[index].kind_id.clone(),
        });
    }
}
