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
    check::{CheckContext, CheckKind, resolve_check},
    combat::{
        adjacent, apply_melee_armor_reduction, monster_melee_skill, rating_to_armor_class,
        rating_to_combat_value, resolve_armored_damage,
    },
    effect::{
        DamageOutcome, DamagePacket, EffectOutcome, EffectSpec, EffectTarget,
        STATUS_BASIC_RESISTANCE, STATUS_BLEEDING, STATUS_BLINDNESS, STATUS_CONFUSION, STATUS_FEAR,
        STATUS_HASTE, STATUS_PARALYSIS, STATUS_POISON, STATUS_PROTECTION_FROM_EVIL, STATUS_SLEEP,
        STATUS_SLOW, STATUS_STUN, STATUS_THERMAL_RESISTANCE, STATUS_VENGEANCE, StatusApplication,
        StatusChange, StatusInstance, StatusStacking, apply_effect, apply_status, resolve_damage,
    },
    error::CoreError,
    event::{DomainEvent, ItemAttributeChange, ProjectileTrace, project_events},
    rng::{RNG_ALGORITHM, RfbRng},
    save::{
        GENERATED_ITEM_ID_PREFIX, actor_from_runtime_spawn, actor_from_spawn,
        derive_next_item_instance_serial, position_from_content,
    },
    scheduler::{
        INITIAL_MONSTER_ENERGY_NEED, INITIAL_PLAYER_ENERGY_NEED, STANDARD_ACTION_COST, gain_energy,
        spend_energy,
    },
    state::{
        Actor, FloorConnectionState, FloorRegionState, FloorState, ItemInstance, ItemLocation,
        MonsterPackIdentity, ResourcePool, RolledAffixState, SummonIdentity,
    },
    stats::{
        AttributeKind, CharacterBuildIdentity, CharacterProgress, DerivedStat,
        DerivedStatsPipeline, StatBounds, StatKind, StatLayer,
    },
};
use rfb_content::{
    AbilityDefinition, AbilityDetectSubjectDefinition, AbilityEffectDefinition,
    AbilityGenocideScopeDefinition, AbilityLevelScalingCurveDefinition,
    AbilityLevelScalingDefinition, AbilityLevelScalingField, AbilityRandomTargetDefinition,
    AbilityStatusStackingDefinition, AbilityTargetDefinition, AbilityTargetModeDefinition,
    ActorResistanceLevel, ActorRole, AffixPropertyBundleDefinition, CastingAttribute,
    CastingProfileDefinition, ContentCatalog, DeviceRechargeProfileDefinition,
    DungeonInstanceLifecycle, EncounterEntryDefinition, EncounterTableDefinition, EquipmentBonuses,
    EquipmentPassive, FloorLifecycle, ItemAttributeDefinition, ItemCurseSeverityDefinition,
    ItemCurseTargetDefinition, ItemEnchantmentRollDefinition, ItemSummonLevelSourceDefinition,
    ItemSummonSelectorDefinition, ItemUseEffectDefinition, MonsterPackBehavior,
    ProceduralLayoutMode, ProceduralMazeDefinition, ProceduralPitDefinition,
    ProceduralRoomGeometryDefinition, ProceduralRoomShape, ProceduralStreamerCandidateDefinition,
    SkillKind, SlayLevel, SlayTarget, StartingItemDefinition, StatModifiers, TaskObjectiveKind,
    TechniqueAttribute, TechniqueProfileDefinition, TerrainFeatureEntryDefinition,
    ThemeVaultCandidateDefinition, WeaponBrand,
};
#[cfg(test)]
use rfb_content::{
    ContentPosition, DungeonEntryRequirementDefinition, DungeonEntryTaskStatus,
    TerrainFeaturePlacement, VaultTransform,
};
use rfb_protocol::{
    AbilityAreaDamageResolutionDto, AbilityBeamDamageResolutionDto, AbilityCastResolutionDto,
    AbilityConeDamageResolutionDto, AbilityControlOutcomeDto, AbilityDetectResolutionDto,
    AbilityDetectSubjectDto, AbilityEffectResolutionDto, AbilityEffectSkipReasonDto,
    AbilityEffectSpecDto, AbilityEffectsResolutionDto, AbilityGenocideScopeDto,
    AbilityProficiencyRankDto, AbilityProgressSaveDto, AbilityRandomBranchSpecDto,
    AbilityRandomTargetDto, AbilityStatusChangeDto, AbilityStatusStackingDto,
    AbilitySummonResolutionDto, AbilityTeleportResolutionDto, AbilityTerrainTransformResolutionDto,
    AbilityVisibleDamageResolutionDto, AttackProfileDto, CampaignStatusDto, CellLightDto,
    CellVisualDto, DamageDiceDto, DeviceRechargeSourceDto, Direction, EquipmentBonusesDto,
    EquipmentPassiveDto, GameCommandEnvelope, GameUpdate, HealingResolutionDto, ItemActivationDto,
    ItemChargesDto, ItemCurseRemovalResolutionDto, ItemCurseResolutionDto, ItemCurseSeverityDto,
    ItemEnchantmentComponentResolutionDto, ItemEnchantmentResolutionDto, ItemEnchantmentsDto,
    ItemIdentificationDto, ItemIdentifyResolutionDto, ItemKnowledgeDto, ItemPropertyDto,
    ItemQualityDto, MeleeBlowDto, MeleeRoutineDto, MonsterAbilityCandidateResolutionDto,
    MonsterAbilityCastResolutionDto, MonsterAbilityDecisionResolutionDto,
    MonsterAbilityRejectionReasonDto, MonsterAbilityTargetResolutionDto,
    MonsterDisplacementResolutionDto, MonsterPackBehaviorDto, MonsterPackRoleDto, Position,
    ProjectileProfileDto, RecallStateDto, ResistanceDto, ResourceGainResolutionDto,
    ResourceGainSourceDto, ResourcePoolSaveDto, ResourceRecoveryResolutionDto, RestResolutionDto,
    RestStopReasonDto, SlayDto, SlayLevelDto, SlayTargetDto, StatModifiersDto, SummonCommandDto,
    SummonCommandModeDto, SummonCommandResolutionDto, TargetModeDto, TargetSelection,
    TargetSpecDto, TaskStatusKindDto, ThrowProfileDto, WeaponBrandDto,
};

mod abilities;
mod capabilities;
mod damage;
mod death;
mod environment_combat;
mod floor;
mod inventory;
mod item_combat;
mod item_knowledge;
mod item_use;
mod monster_abilities;
mod monster_ai;
mod monster_combat;
mod persistence;
mod player_abilities;
mod player_combat;
mod player_stats;
mod progression;
mod snapshot;
mod status_effects;
mod tasks;
mod terrain;
mod validation;
mod world;

#[cfg(test)]
use abilities::AbilityTargetPlan;
use capabilities::{
    HealingRequest, ResourceRestorationRequest, StatusRemovalRequest, apply_healing,
    apply_resource_restoration, apply_status_application, apply_status_removal,
};
use damage::{
    FatalityPolicy, commit_damage_application, plan_damage_application, process_actor_status_tick,
    scale_damage_outcome,
};
use environment_combat::PlayerTrapOutcome;
use floor::{
    FloorTransitionTarget, RecallUseAction, dungeon_instance_id, dungeon_instance_storage_key,
    floor_dungeon_id, parse_dungeon_instance_ordinal,
};
use inventory::{
    CurseEquippedItemRequest, DeviceRechargeRequest, EquippedItemCurseTarget,
    InventoryItemRechargeOutcome, InventoryItemRechargeRequest, ItemEnchantmentRequest,
    ItemIdentificationRequest, ItemKnowledgeState, ItemPropertyKnowledgeState, PickUpOutcome,
    RemoveEquippedCursesRequest,
};
#[cfg(test)]
use item_use::ItemUsePlan;
use player_abilities::AbilityProgress;
#[cfg(test)]
use player_abilities::{SPELL_EXP_EXPERT, SPELL_EXP_MASTER};
use player_stats::{
    ResolvedThrowProfile, actor_melee_routine_dto, derived_speed, resolved_melee_blows,
};
use progression::{
    LifeForceRestorationRequest, apply_attribute_drain, apply_attribute_restoration,
    apply_experience_restoration, apply_learning_capacity_increase, apply_life_force_restoration,
    apply_permanent_attribute_increase, build_definitions, character_skill_progress,
    combine_percentages, initial_character_attributes, initial_resource_pool,
    profile_resource_maximum, resolve_character_build,
};
use status_effects::{
    ability_status_stacking_dto, apply_ability_status_effect, remove_ability_status_effect,
};
use tasks::{
    CampaignState, TaskState, abandoned_task_state, floor_task_id, initial_task_states,
    task_objectives,
};
use terrain::{DoorBashOutcome, DoorOpenOutcome, TerrainDigOutcome, TrapDisarmOutcome};
#[cfg(test)]
use world::generation::{
    GeneratedRoom, TerrainFeaturePlacementContext, set_generated_terrain,
    terrain_feature_placement_candidates,
};
use world::geometry::floor_position_is_walkable;
#[cfg(test)]
use world::geometry::{
    generated_terrain_index, generated_terrain_is_connected, maze_floor_anchors,
    maze_floor_distances, terrain_is_connectable, transformed_vault_dimensions,
    transformed_vault_position, vault_entrance_outward,
};

