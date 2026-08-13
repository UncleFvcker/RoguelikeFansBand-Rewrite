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

## 10. main 当前交接（死亡领域圣骑士正式内容）

- 本批继续直接在主工作树推进，基于 contract-v272 的神授祈祷底座导入
  `demo.class.paladin`、`demo.skill-set.paladin`、`demo.build.paladin-death` 和
  `demo.actor.paladin-player`。
- 原版 Paladin 的属性 `+2/-3/+1/0/+2/+2`、生命 110%、经验 135%、基础 HP 12、
  八项技能及增长、WIS 施法、最低失败率 5 和负重 `450/20/1200` 已进入正式内容；
  Death 的 32 条职业施法参数逐项来自 `master:lib/edit/m_info.txt`。
- 出生物只引用既有 `demo.item.broad-sword`、`demo.item.ring-mail`、
  `demo.item.black-prayers`；本批没有新增或占用 item/ability ID。
- 新游戏列表、职业中英文案、`demo.actor.paladin-player` 三套 tileset 映射和按书祈祷
  UI 已接入。职业显示名保持原版“圣骑士”，Death 只作为 build 领域单独显示。
- 共享协调点：Protocol `1.177`、State Hash Schema v88、active baseline
  `contract-v273`、pack `1.273.0`，content hash 为
  `132b2a15ebcd5b74e2949817b45c88e576c0fae37eaa2e72548972249d70e1ae`。
- 下一步预留 ability ID 仍为 `demo.ability.paladin-hell-lance`；同时补 40 级恐惧抗性。
  摧毁高级异教书经验和逐武器熟练度继续单列，不在本批伪造。

## 11. main 当前交接（圣骑士职业能力）

- `demo.ability.paladin-hell-lance` 已由 class 方向正式占用，并引用新的
  `demo.ability-program.paladin-hell-lance`；其他方向不得重复定义同 ID。
- 地狱长枪在 30 级开放，WIS 检定、Mana 消耗 30、基础失败参数 70、最低失败率 5；
  地狱火 beam 伤害为 `level × 3 + spellDamageBonus`。class power 与 learned spell
  共享施法伤害加成，但不会获得 learned spell 专用的 beam 几率调整。
- `ClassDefinition.levelResistances` 是通用、可选且按等级排序的职业抗性门槛；当前唯一
  内容是圣骑士 40 级恐惧抗性。抗性由 class/level 派生，不新增存档或协议字段。
- 本批没有新增或占用任何 item ID。摧毁高级异教书经验等待生命/圣战高级书正式导入，
  禁止为此建立占位书；逐武器熟练度的后续内容底座见第 12 节。
- 共享协调点：Protocol `1.177`、State Hash Schema v88、save v1、active baseline
  `contract-v274`、pack `1.274.0`，content hash 为
  `e94926512734080f4743341e0eff07e3c96f371fe8cdac674089654b28fa2010`。

## 12. main 当前交接（逐武器熟练度内容底座）

- 新增内容字段 `ClassDefinition.weaponProficiency` 与
  `ItemDefinition.weaponProficiencyBaseItemId`；其他方向不得建立第二套武器身份表。
- 没有新增 item/ability ID。神器只引用现有基础物品 ID：克里斯杜瑞安与杀戮者引用
  `demo.item.executioners-sword`，痛苦引用 `demo.item.glaive`。
- 四个正式职业已导入权威 `master:s_info.txt` 数据；审计覆盖 67 种已选基础武器。
- 共享协调点：pack `1.275.0`、Protocol `1.177`、State Hash Schema v88、save v1、
  active baseline `contract-v274`，content hash 为
  `4274e13bce1b7c3e1808267ac12c1fe4f5fa83e6f256c602c693205396767fa2`。
- 运行时进度、战斗命中/成长、存档和 UI 尚未接入；武术、双持、骑乘仍是独立缺口。

## 13. main 当前交接（逐武器熟练度战斗与存档）

- 沿用第 12 节的内容身份；本批没有新增或占用 item/ability/affix ID，其他方向不得为
  神器或特殊变体建立第二份熟练度进度。
- `CharacterProgress.weapon_proficiencies` 只保存高于职业出生值的规范基础物品训练值；
  `PlayerProgressSaveDto.weaponProficiencies` 必填，严格拒绝缺失、重复、未知、非武器、
  别名及越界数据，不提供旧开发存档兼容。
