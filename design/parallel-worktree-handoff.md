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

## 21. main 当前交接（骑术熟练度 Commit 1）

- 本批不新增 item、ability、material、affix 或其他正式内容 ID。职业内容新增必填的
  `ridingProficiency`；现有四职业值来自 RFB master `s_info.txt`，未来 Cavalry 必须使用
  已审计的 `2000/8000`，不得另设职业默认值。
- `CharacterProgress.ridingProficiency` 是权威当前值，必填进入 save/state hash；面板在
  “杂项熟练度”展示骑术专用等级阈值、当前值和职业上限。其他分支不得把它塞回逐等级重算
  的通用 skill set，也不得套用武器熟练度的 4000/6000/7000/8000 等级边界。
- 骑乘成功、近战成长和弹道碰撞成长均读取该状态。落马成长公式已有正式规则入口，但当前
  main 尚无落马事务；后续骑术 Commit 2 必须在落马 RNG 前调用，不得为了提前消费接口而
  在普通受伤路径伪造增长。
- 共享协调点：pack `1.298.0` / content hash
  `ce5843c05a1a11cd26f24868777b285ba41363e06e7dcbdfac0615df4e6596cb`、Protocol `1.189`、
  State Hash Schema v94、save v1、active baseline `contract-v285`（26 条 exact fixture、
  零 waiver）。

## 22. main 当前交接（骑乘战斗 Commit 2）

- 本批不新增 item、ability、material、affix 或其他正式内容 ID。普通 `Ride` 现在只接受
  已有宠物，野生怪物不抽 RNG、不改变控制权；未来 Cavalry 的“套马”必须另建正式能力，
  不得重新放宽普通 `Ride`。
- 当前包已有的 15 件 RFB `OF_RIDING` 武器（含固定神器 Pain）通过 `ridingWeaponKind`
  登记；Lance 使用
  `lance` 语义获得骑乘 `+15` 命中与 `+2` 伤害骰。后续 items 分支导入新的 RIDING 武器
  时只补该字段，不创建 class 方向的重复物品。
- 职业字段 `ridingCombatExpert` 与 `mountedNonArrowBaseShotCap` 已为 Cavalry/Beastmaster
  留出内容接口。未来 Cavalry 必须声明 `true` 与 `100`；核心不得硬编码
  `demo.class.cavalry`。普通骑手和专家的近战/弹药命中分支、坐骑受控速度均已实现。
- `resolveRidingFall`（Rust `resolve_riding_fall`）是唯一落马事务：非强制分支先增长骑术，
  再用旧 current 做两阶段 RNG；强制分支直接进入八方向蓄水池抽样。无落点保持骑乘并
  结算撞墙伤害；有落点解除骑乘、结算坠落伤害并迁移玩家。怪物近战/能力受伤、坐骑死亡、
  坐骑变形成不适合骑乘的形态复用该入口；坐骑死亡/删除与召唤到期按 master 保持既有
  清理路径，只解除骑乘，不额外结算坠落。
- 尚未实现且不得在 Cavalry 内容中伪造：双手持骑乘落马修正、浮空安全着陆、骑乘羁绊、
  套马驯服资格/失败落马。这些要在对应共享状态或职业能力批次按 master 单独闭合。
- 共享协调点：pack `1.299.0` / content hash
  `0075e65b38104d4103be9a4de0b798b4f404fb37857ca5359f949021b8401868`、Protocol `1.189`、
  State Hash Schema v94、save v1、active baseline `contract-v286`（26 条 exact fixture、
  零 waiver）。

## 23. main 当前交接（骑兵正式内容 Commit 3）

- 新增且由 class 方向拥有：`demo.class.cavalry`、`demo.skill-set.cavalry`、
  `demo.build.cavalry`、`demo.actor.cavalry-player`、`demo.ability.cavalry-rodeo` 与
  `demo.ability-program.cavalry-rodeo`。没有新增 item、material 或 affix ID；出生只引用
  既有阔头矛、皮鳞甲、短弓和箭，items 分支不得重复导入同义物品。
- Cavalry 属性、生命/经验、八项技能、宠物维持除数、骑术 `2000/8000` 与逐武器熟练度
  均来自 RFB `master`。`audit-demo-weapon-proficiencies` 现核验 5 个正式职业和当前 67 种
  基础武器。
