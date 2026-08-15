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
        DamageOutcome, DamagePacket, EffectOutcome, EffectSpec, EffectTarget, STATUS_ANTI_MAGIC,
        STATUS_BASIC_RESISTANCE, STATUS_BERSERK, STATUS_BLEEDING, STATUS_BLINDNESS,
        STATUS_CONFUSION, STATUS_DEMON_LORD_TRANSFORMATION, STATUS_DEVICE_MASTERY, STATUS_FEAR,
        STATUS_FIRE_AURA, STATUS_GIANT_STRENGTH, STATUS_HALLUCINATION, STATUS_HASTE,
        STATUS_HOLD_LIFE, STATUS_HOLY_AURA, STATUS_INVENTORY_PROTECTION, STATUS_INVULNERABILITY,
        STATUS_LEVITATION, STATUS_LIGHT_SPEED, STATUS_MAGIC_RESISTANCE, STATUS_NO_AIR,
        STATUS_PARALYSIS, STATUS_PLAYER_POLYMORPH, STATUS_POISON, STATUS_PROTECTION_FROM_EVIL,
        STATUS_REGENERATION, STATUS_SEE_INVISIBLE, STATUS_SIGHT, STATUS_SLEEP, STATUS_SLOW,
        STATUS_STUN, STATUS_SUSTAIN_CHARISMA, STATUS_SUSTAIN_CONSTITUTION,
        STATUS_SUSTAIN_DEXTERITY, STATUS_SUSTAIN_INTELLIGENCE, STATUS_SUSTAIN_STRENGTH,
        STATUS_SUSTAIN_WISDOM, STATUS_TELEPATHY, STATUS_THERMAL_RESISTANCE, STATUS_TRANSCENDENCE,
        STATUS_TSUYOSHI, STATUS_ULTIMATE_RESISTANCE, STATUS_UNDERSTANDING, STATUS_UNWELL,
        STATUS_VENGEANCE, STATUS_WRAITHFORM, StatusApplication, StatusChange, StatusInstance,
        StatusStacking, apply_effect, apply_status, resolve_damage,
    },
    error::CoreError,
    event::{
        BoltReflectionOutcome, DomainEvent, ItemAttributeChange, ProjectileTrace,
        SelfKnowledgeReport, project_events,
    },
    rng::{RNG_ALGORITHM, RfbRng},
    save::{
        GENERATED_ITEM_ID_PREFIX, actor_from_runtime_spawn, actor_from_spawn,
        actor_max_hp_is_valid, derive_next_item_instance_serial, initial_item_fuel,
        position_from_content,
    },
    scheduler::{
        INITIAL_MONSTER_ENERGY_NEED, INITIAL_PLAYER_ENERGY_NEED, STANDARD_ACTION_COST, energy_gain,
        gain_energy, spend_energy,
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
    AbilityGenocideScopeDefinition, AbilityLevelScalingCurveDefinition,
    AbilityLevelScalingDefinition, AbilityLevelScalingField, AbilityRandomTargetDefinition,
    AbilitySpellPowerDefinition, AbilitySpellPowerField, AbilityStatusStackingDefinition,
    AbilityTargetDefinition, AbilityTargetModeDefinition, AbilityTerrainBeamOperationDefinition,
    ActorDamageType, ActorMovementMode, ActorResistanceLevel, ActorRole,
    AffixPropertyBundleDefinition, AmmunitionBehaviorDefinition, AmmunitionTypeDefinition,
    CastingAttribute, CastingCapacityFormula, CastingFailureFormula, CastingLearningFormula,
    CastingProfileDefinition, CastingRealmProfileDefinition, CastingStudyMode,
    ClassAbilityDefinition, ContentCatalog, DraconianStrikeModeDefinition,
    DungeonInstanceLifecycle, EncounterEntryDefinition, EncounterTableDefinition, EquipmentBonuses,
    EquipmentPassive, FloorLifecycle, InnatePowerCostScalingCurveDefinition, InnatePowerDefinition,
    ItemAttributeDefinition, ItemCurseSeverityDefinition, ItemCurseTargetDefinition,
    ItemDestructionElement, ItemDeviceGenerationDefinition, ItemEnchantmentRollDefinition,
    ItemShatterEffectDefinition, ItemSummonLevelSourceDefinition, ItemSummonSelectorDefinition,
    ItemUseEffectDefinition, MeleeBlowEffectDefinition, MonsterDropKindDefinition,
    MonsterPackBehavior, MutationPeriodicEffectDefinition, PlayerAbilityDefinition,
    ProceduralLayoutMode, ProceduralMazeDefinition, ProceduralPitDefinition,
    ProceduralRoomGeometryDefinition, ProceduralRoomPlacement, ProceduralRoomShape,
    ProceduralStreamerCandidateDefinition, RaceDefinition, RidingWeaponKindDefinition, SkillKind,
    SlayLevel, SlayTarget, SniperShotModeDefinition, StartingItemDefinition, StatModifiers,
    TaskObjectiveKind, TechniqueAttribute, TerrainDiggingResolution, TerrainFeatureEntryDefinition,
    ThemeVaultCandidateDefinition, WeaponBrand, affix_is_compatible_with_item,
};
use rfb_protocol::{
    AbilityAreaDamageResolutionDto, AbilityBanishTargetDto, AbilityBeamDamageResolutionDto,
    AbilityCastResolutionDto, AbilityConeDamageResolutionDto, AbilityControlOutcomeDto,
    AbilityDetectResolutionDto, AbilityDetectSubjectDto, AbilityEffectResolutionDto,
    AbilityEffectSkipReasonDto, AbilityEffectSpecDto, AbilityEffectsResolutionDto,
    AbilityGenocideScopeDto, AbilityMonsterProbeResolutionDto, AbilityProbeAlignmentDto,
    AbilityProbeTargetDto, AbilityProficiencyRankDto, AbilityProgressSaveDto,
    AbilityRandomBranchSpecDto, AbilityRandomTargetDto, AbilityRecallActionDto, AbilitySourceDto,
    AbilityStatusChangeDto, AbilityStatusStackingDto, AbilitySummonCandidateSpecDto,
    AbilitySummonResolutionDto, AbilityTeleportResolutionDto, AbilityTerrainBeamOperationDto,
    AbilityTerrainTransformResolutionDto, AbilityVisibleDamageResolutionDto, AttackProfileDto,
    AutoGetModeDto, CampaignStatusDto, CellLightDto, CellVisualDto, DamageDiceDto, Direction,
    EntityFactionDto, EquipmentBonusesDto, EquipmentPassiveDto, GameCommandEnvelope, GameUpdate,
    GoldAppearanceDto, HealingResolutionDto, ItemActivationDto, ItemChargesDto, ItemCurseEffectDto,
    ItemCurseRemovalResolutionDto, ItemCurseResolutionDto, ItemCurseSeverityDto,
    ItemEnchantmentComponentResolutionDto, ItemEnchantmentResolutionDto, ItemEnchantmentsDto,
    ItemIdentificationDto, ItemIdentifyResolutionDto, ItemKnowledgeDto, ItemOriginKindDto,
    ItemPropertyDto, ItemQualityDto, LocaleDto, MapScaleDto, MeleeBlowDto, MeleeRoutineDto,
    MonsterAbilityCandidateResolutionDto, MonsterAbilityCastResolutionDto,
    MonsterAbilityDecisionResolutionDto, MonsterAbilityRejectionReasonDto,
    MonsterAbilityTargetResolutionDto, MonsterAlignmentDto, MonsterDisplacementResolutionDto,
    MonsterPackBehaviorDto, MonsterPackRoleDto, PendingAbilityDirectionDto,
    PendingMutationDirectionDto, Position, ProbedMonsterDto, ProjectileProfileDto, RecallStateDto,
    ResistanceDto, ResourcePoolSaveDto, ResourceRecoveryResolutionDto, RestResolutionDto,
    RestStopReasonDto, SlayDto, SlayLevelDto, SlayTargetDto, SniperShotModeDto, StatModifiersDto,
    SummonCommandDto, SummonCommandModeDto, SummonCommandResolutionDto, TargetModeDto,
    TargetSelection, TargetSpecDto, TaskStatusKindDto, ThrowProfileDto, VirtueDto, VirtueKindDto,
    WeaponBrandDto, WeaponTraitDto,
};

