// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    effect::StatusInstance,
    error::CoreError,
    resistance::{DamageType, ResistanceLevel, ResistanceProfile},
    state::{
        Actor, BASE_ACTOR_POWER_PER_MILLE, FloorConnectionState, FloorRegionState, FloorState,
        GoldPile, ItemInstance, ItemLocation, MonsterPackIdentity, RolledAffixState,
        SummonIdentity,
    },
    stats::{CharacterBuildIdentity, CharacterProgress},
};
use rfb_content::{
    AbilityTargetModeDefinition, ActorDamageType, ActorResistanceLevel,
    AffixPropertyBundleDefinition, ContentCatalog, ContentPosition, EquipmentBonuses,
    EquipmentPassive, ItemFuelKindDefinition, SlayLevel, SlayTarget, StatModifiers, WeaponBrand,
};
use rfb_protocol::{
    ActorSaveDto, CarriedItemSaveDto, DamageTypeDto, EquipmentBonusesDto, EquipmentItemSaveDto,
    EquipmentPassiveDto, FloorConnectionSaveDto, FloorRegionSaveDto, FloorSaveDto, GoldPileDto,
    InventoryItemSaveDto, ItemActivationDto, ItemChargesDto, ItemEnchantmentsDto, ItemFuelDto,
    ItemFuelKindDto, ItemOriginKindDto, ItemSaveDto, MonsterPackSaveDto,
    NaturalAttributeSetSaveDto, PlayerBuildSaveDto, PlayerProgressSaveDto, PlayerSaveDto, Position,
    ResistanceDto, ResistanceLevelDto, ResistanceSaveDto, RolledAffixSaveDto, SkillProgressSaveDto,
    SlayDto, SlayLevelDto, SlayTargetDto, StatModifiersDto, StatusSaveDto, SummonSaveDto,
    TargetModeDto, TargetSpecDto, TerrainSaveDto, VirtueDto, WeaponBrandDto,
};

pub(crate) const GENERATED_ITEM_ID_PREFIX: &str = "generated.item.";

pub(crate) fn actor_max_hp_is_valid(
    definition: &rfb_content::ActorDefinition,
    max_hp: i32,
) -> bool {
    let Some(hit_points) = definition.hit_point_dice else {
        return max_hp == definition.max_hp;
    };
    let maximum = i32::from(hit_points.dice).saturating_mul(i32::from(hit_points.sides));
    if hit_points.force_maximum {
        max_hp == maximum
    } else {
        (i32::from(hit_points.dice)..=maximum).contains(&max_hp)
    }
}

pub(crate) fn initial_item_fuel(content: &ContentCatalog, kind_id: &str) -> Option<ItemFuelDto> {
    content.item(kind_id).and_then(item_fuel_from_definition)
}

fn item_fuel_from_definition(definition: &rfb_content::ItemDefinition) -> Option<ItemFuelDto> {
    definition.fuel.map(|fuel| ItemFuelDto {
        kind: match fuel.kind {
            ItemFuelKindDefinition::Torch => ItemFuelKindDto::Torch,
            ItemFuelKindDefinition::Lantern => ItemFuelKindDto::Lantern,
            ItemFuelKindDefinition::Oil => ItemFuelKindDto::Oil,
        },
        current: fuel.initial,
        maximum: fuel.maximum,
        light_radius: fuel.light_radius,
    })
}

pub(crate) fn actor_from_spawn(
    id: &str,
    kind_id: &str,
    position: ContentPosition,
    max_hp: i32,
    speed: u16,
    energy_need: i32,
    alerted: bool,
) -> Actor {
    Actor {
        id: id.to_owned(),
        kind_id: kind_id.to_owned(),
        appearance_kind_id: None,
        position: position_from_content(position),
        hp: max_hp,
        max_hp,
        power_per_mille: BASE_ACTOR_POWER_PER_MILLE,
        speed,
        energy_need,
        alerted,
        nice: false,
        visible_invisible: false,
        visible_weird_mind: false,
        eldritch_horror_triggered: false,
        casting_cooldown_remaining: 0,
        observed_player_resistances: BTreeMap::new(),
        statuses: Vec::new(),
        resistances: ResistanceProfile::default(),
        pack: None,
        controller_id: None,
        summon: None,
    }
}

pub(crate) fn actor_from_runtime_spawn(
    id: &str,
    kind_id: &str,
    position: Position,
    max_hp: i32,
    speed: u16,
    energy_need: i32,
    alerted: bool,
) -> Actor {
    Actor {
        id: id.to_owned(),
        kind_id: kind_id.to_owned(),
        appearance_kind_id: None,
        position,
        hp: max_hp,
        max_hp,
        power_per_mille: BASE_ACTOR_POWER_PER_MILLE,
        speed,
        energy_need,
        alerted,
        nice: false,
        visible_invisible: false,
        visible_weird_mind: false,
        eldritch_horror_triggered: false,
        casting_cooldown_remaining: 0,
        observed_player_resistances: BTreeMap::new(),
        statuses: Vec::new(),
        resistances: ResistanceProfile::default(),
        pack: None,
        controller_id: None,
        summon: None,
    }
}

pub(crate) const fn position_from_content(position: ContentPosition) -> Position {
    Position {
        x: position.x as i32,
        y: position.y as i32,
    }
}

pub(crate) fn actor_from_player(
    player: PlayerSaveDto,
    content: &ContentCatalog,
) -> Result<Actor, CoreError> {
    let definition = content
        .actor(&player.kind_id)
        .ok_or_else(|| CoreError::UnknownActor(player.kind_id.clone()))?;
    if player.base_max_hp != 0 && player.base_max_hp != definition.max_hp {
        return Err(CoreError::InvalidSave("player base max HP is invalid"));
    }
    if player.base_speed != definition.speed {
        return Err(CoreError::InvalidSave("player base speed is invalid"));
    }
    let statuses = statuses_from_save(player.statuses)?;
    let resistances = resistances_from_save(player.resistances)?;
    Ok(Actor {
        id: player.id,
        kind_id: player.kind_id,
        appearance_kind_id: None,
        position: player.position,
        hp: player.hp,
        max_hp: definition.max_hp,
        power_per_mille: BASE_ACTOR_POWER_PER_MILLE,
        speed: player.base_speed,
        energy_need: player.energy_need,
        alerted: true,
        nice: false,
        visible_invisible: false,
        visible_weird_mind: false,
        eldritch_horror_triggered: false,
        casting_cooldown_remaining: 0,
        observed_player_resistances: BTreeMap::new(),
        statuses,
        resistances,
        pack: None,
        controller_id: None,
        summon: None,
    })
}

pub(crate) fn derive_next_item_instance_serial(
    player: &Actor,
    entities: &[Actor],
    items: &[ItemInstance],
) -> Result<u64, CoreError> {
    let maximum = std::iter::once(player.id.as_str())
        .chain(entities.iter().map(|entity| entity.id.as_str()))
        .chain(items.iter().map(|item| item.id.as_str()))
        .filter_map(generated_item_serial)
        .max()
        .unwrap_or(0);
    maximum.checked_add(1).ok_or(CoreError::ItemIdExhausted)
}

fn generated_item_serial(id: &str) -> Option<u64> {
    id.strip_prefix(GENERATED_ITEM_ID_PREFIX)?.parse().ok()
}

