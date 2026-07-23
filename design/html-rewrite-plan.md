# RoguelikeFansBand Rust/Tauri 重构计划

状态：原生 Rust + Tauri 2 技术路线已确定，WASM 目标已停止维护

目标仓库：`UncleFvcker/RoguelikeFansBand-Rewrite`

关联文档：

- [旧版行为基准与差分测试](legacy-behavior-baseline.md)
- [Contract 基准更新与差异豁免政策](baseline-update-policy.md)
- [Contract v2 内容运行时迁移](contract-v2-content-migration.md)
- [Contract v3 背包权威状态迁移](contract-v3-inventory-migration.md)
- [Contract v4 装备与批量丢弃迁移](contract-v4-equipment-migration.md)
- [Contract v5 装备属性与物品实例迁移](contract-v5-item-instance-migration.md)
- [Contract v6 基础战斗属性迁移](contract-v6-combat-stats-migration.md)
- [Contract v7：RFB 风格基础近战闭环](contract-v7-rfb-melee-migration.md)
- [Contract v8：行动能量、速度与怪物追踪](contract-v8-action-energy-tracking.md)
- [Contract v9：状态、抗性与效果管线](contract-v9-status-resistance-effects.md)
- [Contract v10：流血与内容驱动元素近战](contract-v10-bleeding-elemental-melee.md)
- [Contract v11：结构化伤害事件、派生属性与检定底座](contract-v11-structured-damage-events.md)
- [Contract v12：武器 AttackProfile 与玩家多段近战](contract-v12-weapon-attack-profile.md)
- [Contract v13：怪物 MeleeRoutine 与稳定 blow 顺序](contract-v13-monster-melee-routines.md)
- [Contract v14：权威 projectile 与发射器基础](contract-v14-projectile-foundation.md)
- [Contract v15：弹药事务与投掷落点](contract-v15-ammunition-throwing.md)
- [Contract v16：核心目标选择与非八方向轨迹](contract-v16-target-selection.md)
- [Contract v17：弹药破损与落地回收](contract-v17-ammunition-recovery.md)
- [Contract v18：投掷攻击与重量射程](contract-v18-thrown-attacks.md)
- [Contract v19：携带重量与拾取容量](contract-v19-inventory-capacity.md)
- [Contract v20：物品知识与未知名称投影](contract-v20-item-knowledge.md)
- [Contract v21：消耗品 UseAction 与可观察鉴定](contract-v21-consumable-use-action.md)
- [Contract v22：实例词条与知识投影](contract-v22-instance-affix-knowledge.md)
- [Contract v23：物品鉴别与完整识别](contract-v23-item-appraisal.md)
- [Contract v24：确定性战利品生成](contract-v24-deterministic-loot-generation.md)
- [Contract v25：怪物携带物与统一死亡掉落事务](contract-v25-monster-carried-items.md)
- [Contract v26：楼层生命周期与确定性程序化楼层](contract-v26-floor-lifecycle.md)
- [Contract v27：程序化房间怪物与地面掉落分配](contract-v27-procedural-room-content.md)
- [Contract v28：门地形状态与方向性交互](contract-v28-door-terrain-state.md)
- [Contract v29：锁门、开锁检定与破门](contract-v29-locked-door-checks.md)
- [Contract v30：权威相邻地形交互查询](contract-v30-authoritative-terrain-interactions.md)
- [Contract v31：秘密门、搜索与地形知识](contract-v31-secret-door-search.md)
- [Contract v47：深度主题 Vault 与群体遭遇](contract-v47-themed-vault.md)
- [Contract v48：楼层生成表、加权 Vault 与巢穴](contract-v48-floor-generation-tables.md)
- [Contract v49：预算化十层压力地牢](contract-v49-budgeted-pressure-dungeon.md)
- [Contract v50：Vault 空间变换与确定性多模板落位](contract-v50-spatial-vault-placement.md)
- [Contract v51：动态 friends/escort 群体与 formation](contract-v51-dynamic-encounter-groups.md)
- [Contract v52：程序化特殊地形表与空间预算](contract-v52-terrain-feature-budgets.md)
- [Contract v53：分阶段洞穴地貌与房间几何预算](contract-v53-staged-cavern-layout.md)
- [Contract v54：湖泊与河流水文阶段](contract-v54-lake-river-hydrology.md)
- [Contract v55：迷宫、毁坏区与岩脉阶段](contract-v55-maze-destroyed-streamers.md)
- [Contract v56：原版式怪物 Pit 与等级阵列](contract-v56-classic-monster-pit.md)
- [Contract v57：Maze-only 专用楼层模式](contract-v57-maze-only-floor.md)
- [Contract v58：权威楼层连接与 shaft](contract-v58-floor-connections.md)
- [Contract v59：持久 pack identity 与首版 pack AI](contract-v59-pack-ai.md)
- [Contract v60：同层多区域主题](contract-v60-regional-themes.md)
- [Contract v61：暂停任务管理与确定性重接](contract-v61-retake-management.md)
- [Contract v62：区域组合生成](contract-v62-regional-composition.md)
- [Contract v63：树状地牢与共享守护者镜像](contract-v63-dungeon-tree-guardian-mirrors.md)
- [Contract v64：多入口 Vault 与连通拼接](contract-v64-multi-entry-vault-connectivity.md)
- [Contract v65：地牢实例身份与生命周期](contract-v65-dungeon-instance-identity.md)
- [前端目标模式 v1](frontend-targeting-v1.md)
- [RFB 全系统梳理与重构实现路线](rfb-system-implementation-roadmap.md)
- [核心协议 v1](protocol-v1.md)
- [确定性模拟、随机数与回放](deterministic-simulation.md)
- [内容数据格式 v1](content-format-v1.md)
- [Tileset manifest 与资源回退 v1](tileset-format-v1.md)
- [Tauri 桌面端到端测试](tauri-desktop-e2e.md)
- [Tauri Android 原生目标](android-target.md)
- [新存档格式 v1](save-format-v1.md)
- [桌面原生存档与诊断 v1](desktop-native-storage-v1.md)
- [桌面崩溃诊断闭环 v1](crash-diagnostics-v1.md)
- [授权、版权与素材迁移审计](licensing-and-assets.md)
- [本地化与中文文本重构计划](localization-rewrite-plan.md)
- [Fluent 本地化运行时 v1](fluent-localization-v1.md)
- [桌面分层 RendererBackend v1](renderer-backend-v1.md)
- [Rust 权威可见性与光照 v1](visibility-lighting-v1.md)
- [静态地形 Chunk 渲染 v1](terrain-chunk-rendering-v1.md)

