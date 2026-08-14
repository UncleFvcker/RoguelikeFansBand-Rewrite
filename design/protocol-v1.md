# RFB CoreTransport 协议 v1

状态：协议 1.218、自动生成的 TypeScript/JSON Schema 与 `TauriNativeTransport` 已实现

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

协议 1.111 固定恢复型物品的结构化事件：状态清除按成功与 no-effect 区分 kind/message key，资源恢复沿用 `GameEventOutcomeDto::ResourceRecovery` 并携带 before/after/recovered；有序序列保持事件声明顺序。协议 DTO 没有新增变体，版本升级用于锁定事件词汇、内容行为和 active baseline；state hash Schema 保持 v49。完整边界见 [Contract v111](contract-v111-restorative-items.md)。

协议 1.112 新增 `ItemIdentifyResolutionDto { itemId, itemKindId, full, changed }` 与 `GameEventOutcomeDto::ItemIdentify`，固定卷轴普通/完整鉴定的结构化结果。`UseItem.target` 继续复用既有 `TargetSelection.item`；背包 DTO 的 `useTargetSpec` 对鉴定卷轴投影 item-only 目标。错误、缺失和自身目标在消耗、RNG 与 world tick 前返回不可用事件。鉴定结果继续使用既有物品知识 DTO，state hash Schema 保持 v49。完整边界见 [Contract v112](contract-v112-scroll-identification.md)。

协议 1.113 扩展既有侦测结果以承载静态物品效果：terrain Mapping/陷阱/通道继续通过 `changedCells` 与 `AbilityDetectResolutionDto.detectedPositions` 返回，actor/item 通过稳定 `detectedEntityIds` 和位置返回；item ID 与 actor ID 共用该通用字段但由事件来源和内容主体区分。`throughWalls` 只属于内容行为，不新增运行时 DTO 字段。没有新增存档字段，state hash Schema 保持 v49。完整边界见 [Contract v113](contract-v113-scroll-detection.md)。

协议 1.114 新增 `RecallStateDto { dungeonId, floorId, remainingTurns? }`，并作为可选 `PlayerDto.recall` / `PlayerSaveDto.recall` 投影稳定目的地与倒计时。随机物品传送复用 `AbilityTeleportResolutionDto` 与 `GameEventOutcomeDto::AbilityTeleport`；跨层、启动/取消/重设/触发召回使用 `item.use-teleported-level`、`item.recall-started/cancelled/reset/triggered` 结构化事件，并和楼梯共用既有 floor transition 事件。错误目标、无合法落点、无跨层目标或地表无召回目的地均在消费、RNG 与 world tick 前返回 `item.use-unavailable`。新增权威 recall 状态使 state hash 升至 Schema v50；save 容器仍为 v1。完整边界见 [Contract v114](contract-v114-scroll-travel-recall.md)。

协议 1.115 新增 `ItemEnchantmentsDto { toHit, toDamage, toArmor }`，并投影到地面、背包、装备及四类 item save DTO。`GameEventOutcomeDto::ItemEnchantment` 返回三个 `ItemEnchantmentComponentResolutionDto`，分别记录 attempts、successes、before、after；成功与全失败事件为 `item.use-enchanted` / `item.use-enchantment-failed`。错误目标在消费、RNG 与 world tick 前返回 `item.use-unavailable`。新增权威实例状态使 state hash 升至 Schema v51；save 容器仍为 v1。完整边界见 [Contract v115](contract-v115-scroll-enchantment.md)。

协议 1.116 新增 `ItemCurseSeverityDto`，并将可选 `curse` 投影到地面、背包、装备和四类 item save DTO。`GameEventOutcomeDto::ItemCurse` 返回目标、before/after 与 artifact resisted；`ItemCurseRemoval` 返回 includeHeavy、已解除 ID 和保留 permanent ID。卸下或替换诅咒装备返回 `item.unequip.cursed`，且不推进 RNG/world tick。新增权威实例状态使 state hash 升至 Schema v52；save 容器仍为 v1。完整边界见 [Contract v116](contract-v116-scroll-curses.md)。

协议 1.117 新增 `GameEventOutcomeDto::ItemSummon`，复用 `AbilitySummonResolutionDto` 返回 owner、解析后的 category、敌友/群体结果、实体 ID、落位和实际 actor kind ID。静态使用与设备激活分别区分成功及零结果事件；只有实际生成实体才让来源种类 Aware。永久 Pet/Kin 继续通过既有 entity save 的 `controllerId` 持久化，不增加新存档字段；save v1 与 state hash Schema v52 均不变。完整边界见 [Contract v117](contract-v117-scroll-summoning.md)。

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

协议 1.118 将 `EquipmentPassiveDto` 收缩为已有权威消费者的 `regeneration` 与 `vampiric`。13 个从未影响规则的历史值不再出现在快照、内容属性或 TypeScript 联合类型中；旧 rolled-affix save 的兼容过滤由存档 DTO 边界负责，未知字符串仍拒绝。save v1 与 state hash Schema v52 保持不变。完整边界见 [Contract v118](contract-v118-passive-surface-cleanup.md)。

协议 1.119 为 `PlayerDto` 和 `PlayerSaveDto` 增加带 false 默认值的 `confusingStrikeReady`。该字段是玩家下一次合格近战命中的权威准备态，不属于 `StatusDto`，也不进入共享 Actor。字段参与存档、回放和 state hash，Schema 升到 v53。完整边界见 [Contract v128](contract-v128-scroll-monster-confusion.md)。

协议 1.120 为物品 glyph 选择新增窄 `UseItemByGlyph { itemId, glyph }` 命令，并为 `InventoryItemDto` 增加省略式 `requiresTargetGlyph`。核心入口立即归一到既有物品使用动作，不扩展通用 `TargetMode`；glyph 是瞬时命令输入，不进入存档或 state hash，Schema 保持 v53。完整边界见 [Contract v130](contract-v130-scroll-genocide.md)。

协议 1.121 为 Recharging 卷轴新增窄 `UseItemForRecharge { itemId, sourceItemId, targetItemId }` 命令，并为 `InventoryItemDto` 增加省略式 `requiresRechargeTargets`。三个 ID 只描述一次背包物品事务，不进入存档或 state hash，也不扩展通用 `TargetSelection`；Schema 保持 v53。完整边界见 [Contract v131](contract-v131-scroll-recharging.md)。

contract-v132 不增加运行时命令或快照 DTO，继续复用 `UseItem` 与既有 `AbilityLearningDto`。`PlayerSaveDto` 增加默认 0、零值省略的 `bonusSpellLearningCapacity`；它是权威持久状态并使 state hash Schema 升至 v54，但 `PROTOCOL_VERSION` 保持 1.121。完整边界见 [Contract v132](contract-v132-scroll-spell.md)。

