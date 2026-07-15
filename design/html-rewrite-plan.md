# RoguelikeFansBand Rust/Tauri 重构计划

状态：原生 Rust + Tauri 2 技术路线已确定，WASM 目标已停止维护

目标仓库：`UncleFvcker/RoguelikeFansBand-Rewrite`

关联文档：

- [旧版行为基准与差分测试](legacy-behavior-baseline.md)
- [核心协议 v1](protocol-v1.md)
- [确定性模拟、随机数与回放](deterministic-simulation.md)
- [内容数据格式 v1](content-format-v1.md)
- [新存档格式 v1](save-format-v1.md)
- [授权、版权与素材迁移审计](licensing-and-assets.md)
- [本地化与中文文本重构计划](localization-rewrite-plan.md)

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
- 3 个 `.local/` 旧存档样本清单和快照规范化 Schema v1；
- Cargo 测试、TypeScript 检查和 Vite UI 构建；
- GitHub Actions 基础 CI。

下一步建议：

1. 建立基准更新审批文件和差异豁免格式；
2. 为 3 个本地旧存档增加字段级解析和导入断言；
3. 将 `ReplayRecorder` 接入 Tauri 诊断导出；
4. 从 Rust 协议 Schema 自动生成 TypeScript 类型，替换当前手写镜像；
5. 建立 `rfb-content` 和第一个原创 JSON 内容包；
6. 加入 ASCII glyph atlas、图片 tileset manifest 和缺失资源回退；
7. 为地图局部更新、消息分层和存档交互建立 Tauri 端到端测试；
8. 建立 Tauri Android target，并验证与 Windows 使用同一原生核心。

每完成一个阶段，都应在本文件更新：

- 当前阶段；
- 已完成接口；
- 兼容性变化；
- 测试结果；
- 新增风险和未决决策。
