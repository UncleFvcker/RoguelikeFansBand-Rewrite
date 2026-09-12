# 牧师来源与消费者审计

审计日期：2026-09-12。对应[牧师计划](priest-class-plan.md)第一步，审计代码基线 `89910d869`（其父集成基线 `6d028028b03770802c2e7a27e1ad59bcb60d25c3`）。以下“当前”“待新增”等消费者状态均指第一步审计时，彼时Priest Class、Build、职业能力及普通入口尚未实现；后续实现进度见计划和[状态页](status.md)。第一步只修改文档，内容1.427.0、协议1.253、State Hash Schema125、save header/payload14/20和contract-v326不变。

唯一RFB来源为 `D:/codex/Frogcomposband/master` 的 `master` Git对象，实际提交 `a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`。下文原版路径与行号均指该提交。通过 `git show` / `git grep` 读取，未依赖原版仓库当前检出的文件。保留既有许可证与来源声明；本次不新增上游内容分发。

## 1. 核对范围与身份

全读 `src/priest.c`，解析 `m_info.txt N:2`、`s_info.txt N:2`；搜索全部 `CLASS_PRIEST`、`priest_is_good/evil`，追踪Class flags、LIFE学习类型、主副领域、技能/武器、出生、奖励、城镇角色及共享生成消费者。当前包的怪物牧师、Priest/PriestEvil掉落主题和未祝福武器辅助函数均不代表玩家职业已接入。

`priest.c:83–94` 以**第一领域**决定善恶：生命/圣战为善，死亡/恶魔为恶。第二领域改换不会改变阵营或职业能力。`tables.c:1721–1722,1795–1798`、`py_birth.c:1956–1968`、`cmd5.c:552–569` 同时约束出生与书本改换：禁用相反阵营和相同主副领域。

本轮稳定ID规则为 `demo.build.priest-<first>-<second>`，下表完整表达24个组合；此时只规划ID，不写正式Build或审计完成标签。

| first | second（每个后缀各一个Build） | 职业能力分支 |
| --- | --- | --- |
| `life` | `sorcery`, `nature`, `arcane`, `craft`, `crusade`, `armageddon` | 善良：祝福武器 |
| `crusade` | `life`, `sorcery`, `nature`, `arcane`, `craft`, `armageddon` | 善良：祝福武器 |
| `death` | `sorcery`, `nature`, `arcane`, `craft`, `daemon`, `armageddon` | 邪恶：驱散敕令 |
| `daemon` | `sorcery`, `nature`, `death`, `arcane`, `craft`, `armageddon` | 邪恶：驱散敕令 |

现有九领域36本书可复用，包含工艺。原版另允许的Chaos、Trump、Law不在本批领域范围内，不因牧师上线顺带扩展其他职业。未发现Priest专属出生种族禁配；沿当前种族入口和组合约束，不套用Duelist的冬贝利禁配。`r_angel.c:280`与`r_info.txt P:Copy:*:Priest`是怪物身体/伪职业，不随玩家Priest导入。

## 2. 职业、出生、成长和技能