- 普通 Ride 继续只接受宠物；套马使用独立 ability effect，先强制上马，再按角色等级、
  骑术、Unique 等级修正及原版短路 RNG 判定驯服。`guardian` / `questor` 保留不可驯服
  tag 入口，失败进入共享强制落马事务；核心仍不按 class ID 分支。
- 新游戏、职业能力方向选择、骑术面板和三套 tileset actor 映射已闭合。中文职业/能力名
  与说明取自 `master:src/cavalry.c`。
- 共享协调点：pack `1.300.0` / content hash
  `bb912c0d2adef96f8930f190e588f6a1a59a94b9df9b70ce59d6634913a4f2d9`、Protocol `1.190`、
  State Hash Schema v94、save v1、active baseline `contract-v287`（26 条 exact fixture、
  零 waiver）。本批未导入 arena/battle 世界状态、浮空安全着陆、骑乘羁绊或双手持落马
  修正；后续应在对应共享系统落地时接入，不在职业内容中造占位状态。

## 24. main 当前交接（重型骑枪 Commit 1）

- 新增并由 class 方向拥有 `demo.item.heavy-lance`；items 分支不得重复导入同义物品。
  固定来源为 `master:k_info.txt` source index 107 / tval 22 / sval 29，中文名严格使用
  `重型骑枪`。本批没有新增 ability、material、affix 或神器 ID。
- 内容保留等级 43、`A:43/4`、40.0 磅、价值 700、`4d10`，并通过既有
  `ridingWeaponKind: lance` 复用骑乘 `+15` 命中与 `+2` 伤害骰规则；基础物品表使用
  depth 43 / weight 25。`master:s_info.txt` 的 Cavalry `W:3:29:1:4` 要求重型骑枪与普通
  Lance 一样使用 `4000/8000` 显式熟练度；其余四职业使用各自原版默认值。逐武器审计现
  核验 5 个正式职业和 68 种基础武器。
- 当前协调点：pack `1.301.0` / content hash
  `b7e20a37795ab1381bb3f3d6e8e9d991215ee04a7ad0bd1b83cde948c1b2b08a`、Protocol `1.190`、
  State Hash Schema v94、save v1、active baseline `contract-v287`。本批只增加内容，不改变
  协议、权威状态或共享初始化；现有 exact fixtures 不需要刷新。

## 25. main 当前交接（坐骑经验、进化与骑乘羁绊 Commit 2）

- 本批不新增 item、ability、material 或 affix ID。坐骑用药复用现有 7 个正式药水 ID；
  items 分支只需维护这些物品的普通生成与效果，不得另建“坐骑药水”同义物品。
- 新内容字段 `ActorDefinition.evolution` 由 class 方向的 RFB r_info 导入器维护。master 的
  348 条进化关系中，320 条本地两端均存在；5 组同名重复原版记录折叠后生成 315 个稳定
  actor 定义。13 个缺失目标（其中 7 条可骑乘链）保留为后续 monsters 分支交接项，未创建
  占位 actor。七条可骑乘缺口为 Chaos/Law/Balance drake、Death drake、Ancient
  multi-hued dragon、Fastitocalon、Unicorn，目标分别是对应 Great Wyrm、Spectral Wyrm、
  Great Wyrm of Many Colours、Leviathan 与 Kirin；其余目标为 Master rogue、Old sorcerer、
  Ultimate beholder、Warp demon、Greater Balrog、Grand Fearlord。
- 权威状态为 `Actor.experience` 与玩家单一 `RidingBond(actorId, actorKindId, value)`。
  普通下马和同一实体楼层往返保留；更换坐骑、死亡、解散、失控或进化重置。进化保留实体
  ID、控制和骑乘，按生命比例换形，并为新 kind 建立 0 羁绊。
- 宠物击杀、玩家满羁绊击杀、2500/5000/10000 门槛、治疗与速度药水 RNG 均按 master
  接入。物品使用继续复用 `UseItem` + entity target，前端没有新增命令协议或待处理状态。
- 当前协调点：pack `1.302.0` / content hash
  `1dcf89e57968a66dcfce99ba036ad077012e8dcbea8e8a0697aca4756d4b9f70`、Protocol `1.191`、
  State Hash Schema v95、save v1、active baseline `contract-v289`（26 条 exact fixture、
  零 waiver）。旧开发存档不兼容。

## 26. main 当前交接（捕获球与最终闭环 Commit 3）

