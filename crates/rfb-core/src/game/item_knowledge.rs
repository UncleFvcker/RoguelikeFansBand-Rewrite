// SPDX-License-Identifier: MPL-2.0

use super::*;
use rfb_protocol::ItemFeelingDto;
mod curses;
mod learning;
mod lore;

// RFB master a0d92b6378: tables.c::adj_pseudo_id.
const RFB_PSEUDO_ID_ADJUSTMENT: [u16; 38] = [
    150, 135, 127, 122, 118, 115, 112, 110, 108, 106, 105, 104, 103, 102, 101, 100, 99, 98, 97, 96,
    95, 94, 93, 92, 91, 90, 88, 86, 84, 82, 80, 78, 76, 74, 72, 70, 68, 65,
];

pub(super) fn item_can_be_sensed(item: &rfb_content::ItemDefinition) -> bool {
    // RFB master a0d92b6378: obj.c::obj_can_sense1/2.
    if item.capture_ball {
        return false;
    }
    if let Some(kind) = item.rfb_base_kind {
        return matches!(kind.tval, 8 | 16..=23 | 30..=40 | 45 | 46 | 50 | 55 | 65 | 66);
    }
    matches!(
        item.equipment_slot.as_deref(),
        Some(
            "weapon"
                | "launcher"
                | "tool"
                | "quiver"
                | "boots"
                | "gloves"
                | "head"
                | "shield"
                | "cloak"
                | "body"
                | "light"
                | "ring"
                | "amulet"
        )
    ) || item.tags.iter().any(|tag| {
        matches!(
            tag.as_str(),
            "ammunition" | "wand" | "staff" | "rod" | "figurine" | "card"
        )
    })
}

impl Game {
    pub(super) fn item_activation_is_known(&self, item: &ItemInstance) -> bool {
        if self
            .content
            .item(&item.kind_id)
            .and_then(|definition| definition.device_generation.as_ref())
            .is_some_and(|generation| generation.rfb_device.is_some())
        {
            self.item_identification(item) != ItemIdentificationDto::Unexamined
        } else if item.activation.is_some() {
            self.item_identification(item) == ItemIdentificationDto::Identified
                || self.known_item_flags(item).contains("ACTIVATE")
        } else {
            self.item_knowledge_dto(&item.kind_id) == ItemKnowledgeDto::Aware
        }
    }

    pub(super) fn lose_mindcraft_information(&mut self, changed: &mut BTreeSet<Position>) {
        if !self.player_auto_identifies_items() {
            let ids = self
                .items
                .iter()
                .filter(|item| {
                    matches!(
                        item.location,
                        ItemLocation::Inventory | ItemLocation::Equipped { .. }
                    ) && self.item_identification(item) != ItemIdentificationDto::Identified
                })
                .map(|item| item.id.clone())
                .collect::<Vec<_>>();
            for id in ids {
                if let Some(knowledge) = self.item_property_knowledge.get_mut(&id) {
                    knowledge.appraised = false;
                    knowledge.feeling = None;
                    knowledge.known_affix_ids.clear();
                    // effects.c::_forget retains instance flags and shared lore.
                }
                if self.player_is_berserker()
                    || self.player_is_duelist()
                    || self.player_has_tomte_item_sensing()
                {
                    self.sense_item_instance(&id, true);
                }
            }
        }
        self.add_virtue(VirtueKindDto::Knowledge, -5);
        self.add_virtue(VirtueKindDto::Enlightenment, -5);
        self.clear_current_floor_memory(changed);
    }

    fn item_base_properties_known(&self, item: &ItemInstance) -> bool {
        self.item_knowledge_dto(&item.kind_id) == ItemKnowledgeDto::Aware
            && (self.item_identification(item) == ItemIdentificationDto::Identified
                || self
                    .content
                    .item(&item.kind_id)
                    .is_some_and(|definition| !definition.tags.iter().any(|tag| tag == "artifact")))
    }

