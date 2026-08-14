# 武器与挖掘工具 Ego 审计及 E3 实施计划

更新时间：2026-08-15

工作分支：`codex/items-next`

## 1. 审计结论

本次只通过 Git 对象读取 `D:/codex/Frogcomposband` 的 `master`；解析到的权威提交为
`efd63661302866038f58d8cd2553b23e6af3bf9d`。依据为：

- `master:lib/edit/e_info.txt`：30 条记录的 index、`T/W/C/F/E`；
- `master:src/ego_name_zh.inc`：逐 index 权威中文名；
- `master:src/ego.c`：选择后的实例化、相容性重试和 `ego_finalize`；
- `master:src/artifact.c`：随机属性、抗性、ESP、sustain 和 ability 辅助函数；
- `master:src/devices.c`：固定及带 bias 的随机 activation；
- `master:src/cmd1.c`、`combat.c`、`wild_realm.c`：特殊武器效果的真实消费者。

结构计数如下：

| 项目 | 结果 |
| --- | ---: |
| `WEAPON` 记录 | 27 |
| `DIGGER` 记录（含跨类型） | 6 |
| 同时属于 `WEAPON/DIGGER` | 3 |
| 仅属于 `DIGGER` | 3 |
| rarity 大于 0、可进入标准选择 | 30 |
| 当前审计标为“至少可表达一项属性” | 28 |
| 当前审计标为完全无属性可表达 | 2 |
| 有显式 `E:` activation | 4 |
| `ego.c` 分支可能随机增加 activation | 9 |
| 去重后可能带 activation 的记录 | 12 |
| 有基础物品子类型限制 | 8 |
| 现状可按权威行为直接开放 | 0 |

`currentImporterExpressible=true` 不是完成门禁。它只检查生成的 affix JSON 是否至少含一个非空字段；
当前 importer 仍会把多数 `C:` 最大值固化成顶格属性、把 Slaying/Craft 固定成两次近似掷骰，并且不执行
`ego.c` 的骰面、诅咒、相容性重试和特殊消费者。因此不能按“28 条已经可用、只补 2 条”推进。
两条被现有审计直接判为空的是 index 6 Arcane 和 index 16 Wild。4 条显式 `E:` 是
11/15/24/42；9 条分支随机 activation 是 6/8/9/10/11/12/14/25/26。8 条基础物品限制是
2/6/23/24/25/26/27/42。

## 2. 30 条逐项审查

`L` 为最低等级，`M` 为最高等级，`R` 为 rarity；`*` 表示无上限。中文名逐字来自
`ego_name_zh.inc`，其中 `& ... ~` 是 RFB 的全名模板，不得当作普通后缀拼接。

