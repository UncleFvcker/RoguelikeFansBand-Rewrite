# 法术领域导入交接

更新时间：2026-08-14

工作分支：`codex/items-next`

本文是后续玩家法术领域导入的当前入口。历史文档
[`legacy-player-spell-import-v1.md`](legacy-player-spell-import-v1.md) 只用于理解早期架构演进；其中把
Death 绑定到 `tval=100` 的内容已经过期，不能作为新导入的身份或行为依据。

## 1. 当前协调点

| 项目 | 当前值 |
| --- | --- |
| main 基线 | `1defe3183` |
| 本分支法术实现基线 | `83541be2b`（完整 Life 第四册） |
| 当前批次 | Crusade 四册完成 |
| demo pack | `1.355.0` |
| content hash | `a8f810c399fe5d9197500070cba8a6d7587bb64996d01ba88a4ca8bdb0390380` |
| Protocol | `1.218` |
| State Hash Schema | v104 |
| save header / payload | v2 / v2 |
| active contract baseline | `contract-v303` |

`main@1defe3183` 已包含 Death、Arcane、Sorcery、Armageddon、Nature，以及 Life 前三册。
`codex/items-next@83541be2b` 又完成了 Life 第四册；当前工作树继续完成了 Daemon 与 Crusade 四册，
因此该分支目前有八个完整 High Mage 领域、32 本能力书和 256 个可学习法术。后续工作不得重做
Life 第四册、Daemon 或 Crusade 领域；应先确认对应提交
是否已进入新的 main。

Daemon 第四册新增四个窄复合效果：`insanity-circle` 依次结算混沌、混乱与魅惑球；`explode-pets`
按宠物实例顺序引爆并让独特宠物逃离；`summon-greater-demon` 只在召唤成功后消耗所选人形尸体；
`hellfire` 在范围伤害后结算 `20+1d30` 生命反噬。送入地狱复用单体灭绝，提振士气与不朽之躯复用
临时状态，恶魔领主形态复用临时种族覆盖和穿墙状态。该投影使 Protocol 升至 `1.214`；没有新增持久
字段，State Hash Schema 与 save schema 均保持不变。

Crusade 第一册新增两个窄效果：`stardust` 保留十次独立散射、独立伤害掷骰、光照网格和可反射投射；
`sanctuary` 对半径 1 的怪物结算睡眠，不要求目标可见，并使用原始等级而非 spell power。该投影使
Protocol 升至 `1.215`；没有新增持久字段，State Hash Schema 与 save schema 均保持不变。完整 32
法术参数、源码差异和后续专用机制见 [`crusade-realm-audit.md`](crusade-realm-audit.md)。

Crusade 第二册复用区域伤害、视野伤害、状态、治疗与诅咒清除，只扩充了既有 `teleport-away` 的
遇怪停止/类别过滤，以及 `visible-apply-status` 的持续时间骰。神圣火焰按善良免疫、邪恶双倍、
其余目标随机减伤结算；驱魔保留不死与恶魔两次独立伤害 RNG；圣言严格按伤害、治疗、清状态顺序。
Protocol 升至 `1.216`；没有新增持久字段，State Hash Schema 与 save schema 均保持不变。

Crusade 第三册复用地形射线、视野伤害、武器烙印与大型光球，只新增 `holy-aura` 通用状态和窄化的
`angel-summoning` 投影。拘捕保留面板 `spell_power(2*level)`、实际原始 `2*level` 的源码差异；
天使斗篷只对邪恶近战接触者反伤；神圣之刃永久赋予杀戮词缀与 `slay-evil`；召唤天使保留 `1/3`
敌对、敌对允许群组、友好仅 50 级允许群组。Protocol 升至 `1.217`；没有新增持久字段，State Hash
Schema 与 save schema 均保持不变。

Crusade 第四册复用英雄、诅咒清除、区域毁灭与 Vengeance，只为无法由既有投影准确表达的驱逐邪恶、
神之愤怒、神圣干预和圣战增加窄复合效果。神之愤怒保留 `10+1d10` 个分解球、半径 4 散射、20 次
选点上限和永久墙阻挡；神圣干预严格按近身伤害、全屏伤害、减速、震慑、混乱、恐惧、冻结、治疗
结算；圣战保留善良怪魅惑、失败者恐惧、十二次骑士群组召唤及四种玩家增益。Protocol 升至
`1.218`；没有新增持久字段，State Hash Schema 与 save schema 均保持不变。

## 2. 唯一权威来源

所有新规则、英文名和中文名都以 `D:/codex/Frogcomposband` 的 Git `master` 对象为准，不能读取其
当前工作树，也不能继续沿用旧固定提交。常用入口如下：