    pub(super) fn player_has_tomte_item_sensing(&self) -> bool {
        self.character_definitions()
            .is_some_and(|(_, race, _, _)| race.id == "rfb-legacy.race.tomte")
            && self.player_tomte_headgear_excess_weight() == 0
    }

    pub(super) fn item_feeling(&self, item: &ItemInstance) -> Option<ItemFeelingDto> {
        self.item_property_knowledge
            .get(&item.id)
            .and_then(|knowledge| knowledge.feeling)
    }

    fn sense_item_instance(&mut self, item_id: &str, strong: bool) {
        self.sense_item(item_id, strong, false);
    }

    pub(super) fn psychometry_item(&mut self, item_id: &str) {
        self.sense_item(item_id, true, true);
    }

    fn sense_item(&mut self, item_id: &str, strong: bool, psychometry: bool) {
        let item = self
            .items
            .iter()
            .find(|item| item.id == item_id)
            .expect("sensed item must exist");
        if self.item_identification(item) != ItemIdentificationDto::Unexamined
            || (!psychometry && self.item_feeling(item).is_some())
        {
            return;
        }
        let definition = self
            .content
            .item(&item.kind_id)
            .expect("item kind must exist");
        if !psychometry && !item_can_be_sensed(definition) {
            return;
        }
        // RFB dungeon.c::value_check_aux1/2; weak feelings must not reveal egos/artifacts.
        let broken = definition.base_value == 0;
        let cursed = item.curse.is_some();
        let device = definition
            .tags
            .iter()
            .any(|tag| matches!(tag.as_str(), "wand" | "staff" | "rod"));
        let known_on_average = matches!(
            definition.equipment_slot.as_deref(),
            Some("ring" | "amulet" | "quiver")
        ) || definition.id == "demo.item.feanorian-lamp";
        let feeling = if !strong && cursed && !device {
            ItemFeelingDto::Cursed
        } else if !strong && broken {
            ItemFeelingDto::Broken
        } else if !strong
            && (item.is_artifact(&self.content)
                || !item.affix_ids.is_empty()
                || !item.rolled_affixes.is_empty())
        {
            ItemFeelingDto::Enchanted
        } else if item.is_artifact(&self.content) {
            if cursed || broken {
                ItemFeelingDto::Terrible
            } else {
                ItemFeelingDto::Special
            }
        } else if !item.affix_ids.is_empty() || !item.rolled_affixes.is_empty() {
            if cursed || broken {
                ItemFeelingDto::Awful
            } else {
                ItemFeelingDto::Excellent
            }
        } else if cursed && (strong || !device) {
            ItemFeelingDto::Bad
        } else if broken {
            ItemFeelingDto::Broken
        } else if strong && known_on_average {
            // These nameless kinds become known instead of retaining an average feeling.
            self.identify_item_instance(item_id, inventory::ItemIdentificationRequest::new(false));
            if psychometry {
                self.item_property_knowledge
                    .entry(item_id.to_owned())
                    .or_default()
                    .feeling = Some(ItemFeelingDto::Average);
            }
            return;
        } else if item.enchantments.to_armor > 0 {
            ItemFeelingDto::Good
        } else if matches!(
            definition.equipment_slot.as_deref(),
            Some("gloves" | "boots")
        ) {
            ItemFeelingDto::Average
        } else {
            let native_bonus = definition
                .melee_profile
                .as_ref()
                .map(|profile| profile.to_hit.saturating_add(profile.to_damage))
                .or_else(|| {
                    definition
                        .projectile_profile
                        .as_ref()
                        .map(|profile| profile.to_hit.saturating_add(profile.to_damage))
                })
                .or_else(|| {
                    definition
                        .ammunition_profile
                        .as_ref()
                        .map(|profile| profile.to_hit.saturating_add(profile.to_damage))
                })
                .unwrap_or(0);
            if native_bonus
                .saturating_add(i32::from(item.enchantments.to_hit))
                .saturating_add(i32::from(item.enchantments.to_damage))
                > 0
            {
                ItemFeelingDto::Good
            } else {
                ItemFeelingDto::Average
            }
        };
        if !strong && known_on_average && feeling == ItemFeelingDto::Average {
            self.identify_item_instance(item_id, inventory::ItemIdentificationRequest::new(false));
            return;
        }
        let feeling = if !strong && feeling == ItemFeelingDto::Good {
            ItemFeelingDto::Enchanted
        } else {
            feeling
        };
        let knowledge = self
            .item_property_knowledge
            .entry(item_id.to_owned())
            .or_default();
        knowledge.discovered = true;
        knowledge.feeling = Some(feeling);
    }