| 来源规则 | 本轮接入与当前消费者 |
| --- | --- |
| `priest.c:112–129`：STR/INT/WIS/DEX/CON/CHR为-1/-3/+3/-1/0/+2；life100、base HP4、exp120、pets35 | Class、skill set与玩家身份待新增；复用[成长](../crates/rfb-core/src/game/progression.rs)、[初始化](../crates/rfb-core/src/game/initialization.rs)和[宠物维持](../crates/rfb-core/src/game/pet_upkeep.rs)，不新建Priest专用经验/生命状态 |
| dis/dev/sav/stl/srh/fos/thn/thb基础25/28/40/2/16/8/48/35，每十级成长7/11/12/0/0/0/13/11 | 进入现有技能参数和成长链，验证实际等级后投影与战斗，而非只检查JSON |
| `priest.c:97–102`出生权杖、长袍、治愈药水各1件，加两领域第一册 | 已有 `demo.item.mace`、`robe`、`healing-potion`；药水为tval75/sval37/source430。保留食物、光源、种族合并等公共出生规则，不把恢复经验等级药水当成治愈药水；不预学法术 |
| `combat.c:238–243`近战blows为500/100/35 | [player_stats.rs](../crates/rfb-core/src/game/player_stats.rs)已有同参数的Mindcrafter计算，当前职业分支不含Priest；复用 `class_base_blows` |
| `combat.c:98–109`、`cmd1.c:1577–1578,2800`：Class近战伤害倍率94% | 当前 `player_melee_damage_percent`只取种族倍率。须接入职业倍率并保留源组合/取整顺序，同时检查普通武器和天赋攻击、预览与实际伤害；不能改成命中率94%，也不能影响射击 |
| `virtue.c:231–233`固有Faith、Temperance | [virtues.rs](../crates/rfb-core/src/game/virtues.rs)补正式职业身份；种族/领域美德沿现有去重和补足，改换副领域不重新抽出生美德 |
| `priest.c:130–131`感知1 FAST/WEAK、感知2 MED/STRONG；`dungeon.c:182–269`基数9000/20000 | [item_knowledge.rs](../crates/rfb-core/src/game/item_knowledge.rs)当前只接Mindcrafter/Mage/Ranger。复用WIS/Knowledge调整、等级提速、装备/背包筛选及混乱阻止；第一类可因Knowledge≥100转强。不得把牧师改为双强感知或拾取自动鉴定 |
| `py_birth.c:2695–2705`推荐点购16/11/16/16/14/8；`xtra1.c`公共HP成长 | 沿当前公共出生属性与成长适配，未实现原版点购和完整额外HP曲线；不新增无人消费的推荐属性状态 |
| `races_a.c:1198–1211`龙人变形牧师落100%档 | 保持现有默认分支；不得因共用Mage代码获得80%。幽灵、冬贝利、托姆特等真实种族消费者按实际交叉影响覆盖 |

`s_info.txt:672–1001`共320条W与3条S。W第一索引加19得到tval，档位0/1/2/3/4表示0/4000/6000/7000/8000；`skills.c:1052–1067`使零武器起点变为 `min(2000, 表内上限)`。源完整分组如下：

| 武器组 | 正常出生初始→表内上限 |
| --- | --- |
| 发射器tval19 | sval0—62：2000→4000；sval63：2000→8000 |
| 挖掘工具tval20 | 全部2000→4000 |
| 钝器tval21 | 默认2000→7000；sval4双截棍2000→4000；sval40为0→0；sval63为2000→8000；sval5权杖、21巫师法杖为4000→7000；战锤是sval8，初始2000。第二步按kind身份复核时校正了原先将sval21写成战锤的笔误 |
| 长柄tval22、剑tval23 | 表内均2000→4000；实际 `skills_weapon_max` 对善良保持4000、邪恶改为6000 |
| 非武器S | 武术0→4000、双持0→4000、骑乘0→2000；起点不套用W的2000补足 |

[weapon_proficiency.rs](../crates/rfb-core/src/game/weapon_proficiency.rs)已有每种底材映射、突变与冬贝利规则。牧师动态上限必须同步实际使用、投影与保存验证；`skills.c:281–325`中武器技能突变先于牧师上限，而牧师刃器上限早于冬贝利军刀特例，不能随意交换优先级。祝福解除刃器不适用，不改变这条熟练度上限。双持/骑乘沿现有公共表达；尚无完整武术熟练度系统，S:0只保留来源边界。

## 3. 九领域参数与书本映射

读取 `m_info.txt N:2` 的 `I:LIFE:WIS:0x04:1:1:430` 和各R/T行。本次把每领域四本书按rank1—4、每本内部八个ID连接，逐项核对现有Mage参数顺序；工艺核对High-Mage参数顺序。**36本书、288个不同法术ID及其正式ability/binding引用全部吻合，每项已关联源R索引、0—31槽位和T行。** 这证明身份/参数映射，不表示运行了288个法术效果或已经导入牧师override。

