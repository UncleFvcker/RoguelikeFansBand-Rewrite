// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    effect::StatusInstance,
    error::CoreError,
    resistance::{DamageType, ResistanceLevel, ResistanceProfile},
    state::{
        Actor, FloorConnectionState, FloorRegionState, FloorState, ItemInstance, ItemLocation,
        MonsterPackIdentity, RolledAffixState, SummonIdentity,
    },
    stats::{CharacterBuildIdentity, CharacterProgress},
};
use rfb_content::{
    ActorDamageType, ActorResistanceLevel, AffixPropertyBundleDefinition, ContentCatalog,
    ContentPosition, EquipmentBonuses, EquipmentPassive, SlayLevel, SlayTarget, StatModifiers,
    WeaponBrand,
};
use rfb_protocol::{
    ActorSaveDto, CarriedItemSaveDto, DamageTypeDto, EquipmentBonusesDto, EquipmentItemSaveDto,
    EquipmentPassiveDto, FloorConnectionSaveDto, FloorRegionSaveDto, FloorSaveDto,
    InventoryItemSaveDto, ItemSaveDto, MonsterPackSaveDto, NaturalAttributeSetSaveDto,
    PlayerBuildSaveDto, PlayerProgressSaveDto, PlayerSaveDto, Position, ResistanceDto,
    ResistanceLevelDto, ResistanceSaveDto, RolledAffixSaveDto, SkillProgressSaveDto, SlayDto,
    SlayLevelDto, SlayTargetDto, StatModifiersDto, StatusSaveDto, SummonSaveDto, TerrainSaveDto,
    WeaponBrandDto,
};

