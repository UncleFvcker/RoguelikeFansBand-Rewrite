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
    CastingProfileDefinition, ContentCatalog, ContentPosition, DeviceRechargeProfileDefinition,
    DungeonInstanceLifecycle, EncounterEntryDefinition, EncounterTableDefinition, EquipmentBonuses,
    EquipmentPassive, FloorLifecycle, ItemAttributeDefinition, ItemCurseSeverityDefinition,
    ItemCurseTargetDefinition, ItemEnchantmentRollDefinition, ItemSummonLevelSourceDefinition,
    ItemSummonSelectorDefinition, ItemUseEffectDefinition, MonsterPackBehavior,
    ProceduralFloorDefinition, ProceduralLayoutMode, ProceduralMazeDefinition,
    ProceduralPitDefinition, ProceduralRoomGeometryDefinition, ProceduralRoomShape,
    ProceduralStreamerCandidateDefinition, SkillKind, SlayLevel, SlayTarget,
    StartingItemDefinition, StatModifiers, TaskObjectiveKind, TechniqueAttribute,
    TechniqueProfileDefinition, TerrainFeatureEntryDefinition, ThemeVaultCandidateDefinition,
    VaultDefinition, VaultTransform, WeaponBrand,
};
#[cfg(test)]
use rfb_content::{
    DungeonEntryRequirementDefinition, DungeonEntryTaskStatus, TerrainFeaturePlacement,
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
mod damage;
mod death;
mod environment_combat;
mod floor;
mod inventory;
mod item_combat;
mod item_use;
mod monster_combat;
mod persistence;
mod player_combat;
mod progression;
mod snapshot;
mod status_effects;
mod tasks;
mod terrain;
mod validation;
mod world;

use abilities::AbilityTargetPlan;
use damage::{
    FatalityPolicy, commit_damage_application, plan_damage_application, process_actor_status_tick,
    scale_damage_outcome,
};
use environment_combat::PlayerTrapOutcome;
use floor::{
    FloorTransitionTarget, RecallUseAction, dungeon_instance_id, dungeon_instance_storage_key,
    floor_dungeon_id, parse_dungeon_instance_ordinal,
};
use inventory::{ItemKnowledgeState, ItemPropertyKnowledgeState, PickUpOutcome};
use item_use::{ItemUsePlan, SettledItemUse};
use progression::{
    build_definitions, character_skill_progress, combine_percentages, initial_character_attributes,
    initial_resource_pool, profile_resource_maximum, resolve_character_build,
};
use status_effects::{
    ability_status_stacking_dto, apply_ability_status_effect, remove_ability_status_effect,
};
use tasks::{
    CampaignState, TaskState, abandoned_task_state, floor_task_id, initial_task_states,
    task_objectives,
};
use terrain::{DoorBashOutcome, DoorOpenOutcome, TerrainDigOutcome, TrapDisarmOutcome};
use validation::{
    floor_connections_are_valid, floor_regions_are_valid, monster_packs_are_valid,
    revealed_terrain_is_valid, rolled_affixes_are_valid,
};
use world::generation::{
    allocate_generated_region_placements, apply_generated_vault_placement,
    assign_generated_footprint_to_region, assign_generated_rooms_to_regions,
    carve_generated_corridor, carve_generated_room, choose_generated_maze_position,
    formation_placement_candidates, free_vault_placement_candidates, generated_non_entry_room_id,
    generated_region_open_positions, generated_room_cells, generated_wall_positions,
    paint_generated_vault, primary_floor_connection_ids, set_generated_terrain,
    terrain_feature_placement_candidates,
};
use world::geometry::{
    floor_position_is_walkable, generated_terrain_index, maze_floor_anchors, maze_floor_path,
    transformed_vault_dimensions, transformed_vault_position,
};
#[cfg(test)]
use world::geometry::{
    generated_terrain_is_connected, maze_floor_distances, terrain_is_connectable,
    vault_entrance_outward,
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
const SPELL_EXP_BEGINNER: u16 = 900;
const SPELL_EXP_SKILLED: u16 = 1200;
const SPELL_EXP_EXPERT: u16 = 1400;
const SPELL_EXP_MASTER: u16 = 1600;
const SPELL_MANA_CONST: u64 = 2400;
const SPELL_MANA_EXPERT: u64 = 1400;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AbilityProgress {
    proficiency: u16,
    proficiency_cap: u16,
    cast_count: u32,
    fail_count: u32,
    cooldown_remaining: u16,
}

impl AbilityProgress {
    const fn new(initial: u16, cap: u16) -> Self {
        Self {
            proficiency: initial,
            proficiency_cap: cap,
            cast_count: 0,
            fail_count: 0,
            cooldown_remaining: 0,
        }
    }
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
struct GeneratedRoom {
    id: String,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    shape: ProceduralRoomShape,
}

impl GeneratedRoom {
    fn center(&self) -> Position {
        Position {
            x: self.x + self.width / 2,
            y: self.y + self.height / 2,
        }
    }

    fn contains(&self, position: Position) -> bool {
        if position.x < self.x
            || position.x >= self.x + self.width
            || position.y < self.y
            || position.y >= self.y + self.height
        {
            return false;
        }
        match self.shape {
            ProceduralRoomShape::Rectangle => true,
            ProceduralRoomShape::Cross => {
                position.x == self.center().x || position.y == self.center().y
            }
        }
    }

    fn area(&self) -> u32 {
        match self.shape {
            ProceduralRoomShape::Rectangle => (self.width * self.height) as u32,
            ProceduralRoomShape::Cross => (self.width + self.height - 1) as u32,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GeneratedVaultPlacement {
    vault: VaultDefinition,
    origin: Position,
    transform: VaultTransform,
    ordinal: u16,
    connector_cells: Vec<Position>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GeneratedVaultPlacementCandidate {
    origin: Position,
    transform: VaultTransform,
    connector_cells: Vec<Position>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GeneratedPitPlacement {
    definition: ProceduralPitDefinition,
    origin: Position,
    outer_entrance: Position,
    inner_entrance: Position,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GeneratedTerrainFeature {
    terrain_id: String,
    position: Position,
}

struct TerrainFeaturePlacementContext<'a> {
    rooms: &'a [GeneratedRoom],
    reserved: &'a BTreeSet<Position>,
    floor_terrain_id: &'a str,
    room_floor_terrain_ids: &'a BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GeneratedRegion {
    state: FloorRegionState,
    room_ids: Vec<String>,
    floor_terrain_id: String,
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

    fn casting_profile(&self) -> Option<&CastingProfileDefinition> {
        self.character_definitions()
            .and_then(|(_, _, class, _)| class.casting_profile.as_ref())
    }

    fn uses_spell_scrolls(&self) -> bool {
        self.character_definitions()
            .is_some_and(|(_, _, class, _)| class.uses_spell_scrolls)
    }

    fn device_recharge_profile(&self) -> Option<&DeviceRechargeProfileDefinition> {
        self.character_definitions()
            .and_then(|(_, _, class, _)| class.device_recharge_profile.as_ref())
    }

    fn effective_casting_ability(
        profile: &CastingProfileDefinition,
        ability: &AbilityDefinition,
    ) -> AbilityDefinition {
        let mut effective = ability.clone();
        if let Some(override_) = profile
            .ability_overrides
            .iter()
            .find(|override_| override_.ability_id == ability.id)
        {
            effective.minimum_level = override_.minimum_level;
            effective.resource_cost = override_.resource_cost;
            effective.base_failure_percent = override_.base_failure_percent;
            if !override_.level_scaling.is_empty() {
                effective.level_scaling.clone_from(&override_.level_scaling);
            }
        }
        effective
    }

    fn apply_player_level_scaling(ability: &mut AbilityDefinition, level: u16) {
        for scaling in ability.level_scaling.clone() {
            let effect = match &mut ability.effect {
                AbilityEffectDefinition::Sequence { effects } => effects
                    .get_mut(usize::from(scaling.effect_index))
                    .expect("validated level scaling effect index must remain available"),
                effect => {
                    debug_assert_eq!(scaling.effect_index, 0);
                    effect
                }
            };
            apply_ability_level_scaling(effect, &scaling, level);
        }
    }

    fn apply_casting_profile_effect_scaling(
        profile: &CastingProfileDefinition,
        ability: &mut AbilityDefinition,
        level: u16,
    ) {
        let AbilityEffectDefinition::BoltOrBeamDamage {
            beam_chance_percent,
            ..
        } = &mut ability.effect
        else {
            return;
        };
        if profile.beam_chance_level_multiplier == 0 {
            return;
        }
        let chance = i32::from(level)
            .saturating_mul(i32::from(profile.beam_chance_level_multiplier))
            .saturating_div(i32::from(profile.beam_chance_level_divisor))
            .saturating_add(i32::from(profile.beam_chance_bonus))
            .clamp(0, 100);
        *beam_chance_percent =
            u8::try_from(chance).expect("clamped casting beam chance must fit u8");
    }

    fn profile_failure_percent(
        &self,
        attribute: AttributeKind,
        minimum_failure_percent: u8,
        ability: &AbilityDefinition,
    ) -> u8 {
        let attribute_index = i32::from(self.effective_player_attributes().index(attribute));
        let level_adjustment =
            i32::from(self.progress.level.saturating_sub(ability.minimum_level)).saturating_mul(3);
        let proficiency = self.ability_progress_value(ability).proficiency;
        let proficiency_adjustment =
            i32::from(proficiency >= SPELL_EXP_EXPERT) + i32::from(proficiency >= SPELL_EXP_MASTER);
        let chance = i32::from(ability.base_failure_percent)
            .saturating_sub(level_adjustment)
            .saturating_sub(attribute_index)
            .saturating_sub(proficiency_adjustment)
            .clamp(i32::from(minimum_failure_percent), 95);
        u8::try_from(chance).expect("validated ability failure chance must fit u8")
    }

    fn casting_attribute_kind(attribute: CastingAttribute) -> AttributeKind {
        match attribute {
            CastingAttribute::Intelligence => AttributeKind::Intelligence,
            CastingAttribute::Wisdom => AttributeKind::Wisdom,
            CastingAttribute::Charisma => AttributeKind::Charisma,
        }
    }

    fn casting_resource_maximum(&self, profile: &CastingProfileDefinition) -> u32 {
        profile_resource_maximum(
            self.progress.level,
            self.effective_player_attributes()
                .index(Self::casting_attribute_kind(profile.casting_attribute)),
            (
                profile.base_capacity,
                profile.capacity_per_level,
                profile.capacity_per_attribute_index,
            ),
        )
    }

    fn technique_profiles(&self) -> &[TechniqueProfileDefinition] {
        self.character_definitions()
            .map(|(_, _, class, _)| class.technique_profiles.as_slice())
            .unwrap_or_default()
    }

    fn technique_attribute_kind(attribute: TechniqueAttribute) -> AttributeKind {
        match attribute {
            TechniqueAttribute::Strength => AttributeKind::Strength,
            TechniqueAttribute::Intelligence => AttributeKind::Intelligence,
            TechniqueAttribute::Wisdom => AttributeKind::Wisdom,
            TechniqueAttribute::Dexterity => AttributeKind::Dexterity,
            TechniqueAttribute::Constitution => AttributeKind::Constitution,
            TechniqueAttribute::Charisma => AttributeKind::Charisma,
        }
    }

    fn item_attribute_kind(attribute: &ItemAttributeDefinition) -> AttributeKind {
        match attribute {
            ItemAttributeDefinition::Strength => AttributeKind::Strength,
            ItemAttributeDefinition::Intelligence => AttributeKind::Intelligence,
            ItemAttributeDefinition::Wisdom => AttributeKind::Wisdom,
            ItemAttributeDefinition::Dexterity => AttributeKind::Dexterity,
            ItemAttributeDefinition::Constitution => AttributeKind::Constitution,
            ItemAttributeDefinition::Charisma => AttributeKind::Charisma,
        }
    }

    fn technique_resource_maximum(&self, profile: &TechniqueProfileDefinition) -> u32 {
        profile_resource_maximum(
            self.progress.level,
            self.effective_player_attributes()
                .index(Self::technique_attribute_kind(profile.governing_attribute)),
            (
                profile.base_capacity,
                profile.capacity_per_level,
                profile.capacity_per_attribute_index,
            ),
        )
    }

    fn device_recharge_resource_maximum(&self, profile: &DeviceRechargeProfileDefinition) -> u32 {
        profile_resource_maximum(
            self.progress.level,
            self.effective_player_attributes()
                .index(Self::technique_attribute_kind(profile.governing_attribute)),
            (
                profile.base_capacity,
                profile.capacity_per_level,
                profile.capacity_per_attribute_index,
            ),
        )
    }

    fn technique_profile_for_ability(
        &self,
        ability: &AbilityDefinition,
    ) -> Option<&TechniqueProfileDefinition> {
        self.technique_profiles().iter().find(|profile| {
            profile.resource_id == ability.resource_id
                && profile
                    .innate_ability_ids
                    .iter()
                    .any(|ability_id| ability_id == &ability.id)
        })
    }

    fn technique_failure_percent(
        &self,
        profile: &TechniqueProfileDefinition,
        ability: &AbilityDefinition,
    ) -> u8 {
        self.profile_failure_percent(
            Self::technique_attribute_kind(profile.governing_attribute),
            profile.minimum_failure_percent,
            ability,
        )
    }

    fn ability_learning_capacity(&self, profile: &CastingProfileDefinition) -> u16 {
        let attribute_index = u32::from(
            self.effective_player_attributes()
                .index(Self::casting_attribute_kind(profile.casting_attribute)),
        );
        let level_bonus = u32::from(profile.learning_capacity_per_level)
            .saturating_mul(u32::from(self.progress.level.saturating_sub(1)));
        let attribute_bonus = u32::from(profile.learning_capacity_per_attribute_index)
            .saturating_mul(attribute_index);
        let raw = u32::from(profile.base_learning_capacity)
            .saturating_add(level_bonus)
            .saturating_add(attribute_bonus);
        (raw.min(u32::from(profile.learning_capacity_cap)) as u16)
            .saturating_add(self.bonus_spell_learning_capacity)
    }

    /// Single source for "which resource pools and abilities does the
    /// current build grant": initialization, level-up refresh, and load-time
    /// validation must all agree on this derivation.
    fn player_ability_baseline(&self) -> (BTreeMap<String, u32>, BTreeSet<String>) {
        let mut pool_maxima = BTreeMap::new();
        let mut ability_ids = BTreeSet::new();
        if let Some(profile) = self.casting_profile() {
            pool_maxima.insert(
                profile.resource_id.clone(),
                self.casting_resource_maximum(profile),
            );
            ability_ids.extend(
                profile
                    .ability_book_ids
                    .iter()
                    .filter_map(|book_id| self.content.ability_book(book_id))
                    .flat_map(|book| book.ability_ids.iter().cloned()),
            );
        }
        for profile in self.technique_profiles() {
            pool_maxima.insert(
                profile.resource_id.clone(),
                self.technique_resource_maximum(profile),
            );
            ability_ids.extend(profile.innate_ability_ids.iter().cloned());
        }
        if let Some(profile) = self.device_recharge_profile() {
            pool_maxima.insert(
                profile.resource_id.clone(),
                self.device_recharge_resource_maximum(profile),
            );
        }
        (pool_maxima, ability_ids)
    }

    fn initialize_player_ability_state(&mut self) {
        self.resources.clear();
        self.learned_abilities.clear();
        self.ability_progress.clear();
        let (pool_maxima, ability_ids) = self.player_ability_baseline();
        for (resource_id, maximum) in pool_maxima {
            let pool = initial_resource_pool(&self.content, &resource_id, maximum);
            self.resources.insert(resource_id, pool);
        }
        for ability_id in ability_ids {
            if let Some(ability) = self.content.ability(&ability_id) {
                self.ability_progress.insert(
                    ability_id,
                    AbilityProgress::new(ability.proficiency.initial, ability.proficiency.cap),
                );
            }
        }
    }

    fn restore_player_ability_state(
        &mut self,
        saved_resources: Vec<ResourcePoolSaveDto>,
        saved_learned_ability_ids: Vec<String>,
        saved_ability_progress: Vec<AbilityProgressSaveDto>,
    ) -> Result<(), CoreError> {
        self.initialize_player_ability_state();
        // Saved pools may be a subset of the initialized set: legacy saves
        // created before a class gained a new resource keep their recorded
        // pools and the missing ones stay at their content-defined initial
        // fill without drawing RNG.
        let mut seen = BTreeSet::new();
        for saved in saved_resources {
            let Some(pool) = self.resources.get_mut(&saved.id) else {
                return Err(CoreError::InvalidSave("player resource ID is invalid"));
            };
            if !seen.insert(saved.id)
                || saved.maximum != pool.maximum
                || saved.current > saved.maximum
            {
                return Err(CoreError::InvalidSave("player resource pool is invalid"));
            }
            pool.current = saved.current;
        }

        let casting_profile = self.casting_profile().cloned();
        if casting_profile.is_none() && !saved_learned_ability_ids.is_empty() {
            return Err(CoreError::InvalidSave(
                "non-caster cannot have learned abilities",
            ));
        }
        if let Some(profile) = &casting_profile {
            let learning_capacity = usize::from(self.ability_learning_capacity(profile));
            if saved_learned_ability_ids.len() > learning_capacity {
                return Err(CoreError::InvalidSave(
                    "learned ability set exceeds learning capacity",
                ));
            }
            for ability_id in saved_learned_ability_ids {
                let Some(ability) = self.content.ability(&ability_id) else {
                    return Err(CoreError::InvalidSave("learned ability ID is invalid"));
                };
                let ability = Self::effective_casting_ability(profile, ability);
                if ability.minimum_level > self.progress.level
                    || !self.profile_supports_ability(profile, &ability_id)
                    || !self.learned_abilities.insert(ability_id)
                {
                    return Err(CoreError::InvalidSave("learned ability set is invalid"));
                }
            }
        }
        let mut seen_progress = BTreeSet::new();
        for saved in saved_ability_progress {
            if !seen_progress.insert(saved.id.clone()) {
                return Err(CoreError::InvalidSave("ability progress set is invalid"));
            }
            let cooldown_turns = self.ability_cooldown_turns(&saved.id);
            let Some(progress) = self.ability_progress.get_mut(&saved.id) else {
                return Err(CoreError::InvalidSave("ability progress ID is invalid"));
            };
            if saved.proficiency_cap != progress.proficiency_cap
                || saved.proficiency > saved.proficiency_cap
                || saved.cooldown_remaining > cooldown_turns
            {
                return Err(CoreError::InvalidSave(
                    "ability progress values are invalid",
                ));
            }
            progress.proficiency = saved.proficiency;
            progress.cast_count = saved.cast_count;
            progress.fail_count = saved.fail_count;
            progress.cooldown_remaining = saved.cooldown_remaining;
        }
        Ok(())
    }

    fn refresh_player_resource_maxima(&mut self) {
        let (pool_maxima, _) = self.player_ability_baseline();
        for (resource_id, maximum) in &pool_maxima {
            let initial = initial_resource_pool(&self.content, resource_id, *maximum);
            let pool = self.resources.entry(resource_id.clone()).or_insert(initial);
            pool.maximum = *maximum;
            pool.current = pool.current.min(*maximum);
        }
        self.resources.retain(|id, _| pool_maxima.contains_key(id));
    }

    fn profile_supports_ability(
        &self,
        profile: &CastingProfileDefinition,
        ability_id: &str,
    ) -> bool {
        profile.ability_book_ids.iter().any(|book_id| {
            self.content
                .ability_book(book_id)
                .is_some_and(|book| book.ability_ids.iter().any(|id| id == ability_id))
        })
    }

    fn ability_book_item_id(
        &self,
        profile: &CastingProfileDefinition,
        ability_id: &str,
    ) -> Option<String> {
        self.items
            .iter()
            .filter(|item| item.location == ItemLocation::Inventory)
            .filter_map(|item| {
                let book_id = self
                    .content
                    .item(&item.kind_id)?
                    .ability_book_id
                    .as_deref()?;
                if !profile.ability_book_ids.iter().any(|id| id == book_id)
                    || !self
                        .content
                        .ability_book(book_id)
                        .is_some_and(|book| book.ability_ids.iter().any(|id| id == ability_id))
                {
                    return None;
                }
                Some(item.id.clone())
            })
            .min()
    }

    fn ability_progress_value(&self, ability: &AbilityDefinition) -> AbilityProgress {
        self.ability_progress
            .get(&ability.id)
            .copied()
            .unwrap_or_else(|| {
                AbilityProgress::new(ability.proficiency.initial, ability.proficiency.cap)
            })
    }

    fn ability_proficiency_rank(proficiency: u16) -> AbilityProficiencyRankDto {
        if proficiency < SPELL_EXP_BEGINNER {
            AbilityProficiencyRankDto::Unskilled
        } else if proficiency < SPELL_EXP_SKILLED {
            AbilityProficiencyRankDto::Beginner
        } else if proficiency < SPELL_EXP_EXPERT {
            AbilityProficiencyRankDto::Skilled
        } else if proficiency < SPELL_EXP_MASTER {
            AbilityProficiencyRankDto::Expert
        } else {
            AbilityProficiencyRankDto::Master
        }
    }

    fn ability_effective_resource_cost(
        &self,
        ability: &AbilityDefinition,
        progress: AbilityProgress,
    ) -> u32 {
        let proficiency = u64::from(progress.proficiency.min(SPELL_EXP_MASTER));
        let factor = SPELL_MANA_CONST
            .saturating_add(SPELL_MANA_EXPERT)
            .saturating_sub(proficiency);
        let numerator = u64::from(ability.resource_cost)
            .saturating_mul(factor)
            .saturating_add(SPELL_MANA_CONST.saturating_sub(1));
        u32::try_from((numerator / SPELL_MANA_CONST).max(1))
            .expect("validated ability mana cost must fit u32")
    }

    fn ability_cooldown_turns(&self, ability_id: &str) -> u16 {
        let Some(ability) = self.content.ability(ability_id) else {
            return 0;
        };
        let Some(cooldown) = ability.cooldown.as_ref() else {
            return 0;
        };
        let Some(group_id) = cooldown.group_id.as_deref() else {
            return cooldown.turns;
        };
        self.content
            .abilities()
            .filter_map(|candidate| {
                candidate.cooldown.as_ref().and_then(|candidate_cooldown| {
                    (candidate_cooldown.group_id.as_deref() == Some(group_id))
                        .then_some(candidate_cooldown.turns)
                })
            })
            .max()
            .unwrap_or(cooldown.turns)
    }

    fn ability_cooldown_remaining(&self, ability: &AbilityDefinition) -> u16 {
        let Some(cooldown) = ability.cooldown.as_ref() else {
            return 0;
        };
        if let Some(group_id) = cooldown.group_id.as_deref() {
            self.content
                .abilities()
                .filter(|candidate| {
                    candidate
                        .cooldown
                        .as_ref()
                        .and_then(|value| value.group_id.as_deref())
                        == Some(group_id)
                })
                .filter_map(|candidate| self.ability_progress.get(&candidate.id))
                .map(|progress| progress.cooldown_remaining)
                .max()
                .unwrap_or(0)
        } else {
            self.ability_progress
                .get(&ability.id)
                .map_or(0, |progress| progress.cooldown_remaining)
        }
    }

    fn decrement_ability_cooldowns(&mut self, turns: u16) {
        if turns == 0 {
            return;
        }
        for progress in self.ability_progress.values_mut() {
            progress.cooldown_remaining = progress.cooldown_remaining.saturating_sub(turns);
        }
    }

    fn record_ability_cast(
        &mut self,
        ability: &AbilityDefinition,
        succeeded: bool,
    ) -> AbilityProgress {
        let progress = self
            .ability_progress
            .entry(ability.id.clone())
            .or_insert_with(|| {
                AbilityProgress::new(ability.proficiency.initial, ability.proficiency.cap)
            });
        if succeeded {
            progress.cast_count = progress.cast_count.saturating_add(1);
            progress.proficiency = progress
                .proficiency
                .saturating_add(ability.proficiency.success_gain)
                .min(progress.proficiency_cap);
        } else {
            progress.fail_count = progress.fail_count.saturating_add(1);
            progress.proficiency = progress
                .proficiency
                .saturating_add(ability.proficiency.failure_gain)
                .min(progress.proficiency_cap);
        }
        if succeeded && let Some(cooldown) = ability.cooldown.as_ref() {
            if let Some(group_id) = cooldown.group_id.as_deref() {
                let group_ids = self
                    .content
                    .abilities()
                    .filter(|candidate| {
                        candidate
                            .cooldown
                            .as_ref()
                            .and_then(|value| value.group_id.as_deref())
                            == Some(group_id)
                    })
                    .map(|candidate| candidate.id.clone())
                    .collect::<Vec<_>>();
                for id in group_ids {
                    if let Some(member) = self.ability_progress.get_mut(&id) {
                        member.cooldown_remaining = cooldown.turns;
                    }
                }
            } else {
                progress.cooldown_remaining = cooldown.turns;
            }
        }
        self.ability_progress
            .get(&ability.id)
            .copied()
            .expect("ability progress must remain available")
    }

    fn ability_failure_percent(
        &self,
        profile: &CastingProfileDefinition,
        ability: &AbilityDefinition,
    ) -> u8 {
        self.profile_failure_percent(
            Self::casting_attribute_kind(profile.casting_attribute),
            profile.minimum_failure_percent,
            ability,
        )
    }

    fn recover_player_resources(&mut self, resting: bool) -> Vec<ResourceRecoveryResolutionDto> {
        let recovery_amounts = self
            .resources
            .keys()
            .map(|id| {
                let definition = self
                    .content
                    .resource(id)
                    .expect("player resource definition must remain available");
                let amount = if resting {
                    definition.rest_recovery_amount
                } else {
                    definition.wait_recovery_amount
                };
                (id.clone(), amount)
            })
            .collect::<BTreeMap<_, _>>();
        let mut recovered = Vec::new();
        for (id, pool) in &mut self.resources {
            let before = pool.current;
            pool.current = pool
                .current
                .saturating_add(recovery_amounts[id])
                .min(pool.maximum);
            if pool.current > before {
                self.resources_touched.insert(id.clone());
                recovered.push(ResourceRecoveryResolutionDto {
                    resource_id: id.clone(),
                    before,
                    after: pool.current,
                    recovered: pool.current - before,
                });
            }
        }
        recovered
    }

    fn decay_player_resources(&mut self) {
        let resource_ids = self.resources.keys().cloned().collect::<Vec<_>>();
        for resource_id in resource_ids {
            if self.resources_touched.contains(&resource_id) {
                continue;
            }
            let decay = self
                .content
                .resource(&resource_id)
                .map_or(0, |definition| definition.turn_decay_amount);
            if decay == 0 {
                continue;
            }
            let pool = self
                .resources
                .get_mut(&resource_id)
                .expect("player resource pool must remain available");
            pool.current = pool.current.saturating_sub(decay);
        }
    }

    fn player_has_depleted_recoverable_resource(&self, resting: bool) -> bool {
        self.resources.iter().any(|(id, pool)| {
            if pool.current >= pool.maximum {
                return false;
            }
            self.content.resource(id).is_some_and(|definition| {
                if resting {
                    definition.rest_recovery_amount > 0
                } else {
                    definition.wait_recovery_amount > 0
                }
            })
        })
    }

    fn visible_hostile_exists(&self) -> bool {
        self.entities.iter().any(|entity| {
            entity.hp > 0
                && !self.actor_is_player_aligned(entity)
                && self.is_visible(entity.position)
        })
    }

    fn resolve_player_rest(
        &mut self,
        requested_turns: u16,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<RestResolutionDto, CoreError> {
        let resource_before = self
            .resources
            .iter()
            .map(|(id, pool)| (id.clone(), pool.current))
            .collect::<BTreeMap<_, _>>();
        let mut completed_turns = 0_u16;
        let stop_reason = if requested_turns == 0 || requested_turns > MAX_REST_TURNS {
            RestStopReasonDto::InvalidTurns
        } else if !self.player_has_depleted_recoverable_resource(true) {
            RestStopReasonDto::FullResources
        } else if self.visible_hostile_exists() {
            RestStopReasonDto::EnemyVisible
        } else {
            loop {
                let hp_before = self.player.hp;
                spend_energy(&mut self.player.energy_need, STANDARD_ACTION_COST);
                self.advance_until_player_ready(events, changed, removed_entities)?;
                completed_turns = completed_turns.saturating_add(1);
                if self.player_is_dead() {
                    break RestStopReasonDto::PlayerDied;
                }
                if self.player.hp < hp_before {
                    break RestStopReasonDto::Damaged;
                }
                if self.visible_hostile_exists() {
                    break RestStopReasonDto::EnemyVisible;
                }
                self.recover_player_resources(true);
                self.decay_player_resources();
                if !self.player_has_depleted_recoverable_resource(true) {
                    break RestStopReasonDto::FullResources;
                }
                if completed_turns >= requested_turns {
                    break RestStopReasonDto::TurnLimit;
                }
            }
        };
        let resource_recoveries = self
            .resources
            .iter()
            .filter_map(|(id, pool)| {
                let before = resource_before.get(id).copied().unwrap_or(pool.current);
                (pool.current > before).then(|| ResourceRecoveryResolutionDto {
                    resource_id: id.clone(),
                    before,
                    after: pool.current,
                    recovered: pool.current - before,
                })
            })
            .collect();
        Ok(RestResolutionDto {
            requested_turns,
            completed_turns,
            stop_reason,
            resource_recoveries,
        })
    }

    fn item_base_modifiers(&self, kind_id: &str) -> StatModifiersDto {
        self.content
            .item(kind_id)
            .map_or_else(StatModifiersDto::default, |definition| StatModifiersDto {
                attack: definition.modifiers.attack,
                defense: definition.modifiers.defense,
                max_hp: definition.modifiers.max_hp,
                strength: definition.modifiers.strength,
                intelligence: definition.modifiers.intelligence,
                wisdom: definition.modifiers.wisdom,
                dexterity: definition.modifiers.dexterity,
                constitution: definition.modifiers.constitution,
                charisma: definition.modifiers.charisma,
                speed: definition.modifiers.speed,
            })
    }

    /// Combines resistance tiers from every defensive source the player
    /// carries: the actor's own profile, the build's race, and each equipped
    /// item plus its affixes. Deterministic merge: immune anywhere wins, then
    /// strong; a resistant source is cancelled back to normal by any
    /// vulnerable source; lone vulnerability stays vulnerable.
    fn effective_player_resistances(&self) -> ResistanceProfile {
        let mut sources: BTreeMap<DamageType, (bool, bool, bool, bool)> = BTreeMap::new();
        let mut record = |damage_type: DamageType, level: ResistanceLevel| {
            let entry = sources.entry(damage_type).or_default();
            match level {
                ResistanceLevel::Immune => entry.0 = true,
                ResistanceLevel::Strong => entry.1 = true,
                ResistanceLevel::Resistant => entry.2 = true,
                ResistanceLevel::Vulnerable => entry.3 = true,
                ResistanceLevel::Normal => {}
            }
        };
        for (damage_type, level) in self.player.resistances.iter() {
            record(damage_type, level);
        }
        for status in &self.player.statuses {
            for (damage_type, level) in &status.granted_resistances {
                record(*damage_type, *level);
            }
        }
        if let Some((_, race, _, _)) = self.character_definitions() {
            for (damage_type, level) in &race.resistances {
                record(
                    DamageType::from(*damage_type),
                    ResistanceLevel::from(*level),
                );
            }
        }
        for item in &self.items {
            if !matches!(&item.location, ItemLocation::Equipped { .. }) {
                continue;
            }
            if let Some(definition) = self.content.item(&item.kind_id) {
                for (damage_type, level) in &definition.resistances {
                    record(
                        DamageType::from(*damage_type),
                        ResistanceLevel::from(*level),
                    );
                }
            }
            for affix_id in &item.affix_ids {
                if let Some(affix) = self.content.affix(affix_id) {
                    for (damage_type, level) in &affix.resistances {
                        record(
                            DamageType::from(*damage_type),
                            ResistanceLevel::from(*level),
                        );
                    }
                }
            }
            for rolled in &item.rolled_affixes {
                for (damage_type, level) in &rolled.properties.resistances {
                    record(
                        DamageType::from(*damage_type),
                        ResistanceLevel::from(*level),
                    );
                }
            }
        }
        let mut profile = ResistanceProfile::default();
        for (damage_type, (immune, strong, resistant, vulnerable)) in sources {
            let level = if immune {
                ResistanceLevel::Immune
            } else if strong {
                ResistanceLevel::Strong
            } else if resistant {
                if vulnerable {
                    ResistanceLevel::Normal
                } else {
                    ResistanceLevel::Resistant
                }
            } else if vulnerable {
                ResistanceLevel::Vulnerable
            } else {
                ResistanceLevel::Normal
            };
            profile.set(damage_type, level);
        }
        profile
    }

    fn player_can_pass_walls(&self) -> bool {
        self.player
            .statuses
            .iter()
            .any(|status| status.grants_wall_passage)
    }

    fn player_incoming_damage_percent(&self) -> u8 {
        self.player
            .statuses
            .iter()
            .map(|status| status.incoming_damage_percent)
            .min()
            .unwrap_or(100)
    }

    fn reduce_player_damage(&self, damage: DamageOutcome) -> DamageOutcome {
        scale_damage_outcome(damage, self.player_incoming_damage_percent())
    }

    /// Status kinds the player cannot receive: the union of the race's
    /// innate immunities and every equipped item's (plus affixes').
    fn player_status_immunities(&self) -> BTreeSet<String> {
        let mut immunities = BTreeSet::new();
        for status in &self.player.statuses {
            immunities.extend(status.granted_status_immunities.iter().cloned());
        }
        if let Some((_, race, _, _)) = self.character_definitions() {
            immunities.extend(race.status_immunities.iter().cloned());
        }
        for item in &self.items {
            if !matches!(&item.location, ItemLocation::Equipped { .. }) {
                continue;
            }
            if let Some(definition) = self.content.item(&item.kind_id) {
                immunities.extend(definition.status_immunities.iter().cloned());
            }
            for affix_id in &item.affix_ids {
                if let Some(affix) = self.content.affix(affix_id) {
                    immunities.extend(affix.status_immunities.iter().cloned());
                }
            }
            for rolled in &item.rolled_affixes {
                immunities.extend(rolled.properties.status_immunities.iter().cloned());
            }
        }
        immunities
    }

    fn item_modifiers(&self, item: &ItemInstance) -> StatModifiersDto {
        let mut modifiers = item.affix_ids.iter().fold(
            self.item_base_modifiers(&item.kind_id),
            |total, affix_id| {
                let affix = self
                    .content
                    .affix(affix_id)
                    .expect("item affix must remain available");
                StatModifiersDto {
                    attack: total.attack.saturating_add(affix.modifiers.attack),
                    defense: total.defense.saturating_add(affix.modifiers.defense),
                    max_hp: total.max_hp.saturating_add(affix.modifiers.max_hp),
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
            add_stat_modifiers_dto(&mut modifiers, &rolled.properties.modifiers);
        }
        modifiers.defense = modifiers
            .defense
            .saturating_add(i32::from(item.enchantments.to_armor));
        modifiers
    }

    fn item_equipment_bonuses(&self, item: &ItemInstance) -> EquipmentBonuses {
        let mut bonuses = self
            .content
            .item(&item.kind_id)
            .map_or_else(EquipmentBonuses::default, |definition| {
                definition.equipment_bonuses.clone()
            });
        for affix_id in &item.affix_ids {
            if let Some(affix) = self.content.affix(affix_id) {
                merge_equipment_bonuses(&mut bonuses, &affix.equipment_bonuses);
            }
        }
        for rolled in &item.rolled_affixes {
            merge_equipment_bonuses(&mut bonuses, &rolled.properties.equipment_bonuses);
        }
        bonuses
    }

    fn item_passives(&self, item: &ItemInstance) -> BTreeSet<EquipmentPassive> {
        let mut passives = self
            .content
            .item(&item.kind_id)
            .map_or_else(BTreeSet::new, |definition| definition.passives.clone());
        for affix_id in &item.affix_ids {
            if let Some(affix) = self.content.affix(affix_id) {
                passives.extend(&affix.passives);
            }
        }
        for rolled in &item.rolled_affixes {
            passives.extend(&rolled.properties.passives);
        }
        passives
    }

    fn player_equipment_passives(&self) -> BTreeSet<EquipmentPassive> {
        self.items
            .iter()
            .filter(|item| matches!(&item.location, ItemLocation::Equipped { .. }))
            .flat_map(|item| self.item_passives(item))
            .collect()
    }

    fn visible_item_modifiers(&self, item: &ItemInstance) -> StatModifiersDto {
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

    fn visible_item_equipment_bonuses(&self, item: &ItemInstance) -> EquipmentBonusesDto {
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

    fn visible_item_passives(&self, item: &ItemInstance) -> Vec<EquipmentPassiveDto> {
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

    fn known_item_properties(&self, item: &ItemInstance) -> Vec<ItemPropertyDto> {
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

    fn item_identification(&self, item: &ItemInstance) -> ItemIdentificationDto {
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

    fn visible_item_quality(&self, item: &ItemInstance) -> Option<ItemQualityDto> {
        (self.item_identification(item) != ItemIdentificationDto::Unexamined)
            .then_some(item.quality)
    }

    fn visible_item_curse(&self, item: &ItemInstance) -> Option<ItemCurseSeverityDto> {
        (self.item_identification(item) != ItemIdentificationDto::Unexamined)
            .then_some(item.curse)
            .flatten()
    }

    fn visible_item_melee_profile(&self, item: &ItemInstance) -> Option<AttackProfileDto> {
        (self.item_knowledge_dto(&item.kind_id) == ItemKnowledgeDto::Aware)
            .then(|| self.item_melee_profile(item))
            .flatten()
    }

    /// Item resistance tiers visible to the player: the base definition is
    /// gated by kind awareness, affix contributions by per-instance affix
    /// knowledge.
    fn visible_item_resistances(&self, item: &ItemInstance) -> Vec<ResistanceDto> {
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

    fn visible_item_status_immunities(&self, item: &ItemInstance) -> Vec<String> {
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

    fn visible_item_slays(&self, item: &ItemInstance) -> Vec<SlayDto> {
        self.visible_item_offense(item)
            .0
            .into_iter()
            .map(|(target, level)| SlayDto {
                target: slay_target_dto(target),
                level: slay_level_dto(level),
            })
            .collect()
    }

    fn visible_item_brands(&self, item: &ItemInstance) -> Vec<WeaponBrandDto> {
        self.visible_item_offense(item)
            .1
            .into_iter()
            .map(weapon_brand_dto)
            .collect()
    }

    fn visible_item_projectile_profile(&self, item: &ItemInstance) -> Option<ProjectileProfileDto> {
        (self.item_knowledge_dto(&item.kind_id) == ItemKnowledgeDto::Aware)
            .then(|| self.item_projectile_profile(item))
            .flatten()
    }

    fn visible_item_throw_profile(&self, item: &ItemInstance) -> Option<ThrowProfileDto> {
        (self.item_knowledge_dto(&item.kind_id) == ItemKnowledgeDto::Aware)
            .then(|| self.item_throw_profile(item))
            .flatten()
    }

    fn equipment_modifiers(&self) -> StatModifiersDto {
        self.items
            .iter()
            .filter(|item| matches!(&item.location, ItemLocation::Equipped { .. }))
            .fold(StatModifiersDto::default(), |total, item| {
                let item = self.item_modifiers(item);
                StatModifiersDto {
                    attack: total.attack.saturating_add(item.attack),
                    defense: total.defense.saturating_add(item.defense),
                    max_hp: total.max_hp.saturating_add(item.max_hp),
                    strength: total.strength.saturating_add(item.strength),
                    intelligence: total.intelligence.saturating_add(item.intelligence),
                    wisdom: total.wisdom.saturating_add(item.wisdom),
                    dexterity: total.dexterity.saturating_add(item.dexterity),
                    constitution: total.constitution.saturating_add(item.constitution),
                    charisma: total.charisma.saturating_add(item.charisma),
                    speed: total.speed.saturating_add(item.speed),
                }
            })
    }

    fn victory_level_cap_unlocked(&self) -> bool {
        self.campaign_state.status != CampaignStatusDto::Active
    }

    fn effective_player_max_hp(&self) -> i32 {
        self.player_derived_stats().max_hp.value
    }

    fn player_derived_stats(&self) -> ActorDerivedStats {
        let definition = self
            .content
            .actor(&self.player.kind_id)
            .expect("player actor definition must remain available");
        self.actor_derived_stats(&self.player, definition, true)
    }

    fn item_melee_profile(&self, item: &ItemInstance) -> Option<AttackProfileDto> {
        self.content
            .item(&item.kind_id)
            .and_then(|definition| definition.melee_profile.as_ref())
            .map(|profile| AttackProfileDto {
                attacks: profile.attacks,
                to_hit: profile
                    .to_hit
                    .saturating_add(i32::from(item.enchantments.to_hit)),
                to_damage: profile
                    .to_damage
                    .saturating_add(i32::from(item.enchantments.to_damage)),
                damage: DamageDiceDto {
                    dice: profile.damage_dice,
                    sides: profile.damage_sides,
                    damage_type: DamageType::from(profile.damage_type).into(),
                },
                source_item_id: Some(item.id.clone()),
            })
    }

    fn item_projectile_profile(&self, item: &ItemInstance) -> Option<ProjectileProfileDto> {
        self.content
            .item(&item.kind_id)
            .and_then(|definition| definition.projectile_profile.as_ref())
            .map(|profile| ProjectileProfileDto {
                range: profile.range,
                to_hit: profile
                    .to_hit
                    .saturating_add(i32::from(item.enchantments.to_hit)),
                to_damage: profile
                    .to_damage
                    .saturating_add(i32::from(item.enchantments.to_damage)),
                damage: DamageDiceDto {
                    dice: profile.damage_dice,
                    sides: profile.damage_sides,
                    damage_type: DamageType::from(profile.damage_type).into(),
                },
                ammo_kind_id: profile.ammo_kind_id.clone(),
                target_spec: projectile_target_spec(profile.range),
                source_item_id: item.id.clone(),
            })
    }

    fn item_weight_tenths_pound(&self, kind_id: &str) -> u16 {
        self.content
            .item(kind_id)
            .map_or(0, |definition| definition.weight_tenths_pound)
    }

    fn carried_weight_tenths_pound(&self) -> u32 {
        self.items
            .iter()
            .filter(|item| {
                matches!(
                    item.location,
                    ItemLocation::Inventory | ItemLocation::Equipped { .. }
                )
            })
            .fold(0_u32, |total, item| {
                total.saturating_add(
                    u32::from(self.item_weight_tenths_pound(&item.kind_id))
                        .saturating_mul(item.quantity),
                )
            })
    }

    fn item_throw_profile(&self, item: &ItemInstance) -> Option<ThrowProfileDto> {
        let definition = self.content.item(&item.kind_id)?;
        definition
            .throw_profile
            .as_ref()
            .map(|profile| ThrowProfileDto {
                range: throw_range(definition.weight_tenths_pound),
                to_hit: profile
                    .to_hit
                    .saturating_add(i32::from(item.enchantments.to_hit)),
                to_damage: profile
                    .to_damage
                    .saturating_add(i32::from(item.enchantments.to_damage)),
                damage: DamageDiceDto {
                    dice: profile.damage_dice,
                    sides: profile.damage_sides,
                    damage_type: DamageType::from(profile.damage_type).into(),
                },
                source_item_id: item.id.clone(),
            })
    }

    fn body_slot_type(&self, slot_id: &str) -> Option<&str> {
        self.body_slots
            .iter()
            .find(|slot| slot.id == slot_id)
            .map(|slot| slot.slot_type.as_str())
    }

    fn player_projectile_profile(&self) -> Option<ResolvedProjectileProfile> {
        self.items.iter().find_map(|item| {
            let ItemLocation::Equipped { slot_id } = &item.location else {
                return None;
            };
            if self.body_slot_type(slot_id) != Some("launcher") {
                return None;
            }
            self.content
                .item(&item.kind_id)?
                .projectile_profile
                .as_ref()
                .and_then(|profile| {
                    let ammo_definition = self.content.item(&profile.ammo_kind_id)?;
                    let ammo_break_chance_percent = ammo_definition.break_chance_percent;
                    let ammunition_enchantments = self
                        .items
                        .iter()
                        .filter(|ammunition| {
                            ammunition.kind_id == profile.ammo_kind_id
                                && ammunition.location == ItemLocation::Inventory
                                && ammunition.quantity > 0
                        })
                        .min_by(|left, right| left.id.cmp(&right.id))
                        .map_or_else(ItemEnchantmentsDto::default, |ammunition| {
                            ammunition.enchantments
                        });
                    Some(ResolvedProjectileProfile {
                        range: profile.range,
                        to_hit: profile
                            .to_hit
                            .saturating_add(i32::from(item.enchantments.to_hit))
                            .saturating_add(i32::from(ammunition_enchantments.to_hit)),
                        to_damage: profile
                            .to_damage
                            .saturating_add(i32::from(item.enchantments.to_damage))
                            .saturating_add(i32::from(ammunition_enchantments.to_damage)),
                        ammunition_to_hit: ammunition_enchantments.to_hit,
                        damage_dice: profile.damage_dice,
                        damage_sides: profile.damage_sides,
                        damage_type: DamageType::from(profile.damage_type),
                        ammo_kind_id: profile.ammo_kind_id.clone(),
                        ammo_break_chance_percent,
                        source_item_id: item.id.clone(),
                    })
                })
        })
    }

    fn player_melee_profile(&self, stats: &ActorDerivedStats) -> ResolvedAttackProfile {
        let definition = self
            .content
            .actor(&self.player.kind_id)
            .expect("player actor definition must remain available");
        let equipped_weapon = self.items.iter().find_map(|item| {
            let ItemLocation::Equipped { slot_id } = &item.location else {
                return None;
            };
            if self.body_slot_type(slot_id) != Some("weapon") {
                return None;
            }
            self.content
                .item(&item.kind_id)
                .and_then(|definition| definition.melee_profile.as_ref())
                .map(|profile| (item.id.clone(), profile))
        });
        let (source_item_id, dice, sides, damage_type, to_hit) = equipped_weapon.map_or_else(
            || {
                (
                    None,
                    definition.damage_dice,
                    definition.damage_sides,
                    definition.damage_type,
                    0,
                )
            },
            |(item_id, profile)| {
                (
                    Some(item_id),
                    profile.damage_dice,
                    profile.damage_sides,
                    profile.damage_type,
                    profile.to_hit,
                )
            },
        );
        ResolvedAttackProfile {
            attacks: u16::try_from(stats.melee_attacks.value)
                .expect("derived melee attack count must fit u16"),
            to_hit,
            to_damage: stats.melee_damage_bonus.value,
            damage_dice: dice,
            damage_sides: sides,
            damage_type: DamageType::from(damage_type),
            source_item_id,
        }
    }

    fn player_melee_damage_multiplier(
        &self,
        profile: &ResolvedAttackProfile,
        target: &Actor,
        definition: &rfb_content::ActorDefinition,
    ) -> i32 {
        if profile.source_item_id.is_none() {
            return 10;
        }
        let mut multiplier = 10;
        let mut apply = |slays: &BTreeMap<SlayTarget, SlayLevel>,
                         brands: &BTreeSet<WeaponBrand>| {
            for (slay_target, level) in slays {
                if slay_target_matches(*slay_target, definition) {
                    multiplier = multiplier.max(slay_multiplier(*slay_target, *level));
                }
            }
            for brand in brands {
                if target.resistances.level(brand_damage_type(*brand)) != ResistanceLevel::Immune {
                    multiplier = multiplier.max(24);
                }
            }
        };
        for item in &self.items {
            if !matches!(&item.location, ItemLocation::Equipped { .. }) {
                continue;
            }
            if let Some(item_definition) = self.content.item(&item.kind_id) {
                apply(&item_definition.slays, &item_definition.brands);
            }
            for affix_id in &item.affix_ids {
                if let Some(affix) = self.content.affix(affix_id) {
                    apply(&affix.slays, &affix.brands);
                }
            }
            for rolled in &item.rolled_affixes {
                apply(&rolled.properties.slays, &rolled.properties.brands);
            }
        }
        for status in &self.player.statuses {
            for brand in &status.granted_brands {
                if target.resistances.level(brand_damage_type(*brand)) != ResistanceLevel::Immune {
                    multiplier = multiplier.max(24);
                }
            }
        }
        multiplier
    }

    fn add_character_stat_contributions(&self, pipeline: &mut DerivedStatsPipeline) {
        let Some((_, race, class, personality)) = self.character_definitions() else {
            return;
        };
        for (layer, source_id, modifiers) in [
            (StatLayer::Species, race.id.as_str(), &race.modifiers),
            (StatLayer::Class, class.id.as_str(), &class.modifiers),
            (
                StatLayer::Personality,
                personality.id.as_str(),
                &personality.modifiers,
            ),
        ] {
            add_nonzero_stat(
                pipeline,
                StatKind::MaxHp,
                layer,
                source_id,
                modifiers.max_hp,
            );
            add_nonzero_stat(
                pipeline,
                StatKind::Attack,
                layer,
                source_id,
                modifiers.attack,
            );
            add_nonzero_stat(
                pipeline,
                StatKind::Defense,
                layer,
                source_id,
                modifiers.defense,
            );
        }
    }

    fn add_character_skill_contributions(&self, pipeline: &mut DerivedStatsPipeline) {
        let Some((_, race, class, personality)) = self.character_definitions() else {
            return;
        };
        for (layer, source_id, skill_set_id) in [
            (
                StatLayer::Species,
                race.id.as_str(),
                race.skill_set_id.as_str(),
            ),
            (
                StatLayer::Class,
                class.id.as_str(),
                class.skill_set_id.as_str(),
            ),
            (
                StatLayer::Personality,
                personality.id.as_str(),
                personality.skill_set_id.as_str(),
            ),
        ] {
            let skill_set = self
                .content
                .skill_set(skill_set_id)
                .expect("validated skill set must remain available");
            for entry in &skill_set.entries {
                let definition = self
                    .content
                    .skill(&entry.skill_id)
                    .expect("validated skill must remain available");
                let amount = entry.base.saturating_add(
                    entry
                        .growth_per_ten_levels
                        .saturating_mul(i32::from(self.progress.level))
                        .saturating_div(10),
                );
                match definition.kind {
                    SkillKind::Disarming => {
                        add_nonzero_stat(pipeline, StatKind::DoorSkill, layer, source_id, amount);
                        add_nonzero_stat(pipeline, StatKind::DisarmSkill, layer, source_id, amount);
                    }
                    SkillKind::Search => {
                        add_nonzero_stat(pipeline, StatKind::SearchSkill, layer, source_id, amount)
                    }
                    SkillKind::Melee => {
                        add_nonzero_stat(pipeline, StatKind::MeleeSkill, layer, source_id, amount)
                    }
                    SkillKind::Ranged => {
                        add_nonzero_stat(pipeline, StatKind::RangedSkill, layer, source_id, amount)
                    }
                    SkillKind::Throwing => add_nonzero_stat(
                        pipeline,
                        StatKind::ThrowingSkill,
                        layer,
                        source_id,
                        amount,
                    ),
                    SkillKind::Digging => {
                        add_nonzero_stat(pipeline, StatKind::DigSkill, layer, source_id, amount)
                    }
                    SkillKind::Device => {
                        add_nonzero_stat(pipeline, StatKind::DeviceSkill, layer, source_id, amount)
                    }
                    SkillKind::SavingThrow => add_nonzero_stat(
                        pipeline,
                        StatKind::SavingThrowSkill,
                        layer,
                        source_id,
                        amount,
                    ),
                    SkillKind::Stealth => {
                        add_nonzero_stat(pipeline, StatKind::StealthSkill, layer, source_id, amount)
                    }
                    SkillKind::Perception => add_nonzero_stat(
                        pipeline,
                        StatKind::PerceptionSkill,
                        layer,
                        source_id,
                        amount,
                    ),
                }
            }
        }
    }

    fn actor_derived_stats(
        &self,
        actor: &Actor,
        definition: &rfb_content::ActorDefinition,
        include_equipment: bool,
    ) -> ActorDerivedStats {
        let mut pipeline = DerivedStatsPipeline::new();
        let base_source = definition.id.as_str();
        pipeline.add(
            StatKind::MaxHp,
            StatLayer::Base,
            base_source,
            if include_equipment {
                self.character_base_max_hp_at_level(self.progress.level)
            } else {
                actor.max_hp
            },
        );
        pipeline.add(
            StatKind::Attack,
            StatLayer::Base,
            base_source,
            definition.attack,
        );
        pipeline.add(
            StatKind::Defense,
            StatLayer::Base,
            base_source,
            definition.defense,
        );
        pipeline.add(
            StatKind::Speed,
            StatLayer::Base,
            base_source,
            i32::from(actor.speed),
        );
        pipeline.add(
            StatKind::MeleeSkill,
            StatLayer::Base,
            base_source,
            if definition.role == ActorRole::Monster {
                monster_melee_skill(definition.attack, definition.level)
            } else if include_equipment && self.build.is_some() {
                0
            } else {
                rating_to_combat_value(definition.attack)
            },
        );
        pipeline.add(
            StatKind::ArmorClass,
            StatLayer::Base,
            base_source,
            rating_to_armor_class(definition.defense),
        );
        pipeline.add(StatKind::MeleeAttacks, StatLayer::Base, base_source, 1);
        pipeline.add(StatKind::MeleeDamageBonus, StatLayer::Base, base_source, 0);
        pipeline.add(
            StatKind::RangedSkill,
            StatLayer::Base,
            base_source,
            if include_equipment && self.build.is_some() {
                0
            } else {
                rating_to_combat_value(definition.attack)
            },
        );
        pipeline.add(
            StatKind::ThrowingSkill,
            StatLayer::Base,
            base_source,
            if include_equipment && self.build.is_some() {
                0
            } else {
                rating_to_combat_value(definition.attack)
            },
        );
        pipeline.add(
            StatKind::DoorSkill,
            StatLayer::Base,
            base_source,
            if include_equipment && self.build.is_some() {
                0
            } else {
                definition.door_skill
            },
        );
        pipeline.add(
            StatKind::BashPower,
            StatLayer::Base,
            base_source,
            definition.bash_power,
        );
        pipeline.add(
            StatKind::SearchSkill,
            StatLayer::Base,
            base_source,
            if include_equipment && self.build.is_some() {
                0
            } else {
                definition.search_skill
            },
        );
        pipeline.add(StatKind::DeviceSkill, StatLayer::Base, base_source, 0);
        pipeline.add(StatKind::SavingThrowSkill, StatLayer::Base, base_source, 0);
        pipeline.add(StatKind::StealthSkill, StatLayer::Base, base_source, 0);
        pipeline.add(StatKind::PerceptionSkill, StatLayer::Base, base_source, 0);
        pipeline.add(
            StatKind::DisarmSkill,
            StatLayer::Base,
            base_source,
            if include_equipment && self.build.is_some() {
                0
            } else {
                definition.disarm_skill
            },
        );
        pipeline.add(
            StatKind::DigSkill,
            StatLayer::Base,
            base_source,
            if include_equipment && self.build.is_some() {
                0
            } else {
                definition.dig_skill
            },
        );

        if include_equipment {
            self.add_character_stat_contributions(&mut pipeline);
            self.add_character_skill_contributions(&mut pipeline);
            for item in self
                .items
                .iter()
                .filter(|item| matches!(&item.location, ItemLocation::Equipped { .. }))
            {
                let modifiers = self.item_modifiers(item);
                add_equipment_stat(&mut pipeline, StatKind::MaxHp, &item.id, modifiers.max_hp);
                add_equipment_stat(&mut pipeline, StatKind::Attack, &item.id, modifiers.attack);
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::Defense,
                    &item.id,
                    modifiers.defense,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::MeleeSkill,
                    &item.id,
                    rating_to_combat_value(modifiers.attack),
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::ArmorClass,
                    &item.id,
                    rating_to_armor_class(modifiers.defense),
                );
                add_equipment_stat(&mut pipeline, StatKind::Speed, &item.id, modifiers.speed);
                let bonuses = self.item_equipment_bonuses(item);
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::MeleeAttacks,
                    &item.id,
                    bonuses.melee_attacks,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::MeleeSkill,
                    &item.id,
                    bonuses.melee_skill,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::RangedSkill,
                    &item.id,
                    bonuses.ranged_skill,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::ThrowingSkill,
                    &item.id,
                    bonuses.throwing_skill,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::DeviceSkill,
                    &item.id,
                    bonuses.device_skill,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::SavingThrowSkill,
                    &item.id,
                    bonuses.saving_throw_skill,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::StealthSkill,
                    &item.id,
                    bonuses.stealth_skill,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::SearchSkill,
                    &item.id,
                    bonuses.search_skill,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::PerceptionSkill,
                    &item.id,
                    bonuses.perception_skill,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::DisarmSkill,
                    &item.id,
                    bonuses.disarming_skill,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::DigSkill,
                    &item.id,
                    bonuses.digging_skill,
                );
                if let Some(profile) = self
                    .content
                    .item(&item.kind_id)
                    .and_then(|definition| definition.melee_profile.as_ref())
                {
                    add_equipment_stat(
                        &mut pipeline,
                        StatKind::MeleeAttacks,
                        &item.id,
                        i32::from(profile.attacks).saturating_sub(1),
                    );
                    add_equipment_stat(
                        &mut pipeline,
                        StatKind::MeleeSkill,
                        &item.id,
                        profile
                            .to_hit
                            .saturating_add(i32::from(item.enchantments.to_hit)),
                    );
                    add_equipment_stat(
                        &mut pipeline,
                        StatKind::MeleeDamageBonus,
                        &item.id,
                        profile
                            .to_damage
                            .saturating_add(i32::from(item.enchantments.to_damage)),
                    );
                }
                if let Some(profile) = self
                    .content
                    .item(&item.kind_id)
                    .and_then(|definition| definition.projectile_profile.as_ref())
                {
                    add_equipment_stat(
                        &mut pipeline,
                        StatKind::RangedSkill,
                        &item.id,
                        profile
                            .to_hit
                            .saturating_add(i32::from(item.enchantments.to_hit)),
                    );
                }
            }
        }

        for status in &actor.statuses {
            let modifiers = status.granted_modifiers;
            for (kind, value) in [
                (StatKind::MaxHp, modifiers.max_hp),
                (StatKind::Attack, modifiers.attack),
                (StatKind::Defense, modifiers.defense),
                (StatKind::MeleeSkill, modifiers.attack),
                (StatKind::ArmorClass, modifiers.defense),
                (StatKind::Speed, modifiers.speed),
            ] {
                pipeline.add_with_origin(
                    kind,
                    StatLayer::Status,
                    &status.kind_id,
                    status.source_id.clone(),
                    value,
                );
            }
            let bonuses = status.granted_equipment_bonuses;
            for (kind, value) in [
                (StatKind::MeleeAttacks, bonuses.melee_attacks),
                (StatKind::MeleeSkill, bonuses.melee_skill),
                (StatKind::MeleeDamageBonus, bonuses.melee_damage),
                (StatKind::RangedSkill, bonuses.ranged_skill),
                (StatKind::ThrowingSkill, bonuses.throwing_skill),
                (StatKind::DeviceSkill, bonuses.device_skill),
                (StatKind::SavingThrowSkill, bonuses.saving_throw_skill),
                (StatKind::StealthSkill, bonuses.stealth_skill),
                (StatKind::SearchSkill, bonuses.search_skill),
                (StatKind::PerceptionSkill, bonuses.perception_skill),
                (StatKind::DisarmSkill, bonuses.disarming_skill),
                (StatKind::DigSkill, bonuses.digging_skill),
            ] {
                pipeline.add_with_origin(
                    kind,
                    StatLayer::Status,
                    &status.kind_id,
                    status.source_id.clone(),
                    value,
                );
            }
            let amount = i32::from(status.intensity).saturating_mul(10);
            if status.kind_id == STATUS_HASTE {
                pipeline.add_with_origin(
                    StatKind::Speed,
                    StatLayer::Status,
                    &status.kind_id,
                    status.source_id.clone(),
                    amount,
                );
            } else if status.kind_id == STATUS_SLOW {
                pipeline.add_with_origin(
                    StatKind::Speed,
                    StatLayer::Status,
                    &status.kind_id,
                    status.source_id.clone(),
                    amount.saturating_neg(),
                );
            }
            if status.kind_id == STATUS_STUN {
                pipeline.add_with_origin(
                    StatKind::MeleeSkill,
                    StatLayer::Status,
                    &status.kind_id,
                    status.source_id.clone(),
                    i32::from(status.intensity)
                        .saturating_mul(10)
                        .saturating_neg(),
                );
                pipeline.add_with_origin(
                    StatKind::ThrowingSkill,
                    StatLayer::Status,
                    &status.kind_id,
                    status.source_id.clone(),
                    i32::from(status.intensity)
                        .saturating_mul(10)
                        .saturating_neg(),
                );
            }
        }

        ActorDerivedStats {
            max_hp: pipeline.resolve(StatKind::MaxHp, StatBounds::UNBOUNDED),
            attack: pipeline.resolve(StatKind::Attack, StatBounds::NON_NEGATIVE),
            defense: pipeline.resolve(StatKind::Defense, StatBounds::NON_NEGATIVE),
            speed: pipeline.resolve(StatKind::Speed, StatBounds::ACTOR_SPEED),
            melee_skill: pipeline.resolve(StatKind::MeleeSkill, StatBounds::NON_NEGATIVE),
            armor_class: pipeline.resolve(StatKind::ArmorClass, StatBounds::NON_NEGATIVE),
            melee_attacks: pipeline.resolve(StatKind::MeleeAttacks, StatBounds::NON_NEGATIVE),
            melee_damage_bonus: pipeline.resolve(StatKind::MeleeDamageBonus, StatBounds::UNBOUNDED),
            ranged_skill: pipeline.resolve(StatKind::RangedSkill, StatBounds::NON_NEGATIVE),
            throwing_skill: pipeline.resolve(StatKind::ThrowingSkill, StatBounds::NON_NEGATIVE),
            door_skill: pipeline.resolve(StatKind::DoorSkill, StatBounds::NON_NEGATIVE),
            bash_power: pipeline.resolve(StatKind::BashPower, StatBounds::NON_NEGATIVE),
            search_skill: pipeline.resolve(StatKind::SearchSkill, StatBounds::NON_NEGATIVE),
            device_skill: pipeline.resolve(StatKind::DeviceSkill, StatBounds::NON_NEGATIVE),
            saving_throw_skill: pipeline
                .resolve(StatKind::SavingThrowSkill, StatBounds::NON_NEGATIVE),
            stealth_skill: pipeline.resolve(StatKind::StealthSkill, StatBounds::NON_NEGATIVE),
            perception_skill: pipeline.resolve(StatKind::PerceptionSkill, StatBounds::NON_NEGATIVE),
            disarm_skill: pipeline.resolve(StatKind::DisarmSkill, StatBounds::NON_NEGATIVE),
            dig_skill: pipeline.resolve(StatKind::DigSkill, StatBounds::NON_NEGATIVE),
        }
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

    fn resolve_player_ability_effect(
        &mut self,
        ability: AbilityDefinition,
        target_plan: AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        match (ability.effect.clone(), target_plan) {
            (AbilityEffectDefinition::Teleport, AbilityTargetPlan::Teleport { destination }) => {
                self.resolve_player_teleport_effect(&ability, destination, events, changed);
            }
            (
                AbilityEffectDefinition::Summon {
                    actor_kind_id,
                    count,
                    duration_turns,
                    hostile,
                    ..
                },
                AbilityTargetPlan::Summon { positions },
            ) => {
                debug_assert_eq!(usize::from(count), positions.len());
                let definition = self
                    .content
                    .actor(&actor_kind_id)
                    .expect("validated summon actor must remain available")
                    .clone();
                let mut entity_ids = Vec::with_capacity(positions.len());
                for (ordinal, position) in positions.iter().copied().enumerate() {
                    let id = self.summon_entity_id(&ability.id, ordinal);
                    let mut entity = actor_from_runtime_spawn(
                        &id,
                        &actor_kind_id,
                        position,
                        definition.max_hp,
                        definition.speed,
                        INITIAL_MONSTER_ENERGY_NEED,
                        true,
                    );
                    entity.resistances = definition_resistance_profile(&definition);
                    if !hostile {
                        entity.summon = Some(SummonIdentity {
                            owner_id: self.player.id.clone(),
                            source_ability_id: ability.id.clone(),
                            remaining_turns: duration_turns,
                        });
                    }
                    changed.insert(position);
                    entity_ids.push(id);
                    self.entities.push(entity);
                }
                events.push(DomainEvent::AbilitySummoned {
                    ability_id: ability.id,
                    resolution: AbilitySummonResolutionDto {
                        owner_id: self.player.id.clone(),
                        actor_kind_id,
                        entity_ids,
                        positions,
                        duration_turns,
                        hostile,
                        group: false,
                        summoned_kind_ids: Vec::new(),
                    },
                });
            }
            (
                AbilityEffectDefinition::SummonCategory {
                    category,
                    upgraded_category,
                    upgrade_at_level,
                    count_dice,
                    count_sides,
                    count_bonus,
                    hostile_chance_percent,
                    friendly_group_chance_percent,
                    hostile_group_chance_percent,
                    group_count_dice,
                    group_count_sides,
                    group_count_bonus,
                    duration_turns,
                    ..
                },
                AbilityTargetPlan::SummonCategory {
                    friendly_candidate_kind_ids,
                    hostile_candidate_kind_ids,
                    positions,
                },
            ) => {
                let hostile = match hostile_chance_percent {
                    0 => false,
                    100 => true,
                    chance => self.rng.bounded(100) < u64::from(chance),
                };
                let group_chance = if hostile {
                    hostile_group_chance_percent
                } else {
                    friendly_group_chance_percent
                };
                let candidates = if hostile {
                    hostile_candidate_kind_ids
                } else {
                    friendly_candidate_kind_ids
                };
                let selected_category = upgraded_category
                    .zip(upgrade_at_level)
                    .filter(|(_, level)| self.progress.level >= *level)
                    .map_or(category, |(category, _)| category);
                let owner_id = self.player.id.clone();
                let resolution = self.resolve_category_summon(
                    CategorySummonSpec {
                        source_id: &ability.id,
                        owner_id: &owner_id,
                        category: &selected_category,
                        count_dice,
                        count_sides,
                        count_bonus,
                        hostile,
                        group_chance_percent: group_chance,
                        group_count_dice,
                        group_count_sides,
                        group_count_bonus,
                        duration_turns,
                    },
                    candidates,
                    positions,
                    changed,
                );
                events.push(DomainEvent::AbilitySummoned {
                    ability_id: ability.id,
                    resolution,
                });
            }
            (AbilityEffectDefinition::Detect { .. }, AbilityTargetPlan::Detect) => {
                self.resolve_player_detection_effect(&ability, events, changed);
            }
            (
                AbilityEffectDefinition::TransformTerrain {
                    source_terrain_ids,
                    target_terrain_id,
                    radius,
                },
                AbilityTargetPlan::TerrainTransform { center, positions },
            ) => {
                for position in &positions {
                    let index = self
                        .index(*position)
                        .expect("planned terrain transformation must remain in bounds");
                    debug_assert!(source_terrain_ids.contains(&self.terrain[index]));
                    self.terrain[index].clone_from(&target_terrain_id);
                    self.revealed_terrain.remove(position);
                    changed.insert(*position);
                }
                events.push(DomainEvent::AbilityTerrainTransformed {
                    ability_id: ability.id,
                    resolution: AbilityTerrainTransformResolutionDto {
                        center,
                        radius,
                        source_terrain_ids,
                        target_terrain_id,
                        transformed_positions: positions,
                    },
                });
            }
            (effect, target_plan)
                if matches!(
                    effect,
                    AbilityEffectDefinition::ApplyStatus { .. }
                        | AbilityEffectDefinition::RemoveStatus { .. }
                        | AbilityEffectDefinition::Control { .. }
                        | AbilityEffectDefinition::Sequence { .. }
                ) =>
            {
                self.resolve_ability_actor_effects(
                    &ability.id,
                    &effect,
                    target_plan,
                    events,
                    changed,
                    removed_entities,
                )?;
                self.clamp_player_hp_to_effective_max();
            }
            (
                AbilityEffectDefinition::Damage { .. },
                AbilityTargetPlan::Projectile { path, .. },
            ) => {
                self.resolve_player_projectile_damage_effect(
                    &ability,
                    path,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                AbilityEffectDefinition::DeathRay { .. },
                AbilityTargetPlan::Projectile { path, .. },
            ) => {
                self.resolve_player_death_ray_effect(
                    &ability,
                    path,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                AbilityEffectDefinition::AreaDamage { .. },
                AbilityTargetPlan::Projectile {
                    path,
                    stop_at_actor,
                },
            ) => {
                self.resolve_player_area_damage_effect(
                    &ability,
                    path,
                    stop_at_actor,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                AbilityEffectDefinition::BeamDamage { .. },
                AbilityTargetPlan::Projectile { path, .. },
            ) => {
                self.resolve_player_beam_damage_effect(
                    &ability,
                    path,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                AbilityEffectDefinition::BoltOrBeamDamage { .. },
                AbilityTargetPlan::Projectile { path, .. },
            ) => {
                self.resolve_player_bolt_or_beam_damage_effect(
                    &ability,
                    path,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                AbilityEffectDefinition::ConeDamage { .. },
                target_plan @ AbilityTargetPlan::Cone { .. },
            ) => {
                self.resolve_player_cone_damage_effect(
                    &ability,
                    target_plan,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (AbilityEffectDefinition::Heal { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_healing_effect(&ability, events);
            }
            (
                AbilityEffectDefinition::IdentifyItem {
                    full_identify_power,
                    full_identify_roll_sides,
                },
                AbilityTargetPlan::Item { item_id },
            ) => {
                let roll = u16::try_from(self.rng.bounded(u64::from(full_identify_roll_sides)) + 1)
                    .expect("validated identify roll must fit u16");
                let full = roll <= full_identify_power;
                let identification = self.identify_item_instance(&item_id, full);
                events.push(DomainEvent::AbilityEffectsResolved {
                    ability_id: ability.id,
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
            (AbilityEffectDefinition::RestoreVitality { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_restore_vitality_effect(&ability, events);
            }
            (AbilityEffectDefinition::VisibleDamage { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_visible_damage_effect(
                    &ability,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (AbilityEffectDefinition::VisibleApplyStatus { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_visible_status_effect(&ability, events, changed);
            }
            (
                AbilityEffectDefinition::EnchantEquippedWeapon { affix_id },
                AbilityTargetPlan::SelfTarget,
            ) => {
                let weapon_index = self.items.iter().position(|item| {
                    let ItemLocation::Equipped { slot_id } = &item.location else {
                        return false;
                    };
                    self.body_slot_type(slot_id) == Some("weapon")
                        && self
                            .content
                            .item(&item.kind_id)
                            .is_some_and(|definition| definition.melee_profile.is_some())
                });
                let (item_id, item_kind_id, added) = if let Some(index) = weapon_index {
                    let item_id = self.items[index].id.clone();
                    let item_kind_id = self.items[index].kind_id.clone();
                    let added = if self.items[index].affix_ids.contains(&affix_id) {
                        false
                    } else {
                        self.items[index].affix_ids.push(affix_id.clone());
                        self.items[index].affix_ids.sort();
                        self.items[index].quality = ItemQualityDto::Fine;
                        let knowledge = self
                            .item_property_knowledge
                            .entry(item_id.clone())
                            .or_default();
                        knowledge.appraised = true;
                        knowledge.identified = true;
                        knowledge.known_affix_ids.insert(affix_id.clone());
                        true
                    };
                    (item_id, item_kind_id, added)
                } else {
                    (String::new(), String::new(), false)
                };
                self.clamp_player_hp_to_effective_max();
                events.push(DomainEvent::AbilityEffectsResolved {
                    ability_id: ability.id.clone(),
                    resolution: AbilityEffectsResolutionDto {
                        target_entity_id: None,
                        target_kind_id: None,
                        effects: vec![AbilityEffectResolutionDto::EnchantEquippedWeapon {
                            effect_index: 0,
                            item_id,
                            item_kind_id,
                            affix_id,
                            added,
                        }],
                    },
                    trace: None,
                });
            }
            (AbilityEffectDefinition::NoOp { reason }, _) => {
                events.push(DomainEvent::AbilityEffectsResolved {
                    ability_id: ability.id.clone(),
                    resolution: AbilityEffectsResolutionDto {
                        target_entity_id: None,
                        target_kind_id: None,
                        effects: vec![AbilityEffectResolutionDto::NoOp {
                            effect_index: 0,
                            reason,
                        }],
                    },
                    trace: None,
                });
            }
            (
                AbilityEffectDefinition::DrainLife { .. },
                AbilityTargetPlan::Projectile { path, .. },
            ) => {
                self.resolve_player_drain_life_effect(
                    &ability,
                    path,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                AbilityEffectDefinition::Genocide {
                    scope,
                    power,
                    radius,
                },
                AbilityTargetPlan::Projectile { path, .. },
            ) => {
                self.resolve_ability_genocide(
                    &ability.id,
                    Some(path),
                    scope,
                    power,
                    radius,
                    events,
                    changed,
                    removed_entities,
                );
            }
            (
                AbilityEffectDefinition::Genocide {
                    scope: AbilityGenocideScopeDefinition::Nearby,
                    power,
                    radius,
                },
                AbilityTargetPlan::SelfTarget,
            ) => {
                self.resolve_ability_genocide(
                    &ability.id,
                    None,
                    AbilityGenocideScopeDefinition::Nearby,
                    power,
                    radius,
                    events,
                    changed,
                    removed_entities,
                );
            }
            (
                AbilityEffectDefinition::AnimateDead {
                    actor_kind_id,
                    corpse_item_kind_id,
                    radius,
                    count,
                },
                AbilityTargetPlan::SelfTarget,
            ) => {
                self.resolve_ability_animate_dead(
                    &ability.id,
                    &actor_kind_id,
                    &corpse_item_kind_id,
                    radius,
                    count,
                    events,
                    changed,
                )?;
            }
            _ => unreachable!("validated ability target plan must match its effect"),
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_ability_genocide(
        &mut self,
        ability_id: &str,
        path: Option<Vec<Position>>,
        scope: AbilityGenocideScopeDefinition,
        power: u16,
        radius: u8,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) {
        let (trace, target_entity_id, target_kind_id, glyph) =
            if scope == AbilityGenocideScopeDefinition::Nearby {
                (None, None, None, None)
            } else {
                let (trace, target_index) =
                    self.trace_projectile_path(path.expect("targeted genocide must retain a path"));
                let Some(target_index) = target_index else {
                    events.push(DomainEvent::AbilityLanded {
                        ability_id: ability_id.to_owned(),
                        trace: trace.clone(),
                    });
                    events.push(DomainEvent::AbilityEffectsResolved {
                        ability_id: ability_id.to_owned(),
                        resolution: AbilityEffectsResolutionDto {
                            target_entity_id: None,
                            target_kind_id: None,
                            effects: vec![AbilityEffectResolutionDto::Skipped {
                                effect_index: 0,
                                reason: AbilityEffectSkipReasonDto::NoTarget,
                            }],
                        },
                        trace: Some(trace),
                    });
                    return;
                };
                let target_entity_id = self.entities[target_index].id.clone();
                let target_kind_id = self.entities[target_index].kind_id.clone();
                let glyph = self
                    .content
                    .actor(&target_kind_id)
                    .map(|definition| definition.glyph.clone());
                (
                    Some(trace),
                    Some(target_entity_id),
                    Some(target_kind_id),
                    glyph,
                )
            };
        let mut candidate_ids = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && match scope {
                        AbilityGenocideScopeDefinition::Single => {
                            target_entity_id.as_deref() == Some(entity.id.as_str())
                        }
                        AbilityGenocideScopeDefinition::Glyph => self
                            .content
                            .actor(&entity.kind_id)
                            .zip(glyph.as_ref())
                            .is_some_and(|(definition, glyph)| &definition.glyph == glyph),
                        AbilityGenocideScopeDefinition::Nearby => {
                            chebyshev_distance(self.player.position, entity.position)
                                <= u32::from(radius)
                        }
                    }
            })
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        candidate_ids.sort();
        let resolution = self.resolve_genocide_candidates(
            candidate_ids,
            scope,
            power,
            changed,
            removed_entities,
        );
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability_id.to_owned(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id,
                target_kind_id,
                effects: vec![AbilityEffectResolutionDto::Genocide {
                    effect_index: 0,
                    scope: ability_genocide_scope_dto(scope),
                    power,
                    radius,
                    glyph: matches!(scope, AbilityGenocideScopeDefinition::Glyph)
                        .then_some(glyph)
                        .flatten(),
                    removed_entity_ids: resolution.removed_entity_ids,
                    resisted_entity_ids: resolution.resisted_entity_ids,
                    fatigue_damage: resolution.fatigue_damage,
                }],
            },
            trace,
        });
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

    #[allow(clippy::too_many_arguments)]
    fn resolve_ability_animate_dead(
        &mut self,
        ability_id: &str,
        actor_kind_id: &str,
        corpse_item_kind_id: &str,
        radius: u8,
        count: u8,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Result<(), CoreError> {
        let origin = self.player.position;
        let mut corpses = self
            .items
            .iter()
            .filter_map(|item| match item.location {
                ItemLocation::Ground(position)
                    if item.kind_id == corpse_item_kind_id
                        && chebyshev_distance(origin, position) <= u32::from(radius) =>
                {
                    Some((
                        chebyshev_distance(origin, position),
                        position.y,
                        position.x,
                        item.id.clone(),
                        position,
                    ))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        corpses.sort();
        corpses.truncate(usize::from(count));
        let consumed_corpse_item_ids = corpses
            .iter()
            .map(|corpse| corpse.3.clone())
            .collect::<Vec<_>>();
        self.items
            .retain(|item| !consumed_corpse_item_ids.contains(&item.id));
        for item_id in &consumed_corpse_item_ids {
            self.item_property_knowledge.remove(item_id);
        }
        let definition = self
            .content
            .actor(actor_kind_id)
            .expect("validated animated actor must remain available")
            .clone();
        let mut entity_ids = Vec::with_capacity(corpses.len());
        let mut positions = Vec::with_capacity(corpses.len());
        for (ordinal, (_, _, _, _, position)) in corpses.into_iter().enumerate() {
            let id = self.summon_entity_id(ability_id, ordinal);
            let mut entity = actor_from_runtime_spawn(
                &id,
                actor_kind_id,
                position,
                definition.max_hp,
                definition.speed,
                INITIAL_MONSTER_ENERGY_NEED,
                true,
            );
            entity.resistances = definition_resistance_profile(&definition);
            entity.controller_id = Some(self.player.id.clone());
            self.entities.push(entity);
            changed.insert(position);
            entity_ids.push(id);
            positions.push(position);
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability_id.to_owned(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::AnimateDead {
                    effect_index: 0,
                    actor_kind_id: actor_kind_id.to_owned(),
                    consumed_corpse_item_ids,
                    entity_ids,
                    positions,
                }],
            },
            trace: None,
        });
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_ability_actor_effects(
        &mut self,
        ability_id: &str,
        effect: &AbilityEffectDefinition,
        target_plan: AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let effects = effect.ordered_effects();
        match target_plan {
            AbilityTargetPlan::SelfTarget => {
                let target_entity_id = self.player.id.clone();
                let target_kind_id = self.player.kind_id.clone();
                let mut resolutions = Vec::with_capacity(effects.len());
                for (index, effect) in effects.iter().enumerate() {
                    let effect_index =
                        u8::try_from(index).expect("validated ability effect index must fit u8");
                    let resolution = match effect {
                        AbilityEffectDefinition::Heal { amount } => {
                            let max_hp = self.effective_player_max_hp();
                            let amount = i32::try_from(*amount)
                                .expect("validated healing amount must fit i32");
                            let outcome = apply_effect(
                                &mut EffectTarget {
                                    hp: &mut self.player.hp,
                                    max_hp,
                                    resistances: &self.player.resistances,
                                    statuses: &mut self.player.statuses,
                                },
                                EffectSpec::Heal { amount },
                            );
                            let EffectOutcome::Healed { requested, applied } = outcome else {
                                unreachable!("healing effects must produce healing outcomes");
                            };
                            AbilityEffectResolutionDto::Heal {
                                effect_index,
                                resolution: HealingResolutionDto { requested, applied },
                            }
                        }
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
                        } => apply_ability_status_effect(
                            &mut self.player,
                            ability_id,
                            effect_index,
                            status_kind_id,
                            *intensity,
                            *duration_ticks,
                            *duration_dice,
                            *duration_sides,
                            *stacking,
                            *resistance_type,
                            *power,
                            granted_resistances,
                            granted_brands,
                            granted_modifiers,
                            granted_equipment_bonuses,
                            granted_status_immunities,
                            granted_race_id.as_deref(),
                            *grants_wall_passage,
                            *incoming_damage_percent,
                            None,
                            None,
                            &mut self.rng,
                        ),
                        AbilityEffectDefinition::RemoveStatus { status_kind_id } => {
                            remove_ability_status_effect(
                                &mut self.player,
                                effect_index,
                                status_kind_id,
                            )
                        }
                        _ => unreachable!(
                            "validated self-target effect sequences contain only actor effects"
                        ),
                    };
                    resolutions.push(resolution);
                }
                events.push(DomainEvent::AbilityEffectsResolved {
                    ability_id: ability_id.to_owned(),
                    resolution: AbilityEffectsResolutionDto {
                        target_entity_id: Some(target_entity_id),
                        target_kind_id: Some(target_kind_id),
                        effects: resolutions,
                    },
                    trace: None,
                });
                self.refresh_player_resource_maxima();
            }
            AbilityTargetPlan::Projectile { path, .. } => {
                let (trace, target_index) = self.trace_projectile_path(path);
                let Some(target_index) = target_index else {
                    let resolutions = effects
                        .iter()
                        .enumerate()
                        .map(|(index, _)| AbilityEffectResolutionDto::Skipped {
                            effect_index: u8::try_from(index)
                                .expect("validated ability effect index must fit u8"),
                            reason: AbilityEffectSkipReasonDto::NoTarget,
                        })
                        .collect();
                    events.push(DomainEvent::AbilityLanded {
                        ability_id: ability_id.to_owned(),
                        trace: trace.clone(),
                    });
                    events.push(DomainEvent::AbilityEffectsResolved {
                        ability_id: ability_id.to_owned(),
                        resolution: AbilityEffectsResolutionDto {
                            target_entity_id: None,
                            target_kind_id: None,
                            effects: resolutions,
                        },
                        trace: Some(trace),
                    });
                    return Ok(());
                };

                let target_entity_id = self.entities[target_index].id.clone();
                let target_kind_id = self.entities[target_index].kind_id.clone();
                let mut resolutions = Vec::with_capacity(effects.len());
                for (index, effect) in effects.iter().enumerate() {
                    let effect_index =
                        u8::try_from(index).expect("validated ability effect index must fit u8");
                    let Some(current_index) = self
                        .entities
                        .iter()
                        .position(|entity| entity.id == target_entity_id && entity.hp > 0)
                    else {
                        resolutions.push(AbilityEffectResolutionDto::Skipped {
                            effect_index,
                            reason: AbilityEffectSkipReasonDto::TargetDead,
                        });
                        continue;
                    };
                    let resolution = match effect {
                        AbilityEffectDefinition::Damage {
                            damage_dice,
                            damage_sides,
                            damage_bonus,
                            damage_type,
                        } => {
                            let raw_damage = self
                                .roll_damage(*damage_dice, *damage_sides)
                                .saturating_add(i32::from(*damage_bonus))
                                .max(0);
                            let damage = self.resolve_ability_damage_to_entity(
                                current_index,
                                ability_id,
                                DamageType::from(*damage_type),
                                raw_damage,
                                trace.clone(),
                                events,
                                changed,
                                removed_entities,
                            )?;
                            AbilityEffectResolutionDto::Damage {
                                effect_index,
                                resolution: damage.into(),
                            }
                        }
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
                        } => {
                            let target_level = self
                                .content
                                .actor(&self.entities[current_index].kind_id)
                                .map(|definition| definition.level);
                            self.entities[current_index].alerted = true;
                            changed.insert(self.entities[current_index].position);
                            apply_ability_status_effect(
                                &mut self.entities[current_index],
                                ability_id,
                                effect_index,
                                status_kind_id,
                                *intensity,
                                *duration_ticks,
                                *duration_dice,
                                *duration_sides,
                                *stacking,
                                *resistance_type,
                                *power,
                                granted_resistances,
                                granted_brands,
                                granted_modifiers,
                                granted_equipment_bonuses,
                                granted_status_immunities,
                                granted_race_id.as_deref(),
                                *grants_wall_passage,
                                *incoming_damage_percent,
                                target_level,
                                None,
                                &mut self.rng,
                            )
                        }
                        AbilityEffectDefinition::RemoveStatus { status_kind_id } => {
                            self.entities[current_index].alerted = true;
                            changed.insert(self.entities[current_index].position);
                            remove_ability_status_effect(
                                &mut self.entities[current_index],
                                effect_index,
                                status_kind_id,
                            )
                        }
                        AbilityEffectDefinition::Control { category, power } => {
                            self.entities[current_index].alerted = true;
                            changed.insert(self.entities[current_index].position);
                            self.resolve_ability_control(
                                current_index,
                                effect_index,
                                category,
                                *power,
                            )
                        }
                        _ => unreachable!(
                            "validated projectile effect sequences contain only actor effects"
                        ),
                    };
                    resolutions.push(resolution);
                }
                events.push(DomainEvent::AbilityEffectsResolved {
                    ability_id: ability_id.to_owned(),
                    resolution: AbilityEffectsResolutionDto {
                        target_entity_id: Some(target_entity_id),
                        target_kind_id: Some(target_kind_id),
                        effects: resolutions,
                    },
                    trace: Some(trace),
                });
            }
            _ => unreachable!("actor effects require a self or projectile target plan"),
        }
        Ok(())
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

    fn random_teleport_candidates(&self, maximum_distance: u16) -> Vec<Position> {
        let origin = self.player.position;
        let occupied = self
            .entities
            .iter()
            .filter(|entity| entity.hp > 0)
            .map(|entity| entity.position)
            .collect::<BTreeSet<_>>();
        let mut candidates = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let position = Position {
                    x: i32::from(x),
                    y: i32::from(y),
                };
                let distance = chebyshev_distance(origin, position);
                if distance > 0
                    && distance <= u32::from(maximum_distance)
                    && self.is_walkable(position)
                    && !occupied.contains(&position)
                {
                    candidates.push((
                        std::cmp::Reverse(distance),
                        position.y,
                        position.x,
                        position,
                    ));
                }
            }
        }
        candidates.sort_unstable();
        candidates.truncate(candidates.len().div_ceil(2));
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

    fn recharging_item_power(&self, item_id: &str) -> Option<u16> {
        let item = self.items.iter().find(|item| {
            item.id == item_id
                && item.location == ItemLocation::Inventory
                && item.quantity > 0
                && item.activation.is_none()
        })?;
        let action = self.content.item(&item.kind_id)?.use_action.as_ref()?;
        match action.effect {
            ItemUseEffectDefinition::RechargeFromDevice { power } => Some(power),
            _ => None,
        }
    }

    fn recharging_item_unavailable_reason(
        &self,
        item_id: &str,
        source_item_id: &str,
        target_item_id: &str,
    ) -> Option<&'static str> {
        if item_id == source_item_id || item_id == target_item_id {
            return Some("recharging-item-is-device");
        }
        if source_item_id == target_item_id {
            return Some("source-is-target");
        }
        if self.recharging_item_power(item_id).is_none() {
            return Some("item-unavailable");
        }
        let source = self.items.iter().find(|item| {
            item.id == source_item_id
                && item.location == ItemLocation::Inventory
                && item.quantity > 0
        });
        if source.is_none_or(|item| !self.item_can_supply_recharge(item)) {
            return Some("source-unavailable");
        }
        let target = self.items.iter().find(|item| {
            item.id == target_item_id
                && item.location == ItemLocation::Inventory
                && item.quantity > 0
        });
        if target.is_none_or(|item| !self.item_can_receive_recharge(item)) {
            return Some("target-not-rechargeable");
        }
        None
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
                events.push(self.resolve_recharge_target(
                    target_item_id,
                    profile.resource_id,
                    false,
                    attempted,
                    power,
                    false,
                ));
            }
            DeviceRechargeSourceDto::Item { item_id } => {
                self.recharge_inventory_item_from_device(
                    target_item_id,
                    item_id,
                    power,
                    profile.source_item_destruction_one_in,
                    events,
                );
            }
        }
    }

    fn recharge_inventory_item_from_device(
        &mut self,
        target_item_id: &str,
        source_item_id: &str,
        power: u32,
        source_destruction_one_in: u16,
        events: &mut Vec<DomainEvent>,
    ) {
        let target_charges = self
            .items
            .iter()
            .find(|item| item.id == target_item_id)
            .and_then(|item| item.charges)
            .expect("preflighted recharge target must carry energy");
        let missing = target_charges
            .maximum
            .saturating_sub(target_charges.current);
        let source_index = self
            .items
            .iter()
            .position(|item| item.id == source_item_id)
            .expect("preflighted recharge source must remain available");
        let source_kind_id = self.items[source_index].kind_id.clone();
        let source_current = self.items[source_index]
            .charges
            .expect("recharge source must carry energy")
            .current;
        let attempted = power.min(source_current).min(missing);
        let destruction_roll = (!self.debug_recharge_sources_survive)
            .then(|| self.rng.bounded(u64::from(source_destruction_one_in)));
        let destroy = destruction_roll == Some(0);
        let artifact = self
            .content
            .item(&source_kind_id)
            .is_some_and(|definition| definition.tags.iter().any(|tag| tag == "artifact"));
        let source_destroyed = destroy && !artifact;
        if source_destroyed {
            let removed = self.items.remove(source_index);
            self.item_property_knowledge.remove(&removed.id);
        } else {
            let source = self
                .items
                .iter_mut()
                .find(|item| item.id == source_item_id)
                .expect("surviving recharge source must remain available");
            source
                .charges
                .as_mut()
                .expect("recharge source must carry energy")
                .current -= attempted;
        }
        events.push(self.resolve_recharge_target(
            target_item_id,
            source_kind_id,
            true,
            attempted,
            power,
            source_destroyed,
        ));
    }

    fn resolve_recharge_target(
        &mut self,
        target_item_id: &str,
        source_id: String,
        source_is_item: bool,
        attempted: u32,
        power: u32,
        source_destroyed: bool,
    ) -> DomainEvent {
        let target = self
            .items
            .iter()
            .find(|item| item.id == target_item_id)
            .expect("preflighted recharge target must remain available");
        let target_kind_id = target.kind_id.clone();
        let target_before = target
            .charges
            .expect("recharge target must carry energy")
            .current;

        let difficulty = u32::try_from(
            self.items
                .iter()
                .find(|item| item.id == target_item_id)
                .and_then(|item| item.activation.as_ref())
                .expect("recharge target must retain dynamic activation")
                .device_check_difficulty,
        )
        .expect("validated device difficulty must be positive");
        let half_difficulty = difficulty / 2;
        let failure_one_in = power.saturating_sub(half_difficulty) / 15;
        let (failure_roll, succeeded) = if self.debug_recharge_attempts_succeed {
            (None, true)
        } else if self.debug_recharge_attempts_fail
            || power <= half_difficulty
            || failure_one_in == 0
        {
            (None, false)
        } else {
            let roll = u32::try_from(self.rng.bounded(u64::from(failure_one_in)))
                .expect("recharge failure roll must fit u32");
            (Some(roll), roll != 0)
        };

        let target = self
            .items
            .iter_mut()
            .find(|item| item.id == target_item_id)
            .expect("recharge target must remain available");
        let charges = target
            .charges
            .as_mut()
            .expect("recharge target must carry energy");
        if succeeded {
            charges.current = charges
                .current
                .saturating_add(attempted)
                .min(charges.maximum);
            if charges.current == charges.maximum {
                target.device_recovery_progress = 0;
            }
        } else if !source_is_item {
            charges.current = 0;
            target.device_recovery_progress = 0;
        }
        let target_after = charges.current;
        DomainEvent::DeviceRechargeResolved {
            target_item_id: target_item_id.to_owned(),
            target_kind_id,
            source_id,
            source_is_item,
            attempted,
            target_before,
            target_after,
            succeeded,
            failure_one_in,
            failure_roll,
            source_destroyed,
        }
    }

    fn use_recharging_item(
        &mut self,
        item_id: &str,
        source_item_id: &str,
        target_item_id: &str,
        events: &mut Vec<DomainEvent>,
    ) {
        if self
            .recharging_item_unavailable_reason(item_id, source_item_id, target_item_id)
            .is_some()
        {
            events.push(DomainEvent::ItemUseUnavailable);
            return;
        }
        let power = u32::from(
            self.recharging_item_power(item_id)
                .expect("preflighted recharging item must retain its power"),
        );
        let index = self
            .items
            .iter()
            .position(|item| item.id == item_id)
            .expect("preflighted recharging item must remain available");
        let kind_id = self.items[index].kind_id.clone();
        self.mark_item_tried(&kind_id);
        if self.items[index].quantity == 1 {
            let removed = self.items.remove(index);
            self.item_property_knowledge.remove(&removed.id);
        } else {
            self.items[index].quantity -= 1;
        }
        self.recharge_inventory_item_from_device(
            target_item_id,
            source_item_id,
            power,
            RECHARGING_ITEM_SOURCE_DESTRUCTION_ONE_IN,
            events,
        );
        self.mark_item_aware(&kind_id);
    }

    fn resolve_inventory_item_effect(
        &mut self,
        settled: SettledItemUse,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let SettledItemUse {
            kind_id,
            profile_id,
            effect,
            plan,
        } = settled;
        match (effect, plan) {
            (
                effect @ (ItemUseEffectDefinition::Heal { .. }
                | ItemUseEffectDefinition::HealDice { .. }
                | ItemUseEffectDefinition::Bless { .. }
                | ItemUseEffectDefinition::ApplySlowness { .. }
                | ItemUseEffectDefinition::ApplySpeed { .. }
                | ItemUseEffectDefinition::ApplyHeroism { .. }
                | ItemUseEffectDefinition::ApplyBerserkStrength { .. }
                | ItemUseEffectDefinition::ApplyPoeticInspiration { .. }
                | ItemUseEffectDefinition::ApplyStoneSkin { .. }
                | ItemUseEffectDefinition::RestoreLifeLevels { .. }
                | ItemUseEffectDefinition::RestoreAllAttributes
                | ItemUseEffectDefinition::RestoreAllVitality { .. }
                | ItemUseEffectDefinition::ApplyRestorativeFeast { .. }
                | ItemUseEffectDefinition::ApplyLifeRestoration { .. }
                | ItemUseEffectDefinition::ApplyThermalResistance { .. }
                | ItemUseEffectDefinition::ApplyBasicResistance { .. }
                | ItemUseEffectDefinition::ApplyPoison { .. }
                | ItemUseEffectDefinition::ApplyBlindness { .. }
                | ItemUseEffectDefinition::DrainAttribute { .. }
                | ItemUseEffectDefinition::RestoreAttribute { .. }
                | ItemUseEffectDefinition::IncreaseAttribute { .. }
                | ItemUseEffectDefinition::AugmentAttributes
                | ItemUseEffectDefinition::ApplyDetonation { .. }
                | ItemUseEffectDefinition::SelfLifeLoss { .. }
                | ItemUseEffectDefinition::Vengeance { .. }
                | ItemUseEffectDefinition::ProtectionFromEvil
                | ItemUseEffectDefinition::PrepareConfusingStrike
                | ItemUseEffectDefinition::IncreaseSpellLearningCapacity
                | ItemUseEffectDefinition::RemoveStatus { .. }
                | ItemUseEffectDefinition::RestoreResource { .. }
                | ItemUseEffectDefinition::RestoreResourceDice { .. }
                | ItemUseEffectDefinition::RestoreResourceFull { .. }
                | ItemUseEffectDefinition::Sequence { .. }),
                ItemUsePlan::SelfTarget,
            ) => {
                self.resolve_item_self_effect(&kind_id, &effect, events);
            }
            (
                ItemUseEffectDefinition::SelfCenteredElementalBlast {
                    base_damage,
                    damage_type,
                    radius,
                    backlash_sides,
                    backlash_bonus,
                    backlash_damage_type,
                    backlash_uses_resistance,
                },
                ItemUsePlan::SelfTarget,
            ) => {
                self.resolve_item_elemental_blast(
                    &kind_id,
                    base_damage,
                    damage_type.into(),
                    radius,
                    backlash_sides,
                    backlash_bonus,
                    backlash_damage_type.into(),
                    backlash_uses_resistance,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (ItemUseEffectDefinition::AggravateMonsters, ItemUsePlan::SelfTarget) => {
                self.resolve_item_aggravation(&kind_id, events, changed);
            }
            (ItemUseEffectDefinition::MassGenocide { power, radius }, ItemUsePlan::SelfTarget) => {
                self.resolve_item_mass_genocide(
                    &kind_id,
                    power,
                    radius,
                    events,
                    changed,
                    removed_entities,
                );
            }
            (ItemUseEffectDefinition::Genocide { power }, ItemUsePlan::GlyphGenocide { glyph }) => {
                self.resolve_item_genocide(
                    &kind_id,
                    &glyph,
                    power,
                    events,
                    changed,
                    removed_entities,
                );
            }
            (
                ItemUseEffectDefinition::CreateAdjacentTerrain { .. },
                ItemUsePlan::CreateAdjacentTerrain { replacements },
            ) => {
                let affected_positions = replacements
                    .into_iter()
                    .map(|(position, target_terrain_id)| {
                        let index = self
                            .index(position)
                            .expect("planned terrain creation must remain in bounds");
                        self.terrain[index] = target_terrain_id;
                        self.revealed_terrain.remove(&position);
                        changed.insert(position);
                        position
                    })
                    .collect::<Vec<_>>();
                if !affected_positions.is_empty() {
                    self.mark_item_aware(&kind_id);
                }
                events.push(DomainEvent::ItemCreatedAdjacentTerrain {
                    source_kind_id: kind_id.clone(),
                    display_name_key: self.item_display_name_key(&kind_id),
                    affected_positions,
                });
            }
            (
                ItemUseEffectDefinition::DestroyAdjacentTrapsAndDoors,
                ItemUsePlan::DestroyAdjacentTrapsAndDoors { replacements },
            ) => {
                let affected_positions = replacements
                    .into_iter()
                    .map(|(position, target_terrain_id)| {
                        let index = self
                            .index(position)
                            .expect("planned terrain replacement must remain in bounds");
                        self.terrain[index] = target_terrain_id;
                        self.revealed_terrain.remove(&position);
                        changed.insert(position);
                        position
                    })
                    .collect();
                self.mark_item_aware(&kind_id);
                events.push(DomainEvent::ItemDestroyedAdjacentTrapsAndDoors {
                    source_kind_id: kind_id.clone(),
                    display_name_key: self.item_display_name_key(&kind_id),
                    affected_positions,
                });
            }
            (
                ItemUseEffectDefinition::Damage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    damage_type,
                },
                ItemUsePlan::Projectile { path },
            ) => {
                let profile_id =
                    profile_id.expect("dynamic damage activation must carry a profile ID");
                let (trace, target_index) = self.trace_projectile_path(path);
                self.mark_item_aware(&kind_id);
                let Some(target_index) = target_index else {
                    events.push(DomainEvent::ItemActivationLanded {
                        source_kind_id: kind_id,
                        profile_id,
                        trace,
                    });
                    return Ok(());
                };
                let target_kind_id = self.entities[target_index].kind_id.clone();
                let target_position = self.entities[target_index].position;
                let definition = self
                    .content
                    .actor(&target_kind_id)
                    .expect("activation target definition must remain available")
                    .clone();
                let target_stats =
                    self.actor_derived_stats(&self.entities[target_index], &definition, false);
                let raw_damage = self
                    .roll_damage(damage_dice, damage_sides)
                    .saturating_add(i32::from(damage_bonus))
                    .max(0);
                let damage_type = DamageType::from(damage_type);
                let resistance = self.entities[target_index].resistances.level(damage_type);
                let damage = resolve_armored_damage(
                    raw_damage,
                    damage_type,
                    target_stats.armor_class.value,
                    resistance,
                );
                self.entities[target_index].alerted = true;
                let application = plan_damage_application(
                    &self.entities[target_index],
                    damage,
                    FatalityPolicy::AtOrBelowZero,
                );
                commit_damage_application(&mut self.entities[target_index], &application);
                changed.insert(target_position);
                self.wake_entity_after_damage(target_index, damage.applied, events);
                if application.fatal {
                    self.resolve_actor_death(
                        target_index,
                        DomainEvent::ItemActivationSlew {
                            source_kind_id: kind_id,
                            profile_id,
                            target_kind_id,
                            damage,
                            trace,
                        },
                        events,
                        changed,
                        removed_entities,
                    )?;
                } else {
                    events.push(DomainEvent::ItemActivationHit {
                        source_kind_id: kind_id,
                        profile_id,
                        target_kind_id,
                        damage,
                        trace,
                    });
                }
            }
            (
                ItemUseEffectDefinition::DispelCategory { category, damage },
                ItemUsePlan::VisibleActors { actor_ids },
            ) => {
                self.resolve_item_dispel_category(
                    &kind_id,
                    &category,
                    damage,
                    actor_ids,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                ItemUseEffectDefinition::BanishVisible { maximum_distance },
                ItemUsePlan::VisibleActors { actor_ids },
            ) => {
                self.resolve_item_banish_visible(
                    &kind_id,
                    maximum_distance,
                    actor_ids,
                    events,
                    changed,
                );
            }
            (
                ItemUseEffectDefinition::Detect {
                    subject,
                    category,
                    radius,
                    persistent,
                    through_walls,
                },
                ItemUsePlan::Detect,
            ) => {
                let (detected_positions, detected_entity_ids) = match subject {
                    AbilityDetectSubjectDefinition::Terrain => (
                        self.detect_terrain_positions(&category, radius, persistent, through_walls),
                        Vec::new(),
                    ),
                    AbilityDetectSubjectDefinition::Actor => {
                        self.detect_actor_positions(&category, radius)
                    }
                    AbilityDetectSubjectDefinition::Item => {
                        self.detect_item_positions(&category, radius, through_walls)
                    }
                };
                if persistent {
                    changed.extend(detected_positions.iter().copied());
                }
                self.mark_item_aware(&kind_id);
                let resolution = AbilityDetectResolutionDto {
                    subject: ability_detect_subject_dto(subject),
                    category,
                    radius,
                    persistent,
                    detected_positions,
                    detected_entity_ids,
                };
                if let Some(profile_id) = profile_id {
                    events.push(DomainEvent::ItemActivationDetected {
                        source_kind_id: kind_id,
                        profile_id,
                        resolution,
                    });
                } else {
                    events.push(DomainEvent::ItemDetected {
                        source_kind_id: kind_id,
                        resolution,
                    });
                }
            }
            (
                ItemUseEffectDefinition::SummonCategory {
                    count_dice,
                    count_sides,
                    count_bonus,
                    hostile,
                    group_chance_percent,
                    group_count_dice,
                    group_count_sides,
                    group_count_bonus,
                    duration_turns,
                    ..
                },
                ItemUsePlan::SummonCategory {
                    category,
                    candidate_kind_ids,
                    positions,
                },
            ) => {
                let owner_id = self.player.id.clone();
                let resolution = self.resolve_category_summon(
                    CategorySummonSpec {
                        source_id: &kind_id,
                        owner_id: &owner_id,
                        category: &category,
                        count_dice,
                        count_sides,
                        count_bonus,
                        hostile,
                        group_chance_percent,
                        group_count_dice,
                        group_count_sides,
                        group_count_bonus,
                        duration_turns,
                    },
                    candidate_kind_ids,
                    positions,
                    changed,
                );
                if !resolution.entity_ids.is_empty() {
                    self.mark_item_aware(&kind_id);
                }
                events.push(DomainEvent::ItemSummoned {
                    source_kind_id: kind_id,
                    profile_id,
                    resolution,
                });
            }
            (ItemUseEffectDefinition::IdentifyItem { full }, ItemUsePlan::Item { item_id }) => {
                self.mark_item_aware(&kind_id);
                let resolution = self.identify_item_instance(&item_id, full);
                events.push(DomainEvent::ItemIdentified {
                    source_kind_id: kind_id.clone(),
                    display_name_key: self.item_display_name_key(&kind_id),
                    resolution,
                });
            }
            (
                ItemUseEffectDefinition::EnchantItem {
                    to_hit,
                    to_damage,
                    to_armor,
                },
                ItemUsePlan::Item { item_id },
            ) => {
                self.mark_item_aware(&kind_id);
                let resolution = self.enchant_item_instance(&item_id, to_hit, to_damage, to_armor);
                events.push(DomainEvent::ItemEnchanted {
                    source_kind_id: kind_id,
                    resolution,
                });
            }
            (ItemUseEffectDefinition::CurseEquippedItem { target }, ItemUsePlan::SelfTarget) => {
                let resolution = self.curse_equipped_item(target);
                if resolution.item_id.is_some() {
                    self.mark_item_aware(&kind_id);
                }
                events.push(DomainEvent::ItemCursed {
                    source_kind_id: kind_id,
                    resolution,
                });
            }
            (
                ItemUseEffectDefinition::RemoveEquippedCurses { include_heavy },
                ItemUsePlan::SelfTarget,
            ) => {
                let resolution = self.remove_equipped_curses(include_heavy);
                if include_heavy || !resolution.removed_item_ids.is_empty() {
                    self.mark_item_aware(&kind_id);
                }
                events.push(DomainEvent::ItemCursesRemoved {
                    source_kind_id: kind_id,
                    resolution,
                });
            }
            (
                ItemUseEffectDefinition::RandomTeleport { .. },
                ItemUsePlan::RandomTeleport { candidates },
            ) => {
                let candidate_index = usize::try_from(self.rng.bounded(candidates.len() as u64))
                    .expect("bounded teleport candidate index must fit usize");
                let destination = candidates[candidate_index];
                let origin = self.player.position;
                self.mark_item_aware(&kind_id);
                events.push(DomainEvent::ItemTeleported {
                    source_kind_id: kind_id,
                    profile_id,
                    resolution: AbilityTeleportResolutionDto {
                        from: origin,
                        to: destination,
                    },
                });
                events.extend(self.relocate_player(destination, changed));
            }
            (
                ItemUseEffectDefinition::TeleportLevel,
                ItemUsePlan::TeleportLevel {
                    upward_targets,
                    downward_targets,
                },
            ) => {
                let prefer_upward = self.rng.bounded(2) == 0;
                let targets = if prefer_upward {
                    if upward_targets.is_empty() {
                        downward_targets
                    } else {
                        upward_targets
                    }
                } else if downward_targets.is_empty() {
                    upward_targets
                } else {
                    downward_targets
                };
                let target_index = if targets.len() == 1 {
                    0
                } else {
                    usize::try_from(self.rng.bounded(targets.len() as u64))
                        .expect("bounded floor target index must fit usize")
                };
                let target = targets[target_index].clone();
                let from_floor_id = self.current_floor_id.clone();
                let transition = self
                    .transition_floor(
                        target.floor_id,
                        target.arrival_connection_id,
                        target.departure_connection_id,
                        false,
                    )?
                    .expect("planned floor teleport must remain available");
                self.mark_item_aware(&kind_id);
                events.push(DomainEvent::ItemTeleportedLevel {
                    source_kind_id: kind_id,
                    from_floor_id,
                    to_floor_id: transition.to_floor_id.clone(),
                });
                self.record_floor_transition(transition, events, changed);
            }
            (
                ItemUseEffectDefinition::Recall {
                    delay_dice,
                    delay_sides,
                    delay_bonus,
                },
                ItemUsePlan::Recall(action),
            ) => {
                self.mark_item_aware(&kind_id);
                match action {
                    RecallUseAction::Cancel => {
                        self.cancel_recall();
                        events.push(DomainEvent::ItemRecallCancelled {
                            source_kind_id: kind_id,
                        });
                    }
                    RecallUseAction::Start => {
                        let rolled_delay = u16::try_from(self.roll_damage(delay_dice, delay_sides))
                            .expect("validated recall delay roll must fit u16")
                            .saturating_add(delay_bonus);
                        let delay = self.debug_recall_delay_turns.unwrap_or(rolled_delay).max(1);
                        let destination = self.start_recall(delay);
                        events.push(DomainEvent::ItemRecallStarted {
                            source_kind_id: kind_id,
                            dungeon_id: destination.dungeon_id,
                            floor_id: destination.floor_id,
                            turns: delay,
                        });
                    }
                }
            }
            (ItemUseEffectDefinition::ResetRecall, ItemUsePlan::ResetRecall(destination)) => {
                let dungeon_id = destination.dungeon_id.clone();
                let floor_id = destination.floor_id.clone();
                self.reset_recall(destination);
                self.mark_item_aware(&kind_id);
                events.push(DomainEvent::ItemRecallReset {
                    source_kind_id: kind_id,
                    dungeon_id,
                    floor_id,
                });
            }
            _ => unreachable!("validated item effect and target plan must remain compatible"),
        }
        Ok(())
    }

    fn adjacent_terrain_creation_replacements(
        &self,
        source_terrain_ids: &[String],
        target_terrain_id: &str,
    ) -> Vec<(Position, String)> {
        let occupied = self
            .entities
            .iter()
            .filter(|entity| entity.hp > 0)
            .map(|entity| entity.position)
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
        TERRAIN_INTERACTION_DIRECTIONS
            .iter()
            .filter_map(|direction| {
                let position = self.position_in_direction(*direction);
                let index = self.index(position)?;
                (!occupied.contains(&position)
                    && !connections.contains(&position)
                    && source_terrain_ids.contains(&self.terrain[index]))
                .then(|| (position, target_terrain_id.to_owned()))
            })
            .collect()
    }

    fn adjacent_trap_door_replacements(&self) -> Vec<(Position, String)> {
        TERRAIN_INTERACTION_DIRECTIONS
            .iter()
            .filter_map(|direction| {
                let position = self.position_in_direction(*direction);
                let terrain = self
                    .index(position)
                    .and_then(|index| self.content.terrain(&self.terrain[index]))?;
                let target_terrain_id = if let Some(trap) = &terrain.trap {
                    Some(trap.disarm_to_terrain_id.clone())
                } else if terrain.tags.iter().any(|tag| tag == "door") {
                    terrain.bash_to_terrain_id.clone()
                } else {
                    None
                }?;
                Some((position, target_terrain_id))
            })
            .collect()
    }

    fn resolve_item_aggravation(
        &mut self,
        source_kind_id: &str,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let origin = self.player.position;
        let sight_radius =
            u32::try_from(VISIBILITY_RADIUS).expect("positive visibility radius must fit u32");
        for index in 0..self.entities.len() {
            if self.entities[index].hp <= 0 {
                continue;
            }
            let position = self.entities[index].position;
            let distance = rfb_distance(origin, position);
            let nearby = distance < sight_radius.saturating_mul(2);
            let hostile_in_los = distance <= sight_radius
                && !self.actor_is_player_aligned(&self.entities[index])
                && has_line_of_sight(self, origin, position);
            if !nearby && !hostile_in_los {
                continue;
            }
            if nearby {
                self.entities[index].alerted = true;
                self.entities[index]
                    .statuses
                    .retain(|status| status.kind_id != STATUS_SLEEP);
            }
            if hostile_in_los {
                apply_status(
                    &mut self.entities[index].statuses,
                    StatusApplication {
                        status: StatusInstance {
                            kind_id: STATUS_HASTE.to_owned(),
                            intensity: 1,
                            remaining_ticks: 100,
                            source_id: Some(source_kind_id.to_owned()),
                            granted_resistances: BTreeMap::new(),
                            granted_brands: BTreeSet::new(),
                            granted_modifiers: StatModifiersDto::default(),
                            granted_equipment_bonuses: EquipmentBonusesDto::default(),
                            granted_status_immunities: BTreeSet::new(),
                            granted_race_id: None,
                            grants_wall_passage: false,
                            incoming_damage_percent: 100,
                        },
                        stacking: StatusStacking::Extend,
                    },
                );
            }
            changed.insert(position);
        }
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemAggravated {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
        });
    }

    fn resolve_item_mass_genocide(
        &mut self,
        source_kind_id: &str,
        power: u16,
        radius: u8,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) {
        let mut candidate_ids = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && chebyshev_distance(self.player.position, entity.position)
                        <= u32::from(radius)
            })
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        candidate_ids.sort();
        let resolution = self.resolve_genocide_candidates(
            candidate_ids,
            AbilityGenocideScopeDefinition::Nearby,
            power,
            changed,
            removed_entities,
        );
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemMassGenocide {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            removed_count: resolution.removed_entity_ids.len(),
            resisted_count: resolution.resisted_entity_ids.len(),
            fatigue_damage: resolution.fatigue_damage,
        });
    }

    fn resolve_item_genocide(
        &mut self,
        source_kind_id: &str,
        glyph: &str,
        power: u16,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) {
        let mut candidate_ids = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && self
                        .content
                        .actor(&entity.kind_id)
                        .is_some_and(|definition| definition.glyph == glyph)
            })
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        candidate_ids.sort();
        let resolution = self.resolve_genocide_candidates(
            candidate_ids,
            AbilityGenocideScopeDefinition::Glyph,
            power,
            changed,
            removed_entities,
        );
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemGenocide {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            glyph: glyph.to_owned(),
            removed_count: resolution.removed_entity_ids.len(),
            resisted_count: resolution.resisted_entity_ids.len(),
            fatigue_damage: resolution.fatigue_damage,
        });
    }

    fn item_visible_actor_ids(&self) -> Vec<String> {
        self.entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && self.is_visible(entity.position)
                    && has_line_of_effect(self, self.player.position, entity.position)
            })
            .map(|entity| entity.id.clone())
            .collect()
    }

    fn resolve_item_banish_visible(
        &mut self,
        source_kind_id: &str,
        maximum_distance: u16,
        actor_ids: Vec<String>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        if actor_ids.is_empty() {
            events.push(DomainEvent::ItemBanishmentNoEffect {
                source_kind_id: source_kind_id.to_owned(),
            });
            return;
        }

        let mut noticed = false;
        for actor_id in actor_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == actor_id && entity.hp > 0)
            else {
                continue;
            };
            let definition = self
                .content
                .actor(&self.entities[index].kind_id)
                .expect("item banishment target definition must remain available")
                .clone();
            let guardian = definition.tags.iter().any(|tag| tag == "guardian");
            let teleport_resistance = definition.tags.iter().any(|tag| tag == "resist-teleport");
            let protected_resistance = definition
                .tags
                .iter()
                .any(|tag| matches!(tag.as_str(), "unique" | "resist-all"));
            let resisted = guardian
                || (teleport_resistance
                    && (protected_resistance
                        || definition.level
                            > u32::try_from(self.rng.bounded(100) + 1)
                                .expect("bounded teleport resistance roll must fit u32")));
            if resisted {
                events.push(DomainEvent::ItemBanishmentResisted {
                    source_kind_id: source_kind_id.to_owned(),
                    target_kind_id: definition.id,
                });
                continue;
            }

            noticed = true;
            let destinations = self.item_banishment_destinations(index, maximum_distance);
            if destinations.is_empty() {
                events.push(DomainEvent::ItemBanishmentNoSpace {
                    source_kind_id: source_kind_id.to_owned(),
                    target_kind_id: definition.id,
                });
                continue;
            }
            let choice = usize::try_from(self.rng.bounded(
                u64::try_from(destinations.len()).expect("banishment candidate count must fit u64"),
            ))
            .expect("bounded banishment candidate index must fit usize");
            let from = self.entities[index].position;
            let to = destinations[choice];
            self.entities[index].position = to;
            changed.insert(from);
            changed.insert(to);
            events.push(DomainEvent::ItemBanishedActor {
                source_kind_id: source_kind_id.to_owned(),
                target_kind_id: definition.id,
                resolution: MonsterDisplacementResolutionDto { actor_id, from, to },
            });
        }
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
    }

    fn item_banishment_destinations(
        &self,
        source_index: usize,
        maximum_distance: u16,
    ) -> Vec<Position> {
        let origin = self.entities[source_index].position;
        let mut maximum = u32::from(maximum_distance).min(200);
        let mut minimum = maximum / 2;
        loop {
            let candidates = self.displacement_destinations(source_index, |position| {
                let distance = origin
                    .x
                    .abs_diff(position.x)
                    .max(origin.y.abs_diff(position.y));
                (minimum..=maximum).contains(&distance)
            });
            if !candidates.is_empty() || (maximum == 200 && minimum == 0) {
                return candidates;
            }
            maximum = maximum.saturating_mul(2).min(200);
            minimum /= 2;
        }
    }

    fn item_category_summon_plan(&self, effect: &ItemUseEffectDefinition) -> ItemUsePlan {
        let ItemUseEffectDefinition::SummonCategory {
            selector,
            maximum_level_source,
            count_dice,
            count_sides,
            count_bonus,
            group_chance_percent,
            group_count_dice,
            group_count_sides,
            group_count_bonus,
            allow_unique,
            radius,
            ..
        } = effect
        else {
            unreachable!("item summon planning requires a category summon effect");
        };
        let resolved_kin_category = self
            .character_definitions()
            .and_then(|(_, race, _, _)| race.kin_category.as_deref());
        let category = match selector {
            ItemSummonSelectorDefinition::AnyMonster => "any-monster",
            ItemSummonSelectorDefinition::Category { category } => category,
            ItemSummonSelectorDefinition::PlayerKin => {
                resolved_kin_category.unwrap_or("player-kin")
            }
        };
        let maximum_level = match maximum_level_source {
            ItemSummonLevelSourceDefinition::DungeonDepth => {
                self.floor_depth(&self.current_floor_id).max(1)
            }
            ItemSummonLevelSourceDefinition::PlayerLevel => self.progress.level.max(1),
        };
        let candidate_kind_ids = if category == "player-kin" {
            Vec::new()
        } else {
            self.summon_category_candidate_kind_ids(category, None, maximum_level, *allow_unique)
        };
        let normal_maximum =
            usize::from(*count_dice) * usize::from(*count_sides) + usize::from(*count_bonus);
        let group_maximum = if *group_chance_percent == 0 {
            0
        } else {
            usize::from(*group_count_dice) * usize::from(*group_count_sides)
                + usize::from(*group_count_bonus)
        };
        let positions = self
            .open_positions_around(self.player.position, *radius)
            .into_iter()
            .take(normal_maximum.max(group_maximum))
            .collect();
        ItemUsePlan::SummonCategory {
            category: category.to_owned(),
            candidate_kind_ids,
            positions,
        }
    }

    fn item_is_valid_identify_target(&self, source_item_id: &str, target_item_id: &str) -> bool {
        source_item_id != target_item_id
            && self.items.iter().any(|item| {
                item.id == target_item_id
                    && item.quantity > 0
                    && match &item.location {
                        ItemLocation::Inventory | ItemLocation::Equipped { .. } => true,
                        ItemLocation::Ground(position) => *position == self.player.position,
                        ItemLocation::CarriedBy { .. } => false,
                    }
            })
    }

    fn item_is_valid_enchant_target(
        &self,
        source_item_id: &str,
        target_item_id: &str,
        effect: &ItemUseEffectDefinition,
    ) -> bool {
        if source_item_id == target_item_id {
            return false;
        }
        let ItemUseEffectDefinition::EnchantItem {
            to_hit,
            to_damage,
            to_armor,
        } = effect
        else {
            return false;
        };
        self.items.iter().any(|item| {
            if item.id != target_item_id
                || item.quantity == 0
                || !matches!(
                    &item.location,
                    ItemLocation::Inventory | ItemLocation::Equipped { .. }
                ) && !matches!(
                    &item.location,
                    ItemLocation::Ground(position) if *position == self.player.position
                )
            {
                return false;
            }
            let Some(definition) = self.content.item(&item.kind_id) else {
                return false;
            };
            if definition.tags.iter().any(|tag| tag == "no-enchant") {
                return false;
            }
            if to_armor.is_some() {
                definition.tags.iter().any(|tag| tag == "armor")
            } else if to_hit.is_some() || to_damage.is_some() {
                definition
                    .tags
                    .iter()
                    .any(|tag| matches!(tag.as_str(), "weapon" | "launcher" | "ammunition"))
            } else {
                false
            }
        })
    }

    fn identify_item_instance(&mut self, item_id: &str, full: bool) -> ItemIdentifyResolutionDto {
        let item = self
            .items
            .iter()
            .find(|item| item.id == item_id)
            .expect("planned identify target must remain available");
        let item_kind_id = item.kind_id.clone();
        let affix_ids = item
            .affix_ids
            .iter()
            .cloned()
            .chain(
                item.rolled_affixes
                    .iter()
                    .map(|rolled| rolled.affix_id.clone()),
            )
            .collect::<BTreeSet<_>>();
        let awareness_before = self.item_knowledge_dto(&item_kind_id);
        let property_before = self.item_property_knowledge.get(item_id).cloned();
        self.mark_item_aware(&item_kind_id);
        let knowledge = self
            .item_property_knowledge
            .entry(item_id.to_owned())
            .or_default();
        knowledge.appraised = true;
        if full {
            knowledge.identified = true;
            knowledge.known_affix_ids.extend(affix_ids);
        }
        let changed = awareness_before != self.item_knowledge_dto(&item_kind_id)
            || property_before.as_ref() != self.item_property_knowledge.get(item_id);
        ItemIdentifyResolutionDto {
            item_id: item_id.to_owned(),
            item_kind_id,
            full,
            changed,
        }
    }

    fn enchant_item_instance(
        &mut self,
        item_id: &str,
        to_hit: Option<ItemEnchantmentRollDefinition>,
        to_damage: Option<ItemEnchantmentRollDefinition>,
        to_armor: Option<ItemEnchantmentRollDefinition>,
    ) -> ItemEnchantmentResolutionDto {
        let item = self
            .items
            .iter()
            .find(|item| item.id == item_id)
            .expect("planned enchantment target must remain available");
        let item_kind_id = item.kind_id.clone();
        let quantity = item.quantity;
        let definition = self
            .content
            .item(&item_kind_id)
            .expect("planned enchantment kind must remain available");
        let artifact = definition.tags.iter().any(|tag| tag == "artifact");
        let ammunition = definition.tags.iter().any(|tag| tag == "ammunition");
        let before = item.enchantments;

        let hit_attempts = self.roll_item_enchantment_attempts(to_hit);
        let damage_attempts = self.roll_item_enchantment_attempts(to_damage);
        let armor_attempts = self.roll_item_enchantment_attempts(to_armor);
        let to_hit = self.resolve_item_enchantment_component(
            before.to_hit,
            hit_attempts,
            quantity,
            ammunition,
            artifact,
        );
        let to_damage = self.resolve_item_enchantment_component(
            before.to_damage,
            damage_attempts,
            quantity,
            ammunition,
            artifact,
        );
        let to_armor = self.resolve_item_enchantment_component(
            before.to_armor,
            armor_attempts,
            quantity,
            ammunition,
            artifact,
        );
        self.items
            .iter_mut()
            .find(|item| item.id == item_id)
            .expect("planned enchantment target must remain available")
            .enchantments = ItemEnchantmentsDto {
            to_hit: to_hit.after,
            to_damage: to_damage.after,
            to_armor: to_armor.after,
        };
        ItemEnchantmentResolutionDto {
            item_id: item_id.to_owned(),
            item_kind_id,
            to_hit,
            to_damage,
            to_armor,
        }
    }

    fn curse_equipped_item(&mut self, target: ItemCurseTargetDefinition) -> ItemCurseResolutionDto {
        let mut candidates = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let ItemLocation::Equipped { slot_id } = &item.location else {
                    return None;
                };
                let definition = self.content.item(&item.kind_id)?;
                let matches_target = match target {
                    ItemCurseTargetDefinition::Weapon => {
                        definition.tags.iter().any(|tag| tag == "weapon")
                    }
                    ItemCurseTargetDefinition::Armor => {
                        definition.tags.iter().any(|tag| tag == "armor")
                    }
                };
                matches_target.then(|| (slot_id.clone(), item.id.clone(), index))
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
        if candidates.is_empty() {
            return ItemCurseResolutionDto {
                item_id: None,
                item_kind_id: None,
                before: None,
                after: None,
                resisted: false,
            };
        }
        let candidate_index = if candidates.len() == 1 {
            0
        } else {
            usize::try_from(self.rng.bounded(candidates.len() as u64))
                .expect("curse target index must fit usize")
        };
        let item_index = candidates[candidate_index].2;
        let item_id = self.items[item_index].id.clone();
        let item_kind_id = self.items[item_index].kind_id.clone();
        let artifact = self
            .content
            .item(&item_kind_id)
            .is_some_and(|definition| definition.tags.iter().any(|tag| tag == "artifact"));
        let resisted = artifact
            && if self.debug_item_curses_land {
                false
            } else if self.debug_item_curses_resisted {
                true
            } else {
                self.rng.bounded(100) < 50
            };
        let before = self.items[item_index].curse;
        if !resisted {
            self.items[item_index].curse =
                Some(before.map_or(ItemCurseSeverityDto::Normal, |severity| {
                    severity.max(ItemCurseSeverityDto::Normal)
                }));
        }
        ItemCurseResolutionDto {
            item_id: Some(item_id),
            item_kind_id: Some(item_kind_id),
            before,
            after: self.items[item_index].curse,
            resisted,
        }
    }

    fn remove_equipped_curses(&mut self, include_heavy: bool) -> ItemCurseRemovalResolutionDto {
        let mut removed_item_ids = Vec::new();
        let mut retained_permanent_item_ids = Vec::new();
        for item in &mut self.items {
            if !matches!(item.location, ItemLocation::Equipped { .. }) {
                continue;
            }
            match item.curse {
                Some(ItemCurseSeverityDto::Normal) => {
                    item.curse = None;
                    removed_item_ids.push(item.id.clone());
                }
                Some(ItemCurseSeverityDto::Heavy) if include_heavy => {
                    item.curse = None;
                    removed_item_ids.push(item.id.clone());
                }
                Some(ItemCurseSeverityDto::Permanent) => {
                    retained_permanent_item_ids.push(item.id.clone());
                }
                Some(ItemCurseSeverityDto::Heavy) | None => {}
            }
        }
        removed_item_ids.sort();
        retained_permanent_item_ids.sort();
        ItemCurseRemovalResolutionDto {
            include_heavy,
            removed_item_ids,
            retained_permanent_item_ids,
        }
    }

    fn roll_item_enchantment_attempts(
        &mut self,
        roll: Option<ItemEnchantmentRollDefinition>,
    ) -> u16 {
        let Some(roll) = roll else {
            return 0;
        };
        let rolled = if roll.dice == 0 {
            0
        } else {
            u16::try_from(self.roll_damage(roll.dice, roll.sides))
                .expect("validated enchantment roll must fit u16")
        };
        rolled.saturating_add(roll.bonus)
    }

    fn resolve_item_enchantment_component(
        &mut self,
        before: u16,
        attempts: u16,
        quantity: u32,
        ammunition: bool,
        artifact: bool,
    ) -> ItemEnchantmentComponentResolutionDto {
        const FAILURE_PER_THOUSAND: [u16; 16] = [
            5, 10, 50, 100, 200, 300, 400, 500, 650, 800, 950, 987, 993, 995, 998, 1000,
        ];
        let mut after = before;
        let pile_probability = if ammunition {
            u64::from(quantity).saturating_mul(100) / 20
        } else {
            u64::from(quantity).saturating_mul(100)
        }
        .max(1);
        for _ in 0..attempts {
            if self.rng.bounded(pile_probability) >= 100 {
                continue;
            }
            let failure = FAILURE_PER_THOUSAND
                .get(usize::from(after))
                .copied()
                .unwrap_or(1000);
            if self.rng.bounded(1000).saturating_add(1) <= u64::from(failure) {
                continue;
            }
            if artifact && self.rng.bounded(100) >= 50 {
                continue;
            }
            after = after.saturating_add(1).min(15);
        }
        ItemEnchantmentComponentResolutionDto {
            attempts,
            successes: after.saturating_sub(before),
            before,
            after,
        }
    }

    fn resolve_item_self_effect(
        &mut self,
        source_kind_id: &str,
        effect: &ItemUseEffectDefinition,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        match effect {
            ItemUseEffectDefinition::Heal { amount } => {
                let amount = i32::try_from(*amount).expect("validated healing amount must fit i32");
                self.resolve_item_healing(source_kind_id, amount, events)
            }
            ItemUseEffectDefinition::HealDice { dice, sides } => {
                let amount = self.roll_damage(*dice, *sides);
                self.resolve_item_healing(source_kind_id, amount, events)
            }
            ItemUseEffectDefinition::Bless {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => {
                self.resolve_item_blessing(
                    source_kind_id,
                    *duration_dice,
                    *duration_sides,
                    *duration_bonus,
                    events,
                );
                true
            }
            ItemUseEffectDefinition::ApplySlowness {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => self.resolve_item_slowness(
                source_kind_id,
                *duration_dice,
                *duration_sides,
                *duration_bonus,
                events,
            ),
            ItemUseEffectDefinition::ApplySpeed {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => self.resolve_item_speed(
                source_kind_id,
                *duration_dice,
                *duration_sides,
                *duration_bonus,
                events,
            ),
            ItemUseEffectDefinition::ApplyHeroism {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => self.resolve_item_heroism(
                source_kind_id,
                *duration_dice,
                *duration_sides,
                *duration_bonus,
                events,
            ),
            ItemUseEffectDefinition::ApplyBerserkStrength {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => self.resolve_item_berserk_strength(
                source_kind_id,
                *duration_dice,
                *duration_sides,
                *duration_bonus,
                events,
            ),
            ItemUseEffectDefinition::ApplyPoeticInspiration {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => self.resolve_item_poetic_inspiration(
                source_kind_id,
                *duration_dice,
                *duration_sides,
                *duration_bonus,
                events,
            ),
            ItemUseEffectDefinition::ApplyStoneSkin {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => self.resolve_item_stone_skin(
                source_kind_id,
                *duration_dice,
                *duration_sides,
                *duration_bonus,
                events,
            ),
            ItemUseEffectDefinition::RestoreLifeLevels { life_force_amount } => {
                self.resolve_item_restore_life_levels(source_kind_id, *life_force_amount, events)
            }
            ItemUseEffectDefinition::RestoreAllAttributes => {
                let noticed = self.restore_all_player_attributes();
                if noticed {
                    self.mark_item_aware(source_kind_id);
                }
                events.push(DomainEvent::ItemRestorationResolved {
                    source_kind_id: source_kind_id.to_owned(),
                    display_name_key: self.item_display_name_key(source_kind_id),
                    noticed,
                });
                noticed
            }
            ItemUseEffectDefinition::RestoreAllVitality { life_force_amount } => {
                let attributes_restored = self.restore_all_player_attributes();
                let vitality_restored =
                    self.restore_player_experience_and_life_force(*life_force_amount, events);
                let noticed = attributes_restored || vitality_restored;
                if noticed {
                    self.mark_item_aware(source_kind_id);
                }
                events.push(DomainEvent::ItemRestorationResolved {
                    source_kind_id: source_kind_id.to_owned(),
                    display_name_key: self.item_display_name_key(source_kind_id),
                    noticed,
                });
                noticed
            }
            ItemUseEffectDefinition::ApplyRestorativeFeast {
                healing_dice,
                healing_sides,
            } => {
                if let Some(index) = self
                    .player
                    .statuses
                    .iter()
                    .position(|status| status.kind_id == STATUS_POISON)
                {
                    let before = self.player.statuses[index].remaining_ticks;
                    let reduction = (before / 5).max(100);
                    let after = before.saturating_sub(reduction);
                    if after == 0 {
                        self.player.statuses.remove(index);
                    } else {
                        self.player.statuses[index].remaining_ticks = after;
                    }
                }
                let healing = self.roll_damage(*healing_dice, *healing_sides);
                let max_hp = self.effective_player_max_hp();
                let player = &mut self.player;
                apply_effect(
                    &mut EffectTarget {
                        hp: &mut player.hp,
                        max_hp,
                        resistances: &player.resistances,
                        statuses: &mut player.statuses,
                    },
                    EffectSpec::Heal { amount: healing },
                );
                self.restore_all_player_attributes();
                self.restore_player_experience_and_life_force(0, events);
                self.mark_item_aware(source_kind_id);
                events.push(DomainEvent::ItemRestorationResolved {
                    source_kind_id: source_kind_id.to_owned(),
                    display_name_key: self.item_display_name_key(source_kind_id),
                    noticed: true,
                });
                true
            }
            ItemUseEffectDefinition::ApplyLifeRestoration {
                healing_amount,
                life_force_amount,
            } => {
                self.restore_player_experience_and_life_force(*life_force_amount, events);
                self.player.statuses.retain(|status| {
                    !matches!(
                        status.kind_id.as_str(),
                        STATUS_POISON
                            | STATUS_BLINDNESS
                            | STATUS_CONFUSION
                            | STATUS_STUN
                            | STATUS_BLEEDING
                            | STATUS_SLOW
                            | "rfb.status.berserk"
                    )
                });
                self.restore_all_player_attributes();
                let amount = i32::try_from(*healing_amount)
                    .expect("validated life restoration amount must fit i32");
                let max_hp = self.effective_player_max_hp();
                let player = &mut self.player;
                apply_effect(
                    &mut EffectTarget {
                        hp: &mut player.hp,
                        max_hp,
                        resistances: &player.resistances,
                        statuses: &mut player.statuses,
                    },
                    EffectSpec::Heal { amount },
                );
                self.mark_item_aware(source_kind_id);
                events.push(DomainEvent::ItemRestorationResolved {
                    source_kind_id: source_kind_id.to_owned(),
                    display_name_key: self.item_display_name_key(source_kind_id),
                    noticed: true,
                });
                true
            }
            ItemUseEffectDefinition::DrainAttribute { attribute } => self
                .resolve_item_drain_attribute(
                    source_kind_id,
                    Self::item_attribute_kind(attribute),
                    events,
                ),
            ItemUseEffectDefinition::RestoreAttribute { attribute } => self
                .resolve_item_restore_attribute(
                    source_kind_id,
                    Self::item_attribute_kind(attribute),
                    events,
                ),
            ItemUseEffectDefinition::IncreaseAttribute { attribute } => self
                .resolve_item_increase_attributes(
                    source_kind_id,
                    &[Self::item_attribute_kind(attribute)],
                    events,
                ),
            ItemUseEffectDefinition::AugmentAttributes => self.resolve_item_increase_attributes(
                source_kind_id,
                &[
                    AttributeKind::Strength,
                    AttributeKind::Intelligence,
                    AttributeKind::Wisdom,
                    AttributeKind::Dexterity,
                    AttributeKind::Constitution,
                    AttributeKind::Charisma,
                ],
                events,
            ),
            ItemUseEffectDefinition::ApplyThermalResistance {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => self.resolve_item_thermal_resistance(
                source_kind_id,
                *duration_dice,
                *duration_sides,
                *duration_bonus,
                events,
            ),
            ItemUseEffectDefinition::ApplyBasicResistance {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => {
                self.resolve_item_basic_resistance(
                    source_kind_id,
                    *duration_dice,
                    *duration_sides,
                    *duration_bonus,
                    events,
                );
                true
            }
            ItemUseEffectDefinition::ApplyPoison {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => self.resolve_item_poison(
                source_kind_id,
                *duration_dice,
                *duration_sides,
                *duration_bonus,
                events,
            ),
            ItemUseEffectDefinition::ApplyBlindness {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => self.resolve_item_blindness(
                source_kind_id,
                *duration_dice,
                *duration_sides,
                *duration_bonus,
                events,
            ),
            ItemUseEffectDefinition::ApplyDetonation {
                damage_dice,
                damage_sides,
                stun_ticks,
                bleeding_ticks,
            } => {
                self.resolve_item_detonation(
                    source_kind_id,
                    *damage_dice,
                    *damage_sides,
                    *stun_ticks,
                    *bleeding_ticks,
                    events,
                );
                true
            }
            ItemUseEffectDefinition::SelfLifeLoss { amount } => {
                self.resolve_item_life_loss(source_kind_id, *amount, events);
                true
            }
            ItemUseEffectDefinition::Vengeance {
                duration_dice,
                duration_sides,
                duration_bonus,
            } => {
                self.resolve_item_vengeance(
                    source_kind_id,
                    *duration_dice,
                    *duration_sides,
                    *duration_bonus,
                    events,
                );
                true
            }
            ItemUseEffectDefinition::ProtectionFromEvil => {
                self.resolve_item_protection_from_evil(source_kind_id, events);
                true
            }
            ItemUseEffectDefinition::PrepareConfusingStrike => {
                self.confusing_strike_ready = true;
                self.mark_item_aware(source_kind_id);
                events.push(DomainEvent::ItemConfusingStrikePrepared {
                    source_kind_id: source_kind_id.to_owned(),
                    display_name_key: self.item_display_name_key(source_kind_id),
                });
                true
            }
            ItemUseEffectDefinition::IncreaseSpellLearningCapacity => {
                let before = self
                    .casting_profile()
                    .map_or(0, |profile| self.ability_learning_capacity(profile));
                if self.uses_spell_scrolls() {
                    self.bonus_spell_learning_capacity =
                        self.bonus_spell_learning_capacity.saturating_add(1);
                }
                let after = self
                    .casting_profile()
                    .map_or(0, |profile| self.ability_learning_capacity(profile));
                self.mark_item_aware(source_kind_id);
                events.push(DomainEvent::ItemSpellLearningCapacityChanged {
                    source_kind_id: source_kind_id.to_owned(),
                    display_name_key: self.item_display_name_key(source_kind_id),
                    before,
                    after,
                });
                true
            }
            ItemUseEffectDefinition::RemoveStatus { status_kind_id } => {
                let max_hp = self.effective_player_max_hp();
                let player = &mut self.player;
                let outcome = apply_effect(
                    &mut EffectTarget {
                        hp: &mut player.hp,
                        max_hp,
                        resistances: &player.resistances,
                        statuses: &mut player.statuses,
                    },
                    EffectSpec::RemoveStatus {
                        kind_id: status_kind_id.clone(),
                    },
                );
                let EffectOutcome::StatusRemoved { removed, .. } = outcome else {
                    unreachable!("status removal must produce a status outcome");
                };
                if removed {
                    self.mark_item_aware(source_kind_id);
                }
                events.push(DomainEvent::ItemStatusRemoved {
                    source_kind_id: source_kind_id.to_owned(),
                    display_name_key: self.item_display_name_key(source_kind_id),
                    status_kind_id: status_kind_id.clone(),
                    removed,
                });
                removed
            }
            ItemUseEffectDefinition::RestoreResource {
                resource_id,
                amount,
            } => self.resolve_item_resource_restoration(
                source_kind_id,
                resource_id,
                *amount,
                false,
                events,
            ),
            ItemUseEffectDefinition::RestoreResourceDice {
                resource_id,
                dice,
                sides,
                bonus,
            } => {
                let rolled = u32::try_from(self.roll_damage(*dice, *sides))
                    .expect("validated resource restoration roll must fit u32")
                    .saturating_add(*bonus);
                self.resolve_item_resource_restoration(
                    source_kind_id,
                    resource_id,
                    rolled,
                    false,
                    events,
                )
            }
            ItemUseEffectDefinition::RestoreResourceFull { resource_id } => {
                self.resolve_item_resource_restoration(source_kind_id, resource_id, 0, true, events)
            }
            ItemUseEffectDefinition::Sequence { effects } => {
                let mut noticed = false;
                for effect in effects {
                    noticed =
                        self.resolve_item_self_effect(source_kind_id, effect, events) || noticed;
                }
                noticed
            }
            ItemUseEffectDefinition::Damage { .. }
            | ItemUseEffectDefinition::SelfCenteredElementalBlast { .. }
            | ItemUseEffectDefinition::AggravateMonsters
            | ItemUseEffectDefinition::MassGenocide { .. }
            | ItemUseEffectDefinition::Genocide { .. }
            | ItemUseEffectDefinition::RechargeFromDevice { .. }
            | ItemUseEffectDefinition::CreateAdjacentTerrain { .. }
            | ItemUseEffectDefinition::DestroyAdjacentTrapsAndDoors
            | ItemUseEffectDefinition::DispelCategory { .. }
            | ItemUseEffectDefinition::BanishVisible { .. }
            | ItemUseEffectDefinition::Detect { .. }
            | ItemUseEffectDefinition::IdentifyItem { .. }
            | ItemUseEffectDefinition::EnchantItem { .. }
            | ItemUseEffectDefinition::CurseEquippedItem { .. }
            | ItemUseEffectDefinition::RemoveEquippedCurses { .. }
            | ItemUseEffectDefinition::SummonCategory { .. }
            | ItemUseEffectDefinition::RandomTeleport { .. }
            | ItemUseEffectDefinition::TeleportLevel
            | ItemUseEffectDefinition::Recall { .. }
            | ItemUseEffectDefinition::ResetRecall => {
                unreachable!("projected item effects cannot resolve as self restoration")
            }
        }
    }

    fn resolve_item_protection_from_evil(
        &mut self,
        source_kind_id: &str,
        events: &mut Vec<DomainEvent>,
    ) {
        let duration = u32::from(self.progress.level)
            .saturating_mul(3)
            .saturating_add(
                u32::try_from(self.roll_damage(1, 25))
                    .expect("protection from evil duration must fit u32"),
            );
        let resolution = apply_ability_status_effect(
            &mut self.player,
            source_kind_id,
            0,
            STATUS_PROTECTION_FROM_EVIL,
            1,
            duration,
            0,
            1,
            AbilityStatusStackingDefinition::Extend,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers::default(),
            &EquipmentBonuses::default(),
            &BTreeSet::new(),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemProtectionFromEvil {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![resolution],
            },
        });
    }

    fn resolve_item_blessing(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) {
        let resolution = apply_ability_status_effect(
            &mut self.player,
            source_kind_id,
            0,
            "rfb.status.blessed",
            1,
            duration_bonus,
            duration_dice,
            duration_sides,
            AbilityStatusStackingDefinition::Extend,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers {
                defense: 5,
                ..StatModifiers::default()
            },
            &EquipmentBonuses {
                melee_skill: 10,
                ranged_skill: 10,
                ..EquipmentBonuses::default()
            },
            &BTreeSet::new(),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        let duration = match &resolution {
            AbilityEffectResolutionDto::ApplyStatus {
                applied_duration_ticks,
                ..
            } => *applied_duration_ticks,
            _ => unreachable!("blessing must produce a status application resolution"),
        };
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemBlessed {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![resolution],
            },
        });
    }

    fn resolve_item_slowness(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let duration_sides =
            u16::try_from(duration_sides).expect("validated slowness die sides must fit u16");
        let duration = u32::try_from(self.roll_damage(duration_dice, duration_sides))
            .expect("validated slowness duration must fit u32")
            .saturating_add(duration_bonus);
        let change = if self.player_status_immunities().contains(STATUS_SLOW) {
            StatusChange::Unchanged
        } else {
            apply_status(
                &mut self.player.statuses,
                StatusApplication {
                    status: StatusInstance {
                        kind_id: STATUS_SLOW.to_owned(),
                        intensity: 1,
                        remaining_ticks: duration,
                        source_id: Some(source_kind_id.to_owned()),
                        granted_resistances: BTreeMap::new(),
                        granted_brands: BTreeSet::new(),
                        granted_modifiers: StatModifiersDto::default(),
                        granted_equipment_bonuses: EquipmentBonusesDto::default(),
                        granted_status_immunities: BTreeSet::new(),
                        granted_race_id: None,
                        grants_wall_passage: false,
                        incoming_damage_percent: 100,
                    },
                    stacking: StatusStacking::KeepStrongest,
                },
            )
        };
        let noticed = matches!(change, StatusChange::Added);
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemSlownessResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            noticed,
        });
        noticed
    }

    fn resolve_item_speed(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let already_hasted = self
            .player
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_HASTE);
        let duration = if already_hasted {
            5
        } else {
            let duration_sides =
                u16::try_from(duration_sides).expect("validated speed die sides must fit u16");
            u32::try_from(self.roll_damage(duration_dice, duration_sides))
                .expect("validated speed duration must fit u32")
                .saturating_add(duration_bonus)
        };
        let change = apply_status(
            &mut self.player.statuses,
            StatusApplication {
                status: StatusInstance {
                    kind_id: STATUS_HASTE.to_owned(),
                    intensity: 1,
                    remaining_ticks: duration,
                    source_id: Some(source_kind_id.to_owned()),
                    granted_resistances: BTreeMap::new(),
                    granted_brands: BTreeSet::new(),
                    granted_modifiers: StatModifiersDto::default(),
                    granted_equipment_bonuses: EquipmentBonusesDto::default(),
                    granted_status_immunities: BTreeSet::new(),
                    granted_race_id: None,
                    grants_wall_passage: false,
                    incoming_damage_percent: 100,
                },
                stacking: StatusStacking::Extend,
            },
        );
        let noticed = matches!(change, StatusChange::Added);
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemSpeedResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
        });
        noticed
    }

    fn resolve_item_heroism(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let resolution = apply_ability_status_effect(
            &mut self.player,
            source_kind_id,
            0,
            "rfb.status.hero",
            1,
            duration_bonus,
            duration_dice,
            duration_sides,
            AbilityStatusStackingDefinition::Extend,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers {
                max_hp: 10,
                ..StatModifiers::default()
            },
            &EquipmentBonuses {
                melee_skill: 12,
                ranged_skill: 12,
                ..EquipmentBonuses::default()
            },
            &BTreeSet::from([STATUS_FEAR.to_owned()]),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        let (duration, noticed) = match resolution {
            AbilityEffectResolutionDto::ApplyStatus {
                applied_duration_ticks,
                change,
                ..
            } => (
                applied_duration_ticks,
                matches!(change, AbilityStatusChangeDto::Added),
            ),
            _ => unreachable!("heroism must produce a status application resolution"),
        };
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemHeroismResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            noticed,
        });
        noticed
    }

    fn resolve_item_berserk_strength(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let resolution = apply_ability_status_effect(
            &mut self.player,
            source_kind_id,
            0,
            "rfb.status.berserk",
            1,
            duration_bonus,
            duration_dice,
            duration_sides,
            AbilityStatusStackingDefinition::Extend,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers {
                defense: -10,
                max_hp: 30,
                ..StatModifiers::default()
            },
            &EquipmentBonuses {
                melee_skill: 12,
                melee_damage: 3 + i32::from(self.progress.level / 5),
                ranged_skill: -12,
                throwing_skill: -20,
                device_skill: -20,
                saving_throw_skill: -30,
                stealth_skill: -7,
                search_skill: -15,
                perception_skill: -15,
                digging_skill: 30,
                ..EquipmentBonuses::default()
            },
            &BTreeSet::from([STATUS_FEAR.to_owned()]),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        let (duration, status_noticed) = match resolution {
            AbilityEffectResolutionDto::ApplyStatus {
                applied_duration_ticks,
                change,
                ..
            } => (
                applied_duration_ticks,
                matches!(change, AbilityStatusChangeDto::Added),
            ),
            _ => unreachable!("berserk strength must produce a status application resolution"),
        };
        if status_noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemBerserkStrengthResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            noticed: status_noticed,
        });
        let healed = self.resolve_item_healing(source_kind_id, 30, events);
        status_noticed || healed
    }

    fn resolve_item_poetic_inspiration(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let resolution = apply_ability_status_effect(
            &mut self.player,
            source_kind_id,
            0,
            "rfb.status.poetic-inspiration",
            1,
            duration_bonus,
            duration_dice,
            duration_sides,
            AbilityStatusStackingDefinition::Extend,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers {
                wisdom: 5,
                charisma: 5,
                ..StatModifiers::default()
            },
            &EquipmentBonuses::default(),
            &BTreeSet::new(),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        let (duration, noticed) = match resolution {
            AbilityEffectResolutionDto::ApplyStatus {
                applied_duration_ticks,
                change,
                ..
            } => (
                applied_duration_ticks,
                matches!(change, AbilityStatusChangeDto::Added),
            ),
            _ => unreachable!("poetic inspiration must produce a status application resolution"),
        };
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemPoeticInspirationResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            noticed,
        });
        noticed
    }

    fn resolve_item_stone_skin(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let defense = 10 + 40 * i32::from(self.progress.level) / 50;
        let resolution = apply_ability_status_effect(
            &mut self.player,
            source_kind_id,
            0,
            "rfb.status.stone-skin",
            1,
            duration_bonus,
            duration_dice,
            duration_sides,
            AbilityStatusStackingDefinition::KeepStrongest,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers {
                defense,
                ..StatModifiers::default()
            },
            &EquipmentBonuses::default(),
            &BTreeSet::new(),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        let (duration, noticed) = match resolution {
            AbilityEffectResolutionDto::ApplyStatus {
                applied_duration_ticks,
                change,
                ..
            } => (
                applied_duration_ticks,
                matches!(change, AbilityStatusChangeDto::Added),
            ),
            _ => unreachable!("stone skin must produce a status application resolution"),
        };
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemStoneSkinResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            noticed,
        });
        noticed
    }

    fn resolve_item_restore_life_levels(
        &mut self,
        source_kind_id: &str,
        life_force_amount: u16,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let noticed = self.restore_player_experience_and_life_force(life_force_amount, events);
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemRestoreLifeLevelsResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            noticed,
        });
        noticed
    }

    fn restore_all_player_attributes(&mut self) -> bool {
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let mut restored = false;
        for attribute in [
            AttributeKind::Strength,
            AttributeKind::Intelligence,
            AttributeKind::Wisdom,
            AttributeKind::Dexterity,
            AttributeKind::Constitution,
            AttributeKind::Charisma,
        ] {
            restored = self.progress.restore_attribute(attribute) || restored;
        }
        if restored {
            self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        }
        restored
    }

    fn restore_player_experience_and_life_force(
        &mut self,
        life_force_amount: u16,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let experience_before = self.progress.experience;
        let life_force_before = self.progress.life_force;
        self.progress.experience = self.progress.maximum_experience;
        self.apply_player_experience(0, events);
        self.progress.life_force = self
            .progress
            .life_force
            .saturating_add(life_force_amount)
            .min(1_000);
        self.progress.experience != experience_before
            || self.progress.life_force != life_force_before
    }

    fn resolve_item_drain_attribute(
        &mut self,
        source_kind_id: &str,
        attribute: AttributeKind,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        if self
            .player_equipment_passives()
            .contains(&attribute_sustain_passive(attribute))
        {
            let value = self.progress.attributes.value(attribute);
            self.mark_item_aware(source_kind_id);
            events.push(DomainEvent::ItemAttributeChanged {
                source_kind_id: source_kind_id.to_owned(),
                display_name_key: self.item_display_name_key(source_kind_id),
                attribute,
                change: ItemAttributeChange::Sustained,
                before: value,
                after: value,
                maximum: self.progress.maximum_attributes.value(attribute),
                noticed: true,
            });
            return true;
        }
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let before = self.progress.attributes.value(attribute);
        let noticed = self.progress.drain_attribute(attribute, &mut self.rng);
        if noticed {
            self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemAttributeChanged {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            attribute,
            change: ItemAttributeChange::Drained,
            before,
            after: self.progress.attributes.value(attribute),
            maximum: self.progress.maximum_attributes.value(attribute),
            noticed,
        });
        noticed
    }

    fn resolve_item_restore_attribute(
        &mut self,
        source_kind_id: &str,
        attribute: AttributeKind,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let before = self.progress.attributes.value(attribute);
        let noticed = self.progress.restore_attribute(attribute);
        if noticed {
            self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemAttributeChanged {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            attribute,
            change: ItemAttributeChange::Restored,
            before,
            after: self.progress.attributes.value(attribute),
            maximum: self.progress.maximum_attributes.value(attribute),
            noticed,
        });
        noticed
    }

    fn resolve_item_increase_attributes(
        &mut self,
        source_kind_id: &str,
        attributes: &[AttributeKind],
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let victorious = self.victory_level_cap_unlocked();
        let mut noticed = false;
        let mut resolutions = Vec::with_capacity(attributes.len());

        for &attribute in attributes {
            let before = self.progress.attributes.value(attribute);
            let maximum_before = self.progress.maximum_attributes.value(attribute);
            let changed =
                self.progress
                    .increase_attribute_permanently(attribute, victorious, &mut self.rng);
            let after = self.progress.attributes.value(attribute);
            let maximum = self.progress.maximum_attributes.value(attribute);
            let change = if maximum > maximum_before {
                ItemAttributeChange::Increased
            } else if after > before {
                ItemAttributeChange::Restored
            } else {
                ItemAttributeChange::Increased
            };
            resolutions.push((attribute, change, before, after, maximum, changed));
            noticed = changed || noticed;
        }

        if noticed {
            self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
            self.mark_item_aware(source_kind_id);
        }
        let display_name_key = self.item_display_name_key(source_kind_id);
        for (attribute, change, before, after, maximum, changed) in resolutions {
            events.push(DomainEvent::ItemAttributeChanged {
                source_kind_id: source_kind_id.to_owned(),
                display_name_key: display_name_key.clone(),
                attribute,
                change,
                before,
                after,
                maximum,
                noticed: changed,
            });
        }
        noticed
    }

    fn resolve_item_thermal_resistance(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let duration_sides =
            u16::try_from(duration_sides).expect("validated thermal die sides must fit u16");
        let duration = u32::try_from(self.roll_damage(duration_dice, duration_sides))
            .expect("validated thermal duration must fit u32")
            .saturating_add(duration_bonus);
        let change = apply_status(
            &mut self.player.statuses,
            StatusApplication {
                status: StatusInstance {
                    kind_id: STATUS_THERMAL_RESISTANCE.to_owned(),
                    intensity: 1,
                    remaining_ticks: duration,
                    source_id: Some(source_kind_id.to_owned()),
                    granted_resistances: BTreeMap::from([
                        (DamageType::Fire, ResistanceLevel::Resistant),
                        (DamageType::Cold, ResistanceLevel::Resistant),
                    ]),
                    granted_brands: BTreeSet::new(),
                    granted_modifiers: StatModifiersDto::default(),
                    granted_equipment_bonuses: EquipmentBonusesDto::default(),
                    granted_status_immunities: BTreeSet::new(),
                    granted_race_id: None,
                    grants_wall_passage: false,
                    incoming_damage_percent: 100,
                },
                stacking: StatusStacking::Extend,
            },
        );
        let noticed = matches!(change, StatusChange::Added);
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemThermalResistanceResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            noticed,
        });
        noticed
    }

    fn resolve_item_basic_resistance(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) {
        let duration_sides =
            u16::try_from(duration_sides).expect("validated resistance die sides must fit u16");
        let duration = u32::try_from(self.roll_damage(duration_dice, duration_sides))
            .expect("validated resistance duration must fit u32")
            .saturating_add(duration_bonus);
        apply_status(
            &mut self.player.statuses,
            StatusApplication {
                status: StatusInstance {
                    kind_id: STATUS_BASIC_RESISTANCE.to_owned(),
                    intensity: 1,
                    remaining_ticks: duration,
                    source_id: Some(source_kind_id.to_owned()),
                    granted_resistances: BTreeMap::from([
                        (DamageType::Acid, ResistanceLevel::Resistant),
                        (DamageType::Electricity, ResistanceLevel::Resistant),
                        (DamageType::Fire, ResistanceLevel::Resistant),
                        (DamageType::Cold, ResistanceLevel::Resistant),
                        (DamageType::Poison, ResistanceLevel::Resistant),
                    ]),
                    granted_brands: BTreeSet::new(),
                    granted_modifiers: StatModifiersDto::default(),
                    granted_equipment_bonuses: EquipmentBonusesDto::default(),
                    granted_status_immunities: BTreeSet::new(),
                    granted_race_id: None,
                    grants_wall_passage: false,
                    incoming_damage_percent: 100,
                },
                stacking: StatusStacking::KeepStrongest,
            },
        );
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemBasicResistanceApplied {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
        });
    }

    fn resolve_item_poison(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let resistance = self
            .effective_player_resistances()
            .level(DamageType::Poison);
        let resistance_threshold = u64::try_from(resistance.reduction_percent().max(0))
            .expect("threshold is non-negative");
        let resisted = self.rng.bounded(55) < resistance_threshold;
        let duration = if resisted {
            None
        } else {
            let duration_sides =
                u16::try_from(duration_sides).expect("validated poison die sides must fit u16");
            let duration = u32::try_from(self.roll_damage(duration_dice, duration_sides))
                .expect("validated poison duration must fit u32")
                .saturating_add(duration_bonus);
            apply_status(
                &mut self.player.statuses,
                StatusApplication {
                    status: StatusInstance {
                        kind_id: STATUS_POISON.to_owned(),
                        intensity: 1,
                        remaining_ticks: duration,
                        source_id: Some(source_kind_id.to_owned()),
                        granted_resistances: BTreeMap::new(),
                        granted_brands: BTreeSet::new(),
                        granted_modifiers: StatModifiersDto::default(),
                        granted_equipment_bonuses: EquipmentBonusesDto::default(),
                        granted_status_immunities: BTreeSet::new(),
                        granted_race_id: None,
                        grants_wall_passage: false,
                        incoming_damage_percent: 100,
                    },
                    stacking: StatusStacking::Extend,
                },
            );
            self.mark_item_aware(source_kind_id);
            Some(duration)
        };
        events.push(DomainEvent::ItemPoisonResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
        });
        !resisted
    }

    fn resolve_item_blindness(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let resistance_threshold = if self.player_status_immunities().contains(STATUS_BLINDNESS) {
            55
        } else {
            0
        };
        let resisted = self.rng.bounded(55) < resistance_threshold;
        let (duration, noticed) = if resisted {
            (None, false)
        } else {
            let duration_sides =
                u16::try_from(duration_sides).expect("validated blindness die sides must fit u16");
            let duration = u32::try_from(self.roll_damage(duration_dice, duration_sides))
                .expect("validated blindness duration must fit u32")
                .saturating_add(duration_bonus);
            let change = apply_status(
                &mut self.player.statuses,
                StatusApplication {
                    status: StatusInstance {
                        kind_id: STATUS_BLINDNESS.to_owned(),
                        intensity: 1,
                        remaining_ticks: duration,
                        source_id: Some(source_kind_id.to_owned()),
                        granted_resistances: BTreeMap::new(),
                        granted_brands: BTreeSet::new(),
                        granted_modifiers: StatModifiersDto::default(),
                        granted_equipment_bonuses: EquipmentBonusesDto::default(),
                        granted_status_immunities: BTreeSet::new(),
                        granted_race_id: None,
                        grants_wall_passage: false,
                        incoming_damage_percent: 100,
                    },
                    stacking: StatusStacking::Extend,
                },
            );
            let noticed = matches!(change, StatusChange::Added);
            if noticed {
                self.mark_item_aware(source_kind_id);
            }
            (Some(duration), noticed)
        };
        events.push(DomainEvent::ItemBlindnessResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            noticed,
        });
        noticed
    }

    fn resolve_item_life_loss(
        &mut self,
        source_kind_id: &str,
        amount: u32,
        events: &mut Vec<DomainEvent>,
    ) {
        let amount = i32::try_from(amount).expect("validated life loss must fit i32");
        self.player.hp = self.player.hp.saturating_sub(amount);
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemLifeLost {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            amount,
            fatal: self.player_is_dead(),
        });
    }

    fn resolve_item_vengeance(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) {
        let resolution = apply_ability_status_effect(
            &mut self.player,
            source_kind_id,
            0,
            STATUS_VENGEANCE,
            1,
            duration_bonus,
            duration_dice,
            duration_sides,
            AbilityStatusStackingDefinition::KeepStrongest,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers::default(),
            &EquipmentBonuses::default(),
            &BTreeSet::new(),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        let duration = match &resolution {
            AbilityEffectResolutionDto::ApplyStatus {
                applied_duration_ticks,
                ..
            } => *applied_duration_ticks,
            _ => unreachable!("vengeance must produce a status application resolution"),
        };
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemVengeanceActivated {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![resolution],
            },
        });
    }

    fn resolve_item_healing(
        &mut self,
        source_kind_id: &str,
        amount: i32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let max_hp = self.effective_player_max_hp();
        let player = &mut self.player;
        let outcome = apply_effect(
            &mut EffectTarget {
                hp: &mut player.hp,
                max_hp,
                resistances: &player.resistances,
                statuses: &mut player.statuses,
            },
            EffectSpec::Heal { amount },
        );
        let EffectOutcome::Healed { requested, applied } = outcome else {
            unreachable!("healing effects must produce healing outcomes");
        };
        if applied > 0 {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemUsed {
            display_name_key: self.item_display_name_key(source_kind_id),
            source_kind_id: source_kind_id.to_owned(),
            requested,
            applied,
        });
        applied > 0
    }

    fn resolve_item_resource_restoration(
        &mut self,
        source_kind_id: &str,
        resource_id: &str,
        requested: u32,
        full: bool,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let (before, after) = if let Some(pool) = self.resources.get_mut(resource_id) {
            let before = pool.current;
            pool.current = if full {
                pool.maximum
            } else {
                pool.current.saturating_add(requested).min(pool.maximum)
            };
            (before, pool.current)
        } else {
            (0, 0)
        };
        let recovered = after.saturating_sub(before);
        if recovered > 0 {
            self.resources_touched.insert(resource_id.to_owned());
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemResourceRestored {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            resolution: ResourceRecoveryResolutionDto {
                resource_id: resource_id.to_owned(),
                before,
                after,
                recovered,
            },
        });
        recovered > 0
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

    fn item_can_receive_recharge(&self, item: &ItemInstance) -> bool {
        item.location == ItemLocation::Inventory
            && item.activation.is_some()
            && self
                .content
                .item(&item.kind_id)
                .is_some_and(|definition| definition.device_generation.is_some())
            && item
                .charges
                .is_some_and(|charges| charges.current < charges.maximum)
    }

    fn item_can_supply_recharge(&self, item: &ItemInstance) -> bool {
        item.location == ItemLocation::Inventory
            && self
                .content
                .item(&item.kind_id)
                .is_some_and(|definition| definition.tags.iter().any(|tag| tag == "device"))
            && item.charges.is_some_and(|charges| charges.current > 0)
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

    #[cfg(test)]
    fn resolve_monster_ability(&mut self, index: usize, events: &mut Vec<DomainEvent>) -> bool {
        self.resolve_monster_ability_with_changes(
            index,
            events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("monster ability test resolution should preserve invariants")
    }

    fn resolve_monster_ability_with_changes(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<bool, CoreError> {
        let source_entity_id = self.entities[index].id.clone();
        let source_kind_id = self.entities[index].kind_id.clone();
        let Some(casting) = self
            .content
            .actor(&source_kind_id)
            .and_then(|definition| definition.monster_casting.clone())
        else {
            return Ok(false);
        };
        if self.entities[index].casting_cooldown_remaining > 0 {
            self.entities[index].casting_cooldown_remaining -= 1;
            return Ok(false);
        }

        // FrogComposband checks the monster's spell frequency before asking
        // monspell.c to filter and choose a currently viable weighted spell.
        // Keep that RNG boundary explicit: every alerted caster action draws
        // exactly one frequency percentile, even if walls or allies later
        // leave no legal spell.
        let frequency_roll = u8::try_from(self.rng.bounded(100) + 1)
            .expect("monster ability percentile must fit u8");
        let mut candidates = Vec::with_capacity(casting.abilities.len());
        let mut viable = Vec::new();
        for candidate in &casting.abilities {
            let ability = self
                .content
                .ability(&candidate.ability_id)
                .expect("validated monster ability must remain available")
                .clone();
            match self.monster_ability_plan(index, ability, candidate.weight) {
                Ok(plan) => {
                    candidates.push(self.monster_ability_candidate_dto(index, &plan, None));
                    viable.push(plan);
                }
                Err(rejection) => {
                    candidates.push(MonsterAbilityCandidateResolutionDto {
                        ability_id: candidate.ability_id.clone(),
                        base_weight: candidate.weight,
                        effective_weight: 0,
                        target_entity_id: None,
                        target_kind_id: None,
                        target_position: None,
                        affected_positions: Vec::new(),
                        enemy_target_count: rejection.enemy_target_count,
                        friendly_risk_count: rejection.friendly_risk_count,
                        rejection_reason: Some(rejection.reason),
                    });
                }
            }
        }
        let viable_ability_ids = viable
            .iter()
            .map(|candidate| candidate.ability.id.clone())
            .collect::<Vec<_>>();
        let total_weight = viable.iter().fold(0_u32, |total, candidate| {
            total.saturating_add(candidate.effective_weight)
        });
        let mut selection_roll = None;
        let mut selected_index = None;
        if frequency_roll <= casting.frequency_percent && total_weight > 0 {
            let roll = u32::try_from(self.rng.bounded(u64::from(total_weight)) + 1)
                .expect("validated monster ability weight roll must fit u32");
            selection_roll = Some(roll);
            let mut remaining = roll;
            for (candidate_index, candidate) in viable.iter().enumerate() {
                if remaining <= candidate.effective_weight {
                    selected_index = Some(candidate_index);
                    break;
                }
                remaining -= candidate.effective_weight;
            }
        }
        let selected_ability_id =
            selected_index.map(|candidate_index| viable[candidate_index].ability.id.clone());
        events.push(DomainEvent::MonsterAbilityDecision {
            resolution: MonsterAbilityDecisionResolutionDto {
                source_entity_id: source_entity_id.clone(),
                source_kind_id: source_kind_id.clone(),
                frequency_percent: casting.frequency_percent,
                frequency_roll,
                candidates,
                viable_ability_ids,
                total_weight,
                selection_roll,
                selected_ability_id: selected_ability_id.clone(),
            },
        });

        let Some(selected_index) = selected_index else {
            return Ok(false);
        };
        let plan = viable[selected_index].clone();
        self.entities[index].casting_cooldown_remaining =
            monster_casting_cooldown(casting.frequency_percent);
        let player_hp_before = self.player.hp;
        let MonsterAbilityPlanResolution {
            target_entity_id,
            target_kind_id,
            affected_positions,
            summon,
            effects,
            targets,
            trace,
        } = self.resolve_monster_ability_plan(
            index,
            &source_kind_id,
            &plan,
            events,
            changed,
            removed_entities,
        );
        events.push(DomainEvent::MonsterAbilityCast {
            resolution: Box::new(MonsterAbilityCastResolutionDto {
                source_entity_id: source_entity_id.clone(),
                source_kind_id,
                ability_id: plan.ability.id,
                target_entity_id,
                target_kind_id,
                affected_positions,
                summon,
                effects,
                targets,
            }),
            trace,
        });
        self.resolve_vengeance_retaliation(
            &source_entity_id,
            player_hp_before.saturating_sub(self.player.hp),
            events,
            changed,
            removed_entities,
        )?;
        Ok(true)
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

    fn monster_ability_plan(
        &self,
        index: usize,
        ability: AbilityDefinition,
        base_weight: u32,
    ) -> Result<MonsterAbilityPlan, MonsterAbilityPlanRejection> {
        let origin = self.entities[index].position;
        let (target, enemy_target_count, friendly_risk_count) = match &ability.effect {
            AbilityEffectDefinition::Heal { .. } => (MonsterAbilityTargetPlan::SelfTarget, 0, 0),
            AbilityEffectDefinition::Summon { count, radius, .. } => {
                let positions = self
                    .summon_positions_around(origin, *count, *radius)
                    .ok_or(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoSpace,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    })?;
                (MonsterAbilityTargetPlan::Summon { positions }, 0, 0)
            }
            AbilityEffectDefinition::SummonCategory {
                category,
                maximum_level,
                count_dice,
                count_sides,
                count_bonus,
                radius,
                ..
            } => {
                // Candidate kinds enumerate in stable id order and are
                // filtered without RNG; the per-summon kind draws happen at
                // execution time.
                let candidate_kind_ids = self
                    .content
                    .actor_definitions()
                    .filter(|definition| {
                        definition.role == ActorRole::Monster
                            && definition.level <= u32::from(*maximum_level)
                            && definition.tags.iter().any(|tag| tag == category)
                    })
                    .map(|definition| definition.id.clone())
                    .collect::<Vec<_>>();
                if candidate_kind_ids.is_empty() {
                    return Err(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoCandidates,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    });
                }
                let maximum_count = usize::from(*count_dice) * usize::from(*count_sides)
                    + usize::from(*count_bonus);
                let positions = self
                    .open_positions_around(origin, *radius)
                    .into_iter()
                    .take(maximum_count)
                    .collect::<Vec<_>>();
                if positions.is_empty() {
                    return Err(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoSpace,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    });
                }
                (
                    MonsterAbilityTargetPlan::SummonCategory {
                        candidate_kind_ids,
                        positions,
                    },
                    0,
                    0,
                )
            }
            AbilityEffectDefinition::ApplyStatus { .. }
            | AbilityEffectDefinition::RemoveStatus { .. }
            | AbilityEffectDefinition::Sequence { .. }
                if ability
                    .target
                    .modes
                    .contains(&AbilityTargetModeDefinition::SelfTarget) =>
            {
                (MonsterAbilityTargetPlan::SelfTarget, 0, 0)
            }
            AbilityEffectDefinition::BlinkSelf { radius } => {
                let radius = u32::from(*radius);
                let destinations = self.displacement_destinations(index, |position| {
                    origin
                        .x
                        .abs_diff(position.x)
                        .max(origin.y.abs_diff(position.y))
                        <= radius
                });
                if destinations.is_empty() {
                    return Err(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoSpace,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    });
                }
                (MonsterAbilityTargetPlan::BlinkSelf { destinations }, 0, 0)
            }
            AbilityEffectDefinition::TeleportSelf { minimum_distance } => {
                let player = self.player.position;
                let escape_candidates = |minimum: u32| {
                    self.displacement_destinations(index, |position| {
                        player
                            .x
                            .abs_diff(position.x)
                            .max(player.y.abs_diff(position.y))
                            >= minimum
                    })
                };
                let minimum = u32::from(*minimum_distance);
                let mut destinations = escape_candidates(minimum);
                if destinations.is_empty() {
                    // The half-distance fallback keeps cramped floors escapable.
                    destinations = escape_candidates(minimum.div_ceil(2));
                }
                if destinations.is_empty() {
                    return Err(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoSpace,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    });
                }
                (MonsterAbilityTargetPlan::EscapeSelf { destinations }, 0, 0)
            }
            AbilityEffectDefinition::Damage { .. }
            | AbilityEffectDefinition::AreaDamage { .. }
            | AbilityEffectDefinition::BeamDamage { .. }
            | AbilityEffectDefinition::ConeDamage { .. }
            | AbilityEffectDefinition::BreathDamage { .. }
            | AbilityEffectDefinition::CurseDamage { .. }
            | AbilityEffectDefinition::TeleportAway { .. }
            | AbilityEffectDefinition::DrainResource { .. }
            | AbilityEffectDefinition::Amnesia
            | AbilityEffectDefinition::ApplyStatus { .. }
            | AbilityEffectDefinition::RemoveStatus { .. }
            | AbilityEffectDefinition::Sequence { .. }
            | AbilityEffectDefinition::TeleportTarget => {
                let mut first_rejection = None;
                let mut selected = None;
                for hostile_target in self.monster_hostile_targets(index) {
                    match self.monster_targeted_ability_plan(index, &ability, hostile_target) {
                        Ok(plan) => {
                            selected = Some(plan);
                            break;
                        }
                        Err(rejection) => {
                            first_rejection.get_or_insert(rejection);
                        }
                    }
                }
                selected.ok_or_else(|| {
                    first_rejection.unwrap_or(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::InvalidTarget,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    })
                })?
            }
            _ => {
                return Err(MonsterAbilityPlanRejection {
                    reason: MonsterAbilityRejectionReasonDto::InvalidTarget,
                    enemy_target_count: 0,
                    friendly_risk_count: 0,
                });
            }
        };
        let utility_multiplier = self
            .monster_ability_utility_multiplier(index, &ability, &target)
            .ok_or(MonsterAbilityPlanRejection {
                reason: MonsterAbilityRejectionReasonDto::NoUtility,
                enemy_target_count,
                friendly_risk_count,
            })?;
        let target_position = monster_plan_target(&target).map(MonsterHostileTarget::position);
        let distance_multiplier = if !matches!(
            target,
            MonsterAbilityTargetPlan::SelfTarget | MonsterAbilityTargetPlan::Summon { .. }
        ) && target_position.is_some_and(|position| {
            origin
                .x
                .abs_diff(position.x)
                .max(origin.y.abs_diff(position.y))
                >= 3
        }) {
            2
        } else {
            1
        };
        let target_multiplier = u32::from(enemy_target_count.max(1));
        let resistance_percent = self.monster_ability_resistance_percent(index, &ability, &target);
        if resistance_percent == 0 {
            return Err(MonsterAbilityPlanRejection {
                reason: MonsterAbilityRejectionReasonDto::NoUtility,
                enemy_target_count,
                friendly_risk_count,
            });
        }
        let weighted = base_weight
            .saturating_mul(utility_multiplier)
            .saturating_mul(distance_multiplier)
            .saturating_mul(target_multiplier)
            .saturating_mul(resistance_percent)
            / 100;
        Ok(MonsterAbilityPlan {
            ability,
            base_weight,
            effective_weight: weighted.max(1),
            enemy_target_count,
            friendly_risk_count,
            target,
        })
    }

    fn monster_targeted_ability_plan(
        &self,
        source_index: usize,
        ability: &AbilityDefinition,
        target: MonsterHostileTarget,
    ) -> Result<(MonsterAbilityTargetPlan, u16, u16), MonsterAbilityPlanRejection> {
        let origin = self.entities[source_index].position;
        let target_position = target.position();
        let (plan, affected_positions) = match &ability.effect {
            AbilityEffectDefinition::AreaDamage { radius, .. } => {
                let trace =
                    self.monster_projectile_trace(source_index, ability, &target, false, false)?;
                let affected_positions = self
                    .area_damage_cells(target_position, *radius)
                    .into_iter()
                    .map(|(_, position)| position)
                    .collect::<Vec<_>>();
                (
                    MonsterAbilityTargetPlan::Area {
                        target,
                        trace,
                        affected_positions: affected_positions.clone(),
                    },
                    affected_positions,
                )
            }
            AbilityEffectDefinition::BeamDamage { .. } => {
                let trace =
                    self.monster_projectile_trace(source_index, ability, &target, false, true)?;
                let affected_positions = trace.traversed.clone();
                (
                    MonsterAbilityTargetPlan::Beam {
                        target,
                        trace,
                        affected_positions: affected_positions.clone(),
                    },
                    affected_positions,
                )
            }
            AbilityEffectDefinition::ConeDamage { radius, .. }
            | AbilityEffectDefinition::BreathDamage { radius, .. } => {
                let direction = direction_toward(origin, target_position).ok_or(
                    MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::InvalidTarget,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    },
                )?;
                let (dx, dy) = direction.delta();
                let path = (1..=ability.target.range)
                    .map(|step| Position {
                        x: origin.x + dx * i32::from(step),
                        y: origin.y + dy * i32::from(step),
                    })
                    .collect::<Vec<_>>();
                let trace = self.trace_monster_path(origin, path);
                let cells = self.cone_damage_cells(origin, &trace.traversed, direction, *radius);
                if !cells
                    .iter()
                    .any(|(_, _, position)| *position == target_position)
                {
                    return Err(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::OutOfRange,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    });
                }
                let affected_positions = cells
                    .into_iter()
                    .map(|(_, _, position)| position)
                    .collect::<Vec<_>>();
                (
                    MonsterAbilityTargetPlan::Cone {
                        target,
                        trace,
                        affected_positions: affected_positions.clone(),
                    },
                    affected_positions,
                )
            }
            AbilityEffectDefinition::Damage { .. }
            | AbilityEffectDefinition::CurseDamage { .. }
            | AbilityEffectDefinition::DrainResource { .. }
            | AbilityEffectDefinition::Amnesia
            | AbilityEffectDefinition::ApplyStatus { .. }
            | AbilityEffectDefinition::RemoveStatus { .. }
            | AbilityEffectDefinition::Sequence { .. } => {
                let trace =
                    self.monster_projectile_trace(source_index, ability, &target, true, false)?;
                (
                    MonsterAbilityTargetPlan::Projectile { target, trace },
                    vec![target_position],
                )
            }
            AbilityEffectDefinition::TeleportAway { minimum_distance } => {
                let trace =
                    self.monster_projectile_trace(source_index, ability, &target, true, false)?;
                // The banished target lands away from the caster; candidates
                // collect without RNG and the halved fallback mirrors
                // teleport-self on cramped floors.
                let banish_candidates = |minimum: u32| {
                    let mut candidates = Vec::new();
                    for y in 0..self.height {
                        for x in 0..self.width {
                            let position = Position {
                                x: i32::from(x),
                                y: i32::from(y),
                            };
                            if position == self.player.position
                                || !self.is_walkable(position)
                                || origin
                                    .x
                                    .abs_diff(position.x)
                                    .max(origin.y.abs_diff(position.y))
                                    < minimum
                                || self
                                    .entities
                                    .iter()
                                    .any(|entity| entity.hp > 0 && entity.position == position)
                            {
                                continue;
                            }
                            candidates.push(position);
                        }
                    }
                    candidates
                };
                let minimum = u32::from(*minimum_distance);
                let mut destinations = banish_candidates(minimum);
                if destinations.is_empty() {
                    destinations = banish_candidates(minimum.div_ceil(2));
                }
                if destinations.is_empty() {
                    return Err(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoSpace,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    });
                }
                (
                    MonsterAbilityTargetPlan::BanishTarget {
                        target,
                        trace,
                        destinations,
                    },
                    vec![target_position],
                )
            }
            AbilityEffectDefinition::TeleportTarget => {
                let trace =
                    self.monster_projectile_trace(source_index, ability, &target, true, false)?;
                // The dragged target lands on the first open cell adjacent to
                // the caster, in the canonical eight-direction order.
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
                let destination = DELTAS
                    .iter()
                    .map(|(dx, dy)| Position {
                        x: origin.x + dx,
                        y: origin.y + dy,
                    })
                    .find(|position| {
                        self.index(*position).is_some()
                            && self.is_walkable(*position)
                            && *position != self.player.position
                            && !self
                                .entities
                                .iter()
                                .any(|entity| entity.hp > 0 && entity.position == *position)
                    })
                    .ok_or(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoSpace,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    })?;
                (
                    MonsterAbilityTargetPlan::DragTarget {
                        target,
                        trace,
                        destination,
                    },
                    vec![target_position],
                )
            }
            _ => {
                return Err(MonsterAbilityPlanRejection {
                    reason: MonsterAbilityRejectionReasonDto::InvalidTarget,
                    enemy_target_count: 0,
                    friendly_risk_count: 0,
                });
            }
        };
        let (enemy_target_count, friendly_risk_count) =
            self.monster_footprint_faction_counts(source_index, &affected_positions);
        if friendly_risk_count > 0 {
            return Err(MonsterAbilityPlanRejection {
                reason: MonsterAbilityRejectionReasonDto::FriendlyRisk,
                enemy_target_count,
                friendly_risk_count,
            });
        }
        Ok((plan, enemy_target_count, friendly_risk_count))
    }

    fn monster_projectile_trace(
        &self,
        index: usize,
        ability: &AbilityDefinition,
        hostile_target: &MonsterHostileTarget,
        clean_shot: bool,
        continue_through_target: bool,
    ) -> Result<ProjectileTrace, MonsterAbilityPlanRejection> {
        let origin = self.entities[index].position;
        let target = hostile_target.position();
        if target == origin
            || self.index(target).is_none()
            || origin.x.abs_diff(target.x).max(origin.y.abs_diff(target.y))
                > u32::from(ability.target.range)
        {
            return Err(MonsterAbilityPlanRejection {
                reason: MonsterAbilityRejectionReasonDto::OutOfRange,
                enemy_target_count: 0,
                friendly_risk_count: 0,
            });
        }
        let path = if continue_through_target {
            projectile_path_through_target(origin, target, ability.target.range)
        } else {
            projectile_path_between(origin, target, ability.target.range)
        }
        .ok_or(MonsterAbilityPlanRejection {
            reason: MonsterAbilityRejectionReasonDto::InvalidTarget,
            enemy_target_count: 0,
            friendly_risk_count: 0,
        })?;
        let trace = self.trace_monster_path(origin, path);
        if !trace.traversed.contains(&target) {
            return Err(MonsterAbilityPlanRejection {
                reason: MonsterAbilityRejectionReasonDto::Blocked,
                enemy_target_count: 0,
                friendly_risk_count: 0,
            });
        }
        if clean_shot {
            for position in trace
                .traversed
                .iter()
                .filter(|position| **position != target)
            {
                if let Some((candidate_index, _)) =
                    self.entities
                        .iter()
                        .enumerate()
                        .find(|(candidate_index, entity)| {
                            *candidate_index != index
                                && entity.hp > 0
                                && entity.position == *position
                        })
                {
                    let enemy = self.entity_is_player_aligned(candidate_index);
                    return Err(MonsterAbilityPlanRejection {
                        reason: if enemy {
                            MonsterAbilityRejectionReasonDto::Blocked
                        } else {
                            MonsterAbilityRejectionReasonDto::FriendlyRisk
                        },
                        enemy_target_count: u16::from(enemy),
                        friendly_risk_count: u16::from(!enemy),
                    });
                }
            }
        }
        Ok(trace)
    }

    fn trace_monster_path(&self, origin: Position, path: Vec<Position>) -> ProjectileTrace {
        let mut impact = origin;
        let mut landing = origin;
        let mut traversed = Vec::new();
        for position in path {
            if self.index(position).is_none() || !self.is_walkable(position) {
                impact = position;
                break;
            }
            impact = position;
            landing = position;
            traversed.push(position);
        }
        ProjectileTrace {
            origin,
            impact,
            landing,
            traversed,
        }
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

    fn monster_ability_resistance_percent(
        &self,
        source_index: usize,
        ability: &AbilityDefinition,
        target: &MonsterAbilityTargetPlan,
    ) -> u32 {
        let Some(hostile_target) = monster_plan_target(target) else {
            return 100;
        };
        if !hostile_target.is_player()
            || !self
                .content
                .actor(&self.entities[source_index].kind_id)
                .and_then(|actor| actor.monster_casting.as_ref())
                .is_some_and(|casting| casting.smart)
        {
            return 100;
        }
        ability
            .effect
            .ordered_effects()
            .iter()
            .filter_map(|effect| match effect {
                AbilityEffectDefinition::Damage { damage_type, .. }
                | AbilityEffectDefinition::AreaDamage { damage_type, .. }
                | AbilityEffectDefinition::BeamDamage { damage_type, .. }
                | AbilityEffectDefinition::ConeDamage { damage_type, .. }
                | AbilityEffectDefinition::BreathDamage { damage_type, .. } => {
                    Some(DamageType::from(*damage_type))
                }
                AbilityEffectDefinition::ApplyStatus {
                    resistance_type, ..
                } => resistance_type.map(DamageType::from),
                _ => None,
            })
            .filter_map(|damage_type| {
                self.entities[source_index]
                    .observed_player_resistances
                    .get(&damage_type)
                    .copied()
            })
            .map(|level| match level {
                ResistanceLevel::Vulnerable => 150,
                ResistanceLevel::Normal => 100,
                ResistanceLevel::Resistant => 50,
                ResistanceLevel::Strong => 35,
                ResistanceLevel::Immune => 0,
            })
            .min()
            .unwrap_or(100)
    }

    fn monster_footprint_faction_counts(
        &self,
        source_index: usize,
        affected_positions: &[Position],
    ) -> (u16, u16) {
        let mut enemies =
            u16::from(affected_positions.contains(&self.player.position) && !self.player_is_dead());
        let mut friendlies = 0_u16;
        for (index, entity) in self.entities.iter().enumerate() {
            if index == source_index
                || entity.hp <= 0
                || !affected_positions.contains(&entity.position)
            {
                continue;
            }
            if self.entity_is_player_aligned(index) {
                enemies = enemies.saturating_add(1);
            } else {
                friendlies = friendlies.saturating_add(1);
            }
        }
        (enemies, friendlies)
    }

    fn monster_ability_utility_multiplier(
        &self,
        source_index: usize,
        ability: &AbilityDefinition,
        target: &MonsterAbilityTargetPlan,
    ) -> Option<u32> {
        if matches!(
            target,
            MonsterAbilityTargetPlan::Area { .. }
                | MonsterAbilityTargetPlan::Beam { .. }
                | MonsterAbilityTargetPlan::Cone { .. }
                | MonsterAbilityTargetPlan::Summon { .. }
                | MonsterAbilityTargetPlan::SummonCategory { .. }
        ) {
            return Some(1);
        }
        let hostile_target = monster_plan_target(target);
        let target_actor = if matches!(target, MonsterAbilityTargetPlan::SelfTarget) {
            Some(&self.entities[source_index])
        } else {
            hostile_target.and_then(|target| match target {
                MonsterHostileTarget::Player { .. } => None,
                MonsterHostileTarget::Summon { entity_id, .. } => {
                    self.entities.iter().find(|entity| entity.id == *entity_id)
                }
            })
        };
        let player_target = hostile_target.is_some_and(MonsterHostileTarget::is_player);
        let effects = ability.effect.ordered_effects();
        let mut useful = false;
        let mut multiplier = 1_u32;
        for effect in effects {
            match effect {
                AbilityEffectDefinition::Damage { .. } if hostile_target.is_some() => useful = true,
                AbilityEffectDefinition::CurseDamage { .. } if hostile_target.is_some() => {
                    useful = true;
                }
                AbilityEffectDefinition::TeleportAway { .. }
                | AbilityEffectDefinition::DrainResource { .. }
                | AbilityEffectDefinition::Amnesia
                    if hostile_target.is_some() =>
                {
                    useful = true;
                }
                AbilityEffectDefinition::BlinkSelf { .. }
                | AbilityEffectDefinition::TeleportSelf { .. } => useful = true,
                AbilityEffectDefinition::TeleportTarget if hostile_target.is_some() => {
                    useful = true;
                }
                AbilityEffectDefinition::Heal { .. } => {
                    let actor = target_actor?;
                    let missing = actor.max_hp.saturating_sub(actor.hp).max(0);
                    let missing_percent = u32::try_from(
                        i64::from(missing)
                            .saturating_mul(100)
                            .saturating_div(i64::from(actor.max_hp.max(1))),
                    )
                    .unwrap_or(100);
                    // Match the original wounded filter: healing is ignored at
                    // 20% wounds or less, then gains weight as wounds deepen.
                    if missing_percent > 20 {
                        useful = true;
                        multiplier = multiplier.max(missing_percent.div_ceil(25).clamp(1, 4));
                    }
                }
                AbilityEffectDefinition::ApplyStatus {
                    status_kind_id,
                    intensity,
                    resistance_type,
                    ..
                } => {
                    let statuses = if player_target {
                        &self.player.statuses
                    } else {
                        &target_actor?.statuses
                    };
                    let immune = resistance_type.is_some_and(|damage_type| {
                        let damage_type = DamageType::from(damage_type);
                        if player_target {
                            self.entities[source_index]
                                .observed_player_resistances
                                .get(&damage_type)
                                .is_some_and(|level| *level == ResistanceLevel::Immune)
                        } else {
                            target_actor.is_some_and(|actor| {
                                actor.resistances.level(damage_type) == ResistanceLevel::Immune
                            })
                        }
                    });
                    if !immune
                        && statuses
                            .iter()
                            .find(|status| status.kind_id == *status_kind_id)
                            .is_none_or(|status| status.intensity < *intensity)
                    {
                        useful = true;
                    }
                }
                AbilityEffectDefinition::RemoveStatus { status_kind_id } => {
                    let statuses = if player_target {
                        &self.player.statuses
                    } else {
                        &target_actor?.statuses
                    };
                    if statuses
                        .iter()
                        .any(|status| status.kind_id == *status_kind_id)
                    {
                        useful = true;
                    }
                }
                _ => {}
            }
        }
        useful.then_some(multiplier)
    }

    fn monster_ability_candidate_dto(
        &self,
        source_index: usize,
        plan: &MonsterAbilityPlan,
        rejection_reason: Option<MonsterAbilityRejectionReasonDto>,
    ) -> MonsterAbilityCandidateResolutionDto {
        let source = &self.entities[source_index];
        let (target_entity_id, target_kind_id, target_position, affected_positions) =
            match &plan.target {
                MonsterAbilityTargetPlan::SelfTarget => (
                    Some(source.id.clone()),
                    Some(source.kind_id.clone()),
                    Some(source.position),
                    Vec::new(),
                ),
                MonsterAbilityTargetPlan::Summon { positions }
                | MonsterAbilityTargetPlan::SummonCategory { positions, .. } => (
                    Some(source.id.clone()),
                    Some(source.kind_id.clone()),
                    Some(source.position),
                    positions.clone(),
                ),
                MonsterAbilityTargetPlan::Projectile { target, .. } => (
                    Some(target.entity_id().to_owned()),
                    Some(target.kind_id().to_owned()),
                    Some(target.position()),
                    vec![target.position()],
                ),
                MonsterAbilityTargetPlan::Area {
                    target,
                    affected_positions,
                    ..
                }
                | MonsterAbilityTargetPlan::Beam {
                    target,
                    affected_positions,
                    ..
                }
                | MonsterAbilityTargetPlan::Cone {
                    target,
                    affected_positions,
                    ..
                } => (
                    Some(target.entity_id().to_owned()),
                    Some(target.kind_id().to_owned()),
                    Some(target.position()),
                    affected_positions.clone(),
                ),
                MonsterAbilityTargetPlan::BlinkSelf { .. }
                | MonsterAbilityTargetPlan::EscapeSelf { .. } => (
                    Some(source.id.clone()),
                    Some(source.kind_id.clone()),
                    Some(source.position),
                    Vec::new(),
                ),
                MonsterAbilityTargetPlan::DragTarget {
                    target,
                    destination,
                    ..
                } => (
                    Some(target.entity_id().to_owned()),
                    Some(target.kind_id().to_owned()),
                    Some(target.position()),
                    vec![*destination],
                ),
                MonsterAbilityTargetPlan::BanishTarget { target, .. } => (
                    Some(target.entity_id().to_owned()),
                    Some(target.kind_id().to_owned()),
                    Some(target.position()),
                    Vec::new(),
                ),
            };
        MonsterAbilityCandidateResolutionDto {
            ability_id: plan.ability.id.clone(),
            base_weight: plan.base_weight,
            effective_weight: plan.effective_weight,
            target_entity_id,
            target_kind_id,
            target_position,
            affected_positions,
            enemy_target_count: plan.enemy_target_count,
            friendly_risk_count: plan.friendly_risk_count,
            rejection_reason,
        }
    }

    fn resolve_monster_ability_plan(
        &mut self,
        source_index: usize,
        source_kind_id: &str,
        plan: &MonsterAbilityPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> MonsterAbilityPlanResolution {
        let source_entity_id = self.entities[source_index].id.clone();
        match &plan.target {
            MonsterAbilityTargetPlan::SelfTarget => {
                let target_entity_id = self.entities[source_index].id.clone();
                let target_kind_id = self.entities[source_index].kind_id.clone();
                let target_position = self.entities[source_index].position;
                let effects = self.resolve_monster_self_effects(source_index, &plan.ability);
                changed.insert(target_position);
                MonsterAbilityPlanResolution {
                    target_entity_id: target_entity_id.clone(),
                    target_kind_id: target_kind_id.clone(),
                    affected_positions: Vec::new(),
                    summon: None,
                    effects: effects.clone(),
                    targets: vec![MonsterAbilityTargetResolutionDto {
                        target_entity_id,
                        target_kind_id,
                        target_position,
                        effects,
                    }],
                    trace: None,
                }
            }
            MonsterAbilityTargetPlan::Projectile { target, trace } => {
                let effects = self.resolve_monster_hostile_effects(
                    &source_entity_id,
                    source_kind_id,
                    &plan.ability,
                    target,
                    events,
                    changed,
                );
                changed.insert(target.position());
                let targets = vec![MonsterAbilityTargetResolutionDto {
                    target_entity_id: target.entity_id().to_owned(),
                    target_kind_id: target.kind_id().to_owned(),
                    target_position: target.position(),
                    effects: effects.clone(),
                }];
                self.remove_defeated_player_summons(
                    targets
                        .iter()
                        .map(|target| target.target_entity_id.as_str()),
                    changed,
                    removed_entities,
                );
                MonsterAbilityPlanResolution {
                    target_entity_id: target.entity_id().to_owned(),
                    target_kind_id: target.kind_id().to_owned(),
                    affected_positions: vec![target.position()],
                    summon: None,
                    effects,
                    targets,
                    trace: Some(trace.clone()),
                }
            }
            MonsterAbilityTargetPlan::Area {
                target,
                trace,
                affected_positions,
            } => {
                let AbilityEffectDefinition::AreaDamage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    damage_type,
                    ..
                } = &plan.ability.effect
                else {
                    unreachable!("monster area plan must retain an area effect");
                };
                let raw_damage = self
                    .roll_damage(*damage_dice, *damage_sides)
                    .saturating_add(i32::from(*damage_bonus))
                    .max(0);
                let target_actors =
                    self.monster_targets_in_footprint(source_index, target, affected_positions);
                let mut targets = Vec::with_capacity(target_actors.len());
                for affected_target in target_actors {
                    let position = affected_target.position();
                    let distance = target
                        .position()
                        .x
                        .abs_diff(position.x)
                        .max(target.position().y.abs_diff(position.y));
                    let prepared = rfb_area_damage(raw_damage, distance);
                    let effect = self.resolve_monster_damage_to_hostile(
                        &source_entity_id,
                        source_kind_id,
                        &plan.ability.id,
                        0,
                        raw_damage,
                        prepared,
                        DamageType::from(*damage_type),
                        &affected_target,
                        events,
                    );
                    changed.insert(position);
                    targets.push(MonsterAbilityTargetResolutionDto {
                        target_entity_id: affected_target.entity_id().to_owned(),
                        target_kind_id: affected_target.kind_id().to_owned(),
                        target_position: position,
                        effects: vec![effect],
                    });
                }
                let effects = targets
                    .iter()
                    .find(|resolution| resolution.target_entity_id == target.entity_id())
                    .map(|resolution| resolution.effects.clone())
                    .unwrap_or_default();
                self.remove_defeated_player_summons(
                    targets
                        .iter()
                        .map(|target| target.target_entity_id.as_str()),
                    changed,
                    removed_entities,
                );
                MonsterAbilityPlanResolution {
                    target_entity_id: target.entity_id().to_owned(),
                    target_kind_id: target.kind_id().to_owned(),
                    affected_positions: affected_positions.clone(),
                    summon: None,
                    effects,
                    targets,
                    trace: Some(trace.clone()),
                }
            }
            MonsterAbilityTargetPlan::Beam {
                target,
                trace,
                affected_positions,
            } => {
                let AbilityEffectDefinition::BeamDamage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    damage_type,
                } = &plan.ability.effect
                else {
                    unreachable!("monster beam plan must retain a beam effect");
                };
                let raw_damage = self
                    .roll_damage(*damage_dice, *damage_sides)
                    .saturating_add(i32::from(*damage_bonus))
                    .max(0);
                let target_actors =
                    self.monster_targets_in_footprint(source_index, target, affected_positions);
                let mut targets = Vec::with_capacity(target_actors.len());
                for affected_target in target_actors {
                    let effect = self.resolve_monster_damage_to_hostile(
                        &source_entity_id,
                        source_kind_id,
                        &plan.ability.id,
                        0,
                        raw_damage,
                        raw_damage,
                        DamageType::from(*damage_type),
                        &affected_target,
                        events,
                    );
                    changed.insert(affected_target.position());
                    targets.push(MonsterAbilityTargetResolutionDto {
                        target_entity_id: affected_target.entity_id().to_owned(),
                        target_kind_id: affected_target.kind_id().to_owned(),
                        target_position: affected_target.position(),
                        effects: vec![effect],
                    });
                }
                let effects = targets
                    .iter()
                    .find(|resolution| resolution.target_entity_id == target.entity_id())
                    .map(|resolution| resolution.effects.clone())
                    .unwrap_or_default();
                self.remove_defeated_player_summons(
                    targets
                        .iter()
                        .map(|target| target.target_entity_id.as_str()),
                    changed,
                    removed_entities,
                );
                MonsterAbilityPlanResolution {
                    target_entity_id: target.entity_id().to_owned(),
                    target_kind_id: target.kind_id().to_owned(),
                    affected_positions: affected_positions.clone(),
                    summon: None,
                    effects,
                    targets,
                    trace: Some(trace.clone()),
                }
            }
            MonsterAbilityTargetPlan::Cone {
                target,
                trace,
                affected_positions,
            } => {
                let (raw_damage, damage_type, radius) = match &plan.ability.effect {
                    AbilityEffectDefinition::ConeDamage {
                        damage_dice,
                        damage_sides,
                        damage_bonus,
                        damage_type,
                        radius,
                    } => (
                        self.roll_damage(*damage_dice, *damage_sides)
                            .saturating_add(i32::from(*damage_bonus))
                            .max(0),
                        damage_type,
                        radius,
                    ),
                    AbilityEffectDefinition::BreathDamage {
                        hp_percent,
                        max_damage,
                        damage_type,
                        radius,
                    } => {
                        // Breath scales with the caster's current vigor: no
                        // damage dice are rolled, and the elemental cap
                        // bounds a healthy caster.
                        let caster_hp = self
                            .entities
                            .iter()
                            .find(|entity| entity.id == source_entity_id)
                            .map_or(0, |entity| entity.hp)
                            .max(0);
                        let scaled = caster_hp
                            .saturating_mul(i32::from(*hp_percent))
                            .div_euclid(100);
                        (scaled.min(i32::from(*max_damage)), damage_type, radius)
                    }
                    _ => unreachable!("monster cone plan must retain a cone or breath effect"),
                };
                let origin = self
                    .entities
                    .iter()
                    .find(|entity| entity.id == source_entity_id)
                    .map_or(trace.origin, |entity| entity.position);
                let direction = direction_toward(origin, target.position())
                    .expect("validated monster cone retains a direction");
                let lateral_distances = self
                    .cone_damage_cells(origin, &trace.traversed, direction, *radius)
                    .into_iter()
                    .map(|(_, lateral, position)| (position, lateral))
                    .collect::<BTreeMap<_, _>>();
                let target_actors =
                    self.monster_targets_in_footprint(source_index, target, affected_positions);
                let mut targets = Vec::with_capacity(target_actors.len());
                for affected_target in target_actors {
                    let lateral_distance = lateral_distances
                        .get(&affected_target.position())
                        .copied()
                        .unwrap_or(0);
                    let prepared = rfb_area_damage(raw_damage, lateral_distance);
                    let effect = self.resolve_monster_damage_to_hostile(
                        &source_entity_id,
                        source_kind_id,
                        &plan.ability.id,
                        0,
                        raw_damage,
                        prepared,
                        DamageType::from(*damage_type),
                        &affected_target,
                        events,
                    );
                    changed.insert(affected_target.position());
                    targets.push(MonsterAbilityTargetResolutionDto {
                        target_entity_id: affected_target.entity_id().to_owned(),
                        target_kind_id: affected_target.kind_id().to_owned(),
                        target_position: affected_target.position(),
                        effects: vec![effect],
                    });
                }
                let effects = targets
                    .iter()
                    .find(|resolution| resolution.target_entity_id == target.entity_id())
                    .map(|resolution| resolution.effects.clone())
                    .unwrap_or_default();
                self.remove_defeated_player_summons(
                    targets
                        .iter()
                        .map(|target| target.target_entity_id.as_str()),
                    changed,
                    removed_entities,
                );
                MonsterAbilityPlanResolution {
                    target_entity_id: target.entity_id().to_owned(),
                    target_kind_id: target.kind_id().to_owned(),
                    affected_positions: affected_positions.clone(),
                    summon: None,
                    effects,
                    targets,
                    trace: Some(trace.clone()),
                }
            }
            MonsterAbilityTargetPlan::Summon { positions } => {
                let AbilityEffectDefinition::Summon {
                    ref actor_kind_id,
                    duration_turns,
                    ..
                } = plan.ability.effect
                else {
                    unreachable!("monster summon plan must retain a summon effect");
                };
                let definition = self
                    .content
                    .actor(actor_kind_id)
                    .expect("validated summoned actor must remain available")
                    .clone();
                let owner_id = self.entities[source_index].id.clone();
                let mut entity_ids = Vec::with_capacity(positions.len());
                for (ordinal, position) in positions.iter().copied().enumerate() {
                    let id = self.summon_entity_id(&plan.ability.id, ordinal);
                    let mut entity = actor_from_runtime_spawn(
                        &id,
                        actor_kind_id,
                        position,
                        definition.max_hp,
                        definition.speed,
                        INITIAL_MONSTER_ENERGY_NEED,
                        true,
                    );
                    entity.resistances = definition_resistance_profile(&definition);
                    entity.summon = Some(SummonIdentity {
                        owner_id: owner_id.clone(),
                        source_ability_id: plan.ability.id.clone(),
                        remaining_turns: duration_turns,
                    });
                    changed.insert(position);
                    entity_ids.push(id);
                    self.entities.push(entity);
                }
                let summon = AbilitySummonResolutionDto {
                    owner_id: owner_id.clone(),
                    actor_kind_id: actor_kind_id.clone(),
                    entity_ids,
                    positions: positions.clone(),
                    duration_turns,
                    hostile: false,
                    group: false,
                    summoned_kind_ids: Vec::new(),
                };
                MonsterAbilityPlanResolution {
                    target_entity_id: owner_id,
                    target_kind_id: self.entities[source_index].kind_id.clone(),
                    affected_positions: positions.clone(),
                    summon: Some(summon),
                    effects: Vec::new(),
                    targets: Vec::new(),
                    trace: None,
                }
            }
            MonsterAbilityTargetPlan::SummonCategory {
                candidate_kind_ids,
                positions,
            } => {
                let AbilityEffectDefinition::SummonCategory {
                    ref category,
                    count_dice,
                    count_sides,
                    count_bonus,
                    duration_turns,
                    ..
                } = plan.ability.effect
                else {
                    unreachable!("monster category summon plan must retain its effect");
                };
                // The count dice roll first, then one bounded draw picks each
                // summon's kind; space shortfalls truncate to the secured
                // cells (planning guaranteed at least one).
                let rolled = self
                    .roll_damage(u16::from(count_dice), u16::from(count_sides))
                    .saturating_add(i32::from(count_bonus))
                    .max(1);
                let count = usize::try_from(rolled).unwrap_or(1).min(positions.len());
                let owner_id = self.entities[source_index].id.clone();
                let mut entity_ids = Vec::with_capacity(count);
                let mut summoned_kind_ids = Vec::with_capacity(count);
                let mut used_positions = Vec::with_capacity(count);
                let planned_positions = positions.iter().copied().take(count).collect::<Vec<_>>();
                for (ordinal, position) in planned_positions.into_iter().enumerate() {
                    let choice = usize::try_from(self.rng.bounded(
                        u64::try_from(candidate_kind_ids.len()).expect("candidate count fits"),
                    ))
                    .expect("bounded draw fits usize");
                    let kind_id = candidate_kind_ids[choice].clone();
                    let definition = self
                        .content
                        .actor(&kind_id)
                        .expect("validated summon candidate must remain available")
                        .clone();
                    let id = self.summon_entity_id(&plan.ability.id, ordinal);
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
                    entity.summon = Some(SummonIdentity {
                        owner_id: owner_id.clone(),
                        source_ability_id: plan.ability.id.clone(),
                        remaining_turns: duration_turns,
                    });
                    changed.insert(position);
                    entity_ids.push(id);
                    summoned_kind_ids.push(kind_id);
                    used_positions.push(position);
                    self.entities.push(entity);
                }
                let summon = AbilitySummonResolutionDto {
                    owner_id: owner_id.clone(),
                    actor_kind_id: category.clone(),
                    entity_ids,
                    positions: used_positions.clone(),
                    duration_turns,
                    hostile: false,
                    group: false,
                    summoned_kind_ids,
                };
                MonsterAbilityPlanResolution {
                    target_entity_id: owner_id,
                    target_kind_id: self.entities[source_index].kind_id.clone(),
                    affected_positions: used_positions,
                    summon: Some(summon),
                    effects: Vec::new(),
                    targets: Vec::new(),
                    trace: None,
                }
            }
            MonsterAbilityTargetPlan::BlinkSelf { destinations }
            | MonsterAbilityTargetPlan::EscapeSelf { destinations } => {
                // The candidate list was collected without RNG at planning
                // time; the actual landing cell consumes one bounded draw.
                let choice = usize::try_from(
                    self.rng
                        .bounded(u64::try_from(destinations.len()).expect("candidate count fits")),
                )
                .expect("bounded draw fits usize");
                let destination = destinations[choice];
                let from = self.entities[source_index].position;
                self.entities[source_index].position = destination;
                changed.insert(from);
                changed.insert(destination);
                let resolution = MonsterDisplacementResolutionDto {
                    actor_id: source_entity_id.clone(),
                    from,
                    to: destination,
                };
                events.push(
                    if matches!(plan.target, MonsterAbilityTargetPlan::BlinkSelf { .. }) {
                        DomainEvent::MonsterBlinked {
                            source_kind_id: source_kind_id.to_owned(),
                            resolution,
                        }
                    } else {
                        DomainEvent::MonsterTeleported {
                            source_kind_id: source_kind_id.to_owned(),
                            resolution,
                        }
                    },
                );
                MonsterAbilityPlanResolution {
                    target_entity_id: source_entity_id.clone(),
                    target_kind_id: self.entities[source_index].kind_id.clone(),
                    affected_positions: vec![from, destination],
                    summon: None,
                    effects: Vec::new(),
                    targets: Vec::new(),
                    trace: None,
                }
            }
            MonsterAbilityTargetPlan::BanishTarget {
                target,
                trace,
                destinations,
            } => {
                // One bounded draw picks the landing cell from the
                // plan-collected candidates, mirroring the escape family.
                let choice = usize::try_from(
                    self.rng
                        .bounded(u64::try_from(destinations.len()).expect("candidate count fits")),
                )
                .expect("bounded draw fits usize");
                let destination = destinations[choice];
                match target {
                    MonsterHostileTarget::Player { .. } => {
                        let from = self.player.position;
                        events.push(DomainEvent::MonsterBanishedTarget {
                            source_kind_id: source_kind_id.to_owned(),
                            target_kind_id: target.kind_id().to_owned(),
                            resolution: MonsterDisplacementResolutionDto {
                                actor_id: target.entity_id().to_owned(),
                                from,
                                to: destination,
                            },
                        });
                        let relocation = self.relocate_player(destination, changed);
                        events.extend(relocation);
                    }
                    MonsterHostileTarget::Summon { entity_id, .. } => {
                        if let Some(banished_index) = self
                            .entities
                            .iter()
                            .position(|entity| entity.id == *entity_id && entity.hp > 0)
                        {
                            let from = self.entities[banished_index].position;
                            self.entities[banished_index].position = destination;
                            changed.insert(from);
                            changed.insert(destination);
                            events.push(DomainEvent::MonsterBanishedTarget {
                                source_kind_id: source_kind_id.to_owned(),
                                target_kind_id: target.kind_id().to_owned(),
                                resolution: MonsterDisplacementResolutionDto {
                                    actor_id: entity_id.clone(),
                                    from,
                                    to: destination,
                                },
                            });
                        }
                    }
                }
                MonsterAbilityPlanResolution {
                    target_entity_id: target.entity_id().to_owned(),
                    target_kind_id: target.kind_id().to_owned(),
                    affected_positions: vec![destination],
                    summon: None,
                    effects: Vec::new(),
                    targets: vec![MonsterAbilityTargetResolutionDto {
                        target_entity_id: target.entity_id().to_owned(),
                        target_kind_id: target.kind_id().to_owned(),
                        target_position: destination,
                        effects: Vec::new(),
                    }],
                    trace: Some(trace.clone()),
                }
            }
            MonsterAbilityTargetPlan::DragTarget {
                target,
                trace,
                destination,
            } => {
                let destination = *destination;
                match target {
                    MonsterHostileTarget::Player { .. } => {
                        let from = self.player.position;
                        events.push(DomainEvent::MonsterDraggedTarget {
                            source_kind_id: source_kind_id.to_owned(),
                            target_kind_id: target.kind_id().to_owned(),
                            resolution: MonsterDisplacementResolutionDto {
                                actor_id: target.entity_id().to_owned(),
                                from,
                                to: destination,
                            },
                        });
                        let relocation = self.relocate_player(destination, changed);
                        events.extend(relocation);
                    }
                    MonsterHostileTarget::Summon { entity_id, .. } => {
                        if let Some(dragged_index) = self
                            .entities
                            .iter()
                            .position(|entity| entity.id == *entity_id && entity.hp > 0)
                        {
                            let from = self.entities[dragged_index].position;
                            self.entities[dragged_index].position = destination;
                            changed.insert(from);
                            changed.insert(destination);
                            events.push(DomainEvent::MonsterDraggedTarget {
                                source_kind_id: source_kind_id.to_owned(),
                                target_kind_id: target.kind_id().to_owned(),
                                resolution: MonsterDisplacementResolutionDto {
                                    actor_id: entity_id.clone(),
                                    from,
                                    to: destination,
                                },
                            });
                        }
                    }
                }
                MonsterAbilityPlanResolution {
                    target_entity_id: target.entity_id().to_owned(),
                    target_kind_id: target.kind_id().to_owned(),
                    affected_positions: vec![destination],
                    summon: None,
                    effects: Vec::new(),
                    targets: vec![MonsterAbilityTargetResolutionDto {
                        target_entity_id: target.entity_id().to_owned(),
                        target_kind_id: target.kind_id().to_owned(),
                        target_position: destination,
                        effects: Vec::new(),
                    }],
                    trace: Some(trace.clone()),
                }
            }
        }
    }

    fn monster_targets_in_footprint(
        &self,
        source_index: usize,
        primary: &MonsterHostileTarget,
        affected_positions: &[Position],
    ) -> Vec<MonsterHostileTarget> {
        let mut targets = self
            .monster_hostile_targets(source_index)
            .into_iter()
            .filter(|target| affected_positions.contains(&target.position()))
            .collect::<Vec<_>>();
        targets.sort_by(|left, right| {
            (left.entity_id() != primary.entity_id())
                .cmp(&(right.entity_id() != primary.entity_id()))
                .then_with(|| left.entity_id().cmp(right.entity_id()))
        });
        targets
    }

    fn remove_defeated_player_summons<'a>(
        &mut self,
        target_entity_ids: impl Iterator<Item = &'a str>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) {
        let mut defeated = target_entity_ids
            .filter_map(|entity_id| {
                self.entities
                    .iter()
                    .position(|entity| {
                        entity.id == entity_id
                            && entity.hp <= 0
                            && self.actor_is_player_aligned(entity)
                    })
                    .map(|index| self.entities[index].id.clone())
            })
            .collect::<Vec<_>>();
        defeated.sort();
        defeated.dedup();
        for entity_id in defeated {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id)
            else {
                continue;
            };
            let removed = self.entities.remove(index);
            changed.insert(removed.position);
            removed_entities.push(removed.id);
        }
    }

    fn resolve_monster_self_effects(
        &mut self,
        source_index: usize,
        ability: &AbilityDefinition,
    ) -> Vec<AbilityEffectResolutionDto> {
        let effects = ability.effect.ordered_effects();
        let mut resolutions = Vec::with_capacity(effects.len());
        for (index, effect) in effects.iter().enumerate() {
            let effect_index =
                u8::try_from(index).expect("validated monster ability effect index must fit u8");
            let resolution = match effect {
                AbilityEffectDefinition::Heal { amount } => {
                    let amount =
                        i32::try_from(*amount).expect("validated healing amount must fit i32");
                    let actor = &mut self.entities[source_index];
                    let outcome = apply_effect(
                        &mut EffectTarget {
                            hp: &mut actor.hp,
                            max_hp: actor.max_hp,
                            resistances: &actor.resistances,
                            statuses: &mut actor.statuses,
                        },
                        EffectSpec::Heal { amount },
                    );
                    let EffectOutcome::Healed { requested, applied } = outcome else {
                        unreachable!("monster healing must produce a healing outcome");
                    };
                    AbilityEffectResolutionDto::Heal {
                        effect_index,
                        resolution: HealingResolutionDto { requested, applied },
                    }
                }
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
                } => apply_ability_status_effect(
                    &mut self.entities[source_index],
                    &ability.id,
                    effect_index,
                    status_kind_id,
                    *intensity,
                    *duration_ticks,
                    *duration_dice,
                    *duration_sides,
                    *stacking,
                    *resistance_type,
                    *power,
                    granted_resistances,
                    granted_brands,
                    granted_modifiers,
                    granted_equipment_bonuses,
                    granted_status_immunities,
                    granted_race_id.as_deref(),
                    *grants_wall_passage,
                    *incoming_damage_percent,
                    None,
                    None,
                    &mut self.rng,
                ),
                AbilityEffectDefinition::RemoveStatus { status_kind_id } => {
                    remove_ability_status_effect(
                        &mut self.entities[source_index],
                        effect_index,
                        status_kind_id,
                    )
                }
                _ => unreachable!("validated monster self effects must remain actor effects"),
            };
            resolutions.push(resolution);
        }
        resolutions
    }

    fn resolve_monster_hostile_effects(
        &mut self,
        source_entity_id: &str,
        source_kind_id: &str,
        ability: &AbilityDefinition,
        target: &MonsterHostileTarget,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Vec<AbilityEffectResolutionDto> {
        if target.is_player() {
            return self.resolve_monster_player_effects(
                source_entity_id,
                source_kind_id,
                ability,
                events,
                changed,
            );
        }
        let effects = ability.effect.ordered_effects();
        let mut resolutions = Vec::with_capacity(effects.len());
        for (index, effect) in effects.iter().enumerate() {
            let effect_index =
                u8::try_from(index).expect("validated monster ability effect index must fit u8");
            let Some(target_index) = self
                .entities
                .iter()
                .position(|entity| entity.id == target.entity_id())
            else {
                resolutions.push(AbilityEffectResolutionDto::Skipped {
                    effect_index,
                    reason: AbilityEffectSkipReasonDto::TargetDead,
                });
                continue;
            };
            if self.entities[target_index].hp <= 0 {
                resolutions.push(AbilityEffectResolutionDto::Skipped {
                    effect_index,
                    reason: AbilityEffectSkipReasonDto::TargetDead,
                });
                continue;
            }
            let resolution = match effect {
                AbilityEffectDefinition::Damage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    damage_type,
                } => {
                    let raw_damage = self
                        .roll_damage(*damage_dice, *damage_sides)
                        .saturating_add(i32::from(*damage_bonus))
                        .max(0);
                    let damage_type = DamageType::from(*damage_type);
                    let definition = self
                        .content
                        .actor(&self.entities[target_index].kind_id)
                        .expect("monster target definition must remain available");
                    let armor_class = self
                        .actor_derived_stats(&self.entities[target_index], definition, false)
                        .armor_class
                        .value;
                    let prepared = if damage_type == DamageType::Physical {
                        apply_melee_armor_reduction(raw_damage, armor_class)
                    } else {
                        raw_damage
                    };
                    self.resolve_monster_damage_to_hostile(
                        source_entity_id,
                        source_kind_id,
                        &ability.id,
                        effect_index,
                        raw_damage,
                        prepared,
                        damage_type,
                        target,
                        events,
                    )
                }
                AbilityEffectDefinition::CurseDamage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                } => {
                    // Summoned targets have no saving-throw skill; the curse
                    // lands in full (documented v98 simplification).
                    let raw_damage = self
                        .roll_damage(*damage_dice, *damage_sides)
                        .saturating_add(i32::from(*damage_bonus))
                        .max(0);
                    self.resolve_monster_damage_to_hostile(
                        source_entity_id,
                        source_kind_id,
                        &ability.id,
                        effect_index,
                        raw_damage,
                        raw_damage,
                        DamageType::Curse,
                        target,
                        events,
                    )
                }
                AbilityEffectDefinition::DrainResource { .. }
                | AbilityEffectDefinition::Amnesia => {
                    // Summons carry no resource pools or map knowledge; both
                    // effects fizzle against them (documented v99 boundary).
                    AbilityEffectResolutionDto::Skipped {
                        effect_index,
                        reason: AbilityEffectSkipReasonDto::NoTarget,
                    }
                }
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
                } => {
                    let target_level = self
                        .content
                        .actor(&self.entities[target_index].kind_id)
                        .map(|definition| definition.level);
                    apply_ability_status_effect(
                        &mut self.entities[target_index],
                        &ability.id,
                        effect_index,
                        status_kind_id,
                        *intensity,
                        *duration_ticks,
                        *duration_dice,
                        *duration_sides,
                        *stacking,
                        *resistance_type,
                        *power,
                        granted_resistances,
                        granted_brands,
                        granted_modifiers,
                        granted_equipment_bonuses,
                        granted_status_immunities,
                        granted_race_id.as_deref(),
                        *grants_wall_passage,
                        *incoming_damage_percent,
                        target_level,
                        None,
                        &mut self.rng,
                    )
                }
                AbilityEffectDefinition::RemoveStatus { status_kind_id } => {
                    remove_ability_status_effect(
                        &mut self.entities[target_index],
                        effect_index,
                        status_kind_id,
                    )
                }
                _ => unreachable!(
                    "validated monster abilities contain only direct actor-target effects"
                ),
            };
            resolutions.push(resolution);
        }
        resolutions
    }

    fn resolve_monster_player_effects(
        &mut self,
        source_entity_id: &str,
        source_kind_id: &str,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Vec<AbilityEffectResolutionDto> {
        let effects = ability.effect.ordered_effects();
        let mut resolutions = Vec::with_capacity(effects.len());
        for (index, effect) in effects.iter().enumerate() {
            let effect_index =
                u8::try_from(index).expect("validated monster ability effect index must fit u8");
            if self.player_is_dead() {
                resolutions.push(AbilityEffectResolutionDto::Skipped {
                    effect_index,
                    reason: AbilityEffectSkipReasonDto::TargetDead,
                });
                continue;
            }
            let resolution = match effect {
                AbilityEffectDefinition::Damage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    damage_type,
                } => {
                    let raw_damage = self
                        .roll_damage(*damage_dice, *damage_sides)
                        .saturating_add(i32::from(*damage_bonus))
                        .max(0);
                    let damage_type = DamageType::from(*damage_type);
                    let target = self.player_derived_stats();
                    let prepared = if damage_type == DamageType::Physical {
                        apply_melee_armor_reduction(raw_damage, target.armor_class.value)
                    } else {
                        raw_damage
                    };
                    self.resolve_monster_damage_to_player(
                        source_entity_id,
                        source_kind_id,
                        &ability.id,
                        effect_index,
                        raw_damage,
                        prepared,
                        damage_type,
                        events,
                    )
                }
                AbilityEffectDefinition::CurseDamage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                } => {
                    // A successful saving throw negates the curse before any
                    // damage dice are drawn; difficulty follows the caster's
                    // definition level.
                    let ability_stat = self.player_derived_stats().saving_throw_skill;
                    let caster_level = self
                        .content
                        .actor(source_kind_id)
                        .map_or(1, |definition| definition.level);
                    let mut difficulty_pipeline = DerivedStatsPipeline::new();
                    difficulty_pipeline.add(
                        StatKind::ActionDifficulty,
                        StatLayer::Environment,
                        source_kind_id,
                        i32::try_from(caster_level).unwrap_or(i32::MAX),
                    );
                    let check = resolve_check(
                        &mut self.rng,
                        CheckContext {
                            kind: CheckKind::SavingThrow,
                            actor_id: self.player.id.clone(),
                            target_id: Some(source_kind_id.to_owned()),
                            ability: ability_stat,
                            difficulty: difficulty_pipeline
                                .resolve(StatKind::ActionDifficulty, StatBounds::NON_NEGATIVE),
                        },
                    );
                    let succeeded = check.succeeded();
                    let skill_id = self
                        .content
                        .skill_by_kind(SkillKind::SavingThrow)
                        .expect("validated saving throw skill must remain available")
                        .id
                        .clone();
                    events.push(DomainEvent::SavingThrowChecked {
                        source_kind_id: source_kind_id.to_owned(),
                        position: self.player.position,
                        succeeded,
                        resolution: check.to_dto(skill_id),
                    });
                    if succeeded {
                        AbilityEffectResolutionDto::Skipped {
                            effect_index,
                            reason: AbilityEffectSkipReasonDto::Saved,
                        }
                    } else {
                        let raw_damage = self
                            .roll_damage(*damage_dice, *damage_sides)
                            .saturating_add(i32::from(*damage_bonus))
                            .max(0);
                        self.resolve_monster_damage_to_player(
                            source_entity_id,
                            source_kind_id,
                            &ability.id,
                            effect_index,
                            raw_damage,
                            raw_damage,
                            DamageType::Curse,
                            events,
                        )
                    }
                }
                AbilityEffectDefinition::DrainResource { amount } => {
                    // The casting-profile pool is drained when present; other
                    // players lose their first non-empty pool in id order.
                    let pool_id = self
                        .casting_profile()
                        .map(|profile| profile.resource_id.clone())
                        .filter(|id| self.resources.contains_key(id))
                        .or_else(|| {
                            self.resources
                                .iter()
                                .find(|(_, pool)| pool.current > 0)
                                .map(|(id, _)| id.clone())
                        });
                    let requested = *amount;
                    let (resource_id, drained) = match pool_id {
                        Some(id) => {
                            let pool = self
                                .resources
                                .get_mut(&id)
                                .expect("selected drain pool must remain available");
                            let drained = pool.current.min(requested);
                            pool.current -= drained;
                            (id, drained)
                        }
                        None => (String::new(), 0),
                    };
                    // The caster feeds on the stolen power, capped at its
                    // own maximum life.
                    let mut caster_healed = 0_u32;
                    if drained > 0
                        && let Some(caster_index) = self
                            .entities
                            .iter()
                            .position(|entity| entity.id == *source_entity_id)
                    {
                        let caster = &mut self.entities[caster_index];
                        let missing = caster.max_hp.saturating_sub(caster.hp).max(0);
                        let healed = i32::try_from(drained).unwrap_or(i32::MAX).min(missing);
                        caster.hp += healed;
                        caster_healed = u32::try_from(healed).unwrap_or(0);
                        changed.insert(caster.position);
                    }
                    AbilityEffectResolutionDto::DrainResource {
                        effect_index,
                        resource_id,
                        requested,
                        drained,
                        caster_healed,
                    }
                }
                AbilityEffectDefinition::Amnesia => {
                    // The saving throw gates the memory wipe exactly like the
                    // curse family; success costs no further RNG.
                    let ability_stat = self.player_derived_stats().saving_throw_skill;
                    let caster_level = self
                        .content
                        .actor(source_kind_id)
                        .map_or(1, |definition| definition.level);
                    let mut difficulty_pipeline = DerivedStatsPipeline::new();
                    difficulty_pipeline.add(
                        StatKind::ActionDifficulty,
                        StatLayer::Environment,
                        source_kind_id,
                        i32::try_from(caster_level).unwrap_or(i32::MAX),
                    );
                    let check = resolve_check(
                        &mut self.rng,
                        CheckContext {
                            kind: CheckKind::SavingThrow,
                            actor_id: self.player.id.clone(),
                            target_id: Some(source_kind_id.to_owned()),
                            ability: ability_stat,
                            difficulty: difficulty_pipeline
                                .resolve(StatKind::ActionDifficulty, StatBounds::NON_NEGATIVE),
                        },
                    );
                    let succeeded = check.succeeded();
                    let skill_id = self
                        .content
                        .skill_by_kind(SkillKind::SavingThrow)
                        .expect("validated saving throw skill must remain available")
                        .id
                        .clone();
                    events.push(DomainEvent::SavingThrowChecked {
                        source_kind_id: source_kind_id.to_owned(),
                        position: self.player.position,
                        succeeded,
                        resolution: check.to_dto(skill_id),
                    });
                    if succeeded {
                        AbilityEffectResolutionDto::Skipped {
                            effect_index,
                            reason: AbilityEffectSkipReasonDto::Saved,
                        }
                    } else {
                        // Only the current floor map memory fades; item
                        // knowledge stays authoritative per the long-term
                        // design constraints.
                        let width = usize::from(self.width);
                        let mut cleared_cells = 0_u32;
                        for (index, explored) in self.explored.iter_mut().enumerate() {
                            if *explored {
                                *explored = false;
                                cleared_cells += 1;
                                changed.insert(Position {
                                    x: i32::try_from(index % width)
                                        .expect("explored x must fit i32"),
                                    y: i32::try_from(index / width)
                                        .expect("explored y must fit i32"),
                                });
                            }
                        }
                        cleared_cells += u32::try_from(self.revealed_terrain.len()).unwrap_or(0);
                        self.revealed_terrain.clear();
                        AbilityEffectResolutionDto::Amnesia {
                            effect_index,
                            cleared_cells,
                        }
                    }
                }
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
                } => {
                    let effective = self.effective_player_resistances();
                    let immunities = self.player_status_immunities();
                    let target_level = u32::from(self.progress.level);
                    let resolution = apply_ability_status_effect(
                        &mut self.player,
                        &ability.id,
                        effect_index,
                        status_kind_id,
                        *intensity,
                        *duration_ticks,
                        *duration_dice,
                        *duration_sides,
                        *stacking,
                        *resistance_type,
                        *power,
                        granted_resistances,
                        granted_brands,
                        granted_modifiers,
                        granted_equipment_bonuses,
                        granted_status_immunities,
                        granted_race_id.as_deref(),
                        *grants_wall_passage,
                        *incoming_damage_percent,
                        Some(target_level),
                        Some((&effective, &immunities)),
                        &mut self.rng,
                    );
                    if let Some(damage_type) = resistance_type.map(DamageType::from) {
                        let level = effective.level(damage_type);
                        self.record_monster_player_resistance(source_entity_id, damage_type, level);
                    }
                    resolution
                }
                AbilityEffectDefinition::RemoveStatus { status_kind_id } => {
                    remove_ability_status_effect(&mut self.player, effect_index, status_kind_id)
                }
                _ => unreachable!(
                    "validated monster abilities contain only direct actor-target effects"
                ),
            };
            resolutions.push(resolution);
        }
        resolutions
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

    fn generate_procedural_floor(
        &mut self,
        definition: &ProceduralFloorDefinition,
        dungeon_instance_id: Option<String>,
    ) -> Result<FloorState, CoreError> {
        let maze_only = definition
            .layout
            .as_ref()
            .is_some_and(|layout| layout.mode == ProceduralLayoutMode::MazeOnly);
        let selected_region_entries = if let Some(table_id) = &definition.region_table_id {
            let table = self
                .content
                .region_table(table_id)
                .expect("validated region table must remain available")
                .clone();
            let mut eligible = table
                .entries
                .into_iter()
                .filter(|entry| {
                    entry.min_depth <= definition.depth && definition.depth <= entry.max_depth
                })
                .collect::<Vec<_>>();
            let placement_count = definition
                .generation_budget
                .as_ref()
                .and_then(|budget| budget.region_placements)
                .expect("validated region floor must retain a placement budget");
            let mut selected = Vec::with_capacity(usize::from(placement_count));
            for _ in 0..placement_count {
                let weights = eligible
                    .iter()
                    .map(|entry| entry.weight)
                    .collect::<Vec<_>>();
                let selected_index = self.roll_weighted_index(&weights);
                selected.push(eligible.remove(selected_index));
            }
            selected
        } else {
            Vec::new()
        };
        let eligible_themes = definition
            .theme_table_id
            .as_ref()
            .and_then(|table_id| self.content.theme_table(table_id))
            .map(|table| {
                table
                    .entries
                    .iter()
                    .filter(|entry| {
                        entry.min_depth <= definition.depth && definition.depth <= entry.max_depth
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let selected_theme = if eligible_themes.is_empty() {
            None
        } else if eligible_themes.len() == 1 {
            Some(eligible_themes[0].clone())
        } else {
            let weights = eligible_themes
                .iter()
                .map(|entry| entry.weight)
                .collect::<Vec<_>>();
            Some(eligible_themes[self.roll_weighted_index(&weights)].clone())
        };
        let generated_floor_terrain_id = selected_theme
            .as_ref()
            .map(|entry| entry.floor_terrain_id.clone())
            .unwrap_or_else(|| definition.floor_terrain_id.clone());
        let uses_spatial_vault_budget =
            definition.generation_budget.as_ref().is_some_and(|budget| {
                budget.vault_placements.is_some() && budget.vault_area_tiles.is_some()
            });
        let eligible_vault_candidates = selected_theme
            .as_ref()
            .map(|theme| {
                theme
                    .vault_candidates
                    .iter()
                    .filter(|candidate| {
                        candidate.min_depth <= definition.depth
                            && definition.depth <= candidate.max_depth
                            && self
                                .content
                                .vault(&candidate.vault_id)
                                .is_some_and(|vault| {
                                    uses_spatial_vault_budget
                                        || vault.width <= 6
                                            && vault.height <= 5
                                            && vault.entrance_positions.len() == 1
                                            && vault.entrance_positions[0].x == vault.width / 2
                                            && vault.entrance_positions[0].y == 0
                                })
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let legacy_vault = if uses_spatial_vault_budget || maze_only {
            None
        } else if eligible_vault_candidates.is_empty() {
            definition
                .vault_id
                .as_ref()
                .and_then(|vault_id| self.content.vault(vault_id))
                .cloned()
        } else if eligible_vault_candidates.len() == 1 {
            self.content
                .vault(&eligible_vault_candidates[0].vault_id)
                .cloned()
        } else {
            let weights = eligible_vault_candidates
                .iter()
                .map(|candidate| candidate.weight)
                .collect::<Vec<_>>();
            let vault_id = &eligible_vault_candidates[self.roll_weighted_index(&weights)].vault_id;
            self.content.vault(vault_id).cloned()
        };
        let guardian = definition.guardian.as_ref().filter(|_| {
            definition.dungeon_id.as_ref().is_some_and(|dungeon_id| {
                self.dungeon_states
                    .get(dungeon_id)
                    .is_some_and(|state| !state.guardian_defeated)
            })
        });
        let task_objectives = self
            .content
            .world(&self.world_id)
            .and_then(|world| {
                world
                    .procedural_floors
                    .iter()
                    .find(|floor| {
                        floor_task_id(floor) == floor_task_id(definition)
                            && !floor.task_stages.is_empty()
                    })
                    .map(|floor| {
                        floor
                            .task_stages
                            .iter()
                            .filter(|stage| {
                                stage.floor_id.as_deref() == Some(definition.id.as_str())
                            })
                            .cloned()
                            .collect::<Vec<_>>()
                    })
            })
            .unwrap_or_else(|| definition.task_objective.iter().cloned().collect());
        let width = definition.width;
        let height = definition.height;
        let mut terrain =
            vec![definition.wall_terrain_id.clone(); usize::from(width) * usize::from(height)];
        let cavern_origin = definition.layout.as_ref().and_then(|layout| {
            layout.cavern.as_ref().map(|cavern| {
                self.generate_connected_cavern(definition, &cavern.terrain_id, &mut terrain)
            })
        });
        let lake_origin = definition.layout.as_ref().and_then(|layout| {
            layout.lake.as_ref().map(|lake| {
                self.generate_connected_lake(
                    definition,
                    &lake.deep_terrain_id,
                    &lake.shallow_terrain_id,
                    &mut terrain,
                )
            })
        });
        let maze_walkable = if maze_only {
            let maze = definition
                .layout
                .as_ref()
                .and_then(|layout| layout.maze.as_ref())
                .expect("validated maze-only layout must retain maze geometry");
            self.generate_maze(definition, maze, &generated_floor_terrain_id, &mut terrain)
        } else {
            BTreeSet::new()
        };
        let rooms = if maze_only {
            Vec::new()
        } else if let Some(layout) = &definition.layout {
            self.generate_budgeted_rooms(
                definition,
                layout
                    .rooms
                    .as_ref()
                    .expect("validated rooms layout must retain room geometry"),
            )
        } else {
            let room_width = 6_i32;
            let room_height = 5_i32;
            let first_x = 1 + i32::try_from(self.rng.bounded(3)).unwrap_or(0);
            let first_y = 1 + i32::try_from(self.rng.bounded(4)).unwrap_or(0);
            let second_x = 11 + i32::try_from(self.rng.bounded(3)).unwrap_or(0);
            let second_y = 11 + i32::try_from(self.rng.bounded(3)).unwrap_or(0);
            vec![
                GeneratedRoom {
                    id: "entry".to_owned(),
                    x: first_x,
                    y: first_y,
                    width: room_width,
                    height: room_height,
                    shape: ProceduralRoomShape::Rectangle,
                },
                GeneratedRoom {
                    id: "remote".to_owned(),
                    x: second_x,
                    y: second_y,
                    width: room_width,
                    height: room_height,
                    shape: ProceduralRoomShape::Rectangle,
                },
            ]
        };
        let content_rooms = if definition
            .layout
            .as_ref()
            .is_some_and(|layout| layout.pit.is_some())
        {
            &rooms[..rooms.len() - 1]
        } else {
            rooms.as_slice()
        };
        let room_region_indexes =
            assign_generated_rooms_to_regions(content_rooms, selected_region_entries.len());
        let mut generated_regions = selected_region_entries
            .iter()
            .enumerate()
            .map(|(region_index, entry)| {
                let theme = self
                    .content
                    .theme_table(&entry.theme_table_id)
                    .and_then(|table| {
                        table
                            .entries
                            .iter()
                            .find(|theme| theme.theme_id == entry.theme_id)
                    })
                    .expect("validated region theme must remain available");
                let room_ids = content_rooms
                    .iter()
                    .zip(&room_region_indexes)
                    .filter(|(_, assigned_region)| **assigned_region == region_index)
                    .map(|(room, _)| room.id.clone())
                    .collect::<Vec<_>>();
                let mut cells = content_rooms
                    .iter()
                    .zip(&room_region_indexes)
                    .filter(|(_, assigned_region)| **assigned_region == region_index)
                    .flat_map(|(room, _)| generated_room_cells(room))
                    .collect::<Vec<_>>();
                cells.sort();
                GeneratedRegion {
                    state: FloorRegionState {
                        region_id: entry.region_id.clone(),
                        theme_id: entry.theme_id.clone(),
                        encounter_table_id: entry.encounter_table_id.clone(),
                        loot_table_id: entry.loot_table_id.clone(),
                        cells,
                    },
                    room_ids,
                    floor_terrain_id: theme.floor_terrain_id.clone(),
                }
            })
            .collect::<Vec<_>>();
        for (room_index, room) in rooms.iter().enumerate() {
            let room_terrain_id = room_region_indexes
                .get(room_index)
                .and_then(|region_index| generated_regions.get(*region_index))
                .map_or(generated_floor_terrain_id.as_str(), |region| {
                    region.floor_terrain_id.as_str()
                });
            carve_generated_room(&mut terrain, width, room, room_terrain_id);
        }
        let (first_center, second_center) = if maze_only {
            maze_floor_anchors(&maze_walkable)
        } else {
            (rooms[0].center(), rooms[1].center())
        };
        let legacy_vault_origin = legacy_vault.as_ref().map(|vault| Position {
            x: second_center.x - i32::from(vault.entrance_positions[0].x),
            y: rooms
                .get(1)
                .expect("legacy vault placement requires a remote room")
                .y,
        });
        if let Some(destroyed) = definition
            .layout
            .as_ref()
            .and_then(|layout| layout.destroyed.as_ref())
        {
            self.generate_destroyed_region(definition, &destroyed.terrain_id, &mut terrain);
        }
        if let Some(river) = definition
            .layout
            .as_ref()
            .and_then(|layout| layout.river.as_ref())
        {
            self.generate_river(
                definition,
                &river.deep_terrain_id,
                &river.shallow_terrain_id,
                lake_origin.unwrap_or(Position {
                    x: i32::from(width / 2),
                    y: i32::from(height / 2),
                }),
                &mut terrain,
            );
        }
        if definition
            .layout
            .as_ref()
            .is_some_and(|layout| layout.destroyed.is_some() || layout.river.is_some())
        {
            for room in &rooms {
                let room_index = rooms
                    .iter()
                    .position(|candidate| candidate.id == room.id)
                    .expect("generated room must retain its stable index");
                let room_terrain_id = room_region_indexes
                    .get(room_index)
                    .and_then(|region_index| generated_regions.get(*region_index))
                    .map_or(generated_floor_terrain_id.as_str(), |region| {
                        region.floor_terrain_id.as_str()
                    });
                carve_generated_room(&mut terrain, width, room, room_terrain_id);
            }
        }
        for connected_rooms in rooms.windows(2) {
            carve_generated_corridor(
                &mut terrain,
                width,
                connected_rooms[0].center(),
                connected_rooms[1].center(),
                &generated_floor_terrain_id,
            );
        }
        if let Some(cavern_origin) = cavern_origin {
            carve_generated_corridor(
                &mut terrain,
                width,
                first_center,
                cavern_origin,
                &generated_floor_terrain_id,
            );
        }
        if let Some(layout) = &definition.layout
            && !layout.streamers.is_empty()
        {
            self.generate_streamers(definition, &layout.streamers, &mut terrain);
        }
        let pit_placement = definition
            .layout
            .as_ref()
            .and_then(|layout| layout.pit.as_ref())
            .map(|pit| {
                self.place_classic_pit(
                    definition,
                    pit,
                    rooms[rooms.len() - 2].center(),
                    &generated_floor_terrain_id,
                    &mut terrain,
                )
            });
        let door_position = (!maze_only).then_some(Position {
            x: (first_center.x + second_center.x) / 2,
            y: first_center.y,
        });
        if let Some(door_position) = door_position {
            set_generated_terrain(
                &mut terrain,
                width,
                door_position,
                &definition.closed_door_terrain_id,
            );
        }
        let down_stair_position = if maze_only {
            second_center
        } else {
            Position {
                x: first_center.x - 1,
                y: first_center.y,
            }
        };
        let fixed_trap_position = if maze_only {
            let route = maze_floor_path(&maze_walkable, first_center, second_center);
            route[route.len() / 2]
        } else {
            Position {
                x: first_center.x,
                y: first_center.y + 1,
            }
        };
        let mut floor_connections = if definition.connections.is_empty() {
            set_generated_terrain(
                &mut terrain,
                width,
                first_center,
                &definition.up_stair_terrain_id,
            );
            if let Some(down_stair_terrain_id) = &definition.down_stair_terrain_id {
                set_generated_terrain(
                    &mut terrain,
                    width,
                    down_stair_position,
                    down_stair_terrain_id,
                );
            }
            Vec::new()
        } else {
            let (primary_up_id, primary_down_id) = primary_floor_connection_ids(definition);
            for (connection_id, position) in [
                (primary_up_id, first_center),
                (primary_down_id, down_stair_position),
            ] {
                if let Some(connection) = connection_id.and_then(|connection_id| {
                    definition
                        .connections
                        .iter()
                        .find(|connection| connection.id == connection_id)
                }) {
                    set_generated_terrain(&mut terrain, width, position, &connection.terrain_id);
                }
            }
            Vec::new()
        };
        set_generated_terrain(
            &mut terrain,
            width,
            fixed_trap_position,
            &definition.trap_terrain_id,
        );
        let vault_placements = if let Some(vault) = legacy_vault.clone() {
            let placement = GeneratedVaultPlacement {
                vault,
                origin: legacy_vault_origin.expect("present vault must have an origin"),
                transform: VaultTransform::Identity,
                ordinal: 1,
                connector_cells: Vec::new(),
            };
            paint_generated_vault(&mut terrain, width, &placement);
            vec![placement]
        } else if uses_spatial_vault_budget {
            self.select_spatial_vault_placements(
                definition,
                &eligible_vault_candidates,
                guardian.is_some(),
                &generated_floor_terrain_id,
                &mut terrain,
            )
        } else {
            Vec::new()
        };
        for placement in &vault_placements {
            let entrance = transformed_vault_position(
                &placement.vault,
                placement.transform,
                placement.vault.entrance_positions[0],
            );
            let anchor = Position {
                x: placement.origin.x + entrance.x,
                y: placement.origin.y + entrance.y,
            };
            let (vault_width, vault_height) =
                transformed_vault_dimensions(&placement.vault, placement.transform);
            let footprint = (0..vault_height).flat_map(|y| {
                (0..vault_width).map(move |x| Position {
                    x: placement.origin.x + i32::from(x),
                    y: placement.origin.y + i32::from(y),
                })
            });
            assign_generated_footprint_to_region(
                &mut generated_regions,
                content_rooms,
                anchor,
                footprint,
            );
        }
        if let Some(pit) = &pit_placement {
            let total_width = pit.definition.inner_width + 6;
            let total_height = pit.definition.inner_height + 6;
            let footprint = (0..total_height).flat_map(|y| {
                (0..total_width).map(move |x| Position {
                    x: pit.origin.x + i32::from(x),
                    y: pit.origin.y + i32::from(y),
                })
            });
            assign_generated_footprint_to_region(
                &mut generated_regions,
                content_rooms,
                pit.outer_entrance,
                footprint,
            );
        }
        if !definition.connections.is_empty() {
            floor_connections = place_generated_floor_connections(
                definition,
                first_center,
                down_stair_position,
                fixed_trap_position,
                &generated_floor_terrain_id,
                &mut terrain,
                &mut self.rng,
            )?;
        }
        let mut feature_reserved = BTreeSet::from([fixed_trap_position]);
        if floor_connections.is_empty() {
            feature_reserved.insert(first_center);
        } else {
            feature_reserved.extend(
                floor_connections
                    .iter()
                    .map(|connection| connection.position),
            );
        }
        if let Some(door_position) = door_position {
            feature_reserved.insert(door_position);
        }
        if floor_connections.is_empty() && definition.down_stair_terrain_id.is_some() {
            feature_reserved.insert(down_stair_position);
        }
        for placement in &vault_placements {
            let (vault_width, vault_height) =
                transformed_vault_dimensions(&placement.vault, placement.transform);
            for y in 0..vault_height {
                for x in 0..vault_width {
                    feature_reserved.insert(Position {
                        x: placement.origin.x + i32::from(x),
                        y: placement.origin.y + i32::from(y),
                    });
                }
            }
            feature_reserved.extend(placement.connector_cells.iter().copied());
        }
        if let Some(pit) = &pit_placement {
            let total_width = pit.definition.inner_width + 6;
            let total_height = pit.definition.inner_height + 6;
            for y in 0..total_height {
                for x in 0..total_width {
                    feature_reserved.insert(Position {
                        x: pit.origin.x + i32::from(x),
                        y: pit.origin.y + i32::from(y),
                    });
                }
            }
            feature_reserved.insert(pit.outer_entrance);
            feature_reserved.insert(pit.inner_entrance);
        }
        let room_floor_terrain_ids = generated_regions
            .iter()
            .map(|region| region.floor_terrain_id.clone())
            .collect::<BTreeSet<_>>();
        let terrain_features = if let Some(table_id) = &definition.terrain_feature_table_id {
            let table = self
                .content
                .terrain_feature_table(table_id)
                .expect("validated terrain feature table must remain available")
                .clone();
            let eligible_entries = table
                .entries
                .iter()
                .filter(|entry| {
                    entry.min_depth <= definition.depth && definition.depth <= entry.max_depth
                })
                .cloned()
                .collect::<Vec<_>>();
            self.place_terrain_features(
                definition,
                &eligible_entries,
                TerrainFeaturePlacementContext {
                    rooms: content_rooms,
                    reserved: &feature_reserved,
                    floor_terrain_id: &generated_floor_terrain_id,
                    room_floor_terrain_ids: &room_floor_terrain_ids,
                },
                &mut terrain,
            )
        } else {
            Vec::new()
        };
        let mut occupied = BTreeSet::from([first_center]);
        occupied.extend(
            floor_connections
                .iter()
                .map(|connection| connection.position),
        );
        if maze_only {
            occupied.insert(fixed_trap_position);
        }
        occupied.extend(terrain_features.iter().map(|feature| feature.position));
        if let Some(pit) = &pit_placement {
            let total_width = pit.definition.inner_width + 6;
            let total_height = pit.definition.inner_height + 6;
            for y in 0..total_height {
                for x in 0..total_width {
                    occupied.insert(Position {
                        x: pit.origin.x + i32::from(x),
                        y: pit.origin.y + i32::from(y),
                    });
                }
            }
        }
        for placement in &vault_placements {
            occupied.extend(
                placement
                    .vault
                    .encounter_groups
                    .iter()
                    .flat_map(|group| &group.member_positions)
                    .map(|local| {
                        let local = transformed_vault_position(
                            &placement.vault,
                            placement.transform,
                            *local,
                        );
                        Position {
                            x: placement.origin.x + local.x,
                            y: placement.origin.y + local.y,
                        }
                    }),
            );
            occupied.extend(placement.vault.loot_spawns.iter().map(|spawn| {
                let local = transformed_vault_position(
                    &placement.vault,
                    placement.transform,
                    spawn.position,
                );
                Position {
                    x: placement.origin.x + local.x,
                    y: placement.origin.y + local.y,
                }
            }));
        }
        if floor_connections.is_empty() && definition.down_stair_terrain_id.is_some() {
            occupied.insert(down_stair_position);
        }
        let guardian_position = guardian.map(|_| Position {
            x: first_center.x + 1,
            y: first_center.y,
        });
        occupied.extend(guardian_position);
        let reserved_actor_slots = definition
            .generation_budget
            .as_ref()
            .and_then(|budget| budget.pit_actor_slots)
            .unwrap_or(0)
            .saturating_add(definition.nest.as_ref().map_or(0, |nest| nest.spawn_count))
            .saturating_add(if guardian.is_some() { 1 } else { 0 })
            .saturating_add(
                vault_placements
                    .iter()
                    .flat_map(|placement| &placement.vault.encounter_groups)
                    .map(|group| {
                        u16::try_from(group.member_positions.len())
                            .expect("validated vault group size must fit u16")
                    })
                    .sum::<u16>(),
            );
        let mut entities = Vec::new();
        let mut regional_loot_allocations = Vec::new();
        if !generated_regions.is_empty() {
            let budget = definition
                .generation_budget
                .as_ref()
                .expect("validated region floor must retain a generation budget");
            let region_count = u16::try_from(generated_regions.len())
                .expect("validated region count must fit u16");
            if budget.group_placements.is_some() && budget.group_actor_slots.is_some() {
                let host = &generated_regions[0];
                let table = self
                    .content
                    .encounter_table(&host.state.encounter_table_id)
                    .expect("validated regional group table must remain available")
                    .clone();
                let eligible_entries = table
                    .entries
                    .iter()
                    .filter(|entry| {
                        entry.min_depth <= definition.depth
                            && definition.depth <= entry.max_depth
                            && self
                                .content
                                .actor(&entry.actor_kind_id)
                                .is_some_and(|actor| actor.level <= u32::from(definition.depth))
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let room_id = &host.room_ids[0];
                let id_prefix = format!("{}.region.{}", definition.id, host.state.region_id);
                entities.extend(self.generate_dynamic_encounter_groups(
                    definition,
                    &table,
                    &eligible_entries,
                    content_rooms,
                    room_id,
                    reserved_actor_slots,
                    region_count,
                    false,
                    &id_prefix,
                    &mut occupied,
                ));
            }
            let actor_budget = budget
                .actor_slots
                .saturating_sub(reserved_actor_slots)
                .saturating_sub(
                    u16::try_from(entities.len())
                        .expect("generated regional group size must fit u16"),
                );
            let loot_budget = budget.loot_placements.saturating_sub(
                vault_placements
                    .iter()
                    .map(|placement| {
                        u16::try_from(placement.vault.loot_spawns.len())
                            .expect("validated vault loot count must fit u16")
                    })
                    .sum::<u16>(),
            );
            let (regional_actor_allocations, loot_allocations) =
                allocate_generated_region_placements(
                    &generated_regions,
                    &terrain,
                    width,
                    &self.content,
                    &occupied,
                    actor_budget,
                    loot_budget,
                );
            regional_loot_allocations = loot_allocations;
            for (region_index, region) in generated_regions.iter().enumerate() {
                let placements = regional_actor_allocations[region_index];
                let table = self
                    .content
                    .encounter_table(&region.state.encounter_table_id)
                    .expect("validated region encounter table must remain available")
                    .clone();
                let eligible_entries = table
                    .entries
                    .iter()
                    .filter(|entry| {
                        entry.group.is_none()
                            && entry.min_depth <= definition.depth
                            && definition.depth <= entry.max_depth
                            && self
                                .content
                                .actor(&entry.actor_kind_id)
                                .is_some_and(|actor| actor.level <= u32::from(definition.depth))
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let weights = eligible_entries
                    .iter()
                    .map(|entry| entry.weight)
                    .collect::<Vec<_>>();
                for ordinal in 0..placements {
                    let entry = &eligible_entries[self.roll_weighted_index(&weights)];
                    let position =
                        self.choose_generated_region_position(region, &terrain, width, &occupied);
                    occupied.insert(position);
                    entities.push(self.generated_actor(
                        format!(
                            "{}.region.{}.encounter.plain.{}",
                            definition.id,
                            region.state.region_id,
                            ordinal + 1
                        ),
                        &entry.actor_kind_id,
                        position,
                    ));
                }
            }
        } else if let Some(table_id) = &definition.encounter_table_id {
            let table = self
                .content
                .encounter_table(table_id)
                .expect("validated floor encounter table must remain available")
                .clone();
            let eligible_entries = table
                .entries
                .iter()
                .filter(|entry| {
                    entry.min_depth <= definition.depth
                        && definition.depth <= entry.max_depth
                        && self
                            .content
                            .actor(&entry.actor_kind_id)
                            .is_some_and(|actor| actor.level <= u32::from(definition.depth))
                })
                .cloned()
                .collect::<Vec<_>>();
            let weights = eligible_entries
                .iter()
                .map(|entry| entry.weight)
                .collect::<Vec<_>>();
            let room_id = if legacy_vault.is_some() {
                "entry"
            } else {
                "remote"
            };
            if definition.generation_budget.as_ref().is_some_and(|budget| {
                budget.group_placements.is_some() && budget.group_actor_slots.is_some()
            }) {
                entities.extend(self.generate_dynamic_encounter_groups(
                    definition,
                    &table,
                    &eligible_entries,
                    content_rooms,
                    room_id,
                    reserved_actor_slots,
                    1,
                    true,
                    &definition.id,
                    &mut occupied,
                ));
            } else {
                let encounter_rolls =
                    definition
                        .generation_budget
                        .as_ref()
                        .map_or(table.rolls, |budget| {
                            table
                                .rolls
                                .min(budget.actor_slots.saturating_sub(reserved_actor_slots))
                        });
                for ordinal in 0..encounter_rolls {
                    let entry = &eligible_entries[self.roll_weighted_index(&weights)];
                    let placement_room_id = if maze_only {
                        "maze"
                    } else if definition.layout.is_some() {
                        generated_non_entry_room_id(content_rooms, ordinal)
                    } else {
                        room_id
                    };
                    let position = if maze_only {
                        choose_generated_maze_position(&maze_walkable, first_center, &occupied)
                    } else {
                        self.choose_generated_room_position(
                            content_rooms,
                            placement_room_id,
                            &occupied,
                        )
                    };
                    occupied.insert(position);
                    entities.push(self.generated_actor(
                        format!("{}.encounter.{}", definition.id, ordinal + 1),
                        &entry.actor_kind_id,
                        position,
                    ));
                }
            }
            if let Some(nest) = &definition.nest {
                let entry = &eligible_entries[self.roll_weighted_index(&weights)];
                for ordinal in 0..nest.spawn_count {
                    let position =
                        self.choose_generated_room_position(&rooms, &nest.room_id, &occupied);
                    occupied.insert(position);
                    let actor = self
                        .content
                        .actor(&entry.actor_kind_id)
                        .expect("validated nest actor must remain available");
                    entities.push(stamped_spawn(
                        actor_from_spawn(
                            &format!("{}.nest.{}", definition.id, ordinal + 1),
                            &entry.actor_kind_id,
                            ContentPosition {
                                x: u16::try_from(position.x).expect("nest actor x must fit u16"),
                                y: u16::try_from(position.y).expect("nest actor y must fit u16"),
                            },
                            actor.max_hp,
                            actor.speed,
                            INITIAL_MONSTER_ENERGY_NEED,
                            actor_starts_alerted(actor),
                        ),
                        actor,
                    ));
                }
            }
        } else {
            for spawn in &definition.actor_spawns {
                let eligible_kind_ids = spawn
                    .actor_kind_ids
                    .iter()
                    .filter(|kind_id| {
                        self.content
                            .actor(kind_id)
                            .is_some_and(|actor| actor.level <= u32::from(definition.depth))
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let kind_index = usize::try_from(
                    self.rng.bounded(
                        u64::try_from(eligible_kind_ids.len())
                            .expect("validated actor candidate count must fit u64"),
                    ),
                )
                .expect("bounded actor candidate index must fit usize");
                let kind_id = &eligible_kind_ids[kind_index];
                let position =
                    self.choose_generated_room_position(&rooms, &spawn.room_id, &occupied);
                occupied.insert(position);
                let actor = self
                    .content
                    .actor(kind_id)
                    .expect("validated procedural actor kind must remain available");
                entities.push(stamped_spawn(
                    actor_from_spawn(
                        &spawn.instance_id,
                        kind_id,
                        ContentPosition {
                            x: u16::try_from(position.x).expect("generated actor x must fit u16"),
                            y: u16::try_from(position.y).expect("generated actor y must fit u16"),
                        },
                        actor.max_hp,
                        actor.speed,
                        INITIAL_MONSTER_ENERGY_NEED,
                        actor_starts_alerted(actor),
                    ),
                    actor,
                ));
            }
        }
        if let Some(pit) = &pit_placement {
            entities.extend(self.generate_classic_pit_actors(definition, pit, &mut occupied));
        }
        for placement in &vault_placements {
            for group in &placement.vault.encounter_groups {
                let eligible_entries = group
                    .entries
                    .iter()
                    .filter(|entry| {
                        entry.min_depth <= definition.depth
                            && definition.depth <= entry.max_depth
                            && self
                                .content
                                .actor(&entry.actor_kind_id)
                                .is_some_and(|actor| actor.level <= u32::from(definition.depth))
                    })
                    .collect::<Vec<_>>();
                let weights = eligible_entries
                    .iter()
                    .map(|entry| entry.weight)
                    .collect::<Vec<_>>();
                for (ordinal, local) in group.member_positions.iter().enumerate() {
                    let entry = eligible_entries[self.roll_weighted_index(&weights)];
                    let actor = self
                        .content
                        .actor(&entry.actor_kind_id)
                        .expect("validated vault encounter actor must remain available");
                    let local =
                        transformed_vault_position(&placement.vault, placement.transform, *local);
                    let position = Position {
                        x: placement.origin.x + local.x,
                        y: placement.origin.y + local.y,
                    };
                    occupied.insert(position);
                    let instance_id = if uses_spatial_vault_budget {
                        format!(
                            "{}.vault.{}.{}.{}",
                            definition.id,
                            placement.ordinal,
                            group.id,
                            ordinal + 1
                        )
                    } else {
                        format!("{}.{}.{}", definition.id, group.id, ordinal + 1)
                    };
                    entities.push(stamped_spawn(
                        actor_from_spawn(
                            &instance_id,
                            &entry.actor_kind_id,
                            ContentPosition {
                                x: u16::try_from(position.x).expect("vault actor x must fit u16"),
                                y: u16::try_from(position.y).expect("vault actor y must fit u16"),
                            },
                            actor.max_hp,
                            actor.speed,
                            INITIAL_MONSTER_ENERGY_NEED,
                            actor_starts_alerted(actor),
                        ),
                        actor,
                    ));
                }
            }
        }
        if let Some(guardian) = guardian {
            let actor = self
                .content
                .actor(&guardian.actor_kind_id)
                .expect("validated dungeon guardian must remain available");
            let max_hp = actor.max_hp;
            let speed = actor.speed;
            let position = guardian_position.expect("present guardian must retain a position");
            entities.push(stamped_spawn(
                actor_from_spawn(
                    &guardian.instance_id,
                    &guardian.actor_kind_id,
                    ContentPosition {
                        x: u16::try_from(position.x).expect("guardian x must fit u16"),
                        y: u16::try_from(position.y).expect("guardian y must fit u16"),
                    },
                    max_hp,
                    speed,
                    INITIAL_MONSTER_ENERGY_NEED,
                    actor_starts_alerted(actor),
                ),
                actor,
            ));
        }
        let mut items =
            self.generate_carried_loot_for_actors(&entities, &definition.id, definition.depth)?;
        if !generated_regions.is_empty() {
            for (region_index, region) in generated_regions.iter().enumerate() {
                let placements = regional_loot_allocations[region_index];
                for ordinal in 0..placements {
                    let room_id = &region.room_ids[usize::from(ordinal) % region.room_ids.len()];
                    let position =
                        self.choose_generated_region_position(region, &terrain, width, &occupied);
                    occupied.insert(position);
                    items.extend(self.generate_loot_instances(
                        &LootContext {
                            table_id: region.state.loot_table_id.clone(),
                            floor_id: definition.id.clone(),
                            depth: definition.depth,
                            source: LootSource::FloorRoom {
                                room_id: room_id.clone(),
                                spawn_id: format!(
                                    "{}.region.{}.loot.{}",
                                    definition.id,
                                    region.state.region_id,
                                    ordinal + 1
                                ),
                            },
                        },
                        ItemLocation::Ground(position),
                    )?);
                }
            }
        } else if let Some(table_id) = &definition.loot_table_id {
            let room_id = if legacy_vault.is_some() {
                "entry"
            } else {
                "remote"
            };
            let floor_loot_placements = definition.generation_budget.as_ref().map_or(1, |budget| {
                budget.loot_placements.saturating_sub(
                    vault_placements
                        .iter()
                        .map(|placement| {
                            u16::try_from(placement.vault.loot_spawns.len())
                                .expect("validated vault loot count must fit u16")
                        })
                        .sum::<u16>(),
                )
            });
            for ordinal in 0..floor_loot_placements {
                let placement_room_id = if maze_only {
                    "maze"
                } else if definition.layout.is_some() {
                    generated_non_entry_room_id(content_rooms, ordinal)
                } else {
                    room_id
                };
                let position = if maze_only {
                    choose_generated_maze_position(&maze_walkable, first_center, &occupied)
                } else {
                    self.choose_generated_room_position(content_rooms, placement_room_id, &occupied)
                };
                occupied.insert(position);
                items.extend(self.generate_loot_instances(
                    &LootContext {
                        table_id: table_id.clone(),
                        floor_id: definition.id.clone(),
                        depth: definition.depth,
                        source: LootSource::FloorRoom {
                            room_id: placement_room_id.to_owned(),
                            spawn_id: format!("{}.loot-table.{}", definition.id, ordinal + 1),
                        },
                    },
                    ItemLocation::Ground(position),
                )?);
            }
        } else {
            for spawn in &definition.loot_spawns {
                let position =
                    self.choose_generated_room_position(&rooms, &spawn.room_id, &occupied);
                occupied.insert(position);
                items.extend(self.generate_loot_instances(
                    &LootContext {
                        table_id: spawn.loot_table_id.clone(),
                        floor_id: definition.id.clone(),
                        depth: definition.depth,
                        source: LootSource::FloorRoom {
                            room_id: spawn.room_id.clone(),
                            spawn_id: spawn.id.clone(),
                        },
                    },
                    ItemLocation::Ground(position),
                )?);
            }
        }
        for placement in &vault_placements {
            for spawn in &placement.vault.loot_spawns {
                let local = transformed_vault_position(
                    &placement.vault,
                    placement.transform,
                    spawn.position,
                );
                let position = Position {
                    x: placement.origin.x + local.x,
                    y: placement.origin.y + local.y,
                };
                occupied.insert(position);
                items.extend(self.generate_loot_instances(
                    &LootContext {
                        table_id: spawn.loot_table_id.clone(),
                        floor_id: definition.id.clone(),
                        depth: definition.depth,
                        source: LootSource::Vault {
                            vault_id: placement.vault.id.clone(),
                            spawn_id: spawn.id.clone(),
                        },
                    },
                    ItemLocation::Ground(position),
                )?);
            }
        }
        for objective in &task_objectives {
            match objective.kind {
                TaskObjectiveKind::CollectItem => {
                    let kind_id = objective
                        .item_kind_id
                        .clone()
                        .expect("validated item objective must have a kind ID");
                    let (activation, charges) = initial_item_runtime_state(
                        &self.content,
                        &mut self.rng,
                        &kind_id,
                        definition.depth,
                    );
                    items.push(ItemInstance {
                        id: objective
                            .item_instance_id
                            .clone()
                            .expect("validated item objective must have an instance ID"),
                        curse: initial_item_curse(&self.content, &kind_id),
                        kind_id,
                        quantity: 1,
                        quality: ItemQualityDto::Ordinary,
                        affix_ids: Vec::new(),
                        rolled_affixes: Vec::new(),
                        enchantments: ItemEnchantmentsDto::default(),
                        activation,
                        charges,
                        device_recovery_progress: 0,
                        location: ItemLocation::Ground(first_center),
                    });
                }
                TaskObjectiveKind::KillActor => {
                    let kind_id = objective
                        .actor_kind_id
                        .as_ref()
                        .expect("validated kill objective must have a kind ID");
                    let actor = self
                        .content
                        .actor(kind_id)
                        .expect("validated objective actor must remain available");
                    entities.push(stamped_spawn(
                        actor_from_spawn(
                            objective
                                .actor_instance_id
                                .as_ref()
                                .expect("validated kill objective must have an instance ID"),
                            kind_id,
                            ContentPosition {
                                x: u16::try_from(first_center.x + 1)
                                    .expect("objective x must fit u16"),
                                y: u16::try_from(first_center.y).expect("objective y must fit u16"),
                            },
                            actor.max_hp,
                            actor.speed,
                            INITIAL_MONSTER_ENERGY_NEED,
                            actor_starts_alerted(actor),
                        ),
                        actor,
                    ));
                }
                TaskObjectiveKind::KillActorKind => {
                    let kind_id = objective
                        .actor_kind_id
                        .as_ref()
                        .expect("validated counted kill objective must have a kind ID");
                    let actor = self
                        .content
                        .actor(kind_id)
                        .expect("validated objective actor must remain available");
                    let remaining = self
                        .task_states
                        .get(floor_task_id(definition))
                        .map_or(objective.required, |state| {
                            state.required.saturating_sub(state.current)
                        });
                    let spawn_count = objective
                        .spawn_count
                        .unwrap_or(objective.required)
                        .min(remaining);
                    for ordinal in 0..spawn_count {
                        entities.push(stamped_spawn(
                            actor_from_spawn(
                                &format!("{}.task-target.{}", definition.id, ordinal + 1),
                                kind_id,
                                ContentPosition {
                                    x: u16::try_from(
                                        first_center.x
                                            + 1
                                            + i32::try_from(ordinal).unwrap_or(i32::MAX),
                                    )
                                    .expect("objective x must fit u16"),
                                    y: u16::try_from(first_center.y)
                                        .expect("objective y must fit u16"),
                                },
                                actor.max_hp,
                                actor.speed,
                                INITIAL_MONSTER_ENERGY_NEED,
                                actor_starts_alerted(actor),
                            ),
                            actor,
                        ));
                    }
                }
                TaskObjectiveKind::EnterFloor => {}
            }
        }
        for region in &mut generated_regions {
            region.state.cells.sort();
            region.state.cells.dedup();
        }
        generated_regions.sort_by(|left, right| left.state.region_id.cmp(&right.state.region_id));
        self.resolve_floor_connection_targets(definition, &mut floor_connections)?;
        Ok(FloorState {
            id: definition.id.clone(),
            dungeon_instance_id,
            width,
            height,
            terrain,
            player_position: first_center,
            entities,
            items,
            explored: vec![false; usize::from(width) * usize::from(height)],
            revealed_terrain: BTreeSet::new(),
            connections: floor_connections,
            regions: generated_regions
                .into_iter()
                .map(|region| region.state)
                .collect(),
        })
    }

    fn resolve_floor_connection_targets(
        &mut self,
        definition: &ProceduralFloorDefinition,
        connections: &mut [FloorConnectionState],
    ) -> Result<(), CoreError> {
        let mut selected_dynamic_targets = BTreeSet::new();
        for state in connections {
            let connection = definition
                .connections
                .iter()
                .find(|connection| connection.id == state.id)
                .ok_or(CoreError::InvalidSave(
                    "generated floor connection is missing from content",
                ))?;
            if connection.target_candidates.is_empty() {
                state.target_floor_id = Some(connection.target_floor_id.clone());
                state.target_connection_id = connection.target_connection_id.clone();
                continue;
            }
            let mut eligible = connection
                .target_candidates
                .iter()
                .filter(|candidate| !selected_dynamic_targets.contains(&candidate.target_floor_id))
                .collect::<Vec<_>>();
            if eligible.is_empty() {
                eligible.extend(connection.target_candidates.iter());
            }
            let weights = eligible
                .iter()
                .map(|candidate| u32::from(candidate.weight))
                .collect::<Vec<_>>();
            let selected = eligible[self.roll_weighted_index(&weights)];
            state.target_floor_id = Some(selected.target_floor_id.clone());
            state.target_connection_id = Some(selected.target_connection_id.clone());
            selected_dynamic_targets.insert(selected.target_floor_id.clone());
        }
        Ok(())
    }

    fn generate_budgeted_rooms(
        &mut self,
        definition: &ProceduralFloorDefinition,
        geometry: &ProceduralRoomGeometryDefinition,
    ) -> Vec<GeneratedRoom> {
        let budget = definition
            .generation_budget
            .as_ref()
            .expect("room geometry requires a generation budget");
        let placement_count = budget
            .room_placements
            .expect("validated room placement count must remain available");
        let mut remaining_area = budget
            .room_area_tiles
            .expect("validated room area budget must remain available");
        let columns = if placement_count <= 4 { 2 } else { 3 };
        let rows = placement_count.div_ceil(columns);
        let interior_width = definition.width - 2;
        let interior_height = definition.height - 2;
        let minimum_room_area = geometry
            .shapes
            .iter()
            .map(|candidate| match candidate.shape {
                ProceduralRoomShape::Rectangle => {
                    u32::from(geometry.min_width) * u32::from(geometry.min_height)
                }
                ProceduralRoomShape::Cross => {
                    u32::from(geometry.min_width) + u32::from(geometry.min_height) - 1
                }
            })
            .min()
            .expect("validated room geometry must retain a shape");
        let mut rooms = Vec::with_capacity(usize::from(placement_count));

        for ordinal in 0..placement_count {
            let column = ordinal % columns;
            let row = ordinal / columns;
            let cell_left = 1 + interior_width * column / columns;
            let cell_right = 1 + interior_width * (column + 1) / columns;
            let cell_top = 1 + interior_height * row / rows;
            let cell_bottom = 1 + interior_height * (row + 1) / rows;
            let future_room_count = placement_count - ordinal - 1;
            let maximum_room_area =
                remaining_area - u32::from(future_room_count) * minimum_room_area;
            let mut shape_candidates = Vec::new();

            for shape_candidate in &geometry.shapes {
                let mut candidates = Vec::new();
                for y in cell_top..cell_bottom {
                    for x in cell_left..cell_right {
                        for height in geometry.min_height..=geometry.max_height {
                            for width in geometry.min_width..=geometry.max_width {
                                if x + width > cell_right || y + height > cell_bottom {
                                    continue;
                                }
                                let room = GeneratedRoom {
                                    id: String::new(),
                                    x: i32::from(x),
                                    y: i32::from(y),
                                    width: i32::from(width),
                                    height: i32::from(height),
                                    shape: shape_candidate.shape,
                                };
                                if room.area() <= maximum_room_area {
                                    candidates.push(room);
                                }
                            }
                        }
                    }
                }
                if !candidates.is_empty() {
                    shape_candidates.push((shape_candidate.weight, candidates));
                }
            }
            let shape_index = if shape_candidates.len() == 1 {
                0
            } else {
                let weights = shape_candidates
                    .iter()
                    .map(|(weight, _)| *weight)
                    .collect::<Vec<_>>();
                self.roll_weighted_index(&weights)
            };
            let candidates = &shape_candidates[shape_index].1;
            let candidate_index = if candidates.len() == 1 {
                0
            } else {
                usize::try_from(
                    self.rng.bounded(
                        u64::try_from(candidates.len())
                            .expect("room geometry candidate count must fit u64"),
                    ),
                )
                .expect("room geometry candidate index must fit usize")
            };
            let mut room = candidates[candidate_index].clone();
            room.id = match ordinal {
                0 => "entry".to_owned(),
                1 => "remote".to_owned(),
                _ => format!("room.{}", ordinal + 1),
            };
            remaining_area -= room.area();
            rooms.push(room);
        }

        rooms
    }

    fn place_classic_pit(
        &mut self,
        floor: &ProceduralFloorDefinition,
        pit: &ProceduralPitDefinition,
        approach: Position,
        floor_terrain_id: &str,
        terrain: &mut [String],
    ) -> GeneratedPitPlacement {
        let placement_count = floor
            .generation_budget
            .as_ref()
            .and_then(|budget| budget.room_placements)
            .expect("validated pit requires room placement budget");
        let columns = if placement_count <= 4 { 2 } else { 3 };
        let rows = placement_count.div_ceil(columns);
        let ordinal = placement_count - 1;
        let column = ordinal % columns;
        let row = ordinal / columns;
        let interior_width = floor.width - 2;
        let interior_height = floor.height - 2;
        let cell_left = 1 + interior_width * column / columns;
        let cell_right = 1 + interior_width * (column + 1) / columns;
        let cell_top = 1 + interior_height * row / rows;
        let cell_bottom = 1 + interior_height * (row + 1) / rows;
        let total_width = pit.inner_width + 6;
        let total_height = pit.inner_height + 6;
        let maximum_x = i32::from(floor.width - total_width - 1);
        let maximum_y = i32::from(floor.height - total_height - 1);
        let origin = Position {
            x: ((i32::from(cell_left + cell_right) - i32::from(total_width)) / 2)
                .clamp(1, maximum_x),
            y: ((i32::from(cell_top + cell_bottom) - i32::from(total_height)) / 2)
                .clamp(1, maximum_y),
        };
        let center_y = origin.y + i32::from(total_height / 2);
        let outer_entrance = Position {
            x: origin.x,
            y: center_y,
        };
        let inner_entrance = Position {
            x: origin.x + 2,
            y: center_y,
        };

        for local_y in 0..total_height {
            for local_x in 0..total_width {
                let on_outer_wall = local_x == 0
                    || local_y == 0
                    || local_x + 1 == total_width
                    || local_y + 1 == total_height;
                let on_inner_wall = local_x == 2
                    || local_y == 2
                    || local_x + 3 == total_width
                    || local_y + 3 == total_height;
                let terrain_id = if on_outer_wall || on_inner_wall {
                    &floor.wall_terrain_id
                } else {
                    floor_terrain_id
                };
                set_generated_terrain(
                    terrain,
                    floor.width,
                    Position {
                        x: origin.x + i32::from(local_x),
                        y: origin.y + i32::from(local_y),
                    },
                    terrain_id,
                );
            }
        }
        set_generated_terrain(terrain, floor.width, outer_entrance, floor_terrain_id);
        carve_generated_corridor(
            terrain,
            floor.width,
            approach,
            outer_entrance,
            floor_terrain_id,
        );
        set_generated_terrain(
            terrain,
            floor.width,
            inner_entrance,
            &floor.closed_door_terrain_id,
        );
        GeneratedPitPlacement {
            definition: pit.clone(),
            origin,
            outer_entrance,
            inner_entrance,
        }
    }

    fn generate_connected_cavern(
        &mut self,
        definition: &ProceduralFloorDefinition,
        terrain_id: &str,
        terrain: &mut [String],
    ) -> Position {
        const CARDINAL_OFFSETS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
        let area = definition
            .generation_budget
            .as_ref()
            .and_then(|budget| budget.cavern_area_tiles)
            .expect("validated cavern area budget must remain available");
        let origin = Position {
            x: i32::from(definition.width / 2),
            y: i32::from(definition.height / 2),
        };
        let mut carved = BTreeSet::from([origin]);
        set_generated_terrain(terrain, definition.width, origin, terrain_id);

        while carved.len() < usize::try_from(area).expect("cavern area must fit usize") {
            let mut frontier = carved
                .iter()
                .flat_map(|position| {
                    CARDINAL_OFFSETS.map(|(dx, dy)| Position {
                        x: position.x + dx,
                        y: position.y + dy,
                    })
                })
                .filter(|position| {
                    position.x > 0
                        && position.y > 0
                        && position.x + 1 < i32::from(definition.width)
                        && position.y + 1 < i32::from(definition.height)
                        && !carved.contains(position)
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            frontier.sort_by_key(|position| (position.y, position.x));
            let index = if frontier.len() == 1 {
                0
            } else {
                usize::try_from(self.rng.bounded(
                    u64::try_from(frontier.len()).expect("cavern frontier count must fit u64"),
                ))
                .expect("cavern frontier index must fit usize")
            };
            let position = frontier[index];
            carved.insert(position);
            set_generated_terrain(terrain, definition.width, position, terrain_id);
        }

        origin
    }

    fn generate_connected_lake(
        &mut self,
        definition: &ProceduralFloorDefinition,
        deep_terrain_id: &str,
        shallow_terrain_id: &str,
        terrain: &mut [String],
    ) -> Position {
        const CARDINAL_OFFSETS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
        let budget = definition
            .generation_budget
            .as_ref()
            .expect("lake requires a generation budget");
        let area = usize::try_from(
            budget
                .lake_area_tiles
                .expect("validated lake area budget must remain available"),
        )
        .expect("lake area must fit usize");
        let deep_area = usize::try_from(
            budget
                .lake_deep_area_tiles
                .expect("validated deep lake area budget must remain available"),
        )
        .expect("deep lake area must fit usize");
        let origin = Position {
            x: i32::from(definition.width / 2),
            y: i32::from(definition.height / 2),
        };
        let mut selected = BTreeSet::from([origin]);
        let mut insertion_order = vec![origin];

        while insertion_order.len() < area {
            let mut frontier = selected
                .iter()
                .flat_map(|position| {
                    CARDINAL_OFFSETS.map(|(dx, dy)| Position {
                        x: position.x + dx,
                        y: position.y + dy,
                    })
                })
                .filter(|position| {
                    position.x > 0
                        && position.y > 0
                        && position.x + 1 < i32::from(definition.width)
                        && position.y + 1 < i32::from(definition.height)
                        && !selected.contains(position)
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            frontier.sort_by_key(|position| (position.y, position.x));
            let index = if frontier.len() == 1 {
                0
            } else {
                usize::try_from(self.rng.bounded(
                    u64::try_from(frontier.len()).expect("lake frontier count must fit u64"),
                ))
                .expect("lake frontier index must fit usize")
            };
            let position = frontier[index];
            selected.insert(position);
            insertion_order.push(position);
        }

        for (ordinal, position) in insertion_order.into_iter().enumerate() {
            let terrain_id = if ordinal < deep_area {
                deep_terrain_id
            } else {
                shallow_terrain_id
            };
            set_generated_terrain(terrain, definition.width, position, terrain_id);
        }
        origin
    }

    fn generate_river(
        &mut self,
        definition: &ProceduralFloorDefinition,
        deep_terrain_id: &str,
        shallow_terrain_id: &str,
        target: Position,
        terrain: &mut [String],
    ) {
        const CARDINAL_OFFSETS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
        let area = usize::try_from(
            definition
                .generation_budget
                .as_ref()
                .and_then(|budget| budget.river_area_tiles)
                .expect("validated river area budget must remain available"),
        )
        .expect("river area must fit usize");
        let side = self.rng.bounded(4);
        let start = match side {
            0 => Position {
                x: 1 + i32::try_from(self.rng.bounded(u64::from(definition.width - 2)))
                    .expect("river start x must fit i32"),
                y: 1,
            },
            1 => Position {
                x: i32::from(definition.width - 2),
                y: 1 + i32::try_from(self.rng.bounded(u64::from(definition.height - 2)))
                    .expect("river start y must fit i32"),
            },
            2 => Position {
                x: 1 + i32::try_from(self.rng.bounded(u64::from(definition.width - 2)))
                    .expect("river start x must fit i32"),
                y: i32::from(definition.height - 2),
            },
            _ => Position {
                x: 1,
                y: 1 + i32::try_from(self.rng.bounded(u64::from(definition.height - 2)))
                    .expect("river start y must fit i32"),
            },
        };
        let mut current = start;
        let mut centerline = vec![current];
        while current != target {
            let move_x = current.x != target.x;
            let move_y = current.y != target.y;
            let advance_x = move_x && (!move_y || self.rng.bounded(2) == 0);
            if advance_x {
                current.x += (target.x - current.x).signum();
            } else {
                current.y += (target.y - current.y).signum();
            }
            centerline.push(current);
        }
        debug_assert!(centerline.len() <= area);
        let mut painted = centerline.iter().copied().collect::<BTreeSet<_>>();
        for position in &centerline {
            set_generated_terrain(terrain, definition.width, *position, deep_terrain_id);
        }

        while painted.len() < area {
            let mut frontier = painted
                .iter()
                .flat_map(|position| {
                    CARDINAL_OFFSETS.map(|(dx, dy)| Position {
                        x: position.x + dx,
                        y: position.y + dy,
                    })
                })
                .filter(|position| {
                    position.x > 0
                        && position.y > 0
                        && position.x + 1 < i32::from(definition.width)
                        && position.y + 1 < i32::from(definition.height)
                        && !painted.contains(position)
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            frontier.sort_by_key(|position| (position.y, position.x));
            let index = if frontier.len() == 1 {
                0
            } else {
                usize::try_from(self.rng.bounded(
                    u64::try_from(frontier.len()).expect("river frontier count must fit u64"),
                ))
                .expect("river frontier index must fit usize")
            };
            let position = frontier[index];
            painted.insert(position);
            set_generated_terrain(terrain, definition.width, position, shallow_terrain_id);
        }
    }

    fn generated_actor(&self, id: String, kind_id: &str, position: Position) -> Actor {
        let actor = self
            .content
            .actor(kind_id)
            .expect("validated generated actor must remain available");
        actor_from_spawn(
            &id,
            kind_id,
            ContentPosition {
                x: u16::try_from(position.x).expect("generated actor x must fit u16"),
                y: u16::try_from(position.y).expect("generated actor y must fit u16"),
            },
            actor.max_hp,
            actor.speed,
            INITIAL_MONSTER_ENERGY_NEED,
            actor_starts_alerted(actor),
        )
    }

    fn generated_pack_actor(
        &self,
        id: String,
        kind_id: &str,
        position: Position,
        pack: MonsterPackIdentity,
    ) -> Actor {
        let mut actor = self.generated_actor(id, kind_id, position);
        actor.pack = Some(pack);
        actor
    }

    fn generate_classic_pit_actors(
        &mut self,
        definition: &ProceduralFloorDefinition,
        pit: &GeneratedPitPlacement,
        occupied: &mut BTreeSet<Position>,
    ) -> Vec<Actor> {
        let table = self
            .content
            .encounter_table(&pit.definition.encounter_table_id)
            .expect("validated pit encounter table must remain available")
            .clone();
        let eligible = table
            .entries
            .iter()
            .filter(|entry| {
                entry.min_depth <= definition.depth
                    && definition.depth <= entry.max_depth
                    && self
                        .content
                        .actor(&entry.actor_kind_id)
                        .is_some_and(|actor| actor.level <= u32::from(definition.depth))
            })
            .cloned()
            .collect::<Vec<_>>();
        let pit_weights = eligible
            .iter()
            .map(|entry| entry.weight)
            .collect::<Vec<_>>();
        let mut roster = (0..pit.definition.roster_size)
            .map(|_| {
                eligible[self.roll_weighted_index(&pit_weights)]
                    .actor_kind_id
                    .clone()
            })
            .collect::<Vec<_>>();
        roster.sort_by(|left, right| {
            let left_level = self
                .content
                .actor(left)
                .expect("pit roster actor must remain available")
                .level;
            let right_level = self
                .content
                .actor(right)
                .expect("pit roster actor must remain available")
                .level;
            right_level.cmp(&left_level).then_with(|| left.cmp(right))
        });

        let half_width = pit.definition.inner_width / 2;
        let half_height = pit.definition.inner_height / 2;
        let maximum_rank = pit.definition.roster_size - 1;
        let mut ordinal = 0_u16;
        let mut actors = Vec::new();
        for local_y in 0..pit.definition.inner_height {
            for local_x in 0..pit.definition.inner_width {
                let dx = local_x.abs_diff(half_width);
                let dy = local_y.abs_diff(half_height);
                let horizontal_rank = dx * maximum_rank / half_width;
                let vertical_rank = dy * maximum_rank / half_height;
                let rank = usize::from(horizontal_rank.max(vertical_rank));
                let kind_id = &roster[rank];
                let position = Position {
                    x: pit.origin.x + 3 + i32::from(local_x),
                    y: pit.origin.y + 3 + i32::from(local_y),
                };
                occupied.insert(position);
                ordinal += 1;
                actors.push(self.generated_actor(
                    format!("{}.pit.{}", definition.id, ordinal),
                    kind_id,
                    position,
                ));
            }
        }
        actors
    }

    #[allow(clippy::too_many_arguments)]
    fn generate_dynamic_encounter_groups(
        &mut self,
        definition: &ProceduralFloorDefinition,
        table: &EncounterTableDefinition,
        eligible_entries: &[EncounterEntryDefinition],
        rooms: &[GeneratedRoom],
        room_id: &str,
        reserved_actor_slots: u16,
        ordinary_actor_reserve: u16,
        fill_plain: bool,
        id_prefix: &str,
        occupied: &mut BTreeSet<Position>,
    ) -> Vec<Actor> {
        let budget = definition
            .generation_budget
            .as_ref()
            .expect("dynamic encounters require a generation budget");
        let group_placement_limit = budget
            .group_placements
            .expect("validated group placement budget must remain available");
        let mut remaining_group_actor_slots = budget
            .group_actor_slots
            .expect("validated group actor budget must remain available");
        let mut remaining_actor_slots = budget.actor_slots.saturating_sub(reserved_actor_slots);
        let grouped_entries = eligible_entries
            .iter()
            .filter(|entry| entry.group.is_some())
            .cloned()
            .collect::<Vec<_>>();
        let plain_entries = eligible_entries
            .iter()
            .filter(|entry| entry.group.is_none())
            .cloned()
            .collect::<Vec<_>>();
        let minimum_group_companions = grouped_entries
            .iter()
            .filter_map(|entry| entry.group.as_ref())
            .map(rfb_content::EncounterGroupDefinition::min_companion_count)
            .min()
            .expect("validated dynamic floor must have a grouped encounter");
        let mut generated = Vec::new();
        let mut leader_ordinal = 0_u16;

        for group_slot in 0..group_placement_limit {
            let future_group_count = group_placement_limit - group_slot - 1;
            let future_companion_reserve =
                future_group_count.saturating_mul(minimum_group_companions);
            let future_actor_reserve = future_group_count
                .saturating_mul(minimum_group_companions.saturating_add(1))
                .saturating_add(ordinary_actor_reserve);
            let available_companion_slots = remaining_group_actor_slots
                .saturating_sub(future_companion_reserve)
                .min(
                    remaining_actor_slots
                        .saturating_sub(future_actor_reserve)
                        .saturating_sub(1),
                );
            let mut candidates = grouped_entries
                .iter()
                .filter(|entry| {
                    entry.group.as_ref().is_some_and(|group| {
                        group.min_companion_count() <= available_companion_slots
                    })
                })
                .cloned()
                .collect::<Vec<_>>();
            let mut placed_group = None;
            while !candidates.is_empty() {
                let weights = candidates
                    .iter()
                    .map(|entry| entry.weight)
                    .collect::<Vec<_>>();
                let selected_index = if candidates.len() == 1 {
                    0
                } else {
                    self.roll_weighted_index(&weights)
                };
                let entry = candidates.remove(selected_index);
                let group = entry
                    .group
                    .as_ref()
                    .expect("grouped encounter candidate must retain its group");
                let friend_min = group
                    .friends
                    .as_ref()
                    .map_or(0, |friends| friends.min_count);
                let friend_max = group
                    .friends
                    .as_ref()
                    .map_or(0, |friends| friends.max_count);
                let escort_min = group.escort.as_ref().map_or(0, |escort| escort.min_count);
                let escort_max = group.escort.as_ref().map_or(0, |escort| escort.max_count);
                let friend_upper =
                    friend_max.min(available_companion_slots.saturating_sub(escort_min));
                let mut friend_count = self.roll_inclusive(friend_min, friend_upper);
                let escort_upper =
                    escort_max.min(available_companion_slots.saturating_sub(friend_count));
                let mut escort_count = self.roll_inclusive(escort_min, escort_upper);
                let formation_placement = loop {
                    let placement_candidates = formation_placement_candidates(
                        rooms,
                        room_id,
                        occupied,
                        group.formation,
                        friend_count.saturating_add(escort_count),
                    );
                    if !placement_candidates.is_empty() {
                        let placement_index = if placement_candidates.len() == 1 {
                            0
                        } else {
                            usize::try_from(
                                self.rng
                                    .bounded(u64::try_from(placement_candidates.len()).expect(
                                        "formation placement candidate count must fit u64",
                                    )),
                            )
                            .expect("formation placement candidate index must fit usize")
                        };
                        break Some(placement_candidates[placement_index].clone());
                    }
                    if escort_count > escort_min {
                        escort_count -= 1;
                    } else if friend_count > friend_min {
                        friend_count -= 1;
                    } else {
                        break None;
                    }
                };
                let Some((leader_position, companion_positions)) = formation_placement else {
                    continue;
                };
                placed_group = Some((
                    entry,
                    friend_count,
                    escort_count,
                    leader_position,
                    companion_positions,
                ));
                break;
            }
            let Some((entry, friend_count, escort_count, leader_position, companion_positions)) =
                placed_group
            else {
                break;
            };

            leader_ordinal += 1;
            occupied.insert(leader_position);
            let leader_id = format!("{id_prefix}.encounter.{leader_ordinal}");
            let pack_id = format!("{id_prefix}.pack.{leader_ordinal}");
            let pack_ai = entry
                .group
                .as_ref()
                .expect("grouped encounter must retain pack AI")
                .pack_ai;
            generated.push(self.generated_pack_actor(
                leader_id.clone(),
                &entry.actor_kind_id,
                leader_position,
                MonsterPackIdentity {
                    id: pack_id.clone(),
                    leader_id: leader_id.clone(),
                    role: MonsterPackRoleDto::Leader,
                    behavior: monster_pack_behavior_dto(pack_ai.leader),
                },
            ));
            for (index, position) in companion_positions
                .iter()
                .take(usize::from(friend_count))
                .copied()
                .enumerate()
            {
                occupied.insert(position);
                generated.push(self.generated_pack_actor(
                    format!(
                        "{id_prefix}.encounter.{leader_ordinal}.friend.{}",
                        index + 1
                    ),
                    &entry.actor_kind_id,
                    position,
                    MonsterPackIdentity {
                        id: pack_id.clone(),
                        leader_id: leader_id.clone(),
                        role: MonsterPackRoleDto::Member,
                        behavior: monster_pack_behavior_dto(pack_ai.friends),
                    },
                ));
            }
            if escort_count > 0 {
                let escort = entry
                    .group
                    .as_ref()
                    .and_then(|group| group.escort.as_ref())
                    .expect("positive escort count must retain an escort table");
                let eligible_escorts = escort
                    .entries
                    .iter()
                    .filter(|escort_entry| {
                        escort_entry.min_depth <= definition.depth
                            && definition.depth <= escort_entry.max_depth
                            && self
                                .content
                                .actor(&escort_entry.actor_kind_id)
                                .is_some_and(|actor| actor.level <= u32::from(definition.depth))
                    })
                    .collect::<Vec<_>>();
                let escort_weights = eligible_escorts
                    .iter()
                    .map(|escort_entry| escort_entry.weight)
                    .collect::<Vec<_>>();
                for (index, position) in companion_positions
                    .iter()
                    .skip(usize::from(friend_count))
                    .take(usize::from(escort_count))
                    .copied()
                    .enumerate()
                {
                    let escort_index = if eligible_escorts.len() == 1 {
                        0
                    } else {
                        self.roll_weighted_index(&escort_weights)
                    };
                    let kind_id = &eligible_escorts[escort_index].actor_kind_id;
                    occupied.insert(position);
                    generated.push(self.generated_pack_actor(
                        format!(
                            "{id_prefix}.encounter.{leader_ordinal}.escort.{}",
                            index + 1
                        ),
                        kind_id,
                        position,
                        MonsterPackIdentity {
                            id: pack_id.clone(),
                            leader_id: leader_id.clone(),
                            role: MonsterPackRoleDto::Member,
                            behavior: monster_pack_behavior_dto(pack_ai.escorts),
                        },
                    ));
                }
            }
            let companion_count = friend_count.saturating_add(escort_count);
            remaining_group_actor_slots =
                remaining_group_actor_slots.saturating_sub(companion_count);
            remaining_actor_slots =
                remaining_actor_slots.saturating_sub(companion_count.saturating_add(1));
        }

        let plain_weights = plain_entries
            .iter()
            .map(|entry| entry.weight)
            .collect::<Vec<_>>();
        while fill_plain && leader_ordinal < table.rolls && remaining_actor_slots > 0 {
            let entry_index = if plain_entries.len() == 1 {
                0
            } else {
                self.roll_weighted_index(&plain_weights)
            };
            let entry = &plain_entries[entry_index];
            let position = self.choose_generated_room_position(rooms, room_id, occupied);
            occupied.insert(position);
            leader_ordinal += 1;
            generated.push(self.generated_actor(
                format!("{}.encounter.{leader_ordinal}", definition.id),
                &entry.actor_kind_id,
                position,
            ));
            remaining_actor_slots -= 1;
        }
        generated
    }

    fn roll_inclusive(&mut self, minimum: u16, maximum: u16) -> u16 {
        debug_assert!(minimum <= maximum);
        if minimum == maximum {
            minimum
        } else {
            minimum
                + u16::try_from(self.rng.bounded(u64::from(maximum - minimum) + 1))
                    .expect("bounded encounter group count must fit u16")
        }
    }

    fn select_spatial_vault_placements(
        &mut self,
        definition: &ProceduralFloorDefinition,
        eligible_candidates: &[ThemeVaultCandidateDefinition],
        guardian_present: bool,
        corridor_terrain_id: &str,
        terrain: &mut [String],
    ) -> Vec<GeneratedVaultPlacement> {
        let budget = definition
            .generation_budget
            .as_ref()
            .expect("spatial vault placement requires a generation budget");
        let placement_limit = budget
            .vault_placements
            .expect("validated spatial vault count must remain available");
        let mut remaining_area = budget
            .vault_area_tiles
            .expect("validated spatial vault area must remain available");
        let fixed_actor_slots = definition
            .nest
            .as_ref()
            .map_or(0, |nest| nest.spawn_count)
            .saturating_add(u16::from(guardian_present));
        let ordinary_placement_reserve = budget.region_placements.unwrap_or(1);
        let mut remaining_vault_actor_slots = budget
            .actor_slots
            .saturating_sub(fixed_actor_slots)
            .saturating_sub(ordinary_placement_reserve);
        let mut remaining_vault_loot_placements = budget
            .loot_placements
            .saturating_sub(ordinary_placement_reserve);
        let mut remaining_candidates = eligible_candidates.to_vec();
        let mut placements = Vec::new();

        'placement_slots: for ordinal in 1..=placement_limit {
            loop {
                let affordable = remaining_candidates
                    .iter()
                    .enumerate()
                    .filter_map(|(index, candidate)| {
                        let vault = self
                            .content
                            .vault(&candidate.vault_id)
                            .expect("validated spatial vault must remain available");
                        let actor_cost = vault
                            .encounter_groups
                            .iter()
                            .map(|group| {
                                u16::try_from(group.member_positions.len())
                                    .expect("validated vault actor count must fit u16")
                            })
                            .sum::<u16>();
                        let loot_cost = u16::try_from(vault.loot_spawns.len())
                            .expect("validated vault loot count must fit u16");
                        let area = u32::from(vault.width) * u32::from(vault.height);
                        (actor_cost <= remaining_vault_actor_slots
                            && loot_cost <= remaining_vault_loot_placements
                            && area <= remaining_area)
                            .then_some((index, candidate.weight))
                    })
                    .collect::<Vec<_>>();
                if affordable.is_empty() {
                    break 'placement_slots;
                }
                let selected_affordable = if affordable.len() == 1 {
                    0
                } else {
                    let weights = affordable
                        .iter()
                        .map(|(_, weight)| *weight)
                        .collect::<Vec<_>>();
                    self.roll_weighted_index(&weights)
                };
                let candidate_index = affordable[selected_affordable].0;
                let candidate = remaining_candidates.remove(candidate_index);
                let vault = self
                    .content
                    .vault(&candidate.vault_id)
                    .expect("validated spatial vault must remain available")
                    .clone();
                let placement_candidates = free_vault_placement_candidates(
                    terrain,
                    definition.width,
                    definition.height,
                    &definition.wall_terrain_id,
                    corridor_terrain_id,
                    &vault,
                    &self.content,
                );
                if placement_candidates.is_empty() {
                    continue;
                }
                let placement_index = if placement_candidates.len() == 1 {
                    0
                } else {
                    usize::try_from(
                        self.rng.bounded(
                            u64::try_from(placement_candidates.len())
                                .expect("vault placement candidate count must fit u64"),
                        ),
                    )
                    .expect("vault placement candidate index must fit usize")
                };
                let candidate = placement_candidates[placement_index].clone();
                let actor_cost = vault
                    .encounter_groups
                    .iter()
                    .map(|group| {
                        u16::try_from(group.member_positions.len())
                            .expect("validated vault actor count must fit u16")
                    })
                    .sum::<u16>();
                let loot_cost = u16::try_from(vault.loot_spawns.len())
                    .expect("validated vault loot count must fit u16");
                let area = u32::from(vault.width) * u32::from(vault.height);
                let placement = GeneratedVaultPlacement {
                    vault,
                    origin: candidate.origin,
                    transform: candidate.transform,
                    ordinal,
                    connector_cells: candidate.connector_cells,
                };
                apply_generated_vault_placement(
                    terrain,
                    definition.width,
                    corridor_terrain_id,
                    &placement,
                );
                remaining_vault_actor_slots =
                    remaining_vault_actor_slots.saturating_sub(actor_cost);
                remaining_vault_loot_placements =
                    remaining_vault_loot_placements.saturating_sub(loot_cost);
                remaining_area = remaining_area.saturating_sub(area);
                placements.push(placement);
                break;
            }
        }
        placements
    }

    fn place_terrain_features(
        &mut self,
        definition: &ProceduralFloorDefinition,
        eligible_entries: &[TerrainFeatureEntryDefinition],
        context: TerrainFeaturePlacementContext<'_>,
        terrain: &mut [String],
    ) -> Vec<GeneratedTerrainFeature> {
        let placement_limit = definition
            .generation_budget
            .as_ref()
            .and_then(|budget| budget.feature_placements)
            .expect("terrain feature placement requires a validated budget");
        let mut placements = Vec::new();

        'placement_slots: for _ in 0..placement_limit {
            let mut remaining_entries = eligible_entries.to_vec();
            loop {
                if remaining_entries.is_empty() {
                    break 'placement_slots;
                }
                let selected_index = if remaining_entries.len() == 1 {
                    0
                } else {
                    let weights = remaining_entries
                        .iter()
                        .map(|entry| entry.weight)
                        .collect::<Vec<_>>();
                    self.roll_weighted_index(&weights)
                };
                let entry = remaining_entries.remove(selected_index);
                let candidates = terrain_feature_placement_candidates(
                    terrain,
                    definition.width,
                    context.floor_terrain_id,
                    context.room_floor_terrain_ids,
                    context.rooms,
                    context.reserved,
                    entry.placement,
                );
                if candidates.is_empty() {
                    continue;
                }
                let position_index = if candidates.len() == 1 {
                    0
                } else {
                    usize::try_from(
                        self.rng.bounded(
                            u64::try_from(candidates.len())
                                .expect("terrain feature candidate count must fit u64"),
                        ),
                    )
                    .expect("terrain feature candidate index must fit usize")
                };
                let position = candidates[position_index];
                set_generated_terrain(terrain, definition.width, position, &entry.terrain_id);
                placements.push(GeneratedTerrainFeature {
                    terrain_id: entry.terrain_id,
                    position,
                });
                break;
            }
        }
        placements
    }

    fn choose_generated_room_position(
        &mut self,
        rooms: &[GeneratedRoom],
        room_id: &str,
        occupied: &BTreeSet<Position>,
    ) -> Position {
        let room = rooms
            .iter()
            .find(|room| room.id == room_id)
            .expect("validated procedural room ID must remain available");
        let candidates = (room.y..room.y + room.height)
            .flat_map(|y| (room.x..room.x + room.width).map(move |x| Position { x, y }))
            .filter(|position| room.contains(*position) && !occupied.contains(position))
            .collect::<Vec<_>>();
        let index = usize::try_from(self.rng.bounded(
            u64::try_from(candidates.len()).expect("generated room candidate count must fit u64"),
        ))
        .expect("bounded generated room candidate index must fit usize");
        candidates[index]
    }

    fn choose_generated_region_position(
        &mut self,
        region: &GeneratedRegion,
        terrain: &[String],
        width: u16,
        occupied: &BTreeSet<Position>,
    ) -> Position {
        let candidates =
            generated_region_open_positions(region, terrain, width, &self.content, occupied);
        let index = usize::try_from(self.rng.bounded(
            u64::try_from(candidates.len()).expect("regional candidate count must fit u64"),
        ))
        .expect("regional candidate index must fit usize");
        candidates[index]
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

    fn validate_loaded_state(&self) -> Result<(), CoreError> {
        let world = self
            .content
            .world(&self.world_id)
            .ok_or_else(|| CoreError::UnknownWorld(self.world_id.clone()))?;
        let valid_floor = |floor_id: &str| {
            floor_id == world.initial_floor_id
                || world
                    .procedural_floors
                    .iter()
                    .any(|floor| floor.id == floor_id)
        };
        if !valid_floor(&self.current_floor_id)
            || self
                .stored_floors
                .values()
                .any(|floor| !valid_floor(&floor.id))
        {
            return Err(CoreError::InvalidSave("floor identity is invalid"));
        }
        let current_dungeon_id = floor_dungeon_id(world, &self.current_floor_id);
        match (&current_dungeon_id, &self.current_dungeon_instance_id) {
            (Some(dungeon_id), Some(instance_id))
                if parse_dungeon_instance_ordinal(instance_id, dungeon_id).is_some() => {}
            (None, None) => {}
            _ => {
                return Err(CoreError::InvalidSave(
                    "active floor dungeon instance identity is invalid",
                ));
            }
        }
        if let Some(recall) = &self.recall {
            let destination_is_valid = world.procedural_floors.iter().any(|floor| {
                floor.id == recall.floor_id
                    && floor.lifecycle == FloorLifecycle::Dungeon
                    && floor.dungeon_id.as_deref() == Some(recall.dungeon_id.as_str())
            });
            let pending_is_valid = recall
                .remaining_turns
                .is_none_or(|turns| (1..=2_000).contains(&turns));
            let current_location_allows_pending = recall.remaining_turns.is_none()
                || self.current_floor_id == world.initial_floor_id
                || current_dungeon_id.is_some();
            if !destination_is_valid || !pending_is_valid || !current_location_allows_pending {
                return Err(CoreError::InvalidSave("player recall state is invalid"));
            }
        }
        for floor in self.stored_floors.values() {
            let expected_instance = floor_dungeon_id(world, &floor.id).is_some();
            if expected_instance != floor.dungeon_instance_id.is_some() {
                return Err(CoreError::InvalidSave(
                    "stored floor dungeon instance identity is invalid",
                ));
            }
        }
        if !floor_connections_are_valid(
            &self.current_floor_id,
            self.width,
            self.height,
            &self.terrain,
            &self.floor_connections,
            world,
        ) {
            return Err(CoreError::InvalidSave(
                "active floor connection state is invalid",
            ));
        }
        if !floor_regions_are_valid(
            &self.current_floor_id,
            (self.width, self.height),
            &self.floor_regions,
            &self.entities,
            &self.items,
            world,
            &self.content,
        ) {
            return Err(CoreError::InvalidSave(
                "active floor region state is invalid",
            ));
        }
        if self.explored.len() != self.terrain.len() {
            return Err(CoreError::InvalidSave(
                "exploration memory dimensions are invalid",
            ));
        }
        if !revealed_terrain_is_valid(
            &self.revealed_terrain,
            &self.terrain,
            self.width,
            self.height,
            &self.content,
        ) {
            return Err(CoreError::InvalidSave(
                "revealed terrain knowledge is invalid",
            ));
        }
        match (self.summon_command.mode, self.summon_command.guard_position) {
            (SummonCommandModeDto::Guard, Some(position))
                if self.index(position).is_some() && self.is_walkable(position) => {}
            (SummonCommandModeDto::Guard, _) => {
                return Err(CoreError::InvalidSave(
                    "summon guard command position is invalid",
                ));
            }
            (_, None) => {}
            (_, Some(_)) => {
                return Err(CoreError::InvalidSave(
                    "non-guard summon command retains a guard position",
                ));
            }
        }
        for terrain_id in &self.terrain {
            if self.content.terrain(terrain_id).is_none() {
                return Err(CoreError::UnknownTerrain(terrain_id.clone()));
            }
        }
        let victory_cap_unlocked = self.campaign_state.status != CampaignStatusDto::Active;
        let expected_skills =
            character_skill_progress(&self.content, self.build.as_ref(), self.progress.level)?;
        if !self.progress.validate(victory_cap_unlocked)
            || self.progress.skills != expected_skills
            || self.progress.hp_progression.first().copied() != Some(self.player.max_hp)
            || self.progress.hp_progression.windows(2).any(|window| {
                let increase = window[1].saturating_sub(window[0]);
                !(1..=10).contains(&increase)
            })
        {
            return Err(CoreError::InvalidSave("character progress is invalid"));
        }
        self.validate_actor(&self.player, ActorRole::Player)?;
        if self.index(self.player.position).is_none() {
            return Err(CoreError::InvalidSave("player position is invalid"));
        }
        let mut instance_ids = BTreeSet::new();
        instance_ids.insert(self.player.id.clone());
        let mut monster_ids = BTreeSet::new();
        let mut positions = BTreeSet::new();
        positions.insert(self.player.position);
        for entity in &self.entities {
            self.validate_actor(entity, ActorRole::Monster)?;
            if let Some(summon) = &entity.summon
                && !self.summon_identity_is_valid(entity, summon)
            {
                return Err(CoreError::InvalidSave("summon state is invalid"));
            }
            if !instance_ids.insert(entity.id.clone())
                || !self.is_walkable(entity.position)
                || !positions.insert(entity.position)
            {
                return Err(CoreError::InvalidSave("entity position is invalid"));
            }
            monster_ids.insert(entity.id.clone());
        }
        if !monster_packs_are_valid(&self.entities) {
            return Err(CoreError::InvalidSave("monster pack state is invalid"));
        }
        let mut equipment_slots = BTreeSet::new();
        for item in &self.items {
            let definition = self
                .content
                .item(&item.kind_id)
                .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?;
            let affixes_are_valid = item.affix_ids.windows(2).all(|pair| pair[0] < pair[1])
                && item
                    .affix_ids
                    .iter()
                    .all(|affix_id| self.content.affix(affix_id).is_some())
                && rolled_affixes_are_valid(item)
                && (item.affix_ids.is_empty()
                    || (definition.max_stack == 1
                        && definition.equipment_slot.is_some()
                        && item.quantity == 1
                        && item.quality != ItemQualityDto::Ordinary))
                && (item.quality == ItemQualityDto::Ordinary
                    || (definition.max_stack == 1 && item.quantity == 1));
            let common_valid = instance_ids.insert(item.id.clone()) && item.quantity != 0;
            if !affixes_are_valid {
                return Err(CoreError::InvalidSave(
                    "item quality or affix state is invalid",
                ));
            }
            match &item.location {
                ItemLocation::Ground(position) => {
                    if !common_valid
                        || !self.is_walkable(*position)
                        || item.quantity > definition.max_stack
                    {
                        return Err(CoreError::InvalidSave("item state is invalid"));
                    }
                }
                ItemLocation::Inventory => {
                    if !common_valid || item.quantity > definition.max_stack {
                        return Err(CoreError::InvalidSave("inventory item state is invalid"));
                    }
                }
                ItemLocation::Equipped { slot_id } => {
                    let fully_identified =
                        self.item_property_knowledge
                            .get(&item.id)
                            .is_some_and(|knowledge| {
                                knowledge.identified
                                    && item.affix_ids.iter().all(|affix_id| {
                                        knowledge.known_affix_ids.contains(affix_id)
                                    })
                            });
                    // The occupied instance must exist on the body and its
                    // type must match the item's declared slot class.
                    let slot_type_matches = self.body_slot_type(slot_id).is_some_and(|slot_type| {
                        definition.equipment_slot.as_deref() == Some(slot_type)
                    });
                    if !common_valid
                        || item.quantity != 1
                        || !slot_type_matches
                        || !equipment_slots.insert(slot_id.clone())
                        || !fully_identified
                    {
                        return Err(CoreError::InvalidSave("equipment item state is invalid"));
                    }
                }
                ItemLocation::CarriedBy { actor_id } => {
                    if !common_valid
                        || !monster_ids.contains(actor_id)
                        || item.quantity > definition.max_stack
                    {
                        return Err(CoreError::InvalidSave("carried item state is invalid"));
                    }
                }
            }
        }
        for floor in self.stored_floors.values() {
            let expected_len = usize::from(floor.width) * usize::from(floor.height);
            if floor.terrain.len() != expected_len
                || floor.explored.len() != expected_len
                || !revealed_terrain_is_valid(
                    &floor.revealed_terrain,
                    &floor.terrain,
                    floor.width,
                    floor.height,
                    &self.content,
                )
                || (floor.id == self.current_floor_id
                    && floor.dungeon_instance_id == self.current_dungeon_instance_id)
                || !floor_position_is_walkable(floor, floor.player_position, &self.content)
            {
                return Err(CoreError::InvalidSave("stored floor state is invalid"));
            }
            if !floor_connections_are_valid(
                &floor.id,
                floor.width,
                floor.height,
                &floor.terrain,
                &floor.connections,
                world,
            ) {
                return Err(CoreError::InvalidSave(
                    "stored floor connection state is invalid",
                ));
            }
            if !floor_regions_are_valid(
                &floor.id,
                (floor.width, floor.height),
                &floor.regions,
                &floor.entities,
                &floor.items,
                world,
                &self.content,
            ) {
                return Err(CoreError::InvalidSave(
                    "stored floor region state is invalid",
                ));
            }
            for terrain_id in &floor.terrain {
                if self.content.terrain(terrain_id).is_none() {
                    return Err(CoreError::UnknownTerrain(terrain_id.clone()));
                }
            }
            let mut floor_positions = BTreeSet::new();
            let mut floor_monster_ids = BTreeSet::new();
            for entity in &floor.entities {
                self.validate_actor(entity, ActorRole::Monster)?;
                if !instance_ids.insert(entity.id.clone())
                    || !floor_position_is_walkable(floor, entity.position, &self.content)
                    || !floor_positions.insert(entity.position)
                {
                    return Err(CoreError::InvalidSave(
                        "stored floor entity state is invalid",
                    ));
                }
                floor_monster_ids.insert(entity.id.clone());
            }
            if !monster_packs_are_valid(&floor.entities) {
                return Err(CoreError::InvalidSave(
                    "stored floor monster pack state is invalid",
                ));
            }
            for item in &floor.items {
                let definition = self
                    .content
                    .item(&item.kind_id)
                    .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?;
                let affixes_are_valid = item.affix_ids.windows(2).all(|pair| pair[0] < pair[1])
                    && item
                        .affix_ids
                        .iter()
                        .all(|affix_id| self.content.affix(affix_id).is_some())
                    && rolled_affixes_are_valid(item)
                    && (item.affix_ids.is_empty()
                        || (definition.max_stack == 1
                            && definition.equipment_slot.is_some()
                            && item.quantity == 1
                            && item.quality != ItemQualityDto::Ordinary))
                    && (item.quality == ItemQualityDto::Ordinary
                        || (definition.max_stack == 1 && item.quantity == 1));
                let location_is_valid = match &item.location {
                    ItemLocation::Ground(position) => {
                        floor_position_is_walkable(floor, *position, &self.content)
                    }
                    ItemLocation::CarriedBy { actor_id } => floor_monster_ids.contains(actor_id),
                    ItemLocation::Inventory | ItemLocation::Equipped { .. } => false,
                };
                if !instance_ids.insert(item.id.clone())
                    || item.quantity == 0
                    || item.quantity > definition.max_stack
                    || !affixes_are_valid
                    || !location_is_valid
                {
                    return Err(CoreError::InvalidSave("stored floor item state is invalid"));
                }
            }
        }
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        let expected_tasks = initial_task_states(world);
        if self.task_states.len() != expected_tasks.len() {
            return Err(CoreError::InvalidSave("task state set is invalid"));
        }
        for (task_id, state) in &self.task_states {
            let Some(expected) = expected_tasks.get(task_id) else {
                return Err(CoreError::InvalidSave("task state ID is invalid"));
            };
            let members = world
                .procedural_floors
                .iter()
                .filter(|floor| floor_task_id(floor) == task_id)
                .collect::<Vec<_>>();
            let objectives = task_objectives(world, task_id);
            let Some(objective) = usize::try_from(state.stage_index)
                .ok()
                .and_then(|stage| objectives.get(stage))
            else {
                return Err(CoreError::InvalidSave("task stage is invalid"));
            };
            let active_is_valid = state.active_floor_id.as_ref().is_some_and(|floor_id| {
                floor_id == &self.current_floor_id
                    && members.iter().any(|floor| floor.id == *floor_id)
            });
            let paused_is_valid = members.iter().any(|floor| {
                self.stored_floors
                    .values()
                    .any(|stored| stored.id == floor.id)
            });
            let status_is_valid = match state.status {
                TaskStatusKindDto::Active => active_is_valid,
                TaskStatusKindDto::Paused => state.active_floor_id.is_none() && paused_is_valid,
                TaskStatusKindDto::Completed => {
                    state.active_floor_id.is_none()
                        && usize::try_from(state.stage_index)
                            .ok()
                            .is_some_and(|stage| stage + 1 == objectives.len())
                        && state.current == state.required
                }
                TaskStatusKindDto::Available
                | TaskStatusKindDto::Failed
                | TaskStatusKindDto::Abandoned => state.active_floor_id.is_none(),
            };
            if (state.stage_index == 0 && expected.required != objective.required)
                || state.required != objective.required
                || state.current > state.required
                || members
                    .first()
                    .and_then(|floor| floor.max_retakes)
                    .is_some_and(|maximum| state.retakes_used > maximum)
                || !status_is_valid
            {
                return Err(CoreError::InvalidSave("task state is invalid"));
            }
        }
        let expected_dungeons = initial_dungeon_states(world);
        if self.dungeon_states.len() != expected_dungeons.len() {
            return Err(CoreError::InvalidSave("dungeon state set is invalid"));
        }
        for (dungeon_id, state) in &self.dungeon_states {
            if !expected_dungeons.contains_key(dungeon_id) {
                return Err(CoreError::InvalidSave("dungeon state ID is invalid"));
            }
            let dungeon = world
                .dungeons
                .iter()
                .find(|dungeon| dungeon.id == *dungeon_id)
                .expect("validated dungeon state must retain its definition");
            if dungeon.entrance_guardian.is_none() && state.entrance_guardian_defeated {
                return Err(CoreError::InvalidSave(
                    "dungeon entrance guardian state is invalid",
                ));
            }
            match (&state.retained_instance_id, state.retained_at_turn) {
                (None, None) => {}
                (Some(instance_id), Some(retained_at_turn)) => {
                    if dungeon.instance_lifecycle == DungeonInstanceLifecycle::ResetOnSurface
                        || parse_dungeon_instance_ordinal(instance_id, dungeon_id).is_none()
                        || retained_at_turn > self.turn
                        || !self.stored_floors.values().any(|floor| {
                            floor.dungeon_instance_id.as_deref() == Some(instance_id.as_str())
                        })
                    {
                        return Err(CoreError::InvalidSave(
                            "retained dungeon instance state is invalid",
                        ));
                    }
                }
                _ => {
                    return Err(CoreError::InvalidSave(
                        "retained dungeon instance state is incomplete",
                    ));
                }
            }
            if let Some(guardian) = &dungeon.entrance_guardian {
                let guardian_present = if self.current_floor_id == world.initial_floor_id {
                    Some(
                        self.entities
                            .iter()
                            .any(|actor| actor.id == guardian.instance_id),
                    )
                } else {
                    self.stored_floors
                        .values()
                        .find(|stored| stored.id == world.initial_floor_id)
                        .map(|floor| {
                            floor
                                .entities
                                .iter()
                                .any(|actor| actor.id == guardian.instance_id)
                        })
                };
                if guardian_present
                    .is_none_or(|present| present == state.entrance_guardian_defeated)
                {
                    return Err(CoreError::InvalidSave(
                        "dungeon entrance guardian state is invalid",
                    ));
                }
            }
            for final_floor in world.procedural_floors.iter().filter(|floor| {
                floor.dungeon_id.as_deref() == Some(dungeon_id.as_str()) && floor.final_floor
            }) {
                let guardian_id = &final_floor
                    .guardian
                    .as_ref()
                    .expect("validated final floor must retain a guardian")
                    .instance_id;
                let guardian_present = if self.current_floor_id == final_floor.id {
                    Some(self.entities.iter().any(|actor| &actor.id == guardian_id))
                } else {
                    self.stored_floors
                        .values()
                        .find(|stored| stored.id == final_floor.id)
                        .map(|floor| floor.entities.iter().any(|actor| &actor.id == guardian_id))
                };
                if guardian_present.is_some_and(|present| present == state.guardian_defeated) {
                    return Err(CoreError::InvalidSave("dungeon guardian state is invalid"));
                }
            }
        }
        let campaign_victory_reached = self.campaign_victory_reached();
        match self.campaign_definition() {
            None if self.campaign_state.status != CampaignStatusDto::Active => {
                return Err(CoreError::InvalidSave("campaign state is invalid"));
            }
            None => {}
            Some(_) => match self.campaign_state.status {
                CampaignStatusDto::Active => {
                    if self.campaign_state.victory_turn.is_some()
                        || self.campaign_state.retired_turn.is_some()
                        || self.campaign_state.final_score.is_some()
                        || campaign_victory_reached
                    {
                        return Err(CoreError::InvalidSave("campaign state is invalid"));
                    }
                }
                CampaignStatusDto::Victorious => {
                    if !campaign_victory_reached
                        || self.campaign_state.retired_turn.is_some()
                        || self.campaign_state.final_score.is_some()
                        || self
                            .campaign_state
                            .victory_turn
                            .is_none_or(|turn| turn > self.turn)
                    {
                        return Err(CoreError::InvalidSave("campaign state is invalid"));
                    }
                }
                CampaignStatusDto::Retired => {
                    let valid_turns = self
                        .campaign_state
                        .victory_turn
                        .zip(self.campaign_state.retired_turn)
                        .is_some_and(|(victory, retired)| {
                            victory <= retired && retired <= self.turn
                        });
                    let valid_score = self.campaign_state.final_score.is_some_and(|score| {
                        self.campaign_state
                            .retired_turn
                            .is_some_and(|turn| score == self.campaign_score_at(turn))
                    });
                    if !campaign_victory_reached
                        || self.current_floor_id != world.initial_floor_id
                        || self.current_dungeon_instance_id.is_some()
                        || !valid_turns
                        || !valid_score
                    {
                        return Err(CoreError::InvalidSave("campaign state is invalid"));
                    }
                }
            },
        }
        let casting_profile = self.casting_profile().cloned();
        let technique_profiles = self.technique_profiles().to_vec();
        let device_recharge_profile = self.device_recharge_profile().cloned();
        if self.bonus_spell_learning_capacity > 0 && !self.uses_spell_scrolls() {
            return Err(CoreError::InvalidSave(
                "bonus spell learning capacity is invalid",
            ));
        }
        if casting_profile.is_some()
            || !technique_profiles.is_empty()
            || device_recharge_profile.is_some()
        {
            let (expected_pool_maxima, expected_ability_ids) = self.player_ability_baseline();
            let pools_valid = self.resources.len() == expected_pool_maxima.len()
                && expected_pool_maxima.iter().all(|(id, expected_maximum)| {
                    self.resources.get(id).is_some_and(|pool| {
                        pool.maximum == *expected_maximum && pool.current <= pool.maximum
                    })
                });
            let learned_valid = match &casting_profile {
                Some(profile) => {
                    self.learned_abilities.len()
                        <= usize::from(self.ability_learning_capacity(profile))
                        && self.learned_abilities.iter().all(|ability_id| {
                            self.content.ability(ability_id).is_some_and(|ability| {
                                let ability = Self::effective_casting_ability(profile, ability);
                                ability.minimum_level <= self.progress.level
                                    && self.profile_supports_ability(profile, ability_id)
                            })
                        })
                }
                None => self.learned_abilities.is_empty(),
            };
            if !pools_valid
                || !learned_valid
                || self
                    .ability_progress
                    .keys()
                    .cloned()
                    .collect::<BTreeSet<_>>()
                    != expected_ability_ids
                || self.ability_progress.iter().any(|(ability_id, progress)| {
                    self.content.ability(ability_id).is_none_or(|ability| {
                        progress.proficiency_cap != ability.proficiency.cap
                            || progress.proficiency > progress.proficiency_cap
                            || progress.cooldown_remaining > self.ability_cooldown_turns(ability_id)
                    })
                })
            {
                return Err(CoreError::InvalidSave("player ability state is invalid"));
            }
        } else if !self.resources.is_empty()
            || !self.learned_abilities.is_empty()
            || !self.ability_progress.is_empty()
        {
            return Err(CoreError::InvalidSave(
                "non-caster player ability state is invalid",
            ));
        }
        for (item_id, knowledge) in &self.item_property_knowledge {
            let Some(item) = self
                .items
                .iter()
                .chain(
                    self.stored_floors
                        .values()
                        .flat_map(|floor| floor.items.iter()),
                )
                .find(|item| &item.id == item_id)
            else {
                return Err(CoreError::InvalidSave(
                    "item property knowledge state is invalid",
                ));
            };
            let empty_knowledge = !knowledge.appraised
                && !knowledge.identified
                && knowledge.known_affix_ids.is_empty();
            let identification_without_appraisal = knowledge.identified && !knowledge.appraised;
            let foreign_affix = knowledge
                .known_affix_ids
                .iter()
                .any(|affix_id| !item.affix_ids.contains(affix_id));
            let incomplete_identification = knowledge.identified
                && item
                    .affix_ids
                    .iter()
                    .any(|affix_id| !knowledge.known_affix_ids.contains(affix_id));
            if empty_knowledge
                || identification_without_appraisal
                || foreign_affix
                || incomplete_identification
            {
                return Err(CoreError::InvalidSave(
                    "item property knowledge state is invalid",
                ));
            }
        }
        let mut allocator_entities = self.entities.clone();
        let mut allocator_items = self.items.clone();
        for floor in self.stored_floors.values() {
            allocator_entities.extend(floor.entities.iter().cloned());
            allocator_items.extend(floor.items.iter().cloned());
        }
        if self.next_item_instance_serial == 0
            || self.next_item_instance_serial
                < derive_next_item_instance_serial(
                    &self.player,
                    &allocator_entities,
                    &allocator_items,
                )?
        {
            return Err(CoreError::InvalidSave(
                "item instance allocator is behind existing IDs",
            ));
        }
        Ok(())
    }

    fn validate_actor(&self, actor: &Actor, expected_role: ActorRole) -> Result<(), CoreError> {
        let definition = self
            .content
            .actor(&actor.kind_id)
            .ok_or_else(|| CoreError::UnknownActor(actor.kind_id.clone()))?;
        let effective_max_hp = if expected_role == ActorRole::Player {
            self.effective_player_max_hp()
        } else {
            actor.max_hp
        };
        let statuses_are_valid = actor.statuses.iter().all(|status| {
            status.intensity > 0
                && status.remaining_ticks > 0
                && !status.kind_id.is_empty()
                && status.kind_id.len() <= 128
                && status.granted_resistances.len() <= 29
                && status
                    .granted_resistances
                    .values()
                    .all(|level| *level != ResistanceLevel::Normal)
                && (1..=100).contains(&status.incoming_damage_percent)
                && status
                    .granted_race_id
                    .as_deref()
                    .is_none_or(|race_id| self.content.race(race_id).is_some())
        }) && actor
            .statuses
            .windows(2)
            .all(|window| window[0].kind_id < window[1].kind_id);
        let resistance_memory_is_valid = if actor.observed_player_resistances.is_empty() {
            true
        } else {
            expected_role == ActorRole::Monster
                && actor.observed_player_resistances.len() <= 6
                && definition
                    .monster_casting
                    .as_ref()
                    .is_some_and(|casting| casting.smart)
                && !self.actor_is_player_aligned(actor)
        };
        if definition.role != expected_role
            || actor.max_hp != definition.max_hp
            || actor.speed != definition.speed
            || actor.speed > 199
            || !statuses_are_valid
            || !resistance_memory_is_valid
            || (expected_role == ActorRole::Monster && actor.hp <= 0)
            || (expected_role == ActorRole::Player && actor.hp < -1_000_000)
            || (expected_role == ActorRole::Monster
                && !(1..=STANDARD_ACTION_COST).contains(&actor.energy_need))
            || (expected_role == ActorRole::Player && actor.hp >= 0 && actor.energy_need > 0)
            || actor.energy_need < -STANDARD_ACTION_COST
            || actor.hp > effective_max_hp
            || (expected_role == ActorRole::Player && actor.pack.is_some())
            || (expected_role == ActorRole::Player && actor.controller_id.is_some())
            || actor
                .controller_id
                .as_deref()
                .is_some_and(|controller_id| controller_id != self.player.id)
            || (actor.controller_id.is_some() && actor.pack.is_some())
            || (actor.summon.is_some() && actor.pack.is_some())
            || definition.monster_casting.as_ref().map_or(
                actor.casting_cooldown_remaining != 0,
                |casting| {
                    actor.casting_cooldown_remaining
                        > monster_casting_cooldown(casting.frequency_percent)
                },
            )
        {
            return Err(CoreError::InvalidSave("actor state is invalid"));
        }
        Ok(())
    }

    fn summon_identity_is_valid(&self, actor: &Actor, summon: &SummonIdentity) -> bool {
        let valid_id = |id: &str| {
            !id.is_empty()
                && id.len() <= 256
                && id.bytes().all(|byte| {
                    byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
                })
        };
        valid_id(&summon.owner_id)
            && valid_id(&summon.source_ability_id)
            && summon.remaining_turns > 0
            && self
                .content
                .ability(&summon.source_ability_id)
                .cloned()
                .map(|ability| {
                    let mut ability = self.casting_profile().map_or_else(
                        || ability.clone(),
                        |profile| Self::effective_casting_ability(profile, &ability),
                    );
                    Self::apply_player_level_scaling(&mut ability, self.progress.level);
                    ability
                })
                .is_some_and(|ability| match &ability.effect {
                    AbilityEffectDefinition::Summon { actor_kind_id, .. } => {
                        actor_kind_id == &actor.kind_id
                    }
                    AbilityEffectDefinition::SummonCategory {
                        category,
                        upgraded_category,
                        maximum_level,
                        ..
                    } => self.content.actor(&actor.kind_id).is_some_and(|kind| {
                        kind.tags.iter().any(|tag| {
                            tag == category
                                || upgraded_category
                                    .as_ref()
                                    .is_some_and(|upgraded| tag == upgraded)
                        }) && kind.level <= u32::from(*maximum_level)
                    }),
                    _ => false,
                })
    }
}

struct ActorDerivedStats {
    max_hp: DerivedStat,
    attack: DerivedStat,
    defense: DerivedStat,
    speed: DerivedStat,
    melee_skill: DerivedStat,
    armor_class: DerivedStat,
    melee_attacks: DerivedStat,
    melee_damage_bonus: DerivedStat,
    ranged_skill: DerivedStat,
    throwing_skill: DerivedStat,
    door_skill: DerivedStat,
    bash_power: DerivedStat,
    search_skill: DerivedStat,
    device_skill: DerivedStat,
    saving_throw_skill: DerivedStat,
    stealth_skill: DerivedStat,
    perception_skill: DerivedStat,
    disarm_skill: DerivedStat,
    dig_skill: DerivedStat,
}

#[derive(Clone)]
struct ResolvedAttackProfile {
    attacks: u16,
    to_hit: i32,
    to_damage: i32,
    damage_dice: u16,
    damage_sides: u16,
    damage_type: DamageType,
    source_item_id: Option<String>,
}

struct ResolvedMeleeBlow {
    method_id: Option<String>,
    to_hit: i32,
    damage_dice: u16,
    damage_sides: u16,
    damage_type: DamageType,
}

struct ResolvedProjectileProfile {
    range: u16,
    to_hit: i32,
    to_damage: i32,
    ammunition_to_hit: u16,
    damage_dice: u16,
    damage_sides: u16,
    damage_type: DamageType,
    ammo_kind_id: String,
    ammo_break_chance_percent: u8,
    source_item_id: String,
}

#[derive(Clone)]
struct ResolvedThrowProfile {
    to_hit: i32,
    to_damage: i32,
    damage_dice: u16,
    damage_sides: u16,
    damage_type: DamageType,
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

impl ResolvedProjectileProfile {
    fn to_dto(&self) -> ProjectileProfileDto {
        ProjectileProfileDto {
            range: self.range,
            to_hit: self.to_hit,
            to_damage: self.to_damage,
            damage: DamageDiceDto {
                dice: self.damage_dice,
                sides: self.damage_sides,
                damage_type: self.damage_type.into(),
            },
            ammo_kind_id: self.ammo_kind_id.clone(),
            target_spec: projectile_target_spec(self.range),
            source_item_id: self.source_item_id.clone(),
        }
    }
}

fn resolved_melee_blows(definition: &rfb_content::ActorDefinition) -> Vec<ResolvedMeleeBlow> {
    definition.melee_routine.as_ref().map_or_else(
        || {
            vec![ResolvedMeleeBlow {
                method_id: None,
                to_hit: 0,
                damage_dice: definition.damage_dice,
                damage_sides: definition.damage_sides,
                damage_type: DamageType::from(definition.damage_type),
            }]
        },
        |routine| {
            routine
                .blows
                .iter()
                .map(|blow| ResolvedMeleeBlow {
                    method_id: Some(blow.method_id.clone()),
                    to_hit: blow.to_hit,
                    damage_dice: blow.damage_dice,
                    damage_sides: blow.damage_sides,
                    damage_type: DamageType::from(blow.damage_type),
                })
                .collect()
        },
    )
}

fn actor_melee_routine_dto(definition: &rfb_content::ActorDefinition) -> MeleeRoutineDto {
    MeleeRoutineDto {
        blows: resolved_melee_blows(definition)
            .into_iter()
            .map(|blow| MeleeBlowDto {
                method_id: blow
                    .method_id
                    .unwrap_or_else(|| "rfb.blow.innate".to_owned()),
                to_hit: blow.to_hit,
                damage: DamageDiceDto {
                    dice: blow.damage_dice,
                    sides: blow.damage_sides,
                    damage_type: blow.damage_type.into(),
                },
            })
            .collect(),
    }
}

impl ResolvedAttackProfile {
    fn to_dto(&self) -> AttackProfileDto {
        AttackProfileDto {
            attacks: self.attacks,
            to_hit: self.to_hit,
            to_damage: self.to_damage,
            damage: DamageDiceDto {
                dice: self.damage_dice,
                sides: self.damage_sides,
                damage_type: self.damage_type.into(),
            },
            source_item_id: self.source_item_id.clone(),
        }
    }
}

fn add_nonzero_stat(
    pipeline: &mut DerivedStatsPipeline,
    kind: StatKind,
    layer: StatLayer,
    source_id: &str,
    amount: i32,
) {
    if amount != 0 {
        pipeline.add(kind, layer, source_id, amount);
    }
}

fn add_equipment_stat(
    pipeline: &mut DerivedStatsPipeline,
    kind: StatKind,
    source_id: &str,
    amount: i32,
) {
    if amount != 0 {
        pipeline.add(kind, StatLayer::Equipment, source_id, amount);
    }
}

fn derived_speed(speed: &DerivedStat) -> u16 {
    u16::try_from(speed.value).expect("derived actor speed must fit u16")
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

impl Game {
    fn generate_maze(
        &mut self,
        definition: &ProceduralFloorDefinition,
        maze: &ProceduralMazeDefinition,
        floor_terrain_id: &str,
        terrain: &mut [String],
    ) -> BTreeSet<Position> {
        let left = i32::from((definition.width - maze.width) / 2);
        let top = i32::from((definition.height - maze.height) / 2);
        for y in top..top + i32::from(maze.height) {
            for x in left..left + i32::from(maze.width) {
                set_generated_terrain(
                    terrain,
                    definition.width,
                    Position { x, y },
                    &definition.wall_terrain_id,
                );
            }
        }

        let columns = usize::from(maze.width.div_ceil(2));
        let rows = usize::from(maze.height.div_ceil(2));
        let vertex_count = columns * rows;
        let root = usize::try_from(
            self.rng
                .bounded(u64::try_from(vertex_count).expect("maze vertex count must fit u64")),
        )
        .expect("maze root must fit usize");
        let node_position = |node: usize| Position {
            x: left + i32::try_from((node % columns) * 2).expect("maze x must fit i32"),
            y: top + i32::try_from((node / columns) * 2).expect("maze y must fit i32"),
        };
        let mut visited = BTreeSet::from([root]);
        let mut stack = vec![root];
        let mut carved = BTreeSet::new();
        let root_position = node_position(root);
        carved.insert(root_position);
        set_generated_terrain(terrain, definition.width, root_position, floor_terrain_id);

        while let Some(&node) = stack.last() {
            let column = node % columns;
            let row = node / columns;
            let mut neighbors = Vec::new();
            if row > 0 && !visited.contains(&(node - columns)) {
                neighbors.push(node - columns);
            }
            if column + 1 < columns && !visited.contains(&(node + 1)) {
                neighbors.push(node + 1);
            }
            if row + 1 < rows && !visited.contains(&(node + columns)) {
                neighbors.push(node + columns);
            }
            if column > 0 && !visited.contains(&(node - 1)) {
                neighbors.push(node - 1);
            }
            if neighbors.is_empty() {
                stack.pop();
                continue;
            }
            let neighbor_index = if neighbors.len() == 1 {
                0
            } else {
                usize::try_from(self.rng.bounded(
                    u64::try_from(neighbors.len()).expect("maze neighbor count must fit u64"),
                ))
                .expect("maze neighbor index must fit usize")
            };
            let neighbor = neighbors[neighbor_index];
            let from = node_position(node);
            let to = node_position(neighbor);
            let connector = Position {
                x: (from.x + to.x) / 2,
                y: (from.y + to.y) / 2,
            };
            for position in [connector, to] {
                carved.insert(position);
                set_generated_terrain(terrain, definition.width, position, floor_terrain_id);
            }
            visited.insert(neighbor);
            stack.push(neighbor);
        }

        carved
    }

    fn generate_destroyed_region(
        &mut self,
        definition: &ProceduralFloorDefinition,
        terrain_id: &str,
        terrain: &mut [String],
    ) -> BTreeSet<Position> {
        const CARDINAL_OFFSETS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
        let budget = definition
            .generation_budget
            .as_ref()
            .expect("destroyed region requires a generation budget");
        let center_count = usize::from(
            budget
                .destruction_centers
                .expect("validated destruction center budget must remain available"),
        );
        let area = usize::try_from(
            budget
                .destroyed_area_tiles
                .expect("validated destroyed area budget must remain available"),
        )
        .expect("destroyed area must fit usize");
        let margin_x = i32::from((definition.width / 5).max(2));
        let margin_y = i32::from((definition.height / 5).max(2));
        let mut center_candidates = (margin_y..i32::from(definition.height) - margin_y)
            .flat_map(|y| {
                (margin_x..i32::from(definition.width) - margin_x).map(move |x| Position { x, y })
            })
            .collect::<Vec<_>>();
        let mut selected = BTreeSet::new();
        for _ in 0..center_count {
            let index = if center_candidates.len() == 1 {
                0
            } else {
                usize::try_from(
                    self.rng.bounded(
                        u64::try_from(center_candidates.len())
                            .expect("destruction center count must fit u64"),
                    ),
                )
                .expect("destruction center index must fit usize")
            };
            selected.insert(center_candidates.remove(index));
        }

        while selected.len() < area {
            let mut frontier = selected
                .iter()
                .flat_map(|position| {
                    CARDINAL_OFFSETS.map(|(dx, dy)| Position {
                        x: position.x + dx,
                        y: position.y + dy,
                    })
                })
                .filter(|position| {
                    position.x > 0
                        && position.y > 0
                        && position.x + 1 < i32::from(definition.width)
                        && position.y + 1 < i32::from(definition.height)
                        && !selected.contains(position)
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            frontier.sort_by_key(|position| (position.y, position.x));
            let index = if frontier.len() == 1 {
                0
            } else {
                usize::try_from(self.rng.bounded(
                    u64::try_from(frontier.len()).expect("destroyed frontier count must fit u64"),
                ))
                .expect("destroyed frontier index must fit usize")
            };
            selected.insert(frontier[index]);
        }
        for position in &selected {
            set_generated_terrain(terrain, definition.width, *position, terrain_id);
        }
        selected
    }

    fn generate_streamers(
        &mut self,
        definition: &ProceduralFloorDefinition,
        streamers: &[ProceduralStreamerCandidateDefinition],
        terrain: &mut [String],
    ) -> BTreeSet<Position> {
        const DIRECTIONS: [(i32, i32); 8] = [
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ];
        let budget = definition
            .generation_budget
            .as_ref()
            .expect("streamers require a generation budget");
        let placement_count = budget
            .streamer_placements
            .expect("validated streamer placement count must remain available");
        let area = usize::try_from(
            budget
                .streamer_area_tiles
                .expect("validated streamer area budget must remain available"),
        )
        .expect("streamer area must fit usize");
        let weights = streamers
            .iter()
            .map(|candidate| candidate.weight)
            .collect::<Vec<_>>();
        let mut assignments = BTreeMap::<Position, String>::new();

        for _ in 0..placement_count {
            let streamer_index = if streamers.len() == 1 {
                0
            } else {
                self.roll_weighted_index(&weights)
            };
            let streamer = &streamers[streamer_index];
            let mut starts = Vec::new();
            for y in (definition.height / 3)..=(definition.height * 2 / 3) {
                for x in (definition.width / 3)..=(definition.width * 2 / 3) {
                    let position = Position {
                        x: i32::from(x),
                        y: i32::from(y),
                    };
                    if terrain[generated_terrain_index(definition.width, position)]
                        == definition.wall_terrain_id
                    {
                        starts.push(position);
                    }
                }
            }
            if starts.is_empty() {
                starts = generated_wall_positions(definition, terrain);
            }
            if starts.is_empty() {
                break;
            }
            starts.sort_by_key(|position| (position.y, position.x));
            let start_index = if starts.len() == 1 {
                0
            } else {
                usize::try_from(self.rng.bounded(
                    u64::try_from(starts.len()).expect("streamer start count must fit u64"),
                ))
                .expect("streamer start index must fit usize")
            };
            let direction_index =
                usize::try_from(self.rng.bounded(8)).expect("streamer direction must fit usize");
            let (dx, dy) = DIRECTIONS[direction_index];
            let mut cursor = starts[start_index];
            while cursor.x > 0
                && cursor.y > 0
                && cursor.x + 1 < i32::from(definition.width)
                && cursor.y + 1 < i32::from(definition.height)
            {
                for y in cursor.y - 1..=cursor.y + 1 {
                    for x in cursor.x - 1..=cursor.x + 1 {
                        let position = Position { x, y };
                        if position.x > 0
                            && position.y > 0
                            && position.x + 1 < i32::from(definition.width)
                            && position.y + 1 < i32::from(definition.height)
                            && terrain[generated_terrain_index(definition.width, position)]
                                == definition.wall_terrain_id
                        {
                            assignments
                                .entry(position)
                                .or_insert_with(|| streamer.terrain_id.clone());
                        }
                    }
                }
                cursor.x += dx;
                cursor.y += dy;
            }
        }

        let mut painted = BTreeSet::new();
        while painted.len() < area {
            let mut candidates = assignments
                .iter()
                .filter_map(|(position, terrain_id)| {
                    (!painted.contains(position)
                        && terrain[generated_terrain_index(definition.width, *position)]
                            == definition.wall_terrain_id)
                        .then_some((*position, terrain_id.as_str()))
                })
                .collect::<Vec<_>>();
            candidates.sort_by_key(|(position, _)| (position.y, position.x));
            if candidates.is_empty() {
                let fallback = generated_wall_positions(definition, terrain);
                if fallback.is_empty() {
                    break;
                }
                let index = if fallback.len() == 1 {
                    0
                } else {
                    usize::try_from(
                        self.rng.bounded(
                            u64::try_from(fallback.len())
                                .expect("streamer fallback count must fit u64"),
                        ),
                    )
                    .expect("streamer fallback index must fit usize")
                };
                let position = fallback[index];
                set_generated_terrain(
                    terrain,
                    definition.width,
                    position,
                    &streamers[0].terrain_id,
                );
                painted.insert(position);
                continue;
            }
            let index = if candidates.len() == 1 {
                0
            } else {
                usize::try_from(self.rng.bounded(
                    u64::try_from(candidates.len()).expect("streamer candidate count must fit u64"),
                ))
                .expect("streamer candidate index must fit usize")
            };
            let (position, terrain_id) = candidates[index];
            set_generated_terrain(terrain, definition.width, position, terrain_id);
            painted.insert(position);
        }
        painted
    }
}

fn place_generated_floor_connections(
    definition: &ProceduralFloorDefinition,
    entry_anchor: Position,
    down_stair_anchor: Position,
    fixed_trap_position: Position,
    floor_terrain_id: &str,
    terrain: &mut [String],
    rng: &mut RfbRng,
) -> Result<Vec<FloorConnectionState>, CoreError> {
    let terrain_ref: &[String] = terrain;
    let mut candidates = (1..definition.height - 1)
        .flat_map(|y| {
            (1..definition.width - 1).filter_map(move |x| {
                let position = Position {
                    x: i32::from(x),
                    y: i32::from(y),
                };
                (position != fixed_trap_position
                    && terrain_ref[generated_terrain_index(definition.width, position)]
                        == floor_terrain_id)
                    .then_some(position)
            })
        })
        .collect::<Vec<_>>();
    let (primary_up_id, primary_down_id) = primary_floor_connection_ids(definition);
    let mut ordered_connections = Vec::with_capacity(definition.connections.len());
    for connection_id in [primary_up_id, primary_down_id].into_iter().flatten() {
        ordered_connections.push(
            definition
                .connections
                .iter()
                .find(|connection| connection.id == connection_id)
                .expect("selected primary connection must remain available"),
        );
    }
    ordered_connections.extend(definition.connections.iter().filter(|connection| {
        primary_up_id != Some(connection.id.as_str())
            && primary_down_id != Some(connection.id.as_str())
    }));

    let mut placed = Vec::with_capacity(definition.connections.len());
    for connection in ordered_connections {
        let position = if primary_up_id == Some(connection.id.as_str()) {
            entry_anchor
        } else if primary_down_id == Some(connection.id.as_str()) {
            down_stair_anchor
        } else {
            if candidates.is_empty() {
                return Err(CoreError::InvalidSave(
                    "generated floor has insufficient connection space",
                ));
            }
            let candidate_index = usize::try_from(rng.bounded(candidates.len() as u64))
                .expect("bounded connection index must fit usize");
            candidates[candidate_index]
        };
        candidates.retain(|candidate| *candidate != position);
        set_generated_terrain(terrain, definition.width, position, &connection.terrain_id);
        placed.push(FloorConnectionState {
            id: connection.id.clone(),
            position,
            target_floor_id: None,
            target_connection_id: None,
        });
    }
    placed.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(placed)
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
