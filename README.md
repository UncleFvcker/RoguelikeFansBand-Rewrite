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
- [Contract v67：地牢入口守卫与可选进入条件](design/contract-v67-dungeon-entrance-guardians.md)
- [Contract v68：胜利、退休与角色评分](design/contract-v68-victory-retirement-scoring.md)
- [Contract v69：可配置地牢实例生命周期](design/contract-v69-configurable-instance-lifecycle.md)
- [Contract v70：RFB 角色成长基础](design/contract-v70-rfb-character-progression.md)
- [Contract v71：RFB 角色构筑、种族职业与技能集合](design/contract-v71-rfb-character-builds.md)
- [Contract v72：可观察技能检定](design/contract-v72-observable-skill-checks.md)
- [Contract v73：法术资源与能力书基础](design/contract-v73-ability-books.md)
- [Contract v74：法术资源恢复与自身治疗](design/contract-v74-resource-recovery-and-healing.md)
- [Contract v75：能力熟练度与冷却](design/contract-v75-ability-proficiency-and-cooldowns.md)
- [Contract v76：学习容量与主动遗忘](design/contract-v76-learning-capacity-and-forgetting.md)
- [Contract v77：RFB 式范围爆发伤害](design/contract-v77-area-damage.md)
- [Contract v78：RFB 式方向射线伤害](design/contract-v78-beam-damage.md)
- [Contract v79：RFB 式锥形能力伤害](design/contract-v79-cone-damage.md)
- [Contract v80：RFB 式定点延长射线](design/contract-v80-targeted-beam-extension.md)
- [Contract v81：首个短距位移能力](design/contract-v81-teleport-ability.md)
- [Contract v82：首个召唤能力](design/contract-v82-summon-ability.md)
- [Contract v83：首个侦测能力](design/contract-v83-detection-ability.md)
- [Contract v84：首个地形改变能力](design/contract-v84-terrain-transform-ability.md)
- [Contract v85：状态能力与有序多效果](design/contract-v85-ordered-status-effects.md)
- [Contract v86：首个怪物施法与能力选择 AI](design/contract-v86-monster-casting-ai.md)
- [Contract v87：怪物施法效用与目标扩展](design/contract-v87-monster-casting-utility.md)
- [Contract v88：怪物目标、战术移动与施法记忆](design/contract-v88-monster-targets-tactics-memory.md)
- [Contract v89：友方召唤物行动与首版命令](design/contract-v89-friendly-summon-commands.md)
- [Contract v90：多职业资源底子与首个技法资源](design/contract-v90-technique-resources.md)
- [Contract v91：怪物位移法术族](design/contract-v91-monster-displacement.md)
- [Contract v92：新状态族（混乱/致盲/麻痹）](design/contract-v92-status-family.md)
- [Contract v93：怪物直伤弹族（bolt/ball）与伤害平坦加值](design/contract-v93-monster-bolt-ball.md)
- [Contract v94：怪物吐息族（breath）与 HP 比例伤害](design/contract-v94-breath-family.md)
- [旧版内容导入管线 v1](design/legacy-content-import-v1.md)
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

当前原创规则契约位于 [`tests/fixtures/contract-v94/scenarios`](tests/fixtures/contract-v94/scenarios)，由 `rfb-contract` 在所有平台运行；`contract-v1` 至 `contract-v93` 作为历史基准保留。

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

协议 1.67 / contract-v67 已增加原版式 dungeon 入口守卫和原创内容可选硬进入条件。入口守卫使用 `GuardPosition` 固守入口附近、仍可相邻攻击，但不会阻止楼梯交互，玩家可以绕过直接进入；击败状态随 dungeon 持久化。任务状态、前置 dungeon 征服和携带物条件在实例序号与 RNG 消耗前原子检查，demo 原版 dungeon 默认不配置这些硬条件。普通 dungeon 回到地表仍立即清空，下一次进入重新生成。完整边界见 [Contract v67 说明](design/contract-v67-dungeon-entrance-guardians.md)。

协议 1.68 / contract-v68 已建立 campaign 胜利、退休结算与内容驱动角色评分。Resonance 是 demo 唯一 campaign victory dungeon；Echo 守护者可被征服但不会提前结束战役。击败所有 victory dungeon 的最终守护者后发布 `CampaignVictorious`，玩家回到地表后可执行 `Retire`，结算后状态冻结且拒绝继续命令。评分为征服地牢、完成任务和胜利奖励之和，再扣除按回合间隔计算的惩罚，最低为 0。内容包升至 1.60.0，content hash 为 `1614fadbf4cd1d3ee03fc011eac069de3a1b8c23ec65b6f09e210f20008dbc4c`，active baseline 共 137 个 exact fixtures，save v1 / state-hash 升至 Schema v27。完整边界见 [Contract v68 说明](design/contract-v68-victory-retirement-scoring.md)。

