// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeSet;

use crate::{
    resistance::DamageType,
    save::position_from_content,
    state::ItemLocation,
    stats::{AttributeKind, CharacterProgress, experience_required_for_level},
};
use rfb_content::{
    AbilityEffectDefinition, ItemUseEffectDefinition, TownFacilityCategory, WildernessDefinition,
    WildernessLocationDefinition, WildernessTerrain,
};
use rfb_protocol::{
    AbilityDetectSpecDto, AbilityDto, AbilityLearningDto, AbilitySummonSpecDto,
    AbilityTerrainTransformSpecDto, AttackProfileDto, AttributeSetDto, AttributeValueDto,
    BodySlotDto, CampaignStateDto, CellDto, CellVisualDto, ContentVisualDto, DamageDiceDto,
    DeviceRechargeDto, EntityDto, EntityFactionDto, EquipmentItemDto, GameSnapshot,
    InventoryItemDto, ItemDto, ItemKnowledgeDto, MapScaleDto, PROTOCOL_VERSION, PlayerBuildDto,
    PlayerDto, PlayerProgressDto, Position, ResistanceDto, ResourcePoolDto, SkillProgressDto,
    SummonDto, TaskServiceDto, TaskStatusDto, TerrainInteractionDto, TerrainInteractionKindDto,
    VisibilityState, WildernessLocationDto, WildernessLocationKindDto,
};

use super::tasks::projected_task_state;
use super::{
    Game, LightSource, TERRAIN_INTERACTION_DIRECTIONS, ability_detect_subject_dto,
    ability_effect_spec_dto, ability_target_spec_dto, actor_melee_routine_dto, combine_percentages,
    derived_speed, item_target_spec, light_from_sources, task_definition, task_floors,
    task_objectives,
};

const WILDERNESS_TOWN_ID: &str = "core.wilderness.town";
const WILDERNESS_ROAD_ID: &str = "core.wilderness.road";
const WILDERNESS_DUNGEON_ID: &str = "core.wilderness.dungeon";
const WILDERNESS_VISUALS: [(&str, &str); 18] = [
    ("core.wilderness.edge", "#"),
    (WILDERNESS_TOWN_ID, "#"),
    (WILDERNESS_ROAD_ID, "."),
    (WILDERNESS_DUNGEON_ID, ">"),
    ("core.wilderness.deep-water", "~"),
    ("core.wilderness.shallow-water", "~"),
    ("core.wilderness.swamp", "~"),
    ("core.wilderness.dirt", "."),
    ("core.wilderness.grass", "."),
    ("core.wilderness.trees", "T"),
    ("core.wilderness.desert", "."),
    ("core.wilderness.shallow-lava", "~"),
    ("core.wilderness.deep-lava", "~"),
    ("core.wilderness.mountain", "^"),
    ("core.wilderness.glacier", "."),
    ("core.wilderness.snow", "."),
    ("core.wilderness.pack-ice", "."),
    ("core.wilderness.terrain-town", "#"),
];

const fn wilderness_terrain_id(terrain: WildernessTerrain) -> &'static str {
    match terrain {
        WildernessTerrain::Edge => "core.wilderness.edge",
        WildernessTerrain::Town => "core.wilderness.terrain-town",
        WildernessTerrain::DeepWater => "core.wilderness.deep-water",
        WildernessTerrain::ShallowWater => "core.wilderness.shallow-water",
        WildernessTerrain::Swamp => "core.wilderness.swamp",
        WildernessTerrain::Dirt => "core.wilderness.dirt",
        WildernessTerrain::Grass => "core.wilderness.grass",
        WildernessTerrain::Trees => "core.wilderness.trees",
        WildernessTerrain::Desert => "core.wilderness.desert",
        WildernessTerrain::ShallowLava => "core.wilderness.shallow-lava",
        WildernessTerrain::DeepLava => "core.wilderness.deep-lava",
        WildernessTerrain::Mountain => "core.wilderness.mountain",
        WildernessTerrain::Glacier => "core.wilderness.glacier",
        WildernessTerrain::Snow => "core.wilderness.snow",
        WildernessTerrain::PackIce => "core.wilderness.pack-ice",
    }
}