本文档是 Rust/Tauri 重构的长期入口。以后每次实现阶段性功能时，应同步更新“当前进度”“接口版本”和“未决问题”，不要让架构约定只存在于聊天记录中。

## 1. 总目标

将 RoguelikeFansBand 逐步重构为：

- Tauri WebView 中的 HTML/CSS/TypeScript 负责界面、地图显示、tileset、动画和输入；
- Rust 游戏核心负责规则、随机数、地图、战斗、AI、物品和存档；
- 地图渲染与文字/UI 完全分层；
- ASCII、图片 tileset 和未来的 WebGL 视觉效果共用同一套语义化外观接口；
- `player_type`、`object_type`、`monster_type` 等内部结构不再成为外部接口和存档格式；
- 旧存档可以由本地只读导入工具转换，现有规则通过行为测试独立重写；旧数据、文本和素材不进入新仓库或发行包；
- Tauri 2 负责 Windows、Linux、macOS 和 Android 封装，各平台共用原生 Rust 核心与 TypeScript/PixiJS 前端；
- 当前不发布浏览器/PWA 版本，也不继续维护 WASM 核心；如未来恢复网页版本，只新增 `WasmWorkerTransport` 适配器，不改变领域核心。

不采用“一次性把 30 万行 C 翻译成 JavaScript”的方式。Rust 是唯一正式核心语言；现有 C 只用于行为对照、旧存档导入和回归测试。TypeScript 通过传输无关的 `CoreTransport` 调用 Tauri 原生适配器，不能依赖 Rust 内部结构。当前已经完成的 WASM 垂直切片只作为架构验证历史保留，Tauri 原生垂直切片完成后从活动 workspace、前端构建和 CI 中移除。

## 2. 当前工程事实

当前工程约有 215 个 C 文件、30 万行以上代码。主要耦合点包括：

- 旧仓库 `src/cave.c`：`map_info()`、地图显示、可见性和当前地图外观合成；
- 旧仓库 `src/z-term.c`：`Term_queue_*`、脏区和终端缓冲；
- 旧仓库 `src/main-win.c`：Win32 字符/GDI 后端；
- 旧仓库 `src/types.h`：大型运行时结构体；
- 旧仓库 `src/load.c`：按字段顺序读取旧存档；
- 旧仓库 `src/save.c`：按字段顺序写入旧存档。

目前 C 核心直接把地图字符、属性、颜色和 UI 文本送入 `Term` 后端。因此地图、文字窗口、光照、tileset 和平台绘制之间存在明显耦合。

旧仓库不作为新工程子模块或内容依赖提交。开发机通过 `RFB_LEGACY_SOURCE=D:/codex/Frogcomposband/master` 读取本地 Git 仓库，并固定解析 `RFB_LEGACY_REF=v1.3.0.7`；工具不得修改旧工作树。公共构建和正式发行不依赖该目录。

## 3. 目标架构

```text
输入层（键盘／鼠标／触摸／手柄）
        │
        ▼
TypeScript/PixiJS UI（Tauri WebView）
        │
        ▼
CoreTransport
        │
        ▼
Tauri Commands / Events
        │
        ▼
原生 Rust 游戏核心
        │
        ├── Game Events ──────► HTML UI 层
        ├── Game Snapshot ────► Cell Appearance Composer
        └── Save API
                                  │
                                  ├── 地形层
                                  ├── 物品层
                                  ├── 怪物／玩家层
                                  ├── 可见性／记忆层
                                  ├── 光照层
                                  └── 动画／特效层
                                             │
                                             ▼
                                      PixiJS/WebGL 地图
```

地图使用 Canvas/WebGL，消息、人物状态、背包、菜单和对话使用 HTML/CSS。不要用大量 DOM 节点绘制地图格。

推荐技术栈：

- Rust + Cargo + Serde：最终游戏核心；
- TypeScript + Vite：前端应用和工具链；
- PixiJS/WebGL2 作为主要地图渲染器；
- Tauri 2 Commands/Events 连接 TypeScript UI 与原生 Rust 核心；
- Tauri 桌面端作为第一正式运行目标，随后启用 Tauri Android；
- `CoreTransport` 隔离 UI 与具体传输实现；
- MessagePack 作为存档、回放和批量数据载荷，Tauri 控制命令使用 Serde DTO；
- Vitest 做 UI/协议测试，Rust 测试规则核心，Tauri WebDriver/Appium 做桌面与 Android 集成测试。

技术取舍：不把规则写成 TypeScript。TypeScript 只负责 UI 和 WebView 生态，Rust 负责稳定数据模型、存档、随机数和全部权威规则。Tauri IPC 只传输 DTO，前端不能直接修改核心状态。

## 4. 核心分层原则

### 4.1 游戏核心不直接绘图

核心不再调用 `Term_putstr()`、`Term_queue_char()` 或 Win32 GDI。核心只产生：

- 游戏状态快照；
- 游戏事件；
- 需要刷新的格子列表；
- UI 状态变化。

### 4.2 外观合成独立于规则

每个地图格先组成语义数据，再由渲染器决定如何显示：

```ts
interface MapCell {
  terrainId: string;
  objectId?: string;
  actorId?: string;
  visibility: "unknown" | "memory" | "visible";
  light: { intensity: number; color: RGB };
  effects: CellEffect[];
  highlights: CellHighlight[];
}

interface CellAppearance {
  glyph?: string;
  tile?: TileRef;
  foreground: RGB;
  background: RGB;
  opacity: number;
  border?: BorderStyle;
  animation?: AnimationRef;
}
```