协议 1.130 为 `ShopCategoryDto` 增加 `magic-shop`，使第四家 Outpost 商店在快照与更新中保持严格类别。设备购买、使用、能量、交易事件和存档继续复用既有 DTO；城墙、城门和地牢位置仍来自内容与地图格，不新增协议字段。save v1 与 state hash Schema v60 保持不变。完整边界见 [Contract v163](contract-v163-walled-outpost-magic-shop.md)。

协议 1.131 为 `ShopCategoryDto` 增加 `armoury` 与 `weaponsmith`，使 Outpost 共享工坊的两个入口保持严格类别。装备购买、穿戴、箭矢聚合、交易事件和存档继续复用既有 DTO；save v1 与 state hash Schema v60 保持不变。完整边界见 [Contract v164](contract-v164-outpost-armoury-weaponsmith.md)。

协议 1.132 为 `ShopCategoryDto` 增加 `bookstore`，使 Outpost 共享奥术建筑的书店入口保持严格类别。购买后的法术书实例继续复用既有库存、能力学习、知识和存档 DTO；save v1 与 state hash Schema v60 保持不变。完整边界见 [Contract v165](contract-v165-outpost-bookstore.md)。

协议 1.133 增加 `DepositAtHome` / `WithdrawFromHome`、`HomeDto` / `HomeItemDto` 与 `homes` 投影，并为 save v1 增加省略式 `homeStates`。Home 库存作为权威状态进入 state hash Schema v61；操作零时间、零金币、零 RNG，且不复用商店价格或店主模型。完整边界见 [Contract v166](contract-v166-outpost-home.md)。

协议 1.134 为 `ShopCategoryDto` 增加 `black-market`。Black Market 继续复用既有商店、库存、报价、交易事件和 save DTO；其 Warrior 买入加倍、卖出减半与店主单件收购上限由核心价格管线执行，不增加新的存档字段，state hash Schema 保持 v61。完整边界见 [Contract v167](contract-v167-outpost-black-market.md)。

协议 1.135 为 `Equip` 增加可选 `slotId`，用于在一个物品有多个合法装备目标时明确选择具体身体槽实例。未提供时继续按声明槽类型使用既有确定性自动选择；声明为 `tool` 的工具可以选择 `tool` 或 `weapon`，其他物品仍只能进入声明类型。该字段只属于瞬时命令，不进入存档；实际装备槽继续由既有 equipment save 保存，state hash Schema 保持 v61。

协议 1.136 为 `PlayerDto` 增加 `encumbranceSpeedPenalty`。`carryCapacityTenthsPound` 改为从有效力量按原版 38 档表动态投影；超重不再拒绝拾取、购买或从 Home 取出，而是在达到容量 120% 后按每 20% 施加 1 点权威速度惩罚。存档与 state hash 输入结构不变，Schema 保持 v61。

协议 1.137 为 save v1 增加必填 `defeatedUniqueActorKindIds`。该集合只记录已死亡的普通非 guardian Unique；当前层与离层仓库中的存活实例仍由 actor 状态直接证明占用，guardian 继续使用既有 dungeon 状态。读取时拒绝重复 ID、非 Unique、guardian 以及与存活实例冲突的集合。该权威状态进入 state hash Schema v63；旧开发存档不兼容。完整边界见 [Contract v173](contract-v173-warrens-allocation-ecology.md)。

协议 1.140 为 `ActorSaveDto` 增加必填 `nice`，保存原版 `FORCE_SLEEP → MFLAG_NICE` 的一次玩家行动出生宽限；当前层和离层仓库使用同一字段，旧开发存档不兼容。该字段进入 state hash Schema v64；它不属于普通 `StatusDto`，也不进入可见 `EntityDto`。完整边界见 [Contract v190](contract-v190-warrens-content-p6-spawn-grace-class-drops.md)。

contract-v191 只增加内容层近战 effect 及对应运行时解释，不新增命令、事件、快照或存档 DTO。失明、混乱、麻痹、减速、眩晕和恐惧继续使用既有 `StatusDto` / `StatusSaveDto`；协议保持 1.140，save v1 与 state hash Schema v64 不变。完整边界见 [Contract v191](contract-v191-warrens-content-p7-non-damage-melee.md)。

协议 1.142 为 `EquipmentPassiveDto` 恢复有权威消费者的 `see-invisible`，并为 `ActorSaveDto` 增加必填 `visibleInvisible`。该布尔值只保存当前已被看破的隐形 actor；实体与格子投影仍复用现有 `EntityDto` / `CellDto`，不可见目标不会泄露 ID。字段进入 state hash Schema v65，旧开发存档不兼容。完整边界见 [Contract v194](contract-v194-warrens-content-p10-movement-visibility-habitats.md)。

协议 1.143 新增方向命令 `Ride`，并以 `PlayerDto.ridingActorId` 投影当前坐骑。`PlayerSaveDto.ridingActorId` 为必填可空字段，坐骑与玩家同格、随普通移动/传送/楼层切换流转且不独立行动；该字段进入 state hash Schema v66，旧开发存档不兼容。完整边界见 [Contract v195](contract-v195-warrens-content-p11-special-mechanics.md)。

协议 1.144 为 `EntityFactionDto` 增加 `friendly`，并为 `ActorSaveDto` 增加省略式 `appearanceKindId`。友善 actor 仍自主行动，但不会被玩家侧 AI 视为敌人；外观种类只覆盖实体投影，真实 actor 种类继续承载属性、AI、掉落和死亡。外观字段进入 state hash Schema v67，旧开发存档不兼容。完整边界见 [Contract v196](contract-v196-warrens-content-p12-special-lifecycles.md)。