- 普通近战每命令按武器训练一次、未命中也训练；射击只在弹道碰到怪物时训练。职业上限、
  怪物最低等级/训练上限、成长插值与概率余数 RNG 来自 RFB master `skills.c`。
- 近战/弓/投石索与弩分别使用原版熟练度命中公式并乘 `BTH_PLUS_ADJ = 3`；该修正不进入
  射速与弹药破损率。能力触发的额外近战不走普通攻击训练入口。
- 共享协调点：pack `1.275.0`、Protocol `1.178`、State Hash Schema v89、save v1、
  active baseline `contract-v275`，content hash 保持
  `4274e13bce1b7c3e1808267ac12c1fe4f5fa83e6f256c602c693205396767fa2`。
- 后续 UI 只能投影这份权威稀疏表与职业初始/上限，不得复制成长状态。武术、双持、骑乘
  仍为独立缺口。

## 14. main 当前交接（逐武器熟练度角色面板）

- `PlayerProgressDto.weaponProficiencies` 是第 13 节权威状态的只读投影；包含规范基础物品
  ID/name key、近战/发射器分类、当前值、职业上限、原版等级与原版命中加成。
- 神器和特殊变体不会投影第二行；角色成长面板仅按基础武器显示，并默认折叠。
- 英文等级为 `Unskilled / Beginner / Skilled / Expert / Master`，中文为“生疏 / 入门 /
  熟练 / 专家 / 大师”。前端不得从数值重新推导等级或弓弩公式。
- 本批没有新增或占用 item/ability/affix ID，也没有新增状态哈希输入。共享协调点：pack
  `1.275.0`、Protocol `1.179`、State Hash Schema v89、save v1、active baseline
  `contract-v276`；content hash 保持
  `4274e13bce1b7c3e1808267ac12c1fe4f5fa83e6f256c602c693205396767fa2`。
- active fixture 共 24 条，含近战成长、射击成长和熟练度存档回放 3 条聚焦契约。武术、
  双持、骑乘仍是独立缺口。

## 15. main 当前交接（挖矿系统第一步）

- 新增内容字段 `TerrainDefinition.digging` 与 `ItemDefinition.tunnelingPval`；旧
  `digToTerrainId/digCheckDifficulty` 已删除，其他分支不得继续写入旧字段或复制 resolver。
- 新占用 terrain ID `demo.terrain.rubble`。本批没有新增 item/ability/affix ID；后续矿脉
  产出优先引用 items 分支已有内容，新增物品必须先声明 ID。
- 玩家挖掘力由 38 档力量表、武器/工具单件重量与 pval 最大值、状态原始加值组成；职业
  skill set 不贡献挖掘。四件现有工具的贡献固定为 46/55/66/75。
- soft/hard/permanent 判定与 retryable 已接入；永久墙消耗行动但零 RNG，怪物复用普通
  近战，地面物品不阻塞，门继续拒绝。
- 共享协调点：pack `1.276.0`、Protocol `1.179`、State Hash Schema v89、save v1、
  active baseline `contract-v277`，content hash 为
  `ee561b30744f44fd627805d8ed0a45eb64b21ec4c13991033b6f6254b29156b9`。

## 16. main 当前交接（挖矿熟练度与只读材料袋）

- `CharacterProgress.miningProficiency` 出生 0、上限 8000，只在玩家成功移除带
  `veinYield` 的矿脉后按原版公式成长；非矿脉、魔法、怪物破墙和地震不得调用成长入口。
- `WeaponProficiencyRankDto` 已泛化为共享 `ProficiencyRankDto`。面板从核心投影读取
  挖掘力、等级、当前值/8000，不在前端重算等级。
- 新占用的非物品状态 ID 为：`rfb.material.iron-ore`、`silver-ore`、`mithril-dust`、
  `crystal-shard`、`herb`、`beast-meat`、`dragon-scale`、`demon-ichor`、
  `arcane-essence`、`rare-catalyst`（全部带 `rfb.material.` 前缀）。它们不是 item ID；
  items 分支不得把同名物品改为这些 ID，后续物品产出应通过明确转换引用材料身份。
