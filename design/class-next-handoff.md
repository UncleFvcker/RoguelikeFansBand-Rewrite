# class-next 职业方向交接

更新时间：2026-08-12

分支：`codex/class-next`

起始基线：`main@3fb94bcd`

## 已完成批次

- 固定死亡领域的正式 High-Mage，以及后续领域可追加的 build/casting profile 接口；
- 正式 Archer 的职业身份、出生物、箭袋、制造弹药、射击暴击/破损率与射击能量；
- Archer 制造弹药的 RFB 中性 `apply_magic`：普通、优秀、极好、诅咒，正负命中/伤害附魔，
  `of Slaying` / `(Elemental)` ammo ego，伤害骰强化上限 9，完全鉴定，player-made 来源与 99% 折价；
- 陶器碎片、断木棍、骨骸和骷髅类尸体均可作为箭/弩栓材料；背包与脚下来源均只消耗一个。
- 原版单一“制造弹药”界面分组：1/10/20 级依次显示弹丸、箭矢和弩栓，三个既有端点
  继续沿用原目标选择流程。

## 稳定 ID 与跨分支边界

- 新增物品：`demo.item.shard-of-pottery`、`demo.item.broken-stick`；
- 复用 items 方向已确定的 `rfb-legacy.affix.slaying`，本分支携带同一份内容定义以保持可独立验证；
- 新增且仅供弹药使用：`demo.affix.ammo-elemental`；
- 没有新增 ability ID；制造弹药继续使用三个既有 `demo.ability.archer-create-*` ID。
- 新增本地化分组键 `ability-group-demo-archer-create-ammo-name`，不是内容 ID。

集成 `items-next` 时，`rfb-legacy.affix.slaying` 必须保留 items 方向的同 ID 定义，不得生成第二个
ammo-slaying ID。若 items 方向随后导入陶器碎片或断木棍，也应统一到以上已声明 ID。

## 共享机制与集成注意

- `ItemEnchantmentsDto` 已从无符号改为有符号，以承载原版诅咒物品的负命中/负伤害；协议 JSON Schema
  因此改变，TypeScript 仍为 `number`。
- `ClassAbilityDefinition` 与 `AbilityDto` 增加可选 `uiGroupNameKey`；协议已升至 `1.176`，
  没有新增命令或持久菜单状态。
- 物品持久状态新增 `damageDiceOverride`、`originKind`、`discountPercent`；普通物品默认值不写入 JSON，
  player-made 弹药会进入存档与 State Hash。
- 本批已将 `STATE_HASH_SCHEMA_VERSION` 提升至 v88、active baseline 提升至
  `contract-v266`，并全量刷新/复验 21 条 fixture；save 容器保持 v1。
- demo pack 为 `1.261.0`，方向分支 lock hash 是
  `846d7565a37113590dcee9e2ea187fdbd4ff2786c0fa85fbe61743834ae89d0a`；main 合并其他内容后应统一重算。

## 明确未做

- 当前 `apply_magic` 只实现 RFB 的中性生成路径。Good/Bad Luck、Chance virtue、coffee/special mode、
  dungeon-specific good/great cap 与全局 `no_egos` 属于共享物品生成上下文，不在 Archer 内伪造；
- 其余 ammo ego（Holy Might、Returning、Endurance、Exploding）仍未导入；
- 本批 UI 元数据可供后续其他职业能力分组复用；法术按书分组仍继续使用既有 `bookNameKey`。

## 验收

- `cargo test -p rfb-core --no-fail-fast`
- `cargo test -p rfb-content`
- `cargo test -p rfb-protocol --features bindings`
- `cargo run -p rfb-protocol --features bindings --bin generate-bindings -- --check`
- `cargo run -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original`
- `cargo run -p rfb-contract -- verify-all tests/fixtures/active/baseline-policy.json`
- `cargo test -p rfb-contract --test contract_fixtures committed_contract_fixtures_pass -- --ignored`

## main 后续批次：死亡领域圣骑士

