# RFB CoreTransport 协议 v1

状态：协议 1.110、自动生成的 TypeScript/JSON Schema 与 `TauriNativeTransport` 已实现

## 1. 适用边界

该协议连接：

- TypeScript/PixiJS UI 与 Tauri 原生 Rust 核心；
- 测试驱动器与核心；
- 未来只读观察器和回放工具。

协议只暴露 DTO、命令、事件、快照和错误。Rust 内部 struct、指针、ECS 组件和存档对象都不是协议的一部分。

## 2. 编码决定

- 开发调试：JSON；
- Tauri 控制命令与低频 DTO：Serde JSON/结构化 IPC；
- 存档、回放和经性能分析确认的批量载荷：MessagePack；
- 协议 Schema：Rust 类型为权威定义，同时生成 JSON Schema 和 TypeScript 类型；
- 字节序：自定义二进制字段统一小端；
- 64 位整数：跨 TypeScript 边界时编码为十进制字符串或固定 8 字节，禁止直接当作 JS `number`；
- 地图批量数据允许使用 `ArrayBuffer`/TypedArray 专用载荷，不能把每格都扩展成大型 JSON 对象。

JSON 与 MessagePack 必须表达相同语义；业务逻辑不能依赖 map key 顺序或具体编码器行为。

Rust `rfb-protocol` 是协议类型的唯一权威来源：

```powershell
cargo run -p rfb-protocol --features bindings --bin generate-bindings
cargo run -p rfb-protocol --features bindings --bin generate-bindings -- --check
```

生成结果分别提交到 `web/src/protocol.ts` 和 `schemas/protocol-v1.schema.json`。前者供 TypeScript 编译器使用，后者供工具、兼容检查和未来插件接口使用；二者都禁止手工编辑，CI 会拒绝与 Rust DTO 不一致的提交。

## 3. 版本与握手

连接建立后，前端首先发送：

```ts
interface HelloRequest {
  protocol: { major: 1; minMinor: number; maxMinor: number };
  client: { name: string; version: string; platform: string };
  capabilities: string[];
}
```

核心返回：

```ts
interface HelloResponse {
  protocol: { major: 1; minor: number };
  coreVersion: string;
  sessionId: string;
  capabilities: string[];
  contentHash: string;
}
```

- major 不同：拒绝连接；
- minor 不同：选择双方共同支持的最高版本；
- 可选功能通过 capability 协商，不通过猜测版本号启用；
- 每次启动或载入游戏创建新的 `sessionId`。

## 4. 消息信封

```ts
interface ProtocolEnvelope<T> {
  protocolVersion: "1.92";
  sessionId: string;
  requestId?: string;
  commandSeq?: number;
  revision?: number;
  kind: string;
  payload: T;
}
```

协议 1.14 新增方向 `Fire` 命令、`ProjectileProfileDto` 和 `ProjectileTraceDto`。射击事件明确输出起点、落点和逐格轨迹，并继续复用结构化伤害 outcome。当前规则边界见 [Contract v14](contract-v14-projectile-foundation.md)。

协议 1.15 新增 `Throw { itemId, direction }`、发射器 `ammoKindId` 与显式 `landing`。射击弹药扣减和投掷实例移动都在核心中原子执行。当前规则边界见 [Contract v15](contract-v15-ammunition-throwing.md)。

协议 1.16 新增 `TargetSpecDto`、`TargetSelection` 和 `FireTarget`。核心声明方向/格子/实体选择模式与射程，非八方向格子或实体目标使用确定性整数路径；前端目标模式消费该规格并只在确认时提交稳定选择。当前规则边界见 [Contract v16](contract-v16-target-selection.md)和[前端目标模式 v1](frontend-targeting-v1.md)。

协议 1.17 保持 DTO 结构不变，新增弹药破损/落地事件语义并固定其 RNG 顺序；版本升级用于拒绝以 1.16 规则解释新的确定性回放。当前规则边界见 [Contract v17](contract-v17-ammunition-recovery.md)。

协议 1.18 新增 `ThrowProfileDto`，背包和装备物品输出整数重量、确定性射程及可选投掷攻击 profile；投掷命中、伤害和死亡继续通过既有结构化 outcome 与 trace 表达。当前规则边界见 [Contract v18](contract-v18-thrown-attacks.md)。

协议 1.19 在 `PlayerDto` 输出权威携带总重和内容容量，并新增整堆拾取超限事件；显示层只格式化整数磅十分位，不重新计算规则重量。当前规则边界见 [Contract v19](contract-v19-inventory-capacity.md)。

协议 1.20 新增 `ItemKnowledgeDto`，物品 DTO 输出核心决定的 `displayNameKey` 和 unknown/tried/aware 状态；未 aware 的背包/装备项不投影隐藏 modifier 与攻击 profile。当前规则边界见 [Contract v20](contract-v20-item-knowledge.md)。

协议 1.23 新增 `Appraise`、实例质量、鉴别级别与扩展后的 `ItemPropertyKnowledgeSaveDto`。`appraised` 只公开质量，`identified` 才公开完整词条；真实修正始终参与权威规则计算。当前规则边界见 [Contract v23](contract-v23-item-appraisal.md)。

协议 1.25 为存档 DTO 增加可选怪物携带物列表，并固定出生携带生成、死亡放下真实实例、随后生成普通掉落的顺序；版本升级用于拒绝以 1.24 规则解释新的状态哈希和回放。当前规则边界见 [Contract v25](contract-v25-monster-carried-items.md)。