    pub(super) fn process_class_item_sensing(&mut self) {
        if !(self.easy_identification
            || self.player_is_mindcrafter()
            || self.player_is_mage()
            || self.player_is_necromancer()
            || self.player_is_bard()
            || self.player_is_rogue()
            || (self.player_is_samurai() || self.player_is_rage_mage())
            || self.player_is_ranger()
            || self.player_is_priest()
            || self.player_is_warrior_mage()
            || self.player_is_magic_eater())
            || self.player_has_status_kind(STATUS_CONFUSION)
            || !self.world_tick.is_multiple_of(10)
        {
            return;
        }
        let level = u32::from(self.progress.level);
        let wisdom = self
            .effective_player_attributes()
            .index(AttributeKind::Wisdom)
            .min(crate::stats::PRE_VICTORY_ATTRIBUTE_INDEX_CAP);
        let knowledge = i32::from(self.virtue_current(VirtueKindDto::Knowledge));
        // ponytail: pack, quiver and bag share one inventory; use the pack's
        // 1-in-3 gate until items carry an actual container identity.
        let frequencies = if self.easy_identification {
            [(false, 0_u32), (true, 0)]
        } else if self.player_is_samurai() || self.player_is_rage_mage() {
            [(false, 9_000_u32), (true, 0)]
        } else if self.player_is_mage()
            || self.player_is_magic_eater()
            || self.player_is_necromancer()
        {
            [(false, 20_000_u32), (true, 9_000)]
        } else if self.player_is_ranger() {
            [(false, 80_000_u32), (true, 80_000)]
        } else if self.player_is_priest() || self.player_is_bard() || self.player_is_rogue() {
            // RFB master a0d92b6378: priest.c FAST/WEAK, MED/STRONG.
            [(false, 9_000_u32), (true, 20_000)]
        } else if self.player_is_warrior_mage() {
            [(false, 20_000_u32), (true, 20_000)]
        } else {
            [(false, 80_000_u32), (true, 20_000)]
        };
        for (second, frequency) in frequencies {
            if !self.easy_identification
                && (self.player_is_samurai() || self.player_is_rage_mage())
                && second
            {
                continue;
            }
            let adjusted =
                frequency * u32::from(RFB_PSEUDO_ID_ADJUSTMENT[usize::from(wisdom)]) / 100;
            let adjusted = adjusted * (625 - knowledge) as u32 / 625;
            let chance = if level >= 35 {
                0
            } else {
                (adjusted >> (level / 5)) / ((level + 10).pow(2) + 40)
            };
            if chance > 1 && self.rng.bounded(u64::from(chance)) != 0 {
                continue;
            }
            let mut candidates = self
                .items
                .iter()
                .filter(|item| {
                    matches!(
                        item.location,
                        ItemLocation::Inventory | ItemLocation::Equipped { .. }
                    ) && self.item_identification(item) == ItemIdentificationDto::Unexamined
                        && self.item_feeling(item).is_none()
                })
                .filter_map(|item| {
                    let definition = self.content.item(&item.kind_id)?;
                    if !item_can_be_sensed(definition) {
                        return None;
                    }
                    let sense2 = definition.rfb_base_kind.map_or_else(
                        || {
                            matches!(
                                definition.equipment_slot.as_deref(),
                                Some("ring" | "amulet" | "light")
                            ) || definition.tags.iter().any(|tag| {
                                matches!(tag.as_str(), "wand" | "staff" | "rod" | "figurine")
                            })
                        },
                        |kind| matches!(kind.tval, 8 | 39 | 40 | 45 | 55 | 65 | 66),
                    );
                    (sense2 == second)
                        .then(|| (item.id.clone(), item.location == ItemLocation::Inventory))
                })
                .collect::<Vec<_>>();
            candidates.sort();
            for (item_id, in_pack) in candidates {
                if in_pack && self.rng.bounded(3) != 0 {
                    continue;
                }
                let strong = self.easy_identification
                    || self.player_is_rogue()
                    || (self.player_is_samurai() || self.player_is_rage_mage())
                    || self.player_is_ranger()
                    || second
                    || knowledge >= 100
                    || (self.player_has_mutation("rfb.mutation.good-luck")
                        && self.rng.bounded(13) == 0);
                self.sense_item_instance(&item_id, strong);
            }
        }
    }

