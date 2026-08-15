# 并行 worktree 通用交接说明

更新时间：2026-08-15
当前工作批次起点：`main@638bab21c`

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

## 37. main 当前交接（Human Commit 1：静态资料收口）

- 本批不新增 item、ability、material、affix、race、class 或 build ID；正式 Human 继续
  使用 `demo.race.rfb-human` 与 `demo.skill-set.human`，其他内容方向无需预留身份。
- Human 对齐 `master:src/races_a.c` 的静态值：六项属性修正 0、生命/经验/商店倍率
  100%、基础 HP 20、察觉基础值 +10。Standard 身体、个人主义美德和 Human kin 保持
  既有通用实现；七个正式 build 均继续组合该 Human。
- 双语说明记录 20 级特殊天赋与 35 级人类弱点，但这两个等级机制仍待后续批次实现；
  本批不创建占位能力或持久状态。
- 共享协调点：pack `1.314.0` / content hash
  `8e2f9f3df586082903fbebf797fc86ae7096e1a6b3ba6141831cc827a29b415d`、Protocol
  `1.198`、State Hash Schema v98、save v1、active baseline `contract-v297`。共同初始化
  的 HP 与察觉变化要求全量刷新并复验 26 条 active fixture，零 waiver。

## 38. main 当前交接（Human Commit 2：通用种族等级变异奖励）

- 本批不新增正式 race、mutation、ability、item、material、affix、class 或 build ID；
  `demo.race.rfb-human` 的 `levelMutationRewards` 仍为空。测试专用 race/build/reward 身份只在
  Core 测试内构造，不属于内容 ID 所有权。
- `RaceDefinition.levelMutationRewards` 只提供 `choice` 与 `casting-attribute` 两种选择；
  内容校验要求奖励 ID 唯一、等级大于 0、候选存在且不重复，并要求所有奖励变异的
  `randomWeight` 为 0。未来 Human 20/35 级内容必须复用这套模型并在导入前声明正式变异 ID。
- 待选择状态由当前等级、种族配置和 `lockedMutationIds` 派生，不保存第二份完成标记。
  自动奖励立即获得并锁定；手动选择不消耗行动、资源或 RNG，待选择期间其他命令由核心
  拒绝。降级不移除奖励，重新升级不重复结算。
- Web 变异面板顶部新增强制选择卡；候选复用现有变异名称、说明和评级。正式 Human
  候选池及六种 35 级弱点行为继续留给后续提交，本批不会提前暴露空菜单或占位效果。
- 共享协调点：pack `1.314.0` / content hash
  `8e2f9f3df586082903fbebf797fc86ae7096e1a6b3ba6141831cc827a29b415d`、Protocol
  `1.199`、State Hash Schema v98、save v1、active baseline `contract-v298`。正式内容未
  使用新字段，26 条 active fixture 只复验、不刷新 assertions，零 waiver。

## 39. main 当前交接（Human Commit 3：六种弱点与 35 级奖励）

- 本批不新增 item、ability、material、affix、race、class、build 或 mutation ID；正式
  Human 复用既有 `rfb.mutation.human-str` 至 `rfb.mutation.human-chr` 六项稳定身份，
  其他方向不得创建同义弱点。
- `demo.race.rfb-human` 在 35 级按职业施法属性自动获得并锁定弱点：INT 对应 human-int、
  WIS 对应 human-wis，DEX/CON/CHR 映射为未来职业保留，非施法职业默认 human-str。
  当前高阶法师获得 INT、圣骑士获得 WIS，战士、弓箭手、骑兵和狙击手获得 STR。
- 六项行为对齐 `master:src/races_a.c`、`mut_a.c`、`fear.c` 与其战斗调用点：STR 在一次
  近战事务首次暴击后封锁后续暴击并增加 20% 行动能量；INT 只降低恐惧专用检定且不再
  错接生命力吸取；WIS 只屏蔽敌对邪恶怪物的心灵感应；DEX 按原版近战、射击和闪避
  概率施加 50+1d50 的速度 -10；CON 仅在未不适时 1/200 触发 50 回合不适；CHR 应用
  装置 -10、近战 -16、射击 -10、法术失败率 +10、最低失败率 1% 和额外 1/20 强制失手。
- 共享协调点：pack `1.315.0` / content hash
  `a2fc4f79fbd069387d5fbb9a08364221b2cf15c15a609448056cd4fd4b11a4ca`、Protocol
  `1.199`、State Hash Schema v98、save v1、active baseline `contract-v299`。无新增权威
  状态；现有 26 条 fixture 均未触发 35 级奖励，只复验、不刷新 assertions，零 waiver。

## 40. main 当前交接（Human Commit 4：被动型半神天赋）

- 本批不新增 item、ability、actor、race、class、build、mutation、material 或 affix ID；
  复用既有不屈不挠、狂饮药水、神圣活力、恐怖巫术、武器多面手、恶魔契约与恶魔之握
  七项稳定 mutation 身份。其他方向不得创建同义内容。
- 七项行为已接入现有 HP 派生、物品能量、共享玩家治疗、法术强度、逐武器熟练度、actor
  死亡和怪物装置充能吸取事务。账本为 132 active / 20 blocked，随机候选仍为 104。
- Human 20 级原版 `mut_demigod_pred` 尚包含九项缺真实消费者的候选，因此本批不配置
  `human-talent` 奖励，不把当前已支持子集冒充完整原版候选池。
- 共享协调点：pack `1.316.0` / content hash
  `e0e6820f83989f8790a455d6af5656ec833341e03ef84b2005652c898c47e5b3`、Protocol
  `1.199`、State Hash Schema v98、save v1、active baseline `contract-v300`。无新增权威
  状态；26 条 fixture 均未持有这些变异，只复验、不刷新 assertions，零 waiver。

## 41. main 当前交接（Human Commit 5：主动与跨系统半神天赋）

- 本批不新增 item、actor、mutation、material 或 affix ID；新增并由 main 拥有
  `rfb.ability.mutation.peerless-tracker`、`rfb.ability-program.mutation.peerless-tracker`、
  `rfb.ability.mutation.fantastic-frenzy` 与
  `rfb.ability-program.mutation.fantastic-frenzy`。其他方向不得创建同义能力。
- 隐秘施法、无双狙击手、闪避、个人崇拜、无双追踪者与奇妙狂乱已按 RFB master 闭合。
  Human 20 级菜单只列出 `mut_demigod_pred` 中当前已闭合的 20 项；仍缺真实消费者的
  ambidextrous、speed-reader、black-marketeer、tread-softly、inspired-smithing、
  strong-mind 与 astral-guide 继续隐藏。
- 怪物 actor 新增权威 `anger` 与 `friendly`；前者提高施法频率并在施法后清零，后者只表示
  个人崇拜产生的运行时友好，宠物仍沿用 `controllerId`。save v1 不兼容缺字段的旧开发存档。
- 共享协调点：pack `1.317.0` / content hash
  `b27d385635fe09ef107ca2dd4e7fe6475d58e7e3320893e899246920779f5cb2`、Protocol
  `1.200`、State Hash Schema v99、active baseline `contract-v301`。
- 按用户要求，本批未运行全量测试或 fixture replay；只完成新增测试、生成/内容校验、变异
  审计和 Web 类型检查。因 actor save/hash 结构变化，26 条 active fixture 必须在明确要求
  合并验收时刷新并复验。

## 42. main 当前交接（items 咒术与毁灭增量整合）

- `codex/items-next` 的完整咒术与毁灭领域已与 main 的狙击手、Human 弱点和半神天赋合并；
  双方新增的能力效果、协议 DTO、actor 状态与战斗事务均保留，没有用一侧实现覆盖另一侧。
- 咒术四册与毁灭四册各含 32 个正式法术，并保留各自 High Mage 构筑、原版书籍分配和商店
  获取路径；出生构筑仍只携带本领域第一册。
- 合并后的共享协调点为 pack `1.320.0`、Protocol `1.201`、State Hash Schema v100、
  save v1、active baseline `contract-v302`，content hash
  `9eba1bb25bb87e06d1396b5393f8c080c10b847186e59b79401b11b5e658a537`。装置强度修正、
  怪物 `minorSlow` 与 main 已有的 `anger` / `friendly` 都是必填权威状态；不兼容缺字段的
  旧开发存档。
- `codex/items-next` 分支及工作树继续保留。内容 hash 与 26 条 active fixture 在本次合并
  验收中统一重新生成并复验。
## 33. monsters-next 当前交接（P73 L81–90 直接导入）

- P73A 导入 L81–85 的 19 个普通分配 actor，P73B 导入 L86–90 的 24 个普通分配 actor；
  全部保留 RFB master 的 source index、层级、权威中文名和 Orc Cave 分配。
- 本批新增 97 个由现有运行时表达的参数化 ability/program，并补齐 Sky Drake 到 Great Wyrm
  of Power 的权威进化链；没有新增 DTO、协议、存档或 state-hash 字段，不刷新 replay fixture。
- 怪物同步器的进化关联改为使用权威 source index，允许普通版和唯一版 Metal Babble 这类
  同名记录共存，且不会把进化目标错误绑定到同名 actor。
- 共享协调点：pack `1.313.0` / content hash
  `d8486a55c802382012ad85738f199162cd0bed95bb824b335c89c9fdb42911ea`；Protocol `1.195`、
  State Hash Schema v97、save v1、active baseline `contract-v292`（26 条 exact fixture、
  零 waiver）均不变。

## 34. monsters-next 当前交接（P74 L91–127 直接导入）

- P74A 导入 L91–95 的 11 个普通分配 actor，P74B 导入 L96–127 的 8 个普通分配 actor；
  全部保留 RFB master 的 source index、层级、权威中文名和 Orc Cave 分配。
- 本批只新增 42 个由现有运行时表达的参数化 ability/program；没有新增 DTO、协议、存档或
  state-hash 字段，也不刷新 replay fixture。
- L81–127 联合审计为 62 selected、0 direct、54 blocked、6 excluded；blocked 留给后续机制
  批次，excluded 仍表示地点限制。
- 共享协调点：pack `1.314.0` / content hash
  `3f329afbe222276e0f6d2b9c7861d5f065af4ec8633774bab3cbc6b1f2490818`；Protocol `1.195`、
  State Hash Schema v97、save v1、active baseline `contract-v292`（26 条 exact fixture、
  零 waiver）均不变。

## 35. monsters-next 当前交接（P75A 分类召唤）

- 新增 `S_CYBER`、`S_CAT` 与按施术者所属神系选择候选的 `S_PANTHEON` 映射；补齐 cyber、
  Egyptian、Olympian 与 `no-summon` 内容标签。玩家与怪物分类召唤共用 `no-summon`
  候选过滤，原版 `NO_SUMMON` 怪物仍可自然生成，但不会响应召唤。
- 实际安全解锁 14 个 actor：7 个 Cyberdemon 召唤者、Freyja 与 Durga、Hathor、Frigg、
  Indra、Amun，以及 a Plain Gold Ring；新增 61 个参数化 ability/program。原计划预计的
  另外 7 个在首个分类 blocker 消除后暴露出 `S_UNIQUE` 或专属 `S_SPECIAL`，继续留在后续批次，
  不以 omission 降级导入。