impl Game {
    pub(super) fn player_dto(&self) -> PlayerDto {
        let stats = self.player_derived_stats();
        let melee_profile = self.player_melee_profile(&stats);
        let melee_profile_dto = melee_profile.to_dto();
        let equipment_modifiers = self.equipment_modifiers();
        let definition = self
            .content
            .actor(&self.player.kind_id)
            .expect("player actor definition must remain available");
        PlayerDto {
            id: self.player.id.clone(),
            kind_id: self.player.kind_id.clone(),
            position: self.player.position,
            hp: self.player.hp,
            max_hp: stats.max_hp.value,
            gold: self.gold,
            nutrition: self.nutrition,
            nutrition_state: self.nutrition_state(),
            speed: derived_speed(&stats.speed),
            energy_need: self.player.energy_need,
            carried_weight_tenths_pound: self.carried_weight_tenths_pound(),
            carry_capacity_tenths_pound: self.player_carry_capacity_tenths_pound(),
            encumbrance_speed_penalty: u16::try_from(self.player_encumbrance_speed_penalty())
                .unwrap_or(u16::MAX),
            inventory_used_slots: self.inventory_used_slots(),
            inventory_slot_capacity: self.inventory_slot_capacity(),
            base_max_hp: self.player.max_hp,
            attack: stats.attack.value,
            base_attack: definition.attack,
            defense: stats.defense.value,
            base_defense: definition.defense,
            melee_skill: stats.melee_skill.value,
            armor_class: stats.armor_class.value,
            melee_damage: DamageDiceDto {
                dice: melee_profile.damage_dice,
                sides: melee_profile.damage_sides,
                damage_type: melee_profile.damage_type.into(),
            },
            melee_profile: melee_profile_dto,
            projectile_profile: self
                .player_projectile_profile()
                .map(|profile| profile.to_dto()),
            is_dead: self.player_is_dead(),
            equipment_modifiers,
            statuses: self
                .player
                .statuses
                .iter()
                .map(crate::effect::StatusInstance::to_dto)
                .collect(),
            confusing_strike_ready: self.confusing_strike_ready,
            resistances: self.effective_player_resistances().to_dtos(),
            progress: self.player_progress_dto(),
            build: self.player_build_dto(),
            resources: self.player_resource_dtos(),
            device_recharge: self
                .device_recharge_profile()
                .map(|profile| DeviceRechargeDto {
                    resource_id: profile.resource_id.clone(),
                    power: profile.power,
                }),
            ability_learning: self.player_ability_learning_dto(),
            abilities: self.player_ability_dtos(),
            summon_command: self.summon_command.clone(),
            recall: self.recall.clone(),
            riding_actor_id: self.riding_actor_id.clone(),
        }
    }

    pub(super) fn projected_player_dto(&self) -> PlayerDto {
        let mut player = self.player_dto();
        if self.map_scale == MapScaleDto::World {
            player.position = self
                .wilderness_position
                .expect("world map requires a wilderness position");
        }
        player
    }

    pub(super) fn player_ability_learning_dto(&self) -> Option<AbilityLearningDto> {
        let profile = self.casting_profile()?;
        let capacity = self.ability_learning_capacity(profile);
        let learned_count = u16::try_from(self.learned_abilities.len())
            .expect("validated learned ability count must fit u16");
        Some(AbilityLearningDto {
            learned_count,
            capacity,
            remaining_slots: capacity.saturating_sub(learned_count),
        })
    }

    fn player_resource_dtos(&self) -> Vec<ResourcePoolDto> {
        self.resources
            .iter()
            .map(|(id, pool)| {
                let definition = self
                    .content
                    .resource(id)
                    .expect("player resource definition must remain available");
                ResourcePoolDto {
                    id: id.clone(),
                    name_key: definition.name_key.clone(),
                    current: pool.current,
                    maximum: pool.maximum,
                    wait_recovery_amount: definition.wait_recovery_amount,
                    rest_recovery_amount: definition.rest_recovery_amount,
                    melee_hit_gain_amount: definition.melee_hit_gain_amount,
                    melee_kill_gain_amount: definition.melee_kill_gain_amount,
                    turn_decay_amount: definition.turn_decay_amount,
                }
            })
            .collect()
    }

