# 战法师来源与消费者审计

审计日期：2026-09-12。对应[接入计划](warrior-mage-class-plan.md)第一步。代码基线为 `main` 的 `a7e9acd903567dc198a45935609811293f2c8001`；本步只修改文档，未新增Class、Build、规则、菜单或生成审计完成记录。当前12职业、94个入口及内容1.429.0、协议1.254、save header/payload14/21、State Hash Schema126、contract-v327保持不变。

唯一RFB来源是 `D:/codex/Frogcomposband/master` 的 `master` Git对象，实际提交 `a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`。所有原版文件和行号均指该提交，通过 `git show` / `git grep` 读取，未读取原版工作区文件。保留现有许可与来源声明；本步不分发新导入的上游内容。

## 1. 身份、范围与来源覆盖

全读 `src/warrior_mage.c`，解析 `lib/edit/m_info.txt N:6` 和 `s_info.txt N:6`，检索全部 `CLASS_WARRIOR_MAGE`、`Warrior-Mage`、`warrior_mage`，再追踪书本类型、Class flags、武器/施法、资源转换、生成和设施等间接条件。

| 来源 | 结论 |
| --- | --- |
| `warrior_mage.c:44`、`classes.c:69,301`、`defines.h:834` | 权威中文名“战法师”，英文Warrior-Mage，Class索引6；正式ID规划为 `demo.class.warrior-mage` |
| `tables.c:1727,1803–1806`、`py_birth.c:1956–1968` | 主领域固定Arcane，副领域可选另一魔法领域；不套用牧师阵营过滤。主副同领域不可选 |
| `py_birth.c:1358` | 位于已有混合职业分类。现有[创角目录](../web/src/character-creation.ts)的`hybrid`可承接；正式接入留第六步 |
| `mspells1.c:270–306`、`monspell.c:3840–3850` | 智能怪物对玩家的反魔法选招权重取20；玩家已有反魔法或暂时禁法时为0。是候选权重，不是固定20%最终施放概率 |
| `r_orc.c:384`及`r_info.txt`的40条`P:Copy:*:Warrior-Mage` | 兽人怪物种族的伪职业及怪物身体复制规则，不随玩家职业导入，也不据此开放怪物种族 |
| `effects.c`的warrior-mage注释、`wild_talent.c:196–197`、`mut.c:74–75` | 前者是共享攻击描述；后两者复用同一转换函数的其他调用模式，不代表已有玩家战法师入口 |

本批8个Build均以 `demo.build.warrior-mage-arcane-` 为前缀，后缀为 `life`、`sorcery`、`nature`、`death`、`craft`、`daemon`、`crusade`、`armageddon`。复用现有九领域36本书；Chaos、Trump、Law仍排除在本批之外。没有查到战法师专属出生种族禁配；沿用当前正式种族和组合约束。

## 2. 出生、成长、战斗与技能