协议 1.69 / contract-v69 已建立内容驱动的 dungeon `instanceLifecycle`：默认 `reset-on-surface`、`persistent` 和带惰性淘汰的 `turn-ttl`。新增 Archive Depths 作为 3 回合 TTL 示例；返回地表可保存一个 retained instance，下次进入续接同一实例，过期后确定性分配下一个实例序号并清理已淘汰实例的物品属性知识。协议 DTO 增加可选 `retainedInstanceId`/`retainedAtTurn`，state hash 升至 Schema v28；v68 及更早存档缺失字段时按默认清理迁移。内容包升至 1.61.0，content hash 为 `06c054a8c083e05b9d0396aa1076fbe2133a6a1ce5f6c32f101e5d1dabd14b70`，active baseline 共 140 个 exact fixtures，零 waiver。普通 Echo/Resonance 仍返回地表即清空。完整边界见 [Contract v69 说明](design/contract-v69-configurable-instance-lifecycle.md)。

协议 1.70 / contract-v70 已建立 RFB 角色成长基础：击杀经验沿用 1–50 级阈值，未征服最终地牢时等级封顶 50 且超过阈值的经验保留；胜利后自动释放封顶经验并解锁等级 100 与 `18/820` 属性桶。玩家保存独立的六维自然属性、出生时确定性生成的 100 级 HP 序列、待分配属性点和装备有效属性修正；新增 `IncreaseAttribute` 命令与自然/有效属性 DTO，旧存档缺少 progress 时按固定迁移规则恢复。state hash 升至 Schema v29；内容包为 1.62.0，content hash 为 `ad6b35c6e0ae8980a74fac51ea1e6597b09559541d4a85d598284dc2cb41d7e6`，active baseline 共 148 个 exact fixtures，零 waiver。普通 Echo/Resonance 仍返回地表即清空，Archive 继续覆盖 retained/TTL。完整边界见 [Contract v70 说明](design/contract-v70-rfb-character-progression.md)。

协议 1.71 / contract-v71 在 v70 之上建立 RFB 角色构筑基础：内容包新增独立 `skills`、`skillSets`、`races`、`classes`、`personalities`、`builds` 根；`PlayerDto`/save 暴露构筑身份与技能聚合，Race/Class/Personality 的属性、生命/经验倍率和出生装备进入现有派生管线。demo 内容包升至 1.63.0，content hash 为 `1c94890a0f39d42a4b496a7222b8c9d191f24fe94b3c9d47d4a1eeea5364c5b4`；state hash 升至 Schema v30，active baseline 共 152 个 exact fixtures，零 waiver。v70 缺少构筑/技能字段的存档按默认 Explorer 迁移，不推进 RNG；四个代表性构筑 fixture 覆盖出生身份、技能、装备和 save round-trip。完整边界见 [Contract v71 说明](design/contract-v71-rfb-character-builds.md)。

协议 1.72 / contract-v72 已把 `device`、`saving-throw`、`stealth` 和 `perception` 接入权威检定与结构化事件。装置失败不消费物品，陷阱豁免可抵消伤害，成功移动会被动发现邻近隐藏 terrain，未警戒怪物按范围与视线检定玩家潜行；actor 警戒状态进入快照、存档和 state hash Schema v31。内容包升至 1.64.0，新增 Resonance Stabilizer、Resonance Ward、隐藏 Echo Rune 和 Echo Listener，content hash 为 `3188f4cf0937f44292980e8ca8fffc1db9c310e961af4502bd9380124e53d54a`；active baseline 共 160 个 exact fixtures、零 waiver。八个新 fixture 以相同 seed 对照 Tinkerer/Vanguard 的成功失败，并覆盖 `alerted` round-trip。完整边界见 [Contract v72 说明](design/contract-v72-observable-skill-checks.md)。

