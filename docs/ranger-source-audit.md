# 游侠来源与消费者审计

审计日期：2026-09-11，初次审计对应[游侠计划](ranger-class-plan.md)第一步、基线 `6f9b22c1a6649833bf2ff975878efb8cedb235d3`。当前前六步已完成：Class/四 Build、出生/成长、双领域随机学习/改换保存、射击/树林/探测、生成/奖励/公会及正式创角/UI；实施与适配边界见第8—12节。内容包1.425.0、协议1.253、State Hash Schema125、save header/payload14/20；本轮未改变保存格式。第七步实战闭环与优化桌面交付仍待完成。

唯一 RFB 来源为 `D:/codex/Frogcomposband/master` 的 `master` Git 对象，实际提交 `a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`。以下源文件及行号均指此提交，使用 `git show` / `git grep` 读取，未读取源仓库当前检出文件。当前项目路径则指上述实现基线。

## 范围与方法

全读 `src/ranger.c`；解析 `lib/edit/m_info.txt N:4` 和 `lib/edit/s_info.txt N:4`，追踪显式 `CLASS_RANGER` 以及 caster flags、学习方式、主副领域、武器表索引和公共规则。核对当前出生/成长、施法/学习/保存、投射物/移动、物品效果/生成、任务、公会和 UI 消费者。

原版 `tables.c:1714,1788` 固定主领域 Nature，副领域允许 Sorcery/Chaos/Death/Trump/Arcane/Daemon。本轮四个 Build 为 `demo.build.ranger-nature-{sorcery,death,arcane,daemon}`，对应自然＋咒术/死亡/奥秘/恶魔；混沌和王牌留给领域批次。Craft 即使已合入也不属于 Ranger 候选。未发现 Ranger 的职业专属种族禁配；保留现有正式种族资格及未开放怪物种族边界，不套用 Duelist 的冬贝利禁配或 Priest 善恶互斥。

本次逐书核对五个领域共 20 本书、160 个稳定 ability ID；四本书拼接顺序与现有 Mage Class 的 32 项 override 顺序逐项一致，再对照源 T 记录。该核对证明身份/参数映射，不表示重新执行了 160 个公共效果测试。

## 1. 职业、出生、成长与种族

| 源规则 | 当前实现及差异 | 落地步骤 |
| --- | --- | --- |
| `ranger.c`：中文名“游侠”；STR/INT/WIS/DEX/CON/CHR 为 +2/0/+2/+1/+1/0；life 106、base HP 8、exp 140、pets 35 | 尚无 Ranger Class。现有 [Class 类型](../crates/rfb-content/src/definitions/characters.rs)、[成长](../crates/rfb-core/src/game/progression.rs)和宠物维持可表达这些修正 | 第二步 |
| 基础 dis/dev/sav/stl/srh/fos/thn/thb 为 30/37/36/3/24/16/56/50，每十级成长 8/11/10/0/0/0/18/16；`combat.c:266` blows 为 500/70/40 | 复用 skill set 与现有近战参数；射击技能的额外 `20+L` 另在装备发射器时计算，不能重复写入基础技能 | 第二/四步 |
| `ranger.c::_birth`：匕首、软皮甲、短弓、随机 20—40 支箭、双方各第一本书 | 已有 `demo.item.dagger`、`soft-leather-armour`、`short-bow`、`arrow` 与四个组合所需书；复用 Class startingItems 数量范围及 Build 书本。未自动学会法术；沿种族出生合并和装备槽规则 | 第二步 |
| `s_info N:4` 为 320 个 W、3 个 S；`skills.c:1052` 默认起点 0 提升到 `min(2000, maximum)` | 见下方完整分组；不得复制 Archer 或 Sniper 的武器熟练度表 | 第二步 |
| `virtue.c:240` 固有 Nature、Temperance，再由种族/出生领域补充 | [virtues.rs](../crates/rfb-core/src/game/virtues.rs)缺 Ranger 身份。改换副领域不重抽出生美德；主 Nature 与固定 Nature 美德按源去重 | 第二步 |
| `ranger.c` 两类感知均 SLOW/STRONG；`dungeon.c:183–273` 两者频率基数均 80000，再经 WIS/Knowledge/等级调整 | [item_knowledge.rs](../crates/rfb-core/src/game/item_knowledge.rs)周期入口仅 Mindcrafter/Mage，且第一类默认弱感知。Ranger 需两类都强，沿既有回合入口、物品分类与背包抽样，不改成拾取即鉴定 | 第二步 |
| `py_birth.c:2696` 建议属性 16/11/16/16/14/8；`xtra1.c::_calc_xtra_hp_aux` Ranger 落默认权重 1/1/1 | 保留当前公共出生属性及 HP progression，不新增源点购或完整额外 HP 曲线。职业修正必须导入；“成长到 50 级”仅指当前 progression 与源职业修正结合 | 第二/七步明确范围 |
| `races_a.c` 龙人变形攻击 Ranger 无职业缩放，沿种族调整后的 100% | 当前默认 100% 可复用；不能随双领域公共化获得 Mage 的 80%。Tomte 头部重装与普通种族装备槽仍影响实际射击/负重 | 第二/四步 |

武器表按 `TV_WEAPON_BEGIN=19` 加 W 的第一索引解释，0/1/2/3/4 的熟练度档位对应 0/4000/6000/7000/8000。普通出生后的完整分组为：