- L81–127 联合审计为 76 selected、0 direct、40 blocked、6 excluded；没有新增 DTO、协议、
  存档或 state-hash 字段，也不刷新 replay fixture。
- 共享协调点：pack `1.315.0` / content hash
  `3eca5a0bf28bde9471cd9f4f58bcf4f066d592a920e4d3ab7a08852dab10b74c`；Protocol `1.195`、
  State Hash Schema v97、save v1、active baseline `contract-v292`（26 条 exact fixture、
  零 waiver）均不变。

## 36. monsters-next 当前交接（P75B 固定召唤与低风险战斗映射）

- 新增 `BR_HOLY_FIRE` 与 `JMP_SHARDS` 映射，分别复用现有吐息和 JumpDamage：信使拉斐尔
  使用 17% 当前生命、上限 250、半径 3 的神圣之火吐息；战斗越前使用等级 85 的碎片跳跃。
- 7 个 `S_SPECIAL` 均通过现有分类召唤 DTO 的单一 `batchCandidates` 精确绑定原版对象：
  伐楼拿召唤 `1d2+2` 摩伽罗，德墨忒尔召唤 `1d4` 树人，贾斯特肖恩召唤 `1d4` 绵羊，
  波塞冬召唤 `1d4` 高等克拉肯，塔洛斯召唤 `1d3` 法术扭曲自动机，梵天与辩才天女
  互相召唤 1 个对方。伐楼拿与波塞冬复用 P70 的 `monster-water-flow` 前置地形效果。
- 本批实际导入 9 个 actor，新增 32 个参数化 ability/program；L81–127 联合审计为
  85 selected、0 direct、31 blocked、6 excluded。没有新增 DTO、协议、存档或 state-hash
  字段，也不刷新 replay fixture。
- 共享协调点：pack `1.316.0` / content hash
  `19bdab0f1a5b6b61246636ee8716198734ffe8b18924cea403acac64a97b1c27`；Protocol `1.195`、
  State Hash Schema v97、save v1、active baseline `contract-v292`（26 条 exact fixture、
  零 waiver）均不变。

## 37. monsters-next 当前交接（P76 复杂召唤与战斗机制）

- P76 导入 24 个 L83–100 actor：`S_UNIQUE` 使用施术者等级向下 40 级的唯一怪物窗口；
  奥林匹斯与印度神系的 12 条 `S_SPECIAL` 保留原版家庭成员、分支概率、唯一额度及 Hades
  熔岩流语义。家庭召唤只在候选仍可生成且附近有合法位置时施放。
- Vayu 的 `BR_AIR` 保留无视抗性、飞行减伤和声音抗性眩晕，Aijem 的 `CHICKEN` 保留固定
  伤害、眩晕与恐惧；同步补齐其后续暴露的 `NO_AIR` 窒息状态。Kaschei 的 `UNLIFE`、
  Unicorn of Order 的 TIME/STUN 接触效果和 Ares 的 STUN 使用显式接触效果，不伪造生命伤害。
- 本批新增 24 个 actor、91 个参数化 ability/program；L81–127 联合审计为 109 selected、
  3 direct、4 blocked、6 excluded。剩余 direct/blocked 不属于 P76：Loki、Shuma-Gorath、
  Pandemonium，以及 Hermes、Zeus、Odin、Zombified Serpent of Chaos。
- 共享协调点：pack `1.317.0` / content hash
  `b40fc1696472d37277ec6630dfd26135b0e92244261989366cf670d9fbe7c1c8`；Protocol `1.195`、
  State Hash Schema v97、save v1、active baseline `contract-v292`（26 条 exact fixture、
  零 waiver）均不变，不刷新 replay fixture。

## 38. monsters-next 当前交接（P77 地点与固定身份怪物）

- P77 导入哥斯拉与高等赛博巨龙天使恶魔巫妖两个荒野怪物，以及巫师索伦、安珀之王
  奥伯龙、混沌之蛇和复活机器四个只供地图显式放置的固定身份怪物。哥斯拉保持真正的
  ocean-only 分配；高等赛博巨龙天使恶魔巫妖的 `WILD_ALL` 优先于附带的 `WILD_OCEAN`，
  可在全部荒野地形生成，并完整保留原版 94 项施法表。
- 混沌之蛇补齐 GUARDIAN 分类、混沌/劣化接触光环与 `S_GUARDIAN`；候选严格限制在
  100 级以下的 guardian。复活机器的 `S_DEAD_UNIQ` 每次掷 `1d2`，每个实例先执行半径 5
  的崩解清场，再从已死亡且当前不存在的普通 Unique 中按原版权重复活；无候选或 1/13
  回退到星际之刃。复活实例保存其 summon 身份，再次死亡不会重复消耗唯一生命周期额度。
- 本批新增 6 个 actor、50 个参数化 ability/program；L81–127 联合审计为 109 selected、
  4 direct、3 blocked、6 excluded。六个 P77 actor 因地点限制均显示 excluded，但已经导入并
  由 selection 管理；剩余 direct/blocked 为后续非 P77 内容。
- 共享协调点：pack `1.318.0` / content hash
  `600467a34cfc7157f16a490db22ba5db32befa077c5d40aa7a783956f1d3b254`；Protocol `1.195`、
  State Hash Schema v97、save v1、active baseline `contract-v292`（26 条 exact fixture、
  零 waiver）均不变，不刷新 replay fixture。

## 39. monsters-next 当前交接（P78 直接怪物导入）

- P78 将 18 条原版来源记录纳入 selection：低阶的沼泽鼠、瘟疫武僧、斯卡文刺客、黏土魔像、
  魔法蘑菇丛、鼠巨魔、大师级盗贼、木乃伊人类、武士、黑骑士、陷阱大师，以及高阶的秃鹫之母
  涅赫贝特、拉之声托特、欺诈者洛基、舒玛-哥拉斯、群魔殿和僵尸化混沌之蛇。
- 狗头人领主穆加什已由 `demo.actor.warrens-keeper` 表达并占用原版 source index 135；本批把该
  现有 actor 登记进 importer selection，不创建重复身份。其余 17 个 actor 为新增内容，并生成
  42 个仅参数不同的既有 ability/program；未扩张运行时、协议、存档或状态哈希结构。
- L1–127 联合审计为 1213 selected、32 direct、5 blocked、113 excluded；P78 的 18 条来源均为
  selected。剩余项属于后续批次，不是本批回归。
- 共享协调点：pack `1.319.0` / content hash
  `74ea1074f2225f0e4798564df735747c49a4b0954c297ad0ac9fc0ef3a93d6cd`；Protocol `1.195`、
  State Hash Schema v97、save v1、active baseline `contract-v292`（26 条 exact fixture、
  零 waiver）均不变，不刷新 replay fixture。

## 40. monsters-next 当前交接（P79 北欧与奥林匹斯召唤者）

- P79 导入英灵狂战士、信使神赫尔墨斯、奥林匹斯之王宙斯和众神之父奥丁。英灵狂战士
  保留 `legacyDungeonIndices: [39]` 与 `asgard` 标签，不进入 Orc Cave 全局分配；其余三只
  保留原版神系标签和分配。
- `S_SPECIAL` 延续既有参数化分类召唤：宙斯召唤 `1d4` 蹒跚怪，赫尔墨斯召唤 `1d16`
  魔法蘑菇丛；奥丁先掷 `1d4` 并封顶为一只，再等权选择英灵狂战士或女武神。为表达赫尔墨斯
  的原版数量，分类召唤内容校验上限从 8 放宽到 16；DTO、协议与执行结构不变。
- 同步补齐 P78 的稳定身份声明：source 135 的原版 ID 是 `mughash-the-kobold-lord`，内容 ID
  继续为既有 `warrens-keeper`，由 selection 的 `sourceId` 明确关联，不复制 actor。
- L1–127 联合审计为 1216 selected、32 direct、2 blocked、113 excluded。赫尔墨斯、宙斯和奥丁
  均为 selected；英灵狂战士因 Asgard 地点限制保持预期的 excluded，但已由 selection 管理。
- 本批新增 4 个 actor、10 个参数化 ability/program。共享协调点：pack `1.320.0` / content
  hash `a04b31e159f7c73a8f0a607e85cdeac4f44fd5795c087b548f2ea1359521bc01`；Protocol
  `1.195`、State Hash Schema v97、save v1、active baseline `contract-v292`（26 条 exact
  fixture、零 waiver）均不变，不刷新 replay fixture。

## 41. monsters-next 当前交接（P80 变体维护者）

- P80 导入变体维护者，保留原版 14 级、Unique、33% 施法频率、两次混乱打击、随机移动、
  飞行、掉落和 Orc Cave 分配；普通 `POLYMORPH` 继续复用既有目标变形能力。
- `S_SOFTWARE_BUG` 使用既有分类召唤 DTO，固定候选为软件漏洞，数量为原版 `1d3+1`；
  变体维护者死亡时在半径 2 的合法位置额外尝试生成 4 只软件漏洞。该死亡分支复用现有
  actor 生成、召唤身份和到期机制，不增加内容 DTO、协议、存档或 state-hash 字段。
- L1–127 联合审计为 1217 selected、32 direct、1 blocked、113 excluded；唯一 remaining
  blocker 是 3 级邋遢霍比特人的正式主题掉落表，与 P80 无关。
- 本批新增 1 个 actor、1 个 ability/program。共享协调点：pack `1.321.0` / content hash
  `bd1900b263d9419d05285c268ec9da0ee56917af81b2d7d8815179152b8ec2af`；Protocol
  `1.195`、State Hash Schema v97、save v1、active baseline `contract-v292` 均不变；现有
  replay fixture 不包含变体维护者，因此不刷新 fixture。

## 43. main 当前交接（monsters P73–P85 增量整合）

- `codex/monsters-next` 的 P73–P85 已与 main 的完整四领域 High Mage、狙击手、Human 弱点
  和半神天赋合并；高阶参数化怪物能力、神系/家庭/固定召唤、复活唯一怪物、空气吐息、
  鸡叫、无空气状态与 main/items 的状态和战斗事务同时保留。
- 内容包含 P81 霍比特人掉落主题、P82 海洋与荒野 Unique、P83 地牢名册、Camelot 骑士召唤
  与名册，以及 0 级荒野居民。怪物来源选择账本同时承认 items 已闭合的 `NO_STUN` 和
  monsters 已闭合的 Egyptian 标签，不把已实现标记继续列为 omission。
- 合并后的共享协调点为 pack `1.330.0`、Protocol `1.201`、State Hash Schema v100、
  save v1、active baseline `contract-v303`，content hash
  `f41fcdc2089191ecc635320b4c0b1dd554e1667e39941bc496a9437db7d6665e`。26 条 active fixture
  在本次合并验收中统一重新生成并复验。
- `codex/monsters-next` 与 `codex/items-next` 分支及工作树继续保留。

## 44. main 当前交接（正式种族 Commit 1：独立种族选择底座）

- 本批不新增或占用 race、class、build、item、ability、material 或 affix ID；新游戏仍只
  开放现有 `demo.race.rfb-human`，下一批正式化半兽人时复用
  `rfb-legacy.race.half-orc`，不得另建重复种族身份。
- 新游戏 IPC 请求新增必填 `raceId`；构筑继续提供 class、personality 与领域，所选正式
  race 独立写入既有 `CharacterBuildIdentity.raceId`。现有 build JSON 的 Human 继续作为
  未显式选择时的默认值，不生成 build × race 组合内容。
