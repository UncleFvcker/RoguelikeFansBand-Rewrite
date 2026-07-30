# 交接文档：P29–P97 迭代史与当前状态

> 面向接手本仓库的下一位开发者/模型。截至 2026-07-30，当前权威基线为协议 1.123 / contract-v148，P97 已完成。
> 通读本文 + `design/pending-implementation.md` + `design/legacy-import-priority-v1.md` 即可接力。

## 0. 项目一句话

用 Rust + Tauri + Web 前端确定性重写 FrogComposband（原版 C 源钉在
`D:/codex/Frogcomposband/master` @ v1.3.0.7 / `191f48c3`），以"契约测试基线"驱动迭代：
每轮 P## 迭代对应（通常）一个逻辑 `contract-vN` 基线，行为由稳定目录
`tests/fixtures/active/scenarios` 下的 exact fixtures 锁死。历史基线由 Git 历史保存，
不再复制到新的版本目录。本文 P29–P56 保留详细迭代史，P57–P97 在当前状态中汇总。

## 1. 架构速查

**Workspace 10 成员**（Rust 2024 / MPL-2.0 / toolchain 1.96）：

| crate | 职责 |
|---|---|
| rfb-core | 权威核心（纯 lib）。`game/mod.rs` 26k+ 行是主体；build.rs 编译期烧入 demo 内容包并校验 lock |
| rfb-protocol | 唯一 DTO 契约层。`PROTOCOL_VERSION` 在 lib.rs 顶部；bindings feature 生成 `web/src/protocol.ts` + `schemas/protocol-v1.schema.json` |
| rfb-content | 内容编译器：20 类 JSON → MessagePack + SHA-256 锁；bin `rfb-contentc`、`generate-content-schemas`（需 `--features schemas`） |
| rfb-contract | 契约工具：observe/verify/refresh/validate-policy |
| rfb-replay | 回放录制/校验；STATE_HASH_SCHEMA_VERSION 引用 core 常量（单一来源） |
| rfb-save | 极薄存档容器，只依赖 protocol |
| rfb-legacy-probe / rfb-legacy-import | 零兄弟依赖的隔离工具，只读访问原版仓库；import 产物在 `.local/packs/rfb-legacy/`（gitignore，重跑无痛） |
| rfb-localization | Fluent 运行时，暂无人依赖 |
| rfb-tauri（web/src-tauri） | 桌面壳。**CI 的 Rust job 用 `--exclude rfb-tauri`，本地 clippy 必须用同款命令** |

依赖方向：protocol 是汇聚点；core→(content,protocol)；replay→(core,protocol)；contract→(core,protocol,save)；tauri→(core,protocol,replay,save)。

**web/src**（~32 个 ts）：`main.ts` 2700 行上帝模块（全部 UI 编排）；`protocol.ts` 是生成物勿手改；英中 Fluent 资源由测试直接核对键与变量集合，无需维护第三份键白名单；npm test 的测试文件列表仍是手工列举。CoreTransport 唯一实现是 Tauri 原生传输——纯 `npm run dev:ui` 连不上核心。

## 2. 铁律约定（违反必炸）

1. **每迭代五件套**：`PROTOCOL_VERSION`、state hash `STATE_HASH_SCHEMA_VERSION`（改 hash 输入结构才 bump）、`pack.json` 版本、`content.lock.json`、`BUILT_IN_CONTENT_HASH`（旧 hash 追加进 `PREVIOUS_BUILT_IN_CONTENT_HASHES` 数组）。内容结构体加字段（即使 serde default）会改内容 hash——lock 不匹配时 rfb-core build.rs 直接 panic，**必须先走五件套再编核心**。内容 hash 用 `rfb-contentc inspect-source` 取新值手写 lock。
2. **fixture active-only**：不再创建或迁移全量版本目录。新 contract 只更新逻辑版本、active policy、新增场景和真正变化的 assertions；新场景 `refresh` 录制前要塞完整占位 assertions 才能解析，录制后人工审阅。`active/waivers/` 只保留 `.gitkeep`，出现任何 waiver 条目均使 policy 验证失败。
3. **显示状态**（镜头/缩放/tileset/语言/准星）永不进存档/回放/state hash。
4. **E2E**（web/e2e/tauri.e2e.mjs）只在 CI Windows job 跑，本地验证套件不含它；覆盖桌面启动、渲染、存档、语言切换和交互工作流，不再把内容字形总数当作行为契约。本地 `cd web && npm run e2e` 约 35s。
5. 新法术/效果形态**永远放新怪物**，别动既有加权池（P34 教训）；clippy 退出码别被管道吞掉，单独跑验证。
6. 全套验证：`cargo fmt --check` / `cargo test --workspace --exclude rfb-tauri` / `cargo clippy --workspace --exclude rfb-tauri --all-targets -- -D warnings` / bindings `--check` / schemas `--check` / `rfb-contentc verify-source` / `validate-policy` / web `check:protocol`+`test`+`typecheck`+`build:ui`（+必要时 e2e）。

