# RFB 内容数据格式 v1

状态：P0 源格式、JSON Schema、确定性编译器和首个原创内容包已实现；当前内容包已扩展至 1.204.0

## 1. 目标

怪物、物品、职业、种族、法术、地形、任务和视觉映射不再编译进巨型 C 结构体。内容定义与运行时实例分离，并满足：

- 稳定 ID；
- 可验证 Schema；
- 确定性加载；
- 本地化显示；
- 模组和数据包扩展；
- 存档可以记录精确内容集合；
- Windows、Linux、macOS 和 Android 原生 Rust 核心使用同一份编译后数据。

## 2. 数据包结构

```text
packs/base/
├─ pack.json
├─ abilities/
├─ abilityBooks/
├─ actors/
├─ affixes/
├─ builds/
├─ classes/
├─ encounterTables/
├─ items/
├─ lootTables/
├─ personalities/
├─ races/
├─ regionTables/
├─ resources/
├─ skills/
├─ skillSets/
├─ terrain/
├─ terrainFeatureTables/
├─ themeTables/
├─ vaults/
├─ worlds/
├─ locales/
└─ assets/
```

当前 v1 编译器实现 `abilities`、`abilityBooks`、`actors`、`affixes`、`builds`、`classes`、`encounterTables`、`items`、`lootTables`、`personalities`、`races`、`regionTables`、`resources`、`skills`、`skillSets`、`terrain`、`terrainFeatureTables`、`themeTables`、`vaults` 和 `worlds` 二十个严格类型根。任务和视觉映射会在相同稳定 ID/Schema 规则下增加独立根；扩展包可以只声明自己实际提供的根。

`pack.json`：

```json
{
  "$schema": "https://rfb.example/schema/pack-v1.json",
  "formatVersion": 1,
  "id": "rfb.base",
  "version": "1.0.0",
  "titleKey": "pack-rfb-base-title",
  "dependencies": [],
  "loadAfter": [],
  "contentRoots": ["monsters", "items", "spells"]
}
```

源文件使用 UTF-8 JSON 和 JSON Schema。开发工具可以提供 JSONC 编辑体验，但进入构建和发布产物前必须转换为严格 JSON。

## 3. 稳定 ID

ID 格式：`namespace.category.name`，仅允许小写 ASCII、数字、点、下划线和短横线。

示例：

```text
rfb.monster.dragon.red
rfb.item.weapon.long_sword
rfb.spell.fire.fire_ball
rfb.terrain.wall.granite
```

规则：

- ID 是逻辑身份，名称由 Fluent key 提供；
- 已发布 ID 不得复用；
- 改名必须进入 alias/migration 表；
- 运行时实例引用定义 ID，不复制完整定义；
- 数组下标、英文显示名和中文译名都不能充当引用。

## 4. 定义与实例

内容定义描述固定规则：

```json
{
  "$schema": "https://rfb.example/schema/monster-v1.json",
  "id": "rfb.monster.dragon.red",
  "nameKey": "monster-red-dragon-name",
  "descriptionKey": "monster-red-dragon-description",
  "level": 60,
  "tags": ["dragon", "fire"],
  "stats": {},
  "abilities": []
}
```

运行时实例只保存定义 ID、实例 ID和动态状态。内容文件不能包含平台路径、Rust 枚举序号或图集坐标。

## 5. 验证与编译

构建工具 `rfb-contentc` 负责：

1. 解析严格 JSON；
2. 验证 Schema；
3. 检查重复 ID、悬空引用和依赖循环；
4. 检查数值范围和互斥字段；
5. 检查本地化 key；
6. 按稳定规则合并数据包；
7. 按 ID 排序并生成规范化内容；
8. 输出 MessagePack 内容包和 SHA-256 content hash；
9. 生成 Rust/TypeScript 开发期索引和审计报告。

`inspect-source` 只编译并显示规范化摘要/hash，不读取或改写 lock；修改内容后先审查该输出，再显式更新 `content.lock.json`。`verify-source` 则要求源内容与已提交 lock 完全一致。

一次性任务层使用 `taskObjective`、`taskReward` 和入口结果 terrain 描述任务闭环。contract-v40 新增 `allowEarlyTaskExit`（默认 `true`）与必需的 `abandonedEntryTerrainId`：前者决定未完成时普通上楼是否允许，后者保存显式放弃后的权威地表结果。contract-v45 新增 `taskStages`：一个共享任务只由一个成员楼层声明有序阶段，每阶段绑定成员 `floorId`，支持收集、进入楼层、单实例击杀和按种类计数击杀；旧 `taskObjective` 继续表示单阶段任务。

contract-v46 为普通地牢楼层增加 `dungeonId`、`finalFloor` 和 `guardian`。同一 dungeon ID 的楼层必须形成从深度 1 到唯一最终层的完整线性链；非最终层必须有 `nextFloorId/downStairTerrainId`，最终层必须没有下行连接并声明唯一守护者实例与 actor kind。

contract-v47 新增独立 `VaultDefinition`：模板声明稳定主题 ID、尺寸、基础 terrain、入口、terrain 覆盖、群体成员位置、带深度范围的加权 actor 候选和 loot table 引用。程序化楼层通过 `themeId/vaultId` 引用模板；编译器保证主题一致、位置可行走且当前楼层深度至少有一个合法 encounter 候选。

contract-v48 新增独立 `EncounterTableDefinition` 与 `ThemeTableDefinition`。encounter 表声明每层 roll 数和带权重/深度范围的怪物候选；theme 表声明主题、楼层 terrain 和带独立权重/深度范围的 Vault 候选。程序化楼层通过 `encounterTableId/lootTableId/themeTableId` 引用这些表，并可用 `nest { roomId, spawnCount }` 生成一次选种的同类群体。编译器拒绝新表引用与旧内联字段混用，并验证深度可用性、主题一致、怪物角色/等级、Vault 尺寸与巢穴房间。

contract-v49 为普通 dungeon 程序化楼层增加可选 `generationBudget { actorSlots, lootPlacements }`。actorSlots 计入巢穴、Vault encounter group、当前守护者和普通 encounter；lootPlacements 计入 Vault loot spawn 和重复楼层 loot table 位置。启用预算必须同时引用 encounter/loot 表，编译器验证范围并确保每条深度合格 Vault 路径预留固定成员后仍有一个普通 encounter 与一个普通 loot placement。

contract-v50 为 Vault 增加可选 `transforms`，允许 identity、四向旋转和四种镜像，入口可位于任一模板边界。楼层预算可成对增加 `vaultPlacements/vaultAreaTiles`，启用后按数量、面积、actor 和 loot 预算选择多个不同 Vault，在地图未开凿 wall 区自由落位；矩形重叠被拒绝，无可行原点的候选稳定移除并继续回退。

contract-v51 为 encounter 条目增加可选 `group`：`friends` 生成同种类成员，`escort` 从独立加权/深度候选中逐个选种，`formation` 支持 `cluster/ring`。楼层预算可成对增加 `groupPlacements/groupActorSlots`；随从同时消耗群体随从预算和 actor 总预算，领袖只消耗 actor 总预算。空间不足时先缩减 escort、再缩减 friends；最小阵容无法放置则原子放弃该群体并回退到其他群体或普通 encounter。

contract-v52 新增独立 `TerrainFeatureTableDefinition`。条目按深度和权重引用 trap、可挖掘障碍或可开启门，并以 `room/corridor` 限定可放置空间。程序化楼层通过 `terrainFeatureTableId` 引用表，并在 `generationBudget.featurePlacements` 声明额外 feature 数量；固定拓扑门/陷阱不计入预算。生成器避开楼梯、固定 feature 和 Vault 矩形，空间失败时移除当前候选并稳定回退，成功位置同时排斥后续 terrain、actor 与 loot。