合成顺序固定为：地形 → 物品 → 怪物/玩家 → 可见性/记忆 → 光照 → 气体/法术/临时效果 → 光标/路径/危险提示 → 对比度保护。

### 4.3 地图与文字严格分层

```text
游戏规则 → 语义化地图状态 → Cell Appearance → Canvas/WebGL 地图
游戏事件 → 消息事件 → HTML 消息面板
```

地图刷新不能因为消息滚动、人物面板刷新或菜单重排而重新绘制整张地图。

### 4.4 现代保留式渲染架构

本项目不以 Brogue、传统终端或任何单一游戏的渲染方式作为规范。视觉参考可以借鉴，但新架构采用现代 GPU 场景与多通道合成设计。

核心输出语义化 `RenderSnapshot` 或增量 `RenderDelta`。前端维护一个长期存在的 `RenderWorld`，只更新发生变化的对象，而不是每回合重新生成整张地图。

```ts
interface RenderDelta {
  revision: number;
  changedCells: CellRenderData[];
  removedEntities: EntityId[];
  changedEntities: EntityRenderData[];
  lightSources: LightSourceRenderData[];
  cameraHints?: CameraHint[];
}
```

建议渲染通道：

1. `TerrainPass`：静态地形，按 16×16 或 32×32 cell chunk 缓存为 RenderTexture；
2. `DecalPass`：血迹、法阵、陷阱、地面装饰等低频变化内容；
3. `ObjectPass`：物品和可拾取对象；
4. `ActorPass`：玩家、怪物、召唤物和动态实体；
5. `VisibilityPass`：未知、记忆、当前可见区域，使用独立 mask；
6. `LightingPass`：光源写入低分辨率 light buffer，再与场景做 multiply/additive 合成；
7. `EffectsPass`：投射物、爆炸、粒子、屏幕震动和临时动画；
8. `InteractionPass`：光标、路径、目标、范围预览和危险提示；
9. `DebugPass`：网格、碰撞、FOV、光照和性能统计，仅开发模式启用。

光照、可见性和记忆状态必须是独立纹理/mask，不能烘焙回地形颜色。这样移动光源不会重建地形 chunk，也不会出现光源颜色被邻接地块永久污染的问题。

PixiJS 作为场景、纹理、batch 和跨 WebGL/WebGPU 的适配层，但业务代码不能直接散落 PixiJS 对象。建立内部 `RendererBackend` 接口：

```ts
interface RendererBackend {
  initialize(target: HTMLCanvasElement): Promise<void>;
  applyDelta(delta: RenderDelta): void;
  resize(viewport: Viewport): void;
  render(frame: FrameContext): void;
  dispose(): void;
}
```

初期实现 `PixiRendererBackend`，测试和低配兼容可实现 `CanvasRendererBackend`。如果未来需要直接使用 WebGPU，不必修改游戏核心和 UI 协议。

性能策略：

- 静态地形 chunk 化和缓存；
- 动态实体使用 GPU instancing/sprite batching；
- 相机视野外对象剔除；
- 使用 dirty cells/deltas，不做无条件整屏重建；
- Tauri IPC 的高频地图更新优先传紧凑 delta；只有性能分析证明必要时才切换为 TypedArray/二进制批量通道；
- 临时动画只存在于前端 RenderWorld，不写入核心存档；
- 固定像素 tileset 默认使用 nearest-neighbor，高清素材可选择 linear；
- 光照缓冲允许低分辨率渲染后上采样，以控制低端显卡开销；
- 地图坐标选择通过相机逆变换完成，不依赖逐 sprite DOM/GPU picking。

ASCII 地图也视为一种 tileset：使用动态 glyph atlas 或 bitmap font atlas，经过同一 `ActorPass`/`TerrainPass` 绘制，而不是另建一套终端式渲染管线。中文消息和复杂排版继续由 HTML UI 层负责。

## 5. Tauri 原生核心接口 v1

TypeScript 依赖传输无关接口，正式实现为 `TauriNativeTransport`：

```ts
interface CoreTransport {
  createGame(options: NewGameOptions): Promise<void>;
  loadGame(data: Uint8Array): Promise<void>;
  saveGame(): Promise<Uint8Array>;
  dispatch(command: GameCommand): Promise<GameUpdate>;
  getSnapshot(): Promise<GameSnapshot>;
}

type GameCommand =
  | { type: "move"; direction: Direction }
  | { type: "pick-up" }
  | { type: "wait" }
  | { type: "use-item"; itemId: ItemId }
  | { type: "cast"; spellId: SpellId; target?: Target }
  | { type: "select-target"; position: Position };

interface GameUpdate {
  turn: number;
  events: GameEvent[];
  dirtyCells: CellPosition[];
  uiChanges: UIChange[];
}
```

接口必须版本化，并使用序列化 DTO；前端不能持有 Rust 引用或修改核心内部对象。未来需要网页版本时，可以实现相同接口的 `WasmWorkerTransport`，但不进入当前开发范围。

## 6. Tileset 设计

核心只输出语义 ID，不保存图集坐标：

```text
terrain.wall.granite
terrain.floor.stone
monster.dragon.red
item.weapon.long_sword
effect.fireball
```

tileset 通过 manifest 映射：

```json
{
  "formatVersion": 1,
  "id": "classic-16x16",
  "tileWidth": 16,
  "tileHeight": 16,
  "atlas": "tiles.png",
  "mappings": {
    "terrain.wall.granite": [4, 2],
    "terrain.floor.stone": [1, 0],
    "monster.dragon.red": [12, 8]
  }
}
```

必须支持：

- ASCII 字体模式；
- 图片 tileset；
- 不同 tile 尺寸和窗口缩放；
- 缺失 tile 自动回退 glyph；
- tileset 热切换；
- 动画 tile 和独立特效图集；
- 地形自动连接；
- 用户自定义映射。

## 7. 数据模型重构