    pub(super) fn apply_player_floor_item_knowledge(&mut self) {
        if self.map_scale != MapScaleDto::Local {
            return;
        }
        let item_ids = self
            .items
            .iter()
            .filter(|item| item.location == ItemLocation::Ground(self.player.position))
            .map(|item| item.id.clone())
            .collect::<Vec<_>>();
        self.apply_player_item_knowledge(item_ids);
    }

    pub(super) fn maia_sense_carried_curse(&mut self, item_id: &str) {
        if !self.player_is_enlightened_maia()
            || !self.items.iter().any(|item| {
                item.id == item_id
                    && item.curse.is_some()
                    && matches!(
                        item.location,
                        ItemLocation::Inventory | ItemLocation::Equipped { .. }
                    )
            })
        {
            return;
        }
        let knowledge = self
            .item_property_knowledge
            .entry(item_id.to_owned())
            .or_default();
        knowledge.discovered = true;
        knowledge.known_curse = true;
        if !knowledge.appraised && !knowledge.identified && knowledge.feeling.is_none() {
            knowledge.feeling = Some(ItemFeelingDto::Cursed);
        }
    }

    pub(super) fn apply_player_item_knowledge(&mut self, mut item_ids: Vec<String>) {
        let identifies = self.player_auto_identifies_items();
        let senses = self.player_is_berserker()
            || self.player_is_duelist()
            || self.player_has_tomte_item_sensing();
        if !identifies && !senses {
            return;
        }
        item_ids.sort();
        for item_id in item_ids {
            if senses {
                self.sense_item_instance(&item_id, true);
            }
            if identifies {
                self.identify_item_instance(
                    &item_id,
                    inventory::ItemIdentificationRequest::new(false),
                );
            }
        }
    }

    pub(super) fn visible_artifact_name(&self, item: &ItemInstance) -> Option<String> {
        (self.item_identification(item) != ItemIdentificationDto::Unexamined)
            .then(|| item.artifact_name.clone())
            .flatten()
    }

    pub(super) fn visible_item_bag_capacity(&self, item: &ItemInstance) -> Option<u16> {
        (self.item_identification(item) != ItemIdentificationDto::Unexamined)
            .then(|| super::inventory::item_bag_capacity(&self.content, item))
            .flatten()
    }

    pub(super) fn visible_item_modifiers(&self, item: &ItemInstance) -> StatModifiersDto {
        let mut modifiers = self.known_item_modifiers(item);
        if self.item_base_properties_known(item)
            || self.item_identification(item) != ItemIdentificationDto::Unexamined
        {
            modifiers.defense = self.body_armor_for_current_form(item, modifiers.defense);
        }
        modifiers
    }