pub(crate) fn actor_from_entity(
    entity: ActorSaveDto,
    content: &ContentCatalog,
) -> Result<Actor, CoreError> {
    let definition = content
        .actor(&entity.kind_id)
        .ok_or_else(|| CoreError::UnknownActor(entity.kind_id.clone()))?;
    let appearance = if let Some(appearance_kind_id) = entity.appearance_kind_id.as_deref() {
        let appearance = content
            .actor(appearance_kind_id)
            .ok_or_else(|| CoreError::UnknownActor(appearance_kind_id.to_owned()))?;
        let changes_form = definition
            .tags
            .iter()
            .any(|tag| matches!(tag.as_str(), "shapechanger" | "chameleon" | "tanuki"));
        let valid = if changes_form {
            appearance.role == rfb_content::ActorRole::Monster
                && appearance.id != definition.id
                && !appearance
                    .tags
                    .iter()
                    .any(|tag| tag == "shadower-appearance")
        } else {
            appearance
                .tags
                .iter()
                .any(|tag| tag == "shadower-appearance")
                && definition.level >= 10
                && !definition
                    .tags
                    .iter()
                    .any(|tag| matches!(tag.as_str(), "unique" | "unique2"))
        };
        if !valid {
            return Err(CoreError::InvalidSave("entity appearance is invalid"));
        }
        Some(appearance)
    } else {
        None
    };
    let runtime_definition = if definition.tags.iter().any(|tag| tag == "chameleon") {
        appearance.unwrap_or(definition)
    } else {
        definition
    };
    if !actor_max_hp_is_valid(runtime_definition, entity.max_hp) {
        return Err(CoreError::InvalidSave("entity base stats are invalid"));
    }
    if entity.base_speed != runtime_definition.speed {
        return Err(CoreError::InvalidSave("entity base speed is invalid"));
    }
    if entity.power_per_mille < 100 {
        return Err(CoreError::InvalidSave("entity power is invalid"));
    }
    let statuses = statuses_from_save(entity.statuses)?;
    let resistances = resistances_from_save(entity.resistances)?;
    let observed_player_resistances =
        observed_resistances_from_save(entity.observed_player_resistances)?;
    Ok(Actor {
        id: entity.id,
        kind_id: entity.kind_id,
        appearance_kind_id: entity.appearance_kind_id,
        position: entity.position,
        hp: entity.hp,
        max_hp: entity.max_hp,
        power_per_mille: entity.power_per_mille,
        speed: entity.base_speed,
        energy_need: entity.energy_need,
        alerted: entity.alerted.unwrap_or_else(|| {
            runtime_definition
                .awareness
                .as_ref()
                .is_none_or(|awareness| awareness.starts_alerted)
        }),
        nice: entity.nice,
        visible_invisible: entity.visible_invisible,
        visible_weird_mind: entity.visible_weird_mind,
        eldritch_horror_triggered: entity.eldritch_horror_triggered,
        casting_cooldown_remaining: entity.casting_cooldown_remaining,
        observed_player_resistances,
        statuses,
        resistances,
        pack: entity.pack.map(|pack| MonsterPackIdentity {
            id: pack.id,
            leader_id: pack.leader_id,
            role: pack.role,
            behavior: pack.behavior,
        }),
        controller_id: entity.controller_id,
        summon: entity.summon.map(|summon| SummonIdentity {
            owner_id: summon.owner_id,
            source_ability_id: summon.source_ability_id,
            remaining_turns: summon.remaining_turns,
        }),
    })
}

pub(crate) fn item_from_dto(
    item: ItemSaveDto,
    content: &ContentCatalog,
) -> Result<ItemInstance, CoreError> {
    let definition = content
        .item(&item.kind_id)
        .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?;
    let fuel = item.fuel.or_else(|| item_fuel_from_definition(definition));
    validate_item_runtime_state(
        definition,
        item.activation.as_ref(),
        item.charges,
        fuel,
        item.device_recovery_progress,
        item.enchantments,
    )?;
    validate_item_creation_state(
        definition,
        item.origin_kind,
        item.damage_dice_override,
        item.discount_percent,
    )?;
    let rolled_affixes = rolled_affixes_from_save(item.rolled_affixes, &item.affix_ids)?;
    Ok(ItemInstance {
        id: item.id,
        kind_id: item.kind_id,
        quantity: item.quantity,
        inscription: item.inscription,
        origin_actor_kind_id: item.origin_actor_kind_id,
        origin_kind: item.origin_kind,
        damage_dice_override: item.damage_dice_override,
        discount_percent: item.discount_percent,
        quality: item.quality,
        affix_ids: item.affix_ids,
        rolled_affixes,
        enchantments: item.enchantments,
        curse: item.curse,
        activation: item.activation,
        charges: item.charges,
        fuel,
        device_recovery_progress: item.device_recovery_progress,
        location: ItemLocation::Ground(item.position),
    })
}

pub(crate) fn inventory_item_from_dto(
    item: InventoryItemSaveDto,
    content: &ContentCatalog,
) -> Result<ItemInstance, CoreError> {
    let definition = content
        .item(&item.kind_id)
        .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?;
    let fuel = item.fuel.or_else(|| item_fuel_from_definition(definition));
    validate_item_runtime_state(
        definition,
        item.activation.as_ref(),
        item.charges,
        fuel,
        item.device_recovery_progress,
        item.enchantments,
    )?;
    validate_item_creation_state(
        definition,
        item.origin_kind,
        item.damage_dice_override,
        item.discount_percent,
    )?;
    let rolled_affixes = rolled_affixes_from_save(item.rolled_affixes, &item.affix_ids)?;
    Ok(ItemInstance {
        id: item.id,
        kind_id: item.kind_id,
        quantity: item.quantity,
        inscription: item.inscription,
        origin_actor_kind_id: item.origin_actor_kind_id,
        origin_kind: item.origin_kind,
        damage_dice_override: item.damage_dice_override,
        discount_percent: item.discount_percent,
        quality: item.quality,
        affix_ids: item.affix_ids,
        rolled_affixes,
        enchantments: item.enchantments,
        curse: item.curse,
        activation: item.activation,
        charges: item.charges,
        fuel,
        device_recovery_progress: item.device_recovery_progress,
        location: ItemLocation::Inventory,
    })
}

pub(crate) fn equipment_item_from_dto(
    item: EquipmentItemSaveDto,
    content: &ContentCatalog,
) -> Result<ItemInstance, CoreError> {
    let definition = content
        .item(&item.kind_id)
        .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?;
    // Slot ids are body-slot instances; the instance-to-type match is
    // enforced by state validation, which owns the body template.
    if definition.equipment_slot.is_none() {
        return Err(CoreError::InvalidSave("equipment metadata is invalid"));
    }
    let fuel = item.fuel.or_else(|| item_fuel_from_definition(definition));
    validate_item_runtime_state(
        definition,
        item.activation.as_ref(),
        item.charges,
        fuel,
        item.device_recovery_progress,
        item.enchantments,
    )?;
    validate_item_creation_state(
        definition,
        item.origin_kind,
        item.damage_dice_override,
        item.discount_percent,
    )?;
    let rolled_affixes = rolled_affixes_from_save(item.rolled_affixes, &item.affix_ids)?;
    Ok(ItemInstance {
        id: item.id,
        kind_id: item.kind_id,
        quantity: item.quantity,
        inscription: item.inscription,
        origin_actor_kind_id: item.origin_actor_kind_id,
        origin_kind: item.origin_kind,
        damage_dice_override: item.damage_dice_override,
        discount_percent: item.discount_percent,
        quality: item.quality,
        affix_ids: item.affix_ids,
        rolled_affixes,
        enchantments: item.enchantments,
        curse: item.curse,
        activation: item.activation,
        charges: item.charges,
        fuel,
        device_recovery_progress: item.device_recovery_progress,
        location: ItemLocation::Equipped {
            slot_id: item.slot_id,
        },
    })
}

