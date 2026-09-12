// SPDX-License-Identifier: MPL-2.0

use crate::error::CoreError;
use crate::event::DomainEvent;
use crate::game::ego::{
    materialize_rfb_weapon_ego_with_rng, merge_affix_properties,
    roll_and_materialize_rfb_ego_from_affixes_with_rng,
};
use crate::game::gold::MAX_PLAYER_GOLD;
use crate::game::inventory::{
    ItemEnchantmentRequest, ItemIdentificationRequest, RemoveEquippedCursesRequest,
};
use crate::game::loot::GeneratedItemDraft;
use crate::game::projectile_geometry::{projectile_path_between, rfb_distance};
use crate::game::terrain::TerrainChangeSource;
use crate::game::visibility::has_line_of_sight;
use crate::game::{Game, device_recharge_resolved_event, weapon_brand_dto};
use crate::resistance::DamageType;
use crate::rng::rfb_m_bonus;
use crate::state::{ItemInstance, ItemLocation, RolledAffixState};
use rfb_content::{
    AbilityDefinition, AbilityEffectDefinition, ActorResistanceLevel,
    AffixPropertyBundleDefinition, ItemDestructionElement,
};
use rfb_protocol::{
    AbilityEffectResolutionDto, AbilityEffectsResolutionDto, ItemCurseRemovalResolutionDto,
    ItemCurseSeverityDto, ItemEnchantmentComponentResolutionDto, ItemEnchantmentsDto,
    ItemOriginKindDto, ItemQualityDto, Position, TargetSelection, VirtueKindDto,
};
use std::collections::BTreeSet;

const DEATH_POISON_BRANDING_ABILITY_ID: &str = "demo.ability.death-poison-branding";

const DEATH_VAMPIRIC_BRANDING_ABILITY_ID: &str = "demo.ability.death-vampiric-branding";

const CRUSADE_HOLY_BLADE_ABILITY_ID: &str = "demo.ability.crusade-holy-blade";

impl Game {
    #[doc(hidden)]
    pub fn debug_prepare_craft_e2e(&mut self) -> Result<(), CoreError> {
        self.entities.clear();
        self.items
            .retain(|item| !matches!(item.location, ItemLocation::CarriedBy { .. }));
        let experience = self
            .experience_required_for_level(50)
            .saturating_sub(self.progress.experience);
        self.apply_player_experience(experience, &mut Vec::new());
        let intelligence = self.progress.attribute_potentials.intelligence.min(118);
        self.progress.attributes.intelligence = intelligence;
        self.progress.maximum_attributes.intelligence = intelligence;
        self.refresh_player_resource_maxima();
        for slug in [
            "grade-holders-book",
            "note-of-acting-master",
            "spiritual-enlightenment",
            "dagger",
        ] {
            self.debug_add_generated_inventory_item(
                &format!("e2e.craft.{slug}"),
                &format!("demo.item.{slug}"),
                50,
            )?;
        }
        for id in ["elemental-brand", "magic-armor", "enchantment", "mundanity"] {
            let ability = format!("demo.ability.craft-{id}");
            let book_id = self
                .items
                .iter()
                .find(|item| {
                    self.content
                        .item(&item.kind_id)
                        .and_then(|definition| definition.ability_book_id.as_ref())
                        .and_then(|id| self.content.ability_book(id))
                        .is_some_and(|book| book.ability_ids.contains(&ability))
                })
                .expect("Craft fixture retains its formal books")
                .id
                .clone();
            self.study_player_ability(&book_id, &ability)
                .expect("level 50 Craft fixture can study four spells");
            self.ability_progress.get_mut(&ability).unwrap().proficiency =
                crate::game::player_abilities::SPELL_EXP_MASTER;
        }
        self.identify_item_instance("e2e.craft.dagger", ItemIdentificationRequest::new(true));
        for pool in self.resources.values_mut() {
            pool.current = pool.maximum;
        }
        self.debug_set_ability_casts_succeed(true);
        Ok(())
    }

    pub(in crate::game) fn craft_ability_item_targets(
        &self,
        ability: &AbilityDefinition,
    ) -> Option<Vec<rfb_protocol::AbilityItemTargetDto>> {
        use AbilityEffectDefinition as E;
        if matches!(ability.effect, E::Mundanity) {
            return Some(self.mundanity_item_targets(None));
        }
        if !matches!(
            ability.effect,
            E::CraftEnchant { .. } | E::CraftItem | E::PolishShield | E::Mundanity | E::BlessWeapon
        ) {
            return None;
        }
        Some(
            self.items
                .iter()
                .filter_map(|item| {
                    let (target, confirmation_key) = match ability.effect {
                        E::CraftItem => (
                            TargetSelection::CraftingItem {
                                item_id: item.id.clone(),
                                quantity: item.quantity,
                            },
                            (item.quantity > 30)
                                .then(|| "item-crafting-quantity-confirm".to_owned()),
                        ),
                        _ => (
                            TargetSelection::Item {
                                item_id: item.id.clone(),
                            },
                            None,
                        ),
                    };
                    self.craft_ability_item_target(ability, &target)
                        .map(|item_id| rfb_protocol::AbilityItemTargetDto {
                            item_id,
                            target,
                            confirmation_key,
                        })
                })
                .collect(),
        )
    }

