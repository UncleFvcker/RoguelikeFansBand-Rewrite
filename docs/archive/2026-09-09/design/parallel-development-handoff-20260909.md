> 历史快照（2026-09-09 归档）：本文保留当时的设计、版本与验收记录，不作为当前工作指令或待办。现行说明见 [文档索引](../../../README.md)。

# 三工作树、四对话并行开发交接

日期：2026-09-09。本轮功能基线：`main@ad0bd6c6a`，已推送至 `origin/main`。本文是新一轮并行开发的入口；分支、职责和集成安排以本文为准，具体行为以当前代码、根目录 `AGENTS.md` 和 RFB 权威来源为准。

本轮并行对话已经开始。种族职业直接使用现有 `main` 工作树，与主线集成对话共用；法术道具、地牢城镇使用两个独立工作树。下文创建命令仅供尚未建立的工作树使用，不重新初始化已有对话。实际起始提交由各对话记录，不把本文中的快照版本当成永久版本。

## 1. 四个对话的分工

| 对话 | 工作树绝对路径 | 分支 | 职责 |
| --- | --- | --- | --- |
| 主线集成 | `D:/codex/RoguelikeFansBand-Rewrite` | `main` | 平时只读审查交付；在集成时段合并两个方向分支，核对 main 上的种族职业提交，处理版本、生成文件及跨方向验证 |
| 种族职业 | `D:/codex/RoguelikeFansBand-Rewrite` | `main` | 直接在主工作树开发并提交种族、职业、成长、专属能力、出生与职业构筑接入 |
| 法术道具 | `D:/codex/RoguelikeFansBand-Rewrite-realms-items` | `codex/realms-items` | 法术领域、书本、普通物品、装备、Ego、神器、消耗品、装置及激活效果 |
| 地牢城镇 | `D:/codex/RoguelikeFansBand-Rewrite-dungeons-towns` | `codex/dungeons-towns` | 地牢、城镇、荒野地点、设施、任务、入口、生态、守卫与奖励接入 |

三个开发方向共用三个工作树；只有种族职业与集成两个对话共用同一主工作树，不另建种族职业分支。法术道具、地牢城镇不得在主工作树中开发功能，也不得把其他工作树作为输出目录。旧 `codex/monsters-next`、`codex/items-next`、`codex/class-next` 已删除，不恢复或从它们继续。本轮没有独立的“全怪物导入”方向。

主线负责正常的集成修复，但不并行重写方向正在实现的功能。合并遇到原版语义不明确的冲突时，说明冲突和依赖，交由相关方向补充；不得直接选整文件 ours/theirs 或削弱测试。

### 主工作树的使用交接

集成对话平时可以只读审查，但 merge、冲突修复、生成文件、版本收口和提交都会修改主工作树，因此与种族职业开发交替进行：

1. 日常由种族职业对话写入主工作树；集成对话只读审查已提交的 diff 和交付记录，不改变文件、索引或 HEAD。
2. 要开始集成时，种族职业先完成当前小批次的必要验证与独立提交，明确暂停写入并交出当前提交号。集成对话确认没有未完成的功能修改；已有无关未跟踪文件如 `release/` 保留即可，不要求删除。
3. 集成期间种族职业不编辑、生成文件、提交或启动会写入同一构建目录的检查。集成对话完成合并、验证、提交和推送后，报告新的 main 提交并交回工作树。
4. 种族职业重新读取 HEAD、状态和相关合并差异后继续，不能用集成前读到的文件内容覆盖新版本。没有明确交接时，集成对话继续只读，不自动 stash、提交或清理种族职业的在途修改。

交接只需在对话中说明当前使用方与提交号，不建立额外锁文件或调度系统。两个独立方向在主工作树交接期间仍可继续各自开发。

## 2. 接手时必须知道的现状

### 版本与实现状态

| 项目 | 本次快照 |
| --- | --- |
| 协议 / State Hash Schema | `1.230` / `v108` |
| save header / payload / 容器 | `v5` / `v5` / `v1` |
| 正式内容包 | `1.384.0`，`packs/rfb-demo-original` |
| 契约基线 | `contract-v306`，26 个 exact fixture |
| 新游戏入口 | 6 个职业构筑、42 个种族；定义数量不等于开放数量 |
| 高阶法师领域 | Death、Arcane、Sorcery、Armageddon、Nature、Life、Daemon、Crusade 已有四册及规则路径；新游戏当前只开放 Death |

