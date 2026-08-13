# RFB 新存档格式 v1

状态：v1 容器、独立权威存档 DTO、校验读写和 Windows/Tauri 原生目录事务已实现；跨 schema 迁移链与旧 C 存档完整导入仍未实现

## 1. 基本决定

- 扩展名：`.rfbsave`；
- 容器格式：RFB 自描述二进制容器；
- 权威载荷：MessagePack；
- 调试导出：规范化 JSON，但不能作为正式可继续游戏的默认格式；
- 校验：SHA-256；
- 默认不压缩 v1 载荷，后续通过容器 flags 增加版本化压缩；
- 旧 C 存档只通过独立导入器读取，新核心不再写旧格式。

## 2. 容器布局

```text
magic              8 bytes   "RFBSAVE\0"
container_version  u16 LE     1
flags              u16 LE
header_length      u32 LE
payload_length     u64 LE
payload_sha256     32 bytes
header_json        header_length bytes UTF-8
payload_msgpack    payload_length bytes
```

读取器必须先验证长度上限，再分配内存。未知 flags、超大长度、截断文件和 hash 不符必须给出明确错误，不能 panic。

## 3. Header

Header 只含无需解码完整世界即可展示的信息：

```ts
interface SaveHeaderV1 {
  format: "rfb-save";
  saveSchemaVersion: 1;
  gameVersion: string;
  protocolVersion: string;
  slotName: string;
  createdAt: string;
  savedAt: string;
  characterSummary: {
    displayName: string;
    level: number;
    locationKey: string;
    turn: number;
  };
  contentId: string;
  contentHash: string;
  payloadEncoding: "messagepack";
}
```

Header 不可信，显示前需要长度限制和转义；载入是否成功以 payload 验证和迁移结果为准。

`slotName` 是桌面原生槽使用的可选显示元数据。Rust 反序列化对缺失字段使用空字符串默认值，因此本字段的加入不破坏已经生成的 v1 存档；手动导出的存档当前写入空名称。桌面目录事务和恢复行为见 [桌面原生存档与诊断 v1](desktop-native-storage-v1.md)。

## 4. Payload

```ts
interface SavePayloadV1 {
  schemaVersion: 1;
  revision: number;
  turn: number;
  worldTick: number;
  lastCommandSeq: number;
  mapScale: "local" | "world";
  wildernessPosition?: Position;
  wildernessViewOffset: Position;
  wildernessSeed: number;
  worldTravelDestination?: Position;
  terrain: TerrainSaveDto;
  player: PlayerSaveDto;
  entities: ActorSaveDto[];
  items: ItemSaveDto[];
  inventory: InventoryItemSaveDto[];
  equipment: EquipmentItemSaveDto[];
  carriedItems: CarriedItemSaveDto[];
  generatedArtifactIds: string[];
  nextItemInstanceSerial: number;
  explored: boolean[];
  rng: RngSaveDto;
  contentId: string;
  contentHash: string;
  worldId: string;
  currentFloorId: string;
  storedFloors: FloorSaveDto[];
}
```

`FloorSaveDto` 保存离层 ID、玩家离层位置、地形、怪物、地面物品、怪物携带物和探索记忆。当前活动层继续占用 payload 顶层的 `terrain`、`entities`、`items`、`carriedItems` 与 `explored`；`storedFloors` 只保存非活动层，载入时拒绝重复 ID 或把当前层同时放入仓库。

当前桌面垂直切片已经把地面 `items`、玩家 `inventory` 物品堆、`equipment` 装备列表、怪物 `carriedItems`、楼层仓库和 `nextItemInstanceSerial` 写入 payload。存档使用独立的 `PlayerSaveDto`、`ActorSaveDto`、`FloorSaveDto` 和物品存档 DTO，不再复用面向前端的 `PlayerDto`、`EntityDto`、`InventoryItemDto` 或 `EquipmentItemDto`。

Rust 运行时内部只保留一个 `ItemInstance` 集合，`ItemLocation` 明确区分 `Ground(position)`、`Inventory`、`Equipped(slotId)` 和 `CarriedBy(actorId)`。拾取、整堆丢弃、装备、卸下和怪物死亡放下携带物只改变同一实例的位置；部分拆堆才分配新的稳定实例 ID。v1 存档线格式投影为 `items`、`inventory`、`equipment` 和带默认值的 `carriedItems` 列表，但这些列表不对应多套核心结构体。

玩家存档保存实例 ID、种类 ID、位置、当前生命、自然最大生命、基础速度、当前 `energyNeed`、状态列表、抗性 profile、资源池与已学能力 ID；怪物保存相同的基础权威运行状态。状态保存稳定 kind ID、强度、剩余 tick 和可选来源 ID；普通抗性不显式写入稀疏列表。最终速度、攻击、防御、近战能力、AC、伤害骰、装备 modifier、能力失败率、目标规格、学习/施放可用性、死亡标志、glyph 和本地化文本均不写入新存档，而是在载入后重新派生。旧 v1 存档缺失状态/抗性字段时按空集合迁移。

背包与装备项保存稳定实例 ID、内容 kind ID、数量及装备槽 ID，不保存选择复选框或 HTML 面板状态。种类级 `itemKnowledge` 只保存非空 tried/aware 记录，并要求 aware 蕴含 tried；旧存档缺失该字段时按空知识表载入。载入后必须验证内容引用、实例 ID 唯一性、`maxStack`、槽位匹配、槽位唯一性、知识记录唯一且引用带外观名称的种类，以及生成实例序号不能落后于任何 `generated.item.N`。旧存档缺失 `equipment` 时按空列表载入，缺失分配序号时从所有现有实例 ID 推导。玩家负生命值代表已死亡，可安全保存和重载；`isDead` 仅是协议派生字段。