| Index | 英文名 / 权威中文名 | 类型 | L/M/R | 权威实例化要点 | 主要缺口 |
| ---: | --- | --- | --- | --- | --- |
| 1 | `of Slaying` / 杀戮之 | W | 0/*/2 | `1+m_bonus(4)` 次目标选择，1/8 时次数翻倍；按 rarity 升级 kill，近战还可能给对应 ESP | 当前固定两掷且没有 ESP |
| 2 | `of Sharpness` / 锋利之 | W | 10/*/2 | 仅剑/长柄；可能增加 `dd`，1/7 强锋锐，否则锋锐；钻石刃另走 1/8 特例；pval 给挖掘 | 近战骰面、Vorpal/2、子类型重试 |
| 3 | `of Force` / 原力之 | W | 20/*/4 | 命中/伤害各 1..3；共享 pval 1..2 给 INT/WIS；法力烙印按每击骰面耗魔并增伤 | 法力烙印消费者、共享 pval |
| 4 | `(Blessed)` / `(受祝福的)` | W/D | 0/60/8 | WIS pval；1/2 善良 ESP、1/5 光照；完整 Slaying；随机一项 ability；受祝福武器 | ability、Blessed、精确 Slaying |
| 5 | `of Extra Attacks` / 额外攻击之 | W/D | 50/*/8 | pval 按等级与基础骰面计算并分段封顶；隼之剑另加；禁止通用骰面 super-charge | source-index pval 分支 |
| 6 | `(Arcane)` / `(奥秘的)` | W | 50/*/6 | 仅魔术师法杖；pval 1..2，1/30 再加；命中/伤害 -10；法术威力、法力烙印、三项负属性；1/5 法师 bias activation | 当前 pack 无魔术师法杖；现审计误判为空；特殊消费者和 activation |
| 7 | `(Armageddon)` / `(毁灭的)` | W | 40/*/3 | 伤害附魔 1..10；按基础骰面走双倍或递增骰面；1/5 元素 brand；剑可锋锐，钝器可冲击/震慑，1/666 法力 | 双维骰面与四类特殊战斗效果 |
| 8 | `(Chaos)` / `(混沌的)` | W | 30/*/4 | 混沌 brand/抗性；随机一项任意抗性；1/5 混沌 bias activation | 随机抗性与 activation |
| 9 | `(Craft)` / `(工匠的)` | W | 15/70/2 | `1+m_bonus(4)` 次元素 brand；首掷可能带对应抗性并提前结束；后续各有 1/3 抗性；深层 1/6 法力；首掷还可能 activation | 当前固定两掷破坏相关性和提前结束 |
| 10 | `(Crusade)` / `(圣战的)` | W | 40/*/4 | 命中/伤害 1..6；WIS 与随机 sustain；可能光照、额外攻击或极稀有法力；额外攻击 pval 有骰面/隼之剑/深层分支；1/5 priestly activation | pval 特例、Blessed、mana、activation |
| 11 | `(Daemon)` / `(恶魔的)` | W | 70/*/6 | pval 给攻击次数/STR/DEX/负 WIS；随机善良/人类 slay、火焰 brand、激怒或负潜行；固定破坏 activation，1/5 随机 demon activation 可覆盖它 | drawback、固定与覆盖 activation |
| 12 | `(Death)` / `(死亡的)` | W/D | 20/*/4 | 吸血/生命保持；随机黑暗、光明弱点、毒/幽冥抗性、slay/kill；极稀有增骰并附重诅咒；1/5 necromantic activation | 动态骰面、弱点、具体诅咒、activation |
| 13 | `(Life)` / `(生命的)` | W | 20/*/4 | pval 1..4；生命倍率、生命保持、受祝福、恶魔/不死 slay | 共享 pval 与 Blessed |
| 14 | `(Nature)` / `(自然的)` | W | 15/*/2 | INT pval；动物 slay/ESP/再生；随机 kill、三元素 brand/抗性；鞭子火焰分支增骰；1/5 ranger activation | 相关随机分支、鞭子特例、activation |
| 15 | `(Trump)` / `(王牌的)` | W | 30/*/6 | 命中/伤害 1..4、pval 1..2；随机高抗 1 次且深层可能再 1 次；随机 CHR、恶魔 slay、ability；随机传送 drawback；固定长距传送 activation | 高抗、随机传送、ability、activation |
| 16 | `(Wild)` / `(狂野的)` | W | 80/*/16 | 骰面改成 `1d(dd*ds)`；各给随机抗性、高抗和 ability；每次命中从 14 种未激活 Wild buff 中选择，持续两回合并受 5 槽替换规则约束 | 当前审计为空；完整 Wild strike 状态机 |
| 17 | `(Order)` / `(秩序的)` | W | 90/*/16 | 骰面改成 `(dd*ds)d1`，且武器不触发普通暴击；音波/碎片抗性 | Order 骰面与暴击抑制 |
| 18 | `(Defender)` / `(防御者的)` | W | 20/*/4 | 命中/伤害各 1..4，护甲为 5 再加 1..8；随机 sustain；1/4 多次高抗，否则多次元素抗性；可有警告/漂浮/再生 | 几何次数抗性与独立附魔 |
| 19 | `of Westernesse` / 西方之地的 | W | 20/50/3 | 命中/伤害 1..5，三属性共享 pval 1..2；三类 slay/ESP、自由行动、识破隐形；1/3 恐惧抗性 | 共享 pval 与动态恐惧抗性 |
| 20 | `of Gondolin` / 贡多林的 | W | 25/*/3 | 命中/伤害 1..8；1/44 demon kill 替换 slay，否则可能恐惧抗性；按等级可能再加 evil slay | 替换语义和等级概率 |
| 21 | `of Morgul` / 魔古尔的 | W | 0/*/16 | 命中/伤害 1..20、护甲 1..10；重诅咒、激怒、毒 brand、不死 ESP、善良 slay，并从 10 种 heavy curse effect 中抽一项 | 不能只用 `curse=heavy` 代替具体诅咒 |
| 22 | `(Pattern)` / `(图案的)` | W | 40/*/6 | 命中/伤害 1..6；STR/CON 共享 pval 1..3；随机高抗；可能生命保持、DEX、恐惧抗性 | 高抗与动态属性 |
| 23 | `of the Noldor` / 诺多精灵的 | W | 70/*/50 | 仅非混沌之刃、基础伤害至少 10 的剑；`dd+1`；命中/伤害 1..10，CHR/SPEED pval 1..5及完整静态能力 | 子类型重试、骰面、共享 pval |
| 24 | `of Jousting` / 马战之 | W | 20/*/1 | 仅长枪/重型长枪；按 `dd*3` 连续增骰；1/3 人类 slay；固定骑乘冲锋 activation | 长枪限制、冲锋事务 |
| 25 | `& Hell Lance~` / `& 地狱长枪~` | W | 30/*/2 | 仅两类长枪；按 `dd*4` 连续增骰；随机恶魔抗性；1/16 吸血；1/5 demon activation；全名替换基础名 | 全名模式、抗性、activation |
| 26 | `& Holy Lance~` / `& 神圣长枪~` | W | 40/*/4 | 仅两类长枪；按 `dd*5` 连续增骰；随机神圣抗性；1/77 Order 转换；1/5 priestly activation；全名 | 全名模式、Order/Blessed、activation |
| 27 | `(Troika)` / `(三头马车)` | W | 60/*/24 | 仅剑；等级决定 `_lva`，再组合 Craft/Slaying、骰面、brand、Vorpal、激怒、抗性、sustain、ability、护甲和额外攻击 | 最大的复合 RNG 分支 |
| 40 | `of Digging` / 挖掘之 | D | 0/40/1 | 挖掘 pval 1..5、免疫酸毁；`AWARE` 在权威 `master` 只有声明而无运行时消费者 | 精确 pval；不得为 `AWARE` 发明效果 |
| 41 | `of Dissolving` / 溶解之 | D | 10/*/2 | `dd+1`、伤害附魔 1..3、挖掘 pval 1..5、酸 brand/免疫 | 工具近战骰面与附魔 |
| 42 | `of Disruption` / 瓦解之 | D | 50/*/4 | 仅鹤嘴锄；`dd+2`、伤害附魔 1..7、STR/挖掘共享 pval 1..5；固定化石为泥 activation | 当前 pack 无鹤嘴锄；定向 terrain beam activation |

