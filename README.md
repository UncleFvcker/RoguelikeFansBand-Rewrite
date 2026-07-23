# RoguelikeFansBand Rewrite

RoguelikeFansBand 的新一代重构工程。

本仓库不直接复制旧 C 工程，而是以稳定协议和行为测试为边界，逐步重新实现游戏核心与前端。

## 技术方向

- Rust：游戏规则、数据模型、随机数、存档、AI 与原生核心
- TypeScript + Vite：Tauri WebView 界面和开发工具
- PixiJS：地图、tileset、光照与动画渲染
- Tauri 2 IPC：TypeScript UI 与原生 Rust 核心通信
- Tauri 2：Windows、Linux、macOS 和 Android 封装
- Fluent：英文/简体中文本地化

当前不以浏览器/PWA 为发布目标，也不维护 WASM 核心。UI 通过 `CoreTransport` 连接 `TauriNativeTransport`；未来如确有网页需求，再单独增加 WASM 适配器。

## 设计文档

- [Rust/Tauri 重构计划](design/html-rewrite-plan.md)
- [旧版行为基准与差分测试](design/legacy-behavior-baseline.md)
- [Contract 基准更新与差异豁免政策](design/baseline-update-policy.md)
- [Contract v2 内容运行时迁移](design/contract-v2-content-migration.md)
- [Contract v3 背包权威状态迁移](design/contract-v3-inventory-migration.md)
- [Contract v4 装备与批量丢弃迁移](design/contract-v4-equipment-migration.md)
- [Contract v5 装备属性与物品实例迁移](design/contract-v5-item-instance-migration.md)
- [Contract v6 基础战斗属性迁移](design/contract-v6-combat-stats-migration.md)
- [Contract v7：RFB 风格基础近战闭环](design/contract-v7-rfb-melee-migration.md)
- [Contract v8：行动能量、速度与怪物追踪](design/contract-v8-action-energy-tracking.md)
- [Contract v9：状态、抗性与效果管线](design/contract-v9-status-resistance-effects.md)
- [Contract v10：流血与内容驱动元素近战](design/contract-v10-bleeding-elemental-melee.md)
- [Contract v11：结构化伤害事件、派生属性与检定底座](design/contract-v11-structured-damage-events.md)
- [Contract v12：武器 AttackProfile 与玩家多段近战](design/contract-v12-weapon-attack-profile.md)
- [Contract v13：怪物 MeleeRoutine 与稳定 blow 顺序](design/contract-v13-monster-melee-routines.md)
- [Contract v14：权威 projectile 与发射器基础](design/contract-v14-projectile-foundation.md)
- [Contract v15：弹药事务与投掷落点](design/contract-v15-ammunition-throwing.md)
- [Contract v16：核心目标选择与非八方向轨迹](design/contract-v16-target-selection.md)
- [Contract v17：弹药破损与落地回收](design/contract-v17-ammunition-recovery.md)
- [Contract v18：重量射程与投掷攻击](design/contract-v18-thrown-attacks.md)
- [Contract v19：携带重量与拾取容量](design/contract-v19-inventory-capacity.md)
- [Contract v20：物品知识与未知名称投影](design/contract-v20-item-knowledge.md)
- [Contract v21：消耗品 UseAction 与可观察鉴定](design/contract-v21-consumable-use-action.md)
- [Contract v22：实例词条与知识投影](design/contract-v22-instance-affix-knowledge.md)
- [Contract v23：物品鉴别与完整识别](design/contract-v23-item-appraisal.md)
- [Contract v24：确定性战利品生成](design/contract-v24-deterministic-loot-generation.md)
- [Contract v25：怪物携带物与统一死亡掉落事务](design/contract-v25-monster-carried-items.md)
- [Contract v26：楼层生命周期与确定性程序化楼层](design/contract-v26-floor-lifecycle.md)
- [Contract v27：程序化房间怪物与地面掉落分配](design/contract-v27-procedural-room-content.md)
- [Contract v28：门地形状态与方向性交互](design/contract-v28-door-terrain-state.md)
- [Contract v29：锁门、开锁检定与破门](design/contract-v29-locked-door-checks.md)
- [Contract v30：权威相邻地形交互查询](design/contract-v30-authoritative-terrain-interactions.md)
- [Contract v31：秘密门、搜索与地形知识](design/contract-v31-secret-door-search.md)
- [Contract v32：隐藏陷阱、触发与解除](design/contract-v32-hidden-traps-disarm.md)
- [Contract v33：挖掘与可破坏地形](design/contract-v33-diggable-terrain.md)
- [Contract v34：多深度楼层连接](design/contract-v34-multi-depth-floors.md)
- [Contract v35：地牢探索实例生命周期](design/contract-v35-dungeon-expedition-lifecycle.md)
- [Contract v36：一次性任务层](design/contract-v36-one-shot-task-floor.md)
- [Contract v37：任务目标与完成/失败](design/contract-v37-task-objective-resolution.md)
- [Contract v38：任务奖励与任务日志](design/contract-v38-task-reward-journal.md)
- [Contract v39：击杀目标与任务进度](design/contract-v39-kill-objective-progress.md)
- [Contract v40：任务放弃与退出限制](design/contract-v40-task-abandon-exit-policy.md)
- [Contract v41：数量击杀与持久进度](design/contract-v41-counted-kill-progress.md)
- [Contract v42：可重接任务](design/contract-v42-retakeable-task.md)
- [Contract v43：独立任务 ID 与共享任务范围](design/contract-v43-shared-task-id.md)
- [Contract v44：权威任务状态机与领域事件订阅](design/contract-v44-task-state-machine.md)
- [Contract v45：有序多阶段任务目标](design/contract-v45-ordered-task-stages.md)
- [Contract v46：多深度最终层与持久守护者](design/contract-v46-final-floor-guardian.md)
- [Contract v47：深度主题 Vault 与群体遭遇](design/contract-v47-themed-vault.md)
- [Contract v48：楼层生成表、加权 Vault 与巢穴](design/contract-v48-floor-generation-tables.md)
- [Contract v49：预算化十层压力地牢](design/contract-v49-budgeted-pressure-dungeon.md)
- [Contract v50：Vault 空间变换与确定性多模板落位](design/contract-v50-spatial-vault-placement.md)
- [Contract v51：动态 friends/escort 群体与 formation](design/contract-v51-dynamic-encounter-groups.md)
- [Contract v52：程序化特殊地形表与空间预算](design/contract-v52-terrain-feature-budgets.md)
- [Contract v53：分阶段洞穴地貌与房间几何预算](design/contract-v53-staged-cavern-layout.md)
- [Contract v54：湖泊与河流水文阶段](design/contract-v54-lake-river-hydrology.md)
- [Contract v55：迷宫、毁坏区与岩脉阶段](design/contract-v55-maze-destroyed-streamers.md)
- [Contract v56：原版式怪物 Pit 与等级阵列](design/contract-v56-classic-monster-pit.md)
- [Contract v57：Maze-only 专用楼层模式](design/contract-v57-maze-only-floor.md)
- [Contract v58：权威楼层连接与 shaft](design/contract-v58-floor-connections.md)
- [Contract v59：持久 pack identity 与首版 pack AI](design/contract-v59-pack-ai.md)
- [Contract v60：同层多区域主题](design/contract-v60-regional-themes.md)
- [Contract v61：暂停任务管理与确定性重接](design/contract-v61-retake-management.md)
- [Contract v62：区域组合生成](design/contract-v62-regional-composition.md)
- [Contract v63：树状地牢与共享守护者镜像](design/contract-v63-dungeon-tree-guardian-mirrors.md)
- [Contract v64：多入口 Vault 与连通拼接](design/contract-v64-multi-entry-vault-connectivity.md)
- [Contract v65：地牢实例身份与生命周期](design/contract-v65-dungeon-instance-identity.md)
- [Contract v66：动态楼梯目标与探索树](design/contract-v66-dynamic-exploration-tree.md)
- [前端目标模式 v1](design/frontend-targeting-v1.md)
- [RFB 全系统梳理与重构实现路线](design/rfb-system-implementation-roadmap.md)
- [待实现内容清单](design/pending-implementation.md)
- [核心协议 v1](design/protocol-v1.md)
- [确定性模拟、随机数与回放](design/deterministic-simulation.md)
- [内容数据格式 v1](design/content-format-v1.md)
- [Tileset manifest 与资源回退 v1](design/tileset-format-v1.md)
- [新存档格式 v1](design/save-format-v1.md)
- [桌面原生存档与诊断 v1](design/desktop-native-storage-v1.md)
- [桌面崩溃诊断闭环 v1](design/crash-diagnostics-v1.md)
- [授权、版权与素材迁移审计](design/licensing-and-assets.md)
- [本地化与中文文本重构计划](design/localization-rewrite-plan.md)
- [Fluent 本地化运行时 v1](design/fluent-localization-v1.md)
- [桌面分层 RendererBackend v1](design/renderer-backend-v1.md)
- [Rust 权威可见性与光照 v1](design/visibility-lighting-v1.md)
- [静态地形 Chunk 渲染 v1](design/terrain-chunk-rendering-v1.md)