contract-v53 为程序化楼层增加可选 `layout`。`layout.rooms` 声明房间尺寸范围和加权 `rectangle/cross` 形状，`layout.cavern` 可引用独立可行走 terrain；`generationBudget` 成对增加 `roomPlacements/roomAreaTiles`，并以 `cavernAreaTiles` 预算基础洞穴地貌。编译器验证尺寸、形状唯一性、权重、分区可容纳性、面积下限和 terrain 引用。运行时先生成精确预算、四向连通的 cavern，再落位并串联精确数量的房间；普通 encounter 与 loot 在非入口房间间稳定轮转。

contract-v54 在 `layout` 增加可选 `lake/river`，两者都引用独立的深浅 terrain 对。`lakeAreaTiles/lakeDeepAreaTiles` 成对预算湖泊总面积和连通深水核心，`riverAreaTiles` 预算从内部边界连到湖心/地图中心的深水中心线与浅水岸。深水必须不可行走、浅水必须可行走；同层 lake/river 必须使用相同材质对。编译器同时验证引用、配对、面积边界和最坏中心线容量。

contract-v55 在 `layout` 增加可选 `maze/destroyed/streamers`。maze 使用奇数宽高和严格派生的 `mazeFloorTiles`；destroyed 以独立 terrain、震中数和总影响面积成组声明；streamers 提供规范排序的加权 terrain 候选，并以条数和总面积预算。编译器验证迷宫公式、尺寸、引用、terrain 互斥、可行走性、候选唯一性、权重和面积边界。

contract-v56 在 `layout` 增加可选 `pit`，声明独立 encounter table、奇数内室宽高和 roster 大小；`pitPlacements/pitActorSlots` 成对声明数量与密集内室 actor 预算。编译器验证表引用、深度候选、尺寸、内室面积、地图容纳、总 actor 预算，并禁止与 legacy nest、动态 group 和空间 Vault 同层组合。

contract-v57 为 `layout` 增加默认 `rooms` 和显式 `maze-only` 模式，并将 `rooms` 几何改为可选。rooms 模式必须声明房间数量/面积预算且不能叠加 maze；maze-only 必须声明 maze 与严格匹配的 `mazeFloorTiles`，禁止房间、洞窟、水系、毁坏区、pit、Vault、nest、动态 group 和 terrain feature，只允许在专用 maze 后继续 streamer 阶段。运行时 encounter/loot 使用连通 maze 区域候选而非伪造 room ID。

contract-v58 为程序化楼层增加 `entryConnectionId` 与 `connections`。每个连接声明稳定 ID、`stairs/shaft` 类型、terrain、目标楼层和非地表目标连接。连接必须双向互引、方向标签匹配、保持相同 lifecycle/dungeon；stairs 深度差为 1，shaft 为 2。根楼层的入口连接必须返回地表。第一组主 up/down 使用既有锚点，附加连接在 Vault 后按种子 RNG 随机落位。

contract-v59 为 `EncounterGroupDefinition` 增加可选 `packAi`，分别声明 leader、friends 和 escorts 的 `seek`、`surround` 或 `guard-leader` 行为；缺失时默认使用 seek/surround/guard-leader。生成器把冻结后的行为写入 actor pack 身份，因此载入存档不重新解释后来修改的内容默认值。

contract-v60 新增独立 `RegionTableDefinition`。候选通过稳定 `regionId` 引用明确 `themeTableId/themeId` 和局部 `encounterTableId/lootTableId`，并声明权重与深度范围。程序化楼层通过 `regionTableId` 和 `generationBudget.regionPlacements` 启用加权无放回区域选择；房间按中心距离完整归属区域，区域主题只绘制所属房间，走廊保留楼层基础 terrain。首版禁止与 Vault、pit/nest、动态群体、terrain feature、分阶段地貌、maze-only 和显式多连接组合。

contract-v42 新增 `retakeable`（默认 `false`）。启用后，未完成时普通离开会保存任务层并保持入口开放；重新进入恢复原楼层，而显式放弃或完成仍关闭入口。

contract-v61 为可重接任务增加可选 `maxRetakes`（1–16）和 `retakeFloorPolicy`。默认 `preserve-floor` 恢复完整保存层；`regenerate-floor` 保留任务阶段/进度，在成功重入时丢弃共享任务的旧成员层并重新生成当前成员层，只补计数目标的剩余数量。相同 `taskId` 的所有成员必须声明一致策略；非可重接任务不能声明这些字段。

contract-v62 允许 `regionTableId` 与 `themeTableId`、`terrainFeatureTableId`、connections、guardian/final、pit、cavern/lake/river/destroyed/streamer 和动态群体组合。区域普通 encounter/loot 继续读取 RegionEntry 的局部表；特殊 footprint 归入宿主区域，生成预算先预留 Vault/pit/guardian/group，再分配区域普通槽位。区域楼层仍不能混用楼层级 encounter/loot、旧内联 spawn、nest 或 maze-only；group budget 启用时每个区域表必须同时保留普通与 grouped 深度候选。

contract-v63 为世界新增 `dungeons`，每项以稳定 ID 引用 `rootFloorId` 和共享 `guardianActorKindId`。显式连接必须在每座地牢内形成单根树：根层深度为 1，每个非根层只有一个父边，stairs/shaft 分别跨 1/2 层，禁止合流、循环、同层和跨地牢边。所有叶层都必须是带 guardian 的程序化最终层，并使用共享 actor kind 和互不相同的镜像 instance ID；非叶层不能声明 final/guardian。旧 `nextFloorId/returnFloorId` 继续用于无显式连接和历史 terrain 标签回退。

contract-v64 将 Vault 的规范入口字段升级为 `entrancePositions`：允许 1–8 个互不重复的边界位置，旧 `entrancePosition` 只在列表缺失时规范化为单元素列表。入口必须可连接；加载器展开 base terrain 与 overrides 后，以固定四向 BFS 证明全部潜在可通行格和全部入口属于同一分量。运行时还会为每个变换后入口寻找最长 12 格的确定性 wall connector，并只提交能让整层潜在可通行格保持单一分量的候选。失败候选继续走稳定回退，不写入部分 terrain。

contract-v71 新增 `skills`、`skillSets`、`races`、`classes`、`personalities` 和 `builds` 六个独立根。技能集合按 `base` 与 `growthPerTenLevels` 聚合；Race/Class/Personality 分别提供属性、生命/经验倍率、基础 HP、技能集合和出生物品；build 绑定三者并声明出生自然属性。世界可选 `playerBuildId`，缺省时 demo 使用 Explorer。编译器验证技能最大值、技能集合引用、来源范围、出生装备堆叠/槽位和构筑组合的总容量，并把所有根按稳定 ID 排序后写入 content hash。

contract-v72 为既有定义增加可选技能消费字段：terrain 的 `perceptionCheckDifficulty`、trap 的 `savingThrowDifficulty`、item `useAction.deviceCheckDifficulty`，以及 actor 的 `awareness.detectionDifficulty/detectionRange/startsAlerted`。编译器验证 difficulty/range 的正整数边界、trap/useAction 字段组合，并要求每种运行时检定都能解析唯一对应 `SkillKind`。缺字段内容保持历史行为。