协议 1.26 新增 `TraverseStairs`，并在快照/增量输出 `floorId`；save v1 增加当前楼层 ID 与离层状态列表。版本升级用于拒绝以 1.25 规则解释新的楼层仓库、state hash 和回放。当前规则边界见 [Contract v26](contract-v26-floor-lifecycle.md)。

协议 1.27 保持 DTO 结构不变，固定程序化楼层的深度过滤、房间怪物/掉落分配与 RNG 顺序；版本升级用于拒绝以 1.26 规则解释新的首次楼层实体集合和回放。当前规则边界见 [Contract v27](contract-v27-procedural-room-content.md)。

协议 1.28 新增方向性 `OpenDoor` / `CloseDoor`，协议 1.29 新增 `BashDoor` 与确定性开锁检定，协议 1.30 输出稳定 `terrainInteractions`。协议 1.31 新增 `Search`；普通 `CellDto` 与交互查询只输出玩家已知 terrain，秘密地形发现位置通过 save v1 持久化但不作为独立快照真值暴露。当前规则边界见 [Contract v31](contract-v31-secret-door-search.md)。

- `requestId` 用于匹配请求和响应；
- `commandSeq` 在会话内严格递增，核心拒绝重复或跳号命令；
- `revision` 表示权威游戏状态版本；
- 任何跨会话消息必须拒绝；
- 命令默认不是幂等操作，前端超时后先查询状态，不得盲目重发。

## 5. 核心 API

```ts
interface GameCoreV1 {
  hello(request: HelloRequest): Promise<HelloResponse>;
  createGame(options: NewGameOptions): Promise<GameSnapshot>;
  loadGame(data: Uint8Array): Promise<GameSnapshot>;
  saveGame(): Promise<Uint8Array>;
  exportReplay(): Promise<Uint8Array>;
  dispatch(command: GameCommandEnvelope): Promise<GameUpdate>;
  getSnapshot(request: SnapshotRequest): Promise<GameSnapshot>;
  closeSession(): Promise<void>;
}
```

`GameCommandEnvelope` 至少包含 `commandSeq`、客户端已知的 `expectedRevision` 和一个具体命令。核心只在 revision 合法时执行会改变规则状态的命令。

协议 1.40 新增 `abandon-task`。该命令只对当前 active 的一次性任务有效，并产生可回放的 `task.abandoned` 结果；任务日志状态同时新增 `abandoned`，与普通失败分开。

协议 1.41 在 save v1 增加可选 `taskProgress`，每项保存稳定 `floorId` 和 `current`。任务日志可投影大于一的 `required`；旧存档缺失计数时按空进度载入。

协议 1.42 为任务日志增加 `paused`，并新增 `task.paused`、`task.resumed` 事件。暂停和恢复仍使用普通 `traverse-stairs` 命令，不引入客户端专用状态修改入口。

协议 1.43 为 `TaskStatusDto` 增加稳定 `taskId`。save v1 的 `taskProgress` 使用 `taskId`，但继续接受旧字段名 `floorId` 并在载入时规范化。

协议 1.44 在 save v1 增加 `taskStates`，保存任务状态、进度、目标数量和当前 active floor。旧 `taskProgress` 仅作为迁移输入；新存档只写入权威任务状态。

协议 1.45 为 `TaskStatusDto` 增加一基 `stage/stages`，为 `TaskStateSaveDto` 增加零基 `stageIndex`。`current/required` 表示当前阶段而非整条任务的累计值；旧单目标任务默认是第 1/1 阶段。

协议 1.46 在 save v1 增加 `dungeonStates`，按稳定 `dungeonId` 保存最终守护者是否已击败。守护者死亡通过 `dungeon.guardian-defeated` 语义事件投影；缺失地牢状态的旧存档按未击败迁移。

协议 1.47 保持 DTO 和 save 字段不变，固定独立 vault 模板、深度加权 encounter group 与主题 loot 的生成和 RNG 顺序；版本升级用于拒绝以 1.46 规则解释新的首次楼层实体集合和回放。当前规则边界见 [Contract v47](contract-v47-themed-vault.md)。

协议 1.48 继续保持 DTO 和 save 字段不变，固定 encounter/theme 表的深度过滤、普通遭遇与楼层掉落读取、多个 Vault 的权重选择、无候选回退和同类巢穴 RNG 顺序；版本升级用于拒绝以 1.47 规则解释新的首次楼层实体集合和回放。当前规则边界见 [Contract v48](contract-v48-floor-generation-tables.md)。

协议 1.49 继续保持 DTO 和 save 字段不变，固定 `actorSlots/lootPlacements` 预算预留、普通 encounter 填充、重复楼层 loot placement 和十层主题分段的 RNG 顺序；版本升级用于拒绝以 1.48 规则解释预算化首次生成和回放。当前规则边界见 [Contract v49](contract-v49-budgeted-pressure-dungeon.md)。

协议 1.50 继续保持 DTO 和 save 字段不变，固定 Vault 变换枚举、自由落位候选顺序、多 Vault area/actor/loot 预算竞争、重叠拒绝和失败候选回退的 RNG 顺序；版本升级用于拒绝以 1.49 规则解释空间 Vault 首次生成和回放。当前规则边界见 [Contract v50](contract-v50-spatial-vault-placement.md)。

协议 1.51 继续保持 DTO 和 save 字段不变，固定动态 friends/escort 数量抽取、escort 选种、`cluster/ring` formation 候选、群体预算竞争、空间缩减和原子回退的 RNG 顺序；版本升级用于拒绝以 1.50 规则解释动态群体首次生成和回放。当前规则边界见 [Contract v51](contract-v51-dynamic-encounter-groups.md)。