    fn visible_item_property_bundles(
        &self,
        item: &ItemInstance,
    ) -> Vec<rfb_content::AffixPropertyBundleDefinition> {
        let identified = self.item_identification(item) != ItemIdentificationDto::Unexamined;
        let fully_known = self.item_identification(item) == ItemIdentificationDto::Identified;
        let flags = if fully_known {
            BTreeSet::new()
        } else {
            self.known_item_flags(item)
        };
        let knowledge = self.item_property_knowledge.get(&item.id);
        let kind = self.content.item(&item.kind_id).expect("item kind exists");
        let base = lore::base_properties(kind);
        let mut bundles = vec![if fully_known || self.item_base_properties_known(item) {
            base
        } else {
            lore::filter_properties(base, &flags, identified)
        }];
        for id in &item.affix_ids {
            let base = lore::affix_properties(self.content.affix(id).expect("affix exists"));
            let known = fully_known || knowledge.is_some_and(|k| k.known_affix_ids.contains(id));
            bundles.push(if known {
                base
            } else {
                lore::filter_properties(base, &flags, identified)
            });
        }
        bundles.push(if fully_known {
            item.intrinsic_properties.clone()
        } else {
            lore::filter_properties(item.intrinsic_properties.clone(), &flags, identified)
        });
        for roll in &item.rolled_affixes {
            let known = fully_known
                || knowledge.is_some_and(|k| k.known_affix_ids.contains(&roll.affix_id));
            bundles.push(if known {
                roll.properties.clone()
            } else {
                lore::filter_properties(roll.properties.clone(), &flags, identified)
            });
        }
        bundles
    }

    fn known_item_modifiers(&self, item: &ItemInstance) -> StatModifiersDto {
        let mut modifiers = StatModifiersDto::default();
        for bundle in self.visible_item_property_bundles(item) {
            add_stat_modifiers_dto(&mut modifiers, &bundle.modifiers);
        }
        modifiers
    }

    pub(super) fn visible_item_equipment_bonuses(
        &self,
        item: &ItemInstance,
    ) -> EquipmentBonusesDto {
        let mut bundles = self.visible_item_property_bundles(item);
        if self.item_is_fixed_artifact(item, 378) {
            bundles[0].equipment_bonuses.melee_skill = 0;
            bundles[0].equipment_bonuses.melee_damage = 0;
        }
        let mut bonuses = EquipmentBonuses::default();
        for bundle in bundles {
            merge_equipment_bonuses(&mut bonuses, &bundle.equipment_bonuses);
        }
        equipment_bonuses_dto(&bonuses)
    }

    pub(super) fn visible_item_passives(&self, item: &ItemInstance) -> Vec<EquipmentPassiveDto> {
        self.visible_item_property_bundles(item)
            .into_iter()
            .flat_map(|bundle| bundle.passives)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(equipment_passive_dto)
            .collect()
    }

    pub(super) fn known_item_properties(&self, item: &ItemInstance) -> Vec<ItemPropertyDto> {
        if item.affix_ids.is_empty() {
            return Vec::new();
        }
        let knowledge = self.item_property_knowledge.get(&item.id);
        let identified = self.item_identification(item) != ItemIdentificationDto::Unexamined;
        let fully_known = self.item_identification(item) == ItemIdentificationDto::Identified;
        let flags = if fully_known {
            BTreeSet::new()
        } else {
            self.known_item_flags(item)
        };
        item.affix_ids
            .iter()
            .filter(|id| identified || knowledge.is_some_and(|k| k.known_affix_ids.contains(*id)))
            .map(|id| {
                let affix = self.content.affix(id).expect("affix exists");
                let all_known =
                    fully_known || knowledge.is_some_and(|k| k.known_affix_ids.contains(id));
                let filter = |bundle| {
                    if all_known {
                        bundle
                    } else {
                        lore::filter_properties(bundle, &flags, identified)
                    }
                };
                let mut bundle = filter(lore::affix_properties(affix));
                for roll in item
                    .rolled_affixes
                    .iter()
                    .filter(|roll| &roll.affix_id == id)
                {
                    super::ego::merge_affix_properties(
                        &mut bundle,
                        &filter(roll.properties.clone()),
                    );
                }
                ItemPropertyDto {
                    affix_id: id.clone(),
                    name_key: affix.name_key.clone(),
                    modifiers: stat_modifiers_dto(&bundle.modifiers),
                    equipment_bonuses: equipment_bonuses_dto(&bundle.equipment_bonuses),
                    passives: bundle
                        .passives
                        .into_iter()
                        .map(equipment_passive_dto)
                        .collect(),
                }
            })
            .collect()
    }