表内书名为 `demo.ability-book.` 后缀，均位于[正式书本目录](../packs/rfb-demo-original/abilityBooks/)。槽位为 `8 × (rank - 1) + 书内下标`；T四项依次为等级/MP/基础失败率/首用经验乘数，正式 `firstSuccessExperience = 等级 × 乘数`。不把乘数直接写成经验值。

| 领域 / R索引 | 32条T行 | rank1→4书本ID后缀 | 等级范围 |
| --- | --- | --- | --- |
| life / 0 | 443–474 | `book-of-common-prayer`, `high-mass`, `book-of-the-unicorn`, `blessings-of-the-grail` | 1—45 |
| sorcery / 1 | 478–509 | `beginners-handbook`, `master-sorcerers-handbook`, `pattern-sorcery`, `grimoire-of-power` | 2—48 |
| nature / 2 | 513–544 | `call-of-the-wild`, `nature-mastery`, `natures-gifts`, `natures-wrath` | 2—44 |
| death / 4 | 583–614 | `black-prayers`, `black-mass`, `black-channels`, `necronomicon` | 1—45 |
| arcane / 6 | 653–684 | `cantrips-for-beginners`, `minor-arcana`, `major-arcana`, `manual-of-mastery` | 1—50 |
| craft / 7 | 688–719 | `handbook-for-pupils`, `grade-holders-book`, `note-of-acting-master`, `spiritual-enlightenment` | 1—99 |
| daemon / 8 | 723–754 | `dark-incantations`, `immortal-rituals`, `demonthoughts`, `hellfire-tome` | 2—50 |
| crusade / 9 | 758–789 | `rites-of-initiation`, `ways-of-war`, `exorcism-and-dispelling`, `wrath-of-god` | 1—46 |
| armageddon / 11 | 796–827 | `book-of-elements`, `earth-wind-and-fire`, `path-of-destruction`, `day-of-ragnarok` | 3—50 |

唯一超过50级的本轮条目是工艺槽31 `demo.ability.craft-mana-brand`：**99/0/0/0**（源719行），是不可用哨兵，不能改成50级、删掉、免费学会或让零费用校验误拒整个Class。恶魔主领域最早2级才有祈祷；1级有法力/学习容量不等于每个Build都能学习出生主书。

生命槽0 `life-cure-light-wounds`为1/1/10/4，死亡槽0 `death-detect-unlife`为1/1/20/4；生命末项45/90/85/250，死亡末项45/75/80/250。说明中的生命/死亡亲和落实为这些职业参数及实际效果分支，不额外虚构通用治疗或死亡伤害加成。现有公共绑定主要服务其他职业，必须用Class override，不能覆盖共享binding。

## 4. 施法、学习、失败与保存