pub(crate) fn carried_item_from_dto(
    item: CarriedItemSaveDto,
    content: &ContentCatalog,
) -> Result<ItemInstance, CoreError> {
    let definition = content
        .item(&item.kind_id)
        .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?;
    let fuel = item.fuel.or_else(|| item_fuel_from_definition(definition));
    validate_item_runtime_state(
        definition,
        item.activation.as_ref(),
        item.charges,
        fuel,
        item.device_recovery_progress,
        item.enchantments,
    )?;
    validate_item_creation_state(
        definition,
        item.origin_kind,
        item.damage_dice_override,
        item.discount_percent,
    )?;
    let rolled_affixes = rolled_affixes_from_save(item.rolled_affixes, &item.affix_ids)?;
    Ok(ItemInstance {
        id: item.id,
        kind_id: item.kind_id,
        quantity: item.quantity,
        inscription: item.inscription,
        origin_actor_kind_id: item.origin_actor_kind_id,
        origin_kind: item.origin_kind,
        damage_dice_override: item.damage_dice_override,
        discount_percent: item.discount_percent,
        quality: item.quality,
        affix_ids: item.affix_ids,
        rolled_affixes,
        enchantments: item.enchantments,
        curse: item.curse,
        activation: item.activation,
        charges: item.charges,
        fuel,
        device_recovery_progress: item.device_recovery_progress,
        location: ItemLocation::CarriedBy {
            actor_id: item.actor_id,
        },
    })
}

fn validate_item_runtime_state(
    definition: &rfb_content::ItemDefinition,
    activation: Option<&ItemActivationDto>,
    charges: Option<ItemChargesDto>,
    fuel: Option<ItemFuelDto>,
    device_recovery_progress: u16,
    enchantments: ItemEnchantmentsDto,
) -> Result<(), CoreError> {
    let configured = definition
        .use_action
        .as_ref()
        .and_then(|action| action.charges);
    let valid = if let Some(generation) = &definition.device_generation {
        match (activation, charges) {
            (Some(activation), Some(charges)) => generation
                .activations
                .iter()
                .find(|profile| profile.id == activation.profile_id)
                .is_some_and(|profile| {
                    let target_spec = TargetSpecDto {
                        modes: profile
                            .target
                            .modes
                            .iter()
                            .map(|mode| match mode {
                                AbilityTargetModeDefinition::Direction => TargetModeDto::Direction,
                                AbilityTargetModeDefinition::Position => TargetModeDto::Position,
                                AbilityTargetModeDefinition::Entity => TargetModeDto::Entity,
                                AbilityTargetModeDefinition::Item => TargetModeDto::Item,
                                AbilityTargetModeDefinition::SelfTarget => {
                                    TargetModeDto::SelfTarget
                                }
                            })
                            .collect(),
                        range: profile.target.range,
                        requires_line_of_effect: profile.target.requires_line_of_effect,
                    };
                    activation.name_key == profile.name_key
                        && activation.cost == profile.charges.cost
                        && activation.device_check_difficulty == profile.device_check_difficulty
                        && activation.target_spec == target_spec
                        && profile.min_depth <= activation.power
                        && activation.power <= profile.max_depth
                        && (profile.charges.minimum..=profile.charges.maximum)
                            .contains(&charges.maximum)
                        && charges.current <= charges.maximum
                }),
            _ => false,
        }
    } else {
        match (configured, activation, charges) {
            (None, None, None) => true,
            (Some(configured), None, Some(charges)) => {
                charges.maximum == configured.maximum && charges.current <= charges.maximum
            }
            _ => false,
        }
    };
    let valid_recovery_progress = match (
        definition
            .device_generation
            .as_ref()
            .and_then(|generation| generation.recovery),
        charges,
    ) {
        (Some(_), Some(charges)) => {
            device_recovery_progress < 1_000
                && (charges.current < charges.maximum || device_recovery_progress == 0)
        }
        _ => device_recovery_progress == 0,
    };
    if !(-15..=15).contains(&enchantments.to_hit)
        || !(-15..=15).contains(&enchantments.to_damage)
        || !(-15..=15).contains(&enchantments.to_armor)
    {
        return Err(CoreError::InvalidSave("item enchantment state is invalid"));
    }
    let valid_fuel = match (item_fuel_from_definition(definition), fuel) {
        (None, None) => true,
        (Some(expected), Some(actual)) => {
            actual.kind == expected.kind
                && actual.maximum == expected.maximum
                && actual.light_radius == expected.light_radius
                && actual.current <= actual.maximum
        }
        _ => false,
    };
    if valid && valid_recovery_progress && valid_fuel {
        Ok(())
    } else {
        Err(CoreError::InvalidSave("item runtime state is invalid"))
    }
}

fn validate_item_creation_state(
    definition: &rfb_content::ItemDefinition,
    origin_kind: Option<ItemOriginKindDto>,
    damage_dice_override: Option<u16>,
    discount_percent: u8,
) -> Result<(), CoreError> {
    let ammunition = definition.tags.iter().any(|tag| tag == "ammunition");
    let origin_is_valid = match origin_kind {
        None => discount_percent == 0,
        Some(ItemOriginKindDto::PlayerMade) => {
            discount_percent == 99 && (ammunition || definition.melee_profile.is_some())
        }
    };
    let damage_override_is_valid =
        damage_dice_override.is_none_or(|dice| (1..=9).contains(&dice) && ammunition);
    if origin_is_valid && damage_override_is_valid {
        Ok(())
    } else {
        Err(CoreError::InvalidSave("item creation state is invalid"))
    }
}

pub(crate) fn player_to_save(
    player_name: &str,
    player: &Actor,
    progress: &CharacterProgress,
    build: Option<&CharacterBuildIdentity>,
    virtues: &[VirtueDto],
) -> PlayerSaveDto {
    PlayerSaveDto {
        id: player.id.clone(),
        name: player_name.to_owned(),
        kind_id: player.kind_id.clone(),
        position: player.position,
        hp: player.hp,
        gold: 0,
        nutrition: rfb_protocol::PLAYER_NUTRITION_BIRTH,
        base_max_hp: player.max_hp,
        base_speed: player.speed,
        energy_need: player.energy_need,
        minor_slow: 0,
        minor_slow_energy: 0,
        chaos_patron_id: None,
        reality_change_ticks: 0,
        pending_mutation_direction: None,
        statuses: player
            .statuses
            .iter()
            .map(StatusInstance::to_save_dto)
            .collect(),
        confusing_strike_ready: false,
        resistances: player.resistances.to_save_dtos(),
        progress: Some(PlayerProgressSaveDto {
            attributes: NaturalAttributeSetSaveDto {
                strength: progress.attributes.strength,
                intelligence: progress.attributes.intelligence,
                wisdom: progress.attributes.wisdom,
                dexterity: progress.attributes.dexterity,
                constitution: progress.attributes.constitution,
                charisma: progress.attributes.charisma,
            },
            maximum_attributes: Some(NaturalAttributeSetSaveDto {
                strength: progress.maximum_attributes.strength,
                intelligence: progress.maximum_attributes.intelligence,
                wisdom: progress.maximum_attributes.wisdom,
                dexterity: progress.maximum_attributes.dexterity,
                constitution: progress.maximum_attributes.constitution,
                charisma: progress.maximum_attributes.charisma,
            }),
            attribute_potentials: NaturalAttributeSetSaveDto {
                strength: progress.attribute_potentials.strength,
                intelligence: progress.attribute_potentials.intelligence,
                wisdom: progress.attribute_potentials.wisdom,
                dexterity: progress.attribute_potentials.dexterity,
                constitution: progress.attribute_potentials.constitution,
                charisma: progress.attribute_potentials.charisma,
            },
            experience: progress.experience,
            maximum_experience: progress.maximum_experience,
            life_force: progress.life_force,
            level: progress.level,
            max_level: progress.max_level,
            pending_attribute_increases: progress.pending_attribute_increases,
            hp_progression: progress.hp_progression.clone(),
            skills: progress
                .skills
                .iter()
                .map(|(id, skill)| SkillProgressSaveDto {
                    id: id.clone(),
                    current: skill.current,
                    maximum: skill.maximum,
                    base: skill.base,
                    growth_per_ten_levels: skill.growth_per_ten_levels,
                })
                .collect(),
        }),
        active_mutation_ids: progress.active_mutation_ids.iter().cloned().collect(),
        locked_mutation_ids: progress.locked_mutation_ids.iter().cloned().collect(),
        virtues: virtues.to_vec(),
        build: build.map(|build| PlayerBuildSaveDto {
            build_id: build.build_id.clone(),
            race_id: build.race_id.clone(),
            class_id: build.class_id.clone(),
            personality_id: build.personality_id.clone(),
        }),
        resources: Vec::new(),
        bonus_spell_learning_capacity: 0,
        learned_ability_ids: Vec::new(),
        ability_progress: Vec::new(),
        summon_command: Default::default(),
        recall: None,
        riding_actor_id: None,
        // Filled by the game's save path, which owns the body template.
        body_slots: Vec::new(),
    }
}

