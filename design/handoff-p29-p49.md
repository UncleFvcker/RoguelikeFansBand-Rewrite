# 交接文档：P29–P51 迭代史与当前状态

> 面向接手本仓库的下一位开发者/模型。截至 2026-07-27，P50–P51 已在当前工作树完成，等待提交。
> 通读本文 + `design/pending-implementation.md` + `design/legacy-import-priority-v1.md` 即可接力。

## 0. 项目一句话

用 Rust + Tauri + Web 前端确定性重写 FrogComposband（原版 C 源钉在
`D:/codex/Frogcomposband/master` @ v1.3.0.7 / `191f48c3`），以"契约测试基线"驱动迭代：
每轮 P## 迭代对应（通常）一个 `contract-vN` 基线，行为由 `tests/fixtures/contract-vN/scenarios`
下的 exact fixtures 锁死。当前基线 **contract-v103，328 个 fixtures，零 waiver**。

## 1. 架构速查

**Workspace 10 成员**（Rust 2024 / MPL-2.0 / toolchain 1.96）：

| crate | 职责 |
|---|---|
| rfb-core | 权威核心（纯 lib）。`game/mod.rs` 26k+ 行是主体；build.rs 编译期烧入 demo 内容包并校验 lock |
| rfb-protocol | 唯一 DTO 契约层。`PROTOCOL_VERSION` 在 lib.rs 顶部；bindings feature 生成 `web/src/protocol.ts` + `schemas/protocol-v1.schema.json` |
| rfb-content | 内容编译器：20 类 JSON → MessagePack + SHA-256 锁；bin `rfb-contentc`、`generate-content-schemas`（需 `--features schemas`） |
| rfb-contract | 契约工具：observe/verify/refresh/migrate-baseline/validate-policy |
| rfb-replay | 回放录制/校验；STATE_HASH_SCHEMA_VERSION 引用 core 常量（单一来源） |
| rfb-save | 极薄存档容器，只依赖 protocol |
| rfb-legacy-probe / rfb-legacy-import | 零兄弟依赖的隔离工具，只读访问原版仓库；import 产物在 `.local/packs/rfb-legacy/`（gitignore，重跑无痛） |
| rfb-localization | Fluent 运行时，暂无人依赖 |
| rfb-tauri（web/src-tauri） | 桌面壳。**CI 的 Rust job 用 `--exclude rfb-tauri`，本地 clippy 必须用同款命令** |

依赖方向：protocol 是汇聚点；core→(content,protocol)；replay→(core,protocol)；contract→(core,protocol,save)；tauri→(core,protocol,replay,save)。

**web/src**（~32 个 ts）：`main.ts` 2700 行上帝模块（全部 UI 编排）；`protocol.ts` 是生成物勿手改；`localization.ts` 的 MESSAGE_KEYS 是手工白名单（新 Fluent 键必须登记，有全量对齐测试）；npm test 的测试文件列表也是手工列举。CoreTransport 唯一实现是 Tauri 原生传输——纯 `npm run dev:ui` 连不上核心。

## 2. 铁律约定（违反必炸）

1. **每迭代五件套**：`PROTOCOL_VERSION`、state hash `STATE_HASH_SCHEMA_VERSION`（改 hash 输入结构才 bump）、`pack.json` 版本、`content.lock.json`、`BUILT_IN_CONTENT_HASH`（旧 hash 追加进 `PREVIOUS_BUILT_IN_CONTENT_HASHES` 数组）。内容结构体加字段（即使 serde default）会改内容 hash——lock 不匹配时 rfb-core build.rs 直接 panic，**必须先走五件套再编核心**。内容 hash 用 `rfb-contentc inspect-source` 取新值手写 lock。
2. **fixture 跨基线**：`rfb-contract migrate-baseline <旧>/scenarios <新>/scenarios`；新场景 `refresh` 录制前要塞完整占位 assertions 才能解析，录制后人工审阅；怪物场景种子狩猎=批量改 seed 重 refresh。每个 `contract-vN/waivers/` 必须有 `.gitkeep`。
3. **显示状态**（镜头/缩放/tileset/语言/准星）永不进存档/回放/state hash。
4. **E2E**（web/e2e/tauri.e2e.mjs）只在 CI Windows job 跑，本地验证套件不含它；其内钉死 `contentVisualCount` 等值，**加带字形的内容必改**（P36 72→73、P47 79→80、P49 80→83、P50 83→85、P51 85→87）。本地 `cd web && npm run e2e` 约 35s。
5. 新法术/效果形态**永远放新怪物**，别动既有加权池（P34 教训）；clippy 退出码别被管道吞掉，单独跑验证。
6. 全套验证：`cargo fmt --check` / `cargo test --workspace --exclude rfb-tauri` / `cargo clippy --workspace --exclude rfb-tauri --all-targets -- -D warnings` / bindings `--check` / schemas `--check` / `rfb-contentc verify-source` / `validate-policy` / web `check:protocol`+`test`+`typecheck`+`build:ui`（+必要时 e2e）。

