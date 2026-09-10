# 狂战士来源与差异审计

第一步已完成，核对日期 2026-09-10。对应[职业计划](berserker-class-plan.md)；本次只做来源和代码审查，没有实现职业、更新内容包或开放入口。下文“待接入”均属于第二、三步实施工作。

## 1. 基线与审计范围

- 本项目基线：`main` 的 `e4f951ab03467fb3140cf5459fd78a609d96efa8`。未跟踪的 `release/` 保留，其他方向工作树不作为已集成能力。
- 原版：`D:/codex/Frogcomposband/master` 的 `master` Git ref，实际提交 `a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`。全部原版读取来自 `git show` / `git grep`，下文源文件行号固定指向该提交。
- 检索 `src` 得到 60 处 `CLASS_BERSERKER`、17 处 `IS_SHERO()`，去重后 73 个命中行；已沿上下文分类。另读职业全文、`s_info`/`m_info` 的 23 号记录、出生/能力/物品消费者、14 个带职业条件的任务脚本与 7 份公会源表。怪物 `P:Copy:*:Berserker` 和同名 Ego 是不同内容身份，不作为玩家职业完成证据。
- 保留现有来源和许可证声明。此文记载规则与对应位置；以后导入新条目时仍须按[内容开发](content-development.md)记录实际来源，不借用历史许可判断替新材料背书。

复查命令：

```powershell
git -C D:/codex/Frogcomposband/master rev-parse master
git -C D:/codex/Frogcomposband/master grep -n -E 'CLASS_BERSERKER|IS_SHERO\(\)' master -- src
git -C D:/codex/Frogcomposband/master grep -n 'Berserker' master -- lib/edit
git -C D:/codex/Frogcomposband/master show master:src/berserker.c
```

### 73 个源命中行的归属

下表覆盖上述两个表达式的全部命中；同一行同时命中只记一次。“不可达”限定于当前正常玩家组合或尚未集成系统，不代表可以忽略现有可达规则。