## 3. P29–P56 迭代史

### 阶段 H 收尾与基建（P29–P30）
- **P29（contract-v89，fe07a7d）友方召唤命令**：玩家召唤物命令/行动模式（跟随/驻守等），阶段 H（怪物施法 AI v86–v89）收官。
- **P30（contract-v90，22acd98）多职业资源底子**：`techniqueProfiles` + 首个技法资源"节奏 tempo"（决斗家近战积累、闲置消散）。同期三批性能/重构：OnceLock 缓存、state_hash 借用载荷、game.rs 拆 mod/tests；修复 rfb-replay hash 常量漏 bump（f013bed）。

### 导入管线建立与法术族大迁移（P31–P43）
- **P31（c95043d）导入管线 v1**：r_info/f_info → `.local/packs/rfb-legacy`（地形 180/1396 怪起步），产物 gitignore。
- **P32（4151ef1）多 blow 近战例程**：旧版怪物多段近战 1:1 映射进 meleeRoutine。
- **P33（12482d4）S: 行状态/治疗映射**：SCARE/SLOW/HASTE/HEAL + 存根资源 essence。
- **P34（contract-v91，1554289）位移族**：blink-self / teleport-self / teleport-target 三效果。
- **P35（contract-v92，9d1f99d）新状态族**：混乱（bounded(4)==0 保持否则重定向+禁施法）/致盲（is_visible 短路）/麻痹（dispatch 层 ParalyzedIdle）；状态 tick 按世界 tick（每行动 10）。导入 CONFUSE 223/BLIND 215/PARALYZE 110。
- **P36（contract-v93，4fc097c）弹/球直伤族 + 平坦加值**：四伤害效果加 `damageBonus`（平坦伤害 1d1+(F-1) 恒等式）；BO_/BA_ 按 monspell.c 默认公式映射 +622 实例。DETECT 系经源码核实为附身专用、不算欠账。
- **P37（contract-v94，e4c8f48）吐息族**：`breath-damage` 伤害=min(施法者当前 HP×pct, cap)、零伤害骰，锥形复用 v79；同轮修 FREQ_N 频率语法（297 怪）、附身组 522 实例重分类、施法表上限 32→64。吐息 337 实例。
- **P38（contract-v95，0ffae02）按类别召唤**：`summon-category` 按 actor 标签+等级上限过滤、数量掷骰、有界抽取；S_ 族映射 493 实例。踩坑：执行臂漏 `entities.push` 静默丢实体；存档校验需同步接受类别召唤物。
- **P39（协议 1.96，026c4ff）伤害类型扩展**：按原版 gf.h 原序扩到 28 类。**纯枚举迭代：demo 零变更、无契约迁移——先例：协议 bump 不必然伴随 contract bump**。异种元素弹/球/吐息全解锁（+778 映射）。
- **P40（contract-v96，6c98169）内容层抗性档**：`ActorDefinition.resistances`（伤害类型→vulnerable/resistant/strong/immune）；**权威模型：生成期盖章、存档为准**（读档不回溯）；RES_/IM_/HURT_ 导入 1023 只怪 3842 条。
- **P41（contract-v97，14a8560）心灵族**：新增 psi 类型（29 类）；MIND_BLAST/BRAIN_SMASH 用既有 Sequence 组合 psi 伤害+状态骑手（psi 抗性同时减伤+缩时）；PSY_SPEAR 首个导入 beam。
- **P42（contract-v98，b0b471b）诅咒族 + 首个法术豁免门**：curse-damage 先掷 v72 saving-throw（成功全免零后续 RNG、失败全额无护甲参与）；CAUSE_1-4 240 实例。豁免难度=等级×3/4，失败种子靠对抗掷低值狩猎。
- **P43（contract-v99，6e05fa7）杂项效果包**：teleport-away 推离/drain-resource 吸资源滋养/amnesia 豁免失忆（清当前层地图记忆）/DISPEL→remove-status。法术导入线仅余结构性缺口（S_ 字形 177、DARKNESS 85、ANIM_DEAD 58、ANTI_MAGIC 47、TRAPS 44、SHRIEK 42 等）。

### 物品导入线（P44–P46）
- **P44（689d735）k_info 物品导入 v2**：544/545 基础物品；tval 形态表（武器 meleeProfile、弓按 sval 配对典型弹药、护甲 defense、消耗品/设备/魔典壳）；行为缺口按类计数（consumable-effect 95/book 72/device 64 等）。
- **P45（276f947）e_info 词条 + a_info 神器**：ego→affix（88/160 落地，72 条力量全在不可表达旗标按 ego-inexpressible 跳过）、神器 392/392；**用户域修正：普通戒指/护符无任何属性**——属性与 pval 只经词条或神器携带（与原版生成模型一致）。
- **P46（bd8e84f）fake bow 修正**：未配对发射器（竖琴/枪械 12 件）按原版 `obj_is_fake_bow` 语义保 launcher 槽、不带射击档、神器固定修正保留。