    pub(super) fn item_identification(&self, item: &ItemInstance) -> ItemIdentificationDto {
        if !self.item_identity_is_known(item) {
            ItemIdentificationDto::Unexamined
        } else if self
            .item_property_knowledge
            .get(&item.id)
            .is_some_and(|k| k.identified)
            || (self.unknown_item_flags(item).is_empty()
                && self.item_unflagged_properties_known(item))
        {
            ItemIdentificationDto::Identified
        } else {
            ItemIdentificationDto::Appraised
        }
    }

    pub(super) fn visible_item_quality(&self, item: &ItemInstance) -> Option<ItemQualityDto> {
        (self.item_identification(item) != ItemIdentificationDto::Unexamined)
            .then_some(item.quality)
    }

    pub(super) fn visible_item_curse(&self, item: &ItemInstance) -> Option<ItemCurseSeverityDto> {
        let known = self.known_item_curse_flags(item);
        if known & 4 != 0 {
            Some(ItemCurseSeverityDto::Permanent)
        } else if known & 2 != 0 {
            Some(ItemCurseSeverityDto::Heavy)
        } else if known & 1 != 0 {
            Some(ItemCurseSeverityDto::Normal)
        } else {
            None
        }
    }

    pub(super) fn visible_item_enchantments(&self, item: &ItemInstance) -> ItemEnchantmentsDto {
        if self.item_identification(item) != ItemIdentificationDto::Unexamined {
            let mut enchantments = item.enchantments;
            let defense = self.known_item_modifiers(item).defense;
            // Display the known instance delta after applying the same body rule as combat.
            enchantments.to_armor = i16::try_from(
                self.body_armor_for_current_form(item, defense + i32::from(enchantments.to_armor))
                    - self.body_armor_for_current_form(item, defense),
            )
            .expect("body armor reduction cannot increase the enchantment magnitude");
            enchantments
        } else {
            Default::default()
        }
    }

    pub(super) fn known_item_blessed(&self, item: &ItemInstance) -> bool {
        self.item_property_knowledge
            .get(&item.id)
            .is_some_and(|k| k.known_blessed)
            || self.known_item_flags(item).contains("BLESSED")
    }

    fn visible_item_combat_state(&self, item: &ItemInstance) -> ItemInstance {
        let mut visible = item.clone();
        if self.item_identification(item) != ItemIdentificationDto::Identified {
            let identified = self.item_identification(item) != ItemIdentificationDto::Unexamined;
            if !identified {
                visible.enchantments = Default::default();
                visible.intrinsic_melee_damage_dice = None;
            }
            let flags = self.known_item_flags(item);
            // Flatten only the visible non-base powers; combat helpers add the base themselves.
            visible.intrinsic_properties = Default::default();
            for bundle in self.visible_item_property_bundles(item).into_iter().skip(1) {
                super::ego::merge_affix_properties(&mut visible.intrinsic_properties, &bundle);
            }
            visible.intrinsic_weapon_traits = item
                .intrinsic_weapon_traits
                .iter()
                .chain(
                    item.rolled_affixes
                        .iter()
                        .flat_map(|roll| &roll.weapon_traits),
                )
                .filter(|trait_| flags.contains(lore::weapon_trait_flag(**trait_)))
                .copied()
                .collect();
            if self
                .item_property_knowledge
                .get(&item.id)
                .is_some_and(|k| k.known_blessed)
            {
                visible
                    .intrinsic_weapon_traits
                    .insert(rfb_protocol::WeaponTraitDto::Blessed);
            }
            if identified {
                visible.intrinsic_melee_damage_dice = item.melee_damage_dice();
            }
            visible.affix_ids.clear();
            visible.rolled_affixes.clear();
        }
        visible
    }