pub(crate) fn actors_to_save(entities: &[Actor]) -> Vec<ActorSaveDto> {
    let mut entities = entities
        .iter()
        .map(|entity| ActorSaveDto {
            id: entity.id.clone(),
            kind_id: entity.kind_id.clone(),
            appearance_kind_id: entity.appearance_kind_id.clone(),
            position: entity.position,
            hp: entity.hp,
            max_hp: entity.max_hp,
            power_per_mille: entity.power_per_mille,
            base_speed: entity.speed,
            energy_need: entity.energy_need,
            alerted: Some(entity.alerted),
            nice: entity.nice,
            visible_invisible: entity.visible_invisible,
            visible_weird_mind: entity.visible_weird_mind,
            eldritch_horror_triggered: entity.eldritch_horror_triggered,
            casting_cooldown_remaining: entity.casting_cooldown_remaining,
            observed_player_resistances: entity
                .observed_player_resistances
                .iter()
                .map(|(damage_type, level)| ResistanceSaveDto {
                    damage_type: (*damage_type).into(),
                    level: (*level).into(),
                })
                .collect(),
            statuses: entity
                .statuses
                .iter()
                .map(StatusInstance::to_save_dto)
                .collect(),
            resistances: entity.resistances.to_save_dtos(),
            pack: entity.pack.as_ref().map(|pack| MonsterPackSaveDto {
                id: pack.id.clone(),
                leader_id: pack.leader_id.clone(),
                role: pack.role,
                behavior: pack.behavior,
            }),
            controller_id: entity.controller_id.clone(),
            summon: entity.summon.as_ref().map(|summon| SummonSaveDto {
                owner_id: summon.owner_id.clone(),
                source_ability_id: summon.source_ability_id.clone(),
                remaining_turns: summon.remaining_turns,
            }),
        })
        .collect::<Vec<_>>();
    entities.sort_by(|left, right| left.id.cmp(&right.id));
    entities
}

fn statuses_from_save(mut statuses: Vec<StatusSaveDto>) -> Result<Vec<StatusInstance>, CoreError> {
    statuses.sort_by(|left, right| left.kind_id.cmp(&right.kind_id));
    let mut seen = BTreeSet::new();
    statuses
        .into_iter()
        .map(|status| {
            if !valid_rule_id(&status.kind_id)
                || !seen.insert(status.kind_id.clone())
                || status.intensity == 0
                || status.remaining_ticks == 0
                || status
                    .source_id
                    .as_deref()
                    .is_some_and(|source| source.is_empty() || source.len() > 128)
                || status.incoming_damage_percent > 100
            {
                return Err(CoreError::InvalidSave("actor status state is invalid"));
            }
            let mut granted_resistances = BTreeMap::new();
            for resistance in status.granted_resistances {
                let damage_type = DamageType::from(resistance.damage_type);
                let level = ResistanceLevel::from(resistance.level);
                if level == ResistanceLevel::Normal
                    || granted_resistances.insert(damage_type, level).is_some()
                {
                    return Err(CoreError::InvalidSave(
                        "actor status resistance state is invalid",
                    ));
                }
            }
            if status
                .granted_brands
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            {
                return Err(CoreError::InvalidSave(
                    "actor status brand state is invalid",
                ));
            }
            let granted_brands = status
                .granted_brands
                .into_iter()
                .map(weapon_brand)
                .collect();
            if status
                .granted_status_immunities
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            {
                return Err(CoreError::InvalidSave(
                    "actor status immunity state is invalid",
                ));
            }
            Ok(StatusInstance {
                kind_id: status.kind_id,
                intensity: status.intensity,
                remaining_ticks: status.remaining_ticks,
                source_id: status.source_id,
                granted_resistances,
                granted_brands,
                granted_modifiers: status.granted_modifiers,
                granted_equipment_bonuses: status.granted_equipment_bonuses,
                granted_status_immunities: status.granted_status_immunities.into_iter().collect(),
                granted_race_id: status.granted_race_id,
                grants_wall_passage: status.grants_wall_passage,
                incoming_damage_percent: status.incoming_damage_percent,
            })
        })
        .collect()
}

fn resistances_from_save(
    resistances: Vec<ResistanceSaveDto>,
) -> Result<ResistanceProfile, CoreError> {
    let mut profile = ResistanceProfile::default();
    let mut seen = BTreeSet::new();
    for resistance in resistances {
        let damage_type = DamageType::from(resistance.damage_type);
        let level = ResistanceLevel::from(resistance.level);
        if !seen.insert(damage_type) || level == ResistanceLevel::Normal {
            return Err(CoreError::InvalidSave("actor resistance state is invalid"));
        }
        profile.set(damage_type, level);
    }
    Ok(profile)
}

fn observed_resistances_from_save(
    resistances: Vec<ResistanceSaveDto>,
) -> Result<BTreeMap<DamageType, ResistanceLevel>, CoreError> {
    let mut observed = BTreeMap::new();
    for resistance in resistances {
        let damage_type = DamageType::from(resistance.damage_type);
        let level = ResistanceLevel::from(resistance.level);
        if observed.insert(damage_type, level).is_some() {
            return Err(CoreError::InvalidSave(
                "monster resistance memory is invalid",
            ));
        }
    }
    Ok(observed)
}

fn valid_rule_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
}

fn rolled_affixes_to_save(rolled_affixes: &[RolledAffixState]) -> Vec<RolledAffixSaveDto> {
    rolled_affixes
        .iter()
        .map(|rolled| {
            let properties = &rolled.properties;
            RolledAffixSaveDto {
                affix_id: rolled.affix_id.clone(),
                modifiers: stat_modifiers_to_dto(&properties.modifiers),
                equipment_bonuses: equipment_bonuses_to_dto(&properties.equipment_bonuses),
                resistances: properties
                    .resistances
                    .iter()
                    .map(|(damage_type, level)| ResistanceDto {
                        damage_type: damage_type_dto(*damage_type),
                        level: resistance_level_dto(*level),
                    })
                    .collect(),
                status_immunities: properties.status_immunities.clone(),
                slays: properties
                    .slays
                    .iter()
                    .map(|(target, level)| SlayDto {
                        target: slay_target_dto(*target),
                        level: slay_level_dto(*level),
                    })
                    .collect(),
                brands: properties
                    .brands
                    .iter()
                    .copied()
                    .map(weapon_brand_dto)
                    .collect(),
                passives: properties
                    .passives
                    .iter()
                    .copied()
                    .map(equipment_passive_dto)
                    .collect(),
            }
        })
        .collect()
}