| 源规则 | 当前消费者与实现要求 |
| --- | --- |
| WIS、1级施法；负重430、武器权重67%、惩罚分母800；允许降低耗魔，无手套负重flag | `priest.c:27–41`、`xtra1.c:3264–3453`；[player_abilities.rs](../crates/rfb-core/src/game/player_abilities.rs)已有RFB MP公式和encumbrance。复用，不能照搬Mage的手套限制、Ranger的450/33/1000参数 |
| MP按 `adj_mag_mana[WIS] × (有效等级+3)/4`，非零+1，再沿种族/装备与负重 | `xtra1.c:3361–3445`；维持公共整数与恢复适配。Priest没有 `CLASS_REGEN_MANA`，`dungeon.c:1277`职业恢复倍率为1，不采用Mage的2 |
| 基础学习容量 `min(adj_mag_study[WIS] × 等级 / 2, 96)`，再加真实额外容量 | `xtra1.c:2933–2966`。Class的spell_book始终LIFE，即使主死亡/恶魔也没有非LIFE的+4；各realm profile的学习bonus为0。不能复用Mage的100上限或Ranger的80/3级起点 |
| 随机学习按书内顺序对合格候选依次 `one_in_(k)`，只选未学祈祷；成功学习+Faith | `cmd5.c:683–710`；复用 `study_random_player_ability`与共享支出。没有候选时不制造替代法术、不耗学习回合；不开放Mage重复研习或手动遗忘 |
| 主副熟练度1600/1400，首用经验、后续练习与遗忘恢复 | `cmd5.c:1490`附近与`xtra1.c:2980`之后；复用现有学习顺序、remembered集合和ability progress；源同时活动最多64槽，容量是累计支出额度，不是同时持有96个法术 |
| 祈祷失败：副领域+5，每把不适用刃器+25；按源阶段应用装备、美德、最低失败、震慑、95上限与练习减免 | `spells3.c:3396–3440`。当前 `ability_failure_percent`副领域只认Mage、没有刃器25；Priest的最低失败取属性表，没有Ranger固定5%。**两种职业能力走 `calculate_fail_rate`，不叠加书本的副领域或刃器惩罚** |
| WIS施法者的阵营失败惩罚上限10，其他施法属性上限5；生命/圣战偏邪、死亡/恶魔偏善和自然失衡分别按源阈值插值 | 第三步补充核对 `virtue.c:690–730`：源 `which_stat == A_WIS` 将上限从5改为10。原有共享 `book_spell_alignment_modifier`固定5，须同时修正Priest与Ranger；强烈同阵营的生命/圣战或死亡/恶魔仍减1。职业主领域的善恶资格与动态美德阵营是不同规则，不以动态美德改变职业能力分支 |
| beam基础为等级/2；公共领域效果用当前Class参数 | `do-spell.c:200–207`；现有profile可表达。神圣法球 `do-spell.c:6667–6693` 的Priest为 `3d6 + L + floor(L/2)`，30级起半径3；当前法术已配置3/2缩放，需验证实际投影与施放，不新添一层加成 |
| 改换副领域先清掉旧副领域学习/首用/练习/遗忘，保留主领域、旧领域历史与已付支出，然后继续学习 | `cmd5.c:573–601,660–717`；复用[mage_realms既有状态](../crates/rfb-core/src/game/spell_realms.rs)，不因字段旧名称复制Priest状态。确认后无候选仍保留改换，取消确认保留原状态 |
| 保存边界 | [角色内容校验](../crates/rfb-content/src/validation/characters.rs)、[spell_realms.rs](../crates/rfb-core/src/game/spell_realms.rs)、`player_spell_memory_is_valid`、[validation.rs](../crates/rfb-core/src/game/validation.rs)必须同时接线：限制当前/历史/待确认领域，随机职业未改换时支出与学习数一致，历史容量采用96而非当前非Ranger分支的100；拒绝伪造跨阵营、重复/非法历史、超额支出与错误能力状态 |

源 `xtra1.c:3186–3212` 按主领域可学项限制new_spells的分支只适用于没有第二领域的角色，本轮24个双领域Build不进入该分支。共享学习投影应按预算与实际书本候选表达，不能误加主领域候选数上限。反魔法、恐惧、混乱、无光/失明、MP不足施法等沿现有边界；源通用过度施法与疲劳细节未完全等价，职业能力的MP→HP支付则是本轮必须实现的行为。

## 5. 刃器、祝福与职业能力

### 刃器约束

`priest.c:44–81`：善良牧师使用未祝福剑/长柄时，每把武器-2命中/-2伤害并标记icky；实际判定不要求已鉴定。面向玩家的“已知不适用”判断才要求已鉴定。`obj_kind.c:108–116`的favorite读取**已知祝福**，两者不可混为一套泄漏未知特性的UI判断。`object1.c:768–790`的 `obj_learn_equipped`还为Priest学习装备的Blessed标志。

现有 `good_priest_weapon_penalty`及战斗路径已能扣值，但尚无正式Class；其调用检查出生的任意领域，需收敛为固定第一领域，检查是否存在技能层与to-hit层重复扣减，并接入实际双武器失败率。祝福标志既可来自定义/Ego，也可来自实例；当前 [state.rs](../crates/rfb-core/src/state.rs)已有 `intrinsic_weapon_traits`，[save.rs](../crates/rfb-core/src/save.rs)已有往返，优先复用。祝福知识应只暴露已知事实，不以整件鉴定代替得知一个属性。

