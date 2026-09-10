// SPDX-License-Identifier: MPL-2.0
// Game aggregate and rule orchestration.

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::{Arc, OnceLock},
};

use crate::resistance::{
    DamageType, ResistanceLevel, ResistanceProfile, definition_resistance_profile,
};
use crate::{
    action::GameAction,
    check::{
        CheckContext, CheckKind, CheckResult, resolve_check, resolve_check_with_forced_failure,
    },
    combat::{
        adjacent, apply_melee_armor_reduction, monster_melee_skill, rating_to_armor_class,
        rating_to_combat_value, resolve_armored_damage,
    },
    effect::{
        DamageOutcome, DamagePacket, EffectOutcome, EffectSpec, EffectTarget,
        STATUS_BASIC_RESISTANCE, STATUS_BERSERK, STATUS_BLEEDING, STATUS_BLINDNESS,
        STATUS_CONFUSION, STATUS_DEMON_LORD_TRANSFORMATION, STATUS_FEAR, STATUS_FIRE_AURA,
        STATUS_GIANT_STRENGTH, STATUS_HALLUCINATION, STATUS_HASTE, STATUS_HOLD_LIFE,
        STATUS_HOLY_AURA, STATUS_INVENTORY_PROTECTION, STATUS_INVULNERABILITY, STATUS_LEVITATION,
        STATUS_LIGHT_SPEED, STATUS_MAGIC_RESISTANCE, STATUS_NO_AIR, STATUS_PARALYSIS,
        STATUS_PLAYER_POLYMORPH, STATUS_POISON, STATUS_PROTECTION_FROM_EVIL, STATUS_REGENERATION,
        STATUS_SEE_INVISIBLE, STATUS_SIGHT, STATUS_SLEEP, STATUS_SLOW, STATUS_STUN,
        STATUS_SUSTAIN_CHARISMA, STATUS_SUSTAIN_CONSTITUTION, STATUS_SUSTAIN_DEXTERITY,
        STATUS_SUSTAIN_INTELLIGENCE, STATUS_SUSTAIN_STRENGTH, STATUS_SUSTAIN_WISDOM,
        STATUS_TELEPATHY, STATUS_THERMAL_RESISTANCE, STATUS_TRANSCENDENCE, STATUS_TSUYOSHI,
        STATUS_ULTIMATE_RESISTANCE, STATUS_UNDERSTANDING, STATUS_UNWELL, STATUS_VENGEANCE,
        STATUS_WRAITHFORM, StatusApplication, StatusChange, StatusInstance, StatusStacking,
        apply_effect, apply_status, resolve_damage,
    },
    error::CoreError,
    event::{
        DomainEvent, ItemAttributeChange, ProjectileTrace, SelfKnowledgeReport, project_events,
    },
    rng::{RNG_ALGORITHM, RfbRng},
    save::{
        GENERATED_ITEM_ID_PREFIX, actor_from_runtime_spawn, actor_max_hp_is_valid,
        derive_next_item_instance_serial, initial_item_fuel, position_from_content,
    },
    scheduler::{
        INITIAL_MONSTER_ENERGY_NEED, STANDARD_ACTION_COST, energy_gain, gain_energy, spend_energy,
    },
    state::{
        Actor, BASE_ACTOR_POWER_PER_MILLE, CapturedActor, FloorConnectionState, FloorRegionState,
        FloorState, GoldPile, HomeState, ItemInstance, ItemLocation, MonsterPackIdentity,
        ResourcePool, RidingBond, RolledAffixState, ShopState, SummonIdentity, TownState,
    },
    stats::{
        AttributeKind, AttributeSet, CharacterBuildIdentity, CharacterProgress, DerivedStat,
        DerivedStatsPipeline, StatBounds, StatKind, StatLayer,
    },
};
use rfb_content::{
    AbilityDefinition, AbilityDetectSubjectDefinition, AbilityEffectDefinition,
    AbilityGenocideScopeDefinition, AbilityRandomTargetDefinition, AbilitySpellPowerField,
    AbilityStatusStackingDefinition, AbilityTargetDefinition, AbilityTargetModeDefinition,
    ActorDamageType, ActorMovementMode, ActorRole, AffixPropertyBundleDefinition,
    AmmunitionBehaviorDefinition, AmmunitionTypeDefinition, CastingAttribute,
    CastingCapacityFormula, CastingFailureFormula, CastingLearningFormula,
    CastingProfileDefinition, CastingRealmProfileDefinition, CastingStudyMode,
    ClassAbilityDefinition, ContentCatalog, DraconianStrikeModeDefinition,
    DungeonInstanceLifecycle, EncounterEntryDefinition, EncounterTableDefinition, EquipmentBonuses,
    EquipmentPassive, FloorLifecycle, InnatePowerCostScalingCurveDefinition, InnatePowerDefinition,
    ItemAttributeDefinition, ItemCurseSeverityDefinition, ItemCurseTargetDefinition,
    ItemDestructionElement, ItemDeviceGenerationDefinition, ItemEnchantmentRollDefinition,
    ItemShatterEffectDefinition, ItemSummonLevelSourceDefinition, ItemSummonSelectorDefinition,
    ItemUseEffectDefinition, MeleeBlowEffectDefinition, MonsterPackBehavior,
    MutationPeriodicEffectDefinition, PlayerAbilityDefinition, ProceduralLayoutMode,
    ProceduralMazeDefinition, ProceduralPitDefinition, ProceduralRoomGeometryDefinition,
    ProceduralRoomPlacement, ProceduralRoomShape, ProceduralStreamerCandidateDefinition,
    RaceDefinition, RidingWeaponKindDefinition, SkillKind, SlayLevel, SlayTarget,
    SniperShotModeDefinition, StatModifiers, TaskObjectiveKind, TechniqueAttribute,
    TerrainDiggingResolution, TerrainFeatureEntryDefinition, ThemeVaultCandidateDefinition,
    WeaponBrand,
};
use rfb_protocol::{
    AbilityCastResolutionDto, AbilityDetectResolutionDto, AbilityDetectSubjectDto,
    AbilityEffectResolutionDto, AbilityEffectSkipReasonDto, AbilityEffectsResolutionDto,
    AbilityGenocideScopeDto, AbilityProficiencyRankDto, AbilityProgressSaveDto, AbilitySourceDto,
    AbilityStatusChangeDto, AbilityStatusStackingDto, AbilitySummonResolutionDto,
    AbilityTeleportResolutionDto, AbilityTerrainTransformResolutionDto, AttackProfileDto,
    AutoGetModeDto, CampaignStatusDto, CellVisualDto, DamageDiceDto, Direction,
    EquipmentBonusesDto, EquipmentPassiveDto, GameCommandEnvelope, GameUpdate, GoldAppearanceDto,
    HealingResolutionDto, ItemActivationDto, ItemChargesDto, ItemCurseEffectDto,
    ItemCurseRemovalResolutionDto, ItemCurseResolutionDto, ItemCurseSeverityDto,
    ItemEnchantmentComponentResolutionDto, ItemEnchantmentResolutionDto, ItemEnchantmentsDto,
    ItemIdentificationDto, ItemIdentifyResolutionDto, ItemKnowledgeDto, ItemOriginKindDto,
    ItemPropertyDto, ItemQualityDto, LocaleDto, MapScaleDto, MeleeBlowDto, MeleeRoutineDto,
    MonsterAbilityCandidateResolutionDto, MonsterAbilityCastResolutionDto,
    MonsterAbilityDecisionResolutionDto, MonsterAbilityRejectionReasonDto,
    MonsterAbilityTargetResolutionDto, MonsterDisplacementResolutionDto, MonsterPackBehaviorDto,
    MonsterPackRoleDto, PendingAbilityDirectionDto, PendingMutationDirectionDto, Position,
    ProjectileProfileDto, RecallStateDto, ResistanceDto, ResourcePoolSaveDto,
    ResourceRecoveryResolutionDto, RestResolutionDto, RestStopReasonDto, SlayDto, SlayLevelDto,
    SlayTargetDto, StatModifiersDto, SummonCommandDto, SummonCommandModeDto,
    SummonCommandResolutionDto, TargetModeDto, TargetSelection, TargetSpecDto, TaskStatusKindDto,
    ThrowProfileDto, VirtueDto, VirtueKindDto, WeaponBrandDto, WeaponTraitDto,
};

mod abilities;
mod ability_projection;
mod ability_scaling;
mod bounty;
mod casino;
pub(crate) use bounty::BountyOfficeOutcome;
mod capabilities;
mod capture_ball;
mod chaos_patron;
mod damage;
mod death;
mod ego;
pub(crate) use ego::{device_capacity, device_difficulty};
mod environment_combat;
mod floor;
mod gold;
mod ground_item_effects;
mod hunger;
mod initialization;
mod inventory;
mod item_combat;
mod item_curses;
mod item_knowledge;
mod item_use;
mod item_value;
mod lighting;
mod loot;
mod mining;
mod mogaminator;
mod monster_abilities;
mod monster_ai;
mod monster_combat;
mod monster_ecology;
mod movement;
mod museum;
// M2 deliberately establishes this core transaction boundary before any item
// effect is allowed to call it; Polymorph remains blocked until its own batch.
#[allow(dead_code)]
mod mutations;
mod persistence;
mod pet_upkeep;
mod player_abilities;
mod player_combat;
mod player_stats;
mod progression;
mod projectile_geometry;
mod riding_bond;
mod riding_proficiency;
mod snapshot;
mod status_effects;
mod tasks;
mod terrain;
pub(crate) mod town;
pub use museum::SharedMuseum;
mod trait_details;
mod travel;
mod turn;
mod validation;
mod virtues;
mod visibility;
mod weapon_proficiency;
mod wilderness;
mod world;

const CRUSADE_ARREST_ABILITY_ID: &str = "demo.ability.crusade-arrest";

const HUMAN_STR_MUTATION_ID: &str = "rfb.mutation.human-str";
const HUMAN_INT_MUTATION_ID: &str = "rfb.mutation.human-int";
const HUMAN_WIS_MUTATION_ID: &str = "rfb.mutation.human-wis";
const HUMAN_DEX_MUTATION_ID: &str = "rfb.mutation.human-dex";
const HUMAN_CHR_MUTATION_ID: &str = "rfb.mutation.human-chr";

#[cfg(test)]
use abilities::AbilityTargetPlan;
use ability_projection::target_spec_dto;
use capabilities::{
    HealingOutcome, HealingRequest, ResourceRestorationRequest, StatusRemovalRequest,
    apply_healing, apply_resource_restoration, apply_status_application, apply_status_removal,
};
use damage::{
    FatalityPolicy, commit_damage_application, commit_final_player_damage, plan_damage_application,
    process_actor_status_tick, process_actor_status_tick_with, scale_damage_outcome,
};
use ego::materialize_ego_with_rng;
use environment_combat::PlayerTrapOutcome;
use floor::{
    FloorTransitionTarget, RecallUseAction, dungeon_instance_id, dungeon_instance_storage_key,
    floor_dungeon_id, parse_dungeon_instance_ordinal,
};
use inventory::{
    CurseEquippedItemRequest, DeviceRechargeRequest, EquippedItemCurseTarget,
    InventoryItemRechargeOutcome, ItemEnchantmentRequest, ItemIdentificationRequest,
    ItemKnowledgeState, ItemPropertyKnowledgeState, PickUpOutcome, RemoveEquippedCursesRequest,
};
use mogaminator::MogaminatorState;
use player_abilities::AbilityProgress;
#[cfg(test)]
use player_abilities::SPELL_EXP_MASTER;
use player_stats::{
    ResolvedThrowProfile, actor_melee_routine_dto, derived_speed, resolved_melee_blows,
};
use progression::{
    LifeForceRestorationRequest, apply_attribute_drain, apply_attribute_drain_with_amount,
    apply_attribute_restoration, apply_experience_restoration, apply_learning_capacity_increase,
    apply_life_force_restoration, apply_permanent_attribute_drain,
    apply_permanent_attribute_increase, build_definitions, character_skill_progress,
    combine_percentages, initial_resource_pool, profile_resource_maximum, resolve_character_build,
};
use projectile_geometry::{has_line_of_effect, projectile_path_between, rfb_distance};
use status_effects::{
    ability_status_stacking_dto, apply_ability_status_effect, remove_ability_status_effect,
};
use tasks::{
    CampaignState, TaskServiceCompletionOutcome, TaskState, abandoned_task_state,
    initial_task_states, task_applies_to_floor, task_definition, task_floors, task_initial_state,
    task_objectives,
};
use terrain::{DoorBashOutcome, DoorOpenOutcome, TerrainDigOutcome, TrapDisarmOutcome};
#[cfg(test)]
use world::generation::GeneratedRoom;
#[cfg(test)]
use world::geometry::generated_terrain_is_connected;
use world::geometry::{floor_actor_position_is_enterable, floor_position_is_walkable};

pub const DEFAULT_WORLD_ID: &str = "demo.world.middle-earth";
const EQUIPMENT_REGENERATION_INTERVAL_TICKS: u32 = 10;
const BUILT_IN_CONTENT_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/rfb-demo-original.rfbcontent"));
pub const STATE_HASH_SCHEMA_VERSION: u16 = 117;
#[cfg(test)]
const RFB_WARRIOR_BUILD_ID: &str = "demo.build.warrior";
const BASE_THROW_RANGE_BUDGET: u16 = 50;
const MIN_THROW_RANGE: u16 = 2;
const MAX_THROW_RANGE: u16 = 10;
const MAX_REST_TURNS: u16 = 9_999;
const NATURAL_HP_REGENERATION_INTERVAL_TICKS: u32 = 10;
const NATURAL_HP_REGENERATION_FACTOR: u64 = 197;
const NATURAL_HP_REGENERATION_BASE: u64 = 1_442;
const NATURAL_HP_REGENERATION_SCALE: u64 = 65_536;
const MONSTER_REGENERATION_INTERVAL_TICKS: u32 = 100;
const MONSTER_REGENERATION_MAXIMUM: i32 = 400;
const TERRAIN_INTERACTION_DIRECTIONS: [Direction; 8] = [
    Direction::North,
    Direction::NorthEast,
    Direction::East,
    Direction::SouthEast,
    Direction::South,
    Direction::SouthWest,
    Direction::West,
    Direction::NorthWest,
];

fn scale_actor_power(value: i32, power_per_mille: u16) -> i32 {
    i32::try_from(
        i64::from(value)
            .saturating_mul(i64::from(power_per_mille))
            .saturating_div(i64::from(BASE_ACTOR_POWER_PER_MILLE)),
    )
    .unwrap_or_else(|_| {
        if value.is_negative() {
            i32::MIN
        } else {
            i32::MAX
        }
    })
}
const RECHARGING_ITEM_SOURCE_DESTRUCTION_ONE_IN: u16 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
struct GenocideResolution {
    removed_entity_ids: Vec<String>,
    resisted_entity_ids: Vec<String>,
    fatigue_damage: i32,
}