协议 1.52 继续保持 DTO 和 save 字段不变，固定 terrain feature 深度过滤、权重选择、room/corridor 候选顺序、保留位置、空间失败回退及 actor/loot 占位的 RNG 顺序；版本升级用于拒绝以 1.51 规则解释特殊地形首次生成和回放。当前规则边界见 [Contract v52](contract-v52-terrain-feature-budgets.md)。

协议 1.53 继续保持 DTO 和 save 字段不变，固定 cavern 连通前沿、room shape/尺寸/位置候选、几何预算保留、房间连接和跨房间 encounter/loot 分布的 RNG 顺序；版本升级用于拒绝以 1.52 规则解释分阶段 layout 首次生成和回放。当前规则边界见 [Contract v53](contract-v53-staged-cavern-layout.md)。

协议 1.54 仍不增加 DTO 或 save 字段，固定 lake 连通前沿、深浅分层、river 边界/坐标/轴向选择、浅水岸扩展和房间/隧道覆盖顺序的 RNG 语义；版本升级用于拒绝以 1.53 规则解释首次水文生成和回放。当前规则边界见 [Contract v54](contract-v54-lake-river-hydrology.md)。

协议 1.55 继续保持 DTO 与 save 字段不变，固定 maze 根节点/邻居、destroyed 震中/前沿、streamer 权重/起点/方向/候选和稳定回退的 RNG 语义；版本升级用于拒绝以 1.54 规则解释首次 late-terrain 生成和回放。当前规则边界见 [Contract v55](contract-v55-maze-destroyed-streamers.md)。

协议 1.56 仍不增加 DTO 或 save 字段，固定 pit roster 权重抽取、等级/ID 排序、中心阶位映射、复合房间覆盖、单入口连接和 footprint 保留顺序；版本升级用于拒绝以 1.55 规则解释首次 pit 生成和回放。当前规则边界见 [Contract v56](contract-v56-classic-monster-pit.md)。

协议 1.57 仍不增加 DTO 或 save 字段；`layout.mode = maze-only`、BFS 远距锚点、路径陷阱和区域化 encounter/loot 都属于首次楼层生成规则。版本升级用于拒绝以 1.56 的“maze 叠加房间”顺序解释新生成与回放。当前规则边界见 [Contract v57](contract-v57-maze-only-floor.md)。

协议 1.58 在 save v1 新增 `FloorConnectionSaveDto`：当前层使用可选 `floorConnections`，离层 `FloorSaveDto` 使用可选 `connections`，都保存稳定连接 ID 与位置。缺失或空列表表示 v57 及更早的已生成楼层，运行时继续使用 legacy 楼梯标签；非空列表必须与内容定义和实际 terrain 完全匹配。版本升级同时固定附加连接的 RNG 落位、独立到达点与 shaft 跨层语义。当前规则边界见 [Contract v58](contract-v58-floor-connections.md)。

协议 1.59 为 `ActorSaveDto` 新增可选 `pack`，保存稳定 pack ID、leader ID、`leader/member` 角色和冻结的 `seek/surround/guard-leader` 行为。缺失字段兼容 v58 及更早存档并按独立 `seek` 行动；存在字段时必须通过 pack 内唯一 leader、引用一致性和玩家无 pack 状态校验。当前规则边界见 [Contract v59](contract-v59-pack-ai.md)。

协议 1.60 新增 `FloorRegionSaveDto`，并由 `SavePayloadV1.floorRegions` 与 `FloorSaveDto.regions` 分别保存当前层和离层区域。每个区域包含稳定 region/theme ID、局部 encounter/loot 表引用和完整格集合；缺失字段兼容 v59 及更早存档，不重建区域或推进 RNG。当前规则边界见 [Contract v60](contract-v60-regional-themes.md)。

协议 1.61 新增 `abandon-paused-task { taskId }`，让地表任务日志可以精确关闭一个 paused 共享任务；无效请求投影 `task.abandon-unavailable`。`TaskStatusDto` 新增 `retakesUsed/maxRetakes`，`TaskStateSaveDto` 新增默认值为 0 的 `retakesUsed`。有限次数只在成功恢复时递增，耗尽后的入口拒绝继续使用 `floor.transition-unavailable`。当前规则边界见 [Contract v61](contract-v61-retake-management.md)。

协议 1.62 统一区域组合生成语义：区域楼层可以与 theme/Vault、动态群体、terrain feature、pit、guardian、分阶段地貌和显式连接共存；特殊 footprint 归属宿主区域，区域 actor 的寻路保持在持久边界内。DTO、save 容器和 state hash Schema 不新增字段。当前规则边界见 [Contract v62](contract-v62-regional-composition.md)。

协议 1.63 新增内容侧 `DungeonDefinition`，把显式楼层连接约束为单根树，并允许多个程序化最终叶层共享同一守护者 actor kind。每个叶层使用不同镜像实例 ID；任一镜像被击败后只结算一次地牢征服，并移除其他已生成镜像、抑制尚未生成镜像。协议 DTO、save 容器和 state hash Schema 不新增字段。当前规则边界见 [Contract v63](contract-v63-dungeon-tree-guardian-mirrors.md)。

