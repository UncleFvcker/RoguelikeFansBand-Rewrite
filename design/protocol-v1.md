# RFB CoreTransport 协议 v1

状态：协议 1.21、自动生成的 TypeScript/JSON Schema 与 `TauriNativeTransport` 已实现

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
  protocolVersion: "1.21";
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

协议 1.21 新增 `UseItem`、背包 `usable` 投影和结构化 `HealingResolutionDto`；核心在单一事务中消费物品、应用效果并按实际可观察结果更新知识。当前规则边界见 [Contract v21](contract-v21-consumable-use-action.md)。

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

当前命令集包括八向 `Move`、`Wait`、`PickUp`、`Equip`、`Unequip`、`Drop`、`DropQuantity`、`Fire`、`FireTarget` 和 `Throw`。`PickUp` 在玩家脚下按实例 ID 确定性选择物品堆；`Equip`/`Unequip` 在背包与稳定槽位之间移动完整物品；`Drop` 原子移动多个所选完整物品堆；`DropQuantity` 拆分单个物品堆并使用持久化生成实例 ID；`Fire` 保留方向快捷入口，`FireTarget` 提交稳定方向/格子/实体目标并原子消费匹配弹药；`Throw` 原子拆分或移动一件背包物品到权威落点。命令先转换为 `GameAction`；当前所有已接入且被核心接受的行动消耗 100 能量、增加一个玩家 `turn`，随后调度世界脉冲直到玩家再次就绪或死亡。

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