| 源位置（`src/` 下） | 分类与落点 |
| --- | --- |
| `classes.c:25,172`；`defines.h:851` | 注册英文脚本身份、23 号职业与 getter。新建正式 Class/Build，不复用同名 actor |
| `py_birth.c:1347,2623`；`spoilers.c:1246`；`util.c:3747` | 近战分类、推荐属性分配、旧菜单“蛮力/特殊”。分类与职业能力分组接现有 UI；推荐分配不是职业加成 |
| `combat.c:169`；`xtra1.c:3530,3686,3847,4511,4529,4665,4749,4825` | 攻次、HP 分布、狂暴不加 30 最大 HP、负重上限、狂暴攻防/技能、双持。第二步核心消费者，见第 2–3 节 |
| `xtra1.c:5278` | 武僧空手分支的重甲例外。`equip.c` 不会为普通狂战士建立武僧 `bare_hands`，职业也不设 `monk_lvl`；不是本批新增徒手招式的理由 |
| `xtra1.c:938`；`files.c:1570`；`spells2.c:285` | 狂暴状态栏、恐惧免疫与自我认知显示。Rust 派生并投影；不另存一个永久计时器 |
| `cmd1.c:2757,3058` | 近战随机穿无敌、击杀返还未用行动能量。接真实每击/每轮消费者，见第 3 节 |
| `cmd1.c:3860,3870,4952,4971` | 直接近战的友方确认与移动遇友方攻击；不能继续走普通让位路径 |
| `cmd1.c:4370,4412`；`autopick.c:2678` | 禁止用随身装置自动探陷阱、测绘、鉴定。已有自动鉴定必须拦截；尚无自动探测/测绘消费者的部分不预造系统 |
| `cmd2.c:1907` | 撞门几率中的差值加倍；不是简单把力量或最终 BashPower 加倍 |
| `cmd6.c:877,1134`；`devices.c:47,127,181` | 不读普通卷轴、不能使用装置/装备发动、不使用书本施法卷轴。费用和行动入口见第 5 节 |
| `dungeon.c:3913,3960,3974,4002,4027`；`spells.c:1235` | 无书浏览/施技；跳过指定反魔和狂暴限制，仍受禁咒、混乱等真实条件约束；低等级没有战技的提示 |
| `dungeon.c:6471,6476` | coffee-break 不发归还卷轴；thrall 出生改给药水。当前没有这些开局模式，暂不实现 |
| `effects.c:20,248,2893`；`defines.h:5265` | 自由行动特例、驱散/状态递减后仍狂暴、宏包含职业身份。第二步接免疫与派生状态 |
| `equip.c:1235` | 强行卸下非永久诅咒装备；正常/重诅咒短路 RNG、成功知识更新和失败耗时，见第 5 节 |
| `monster2.c:99` | 看破隐形判定用搜索技能 10，覆盖职业负搜索技能；必须接现有可见性刷新 |
| `mspells1.c:281,391` | `anti_magic_check` 对狂战士返回 0；永久狂暴本身不吸引怪物驱散。需要核对怪物封魔/驱散消费者，不把所有增益都视为永久 |
| `mut.c:180,783` | 随机获得主动突变的权重为 0；半神天赋排除 Astral Guide/Fantastic Frenzy。不是删除已得主动能力，见第 5 节 |
| `races_a.c:1180`；`races_k.c:1191` | 龙人变形天然攻击等级乘 170%；吸血鬼不发黑暗卷轴。两个种族均有当前消费者，必须处理 |
| `beorning.c:520` | 熊形态生命系数改为 `115 + L/3`。当前未接入 Beorning，列为未开放种族限制 |
| `virtue.c:314` | 出生美德为 Valour、Individualism，复用美德初始化 |
| `spells.c:1957` | 摧毁高阶书得经验；现有手动/自动销毁可达，需共用奖励消费者 |
| `spells2.c:4316` | TY_CURSE 麻痹分支的狂战士绝对例外；当前 TY_CURSE 已实现，不能只依赖普通自由行动免疫 |
| `artifact.c:1323,2218,3334` | 随机神器的警告/禁止传送分支、战士偏好、Terror Mask 生成例外；当前完整生成器和该面具未接入，见第 6 节 |
| `corny.c:270,282,377` | 保险仅药水可保毁损、祛魔保险上限额外 +3、禁完整装置恢复。当前无该保险业务，不新增保险系统 |
| `files.c:3024,3039` | 幽灵狂战士评分乘数 −2000；近战挑战模式不加通常的 +500。当前无原版评分/挑战模式，不冒充已实现 |
| `magic_eater.c:447`；`necromancer.c:137`；`r_mummy.c:908` | 其他互斥职业/怪物种族的狂暴消费者，当前普通狂战士不可达；不搬入新职业 |
| `racial.c:30` | `can_do_cmd_cast` 的狂暴阻止；源码调用者仅 `red_mage.c` 的连续书本施法，**不是通用种族 power 禁用入口** |
| `wild_realm.c:49,56` | 随机武器狂暴开关的消息/清状态不得取消职业狂暴；现有 Wild 武器消费者需回归 |

除宏命中外，已检查 `p_ptr->shero` 的直接读取：计时/驱散/清除调用最终经过 `set_shero`；狂怒法师专属分支互斥；`spells1.c` 的临时法术反弹/狂怒护甲在狂暴时有额外效果，若其效果由现有来源获得，须消费同一派生狂暴判定。`save.c/load.c` 是原版计时器格式，本项目不照搬字段。`auto_pseudo_id` 的落地、自动拾取、遗忘恢复另见第 3 节。

## 2. 身份、出生与成长

