# Warrens 怪物接入与对话交接手册

状态：P41 完成后的当前工作手册。它描述现在唯一在用的怪物接入路径，供新的
Codex 对话在 monster worktree 中继续工作；历次批次的详细规则仍以对应 contract
文档和 [怪物机制清单](warrens-monster-mechanism-backlog.md) 为准。

## 1. 接手时先确认的状态

截至 P41，monster worktree 的功能提交为 `39dfc59b`：

| 项目 | 当前值 |
| --- | --- |
| 工作树 | `D:\codex\rfb-monsters` |
| 分支 | `work/monsters` |
| 最新怪物批次 | P41 妖鬼与 `ELDRITCH_HORROR` |
| 正式内容包 | `1.225.0` |
| 内容哈希 | `005d3db278c595029ef2a65e8f46dcd3748c303bc96681a1a513dfc24b54c43d` |
| 严格同步怪物 | 369 |
| 正式 actor / ability | 434 / 174 |
| 协议 / State Hash | `1.158` / Schema `v78` |
| 行为基线 | `contract-v229`，471 条 exact fixtures |

这些数字只是交接锚点。开始新批次前必须重新读取 `pack.json`、
`content.lock.json`、`rfb-protocol::PROTOCOL_VERSION`、
`STATE_HASH_SCHEMA_VERSION` 和 `baseline-policy.json`，因为集成对话可能已经推进。

先执行：

```powershell
git status --short
git log -5 --oneline
```

工作树不干净时，不得清理、回滚或覆盖来源不明的改动。若主集成分支已经前进，
先明确同步点，再继续怪物批次；不要在功能提交里顺手合并城镇或物品工作。

## 2. 权威来源与不可变规则

新规则和内容的唯一权威来源是 `D:\codex\Frogcomposband` 仓库的 `master` Git
ref。只能通过 Git 对象读取，不能依赖该仓库当前检出的分支或工作树：

```powershell
$rfbGitDir = 'D:\codex\Frogcomposband\.git'
git --git-dir=$rfbGitDir rev-parse master
git --git-dir=$rfbGitDir show master:lib/edit/r_info.txt
git --git-dir=$rfbGitDir show master:src/monster_name_zh.inc
git --git-dir=$rfbGitDir show master:lib/help/PossessorStats.csv
```

每只候选怪物至少核对：

- `N/G/I/W`：源索引、英文名、字形；速度、生命骰、感知、AC、睡眠、重量；
  等级、稀有度、最大深度、经验值和进化字段；
- 所有 `B:`：有序近战方式、效果与骰值；
- 所有 `A:` / `O:`：接触光环与职业/主题掉落；
- 所有 `F:`：类型、抗性、移动、群体、掉落、分配和特殊机制；
- 所有 `S:`：施法频率、能力 token、显式骰值和召唤类别；
- `D:` 与中文表：中文显示名必须逐字使用权威表；没有权威中文名时标记
  unresolved，不能自行翻译。

旧开发存档没有兼容义务。除非用户明确要求，不增加旧字段迁移、双读、fallback
或兼容层。

## 3. 先按机制风险分批，不再按等级硬切

新候选按下面顺序归类，停在第一种能完整表达真实行为的类型：

| 批次类型 | 判定 | 通常改动 | 代表批次 |
| --- | --- | --- | --- |
| 直接复用 | 现有 actor 字段已表达全部 `B/F`，且无怪物施法缺口 | selection、生成 actor、名称/描述、内容断言 | P37A |
| 复用现有能力 | `S:` 的完整参数签名已经存在 | 同上；actor 只引用既有 ability | P37B |
| 参数化能力 | effect 已存在，但骰值、等级、半径、上限或召唤签名不同 | importer 映射生成新的 ability JSON 和 Ability Program | P38A/P38B |
| 低风险窄机制 | 缺少一个边界明确、无新持久状态的真实行为 | 最小内容字段或专用 effect、一个运行时入口、聚焦测试 | P39 |
| 复杂机制 | 影响形态、可见性、AI、RNG、存档或 State Hash | 最少的新状态与触发点、save/hash/协议收口、完整契约 | P40/P41 |

实现前的规划表至少保留这些列，便于另一对话复核依赖而不重做扫描：