协议 1.145 增加 `EnterWorldMap` / `LeaveWorldMap`、`MapScaleDto`，并在既有 `CellDto` 上增加可选 `dangerLevel` 与 `locations`。世界尺度仍使用同一组 `width`、`height`、`cells`、`visualCells` 和玩家位置，不建立第二套地图协议。`mapScale`、荒野位置与种子进入 state hash Schema v68。W2 不增加 DTO：世界尺度现在接受既有 `Move` 和 `LeaveWorldMap`，其他战术命令仍拒绝；局部荒野继续投影为普通本地 cells。W3 同样不增加 DTO 或哈希字段：昼夜从 `worldTick` 派生，伏击复用局部投影与既有无参数事件形状，威胁锁从 `.ambush.` 敌对 actor 推导。W4 只以现有地点内容约束局部地牢入口，并复用保存的楼层恢复返回位置，不新增命令、事件、DTO、存档或哈希字段。协议 1.146 为 `EnterWorldMap` 增加 `leavePets` / `cancelRecall` 明示确认，增加单步 `TravelWorld { destination }`，并在 snapshot/update 投影可选 `worldTravelDestination`；目标进入 save v1 与 state hash Schema v69，使自动旅行可在伏击和读档后恢复。完整边界见 [Wilderness W1](wilderness-w1-map-state-display.md)、[Wilderness W2](wilderness-w2-travel-local-generation.md)、[Wilderness W3](wilderness-w3-day-night-ambush.md)、[Wilderness W4](wilderness-w4-location-loop.md) 与 [Wilderness W5](wilderness-w5-original-extensions.md)。
协议 1.145 为 `TerrainSaveDto` 增加必填逐格 `glow`，并增加 `darken-room` ability spec 与带 `clearedCells` 的结算结果。当前层和离层存储使用同一 glow 数据；它进入 state hash Schema v68，旧开发存档不兼容。完整边界见 [Contract v199](contract-v199-warrens-content-p15-darkness.md)。

集成协议 1.147 同时保留荒野 W1–W5 与房间永久光/黑暗字段；相对主线 1.146 新增的 `glow` 权威状态进入 state hash Schema v70，contract 基线统一刷新为 v204。

协议 1.148 为 `AbilityEffectSpecDto` 增加怪物专用 `blink-target { radius }`。它使用既有投射目标与怪物位移结算，只在目标当前位置给定半径内选择可通行空格；P29 的半径固定为 10。该效果不增加命令、存档或 state-hash 输入，save v1 与 Schema v70 保持不变。完整边界见 [Contract v213](contract-v213-warrens-content-p28-p29-ant-summon-target-blink.md)。

协议 1.148 为 `ItemPropertyKnowledgeSaveDto` 增加必填 `discovered`。玩家视野和物品探测写入该实例级状态；`GameSnapshot.items` 与 `CellDto.itemId` 只投影已发现的非金币地面物品。该状态进入 state hash Schema v71，save 容器保持 v1，旧开发存档不兼容。完整边界见 [O1 物品发现](object-list-o1-item-discovery.md)。

协议 1.149 增加单步 `TravelLocal { destination }`。Core 只根据当前已探索且可通行的地图知识选择下一步，避开已知陷阱，并复用普通 `Move` 的行动、怪物、时间、饥饿与光源结算；地图选点、循环定位楼梯、连续派发和中断由前端负责。普通地图目标遵循 RFB 原版生命周期，只在本次运行中供大写 `J` 恢复，不进入 save 或 state hash；Schema 保持 v71，基线升至 contract-v206。

集成协议 1.151 同时保留怪物目标闪现、物品发现、本地旅行和墨家名器双语配置；配置进入 save v1 与 state hash Schema v72，contract 基线统一刷新为 v215。

协议 1.152 新增 `MutationRatingDto`、`PlayerMutationDto` 与 `PlayerDto.mutations`，只投影角色当前 active 变异的稳定 ID、权威中文名称和描述、评级及锁定状态。`PlayerSaveDto` 必填保存排序后的 `activeMutationIds` 与 `lockedMutationIds`；未知 ID、重复 ID 及 locked 非 active 子集均拒绝载入。变异集合进入 state hash Schema v73，save 容器仍为 v1，旧开发存档不兼容。完整边界见 [Contract v216](contract-v216-mutation-authoritative-state.md)。

协议 1.153 为每个 `AttributeValueDto` 增加个人 `potential` 投影，并在 `PlayerProgressSaveDto` 中必填保存六维 `attributePotentials`。自然属性的永久成长上限是个人潜力与当前全局阶段上限的较小值；新生药水重掷 HP 成长与潜力、恢复生命力并移除全部未锁定变异。新增状态进入 State Hash Schema v74，save 容器仍为 v1，旧开发存档不兼容。完整边界见 [Contract v217](contract-v217-new-life-and-attribute-potentials.md)。

协议 1.152 为 `AbilityEffectSpecDto` 与 `AbilityEffectResolutionDto` 增加怪物专用 `teleport-level`。它复用现有楼层切换事务，先判定 Nexus 抗性与玩家豁免；不增加命令、存档字段或 state-hash 输入，save v1 与 Schema v72 不变。完整边界见 [Contract v218](contract-v218-warrens-content-p33-level-16-blockers.md)。

集成协议 1.154 同时保留角色变异、属性潜力与怪物层间传送，State Hash Schema 保持 v74，契约基线统一为 contract-v226。

协议 1.152 增加 `DestroyItem` / `InscribeItem`，为地面、背包、装备、商店与 Home 物品投影增加可选 `inscription`，并为墨家名器配置增加 `leaveDestroyedItems`。人工销毁与 `!` 规则共用 Core 的唯一保护判定；神器、任务物品、`indestructible` 内容标签及 `!k` / `!*` 铭文均拒绝销毁。规则铭文和销毁事件携带原始规则行号。铭文及保留选项进入 save v1 与 state hash Schema v73；旧开发存档不兼容。

协议 1.153 增加 `ResolveMogaminatorQuery` 和 `pendingQuery` 投影：`;` 规则按物品实例建立一次权威确认，拒绝结果随角色保存；物品离开脚下后待确认自动失效。`?` 规则在 Core 内按稳定实例 ID 优先消耗鉴定装置充能、其次消耗鉴定卷轴，成功后仅重新匹配一次。尸体来源、悬赏目标、延期谓词所需角色/物品/法术书数据进入 save v1 与 state hash Schema v74；内容包升级为 1.212.0。

contract-v216 完成墨家名器双语默认模板：英文模板严格采用 RFB `master:lib/pref/pickpref.prf`，中文模板保持同序同动作并使用权威中文匹配名。当前界面语言选择对应的每角色规则源；导入只替换当前语言的文本，不猜测语言，导出亦只导出当前文本。以上行为复用既有 `ConfigureMogaminator`、`defaultSource` 和双语 source 投影，没有新增 DTO；协议保持 1.153，Schema 保持 v74。

协议 1.154 为每角色墨家名器配置增加 `AutoGetModeDto` 三态：`off`、`ammo`、`wanted`。`ConfigureMogaminator` 原子更新该模式，`MogaminatorDto` 投影给编辑器，`MogaminatorSaveDto` 负责保存；默认 `off`。该权威配置进入 state hash Schema v75，save 容器仍为 v1，旧开发存档不兼容。G0 只建立配置闭环，不增加 Ctrl+G 命令、目标搜索或拾取执行。完整边界见 [G0：每角色自动拾取模式](autoget-g0-character-mode.md)。