    fn player_ability_dtos(&self) -> Vec<AbilityDto> {
        let casting_profile = self.casting_profile();
        let book_ability_ids = casting_profile
            .map(|profile| {
                profile
                    .ability_book_ids
                    .iter()
                    .filter_map(|book_id| self.content.ability_book(book_id))
                    .flat_map(|book| book.ability_ids.iter().cloned())
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        let innate_ability_ids = self
            .technique_profiles()
            .iter()
            .flat_map(|profile| profile.innate_ability_ids.iter().cloned())
            .collect::<BTreeSet<_>>();
        book_ability_ids
            .iter()
            .cloned()
            .chain(innate_ability_ids.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .filter_map(|ability_id| {
                let ability = self.content.ability(&ability_id)?;
                let innate = innate_ability_ids.contains(&ability_id);
                let mut effective_ability = if innate {
                    ability.clone()
                } else {
                    Self::effective_casting_ability(
                        casting_profile.expect("book ability requires casting profile"),
                        ability,
                    )
                };
                Self::apply_player_level_scaling(&mut effective_ability, self.progress.level);
                if let Some(profile) = casting_profile
                    && !innate
                {
                    Self::apply_casting_profile_effect_scaling(
                        profile,
                        &mut effective_ability,
                        self.progress.level,
                    );
                }
                let ability = &effective_ability;
                let player = Self::player_ability_parameters(ability);
                let progress = self.ability_progress_value(ability);
                let resource_cost = self.ability_effective_resource_cost(ability, progress);
                let cooldown_remaining = self.ability_cooldown_remaining(ability);
                let learned = self.learned_abilities.contains(&ability_id);
                let book_item_id = if innate {
                    None
                } else {
                    casting_profile
                        .and_then(|profile| self.ability_book_item_id(profile, &ability_id))
                };
                let level_available = self.progress.level >= player.minimum_level;
                let resource_available = self
                    .resources
                    .get(&player.resource_id)
                    .is_some_and(|pool| pool.current >= resource_cost);
                let failure_percent = if innate {
                    let profile = self.technique_profile_for_ability(ability)?;
                    self.technique_failure_percent(profile, ability)
                } else {
                    self.ability_failure_percent(casting_profile?, ability)
                };
                Some(AbilityDto {
                    id: ability.id.clone(),
                    name_key: ability.name_key.clone(),
                    description_key: ability.description_key.clone(),
                    minimum_level: player.minimum_level,
                    innate,
                    resource_id: player.resource_id.clone(),
                    base_resource_cost: player.resource_cost,
                    resource_cost,
                    failure_percent,
                    proficiency: progress.proficiency,
                    proficiency_cap: progress.proficiency_cap,
                    proficiency_rank: Self::ability_proficiency_rank(progress.proficiency),
                    cast_count: progress.cast_count,
                    fail_count: progress.fail_count,
                    cooldown_remaining,
                    cooldown_turns: player.cooldown.as_ref().map_or(0, |value| value.turns),
                    cooldown_group_id: player
                        .cooldown
                        .as_ref()
                        .and_then(|value| value.group_id.clone()),
                    area_radius: match ability.effect {
                        AbilityEffectDefinition::AreaDamage { radius, .. } => Some(radius),
                        _ => None,
                    },
                    beam_damage: matches!(
                        ability.effect,
                        AbilityEffectDefinition::BeamDamage { .. }
                    ),
                    cone_radius: match ability.effect {
                        AbilityEffectDefinition::ConeDamage { radius, .. } => Some(radius),
                        _ => None,
                    },
                    teleport: matches!(ability.effect, AbilityEffectDefinition::Teleport),
                    summon: match &ability.effect {
                        AbilityEffectDefinition::Summon {
                            actor_kind_id,
                            count,
                            radius,
                            duration_turns,
                            hostile,
                        } => Some(AbilitySummonSpecDto {
                            actor_kind_id: actor_kind_id.clone(),
                            count: *count,
                            radius: *radius,
                            duration_turns: *duration_turns,
                            hostile: *hostile,
                        }),
                        _ => None,
                    },
                    detect: match &ability.effect {
                        AbilityEffectDefinition::Detect {
                            subject,
                            category,
                            radius,
                            persistent,
                        } => Some(AbilityDetectSpecDto {
                            subject: ability_detect_subject_dto(*subject),
                            category: category.clone(),
                            radius: *radius,
                            persistent: *persistent,
                        }),
                        _ => None,
                    },
                    terrain_transform: match &ability.effect {
                        AbilityEffectDefinition::TransformTerrain {
                            source_terrain_ids,
                            target_terrain_id,
                            radius,
                        } => Some(AbilityTerrainTransformSpecDto {
                            source_terrain_ids: source_terrain_ids.clone(),
                            target_terrain_id: target_terrain_id.clone(),
                            radius: *radius,
                        }),
                        _ => None,
                    },
                    effects: ability
                        .effect
                        .ordered_effects()
                        .iter()
                        .map(ability_effect_spec_dto)
                        .collect(),
                    target_spec: ability_target_spec_dto(ability),
                    learned,
                    book_item_id: book_item_id.clone(),
                    can_study: !innate
                        && !learned
                        && level_available
                        && book_item_id.is_some()
                        && self
                            .player_ability_learning_dto()
                            .is_some_and(|learning| learning.remaining_slots > 0),
                    can_forget: !innate && learned,
                    can_cast: if innate {
                        level_available && resource_available && cooldown_remaining == 0
                    } else {
                        learned
                            && level_available
                            && resource_available
                            && cooldown_remaining == 0
                            && book_item_id.is_some()
                    },
                })
            })
            .collect()
    }

    fn player_progress_dto(&self) -> PlayerProgressDto {
        let natural = self.progress.attributes;
        let effective = self.effective_player_attributes();
        let skills = self.effective_player_skill_progress();
        let value = |kind| AttributeValueDto {
            natural: natural.value(kind),
            maximum_natural: self.progress.maximum_attributes.value(kind),
            effective: effective.value(kind),
            index: effective.index(kind),
        };
        let victory_unlocked = self.victory_level_cap_unlocked();
        let level_cap = CharacterProgress::level_cap(victory_unlocked);
        PlayerProgressDto {
            level: self.progress.level,
            max_level: self.progress.max_level,
            experience: self.progress.experience,
            maximum_experience: self.progress.maximum_experience,
            life_force: self.progress.life_force,
            level_cap,
            attribute_cap: CharacterProgress::attribute_cap(victory_unlocked),
            attribute_index_cap: CharacterProgress::attribute_index_cap(victory_unlocked),
            experience_for_next_level: (self.progress.level < level_cap)
                .then(|| experience_required_for_level(self.progress.level.saturating_add(1))),
            pending_attribute_increases: self.progress.pending_attribute_increases,
            victory_level_cap_unlocked: victory_unlocked,
            attributes: AttributeSetDto {
                strength: value(AttributeKind::Strength),
                intelligence: value(AttributeKind::Intelligence),
                wisdom: value(AttributeKind::Wisdom),
                dexterity: value(AttributeKind::Dexterity),
                constitution: value(AttributeKind::Constitution),
                charisma: value(AttributeKind::Charisma),
            },
            skills: skills
                .iter()
                .map(|(id, skill)| SkillProgressDto {
                    id: id.clone(),
                    name_key: self
                        .content
                        .skill(id)
                        .map_or_else(|| id.clone(), |definition| definition.name_key.clone()),
                    current: skill.current,
                    maximum: skill.maximum,
                    base: skill.base,
                    growth_per_ten_levels: skill.growth_per_ten_levels,
                })
                .collect(),
        }
    }

    fn player_build_dto(&self) -> Option<PlayerBuildDto> {
        let (build, race, class, personality) = self.character_definitions()?;
        Some(PlayerBuildDto {
            build_id: build.id.clone(),
            build_name_key: build.name_key.clone(),
            race_id: race.id.clone(),
            race_name_key: race.name_key.clone(),
            class_id: class.id.clone(),
            class_name_key: class.name_key.clone(),
            personality_id: personality.id.clone(),
            personality_name_key: personality.name_key.clone(),
            life_percent: combine_percentages([
                race.life_percent,
                class.life_percent,
                personality.life_percent,
            ]),
            experience_percent: combine_percentages([
                race.experience_percent,
                class.experience_percent,
                personality.experience_percent,
            ]),
        })
    }

    pub(super) fn campaign_state_dto(&self) -> CampaignStateDto {
        let (conquered_dungeons, completed_tasks) = self.campaign_counts();
        CampaignStateDto {
            status: self.campaign_state.status,
            score: self
                .campaign_state
                .final_score
                .unwrap_or_else(|| self.campaign_score_at(self.turn)),
            conquered_dungeons,
            completed_tasks,
            victory_turn: self.campaign_state.victory_turn,
            retired_turn: self.campaign_state.retired_turn,
        }
    }

    pub(super) fn entities_dto(&self) -> Vec<EntityDto> {
        let mut entities = self
            .entities
            .iter()
            .filter(|entity| self.entity_is_visible_to_player(entity))
            .map(|entity| {
                let definition = self
                    .content
                    .actor(&entity.kind_id)
                    .expect("entity actor definition must remain available");
                let stats = self.actor_derived_stats(entity, definition, false);
                EntityDto {
                    id: entity.id.clone(),
                    kind_id: entity
                        .appearance_kind_id
                        .clone()
                        .unwrap_or_else(|| entity.kind_id.clone()),
                    position: entity.position,
                    hp: entity.hp,
                    max_hp: entity.max_hp,
                    speed: derived_speed(&stats.speed),
                    energy_need: entity.energy_need,
                    alerted: entity.alerted,
                    casting_cooldown_remaining: entity.casting_cooldown_remaining,
                    observed_player_resistances: entity
                        .observed_player_resistances
                        .iter()
                        .map(|(damage_type, level)| ResistanceDto {
                            damage_type: (*damage_type).into(),
                            level: (*level).into(),
                        })
                        .collect(),
                    attack: stats.attack.value,
                    defense: stats.defense.value,
                    melee_skill: stats.melee_skill.value,
                    armor_class: stats.armor_class.value,
                    melee_damage: DamageDiceDto {
                        dice: definition.damage_dice,
                        sides: definition.damage_sides,
                        damage_type: DamageType::from(definition.damage_type).into(),
                    },
                    melee_profile: AttackProfileDto {
                        attacks: 1,
                        to_hit: 0,
                        to_damage: 0,
                        damage: DamageDiceDto {
                            dice: definition.damage_dice,
                            sides: definition.damage_sides,
                            damage_type: DamageType::from(definition.damage_type).into(),
                        },
                        source_item_id: None,
                    },
                    melee_routine: actor_melee_routine_dto(definition),
                    statuses: entity
                        .statuses
                        .iter()
                        .map(crate::effect::StatusInstance::to_dto)
                        .collect(),
                    faction: if self.actor_is_player_aligned(entity) {
                        EntityFactionDto::Player
                    } else if self.actor_is_friendly(entity) {
                        EntityFactionDto::Friendly
                    } else {
                        EntityFactionDto::Hostile
                    },
                    controller_id: entity.controller_id.clone(),
                    summon: entity.summon.as_ref().map(|summon| SummonDto {
                        owner_id: summon.owner_id.clone(),
                        source_ability_id: summon.source_ability_id.clone(),
                        remaining_turns: summon.remaining_turns,
                    }),
                }
            })
            .collect::<Vec<_>>();
        entities.sort_by(|left, right| left.id.cmp(&right.id));
        entities
    }

    pub(super) fn items_dto(&self) -> Vec<ItemDto> {
        let mut items = self
            .items
            .iter()
            .filter_map(|item| {
                let ItemLocation::Ground(position) = &item.location else {
                    return None;
                };
                if !self.item_is_discovered(&item.id) {
                    return None;
                }
                Some(ItemDto {
                    id: item.id.clone(),
                    kind_id: item.kind_id.clone(),
                    display_name_key: self.item_display_name_key(&item.kind_id),
                    knowledge: self.item_knowledge_dto(&item.kind_id),
                    position: *position,
                    quantity: item.quantity,
                    inscription: item.inscription.clone(),
                    fuel: item.fuel,
                    enchantments: item.enchantments,
                    curse: self.visible_item_curse(item),
                })
            })
            .collect::<Vec<_>>();
        items.sort_by(|left, right| left.id.cmp(&right.id));
        items
    }

    pub(super) fn inventory_dto(&self) -> Vec<InventoryItemDto> {
        let mut inventory = self
            .items
            .iter()
            .filter_map(|item| {
                if item.location != ItemLocation::Inventory {
                    return None;
                }
                Some(InventoryItemDto {
                    id: item.id.clone(),
                    kind_id: item.kind_id.clone(),
                    display_name_key: self.item_display_name_key(&item.kind_id),
                    knowledge: self.item_knowledge_dto(&item.kind_id),
                    usable: self.content.item(&item.kind_id).is_some_and(|definition| {
                        definition.use_action.as_ref().is_some_and(|action| {
                            action.charges.is_none_or(|charges| {
                                item.charges
                                    .is_some_and(|state| state.current >= charges.cost)
                            })
                        }) || item.activation.as_ref().is_some_and(|activation| {
                            item.charges
                                .is_some_and(|state| state.current >= activation.cost)
                        })
                    }),
                    charges: (self.item_knowledge_dto(&item.kind_id) == ItemKnowledgeDto::Aware)
                        .then_some(item.charges)
                        .flatten(),
                    fuel: item.fuel,
                    activation: (self.item_knowledge_dto(&item.kind_id) == ItemKnowledgeDto::Aware)
                        .then(|| item.activation.clone())
                        .flatten(),
                    use_target_spec: item
                        .activation
                        .as_ref()
                        .map(|activation| activation.target_spec.clone())
                        .or_else(|| {
                            self.content
                                .item(&item.kind_id)
                                .and_then(|definition| definition.use_action.as_ref())
                                .and_then(|action| match &action.effect {
                                    ItemUseEffectDefinition::IdentifyItem { .. }
                                    | ItemUseEffectDefinition::EnchantItem { .. } => {
                                        Some(item_target_spec())
                                    }
                                    _ => None,
                                })
                        }),
                    requires_target_glyph: self.inventory_item_use_effect(&item.id).is_some_and(
                        |(effect, _)| matches!(effect, ItemUseEffectDefinition::Genocide { .. }),
                    ),
                    requires_recharge_targets: self
                        .inventory_item_use_effect(&item.id)
                        .is_some_and(|(effect, _)| {
                            matches!(effect, ItemUseEffectDefinition::RechargeFromDevice { .. })
                        }),
                    can_receive_recharge: self.item_can_receive_recharge(item),
                    can_supply_recharge: self.item_can_supply_recharge(item),
                    quantity: item.quantity,
                    inscription: item.inscription.clone(),
                    enchantments: item.enchantments,
                    curse: self.visible_item_curse(item),
                    weight_tenths_pound: self.item_weight_tenths_pound(&item.kind_id),
                    equipment_slot: self
                        .content
                        .item(&item.kind_id)
                        .and_then(|definition| definition.equipment_slot.clone()),
                    modifiers: self.visible_item_modifiers(item),
                    equipment_bonuses: self.visible_item_equipment_bonuses(item),
                    resistances: self.visible_item_resistances(item),
                    status_immunities: self.visible_item_status_immunities(item),
                    slays: self.visible_item_slays(item),
                    brands: self.visible_item_brands(item),
                    passives: self.visible_item_passives(item),
                    identification: self.item_identification(item),
                    quality: self.visible_item_quality(item),
                    known_properties: self.known_item_properties(item),
                    melee_profile: self.visible_item_melee_profile(item),
                    projectile_profile: self.visible_item_projectile_profile(item),
                    throw_profile: self.visible_item_throw_profile(item),
                })
            })
            .collect::<Vec<_>>();
        inventory.sort_by(|left, right| left.id.cmp(&right.id));
        inventory
    }

    pub(super) fn equipment_dto(&self) -> Vec<EquipmentItemDto> {
        let mut equipment = self
            .items
            .iter()
            .filter_map(|item| {
                let ItemLocation::Equipped { slot_id } = &item.location else {
                    return None;
                };
                Some(EquipmentItemDto {
                    id: item.id.clone(),
                    kind_id: item.kind_id.clone(),
                    display_name_key: self.item_display_name_key(&item.kind_id),
                    knowledge: self.item_knowledge_dto(&item.kind_id),
                    quantity: item.quantity,
                    inscription: item.inscription.clone(),
                    fuel: item.fuel,
                    enchantments: item.enchantments,
                    curse: self.visible_item_curse(item),
                    weight_tenths_pound: self.item_weight_tenths_pound(&item.kind_id),
                    slot_id: slot_id.clone(),
                    modifiers: self.visible_item_modifiers(item),
                    equipment_bonuses: self.visible_item_equipment_bonuses(item),
                    resistances: self.visible_item_resistances(item),
                    status_immunities: self.visible_item_status_immunities(item),
                    slays: self.visible_item_slays(item),
                    brands: self.visible_item_brands(item),
                    passives: self.visible_item_passives(item),
                    identification: self.item_identification(item),
                    quality: self.visible_item_quality(item),
                    known_properties: self.known_item_properties(item),
                    melee_profile: self.visible_item_melee_profile(item),
                    projectile_profile: self.visible_item_projectile_profile(item),
                    throw_profile: self.visible_item_throw_profile(item),
                })
            })
            .collect::<Vec<_>>();
        equipment.sort_by(|left, right| left.slot_id.cmp(&right.slot_id));
        equipment
    }

    #[must_use]
    pub fn snapshot(&self) -> GameSnapshot {
        let (width, height) = self.projected_dimensions();
        let cells = self.projected_cells();
        let visual_cells = self.visual_cells();
        let world_map = self.map_scale == MapScaleDto::World;
        GameSnapshot {
            protocol_version: PROTOCOL_VERSION.to_owned(),
            revision: self.revision,
            turn: self.turn,
            world_tick: self.world_tick,
            last_command_seq: self.last_command_seq,
            map_scale: self.map_scale,
            world_travel_destination: self.world_travel_destination,
            width,
            height,
            cells,
            visual_cells,
            player: self.projected_player_dto(),
            entities: if world_map {
                Vec::new()
            } else {
                self.entities_dto()
            },
            items: if world_map {
                Vec::new()
            } else {
                self.items_dto()
            },
            gold_piles: if world_map {
                Vec::new()
            } else {
                self.gold_pile_dtos()
            },
            inventory: self.inventory_dto(),
            equipment: self.equipment_dto(),
            body_slots: self
                .body_slots
                .iter()
                .map(|slot| BodySlotDto {
                    id: slot.id.clone(),
                    slot_type: slot.slot_type.clone(),
                })
                .collect(),
            mogaminator: self.mogaminator_dto(Vec::new()),
            content_id: self.content.pack_id().to_owned(),
            content_hash: self.content.content_hash().to_owned(),
            content_visuals: self.content_visuals(),
            world_id: self.world_id.clone(),
            floor_id: self.current_floor_id.clone(),
            dungeon_instance_id: self.current_dungeon_instance_id.clone(),
            town: (!world_map).then(|| self.current_town_dto()).flatten(),
            shops: if world_map {
                Vec::new()
            } else {
                self.current_shop_dtos()
            },
            homes: if world_map {
                Vec::new()
            } else {
                self.current_home_dtos()
            },
            task_services: if world_map {
                Vec::new()
            } else {
                self.current_task_service_dtos()
            },
            terrain_interactions: if world_map {
                Vec::new()
            } else {
                self.terrain_interactions()
            },
            tasks: self.task_statuses(),
            campaign: self.campaign_state_dto(),
            state_hash: self.state_hash(),
        }
    }

    fn content_visuals(&self) -> Vec<ContentVisualDto> {
        let mut visuals = self
            .content
            .visual_glyphs()
            .into_iter()
            .map(|(id, glyph)| ContentVisualDto { id, glyph })
            .collect::<Vec<_>>();
        for appearance in [
            rfb_protocol::GoldAppearanceDto::Copper,
            rfb_protocol::GoldAppearanceDto::Silver,
            rfb_protocol::GoldAppearanceDto::Garnets,
            rfb_protocol::GoldAppearanceDto::Gold,
            rfb_protocol::GoldAppearanceDto::Opals,
            rfb_protocol::GoldAppearanceDto::Sapphires,
            rfb_protocol::GoldAppearanceDto::Rubies,
            rfb_protocol::GoldAppearanceDto::Diamonds,
            rfb_protocol::GoldAppearanceDto::Emeralds,
            rfb_protocol::GoldAppearanceDto::Mithril,
            rfb_protocol::GoldAppearanceDto::Adamantite,
        ] {
            visuals.push(ContentVisualDto {
                id: super::gold::gold_visual_id(appearance).to_owned(),
                glyph: "$".to_owned(),
            });
        }
        if self
            .content
            .world(&self.world_id)
            .is_some_and(|world| world.wilderness.is_some())
        {
            visuals.extend(
                WILDERNESS_VISUALS
                    .into_iter()
                    .map(|(id, glyph)| ContentVisualDto {
                        id: id.to_owned(),
                        glyph: glyph.to_owned(),
                    }),
            );
        }
        visuals.sort_by(|left, right| left.id.cmp(&right.id));
        visuals
    }

    pub(super) fn projected_dimensions(&self) -> (u16, u16) {
        if self.map_scale == MapScaleDto::World {
            let wilderness = self.wilderness();
            (wilderness.width, wilderness.height)
        } else {
            (self.width, self.height)
        }
    }

    pub(super) fn projected_cells(&self) -> Vec<CellDto> {
        let (width, height) = self.projected_dimensions();
        let mut cells = Vec::with_capacity(usize::from(width) * usize::from(height));
        for y in 0..height {
            for x in 0..width {
                let position = Position {
                    x: i32::from(x),
                    y: i32::from(y),
                };
                cells.push(if self.map_scale == MapScaleDto::World {
                    self.wilderness_cell_dto(position)
                } else {
                    self.cell_dto(position)
                });
            }
        }
        cells
    }

    pub(super) fn cell_dto(&self, position: Position) -> CellDto {
        let actor_id = if self.player.position == position {
            Some(self.player.id.clone())
        } else {
            self.entities
                .iter()
                .find(|entity| {
                    entity.position == position && self.entity_is_visible_to_player(entity)
                })
                .map(|entity| entity.id.clone())
        };
        CellDto {
            position,
            terrain_id: self.known_terrain_at(position).to_owned(),
            item_id: self
                .gold_piles
                .iter()
                .filter(|pile| pile.position == position)
                .min_by(|left, right| left.id.cmp(&right.id))
                .map(|pile| pile.id.clone())
                .or_else(|| {
                    self.items
                        .iter()
                        .filter(|item| {
                            item.location == ItemLocation::Ground(position)
                                && self.item_is_discovered(&item.id)
                        })
                        .min_by(|left, right| left.id.cmp(&right.id))
                        .map(|item| item.id.clone())
                }),
            actor_id,
            danger_level: None,
            locations: Vec::new(),
        }
    }

    pub(super) fn wilderness(&self) -> &WildernessDefinition {
        self.content
            .world(&self.world_id)
            .and_then(|world| world.wilderness.as_ref())
            .expect("world map state requires wilderness content")
    }

    pub(super) fn wilderness_cell_dto(&self, position: Position) -> CellDto {
        let wilderness = self.wilderness();
        let symbol = wilderness.rows[usize::try_from(position.y).expect("validated wilderness y")]
            .as_bytes()[usize::try_from(position.x).expect("validated wilderness x")];
        let legend = wilderness
            .legend
            .iter()
            .find(|entry| entry.symbol.as_bytes() == [symbol])
            .expect("validated wilderness symbol must remain defined");
        let locations = wilderness
            .locations
            .iter()
            .filter_map(|location| match location {
                WildernessLocationDefinition::Town {
                    position: candidate,
                    town_id,
                } if position_from_content(*candidate) == position => Some(WildernessLocationDto {
                    kind: WildernessLocationKindDto::Town,
                    id: town_id.clone(),
                }),
                WildernessLocationDefinition::Dungeon {
                    position: candidate,
                    dungeon_id,
                } if position_from_content(*candidate) == position => Some(WildernessLocationDto {
                    kind: WildernessLocationKindDto::Dungeon,
                    id: dungeon_id.clone(),
                }),
                _ => None,
            })
            .collect::<Vec<_>>();
        let terrain_id = if locations
            .iter()
            .any(|location| location.kind == WildernessLocationKindDto::Town)
        {
            WILDERNESS_TOWN_ID
        } else if legend.road {
            WILDERNESS_ROAD_ID
        } else if locations
            .iter()
            .any(|location| location.kind == WildernessLocationKindDto::Dungeon)
        {
            WILDERNESS_DUNGEON_ID
        } else {
            wilderness_terrain_id(legend.terrain)
        };
        CellDto {
            position,
            terrain_id: terrain_id.to_owned(),
            item_id: None,
            actor_id: (self.wilderness_position == Some(position)).then(|| self.player.id.clone()),
            danger_level: Some(legend.level),
            locations,
        }
    }

    pub(super) fn visual_cells(&self) -> Vec<CellVisualDto> {
        if self.map_scale == MapScaleDto::World {
            let (width, height) = self.projected_dimensions();
            return (0..height)
                .flat_map(|y| {
                    (0..width).map(move |x| CellVisualDto {
                        position: Position {
                            x: i32::from(x),
                            y: i32::from(y),
                        },
                        visibility: VisibilityState::Visible,
                        light: rfb_protocol::CellLightDto {
                            color: 0xffffff,
                            intensity: 100,
                        },
                    })
                })
                .collect();
        }
        // Light sources are collected once per pass; scanning every entity
        // and ground item again for each of the W*H cells is the dominant
        // fixed cost of a visual rebuild on larger maps.
        let light_sources = self.collect_light_sources();
        let mut visuals = Vec::with_capacity(self.terrain.len());
        for y in 0..self.height {
            for x in 0..self.width {
                visuals.push(self.cell_visual(
                    &light_sources,
                    Position {
                        x: i32::from(x),
                        y: i32::from(y),
                    },
                ));
            }
        }
        visuals
    }

    pub(super) fn changed_visual_cells(
        current: &[CellVisualDto],
        previous: &[CellVisualDto],
    ) -> Vec<CellVisualDto> {
        current
            .iter()
            .zip(previous.iter())
            .filter_map(|(current, before)| (current != before).then_some(*current))
            .collect()
    }

    fn cell_visual(&self, light_sources: &[LightSource], position: Position) -> CellVisualDto {
        let index = self.index(position).expect("validated visual position");
        CellVisualDto {
            position,
            visibility: if self.is_visible(position) {
                VisibilityState::Visible
            } else if self.explored[index] {
                VisibilityState::Remembered
            } else {
                VisibilityState::Hidden
            },
            light: light_from_sources(
                light_sources,
                position,
                self.ambient_light(position, light_sources),
            ),
        }
    }

    pub(super) fn terrain_interactions(&self) -> Vec<TerrainInteractionDto> {
        let mut interactions = Vec::new();
        for direction in TERRAIN_INTERACTION_DIRECTIONS {
            let position = self.position_in_direction(direction);
            if self.index(position).is_none() {
                continue;
            }
            let Some(terrain) = self.content.terrain(self.known_terrain_at(position)) else {
                continue;
            };
            let unavailable_reason = self.terrain_interaction_unavailable_reason(position);
            let available = unavailable_reason.is_none();
            if terrain.open_to_terrain_id.is_some() {
                interactions.push(TerrainInteractionDto {
                    kind: TerrainInteractionKindDto::OpenDoor,
                    direction,
                    position,
                    terrain_id: terrain.id.clone(),
                    requires_check: terrain.open_check_difficulty.is_some(),
                    available,
                    unavailable_reason,
                });
            }
            if terrain.close_to_terrain_id.is_some() {
                interactions.push(TerrainInteractionDto {
                    kind: TerrainInteractionKindDto::CloseDoor,
                    direction,
                    position,
                    terrain_id: terrain.id.clone(),
                    requires_check: false,
                    available,
                    unavailable_reason,
                });
            }
            if terrain.bash_to_terrain_id.is_some() {
                interactions.push(TerrainInteractionDto {
                    kind: TerrainInteractionKindDto::BashDoor,
                    direction,
                    position,
                    terrain_id: terrain.id.clone(),
                    requires_check: true,
                    available,
                    unavailable_reason,
                });
            }
            if terrain.trap.is_some() {
                interactions.push(TerrainInteractionDto {
                    kind: TerrainInteractionKindDto::DisarmTrap,
                    direction,
                    position,
                    terrain_id: terrain.id.clone(),
                    requires_check: true,
                    available,
                    unavailable_reason,
                });
            }
            if terrain.dig_to_terrain_id.is_some() {
                interactions.push(TerrainInteractionDto {
                    kind: TerrainInteractionKindDto::DigTerrain,
                    direction,
                    position,
                    terrain_id: terrain.id.clone(),
                    requires_check: true,
                    available,
                    unavailable_reason,
                });
            }
        }
        interactions
    }

    pub(super) fn task_statuses(&self) -> Vec<TaskStatusDto> {
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        world
            .tasks
            .iter()
            .filter_map(|task| {
                let task_id = &task.id;
                let state = projected_task_state(world, &self.task_states, task_id)?;
                let task =
                    task_definition(world, task_id).expect("task state must retain its definition");
                let floor = task_floors(world, task_id)
                    .next()
                    .expect("task state must have a representative floor");
                let stages = u32::try_from(task_objectives(world, task_id).len())
                    .expect("validated task stage count must fit u32");
                Some(TaskStatusDto {
                    task_id: task_id.clone(),
                    floor_id: floor.id.clone(),
                    name_key: task.name_key.clone(),
                    description_key: Some(task.description_key.clone()),
                    source_facility_id: task.source_facility_id.clone(),
                    status: state.status,
                    current: state.current,
                    required: state.required,
                    stage: state.stage_index.saturating_add(1),
                    stages,
                    retakes_used: state.retakes_used,
                    max_retakes: floor.max_retakes,
                })
            })
            .collect()
    }

    pub(super) fn current_task_service_dtos(&self) -> Vec<TaskServiceDto> {
        let Some(world) = self.content.world(&self.world_id) else {
            return Vec::new();
        };
        let Some(town) = world
            .town_id
            .as_deref()
            .and_then(|town_id| self.content.town(town_id))
            .filter(|town| self.current_floor_id == town.floor_id)
        else {
            return Vec::new();
        };
        town.facility_ids
            .iter()
            .filter_map(|facility_id| self.content.town_facility(facility_id))
            .filter(|facility| facility.category == TownFacilityCategory::QuestGiver)
            .map(|facility| {
                let entrance_position = position_from_content(facility.entrance_position);
                let player_at_entrance = self.player.position == entrance_position;
                let mut tasks = if player_at_entrance {
                    facility
                        .task_ids
                        .iter()
                        .filter_map(|task_id| {
                            let task = task_definition(world, task_id)?;
                            let state = projected_task_state(world, &self.task_states, task_id)?;
                            let floor = task_floors(world, task_id).next()?;
                            let stages = u32::try_from(task.objectives.len()).ok()?;
                            Some(TaskStatusDto {
                                task_id: task.id.clone(),
                                floor_id: floor.id.clone(),
                                name_key: task.name_key.clone(),
                                description_key: Some(task.description_key.clone()),
                                source_facility_id: task.source_facility_id.clone(),
                                status: state.status,
                                current: state.current,
                                required: state.required,
                                stage: state.stage_index.saturating_add(1),
                                stages,
                                retakes_used: state.retakes_used,
                                max_retakes: floor.max_retakes,
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                tasks.sort_by(|left, right| left.task_id.cmp(&right.task_id));
                TaskServiceDto {
                    id: facility.id.clone(),
                    name_key: facility.name_key.clone(),
                    description_key: facility.description_key.clone(),
                    owner_name_key: facility
                        .owner_name_key
                        .clone()
                        .expect("validated quest giver must retain an owner name"),
                    entrance_position,
                    entrance_terrain_id: facility.entrance_terrain_id.clone(),
                    player_at_entrance,
                    tasks,
                }
            })
            .collect()
    }
}