- 角色创建与读档只接受带 `rfb-compatibility` 标签的种族。存档已包含 `raceId`，读档现按
  保存的种族恢复，同时仍要求 class/personality 与 build 定义一致。
- 共享协调点保持 pack `1.330.0`、Protocol `1.201`、State Hash Schema v100、save v1、
  active baseline `contract-v303`。本批没有内容、权威状态或 State Hash 输入变化，不刷新
  既有 fixture；只执行新增种族覆盖/存档测试、桌面初始化测试与 Web 定向测试。

## 45. main 当前交接（正式种族 Commit 2：半兽人）

- 本批不新增内容 ID；正式半兽人直接复用 `rfb-legacy.race.half-orc` 与
  `rfb-legacy.skill-set.race-half-orc`。items、monsters 和其他方向不得另建重复身份。
- 半兽人按 `master:src/races_a.c` 保留 `+2/-1/0/0/+1/-1`、生命 103%、基础 HP 20、
  经验 110%、商店 120%、八项技能与黑暗抗性，并补齐原版 3 格红外视觉。新游戏正式种族
  列表现为 Human 与半兽人；玩家 actor 和 tileset 仍由职业构筑决定。
- 半兽人在 30 级使用与 Human 相同的 20 项已闭环 `mut_demigod_pred` 候选。原版池中仍未
  实现的七项为 `ambidextrous`、`speed-reader`、`black-marketeer`、`tread-softly`、
  `inspired-smithing`、`strong-mind`、`astral-guide`；继续隐藏，不创建占位效果。
- `RaceDefinition.infravision` 与 legacy race importer 现在保留种族红外视觉，当前正式内容
  只为半兽人启用。该值由 race ID 和内容派生，不增加存档或 State Hash 字段。
- 共享协调点：pack `1.331.0` / content hash
  `340bc4e519c8ded18b69e24d9bb39e66f6e38e3e78832590ecc685d45b6c84c5`、Protocol
  `1.201`、State Hash Schema v100、save v1、active baseline `contract-v303`。现有 fixture
  均仍选择 Human，不刷新；只运行新增半兽人内容、成长/存档、importer 与 Web 定向测试。

## 46. main 当前交接（正式种族 Commit 3：新增范围验收）

- 新增验收锁定同一 Warrior 构筑下 Human/半兽人的属性、最大 HP、经验倍率、技能和商店
  价格倍率差异；build 未显式覆盖 race 时继续默认 Human，且初始 state hash/RNG 与显式
  Human 相同。未正式提升的 legacy race 和未知 race 均不能从请求注入。
- 既有核心测试覆盖半兽人黑暗抗性、3 格红外视觉、29 级无奖励、30 级进入天赋选择以及
  选择锁定/save 往返；新增 replay 聚焦测试锁定选择命令及最终 state hash，Web 聚焦测试
  锁定所选 `raceId` 进入 `NewSessionRequest`。
- 本批不新增任何内容 ID，不变更 pack `1.331.0`、Protocol `1.201`、State Hash Schema
  v100、save v1 或 active baseline `contract-v303`。按用户要求不运行、不刷新全量 fixture；
  合并验收时再统一执行。

## 47. main 当前交接（高等精灵 Commit 1：种族天生看破隐形）

- 本批不新增或占用 race、item、ability、material 或 affix ID。高等精灵后续正式内容必须
  复用现有 `rfb-legacy.race.high-elf` 与 `rfb-legacy.skill-set.race-high-elf`。
- `RaceDefinition.seeInvisible` 默认 `false`，为当前有效种族贡献一份看破隐形来源；装备与
  状态来源继续逐份叠加。变形通过既有 `character_definitions()` 自动切换该来源，无新增
  权威状态、协议投影或存档字段。
- legacy importer 现从 race `calc_bonuses` 提取顶层无条件 `p_ptr->see_inv++;`，条件式语句
  不导入并继续保留在 hook gap。`infravision` 已完整落入内容，不再重复登记为 gap。
- 本批不修改现有 race JSON；`seeInvisible` 的默认值在序列化时省略，因此协调点仍为 pack
  `1.331.0`、Protocol `1.201`、State Hash Schema v100、save v1、active baseline
  `contract-v303`，content lock 与既有 fixture 不变。按用户要求只运行新增和直接相关测试。

## 48. main 当前交接（高等精灵 Commit 2：雪地与种族美德）

- 本批不新增或占用 race、item、ability、material 或 affix ID。高等精灵继续复用
  `rfb-legacy.race.high-elf`，并增加通用 `snow-adapted` 标签；legacy polymorph race 同步会
  保留该标签，其他方向不得以 race ID 硬编码雪地豁免。
- 成功移动进入 `snow` 地形时，普通角色以 33% 为基础，超重按 RFB 携重比例追加，骑乘
  固定 40%；附加量只对 `min(120, actionCost)` 计算。飞行、穿墙、Snow habitat 坐骑和
  `snow-adapted` 种族豁免。本地与世界地图共用同一路径，受阻或转为攻击时不加费。
- 初始美德表现包含高等精灵 `Vitality` 与半兽人 `Valour`；这是内容身份派生的初始选择，
  不增加权威状态、协议或 save 字段。
- 协调点为 pack `1.332.0` / content hash
  `de97fefcb8c224d5dc5b989c7d531808d5a26b9acc9bf190a3b114b76ca2fc2d`、Protocol
  `1.201`、State Hash Schema v100、save v1、active baseline `contract-v303`。依用户要求
  不运行、不刷新全量 fixture；合并验收时再统一处理受雪地移动与半兽人初始化影响的用例。

## 49. main 当前交接（高等精灵 Commit 3：正式内容与 UI）

- 本批不新增 race、item、ability、material 或 affix ID；正式种族继续使用
  `rfb-legacy.race.high-elf`，其他方向不得创建重复 High-Elf 内容。
- High-Elf 现具备 RFB master 的完整静态属性、技能、生命/经验/商店倍率、4 格红外视觉、光明
  抗性、天生看破隐形、雪地适应、Standard 身体和既有精灵 kin；无等级奖励或主动种族能力。
- New Game 列表与中英文案已正式开放。职业 build 仍默认 Human，用户选择的 High-Elf 通过既有
  `raceId` 覆盖路径进入 save、replay 与 State Hash，不新增协议或持久化字段。
- 共享协调点为 pack `1.333.0` / content hash
  `6cf1fcef5b2162e3ecc653c91dcc2fb5beae7d95ca89091b660ff7b96e2336fd`、Protocol `1.201`、
  State Hash Schema v100、save v1、active baseline `contract-v303`。按用户要求只执行新增和直接
  相关测试，全量 fixture 留到合并验收。

## 50. main 当前交接（登丹人正式内容与 UI）

- 本批不新增 race、skill set、item、ability、mutation、material 或 affix ID；登丹人继续
  使用 `rfb-legacy.race.dunadan`，其他方向不得创建重复身份。
- RFB master 的静态属性、生命 100%、基础 HP 20、经验 160%、商店 100%、八项技能、Human
  kin、体质维持和 30 级半神天赋已闭合。候选池与 Human 当前 20 项已实现候选完全一致；
  七项未实现原版天赋继续隐藏。
- 种族属性维持由有效种族派生，因此变形会获得或失去；等级奖励改为永久选择种族派生，
  防止临时登丹人形态领取 `dunadan-talent`。New Game 通过既有 `raceId` 正式开放“登丹人”，
  玩家 actor 与 tileset 不增加种族副本。
- 共享协调点为 pack `1.334.0` / content hash
  `902ac3141bab282d663871d70e50a80cb4c9556b347f32eeddf230943b1e7fd3`、Protocol `1.201`、
  State Hash Schema v100、save v1、active baseline `contract-v303`。按用户要求不运行、不刷新
  全量 fixture；合并验收时统一处理。

## 51. main 当前交接（野蛮人 Commit 1：种族能力底座）

- 种族方向已预留并拥有 `rfb.ability.race.berserk`，复用
  `rfb.ability-program.mutation.berserk`；本批不新增 item、material、affix、resource 或
  ability-program ID。items 等并行方向不得重复导入该能力。
- `RaceDefinition.abilities` 与变异能力共享 `InnatePowerDefinition`，能力从当前有效种族派生。
  临时变形随形态获得或失去，不增加持久解锁状态。协议以 `AbilitySourceDto::Race` 区分来源，
  前端继续显示既有“先天 / Innate”标签。
- 原版能力菜单限制和费用路径已统一：种族、职业、变异能力受恐惧与混乱限制；天生能力先扣
  职业 SP、不足部分扣 HP，成功与失败均支付；狂暴时禁止已学法术。共享狂暴程序使用
  `keep-strongest`，较短的新持续时间不会覆盖较长状态。
- `rfb.ability.race.berserk` 已承载原版 `10 + 1d(level)` 与 `level / 5` 近战伤害增长，但尚未
  挂到野蛮人正式种族内容，也未开放 New Game。共享协调点为 pack `1.335.0` / hash
  `bf30576bef5c777b778d085de8e69e240a163a151901c6fa6b91d623fd444f6f`、Protocol `1.202`、
  State Hash Schema v100、save v1、active baseline `contract-v303`；不刷新全量 fixture。

## 52. main 当前交接（野蛮人 Commit 2：正式内容）

- 本批不新增 ID；`rfb-legacy.race.barbarian` 已正式拥有
  `rfb.ability.race.berserk`。其他方向不得另建野蛮人 race、talent 或 berserk ability 身份。
- RFB master 的静态数值、技能、一层恐惧抗性、8 级种族狂暴、30 级
  `barbarian-talent`、Standard 身体、Human kin、polymorph candidate 和初始“勇猛”均已闭合。
  天赋候选复用当前 20 项已实现半神天赋，不引入七项尚未闭环天赋的占位效果。
- importer 只接受字面量 `power_info` 表；`berserk_spell` 映射既有稳定 ability ID，未知能力按
  `race_hook_gaps` 继续审计而不生成占位内容。`RES_FEAR` 已加入静态种族抗性导入。
- 共享协调点为 pack `1.336.0` / content hash
  `fecd2a3598b99e61a66e07c76376e0e6c79da2e1702d6dafa2ce63ca0859cb8f`、Protocol `1.202`、
  State Hash Schema v100、save v1、active baseline `contract-v303`。前端 New Game 入口尚未加入，
  留给 Commit 3；不刷新全量 fixture。

## 53. main 当前交接（野蛮人 Commit 3：新游戏 UI 与验收）

- New Game 已正式开放 `rfb-legacy.race.barbarian` / “野蛮人”，通过既有 `raceId` 选择路径
  初始化。玩家 actor 与 tileset 仍由职业 build 唯一决定，本批不新增外观资源。
- `AbilityDto` 增加可选 `governingAttribute`，使既有 STR 检定数据进入能力面板；野蛮人狂暴
  显示“先天”、8 级、STR、消耗 10 和当前失败率。该字段也适用于已有变异/职业主动能力，不
  复制内容配置。
- 聚焦验收覆盖正式静态档案、恐惧抗性、7/8 级能力边界、SP→HP、失败支付、零预算拒绝、
  `10 + 1d(level)`、`level / 5`、重施不缩短、狂暴禁法术、临时野蛮人形态、30 级天赋、save
  与 replay。依用户要求不运行或刷新全量 fixture，留待合并验收。