协议 1.73 / contract-v73 已建立首个能力书施法闭环：内容包新增独立 resource、ability 与 ability-book 根，Mage 以 Intelligence 计算 Mana 上限和失败率，Scholar 出生携带 Echo Primer，可学习并施放 Resonant Bolt。资源在失败检定前扣除，失败仍耗 Mana；资源不足、未学习或缺少书本会结构化拒绝且不推进施法 RNG。资源池、已学能力、施法 outcome、存档迁移和 Web 能力面板进入 state hash Schema v32。内容包升至 1.65.0，content hash 为 `fa88458239f225a5033e5910c64ba30f8e1e4095fc82b1ebce6a5c914e05ad2d`；该历史基准共 166 个 exact fixtures、零 waiver。完整边界见 [Contract v73 说明](design/contract-v73-ability-books.md)。

协议 1.74 / contract-v74 已补齐首轮资源恢复与非伤害能力：Mana 在等待后恢复 1，`Rest { turns }` 每个实际休息回合恢复 3 并真实推进调度器；满资源、可见敌人、受伤与死亡都有结构化停止原因。协议新增稳定 `self` 目标、`heal` 能力效果、资源恢复与休息 outcome；Stillwater Notes 让 Scholar 学习 Mending Echo，以 4 Mana 治疗自身 6 点生命。Web 显示恢复速率、提供休息按钮，并直接提交自身目标。完整历史边界见 [Contract v74 说明](design/contract-v74-resource-recovery-and-healing.md)。

协议 1.75 / contract-v75 在此基础上加入参考 RFB 原版的五档能力熟练度、Mana 成本曲线、Expert/Master 失败率修正、成功/失败统计、可选每能力/共享组冷却和存档迁移。能力进度进入 state hash Schema v34；普通能力默认无冷却，冷却拒绝不扣资源且不抽 RNG。内容包升至 1.67.0，content hash 为 `bcc23bf5834c37bf7fb0874bcb1dfc72c751efad36f76d94b07391100e976316`，active baseline 共 182 个 exact fixtures、零 waiver。完整边界见 [Contract v75 说明](design/contract-v75-ability-proficiency-and-cooldowns.md)。

协议 1.76 / contract-v76 在 v75 之上加入独立学习容量、容量投影、主动 `ForgetAbility`、遗忘/重新学习事件和容量满零 RNG 拒绝。能力进度不因遗忘清除，重新学习恢复熟练度、统计与冷却；demo 内容包升至 1.68.0，新增 Harmonic Spark，content hash 为 `c16f6cf31b726461910fb09bc775b5b6d79af889fe0de046043f085e9593ad04`，active baseline 共 186 个 exact fixtures、零 waiver。state hash 仍为 Schema v34，save 容器仍为 v1。完整边界见 [Contract v76 说明](design/contract-v76-learning-capacity-and-forgetting.md)。

协议 1.77 / contract-v77 在 v76 之上加入 RFB 式范围爆发伤害：定点目标穿过中途怪物、方向目标在首个怪物处停止，墙体阻断传播，爆发按原版整数距离由内向外稳定结算，并对每个 actor 复用既有抗性/击杀/掉落管线。无效目标在 Mana、施法 RNG 和熟练度前拒绝，空爆仍正常消耗资源并只投一次基础伤害骰；demo 内容包升至 1.69.0，新增 Echo Burst，content hash 为 `acecaf504ebc3affaf67fbd8400016d85a8f4fd6b70fb7de3f1626887e5c6d62`，active baseline 共 190 个 exact fixtures、零 waiver。state hash 仍为 Schema v34，save 容器仍为 v1。完整边界见 [Contract v77 说明](design/contract-v77-area-damage.md)。

协议 1.78 / contract-v78 在 v77 之上加入 RFB `fire_beam()` 式方向射线：射线穿过 actor、被墙体/边界截断，按近到远稳定顺序复用既有抗性、击杀、经验、掉落和任务管线，并且每次射线只投一次基础伤害骰。方向以外的目标模式在资源/RNG 前拒绝，空射仍消耗资源并投一次伤害骰；demo 内容包升至 1.70.0，新增 Echo Lance，content hash 为 `6f5f545e3b2c9cab98b6cd33f328679228b643ae147f20739c982863eba47bea`，active baseline 共 194 个 exact fixtures、零 waiver。state hash 仍为 Schema v34，save 容器仍为 v1。完整边界见 [Contract v78 说明](design/contract-v78-beam-damage.md)。