## 3. 当前可复用能力与真实缺口

可直接复用的能力：

- 11 类 slay/kill、6 类普通元素/混沌 brand 及其近战消费者；
- 属性、装备加成、抗性、状态免疫、元素毁坏免疫；
- 再生、识破隐形、吸血、生命保持、漂浮、警告、缓慢消化、现有九类 ESP；
- 物品实例上的 `enchantments`、`curse`、`activation/charges` 和 `rolledAffixes`；
- `abilities.rs` 中已经为造箭实现的 `m_bonus`、Slaying 和 Craft 近似原版流程，可移动到共享
  ego owner，不能再复制第三份；
- 长距随机传送、区域毁灭和 ability 的 `terrain-beam` 已有消费者，可供 activation 复用。

必须闭合的缺口：

1. **基础物品身份与重试。** 当前 `ItemDefinition` 没有 `tval/sval` 权威身份，无法可靠判断魔术师法杖、
   钻石刃、混沌之刃、长枪、鹤嘴锄等限制。增加最小 `rfbBaseKind` 元数据，由 importer 写入；每次
   selector 调用仍只抽一次，但 owner 在 `ego.c` 会拒绝的结果上重新选择，保留原版额外 RNG 消耗。
2. **逐实例骰面。** 现有 `damageDiceOverride` 只服务弹药且只能改 `dd`。武器 ego 同时改 `dd/ds`。
   优先把绝对骰面结果存入该 ego 的 `rolledAffixes`，避免给所有物品增加无关顶层状态字段。