- 材料袋本批只做权威稀疏存档与只读十项投影，不生成材料，不导入烹饪、炼药或材料转化。
  读档拒绝缺字段、挖矿值越界、重复/未知/零数量材料。
- 共享协调点：pack `1.276.0`、Protocol `1.180`、State Hash Schema v90、save v1、
  active baseline `contract-v278`（25 条 exact fixture、零 waiver）；content hash 保持
  `ee561b30744f44fd627805d8ed0a45eb64b21ec4c13991033b6f6254b29156b9`。

## 17. main 当前交接（矿脉、金币与材料收益）

- 新占用 terrain ID：`demo.terrain.magma-hidden-treasure`、
  `demo.terrain.quartz-hidden-treasure`、`demo.terrain.magma-treasure`、
  `demo.terrain.quartz-treasure`。本批没有新增 item/material/ability/affix ID；items 分支
  不得重复导入这四个 terrain。
- streamer 先判已知、未命中再判隐藏富矿；隐藏富矿复用 `concealedAsTerrainId` 与搜索。
  玩家挖掘统一按“地形 → 熟练度 → 材料 → 金币 → 额外物品”结算，材料与金币读取增长后
  的熟练度。魔法富矿只生成普通金币，怪物破墙没有玩家收益。
- 碎石与富矿额外物品复用楼层 loot；碎石物品来源为新增 `rubble`。原版 Artifact 重试
  已由第 19 节完成；幸运、德行与特殊游戏模式修正仍是共享物品生成缺口，其他分支不得
  在职业或挖矿代码中建立第二套质量生成器。
- 共享协调点：pack `1.277.0`、Protocol `1.181`、State Hash Schema v90、save v1、
  active baseline `contract-v279`（26 条 exact fixture、零 waiver）；content hash 为
  `9e84e738fecbc3b74933c4a708c5a89cd77dd7bdd000c11b76c7d57184abec26`。

## 18. main 当前交接（固定神器统一生成底座）

- 本批不新增正式 item/ability/material/affix ID。随机固定神器池只使用
  `demo.item.crisdurian`、`demo.item.pain`、`demo.item.slayer`；旧
  `demo.item.relic-blade` 没有 `artifactGeneration`，不得加入池或写入唯一状态。
- 核心内部统一使用 `Ordinary / Good / Great / Artifact` 生成意图；`ItemQuality` 与协议
  品质枚举不增加神器值。生成先返回不占实例 ID 的 draft，调用方接受后才提交实例并登记
  神器；其他方向不得在挖矿、任务或职业代码中复制第二套质量/神器生成器。
- Artifact 按 RFB 顺序执行 1/10 瞬时神器入口、`sourceIndex` 顺序、已生成排除、基础物品
  匹配、神器/基础物品 OOD 和稀有度判定；普通固定神器最多尝试四次，失败草稿按 Great
  生成。当前没有 `instant=true` 候选，但入口 RNG 顺序已固定。
- `generated_artifact_ids` 是权威全局唯一集合；`SavePayloadV1.generatedArtifactIds` 必填，
  严格拒绝重复、未知、非正式神器及缺少登记的神器实例。Old Castle 显式奖励始终强制
  授予，同时登记集合，从而只排除后续随机重复。
- 共享协调点：pack `1.294.0` / content hash
  `ee2a72864ac9b521e5825f79a9e020cf798ff1a398e887ca2e7a2b1a5b8edbed`、Protocol `1.187`、
  State Hash Schema v93、save v1、active baseline `contract-v283`（26 条 exact fixture、
  零 waiver）。挖矿额外物品已在第 19 节接入该 draft 接口；丢弃的尝试不会提交实例 ID
  或神器唯一状态。

## 19. main 当前交接（挖矿额外物品品质与固定神器）

- 本批不新增正式 item、ability、material 或 affix ID；固定神器继续只使用
  `demo.item.crisdurian`、`demo.item.pain`、`demo.item.slayer`，由 items 方向持有。
- 富矿额外物品以一次 d100 按 Artifact / Great / Good / Ordinary 顺序分段；挖矿熟练度
  0 时三档概率均为 0，8000 时分别为 5%、20%、40%，沿用原版四舍五入公式。
- Artifact 档最多调用 20 次共享 Artifact 请求，只采用带正式 `artifactGeneration` 的
  固定神器。20 次全部失败后只调用一次 Great；丢弃 draft 不占实例序号、不登记神器，
  固定神器在最终提交时才写入唯一集合，最终来源统一为 `rubble`。