详细事实与证据见 [current-status.md](current-status.md)。始终分别记录“内容已定义、规则已实现、玩家入口已开放、实际验收通过”。七个未开放领域不是待重新导入的空白，也不能因核心测试通过就直接声称全部玩家流程已验收。尚未选择本轮要新增的具体种族、职业、领域、物品和地点；新对话先对照现状选择一个有明确验收边界的小批次，不把整个方向视为一次提交。

### 刚完成的核心拆分

六步重构已合入 `ad0bd6c6a`，没有改变玩法、内容或契约预期。不要恢复旧的大文件结构。

| 要修改的职责 | 当前入口，相对于 `crates/rfb-core/src/game/` |
| --- | --- |
| 构造与出生 | `initialization.rs` |
| 学习、遗忘、玩家能力进度 | `player_abilities.rs` |
| 等级与威力计算 / 能力说明投影 | `ability_scaling.rs` / `ability_projection.rs` |
| 施法事务 / 目标规划 | `abilities/casting.rs` / `abilities/targeting.rs` |
| 能力效果 | `abilities/{damage,control,restoration,items,terrain,travel,summoning,compound}.rs` |
| 掉落与生成草稿 | `loot.rs`；复用已有生成和提交规则 |
| 弹道 / 感知 / 光照 | `projectile_geometry.rs` / `visibility.rs` / `lighting.rs` |
| 地点与楼层 | `world/`、`floor.rs`、`travel.rs`、`town.rs`、`tasks.rs` |
| 能力测试 | `tests/abilities/`，按十个行为族定位；不是旧 `tests/abilities.rs` |

`game/mod.rs` 仍保留 `Game` 权威状态与完整命令协调；`abilities/mod.rs` 保留效果分发。继续使用既有 `impl Game`，只为真实消费者开放必要的内部可见性，不加管理器、通用上下文、转发层或第二条规则路径。

重构验收已通过：完整核心 863 项、回放 8 项、前端 180 项；workspace 测试与 Clippy（排除 Tauri）、Tauri all-target 检查、协议绑定、内容 Schema/source-lock 检查和 26 条 exact fixture 回放。此记录不是新功能的验收结果，也没有新增桌面或 Android 全量验收。细节见 [重构执行记录](core-large-file-refactor-plan.md) 和 [测试迁移索引](ability-test-module-map.md)。

## 3. 所有方向共同遵守

### 实现与停止条件

- 完成本批实际需要的最小完整行为，优先复用现有类型、函数和状态。没有当前调用者、原版要求或已观察到的问题，不新增兼容路径、回退链、重试、缓存、配置开关、管理器或抽象层；不会因为“以后可能需要”提前造底座。
- 校验放在已有的真实输入/状态边界，内部调用复用已成立的约束，不逐层重复验证或吞错补默认值。保留真实的内容引用校验、严格读档、RNG/ID 一致性、事务原子性与共享工作树写入隔离。
- 只读本批调用链、所需权威源与直接相关测试；不把每次内容增加变成全仓审计。不因为修改了共享文件就停工，只在实际同时写入同一内容或存在未满足依赖时协调。
- 先利用已有测试；只有当前行为或回归风险缺少覆盖时补最小有效测试。不要求每层各新增一份测试，不复制静态数据矩阵来凑测试数量。
- 适用的检查通过、目标行为完成后就交付。没有新代码、失败或明确未覆盖的受影响路径，不重复或扩大测试；文档修改不重跑代码套件。验证具体选择见第 7 节。

上述默认适用于所有模型和三个开发方向。旧专题的长验收清单是候选覆盖范围，不是每批新增测试或全量重跑的配额；不能用这些默认跳过本批真实边界或根目录 AGENTS.md 的适用要求。

### 项目事实与行为边界