### 身体/角色/旗标（P47–P49）
- **P47（contract-v100，3c0671b）身体/槽位模板**：装备模型从"物品自声明槽+同名一件"升级为显式身体模板。核心 `STANDARD_BODY_SLOTS` 13 槽（**ring-1+ring-2 双戒指、light 光源槽**；单实例槽 id=类型名故旧档零迁移）；`RaceDefinition.bodySlots` 种族自声明（对齐 b_info 按种族绑定）；装备按类型找首个空实例、满则确定性顶替首实例（item.equip.swap）；player.bodySlots 入档入 hash（**Schema v41**）、旧档零 RNG 派生。前端全槽位面板（空槽"空缺"、同类型序号）。导入器光源接 light 槽（帕蓝提尔等 8 件神器六维回收）。记录差异：双持手与箭袋未纳入。
- **P48（c8ce182）b_info+种族+性格导入（T1）**：**首次代码侧结构化提取**——种族/性格无数据文件，从 src/*.c 提取函数体并解析 `me.field = 值;` 赋值行（右值非整数字面量的 21 个怪物种族标记 dynamic 跳过）。67/88 种族、20/21 性格、113 身体模板普查；钩子缺口量化（calc_bonuses 76/birth 27）；八技能花名册 1:1 + 87 skillSets；玩家种族绑 Standard 12 槽（刻意无原创 charm 槽）。
- **P49（contract-v101，ec91b5b+707fb7f）装备/内在旗标·防御面（T2 前半）**：协议 1.101 / 包 1.92.0 / Schema 保持 v41 / 323 fixtures。Item/Affix/Race 三处统一声明 `resistances`（复用 v96 档位词表）+ `statusImmunities`（FREE_ACT→rfb.status.paralysis）、StatModifiers 新增 `speed`。核心 `effective_player_resistances()` 确定性合并：**immune 任一即胜；strong>resistant 取最高正档、遇任一 vulnerable 源降一档；纯 vulnerable 保持**——派生值不入存档/hash，穿脱即时生效。免疫查表在落状态前跳过（既有 skip 形状零后续 RNG）；装备 speed 进派生速度管线。物品 DTO 知识门控暴露防御表面。demo 三新物品：御火指环/疾行靴/镇静吊坠（fixtures 321-323）。导入器回灌：ego 105/160（+17）、神器 392/392、35 词条/33 种族/321 物品带防御表面，RES_*/IM_*/SPEED/FREE_ACT 全部退出未映射清单。本迭代细节见 `design/contract-v101-defensive-flags.md`。
- **P50（contract-v102）装备旗标·进攻面（T2 后半）**：协议 1.102 / 包 1.93.0 / Schema 保持 v41 / 326 fixtures。Item/Affix 统一 `slays`（11 类目标，slay/kill 两档）与五元素 `brands`；玩家持武器近战按原版 tier 只放大武器骰，多项取最高，元素 immune 压制对应 brand，零额外 RNG。DTO 与 Web 按物品知识门控显示。demo 新增屠龙刃/余烬刃/寒霜猎手词条（fixtures 324-326）。导入回灌：ego 107/160、神器 392/392，12 词条/130 物品带 slay，5 词条/90 物品带基础 brand。详见 `design/contract-v102-offensive-flags.md`。
- **P51（contract-v103）动态 affix 实例 + 装备 passive**：协议 1.103 / 包 1.94.0 / Schema v42 / 328 fixtures。Affix 新增按深度过滤的加权 `rollGroups`，生成结果以 `rolledAffixes` materialize 到物品实例、存档和 hash；旧档缺结果保持空且零 RNG。`equipmentBonuses` 覆盖额外攻击、十类技能、红外/光照，`passives` 建立 14 项能力词表；regeneration 已每 10 ticks 恢复 1 HP，其余 passive 保留为后续系统的权威数据。Web 中英显示完整属性，contract final state 直接记录 inventory/equipment DTO。demo Adaptive Echo 的 fixtures 327-328 锁住两个浅层候选、真实掉落、鉴定、装备、再生和回档。真实导入 ego 128/160、神器 392/392；详见 `design/contract-v103-dynamic-affixes.md`。

## 4. 当前基线与下一步