- 共享协调点为 pack `1.336.0` / content hash
  `fecd2a3598b99e61a66e07c76376e0e6c79da2e1702d6dafa2ce63ca0859cb8f`、Protocol `1.203`、
  State Hash Schema v100、save v1、active baseline `contract-v303`。

## 54. main 当前交接（霍比特人正式内容、新游戏 UI 与验收）

- 种族方向拥有既有 `rfb-legacy.race.hobbit`，并新增、拥有
  `rfb.ability.race.create-food` 与 `rfb.ability-program.race.create-food`。能力只引用既有
  `demo.item.ration-of-food`；其他方向不得重复建立这些身份。
- 霍比特人已按 RFB master 闭合六维、生命 92%、基础 HP 14、经验 120%、商店 100%、八项
  技能、4 格红外、Hobbit kin、Standard 身体、初始“节制”和 15 级 INT/10/50“制造食物”。
  原版没有 30 级半神奖励或额外抗性，本地也未添加。
- New Game 通过既有 `raceId` 正式开放“霍比特人”；玩家 actor 与 tileset 仍由职业 build
  决定。制造食物复用共享 `create-item` 与 `drop_near` 路径，生成普通、`Acquire` 来源的一份
  口粮；成功、失败、SP→HP、save/replay、拥挤落点与变形切换均由聚焦测试覆盖。
- `packs/rfb-demo-original/lootTables/hobbit.json` 继续只表示怪物掉落主题，不是玩家种族被动，
  不得接入玩家掉落。协调点为 pack `1.337.0` / content hash
  `a0d86072c1d0fdd278b5a0feac9546a79b99c736a0e264f51297bc6675f0c683`、Protocol `1.204`、
  State Hash Schema v100、save v1、active baseline `contract-v303`。依用户要求不运行或刷新全量
  fixture，留待合并验收。

## 55. main 当前交接（狗头人正式内容、新游戏 UI 与验收）

- 种族方向拥有既有 `rfb-legacy.race.kobold`，并新增、拥有
  `rfb.ability.race.poison-dart` 与 `rfb.ability-program.race.poison-dart`。毒镖是无限携带的
  种族能力，不建立 dart/ammo 物品；其他方向不得重复建立这些身份。
- 狗头人已按 RFB master 闭合六维、生命 98%、基础 HP 19、经验 90%、商店 120%、八项技能、
  3 格红外、毒抗、Kobold kin、Standard 身体、初始“荣誉”和 12 级 DEX/8/50“毒镖”。原版
  没有等级奖励；种族描述中的带毒武器偏好不是被动品牌。
- New Game 通过既有 `raceId` 正式开放“狗头人”；玩家 actor 与 tileset 仍由职业 build 决定。
  毒镖复用共享 bolt、反射和抗性链，以 `0d0 + level` 精确表达原版实际施放伤害，不生成或
  消耗弹药。门槛、支付、毒抗、固定伤害、首目标停止、save/replay、变形切换与前端入口均由
  新增或直接相关聚焦测试覆盖。
- 协调点为 pack `1.338.0` / content hash
  `1e4ac419da5c5a8a3c7aea75fc4758d2d87dfeac197bcaed7b8d5ab62064d353`、Protocol `1.204`、
  State Hash Schema v100、save v1、active baseline `contract-v303`。依用户要求不运行或刷新全量
  fixture，留待合并验收。

## 56. main 当前交接（矮人正式内容、新游戏 UI 与验收）

- 种族方向拥有既有 `rfb-legacy.race.dwarf`，并新增、拥有
  `rfb.ability.race.detect-doors-stairs-traps`、`rfb.ability.race.detect-treasure` 与
  `rfb.ability-program.race.detect-treasure`。门/陷阱侦测共享既有
  `demo.ability-program.arcane-detect-doors-traps`；其他方向不得重复建立这些身份。本批不占用
  item、material、affix、resource 或玩家 actor ID。
- 矮人已按 RFB master 闭合六维、生命 103%、基础 HP 22、经验 135%、商店 115%、八项技能、
  5 格红外、失明抗性、Dwarf kin、Standard 身体、初始“勤勉”和两项原版侦测能力；原版没有
  等级奖励。临时形态使用有效种族被动和能力，但等级奖励继续取角色出生种族。
- “侦测门与陷阱”覆盖半径 30 内的陷阱、门和上下楼梯；“侦测宝藏”严格只侦测宝藏矿脉，
  包括隐藏岩浆/石英富矿，不包含散落金币。New Game 通过既有 `raceId` 正式开放“矮人”，
  玩家 actor 与 tileset 继续由职业 build 决定。
- 门槛、能力投影、成功/失败与 SP→HP 支付、隔墙侦测、散落金币排除、save/replay、变形切换
  和 Web 入口均由新增或直接相关聚焦测试覆盖。协调点为 pack `1.339.0` / content hash
  `0a04d780fa04c221a43fba292e9ce33826c93fa1fc02943f8f5928ef52f90e74`、Protocol `1.204`、
  State Hash Schema v100、save v1、active baseline `contract-v303`。依用户要求不运行或刷新全量
  fixture，留待合并验收。

## 57. main 当前交接（尼伯龙人正式内容、新游戏 UI 与验收）

- 种族方向继续拥有既有 `rfb-legacy.race.nibelung` 与
  `rfb-legacy.skill-set.race-nibelung`。尼伯龙人只引用已经由种族方向拥有的
  `rfb.ability.race.detect-doors-stairs-traps` 和 `rfb.ability.race.detect-treasure`；本批不新增
  或占用 ability、program、item、material、affix、resource 或玩家 actor ID。
- 尼伯龙人已按 RFB master 闭合六维、生命 101%、基础 HP 21、经验 150%、商店 115%、八项
  技能、5 格红外、黑暗与解除魔法抗性、Nibelung kin、Standard 身体、初始“耐心”和两项
  10 级 WIS/CHR、消耗 5、失败参数 50 的原版侦测能力；原版没有等级奖励。
- New Game 通过既有 `raceId` 正式开放“尼伯龙人”，玩家 actor 与 tileset 继续由职业 build
  决定。静态内容、能力门槛与投影、种族被动、美德、本地化和 Web 入口均由新增或直接相关
  聚焦测试覆盖。
- 协调点为 pack `1.340.0` / content hash
  `cfbe50b700ca2647b4f8519970012c92e8eb42be4f14f541c2b1741434b3b930`、Protocol `1.204`、
  State Hash Schema v100、save v1、active baseline `contract-v303`。依用户要求不运行或刷新全量
  fixture，留待合并验收。

## 58. main 当前交接（侏儒正式内容、新游戏 UI 与验收）

- 种族方向继续拥有既有 `rfb-legacy.race.gnome` 与 `rfb-legacy.skill-set.race-gnome`，并新增、
  拥有 `rfb.ability.race.phase-door`。该能力共享 `demo.ability-program.arcane-blink`，不得与书本
  法术 `demo.ability.sorcery-phase-door` 合并身份；本批不占用新的 ability program、item、
  material、affix、resource 或玩家 actor ID。
- 侏儒已按 RFB master 闭合六维、生命 95%、基础 HP 16、经验 115%、商店 115%、八项技能、
  4 格红外、麻痹免疫、Gnome kin、Standard 身体、初始“知识”和 5 级 INT/2/50“相位之门”；
  原版没有等级奖励。
- New Game 通过既有 `raceId` 正式开放“侏儒”，玩家 actor 与 tileset 继续由职业 build 决定。
  4/5 级门槛、资源支付、10 格传送、麻痹免疫、美德、种族/书本双能力身份和 Web 入口均由
  新增或直接相关聚焦测试覆盖。
- 协调点为 pack `1.341.0` / content hash
  `7d7e534496653c1a846dd461c73f245e4a9ce36fb3715b80d2976d3c659dbc47`、Protocol `1.204`、
  State Hash Schema v100、save v1、active baseline `contract-v303`。依用户要求不运行或刷新全量
  fixture，留待合并验收。

## 59. main 当前交接（半巨人正式内容、新游戏 UI 与验收）

- 种族方向继续拥有既有 `rfb-legacy.race.half-giant` 与
  `rfb-legacy.skill-set.race-half-giant`，并新增、拥有 `rfb.ability.race.stone-to-mud`。该能力共享
  `demo.ability-program.arcane-stone-to-mud`；其他方向不得重复建立该能力身份。本批不占用新的
  ability program、item、material、affix、resource 或玩家 actor ID。
- 半巨人已按 RFB master 闭合六维、生命 108%、基础 HP 26、经验 150%、商店 125%、八项
  技能、3 格红外、力量维持、碎片抗性、Giant kin、Standard 身体、初始“正义”和 20 级
  STR/10/70“化石为泥”；原版没有等级奖励。
- New Game 通过既有 `raceId` 正式开放“半巨人”，玩家 actor 与 tileset 继续由职业 build 决定。
  化石为泥复用共享 terrain-beam 和 `Magic` 地形变更来源；普通矿脉不会因此增加挖矿熟练度或
  生成材料、金币、物品。既有“魔法摧毁富矿只掉普通金币”的共享规则不变。
- 协调点为 pack `1.342.0` / content hash
  `dea74959256bde3ccda9d728a448a121d50eb525bc4e3fa2b9f419734be4122d`、Protocol `1.204`、State Hash
  Schema v100、save v1、active baseline `contract-v303`。依用户要求不运行或刷新全量 fixture，
  留待合并验收。

## 60. main 当前交接（半巨魔正式内容、种族再生与新游戏入口）

- 种族方向继续拥有既有 `rfb-legacy.race.half-troll` 与
  `rfb-legacy.skill-set.race-half-troll`，并直接引用已有 `rfb.ability.race.berserk`；其他方向不得
  为半巨魔复制狂暴身份。本批不新增 ability、ability program、item、material、affix、resource
  或玩家 actor ID。
- 内容模型新增默认 0 的 `RaceDefinition.regenerationRateModifierPercent`，直接并入已有变异/状态
  再生倍率函数，跟随当前有效种族且不保存重复状态。半巨魔按 RFB master 闭合六维、生命 107%、
  基础 HP 25、经验 150%、商店 135%、八项技能、3 格红外、力量维持、再生 +100%、初始“勇猛”
  和 10 级 STR/12/50“狂暴”；原版没有等级奖励。
- New Game 通过既有 `raceId` 正式开放“半巨魔”，外观继续由职业 build 决定。内容、能力门槛、
  共享狂暴、再生倍率、有效种族切换、美德与 Web 入口由新增或直接相关聚焦测试覆盖。
- 协调点为 pack `1.343.0` / content hash
  `23c6ddf5d8d8080af9fab3cafd85847077cbff7db76715094eaee5b69b2ea0d3`、Protocol `1.204`、State Hash
  Schema v100、save v1、active baseline `contract-v303`。依用户要求不运行或刷新全量 fixture，
  留待合并验收。

## 61. main 当前交接（半泰坦正式内容、共享怪物探测与知识持久化）

