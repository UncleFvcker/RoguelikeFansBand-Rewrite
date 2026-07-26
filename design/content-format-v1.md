# RFB 内容数据格式 v1

状态：P0 源格式、JSON Schema、确定性编译器和首个原创内容包已实现；当前内容包已扩展至 1.80.0

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
- 单文件 1 MiB、单包 16 MiB、最多 2048 文件的输入上限；
- 禁止内容目录和文件符号链接；
- 稳定 ID、语义版本、消息 key、glyph、tag 和数值范围检查；
- 世界中的地形、角色与物品悬空引用检查；
- 定义、tag、spawn 和地形覆盖的规范化排序；
- `RFBCONT\0`、MessagePack payload、长度和 SHA-256 校验；
- `content.lock.json` 固定包 ID、版本和编译 content hash；
- 二十一份提交到 `schemas/content-v1/` 的 JSON Schema。

角色定义使用必需的基础战斗字段；玩家可声明携带容量与门/搜索技能，怪物可声明 melee routine、出生携带与死亡掉落、awareness，以及 `monsterCasting` 的百分比频率、加权能力集合、smart、偏好距离和撤退阈值。物品、资源、能力、能力书、affix、encounter/loot/theme/region/terrain-feature 表、Vault 和 world 使用独立稳定 ID 与交叉引用；编译器验证目标存在、角色类别、范围、数量、权重和互斥旧字段。原创包 1.80.0 覆盖角色成长与构筑、玩家能力循环、怪物 caster 效用/阵营目标/多格结算/战术移动/有限记忆、固定词条与鉴别，以及楼层/任务/树状地牢/Vault/区域主题/群体/分阶段地貌等现有纵切。

多包拓扑排序、patch、locale 完整性和开发期索引仍待后续实现。

contract-v79 以 1.71.0 增加固定八向 `cone-damage` 能力效果和 Echo Fan；锥形半径、伤害参数与目标模式继续由内容定义，能力进度仍由 `abilityProgress` 保存，当前 state hash 为 Schema v34。

contract-v80 以 1.72.0 扩展 Echo Lance 的 `beam-damage` 目标模式为方向、格子和实体，并固定定点/实体目标穿过目标后的延长路径；路径、阻断、共享伤害骰和拒绝边界由核心定义，能力进度仍由 `abilityProgress` 保存，state hash 继续为 Schema v34。

contract-v81 以 1.73.0 增加 `teleport` 能力效果和 Echo Step，固定 position 落点、可见性/line of effect、可行走与 actor 占用拒绝，以及成功后复用普通移动到达管线；能力进度仍由 `abilityProgress` 保存，state hash 继续为 Schema v34。

contract-v82 以 1.74.0 增加 `summon` 能力效果、Echo Companion 与友方召唤 actor；空间验证、稳定实例 ID、所有者/阵营、玩家回合生命周期和到期移除由核心定义，召唤身份进入 `ActorSaveDto.summon`，state hash 升至 Schema v35。

contract-v83 以 1.75.0 增加 `detect` 能力效果、Echo Pulse 与 Echo Sight；类别/半径、FOV 与隐藏投影筛选、稳定结果顺序、瞬时/持久知识边界由核心定义，持久结果复用 `revealedTerrain`，state hash 升至 Schema v36。

contract-v84 以 1.76.0 增加 `transform-terrain` 能力效果、Echo Delving 与 Echo Rampart；来源/目标 terrain 集、范围、FOV/line of effect、占用格、连接/边界保护和原子写入由核心定义，地形继续复用既有 save/state hash 字段。

contract-v88 以 1.80.0 扩展 `MonsterCastingDefinition`：`smart` 控制确定性观察学习，`preferredDistance` 声明 2–16 格的首版保持距离，`fleeHpPercent` 声明 0–99% 受伤撤退阈值。频率与能力权重仍由内容声明；阵营目标、敌我 footprint、实际结算后抗性观察、移动格选择和 RNG 边界由核心定义。Echo Cantor 启用 smart、3 格偏好距离和 25% 撤退阈值；content hash 为 `29116f924e1ef4ddf6b0aa43f3b1b1bd0b4d28245ac086bce30d7a008e8e9e8e`。

contract-v85 以 1.77.0 增加状态添加/移除、有序 `sequence`、Echo Quickening 与 Echo Binding；逐效果顺序、堆叠、抗性缩时、免疫、部分无效和目标死亡跳过由核心定义。