- 正式内容 ID：`demo.class.paladin`、`demo.skill-set.paladin`、
  `demo.build.paladin-death`、`demo.actor.paladin-player`。
- 出生只复用 `demo.item.broad-sword`、`demo.item.ring-mail` 和
  `demo.item.black-prayers`；没有新增物品或能力 ID。
- Paladin 的原版属性、八项技能、WIS 施法、`450/20/1200` 负重及 32 条 Death
  `m_info` 覆盖已导入，学习模式复用 `divine-random`。
- 第二步协调点：pack 1.273.0、Protocol 1.177、State Hash Schema v88、
  contract-v273。
- 后续 contract-v274 已正式占用 `demo.ability.paladin-hell-lance`：30 级 WIS、消耗 30、
  基础失败参数 70，伤害 `level × 3 + spellDamageBonus`；40 级恐惧抗性使用通用
  `ClassDefinition.levelResistances`，没有新增物品 ID。
- 当前最终协调点：pack 1.274.0、Protocol 1.177、State Hash Schema v88、
  contract-v274，content hash 为
  `e94926512734080f4743341e0eff07e3c96f371fe8cdac674089654b28fa2010`。
- 摧毁高级异教书经验继续等待生命/圣战高级书本，不导入占位书；逐武器熟练度内容
  底座见下一节，运行时仍待后续提交。

## 逐武器熟练度第一步

- 包 1.275.0 已加入 `ClassDefinition.weaponProficiency` 与
  `ItemDefinition.weaponProficiencyBaseItemId`，没有新增或占用 item/ability ID。
- 战士、高阶法师、弓箭手、圣骑士的默认值与覆盖项已按权威 `master:s_info.txt` 导入；
  克里斯杜瑞安/杀戮者共享斩首剑身份，痛苦共享长柄大刀身份。
- 审计命令读取 `master` Git 对象并核验 4 个职业、67 种基础武器：
  `audit-demo-weapon-proficiencies <selection> <adaptations> <classes>`。
- 当前只完成内容底座；战斗成长、命中修正、存档和 UI 留给后续提交。Protocol 1.177、
  State Hash Schema v88、save v1、active baseline contract-v274 均不变；content hash 为
  `4274e13bce1b7c3e1808267ac12c1fe4f5fa83e6f256c602c693205396767fa2`。

## 逐武器熟练度第二步

- `CharacterProgress.weapon_proficiencies` 已作为规范基础物品 ID 到训练值的独立稀疏表接入；
  未记录项读取职业出生值，神器/特殊变体只训练其 `weaponProficiencyBaseItemId`。
- 普通近战每个攻击命令训练一次且未命中也可成长；射击只在弹道碰到怪物时训练。等级
  门槛、怪物训练上限、成长插值和概率余数 RNG 均忠实使用 RFB master `skills.c`。
- 近战、弓和投石索应用 `(current - 4000) / 200 × 3` 命中修正，弩应用
  `current / 400 × 3`；不会把熟练度混入射速或弹药破损率。
- `PlayerProgressSaveDto.weaponProficiencies` 是必填稀疏数组；读档严格拒绝缺字段、重复、
  未知/非武器、别名、不高于出生值或超过职业上限的数据，不兼容旧开发存档。
- 本步没有新增或占用任何内容 ID。共享协调点为 pack 1.275.0、Protocol 1.178、State
  Hash Schema v89、save v1、active baseline contract-v275；content hash 保持
  `4274e13bce1b7c3e1808267ac12c1fe4f5fa83e6f256c602c693205396767fa2`。
- 角色面板收口见下一节；武术、双持、骑乘仍是独立缺口。

## 逐武器熟练度第三步

- `PlayerProgressDto.weaponProficiencies` 投影 67 种规范基础武器，包含近战/发射器分类、
  当前值、职业上限、原版等级及原版命中加成；神器和特殊变体不重复出现。
- 角色成长面板新增默认折叠的“武器熟练度”，按近战武器和发射器分组；中英文等级固定为
  `Unskilled / Beginner / Skilled / Expert / Master` 与“生疏 / 入门 / 熟练 / 专家 / 大师”。