协议 1.3 增加 `explored` 布尔数组保存 Rust 权威地图记忆。旧存档缺失该字段时按空记忆载入并揭示玩家当前 FOV；探索记忆不参与 state hash。协议 1.23 在物品位置 DTO 保存质量，并以 `itemPropertyKnowledge` 保存 `appraised`、`identified` 与已知词条；v22 记录可从已有词条和装备位置迁移。协议 1.24 的战利品生成直接写入既有实例字段和 `nextItemInstanceSerial`。协议 1.25 新增可选 `carriedItems`；旧存档缺失时按空列表载入，不为已有怪物补抽携带物。协议 1.26 新增带默认值的 `currentFloorId` 和 `storedFloors`；旧存档迁移到世界入口层且没有离层状态。协议 1.27 不增加存档字段；已访问的 v26 程序化层保持原有实体集合，不补生成 v27 房间内容。协议 1.28 继续直接保存 terrain ID 数组；旧程序化层不会补插门。协议 1.29 的锁定/破损门继续使用 terrain ID；协议 1.30 的 `terrainInteractions` 是派生视图。协议 1.31 在当前层和每个 `FloorSaveDto` 新增带默认值的 `revealedTerrain` 稳定位置列表；旧存档按空知识载入，非法、重复、越界或指向非隐藏 terrain 的记录被拒绝。协议 1.41 新增可选 `taskProgress`；协议 1.43 将进度键规范为 `taskId`，并兼容读取旧 `floorId` 后按内容映射迁移。协议 1.44 新增可选 `taskStates`，保存任务 ID、状态、当前/要求进度和活跃楼层；旧 `taskProgress` 只作为迁移输入，新存档不再写入重复进度副本。协议 1.45 在任务状态中增加带默认值的 `stageIndex`，保存当前有序阶段；旧单目标状态按第零阶段载入。协议 1.46 新增可选 `dungeonStates`，保存守护者击败状态；探索实例楼层清除时该状态仍保留。协议 1.47 不增加存档字段；新生成 vault 仍作为普通 terrain、actor 和 item 保存，旧存档已有楼层不会被补绘。协议 1.48 也不增加存档字段；新表、加权 Vault 和巢穴只影响尚未生成楼层，v47 已生成楼层不会被回填或额外消费 RNG。协议 1.49 仍不增加存档字段；预算只影响尚未生成楼层，v48 已生成楼层不补 actor/loot，新压力地牢缺失的 `dungeonStates` 项只在已知旧 content hash 迁移时按未击败补入。协议 1.50 继续复用既有楼层 terrain/actor/item/RNG 字段；v49 已生成楼层不补绘旋转、镜像或多 Vault，也不推进 RNG。协议 1.51 同样只保存生成后的普通 actor 实体；v50 已生成楼层不补 friends/escort formation，也不推进 RNG。协议 1.52 直接把额外 trap/door/rubble 保存为既有 terrain ID；v51 已生成楼层不补放 terrain feature，也不推进 RNG。协议 1.53 同样只保存生成后的普通 terrain/actor/item；v52 已生成楼层不补绘 cavern、不重建房间，也不推进 RNG。协议 1.54 继续复用 terrain/actor/item/RNG 字段；v53 已生成楼层不补绘 lake/river，也不推进 RNG。协议 1.55 同样只保存最终 terrain/actor/item/RNG；v54 已生成楼层不补建 maze/destroyed/streamer，也不推进 RNG。state hash Schema v19 覆盖任务状态与持久地牢状态，也已覆盖生成后的 terrain、actor、item、RNG 与 content hash。

协议 1.56 继续复用既有 terrain/actor/item/RNG 字段；v55 已生成楼层不补建 pit、不补 actor，也不推进 RNG。state hash 继续使用 Schema v19。

协议 1.57 继续复用既有 terrain/actor/item/RNG 字段；v56 已生成楼层不改写为 maze-only、不移动既有 pit/loot，也不推进 RNG。state hash 继续使用 Schema v19。

协议 1.58 为当前层新增可选 `floorConnections`，为每个 `FloorSaveDto` 新增可选 `connections`；每项保存稳定连接 ID 与坐标。空列表兼容 v57 及更早的已生成楼层，并继续使用 legacy terrain 标签；非空列表必须与内容连接集合、坐标唯一性和实际 terrain 完全一致。save 容器仍为 v1，state hash 升至 Schema v20。

协议 1.59 为 `ActorSaveDto` 新增可选 `pack`，保存 pack ID、leader ID、角色和行为。当前层与离层 actor 使用同一格式；缺失字段兼容 v58 及更早存档，不重建群体或推进 RNG。载入拒绝非法 ID、缺失或重复 leader、跨 pack 引用、不一致角色以及玩家 pack。save 容器仍为 v1，state hash 升至 Schema v21。

协议 1.60 为当前层增加 `floorRegions`，并为离层 `FloorSaveDto` 增加 `regions`。每个 `FloorRegionSaveDto` 保存 region/theme ID、局部 encounter/loot 表引用和规范排序格集合。载入拒绝重复、重叠或越界边界、与楼层内容不一致的引用，以及区域外怪物/地面掉落。v59 及更早存档缺失字段时保留空区域，不重建地图或推进 RNG。save 容器仍为 v1，state hash 升至 Schema v22。

协议 1.61 为 `TaskStateSaveDto` 增加带默认值的 `retakesUsed`。载入拒绝超过内容 `maxRetakes` 的计数；v60 及更早存档按 0 次迁移，不在载入时重建任务层或推进 RNG。`regenerate-floor` 只在玩家之后显式重入时丢弃已保存成员层并生成剩余目标。save 容器仍为 v1，state hash 升至 Schema v23。

协议 1.62 不新增 save DTO 字段。区域组合生成继续使用 v60 的 `floorRegions/regions`，并把 Vault/pit footprint、局部表引用和特殊阶段产生的 actor/loot 归入互斥宿主区域；区域 actor 的行动保持在其当前 region cells 内。save 容器仍为 v1，state hash 继续为 Schema v23。

协议 1.63 不新增 save DTO 字段。现有 `dungeonStates[].guardianDefeated` 表示整座地牢的共享征服状态；各最终叶层镜像继续作为普通 actor 保存在当前层或 `storedFloors`。载入时检查所有已生成最终叶层：未征服层必须保留对应镜像，已征服地牢不得残留任何镜像。已知 v62 content hash 的旧连接集合若不再匹配 v63 树定义，只清除连接索引并使用 terrain 楼梯标签回退，保留地图、actor、item 和 RNG。save 容器仍为 v1，state hash 继续为 Schema v23。

协议 1.64 不新增 save DTO 字段。多入口 Vault 的模板、transform 和 connector 不作为独立运行时状态保存；它们的最终结果已经包含在当前层或 `storedFloors` 的完整 terrain、actor 和 item 中。已知 v63 content hash 的存档保持这些字段与 RNG draw counter 原样载入，不补绘 connector、不替换旧 Vault，也不推进生成 RNG。save 容器仍为 v1，state hash 继续为 Schema v23。

协议 1.65 为 `SavePayloadV1` 增加可选 `currentDungeonInstanceId`，为 `FloorSaveDto` 增加可选 `dungeonInstanceId`，并为 `DungeonStateSaveDto` 增加默认值为 0 的 `nextInstanceOrdinal`。缺失字段的 v64 及更早存档按 floor 的 dungeon 定义迁移到首实例，不重建地形、不重抽 RNG；一次性任务和地表 floor 保持空实例。返回地表时只丢弃当前实例的离层 floor，其他 dungeon/任务 floor 保留。save 容器仍为 v1，state hash 升至 Schema v24。

协议 1.66 为 `FloorConnectionSaveDto` 增加可选 `targetFloorId` 与 `targetConnectionId`。新生成楼层保存每个连接的解析目标；旧存档缺失时沿用内容中的固定 `targetFloorId/targetConnectionId`，不重建地形、不推进 RNG。动态目标和首次到达时写入的返回连接进入 state hash Schema v25。普通 dungeon 返回地表仍清理整个实例，下一次进入重新生成；save 容器仍为 v1。