contract-v73 新增 `resources`、`abilities` 和 `abilityBooks` 三个独立根。能力声明等级、资源成本、基础失败率、`TargetSpec` 与首个伤害效果；能力书引用稳定能力 ID；item 可用 `abilityBookId` 绑定实体书本；Class 可用 `castingProfile` 声明资源、施法属性、容量公式、最低失败率和支持书本。编译器验证书本物品不可堆叠/装备/普通使用、所有引用与资源一致、能力等级/成本/范围/效果骰边界，以及职业能力书只能包含使用其指定资源的能力。

contract-v74 为资源增加带默认值的 `waitRecoveryAmount` 与 `restRecoveryAmount`，并把 `self` 纳入稳定目标模式、把 `heal { amount }` 纳入能力效果。恢复量必须为非负整数；自身目标必须使用零射程且不要求视线，治疗量必须为正数。缺少恢复字段的旧内容按 0 处理，不会隐式获得恢复能力。

contract-v75 为 `AbilityDefinition` 增加必需的 `proficiency.initial/cap/successGain/failureGain`，以及可选 `cooldown.turns/groupId`。熟练度初值不能超过上限，上限不超过 RFB 的 1600 档；增量必须为非负整数。冷却回合必须为正数，组 ID 必须是稳定非空标识。普通能力可以省略冷却，表示无冷却；能力的实际熟练度和冷却进度属于存档状态而不是内容定义。

contract-v172 将 Ability Program 与玩家施法策略完全分离。每个能力必须声明 Program，但 `playerAbilityBindings` 只在玩家入口需要时提供等级、资源/成本、失败率、熟练度和冷却。缺少绑定不表示能力永久属于怪物；未来能力书、职业先天技、种族或怪物模式要授予玩家时再添加绑定。绑定本身不授予可用性，actor 的 `monsterCasting` 也不依赖该绑定。编译器拒绝未知、重复或参数非法的绑定，并拒绝任何引用了无绑定能力的玩家入口。

contract-v76 为 Class `castingProfile` 增加独立学习容量字段：`baseLearningCapacity`、`learningCapacityPerLevel`、`learningCapacityPerAttributeIndex` 与 `learningCapacityCap`。容量由等级和施法属性桶纯函数派生，与 Mana 容量分离；`ForgetAbility` 只移除已学集合并保留能力进度，容量满与遗忘前置拒绝均不抽能力 RNG。缺字段的旧存档按当前内容初值迁移，已学数量超过容量时原子拒绝。

contract-v77 为 `AbilityEffectDefinition` 增加 `area-damage`，声明伤害骰、伤害类型与 1–9 的 `radius`。范围能力沿用 `TargetSpec` 的射程/视线验证，并在 DTO 中输出可选 `areaRadius`；定点与方向投射的停止策略、RFB 距离衰减和墙体遮挡由核心固定，内容包只声明效果参数。当前 demo 的 `Echo Burst` 使用 2d4 electricity、半径 2、射程 6。范围效果不增加 save 字段，能力进度继续由 `abilityProgress` 承载。

contract-v78 为 `AbilityEffectDefinition` 增加 `beam-damage`，声明伤害骰与伤害类型；首版要求 `TargetSpec.modes` 仅为 `direction`，DTO 输出可选 `beamDamage`。核心固定 RFB `fire_beam()` 式逐格推进、穿透 actor、墙体/边界截断、近到远顺序和共享一次基础伤害骰。当前 demo 的 `Echo Lance` 使用 2d4 electricity、方向射程 6。射线效果不增加 save 字段，能力进度继续由 `abilityProgress` 承载。

contract-v79 为 `AbilityEffectDefinition` 增加 `cone-damage`，声明伤害骰、伤害类型和 1–9 半径；首版要求 `TargetSpec.modes` 仅为 `direction`，DTO 输出可选 `coneRadius`。核心固定八向中心线逐层展开、穿透 actor、墙体/边界截断、近到远与横向距离稳定顺序、整数侧向衰减和共享一次基础伤害骰。当前 demo 的 `Echo Fan` 使用 2d4 electricity、方向射程 6、半径 2。锥形效果不增加 save 字段，能力进度继续由 `abilityProgress` 承载。

contract-v80 扩展既有 `beam-damage` 的目标模式：`direction` 保持 v78 的方向射线，`position` 与 `entity` 使用稳定整数斜率经过目标后继续到内容射程上限；DTO 继续复用 `targetSpec`，不增加新的伤害参数或存档字段。目标必须存在、可见且不超距；actor 不阻挡，墙体/不可行走地形/边界截断，路径按近到远稳定结算并共享一次基础伤害骰。当前 demo 的 `Echo Lance` 使用 2d4 electricity、射程 6，并接受方向、格子和实体目标。

contract-v81 为 `AbilityEffectDefinition` 增加无参数的 `teleport`；编译器要求首版只声明单一 `position` 目标、1–64 格射程和 `requiresLineOfEffect`。DTO 通过 `AbilityDto.teleport` 投影效果。核心要求落点非当前格、在地图内、当前可见、满足 line of effect、可行走且无存活 actor 占据；当前 demo 的 `Echo Step` 使用 6 格射程、4 Mana 和 25% 初始失败率，并收入 Echo Primer。位移效果不增加 save 字段，能力进度继续由 `abilityProgress` 承载。

contract-v82 为 `AbilityEffectDefinition` 增加 `summon { actorKindId, count, radius, durationTurns }`。编译器要求目标 actor 为 Monster，数量与半径均为 1–8，生命周期为 1–10,000，并强制单一 `self` 目标、零射程且不要求 line of effect。DTO 通过 `AbilityDto.summon` 投影召唤规格；当前 demo 的 `Echo Companion` 召唤 2 个 `demo.actor.echo-companion`，半径 2、生命周期 5 回合、6 Mana、20% 基础失败率，并收入 Echo Primer。召唤 actor 的所有者、源能力和剩余回合属于运行时/save 状态，不重复写入内容定义。

contract-v83 为 `AbilityEffectDefinition` 增加 `detect { category, radius, persistent }`。编译器要求 category 匹配现有 terrain tag、半径为 1–8，并强制单一 `self` 目标、零射程且不要求 line of effect。DTO 通过 `AbilityDto.detect` 投影侦测规格；当前 demo 的 `Echo Pulse` 侦测半径 4 内的 `perception-cue` 并只返回瞬时位置，`Echo Sight` 侦测半径 6 内的 `hidden` 并写入持久 terrain 知识。两者均收入 Echo Primer；FOV、隐藏投影筛选、稳定顺序和知识写入由核心定义。

contract-v84 为 `AbilityEffectDefinition` 增加 `transform-terrain { sourceTerrainIds, targetTerrainId, radius }`。编译器把 1–32 个来源 terrain ID 稳定排序并拒绝重复、缺失引用、来源等于目标和 0–8 以外半径，同时强制单一 `position` 目标、1–64 射程和 line of effect。DTO 通过 `AbilityDto.terrainTransform` 投影转换规格；Echo Delving 把明确岩壁/瓦砾集合转换为普通地面，Echo Rampart 把明确地面集合转换为回声瓦砾。FOV、占用格、连接/边界保护、稳定原子提交和 `changedCells` 由核心定义。

contract-v85 为 `AbilityEffectDefinition` 增加 `apply-status`、`remove-status` 和含 2–8 个子效果的 `sequence`。旧单一 `effect` 对象继续直接读取；sequence 禁止嵌套，并在首版限制为同一 actor 目标上的 damage/heal/status 组合。状态添加声明稳定 kind ID、强度、持续 tick、replace/extend/keep-strongest 与可选抗性类型。编译器拒绝零持续时间、零强度、混合 self/投影语义和 sequence 中的世界/多目标效果。DTO 通过 `AbilityDto.effects` 按声明顺序投影扁平规格。