fn rolled_affixes_from_save(
    rolled_affixes: Vec<RolledAffixSaveDto>,
    affix_ids: &[String],
) -> Result<Vec<RolledAffixState>, CoreError> {
    if rolled_affixes
        .windows(2)
        .any(|pair| pair[0].affix_id >= pair[1].affix_id)
    {
        return Err(CoreError::InvalidSave(
            "rolled affix instance state is invalid",
        ));
    }
    rolled_affixes
        .into_iter()
        .map(|rolled| {
            if !valid_rule_id(&rolled.affix_id)
                || affix_ids.binary_search(&rolled.affix_id).is_err()
                || rolled
                    .resistances
                    .windows(2)
                    .any(|pair| pair[0].damage_type >= pair[1].damage_type)
                || rolled
                    .status_immunities
                    .windows(2)
                    .any(|pair| pair[0] >= pair[1])
                || rolled
                    .status_immunities
                    .iter()
                    .any(|status_id| !valid_rule_id(status_id))
                || rolled
                    .slays
                    .windows(2)
                    .any(|pair| pair[0].target >= pair[1].target)
                || rolled.brands.windows(2).any(|pair| pair[0] >= pair[1])
                || rolled.passives.windows(2).any(|pair| pair[0] >= pair[1])
            {
                return Err(CoreError::InvalidSave(
                    "rolled affix instance state is invalid",
                ));
            }
            let mut resistances = BTreeMap::new();
            for resistance in rolled.resistances {
                let Some(damage_type) = actor_damage_type(resistance.damage_type) else {
                    return Err(CoreError::InvalidSave(
                        "rolled affix instance state is invalid",
                    ));
                };
                let Some(level) = actor_resistance_level(resistance.level) else {
                    return Err(CoreError::InvalidSave(
                        "rolled affix instance state is invalid",
                    ));
                };
                resistances.insert(damage_type, level);
            }
            let properties = AffixPropertyBundleDefinition {
                modifiers: stat_modifiers_from_dto(rolled.modifiers),
                equipment_bonuses: equipment_bonuses_from_dto(rolled.equipment_bonuses),
                resistances,
                status_immunities: rolled.status_immunities,
                slays: rolled
                    .slays
                    .into_iter()
                    .map(|slay| (slay_target(slay.target), slay_level(slay.level)))
                    .collect(),
                brands: rolled.brands.into_iter().map(weapon_brand).collect(),
                passives: rolled.passives.into_iter().map(equipment_passive).collect(),
            };
            if affix_property_bundle_out_of_range(&properties)
                || properties == AffixPropertyBundleDefinition::default()
            {
                return Err(CoreError::InvalidSave(
                    "rolled affix instance state is invalid",
                ));
            }
            Ok(RolledAffixState {
                affix_id: rolled.affix_id,
                properties,
            })
        })
        .collect()
}

fn stat_modifiers_to_dto(modifiers: &StatModifiers) -> StatModifiersDto {
    StatModifiersDto {
        attack: modifiers.attack,
        defense: modifiers.defense,
        max_hp: modifiers.max_hp,
        strength: modifiers.strength,
        intelligence: modifiers.intelligence,
        wisdom: modifiers.wisdom,
        dexterity: modifiers.dexterity,
        constitution: modifiers.constitution,
        charisma: modifiers.charisma,
        speed: modifiers.speed,
        spell_power_bonus: modifiers.spell_power_bonus,
    }
}

fn stat_modifiers_from_dto(modifiers: StatModifiersDto) -> StatModifiers {
    StatModifiers {
        attack: modifiers.attack,
        defense: modifiers.defense,
        max_hp: modifiers.max_hp,
        strength: modifiers.strength,
        intelligence: modifiers.intelligence,
        wisdom: modifiers.wisdom,
        dexterity: modifiers.dexterity,
        constitution: modifiers.constitution,
        charisma: modifiers.charisma,
        speed: modifiers.speed,
        spell_power_bonus: modifiers.spell_power_bonus,
    }
}

fn equipment_bonuses_to_dto(bonuses: &EquipmentBonuses) -> EquipmentBonusesDto {
    EquipmentBonusesDto {
        melee_attacks: bonuses.melee_attacks,
        melee_skill: bonuses.melee_skill,
        melee_damage: bonuses.melee_damage,
        ranged_skill: bonuses.ranged_skill,
        throwing_skill: bonuses.throwing_skill,
        device_skill: bonuses.device_skill,
        saving_throw_skill: bonuses.saving_throw_skill,
        stealth_skill: bonuses.stealth_skill,
        search_skill: bonuses.search_skill,
        perception_skill: bonuses.perception_skill,
        disarming_skill: bonuses.disarming_skill,
        digging_skill: bonuses.digging_skill,
        infravision: bonuses.infravision,
        light_radius: bonuses.light_radius,
    }
}

fn equipment_bonuses_from_dto(bonuses: EquipmentBonusesDto) -> EquipmentBonuses {
    EquipmentBonuses {
        melee_attacks: bonuses.melee_attacks,
        melee_skill: bonuses.melee_skill,
        melee_damage: bonuses.melee_damage,
        ranged_skill: bonuses.ranged_skill,
        throwing_skill: bonuses.throwing_skill,
        device_skill: bonuses.device_skill,
        saving_throw_skill: bonuses.saving_throw_skill,
        stealth_skill: bonuses.stealth_skill,
        search_skill: bonuses.search_skill,
        perception_skill: bonuses.perception_skill,
        disarming_skill: bonuses.disarming_skill,
        digging_skill: bonuses.digging_skill,
        infravision: bonuses.infravision,
        light_radius: bonuses.light_radius,
    }
}

fn affix_property_bundle_out_of_range(properties: &AffixPropertyBundleDefinition) -> bool {
    let modifiers = &properties.modifiers;
    let bonuses = &properties.equipment_bonuses;
    [
        modifiers.attack,
        modifiers.defense,
        modifiers.max_hp,
        bonuses.melee_skill,
        bonuses.melee_damage,
        bonuses.ranged_skill,
        bonuses.throwing_skill,
        bonuses.device_skill,
        bonuses.saving_throw_skill,
        bonuses.stealth_skill,
        bonuses.search_skill,
        bonuses.perception_skill,
        bonuses.disarming_skill,
        bonuses.digging_skill,
    ]
    .into_iter()
    .any(|value| !(-1_000_000..=1_000_000).contains(&value))
        || [
            modifiers.strength,
            modifiers.intelligence,
            modifiers.wisdom,
            modifiers.dexterity,
            modifiers.constitution,
            modifiers.charisma,
            modifiers.speed,
            modifiers.spell_power_bonus,
        ]
        .into_iter()
        .any(|value| !(-100..=100).contains(&value))
        || !(-8..=8).contains(&bonuses.melee_attacks)
        || !(-64..=64).contains(&bonuses.infravision)
        || !(-8..=8).contains(&bonuses.light_radius)
}

const fn damage_type_dto(value: ActorDamageType) -> DamageTypeDto {
    match value {
        ActorDamageType::Physical => DamageTypeDto::Physical,
        ActorDamageType::Acid => DamageTypeDto::Acid,
        ActorDamageType::Electricity => DamageTypeDto::Electricity,
        ActorDamageType::Fire => DamageTypeDto::Fire,
        ActorDamageType::Cold => DamageTypeDto::Cold,
        ActorDamageType::Poison => DamageTypeDto::Poison,
        ActorDamageType::Light => DamageTypeDto::Light,
        ActorDamageType::Dark => DamageTypeDto::Dark,
        ActorDamageType::Blindness => DamageTypeDto::Blindness,
        ActorDamageType::Fear => DamageTypeDto::Fear,
        ActorDamageType::Confusion => DamageTypeDto::Confusion,
        ActorDamageType::Nether => DamageTypeDto::Nether,
        ActorDamageType::Nexus => DamageTypeDto::Nexus,
        ActorDamageType::Sound => DamageTypeDto::Sound,
        ActorDamageType::Shards => DamageTypeDto::Shards,
        ActorDamageType::Chaos => DamageTypeDto::Chaos,
        ActorDamageType::Disenchant => DamageTypeDto::Disenchant,
        ActorDamageType::Time => DamageTypeDto::Time,
        ActorDamageType::Mana => DamageTypeDto::Mana,
        ActorDamageType::Gravity => DamageTypeDto::Gravity,
        ActorDamageType::Inertia => DamageTypeDto::Inertia,
        ActorDamageType::Plasma => DamageTypeDto::Plasma,
        ActorDamageType::Force => DamageTypeDto::Force,
        ActorDamageType::Nuke => DamageTypeDto::Nuke,
        ActorDamageType::Disintegrate => DamageTypeDto::Disintegrate,
        ActorDamageType::Storm => DamageTypeDto::Storm,
        ActorDamageType::HolyFire => DamageTypeDto::HolyFire,
        ActorDamageType::HellFire => DamageTypeDto::HellFire,
        ActorDamageType::Ice => DamageTypeDto::Ice,
        ActorDamageType::Water => DamageTypeDto::Water,
        ActorDamageType::Psi => DamageTypeDto::Psi,
        ActorDamageType::Curse => DamageTypeDto::Curse,
    }
}