| 源规则 | 当前接入点与实施要求 |
| --- | --- |
| `warrior_mage.c:47–60`：STR/INT/WIS/DEX/CON/CHR为+2/+2/-1/+1/0/+1；life105、base HP8、exp140、pets35 | 新增Class、skill set、玩家actor与Build；复用[初始化](../crates/rfb-core/src/game/initialization.rs)、[成长](../crates/rfb-core/src/game/progression.rs)、[宠物维持](../crates/rfb-core/src/game/pet_upkeep.rs)，不增加第二套HP/经验状态 |
| 基础dis/dev/sav/stl/srh/fos/thn/thb为30/36/34/2/18/16/56/50；每十级成长7/10/10/0/0/0/18/15 | 接入既有技能定义，验证实际等级、命中和投影；不能复制Ranger的远程成长16 |
| `warrior_mage.c:27–32`出生短剑、软皮甲各1，加双领域第一册 | 复用 `demo.item.short-sword`（kind53，23/10）、`demo.item.soft-leather-armour`（kind249，36/4）和实体书；沿公共食物/光源/种族物品合并，无额外职业药水、不预学法术 |
| `combat.c:292–294`近战次数525/70/30；`class_melee_mult`默认100% | [player_stats.rs](../crates/rfb-core/src/game/player_stats.rs)的`class_base_blows`已有计算但缺战法师分支。525表示5.25次的源上限参数，不是每回合固定5.25击；不得套用Priest的94%或Mage的400/100/20 |
| `virtue.c:251–254`固有Enchantment、Valour | [virtues.rs](../crates/rfb-core/src/game/virtues.rs)新增职业映射；种族/领域美德沿公共去重补足，改换副领域不重抽出生美德 |
| `warrior_mage.c:61–62`感知1 MED/WEAK、感知2 MED/STRONG；`dungeon.c:182–269`两类基数均20000 | [item_knowledge.rs](../crates/rfb-core/src/game/item_knowledge.rs)补职业资格和频率；保持WIS/Knowledge调整、背包筛选、混乱阻止。Knowledge≥100可令第一类变强；不照搬Mage第二类9000或Ranger双强 |
| `py_birth.c:2709–2722`建议点购16/16/9/16/14/10 | 当前普通Build采用六项13，再叠种族/职业/性格；保留公共出生属性适配，不新增未使用的点购系统 |
| `races_a.c:1180–1235`龙人变形倍率表无战法师，默认100% | 当前[player_stats.rs](../crates/rfb-core/src/game/player_stats.rs)默认档已合适，不因共享Mage学习获得80%。冬贝利、托姆特、幽灵、英灵等按实际武器/槽位/资源与治疗消费者验证 |
| 智能怪物反魔法权重20/0 | [monster_ai.rs](../crates/rfb-core/src/game/monster_ai.rs)当前只对Duelist作10/0覆盖；战法师需沿同一智能、玩家目标、反魔法效果分支补20/0，不影响普通怪物、其他法术或怪物互战 |

`s_info.txt:2000–2331`含320条W、3条S。W第一索引加19为tval；档位0/1/2/3/4对应0/4000/6000/7000/8000。`skills.c:1052–1067`使零武器出生值变为 `min(2000, 表内上限)`，S不执行这一补足。

| 武器组 | 普通出生初始→上限 |
| --- | --- |
| 发射器19 | sval0—62为2000→4000，sval63为2000→8000 |
| 挖掘工具20、长柄22 | 全部2000→6000 |
| 钝器21 | 默认2000→6000；双截棍sval4为2000→4000，sval40为0→0，sval63为2000→8000 |
| 剑23 | 默认2000→6000；小剑sval8、短剑sval10、军刀sval11为4000→8000；毒针sval32为2000→8000 |
| S非武器 | 武术0→4000、双持0→6000、骑乘0→4000 |

正式物品身份已核对为 `small-sword`、`short-sword`、`sabre`、`poison-needle`。使用[weapon_proficiency.rs](../crates/rfb-core/src/game/weapon_proficiency.rs)现有底材映射、武器技能突变和冬贝利优先级；毒针的特殊单击/命中规则仍由自身消费者处理，熟练度上限8000不赋予普通武器连击。尚无完整武术熟练度运行系统，S:0保留源记录；未导入的特殊sval不另造替代物品。

## 3. 九领域参数与身份映射

源 `m_info.txt:1462` 为 `I:SORCERY:INT:0x07:0:1:430`。本步按领域与rank1—4连接现有书本，每本八项，逐项校验ability及player binding引用，并与现有Mage（工艺用High-Mage）override顺序交叉核对：**36本书、288项不同法术ID全部对应，源槽位和T记录完整**。这证明身份和参数映射，不表示已经导入战法师参数或执行过288种效果。

T四项为最低等级/MP/基础失败率/首用经验乘数；写入 `firstSuccessExperience` 时用源最低等级乘以该乘数。书本后缀均属于 `demo.ability-book.`，目录为[abilityBooks](../packs/rfb-demo-original/abilityBooks/)。