contract-v86 为 Monster actor 增加可选 `monsterCasting`：`frequencyPercent` 使用 1–100 百分比，`abilities` 使用 1–32 个唯一能力 ID 与正权重。50% 等价于 1 in 2，25% 等价于 1 in 4；成功施法后的运行时冷却由核心按 `ceil(100 / frequencyPercent)` 计算。编译器拒绝玩家 actor、重复或缺失能力、零权重，以及首版怪物执行器不支持的 self/世界/多目标效果；首版只接受直接 actor 目标的 damage/status/sequence。

contract-v87 扩展 `monsterCasting` 可引用的能力子集：允许 self 目标的 heal/status/sequence/summon、实体目标的 area/beam、方向目标的 cone，并继续拒绝 teleport、detect、terrain transform 和混合目标语义。内容权重仍是基础概率；HP/状态/距离效用、footprint 风险、召唤空间和拒绝原因由核心纯计算。怪物召唤与玩家召唤复用同一个 `summon` 内容效果，运行时 owner 决定阵营。

contract-v43 新增可选 `taskId`。相同 task ID 的任务层组成一个结算组，共享进度与结果；组内目标种类、required 和重接策略必须一致，并且整组恰好声明一个奖励。`kill-actor-kind` 可用 `spawnCount` 控制单个成员楼层生成的目标数量。

当前已完成第 1、2、3、7、8 项的单包版本，包括：

- `deny_unknown_fields` 严格 JSON 解析；
- 单文件 1 MiB、单包 16 MiB、最多 32768 文件的输入上限；
- 禁止内容目录和文件符号链接；
- 稳定 ID、语义版本、消息 key、glyph、tag 和数值范围检查；
- 世界中的地形、角色与物品悬空引用检查；
- 定义、tag、spawn 和地形覆盖的规范化排序；
- `RFBCONT\0`、MessagePack payload、长度和 SHA-256 校验；
- `content.lock.json` 固定包 ID、版本和编译 content hash；
- 二十一份提交到 `schemas/content-v1/` 的 JSON Schema。

角色定义使用必需的基础战斗字段；玩家可声明携带容量与门/搜索技能，怪物可声明 melee routine、出生携带与死亡掉落、awareness，以及 `monsterCasting` 的百分比频率、加权能力集合、smart、偏好距离和撤退阈值。物品、资源、能力、能力书、affix、encounter/loot/theme/region/terrain-feature 表、Vault 和 world 使用独立稳定 ID 与交叉引用；编译器验证目标存在、角色类别、范围、数量、权重和互斥旧字段。原创包 1.80.0 覆盖角色成长与构筑、玩家能力循环、怪物 caster 效用/阵营目标/多格结算/战术移动/有限记忆、固定词条与鉴别，以及楼层/任务/树状地牢/Vault/区域主题/群体/分阶段地貌等现有纵切。

contract-v106 扩展能力内容词汇：`apply-status` 可声明基础时长加骰时长、属性修正、装备加值和状态免疫；`random-choice` 使用有序 `maximumRoll` 阈值、等级加值除数和分支目标；`visible-damage` 对当前可见目标共享伤害骰；`enchant-equipped-weapon` 引用稳定 affix；`drain-life.repeat` 声明重复追踪次数；`summon.hostile` 区分敌对固定召唤；`no-op.reason` 为尚无通用系统的具名分支保留可观察缺口。等级缩放增加 `linear`/`prorated` 曲线以及状态/装备加值字段。编译器验证阈值覆盖、目标语义、引用、骰值和边界；demo 包 1.97.0 含 60 abilities、4 ability books、18 items 和 4 affixes，当前 state hash 为 Schema v45。完整边界见 [Contract v106](contract-v106-death-third-book.md)。

contract-v107 增加 `item` 目标、`death-ray`、`identify-item`、`restore-vitality` 与 nearby `genocide`；`summon-category` 可声明升级类别、等级门、敌友/群体概率、群体骰和敌对 unique，`apply-status` 可引用临时 Race 并授予穿墙/入伤比例。编译器验证 Race/类别引用、概率组合、等级门、power/radius 和入伤范围，并要求零基础的新增缩放字段存在匹配 `levelScaling`。demo 包 1.98.0 含 68 abilities、5 ability books、19 items、4 affixes、28 actors、4 races 和 13 skill sets，当前 state hash 为 Schema v46。完整边界见 [Contract v107](contract-v107-death-fourth-book.md)。

contract-v108 为物品 `useAction` 增加 `heal-dice` 和可选 `charges { initial, maximum, cost }`。治疗骰限制为 1–100 骰、1–10000 面；充能容量限制为 1–1000000，initial 不得超过 maximum，cost 必须为正且不超过 maximum。带充能动作的物品必须 `maxStack: 1`、带 `device` 标签并声明合法设备检定难度。demo 包 1.99.0 含 68 abilities、5 ability books、20 items、4 affixes、28 actors、4 races 和 13 skill sets，当前 state hash 为 Schema v47。完整边界见 [Contract v108](contract-v108-charged-items.md)。

contract-v109 为物品增加互斥于 `useAction` 的 `deviceGeneration.activations`。每个候选声明稳定 ID、名称键、权重、1–100 深度范围、设备难度、随机容量区间/成本、目标规格和 heal/damage/detect 效果；编译器验证 ID 唯一、全深度覆盖、容量与成本、效果目标匹配以及 detect category 在内容 tag 中存在。demo 包 1.100.0 含 68 abilities、5 ability books、23 items、4 affixes、28 actors、4 races 和 13 skill sets，当前 state hash 为 Schema v48。完整边界见 [Contract v109](contract-v109-dynamic-devices.md)。

contract-v110 为 `deviceGeneration` 增加可选 `recovery { intervalTicks, energyPerMille }`，并为 Class 增加可选 `deviceRechargeProfile`（资源、主宰属性、上限公式、power 和设备来源损毁率）。编译器限制恢复间隔为 1–10000 tick、千分比为 1–1000，并验证 recharge 资源引用、上限参数、power 与损毁率。demo 包 1.101.0 新增 Resonance 资源，现含 68 abilities、5 ability books、23 items、4 affixes、3 resources、28 actors、4 races 和 13 skill sets，当前 state hash 为 Schema v49。完整边界见 [Contract v110](contract-v110-device-recharge.md)。

contract-v111 为物品效果增加 `remove-status`、`restore-resource`、`restore-resource-dice`、`restore-resource-full` 和 `sequence`。序列限制为 2–8 个非嵌套、自目标恢复步骤；编译器验证资源引用、状态 ID、固定值/骰值边界，并拒绝 damage/detect 或嵌套 sequence。demo 包 1.102.0 新增两种恢复药水，现含 68 abilities、5 ability books、25 items、4 affixes、3 resources、28 actors、4 races 和 13 skill sets，state hash 保持 Schema v49。完整边界见 [Contract v111](contract-v111-restorative-items.md)。

contract-v112 为物品效果增加 `identify-item { full }`。固定 `useAction` 和动态 activation 都必须声明 item-only 目标；恢复型 sequence 拒绝嵌套鉴定。demo 包 1.103.0 新增普通/完全鉴定卷轴，现含 68 abilities、5 ability books、27 items、4 affixes、3 resources、28 actors、4 races 和 13 skill sets，state hash 保持 Schema v49。完整边界见 [Contract v112](contract-v112-scroll-identification.md)。