| 源索引 | 权威中文名 | 等级 | `B/F/S` 关键依赖 | 风险分类 | 新 ability 参数 | 机制 blocker | 版本影响 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 327 | 妖鬼 | 19 | 三段近战、亡灵、`ELDRITCH_HORROR` | 复杂机制 | 无 | 新进入视野的理智冲击 | Protocol/Schema |

“相似”不等于“可复用”。楼层传送不能用同层放逐近似，理智冲击不能用普通
恐惧近似，真实形态变化也不能用纯外观投影近似。缺少真实机制的怪物应留在候选
表中，等窄契约完成后再接入。

数值不同不需要新 effect 类型，但需要一条携带完整参数签名的 ability 内容记录。
相同签名由 importer 去重并共享；不要建立数值覆盖层。

以下 `S:` token 是原版附身者/拟似者提示，不生成怪物施法能力：

```text
DETECT_TRAPS DETECT_EVIL DETECT_MONSTERS DETECT_OBJECTS IDENTIFY MAPPING
CLAIRVOYANCE MULTIPLY BLESS HEROISM BERSERK
```

## 4. 唯一内容接入流水线

### 4.1 严格选择文件

正式选择只维护在：

```text
packs/rfb-demo-original/legacy-warrens-monster-selection.json
```

普通条目格式：

```json
{
  "sourceIndex": 327,
  "id": "ghast",
  "tags": ["undead", "warrens"],
  "omittedFlags": ["COLD_BLOOD", "POS_HOLD_LIFE"]
}
```

- `sourceIndex` 固定原版身份；`id` 是正式稳定 ID 的尾段。
- 只有稳定 ID 必须不同于源英文名时才增加 `sourceId`。
- `tags` 只补项目已承诺的分类或 habitat，不用标签伪装未实现机制。
- `omittedFlags` 必须与 importer 当前无法表达的源旗标集合完全相等。多写、少写、
  名称漂移、重复索引和 `DEPRECATED` 记录都会让同步失败。
- 废弃记录只能通过顶层 `deprecatedReplacements` 显式绑定到活跃替代索引。

### 4.2 扩展 importer

主要入口在 [`crates/rfb-legacy-import/src/content.rs`](../crates/rfb-legacy-import/src/content.rs)：

- `monster_flag_is_mapped`：完整导入器已经认识的旗标；
- `demo_monster_flag_is_handled`：正式 demo 同步确认能承载的旗标；
- `demo_monster_json`：严格遗漏核对、actor 字段、施法和掉落生成；
- `map_spell_token` 及其子函数：能力 token → 参数化 ability；
- `POSSESSOR_ONLY_SPELLS`：不属于怪物施法的 token；
- `sync_demo_monsters`：固定选择、生成 actor/ability/Ability Program。

新增映射必须留下一个最小 synthetic importer 测试。测试只使用合成记录，不能把
整段旧版数据复制进 fixture。

### 4.3 运行严格同步

```powershell
$env:RFB_LEGACY_SOURCE = 'D:\codex\Frogcomposband'
cargo run -p rfb-legacy-import -- sync-demo-monsters `
  packs/rfb-demo-original/legacy-warrens-monster-selection.json `
  packs/rfb-demo-original/actors
```

最后一个参数必须是 `packs/rfb-demo-original/actors`，不能是 pack 根目录。
同步会重写全部已选 actor，并在相邻 `abilities/`、`abilityPrograms/` 写入所需参数
记录。它不会替你删除已经失去引用的旧 ability 文件，因此映射变化后要审查 orphan。

生成的 actor JSON 是 importer 产物，不手改。先审查：

```powershell
git status --short
git diff -- packs/rfb-demo-original/actors `
  packs/rfb-demo-original/abilities `
  packs/rfb-demo-original/abilityPrograms
```

## 5. 缺少机制时的最短实现路径

先搜索已有消费者，优先顺序固定为：复用现有字段 → 复用现有 effect → 增加窄
内容字段/effect → 最后才增加运行时状态。常用位置如下：