- 本步没有新增权威状态、内容字段或内容 ID。共享协调点为 pack 1.275.0、Protocol 1.179、
  State Hash Schema v89、save v1、active baseline contract-v276；content hash 保持
  `4274e13bce1b7c3e1808267ac12c1fe4f5fa83e6f256c602c693205396767fa2`。
- active fixture 为 24 条：原 21 条共享投影已刷新，并新增近战成长、射击成长和熟练度
  存档回放。武术、双持、骑乘仍为独立缺口。

## 挖矿系统第一步

- 包 1.276.0 新增 `TerrainDefinition.digging` 与 `ItemDefinition.tunnelingPval`，并占用
  terrain ID `demo.terrain.rubble`；没有新增 item、ability 或 affix ID。
- 玩家挖掘力使用 RFB master 的 38 档力量表及装备最大值。Shovel、Pick、Gnomish
  Shovel、Orcish Pick 分别贡献 46/55/66/75；Warrior skill set 不再错误叠加挖掘成长。
- soft/hard/permanent 判定、继续挖掘条件、永久墙零 RNG、怪物转普通近战和地面物品不
  阻塞均已接入。前端通过既有事件 args 的 `retryable` 保留挖掘模式。
- 共享协调点：pack 1.276.0、Protocol 1.179、State Hash Schema v89、save v1、active
  baseline contract-v277；content hash 为
  `ee561b30744f44fd627805d8ed0a45eb64b21ec4c13991033b6f6254b29156b9`。
- 矿脉财宝、碎石掉落和挖掘疲劳尚未接入，后续不得在职业代码中另建挖矿判定。

## 挖矿系统第二步

- 新增通用挖矿熟练度：出生 0、上限 8000，只由玩家成功移除矿脉增长；普通/富矿公式、
  等级边界与跨档中文提示均来自 RFB master。
- 新增只读材料袋和十个 `rfb.material.*` 状态 ID；这些不是物品 ID，不与 items 分支争用。
  本批不生成材料，也不实现烹饪、炼药和材料转化。
- `PlayerProgressSaveDto` 的挖矿/材料字段必填并严格校验；`PlayerProgressDto` 投影挖掘力、
  共享熟练度等级、当前值/8000 及十种材料数量，角色面板提供默认折叠入口。
- 共享协调点：pack 1.276.0、Protocol 1.180、State Hash Schema v90、save v1、active
  baseline contract-v278（25 条 exact fixture、零 waiver）；content hash 保持
  `ee561b30744f44fd627805d8ed0a45eb64b21ec4c13991033b6f6254b29156b9`。

## 挖矿系统第三步

- 新增四个富矿 terrain ID：`demo.terrain.magma-hidden-treasure`、
  `demo.terrain.quartz-hidden-treasure`、`demo.terrain.magma-treasure`、
  `demo.terrain.quartz-treasure`。没有新增 item/material/ability/affix ID，不与 items 分支
  争用内容身份。
- streamer、隐藏搜索、玩家材料/金币/额外物品、魔法普通金币和碎石楼层掉落已闭合；所有
  收益通过来源统一的 terrain change 事务结算，不在职业代码中重复实现。
- 碎石掉落使用 `ItemOriginKindDto::Rubble`。原版 artifact 尝试、幸运、德行与特殊模式
  修正仍属于共享物品生成缺口；后续职业只能复用统一生成器。
- 共享协调点：pack 1.277.0、Protocol 1.181、State Hash Schema v90、save v1、active
  baseline contract-v279（26 条 exact fixture、零 waiver）；content hash 为
  `9e84e738fecbc3b74933c4a708c5a89cd77dd7bdd000c11b76c7d57184abec26`。

## main 当前批次：骑兵

- class 方向新增 `demo.class.cavalry`、`demo.skill-set.cavalry`、`demo.build.cavalry`、
  `demo.actor.cavalry-player`、`demo.ability.cavalry-rodeo`、
  `demo.ability-program.cavalry-rodeo`；没有新增物品、材料或 affix ID。