- 所有新规则与内容事实读取 `D:/codex/Frogcomposband/master` 仓库的 `master` Git ref，通过 Git 对象读取，禁止使用其当前检出分支或工作树文件。接手时记录 `git -C D:/codex/Frogcomposband/master rev-parse master`；本次观察到的是 `a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`，以后以当时的 `master` 为准。
- 中文显示名称逐字采用上述 ref 的运行时中文表或源字符串；未找到就标记 unresolved，不自行翻译、不创建占位中文名。旧契约中显式固定的历史源码提交只用于该历史契约。
- Rust Core 是规则、RNG、状态、ID 与命令结果的唯一权威；前端消费投影，不复制公式或业务状态。
- 测试从新存档开始，不添加旧开发存档兼容、双读或默认补值。
- 优先复用已实现的内容与行为。只扩充本批需要的机制，不顺带重构共享大模块或重做已完成的系统。
- fixture 只覆盖一个最小行为；非移动测试使用直接玩家位置前置条件；通用商店购买选择第一项投影库存，只有物品身份本身是测试主题时才固定实例。
- 不覆盖其他人的修改，不自动清理工作树、缓存、`release/` 或无关文件。构建缓存是否存在不影响功能基线；缺失时允许按需重建，不声称缓存已清理。

读取权威资料示例：

```powershell
git -C D:/codex/Frogcomposband/master show master:lib/edit/d_info.txt
git -C D:/codex/Frogcomposband/master grep -n '目标符号' master -- src lib
```

## 4. 方向所有权与交叉依赖

目录不是绝对锁；同目录的文件按内容身份归属。每批交付说明具体修改文件、稳定 ID 和共享机制，避免三方都编辑同一个通用定义。

| 范围 | 默认负责人 | 交叉处理方式 |
| --- | --- | --- |
| `races/`、`classes/`、职业 `skillSets/`、职业出生和成长 | 种族职业 | 种族/职业专属 ability、program、binding 同属此方向；通用效果扩展先声明依赖 |
| 新职业 Build、职业专属玩家 actor | 种族职业 | 领域书、物品引用使用已有 ID；不存在的物品向法术道具方向交接 |
| 已有职业的新领域 Build、`abilityBooks/`、领域法术及程序 | 法术道具 | 职业框架由种族职业负责；新增职业与新领域互相依赖时指定一个 Build 文件负责人 |
| `items/`、`affixes/`、物品效果程序、装置激活和通用生成机制 | 法术道具 | 职业出生、商店库存、任务奖励引用同一物品 ID，不复制近似物品 |
| `worlds/middle-earth.json`、地点、城镇设施、商店、任务、生态与掉落表 | 地牢城镇 | 奖励物品定义由法术道具负责；地牢专属怪物、守卫和其必要能力由地牢城镇负责，优先复用已导入 actor |
| 召唤与怪物公共机制 | 按本批主行为指定一个负责人 | 法术召唤池与地牢生态共同使用 actor；不并行导入同一怪物或复制召唤/生成规则 |
| `initialization.rs`、`player_abilities.rs`、`abilities/*`、`loot.rs` 等共享规则 | 按本批主行为指定一个负责人 | 另一个方向引用已合入底座，或等待先行提交；文件拆小不意味着没有语义冲突 |
| Fluent、tileset 映射、前端菜单与 `PLAYTEST_*_IDS` | 各方向补本批所需接入，集成核对 | 同一键、actor 映射、菜单文件同时改动时在交付中标明；不能只添加定义就报告入口开放 |

稳定 ID 必须先搜索当前正式包与待合并交付清单，再复用或新增。遇到跨方向缺口，记录“所需 ID、原版来源、所需行为、提供方向、依赖提交”，不得先造占位内容让引用验证通过。

schema、协议、持久状态、RNG 或公共初始化确需变化时，按真实调用者确定实现与验证范围。只有两个方向确实依赖同一项新机制时，才指定一个方向先实现可验证的公共底座，其他方向同步后使用；只服务本批的必要改动直接随本批交付，不强制另拆底座提交或等待其他方向。种族职业负责的提交直接进入 main，两个分支的提交由主线合入。常规实现选择自主完成，协调只针对实际重叠或依赖。

## 5. 工作树建立与日常推进

只新增下面两个方向工作树，种族职业直接使用现有 main。创建前先检查现状；下面命令由负责建立工作树的新对话执行，本次尚未执行。若目录或分支已存在，检查其归属与状态后复用，不覆盖、不重建。

```powershell
git -C D:/codex/RoguelikeFansBand-Rewrite status --short
git -C D:/codex/RoguelikeFansBand-Rewrite worktree list
git -C D:/codex/RoguelikeFansBand-Rewrite worktree add -b codex/realms-items D:/codex/RoguelikeFansBand-Rewrite-realms-items main
git -C D:/codex/RoguelikeFansBand-Rewrite worktree add -b codex/dungeons-towns D:/codex/RoguelikeFansBand-Rewrite-dungeons-towns main
```