| 项目 | 锁定的来源值与当前差异 |
| --- | --- |
| 名称/入口 | `berserker_get_class()`：“狂战士”；`_caster_info`：“狂暴之力”；近战分类。拟用 `demo.class.berserker` / `demo.build.berserker`，当前不存在这两个身份 |
| 属性 | STR +8、INT −20、WIS −20、DEX +4、CON +4、CHR +4。源推荐基础分配 `17/8/8/17/15/9` 独立于加成；本项目当前六项基础 13 的出生约定继续适用 |
| 生存/经验 | life 200、base HP 22、exp 160、pets 255；无书、无领域、无学习容量、无 MP 池。`xtra1.c::calc_mana` 遇 `CASTER_USE_HP` 直接排除 MP |
| 基础技能 | dis −100、dev −1000、sav −200、stl −100、srh −100、fos −100、thn 120、thb −2000；每十级仅 thn +50，其他 +0。`skill_tht` 从 thb 初始化，本项目 Throwing 项也应使用 −2000/0 |
| 武器熟练度 | `lib/edit/s_info.txt:N:23`：发射器全部 0/0；其余通常 4000/8000。法师之杖（21/21）与钓竿（21/40）0/0；Grond（21/50）、毒针（23/32）表为 0/8000，但 `skills_on_birth` 将可成长的零初始值提升到 **2000**。仅对正式存在的基底建覆盖，继承现有神器→基底映射，不为缺失基底建占位物品 |
| 其他熟练度 | 武术 4000/8000、双持 4000/8000、骑乘 0/0。武术记录不解锁徒手招式；当前双持有上限字段但出生默认为 0，需接入 4000 的实际初始值 |
| 出生物 | `TV_POLEARM/SV_BROAD_AXE` 一把、`TV_HARD_ARMOR/SV_AUGMENTED_CHAIN_MAIL` 一件、`SV_POTION_HEALING` 一瓶。已存在 `demo.item.broad-axe`、`demo.item.augmented-chain-mail`、`demo.item.healing-potion`；前两项中文表为“阔斧”“强化链甲”（`localization/lib_edit_text_to_translate.tsv` 对应 k_info:721/1951），复用现有显示名 |
| 出生合并 | 原版先熟练度、性格出生、职业出生、种族出生/默认食物与光源；`py_birth_obj` 完整鉴定、优先空槽，防重复装备，Android 不给身体护甲。本项目为食物/光源后 race→class→personality→build，沿用当前初始化顺序与单源 RNG；对实际开放的吸血鬼、装备槽冲突和正式出生知识做测试，不改所有既有种族的出生顺序 |
| 美德 | Valour、Individualism 随职业进入既有去重/补位算法；使用现有 enum 和中文显示名 |

**负技能不能只写进 JSON。** 内容项与 `SkillProgress.base` 支持有符号整数，`SkillProgress::at_level` 的 `current` 却夹在 0..maximum；`player_stats.rs` 虽从原始 base/growth 重新叠加，最终多数技能又用 `NON_NEGATIVE`。因此需要在真正需要负值的技能与消费者保留符号，并让属性详情/投影一致。原版隐匿最终仍限制为非负，反魔/抗魔可抬高豁免下限；不能一刀切移除全部界限，或用把装置技能设很低代替显式禁用。

**公共模型边界：** 当前 `progression.rs::character_base_max_hp_at_level` 使用项目既有 HP 掷骰序列、生命百分比和体质百分比；与原版 `_calc_xtra_hp_aux` 的职业线性分布及 base HP 加算顺序不同。狂战士接入复用此公共成长模型，准确导入上述职业参数，不声称逐级最终 HP 与原版逐点相同，也不在本批另造专用 HP 系统。公共出生物合并、整数再生与装备即鉴定等现有适配边界同样须在交付中保留说明。

## 3. 常驻效果与近战