- 种族方向继续拥有既有 `rfb-legacy.race.half-titan` 与
  `rfb-legacy.skill-set.race-half-titan`，并新增、拥有 `rfb.ability.race.probe-monsters`。该能力共享
  `demo.ability-program.sniper-probe-monsters` 与现有怪物探测面板；其他方向不得重复建立种族探测
  身份。本批不占用新的 ability program、item、material、affix、resource 或玩家 actor ID。
- 半泰坦按 RFB master 闭合六维、生命 110%、基础 HP 28、经验 200%、商店 90%、八项技能、
  混沌抗性、Giant kin、Standard 身体、初始“和谐”和 15 级 INT/10/60“探测怪物”；原版没有
  等级奖励。New Game 通过既有 `raceId` 正式开放，外观继续由职业 build 决定。
- 已探测 Actor 种类知识不再要求当前职业具有 `snipingProfile`，能力来源消失后仍保留。读档只对
  该集合检查 ID 唯一和 Actor 引用存在；狙击专注的职业与上限校验保持不变。能力门槛、共享面板
  事件、知识保留、非狙击读档、非法知识、美德和 Web 入口由聚焦测试覆盖。
- 协调点为 pack `1.344.0` / content hash
  `7930a9ba2980097431e039479334265842cb54bd143e279efde2c93fd47da96b`、Protocol `1.204`、State Hash
  Schema v100、save v1、active baseline `contract-v303`。依用户要求不运行或刷新全量 fixture，
  留待合并验收。

## 62. main 当前交接（Nature/Life 物品与怪物/地牢增量整合）

- `codex/items-next` 的 7 个增量提交已通过 merge commit `30fccbae6` 接入 main；Nature 全四册与
  Life 前三册复用统一的物品生成、堆叠和落点路径。`codex/monsters-next` 的源 tip 为
  `baf320ff6`，接入 Mirror Shield、Camelot、Tidal Cave、Icky Cave、Hideout/Man cave 互斥选择、
  Troll Cave/Olog 奖励、元素库存损坏和箭袋保护。
- 伤害路径保留 Life Transcendence 的最终玩家伤害提交，再以实际承受伤害触发怪物增量的库存
  损坏；没有建立第二套伤害结算。互斥地牢选择新增必填的 `DungeonStateSaveDto.suppressed`，因此
  存档头与载荷 schema 同步升到 v2；外层二进制容器格式仍为 v1，项目也不兼容旧开发存档。
- 读档验收发现并修正了一处真实验证错误：合法的 `GuardLeader` 怪物群首领曾被
  `monster_packs_are_valid` 拒绝。其余编译/测试问题均为新增必填字段后的手写测试构造或确定性
  断言过期，包括 `PlayerSaveDto.pending_ability_direction`、
  `EquipmentItemDto.permanentDestructionImmunities`、新书店库存和初始地牢选择导致的 RNG 位移。
- 共享协调点为 pack `1.346.0` / content hash
  `ffa579c5d8810be3bd1c8c35220a9bb8fff58ad5f6b96d5f821110e03cf38020`、Protocol `1.211`、
  State Hash Schema v104、save v2。共同初始化 RNG 改变后已按规则刷新并验证 26 个 active fixtures；
  Rust workspace 728 项 core、246 项 content 及其余 crate 测试、Clippy、生成器、source/content
  验证，以及 Web 148 项测试、typecheck 和生产 UI build 均通过。

## 63. main 当前交接（独眼巨人、投掷巨石与岩石伤害）

- 种族方向继续拥有既有 `rfb-legacy.race.cyclops` 与
  `rfb-legacy.skill-set.race-cyclops`，并新增、拥有 `rfb.ability.race.throw-boulder` 和
  `rfb.ability-program.race.throw-boulder`；其他方向不得复制这些身份。本批没有新增或占用 item、
  material、affix、resource 或 actor ID。
- 独眼巨人已按 RFB master 闭合完整静态资料、音波抗性、初始“知识”和 20 级 STR“投掷巨石”。
  伤害及额外费用共用显式权重的 prorated 缩放，20 级为 54 伤害/8 消耗，50 级为 250/36；
  New Game 已加入既有 `raceId`，玩家外观继续由职业 build 决定。
- 共享伤害枚举新增 `rock`。怪物侧保留无音波抗性时的原版眩晕检定；反射到玩家时保留碎片/流血和
  音波/眩晕二选一及库存损坏，并接入树木、寒冷易损地面物品事务。Protocol 因公共 DTO 枚举推进到
  `1.212`，生成的 schema 与 TypeScript binding 已同步；没有新权威状态、save 字段或公共初始化
  RNG 变化，因此不刷新 active fixture。
- 共享协调点为 pack `1.347.0` / content hash
  `ed59d59d22b47cc3695b727fb64eff5ffe3fa7d6058560bd17f87b4c9525b40c`、Protocol `1.212`、
  State Hash Schema v104、save v2、active baseline `contract-v303`（26 个 exact fixture）。聚焦内容、
  导入器、本地化、核心、Web、生成器与格式检查均通过；本批已随本节共同提交。下一正式种族是
  `rfb-legacy.race.yeek`。

## 64. main 当前交接（伊克人、恐吓怪物与等级酸免疫）

- 种族方向继续拥有既有 `rfb-legacy.race.yeek` 与 `rfb-legacy.skill-set.race-yeek`，并新增、拥有
  `rfb.ability.race.scare-monster` 和 `rfb.ability-program.race.scare-monster`；其他方向不得复制
  这些身份。本批不新增或占用 item、material、affix、resource 或 actor ID。
- 伊克人按 RFB master 闭合完整静态资料、2 格红外、酸抗性、初始“牺牲”和 15 级 WIS/15/50
  “恐吓怪物”；20 级时酸抗性提升为酸免疫，原版没有等级奖励。能力使用方向投射，对首个怪物按
  原版玩家等级分段公式与魅力豁免修正进行恐惧检定，成功持续 `3d(playerLevel / 2) + 1`。
- 内容模型新增可同时供 Race/Class 使用的等级抗性定义，按当前有效种族派生，因此临时变形会获得并
  在解除时失去酸免疫与种族能力。通用状态投射现在读取目标 Actor 的状态免疫和 `resist-all`；共享
  状态投射仍未建模原版 `PROJECT_REFLECTABLE`，与既有“迷惑怪物”等能力的当前边界一致。
- 共享协调点为 pack `1.348.0` / content hash
  `ea02a2ca6032c1243523c5d667c933f325294ecdb38faed3f831cd705d36f433`、Protocol `1.212`、
  State Hash Schema v104、save v2、active baseline `contract-v303`（26 个 exact fixture）。聚焦内容、
  导入器、核心、美德、Web、schema、source 验证、格式和 diff 检查均在本批完成；未运行或刷新全量
  fixture。下一正式种族是 `rfb-legacy.race.klackon`（克拉克人）。

## 65. main 当前交接（克拉克人、喷吐酸液与等级速度）

- 种族方向继续拥有既有 `rfb-legacy.race.klackon` 与
  `rfb-legacy.skill-set.race-klackon`，并新增、拥有 `rfb.ability.race.spit-acid`；其他方向不得
  复制该种族能力身份。它复用既有 `rfb.ability-program.mutation.spit-acid`，本批不新增 ability
  program、item、material、affix、resource 或 actor ID。
- 克拉克人按 RFB master 闭合完整静态资料、2 格红外、酸/混乱抗性、初始“勤勉”和 9 级
  DEX/9/50“喷吐酸液”；原版没有等级奖励。喷酸在 25 级前投射酸液箭，25 级起改为半径 2 的
  酸液球，伤害为玩家等级两倍，费用为 `9 + level / 5`。
- `RaceDefinition.speedPerTenLevels` 直接在既有 Species 统计层按 `level / 10` 派生，没有新增状态或
  种族专用分支。临时变形因此会获得并在解除时失去等级速度、两项抗性、红外与喷酸能力。
- 共享协调点为 pack `1.349.0` / content hash
  `28dae5ec2e1c29156610621c25e249ea7a940be965ffa1e26323d61313e711b1`、Protocol `1.212`、
  State Hash Schema v104、save v2、active baseline `contract-v303`（26 个 exact fixture）。聚焦内容、
  导入器、核心、美德、Web、schema、source 验证、格式和 diff 检查均在本批完成；未运行或刷新全量
  fixture。下一正式种族是 `rfb-legacy.race.dark-elf`（黑暗精灵）。

## 66. main 当前交接（黑暗精灵、魔法飞弹与法术容量）

- 种族方向继续拥有既有 `rfb-legacy.race.dark-elf` 与
  `rfb-legacy.skill-set.race-dark-elf`；本批新增并拥有 `rfb.ability.race.magic-missile` 和
  `rfb.ability-program.race.magic-missile`，其他方向不得复制这些身份。
- Ability Program 表达原版 `magic_missile_spell` 的等级伤害骰与 bolt/beam 投射；Race 内容表达
  1 级 INT/2/30 能力、黑暗抗性、20 级看破隐形、`spell_cap += 3` 与正式选择标签；初始美德按
  权威中文名记为“神秘”（`Enchantment`）。普通职业的 beam 概率使用 `max(0, level / 2 - 10)`，
  High-Mage 使用玩家等级，并复用既有职业法术伤害与 spell power 路径。
- 默认空值的 `RaceDefinition.seeInvisibleMinimumLevel` 与默认 0 的 `spellCapacityBonus` 均按当前有效
  种族派生，不保存重复状态；职业与种族 `spell_cap` 加法合并后一次应用。临时变形因此会获得并在
  解除时失去黑暗抗性、红外、等级视隐、法术容量与魔法飞弹。
- 本批不新增 item、material、affix、resource 或 actor ID。共享协调点为 pack `1.350.0` / content
  hash `3db290a0ff990486082f7710691d0050d5176fa3464a50f93e8d17a02355a494`、Protocol `1.212`、
  State Hash Schema v104、save v2、active baseline `contract-v303`（26 个 exact fixture）。新增和直接
  相关内容、导入器、核心、美德、Web、schema、source 与协议检查通过；按用户要求停止完整 Rust
  回归，未运行或刷新全量 fixture。下一普通顺序项为龙人，必须先规划动态亚种与 35 级天赋纵切。

## 67. main 当前交接（夺心魔、心灵震爆与等级心灵感应）

- 种族方向继续拥有既有 `rfb-legacy.race.mindflayer` 与
  `rfb-legacy.skill-set.race-mindflayer`；本批新增并拥有 `rfb.ability.race.mind-blast`，复用既有
  `rfb.ability-program.mutation.mind-blast`。其他方向不得复制该能力身份。
- Race 内容表达 5 级 INT/3/50“心灵震爆”、智力/感知维持、15 级看破隐形、30 级永久心灵感应、
  初始“启蒙”与正式选择标签。心灵震爆继续使用共享 psi 伤害和施法攻击加值路径，伤害为
  `3 + (level - 1) / 5` 个 d3。
- 默认空值的 `RaceDefinition.telepathyMinimumLevel` 直接接入既有永久心灵感应消费者，并按当前有效
  种族派生；临时变形因此会获得并在解除时失去属性维持、红外、等级视隐、等级心灵感应与心灵震爆。