每个方向接手后检查 `git status --short`、`git branch --show-current`、`git rev-parse HEAD` 和 `git merge-base HEAD main`。确认本树实际包含重构提交与本文。两个独立工作树看不到主工作树的未提交文件；种族职业与集成共用的文件、索引和 HEAD 则完全相同，必须遵守上述交接顺序。

小批次推进顺序：核对原版与当前实现 → 列本批目标/排除项/稳定 ID/共享依赖 → 完成规则、内容及实际所需入口 → 聚焦验证 → 独立提交 → 报告交付。共享底座和依赖它的内容可分成两个可验证提交，不把数个无关系统塞进一次提交。

两个独立方向分支默认用普通 merge 同步最新 `main`，不擅自 rebase、force push 或改写别人依赖的提交。同步前确保已妥善保存自己的修改。交付提交冻结后，主线按提交号合并，避免合并过程中继续移动待验收目标。种族职业的提交已在 main，无需再 merge 或 cherry-pick；交付后由集成对话核对并统一推送。

各工作树默认使用自己的 `target/` 和前端依赖目录；不要为了省缓存强制三方共用一个构建输出目录。只有要运行前端检查时才安装本树缺少的依赖，不顺带搭建未使用的平台环境。

## 6. 版本、生成文件与合并

高冲突区包括 `pack.json`、`content.lock.json`、`rfb-content` 类型与验证、`rfb-legacy-import/src/content.rs`、`rfb-protocol/src/lib.rs`、生成的 `web/src/protocol.ts`、`schemas/`、`persistence.rs`、State Hash 输入、公共初始化/RNG、Fluent、入口菜单及 `tests/fixtures/active/`。

每个方向的提交必须保持自身可验证，包括种族职业直接写入 main 的提交：内容变更更新本树 pack 版本与 lock；实际改变协议或内容类型时生成对应绑定/Schema。不得手工修改生成文件，不能交付故意失配的 lock 或承诺“集成时再让它编译”。仅在存在真实跨方向依赖时按前述方式安排先行底座。

主线每次集成一个小批次：

1. 先完成与种族职业的主工作树交接，再检查交付提交和依赖、读取 diff 与验证记录。先处理共享底座，再处理依赖内容；纯独立批次按准备完成顺序集成，无固定的三方向先后。
2. 核对已在 main 上的种族职业提交，用普通 merge 合并法术道具、地牢城镇的交付提交，解决稳定 ID、语义及真实共享文件冲突。文本无冲突不代表内容引用、学习参数、奖励来源与 RNG 顺序一致。
3. 对合并后的完整内容选择合适的新 pack 版本，使用现有 `inspect-source` 结果更新 lock 并运行 `verify-source`；不能保留任意一侧过期 hash，也不能把两侧版本号机械叠加。只改代码而未改内容时不无故升 pack。
4. 若涉及协议/类型，合并真实定义后重新生成绑定/Schema；版本按最终变更含义协调，不由三个方向各自重复升级同一字段。
5. 复验或有依据地刷新实际受影响的 fixture 类别。State Hash Schema v62 已一次性移除 `contentHash`，纯内容变化不导致全量刷新，也不为内容版本本身升级 State Hash Schema。
6. 更新 [current-status.md](current-status.md) 的实际变化、玩家入口与验收范围，记录合并提交和剩余缺口；验证通过后提交并推送主线，报告新的 HEAD 并将主工作树交回种族职业。

`verify-all`、`refresh-all` 或 ignored 的完整契约回放，只用于状态哈希输入结构、共享协议投影、公共初始化/RNG 变化，或明确的里程碑验收。先解释行为变化，再刷新预期；不能以刷新掩盖失败。精确政策见 [baseline-update-policy.md](baseline-update-policy.md) 与 `tests/fixtures/active/baseline-policy.json`。

## 7. 验证选择

日常按本批实际改动执行聚焦测试、内容验证、类型检查和相关构建，不因多工作树就默认运行全部桌面 E2E。

**按改动触发检查，已有覆盖可直接复用，通过后停止。** 以下方向表用于找测试入口，不要求整行全跑。新增持久状态才需要验证其保存/恢复和相关回放；仅增加已有字段的一个种族数值，不自动增加独立 save/replay 测试。