- 职业自身提供恐惧免疫、STR/DEX/CON 维持、再生 +100、自由行动 +3；35 级免震慑，40 级反射。速度为 `2 + [L≥30] + [L≥40] + [L≥45] + [L≥50]`；护甲 `10+L/2`、挖掘 `100+8L`，均需叠加下面的普通狂暴消费者。
- 狂暴共用规则另加恐惧免疫、武器命中 +12、伤害 `3+L/5`、发射命中 −12、投掷技能 −20、护甲 −10、隐匿 −7、装置 −20、豁免 −30、搜索/感知各 −15、挖掘 +30。狂战士每只持武器的手都获得完整狂暴伤害，不按手数分摊。源 `xtra1.c:4540–4541` 又给前两手实际 `to_h` 各 +12，未同步 `dis_to_h`；这是原版实际命中与显示差异，按执行值接入并让本项目展示与执行一致，不能把额外 +12 直接遗漏。
- 职业武器加成另为命中 `L/5`、伤害 `L/6`；双手持用且 `!omoi` 时再加同值；额外攻次 `4L/100`。普通双持的力量伤害熟练度折减对狂战士跳过，双持重量除数用 24（当前战士 18，其余默认 8）。沿每把武器已有强度/双手持用和增伤路径计算，避免按每手重复加全身加成。
- 基础攻次参数为 `max=600,wgt=70,mult=75`，先按 STR/DEX、实际重量、持重和 `omoi` 得基础攻次，再叠加额外攻次。当前真实 STR/DEX 攻次函数为 `mindcrafter_base_blows`；可提取其现有计算并给已确认职业参数，不另写平行公式。过重、双持、负攻次装备和托姆贝利的 `4L/100` 减攻次必须在最终顺序中验证。
- `weight_limit` 为通常力量负重值的 `3/2`，接 `player_carry_capacity_tenths_pound`。撞门则先算 `bash−doorPower*10`，狂战士将差值乘 2，随后保底 1 再掷百分比；当前 `terrain.rs` 用统一技能判定，必须明确表达此职业差值规则，不能混淆成挖掘/负重加成。
- 每个进入 `mon_damage_mod` 的武器命中，在前面的死亡镰刀条件未短路时掷 `one_in_(2)` 请求穿无敌；未请求穿透时仍有通常的 `1/13` 穿透机会。完整顺序还包括特殊全抗减伤，不等于把所有伤害简单乘二。普通/双持/群攻和共享精神之矛的无敌处理均要回归。
- 近战击杀返还未用能量：`frac = (_allow_crits ? 100 : 120)/max(1,weapon_ct)`，随后 `energy_use = hand*frac + num*frac/num_blow`；`hand` 与 `num` 使用该次执行的手/击次序号。只在原条件 `energy_use != 0` 时结算，保留整数顺序；不能用击杀后固定减半替代。五项战技外层会在执行后设置其 100 能量，不能再在外面重复退一次。
- 非敌对目标的普通移动让位与近战确认需加入狂战士分支：源会攻击，不限于主动选定敌人。仍保留本项目真实的宠物/任务实体完整性约束，确认目标死亡、移位后不复用失效索引。
- 看破隐形已有 `visibility.rs::refresh_invisible_visibility`，狂战士每个 see-invisible 来源使用搜索值 10 对 `randint0(50+monsterLevel/2)` 判定。不能因为职业搜索为负导致永远看不见，也不能直接给永久看破隐形。
- 自动伪鉴定来自 `auto_pseudo_id`：`autopick.c::_sense_object_floor` 对可感觉物品做强感觉；遗忘后按 `effects.c` 重新感觉。它不等于免费完整鉴定，也不依赖心灵术士的定时感知抽样。优先合入 `item_knowledge.rs` 现有托姆特/地面感觉消费者，同时保留更高优先级的完整鉴定。职业本身无 `CLASS_SENSE1/2` 标志，不另编造背包定时感觉表。

**常驻狂暴的表示：** 当前 `player_has_status_kind` 只查计时状态，临时狂暴还携带 +30 最大 HP。新增职业以身份派生狂暴，相关判定与 UI 消费同一来源；不得每回合重新施放狂暴，或保存永不递减的伪计时器。源 `set_shero` 对狂战士强制为 1，驱散、治疗清状态、Wild 武器关状态、遗忘都不能消除此职业事实；怪物驱散 AI 不因它单独选择驱散。

狂暴药水仍真实治疗 30，并按药水路径消费持续时间 RNG；它对狂战士不再增加一层狂暴数值或 +30 最大 HP。不要因“不能重复触发常驻效果”误删药水本身的治疗。35 级免震慑进入同一状态免疫边界；TY_CURSE 必须另接职业绝对免麻痹分支，当前实现会绕过普通状态免疫直接写麻痹。

## 4. 六项能力、费用、失败与取消

中文名取实际 `SPELL_NAME`。`recall_spell` 的“归还术”与职业描述的“召回（Recall）”是源内差异，本项目采用前者。没有需要自行翻译的职业/能力名称。