pub const BUILT_IN_WORLD_ID: &str = "demo.world.original-v1";
const PREVIOUS_BUILT_IN_CONTENT_HASHES: [&str; 141] = [
    "880610557b208e7c2459ff876c4ace1cb2ef9903986cb7883a04d511ca13c025",
    "0a76daadea3a9683ea8173aa8f65e6195a5582bdf7fdad215cea1a2896dfefcc",
    "cd2c813d224189c925a940e60a915fe3dcf6efa0ccadfc7363d06d428f56525f",
    "36bdba260173b9ba7477e85b886c134affed0369aa4f7a485e59e4408e618ebd",
    "d0537220f093719e623b51bf589dd0a3d8a67ccdc534a1502adcebe094120e9b",
    "e597eb10e3eec454ea78e8ad4e874a8ef41732c6f497083f4fb698d9a1935c69",
    "ee3446edab3354c091bd1edc6e0b5e8d478fd090767fee6796614d9372286a53",
    "12ba3295dfa8a9884bc7464a78b7dbb9cded01409ff22777db02df85d1aabed7",
    "dc371da0d48375a811a6421f1ccaa2e1310daa7aab856f852388f7da1a04c2b5",
    "6449bc9fa8717d7f6ffc4a2a9643c8e40d20f04c196fa80f23bec2823de8e3d5",
    "ce3d3810b9be824f20230d83d5978dbb555f5766813b5ac43c059be0e6293fe0",
    "cb56a8e9dd6d7280b38fe4e388fc0f7ce08fd4a40cef2c8886907e3c662ffc96",
    "87e77fccea2c1ea40a6d952abf8d0b38d286c049b34b73f0da93f00288d1c2ae",
    "154f5c333d2e352ff13734823a8cfded3e513b545c7b2e934663954887c375cf",
    "479728aa3cead56c7dbf886a1beb4a9f20b5034085da8836cb82f2191246e979",
    "43b38c37bc03ae81f8fe1e5a3f3c8afeba47921ff05321011bc227fb5813387f",
    "52c3db16ad5240ff83ba652b09ef70cccac991a586b593f84c11956a55539596",
    "419260921954602e9b707dd8c260f80ad3ff1ad0504ea2dfbde739ec64ca2d54",
    "130f0f9fbddbdb12d7742d222e2e4deceabddb51810834c264da45678e15d474",
    "b37af3a660c95c024d12c8232b6b5467cb7d57982e09431748f1516ed3c550c3",
    "a3b8149e550f4211b496d6500171e52031baccc2223c7c60bbb1874cf2015cab",
    "bdefe542bb40a876ae29f1e504ad8d9c7fcbbc4e5eba8092d937782fb88a74c3",
    "febe50b7a55a637a05d78135f14aa8f72fa457632ae8d705c002e92acf9e4fd9",
    "51ffdccfe19a9f159adc15c2f62965ff4a5d44b55990eb9f29df96870937a043",
    "f060f44c88033e8ef75478929a354d6b5b0bc5f933ca2772e79c3440940942e8",
    "2d2900d8052b0a600346d0b87cc3b3d5bb5138f851abbf2b95afa196bbbaaca2",
    "e69258b4a303a38c10221f90d01c49628eb9ef737e97c7e777fe30070a025f81",
    "224e4cc12f1f1a99e245b5e1a96e7c9371a6873460b6197c0f18007542c1a079",
    "4fdb1018d89fadee287aeff70b2ca059f62b867cfd8db8ed7f6409f7bbbd4765",
    "8319b75e64585ef782358ed5287e087d14fab3626dfa854296696751f66896ac",
    "830b8ededc0dadb5600436137da7edb41353f945a09a4325d05546e16e75c4a8",
    "738d40e03f4c4eaebb91d47c74ad7decd7c13ddd12cc41238d177408f66ea0cf",
    "c390fb30dcc041b266ee895e72441cf656dbacc470a24ba86bd8d7b948be994f",
    "b44f98cea0cc7f125421faebf3085a23c79228be2573daca38acef63abcca6ea",
    "328600bfda30da20bd2efe7faac1f97eda03cccecb3ae0b36f4b683e74e5869e",
    "02df91742a4ad4daf3aebe88c397f0a70396e36f9afc293cd87bdc310715929b",
    "9ff7c821379c543d13fc5ee690a84c71fa4267f210381781a54378040a876403",
    "7a65a77e6fec214a86be9ba7e6abbbebae14c7a68094b628f55d5960002e0b4f",
    "b37398cb9d005302c958a9e300d07a435e8631d6a5cd44ba63b0086069577c43",
    "0e6cf15310644e7b3eb2f7acb0c18a8b1a7fb08739e981e7492d4079e61ab44a",
    "e03cb30ea8e1cd5821c14b54c4a038d30323cfc2cb6e0d6c483cbb006d70916f",
    "ae7b19dd780d73091a5b34aed2f67dcbc5650d2e2ed1d7748cc86f48020f8fb0",
    "9c8fc3226c20300a308d21a5da69033efb853169214f4c411e6c740800bdf9ad",
    "5d65fd9ca827dd05fc035650b82046edb592d563565c7e4075b32512a43f4e1f",
    "7eea25faef326b6d2250af357359902d0acf32d393c831655508a7e7eee5f2f0",
    "de045e1652d6e484937743b84a98e5e77887f28340a6492e72e8c6e1f72326e6",
    "1f8848e160b4ec51ca36acc512920946888fec20a36d7ac7b860bdb126aff79a",
    "11a28d24125572468148dce77f0082340ab82a3a7ef87637303578681b31c4e9",
    "e3c0d8653f86663c6bb7eb2cf99caf9d1ba5a259566560d7d70bb9592de2b1e9",
    "461242cb2164434a7ef44a3692f1c9fa4ffe9921f07c17e0857c96f2f2d95041",
    "d209d68a6a39af21eee8d1a951684be86e847ab570823c9c2604fa199e4571e1",
    "ee07c276bbe568fafc1e1d6942e9d57d158bd250ed452b32c01c774d8521e96d",
    "4cdcad204a7ccad6d67b8dcb50ccdcc188220a72d258c37219974fad51e5274d",
    "9789fcbbd8431ed745d8a0305cc81a54cc7e45ce79be86ed76e0227d66564a02",
    "56fc449617a4c05c12ff11716c14b4f5c680cada9ad86c6ece736b52fa904bc2",
    "9d25687c1296bc6f9953024bd76bb9eefc4c1e3955280b96d34d565ff7ca289d",
    "246f51864965fac494c7a39959f591caa0434d9fa4eac839501f9d09526eb617",
    "9f3e3d5dee1e8777179179259380990b9253aa7f195f08cd29cbbd58562793df",
    "834acbe3d025810eb1399db74689d35a4d3dae34862bcbf1271c8d20ad11d9fc",
    "71d2f947fe2bb7b5e2190a12fdff12ba47ea9f7fc17b1eb26390b46d8abd092b",
    "1614fadbf4cd1d3ee03fc011eac069de3a1b8c23ec65b6f09e210f20008dbc4c",
    "06c054a8c083e05b9d0396aa1076fbe2133a6a1ce5f6c32f101e5d1dabd14b70",
    "84a8696e872a53ff24800645e5df8db49059f4f8cac0b4ebf17a982b16e529d5",
    "ad6b35c6e0ae8980a74fac51ea1e6597b09559541d4a85d598284dc2cb41d7e6",
    "1c94890a0f39d42a4b496a7222b8c9d191f24fe94b3c9d47d4a1eeea5364c5b4",
    "3188f4cf0937f44292980e8ca8fffc1db9c310e961af4502bd9380124e53d54a",
    "fa88458239f225a5033e5910c64ba30f8e1e4095fc82b1ebce6a5c914e05ad2d",
    "9f61f6161b77c553fc9dfed8d2e550abca8794d1dc997fb2af3f953feb711cb0",
    "bcc23bf5834c37bf7fb0874bcb1dfc72c751efad36f76d94b07391100e976316",
    "c16f6cf31b726461910fb09bc775b5b6d79af889fe0de046043f085e9593ad04",
    "acecaf504ebc3affaf67fbd8400016d85a8f4fd6b70fb7de3f1626887e5c6d62",
    "6f5f545e3b2c9cab98b6cd33f328679228b643ae147f20739c982863eba47bea",
    "817ccfc5924d6dd8d957fb1f2c97f191c08dd5c34aa1ff9dea265716d3236835",
    "30c38e57bd9a9d22694e02da9c2b5f07b76af0a4009deb59bbbc605703f5a504",
    "66e60826777d1bf79efb3eef6d718bcf3ed101e30c43d562fd122ff402eda95d",
    "aab3548090030a1d2d46496581fb41a9f2892213186aeb2236a7a79065fc069f",
    "8ac0aee6fe54abb2c97bbed3eedaaa510d32393126bd08f89d046d515a66213b",
    "6e3906fff5447c3b83630e85e6c789a0dc151d9e16e1faa484ed10dda41a3ee4",
    "d056b65f8e2c61615e48badd8a6f02cd725007789535aa363448c8a0e8288bea",
    "be6b9b098c495ee3f2af6075ea5790d16eae7e8487c1fa310575c0dad8cba5bd",
    "f9e9ccc93635da7f568a2cdd83f90024f86cd13d1d0ff43627f725dde4e3ecac",
    "29116f924e1ef4ddf6b0aa43f3b1b1bd0b4d28245ac086bce30d7a008e8e9e8e",
    "43da90740e88ba63d9839c992a90b0fcc9c008a379919e2bc624a208978e6252",
    "81e4e9d5f14d5a6e9990db8a6b1a60623eba81279c288b266d3274cfee523916",
    "3ed414503866baf22dd248b5a6e8bab6836ddfb0b288812a9a4bfd9cbd7eeecc",
    "134479da14e58dfd8c52d6587a33ad61ac97f7c430632ffca6ccd378b9ba7f30",
    "2646a2fe3c9bd4f56f22bbc604a4e303bf15f28d9ba6445645b396ef03f27dae",
    "01b74e86466aa5abfe682443819379504dde2efdf5d67d126fc3f1d20eb197a4",
    "f1fba31216da594e34b36b23bdf4570b46a934c7360ad0d66e01f1284529a9f2",
    "bb07fafa930ab51316bb5f11c819dda81b3003b238dfa2bf5e7dbb4b161b9a1b",
    "086d65709052cee99f2ddd3e44ed5b8776c3a3d52f9d96799bbddec9282cda34",
    "b425bafec4d4108b9eab4fd323b7b592f1e65ffb4197d45bcb1bc59567b61eff",
    "1380958f4743b474abe00c2dbbcf6719aa791945405f0276badc0d8d35a106e1",
    "83dc1e5a58f408b9945d627e469d2b53c1731963fb19752dcbc5c9c91359b188",
    "66842fb3b3291494aed23368a30374e93a8bceea2b397382ff89f08ecac180aa",
    "271fcf3f85ca347791150dbc8eec0040b9dd70e8315bdb3874bc2fc628d637bd",
    "c0708c7866d93bdbb6601d349300cd5ef5e95a7ebd754de60d62e27d6c4071c6",
    "26fdeb15063fa5ccc5a672cd8d2376f7ea66e7dc487fef6f1a4d5640a1050cf9",
    "5e6e5f4ee9b83eb8d80e05c8aa893bd8d19c1db1bdd18c97fe3e120fd823a88c",
    "d8bdbdd4d4e85862a97229c279a874668b9b1d3ce9035aa6f17a11cff7b3af80",
    "4105aec18bdc40aced03bb503ec31e30385248545266d116b1d0088a374c04c8",
    "8432e5d6b0143608415de0f49969b6445cd902ef4db58c218c347b5da85cabab",
    "f2bf96ea4a980a6a9914ca80dff5527a5e04b2e36d25aa668b118e6562c9cad9",
    "12c9160aec3bf8ebc6b7c92a785ad1ed8ad2dd23af674bd4bc6c445d2762d2e7",
    "c02d577a3eaf36f61c636c1b8bbdfcfa30935aef08ec4d9c5b59e77ef21b4d25",
    "10d3813ec933dd881c23229b604c5f64e67716a56ebdb20b6a844c98593a7653",
    "36d07a047c3a9a331f051d4a0ebaa87070caef56408efb375e3b61e7e3fb1d86",
    "9bfa2632f2be9129e39a59dad72f7bb9a64fd2f403d74c3feaee1302fb0fe459",
    "9d1c6c1e01fb4533aa5a9868f0adfcbe876148d98585412783d0da93f4019dff",
    "0b9023398c8213f9e74d7f0d4d076b8ce70819dbb5cd8cc4eb3a2b84d4996210",
    "99398a53687b4cf106939ddebcb08865f4a24ee147795e9de2ae8e08036aaf00",
    "a9fa7d716f4f5e13ba8f97cb9c72f1dfbb4ed84c83a284b3cde2219549fcb1dd",
    "b62824da6e34e2f72a367f94b2e46e50e279ba6ac4df88bece81021a156e90ab",
    "3fd2b0a8b58531b89629aa2b50ef943a7a5687bdcb619991a26a3c81a7437bf7",
    "ab0bcb63b25c6729fd95d5fba97a4f618f7aca4589f3931a9ac149615d6062b5",
    "db5233e09952166a195617182db8020cfacc457e2279d0ff403f16a941c49db2",
    "337e8599f02e53264b45ac1e899eb47b5ec6f4eeb6be0ae31b517c67ae6fb82b",
    "39a7a79bdabafa301140266e7119735a0a0f16ef6a7071b8c5d06de6a53655a8",
    "7d344bf57cf11e303fbbd6b98f9792e572792e97a696e9a2c1987ba6f349a149",
    "c920d9f1b78d5f51a8ebb1097a54c1f74efe7b4a83eb469809b2c3e60d9717d3",
    "757be0f1513b9cbfb2f77e08ceef8bff8ffcdb10fc7da17a0da05dbe32f908a0",
    "27ad6b88a3e4bdeb4f1464d2081f6f59e62cbbfbab14ed09e9b5bdfaf43ead24",
    "786aba7f693bac066d6caa0dbc848c97ac7bc01e4652bfeb2674cfa739130549",
    "d486f818e41cea542ac951f6a92abca69e298d29f5139e6219ddd0c34836ad52",
    "25d972db57c825d4e23f5a61532c00579f9467acbe10edf97f2c0600b00514f5",
    "5ef19e0ecaf7328a7eb4ef3ff69ca066858ca0cc718c6b2db84b078e281f2404",
    "1c6e2bf891c76796cca6eb53ea014caa03fb8bb1fa3a95b8df8fd81f942e8562",
    "497fbc6b137e9bc2d8162ad52b0253f4d655a37c58abe391be6bcdd94ef94d9e",
    "3098d9de2051029b4509acc3b8973cec0b76679dcacfa6ace1244864bc3f363d",
    "b33b104f3d7fd2153a66597b4f7685647020f3c9e3352366840dac326e650a57",
    "1b3c059fedbc14ad79a9549a8b0bd4496f22785355e2bb4ef1ce3a0f763c7e35",
    "99c41b9668586d97987cc18a459632c8f444d9c8dffbf1e6e024f2ce35a11091",
    "de5986a0133867854afb49f98e06a294528d9e4360bc88e7a0fa78d48fff8846",
    "6ecb079e1a1dd1e653e7c4d201f264d72e7c1db9bfe466f8d1ffa410cfee36e0",
    "48611b108dafc4b06836073ca6b5c6881779c653cbab569a7fdeaec82c1c707a",
    "8b3bdb097563d99b6433a5746c07d395b406d5c8d86616540e0126cd6af72404",
    "9f28bf79c8fc72bbcf97beec23da1c1fa0a10045b5c363defcb59e9a29457ed5",
    "136cc9508d1d45997f193c39689f8604e6e06db258e4a2d22e65b7a24b72f717",
    "ffd8f8111a5b956a26a6af12bd242aad04a322bb996f587a08fae9db4488925b",
    "2b1bf5beabe42513d3ad70e0d536274a773babf391c085f3af4ca7a720a2e003",
    "a8eb3c1a5b74f683bd5a71728da916f67972088769e3155cdc0b89c88b4e874c",
];
const EQUIPMENT_REGENERATION_INTERVAL_TICKS: u32 = 10;
const BUILT_IN_CONTENT_HASH: &str =
    "cf977b882f1650f641035e1e12b22cca6430106a4992cceefd2e496060f51774";