| 实际变化 | 本批验证选择 |
| --- | --- |
| 仅 Markdown 文档 | 链接/命令核对及 `git diff --check`；不编译、不跑代码测试 |
| 内容沿用已有机制，未改 Rust/前端/协议类型 | 内容 source/lock 验证，加直接覆盖新增引用、参数或行为的现有/必要新增检查；不跑无变化的前端与 schema 生成器 |
| Rust 运行时或 importer 改动 | 新增行为及受影响既有调用路径的聚焦测试；Rust 格式、相关编译/lint。`cargo test` 已编译相同目标时，不机械补同范围 `cargo check`；其他实际受影响 target 才补检查 |
| 前端行为或协议消费者变化 | 相关测试文件/测试名和 `typecheck`；需要确认构建链或交付 UI 产物时才 build，不因后端内容变更默认跑整个 `npm test` |
| 内容类型或协议定义变化 | 生成并检查实际变化的 Schema/绑定，验证真实消费者；不要每批生成所有无变化文件 |
| 持久状态、共享投影、公共初始化或 RNG 变化 | 按实际影响验证保存/恢复、回放和相应 fixture；按 AGENTS.md 的触发条件扩大范围，先解释原因再更新预期 |

测试以可观察行为、真实边界和已知回归为对象。一次表驱动测试能清楚覆盖的同类数据，不拆成大量重复测试；已有通用测试覆盖的稳定机制不为每个内容条目复制一套。测试数没有固定上下限，不能为了少测而漏掉新增分支或削弱断言。

集成复用方向已报告的有效验证结果，检查合并造成的交叉影响和冲突修复；普通 merge 不是全量验收触发器。只有合并改变了被测输入、发现失败或存在尚未覆盖的真实交叉路径时才补对应检查，明确的里程碑再执行完整套件。

| 方向 | 优先验证 |
| --- | --- |
| 种族职业 | 本批种族/职业、出生、成长、学习、战斗与必要存档/回放测试；入口变更时验证菜单和投影 |
| 法术道具 | `tests/abilities/` 对应行为族、`high_mage`、物品/装置/装备/掉落及实际修改的 importer 测试；确认取消、失败、资源和 RNG 行为 |
| 地牢城镇 | `world`、`town`、`tasks`、地牢生成、旅行/召回、守卫奖励及必要存档/回放测试；验证地点正常可达 |
| 主线集成 | 受影响方向与交叉调用测试、生成文件/source-lock 检查；共享变化或明确里程碑再扩大全量范围 |

下面仅展示几种命令形式，不是一组必跑流水线；测试名/模块路径替换为本批实际目标。单个测试优先完整名称配 `--exact`，同族测试用模块路径，避免短子串意外匹配大量无关测试：

```powershell
cargo fmt --all -- --check
git diff --check
cargo test -p rfb-core --lib game::tests::abilities::items::
cargo run -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original
```

生成绑定使用 `cargo run -p rfb-protocol --features bindings --bin generate-bindings`，内容 Schema 使用 `cargo run -p rfb-content --features schemas --bin generate-content-schemas`；只在相应类型变化时生成并追加 `-- --check` 检查。内容锁更新沿用现有 `inspect-source` / `verify-source` 流程，不凭空假定有 `write-lock` 命令。前端聚焦测试示例：在 `web` 执行 `node --test src/character-traits-panel.test.ts`；修改其他界面时选择对应文件。全量 `npm test` 用于实际跨界面变更或明确验收。