不要把 C 的大型结构体原样搬到 Rust 或 TypeScript。Rust 内部拆成领域对象；TypeScript 只接收序列化 DTO。以下 TypeScript 类型用于描述跨边界数据，不代表核心内部实现：

```ts
interface PlayerState {
  id: PlayerId;
  identity: PlayerIdentity;
  progression: PlayerProgression;
  resources: PlayerResources;
  position: Position;
  inventory: InventoryState;
  equipment: EquipmentState;
  statuses: StatusCollection;
  abilities: AbilityCollection;
}

interface ItemInstance {
  id: ItemId;
  kindId: ItemKindId;
  quantity: number;
  enchantments: Enchantment[];
  affixes: AffixId[];
  flags: ItemFlagId[];
  location: ItemLocation;
  metadata: Record<string, unknown>;
}
```

规则：

- 物品种类与物品实例分离；
- 使用稳定字符串 ID，不依赖数组下标；
- 临时计算值不写入存档；
- UI 不直接修改核心对象；
- 增加字段必须有默认值；
- 删除或改名必须提供迁移器；
- 外部模块只依赖 DTO 和命令，不依赖内部 struct 布局。

Rust 内部优先使用强类型 ID、新类型包装和显式聚合，避免把所有数据重新塞进一个巨型 `Player`：

```rust
pub struct PlayerState {
    pub id: PlayerId,
    pub identity: PlayerIdentity,
    pub progression: PlayerProgression,
    pub resources: PlayerResources,
    pub position: Position,
    pub inventory: Inventory,
    pub equipment: Equipment,
    pub statuses: StatusCollection,
    pub abilities: AbilityCollection,
}
```

## 8. 新存档策略

独立兼容工具从 `RFB_LEGACY_SOURCE` 或玩家选择的旧存档读取旧格式，并导出新格式。Rust 核心只写新格式；导入器代码可以进入新仓库，但旧存档样本、旧内容数据和转换输出只保存在 `.local/`。

新存档使用版本化 RFB 容器和 MessagePack 载荷，开发阶段可导出 JSON；完整容灾规则见[新存档格式 v1](save-format-v1.md)：

```ts
interface SaveGame {
  format: "rfb-save";
  formatVersion: number;
  gameVersion: string;
  metadata: SaveMetadata;
  world: WorldSave;
  player: PlayerSave;
  levels: LevelSave[];
  rng: RandomState;
}
```

迁移必须是连续的 `v1 → v2 → v3`，而不是让每个版本都直接兼容所有历史格式。

## 9. 分阶段路线

### 阶段 0：行为基准

- 固定旧版 `v1.3.0.7` / `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9`；
- 固定随机种子测试；
- 关键战斗、物品生成和状态效果测试；
- 输入录像/回放；
- 旧存档读取与保存回环；
- 记录当前 `Term`、`map_info()` 和窗口刷新流程；
- 建立截图和地图快照基准。

具体格式和完成门槛见[旧版行为基准与差分测试规范](legacy-behavior-baseline.md)。阶段 0 可以并行创建最小 Cargo workspace 和测试工具，但行为基准没有达到门槛前，不批量迁移规则模块。

### 阶段 1：Rust 工作区与协议骨架

- 建立 Cargo workspace；
- 建立 `rfb-core`、`rfb-protocol`、`rfb-content`、`rfb-save` 和 `rfb-replay` crate；
- 建立 Tauri 2 应用和 `TauriNativeTransport`；
- 定义稳定 ID、随机数、命令、事件、快照和错误类型；
- 实现最小地图、玩家位置和回合推进；
- C 版本继续作为规则基准，不在 Rust 中调用 Win32/Term。

验收：Rust 单元测试中可以创建最小世界、移动玩家、输出确定性的 `GameUpdate`，并完成新格式存档回环。

### 阶段 2：Tauri + TypeScript + PixiJS 前端 MVP

- 建立 Vite/TypeScript 前端；
- 使用 PixiJS 建立地图 stage 和 sprite batch；
- HTML 消息区；
- HTML 人物状态和背包；
- 键盘输入；
- Tauri Commands/Events 与原生 Rust 核心通信；
- 新格式存档导入/导出；
- 一个 ASCII tileset 和一个图片 tileset。

验收：Windows Tauri 应用中完成一段完整的移动和战斗流程，地图与文字互不覆盖；核心不依赖 WebView 和平台 UI。

### 阶段 3：现代地图渲染

- `RendererBackend` 接口和 PixiJS 实现；
- Terrain chunk cache 和动态实体 batch；
- 独立 Visibility/Memory mask；
- 独立低分辨率 Lighting buffer；
- Decal、Actor、Effect、Interaction 多通道；
- 动画和投射物只存在于前端 RenderWorld；
- 路径、目标、危险边框作为最后的 InteractionPass；
- dirty cells/RenderDelta 局部刷新；
- resize、缩放、最小化/恢复回归测试。

### 阶段 4：桌面与 Android 跨平台能力

- 桌面版复用同一套 TypeScript/PixiJS 前端；
- Windows、Linux、macOS 和 Android 均运行原生 Rust 核心；
- 实现桌面文件选择、存档目录、日志、崩溃报告和自动更新接口；
- 实现 Android 应用生命周期、返回键、存档分享、触摸输入和横竖屏布局；
- 所有平台使用相同协议和内容数据。

验收：同一个存档和固定种子在 Windows、Linux、macOS 和 Android 原生核心中产生一致的事件和状态哈希。

### 阶段 5：逐模块完成 Rust 规则核心

建议顺序：内容加载 → 随机数与回放 → 地图 → 物品 → 玩家状态 → 状态效果 → 怪物 → 战斗 → 法术 → AI → 地图生成/任务。

详细的系统边界、依赖顺序、内容规模和 contract-v8 之后的实施路线见 [RFB 全系统梳理与重构实现路线](rfb-system-implementation-roadmap.md)。实际推进以该文档的行动调度、效果管线、派生属性和知识状态四个公共底座为先，不按旧 C 文件顺序批量迁移。