- 本批不新增 ability program、item、material、affix、resource 或 actor ID。共享协调点为 pack
  `1.351.0` / content hash `e8970160ca2e3d84732228d705014b132f855cb2cabb829abbb82cbfa4f68d7e`、
  Protocol `1.212`、State Hash Schema v104、save v2、active baseline `contract-v303`（26 个 exact
  fixture）。仅新增和直接相关内容、导入器、核心、美德、Web、schema、source、协议、类型及格式
  检查通过；未运行或刷新全量 fixture。龙人动态纵切继续延期，下一静态种族为
  `rfb-legacy.race.imp`（小恶魔）。

## 68. main 当前交接（小恶魔、火焰箭/火球术与恶魔身份）

- 种族方向继续拥有既有 `rfb-legacy.race.imp` 与 `rfb-legacy.skill-set.race-imp`；本批新增并拥有
  `rfb.ability.race.imp-fire` 和 `rfb.ability-program.race.imp-fire`，其他方向不得复制这些身份。
- Race 内容表达 9 级 INT/8/50“火焰箭/火球术”、3 格红外、火抗、10 级看破隐形、恶魔身份、
  初始“信仰”与正式选择标签。能力在 30 级由 L 点单体火焰箭切换为半径 2、中心 2L 点火球，费用
  由 8 增加到 15；临时变形按当前有效种族获得并失去全部被动和能力。
- `BoltOrAreaDamage` 新增默认 1 的 `areaDamageMultiplier`，只在 area 等级阈值后、最终 spell power
  前作用于伤害骰和固定加值；公共 DTO 继续投影缩放后的最终值，Protocol、State Hash Schema、save
  与 active fixture 均不变。
- 本批不新增或占用 item、material、affix、resource 或 actor ID。共享协调点为 pack `1.352.0` /
  content hash `9fa98c5f1f4499138f5bcc7637a2941998a009d713e8388eead4c3d57843f042`、Protocol `1.212`、
  State Hash Schema v104、save v2、active baseline `contract-v303`（26 个 exact fixture）。新增和直接
  相关内容、倍率校验、导入器、核心、美德、本地化、Web、schema、source、协议、类型及格式检查
  通过；未运行或刷新全量 fixture。龙人动态纵切继续延期，下一静态种族为
  `rfb-legacy.race.golem`（魔像）。

## 69. main 当前交接（龙人专项阶段 3：隐藏亚种与动态喷吐）

- 种族方向新增并拥有九个隐藏 Race：
  `rfb-legacy.race.draconian-{red,white,blue,black,green,bronze,crystal,gold,shadow}`，以及逐亚种
  SkillSet、`rfb.ability.race.draconian-*-breath` 和
  `rfb.ability-program.race.draconian-*-breath`。其他方向不得复制这些 ID。本批没有新增或占用 item、
  material、affix、resource、actor 或公共协议 ID。
- 九个亚种的六维、生命/经验/商店倍率、技能、红外、抗性、漂浮，以及水晶 AC +10/40 级反射均按
  RFB `master:src/races_a.c` 落地。新增 `DraconianBreathDamage` 只存在于内容/核心内部，执行前降低
  为现有箭、束、锥形伤害；按当前 HP、亚种上限和等级立方项计算伤害，并让既有
  `rfb.mutation.draconian-breath` 控制减半伤害与 `2/3` 费用惩罚。
- 九个 Race 均不带 `rfb-compatibility` 和 `polymorph-candidate`，New Game 会返回
  `CharacterRaceUnavailable`。它们在 35 级龙人力量和出生亚种选择闭合前不是正式可选内容；后续
  工作不得提前将其加入 `PLAYTEST_RACE_IDS` 或请求白名单。
- 共享协调点为 pack `1.354.0` / content hash
  `087d9a1edf25f204228c4efdc8f396365684912eb85df06e039ab3c6ba4b515b`、Protocol `1.212`、
  State Hash Schema v104、save v2、active baseline `contract-v303`（26 个 exact fixture）。新增内容、
  核心和本地化测试以及 source/schema 检查通过；按用户要求不运行全量 fixture，留待主合并验收。

## 70. main 当前交接（龙人专项阶段 4：八项 35 级力量）

- 种族方向新增并拥有 `rfb.ability.mutation.draconian-kin`、对应 ability program，以及九组
  `rfb.ability.mutation.draconian-strike-{red,white,blue,black,green,bronze,crystal,gold,shadow}`
  和对应 ability program；其他方向不得复制这些 ID。本批不新增 item、material、affix、resource、
  actor 或公共协议 ID。
- 九个隐藏 Race 现在在 35 级提供除变形外的八项力量选择，并按出生亚种覆盖龙皮 AC/光环、本系
  strong 抗性与龙之打击类型/消耗。龙之打击内部效果执行元素品牌、混乱、震慑、vorpal 或吸血，
  DTO 继续投影既有近战效果；召唤同族按玩家等级从 `kin-glyph-100` 召唤持久友方并允许群体。
- ledger 中龙皮、龙之打击、致命吐息、召唤同族和抗性增加已改为 active，魔法抗性、再生和远古
  知识重新验收。`rfb.mutation.draconian-metamorphosis` 继续 blocked 且不在当前候选池；九个 Race
  仍不得加入 `PLAYTEST_RACE_IDS`、New Game 白名单或正式变形候选。
- 共享协调点为 pack `1.355.0` / content hash
  `6ee346ff1ecf60fe7c2dd62806d65a12dfe6db65b013a2cde828dcd421fdc32d`、Protocol `1.212`、
  State Hash Schema v104、save v2、active baseline `contract-v303`（26 个 exact fixture）。本批仅运行
  新增的内容、核心与中文聚焦测试；source/schema/格式/相关编译检查通过，未运行全量 fixture。

## 71. main 当前交接（龙人专项阶段 5：变形与龙形身体）

- 种族方向复用并继续拥有既有 `rfb.mutation.draconian-metamorphosis`，没有新增 ability、ability
  program、item、material、affix、resource、actor 或公共协议 ID；其他方向不得复制该 mutation，
  也不得另建龙形状态、身体或攻击身份。ledger 已将该项从 blocked 改为 active。
- 九个隐藏龙人 Race 的 35 级候选池均加入变形，弓箭手、骑兵和狙击手按原版排除。选择后由出生
  龙人身份和已锁定 mutation 派生永久龙形：身体使用六个戒指及护符、光源、斗篷、头盔槽，失效
  装备复用既有迁移事务回包；AC、爪击、撕咬及亚种/职业倍率均在现有统计和近战路径实时计算。
- 变形龙人对物理 polymorph 入口免疫；临时种族形态仍具有更高优先级，结束时会通过同一身体协调
  路径回到龙形。存档继续使用既有 `bodySlots`、出生种族和锁定 mutation，未新增权威状态、Protocol
  DTO、State Hash Schema 或 save schema。其他工作树若触及身体槽迁移、mutation 锁定、近战攻击
  档案或玩家 polymorph，必须保留这一单一事实源和龙形优先级。
- 共享协调点为 pack `1.356.0` / content hash
  `08a6e2a8f95727c4b47b4d2485c81decad7d42d8a0215e30253eea62cc4dc943`、Protocol `1.212`、
  State Hash Schema v104、save v2、active baseline `contract-v303`（26 个 exact fixture）。本阶段仅运行
  新增内容、核心和 replay 聚焦测试；source/格式/相关编译检查通过，未运行或刷新全量测试与 fixture。
  九个 Race 仍不带 `rfb-compatibility`，不得提前加入 New Game 白名单或正式 UI。

## 72. main 当前交接（龙人专项阶段 6：正式 New Game 亚种选择）

- 种族方向复用并继续拥有九个既有
  `rfb-legacy.race.draconian-{red,white,blue,black,green,bronze,crystal,gold,shadow}` ID；本批没有
  新增 ability、ability program、skill set、item、material、affix、resource、actor 或公共协议 ID。
  九个 Race 现均带 `rfb-compatibility`，其他方向不得恢复隐藏状态或建立重复的龙人创建身份。
- Web 的同一个 `PLAYTEST_RACE_IDS` 白名单和原生 `<select>` 现在正式包含九个 ID，并以权威中文
  “龙人分支”分组。请求仍只提交既有 `raceId`，玩家 Actor 与 tileset 仍由职业 Build 决定；没有增加
  “职业 × 亚种”Build 或第二套出生流程。
- 九个 Race 仍刻意不带 `polymorph-candidate`。普通临时种族变形方向不得因为正式 New Game 开放而
  自动把九个亚种加入候选；若未来改变该边界，必须单独验收亚种选择、35 级奖励和龙形身体交互。
- 共享协调点为 pack `1.357.0` / content hash
  `43c8437b663e727646a077e75a1f7a55318651087062542ffa6e60fbe399108c`、Protocol `1.212`、
  State Hash Schema v104、save v2、active baseline `contract-v303`（26 个 exact fixture）。本阶段只运行
  三项新增聚焦测试；source、相关 Rust 测试目标编译、Web typecheck、格式和 diff 检查通过，未运行
  或刷新全量测试与 fixture。下一普通静态种族仍为 `rfb-legacy.race.golem`（魔像）。

## 73. main 当前交接（龙人专项阶段 7：里程碑最终交接）

- 跨工作树最终实现基线为 `ef212db26`（`Expose Draconian subraces in New Game`），协调版本为 pack
  `1.357.0` / content hash `43c8437b663e727646a077e75a1f7a55318651087062542ffa6e60fbe399108c`、
  Protocol `1.212`、State Hash Schema v104、save v2、active baseline `contract-v303`（26 个 exact
  fixture）。阶段 7 为文档封板，不推进任何版本或 lock。
- 九个龙人 Race/SkillSet、逐亚种喷吐 Ability/Program、35 级力量、变形 mutation 及其既有 ID 继续由
  种族方向拥有；没有遗留待声明的龙人 item、material、affix、resource、actor 或公共协议 ID。其他
  工作树只复用这些身份，不得复制、改名或恢复隐藏标签。
- 最终入口的三项新增测试——九亚种正式创建、New Game 龙人分组、本地化 optgroup label——均通过；
  `verify-source`、相关 Rust 测试目标编译、Web typecheck、格式和 diff 检查通过。阶段 3–5 的新增行为
  测试记录在 `class-next-handoff.md`，按用户要求没有累计运行全量测试或刷新 fixture。
- 龙人专项至此封板。普通临时 polymorph 仍不得自动加入九个龙人亚种；职业/种族方向的下一项为
  `rfb-legacy.race.golem`（魔像）。

## 74. main 当前交接（魔像专项最终实现与正式开放）

- 种族方向继续拥有 `rfb-legacy.race.golem`、`rfb-legacy.skill-set.race-golem`，并新增、拥有
  `rfb.ability.race.golem-stone-skin`、`demo.item.staff-of-nothing`、
  `demo.device-activation.nothing` 和 `demo.effect.staff-of-nothing`。石肤术复用
  `demo.ability-program.nature-stone-skin`；其他方向不得复制或改名这些身份。
- 通用内容/核心边界新增等级种族 AC/速度、`holdLifeMinimumLevel`、食物营养除数、`slow-digestion`、
  `device-eater` 和出生物品 `fullyCharged`。`AbsorbDevice` 命令及物品 `absorbable` 投影已占用
  Protocol `1.213`；没有新增权威状态、save 字段或 State Hash 输入。其他方向应复用这些现有路径，
  不另建构装体进食或装置吸收事务。
