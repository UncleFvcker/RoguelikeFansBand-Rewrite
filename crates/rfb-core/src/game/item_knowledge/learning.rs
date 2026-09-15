// SPDX-License-Identifier: MPL-2.0
//! RFB master a0d92b6378: object1.c::obj_learn_equipped/obj_learn_slay,
//! equip.c::equip_learn_*, resist.c::res_calc_dam, combat.c and cmd2.c.
use super::*;
use rfb_content::AffixPropertyBundleDefinition;

impl Game {
    pub(in crate::game) fn learn_equipped_item(&mut self, item_id: &str) {
        let item = self
            .items
            .iter()
            .find(|item| item.id == item_id)
            .expect("equipped item exists")
            .clone();
        let flags = self.actual_item_flags(&item);
        let priest = self.player_is_priest();
        for flag in flags {
            if rfb_content::RfbPvalFlagDefinition::ALL
                .iter()
                .any(|pval| pval.source_flag() == flag)
                || matches!(
                    flag.as_str(),
                    "LEVITATION"
                        | "REGEN"
                        | "EASY_SPELL"
                        | "DEC_MANA"
                        | "AURA_FIRE"
                        | "AURA_ELEC"
                        | "AURA_COLD"
                        | "AURA_SHARDS"
                        | "LITE"
                        | "DARKNESS"
                        | "SLOW_DIGEST"
                        | "REGEN_MANA"
                )
                || (priest && flag == "BLESSED")
            {
                self.learn_item_flag_on(&item, &flag);
            }
        }
    }

    pub(in crate::game) fn learn_equipment_flag(&mut self, flag: &str) {
        let items = self
            .items
            .iter()
            .filter(|item| {
                matches!(&item.location, ItemLocation::Equipped { slot_id }
                if self.body_slot_type(slot_id) != Some("tool"))
            })
            .cloned()
            .collect::<Vec<_>>();
        for item in items {
            self.learn_item_flag_on(&item, flag);
        }
    }

    pub(in crate::game) fn learn_damage_resistance(&mut self, damage: &DamageOutcome) {
        if damage.requested <= 0 || damage.applied == damage.requested {
            return;
        }
        let Some(element) = resistance_flag_suffix(damage.damage_type) else {
            return;
        };
        let prefix = if damage.applied > damage.requested {
            "VULN"
        } else if damage.applied == 0 {
            "IM"
        } else {
            "RES"
        };
        self.learn_equipment_flag(&format!("{prefix}_{element}"));
    }

    pub(in crate::game) fn learn_status_protection(&mut self, status: &str) {
        let flag = match status {
            STATUS_PARALYSIS | STATUS_SLOW => "FREE_ACT",
            STATUS_FEAR => "RES_FEAR",
            STATUS_BLINDNESS => "RES_BLIND",
            STATUS_CONFUSION => "RES_CONF",
            STATUS_HALLUCINATION => "RES_CHAOS",
            STATUS_POISON => "RES_POIS",
            STATUS_BLEEDING => "RES_SHARDS",
            STATUS_STUN => "RES_SOUND",
            _ => return,
        };
        self.learn_equipment_flag(flag);
    }

    pub(in crate::game) fn learn_attribute_sustain(&mut self, attribute: AttributeKind) {
        self.learn_equipment_flag(match attribute {
            AttributeKind::Strength => "SUST_STR",
            AttributeKind::Intelligence => "SUST_INT",
            AttributeKind::Wisdom => "SUST_WIS",
            AttributeKind::Dexterity => "SUST_DEX",
            AttributeKind::Constitution => "SUST_CON",
            AttributeKind::Charisma => "SUST_CHR",
        });
    }

    pub(in crate::game) fn learn_resisted_status(
        &mut self,
        damage: DamageType,
        requested: u32,
        applied: u32,
    ) {
        if requested == 0 || requested == applied {
            return;
        }
        if let Some(element) = resistance_flag_suffix(damage) {
            let prefix = if applied > requested { "VULN" } else { "RES" };
            self.learn_equipment_flag(&format!("{prefix}_{element}"));
        }
    }

    pub(in crate::game) fn learn_status_resolution(
        &mut self,
        resolution: &AbilityEffectResolutionDto,
        damage: Option<DamageType>,
    ) {
        if let AbilityEffectResolutionDto::ApplyStatus {
            status_kind_id,
            requested_duration_ticks,
            applied_duration_ticks,
            resistance,
            change,
            ..
        } = resolution
        {
            if resistance.is_some()
                && let Some(damage) = damage
            {
                self.learn_resisted_status(
                    damage,
                    *requested_duration_ticks,
                    *applied_duration_ticks,
                );
            } else if *change == AbilityStatusChangeDto::Immune {
                self.learn_status_protection(status_kind_id);
            }
            // A successful level/power saving throw is not an equipment observation.
        }
    }

