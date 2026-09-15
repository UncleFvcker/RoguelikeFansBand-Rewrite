// SPDX-License-Identifier: MPL-2.0
//! RFB master a0d92b6378: defines.h::OFC_*, object1.c::obj_learn_curse.
use super::*;

pub(in crate::game) fn curse_effect_flag(effect: ItemCurseEffectDto) -> u32 {
    let index = super::super::ego::curses::CURSE_EFFECTS
        .iter()
        .position(|candidate| *candidate == Some(effect))
        .expect("supported curse effect");
    1_u32 << (index + 4)
}

impl Game {
    pub(in crate::game) fn actual_item_curse_flags(&self, item: &ItemInstance) -> u32 {
        let Some(severity) = item.curse else {
            return 0;
        };
        let mut flags = 1;
        if self.item_has_heavy_curse(item) {
            flags |= 2;
        }
        if severity == ItemCurseSeverityDto::Permanent {
            flags |= 4;
        }
        for effect in item.intrinsic_curse_effects.iter().chain(
            item.rolled_affixes
                .iter()
                .flat_map(|roll| &roll.curse_effects),
        ) {
            flags |= curse_effect_flag(*effect);
        }
        flags
    }

    pub(in crate::game) fn known_item_curse_flags(&self, item: &ItemInstance) -> u32 {
        self.item_property_knowledge
            .get(&item.id)
            .map_or(0, |knowledge| {
                (knowledge.known_curse_flags | u32::from(knowledge.known_curse))
                    & self.actual_item_curse_flags(item)
            })
    }

    pub(in crate::game) fn item_curse_effect_is_known(
        &self,
        item: &ItemInstance,
        effect: ItemCurseEffectDto,
    ) -> bool {
        let intrinsic_flag = match effect {
            ItemCurseEffectDto::Aggravate => Some("AGGRAVATE"),
            ItemCurseEffectDto::Teleport => Some("TELEPORT"),
            ItemCurseEffectDto::TyCurse => Some("TY_CURSE"),
            ItemCurseEffectDto::DrainExperience => Some("DRAIN_EXP"),
            _ => None,
        };
        self.known_item_curse_flags(item) & curse_effect_flag(effect) != 0
            || intrinsic_flag.is_some_and(|flag| self.known_item_flags(item).contains(flag))
    }

    pub(in crate::game) fn learn_item_curse(&mut self, item_id: &str, effect: ItemCurseEffectDto) {
        let item = self
            .items
            .iter()
            .find(|item| item.id == item_id)
            .expect("curse source exists");
        let flag = self.actual_item_curse_flags(item) & curse_effect_flag(effect);
        if flag != 0 {
            let knowledge = self
                .item_property_knowledge
                .entry(item_id.to_owned())
                .or_default();
            knowledge.discovered = true;
            knowledge.known_curse_flags |= flag;
        }
    }

    pub(in crate::game) fn learn_all_item_curses(&mut self, item_id: &str) {
        let item = self
            .items
            .iter()
            .find(|item| item.id == item_id)
            .expect("identified item exists");
        let flags = self.actual_item_curse_flags(item);
        let knowledge = self
            .item_property_knowledge
            .entry(item_id.to_owned())
            .or_default();
        knowledge.known_curse_flags = flags;
        knowledge.known_curse = flags & 1 != 0;
    }

    pub(in crate::game) fn learn_equipped_curse(&mut self, effect: ItemCurseEffectDto) {
        let ids = self
            .items
            .iter()
            .filter(|item| self.item_has_active_equipped_curse_effect(item, effect))
            .map(|item| item.id.clone())
            .collect::<Vec<_>>();
        for id in ids {
            self.learn_item_curse(&id, effect);
        }
        if effect == ItemCurseEffectDto::DrainExperience {
            self.learn_equipment_flag("DRAIN_EXP");
        }
    }

    pub(in crate::game) fn item_needs_full_identification(&self, item: &ItemInstance) -> bool {
        self.item_identification(item) != ItemIdentificationDto::Identified
            || self.actual_item_curse_flags(item) & !self.known_item_curse_flags(item) != 0
    }

    pub(in crate::game) fn item_needs_identification(
        &self,
        item: &ItemInstance,
        full: bool,
    ) -> bool {
        if full || self.easy_identification {
            self.item_needs_full_identification(item)
        } else {
            !self.item_identity_is_known(item)
        }
    }
}