协议 1.67 为 `DungeonStateSaveDto` 增加可选 `entranceGuardianDefeated`。新存档显式保存入口守卫是否已击败；v66 及更早存档缺字段时将内容新增的入口守卫视为已抑制，不在地表回填实体、不重建楼层，也不推进 RNG。载入时入口守卫实体必须与该状态一致。入口守卫状态进入 state hash Schema v26；普通 dungeon 返回地表仍清理整个实例，save 容器仍为 v1。

协议 1.68 为 `SavePayloadV1` 增加可选 `campaignState`，保存 campaign 状态、胜利回合、退休回合和冻结的最终分数。v67 及更早存档缺失时按已保存的 victory dungeon 征服状态推导 active 或 victorious，不补生成内容或推进 RNG。退休存档必须满足胜利条件、退休发生在地表、回合顺序合法且最终分数与评分公式一致；state hash 使用 Schema v27，save 容器仍为 v1。普通 dungeon 返回地表继续清理实例。

协议 1.69 为 `DungeonStateSaveDto` 增加可选 `retainedInstanceId` 与 `retainedAtTurn`。只有 `persistent`/`turn-ttl` dungeon 可以保存一个 retained 实例；其 stored floors、实例 ID 和物品属性知识一起参与存档校验。载入时 retained 实例缺楼层、生命周期为 reset、回合顺序非法或字段不完整都会拒绝。v68 及更早存档缺少字段时按默认 `reset-on-surface` 迁移，不回填 retained 实例、不重建地图、不推进 RNG。TTL 到期清理实例及其已删除物品的实例属性知识；种类级物品知识继续保留。state hash 使用 Schema v28，save 容器仍为 v1。

协议 1.70 为 `PlayerSaveDto.progress` 增加角色成长权威字段：六维自然属性、经验、等级、历史最高等级、待分配属性点和 100 项出生 HP 序列。缺少该可选字段的 v69 及更早存档按固定初始属性、等级 1、经验 0、每级生命 +6 的 legacy 规则迁移，不重建地图或推进正式 RNG；迁移后的 progress 与玩家生命校验一起进入 state hash Schema v29。未胜利存档可保留 50 级封顶后的额外经验，胜利或退休载入会在校验前确定性解锁并结算到 100 级。装备属性只作为派生有效属性，不写入自然属性字段。

协议 1.71 在 `PlayerSaveDto` 中增加可选 `build`，保存 build/Race/Class/Personality 的稳定 ID；`PlayerProgressSaveDto` 增加可选 `skills`，保存按当前内容和等级聚合的技能状态。v70 及更早存档缺少构筑或技能字段时使用世界默认构筑（demo 为 Explorer），并在载入时按保存等级重算技能；不生成出生装备、不覆盖已有物品、不推进正式 RNG。带出生装备的构筑同时保存种类知识和实例属性知识，确保 round-trip 后不丢失已知状态。构筑身份、技能聚合和出生装备实例进入 state hash Schema v30；save 容器仍为 v1。

协议 1.72 为 `ActorSaveDto` 增加可选 `alerted`。新存档显式保存怪物是否已经察觉玩家；v71 及更早存档缺少字段时按当前 actor 内容恢复，声明 `awareness.startsAlerted=false` 的怪物保持未警戒，其他怪物保持历史默认警戒。迁移不补做 stealth 检定、不移动怪物、不推进 RNG。警戒状态与技能检定造成的生命、物品知识和 terrain 揭示结果进入 state hash Schema v31；save 容器仍为 v1。

协议 1.73 为 `PlayerSaveDto` 增加可选 `resources` 与 `learnedAbilityIds`。每个 `ResourcePoolSaveDto` 保存稳定资源 ID、当前值和最大值；已学能力只保存稳定能力 ID。载入会依据当前 Class casting profile、等级和有效属性验证资源集合与最大值，并验证每个已学能力的等级、资源和能力书支持关系。v72 及更早存档缺少这些字段时，施法职业恢复按当前构筑计算的满资源和空已学列表，非施法职业保持空集合；迁移不补学习、不改变物品、不推进 RNG。资源和已学能力进入 state hash Schema v32；save 容器仍为 v1。

协议 1.74 不增加正式 save 字段。等待/休息恢复量来自当前内容中的 `ResourceDefinition`，存档继续只保存资源当前值/上限与已学能力 ID；`Rest` 的请求回合数和停止 outcome 只存在于命令/事件/回放中。载入 v73 内容 hash 的存档时不会补发 Stillwater Notes、自动学习 Mending Echo 或推进 RNG，既有资源值、物品与已学能力原样校验后进入当前规则。恢复后的资源、真实 `turn/worldTick`、生命、状态和 RNG 位置进入 state hash Schema v33；save 容器仍为 v1。

协议 1.75 为 `PlayerSaveDto` 增加可选 `abilityProgress`。每项保存稳定能力 ID、熟练度、内容上限、成功/失败次数和冷却剩余；能力定义中的初值、增量、冷却回合与组 ID不重复写入存档。载入 v73/v74 或其他缺少该字段的旧存档时，运行时按当前 Class casting profile 与内容能力建立默认进度，再恢复已有资源和已学能力；不自动学习、不补发物品、不推进 RNG。显式进度必须匹配当前能力上限，熟练度与冷却不能越界，重复/未知能力 ID 原子拒绝。能力进度、冷却和统计进入 state hash Schema v34；save 容器仍为 v1。完整边界见 [Contract v75](contract-v75-ability-proficiency-and-cooldowns.md)。

协议 1.76 不新增 save 字段：学习容量是 Class 内容与角色 progress 的派生值，主动遗忘只修改已有 `learnedAbilityIds`，不清除 `abilityProgress`。重新学习同一能力恢复原熟练度、统计与冷却；载入时若已学数量超过当前内容容量则原子拒绝。缺少 `abilityProgress` 的 v75 及更早存档继续按内容初值迁移，save 容器仍为 v1。完整边界见 [Contract v76](contract-v76-learning-capacity-and-forgetting.md)。

协议 1.77 不新增 save 字段：范围半径、伤害骰、伤害类型和目标模式来自当前内容；资源当前值、已学集合与 `abilityProgress` 继续使用既有字段。载入 v76 及更早存档时，缺失范围能力只按当前内容能力集合验证，不补学习、不生成物品、不重建地图、不推进 RNG。范围爆发的 footprint、目标顺序和事件只存在于命令执行结果/回放中，权威终态仍由既有 actor HP、资源、物品、任务和 RNG 字段表达。save 容器仍为 v1，state hash 仍为 Schema v34。完整边界见 [Contract v77](contract-v77-area-damage.md)。

协议 1.78 不新增 save 字段：射线形状、方向目标模式、伤害骰和伤害类型来自当前内容；资源当前值、已学集合与 `abilityProgress` 继续使用既有字段。载入 v77 及更早存档时，不自动学习 Echo Lance、不补发书本、不重建地图、不推进 RNG。射线 footprint、阻断格、目标顺序和事件只存在于命令执行结果/回放中，权威终态仍由既有 actor HP、资源、物品、任务和 RNG 字段表达。save 容器仍为 v1，state hash 仍为 Schema v34。完整边界见 [Contract v78](contract-v78-beam-damage.md)。