contract-v86 以 1.78.0 增加 Echo Cantor 和 Monster actor `monsterCasting`；百分比频率、稳定加权候选、直接投影可用性、clean-shot 友军阻挡与按自身行动计数的逆频率冷却由核心定义。

contract-v87 以 1.79.0 扩展 Echo Cantor 的候选池，并增加 Call Discord 与 Discordant Echo；怪物可复用自身治疗/增益、范围/射线/锥形和召唤效果，HP/状态/距离有效权重、次级实体风险与敌对 owner 由核心定义。

contract-v88 以 1.80.0 增加 `smart`、`preferredDistance` 和 `fleeHpPercent`，并让 Echo Cantor 使用 3 格偏好距离、25% 受伤撤退和已观察抗性记忆；阵营目标、敌我计数和实际多目标结算由核心定义。contract-v89 只增加玩家级召唤物命令、行动与跨层规则，不修改内容 schema 或 demo 数据，因此内容版本/hash 保持不变。contract-v90 以 1.81.0 为 `ResourceDefinition` 增加 `initialFillPercent`、`meleeHitGainAmount`、`meleeKillGainAmount` 和 `turnDecayAmount`，为 `ClassDefinition` 增加多条目 `techniqueProfiles`（资源、主宰属性、上限公式、最低失败率与先天能力），并加入节奏资源、决斗家职业/构筑/技能集与弦月斩、涌动节奏两个技法能力；Mana 与既有职业数据不变。

当前原创包的 active 编译版本为 1.80.0，content hash 为 `29116f924e1ef4ddf6b0aa43f3b1b1bd0b4d28245ac086bce30d7a008e8e9e8e`；其能力效果集合包含普通伤害、范围爆发、方向/定点/实体延长射线、固定八向锥形、精确短距位移、友方/敌对召唤、瞬时/持久 terrain 侦测、原版式 terrain 转换、状态添加/移除、有序多效果和固定治疗，并由怪物 caster 复用 actor 与多格目标子集。

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
- 已完成：`packs/rfb-demo-original`，包含 47 种地形、12 种 actor、8 种原创物品、1 种资源、6 个能力、2 本能力书、10 个 skill、11 个 skill set、3 个 Race、5 个 Class、3 个 Personality、5 个 build、6 张 encounter table、7 张 loot table、3 张 theme table、1 张 region table、1 张 terrain feature table、6 个 Vault 和 1 个 world；
- 已完成：确定性 hash、lock 文件、checksum 损坏和悬空引用测试；
- 已完成：内容 Schema 生成与 CI 漂移检查；
- 已完成：Rust 核心运行时解码 `.rfbcontent`，按稳定 ID 建立地形、角色、物品和世界索引；
- 已完成：核心从编译世界创建地图和实例，存档验证真实 content ID/hash 和 world ID；
- 已完成：前端从核心快照取得内容 glyph，不再在 TypeScript 构建期导入内容 JSON；
- 待完成：多包依赖图、patch、locale 回退和已安装内容集合迁移。

首个包的真实编译 hash 与 contract-v1 使用的早期占位 content hash 不同。运行时激活通过 `contract-v2` 和 state hash Schema v2 完成；背包、装备、物品实例、战斗、行动调度与状态抗性依次迁移到 contract-v3–v9。contract-v12 至 v21 依次建立近战、怪物 routine、投射、重量、知识和消耗品；contract-v22–v25 建立 affix、质量、loot table 与怪物携带物；contract-v26–v45 建立程序化楼层、地形交互、多层探索和任务状态机；contract-v46–v62 建立最终守护者、Vault/encounter/theme/region/terrain feature 表、预算、群体和分阶段地貌；contract-v63–v69 建立树状地牢、多入口 Vault、实例身份、动态探索树、入口守卫、campaign 和可配置实例生命周期；contract-v70–v72 建立角色成长、构筑和可观察技能检定；contract-v73–v85 依次建立玩家资源、能力书、恢复、熟练度/冷却、学习容量、范围/射线/锥形/位移/召唤/侦测/地形/状态及有序效果；contract-v86–v88 建立怪物施法、效用、阵营目标、战术移动与有限抗性记忆；contract-v89 建立友方召唤物行动与全局命令；contract-v90 建立多职业资源底子与首个技法资源。当前 state hash 为 Schema v40。