- 原版属性、生命/经验、八项技能、宠物维持除数、骑术 `2000/8000`、逐武器熟练度与
  15–25 支箭出生数量均已导入。逐武器审计现覆盖 5 个正式职业、67 种基础武器。
- 套马在 10 级开放，使用 STR、消耗 0、基础失败率 50；先强制上马，再执行等级、骑术、
  Unique 修正和两次短路随机检定。guardian/questor 不可驯服，失败进入统一强制落马。
- 新游戏入口、三套 tileset、骑术面板及既有能力方向选择均已接通；中文“骑兵”“套马”
  及说明使用 `master:src/cavalry.c`。
- 当前协调点：pack `1.300.0`，content hash
  `bb912c0d2adef96f8930f190e588f6a1a59a94b9df9b70ce59d6634913a4f2d9`，Protocol `1.190`，
  State Hash Schema v94，save v1，active baseline `contract-v287`。

## main 当前批次：坐骑经验、进化与骑乘羁绊

- 未新增正式内容 ID；复用现有七个 RFB 药水 ID。`ActorDefinition.evolution` 以稳定 actor
  ID 表达，当前导入 320 条可用原版关系，对应 315 个独立定义；13 个缺失目标留待怪物内容
  补齐，禁止在 class 方向创建占位 actor。
- actor 经验、羁绊、宠物击杀经验分配、进化换形和 2500/5000/10000 门槛已闭合。普通
  下马和楼层往返保留同一实体羁绊，进化为新 kind 后从 0 重新培养。
- 当前协调点：pack `1.302.0`，content hash
  `1dcf89e57968a66dcfce99ba036ad077012e8dcbea8e8a0697aca4756d4b9f70`，Protocol `1.191`，
  State Hash Schema v95，save v1，active baseline `contract-v289`。

## main 当前批次：捕获球与最终闭环

- 新增并由 class 方向拥有 `demo.item.capture-ball`；没有新增 ability、material、affix 或
  actor ID。items 分支不得重复导入同义物品。正式数据来自 RFB master source index 704：
  等级 15、`A:15/4`、12.0 磅、价值 1000、不可堆叠、shield 槽；2.0 磅旧计划值已按
  `W:15:0:0:120:1000` 修正。
- 捕获/释放复用 `UseItem` 的 entity/direction 目标；球内只保存 kind、速度、生命与经验。
  当前坐骑捕获、羁绊重置、丢弃/投掷敌对骰、摧毁强制宠物释放、周期恢复和 Unique 生成
  排除均已闭合。怪物资格来自通用 `capturePolicy`，核心不硬编码 actor ID。
- 当前协调点：pack `1.303.0`，content hash
  `538cce0f525d1530dbb109f4cf75074c69130b09eebca10d672628ad770467e5`，Protocol `1.192`，
  State Hash Schema v96，save v1，active baseline `contract-v290`。

## main 当前批次：狙击手 Commit 1 专注底座

- 未新增正式内容 ID。新增可选 `snipingProfile`、`concentrate` 效果及职业能力的
  `minimumConcentration` / `hitPointCost`；正式 Sniper class/build/skillset/出生内容和
  箭术能力仍属于后续单职业提交。
- 专注、弩栓命中/暴击、超额射速折半、AC 与弹药段伤害修正按 RFB master 接入；Web
  面板投影专注当前值/上限。
- `sniperConcentration` 与预留给 Commit 3 的 `probedActorKindIds` 已进入 save/hash，旧开发
  存档不兼容。协调版本：pack `1.312.0`、Protocol `1.196`、State Hash Schema v98、
  active baseline `contract-v293`。

## main 当前批次：狙击手 Commit 2 统一特殊射击事务

- 未新增正式内容 ID。新增 `sniper-shot` 效果的闪耀、撤退、除陷、燃烧、碎岩、冰冻、
  击退、穿透八种模式；正式能力定义、职业和出生内容继续留给下一批。