协议 1.64 将内容侧 Vault 入口规范化为 1–8 个 `entrancePositions`，并固定模板内部连通校验、每入口最长 12 格的四向 BFS connector、整层连通证明和原子失败回退。旧 `entrancePosition` 继续可读并迁移为单元素列表。协议 DTO、save 容器和 state hash Schema 不新增字段；版本升级用于拒绝以 1.63 的单入口落位规则解释首次楼层生成与回放。当前规则边界见 [Contract v64](contract-v64-multi-entry-vault-connectivity.md)。

协议 1.65 新增可选 `currentDungeonInstanceId`、`FloorSaveDto.dungeonInstanceId` 和 `DungeonStateSaveDto.nextInstanceOrdinal`。地表/任务层使用空实例，dungeon 层按 `<dungeonId>.instance.<ordinal>` 分配并在同实例上下楼传递；离层仓库键由实例+floor 组成，回地表只清理当前实例。v64 旧存档缺失字段时迁移为首实例，不补生成、不推进 RNG。snapshot/update 同步暴露实例 ID，state hash 升至 Schema v24。当前规则边界见 [Contract v65](contract-v65-dungeon-instance-identity.md)。

协议 1.66 为 `FloorConnectionSaveDto` 增加可选 `targetFloorId` 与 `targetConnectionId`。楼层连接可由内容 `targetCandidates` 按权重无放回解析为实例级探索树；首次到达动态目标时，目标连接的返回目标写入实际父连接。v65 及更早存档缺字段时使用内容固定目标，不重建楼层或推进 RNG。普通 dungeon 回地表继续清理当前实例；save 容器仍为 v1，state hash 升至 Schema v25。当前规则边界见 [Contract v66](contract-v66-dynamic-exploration-tree.md)。

协议 1.67 为内容侧 `DungeonDefinition` 增加可选 `entranceGuardian` 与 `entryRequirements`，并增加 `GuardPosition` pack behavior。入口守卫固守地表位置但不阻止楼梯交互；硬条件支持任务状态、前置 dungeon 征服和携带物数量，并在实例序号与生成 RNG 之前原子检查。`DungeonStateSaveDto` 增加可选 `entranceGuardianDefeated`；v66 及更早存档缺字段时抑制新守卫，不补实体或推进 RNG。save 容器仍为 v1，state hash 升至 Schema v26。当前规则边界见 [Contract v67](contract-v67-dungeon-entrance-guardians.md)。

协议 1.68 增加可选世界 `campaign` 定义、`GameCommand.Retire`、`CampaignStateDto` 和 `CampaignStateSaveDto`。`GameSnapshot`/`GameUpdate` 都携带 campaign 状态；事件增加胜利、退休和不可退休投影。只有 campaign victory dungeon 全部征服后才进入 victorious，只有 victorious 且位于地表才可退休；退休保存最终分数并拒绝后续命令。`SavePayloadV1.campaignState` 缺失时按旧 dungeon 状态推导，save 容器仍为 v1，state hash 升至 Schema v27。完整边界见 [Contract v68](contract-v68-victory-retirement-scoring.md)。

协议 1.69 为 `DungeonStateSaveDto` 增加可选 `retainedInstanceId` 与 `retainedAtTurn`，并为内容 `DungeonDefinition` 增加 `instanceLifecycle`（`reset-on-surface`、`persistent`、`turn-ttl`）。返回地表的 dungeon 实例按策略清理或保留；TTL 在下一次进入时按回合差惰性淘汰。v68 及更早存档缺失 retained 字段时按默认清理迁移，不生成内容或推进 RNG；state hash 升至 Schema v28，save 容器仍为 v1。完整边界见 [Contract v69](contract-v69-configurable-instance-lifecycle.md)。

协议 1.70 为 `PlayerSaveDto` 增加可选 `progress`，其中保存六维自然属性、经验、当前/历史最高等级、待分配属性点和独立 HP 成长序列；`PlayerDto`/`GameSnapshot` 暴露 `PlayerProgressDto`，包含阶段等级上限、属性上限、18/xx 桶索引、下一等级阈值和装备合并后的有效属性。新增 `IncreaseAttribute` 命令及属性增加/不可用事件；命令不推进世界脉冲。缺少 `progress` 的旧存档按确定性 legacy 序列迁移，胜利/退休存档载入时会先结算封顶经验。state hash 升至 Schema v29，save 容器仍为 v1。完整边界见 [Contract v70](contract-v70-rfb-character-progression.md)。

协议 1.71 为内容和玩家 DTO 增加构筑基础：`PlayerDto.build` 暴露 `PlayerBuildDto`，其中包含 build/Race/Class/Personality 身份和合并后的生命、经验倍率；`PlayerProgressDto.skills` 暴露技能当前值、最大值、base 与每十级成长值。`PlayerSaveDto.build` 和 `PlayerProgressSaveDto.skills` 保存权威身份与聚合结果。v70 及更早存档缺少这些字段时按世界默认构筑迁移，技能按当前等级和内容确定性重算；不一致的构筑或技能状态拒绝载入。state hash 升至 Schema v30，save 容器仍为 v1。完整边界见 [Contract v71](contract-v71-rfb-character-builds.md)。

协议 1.72 新增 `CheckOutcomeDto`、`CheckResolutionDto` 和 `GameEventOutcomeDto.check`，让 device、saving-throw、stealth、perception 事件携带技能 ID、ability、difficulty、百分位骰、对抗骰、阈值与结果。`EntityDto.alerted` 暴露怪物警戒状态，`ActorSaveDto.alerted` 以可选字段保存；旧存档缺字段时按 actor 内容默认值恢复。警戒状态进入 state hash Schema v31，save 容器仍为 v1。完整边界见 [Contract v72](contract-v72-observable-skill-checks.md)。