| 职责 | 文件 |
| --- | --- |
| Actor 内容模型 | `crates/rfb-content/src/definitions/actors.rs` |
| Actor 内容验证 | `crates/rfb-content/src/validation/actors.rs` |
| 怪物 AI 与施法选择 | `crates/rfb-core/src/game/monster_ai.rs` |
| 怪物能力结算 | `crates/rfb-core/src/game/monster_abilities.rs` |
| 近战、偷窃、接触光环 | `crates/rfb-core/src/game/monster_combat.rs` |
| 分配、群体、形态、生态触发 | `crates/rfb-core/src/game/monster_ecology.rs` |
| 行动时序 | `crates/rfb-core/src/game/turn.rs` |
| 运行时 actor / 存档转换 | `crates/rfb-core/src/state.rs`、`save.rs` |
| 对外事件 | `crates/rfb-core/src/event.rs` |
| 协议与 save DTO | `crates/rfb-protocol/src/lib.rs` |
| Web 事件显示 | `web/src/event-format.ts` |

实现时保持以下确定性边界：

- RNG 只在原版会抽取的位置消费，拒绝、免疫和无目标路径应保持零额外抽取；
- 多效果顺序按源声明顺序，死亡后立即停止不再合法的后续结算；
- 规则身份使用 `kindId`，外观或真实形态使用项目已有的 runtime definition；
- 可以派生的值不保存；只有跨存档必须保留、且无法从现有状态恢复的事实才新增
  save 字段；
- 不为一只怪物建通用框架。先实现能覆盖当前真实机制的窄契约。

新增运行时机制至少需要一个聚焦 Core 测试，固定触发门、RNG 抽取数、作用顺序和
关键失败路径。新增事件时同步英中 Fluent 和 `web/src/event-format.test.ts`。

## 6. 名称、描述和内容断言

每只正式怪物通常还需要：

- `locales/zh-CN/content.ftl`：权威中文名和权威中文描述；
- `locales/en-US/content.ftl`：稳定英文显示名和项目自有简述；
- `crates/rfb-content/src/tests/world.rs`：Warrens roster 元组及本批关键语义断言；
- `crates/rfb-content/src/tests/pipeline.rs`：正式 actor 总数；
- `crates/rfb-content/src/tests/catalog.rs`：正式包版本及受影响内容集合；
- 若新增 actor 改变召唤候选、分配池或其他精确集合，同步对应 Core 断言。

严格同步数量和正式 actor 数量不是同一个数：正式包还包含手写、任务和系统 actor。

## 7. 版本、哈希与 contract 收口

### 7.1 何时升级什么

| 改动 | Pack | Protocol | State Hash Schema | Fixture 范围 |
| --- | --- | --- | --- | --- |
| 只新增 actor/ability/locales | 升级 | 不变 | 不变 | 只验证/刷新实际受影响分类 |
| Actor 内容 Schema 增字段，但不保存运行时状态 | 升级 | 通常不变 | 不变 | 受影响分类 |
| 新增对外 DTO 或 save 字段 | 升级 | 升级 | 若进入哈希则升级 | 按公共影响决定，通常全量 |
| 公共初始化、RNG 或 State Hash 输入变化 | 视内容而定 | 视 DTO 而定 | 按结构变化升级 | 全量回放/刷新 |
| 仅 Web 文案或 formatter | 不变 | 不变 | 不变 | Web/本地化测试 |

当前项目每个正式怪物批次都写一份 `design/contract-vN-*.md`，并同步：

- `crates/rfb-contract/src/lib.rs` 的 `ACTIVE_BASELINE`；
- `tests/fixtures/active/baseline-policy.json` 的 `baseline`；
- README、内容格式、路线图、pending 和怪物 backlog 的当前状态。

不要因为纯内容哈希变化就全量刷新 fixture。只有 State Hash 输入、公共协议投影、
公共初始化/RNG 或实际场景行为变化时才扩大范围；具体规则见
[基线更新策略](baseline-update-policy.md)。

### 7.2 内容包和锁

先提升 `packs/rfb-demo-original/pack.json`，再取得确定性摘要：

```powershell
cargo run -p rfb-content --bin rfb-contentc -- inspect-source `
  packs/rfb-demo-original
```

人工审查摘要后，把版本与哈希写入 `content.lock.json`，然后验证：

```powershell
cargo run -p rfb-content --bin rfb-contentc -- verify-source `
  packs/rfb-demo-original
```

若内容模型或协议 DTO 改变，分别重新生成并提交生成物：