- 八种模式和普通射击共用唯一 projectile resolver，完整复用发射器、弹药、行动能量、
  重弓、逐武器/骑术成长、品牌/杀戮、暴击、死亡、掉落、破损与 Easy Tiring II。
- 撤退距离按 RFB master 修正为 `10 + 2 * concentration`；碎岩使用不会触发挖矿收益的
  `Projectile` 地形来源；穿透每继续越过一个碰撞目标才消耗一级专注。
- 协调版本：pack `1.312.0`、Protocol `1.197`、State Hash Schema v98、save v1、
  active baseline `contract-v294`。

## main 当前批次：狙击手 Commit 3 高级射击与探测怪物

- 未新增正式 item、ability、material、affix、class 或 build ID；正式狙击手及其 16 个
  职业能力仍由后续单职业内容提交拥有，其他内容分支无需预留身份。
- `sniper-shot` 已覆盖邪恶、神圣、爆炸、双重、雷霆、针刺与圣星之箭；全部继续复用唯一
  projectile 事务。特殊倍率和弹药品牌/杀戮倍率取最大值，再应用专注增伤；爆炸为物理
  范围伤害，双重射击共享一次行动，针刺保留嵌套 RNG 与 Unique/Unique2 免疫，圣星之箭
  射击后对玩家施加原版减速与震慑。
- `probe-monsters` 只收集既可见又有投射视线的非模糊怪物，逐实体投影生命、速度、AC、
  阵营、抗性、状态免疫、近战与施法能力，并记录稳定 actor kind lore。Web 收到 typed
  outcome 后打开可浏览的探测面板。
- 协调版本：pack `1.312.0`、Protocol `1.198`、State Hash Schema v98、save v1、
  active baseline `contract-v295`。现有 26 条 fixture 不进入未正式绑定的狙击手能力路径，
  因而只复验、不刷新断言。

## main 当前批次：狙击手 Commit 4 正式职业内容与 UI

- class 方向新增并拥有 `demo.class.sniper`、`demo.skill-set.sniper`、
  `demo.build.sniper`、`demo.actor.sniper-player`，以及 `demo.ability.sniper-*` 与同名
  `demo.ability-program.sniper-*` 的 17 对正式身份；items/monsters 分支不得重复创建。
- 本批不新增 item、affix、resource、material 或 ability book ID。出生引用现有匕首、
  软皮甲、轻弩和 20–30 支弩栓。
- 属性、life/base HP/经验、八项技能、宠物维持除数、逐武器熟练度和骑术均来自 RFB
  `master:src/sniper.c` 与 `master:lib/edit/s_info.txt`。审计现覆盖 6 个正式职业和 68 种
  基础武器。
- New Game、双语文案、能力分组、player actor 与三套 tileset 已接通。原版强力慢速物品
  感知等待共享装备感知系统，不在职业代码中造占位实现。
- 协调点：pack `1.313.0` / content hash
  `8b89d37d689db0c180feb1dbe213a3aa30aef910bd72a12a6c3d1af8222296dc`、Protocol
  `1.198`、State Hash Schema v98、save v1、active baseline `contract-v296`。现有 26 条
  fixture 不选择 Sniper，只复验、不刷新。

## main 当前批次：Human Commit 3 六种弱点与 35 级奖励

- 不新增内容 ID；正式 Human 在 35 级复用并锁定既有六项 `rfb.mutation.human-*`。
  高阶法师按 INT、圣骑士按 WIS、现有非施法职业按 STR 自动映射，DEX/CON/CHR 映射已
  留给未来正式职业。
- 六种弱点的战斗、恐惧、心灵感应、周期状态、技能与法术失败行为均已按 RFB master
  接入；降级、重新升级和 save/replay 不重复授予。
- 协调版本：pack `1.315.0`、Protocol `1.199`、State Hash Schema v98、save v1、
  active baseline `contract-v299`。

## main 当前批次：Human Commit 4 被动型半神天赋