| 领域 / R索引 | 32条T行 | rank1→4书本后缀 | 表内等级 |
| --- | --- | --- | --- |
| life / 0 | 1466–1497 | `book-of-common-prayer`, `high-mass`, `book-of-the-unicorn`, `blessings-of-the-grail` | 2—50 |
| sorcery / 1 | 1501–1532 | `beginners-handbook`, `master-sorcerers-handbook`, `pattern-sorcery`, `grimoire-of-power` | 1—48 |
| nature / 2 | 1536–1567 | `call-of-the-wild`, `nature-mastery`, `natures-gifts`, `natures-wrath` | 2—50 |
| death / 4 | 1606–1637 | `black-prayers`, `black-mass`, `black-channels`, `necronomicon` | 1—50 |
| arcane / 6 | 1676–1707 | `cantrips-for-beginners`, `minor-arcana`, `major-arcana`, `manual-of-mastery` | 1—50 |
| craft / 7 | 1711–1742 | `handbook-for-pupils`, `grade-holders-book`, `note-of-acting-master`, `spiritual-enlightenment` | 2—99 |
| daemon / 8 | 1746–1777 | `dark-incantations`, `immortal-rituals`, `demonthoughts`, `hellfire-tome` | 2—50 |
| crusade / 9 | 1781–1812 | `rites-of-initiation`, `ways-of-war`, `exorcism-and-dispelling`, `wrath-of-god` | 2—50 |
| armageddon / 11 | 1819–1850 | `book-of-elements`, `earth-wind-and-fire`, `path-of-destruction`, `day-of-ragnarok` | 2—50 |

奥秘槽0为1/1/20/4，槽31为50/140/80/200。表内唯一超过50级的是工艺槽31 `craft-mana-brand` 的99/0/0/0哨兵，保留源值。1级能学习主奥秘；只有咒术、死亡副领域在1级已有可学项，其余副领域最早2级。双书出生不等于双方立即都能学习。

表外还必须保留两条现有 `lawyer.c:10–30` 公共修正：[effective_casting_ability](../crates/rfb-core/src/game/player_abilities.rs)已实现死亡槽21 `death-vampirism-true` 的追加耗魔，以及生命槽23 `life-warding-true` 仅主生命可学。战法师始终主奥秘，所以后者运行时最低等级为99，即使源T行不超过50也不可在本批等级范围学会。

源 `do-spell.c:3609–3625,6041–6057,6677–6693` 的死亡熵之法球、恶魔地狱之焰、圣战神圣法球，战法师都用 `3d6 + L + floor(L/4)`，30级起半径3。现有共享定义可能用3/2成长，必须通过Class override表达5/4，沿已有override机制保留各自半径缩放。Chaos的法力爆发也有同类分支，但不在本轮领域内。`beam_chance`（200–207）为 `floor(L/2)`，不是Mage的L。

## 4. MP、学习、改换与保存