当前原创规则契约位于 [`tests/fixtures/contract-v66/scenarios`](tests/fixtures/contract-v66/scenarios)，由 `rfb-contract` 在所有平台运行；`contract-v1` 至 `contract-v65` 作为历史基准保留。

确定性命令回放由 [`rfb-replay`](crates/rfb-replay) 提供：正式 `.rfbreplay` 使用带 SHA-256 校验的 MessagePack 容器，JSON 仅用于调试。

## 原项目

旧版 RFB 源码和当前可玩版本继续保留在：

[UncleFvcker/RoguelikeFansBand-zh-CN](https://github.com/UncleFvcker/RoguelikeFansBand-zh-CN)

旧项目在重构期间只作为规则行为、平台表现和旧存档格式的本地参考实现。

旧版内容不会复制进本仓库或新游戏发行包。开发工具通过本地环境变量 `RFB_LEGACY_SOURCE` 只读访问旧仓库，并固定读取 `v1.3.0.7`；默认开发路径见 [`.env.example`](.env.example)。新游戏内容、文本和素材均单独创作。

## 许可证

- 原创 Rust/TypeScript 代码、工具、测试和 Schema：`MPL-2.0`；
- 原创文档、游戏数据和美术素材：`CC BY-SA 4.0`；
- 第三方内容：保留各自许可证；
- 旧 RFB/FrogComposband/Angband 内容不在本仓库中，也不由上述许可证重新授权。

完整适用范围见 [LICENSES/README.md](LICENSES/README.md) 和 [NOTICE](NOTICE)。

## 当前阶段

协议 1.49 / contract-v49 已建立楼层级 `actorSlots/lootPlacements` 总预算，并新增独立十层共鸣压力地牢：actor 上限由 2 增长至 10，loot placement 由 1 增长至 3，深度 4 切换第二主题 terrain，深度 10 生成 9 个普通遭遇和 1 个持久守护者。active baseline 共 99 个 exact fixtures，内容包为 1.42.0、terrain 37、actor 8、encounter table 2、loot table 5、theme table 2、vault 2；save v1 / state-hash Schema v19 不变。完整边界见 [Contract v49 说明](design/contract-v49-budgeted-pressure-dungeon.md)。

协议 1.50 / contract-v50 已建立 Vault 八向旋转/镜像、边界入口、自由 wall 区落位、同层多 Vault 数量/面积预算、重叠拒绝和确定性失败回退。共鸣压力地牢深度 8 会跳过无法落位的 12×12 高权重模板，并在 9 actor/3 loot 总预算内放置两个小型 Vault。active baseline 共 100 个 exact fixtures，内容包为 1.43.0、terrain 37、actor 8、encounter table 2、loot table 5、theme table 2、vault 5；save v1 / state-hash Schema v19 不变。完整边界见 [Contract v50 说明](design/contract-v50-spatial-vault-placement.md)。

协议 1.51 / contract-v51 已建立 encounter 动态 friends/escort、`cluster/ring` formation、群体数量/随从 actor 预算、空间压力缩减和原子回退。共鸣压力地牢深度 6/7 分别生成 ring 与 cluster 群体，并在 7/8 actor 总预算内由普通遭遇填满剩余槽位。active baseline 共 102 个 exact fixtures，内容包为 1.44.0、terrain 37、actor 10、encounter table 3、loot table 5、theme table 2、vault 5；save v1 / state-hash Schema v19 不变。完整边界见 [Contract v51 说明](design/contract-v51-dynamic-encounter-groups.md)。

协议 1.52 / contract-v52 已建立独立 terrain feature 表、room/corridor 放置语义、深度加权选择、额外特殊地形预算、占位排斥与空间失败回退。共鸣压力地牢深度 3–10 会在固定拓扑门/陷阱之外放置 2–4 个 trap、rubble、locked/secret door。active baseline 共 104 个 exact fixtures，内容包为 1.45.0、terrain 37、actor 10、encounter table 3、loot table 5、theme table 2、terrain feature table 1、vault 5；save v1 / state-hash Schema v19 不变。完整边界见 [Contract v52 说明](design/contract-v52-terrain-feature-budgets.md)。

协议 1.55 / contract-v55 已沿原版 `build_maze_vault()`、`destroy_level()` 与 `build_streamer()` 增加内容驱动的完美迷宫、多震中毁坏区和加权岩脉阶段。深度 9 生成 15×15、127 通路格的 maze 与 24 格 streamer；深度 10 生成 48 格 destroyed 区与 24 格 streamer，房间/隧道仍保证主链连通。active baseline 共 110 个 exact fixtures，内容包为 1.48.0、terrain 42、actor 10、encounter table 3、loot table 5、theme table 2、terrain feature table 1、vault 5；save v1 / state-hash Schema v19 不变。完整边界见 [Contract v55 说明](design/contract-v55-maze-destroyed-streamers.md)。

协议 1.56 / contract-v56 已参考原版 `Monster Pit I` 与 `_init_formation()` 增加独立复合 pit 房间、单入口内室、专属加权怪物池和中心强化的等级阵列。深度 9 生成 11×11 外墙/环廊/内墙结构并以 25 个 actor 填满 5×5 内室；普通 encounter、loot 和 terrain feature 排除整个 pit footprint。active baseline 共 112 个 exact fixtures，内容包为 1.49.0、terrain 42、actor 10、encounter table 4、loot table 5、theme table 2、terrain feature table 1、vault 5；save v1 / state-hash Schema v19 不变。完整边界见 [Contract v56 说明](design/contract-v56-classic-monster-pit.md)。

协议 1.57 / contract-v57 已参考原版 `DF1_MAZE` 独立生成分支建立 `maze-only` 专用楼层模式。深度 9 现在跳过普通房间与走廊，只保留 127 格连通 maze、远距上下楼锚点、路径陷阱、streamer 和区域化 encounter/loot；v56 pit 移到深度 10 并继续与最终守护者和晚期地貌共存。active baseline 共 114 个 exact fixtures，内容包为 1.50.0、content hash 为 `d209d68a6a39af21eee8d1a951684be86e847ab570823c9c2604fa199e4571e1`；save v1 / state-hash Schema v19 不变。完整边界见 [Contract v57 说明](design/contract-v57-maze-only-floor.md)。

协议 1.58 / contract-v58 已建立稳定连接 ID、同层多座普通楼梯、独立到达点和跨两层 shaft。主 up/down 保留旧锚点，附加连接在 Vault 之后使用种子 RNG 从合法格随机落位；当前层与离层存档保存连接 ID→位置，v57 无连接列表的旧楼层继续走 legacy 标签回退。active baseline 共 117 个 exact fixtures，内容包为 1.51.0、content hash 为 `ee07c276bbe568fafc1e1d6942e9d57d158bd250ed452b32c01c774d8521e96d`；save 容器仍为 v1，state-hash 升至 Schema v20。完整边界见 [Contract v58 说明](design/contract-v58-floor-connections.md)。

协议 1.60 / contract-v60 已增加独立 `regionTables`、楼层 `regionTableId/regionPlacements`、权重无放回区域选择、按房间中心归属的局部 terrain，以及区域限定的 encounter/loot。区域 ID、主题、局部表引用和完整格集合随当前层与离层持久化；v59 旧存档缺失区域时不补生成、不推进 RNG。active baseline 共 119 个 exact fixtures，内容包为 1.53.0、content hash 为 `9789fcbbd8431ed745d8a0305cc81a54cc7e45ce79be86ed76e0227d66564a02`；save 容器仍为 v1，state-hash 升至 Schema v22。完整边界见 [Contract v60 说明](design/contract-v60-regional-themes.md)。

协议 1.61 / contract-v61 已为可重接任务增加 `maxRetakes` 与 `preserve-floor/regenerate-floor` 策略，成功恢复次数进入权威任务状态；地表任务日志可按 `taskId` 永久放弃 paused 任务。重建会保留阶段与进度，只生成剩余计数目标；次数耗尽的入口拒绝不改变 RNG。active baseline 共 121 个 exact fixtures，内容包为 1.54.0、content hash 为 `56fc449617a4c05c12ff11716c14b4f5c680cada9ad86c6ece736b52fa904bc2`；save 容器仍为 v1，state-hash 升至 Schema v23。完整边界见 [Contract v61 说明](design/contract-v61-retake-management.md)。

协议 1.62 / contract-v62 已解除区域楼层与全层 theme/Vault、dynamic formation、terrain feature、pit、guardian、显式连接和 cavern/lake/river/destroyed/streamer 的阶段隔离。特殊 footprint 归入单一宿主区域，普通 actor/loot 按实际可行走容量分配，区域怪物寻路保持在持久边界内；demo 在 echo depth 2 和 resonance depth 6/7/8/10 覆盖各组合。active baseline 共 125 个 exact fixtures，内容包为 1.55.0、content hash 为 `9d25687c1296bc6f9953024bd76bb9eefc4c1e3955280b96d34d565ff7ca289d`；save v1 / state-hash Schema v23 不变。完整边界见 [Contract v62 说明](design/contract-v62-regional-composition.md)。

协议 1.63 / contract-v63 已增加独立 dungeon 定义、单根楼层树、唯一父边、多个程序化最终叶层和共享守护者镜像。回声地牢的普通楼梯与 shaft 现在进入不同子层；击败任一镜像只结算一次征服，并确定性移除其他已生成镜像。active baseline 共 127 个 exact fixtures，内容包为 1.56.0、content hash 为 `246f51864965fac494c7a39959f591caa0434d9fa4eac839501f9d09526eb617`；save v1 / state-hash Schema v23 不变。完整边界见 [Contract v63 说明](design/contract-v63-dungeon-tree-guardian-mirrors.md)。

协议 1.64 / contract-v64 已把 Vault 入口升级为 1–8 个唯一边界位置，并在加载时证明模板内部可通行格连通；落位时每个入口使用固定方向、最多 12 格的 BFS connector 接入既有走廊，只有整层连通证明通过才原子提交。demo 新增 8×8 四入口 Crossroads，与不可落位 Monolith 一同覆盖加权选择和稳定回退。active baseline 共 129 个 exact fixtures，内容包为 1.57.0、content hash 为 `9f3e3d5dee1e8777179179259380990b9253aa7f195f08cd29cbbd58562793df`；save v1 / state-hash Schema v23 不变。完整边界见 [Contract v64 说明](design/contract-v64-multi-entry-vault-connectivity.md)。

协议 1.65 / contract-v65 已增加显式 dungeon instance identity。每座地牢按稳定序号分配 <dungeonId>.instance.N，当前层、离层 floor 与存档都携带实例 ID；仓库键使用实例+floor，返回地表只清理当前实例，不再误删其他 dungeon 或任务楼层。v64 存档缺失字段时确定性迁移为首实例，不重建地图或推进 RNG。active baseline 共 131 个 exact fixtures，内容包仍为 1.57.0、content hash 为 9f3e3d5dee1e8777179179259380990b9253aa7f195f08cd29cbbd58562793df；save v1 / state-hash 升至 Schema v24。完整边界见 [Contract v65 说明](design/contract-v65-dungeon-instance-identity.md)。

协议 1.66 / contract-v66 已增加动态楼梯目标与实例级探索树解析。连接可声明多个加权候选，同层按稳定连接 ID 无放回选择不同目标 floor；解析后的 target floor/connection 随楼层存档，目标 arrival connection 在首次到达时原子修正，v65 旧存档缺字段时固定目标回退且不推进 RNG。普通 dungeon 回到地表仍立即清空，下一次进入重新生成。active baseline 共 132 个 exact fixtures，内容包为 1.58.0、content hash 为 `834acbe3d025810eb1399db74689d35a4d3dae34862bcbf1271c8d20ad11d9fc`；save v1 / state-hash 升至 Schema v25。完整边界见 [Contract v66 说明](design/contract-v66-dynamic-exploration-tree.md)。

阶段 E 的楼层生命周期、房间内容分配、门、秘密地形、陷阱、挖掘、三层/十层地牢、动态树状分支、多个最终层、共享持久守护者、楼层生成表、actor/loot 总预算、深度与同层多区域主题、区域特殊阶段组合、Vault 多入口/空间落位/跨走廊拼接、巢穴、动态 friends/escort formation、持久 pack AI、程序化地貌、原版式 pit、maze-only、多楼梯、独立到达点、shaft 与实例级探索生命周期已经建立；普通 dungeon 返回地表即清空，下一次进入重新生成。任务线也已补齐暂停任务的地表放弃、重接上限与确定性重建。下一步推进多 dungeon 进入条件、胜利/退休评分和可配置实例生命周期。

Tauri 2 Windows 原生垂直切片已经建立：`TauriNativeTransport` 直接调用 Rust 核心，移动、等待、怪物追踪、基础战斗、地面物品拾取、背包多选、鉴别、装备/卸下、整堆批量丢弃和部分数量丢弃均已接入；攻击、防御和最大生命由 Rust 权威派生，回声护符基础提供攻击 +1、防御 +1、最大生命 +4，完整识别后其谐振锋芒再提供攻击 +1。拆分物品使用持久化 `generated.item.N` 实例 ID。三套键位预设、Fluent 中英双语热切换、五层 PixiJS RendererBackend、Rust 权威 FOV/探索记忆/内容标签光源、桌面命名存档槽、`.rfbsave` 手动导入导出和 `.rfbreplay` 诊断回放均已接入。PixiJS 地形层根据 192×64 原创压力场景实测使用默认 16×16 RenderTexture chunk；`pixi-layered-chunks-v3` 后端保留整图语义数据，但玩家居中模式只为可见 chunk 挂载并复用 object/actor/visibility/lighting 动态视图。16 格 profile 的动态对象从整图理论值 86,016 降到 7,168，初始化约从 133 ms 降到 30 ms；整图滚动模式仍会按需挂载全部 chunk。动态规则 dirty cells、静态缓存和视图复用相互独立。原生存档使用应用私有目录、原子替换和三份备份，并提供结构化错误与本地日志。Rust panic、未正常退出和前端未处理异常已接入自动本地 `.rfbdiagnostic` 闭环，最多轮换保留 5 份且不自动上传。简体中文为默认语言；相机、缩放和本地化属于前端显示状态，不影响权威 state hash。旧 `rfb-wasm`、Web Worker、wasm-pack 和 wasm32 构建目标已经从 workspace、前端和 CI 删除。

### 本地验证

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --exclude rfb-tauri --all-targets -- -D warnings
cargo test -p rfb-contract
cargo run -p rfb-protocol --features bindings --bin generate-bindings -- --check
cargo run -p rfb-content --features schemas --bin generate-content-schemas -- --check
cargo run -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original

cd web
npm ci
npm test
npm run build -- --no-bundle
# 启动可玩开发版：npm run dev
```

Rust 是 CoreTransport DTO 的唯一权威来源。修改 `rfb-protocol` 后运行：

```powershell
cargo run -p rfb-protocol --features bindings --bin generate-bindings
```

该命令更新 `web/src/protocol.ts` 和 `schemas/protocol-v1.schema.json`；CI 使用 `--check` 拒绝未同步的生成文件。

验证或编译原创内容包：

```powershell
cargo run -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original
cargo run -p rfb-content --bin rfb-contentc -- compile packs/rfb-demo-original target/generated/rfb-demo-original.rfbcontent
cargo run -p rfb-content --features schemas --bin generate-content-schemas
```

内容编译器会严格解析 JSON、校验稳定 ID/引用/范围，规范化排序后输出带 SHA-256 校验的 MessagePack 容器。修改内容时可先运行 `rfb-contentc inspect-source packs/rfb-demo-original` 查看新 hash，再显式更新 lock；首个原创包的固定 content hash 记录在 `packs/rfb-demo-original/content.lock.json`。

如需生成本地旧版参考 manifest：

```powershell
$env:RFB_LEGACY_SOURCE="D:/codex/Frogcomposband/master"
$env:RFB_LEGACY_REF="v1.3.0.7"
$env:RFB_LEGACY_COMMIT="191f48c3fd1cdbc81a3d3395a88cd6758402b4d9"
cargo run -p rfb-legacy-probe
```

登记本地旧存档样本时显式传入至少 3 个旧仓库内的文件路径：

```powershell
cargo run -p rfb-legacy-probe -- catalog-saves <旧存档1> <旧存档2> <旧存档3>
```

工具只把中性命名副本、SHA-256、四字节版本头和本地清单写入被 Git 忽略的 `.local/legacy-baseline/`。当前机器已经准备两份 1.3.0.7 样本和一份 1.2.0.6 迁移样本。

解析旧存档的稳定前缀并建立本地字段断言：

```powershell
cargo run -p rfb-legacy-import -- inspect-prefix .local/legacy-baseline/saves/legacy-save-01.bin
cargo run -p rfb-legacy-import -- record-catalog .local/legacy-baseline/save-samples.json
cargo run -p rfb-legacy-import -- verify-catalog .local/legacy-baseline/save-samples.json
```

`rfb-legacy-import` 当前只读取不依赖旧 C 结构体内存布局的 409 字节稳定前缀，包括版本、保存元数据、63 项 RNG 状态和选项位。生成的 `parsed-save-samples.json` 仍位于 `.local/`，不会进入 Git；`record-catalog` 拒绝覆盖已有基线。

快照规范化和 hash：

```powershell
cargo run -p rfb-contract -- normalize-snapshot <snapshot.json>
cargo run -p rfb-contract -- hash-snapshot <snapshot.json>
cargo run -p rfb-contract -- validate-policy tests/fixtures/contract-v66/baseline-policy.json
```

当前 132 个原创 contract fixtures、自动协议生成、原创内容包、ASCII glyph atlas、图片 tileset manifest、缺失资源回退和 Windows Tauri 端到端测试已经建立。桌面 E2E 可用以下命令运行：

```powershell
cd web
npm run e2e
```

测试覆盖 Rust 权威 FOV/光照增量、地图局部更新、terrain chunk 缓存/失效/视口剔除、Canvas/HTML 消息分层、镜头与缩放、地面物品拾取、背包多选、装备属性、卸下、部分/批量丢弃、原生存档槽的新建/载入/覆盖/删除、手动存档导出与恢复、回放导出、自动崩溃诊断和 tileset 热切换；失败时会在仓库根目录的 `test-results/` 生成截图和日志。

Tauri Android ARM64 Debug APK 构建链也已经建立，Windows 本地可运行：

```powershell
.\scripts\build-android.ps1 -Proxy http://127.0.0.1:7897
```

Android 与 Windows 使用同一个 Rust 核心和 Tauri Commands。详细依赖、产物位置和当前尚未完成的真机验证见 [Tauri Android 原生目标](design/android-target.md)。