协议 1.79 不新增 save 字段：锥形半径、方向目标模式、伤害骰和伤害类型来自当前内容；资源当前值、已学集合与 `abilityProgress` 继续使用既有字段。载入 v78 及更早存档时，不自动学习 Echo Fan、不补发书本、不重建地图、不推进 RNG。锥形 footprint、阻断格、目标顺序和事件只存在于命令执行结果/回放中，权威终态仍由既有 actor HP、资源、物品、任务和 RNG 字段表达。save 容器仍为 v1，state hash 仍为 Schema v34。完整边界见 [Contract v79](contract-v79-cone-damage.md)。

协议 1.80 不新增 save 字段：定点/实体射线的目标模式、稳定斜率、伤害骰和伤害类型来自当前内容；资源当前值、已学集合与 `abilityProgress` 继续使用既有字段。载入 v79 及更早存档时，已学 Echo Lance 保留已有进度并从当前内容取得新增目标模式；不自动学习能力、不补发书本、不重建地图、不推进 RNG。射线 footprint、延长路径、阻断格、目标顺序和事件只存在于命令执行结果/回放中，权威终态仍由既有 actor HP、资源、物品、任务和 RNG 字段表达。save 容器仍为 v1，state hash 仍为 Schema v34。完整边界见 [Contract v80](contract-v80-targeted-beam-extension.md)。

协议 1.81 不新增 save 字段：位移效果、position 目标、落点验证和传送事件来自当前内容与命令结果；资源当前值、已学集合与 `abilityProgress` 继续使用既有字段。载入 v80 及更早存档时，不自动学习 Echo Step、不补发 Echo Primer、不重建地图、不推进 RNG；显式 study-save 仍按当前书本和能力定义学习。传送起点/终点、落点检查与到达事件只存在于命令执行结果/回放中，权威终态仍由既有玩家位置、资源、物品、任务和 RNG 字段表达。save 容器仍为 v1，state hash 仍为 Schema v34。完整边界见 [Contract v81](contract-v81-teleport-ability.md)。

协议 1.82 为 `ActorSaveDto` 增加可选 `summon`，其中 `SummonSaveDto` 保存 `ownerId`、`sourceAbilityId` 和 `remainingTurns`。缺少该字段的 v81 及更早 actor 按普通敌对 actor 载入，不生成召唤物、不改变已有实体、不推进 RNG。v82 首版只生成玩家 owner；协议 1.87 允许保存有效规则 ID 的怪物 owner。源能力必须存在且仍召唤同一 actor kind，剩余回合必须为正，并且不能与 pack identity 同时存在；任一不一致都会原子拒绝存档。召唤规格继续由锁定内容包提供，阵营由 owner 是否为当前玩家推导。save 容器仍为 v1，召唤身份和生命周期进入 state hash Schema v35。完整边界见 [Contract v82](contract-v82-summon-ability.md) 和 [Contract v87](contract-v87-monster-casting-utility.md)。

协议 1.83 不新增 save 字段：持久侦测复用既有 `revealedTerrain`，瞬时侦测结果只存在于本次事件。载入 v82 及更早存档时，不自动学习 Echo Pulse/Echo Sight、不补发能力书、不重建地图、不推进 RNG；已有秘密 terrain 发现知识原样保留。持久侦测命中的真实 terrain 会进入 `revealedTerrain`，瞬时命中不会进入存档。save 容器仍为 v1，新的知识/RNG 规则边界进入 state hash Schema v36。完整边界见 [Contract v83](contract-v83-detection-ability.md)。

协议 1.84 仍不新增 save 字段：地形改变能力直接修改当前 `TerrainSaveDto.terrainIds`，离层时继续由既有 `FloorSaveDto.terrain` 保存；修改格会从 `revealedTerrain` 移除。载入 v83 及更早存档时，不自动学习 Echo Delving/Echo Rampart、不补发能力书、不改写 terrain、不推进 RNG；旧 built-in content hash 只迁移到当前内容定义。terrain 原本已进入 state hash，故 save 容器保持 v1、state hash 保持 Schema v36。完整边界见 [Contract v84](contract-v84-terrain-transform-ability.md)。

协议 1.85 仍不新增 save 字段：Echo Quickening/Echo Binding 产生的状态继续写入既有 `ActorSaveDto.statuses` / `StatusSaveDto`，包括稳定 kind ID、强度、剩余 tick 和能力来源 ID。逐效果索引、抗性缩放结果、`no-target` 与 `target-dead` 只属于命令事件，不重复保存。载入 v84 及更早存档时不自动学习新能力、不添加或移除状态、不补发书本且不推进 RNG；旧 built-in content hash 只迁移到当前内容定义。actor statuses 原本已进入 state hash，故 save 容器保持 v1、state hash 保持 Schema v36。完整边界见 [Contract v85](contract-v85-ordered-status-effects.md)。

协议 1.86 为 `ActorSaveDto` 新增默认零的 `castingCooldownRemaining`。成功怪物施法按内容频率计算并保存剩余自身行动冷却；当前层和 `FloorSaveDto` 离层 actor 使用同一字段。频率骰、可用候选、权重骰和逐效果结果属于命令事件，不重复保存。载入 v85 及更早存档时缺失字段迁移为零，不自动触发怪物施法、不推进 RNG；旧 built-in content hash 只迁移到当前内容定义。save 容器保持 v1，新增权威冷却状态使 state hash 升至 Schema v37。完整边界见 [Contract v86](contract-v86-monster-casting-ai.md)。

协议 1.87 不新增 save 字段。候选有效权重、主目标、footprint、拒绝原因和选择骰只属于命令事件；自疗/状态效用由已保存 actor HP/status 与当前内容纯计算。敌对召唤继续写入既有 `SummonSaveDto`，其中 owner 是怪物施法者实例 ID；owner 后续死亡不会使存档无效或提前删除召唤物。载入 v86 及更早存档时不补生成 Discordant Echo、不触发施法、不推进 RNG；旧 built-in content hash 只迁移到当前内容定义。save 容器保持 v1，state hash 保持 Schema v37。完整边界见 [Contract v87](contract-v87-monster-casting-utility.md)。

协议 1.88 为 `ActorSaveDto` 增加默认空的 `observedPlayerResistances`，只保存 smart caster 已实际观察到的伤害类型与抗性级别。当前层与离层 actor 使用相同字段；目标排序、敌我计数、战术移动候选和每次施法目标结果只属于当前状态纯计算或命令事件。载入 v87 及更早存档时缺失字段迁移为空，不读取玩家抗性、不补观察、不触发移动或施法且不推进 RNG。非 smart actor、玩家拥有的召唤物或重复伤害类型携带非空记忆会被拒绝。save 容器保持 v1，新增权威记忆使 state hash 升至 Schema v38。完整边界见 [Contract v88](contract-v88-monster-targets-tactics-memory.md)。