协议 1.155 为 `MogaminatorDto` 增加派生的 `autoGetTarget` 坐标，并为 `GoldPileDto` 增加必填 `discovered`。Core 按原版距离、投射规则、已探索可达性及稳定实例 ID 选择目标；`wanted` 复用墨家名器首条命中和统一销毁保护，`ammo` 只认 `=g` 铭文。未发现金币不进入快照、格子占用投影或自动拾取目标；视野与金币探测均可登记发现。金币发现状态进入 save v1 与 state hash Schema v76，旧开发存档不兼容；目标本身只派生、不保存。完整边界见 [G1：权威自动拾取目标](autoget-g1-authoritative-target.md)。

协议 1.156 将 `autoGetTarget` 收紧为权威对象 ID 与坐标，并增加单步 `AutoGet { objectId }`。Core 只接受当前权威候选集中的对象：远端目标复用 O3 寻路转成一次普通移动，脚下按 ID 收取一堆金币、执行墨家名器动作或拾取 `=g` 物品；脚下、失效目标和普通 `PickUp` 均不推进世界时间。完整边界见 [G2：单步自动拾取命令](autoget-g2-single-step-command.md)。

G3 不改变协议：Web 将 Ctrl+G 与小写 `g` 分离，先执行零耗时普通拾取，再锁定 Core 投影的对象 ID 连续派发 `AutoGet`；对象消失后才请求下一个目标。中断只读取权威更新，完整边界见 [G3：Ctrl+G 连续调度](autoget-g3-continuous-dispatch.md)。

G4 以协议 1.156、state hash Schema v76 和 contract-v219 完成验收，不增加空协议版本。最初计划中的 1.154/v75 已分别由 G0 的模式状态采用；G1 的金币发现状态随后把最终 Schema 推进到 v76。验收记录见 [G4：协议与验收收口](autoget-g4-acceptance.md)。

集成协议 1.157 同时保留城镇自动拾取、物品变异与怪物能力的协议投影和存档字段；组合后的 state-hash 输入提升为 Schema v77，save 容器仍为 v1，契约基线统一为 contract-v227。

协议 1.158 为 `ActorSaveDto` 增加可选 `eldritchHorrorTriggered`，记录同一怪物已
造成过一次真实理智冲击，供重新进入视野时的 `1/5` 门使用。理智后果通过既有
状态、属性、地图知识和变异投影表达；没有新增通用事件 outcome DTO。该标记进入
state-hash Schema v78，save 容器保持 v1，基线为 contract-v229。

contract-v230 纠正出生城镇与局部荒野的初始怪物分配：城镇出生层不再调用荒野
分配，道路/非道路荒野分别采用原版 4/10 次初始分配。协议 `1.158`、State Hash
Schema `v78` 和 save v1 均不变。

协议 1.159 为 `SavePayloadV1` 增加必填 `wildernessViewOffset`，以 `(-1..=1,
-1..=1)` 表示玩家所在世界格相对 3×3 荒野视口中心的区块偏移；字段进入
state-hash Schema v79。动态荒野层固定为 96×33，即 3×3 个 32×11 区块；
城镇与地下城尺寸不变。contract-v231 只建立持久坐标模型，实际逐区块卷动仍属
后续实现；save 容器保持 v1，旧开发存档不兼容。

contract-v232 不改变协议 DTO。动态荒野把世界格与视口偏移映射为绝对 32×11
区块，最多缓存视口周围 5×5 个派生地形区块；缓存不保存、不投影，也不进入
State Hash。成功进入世界地图时清空缓存并以固定奇数步长推进既有
`wildernessSeed`，下一次局部荒野使用新一代微地形。协议保持 1.159，State
Hash Schema 保持 v79，save 容器保持 v1。

协议 1.160 为 `GameUpdate` 增加可选 `mapTranslation: Position`。动态荒野视口
每平移一个区块便投影 `(-32, 0)`、`(32, 0)`、`(0, -11)`、`(0, 11)` 或对应
对角组合，供客户端同步本地自动旅行目标；未发生视口平移时省略。地图格仍通过
既有 `changedCells` 与 `changedVisualCells` 全量刷新。contract-v233 不增加存档
字段，State Hash Schema 保持 v79。

contract-v234 不改变协议 DTO。普通荒野卷动只在新暴露条带补充怪物，伏击判定
与激活仅允许发生在世界地图；进入伏击后仍使用既有局部战斗地图。协议保持
1.160，State Hash Schema 保持 v79。

contract-v235 复用协议 1.160 已有的 `GameUpdate.mapTranslation`，让客户端同步
查看、瞄准和本地旅行的临时坐标，并在打开对象列表时重建坐标、按稳定对象 ID
保留选择。地图继续使用全量 changed 集合，不增加分块渲染或动画协议。

contract-v236 不改变协议 DTO。普通荒野卷动在正规化后真正跨入城镇世界格时，
直接投影既有独立城镇 FloorState；该次更新不发送 `mapTranslation`。协议保持
1.160，State Hash Schema 保持 v79。

contract-v237 不改变协议 DTO。可变尺寸城镇以 `mapOrigin` 嵌入同一 96×33 连续
荒野视口；卷动继续使用既有 `mapTranslation`，Town、Shop、Home 与任务设施继续
使用既有 DTO。城镇地表状态在活动视口与既有 `storedFloors` 之间裁剪同步，未新增
存档字段；协议保持 1.160，State Hash Schema 保持 v79，save 容器保持 v1。

协议 1.161 为 `EntityDto` 增加必填 `glyph`。ESP 仅通过心灵感应发现、但未被视觉
看到的敌人继续投影原怪物 glyph，同时把 `kindId` 和战斗细节替换为通用“怪物”
身份；`EMPTY_MIND` 永不被基础 ESP 发现，`WEIRD_MIND` 的 1/10 感知结果进入存档与
state hash，因此 State Hash Schema 升至 v80。save 容器保持 v1，旧开发存档不兼容。

协议 1.162 用 `AbilitySourceDto` 的 `learned`、`technique`、`mutation` 取代
`AbilityDto.innate`，并允许没有 SP 池的主动变异省略 `resourceId`。
`AbilityCastResolutionDto` 新增实际支付的 `resourcePaid` 与 `hpPaid`；普通技能仍
全部支付职业资源，主动变异则按 RFB 规则先支付现有 SP、再以 HP 支付差额。
变异能力继续复用 `CastAbility`、目标与效果结果 DTO，不进入学习、熟练度或冷却
持久状态。save 与 state hash 输入未变化，State Hash Schema 保持 v80。