- 发射器（tval 19）：短弓、长弓及 sval 63 为 4000→8000；投石索、轻弩、重弩为 2000→7000；其余表内槽位为 2000→8000。
- 挖掘工具（20）、长柄（22）：全部 2000→6000。
- 钝器（21）：默认 2000→6000；双节棍（sval 4）2000→4000、钓竿（40）0→0、sval 63 为 2000→8000。
- 刃器（23）：默认 2000→6000；匕首（4）、短剑（10）、阔剑（16）4000→8000，Main Gauche（5）2000→7000，毒针（32）2000→8000。此处武器中文显示仍复用现有 item 名称，不按审计说明另命名。
- Martial Arts、Dual Wielding、Riding 均为 0→6000。现有 Class 字段与公共成长消费可复用双持和骑乘；第二步实现时确认当前没有武术熟练度系统，Martial Arts 的源值只留记录，不新增孤立字段或整套徒手系统。0 起点不表示禁止徒手、双持或骑乘。尚未导入的表内武器槽位只留来源记录，不为填满矩阵导入无关物品。

## 2. 法术表、MP、费用与失败

`m_info N:4 I:LIFE:WIS:0x06:0:3:450`：LIFE 表示随机祈祷式学习，并不授予生命领域。`0x06` 为 `MAGIC_FAIL_5PERCENT | MAGIC_GAIN_EXP`；最低失败率和手套负重还须按 `ranger.c` 的 caster 信息落实。源五领域索引分别为 Sorcery=1、Nature=2、Death=4、Arcane=6、Daemon=8；7 是未开放给 Ranger 的 Craft，不能误读成 Daemon。

| 领域 | 核对 T 记录 | 与 Mage 等级/费用/失败三元组不同 | 与 Mage 原始首用 XP 不同 | 等级 >50 的记录 |
| --- | ---: | ---: | ---: | ---: |
| 自然 nature | 32 | 31 | 30 | 0 |
| 咒术 sorcery | 32 | 32 | 32 | 3 |
| 死亡 death | 32 | 32 | 31 | 6 |
| 奥秘 arcane | 32 | 32 | 32 | 1 |
| 恶魔 daemon | 32 | 32 | 31 | 5 |
| 合计 | 160 | 159 | 156 | 15 |

首用 XP 比较为源 `minimumLevel × sexp` 与当前 Mage `firstSuccessExperience`，不是实际游戏中完成了首用奖励的计数。每个领域的本轮不可学习项按零起始源 index 如下：

- Sorcery：28 `sorcery-device-mastery`、30 `sorcery-banish`、31 `sorcery-invulnerability`。
- Death：15 `death-genocide`、23 `death-darkness-storm`、28 `death-restore-life`、29 `death-mass-genocide`、30 `death-hellfire`、31 `death-wraithform`。
- Arcane：31 `arcane-clairvoyance`。
- Daemon：23 `daemon-doom-hand`、28 `daemon-summon-greater-demon`、29 `daemon-hellfire`、30 `daemon-send-to-hell`、31 `daemon-polymorph-demonlord`。

上述 ID 均省略公共前缀 `demo.ability.`。书仍可拥有/浏览这些条目，但不能学习或施放；不是从公共书中删除它们。

| 源规则 | 当前消费者与实施要求 |
| --- | --- |
| WIS，3 级起点；`xtra1.c:3399–3434` 先在 L<3 把最大/当前 MP 置 0，再令有效等级 `L−2`；`calc_mana_aux:3361` 使用 `adj_mag_mana × (有效等级+3)/4`，非零加 1，再做种族修正 | [player_abilities.rs](../crates/rfb-core/src/game/player_abilities.rs)的 RfbMana 直接使用 `L+3`，CastingProfile 没有此施法起点。第二步需让 MP 和学习公式共用真实起点，不能只在 UI 隐藏 1—2 级法术。Ranger 从 3 级起分子为 `L+1` |
| 负重 450/33/1000，手套无 FREE_ACT/MAGIC_MASTERY/正 DEX 时法力 75%；Ranger 无 CLASS_REGEN_MANA | 现有 encumbrance 可表达，资源恢复应为普通 100%，不能继承 Mage 的 200%。保留现有资源上限刷新/clamp 适配；核对换装、属性与负重实际 MP |
| `object1.c:_object_gives_esdm` 普通 EASY_SPELL/DEC_MANA 依赖 `CASTER_ALLOW_DEC_MANA`，Ranger 没有；源还存在按固定神器身份的例外 | 当前失败/耗魔直接消费 EasySpell/ReducedManaCost 装备被动，未核对此资格。第二/五步需在实际被动消费及物品有效属性路径区分 Ranger，保留源明确神器例外；不能简单禁止装备或从普通生成池删掉物品 |
| `do-spell.c:202` Ranger beam chance 为 `floor(L/2)` | 现有 beam multiplier/divisor 可表达 1/2；不是 Mage 的 L 或 High-Mage 的 L+10 |
| `do-spell.c:3609,6041` 熵之法球/地狱之焰 Ranger 等级伤害为 `L+floor(L/4)`，基础 3d6，半径 L<30 为 2、否则 3 | 现有两个 ability 的等级缩放为 `floor(3L/2)`。使用 Ranger realm override 的 levelScaling 修正，两项均可达，不另复制 program；保留半径及其他效果 |
| `lawyer.c` Death index 21 费用为 `base + clamp(base,50,100)`，封顶 250；与职业无关 | 当前 effective_casting_ability 已公共处理，复用。先导入 Ranger 基础费用，再做该调整，最后做熟练度费用，避免重复加算 |
| `spells3.c:spell_chance` 副领域 +5 只限 Mage/Blood-Mage/Priest/Yellow-Mage；Ranger 不在内 | 第三步公共化学习/保存时必须保留独立的失败率条件。Ranger 仍受骑乘、装备/状态、美德、WIS 最低失败率、眩晕、95 封顶及熟练度减免的源顺序影响；最低 5 不是最终绝不低于 5，Expert/Master 减免在后 |
| `mod_need_mana` 用当前主副熟练度做整数减耗；成功首用奖励及 `MAGIC_GAIN_EXP` 适用于 Ranger | 已有精确整数算式、首用字段和练习实现可复用。主副 cap、深度/难度练习、美德和 Death 失败反噬当前多处只在 `player_is_mage()` 分支生效，第三步须接 Ranger；不把“所有 Mage 专属规则”一起放开 |