mod abilities;
mod bounty;
pub(crate) use bounty::BountyOfficeOutcome;
mod capabilities;
mod capture_ball;
mod chaos_patron;
mod damage;
mod death;
mod ego;
mod environment_combat;
mod floor;
mod gold;
mod ground_item_effects;
mod hunger;
mod inventory;
mod item_combat;
mod item_curses;
mod item_knowledge;
mod item_use;
mod lighting;
mod mining;
mod mogaminator;
mod monster_abilities;
mod monster_ai;
mod monster_combat;
mod monster_ecology;
mod movement;
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
mod riding_bond;
mod riding_proficiency;
mod snapshot;
mod status_effects;
mod tasks;
mod terrain;
pub(crate) mod town;
mod travel;
mod turn;
mod validation;
mod virtues;
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
use capabilities::{
    HealingOutcome, HealingRequest, ResourceRestorationRequest, StatusRemovalRequest,
    apply_healing, apply_resource_restoration, apply_status_application, apply_status_removal,
};
use damage::{
    FatalityPolicy, commit_damage_application, commit_final_player_damage, plan_damage_application,
    process_actor_status_tick, process_actor_status_tick_with, scale_damage_outcome,
};
use ego::{
    EgoMaterialization, materialize_ego_with_rng, materialize_rfb_harp_intrinsic_with_rng,
    merge_affix_properties, roll_and_materialize_rfb_ego_from_affixes_with_rng,
};
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
use mutations::LuckBias;
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
    combine_percentages, initial_character_attributes, initial_resource_pool,
    profile_resource_maximum, resolve_character_build,
};
use status_effects::{
    ability_status_stacking_dto, apply_ability_status_effect, remove_ability_status_effect,
};
use tasks::{
    CampaignState, TaskState, abandoned_task_state, initial_task_states, task_applies_to_floor,
    task_definition, task_floors, task_initial_state, task_objectives,
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
pub const STATE_HASH_SCHEMA_VERSION: u16 = 108;
#[cfg(test)]
const RFB_WARRIOR_BUILD_ID: &str = "demo.build.warrior";
const VISIBILITY_RADIUS: i32 = 8;
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
const SURFACE_AMBIENT_LIGHT: u8 = 48;
const DUNGEON_AMBIENT_LIGHT: u8 = 0;
const ROOM_GLOW_LIGHT: u8 = 48;
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
const ITEM_LIGHT_RADIUS: i32 = 4;

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
const PLAYER_LIGHT_COLOR: u32 = 0xffd7a3;
const ACTOR_LIGHT_COLOR: u32 = 0xff8a4c;
const ITEM_LIGHT_COLOR: u32 = 0x8ad9ff;
const RECHARGING_ITEM_SOURCE_DESTRUCTION_ONE_IN: u16 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
struct GenocideResolution {
    removed_entity_ids: Vec<String>,
    resisted_entity_ids: Vec<String>,
    fatigue_damage: i32,
}

#[derive(Debug, Clone, Copy)]
struct CategorySummonSpec<'a> {
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
struct LootContext {
    table_id: String,
    floor_id: String,
    depth: u16,
    source: LootSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemGenerationMode {
    Ordinary,
    Good,
    Great,
    Artifact,
}

impl ItemGenerationMode {
    const fn minimum_quality(self) -> rfb_content::ItemQuality {
        match self {
            Self::Ordinary => rfb_content::ItemQuality::Ordinary,
            Self::Good => rfb_content::ItemQuality::Fine,
            Self::Great | Self::Artifact => rfb_content::ItemQuality::Exceptional,
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
struct GeneratedItemDraft {
    kind_id: String,
    quantity: u32,
    origin_kind: Option<ItemOriginKindDto>,
    quality: ItemQualityDto,
    affix_ids: Vec<String>,
    rolled_affixes: Vec<RolledAffixState>,
    intrinsic_properties: AffixPropertyBundleDefinition,
    enchantments: ItemEnchantmentsDto,
    curse: Option<ItemCurseSeverityDto>,
    activation: Option<ItemActivationDto>,
    charges: Option<ItemChargesDto>,
    fuel: Option<rfb_protocol::ItemFuelDto>,
}

impl GeneratedItemDraft {
    fn into_item_instance(self, id: String, location: ItemLocation) -> ItemInstance {
        ItemInstance {
            id,
            kind_id: self.kind_id,
            quantity: self.quantity,
            inscription: None,
            origin_actor_kind_id: None,
            origin_kind: self.origin_kind,
            damage_dice_override: None,
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
enum LootSource {
    MonsterCarried { actor_id: String },
    MonsterDeath { actor_id: String },
    FloorRoom { room_id: String, spawn_id: String },
    Vault { vault_id: String, spawn_id: String },
    ItemUse { item_id: String },
    Rubble { position: Position },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DungeonState {
    suppressed: bool,
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

fn dungeon_substitution_uses_alternate(
    primary: &rfb_content::DungeonDefinition,
    alternate: &rfb_content::DungeonDefinition,
    seed: u64,
) -> bool {
    let primary_index = u64::from(
        primary
            .legacy_index
            .expect("validated substituted dungeon must retain a legacy index"),
    );
    let alternate_index = u64::from(
        alternate
            .legacy_index
            .expect("validated alternate dungeon must retain a legacy index"),
    );
    let pair_modulus = primary_index * 48 + alternate_index;
    let selected = seed % (pair_modulus * 2) >= pair_modulus;
    selected
        && primary
            .substitution
            .as_ref()
            .and_then(|substitution| substitution.alternate_gate_one_in)
            .is_none_or(|one_in| seed.is_multiple_of(u64::from(one_in)))
}

fn initial_dungeon_states(
    world: &rfb_content::WorldDefinition,
    seed: u64,
) -> BTreeMap<String, DungeonState> {
    let mut states = base_dungeon_states(world);
    for primary in &world.dungeons {
        let Some(substitution) = &primary.substitution else {
            continue;
        };
        let alternate = world
            .dungeons
            .iter()
            .find(|dungeon| dungeon.id == substitution.alternate_dungeon_id)
            .expect("validated dungeon substitution must retain its alternate");
        let suppressed_id = if dungeon_substitution_uses_alternate(primary, alternate, seed) {
            &primary.id
        } else {
            &alternate.id
        };
        states
            .get_mut(suppressed_id)
            .expect("substituted dungeon state must remain available")
            .suppressed = true;
    }
    states
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

/// Body slots come from the build's race when it declares any, otherwise
/// the standard template applies. Games without a build use the standard
/// template as well.
fn resolve_body_slots(
    content: &ContentCatalog,
    identity: Option<&CharacterBuildIdentity>,
) -> Result<Vec<BodySlot>, CoreError> {
    let Some(identity) = identity else {
        return Ok(standard_body_slots());
    };
    let (_, race, _, _) = build_definitions(content, identity)?;
    Ok(body_slots_for_race(race))
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
        || (declared_slot_type == "tool" && target_slot_type == "weapon")
}

fn append_starting_items(
    content: &ContentCatalog,
    identity: Option<&CharacterBuildIdentity>,
    body_slots: &[BodySlot],
    items: &mut Vec<ItemInstance>,
    next_serial: &mut u64,
    rng: &mut RfbRng,
) -> Result<(), CoreError> {
    let Some(identity) = identity else {
        return Ok(());
    };
    let (build, race, class, personality) = build_definitions(content, identity)?;
    for starting_item in race
        .starting_items
        .iter()
        .chain(class.starting_items.iter())
        .chain(personality.starting_items.iter())
        .chain(build.starting_items.iter())
    {
        append_starting_item(content, starting_item, body_slots, items, next_serial, rng)?;
    }
    Ok(())
}

fn append_starting_item(
    content: &ContentCatalog,
    starting_item: &StartingItemDefinition,
    body_slots: &[BodySlot],
    items: &mut Vec<ItemInstance>,
    next_serial: &mut u64,
    rng: &mut RfbRng,
) -> Result<(), CoreError> {
    let definition = content
        .item(&starting_item.item_kind_id)
        .ok_or_else(|| CoreError::UnknownItem(starting_item.item_kind_id.clone()))?;
    let location = if starting_item.equipped {
        let slot_type = definition
            .equipment_slot
            .as_deref()
            .ok_or(CoreError::InvalidSave("starting equipment is invalid"))?;
        let occupied = |slot_id: &str| {
            items.iter().any(|item| {
                matches!(
                    &item.location,
                    ItemLocation::Equipped { slot_id: equipped } if equipped == slot_id
                )
            })
        };
        let slot = body_slot_instance_for_type(body_slots, slot_type, occupied)
            .ok_or(CoreError::InvalidSave("starting equipment is invalid"))?;
        ItemLocation::Equipped {
            slot_id: slot.id.clone(),
        }
    } else {
        ItemLocation::Inventory
    };
    let id = format!("{GENERATED_ITEM_ID_PREFIX}{next_serial}");
    *next_serial = next_serial
        .checked_add(1)
        .ok_or(CoreError::ItemIdExhausted)?;
    let (activation, mut charges) =
        initial_item_runtime_state(content, rng, &starting_item.item_kind_id, &[], 1);
    if starting_item.fully_charged {
        let charges = charges
            .as_mut()
            .expect("validated fully charged starting item must be a device");
        charges.current = charges.maximum;
    }
    let quantity = starting_item
        .maximum_quantity
        .map_or(starting_item.quantity, |maximum| {
            starting_item.quantity
                + u32::try_from(rng.bounded(u64::from(maximum - starting_item.quantity + 1)))
                    .expect("validated birth item quantity must fit u32")
        });
    items.push(ItemInstance {
        id,
        kind_id: starting_item.item_kind_id.clone(),
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
        curse: initial_item_curse(content, &starting_item.item_kind_id),
        permanent_destruction_immunities: Default::default(),
        activation,
        charges,
        fuel: initial_item_fuel(content, &starting_item.item_kind_id),
        device_recovery_progress: 0,
        captured_actor: None,
        location,
    });
    Ok(())
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
) -> Option<&'a ItemDeviceGenerationDefinition> {
    let definition = content.item(kind_id)?;
    definition.device_generation.as_ref().or_else(|| {
        affix_ids.iter().find_map(|affix_id| {
            content
                .affix(affix_id)
                .and_then(|affix| affix.device_generation.as_ref())
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
    let Some(generation) = item_device_generation(content, kind_id, affix_ids) else {
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
    ability_progress: BTreeMap<String, AbilityProgress>,
    entities: Vec<Actor>,
    items: Vec<ItemInstance>,
    gold: u32,
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
    pub const DEFAULT_PLAYER_NAME: &'static str = "RFB Demo Character";

    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self::from_content(
            seed,
            load_built_in_content().expect("built-in content should decode"),
            DEFAULT_WORLD_ID,
        )
        .expect("built-in world should create a game")
    }

    pub fn new_with_build(seed: u64, build_id: &str) -> Result<Self, CoreError> {
        Self::from_content_with_build(
            seed,
            load_built_in_content().expect("built-in content should decode"),
            DEFAULT_WORLD_ID,
            build_id,
        )
    }

    pub fn new_with_build_and_name(
        seed: u64,
        build_id: &str,
        player_name: &str,
    ) -> Result<Self, CoreError> {
        Self::from_content_internal(
            seed,
            load_built_in_content().expect("built-in content should decode"),
            DEFAULT_WORLD_ID,
            Some(build_id),
            None,
            player_name,
        )
    }

    pub fn new_with_build_race_and_name(
        seed: u64,
        build_id: &str,
        race_id: &str,
        player_name: &str,
    ) -> Result<Self, CoreError> {
        Self::from_content_internal(
            seed,
            load_built_in_content().expect("built-in content should decode"),
            DEFAULT_WORLD_ID,
            Some(build_id),
            Some(race_id),
            player_name,
        )
    }

    pub fn from_content(
        seed: u64,
        content: Arc<ContentCatalog>,
        world_id: &str,
    ) -> Result<Self, CoreError> {
        Self::from_content_internal(
            seed,
            content,
            world_id,
            None,
            None,
            Self::DEFAULT_PLAYER_NAME,
        )
    }

    pub fn from_content_with_build(
        seed: u64,
        content: Arc<ContentCatalog>,
        world_id: &str,
        build_id: &str,
    ) -> Result<Self, CoreError> {
        Self::from_content_internal(
            seed,
            content,
            world_id,
            Some(build_id),
            None,
            Self::DEFAULT_PLAYER_NAME,
        )
    }

    fn from_content_internal(
        seed: u64,
        content: Arc<ContentCatalog>,
        world_id: &str,
        build_id: Option<&str>,
        race_id: Option<&str>,
        player_name: &str,
    ) -> Result<Self, CoreError> {
        let player_name = normalize_player_name(player_name).ok_or(CoreError::InvalidPlayerName)?;
        let world = content
            .world(world_id)
            .ok_or_else(|| CoreError::UnknownWorld(world_id.to_owned()))?;
        let build = resolve_character_build(
            &content,
            build_id.or(world.player_build_id.as_deref()),
            race_id,
        )?;
        let starts_at_night = build
            .as_ref()
            .and_then(|identity| content.race(&identity.race_id))
            .is_some_and(|race| race.tags.iter().any(|tag| tag == "night-start"));
        let starting_world_tick = if starts_at_night {
            wilderness::WILDERNESS_NIGHT_START_TICK
        } else {
            0
        };
        let width = world.width;
        let height = world.height;
        let mut terrain =
            vec![world.fill_terrain_id.clone(); usize::from(width) * usize::from(height)];
        for y in 0..height {
            for x in 0..width {
                if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
                    terrain[usize::from(y) * usize::from(width) + usize::from(x)] =
                        world.border_terrain_id.clone();
                }
            }
        }
        for terrain_override in &world.terrain_overrides {
            for position in &terrain_override.positions {
                terrain[usize::from(position.y) * usize::from(width) + usize::from(position.x)] =
                    terrain_override.terrain_id.clone();
            }
        }
        let player_kind_id = build
            .as_ref()
            .and_then(|identity| content.build(&identity.build_id))
            .and_then(|build| build.player_actor_id.as_deref())
            .unwrap_or(&world.player.kind_id);
        let player_definition = content
            .actor(player_kind_id)
            .ok_or_else(|| CoreError::UnknownActor(player_kind_id.to_owned()))?;
        let player = actor_from_spawn(
            &world.player.instance_id,
            player_kind_id,
            world.player.position,
            player_definition.max_hp,
            player_definition.speed,
            INITIAL_PLAYER_ENERGY_NEED,
            true,
        );
        let mut rng = RfbRng::seeded(seed);
        let virtues = virtues::initial_virtues(&content, build.as_ref(), &mut rng);
        let gold = gold::starting_gold(build.as_ref(), &mut rng);
        let starting_ration_quantity = hunger::starting_ration_quantity(build.as_ref(), &mut rng);
        let starting_torches = lighting::starting_torch_supply(build.as_ref(), &mut rng);
        let mut progress = CharacterProgress::new(seed, player_definition.max_hp);
        if let Some(identity) = build.as_ref() {
            let (definition, _, class, _) = build_definitions(&content, identity)?;
            progress.attributes = initial_character_attributes(definition);
            progress.maximum_attributes = progress.attributes;
            progress.riding_proficiency = class.riding_proficiency.initial;
        }
        progress.replace_skills(character_skill_progress(
            &content,
            build.as_ref(),
            progress.level,
        )?);
        let mut entities = world
            .actors
            .iter()
            .map(|spawn| {
                let definition = content
                    .actor(&spawn.kind_id)
                    .ok_or_else(|| CoreError::UnknownActor(spawn.kind_id.clone()))?;
                Ok(spawn_actor_from_definition(
                    &mut rng,
                    definition,
                    &spawn.instance_id,
                    position_from_content(spawn.position),
                    INITIAL_MONSTER_ENERGY_NEED,
                    actor_starts_alerted(definition),
                ))
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
        for dungeon in world.dungeons.iter().filter(|_| world.wilderness.is_none()) {
            let Some(guardian) = &dungeon.entrance_guardian else {
                continue;
            };
            let definition = content
                .actor(&guardian.actor_kind_id)
                .ok_or_else(|| CoreError::UnknownActor(guardian.actor_kind_id.clone()))?;
            let mut actor = spawn_actor_from_definition(
                &mut rng,
                definition,
                &guardian.instance_id,
                position_from_content(guardian.position),
                INITIAL_MONSTER_ENERGY_NEED,
                actor_starts_alerted(definition),
            );
            actor.pack = Some(MonsterPackIdentity {
                id: guardian.instance_id.clone(),
                leader_id: guardian.instance_id.clone(),
                role: MonsterPackRoleDto::Leader,
                behavior: MonsterPackBehaviorDto::GuardPosition,
            });
            entities.push(actor);
        }
        let mut items = world
            .items
            .iter()
            .map(|spawn| {
                let materialization = materialize_ego_with_rng(
                    &content,
                    &mut rng,
                    &spawn.kind_id,
                    spawn.affix_ids.clone(),
                    |_| 1,
                    1,
                );
                let mut item = ItemInstance {
                    id: spawn.instance_id.clone(),
                    kind_id: spawn.kind_id.clone(),
                    quantity: spawn.quantity,
                    inscription: None,
                    origin_actor_kind_id: None,
                    origin_kind: None,
                    damage_dice_override: None,
                    discount_percent: 0,
                    quality: item_quality_dto(spawn.quality),
                    affix_ids: Vec::new(),
                    rolled_affixes: Vec::new(),
                    intrinsic_properties: Default::default(),
                    enchantments: ItemEnchantmentsDto::default(),
                    curse: initial_item_curse(&content, &spawn.kind_id),
                    permanent_destruction_immunities: Default::default(),
                    activation: None,
                    charges: None,
                    fuel: initial_item_fuel(&content, &spawn.kind_id),
                    device_recovery_progress: 0,
                    captured_actor: None,
                    location: ItemLocation::Ground(position_from_content(spawn.position)),
                };
                materialization.apply_to(&mut item);
                item
            })
            .collect::<Vec<_>>();
        let body_slots = resolve_body_slots(&content, build.as_ref())?;
        let mut next_item_instance_serial =
            derive_next_item_instance_serial(&player, &entities, &items)?;
        if let Some(quantity) = starting_ration_quantity {
            append_starting_item(
                &content,
                &StartingItemDefinition {
                    item_kind_id: hunger::RATION_ITEM_KIND_ID.to_owned(),
                    quantity,
                    maximum_quantity: None,
                    equipped: false,
                    fully_charged: false,
                },
                &body_slots,
                &mut items,
                &mut next_item_instance_serial,
                &mut rng,
            )?;
        }
        if let Some(supply) = starting_torches {
            for _ in 0..supply.quantity {
                append_starting_item(
                    &content,
                    &StartingItemDefinition {
                        item_kind_id: lighting::WOODEN_TORCH_ITEM_KIND_ID.to_owned(),
                        quantity: 1,
                        maximum_quantity: None,
                        equipped: false,
                        fully_charged: false,
                    },
                    &body_slots,
                    &mut items,
                    &mut next_item_instance_serial,
                    &mut rng,
                )?;
                items
                    .last_mut()
                    .and_then(|item| item.fuel.as_mut())
                    .expect("validated birth torch must have fuel")
                    .current = supply.fuel;
            }
        }
        append_starting_items(
            &content,
            build.as_ref(),
            &body_slots,
            &mut items,
            &mut next_item_instance_serial,
            &mut rng,
        )?;
        let initial_floor_id = world.initial_floor_id.clone();
        let wilderness_position = world
            .wilderness
            .as_ref()
            .map(|wilderness| position_from_content(wilderness.start_position));
        let task_states = initial_task_states(world, seed);
        let dungeon_states = initial_dungeon_states(world, seed);
        let (town_states, shop_states) = town::initial_town_and_shop_states(
            world,
            &content,
            &mut rng,
            &mut next_item_instance_serial,
        )?;
        let home_states = town::initial_home_states(world, &content);
        let chaos_patron_id = chaos_patron::initial_chaos_patron_id(&content, &mut rng);
        let mogaminator = MogaminatorState::for_character(&content, seed);
        let generated_artifact_ids = items
            .iter()
            .filter(|item| {
                content
                    .item(&item.kind_id)
                    .is_some_and(|definition| definition.artifact_generation.is_some())
            })
            .map(|item| item.kind_id.clone())
            .collect();
        let mut game = Self {
            content,
            world_id: world_id.to_owned(),
            map_scale: MapScaleDto::Local,
            wilderness_position,
            wilderness_view_offset: Position::default(),
            wilderness_seed: seed,
            wilderness_terrain_cache: BTreeMap::new(),
            world_travel_destination: None,
            interface_locale: LocaleDto::ZhCn,
            mogaminator,
            current_floor_id: initial_floor_id,
            current_dungeon_instance_id: None,
            reproduction_suppressed: false,
            stored_floors: BTreeMap::new(),
            width,
            height,
            terrain,
            glow: vec![false; usize::from(width) * usize::from(height)],
            player_name,
            player,
            riding_actor_id: None,
            riding_bond: None,
            build,
            body_slots,
            progress,
            virtues,
            resources: BTreeMap::new(),
            last_visual_cells: None,
            bonus_spell_learning_capacity: 0,
            learned_abilities: BTreeSet::new(),
            ability_progress: BTreeMap::new(),
            entities,
            items,
            gold,
            nutrition: rfb_protocol::PLAYER_NUTRITION_BIRTH,
            fasting: false,
            gold_piles: Vec::new(),
            item_knowledge: BTreeMap::new(),
            item_property_knowledge: BTreeMap::new(),
            task_states,
            bounty_state: bounty::BountyState::default(),
            command_actor_deaths: Vec::new(),
            dungeon_states,
            defeated_limited_actor_counts: BTreeMap::new(),
            generated_artifact_ids,
            town_states,
            shop_states,
            home_states,
            campaign_state: CampaignState::default(),
            summon_command: SummonCommandDto::default(),
            recall: None,
            confusing_strike_ready: false,
            sniper_concentration: 0,
            probed_actor_kind_ids: BTreeSet::new(),
            minor_slow: 0,
            minor_slow_energy: 0,
            chaos_patron_id,
            reality_change_ticks: 0,
            pending_mutation_direction: None,
            pending_ability_direction: None,
            next_item_instance_serial,
            next_gold_pile_serial: 1,
            explored: vec![false; usize::from(width) * usize::from(height)],
            revealed_terrain: BTreeSet::new(),
            floor_connections: Vec::new(),
            floor_regions: Vec::new(),
            rng,
            revision: 0,
            turn: 0,
            world_tick: starting_world_tick,
            last_non_melee_fear_aura_tick: None,
            last_command_seq: 0,
            debug_ability_casts_succeed: false,
            debug_recharge_attempts_succeed: false,
            debug_recharge_attempts_fail: false,
            debug_recharge_sources_survive: false,
            debug_recall_delay_turns: None,
            debug_item_curses_land: false,
            debug_item_curses_resisted: false,
            monster_division_remainders: BTreeMap::new(),
        };
        game.initialize_birth_race_mutations();
        game.initialize_player_ability_state();
        game.initialize_starting_item_knowledge();
        let mut initial_entities = std::mem::take(&mut game.entities);
        for actor in &mut initial_entities {
            game.maybe_initialize_chameleon_form(actor);
        }
        game.entities = initial_entities;
        game.player.hp = game.effective_player_max_hp();
        game.initialize_carried_loot()?;
        game.initialize_continuous_wilderness_surface()?;
        game.refresh_daily_bounty_target();
        game.refresh_invisible_visibility(true, &BTreeMap::new());
        game.refresh_weird_mind_visibility(true, &BTreeMap::new());
        game.reveal_current_visibility();
        Ok(game)
    }

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
                    | GameAction::IdentifyAllAtFacility { .. }
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
                Ok(outcome) => events.push(DomainEvent::TaskRewarded {
                    item_kind_id: outcome.item_kind_id,
                    quantity: outcome.quantity,
                }),
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
            GameAction::UseFacilityService {
                facility_id,
                service,
                item_id,
            } => match self.use_town_facility_service(
                &facility_id,
                service,
                item_id.as_deref(),
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
                if let Some((stacks, quantity)) = self.drop_inventory_items(&item_ids) {
                    changed.insert(self.player.position);
                    for item_id in &item_ids {
                        self.force_open_capture_ball(
                            item_id,
                            self.player.position,
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
                if let Some((stacks, dropped_quantity)) =
                    self.drop_inventory_quantity(&item_id, quantity)?
                {
                    changed.insert(self.player.position);
                    self.force_open_capture_ball(
                        &item_id,
                        self.player.position,
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
                    let movement_blocked = if let Some(mount_id) = self.riding_actor_id.as_deref() {
                        self.entities
                            .iter()
                            .position(|entity| entity.id == mount_id)
                            .is_none_or(|mount_index| {
                                !self.actor_can_enter_position(mount_index, target)
                            })
                    } else {
                        self.player_can_enter_local_wilderness(target).map_or_else(
                            || {
                                self.index(target).is_none()
                                    || (!self.is_walkable(target)
                                        && !self.player_can_pass_walls()
                                        && !self.index(target).is_some_and(|index| {
                                            self.content.terrain(&self.terrain[index]).is_some_and(
                                                |terrain| {
                                                    self.player_can_cross_tree_terrain(terrain)
                                                },
                                            )
                                        }))
                            },
                            |can_enter| !can_enter,
                        )
                    };
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

    fn study_player_ability(
        &mut self,
        book_item_id: &str,
        ability_id: &str,
    ) -> Result<(), &'static str> {
        let Some(profile) = self.casting_profile().cloned() else {
            return Err("no-casting-profile");
        };
        if profile.study_mode != CastingStudyMode::Chosen {
            return Err("study-mode-mismatch");
        }
        if let Some(reason) = self.ability_study_unavailable_reason() {
            return Err(reason);
        }
        let Some(ability) = self.content.ability(ability_id) else {
            return Err("unknown-ability");
        };
        let ability = self.effective_casting_ability(&profile, ability);
        if self.learned_abilities.contains(ability_id) {
            return Err("already-learned");
        }
        if self.progress.level < Self::player_ability_parameters(&ability).minimum_level {
            return Err("level-too-low");
        }
        if !self.profile_supports_ability(&profile, ability_id) {
            return Err("ability-not-supported");
        }
        if self.learned_abilities.len() >= usize::from(self.ability_learning_capacity(&profile)) {
            return Err("learning-capacity-full");
        }
        let Some(book_id) = self.study_book_id(book_item_id) else {
            return Err("book-unavailable");
        };
        if !self.active_casting_book_ids().contains(&book_id)
            || !self
                .content
                .ability_book(book_id)
                .is_some_and(|book| book.ability_ids.iter().any(|id| id == ability_id))
        {
            return Err("book-mismatch");
        }
        self.learned_abilities.insert(ability_id.to_owned());
        Ok(())
    }

    fn study_random_player_ability(&mut self, book_item_id: &str) -> Result<String, &'static str> {
        let Some(profile) = self.casting_profile().cloned() else {
            return Err("no-casting-profile");
        };
        if profile.study_mode != CastingStudyMode::DivineRandom {
            return Err("study-mode-mismatch");
        }
        if let Some(reason) = self.ability_study_unavailable_reason() {
            return Err(reason);
        }
        if self.learned_abilities.len() >= usize::from(self.ability_learning_capacity(&profile)) {
            return Err("learning-capacity-full");
        }
        let Some(book_id) = self.study_book_id(book_item_id).map(str::to_owned) else {
            return Err("book-unavailable");
        };
        if !self.active_casting_book_ids().contains(&book_id.as_str()) {
            return Err("book-mismatch");
        }
        let candidates = self
            .content
            .ability_book(&book_id)
            .ok_or("book-mismatch")?
            .ability_ids
            .iter()
            .filter_map(|ability_id| {
                let ability = self.content.ability(ability_id)?;
                let ability = self.effective_casting_ability(&profile, ability);
                (!self.learned_abilities.contains(ability_id)
                    && self.progress.level
                        >= Self::player_ability_parameters(&ability).minimum_level)
                    .then(|| ability_id.clone())
            })
            .collect::<Vec<_>>();
        let mut gift = None;
        for (index, ability_id) in candidates.into_iter().enumerate() {
            if self.rng.bounded((index + 1) as u64) == 0 {
                gift = Some(ability_id);
            }
        }
        let ability_id = gift.ok_or("no-learnable-abilities")?;
        self.learned_abilities.insert(ability_id.clone());
        Ok(ability_id)
    }

    fn ability_study_unavailable_reason(&self) -> Option<&'static str> {
        if self.player_has_status_kind(STATUS_BLINDNESS) {
            Some("blind")
        } else if !self.position_is_lit(self.player.position) {
            Some("no-light")
        } else if self.player_has_status_kind(STATUS_CONFUSION) {
            Some("confused")
        } else {
            None
        }
    }

    fn study_book_id(&self, book_item_id: &str) -> Option<&str> {
        self.items
            .iter()
            .find(|item| {
                item.id == book_item_id
                    && (item.location == ItemLocation::Inventory
                        || item.location == ItemLocation::Ground(self.player.position))
            })
            .and_then(|item| self.content.item(&item.kind_id))
            .and_then(|item| item.ability_book_id.as_deref())
    }

    fn forget_player_ability(&mut self, ability_id: &str) -> Result<(), &'static str> {
        let Some(profile) = self.casting_profile().cloned() else {
            return Err("no-casting-profile");
        };
        if self.content.ability(ability_id).is_none() {
            return Err("unknown-ability");
        }
        if !self.profile_supports_ability(&profile, ability_id) {
            return Err("ability-not-supported");
        }
        if !self.learned_abilities.remove(ability_id) {
            return Err("not-learned");
        }
        Ok(())
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
        if candidates.is_empty() || positions.is_empty() {
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
                            || definition.tags.iter().any(|tag| tag == category)
                            || (category == "magic-item"
                                && (definition.artifact_generation.is_some()
                                    || definition.tags.iter().any(|tag| tag == "artifact")
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

    fn ability_path(
        &self,
        ability: &AbilityDefinition,
        target: &TargetSelection,
    ) -> Option<Vec<Position>> {
        let mode = match target {
            TargetSelection::Direction { .. } => AbilityTargetModeDefinition::Direction,
            TargetSelection::Position { .. } => AbilityTargetModeDefinition::Position,
            TargetSelection::Entity { .. } => AbilityTargetModeDefinition::Entity,
            TargetSelection::Item { .. } => AbilityTargetModeDefinition::Item,
            TargetSelection::Town { .. } => AbilityTargetModeDefinition::Town,
            TargetSelection::SelfTarget => AbilityTargetModeDefinition::SelfTarget,
        };
        if !ability.target.modes.contains(&mode) {
            return None;
        }
        self.projectile_path(target, ability.target.range)
    }

    fn beam_ability_path(
        &self,
        ability: &AbilityDefinition,
        target: &TargetSelection,
    ) -> Option<Vec<Position>> {
        let mode = match target {
            TargetSelection::Direction { .. } => AbilityTargetModeDefinition::Direction,
            TargetSelection::Position { .. } => AbilityTargetModeDefinition::Position,
            TargetSelection::Entity { .. } => AbilityTargetModeDefinition::Entity,
            TargetSelection::Item { .. } => AbilityTargetModeDefinition::Item,
            TargetSelection::Town { .. } => AbilityTargetModeDefinition::Town,
            TargetSelection::SelfTarget => AbilityTargetModeDefinition::SelfTarget,
        };
        if !ability.target.modes.contains(&mode) {
            return None;
        }
        match target {
            TargetSelection::Direction { .. } => self.projectile_path(target, ability.target.range),
            TargetSelection::Position { position } => {
                self.targeted_projectile_path_through_target(*position, ability.target.range)
            }
            TargetSelection::Entity { entity_id } => {
                let position = self
                    .entities
                    .iter()
                    .find(|entity| {
                        entity.id == *entity_id && self.entity_is_visible_to_player(entity)
                    })
                    .map(|entity| entity.position)?;
                self.targeted_projectile_path_through_target(position, ability.target.range)
            }
            TargetSelection::SelfTarget => None,
            TargetSelection::Item { .. } => None,
            TargetSelection::Town { .. } => None,
        }
    }

    fn projectile_path(&self, target: &TargetSelection, range: u16) -> Option<Vec<Position>> {
        let origin = self.player.position;
        match target {
            TargetSelection::Direction { direction } => {
                let (dx, dy) = direction.delta();
                Some(
                    (1..=range)
                        .map(|step| Position {
                            x: origin.x + dx * i32::from(step),
                            y: origin.y + dy * i32::from(step),
                        })
                        .collect(),
                )
            }
            TargetSelection::Position { position } => {
                self.targeted_projectile_path(*position, range)
            }
            TargetSelection::Entity { entity_id } => {
                let position = self
                    .entities
                    .iter()
                    .find(|entity| {
                        entity.id == *entity_id && self.entity_is_visible_to_player(entity)
                    })
                    .map(|entity| entity.position)?;
                self.targeted_projectile_path(position, range)
            }
            TargetSelection::SelfTarget => None,
            TargetSelection::Item { .. } => None,
            TargetSelection::Town { .. } => None,
        }
    }

    fn targeted_projectile_path(&self, target: Position, range: u16) -> Option<Vec<Position>> {
        self.projectile_path_to_position(target, range, false, true)
    }

    fn targeted_projectile_path_through_target(
        &self,
        target: Position,
        range: u16,
    ) -> Option<Vec<Position>> {
        self.projectile_path_to_position(target, range, true, true)
    }

    fn untargeted_projectile_path(&self, target: Position, range: u16) -> Option<Vec<Position>> {
        self.projectile_path_to_position(target, range, false, false)
    }

    fn projectile_path_to_position(
        &self,
        target: Position,
        range: u16,
        continue_through_target: bool,
        require_visible: bool,
    ) -> Option<Vec<Position>> {
        let origin = self.player.position;
        if target == origin
            || self.index(target).is_none()
            || (require_visible && !self.is_visible(target))
            || origin.x.abs_diff(target.x).max(origin.y.abs_diff(target.y)) > u32::from(range)
        {
            return None;
        }

        let mut x = origin.x;
        let mut y = origin.y;
        let dx = (target.x - x).abs();
        let sx = if x < target.x { 1 } else { -1 };
        let dy = -(target.y - y).abs();
        let sy = if y < target.y { 1 } else { -1 };
        let mut error = dx + dy;
        let mut path = Vec::new();
        let max_steps = usize::from(range);
        while path.len() < max_steps {
            if !continue_through_target && x == target.x && y == target.y {
                break;
            }
            let doubled = error.saturating_mul(2);
            if doubled >= dy {
                error += dy;
                x += sx;
            }
            if doubled <= dx {
                error += dx;
                y += sy;
            }
            path.push(Position { x, y });
            if !continue_through_target && (x == target.x && y == target.y) {
                break;
            }
            if path.len() >= max_steps {
                break;
            }
        }
        Some(path)
    }

    fn trace_projectile_path(&self, path: Vec<Position>) -> (ProjectileTrace, Option<usize>) {
        self.trace_projectile_path_with_actor_policy(path, true)
    }

    fn trace_projectile_path_with_actor_policy(
        &self,
        path: Vec<Position>,
        stop_at_actor: bool,
    ) -> (ProjectileTrace, Option<usize>) {
        self.trace_projectile_path_with_damage_policy(path, stop_at_actor, None)
    }

    fn trace_projectile_path_with_damage_policy(
        &self,
        path: Vec<Position>,
        stop_at_actor: bool,
        damage_type: Option<DamageType>,
    ) -> (ProjectileTrace, Option<usize>) {
        let origin = self.player.position;
        let mut impact = origin;
        let mut landing = origin;
        let mut traversed = Vec::new();
        let mut target_index = None;
        for position in path {
            impact = position;
            let traversable = self.index(position).is_some_and(|index| {
                if damage_type == Some(DamageType::Disintegrate) {
                    !self
                        .content
                        .terrain(&self.terrain[index])
                        .is_some_and(|terrain| terrain.tags.iter().any(|tag| tag == "permanent"))
                } else {
                    self.is_walkable(position)
                }
            });
            if !traversable {
                break;
            }
            landing = position;
            traversed.push(position);
            if let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.position == position)
            {
                target_index = Some(index);
                if stop_at_actor {
                    break;
                }
            }
        }
        (
            ProjectileTrace {
                origin,
                impact,
                landing,
                traversed,
            },
            target_index,
        )
    }

    fn area_damage_targets(
        &self,
        center: Position,
        radius: u8,
        target_category: Option<&str>,
    ) -> (Vec<Position>, Vec<(String, u32)>) {
        self.area_damage_targets_for_type(center, radius, target_category, DamageType::Physical)
    }

    fn area_damage_targets_for_type(
        &self,
        center: Position,
        radius: u8,
        target_category: Option<&str>,
        damage_type: DamageType,
    ) -> (Vec<Position>, Vec<(String, u32)>) {
        let cells = self.area_damage_cells_for_type(center, radius, damage_type);
        let affected_positions = cells.iter().map(|(_, position)| *position).collect();
        let targets = cells
            .iter()
            .flat_map(|(distance, position)| {
                self.entities
                    .iter()
                    .filter(move |entity| {
                        entity.hp > 0
                            && entity.position == *position
                            && target_category.is_none_or(|category| {
                                self.content
                                    .actor(&entity.kind_id)
                                    .is_some_and(|definition| {
                                        actor_matches_category(definition, category)
                                    })
                            })
                    })
                    .map(move |entity| (entity.id.clone(), *distance))
            })
            .collect();
        (affected_positions, targets)
    }

    fn area_damage_cells(&self, center: Position, radius: u8) -> Vec<(u32, Position)> {
        self.area_damage_cells_for_type(center, radius, DamageType::Physical)
    }

    fn area_damage_cells_for_type(
        &self,
        center: Position,
        radius: u8,
        damage_type: DamageType,
    ) -> Vec<(u32, Position)> {
        let mut cells = Vec::new();
        let radius_limit = u32::from(radius);
        let radius = i32::from(radius);
        for y in center.y - radius..=center.y + radius {
            for x in center.x - radius..=center.x + radius {
                let position = Position { x, y };
                let distance = rfb_distance(center, position);
                if distance > radius_limit
                    || !self.index(position).is_some_and(|_| {
                        if damage_type == DamageType::Disintegrate {
                            has_disintegration_line_of_effect(self, center, position)
                        } else {
                            has_line_of_effect(self, center, position)
                        }
                    })
                {
                    continue;
                }
                cells.push((distance, position));
            }
        }
        cells.sort_by_key(|(distance, position)| (*distance, position.y, position.x));
        cells
    }

    fn beam_damage_targets(&self, path: &[Position]) -> Vec<String> {
        path.iter()
            .flat_map(|position| {
                self.entities
                    .iter()
                    .filter(move |entity| entity.hp > 0 && entity.position == *position)
                    .map(|entity| entity.id.clone())
            })
            .collect()
    }

    fn cone_damage_targets(
        &self,
        centerline: &[Position],
        direction: Direction,
        radius: u8,
        damage_type: DamageType,
    ) -> (Vec<Position>, Vec<(String, u32)>) {
        let cells = self.cone_damage_cells(
            self.player.position,
            centerline,
            direction,
            radius,
            damage_type,
        );
        let affected_positions = cells.iter().map(|(_, _, position)| *position).collect();
        let targets = cells
            .iter()
            .flat_map(|(_, lateral_distance, position)| {
                self.entities
                    .iter()
                    .filter(move |entity| entity.hp > 0 && entity.position == *position)
                    .map(move |entity| (entity.id.clone(), *lateral_distance))
            })
            .collect();
        (affected_positions, targets)
    }

    fn cone_damage_cells(
        &self,
        origin: Position,
        centerline: &[Position],
        direction: Direction,
        radius: u8,
        damage_type: DamageType,
    ) -> Vec<(i32, u32, Position)> {
        let depth = i32::try_from(centerline.len()).unwrap_or(i32::MAX);
        if depth == 0 {
            return Vec::new();
        }
        let (dx, dy) = direction.delta();
        let width_denominator = (depth - 1).max(1);
        let mut cells = Vec::new();
        for (index, center) in centerline.iter().enumerate() {
            let layer = i32::try_from(index + 1).unwrap_or(i32::MAX);
            debug_assert_eq!(
                *center,
                Position {
                    x: origin.x + dx * layer,
                    y: origin.y + dy * layer,
                }
            );
            let width = if depth == 1 {
                0
            } else {
                i32::from(radius).saturating_mul(layer - 1) / width_denominator
            };
            for y in center.y - width..=center.y + width {
                for x in center.x - width..=center.x + width {
                    let position = Position { x, y };
                    let position_layer = origin
                        .x
                        .abs_diff(position.x)
                        .max(origin.y.abs_diff(position.y));
                    let offset_x = position.x - origin.x;
                    let offset_y = position.y - origin.y;
                    let forward = offset_x * dx + offset_y * dy;
                    let lateral = (offset_x * dy - offset_y * dx).abs();
                    if position_layer != u32::try_from(layer).unwrap_or(u32::MAX)
                        || forward <= 0
                        || lateral > forward
                        || self.index(position).is_none()
                        || if damage_type == DamageType::Disintegrate {
                            !has_disintegration_line_of_effect(self, origin, position)
                        } else {
                            !has_line_of_effect(self, origin, position)
                        }
                    {
                        continue;
                    }
                    let lateral_distance = center
                        .x
                        .abs_diff(position.x)
                        .max(center.y.abs_diff(position.y));
                    cells.push((layer, lateral_distance, position));
                }
            }
        }
        cells.sort_by_key(|(layer, lateral_distance, position)| {
            (*layer, *lateral_distance, position.y, position.x)
        });
        cells
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
        let broken = hit_body && self.rng.bounded(100) < u64::from(break_chance_percent);
        if broken {
            self.item_property_knowledge.remove(&ammunition.id);
            events.push(DomainEvent::ProjectileAmmoBroken {
                ammo_kind_id: ammunition.kind_id,
            });
            return;
        }
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
            && item_device_generation(&self.content, &item.kind_id, &item.affix_ids)
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
            let profile = item_device_generation(&self.content, &item.kind_id, &item.affix_ids)?
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
            if self.index(target).is_none()
                || (!self.is_walkable(target) && !self.player_can_pass_walls())
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

    fn initialize_starting_item_knowledge(&mut self) {
        for item in self.items.iter().filter(|item| {
            matches!(
                item.location,
                ItemLocation::Inventory | ItemLocation::Equipped { .. }
            )
        }) {
            if self
                .content
                .item(&item.kind_id)
                .is_some_and(|definition| definition.appearance_name_key.is_some())
            {
                self.item_knowledge.insert(
                    item.kind_id.clone(),
                    ItemKnowledgeState {
                        tried: true,
                        aware: true,
                    },
                );
            }
            if matches!(item.location, ItemLocation::Equipped { .. }) {
                self.item_property_knowledge.insert(
                    item.id.clone(),
                    ItemPropertyKnowledgeState {
                        discovered: true,
                        appraised: true,
                        identified: true,
                        known_affix_ids: item.affix_ids.iter().cloned().collect(),
                    },
                );
            }
        }
    }

    fn initialize_carried_loot(&mut self) -> Result<(), CoreError> {
        let floor_id = self.current_floor_id.clone();
        let depth = self.floor_depth(&floor_id);
        let actors = self.entities.clone();
        let generated = self.generate_carried_loot_for_actors(&actors, &floor_id, depth)?;
        self.items.extend(generated);
        Ok(())
    }

    fn generate_carried_loot_for_actors(
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

    fn generate_death_loot(
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
                        depth,
                        source: LootSource::MonsterDeath {
                            actor_id: actor.id.clone(),
                        },
                    },
                    ItemLocation::Ground(actor.position),
                    drop.minimum_quality,
                )?);
            }
        } else if let Some(table_id) = table_id {
            let context = LootContext {
                table_id,
                floor_id: floor_id.clone(),
                depth,
                source: LootSource::MonsterDeath {
                    actor_id: actor.id.clone(),
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
            let context = reward_table_id.map(|table_id| LootContext {
                table_id,
                floor_id: floor_id.clone(),
                depth,
                source: LootSource::MonsterDeath {
                    actor_id: actor.id.clone(),
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
                    self.commit_generated_item_draft(draft, ItemLocation::Ground(actor.position))?,
                );
            } else if let Some(kind_id) = reward_artifact_kind_id
                && !self.generated_artifact_ids.contains(&kind_id)
            {
                let context = context
                    .as_ref()
                    .expect("validated artifact guardian reward must retain a fallback table");
                let draft = self.fixed_item_draft(context, kind_id);
                generated.push(
                    self.commit_generated_item_draft(draft, ItemLocation::Ground(actor.position))?,
                );
            } else if let Some(context) = context {
                let mode = if artifact_reward {
                    ItemGenerationMode::Artifact
                } else {
                    ItemGenerationMode::Ordinary
                };
                generated.extend(self.generate_loot_instances_internal(
                    &context,
                    ItemLocation::Ground(actor.position),
                    true,
                    None,
                    mode,
                )?);
            }
        }
        Ok((generated, gold))
    }

    fn generate_loot_instances(
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
        minimum_quality: rfb_content::ItemQuality,
    ) -> Result<Vec<ItemInstance>, CoreError> {
        self.next_item_instance_serial
            .checked_add(1)
            .ok_or(CoreError::ItemIdExhausted)?;
        let Some(draft) = self.generate_one_loot_draft(context, minimum_quality.into()) else {
            return Ok(Vec::new());
        };
        Ok(vec![self.commit_generated_item_draft(draft, location)?])
    }

    fn generate_loot_instances_internal(
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

    fn generate_one_loot_draft(
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
                LootSource::MonsterCarried { actor_id } | LootSource::MonsterDeath { actor_id } => {
                    !actor_id.is_empty()
                }
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
        let eligible_entries = table
            .entries
            .iter()
            .filter(|entry| {
                entry.min_depth <= context.depth
                    && context.depth <= entry.max_depth
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
            if mode == ItemGenerationMode::Artifact
                && let Some(kind_id) = self.roll_instant_fixed_artifact_kind_id(context)
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
            if mode == ItemGenerationMode::Artifact {
                let artifact_kind_id = (0..4).find_map(|_| {
                    self.roll_fixed_artifact_kind_id(context, Some(&entry.item_kind_id), false)
                });
                if let Some(kind_id) = artifact_kind_id {
                    generated.push(self.fixed_item_draft(context, kind_id));
                    continue;
                }
            }
            let harp_intrinsic_properties =
                self.content.item(&entry.item_kind_id).and_then(|item| {
                    materialize_rfb_harp_intrinsic_with_rng(&mut self.rng, item, generation_depth)
                });
            let rolled_quality = match table.quality_policy {
                Some(policy) => self.roll_rfb_depth_loot_quality(
                    policy,
                    generation_depth,
                    jewelry,
                    minimum_quality,
                ),
                None => self.roll_loot_quality(
                    &table.quality_weights,
                    &quality_weights,
                    minimum_quality,
                ),
            };
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
                item.max_stack == 1 && item.equipment_slot.is_some() && entry.quantity == 1
            });
            let quality = if supports_quality {
                rolled_quality
            } else {
                ItemQualityDto::Ordinary
            };
            let rfb_materialization = (table.rfb_ego_policy
                == Some(rfb_content::LootRfbEgoPolicyDefinition::WeaponDigger)
                && quality_allows_natural_affix(table.quality_policy, quality))
            .then(|| {
                self.content.item(&entry.item_kind_id).and_then(|item| {
                    roll_and_materialize_rfb_ego_from_affixes_with_rng(
                        &mut self.rng,
                        item,
                        self.content.affix_definitions(),
                        generation_depth,
                        harp_intrinsic_properties.as_ref(),
                    )
                })
            })
            .flatten();
            let materialization = rfb_materialization.unwrap_or_else(|| {
                let rolled_affix_id = preselected_generic_affix_id.unwrap_or_else(|| {
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
                let affix_is_required = table
                    .affix_weights
                    .iter()
                    .all(|entry| entry.affix_id.is_some());
                let affix_ids = if affix_is_required
                    || quality_allows_natural_affix(table.quality_policy, quality)
                {
                    rolled_affix_id.flatten().iter().cloned().collect()
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
                )
            });
            let EgoMaterialization {
                affix_ids,
                rolled_affixes,
                intrinsic_properties: ego_intrinsic_properties,
                enchantment_delta,
                curse,
                activation,
                charges,
                ..
            } = materialization;
            let mut intrinsic_properties = harp_intrinsic_properties.unwrap_or_default();
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
            generated.push(GeneratedItemDraft {
                kind_id: entry.item_kind_id.clone(),
                quantity: entry.quantity,
                origin_kind: match &context.source {
                    LootSource::Rubble { .. } => Some(ItemOriginKindDto::Rubble),
                    _ => None,
                },
                quality,
                affix_ids,
                rolled_affixes,
                intrinsic_properties,
                enchantments: enchantment_delta,
                curse,
                activation,
                charges,
                fuel: initial_item_fuel(&self.content, &entry.item_kind_id),
            });
        }
        generated
    }

    fn roll_instant_fixed_artifact_kind_id(&mut self, context: &LootContext) -> Option<String> {
        if self.rng.bounded(10) != 0 {
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

    fn roll_fixed_artifact_kind_id(
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

    fn fixed_item_draft(&mut self, context: &LootContext, kind_id: String) -> GeneratedItemDraft {
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
        );
        GeneratedItemDraft {
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

    fn commit_generated_item_draft(
        &mut self,
        draft: GeneratedItemDraft,
        location: ItemLocation,
    ) -> Result<ItemInstance, CoreError> {
        let id = self.allocate_item_instance_id()?;
        self.register_generated_artifact(&draft.kind_id);
        Ok(draft.into_item_instance(id, location))
    }

    fn register_generated_artifact(&mut self, kind_id: &str) {
        if self
            .content
            .item(kind_id)
            .is_some_and(|item| item.artifact_generation.is_some())
        {
            self.generated_artifact_ids.insert(kind_id.to_owned());
        }
    }

    fn roll_loot_quality(
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

    fn roll_rfb_depth_loot_quality(
        &mut self,
        policy: rfb_content::LootQualityPolicyDefinition,
        depth: u16,
        jewelry: bool,
        minimum: rfb_content::ItemQuality,
    ) -> ItemQualityDto {
        let (good_percent, great_percent) =
            rfb_depth_quality_percentages(policy, depth, jewelry, self.player_luck_bias());
        let roll = self.rng.bounded(10_000);
        let quality = match minimum {
            rfb_content::ItemQuality::Exceptional => rfb_content::ItemQuality::Exceptional,
            rfb_content::ItemQuality::Fine => {
                if roll < great_percent * 100 {
                    rfb_content::ItemQuality::Exceptional
                } else {
                    rfb_content::ItemQuality::Fine
                }
            }
            rfb_content::ItemQuality::Ordinary => {
                let exceptional_threshold = good_percent * great_percent;
                if roll < exceptional_threshold {
                    rfb_content::ItemQuality::Exceptional
                } else if roll < good_percent * 100 {
                    rfb_content::ItemQuality::Fine
                } else {
                    rfb_content::ItemQuality::Ordinary
                }
            }
        };
        item_quality_dto(quality)
    }

    fn luck_adjusted_item_generation_depth(&mut self, depth: u16, staff: bool) -> u16 {
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

    fn reveal_current_visibility(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                let position = Position {
                    x: i32::from(x),
                    y: i32::from(y),
                };
                if self.is_visible(position) {
                    let index = self.index(position).expect("visibility position is valid");
                    self.explored[index] = true;
                }
            }
        }
        let discovered_item_ids = self
            .items
            .iter()
            .filter_map(|item| match item.location {
                ItemLocation::Inventory | ItemLocation::Equipped { .. } => Some(item.id.clone()),
                ItemLocation::Ground(position) if self.is_visible(position) => {
                    Some(item.id.clone())
                }
                ItemLocation::Ground(_)
                | ItemLocation::CarriedBy { .. }
                | ItemLocation::Shop { .. }
                | ItemLocation::Home { .. } => None,
            })
            .collect::<Vec<_>>();
        self.mark_item_instances_discovered(&discovered_item_ids);
        let discovered_gold_ids = self
            .gold_piles
            .iter()
            .filter(|pile| self.is_visible(pile.position))
            .map(|pile| pile.id.clone())
            .collect::<Vec<_>>();
        self.mark_gold_piles_discovered(&discovered_gold_ids);
    }

    pub(super) fn clear_current_floor_memory(&mut self, changed: &mut BTreeSet<Position>) -> u32 {
        let width = usize::from(self.width);
        let mut cleared_cells = 0_u32;
        for (index, explored) in self.explored.iter_mut().enumerate() {
            if *explored {
                *explored = false;
                cleared_cells += 1;
                changed.insert(Position {
                    x: i32::try_from(index % width).expect("explored x must fit i32"),
                    y: i32::try_from(index / width).expect("explored y must fit i32"),
                });
            }
        }
        cleared_cells += u32::try_from(self.revealed_terrain.len()).unwrap_or(0);
        self.revealed_terrain.clear();
        cleared_cells
    }

    fn mark_item_instances_discovered(&mut self, item_ids: &[String]) {
        for item_id in item_ids {
            self.item_property_knowledge
                .entry(item_id.clone())
                .or_default()
                .discovered = true;
        }
    }

    fn item_is_discovered(&self, item_id: &str) -> bool {
        self.item_property_knowledge
            .get(item_id)
            .is_some_and(|knowledge| knowledge.discovered)
    }

    fn is_visible(&self, position: Position) -> bool {
        // Blindness suppresses the whole player FOV except the occupied cell:
        // visuals fall back to remembered knowledge, visibility-gated targeting
        // rejects, and rest no longer interrupts on enemies the player cannot
        // see. Monster senses do not route through this helper.
        if self.player_has_status_kind(STATUS_BLINDNESS) {
            return position == self.player.position;
        }
        if squared_distance(self.player.position, position) > VISIBILITY_RADIUS * VISIBILITY_RADIUS
        {
            return false;
        }
        has_line_of_sight(self, self.player.position, position)
            && (self.floor_has_environment_light()
                || position == self.player.position
                || self.position_is_lit(position))
    }

    fn actor_is_invisible(&self, entity: &Actor) -> bool {
        self.actor_runtime_definition(entity)
            .is_some_and(|definition| definition.tags.iter().any(|tag| tag == "invisible"))
    }

    fn entity_is_visible_to_player(&self, entity: &Actor) -> bool {
        self.entity_is_visually_visible_to_player(entity)
            || self.entity_is_visible_by_telepathy(entity)
    }

    fn entity_is_visually_visible_to_player(&self, entity: &Actor) -> bool {
        (self.is_visible(entity.position) || self.entity_is_visible_by_infravision(entity))
            && (!self.actor_is_invisible(entity) || entity.visible_invisible)
    }

    fn entity_is_visible_by_telepathy(&self, entity: &Actor) -> bool {
        let Some(definition) = self.actor_runtime_definition(entity) else {
            return false;
        };
        let full_telepathy = self.player_has_telepathy();
        let targeted_esp = self.player_has_targeted_esp(definition);
        if (!full_telepathy && !targeted_esp)
            || squared_distance(self.player.position, entity.position)
                > VISIBILITY_RADIUS * VISIBILITY_RADIUS
        {
            return false;
        }
        if targeted_esp {
            return true;
        }
        if self.player_has_mutation(HUMAN_WIS_MUTATION_ID)
            && !self.actor_is_player_side(entity)
            && definition.tags.iter().any(|tag| tag == "evil")
        {
            return false;
        }
        if definition.tags.iter().any(|tag| tag == "empty-mind") {
            false
        } else if definition.tags.iter().any(|tag| tag == "weird-mind") {
            entity.visible_weird_mind
        } else {
            true
        }
    }

    fn entity_is_fuzzy_to_player(&self, entity: &Actor) -> bool {
        !self.actor_is_player_aligned(entity)
            && !self.entity_is_visually_visible_to_player(entity)
            && self.entity_is_visible_by_telepathy(entity)
    }

    fn entity_is_visible_by_infravision(&self, entity: &Actor) -> bool {
        if self.player_has_status_kind(STATUS_BLINDNESS) {
            return false;
        }
        let range = self.player_infravision_range();
        range > 0
            && squared_distance(self.player.position, entity.position) <= range * range
            && has_line_of_sight(self, self.player.position, entity.position)
            && self
                .actor_runtime_definition(entity)
                .is_some_and(|definition| {
                    !definition.tags.iter().any(|tag| tag == "cold-blooded")
                        || definition
                            .contact_auras
                            .iter()
                            .any(|aura| aura.damage_type == ActorDamageType::Fire)
                })
    }

    fn refresh_invisible_visibility(
        &mut self,
        full: bool,
        previous_positions: &BTreeMap<String, Position>,
    ) {
        let sources = self.player_see_invisible_sources();
        let search_skill = self.player_derived_stats().search_skill.value.max(0) as u64;
        let candidates = self
            .entities
            .iter()
            .enumerate()
            .filter_map(|(index, entity)| {
                let definition = self.actor_runtime_definition(entity)?;
                definition
                    .tags
                    .iter()
                    .any(|tag| tag == "invisible")
                    .then_some((
                        index,
                        entity.id.clone(),
                        entity.position,
                        definition.level,
                        entity.visible_invisible,
                    ))
            })
            .collect::<Vec<_>>();
        for (index, id, position, level, was_visible) in candidates {
            if !self.is_visible(position) || sources == 0 {
                self.entities[index].visible_invisible = false;
                continue;
            }
            let moved = previous_positions
                .get(&id)
                .is_none_or(|before| *before != position);
            if !full && !moved {
                self.entities[index].visible_invisible = was_visible;
                continue;
            }
            let difficulty = u64::from(50_u32.saturating_add(level / 2));
            self.entities[index].visible_invisible =
                (0..sources).any(|_| self.rng.bounded(difficulty) < search_skill);
        }
    }

    fn refresh_weird_mind_visibility(
        &mut self,
        full: bool,
        previous_positions: &BTreeMap<String, Position>,
    ) {
        let has_telepathy = self.player_has_telepathy();
        let candidates = self
            .entities
            .iter()
            .enumerate()
            .filter_map(|(index, entity)| {
                self.actor_runtime_definition(entity)?
                    .tags
                    .iter()
                    .any(|tag| tag == "weird-mind")
                    .then_some((
                        index,
                        entity.id.clone(),
                        entity.position,
                        entity.visible_weird_mind,
                    ))
            })
            .collect::<Vec<_>>();
        for (index, id, position, was_visible) in candidates {
            if !has_telepathy
                || squared_distance(self.player.position, position)
                    > VISIBILITY_RADIUS * VISIBILITY_RADIUS
            {
                self.entities[index].visible_weird_mind = false;
                continue;
            }
            let moved = previous_positions
                .get(&id)
                .is_none_or(|before| *before != position);
            if !full && !moved {
                self.entities[index].visible_weird_mind = was_visible;
                continue;
            }
            self.entities[index].visible_weird_mind = self.rng.bounded(10) == 0;
        }
    }

    fn floor_has_environment_light(&self) -> bool {
        self.is_wilderness_floor()
            || self.current_town().is_some()
            || self
                .content
                .world(&self.world_id)
                .is_some_and(|world| self.current_floor_id == world.initial_floor_id)
    }

    fn ambient_light(&self, position: Position, sources: &[LightSource]) -> u8 {
        if self.floor_has_environment_light() && self.wilderness_is_daytime() {
            SURFACE_AMBIENT_LIGHT
        } else if self.index(position).is_some_and(|index| self.glow[index])
            && !sources
                .iter()
                .any(|source| source.darkness && source.contains(position))
        {
            ROOM_GLOW_LIGHT
        } else {
            DUNGEON_AMBIENT_LIGHT
        }
    }

    fn position_is_lit(&self, position: Position) -> bool {
        let sources = self.collect_light_sources();
        sources
            .iter()
            .any(|source| !source.darkness && source.contains(position))
            || self.ambient_light(position, &sources) > 0
    }

    fn connected_glow_positions(&self, origin: Position) -> Vec<Position> {
        let Some(origin_index) = self.index(origin) else {
            return Vec::new();
        };
        if !self.glow[origin_index] {
            return Vec::new();
        }

        let mut visited = BTreeSet::from([origin]);
        let mut queue = VecDeque::from([origin]);
        let mut positions = Vec::new();
        while let Some(position) = queue.pop_front() {
            positions.push(position);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let neighbor = Position {
                        x: position.x + dx,
                        y: position.y + dy,
                    };
                    let Some(index) = self.index(neighbor) else {
                        continue;
                    };
                    if self.glow[index] && visited.insert(neighbor) {
                        queue.push_back(neighbor);
                    }
                }
            }
        }
        positions
    }

    fn darken_room(&mut self, origin: Position) -> Vec<Position> {
        let darkened = self.connected_glow_positions(origin);
        for position in &darkened {
            let index = self
                .index(*position)
                .expect("connected glow position must remain in bounds");
            self.glow[index] = false;
        }
        darkened
    }

    fn collect_light_sources(&self) -> Vec<LightSource> {
        // The source order mirrors the original per-cell scan (player, then
        // entities, then ground items) so strict-greater comparisons keep
        // resolving ties identically.
        let mut sources = Vec::new();
        if let Some(radius) = self.player_light_radius() {
            sources.push(LightSource {
                position: self.player.position,
                radius,
                maximum: 72,
                color: PLAYER_LIGHT_COLOR,
                darkness: false,
            });
        }
        for entity in &self.entities {
            let Some(definition) = self.actor_runtime_definition(entity) else {
                continue;
            };
            let Some(light) = definition.light else {
                continue;
            };
            if !light.intrinsic
                && entity
                    .statuses
                    .iter()
                    .any(|status| status.kind_id == STATUS_SLEEP)
            {
                continue;
            }
            sources.push(LightSource {
                position: entity.position,
                radius: i32::from(light.radius),
                maximum: 64,
                color: ACTOR_LIGHT_COLOR,
                darkness: light.darkness,
            });
        }
        for item in &self.items {
            let ItemLocation::Ground(item_position) = &item.location else {
                continue;
            };
            let Some(definition) = self.content.item(&item.kind_id) else {
                continue;
            };
            if definition.fuel.is_some() || !definition.tags.iter().any(|tag| tag == "light-source")
            {
                continue;
            }
            sources.push(LightSource {
                position: *item_position,
                radius: ITEM_LIGHT_RADIUS,
                maximum: 52,
                color: ITEM_LIGHT_COLOR,
                darkness: false,
            });
        }
        sources
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

fn rfb_depth_quality_percentages(
    policy: rfb_content::LootQualityPolicyDefinition,
    depth: u16,
    jewelry: bool,
    luck: LuckBias,
) -> (u64, u64) {
    let rfb_content::LootQualityPolicyDefinition::RfbDepth {
        good_cap_percent,
        great_cap_percent,
    } = policy;
    let mut good_cap = u64::from(good_cap_percent);
    let mut great_cap = u64::from(great_cap_percent);
    if luck == LuckBias::Bad {
        good_cap = good_cap.saturating_sub(5);
        great_cap = great_cap.saturating_sub(great_cap / 4);
    }
    let mut good = (u64::from(depth) + 10).min(good_cap);
    let mut great = (good * 2 / 3).min(great_cap);
    if jewelry {
        good = good.saturating_add(30).min(good_cap);
    }
    if luck == LuckBias::Good {
        good = good.saturating_add(5).min(100);
        great = great.saturating_add(2).min(100);
    }
    (good, great)
}

fn quality_allows_natural_affix(
    policy: Option<rfb_content::LootQualityPolicyDefinition>,
    quality: ItemQualityDto,
) -> bool {
    match policy {
        Some(_) => quality == ItemQualityDto::Exceptional,
        None => quality != ItemQualityDto::Ordinary,
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
    if resistance == ResistanceLevel::Immune {
        return 0;
    }
    let multiplier = 100_i64.saturating_sub(i64::from(resistance.reduction_percent()));
    u32::try_from(
        i64::from(requested)
            .saturating_mul(multiplier)
            .saturating_div(100)
            .clamp(1, i64::from(u32::MAX)),
    )
    .expect("clamped status duration must fit u32")
}

fn scaled_ability_level_value(
    base: u64,
    scaling: &AbilityLevelScalingDefinition,
    level: u16,
) -> u64 {
    let addition = match scaling.curve {
        AbilityLevelScalingCurveDefinition::Linear => {
            u64::from(level.saturating_sub(scaling.level_offset))
                .saturating_mul(u64::from(scaling.multiplier))
                / u64::from(scaling.divisor)
        }
        AbilityLevelScalingCurveDefinition::Prorated => prorated_level_value(
            u64::from(scaling.multiplier),
            level,
            scaling.linear_weight,
            scaling.quadratic_weight,
            scaling.cubic_weight,
        ),
    };
    let scaled = base.saturating_add(addition);
    scaling
        .maximum
        .map_or(scaled, |maximum| scaled.min(maximum))
}

fn prorated_level_value(
    amount: u64,
    level: u16,
    linear_weight: u16,
    quadratic_weight: u16,
    cubic_weight: u16,
) -> u64 {
    let level = u64::from(level.min(50));
    if level == 50 {
        return amount;
    }
    let linear_weight = u64::from(linear_weight);
    let quadratic_weight = u64::from(quadratic_weight);
    let cubic_weight = u64::from(cubic_weight);
    let total_weight = linear_weight + quadratic_weight + cubic_weight;
    amount * level * linear_weight / (50 * total_weight)
        + amount * level * level * quadratic_weight / (50 * 50 * total_weight)
        + (amount * level * level / 50) * level * cubic_weight / (50 * 50 * total_weight)
}

fn apply_ability_level_scaling(
    effect: &mut AbilityEffectDefinition,
    scaling: &AbilityLevelScalingDefinition,
    level: u16,
) {
    match (effect, scaling.field) {
        (
            AbilityEffectDefinition::Damage { damage_dice, .. }
            | AbilityEffectDefinition::Malediction { damage_dice, .. }
            | AbilityEffectDefinition::AreaDamage { damage_dice, .. }
            | AbilityEffectDefinition::LavaFlow { damage_dice, .. }
            | AbilityEffectDefinition::BeamDamage { damage_dice, .. }
            | AbilityEffectDefinition::BoltOrBeamDamage { damage_dice, .. }
            | AbilityEffectDefinition::Stardust { damage_dice, .. }
            | AbilityEffectDefinition::BoltOrAreaDamage { damage_dice, .. }
            | AbilityEffectDefinition::ConeDamage { damage_dice, .. }
            | AbilityEffectDefinition::CurseDamage { damage_dice, .. }
            | AbilityEffectDefinition::VisibleDamage { damage_dice, .. }
            | AbilityEffectDefinition::DrainLife { damage_dice, .. },
            AbilityLevelScalingField::DamageDice,
        ) => {
            *damage_dice = u16::try_from(scaled_ability_level_value(
                u64::from(*damage_dice),
                scaling,
                level,
            ))
            .expect("validated level-scaled damage dice must fit u16");
        }
        (
            AbilityEffectDefinition::Damage { damage_sides, .. }
            | AbilityEffectDefinition::Malediction { damage_sides, .. }
            | AbilityEffectDefinition::AreaDamage { damage_sides, .. }
            | AbilityEffectDefinition::LavaFlow { damage_sides, .. }
            | AbilityEffectDefinition::BeamDamage { damage_sides, .. }
            | AbilityEffectDefinition::LightArea { damage_sides, .. }
            | AbilityEffectDefinition::BoltOrBeamDamage { damage_sides, .. }
            | AbilityEffectDefinition::BoltOrAreaDamage { damage_sides, .. }
            | AbilityEffectDefinition::ConeDamage { damage_sides, .. }
            | AbilityEffectDefinition::CurseDamage { damage_sides, .. }
            | AbilityEffectDefinition::VisibleDamage { damage_sides, .. }
            | AbilityEffectDefinition::DrainLife { damage_sides, .. },
            AbilityLevelScalingField::DamageSides,
        ) => {
            *damage_sides = u16::try_from(scaled_ability_level_value(
                u64::from(*damage_sides),
                scaling,
                level,
            ))
            .expect("validated level-scaled damage sides must fit u16");
        }
        (
            AbilityEffectDefinition::Damage { damage_bonus, .. }
            | AbilityEffectDefinition::Malediction { damage_bonus, .. }
            | AbilityEffectDefinition::AreaDamage { damage_bonus, .. }
            | AbilityEffectDefinition::LavaFlow { damage_bonus, .. }
            | AbilityEffectDefinition::InsanityCircle { damage_bonus, .. }
            | AbilityEffectDefinition::Hellfire { damage_bonus, .. }
            | AbilityEffectDefinition::BeamDamage { damage_bonus, .. }
            | AbilityEffectDefinition::BoltOrBeamDamage { damage_bonus, .. }
            | AbilityEffectDefinition::BoltOrAreaDamage { damage_bonus, .. }
            | AbilityEffectDefinition::ConeDamage { damage_bonus, .. }
            | AbilityEffectDefinition::CurseDamage { damage_bonus, .. }
            | AbilityEffectDefinition::VisibleDamage { damage_bonus, .. }
            | AbilityEffectDefinition::DrainLife { damage_bonus, .. },
            AbilityLevelScalingField::DamageBonus,
        ) => {
            *damage_bonus = u16::try_from(scaled_ability_level_value(
                u64::from(*damage_bonus),
                scaling,
                level,
            ))
            .expect("validated level-scaled damage bonus must fit u16");
        }
        (AbilityEffectDefinition::DeathRay { power }, AbilityLevelScalingField::DeathRayPower) => {
            *power = u32::try_from(scaled_ability_level_value(
                u64::from(*power),
                scaling,
                level,
            ))
            .expect("validated level-scaled death ray power must fit u32");
        }
        (
            AbilityEffectDefinition::TeleportAway { power, .. },
            AbilityLevelScalingField::TeleportAwayPower,
        )
        | (
            AbilityEffectDefinition::RechargeFromPlayer { power },
            AbilityLevelScalingField::RechargePower,
        ) => {
            *power = u16::try_from(scaled_ability_level_value(
                u64::from(*power),
                scaling,
                level,
            ))
            .expect("validated level-scaled effect power must fit u16");
        }
        (
            AbilityEffectDefinition::IdentifyItem {
                full_identify_power,
                ..
            },
            AbilityLevelScalingField::IdentifyPower,
        ) => {
            *full_identify_power = u16::try_from(scaled_ability_level_value(
                u64::from(*full_identify_power),
                scaling,
                level,
            ))
            .expect("validated level-scaled identify power must fit u16");
        }
        (
            AbilityEffectDefinition::BoltOrBeamDamage {
                beam_chance_percent,
                ..
            },
            AbilityLevelScalingField::BeamChancePercent,
        ) => {
            *beam_chance_percent = u8::try_from(scaled_ability_level_value(
                u64::from(*beam_chance_percent),
                scaling,
                level,
            ))
            .expect("validated level-scaled beam chance must fit u8");
        }
        (
            AbilityEffectDefinition::AreaDamage { radius, .. }
            | AbilityEffectDefinition::LavaFlow { radius, .. }
            | AbilityEffectDefinition::InsanityCircle { radius, .. }
            | AbilityEffectDefinition::Hellfire { radius, .. }
            | AbilityEffectDefinition::LightArea { radius, .. }
            | AbilityEffectDefinition::BoltOrAreaDamage { radius, .. }
            | AbilityEffectDefinition::ConeDamage { radius, .. }
            | AbilityEffectDefinition::BreathDamage { radius, .. }
            | AbilityEffectDefinition::Detect { radius, .. }
            | AbilityEffectDefinition::BlinkSelf { radius },
            AbilityLevelScalingField::Radius,
        ) => {
            *radius = u8::try_from(scaled_ability_level_value(
                u64::from(*radius),
                scaling,
                level,
            ))
            .expect("validated level-scaled radius must fit u8");
        }
        (AbilityEffectDefinition::DimensionDoor { range }, AbilityLevelScalingField::Radius) => {
            *range = u16::try_from(scaled_ability_level_value(
                u64::from(*range),
                scaling,
                level,
            ))
            .expect("validated level-scaled dimension door range must fit u16");
        }
        (
            AbilityEffectDefinition::ApplyStatus { intensity, .. }
            | AbilityEffectDefinition::VisibleApplyStatus { intensity, .. },
            AbilityLevelScalingField::StatusIntensity,
        ) => {
            *intensity = u16::try_from(scaled_ability_level_value(
                u64::from(*intensity),
                scaling,
                level,
            ))
            .expect("validated level-scaled status intensity must fit u16");
        }
        (
            AbilityEffectDefinition::ApplyStatus { duration_ticks, .. }
            | AbilityEffectDefinition::VisibleApplyStatus { duration_ticks, .. }
            | AbilityEffectDefinition::SustainAttributes { duration_ticks },
            AbilityLevelScalingField::StatusDurationTicks,
        ) => {
            *duration_ticks = u32::try_from(scaled_ability_level_value(
                u64::from(*duration_ticks),
                scaling,
                level,
            ))
            .expect("validated level-scaled status duration must fit u32");
        }
        (
            AbilityEffectDefinition::ApplyStatus { duration_sides, .. }
            | AbilityEffectDefinition::VisibleApplyStatus { duration_sides, .. },
            AbilityLevelScalingField::StatusDurationSides,
        ) => {
            *duration_sides = u32::try_from(scaled_ability_level_value(
                u64::from(*duration_sides),
                scaling,
                level,
            ))
            .expect("validated level-scaled status duration sides must fit u32");
        }
        (
            AbilityEffectDefinition::ApplyStatus {
                granted_modifiers, ..
            },
            AbilityLevelScalingField::StatusDefense,
        ) => {
            granted_modifiers.defense = i32::try_from(scaled_ability_level_value(
                u64::try_from(granted_modifiers.defense)
                    .expect("validated status defense must be non-negative"),
                scaling,
                level,
            ))
            .expect("validated level-scaled status defense must fit i32");
        }
        (
            AbilityEffectDefinition::ApplyStatus {
                power: Some(power), ..
            }
            | AbilityEffectDefinition::VisibleApplyStatus {
                power: Some(power), ..
            }
            | AbilityEffectDefinition::Sanctuary { power, .. }
            | AbilityEffectDefinition::Entangle { power, .. }
            | AbilityEffectDefinition::TurnUndead { power },
            AbilityLevelScalingField::StatusPower,
        )
        | (
            AbilityEffectDefinition::Control { power, .. }
            | AbilityEffectDefinition::InsanityCircle {
                control_power: power,
                ..
            },
            AbilityLevelScalingField::ControlPower,
        )
        | (
            AbilityEffectDefinition::Genocide { power, .. },
            AbilityLevelScalingField::GenocidePower,
        ) => {
            *power = u16::try_from(scaled_ability_level_value(
                u64::from(*power),
                scaling,
                level,
            ))
            .expect("validated level-scaled effect power must fit u16");
        }
        (
            AbilityEffectDefinition::SummonCategory { maximum_level, .. },
            AbilityLevelScalingField::SummonMaximumLevel,
        ) => {
            *maximum_level = u16::try_from(scaled_ability_level_value(
                u64::from(*maximum_level),
                scaling,
                level,
            ))
            .expect("validated level-scaled summon maximum level must fit u16");
        }
        (
            AbilityEffectDefinition::FetchItem {
                maximum_weight_tenths_pound,
            },
            AbilityLevelScalingField::MaximumWeight,
        ) => {
            *maximum_weight_tenths_pound = u32::try_from(scaled_ability_level_value(
                u64::from(*maximum_weight_tenths_pound),
                scaling,
                level,
            ))
            .expect("validated level-scaled fetch weight must fit u32");
        }
        (
            AbilityEffectDefinition::Banish { maximum_distance },
            AbilityLevelScalingField::BanishDistance,
        ) => {
            *maximum_distance = u16::try_from(scaled_ability_level_value(
                u64::from(*maximum_distance),
                scaling,
                level,
            ))
            .expect("validated level-scaled banish distance must fit u16");
        }
        (
            AbilityEffectDefinition::DeviceMastery { duration_base, .. },
            AbilityLevelScalingField::DeviceMasteryDurationBase,
        ) => {
            *duration_base = u16::try_from(scaled_ability_level_value(
                u64::from(*duration_base),
                scaling,
                level,
            ))
            .expect("validated device mastery duration must fit u16");
        }
        (
            AbilityEffectDefinition::DeviceMastery {
                device_power_bonus, ..
            },
            AbilityLevelScalingField::DevicePowerBonus,
        ) => {
            *device_power_bonus = i32::try_from(scaled_ability_level_value(
                u64::try_from(*device_power_bonus)
                    .expect("validated device power bonus must be non-negative"),
                scaling,
                level,
            ))
            .expect("validated device power bonus must fit i32");
        }
        (
            AbilityEffectDefinition::ApplyStatus {
                granted_equipment_bonuses,
                ..
            },
            AbilityLevelScalingField::StatusMeleeDamage,
        ) => {
            granted_equipment_bonuses.melee_damage = i32::try_from(scaled_ability_level_value(
                u64::try_from(granted_equipment_bonuses.melee_damage)
                    .expect("validated status melee damage must be non-negative"),
                scaling,
                level,
            ))
            .expect("validated level-scaled status melee damage must fit i32");
        }
        (
            AbilityEffectDefinition::BeamDamage {
                maximum_range: Some(maximum_range),
                ..
            },
            AbilityLevelScalingField::MaximumRange,
        ) => {
            *maximum_range = u16::try_from(scaled_ability_level_value(
                u64::from(*maximum_range),
                scaling,
                level,
            ))
            .expect("validated level-scaled beam range must fit u16");
        }
        _ => unreachable!("content validation must reject incompatible level scaling fields"),
    }
}

fn spell_power_value(value: u64, bonus: i32) -> u64 {
    let value = i128::from(value);
    let adjusted = value + value * i128::from(bonus) / 13;
    u64::try_from(adjusted.max(0)).expect("non-negative spell power must fit u64")
}

fn device_power_value(value: u64, bonus: i32) -> u64 {
    let value = i128::from(value);
    let adjusted = value + value * i128::from(bonus) / 20;
    u64::try_from(adjusted.max(0)).expect("non-negative device power must fit u64")
}

fn apply_ability_spell_power(
    effect: &mut AbilityEffectDefinition,
    definition: AbilitySpellPowerDefinition,
    bonus: i32,
) {
    let scaled = |value| spell_power_value(value, bonus);
    match (effect, definition.field) {
        (
            AbilityEffectDefinition::Stardust { damage_dice, .. },
            AbilitySpellPowerField::DamageDice,
        ) => {
            *damage_dice = u16::try_from(scaled(u64::from(*damage_dice)))
                .expect("spell-powered damage dice must fit u16");
        }
        (
            AbilityEffectDefinition::Damage { damage_sides, .. }
            | AbilityEffectDefinition::Malediction { damage_sides, .. }
            | AbilityEffectDefinition::AreaDamage { damage_sides, .. }
            | AbilityEffectDefinition::LavaFlow { damage_sides, .. }
            | AbilityEffectDefinition::BeamDamage { damage_sides, .. }
            | AbilityEffectDefinition::LightArea { damage_sides, .. }
            | AbilityEffectDefinition::BoltOrBeamDamage { damage_sides, .. }
            | AbilityEffectDefinition::BoltOrAreaDamage { damage_sides, .. }
            | AbilityEffectDefinition::ConeDamage { damage_sides, .. }
            | AbilityEffectDefinition::VisibleDamage { damage_sides, .. }
            | AbilityEffectDefinition::DrainLife { damage_sides, .. },
            AbilitySpellPowerField::DamageSides,
        ) => {
            *damage_sides = u16::try_from(scaled(u64::from(*damage_sides)))
                .expect("spell-powered damage sides must fit u16");
        }
        (AbilityEffectDefinition::HealDice { sides, .. }, AbilitySpellPowerField::HealingSides) => {
            *sides = u16::try_from(scaled(u64::from(*sides)))
                .expect("spell-powered healing sides must fit u16");
        }
        (AbilityEffectDefinition::Heal { amount }, AbilitySpellPowerField::HealingAmount) => {
            *amount = u32::try_from(scaled(u64::from(*amount)))
                .expect("spell-powered healing amount must fit u32");
        }
        (
            AbilityEffectDefinition::Damage { damage_bonus, .. }
            | AbilityEffectDefinition::Malediction { damage_bonus, .. }
            | AbilityEffectDefinition::AreaDamage { damage_bonus, .. }
            | AbilityEffectDefinition::LavaFlow { damage_bonus, .. }
            | AbilityEffectDefinition::InsanityCircle { damage_bonus, .. }
            | AbilityEffectDefinition::Hellfire { damage_bonus, .. }
            | AbilityEffectDefinition::BeamDamage { damage_bonus, .. }
            | AbilityEffectDefinition::BoltOrBeamDamage { damage_bonus, .. }
            | AbilityEffectDefinition::BoltOrAreaDamage { damage_bonus, .. }
            | AbilityEffectDefinition::ConeDamage { damage_bonus, .. }
            | AbilityEffectDefinition::VisibleDamage { damage_bonus, .. }
            | AbilityEffectDefinition::DrainLife { damage_bonus, .. },
            AbilitySpellPowerField::DamageBonus,
        ) => {
            *damage_bonus = u16::try_from(scaled(u64::from(*damage_bonus)))
                .expect("spell-powered damage bonus must fit u16");
        }
        (
            AbilityEffectDefinition::AreaDamage { radius, .. }
            | AbilityEffectDefinition::LavaFlow { radius, .. }
            | AbilityEffectDefinition::InsanityCircle { radius, .. }
            | AbilityEffectDefinition::Hellfire { radius, .. }
            | AbilityEffectDefinition::LightArea { radius, .. }
            | AbilityEffectDefinition::BoltOrAreaDamage { radius, .. }
            | AbilityEffectDefinition::ConeDamage { radius, .. }
            | AbilityEffectDefinition::Earthquake { radius, .. },
            AbilitySpellPowerField::Radius,
        ) => {
            *radius =
                u8::try_from(scaled(u64::from(*radius))).expect("spell-powered radius must fit u8");
        }
        (AbilityEffectDefinition::DimensionDoor { range }, AbilitySpellPowerField::Radius) => {
            *range = u16::try_from(scaled(u64::from(*range)))
                .expect("spell-powered dimension door range must fit u16");
        }
        (
            AbilityEffectDefinition::ApplyStatus { duration_ticks, .. }
            | AbilityEffectDefinition::VisibleApplyStatus { duration_ticks, .. }
            | AbilityEffectDefinition::SustainAttributes { duration_ticks },
            AbilitySpellPowerField::StatusDurationTicks,
        ) => {
            *duration_ticks = u32::try_from(scaled(u64::from(*duration_ticks)))
                .expect("spell-powered status duration must fit u32");
        }
        (
            AbilityEffectDefinition::ApplyStatus { duration_sides, .. }
            | AbilityEffectDefinition::VisibleApplyStatus { duration_sides, .. },
            AbilitySpellPowerField::StatusDurationSides,
        ) => {
            *duration_sides = u32::try_from(scaled(u64::from(*duration_sides)))
                .expect("spell-powered status duration sides must fit u32");
        }
        (
            AbilityEffectDefinition::ApplyStatus {
                power: Some(power), ..
            }
            | AbilityEffectDefinition::VisibleApplyStatus {
                power: Some(power), ..
            }
            | AbilityEffectDefinition::Entangle { power, .. },
            AbilitySpellPowerField::StatusPower,
        )
        | (AbilityEffectDefinition::Control { power, .. }, AbilitySpellPowerField::ControlPower)
        | (
            AbilityEffectDefinition::InsanityCircle {
                control_power: power,
                ..
            },
            AbilitySpellPowerField::ControlPower,
        )
        | (
            AbilityEffectDefinition::Genocide { power, .. },
            AbilitySpellPowerField::GenocidePower,
        ) => {
            *power = u16::try_from(scaled(u64::from(*power)))
                .expect("spell-powered effect power must fit u16");
        }
        (
            AbilityEffectDefinition::SummonCategory { maximum_level, .. },
            AbilitySpellPowerField::SummonMaximumLevel,
        ) => {
            *maximum_level = u16::try_from(scaled(u64::from(*maximum_level)))
                .expect("spell-powered summon maximum level must fit u16");
        }
        (
            AbilityEffectDefinition::IdentifyItem {
                full_identify_power,
                ..
            },
            AbilitySpellPowerField::IdentifyPower,
        ) => {
            *full_identify_power = u16::try_from(scaled(u64::from(*full_identify_power)))
                .expect("spell-powered identify power must fit u16");
        }
        (
            AbilityEffectDefinition::TeleportAway { power, .. },
            AbilitySpellPowerField::TeleportAwayPower,
        )
        | (
            AbilityEffectDefinition::RechargeFromPlayer { power },
            AbilitySpellPowerField::RechargePower,
        ) => {
            *power = u16::try_from(scaled(u64::from(*power)))
                .expect("spell-powered effect power must fit u16");
        }
        (
            AbilityEffectDefinition::FetchItem {
                maximum_weight_tenths_pound,
            },
            AbilitySpellPowerField::MaximumWeight,
        ) => {
            *maximum_weight_tenths_pound =
                u32::try_from(scaled(u64::from(*maximum_weight_tenths_pound)))
                    .expect("spell-powered fetch weight must fit u32");
        }
        (
            AbilityEffectDefinition::Banish { maximum_distance },
            AbilitySpellPowerField::BanishDistance,
        ) => {
            *maximum_distance = u16::try_from(scaled(u64::from(*maximum_distance)))
                .expect("spell-powered banish distance must fit u16");
        }
        (
            AbilityEffectDefinition::DeviceMastery { duration_base, .. },
            AbilitySpellPowerField::DeviceMasteryDurationBase,
        ) => {
            *duration_base = u16::try_from(scaled(u64::from(*duration_base)))
                .expect("spell-powered device mastery duration must fit u16");
        }
        (
            AbilityEffectDefinition::Clairvoyance {
                telepathy_duration_sides,
                ..
            },
            AbilitySpellPowerField::ClairvoyanceDurationSides,
        ) => {
            *telepathy_duration_sides = u16::try_from(scaled(u64::from(*telepathy_duration_sides)))
                .expect("spell-powered clairvoyance duration must fit u16");
        }
        (
            AbilityEffectDefinition::BeamDamage {
                maximum_range: Some(maximum_range),
                ..
            },
            AbilitySpellPowerField::MaximumRange,
        ) => {
            *maximum_range = u16::try_from(scaled(u64::from(*maximum_range)))
                .expect("spell-powered beam range must fit u16");
        }
        (
            _,
            AbilitySpellPowerField::FinalDamage
            | AbilitySpellPowerField::FinalHealing
            | AbilitySpellPowerField::RandomChoiceRoll
            | AbilitySpellPowerField::MaledictionDeathRayPower
            | AbilitySpellPowerField::MaledictionFearPower
            | AbilitySpellPowerField::InvulnerabilityDuration,
        ) => {}
        _ => unreachable!("content validation must reject incompatible spell power fields"),
    }
}

fn ability_has_spell_power_field(
    ability: &AbilityDefinition,
    effect_index: u8,
    field: AbilitySpellPowerField,
) -> bool {
    ability
        .spell_power_fields
        .iter()
        .any(|definition| definition.effect_index == effect_index && definition.field == field)
}

fn spell_powered_ability_value(
    ability: &AbilityDefinition,
    effect_index: u8,
    field: AbilitySpellPowerField,
    value: u64,
) -> u64 {
    if ability_has_spell_power_field(ability, effect_index, field) {
        spell_power_value(value, ability.spell_power_bonus)
    } else {
        value
    }
}

fn ability_effect_spec_dto(effect: &AbilityEffectDefinition) -> AbilityEffectSpecDto {
    match effect {
        AbilityEffectDefinition::JumpDamage { .. }
        | AbilityEffectDefinition::BirdDrop
        | AbilityEffectDefinition::DraconianBreathDamage { .. } => {
            unreachable!("non-projected effects are resolved before player ability projection")
        }
        AbilityEffectDefinition::BlinkSelf { radius } => {
            AbilityEffectSpecDto::BlinkSelf { radius: *radius }
        }
        AbilityEffectDefinition::BlinkTarget { radius } => {
            AbilityEffectSpecDto::BlinkTarget { radius: *radius }
        }
        AbilityEffectDefinition::TeleportSelf { minimum_distance } => {
            AbilityEffectSpecDto::TeleportSelf {
                minimum_distance: *minimum_distance,
            }
        }
        AbilityEffectDefinition::TeleportTarget => AbilityEffectSpecDto::TeleportTarget,
        AbilityEffectDefinition::TeleportLevel => AbilityEffectSpecDto::TeleportLevel,
        AbilityEffectDefinition::CreateStair {
            up_terrain_id,
            down_terrain_id,
        } => AbilityEffectSpecDto::CreateStair {
            up_terrain_id: up_terrain_id.clone(),
            down_terrain_id: down_terrain_id.clone(),
        },
        AbilityEffectDefinition::TeleportTown => AbilityEffectSpecDto::TeleportTown,
        AbilityEffectDefinition::SelfKnowledge => AbilityEffectSpecDto::SelfKnowledge,
        AbilityEffectDefinition::DimensionDoor { range } => {
            AbilityEffectSpecDto::DimensionDoor { range: *range }
        }
        AbilityEffectDefinition::Damage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
        } => AbilityEffectSpecDto::Damage {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            damage_type: DamageType::from(*damage_type).into(),
            final_damage_spell_power_bonus: None,
        },
        AbilityEffectDefinition::Malediction {
            damage_dice,
            damage_sides,
            damage_bonus,
        } => AbilityEffectSpecDto::Damage {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            damage_type: DamageType::HellFire.into(),
            final_damage_spell_power_bonus: None,
        },
        AbilityEffectDefinition::AreaDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            radius,
            target_category,
        } => AbilityEffectSpecDto::AreaDamage {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            damage_type: DamageType::from(*damage_type).into(),
            radius: *radius,
            target_category: target_category.clone(),
            final_damage_spell_power_bonus: None,
        },
        AbilityEffectDefinition::BeamDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            ..
        } => AbilityEffectSpecDto::BeamDamage {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            damage_type: DamageType::from(*damage_type).into(),
            final_damage_spell_power_bonus: None,
        },
        AbilityEffectDefinition::LightLine {
            damage_dice,
            damage_sides,
        } => AbilityEffectSpecDto::LightLine {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
        },
        AbilityEffectDefinition::LightArea {
            damage_dice,
            damage_sides,
            radius,
            ..
        } => AbilityEffectSpecDto::LightArea {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            radius: *radius,
            final_damage_spell_power_bonus: None,
        },
        AbilityEffectDefinition::BoltOrBeamDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            beam_chance_percent,
            ..
        } => AbilityEffectSpecDto::BoltOrBeamDamage {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            damage_type: DamageType::from(*damage_type).into(),
            beam_chance_percent: *beam_chance_percent,
            final_damage_spell_power_bonus: None,
        },
        AbilityEffectDefinition::Stardust {
            damage_dice,
            damage_sides,
            count,
            deviation,
        } => AbilityEffectSpecDto::Stardust {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            count: *count,
            deviation: *deviation,
        },
        AbilityEffectDefinition::BoltOrAreaDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            area_from_level,
            radius,
            ..
        } => AbilityEffectSpecDto::BoltOrAreaDamage {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            damage_type: DamageType::from(*damage_type).into(),
            area_from_level: *area_from_level,
            radius: *radius,
            final_damage_spell_power_bonus: None,
        },
        AbilityEffectDefinition::ConeDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            radius,
        } => AbilityEffectSpecDto::ConeDamage {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            damage_type: DamageType::from(*damage_type).into(),
            radius: *radius,
            final_damage_spell_power_bonus: None,
        },
        AbilityEffectDefinition::BreathDamage {
            hp_percent,
            max_damage,
            damage_type,
            radius,
        } => AbilityEffectSpecDto::BreathDamage {
            hp_percent: *hp_percent,
            max_damage: *max_damage,
            damage_type: DamageType::from(*damage_type).into(),
            radius: *radius,
        },
        AbilityEffectDefinition::CurseDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_is_current_hp_percent,
            nonlethal,
        } => AbilityEffectSpecDto::CurseDamage {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            damage_is_current_hp_percent: *damage_is_current_hp_percent,
            nonlethal: *nonlethal,
        },
        AbilityEffectDefinition::DeathRay { power } => {
            AbilityEffectSpecDto::DeathRay { power: *power }
        }
        AbilityEffectDefinition::TeleportAway {
            minimum_distance,
            power,
            stop_at_actor,
            target_category,
        } => AbilityEffectSpecDto::TeleportAway {
            minimum_distance: *minimum_distance,
            power: *power,
            stop_at_actor: *stop_at_actor,
            target_category: target_category.clone(),
        },
        AbilityEffectDefinition::RechargeFromPlayer { power } => {
            AbilityEffectSpecDto::RechargeFromPlayer { power: *power }
        }
        AbilityEffectDefinition::Clairvoyance {
            telepathy_duration_ticks,
            telepathy_duration_dice,
            telepathy_duration_sides,
            ..
        } => AbilityEffectSpecDto::Clairvoyance {
            telepathy_duration_ticks: *telepathy_duration_ticks,
            telepathy_duration_dice: *telepathy_duration_dice,
            telepathy_duration_sides: *telepathy_duration_sides,
        },
        AbilityEffectDefinition::CallSunlight { vampire_damage } => {
            AbilityEffectSpecDto::CallSunlight {
                vampire_damage: *vampire_damage,
            }
        }
        AbilityEffectDefinition::NatureWrath => AbilityEffectSpecDto::NatureWrath,
        AbilityEffectDefinition::Probe => AbilityEffectSpecDto::Probe,
        AbilityEffectDefinition::CreateDoor { terrain_id } => AbilityEffectSpecDto::CreateDoor {
            terrain_id: terrain_id.clone(),
        },
        AbilityEffectDefinition::DeviceMastery {
            duration_base,
            device_power_bonus,
        } => AbilityEffectSpecDto::DeviceMastery {
            duration_base: *duration_base,
            device_power_bonus: *device_power_bonus,
        },
        AbilityEffectDefinition::Banish { maximum_distance } => AbilityEffectSpecDto::Banish {
            maximum_distance: *maximum_distance,
        },
        AbilityEffectDefinition::Invulnerability {
            duration_dice,
            duration_sides,
            duration_bonus,
        } => AbilityEffectSpecDto::Invulnerability {
            duration_dice: *duration_dice,
            duration_sides: *duration_sides,
            duration_bonus: *duration_bonus,
            duration_spell_power_bonus: None,
        },
        AbilityEffectDefinition::DrainResource { amount } => {
            AbilityEffectSpecDto::DrainResource { amount: *amount }
        }
        AbilityEffectDefinition::Amnesia => AbilityEffectSpecDto::Amnesia,
        AbilityEffectDefinition::DarkenRoom => AbilityEffectSpecDto::DarkenRoom,
        AbilityEffectDefinition::AggravateMonsters => AbilityEffectSpecDto::AggravateMonsters,
        AbilityEffectDefinition::Teleport => AbilityEffectSpecDto::Teleport,
        AbilityEffectDefinition::FetchItem {
            maximum_weight_tenths_pound,
        } => AbilityEffectSpecDto::FetchItem {
            maximum_weight_tenths_pound: *maximum_weight_tenths_pound,
        },
        AbilityEffectDefinition::ConsumeTerrain { nutrition } => {
            AbilityEffectSpecDto::ConsumeTerrain {
                nutrition: *nutrition,
            }
        }
        AbilityEffectDefinition::CreateItem {
            item_kind_id,
            quantity,
        } => AbilityEffectSpecDto::CreateItem {
            item_kind_id: item_kind_id.clone(),
            quantity: *quantity,
        },
        AbilityEffectDefinition::CreateAmmunition {
            item_kind_ids,
            quantity_minimum,
            quantity_maximum,
            source_item_tags,
            source_terrain_tags,
        } => AbilityEffectSpecDto::CreateAmmunition {
            item_kind_ids: item_kind_ids.clone(),
            quantity_minimum: *quantity_minimum,
            quantity_maximum: *quantity_maximum,
            source_item_tags: source_item_tags.clone(),
            source_terrain_tags: source_terrain_tags.clone(),
        },
        AbilityEffectDefinition::TransmuteItemToGold {
            value_divisor,
            unit_value_cap,
        } => AbilityEffectSpecDto::TransmuteItemToGold {
            value_divisor: *value_divisor,
            unit_value_cap: *unit_value_cap,
        },
        AbilityEffectDefinition::DrainItemMagic {
            base_power,
            level_multiplier,
            level_divisor,
        } => AbilityEffectSpecDto::DrainItemMagic {
            base_power: *base_power,
            level_multiplier: *level_multiplier,
            level_divisor: *level_divisor,
        },
        AbilityEffectDefinition::ReportMagic => AbilityEffectSpecDto::ReportMagic,
        AbilityEffectDefinition::Earthquake {
            radius,
            affect_chance_percent,
            floor_terrain_id,
            wall_terrain_ids,
        } => AbilityEffectSpecDto::Earthquake {
            radius: *radius,
            affect_chance_percent: *affect_chance_percent,
            floor_terrain_id: floor_terrain_id.clone(),
            wall_terrain_ids: wall_terrain_ids.clone(),
        },
        AbilityEffectDefinition::AreaDestruction {
            minimum_radius,
            maximum_radius,
            floor_terrain_id,
            wall_terrain_id,
            quartz_terrain_id,
            magma_terrain_id,
        } => AbilityEffectSpecDto::AreaDestruction {
            minimum_radius: *minimum_radius,
            maximum_radius: *maximum_radius,
            floor_terrain_id: floor_terrain_id.clone(),
            wall_terrain_id: wall_terrain_id.clone(),
            quartz_terrain_id: quartz_terrain_id.clone(),
            magma_terrain_id: magma_terrain_id.clone(),
        },
        AbilityEffectDefinition::SuppressMonsterReproduction {
            damage_dice,
            damage_sides,
            damage_bonus,
        } => AbilityEffectSpecDto::SuppressMonsterReproduction {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
        },
        AbilityEffectDefinition::MeleeThenTeleport {
            radius,
            failure_threshold,
        } => AbilityEffectSpecDto::MeleeThenTeleport {
            radius: *radius,
            failure_threshold: *failure_threshold,
        },
        AbilityEffectDefinition::PolymorphSelf => AbilityEffectSpecDto::PolymorphSelf,
        AbilityEffectDefinition::PolymorphTarget => AbilityEffectSpecDto::PolymorphTarget,
        AbilityEffectDefinition::SwapPosition => AbilityEffectSpecDto::SwapPosition,
        AbilityEffectDefinition::Recall {
            delay_dice,
            delay_sides,
            delay_bonus,
        } => AbilityEffectSpecDto::Recall {
            delay_dice: *delay_dice,
            delay_sides: *delay_sides,
            delay_bonus: *delay_bonus,
        },
        AbilityEffectDefinition::ResistElements {
            duration_dice,
            duration_sides,
            duration_bonus,
        } => AbilityEffectSpecDto::ResistElements {
            duration_dice: *duration_dice,
            duration_sides: *duration_sides,
            duration_bonus: *duration_bonus,
        },
        AbilityEffectDefinition::Summon {
            actor_kind_id,
            count,
            radius,
            duration_turns,
            hostile,
        } => AbilityEffectSpecDto::Summon {
            actor_kind_id: actor_kind_id.clone(),
            count: *count,
            radius: *radius,
            duration_turns: *duration_turns,
            hostile: *hostile,
        },
        AbilityEffectDefinition::SummonCategory {
            category,
            upgraded_category,
            upgrade_at_level,
            maximum_level,
            count_dice,
            count_sides,
            count_bonus,
            maximum_count,
            batch_candidates,
            hostile_chance_percent,
            friendly_group_chance_percent,
            hostile_group_chance_percent,
            group_count_dice,
            group_count_sides,
            group_count_bonus,
            allow_unique_hostile,
            radius,
            duration_turns,
        } => AbilityEffectSpecDto::SummonCategory {
            category: category.clone(),
            upgraded_category: upgraded_category.clone(),
            upgrade_at_level: *upgrade_at_level,
            maximum_level: *maximum_level,
            count_dice: *count_dice,
            count_sides: *count_sides,
            count_bonus: *count_bonus,
            maximum_count: *maximum_count,
            batch_candidates: batch_candidates
                .iter()
                .map(|candidate| AbilitySummonCandidateSpecDto {
                    actor_kind_id: candidate.actor_kind_id.clone(),
                    weight: candidate.weight,
                })
                .collect(),
            hostile_chance_percent: *hostile_chance_percent,
            friendly_group_chance_percent: *friendly_group_chance_percent,
            hostile_group_chance_percent: *hostile_group_chance_percent,
            group_count_dice: *group_count_dice,
            group_count_sides: *group_count_sides,
            group_count_bonus: *group_count_bonus,
            allow_unique_hostile: *allow_unique_hostile,
            radius: *radius,
            duration_turns: *duration_turns,
        },
        AbilityEffectDefinition::NatureGate {
            animal_category,
            hound_category,
            hydra_category,
            ent_actor_kind_id,
            radius,
            duration_turns,
        } => AbilityEffectSpecDto::NatureGate {
            animal_category: animal_category.clone(),
            hound_category: hound_category.clone(),
            hydra_category: hydra_category.clone(),
            ent_actor_kind_id: ent_actor_kind_id.clone(),
            radius: *radius,
            duration_turns: *duration_turns,
        },
        AbilityEffectDefinition::DemonSummoning => AbilityEffectSpecDto::DemonSummoning,
        AbilityEffectDefinition::AngelSummoning => AbilityEffectSpecDto::AngelSummoning,
        AbilityEffectDefinition::BanishEvil => AbilityEffectSpecDto::BanishEvil { power: 0 },
        AbilityEffectDefinition::WrathOfGod => AbilityEffectSpecDto::WrathOfGod {
            damage: 0,
            radius: 2,
            minimum_count: 11,
            maximum_count: 20,
        },
        AbilityEffectDefinition::DivineIntervention => AbilityEffectSpecDto::DivineIntervention {
            adjacent_damage: 0,
            visible_damage: 0,
            healing: 0,
            control_power: 0,
            stun_duration_ticks: 0,
        },
        AbilityEffectDefinition::Crusade => AbilityEffectSpecDto::Crusade {
            charm_power: 0,
            summon_attempts: 12,
        },
        AbilityEffectDefinition::InsanityCircle {
            damage_bonus,
            control_power,
            radius,
        } => AbilityEffectSpecDto::InsanityCircle {
            damage_bonus: *damage_bonus,
            control_power: *control_power,
            radius: *radius,
        },
        AbilityEffectDefinition::ExplodePets => AbilityEffectSpecDto::ExplodePets,
        AbilityEffectDefinition::SummonGreaterDemon {
            corpse_item_kind_id,
            radius,
        } => AbilityEffectSpecDto::SummonGreaterDemon {
            corpse_item_kind_id: corpse_item_kind_id.clone(),
            radius: *radius,
        },
        AbilityEffectDefinition::Hellfire {
            damage_bonus,
            radius,
            backlash_dice,
            backlash_sides,
            backlash_bonus,
        } => AbilityEffectSpecDto::Hellfire {
            damage_bonus: *damage_bonus,
            radius: *radius,
            backlash_dice: *backlash_dice,
            backlash_sides: *backlash_sides,
            backlash_bonus: *backlash_bonus,
        },
        AbilityEffectDefinition::LavaFlow {
            damage_dice,
            damage_sides,
            damage_bonus,
            radius,
            target_terrain_id,
        } => AbilityEffectSpecDto::LavaFlow {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            radius: *radius,
            target_terrain_id: target_terrain_id.clone(),
            final_damage_spell_power_bonus: None,
        },
        AbilityEffectDefinition::DoomHand => AbilityEffectSpecDto::DoomHand,
        AbilityEffectDefinition::Sanctuary { power, radius } => AbilityEffectSpecDto::Sanctuary {
            power: *power,
            radius: *radius,
        },
        AbilityEffectDefinition::Detect {
            subject,
            category,
            radius,
            persistent,
            through_walls,
        } => AbilityEffectSpecDto::Detect {
            subject: ability_detect_subject_dto(*subject),
            category: category.clone(),
            radius: *radius,
            persistent: *persistent,
            through_walls: *through_walls,
        },
        AbilityEffectDefinition::RefuelEquippedLight {
            maximum_fraction_divisor,
        } => AbilityEffectSpecDto::RefuelEquippedLight {
            maximum_fraction_divisor: *maximum_fraction_divisor,
        },
        AbilityEffectDefinition::TransformTerrain {
            source_terrain_ids,
            target_terrain_id,
            radius,
        } => AbilityEffectSpecDto::TransformTerrain {
            source_terrain_ids: source_terrain_ids.clone(),
            target_terrain_id: target_terrain_id.clone(),
            radius: *radius,
        },
        AbilityEffectDefinition::CreateAdjacentTerrain {
            source_terrain_ids,
            target_terrain_id,
        } => AbilityEffectSpecDto::CreateAdjacentTerrain {
            source_terrain_ids: source_terrain_ids.clone(),
            target_terrain_id: target_terrain_id.clone(),
        },
        AbilityEffectDefinition::CreateCurrentTerrain {
            source_terrain_ids,
            target_terrain_id,
        } => AbilityEffectSpecDto::CreateCurrentTerrain {
            source_terrain_ids: source_terrain_ids.clone(),
            target_terrain_id: target_terrain_id.clone(),
        },
        AbilityEffectDefinition::TerrainBeam { operation } => AbilityEffectSpecDto::TerrainBeam {
            operation: match operation {
                AbilityTerrainBeamOperationDefinition::JamDoors => {
                    AbilityTerrainBeamOperationDto::JamDoors
                }
                AbilityTerrainBeamOperationDefinition::DestroyTrapsAndDoors => {
                    AbilityTerrainBeamOperationDto::DestroyTrapsAndDoors
                }
                AbilityTerrainBeamOperationDefinition::StoneToMud => {
                    AbilityTerrainBeamOperationDto::StoneToMud
                }
            },
        },
        AbilityEffectDefinition::ApplyStatus {
            status_kind_id,
            intensity,
            duration_ticks,
            duration_dice,
            duration_sides,
            stacking,
            resistance_type,
            power,
            granted_resistances,
            granted_brands,
            granted_modifiers,
            granted_equipment_bonuses,
            granted_status_immunities,
            granted_race_id,
            grants_wall_passage,
            incoming_damage_percent,
        } => AbilityEffectSpecDto::ApplyStatus {
            status_kind_id: status_kind_id.clone(),
            intensity: *intensity,
            duration_ticks: *duration_ticks,
            duration_dice: *duration_dice,
            duration_sides: *duration_sides,
            stacking: ability_status_stacking_dto(*stacking),
            resistance_type: resistance_type.map(DamageType::from).map(Into::into),
            power: *power,
            granted_resistances: granted_resistances
                .iter()
                .map(|(damage_type, level)| ResistanceDto {
                    damage_type: DamageType::from(*damage_type).into(),
                    level: ResistanceLevel::from(*level).into(),
                })
                .collect(),
            granted_modifiers: stat_modifiers_dto(granted_modifiers),
            granted_equipment_bonuses: equipment_bonuses_dto(granted_equipment_bonuses),
            granted_status_immunities: granted_status_immunities.iter().cloned().collect(),
            granted_race_id: granted_race_id.clone(),
            grants_wall_passage: *grants_wall_passage,
            incoming_damage_percent: *incoming_damage_percent,
            granted_brands: granted_brands
                .iter()
                .copied()
                .map(weapon_brand_dto)
                .collect(),
        },
        AbilityEffectDefinition::RemoveStatus { status_kind_id } => {
            AbilityEffectSpecDto::RemoveStatus {
                status_kind_id: status_kind_id.clone(),
            }
        }
        AbilityEffectDefinition::Control { category, power } => AbilityEffectSpecDto::Control {
            category: category.clone(),
            power: *power,
        },
        AbilityEffectDefinition::SniperShot { mode } => AbilityEffectSpecDto::SniperShot {
            mode: match mode {
                SniperShotModeDefinition::Shining => SniperShotModeDto::Shining,
                SniperShotModeDefinition::Retreat => SniperShotModeDto::Retreat,
                SniperShotModeDefinition::Disarm => SniperShotModeDto::Disarm,
                SniperShotModeDefinition::Burning => SniperShotModeDto::Burning,
                SniperShotModeDefinition::Shatter => SniperShotModeDto::Shatter,
                SniperShotModeDefinition::Freezing => SniperShotModeDto::Freezing,
                SniperShotModeDefinition::Knockback => SniperShotModeDto::Knockback,
                SniperShotModeDefinition::Piercing => SniperShotModeDto::Piercing,
                SniperShotModeDefinition::Evil => SniperShotModeDto::Evil,
                SniperShotModeDefinition::Holy => SniperShotModeDto::Holy,
                SniperShotModeDefinition::Exploding => SniperShotModeDto::Exploding,
                SniperShotModeDefinition::Double => SniperShotModeDto::Double,
                SniperShotModeDefinition::Thunder => SniperShotModeDto::Thunder,
                SniperShotModeDefinition::Needle => SniperShotModeDto::Needle,
                SniperShotModeDefinition::Final => SniperShotModeDto::Final,
            },
        },
        AbilityEffectDefinition::MeleeAdjacent => AbilityEffectSpecDto::MeleeAdjacent,
        AbilityEffectDefinition::DraconianStrike { .. } => AbilityEffectSpecDto::MeleeAdjacent,
        AbilityEffectDefinition::ProbeMonsters => AbilityEffectSpecDto::ProbeMonsters,
        AbilityEffectDefinition::Concentrate => AbilityEffectSpecDto::Concentrate,
        AbilityEffectDefinition::Rodeo => AbilityEffectSpecDto::Rodeo,
        AbilityEffectDefinition::DrainLife {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            target_category,
            repeat,
            feeds,
        } => AbilityEffectSpecDto::DrainLife {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            damage_type: DamageType::from(*damage_type).into(),
            target_category: target_category.clone(),
            repeat: *repeat,
            feeds: *feeds,
            final_damage_spell_power_bonus: None,
        },
        AbilityEffectDefinition::Genocide {
            scope,
            power,
            radius,
            target_category,
            fatigue,
            ..
        } => AbilityEffectSpecDto::Genocide {
            scope: ability_genocide_scope_dto(*scope),
            power: *power,
            radius: *radius,
            target_category: target_category.clone(),
            fatigue: *fatigue,
        },
        AbilityEffectDefinition::IdentifyItem {
            full_identify_power,
            full_identify_roll_sides,
        } => AbilityEffectSpecDto::IdentifyItem {
            full_identify_power: *full_identify_power,
            full_identify_roll_sides: *full_identify_roll_sides,
        },
        AbilityEffectDefinition::IdentifyOrMassIdentify { mass, .. } => {
            if *mass {
                AbilityEffectSpecDto::MassIdentify
            } else {
                AbilityEffectSpecDto::IdentifyItem {
                    full_identify_power: 0,
                    full_identify_roll_sides: 0,
                }
            }
        }
        AbilityEffectDefinition::RestoreVitality { life_force, .. } => {
            AbilityEffectSpecDto::RestoreVitality {
                life_force: *life_force,
            }
        }
        AbilityEffectDefinition::AlterReality => AbilityEffectSpecDto::AlterReality,
        AbilityEffectDefinition::AnimateDead {
            actor_kind_id,
            corpse_item_kind_id,
            radius,
            count,
            ..
        } => AbilityEffectSpecDto::AnimateDead {
            actor_kind_id: actor_kind_id.clone(),
            corpse_item_kind_id: corpse_item_kind_id.clone(),
            radius: *radius,
            count: *count,
        },
        AbilityEffectDefinition::Heal { amount } => AbilityEffectSpecDto::Heal { amount: *amount },
        AbilityEffectDefinition::HealDice { dice, sides } => AbilityEffectSpecDto::HealDice {
            dice: *dice,
            sides: *sides,
            final_healing_spell_power_bonus: None,
        },
        AbilityEffectDefinition::RemoveEquippedCurses { include_heavy } => {
            AbilityEffectSpecDto::RemoveEquippedCurses {
                include_heavy: *include_heavy,
            }
        }
        AbilityEffectDefinition::BeginFasting => AbilityEffectSpecDto::BeginFasting,
        AbilityEffectDefinition::TurnUndead { power } => {
            AbilityEffectSpecDto::TurnUndead { power: *power }
        }
        AbilityEffectDefinition::SustainAttributes { duration_ticks } => {
            AbilityEffectSpecDto::SustainAttributes {
                duration_ticks: *duration_ticks,
            }
        }
        AbilityEffectDefinition::CureMutation => AbilityEffectSpecDto::CureMutation,
        AbilityEffectDefinition::ReduceStatus {
            status_kind_id,
            amount,
            current_divisor,
            remaining_divisor,
        } => AbilityEffectSpecDto::ReduceStatus {
            status_kind_id: status_kind_id.clone(),
            amount: *amount,
            current_divisor: *current_divisor,
            remaining_divisor: *remaining_divisor,
        },
        AbilityEffectDefinition::SatisfyHunger => AbilityEffectSpecDto::SatisfyHunger,
        AbilityEffectDefinition::DevourFlesh {
            maximum_hp_divisor,
            bleeding_amount,
        } => AbilityEffectSpecDto::DevourFlesh {
            maximum_hp_divisor: *maximum_hp_divisor,
            bleeding_amount: *bleeding_amount,
        },
        AbilityEffectDefinition::Vomit => AbilityEffectSpecDto::Vomit,
        AbilityEffectDefinition::VisibleDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            target_category,
            unlife_change_on_hit,
        } => AbilityEffectSpecDto::VisibleDamage {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            damage_type: DamageType::from(*damage_type).into(),
            target_category: target_category.clone(),
            unlife_change_on_hit: *unlife_change_on_hit,
            final_damage_spell_power_bonus: None,
        },
        AbilityEffectDefinition::VisibleApplyStatus {
            status_kind_id,
            intensity,
            duration_ticks,
            duration_dice,
            duration_sides,
            stacking,
            resistance_type,
            power,
            target_category,
        } => AbilityEffectSpecDto::VisibleApplyStatus {
            status_kind_id: status_kind_id.clone(),
            intensity: *intensity,
            duration_ticks: *duration_ticks,
            duration_dice: *duration_dice,
            duration_sides: *duration_sides,
            stacking: ability_status_stacking_dto(*stacking),
            resistance_type: resistance_type.map(DamageType::from).map(Into::into),
            power: *power,
            target_category: target_category.clone(),
        },
        AbilityEffectDefinition::Entangle {
            power,
            duration_ticks,
        } => AbilityEffectSpecDto::Entangle {
            power: *power,
            duration_ticks: *duration_ticks,
        },
        AbilityEffectDefinition::MassSleepOrStasis { stasis, power, .. } => {
            AbilityEffectSpecDto::VisibleApplyStatus {
                status_kind_id: if *stasis {
                    STATUS_PARALYSIS.to_owned()
                } else {
                    STATUS_SLEEP.to_owned()
                },
                intensity: 1,
                duration_ticks: if *stasis { 20 } else { 500 },
                duration_dice: 0,
                duration_sides: 0,
                stacking: if *stasis {
                    AbilityStatusStackingDto::Extend
                } else {
                    AbilityStatusStackingDto::KeepStrongest
                },
                resistance_type: None,
                power: Some(*power),
                target_category: None,
            }
        }
        AbilityEffectDefinition::SleepingDust { .. } => {
            unreachable!("sleeping dust must be level-scaled before projection")
        }
        AbilityEffectDefinition::BrandWeapon {
            affix_id,
            brand,
            resistance,
        } => AbilityEffectSpecDto::BrandWeapon {
            affix_id: affix_id.clone(),
            brand: brand.map(weapon_brand_dto),
            resistance: resistance.map(DamageType::from).map(Into::into),
        },
        AbilityEffectDefinition::ProtectFromCorrosion => AbilityEffectSpecDto::ProtectFromCorrosion,
        AbilityEffectDefinition::RandomChoice {
            roll_sides,
            level_bonus_divisor,
            branches,
        } => AbilityEffectSpecDto::RandomChoice {
            roll_sides: *roll_sides,
            level_bonus_divisor: *level_bonus_divisor,
            branches: branches
                .iter()
                .map(|branch| AbilityRandomBranchSpecDto {
                    maximum_roll: branch.maximum_roll,
                    target: match branch.target {
                        AbilityRandomTargetDefinition::CastTarget => {
                            AbilityRandomTargetDto::CastTarget
                        }
                        AbilityRandomTargetDefinition::SelfTarget => {
                            AbilityRandomTargetDto::SelfTarget
                        }
                    },
                    effect: Box::new(ability_effect_spec_dto(&branch.effect)),
                })
                .collect(),
            roll_spell_power_bonus: None,
        },
        AbilityEffectDefinition::NoOp { reason } => AbilityEffectSpecDto::NoOp {
            reason: reason.clone(),
        },
        AbilityEffectDefinition::Sequence { effects } => AbilityEffectSpecDto::Sequence {
            effects: effects.iter().map(ability_effect_spec_dto).collect(),
        },
    }
}

fn player_ability_effect_spec_dto(
    ability: &AbilityDefinition,
    effect_index: u8,
    effect: &AbilityEffectDefinition,
    level: u16,
    spell_damage_bonus: u16,
) -> AbilityEffectSpecDto {
    let mut spec = ability_effect_spec_dto(effect);
    match &mut spec {
        AbilityEffectSpecDto::BanishEvil { power } => {
            *power =
                spell_power_value(100, ability.spell_power_bonus).min(u64::from(u16::MAX)) as u16;
        }
        AbilityEffectSpecDto::WrathOfGod { damage, .. } => {
            let raw = level
                .saturating_mul(3)
                .saturating_add(25)
                .saturating_add(spell_damage_bonus);
            *damage = spell_power_value(u64::from(raw), ability.spell_power_bonus)
                .min(u64::from(u16::MAX)) as u16;
        }
        AbilityEffectSpecDto::DivineIntervention {
            adjacent_damage,
            visible_damage,
            healing,
            control_power,
            stun_duration_ticks,
        } => {
            *adjacent_damage = spell_power_value(
                u64::from(level.saturating_mul(11)),
                ability.spell_power_bonus,
            )
            .min(u64::from(u16::MAX)) as u16;
            *visible_damage = spell_power_value(
                u64::from(level.saturating_mul(4).saturating_add(spell_damage_bonus)),
                ability.spell_power_bonus,
            )
            .min(u64::from(u16::MAX)) as u16;
            *healing =
                spell_power_value(100, ability.spell_power_bonus).min(u64::from(u16::MAX)) as u16;
            *control_power = spell_power_value(
                u64::from(level.saturating_mul(4)),
                ability.spell_power_bonus,
            )
            .min(u64::from(u16::MAX)) as u16;
            *stun_duration_ticks = u32::from(5_u16.saturating_add(level / 5));
        }
        AbilityEffectSpecDto::Crusade { charm_power, .. } => {
            *charm_power = level.saturating_mul(4);
        }
        _ => {}
    }
    if ability.id == CRUSADE_ARREST_ABILITY_ID
        && let AbilityEffectSpecDto::ApplyStatus {
            power: Some(power), ..
        } = &mut spec
    {
        *power = u16::try_from(spell_power_value(
            u64::from(*power),
            ability.spell_power_bonus,
        ))
        .expect("validated Arrest display power must fit u16");
    }
    if ability.spell_power_bonus == 0 {
        return spec;
    }
    if ability_has_spell_power_field(ability, effect_index, AbilitySpellPowerField::FinalDamage) {
        let bonus = Some(ability.spell_power_bonus);
        match &mut spec {
            AbilityEffectSpecDto::Damage {
                final_damage_spell_power_bonus,
                ..
            }
            | AbilityEffectSpecDto::AreaDamage {
                final_damage_spell_power_bonus,
                ..
            }
            | AbilityEffectSpecDto::LavaFlow {
                final_damage_spell_power_bonus,
                ..
            }
            | AbilityEffectSpecDto::BeamDamage {
                final_damage_spell_power_bonus,
                ..
            }
            | AbilityEffectSpecDto::LightArea {
                final_damage_spell_power_bonus,
                ..
            }
            | AbilityEffectSpecDto::BoltOrBeamDamage {
                final_damage_spell_power_bonus,
                ..
            }
            | AbilityEffectSpecDto::BoltOrAreaDamage {
                final_damage_spell_power_bonus,
                ..
            }
            | AbilityEffectSpecDto::ConeDamage {
                final_damage_spell_power_bonus,
                ..
            }
            | AbilityEffectSpecDto::DrainLife {
                final_damage_spell_power_bonus,
                ..
            }
            | AbilityEffectSpecDto::VisibleDamage {
                final_damage_spell_power_bonus,
                ..
            } => *final_damage_spell_power_bonus = bonus,
            AbilityEffectSpecDto::RandomChoice { branches, .. } => {
                for branch in branches {
                    match branch.effect.as_mut() {
                        AbilityEffectSpecDto::Damage {
                            final_damage_spell_power_bonus,
                            ..
                        }
                        | AbilityEffectSpecDto::AreaDamage {
                            final_damage_spell_power_bonus,
                            ..
                        }
                        | AbilityEffectSpecDto::BeamDamage {
                            final_damage_spell_power_bonus,
                            ..
                        }
                        | AbilityEffectSpecDto::LightArea {
                            final_damage_spell_power_bonus,
                            ..
                        }
                        | AbilityEffectSpecDto::BoltOrBeamDamage {
                            final_damage_spell_power_bonus,
                            ..
                        }
                        | AbilityEffectSpecDto::BoltOrAreaDamage {
                            final_damage_spell_power_bonus,
                            ..
                        }
                        | AbilityEffectSpecDto::ConeDamage {
                            final_damage_spell_power_bonus,
                            ..
                        }
                        | AbilityEffectSpecDto::DrainLife {
                            final_damage_spell_power_bonus,
                            ..
                        }
                        | AbilityEffectSpecDto::VisibleDamage {
                            final_damage_spell_power_bonus,
                            ..
                        } => *final_damage_spell_power_bonus = bonus,
                        _ => unreachable!("validated random damage branch must project damage"),
                    }
                }
            }
            _ => unreachable!("validated final damage marker must project a damage effect"),
        }
    }
    if ability_has_spell_power_field(ability, effect_index, AbilitySpellPowerField::FinalHealing) {
        let AbilityEffectSpecDto::HealDice {
            final_healing_spell_power_bonus,
            ..
        } = &mut spec
        else {
            unreachable!("validated final healing marker must project a healing-dice effect");
        };
        *final_healing_spell_power_bonus = Some(ability.spell_power_bonus);
    }
    if ability_has_spell_power_field(
        ability,
        effect_index,
        AbilitySpellPowerField::RandomChoiceRoll,
    ) {
        let AbilityEffectSpecDto::RandomChoice {
            roll_spell_power_bonus,
            ..
        } = &mut spec
        else {
            unreachable!("validated random roll marker must project a random choice effect");
        };
        *roll_spell_power_bonus = Some(ability.spell_power_bonus);
    }
    if ability_has_spell_power_field(
        ability,
        effect_index,
        AbilitySpellPowerField::InvulnerabilityDuration,
    ) {
        let AbilityEffectSpecDto::Invulnerability {
            duration_spell_power_bonus,
            ..
        } = &mut spec
        else {
            unreachable!("validated invulnerability marker must project invulnerability");
        };
        *duration_spell_power_bonus = Some(ability.spell_power_bonus);
    }
    spec
}

fn ability_target_spec_dto(ability: &AbilityDefinition) -> TargetSpecDto {
    target_spec_dto(&ability.target)
}

fn target_spec_dto(target: &AbilityTargetDefinition) -> TargetSpecDto {
    TargetSpecDto {
        modes: target
            .modes
            .iter()
            .map(|mode| match mode {
                AbilityTargetModeDefinition::Direction => TargetModeDto::Direction,
                AbilityTargetModeDefinition::Position => TargetModeDto::Position,
                AbilityTargetModeDefinition::Entity => TargetModeDto::Entity,
                AbilityTargetModeDefinition::Item => TargetModeDto::Item,
                AbilityTargetModeDefinition::Town => TargetModeDto::Town,
                AbilityTargetModeDefinition::SelfTarget => TargetModeDto::SelfTarget,
            })
            .collect(),
        range: target.range,
        requires_line_of_effect: target.requires_line_of_effect,
    }
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

fn source_intensity(source: Position, target: Position, radius: i32, maximum: u8) -> u8 {
    let distance = rfb_distance(source, target);
    let radius = u32::try_from(radius).expect("validated light radius must be non-negative");
    if distance > radius {
        return 0;
    }

    // RFB treats the source and all eight adjacent grids as the same inner
    // light band. Every included outer band remains lit at reduced strength.
    let remaining = radius.saturating_sub(distance.saturating_sub(1));
    u8::try_from(u32::from(maximum).saturating_mul(remaining) / radius.max(1))
        .expect("scaled light intensity must fit u8")
}

#[derive(Debug, Clone, Copy)]
struct LightSource {
    position: Position,
    radius: i32,
    maximum: u8,
    color: u32,
    darkness: bool,
}

impl LightSource {
    fn contains(self, position: Position) -> bool {
        rfb_distance(self.position, position)
            <= u32::try_from(self.radius).expect("validated light radius must be non-negative")
    }
}

fn light_from_sources(
    sources: &[LightSource],
    position: Position,
    ambient_light: u8,
) -> CellLightDto {
    let mut strongest = (0_u8, PLAYER_LIGHT_COLOR);
    for source in sources.iter().filter(|source| !source.darkness) {
        let boost = source_intensity(source.position, position, source.radius, source.maximum);
        if boost > strongest.0 {
            strongest = (boost, source.color);
        }
    }
    CellLightDto {
        color: strongest.1,
        intensity: ambient_light.saturating_add(strongest.0),
    }
}

/// Angband/RFB's integer distance approximation: a rounded Euclidean
/// distance with the familiar max + min/2 fast path.  Keeping this separate
/// from the UI's Chebyshev targeting distance makes ball falloff and rings
/// match the original projection routine.
fn rfb_distance(from: Position, to: Position) -> u32 {
    let dy = from.y.abs_diff(to.y);
    let dx = from.x.abs_diff(to.x);
    let target = dy.saturating_mul(dy).saturating_add(dx.saturating_mul(dx));
    let mut distance = if dy > dx {
        dy + (dx >> 1)
    } else {
        dx + (dy >> 1)
    };
    if dy == 0 || dx == 0 {
        return distance;
    }
    loop {
        let denominator = distance.saturating_mul(2).max(1);
        let error = (target as i64 - distance.saturating_mul(distance) as i64) / denominator as i64;
        if error == 0 {
            return distance;
        }
        let next = (distance as i64 + error).max(0);
        distance = u32::try_from(next).unwrap_or(u32::MAX);
    }
}

fn rfb_area_damage(base_damage: i32, distance: u32) -> i32 {
    let numerator = i64::from(base_damage.max(0)).saturating_add(i64::from(distance));
    i32::try_from(numerator / i64::from(distance.saturating_add(1))).unwrap_or(i32::MAX)
}

fn projectile_path_between(
    origin: Position,
    target: Position,
    range: u16,
) -> Option<Vec<Position>> {
    if target == origin
        || origin.x.abs_diff(target.x).max(origin.y.abs_diff(target.y)) > u32::from(range)
    {
        return None;
    }
    let mut x = origin.x;
    let mut y = origin.y;
    let dx = (target.x - x).abs();
    let sx = if x < target.x { 1 } else { -1 };
    let dy = -(target.y - y).abs();
    let sy = if y < target.y { 1 } else { -1 };
    let mut error = dx + dy;
    let mut path = Vec::new();
    while path.len() < usize::from(range) {
        let doubled = error.saturating_mul(2);
        if doubled >= dy {
            error += dy;
            x += sx;
        }
        if doubled <= dx {
            error += dx;
            y += sy;
        }
        let position = Position { x, y };
        path.push(position);
        if position == target {
            return Some(path);
        }
    }
    None
}

fn projectile_path_through_target(
    origin: Position,
    target: Position,
    range: u16,
) -> Option<Vec<Position>> {
    if target == origin
        || origin.x.abs_diff(target.x).max(origin.y.abs_diff(target.y)) > u32::from(range)
    {
        return None;
    }
    let mut x = origin.x;
    let mut y = origin.y;
    let dx = (target.x - x).abs();
    let sx = if x < target.x { 1 } else { -1 };
    let dy = -(target.y - y).abs();
    let sy = if y < target.y { 1 } else { -1 };
    let mut error = dx + dy;
    let mut path = Vec::with_capacity(usize::from(range));
    while path.len() < usize::from(range) {
        let doubled = error.saturating_mul(2);
        if doubled >= dy {
            error += dy;
            x += sx;
        }
        if doubled <= dx {
            error += dx;
            y += sy;
        }
        path.push(Position { x, y });
    }
    path.contains(&target).then_some(path)
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

fn has_line_of_effect(game: &Game, from: Position, to: Position) -> bool {
    let mut x = from.x;
    let mut y = from.y;
    let dx = (to.x - from.x).abs();
    let dy = (to.y - from.y).abs();
    let step_x = if from.x < to.x { 1 } else { -1 };
    let step_y = if from.y < to.y { 1 } else { -1 };
    let mut error = dx - dy;

    loop {
        if x == to.x && y == to.y {
            return true;
        }
        if !(game.is_walkable(Position { x, y }) || (x == from.x && y == from.y)) {
            return false;
        }
        let double_error = error * 2;
        if double_error > -dy {
            error -= dy;
            x += step_x;
        }
        if double_error < dx {
            error += dx;
            y += step_y;
        }
        if game.index(Position { x, y }).is_none() {
            return false;
        }
    }
}

fn has_disintegration_line_of_effect(game: &Game, from: Position, to: Position) -> bool {
    let mut x = from.x;
    let mut y = from.y;
    let dx = (to.x - from.x).abs();
    let dy = (to.y - from.y).abs();
    let step_x = if from.x < to.x { 1 } else { -1 };
    let step_y = if from.y < to.y { 1 } else { -1 };
    let mut error = dx - dy;

    loop {
        let position = Position { x, y };
        let Some(index) = game.index(position) else {
            return false;
        };
        if position != from
            && game
                .content
                .terrain(&game.terrain[index])
                .is_some_and(|terrain| terrain.tags.iter().any(|tag| tag == "permanent"))
        {
            return false;
        }
        if position == to {
            return true;
        }
        let double_error = error * 2;
        if double_error > -dy {
            error -= dy;
            x += step_x;
        }
        if double_error < dx {
            error += dx;
            y += step_y;
        }
    }
}

fn has_line_of_sight(game: &Game, from: Position, to: Position) -> bool {
    let mut x = from.x;
    let mut y = from.y;
    let dx = (to.x - from.x).abs();
    let dy = (to.y - from.y).abs();
    let step_x = if from.x < to.x { 1 } else { -1 };
    let step_y = if from.y < to.y { 1 } else { -1 };
    let mut error = dx - dy;

    loop {
        if x == to.x && y == to.y {
            return true;
        }
        if !(x == from.x && y == from.y)
            && game
                .index(Position { x, y })
                .and_then(|index| game.content.terrain(&game.terrain[index]))
                .is_some_and(|terrain| terrain.blocks_sight)
        {
            return false;
        }
        let double_error = error * 2;
        if double_error > -dy {
            error -= dy;
            x += step_x;
        }
        if double_error < dx {
            error += dx;
            y += step_y;
        }
    }
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