const BUILT_IN_CONTENT_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/rfb-demo-original.rfbcontent"));
pub const STATE_HASH_SCHEMA_VERSION: u16 = 55;
const VISIBILITY_RADIUS: i32 = 8;
const BASE_THROW_RANGE_BUDGET: u16 = 50;
const MIN_THROW_RANGE: u16 = 2;
const MAX_THROW_RANGE: u16 = 10;
const MAX_REST_TURNS: u16 = 100;
const AMBIENT_LIGHT: u8 = 28;
const PLAYER_LIGHT_RADIUS: i32 = 6;
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
const ACTOR_LIGHT_RADIUS: i32 = 5;
const ITEM_LIGHT_RADIUS: i32 = 4;
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
    Summon {
        positions: Vec<Position>,
    },
    SummonCategory {
        candidate_kind_ids: Vec<String>,
        positions: Vec<Position>,
    },
    BlinkSelf {
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
}

fn monster_plan_target(target: &MonsterAbilityTargetPlan) -> Option<&MonsterHostileTarget> {
    match target {
        MonsterAbilityTargetPlan::Projectile { target, .. }
        | MonsterAbilityTargetPlan::Area { target, .. }
        | MonsterAbilityTargetPlan::Beam { target, .. }
        | MonsterAbilityTargetPlan::Cone { target, .. }
        | MonsterAbilityTargetPlan::DragTarget { target, .. }
        | MonsterAbilityTargetPlan::BanishTarget { target, .. } => Some(target),
        MonsterAbilityTargetPlan::SelfTarget
        | MonsterAbilityTargetPlan::Summon { .. }
        | MonsterAbilityTargetPlan::SummonCategory { .. }
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum LootSource {
    MonsterCarried { actor_id: String },
    MonsterDeath { actor_id: String },
    FloorRoom { room_id: String, spawn_id: String },
    Vault { vault_id: String, spawn_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DungeonState {
    guardian_defeated: bool,
    entrance_guardian_defeated: bool,
    next_instance_ordinal: u32,
    retained_instance_id: Option<String>,
    retained_at_turn: Option<u32>,
}

fn initial_dungeon_states(world: &rfb_content::WorldDefinition) -> BTreeMap<String, DungeonState> {
    world
        .dungeons
        .iter()
        .map(|dungeon| {
            (
                dungeon.id.clone(),
                DungeonState {
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

/// The engine's standard humanoid body: the slot roster every player uses
/// unless their race declares its own `bodySlots`. Single-instance slot ids
/// equal their type so pre-template saves (e.g. `charm`) stay valid.
const STANDARD_BODY_SLOTS: [(&str, &str); 13] = [
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
    if race.body_slots.is_empty() {
        return Ok(standard_body_slots());
    }
    Ok(race
        .body_slots
        .iter()
        .map(|slot| BodySlot {
            id: slot.id.clone(),
            slot_type: slot.slot_type.clone(),
        })
        .collect())
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
    let (activation, charges) =
        initial_item_runtime_state(content, rng, &starting_item.item_kind_id, 1);
    items.push(ItemInstance {
        id,
        kind_id: starting_item.item_kind_id.clone(),
        quantity: starting_item.quantity,
        quality: ItemQualityDto::Ordinary,
        affix_ids: Vec::new(),
        rolled_affixes: Vec::new(),
        enchantments: ItemEnchantmentsDto::default(),
        curse: initial_item_curse(content, &starting_item.item_kind_id),
        activation,
        charges,
        device_recovery_progress: 0,
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

fn initial_item_runtime_state(
    content: &ContentCatalog,
    rng: &mut RfbRng,
    kind_id: &str,
    depth: u16,
) -> (Option<ItemActivationDto>, Option<ItemChargesDto>) {
    let Some(definition) = content.item(kind_id) else {
        return (None, None);
    };
    let Some(generation) = &definition.device_generation else {
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

#[derive(Debug, Clone)]
pub struct Game {
    content: Arc<ContentCatalog>,
    world_id: String,
    current_floor_id: String,
    current_dungeon_instance_id: Option<String>,
    stored_floors: BTreeMap<String, FloorState>,
    width: u16,
    height: u16,
    terrain: Vec<String>,
    player: Actor,
    build: Option<CharacterBuildIdentity>,
    body_slots: Vec<BodySlot>,
    progress: CharacterProgress,
    resources: BTreeMap<String, ResourcePool>,
    resources_touched: BTreeSet<String>,
    last_visual_cells: Option<Vec<CellVisualDto>>,
    bonus_spell_learning_capacity: u16,
    learned_abilities: BTreeSet<String>,
    ability_progress: BTreeMap<String, AbilityProgress>,
    entities: Vec<Actor>,
    items: Vec<ItemInstance>,
    item_knowledge: BTreeMap<String, ItemKnowledgeState>,
    item_property_knowledge: BTreeMap<String, ItemPropertyKnowledgeState>,
    task_states: BTreeMap<String, TaskState>,
    dungeon_states: BTreeMap<String, DungeonState>,
    campaign_state: CampaignState,
    summon_command: SummonCommandDto,
    recall: Option<RecallStateDto>,
    confusing_strike_ready: bool,
    next_item_instance_serial: u64,
    explored: Vec<bool>,
    revealed_terrain: BTreeSet<Position>,
    floor_connections: Vec<FloorConnectionState>,
    floor_regions: Vec<FloorRegionState>,
    rng: RfbRng,
    revision: u32,
    turn: u32,
    world_tick: u32,
    last_command_seq: u32,
    debug_ability_casts_succeed: bool,
    debug_recharge_attempts_succeed: bool,
    debug_recharge_attempts_fail: bool,
    debug_recharge_sources_survive: bool,
    debug_recall_delay_turns: Option<u16>,
    debug_item_curses_land: bool,
    debug_item_curses_resisted: bool,
}

impl Game {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self::from_content(
            seed,
            load_built_in_content().expect("built-in content should decode"),
            BUILT_IN_WORLD_ID,
        )
        .expect("built-in world should create a game")
    }

    pub fn new_with_build(seed: u64, build_id: &str) -> Result<Self, CoreError> {
        Self::from_content_with_build(
            seed,
            load_built_in_content().expect("built-in content should decode"),
            BUILT_IN_WORLD_ID,
            build_id,
        )
    }

    pub fn from_content(
        seed: u64,
        content: Arc<ContentCatalog>,
        world_id: &str,
    ) -> Result<Self, CoreError> {
        Self::from_content_internal(seed, content, world_id, None)
    }

    pub fn from_content_with_build(
        seed: u64,
        content: Arc<ContentCatalog>,
        world_id: &str,
        build_id: &str,
    ) -> Result<Self, CoreError> {
        Self::from_content_internal(seed, content, world_id, Some(build_id))
    }

    fn from_content_internal(
        seed: u64,
        content: Arc<ContentCatalog>,
        world_id: &str,
        build_id: Option<&str>,
    ) -> Result<Self, CoreError> {
        let world = content
            .world(world_id)
            .ok_or_else(|| CoreError::UnknownWorld(world_id.to_owned()))?;
        let build =
            resolve_character_build(&content, build_id.or(world.player_build_id.as_deref()))?;
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
        let player_definition = content
            .actor(&world.player.kind_id)
            .ok_or_else(|| CoreError::UnknownActor(world.player.kind_id.clone()))?;
        let player = actor_from_spawn(
            &world.player.instance_id,
            &world.player.kind_id,
            world.player.position,
            player_definition.max_hp,
            player_definition.speed,
            INITIAL_PLAYER_ENERGY_NEED,
            true,
        );
        let mut progress = CharacterProgress::new(seed, player_definition.max_hp);
        if let Some(identity) = build.as_ref() {
            let (definition, _, _, _) = build_definitions(&content, identity)?;
            progress.attributes = initial_character_attributes(definition);
            progress.maximum_attributes = progress.attributes;
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
                Ok(stamped_spawn(
                    actor_from_spawn(
                        &spawn.instance_id,
                        &spawn.kind_id,
                        spawn.position,
                        definition.max_hp,
                        definition.speed,
                        INITIAL_MONSTER_ENERGY_NEED,
                        actor_starts_alerted(definition),
                    ),
                    definition,
                ))
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
        for dungeon in &world.dungeons {
            let Some(guardian) = &dungeon.entrance_guardian else {
                continue;
            };
            let definition = content
                .actor(&guardian.actor_kind_id)
                .ok_or_else(|| CoreError::UnknownActor(guardian.actor_kind_id.clone()))?;
            let mut actor = stamped_spawn(
                actor_from_spawn(
                    &guardian.instance_id,
                    &guardian.actor_kind_id,
                    guardian.position,
                    definition.max_hp,
                    definition.speed,
                    INITIAL_MONSTER_ENERGY_NEED,
                    actor_starts_alerted(definition),
                ),
                definition,
            );
            actor.pack = Some(MonsterPackIdentity {
                id: guardian.instance_id.clone(),
                leader_id: guardian.instance_id.clone(),
                role: MonsterPackRoleDto::Leader,
                behavior: MonsterPackBehaviorDto::GuardPosition,
            });
            entities.push(actor);
        }
        let mut rng = RfbRng::seeded(seed);
        let mut items = world
            .items
            .iter()
            .map(|spawn| {
                let (activation, charges) =
                    initial_item_runtime_state(&content, &mut rng, &spawn.kind_id, 1);
                ItemInstance {
                    id: spawn.instance_id.clone(),
                    kind_id: spawn.kind_id.clone(),
                    quantity: spawn.quantity,
                    quality: item_quality_dto(spawn.quality),
                    affix_ids: spawn.affix_ids.clone(),
                    rolled_affixes: Vec::new(),
                    enchantments: ItemEnchantmentsDto::default(),
                    curse: initial_item_curse(&content, &spawn.kind_id),
                    activation,
                    charges,
                    device_recovery_progress: 0,
                    location: ItemLocation::Ground(position_from_content(spawn.position)),
                }
            })
            .collect::<Vec<_>>();
        let body_slots = resolve_body_slots(&content, build.as_ref())?;
        let mut next_item_instance_serial =
            derive_next_item_instance_serial(&player, &entities, &items)?;
        append_starting_items(
            &content,
            build.as_ref(),
            &body_slots,
            &mut items,
            &mut next_item_instance_serial,
            &mut rng,
        )?;
        let initial_floor_id = world.initial_floor_id.clone();
        let task_states = initial_task_states(world);
        let dungeon_states = initial_dungeon_states(world);
        let mut game = Self {
            content,
            world_id: world_id.to_owned(),
            current_floor_id: initial_floor_id,
            current_dungeon_instance_id: None,
            stored_floors: BTreeMap::new(),
            width,
            height,
            terrain,
            player,
            build,
            body_slots,
            progress,
            resources: BTreeMap::new(),
            resources_touched: BTreeSet::new(),
            last_visual_cells: None,
            bonus_spell_learning_capacity: 0,
            learned_abilities: BTreeSet::new(),
            ability_progress: BTreeMap::new(),
            entities,
            items,
            item_knowledge: BTreeMap::new(),
            item_property_knowledge: BTreeMap::new(),
            task_states,
            dungeon_states,
            campaign_state: CampaignState::default(),
            summon_command: SummonCommandDto::default(),
            recall: None,
            confusing_strike_ready: false,
            next_item_instance_serial,
            explored: vec![false; usize::from(width) * usize::from(height)],
            revealed_terrain: BTreeSet::new(),
            floor_connections: Vec::new(),
            floor_regions: Vec::new(),
            rng,
            revision: 0,
            turn: 0,
            world_tick: 0,
            last_command_seq: 0,
            debug_ability_casts_succeed: false,
            debug_recharge_attempts_succeed: false,
            debug_recharge_attempts_fail: false,
            debug_recharge_sources_survive: false,
            debug_recall_delay_turns: None,
            debug_item_curses_land: false,
            debug_item_curses_resisted: false,
        };
        game.initialize_player_ability_state();
        game.initialize_starting_item_knowledge();
        game.player.hp = game.effective_player_max_hp();
        game.initialize_carried_loot()?;
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
        self.validate_runtime_invariants(&action)?;
        let base_revision = self.revision;
        // The world only mutates inside dispatch, so the visuals recorded at
        // the end of the previous command are exactly this command's "before"
        // frame; recomputing them here would be a second full-map pass.
        let previous_visuals = self
            .last_visual_cells
            .take()
            .unwrap_or_else(|| self.visual_cells());
        let mut changed = BTreeSet::new();
        let mut events = Vec::new();
        let mut removed_entities = Vec::new();
        self.resources_touched.clear();
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
            GameAction::Equip { item_id }
                if self.cursed_equipment_replaced_by(item_id).is_some()
        );
        let unavailable_recharge = matches!(
            &action,
            GameAction::RechargeItem {
                target_item_id,
                source,
            } if self
                .device_recharge_unavailable_reason(target_item_id, source)
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
        let advances_world = !depleted_device_use
            && !zero_time_unavailable_item_use
            && !cursed_unequip
            && !cursed_equip_replacement
            && !unavailable_recharge
            && !unavailable_recharging_item
            && !matches!(
                &action,
                GameAction::Retire
                    | GameAction::IncreaseAttribute { .. }
                    | GameAction::Rest { .. }
                    | GameAction::SetSummonCommand { .. }
            );
        // Paralysis wastes any world-advancing action: the substituted idle
        // still spends the turn (energy, monster actions, status ticks) but
        // never grants deliberate wait recovery. Zero-time commands and Rest
        // stay available; rest turns tick paralysis down like any status.
        if advances_world && self.player_has_status_kind(STATUS_PARALYSIS) {
            action = GameAction::ParalyzedIdle;
        }
        let action_cost = action.energy_cost();
        let recover_after_wait = matches!(&action, GameAction::Wait);
        let mut turn_advance = 1_u32;
        if advances_world {
            self.decrement_ability_cooldowns(1);
        }

        match action {
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
            GameAction::CloseDoor { direction } => {
                if let Some(position) = self.close_door(direction) {
                    changed.insert(position);
                    events.push(DomainEvent::DoorClosed { position });
                } else {
                    events.push(DomainEvent::DoorCloseUnavailable);
                }
            }
            GameAction::Drop { item_ids } => {
                if let Some((stacks, quantity)) = self.drop_inventory_items(&item_ids) {
                    changed.insert(self.player.position);
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
                    events.push(DomainEvent::ItemsDropped {
                        stacks,
                        quantity: dropped_quantity,
                    });
                } else {
                    events.push(DomainEvent::NoItemsDropped);
                }
            }
            GameAction::Equip { item_id } => {
                if let Some((target_kind_id, slot_id, severity)) =
                    self.cursed_equipment_replaced_by(&item_id)
                {
                    events.push(DomainEvent::ItemUnequipCursed {
                        target_kind_id,
                        slot_id,
                        severity,
                    });
                } else if let Some(outcome) = self.equip_inventory_item(&item_id) {
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
            }
            GameAction::Fire { direction } => self.resolve_player_projectile(
                TargetSelection::Direction { direction },
                &mut events,
                &mut changed,
                &mut removed_entities,
            )?,
            GameAction::FireTarget { target } => self.resolve_player_projectile(
                target,
                &mut events,
                &mut changed,
                &mut removed_entities,
            )?,
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
            GameAction::RechargeItem {
                target_item_id,
                source,
            } => {
                self.recharge_inventory_item(&target_item_id, &source, &mut events);
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
                    ability_id,
                    reason: reason.to_owned(),
                }),
            },
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
            GameAction::PickUp => match self.pick_up_at_player()? {
                PickUpOutcome::Picked { kind_id, quantity } => {
                    changed.insert(self.player.position);
                    events.push(DomainEvent::ItemPickedUp {
                        target_kind_id: kind_id,
                        quantity,
                    });
                }
                PickUpOutcome::OverCapacity {
                    kind_id,
                    quantity,
                    current_weight,
                    pickup_weight,
                    capacity,
                } => events.push(DomainEvent::ItemPickupOverCapacity {
                    target_kind_id: kind_id,
                    quantity,
                    current_weight,
                    pickup_weight,
                    capacity,
                }),
                PickUpOutcome::Nothing => events.push(DomainEvent::NothingToPickUp),
            },
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
                let direction = self.confused_direction(direction, &mut events);
                let (dx, dy) = direction.delta();
                let target = Position {
                    x: self.player.position.x + dx,
                    y: self.player.position.y + dy,
                };
                if self.index(target).is_none()
                    || (!self.is_walkable(target) && !self.player_can_pass_walls())
                {
                    events.push(DomainEvent::MoveBlocked);
                } else if let Some(index) = self
                    .entities
                    .iter()
                    .position(|entity| entity.position == target)
                {
                    changed.insert(target);
                    if self.player_fear_blocks_melee(index) {
                        events.push(DomainEvent::PlayerFearBlocked {
                            status_kind_id: STATUS_FEAR.to_owned(),
                        });
                    } else {
                        self.resolve_player_melee(
                            index,
                            &mut events,
                            &mut changed,
                            &mut removed_entities,
                        )?;
                    }
                } else {
                    events.extend(self.relocate_player(target, &mut changed));
                }
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
            GameAction::DigTerrain { direction } => match self.dig_terrain(direction) {
                Some(TerrainDigOutcome::Succeeded { position }) => {
                    changed.insert(position);
                    events.push(DomainEvent::TerrainDug { position });
                }
                Some(TerrainDigOutcome::Failed { position }) => {
                    events.push(DomainEvent::TerrainDigFailed { position });
                }
                None => events.push(DomainEvent::TerrainDigUnavailable),
            },
        }

        if advances_world {
            spend_energy(&mut self.player.energy_need, action_cost);
            self.advance_until_player_ready(&mut events, &mut changed, &mut removed_entities)?;
            if recover_after_wait && !self.player_is_dead() {
                events.extend(
                    self.recover_player_resources(false)
                        .into_iter()
                        .map(|resolution| DomainEvent::ResourceRecovered { resolution }),
                );
            }
            if !self.player_is_dead() {
                self.decay_player_resources();
            }
        }
        self.apply_task_events(&events)?;
        self.apply_campaign_events(&mut events);

        self.last_command_seq = envelope.command_seq;
        self.turn = self.turn.saturating_add(turn_advance);
        self.revision = self.revision.saturating_add(1);
        self.reveal_current_visibility();
        let current_visuals = self.visual_cells();
        let changed_visual_cells = Self::changed_visual_cells(&current_visuals, &previous_visuals);
        self.last_visual_cells = Some(current_visuals);
        let events = project_events(events);

        Ok(GameUpdate {
            base_revision,
            revision: self.revision,
            turn: self.turn,
            world_tick: self.world_tick,
            command_seq: self.last_command_seq,
            floor_id: self.current_floor_id.clone(),
            dungeon_instance_id: self.current_dungeon_instance_id.clone(),
            events,
            changed_cells: changed
                .into_iter()
                .map(|position| self.cell_dto(position))
                .collect(),
            changed_visual_cells,
            player: self.player_dto(),
            entities: self.entities_dto(),
            items: self.items_dto(),
            inventory: self.inventory_dto(),
            equipment: self.equipment_dto(),
            removed_entities,
            terrain_interactions: self.terrain_interactions(),
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
            initial_item_runtime_state(&self.content, &mut self.rng, kind_id, depth);
        self.items.push(ItemInstance {
            id: id.to_owned(),
            kind_id: kind_id.to_owned(),
            quantity: 1,
            quality: ItemQualityDto::Ordinary,
            affix_ids: Vec::new(),
            rolled_affixes: Vec::new(),
            enchantments: ItemEnchantmentsDto::default(),
            curse: initial_item_curse(&self.content, kind_id),
            activation,
            charges,
            device_recovery_progress: 0,
            location: ItemLocation::Inventory,
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
        let Some(ability) = self.content.ability(ability_id) else {
            return Err("unknown-ability");
        };
        let ability = Self::effective_casting_ability(&profile, ability);
        if self.learned_abilities.contains(ability_id) {
            return Err("already-learned");
        }
        if self.progress.level < ability.minimum_level {
            return Err("level-too-low");
        }
        if !self.profile_supports_ability(&profile, ability_id) {
            return Err("ability-not-supported");
        }
        if self.learned_abilities.len() >= usize::from(self.ability_learning_capacity(&profile)) {
            return Err("learning-capacity-full");
        }
        let Some(book_id) = self
            .items
            .iter()
            .find(|item| {
                item.id == book_item_id
                    && item.location == ItemLocation::Inventory
                    && item.quantity == 1
            })
            .and_then(|item| self.content.item(&item.kind_id))
            .and_then(|item| item.ability_book_id.as_deref())
        else {
            return Err("book-unavailable");
        };
        if !profile.ability_book_ids.iter().any(|id| id == book_id)
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
                .any(|tag| matches!(tag.as_str(), "unique" | "guardian"));
            let fatigue_sides = match scope {
                AbilityGenocideScopeDefinition::Single => target_level.div_ceil(2),
                AbilityGenocideScopeDefinition::Glyph => 4,
                AbilityGenocideScopeDefinition::Nearby => 3,
            }
            .max(1);
            fatigue_damage = fatigue_damage.saturating_add(
                i32::try_from(self.rng.bounded(u64::from(fatigue_sides)) + 1)
                    .expect("genocide fatigue roll must fit i32"),
            );
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
        self.player.hp = self.player.hp.saturating_sub(fatigue_damage);
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
        self.content
            .actor_definitions()
            .filter(|definition| {
                let unique = definition.tags.iter().any(|tag| tag == "unique");
                definition.role == ActorRole::Monster
                    && definition.level <= u32::from(maximum_level)
                    && (category == "any-monster" || actor_matches_category(definition, category))
                    && excluded_category
                        .is_none_or(|category| !actor_matches_category(definition, category))
                    && !definition.tags.iter().any(|tag| tag == "guardian")
                    && (allow_unique || !unique)
                    && (!unique
                        || !self
                            .entities
                            .iter()
                            .any(|entity| entity.kind_id == definition.id && entity.hp > 0))
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
        let count = usize::try_from(rolled).unwrap_or(1).min(positions.len());
        let mut entity_ids = Vec::with_capacity(count);
        let mut summoned_kind_ids = Vec::with_capacity(count);
        let mut used_positions = Vec::with_capacity(count);
        for (ordinal, position) in positions.into_iter().take(count).enumerate() {
            if candidates.is_empty() {
                break;
            }
            let choice = usize::try_from(
                self.rng
                    .bounded(u64::try_from(candidates.len()).expect("candidate count fits")),
            )
            .expect("bounded summon choice must fit usize");
            let kind_id = candidates[choice].clone();
            let definition = self
                .content
                .actor(&kind_id)
                .expect("planned summon candidate must remain available")
                .clone();
            if definition.tags.iter().any(|tag| tag == "unique") {
                candidates.remove(choice);
            }
            let id = self.summon_entity_id(spec.source_id, ordinal);
            let mut entity = actor_from_runtime_spawn(
                &id,
                &kind_id,
                position,
                definition.max_hp,
                definition.speed,
                INITIAL_MONSTER_ENERGY_NEED,
                true,
            );
            entity.resistances = definition_resistance_profile(&definition);
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
            summoned_kind_ids.push(kind_id);
            used_positions.push(position);
            self.entities.push(entity);
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
    ) -> Option<Vec<Position>> {
        let candidates = self.open_positions_around(origin, radius);
        let count = usize::from(count);
        (candidates.len() >= count).then(|| candidates.into_iter().take(count).collect())
    }

    fn open_positions_around(&self, origin: Position, radius: u8) -> Vec<Position> {
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
                | ItemLocation::CarriedBy { .. } => None,
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
                    || !self.is_walkable(position)
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
            if category == "map" {
                for position in &positions {
                    let index = self
                        .index(*position)
                        .expect("mapped position must remain valid");
                    self.explored[index] = true;
                }
            } else {
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
                        .is_some_and(|definition| definition.tags.iter().any(|tag| tag == category))
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
                        category == "item" || definition.tags.iter().any(|tag| tag == category)
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

    fn terrain_transform_positions(
        &self,
        ability: &AbilityDefinition,
        center: Position,
        source_terrain_ids: &[String],
        target_terrain_id: &str,
        radius: u8,
    ) -> Option<Vec<Position>> {
        let origin = self.player.position;
        if !ability
            .target
            .modes
            .contains(&AbilityTargetModeDefinition::Position)
            || self.index(center).is_none()
            || origin.x.abs_diff(center.x).max(origin.y.abs_diff(center.y))
                > u32::from(ability.target.range)
            || !self.is_visible(center)
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
            .chain(std::iter::once(origin))
            .chain(self.items.iter().filter_map(|item| match item.location {
                ItemLocation::Ground(position) => Some(position),
                ItemLocation::Inventory
                | ItemLocation::Equipped { .. }
                | ItemLocation::CarriedBy { .. } => None,
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
                    || !self.is_visible(position)
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
                    .find(|entity| entity.id == *entity_id)
                    .map(|entity| entity.position)?;
                self.targeted_projectile_path_through_target(position, ability.target.range)
            }
            TargetSelection::SelfTarget => None,
            TargetSelection::Item { .. } => None,
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
                    .find(|entity| entity.id == *entity_id)
                    .map(|entity| entity.position)?;
                self.targeted_projectile_path(position, range)
            }
            TargetSelection::SelfTarget => None,
            TargetSelection::Item { .. } => None,
        }
    }

    fn targeted_projectile_path(&self, target: Position, range: u16) -> Option<Vec<Position>> {
        self.targeted_projectile_path_with_policy(target, range, false)
    }

    fn targeted_projectile_path_through_target(
        &self,
        target: Position,
        range: u16,
    ) -> Option<Vec<Position>> {
        self.targeted_projectile_path_with_policy(target, range, true)
    }

    fn targeted_projectile_path_with_policy(
        &self,
        target: Position,
        range: u16,
        continue_through_target: bool,
    ) -> Option<Vec<Position>> {
        let origin = self.player.position;
        if target == origin
            || self.index(target).is_none()
            || !self.is_visible(target)
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
        let origin = self.player.position;
        let mut impact = origin;
        let mut landing = origin;
        let mut traversed = Vec::new();
        let mut target_index = None;
        for position in path {
            impact = position;
            if self.index(position).is_none() || !self.is_walkable(position) {
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
        let cells = self.area_damage_cells(center, radius);
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
        let mut cells = Vec::new();
        let radius_limit = u32::from(radius);
        let radius = i32::from(radius);
        for y in center.y - radius..=center.y + radius {
            for x in center.x - radius..=center.x + radius {
                let position = Position { x, y };
                let distance = rfb_distance(center, position);
                if distance > radius_limit
                    || !self
                        .index(position)
                        .is_some_and(|_| has_line_of_effect(self, center, position))
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
    ) -> (Vec<Position>, Vec<(String, u32)>) {
        let cells = self.cone_damage_cells(self.player.position, centerline, direction, radius);
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
                        || !has_line_of_effect(self, origin, position)
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

    fn take_inventory_item_kind(
        &mut self,
        kind_id: &str,
    ) -> Result<Option<ItemInstance>, CoreError> {
        let Some(index) = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                item.kind_id == kind_id
                    && item.location == ItemLocation::Inventory
                    && item.quantity > 0
            })
            .min_by(|(_, left), (_, right)| left.id.cmp(&right.id))
            .map(|(index, _)| index)
        else {
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

    fn device_recharge_unavailable_reason(
        &self,
        target_item_id: &str,
        source: &DeviceRechargeSourceDto,
    ) -> Option<&'static str> {
        let Some(profile) = self.device_recharge_profile() else {
            return Some("no-profile");
        };
        let target = self.items.iter().find(|item| {
            item.id == target_item_id
                && item.location == ItemLocation::Inventory
                && item.quantity > 0
        });
        let Some(target) = target else {
            return Some("target-unavailable");
        };
        if !self.item_can_receive_recharge(target) {
            return Some("target-not-rechargeable");
        }
        match source {
            DeviceRechargeSourceDto::Resource => {
                if self
                    .resources
                    .get(&profile.resource_id)
                    .is_none_or(|pool| pool.current == 0)
                {
                    Some("resource-empty")
                } else {
                    None
                }
            }
            DeviceRechargeSourceDto::Item { item_id } => {
                if item_id == target_item_id {
                    return Some("source-is-target");
                }
                self.items
                    .iter()
                    .find(|item| {
                        item.id == *item_id
                            && item.location == ItemLocation::Inventory
                            && item.quantity > 0
                    })
                    .filter(|item| self.item_can_supply_recharge(item))
                    .map_or(Some("source-unavailable"), |_| None)
            }
        }
    }

    fn recharge_inventory_item(
        &mut self,
        target_item_id: &str,
        source: &DeviceRechargeSourceDto,
        events: &mut Vec<DomainEvent>,
    ) {
        if let Some(reason) = self.device_recharge_unavailable_reason(target_item_id, source) {
            events.push(DomainEvent::DeviceRechargeUnavailable {
                target_item_id: target_item_id.to_owned(),
                reason: reason.to_owned(),
            });
            return;
        }
        let profile = self
            .device_recharge_profile()
            .cloned()
            .expect("validated recharge action must retain its class profile");
        let power = u32::from(profile.power);
        match source {
            DeviceRechargeSourceDto::Resource => {
                let target_charges = self
                    .items
                    .iter()
                    .find(|item| item.id == target_item_id)
                    .and_then(|item| item.charges)
                    .expect("preflighted recharge target must carry energy");
                let missing = target_charges
                    .maximum
                    .saturating_sub(target_charges.current);
                let pool = self
                    .resources
                    .get_mut(&profile.resource_id)
                    .expect("recharge resource must remain initialized");
                let attempted = power.min(pool.current).min(missing);
                pool.current -= attempted;
                self.resources_touched.insert(profile.resource_id.clone());
                let outcome = self.recharge_inventory_item_from_resource(
                    target_item_id,
                    InventoryItemRechargeRequest::new(attempted, power),
                );
                events.push(device_recharge_resolved_event(
                    outcome,
                    profile.resource_id,
                    false,
                    false,
                ));
            }
            DeviceRechargeSourceDto::Item { item_id } => {
                let outcome = self.recharge_inventory_item_from_device(
                    target_item_id,
                    item_id,
                    DeviceRechargeRequest::new(power, profile.source_item_destruction_one_in),
                );
                events.push(device_recharge_resolved_event(
                    outcome.target,
                    outcome.source_kind_id,
                    true,
                    outcome.source_destroyed,
                ));
            }
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
        let Some(index) = self.items.iter().position(|item| {
            item.id == item_id && item.location == ItemLocation::Inventory && item.quantity > 0
        }) else {
            return Ok(None);
        };
        let item = &self.items[index];
        let definition = self.content.item(&item.kind_id).cloned().ok_or_else(|| {
            CoreError::Invariant(format!(
                "inventory item {} references missing kind {}",
                item.id, item.kind_id
            ))
        })?;
        if let Some(activation) = &item.activation
            && definition
                .device_generation
                .as_ref()
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
                && item.location == ItemLocation::Inventory
                && item.quantity > 0
        })?;
        let definition = self.content.item(&item.kind_id)?;
        if let Some(activation) = &item.activation {
            let profile = definition
                .device_generation
                .as_ref()?
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
        let Some((effect, target_definition)) = self.inventory_item_use_effect(source_item_id)
        else {
            return false;
        };
        (target_glyph.is_some()
            || matches!(effect, ItemUseEffectDefinition::Genocide { .. })
            || matches!(
                effect,
                ItemUseEffectDefinition::IdentifyItem { .. }
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

    /// Confusion scrambles one in-flight move: a bounded(4) draw of 0 keeps
    /// the intended direction (no event), anything else redirects to a
    /// bounded(8) draw over the canonical direction order. Both draws only
    /// happen while the status is active, so unconfused replays are
    /// byte-identical.
    fn confused_direction(
        &mut self,
        intended: Direction,
        events: &mut Vec<DomainEvent>,
    ) -> Direction {
        const CANONICAL_DIRECTIONS: [Direction; 8] = [
            Direction::North,
            Direction::NorthEast,
            Direction::East,
            Direction::SouthEast,
            Direction::South,
            Direction::SouthWest,
            Direction::West,
            Direction::NorthWest,
        ];
        if !self.player_has_status_kind(STATUS_CONFUSION) {
            return intended;
        }
        if self.rng.bounded(4) == 0 {
            return intended;
        }
        let actual =
            CANONICAL_DIRECTIONS[usize::try_from(self.rng.bounded(8)).expect("index fits")];
        events.push(DomainEvent::PlayerConfusedMove { intended, actual });
        actual
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
        let ability = self.player_derived_stats().melee_skill;
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

    fn advance_until_player_ready(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        loop {
            self.world_tick = self.world_tick.saturating_add(1);
            self.process_status_tick(events, changed, removed_entities)?;
            if self.player_is_dead() {
                break;
            }
            self.process_equipment_regeneration(events);
            self.process_inventory_device_recovery(events);
            self.process_monster_energy_pulse(events, changed, removed_entities)?;
            if self.player_is_dead() {
                break;
            }
            let speed = derived_speed(&self.player_derived_stats().speed);
            gain_energy(&mut self.player.energy_need, speed);
            if self.player.energy_need <= 0 {
                break;
            }
        }
        self.advance_summon_lifetimes(events, changed, removed_entities);
        if !self.player_is_dead() {
            self.advance_recall(events, changed)?;
        }
        Ok(())
    }

    fn process_equipment_regeneration(&mut self, events: &mut Vec<DomainEvent>) {
        if !self
            .world_tick
            .is_multiple_of(EQUIPMENT_REGENERATION_INTERVAL_TICKS)
            || !self
                .player_equipment_passives()
                .contains(&EquipmentPassive::Regeneration)
        {
            return;
        }
        let maximum = self.effective_player_max_hp();
        let before = self.player.hp;
        self.player.hp = self.player.hp.saturating_add(1).min(maximum);
        let applied = self.player.hp.saturating_sub(before);
        if applied > 0 {
            events.push(DomainEvent::EquipmentRegenerated {
                resolution: HealingResolutionDto {
                    requested: 1,
                    applied,
                },
            });
        }
    }

    fn process_inventory_device_recovery(&mut self, events: &mut Vec<DomainEvent>) {
        let world_tick = self.world_tick;
        let content = &self.content;
        for item in &mut self.items {
            if item.location != ItemLocation::Inventory {
                continue;
            }
            let Some(recovery) = content
                .item(&item.kind_id)
                .and_then(|definition| definition.device_generation.as_ref())
                .and_then(|generation| generation.recovery)
            else {
                continue;
            };
            if !world_tick.is_multiple_of(u32::from(recovery.interval_ticks)) {
                continue;
            }
            let Some(charges) = item.charges.as_mut() else {
                continue;
            };
            if charges.current >= charges.maximum {
                item.device_recovery_progress = 0;
                continue;
            }
            let scaled = u64::from(charges.maximum)
                .saturating_mul(u64::from(recovery.energy_per_mille))
                .saturating_add(u64::from(item.device_recovery_progress));
            let gain =
                u32::try_from(scaled / 1_000).expect("validated device recovery gain must fit u32");
            item.device_recovery_progress =
                u16::try_from(scaled % 1_000).expect("recovery remainder must fit u16");
            if gain == 0 {
                continue;
            }
            let before = charges.current;
            charges.current = charges.current.saturating_add(gain).min(charges.maximum);
            let applied = charges.current.saturating_sub(before);
            if charges.current == charges.maximum {
                item.device_recovery_progress = 0;
            }
            if applied > 0 {
                events.push(DomainEvent::DeviceEnergyRecovered {
                    target_item_id: item.id.clone(),
                    target_kind_id: item.kind_id.clone(),
                    amount: applied,
                    current: charges.current,
                    maximum: charges.maximum,
                });
            }
        }
    }

    fn advance_summon_lifetimes(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) {
        let mut entity_ids = self
            .entities
            .iter()
            .filter(|entity| entity.summon.is_some())
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        entity_ids.sort();
        for entity_id in entity_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id)
            else {
                continue;
            };
            let expires = self.entities[index]
                .summon
                .as_ref()
                .is_some_and(|summon| summon.remaining_turns <= 1);
            if expires {
                let position = self.entities[index].position;
                let target_kind_id = self.entities[index].kind_id.clone();
                let removed_id = self.entities[index].id.clone();
                self.entities.remove(index);
                changed.insert(position);
                removed_entities.push(removed_id.clone());
                events.push(DomainEvent::SummonExpired {
                    entity_id: removed_id,
                    target_kind_id,
                });
            } else if let Some(summon) = self.entities[index].summon.as_mut() {
                summon.remaining_turns = summon.remaining_turns.saturating_sub(1);
            }
        }
    }

    fn process_status_tick(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let player_damage_percent = self.player_incoming_damage_percent();
        let player_tick = process_actor_status_tick(&mut self.player, false, player_damage_percent);
        let player_status_expired = !player_tick.expired.is_empty();
        for damage in player_tick.damage {
            events.push(DomainEvent::PlayerStatusDamaged {
                status_kind_id: damage.status_kind_id,
                damage: damage.outcome,
            });
        }
        for status_kind_id in player_tick.expired {
            events.push(DomainEvent::PlayerStatusExpired { status_kind_id });
        }
        if player_status_expired {
            self.refresh_player_resource_maxima();
        }
        self.clamp_player_hp_to_effective_max();
        if let Some(damage) = player_tick.fatal_damage {
            events.push(DomainEvent::PlayerDiedFromStatus {
                status_kind_id: damage.status_kind_id,
                damage: damage.outcome,
            });
            return Ok(());
        }

        let mut entity_ids = self
            .entities
            .iter()
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        entity_ids.sort();
        for entity_id in entity_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id)
            else {
                continue;
            };
            let target_kind_id = self.entities[index].kind_id.clone();
            let tick = process_actor_status_tick(&mut self.entities[index], true, 100);
            if tick.awakened {
                events.push(DomainEvent::EntityAwakened {
                    target_kind_id: target_kind_id.clone(),
                });
            }
            for damage in tick.damage {
                events.push(DomainEvent::EntityStatusDamaged {
                    target_kind_id: target_kind_id.clone(),
                    status_kind_id: damage.status_kind_id,
                    damage: damage.outcome,
                });
            }
            for status_kind_id in tick.expired {
                events.push(DomainEvent::EntityStatusExpired {
                    target_kind_id: target_kind_id.clone(),
                    status_kind_id,
                });
            }
            if let Some(damage) = tick.fatal_damage {
                self.resolve_actor_death(
                    index,
                    DomainEvent::EntityDiedFromStatus {
                        target_kind_id,
                        status_kind_id: damage.status_kind_id,
                        damage: damage.outcome,
                    },
                    events,
                    changed,
                    removed_entities,
                )?;
            }
        }
        Ok(())
    }

    fn process_monster_energy_pulse(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let mut entity_ids = self
            .entities
            .iter()
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        entity_ids.sort();
        let mut surround_reservations = BTreeSet::new();

        for entity_id in entity_ids {
            if self.player_is_dead() {
                break;
            }
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id)
            else {
                continue;
            };
            let definition = self
                .content
                .actor(&self.entities[index].kind_id)
                .expect("monster actor definition must remain available");
            let speed = derived_speed(
                &self
                    .actor_derived_stats(&self.entities[index], definition, false)
                    .speed,
            );
            gain_energy(&mut self.entities[index].energy_need, speed);
            if self.entities[index].energy_need > 0 {
                continue;
            }
            spend_energy(&mut self.entities[index].energy_need, STANDARD_ACTION_COST);
            if self.entities[index]
                .statuses
                .iter()
                .any(|status| status.kind_id == STATUS_SLEEP)
            {
                events.push(DomainEvent::MonsterSlept {
                    target_kind_id: self.entities[index].kind_id.clone(),
                });
                continue;
            }
            self.resolve_monster_action(
                index,
                events,
                changed,
                removed_entities,
                &mut surround_reservations,
            )?;
        }
        Ok(())
    }

    fn resolve_monster_action(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
        surround_reservations: &mut BTreeSet<Position>,
    ) -> Result<(), CoreError> {
        if self.entity_is_player_aligned(index) {
            self.resolve_player_summon_action(index, events, changed, removed_entities)?;
            return Ok(());
        }
        if !self.entities[index].alerted && !self.resolve_monster_detection(index, events) {
            return Ok(());
        }
        if self.resolve_monster_ability_with_changes(index, events, changed, removed_entities)? {
            return Ok(());
        }
        let Some(primary_target) = self.monster_hostile_targets(index).into_iter().next() else {
            return Ok(());
        };
        let casting = self
            .content
            .actor(&self.entities[index].kind_id)
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
            let old_position = self.entities[index].position;
            self.entities[index].position = next_position;
            changed.insert(old_position);
            changed.insert(next_position);
            events.push(match reason {
                MonsterTacticalReason::Wounded => DomainEvent::MonsterFled {
                    source_kind_id: self.entities[index].kind_id.clone(),
                    target_kind_id: primary_target.kind_id().to_owned(),
                },
                MonsterTacticalReason::KeepDistance => DomainEvent::MonsterKeptDistance {
                    source_kind_id: self.entities[index].kind_id.clone(),
                    target_kind_id: primary_target.kind_id().to_owned(),
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
                let leader_position = self.entities[index].pack.as_ref().and_then(|pack| {
                    self.entities
                        .iter()
                        .find(|entity| entity.id == pack.leader_id)
                        .map(|leader| leader.position)
                });
                match leader_position {
                    Some(position) if adjacent(self.entities[index].position, position) => None,
                    Some(position) => self.next_monster_step_toward(index, position, true),
                    None => self.next_monster_step(index),
                }
            }
            MonsterPackBehaviorDto::GuardPosition => None,
        };
        let Some(next_position) = next_position else {
            return Ok(());
        };
        let old_position = self.entities[index].position;
        self.entities[index].position = next_position;
        changed.insert(old_position);
        changed.insert(next_position);
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
        let targets = self.player_summon_hostile_targets(index);
        let adjacent_target = targets.iter().find(|entity_id| {
            self.entities
                .iter()
                .find(|entity| entity.id == **entity_id)
                .is_some_and(|target| adjacent(self.entities[index].position, target.position))
        });
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
                        self.move_entity(index, next_position, changed);
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
            self.move_entity(index, next_position, changed);
        }
        Ok(())
    }

    fn player_summon_hostile_targets(&self, index: usize) -> Vec<String> {
        let origin = self.entities[index].position;
        let mut targets = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && entity.id != self.entities[index].id
                    && !self.actor_is_player_aligned(entity)
            })
            .map(|entity| {
                (
                    chebyshev_distance(origin, entity.position),
                    entity.id.clone(),
                )
            })
            .collect::<Vec<_>>();
        targets.sort();
        targets
            .into_iter()
            .map(|(_, entity_id)| entity_id)
            .collect()
    }

    fn next_player_summon_step_away_from_owner(&self, index: usize) -> Option<Position> {
        const DELTAS: [(i32, i32); 8] = [
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ];

        let start = self.entities[index].position;
        let current_distance = chebyshev_distance(start, self.player.position);
        let occupied = self
            .entities
            .iter()
            .enumerate()
            .filter(|(entity_index, entity)| *entity_index != index && entity.hp > 0)
            .map(|(_, entity)| entity.position)
            .collect::<BTreeSet<_>>();
        let mut candidates = DELTAS
            .iter()
            .enumerate()
            .filter_map(|(order, (dx, dy))| {
                let position = Position {
                    x: start.x + dx,
                    y: start.y + dy,
                };
                if position == self.player.position
                    || occupied.contains(&position)
                    || !self.is_walkable(position)
                {
                    return None;
                }
                let distance = chebyshev_distance(position, self.player.position);
                (distance > current_distance).then_some((
                    std::cmp::Reverse(distance),
                    order,
                    position,
                ))
            })
            .collect::<Vec<_>>();
        candidates.sort();
        candidates.first().map(|(_, _, position)| *position)
    }

    fn move_entity(
        &mut self,
        index: usize,
        next_position: Position,
        changed: &mut BTreeSet<Position>,
    ) {
        let old_position = self.entities[index].position;
        self.entities[index].position = next_position;
        changed.insert(old_position);
        changed.insert(next_position);
    }

    /// Row-major enumeration of open destinations for monster displacement:
    /// inside the map, walkable, free of the player and living actors, and
    /// different from the caster's current cell.
    fn displacement_destinations(
        &self,
        source_index: usize,
        accepts: impl Fn(Position) -> bool,
    ) -> Vec<Position> {
        let origin = self.entities[source_index].position;
        let mut destinations = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let position = Position {
                    x: i32::from(x),
                    y: i32::from(y),
                };
                if position == origin
                    || position == self.player.position
                    || !self.is_walkable(position)
                    || !accepts(position)
                    || self
                        .entities
                        .iter()
                        .any(|entity| entity.hp > 0 && entity.position == position)
                {
                    continue;
                }
                destinations.push(position);
            }
        }
        destinations
    }

    fn entity_is_player_aligned(&self, index: usize) -> bool {
        self.actor_is_player_aligned(&self.entities[index])
    }

    fn actor_is_player_aligned(&self, actor: &Actor) -> bool {
        actor.controller_id.as_deref() == Some(self.player.id.as_str())
            || actor
                .summon
                .as_ref()
                .is_some_and(|summon| summon.owner_id == self.player.id)
    }

    fn monster_hostile_targets(&self, source_index: usize) -> Vec<MonsterHostileTarget> {
        let origin = self.entities[source_index].position;
        let mut targets = Vec::new();
        if !self.player_is_dead() {
            targets.push(MonsterHostileTarget::Player {
                entity_id: self.player.id.clone(),
                kind_id: self.player.kind_id.clone(),
                position: self.player.position,
            });
        }
        targets.extend(
            self.entities
                .iter()
                .enumerate()
                .filter(|(index, entity)| {
                    *index != source_index && entity.hp > 0 && self.entity_is_player_aligned(*index)
                })
                .map(|(_, entity)| MonsterHostileTarget::Summon {
                    entity_id: entity.id.clone(),
                    kind_id: entity.kind_id.clone(),
                    position: entity.position,
                }),
        );
        targets.sort_by(|left, right| {
            let left_position = left.position();
            let right_position = right.position();
            let left_distance = origin
                .x
                .abs_diff(left_position.x)
                .max(origin.y.abs_diff(left_position.y));
            let right_distance = origin
                .x
                .abs_diff(right_position.x)
                .max(origin.y.abs_diff(right_position.y));
            left_distance
                .cmp(&right_distance)
                .then_with(|| right.is_player().cmp(&left.is_player()))
                .then_with(|| left.entity_id().cmp(right.entity_id()))
        });
        targets
    }

    fn resolve_monster_detection(&mut self, index: usize, events: &mut Vec<DomainEvent>) -> bool {
        let kind_id = self.entities[index].kind_id.clone();
        let Some(awareness) = self
            .content
            .actor(&kind_id)
            .and_then(|definition| definition.awareness.clone())
        else {
            self.entities[index].alerted = true;
            return true;
        };
        let monster_position = self.entities[index].position;
        let distance = monster_position
            .x
            .abs_diff(self.player.position.x)
            .max(monster_position.y.abs_diff(self.player.position.y));
        if distance > u32::from(awareness.detection_range)
            || !has_line_of_sight(self, monster_position, self.player.position)
        {
            return false;
        }
        let ability = self.player_derived_stats().stealth_skill;
        let mut difficulty_pipeline = DerivedStatsPipeline::new();
        difficulty_pipeline.add(
            StatKind::ActionDifficulty,
            StatLayer::Environment,
            &kind_id,
            awareness.detection_difficulty,
        );
        let check = resolve_check(
            &mut self.rng,
            CheckContext {
                kind: CheckKind::StealthDetection,
                actor_id: self.player.id.clone(),
                target_id: Some(self.entities[index].id.clone()),
                ability,
                difficulty: difficulty_pipeline
                    .resolve(StatKind::ActionDifficulty, StatBounds::NON_NEGATIVE),
            },
        );
        let stayed_hidden = check.succeeded();
        let skill_id = self
            .content
            .skill_by_kind(SkillKind::Stealth)
            .expect("validated stealth skill must remain available")
            .id
            .clone();
        events.push(DomainEvent::StealthChecked {
            source_kind_id: kind_id,
            succeeded: stayed_hidden,
            resolution: check.to_dto(skill_id),
        });
        if stayed_hidden {
            false
        } else {
            self.entities[index].alerted = true;
            true
        }
    }

    fn next_monster_step(&self, index: usize) -> Option<Position> {
        self.monster_hostile_targets(index)
            .first()
            .and_then(|target| self.next_monster_step_toward(index, target.position(), true))
    }

    fn next_monster_step_away(&self, index: usize) -> Option<Position> {
        const DELTAS: [(i32, i32); 8] = [
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ];

        let start = self.entities[index].position;
        let targets = self.monster_hostile_targets(index);
        let minimum_distance = |position: Position| {
            targets
                .iter()
                .map(|target| {
                    position
                        .x
                        .abs_diff(target.position().x)
                        .max(position.y.abs_diff(target.position().y))
                })
                .min()
                .unwrap_or(0)
        };
        let current_distance = minimum_distance(start);
        let movement_region = self
            .floor_regions
            .iter()
            .find(|region| region.cells.contains(&start));
        let occupied = self
            .entities
            .iter()
            .enumerate()
            .filter(|(entity_index, entity)| *entity_index != index && entity.hp > 0)
            .map(|(_, entity)| entity.position)
            .collect::<BTreeSet<_>>();
        let mut candidates = DELTAS
            .iter()
            .enumerate()
            .filter_map(|(order, (dx, dy))| {
                let position = Position {
                    x: start.x + dx,
                    y: start.y + dy,
                };
                if position == self.player.position
                    || occupied.contains(&position)
                    || !self.is_walkable(position)
                    || movement_region.is_some_and(|region| !region.cells.contains(&position))
                {
                    return None;
                }
                let distance = minimum_distance(position);
                (distance > current_distance).then_some((
                    std::cmp::Reverse(distance),
                    order,
                    position,
                ))
            })
            .collect::<Vec<_>>();
        candidates.sort();
        candidates.first().map(|(_, _, position)| *position)
    }

    fn next_surround_step(
        &self,
        index: usize,
        reservations: &mut BTreeSet<Position>,
    ) -> Option<Position> {
        const DELTAS: [(i32, i32); 8] = [
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ];

        let pack = self.entities[index].pack.as_ref()?;
        let mut surround_members = self
            .entities
            .iter()
            .filter(|entity| {
                entity.pack.as_ref().is_some_and(|candidate| {
                    candidate.id == pack.id
                        && candidate.behavior == MonsterPackBehaviorDto::Surround
                })
            })
            .map(|entity| entity.id.as_str())
            .collect::<Vec<_>>();
        surround_members.sort_unstable();
        let rank = surround_members
            .iter()
            .position(|actor_id| *actor_id == self.entities[index].id)
            .unwrap_or(0);
        let occupied = self
            .entities
            .iter()
            .enumerate()
            .filter(|(entity_index, _)| *entity_index != index)
            .map(|(_, entity)| entity.position)
            .collect::<BTreeSet<_>>();
        for offset in 0..DELTAS.len() {
            let (dx, dy) = DELTAS[(rank + offset) % DELTAS.len()];
            let target = Position {
                x: self.player.position.x + dx,
                y: self.player.position.y + dy,
            };
            if target == self.player.position
                || occupied.contains(&target)
                || reservations.contains(&target)
                || !self.is_walkable(target)
            {
                continue;
            }
            if let Some(step) = self.next_monster_step_toward(index, target, false) {
                reservations.insert(target);
                return Some(step);
            }
        }
        None
    }

    fn next_monster_step_toward(
        &self,
        index: usize,
        target: Position,
        stop_adjacent: bool,
    ) -> Option<Position> {
        const DELTAS: [(i32, i32); 8] = [
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ];

        let start = self.entities[index].position;
        let movement_region = self
            .floor_regions
            .iter()
            .find(|region| region.cells.contains(&start));
        let occupied_now = self
            .entities
            .iter()
            .enumerate()
            .filter(|(entity_index, _)| *entity_index != index)
            .map(|(_, entity)| entity.position)
            .collect::<BTreeSet<_>>();
        let moving_pack_id = self.entities[index]
            .pack
            .as_ref()
            .map(|pack| pack.id.as_str());
        let path_blockers =
            self.entities
                .iter()
                .enumerate()
                .filter(|(entity_index, entity)| {
                    *entity_index != index
                        && !entity.pack.as_ref().is_some_and(|pack| {
                            moving_pack_id.is_some_and(|moving| moving == pack.id)
                        })
                })
                .map(|(_, entity)| entity.position)
                .collect::<BTreeSet<_>>();
        let mut visited = BTreeSet::from([start]);
        let mut queue = VecDeque::new();

        let mut initial = DELTAS
            .iter()
            .enumerate()
            .map(|(order, (dx, dy))| {
                let position = Position {
                    x: start.x + dx,
                    y: start.y + dy,
                };
                (squared_distance(position, target), order, position)
            })
            .collect::<Vec<_>>();
        initial.sort();
        for (_, _, position) in initial {
            if position == self.player.position
                || occupied_now.contains(&position)
                || !self.is_walkable(position)
                || movement_region.is_some_and(|region| !region.cells.contains(&position))
                || !visited.insert(position)
            {
                continue;
            }
            if (!stop_adjacent && position == target)
                || (stop_adjacent && adjacent(position, target))
            {
                return Some(position);
            }
            queue.push_back((position, position));
        }

        while let Some((position, first_step)) = queue.pop_front() {
            let mut neighbors = DELTAS
                .iter()
                .enumerate()
                .map(|(order, (dx, dy))| {
                    let next = Position {
                        x: position.x + dx,
                        y: position.y + dy,
                    };
                    (squared_distance(next, target), order, next)
                })
                .collect::<Vec<_>>();
            neighbors.sort();
            for (_, _, next) in neighbors {
                if next == self.player.position
                    || path_blockers.contains(&next)
                    || !self.is_walkable(next)
                    || movement_region.is_some_and(|region| !region.cells.contains(&next))
                    || !visited.insert(next)
                {
                    continue;
                }
                if (!stop_adjacent && next == target) || (stop_adjacent && adjacent(next, target)) {
                    return Some(first_step);
                }
                queue.push_back((next, first_step));
            }
        }
        None
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

    fn generate_death_loot(&mut self, actor: &Actor) -> Result<Vec<ItemInstance>, CoreError> {
        let Some(table_id) = self
            .content
            .actor(&actor.kind_id)
            .and_then(|definition| definition.loot_table_id.clone())
        else {
            return Ok(Vec::new());
        };
        self.generate_loot_instances(
            &LootContext {
                table_id,
                floor_id: self.current_floor_id.clone(),
                depth: self.floor_depth(&self.current_floor_id),
                source: LootSource::MonsterDeath {
                    actor_id: actor.id.clone(),
                },
            },
            ItemLocation::Ground(actor.position),
        )
    }

    fn generate_loot_instances(
        &mut self,
        context: &LootContext,
        location: ItemLocation,
    ) -> Result<Vec<ItemInstance>, CoreError> {
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
            };
        debug_assert!(context_is_valid, "validated loot context must remain valid");
        let table = self
            .content
            .loot_table(&context.table_id)
            .expect("validated actor loot table must remain available")
            .clone();
        self.next_item_instance_serial
            .checked_add(u64::from(table.rolls))
            .ok_or(CoreError::ItemIdExhausted)?;
        let entry_weights = table
            .entries
            .iter()
            .map(|entry| entry.weight)
            .collect::<Vec<_>>();
        let quality_weights = table
            .quality_weights
            .iter()
            .map(|entry| entry.weight)
            .collect::<Vec<_>>();
        let affix_weights = table
            .affix_weights
            .iter()
            .map(|entry| entry.weight)
            .collect::<Vec<_>>();
        let mut generated = Vec::with_capacity(usize::from(table.rolls));
        for _ in 0..table.rolls {
            let entry_index = self.roll_weighted_index(&entry_weights);
            let quality_index = self.roll_weighted_index(&quality_weights);
            let affix_index = self.roll_weighted_index(&affix_weights);
            let entry = &table.entries[entry_index];
            let quality = item_quality_dto(table.quality_weights[quality_index].quality);
            let affix_ids = if quality == ItemQualityDto::Ordinary {
                Vec::new()
            } else {
                table.affix_weights[affix_index]
                    .affix_id
                    .iter()
                    .cloned()
                    .collect()
            };
            let rolled_affixes = self.roll_affix_properties(&affix_ids, context.depth);
            let (activation, charges) = initial_item_runtime_state(
                &self.content,
                &mut self.rng,
                &entry.item_kind_id,
                context.depth,
            );
            let item = ItemInstance {
                id: self.allocate_item_instance_id()?,
                kind_id: entry.item_kind_id.clone(),
                quantity: entry.quantity,
                quality,
                affix_ids,
                rolled_affixes,
                enchantments: ItemEnchantmentsDto::default(),
                curse: initial_item_curse(&self.content, &entry.item_kind_id),
                activation,
                charges,
                device_recovery_progress: 0,
                location: location.clone(),
            };
            generated.push(item);
        }
        Ok(generated)
    }

    fn roll_affix_properties(&mut self, affix_ids: &[String], depth: u16) -> Vec<RolledAffixState> {
        let mut rolled_affixes = Vec::new();
        for affix_id in affix_ids {
            let roll_groups = self
                .content
                .affix(affix_id)
                .expect("selected affix must remain available")
                .roll_groups
                .clone();
            let mut properties = AffixPropertyBundleDefinition::default();
            for group in roll_groups {
                let eligible = group
                    .candidates
                    .iter()
                    .filter(|candidate| {
                        candidate.min_depth <= depth && depth <= candidate.max_depth
                    })
                    .collect::<Vec<_>>();
                if eligible.is_empty() {
                    continue;
                }
                let weights = eligible
                    .iter()
                    .map(|candidate| candidate.weight)
                    .collect::<Vec<_>>();
                for _ in 0..group.rolls {
                    let selected = eligible[self.roll_weighted_index(&weights)];
                    merge_affix_properties(&mut properties, &selected.properties);
                }
            }
            if properties != AffixPropertyBundleDefinition::default() {
                rolled_affixes.push(RolledAffixState {
                    affix_id: affix_id.clone(),
                    properties,
                });
            }
        }
        rolled_affixes
    }

    fn roll_weighted_index(&mut self, weights: &[u32]) -> usize {
        let total = weights.iter().map(|weight| u64::from(*weight)).sum();
        let mut roll = self.rng.bounded(total);
        for (index, weight) in weights.iter().enumerate() {
            let weight = u64::from(*weight);
            if roll < weight {
                return index;
            }
            roll -= weight;
        }
        unreachable!("validated positive weighted table must select an entry")
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
    }

    fn collect_light_sources(&self) -> Vec<LightSource> {
        // The source order mirrors the original per-cell scan (player, then
        // entities, then ground items) so strict-greater comparisons keep
        // resolving ties identically.
        let mut sources = vec![LightSource {
            position: self.player.position,
            radius: PLAYER_LIGHT_RADIUS,
            maximum: 72,
            color: PLAYER_LIGHT_COLOR,
        }];
        for entity in &self.entities {
            let Some(definition) = self.content.actor(&entity.kind_id) else {
                continue;
            };
            if !definition.tags.iter().any(|tag| tag == "light-source") {
                continue;
            }
            sources.push(LightSource {
                position: entity.position,
                radius: ACTOR_LIGHT_RADIUS,
                maximum: 64,
                color: ACTOR_LIGHT_COLOR,
            });
        }
        for item in &self.items {
            let ItemLocation::Ground(item_position) = &item.location else {
                continue;
            };
            let Some(definition) = self.content.item(&item.kind_id) else {
                continue;
            };
            if !definition.tags.iter().any(|tag| tag == "light-source") {
                continue;
            }
            sources.push(LightSource {
                position: *item_position,
                radius: ITEM_LIGHT_RADIUS,
                maximum: 52,
                color: ITEM_LIGHT_COLOR,
            });
        }
        sources
    }

    fn abandon_paused_task(&mut self, task_id: &str) -> Option<Vec<Position>> {
        let world = self.content.world(&self.world_id)?;
        if self.current_floor_id != world.initial_floor_id
            || self
                .task_states
                .get(task_id)
                .is_none_or(|state| state.status != TaskStatusKindDto::Paused)
        {
            return None;
        }
        let members = world
            .procedural_floors
            .iter()
            .filter(|floor| {
                floor.lifecycle == FloorLifecycle::OneShot
                    && floor.retakeable
                    && floor_task_id(floor) == task_id
            })
            .cloned()
            .collect::<Vec<_>>();
        let initial_required = initial_task_states(world).get(task_id)?.required;
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
        let old_position = self.player.position;
        self.player.position = destination;
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
        events
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
}

fn equipment_bonuses_dto(bonuses: &EquipmentBonuses) -> EquipmentBonusesDto {
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

const fn equipment_passive_dto(passive: EquipmentPassive) -> EquipmentPassiveDto {
    match passive {
        EquipmentPassive::Regeneration => EquipmentPassiveDto::Regeneration,
        EquipmentPassive::Vampiric => EquipmentPassiveDto::Vampiric,
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

fn merge_affix_properties(
    total: &mut AffixPropertyBundleDefinition,
    addition: &AffixPropertyBundleDefinition,
) {
    merge_stat_modifiers(&mut total.modifiers, &addition.modifiers);
    merge_equipment_bonuses(&mut total.equipment_bonuses, &addition.equipment_bonuses);
    for (damage_type, level) in &addition.resistances {
        let current = total.resistances.entry(*damage_type).or_insert(*level);
        if actor_resistance_rank(*level) > actor_resistance_rank(*current) {
            *current = *level;
        }
    }
    let mut status_immunities = total
        .status_immunities
        .iter()
        .chain(&addition.status_immunities)
        .cloned()
        .collect::<BTreeSet<_>>();
    total.status_immunities = std::mem::take(&mut status_immunities).into_iter().collect();
    for (target, level) in &addition.slays {
        let current = total.slays.entry(*target).or_insert(*level);
        if *level > *current {
            *current = *level;
        }
    }
    total.brands.extend(&addition.brands);
    total.passives.extend(&addition.passives);
}

fn merge_stat_modifiers(total: &mut StatModifiers, addition: &StatModifiers) {
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
}

fn merge_equipment_bonuses(total: &mut EquipmentBonuses, addition: &EquipmentBonuses) {
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

const fn actor_resistance_rank(level: ActorResistanceLevel) -> u8 {
    match level {
        ActorResistanceLevel::Vulnerable => 0,
        ActorResistanceLevel::Resistant => 1,
        ActorResistanceLevel::Strong => 2,
        ActorResistanceLevel::Immune => 3,
    }
}

fn throw_range(weight_tenths_pound: u16) -> u16 {
    (BASE_THROW_RANGE_BUDGET / weight_tenths_pound.max(1)).clamp(MIN_THROW_RANGE, MAX_THROW_RANGE)
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
    }
}

const fn brand_damage_type(brand: WeaponBrand) -> DamageType {
    match brand {
        WeaponBrand::Acid => DamageType::Acid,
        WeaponBrand::Electricity => DamageType::Electricity,
        WeaponBrand::Fire => DamageType::Fire,
        WeaponBrand::Cold => DamageType::Cold,
        WeaponBrand::Poison => DamageType::Poison,
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
    if category == "living" {
        return !definition
            .tags
            .iter()
            .any(|tag| matches!(tag.as_str(), "demon" | "undead" | "nonliving"));
    }
    definition.tags.iter().any(|tag| tag == category)
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
        AbilityLevelScalingCurveDefinition::Prorated => {
            let level = u64::from(level.min(50));
            let amount = u64::from(scaling.multiplier);
            if level == 50 {
                amount
            } else {
                let linear_weight = 1_u64;
                let quadratic_weight = u64::from(scaling.quadratic_weight);
                let cubic_weight = u64::from(scaling.cubic_weight);
                let weight = linear_weight + quadratic_weight + cubic_weight;
                amount * level * linear_weight / (50 * weight)
                    + amount * level * level * quadratic_weight / (50 * 50 * weight)
                    + (amount * level * level / 50) * level * cubic_weight / (50 * 50 * weight)
            }
        }
    };
    let scaled = base.saturating_add(addition);
    scaling
        .maximum
        .map_or(scaled, |maximum| scaled.min(maximum))
}

fn apply_ability_level_scaling(
    effect: &mut AbilityEffectDefinition,
    scaling: &AbilityLevelScalingDefinition,
    level: u16,
) {
    match (effect, scaling.field) {
        (
            AbilityEffectDefinition::Damage { damage_dice, .. }
            | AbilityEffectDefinition::AreaDamage { damage_dice, .. }
            | AbilityEffectDefinition::BeamDamage { damage_dice, .. }
            | AbilityEffectDefinition::BoltOrBeamDamage { damage_dice, .. }
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
            | AbilityEffectDefinition::AreaDamage { damage_sides, .. }
            | AbilityEffectDefinition::BeamDamage { damage_sides, .. }
            | AbilityEffectDefinition::BoltOrBeamDamage { damage_sides, .. }
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
            | AbilityEffectDefinition::AreaDamage { damage_bonus, .. }
            | AbilityEffectDefinition::BeamDamage { damage_bonus, .. }
            | AbilityEffectDefinition::BoltOrBeamDamage { damage_bonus, .. }
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
            | AbilityEffectDefinition::ConeDamage { radius, .. }
            | AbilityEffectDefinition::BreathDamage { radius, .. },
            AbilityLevelScalingField::Radius,
        ) => {
            *radius = u8::try_from(scaled_ability_level_value(
                u64::from(*radius),
                scaling,
                level,
            ))
            .expect("validated level-scaled radius must fit u8");
        }
        (
            AbilityEffectDefinition::ApplyStatus { intensity, .. },
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
            AbilityEffectDefinition::ApplyStatus { duration_ticks, .. },
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
            AbilityEffectDefinition::ApplyStatus { duration_sides, .. },
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
                power: Some(power), ..
            },
            AbilityLevelScalingField::StatusPower,
        )
        | (
            AbilityEffectDefinition::Control { power, .. },
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
        _ => unreachable!("content validation must reject incompatible level scaling fields"),
    }
}

fn ability_effect_spec_dto(effect: &AbilityEffectDefinition) -> AbilityEffectSpecDto {
    match effect {
        AbilityEffectDefinition::BlinkSelf { radius } => {
            AbilityEffectSpecDto::BlinkSelf { radius: *radius }
        }
        AbilityEffectDefinition::TeleportSelf { minimum_distance } => {
            AbilityEffectSpecDto::TeleportSelf {
                minimum_distance: *minimum_distance,
            }
        }
        AbilityEffectDefinition::TeleportTarget => AbilityEffectSpecDto::TeleportTarget,
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
        },
        AbilityEffectDefinition::BeamDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
        } => AbilityEffectSpecDto::BeamDamage {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            damage_type: DamageType::from(*damage_type).into(),
        },
        AbilityEffectDefinition::BoltOrBeamDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            beam_chance_percent,
        } => AbilityEffectSpecDto::BoltOrBeamDamage {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            damage_type: DamageType::from(*damage_type).into(),
            beam_chance_percent: *beam_chance_percent,
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
        } => AbilityEffectSpecDto::CurseDamage {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
        },
        AbilityEffectDefinition::DeathRay { power } => {
            AbilityEffectSpecDto::DeathRay { power: *power }
        }
        AbilityEffectDefinition::TeleportAway { minimum_distance } => {
            AbilityEffectSpecDto::TeleportAway {
                minimum_distance: *minimum_distance,
            }
        }
        AbilityEffectDefinition::DrainResource { amount } => {
            AbilityEffectSpecDto::DrainResource { amount: *amount }
        }
        AbilityEffectDefinition::Amnesia => AbilityEffectSpecDto::Amnesia,
        AbilityEffectDefinition::Teleport => AbilityEffectSpecDto::Teleport,
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
        AbilityEffectDefinition::Detect {
            subject,
            category,
            radius,
            persistent,
        } => AbilityEffectSpecDto::Detect {
            subject: ability_detect_subject_dto(*subject),
            category: category.clone(),
            radius: *radius,
            persistent: *persistent,
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
        AbilityEffectDefinition::DrainLife {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            target_category,
            repeat,
        } => AbilityEffectSpecDto::DrainLife {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            damage_type: DamageType::from(*damage_type).into(),
            target_category: target_category.clone(),
            repeat: *repeat,
        },
        AbilityEffectDefinition::Genocide {
            scope,
            power,
            radius,
        } => AbilityEffectSpecDto::Genocide {
            scope: ability_genocide_scope_dto(*scope),
            power: *power,
            radius: *radius,
        },
        AbilityEffectDefinition::IdentifyItem {
            full_identify_power,
            full_identify_roll_sides,
        } => AbilityEffectSpecDto::IdentifyItem {
            full_identify_power: *full_identify_power,
            full_identify_roll_sides: *full_identify_roll_sides,
        },
        AbilityEffectDefinition::RestoreVitality { life_force } => {
            AbilityEffectSpecDto::RestoreVitality {
                life_force: *life_force,
            }
        }
        AbilityEffectDefinition::AnimateDead {
            actor_kind_id,
            corpse_item_kind_id,
            radius,
            count,
        } => AbilityEffectSpecDto::AnimateDead {
            actor_kind_id: actor_kind_id.clone(),
            corpse_item_kind_id: corpse_item_kind_id.clone(),
            radius: *radius,
            count: *count,
        },
        AbilityEffectDefinition::Heal { amount } => AbilityEffectSpecDto::Heal { amount: *amount },
        AbilityEffectDefinition::VisibleDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            target_category,
        } => AbilityEffectSpecDto::VisibleDamage {
            damage_dice: *damage_dice,
            damage_sides: *damage_sides,
            damage_bonus: *damage_bonus,
            damage_type: DamageType::from(*damage_type).into(),
            target_category: target_category.clone(),
        },
        AbilityEffectDefinition::VisibleApplyStatus {
            status_kind_id,
            intensity,
            duration_ticks,
            stacking,
            target_category,
        } => AbilityEffectSpecDto::VisibleApplyStatus {
            status_kind_id: status_kind_id.clone(),
            intensity: *intensity,
            duration_ticks: *duration_ticks,
            stacking: ability_status_stacking_dto(*stacking),
            target_category: target_category.clone(),
        },
        AbilityEffectDefinition::EnchantEquippedWeapon { affix_id } => {
            AbilityEffectSpecDto::EnchantEquippedWeapon {
                affix_id: affix_id.clone(),
            }
        }
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
        },
        AbilityEffectDefinition::NoOp { reason } => AbilityEffectSpecDto::NoOp {
            reason: reason.clone(),
        },
        AbilityEffectDefinition::Sequence { .. } => {
            unreachable!("nested ability effect sequences are rejected by content validation")
        }
    }
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
    }
}

/// Content-declared resistances are stamped whenever an entity is built from
/// its definition; loaded saves keep their stored profiles untouched.
fn stamped_spawn(mut actor: Actor, definition: &rfb_content::ActorDefinition) -> Actor {
    actor.resistances = definition_resistance_profile(definition);
    actor
}

fn actor_starts_alerted(definition: &rfb_content::ActorDefinition) -> bool {
    definition
        .awareness
        .as_ref()
        .is_none_or(|awareness| awareness.starts_alerted)
}

fn source_intensity(source: Position, target: Position, radius: i32, maximum: u8) -> u8 {
    let distance = squared_distance(source, target);
    let radius_squared = radius * radius;
    if distance > radius_squared {
        return 0;
    }
    let remaining = radius_squared - distance;
    u8::try_from(
        (u32::from(maximum) * u32::try_from(remaining).unwrap_or(0))
            / u32::try_from(radius_squared).unwrap_or(1),
    )
    .unwrap_or(maximum)
}

#[derive(Debug, Clone, Copy)]
struct LightSource {
    position: Position,
    radius: i32,
    maximum: u8,
    color: u32,
}

fn light_from_sources(sources: &[LightSource], position: Position) -> CellLightDto {
    let mut strongest = (0_u8, PLAYER_LIGHT_COLOR);
    for source in sources {
        let boost = source_intensity(source.position, position, source.radius, source.maximum);
        if boost > strongest.0 {
            strongest = (boost, source.color);
        }
    }
    CellLightDto {
        color: strongest.1,
        intensity: AMBIENT_LIGHT.saturating_add(strongest.0),
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