```powershell
cargo run -p rfb-content --features schemas --bin generate-content-schemas
cargo run -p rfb-protocol --features bindings --bin generate-bindings
```

## 8. 验证门

先跑最窄测试，再按实际改动扩展。常规怪物批次的最小验收是：

```powershell
cargo fmt --all -- --check
cargo test -p rfb-legacy-import
cargo test -p rfb-content
cargo test -p rfb-core <聚焦测试过滤词>
cargo run -p rfb-content --features schemas --bin generate-content-schemas -- --check
cargo run -p rfb-protocol --features bindings --bin generate-bindings -- --check
cargo run -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original
cargo test -p rfb-contract
git diff --check
```

改到事件、本地化或 Web 时再加：

```powershell
cargo test -p rfb-localization
Set-Location web
npm test
npm run typecheck
```

State Hash、公共初始化或 RNG 变化时：

```powershell
cargo run -p rfb-contract -- refresh-all tests/fixtures/active/baseline-policy.json
cargo run -p rfb-contract -- verify-all tests/fixtures/active/baseline-policy.json
```

内容或行为只影响局部时，使用 `list-categories`、`refresh-category` 和
`verify-category`。日常不运行完整桌面 E2E；只有相关故障、明确要求或里程碑验收
才运行。

最终至少执行：

```powershell
cargo check --workspace --exclude rfb-tauri
git status --short
```

## 9. 多对话与冲突边界

城镇、物品和怪物必须使用不同 worktree/分支。即使功能方向不同，也会同时修改：

- `crates/rfb-legacy-import/src/content.rs`；
- `pack.json`、`content.lock.json`、Fluent 内容；
- actor/ability 总数和精确候选集合测试；
- `rfb-protocol/src/lib.rs`、Core save/state/event；
- contract baseline、README、路线图和发布文档。

同一个 monster worktree 同一时间只允许一个写入对话。方向对话提交聚焦功能；
pack/hash、协议、Schema 和 baseline 最好由集成对话统一收口。若怪物对话自行收口，
必须先确认其他方向没有同时修改这些共享文件。

交接消息至少包含：

- worktree、分支和提交哈希；
- 本批怪物源索引与 contract 文档；
- 修改过的共享文件；
- Pack/Protocol/Schema/baseline/count/hash；
- 已运行的测试和未运行的大型测试；
- 工作树是否干净、是否仍有 blocker。

## 10. 历史样例与下一批起点

- [P37A：直接复用](contract-v222-warrens-content-p37a-direct-harvest.md)
- [P37B：复用现有能力](contract-v223-warrens-content-p37b-existing-abilities.md)
- [P38A：伤害参数](contract-v224-warrens-content-p38a-damage-parameters.md)
- [P38B：治疗与召唤参数](contract-v225-warrens-content-p38b-healing-summoning.md)
- [P39：低风险窄机制](contract-v226-warrens-content-p39-jump-light-multiple-auras.md)
- [P40：复杂形态状态](contract-v228-warrens-content-p40-chameleon.md)
- [P41：复杂可见性与理智后果](contract-v229-warrens-content-p41-eldritch-horror.md)

下一批不要直接假定为某个等级或沿用旧聊天名单。重新从当前 `master` 扫描尚未进入
selection 的记录，列出每只怪物的 `B/F/S` 依赖，再按第 3 节五类风险排序；优先
收割直接复用和现有能力批次，复杂机制各自单独成批。

## 11. 可复制给新对话的开场

```text
在 D:\codex\rfb-monsters 的 work/monsters 工作树继续怪物线。
先完整阅读 AGENTS.md、design/warrens-monster-integration-handoff.md 和
design/warrens-monster-mechanism-backlog.md，并重新读取当前 pack/protocol/schema/
baseline，不能照抄文档里的历史数字。

权威来源只读 D:\codex\Frogcomposband 的 master Git 对象；中文名必须使用权威表。
先扫描尚未进入 legacy-warrens-monster-selection.json 的候选，按“直接复用、现有
能力、参数化能力、低风险窄机制、复杂机制”分类，报告名单与依赖。得到推进指令后
再实现。不要增加通用框架、旧开发存档兼容层或近似机制；不要修改或清理其他方向
的未提交改动。
```