协议 1.144 为 `ActorSaveDto` 增加可省略的 `appearanceKindId`。当前只接受带 `shadower-appearance` 标签的外观，并要求真实 actor 至少 10 级且非 Unique；载入不会重抽外观概率。save 容器保持 v1，外观状态进入 state hash Schema v67，旧开发存档不兼容。完整边界见 [Contract v196](contract-v196-warrens-content-p12-special-lifecycles.md)。

协议 1.145 为 `SavePayloadV1` 增加 `mapScale`、`wildernessPosition` 与 `wildernessSeed`。有荒野的世界必须保存非边界合法位置，并要求世界实际定义荒野。W2 复用既有楼层结构：普通荒野激活 `core.floor.wilderness`，同时只在 `storedFloors` 保存初始城镇地表；离开的普通荒野格不保存。`world` 尺度允许当前层是初始地表或该动态荒野层。三项 W1 状态继续进入 state hash Schema v68；W2 不改变 hash 输入结构，save 与 replay 容器版本保持 v1。完整边界见 [Wilderness W1](wilderness-w1-map-state-display.md) 与 [Wilderness W2](wilderness-w2-travel-local-generation.md)。

协议 1.146 增加可选 `worldTravelDestination`。它必须指向当前权威 wilderness 的非边界格，在抵达或无法寻路时清空，在伏击、返回世界图和读档之间保留；该权威字段进入 state hash Schema v69，save 容器版本仍为 v1。完整边界见 [Wilderness W5](wilderness-w5-original-extensions.md)。

协议 1.145 为当前层和离层的 `TerrainSaveDto` 增加必填逐格 `glow`。长度必须等于楼层面积；暗化后的永久房间光随楼层切换和原生存档原样恢复，并进入 state hash Schema v68。save 容器保持 v1，旧开发存档不兼容。完整边界见 [Contract v199](contract-v199-warrens-content-p15-darkness.md)。

集成协议 1.147 同时保存荒野状态、自动旅行目标与逐格永久光；相对主线 1.146 新增的 `glow` 使 state hash 升至 Schema v70，save 容器仍为 v1。

协议 1.148 在 `ItemPropertyKnowledgeSaveDto` 中增加必填 `discovered`，保存玩家已经看见或探测到的物品实例。发现状态随实例跨地面、背包、装备、怪物携带、Home 与楼层存储流转；已不存在实例的知识不会写入存档。该字段进入 state hash Schema v71，save 容器保持 v1，旧开发存档不兼容。完整边界见 [O1 物品发现](object-list-o1-item-discovery.md)。

协议 1.89 为 `PlayerSaveDto` 增加默认的 `summonCommand`。旧存档缺失时恢复为 `follow` 且无锚点；`guard` 必须携带地图内可行走锚点，其他模式必须没有锚点，否则拒绝载入。当前层和离层召唤物仍使用既有 `SummonSaveDto`；跨层跟随只移动实体及其携带物，不改变 owner/source/lifetime。save 容器保持 v1，新增权威命令状态使 state hash 升至 Schema v39。完整边界见 [Contract v89](contract-v89-friendly-summon-commands.md)。

协议 1.90 不新增存档字段：技法资源池写入既有 `PlayerSaveDto.resources`，先天能力熟练度写入 `abilityProgress`，`learnedAbilityIds` 仍只含研读所得。存档中的资源池放宽为子集匹配：缺失的池按内容 `initialFillPercent` 初始化且不抽 RNG；未知 ID、上限不符或超上限仍拒绝。无 `castingProfile` 的类不得携带 `learnedAbilityIds`。save 容器保持 v1，技法资源池与先天熟练度使 state hash 升至 Schema v40。完整边界见 [Contract v90](contract-v90-technique-resources.md)。

协议 1.91 不新增存档字段：怪物位移只改变实体/玩家位置，全部由既有字段承载，state hash 沿用 Schema v40。完整边界见 [Contract v91](contract-v91-monster-displacement.md)。

协议 1.106 为 `StatusSaveDto` 增加带默认值的 `grantedModifiers`、`grantedEquipmentBonuses` 与 `grantedStatusImmunities`；持续时间骰只在施放时消费，存档继续保存最终 `remainingTicks`，读档不重掷。Vampiric Branding 生成的 affix 通过既有物品实例 `affixIds` 保存，吸血 passive 由该权威实例派生；RandomChoice、VisibleDamage、重复 Drain Life 的分支/目标/逐击结果只属于事件和回放，不重复保存。旧档缺失新增字段时按空值迁移，不补 affix、不重抽 RNG；状态授予字段与永久 affix 使 state hash 升至 Schema v45。完整边界见 [Contract v106](contract-v106-death-third-book.md)。

协议 1.107 为 `PlayerProgressSaveDto` 增加 `maximumExperience` 与 `lifeForce`，为 `StatusSaveDto` 增加 `grantedRaceId`、`grantsWallPassage` 与 `incomingDamagePercent`。旧进度缺失/为零的历史最高经验按当前经验迁移，生命力默认 1000；旧状态默认无 Race 覆盖、不可穿墙且承受 100% 伤害。Esoteria 继续把鉴定结果写入既有 `itemPropertyKnowledge`，临时 Race 只保存状态引用，不复制派生技能或属性；Wraithform 到期时不重定位玩家。save 容器保持 v1，新增权威进度与状态字段使 state hash 升至 Schema v46。完整边界见 [Contract v107](contract-v107-death-fourth-book.md)。

协议 1.108 为地面、背包、装备和怪物携带四类物品 save DTO 增加可选 `charges { current, maximum }`，与同一个 `ItemInstance` 一起跨位置和楼层移动。当前内容声明 charged action 时，存档必须携带充能且 maximum 必须等于内容容量、current 不得超限；非充能种类携带充能或 charged kind 缺失充能均拒绝。历史内容没有 charged kind，旧档既有物品继续以无充能状态载入且不补抽 RNG。save 容器保持 v1，实例充能使 state hash 升至 Schema v47。完整边界见 [Contract v108](contract-v108-charged-items.md)。

协议 1.109 为地面、背包、装备和怪物携带四类物品 save DTO 增加可选 `activation`。动态设备必须同时保存 profile ID/名称键、生成 power、设备难度、实例成本、完整目标规格和充能；载入时逐项对照当前 `deviceGeneration` 候选，并验证 power 深度范围、maximum 容量范围及 current 上限。缺任一动态字段、静态 kind 携带 activation、profile 被替换或目标/成本/难度被篡改均拒绝。历史 built-in hash 迁移时不为已有物品补抽 activation，P58 静态 charged item 继续按固定容量规则读取。save 容器保持 v1，动态设备身份使 state hash 升至 Schema v48。完整边界见 [Contract v109](contract-v109-dynamic-devices.md)。

协议 1.110 为四类物品 save DTO 增加带默认值的 `deviceRecoveryProgress`。声明自然恢复的动态设备允许 0–999 余数；满能量时余数必须为 0，未声明恢复或没有充能的物品也不得携带非零余数。旧档缺字段按 0 迁移，载入不补恢复、不抽 RNG。主动充能只修改既有资源池、设备能量和可能被销毁的来源实例，不增加显示缓存；职业 recharge profile 的资源池继续通过 `PlayerSaveDto.resources` 保存并按既有子集迁移规则载入。save 容器保持 v1，恢复余数使 state hash 升至 Schema v49。完整边界见 [Contract v110](contract-v110-device-recharge.md)。