协议 1.163 为 M5-A/B 主动变异补齐来源无关的效果投影：随机自身传送、隔空取物、
换位、Recall、元素抗性、等级阈值伤害，以及带类别/疲劳参数的 Banish。诅咒探测
进入既有 Detect 枚举，吸血可声明进食，群体状态可携带抗性检定。新增结构化的取物、
换位与 Recall 结算；22 个变异仍通过统一 `CastAbility` 入口施放，不增加存档字段，
save v1 与 State Hash Schema v80 不变，基线推进至 contract-v242。

协议 1.165 为 M6-A 增加 `ResolveMutationDirection`、
`PlayerDto.pendingMutationDirection`、`PlayerDto.minorSlow` 与休息中断原因。Produce
Mana 触发后暂停当前周期序列，客户端必须选择八方向之一；解析后从下一项周期变异
继续并完成同一 tick。待选方向与 `minorSlow` 进入 save v1 和 State Hash Schema
v82；旧开发存档不兼容。

协议 1.166 为 M6-B 在 `PlayerDto` 增加 `realityChangeTicks`，投影影中漫步触发的
通用延迟现实改变倒计时。倒计时结束时只有普通程序地下城重生成；固定任务层、城镇
和连续荒野保持不变，荒野 seed 不推进。笨手笨脚的伤害与可选武器掉落通过结构化
事件投影。倒计时进入 save v1 和 State Hash Schema v83；旧开发存档不兼容。

协议 1.169 增加窄命令 `StayAtInn { facilityId }`。阿南巴旅店住宿通过既有通用事件
投影成功、费用、余额、时间跨度和拒绝原因；不新增持久字段或专用建筑服务框架。
State Hash Schema 保持 v85，save 容器保持 v1，基线推进至 contract-v252。

contract-v253 只增加只读怪物审计命令并同步 Orc Cave 29–32 级现有机制可表达内容；
协议仍为 1.169，State Hash Schema 仍为 v85，save 容器仍为 v1。

contract-v254 扩展仅供怪物使用的内容侧 `jump-damage` 固定伤害，并复用既有分类召唤
接入 Hydra 与 Zoopi；不增加协议 DTO 或持久状态。协议仍为 1.169，State Hash Schema
仍为 v85，save 容器仍为 v1。

contract-v255 为怪物近战物理伤害增加仅用于内容和运行时结算的 `vampiric` 标记，
按实际伤害治疗攻击者；非生命玩家不提供治疗。不增加协议 DTO 或持久状态，协议仍为
1.169，State Hash Schema 仍为 v85，save 容器仍为 v1。

contract-v256 复用既有 `AnimateDead` 效果与结算 DTO，为内容侧增加怪物专用的遗骸
失败率，并生成敌对召唤物。不增加协议字段或持久状态，协议仍为 1.169，State Hash
Schema 仍为 v85，save 容器仍为 v1。

协议 1.170 / contract-v257 增加怪物专用 `PolymorphTarget` 能力效果与窄结算 DTO。
玩家目标复用既有变异重组事务；玩家阵营召唤物复用变色龙候选选择与形态属性刷新，
并直接替换实际 actor kind。接触光环的火焰、闪电和诅咒仍走既有伤害、抗性与豁免
事件，不增加专用协议。State Hash Schema 保持 v85，save 容器保持 v1。

contract-v258 的反击/恐惧光环、狸猫外观和 `UNIQUE2` 生命周期均复用现有 DTO；
恐惧光环只产生通用事件，狸猫继续使用 `appearanceKindId`。协议保持 1.170，State
Hash Schema 保持 v85，save 容器保持 v1。

协议 1.171 / contract-v259 为 `ActorSaveDto` 增加默认 1000、常态省略的
`powerPerMille`，保存 UNLIFE 对怪物造成的永久强度变化；Actor 投影继续按派生属性
显示结果，不暴露第二套战斗 DTO。装备 passive 增加 `hold-life`，供生命力吸取前的
逐来源豁免使用。生命力仍复用现有 `CharacterProgress.lifeForce`，但现在参与最大生命
派生与当前生命同比例缩放。State Hash Schema 升至 v86，save 容器保持 v1；旧开发
存档不作为兼容边界。完整边界见 [Contract v259](contract-v259-orc-cave-unlife.md)。

协议 1.172 / contract-v261 为 `ShopCategoryDto` 增加 `shroomery`。快速恢复继续复用
既有物品治疗、状态与商店 DTO，不增加物品专用协议字段；`rfb.status.regeneration`
通过现有 `StatusDto` 投影。State Hash Schema 保持 v86，save 容器保持 v1。

协议 1.173 / contract-v262 在 `ShopDto` 投影可选 `innStayCost` 和仅含已访问城镇的
`innTravelDestinations`，并增加窄命令 `TravelFromInn { facilityId,
destinationTownId }`。旅行固定收费 500 金币，直接重建目标城镇的连续地表视口并
落在目标旅店入口；不进入世界地图、不推进 `wildernessSeed`。住宿继续使用
`StayAtInn`，价格不再绑定具体内容 ID。State Hash Schema 保持 v86，save 容器保持 v1。

协议 1.174 / contract-v263 增加 `IdentifyAtFacility` 与 `RenameAtFacility`，并由
`TaskServiceDto` 投影对应内容价格。`PlayerDto.name` 成为必需字段；新建角色输入、伯爵府
合法改名和存档恢复使用同一 1–32 字符验证。State Hash Schema 升至 v87，save 容器保持 v1。

协议 1.175 / contract-v264 退役原创职业协议：`AbilitySourceDto` 删除 `technique`，
`ResourcePoolDto` 删除近战获得与逐回合衰减提示，删除职业 `RechargeItem` 命令、
`PlayerDto.deviceRecharge` 和对应来源 DTO。物品 `UseItemForRecharge`、设备自然恢复、
`canReceiveRecharge` / `canSupplyRecharge` 及结构化充能结果继续保留，供卷轴和正式物品
能力复用。没有新增持久状态，State Hash Schema 保持 v87，save 容器保持 v1。

协议 1.176 / contract-v266 为 `AbilityDto` 增加可选 `uiGroupNameKey`。Archer 的三个
既有制造端点使用同一分组名，前端按 `minimumLevel` 只展示已开放子项；执行仍复用
`CastAbility` 和既有目标选择。玩家制造弹药物品的持久字段正式纳入 State Hash Schema
v88，save 容器保持 v1。

协议 1.177 / contract-v272 增加书本级 `StudyPrayer { bookItemId }` 命令，并由
`AbilityLearningDto.studyMode` 区分玩家点选的 `chosen` 与神授随机的
`divine-random`。成功继续产生既有 `ability.studied` 事件；没有新增待处理状态、存档
字段或状态哈希输入，State Hash Schema 保持 v88，save 容器保持 v1。