contract-v113 为侦测主体增加 `item`，并为物品 `detect` 增加默认关闭的 `throughWalls`。地图效果持久写入已探索格，隐藏地形侦测持久写入揭示集合；actor/item 结果只通过事件返回稳定实例 ID 与位置，不进入存档。demo 包 1.104.0 新增三种地图/侦测卷轴，现含 68 abilities、5 ability books、30 items、4 affixes、3 resources、28 actors、4 races 和 13 skill sets，state hash 保持 Schema v49。完整边界见 [Contract v113](contract-v113-scroll-detection.md)。

contract-v114 为物品效果增加 `random-teleport { maximumDistance }`、`teleport-level`、`recall { delayDice, delaySides, delayBonus }` 和 `reset-recall`。四种效果都是 self-only；编译器验证距离 1–200、延迟骰和目标模式。跨层与召回目标由 world/dungeon/floor 的稳定引用和运行时树连接解析，不把实例 ID 写入内容定义。demo 包 1.105.0 新增五种传送/召回卷轴，现含 68 abilities、5 ability books、35 items、4 affixes、3 resources、28 actors、4 races 和 13 skill sets，state hash 升至 Schema v50。完整边界见 [Contract v114](contract-v114-scroll-travel-recall.md)。

contract-v115 为物品效果增加 `enchant-item`，使用可选 `toHit`、`toDamage`、`toArmor` 骰定义声明各属性尝试次数。编译器要求至少一个分支、禁止武器/护甲分支混用，并把效果限制为 item-only。demo 包 1.106.0 新增五种附魔卷轴和 Resonance Mail，现含 68 abilities、5 ability books、41 items、4 affixes、3 resources、28 actors、4 races 和 13 skill sets；实例强化进入存档和 state hash Schema v51。完整边界见 [Contract v115](contract-v115-scroll-enchantment.md)。

contract-v116 为物品效果增加 self-only 的 `curse-equipped-item { target }` 与 `remove-equipped-curses { includeHeavy }`，并为可装备物品增加生成期 `initialCurse`。严重度固定为 normal/heavy/permanent；无装备槽物品禁止声明初始诅咒。demo 包 1.107.0 新增四种诅咒/解除卷轴及三件边界装备，现含 68 abilities、5 ability books、48 items、4 affixes、3 resources、28 actors、4 races 和 13 skill sets；实例诅咒进入存档和 state hash Schema v52。完整边界见 [Contract v116](contract-v116-scroll-curses.md)。

contract-v117 为 Race 增加可选 `kinCategory`，并为物品效果增加 self-only 的 `summon-category`。selector 可选择任意怪物、显式 actor category 或当前有效 Race 的 kin category；最高等级来源可选择地牢深度或玩家等级，数量/群体/敌对/unique/半径均显式声明。物品召唤首版固定 `durationTurns: 0`，永久友方由运行时保存 `controllerId`。demo 包 1.108.0 新增四种召唤卷轴，现含 68 abilities、5 ability books、52 items、4 affixes、3 resources、28 actors、4 races 和 13 skill sets；state hash 保持 Schema v52。完整边界见 [Contract v117](contract-v117-scroll-summoning.md)。

contract-v132 为 Class 增加默认 false 的 `usesSpellScrolls`，并为静态消耗品增加无参数 `increase-spell-learning-capacity`。该效果不能作为动态 activation，也不开放 amount；职业资格与学习容量 bonus 由核心解释。demo 包 1.123.0 新增 Spell Scroll，现含 68 abilities、5 ability books、69 items、4 affixes、3 resources、28 actors、4 races 和 13 skill sets；bonus 进入 state hash Schema v54。完整边界见 [Contract v132](contract-v132-scroll-spell.md)。

contract-v133 为静态消耗品增加 self-only 的 `apply-slowness { durationDice, durationSides, durationBonus }`。持续时间骰由核心在消费后结算并以 KeepStrongest 合并 `rfb.status.slow`；动态 activation、充能和设备检定不允许使用该效果。demo 包 1.124.0 新增 Slowness Potion，现含 68 abilities、5 ability books、70 items、4 affixes、3 resources、28 actors、4 races 和 13 skill sets；state hash 保持 Schema v54。完整边界见 [Contract v133](contract-v133-potion-slowness.md)。

contract-v134 为静态消耗品增加 self-only 的 `self-life-loss { amount }`。核心直接扣除固定生命，不经过护甲、抗性或 incoming-damage 缩放；动态 activation、充能和设备检定不允许使用该效果。demo 包 1.125.0 新增原创 Mortal Draught，现含 68 abilities、5 ability books、71 items、4 affixes、3 resources、28 actors、4 races 和 13 skill sets；state hash 保持 Schema v54。完整边界见 [Contract v134](contract-v134-potion-death.md)。

contract-v135 为静态消耗品增加 self-only 的 `apply-poison { durationDice, durationSides, durationBonus }`。核心先用既有 Poison 抗性档数值完成固定 `bounded(55)` 阈值检定，只有失败才抽持续时间并以 Extend 合并 Poison；动态 activation、充能和设备检定不允许使用该效果。demo 包 1.126.0 新增原创 Venom Draught，现含 68 abilities、5 ability books、72 items、4 affixes、3 resources、28 actors、4 races 和 13 skill sets；state hash 保持 Schema v54。完整边界见 [Contract v135](contract-v135-potion-poison.md)。

contract-v136 为静态消耗品增加 self-only 的 `apply-thermal-resistance { durationDice, durationSides, durationBonus }`。核心只抽一次持续时间，以 Extend 应用单一 Thermal 状态并同时授予 Fire/Cold Resistant；动态 activation、充能和设备检定不允许使用该效果。demo 包 1.127.0 新增原创 Temperate Tonic，现含 68 abilities、5 ability books、73 items、4 affixes、3 resources、28 actors、4 races 和 13 skill sets；state hash 保持 Schema v54。完整边界见 [Contract v136](contract-v136-potion-thermal-resistance.md)。

contract-v137 为静态消耗品增加 self-only 的 `apply-basic-resistance { durationDice, durationSides, durationBonus }`。核心每次只抽一次持续时间，以 KeepStrongest 应用单一 Basic Resistance 状态并同时授予 Acid/Electricity/Fire/Cold/Poison Resistant；合法使用无条件识别，动态 activation、充能和设备检定不允许使用该效果。demo 包 1.128.0 新增原创 Prismatic Elixir，现含 68 abilities、5 ability books、74 items、4 affixes、3 resources、28 actors、4 races 和 13 skill sets；state hash 保持 Schema v54。完整边界见 [Contract v137](contract-v137-potion-basic-resistance.md)。

contract-v138 为静态消耗品增加 self-only 的 `apply-speed { durationDice, durationSides, durationBonus }`。核心在没有 Haste 时抽取初始持续时间并识别，已有 Haste 时零 RNG、固定延长 5 ticks；动态 activation、充能和设备检定不允许使用该效果。demo 包 1.129.0 新增原创 Swiftstep Tonic，现含 68 abilities、5 ability books、75 items、4 affixes、3 resources、28 actors、4 races 和 13 skill sets；state hash 保持 Schema v54。完整边界见 [Contract v138](contract-v138-potion-speed.md)。

contract-v139 为静态消耗品增加 self-only 的 `apply-heroism { durationDice, durationSides, durationBonus }`。核心每次抽取持续时间并以 Extend 应用既有 Hero 状态，授予 max HP、melee/ranged skill 与 Fear 免疫；只有首次新增状态才识别。动态 activation、充能和设备检定不允许使用该效果。demo 包 1.130.0 新增原创 Valor Tonic，现含 68 abilities、5 ability books、76 items、4 affixes、3 resources、28 actors、4 races 和 13 skill sets；state hash 保持 Schema v54。完整边界见 [Contract v139](contract-v139-potion-heroism.md)。