协议 1.111 的恢复型物品不增加存档字段。资源恢复写入既有 `PlayerSaveDto.resources`，状态清除写入既有 statuses，消耗后的数量使用现有四类物品实例，`tried`/`aware` 继续由 `itemKnowledge` 保存；缺少资源池不会在存档中创建新池。save 容器保持 v1，state hash Schema 保持 v49。完整边界见 [Contract v111](contract-v111-restorative-items.md)。

协议 1.112 的鉴定卷轴不增加存档字段。来源卷轴种类的 `aware` 写入既有 `itemKnowledge`；目标实例的 `appraised`、`identified` 与 `knownAffixIds` 写入既有 `itemPropertyKnowledge`，剩余卷轴堆叠使用原物品实例数量。旧内容 hash 迁移不补发卷轴、不补鉴定、不抽 RNG。save 容器保持 v1，state hash Schema 保持 v49。完整边界见 [Contract v112](contract-v112-scroll-identification.md)。

协议 1.113 的地图/侦测卷轴不增加存档字段。Mapping 复用当前层 `explored`，陷阱/通道侦测复用 `revealedTerrain`，actor/item 侦测结果只存在于命令事件；旧内容 hash 迁移不补地图知识、不侦测实体、不抽 RNG。save 容器保持 v1，state hash Schema 保持 v49。完整边界见 [Contract v113](contract-v113-scroll-detection.md)。

协议 1.114 为 `PlayerSaveDto` 增加可选 `recall { dungeonId, floorId, remainingTurns? }`。目的地保存稳定内容 ID，不保存 dungeon instance ID；载入时要求 floor 是同一 dungeon 的有效 dungeon floor，待触发倒计时必须为 1–2000，且当前位置必须是地表或 dungeon floor。v113 built-in 地牢存档缺字段时从当前楼层无 RNG 派生目的地，地表旧档保持 `None`。倒计时写入最终剩余行动周期，读档不重掷；返回地表后 `reset-on-surface` 实例仍按既有生命周期清除，从地表触发召回会为稳定目标楼层建立新实例。save 容器保持 v1，新增权威 recall 状态使 state hash 升至 Schema v50。完整边界见 [Contract v114](contract-v114-scroll-travel-recall.md)。

协议 1.115 为 `ItemSaveDto`、`InventoryItemSaveDto`、`EquipmentItemSaveDto` 和 `CarriedItemSaveDto` 增加可选 `enchantments { toHit, toDamage, toArmor }`。缺失字段迁移为全零，不读取内容表、不补抽 RNG；任一单项超过 +15 时以 `item enchantment state is invalid` 拒绝载入。拆分、掉落、射击弹药结算和楼层仓库必须保留强化值，堆叠只合并强化及其他运行时属性兼容的实例。save 容器保持 v1，权威实例字段使 state hash 升至 Schema v51。完整边界见 [Contract v115](contract-v115-scroll-enchantment.md)。

协议 1.116 为四类 item save DTO 增加可选 `curse`，值为 normal/heavy/permanent。缺失字段迁移为无诅咒，不读取 `initialCurse`、不补抽 RNG；载入后实例值保持权威。拆分、掉落、射击弹药结算和楼层仓库保留严重度，堆叠只合并诅咒及其他运行时属性兼容的实例。save 容器保持 v1，权威实例字段使 state hash 升至 Schema v52。完整边界见 [Contract v116](contract-v116-scroll-curses.md)。

禁止保存：

- Rust 内存布局和枚举下标；
- TypeScript UI 状态；
- RenderWorld、纹理和动画；
- 已本地化完成的系统句子；
- 临时计算缓存；
- 绝对文件路径；
- 网络令牌和崩溃报告信息。

允许保存玩家自定义名称和模组声明的用户内容，但必须限制长度。

## 5. 写入事务

桌面版执行：

1. 在同一目录创建唯一临时文件；
2. 完整写入并 flush；
3. 重新读取 header 和 checksum 做快速验证；
4. 尽平台能力执行 `fsync`；
5. 将现有正式存档轮换为 `.bak1`；
6. 原子 rename 临时文件为正式存档；
7. 保留最近 3 个备份；当前数量固定，未来再提供设置；
8. 失败时保留最后一个有效正式存档。

不得先删除旧存档再写新文件。临时文件清理由启动时的恢复流程处理。

Android 使用应用私有目录和同样的临时文件、校验、原子替换与备份流程；通过系统文件选择器进行玩家主动导入、导出和分享。各平台路径由 Tauri 适配层提供，核心存档格式不感知操作系统。

## 6. 载入与恢复

载入顺序：

1. 验证 magic、容器版本、flags 和长度；
2. 验证 payload SHA-256；
3. 解析 Header 和 MessagePack；
4. 验证 Schema 与数值上限；
5. 验证内容包集合；
6. 连续执行迁移；
7. 构建临时世界并运行不变量检查；
8. 全部成功后替换当前会话。

正式文件损坏时，按 `.bak1` → `.bak2` → `.bak3` 查找最近有效备份，并在恢复前告知玩家。损坏文件不得静默覆盖。

## 7. 迁移规则

- 迁移是 `v1 → v2 → v3` 连续函数；
- 每一步输入输出都有 fixture 和 hash；
- 增加字段必须提供默认值或可推导规则；
- ID 改名通过显式 alias 表；
- 无法无损迁移时停止并说明具体缺失内容；
- 迁移在内存中的临时副本上执行；
- 成功载入旧版本不会立刻覆盖原文件，下一次保存才写新版本；
- 发布版本不能删除仍在支持窗口内的迁移器。

## 8. 旧 C 存档导入

旧格式读取器作为隔离工具存在：

```text
crates/rfb-legacy-import/
```

导入流程输出转换报告，包括：

- 旧版本识别结果；
- 已转换字段；
- 无法转换或采用默认值的字段；
- 名称到稳定 ID 的映射；
- 内容包要求；
- 新存档 hash。

导入器只读旧文件，绝不原地覆盖。旧存档解析器必须限制字符串长度、计数和分配大小，并使用 fuzz/corpus 测试。

当前第一阶段已经实现链式 XOR 解码和 409 字节稳定前缀解析，覆盖版本、保存元数据、63 项 RNG 状态和选项位；三份本地样本通过长度、SHA-256、版本和字段级精确复验。旧 `player_type`、物品、地图等可变布局尚未进入解析范围，也不会直接映射为新核心结构。

## 9. 内容包和模组

存档记录每个包的 ID、版本、hash 和加载顺序。载入时分为：

- 完全匹配：正常载入；
- 版本不同但存在内容迁移器：迁移后载入；
- 缺失或 hash 不符：默认拒绝，展示差异；
- 用户明确进入未来的恢复模式：只在复制文件上操作，并生成不可逆警告。

## 10. 安全与隐私