协议 1.73 新增 `StudyAbility { bookItemId, abilityId }` 与 `CastAbility { abilityId, target }`，以及 `ResourcePoolDto`、`AbilityDto`、`AbilityCastResolutionDto` 和 `GameEventOutcomeDto.ability-cast`。`PlayerDto` 暴露资源池、已学状态、书本实例、当前失败率与学习/施放可用性；拒绝原因和施法成功/失败、落空、命中、击杀均使用结构化事件。`PlayerSaveDto` 保存资源池与已学能力 ID，旧存档缺字段时确定性恢复满资源和空已学列表。state hash 升至 Schema v32，save 容器仍为 v1。完整边界见 [Contract v73](contract-v73-ability-books.md)。

协议 1.74 新增 `Rest { turns }`、稳定 `TargetSelection.self`、`ResourceRecoveryResolutionDto`、`RestResolutionDto`、`RestStopReasonDto`，以及 `GameEventOutcomeDto.resource-recovery/rest`。`ResourcePoolDto` 输出带兼容默认值的等待/休息恢复量；普通 `Wait` 在调度完成且玩家仍存活时应用等待恢复。`Rest` 最多请求 100 回合，每步真实运行能量调度器，并在资源已满、可见敌人、受伤或死亡时停止；治疗能力复用既有 `HealResolutionDto`。state hash 升至 Schema v33，save 容器仍为 v1。完整边界见 [Contract v74](contract-v74-resource-recovery-and-healing.md)。

协议 1.75 为 `AbilityDto` 增加基础/实际资源成本、熟练度、熟练等级、成功/失败统计和冷却字段；`AbilityCastResolutionDto` 输出施法前后进度；`PlayerSaveDto` 增加 `abilityProgress`。内容能力可声明 `proficiency` 与可选 `cooldown`/`groupId`。熟练度采用 RFB 五档阈值 `0/900/1200/1400/1600`，影响 Mana 成本和 Expert/Master 失败率；成功施法增加熟练度/次数，失败增加失败次数，冷却拒绝在扣资源与 RNG 前返回。缺少 `abilityProgress` 的旧存档按当前内容初值迁移，不推进 RNG。state hash 升至 Schema v34，save 容器仍为 v1。完整边界见 [Contract v75](contract-v75-ability-proficiency-and-cooldowns.md)。
协议 1.76 为 Class casting profile 增加独立学习容量公式，`PlayerDto` 输出 `abilityLearning` 的已学数量/容量/剩余槽位，`AbilityDto` 增加 `canForget`，并新增 `ForgetAbility { abilityId }`。容量满、重复遗忘和其他学习/遗忘前置拒绝都在 RNG 前原子返回；遗忘只移除已学集合，保留 `abilityProgress`，重新学习恢复熟练度、统计和冷却。save 容器仍为 v1，已有 state hash Schema v34 继续覆盖权威已学集合与能力进度。完整边界见 [Contract v76](contract-v76-learning-capacity-and-forgetting.md)。

协议 1.77 新增 `AbilityEffectDefinition.area-damage` 的 DTO 投影 `areaRadius`、`AbilityAreaDamageResolutionDto` 与 `GameEventOutcomeDto.ability-area-damage`。范围能力使用稳定 `TargetSelection`：定点目标穿过中途 actor，方向目标在首个 actor 停止；墙体阻断爆发，按 RFB `distance()` 与 `(baseDamage + distance) / (distance + 1)` 由内向外结算。目标验证仍在资源/RNG/熟练度之前，空爆保留成功施法与单次基础伤害骰；save 容器与 state hash Schema v34 不变。完整边界见 [Contract v77](contract-v77-area-damage.md)。

协议 1.78 新增 `AbilityEffectDefinition.beam-damage` 的 DTO 投影 `beamDamage`、`AbilityBeamDamageResolutionDto` 与 `GameEventOutcomeDto.ability-beam-damage`。首版只接受方向目标；射线穿过 actor，按固定八向逐格推进并在墙体/边界截断，所有路径目标按近到远顺序复用既有伤害管线且共享一次基础伤害骰。方向以外的目标模式在资源/RNG/熟练度之前拒绝，空射仍保留成功施法、资源消耗和单次伤害骰；save 容器与 state hash Schema v34 不变。完整边界见 [Contract v78](contract-v78-beam-damage.md)。

协议 1.79 新增 `AbilityEffectDefinition.cone-damage` 的 DTO 投影 `coneRadius`、`AbilityConeDamageResolutionDto` 与 `GameEventOutcomeDto.ability-cone-damage`。首版只接受方向目标；锥形沿固定八向中心线逐层从宽度 0 展开到配置半径，actor 不阻挡，墙体/边界截断，候选格按近到远、横向距离和坐标稳定排序，侧向目标复用整数衰减并共享一次基础伤害骰。方向以外的目标模式在资源/RNG/熟练度之前拒绝，空锥仍保留成功施法、资源消耗和单次伤害骰；save 容器与 state hash Schema v34 不变。完整边界见 [Contract v79](contract-v79-cone-damage.md)。