本轮 Nature/Sorcery/Arcane 没有发现需要复制效果程序的 Ranger 专属分支；沿现有公共程序及其验证。Chaos、Trump 和其他非法领域中的职业分支不进入本轮。

## 3. 随机学习、熟练度、遗忘与改换

**结论：游侠不能重复研习已学法术，但必须记录共享学习支出。** `cmd5.c:697` 调用 `spell_okay(spell,FALSE,TRUE,realm,FALSE)`，`spells3.c:3454` 对已学项返回 `!study_pray`，因此随机候选排除已学和已遗忘项，还检查等级与种族领域资格。源后面的重复研习分支对该学习方式不可达。80 上限仍有意义：改换副领域清除旧进度，却不退还其学习支出。

| 行为 | 冻结后的源语义 | 当前差异 / 第三步工作 |
| --- | --- | --- |
| 共享容量 | `min(floor(adj_mag_study[WIS] × max(L−2,0)/2),80)`，再计 add_spells；额外 bonus 为 0。new_spells = allowed + add_spells + forgotten − learned_spells | 当前 RfbDualRealm 使用 L；DivineRandom 以 learned IDs 数量判满。复用 `spent_spell_learning` 与额外容量，但改为 Ranger 公式；不套单领域折半或 Mage 的 100 总上限 |
| 抽取 | 只遍历所选书本，按源 index 顺序对合格项执行 `one_in_(k)` reservoir sampling；没有候选不抽 RNG | 现有 `study_random_player_ability` 已用相同抽样形态，可保留。补共享预算、源资格和 paid spending；不要改为一次随机索引或跨两领域抽取 |
| 成功学习 | 追加一次学习顺序/支出，行动能量 100；按 `mp_ptr->spell_book` 增加 Faith，Ranger 此处不是 Nature | 当前 StudyPrayer 仅记学习事件，未接该美德变化。使用公共 virtue_add，缺少 Faith 槽位时不强行新增；无候选或前置拒绝不能多付回合/支出 |
| 重复/遗忘 | 无手动重复研习入口，也无手动遗忘退款。等级/容量损失按逆学习顺序遗忘，恢复按原顺序；已遗忘项不被重新随机抽取 | 当前 DivineRandom 的学习顺序排除可复用；手动 forget 和保存预算校验目前仅对 Mage 收紧，需覆盖 Ranger 的源限制 |
| 熟练度 | 主领域 Master=1600，副领域 Expert=1400；以成功施法练习提升。失败不按普通固定 gain 涨熟练度 | 当前 cap 和 grow_mage_spell/失败反噬入口限 Mage。复用难度、地牢/荒野层级、城镇不练习及无效攻击识别；不得用开放重复研习替代实际练习 |
| 改换资格 | `cmd5.c:item_tester_learn_spell` 允许职业第二领域候选书，必须有可用学习机会、光照、非失明/混乱，书在背包或脚下 | 当前 [spell_realms.rs](../crates/rfb-core/src/game/spell_realms.rs)只允许 Mage。Ranger 第一领域固定 Nature，候选只能是本轮四项；不能拿所有 realmProfiles 自动作候选 |
| 改换提交点 | 先选书、确认改换，清掉旧副领域 learned/worked/forgotten、熟练度及顺序，记录 old_realm；保留主领域与 paid spending；然后继续随机学习 | 拒绝确认无变化；确认后即使新书无可学项，改换仍成立，未成功学习不耗 100 能量。Ranger 没有 Mage 后续点选法术步骤。复用待确认书本状态并按随机流程继续，避免前端拼装规则 |
| 换回旧领域 | 旧领域的原学习/熟练度/首用记录不恢复，重新学习另付次数 | 验证旧副领域进度清理、历史去重、主领域保存及首用经验按清理后的记录重算；不能按出生 Build 覆盖当前领域 |

保存复用现有领域状态而不建另一套 Ranger 存档。当前状态命名为 `mage_realms` / `MageRealmsSaveDto`，相关身份限制散布在 `spell_realms.rs`、`player_abilities.rs`、[persistence.rs](../crates/rfb-core/src/game/persistence.rs)、[validation.rs](../crates/rfb-core/src/game/validation.rs)、[mod.rs](../crates/rfb-core/src/game/mod.rs)。实现时可将实际共用状态命名调整为双领域状态；以所需类型变化决定协议/保存/哈希版本，生成绑定，拒绝非法副领域、旧领域残留、无效顺序和支出，不能放宽保存完整性检查。

当前领域的直接消费者包括 active_casting_realm_profiles/book IDs、学习和施法投影、[mogaminator.rs](../crates/rfb-core/src/game/mogaminator.rs)领域谓词、[loot/allocation.rs](../crates/rfb-core/src/game/loot/allocation.rs)书本需求/发现次数、[town.rs](../crates/rfb-core/src/game/town.rs)领域公会资格。主领域奖励和出生美德继续读取固定主领域；改换不清空全局发现次数、不重置商店库存。已有 shop 初始配置和书本获取路径沿用公共适配。

## 4. 射击、树林与职业能力