### 两种职业能力

| 分支 | 源名与来源 | 源资格/费用 |
| --- | --- | --- |
| 第一领域生命/圣战 | **祝福武器**，`priest.c:5`、`spells_a.c:538–554` | 35级、WIS、70、基础失败90 |
| 第一领域死亡/恶魔 | **驱散敕令**，`priest.c:11`、`spells_c.c:1578–1601` | 42级、WIS、40、基础失败80 |

二者均经 `spells.c:1447–1544 do_cmd_power`：成功和自然失败都会花费，先MP、不足扣HP；预算不足拒绝，效果内取消不完成施放。当前 `ability_cost_spills_into_hit_points`只覆盖种族/突变和游侠探测，需要接入两项正式Class能力；费用拆分沿已有事件和投影。装备恐惧/混乱等沿该能力真实入口，不能把“是牧师”当作调用成功。

祝福动作按 `spells3.c:2873–3005`：

- 在背包、装备、脚下选择真实武器，取消不改物品。普通诅咒可去除；重诅咒 `randint1(100)<33`（32/100）或永久诅咒阻止本次祝福，并保留源执行顺序。
- 清咒后若本已Blessed则结束。普通武器通常必成；SLAY_GOOD令成功率1/3，KILL_GOOD令1/5，兼有SLAY_EVIL/KILL_EVIL恢复为1；特殊神器139/334覆盖为1/10。
- 成功将Blessed写入真实和已知属性，`discount=99`。抵抗时对正的命中、伤害、护甲附魔分别-1，减后仍>5再各有33%机会-1；顺序与随机消耗不合并成一个掷骰。
- 两件特殊神器139/334当前没有正式定义，记录缺失身份分支，暂不导入或替代；真实已存在的杀善/杀恶、诅咒、附魔和物品价值消费者须实现并验证。背包、地面、装备、保存、堆叠元数据都必须保留正确实例变化。

驱散敕令按顺序对视线目标执行：驱散伤害 `spell_power(4L + to_d_spell)`，恐吓威力 `spell_power(4L)`，放逐距离 `spell_power(4L)`。`spells2.c:1976,4591,4609`分别使用GF_DISP_ALL、GF_AWAY_ALL、GF_TURN_ALL；这里的放逐是**传送离开**，不是从世界删除怪物。复用[damage.rs](../crates/rfb-core/src/game/abilities/damage.rs)的VisibleDamage、[control.rs](../crates/rfb-core/src/game/abilities/control.rs)的可见恐惧/Banish及复合执行入口；逐阶段只处理仍存在的合法目标，保留唯一怪物/抗性/无落点规则与“无目标仍施放”的行为。

## 6. 生成、奖励与设施

