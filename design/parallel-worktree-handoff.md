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