| 源规则 | 当前消费者与缺口 | 验证要求 |
| --- | --- | --- |
| `ranger.c:3–14` 有 TV_BOW 装备时 `thb += 20+L`；这是发射器类别，不限 Arrow | [player_stats.rs](../crates/rfb-core/src/game/player_stats.rs)缺此职业加成，必须进入派生 ranged skill，供命中、射速及破损共用，不能只加面板命中 | 未装备/短弓/投石索/弩、成长前后、重弓 |
| `xtra1.c:5024` 公共基础射速提升后，Ranger 对非 TV_ARROW 设置 base_shot=100；`types.h:968` NUM_SHOTS=base_shot+xtra_shot | 现有解析支持公共技能射速、Sniper 折半、骑乘非箭上限和装备额外射速；新增 Ranger 的基值覆盖必须在装备额外射速前，不削掉 XTRA_SHOTS。源额外射速为 `15×pval` | 实际发射 energy，而非只查 base_shot；两件奖励弓要验证额外射速/倍率 |
| `heavy_armor()` 对普通 Ranger 返回 false，但 Tomte 头部重装可为 true | 当前基础射速只查 heavy_shoot，未查 Tomte 这一源例外。不能自行对普通 Ranger 加“穿重甲就失去箭射速”的规则 | 普通重甲仍影响 MP；Tomte 头部超重另验射速，修公共条件时回归直接调用者 |
| `cmd1.c:5179` Ranger 免树地形双倍能量，雪地修正仍独立；源普通角色可慢速穿树 | 当前 surface-tree 是 non-walkable/flyable，森林种族由 [movement.rs](../crates/rfb-core/src/game/movement.rs)及 [wilderness.rs](../crates/rfb-core/src/game/wilderness.rs)的 forest-adapted 判断放行，没有普通步行穿树的双倍能量路径。**保留现有地形公共适配**，本轮将 Ranger 接入通行并保持普通移动能量；不借本轮重写所有角色的树地形规则 | 未骑乘、相关坐骑、荒野/局部通行和雪地；明确不能穿任意墙。此适配不等于完整复刻原版普通职业慢速穿树 |
| `ranger.c` 能力 15/WIS/20/90；`spells_m.c:886` SPELL_NAME 为“探测怪物”，default SPELL_FAIL_MIN=0、energy=100 | 职业介绍使用“探查怪物”，但实际能力名应逐字采用“探测怪物”。复用 `demo.ability-program.sniper-probe-monsters` 与现有本地化；不能照搬 Sniper 的 INT/80/固定 HP 配置 | 14/15 级资格、成功/失败/无目标，正确名称及 WIS 失败率 |
| `spells.c:1449–1540 do_cmd_power` 以 HP+MP 判预算，优先扣 MP，不足部分扣 HP，失败也支付；与书本过度施法不同 | 当前 Class 能力要求全额资源，只有 Race/Mutation 走 innate spill。Ranger 探测必须接入现有先 MP 后 HP 的支付能力，并保留 Class 来源/能力 UI；不要把所有 Class 法术和技巧全局改为溢出支付 | MP≥20、MP<20但总量足够、总量不足、失败支付与保存；原生资源不足提示按真实 HP/MP 投影 |
| `spells2.c:2288 → cmd3.c:MON_LIST_PROBING` 只处理可见、非模糊、可投射到且非幻觉中的怪物；揭露伪装并更新 lore；空列表也成功 | [abilities/terrain.rs](../crates/rfb-core/src/game/abilities/terrain.rs)现有探查程序已处理这些过滤、伪装和怪物知识，可复用 | 隔墙/模糊/幻觉/伪装、知识恢复，无目标不得退款 |

`mspells1.c` 的源怪物施法 AI 将 Ranger 的施法倾向评分设为 20；本项目当前使用已有怪物能力决策与资源投影，没有复刻整套源职业概率评分。保留公共 AI 适配，不为本职业单独加第二套怪物决策系统。`r_poss.c` 的 Ranger 身体模板属于尚未开放的 Possessor 范围，不随普通游侠导入。

## 5. 生成、固定神器、任务及公会

| 源规则 / 消费者 | 当前差异与第五步工作 |
| --- | --- |
| `obj_kind.c:152` Ranger favorite 为发射器；`object2.c:_is_favorite_weapon` 用于 Tailored melee 候选 | allocation 当前只对 Archer 排除普通近战候选，Ranger 会误收。补 Ranger 的真实 favorite 资格；普通生成池、正常装备和使用近战武器不受此 Tailored 条件限制 |
| `object2.c:3574` 1/5 强制弓分支只有 Archer/Sniper；`_is_device_class:2429` 不含 Ranger | **不**给 Ranger 增加 1/5 弓抽取或 1/7 装置偏好；书本 needs-book 的 1/10 路径和当前双方高阶书需求继续复用。会使用装置不等于源生成偏好职业 |
| Ego/负向生成及完成品资格 | 沿现有 Ego/curse、种族槽位、手套及物品有效属性消费者；补 Ranger 条件的行为证据，不加一份职业专属生成允许列表 |
| `artifact.c:2256` 无外部主题的创造神器卷轴，进入 1/4 职业 bias 后 Ranger=(Ranger,30)；后续 Warrior 分支遵循源 RNG | [random_artifact.rs](../crates/rfb-core/src/game/random_artifact.rs)已有 Sniper 对应分支，增加 Ranger 身份即可。自然/显式主题生成不能一律强制此职业 bias；卷轴、主题与自然分别审计 |
| `q_old_castle.txt:215–219` RANDOM27%5 为 0→Belthronding，其余→Yoichi | [middle-earth.json](../packs/rfb-demo-original/worlds/middle-earth.json)缺 Ranger 1:4 奖励；[tasks.rs](../crates/rfb-core/src/game/tasks.rs)持久奖励选择/重复神器替代当前限 Duelist/Mage/High-Mage，必须接入 Ranger。保留已有任务选择 seed 适配，领取失败/途中动作不重抽 |
| `q_thieves.txt` 默认 long sword，Ranger 没有职业覆盖，快速 Mage 魔杖分支不适用 | 当前默认 broad-sword，仅 Mage 等特定覆盖 long-sword；普通游侠应增加源 long-sword 奖励，复用现有物品。尚未开放怪物种族的奖励覆盖不扩入本轮 |
| 源城镇 B:11 Ranger Owner | 六座正式城镇当前只有五个已开放弓手公会：Anambar、Angwil、Morivant、Telmora、Thalos；Outpost 没有该设施。五处均加入 Ranger，复用强化弹药和弓服务/定价，不按副领域改变职业资格；本步不新增 Outpost 设施 |
| Thalos B:8、Angwil B:8 Ranger Member | 对应 `thalos-sorcery-tower`、`angwil-mage-tower`，两者已有设施但缺 memberClassIds。Member 不是 Owner；源鉴定 200/1000，Ranger 使用非 Owner 价格。不要误连到 Angwil 内殿或 Morivant 咒术塔 |
| Morivant 咒术塔及其他按领域设施 | 当前 ownerRealmIds 由主领域和 current_second_realm_id 匹配。副 Sorcery 时满足资格，改换后立即改变，不能固定按出生组合；`town.rs` 已有匹配实现，需验证新职业当前领域接线 |