| 等级 | 能力 / 基础费用 / 基础失败率 | 执行语义 |
| --- | --- | --- |
| 8 | 侦测凶意 / 5 / 40% | `spells_c.c:1021` → `detect_monsters_mind`：范围 30，黑暗地牢范围除以 3，仅排除 EMPTY_MIND。**不要求 SMART，不排除动物、WEIRD_MIND 或友方**；不采用说明里的“有智能”作为另一个过滤条件。无目标也成功付费 |
| 15 | 冲锋 / 20 / 0% | `berserker.c::_charge_spell`：拒绝骑乘、取消与方向 5；相邻无怪时通常取消，但失明时空挥仍成功收费。有怪先普通近战，再检查目标地形可进入且非陷阱、另一侧可进入/非陷阱/无怪，满足则移到另一侧且不拾取。**不要求怪物已死**；另一侧不通仍已完成攻击并收费 |
| 20 | 粉碎陷阱 / 15 / 0% | `_smash_trap_spell` → `move_player(...,break_trap=TRUE)`，陷阱在 `hit_trap` 末尾才被移除；保留其伤害、传送/换层等先前效果。选择方向后即返回成功，不能要求目的格预先有已知陷阱；移动受阻也不自动变成免费取消 |
| 25 | 地震术 / 20 / 60% | `spells_c.c:1393`：以玩家为中心半径 10，复用现有完整地震，不新增独立地形 RNG |
| 30 | 大屠杀 / 80 / 75% | `spells_m.c:212`：依南、北、东、西、东南、西南、东北、西北顺序，对相邻有怪且“可见或该格可投射”的格子调用普通近战；不是八个普通伤害包。空周围仍返回成功。保留每次近战的吸血、光环、自伤、死亡和击杀效果；死亡/离开当前层后必须停止访问旧实体，不让旧索引破坏状态 |
| 10 | 归还术 / 10 / 70% | `spells_m.c:1053` → `word_of_recall(TRUE)`；独立 power。复用本项目目的地选择、延迟、取消、地下城/任务限制；函数返回 false 才取消，不把“取消已启动归还”与“取消界面选择”混为一谈 |

### 费用与实际失败公式

五项战技用力量，归还术用敏捷；**二者都调用 `calculate_fail_rate`，不是两套不同的失败数学公式**。它们的来源身份和费用路径不同，现有心灵术士失败回调不适用于本职业。

`L` 为等级，`lv` 为技能等级，`A` 为有效属性索引；原版通用顺序：等级不足 100%；基础失败 0 则立即为 0；否则从基础失败减 `3*(L−lv)`，加 `to_m_chance`，减 `3*(adj_mag_stat[A]−1)`；heavy_spell +20、easy_spell −4、Arcane Mastery −3、Athena −2（其最小失败率再 −1）；先夹到 `max(adj_mag_fail[A],caster.min_fail)`，再加 `stun/2` 并封顶 95%；最后 heavy_spell +5、easy_spell −1，限制在 0..100。狂战士 caster 的默认最小值为 0。当前 Class 公式的震慑加成仅对心灵术士生效，第二步需让狂战士正确消费，同时保持零基础失败提前返回。

战技先经过 `_add_extra_costs`，但这五项没有额外费用/最小失败回调，能量均为默认 100。`object1.c::_object_gives_esdm` 按 caster 资格过滤普通 EASY_SPELL/DEC_MANA；狂战士不带 `CASTER_ALLOW_DEC_MANA`，不能因为装备有该标志就通用减费/减失败。原版 `ART_NAMAKE_BOW` 的无职业 DEC_MANA 例外尚无当前对应物品，记录为未接入神器限制。独立 power 的 `_add_extra_costs_powers` 明确不使用 DEC_MANA。

| 分支 | 原版顺序 | 本批接法 |
| --- | --- | --- |
| 五项 HP 战技 | 预算为当前 HP；费用大于 HP 时拒绝；掷失败→执行/普通失败→设置能量→`take_hit(DAMAGE_USELIFE,cost)` | 复用 Class HP 字段与执行器，补效果后结算；不能把 HP 放入第二个资源池，也不能先扣再退来近似战斗中回血 |
| 归还术 / 通用 power | 预算 HP+SP；掷失败→执行/失败→支付能量→先耗剩余 SP，不足部分扣 HP | 狂战士本身无 MP，通常全为 HP；仍区分 power 的属性/费用来源。复用既有种族 power 思路，不误套心灵术士反噬 |
| HP 恰好等于费用 | 可以尝试；效果后扣到 0 不死亡，`chp<0` 才死亡。原版扣到 0 还加 Sacrifice +1、Chance +2 | 当前 `player_is_dead` 同为 `<0`，保留边界并检查真实状态/美德；效果中死亡时源 `take_hit` 不再扣第二次 |
| 普通失败 | 默认 `SPELL_FAIL` 返回 true，无效果，支付完整费用和 100 能量；无职业失败回调 | 断言 HP/行动/事件，不能把失败当免费，也不能引入反噬 |
| 取消 | 原版先掷失败，成功分支才让效果询问目标；效果 false 则不付 HP/行动，但 RNG 已前进 | 延续本项目既有“目标预检/取消不改 RNG”交互契约，明确这是适配差异。预检不能误拒绝盲目空挥、无陷阱移动等源允许的收费尝试 |