协议 1.80 扩展既有 `AbilityEffectDefinition.beam-damage` 的目标入口，不新增伤害字段；`AbilityDto.targetSpec` 继续投影 `direction`、`position` 与 `entity` 模式，`AbilityBeamDamageResolutionDto` 和 `GameEventOutcomeDto.ability-beam-damage` 继续复用。定点/实体目标必须存在、可见且不超距，核心沿稳定整数斜率经过目标继续到内容射程，actor 不阻挡，墙体/不可行走地形/边界截断，按近到远结算并共享一次基础伤害骰。自身、缺失、不可见和超距目标在 Mana/RNG/熟练度之前拒绝；save 容器与 state hash Schema v34 不变。完整边界见 [Contract v80](contract-v80-targeted-beam-extension.md)。

协议 1.81 新增 `AbilityEffectDefinition.teleport` 的 DTO 投影 `AbilityDto.teleport`、`AbilityTeleportResolutionDto` 与 `GameEventOutcomeDto.ability-teleport`。首版只接受 `position` 目标；落点必须非当前格、在地图内、可见、满足 line of effect、可行走且无存活 actor 占据。落点验证在 Mana、施法 RNG 和熟练度前完成；成功后精确移动并复用普通移动的被动感知、陷阱触发和死亡处理。save 容器与 state hash Schema v34 不变。完整边界见 [Contract v81](contract-v81-teleport-ability.md)。

协议 1.82 新增 `AbilitySummonSpecDto` 与 `AbilityDto.summon`，以及 `AbilitySummonResolutionDto` 和 `GameEventOutcomeDto.ability-summon`。`EntityDto` 新增带兼容默认值的 `faction` 与可选 `SummonDto`；`ActorSaveDto` 新增可选 `SummonSaveDto`，保存所有者、源能力和剩余回合。首版召唤只接受 `self` 目标，空间不足在 Mana/RNG/熟练度前原子拒绝；成功生成稳定 actor 实例，失败率失败仍支付资源但不生成实体。召唤物不参加敌对怪物 AI或可见敌人判断，并按玩家回合到期移除。save 容器仍为 v1，state hash 升至 Schema v35。完整边界见 [Contract v82](contract-v82-summon-ability.md)。

协议 1.83 新增 `AbilityDetectSpecDto` 与 `AbilityDto.detect`，以及 `AbilityDetectResolutionDto` 和 `GameEventOutcomeDto.ability-detect`。首版侦测只接受 `self` 目标，并按内容 category/radius 筛选当前 FOV 内具有隐藏投影的 terrain；结果按距离、`y`、`x` 稳定输出。`persistent` 结果写入 `revealedTerrain` 并通过 `changedCells` 返回，瞬时结果只存在于 `ability.detect` outcome。空结果仍是合法施法，非法目标和资源不足在 RNG 前拒绝；save 容器仍为 v1，state hash 升至 Schema v36。完整边界见 [Contract v83](contract-v83-detection-ability.md)。

协议 1.84 新增 `AbilityTerrainTransformSpecDto` 与 `AbilityDto.terrainTransform`，以及 `AbilityTerrainTransformResolutionDto` 和 `GameEventOutcomeDto.ability-terrain-transform`。首版地形改变只接受 `position` 目标，返回中心、半径、规范化来源 terrain 集、目标 terrain 和稳定排序的 `transformedPositions`。实际修改格同步进入 `changedCells`；非法/超距目标和资源不足在 RNG 前拒绝，失败不产生地形 outcome，空结果成功仍返回结构化 outcome。save 容器继续为 v1，terrain 原本已进入 save/hash，因此 state hash 保持 Schema v36。完整边界见 [Contract v84](contract-v84-terrain-transform-ability.md)。

协议 1.85 新增 `AbilityEffectSpecDto`、`AbilityStatusStackingDto` 与 `AbilityDto.effects`，把所有能力投影为有序效果列表；旧的 area/beam/cone/teleport/summon/detect/terrainTransform 专用字段继续兼容。`AbilityEffectResolutionDto`、`AbilityEffectsResolutionDto`、`AbilityStatusChangeDto` 和 `AbilityEffectSkipReasonDto` 为 `GameEventOutcomeDto.ability-effects` 返回逐效果索引、伤害/治疗/状态结果、抗性缩时、免疫、`no-target` 与 `target-dead`。整次施法仍只支付一次资源并先抽一次失败率；子效果按顺序执行，前序击杀会跳过后续效果及其 RNG。save v1/state hash Schema v36 保持不变。完整边界见 [Contract v85](contract-v85-ordered-status-effects.md)。

协议 1.86 新增 `MonsterAbilityDecisionResolutionDto`、`MonsterAbilityCastResolutionDto` 与对应 `monster-ability-decision` / `monster-ability-cast` outcome，返回施法者、百分比频率及骰值、稳定可用能力、权重选择和逐效果结果。`EntityDto` 与 `ActorSaveDto` 增加 `castingCooldownRemaining`；成功施法按 `ceil(100 / frequencyPercent)` 设置自身行动冷却，冷却行动不抽施法 RNG。save 容器仍为 v1，新增权威冷却状态使 state hash 升至 Schema v37。完整边界见 [Contract v86](contract-v86-monster-casting-ai.md)。

协议 1.87 新增 `MonsterAbilityRejectionReasonDto`、`MonsterAbilityCandidateResolutionDto` 和 `MonsterAbilityDecisionResolutionDto.candidates`，逐候选返回基础/有效权重、主目标、稳定 footprint 与拒绝原因。`MonsterAbilityCastResolutionDto` 增加 `affectedPositions` 和可选 summon resolution；怪物自身法术/召唤不再伪造 projectile trace。敌对召唤继续使用既有 `SummonDto/SummonSaveDto`，由非玩家 owner 投影 hostile 阵营。没有新增存档字段，save v1/state hash Schema v37 不变。完整边界见 [Contract v87](contract-v87-monster-casting-utility.md)。