两件必要固定神器的完整源记录如下，均为长弓（tval/sval=19/13），没有 E 激活记录，也未发现 `ART_BELTHRONDING` / `ART_YOICHI` 的额外运行时专属分支：

| 源 ID | 权威中文后缀 | I/W/P | F 与导入要求 |
| --- | --- | --- | --- |
| a_info 124 `'Belthronding'` | `『贝尔斯隆丁』` | pval 4；等级70/稀有20/重量40/价值60000；AC0/倍率x3.00/命中20/伤害33/防御0 | DEX、STEALTH、HIDE_TYPE、RES_DISEN、XTRA_SHOTS、SHOW_MODS；pval 同时驱动 DEX/潜行和 60 额外射速，保留两行源描述 |
| a_info 148 `of Yoichi` | `与一的` | pval 4；等级50/稀有30/重量40/价值30000；AC0/倍率x4.00/命中40/伤害23/防御0 | DEX、HIDE_TYPE、SEE_INVIS、SHOW_MODS；倍率与附魔进入真实发射器路径，无额外射速、无激活 |

中文来自同一 ref 的 `localization/lib_edit_text_to_translate.tsv:420,489`（EDIT_00419/EDIT_00488）。`lib_edit_text_to_translate_unique.tsv` 对应行为空，不能拿空表覆盖已有中文；采用非空中文表逐字值，不自行另译。第五步以 `demo.item.belthronding` / `demo.item.yoichi` 导入，固定神器自然分配/唯一性/重复替代同时接入；Yoichi 没有源 D 描述，沿用正式长弓描述。

