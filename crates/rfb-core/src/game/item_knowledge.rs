// SPDX-License-Identifier: MPL-2.0

use super::*;

impl Game {
    pub(super) fn visible_item_modifiers(&self, item: &ItemInstance) -> StatModifiersDto {
        if self.item_knowledge_dto(&item.kind_id) != ItemKnowledgeDto::Aware {
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
        if self.item_knowledge_dto(&item.kind_id) != ItemKnowledgeDto::Aware {
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
        for rolled in &item.rolled_affixes {
            if known.is_some_and(|knowledge| knowledge.known_affix_ids.contains(&rolled.affix_id)) {
                merge_equipment_bonuses(&mut bonuses, &rolled.properties.equipment_bonuses);
            }
        }
        equipment_bonuses_dto(&bonuses)
    }

    pub(super) fn visible_item_passives(&self, item: &ItemInstance) -> Vec<EquipmentPassiveDto> {
        if self.item_knowledge_dto(&item.kind_id) != ItemKnowledgeDto::Aware {
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

    pub(super) fn visible_item_melee_profile(
        &self,
        item: &ItemInstance,
    ) -> Option<AttackProfileDto> {
        (self.item_knowledge_dto(&item.kind_id) == ItemKnowledgeDto::Aware)
            .then(|| self.item_melee_profile(item))
            .flatten()
    }

    /// Item resistance tiers visible to the player: the base definition is
    /// gated by kind awareness, affix contributions by per-instance affix
    /// knowledge.
    pub(super) fn visible_item_resistances(&self, item: &ItemInstance) -> Vec<ResistanceDto> {
        let mut profile = ResistanceProfile::default();
        let mut record = |damage_type: DamageType, level: ResistanceLevel| {
            let current = profile.level(damage_type);
            if resistance_rank(level) > resistance_rank(current) {
                profile.set(damage_type, level);
            }
        };
        if self.item_knowledge_dto(&item.kind_id) == ItemKnowledgeDto::Aware
            && let Some(definition) = self.content.item(&item.kind_id)
        {
            for (damage_type, level) in &definition.resistances {
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
        profile.to_dtos()
    }

    pub(super) fn visible_item_status_immunities(&self, item: &ItemInstance) -> Vec<String> {
        let mut immunities = BTreeSet::new();
        if self.item_knowledge_dto(&item.kind_id) == ItemKnowledgeDto::Aware
            && let Some(definition) = self.content.item(&item.kind_id)
        {
            immunities.extend(definition.status_immunities.iter().cloned());
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
        if self.item_knowledge_dto(&item.kind_id) == ItemKnowledgeDto::Aware
            && let Some(definition) = self.content.item(&item.kind_id)
        {
            record(&definition.slays, &definition.brands);
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
        (self.item_knowledge_dto(&item.kind_id) == ItemKnowledgeDto::Aware)
            .then(|| self.item_projectile_profile(item))
            .flatten()
    }

    pub(super) fn visible_item_throw_profile(
        &self,
        item: &ItemInstance,
    ) -> Option<ThrowProfileDto> {
        (self.item_knowledge_dto(&item.kind_id) == ItemKnowledgeDto::Aware)
            .then(|| self.item_throw_profile(item))
            .flatten()
    }
}