| Git 对象 | 用途 |
| --- | --- |
| `master:lib/edit/k_info.txt` | 法书物品的 `N/I/W/A/F/M`：英文名、tval/sval、生成等级、分配、最大深度、元素免疫和城镇库存数据 |
| `master:src/kind_name_zh.inc` | 法书物品的权威中文名 |
| `master:lib/edit/m_info.txt` | 各职业的 32 条 `level/mana/fail/exp`；High Mage 固定读取 `N:10` 段 |
| `master:src/do-spell.c` | 通用领域法术的权威中文名、说明和实际施放分支 |
| `master:src/spells*.c` 及实际被调函数 | `do-spell.c` 委托出去的共享结算；必须继续追到真正改变状态的位置 |

只通过 Git 对象读取，例如：

```powershell
git -C D:/codex/Frogcomposband show master:lib/edit/k_info.txt
git -C D:/codex/Frogcomposband show master:lib/edit/m_info.txt
git -C D:/codex/Frogcomposband show master:src/do-spell.c
git -C D:/codex/Frogcomposband show master:src/kind_name_zh.inc
git -C D:/codex/Frogcomposband grep -n "目标文本" master -- src localization lib/edit
```

说明文本和执行代码不一致时，以实际执行代码为行为依据，并在测试或提交说明中记录差异。中文显示名
仍严格采用 `master` 中对应运行时中文字符串；没有中文定义时应标为 unresolved，不能自行翻译。

## 3. 领域身份与完成状态

`k_info` 的 source index 由第一个显式 `N:500` 起按后续 `N:*` 顺序递增。当前映射为：

| Realm | realm index | tval | source index | High Mage 状态 |
| --- | ---: | ---: | ---: | --- |
| Life | 0 | 90 | 500–503 | 本分支完整；第四册尚需确认是否已合入 main |
| Sorcery | 1 | 91 | 504–507 | 完整 |
| Nature | 2 | 92 | 508–511 | 完整 |
| Chaos | 3 | 93 | 512–515 | 未导入 |
| Death | 4 | 94 | 516–519 | 完整 |
| Trump | 5 | 95 | 520–523 | 未导入 |
| Arcane | 6 | 96 | 524–527 | 完整 |
| Craft | 7 | 97 | 528–531 | 未导入 |
| Daemon | 8 | 98 | 532–535 | 完整；原版 `m_info` 注释拼作 `Deamon` |
| Crusade | 9 | 99 | 536–539 | 完整 |
| Necromancy | 10 | 100 | 540–543 | High Mage 的 `R:10:0`，不得导入为 High Mage 领域 |
| Armageddon | 11 | 101 | 544–547 | 完整 |

三个未开始领域及已完成圣战领域的四册权威身份是：

| Realm | 第一册 | 第二册 | 第三册 | 第四册 |
| --- | --- | --- | --- | --- |
| Chaos | Sign of Chaos／混沌的标志 | Chaos Mastery／混沌精通 | Chaos Channels／混沌通道 | Armageddon Tome／末日巨著 |
| Trump | Conjurings & Tricks／戏法与把戏 | Deck of Many Things／万象无常牌 | Trumps of Doom／末日王牌 | Five Aces／五张王牌 |
| Craft | Handbook for Pupils／学徒手册 | Grade Holder's Book／持阶者之书 | Note of Acting Master／代理宗师笔记 | Spiritual Enlightenment／灵性启迪 |
| Crusade | Rites of Initiation／入会仪式 | Ways of War／战争之道 | Exorcism and Dispelling／驱魔与净除 | Wrath of God／神之怒 |

不要因为 Chaos 第四册英文名含 `Armageddon`，就把它绑定到 Armageddon realm；物品身份始终由
`I:tval:sval` 决定。

## 4. 当前内容模型

一个正式领域由以下几层组成，身份不可合并：

```text
实体法书 item
  -> abilityBook（realmId、rank、严格 8 个 abilityId）
  -> ability（名称、目标、费用默认值、投影元数据）
  -> abilityProgram（实际效果）
  -> playerAbilityBinding（玩家施放绑定）

High Mage class realmProfile
  -> 四本 abilityBook
  -> 32 条来自 m_info/N:10 的 level/mana/fail abilityOverrides

High Mage realm build
  -> firstRealmId
  -> 只额外发放第一册实体法书
```

稳定命名沿用现状：

- 构筑：`demo.build.high-mage-<realm>`；
- 实体法书：`demo.item.<book-slug>`；
- 能力书：`demo.ability-book.<book-slug>`；
- 法术：`demo.ability.<realm>-<spell-slug>`；
- 程序：`demo.ability-program.<realm>-<spell-slug>`；
- importer 生成书映射：`rfb-legacy.ability-book.<realm>-<book-slug>`。