    pub(in crate::game) fn craft_ability_item_target(
        &self,
        ability: &AbilityDefinition,
        target: &TargetSelection,
    ) -> Option<String> {
        use AbilityEffectDefinition as E;
        if matches!(ability.effect, E::Mundanity) {
            return self.mundanity_target(None, target);
        }
        let (id, quantity) = match target {
            TargetSelection::Item { item_id } => (item_id, None),
            TargetSelection::CraftingItem { item_id, quantity }
                if matches!(ability.effect, E::CraftItem) =>
            {
                (item_id, Some(*quantity))
            }
            _ => return None,
        };
        let item = self.items.iter().find(|item| {
            item.id == *id
                && item.quantity > 0
                && (matches!(
                    item.location,
                    ItemLocation::Inventory | ItemLocation::Equipped { .. }
                ) || item.location == ItemLocation::Ground(self.player.position))
        })?;
        let definition = self.content.item(&item.kind_id)?;
        let valid = match ability.effect {
            E::BlessWeapon => definition
                .rfb_base_kind
                .is_some_and(|base| (19..=23).contains(&base.tval)),
            E::CraftEnchant { .. } => {
                definition.rfb_base_kind.is_some_and(|base| {
                    matches!(base.tval, 16..=23 | 30..=38) && (base.tval, base.sval) != (23, 32)
                }) && !self.item_resists_enchantment(item)
            }
            E::PolishShield => definition.rfb_base_kind.is_some_and(|base| base.tval == 34),
            E::CraftItem => {
                self.item_is_valid_crafting_target(None, id)
                    && quantity.is_none_or(|q| q == item.quantity)
                    && (item.quantity <= 30 || quantity == Some(item.quantity))
            }
            _ => false,
        };
        valid.then(|| id.clone())
    }