明确的集成里程碑沿用 [重构计划第 15 节](core-large-file-refactor-plan.md#15-整轮重构里程碑验收2026-09-09) 的 workspace、Tauri 检查及完整契约命令。制作可玩桌面调试包在 `web` 使用 `npm run build:standalone:debug`；正式包使用 Tauri build，不能以普通 Cargo build 代替。桌面 E2E 仅在相关失败、用户要求或明确的玩家流程里程碑执行；Android 不默认执行。

## 8. 每批交付给主线的格式

直接在方向对话的交付报告中填写，不要求每个小批次另建一套文件：

```text
方向 / 工作树 / 分支：
起始 main 提交 / 本批交付提交：
权威 RFB master 提交及具体源码位置：
完成的具体行为：
内容定义 / 规则实现 / 玩家入口 / 实际验收分别是什么状态：
新增、复用或改变的稳定 ID：
主要文件；共享文件和跨方向依赖提交：
协议 / save / State Hash / RNG / 公共初始化是否变化：
内容版本、lock 和 fixture 变化原因及范围：
实际执行的验证命令与结果：
未运行的相关检查、未实现行为、unresolved 名称或其他剩余项：
建议合并顺序与同步要求：
```

没有变化的版本/状态项合并写一句“协议、存档、State Hash、RNG 未变”即可；无关的未运行检查无需逐项列清单。只记录能帮助集成判断的证据，不为普通提交另写长报告。

## 9. 四个新对话的启动提示词

以下提示词各自复制到对应的新对话。仍是四个对话，但只有三个工作树。尚未初始化时由主线先提交本文并建立两个方向树，再将主工作树交给种族职业；已经运行的对话使用第 11 节纠偏，不重新初始化。所有提示词都应落实第 3 节的最小实现与第 7 节的按改动验证。若实际目录不同，在提示词中统一替换。

### 主线集成对话

```text
请接手 RFB Rewrite 主线集成，工作树 D:/codex/RoguelikeFansBand-Rewrite，分支 main。
先读 AGENTS.md、design/parallel-development-handoff-20260909.md、design/current-status.md。
先检查状态；如果本轮交接文档仍未提交，将本次交接文档修改独立提交并推送，不纳入 release 或其他无关文件。
只检查或创建 codex/realms-items 和 codex/dungeons-towns 两个方向工作树，从包含交接文档的最新 main 开始；已存在时检查后复用。种族职业直接使用当前 main 工作树，不另建工作树或分支。
本对话平时只读审查，负责核对 main 上的种族职业提交，并合并两个方向分支的已交付提交，不重做方向功能。
开始任何集成写入前，确认种族职业已完成当前小批次提交并明确暂停写入；没有交接则保持只读，不自动 stash 或提交其在途修改。
按依赖处理共享底座、稳定 ID、语义冲突、版本和生成物；遵守 fixture 分类验证政策，保留其他人的修改。
收到方向交付并接管工作树后完成相关验证、更新当前状态、提交并推送 main，报告新的 HEAD 并交回种族职业。没有交付时报告已就绪，不擅自推进三个方向的功能。
```

### 种族职业对话

```text
请接手 RFB Rewrite 种族职业方向。
直接使用工作树 D:/codex/RoguelikeFansBand-Rewrite，分支 main，不另建工作树或分支；与主线集成对话共用该目录、索引和 HEAD。
先读 AGENTS.md、design/parallel-development-handoff-20260909.md、design/current-status.md、design/class-race-import-handoff.md。
本方向负责种族、职业、成长、出生、职业构筑和种族/职业专属能力。法术领域、通用物品和地点由其他方向负责。
先检查已实现内容和玩家入口，再依据权威 RFB master 确定一个具体小批次；若我已指定目标则直接推进，否则选择依赖已满足、范围最小的一批，说明依据后推进。
确认主工作树已交给本对话后开始写入。完成规则、内容、所需入口及聚焦测试，将本方向修改独立提交到 main，按交接模板提供提交号和验证结果；不合并其他方向分支，由集成对话统一推送。
需要集成时先完成当前小批次提交并明确暂停写入，交出工作树；集成结束后重新读取 HEAD、状态和相关 diff 再继续，不与集成同时编辑、生成、提交或运行写入同一构建目录的检查。
不恢复旧 class-next 分支，不重做已经合入的六步核心拆分，不覆盖其他人的工作。
```

### 法术道具对话

```text
请接手 RFB Rewrite 法术领域和道具增加方向。
工作树 D:/codex/RoguelikeFansBand-Rewrite-realms-items，分支 codex/realms-items；缺失时按主线交接检查并从包含交接文档的最新 main 创建，仅创建本方向。
先读 AGENTS.md、design/parallel-development-handoff-20260909.md、design/current-status.md、design/spell-realm-import-handoff.md；Ego 工作再读 design/ego-import-plan.md 及对应子类计划。
本方向负责领域、书本、领域 Build、物品、Ego、神器、消耗品、装置与激活；职业专属能力和地点内容归其他方向。
八领域已有四册内容与规则路径，当前新游戏只开放 Death，不能把其余七领域当作未导入或已验收。
先核对原版 master 与现状，确定一个领域/一册/一类物品的小批次；若我已指定目标则直接推进，否则选择依赖已满足、范围最小的一批，说明依据后推进。
复用拆分后的 abilities、player_abilities 和 loot，完成本批所需入口与聚焦测试，独立提交并按交接模板报告，不合并或推送 main。
跨方向的出生物、任务奖励和 actor 使用唯一稳定 ID，不自行造占位内容或复制共享规则。
```

### 地牢城镇对话

```text
请接手 RFB Rewrite 地牢城镇方向。
工作树 D:/codex/RoguelikeFansBand-Rewrite-dungeons-towns，分支 codex/dungeons-towns；缺失时按主线交接检查并从包含交接文档的最新 main 创建，仅创建本方向。
先读 AGENTS.md、design/parallel-development-handoff-20260909.md、design/current-status.md，再读 design/dungeon-addition-handoff.md、design/town-system-handoff.md 的实现方法。
旧专题中的工作树、版本和“尚未提交”描述属于历史，不恢复 monsters-next 或 rfb-town 工作树，也不重做已合入地点。
本方向负责地牢、城镇、荒野地点、设施、商店、任务、生态及必要的专属怪物/守卫；奖励物品定义和通用物品机制归法术道具方向。
先从权威 RFB master 与当前 middle-earth.json、城镇和任务内容核对差距，确定一个地点或一条设施/任务链的小批次；若我已指定目标则直接推进，否则选择依赖已满足、范围最小的一批，说明依据后推进。
复用现有 world/floor/travel/town/tasks 与 loot，完成正常入口、规则、奖励、必要保存恢复和聚焦测试，独立提交并按交接模板报告，不合并或推送 main。
```

## 10. 专题资料的使用顺序

1. 根目录 `AGENTS.md` 与当前代码：约束、模型和实际行为。
2. 本文：本轮分工、工作树、共享边界和交付流程。
3. [current-status.md](current-status.md)：状态口径、玩家入口和验收证据。
4. [职业种族](class-race-import-handoff.md)、[法术领域](spell-realm-import-handoff.md)、[Ego](ego-import-plan.md)、[地牢](dungeon-addition-handoff.md)、[城镇](town-system-handoff.md)：按需读取实现清单，逐项核对当前代码。
5. [旧并行记录](parallel-worktree-handoff.md)、旧 class-next 和历史 contract 文档：只作历史背景，不继承旧分支占用、版本预留或“待合并”结论。

## 11. 已启动对话的行为调整

不要只改 main 中的文件就假定其他对话已采用新规则。独立工作树有各自的文档版本，运行中的对话也可能仍在沿用先前读到的要求。把下面完整消息分别发给四个对话，即使本分支尚未同步本文，也能直接按消息执行：

```text
继续当前已授权任务，从现在起按以下要求调整做法，不重开任务、不撤销他人的代码，也不因为这次纠偏重新跑已通过的检查：
1. 实现本批所需的最小完整行为。复用现有类型、函数和状态；没有当前需求、原版要求或观察到的问题，不新增兼容层、回退链、重试、管理器、配置开关或未来抽象。真实的输入校验、严格读档、事务和 RNG/ID 一致性仍保留。
2. 测试按实际改动选择。先用已有覆盖，只有缺少当前行为/回归证据时补最小有效测试；不要求每层各添一套，不因一项内容增加重测全部职业、领域、保存/回放、前端或 E2E。
3. 相关检查通过就交付。没有新代码、失败或未覆盖的真实影响，不扩大或重复测试；仅文档变更不跑代码套件。协议、状态哈希、共享初始化/RNG 的实际变化仍按 AGENTS.md 执行必要检查。
4. 共享文件不等于需要审批或先造底座；只有实际并发重叠/跨方向依赖才协调。main 上种族职业和集成的写入交接继续遵守。
5. 如能读到最新版 design/parallel-development-handoff-20260909.md，重读第 3、7、11 节；本树还是旧版时先按本消息执行，不为同步文档强行 merge、stash 或中断其他工作。旧专题中的长验收清单按本批适用性取舍。
现在用不超过五行说明：当前目标、复用的现有机制、最小验证集合、删去哪些多余步骤；随后继续执行。不要只回复“收到”，不要发起全仓清理或修改其他对话的工作。
```

集成对话在方便的文档提交中带上本次规则更新；两个方向在正常同步 main 时带入，不为规则同步制造额外代码合并。长期项目要求由根目录 AGENTS.md 维护；专题文档给出本轮细化。发现相互矛盾的旧指南时指出具体条款，按用户最新要求和适用项目规则处理，不靠叠加更多“必须”掩盖矛盾。