- 新增且由 class 方向拥有 `demo.item.capture-ball`；items 分支不得重复导入同义物品。本批
  没有新增 ability、material、affix 或 actor ID。固定来源是 RFB `master:k_info.txt`
  source index 704，正式数据为等级 15、`A:15/4`、12.0 磅、价值 1000、不可堆叠和 shield
  槽；先前计划中的 2.0 磅是笔误。
- `ActorDefinition.capturePolicy` 由 monster 导入器按 RFB flags 生成。`normal` 为兼容默认，
  Unique/Nazgûl 使用 `pet-only`，`UNIQUE2`、questor 及三个特殊合体怪使用 `immune`。后续
  monsters 分支导入新怪时必须同步该字段；运行时不能靠显示名或原版数字序号判断，来源
  序号到策略的映射只留在导入审计层。
- 球内权威状态只保存 kind、speed、hp/maxHp 与 experience；不保存旧实体 ID、临时状态、
  summon 或 pack。正常释放生成新宠物；捕获当前坐骑强制落马并重置羁绊。丢弃/投掷按
  `1/4` 敌对骰释放，显式或环境摧毁保持宠物阵营。
- Unique 生成资格通过遍历 active/stored/shop/home 中所有物品实例派生；其他方向不得增加
  第二份“球内唯一怪物”计数或存档字段。
- 当前协调点：pack `1.303.0` / content hash
  `538cce0f525d1530dbb109f4cf75074c69130b09eebca10d672628ad770467e5`、Protocol `1.192`、
  State Hash Schema v96、save v1、active baseline `contract-v290`（26 条 exact fixture、
  零 waiver）。旧开发存档不兼容。

## 27. items-next 奥秘前两册

- `codex/items-next` 新增 `high-mage-arcane` 构筑、《初学者戏法》和《小阿卡纳》共十六个
  法术；Arcane 出生仍只携带第一册，死亡构筑、High Mage 通用出生装备及白马旅店奖励
  保持不变。
- 第二册复用正式物品、商店与基础池路径；运行时只增加穿墙探测、已装备光源补充燃料和
  按当前值比例削减状态三个窄表面。寒冷/火焰抗性继续使用既有独立状态和 spell power。
- 分支协调点为 pack `1.299.0`、Protocol `1.190`、State Hash Schema v93、save v1、
  active baseline `contract-v284`；书店固定库存变化已刷新并复验全部 26 条 active fixture。

## 28. items-next 奥秘第三册

- 新增《大阿卡纳》及抵抗闪电、抵抗酸液、治疗中伤、传送、鉴定、化石为泥、光之射线、
  充饥八个正式法术；Arcane 出生仍只携带第一册，High Mage 通用出生装备、死亡构筑和
  白马旅店奖励保持不变。
- 运行时只增加化石为泥、治疗中伤流血余量与充饥三个窄表面；长距离传送复用既有随机
  传送及 Astral Guide，鉴定和光之射线复用既有实现。
- 分支协调点为 pack `1.300.0`、Protocol `1.191`、State Hash Schema v93、save v1、
  active baseline `contract-v284`。

## 29. items-next 奥秘第四册前置能力

- 注册《大师手册》前七个法术：识破隐形、抵抗毒素、传送楼层、传送离开、充能、探测、
  召回之语；在第八个法术完成前，该书未进入正式获取路径。
- 识破隐形使用独立状态；传送离开按射线路径、power 与原版抵抗规则结算；玩家法力充能
  在成功施法后再支付额外法力，过载失败清空目标充能但不销毁物品。取消目标、基础施法
  失败与额外法力不足均保持原子性和既定 RNG 顺序。
- 分支协调点为 pack `1.301.0`、Protocol `1.192`、State Hash Schema v93、save v1、
  active baseline `contract-v284`；协议投影变更后已刷新并复验全部 26 条 active fixture。

## 30. items-next 奥秘完整领域

- 完成《大师手册》第八个法术“透视”：Knowledge 与 Enlightenment 各增加 1，永久绘制并
  照亮当前层、揭示全部地面物品；仅在没有永久 ESP 时获得 `25 + 1d30` 临时 ESP。
- 《大师手册》正式绑定八个法术并进入基础物品池及 Outpost、Anambar 两家书店；Arcane
  四册严格各八个法术。Arcane 出生仍只携带第一册，死亡构筑、High Mage 通用出生装备和
  白马旅店奖励保持不变。