| 来源与行为 | 当前消费者和待做接线 |
| --- | --- |
| `obj_kind.c:108–116`偏好钝器或已知祝福武器 | [loot/allocation.rs](../crates/rfb-core/src/game/loot/allocation.rs)的Tailored候选和[mogaminator.rs](../crates/rfb-core/src/game/mogaminator.rs)favorite谓词都需Priest。即使邪恶牧师战斗允许剑，也不能据此改变源favorite。现有定义标签不足以代替实例的已知祝福 |
| 普通/主题/Tailored的装置和书本 | 不添加Mage专属装置优先抽取。当前领域书本需求、发现次数、三四阶书限制复用公共分配；改换后更新书本偏好、自动铭刻和服务资格，不重置全局发现次数或商店库存 |
| `artifact.c:2244–2247`玩家神器卷轴 | Priestly偏向，后续30% Warrior转换；邪恶主领域不把这个Class switch改为Necromantic。`random_artifact.rs`当前无Priest玩家分支；已存在的“priest-evil”主题是另一条路径 |
| Ego/负向与固定神器 | 复用普通主题、已知/未知、属性与当前身份分支。祝福影响装备和被诅咒抵抗，但不能因此把所有不可用物品移出普通生成池；两件奖励神器无需重新导入 |
| `q_thieves.txt:88–89` | 盗贼任务奖励 `demo.item.war-hammer`，替代当前普通奖励；沿正式任务入口与库存事务 |
| `q_orcs.txt:69–70` | **兽人营地奖励HAFTED＋EGO(slaying)**。当前 `demo.task.anambar-orc-camp`只有Berserker覆盖，已有钝器权重/深度20/Ego适配可复用；Priest缺失时会错误落到通用魔杖奖励。这是本次审计补出的当前可达缺口 |
| `q_old_castle.txt:351–355` | `demo.item.aule` : `demo.item.palantir-of-westernesse` = 1:4，善恶相同。[tasks.rs](../crates/rfb-core/src/game/tasks.rs)的持久选择和已生成神器替代职业名单均缺Priest；新增Class奖励配置与两处接线。失败领取、读档和中途RNG不得重抽 |
| `t_ana.txt:169–175` | 阿南巴玛门神庙 `anambar-mammon-temple`：Priest Owner，善恶不限。已有设施/服务缺ownerClassIds；源治疗0/500、恢复500/2500、治疗突变10000/100000，按现有报价/变异筛选表达 |
| `t_angwil.txt:139–144` | 安格维尔内殿 `angwil-inner-temple`：Priest Owner，善恶不限。源治疗0/150、恢复300/1000；已有设施缺ownerClassIds |
| 当前按领域的寺庙/塔 | Morivant内殿、Telmora/Thalos生命寺庙依Life；Morivant咒术塔依Sorcery。[town.rs](../crates/rfb-core/src/game/town.rs)已匹配第一领域和current_second_realm_id，需新职业实际验证。Angwil/Thalos法师塔不因Priest加入而获得Ranger的Member资格；Trump塔仍受本轮未开放领域/既有种族资格约束 |

源奖励的HAFTED类别分配、旧城堡RANDOM27及重复神器替代使用当前已记录的生成适配；不能把1:4概率与保存稳定性误报为复刻源全部随机数序列。当前[世界定义](../packs/rfb-demo-original/worlds/middle-earth.json)已有上述三个任务和两处寺庙，不新增地点或改换任务链。

## 7. 每Build的五类审计责任

24个Build都要登记五类记录，复用下表共同条件/消费者证据，不复制120套行为测试。本步不修改[正式审计输入](../design/generation-build-applicability.json)或[生成报告](../design/ego-contract-audit.json)，也不将尚未开放身份标成implemented。

| 正式area ID | 关联的现有condition ID | 本轮应交付的证据 |
| --- | --- | --- |
| `base-allocation-tailored` | `tailored-favorite`, `tailored-device-class`, `tailored-compatible-kinds`, `needs-book`, `great-book-count`, `good-book-count`, `tailored-class-hooks`, `base-theme` | 善恶真实Build的偏好/装备；祝福知识边界；无额外装置优先；按当前领域分配、发现数与改换保存 |
| `ego-negative` | `ego-theme`, `bad-luck-randart`, `bad-luck-tomte-hat`, `theme-jewelry-power` | 共同Ego/负向规则配合真实种族槽位、诅咒/祝福装备、使用/拒绝及读档；主题与玩家Class分开 |
| `random-artifact` | `artifact-theme-bias`, `artifact-scroll-class-bias`, `artifact-scroll-virtues`, `theme-reset` | 自然/主题沿公共规则；玩家卷轴Priestly/30边界、实际使用和保存续演，恶领域不篡改卷轴Class分支 |
| `fixed-artifact-reward` | `fixed-artifact-identity`, `bad-luck-fixed-special`, `bad-luck-fixed-normal` | 旧城堡1:4、两件奖励的使用/唯一性/替代、失败原子性；盗贼战锤与兽人营地杀戮钝器的真实领取 |
| `use-save` | 上述生成条件的实际消费者 | 祝福前后武器、费用与失败、双领域改换后的铭刻/书本/寺庙服务、生成/装备与读档后相同行动和RNG |