- 玩家显式挖掘仍按“地形 → 熟练度 → 材料 → 金币 → 额外物品”结算。魔法破墙只生成
  普通金币，不进入挖矿额外物品或神器路径。
- 共享协调点保持 pack `1.294.0` / content hash
  `ee2a72864ac9b521e5825f79a9e020cf798ff1a398e887ca2e7a2b1a5b8edbed`、Protocol `1.187`、
  State Hash Schema v93、save v1；active baseline 推进至 `contract-v284`。现有 26 条 exact
  fixture 不进入新增品质分支，逐条及全量 verify 均零漂移，因此不刷新无关快照，零 waiver。

## 20. main 当前交接（items / monsters 新增量整合）

- items 的元素地面物品破坏与 main 的固定神器生成共用当前物品定义；固定神器和声明忽略
  对应元素的物品不会被地面效果销毁。新增 `endurance` affix 后当前共 11 个 affix。
- monsters 的 P59/P60 已接入 8 个 actor、14 个 ability；P60 批量召唤通过
  `SummonCategory.batchCandidates` 先掷数量、再一次选择整批候选，不新增持久状态。
- 共享协调点：pack `1.297.0` / content hash
  `f7aebe082ef8e6b0d5e98633ea229592d516f9d9251db5645a99e11712098744`、Protocol `1.188`、
  State Hash Schema v93、save v1、active baseline `contract-v284`（26 条 exact fixture、
  零 waiver）。`codex/items-next` 与 `codex/monsters-next` 分支及工作树继续保留。

## 21. monsters-next 当前交接（P61 戒灵生命周期）

- 新增 actor `demo.actor.nazgul`（source index 696，中文名“戒灵”），占用内容字段
  `lifetimeInstanceLimit: 5`；普通 `unique` 继续隐式使用上限 1，`unique2` 不记录死亡，
  只限制同时存活一只。
- `defeatedLimitedActorCounts` 是按 actor ID 计数的权威死亡表。普通分配、固定召唤、
  分类召唤、当前楼层与 stored floors 恢复统一按“上限 - 已死亡 - 全楼层存活”计算额度；
  非死亡移除不写入死亡表。戒灵沿用 `unique` 的变形免疫。
- 共享协调点：pack `1.298.0` / content hash
  `ea49544398120d561c201e480e8dce5b918c75d326a73b786ea4c0d371ad7a7b`、Protocol `1.189`、
  State Hash Schema v94、save v1、active baseline `contract-v285`（26 条 exact fixture、
  零 waiver）。旧开发存档不兼容缺少 `defeatedLimitedActorCounts` 的 payload。

## 22. monsters-next 当前交接（P62 玩家临时变形）

- 新增 actor `demo.actor.lord-of-change`（source index 745，中文名“万变魔君”）及近战效果
  `polymorph-player`；两次爪击各自保留原版 20% 门控，效果只改变玩家临时形态，不调用
  怪物目标变形逻辑。
- 临时形态复用既有 `grantedRaceId` 状态，导入 44 个仅供变形使用的 race profile 与
  `legacyIndex`。原版分支按屁精、伊克、小狗头人、疥癣麻风病人及 0–74 race index
  拒绝重掷执行；这些 profile 不开放为出生 build。
- 身体槽随形态立即调整，不合槽装备移入背包或落在脚下；状态到期恢复永久种族身体，
  但不自动重新装备。免疫、豁免、形态与持续时间维持原版 RNG 顺序。
- 共享协调点：pack `1.299.0` / content hash
  `3d83f462010420e8054c18476f7589d859c8e2e9a1c175a08bd3797e120d4c83`、Protocol `1.189`、
  State Hash Schema v94、save v1。没有新增存档或状态哈希字段，active replay baseline
  继续使用 `contract-v285`；行为契约记录为 `contract-v286-player-polymorph`。

## 23. monsters-next 当前交接（猫之女神巴斯特）

- 新增 actor `demo.actor.bast-goddess-of-cats`（source index 777，中文名“猫之女神巴斯特”）。
  四次近战、拖拽、自疗均复用现有机制；`S_KIN` 复用分类召唤，按原版 glyph `f`
  召唤两个等级不超过 62 的同族，避免唯一怪物固定召唤自身。