- 分支协调点为 pack `1.302.0`、Protocol `1.193`、State Hash Schema v93、save v1、
  active baseline `contract-v284`；两家书店固定库存变化已刷新并复验全部 26 条 active fixture。

## 31. main 当前交接（items Arcane 增量整合）

- `codex/items-next` 的完整 Arcane 领域已与 main 的骑兵、坐骑羁绊和捕获球闭环合并；
  `rodeo` 与 Arcane 的 terrain beam、充能、传送和透视能力共用同一协议投影与执行入口。
- 合并后共有 8 本 ability book、308 个 item、820 个 ability、5 个 class 和 6 个 build；
  Arcane 四册共 32 个法术，正式获取路径与 Cavalry 出生/职业路径同时保留。
- 共享协调点：pack `1.304.0` / content hash
  `900d8b206a6bc3e186ccc57559f955ab676840dacf0013ed5d2790fb408d49d9`、Protocol `1.194`、
  State Hash Schema v96、save v1、active baseline `contract-v291`（26 条 exact fixture、
  零 waiver）。`codex/items-next` 分支及工作树继续保留。

## 32. main 当前交接（monsters P61–P72 增量整合）

- 有限生命周期怪物统一使用 `defeatedLimitedActorCounts`：普通 Unique 上限 1，`unique2`
  只限制同时存活，戒灵使用显式总额度 5。当前楼层、stored floors 与捕获球中的怪物共同
  占用额度；非死亡移除不写入死亡表。旧开发存档不兼容缺少该字段的 payload。
- 玩家临时变形复用 `grantedRaceId` 状态及 44 个只供变形使用的 race profile；身体槽随
  形态调整，不合槽装备移入背包或落地。Lord of Change 的近战效果、原版免疫/豁免和
  RNG 顺序均保留，变形 profile 不开放为出生 build。
- P63–P72 完成 L64–80 的直接批次、低风险共享映射、Dio 的 WORLD 额外行动、神系标签与
  召唤、Aegir 水流召唤链、Banor/Rupart 分合生命周期以及 ocean/dungeon/fixed-placement
  地点边界。L71–80 审计为 83 imported、76 selected、0 direct、0 blocked、7 excluded；
  excluded 表示地点限制而非内容缺口。
- 与 main 的骑术/捕获球及 items 的 Arcane 领域组合后，捕获球中的有限生命周期怪物会
  正确占用生成额度；坐骑受怪物能力伤害及变形时仍复用共享落马事务；Arcane 与 monster
  ability 的协议表面同时保留。
- 共享协调点：pack `1.312.0`、Protocol `1.195`、State Hash Schema v97、save v1、
  active baseline `contract-v292`（26 条 exact fixture、零 waiver），content hash
  `f66b18b842e434ef84787e664a4fe94107a27b2f47bbf0a6ddffb087c0c0284b`。
  `codex/monsters-next` 分支及工作树继续保留。

## 33. main 当前交接（狙击手 Commit 1：专注底座）

- 本批不新增正式 item、ability、material、affix、class 或 build ID；正式狙击手内容留给
  后续单职业导入。items/monsters 分支无需预留或重复创建任何身份。
- `ClassDefinition.snipingProfile` 保存原版狙击手当前需要的射击派生与专注公式；
  `ClassAbilityDefinition.minimumConcentration/hitPointCost` 为后续箭术提供通用门槛与生命
  成本。新 `concentrate` 效果仅允许有 sniping profile 的职业使用。
- `sniperConcentration` 与 `probedActorKindIds` 一次性进入 save/state hash；后者在 Commit 3
  的“侦察”能力使用前保持空集合。读档严格拒绝非狙击手状态、超上限专注、重复/未知
  actor kind。
- 普通射击按清零前专注计算 AC、弹药段伤害和暴击；有效射击后清零。其他实际耗时命令
  清零；未知能力、非法目标、资源/生命不足和缺弹等前置拒绝不清零、不增加 RNG。
- 当前协调点：pack `1.312.0` / content hash
  `f66b18b842e434ef84787e664a4fe94107a27b2f47bbf0a6ddffb087c0c0284b`、Protocol `1.196`、
  State Hash Schema v98、save v1、active baseline `contract-v293`（26 条 exact fixture、
  零 waiver）。

