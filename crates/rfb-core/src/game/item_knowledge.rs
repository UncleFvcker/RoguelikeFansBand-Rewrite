// SPDX-License-Identifier: MPL-2.0

use super::*;
use rfb_protocol::ItemFeelingDto;

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
                    )
                })
                .map(|item| item.id.clone())
                .collect::<Vec<_>>();
            for id in ids {
                if let Some(knowledge) = self.item_property_knowledge.get_mut(&id) {
                    if knowledge.identified {
                        continue;
                    }
                    knowledge.appraised = false;
                    knowledge.feeling = None;
                    knowledge.known_affix_ids.clear();
                    knowledge.known_blessed = false;
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
            && (self.item_identification(item) != ItemIdentificationDto::Unexamined
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
        if !(self.player_is_mindcrafter()
            || self.player_is_mage()
            || self.player_is_ranger()
            || self.player_is_priest()
            || self.player_is_warrior_mage())
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
        let frequencies = if self.player_is_mage() {
            [(false, 20_000_u32), (true, 9_000)]
        } else if self.player_is_ranger() {
            [(false, 80_000_u32), (true, 80_000)]
        } else if self.player_is_priest() {
            // RFB master a0d92b6378: priest.c FAST/WEAK, MED/STRONG.
            [(false, 9_000_u32), (true, 20_000)]
        } else if self.player_is_warrior_mage() {
            [(false, 20_000_u32), (true, 20_000)]
        } else {
            [(false, 80_000_u32), (true, 20_000)]
        };
        for (second, frequency) in frequencies {
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
                let strong = self.player_is_ranger()
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
        (self.item_identification(item) == ItemIdentificationDto::Identified)
            .then(|| item.artifact_name.clone())
            .flatten()
    }

    pub(super) fn visible_item_bag_capacity(&self, item: &ItemInstance) -> Option<u16> {
        (self.item_identification(item) == ItemIdentificationDto::Identified)
            .then(|| super::inventory::item_bag_capacity(&self.content, item))
            .flatten()
    }

    pub(super) fn visible_item_modifiers(&self, item: &ItemInstance) -> StatModifiersDto {
        if !self.item_base_properties_known(item) {
            return StatModifiersDto::default();
        }
        let known = self.item_property_knowledge.get(&item.id);
        let mut modifiers = item.affix_ids.iter().fold(
            self.item_base_modifiers(&item.kind_id),
            |total, affix_id| {
                let Some(affix) = known
                    .filter(|knowledge| knowledge.known_affix_ids.contains(affix_id))
                    .and_then(|_| self.content.affix(affix_id))
                else {
                    return total;
                };
                StatModifiersDto {
                    attack: total.attack.saturating_add(affix.modifiers.attack),
                    defense: total.defense.saturating_add(affix.modifiers.defense),
                    max_hp: total.max_hp.saturating_add(affix.modifiers.max_hp),
                    spell_power_bonus: total
                        .spell_power_bonus
                        .saturating_add(affix.modifiers.spell_power_bonus),
                    device_power_bonus: total
                        .device_power_bonus
                        .saturating_add(affix.modifiers.device_power_bonus),
                    strength: total.strength.saturating_add(affix.modifiers.strength),
                    intelligence: total
                        .intelligence
                        .saturating_add(affix.modifiers.intelligence),
                    wisdom: total.wisdom.saturating_add(affix.modifiers.wisdom),
                    dexterity: total.dexterity.saturating_add(affix.modifiers.dexterity),
                    constitution: total
                        .constitution
                        .saturating_add(affix.modifiers.constitution),
                    charisma: total.charisma.saturating_add(affix.modifiers.charisma),
                    speed: total.speed.saturating_add(affix.modifiers.speed),
                }
            },
        );
        if known.is_some_and(|knowledge| knowledge.identified) {
            add_stat_modifiers_dto(&mut modifiers, &item.intrinsic_properties.modifiers);
        }
        for rolled in &item.rolled_affixes {
            if known.is_some_and(|knowledge| knowledge.known_affix_ids.contains(&rolled.affix_id)) {
                add_stat_modifiers_dto(&mut modifiers, &rolled.properties.modifiers);
            }
        }
        modifiers
    }

    pub(super) fn visible_item_equipment_bonuses(
        &self,
        item: &ItemInstance,
    ) -> EquipmentBonusesDto {
        if !self.item_base_properties_known(item) {
            return EquipmentBonusesDto::default();
        }
        let mut bonuses = self
            .content
            .item(&item.kind_id)
            .map_or_else(EquipmentBonuses::default, |definition| {
                definition.equipment_bonuses.clone()
            });
        let known = self.item_property_knowledge.get(&item.id);
        for affix_id in &item.affix_ids {
            if known.is_some_and(|knowledge| knowledge.known_affix_ids.contains(affix_id))
                && let Some(affix) = self.content.affix(affix_id)
            {
                merge_equipment_bonuses(&mut bonuses, &affix.equipment_bonuses);
            }
        }
        if known.is_some_and(|knowledge| knowledge.identified) {
            merge_equipment_bonuses(&mut bonuses, &item.intrinsic_properties.equipment_bonuses);
        }
        for rolled in &item.rolled_affixes {
            if known.is_some_and(|knowledge| knowledge.known_affix_ids.contains(&rolled.affix_id)) {
                merge_equipment_bonuses(&mut bonuses, &rolled.properties.equipment_bonuses);
            }
        }
        equipment_bonuses_dto(&bonuses)
    }

    pub(super) fn visible_item_passives(&self, item: &ItemInstance) -> Vec<EquipmentPassiveDto> {
        if !self.item_base_properties_known(item) {
            return Vec::new();
        }
        let mut passives = self
            .content
            .item(&item.kind_id)
            .map_or_else(BTreeSet::new, |definition| definition.passives.clone());
        let known = self.item_property_knowledge.get(&item.id);
        for affix_id in &item.affix_ids {
            if known.is_some_and(|knowledge| knowledge.known_affix_ids.contains(affix_id))
                && let Some(affix) = self.content.affix(affix_id)
            {
                passives.extend(&affix.passives);
            }
        }
        if known.is_some_and(|knowledge| knowledge.identified) {
            passives.extend(&item.intrinsic_properties.passives);
        }
        for rolled in &item.rolled_affixes {
            if known.is_some_and(|knowledge| knowledge.known_affix_ids.contains(&rolled.affix_id)) {
                passives.extend(&rolled.properties.passives);
            }
        }
        passives.into_iter().map(equipment_passive_dto).collect()
    }

    pub(super) fn known_item_properties(&self, item: &ItemInstance) -> Vec<ItemPropertyDto> {
        self.item_property_knowledge
            .get(&item.id)
            .into_iter()
            .flat_map(|knowledge| &knowledge.known_affix_ids)
            .filter_map(|affix_id| {
                self.content.affix(affix_id).map(|affix| {
                    let mut modifiers = stat_modifiers_dto(&affix.modifiers);
                    let mut equipment_bonuses = affix.equipment_bonuses.clone();
                    let mut passives = affix.passives.clone();
                    if let Some(rolled) = item
                        .rolled_affixes
                        .iter()
                        .find(|rolled| rolled.affix_id == *affix_id)
                    {
                        add_stat_modifiers_dto(&mut modifiers, &rolled.properties.modifiers);
                        merge_equipment_bonuses(
                            &mut equipment_bonuses,
                            &rolled.properties.equipment_bonuses,
                        );
                        passives.extend(&rolled.properties.passives);
                    }
                    ItemPropertyDto {
                        affix_id: affix.id.clone(),
                        name_key: affix.name_key.clone(),
                        modifiers,
                        equipment_bonuses: equipment_bonuses_dto(&equipment_bonuses),
                        passives: passives.into_iter().map(equipment_passive_dto).collect(),
                    }
                })
            })
            .collect()
    }

    pub(super) fn item_identification(&self, item: &ItemInstance) -> ItemIdentificationDto {
        self.item_property_knowledge.get(&item.id).map_or(
            ItemIdentificationDto::Unexamined,
            |knowledge| {
                if knowledge.identified {
                    ItemIdentificationDto::Identified
                } else if knowledge.appraised {
                    ItemIdentificationDto::Appraised
                } else {
                    ItemIdentificationDto::Unexamined
                }
            },
        )
    }

    pub(super) fn visible_item_quality(&self, item: &ItemInstance) -> Option<ItemQualityDto> {
        (self.item_identification(item) != ItemIdentificationDto::Unexamined)
            .then_some(item.quality)
    }

    pub(super) fn visible_item_curse(&self, item: &ItemInstance) -> Option<ItemCurseSeverityDto> {
        (self.item_identification(item) != ItemIdentificationDto::Unexamined)
            .then_some(item.curse)
            .flatten()
    }

    pub(super) fn visible_item_enchantments(&self, item: &ItemInstance) -> ItemEnchantmentsDto {
        if self.item_identification(item) == ItemIdentificationDto::Identified {
            item.enchantments
        } else {
            Default::default()
        }
    }

    pub(super) fn known_item_blessed(&self, item: &ItemInstance) -> bool {
        self.item_property_knowledge
            .get(&item.id)
            .is_some_and(|knowledge| knowledge.known_blessed)
            || (self.item_base_properties_known(item)
                && self.item_has_weapon_trait(
                    &self.visible_item_combat_state(item),
                    rfb_protocol::WeaponTraitDto::Blessed,
                ))
    }

    fn visible_item_combat_state(&self, item: &ItemInstance) -> ItemInstance {
        let mut visible = item.clone();
        if self.item_identification(item) != ItemIdentificationDto::Identified {
            visible.enchantments = Default::default();
            visible.intrinsic_properties = Default::default();
            visible.intrinsic_melee_damage_dice = None;
            visible.intrinsic_weapon_traits.clear();
            let known = self.item_property_knowledge.get(&item.id);
            if known.is_some_and(|knowledge| knowledge.known_blessed) {
                visible
                    .intrinsic_weapon_traits
                    .insert(rfb_protocol::WeaponTraitDto::Blessed);
            }
            visible
                .affix_ids
                .retain(|id| known.is_some_and(|knowledge| knowledge.known_affix_ids.contains(id)));
            visible
                .rolled_affixes
                .retain(|roll| visible.affix_ids.contains(&roll.affix_id));
        }
        visible
    }

    pub(super) fn visible_item_melee_profile(
        &self,
        item: &ItemInstance,
    ) -> Option<AttackProfileDto> {
        (self.item_base_properties_known(item))
            .then(|| self.item_melee_profile(&self.visible_item_combat_state(item)))
            .flatten()
    }

    /// Item resistance tiers visible to the player: the base definition is
    /// gated by kind awareness, affix contributions by per-instance affix
    /// knowledge.
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
        let mut sources = Vec::new();
        let mut record = |damage_type: DamageType, level: ResistanceLevel| {
            sources.push(ResistanceDto {
                damage_type: damage_type.into(),
                level: level.into(),
            });
        };
        if self.item_base_properties_known(item)
            && let Some(definition) = self.content.item(&item.kind_id)
        {
            for (damage_type, level) in &definition.resistances {
                record(
                    DamageType::from(*damage_type),
                    ResistanceLevel::from(*level),
                );
            }
            for (damage_type, level) in item
                .intrinsic_properties
                .resistances
                .iter()
                .filter(|_| self.item_identification(item) == ItemIdentificationDto::Identified)
            {
                record(
                    DamageType::from(*damage_type),
                    ResistanceLevel::from(*level),
                );
            }
        }
        let known = self.item_property_knowledge.get(&item.id);
        for affix_id in &item.affix_ids {
            if known.is_some_and(|knowledge| knowledge.known_affix_ids.contains(affix_id))
                && let Some(affix) = self.content.affix(affix_id)
            {
                for (damage_type, level) in &affix.resistances {
                    record(
                        DamageType::from(*damage_type),
                        ResistanceLevel::from(*level),
                    );
                }
            }
        }
        for rolled in &item.rolled_affixes {
            if known.is_some_and(|knowledge| knowledge.known_affix_ids.contains(&rolled.affix_id)) {
                for (damage_type, level) in &rolled.properties.resistances {
                    record(
                        DamageType::from(*damage_type),
                        ResistanceLevel::from(*level),
                    );
                }
            }
        }
        sources
    }

    pub(super) fn visible_item_status_immunities(&self, item: &ItemInstance) -> Vec<String> {
        let mut immunities = BTreeSet::new();
        if self.item_base_properties_known(item)
            && let Some(definition) = self.content.item(&item.kind_id)
        {
            immunities.extend(definition.status_immunities.iter().cloned());
            if self.item_identification(item) == ItemIdentificationDto::Identified {
                immunities.extend(item.intrinsic_properties.status_immunities.iter().cloned());
            }
        }
        let known = self.item_property_knowledge.get(&item.id);
        for affix_id in &item.affix_ids {
            if known.is_some_and(|knowledge| knowledge.known_affix_ids.contains(affix_id))
                && let Some(affix) = self.content.affix(affix_id)
            {
                immunities.extend(affix.status_immunities.iter().cloned());
            }
        }
        for rolled in &item.rolled_affixes {
            if known.is_some_and(|knowledge| knowledge.known_affix_ids.contains(&rolled.affix_id)) {
                immunities.extend(rolled.properties.status_immunities.iter().cloned());
            }
        }
        immunities.into_iter().collect()
    }

    fn visible_item_offense(
        &self,
        item: &ItemInstance,
    ) -> (BTreeMap<SlayTarget, SlayLevel>, BTreeSet<WeaponBrand>) {
        let mut slays = BTreeMap::new();
        let mut brands = BTreeSet::new();
        let mut record = |source_slays: &BTreeMap<SlayTarget, SlayLevel>,
                          source_brands: &BTreeSet<WeaponBrand>| {
            for (target, level) in source_slays {
                let current = slays.entry(*target).or_insert(*level);
                if *level > *current {
                    *current = *level;
                }
            }
            brands.extend(source_brands);
        };
        if self.item_base_properties_known(item)
            && let Some(definition) = self.content.item(&item.kind_id)
        {
            record(&definition.slays, &definition.brands);
            if self.item_identification(item) == ItemIdentificationDto::Identified {
                record(
                    &item.intrinsic_properties.slays,
                    &item.intrinsic_properties.brands,
                );
            }
        }
        let known = self.item_property_knowledge.get(&item.id);
        for affix_id in &item.affix_ids {
            if known.is_some_and(|knowledge| knowledge.known_affix_ids.contains(affix_id))
                && let Some(affix) = self.content.affix(affix_id)
            {
                record(&affix.slays, &affix.brands);
            }
        }
        for rolled in &item.rolled_affixes {
            if known.is_some_and(|knowledge| knowledge.known_affix_ids.contains(&rolled.affix_id)) {
                record(&rolled.properties.slays, &rolled.properties.brands);
            }
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
        (self.item_base_properties_known(item))
            .then(|| self.item_projectile_profile(&self.visible_item_combat_state(item)))
            .flatten()
    }

    pub(super) fn visible_item_throw_profile(
        &self,
        item: &ItemInstance,
    ) -> Option<ThrowProfileDto> {
        (self.item_base_properties_known(item))
            .then(|| self.item_throw_profile(&self.visible_item_combat_state(item)))
            .flatten()
    }
}