同一法术的物理书和 ability 身份是共享内容。High Mage 的等级、法力和失败率在
`packs/rfb-demo-original/classes/high-mage.json` 的对应 `realmProfile.abilityOverrides` 表达；首次成功
经验写在 `playerAbilityBinding.firstSuccessExperience`。binding 的默认等级、法力和失败率也应与当前
High Mage 数值一致，不能因此复制一套 High-Mage 专用法术身份。

## 5. 每个领域的推荐拆分

先一次性审计全部 32 个法术，再按四册推进。不要只看法术名就开始扩 runtime。

1. 第一册：加入 realm profile、`high-mage-<realm>` 构筑、第一册物品/能力书和前八个真实法术；
   importer 只新增该领域明确的四组 tval/sval 映射。完成后领域必须可创建、学习和施放。
2. 第二册：补第 9–16 个法术及第二册正式获取。
3. 第三册：补第 17–24 个法术及高级书获取；同步原版四元素 `IGNORE_*` 等物品属性。
4. 第四册：补第 25–32 个法术，确认四册各八个、无 `NoOp`，再做版本与内容锁收口。

复杂法术在真实行为完成前不能放进玩家可获得的能力书。若一册最后一项需要新事务，可以先完成前置
runtime，但不要出售或掉落一册不完整法书。

第一册只加入新构筑的 `startingItems`。不得修改 `high-mage.json` 的通用出生装备，也不得影响已有
High Mage 构筑。法书的普通获取遵循 `k_info`：

- `minDepth = A.level`，不是 `W.level`；
- `weight = 100 / A.chance`；
- 同一物品的多组 `A:` 必须全部保留；
- 正数 `W.maximumLevel` 写入 `maxDepth`；
- `F:TOWN`、`M:` 与现有正式商店规则共同决定商店接入，不能只按册序猜测；未进书店的高级书才按
  当前项目约定接入黑市；
- 普通法书进入共享 `base-items`，不能绑定某个地牢；
- 固定奖励和白马旅店奖励实例不属于领域导入范围。

## 6. 实现原则

优先复用已经闭环的共享表面：投射、bolt/beam/ball/cone、地面物品元素影响、探测与地图、治疗与
状态、传送与召回、召唤与宠物维持、物品选择/鉴定/充能/烙印/附魔、地形改变、地震与区域毁灭、
virtue、`spell_power` 和受限随机分支。

只有原版行为无法由现有模型表达时才增加最窄的字段或复合事务：

- 不为一个法术建设任意表达式、任意条件或递归效果 DSL；
- 不复制物品侧、怪物侧或已有领域已经存在的结算算法；
- 投影数值和实际结算必须调用同一缩放路径；
- 明确哪些数值接受 `spell_power`，不能把所有骰面或所有半径默认放大；
- 随机分支要锁定原版抽取顺序，未进入的分支不得偷耗 RNG；
- 需要方向或物品目标的分支取消后必须原子回退，不扣法力、行动，也不保留部分效果；
- bolt、beam、ball、cone 是否影响路径地面物品按原版逐法术声明，不能从同类法术推断；
- 新增持久状态才更新 save 和 State Hash；纯内容或瞬时结算不能借机推进这些版本。

项目不兼容旧开发存档。新增必填持久字段时直接更新当前 DTO、校验、save schema 和聚焦读档测试，
不要增加迁移器或兼容分支。

## 7. 主要文件所有权

| 路径 | 责任 |
| --- | --- |
| `packs/rfb-demo-original/items/` | 四本实体法书 |
| `packs/rfb-demo-original/abilityBooks/` | 每册严格八个法术的目录 |
| `packs/rfb-demo-original/abilities/` | 法术身份、目标和展示参数 |
| `packs/rfb-demo-original/abilityPrograms/` | 法术效果程序 |
| `packs/rfb-demo-original/playerAbilityBindings/` | 玩家施放绑定 |
| `packs/rfb-demo-original/classes/high-mage.json` | realm profile 与 32 条 High Mage 参数覆盖 |
| `packs/rfb-demo-original/builds/` | 新领域构筑；只发第一册 |
| `packs/rfb-demo-original/legacy-item-adaptations.json` | 四本原版物品 source index 的显式激活映射 |
| `packs/rfb-demo-original/lootTables/base-items.json` | 按原版 `A:` 写入共享基础物品池 |
| `packs/rfb-demo-original/shops/` | 书店或黑市库存，不是固定奖励实例 |
| `crates/rfb-legacy-import/src/content.rs` | 明确的 tval/sval 法书映射和 importer 审计测试 |
| `crates/rfb-content/src/definitions/abilities.rs` | 只有确有需要时扩充能力 schema |
| `crates/rfb-content/src/validation/abilities.rs` | 与 runtime 一致的内容合法性约束 |
| `crates/rfb-core/src/game/abilities.rs` | 能力规划和实际效果 |
| `crates/rfb-core/src/game/player_abilities.rs` | 玩家目标、费用、失败与提交事务 |
| `crates/rfb-core/src/game/mod.rs` | 共享投影或少量仍未拆出的能力入口 |
| `crates/rfb-content/src/tests/abilities.rs` | 书本、程序和参数内容测试 |
| `crates/rfb-core/src/game/tests/high_mage.rs` | 学习、施放、公式、RNG 与构筑边界测试 |