生命费用绕过通常无敌与灵体减伤，不接普通可减免攻击伤害；不能因此推断它跳过 `take_hit` 中的所有前置修正。源 `DAMAGE_USELIFE` 仍有既有美德、死亡及少数其他职业防御消费者。狂战士正常可达的支付边界随本批接入，其他互斥职业的特殊防御不新增。

**状态限制：** 无书、不要求视线内有书或学习点。`dungeon.c` 对五项战技豁免 NO_MAGIC 地牢、装备反魔和狂暴本身，但 `tim_no_spells` 仍阻止；混乱/恐惧由能力标志过滤，这六项没有额外可用标志，失明并不通用禁用它们。种族和已存在的主动突变使用独立 power 入口，不因常驻狂暴被统一封死。

## 5. 物品、突变和种族的实际边界

| 项目 | 源规则与实施要求 |
| --- | --- |
| 普通卷轴 | `do_cmd_read_scroll_aux` 在“不识字”检查前已设行动 100（Speed Reader 为 50），不消耗卷轴。前端可显示不可用；实际提交时按选定适配规则处理，不能声称原版拒绝一律零行动 |
| 魔杖/法杖/魔棒 | `device_calc_fail_rate` 返回 1000/1000，`device_try` 后职业失败分支把行动归零，不消耗充能。不能依赖低技能仍有保底成功率的通用 `resolve_check` |
| 装备发动/捕获球 | `effect_calc_fail_rate_aux` 同为 1000；`do_cmd_activate_aux` 失败保留 100 行动，不发动/不进入冷却。捕获球在该判定**之后**，其独立捕获/释放入口也不能绕过职业限制 |
| 自动鉴定 | 原版 `autopick_auto_id` 直接排除狂战士；当前 `mogaminator_auto_identify` 可直接找卷轴/装置付款鉴定，必须在共用资格判定中拦截，不能只禁手动 UseItem |
| 允许的物品行为 | 药水、食物、通常装备/卸装与灯具补充等按自身规则继续可用；读不懂/无法发动不等于禁止持有、出售、投掷或销毁该物品 |
| 强拆正常诅咒 | `equip.c` 用 `one_in_(4)`；成功清除 curse_flags/known_curse_flags、标记感觉并清旧 feeling，再继续卸装；失败仍花行动，物品仍穿着 |
| 强拆重诅咒 | `one_in_(7) || one_in_(4)`，第二次只在第一次失败时掷；概率为 **5/14**，不是 1/7，也不是两次必掷。永久诅咒直接拒绝、零行动、不掷这两次 RNG |
| 替换/丢弃已装备物 | 源通过同一卸装校验；当前 `mod.rs` 预判与 `inventory.rs` 卸装/替换都直接挡诅咒。必须收口到同一强拆判定，验证背包容量/槽位等失败不会清诅咒后丢物。只改 `Unequip` 会留绕过或不一致 |
| 突变 | `_mut_prob_gain` 排除随机获得的 `MUT_TYPE_ACTIVATION`；不改变随机失去，不清除已经存在或其他合法来源给予的能力。`mut_demigod_pred` 排除 Astral Guide、Fantastic Frenzy；现有 `mutation_choice_exclusions_by_class` 足以表达，连同使用该选择池的人类路径处理 |
| 龙人 | `races_a.c::_draconian_attack_level` 先 `L*2`、亚种倍率，再职业 170%。落在 `player_stats.rs::draconian_metamorphosis_attack_level`，**不是吐息伤害或种族 lifePercent**；龙人变形仍可能合法获得，不因它是主动 power 而封禁 |
| 吸血鬼 | `_vampire_birth` 对狂战士跳过黑暗卷轴，连 2–5 数量 RNG 也不抽。当前 race startingItems 无职业过滤，需在现有出生消费者处理，不能临时改全局种族定义 |
| 托姆特/托姆贝利/水晶龙人 | 感觉优先级、减攻次与免疫/反射叠加均复用既有消费者，验证无重复加成；不因为两个来源同时具有能力就重复触发 |
| 无生命/特殊恢复种族 | HP 战技不能套用血骑士的非生命禁用条件；当前合法种族照常可选，按其实际恢复/进食路径验证费用后继续游戏 |