| 来源规则 | 当前调用链与接入要求 |
| --- | --- |
| INT，1级起施法；负重430/武器33%/惩罚分母1200；手套负重与降低耗魔资格 | `warrior_mage.c:10–24`、`xtra1.c:3264–3453`。复用现有RFB MP、手套与装备重量计算；不照搬Mage的100%武器权重、600分母或Ranger禁止降低耗魔 |
| MP基础 `adj_mag_mana[INT] × (L+3)/4`，非零+1，再处理种族/装备/负重 | 沿[player_abilities.rs](../crates/rfb-core/src/game/player_abilities.rs)公共公式与整数顺序。Class没有`CLASS_REGEN_MANA`，`dungeon.c:1277`恢复倍率1，对应100%，不是Mage的200% |
| 自主学习、重复研习；学习成功加Knowledge，不因当前副领域是生命而改Faith | `cmd5.c:674–681,734–776,795–814`按Class的SORCERY书类分流。`study_player_ability`当前重复研习和累计支出/Knowledge更新只认Mage，需把战法师纳入真实自主双领域消费者；Priest/Ranger仍随机学习 |
| 学习上限80加非LIFE书类4点，即通常84 | `xtra1.c:2933–2966`准确公式是 `min(floor(adj_mag_study[INT] × L / 2), 84)`，再加`add_spells`。4点增加的是上限，不是每级计算结果额外+4；可用数还加遗忘数量、减累计学习支出。当前profile以cap80和各realm bonus4表达即可 |
| 主副熟练度1600/1400，重复研习消耗一次累计支出 | `cmd5.c:734–776,1457–1515`。复用学习顺序、深度练习、首用经验和遗忘恢复；同时活动最多64槽，84是累计学习预算，不是84个同时持有法术 |
| 书本失败没有副领域固定+5，最低失败由INT表决定 | `spells3.c:3396–3416`的额外5%名单不含战法师；当前名单无需扩大。沿共同美德、震慑、练习、EasySpell等阶段；`virtue.c:690–730`的INT阵营失败惩罚上限5，不套WIS的10 |
| 副领域改换与保存 | [spell_realms.rs](../crates/rfb-core/src/game/spell_realms.rs)可复用当前/历史/待确认领域、实际背包/脚下书选择；确认先清旧副领域进度并记历史，保留主领域与已付学习支出，再让玩家选择法术。取消后续选法术不回滚已确认领域，也不自动随机学习 |
| 内容与存档资格 | [ClassDefinition](../crates/rfb-content/src/definitions/characters.rs)、[角色校验](../crates/rfb-content/src/validation/characters.rs)需限制主奥秘、合法非同名副领域。`player_uses_dual_realm_learning`目前仅Mage/Ranger/Priest；加入战法师后同步初始化、领域DTO和保存验证 |
| 累计支出校验 | `player_spell_memory_is_valid`当前自主重复研习上界只认Mage，其他双领域按学习数相等；历史容量默认100。战法师须采用自主研习约束及 `min(3 × max_level,84)` 的最高历史容量，加既有额外容量/遗忘余量，拒绝伪造超额支出与错配进度 |
| UI投影 | [snapshot.rs](../crates/rfb-core/src/game/snapshot.rs)的`can_study`重复学习同样只认Mage，需随规则更新；前端沿能力/书本投影显示，不复制职业名单或容量公式 |

符文无合法落点的取消前置当前在[casting.rs](../crates/rfb-core/src/game/abilities/casting.rs)只对Mage检查；战法师实际可达的单格符文也要沿同一效果确认取消/耗时语义。书本施法仍受真实书本、光照、混乱、恐惧、禁法等现有边界约束；两项职业能力则走下一节的不同入口。

## 5. 两项25级主动能力

`warrior_mage.c:3–8`两项均为INT、25级、标称MP费用0、基础失败50；名字逐字来自 `spells_h.c:320` 与 `spells_s.c:386`。拟使用 `demo.ability.warrior-mage-hp-to-sp`、`demo.ability.warrior-mage-sp-to-hp`，不复用随机突变ID。

| 能力 | 源CAST行为与必测后果 |
| --- | --- |
| **生命转法力** | `spells_h.c:359–378`先`take_hit(DAMAGE_USELIFE,L)`，再以返回的实际生命伤害整除5增加MP，截断至最大值。满MP仍先付生命；没有按MP缺口缩减生命损失。零实际伤害会提示转换失败，CAST仍返回true。HP低于L可能死亡，正好归零不按负HP死亡处理 |
| **法力转生命** | `spells_s.c:423–434`当前MP至少`floor(L/5)`才扣除该值并调用`hp_player(L)`；MP不足只提示转换失败，CAST仍返回true。满HP仍扣MP；恢复量经公共治疗修正与最大HP截断，不直接写成HP+L |

原版顺序必须保留：

1. `spells.c:1447–1544 do_cmd_power`检查能力资格、混乱/恐惧等，随后掷自然失败；标称费用0，因此通用预算检查不会预先拒绝低HP转换或MP不足转换。自然失败不执行转换，HP/MP不变，但正常耗时。
2. 成功后才执行内部资源变化。不能把L生命或L/5MP写成通用Class前置费用，否则会改变失败支付、低HP、MP不足和满池语义。调用者返回true的“没有转换收益”也不等于取消或免费动作。
3. `effects.c:6377–6657`的USELIFE跳过普通无敌、灵体减伤等防护，但仍经Transcendence先由MP吸收；返还的是剩余实际HP伤害。当前[damage.rs](../crates/rfb-core/src/game/damage.rs)已有这一最终伤害原语与`BelowZero`死亡策略，必须利用返回结果计算MP，不直接用L/5。
4. 原主动转换没有调用`spell_power`，不能受书本法术威力或降低耗魔把转换比例放大。治疗仍复用[apply_player_healing](../crates/rfb-core/src/game/mutations.rs)的已有突变/种族/截断；源公共治疗的其他差异见第8节。

