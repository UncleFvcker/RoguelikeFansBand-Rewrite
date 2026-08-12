# 并行 worktree 通用交接说明

更新时间：2026-08-12  
当前工作批次起点：`main@bf7d28958`

本说明供新的 Codex 对话接手独立方向时使用。每个写入方向必须使用自己的
worktree 和分支；`main` 只负责集成、版本与发布收口。

## 1. 子任务交接模板

创建新对话时，复制下面内容并填完方括号：

```text
项目：D:\codex\RoguelikeFansBand-Rewrite
工作树：[绝对路径]
分支：[codex/分支名]
起始基线：[main 提交]

目标：
- [这一对话要完成的单一、可验收目标]

拥有范围：
- [允许修改的目录、文件或机制]

不拥有范围：
- [由其他对话负责的目录、文件或机制]
- 不清理、回滚或覆盖来源不明的现有修改。

已完成与待完成：
- 已完成：[当前可靠基线]
- 待完成：[本批明确工作]
- 明确不做：[后续批次或无关扩展]

共享边界：
- [是否修改协议、存档、State Hash、RNG、公共初始化、world 定义或 importer]
- [是否新增跨方向使用的 actor、item、ability、task 或 terrain ID]
- [可能与其他分支冲突的具体文件]

验收：
- [聚焦 Rust 测试]
- [内容验证]
- [需要时的前端测试、typecheck 或 build:ui]
- 不运行旧 E2E，除非任务直接涉及 E2E 故障或用户明确要求。

交付：
- 只提交本方向修改和聚焦测试。
- 报告提交号、修改摘要、已运行检查、未运行检查和已知剩余问题。
- 不自行合并其他方向；由 main 集成对话合并并统一收口版本。
```

## 2. 接手时先检查

```powershell
git status --short
git branch --show-current
git log -5 --oneline
git merge-base HEAD main
```

- 工作树不干净时，先辨认已有修改的归属；不得擅自清理或覆盖。
- 分支基线落后时，先在交接中说明，不要把同步主分支夹进功能提交。
- 一个对话只推进交接中声明的方向。发现相邻缺口时记录，不顺手扩建。

## 3. 项目共同约束

- Rust Core 是唯一权威规则层；TypeScript 只负责输入、投影和界面。
- RFB 原版事实以 `D:\codex\Frogcomposband` 仓库的 `master` Git ref 为准，
  必须通过 Git 对象读取，不能依赖该仓库当前工作树。
- 导入内容的中文名必须逐字采用权威中文表或源字符串；没有权威名称时标记
  `unresolved`，不得自行翻译。
- 测试从新存档开始。除非用户明确要求，不增加旧开发存档兼容、双读或迁移路径。
- `web/src/protocol.ts` 是生成文件，不得手工修改。
- Contract fixture 只覆盖一个最小行为；非移动测试使用直接位置前置条件。
- 日常只运行聚焦测试、内容验证、类型检查和相关构建。旧桌面 E2E 不作为日常验收。
- 可玩桌面包必须使用 standalone Tauri 构建，普通 Cargo build 不能代替分发验收。

## 4. 文件所有权与高冲突区

方向分支应独占自己的内容文件和运行时模块。以下文件或概念属于高冲突区，修改前
必须在交接中明确声明：

- `crates/rfb-protocol/src/lib.rs` 与生成的 `web/src/protocol.ts`；
- `crates/rfb-core/src/game/persistence.rs`、State Hash 输入及版本常量；
- 公共 RNG 顺序、角色初始化、回合入口和 `GameUpdate` 投影；
- `packs/rfb-demo-original/worlds/middle-earth.json`；
- importer 的共享解析与生成逻辑；
- `packs/rfb-demo-original/pack.json`、`content.lock.json`；
- `schemas/`、Contract baseline、公共 fixtures 和目录数量断言。

`pack.json` 与 `content.lock.json` 可在方向分支中为了内容验证而更新，但它们是预期的
机械冲突。集成时不保留任一分支的最终 hash；所有内容合并后统一确定版本并重新生成
内容锁。协议版本、State Hash Schema、Contract baseline 和批量 fixture 刷新同样只在
集成阶段统一执行一次。

例外：用户已明确授权 `codex/class-next` 的 Archer Commit 3 在方向分支完成一次完整
收口。该提交暂时独占 `AbilityDto.uiGroupNameKey`、Protocol `1.176`、State Hash Schema
v88、`contract-v266`、公共 fixtures 与 pack `1.261.0`；其他方向不得并行编辑这些协调点，
集成时以 main 的最新版本为准重放一次机械版本合并。

若两个方向都需要同一个共享文件，优先采取以下顺序：