3. **共享 pval 和独立 `C:` 掷骰。** `to_h/to_d/to_a` 必须分别写入实例 `enchantments`；同一次 pval
   必须同时驱动该记录的所有属性/装备加成，不能把 maxima 写成静态顶格值。
4. **特殊武器消费者。** 增加最小 typed weapon traits：Mana、Vorpal/Vorpal2、Order、Wild、Impact、
   Stun、Blessed。Wild 复用标准状态系统表达 14 种两回合效果，不新增通用脚本语言。
5. **装备副作用和诅咒。** 闭合激怒、黑暗、随机传送，以及 Morgul/Death 可抽到的具体 heavy curse
   effect；仅用 Normal/Heavy/Permanent 严重度不足以还原运行时行为。
6. **activation。** 4 条 `E:` 中 Teleport/Destruction 可复用，Charge 和定向 Stone to Mud 需新增窄
   事务。另有 9 条走 `effect_add_random(bias)`；在各记录最低等级处候选数从 9 到 32 不等，不能用一
   个占位 activation 代替。应单独导入 `devices.c::_effect_info` 的相关权重、等级和 bias，并映射到
   已有 item effect/effect program；缺消费者的 effect 在补齐前阻止相关 ego 开放。
7. **名称组合。** 两条 `FULL_NAME` 必须替换基础名；其他中文前缀继续使用权威字符串。给 affix 增加
   最小名称放置模式，不把 `&`、`~` 显示给玩家，也不把地狱/神圣长枪拼成普通后缀。

Mauler 专属的 `ego_weapon_adjust_weight` 已审查，但当前 pack 没有 Mauler class。它只在该职业生成
骰面提升武器时改重量，本批不为不可达消费者新增持久字段；以后导入 Mauler 时以本审计为前置补齐。

## 4. 实施顺序与提交边界

所有子批独立提交。正式定义可以先存在于测试输入，但只有第 7 步完成后才进入玩家可达随机池。

### E3.1：锁定 30 条权威契约

- 为 30 条增加一张最小 contract expectation 表，锁定 index、中文名、类型、L/M/R、`C/F/E`、
  子类型限制和分支 activation；不为此编写 `ego.c` C 源码解析器；
- 给 base item importer 增加 `rfbBaseKind` 身份，并验证选中的普通武器/工具不重号；
- 增加 affix 名称放置模式和 Hell/Holy Lance 的中英文组合测试。

提交目标：`feat: add weapon ego source identity`

### E3.2：扩展原子物化结果

状态：已完成。共享 `rfb_m_bonus` 保持造箭 RNG 顺序并支持显式 generation level；实例级附魔、
近战骰面、特殊武器性质与 heavy-mask curse effect 全部随 `rolledAffixes` 持久化，物化成功后才由
`apply_to` 一次提交。协议推进至 `1.222`、save header/payload 推进至 v3、State Hash Schema
推进至 v105。

- 把造箭已有的 `m_bonus` 移入共享纯 RNG helper，输入显式 generation level；
- `EgoMaterialization` 增加附魔 delta、逐实例近战骰面、typed weapon traits 和具体 curse effects；
- `apply_to` 一次提交所有结果，retry/失败不得留下部分属性；
- `rolledAffixes` 保存所有随机结果，读档不重掷；
- 如 persisted DTO 结构发生变化，在本提交一次性推进 Protocol/save/state-hash schema，并按项目规则执行
  全量结构 fixture 刷新；后续纯内容提交不重复推进。

提交目标：`feat: add weapon ego materialization state`

### E3.3：实现共同生成规则与 30 条 source-index 分支

状态：已完成。共同物化器覆盖 1–27、40–42，拒绝分支由选择器重试并保持物品原子性；Slaying/Craft
成为造箭与武器共享的权威 RNG helper，30 条分支由完整 source-index 测试锁定，Sharpness、Extra
Attacks、Armageddon、Defender、Troika 与三个 digger 另有固定 seed 断言。补齐 Slaying 所需的
ESP Evil/Living 后协议推进至 `1.223`；save 与 State Hash 结构未变。Mauler-only 重量调整仍明确不在
本阶段实现。