- 不新增正式内容 ID；复用既有七项 `rfb.mutation.*`，完成不屈不挠、狂饮药水、神圣活力、
  恐怖巫术、武器多面手、恶魔契约与恶魔之握的原版被动行为。
- 运行时复用现有 HP、物品、治疗、法强、熟练度、死亡和装置事务；无脚本解释器、无新增
  协议或存档状态。账本推进至 132 active / 20 blocked。
- Human 20 级完整候选池仍有九项未实现，`demo.race.rfb-human` 暂不配置残缺选择。
- 协调版本：pack `1.316.0`、Protocol `1.199`、State Hash Schema v98、save v1、
  active baseline `contract-v300`。

## main 当前批次：Human Commit 5 主动与跨系统半神天赋

- 新增 ID 仅为无双追踪者与奇妙狂乱的两对 `rfb.ability.mutation.*` /
  `rfb.ability-program.mutation.*`；不新增职业、物品、怪物、材料、词缀或变异 ID。
- 隐秘施法/无双狙击手通过原版怪物怒气事务工作；闪避覆盖喷吐、火箭、投石和地震；个人
  崇拜在敌方召唤落地前依次执行友好/宠物保存；无双追踪者和“大屠杀”复用现有能力、
  探测与近战事务。奇妙狂乱的普通击杀按本次已用攻击数保存剩余行动能量。
- Human 20 级选择当前开放 20 项忠实闭环候选。原版仍有七项未实现候选，继续隐藏，详见
  `contract-v301-human-active-demigod-talents.md`，不得以相近现成功能替代。
- 协调版本：pack `1.317.0` / hash
  `b27d385635fe09ef107ca2dd4e7fe6475d58e7e3320893e899246920779f5cb2`、Protocol `1.200`、
  State Hash Schema v99、save v1、baseline `contract-v301`。全量 fixture 刷新和 replay 按
  用户要求留到合并验收；本批只验证新增测试与直接受影响的生成/内容校验。

## main 当前批次：正式种族 Commit 1 独立选择底座

- 新游戏请求和桌面初始化增加必填 `raceId`，核心以所选正式种族覆盖 build 默认种族；
  build 仍默认 Human，且不复制职业与种族的组合 JSON。
- 角色创建只接受带 `rfb-compatibility` 标签的 race。读档使用既有保存 `raceId`，并继续
  校验 class/personality 与 build 定义；save 和 State Hash 没有新增字段。
- 本批不新增内容 ID。正式选择列表目前只有 `demo.race.rfb-human`；下一批半兽人复用
  `rfb-legacy.race.half-orc`，不得创建第二个 ID。
- 协调版本保持 pack `1.330.0`、Protocol `1.201`、State Hash Schema v100、save v1、
  baseline `contract-v303`；按用户要求只运行新增和直接相关测试。

## main 当前批次：正式种族 Commit 2 半兽人

- 不新增内容 ID；正式选择直接提升 `rfb-legacy.race.half-orc` 与既有 skill set。新游戏列表
  现包含 Human 和半兽人，职业构筑继续拥有玩家外观。
- 按 RFB master 补齐 3 格红外视觉和 30 级天赋选择；静态属性、技能、生命、基础 HP、
  经验、商店倍率与黑暗抗性已经核验一致。半兽人复用 Human 当前 20 项已闭环天赋池。
- 原版池中尚未实现的七项 `ambidextrous`、`speed-reader`、`black-marketeer`、
  `tread-softly`、`inspired-smithing`、`strong-mind`、`astral-guide` 继续隐藏。
- 协调版本：pack `1.331.0` / hash
  `340bc4e519c8ded18b69e24d9bb39e66f6e38e3e78832590ecc685d45b6c84c5`、Protocol `1.201`、
  State Hash Schema v100、save v1、baseline `contract-v303`；未刷新既有 fixture。

## main 当前批次：正式种族 Commit 3 新增范围验收

- 同一 Warrior 构筑的 Human/半兽人验收现直接比较属性档位、最大 HP、经验倍率、职业合成
  技能和商店价格倍率；未显式覆盖 race 时仍与显式 Human 产生完全相同的初始 state hash 与
  RNG 计数。