const fn actor_damage_type(value: DamageTypeDto) -> Option<ActorDamageType> {
    Some(match value {
        DamageTypeDto::Physical => ActorDamageType::Physical,
        DamageTypeDto::Acid => ActorDamageType::Acid,
        DamageTypeDto::Electricity => ActorDamageType::Electricity,
        DamageTypeDto::Fire => ActorDamageType::Fire,
        DamageTypeDto::Cold => ActorDamageType::Cold,
        DamageTypeDto::Poison => ActorDamageType::Poison,
        DamageTypeDto::Light => ActorDamageType::Light,
        DamageTypeDto::Dark => ActorDamageType::Dark,
        DamageTypeDto::Blindness => ActorDamageType::Blindness,
        DamageTypeDto::Fear => ActorDamageType::Fear,
        DamageTypeDto::Confusion => ActorDamageType::Confusion,
        DamageTypeDto::Nether => ActorDamageType::Nether,
        DamageTypeDto::Nexus => ActorDamageType::Nexus,
        DamageTypeDto::Sound => ActorDamageType::Sound,
        DamageTypeDto::Shards => ActorDamageType::Shards,
        DamageTypeDto::Chaos => ActorDamageType::Chaos,
        DamageTypeDto::Disenchant => ActorDamageType::Disenchant,
        DamageTypeDto::Time => ActorDamageType::Time,
        DamageTypeDto::Mana => ActorDamageType::Mana,
        DamageTypeDto::Gravity => ActorDamageType::Gravity,
        DamageTypeDto::Inertia => ActorDamageType::Inertia,
        DamageTypeDto::Plasma => ActorDamageType::Plasma,
        DamageTypeDto::Force => ActorDamageType::Force,
        DamageTypeDto::Nuke => ActorDamageType::Nuke,
        DamageTypeDto::Disintegrate => ActorDamageType::Disintegrate,
        DamageTypeDto::Storm => ActorDamageType::Storm,
        DamageTypeDto::HolyFire => ActorDamageType::HolyFire,
        DamageTypeDto::HellFire => ActorDamageType::HellFire,
        DamageTypeDto::Ice => ActorDamageType::Ice,
        DamageTypeDto::Water => ActorDamageType::Water,
        DamageTypeDto::Psi => ActorDamageType::Psi,
        DamageTypeDto::Curse => ActorDamageType::Curse,
    })
}

const fn resistance_level_dto(value: ActorResistanceLevel) -> ResistanceLevelDto {
    match value {
        ActorResistanceLevel::Vulnerable => ResistanceLevelDto::Vulnerable,
        ActorResistanceLevel::Resistant => ResistanceLevelDto::Resistant,
        ActorResistanceLevel::Strong => ResistanceLevelDto::Strong,
        ActorResistanceLevel::Immune => ResistanceLevelDto::Immune,
    }
}

const fn actor_resistance_level(value: ResistanceLevelDto) -> Option<ActorResistanceLevel> {
    match value {
        ResistanceLevelDto::Vulnerable => Some(ActorResistanceLevel::Vulnerable),
        ResistanceLevelDto::Normal => None,
        ResistanceLevelDto::Resistant => Some(ActorResistanceLevel::Resistant),
        ResistanceLevelDto::Strong => Some(ActorResistanceLevel::Strong),
        ResistanceLevelDto::Immune => Some(ActorResistanceLevel::Immune),
    }
}

const fn slay_target_dto(value: SlayTarget) -> SlayTargetDto {
    match value {
        SlayTarget::Animal => SlayTargetDto::Animal,
        SlayTarget::Evil => SlayTargetDto::Evil,
        SlayTarget::Good => SlayTargetDto::Good,
        SlayTarget::Living => SlayTargetDto::Living,
        SlayTarget::Human => SlayTargetDto::Human,
        SlayTarget::Undead => SlayTargetDto::Undead,
        SlayTarget::Demon => SlayTargetDto::Demon,
        SlayTarget::Orc => SlayTargetDto::Orc,
        SlayTarget::Troll => SlayTargetDto::Troll,
        SlayTarget::Giant => SlayTargetDto::Giant,
        SlayTarget::Dragon => SlayTargetDto::Dragon,
    }
}

const fn slay_target(value: SlayTargetDto) -> SlayTarget {
    match value {
        SlayTargetDto::Animal => SlayTarget::Animal,
        SlayTargetDto::Evil => SlayTarget::Evil,
        SlayTargetDto::Good => SlayTarget::Good,
        SlayTargetDto::Living => SlayTarget::Living,
        SlayTargetDto::Human => SlayTarget::Human,
        SlayTargetDto::Undead => SlayTarget::Undead,
        SlayTargetDto::Demon => SlayTarget::Demon,
        SlayTargetDto::Orc => SlayTarget::Orc,
        SlayTargetDto::Troll => SlayTarget::Troll,
        SlayTargetDto::Giant => SlayTarget::Giant,
        SlayTargetDto::Dragon => SlayTarget::Dragon,
    }
}

const fn slay_level_dto(value: SlayLevel) -> SlayLevelDto {
    match value {
        SlayLevel::Slay => SlayLevelDto::Slay,
        SlayLevel::Kill => SlayLevelDto::Kill,
    }
}

const fn slay_level(value: SlayLevelDto) -> SlayLevel {
    match value {
        SlayLevelDto::Slay => SlayLevel::Slay,
        SlayLevelDto::Kill => SlayLevel::Kill,
    }
}

const fn weapon_brand_dto(value: WeaponBrand) -> WeaponBrandDto {
    match value {
        WeaponBrand::Acid => WeaponBrandDto::Acid,
        WeaponBrand::Electricity => WeaponBrandDto::Electricity,
        WeaponBrand::Fire => WeaponBrandDto::Fire,
        WeaponBrand::Cold => WeaponBrandDto::Cold,
        WeaponBrand::Poison => WeaponBrandDto::Poison,
        WeaponBrand::Chaos => WeaponBrandDto::Chaos,
    }
}

const fn weapon_brand(value: WeaponBrandDto) -> WeaponBrand {
    match value {
        WeaponBrandDto::Acid => WeaponBrand::Acid,
        WeaponBrandDto::Electricity => WeaponBrand::Electricity,
        WeaponBrandDto::Fire => WeaponBrand::Fire,
        WeaponBrandDto::Cold => WeaponBrand::Cold,
        WeaponBrandDto::Poison => WeaponBrand::Poison,
        WeaponBrandDto::Chaos => WeaponBrand::Chaos,
    }
}