- 魔像完整行为包括天然 AC `10 + floor(2L/5)`、速度 `-floor(L/16)`、35 级生命力保护、构装体被动、
  1/20 食物营养、背包/脚下/空装置吸收、20 级“石肤术”、出生无口粮及 21/21“空手法杖”、初始
  “正义”。没有职业过滤；临时变形按当前有效种族获得并失去相关被动和能力。
- `rfb-compatibility`、Web option 与 `PLAYTEST_RACE_IDS` 已加入，正式 New Game 种族数为 29；请求仍
  使用现有 `raceId`，玩家 Actor/tileset 继续由职业 Build 决定。最终实现提交为 `25b2bc0b6`
  （`Expose Golem in New Game`），此前三个阶段为 `2e1c74061`、`7e3114a7b`、`5e8fbee5c`。
- 最终协调点为 pack `1.361.0` / content hash
  `48db8bec826fc84f1b4b262ca621721b1ef729d4e78a79c4344914822d01d095`、Protocol `1.213`、
  State Hash Schema v104、save v2、active baseline `contract-v303`（26 个 exact fixture）。魔像内容、
  核心、中文名、importer、Web 及吸收后的 save/state-hash/replay 聚焦测试通过；source/schema、相关
  编译、Web typecheck 和协议 binding 检查通过。按用户要求未运行或刷新全量测试与 fixture。
- 下一普通静态种族为 `rfb-legacy.race.skeleton`（骷髅）。

## 75. main 当前交接（items 与 monster 增量整合）

- `codex/items-next` 的 `869169459` 已通过合并提交 `f199f84f2` 进入 main，带入 Life、Daemon、
  Crusade 三个法术领域及 ego 导入审计/元数据；`codex/monsters-next` 的 `5a268a14d` 本批继续以
  双亲合并进入 main，带入 P96–P101 的 13 个地牢、遭遇/掉落/地形表、固定神器与装备被动。
- 主工作树保留并联合了种族、状态与装备三条被动来源：反射、漂浮、消化减慢、接触光环、照明半径
  和地面物品影响均只有一条最终判定路径。房间型地面物品生成额外按最终地形过滤不可行走格，修复
  Castle 暗门上生成物品后真实存档无法恢复的问题；存档校验没有放宽。
- 最终协调点为 pack `1.365.0` / content hash
  `0732644f106d5571df518ff0546500cf7b783a910cfdd03c775767eb2e6aa80f`、Protocol `1.221`、
  State Hash Schema v104、save v2、active baseline `contract-v303`。公共初始化、内容池及 RNG 投影改变后，
  已按规则刷新并再次验证 26 个 active fixtures，0 waiver。
- 验收通过：`rfb-core` 830 项、`rfb-content` 287 项、本地化 21 项、其余 Rust workspace 测试、
  Clippy `-D warnings`、格式、协议/内容生成器、source lock 校验，以及 Web 152 项测试、typecheck 和
  生产 UI build。`codex/items-next`、`codex/monsters-next` 分支及对应工作树均保留，不做删除。

## 76. main 当前交接（僵尸正式内容、夜间出生与恢复生命）

- 种族方向继续拥有 `rfb-legacy.race.zombie`、`rfb-legacy.skill-set.race-zombie`，并新增、拥有
  `rfb.ability.race.restore-life` 与 `rfb.ability-program.race.restore-life`。出生空手法杖、缓慢消化、
  1/20 食物营养和装置吸收全部复用魔像批次既有身份与路径，其他方向不得复制或改名。
- 通用 `night-start` 标签只影响新角色初始化的 `worldTick`，不新增 Protocol、save 或 State Hash
  字段；临时变形不会改变时间。`RestoreVitality` 的生命力参数现按原版执行加法并封顶，现有 1000 点
  恢复保持等价。种族能力失败率按原版先应用最低失败率、再封顶 95%，支持僵尸的极低感知边界。
- 僵尸完整行为包括虚空/毒抗、5 级寒冷抗性、看破隐形、生命力保护、非生命/亡灵身份、30 级“恢复
  生命”、夜间出生、无普通口粮、满充能空手法杖和初始“非生”。`rfb-compatibility`、Web option 与
  `PLAYTEST_RACE_IDS` 已加入，正式 New Game 种族数为 30。
- 实现提交为 `2ecec68cd`（`Import Zombie race`）。共享协调点为 pack `1.366.0` / content hash
  `795d9e95d5285636b4a0273eb438f15d86f8ca6c4a7fb6ade99310ee094b9f2c`、Protocol `1.221`、
  State Hash Schema v104、save v2、active baseline `contract-v303`。新增内容、本地化、importer、核心和
  Web 共 9 项聚焦测试通过，含 save/state-hash/replay；按用户要求未运行全量测试或刷新 fixture。
- 下一普通静态种族为 `rfb-legacy.race.skeleton`（骷髅）。

## 77. main 当前交接（骷髅正式内容、食物漏落与药水泼洒）

- 种族方向继续拥有 `rfb-legacy.race.skeleton` 与 `rfb-legacy.skill-set.race-skeleton`；本批没有新增
  内容或协议 ID。恢复生命、空手法杖、缓慢消化、装置吸收、非生命/亡灵身份和夜间出生全部复用
  现有路径，其他方向不得复制这些身份或另建第二套亡灵代谢事务。
- 骷髅完整行为包括碎片/毒抗、10 级寒冷抗性、看破隐形、生命力保护、30 级“恢复生命”、夜间出生、
  无普通口粮、满充能空手法杖和初始“非生”。`rfb-compatibility`、Web option 与
  `PLAYTEST_RACE_IDS` 已加入，正式 New Game 种族数为 31。
- 既有物品使用事务现在按当前有效种族处理骷髅：普通食物在魔法效果结算后掉到当前脚下，蘑菇和
  精灵行粮在效果后消失，所有这些均不提供营养；药水在饮用效果后复用内容已有的 `shatter_effect`
  于当前玩家位置泼洒。临时骷髅形态也走同一规则，未增加 Protocol、save 或 State Hash 字段。
- 实现提交为 `be87dc1b2`（`Import Skeleton race`）。共享协调点为 pack `1.367.0` / content hash
  `a5d56b9c5e0f6c2fece100b5b117e363d0be4a78bab82f94927d5c48c3a8310d`、Protocol `1.221`、
  State Hash Schema v104、save v2、active baseline `contract-v303`。新增内容、本地化、importer、核心和
  Web 共 10 项聚焦测试通过，含 save/state-hash/replay；按用户要求未运行全量测试或刷新 fixture。

## 78. main 当前交接（木精灵正式内容、自然感知与树木通行）

- 种族方向继续拥有 `rfb-legacy.race.wood-elf` 与 `rfb-legacy.skill-set.race-wood-elf`，并新增、拥有
  `rfb.ability.race.wood-elf-nature-awareness`。能力复用既有
  `demo.ability-program.nature-awareness`，其他方向不得复制或改名这些身份，也不得另建侦测执行器。
- 木精灵完整行为包括权威属性/生命/HP/经验/红外/商店/技能矩阵、标准身体与出生、初始“自然”、
  20 级“自然感知”，以及未骑乘时按普通行动成本穿越树木。临时木精灵形态按当前有效种族获得并
  失去能力与树木通行；未修改普通角色和坐骑的树木规则。
- `rfb-compatibility`、Web option 与 `PLAYTEST_RACE_IDS` 已加入，正式 New Game 种族数为 32。没有
  新增 Protocol、save 或 State Hash 字段，也没有刷新 fixture。
- 实现提交为 `9d3397869`（`Import Wood-Elf race`）。共享协调点为 pack `1.368.0` / content hash
  `cce98fbd13eb10f345494c1562b72170d511a0f6627371e93068d5405800efb5`、Protocol `1.221`、
  State Hash Schema v104、save v2、active baseline `contract-v303`。新增内容、本地化、importer、核心和
  Web 共 7 项聚焦测试通过，含 save/state-hash/replay；按用户要求未运行全量测试或刷新 fixture。

## 79. main 当前交接（神使正式内容、飞行与看破隐形）

- 种族方向继续拥有 `rfb-legacy.race.archon` 与 `rfb-legacy.skill-set.race-archon`；本批没有新增内容或
  协议 ID。飞行、看破隐形、红外、标准身体和出生全部复用既有模型，其他方向不得复制这些身份或
  另建 Archon 专用被动路径。
- 神使完整行为包括权威属性/生命/HP/经验/红外/商店/技能矩阵、飞行、看破隐形、天使身份、标准
  身体/出生和初始“正义”。临时神使形态按当前有效种族获得并失去同一组被动；没有主动能力或职业过滤。
- 原作玩家阵营 `+200` 仍没有统一模型，本批明确不以 `good` 标签或局部状态替代；importer 保留对应
  `calc_bonuses` gap，后续只有在通用玩家阵营系统形成完整持久化生命周期时才闭合。
- `rfb-compatibility`、Web option 与 `PLAYTEST_RACE_IDS` 已加入，正式 New Game 种族数为 33。没有
  新增 Protocol、save 或 State Hash 字段，也没有刷新 fixture。
- 实现提交为 `7dafd6c6f`（`Import Archon race`）。共享协调点为 pack `1.369.0` / content hash
  `ca4c7b26e1bf204efefadedd2f116f95f2d4d713aeec543c417947033a68542b`、Protocol `1.221`、
  State Hash Schema v104、save v2、active baseline `contract-v303`。新增内容、本地化、importer、核心和
  Web 共 6 项聚焦测试通过，含 save/state-hash；按用户要求未运行全量测试或刷新 fixture。

## 80. main 当前交接（小妖精正式内容、睡眠粉与等级速度）

- 种族方向继续拥有 `rfb-legacy.race.sprite` 与 `rfb-legacy.skill-set.race-sprite`，并新增、拥有
  `rfb.ability.race.sleeping-dust` 和 `rfb.ability-program.race.sleeping-dust`。飞行、光抗、等级速度及
  两种睡眠结算都复用既有路径，其他方向不得复制这些身份或另建第二套睡眠事务。
- 小妖精完整行为包括权威属性/生命/HP/经验/红外/商店/技能矩阵、光抗、飞行、每 10 级速度 +1、
  标准身体/出生、初始“自然”和 12 级“睡眠粉”。临时小妖精形态按当前有效种族获得并失去相同被动。
- 内部 `sleeping-dust` 步骤只负责 25 级边界：低等级变为半径 1 `Sanctuary`，高等级变为
  `VisibleApplyStatus`；协议投影仍是既有类型。没有新增 Protocol、save 或 State Hash 字段。
- `rfb-compatibility`、Web option 与 `PLAYTEST_RACE_IDS` 已加入，正式 New Game 种族数为 34。
- 实现提交为 `638bab21c`（`Import Sprite race`）。共享协调点为 pack `1.370.0` / content hash
  `6923cb3c4cf41abd17e2ab03046b38dc0a027027b08a58c1ea648ec83fd510d5`、Protocol `1.221`、
  State Hash Schema v104、save v2、active baseline `contract-v303`。新增内容、本地化、importer、核心和
  Web 共 7 项聚焦测试通过，含 save/state-hash；schema/source lock/格式检查通过。按用户要求未运行
  全量测试或刷新 fixture。

## 81. main 当前交接（屁精正式内容、吞噬血肉与蘑菇增益）