协议 1.178 / contract-v275 为 `PlayerProgressSaveDto` 增加必填的稀疏
`weaponProficiencies`，保存高于职业出生值的规范基础武器训练值。该权威成长状态进入
State Hash Schema v89；save 容器保持 v1，不兼容缺少该字段的旧开发存档。

协议 1.179 / contract-v276 为 `PlayerProgressDto` 增加只读 `weaponProficiencies`，每项
投影规范基础武器、近战/发射器分类、当前值、职业上限、原版等级与原版命中加成。该字段
不增加权威状态，State Hash Schema 保持 v89，save 容器保持 v1。

协议 1.180 / contract-v278 将 `WeaponProficiencyRankDto` 泛化为
`ProficiencyRankDto`，并为 `PlayerProgressDto` 增加 `miningProficiency` 与 `materials`：
前者投影当前挖掘力、原版等级、当前值和 8000 上限，后者按固定身份投影十种材料数量。
`PlayerProgressSaveDto` 同步增加必填挖矿熟练度和稀疏材料数组；权威状态进入 State Hash
Schema v90，save 容器保持 v1。

协议 1.181 / contract-v279 为 `ItemOriginKindDto` 增加 `rubble`，用于投影并持久化碎石
掉落来源。富矿材料与金币继续使用既有 `PlayerProgressDto.materials` 和 `GoldPileDto`；
没有新增权威字段，State Hash Schema 保持 v90，save 容器保持 v1。
协议 1.177 为能力 effect 投影增加窄化的 `light-line` 与 `area-destruction` 规格，
并为后者增加结构化结算结果；怪物变形与地震继续复用既有 DTO。该变化只完成
`Invoke Spirits` 的既有随机分支，不增加命令、待处理输入或存档字段。State Hash
Schema 保持 v88，save 容器保持 v1。

协议 1.178 为 `AbilityEffectSpecDto` 增加窄化的 `sequence` 投影，用于
`Invoke Spirits` 最高随机档的一层 self 组合效果。随机分支的等级缩放在服务端物化后
再投影，不新增客户端配置 DTO、命令、待处理输入或存档字段。State Hash Schema 保持
v88，save 容器保持 v1。

协议 1.184 / contract-v282 增加零时间 `DismissPets` 命令、派生的
`PlayerDto.petUpkeep` 摘要，以及宠物维持法力损失、零法力解散要求和冷落结果事件。
维持摘要不保存；它由当前职业、等级、法力池和存活的玩家控制 actor 重新计算。
State Hash Schema 保持 v92，save 容器保持 v1。
协议 1.185 / P50 为 `AbilityEffectSpecDto::CurseDamage` 增加
`damageIsCurrentHpPercent` 与 `nonlethal`，用于准确投影“毁灭之手”的当前生命百分比和
非致死语义。没有新增持久状态，State Hash Schema 保持 v88，save 容器保持 v1。

协议 1.186 / P56B 为 `AbilityEffectSpecDto::SummonCategory` 增加可选
`maximumCount`，用于保留 `min(1d4, 3)` 这类召唤数量上限。没有新增持久状态，
State Hash Schema 保持 v88，save 容器保持 v1。

协议 1.187 / contract-v283 为 `SavePayloadV1` 增加必填 `generatedArtifactIds`，按稳定
物品 ID 保存已生成的 RFB 固定神器。该字段不进入游戏快照，普通物品品质枚举也不增加
“神器”伪值；固定神器身份继续来自内容定义。权威集合进入 State Hash Schema v93，
save 容器保持 v1，不兼容缺少该字段的旧开发存档。

协议 1.188 / P60 为 `AbilityEffectSpecDto::SummonCategory` 增加可选
`batchCandidates`，用于先掷数量、再以一次加权选择固定整批召唤对象。该字段仅允许怪物
能力使用；没有新增持久状态，State Hash Schema 保持 v93，save 容器保持 v1。

协议 1.189 / contract-v285 为 `PlayerProgressDto` 增加 `ridingProficiency`，投影骑术
专用等级、当前值与职业上限；`PlayerProgressSaveDto` 同步增加必填当前值。该权威成长
状态进入 State Hash Schema v94，save 容器保持 v1，不兼容缺少字段的旧开发存档。

协议 1.190 / contract-v287 为 `AbilityEffectSpecDto` 增加无参数 `rodeo` 变体，使客户端
能投影正式骑兵职业能力。命令继续使用既有 `UseAbility` 与方向 `TargetSelection`，事件
继续使用通用 `GameEventDto`；没有新增存档字段或权威状态结构，State Hash Schema 保持
v94，save 容器保持 v1。

协议 1.191 / contract-v289 增加 `PetDto` 列表，投影受控 actor 的种类、等级、进化经验、
骑乘状态与当前羁绊；`InventoryItemDto.mountUsable` 只投影当前坐骑已解锁的药水入口。
命令继续复用 `UseItem` 与 entity `TargetSelection`。`ActorSaveDto.experience` 和
`PlayerSaveDto.ridingBond` 为必填权威字段，进入 State Hash Schema v95；save 容器保持
v1，不兼容缺少字段的旧开发存档。

协议 1.192 / contract-v290 为背包与装备物品投影增加 `captureBall`、可选
`capturedActor` 和装备侧 `useTargetSpec`，商店库存与家中物品也投影可选 `capturedActor`。
空球复用既有 `UseItem` 的 entity 目标，满球复用 direction 目标；当前坐骑与玩家同格时仍
可作为 entity 目标。四种物品 save DTO 均增加必填可空 `capturedActor`，进入 State Hash
Schema v96；save 容器保持 v1，不兼容缺少字段的旧开发存档。

协议 1.189 / Arcane 第一册为 `AbilityEffectSpecDto` 增加窄化的 `light-area`、
`terrain-beam`、`heal-dice` 与 `reduce-status` 投影，并为状态削减增加结构化结算结果。
这些表面只承载区域照明、门闩/门陷阱射线与骰式治疗；没有新增待处理输入或存档字段，
State Hash Schema 保持 v93，save 容器保持 v1。

协议 1.190 / Arcane 第二册为探测规格与结算增加 `throughWalls`，为
`reduce-status` 增加可选 `currentDivisor`，并增加窄化的 `refuel-equipped-light` 规格与
结构化结算结果。它们分别承载原版穿墙探测、`max(100, current / 5)` 解毒和已装备火把/
提灯补充一半最大燃料；没有新增待处理输入或存档字段，State Hash Schema 保持 v93，
save 容器保持 v1。