每完成一个 Rust 模块，都要使用固定种子、事件日志和快照与 C 版本对照。任何模块没有测试和接口隔离时，不进入大规模替换。

### 阶段 6：插件和公共接口

提供数据包、tileset、本地化、UI 扩展、命令、事件、存档迁移和只读观察器接口。

第一版插件优先使用声明式 JSON 数据，不允许任意脚本直接改内部状态。

## 10. 第一里程碑

第一个目标不是“完整重写”，而是：

> 用 Tauri 原生 Rust 核心驱动 TypeScript + PixiJS 地图，并使用独立 HTML 消息面板完成移动、基础战斗、拾取和新格式存档。

交付物：

- Cargo workspace 和 `rfb-core`；
- `web/` TypeScript/Vite 工程骨架；
- Tauri 2 Windows 应用和原生 Rust transport；
- Command API v1；
- Snapshot API v1；
- PixiJS 地图和 ASCII glyph atlas；
- HTML 消息/人物面板；
- tileset manifest v1；
- 一个 ASCII 和一个图片 tileset；
- 固定种子回放测试；
- Windows Tauri 可玩演示；
- Tauri Android 工程可以编译并加载同一前端。

## 11. 测试与质量门槛

每个阶段至少通过：

- 编译检查；
- 固定随机种子回放；
- 存档读写回环；
- 中文文本宽度测试；
- 宽字符和行尾测试；
- 地图/文字分层截图测试；
- ASCII/tileset 双后端一致性测试；
- resize、缩放、最小化恢复测试；
- 不同 tileset 缺失资源回退测试。

视觉层尤其要避免：

- 规则代码直接设置颜色；
- 后端即时绘制绕过脏区；
- 光照结果写回地形定义；
- 以整屏重绘代替 dirty cells；
- 颜色和 glyph 状态分散在多个不可追踪缓存中。

## 12. 当前进度与下一步

当前状态：P0 规范和 Tauri 2 Windows 原生垂直切片已经建立。WASM 原型完成架构验证后已经退出活动代码；workspace、前端、依赖和 CI 不再包含 `rfb-wasm`、Web Worker、wasm-pack 或 wasm32 构建目标。

已完成：