const fn equipment_passive_dto(value: EquipmentPassive) -> EquipmentPassiveDto {
    match value {
        EquipmentPassive::Regeneration => EquipmentPassiveDto::Regeneration,
        EquipmentPassive::SeeInvisible => EquipmentPassiveDto::SeeInvisible,
        EquipmentPassive::Vampiric => EquipmentPassiveDto::Vampiric,
        EquipmentPassive::HoldLife => EquipmentPassiveDto::HoldLife,
        EquipmentPassive::SustainStrength => EquipmentPassiveDto::SustainStrength,
        EquipmentPassive::SustainIntelligence => EquipmentPassiveDto::SustainIntelligence,
        EquipmentPassive::SustainWisdom => EquipmentPassiveDto::SustainWisdom,
        EquipmentPassive::SustainDexterity => EquipmentPassiveDto::SustainDexterity,
        EquipmentPassive::SustainConstitution => EquipmentPassiveDto::SustainConstitution,
        EquipmentPassive::SustainCharisma => EquipmentPassiveDto::SustainCharisma,
    }
}

const fn equipment_passive(value: EquipmentPassiveDto) -> EquipmentPassive {
    match value {
        EquipmentPassiveDto::Regeneration => EquipmentPassive::Regeneration,
        EquipmentPassiveDto::SeeInvisible => EquipmentPassive::SeeInvisible,
        EquipmentPassiveDto::Vampiric => EquipmentPassive::Vampiric,
        EquipmentPassiveDto::HoldLife => EquipmentPassive::HoldLife,
        EquipmentPassiveDto::SustainStrength => EquipmentPassive::SustainStrength,
        EquipmentPassiveDto::SustainIntelligence => EquipmentPassive::SustainIntelligence,
        EquipmentPassiveDto::SustainWisdom => EquipmentPassive::SustainWisdom,
        EquipmentPassiveDto::SustainDexterity => EquipmentPassive::SustainDexterity,
        EquipmentPassiveDto::SustainConstitution => EquipmentPassive::SustainConstitution,
        EquipmentPassiveDto::SustainCharisma => EquipmentPassive::SustainCharisma,
    }
}

pub(crate) fn items_to_save(items: &[ItemInstance]) -> Vec<ItemSaveDto> {
    let mut items = items
        .iter()
        .filter_map(|item| {
            let ItemLocation::Ground(position) = &item.location else {
                return None;
            };
            Some(ItemSaveDto {
                id: item.id.clone(),
                kind_id: item.kind_id.clone(),
                position: *position,
                quantity: item.quantity,
                inscription: item.inscription.clone(),
                origin_actor_kind_id: item.origin_actor_kind_id.clone(),
                origin_kind: item.origin_kind,
                damage_dice_override: item.damage_dice_override,
                discount_percent: item.discount_percent,
                quality: item.quality,
                affix_ids: item.affix_ids.clone(),
                rolled_affixes: rolled_affixes_to_save(&item.rolled_affixes),
                enchantments: item.enchantments,
                curse: item.curse,
                activation: item.activation.clone(),
                charges: item.charges,
                fuel: item.fuel,
                device_recovery_progress: item.device_recovery_progress,
            })
        })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| left.id.cmp(&right.id));
    items
}

pub(crate) fn inventory_to_save(items: &[ItemInstance]) -> Vec<InventoryItemSaveDto> {
    let mut inventory = items
        .iter()
        .filter_map(|item| {
            if item.location != ItemLocation::Inventory {
                return None;
            }
            Some(InventoryItemSaveDto {
                id: item.id.clone(),
                kind_id: item.kind_id.clone(),
                quantity: item.quantity,
                inscription: item.inscription.clone(),
                origin_actor_kind_id: item.origin_actor_kind_id.clone(),
                origin_kind: item.origin_kind,
                damage_dice_override: item.damage_dice_override,
                discount_percent: item.discount_percent,
                quality: item.quality,
                affix_ids: item.affix_ids.clone(),
                rolled_affixes: rolled_affixes_to_save(&item.rolled_affixes),
                enchantments: item.enchantments,
                curse: item.curse,
                activation: item.activation.clone(),
                charges: item.charges,
                fuel: item.fuel,
                device_recovery_progress: item.device_recovery_progress,
            })
        })
        .collect::<Vec<_>>();
    inventory.sort_by(|left, right| left.id.cmp(&right.id));
    inventory
}

pub(crate) fn equipment_to_save(items: &[ItemInstance]) -> Vec<EquipmentItemSaveDto> {
    let mut equipment = items
        .iter()
        .filter_map(|item| {
            let ItemLocation::Equipped { slot_id } = &item.location else {
                return None;
            };
            Some(EquipmentItemSaveDto {
                id: item.id.clone(),
                kind_id: item.kind_id.clone(),
                quantity: item.quantity,
                inscription: item.inscription.clone(),
                origin_actor_kind_id: item.origin_actor_kind_id.clone(),
                origin_kind: item.origin_kind,
                damage_dice_override: item.damage_dice_override,
                discount_percent: item.discount_percent,
                slot_id: slot_id.clone(),
                quality: item.quality,
                affix_ids: item.affix_ids.clone(),
                rolled_affixes: rolled_affixes_to_save(&item.rolled_affixes),
                enchantments: item.enchantments,
                curse: item.curse,
                activation: item.activation.clone(),
                charges: item.charges,
                fuel: item.fuel,
                device_recovery_progress: item.device_recovery_progress,
            })
        })
        .collect::<Vec<_>>();
    equipment.sort_by(|left, right| {
        left.slot_id
            .cmp(&right.slot_id)
            .then_with(|| left.id.cmp(&right.id))
    });
    equipment
}

pub(crate) fn carried_items_to_save(items: &[ItemInstance]) -> Vec<CarriedItemSaveDto> {
    let mut carried = items
        .iter()
        .filter_map(|item| {
            let ItemLocation::CarriedBy { actor_id } = &item.location else {
                return None;
            };
            Some(CarriedItemSaveDto {
                id: item.id.clone(),
                kind_id: item.kind_id.clone(),
                quantity: item.quantity,
                inscription: item.inscription.clone(),
                origin_actor_kind_id: item.origin_actor_kind_id.clone(),
                origin_kind: item.origin_kind,
                damage_dice_override: item.damage_dice_override,
                discount_percent: item.discount_percent,
                actor_id: actor_id.clone(),
                quality: item.quality,
                affix_ids: item.affix_ids.clone(),
                rolled_affixes: rolled_affixes_to_save(&item.rolled_affixes),
                enchantments: item.enchantments,
                curse: item.curse,
                activation: item.activation.clone(),
                charges: item.charges,
                fuel: item.fuel,
                device_recovery_progress: item.device_recovery_progress,
            })
        })
        .collect::<Vec<_>>();
    carried.sort_by(|left, right| {
        left.actor_id
            .cmp(&right.actor_id)
            .then_with(|| left.id.cmp(&right.id))
    });
    carried
}

pub(crate) fn floor_to_save(floor: &FloorState) -> FloorSaveDto {
    FloorSaveDto {
        id: floor.id.clone(),
        dungeon_instance_id: floor.dungeon_instance_id.clone(),
        reproduction_suppressed: floor.reproduction_suppressed,
        player_position: floor.player_position,
        terrain: TerrainSaveDto {
            width: floor.width,
            height: floor.height,
            terrain_ids: floor.terrain.clone(),
            glow: floor.glow.clone(),
        },
        entities: actors_to_save(&floor.entities),
        items: items_to_save(&floor.items),
        gold_piles: gold_piles_to_save(&floor.gold_piles),
        carried_items: carried_items_to_save(&floor.items),
        explored: floor.explored.clone(),
        revealed_terrain: floor.revealed_terrain.iter().copied().collect(),
        connections: floor_connections_to_save(&floor.connections),
        regions: floor_regions_to_save(&floor.regions),
    }
}