    pub(super) fn visible_item_melee_profile(
        &self,
        item: &ItemInstance,
    ) -> Option<AttackProfileDto> {
        (self.item_base_properties_known(item)
            || self.item_identification(item) != ItemIdentificationDto::Unexamined)
            .then(|| self.item_melee_profile(&self.visible_item_combat_state(item)))
            .flatten()
    }

    /// Resistance sources use the same instance/shared flag knowledge as other powers.
    pub(super) fn visible_item_resistances(&self, item: &ItemInstance) -> Vec<ResistanceDto> {
        let mut profile = ResistanceProfile::default();
        for source in self.visible_item_resistance_sources(item) {
            let damage_type = DamageType::from(source.damage_type);
            let level = ResistanceLevel::from(source.level);
            let current = profile.level(damage_type);
            if resistance_rank(level) > resistance_rank(current) {
                profile.set(damage_type, level);
            }
        }
        profile.to_dtos()
    }

    pub(super) fn visible_item_resistance_sources(
        &self,
        item: &ItemInstance,
    ) -> Vec<ResistanceDto> {
        self.visible_item_property_bundles(item)
            .into_iter()
            .flat_map(|bundle| bundle.resistances)
            .map(|(damage_type, level)| ResistanceDto {
                damage_type: DamageType::from(damage_type).into(),
                level: ResistanceLevel::from(level).into(),
            })
            .collect()
    }

    pub(super) fn visible_item_status_immunities(&self, item: &ItemInstance) -> Vec<String> {
        self.visible_item_property_bundles(item)
            .into_iter()
            .flat_map(|bundle| bundle.status_immunities)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn visible_item_offense(
        &self,
        item: &ItemInstance,
    ) -> (BTreeMap<SlayTarget, SlayLevel>, BTreeSet<WeaponBrand>) {
        let mut slays = BTreeMap::new();
        let mut brands = BTreeSet::new();
        for bundle in self.visible_item_property_bundles(item) {
            for (target, level) in bundle.slays {
                let current = slays.entry(target).or_insert(level);
                if level > *current {
                    *current = level;
                }
            }
            brands.extend(bundle.brands);
        }
        (slays, brands)
    }

    pub(super) fn visible_item_slays(&self, item: &ItemInstance) -> Vec<SlayDto> {
        self.visible_item_offense(item)
            .0
            .into_iter()
            .map(|(target, level)| SlayDto {
                target: slay_target_dto(target),
                level: slay_level_dto(level),
            })
            .collect()
    }

    pub(super) fn visible_item_brands(&self, item: &ItemInstance) -> Vec<WeaponBrandDto> {
        self.visible_item_offense(item)
            .1
            .into_iter()
            .map(weapon_brand_dto)
            .collect()
    }

    pub(super) fn visible_item_projectile_profile(
        &self,
        item: &ItemInstance,
    ) -> Option<ProjectileProfileDto> {
        (self.item_base_properties_known(item)
            || self.item_identification(item) != ItemIdentificationDto::Unexamined)
            .then(|| self.item_projectile_profile(&self.visible_item_combat_state(item)))
            .flatten()
    }

    pub(super) fn visible_item_throw_profile(
        &self,
        item: &ItemInstance,
    ) -> Option<ThrowProfileDto> {
        (self.item_base_properties_known(item)
            || self.item_identification(item) != ItemIdentificationDto::Unexamined)
            .then(|| self.item_throw_profile(&self.visible_item_combat_state(item)))
            .flatten()
    }
}