- 半兽人的黑暗抗性、3 格红外视觉、29/30 级奖励边界、选择锁定和 save 往返继续由核心
  聚焦测试锁定；新增 replay 测试验证 30 级天赋选择命令和最终 state hash 可重放。
- 非正式 legacy race 与未知 race 仍从角色创建入口拒绝；Web 聚焦测试验证所选
  `raceId` 原样进入 `NewSessionRequest`。本批不改内容、协议、save 或 State Hash 版本，
  不刷新也不运行全量 fixture。

## main 当前批次：高等精灵 Commit 1 种族天生看破隐形

- `RaceDefinition.seeInvisible` 是默认关闭、内容派生的单一看破隐形来源；它与装备和临时
  状态逐份叠加，并通过现有有效种族解析自动跟随变形获得或失去，不增加存档状态。
- legacy race importer 只提取 `calc_bonuses` 顶层无条件的 `p_ptr->see_inv++;`；条件式来源
  继续由原 hook gap 记录。已经落入 JSON 的 `infravision` 不再重复登记为 gap。
- 本批不新增内容 ID，也不正式开放高等精灵；下一批在现有
  `rfb-legacy.race.high-elf` 上补齐静态内容并设置 `seeInvisible: true`，不得创建重复 race。
- 协调版本保持 pack `1.331.0`、Protocol `1.201`、State Hash Schema v100、save v1、
  baseline `contract-v303`；默认 `false` 省略序列化，现有内容锁与 fixture 均不变。

## main 当前批次：高等精灵 Commit 2 雪地与种族美德

- 成功进入带 `snow` 标签地形的移动恢复 RFB 耗时：普通角色增加 33%，超重追加量采用
  `min(200, totalWeight * 100 / weightLimit) - 100`，骑乘固定 40%；附加基数为
  `min(120, actionCost)`。本地移动和世界地图移动共用同一结算，受阻、攻击等未移动结果不加费。
- 飞行、穿墙、具备 Snow habitat 的坐骑及带 `snow-adapted` 标签的有效种族豁免。现有
  `rfb-legacy.race.high-elf` 获得该标签，importer 同步保留；本批仍不把高等精灵加入正式选择。
- 初始美德表补齐高等精灵“活力”与已正式半兽人“勇猛”，不新增美德状态或协议字段。
- 协调版本：pack `1.332.0` / hash
  `de97fefcb8c224d5dc5b989c7d531808d5a26b9acc9bf190a3b114b76ca2fc2d`、Protocol
  `1.201`、State Hash Schema v100、save v1、baseline `contract-v303`。按用户要求未刷新或
  运行全量 fixture，只执行新增和直接相关测试。

## main 当前批次：高等精灵 Commit 3 正式内容与 UI

- 本批不新增内容 ID，直接正式化 `rfb-legacy.race.high-elf`。静态属性、生命 99%、基础 HP
  19、经验 190%、商店 90%、八项技能、光明抗性均按 RFB master 核验；补齐 4 格红外视觉、
  一份天生看破隐形来源及 `humanoid` / `standard-body` / `rfb-compatibility` 标签。
- 高等精灵保留 `kin-glyph-104`、`snow-adapted` 和 polymorph 身份，不增加等级奖励、种族能力、
  玩家 actor、物品、能力、材料或词缀 ID。legacy importer 重导入时会保留其正式运行标签。
- New Game 正式种族列表加入“高等精灵”；中文名称和说明使用 RFB master 原文，英文恢复为
  `High-Elf` 及原版说明。选种、save、看破隐形 RNG replay、光抗、红外视觉、雪地豁免和初始
  “活力”美德均有直接测试。
- 协调版本：pack `1.333.0` / hash
  `6cf1fcef5b2162e3ecc653c91dcc2fb5beae7d95ca89091b660ff7b96e2336fd`、Protocol `1.201`、
  State Hash Schema v100、save v1、baseline `contract-v303`。依用户要求不运行或刷新全量 fixture。