pub(crate) const GENERATED_ITEM_ID_PREFIX: &str = "generated.item.";

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
        position: position_from_content(position),
        hp: max_hp,
        max_hp,
        speed,
        energy_need,
        alerted,
        casting_cooldown_remaining: 0,
        observed_player_resistances: BTreeMap::new(),
        statuses: Vec::new(),
        resistances: ResistanceProfile::default(),
        pack: None,
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
        position,
        hp: max_hp,
        max_hp,
        speed,
        energy_need,
        alerted,
        casting_cooldown_remaining: 0,
        observed_player_resistances: BTreeMap::new(),
        statuses: Vec::new(),
        resistances: ResistanceProfile::default(),
        pack: None,
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
        position: player.position,
        hp: player.hp,
        max_hp: definition.max_hp,
        speed: player.base_speed,
        energy_need: player.energy_need,
        alerted: true,
        casting_cooldown_remaining: 0,
        observed_player_resistances: BTreeMap::new(),
        statuses,
        resistances,
        pack: None,
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
    if entity.max_hp != 0 && entity.max_hp != definition.max_hp {
        return Err(CoreError::InvalidSave("entity base stats are invalid"));
    }
    if entity.base_speed != definition.speed {
        return Err(CoreError::InvalidSave("entity base speed is invalid"));
    }
    let statuses = statuses_from_save(entity.statuses)?;
    let resistances = resistances_from_save(entity.resistances)?;
    let observed_player_resistances =
        observed_resistances_from_save(entity.observed_player_resistances)?;
    Ok(Actor {
        id: entity.id,
        kind_id: entity.kind_id,
        position: entity.position,
        hp: entity.hp,
        max_hp: definition.max_hp,
        speed: entity.base_speed,
        energy_need: entity.energy_need,
        alerted: entity.alerted.unwrap_or_else(|| {
            definition
                .awareness
                .as_ref()
                .is_none_or(|awareness| awareness.starts_alerted)
        }),
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
    content
        .item(&item.kind_id)
        .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?;
    let rolled_affixes = rolled_affixes_from_save(item.rolled_affixes, &item.affix_ids)?;
    Ok(ItemInstance {
        id: item.id,
        kind_id: item.kind_id,
        quantity: item.quantity,
        quality: item.quality,
        affix_ids: item.affix_ids,
        rolled_affixes,
        location: ItemLocation::Ground(item.position),
    })
}

pub(crate) fn inventory_item_from_dto(
    item: InventoryItemSaveDto,
    content: &ContentCatalog,
) -> Result<ItemInstance, CoreError> {
    content
        .item(&item.kind_id)
        .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?;
    let rolled_affixes = rolled_affixes_from_save(item.rolled_affixes, &item.affix_ids)?;
    Ok(ItemInstance {
        id: item.id,
        kind_id: item.kind_id,
        quantity: item.quantity,
        quality: item.quality,
        affix_ids: item.affix_ids,
        rolled_affixes,
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
    let rolled_affixes = rolled_affixes_from_save(item.rolled_affixes, &item.affix_ids)?;
    Ok(ItemInstance {
        id: item.id,
        kind_id: item.kind_id,
        quantity: item.quantity,
        quality: item.quality,
        affix_ids: item.affix_ids,
        rolled_affixes,
        location: ItemLocation::Equipped {
            slot_id: item.slot_id,
        },
    })
}

pub(crate) fn carried_item_from_dto(
    item: CarriedItemSaveDto,
    content: &ContentCatalog,
) -> Result<ItemInstance, CoreError> {
    content
        .item(&item.kind_id)
        .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?;
    let rolled_affixes = rolled_affixes_from_save(item.rolled_affixes, &item.affix_ids)?;
    Ok(ItemInstance {
        id: item.id,
        kind_id: item.kind_id,
        quantity: item.quantity,
        quality: item.quality,
        affix_ids: item.affix_ids,
        rolled_affixes,
        location: ItemLocation::CarriedBy {
            actor_id: item.actor_id,
        },
    })
}

pub(crate) fn player_to_save(
    player: &Actor,
    progress: &CharacterProgress,
    build: Option<&CharacterBuildIdentity>,
) -> PlayerSaveDto {
    PlayerSaveDto {
        id: player.id.clone(),
        kind_id: player.kind_id.clone(),
        position: player.position,
        hp: player.hp,
        base_max_hp: player.max_hp,
        base_speed: player.speed,
        energy_need: player.energy_need,
        statuses: player
            .statuses
            .iter()
            .map(StatusInstance::to_save_dto)
            .collect(),
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
            experience: progress.experience,
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
        build: build.map(|build| PlayerBuildSaveDto {
            build_id: build.build_id.clone(),
            race_id: build.race_id.clone(),
            class_id: build.class_id.clone(),
            personality_id: build.personality_id.clone(),
        }),
        resources: Vec::new(),
        learned_ability_ids: Vec::new(),
        ability_progress: Vec::new(),
        summon_command: Default::default(),
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
            position: entity.position,
            hp: entity.hp,
            max_hp: entity.max_hp,
            base_speed: entity.speed,
            energy_need: entity.energy_need,
            alerted: Some(entity.alerted),
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
            {
                return Err(CoreError::InvalidSave("actor status state is invalid"));
            }
            Ok(StatusInstance {
                kind_id: status.kind_id,
                intensity: status.intensity,
                remaining_ticks: status.remaining_ticks,
                source_id: status.source_id,
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
    }
}

fn equipment_bonuses_to_dto(bonuses: &EquipmentBonuses) -> EquipmentBonusesDto {
    EquipmentBonusesDto {
        melee_attacks: bonuses.melee_attacks,
        melee_skill: bonuses.melee_skill,
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
        DamageTypeDto::Curse => return None,
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
    }
}

const fn weapon_brand(value: WeaponBrandDto) -> WeaponBrand {
    match value {
        WeaponBrandDto::Acid => WeaponBrand::Acid,
        WeaponBrandDto::Electricity => WeaponBrand::Electricity,
        WeaponBrandDto::Fire => WeaponBrand::Fire,
        WeaponBrandDto::Cold => WeaponBrand::Cold,
        WeaponBrandDto::Poison => WeaponBrand::Poison,
    }
}

const fn equipment_passive_dto(value: EquipmentPassive) -> EquipmentPassiveDto {
    match value {
        EquipmentPassive::SeeInvisible => EquipmentPassiveDto::SeeInvisible,
        EquipmentPassive::Telepathy => EquipmentPassiveDto::Telepathy,
        EquipmentPassive::Levitation => EquipmentPassiveDto::Levitation,
        EquipmentPassive::Regeneration => EquipmentPassiveDto::Regeneration,
        EquipmentPassive::HoldLife => EquipmentPassiveDto::HoldLife,
        EquipmentPassive::SustainStrength => EquipmentPassiveDto::SustainStrength,
        EquipmentPassive::SustainIntelligence => EquipmentPassiveDto::SustainIntelligence,
        EquipmentPassive::SustainWisdom => EquipmentPassiveDto::SustainWisdom,
        EquipmentPassive::SustainDexterity => EquipmentPassiveDto::SustainDexterity,
        EquipmentPassive::SustainConstitution => EquipmentPassiveDto::SustainConstitution,
        EquipmentPassive::SustainCharisma => EquipmentPassiveDto::SustainCharisma,
        EquipmentPassive::Blessed => EquipmentPassiveDto::Blessed,
        EquipmentPassive::EasySpell => EquipmentPassiveDto::EasySpell,
        EquipmentPassive::DevicePower => EquipmentPassiveDto::DevicePower,
    }
}

const fn equipment_passive(value: EquipmentPassiveDto) -> EquipmentPassive {
    match value {
        EquipmentPassiveDto::SeeInvisible => EquipmentPassive::SeeInvisible,
        EquipmentPassiveDto::Telepathy => EquipmentPassive::Telepathy,
        EquipmentPassiveDto::Levitation => EquipmentPassive::Levitation,
        EquipmentPassiveDto::Regeneration => EquipmentPassive::Regeneration,
        EquipmentPassiveDto::HoldLife => EquipmentPassive::HoldLife,
        EquipmentPassiveDto::SustainStrength => EquipmentPassive::SustainStrength,
        EquipmentPassiveDto::SustainIntelligence => EquipmentPassive::SustainIntelligence,
        EquipmentPassiveDto::SustainWisdom => EquipmentPassive::SustainWisdom,
        EquipmentPassiveDto::SustainDexterity => EquipmentPassive::SustainDexterity,
        EquipmentPassiveDto::SustainConstitution => EquipmentPassive::SustainConstitution,
        EquipmentPassiveDto::SustainCharisma => EquipmentPassive::SustainCharisma,
        EquipmentPassiveDto::Blessed => EquipmentPassive::Blessed,
        EquipmentPassiveDto::EasySpell => EquipmentPassive::EasySpell,
        EquipmentPassiveDto::DevicePower => EquipmentPassive::DevicePower,
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
                quality: item.quality,
                affix_ids: item.affix_ids.clone(),
                rolled_affixes: rolled_affixes_to_save(&item.rolled_affixes),
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
                quality: item.quality,
                affix_ids: item.affix_ids.clone(),
                rolled_affixes: rolled_affixes_to_save(&item.rolled_affixes),
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
                slot_id: slot_id.clone(),
                quality: item.quality,
                affix_ids: item.affix_ids.clone(),
                rolled_affixes: rolled_affixes_to_save(&item.rolled_affixes),
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
                actor_id: actor_id.clone(),
                quality: item.quality,
                affix_ids: item.affix_ids.clone(),
                rolled_affixes: rolled_affixes_to_save(&item.rolled_affixes),
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
        player_position: floor.player_position,
        terrain: TerrainSaveDto {
            width: floor.width,
            height: floor.height,
            terrain_ids: floor.terrain.clone(),
        },
        entities: actors_to_save(&floor.entities),
        items: items_to_save(&floor.items),
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
        width: floor.terrain.width,
        height: floor.terrain.height,
        terrain: floor.terrain.terrain_ids,
        player_position: floor.player_position,
        entities,
        items,
        explored: floor.explored,
        revealed_terrain,
        connections,
        regions,
    })
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