协议 1.79 / contract-v79 在 v78 之上加入 RFB 式固定八向锥形能力：中心线从相邻格开始逐层展开到配置半径，actor 不阻挡，墙体/边界截断，候选格按近到远、横向距离和坐标稳定排序，侧向目标复用既有整数衰减并共享一次基础伤害骰。无效目标模式在 Mana/RNG 前拒绝，空锥仍消耗资源并投一次伤害骰；demo 内容包升至 1.71.0，新增 Echo Fan，content hash 为 `817ccfc5924d6dd8d957fb1f2c97f191c08dd5c34aa1ff9dea265716d3236835`，active baseline 共 198 个 exact fixtures、零 waiver。state hash 仍为 Schema v34，save 容器仍为 v1。完整边界见 [Contract v79 说明](design/contract-v79-cone-damage.md)。

协议 1.80 / contract-v80 在 v79 之上补齐 RFB `project_hook()` 的定点延长射线：Echo Lance 现在接受方向、格子和实体目标；定点/实体目标在可见且不超距时沿稳定整数斜率穿过目标继续推进到最大射程，actor 不阻挡，墙体/边界截断，所有命中共享一次基础伤害骰。自身、缺失、不可见和超距目标在 Mana/RNG 前拒绝，不新增存档字段；demo 内容包升至 1.72.0，content hash 为 `30c38e57bd9a9d22694e02da9c2b5f07b76af0a4009deb59bbbc605703f5a504`，active baseline 共 202 个 exact fixtures、零 waiver。state hash 仍为 Schema v34，save 容器仍为 v1。完整边界见 [Contract v80 说明](design/contract-v80-targeted-beam-extension.md)。

协议 1.81 / contract-v81 在 v80 之上加入首个短距位移能力 Echo Step：teleport 效果只接受 position 目标，落点必须非当前格、在地图内、可见、满足 line of effect、可行走且无存活 actor 占据。所有落点拒绝在 Mana、施法 RNG 和熟练度前返回；成功后精确移动并复用普通移动的被动感知、陷阱触发和死亡处理，不增加误传送骰或存档字段；demo 内容包升至 1.73.0，content hash 为 `66e60826777d1bf79efb3eef6d718bcf3ed101e30c43d562fd122ff402eda95d`，active baseline 共 209 个 exact fixtures、零 waiver。state hash 仍为 Schema v34，save 容器仍为 v1。完整边界见 [Contract v81 说明](design/contract-v81-teleport-ability.md)。

协议 1.82 / contract-v82 在 v81 之上加入首个内容驱动召唤能力 Echo Companion：`summon` 声明友方 actor、数量、半径和生命周期；核心按距离/坐标稳定选择空地，生成带所有者与源能力的稳定实例 ID。空间不足在 Mana、施法 RNG 和熟练度前原子拒绝，失败率失败仍消耗 Mana但不生成 actor；召唤物不进入敌对 AI 或可见敌人判断，并按玩家回合递减后发出到期移除事件。demo 内容包升至 1.74.0，content hash 为 `aab3548090030a1d2d46496581fb41a9f2892213186aeb2236a7a79065fc069f`，active baseline 共 213 个 exact fixtures、零 waiver。save 容器仍为 v1，state hash 升至 Schema v35。完整边界见 [Contract v82 说明](design/contract-v82-summon-ability.md)。

协议 1.83 / contract-v83 在 v82 之上加入首个内容驱动侦测能力：Echo Pulse 返回 `perception-cue` 的瞬时位置，Echo Sight 把 `hidden` terrain 写入持久 `revealedTerrain`。侦测只扫描当前地图、半径和玩家 FOV 内尚未发现且具有隐藏投影的 terrain；结果按距离、`y`、`x` 稳定排序，墙后或视野外真值不会泄漏。空结果仍按正常施法支付 Mana、抽失败率并增加熟练度，非法目标和资源不足则在 RNG 前拒绝。demo 内容包升至 1.75.0，content hash 为 `8ac0aee6fe54abb2c97bbed3eedaaa510d32393126bd08f89d046d515a66213b`，active baseline 共 221 个 exact fixtures、零 waiver。save 容器仍为 v1，state hash 升至 Schema v36。完整边界见 [Contract v83 说明](design/contract-v83-detection-ability.md)。

