# RFB 内容数据格式 v1

状态：P0 源格式、JSON Schema、确定性编译器和首个原创内容包已实现

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
├─ actors/
├─ affixes/
├─ encounterTables/
├─ items/
├─ lootTables/
├─ terrain/
├─ themeTables/
├─ vaults/
├─ worlds/
├─ locales/
└─ assets/
```

当前 v1 编译器实现 `actors`、`affixes`、`encounterTables`、`items`、`lootTables`、`terrain`、`themeTables`、`vaults` 和 `worlds` 九个严格类型根。后续怪物能力、职业、种族、法术、任务和视觉映射会在相同稳定 ID/Schema 规则下增加独立根；扩展包可以只声明自己实际提供的根。

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

contract-v42 新增 `retakeable`（默认 `false`）。启用后，未完成时普通离开会保存任务层并保持入口开放；重新进入恢复原楼层，而显式放弃或完成仍关闭入口。

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
- 十份提交到 `schemas/content-v1/` 的 JSON Schema。

角色定义使用必需的基础战斗字段；玩家通过 `carryCapacityTenthsPound` 声明正整数携带容量，并可用 `doorSkill` / `bashPower` / `searchSkill` 声明开锁、破门和搜索基础能力。怪物可声明 `meleeRoutine.blows`、出生携带用 `carriedLootTableId` 和死亡生成用 `lootTableId`。物品必须声明整数重量，并可声明近战、发射、投掷或使用 profile。独立 `AffixDefinition` 声明实例修正；世界实例可声明 `ordinary`、`fine`、`exceptional` 质量，非普通质量与 affix 都只允许数量为一、不可堆叠的实例，affix 还要求物品可装备。独立 `LootTableDefinition` 声明加权物品、品质和词条。独立 `EncounterTableDefinition` 声明楼层怪物 roll 与深度加权候选；独立 `ThemeTableDefinition` 声明楼层主题 terrain 与加权 Vault 候选；独立 `VaultDefinition` 声明主题 terrain、空间变换、深度加权 encounter group 与主题掉落。地形可声明互斥的 `openToTerrainId` 或 `closeToTerrainId`；普通门互反转换要求关闭态不可行走/阻光、开启态可行走/透光。带 `openCheckDifficulty` 的锁门允许单向解锁到普通开启态；`bashToTerrainId` / `bashCheckDifficulty` 成对声明破门结果；`concealedAsTerrainId` / `searchCheckDifficulty` 成对声明隐藏投影和搜索难度，真实/伪装 terrain 必须保持相同碰撞与阻光语义。世界定义必须声明稳定 `initialFloorId` 和首个 `proceduralFloor`，后者固定楼层 ID、名称、返回层、深度、尺寸、地形引用、楼层表引用以及可选 actor/loot/Vault 空间预算。怪物候选必须引用怪物定义且至少包含一个等级不高于该层深度的候选；运行时只从符合深度的候选中抽取。旧 `actorSpawns/lootSpawns/themeId/vaultId` 保留为兼容输入，但不能与对应新表引用混用。原创包 1.43.0 覆盖固定词条、鉴别、怪物携带物、确定性死亡掉落、楼层/任务/地牢生命周期、三层 Vault/巢穴地牢、预算化十层压力地牢和深度 8 多 Vault 空间场景。

多包拓扑排序、patch、locale 完整性和开发期索引仍待后续实现。

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
- 已完成：`packs/rfb-demo-original`，包含 37 种地形、一个玩家原型、九种原创怪物、五种原创物品、三个 encounter table、五个 loot table、两个 theme table、一个 terrain feature table、五个 vault 和一个带 20×20 地表、三层主题地牢及十层压力地牢的世界；
- 已完成：确定性 hash、lock 文件、checksum 损坏和悬空引用测试；
- 已完成：内容 Schema 生成与 CI 漂移检查；
- 已完成：Rust 核心运行时解码 `.rfbcontent`，按稳定 ID 建立地形、角色、物品和世界索引；
- 已完成：核心从编译世界创建地图和实例，存档验证真实 content ID/hash 和 world ID；
- 已完成：前端从核心快照取得内容 glyph，不再在 TypeScript 构建期导入内容 JSON；
- 待完成：多包依赖图、patch、locale 回退和已安装内容集合迁移。

首个包的真实编译 hash 与 contract-v1 使用的早期占位 content hash 不同。运行时激活通过 `contract-v2` 和 state hash Schema v2 完成；背包、装备、物品实例、战斗、行动调度与状态抗性依次迁移到 contract-v3–v9。contract-v12 至 v21 依次建立近战、怪物 routine、投射、重量、知识和消耗品；contract-v22 以 1.17.0 增加 affix 根和实例引用，contract-v23 以 1.18.0 增加实例质量，contract-v24 以 1.19.0 增加 loot table 根和死亡引用，contract-v25 以 1.20.0 增加出生携带引用，contract-v26 以 1.21.0 增加稳定入口层和程序化楼层定义，contract-v27 以 1.22.0 增加深度与房间内容，contract-v28–v35 建立地形交互、多层与探索生命周期，contract-v36–v45 建立任务状态机，contract-v46 以 1.39.0 建立最终层与守护者，contract-v47 以 1.40.0 增加 vault 根、深度 encounter group 与主题 loot，contract-v48 以 1.41.0 增加 encounter/theme 根、楼层表引用、加权 Vault 与巢穴，contract-v49 以 1.42.0 增加 actor/loot 生成预算、深度主题分段和十层压力场景，contract-v50 以 1.43.0 增加 Vault 变换、空间预算、多模板落位和失败回退，contract-v51 以 1.44.0 增加动态 friends/escort、formation 和群体预算，contract-v52 以 1.45.0 增加 terrain feature 根、room/corridor 放置和特殊地形预算，contract-v53 以 1.46.0 增加分阶段 layout、cavern 地貌和房间几何预算，contract-v54 以 1.47.0 增加深浅 lake/river 水文阶段与面积预算，contract-v55 以 1.48.0 增加 maze/destroyed/streamer 阶段与空间预算。当前 state hash 为 Schema v19。
