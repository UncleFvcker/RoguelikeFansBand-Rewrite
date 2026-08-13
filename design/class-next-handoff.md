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