- **P57** 完成 Death 第四册，四册合计 32 abilities、4 books、12 个静态职业和 384 行参数覆盖。
- **P58–P61** 建立充能物品、动态设备、自然恢复/主动充能和有序恢复消耗品。
- **P62–P67** 依次完成鉴定、地图侦测、传送召回、装备附魔、诅咒解除和四种召唤卷轴；真实 `scroll-effect` 缺口降至 34。
- **contract-v118 维护** 删除 13 类无权威消费者的装备 passive；只保留 regeneration 与 vampiric，历史 no-op 兼容留在存档 DTO 边界。
- **P68 / contract-v119** 接入 Dispel Undead 与 Banishment，共用可见且 line-of-effect 可达的 actor 快照。协议保持 1.118，demo 1.110.0，save v1，state hash Schema v52，active baseline 422 exact、零 waiver；内置 hash 为 `a9fa7d716f4f5e13ba8f97cb9c72f1dfbb4ed84c83a284b3cde2219549fcb1dd`。固定原版导入的 `scroll-effect` 34→32，真实包 hash 为 `eaf66414ab9d7eda4bac24957b4263e101250ac90b84a3f5cff9d0b9730e1bf7`。
- **P69 / contract-v120** 接入 Blessing、Holy Chant 与 Holy Prayer，复用 self-target 与既有 blessed 状态结算。协议保持 1.118，demo 1.111.0，save v1，state hash Schema v52，active baseline 423 exact、零 waiver；内置 hash 为 `b62824da6e34e2f72a367f94b2e46e50e279ba6ac4df88bece81021a156e90ab`。固定原版导入的 `scroll-effect` 32→29，真实包 hash 为 `b008570c950fab4541286f1eccd86926f1c535cc0dea0770f038cca523b4e643`。
- **P70 / contract-v121** 接入 Trap/Door Destruction，按固定八方向把陷阱替换为 disarm target、封闭门替换为 bash target；空用消费、Aware、零 RNG，开启/破损门保持不变。协议保持 1.118，demo 1.112.0，save v1，state hash Schema v52，active baseline 424 exact、零 waiver；内置 hash 为 `3fd2b0a8b58531b89629aa2b50ef943a7a5687bdcb619991a26a3c81a7437bf7`。固定原版导入的 `scroll-effect` 29→28，真实包 hash 为 `ad65fb2058f2a01b47ec73a616606d4550b5b807cb653d9410aafe0bfd49b6e2`。
- **P71 / contract-v122** 接入 Fire 与 Ice，复用 self-target、既有范围格/墙阻挡/RFB 衰减、actor 抗性/死亡和玩家入伤管线；Fire 为 666/r4/`25+1d25`，Ice 为 800/r4/`30+1d30`。协议保持 1.118，demo 1.113.0，save v1，state hash Schema v52，active baseline 425 exact、零 waiver；内置 hash 为 `ab0bcb63b25c6729fd95d5fba97a4f618f7aca4589f3931a9ac149615d6062b5`。固定原版导入的 `scroll-effect` 28→26，真实包 hash 为 `54649044572c7ef0f36e7d078dc338680cab6489cfb29c3f723dbf5a7a5bc280`。
- **P72 / contract-v123** 接入 Mana 卷轴，复用同一中心爆发路径并用必填 `backlashUsesResistance` 区分反噬：1100/r4 actor mana 爆发尊重目标抗性，玩家 `50+1d50` 反噬忽略玩家 Mana 抗性但保留 incoming-damage 百分比。协议保持 1.118，demo 1.114.0，save v1，state hash Schema v52，active baseline 426 exact、零 waiver；内置 hash 为 `db5233e09952166a195617182db8020cfacc457e2279d0ff403f16a941c49db2`。固定原版导入的 `scroll-effect` 26→25，真实包 hash 为 `745204c6290b7cc64d5a5eda1783bb4212b43a74d932aa822799c46301fe03a5`。
- **P73 / contract-v124** 接入 Aggravate Monster。距离小于当前两倍权威视距的存活 actor 清除 sleep 并警戒，当前视距内有几何 LOS 的敌对 actor 延长 100 ticks haste；玩家阵营只唤醒。合法使用无条件消费、Tried + Aware 且零效果 RNG。协议保持 1.118，demo 1.115.0，save v1，state hash Schema v52，active baseline 427 exact、零 waiver；内置 hash 为 `337e8599f02e53264b45ac1e899eb47b5ec6f4eeb6be0ae31b517c67ae6fb82b`。固定原版导入的 `scroll-effect` 25→24，真实包 hash 为 `3dd566a5705f3d7d9671a2fbabc03451802718024a1870b236af3d0088dd8ec7`。
- **P74 / contract-v125** 接入 Mass Genocide。半径 20 内存活 actor 按稳定实体 ID 顺序结算，普通目标按 power 300 对抗直接移除，unique/guardian 必定抵抗，每候选均产生 `1d3` 疲劳；空候选消费、Aware、零效果 RNG。直接移除不触发 XP、掉落、尸体、任务或守护者胜利事务。协议保持 1.118，demo 1.116.0，save v1，state hash Schema v52，active baseline 428 exact、零 waiver；内置 hash 为 `39a7a79bdabafa301140266e7119735a0a0f16ef6a7071b8c5d06de6a53655a8`。固定原版导入的 `scroll-effect` 24→23，真实包 hash 为 `aeba4b11bddc16259fd02558f666bdca774fe3f5dd7d347b35330cc6bc24436b`。
- **P75 / contract-v126** 接入 Forest Creation 与 Wall Creation。固定八邻格只替换显式源地形，跳过玩家、存活 actor、地面物品和权威楼层连接；预先规划后原子提交，成功才 Aware，空结果消费、Tried-only、零效果 RNG。协议保持 1.118，demo 1.117.0，save v1，state hash Schema v52，active baseline 429 exact、零 waiver；内置 hash 为 `7d344bf57cf11e303fbbd6b98f9792e572792e97a696e9a2c1987ba6f349a149`。固定原版导入从 `FF_FLOOR` 派生源 ID，`scroll-effect` 23→21，真实包 hash 为 `1eb1303a7476dcbce4209460a0af728019680112d55a767c03d2c39ade00bdad`。
- **P76 / contract-v127** 接入 Vengeance。`25+1d25` KeepStrongest 状态在完整怪物 melee routine 或 spell cast 后按实际玩家 HP 损失反击来源一次；零伤害/玩家死亡抑制，每次反击扣 5 ticks，零 RNG、跳过抗性，击杀复用统一 actor death 事务。协议保持 1.118，demo 1.118.0，save v1，state hash Schema v52，active baseline 430 exact、零 waiver；内置 hash 为 `c920d9f1b78d5f51a8ebb1097a54c1f74efe7b4a83eb469809b2c3e60d9717d3`。固定原版导入的 `scroll-effect` 21→20，真实包 hash 为 `2178aea924ffe39476e2c89c668e13a98555b2f8a41d9315aa9630b32d0f4afc`。
- **P77 / contract-v128** 接入 Monster Confusion。玩家专属准备态在 miss/致死时保留，首个非致死命中先清态，再按 `NO_CONF`、目标 level 与玩家 level 结算 confusion；不建立通用 on-hit/prepared-effect。协议 1.119，demo 1.119.0，save v1，state hash Schema v53，active baseline 431 exact、零 waiver；内置 hash 为 `757be0f1513b9cbfb2f77e08ceef8bff8ffcdb10fc7da17a0da05dbe32f908a0`。固定原版导入的 `scroll-effect` 20→19，真实包 hash 为 `cd8e1982e33c20555019b77bec49a44fb1028e81bf54729923b5e78a7cbc1d3e`。
- **P78 / contract-v129** 接入 Protection from Evil。Extend 状态持续 `3 * level + 1d25`；只在 evil 怪物近战命中后、伤害骰前执行 Wisdom/等级对抗和 `one_in(3)` 绕过，击退时跳过该 blow。协议保持 1.119，demo 1.120.0，save v1，state hash Schema v53，active baseline 432 exact、零 waiver；内置 hash 为 `27ad6b88a3e4bdeb4f1464d2081f6f59e62cbbfbab14ed09e9b5bdfaf43ead24`。固定原版导入的 `scroll-effect` 19→18，真实包 hash 为 `db78e5d8fe181d88943b024647afb94791c0e3f00adb25ab3271e18c67bde408`。
- **P79 / contract-v130** 接入 Genocide。单字符 glyph 选择当前楼层存活 actor，按稳定实体 ID 复用既有 `1d4` 疲劳、unique/guardian 保护和 power 300 对抗；非法输入原子拒绝，合法空选择消费、Aware、零效果 RNG。协议 1.120，demo 1.121.0，save v1，state hash Schema v53，active baseline 433 exact、零 waiver；内置 hash 为 `786aba7f693bac066d6caa0dbc848c97ac7bc01e4652bfeb2674cfa739130549`。固定原版导入的 `scroll-effect` 18→17，真实包 hash 为 `4814e2cd4a0d8ac582c1b514e1cbc7998760cbe26f6293a6ab5bd5ff5324707a`。
- **P80 / contract-v131** 接入 Recharging。互异的卷轴/来源/目标背包 ID 通过窄 `UseItemForRecharge` 命令提交；非法组合零时间零 RNG，合法事务消费卷轴后先支付来源损毁或能量，再复用 P60 目标失败公式。协议 1.121，demo 1.122.0，save v1，state hash Schema v53，active baseline 434 exact、零 waiver；内置 hash 为 `d486f818e41cea542ac951f6a92abca69e298d29f5139e6219ddd0c34836ad52`。固定原版导入的 `scroll-effect` 17→16，真实包 hash 为 `3df0f3da5a5700ba42d0e6b40a1bcd630d298d1f808292f1da5e043dfb33084b`。
- **P81 / contract-v132** 接入 Spell。Class `usesSpellScrolls` 声明资格，合格职业固定永久增加 1 点学习容量；无资格职业仍消费、Aware、推进时间且零效果 RNG。bonus 进入默认 0 的 save 字段，协议保持 1.121，demo 1.123.0，state hash Schema v54，active baseline 435 exact、零 waiver；内置 hash 为 `25d972db57c825d4e23f5a61532c00579f9467acbe10edf97f2c0600b00514f5`。固定原版导入的 `scroll-effect` 16→15，真实包 hash 为 `6feceb4793b043f03c826cb242a9e182edf49ea2c708fffac31fa8f30daf589d`。
- **P82 / contract-v133** 接入 Slowness Potion。窄 `apply-slowness` 固定 `15+1d25` 并总是掷一次持续时间，以 KeepStrongest 合并 Slow；只有首次新增状态才 Aware，已有 Slow 即使被延长也保持 Tried-only。协议保持 1.121，demo 1.124.0，state hash Schema v54，active baseline 436 exact、零 waiver；内置 hash 为 `5ef19e0ecaf7328a7eb4ef3ff69ca066858ca0cc718c6b2db84b078e281f2404`。固定原版导入的 `consumable-effect` 81→80，真实包 hash 为 `d13e08a4feccd9717bac5eeab937f81266cad791e7ca53d8ca631abf88fe5764`。
- **P83 / contract-v134** 接入 Death Potion。窄 `self-life-loss { amount: 5000 }` 直接扣除玩家生命，绕过护甲、抗性与 incoming-damage 缩放，零效果 RNG 且总是 Aware；demo 使用原创公开物品 Mortal Draught。协议保持 1.121，demo 1.125.0，state hash Schema v54，active baseline 437 exact、零 waiver；内置 hash 为 `1c6e2bf891c76796cca6eb53ea014caa03fb8bb1fa3a95b8df8fd81f942e8562`。固定原版导入的 `consumable-effect` 80→79，真实包 hash 为 `ab0e840f704f3c9a1e9de7ba5c6c2f0ab28ea6dc775a037a54104b1bb9970210`。
- **P84 / contract-v135** 接入 Poison Potion。窄 `apply-poison` 先固定抽取 `bounded(55)` 并与既有 Poison 抗性档阈值比较；抵抗成功保持 Tried-only 且不抽持续时间，失败后才抽 `1d15+9`、Extend Poison 并 Aware。协议保持 1.121，demo 1.126.0，state hash Schema v54，active baseline 439 exact、零 waiver；内置 hash 为 `497fbc6b137e9bc2d8162ad52b0253f4d655a37c58abe391be6bcdd94ef94d9e`。固定原版导入的 `consumable-effect` 79→78，真实包 hash 为 `54244a2fd227878c7017bc8dfe2bd125c48f65cb093a198547bdcd891f1aef3c`。
- **P85 / contract-v136** 接入 Thermal Potion。窄 `apply-thermal-resistance` 只抽一次 `1d10+10`，以 Extend 应用单一 Thermal 状态并同时授予 Fire/Cold Resistant；首次新增才 Aware，已有状态延长保持 Tried-only。协议保持 1.121，demo 1.127.0，state hash Schema v54，active baseline 440 exact、零 waiver；内置 hash 为 `3098d9de2051029b4509acc3b8973cec0b76679dcacfa6ace1244864bc3f363d`。固定原版导入的 `consumable-effect` 78→77，真实包 hash 为 `9832b1a0d8c31d49407adb4f4a9dd9982292dab35b1d50c8b187670fa825a370`。
- **P86 / contract-v137** 接入 Resistance Potion。窄 `apply-basic-resistance` 每次只抽一次 `1d20+20`，以 KeepStrongest 应用单一 Basic Resistance 状态并同时授予 Acid/Electricity/Fire/Cold/Poison Resistant；合法使用无条件 Aware。协议保持 1.121，demo 1.128.0，state hash Schema v54，active baseline 441 exact、零 waiver；内置 hash 为 `b33b104f3d7fd2153a66597b4f7685647020f3c9e3352366840dac326e650a57`。固定原版导入的 `consumable-effect` 77→76，真实包 hash 为 `430e28aaf60a043a344c02dc8d41185aaa0e33e0393da034fe0af9bbf0d785a2`。
- **P87 / contract-v138** 接入 Speed Potion。窄 `apply-speed` 在没有 Haste 时抽一次 `1d25+15` 并 Aware，已有 Haste 时零 RNG、固定延长 5 ticks；复用既有速度派生和调度。协议保持 1.121，demo 1.129.0，state hash Schema v54，active baseline 442 exact、零 waiver；内置 hash 为 `1b3c059fedbc14ad79a9549a8b0bd4496f22785355e2bb4ef1ce3a0f763c7e35`。固定原版导入的 `consumable-effect` 76→75，真实包 hash 为 `4b35c7d998cbb576b952384ce2c587a261a4dd28628dda451f04466e116a983f`。
- **P88 / contract-v139** 接入 Heroism Potion。窄 `apply-heroism` 每次抽取 `1d25+25` 并 Extend Hero，复用既有状态派生授予 max HP +10、melee/ranged skill +12 与 Fear 免疫；首次新增才 Aware，已有状态延长保持 Tried-only。协议保持 1.121，demo 1.130.0，state hash Schema v54，active baseline 443 exact、零 waiver；内置 hash 为 `99c41b9668586d97987cc18a459632c8f444d9c8dffbf1e6e024f2ce35a11091`。固定原版导入的 `consumable-effect` 75→74，真实包 hash 为 `47b741de879cefd63ad79a6d9ea4643c1e37b4444c63b9b581a3598a620241cc`。
- **contract-v139 后 importer 维护** 复用 P61 已有恢复序列，将 tval 75/sval 67 映射为固定治疗 200 后依次解除 Blindness、Confusion 与 Stun；不增加 demo、contract 或 fixture。固定原版导入的 `consumable-effect` 74→73，源码校验、编译与二进制回读 hash 均为 `50318233b8a4df980ac2b5c3492a8633a4a0b6536d5cd65ed62aaf23a21ac282`。
- **P90 / contract-v140** 接入 Berserk Strength Potion。窄 `apply-berserk-strength` 先按 `1d25+25` Extend Berserk，再治疗 30；首次新增状态或实际治疗任一成立即 Aware，单纯延长保持 Tried-only。协议保持 1.121，demo 1.131.0，state hash Schema v54，active baseline 444 exact、零 waiver；内置 hash 为 `de5986a0133867854afb49f98e06a294528d9e4360bc88e7a0fa78d48fff8846`。固定原版导入的 `consumable-effect` 73→72，真实包 hash 为 `b143ba1a8198e280fbedfdb595088e9b572ef830731eed7ee101d6ce9f80ac0d`。
- **P91 / contract-v141** 接入 Poetic Inspiration Potion。窄 `apply-poetic-inspiration` 每次按 `1d100+100` Extend 状态并授予 Wisdom/Charisma 各 +5；首次新增才 Aware，重复延长保持 Tried-only。协议保持 1.121，demo 1.132.0，state hash Schema v54，active baseline 445 exact、零 waiver；内置 hash 为 `6ecb079e1a1dd1e653e7c4d201f264d72e7c1db9bfe466f8d1ffa410cfee36e0`。固定原版导入的 `consumable-effect` 72→71，真实包 hash 为 `53fd88e36019c7c40f177a00cc16a9bc019c51e3f31cb8c9b5b7036417a8fa89`。
- **contract-v141 后 importer 维护** 复用 P84 已有 `apply-poison`，将 tval 80/sval 0 映射为相同抗性检定与 `1d10+9` Poison；不增加核心、demo、contract 或 fixture。固定原版导入的 `consumable-effect` 71→70，源码校验、编译与二进制回读 hash 均为 `f916b49530a6eebe54908ecdc18ab32360e17dd3177d759df68b4003e8abe602`。
- **P92 / contract-v142** 接入 Stone Skin Potion。窄 `apply-stone-skin` 每次按 `1d20+20` 以 KeepStrongest 应用状态，并按饮用时等级授予 `10 + 40 * level / 50` defense；首次新增才 Aware，更长刷新保持无新效果。协议保持 1.121，demo 1.133.0，state hash Schema v54，active baseline 446 exact、零 waiver；内置 hash 为 `48611b108dafc4b06836073ca6b5c6881779c653cbab569a7fdeaec82c1c707a`。固定原版导入的 `consumable-effect` 70→69，真实包 hash 为 `845faf23ab10df14f22dbf5c14481db63385e210011d548ee7bbd18ee5cb4136`。
- **P93 / contract-v143** 接入 Restore Life Levels Potion。窄 `restore-life-levels { lifeForceAmount: 150 }` 先恢复历史最高经验并重算等级，再增加生命力且封顶 1000；任一变化才 Aware，完全无变化保持 Tried-only，效果零 RNG。协议保持 1.121，demo 1.134.0，state hash Schema v54，active baseline 447 exact、零 waiver；内置 hash 为 `8b3bdb097563d99b6433a5746c07d395b406d5c8d86616540e0126cd6af72404`。固定原版导入的 `consumable-effect` 69→68，真实包 hash 为 `c7d1868b4ed9452c9159b6870af80eb942bfca3350f76d42c2b540a90b710ed1`。
- **P94 / contract-v144** 接入 Blindness Potion 与 Blindness Food。窄 `apply-blindness` 固定先抽一次 `bounded(55)` 抗性 RNG，拥有 Blindness 免疫时短路持续时间；未抵抗时按来源掷 `1d100+99` 或 `1d25+24` 并 Extend Blindness，首次新增才 Aware，已有状态延长保持 Tried-only。协议保持 1.121，demo 1.135.0，state hash Schema v54，active baseline 448 exact、零 waiver；内置 hash 为 `9f28bf79c8fc72bbcf97beec23da1c1fa0a10045b5c363defcb59e9a29457ed5`。固定原版导入的 `consumable-effect` 68→66，`food-nutrition` 保持 28，真实包 hash 为 `47f5a78d899de6cee7339c97832e8cd2aef84049d1394ce42bf6dbcc644e8c39`。
- **P95 / contract-v145** 接入 Detonations Potion。窄 `apply-detonation` 按 `50d20` 直接伤害，绕过护甲与 Physical resistance、保留 `incomingDamagePercent`；玩家存活时以 KeepStrongest 施加 75 ticks Stun、以 Extend 施加 5000 ticks Bleeding，致死时不施加后续状态，合法使用无条件 Aware。协议保持 1.121，demo 1.136.0，state hash Schema v54，active baseline 449 exact、零 waiver；内置 hash 为 `136cc9508d1d45997f193c39689f8604e6e06db258e4a2d22e65b7a24b72f717`。固定原版导入的 `consumable-effect` 66→65，真实包 hash 为 `e724905cda4f306f6080e80844e61af0a51f1cc692ae678bedbcf7850f33adb6`。
- **P96 / contract-v146** 接入属性损伤与恢复。玩家进度分离当前自然属性和历史最大自然属性；六种 `drain-attribute` 按原版 18/xx 公式降低当前值，六种 `restore-attribute` 无 RNG 恢复至历史最大值。当前值为 3 时保持下限，旧存档缺最大属性时迁移，current > maximum 的损坏存档拒绝载入；实际变化才 Aware，无变化仍消费并保持 Tried-only。协议升至 1.122，demo 1.137.0，state hash Schema v55，active baseline 450 exact、零 waiver；内置 hash 为 `ffd8f8111a5b956a26a6af12bd242aad04a322bb996f587a08fae9db4488925b`。固定原版导入的 `consumable-effect` 65→53，`food-nutrition` 保持 28，`scroll-effect` 保持 15，真实导入内容 hash 为 `450e3eeaa989e04f15747578abb45449ef9662507b47e6a0e8c823cc93dce867`。
- **P96 修正 / contract-v147** 修复属性变化后资源池先 clamp、再二次比例缩放；六种 sustain passive 同步回到内容、协议、存档、导入器和属性损伤入口。装备维持时属性不变、零效果 RNG，但来源药水 Aware 并发出 sustained 事件。fixture schema 升至 2，schema 1 只迁移六项全缺失的历史投影，部分缺失显式失败；Web cap 判断改用 `maximumNatural`。协议 1.123，demo 1.138.0，state hash Schema 保持 v55，active baseline 451 exact、零 waiver；内置 hash 为 `2b1bf5beabe42513d3ad70e0d536274a773babf391c085f3af4ca7a720a2e003`，真实导入内容 hash 为 `21fb38c839a993bcb5b2b6562a7ff46ce537255052fa4ef41bebc4db00a245c3`。
- **P97 / contract-v148** 接入六种单属性增长与 Augmentation。每项先恢复损伤，再按原版三段公式增长历史最大值；封顶跳过 RNG，Augmentation 固定按六维顺序继续处理。实际变化才 Aware，不消费等级提升点，整瓶只刷新一次派生。协议保持 1.123，demo 1.139.0，state hash Schema 保持 v55，active baseline 452 exact、零 waiver；内置 hash 为 `a8eb3c1a5b74f683bd5a71728da916f67972088769e3155cdc0b89c88b4e874c`。固定原版导入的 `consumable-effect` 53→46，真实导入内容 hash 为 `2a5a78a6c8518385e45babebcc2670edd9ddb653a1eca8da2c78635c497e1138`。
- 下一步重新核对真实报告后选择单一纵切；剩余 15 个卷轴与 46 个其他消耗品分别排期，不把通用状态/伤害 DSL、状态抗性框架、`AbilityEffectDefinition`、通用地形 DSL 或物品事务框架提前纳入。
- 长期设计约束与地牢/楼梯/守护者决定见既有设计文档；显示状态不入存档、回放或 state hash。

## 5. 常用命令

```bash
# 全套验证（CI 等价，Rust 侧）
cargo fmt --all -- --check
cargo test --workspace --exclude rfb-tauri
cargo clippy --workspace --exclude rfb-tauri --all-targets -- -D warnings
cargo run -p rfb-protocol --features bindings --bin generate-bindings -- --check
cargo run -p rfb-content --features schemas --bin generate-content-schemas -- --check
cargo run -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original
cargo run -p rfb-contract -- validate-policy tests/fixtures/active/baseline-policy.json
# web（在 web/ 下）
npm run check:protocol && npm test && npm run typecheck && npm run build:ui && npm run e2e
# 导入器实跑
RFB_LEGACY_SOURCE=D:/codex/Frogcomposband/master cargo run -p rfb-legacy-import -- import-content .local/packs/rfb-legacy
```