协议 1.88 为候选增加 `enemyTargetCount/friendlyRiskCount`，新增 `MonsterAbilityTargetResolutionDto` 并通过 `MonsterAbilityCastResolutionDto.targets` 返回每个实际命中玩家阵营目标的逐效果结果。`EntityDto/ActorSaveDto.observedPlayerResistances` 保存 smart caster 只从实际结算学习到的有限玩家抗性；缺失字段兼容为空。玩家召唤物成为法术、追踪和近战目标，保持距离与受伤撤退通过普通移动事件投影。save 容器仍为 v1，新增抗性记忆使 state hash 升至 Schema v38。完整边界见 [Contract v88](contract-v88-monster-targets-tactics-memory.md)。

协议 1.89 增加 `SetSummonCommand`、`SummonCommandModeDto`、`SummonCommandDto` 和结构化 `SummonCommandResolutionDto`。`PlayerDto/PlayerSaveDto.summonCommand` 暴露 Follow、Attack、Keep Distance、Guard 与可选 Guard 锚点；命令不推进世界时间。友方召唤物行动投影为 `combat.summon-*`，跨层结果投影为 `summon.followed-floor/could-not-follow`。旧存档默认 Follow；命令状态进入 state hash Schema v39。完整边界见 [Contract v89](contract-v89-friendly-summon-commands.md)。

协议 1.90 增加 `ResourceGainSourceDto`、`ResourceGainResolutionDto` 与事件 `resource.gained`；`ResourcePoolDto` 暴露可选的 `meleeHitGainAmount/meleeKillGainAmount/turnDecayAmount`，`AbilityDto.innate` 标记先天技法能力。技法能力沿用 `CastAbility` 与既有拒绝事件；获得与衰减不新增命令或事件之外的入口。内容包升至 1.81.0；技法资源池与先天熟练度进入 state hash Schema v40。完整边界见 [Contract v90](contract-v90-technique-resources.md)。

协议 1.91 增加 `MonsterDisplacementResolutionDto` 与事件 `monster.blinked`、`monster.teleported`、`monster.dragged-target`；`AbilityEffectSpecDto` 增加 `blink-self`/`teleport-self`/`teleport-target` 三个规格。位移是怪物施法专用形态，不新增命令；内容包升至 1.82.0，state hash 沿用 Schema v40。完整边界见 [Contract v91](contract-v91-monster-displacement.md)。

协议 1.92 增加事件 `status.confused-move`（args: intended/actual 方向 token）与 `status.paralyzed`（args: status）；`ability.cast-unavailable` 的开放 reason 集合新增 `confused`。无新增 DTO；三个新状态种类（`rfb.status.confusion`/`rfb.status.blindness`/`rfb.status.paralysis`）沿用既有 `StatusDto`。内容包升至 1.83.0，state hash 沿用 Schema v40。完整边界见 [Contract v92](contract-v92-status-family.md)。

协议 1.106 为 Death 第三册增加随机效果与持久装备状态的通用表面：`AbilityEffectSpecDto` 新增 `random-choice`、`no-op`、`visible-damage` 和 `enchant-equipped-weapon`，`drain-life` 增加 `repeat`，固定 `summon` 增加 `hostile`；逐效果结果返回随机分支、明确空操作和永久附魔事务。`ApplyStatus`/`StatusDto`/`StatusSaveDto` 增加随机持续骰、属性修正、装备加值与状态免疫，装备 passive 新增 `vampiric`。永久 affix、状态授予字段与吸血结算进入 save/replay/state hash Schema v45；旧字段缺失时按空值迁移。完整边界见 [Contract v106](contract-v106-death-third-book.md)。

协议 1.107 为 Death 第四册增加 `item` 目标模式及稳定 `itemId` 选择，并扩展 `AbilityEffectSpecDto`/逐效果结果：新增 `death-ray`、`identify-item`、`restore-vitality`，`genocide` 增加 `nearby` 与半径，`summon-category` 增加升级类别、敌友/群体概率和敌对 unique 开关，`apply-status` 增加临时 Race、穿墙和入伤比例。`PlayerProgressDto` 增加历史最高经验与生命力，状态 DTO 保存形态/穿墙/入伤字段；这些权威状态进入 save/replay/state hash Schema v46。完整边界见 [Contract v107](contract-v107-death-fourth-book.md)。

协议 1.108 增加 `ItemChargesDto { current, maximum }` 和 `InventoryItemDto.charges`。精确充能只在种类知识为 aware 时出现；`usable` 同时考虑使用动作和当前充能是否足够，因此耗尽设备即使未鉴定也不能发送有效使用操作。成功/失败继续复用设备检定事件，效果继续复用 healing resolution；扣费后的充能由同一 update 背包投影返回，不引入第二套显示状态。实例充能进入 save/replay/state hash Schema v47。完整边界见 [Contract v108](contract-v108-charged-items.md)。

协议 1.109 为 `UseItem` 增加可选 `target`，增加 `ItemActivationDto { profileId, nameKey, power, cost, deviceCheckDifficulty, targetSpec }`，并为背包 DTO 增加可选 `activation` / `useTargetSpec`。activation 与精确充能只在种类 aware 时出现；目标规格始终投影，以便未知设备进入既有 direction/position/entity/self 选择器。错误目标在设备检定前零 RNG 拒绝，成功伤害/击杀/落空/侦测使用结构化设备事件；静态 P58 `useAction` 保持兼容。动态实例状态进入 save/replay/state hash Schema v48。完整边界见 [Contract v109](contract-v109-dynamic-devices.md)。