`do_cmd_power`不检查装备反魔法、暂时禁法、地牢NO_MAGIC或狂暴来禁用这两项转换；不得因战法师有castingProfile就套书本禁法。当前Class能力的`blocked_by_dungeon_anti_magic`和共同混乱/恐惧处理可以表达这一区别，需真实验证。两项都是self目标；取消能力选择不进行失败掷骰、资源变化或回合结算。

失败率沿 `spells.c:1006–1057 calculate_fail_rate_aux`，不是书本熟练度公式：等级/INT、装备与突变、最低失败、震慑和95上限、EasySpell等按源阶段处理。当前 `class_ability_failure_percent`的EasySpell和震慑部分仅服务少数已有职业，战法师必须补入；魅力吊坠恰好有EasySpell，不能只验证裸装。源HeavySpell、未开放身份等共同差异不在这里制造职业特判来掩盖。

[mutations.rs](../crates/rfb-core/src/game/mutations.rs)的`resolve_periodic_sp_to_hp`和`resolve_periodic_hp_to_sp`是1:1的周期转换，对应源SPELL_PROCESS，不能改成主动5:1规则。两个主动效果落在现有[restoration.rs](../crates/rfb-core/src/game/abilities/restoration.rs)职责范围，复用既有资源、伤害和治疗原语；不新增转换管理器或独立持久状态。效果结果与UI必须显示真实损失/收益及失败/不足原因，不能只有“费用0”。类型变化时再生成Schema/协议绑定。

## 6. 生成、奖励、设施与必要神器

| 来源规则 | 当前状态与后续接入 |
| --- | --- |
| `obj_kind.c:97–162`favorite默认近战武器和弓均可 | [allocation.rs](../crates/rfb-core/src/game/loot/allocation.rs)、[mogaminator.rs](../crates/rfb-core/src/game/mogaminator.rs)的普通分支可复用；不套Priest祝福、Ranger仅弓或Cavalry骑乘武器过滤 |
| `object2.c:2429–2454`战法师不是device class | 能使用装置不等于Tailored装置偏好。保留普通装置获取，不加入Mage/High-Mage的装置候选或额外1/7类别抽取 |
| `object2.c:2456–2557,3441–3495,3654–3661` | Tailored沿身体槽位、当前双方领域的高阶书、发现次数与1/10 needs-book；Arcane不在优质/Tailored高阶书候选中，不能为主奥秘改写源名单。主奥秘仍参与源needs-book判断，候选过滤与类别意图分别验证；改换副领域保留发现记录并切换当前书本消费者 |
| `autopick.c:848–868`、`object2.c:3747–3752` | 手套的FREE_ACT/MAGIC_MASTERY/正DEX豁免由现有`item_has_glove_encumbrance`承接，Tailored成品及已知不适用提示需复验。未知属性不可被UI预先揭露；不能把不适用物品从普通池删掉 |
| `ego.c`、`artifact.c`及共享身体/幸运/主题条件 | 未查到额外战法师Ego材质或重量分支；保留真正影响结果的种族槽位、坏运/好运、Chance、美德、诅咒与主题。不存在“所有生成都等同Mage”的结论 |
| `artifact.c:2265–2269`神器卷轴 | Mage bias，后续Warrior转换20；[random_artifact.rs](../crates/rfb-core/src/game/random_artifact.rs)当前职业名单需补战法师。怪物掉落主题与玩家Class偏向仍是两条调用路径 |
| `q_thieves.txt:39`普通盗贼任务 | 源默认长剑。当前世界任务默认阔剑，只对Mage/Ranger覆盖长剑；战法师须新增 `demo.item.long-sword` 职业奖励，不能误领 `broad-sword` |
| `q_orcs.txt:55–73`兽人营地 | 战法师没有专属覆盖，当前有荒野模式沿 `demo.item.frost-ball-wand`；不复制Priest/Berserker钝器奖励。源无荒野替代不是本批模式 |
| `q_old_castle.txt:406–410` | 出生持久选择神器22/219，权重1:4。[tasks.rs](../crates/rfb-core/src/game/tasks.rs)需将战法师加入持久选择与已生成神器替代条件，世界Class override配置两件正式实物；失败领取/读档/推进其他RNG不重抽 |
| `t_angwil.txt:127–137`、`t_thalos.txt:196–206` | [安格维尔法师塔](../packs/rfb-demo-original/townFacilities/angwil-mage-tower.json)、[萨洛斯巫术之塔](../packs/rfb-demo-original/townFacilities/thalos-sorcery-tower.json)新增Warrior-Mage Member。源鉴定价格Owner200、其他1000；Member不是Owner，不因此获得200价格 |
| 按领域资格的其他设施 | [town.rs](../crates/rfb-core/src/game/town.rs)已消费主领域和`current_second_realm_id`；生命副领域影响现有生命寺庙，咒术副领域影响Morivant塔，改换后重新投影。两座Class Member塔不随副领域丢失资格 |