1. 指定一个方向暂时独占该文件；
2. 另一个方向只记录依赖，等待前者合并；
3. 无法拆开时，把共享底座单独做成一个先行提交并先合入 `main`；
4. 不在两个分支中各自实现相似但不同的协议、状态或事务。

## 5. 最小验证原则

按实际改动选择检查，不机械运行全部命令：

```powershell
cargo test -p rfb-core <相关测试过滤器>
cargo test -p rfb-content
cargo test -p rfb-contract <相关测试过滤器>

Set-Location web
npm test
npm run typecheck
npm run build:ui
```

- 改动内容定义或 importer：至少运行相关 importer/内容测试和内容验证。
- 改动协议或前端投影：生成绑定后运行协议测试、前端测试和 `typecheck`。
- 改动持久状态、State Hash、公共初始化或 RNG：明确列出受影响 fixture，并由集成对话
  判断是否需要 `refresh-all` / `verify-all`。
- 不用跳过、削弱断言或添加无条件 fallback 来让测试通过。

## 6. 提交前交接清单

```powershell
git status --short
git diff --check
git diff --stat
```

最终交接必须说明：

- 分支、基线和提交号；
- 实际修改的行为与主要文件；
- 新增或改变的稳定 ID；
- 协议、存档、State Hash、RNG 和内容版本是否受影响；
- 已运行及未运行的测试；
- 仍未实现的原版行为或已记录 omission；
- 与其他方向可能发生的文件冲突和建议合并顺序。

## 7. main 集成对话职责

集成对话按依赖顺序合并，不重新实现方向功能：

1. 先合并共享底座，再合并依赖它的内容方向；
2. 解决语义冲突，包括协议版本、RNG 顺序、存档结构和稳定 ID 重复；
3. 合并全部内容后统一更新 pack 版本与内容锁；
4. 统一生成协议绑定和 Schema；
5. 只刷新真正受影响的 fixtures；
6. 运行跨方向验证，最后提交并推送 `main`。

严重冲突无法在不改变任一方向语义的情况下解决时，停止合并并给出修复 plan：列出
冲突文件、双方合同、建议权威实现、依赖顺序、版本影响和重新验收范围。

## 8. class-next 当前交接（Archer Commit 3）

- 分支：`codex/class-next`；起始基线仍为 `main@3fb94bcd`。
- Archer 的一个原版“制造弹药”菜单由三个既有执行 ID 组成：
  `demo.ability.archer-create-shots`、`demo.ability.archer-create-arrows`、
  `demo.ability.archer-create-bolts`；没有新增 ability ID 或命令。
- 三项 `ClassAbilityDefinition.uiGroupNameKey` 均为
  `ability-group-demo-archer-create-ammo-name`。前端只对分组子项执行等级隐藏，1/10/20
  级依次开放；方向和物品目标选择完全复用既有流程。
- 内容 ID 所有权保持：`demo.item.shard-of-pottery`、`demo.item.broken-stick`、
  `demo.affix.ammo-elemental`；`rfb-legacy.affix.slaying` 与 items 方向同 ID 合并，禁止
  再建 ammo-slaying。
- 共享协调点：Protocol `1.176`、State Hash Schema v88、active baseline
  `contract-v266`、pack `1.261.0`。合并其他方向后重新生成协议/内容 Schema、内容锁，
  并只在最终 State Hash 版本再刷新一次 fixtures。
- 明确剩余：Good/Bad Luck、Chance virtue、coffee/special mode、地下城 good/great
  上限和全局 `no_egos` 继续归共享物品生成上下文，不在 Archer 内建立替代规则。

## 9. main 当前交接（神授祈祷底座）

- 本批直接在主工作树推进，起点为 `main@bf7d28958`；后续圣骑士内容应以包含
  contract-v272 的 main 为起点，不在旧 `codex/class-next` 上重做学习命令。
- `CastingProfileDefinition.studyMode` 支持默认 `chosen` 与 `divine-random`；高阶法师
  继续使用默认点选学习。书本级命令为 `StudyPrayer { bookItemId }`，没有新增 ability
  ID、item ID 或持久状态。
- 神授候选只来自所选的当前领域能力书，排除已学和等级不足项目；按书内稳定顺序使用
  唯一 gameplay RNG 做原版 `one_in_(k)` 蓄水池抽样。背包和玩家脚下的书均可使用，
  失明、无光和混乱会在核心阻止学习。
- 共享协调点：Protocol `1.177`、State Hash Schema v88、active baseline
  `contract-v272`、pack `1.272.0`。内容源未改变，content hash 保持
  `2f88338bb3fe9bfa13ac703d0b58ae4521bade19619805c5fe37da977a8b4858`。
- 下一步圣骑士内容只需声明 `studyMode: divine-random` 并增加已预留的
  `demo.ability.paladin-hell-lance`；不新增祈祷菜单状态或第二套 RNG。