- 20×20 原创测试地图、玩家移动、等待和基础攻击；
- xoshiro256** 固定种子 RNG、command sequence、revision 和 state hash；
- MessagePack Command/Snapshot/Update DTO；
- RFB 存档容器、SHA-256 校验和与读写回环；
- 本地 `v1.3.0.7` Git 对象探针和 `.local/` manifest；
- PixiJS 保留式 cell 对象和 changed cells 局部更新；
- 地图 Canvas 与 HTML 消息/状态严格分层；
- `CoreTransport` 与正式 `TauriNativeTransport`；
- Tauri Commands 驱动的原生 Rust 游戏会话；
- Windows Tauri Release EXE 构建；
- 小键盘、Vi 和 WASD 三套互斥移动预设；
- WASM crate、Worker 和相关构建链清理；
- `rfb-contract` 契约测试驱动和首批 20 个原创 exact fixtures；
- `rfb-replay` v1、每 100 命令/最终检查点和 10,000 回合无漂移测试；
- Tauri 原生会话 `ReplayRecorder`、`.rfbreplay` 诊断导出和前端下载入口；
- Rust DTO 自动生成 `web/src/protocol.ts`、JSON Schema 和 CI 漂移检查；
- `rfb-content`、内容 Schema、确定性 MessagePack 编译器和首个原创 JSON 内容包；
- 动态 ASCII glyph atlas、tileset manifest v1、原创 SVG 图片 atlas、热切换和缺失资源回退；
- 3 个 `.local/` 旧存档样本清单和快照规范化 Schema v1；
- `rfb-legacy-import` 链式 XOR 解码、稳定前缀解析和 3 个本地样本字段断言；
- baseline policy v1、fixture 级差异豁免格式和 CI 验证；
- Cargo 测试、TypeScript 检查和 Vite UI 构建；
- GitHub Actions 基础 CI。
- Tauri 内嵌 WebDriver 桌面端到端测试，覆盖 dirty cells、地图/消息分层、存档读写、回放导出和 tileset 热切换；
- E2E 失败截图与进程日志归档，测试驱动受 debug feature 和显式端口双重限制。
- Tauri Android Gradle/Kotlin 工程、ARM64 Rust target 和 Debug APK 构建链；
- Android CI 可重复构建并上传未签名调试 APK。
- `.rfbcontent` 运行时解码、稳定内容索引和内容驱动的世界创建；
- 协议 1.3 包含内容视觉目录、地图物品、背包 DTO、真实 content/world 身份和权威视觉格；
- contract-v2 与 state hash Schema v2 基准迁移，contract-v1 历史保留。
- `PickUp` 拾取命令、确定性堆叠、HTML 背包面板和存档/回放闭环；
- contract-v3 与 state hash Schema v3 基准迁移，新增成功/空地拾取场景。
- Fluent `zh-CN`/`en-US` 双语运行时、Rust/TypeScript 共用资源、语言热切换和中文默认界面；
- 消息历史、背包、内容名称、按键提示和主要桌面 UI 已移除业务文案硬编码。
- `RendererBackend`/`PixiRendererBackend`、RenderWorld 和地形/物品/角色/可见性/光照五层已建立；
- 玩家阅读光使用独立 buffer 与有限 dirty footprint，glyph 绘制加入对比度保护；
- 整图/玩家居中两种镜头模式、15×15 玩家视口、地图边缘钳制和 ResizeObserver 相机重算已建立；镜头只变换 PixiJS 世界容器，不影响 dirty cells、存档、回放或 state hash。
- 75%–200% 五档画面缩放已接入同一相机容器；整图与玩家居中模式共用缩放，Canvas resize 不重建节点或重新提交 RenderCell。
- 协议 1.3 已提供 Rust 权威 FOV、探索记忆和内容标签光源；前端临时 `all-visible`/阅读光已移除，记忆/隐藏格不再暴露当前物品和角色。
- Tauri 应用私有存档目录、命名存档槽、原子替换、三份备份、损坏恢复、结构化错误和本地诊断日志已建立；手动 `.rfbsave` 导入/导出继续保留。
- 桌面 E2E 已覆盖原生槽的新建、列表、载入后命令序列同步、覆盖和删除。
- PixiJS backend 已升级为 `pixi-layered-chunks-v3`：默认 16×16 静态地形 RenderTexture、按 chunk 的五层分组、玩家居中视口外剔除、可见 chunk 动态视图复用和缓存重建诊断已建立。
- 协议 1.4、state hash Schema v4 和 contract-v4 已建立；Rust 权威装备列表、装备/卸下、完整物品堆批量丢弃、HTML 多选背包和原创回声护符已进入存档/回放闭环。
- 协议 1.5、state hash Schema v5 和 contract-v5 已建立；回声护符提供实际 `maxHp +4`，玩家基础/装备/最终属性分层输出，`generated.item.N` 分配器进入存档与回放，HTML 背包支持单堆指定数量丢弃。原创内容包升级到 1.2.0，旧 1.0.0/1.1.0 内置存档有显式迁移。
- 协议 1.6、state hash Schema v6 和 contract-v6 已建立；角色内容定义提供基础攻击/防御，装备修正扩展到攻击、防御和最大生命，玩家撞击怪物的伤害由 Rust 使用 `max(1, attack + rng(0..1) - defense)` 权威结算。原创内容包升级到 1.3.0，并显式迁移 1.2.0 内置存档。
- 协议 1.7、state hash Schema v7 和 contract-v7 已建立；玩家/怪物近战使用 RFB 风格 5% 自动命中/失误、攻击能力对抗 AC、内容伤害骰和普通物理 AC 减伤。邻接怪物在玩家回合末按稳定顺序反击，玩家受伤、负生命死亡、死亡事件、存档/回放和死亡后命令拒绝已经闭环。原创内容包升级到 1.4.0，并显式兼容 1.3.0 内置存档。
- 协议 1.8、state hash Schema v8 和 contract-v8 已建立；`GameAction`、标准行动成本、原创整数速度曲线、`worldTick`、稳定怪物调度、八方向 BFS 追踪和死亡队列中断已进入存档/回放闭环。原创内容包升级到 1.5.0，并显式兼容 1.0.0–1.4.0 的已知内置内容 hash。
- 协议 1.9、state hash Schema v9 和 contract-v9 已建立；玩家/怪物状态、玩家抗性、加速/减速派生速度、毒素 tick、过期和持续伤害死亡已进入存档/回放闭环。active baseline 共 36 个 exact fixtures，内容包继续使用 1.5.0。
- 协议 1.10 和 contract-v10 已建立；流血周期伤害、内容驱动近战伤害类型、火焰抗性/免疫已经进入规则闭环。内容包升级到 1.6.0，state hash Schema 继续为 v9，active baseline 共 39 个 exact fixtures。
- 协议 1.11 和 contract-v11 已建立；伤害/死亡事件携带结构化 outcome，内容包 1.7.0 为酸、电、火、冷、毒提供独立近战来源；来源可追踪的派生属性管线与结构化检定结果已接管现有装备、速度和近战命中，眩晕能力削弱与恐惧行动限制已接入。active baseline 共 47 个 exact fixtures，state hash Schema 继续为 v9。
- 协议 1.12 和 contract-v12 已建立；内容包 1.8.0 新增回声刃及武器 `AttackProfile`，玩家多段近战按稳定顺序逐击并在死亡后立即中断。active baseline 共 48 个 exact fixtures，save v1 / state hash Schema v9 不变。
- 协议 1.13 和 contract-v13 已建立；内容包 1.9.0 新增回声猎犬及怪物 `MeleeRoutine`，逐 blow 独立命中/伤害、method 事件参数和死亡中断已闭环。active baseline 共 49 个 exact fixtures，save v1 / state hash Schema v9 不变。
- 协议 1.14 和 contract-v14 已建立；内容包 1.10.0 新增共鸣投射器及发射器 `projectileProfile`，方向射击、权威直线轨迹、边界/墙壁/首目标碰撞和结构化 trace 已闭环。active baseline 共 50 个 exact fixtures，save v1 / state hash Schema v9 不变。
- 协议 1.15 和 contract-v15 已建立；内容包 1.11.0 新增共鸣弹丸与稳定 `ammoKindId`，射击弹药消费、碰撞/落点分离和单件投掷实例事务已闭环。active baseline 共 52 个 exact fixtures，save v1 / state hash Schema v9 不变。
- 协议 1.16 和 contract-v16 已建立；方向/格子/实体 `TargetSpec`、稳定 `TargetSelection`、目标前置校验和非八方向整数轨迹已闭环。active baseline 共 53 个 exact fixtures，内容包 1.11.0、save v1 / state hash Schema v9 不变。
- 前端目标模式 v1 已建立；`F`/按钮进入、三套方向预设移动准星、`Enter` 确认、`Esc` 取消，并按稳定实体 ID 或格子提交 `FireTarget`。准星跟随相机与缩放，不进入权威状态。
- 协议 1.17 和 contract-v17 已建立；内容包 1.12.0 通过 `breakChancePercent` 声明折损率，撞击实体后确定性检定破损，未撞实体时弹药无 RNG 落地回收。active baseline 共 54 个 exact fixtures，save v1 / state hash Schema v9 不变。
- 协议 1.18 和 contract-v18 已建立；内容包 1.13.0 为物品声明整数磅十分位重量与可选投掷 profile，重量决定 2–10 格射程，投掷独立完成命中、伤害和权威落点事务。active baseline 共 55 个 exact fixtures，save v1 / state hash Schema v9 不变。
- 协议 1.19 和 contract-v19 已建立；内容包 1.14.0 为玩家声明整数携带容量，核心汇总背包/装备重量并原子拒绝超限整堆拾取，HTML 背包显示权威总重/容量。active baseline 共 56 个 exact fixtures，save v1 / state hash Schema v9 不变。
- 协议 1.20 和 contract-v20 已建立；内容包 1.15.0 为可伪装物品声明外观名称，核心保存种类级 unknown/tried/aware 知识并决定名称和隐藏 profile 投影，HTML 只消费 `displayNameKey`。active baseline 共 57 个 exact fixtures，save v1 新增可选知识字段，state hash 升至 Schema v10。
- 协议 1.21 和 contract-v21 已建立；内容包 1.16.0 为发光碎片声明首个治疗 `UseAction`，核心原子消耗单件并按实际治疗结果更新 tried/aware，HTML 通过 `usable` 提供使用入口。active baseline 共 58 个 exact fixtures，save v1 / state hash Schema v10 不变。
- 协议 1.22 和 contract-v22 已建立；内容包 1.17.0 新增独立 affix 根与固定实例引用，核心首次装备时发现词条，HTML 只渲染 `knownProperties`。active baseline 共 59 个 exact fixtures，save v1 / state hash Schema v11。
- 协议 1.23 和 contract-v23 已建立；内容包 1.18.0 为实例保存质量，HTML 的鉴别入口只显示质量，装备后才显示完整词条。active baseline 共 60 个 exact fixtures，save v1 / state hash Schema v12。
- 协议 1.24 和 contract-v24 已建立；内容包 1.19.0 新增加权掉落表和怪物引用，死亡生成按物品/品质/词条固定消耗三次 RNG，并复用既有鉴别投影。active baseline 共 61 个 exact fixtures，save v1 / state hash Schema v12 不变。
- 协议 1.25 和 contract-v25 已建立；内容包 1.20.0 新增怪物出生携带表，核心保存 `CarriedBy(actorId)` 所有权并在统一死亡事务中先放下真实实例、再生成普通掉落。active baseline 共 62 个 exact fixtures，save v1 新增可选携带列表，state hash 升至 Schema v13。
- 协议 1.26 和 contract-v26 已建立；内容包 1.21.0 新增稳定入口层、程序化层和上下楼梯地形，核心保存离层 `FloorState` 并在首次进入时按固定四次 RNG 生成双房间/L 形走廊。active baseline 共 63 个 exact fixtures，save v1 新增当前/离层楼层字段，state hash 升至 Schema v14。
- 协议 1.27 和 contract-v27 已建立；内容包 1.22.0 新增楼层深度、稳定房间怪物/掉落生成项和第三个 loot table，首次生成固定完成怪物种类/位置与地面 loot，返回不重新抽取。active baseline 共 64 个 exact fixtures，save v1 / state hash Schema v14 不变。
- 协议 1.31 和 contract-v31 已建立；内容包 1.25.0 新增隐藏门投影和搜索能力，核心只向普通格子 DTO/交互查询输出玩家已知 terrain，前端使用大写 S 主动搜索。active baseline 共 68 个 exact fixtures，save v1 / state hash Schema v15。
- 协议 1.39 和 contract-v39 已建立；内容包 1.33.0 把任务目标扩展为稳定实例收集与击杀，任务日志投影 `current/required`，一次性讨伐层复用普通死亡、掉落、奖励和入口关闭管线。active baseline 共 77 个 exact fixtures，save v1 / state hash Schema v15 不变。
- 协议 1.40 和 contract-v40 已建立；内容包 1.34.0 新增任务提前退出政策、显式放弃命令与独立 abandoned 状态，前端为 active 任务提供放弃入口。active baseline 共 79 个 exact fixtures，save v1 / state hash Schema v15 不变。
- 协议 1.41 和 contract-v41 已建立；内容包 1.35.0 新增按 actor kind 累计的数量击杀目标，save v1 保存任务计数，state hash 升至 Schema v16。active baseline 共 81 个 exact fixtures。
- 协议 1.42 和 contract-v42 已建立；内容包 1.36.0 新增可重接任务、paused/resumed 状态和完整任务层恢复。active baseline 共 83 个 exact fixtures，save v1 / state hash Schema v16 不变。
- 协议 1.43 和 contract-v43 已建立；内容包 1.37.0 新增独立 task ID、跨入口共享进度与整组入口结算。active baseline 共 85 个 exact fixtures，save v1 / state hash Schema v16 不变。
- 协议 1.44 和 contract-v44 已建立；任务状态迁入权威 `TaskState`，击杀和拾取进度改由集中领域事件消费器推进，state hash 升至 Schema v17。active baseline 共 86 个 exact fixtures，内容包继续为 1.37.0。
- 协议 1.45 和 contract-v45 已建立；内容包 1.38.0 新增跨成员楼层的有序 `taskStages`，任务状态保存当前阶段，任务日志显示阶段编号，state hash 升至 Schema v18。active baseline 共 88 个 exact fixtures。
- 协议 1.46 和 contract-v46 已建立；内容包 1.39.0 将回声地牢扩展为三层，新增显式最终层、确定性守护者、持久击败状态和守护者死亡事件，state hash 升至 Schema v19。active baseline 共 91 个 exact fixtures。
- 协议 1.47 和 contract-v47 已建立；内容包 1.40.0 新增独立 vault 根，深度 2 的主题模板绘制隐藏入口与固定 terrain，按深度加权表生成 3 人群组并使用专属 loot table。save v1 / state hash Schema v19 不变，active baseline 共 92 个 exact fixtures。
- 协议 1.48 和 contract-v48 已建立；内容包 1.41.0 新增独立 encounter/theme 根和楼层表引用，普通房间怪物/掉落改为表驱动，深度 2 在两个主题 Vault 间确定性加权选择，深度 1 建立首个同类巢穴，深度 3 验证无 Vault 候选回退。save v1 / state hash Schema v19 不变，active baseline 共 96 个 exact fixtures。
- 协议 1.49 和 contract-v49 已建立；内容包 1.42.0 新增 actorSlots/lootPlacements 总预算和独立十层压力地牢，深度 4 切换第二主题，最终层按 10 actor/3 loot placement 规模生成。save v1 / state hash Schema v19 不变，active baseline 共 99 个 exact fixtures。
- 协议 1.50 和 contract-v50 已建立；内容包 1.43.0 新增 Vault 八向变换、边界入口、自由 wall 区落位、多 Vault 数量/面积预算、重叠拒绝与稳定失败回退。深度 8 在 9 actor/3 loot placement 内落位两个小模板并跳过无法落位的 12×12 候选；save v1 / state hash Schema v19 不变，active baseline 共 100 个 exact fixtures。
- 协议 1.51 和 contract-v51 已建立；内容包 1.44.0 新增动态 friends/escort、`cluster/ring` formation、群体数量/随从 actor 预算、空间缩减与原子回退。深度 6/7 分别生成 ring/cluster 群体并保持 7/8 actor 总预算；save v1 / state hash Schema v19 不变，active baseline 共 102 个 exact fixtures。
- 协议 1.52 和 contract-v52 已建立；内容包 1.45.0 新增独立 terrain feature 表、room/corridor 放置、深度权重、额外特殊地形预算、占位排斥与失败回退。压力地牢深度 3–10 放置 2–4 个额外 trap/rubble/door；save v1 / state hash Schema v19 不变，active baseline 共 104 个 exact fixtures。
- 协议 1.56 和 contract-v56 已建立；内容包 1.49.0 在分阶段 layout 中新增原版式复合 pit、专属 encounter table、密集等级阵列和 footprint 保留。save v1 / state hash Schema v19 不变，active baseline 共 112 个 exact fixtures。
- 协议 1.57 和 contract-v57 已建立；内容包 1.50.0 新增 `layout.mode = maze-only`，深度 9 完全跳过房间/走廊并以连通 maze 区域落位楼梯、encounter 和 loot，pit 移到深度 10。save v1 / state hash Schema v19 不变，active baseline 共 114 个 exact fixtures。
- 协议 1.58 和 contract-v58 已建立；内容包 1.51.0 新增稳定楼层连接 ID、两组独立普通楼梯和跨两层 shaft，附加连接按种子 RNG 随机落位并保存 ID→位置。save 容器仍为 v1，state hash 升至 Schema v20，active baseline 共 117 个 exact fixtures。
- 协议 1.59 和 contract-v59 已建立；内容包 1.52.0 为动态群体新增行为声明，生成稳定 pack ID、leader/member 身份和 `seek/surround/guard-leader` 首版 AI。save 容器仍为 v1，state hash 升至 Schema v21，active baseline 共 117 个 exact fixtures。
- 协议 1.60 和 contract-v60 已建立；内容包 1.53.0 新增 region table 根、权重无放回区域选择、按房间中心归属的局部 terrain/encounter/loot 和持久区域边界。save 容器仍为 v1，state hash 升至 Schema v22，active baseline 共 119 个 exact fixtures。
- 协议 1.61 和 contract-v61 已建立；内容包 1.54.0 新增 paused 任务的地表放弃、重接次数限制和保留进度的确定性成员层重建。save v1 新增带默认值的 `retakesUsed`，state hash 升至 Schema v23，active baseline 共 121 个 exact fixtures。
- 协议 1.62 和 contract-v62 已建立；内容包 1.55.0 允许区域与 Vault、动态群体、terrain feature、pit、guardian、分阶段地貌和显式连接组合，区域特殊 footprint 持久归属宿主并限制区域怪物寻路。save v1 与 state hash Schema v23 不变，active baseline 共 125 个 exact fixtures。
- 协议 1.63 和 contract-v63 已建立；内容包 1.56.0 新增独立 dungeon 定义、单根楼层树、不同楼梯进入不同子层、多个程序化最终叶层与共享守护者镜像。击败任一镜像只结算一次征服，并删除其他已生成镜像、抑制未访问镜像。save v1 与 state hash Schema v23 不变，active baseline 共 127 个 exact fixtures。
- 协议 1.64 和 contract-v64 已建立；内容包 1.57.0 将 Vault 升级为 1–8 个边界入口，加载时证明模板内部连通，落位时为每个入口生成最长 12 格的确定性 BFS connector，并以整层连通证明和原子回退拒绝失败候选。save v1 与 state hash Schema v23 不变，active baseline 共 129 个 exact fixtures。
- 协议 1.65 和 contract-v65 已建立；运行时 dungeon floor 分配稳定实例序号，当前/离层存档保存实例 ID，仓库键按实例+floor 隔离，返回地表只清理当前实例，v64 存档缺失字段按首实例迁移。save v1 仍为容器 v1，state hash 升至 Schema v24，active baseline 共 131 个 exact fixtures。
- 桌面崩溃诊断闭环 v1 已建立：活动会话标记、正常退出清理、Rust panic/未正常退出的下次启动恢复、前端未处理异常即时报告、256 KiB 脱敏日志尾部和最近 5 份 `.rfbdiagnostic` 自动轮换均已接入；不提供手动日志导出，也不自动上传。
- 192×64 原创渲染压力场景和 profile Schema v1 已接入 Windows E2E/CI artifact；8/16/32 格对比后默认 chunk 调整为 16。`visible-chunk-reuse-v1` 已把 16 格玩家居中模式的动态 Pixi 对象从整图理论值 86,016 降到 7,168，初始化约从 133 ms 降到 30 ms；不可见格仍保留最新语义数据，整图滚动模式保持完整显示。

下一步建议：

1. 继续 Stage E 地牢生态，推进同一 dungeon 多实例显式选择、动态探索树和实例淘汰策略；
2. 补充 resize、最小化/恢复和 DPI 场景；整图滚动矩形虚拟化等到更大可玩地图需要整图模式时再实现；
3. 根据真实硬崩溃报告决定是否增加 Windows minidump，不预先引入自动上传服务；
5. 新功能继续同步增加 Fluent 文本，发现实际可见英文时按场景修正，不主动重扫旧 RFB 文本；Android 继续只保留编译 CI，真机、触屏和生命周期测试暂缓。

每完成一个阶段，都应在本文件更新：

- 当前阶段；
- 已完成接口；
- 兼容性变化；
- 测试结果；
- 新增风险和未决决策。