每个拟开放 Build 的五类审计记录分别包含基础分配/Tailored、Ego/负向生成、随机神器、固定神器/奖励、使用/保存；共同条件和既有行为证据复用，Ranger 实际缺口补齐后方可登记。第五步准备记录，第六步与普通入口同时纳入 [generation-build-applicability.json](../design/generation-build-applicability.json)，按[内容开发](content-development.md#职业与领域-build-的生成接入)生成报告和只读检查；本步不预填 completed。

## 6. 来源许可与保留的公共适配

本步核对了 `ranger.c`、`a_info.txt` 对应记录、中文表、`src/angband.h` 和 `src/artifact.c` 的通知。源 angband.h 保留 James E. Wilson 的教育、研究及非营利复制/分发条件，项目已在 [RFB-UPSTREAM-NOTICE.txt](../LICENSES/RFB-UPSTREAM-NOTICE.txt)保存该通知；artifact.c 另列 James E. Wilson、Robert A. Koeneke，已在 [NOTICE](../NOTICE)保留。两件 a_info 记录及 Ranger 职业文件没有单独的重新许可声明；中文表同样没有提供独立重新许可依据。

这些观察仅用于记录实际材料与保留通知，不表示默认 MPL-2.0/CC BY-SA 或其他旧批次结论自动覆盖新材料。第二/五步实际改编与导入时随来源记录维护 NOTICE；不擅自给源描述、名称或规则实现换许可证。本步只提交审计说明，没有复制整份上游内容文件。

明确保留：当前出生属性与 HP progression、整数装置 SP、公共书本法术先校验目标/资源的命令约定、当前资源上限刷新方式、树地形 non-walkable/flyable 适配、Mogaminator 替代源 `.prf`、现有商店配置、任务选择 seed 和怪物 AI。它们应在最终交付说明中与源职业规则区分。

**不作适配豁免的剩余缺口：** Tailored favorite 消费、卷轴 bias、两把奖励弓与任务/公会关联。职业/参数/出生/成长及学习/练习/失败/遗忘/改换保存已在第二、三步完成，射击/森林资格与探测MP→HP在第四步完成。

## 7. 后续退出证据

1. 第二步：四 Build 真正新游戏出生、双书与随机箭数、320 W/3 S 映射、1/2/3 级 MP/容量、WIS/负重/手套、职业美德及周期强感知；160 参数、15 个不可学条目、beam 和两项等级伤害。新类型才生成 Schema，内容变更更新包与 lock。
2. 第三步：同 seed 随机抽取/无候选不抽 RNG、不重复学习、成功 Faith/支出、改换后预算不退款、1600/1400 练习边界、无副领域 +5、遗忘/恢复、失败反噬；确认后无候选仍保留领域、历史换回和保存后相同后续动作。覆盖直接受影响 Mage/Paladin 路径，不机械扩张所有职业。
3. 第四步：发射器装备技能加成和实际行动能量、非箭100基值后仍加装备射速、重弓/Tomte边界、树林通行/雪地/骑乘、15级探测的实际 WIS 失败率、MP/HP支付、无目标/失败和知识保存。
4. 第五/六步：四个真实 Build 的生成与当前领域消费者、1:4旧城堡选择/两件固定弓使用和重复替代、普通盗贼奖励、五个已开放公会Owner/两塔Member；正式入口、书本随机学习/改换UI、完整来源审计生成检查及本地化/可访问性证据。
5. 第七步：新存档弓箭→3级双方学习施法→15级探测→改换与保存继续；明确所有经验、地图和物品准备。Tauri standalone 的 WebDriver 行为验收与优化 EXE 原生烟测分别记录，交付源码/程序/许可/校验值；不宣称自然高等级练级、通关或 Android 完成。

本步验证为：源提交与当前基线核对、320 W/3 S 分组、五领域160项参数比对及书内ID顺序验证、源调用链与消费者静态审计、文档链接/格式检查。未运行游戏测试、内容生成或桌面构建，因为本步未改变运行时行为。

## 8. 第二步实现与边界

正式 `classes/ranger.json`、`skillSets/ranger.json` 和四个 `ranger-nature-*` Build 复用已有物品和领域效果；玩家 actor 只沿用基础参数并提供游侠显示身份。160项参数按本轮源提交的 `N:4` 导入，另按领域书 rank/书内槽位独立逐项比对。15项 `99:0:0:0` 保留原值，内容校验仅允许该不可学哨兵采用零费用；探测是公共效果，解除其必须绑定狙击配置的旧校验，保留 Concentrate/SniperShot 限制。

`CastingProfileDefinition.firstSpellLevel` 默认为1，游侠为3；现有 MP 和学习成长使用有效等级 `max(L−2,0)`，游侠基础容量上限80、无领域bonus、无额外法力再生。重量450/33/1000、手套3/4与豁免、最低失败率5、beam `L/2` 和两项法球 `L+floor(L/4)` 进入现有消费者。普通 EasySpell/ReducedManaCost 按源资格排除；保留源 Namake Bow（182）的减耗例外判断，但该神器未导入，不宣称实物验收。

职业基础/成长、500/70/40近战、当前武器映射、双持/骑乘、美德 Nature/Temperance 和两路 Slow/Strong 感知已实现。武术熟练度仍为公共系统缺失；没有新增无人消费的字段。探测仅定义15级/WIS/20/90引用，MP→HP支付与行为验收在第四步完成；随机学习的完整共享支出、主副熟练度与领域状态仍属于第三步。

行为测试位于 [ranger.rs](../crates/rfb-core/src/game/tests/ranger.rs)：四组合真新游戏、双书/装备/箭数量、新存档及RNG往返、1—50级经验成长、1/2/3级MP/容量、四类种族交叉、熟练度/美德、强感知、重武器/负重/手套豁免、普通易施法/减耗排除、首用经验只奖励一次和等级伤害/beam投影。内容约束测试覆盖起点范围、零费用哨兵和主副领域资格。验证记录随本批提交说明保存；本步不进行前端或桌面验收。

## 9. 第三步实现与边界

`player_uses_dual_realm_learning` 只覆盖已实施的 Mage/Ranger，复用既有学习顺序、累计支出、当前/历史副领域及待确认书本。保存继续使用现有 `mageRealms` 字段和 `MageRealmsSaveDto`，没有双份状态或兼容分支；存档格式未变。Ranger 未改换前的支出必须等于学习顺序长度，改换后保留累计支出；历史上限按 `min(3×max(maxLevel−2,0),80)+额外容量` 和既有遗忘范围校验。非法领域、缺失/旧领域进度、重复顺序、超限熟练度、无效待确认书本与伪造支出均拒绝。

随机学习仍按 `cmd5.c` 的 reservoir sampling，成功只记一次支出和Faith，无候选/前置失败不抽取、不耗回合。禁止重复研习及手动遗忘；等级/属性损失按学习顺序遗忘，恢复原顺序。共享练习/失败/美德消费者使用当前领域，主领域上限1600、副领域1400；保留Ranger无副领域额外5%失败率，眩晕、95封顶及熟练度减免顺序。3级法术的练习难度高于Mage首级法术；源比例 `2000/(10+depth)` 需要足够深的环境才能到达Master，不直接复制法师深度20的测试预期。

确认改换先清除旧副领域学习/首用/练习并记录历史，再沿随机学习入口继续；成功花100能量。确认后的无候选或预算耗尽保留改换，未学习不耗回合；取消确认保留原状态。换回旧领域重新学习并重新取得首用经验，不恢复旧熟练度。当前领域自动进入已有书本、Mogaminator和设施消费者；本步未补第五步的职业生成/奖励/公会专属差异。

测试揭示初次审计只验证了源JSON书序，而编译器 `validation/abilities.rs` 又按ID重排书内条目，影响随机抽取和死亡反噬的源slot。现已去掉该重排，保留去重/引用校验；书内顺序成为编译后的实际语义，内容包与lock更新为1.424.0。没有复制Ranger专属书表，也没有刷新源参数。`spell_okay` 另调用的 `maia_forbids_realm` 只限制尚未实现的启蒙/腐化迈雅亚种；当前基础迈雅没有该亚种状态，本步不新增整套亚种系统。

新增 [learning.rs](../crates/rfb-core/src/game/tests/ranger/learning.rs) 和 [realm_change.rs](../crates/rfb-core/src/game/tests/ranger/realm_change.rs) 验证四组合双方书本实际学习/施法、源抽取顺序与RNG、早期拒绝/无候选、共享预算、Faith、遗忘恢复、主副练习/失败边界、死亡反噬和Nature's Wrath待选方向的保存提交点，以及改换确认/取消、旧领域换回、非法书本和保存完整性。连同第二步共25项游侠专项通过；相关回归及契约结果随本批提交记录。正式UI仍未开放，未宣称桌面或Android验收。

## 10. 第四步实现与边界

本步重新读取同一 `master` 提交的 `ranger.c`、`xtra1.c:4955–5035,6051`、`cmd1.c:5177–5209`、`spells.c:1449–1540` 与 `spells_m.c:886`。装备发射器时的 `20+L` 进入已有派生射击技能，因此命中、基础射速与弹药破损共同消费；未装备不加成，竖琴虽获得技能加成但没有普通弹药射击入口。非箭基础射速重设100，再处理装备额外射速。公共射击链原先只读取发射器自身额外射速，且重弓仍保留该值，现按源汇总全部有效装备并在重弓时清零。Tomte超重头具阻止技能提升基础射速，但不清空装备额外射速；普通游侠重甲只沿既有MP等规则，不阻止技能射速。

树林沿现有non-walkable/flyable适配，由同一个森林适应资格供局部/荒野通行消费，步行及陆生坐骑保持普通行动能量。雪地代价、飞行、穿墙和水生坐骑资格独立。最小移动测试发现坐骑进入树格后保存会被旧的种类地形校验拒绝；现只对当前坐骑复用真实移动资格，普通怪物仍按原校验。合法骑乘树格可恢复，伪造水生坐骑上岸或普通马留在树格仍拒绝。

探测沿Class来源、15/WIS/20/90配置和公共ProbeMonsters效果，仅该Class能力接入现有资源溢出支付。优先MP，余量扣HP，失败也支付；可见目标为空、幻觉或过滤后为空仍成功收费。既有Class支付点在效果后扣HP，能量为100；回合再生单独继续运行。投影保留基础费用20，同时给出当前实际MP费用与HP余量，已有UI可消费这两个字段；没有新增资源类型或保存状态，也没有改变其他Class技巧的支付规则。

[combat.rs](../crates/rfb-core/src/game/tests/ranger/combat.rs)覆盖1/25/50级短弓、投石索、轻重弩的实际发射能量，命中/破损消费者和射击后保存续演；额外射速、重弓、普通重甲、Tomte头具、骑乘与竖琴；树林实际移动/坐骑保存及非法位置拒绝；14/15级探测资格、WIS而非INT失败率、MP充足/不足/零、总量不足和恰好耗尽、失败/无目标、隔墙/模糊/幻觉/伪装、怪物知识及保存后相同动作/RNG。游侠32项和直接受影响的弓箭手/狙击手、骑乘、树种族/雪地、种族与变异支付、攻击投影等92项回归通过；格式、Clippy与当前契约结果随提交记录。

本步仅规则与测试/文档变化，内容包、lock、协议、保存格式和State Hash Schema不变。没有导入两件奖励弓；它们的实物额外射速/倍率验收属于第五步。普通创角、完整UI与Tauri standalone仍分别留在第六、七步，本步不宣称桌面已可玩验收。

## 11. 第五步实现与五类记录准备

实际来源仍为 `master` 的 `a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`，本批重新核对 `obj_kind.c`、`object2.c`、`artifact.c`、两件 `a_info` 记录及中文表、旧城堡/盗贼任务和城镇 B:11/B:8。两件固定弓、职业奖励和设施关联进入正式内容包 1.425.0，NOTICE 保留来源；没有新增类型、保存状态或 State Hash Schema。

基础分配只把 Ranger 加入已有近战 favorite 过滤，保留书本需求的 1/10 抽取；没有 Archer/Sniper 的 1/5 弓抽取和 Mage 的装置偏好。当前主副领域及高阶书发现数共同决定需求，改换后旧领域书不再满足 Tailored 条件，历史发现数不清零。Ego、负向属性与自然/主题随机神器复用公共流程；只有无外部主题的创造神器卷轴在 1/4 职业分支中使用 Ranger bias 和30%的 Warrior 转换。

长弓『贝尔斯隆丁』与与一的长弓保留源属性、长弓熟练度、自然固定池和唯一性登记。旧城堡按已有持久任务 seed 适配选择 1:4；中途 RNG 操作、失败领取及读档均不重抽，已生成过的固定弓替换为命名随机神器长弓，满背包失败不修改状态。盗贼奖励采用源普通长剑。弓手公会当前五处均为 Owner，Anambar/Angwil 弹药基价22，其余三处20，实际价格仍应用公共城镇价格；强化弓按现有计算费用。Thalos/Angwil 两塔仅为 Member，普通全包鉴定用非Owner基价1000，完成 appraised，不冒充完全鉴定。Morivant 咒术塔仍按当前领域，离开 Sorcery 后失去 Owner，旧低价实际被拒。

以下是第六步登记的五类共同证据与判定，适用范围是当前正式内容和已完成规则；深度80生成、奖励可领取状态等均为显式测试准备，不表示自然通关。完整函数名见对应测试文件。

| 正式 area ID | 登记判定与依据 | 实现/行为证据 |
| --- | --- | --- |
| `base-allocation-tailored` | implemented：favorite、书本/当前领域及种族槽位；无弓/装置额外偏好 | [allocation.rs](../crates/rfb-core/src/game/loot/allocation.rs) 的 `tailored_uses_playable_class_equipment_realms_and_birth_race` 与 `ranger_tailored_books_follow_current_realms_without_bow_or_device_preference_draws` |
| `ego-negative` | no-special-difference：没有 Ranger 专用 Ego 分支，现有槽位/手套、诅咒和装备资格继续生效 | [ego/applicability.rs](../crates/rfb-core/src/game/ego/applicability.rs) 的主题生成/装备/读档测试已扩至四组合；[scheduling/tests.rs](../crates/rfb-core/src/game/random_artifact/scheduling/tests.rs) 的自然 -2 诅咒神器生成、装备、卸下拒绝与继续生成 |
| `random-artifact` | implemented：自然与显式主题保持公共偏向，卷轴增加 Ranger/30 | [random_artifact.rs](../crates/rfb-core/src/game/random_artifact.rs)、[tests.rs](../crates/rfb-core/src/game/random_artifact/tests.rs) 的源抽取边界；[scheduling/tests.rs](../crates/rfb-core/src/game/random_artifact/scheduling/tests.rs) 的自然负向与无主题/显式Mage主题生成保存；[items.rs](../crates/rfb-core/src/game/tests/items.rs) 的实际神器卷轴使用涵盖四组合 |
| `fixed-artifact-reward` | implemented：固定弓、1:4、持久选择、唯一性与重复替代、普通盗贼奖励 | [tasks.rs](../crates/rfb-core/src/game/tasks.rs)、正式两件 item；[ranger/generation.rs](../crates/rfb-core/src/game/tests/ranger/generation.rs) 的 `rewards_keep_birth_selection_replace_unique_bows_and_fail_atomically` 及固定弓实际生成/装备/发射 |
| `use-save` | implemented：共享使用/拒绝、五公会两塔、当前领域铭刻/角色和保存续演 | [ranger/generation.rs](../crates/rfb-core/src/game/tests/ranger/generation.rs)、[ranger/combat.rs](../crates/rfb-core/src/game/tests/ranger/combat.rs)、上述 Ego/神器卷轴保存消费者 |

| 拟登记 Build | 主领域 / 初始副领域 | 五类记录范围 |
| --- | --- | --- |
| `demo.build.ranger-nature-sorcery` | nature / sorcery | 上表五类；Morivant 咒术塔初始Owner，改换后即时重算 |
| `demo.build.ranger-nature-death` | nature / death | 上表五类；需要双方高阶书，副领域可改换 |
| `demo.build.ranger-nature-arcane` | nature / arcane | 上表五类；沿实际书本/发现数判定需求，副领域可改换 |
| `demo.build.ranger-nature-daemon` | nature / daemon | 上表五类；需要双方高阶书，副领域可改换 |

四组合的生成/奖励/服务检查均从真实 Build 新游戏开始，共用部分保留共同证据，不复制职业专属矩阵或另造生成器。第五步只准备记录，第六步已与普通创角入口一同写入正式审计输入并运行完整来源审计；没有改变 deferred-unavailable-build 身份规则。缺失混沌/王牌、尚未导入的 Namake Bow 和公共武术/树地形等边界仍按前文保留。

## 12. 第六步 UI 与正式入口

[character-creation.ts](../web/src/character-creation.ts)的共享嵌套菜单新增游侠四个第二领域选项，第一领域固定自然；描述和种族限制仍沿正式内容与现有入口校验。未选择第二领域时阻止开始，返回/切换页取消未确认草稿，已确认职业与种族继续沿现有规则重验。

[status-panel.ts](../web/src/status-panel.ts)按已有 divine-random 投影提供每本书一个“随机学习”按钮；命令继续使用共享 study-prayer，不在前端抽法术。低等级说明取自实际书本法术的最低等级；学习容量、熟练度、遗忘、探测资格和费用均消费核心投影。随机学习后按书本实例恢复按钮焦点，无候选时落在可聚焦书名。双领域说明与确认后果按学习模式区分，游侠不再显示法师可重复研习的说明；确认立即尝试随机学习，无候选时改换仍生效。

四个 Build 的20项记录已纳入正式[审计输入](../design/generation-build-applicability.json)，沿第11节共同证据运行完整来源审计并生成[报告](../design/ego-contract-audit.json)。当前70个正式入口的只读检查通过；没有手改生成报告、关闭不可用身份约束或宣称所有源功能等价。

Tauri standalone WebDriver 专项位于 [ranger.e2e.mjs](../web/e2e/ranger.e2e.mjs)，复现：在 `web` 执行 `npm run e2e:build`，再执行 `node e2e/tauri.e2e.mjs --ranger-ui`。中英文各从正式UI创建四组合人类1级角色；随后明确授予2/3/15级经验、安静照明场地和第三领域书，用真实经验损失/恢复检查遗忘。验证随机学习结果与主副1600/1400上限、低等级说明、探测资格、草稿/种族/职业切换、原生键盘焦点、创建/学习/确认忙碌锁、Esc取消、待确认保存和确认立即学习、已改换存档的原生导入。390px与200%布局和截图保留在 `test-results/ranger-ui/`。测试准备沿原Mage入口共享为 `prepare_spell_learning_e2e`，只在webdriver构建可用；普通构建拒绝调用。

本步只开放并验收UI，不声明自然练级、探测/双方施法的完整桌面实战、优化EXE交付或Android完成；这些仍由第七步闭环处理。内容包、协议、保存格式和状态哈希版本均不变，没有刷新fixture；具体检查结果随提交记录。
