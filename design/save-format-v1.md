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
  terrain: TerrainSaveDto;
  player: PlayerSaveDto;
  entities: ActorSaveDto[];
  items: ItemSaveDto[];
  inventory: InventoryItemSaveDto[];
  equipment: EquipmentItemSaveDto[];
  carriedItems: CarriedItemSaveDto[];
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

协议 1.89 为 `PlayerSaveDto` 增加默认的 `summonCommand`。旧存档缺失时恢复为 `follow` 且无锚点；`guard` 必须携带地图内可行走锚点，其他模式必须没有锚点，否则拒绝载入。当前层和离层召唤物仍使用既有 `SummonSaveDto`；跨层跟随只移动实体及其携带物，不改变 owner/source/lifetime。save 容器保持 v1，新增权威命令状态使 state hash 升至 Schema v39。完整边界见 [Contract v89](contract-v89-friendly-summon-commands.md)。

协议 1.90 不新增存档字段：技法资源池写入既有 `PlayerSaveDto.resources`，先天能力熟练度写入 `abilityProgress`，`learnedAbilityIds` 仍只含研读所得。存档中的资源池放宽为子集匹配：缺失的池按内容 `initialFillPercent` 初始化且不抽 RNG；未知 ID、上限不符或超上限仍拒绝。无 `castingProfile` 的类不得携带 `learnedAbilityIds`。save 容器保持 v1，技法资源池与先天熟练度使 state hash 升至 Schema v40。完整边界见 [Contract v90](contract-v90-technique-resources.md)。

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