协议 1.84 / contract-v84 在 v83 之上加入内容驱动 `transform-terrain`：Echo Delving 参考原版 `GF_KILL_WALL` 把合法岩壁/瓦砾转为地面，Echo Rampart 参考 `GF_MAKE_WALL` 把未占用地面转为阻挡瓦砾。目标中心必须在射程、FOV 和 line of effect 内；候选按距离、`y`、`x` 稳定排序，并跳过玩家、存活 actor、地面物品、地图边界、floor connection 及楼梯/shaft/入口标签。候选在资源与失败率前完整收集，成功时一次提交；空结果仍正常施法，非法/超距目标和资源不足保持零 RNG，失败支付 Mana但不改地形。修改直接进入 `changedCells`、楼层存档和既有 terrain state hash，不做自动连通修复。demo 内容包升至 1.76.0，content hash 为 `6e3906fff5447c3b83630e85e6c789a0dc151d9e16e1faa484ed10dda41a3ee4`；该历史 baseline 共 231 个 exact fixtures、零 waiver，save v1 与 state hash Schema v36 保持不变。完整边界见 [Contract v84 说明](design/contract-v84-terrain-transform-ability.md)。

协议 1.85 / contract-v85 在 v84 之上加入状态能力与有序多效果：旧单一 `effect` 内容保持兼容，新 `sequence` 允许 2–8 个同目标 actor 效果。Echo Quickening 依次添加 haste 并移除 slow；Echo Binding 先造成 cold damage，目标存活时再添加受 cold 抗性确定性缩时的 slow。效果按数组顺序结算，部分无效不回滚，免疫返回零持续时间；前序击杀后续效果标记 `target-dead`，空投影标记 `no-target`，二者都不抽取被跳过的伤害骰。协议通过 `AbilityDto.effects` 和 `ability.effects` 返回逐效果规格与结果。demo 内容包升至 1.77.0，content hash 为 `d056b65f8e2c61615e48badd8a6f02cd725007789535aa363448c8a0e8288bea`；该历史 baseline 共 242 个 exact fixtures、零 waiver，save v1 与 state hash Schema v36 保持不变。完整边界见 [Contract v85 说明](design/contract-v85-ordered-status-effects.md)。

协议 1.86 / contract-v86 开始阶段 H 的怪物施法纵切。Monster actor 可声明百分比施法频率和有序加权能力集合；已警戒怪物先抽频率骰，再过滤射程、墙体和 clean-shot 友军阻挡，频率通过时才抽权重并复用既有伤害、状态、抗性、死亡与有序效果管线。频率失败或无可用法术时继续近战/移动。施法后按 `ceil(100 / frequencyPercent)` 增加自身行动冷却，因此 50% 为 2 行动、25% 为 4 行动；冷却行动不抽施法 RNG。demo 新增 Echo Cantor，内容包升至 1.78.0，content hash 为 `be6b9b098c495ee3f2af6075ea5790d16eae7e8487c1fa310575c0dad8cba5bd`；该历史 baseline 共 249 个 exact fixtures、零 waiver。怪物冷却进入 save/replay，state hash 升至 Schema v37。完整边界见 [Contract v86 说明](design/contract-v86-monster-casting-ai.md)。

协议 1.87 / contract-v87 在同一选择层上加入纯效用调整和新目标执行器：健康或轻伤时剔除自疗，重伤按损失比例提高治疗权重，重复/免疫状态与无状态可移除以 `no-utility` 剔除，距离至少 3 格时提高对玩家施法权重。范围爆发、射线和锥形复用玩家侧几何并保守拒绝 footprint 内的次级实体；Call Discord 会生成由怪物施法者拥有、投影为 hostile、能够行动且可保存/回放的限时 Discordant Echo。协议返回每个候选的基础/有效权重、目标、footprint 与拒绝原因。内容包升至 1.79.0，content hash 为 `f9e9ccc93635da7f568a2cdd83f90024f86cd13d1d0ff43627f725dde4e3ecac`；active baseline 共 257 个 exact fixtures、零 waiver，save v1 / state hash Schema v37 不变。完整边界见 [Contract v87 说明](design/contract-v87-monster-casting-utility.md)。

协议 1.88 / contract-v88 把玩家阵营召唤物纳入怪物法术、追踪和近战目标，并按距离、玩家优先级与稳定 ID 选择目标。多格法术显式返回敌我计数，无友军风险时按敌方命中数加权，并用一次基础伤害骰结算所有玩家阵营目标。Echo Cantor 在距离小于 3 时尝试拉开、25% 生命时撤退；聪明施法者只在效果实际作用于玩家后记录抗性，后续按已观察知识降权或剔除免疫候选。内容包升至 1.80.0，content hash 为 `29116f924e1ef4ddf6b0aa43f3b1b1bd0b4d28245ac086bce30d7a008e8e9e8e`；active baseline 共 265 个 exact fixtures、零 waiver，save v1 / state hash 升至 Schema v38。完整边界见 [Contract v88 说明](design/contract-v88-monster-targets-tactics-memory.md)。