协议 1.110 增加 `RechargeItem { targetItemId, source }`，其中来源为职业 `resource` 或另一件 `item`；`PlayerDto.deviceRecharge` 投影资源 ID/power，背包 DTO 增加 `canReceiveRecharge` / `canSupplyRecharge`。合法充能消耗普通行动并返回结构化成功/失败事件；无 profile、目标/来源非法或资源为空时返回不可用事件，不推进 world tick、不抽 RNG。自然恢复只通过能量恢复事件投影，确定性余数不进入运行时背包 DTO。恢复余数及充能结果进入 save/replay/state hash Schema v49。完整边界见 [Contract v110](contract-v110-device-recharge.md)。

当前命令集包括八向 `Move`、`Wait`、`Rest`、物品/装备操作、terrain 交互、楼层/任务/campaign 操作、`Fire`、`FireTarget`、`Throw`、`StudyAbility` 和 `CastAbility`。`StudyAbility` 以稳定书本实例和能力 ID 学习，不消耗书本；`CastAbility` 提交稳定 `TargetSelection`，通过前置检查后原子扣除资源并投影失败率结果。命令先转换为 `GameAction`；普通行动消耗 100 能量并增加一个玩家 `turn`，已知充能不足的设备使用按原版语义不消耗能量或推进 world tick。`Rest` 是确定性宏命令：revision 和命令序号只前进一次，`turn` 增加实际完成回合数且至少增加 1，每个完成回合都通过同一调度器推进世界脉冲。

UI 本地操作，例如展开面板、滚动消息、移动相机和播放动画，不发送到核心。

`exportReplay()` 导出当前新游戏或最近一次载入存档之后的成功命令段，使用正式 `.rfbreplay` 容器。失败命令不进入记录；回放不包含完整初始存档、玩家姓名或本地路径，因此复验载入后的回放时仍需要具有相同 state hash 的初始状态。

## 6. 更新与快照

```ts
interface GameUpdate {
  baseRevision: number;
  revision: number;
  turn: number;
  worldTick: number;
  events: GameEventDto[];
  renderDelta?: RenderDeltaDto;
  uiDelta?: UiDeltaDto;
  stateHash: string;
}
```

`GameEventDto[]` 是领域事件的前端投影，不是核心内部事件模型。核心使用强类型 `DomainEvent` 保留伤害、数量、物品种类、槽位和来源/目标等语义字段，并在构建 `GameUpdate` 时一次性转换；前端不得根据 message key 反推规则结果。

要求：

- `baseRevision` 必须等于前端当前 revision；
- revision 必须连续增长；
- 前端发现缺口、乱序、未知实体或 hash 不一致时，停止应用增量并请求完整快照；
- `GameSnapshot` 必须足以重建全部权威 UI 和 RenderWorld；
- 动画进度、粒子和相机插值不属于权威快照；
- 大地图快照可按 chunk 分片，但必须带 snapshot ID、分片序号和总数。

`RenderDelta` 中删除操作先于新增/更新操作应用，同一个 revision 内的排序规则必须固定。

## 7. ID 规则

- 内容定义：稳定字符串 ID，例如 `monster.dragon.red`；
- 运行时实体：会话内不复用的 64 位 ID，跨 TypeScript 边界使用字符串；
- 存档实体：保存稳定 ID 和实例 ID，不保存数组下标；
- 消息、命令、错误和 capability 都使用命名空间字符串；
- ID 一旦进入已发布存档或内容包，不能静默改名，必须提供 alias 或迁移器。

## 8. 错误模型

```ts
interface ProtocolError {
  code: string;
  category: "protocol" | "validation" | "game-rule" | "io" | "content" | "internal";
  messageKey: string;
  args?: Record<string, unknown>;
  retryable: boolean;
  diagnosticsId?: string;
}
```

核心不把 Rust panic、文件路径或英文拼接句子直接展示给玩家。技术细节进入本地日志；用户消息通过本地化 `messageKey` 生成。

核心 panic、Tauri command 失败或事件通道断开后，前端必须把会话标为不可继续，避免在未知状态下重复执行命令。

## 9. 协议兼容规则

minor 版本允许：

- 增加带默认行为的可选字段；
- 增加 capability 控制的新消息；
- 增加前端可以忽略的事件类型。

major 版本要求：

- 删除或重命名字段；
- 改变字段语义；
- 改变命令执行顺序；
- 修改 ID 或 revision 基本规则。

所有 DTO 必须拒绝未知的必需字段值，但应按 Schema 规则忽略未知可选字段。

## 10. 测试门槛

- Rust → JSON → TypeScript fixture；
- TypeScript → MessagePack → Rust fixture；
- Windows、Linux、macOS 和 Android 原生核心对相同命令流产生相同 state hash；
- `TauriNativeTransport` 与直接 Rust 测试驱动器产生相同 DTO；
- revision 缺口触发完整重同步；
- 重复 commandSeq 不会重复执行；
- 未知 capability 和可选字段保持向前兼容；
- fuzz 测试不会因畸形消息 panic 或越界分配；
- 已提交的 TypeScript/JSON Schema 与 Rust DTO 发生漂移时由 CI 阻止；
- 协议 Schema 的破坏性变化需要在后续兼容性检查中显式批准。