- 实现独立 `C:` 掷骰、共享 pval、fire-brand 自动光照、sustain/ability/元素抗性/高抗辅助函数；
- 精确迁移 Slaying 与 Craft 的等级、次数、相关分支和 ESP，随后让造箭复用同一 helper；
- 按 source index 实现 1–27、40–42 的选择后分支、基础物品拒绝重试和通用骰面 super-charge；
- 对 Sharpness、Extra Attacks、Armageddon、Defender、Troika 和三个 digger 建立代表性固定 seed 测试；
- 不实现 Mauler-only 重量调整。

提交目标可拆为：

1. `feat: materialize basic weapon egos`
2. `feat: materialize special weapon egos`
3. `feat: materialize digger egos`

### E3.4：闭合特殊近战消费者

- Mana：每次有效近战按当前骰面计算资源成本，资源足够才增伤并扣除；
- Vorpal/Vorpal2：闭合连锁伤害概率，并与现有暴击顺序保持一致；
- Order：固定骰面结果并禁止普通暴击；
- Wild：命中后从未激活效果中抽取，最多五槽，持续两回合，满槽随机替换；
- Impact/Stun：复用现有地形与状态事务；
- Blessed：作为明确武器性质供职业惩罚和抵抗诅咒消费者查询；
- 每个性质至少覆盖成功、条件不满足和 RNG/资源不应消耗的路径。

提交目标可拆为：

1. `feat: implement vorpal order and mana weapons`
2. `feat: implement impact stun and blessed weapons`
3. `feat: implement wild weapons`

### E3.5：闭合装备副作用与重诅咒

- 实现装备激怒、黑暗半径和 RFB 随机传送时点；
- 从权威 heavy mask 中为 Morgul/Death 抽取具体 curse effect，并与 Heavy 严重度一起物化；
- 移除诅咒后停止 curse effect，但不移除 ego 固有的静态 drawback；
- 用直接装备预条件测试周期效果，不创建与目标无关的移动步骤。

提交目标：`feat: implement cursed weapon egos`

### E3.6：闭合固定与 bias activation

- 先实现 4 条固定 `E:`：Destruction、Teleport、Charge、Stone to Mud；
- 给 item activation 增加最窄的 effect-program 复用途径，避免复制 spell resolver；
- 从 `_effect_info` 导入 9 条分支实际使用的 level/rarity/bias 候选，并保持 source order 与
  `max(255/rarity,1)` 权重；
- Daemon 随机 activation 成功时覆盖固定 Destruction，未触发时保留固定项；
- activation 目标、取消、charge/recovery 和 RNG 顺序分别做 exact tests。

提交目标可拆为：

1. `feat: add weapon ego activations`
2. `feat: add biased ego activations`

### E3.7：导入内容并开放自然生成

- importer 生成 30 条正式 affix 和 30 条权威中英文消息；
- Arcane 与 Disruption 定义照常存在；当前缺少魔术师法杖/鹤嘴锄时只会按原版走拒绝重试，不会产生
  不相容实例；
- 给 `base-items` 增加窄的 RFB ego policy，仅对本批已完成的 `WEAPON/DIGGER` 生效；其他装备继续使用
  现有 affix policy，直到 E4/E5；
- 删除 Slaying 的固定“两掷近似”自然生成路径，保留显式法术引用但改走共享实例化；
- 更新 pack version、content lock 和只受本批行为影响的 fixtures；不因纯内容变化推进 state hash。

提交目标：`feat: import weapon and digger egos`

## 5. 完成门禁

E3 只有同时满足以下条件才完成：

- audit 恰好得到 30 条、全部有权威中文名、全部 rarity 大于 0；
- 30 条 affix 都通过 schema/content validation，index 和 `rfbBaseKind` 身份无冲突；
- 8 条子类型限制会按权威分支重试，绝不把 Arcane/Disruption 等装到错误基础物品；
- 12 条 activation-capable ego 没有占位效果或静默丢失分支；
- 所有随机结果保存/载入后完全一致，不重掷；
- 特殊 trait 均有真实战斗或周期消费者，不能只显示在 UI；
- 自然掉落只在本批全部闭合后开放，Craft「工艺」仍等 E4/E5 完成后再开放；
- 聚焦测试、内容校验、类型检查通过，提交后工作树干净。