## 3. P29–P51 迭代史

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

## 4. 当前缺口与下一步候选

- **P52 已完成（纯工具）**：54 个职业壳与职业 skillSet；53 份 m_info 共 636 个领域行、144 个可读行和 4608 条逐法术参数；C caster_info 壳与领域可读性表；s_info 的 16640 条武器熟练和 156 条专项熟练进入报告。详见 `design/legacy-class-import-v1.md`。
- **P53 已完成（首批运行时纵切）**：`CastingProfileDefinition.abilityOverrides` 保留同一法术的职业级等级/耗魔/失败率；Death 第一册 `[Stench of Death]` 映射 Malediction、Stinking Cloud、Horrify，12 个静态职业生成真实 castingProfile，共 3 abilities / 1 ability book / 36 行职业参数。敏捷、生命与动态档案继续显式排除。大型源包文件预算 4096→32768，16 MiB 源字节预算不变。详见 `design/legacy-player-spell-import-v1.md`。
- **P54 已完成（contract-v104）**：七类 `levelScaling` 在能力投影/施放统一物化；actor Detect、status power、sleep/受伤唤醒、状态授予临时抗性、Control/controller identity/pack 解散/友方 AI 全部入协议、存档和 Schema v43。Death 第一册达到 8 abilities / 1 ability book / 12 casting profiles / 96 行覆盖；协议 1.104、包 1.95.0、334 exact fixtures、零 waiver。真实 Death 效果缺口 480→384，详见 `design/contract-v104-death-first-book.md`。
- **P55 已完成（contract-v105）**：Death 第二册 8/8 槽位落地；活体限定、bolt-or-beam、职业 beam 概率、自身中心 AoE、single/glyph Genocide、临时 poison 品牌、Drain Life、尸体与永久 Animate Dead 全部进入协议/存档/回放。两册合计 16 abilities、2 books、12 casting profiles、192 行覆盖；协议 1.105、包 1.96.0、Schema v44、343 exact fixtures、零 waiver，Death 效果缺口 480→288。详见 `design/contract-v105-death-second-book.md`。
- **P56 候选**：先读取 Death 第三册八个实际槽位并按系统族聚类；设备/消耗品效果系统仍是并列高收益候选，不以法术名或物品名做行为近似。
- 备选：**设备与消耗品效果系统**（行为缺口 231 + 激活 193，解锁卷轴/魔杖/药水实际效果）；**法术清尾**（S_ 特殊/字形 177、SHRIEK、TRAPS、DARKNESS 房间光照、ANIM_DEAD、ANTI_MAGIC）。
- 导入优先级路线（design/legacy-import-priority-v1.md）：T1✅ T2 防御面✅ T2 进攻面✅ T2 动态实例/passive✅ T3 职业壳+m_info✅ T4 玩家领域法术首批✅（继续逐册）∥设备效果 → T5 d_info/v_info/任务/城镇荒野。
- 能力性旗标已结构化入内容/实例/DTO，但除 REGEN 外仍需运行时消费者；另有种族 rank 动态（21 怪物种族）、双持/箭袋槽、非标准身体玩法待对应系统。
- 长期设计约束（用户已确认、不得推翻）与地牢/楼梯/守护者决定见既有设计文档；显示状态不入档是铁律。

## 5. 常用命令

```bash
# 全套验证（CI 等价，Rust 侧）
cargo fmt --all -- --check
cargo test --workspace --exclude rfb-tauri
cargo clippy --workspace --exclude rfb-tauri --all-targets -- -D warnings
cargo run -p rfb-protocol --features bindings --bin generate-bindings -- --check
cargo run -p rfb-content --features schemas --bin generate-content-schemas -- --check
cargo run -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original
cargo run -p rfb-contract -- validate-policy tests/fixtures/contract-v105/baseline-policy.json
# web（在 web/ 下）
npm run check:protocol && npm test && npm run typecheck && npm run build:ui && npm run e2e
# 导入器实跑
RFB_LEGACY_SOURCE=D:/codex/Frogcomposband/master cargo run -p rfb-legacy-import -- import-content .local/packs/rfb-legacy
```