当前conditionScopes没有Priest专属 `deferred-unavailable-build` 条目；现有 `implemented-current-builds` 的范围仍是70个已开放入口。新增Priest时扩展真实覆盖，不将不存在的豁免“直接关闭”。Mauler/Bard/Disciple等无关未开放身份依赖保留。第五步准备证据、第六步与普通入口一起登记并由工具重生成；每个Build的共同依据可复用，但其first/second身份和合法领域必须明确。

### 第五步准备记录（代码已写，全部待第七步运行）

以下后续记录对应同一来源提交 `a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`。测试从正式Class枚举24个Build并断言数量；每个真实身份都执行下列共同用例，不把怪物掉落主题等同玩家Class。正式审计输入/报告尚未将牧师标为implemented。

| 第一领域 | 完整Build ID（每项均关联下表五类） |
| --- | --- |
| life | `demo.build.priest-life-sorcery`、`demo.build.priest-life-nature`、`demo.build.priest-life-arcane`、`demo.build.priest-life-craft`、`demo.build.priest-life-crusade`、`demo.build.priest-life-armageddon` |
| crusade | `demo.build.priest-crusade-life`、`demo.build.priest-crusade-sorcery`、`demo.build.priest-crusade-nature`、`demo.build.priest-crusade-arcane`、`demo.build.priest-crusade-craft`、`demo.build.priest-crusade-armageddon` |
| death | `demo.build.priest-death-sorcery`、`demo.build.priest-death-nature`、`demo.build.priest-death-arcane`、`demo.build.priest-death-craft`、`demo.build.priest-death-daemon`、`demo.build.priest-death-armageddon` |
| daemon | `demo.build.priest-daemon-sorcery`、`demo.build.priest-daemon-nature`、`demo.build.priest-daemon-death`、`demo.build.priest-daemon-arcane`、`demo.build.priest-daemon-craft`、`demo.build.priest-daemon-armageddon` |

| area ID | 已准备的真实消费者与用例 |
| --- | --- |
| `base-allocation-tailored` | [allocation.rs](../crates/rfb-core/src/game/loot/allocation.rs)的`ranger_and_priest_tailored_books_follow_current_realms_without_extra_preference_draws`覆盖全部24身份的钝器/刃器/弓/装置候选、当前高阶书发现数、类别RNG及改换后实际行选择/保存；[items.rs](../crates/rfb-core/src/game/tests/items.rs)的`all_priest_builds_generate_tailored_hafted_weapons_equip_and_resume_generation`实际生成、拾取、装备并保存后继续生成。善恶祝福知识边界由[mogaminator.rs](../crates/rfb-core/src/game/mogaminator.rs)专项覆盖 |
| `ego-negative` | 全部24身份扩展[scheduling/tests.rs](../crates/rfb-core/src/game/random_artifact/scheduling/tests.rs)的`random_artifact_negative_power_reaches_a_cursed_equippable_instance`，实际负向生成、装备/诅咒拒绝和保存续演；兽人营地奖励实际物化slaying Ego并装备。既有主题、Tomte槽位与坏运算法沿公共测试，无新增牧师主题副本 |
| `random-artifact` | 全部24身份扩展`random_artifact_save_preserves_rejected_names_and_continued_generation`及[items.rs](../crates/rfb-core/src/game/tests/items.rs)的`artifact_scroll_keeps_selected_equipment_identity_properties_and_saved_name`，覆盖自然/主题、名称排除与后续RNG、背包/已装备真实卷轴；[random_artifact/tests.rs](../crates/rfb-core/src/game/random_artifact/tests.rs)的职业bias边界用例新增善恶Priestly/30，源PriestEvil怪物主题仍为Necromantic |
| `fixed-artifact-reward` | [priest/generation.rs](../crates/rfb-core/src/game/tests/priest/generation.rs)的全24身份盗贼/兽人营地领取和保存；四主领域旧城堡出生1:4、领取前推进RNG不重抽、两种实物装备/晶球激活、唯一性、重复替代、满背包原子性及重复领取拒绝 |
| `use-save` | 上述全24身份生成/装备/保存实际链路；[priest/generation.rs](../crates/rfb-core/src/game/tests/priest/generation.rs)的两寺庙全24身份Owner、实际治疗/恢复/净化突变费用及读档，领域改换后真实铭刻/服务资格；[priest/powers.rs](../crates/rfb-core/src/game/tests/priest/powers.rs)补祝福知识、费用和驱散传送/保存。领域间不同战斗行为按善恶代表及四主领域资格覆盖 |