contract-v140 为静态消耗品增加 self-only 的 `apply-berserk-strength { durationDice, durationSides, durationBonus }`。核心先按内容骰 Extend 既有 Berserk，再固定治疗 30；首次新增状态或实际治疗任一成立即识别。动态 activation、充能和设备检定不允许使用该效果。demo 包 1.131.0 新增原创 Fury Draught，现含 68 abilities、5 ability books、77 items、4 affixes、3 resources、28 actors、4 races 和 13 skill sets；state hash 保持 Schema v54。完整边界见 [Contract v140](contract-v140-potion-berserk-strength.md)。

contract-v141 为静态消耗品增加 self-only 的 `apply-poetic-inspiration { durationDice, durationSides, durationBonus }`。核心每次按内容骰 Extend Poetic Inspiration，状态通过既有修正字段授予 Wisdom/Charisma 各 +5；首次新增才识别。动态 activation、充能和设备检定不允许使用该效果。demo 包 1.132.0 新增原创 Muse Tonic，现含 68 abilities、5 ability books、78 items、4 affixes、3 resources、28 actors、4 races 和 13 skill sets；state hash 保持 Schema v54。完整边界见 [Contract v141](contract-v141-potion-poetic-inspiration.md)。

contract-v142 为静态消耗品增加 self-only 的 `apply-stone-skin { durationDice, durationSides, durationBonus }`。核心每次按内容骰以 KeepStrongest 应用 Stone Skin，按饮用时等级通过既有状态修正授予 defense；首次新增才识别。动态 activation、充能和设备检定不允许使用该效果。demo 包 1.133.0 新增原创 Granite Tonic，现含 68 abilities、5 ability books、79 items、4 affixes、3 resources、28 actors、4 races 和 13 skill sets；state hash 保持 Schema v54。完整边界见 [Contract v142](contract-v142-potion-stone-skin.md)。

contract-v143 为静态消耗品增加 self-only 的 `restore-life-levels { lifeForceAmount }`。核心按固定顺序恢复历史最高经验、重算等级，再增加生命力并封顶 1000；任一实际变化才识别，效果不抽 RNG。动态 activation、充能和设备检定不允许使用该效果。demo 包 1.134.0 新增原创 Renewal Tonic，现含 68 abilities、5 ability books、80 items、4 affixes、3 resources、28 actors、4 races 和 13 skill sets；state hash 保持 Schema v54。完整边界见 [Contract v143](contract-v143-potion-restore-life-levels.md)。

多包拓扑排序、patch、locale 完整性和开发期索引仍待后续实现。

contract-v79 以 1.71.0 增加固定八向 `cone-damage` 能力效果和 Echo Fan；锥形半径、伤害参数与目标模式继续由内容定义，能力进度仍由 `abilityProgress` 保存，当前 state hash 为 Schema v34。

contract-v80 以 1.72.0 扩展 Echo Lance 的 `beam-damage` 目标模式为方向、格子和实体，并固定定点/实体目标穿过目标后的延长路径；路径、阻断、共享伤害骰和拒绝边界由核心定义，能力进度仍由 `abilityProgress` 保存，state hash 继续为 Schema v34。

contract-v81 以 1.73.0 增加 `teleport` 能力效果和 Echo Step，固定 position 落点、可见性/line of effect、可行走与 actor 占用拒绝，以及成功后复用普通移动到达管线；能力进度仍由 `abilityProgress` 保存，state hash 继续为 Schema v34。

contract-v82 以 1.74.0 增加 `summon` 能力效果、Echo Companion 与友方召唤 actor；空间验证、稳定实例 ID、所有者/阵营、玩家回合生命周期和到期移除由核心定义，召唤身份进入 `ActorSaveDto.summon`，state hash 升至 Schema v35。

contract-v196 以 1.191.0 为 actor 增加默认 false 的 `friendly`，表示自主的玩家侧怪物而不是控制或召唤所有权；怪物 `TRAPS` 继续使用既有 `transform-terrain` 能力程序。追踪者以 `shadower-appearance` 标签成为非分配外观定义，外观概率与真实身份保存由核心负责。

荒野 W0 以 1.192.0 为 `WorldDefinition` 增加可选 `wilderness`：独立宽高、起点、单字符图例、定长字符串行和强类型 town/dungeon 地点。图例保留 15 类 RFB 荒野 terrain、原始危险等级和道路事实；编译器验证尺寸、行宽、符号、边界、起点、唯一地点和正式内容引用。该字段当前只进入内容 artifact，不进入协议、存档或战术 floor。

物品 P1 以 1.193.0 从 RFB `master` 严格选择并同步 8 件等级 10、带 `TOWN` 标记的普通武器，保留权威重量、价值、槽位和伤害骰，并加入 Outpost 武器店；当前深度 1–9 的 Warrens 掉落表不变。

物品 P2 以 1.194.0 正式化 45 个规则完整的原版卷轴身份和 44 个原版药水身份；新增 5 个卷轴、22 个药水及对应强类型 effect program，并为全部 89 个条目固定权威名称、中文名、flavor、重量和价值。原版 `TOWN` 条目进入 Alchemist/Temple，等级 0–9 条目按源等级进入 Warrens。三个动态设备壳与四本死亡领域法书完成覆盖账本核对。Treasure Detection 因 Rewrite 金币堆不是 item instance，明确记录为 `gold-detection` 阻塞，不以普通物品探测替代。内容 hash 为 `7b040392db6925522459b6b0fd6f484a615d67d16eaed95f2885d026d0618774`。

物品 P3.1 以 1.195.0 增加 `satisfy-hunger`、通用 `apply-status`、`self-damage` 与 `drain-resource-full`，并把这些窄 self 效果纳入 2–8 步非嵌套 `sequence`。食物严格按“特殊效果后增加源 `pval` 营养”执行；幻觉保存为普通状态，Web 仅用 cell index 与权威 `worldTick` 做确定性显示扰动，不触碰核心 RNG。23 个计划身份全部由 blocked 转 active；Fast Recovery、酒、葡萄酒和精灵面包继续保留完整 blocker。正式包现有 204 items，内容 hash 为 `56ed89d064461fa87225ef8e06f030b75c29648bdaa33a8236e3c2f116e0dcf7`，协议、存档和 state-hash Schema 不变。

contract-v197 以 1.193.0 收紧既有怪物近战内容：只有 `armorMitigated: true` 的 physical `damage` effect 可以使用 `damageDice: 0, damageSides: 0`，用于权威无骰 `HURT`；其他 damage 和 poison 仍要求正骰。`S_LOUSE` 不增加内容类型，继续使用既有 `summon-category`，由 `louse` actor tag 与最大等级约束候选。

contract-v198 以 1.194.0 增加窄怪物近战 `disenchant { chancePercent? }`。它不携带伤害骰，只复用玩家有效 Disenchant 抗性、已建模正面状态移除和已装备物品的 `enchantments`；带骰 `DISENCHANT` 仍是普通 damage effect。当前格式不为原版 pval、逐件 `OF_RES_DISEN` 或怪物装备另建影子字段。

contract-v199 以 1.195.0 为既有 actor `light` 增加默认 false 的 `darkness`，并增加怪物施法 effect `darken-room`。负半径不另建第二套形状模型：同一个正整数 `radius` 描述作用域，`darkness` 只决定它压制永久房间光而不是产生主动光。`darken-room` 必须以非 self 的 position/entity 目标使用，当前不开放给玩家能力。