- `EGYPTIAN` 与 `EGYPTIAN2` 当前只作为未开放神系/金字塔地牢的来源元数据显式省略，
  没有新增神系运行时。神系选择进入范围时应统一恢复其抑制与分配语义。
- 共享协调点：pack `1.300.0` / content hash
  `87f34f4581da4c9065385295f76bd2cfe2a9ae2540cc474bba9777331f7f416a`；Protocol `1.189`、
  State Hash Schema v94、save v1、active replay baseline `contract-v285` 均不变。

## 24. monsters-next 当前交接（P63 L64–68 直接导入）

- P63A 导入 L64–65 的 12 个普通分配 actor：守护神瓦吉特、炸脖龙、混沌猎犬、超人洛克、
  皇帝雷扎克、超人洛克的克隆体、解离漩涡、骨魔、青铜魔像、骸骨魔像、赛特之兽、女武神。
- P63B 导入 L67–68 的 11 个普通分配 actor：混沌巨龙、律法巨龙、蹒跚怪、格拉基、
  操纵大师布雷斯、冰之吉西斯尔、隐形粉红独角兽、征服者大国主、多彩巨龙、女巫师菲奥娜、
  智慧之神思金神。全部保留 RFB master 的 source index、层级和 Orc Cave 分配。
- 本批只新增 40 组既有运行时可表达的参数化 ability/program；没有新增运行时、协议、存档或
  state-hash 字段，也不刷新无关 replay fixture。
- 共享协调点：pack `1.301.0` / content hash
  `c39b143827508a1ea9917f9f45b8e3810f19f03e0ad59b370954677c7a11986d`；Protocol `1.189`、
  State Hash Schema v94、save v1、active replay baseline `contract-v285` 均不变。

## 25. monsters-next 当前交接（P64A L69–70 直接导入）

- 导入 L69–70 的 13 个普通分配 actor：阿登森林领主朱利安、老术士、雷神建御雷神、冰魔、
  蒸汽驱动的机械龙、失落避风港女王昆德丽、邪恶天龙提亚马特、诺萨、兰-提戈斯、
  弹跳地雷、勇者池田、飞天面条神怪、死神镰刀。
- 全部保留 RFB master 的 source index、层级和 Orc Cave 分配；本批只新增 44 组现有运行时
  可表达的参数化 ability/program，没有新增运行时、协议、存档或 state-hash 字段。
- 共享协调点：pack `1.302.0` / content hash
  `0540fb39f3943d390b72fc7815db4e365f68c19e86e3835f91ce4bb04d1c11a0`；Protocol `1.189`、
  State Hash Schema v94、save v1、active replay baseline `contract-v285` 均不变。

## 26. monsters-next 当前交接（P64B 低风险共享映射）

- 导入 7 个 actor：恐惧领主、睡神许普诺斯、终极眼魔、噩梦巨龙、恐惧领主特塞拉库斯、
  解离蜘蛛、蛇王婆苏吉；全部保留 RFB master 的 source index、层级和 Orc Cave 分配。
- `S_NIGHTMARE` 与噩梦巨龙的 `S_SPECIAL` 精确召唤梦魇，数量分别为 `1d3+1`、`1d3+2`；
  `S_AMBERITE`、`S_NAGA` 复用现有分类召唤及唯一额度检查。许普诺斯的 `OLYMPIAN2`
  作为尚未开放神系的来源元数据显式省略。
- `BRAIN_SMASH` 复用 psi 伤害与致盲、混乱、麻痹、减速序列；`JMP_DISINTEGRATE` 复用
  `JumpDamage`。特塞拉库斯和解离蜘蛛保留原版 `DARK`、`DISINTEGRATE` 接触光环；只放宽
  内容验证对既有伤害类型的准入，没有新增 DTO、运行时、协议、存档或 state-hash 字段。
- 共享协调点：pack `1.303.0` / content hash
  `3ee672ec81f6a4c858ee72f01578c5cbf6383ffa990f9769312838ae7f1a0daa`；Protocol `1.189`、
  State Hash Schema v94、save v1、active replay baseline `contract-v285` 均不变。

## 27. monsters-next 当前交接（P65 迪奥·布兰度 WORLD）