### 两件必要奖励的身份和依赖

本步按正式item的 `artifactGeneration.sourceIndex` 全目录核对：22与219均缺失；底材37/20也缺失，40/0已存在。所有名称均有权威中文来源，无需自行翻译。

| 项目 | 原版身份、数值与正式接入要求 |
| --- | --- |
| 神器22 `of Lohengrin` | `a_info.txt:300–308`；中文表`artifact_name_zh.inc:29`为“罗恩格林的”，底材中文为“秘银链甲”（`kind_name_zh.inc:294`）。源37/20、pval4，等级80/稀有度9、重量150、价值135000；基础AC28、命中-1、附加AC20，INT/WIS/潜行、看隐形、持久生命与酸电火冷毒/冥界/黑暗/恐惧抗性。拟用`demo.item.lohengrin`，显示名按源底材/神器名称组合规则生成 |
| 底材287 `Mithril Chain Mail` | `k_info.txt:2053–2058`，37/20，中文“秘银链甲”；等级55、重量150、价值7000、AC35、命中-1，分配55/4、四元素忽略破坏。拟用`demo.item.mithril-chain-mail`。保留底材和神器各自AC，不用已有其他重甲替代 |
| 神器219 `Charmed Pendant` | `a_info.txt:2360–2367`，中文表226行为“魅力吊坠”；40/0、pval2，等级50/稀有度50、重量2、价值100000、AC+5。INT/CHR/红外/搜索、看隐形/自由行动/慢消化/再生/光/警告、三元素光环、EasySpell/MagicMastery；FULL_NAME。拟用`demo.item.charmed-pendant`，复用已存在底材`demo.item.amulet`（kind311） |

神器22激活为 `HEAL_CURING_HERO:50:300:777`；经 `devices.c:2466–2488 effect_parse`核对，格式是**效果:等级:冷却:额外量**，所以power/难度50、源冷却300、治疗额外量777。这里的777覆盖默认治疗公式，不是加在默认300之上。神器219为 `RESTORE_MANA:50:777`，power/难度50、冷却777。`devices.c:4933–4970`已有准确效果来源，项目已有Ego/随机神器的天使治愈组合以及 `RestoreResourceFull`。复用原语并保留本神器参数，不复制Ego的900冷却/80难度；按现有源回合到tick约定分别表达300和777冷却。

天使治愈还涉及清理失明/流血/混乱/震慑、减毒、解除狂暴、英雄状态和minislow减少1；恢复法力还给背包魔杖/法杖25%、魔棒50%充能，跳过恢复法力装置，并解除狂暴。当前[item_use.rs](../crates/rfb-core/src/game/item_use.rs)分别有资源满恢复与`RechargeCarriedDevices`原语，不能假设`RestoreResourceFull`自动执行后者。现有Ego天使治愈组合使用完全清毒、固定治疗公式且省略部分后果，也不能原样拷贝；第五步需按新神器的真实效果组装并记录公共未表达项，不能仅用“加血/加满蓝”验收。神器219的EasySpell需与两项职业能力实际失败率联动验证。