对卷轴/发动等被拒绝的手动尝试，第二步采用源上表的行动差异；UI 只说明结果。现有取消不消费 RNG 的约定继续保留，不为模拟原版必败装置尝试而在不可用投影中掷骰。该交互适配必须在测试和最终交付中明确，不能悄悄改变其他职业的拒绝策略。

## 6. 正式可达关联与明确的系统限制

### 必须随本批接入

当前世界已支持 `reward.classOverrides`，下列八项任务对新职业仍会落到普通奖励，不能开放后再补。优先复用正式物品/Ego 和已有加权奖励；没有的必要条目随第三步合法导入。表中保留源 token，不自行翻译新物品名。

| 源任务 | 当前任务 ID | 狂战士奖励 |
| --- | --- | --- |
| `q_orcs.txt` | `demo.task.anambar-orc-camp` | `OBJ(HAFTED):EGO(slaying)`；复用已有钝器基底和适用 slaying Ego，不给普通魔杖 |
| `q_apina.txt` | `demo.task.anambar-apina-island` | `SV_POTION_LIFE` ×1 |
| `q_htower.txt` | `demo.task.anambar-lord-bovin-treachery` | `SV_POTION_LIFE` ×1 |
| `q_cellar.txt` | `demo.task.anambar-cellar-killer` | `SV_POTION_LIFE` ×1 |
| `q_birds.txt`（`q_info:62`） | `demo.task.crows-nest` | `demo.item.enlightenment-potion` ×1 |
| `q_vapor2.txt`（`q_info:20`） | `demo.task.vapor-quest` | `OBJ(sling,DEPTH+15):EGO(hunter)`；保留深度与 Ego 生成语义，不把普通弹弓数量当完成。旧 `q_vapor.txt` 无此职业覆盖，当前 master 的任务索引使用 v2 |
| `q_old_castle.txt` | `demo.task.old-castle` | `demo.item.slayer` : `demo.item.pain` = 1:4；已有战士奖励池可直接复用 |
| `q_wtower.txt` | `demo.task.thalos-old-watchtower` | `OBJ(long sword):EGO(death)`；复用现有基底与该 Ego |

`SV_POTION_LIFE` 尚无当前正式对应物品，不能用 `new-life-potion` 或 `healing-potion` 替代。任务的通用 HAFTED/深度修正与现有奖励类型如有表达缺口，只补这两个真实消费者需要的字段/选择，沿现有物品生成路径执行，不新建任务奖励生成器。

五座现有战士公会（Morivant、Angwil、Telmora、Anambar、Thalos）源 `B:7:C:Berserker:Owner` 必须加入正式 `ownerClassIds`。原始 membership 记录已保留，但 `town_facility_membership` 只消费正式 ID；不能把来源表有字样当作会员规则已接通。现有强化价格/上限复用原消费者；源 `t_lite/t_ulite` 两种旧镇表当前没有对应开放地点。

摧毁高阶法术书：源 `cmd3.c::high_level_book` 要求特定魔法书 tval 且 sval>1（不包含 Arcane）；战士/狂战士符合经验奖励，Android 排除。每本经验先取 `min(max_exp/20,10000)`，第三本（sval=2）再除以 4，随后至少 1，最后乘销毁数量；生命书增加 Unlife、减少 Vitality，死亡/死灵高阶书反向。当前 `inventory.rs::destroy_item` 只删物品，手动与 Mogaminator 已共用它：接一个真实奖励消费者，并回归已受影响战士，不在两个入口各写一次。

### 不作为当前职业开放前的虚构依赖