#[derive(Debug, Clone, Copy)]
struct CategorySummonSpec<'a> {
    // summon_specific also places environmental creatures; NO_SUMMON only blocks spells.
    is_spell: bool,
    source_id: &'a str,
    owner_id: &'a str,
    category: &'a str,
    count_dice: u8,
    count_sides: u8,
    count_bonus: u8,
    maximum_count: Option<u8>,
    hostile: bool,
    group_chance_percent: u8,
    group_count_dice: u8,
    group_count_sides: u8,
    group_count_bonus: u8,
    duration_turns: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MonsterAbilityPlan {
    ability: AbilityDefinition,
    base_weight: u32,
    effective_weight: u32,
    enemy_target_count: u16,
    friendly_risk_count: u16,
    target: MonsterAbilityTargetPlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MonsterAbilityPlanResolution {
    target_entity_id: String,
    target_kind_id: String,
    affected_positions: Vec<Position>,
    summon: Option<AbilitySummonResolutionDto>,
    effects: Vec<AbilityEffectResolutionDto>,
    targets: Vec<MonsterAbilityTargetResolutionDto>,
    trace: Option<ProjectileTrace>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActorDeathRecord {
    actor_id: String,
    actor_kind_id: String,
    position: Position,
    credit_player: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MonsterAbilityPlanRejection {
    reason: MonsterAbilityRejectionReasonDto,
    enemy_target_count: u16,
    friendly_risk_count: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MonsterTacticalReason {
    Wounded,
    KeepDistance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActorStepOutcome {
    Moved,
    Interacted,
    Blocked,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MonsterHostileTarget {
    Player {
        entity_id: String,
        kind_id: String,
        position: Position,
    },
    Summon {
        entity_id: String,
        kind_id: String,
        position: Position,
    },
}

impl MonsterHostileTarget {
    fn entity_id(&self) -> &str {
        match self {
            Self::Player { entity_id, .. } | Self::Summon { entity_id, .. } => entity_id,
        }
    }

    fn kind_id(&self) -> &str {
        match self {
            Self::Player { kind_id, .. } | Self::Summon { kind_id, .. } => kind_id,
        }
    }

    const fn position(&self) -> Position {
        match self {
            Self::Player { position, .. } | Self::Summon { position, .. } => *position,
        }
    }

    const fn is_player(&self) -> bool {
        matches!(self, Self::Player { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MonsterAbilityTargetPlan {
    SelfTarget,
    Projectile {
        target: MonsterHostileTarget,
        trace: ProjectileTrace,
    },
    Area {
        target: MonsterHostileTarget,
        trace: ProjectileTrace,
        affected_positions: Vec<Position>,
    },
    JumpDamage {
        affected_positions: Vec<Position>,
        destinations: Vec<Position>,
    },
    Beam {
        target: MonsterHostileTarget,
        trace: ProjectileTrace,
        affected_positions: Vec<Position>,
    },
    Cone {
        target: MonsterHostileTarget,
        trace: ProjectileTrace,
        affected_positions: Vec<Position>,
    },
    TerrainTransform {
        target: MonsterHostileTarget,
        trace: ProjectileTrace,
        center: Position,
        positions: Vec<Position>,
    },
    Summon {
        positions: Vec<Position>,
    },
    SummonCategory {
        candidate_kind_ids: Vec<String>,
        positions: Vec<Position>,
    },
    BanorRupartSplit {
        positions: Vec<Position>,
    },
    BanorRupartMerge {
        counterpart_entity_id: String,
        destination: Position,
    },
    BlinkSelf {
        destinations: Vec<Position>,
    },
    BlinkTarget {
        target: MonsterHostileTarget,
        trace: ProjectileTrace,
        destinations: Vec<Position>,
    },
    EscapeSelf {
        destinations: Vec<Position>,
    },
    DragTarget {
        target: MonsterHostileTarget,
        trace: ProjectileTrace,
        destination: Position,
    },
    BanishTarget {
        target: MonsterHostileTarget,
        trace: ProjectileTrace,
        destinations: Vec<Position>,
    },
    BirdDrop {
        target: MonsterHostileTarget,
        trace: ProjectileTrace,
        destination: Position,
        escape_destinations: Vec<Position>,
    },
}

fn monster_plan_target(target: &MonsterAbilityTargetPlan) -> Option<&MonsterHostileTarget> {
    match target {
        MonsterAbilityTargetPlan::Projectile { target, .. }
        | MonsterAbilityTargetPlan::Area { target, .. }
        | MonsterAbilityTargetPlan::Beam { target, .. }
        | MonsterAbilityTargetPlan::Cone { target, .. }
        | MonsterAbilityTargetPlan::TerrainTransform { target, .. }
        | MonsterAbilityTargetPlan::DragTarget { target, .. }
        | MonsterAbilityTargetPlan::BlinkTarget { target, .. }
        | MonsterAbilityTargetPlan::BanishTarget { target, .. }
        | MonsterAbilityTargetPlan::BirdDrop { target, .. } => Some(target),
        MonsterAbilityTargetPlan::SelfTarget
        | MonsterAbilityTargetPlan::JumpDamage { .. }
        | MonsterAbilityTargetPlan::Summon { .. }
        | MonsterAbilityTargetPlan::SummonCategory { .. }
        | MonsterAbilityTargetPlan::BanorRupartSplit { .. }
        | MonsterAbilityTargetPlan::BanorRupartMerge { .. }
        | MonsterAbilityTargetPlan::BlinkSelf { .. }
        | MonsterAbilityTargetPlan::EscapeSelf { .. } => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DungeonState {
    suppressed: bool,
    recall_floor_id: Option<String>,
    guardian_defeated: bool,
    entrance_guardian_defeated: bool,
    next_instance_ordinal: u32,
    retained_instance_id: Option<String>,
    retained_at_turn: Option<u32>,
}

fn base_dungeon_states(world: &rfb_content::WorldDefinition) -> BTreeMap<String, DungeonState> {
    world
        .dungeons
        .iter()
        .map(|dungeon| {
            (
                dungeon.id.clone(),
                DungeonState {
                    suppressed: false,
                    recall_floor_id: None,
                    guardian_defeated: false,
                    entrance_guardian_defeated: false,
                    next_instance_ordinal: 0,
                    retained_instance_id: None,
                    retained_at_turn: None,
                },
            )
        })
        .collect()
}

impl Game {
    pub(super) fn dungeon_is_active(&self, dungeon_id: &str) -> bool {
        self.dungeon_states
            .get(dungeon_id)
            .is_some_and(|state| !state.suppressed)
    }
}

/// The engine's standard humanoid body: the slot roster every player uses
/// unless their race declares its own `bodySlots`. Single-instance slot ids
/// equal their type so pre-template saves (e.g. `charm`) stay valid.
const STANDARD_BODY_SLOTS: [(&str, &str); 15] = [
    ("weapon", "weapon"),
    ("launcher", "launcher"),
    ("body", "body"),
    ("head", "head"),
    ("shield", "shield"),
    ("cloak", "cloak"),
    ("gloves", "gloves"),
    ("boots", "boots"),
    ("ring-1", "ring"),
    ("ring-2", "ring"),
    ("amulet", "amulet"),
    ("light", "light"),
    ("charm", "charm"),
    ("container", "container"),
    ("tool", "tool"),
];

const DRACONIAN_METAMORPHOSIS_MUTATION_ID: &str = "rfb.mutation.draconian-metamorphosis";
const DRAGON_BODY_SLOTS: [(&str, &str); 10] = [
    ("ring-1", "ring"),
    ("ring-2", "ring"),
    ("ring-3", "ring"),
    ("ring-4", "ring"),
    ("ring-5", "ring"),
    ("ring-6", "ring"),
    ("amulet", "amulet"),
    ("light", "light"),
    ("cloak", "cloak"),
    ("head", "head"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct BodySlot {
    id: String,
    slot_type: String,
}

fn standard_body_slots() -> Vec<BodySlot> {
    STANDARD_BODY_SLOTS
        .iter()
        .map(|(id, slot_type)| BodySlot {
            id: (*id).to_owned(),
            slot_type: (*slot_type).to_owned(),
        })
        .collect()
}

fn dragon_body_slots() -> Vec<BodySlot> {
    DRAGON_BODY_SLOTS
        .iter()
        .map(|(id, slot_type)| BodySlot {
            id: (*id).to_owned(),
            slot_type: (*slot_type).to_owned(),
        })
        .collect()
}

fn body_slots_for_race(race: &RaceDefinition) -> Vec<BodySlot> {
    if race.body_slots.is_empty() {
        return standard_body_slots();
    }
    race.body_slots
        .iter()
        .map(|slot| BodySlot {
            id: slot.id.clone(),
            slot_type: slot.slot_type.clone(),
        })
        .collect()
}

fn resolve_permanent_body_slots(
    content: &ContentCatalog,
    identity: Option<&CharacterBuildIdentity>,
    active_mutation_ids: &BTreeSet<String>,
) -> Result<Vec<BodySlot>, CoreError> {
    let Some(identity) = identity else {
        return Ok(standard_body_slots());
    };
    let (_, race, _, _) = build_definitions(content, identity)?;
    if race.tags.iter().any(|tag| tag == "draconian")
        && active_mutation_ids.contains(DRACONIAN_METAMORPHOSIS_MUTATION_ID)
    {
        return Ok(dragon_body_slots());
    }
    Ok(body_slots_for_race(race))
}

fn body_slot_instance_for_type<'a>(
    body_slots: &'a [BodySlot],
    slot_type: &str,
    occupied: impl Fn(&str) -> bool,
) -> Option<&'a BodySlot> {
    let mut first_match = None;
    for slot in body_slots {
        if slot.slot_type != slot_type {
            continue;
        }
        if first_match.is_none() {
            first_match = Some(slot);
        }
        if !occupied(&slot.id) {
            return Some(slot);
        }
    }
    first_match
}

fn item_can_occupy_slot_type(declared_slot_type: &str, target_slot_type: &str) -> bool {
    declared_slot_type == target_slot_type
        || (declared_slot_type == "weapon" && target_slot_type == "shield")
        || (declared_slot_type == "tool" && target_slot_type == "weapon")
}

fn initial_item_charges(content: &ContentCatalog, kind_id: &str) -> Option<ItemChargesDto> {
    content
        .item(kind_id)
        .and_then(|definition| definition.use_action.as_ref())
        .and_then(|action| action.charges)
        .map(|charges| ItemChargesDto {
            current: charges.initial,
            maximum: charges.maximum,
        })
}

fn item_curse_severity_dto(value: ItemCurseSeverityDefinition) -> ItemCurseSeverityDto {
    match value {
        ItemCurseSeverityDefinition::Normal => ItemCurseSeverityDto::Normal,
        ItemCurseSeverityDefinition::Heavy => ItemCurseSeverityDto::Heavy,
        ItemCurseSeverityDefinition::Permanent => ItemCurseSeverityDto::Permanent,
    }
}

fn device_recharge_resolved_event(
    outcome: InventoryItemRechargeOutcome,
    source_id: String,
    source_is_item: bool,
    source_destroyed: bool,
) -> DomainEvent {
    DomainEvent::DeviceRechargeResolved {
        target_item_id: outcome.target_item_id,
        target_kind_id: outcome.target_kind_id,
        source_id,
        source_is_item,
        attempted: outcome.attempted,
        target_before: outcome.target_before,
        target_after: outcome.target_after,
        succeeded: outcome.succeeded,
        failure_one_in: outcome.failure_one_in,
        failure_roll: outcome.failure_roll,
        source_destroyed,
    }
}

fn initial_item_curse(content: &ContentCatalog, kind_id: &str) -> Option<ItemCurseSeverityDto> {
    content
        .item(kind_id)
        .and_then(|definition| definition.initial_curse)
        .map(item_curse_severity_dto)
}

pub(crate) fn item_device_generation<'a>(
    content: &'a ContentCatalog,
    kind_id: &str,
    affix_ids: &[String],
    profile_id: Option<&str>,
    random_artifact: bool,
) -> Option<&'a ItemDeviceGenerationDefinition> {
    let definition = content.item(kind_id)?;
    // Instance activations outlive an Ego identity and are independent of the base kind.
    if (random_artifact || affix_ids.iter().any(|id| id == "rfb-legacy.affix.blasted"))
        && let Some(id) = profile_id
    {
        return content
            .item_definitions()
            .filter_map(|item| item.device_generation.as_ref())
            .chain(
                content
                    .affix_definitions()
                    .filter_map(|affix| affix.device_generation.as_ref()),
            )
            .find(|generation| {
                generation
                    .activations
                    .iter()
                    .any(|profile| profile.id == id)
            });
    }
    definition
        .device_generation
        .iter()
        .chain(affix_ids.iter().filter_map(|affix_id| {
            content
                .affix(affix_id)
                .and_then(|affix| affix.device_generation.as_ref())
        }))
        .find(|generation| {
            profile_id.is_none_or(|id| {
                generation
                    .activations
                    .iter()
                    .any(|profile| profile.id == id)
            })
        })
}

fn initial_item_runtime_state(
    content: &ContentCatalog,
    rng: &mut RfbRng,
    kind_id: &str,
    affix_ids: &[String],
    depth: u16,
) -> (Option<ItemActivationDto>, Option<ItemChargesDto>) {
    if content.item(kind_id).is_none() {
        return (None, None);
    }
    let Some(generation) = item_device_generation(content, kind_id, affix_ids, None, false) else {
        return (None, initial_item_charges(content, kind_id));
    };
    let power = depth.clamp(1, 100);
    let eligible = generation
        .activations
        .iter()
        .filter(|activation| activation.min_depth <= power && power <= activation.max_depth)
        .collect::<Vec<_>>();
    debug_assert!(
        !eligible.is_empty(),
        "validated device generation must cover every supported depth"
    );
    let total_weight = eligible
        .iter()
        .map(|activation| u64::from(activation.weight))
        .sum::<u64>();
    let mut selection_roll = rng.bounded(total_weight);
    let selected = eligible
        .into_iter()
        .find(|activation| {
            if selection_roll < u64::from(activation.weight) {
                true
            } else {
                selection_roll -= u64::from(activation.weight);
                false
            }
        })
        .expect("validated weighted device activation must select a candidate");
    let capacity_span = u64::from(
        selected
            .charges
            .maximum
            .saturating_sub(selected.charges.minimum),
    ) + 1;
    let maximum = selected.charges.minimum.saturating_add(
        u32::try_from(rng.bounded(capacity_span)).expect("bounded capacity roll must fit u32"),
    );
    let current_span = u64::from(maximum.saturating_sub(selected.charges.cost)) + 1;
    let current = selected.charges.cost.saturating_add(
        u32::try_from(rng.bounded(current_span)).expect("bounded current charge roll must fit u32"),
    );
    // Original equipment effects have their own fixed effect level; object
    // generation depth only selects the profile. Devices retain depth power.
    let power = if selected.rfb_value.is_some() {
        u16::try_from(selected.device_check_difficulty).expect("validated equipment effect level")
    } else {
        power
    };
    (
        Some(ItemActivationDto {
            profile_id: selected.id.clone(),
            name_key: selected.name_key.clone(),
            power,
            cost: selected.charges.cost,
            device_check_difficulty: selected.device_check_difficulty,
            target_spec: target_spec_dto(&selected.target),
        }),
        Some(ItemChargesDto { current, maximum }),
    )
}

fn normalize_player_name(name: &str) -> Option<String> {
    let name = name.trim();
    let length = name.chars().count();
    (length != 0 && length <= 32 && !name.chars().any(char::is_control)).then(|| name.to_owned())
}

#[derive(Debug, Clone)]
pub struct Game {
    content: Arc<ContentCatalog>,
    world_id: String,
    map_scale: MapScaleDto,
    wilderness_position: Option<Position>,
    wilderness_view_offset: Position,
    wilderness_seed: u64,
    wilderness_terrain_cache: BTreeMap<Position, Vec<String>>,
    world_travel_destination: Option<Position>,
    interface_locale: LocaleDto,
    mogaminator: MogaminatorState,
    current_floor_id: String,
    current_dungeon_instance_id: Option<String>,
    reproduction_suppressed: bool,
    stored_floors: BTreeMap<String, FloorState>,
    width: u16,
    height: u16,
    terrain: Vec<String>,
    glow: Vec<bool>,
    daylight_suppressed: Vec<bool>,
    vault_cells: Vec<bool>,
    player_name: String,
    player: Actor,
    riding_actor_id: Option<String>,
    riding_bond: Option<RidingBond>,
    build: Option<CharacterBuildIdentity>,
    body_slots: Vec<BodySlot>,
    progress: CharacterProgress,
    virtues: [VirtueDto; virtues::VIRTUE_SLOT_COUNT],
    resources: BTreeMap<String, ResourcePool>,
    last_visual_cells: Option<Vec<CellVisualDto>>,
    bonus_spell_learning_capacity: u16,
    learned_abilities: BTreeSet<String>,
    ability_learning_order: Vec<String>,
    ability_progress: BTreeMap<String, AbilityProgress>,
    entities: Vec<Actor>,
    items: Vec<ItemInstance>,
    gold: u32,
    fame: u16,
    casino: Option<rfb_protocol::CasinoStateSaveDto>,
    nutrition: u16,
    fasting: bool,
    gold_piles: Vec<GoldPile>,
    item_knowledge: BTreeMap<String, ItemKnowledgeState>,
    item_property_knowledge: BTreeMap<String, ItemPropertyKnowledgeState>,
    task_states: BTreeMap<String, TaskState>,
    bounty_state: bounty::BountyState,
    command_actor_deaths: Vec<ActorDeathRecord>,
    dungeon_states: BTreeMap<String, DungeonState>,
    defeated_limited_actor_counts: BTreeMap<String, u16>,
    generated_artifact_ids: BTreeSet<String>,
    town_states: BTreeMap<String, TownState>,
    shop_states: BTreeMap<String, ShopState>,
    home_states: BTreeMap<String, HomeState>,
    campaign_state: CampaignState,
    summon_command: SummonCommandDto,
    recall: Option<RecallStateDto>,
    confusing_strike_ready: bool,
    sniper_concentration: u8,
    probed_actor_kind_ids: BTreeSet<String>,
    minor_slow: u8,
    minor_slow_energy: u16,
    chaos_patron_id: Option<String>,
    reality_change_ticks: u8,
    pending_mutation_direction: Option<PendingMutationDirectionDto>,
    pending_ability_direction: Option<PendingAbilityDirectionDto>,
    next_item_instance_serial: u64,
    next_gold_pile_serial: u64,
    explored: Vec<bool>,
    revealed_terrain: BTreeSet<Position>,
    floor_connections: Vec<FloorConnectionState>,
    floor_regions: Vec<FloorRegionState>,
    rng: RfbRng,
    revision: u32,
    turn: u32,
    world_tick: u32,
    last_non_melee_fear_aura_tick: Option<u32>,
    last_command_seq: u32,
    debug_ability_casts_succeed: bool,
    debug_recharge_attempts_succeed: bool,
    debug_recharge_attempts_fail: bool,
    debug_recharge_sources_survive: bool,
    debug_recall_delay_turns: Option<u16>,
    debug_item_curses_land: bool,
    debug_item_curses_resisted: bool,
    monster_division_remainders: BTreeMap<String, bool>,
}

impl Game {
    pub fn dispatch(&mut self, envelope: GameCommandEnvelope) -> Result<GameUpdate, CoreError> {
        if envelope.expected_revision != self.revision {
            return Err(CoreError::RevisionMismatch {
                expected: self.revision,
                received: envelope.expected_revision,
            });
        }
        let expected_seq = self.last_command_seq.saturating_add(1);
        if envelope.command_seq != expected_seq {
            return Err(CoreError::CommandSequence {
                expected: expected_seq,
                received: envelope.command_seq,
            });
        }
        if self.campaign_state.status == CampaignStatusDto::Retired {
            return Err(CoreError::CampaignEnded);
        }
        if self.player_is_dead() {
            return Err(CoreError::PlayerDead);
        }

        let mut action = GameAction::from(envelope.command);
        let pending_race_mutation_choice = self.pending_race_mutation_choice();
        let race_mutation_choice_pending = pending_race_mutation_choice.is_some();
        if self.pending_mutation_direction.is_some()
            && !matches!(action, GameAction::ResolveMutationDirection { .. })
            && !(race_mutation_choice_pending
                && matches!(action, GameAction::ChooseRaceMutation { .. }))
        {
            return Err(CoreError::MutationDirectionRequired);
        }
        if self.pending_mutation_direction.is_none()
            && matches!(action, GameAction::ResolveMutationDirection { .. })
        {
            return Err(CoreError::MutationDirectionUnavailable);
        }
        if self.pending_ability_direction.is_some()
            && !matches!(
                action,
                GameAction::ResolveAbilityDirection { .. } | GameAction::CancelAbilityDirection
            )
        {
            return Err(CoreError::AbilityDirectionRequired);
        }
        if self.pending_ability_direction.is_none()
            && matches!(
                action,
                GameAction::ResolveAbilityDirection { .. } | GameAction::CancelAbilityDirection
            )
        {
            return Err(CoreError::AbilityDirectionUnavailable);
        }
        if self.casino.is_some()
            && !matches!(
                action,
                GameAction::Casino { .. } | GameAction::SetInterfaceLocale { .. }
            )
        {
            return Err(CoreError::CasinoInProgress);
        }
        if race_mutation_choice_pending && !matches!(action, GameAction::ChooseRaceMutation { .. })
        {
            return Err(CoreError::RaceMutationChoiceRequired);
        }
        if !race_mutation_choice_pending && matches!(action, GameAction::ChooseRaceMutation { .. })
        {
            return Err(CoreError::RaceMutationChoiceUnavailable);
        }
        if let GameAction::ChooseRaceMutation {
            reward_id,
            mutation_id,
        } = &action
            && pending_race_mutation_choice.as_ref().is_none_or(
                |(pending_reward_id, candidates)| {
                    pending_reward_id != reward_id
                        || !candidates.iter().any(|candidate| candidate == mutation_id)
                },
            )
        {
            return Err(CoreError::RaceMutationChoiceUnavailable);
        }
        let reevaluate_all_mogaminator_items = matches!(
            &action,
            GameAction::ConfigureMogaminator { .. } | GameAction::SetInterfaceLocale { .. }
        );
        let configuring_mogaminator = matches!(&action, GameAction::ConfigureMogaminator { .. });
        let item_property_knowledge_before = self
            .mogaminator
            .enabled
            .then(|| self.item_property_knowledge.clone());
        let item_knowledge_before = self
            .mogaminator
            .enabled
            .then(|| self.item_knowledge.clone());
        let nice_entities_at_command_start = self
            .entities
            .iter()
            .filter(|entity| entity.nice)
            .map(|entity| entity.id.clone())
            .collect::<BTreeSet<_>>();
        self.command_actor_deaths.clear();
        self.validate_runtime_invariants(&action)?;
        self.refresh_daily_bounty_target();
        let base_revision = self.revision;
        let world_tick_before_command = self.world_tick;
        let player_position_before_command = self.player.position;
        let floor_before_command = self.current_floor_id.clone();
        let visible_monster_auras_before_action = self.visible_monster_aura_entity_ids();
        let wilderness_position_before_command = self.wilderness_position;
        let wilderness_view_offset_before_command = self.wilderness_view_offset;
        let light_radius_before_command = self.player_light_radius();
        let see_invisible_sources_before_command = self.player_see_invisible_sources();
        let telepathy_before_command = self.player_has_telepathy();
        let entity_positions_before_command = self
            .entities
            .iter()
            .map(|entity| (entity.id.clone(), entity.position))
            .collect::<BTreeMap<_, _>>();
        let map_scale_before_command = self.map_scale;
        let previous_dimensions = self.projected_dimensions();
        // The world only mutates inside dispatch, so the visuals recorded at
        // the end of the previous command are exactly this command's "before"
        // frame; recomputing them here would be a second full-map pass.
        let previous_visuals = self
            .last_visual_cells
            .take()
            .unwrap_or_else(|| self.visual_cells());
        let mut nature_wrath_before = matches!(
            &action,
            GameAction::CastAbility { ability_id, target }
                if matches!(target, TargetSelection::SelfTarget)
                    && self.content.ability(ability_id).is_some_and(|ability| {
                        matches!(ability.effect, AbilityEffectDefinition::NatureWrath)
                    })
        )
        .then(|| self.clone());
        let mut changed = BTreeSet::new();
        let mut events = Vec::new();
        let mut chaos_patron_event_cursor = 0;
        let mut removed_entities = Vec::new();
        let mut map_translation = None;
        let mut mogaminator_diagnostics = Vec::new();
        let depleted_device_use = matches!(
            &action,
            GameAction::UseItem { item_id, .. } if self.item_charge_is_insufficient(item_id)
        );
        let zero_time_unavailable_item_use = matches!(
            &action,
            GameAction::UseItem {
                item_id,
                target,
                target_glyph,
            }
                if self.item_use_is_zero_time_unavailable(
                    item_id,
                    target.as_ref(),
                    target_glyph.as_deref(),
                )
        );
        let cursed_unequip = matches!(
            &action,
            GameAction::Unequip { slot_id } if self.cursed_equipment_in_slot(slot_id).is_some()
        );
        let cursed_equip_replacement = matches!(
            &action,
            GameAction::Equip { item_id, slot_id }
                if self
                    .cursed_equipment_replaced_by(item_id, slot_id.as_deref())
                    .is_some()
        );
        let unavailable_light_refuel = matches!(
            &action,
            GameAction::RefuelLight {
                target_item_id,
                source_item_id,
            } if self
                .refuel_light_unavailable_reason(target_item_id, source_item_id)
                .is_some()
        );
        let unavailable_recharging_item = matches!(
            &action,
            GameAction::UseItemForRecharge {
                item_id,
                source_item_id,
                target_item_id,
            } if self
                .recharging_item_unavailable_reason(item_id, source_item_id, target_item_id)
                .is_some()
        );
        let world_travel_direction = match &action {
            GameAction::TravelWorld { destination } => {
                self.next_world_travel_direction(*destination)
            }
            _ => None,
        };
        let unavailable_world_travel =
            matches!(&action, GameAction::TravelWorld { .. }) && world_travel_direction.is_none();
        let local_travel_direction = match &action {
            GameAction::TravelLocal { destination } => {
                self.next_local_travel_direction(*destination)
            }
            _ => None,
        };
        let unavailable_local_travel =
            matches!(&action, GameAction::TravelLocal { .. }) && local_travel_direction.is_none();
        let zero_time_unavailable_ability = matches!(
            &action,
            GameAction::CastAbility { ability_id, .. }
                if self.ability_state_unavailable_reason(ability_id).is_some()
        );
        if let Some(direction) = local_travel_direction {
            action = GameAction::Move { direction };
        }
        let auto_get_target = match &action {
            GameAction::AutoGet { object_id } => self.mogaminator_auto_get_position(object_id),
            _ => None,
        };
        if let Some(direction) = auto_get_target.and_then(|target| {
            (target != self.player.position)
                .then(|| self.next_local_travel_direction(target))
                .flatten()
        }) {
            action = GameAction::Move { direction };
        }
        let mut advances_world = !depleted_device_use
            && !zero_time_unavailable_item_use
            && !cursed_unequip
            && !cursed_equip_replacement
            && !unavailable_light_refuel
            && !unavailable_recharging_item
            && !unavailable_world_travel
            && !unavailable_local_travel
            && !zero_time_unavailable_ability
            && !matches!(
                &action,
                GameAction::Retire
                    | GameAction::AcceptTask { .. }
                    | GameAction::BuyFromShop { .. }
                    | GameAction::ClaimTaskReward { .. }
                    | GameAction::DepositAtHome { .. }
                    | GameAction::DismissPets
                    | GameAction::EnterWorldMap { .. }
                    | GameAction::IdentifyAtFacility { .. }
                    | GameAction::ResearchItemAtFacility { .. }
                    | GameAction::ResearchMonsterAtFacility { .. }
                    | GameAction::TeleportToDungeonLevelAtFacility { .. }
                    | GameAction::EatAtInn { .. }
                    | GameAction::AskReputationAtInn { .. }
                    | GameAction::IdentifyAllAtFacility { .. }
                    | GameAction::Casino { .. }
                    | GameAction::UseFacilityService { .. }
                    | GameAction::UseBountyOffice { .. }
                    | GameAction::IncreaseAttribute { .. }
                    | GameAction::ChooseRaceMutation { .. }
                    | GameAction::LeaveWorldMap
                    | GameAction::RenameAtFacility { .. }
                    | GameAction::Rest { .. }
                    | GameAction::SellToShop { .. }
                    | GameAction::StayAtInn { .. }
                    | GameAction::TravelFromInn { .. }
                    | GameAction::WithdrawFromHome { .. }
                    | GameAction::SetSummonCommand { .. }
                    | GameAction::ConfigureMogaminator { .. }
                    | GameAction::AutoGet { .. }
                    | GameAction::PickUp
                    | GameAction::ResolveMogaminatorQuery { .. }
                    | GameAction::ResolveMutationDirection { .. }
                    | GameAction::CancelAbilityDirection
                    | GameAction::InscribeItem { .. }
                    | GameAction::SetInterfaceLocale { .. }
            );
        // Paralysis wastes any world-advancing action: the substituted idle
        // still spends the turn (energy, monster actions, status ticks) but
        // never grants deliberate wait recovery. Zero-time commands and Rest
        // stay available; rest turns tick paralysis down like any status.
        if advances_world && self.player_has_status_kind(STATUS_PARALYSIS) {
            action = GameAction::ParalyzedIdle;
        }
        let projectile_action = matches!(
            &action,
            GameAction::Fire { .. } | GameAction::FireTarget { .. }
        ) || matches!(
            &action,
            GameAction::CastAbility { ability_id, .. }
                if self.content.ability(ability_id).is_some_and(|ability| {
                    matches!(ability.effect, AbilityEffectDefinition::SniperShot { .. })
                })
        );
        let mut action_cost = if map_scale_before_command == MapScaleDto::World && advances_world {
            STANDARD_ACTION_COST.saturating_mul(wilderness::WORLD_MAP_ACTION_MULTIPLIER)
        } else if projectile_action {
            self.player_projectile_profile()
                .map_or_else(|| action.energy_cost(), |profile| profile.energy_cost)
        } else {
            action.energy_cost()
        };
        action_cost = self.player_mutation_action_energy_cost(&action, action_cost);
        if let GameAction::UseItem { item_id, .. } = &action
            && let Some(item) = self.items.iter().find(|item| item.id == *item_id)
            && ego::item_has_ego(&self.content, item, 256)
        {
            action_cost -= action_cost * i32::from(ego::device_pval(item)) / 10;
        }
        let astral_guide_blink = match &action {
            GameAction::CastAbility { ability_id, .. }
                if self.player_has_astral_guide()
                    && self.content.ability(ability_id).is_some_and(|ability| {
                        matches!(ability.effect, AbilityEffectDefinition::BlinkSelf { .. })
                    }) =>
            {
                Some(ability_id.clone())
            }
            _ => None,
        };
        let dimension_door = match &action {
            GameAction::CastAbility { ability_id, .. }
                if !self.player_has_astral_guide()
                    && self.content.ability(ability_id).is_some_and(|ability| {
                        matches!(
                            ability.effect,
                            AbilityEffectDefinition::DimensionDoor { .. }
                        )
                    }) =>
            {
                Some(ability_id.clone())
            }
            _ => None,
        };
        let automatic_pickup_after_move = matches!(&action, GameAction::Move { .. });
        let recover_after_wait = matches!(&action, GameAction::Wait);
        let pet_neglect_allowed = self.pet_upkeep().unsafe_warning();
        let mut turn_advance = 1_u32;
        let mut player_moved = false;
        if advances_world {
            self.decrement_ability_cooldowns(1);
        }
        if (advances_world || matches!(&action, GameAction::Rest { turns } if *turns > 0))
            && !matches!(
                &action,
                GameAction::CastAbility { .. }
                    | GameAction::Fire { .. }
                    | GameAction::FireTarget { .. }
            )
        {
            self.sniper_concentration = 0;
        }

        match action {
            GameAction::AcceptTask {
                facility_id,
                task_id,
            } => match self.accept_task(&facility_id, &task_id) {
                Ok(positions) => {
                    changed.extend(positions);
                    events.push(DomainEvent::TaskAccepted { task_id });
                }
                Err(reason) => events.push(DomainEvent::TaskAcceptUnavailable {
                    facility_id,
                    task_id,
                    reason: reason.to_owned(),
                }),
            },
            GameAction::ClaimTaskReward {
                facility_id,
                task_id,
            } => match self.claim_task_reward(&facility_id, &task_id) {
                Ok(TaskServiceCompletionOutcome::Rewarded(outcome)) => {
                    events.push(DomainEvent::TaskRewarded {
                        item_kind_id: outcome.item_kind_id,
                        quantity: outcome.quantity,
                    })
                }
                Ok(TaskServiceCompletionOutcome::Concluded { floor_id }) => {
                    events.push(DomainEvent::TaskCompleted { floor_id });
                }
                Err(reason) => events.push(DomainEvent::TaskRewardClaimUnavailable {
                    facility_id,
                    task_id,
                    reason: reason.to_owned(),
                }),
            },
            GameAction::AbandonPausedTask { task_id } => {
                if let Some(positions) = self.abandon_paused_task(&task_id) {
                    changed.extend(positions);
                    events.push(DomainEvent::TaskAbandoned {
                        floor_id: task_id.clone(),
                    });
                    events.push(DomainEvent::OneShotFloorClosed { floor_id: task_id });
                } else {
                    events.push(DomainEvent::TaskAbandonUnavailable);
                }
            }
            GameAction::Appraise { item_id } => {
                if let Some((target_kind_id, quality)) = self.appraise_inventory_item(&item_id) {
                    events.push(DomainEvent::ItemAppraised {
                        target_kind_id,
                        quality,
                    });
                } else {
                    events.push(DomainEvent::ItemAppraiseUnavailable);
                }
            }
            GameAction::IncreaseAttribute { attribute } => {
                if let Some((natural, effective, index)) = self.increase_player_attribute(attribute)
                {
                    events.push(DomainEvent::PlayerAttributeIncreased {
                        attribute,
                        natural,
                        effective,
                        index,
                        pending_attribute_increases: self.progress.pending_attribute_increases,
                    });
                } else {
                    events.push(DomainEvent::PlayerAttributeIncreaseUnavailable { attribute });
                }
            }
            GameAction::ChooseRaceMutation {
                reward_id,
                mutation_id,
            } => {
                let chosen = self.choose_race_mutation(&reward_id, &mutation_id, &mut events);
                debug_assert!(
                    chosen,
                    "validated race mutation choice must remain available"
                );
            }
            GameAction::BashDoor { direction } => match self.bash_door(direction) {
                Some(DoorBashOutcome::Succeeded { position }) => {
                    changed.insert(position);
                    events.push(DomainEvent::DoorBashedOpen { position });
                }
                Some(DoorBashOutcome::Failed { position }) => {
                    events.push(DomainEvent::DoorBashFailed { position });
                }
                None => events.push(DomainEvent::DoorBashUnavailable),
            },
            GameAction::BuyFromShop {
                shop_id,
                item_id,
                quantity,
            } => match self.buy_from_shop(&shop_id, &item_id, quantity) {
                Ok(outcome) => events.push(DomainEvent::ShopPurchaseCompleted { outcome }),
                Err(reason) => events.push(DomainEvent::ShopTransactionUnavailable {
                    shop_id,
                    item_id,
                    reason: reason.to_owned(),
                }),
            },
            GameAction::IdentifyAtFacility {
                facility_id,
                item_id,
            } => match self.identify_at_facility(&facility_id, &item_id) {
                Ok(outcome) => events.push(DomainEvent::FacilityItemIdentified { outcome }),
                Err(reason) => events.push(DomainEvent::FacilityIdentifyUnavailable {
                    facility_id,
                    item_id,
                    reason: reason.to_owned(),
                }),
            },
            GameAction::ResearchItemAtFacility {
                facility_id,
                item_id,
            } => match self.research_item_at_facility(&facility_id, &item_id) {
                Ok(outcome) => events.push(DomainEvent::FacilityItemIdentified { outcome }),
                Err(reason) => events.push(DomainEvent::FacilityIdentifyUnavailable {
                    facility_id,
                    item_id,
                    reason: reason.to_owned(),
                }),
            },
            GameAction::IdentifyAllAtFacility { facility_id } => {
                match self.identify_all_at_facility(&facility_id) {
                    Ok(outcome) => events.push(DomainEvent::FacilityItemsIdentified { outcome }),
                    Err(reason) => events.push(DomainEvent::FacilityIdentifyAllUnavailable {
                        facility_id,
                        reason: reason.to_owned(),
                    }),
                }
            }
            GameAction::Casino {
                facility_id,
                action,
            } => {
                if let Err(reason) = self.casino_action(&facility_id, action, &mut events) {
                    events.push(DomainEvent::CasinoUnavailable {
                        facility_id,
                        reason: reason.to_owned(),
                    });
                }
            }
            GameAction::UseFacilityService {
                facility_id,
                service,
                item_id,
                enchantment_steps,
            } => match self.use_town_facility_service(
                &facility_id,
                service,
                item_id.as_deref(),
                enchantment_steps,
                &mut events,
            ) {
                Ok(outcome) => events.push(DomainEvent::FacilityServiceCompleted { outcome }),
                Err(reason) => events.push(DomainEvent::FacilityServiceUnavailable {
                    facility_id,
                    service,
                    reason: reason.to_owned(),
                }),
            },
            GameAction::UseBountyOffice {
                facility_id,
                action,
                item_id,
            } => match self.use_bounty_office(&facility_id, action, item_id.as_deref()) {
                Ok(outcome) => events.push(DomainEvent::BountyOfficeCompleted { outcome }),
                Err(reason) => events.push(DomainEvent::BountyOfficeUnavailable {
                    facility_id,
                    action,
                    reason: reason.to_owned(),
                }),
            },
            GameAction::RenameAtFacility { facility_id, name } => {
                match self.rename_at_facility(&facility_id, &name) {
                    Ok(outcome) => events.push(DomainEvent::FacilityPlayerRenamed { outcome }),
                    Err(reason) => events.push(DomainEvent::FacilityRenameUnavailable {
                        facility_id,
                        reason: reason.to_owned(),
                    }),
                }
            }
            GameAction::EatAtInn { facility_id } => {
                if let Err(reason) = self.eat_at_inn(&facility_id, &mut events) {
                    events.push(DomainEvent::InnFoodUnavailable {
                        facility_id,
                        reason: reason.to_owned(),
                    });
                }
            }
            GameAction::AskReputationAtInn { facility_id } => {
                if let Err(reason) = self.ask_reputation_at_inn(&facility_id, &mut events) {
                    events.push(DomainEvent::InnReputationUnavailable {
                        facility_id,
                        reason: reason.to_owned(),
                    });
                }
            }
            GameAction::ResearchMonsterAtFacility {
                facility_id,
                actor_kind_id,
            } => {
                if let Err(reason) =
                    self.research_monster_at_facility(&facility_id, &actor_kind_id, &mut events)
                {
                    events.push(DomainEvent::MonsterResearchUnavailable {
                        facility_id,
                        reason: reason.to_owned(),
                    });
                }
            }
            GameAction::TeleportToDungeonLevelAtFacility {
                facility_id,
                dungeon_id,
                depth,
            } => {
                match self.teleport_to_dungeon_level_at_facility(&facility_id, &dungeon_id, depth) {
                    Ok(outcome) => events.push(DomainEvent::FacilityServiceCompleted { outcome }),
                    Err(reason) => events.push(DomainEvent::FacilityServiceUnavailable {
                        facility_id,
                        service: rfb_protocol::FacilityServiceKindDto::Recall,
                        reason: reason.to_owned(),
                    }),
                }
            }
            GameAction::StayAtInn { facility_id } => match self.stay_at_inn(&facility_id) {
                Ok(outcome) => events.push(DomainEvent::InnStayCompleted { outcome }),
                Err(reason) => events.push(DomainEvent::InnStayUnavailable {
                    facility_id,
                    reason: reason.to_owned(),
                }),
            },
            GameAction::TravelFromInn {
                facility_id,
                destination_town_id,
            } => {
                if let Some(reason) =
                    self.inn_travel_unavailable_reason(&facility_id, &destination_town_id)
                {
                    events.push(DomainEvent::InnTravelUnavailable {
                        facility_id,
                        destination_town_id,
                        reason: reason.to_owned(),
                    });
                } else {
                    let outcome = self.travel_from_inn(&facility_id, &destination_town_id)?;
                    events.push(DomainEvent::InnTravelCompleted { outcome });
                }
            }
            GameAction::DepositAtHome {
                facility_id,
                item_id,
                quantity,
            } => match self.deposit_at_home(&facility_id, &item_id, quantity) {
                Ok(outcome) => events.push(DomainEvent::HomeItemDeposited { outcome }),
                Err(reason) => events.push(DomainEvent::HomeTransferUnavailable {
                    facility_id,
                    item_id,
                    reason: reason.to_owned(),
                }),
            },
            GameAction::CloseDoor { direction } => {
                if let Some(position) = self.close_door(direction) {
                    changed.insert(position);
                    events.push(DomainEvent::DoorClosed { position });
                } else {
                    events.push(DomainEvent::DoorCloseUnavailable);
                }
            }
            GameAction::ConfigureMogaminator {
                enabled,
                leave_destroyed_items,
                auto_get_mode,
                locale,
                source,
            } => {
                mogaminator_diagnostics = self.configure_mogaminator(
                    enabled,
                    leave_destroyed_items,
                    auto_get_mode,
                    locale,
                    source,
                );
            }
            GameAction::ResolveMogaminatorQuery { item_id, pick_up } => {
                if let Some(outcome) = self.resolve_mogaminator_query(&item_id, pick_up)? {
                    self.record_pick_up_outcome(outcome, &mut events, &mut changed);
                }
            }
            GameAction::ResolveMutationDirection { direction } => {
                let pending = self.resolve_pending_mutation_direction(
                    direction,
                    &mut events,
                    &mut changed,
                    &mut removed_entities,
                )?;
                self.resume_after_periodic_mutation(
                    pending,
                    &mut events,
                    &mut changed,
                    &mut removed_entities,
                )?;
            }
            GameAction::DestroyItem { item_id, quantity } => {
                let opens_capture_ball = self.items.iter().any(|item| {
                    item.id == item_id
                        && quantity > 0
                        && quantity <= item.quantity
                        && item.captured_actor.is_some()
                        && (item.location == ItemLocation::Inventory
                            || item.location == ItemLocation::Ground(self.player.position))
                        && self.can_destroy_item(item).is_ok()
                });
                if opens_capture_ball {
                    self.force_open_capture_ball(
                        &item_id,
                        self.player.position,
                        false,
                        &mut events,
                        &mut changed,
                    );
                }
                match self.destroy_item(&item_id, quantity) {
                    Ok(outcome) => events.push(DomainEvent::ItemDestroyed {
                        target_kind_id: outcome.kind_id,
                        quantity: outcome.quantity,
                        rule_line: None,
                    }),
                    Err(reason) => events.push(DomainEvent::ItemDestroyUnavailable {
                        item_id,
                        reason: reason.reason().to_owned(),
                        rule_line: None,
                    }),
                }
            }
            GameAction::InscribeItem {
                item_id,
                inscription,
            } => match self.inscribe_item(&item_id, inscription) {
                Ok(outcome) => events.push(DomainEvent::ItemInscribed {
                    target_kind_id: outcome.kind_id,
                    inscription: outcome.inscription,
                    rule_line: None,
                }),
                Err(reason) => events.push(DomainEvent::ItemInscribeUnavailable {
                    item_id,
                    reason: reason.to_owned(),
                }),
            },
            GameAction::Drop { item_ids } => {
                if let Some((stacks, quantity, position)) = self.drop_inventory_items(&item_ids) {
                    changed.insert(position);
                    for item_id in &item_ids {
                        self.force_open_capture_ball(
                            item_id,
                            position,
                            true,
                            &mut events,
                            &mut changed,
                        );
                    }
                    events.push(DomainEvent::ItemsDropped { stacks, quantity });
                } else {
                    events.push(DomainEvent::NoItemsDropped);
                }
            }
            GameAction::DropQuantity { item_id, quantity } => {
                if let Some((stacks, dropped_quantity, position)) =
                    self.drop_inventory_quantity(&item_id, quantity)?
                {
                    changed.insert(position);
                    self.force_open_capture_ball(
                        &item_id,
                        position,
                        true,
                        &mut events,
                        &mut changed,
                    );
                    events.push(DomainEvent::ItemsDropped {
                        stacks,
                        quantity: dropped_quantity,
                    });
                } else {
                    events.push(DomainEvent::NoItemsDropped);
                }
            }
            GameAction::Equip { item_id, slot_id } => {
                if let Some((target_kind_id, slot_id, severity)) =
                    self.cursed_equipment_replaced_by(&item_id, slot_id.as_deref())
                {
                    events.push(DomainEvent::ItemUnequipCursed {
                        target_kind_id,
                        slot_id,
                        severity,
                    });
                } else if let Some(outcome) =
                    self.equip_inventory_item(&item_id, slot_id.as_deref())
                {
                    self.refresh_player_resource_maxima();
                    let discovered_affix_ids = outcome.discovered_affix_ids.clone();
                    let equipped_kind_id = outcome.kind_id.clone();
                    events.push(DomainEvent::ItemEquipped {
                        target_kind_id: outcome.kind_id,
                        slot_id: outcome.slot_id,
                        replaced_kind_id: outcome.replaced_kind_id,
                    });
                    for affix_id in discovered_affix_ids {
                        let property_name_key = self
                            .content
                            .affix(&affix_id)
                            .expect("equipped affix must remain available")
                            .name_key
                            .clone();
                        events.push(DomainEvent::ItemPropertyDiscovered {
                            target_kind_id: equipped_kind_id.clone(),
                            property_name_key,
                        });
                    }
                } else {
                    events.push(DomainEvent::ItemEquipUnavailable);
                }
            }
            GameAction::CastAbility { ability_id, target } => {
                self.resolve_player_ability(
                    &ability_id,
                    target,
                    &mut events,
                    &mut changed,
                    &mut removed_entities,
                )?;
                if let Some(branch_roll) = abilities::nature_wrath_direction_roll(&events) {
                    let cast_resolution = events.iter().find_map(|event| match event {
                        DomainEvent::AbilityCastSucceeded { resolution }
                            if resolution.ability_id == ability_id =>
                        {
                            Some(resolution.clone())
                        }
                        _ => None,
                    });
                    if let (Some(before), Some(cast_resolution)) =
                        (nature_wrath_before.take(), cast_resolution)
                    {
                        let advanced_rng = self.rng.clone();
                        *self = before;
                        self.rng = advanced_rng;
                        self.pending_ability_direction = Some(PendingAbilityDirectionDto {
                            ability_id,
                            branch_roll,
                            cast_resolution,
                        });
                        events.clear();
                        changed.clear();
                        removed_entities.clear();
                        advances_world = false;
                        action_cost = 0;
                    }
                }
            }
            GameAction::CancelAbilityDirection => {
                self.pending_ability_direction = None;
            }
            GameAction::ResolveAbilityDirection { direction } => {
                self.resolve_pending_ability_direction(
                    direction,
                    &mut events,
                    &mut changed,
                    &mut removed_entities,
                )?;
            }
            GameAction::Fire { direction } => self.resolve_player_projectile(
                TargetSelection::Direction { direction },
                player_combat::ProjectileMode::Normal,
                &mut events,
                &mut changed,
                &mut removed_entities,
            )?,
            GameAction::FireTarget { target } => self.resolve_player_projectile(
                target,
                player_combat::ProjectileMode::Normal,
                &mut events,
                &mut changed,
                &mut removed_entities,
            )?,
            GameAction::EnterWorldMap { cancel_recall, .. } => {
                if cancel_recall && self.recall_is_active() {
                    self.cancel_recall();
                }
                self.store_visible_town_states();
                self.advance_wilderness_generation();
                self.map_scale = MapScaleDto::World;
            }
            GameAction::LeaveWorldMap => {
                if self.leave_world_map()? && self.wilderness_is_daytime() {
                    events.push(DomainEvent::WildernessInterestingDiscovery);
                }
            }
            GameAction::TravelWorld { destination } => {
                if let Some(direction) = world_travel_direction {
                    self.world_travel_destination = Some(destination);
                    if !self.move_on_world_map(direction, &mut changed) {
                        events.push(DomainEvent::MoveBlocked);
                    } else {
                        player_moved = true;
                        if self.wilderness_position == Some(destination) {
                            self.world_travel_destination = None;
                        }
                        if !self.player_is_dead() && self.roll_wilderness_ambush() {
                            self.activate_wilderness_ambush()?;
                            action_cost = STANDARD_ACTION_COST;
                            events.push(DomainEvent::WildernessAmbushed);
                        }
                    }
                } else {
                    self.world_travel_destination = None;
                    events.push(DomainEvent::MoveBlocked);
                }
            }
            GameAction::TravelLocal { .. } => events.push(DomainEvent::MoveBlocked),
            GameAction::Throw { item_id, direction } => {
                self.throw_inventory_item(
                    &item_id,
                    direction,
                    &mut events,
                    &mut changed,
                    &mut removed_entities,
                )?;
            }
            action @ (GameAction::TraverseStairs | GameAction::AbandonTask) => {
                let abandon_task = matches!(action, GameAction::AbandonTask);
                if let Some(transition) = self.traverse_stairs(abandon_task)? {
                    self.record_floor_transition(transition, &mut events, &mut changed);
                } else {
                    events.push(DomainEvent::FloorTransitionUnavailable);
                }
            }
            GameAction::AbsorbDevice { item_id } => {
                if let Some(outcome) = self.absorb_device(&item_id) {
                    events.push(DomainEvent::DeviceAbsorbed {
                        item_id: outcome.item_id,
                        item_kind_id: outcome.item_kind_id,
                        charges_before: outcome.charges_before,
                        charges_after: outcome.charges_after,
                        drained: outcome.drained,
                        nutrition_before: outcome.nutrition_before,
                        nutrition_after: outcome.nutrition_after,
                    });
                } else {
                    events.push(DomainEvent::DeviceAbsorptionUnavailable { item_id });
                }
            }
            GameAction::UseItem {
                item_id,
                target,
                target_glyph,
            } => {
                self.use_inventory_item(
                    &item_id,
                    target.as_ref(),
                    target_glyph.as_deref(),
                    &mut events,
                    &mut changed,
                    &mut removed_entities,
                )?;
            }
            GameAction::RefuelLight {
                target_item_id,
                source_item_id,
            } => {
                if let Some(reason) =
                    self.refuel_light_unavailable_reason(&target_item_id, &source_item_id)
                {
                    events.push(DomainEvent::LightRefuelUnavailable {
                        target_item_id,
                        source_item_id,
                        reason: reason.to_owned(),
                    });
                } else if let Some(outcome) =
                    self.refuel_equipped_light(&target_item_id, &source_item_id)
                {
                    events.push(DomainEvent::LightRefueled {
                        target_item_id: outcome.target_item_id,
                        target_kind_id: outcome.target_kind_id,
                        source_kind_id: outcome.source_kind_id,
                        amount: outcome.amount,
                        current: outcome.current,
                        maximum: outcome.maximum,
                    });
                }
            }
            GameAction::UseItemForRecharge {
                item_id,
                source_item_id,
                target_item_id,
            } => {
                self.use_recharging_item(&item_id, &source_item_id, &target_item_id, &mut events);
            }
            GameAction::ForgetAbility { ability_id } => {
                match self.forget_player_ability(&ability_id) {
                    Ok(()) => events.push(DomainEvent::AbilityForgotten { ability_id }),
                    Err(reason) => events.push(DomainEvent::AbilityForgetUnavailable {
                        ability_id,
                        reason: reason.to_owned(),
                    }),
                }
            }
            GameAction::StudyAbility {
                book_item_id,
                ability_id,
            } => match self.study_player_ability(&book_item_id, &ability_id) {
                Ok(()) => events.push(DomainEvent::AbilityStudied { ability_id }),
                Err(reason) => events.push(DomainEvent::AbilityStudyUnavailable {
                    target_id: ability_id,
                    reason: reason.to_owned(),
                }),
            },
            GameAction::StudyPrayer { book_item_id } => {
                let target_id = self
                    .items
                    .iter()
                    .find(|item| item.id == book_item_id)
                    .map_or_else(|| book_item_id.clone(), |item| item.kind_id.clone());
                match self.study_random_player_ability(&book_item_id) {
                    Ok(ability_id) => events.push(DomainEvent::AbilityStudied { ability_id }),
                    Err(reason) => events.push(DomainEvent::AbilityStudyUnavailable {
                        target_id,
                        reason: reason.to_owned(),
                    }),
                }
            }
            GameAction::Retire => {
                if let Some(score) = self.retire_campaign() {
                    events.push(DomainEvent::CampaignRetired { score });
                } else {
                    events.push(DomainEvent::CampaignRetireUnavailable);
                }
            }
            GameAction::Rest { turns } => {
                let resolution = self.resolve_player_rest(
                    turns,
                    &mut events,
                    &mut changed,
                    &mut removed_entities,
                )?;
                self.decrement_ability_cooldowns(resolution.completed_turns);
                turn_advance = u32::from(resolution.completed_turns).max(1);
                if matches!(
                    resolution.stop_reason,
                    RestStopReasonDto::FullResources | RestStopReasonDto::TurnLimit
                ) {
                    events.push(DomainEvent::RestCompleted { resolution });
                } else {
                    events.push(DomainEvent::RestInterrupted { resolution });
                }
            }
            GameAction::Wait => events.push(DomainEvent::Waited),
            GameAction::AutoGet { object_id } => {
                self.apply_player_floor_item_knowledge();
                let valid_target =
                    auto_get_target.is_some_and(|target| target == self.player.position);
                if !valid_target {
                    events.push(DomainEvent::NothingToPickUp);
                } else if let Some(gold) = self.pick_up_gold_at_player(Some(&object_id)) {
                    changed.insert(self.player.position);
                    events.push(DomainEvent::GoldPickedUp {
                        amount: gold.gained,
                        balance: gold.balance,
                    });
                } else if self.mogaminator.auto_get_mode == AutoGetModeDto::Ammo {
                    let outcome = self.pick_up_item_at_player(Some(&object_id))?;
                    self.record_pick_up_outcome(outcome, &mut events, &mut changed);
                } else {
                    let resolutions = self.apply_mogaminator_to_items(vec![object_id], true)?;
                    self.record_mogaminator_resolutions(resolutions, &mut events, &mut changed);
                }
            }
            GameAction::PickUp => {
                let gold_pickup = self.pick_up_gold_at_player(None);
                if let Some(gold) = gold_pickup {
                    changed.insert(self.player.position);
                    events.push(DomainEvent::GoldPickedUp {
                        amount: gold.gained,
                        balance: gold.balance,
                    });
                }
                let mut picked_item = false;
                let resolutions = self.apply_mogaminator_at_player()?;
                picked_item |=
                    self.record_mogaminator_resolutions(resolutions, &mut events, &mut changed);
                let nothing_to_pick_up = if self.mogaminator.pending_query.is_none() {
                    let outcome = self.pick_up_at_player()?;
                    let nothing = matches!(&outcome, PickUpOutcome::Nothing);
                    picked_item |= self.record_pick_up_outcome(outcome, &mut events, &mut changed);
                    nothing
                } else {
                    false
                };
                if nothing_to_pick_up && gold_pickup.is_none() && !picked_item {
                    events.push(DomainEvent::NothingToPickUp);
                }
            }
            GameAction::Unequip { slot_id } => {
                if let Some((target_kind_id, severity)) = self.cursed_equipment_in_slot(&slot_id) {
                    events.push(DomainEvent::ItemUnequipCursed {
                        target_kind_id,
                        slot_id,
                        severity,
                    });
                } else if let Some(kind_id) = self.unequip_slot(&slot_id) {
                    self.refresh_player_resource_maxima();
                    events.push(DomainEvent::ItemUnequipped {
                        target_kind_id: kind_id,
                        slot_id,
                    });
                } else {
                    events.push(DomainEvent::ItemUnequipUnavailable { slot_id });
                }
            }
            GameAction::ParalyzedIdle => {
                events.push(DomainEvent::PlayerParalyzed {
                    status_kind_id: STATUS_PARALYSIS.to_owned(),
                });
            }
            GameAction::Move { direction } => {
                if self.map_scale == MapScaleDto::World {
                    if !self.move_on_world_map(direction, &mut changed) {
                        events.push(DomainEvent::MoveBlocked);
                    } else {
                        player_moved = true;
                        if !self.player_is_dead() && self.roll_wilderness_ambush() {
                            self.activate_wilderness_ambush()?;
                            action_cost = STANDARD_ACTION_COST;
                            events.push(DomainEvent::WildernessAmbushed);
                        }
                    }
                } else {
                    let direction = self.confused_direction(direction, &mut events);
                    let (dx, dy) = direction.delta();
                    let target = Position {
                        x: self.player.position.x + dx,
                        y: self.player.position.y + dy,
                    };
                    let movement_blocked = !self.player_can_enter_position(target);
                    if movement_blocked {
                        events.push(DomainEvent::MoveBlocked);
                    } else if let Some(index) = self
                        .entities
                        .iter()
                        .position(|entity| entity.position == target)
                    {
                        changed.insert(target);
                        if self.actor_is_player_side(&self.entities[index]) {
                            events.push(DomainEvent::MoveBlocked);
                        } else if self.player_fear_blocks_melee(index) {
                            events.push(DomainEvent::PlayerFearBlocked {
                                status_kind_id: STATUS_FEAR.to_owned(),
                            });
                        } else {
                            let melee = self.resolve_player_melee(
                                index,
                                true,
                                &mut events,
                                &mut changed,
                                &mut removed_entities,
                            )?;
                            if melee.killed && self.player_preserves_melee_energy_on_kill() {
                                action_cost = action_cost
                                    .saturating_mul(i32::from(melee.attacks_used))
                                    .saturating_div(i32::from(melee.attacks_available))
                                    .max(1);
                            }
                        }
                    } else if self.warn_player_of_hidden_trap(target, &mut events, &mut changed) {
                        // Warning spends the action revealing the danger; a repeated move is the
                        // player's explicit choice to step onto the now-visible trap.
                    } else {
                        match self
                            .scroll_wilderness_for_player_entry(target, &mut removed_entities)?
                        {
                            wilderness::WildernessPlayerEntry::Blocked => {
                                events.push(DomainEvent::MoveBlocked);
                            }
                            wilderness::WildernessPlayerEntry::Local {
                                target,
                                crossed_world_cell,
                                translation,
                            } => {
                                map_translation = translation;
                                self.destroy_wall_for_player_entry(
                                    target,
                                    &mut events,
                                    &mut changed,
                                );
                                if crossed_world_cell
                                    && self.wilderness_is_daytime()
                                    && self.wilderness_has_interesting_site()
                                {
                                    events.push(DomainEvent::WildernessInterestingDiscovery);
                                }
                                events.extend(self.relocate_player(target, &mut changed));
                                player_moved = true;
                                if let Some(translation) = translation {
                                    self.populate_scrolled_wilderness(translation);
                                }
                            }
                        }
                    }
                }
            }
            GameAction::Ride { direction } => {
                self.resolve_riding(direction, &mut events, &mut changed);
            }
            GameAction::OpenDoor { direction } => match self.open_door(direction) {
                Some(DoorOpenOutcome::Opened { position }) => {
                    changed.insert(position);
                    events.push(DomainEvent::DoorOpened { position });
                }
                Some(DoorOpenOutcome::Unlocked { position }) => {
                    changed.insert(position);
                    events.push(DomainEvent::DoorUnlocked { position });
                    events.push(DomainEvent::DoorOpened { position });
                }
                Some(DoorOpenOutcome::UnlockFailed { position }) => {
                    events.push(DomainEvent::DoorUnlockFailed { position });
                }
                None => events.push(DomainEvent::DoorOpenUnavailable),
            },
            GameAction::Search => {
                let discovered = self.search_hidden_terrain();
                if discovered.is_empty() {
                    events.push(DomainEvent::SearchFoundNothing);
                } else {
                    for position in discovered {
                        changed.insert(position);
                        events.push(DomainEvent::SecretTerrainDiscovered { position });
                    }
                }
            }
            GameAction::SellToShop {
                shop_id,
                item_id,
                quantity,
            } => match self.sell_to_shop(&shop_id, &item_id, quantity) {
                Ok(outcome) => events.push(DomainEvent::ShopSaleCompleted { outcome }),
                Err(reason) => events.push(DomainEvent::ShopTransactionUnavailable {
                    shop_id,
                    item_id,
                    reason: reason.to_owned(),
                }),
            },
            GameAction::WithdrawFromHome {
                facility_id,
                item_id,
                quantity,
            } => match self.withdraw_from_home(&facility_id, &item_id, quantity) {
                Ok(outcome) => events.push(DomainEvent::HomeItemWithdrawn { outcome }),
                Err(reason) => events.push(DomainEvent::HomeTransferUnavailable {
                    facility_id,
                    item_id,
                    reason: reason.to_owned(),
                }),
            },
            GameAction::SetSummonCommand { mode } => {
                self.summon_command = SummonCommandDto {
                    mode,
                    guard_position: (mode == SummonCommandModeDto::Guard)
                        .then_some(self.player.position),
                };
                let affected_summons = self
                    .entities
                    .iter()
                    .filter(|entity| entity.hp > 0 && self.actor_is_player_aligned(entity))
                    .count()
                    .try_into()
                    .unwrap_or(u16::MAX);
                events.push(DomainEvent::SummonCommandChanged {
                    resolution: SummonCommandResolutionDto {
                        command: self.summon_command.clone(),
                        affected_summons,
                    },
                });
            }
            GameAction::DismissPets => {
                let count = self.dismiss_controlled_pets(&mut changed, &mut removed_entities);
                events.push(DomainEvent::PetsDismissed {
                    count,
                    upkeep_percent: self.pet_upkeep().percent,
                });
            }
            GameAction::SetInterfaceLocale { locale } => {
                self.interface_locale = locale;
            }
            GameAction::DisarmTrap { direction } => match self.disarm_trap(direction) {
                Some(TrapDisarmOutcome::Succeeded { position }) => {
                    changed.insert(position);
                    events.push(DomainEvent::TrapDisarmed { position });
                }
                Some(TrapDisarmOutcome::Failed { position }) => {
                    events.push(DomainEvent::TrapDisarmFailed { position });
                }
                None => events.push(DomainEvent::TrapDisarmUnavailable),
            },
            GameAction::DigTerrain { direction } => {
                match self.dig_terrain(direction, &mut events, &mut changed) {
                    Some(TerrainDigOutcome::Succeeded {
                        position,
                        proficiency_improved,
                    }) => {
                        changed.insert(position);
                        events.push(DomainEvent::TerrainDug { position });
                        if proficiency_improved {
                            events.push(DomainEvent::MiningProficiencyImproved);
                        }
                    }
                    Some(TerrainDigOutcome::Failed {
                        position,
                        retryable,
                    }) => {
                        events.push(DomainEvent::TerrainDigFailed {
                            position,
                            retryable,
                        });
                    }
                    Some(TerrainDigOutcome::ActorBlocked { position, index }) => {
                        changed.insert(position);
                        if self.actor_is_player_side(&self.entities[index]) {
                            events.push(DomainEvent::MoveBlocked);
                        } else if self.player_fear_blocks_melee(index) {
                            events.push(DomainEvent::PlayerFearBlocked {
                                status_kind_id: STATUS_FEAR.to_owned(),
                            });
                        } else {
                            let melee = self.resolve_player_melee(
                                index,
                                true,
                                &mut events,
                                &mut changed,
                                &mut removed_entities,
                            )?;
                            if melee.killed && self.player_preserves_melee_energy_on_kill() {
                                action_cost = action_cost
                                    .saturating_mul(i32::from(melee.attacks_used))
                                    .saturating_div(i32::from(melee.attacks_available))
                                    .max(1);
                            }
                        }
                    }
                    None => events.push(DomainEvent::TerrainDigUnavailable),
                }
            }
        }

        if player_moved {
            action_cost = self.player_snow_movement_action_cost(action_cost);
            action_cost = self.player_wall_movement_action_cost(action_cost);
        }

        self.process_chaos_patron_level_rewards(
            &mut events,
            &mut chaos_patron_event_cursor,
            &mut changed,
            &mut removed_entities,
        )?;

        self.resolve_newly_visible_monster_auras(
            &visible_monster_auras_before_action,
            &mut events,
            &mut changed,
        );

        self.apply_player_floor_item_knowledge();
        if automatic_pickup_after_move
            && map_scale_before_command == MapScaleDto::Local
            && self.map_scale == MapScaleDto::Local
            && self.player.position != player_position_before_command
            && !self.player_is_dead()
        {
            let resolutions = self.apply_mogaminator_at_player()?;
            self.record_mogaminator_resolutions(resolutions, &mut events, &mut changed);
        }

        if self.player_has_status_kind(STATUS_UNDERSTANDING) || self.player_auto_identifies_items()
        {
            let count = self.identify_carried_items();
            if count > 0 {
                events.push(DomainEvent::ItemAutoIdentified { count });
            }
        }

        if self.mogaminator.enabled {
            let reevaluate_all = reevaluate_all_mogaminator_items
                && (!configuring_mogaminator || mogaminator_diagnostics.is_empty());
            let item_ids = self
                .items
                .iter()
                .filter(|item| item.location == ItemLocation::Inventory)
                .filter(|item| {
                    reevaluate_all
                        || item_property_knowledge_before
                            .as_ref()
                            .is_some_and(|before| {
                                before.get(&item.id) != self.item_property_knowledge.get(&item.id)
                            })
                        || item_knowledge_before.as_ref().is_some_and(|before| {
                            before.get(&item.kind_id) != self.item_knowledge.get(&item.kind_id)
                        })
                })
                .map(|item| item.id.clone())
                .collect::<Vec<_>>();
            let resolutions = self.apply_mogaminator_to_carried_items(item_ids)?;
            self.record_mogaminator_resolutions(resolutions, &mut events, &mut changed);
        }

        if advances_world && !self.player_is_dead() {
            events.extend(self.resolve_wilderness_terrain_hazard(self.player.position));
        }
        if advances_world {
            if astral_guide_blink.as_ref().is_some_and(|ability_id| {
                events.iter().any(|event| {
                    matches!(
                        event,
                        DomainEvent::AbilityCastSucceeded { resolution }
                            if resolution.ability_id == *ability_id
                    )
                })
            }) {
                action_cost /= 3;
            }
            if let Some(ability_id) = dimension_door {
                let failed = events.iter().find_map(|event| match event {
                    DomainEvent::AbilityEffectsResolved {
                        ability_id: resolved_id,
                        resolution,
                        ..
                    } if *resolved_id == ability_id => {
                        resolution.effects.iter().find_map(|effect| match effect {
                            AbilityEffectResolutionDto::DimensionDoor { failed, .. } => {
                                Some(*failed)
                            }
                            _ => None,
                        })
                    }
                    _ => None,
                });
                if let Some(failed) = failed {
                    let extra = STANDARD_ACTION_COST
                        .saturating_mul(i32::from(60_u16.saturating_sub(self.progress.level)))
                        / 100;
                    action_cost = action_cost.saturating_add(extra).saturating_add(if failed {
                        extra
                    } else {
                        0
                    });
                }
            }
            let vomit_extra_energy = events.iter().find_map(|event| match event {
                DomainEvent::AbilityEffectsResolved { resolution, .. } => {
                    resolution.effects.iter().find_map(|effect| match effect {
                        AbilityEffectResolutionDto::Vomit {
                            extra_energy_cost, ..
                        } => Some(*extra_energy_cost),
                        _ => None,
                    })
                }
                _ => None,
            });
            if let Some(extra_energy_cost) = vomit_extra_energy {
                action_cost = action_cost.saturating_add(i32::from(extra_energy_cost));
            }
            spend_energy(&mut self.player.energy_need, action_cost);
            self.advance_until_player_ready(
                false,
                self.map_scale != MapScaleDto::World,
                pet_neglect_allowed,
                &mut events,
                &mut changed,
                &mut removed_entities,
            )?;
            if recover_after_wait
                && !self.player_is_dead()
                && !self.wilderness_blocks_regeneration()
            {
                self.recover_player_resources(false, &mut events);
            } else if !self.player_is_dead() && !self.wilderness_blocks_regeneration() {
                self.apply_pet_upkeep_mana_loss(&mut events);
            }
        }
        self.refresh_daily_bounty_target();
        if let Some(actor_kind_id) = self.apply_bounty_deaths() {
            events.push(DomainEvent::BountyMissionCompleted { actor_kind_id });
        }
        self.apply_task_events(&mut events)?;
        self.apply_campaign_events(&mut events);
        self.process_chaos_patron_level_rewards(
            &mut events,
            &mut chaos_patron_event_cursor,
            &mut changed,
            &mut removed_entities,
        )?;
        self.clear_stale_mogaminator_query();

        let full_visibility_refresh = self.player.position != player_position_before_command
            || self.current_floor_id != floor_before_command
            || self.player_light_radius() != light_radius_before_command
            || self.player_see_invisible_sources() != see_invisible_sources_before_command
            || self.player_has_telepathy() != telepathy_before_command;
        let visible_monster_auras_before_invisible_refresh = self.visible_monster_aura_entity_ids();
        self.refresh_invisible_visibility(
            full_visibility_refresh,
            &entity_positions_before_command,
        );
        self.refresh_weird_mind_visibility(
            full_visibility_refresh,
            &entity_positions_before_command,
        );
        self.resolve_newly_visible_monster_auras(
            &visible_monster_auras_before_invisible_refresh,
            &mut events,
            &mut changed,
        );

        if self.world_tick != world_tick_before_command && self.map_scale == MapScaleDto::Local {
            // Clear only the grace windows that existed before this command.
            // Monsters generated while entering a floor keep their grace for
            // the player's first action on that floor.
            for entity in &mut self.entities {
                if nice_entities_at_command_start.contains(&entity.id) {
                    entity.nice = false;
                }
            }
        }

        self.last_command_seq = envelope.command_seq;
        self.turn = self.turn.saturating_add(turn_advance);
        self.revision = self.revision.saturating_add(1);
        let newly_discovered_gold_positions = self
            .gold_piles
            .iter()
            .filter(|pile| !pile.discovered && self.is_visible(pile.position))
            .map(|pile| pile.position)
            .collect::<Vec<_>>();
        self.reveal_current_visibility();
        changed.extend(newly_discovered_gold_positions);
        let current_dimensions = self.projected_dimensions();
        let current_visuals = self.visual_cells();
        let map_scale_changed = self.map_scale != map_scale_before_command;
        let wilderness_local_projection_changed = self.map_scale == MapScaleDto::Local
            && map_scale_before_command == MapScaleDto::Local
            && (self.is_wilderness_floor()
                || floor_before_command == wilderness::WILDERNESS_FLOOR_ID)
            && (self.current_floor_id != floor_before_command
                || self.wilderness_position != wilderness_position_before_command
                || self.wilderness_view_offset != wilderness_view_offset_before_command);
        let map_projection_changed = map_scale_changed
            || wilderness_local_projection_changed
            || current_dimensions != previous_dimensions;
        let changed_visual_cells = if !map_projection_changed {
            Self::changed_visual_cells(&current_visuals, &previous_visuals)
        } else {
            current_visuals.clone()
        };
        self.last_visual_cells = Some(current_visuals);
        let events = project_events(events);
        let changed_cells = if map_scale_changed || wilderness_local_projection_changed {
            self.projected_cells()
        } else {
            changed
                .into_iter()
                .map(|position| {
                    if self.map_scale == MapScaleDto::World {
                        self.wilderness_cell_dto(position)
                    } else {
                        self.cell_dto(position)
                    }
                })
                .collect()
        };
        let world_map = self.map_scale == MapScaleDto::World;

        Ok(GameUpdate {
            base_revision,
            revision: self.revision,
            turn: self.turn,
            world_tick: self.world_tick,
            command_seq: self.last_command_seq,
            map_scale: self.map_scale,
            map_translation,
            world_travel_destination: self.world_travel_destination,
            width: current_dimensions.0,
            height: current_dimensions.1,
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
            events,
            changed_cells,
            changed_visual_cells,
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
            mogaminator: self.mogaminator_dto(mogaminator_diagnostics),
            removed_entities,
            terrain_interactions: if world_map {
                Vec::new()
            } else {
                self.terrain_interactions()
            },
            tasks: self.task_statuses(),
            campaign: self.campaign_state_dto(),
            state_hash: self.state_hash(),
        })
    }

    #[must_use]
    pub const fn revision(&self) -> u32 {
        self.revision
    }

    #[must_use]
    pub const fn turn(&self) -> u32 {
        self.turn
    }

    #[must_use]
    pub const fn last_command_seq(&self) -> u32 {
        self.last_command_seq
    }

    #[must_use]
    pub const fn rng_draw_counter(&self) -> u64 {
        self.rng.draw_counter
    }

    #[must_use]
    pub const fn rng_algorithm(&self) -> &'static str {
        RNG_ALGORITHM
    }

    #[doc(hidden)]
    pub fn debug_set_ability_casts_succeed(&mut self, enabled: bool) {
        self.debug_ability_casts_succeed = enabled;
    }

    #[doc(hidden)]
    pub fn debug_set_recharge_attempts_succeed(&mut self, enabled: bool) {
        self.debug_recharge_attempts_succeed = enabled;
    }

    #[doc(hidden)]
    pub fn debug_set_recharge_attempts_fail(&mut self, enabled: bool) {
        self.debug_recharge_attempts_fail = enabled;
    }

    #[doc(hidden)]
    pub fn debug_set_recharge_sources_survive(&mut self, enabled: bool) {
        self.debug_recharge_sources_survive = enabled;
    }

    #[doc(hidden)]
    pub fn debug_set_recall_delay_turns(&mut self, turns: Option<u16>) {
        self.debug_recall_delay_turns = turns;
    }

    #[doc(hidden)]
    pub fn debug_set_item_curses_land(&mut self, enabled: bool) {
        self.debug_item_curses_land = enabled;
    }

    #[doc(hidden)]
    pub fn debug_set_item_curses_resisted(&mut self, enabled: bool) {
        self.debug_item_curses_resisted = enabled;
    }

    #[doc(hidden)]
    pub fn debug_add_generated_inventory_item(
        &mut self,
        id: &str,
        kind_id: &str,
        depth: u16,
    ) -> Result<(), CoreError> {
        if self.items.iter().any(|item| item.id == id) {
            return Err(CoreError::InvalidSave("duplicate item instance ID"));
        }
        if self.content.item(kind_id).is_none() {
            return Err(CoreError::UnknownItem(kind_id.to_owned()));
        }
        let (activation, charges) =
            initial_item_runtime_state(&self.content, &mut self.rng, kind_id, &[], depth);
        self.items.push(ItemInstance {
            previously_worn: false,
            artifact_name: None,
            intrinsic_melee_damage_dice: None,
            intrinsic_weight_tenths_pound: None,
            intrinsic_weapon_traits: Default::default(),
            intrinsic_curse_effects: Default::default(),
            id: id.to_owned(),
            kind_id: kind_id.to_owned(),
            quantity: 1,
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
            curse: initial_item_curse(&self.content, kind_id),
            permanent_destruction_immunities: Default::default(),
            activation,
            charges,
            fuel: initial_item_fuel(&self.content, kind_id),
            device_recovery_progress: 0,
            captured_actor: None,
            location: ItemLocation::Inventory,
        });
        Ok(())
    }

    #[doc(hidden)]
    pub fn debug_prepare_supply_e2e_gold(&mut self, amount: u32) -> Result<(), CoreError> {
        self.entities.clear();
        self.items
            .retain(|item| !matches!(item.location, ItemLocation::CarriedBy { .. }));
        self.gold_piles.clear();
        let id = self.allocate_gold_pile_id()?;
        self.gold_piles.push(GoldPile {
            id,
            position: self.player.position,
            amount,
            appearance: GoldAppearanceDto::Gold,
            discovered: true,
        });
        Ok(())
    }

    /// Desktop acceptance precondition: a living level-19 player at one life force,
    /// beside an awake original barrow-wight. The actual attack and conversion stay random.
    #[doc(hidden)]
    pub fn debug_prepare_life_force_e2e(&mut self, seed: u64) {
        self.entities.clear();
        self.items
            .retain(|item| !matches!(item.location, ItemLocation::CarriedBy { .. }));
        self.apply_player_experience(self.experience_required_for_level(19), &mut Vec::new());
        let east = Position {
            x: self.player.position.x + 1,
            y: self.player.position.y,
        };
        for position in [self.player.position, east] {
            let index = self
                .index(position)
                .expect("acceptance positions must be on the local map");
            self.terrain[index] = "demo.terrain.floor".to_owned();
            self.daylight_suppressed[index] = true;
        }
        let mut actor = self.generated_actor(
            "test.life-force".to_owned(),
            "demo.actor.barrow-wight",
            east,
        );
        actor.energy_need = 0;
        actor.nice = false;
        actor.statuses.clear();
        self.entities.push(actor);
        self.progress.life_force = 1;
        self.player.hp = self.effective_player_max_hp();
        self.rng = RfbRng::seeded(seed);
        self.reveal_current_visibility();
    }

    #[must_use]
    pub fn content_id(&self) -> &str {
        self.content.pack_id()
    }

    #[must_use]
    pub fn content_hash(&self) -> &str {
        self.content.content_hash()
    }

    #[must_use]
    pub fn world_id(&self) -> &str {
        &self.world_id
    }

    #[must_use]
    pub fn location_key(&self) -> &str {
        let world = self
            .content
            .world(&self.world_id)
            .expect("game world must remain in its content catalog");
        world
            .procedural_floors
            .iter()
            .find(|floor| floor.id == self.current_floor_id)
            .map_or(&world.name_key, |floor| &floor.name_key)
    }

    fn floor_depth(&self, floor_id: &str) -> u16 {
        let world = self
            .content
            .world(&self.world_id)
            .expect("game world must remain in its content catalog");
        world
            .procedural_floors
            .iter()
            .find(|floor| floor.id == floor_id)
            .map_or(0, |floor| floor.depth)
    }

    fn resolve_genocide_candidates(
        &mut self,
        candidate_ids: Vec<String>,
        scope: AbilityGenocideScopeDefinition,
        power: u16,
        applies_fatigue: bool,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> GenocideResolution {
        let mut removed_entity_ids = Vec::new();
        let mut resisted_entity_ids = Vec::new();
        let mut fatigue_damage = 0_i32;
        for entity_id in candidate_ids {
            let Some(entity) = self.entities.iter().find(|entity| entity.id == entity_id) else {
                continue;
            };
            let definition = self
                .content
                .actor(&entity.kind_id)
                .expect("genocide target definition must remain available");
            let target_level = definition.level;
            let protected = definition
                .tags
                .iter()
                .any(|tag| matches!(tag.as_str(), "unique" | "unique2" | "guardian"));
            let fatigue_sides = match scope {
                AbilityGenocideScopeDefinition::Single => target_level.div_ceil(2),
                AbilityGenocideScopeDefinition::Glyph => 4,
                AbilityGenocideScopeDefinition::Nearby => 3,
            }
            .max(1);
            if applies_fatigue {
                fatigue_damage = fatigue_damage.saturating_add(
                    i32::try_from(self.rng.bounded(u64::from(fatigue_sides)) + 1)
                        .expect("genocide fatigue roll must fit i32"),
                );
            }
            if protected {
                resisted_entity_ids.push(entity_id);
                continue;
            }
            let roll = u32::try_from(self.rng.bounded(u64::from(power)))
                .expect("validated genocide power roll must fit u32");
            if target_level > roll {
                resisted_entity_ids.push(entity_id);
            } else {
                removed_entity_ids.push(entity_id);
            }
        }
        for entity_id in &removed_entity_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| &entity.id == entity_id)
            else {
                continue;
            };
            let removed = self.entities.remove(index);
            if self.riding_actor_id.as_deref() == Some(removed.id.as_str()) {
                self.riding_actor_id = None;
            }
            self.clear_riding_bond_for(&removed.id);
            if let Some(pack_id) = removed
                .pack
                .as_ref()
                .and_then(|pack| (pack.role == MonsterPackRoleDto::Leader).then(|| pack.id.clone()))
            {
                for entity in &mut self.entities {
                    if entity.pack.as_ref().is_some_and(|pack| pack.id == pack_id) {
                        entity.pack = None;
                    }
                }
            }
            let carried_item_ids = self
                .items
                .iter()
                .filter_map(|item| match &item.location {
                    ItemLocation::CarriedBy { actor_id } if actor_id == entity_id => {
                        Some(item.id.clone())
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            self.items.retain(|item| {
                !matches!(&item.location, ItemLocation::CarriedBy { actor_id } if actor_id == entity_id)
            });
            for item_id in carried_item_ids {
                self.item_property_knowledge.remove(&item_id);
            }
            changed.insert(removed.position);
            removed_entities.push(removed.id);
        }
        fatigue_damage = i32::try_from(
            i64::from(fatigue_damage)
                .saturating_mul(i64::from(self.player_incoming_damage_percent()))
                .saturating_add(99)
                .saturating_div(100),
        )
        .unwrap_or(i32::MAX);
        let fatigue_damage = self
            .apply_final_player_damage(
                resolve_damage(
                    DamagePacket::new(fatigue_damage, DamageType::Physical),
                    ResistanceLevel::Normal,
                ),
                FatalityPolicy::BelowZero,
            )
            .damage
            .applied;
        GenocideResolution {
            removed_entity_ids,
            resisted_entity_ids,
            fatigue_damage,
        }
    }

    fn summon_category_candidate_kind_ids(
        &self,
        category: &str,
        excluded_category: Option<&str>,
        maximum_level: u16,
        allow_unique: bool,
    ) -> Vec<String> {
        let current_task_id = self.current_floor_task_id();
        self.content
            .actor_definitions()
            .filter(|definition| {
                let unique = definition
                    .tags
                    .iter()
                    .any(|tag| matches!(tag.as_str(), "unique" | "unique2"));
                definition.role == ActorRole::Monster
                    && definition.level <= u32::from(maximum_level)
                    && (category == "any-monster" || actor_matches_category(definition, category))
                    && excluded_category
                        .is_none_or(|category| !actor_matches_category(definition, category))
                    && !definition.tags.iter().any(|tag| tag == "guardian")
                    && actor_answers_summons(definition)
                    && self.dungeon_allows_monster(&self.current_floor_id, definition)
                    && definition.allocation.as_ref().is_none_or(|allocation| {
                        monster_ecology::actor_allocation_matches_task(allocation, current_task_id)
                    })
                    && (allow_unique || !unique)
                    && (!unique || self.unique_actor_kind_is_available(&definition.id))
            })
            .map(|definition| definition.id.clone())
            .collect()
    }

    fn resolve_category_summon(
        &mut self,
        spec: CategorySummonSpec<'_>,
        mut candidates: Vec<String>,
        positions: Vec<Position>,
        changed: &mut BTreeSet<Position>,
    ) -> AbilitySummonResolutionDto {
        if candidates.is_empty()
            || positions.is_empty()
            || (spec.is_spell && self.equipment_blocks_summoning())
        {
            return AbilitySummonResolutionDto {
                owner_id: spec.owner_id.to_owned(),
                actor_kind_id: spec.category.to_owned(),
                entity_ids: Vec::new(),
                positions: Vec::new(),
                duration_turns: spec.duration_turns,
                hostile: spec.hostile,
                group: false,
                summoned_kind_ids: Vec::new(),
            };
        }
        let group = match spec.group_chance_percent {
            0 => false,
            100 => true,
            chance => self.rng.bounded(100) < u64::from(chance),
        };
        let (dice, sides, bonus) = if group {
            (
                spec.group_count_dice,
                spec.group_count_sides,
                spec.group_count_bonus,
            )
        } else {
            (spec.count_dice, spec.count_sides, spec.count_bonus)
        };
        let rolled = self
            .roll_damage(u16::from(dice), u16::from(sides))
            .saturating_add(i32::from(bonus))
            .max(1);
        let count = usize::try_from(rolled)
            .unwrap_or(1)
            .min(spec.maximum_count.map_or(usize::MAX, usize::from))
            .min(positions.len());
        let mut entity_ids = Vec::with_capacity(count);
        let mut summoned_kind_ids = Vec::with_capacity(count);
        let mut used_positions = Vec::with_capacity(count);
        for position in positions {
            if entity_ids.len() >= count {
                break;
            }
            if candidates.is_empty() {
                break;
            }
            let eligible_choices = candidates
                .iter()
                .enumerate()
                .filter_map(|(index, kind_id)| {
                    (self.actor_kind_available_instance_count(kind_id) > 0
                        && self.actor_kind_can_enter_position(kind_id, position))
                    .then_some(index)
                })
                .collect::<Vec<_>>();
            if eligible_choices.is_empty() {
                continue;
            }
            let eligible_choice = usize::try_from(self.rng.bounded(
                u64::try_from(eligible_choices.len()).expect("eligible candidate count fits"),
            ))
            .expect("bounded summon choice must fit usize");
            let choice = eligible_choices[eligible_choice];
            let kind_id = candidates[choice].clone();
            let definition = self
                .content
                .actor(&kind_id)
                .expect("planned summon candidate must remain available")
                .clone();
            let id = self.summon_entity_id(spec.source_id, entity_ids.len());
            let mut entity = spawn_actor_from_definition(
                &mut self.rng,
                &definition,
                &id,
                position,
                INITIAL_MONSTER_ENERGY_NEED,
                true,
            );
            self.maybe_initialize_chameleon_form(&mut entity);
            if !spec.hostile {
                if spec.duration_turns == 0 {
                    entity.controller_id = Some(spec.owner_id.to_owned());
                } else {
                    entity.summon = Some(SummonIdentity {
                        owner_id: spec.owner_id.to_owned(),
                        source_ability_id: spec.source_id.to_owned(),
                        remaining_turns: spec.duration_turns,
                    });
                }
            }
            changed.insert(position);
            entity_ids.push(id);
            summoned_kind_ids.push(kind_id.clone());
            used_positions.push(position);
            self.entities.push(entity);
            if self.actor_kind_available_instance_count(&kind_id) == 0 {
                candidates.remove(choice);
            }
        }
        AbilitySummonResolutionDto {
            owner_id: spec.owner_id.to_owned(),
            actor_kind_id: spec.category.to_owned(),
            entity_ids,
            positions: used_positions,
            duration_turns: spec.duration_turns,
            hostile: spec.hostile,
            group,
            summoned_kind_ids,
        }
    }

    fn teleport_destination(
        &self,
        ability: &AbilityDefinition,
        destination: Position,
    ) -> Option<Position> {
        let origin = self.player.position;
        if !ability
            .target
            .modes
            .contains(&AbilityTargetModeDefinition::Position)
            || destination == origin
            || self.index(destination).is_none()
            || origin
                .x
                .abs_diff(destination.x)
                .max(origin.y.abs_diff(destination.y))
                > u32::from(ability.target.range)
            || !self.is_visible(destination)
            || (ability.target.requires_line_of_effect
                && !has_line_of_effect(self, origin, destination))
            || !self.is_walkable(destination)
            || self
                .entities
                .iter()
                .any(|entity| entity.hp > 0 && entity.position == destination)
        {
            return None;
        }
        Some(destination)
    }

    fn summon_positions_around(
        &self,
        origin: Position,
        count: u8,
        radius: u8,
        actor_kind_id: &str,
    ) -> Option<Vec<Position>> {
        let candidates = self.open_positions_around_for_actor_kind(origin, radius, actor_kind_id);
        let count = usize::from(count);
        (candidates.len() >= count).then(|| candidates.into_iter().take(count).collect())
    }

    fn open_positions_around_for_actor_kind(
        &self,
        origin: Position,
        radius: u8,
        actor_kind_id: &str,
    ) -> Vec<Position> {
        self.open_positions_around_matching(origin, radius, |position| {
            self.actor_kind_can_enter_position(actor_kind_id, position)
        })
    }

    fn open_positions_around(&self, origin: Position, radius: u8) -> Vec<Position> {
        self.open_positions_around_matching(origin, radius, |position| self.is_walkable(position))
    }

    fn open_positions_around_for_actor_kinds(
        &self,
        origin: Position,
        radius: u8,
        actor_kind_ids: &[String],
    ) -> Vec<Position> {
        self.open_positions_around_matching(origin, radius, |position| {
            actor_kind_ids
                .iter()
                .any(|kind_id| self.actor_kind_can_enter_position(kind_id, position))
        })
    }

    fn open_positions_around_matching(
        &self,
        origin: Position,
        radius: u8,
        accepts: impl Fn(Position) -> bool,
    ) -> Vec<Position> {
        let occupied = self
            .entities
            .iter()
            .filter(|entity| entity.hp > 0)
            .map(|entity| entity.position)
            .chain(std::iter::once(origin))
            .chain(self.items.iter().filter_map(|item| match item.location {
                ItemLocation::Ground(position) => Some(position),
                ItemLocation::Inventory
                | ItemLocation::Equipped { .. }
                | ItemLocation::CarriedBy { .. }
                | ItemLocation::Shop { .. }
                | ItemLocation::Home { .. } => None,
            }))
            .collect::<BTreeSet<_>>();
        let mut candidates = Vec::new();
        for y in
            origin.y.saturating_sub(i32::from(radius))..=origin.y.saturating_add(i32::from(radius))
        {
            for x in origin.x.saturating_sub(i32::from(radius))
                ..=origin.x.saturating_add(i32::from(radius))
            {
                let position = Position { x, y };
                let distance = origin.x.abs_diff(x).max(origin.y.abs_diff(y));
                if distance == 0
                    || distance > u32::from(radius)
                    || self.index(position).is_none()
                    || !accepts(position)
                    || occupied.contains(&position)
                {
                    continue;
                }
                candidates.push((distance, position.y, position.x, position));
            }
        }
        candidates.sort_unstable_by_key(|(distance, y, x, _)| (*distance, *y, *x));
        candidates.into_iter().map(|entry| entry.3).collect()
    }

    fn detect_terrain_positions(
        &mut self,
        category: &str,
        radius: u8,
        persistent: bool,
        through_walls: bool,
    ) -> Vec<Position> {
        let origin = self.player.position;
        let radius_distance = u32::from(radius);
        let radius_offset = i32::from(radius);
        let mut candidates = Vec::new();
        for y in origin.y.saturating_sub(radius_offset)..=origin.y.saturating_add(radius_offset) {
            for x in origin.x.saturating_sub(radius_offset)..=origin.x.saturating_add(radius_offset)
            {
                let position = Position { x, y };
                let distance = origin.x.abs_diff(x).max(origin.y.abs_diff(y));
                if distance > radius_distance || (!through_walls && !self.is_visible(position)) {
                    continue;
                }
                let Some(index) = self.index(position) else {
                    continue;
                };
                if category == "map" {
                    if !self.explored[index] {
                        candidates.push((distance, position.y, position.x, position));
                    }
                    continue;
                }
                let Some(terrain) = self.content.terrain(&self.terrain[index]) else {
                    continue;
                };
                if !terrain.tags.iter().any(|tag| tag == category)
                    || (persistent
                        && terrain.concealed_as_terrain_id.is_some()
                        && self.revealed_terrain.contains(&position))
                {
                    continue;
                }
                candidates.push((distance, position.y, position.x, position));
            }
        }
        candidates.sort_unstable_by_key(|(distance, y, x, _)| (*distance, *y, *x));
        let positions = candidates
            .into_iter()
            .map(|(_, _, _, position)| position)
            .collect::<Vec<_>>();
        if persistent {
            for position in &positions {
                let index = self
                    .index(*position)
                    .expect("detected position must remain valid");
                self.explored[index] = true;
            }
            if category != "map" {
                let concealed_positions = positions
                    .iter()
                    .copied()
                    .filter(|position| {
                        self.index(*position)
                            .and_then(|index| self.content.terrain(&self.terrain[index]))
                            .is_some_and(|terrain| terrain.concealed_as_terrain_id.is_some())
                    })
                    .collect::<Vec<_>>();
                self.revealed_terrain.extend(concealed_positions);
            }
        }
        positions
    }

    fn detect_actor_positions(&self, category: &str, radius: u8) -> (Vec<Position>, Vec<String>) {
        let origin = self.player.position;
        let mut candidates = self
            .entities
            .iter()
            .filter(|entity| entity.hp > 0)
            .filter(|entity| {
                chebyshev_distance(origin, entity.position) <= u32::from(radius)
                    && self
                        .content
                        .actor(&entity.kind_id)
                        .is_some_and(|definition| actor_matches_category(definition, category))
            })
            .map(|entity| {
                (
                    chebyshev_distance(origin, entity.position),
                    entity.position.y,
                    entity.position.x,
                    entity.id.clone(),
                    entity.position,
                )
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            (left.0, left.1, left.2, left.3.as_str()).cmp(&(
                right.0,
                right.1,
                right.2,
                right.3.as_str(),
            ))
        });
        let positions = candidates.iter().map(|candidate| candidate.4).collect();
        let entity_ids = candidates
            .into_iter()
            .map(|candidate| candidate.3)
            .collect();
        (positions, entity_ids)
    }

    fn detect_item_positions(
        &self,
        category: &str,
        radius: u8,
        through_walls: bool,
    ) -> (Vec<Position>, Vec<String>) {
        let origin = self.player.position;
        let mut candidates = self
            .items
            .iter()
            .filter_map(|item| {
                let ItemLocation::Ground(position) = &item.location else {
                    return None;
                };
                let distance = chebyshev_distance(origin, *position);
                if distance > u32::from(radius)
                    || (!through_walls && !self.is_visible(*position))
                    || !self.content.item(&item.kind_id).is_some_and(|definition| {
                        category == "item"
                            || (category == "artifact" && item.is_artifact(&self.content))
                            || definition.tags.iter().any(|tag| tag == category)
                            || (category == "magic-item"
                                && (item.is_artifact(&self.content)
                                    || !item.affix_ids.is_empty()
                                    || !item.rolled_affixes.is_empty()
                                    || definition.device_generation.is_some()
                                    || definition.tags.iter().any(|tag| {
                                        matches!(
                                            tag.as_str(),
                                            "jewelry"
                                                | "device"
                                                | "scroll"
                                                | "potion"
                                                | "book"
                                                | "spellbook"
                                        )
                                    })
                                    || (definition.equipment_slot.is_some()
                                        && (item.enchantments.to_armor > 0
                                            || item.enchantments.to_hit
                                                + item.enchantments.to_damage
                                                > 0))))
                    })
                {
                    return None;
                }
                Some((distance, position.y, position.x, item.id.clone(), *position))
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            (left.0, left.1, left.2, left.3.as_str()).cmp(&(
                right.0,
                right.1,
                right.2,
                right.3.as_str(),
            ))
        });
        let positions = candidates.iter().map(|candidate| candidate.4).collect();
        let item_ids = candidates
            .into_iter()
            .map(|candidate| candidate.3)
            .collect();
        (positions, item_ids)
    }

    fn detect_gold_positions(
        &self,
        radius: u8,
        through_walls: bool,
    ) -> (Vec<Position>, Vec<String>) {
        let origin = self.player.position;
        let mut candidates = self
            .gold_piles
            .iter()
            .filter_map(|pile| {
                let distance = chebyshev_distance(origin, pile.position);
                (distance <= u32::from(radius) && (through_walls || self.is_visible(pile.position)))
                    .then(|| {
                        (
                            distance,
                            pile.position.y,
                            pile.position.x,
                            pile.id.clone(),
                            pile.position,
                        )
                    })
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            (left.0, left.1, left.2, left.3.as_str()).cmp(&(
                right.0,
                right.1,
                right.2,
                right.3.as_str(),
            ))
        });
        let positions = candidates.iter().map(|candidate| candidate.4).collect();
        let pile_ids = candidates
            .into_iter()
            .map(|candidate| candidate.3)
            .collect();
        (positions, pile_ids)
    }

    fn terrain_transform_positions(
        &self,
        ability: &AbilityDefinition,
        center: Position,
        source_terrain_ids: &[String],
        target_terrain_id: &str,
        radius: u8,
    ) -> Option<Vec<Position>> {
        self.terrain_transform_positions_from(
            ability,
            None,
            center,
            source_terrain_ids,
            target_terrain_id,
            radius,
        )
    }

    fn terrain_transform_positions_from(
        &self,
        ability: &AbilityDefinition,
        monster_origin: Option<Position>,
        center: Position,
        source_terrain_ids: &[String],
        target_terrain_id: &str,
        radius: u8,
    ) -> Option<Vec<Position>> {
        let (origin, require_visible) = monster_origin
            .map(|origin| (origin, false))
            .unwrap_or((self.player.position, true));
        if !ability
            .target
            .modes
            .contains(&AbilityTargetModeDefinition::Position)
            || self.index(center).is_none()
            || origin.x.abs_diff(center.x).max(origin.y.abs_diff(center.y))
                > u32::from(ability.target.range)
            || (require_visible && !self.is_visible(center))
            || (ability.target.requires_line_of_effect && !has_line_of_effect(self, origin, center))
        {
            return None;
        }
        debug_assert!(self.content.terrain(target_terrain_id).is_some());

        let occupied = self
            .entities
            .iter()
            .filter(|entity| entity.hp > 0)
            .map(|entity| entity.position)
            .chain(std::iter::once(self.player.position))
            .chain(self.items.iter().filter_map(|item| match item.location {
                ItemLocation::Ground(position) => Some(position),
                ItemLocation::Inventory
                | ItemLocation::Equipped { .. }
                | ItemLocation::CarriedBy { .. }
                | ItemLocation::Shop { .. }
                | ItemLocation::Home { .. } => None,
            }))
            .collect::<BTreeSet<_>>();
        let connections = self
            .floor_connections
            .iter()
            .map(|connection| connection.position)
            .collect::<BTreeSet<_>>();
        let radius_limit = u32::from(radius);
        let radius_offset = i32::from(radius);
        let max_x = i32::from(self.width).saturating_sub(1);
        let max_y = i32::from(self.height).saturating_sub(1);
        let mut candidates = Vec::new();
        for y in center.y.saturating_sub(radius_offset)..=center.y.saturating_add(radius_offset) {
            for x in center.x.saturating_sub(radius_offset)..=center.x.saturating_add(radius_offset)
            {
                let position = Position { x, y };
                let distance = rfb_distance(center, position);
                if distance > radius_limit
                    || position.x <= 0
                    || position.y <= 0
                    || position.x >= max_x
                    || position.y >= max_y
                    || occupied.contains(&position)
                    || connections.contains(&position)
                    || (require_visible && !self.is_visible(position))
                    || !has_line_of_effect(self, center, position)
                {
                    continue;
                }
                let Some(index) = self.index(position) else {
                    continue;
                };
                let terrain_id = &self.terrain[index];
                let Some(terrain) = self.content.terrain(terrain_id) else {
                    continue;
                };
                if terrain.tags.iter().any(|tag| {
                    matches!(
                        tag.as_str(),
                        "stairs-down" | "stairs-up" | "shaft" | "dungeon-entry" | "task-entry"
                    )
                }) {
                    continue;
                }
                if source_terrain_ids.binary_search(terrain_id).is_err() {
                    continue;
                }
                candidates.push((distance, position.y, position.x, position));
            }
        }
        candidates.sort_unstable_by_key(|(distance, y, x, _)| (*distance, *y, *x));
        Some(
            candidates
                .into_iter()
                .map(|(_, _, _, position)| position)
                .collect(),
        )
    }

    fn summon_entity_id(&self, ability_id: &str, ordinal: usize) -> String {
        let command_seq = self.last_command_seq.saturating_add(1);
        let base = format!("summon.{ability_id}.{command_seq}.{ordinal}");
        if self.entities.iter().all(|entity| entity.id != base) {
            return base;
        }
        let mut suffix = 1_u32;
        loop {
            let candidate = format!("{base}.{suffix}");
            if self.entities.iter().all(|entity| entity.id != candidate) {
                return candidate;
            }
            suffix = suffix.saturating_add(1);
        }
    }

    fn settle_projectile_ammunition(
        &mut self,
        mut ammunition: ItemInstance,
        landing: Position,
        hit_body: bool,
        break_chance_percent: u8,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let drop_position =
            self.ground_drop_position(landing, ammunition.is_artifact(&self.content));
        let broken = (hit_body && self.rng.bounded(100) < u64::from(break_chance_percent))
            || drop_position.is_none();
        if broken {
            self.item_property_knowledge.remove(&ammunition.id);
            events.push(DomainEvent::ProjectileAmmoBroken {
                ammo_kind_id: ammunition.kind_id,
            });
            return;
        }
        let landing = drop_position.expect("unbroken ammunition has a valid drop position");
        ammunition.location = ItemLocation::Ground(landing);
        let ammo_kind_id = ammunition.kind_id.clone();
        self.items.push(ammunition);
        changed.insert(landing);
        events.push(DomainEvent::ProjectileAmmoRecovered { ammo_kind_id });
    }

    fn take_inventory_item(&mut self, item_id: &str) -> Result<Option<ItemInstance>, CoreError> {
        let Some(index) = self.items.iter().position(|item| {
            item.id == item_id && item.location == ItemLocation::Inventory && item.quantity > 0
        }) else {
            return Ok(None);
        };
        if self.items[index].quantity == 1 {
            Ok(Some(self.items.remove(index)))
        } else {
            let id = self.allocate_item_instance_id()?;
            let mut split = self.items[index].clone();
            let knowledge = self.item_property_knowledge.get(&split.id).cloned();
            self.items[index].quantity -= 1;
            split.id = id.clone();
            split.quantity = 1;
            split.location = ItemLocation::Inventory;
            if let Some(knowledge) = knowledge {
                self.item_property_knowledge.insert(id, knowledge);
            }
            Ok(Some(split))
        }
    }

    fn item_charge_is_insufficient(&self, item_id: &str) -> bool {
        let Some(item) = self.items.iter().find(|item| {
            item.id == item_id && item.location == ItemLocation::Inventory && item.quantity > 0
        }) else {
            return false;
        };
        let cost = item
            .activation
            .as_ref()
            .map(|activation| activation.cost)
            .or_else(|| {
                self.content
                    .item(&item.kind_id)
                    .and_then(|definition| definition.use_action.as_ref())
                    .and_then(|action| action.charges)
                    .map(|charges| charges.cost)
            });
        let Some(cost) = cost else {
            return false;
        };
        item.charges.is_none_or(|state| state.current < cost)
    }

    fn inventory_item_use_context(
        &self,
        item_id: &str,
    ) -> Result<Option<(usize, rfb_content::ItemDefinition)>, CoreError> {
        let Some(index) = self
            .items
            .iter()
            .position(|item| item.id == item_id && item.quantity > 0)
        else {
            return Ok(None);
        };
        let item = &self.items[index];
        let definition = self.content.item(&item.kind_id).cloned().ok_or_else(|| {
            CoreError::Invariant(format!(
                "inventory item {} references missing kind {}",
                item.id, item.kind_id
            ))
        })?;
        if item.location != ItemLocation::Inventory
            && !(matches!(item.location, ItemLocation::Equipped { .. })
                && (definition.capture_ball || item.activation.is_some()))
        {
            return Ok(None);
        }
        if let Some(activation) = &item.activation
            && item_device_generation(
                &self.content,
                &item.kind_id,
                &item.affix_ids,
                item.activation
                    .as_ref()
                    .map(|activation| activation.profile_id.as_str()),
                item.artifact_name.is_some(),
            )
            .and_then(|generation| {
                generation
                    .activations
                    .iter()
                    .find(|profile| profile.id == activation.profile_id)
            })
            .is_none()
        {
            return Err(CoreError::Invariant(format!(
                "dynamic item {} references missing activation profile {}",
                item.id, activation.profile_id
            )));
        }
        Ok(Some((index, definition)))
    }

    fn inventory_item_use_effect(
        &self,
        source_item_id: &str,
    ) -> Option<(&ItemUseEffectDefinition, Option<&AbilityTargetDefinition>)> {
        let item = self.items.iter().find(|item| {
            item.id == source_item_id
                && (item.location == ItemLocation::Inventory
                    || (matches!(item.location, ItemLocation::Equipped { .. })
                        && item.activation.is_some()))
                && item.quantity > 0
        })?;
        let definition = self.content.item(&item.kind_id)?;
        if let Some(activation) = &item.activation {
            let profile = item_device_generation(
                &self.content,
                &item.kind_id,
                &item.affix_ids,
                item.activation
                    .as_ref()
                    .map(|activation| activation.profile_id.as_str()),
                item.artifact_name.is_some(),
            )?
            .activations
            .iter()
            .find(|candidate| candidate.id == activation.profile_id)?;
            Some((&profile.effect, Some(&profile.target)))
        } else {
            definition
                .use_action
                .as_ref()
                .map(|action| (&action.effect, None))
        }
    }

    fn item_use_is_zero_time_unavailable(
        &self,
        source_item_id: &str,
        target: Option<&TargetSelection>,
        target_glyph: Option<&str>,
    ) -> bool {
        if let Some(valid) = self.mount_item_target_is_valid(source_item_id, target) {
            return !valid;
        }
        let Some((effect, target_definition)) = self.inventory_item_use_effect(source_item_id)
        else {
            return false;
        };
        (target_glyph.is_some()
            || matches!(effect, ItemUseEffectDefinition::Genocide { .. })
            || matches!(
                effect,
                ItemUseEffectDefinition::AbilityEffect { .. }
                    | ItemUseEffectDefinition::IdentifyItem { .. }
                    | ItemUseEffectDefinition::EnchantItem { .. }
                    | ItemUseEffectDefinition::CraftItem { .. }
                    | ItemUseEffectDefinition::RechargeFromDevice { .. }
                    | ItemUseEffectDefinition::RandomTeleport { .. }
                    | ItemUseEffectDefinition::TeleportLevel
                    | ItemUseEffectDefinition::Recall { .. }
                    | ItemUseEffectDefinition::ResetRecall
            ))
            && self
                .item_use_plan(
                    source_item_id,
                    effect,
                    target_definition,
                    target,
                    target_glyph,
                )
                .is_none()
    }

    fn item_effect_path(
        &self,
        target_definition: &AbilityTargetDefinition,
        target: &TargetSelection,
    ) -> Option<Vec<Position>> {
        let mode = match target {
            TargetSelection::Direction { .. } => AbilityTargetModeDefinition::Direction,
            TargetSelection::Position { .. } => AbilityTargetModeDefinition::Position,
            TargetSelection::Entity { .. } => AbilityTargetModeDefinition::Entity,
            TargetSelection::Item { .. } => AbilityTargetModeDefinition::Item,
            TargetSelection::Town { .. } => AbilityTargetModeDefinition::Town,
            TargetSelection::CraftingItem { .. } => return None,
            TargetSelection::SelfTarget => AbilityTargetModeDefinition::SelfTarget,
        };
        target_definition
            .modes
            .contains(&mode)
            .then(|| self.projectile_path(target, target_definition.range))
            .flatten()
    }

    fn player_is_dead(&self) -> bool {
        self.player.hp < 0
    }

    fn player_has_status_kind(&self, kind_id: &str) -> bool {
        self.player
            .statuses
            .iter()
            .any(|status| status.kind_id == kind_id)
    }

    fn player_fear_blocks_melee(&mut self, target_index: usize) -> bool {
        let Some(fear) = self
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_FEAR)
            .cloned()
        else {
            return false;
        };
        let mut ability = self.player_derived_stats().melee_skill;
        if self.player_has_mutation(HUMAN_INT_MUTATION_ID) {
            ability = ability.with_modifier(
                StatLayer::Status,
                HUMAN_INT_MUTATION_ID,
                -10,
                StatBounds::NON_NEGATIVE,
            );
        }
        let mut difficulty = DerivedStatsPipeline::new();
        difficulty.add_with_origin(
            StatKind::ActionDifficulty,
            StatLayer::Status,
            &fear.kind_id,
            fear.source_id,
            i32::from(fear.intensity).saturating_mul(40),
        );
        !resolve_check(
            &mut self.rng,
            CheckContext {
                kind: CheckKind::FearAction,
                actor_id: self.player.id.clone(),
                target_id: Some(self.entities[target_index].id.clone()),
                ability,
                difficulty: difficulty
                    .resolve(StatKind::ActionDifficulty, StatBounds::NON_NEGATIVE),
            },
        )
        .succeeded()
    }

    fn resolve_monster_action(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
        surround_reservations: &mut BTreeSet<Position>,
    ) -> Result<(), CoreError> {
        self.resolve_monster_action_during_world(
            index,
            events,
            changed,
            removed_entities,
            surround_reservations,
            false,
        )
    }

    fn resolve_monster_action_during_world(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
        surround_reservations: &mut BTreeSet<Position>,
        world_stopped: bool,
    ) -> Result<(), CoreError> {
        self.maybe_change_chameleon_form(index, events, changed);
        self.reroll_shapechanger_appearance(index);
        let never_moves = self
            .actor_runtime_definition(&self.entities[index])
            .is_some_and(|definition| definition.movement.never_moves);
        if self.entity_is_player_aligned(index) {
            if !never_moves
                && self.resolve_original_random_movement(
                    index,
                    events,
                    changed,
                    removed_entities,
                )?
            {
                return Ok(());
            }
            self.resolve_player_summon_action(index, events, changed, removed_entities)?;
            return Ok(());
        }
        if !self.entities[index].alerted && !self.resolve_monster_detection(index, events) {
            return Ok(());
        }
        if self.resolve_monster_ability_with_changes(
            index,
            events,
            changed,
            removed_entities,
            world_stopped,
        )? {
            return Ok(());
        }
        if !never_moves
            && self.resolve_original_random_movement(index, events, changed, removed_entities)?
        {
            return Ok(());
        }
        let Some(primary_target) = self.monster_hostile_targets(index).into_iter().next() else {
            return Ok(());
        };
        if self.monster_can_use_ranged_melee(index, &primary_target) {
            self.resolve_monster_melee_target(
                index,
                &primary_target,
                events,
                changed,
                removed_entities,
            )?;
            return Ok(());
        }
        if never_moves {
            if adjacent(self.entities[index].position, primary_target.position()) {
                self.resolve_monster_melee_target(
                    index,
                    &primary_target,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            return Ok(());
        }
        let casting = self
            .actor_runtime_definition(&self.entities[index])
            .and_then(|definition| definition.monster_casting.clone());
        let current_distance = self.entities[index]
            .position
            .x
            .abs_diff(primary_target.position().x)
            .max(
                self.entities[index]
                    .position
                    .y
                    .abs_diff(primary_target.position().y),
            );
        let hp_percent = i64::from(self.entities[index].hp.max(0))
            .saturating_mul(100)
            .saturating_div(i64::from(self.entities[index].max_hp.max(1)));
        let tactical_reason = casting.as_ref().and_then(|casting| {
            if casting.flee_hp_percent > 0 && hp_percent <= i64::from(casting.flee_hp_percent) {
                Some(MonsterTacticalReason::Wounded)
            } else if casting
                .preferred_distance
                .is_some_and(|distance| current_distance < u32::from(distance))
            {
                Some(MonsterTacticalReason::KeepDistance)
            } else {
                None
            }
        });
        if let Some(reason) = tactical_reason
            && let Some(next_position) = self.next_monster_step_away(index)
        {
            let source_kind_id = self.entities[index].kind_id.clone();
            let target_kind_id = primary_target.kind_id().to_owned();
            if self.move_entity(index, next_position, events, changed, removed_entities)?
                != ActorStepOutcome::Moved
            {
                return Ok(());
            }
            events.push(match reason {
                MonsterTacticalReason::Wounded => DomainEvent::MonsterFled {
                    source_kind_id,
                    target_kind_id,
                },
                MonsterTacticalReason::KeepDistance => DomainEvent::MonsterKeptDistance {
                    source_kind_id,
                    target_kind_id,
                },
            });
            return Ok(());
        }
        let behavior = self.entities[index]
            .pack
            .as_ref()
            .map_or(MonsterPackBehaviorDto::Seek, |pack| pack.behavior);
        if adjacent(self.entities[index].position, primary_target.position()) {
            if behavior == MonsterPackBehaviorDto::Surround {
                surround_reservations.insert(self.entities[index].position);
            }
            self.resolve_monster_melee_target(
                index,
                &primary_target,
                events,
                changed,
                removed_entities,
            )?;
            return Ok(());
        }
        let next_position = match behavior {
            MonsterPackBehaviorDto::Seek => {
                self.next_monster_step_toward(index, primary_target.position(), true)
            }
            MonsterPackBehaviorDto::Surround => self
                .next_surround_step(index, surround_reservations)
                .or_else(|| self.next_monster_step(index)),
            MonsterPackBehaviorDto::GuardLeader => {
                let pack = self.entities[index].pack.as_ref();
                let is_leader = pack.is_some_and(|pack| pack.leader_id == self.entities[index].id);
                let leader_position = pack.and_then(|pack| {
                    self.entities
                        .iter()
                        .find(|entity| entity.id == pack.leader_id)
                        .map(|leader| leader.position)
                });
                match leader_position {
                    Some(_) if is_leader => self
                        .next_surround_step(index, surround_reservations)
                        .or_else(|| self.next_monster_step(index)),
                    Some(position) if current_distance > 3 => {
                        self.next_monster_step_toward(index, position, true)
                    }
                    Some(_) => self
                        .next_surround_step(index, surround_reservations)
                        .or_else(|| self.next_monster_step(index)),
                    None => self.next_monster_step(index),
                }
            }
            // Entrance guardians use the established fixed-post contract:
            // they may attack an adjacent target above, but never leave the
            // declared entrance position to pursue one.
            MonsterPackBehaviorDto::GuardPosition => None,
            MonsterPackBehaviorDto::Lure => self
                .next_monster_hiding_step(index)
                .or_else(|| self.next_monster_step(index)),
            MonsterPackBehaviorDto::Shoot => None,
            MonsterPackBehaviorDto::MaintainDistance => {
                if current_distance <= 5
                    && self.player.hp.saturating_mul(5)
                        >= self.effective_player_max_hp().saturating_mul(4)
                {
                    self.next_monster_step_away(index)
                } else {
                    self.next_monster_step(index)
                }
            }
        };
        let Some(next_position) = next_position else {
            return Ok(());
        };
        self.move_entity(index, next_position, events, changed, removed_entities)?;
        Ok(())
    }

    fn wake_entity_after_damage(
        &mut self,
        index: usize,
        applied_damage: i32,
        events: &mut Vec<DomainEvent>,
    ) {
        if applied_damage <= 0 || self.entities[index].hp <= 0 {
            return;
        }
        self.wake_entity(index, events);
    }

    fn wake_entity(&mut self, index: usize, events: &mut Vec<DomainEvent>) {
        let before = self.entities[index].statuses.len();
        self.entities[index]
            .statuses
            .retain(|status| status.kind_id != STATUS_SLEEP);
        if self.entities[index].statuses.len() != before {
            events.push(DomainEvent::EntityAwakened {
                target_kind_id: self.entities[index].kind_id.clone(),
            });
        }
    }

    fn resolve_player_summon_action(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let never_moves = self
            .actor_runtime_definition(&self.entities[index])
            .is_some_and(|definition| definition.movement.never_moves);
        let targets = self.player_summon_hostile_targets(index);
        let adjacent_target = targets.iter().find(|entity_id| {
            self.entities
                .iter()
                .find(|entity| entity.id == **entity_id)
                .is_some_and(|target| adjacent(self.entities[index].position, target.position))
        });
        if never_moves {
            if let Some(target_id) = adjacent_target {
                self.resolve_player_summon_melee(
                    index,
                    target_id,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            return Ok(());
        }
        let owner_position = self.player.position;
        let next_position = match self.summon_command.mode {
            SummonCommandModeDto::Follow => {
                if let Some(target_id) = adjacent_target {
                    self.resolve_player_summon_melee(
                        index,
                        target_id,
                        events,
                        changed,
                        removed_entities,
                    )?;
                    return Ok(());
                }
                if adjacent(self.entities[index].position, owner_position) {
                    None
                } else {
                    self.next_monster_step_toward(index, owner_position, true)
                }
            }
            SummonCommandModeDto::Attack => {
                let Some(target_id) = targets.first() else {
                    if adjacent(self.entities[index].position, owner_position) {
                        return Ok(());
                    }
                    if let Some(next_position) =
                        self.next_monster_step_toward(index, owner_position, true)
                    {
                        self.move_entity(index, next_position, events, changed, removed_entities)?;
                    }
                    return Ok(());
                };
                let target_position = self
                    .entities
                    .iter()
                    .find(|entity| entity.id == *target_id)
                    .expect("collected summon target must remain available")
                    .position;
                if adjacent(self.entities[index].position, target_position) {
                    self.resolve_player_summon_melee(
                        index,
                        target_id,
                        events,
                        changed,
                        removed_entities,
                    )?;
                    return Ok(());
                }
                self.next_monster_step_toward(index, target_position, true)
            }
            SummonCommandModeDto::KeepDistance => {
                let distance = chebyshev_distance(self.entities[index].position, owner_position);
                if distance < 3 {
                    self.next_player_summon_step_away_from_owner(index)
                } else if distance > 3 {
                    self.next_monster_step_toward(index, owner_position, true)
                } else if let Some(target_id) = adjacent_target {
                    self.resolve_player_summon_melee(
                        index,
                        target_id,
                        events,
                        changed,
                        removed_entities,
                    )?;
                    return Ok(());
                } else {
                    None
                }
            }
            SummonCommandModeDto::Guard => {
                if let Some(target_id) = adjacent_target {
                    self.resolve_player_summon_melee(
                        index,
                        target_id,
                        events,
                        changed,
                        removed_entities,
                    )?;
                    return Ok(());
                }
                let guard_position = self.summon_command.guard_position.unwrap_or(owner_position);
                if self.entities[index].position == guard_position
                    || adjacent(self.entities[index].position, guard_position)
                {
                    None
                } else {
                    self.next_monster_step_toward(index, guard_position, true)
                }
            }
        };
        if let Some(next_position) = next_position {
            self.move_entity(index, next_position, events, changed, removed_entities)?;
        }
        Ok(())
    }

    fn move_entity(
        &mut self,
        index: usize,
        next_position: Position,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<ActorStepOutcome, CoreError> {
        let old_position = self.entities[index].position;
        let moving_entity_id = self.entities[index].id.clone();
        if let Some(target_index) = self
            .entities
            .iter()
            .position(|entity| entity.hp > 0 && entity.position == next_position)
        {
            if self.actor_can_kill_body_blocker(index, target_index) {
                let target = MonsterHostileTarget::Summon {
                    entity_id: self.entities[target_index].id.clone(),
                    kind_id: self.entities[target_index].kind_id.clone(),
                    position: next_position,
                };
                self.resolve_monster_melee_target(
                    index,
                    &target,
                    events,
                    changed,
                    removed_entities,
                )?;
                return Ok(ActorStepOutcome::Interacted);
            }
            if !self.actor_can_move_body_blocker(index, target_index) {
                return Ok(ActorStepOutcome::Blocked);
            }
            self.entities[target_index].position = old_position;
            self.wake_entity(target_index, events);
        } else {
            if let Some(broken) =
                self.try_monster_break_warding_glyph(index, next_position, events, changed)
                && !broken
            {
                return Ok(ActorStepOutcome::Interacted);
            }
            if !self.actor_can_enter_position(index, next_position) {
                match self.try_monster_door_interaction(index, next_position, events, changed) {
                    Some(true) => {}
                    Some(false) => return Ok(ActorStepOutcome::Interacted),
                    None => {
                        if !self.try_monster_destroy_terrain(index, next_position, events, changed)
                        {
                            return Ok(ActorStepOutcome::Blocked);
                        }
                    }
                }
            }
        }
        self.entities[index].position = next_position;
        changed.insert(old_position);
        changed.insert(next_position);
        if !self.trigger_actor_trap(index, next_position, events, changed, removed_entities)? {
            return Ok(ActorStepOutcome::Removed);
        }
        let Some(index) = self
            .entities
            .iter()
            .position(|entity| entity.id == moving_entity_id && entity.hp > 0)
        else {
            return Ok(ActorStepOutcome::Removed);
        };
        self.pick_up_items_under_monster(index, next_position, events, changed);
        self.destroy_items_under_monster(index, next_position, events, changed);
        Ok(ActorStepOutcome::Moved)
    }

    fn monster_can_use_ranged_melee(&self, index: usize, target: &MonsterHostileTarget) -> bool {
        let definition = self
            .actor_runtime_definition(&self.entities[index])
            .expect("monster actor definition must remain available");
        if !definition.ranged_melee
            || self.entities[index]
                .statuses
                .iter()
                .any(|status| matches!(status.kind_id.as_str(), STATUS_CONFUSION | STATUS_FEAR))
        {
            return false;
        }
        let origin = self.entities[index].position;
        let destination = target.position();
        let dx = origin.x.abs_diff(destination.x);
        let dy = origin.y.abs_diff(destination.y);
        if dx.max(dy) != 2 || dx.min(dy) >= 2 || !has_line_of_effect(self, origin, destination) {
            return false;
        }
        projectile_path_between(origin, destination, 2).is_some_and(|path| {
            path.into_iter()
                .filter(|position| *position != destination)
                .all(|position| {
                    position != self.player.position
                        && !self
                            .entities
                            .iter()
                            .any(|entity| entity.hp > 0 && entity.position == position)
                })
        })
    }

    fn resolve_riding(
        &mut self,
        direction: Direction,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let (dx, dy) = direction.delta();
        let target = Position {
            x: self.player.position.x + dx,
            y: self.player.position.y + dy,
        };
        if let Some(mount_id) = self.riding_actor_id.clone() {
            let Some(mount_index) = self
                .entities
                .iter()
                .position(|entity| entity.id == mount_id && entity.hp > 0)
            else {
                self.riding_actor_id = None;
                self.clear_riding_bond_for(&mount_id);
                events.push(DomainEvent::RidingUnavailable);
                return;
            };
            if !self.player_can_enter_unmounted_position(target)
                || self
                    .entities
                    .iter()
                    .any(|entity| entity.hp > 0 && entity.position == target)
            {
                events.push(DomainEvent::RidingUnavailable);
                return;
            }
            let target_kind_id = self.entities[mount_index].kind_id.clone();
            self.riding_actor_id = None;
            events.extend(self.relocate_player(target, changed));
            events.push(DomainEvent::RidingDismounted { target_kind_id });
            return;
        }

        self.try_mount(direction, true, events, changed);
    }

    fn roll_damage(&mut self, dice: u16, sides: u16) -> i32 {
        (0..dice).fold(0_i32, |total, _| {
            let roll = i32::try_from(self.rng.bounded(u64::from(sides)))
                .unwrap_or(i32::MAX)
                .saturating_add(1);
            total.saturating_add(roll)
        })
    }

    fn roll_weighted_index(&mut self, weights: &[u32]) -> usize {
        roll_weighted_index_with_rng(&mut self.rng, weights)
    }

    fn clamp_player_hp_to_effective_max(&mut self) {
        self.player.hp = self.player.hp.min(self.effective_player_max_hp());
    }

    fn allocate_item_instance_id(&mut self) -> Result<String, CoreError> {
        loop {
            let serial = self.next_item_instance_serial;
            let next = serial.checked_add(1).ok_or(CoreError::ItemIdExhausted)?;
            let candidate = format!("{GENERATED_ITEM_ID_PREFIX}{serial}");
            self.next_item_instance_serial = next;
            if !self.instance_id_exists(&candidate) {
                return Ok(candidate);
            }
        }
    }

    fn instance_id_exists(&self, candidate: &str) -> bool {
        self.player.id == candidate
            || self.entities.iter().any(|entity| entity.id == candidate)
            || self.items.iter().any(|item| item.id == candidate)
            || self
                .shop_states
                .values()
                .flat_map(|state| state.inventory.iter())
                .any(|item| item.id == candidate)
            || self
                .home_states
                .values()
                .flat_map(|state| state.inventory.iter())
                .any(|item| item.id == candidate)
    }

    fn abandon_paused_task(&mut self, task_id: &str) -> Option<Vec<Position>> {
        let world = self.content.world(&self.world_id)?;
        if (self.current_floor_id != world.initial_floor_id && self.current_town().is_none())
            || self
                .task_states
                .get(task_id)
                .is_none_or(|state| state.status != TaskStatusKindDto::Paused)
        {
            return None;
        }
        let members = task_floors(world, task_id)
            .filter(|floor| floor.lifecycle == FloorLifecycle::OneShot && floor.retakeable)
            .cloned()
            .collect::<Vec<_>>();
        let initial_required =
            task_initial_state(world, task_definition(world, task_id)?, &self.task_states).required;
        if members.is_empty() {
            return None;
        }

        self.discard_stored_task_floors(&members);
        let mut changed = BTreeSet::new();
        for definition in &members {
            let (Some(entry_id), Some(abandoned_id)) = (
                definition.entry_terrain_id.as_deref(),
                definition.abandoned_entry_terrain_id.as_deref(),
            ) else {
                continue;
            };
            for (index, terrain_id) in self.terrain.iter_mut().enumerate() {
                if terrain_id == entry_id {
                    *terrain_id = abandoned_id.to_owned();
                    changed.insert(Position {
                        x: i32::try_from(index % usize::from(self.width)).ok()?,
                        y: i32::try_from(index / usize::from(self.width)).ok()?,
                    });
                }
            }
        }
        let state = self
            .task_states
            .get_mut(task_id)
            .expect("paused task state must remain available");
        *state = abandoned_task_state(state, initial_required);
        self.fame_on_failure();
        Some(changed.into_iter().collect())
    }

    fn terrain_at(&self, position: Position) -> &str {
        &self.terrain[self.index(position).expect("validated map position")]
    }

    fn known_terrain_at(&self, position: Position) -> &str {
        let terrain_id = self.terrain_at(position);
        let definition = self
            .content
            .terrain(terrain_id)
            .expect("active terrain must remain available");
        if !self.revealed_terrain.contains(&position)
            && let Some(concealed_as) = definition.concealed_as_terrain_id.as_deref()
        {
            concealed_as
        } else {
            terrain_id
        }
    }

    fn relocate_player(
        &mut self,
        destination: Position,
        changed: &mut BTreeSet<Position>,
    ) -> Vec<DomainEvent> {
        let forget_after_move = self
            .content
            .world(&self.world_id)
            .and_then(|world| {
                world
                    .procedural_floors
                    .iter()
                    .find(|floor| floor.id == self.current_floor_id)
            })
            .is_some_and(|floor| floor.forget_after_move);
        let old_position = self.player.position;
        self.player.position = destination;
        if let Some(mount_id) = self.riding_actor_id.as_deref()
            && let Some(mount) = self
                .entities
                .iter_mut()
                .find(|entity| entity.id == mount_id)
        {
            mount.position = destination;
        }
        self.mark_current_town_visited();
        self.mark_shop_visited_at_player()
            .expect("shop entry must preserve validated item allocation");
        self.maintain_shop_at_player()
            .expect("shop maintenance must preserve validated item allocation");
        changed.insert(old_position);
        changed.insert(destination);

        let mut events = Vec::new();
        for position in self.passive_perception(&mut events) {
            changed.insert(position);
        }
        if let Some(PlayerTrapOutcome::Triggered {
            source_kind_id,
            damage,
        }) = self.trigger_player_trap(destination, &mut events)
        {
            events.push(DomainEvent::TrapTriggered {
                position: destination,
                damage,
            });
            if self.player_is_dead() {
                events.push(DomainEvent::PlayerDied {
                    source_kind_id,
                    method_id: None,
                    damage,
                });
            }
        }
        if forget_after_move {
            self.clear_current_floor_memory(changed);
        }
        events
    }

    fn warn_player_of_hidden_trap(
        &mut self,
        position: Position,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> bool {
        if self.revealed_terrain.contains(&position)
            || !self
                .player_equipment_passives()
                .contains(&EquipmentPassive::Warning)
        {
            return false;
        }
        let Some(index) = self.index(position) else {
            return false;
        };
        let is_trap = self
            .content
            .terrain(&self.terrain[index])
            .is_some_and(|terrain| terrain.trap.is_some());
        if !is_trap || self.rng.bounded(13) == 0 {
            return false;
        }
        self.revealed_terrain.insert(position);
        changed.insert(position);
        events.push(DomainEvent::ItemWarnedOfTrap { position });
        true
    }

    fn passive_perception(&mut self, events: &mut Vec<DomainEvent>) -> Vec<Position> {
        let candidates = TERRAIN_INTERACTION_DIRECTIONS
            .into_iter()
            .filter_map(|direction| {
                let position = self.position_in_direction(direction);
                let index = self.index(position)?;
                if self.revealed_terrain.contains(&position) {
                    return None;
                }
                let terrain = self.content.terrain(&self.terrain[index])?;
                Some((
                    position,
                    terrain.id.clone(),
                    terrain.perception_check_difficulty?,
                ))
            })
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return Vec::new();
        }
        let ability = self.player_derived_stats().perception_skill;
        let skill_id = self
            .content
            .skill_by_kind(SkillKind::Perception)
            .expect("validated perception skill must remain available")
            .id
            .clone();
        let mut discovered = Vec::new();
        for (position, terrain_id, difficulty) in candidates {
            let mut difficulty_pipeline = DerivedStatsPipeline::new();
            difficulty_pipeline.add(
                StatKind::ActionDifficulty,
                StatLayer::Environment,
                &terrain_id,
                difficulty,
            );
            let check = resolve_check(
                &mut self.rng,
                CheckContext {
                    kind: CheckKind::PassivePerception,
                    actor_id: self.player.id.clone(),
                    target_id: Some(terrain_id),
                    ability: ability.clone(),
                    difficulty: difficulty_pipeline
                        .resolve(StatKind::ActionDifficulty, StatBounds::NON_NEGATIVE),
                },
            );
            let succeeded = check.succeeded();
            events.push(DomainEvent::PerceptionChecked {
                position,
                succeeded,
                resolution: check.to_dto(skill_id.clone()),
            });
            if succeeded {
                self.revealed_terrain.insert(position);
                discovered.push(position);
            }
        }
        discovered
    }

    fn position_in_direction(&self, direction: rfb_protocol::Direction) -> Position {
        let (dx, dy) = direction.delta();
        Position {
            x: self.player.position.x + dx,
            y: self.player.position.y + dy,
        }
    }

    fn index(&self, position: Position) -> Option<usize> {
        if position.x < 0
            || position.y < 0
            || position.x >= i32::from(self.width)
            || position.y >= i32::from(self.height)
        {
            return None;
        }
        Some(position.y as usize * usize::from(self.width) + position.x as usize)
    }

    fn is_walkable(&self, position: Position) -> bool {
        self.index(position)
            .and_then(|index| self.content.terrain(&self.terrain[index]))
            .is_some_and(|terrain| terrain.walkable)
    }
}

fn item_quality_dto(quality: rfb_content::ItemQuality) -> ItemQualityDto {
    match quality {
        rfb_content::ItemQuality::Ordinary => ItemQualityDto::Ordinary,
        rfb_content::ItemQuality::Fine => ItemQualityDto::Fine,
        rfb_content::ItemQuality::Exceptional => ItemQualityDto::Exceptional,
    }
}

fn stat_modifiers_dto(modifiers: &StatModifiers) -> StatModifiersDto {
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
        device_power_bonus: modifiers.device_power_bonus,
    }
}

fn add_stat_modifiers_dto(total: &mut StatModifiersDto, addition: &StatModifiers) {
    total.attack = total.attack.saturating_add(addition.attack);
    total.defense = total.defense.saturating_add(addition.defense);
    total.max_hp = total.max_hp.saturating_add(addition.max_hp);
    total.strength = total.strength.saturating_add(addition.strength);
    total.intelligence = total.intelligence.saturating_add(addition.intelligence);
    total.wisdom = total.wisdom.saturating_add(addition.wisdom);
    total.dexterity = total.dexterity.saturating_add(addition.dexterity);
    total.constitution = total.constitution.saturating_add(addition.constitution);
    total.charisma = total.charisma.saturating_add(addition.charisma);
    total.speed = total.speed.saturating_add(addition.speed);
    total.spell_power_bonus = total
        .spell_power_bonus
        .saturating_add(addition.spell_power_bonus);
    total.device_power_bonus = total
        .device_power_bonus
        .saturating_add(addition.device_power_bonus);
}

fn equipment_bonuses_dto(bonuses: &EquipmentBonuses) -> EquipmentBonusesDto {
    EquipmentBonusesDto {
        life_percent: bonuses.life_percent,
        launcher_multiplier_delta_percent: bonuses.launcher_multiplier_delta_percent,
        base_shot_delta_percent: bonuses.base_shot_delta_percent,
        weapon_dice_bonus: bonuses.weapon_dice_bonus,
        melee_attacks_delta_percent: bonuses.melee_attacks_delta_percent,
        spell_capacity_bonus: bonuses.spell_capacity_bonus,
        magic_resistance_percent: bonuses.magic_resistance_percent,
        melee_attacks: bonuses.melee_attacks,
        melee_skill: bonuses.melee_skill,
        melee_damage: bonuses.melee_damage,
        ranged_skill: bonuses.ranged_skill,
        throwing_skill: bonuses.throwing_skill,
        device_skill: bonuses.device_skill,
        saving_throw_skill: bonuses.saving_throw_skill,
        saving_throw_skill_override: bonuses.saving_throw_skill_override,
        stealth_skill: bonuses.stealth_skill,
        search_skill: bonuses.search_skill,
        perception_skill: bonuses.perception_skill,
        disarming_skill: bonuses.disarming_skill,
        digging_skill: bonuses.digging_skill,
        infravision: bonuses.infravision,
        light_radius: bonuses.light_radius,
    }
}

const fn equipment_passive_dto(passive: EquipmentPassive) -> EquipmentPassiveDto {
    match passive {
        EquipmentPassive::Regeneration => EquipmentPassiveDto::Regeneration,
        EquipmentPassive::SeeInvisible => EquipmentPassiveDto::SeeInvisible,
        EquipmentPassive::Vampiric => EquipmentPassiveDto::Vampiric,
        EquipmentPassive::HoldLife => EquipmentPassiveDto::HoldLife,
        EquipmentPassive::Levitation => EquipmentPassiveDto::Levitation,
        EquipmentPassive::Warning => EquipmentPassiveDto::Warning,
        EquipmentPassive::SlowDigestion => EquipmentPassiveDto::SlowDigestion,
        EquipmentPassive::ReflectsBolts => EquipmentPassiveDto::ReflectsBolts,
        EquipmentPassive::FireAura => EquipmentPassiveDto::FireAura,
        EquipmentPassive::ColdAura => EquipmentPassiveDto::ColdAura,
        EquipmentPassive::ElectricityAura => EquipmentPassiveDto::ElectricityAura,
        EquipmentPassive::RevengeAura => EquipmentPassiveDto::RevengeAura,
        EquipmentPassive::ManaRegeneration => EquipmentPassiveDto::ManaRegeneration,
        EquipmentPassive::AntiMagic => EquipmentPassiveDto::AntiMagic,
        EquipmentPassive::AntiTeleport => EquipmentPassiveDto::AntiTeleport,
        EquipmentPassive::AntiSummoning => EquipmentPassiveDto::AntiSummoning,
        EquipmentPassive::NightVision => EquipmentPassiveDto::NightVision,
        EquipmentPassive::DualWielding => EquipmentPassiveDto::DualWielding,
        EquipmentPassive::NoEnchant => EquipmentPassiveDto::NoEnchant,
        EquipmentPassive::ShardsAura => EquipmentPassiveDto::ShardsAura,
        EquipmentPassive::ReducedManaCost => EquipmentPassiveDto::ReducedManaCost,
        EquipmentPassive::EasySpell => EquipmentPassiveDto::EasySpell,
        EquipmentPassive::AutoIdentify => EquipmentPassiveDto::AutoIdentify,
        EquipmentPassive::Blessed => EquipmentPassiveDto::Blessed,
        EquipmentPassive::EspAnimal => EquipmentPassiveDto::EspAnimal,
        EquipmentPassive::EspUndead => EquipmentPassiveDto::EspUndead,
        EquipmentPassive::EspDemon => EquipmentPassiveDto::EspDemon,
        EquipmentPassive::EspOrc => EquipmentPassiveDto::EspOrc,
        EquipmentPassive::EspTroll => EquipmentPassiveDto::EspTroll,
        EquipmentPassive::EspGiant => EquipmentPassiveDto::EspGiant,
        EquipmentPassive::EspDragon => EquipmentPassiveDto::EspDragon,
        EquipmentPassive::EspHuman => EquipmentPassiveDto::EspHuman,
        EquipmentPassive::EspGood => EquipmentPassiveDto::EspGood,
        EquipmentPassive::EspEvil => EquipmentPassiveDto::EspEvil,
        EquipmentPassive::EspLiving => EquipmentPassiveDto::EspLiving,
        EquipmentPassive::EspNonliving => EquipmentPassiveDto::EspNonliving,
        EquipmentPassive::Telepathy => EquipmentPassiveDto::Telepathy,
        EquipmentPassive::SustainStrength => EquipmentPassiveDto::SustainStrength,
        EquipmentPassive::SustainIntelligence => EquipmentPassiveDto::SustainIntelligence,
        EquipmentPassive::SustainWisdom => EquipmentPassiveDto::SustainWisdom,
        EquipmentPassive::SustainDexterity => EquipmentPassiveDto::SustainDexterity,
        EquipmentPassive::SustainConstitution => EquipmentPassiveDto::SustainConstitution,
        EquipmentPassive::SustainCharisma => EquipmentPassiveDto::SustainCharisma,
    }
}

const fn attribute_sustain_passive(attribute: AttributeKind) -> EquipmentPassive {
    match attribute {
        AttributeKind::Strength => EquipmentPassive::SustainStrength,
        AttributeKind::Intelligence => EquipmentPassive::SustainIntelligence,
        AttributeKind::Wisdom => EquipmentPassive::SustainWisdom,
        AttributeKind::Dexterity => EquipmentPassive::SustainDexterity,
        AttributeKind::Constitution => EquipmentPassive::SustainConstitution,
        AttributeKind::Charisma => EquipmentPassive::SustainCharisma,
    }
}

fn roll_weighted_index_with_rng(rng: &mut RfbRng, weights: &[u32]) -> usize {
    let total = weights.iter().map(|weight| u64::from(*weight)).sum();
    let mut roll = rng.bounded(total);
    for (index, weight) in weights.iter().enumerate() {
        let weight = u64::from(*weight);
        if roll < weight {
            return index;
        }
        roll -= weight;
    }
    unreachable!("validated positive weighted table must select an entry")
}

fn merge_equipment_bonuses(total: &mut EquipmentBonuses, addition: &EquipmentBonuses) {
    total.weapon_dice_bonus = total
        .weapon_dice_bonus
        .saturating_add(addition.weapon_dice_bonus);
    total.melee_attacks_delta_percent += addition.melee_attacks_delta_percent;
    total.spell_capacity_bonus += addition.spell_capacity_bonus;
    total.magic_resistance_percent += addition.magic_resistance_percent;
    total.life_percent = total.life_percent.saturating_add(addition.life_percent);
    total.launcher_multiplier_delta_percent = total
        .launcher_multiplier_delta_percent
        .saturating_add(addition.launcher_multiplier_delta_percent);
    total.base_shot_delta_percent = total
        .base_shot_delta_percent
        .saturating_add(addition.base_shot_delta_percent);
    total.melee_attacks = total.melee_attacks.saturating_add(addition.melee_attacks);
    total.melee_skill = total.melee_skill.saturating_add(addition.melee_skill);
    total.melee_damage = total.melee_damage.saturating_add(addition.melee_damage);
    total.ranged_skill = total.ranged_skill.saturating_add(addition.ranged_skill);
    total.throwing_skill = total.throwing_skill.saturating_add(addition.throwing_skill);
    total.device_skill = total.device_skill.saturating_add(addition.device_skill);
    total.saving_throw_skill = total
        .saving_throw_skill
        .saturating_add(addition.saving_throw_skill);
    total.stealth_skill = total.stealth_skill.saturating_add(addition.stealth_skill);
    total.search_skill = total.search_skill.saturating_add(addition.search_skill);
    total.perception_skill = total
        .perception_skill
        .saturating_add(addition.perception_skill);
    total.disarming_skill = total
        .disarming_skill
        .saturating_add(addition.disarming_skill);
    total.digging_skill = total.digging_skill.saturating_add(addition.digging_skill);
    total.infravision = total.infravision.saturating_add(addition.infravision);
    total.light_radius = total.light_radius.saturating_add(addition.light_radius);
}

fn throw_range(weight_tenths_pound: u16, mighty: bool) -> u16 {
    let budget = if mighty {
        BASE_THROW_RANGE_BUDGET.saturating_mul(6) / 5
    } else {
        BASE_THROW_RANGE_BUDGET
    };
    let maximum = if mighty {
        MAX_THROW_RANGE.saturating_add(2)
    } else {
        MAX_THROW_RANGE
    };
    (budget / weight_tenths_pound.max(1)).clamp(MIN_THROW_RANGE, maximum)
}

fn item_target_spec() -> TargetSpecDto {
    TargetSpecDto {
        modes: vec![TargetModeDto::Item],
        range: 0,
        requires_line_of_effect: false,
    }
}

fn projectile_target_spec(range: u16) -> TargetSpecDto {
    TargetSpecDto {
        modes: vec![
            TargetModeDto::Direction,
            TargetModeDto::Position,
            TargetModeDto::Entity,
        ],
        range,
        requires_line_of_effect: true,
    }
}

const fn slay_target_dto(target: SlayTarget) -> SlayTargetDto {
    match target {
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

const fn slay_level_dto(level: SlayLevel) -> SlayLevelDto {
    match level {
        SlayLevel::Slay => SlayLevelDto::Slay,
        SlayLevel::Kill => SlayLevelDto::Kill,
    }
}

const fn weapon_brand_dto(brand: WeaponBrand) -> WeaponBrandDto {
    match brand {
        WeaponBrand::Acid => WeaponBrandDto::Acid,
        WeaponBrand::Electricity => WeaponBrandDto::Electricity,
        WeaponBrand::Fire => WeaponBrandDto::Fire,
        WeaponBrand::Cold => WeaponBrandDto::Cold,
        WeaponBrand::Poison => WeaponBrandDto::Poison,
        WeaponBrand::Chaos => WeaponBrandDto::Chaos,
    }
}

const fn brand_damage_type(brand: WeaponBrand) -> DamageType {
    match brand {
        WeaponBrand::Acid => DamageType::Acid,
        WeaponBrand::Electricity => DamageType::Electricity,
        WeaponBrand::Fire => DamageType::Fire,
        WeaponBrand::Cold => DamageType::Cold,
        WeaponBrand::Poison => DamageType::Poison,
        WeaponBrand::Chaos => DamageType::Chaos,
    }
}

fn slay_target_matches(target: SlayTarget, definition: &rfb_content::ActorDefinition) -> bool {
    let has_tag = |expected: &str| definition.tags.iter().any(|tag| tag == expected);
    match target {
        SlayTarget::Animal => has_tag("animal"),
        SlayTarget::Evil => has_tag("evil"),
        SlayTarget::Good => has_tag("good"),
        SlayTarget::Living => !has_tag("demon") && !has_tag("undead") && !has_tag("nonliving"),
        SlayTarget::Human => has_tag("human"),
        SlayTarget::Undead => has_tag("undead"),
        SlayTarget::Demon => has_tag("demon"),
        SlayTarget::Orc => has_tag("orc"),
        SlayTarget::Troll => has_tag("troll"),
        SlayTarget::Giant => has_tag("giant"),
        SlayTarget::Dragon => has_tag("dragon"),
    }
}

fn actor_matches_category(definition: &rfb_content::ActorDefinition, category: &str) -> bool {
    if category == "any-monster" {
        return definition.role == ActorRole::Monster;
    }
    if category == "normal-monster" {
        return definition.role == ActorRole::Monster
            && !definition.tags.iter().any(|tag| tag == "invisible");
    }
    if category == "living" {
        return !definition
            .tags
            .iter()
            .any(|tag| matches!(tag.as_str(), "demon" | "undead" | "nonliving"));
    }
    definition.tags.iter().any(|tag| tag == category)
}

fn actor_answers_summons(definition: &rfb_content::ActorDefinition) -> bool {
    !definition.tags.iter().any(|tag| tag == "no-summon")
}

/// FrogComposband's melee `slay_tiers`, expressed in tenths. Integer
/// truncation is preserved (the mid-tier kill multiplier is 46, not 46.25).
const fn slay_multiplier(target: SlayTarget, level: SlayLevel) -> i32 {
    let tier = match target {
        SlayTarget::Evil | SlayTarget::Good | SlayTarget::Living => 0,
        SlayTarget::Animal | SlayTarget::Human => 1,
        SlayTarget::Undead
        | SlayTarget::Demon
        | SlayTarget::Orc
        | SlayTarget::Troll
        | SlayTarget::Giant
        | SlayTarget::Dragon => 2,
    };
    match (tier, level) {
        (0, SlayLevel::Slay) => 19,
        (1, SlayLevel::Slay) => 24,
        (2, SlayLevel::Slay) => 28,
        (0, SlayLevel::Kill) => 40,
        (1, SlayLevel::Kill) => 46,
        (2, SlayLevel::Kill) => 56,
        _ => unreachable!(),
    }
}

const fn ability_detect_subject_dto(
    subject: AbilityDetectSubjectDefinition,
) -> AbilityDetectSubjectDto {
    match subject {
        AbilityDetectSubjectDefinition::Terrain => AbilityDetectSubjectDto::Terrain,
        AbilityDetectSubjectDefinition::Actor => AbilityDetectSubjectDto::Actor,
        AbilityDetectSubjectDefinition::Item => AbilityDetectSubjectDto::Item,
        AbilityDetectSubjectDefinition::Gold => AbilityDetectSubjectDto::Gold,
        AbilityDetectSubjectDefinition::Curse => AbilityDetectSubjectDto::Curse,
    }
}

const fn ability_genocide_scope_dto(
    scope: AbilityGenocideScopeDefinition,
) -> AbilityGenocideScopeDto {
    match scope {
        AbilityGenocideScopeDefinition::Single => AbilityGenocideScopeDto::Single,
        AbilityGenocideScopeDefinition::Glyph => AbilityGenocideScopeDto::Glyph,
        AbilityGenocideScopeDefinition::Nearby => AbilityGenocideScopeDto::Nearby,
    }
}

const fn resistance_rank(level: ResistanceLevel) -> u8 {
    match level {
        ResistanceLevel::Vulnerable => 0,
        ResistanceLevel::Normal => 1,
        ResistanceLevel::Resistant => 2,
        ResistanceLevel::Strong => 3,
        ResistanceLevel::Immune => 4,
    }
}

fn resisted_status_duration(requested: u32, resistance: ResistanceLevel) -> u32 {
    status_effects::resisted_status_duration_with_percent(requested, resistance.reduction_percent())
}

fn squared_distance(left: Position, right: Position) -> i32 {
    let dx = left.x - right.x;
    let dy = left.y - right.y;
    dx * dx + dy * dy
}

fn chebyshev_distance(left: Position, right: Position) -> u32 {
    left.x.abs_diff(right.x).max(left.y.abs_diff(right.y))
}

const fn monster_pack_behavior_dto(behavior: MonsterPackBehavior) -> MonsterPackBehaviorDto {
    match behavior {
        MonsterPackBehavior::Seek => MonsterPackBehaviorDto::Seek,
        MonsterPackBehavior::Surround => MonsterPackBehaviorDto::Surround,
        MonsterPackBehavior::GuardLeader => MonsterPackBehaviorDto::GuardLeader,
        MonsterPackBehavior::GuardPosition => MonsterPackBehaviorDto::GuardPosition,
        MonsterPackBehavior::Lure => MonsterPackBehaviorDto::Lure,
        MonsterPackBehavior::Shoot => MonsterPackBehaviorDto::Shoot,
        MonsterPackBehavior::MaintainDistance => MonsterPackBehaviorDto::MaintainDistance,
    }
}

/// Content-declared resistances are stamped whenever an entity is built from
/// its definition; loaded saves keep their stored profiles untouched.
fn stamped_spawn(mut actor: Actor, definition: &rfb_content::ActorDefinition) -> Actor {
    actor.resistances = definition_resistance_profile(definition);
    actor.nice = definition.force_sleep;
    actor
}

fn actor_spawn_max_hp(rng: &mut RfbRng, definition: &rfb_content::ActorDefinition) -> i32 {
    let Some(hit_points) = definition.hit_point_dice else {
        return definition.max_hp;
    };
    if hit_points.force_maximum {
        return i32::from(hit_points.dice).saturating_mul(i32::from(hit_points.sides));
    }
    (0..hit_points.dice).fold(0_i32, |total, _| {
        let roll = i32::try_from(rng.bounded(u64::from(hit_points.sides)))
            .unwrap_or(i32::MAX)
            .saturating_add(1);
        total.saturating_add(roll)
    })
}

fn spawn_actor_from_definition(
    rng: &mut RfbRng,
    definition: &rfb_content::ActorDefinition,
    id: &str,
    position: Position,
    energy_need: i32,
    alerted: bool,
) -> Actor {
    stamped_spawn(
        actor_from_runtime_spawn(
            id,
            &definition.id,
            position,
            actor_spawn_max_hp(rng, definition),
            definition.speed,
            energy_need,
            alerted,
        ),
        definition,
    )
}

fn actor_starts_alerted(definition: &rfb_content::ActorDefinition) -> bool {
    definition
        .awareness
        .as_ref()
        .is_none_or(|awareness| awareness.starts_alerted)
}

fn direction_toward(from: Position, to: Position) -> Option<Direction> {
    match ((to.x - from.x).signum(), (to.y - from.y).signum()) {
        (0, -1) => Some(Direction::North),
        (1, -1) => Some(Direction::NorthEast),
        (1, 0) => Some(Direction::East),
        (1, 1) => Some(Direction::SouthEast),
        (0, 1) => Some(Direction::South),
        (-1, 1) => Some(Direction::SouthWest),
        (-1, 0) => Some(Direction::West),
        (-1, -1) => Some(Direction::NorthWest),
        _ => None,
    }
}

fn monster_casting_cooldown(frequency_percent: u8) -> u16 {
    u16::from(100_u8.div_ceil(frequency_percent))
}

pub fn load_built_in_content() -> Result<Arc<ContentCatalog>, CoreError> {
    // The built-in pack is immutable for the lifetime of the process, so the
    // decode + validation pass only needs to run once; failures stay uncached.
    static BUILT_IN_CATALOG: OnceLock<Arc<ContentCatalog>> = OnceLock::new();
    if let Some(catalog) = BUILT_IN_CATALOG.get() {
        return Ok(Arc::clone(catalog));
    }
    let catalog = Arc::new(ContentCatalog::from_bytes(BUILT_IN_CONTENT_BYTES)?);
    Ok(Arc::clone(BUILT_IN_CATALOG.get_or_init(|| catalog)))
}

#[cfg(test)]
mod tests;