集成包 1.200.0 同时包含荒野 W0–W5、物品 P1–P3.1 与怪物 P13–P19，共 86 terrain、235 actors、204 items、114 abilities，内容 hash 为 `2273089117afc9e9f5ac4947407da9463d6eb8946fcbf7fb3a1a3f27cebd336b`。

contract-v205 / 包 1.201.0 为 actor 增加可选 `contactAura` 与分配字段 `legacyDungeonIndices`，为 dungeon 增加可选正整数 `legacyIndex`；经验吸取使用窄 `drain-experience` melee effect，变形继续复用既有 `appearanceKindId`。该版包共 86 terrain、241 actors、204 items、116 abilities，内容 hash 为 `e2fd133bcd3f2e3c2fd4d3ab8e25da6c437bfa18bede03d039d55a3db35406ae`。协议 1.147 与 state hash Schema v70 不变。

contract-v206 / 包 1.202.0 只增加 20 条十二级非施法 actor 内容记录；既有 actor 字段完整承载经验吸取、毒素与属性损伤、眩晕、死亡爆炸、繁殖、群体、护卫、荒野/地牢限定、水生、骑乘、穿墙/毁墙、隐形、Unique、尸骨和 Warrior 掉落。该版包共 86 terrain、261 actors、204 items、116 abilities，严格同步 196 条，内容 hash 为 `8f68bd58310207e0a9e7d1370d1a09731213fb1323753f6f28e2182b8ef2f8dc`。协议 1.147 与 state hash Schema v70 不变。

contract-v207 / 包 1.203.0 增加 7 条十二级施法 actor 与 8 个按完整参数签名去重的 ability 内容记录；没有增加数值覆盖层或 effect 类型。当前包共 86 terrain、268 actors、204 items、124 abilities，严格同步 203 条，内容 hash 为 `463afcc8f813025b618ed68697d3cc67c99483ed56f5dc598d62a30a120c8502`。协议 1.147 与 state hash Schema v70 不变。

contract-v208 / 包 1.204.0 增加 10 条十三级 actor 与 5 个按完整参数签名生成的 ability 内容记录；自我加速只绑定既有 Haste 状态效果，没有增加数值覆盖层或 effect 类型。当前包共 86 terrain、278 actors、204 items、129 abilities，严格同步 213 条，内容 hash 为 `4b1c3378af39464ad9450bfc3148fc338b79f3ccd17bedf6fe2f776d226e23cb`。协议 1.147 与 state hash Schema v70 不变。

contract-v209 / 包 1.205.0 为 actor 增加默认 false 的 `movesWeakerBodies` 与 `regenerates`。前者只控制符合经验值、阵营和双向地形条件的换位，后者只将公共怪物再生量翻倍；固定间隔、低 HP RNG 与 400 上限留在核心。黏菌及 `mind-blast-7d7` 进入正式包。当前包共 86 terrain、279 actors、204 items、130 abilities，严格同步 214 条，内容 hash 为 `3d94f3bff136355b23ad4a864f8308197606e79d2c92a3f36ad07f6b69a2c886`。协议 1.147 与 state hash Schema v70 不变。

contract-v83 以 1.75.0 增加 `detect` 能力效果、Echo Pulse 与 Echo Sight；类别/半径、FOV 与隐藏投影筛选、稳定结果顺序、瞬时/持久知识边界由核心定义，持久结果复用 `revealedTerrain`，state hash 升至 Schema v36。

contract-v84 以 1.76.0 增加 `transform-terrain` 能力效果、Echo Delving 与 Echo Rampart；来源/目标 terrain 集、范围、FOV/line of effect、占用格、连接/边界保护和原子写入由核心定义，地形继续复用既有 save/state hash 字段。

contract-v88 以 1.80.0 扩展 `MonsterCastingDefinition`：`smart` 控制确定性观察学习，`preferredDistance` 声明 2–16 格的首版保持距离，`fleeHpPercent` 声明 0–99% 受伤撤退阈值。频率与能力权重仍由内容声明；阵营目标、敌我 footprint、实际结算后抗性观察、移动格选择和 RNG 边界由核心定义。Echo Cantor 启用 smart、3 格偏好距离和 25% 撤退阈值；content hash 为 `29116f924e1ef4ddf6b0aa43f3b1b1bd0b4d28245ac086bce30d7a008e8e9e8e`。

contract-v85 以 1.77.0 增加状态添加/移除、有序 `sequence`、Echo Quickening 与 Echo Binding；逐效果顺序、堆叠、抗性缩时、免疫、部分无效和目标死亡跳过由核心定义。

contract-v86 以 1.78.0 增加 Echo Cantor 和 Monster actor `monsterCasting`；百分比频率、稳定加权候选、直接投影可用性、clean-shot 友军阻挡与按自身行动计数的逆频率冷却由核心定义。

contract-v87 以 1.79.0 扩展 Echo Cantor 的候选池，并增加 Call Discord 与 Discordant Echo；怪物可复用自身治疗/增益、范围/射线/锥形和召唤效果，HP/状态/距离有效权重、次级实体风险与敌对 owner 由核心定义。

contract-v88 以 1.80.0 增加 `smart`、`preferredDistance` 和 `fleeHpPercent`，并让 Echo Cantor 使用 3 格偏好距离、25% 受伤撤退和已观察抗性记忆；阵营目标、敌我计数和实际多目标结算由核心定义。contract-v89 只增加玩家级召唤物命令、行动与跨层规则，不修改内容 schema 或 demo 数据，因此内容版本/hash 保持不变。contract-v90 以 1.81.0 为 `ResourceDefinition` 增加 `initialFillPercent`、`meleeHitGainAmount`、`meleeKillGainAmount` 和 `turnDecayAmount`，为 `ClassDefinition` 增加多条目 `techniqueProfiles`（资源、主宰属性、上限公式、最低失败率与先天能力），并加入节奏资源、决斗家职业/构筑/技能集与弦月斩、涌动节奏两个技法能力；Mana 与既有职业数据不变。contract-v91 以 1.82.0 为能力效果增加 `blink-self`、`teleport-self` 与 `teleport-target` 三种怪物位移形态（怪物施法白名单准入），并加入裂隙潜行者与三个位移能力。