- 文件大小、地图数量、实体数量、字符串和嵌套深度均设上限；
- 不解析存档内的脚本、HTML 或外部路径；
- MessagePack 未知扩展类型默认拒绝；
- 导入文件不能触发网络请求；
- Header 中的玩家文本按不可信内容转义；
- 崩溃报告上传存档必须由玩家单独确认。

## 11. v1 验收

- Windows、Linux、macOS 和 Android 原生核心读写相同 fixture；
- 保存 → 读取 → 保存得到语义相同状态和相同 state hash；
- 模拟断电不会丢失最后一个有效备份；
- 单字节损坏能被 checksum 发现；
- 截断、超大长度和畸形 MessagePack 不会 panic；
- v1 → v2 示例迁移证明连续迁移机制可用；
- 三个仅保存在本机 `.local/` 中的 `v1.3.0.7` 旧存档样本可以导入或给出结构化失败报告。

协议 1.118 不增加存档字段。历史 `RolledAffixSaveDto.passives` 中 13 个已知无规则消费者的 no-op 值在 DTO 反序列化边界丢弃，`regeneration` 与 `vampiric` 正常保留，其他未知值继续失败；迁移不重掷 affix、不替换能力、不推进 RNG。静态 affix 由既有内容 hash 迁移到当前定义。save 容器保持 v1，state hash Schema 保持 v52。完整边界见 [Contract v118](contract-v118-passive-surface-cleanup.md)。

协议 1.119 在 `PlayerSaveDto` 增加 `confusingStrikeReady`；旧存档缺字段迁移为 false，true 原样回读，不重抽 RNG。save 容器保持 v1，因该准备态进入权威 hash，state hash Schema 升到 v53。完整边界见 [Contract v128](contract-v128-scroll-monster-confusion.md)。

协议 1.121 下的 contract-v132 为 `PlayerSaveDto` 增加默认 0、零值省略的 `bonusSpellLearningCapacity`。旧存档缺字段迁移为 0；非零值要求当前 Class 明确 `usesSpellScrolls`，否则载入时拒绝。bonus 与既有 Class 学习容量公式相加并进入 state hash Schema v54；save 容器保持 v1。完整边界见 [Contract v132](contract-v132-scroll-spell.md)。

协议 1.152 为 `PlayerSaveDto` 增加必填 `activeMutationIds` 与 `lockedMutationIds`。两者按稳定 ID 排序保存；locked 必须是 active 的子集，且所有 ID 必须解析到当前内容包的 `MutationDefinition`。重复、未知或悬空锁定状态直接拒绝。两组集合进入 state hash Schema v73；save 容器仍为 v1，不为旧开发存档补默认值。完整边界见 [Contract v216](contract-v216-mutation-authoritative-state.md)。

协议 1.153 为 `PlayerProgressSaveDto` 增加必填 `attributePotentials`，保存六个原版编码的个人属性潜力。每项必须来自 `78 + 10 × 1d7`，六次骰点总和必须为 24；当前与历史最大自然属性都不得超过个人潜力和全局阶段上限。`hpProgression` 继续是唯一 HP 成长权威状态，不增加生命评级旁路字段。潜力与重掷后的 HP 序列进入 State Hash Schema v74；save 容器仍为 v1，不为旧开发存档补默认值。完整边界见 [Contract v217](contract-v217-new-life-and-attribute-potentials.md)。

协议 1.152 为全部物品位置的 save DTO 增加可选 `inscription`，并为每角色 `MogaminatorSaveDto` 增加必填 `leaveDestroyedItems`。铭文随拾取、丢弃、装备、Home、商店、怪物携带和楼层归档保持，且参与堆叠兼容性。两个字段进入 state hash Schema v73；save 容器仍为 v1，旧开发存档不兼容。

协议 1.153 为 `MogaminatorSaveDto` 增加待确认物品、已拒绝实例和每角色悬赏唯一怪物集合；所有物品位置的 save DTO 增加可选 `originActorKindId`。尸体来源参与堆叠兼容性，防止不同怪物的尸体合并后丢失 `wanted` / `unique` / `human` 判定。以上字段进入 state hash Schema v74；save 容器仍为 v1，旧开发存档不兼容。

contract-v216 不改变存档结构。每个角色继续在 `MogaminatorSaveDto` 中独立保存中文与英文规则源，界面语言只决定当前使用和编辑哪一份；恢复默认显式用对应内置模板覆盖该语言的角色副本。文本导入经既有配置命令校验成功后才成为权威状态，导出不修改存档。save 容器仍为 v1，state hash Schema 保持 v74。

协议 1.154 为每角色 `MogaminatorSaveDto` 增加必填 `autoGetMode`，合法值为 `off`、`ammo`、`wanted`，新角色默认 `off`。模式与中英文规则文本共同保存，并进入 state hash Schema v75；save 容器仍为 v1，旧开发存档不兼容。G0 尚不保存自动拾取目标或行走状态。

协议 1.155 为保存兼用的 `GoldPileDto` 增加必填 `discovered`。视野或金币探测发现后，该状态随当前层和离层楼层保存，并进入 state hash Schema v76；旧开发存档不兼容。`MogaminatorDto.autoGetTarget` 由当前地图、知识、规则和模式派生，不写入存档。

集成协议 1.157 将上述角色变异、属性潜力、物品铭文、墨家名器配置与金币发现状态共同纳入 state hash Schema v77；save 容器仍为 v1，不提供旧开发存档兼容路径。

协议 1.158 为每个 actor 保存可选的 `eldritchHorrorTriggered`。字段只区分首次
理智冲击与同一怪物的低概率重触发，不保存派生可见性或新的理智数值。它进入
state hash Schema v78；save 容器仍为 v1，不兼容旧开发存档。

协议 1.159 增加必填 `wildernessViewOffset: Position`。两个分量都必须位于
`-1..=1`；非零值只允许出现在 local 尺度的动态荒野层。该状态与既有
`wildernessPosition`、`wildernessSeed` 一起保存并进入 state hash Schema v79。
动态荒野层尺寸固定为 96×33，按 32×11 划分为 3×3 区块；save 容器仍为 v1，
旧开发存档不兼容。

contract-v232 将 `wildernessSeed` 解释为当前荒野代数种子。每次成功进入世界
地图时以 `wrapping_add(0x9E3779B97F4A7C15)` 推进一次并照常保存；同一代的
绝对区块可从 seed 与坐标重建。最多 5×5 个区块的运行时地形缓存不写入 payload，
读档后为空且不改变重建结果；存档与 State Hash Schema 版本均不变化。

contract-v233 使用既有 `wildernessPosition`、`wildernessViewOffset` 与
`wildernessSeed` 保存卷动后的权威视口。重叠地形、探索状态和仍在视口内的动态
对象继续落入既有活动层字段；派生缓存与单次更新的 `mapTranslation` 均不保存。
save 容器保持 v1，State Hash Schema 保持 v79。