先搜索现有 effect/program/状态和测试，不要按目录名猜测能力尚未实现。

## 8. 版本与生成物

| 变化 | 必须更新 |
| --- | --- |
| 仅内容 JSON/本地化 | pack 版本、`content.lock.json`，以及 README 中当前内容摘要 |
| ability/content schema 改变 | 上述项目 + 内容 schema 生成物 |
| Protocol DTO 改变 | Protocol 版本 + bindings + protocol schema |
| State Hash 输入结构改变 | State Hash Schema；按项目规则处理相关 fixtures |
| save 权威字段改变 | save header/payload schema、严格读档校验和聚焦 save/replay 测试 |

内容修改后先运行 `inspect-source`，人工确认新版本、数量和 hash，再把同一值写入 lock 并运行
`verify-source`。不要用手工估算 hash。

## 9. 聚焦验收

用户已明确：领域日常提交只运行新增和直接相关测试；全量 workspace、桌面、Web 和全量 fixture 留到
合并验收。每册至少覆盖：

- 八个槽位顺序、权威中英文名和 `level/mana/fail/exp`；
- 等级边界，以及等级 1/25/50 或法术自身关键阈值；
- 正、零、负 `spell_power` 的投影与实际结算一致；
- 抗性、唯一怪、免疫、地形和物品保护分支；
- 取消目标、施法失败、资源不足的原子性；
- 随机分支的 RNG 消费顺序；
- 新构筑只得到第一册，其他构筑和通用出生装备不变；
- 非本领域角色不能从该书学习法术；
- 法书 `A:` 深度、权重、最大深度、商店和基础池获取正确；
- ability book 中不存在 `NoOp`。

常用聚焦命令：

```powershell
$realmFilter = "chaos"
cargo test -p rfb-content $realmFilter
cargo test -p rfb-core $realmFilter
cargo test -p rfb-legacy-import physical_spellbooks_use_explicit_realm_tvals_without_capturing_necromancy
cargo run -q -p rfb-content --bin rfb-contentc -- inspect-source packs/rfb-demo-original
cargo run -q -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original
cargo run -p rfb-content --features schemas --bin generate-content-schemas -- --check
cargo run -p rfb-protocol --features bindings --bin generate-bindings -- --check
cargo fmt --check
git diff --check
```

只在本册触及对应 schema/Protocol 时运行相关生成器；无需为纯内容提交重复生成无变化的文件。

## 10. 不可越过的边界

- Death 固定为 `tval=94`、source 516–519；`tval=100` 是 Necromancy，不能被显式映射或宽泛
  `spellbook` 判定捕获。
- 不修改白马旅店奖励、任务奖励或任何既有固定奖励实例。
- 不顺带修改 High Mage 通用出生装备、其他职业出生装备或既有领域构筑。
- 不把普通法书强行绑定到地牢；普通掉落只进入共享基础物品池。
- 不为尚未选择的其余领域预造 enum、profile、空书或 `NoOp` 法术。
- 不把原版说明中的效果当成已执行代码；必须追踪真实分支和被调函数。
- 不自行翻译中文名，也不“修正”原版已有的权威中文措辞。
- 不为旧开发存档增加兼容层。

## 11. 一册完成的定义

提交前确认以下事项全部成立：

- [ ] 八个法术都有 ability、program、binding 和真实行为；
- [ ] ability book 严格按原版槽位绑定八个 ability ID；
- [ ] High Mage 参数逐条来自 `m_info/N:10`，没有误取 Mage 或其他职业段；
- [ ] 实体书的 tval/sval、`W/A/F`、英文名和中文名来自 `master` Git 对象；
- [ ] 第一册构筑边界或后续册获取路径正确；
- [ ] 没有新增 `NoOp`、虚构行为、地牢绑定或固定奖励实例；
- [ ] 新增及直接相关测试通过；
- [ ] pack、lock、必要生成物和版本只按真实变化更新；
- [ ] `git diff --check` 通过，且没有带入其他工作树的无关改动。

## 12. 当前下一方向：Ego

Craft 第四册「工艺」会进入完整的 RFB ego 选择与实例化流程，现有 15 条 affix 和显式等概率
`craft-item` 候选不足以表达权威行为。因此领域导入暂时让位于
[`ego-import-plan.md`](ego-import-plan.md)：先闭合 122 条 Craft 类型 ego（其中 121 条 rarity > 0
可由标准选择器抽中），再恢复 Craft 领域；不得用当前小候选池或单一“工艺”词条近似第四册。