当前包的 active 编译版本为 1.153.0，content hash 为 `cbcca1349df4d40a76a5de10759d3a2bffa17bfe4c71fc486389c5b21b4d525e`。contract-v150 新增第二个 `demo.world.warrens-journey`：九层线性 dungeon、独立 campaign、四个早期 actor、两件基础物品、一个 terrain、一个 encounter table 和两个 loot tables。contract-v151 在既有 Schema 内新增 `demo.build.warrior`、一个独立玩家 actor、显式采用 RFB Standard 槽位的 Human race，以及 Broad Sword、Chain Mail、Short Bow、Arrow 四件出生物品；生产 Warrens 世界改以 Warrior 开局。contract-v153 为 `ProceduralRoomShape` 增加 `cavern`，并为 rooms layout 增加可选的 `stairs.up/down` 数量范围。contract-v154 新增四种独立地表 terrain，把生产入口从室内占位视觉改为草地、土路、岩壁与密林组成的固定地表；Schema 不变。contract-v155 为 rooms geometry 增加默认 `partitioned`、可选 `free` 的放置策略，并以既有 streamer 预算表达 Warrens 岩浆岩/石英矿脉。contract-v156 为 loot table 增加深度、概率与数量骰，为 actor 增加概率遗骸，为程序化楼层增加面积缩放的房间/全图物品分配，并为 guardian 增加独立奖励表。contract-v157 为 actor 增加物品成功掉落后的金币替换概率，并为程序化楼层增加面积缩放的 `goldAllocation`；金币实例与钱包属于运行时协议，不是内容根。contract-v158 加入口粮和 `increase-nutrition` 效果，并为 Warrens 程序化楼层增加 50% 食物保证尝试。contract-v159 为 item 增加严格的 `fuel { kind, initial, maximum, lightRadius }`，录入木制火把、黄铜灯笼和油瓶，并为 Warrens 增加 50% 光源保证尝试及油/灯笼 1:2 权重；实例当前燃料仍属于运行时协议。contract-v160 新增严格的 `towns`/`shops` 根、`TownDefinition`/`ShopDefinition`、world `townId` 及城镇/商店/入口交叉验证；店主、库存、价格与交易仍是运行时后续边界。contract-v161 为 item 增加 `baseValue`、为 Race 增加 `shopAdjustPercent`，并扩展 `ShopDefinition` 的固定 owner、严格 stock 数量范围和 maintenance 周期；General Store 必须且只能列出首批四种正价值补给。contract-v162 增加 Temple/Alchemist 类别与各自严格库存。contract-v163 增加 Magic Shop 类别、三种 RFB 原版动态设备、两个城镇边界 terrain 和严格四店库存，并把 Warrens 地表重排为围墙城镇与城外地牢入口。完整选择与兼容差异见 [Contract v150](contract-v150-warrens-journey.md)、[Contract v151](contract-v151-warrior-and-dungeon-status.md)、[Contract v153](contract-v153-warrens-map-generation.md)、[Contract v154](contract-v154-warrens-surface-entry.md)、[Contract v155](contract-v155-warrens-generation-density.md)、[Contract v156](contract-v156-warrens-loot.md)、[Contract v157](contract-v157-gold-wallet.md)、[Contract v158](contract-v158-food-hunger.md)、[Contract v159](contract-v159-fuel-light.md)、[Contract v160](contract-v160-outpost-content.md)、[Contract v161](contract-v161-general-store-transactions.md)、[Contract v162](contract-v162-outpost-supply-court.md) 与 [Contract v163](contract-v163-walled-outpost-magic-shop.md)。

运行时只加载验证通过的编译包。开发热重载也必须先通过相同验证，不能绕过 Schema。

## 6. 数据包组合

- 依赖先按拓扑排序；
- 同级包按明确的用户加载顺序，再以 pack ID 作为稳定 tie-breaker；
- 默认禁止两个包静默定义同一 ID；
- 修改已有定义必须使用显式 patch 文件；
- patch 只能修改 Schema 允许的字段；
- 删除内容必须显式声明，并在载入旧存档时给出迁移或缺失内容错误；
- 合并结果和加载顺序进入 content hash。

v1 不支持任意脚本执行。复杂规则由核心提供带版本的声明式组件和效果 ID。

## 7. Patch 格式

v1 使用受限字段操作，不使用依赖数组下标的通用 JSON Patch：

```json
{
  "formatVersion": 1,
  "target": "rfb.monster.dragon.red",
  "set": { "level": 62 },
  "addTags": ["boss-candidate"],
  "removeTags": []
}
```

列表型复杂对象必须带稳定子 ID，patch 按子 ID 增删改，禁止按第几个元素定位。

## 8. Tileset 与本地化

- 内容只提供语义 ID、glyph fallback 和可选视觉标签；
- tileset manifest 把语义 ID 映射到资源；
- 名称和描述只引用 Fluent key；
- 数据包可以附带 locale，但不能覆盖其他包的 key，除非 manifest 显式声明翻译扩展关系；
- 缺失图片 tile 时回退 glyph，缺失当前语言时回退 `en-US`。

## 9. 存档兼容

存档记录：

- 已启用包 ID、版本和 hash；
- 合并后的总 content hash；
- 使用到的定义 ID；
- 必要的迁移 alias 版本。

载入时如果内容集合不同，默认拒绝继续并展示差异。未来可以提供“安全模式”，但不能把缺失定义静默替换成另一对象。

## 10. 安全限制

- 单文件、单包、贴图尺寸和解压后总大小设上限；
- 所有相对路径规范化后必须留在包目录内；
- 禁止远程 URL 在游戏运行时自动下载代码或资源；
- 图片、字体和本地化文件按不可信输入处理；
- 编译器和运行时解析器都进行 fuzz 测试；
- 数据包不能访问文件系统、网络或核心内部对象。

## 11. v1 验收

- 一个基础包可以定义最小地图、玩家、怪物和物品；
- 所有原生平台加载后产生相同 content hash；
- 重复 ID、悬空引用、循环依赖和非法 patch 都会失败；
- 包加载顺序可复现；
- 缺失本地化和 tileset 映射有明确回退；
- 存档能够验证精确内容集合。

当前完成情况：

- 已完成：`rfb-content` crate、`rfb-contentc`、源包验证和编译容器回环；
- 已完成：`packs/rfb-demo-original`，包含 68 种地形、44 种 actor、140 种物品、3 种资源、69 个能力、6 本能力书、10 个 skill、13 个 skill set、5 个 Race、6 个 Class、3 个 Personality、7 个 build、7 张 encounter table、14 张 loot table、3 张 theme table、1 张 region table、1 张 terrain feature table、6 个 Vault、1 个 town、1 个 town facility、8 个 shop 和 2 个 world；
- 已完成：确定性 hash、lock 文件、checksum 损坏和悬空引用测试；
- 已完成：内容 Schema 生成与 CI 漂移检查；
- 已完成：Rust 核心运行时解码 `.rfbcontent`，按稳定 ID 建立地形、角色、物品和世界索引；
- 已完成：核心从编译世界创建地图和实例，存档验证真实 content ID/hash 和 world ID；
- 已完成：前端从核心快照取得内容 glyph，不再在 TypeScript 构建期导入内容 JSON；
- 待完成：多包依赖图、patch、locale 回退和已安装内容集合迁移。

首个包的真实编译 hash 与 contract-v1 使用的早期占位 content hash 不同。运行时激活通过 `contract-v2` 和 state hash Schema v2 完成；背包、装备、物品实例、战斗、行动调度与状态抗性依次迁移到 contract-v3–v9。contract-v12 至 v21 依次建立近战、怪物 routine、投射、重量、知识和消耗品；contract-v22–v25 建立 affix、质量、loot table 与怪物携带物；contract-v26–v45 建立程序化楼层、地形交互、多层探索和任务状态机；contract-v46–v69 建立生成表、分阶段地貌、树状地牢、实例身份、campaign 和生命周期；contract-v70–v90 建立成长、构筑、玩家/怪物施法、召唤物与职业资源；contract-v91–v103 建立导入所需的法术族、抗性、身体槽、装备旗标和动态 affix；contract-v104–v107 完成 Death 四册；contract-v108–v149 继续建立设备与窄物品效果；contract-v150–v172 建立 Warrens 玩家流程、Outpost、物品/背包/负重和可选玩家能力策略；contract-v173–v180 完成 Warrens W1–W13。v177–v180 为 actor 加入有序 melee effects、死亡爆炸、terrain interaction、typed light 与 death drop，为 terrain 加入 `monsterDestroyToTerrainId`，为 item/affix 加入 `resistsMonsterDestruction`。当前 state hash 为 Schema v63；内置内容 hash 为 `fed9c01421e0ee68a6cde5d0b864aee32f4a218d58457cc0d0d06ab6b7d6334f`。