协议 1.191 / Arcane 第三册为 `terrain-beam` 增加 `stone-to-mud` 操作，为
`reduce-status` 增加仅允许流血使用的可选 `remainingDivisor`，并增加窄化的
`satisfy-hunger` 规格与结构化结算结果。它们分别承载原版 `1d30+20` 化石为泥、
`current / 2 - 50` 治疗中伤和 `PY_FOOD_MAX - 1` 充饥；基础鉴定允许以 0/0 表示不掷
完整鉴定判定。没有新增待处理输入或存档字段，State Hash Schema 保持 v93，save 容器保持 v1。

协议 1.192 / Arcane 第四册前置为 `teleport-away` 增加玩家射线所需的 power，
并增加窄化的 `recharge-from-player` 能力规格及 Teleport Away 结算结果。充能继续使用
既有物品目标和装置充能事件，额外消耗能力资源；没有新增持久状态，State Hash Schema
保持 v93，save 容器保持 v1。

协议 1.193 / Arcane 完整领域增加窄化的 `clairvoyance` 能力规格，投影临时 ESP 的
固定时长与骰式时长。结算复用既有探测、状态和 virtue 事件：永久绘制并照亮当前层、
揭示全部地面物品，并仅在玩家没有永久 ESP 时掷 `25 + 1d30` 临时 ESP。没有新增命令、
待处理输入或持久状态，State Hash Schema 保持 v93，save 容器保持 v1。

协议 1.189 / contract-v285 将 `SavePayloadV1.defeatedUniqueActorKindIds` 泛化为必填的
`defeatedLimitedActorCounts`，按 actor ID 持久化有限生命周期怪物的死亡数量。普通
`unique` 的隐式生命周期上限仍为 1，`unique2` 仍只限制同时存活一只；显式
`lifetimeInstanceLimit` 可提供更高的跨楼层、跨死亡总额度。该权威表在组合后的协议
1.195 中进入 State Hash Schema v97，save 容器保持 v1，不兼容缺少新字段的旧开发存档。
协议 1.196 / contract-v293 为 `AbilityEffectSpecDto` 增加 `concentrate`，为 `AbilityDto`
增加专注门槛与生命成本，并以 `PlayerDto.sniperConcentration` 投影狙击手专注当前值和
等级上限。命令继续复用 `CastAbility` 与普通射击，不增加新的待处理输入状态。

协议 1.197 / contract-v294 为 `AbilityEffectSpecDto` 增加 `sniper-shot` 及八种射击模式。
特殊射击继续复用 `CastAbility`、现有目标选择和普通 projectile 事务；射程与行动能量取自
当前发射器，不增加新的命令、待处理输入或持久状态。State Hash Schema 保持 v98，save
容器保持 v1。

协议 1.198 / contract-v295 为 `sniper-shot` 增加邪恶、神圣、爆炸、双重、雷霆、针刺和
圣星之箭七种模式，并增加无参数 `probe-monsters` 效果及逐实体的怪物探测结算。探测结果
投影稳定实体/种类 ID、位置、生命、速度、AC、阵营、抗性、状态免疫、近战与施法能力；
已探测种类继续使用 contract-v293 已持久化的 `probedActorKindIds`。没有新增命令、待处理
输入或权威状态结构，State Hash Schema 保持 v98，save 容器保持 v1。

协议 1.199 / contract-v298 增加零时间的 `ChooseRaceMutation { rewardId, mutationId }`
命令，以及可选的 `PlayerDto.pendingRaceMutationChoice`。候选复用 `PlayerMutationDto` 的名称、
说明与评级投影；待选择状态由当前等级、种族奖励配置和既有锁定变异集合派生，不进入
save 或 State Hash。待选择期间核心只接受该选择命令，State Hash Schema 保持 v98，save
容器保持 v1。

协议 1.200 / contract-v301 为能力效果增加无参数 `melee-adjacent`，供原版“大屠杀”复用
普通近战事务；`ActorSaveDto` 增加必填 `anger` 与 `friendly`。前者保存远程伤害触发的
0–100 怒气，后者保存“个人崇拜”产生的运行时友好阵营。两项进入 State Hash Schema v99；
save 容器保持 v1，不提供旧开发存档默认值。

协议 1.201 / contract-v302 合并咒术与毁灭领域的分支协议增量：增加批量鉴定、城镇目标、
造楼梯、城镇传送、角色自省、次元门、探知、造门、装置精通、放逐、无敌结界等窄化能力
投影及结构化结果。`StatModifiersDto` 增加装置强度修正，`EntityDto` 与 `ActorSaveDto`
增加必填的 0..10 `minorSlow`；后两项权威状态与 contract-v301 的怒气、友好状态共同进入
State Hash Schema v100。save 容器保持 v1，不为缺少这些字段的旧开发存档提供兼容默认值。

协议 1.202 / active baseline contract-v303 为 `AbilitySourceDto` 增加 `race`，使种族天生
能力与职业能力、已学法术和变异能力在同一 `AbilityDto` 列表中保持可辨来源。能力继续复用
既有 `CastAbility`、目标选择和结算投影；没有新增命令、权威状态或 save 字段，State Hash
Schema 保持 v100，save 容器保持 v1。

协议 1.203 / active baseline contract-v303 为 `AbilityDto` 增加可选的
`governingAttribute`，用于显示种族、变异及职业主动能力的当前检定属性。它只投影既有内容
定义，不增加命令、权威状态、RNG 或 save 字段；State Hash Schema 保持 v100，save 容器
保持 v1。

协议 1.204 / active baseline contract-v303 为能力效果增加 `create-item`，并以结构化结果
投影生成的物品种类、数量、落点和目标实例 ID；`ItemOriginKindDto` 增加 `acquire`，保存原版
`ORIGIN_ACQUIRE` 来源；自然领域第一册同时使 Beam 可声明独立最大射程并参与等级、
`spell_power` 缩放。能力仍复用现有施放命令和 self 目标；没有新增权威状态或 save 字段，
State Hash Schema 保持 v100，save 容器保持 v1。

协议 1.205 为自然领域第二册增加窄化的 `entangle` 与 `nature-gate` 能力投影。前者保留
唯一怪免疫和原版旧式等级检定；后者按施法者等级选择 Ranger 动物、猎犬、九头蛇或树人，
生成物继续进入既有宠物维持系统。固定治疗值现在可显式参与 `spell_power`。这些变化不增加
命令、待处理输入或持久状态，State Hash Schema 保持 v100，save 容器保持 v1。

协议 1.206 为自然领域第三册增加相邻地形创建、永久防腐保护和召唤阳光的窄化能力投影及
结构化结算；状态防御加值可以显式参与等级缩放。物品实例、背包、装备、地面、商店与住宅
投影增加永久元素破坏免疫集合，save DTO 同步增加必填字段。该集合进入 State Hash Schema
v101；save 容器保持 v1，不为缺少该字段的旧开发存档提供兼容默认值。