- 新增 actor `demo.actor.dio-brando`（source index 878，中文名“迪奥·布兰度”），保留原版
  L66、Orc Cave 分配、近战、寒冷光环、能力和掉落。
- `WORLD` 使用现有 self-target `NoOp` 内容效果加 `monster-world` 行为标签；施法后严格按原版
  `1d2+2` 让同一怪物立即执行 3–4 次行动，不消耗普通能量、不推进 world tick 或其他怪物。
  额外行动期间 WORLD 候选固定为无效，防止时间停止递归；施术者死亡、玩家死亡或楼层切换
  会立即终止剩余行动。`S_KIN` 按原版 glyph `V` 召唤两个不高于 L66 的同族，避免唯一怪物
  固定召唤自身而永远无候选。
- 本批只增加调用期布尔门控，不增加持久状态、DTO、协议、存档或 state-hash 字段，因此不刷新
  无关 replay fixture。
- 共享协调点：pack `1.304.0` / content hash
  `c2986cb253ea4364160787bba7f6ca119afae52f3fae0217655d55cca5c273f2`；Protocol `1.189`、
  State Hash Schema v94、save v1、active replay baseline `contract-v285` 均不变。

## 28. monsters-next 当前交接（P66 L71–76 直接导入）

- P66A 导入 L71–73 的 16 个普通分配 actor，P66B 导入 L74–76 的 17 个普通分配 actor；
  全部保留 RFB master 的 source index、层级、权威中文名和 Orc Cave 分配。
- 本批只新增现有运行时可表达的参数化 ability/program，没有新增 importer 例外、DTO、运行时、
  协议、存档或 state-hash 字段，也不刷新无关 replay fixture。
- 共享协调点：pack `1.305.0` / content hash
  `f368e19819dc892d9514fe9204c19660a0fc6bba68cd4cad4fdbc4fec12ddc2c`；Protocol `1.189`、
  State Hash Schema v94、save v1、active replay baseline `contract-v285` 均不变。

## 29. monsters-next 当前交接（P67 L77–80 直接导入）

- P67A 导入 L77–78 的 11 个普通分配 actor，P67B 导入 L79–80 的 15 个普通分配 actor；
  全部保留 RFB master 的 source index、层级、权威中文名和 Orc Cave 分配。
- 本批只新增现有运行时可表达的参数化 ability/program，没有新增 importer 例外、DTO、运行时、
  协议、存档或 state-hash 字段，也不刷新无关 replay fixture。
- 共享协调点：pack `1.306.0` / content hash
  `a9628dbec24ccc3ec4363bbc6a7a6ae13ac1706984925cc71872d13c9b999514`；Protocol `1.189`、
  State Hash Schema v94、save v1、active replay baseline `contract-v285` 均不变。

## 30. monsters-next 当前交接（P68 低风险映射）

- 导入黄泉之神伊邪那美、火之天使乌列尔、黑夜主宰奈芙蒂斯、地狱蜘蛛、三世卡尔达姆，
  以及 source index 871/1110 两条独立的散失金属史莱姆记录。
- `BA_POIS`、`BA_NEXUS`、`JMP_HELL_FIRE` 与 plasma/hell-fire 接触光环复用现有伤害
  DTO/运行时；Caldarm 的 `S_SPECIAL` 通过 `clone-of-locke` 精确分类召唤 1d3 个 65 级以下
  超人洛克克隆体。`KILL_EXP` 按当前仅击杀结算经验的既有规则省略，不新增运行时分支。
- L71–80 审计现为 67 selected、0 direct、9 blocked、7 excluded；剩余 blocker 是 Banor 三体、
  Skadi、Aegir、Heimdall、Magni、Ganesha、Agni，留给后续 pantheon/SPECIAL 批次。
- 共享协调点：pack `1.307.0` / content hash
  `7d1d1a197247782f4c3466e4ff0465e0afdf10ff964ac564e95d7f206a0d23f3`；Protocol `1.189`、
  State Hash Schema v94、save v1、active replay baseline `contract-v285` 均不变。

## 31. monsters-next 当前交接（P69 神系内容）

- 导入女猎手斯卡蒂、彩虹桥守卫海姆达尔、索尔之子曼尼、象头神伽内什和三重火神阿耆尼；
  `NORSE`、`HINDU` 分别落为 `norse`、`hindu` 内容标签，不新增神系启停状态。