第四/五步补充：`project_hack`（`spells2.c:1850–1880`）要求LOS及projectable，并不要求看见怪物；驱散三阶段按此筛选，RES_ALL免伤和提前跳过恐惧，放逐沿既有唯一/抗传送规则。装备仍沿公共“穿戴即鉴定”适配；祝福动作本身只揭示Blessed，不额外鉴定其他词缀。部分知识新增保存字段，失忆/平凡化清除，伪造无实际祝福的记录拒绝载入。源神器139/334成功率分支已按正式source metadata索引接入，但两身份未导入，因此没有其实际使用验收。

## 8. 适配、排除项与后续验证

**沿用的公共适配：** 当前生命/经验/基础属性模型，统一背包的感知抽样、离散恢复/回合能量，现有技能/双持/骑乘和法术目标模型，任务类别分配与持久选择。没有完整源武术、疲劳、全部怪物身体/伪职业或全部神器身份；`monspell.c:5002–5016`的Priest身体治疗费用75%只属于怪物身体体系，不给玩家所有治疗减耗25%。

**必须补齐，不能作为适配豁免：** 24个Build、288参数、96学习上限/保存支出、善恶所有入口校验、副领域5%、刃器-2与25%、熟练度优先级、94%近战倍率、两种职业能力及MP→HP、源祝福状态/知识/费用、三个任务、两寺庙Owner和五类生成消费者。书本领域公共效果已存在不等于这些职业分支完成。

**名称：** 新增职业“牧师”、分组“祈祷”及“祝福武器”“驱散敕令”均有上述中文源字符串，本批新增名称没有unresolved。实体书、武器、药水和神器复用已有正式ID/权威中文键；不为未接入的139/334或缺失领域自译新名称。缺失身份是内容范围缺口，不与缺失中文混淆。

后续按[七步计划](priest-class-plan.md)实施，优先补以下有效行为证据：

1. 第二步：24个真实Build新出生与参数引用；1级MP/容量及恶魔主书2级边界、真实升级、负重/手套、两类感知、武器/双持/骑乘、美德和保存。
2. 第三步：善恶合法/非法领域、双方随机学习、96/额外容量、首用/练习、遗忘恢复、副领域5%、改换历史/待确认/伪造存档和相同随机学习续演。
3. 第四步：34/35、41/42级能力资格；单/双刃器与祝福前后、突变/冬贝利优先级、94%普通/天赋伤害；祝福取消/除咒/抵抗/减附魔/知识/折扣/保存，驱散后死亡过滤、恐惧和实际传送、MP不足溢出HP及自然失败费用。
4. 第五/六步：三任务、两寺庙和动态领域服务；新Build五类生成记录及正式入口、源命名、中英文24组合、焦点/忙碌锁/取消、窄屏和缩放。共享学习/物品/近战改动按实际影响回归Mage/Ranger/Paladin/High-Mage等消费者。
5. 第七步：善恶各一条正式出生实战，采用已授权的快速地牢入口；显式标注经验/物资准备和存档续演，同源WebDriver实战与优化EXE原生烟测分别记录。不使用Computer Use。

本步实际验证仅为Git来源读取、288项映射/引用与顺序、320条W/3条S解析、24个合法组合、正式任务/物品/设施交叉核对、文档链接和diff检查。没有执行编译、游戏测试、桌面或Android，也没有导入内容、刷新生成报告或契约fixture。