协议 1.207 为自然领域第四册增加无参数 `nature-wrath` 能力投影，以及
`ResolveAbilityDirection`、`CancelAbilityDirection` 两条命令。六分支事务先完成一次原版
`1d6` 选择；仅闪电矢和三连碎片球分支保存 `PlayerSaveDto.pendingAbilityDirection`，待玩家
选择方向后才一次性支付已结算的法力、熟练度和行动成本，取消不留下部分效果。该待处理状态
进入 State Hash Schema v102；save 容器保持 v1，不兼容缺少该字段的旧开发存档。

协议 1.208 为生命领域第一册补充 `heal-dice` 的整次治疗量 `spell_power` 投影，并允许
`light-area` 声明整次伤害的 `spell_power`。两者只增加可观察公式元数据，运行时继续复用既有
治疗、照明与区域伤害事务；不增加命令、持久状态或 State Hash 输入。State Hash Schema
保持 v102，save 容器保持 v1。

协议 1.209 为生命领域第二册增加普通装备解咒、开始禁食、亡灵退散和脚下地形创建的
能力投影，并在 `PlayerDto` 与 `PlayerSaveDto` 增加必填 `fasting`。禁食状态进入
State Hash Schema v103；save 容器保持 v1，旧开发存档不兼容。

协议 1.210 为生命领域第三册增加有序属性维持、随机治愈变异的能力投影及结构化结算，
并允许视野伤害在实际命中亡灵后更新 Unlife。`transcendence` 复用既有临时状态容器，
在统一的最终玩家伤害入口按 1:1 先消耗法力、再扣除剩余生命；结界序列复用脚下与相邻
地形事务。没有增加新的持久字段，State Hash Schema 保持 v103，save 容器保持 v1。

协议 1.211 / P89E 为 `EquipmentItemDto` 增加 `usable`、可选 `charges` 和可选
`activation`，使已装备的固定神器可沿用现有 `UseItem` 命令与装置充能状态执行激活。
同时为 `DungeonStateSaveDto` 增加必填 `suppressed`，持久化共享入口地牢的替代选择；该状态
及合并后的状态增量进入 State Hash Schema v104，save header/payload schema 升至 v2，
二进制容器格式仍为 v1；不为缺少新字段的旧开发存档提供兼容默认值。

协议 1.212 为 `DamageTypeDto` 增加 `rock`，供独眼巨人的“投掷巨石”区分原版岩石命中：
怪物若没有音波抗性则进行等级检定并可能眩晕；反射回玩家时按原版等概率进入碎片/流血或
音波/眩晕分支，并复用现有元素库存损坏事务。该变化不增加命令、权威状态或 save 字段，
State Hash Schema 保持 v104，save header/payload schema 保持 v2。

协议 1.213 增加独立的 `AbsorbDevice` 命令，并在背包与地面物品投影增加默认 `false` 的
`absorbable`。有效种族可从背包或脚下吸收装置的一次使用充能并恢复营养；该事务不复用普通
`UseItem` 的装置激活路径，也不增加权威状态或 save 字段。State Hash Schema 保持 v104，
save header/payload schema 保持 v2。

协议 1.214 为恶魔领域第二册增加无参数 `demon-summoning` 能力投影。该复合效果固定原版的
`1/3` 敌对判定、动态召唤等级与 50 级友好群组边界；不增加命令或持久状态，State Hash
Schema 保持 v104，save header/payload schema 保持 v2。

协议 1.215 为恶魔领域第三册增加 `lava-flow` 与 `doom-hand` 能力投影，并允许
`random-choice` 分支承载锥形伤害。前者在以施法者为中心造成火焰伤害后，用一次独立随机值
决定深熔岩地形强度；后者保留唯一怪免疫、抗性检定与按当前生命百分比扣血。恶魔变形继续
复用临时种族覆盖，火焰光环复用通用状态和接触反伤；不增加持久字段，State Hash Schema
保持 v104，save header/payload schema 保持 v2。

协议 1.216 为恶魔领域第四册增加 `insanity-circle`、`explode-pets`、
`summon-greater-demon` 与 `hellfire` 能力投影，以及宠物爆炸、尸体献祭和玩家自伤的结构化
结算。送入地狱复用单体灭绝，恶魔领主变形复用临时种族覆盖和穿墙状态；没有新增持久字段，
State Hash Schema 保持 v104，save header/payload schema 保持 v2。

协议 1.217 为圣战领域第一册增加 `stardust` 与 `sanctuary` 能力投影。前者固定十次独立散射、
伤害掷骰、可反射光属性投射和命中网格照明；后者对半径 1 内怪物结算睡眠且不要求目标可见。
其余六个法术复用既有投射、探测、状态清除、恐惧、传送和有序状态事务；没有新增持久字段，
State Hash Schema 保持 v104，save header/payload schema 保持 v2。

协议 1.218 为圣战领域第二册扩充既有瞬时效果投影：`teleport-away` 增加 `stopAtActor` 与可选
`targetCategory`，用于表达半径 0 的邪恶目标定向传送；`visible-apply-status` 增加持续时间骰，
用于表达驱魔的 `1+3d(level/2)` 恐惧。驱魔的两次视野伤害与圣言的伤害、治疗、状态清除仍使用
有序效果事务；没有新增持久字段，State Hash Schema 保持 v104，save header/payload schema保持 v2。

协议 1.219 为圣战领域第三册增加 `angel-summoning` 能力投影，固定 `1/3` 敌对、敌对群组、
50 级友好群组和原始 `3*level/2` 召唤等级边界。拘捕继续投影为标准 `apply-status`，运行时保留面板
spell power 与实际原始 power 的源码差异；天使斗篷复用通用状态，神圣之刃复用永久物品词缀。
没有新增持久字段，State Hash Schema 保持 v104，save header/payload schema 保持 v2。

协议 1.220 为圣战领域第四册增加 `banish-evil`、`wrath-of-god`、`divine-intervention` 与
`crusade` 能力投影，分别公开驱逐强度、分解球伤害与数量、神圣干预的两段伤害/控制/治疗参数，
以及圣战的魅惑强度和十二次召唤尝试。英雄气概、驱除诅咒、末日审判和以眼还眼继续复用既有
投影；没有新增持久字段，State Hash Schema 保持 v104，save header/payload schema 保持 v2。

协议 1.221 为 `EquipmentBonusesDto` 增加默认 0 的 `lifePercent`，并为
`EquipmentPassiveDto` 增加漂浮、警告、减缓消化和九类定向 ESP。它们统一来自已装备物品的
正式词缀投影；没有新增命令或持久字段，State Hash Schema 保持 v104，save header/payload
schema 保持 v2。
