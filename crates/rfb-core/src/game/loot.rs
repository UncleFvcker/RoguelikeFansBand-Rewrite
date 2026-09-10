// SPDX-License-Identifier: MPL-2.0
// Item generation, carried loot, and death drops.

use rfb_content::{
    AffixPropertyBundleDefinition, MonsterDropKindDefinition, affix_is_compatible_with_item,
};
use rfb_protocol::{
    ItemActivationDto, ItemChargesDto, ItemCurseSeverityDto, ItemEnchantmentsDto,
    ItemOriginKindDto, ItemQualityDto, Position,
};

use super::ego::{
    EgoMaterialization, materialize_ego_with_rng, materialize_rfb_harp_intrinsic_with_rng,
    merge_affix_properties, roll_and_materialize_rfb_ego_from_affixes_with_rng,
};
use super::mutations::LuckBias;
use super::{Game, initial_item_curse, item_quality_dto};
use crate::{
    CoreError,
    rng::RfbRng,
    save::initial_item_fuel,
    state::{Actor, GoldPile, ItemInstance, ItemLocation, RolledAffixState},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LootContext {
    pub(super) table_id: String,
    pub(super) floor_id: String,
    /// Source object_level, which may differ from the floor's dungeon depth.
    pub(super) depth: u16,
    pub(super) source: LootSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ItemGenerationMode {
    Ordinary,
    Good,
    /// AM_GREAT without AM_GOOD, present on six original monster definitions.
    GreatOnly,
    Great,
    TailoredGreat,
    Artifact {
        no_fixed_artifact: bool,
    },
}

impl ItemGenerationMode {
    const fn minimum_power(self) -> i16 {
        match self {
            Self::Ordinary | Self::GreatOnly => 0,
            Self::Good => 1,
            Self::Great | Self::TailoredGreat => 2,
            Self::Artifact { .. } => 3,
        }
    }
    const fn minimum_quality(self) -> rfb_content::ItemQuality {
        match self {
            Self::Ordinary | Self::GreatOnly => rfb_content::ItemQuality::Ordinary,
            Self::Good => rfb_content::ItemQuality::Fine,
            Self::Great | Self::TailoredGreat | Self::Artifact { .. } => {
                rfb_content::ItemQuality::Exceptional
            }
        }
    }
}

impl From<rfb_content::ItemQuality> for ItemGenerationMode {
    fn from(value: rfb_content::ItemQuality) -> Self {
        match value {
            rfb_content::ItemQuality::Ordinary => Self::Ordinary,
            rfb_content::ItemQuality::Fine => Self::Good,
            rfb_content::ItemQuality::Exceptional => Self::Great,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GeneratedItemDraft {
    pub(super) kind_id: String,
    pub(super) quantity: u32,
    pub(super) origin_kind: Option<ItemOriginKindDto>,
    pub(super) quality: ItemQualityDto,
    pub(super) affix_ids: Vec<String>,
    pub(super) rolled_affixes: Vec<RolledAffixState>,
    pub(super) intrinsic_properties: AffixPropertyBundleDefinition,
    pub(super) enchantments: ItemEnchantmentsDto,
    pub(super) damage_dice_override: Option<u16>,
    pub(super) curse: Option<ItemCurseSeverityDto>,
    pub(super) activation: Option<ItemActivationDto>,
    pub(super) charges: Option<ItemChargesDto>,
    pub(super) fuel: Option<rfb_protocol::ItemFuelDto>,
}

impl GeneratedItemDraft {
    pub(super) fn into_item_instance(self, id: String, location: ItemLocation) -> ItemInstance {
        ItemInstance {
            previously_worn: false,
            artifact_name: None,
            intrinsic_melee_damage_dice: None,
            intrinsic_weight_tenths_pound: None,
            intrinsic_weapon_traits: Default::default(),
            intrinsic_curse_effects: Default::default(),
            id,
            kind_id: self.kind_id,
            quantity: self.quantity,
            inscription: None,
            origin_actor_kind_id: None,
            origin_kind: self.origin_kind,
            damage_dice_override: self.damage_dice_override,
            discount_percent: 0,
            quality: self.quality,
            affix_ids: self.affix_ids,
            rolled_affixes: self.rolled_affixes,
            intrinsic_properties: self.intrinsic_properties,
            enchantments: self.enchantments,
            curse: self.curse,
            permanent_destruction_immunities: Default::default(),
            activation: self.activation,
            charges: self.charges,
            fuel: self.fuel,
            device_recovery_progress: 0,
            captured_actor: None,
            location,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum LootSource {
    MonsterCarried { actor_id: String },
    MonsterDeath { actor_id: String, themed: bool },
    FloorRoom { room_id: String, spawn_id: String },
    Vault { vault_id: String, spawn_id: String },
    ItemUse { item_id: String },
    Rubble { position: Position },
}

impl Game {
    pub(super) fn initialize_carried_loot(&mut self) -> Result<(), CoreError> {
        let floor_id = self.current_floor_id.clone();
        let depth = self.floor_depth(&floor_id);
        let actors = self.entities.clone();
        let generated = self.generate_carried_loot_for_actors(&actors, &floor_id, depth)?;
        self.items.extend(generated);
        Ok(())
    }

    pub(super) fn generate_carried_loot_for_actors(
        &mut self,
        actors: &[Actor],
        floor_id: &str,
        depth: u16,
    ) -> Result<Vec<ItemInstance>, CoreError> {
        let mut carriers = actors
            .iter()
            .filter_map(|actor| {
                self.content
                    .actor(&actor.kind_id)
                    .and_then(|definition| definition.carried_loot_table_id.clone())
                    .map(|table_id| (actor.id.clone(), table_id))
            })
            .collect::<Vec<_>>();
        carriers.sort_by(|left, right| left.0.cmp(&right.0));
        let mut items = Vec::new();
        for (actor_id, table_id) in carriers {
            let generated = self.generate_loot_instances(
                &LootContext {
                    table_id,
                    floor_id: floor_id.to_owned(),
                    depth,
                    source: LootSource::MonsterCarried {
                        actor_id: actor_id.clone(),
                    },
                },
                ItemLocation::CarriedBy { actor_id },
            )?;
            items.extend(generated);
        }
        Ok(items)
    }

    pub(super) fn generate_death_loot(
        &mut self,
        actor: &Actor,
    ) -> Result<(Vec<ItemInstance>, Vec<GoldPile>), CoreError> {
        let actor_definition = self
            .content
            .actor(&actor.kind_id)
            .expect("living actor definition must remain available")
            .clone();
        let table_id = actor_definition.loot_table_id.clone();
        let guardian_reward = self
            .content
            .world(&self.world_id)
            .and_then(|world| {
                world
                    .procedural_floors
                    .iter()
                    .find(|floor| floor.id == self.current_floor_id)
            })
            .and_then(|floor| floor.guardian.as_ref())
            .filter(|guardian| guardian.instance_id == actor.id)
            .map(|guardian| {
                (
                    guardian.reward_loot_table_id.clone(),
                    guardian.reward_artifact_item_kind_id.clone(),
                    guardian.reward_first_realm_book_rank,
                )
            });
        let floor_id = self.current_floor_id.clone();
        let depth = self.floor_depth(&floor_id);
        let mut generated = Vec::new();
        let mut gold = Vec::new();
        if let Some(drop) = actor_definition.death_drop.clone() {
            let unique = actor_definition.tags.iter().any(|tag| tag == "unique");
            let mut count = u32::from(drop.base_rolls);
            for roll in &drop.chance_rolls {
                if (roll.guaranteed_for_unique && unique)
                    || self.rng.bounded(100) < u64::from(roll.percent)
                {
                    count = count.saturating_add(1);
                }
            }
            for dice in &drop.count_dice {
                for _ in 0..dice.dice {
                    count = count.saturating_add(
                        u32::try_from(self.rng.bounded(u64::from(dice.sides)) + 1)
                            .expect("validated monster drop die must fit u32"),
                    );
                }
            }
            if count > 2 && !unique && drop.minimum_quality != rfb_content::ItemQuality::Exceptional
            {
                count = 2 + (count - 2) / 2;
            }
            let object_level = {
                let actor_level = actor_definition.level.min(u32::from(u16::MAX));
                let floor_level = u32::from(depth);
                if actor_level >= floor_level {
                    actor_level
                } else {
                    (actor_level + floor_level) / 2
                }
            };
            for _ in 0..count {
                let drops_gold = match drop.kind {
                    MonsterDropKindDefinition::Gold => true,
                    MonsterDropKindDefinition::Items => false,
                    MonsterDropKindDefinition::ItemsAndGold => self.rng.bounded(100) < 20,
                };
                if drops_gold {
                    gold.push(self.generate_gold_pile(
                        actor.position,
                        u16::try_from(object_level).expect("bounded gold level must fit u16"),
                        true,
                    )?);
                    continue;
                }
                let use_theme = drop.theme_table_id.is_some()
                    && self.rng.bounded(100) < u64::from(drop.theme_chance_percent);
                let table_id = if use_theme {
                    drop.theme_table_id
                        .as_ref()
                        .expect("checked monster theme table must exist")
                } else {
                    drop.item_table_id
                        .as_ref()
                        .expect("validated item drop must define a table")
                };
                generated.extend(self.generate_one_loot_instance(
                    &LootContext {
                        table_id: table_id.clone(),
                        floor_id: floor_id.clone(),
                        depth: u16::try_from(object_level).expect("bounded monster object level"),
                        source: LootSource::MonsterDeath {
                            actor_id: actor.id.clone(),
                            themed: use_theme,
                        },
                    },
                    ItemLocation::Ground(actor.position),
                    if drop.great_only {
                        ItemGenerationMode::GreatOnly
                    } else {
                        drop.minimum_quality.into()
                    },
                )?);
            }
        } else if let Some(table_id) = table_id {
            let context = LootContext {
                table_id,
                floor_id: floor_id.clone(),
                depth,
                source: LootSource::MonsterDeath {
                    actor_id: actor.id.clone(),
                    themed: false,
                },
            };
            if let Some(gold_chance) = actor_definition.gold_drop_chance_percent {
                let table = self
                    .content
                    .loot_table(&context.table_id)
                    .expect("validated actor loot table must remain available");
                let successful_drop = table
                    .roll_chance_percent
                    .is_none_or(|chance| self.rng.bounded(100) < u64::from(chance));
                if successful_drop {
                    if self.rng.bounded(100) < u64::from(gold_chance) {
                        let actor_level = actor_definition.level.min(u32::from(u16::MAX));
                        let floor_level = u32::from(depth);
                        let object_level = if actor_level >= floor_level {
                            actor_level
                        } else {
                            (actor_level + floor_level) / 2
                        };
                        gold.push(self.generate_gold_pile(
                            actor.position,
                            u16::try_from(object_level).expect("bounded gold level must fit u16"),
                            true,
                        )?);
                    } else {
                        generated.extend(self.generate_loot_instances_after_roll_chance(
                            &context,
                            ItemLocation::Ground(actor.position),
                        )?);
                    }
                }
            } else {
                generated.extend(
                    self.generate_loot_instances(&context, ItemLocation::Ground(actor.position))?,
                );
            }
        }
        if let Some((reward_table_id, reward_artifact_kind_id, first_realm_book_rank)) =
            guardian_reward
        {
            let artifact_reward = reward_artifact_kind_id.is_some();
            // The current replacement is high-quality base equipment, but keeps
            // the original replacement artifact's protection against loss on water.
            let reward_position = self
                .ground_drop_position(actor.position, artifact_reward)
                .unwrap_or(actor.position);
            let context = reward_table_id.map(|table_id| LootContext {
                table_id,
                floor_id: floor_id.clone(),
                depth,
                source: LootSource::MonsterDeath {
                    actor_id: actor.id.clone(),
                    themed: false,
                },
            });
            let first_realm_book_kind_id = first_realm_book_rank.and_then(|rank| {
                let realm_id = self
                    .build
                    .as_ref()
                    .and_then(|identity| self.content.build(&identity.build_id))
                    .and_then(|build| build.first_realm_id.as_deref())?;
                self.content
                    .item_definitions()
                    .find(|item| {
                        item.ability_book_id
                            .as_deref()
                            .and_then(|book_id| self.content.ability_book(book_id))
                            .is_some_and(|book| {
                                book.realm_id.as_deref() == Some(realm_id)
                                    && book.rank == Some(rank)
                            })
                    })
                    .map(|item| item.id.clone())
            });
            if let Some(kind_id) = first_realm_book_kind_id {
                let context = context
                    .as_ref()
                    .expect("validated realm-book reward must retain a fallback table");
                let draft = self.fixed_item_draft(context, kind_id);
                generated.push(
                    self.commit_generated_item_draft(draft, ItemLocation::Ground(reward_position))?,
                );
            } else if let Some(kind_id) = reward_artifact_kind_id
                && !self.generated_artifact_ids.contains(&kind_id)
            {
                let context = context
                    .as_ref()
                    .expect("validated artifact guardian reward must retain a fallback table");
                let draft = self.fixed_item_draft(context, kind_id);
                generated.push(
                    self.commit_generated_item_draft(draft, ItemLocation::Ground(reward_position))?,
                );
            } else if let Some(context) = context {
                let mode = if artifact_reward {
                    ItemGenerationMode::Artifact {
                        no_fixed_artifact: false,
                    }
                } else {
                    ItemGenerationMode::Ordinary
                };
                generated.extend(self.generate_loot_instances_internal(
                    &context,
                    ItemLocation::Ground(reward_position),
                    true,
                    None,
                    mode,
                )?);
            }
        }
        Ok((generated, gold))
    }

    pub(super) fn generate_loot_instances(
        &mut self,
        context: &LootContext,
        location: ItemLocation,
    ) -> Result<Vec<ItemInstance>, CoreError> {
        self.generate_loot_instances_internal(
            context,
            location,
            true,
            None,
            ItemGenerationMode::Ordinary,
        )
    }

    fn generate_loot_instances_after_roll_chance(
        &mut self,
        context: &LootContext,
        location: ItemLocation,
    ) -> Result<Vec<ItemInstance>, CoreError> {
        self.generate_loot_instances_internal(
            context,
            location,
            false,
            None,
            ItemGenerationMode::Ordinary,
        )
    }

    fn generate_one_loot_instance(
        &mut self,
        context: &LootContext,
        location: ItemLocation,
        mode: ItemGenerationMode,
    ) -> Result<Vec<ItemInstance>, CoreError> {
        self.next_item_instance_serial
            .checked_add(1)
            .ok_or(CoreError::ItemIdExhausted)?;
        let Some(draft) = self.generate_one_loot_draft(context, mode) else {
            return Ok(Vec::new());
        };
        Ok(vec![self.commit_generated_item_draft(draft, location)?])
    }

    pub(super) fn generate_loot_instances_internal(
        &mut self,
        context: &LootContext,
        location: ItemLocation,
        roll_table_chance: bool,
        roll_count_override: Option<u16>,
        mode: ItemGenerationMode,
    ) -> Result<Vec<ItemInstance>, CoreError> {
        let table = self
            .content
            .loot_table(&context.table_id)
            .expect("validated actor loot table must remain available");
        let maximum_rolls = roll_count_override.map_or_else(
            || {
                table.roll_dice.map_or(u32::from(table.rolls), |dice| {
                    u32::from(table.rolls) + u32::from(dice.dice) * u32::from(dice.sides)
                })
            },
            u32::from,
        );
        self.next_item_instance_serial
            .checked_add(u64::from(maximum_rolls))
            .ok_or(CoreError::ItemIdExhausted)?;
        let drafts = self.generate_loot_drafts_internal(
            context,
            roll_table_chance,
            roll_count_override,
            mode,
        );
        drafts
            .into_iter()
            .map(|draft| self.commit_generated_item_draft(draft, location.clone()))
            .collect()
    }

    pub(super) fn generate_one_loot_draft(
        &mut self,
        context: &LootContext,
        mode: ItemGenerationMode,
    ) -> Option<GeneratedItemDraft> {
        self.generate_loot_drafts_internal(context, false, Some(1), mode)
            .pop()
    }

    fn generate_loot_drafts_internal(
        &mut self,
        context: &LootContext,
        roll_table_chance: bool,
        roll_count_override: Option<u16>,
        mode: ItemGenerationMode,
    ) -> Vec<GeneratedItemDraft> {
        let context_is_valid = !context.floor_id.is_empty()
            && match &context.source {
                LootSource::MonsterCarried { actor_id }
                | LootSource::MonsterDeath { actor_id, .. } => !actor_id.is_empty(),
                LootSource::FloorRoom { room_id, spawn_id } => {
                    context.depth > 0 && !room_id.is_empty() && !spawn_id.is_empty()
                }
                LootSource::Vault { vault_id, spawn_id } => {
                    context.depth > 0 && !vault_id.is_empty() && !spawn_id.is_empty()
                }
                LootSource::ItemUse { item_id } => !item_id.is_empty(),
                LootSource::Rubble { position } => {
                    context.depth > 0 && self.index(*position).is_some()
                }
            };
        debug_assert!(context_is_valid, "validated loot context must remain valid");
        let table = self
            .content
            .loot_table(&context.table_id)
            .expect("validated actor loot table must remain available")
            .clone();
        let minimum_quality = mode.minimum_quality();
        // RFB master a0d92b6378: object2.c::kind_is_tailored uses the birth race.
        // This is a reward preference, not an equipment restriction.
        let tomte_headgear = mode == ItemGenerationMode::TailoredGreat
            && self
                .build
                .as_ref()
                .is_some_and(|build| build.race_id == "rfb-legacy.race.tomte");
        let rfb_generation = table.rfb_ego_policy.is_some();
        let eligible_entries = table
            .entries
            .iter()
            .filter(|entry| {
                entry.min_depth <= context.depth
                    && context.depth <= entry.max_depth
                    && (!tomte_headgear
                        || self.content.item(&entry.item_kind_id).is_some_and(|item| {
                            item.equipment_slot.as_deref() != Some("head")
                                || item.id == "demo.item.knit-cap"
                        }))
                    && (minimum_quality == rfb_content::ItemQuality::Ordinary
                        || self.content.item(&entry.item_kind_id).is_some_and(|item| {
                            item.max_stack == 1
                                && item.equipment_slot.is_some()
                                && entry.quantity == 1
                        }))
            })
            .collect::<Vec<_>>();
        if eligible_entries.is_empty() {
            return Vec::new();
        }
        let entry_weights = eligible_entries
            .iter()
            .map(|entry| entry.weight)
            .collect::<Vec<_>>();
        let quality_weights = table
            .quality_weights
            .iter()
            .map(|entry| entry.weight)
            .collect::<Vec<_>>();
        let mut roll_count = roll_count_override.unwrap_or(table.rolls);
        if roll_count_override.is_none()
            && roll_table_chance
            && table
                .roll_chance_percent
                .is_some_and(|chance| self.rng.bounded(100) >= u64::from(chance))
        {
            roll_count = 0;
        } else if roll_count_override.is_none()
            && let Some(dice) = table.roll_dice
        {
            for _ in 0..dice.dice {
                roll_count = roll_count.saturating_add(
                    u16::try_from(self.rng.bounded(u64::from(dice.sides)) + 1)
                        .expect("validated loot die must fit u16"),
                );
            }
        }
        let mut generated = Vec::with_capacity(usize::from(roll_count));
        for _ in 0..roll_count {
            if (rfb_generation || matches!(mode, ItemGenerationMode::Artifact { .. }))
                && let Some(kind_id) = self.roll_instant_fixed_artifact_kind_id(
                    context,
                    if mode.minimum_power() == 0 { 1_000 } else { 10 },
                )
            {
                generated.push(self.fixed_item_draft(context, kind_id));
                continue;
            }
            let entry_index = self.roll_weighted_index(&entry_weights);
            let entry = eligible_entries[entry_index];
            let staff = self
                .content
                .item(&entry.item_kind_id)
                .is_some_and(|definition| definition.tags.iter().any(|tag| tag == "staff"));
            let jewelry = self
                .content
                .item(&entry.item_kind_id)
                .and_then(|definition| definition.equipment_slot.as_deref())
                .is_some_and(|slot| matches!(slot, "ring" | "amulet"));
            let generation_depth = self.luck_adjusted_item_generation_depth(context.depth, staff);
            let device = self
                .content
                .item(&entry.item_kind_id)
                .is_some_and(|item| item.tags.iter().any(|tag| tag == "device"));
            let rolled_power = match table.quality_policy {
                Some(policy) => {
                    self.roll_rfb_depth_loot_power(policy, generation_depth, jewelry, device, mode)
                }
                None => (match self.roll_loot_quality(
                    &table.quality_weights,
                    &quality_weights,
                    minimum_quality,
                ) {
                    ItemQualityDto::Ordinary => 0,
                    ItemQualityDto::Fine => 1,
                    ItemQualityDto::Exceptional => 2,
                })
                .max(mode.minimum_power()),
            };
            let artifact_rolls = if matches!(
                mode,
                ItemGenerationMode::Artifact {
                    no_fixed_artifact: true
                }
            ) {
                0
            } else if matches!(mode, ItemGenerationMode::Artifact { .. })
                || (rfb_generation
                    && matches!(
                        mode,
                        ItemGenerationMode::Great
                            | ItemGenerationMode::TailoredGreat
                            | ItemGenerationMode::GreatOnly
                    ))
            {
                4
            } else if rfb_generation && rolled_power >= 2 {
                1
            } else {
                0
            };
            let artifact_kind_id = (0..artifact_rolls).find_map(|_| {
                self.roll_fixed_artifact_kind_id(context, Some(&entry.item_kind_id), false)
                    .or_else(|| {
                        (self.player_luck_bias() == LuckBias::Good && self.rng.bounded(77) == 0)
                            .then(|| {
                                self.roll_fixed_artifact_kind_id(
                                    context,
                                    Some(&entry.item_kind_id),
                                    false,
                                )
                            })
                            .flatten()
                    })
            });
            if let Some(kind_id) = artifact_kind_id {
                generated.push(self.fixed_item_draft(context, kind_id));
                continue;
            }
            let mut base_intrinsic_properties =
                self.content.item(&entry.item_kind_id).and_then(|item| {
                    materialize_rfb_harp_intrinsic_with_rng(&mut self.rng, item, generation_depth)
                });
            let preselected_generic_affix_id = (table.rfb_ego_policy
                != Some(rfb_content::LootRfbEgoPolicyDefinition::WeaponDigger))
            .then(|| {
                let eligible_affixes = table
                    .affix_weights
                    .iter()
                    .filter(|affix_weight| {
                        affix_weight.affix_id.as_ref().is_none_or(|affix_id| {
                            self.content.affix(affix_id).is_some_and(|affix| {
                                self.content.item(&entry.item_kind_id).is_some_and(|item| {
                                    affix_is_compatible_with_item(affix, item, generation_depth)
                                })
                            })
                        })
                    })
                    .collect::<Vec<_>>();
                let affix_weights = eligible_affixes
                    .iter()
                    .map(|entry| entry.weight)
                    .collect::<Vec<_>>();
                (!eligible_affixes.is_empty()).then(|| {
                    let affix_index = self.roll_weighted_index(&affix_weights);
                    eligible_affixes[affix_index].affix_id.clone()
                })
            });
            let supports_quality = self.content.item(&entry.item_kind_id).is_some_and(|item| {
                (item.max_stack == 1 && item.equipment_slot.is_some() && entry.quantity == 1)
                    || (table.rfb_ego_policy
                        == Some(rfb_content::LootRfbEgoPolicyDefinition::WeaponDigger)
                        && ((item.rfb_base_kind.is_some() && item.ammunition_profile.is_some())
                            || item.tags.iter().any(|tag| tag == "device")))
            });
            let mut power = if supports_quality { rolled_power } else { 0 };
            if rfb_generation
                && jewelry
                && power == 0
                && matches!(
                    context.source,
                    LootSource::MonsterDeath { themed: true, .. }
                )
            {
                power = 1;
            }
            let rfb_light = table.rfb_ego_policy
                == Some(rfb_content::LootRfbEgoPolicyDefinition::WeaponDigger)
                && self
                    .content
                    .item(&entry.item_kind_id)
                    .and_then(|item| item.rfb_base_kind)
                    .is_some_and(|base| base.tval == 39);
            let mut fuel = initial_item_fuel(&self.content, &entry.item_kind_id);
            if rfb_light && let Some(fuel) = &mut fuel {
                if fuel.current > 0 {
                    fuel.current = 1 + self.rng.bounded(u64::from(fuel.current)) as u16;
                }
                if power == 1 && self.rng.bounded(3) == 0 {
                    power = 2;
                }
            }
            if rfb_generation
                && let Some(properties) = super::ego::roll_container_capacity(
                    &mut self.rng,
                    self.content.item(&entry.item_kind_id).unwrap(),
                    power,
                )
            {
                base_intrinsic_properties = Some(properties);
            }
            if rfb_generation
                && let Some(properties) = super::ego::dragon::materialize(
                    &mut self.rng,
                    self.content.item(&entry.item_kind_id).unwrap(),
                    &mut power,
                    matches!(
                        mode,
                        ItemGenerationMode::Artifact {
                            no_fixed_artifact: true
                        }
                    ),
                )
            {
                merge_affix_properties(
                    base_intrinsic_properties.get_or_insert_with(Default::default),
                    &properties,
                );
            }
            let rfb_armor = table.rfb_ego_policy
                == Some(rfb_content::LootRfbEgoPolicyDefinition::WeaponDigger)
                && self
                    .content
                    .item(&entry.item_kind_id)
                    .and_then(|item| item.rfb_base_kind)
                    .is_some_and(|base| matches!(base.tval, 30..=38));
            let armor_enchantment = if rfb_armor && power != 0 {
                super::ego::roll_rfb_armor_enchantment(&mut self.rng, generation_depth, power)
            } else {
                0
            };
            let base_kind = self
                .content
                .item(&entry.item_kind_id)
                .and_then(|item| item.rfb_base_kind);
            let rfb_weapon =
                rfb_generation && base_kind.is_some_and(|base| matches!(base.tval, 16..=23));
            let mut allow_weapon_ego = !base_kind
                .is_some_and(|base| matches!((base.tval, base.sval), (23, 32 | 34) | (22, 50)));
            let weapon_enchantment = if rfb_weapon && allow_weapon_ego && power != 0 {
                let enchantment = super::ego::roll_rfb_weapon_enchantment(
                    &mut self.rng,
                    self.content.item(&entry.item_kind_id).unwrap(),
                    generation_depth,
                    power,
                );
                allow_weapon_ego = enchantment.is_some();
                enchantment.unwrap_or_default()
            } else {
                ItemEnchantmentsDto::default()
            };
            let rfb_device = table.rfb_ego_policy
                == Some(rfb_content::LootRfbEgoPolicyDefinition::WeaponDigger)
                && self
                    .content
                    .item(&entry.item_kind_id)
                    .is_some_and(|item| item.tags.iter().any(|tag| tag == "device"));
            let rfb_jewelry = jewelry
                && table.rfb_ego_policy
                    == Some(rfb_content::LootRfbEgoPolicyDefinition::WeaponDigger);
            let rfb_materialization = if rfb_jewelry && power != 0 {
                self.content.item(&entry.item_kind_id).and_then(|item| {
                    super::ego::roll_jewelry(
                        &self.content,
                        &mut self.rng,
                        item,
                        generation_depth,
                        power,
                    )
                })
            } else if rfb_device {
                self.content.item(&entry.item_kind_id).and_then(|item| {
                    super::ego::materialize_device(
                        &self.content,
                        &mut self.rng,
                        item,
                        generation_depth,
                        power.abs() >= 2,
                        None,
                    )
                })
            } else {
                (table.rfb_ego_policy
                    == Some(rfb_content::LootRfbEgoPolicyDefinition::WeaponDigger)
                    && (!rfb_weapon || allow_weapon_ego)
                    && !(power < 0
                        && base_kind.is_some_and(|base| matches!(base.tval, 16..=18 | 46)))
                    && power_allows_natural_affix(table.quality_policy, power))
                .then(|| {
                    self.content.item(&entry.item_kind_id).and_then(|item| {
                        roll_and_materialize_rfb_ego_from_affixes_with_rng(
                            weapon_enchantment,
                            &mut self.rng,
                            item,
                            self.content.affix_definitions(),
                            generation_depth,
                            base_intrinsic_properties.as_ref(),
                        )
                    })
                })
                .flatten()
            };
            let mut materialization = rfb_materialization.unwrap_or_else(|| {
                let rolled_affix_id = preselected_generic_affix_id.flatten().flatten();
                let affix_is_required = !table.affix_weights.is_empty()
                    && table
                        .affix_weights
                        .iter()
                        .all(|entry| entry.affix_id.is_some());
                let affix_ids = if affix_is_required
                    || power_allows_natural_affix(table.quality_policy, power)
                {
                    rolled_affix_id.iter().cloned().collect()
                } else {
                    Vec::new()
                };
                materialize_ego_with_rng(
                    &self.content,
                    &mut self.rng,
                    &entry.item_kind_id,
                    affix_ids,
                    |_| generation_depth,
                    generation_depth,
                    power,
                )
            });
            if !materialization.clear_armor_enchantment {
                materialization.enchantment_delta.to_armor += armor_enchantment;
            }
            let launcher_ego = base_kind.is_some_and(|base| base.tval == 19 && base.sval != 70)
                && !materialization.affix_ids.is_empty();
            if !launcher_ego {
                if !materialization.clear_hit_enchantment {
                    materialization.enchantment_delta.to_hit += weapon_enchantment.to_hit;
                }
                if !materialization.clear_damage_enchantment {
                    materialization.enchantment_delta.to_damage += weapon_enchantment.to_damage;
                }
            }
            if materialization.extinguish_fuel
                && let Some(fuel) = &mut fuel
            {
                fuel.current = 0;
            }
            let EgoMaterialization {
                curse_on_finalize,
                kind_id_override,
                affix_ids,
                rolled_affixes,
                intrinsic_properties: ego_intrinsic_properties,
                enchantment_delta,
                ammunition_damage_dice,
                curse,
                activation,
                charges,
                ..
            } = materialization;
            let mut intrinsic_properties = base_intrinsic_properties.unwrap_or_default();
            if rolled_affixes.is_empty()
                && let Some(item) = self.content.item(&entry.item_kind_id).filter(|item| {
                    item.rfb_base_kind
                        .is_some_and(|base| base.tval == 35 && base.sval == 2)
                })
            {
                let pval = 1 + self.rng.bounded(4) as i32;
                super::ego::remember_rfb_pval(
                    &mut intrinsic_properties,
                    [
                        rfb_content::RfbPvalFlagDefinition::Stealth,
                        rfb_content::RfbPvalFlagDefinition::Search,
                    ],
                    pval,
                );
                intrinsic_properties.equipment_bonuses.stealth_skill +=
                    pval - item.equipment_bonuses.stealth_skill;
                intrinsic_properties.equipment_bonuses.search_skill +=
                    5 * pval - item.equipment_bonuses.search_skill;
                intrinsic_properties.equipment_bonuses.perception_skill +=
                    5 * pval - item.equipment_bonuses.perception_skill;
            }
            if let Some(properties) = ego_intrinsic_properties {
                merge_affix_properties(&mut intrinsic_properties, &properties);
            }
            let curse = curse.map_or_else(
                || initial_item_curse(&self.content, &entry.item_kind_id),
                |generated| {
                    Some(
                        initial_item_curse(&self.content, &entry.item_kind_id)
                            .map_or(generated, |initial| initial.max(generated)),
                    )
                },
            );
            let mut draft = GeneratedItemDraft {
                kind_id: kind_id_override.unwrap_or_else(|| entry.item_kind_id.clone()),
                quantity: entry.quantity,
                origin_kind: match &context.source {
                    LootSource::Rubble { .. } => Some(ItemOriginKindDto::Rubble),
                    _ => None,
                },
                quality: power_quality(power),
                affix_ids,
                rolled_affixes,
                intrinsic_properties,
                enchantments: enchantment_delta,
                damage_dice_override: ammunition_damage_dice,
                curse,
                activation,
                charges,
                fuel,
            };
            if rfb_generation {
                let final_curse = curse_on_finalize
                    || (power == -2
                        && !rfb_jewelry
                        && !rfb_device
                        && base_kind.is_none_or(|base| base.tval != 46)
                        && !draft.affix_ids.is_empty());
                if final_curse || (power == -1 && (rfb_weapon || rfb_armor)) {
                    super::ego::curses::finalize_draft(
                        &self.content,
                        &mut self.rng,
                        &mut draft,
                        final_curse,
                    );
                }
            }
            generated.push(draft);
        }
        generated
    }

    pub(super) fn roll_instant_fixed_artifact_kind_id(
        &mut self,
        context: &LootContext,
        one_in: u64,
    ) -> Option<String> {
        if self.rng.bounded(one_in) != 0 {
            return None;
        }
        self.roll_fixed_artifact_kind_id(context, None, true)
    }

    fn fixed_artifact_reference_depth(&self, context: &LootContext) -> u16 {
        self.content
            .world(&self.world_id)
            .and_then(|world| {
                world
                    .procedural_floors
                    .iter()
                    .find(|floor| floor.id == context.floor_id)
            })
            .map_or(context.depth, |floor| floor.depth)
    }

    pub(super) fn roll_fixed_artifact_kind_id(
        &mut self,
        context: &LootContext,
        base_item_kind_id: Option<&str>,
        instant: bool,
    ) -> Option<String> {
        let reference_depth = self.fixed_artifact_reference_depth(context);
        if reference_depth == 0 {
            return None;
        }
        let mut candidates = self
            .content
            .item_definitions()
            .filter_map(|item| {
                let generation = item.artifact_generation.as_ref()?;
                Some((
                    generation.source_index,
                    item.id.clone(),
                    item.generation_level,
                    generation.base_item_kind_id.clone(),
                    generation.rarity_one_in,
                    generation.instant,
                ))
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|candidate| candidate.0);

        for (_, kind_id, artifact_level, base_kind_id, rarity_one_in, candidate_instant) in
            candidates
        {
            if candidate_instant != instant
                || self.generated_artifact_ids.contains(&kind_id)
                || base_item_kind_id.is_some_and(|expected| expected != base_kind_id)
            {
                continue;
            }
            if artifact_level > reference_depth {
                let difference = u64::from(artifact_level - reference_depth);
                let one_in = if instant {
                    difference.saturating_mul(2)
                } else {
                    difference.saturating_mul(difference)
                };
                if self.rng.bounded(one_in) != 0 {
                    continue;
                }
            }
            if self.rng.bounded(u64::from(rarity_one_in)) != 0 {
                continue;
            }
            if instant {
                let base_level = self
                    .content
                    .item(&base_kind_id)
                    .expect("validated artifact base item must remain available")
                    .generation_level;
                if base_level > context.depth {
                    let one_in = u64::from(base_level - context.depth).saturating_mul(5);
                    if self.rng.bounded(one_in) != 0 {
                        continue;
                    }
                }
            }
            return Some(kind_id);
        }
        None
    }

    pub(super) fn fixed_item_draft(
        &mut self,
        context: &LootContext,
        kind_id: String,
    ) -> GeneratedItemDraft {
        let affix_ids = self
            .content
            .item(&kind_id)
            .and_then(|item| item.artifact_generation.as_ref())
            .map(|generation| generation.affix_ids.clone())
            .unwrap_or_default();
        let EgoMaterialization {
            affix_ids,
            rolled_affixes,
            intrinsic_properties,
            enchantment_delta,
            activation,
            charges,
            ..
        } = materialize_ego_with_rng(
            &self.content,
            &mut self.rng,
            &kind_id,
            affix_ids,
            |_| context.depth,
            context.depth,
            2,
        );
        GeneratedItemDraft {
            damage_dice_override: None,
            quantity: 1,
            origin_kind: match &context.source {
                LootSource::Rubble { .. } => Some(ItemOriginKindDto::Rubble),
                _ => None,
            },
            quality: ItemQualityDto::Ordinary,
            affix_ids,
            rolled_affixes,
            intrinsic_properties: intrinsic_properties.unwrap_or_default(),
            enchantments: enchantment_delta,
            curse: initial_item_curse(&self.content, &kind_id),
            activation,
            charges,
            fuel: initial_item_fuel(&self.content, &kind_id),
            kind_id,
        }
    }

    pub(super) fn commit_generated_item_draft(
        &mut self,
        draft: GeneratedItemDraft,
        location: ItemLocation,
    ) -> Result<ItemInstance, CoreError> {
        let id = self.allocate_item_instance_id()?;
        self.register_generated_artifact(&draft.kind_id);
        Ok(draft.into_item_instance(id, location))
    }

    pub(super) fn register_generated_artifact(&mut self, kind_id: &str) {
        if self
            .content
            .item(kind_id)
            .is_some_and(|item| item.artifact_generation.is_some())
        {
            self.generated_artifact_ids.insert(kind_id.to_owned());
        }
    }

    pub(super) fn roll_loot_quality(
        &mut self,
        weights: &[rfb_content::LootQualityWeightDefinition],
        raw_weights: &[u32],
        minimum: rfb_content::ItemQuality,
    ) -> ItemQualityDto {
        let luck = self.player_luck_bias();
        if luck == LuckBias::Neutral {
            let index = self.roll_weighted_index(raw_weights);
            return item_quality_dto(weights[index].quality.max(minimum));
        }

        let total = weights
            .iter()
            .map(|entry| u64::from(entry.weight))
            .sum::<u64>();
        let fine = weights
            .iter()
            .filter(|entry| entry.quality == rfb_content::ItemQuality::Fine)
            .map(|entry| u64::from(entry.weight))
            .sum::<u64>();
        let exceptional = weights
            .iter()
            .filter(|entry| entry.quality == rfb_content::ItemQuality::Exceptional)
            .map(|entry| u64::from(entry.weight))
            .sum::<u64>();
        let scale = total.saturating_mul(100);
        let mut good_or_better = fine.saturating_add(exceptional).saturating_mul(100);
        let mut exceptional = exceptional.saturating_mul(100);
        match luck {
            LuckBias::Good => {
                good_or_better = good_or_better.saturating_add(total.saturating_mul(5));
                exceptional = exceptional.saturating_add(total.saturating_mul(2));
            }
            LuckBias::Bad => {
                good_or_better = good_or_better.saturating_sub(total.saturating_mul(5));
                exceptional = exceptional.saturating_sub(exceptional / 4);
            }
            LuckBias::Neutral => unreachable!(),
        }
        good_or_better = good_or_better.min(scale);
        exceptional = exceptional.min(good_or_better);
        let ordinary = scale.saturating_sub(good_or_better);
        let fine = good_or_better.saturating_sub(exceptional);
        let roll = self.rng.bounded(scale);
        let quality = if roll < ordinary {
            rfb_content::ItemQuality::Ordinary
        } else if roll < ordinary.saturating_add(fine) {
            rfb_content::ItemQuality::Fine
        } else {
            rfb_content::ItemQuality::Exceptional
        };
        item_quality_dto(quality.max(minimum))
    }

    pub(super) fn roll_rfb_depth_loot_power(
        &mut self,
        policy: rfb_content::LootQualityPolicyDefinition,
        depth: u16,
        jewelry: bool,
        device: bool,
        mode: ItemGenerationMode,
    ) -> i16 {
        let (good_percent, great_percent) =
            rfb_depth_quality_percentages(policy, depth, jewelry, self.player_luck_bias());
        let chance = i32::from(self.virtue_current(rfb_protocol::VirtueKindDto::Chance));
        let good = good_percent + chance / 50;
        let great = great_percent + chance / 100;
        let no_egos = self.content.world(&self.world_id).unwrap().no_egos && !jewelry;
        if mode.minimum_power() >= 1 || rfb_magik(&mut self.rng, good) {
            if no_egos {
                1
            } else if mode.minimum_power() >= 2
                || mode == ItemGenerationMode::GreatOnly
                || rfb_magik(&mut self.rng, great)
            {
                mode.minimum_power().max(2)
            } else {
                1
            }
        } else if rfb_magik(&mut self.rng, (good + 2) / 3) {
            if !no_egos && rfb_magik(&mut self.rng, great) {
                -2
            } else if !jewelry && !device && super::ego::randint1(&mut self.rng, depth) > 10 {
                0
            } else {
                -1
            }
        } else {
            0
        }
    }

    pub(super) fn luck_adjusted_item_generation_depth(&mut self, depth: u16, staff: bool) -> u16 {
        let depth = depth.min(127);
        if self.player_luck_bias() != LuckBias::Bad {
            return depth;
        }
        let divisor = if self.rng.bounded(20) == 0 { 20 } else { 6 };
        let mut adjusted = depth.saturating_sub(depth / divisor);
        if staff {
            adjusted = adjusted.saturating_sub(adjusted / 15);
        }
        adjusted
    }
}

pub(super) fn rfb_depth_quality_percentages(
    policy: rfb_content::LootQualityPolicyDefinition,
    depth: u16,
    jewelry: bool,
    luck: LuckBias,
) -> (i32, i32) {
    let rfb_content::LootQualityPolicyDefinition::RfbDepth {
        good_cap_percent,
        great_cap_percent,
    } = policy;
    let mut good_cap = i32::from(good_cap_percent);
    let mut great_cap = i32::from(great_cap_percent);
    if luck == LuckBias::Bad {
        good_cap -= 5;
        great_cap -= great_cap / 4;
    }
    let mut good = (i32::from(depth) + 10).min(good_cap);
    let mut great = (good * 2 / 3).min(great_cap);
    if jewelry {
        good = (good + 30).min(good_cap);
    }
    if luck == LuckBias::Good {
        good += 5;
        great += 2;
    }
    (good, great)
}

fn rfb_magik(rng: &mut RfbRng, percent: i32) -> bool {
    // Preserve master z-rand.h literally, including its P <= 0 short circuit.
    percent <= 0 || rng.bounded(100) < percent as u64
}

pub(super) fn power_allows_natural_affix(
    policy: Option<rfb_content::LootQualityPolicyDefinition>,
    power: i16,
) -> bool {
    match policy {
        Some(_) => power.abs() >= 2,
        None => power != 0,
    }
}

pub(super) const fn power_quality(power: i16) -> ItemQualityDto {
    match power {
        1 => ItemQualityDto::Fine,
        2.. => ItemQualityDto::Exceptional,
        _ => ItemQualityDto::Ordinary,
    }
}