- 海姆达尔的 `S_PANTHEON` 复用现有 `SummonCategory`，召唤 1d2 个不高于 77 级的
  `norse` 唯一怪物；唯一生命周期继续由现有统一额度检查约束。
- L71–80 审计现为 72 selected、0 direct、4 blocked、7 excluded；剩余 blocker 仅为 Banor 三体
  的 `SPECIAL` 和 Aegir 的 `S_SPECIAL`。
- 共享协调点：pack `1.308.0` / content hash
  `255d99b948f7200e2ec03ad78881a7d86d72ae6507eb616614e2b5a71ba210c3`；Protocol `1.189`、
  State Hash Schema v94、save v1、active replay baseline `contract-v285` 均不变。

## 32. monsters-next 当前交接（P70 埃吉尔召唤链）

- 导入海巨人与海巨人神王埃吉尔；海巨人保持 `WILD_OCEAN` 对应的真正 ocean-only 分配，
  埃吉尔保留 `norse`、`unique` 与 Orc Cave 分配。
- 埃吉尔的 `S_SPECIAL` 复用现有 `SummonCategory` 与整批候选：先掷 `1d4` 数量，再以施法者为
  中心产生权威源码的强度 3、半径 8 水流，随后一次 50/50 掷骰选定整批海巨人或低等克拉肯；
  永久地形和楼层连接不被覆盖。该怪物专属副作用由 ability tag 标记，不新增协议 DTO。
- L71–80 审计现为 73 selected、0 direct、3 blocked、7 excluded；剩余 blocker 仅为 Banor 三体
  的 `SPECIAL`。
- 共享协调点：pack `1.309.0` / content hash
  `7d2f9799ac6fcd8ab71b6de2be52b4f77bcda70594a1bb1f266ff6e4e345a09c`；Protocol `1.189`、
  State Hash Schema v94、save v1、active replay baseline `contract-v285` 均不变。

## 33. monsters-next 当前交接（P71 巴诺／鲁巴特三体）

- 导入巴诺尔＝鲁巴特、摄政王巴诺尔和鲁巴特将军，并实现三体共用的 `SPECIAL` 转换。
- 合体按权威源码拆成两个分体：当前生命各为 `(hp + 1) / 2`、最大生命各为 `maxHp / 2`；
  两个分体同层存活时可在另一分体的位置合并，当前生命与最大生命分别相加。
- 转换通过既有召唤结果投影并执行无死亡移除，不产生掉落、经验或唯一死亡计数；任一形态真实死亡会
  关闭整个三体生命周期，单个幸存分体仍可保存恢复，但不能再合体或重新生成整合体。
- L71–80 审计现为 76 selected、0 direct、0 blocked、7 excluded。
- 共享协调点：pack `1.310.0` / content hash
  `ea60ac6fbe1c44b29cdaf3b7db63c8bbef39ba56b9f08bf1159906741dd13d2c`；Protocol `1.189`、
  State Hash Schema v94、save v1、active replay baseline `contract-v285` 均不变。

## 34. monsters-next 当前交接（P72 地点受限怪物）

- 导入鲸鱼亲王马斯玛格、尘世巨蟒耶梦加得和利维坦，三者保留 `WILD_OCEAN` 对应的
  ocean-only 分配；导入兽王贾姆巴万和瓦纳拉之王波林，保留 `legacyDungeonIndices: [43]`，
  不进入 Orc Cave 或全局地牢分配。
- 波林的 `S_VANARA` 复用既有 `SummonCategory`，召唤 1d3+1 个不高于 76 级的 `vanara`
  候选；既有瓦纳拉与瓦纳拉贤者补充同一分类标签，不增加运行时结构。
- 导入地下室猫与篡位者埃里克，移除随机 allocation 并标记 `fixed-placement`，等待任务地图
  显式放置。L71–80 审计现为 83 imported、76 selected、0 direct、0 blocked、7 excluded；
  excluded 仍表示这些 actor 的地点限制，不是内容缺口。
- 共享协调点：pack `1.311.0` / content hash
  `7a1bf49cd48f39c0c6238ab67c2ba1d6329e17110bd4bf737a1db525190ca59c`；Protocol `1.189`、
  State Hash Schema v94、save v1、active replay baseline `contract-v285` 均不变。