## main 当前批次：登丹人正式内容与 UI

- 不新增内容 ID；正式种族复用 `rfb-legacy.race.dunadan` 与
  `rfb-legacy.skill-set.race-dunadan`。静态属性、生命/经验/商店倍率、八项技能、Standard
  身体、Human kin、体质维持和初始“个人主义”均按 RFB master 锁定。
- 登丹人在 30 级使用与 Human、半兽人相同的 20 项已闭环半神天赋候选；原版尚未闭环的
  七项继续隐藏。等级奖励固定读取永久选择种族，临时变形成登丹人只取得体质维持，不会
  获得 `dunadan-talent`。
- New Game 列表现包含 Human、半兽人、高等精灵和登丹人；继续使用既有 `raceId`，玩家
  actor 与三套 tileset 外观仍完全由职业构筑决定。
- 协调版本为 pack `1.334.0` / hash
  `902ac3141bab282d663871d70e50a80cb4c9556b347f32eeddf230943b1e7fd3`、Protocol `1.201`、
  State Hash Schema v100、save v1、baseline `contract-v303`。只运行新增和直接相关测试；全量
  fixture 留到合并验收。

## main 当前批次：野蛮人 Commit 1 种族能力底座

- 本批预留并归种族方向所有 `rfb.ability.race.berserk`；它复用既有
  `rfb.ability-program.mutation.berserk`，不新增 item、material、affix、resource 或
  ability-program ID。其他并行方向不得重复创建这个 ability ID。
- `RaceDefinition.abilities` 使用与变异能力共享的 `InnatePowerDefinition`。运行时从当前
  有效种族派生，因此临时变形成带能力的种族会获得能力，解除后立即失去；不保存第二份解锁
  状态。种族、职业和变异能力受恐惧与混乱限制，费用先扣职业 SP、不足部分扣 HP；已学法术在
  狂暴时不可施放。
- 共享狂暴程序改为 `keep-strongest`；种族版能力预先承载原版 `10 + 1d(level)` 持续时间和
  `level / 5` 额外近战伤害。野蛮人种族尚未加入能力配置或 New Game，留给后续正式内容提交。
- 协调版本为 pack `1.335.0` / hash
  `bf30576bef5c777b778d085de8e69e240a163a151901c6fa6b91d623fd444f6f`、Protocol `1.202`、
  State Hash Schema v100、save v1、active baseline `contract-v303`。按用户要求只运行新增和直接
  相关测试，不刷新全量 fixture。

## main 当前批次：野蛮人 Commit 2 正式内容

- 不新增内容 ID；正式种族继续使用 `rfb-legacy.race.barbarian`，主动能力使用 Commit 1 已预留的
  `rfb.ability.race.berserk`。属性、生命 103%、基础 HP 22、经验 135%、商店 120%、八项技能、
  一层恐惧抗性与 8 级 STR/10/30 狂暴均按 RFB master 锁定。
- 野蛮人在 30 级使用 `barbarian-talent`，候选与 Human、半兽人、登丹人当前 20 项已闭环半神
  天赋一致；保留 `polymorph-candidate`，并加入 `humanoid`、`standard-body`、
  `rfb-compatibility`。初始美德为“勇猛”。
- importer 只读取字面量 `power_info` 行；已知 `berserk_spell` 映射到稳定 ability ID，未知或
  非字面量能力继续进入 `race_hook_gaps`，不会生成占位能力。`RES_FEAR` 现在进入正式 resistance
  投影。英文说明恢复 RFB 历史权威文本，中文继续使用 master 原文。
- 协调版本为 pack `1.336.0` / hash
  `fecd2a3598b99e61a66e07c76376e0e6c79da2e1702d6dafa2ce63ca0859cb8f`、Protocol `1.202`、
  State Hash Schema v100、save v1、active baseline `contract-v303`。New Game 列表与前端入口留给
  Commit 3；本批只运行新增和直接相关测试。