pub(crate) fn floor_from_save(
    floor: FloorSaveDto,
    content: &ContentCatalog,
) -> Result<FloorState, CoreError> {
    let expected_len = usize::from(floor.terrain.width) * usize::from(floor.terrain.height);
    if expected_len == 0
        || floor.terrain.terrain_ids.len() != expected_len
        || floor.terrain.glow.len() != expected_len
    {
        return Err(CoreError::InvalidSave("terrain dimensions are invalid"));
    }
    let revealed_terrain = revealed_terrain_from_save(
        floor.revealed_terrain,
        &floor.terrain.terrain_ids,
        floor.terrain.width,
        floor.terrain.height,
        content,
    )?;
    let connections =
        floor_connections_from_save(floor.connections, floor.terrain.width, floor.terrain.height)?;
    let regions = floor_regions_from_save(
        floor.regions,
        floor.terrain.width,
        floor.terrain.height,
        content,
    )?;
    let entities = floor
        .entities
        .into_iter()
        .map(|entity| actor_from_entity(entity, content))
        .collect::<Result<Vec<_>, CoreError>>()?;
    let mut items = floor
        .items
        .into_iter()
        .map(|item| item_from_dto(item, content))
        .collect::<Result<Vec<_>, CoreError>>()?;
    items.extend(
        floor
            .carried_items
            .into_iter()
            .map(|item| carried_item_from_dto(item, content))
            .collect::<Result<Vec<_>, CoreError>>()?,
    );
    Ok(FloorState {
        id: floor.id,
        dungeon_instance_id: floor.dungeon_instance_id,
        reproduction_suppressed: floor.reproduction_suppressed,
        width: floor.terrain.width,
        height: floor.terrain.height,
        terrain: floor.terrain.terrain_ids,
        glow: floor.terrain.glow,
        player_position: floor.player_position,
        entities,
        items,
        gold_piles: gold_piles_from_save(floor.gold_piles),
        explored: floor.explored,
        revealed_terrain,
        connections,
        regions,
    })
}

pub(crate) fn gold_piles_to_save(piles: &[GoldPile]) -> Vec<GoldPileDto> {
    let mut piles = piles
        .iter()
        .map(|pile| GoldPileDto {
            id: pile.id.clone(),
            position: pile.position,
            amount: pile.amount,
            appearance: pile.appearance,
            discovered: pile.discovered,
        })
        .collect::<Vec<_>>();
    piles.sort_by(|left, right| left.id.cmp(&right.id));
    piles
}

pub(crate) fn gold_piles_from_save(piles: Vec<GoldPileDto>) -> Vec<GoldPile> {
    piles
        .into_iter()
        .map(|pile| GoldPile {
            id: pile.id,
            position: pile.position,
            amount: pile.amount,
            appearance: pile.appearance,
            discovered: pile.discovered,
        })
        .collect()
}

pub(crate) fn floor_regions_to_save(regions: &[FloorRegionState]) -> Vec<FloorRegionSaveDto> {
    let mut regions = regions
        .iter()
        .map(|region| FloorRegionSaveDto {
            region_id: region.region_id.clone(),
            theme_id: region.theme_id.clone(),
            encounter_table_id: region.encounter_table_id.clone(),
            loot_table_id: region.loot_table_id.clone(),
            cells: region.cells.clone(),
        })
        .collect::<Vec<_>>();
    regions.sort_by(|left, right| left.region_id.cmp(&right.region_id));
    for region in &mut regions {
        region.cells.sort();
    }
    regions
}

pub(crate) fn floor_regions_from_save(
    regions: Vec<FloorRegionSaveDto>,
    width: u16,
    height: u16,
    content: &ContentCatalog,
) -> Result<Vec<FloorRegionState>, CoreError> {
    let mut restored = regions
        .into_iter()
        .map(|mut region| {
            region.cells.sort();
            if !valid_rule_id(&region.region_id)
                || !valid_rule_id(&region.theme_id)
                || !valid_rule_id(&region.encounter_table_id)
                || !valid_rule_id(&region.loot_table_id)
                || region.cells.is_empty()
                || content
                    .encounter_table(&region.encounter_table_id)
                    .is_none()
                || content.loot_table(&region.loot_table_id).is_none()
                || region.cells.windows(2).any(|pair| pair[0] == pair[1])
                || region.cells.iter().any(|position| {
                    position.x < 0
                        || position.y < 0
                        || position.x >= i32::from(width)
                        || position.y >= i32::from(height)
                })
            {
                return Err(CoreError::InvalidSave("floor region state is invalid"));
            }
            Ok(FloorRegionState {
                region_id: region.region_id,
                theme_id: region.theme_id,
                encounter_table_id: region.encounter_table_id,
                loot_table_id: region.loot_table_id,
                cells: region.cells,
            })
        })
        .collect::<Result<Vec<_>, CoreError>>()?;
    restored.sort_by(|left, right| left.region_id.cmp(&right.region_id));
    let mut occupied = BTreeSet::new();
    if restored
        .windows(2)
        .any(|pair| pair[0].region_id == pair[1].region_id)
        || restored
            .iter()
            .flat_map(|region| region.cells.iter().copied())
            .any(|position| !occupied.insert(position))
    {
        return Err(CoreError::InvalidSave("floor region state is invalid"));
    }
    Ok(restored)
}

pub(crate) fn floor_connections_to_save(
    connections: &[FloorConnectionState],
) -> Vec<FloorConnectionSaveDto> {
    let mut connections = connections
        .iter()
        .map(|connection| FloorConnectionSaveDto {
            id: connection.id.clone(),
            position: connection.position,
            target_floor_id: connection.target_floor_id.clone(),
            target_connection_id: connection.target_connection_id.clone(),
        })
        .collect::<Vec<_>>();
    connections.sort_by(|left, right| left.id.cmp(&right.id));
    connections
}

pub(crate) fn floor_connections_from_save(
    connections: Vec<FloorConnectionSaveDto>,
    width: u16,
    height: u16,
) -> Result<Vec<FloorConnectionState>, CoreError> {
    let mut restored = connections
        .into_iter()
        .map(|connection| {
            if !valid_rule_id(&connection.id)
                || connection.position.x < 0
                || connection.position.y < 0
                || connection.position.x >= i32::from(width)
                || connection.position.y >= i32::from(height)
            {
                return Err(CoreError::InvalidSave("floor connection state is invalid"));
            }
            Ok(FloorConnectionState {
                id: connection.id,
                position: connection.position,
                target_floor_id: connection.target_floor_id,
                target_connection_id: connection.target_connection_id,
            })
        })
        .collect::<Result<Vec<_>, CoreError>>()?;
    restored.sort_by(|left, right| left.id.cmp(&right.id));
    let unique_positions = restored
        .iter()
        .map(|connection| connection.position)
        .collect::<BTreeSet<_>>();
    if restored.windows(2).any(|pair| pair[0].id == pair[1].id)
        || unique_positions.len() != restored.len()
    {
        return Err(CoreError::InvalidSave("floor connection state is invalid"));
    }
    Ok(restored)
}

pub(crate) fn revealed_terrain_from_save(
    positions: Vec<Position>,
    terrain: &[String],
    width: u16,
    height: u16,
    content: &ContentCatalog,
) -> Result<BTreeSet<Position>, CoreError> {
    let mut revealed = BTreeSet::new();
    for position in positions {
        if position.x < 0
            || position.y < 0
            || position.x >= i32::from(width)
            || position.y >= i32::from(height)
            || !revealed.insert(position)
        {
            return Err(CoreError::InvalidSave(
                "revealed terrain knowledge is invalid",
            ));
        }
        let index = position.y as usize * usize::from(width) + position.x as usize;
        let Some(definition) = terrain
            .get(index)
            .and_then(|terrain_id| content.terrain(terrain_id))
        else {
            return Err(CoreError::InvalidSave(
                "revealed terrain knowledge is invalid",
            ));
        };
        if definition.concealed_as_terrain_id.is_none() {
            return Err(CoreError::InvalidSave(
                "revealed terrain knowledge is invalid",
            ));
        }
    }
    Ok(revealed)
}