- `q_bears`、`q_doom1`、`q_doom2`、`q_eric`、`q_stable`、`q_wargs` 对应任务/模式尚未开放：分别为启蒙药水×1、元素戒指、治疗药水 `5+1d4`、启蒙药水 `5+1d4`、酒×10，以及 SPEED=2 模式不额外给化石为泥魔杖。记录规则，待对应任务进入正式世界时接入，不本批造整套地点。
- `artifact.c`：随机神器 case31 对狂战士 10% 给 WARNING、90% 给 NO_TELE；卷轴造神器的职业偏好为 WARRIOR；Terror Mask 对狂战士给额外 power/resistance 而不走通常的 aggravate/TY/heavy curse 分支。当前 main 只有部分神器实例/偏好基础，没有完整随机神器生成器，且该面具不在正式包；不把其他工作树的开发视为主线实现。已有可达物品/奖励仍按上表处理。
- Beorning 熊形态、monster-class 武术/附身、其他互斥职业机制、Cornucopia 保险、原版最终评分、coffee-break/thrall/挑战开局、特殊时间停止/疲劳系统不在当前开放范围。本批不为它们增加持久状态；以后接入时按本表核对职业交叉。
- 当前没有通过随身装置自动探陷阱/测绘的完整路径；资格规则应易于从同一物品限制读取，但不为未来增加自动化管理器。已有 Mogaminator 自动鉴定属于本批必做。

## 7. 修改落点、验证与第一步结论

| 实施批次 | 现有落点 | 必须证明的行为 |
| --- | --- | --- |
| 第二步：定义/出生 | 正式 Class/Build/技能集、`initialization.rs`、`progression.rs`、`weapon_proficiency.rs`、`virtues.rs` | 无 MP/领域；三项出生物和种族合并；负技能显示与实际判定；熟练度 4000/8000 与 0/0；五公会会员。不同于原版的公共 HP/出生约定明确记录 |
| 第二步：被动/战斗 | `player_stats.rs`、`player_combat.rs`、`terrain.rs`、`visibility.rs`、`turn.rs`、`item_knowledge.rs`、`item_curses/ty_curse.rs` | 狂暴派生/不叠加、无 +30 最大 HP、药水仍治疗；速度/免震慑/反射边界；双持/负重/攻次；穿无敌、击杀耗时、友方攻击；看破隐形与 TY_CURSE 特例 |
| 第二步：使用/费用底座 | `abilities/casting.rs`、`player_abilities.rs`、`snapshot.rs`、`item_use.rs`、`inventory.rs`、`capture_ball.rs`、`mogaminator.rs`、`mutations.rs` | Class HP 已有字段/投影，补效果后支付、震慑和独立 power 费用；禁止真实物品使用及自动鉴定绕过；强拆成功/失败、替换与永久诅咒；随机/天赋突变过滤 |
| 第三步：效果和奖励 | `abilities/`、既有移动/近战/陷阱/地震/归还、正式世界奖励和物品生成 | 六项真实执行、无目标收费边界、目标验证/取消、原序 RNG、效果中死亡/吸血；八任务覆盖及高阶书销毁奖励；保存恢复后确定性继续 |
| 第四/五步：入口与验收 | 现有创角、能力和物品面板、`trait_details.rs`、协议消费者与 Tauri standalone | 1 级空技能组、等级解锁、HP 费用/失败/原因一致，中英文、键盘取消、窄屏和缩放；普通新开局与有明确前提的高等级操作证据 |

优先复用 `tests/abilities/casting.rs`、`tests/mindcrafter/`、`tests/inventory.rs`、`tests/items.rs`、`tests/mutations.rs`、`tests/progression.rs`、`tests/weapon_traits.rs`、现有能量/近战、地震/归还、任务奖励与公会检查。补测试按真实缺口，不复制整套静态技能表或全种族矩阵。强拆重诅咒要测短路是否消费第二次 RNG；费用要测不足/恰好相等/失败/吸血后支付；侦测要测 EMPTY_MIND 与普通动物的差别；粉碎陷阱要测先触发；狂暴要测治疗清除、驱散和 Wild 武器切换。

当前规则无需新增“已解锁能力”集合、第二 HP 池或狂战士管理器。可派生值不持久化；确需改变权威状态/共享投影/公共初始化时按[验证与契约](testing.md)覆盖保存、哈希、回放，类型变化通过生成器生成。纯内容 hash 不推动 State Hash Schema 变化。共享文件只在真实并发或交叉依赖时按[并行协作](parallel-development.md)协调。

**第一步结论：** 来源、中文名、出生/技能/熟练度、常驻与公共战斗消费者、六项能力的数学与时序、物品/突变/种族、公会/任务/销毁关联均已分类并落到实施批次；未实现系统限制已单列。无需追加新的先行系统才能开始第二步。此结论是审计完成，不能解读为上述规则已经实现或测试通过。本次仅检查文档链接、来源引用与 diff，不编译或运行游戏测试。