contract-v234 的卷动怪物轮数由初始轮数、新暴露绝对区块集合和固定 salt 直接
计算；不保存小数余数、条带生成状态或额外 RNG。伏击继续只使用既有世界地图
状态。save 容器保持 v1，State Hash Schema 保持 v79。

contract-v235 只同步客户端临时光标、旅行目标和对象列表选择；这些状态不写入
存档，也不进入 State Hash。save 容器保持 v1，State Hash Schema 保持 v79。

contract-v236 在卷动进入城镇时把 `wildernessViewOffset` 归零，继续保存正规化后
的城镇世界坐标和既有独立城镇 FloorState。派生荒野缓存不保存，直接进入城镇也
不推进 `wildernessSeed`；save 容器保持 v1，State Hash Schema 保持 v79。

contract-v237 取消本地步行进入城镇时的独立层切换。活动地表继续保存为既有动态
荒野层，城镇原尺寸地形、探索、实体、地面物品与金币仍复用 `storedFloors` 中的
既有 `FloorState`，按可见矩形与活动视口双向同步。`wildernessPosition`、
`wildernessViewOffset` 和 `wildernessSeed` 已足以恢复视口；未新增城镇缓存或
`insideTown` 字段。save 容器保持 v1，State Hash Schema 保持 v79。

contract-v244 为 M6-A 在 `PlayerSaveDto` 增加必填 `minorSlow` 和可选
`pendingMutationDirection`。`minorSlow` 只允许 0..=10；待选方向只允许引用当前
active 的 Produce Mana 且只能出现在本地地图。两者进入 State Hash Schema v82；
`unwell` 继续复用既有状态保存。save 容器保持 v1，不兼容旧开发存档。

contract-v245 为 M6-B 在 `PlayerSaveDto` 增加 `realityChangeTicks`，合法范围为
0..=35。该倒计时与当前楼层、RNG 一起进入 State Hash Schema v83；到零时仅允许
普通程序地下城重生成。固定任务层、城镇和连续荒野不会被替换，也不会推进
`wildernessSeed`。save 容器保持 v1，不兼容旧开发存档。

contract-v260 不增加存档字段。玩家从连续荒野进入非城镇地表地牢时，当前活动
荒野层复用既有 `FloorState` 保存到 `storedFloors["core.floor.wilderness"]`；
从地牢返回时恢复该层及其世界坐标、视口偏移、实体和地面物品。载入验证只在
当前世界定义了荒野时允许这个保留 ID。save 容器保持 v1，State Hash Schema
保持 v86。

contract-v261 不增加存档字段。蘑菇店继续使用既有 `ShopStateSaveDto`，快速恢复的
定时再生继续使用既有 `StatusSaveDto`；State Hash Schema 保持 v86，save 容器保持
v1。

contract-v262 不增加存档字段。旅店住宿价格来自内容定义，城镇旅行资格直接读取既有
`TownStateSaveDto.visited`；旅行后仍使用现有 wilderness position、view offset、
FloorState 与 ShopState。State Hash Schema 保持 v86，save 容器保持 v1。

contract-v263 为 `PlayerSaveDto` 增加必需的 `name`，新游戏与伯爵府合法改名使用同一名称
验证，读档不保留旧开发存档兼容。名称进入玩家投影和状态哈希，因此 State Hash Schema
升至 v87；save 容器仍为 v1。

contract-v264 删除节奏技法和原创装置师职业充能后，`PlayerSaveDto.resources` 只接受
当前职业精确声明的资源集合，不再为已退役职业补建缺失资源池。设备自然恢复余数与
物品充能继续使用既有字段。删除的资源行为配置、职业投影和瞬时 touched 集合都不在
state hash 输入中，因此 State Hash Schema 保持 v87；save 容器仍为 v1，不兼容旧开发存档。

contract-v266 正式把玩家制造弹药的 `damageDiceOverride`、`originKind` 和
`discountPercent` 纳入物品权威存档与 State Hash Schema v88。普通物品继续省略默认值，
save 容器保持 v1；测试只从新存档开始，不增加旧开发存档迁移路径。

contract-v275 为 `PlayerProgressSaveDto` 增加必填 `weaponProficiencies`。每项只保存稳定的
规范基础物品 ID 与高于当前职业出生值的训练值；职业出生值和训练上限继续来自内容，
不会重复写入存档。载入严格拒绝字段缺失、重复 ID、未知或非武器 ID、神器/特殊变体别名、
不高于出生值或超过职业上限的记录，不提供旧开发存档迁移。该稀疏表进入 State Hash
Schema v89；Protocol 升至 1.178，save 容器保持 v1。

contract-v278 为 `PlayerProgressSaveDto` 增加必填 `miningProficiency` 与 `materials`。
挖矿熟练度范围为 0–8000；材料使用固定 `rfb.material.*` 身份的稀疏非零数组。载入严格
拒绝缺字段、熟练度越界、重复/未知材料 ID 和零数量材料，不提供旧开发存档兼容。
两者进入 State Hash Schema v90；Protocol 升至 1.180，save 容器保持 v1。

contract-v279 没有增加存档字段。碎石掉落使用既有 `ItemSaveDto.originKind` 保存新增枚举值
`rubble`；矿脉材料、熟练度和金币继续使用 contract-v278 及既有金币堆字段。Protocol
升至 1.181，State Hash Schema 保持 v90，save 容器保持 v1。
contract-v282 不增加存档字段。宠物维持从既有职业身份、玩家等级、资源池以及 actor 的
`controllerId` / `summon.ownerId` 派生；解散、消失和转敌直接写回既有 actor 集与控制归属。
State Hash Schema 保持 v92，save 容器保持 v1。

contract-v283 为 `SavePayloadV1` 增加必填 `generatedArtifactIds`，保存已经提交或由显式
任务奖励授予的正式 RFB 固定神器 ID。载入拒绝重复、未知、缺少 `artifactGeneration`
元数据的 ID，以及存在神器实例却缺少相应登记的状态；集合允许保留已经销毁的神器。
该状态进入 State Hash Schema v93，Protocol 升至 1.187，save 容器保持 v1，不提供旧
开发存档兼容默认值。

contract-v285 为 `PlayerProgressSaveDto` 增加必填 `ridingProficiency`。载入按当前职业
内容严格校验 `initial <= current <= maximum <= 8000`；无职业构筑时只允许 0，不为缺少
字段或低于职业出生值的旧开发存档补默认值。该状态进入 State Hash Schema v94，Protocol
升至 1.189，save 容器保持 v1。

contract-v289 为每个 `ActorSaveDto` 增加必填 `experience`，无进化关系的 actor 只允许 0，
有进化关系的 actor 必须低于当前形态阈值。`PlayerSaveDto.ridingBond` 也是必填字段，值为
null 或稳定 actor ID、当前 kind ID 与 0–10000 羁绊值；载入要求目标仍是存活、可骑乘且由
玩家控制的 actor，允许目标暂存于离层状态。进化保留实体 ID、控制和骑乘状态，但将新形态
羁绊重置为 0。两项状态进入 State Hash Schema v95；Protocol 升至 1.191，save 容器保持
v1，不提供旧开发存档兼容默认值。