首次导入遵循现有固定神器/底材来源、锁和许可流程；普通自然生成、任务领取、装备、激活冷却与保存唯一性均为第五步范围。没有新的上游授权判断或自动移交其他方向；若出现真实共享写入，再协调责任。

## 7. 每个Build的五类审计责任

第六步开放8个入口时，对前述8个完整Build ID分别登记下列五个area，共40项记录。现在只列责任，**不把尚未实现的消费者标为implemented，不修改现有94入口报告**。

| area ID | 应复用的条件与必须补齐的行为证据 |
| --- | --- |
| `base-allocation-tailored` | 共享类别/身体槽位/favorite/device-class/needs-book/高阶书发现条件；真实8个身份核对非装置偏好、主奥秘与当前副领域、手套成品拒绝，代表场景实际生成/装备并在读档后继续生成 |
| `ego-negative` | 复用主题、幸运/坏运、槽位和诅咒链；8个身份覆盖生成真实负向物品、装备/禁止卸下及保存，不能只检查Class字符串或引用旧六职业测试 |
| `random-artifact` | 职业卷轴Mage/20边界；真实卷轴选中实例、命名、知识/属性与保存后后续RNG，区分自然/怪物主题/玩家卷轴 |
| `fixed-artifact-reward` | 长剑和普通兽人营地奖励；两件新神器的出生1:4、两结果领取、重复替代、满背包事务、重复领取拒绝、装备/实际激活/冷却和保存 |
| `use-save` | 生成物的实际使用、当前领域自动铭刻与设施资格、普通收费；两项转换与神器效果后保存恢复，再执行相同行动得到相同结果和状态哈希 |

既有责任入口为[生成适用性输入](../design/generation-build-applicability.json)的五个sharedReviews，流程见[内容开发](content-development.md#职业与领域-build-的生成接入)。使用现有生成/物品/领域/任务测试作为起点，只补真实缺口；参数/身份用数据遍历，资源转换、奖励和领域差异用实际动作代表，避免八份完整战役或静态矩阵测试。新用例应在写入实现后才作为已存在引用登记。

## 8. 适配与第一步结果

本批应直接补齐的遗漏：自主重复学习与84历史容量校验、25级内部转换支付、EasySpell/震慑的职业能力消费者、20/0智能怪物反魔法权重、三种法球5/4成长、长剑任务覆盖、两件神器与秘银链甲、两塔Member、40项生成记录和正式UI。不能因已有函数/配置就宣布这些已完成。

继续沿用并明确记录的公共适配：固定Build基础属性和现有成长曲线、统一背包/装备知识和感知容器、已有回合/tick与怪物AI选择模型、尚未完整表达的普通过度施法/疲劳、武术熟练度、源特殊防护/未开放身份。源`hp_player_aux`还按Vitality美德修正治疗并增加低HP治疗的Temperance，当前共享`apply_player_healing`未表达这两项；本职业使用同一治疗原语并保留这个共同差异，不复制仅战法师生效的治疗公式。现有种族治疗比例、治疗突变、最大HP截断、Transcendence与实际死亡等已表达消费者必须验证，不能归入未实现豁免。

本步实际完成的静态核对：

- 原版master提交与工作区基线确认；直接职业引用和间接学习、装备、生成、任务、设施调用链核对。
- 用只读Python脚本解析Git对象，核对36书/288项参数槽位、ability和player binding引用及既有override顺序，全部通过；表内99级哨兵与表外生命限制分别记录。
- 解析320条W/3条S并核对正式特殊武器身份；按sourceIndex确认两件神器及秘银链甲缺失、出生装备和护身符底材已存在；核对权威中文表。
- 核对当前40条未来生成责任对应的五类公共条件；本步未新登记入口、生成报告或执行游戏测试。
- 文档本地链接和diff空白检查通过。本步没有Rust/前端编译、运行时回归、Schema/协议生成、内容锁刷新、契约刷新、桌面构建或试玩；这些不属于纯来源审计验收。

第一步已完成。本文保留审计时的代码基线与缺口；后续实际完成范围以[接入计划](warrior-mage-class-plan.md)和提交说明为准。没有未决中文名或职业规则选择；第七步负责统一实战与桌面交付，高等级测试准备须明确记录。