    pub(super) fn resolve_player_craft_item_effect(
        &mut self,
        ability: &AbilityDefinition,
        item_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> Result<(), CoreError> {
        let succeeded = match ability.effect {
            AbilityEffectDefinition::BlessWeapon => self.bless_weapon(item_id),
            AbilityEffectDefinition::CraftItem => self.craft_item(item_id).is_some(),
            AbilityEffectDefinition::Mundanity => self.mundanify_item(item_id),
            AbilityEffectDefinition::CraftEnchant {
                maximum, increment, ..
            } => self.craft_enchant_item(item_id, maximum, increment),
            AbilityEffectDefinition::PolishShield => self.polish_shield(item_id),
            _ => unreachable!("craft item executor requires an item magic effect"),
        };
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::ItemMagic {
                    effect_index: 0,
                    item_id: item_id.to_owned(),
                    succeeded,
                }],
            },
            trace: None,
        });
        Ok(())
    }

    fn bless_weapon(&mut self, item_id: &str) -> bool {
        let index = self
            .items
            .iter()
            .position(|item| item.id == item_id)
            .unwrap();
        let item = &self.items[index];
        // RFB master a0d92b6378, spells3.c::bless_weapon: clear curses before
        // checking an existing blessing. A resisted blessing still spends the power.
        if let Some(curse) = item.curse {
            let heavy = curse == ItemCurseSeverityDto::Heavy
                || item.intrinsic_properties.rfb_heavy_curse
                || item
                    .rolled_affixes
                    .iter()
                    .any(|roll| roll.properties.rfb_heavy_curse);
            if (heavy && self.rng.bounded(100) < 32) || curse == ItemCurseSeverityDto::Permanent {
                return false;
            }
            crate::game::inventory::clear_item_curse(&mut self.items[index]);
            let knowledge = self
                .item_property_knowledge
                .entry(item_id.to_owned())
                .or_default();
            knowledge.discovered = true;
            knowledge.appraised = true;
            knowledge.feeling = None;
        }
        let item = &self.items[index];
        if self.item_has_weapon_trait(item, rfb_protocol::WeaponTraitDto::Blessed) {
            return true;
        }
        let object = crate::game::item_value::instance::value_object(&self.content, item)
            .expect("blessable formal weapon retains its authoritative properties");
        let mut chance = if object.flags.contains("KILL_GOOD") {
            5
        } else if object.flags.contains("SLAY_GOOD") {
            3
        } else {
            1
        };
        if object.flags.contains("SLAY_EVIL") || object.flags.contains("KILL_EVIL") {
            chance = 1;
        }
        // Source artifact indices, read from authoritative artifact metadata.
        if matches!(object.fixed_artifact, 139 | 334) {
            chance = 10;
        }
        if crate::game::ego::one_in(&mut self.rng, chance) {
            let item = &mut self.items[index];
            item.intrinsic_weapon_traits
                .insert(rfb_protocol::WeaponTraitDto::Blessed);
            item.discount_percent = 99;
            let knowledge = self
                .item_property_knowledge
                .entry(item_id.to_owned())
                .or_default();
            knowledge.discovered = true;
            knowledge.known_blessed = true;
            return true;
        }
        let item = &mut self.items[index];
        for (total, delta) in [
            (object.to_h, &mut item.enchantments.to_hit),
            (object.to_d, &mut item.enchantments.to_damage),
            (object.to_a, &mut item.enchantments.to_armor),
        ] {
            if total > 0 {
                *delta -= 1;
            }
            if total - 1 > 5 && self.rng.bounded(100) < 33 {
                *delta -= 1;
            }
        }
        false
    }

    fn craft_enchant_item(&mut self, item_id: &str, maximum: u16, increment: u16) -> bool {
        let index = self
            .items
            .iter()
            .position(|item| item.id == item_id)
            .unwrap();
        let item = &self.items[index];
        let weapon = self
            .content
            .item(&item.kind_id)
            .unwrap()
            .rfb_base_kind
            .unwrap()
            .tval
            < 30;
        let before = self.item_total_enchantments(item);
        let change = |value: i16| (maximum as i16 - value).clamp(0, increment as i16);
        let delta = ItemEnchantmentsDto {
            to_hit: if weapon { change(before.to_hit) } else { 0 },
            to_damage: if weapon { change(before.to_damage) } else { 0 },
            to_armor: if weapon { 0 } else { change(before.to_armor) },
        };
        if delta.is_empty() {
            if self.rng.bounded(3) == 0 && self.virtue_current(VirtueKindDto::Enchantment) < 100 {
                self.add_virtue(VirtueKindDto::Enchantment, -1);
            }
            return false;
        }
        let plain = item.affix_ids.is_empty() && !item.is_artifact(&self.content);
        let item = &mut self.items[index];
        item.enchantments.to_hit += delta.to_hit;
        item.enchantments.to_damage += delta.to_damage;
        item.enchantments.to_armor += delta.to_armor;
        for (before, delta) in [
            (before.to_hit, delta.to_hit),
            (before.to_damage, delta.to_damage),
            (before.to_armor, delta.to_armor),
        ] {
            if delta > 0
                && before + delta >= 0
                && item.curse == Some(ItemCurseSeverityDto::Normal)
                && self.rng.bounded(100) < 25
            {
                item.curse = None;
            }
        }
        if plain {
            item.discount_percent = 99;
        }
        self.add_virtue(VirtueKindDto::Enchantment, 1);
        true
    }

    fn polish_shield(&mut self, item_id: &str) -> bool {
        let index = self
            .items
            .iter()
            .position(|item| item.id == item_id)
            .unwrap();
        let item = &self.items[index];
        if item.is_artifact(&self.content)
            || !item.affix_ids.is_empty()
            || item.curse.is_some()
            || self
                .content
                .item(&item.kind_id)
                .unwrap()
                .rfb_base_kind
                .unwrap()
                .sval
                == 10
        {
            self.add_virtue(VirtueKindDto::Enchantment, -2);
            return false;
        }
        let item = &mut self.items[index];
        item.affix_ids
            .push("rfb-legacy.affix.reflection".to_owned());
        item.quality = ItemQualityDto::Exceptional;
        item.discount_percent = 99;
        item.origin_kind = Some(ItemOriginKindDto::PlayerMade);
        let attempts = 4 + self.rng.bounded(3) as u16;
        self.enchant_item_instance(item_id, ItemEnchantmentRequest::new(0, 0, attempts));
        self.add_virtue(VirtueKindDto::Enchantment, 2);
        true
    }

    pub(super) fn resolve_player_remove_equipped_curses_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::RemoveEquippedCurses { include_heavy } = ability.effect else {
            unreachable!("curse removal executor requires a curse removal effect");
        };
        let outcome = self.remove_equipped_curses(RemoveEquippedCursesRequest::new(include_heavy));
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::RemoveEquippedCurses {
                    effect_index: 0,
                    resolution: ItemCurseRemovalResolutionDto {
                        include_heavy: outcome.include_heavy,
                        removed_item_ids: outcome.removed_item_ids,
                        retained_permanent_item_ids: outcome.retained_permanent_item_ids,
                    },
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_refuel_equipped_light_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::RefuelEquippedLight {
            maximum_fraction_divisor,
        } = ability.effect
        else {
            unreachable!("light refuel executor requires a light refuel effect");
        };
        let mut item_id = None;
        let mut before = 0;
        let mut after = 0;
        if let Some(item) = self.items.iter_mut().find(|item| {
            matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == "light")
                && item.fuel.is_some_and(|fuel| {
                    matches!(
                        fuel.kind,
                        rfb_protocol::ItemFuelKindDto::Torch
                            | rfb_protocol::ItemFuelKindDto::Lantern
                    )
                })
        }) {
            item_id = Some(item.id.clone());
            let fuel = item.fuel.as_mut().expect("selected light must retain fuel");
            before = fuel.current;
            fuel.current = fuel
                .current
                .saturating_add(fuel.maximum / maximum_fraction_divisor)
                .min(fuel.maximum);
            after = fuel.current;
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::RefuelEquippedLight {
                    effect_index: 0,
                    item_id,
                    before,
                    after,
                }],
            },
            trace: None,
        });
    }

    pub(in crate::game) fn resolve_player_identify_item_effect(
        &mut self,
        ability: &AbilityDefinition,
        item_id: &str,
        events: &mut Vec<DomainEvent>,
    ) {
        let (full_identify_power, full_identify_roll_sides) = match ability.effect {
            AbilityEffectDefinition::IdentifyItem {
                full_identify_power,
                full_identify_roll_sides,
            } => (full_identify_power, full_identify_roll_sides),
            AbilityEffectDefinition::IdentifyOrMassIdentify { mass: false, .. } => (0, 0),
            _ => unreachable!("item identification executor requires an identify item effect"),
        };
        let roll = if full_identify_roll_sides == 0 {
            0
        } else {
            u16::try_from(self.rng.bounded(u64::from(full_identify_roll_sides)) + 1)
                .expect("validated identify roll must fit u16")
        };
        let full = full_identify_roll_sides > 0 && roll <= full_identify_power;
        let identification =
            self.identify_item_instance(item_id, ItemIdentificationRequest::new(full));
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::IdentifyItem {
                    effect_index: 0,
                    item_id: identification.item_id,
                    item_kind_id: identification.item_kind_id,
                    full_identify_power,
                    full_identify_roll_sides,
                    roll,
                    full,
                    changed: identification.changed,
                }],
            },
            trace: None,
        });
    }

    pub(in crate::game) fn resolve_player_mass_identify_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::IdentifyOrMassIdentify { mass: true, .. } = ability.effect
        else {
            unreachable!("mass identification executor requires the upgraded identify effect");
        };
        let mut item_ids = self
            .items
            .iter()
            .filter(|item| {
                item.quantity > 0
                    && matches!(
                        item.location,
                        ItemLocation::Inventory | ItemLocation::Equipped { .. }
                    )
            })
            .map(|item| item.id.clone())
            .collect::<Vec<_>>();
        item_ids.sort();
        let effects = item_ids
            .into_iter()
            .map(|item_id| {
                let identification =
                    self.identify_item_instance(&item_id, ItemIdentificationRequest::new(false));
                AbilityEffectResolutionDto::IdentifyItem {
                    effect_index: 0,
                    item_id: identification.item_id,
                    item_kind_id: identification.item_kind_id,
                    full_identify_power: 0,
                    full_identify_roll_sides: 0,
                    roll: 0,
                    full: false,
                    changed: identification.changed,
                }
            })
            .collect();
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects,
            },
            trace: None,
        });
    }

    pub(super) fn item_is_brandable_weapon(&self, item: &ItemInstance) -> bool {
        (matches!(
            &item.location,
            ItemLocation::Inventory | ItemLocation::Equipped { .. }
        ) || item.location == ItemLocation::Ground(self.player.position))
            && !item.is_artifact(&self.content)
            && item.affix_ids.is_empty()
            && item.rolled_affixes.is_empty()
            && self.content.item(&item.kind_id).is_some_and(|definition| {
                definition.melee_profile.is_some()
                    && !self.item_resists_enchantment(item)
                    && !definition.tags.iter().any(|tag| tag == "unbrandable")
            })
    }

    pub(super) fn item_can_receive_corrosion_protection(&self, item: &ItemInstance) -> bool {
        (matches!(
            &item.location,
            ItemLocation::Inventory | ItemLocation::Equipped { .. }
        ) || item.location == ItemLocation::Ground(self.player.position))
            && self
                .content
                .item(&item.kind_id)
                .is_some_and(|definition| definition.tags.iter().any(|tag| tag == "armor"))
    }

    pub(super) fn resolve_player_corrosion_protection_effect(
        &mut self,
        ability: &AbilityDefinition,
        item_id: &str,
        events: &mut Vec<DomainEvent>,
    ) {
        debug_assert!(matches!(
            ability.effect,
            AbilityEffectDefinition::ProtectFromCorrosion
        ));
        let item = self
            .items
            .iter_mut()
            .find(|item| item.id == item_id)
            .expect("planned corrosion protection target must remain available");
        let item_kind_id = item.kind_id.clone();
        let already_protected = !item
            .permanent_destruction_immunities
            .insert(ItemDestructionElement::Acid);
        let defense_before = item.enchantments.to_armor;
        if item.curse.is_none() && item.enchantments.to_armor < 0 {
            item.enchantments.to_armor = 0;
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::ProtectFromCorrosion {
                    effect_index: 0,
                    item_id: item_id.to_owned(),
                    item_kind_id,
                    already_protected,
                    defense_before,
                    defense_after: item.enchantments.to_armor,
                }],
            },
            trace: None,
        });
    }

    pub(in crate::game) fn resolve_player_brand_weapon_effect(
        &mut self,
        ability: &AbilityDefinition,
        item_id: &str,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::BrandWeapon {
            affix_id,
            brand,
            resistance,
        } = &ability.effect
        else {
            unreachable!("weapon branding executor requires a weapon branding effect");
        };
        let item_index = self
            .items
            .iter()
            .position(|item| item.id == item_id)
            .expect("planned branding target must remain available");
        let item_kind_id = self.items[item_index].kind_id.clone();
        let mut properties = AffixPropertyBundleDefinition::default();
        if let Some(brand) = brand {
            properties.brands.insert(*brand);
        }
        if let Some(resistance) = resistance {
            properties
                .resistances
                .insert(*resistance, ActorResistanceLevel::Resistant);
        }
        let affix = self
            .content
            .affix(affix_id)
            .expect("validated branding affix must remain available")
            .clone();
        if affix.rfb_ego.is_some() {
            let item_definition = self
                .content
                .item(&item_kind_id)
                .expect("planned branding item kind must remain available")
                .clone();
            let mut materialization = materialize_rfb_weapon_ego_with_rng(
                &mut self.rng,
                &item_definition,
                &affix,
                self.progress.level,
            )
            .expect("planned branding target must accept its RFB weapon ego");
            if properties != AffixPropertyBundleDefinition::default() {
                let rolled = materialization
                    .rolled_affixes
                    .iter_mut()
                    .find(|rolled| rolled.affix_id == *affix_id)
                    .expect("branded RFB ego must record its generated properties");
                merge_affix_properties(&mut rolled.properties, &properties);
            }
            materialization.apply_to(&mut self.items[item_index]);
        } else {
            let item = &mut self.items[item_index];
            item.affix_ids.push(affix_id.clone());
            item.affix_ids.sort();
            if properties != AffixPropertyBundleDefinition::default() {
                item.rolled_affixes.push(RolledAffixState {
                    affix_id: affix_id.clone(),
                    properties,
                    ..RolledAffixState::default()
                });
                item.rolled_affixes
                    .sort_by(|left, right| left.affix_id.cmp(&right.affix_id));
            }
        }
        let item = &mut self.items[item_index];
        if item.quality == ItemQualityDto::Ordinary {
            item.quality = ItemQualityDto::Fine;
        }
        item.origin_kind = Some(ItemOriginKindDto::PlayerMade);
        item.discount_percent = 99;

        let enchantment_attempts = u16::try_from(self.rng.bounded(3) + 4)
            .expect("branding enchantment attempts must fit u16");
        let enchantment = self.enchant_item_instance(
            item_id,
            ItemEnchantmentRequest::new(enchantment_attempts, enchantment_attempts, 0),
        );
        if matches!(
            ability.id.as_str(),
            DEATH_POISON_BRANDING_ABILITY_ID
                | DEATH_VAMPIRIC_BRANDING_ABILITY_ID
                | CRUSADE_HOLY_BLADE_ABILITY_ID
        ) {
            self.add_virtue(VirtueKindDto::Enchantment, 2);
        }
        self.identify_item_instance(item_id, ItemIdentificationRequest::new(true));
        self.clamp_player_hp_to_effective_max();
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::BrandWeapon {
                    effect_index: 0,
                    item_id: item_id.to_owned(),
                    item_kind_id,
                    affix_id: affix_id.clone(),
                    brand: brand.map(weapon_brand_dto),
                    resistance: resistance.map(DamageType::from).map(Into::into),
                    to_hit: ItemEnchantmentComponentResolutionDto {
                        attempts: enchantment.to_hit.attempts,
                        successes: enchantment.to_hit.successes,
                        before: enchantment.to_hit.before,
                        after: enchantment.to_hit.after,
                    },
                    to_damage: ItemEnchantmentComponentResolutionDto {
                        attempts: enchantment.to_damage.attempts,
                        successes: enchantment.to_damage.successes,
                        before: enchantment.to_damage.before,
                        after: enchantment.to_damage.after,
                    },
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_fetch_item_effect(
        &mut self,
        ability: &AbilityDefinition,
        target: TargetSelection,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::FetchItem {
            maximum_weight_tenths_pound,
        } = ability.effect
        else {
            unreachable!("fetch item executor requires a fetch item effect");
        };
        let origin = self.player.position;
        let range = ability.target.range.min(18);
        let positions = match target {
            TargetSelection::Direction { .. } => self
                .projectile_path(&target, range)
                .expect("validated fetch direction")
                .into_iter()
                .take_while(|position| {
                    rfb_distance(origin, *position) <= u32::from(range)
                        && self.terrain_is_projectable(*position)
                })
                .collect::<Vec<_>>(),
            _ => {
                let position = match target {
                    TargetSelection::Position { position } => position,
                    TargetSelection::Entity { entity_id } => {
                        self.entities
                            .iter()
                            .find(|entity| entity.id == entity_id)
                            .expect("validated fetch entity")
                            .position
                    }
                    _ => unreachable!("validated fetch target"),
                };
                let valid = rfb_distance(origin, position) <= u32::from(range)
                    && self
                        .index(position)
                        .is_some_and(|index| !self.vault_cells[index])
                    && (!ability.target.requires_line_of_effect
                        || (has_line_of_sight(self, origin, position)
                            && projectile_path_between(origin, position, range).is_some_and(
                                |path| path.into_iter().all(|at| self.terrain_is_projectable(at)),
                            )));
                if valid { vec![position] } else { Vec::new() }
            }
        };
        let can_drop = self.can_drop_item_at(origin)
            && !self
                .items
                .iter()
                .any(|item| item.location == ItemLocation::Ground(origin))
            && !self.gold_piles.iter().any(|pile| pile.position == origin);
        let candidate = positions.iter().filter(|_| can_drop).find_map(|position| {
            self.items
                .iter()
                .enumerate()
                .filter(|(_, item)| matches!(item.location, ItemLocation::Ground(at) if at == *position))
                .min_by(|left, right| left.1.id.cmp(&right.1.id))
                .map(|(index, item)| (index, item.id.clone(), *position))
        });
        let mut item_id = None;
        let mut from = None;
        let mut moved = false;
        if let Some((index, id, position)) = candidate {
            let weight = u32::from(self.item_instance_weight(&self.items[index]));
            item_id = Some(id);
            from = Some(position);
            if weight <= maximum_weight_tenths_pound {
                self.items[index].location = ItemLocation::Ground(self.player.position);
                changed.insert(position);
                changed.insert(self.player.position);
                moved = true;
            }
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::FetchItem {
                    effect_index: 0,
                    item_id,
                    from,
                    to: self.player.position,
                    moved,
                }],
            },
            trace: None,
        });
    }

    fn roll_rfb_ammunition_magic_power(&mut self) -> i8 {
        let level = self.progress.level.min(127);
        let good_chance = level.saturating_add(10).min(75);
        let great_chance = good_chance.saturating_mul(2).saturating_div(3).min(20);
        if self.rng.bounded(100) < u64::from(good_chance) {
            return if self.rng.bounded(100) < u64::from(great_chance) {
                2
            } else {
                1
            };
        }
        if self.rng.bounded(100) >= u64::from(good_chance.saturating_add(2) / 3) {
            return 0;
        }
        if self.rng.bounded(100) < u64::from(great_chance) {
            return -2;
        }
        if self.rng.bounded(u64::from(level.max(1))).saturating_add(1) > 10 {
            0
        } else {
            -1
        }
    }

    pub(in crate::game) fn apply_rfb_ammunition_magic(&mut self, item: &mut ItemInstance) {
        let level = self.progress.level.min(127);
        let power = self.roll_rfb_ammunition_magic_power();
        item.origin_kind = Some(ItemOriginKindDto::PlayerMade);
        item.discount_percent = 99;
        item.quality = match power {
            1 => ItemQualityDto::Fine,
            2 => ItemQualityDto::Exceptional,
            _ => ItemQualityDto::Ordinary,
        };
        if power == 0 {
            return;
        }

        let primary_to_hit = 1_u16
            .saturating_add(u16::try_from(self.rng.bounded(5)).expect("d5 roll fits u16"))
            .saturating_add(rfb_m_bonus(&mut self.rng, 5, level));
        let primary_to_damage = 1_u16
            .saturating_add(u16::try_from(self.rng.bounded(5)).expect("d5 roll fits u16"))
            .saturating_add(rfb_m_bonus(&mut self.rng, 5, level));
        let extra_to_hit = rfb_m_bonus(&mut self.rng, 10, level).saturating_add(1) / 2;
        let extra_to_damage = rfb_m_bonus(&mut self.rng, 10, level).saturating_add(1) / 2;
        let sign = if power < 0 { -1_i16 } else { 1_i16 };
        item.enchantments.to_hit = sign.saturating_mul(
            i16::try_from(primary_to_hit.saturating_add(if power.abs() > 1 {
                extra_to_hit
            } else {
                0
            }))
            .expect("bounded ammunition enchantment fits i16"),
        );
        item.enchantments.to_damage = sign.saturating_mul(
            i16::try_from(primary_to_damage.saturating_add(if power.abs() > 1 {
                extra_to_damage
            } else {
                0
            }))
            .expect("bounded ammunition enchantment fits i16"),
        );
        if power < 0 {
            item.curse = Some(if power < -1 {
                ItemCurseSeverityDto::Heavy
            } else {
                ItemCurseSeverityDto::Normal
            });
            return;
        }
        if power < 2 {
            return;
        }

        let definition = self
            .content
            .item(&item.kind_id)
            .expect("created ammunition kind must remain defined");
        let materialization = roll_and_materialize_rfb_ego_from_affixes_with_rng(
            self.progress
                .active_mutation_ids
                .contains("rfb.mutation.bad-luck"),
            rfb_protocol::ItemEnchantmentsDto::default(),
            &mut self.rng,
            definition,
            self.content.affix_definitions(),
            level,
            None,
        )
        .expect("created ammunition must have a compatible RFB ego");
        materialization.apply_to(item);
    }

    pub(super) fn resolve_player_create_item_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::CreateItem {
            item_kind_id,
            quantity,
        } = &ability.effect
        else {
            unreachable!("item creation executor requires a create-item effect");
        };
        let draft = GeneratedItemDraft {
            artifact_name: None,
            intrinsic_melee_damage_dice: None,
            intrinsic_weight_tenths_pound: None,
            intrinsic_weapon_traits: Default::default(),
            intrinsic_curse_effects: Default::default(),
            permanent_destruction_immunities: Default::default(),
            damage_dice_override: None,
            kind_id: item_kind_id.clone(),
            quantity: *quantity,
            origin_kind: Some(ItemOriginKindDto::Acquire),
            quality: ItemQualityDto::Ordinary,
            affix_ids: Vec::new(),
            rolled_affixes: Vec::new(),
            intrinsic_properties: Default::default(),
            enchantments: ItemEnchantmentsDto::default(),
            curse: None,
            activation: None,
            charges: None,
            fuel: None,
        };
        let placed = self.drop_generated_item_near(draft, self.player.position)?;
        let (position, destination_item_ids) = placed.unwrap_or((self.player.position, Vec::new()));
        let placed_quantity = if destination_item_ids.is_empty() {
            0
        } else {
            *quantity
        };
        if placed_quantity > 0 {
            changed.insert(position);
        } else {
            events.push(DomainEvent::ItemDestroyed {
                target_kind_id: item_kind_id.clone(),
                quantity: *quantity,
                rule_line: None,
            });
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::CreateItem {
                    effect_index: 0,
                    item_kind_id: item_kind_id.clone(),
                    quantity: placed_quantity,
                    position,
                    destination_item_ids,
                }],
            },
            trace: None,
        });
        Ok(())
    }

    pub(in crate::game) fn resolve_player_create_ammunition_effect(
        &mut self,
        ability: &AbilityDefinition,
        source_item_id: Option<String>,
        source_terrain: Option<(Position, String, String)>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::CreateAmmunition {
            item_kind_ids,
            quantity_minimum,
            quantity_maximum,
            ..
        } = &ability.effect
        else {
            unreachable!("ammunition creation executor requires a create-ammunition effect");
        };
        let maximum_tier = u16::try_from(item_kind_ids.len() - 1)
            .expect("validated ammunition tier count must fit u16");
        let tier = usize::from(rfb_m_bonus(
            &mut self.rng,
            maximum_tier,
            self.progress.level,
        ));
        let item_kind_id = item_kind_ids[tier].clone();
        let quantity = quantity_minimum.saturating_add(
            u32::try_from(
                self.rng
                    .bounded(u64::from(quantity_maximum - quantity_minimum + 1)),
            )
            .expect("validated ammunition quantity must fit u32"),
        );

        let item_id = self.allocate_item_instance_id()?;
        let mut item = ItemInstance {
            previously_worn: false,
            book_counted: false,
            artifact_name: None,
            intrinsic_melee_damage_dice: None,
            intrinsic_weight_tenths_pound: None,
            intrinsic_weapon_traits: Default::default(),
            intrinsic_curse_effects: Default::default(),
            id: item_id.clone(),
            kind_id: item_kind_id.clone(),
            quantity,
            inscription: None,
            origin_actor_kind_id: None,
            origin_kind: None,
            damage_dice_override: None,
            discount_percent: 0,
            quality: ItemQualityDto::Ordinary,
            affix_ids: Vec::new(),
            rolled_affixes: Vec::new(),
            intrinsic_properties: Default::default(),
            enchantments: ItemEnchantmentsDto::default(),
            curse: None,
            permanent_destruction_immunities: Default::default(),
            activation: None,
            charges: None,
            fuel: None,
            device_recovery_progress: 0,
            captured_actor: None,
            location: ItemLocation::Inventory,
        };
        self.apply_rfb_ammunition_magic(&mut item);
        // Artemis creates the same enchanted ammunition without consuming material.
        if source_item_id.is_none() && source_terrain.is_none() {
            item.origin_kind = Some(ItemOriginKindDto::Acquire);
        }

        if let Some(item_id) = source_item_id.as_deref() {
            self.destroy_item(item_id, 1)
                .expect("planned ammunition material must remain destroyable");
        }
        if let Some((position, _, target_terrain_id)) = &source_terrain {
            self.replace_terrain_from_source(
                *position,
                target_terrain_id,
                TerrainChangeSource::Magic,
                events,
                changed,
            );
        }
        let destination_item_ids = if self.inventory_quantity_capacity_for(&item, false) >= quantity
        {
            self.carry_shop_purchase_item(item)
        } else {
            if let Some(position) =
                self.ground_drop_position(self.player.position, item.is_artifact(&self.content))
            {
                item.location = ItemLocation::Ground(position);
                self.items.push(item);
                changed.insert(position);
                vec![item_id]
            } else {
                events.push(DomainEvent::ItemDestroyed {
                    target_kind_id: item.kind_id,
                    quantity: item.quantity,
                    rule_line: None,
                });
                Vec::new()
            }
        };
        self.mark_item_aware(&item_kind_id);
        for destination_id in &destination_item_ids {
            self.identify_item_instance(destination_id, ItemIdentificationRequest::new(true));
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::CreateAmmunition {
                    effect_index: 0,
                    source_item_id,
                    source_position: source_terrain.as_ref().map(|(position, _, _)| *position),
                    item_kind_id,
                    quantity,
                    destination_item_ids,
                }],
            },
            trace: None,
        });
        Ok(())
    }

    pub(super) fn resolve_player_transmute_item_effect(
        &mut self,
        ability: &AbilityDefinition,
        item_id: &str,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::TransmuteItemToGold {
            value_divisor,
            unit_value_cap,
        } = ability.effect
        else {
            unreachable!("item transmutation executor requires a transmute-item-to-gold effect");
        };
        let item = self
            .items
            .iter()
            .find(|item| item.id == item_id)
            .expect("planned transmutation item must remain available")
            .clone();
        let unit_value = self
            .content
            .item(&item.kind_id)
            .expect("planned transmutation item definition must remain available")
            .base_value
            .saturating_div(u32::from(value_divisor))
            .min(unit_value_cap);
        let requested = unit_value.saturating_mul(item.quantity);
        self.destroy_item(item_id, item.quantity)
            .expect("planned transmutation must remain valid");
        let before = self.gold;
        self.gold = self.gold.saturating_add(requested).min(MAX_PLAYER_GOLD);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: Some(item.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::TransmuteItemToGold {
                    effect_index: 0,
                    item_id: item.id,
                    item_kind_id: item.kind_id,
                    quantity: item.quantity,
                    gold_gained: self.gold.saturating_sub(before),
                    gold_balance: self.gold,
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_drain_item_magic_effect(
        &mut self,
        ability: &AbilityDefinition,
        item_id: &str,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::DrainItemMagic {
            base_power,
            level_multiplier,
            level_divisor,
        } = ability.effect
        else {
            unreachable!("magic drain executor requires a drain-item-magic effect");
        };
        let index = self
            .items
            .iter()
            .position(|item| item.id == item_id)
            .expect("planned magic drain item must remain available");
        let item_kind_id = self.items[index].kind_id.clone();
        let artifact = self.items[index].is_artifact(&self.content);
        let difficulty = u32::try_from(
            self.items[index]
                .activation
                .as_ref()
                .expect("planned magic drain item must retain its activation")
                .device_check_difficulty
                .max(0),
        )
        .expect("nonnegative device difficulty fits u32");
        let charges_before = self.items[index]
            .charges
            .expect("planned magic drain item must retain charges")
            .current;
        let drained = difficulty.min(charges_before);
        let power = u32::from(base_power).saturating_add(
            u32::from(self.progress.level).saturating_mul(u32::from(level_multiplier))
                / u32::from(level_divisor),
        );
        let failure_odds = power.saturating_sub(difficulty / 2) / 5;
        let failed = failure_odds == 0 || self.rng.bounded(u64::from(failure_odds)) == 0;
        let mut destroyed = false;
        if failed && !artifact && self.rng.bounded(10) == 0 {
            if self.items[index].quantity == 1 {
                let removed = self.items.remove(index);
                self.item_property_knowledge.remove(&removed.id);
            } else {
                self.items[index].quantity -= 1;
            }
            destroyed = true;
        } else {
            self.decrease_item_charges(index, if failed { charges_before } else { drained });
        }
        let resource_id = self
            .casting_profile()
            .map(|profile| profile.resource_id.clone());
        let resource_before = resource_id
            .as_deref()
            .and_then(|id| self.resources.get(id))
            .map_or(0, |pool| pool.current);
        if !failed
            && let Some(resource_id) = resource_id.as_deref()
            && let Some(pool) = self.resources.get_mut(resource_id)
        {
            pool.current = pool.current.saturating_add(drained).min(pool.maximum);
        }
        let resource_after = resource_id
            .as_deref()
            .and_then(|id| self.resources.get(id))
            .map_or(0, |pool| pool.current);
        let charges_after = if destroyed {
            0
        } else {
            self.items
                .iter()
                .find(|item| item.id == item_id)
                .and_then(|item| item.charges)
                .map_or(0, |charges| charges.current)
        };
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: Some(item_kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::DrainItemMagic {
                    effect_index: 0,
                    item_id: item_id.to_owned(),
                    item_kind_id,
                    charges_before,
                    charges_after,
                    drained: if failed { charges_before } else { drained },
                    destroyed,
                    failed,
                    resource_id,
                    resource_before,
                    resource_after,
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_recharge_effect(
        &mut self,
        ability: &AbilityDefinition,
        item_id: &str,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::RechargeFromPlayer { power } = ability.effect else {
            unreachable!("recharge executor requires a player recharge effect");
        };
        let item_activation = ability.tags.iter().any(|tag| tag == "item-activation");
        let resource_id = if item_activation {
            "demo.resource.mana"
        } else {
            &Self::player_ability_parameters(ability).resource_id
        };
        let available = if item_activation {
            // Classes without MP can activate Athena, but cannot supply energy.
            self.resources
                .get(resource_id)
                .map_or(0, |pool| pool.current)
        } else {
            self.resources
                .get(resource_id)
                .expect("validated recharge resource must remain available")
                .current
        };
        let missing = self
            .items
            .iter()
            .find(|item| item.id == item_id)
            .and_then(|item| item.charges)
            .map(|charges| charges.maximum.saturating_sub(charges.current))
            .expect("preflighted recharge target must retain charge capacity");
        let attempted = u32::from(power).min(available).min(missing);
        if attempted > 0 {
            self.resources
                .get_mut(resource_id)
                .expect("spent recharge resource must be present")
                .current -= attempted;
        }
        let outcome =
            self.recharge_inventory_item_from_player(item_id, attempted, u32::from(power));
        events.push(device_recharge_resolved_event(
            outcome,
            ability.id.clone(),
            item_activation,
            false,
        ));
    }
}