## 34. main 当前交接（狙击手 Commit 2：统一特殊射击事务）

- 本批不新增正式 item、ability、material、affix、class 或 build ID，也没有需要其他内容
  分支预留的身份。正式狙击手及其 16 个职业能力仍由 class 方向后续导入。
- `sniper-shot` 能力效果支持闪耀、撤退、除陷、燃烧、碎岩、冰冻、击退和穿透八种模式，
  全部复用普通射击的发射器/弹药选择、能量、重弓、逐武器与骑术成长、词缀/品牌/杀戮、
  暴击、死亡、掉落和破损事务。能力投影的射程与能量来自当前发射器。
- 原版射击撤退距离是 `10 + 2 * concentration`，并非固定 10；穿透射击的 N 层专注最多
  结算 N+1 个碰撞目标。`Projectile` 地形变更来源明确不触发挖矿成长、材料、金币或额外
  物品。
- Easy Tiring II 已接入普通与特殊射击；魔法等其他原版消费者仍属于共享 mutation 后续，
  不在狙击手代码内预造。
- 当前协调点：pack `1.312.0` / content hash
  `f66b18b842e434ef84787e664a4fe94107a27b2f47bbf0a6ddffb087c0c0284b`、Protocol `1.197`、
  State Hash Schema v98、save v1、active baseline `contract-v294`（26 条 exact fixture、
  零 waiver）。

## 35. main 当前交接（狙击手 Commit 3：高级射击与探测怪物）

- 本批不新增正式 item、ability、material、affix、class 或 build ID；正式狙击手职业内容
  仍由 class 方向后续一次性导入，items/monsters 分支不应重复创建任何同义身份。
- `sniper-shot` 新增邪恶、神圣、爆炸、双重、雷霆、针刺与圣星之箭七种模式。特殊倍率
  与弹药既有品牌/杀戮倍率取最大值后再乘专注增伤；破损率、爆炸物理范围、双发实例顺序、
  针刺嵌套 RNG/Unique 免疫及最终技后坐力均按 `master:src/sniper.c`、`src/cmd2.c` 接入
  唯一 projectile resolver。
- `probe-monsters` 使用自目标 ability program，只投影当前可见、非模糊且有投射视线的
  怪物实例，不按种类合并。typed outcome 包含生命、速度、AC、阵营、抗性、状态免疫、
  近战和施法能力；`probedActorKindIds` 复用 Commit 1 的 save/hash 字段。Web 端收到结果
  后打开双栏浏览面板。
- 共享协调点：pack `1.312.0` / content hash
  `f66b18b842e434ef84787e664a4fe94107a27b2f47bbf0a6ddffb087c0c0284b`、Protocol `1.198`、
  State Hash Schema v98、save v1、active baseline `contract-v295`（26 条 exact fixture、
  零 waiver）。现有 fixture 不进入尚未正式绑定的狙击手路径，验证后不刷新 assertions。

## 36. main 当前交接（狙击手 Commit 4：正式职业内容与 UI）

- class 方向正式拥有 `demo.class.sniper`、`demo.skill-set.sniper`、
  `demo.build.sniper`、`demo.actor.sniper-player`，以及 17 对
  `demo.ability.sniper-*` / `demo.ability-program.sniper-*`。其他方向不得重复创建同义 ID。
- 本批未新增 item、affix、resource、material 或 ability book ID；出生只引用现有
  `demo.item.dagger`、`demo.item.soft-leather-armour`、`demo.item.light-crossbow` 与
  `demo.item.bolt`。
- RFB master 职业数据、逐武器熟练度 N:27、0/0 骑术、20–30 随机弩栓和十七项能力参数
  已闭合。逐武器审计现核验 6 个正式职业和 68 种基础武器。
- New Game、双语职业/能力文案、十六项“狙击”分组、探测怪物独立入口和三套 tileset
  映射已接通。`CLASS_SENSE1_SLOW | CLASS_SENSE1_STRONG` 等待通用装备感知系统。
- 共享协调点：pack `1.313.0` / content hash
  `8b89d37d689db0c180feb1dbe213a3aa30aef910bd72a12a6c3d1af8222296dc`、Protocol
  `1.198`、State Hash Schema v98、save v1、active baseline `contract-v296`。内容变化
  不改变既有构筑初始化或 state-hash 输入，26 条 active fixture 只复验、不刷新。