协议 1.89 / contract-v89 让玩家拥有的召唤物进入 actor 能量调度器，并提供 Follow、Attack、Keep Distance、Guard 四种零世界时间全局命令。召唤物复用自身近战和完整死亡事务，击杀经验、任务与掉落归属玩家；切层时仅 2 格内召唤物跟随并稳定落位，远处召唤物留在来源层。命令及 Guard 锚点进入 save/replay 和 state hash Schema v39；内容包保持 1.80.0 与同一 content hash。active baseline 共 272 个 exact fixtures、零 waiver。完整边界见 [Contract v89 说明](design/contract-v89-friendly-summon-commands.md)。

协议 1.90 / contract-v90 建立多职业资源底子：资源定义新增初始填充、近战命中/击杀获得与闲置衰减字段，职业可声明多条 techniqueProfiles（独立上限公式、主宰属性、最低失败率与先天能力）。首个技法资源“节奏”由决斗家纵切承载：近战命中 +2、击杀 +3、闲置回合 -1，等待/休息不恢复；弦月斩与涌动节奏为先天技法，复用既有 cast/熟练度/冷却管线，资源不足拒绝不抽 RNG。旧存档资源池放宽为子集匹配，缺失的池按初始填充恢复且零 RNG。内容包升至 1.81.0，content hash 为 `43da90740e88ba63d9839c992a90b0fcc9c008a379919e2bc624a208978e6252`；active baseline 共 282 个 exact fixtures、零 waiver，save v1 / state hash 升至 Schema v40。完整边界见 [Contract v90 说明](design/contract-v90-technique-resources.md)。

阶段 E 的楼层生命周期、房间内容分配、门、秘密地形、陷阱、挖掘、三层/十层地牢、动态树状分支、多个最终层、共享持久守护者、楼层生成表、actor/loot 总预算、深度与同层多区域主题、区域特殊阶段组合、Vault 多入口/空间落位/跨走廊拼接、巢穴、动态 friends/escort formation、持久 pack AI、程序化地貌、原版式 pit、maze-only、多楼梯、独立到达点、shaft、实例级探索生命周期、入口守卫/可选进入条件、campaign 胜利/退休评分和可配置实例生命周期已经建立。阶段 F 的角色成长、构筑与首轮技能消费已由 v72 固定；阶段 G 的玩家施法循环已由 v73–v85 固定；阶段 H 已由 v86–v89 建立怪物施法、效用权重、阵营目标、多格结算、敌对召唤、战术移动、有限抗性记忆和玩家召唤物命令/行动；v90 建立多职业资源底子与首个技法资源。普通 Echo/Resonance 仍返回地表即清空；原创 Archive 覆盖 retained/TTL。任务线也已补齐暂停任务的地表放弃、重接上限与确定性重建。运行时地形破坏直接写入权威地图，不触发自动连通修复；玩家可通过挖掘自行恢复通路。v92 建立混乱/致盲/麻痹新状态族与玩家侧效果。v93 为四种伤害效果补充平坦加值并映射旧版弹/球直伤两族（DETECT 系经源码核实为附身专用组、怪物不施放，已按不适用归档）。v94 建立吐息族：伤害 = 施法者当前 HP 百分比封顶上限、零伤害骰，锥形几何复用 v79 机制，导入器同轮补齐 FREQ_N 频率语法并把附身组 token 重分类为不适用。下一步按导入缺口报告推进 S_ 召唤族或伤害类型扩展。

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
$env:RFB_LEGACY_SOURCE = "D:/codex/Frogcomposband/master"; cargo run -p rfb-legacy-import -- import-content .local/packs/rfb-legacy
```

`rfb-legacy-import` 当前只读取不依赖旧 C 结构体内存布局的 409 字节稳定前缀，包括版本、保存元数据、63 项 RNG 状态和选项位。生成的 `parsed-save-samples.json` 仍位于 `.local/`，不会进入 Git；`record-catalog` 拒绝覆盖已有基线。

快照规范化和 hash：

```powershell
cargo run -p rfb-contract -- normalize-snapshot <snapshot.json>
cargo run -p rfb-contract -- hash-snapshot <snapshot.json>
cargo run -p rfb-contract -- validate-policy tests/fixtures/contract-v94/baseline-policy.json
```

当前 306 个原创 contract fixtures、自动协议生成、原创内容包、ASCII glyph atlas、图片 tileset manifest、缺失资源回退和 Windows Tauri 端到端测试已经建立。桌面 E2E 可用以下命令运行：

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