- 种族方向继续拥有 `rfb-legacy.race.snotling` 与 `rfb-legacy.skill-set.race-snotling`，并新增、拥有
  `rfb.ability.race.devour-flesh` 和 `rfb.ability-program.race.devour-flesh`。其他方向不得复制或改名
  这些身份；出生蘑菇复用既有 `demo.item.fast-recovery-mushroom`。
- 通用能力投影新增 `DevourFlesh`，Protocol 因此推进到 `1.222`；它只表达把营养设为 14999、增加
  100 流血和承受最大 HP 三分之一伤害的同一事务。`usable-while-confused` 只允许明确标记的能力绕过
  混乱限制；前端确认取消不提交命令。没有新增权威状态、save 字段或 State Hash 输入。
- 屁精完整行为包括权威属性/生命/HP/经验/红外/商店/技能矩阵、标准身体/出生、初始“荣誉”、1 级
  “吞噬血肉”、出生 1–3 个快速恢复蘑菇，以及任何蘑菇在普通效果后共享一次 RNG 施加加速、石肤、
  英雄和巨力。蘑菇增益使用当前有效种族；蘑菇店拒绝同时检查当前有效种族和出生种族，以阻止变形
  绕过商店限制。
- `rfb-compatibility`、Web option 与 `PLAYTEST_RACE_IDS` 已加入，正式 New Game 种族数为 35。
- 实现提交为 `c8ca8e4d0`（`Import Snotling race`）。共享协调点为 pack `1.371.0` / content hash
  `c1eb19e4f44cc33dea8ba18ffa6ea266162723266cdc8bb27c2aa5517b791a36`、Protocol `1.222`、
  State Hash Schema v104、save v2、active baseline `contract-v303`。新增内容、本地化、importer、核心和
  Web 共 10 项聚焦测试通过，含 save/state-hash/replay；schema/source lock、bindings、typecheck、
  格式和 diff 检查通过。按用户要求未运行全量测试或刷新 fixture。

## 82. main 当前交接（博伊特人正式内容、呕吐与投掷加成）

- 种族方向继续拥有 `rfb-legacy.race.boit` 与 `rfb-legacy.skill-set.race-boit`，并新增、拥有
  `rfb.ability.race.vomit` 和 `rfb.ability-program.race.vomit`。其他方向不得复制或改名这些身份；
  标准口粮、火把、毒素伤害和营养状态均复用现有内容与事务。
- 通用能力投影新增 `Vomit`，Protocol 因此推进到 `1.223`；`usable-while-afraid` 与既有
  `usable-while-confused` 只允许明确标记的能力绕过对应限制。呕吐按原版清除中毒、在玩家中心造成
  半径 1 毒伤并降低营养，空腹时额外承受 10 自伤和 15 行动能量。没有新增权威状态、save 字段或
  State Hash 输入。
- 博伊特人完整行为包括权威属性/速度/生命/HP/经验/红外/商店/技能矩阵、投掷 +25、标准身体/出生、
  初始“牺牲”和 1 级“呕吐”。importer 同步提取 `skill_tht` 并生成 legacy throwing Skill，避免重导入
  出现悬空引用。
- `rfb-compatibility`、Web option 与 `PLAYTEST_RACE_IDS` 已加入，正式 New Game 种族数为 36。
- 实现提交为 `8bfeee7da`（`Import Boit race`）。共享协调点为 pack `1.372.0` / content hash
  `72c8fc68e42729b2da8914951768ee46a2fd1e8854ee221eabff500650a58e4d`、Protocol `1.223`、
  State Hash Schema v104、save v2、active baseline `contract-v303`。新增内容、本地化、importer、核心和
  Web 共 6 项聚焦测试通过，含 save/state-hash/replay；schema/source lock、bindings、typecheck、
  格式和 diff 检查通过。按用户要求未运行全量测试或刷新 fixture。

## 83. main 当前交接（英灵战士正式内容、治疗减半与半神选择）

- 种族方向继续拥有 `rfb-legacy.race.einheri` 与 `rfb-legacy.skill-set.race-einheri`；狂暴复用既有
  `rfb.ability.race.berserk`，其他方向不得复制这些身份。30 级奖励使用 `einheri-talent` 和当前共享的
  20 项已闭合半神候选。
- 通用 Race 内容新增 `healingReceivedPercent`，默认 100；英灵战士为 50。共享治疗先应用突变加成再
  应用有效种族倍率，临时形态因此获得并在解除后失去减疗。自然再生与旅店完整休息不走该事务；没有
  新增 Protocol、save 字段或 State Hash 输入。
- 英灵战士完整行为包括权威属性/生命/HP/经验/红外/商店/技能矩阵、生命力保护、再生 +100%、1 级
  “狂暴”、30 级半神选择、非生命/亡灵身份、标准身体/出生和初始“非生”。它正常进食且白天出生，
  不得因 undead 标签而获得装置吸收、缓慢消化或夜间出生。
- `rfb-compatibility`、Web option 与 `PLAYTEST_RACE_IDS` 已加入，正式 New Game 种族数为 37。
- 实现提交为 `8bb8236cf`（`Import Einheri race`）。共享协调点为 pack `1.373.0` / content hash
  `219d142fb3e7449c7e5d40c4c99c048f1e16c8a91388e18e058369c9cee349ce`、Protocol `1.223`、
  State Hash Schema v104、save v2、active baseline `contract-v303`。新增内容、本地化、importer、核心和
  Web 共 7 项聚焦测试通过，含 save/state-hash/replay；schema/source lock、typecheck、格式和 diff
  检查通过。按用户要求未运行全量测试或刷新 fixture。

## 84. main 当前交接（库塔正式内容、横向膨胀与豁免覆盖）

- 种族方向继续拥有 `rfb-legacy.race.kutar` 与 `rfb-legacy.skill-set.race-kutar`，并新增、拥有
  `rfb.ability.race.kutar-expand`、`rfb.ability-program.race.kutar-expand` 和
  `rfb.status.kutar-expand`。其他方向不得复制或改名这些身份。
- 通用状态授予技能数据新增可选 `savingThrowSkillOverride`；它覆盖最终豁免技能并沿既有状态 DTO、
  save 与 state hash 生命周期持久化。Protocol 因共享 DTO 推进到 `1.224`，State Hash Schema 推进到
  v105，save schema 保持 v2。任何依赖 state hash fixture 的并行方向必须基于 v105 重算，不能继续提交
  v104 预期值。
- 库塔完整行为包括权威属性/生命/HP/经验/红外/商店/技能矩阵、混乱抗性、标准身体/出生、初始
  “生命力”，以及 20 级魅力能力“横向膨胀”：消耗 15、基础失败率 70%、持续 `30 + 1d20`、
  +35 AC、豁免技能固定为 10。
- `rfb-compatibility`、Web option 与 `PLAYTEST_RACE_IDS` 已加入，正式 New Game 种族数为 38。
- 实现提交为 `33addaf2d`（`Import Kutar race`）。共享协调点为 pack `1.374.0` / content hash
  `0a6076df6cfbf9fa35a784935592c610161fe5bb4c74b6358f9d0231d7c9a78f`、Protocol `1.224`、
  State Hash Schema v105、save v2。新增内容、本地化、importer、核心和 Web 共 6 项聚焦测试通过，
  含 save/state-hash/replay；schema/source lock、bindings、typecheck、格式和 diff 检查通过。按用户要求
  未运行全量测试或刷新 `contract-v303` fixture；现有 v104 fixture 留给主合并验收统一迁移。

## 85. main 当前交接（安珀人正式内容、阴影位移与漫步全知阵）

- 种族方向继续拥有 `rfb-legacy.race.amberite` 与 `rfb-legacy.skill-set.race-amberite`，并新增、拥有
  `rfb.ability.race.amberite-shadow-shifting`、
  `rfb.ability-program.race.amberite-shadow-shifting`、
  `rfb.ability.race.amberite-pattern-mindwalk` 和
  `rfb.ability-program.race.amberite-pattern-mindwalk`。其他方向不得复制或改名这些身份。
- 通用能力效果新增 `alter-reality`，直接切换现有现实重构倒计时：0 时启动 15..35，非 0 时取消；
  协议投影记录前后值。Protocol 因共享 DTO 推进到 `1.225`，State Hash Schema 保持 v105，save schema
  保持 v2。
- 安珀人完整行为包括权威属性/生命/HP/经验/红外/商店/技能矩阵、体质维持、再生 +100%、标准
  身体/出生、初始“荣誉”，以及 30 级智力能力“阴影位移”和 40 级感知能力“漫步全知阵”。后者
  清除中毒、幻觉、震慑、流血、失明、恐惧并恢复六维损伤、经验等级和 1000 生命力；原版不直接
  治疗 HP，也不清除混乱。
- `rfb-compatibility`、Web option 与 `PLAYTEST_RACE_IDS` 已加入，正式 New Game 种族数为 39。
- 实现提交为 `503893dd4`（`Import Amberite race`）。共享协调点为 pack `1.375.0` / content hash
  `31b8d0c0f3c2824bc5a6cabd36d5661bcac3e5133fdb3d59c02353b6707c0719`、Protocol `1.225`、
  State Hash Schema v105、save v2。新增内容、本地化、importer、核心和 Web 共 6 项聚焦测试通过，
  含 save/state-hash/replay；schema/source lock、bindings、typecheck、格式和 diff 检查通过。按用户要求
  未运行全量测试或刷新 `contract-v303` fixture；现有 v104 fixture 留给主合并验收统一迁移。

## 86. main 当前交接（兽化人正式内容、随机突变成长与再生宽容）

- 种族方向继续拥有 `rfb-legacy.race.beastman` 与 `rfb-legacy.skill-set.race-beastman`；本批没有新增
  ability、program、item、material、affix、resource 或 actor ID，其他方向不得复制或改名既有身份。
- 永久兽化人出生时从现有随机池按权重取得一项 Good/Great 良性突变；首次达到每个新等级时逐级执行
  `1/5` 随机突变判定，恢复旧等级不重复触发。未锁定突变前 10 项不降低再生，之后每项降低 5%，
  最低 10%；其他种族仍使用每项 10% 的既有公式。以上规则读取永久选择种族，临时形态不触发。
- 兽化人完整行为包括权威属性/生命/HP/经验/红外/商店/技能矩阵、混乱/声音抗性、标准身体/出生、
  初始“运气”、出生良性突变、升级随机突变和专属再生惩罚。既有突变集合与 RNG 已覆盖 save、
  state hash 和 replay，不增加 Protocol、State Hash 或 save 字段。
- `rfb-compatibility`、Web option 与 `PLAYTEST_RACE_IDS` 已加入，正式 New Game 种族数为 40。
- 实现提交为 `435df76b0`（`Import Beastman race`）。共享协调点为 pack `1.376.0` / content hash
  `6abdb4a92ae78be7d6f4b5e6dede4dcfad1d176dc5916b2719e7e092b515f713`、Protocol `1.225`、
  State Hash Schema v105、save v2。新增内容、本地化、importer、核心和 Web 共 6 项聚焦测试通过，
  含 save/state-hash/replay；source lock、typecheck、Rust check/format 和 diff 检查通过。按用户要求未
  运行全量测试或刷新 `contract-v303` fixture；现有 v104 fixture 留给主合并验收统一迁移。