    pub(in crate::game) fn melee_learning_sources(&self, weapon_id: &str) -> Vec<ItemInstance> {
        self.items
            .iter()
            .filter(|item| {
                let ItemLocation::Equipped { slot_id } = &item.location else {
                    return false;
                };
                let slot = self.body_slot_type(slot_id);
                !matches!(slot, Some("tool" | "launcher"))
                    && (slot != Some("ring") || self.ring_affects_weapon(slot_id, Some(weapon_id)))
                    && (item.id == weapon_id
                        || self
                            .content
                            .item(&item.kind_id)
                            .is_some_and(|kind| kind.melee_profile.is_none()))
            })
            .cloned()
            .collect()
    }

    pub(in crate::game) fn learn_melee_offense(&mut self, weapon_id: &str, target_index: usize) {
        let sources = self.melee_learning_sources(weapon_id);
        self.learn_offense_from_hit(&sources, target_index);
    }

    pub(in crate::game) fn learn_melee_trait(&mut self, weapon_id: &str, flag: &str) {
        for item in self.melee_learning_sources(weapon_id) {
            if item.id == weapon_id
                || (flag == "IMPACT"
                    && matches!(&item.location,
                ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) == Some("gloves")))
            {
                self.learn_item_flag_on(&item, flag);
                if flag == "VORPAL2" {
                    self.learn_item_flag_on(&item, "VORPAL");
                }
            }
        }
    }

    // Splitting a projectile preserves the stack's knowledge; observations made
    // in flight belong to both the fired unit and the remaining original stack.
    pub(in crate::game) fn projectile_learning_sources(
        &self,
        projectile: &ItemInstance,
        stack_id: &str,
    ) -> Vec<ItemInstance> {
        let mut sources = vec![projectile.clone()];
        if projectile.id != stack_id
            && let Some(stack) = self.items.iter().find(|item| item.id == stack_id)
        {
            sources.push(stack.clone());
        }
        sources
    }

    pub(in crate::game) fn learn_projectile_trait(&mut self, sources: &[ItemInstance], flag: &str) {
        for item in sources {
            self.learn_item_flag_on(item, flag);
            if flag == "VORPAL2" {
                self.learn_item_flag_on(item, "VORPAL");
            }
        }
    }

    pub(in crate::game) fn learn_offense_from_hit(
        &mut self,
        sources: &[ItemInstance],
        target_index: usize,
    ) {
        let target = &self.entities[target_index];
        let definition = self
            .actor_runtime_definition(target)
            .expect("hit target exists");
        let mut slays = BTreeMap::new();
        let mut brands = BTreeSet::new();
        for item in sources {
            let bundles =
                std::iter::once(lore::base_properties(
                    self.content.item(&item.kind_id).expect("item kind exists"),
                ))
                .chain(item.affix_ids.iter().map(|id| {
                    lore::affix_properties(self.content.affix(id).expect("affix exists"))
                }))
                .chain(std::iter::once(item.intrinsic_properties.clone()))
                .chain(
                    item.rolled_affixes
                        .iter()
                        .map(|roll| roll.properties.clone()),
                );
            for bundle in bundles {
                for (category, level) in bundle.slays {
                    let current = slays.entry(category).or_insert(level);
                    if level > *current {
                        *current = level;
                    }
                }
                brands.extend(bundle.brands);
            }
        }
        slays.retain(|category, _| slay_target_matches(*category, definition));
        brands.retain(|brand| {
            target.resistances.level(brand_damage_type(*brand)) != ResistanceLevel::Immune
        });
        let flags = lore::property_flags(&AffixPropertyBundleDefinition {
            slays,
            brands,
            ..Default::default()
        });
        for item in sources {
            for flag in &flags {
                self.learn_item_flag_on(item, flag);
            }
        }
    }
}

pub(super) fn resistance_flag_suffix(damage: DamageType) -> Option<&'static str> {
    Some(match damage {
        DamageType::Acid => "ACID",
        DamageType::Electricity => "ELEC",
        DamageType::Fire => "FIRE",
        DamageType::Cold => "COLD",
        DamageType::Poison => "POIS",
        DamageType::Light => "LITE",
        DamageType::Dark => "DARK",
        DamageType::Blindness => "BLIND",
        DamageType::Fear => "FEAR",
        DamageType::Confusion => "CONF",
        DamageType::Nether => "NETHER",
        DamageType::Nexus => "NEXUS",
        DamageType::Sound => "SOUND",
        DamageType::Shards | DamageType::Rocket => "SHARDS",
        DamageType::Chaos => "CHAOS",
        DamageType::Disenchant => "DISEN",
        DamageType::Time => "TIME",
        _ => return None,
    })
}
