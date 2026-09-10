# 心灵术士：来源与运行时缺口

第一步完成，审计日期 2026-09-10；尚未实现或开放职业。对应[职业计划](mindcrafter-class-plan.md)。本记录用于后续实现，不把“已有类型”视为原版规则已完成。

审计主线为 `1eb6abac2`，开始时仅有未跟踪的 `release/`。原版只读取 `D:/codex/Frogcomposband/master` 的 `master` Git 对象，实际提交为 `a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`。下文原版路径、函数与行号均对应此提交；本项目定位对应审计主线。实施时若 ref 或主线改变，只核对相关差异。

## 1. 身份与全部显式职业分支

`git grep -n CLASS_MINDCRAFTER master -- src` 共 19 处；另核对 `mindcrafter.c`、数据表及不使用该宏的消费者。

| 原版位置 | 实际含义与本批处理 |
| --- | --- |
| `classes.c:43,223`；`defines.h:837` | 注册与工厂，原版职业索引 9。本项目拟用 `demo.class.mindcrafter` / `demo.build.mindcrafter`，审计主线未定义；不能将原版数字索引作为新运行时身份。 |
| `py_birth.c:1360,2698` | 创角分类“心智”；推荐属性 `16,11,16,16,14,8` 是分配建议，不能当职业加成。沿用本项目构筑基线。 |
| `combat.c:240` | `max=500, wgt=100, mult=35`，参与力量/敏捷/武器重量攻次公式；不是固定每回合五次。 |
| `devices.c:38` | `class_uses_spell_scrolls()` 返回假，区别书本施法类卷轴；不代表禁止普通卷轴、魔杖或法杖。 |
| `dungeon.c:3918,3984,4031` | 无书浏览、反魔法提示“灵能”、无书施放入口。能力仍受施法状态限制。 |
| `gf.c:2510` | PSI_DRAIN 回蓝文案称“精神力”，资源仍是法力，不增加第二种资源。 |
| `load.c:1639` | 清旧存档的 `add_spells`；本批新开发存档不实现此兼容分支。 |
| `mspells1.c:308` → `monspell.c:3850` | `anti_magic_check()` 的 30 用于智能怪物反魔法法术选择权重，已有反魔法/禁咒状态时权重为 0；不是玩家 30% 反魔法抗性。现有怪物 AI 未见等价职业权重，后续在实际反魔法调用者接入。 |
| `object1.c:340` | 装备 `ART_STONE_OF_MIND=328` 时，心灵术士同时获得 EASY_SPELL 与 DEC_MANA；普通装备旗标仍受 caster options 限制，不能一律生效。 |
| `artifact.c:2282` | 随机神器职业偏好为 `BIAS_PRIESTLY`、`warrior_bias=20`。审计主线未具备此完整生成消费者，列为限制；不把另一工作树的未提交实现当作主线能力。 |
| `races_a.c:1199` | 龙人职业生命乘数 100；本项目龙人默认分支已是 100，不另加一次职业修正。 |
| `virtue.c:272` | 出生美德 Harmony、Enlightenment、Patience，进入现有美德出生合并。 |
| `spoilers.c:1259`；`util.c:3743` | 静态导出分类“心灵”、旧菜单“心灵感应/特殊”；本项目创角采用实际 `_class_groups` 的“心智”，不新建两套分类。 |

额外数据来源：`tables.c:1731,1809` 两领域均 `CH_NONE`；`lib/edit/m_info.txt:2156` 的 `N:9` 为 `NONE/WIS`，`spell_first=99` 不表示无法力；`lib/edit/s_info.txt:2996–3327` 是职业熟练度；`lib/edit/q_old_castle.txt:371–374` 是职业奖励。已有怪物数据中的 Mindcrafter 是独立身份，不改名、不重复导入。

显示名取 `mindcrafter_get_class()` 的“心灵术士”；`clear_mind_spell()` 的“头脑清明”优先于职业说明中的“清晰心智”。神器 328 中文表是“冥想之石”，原计划“心智之石”已纠正。

## 2. 出生、成长与被动

| 项目 | 来源值与落点 |
| --- | --- |
| 属性 | STR/INT/WIS/DEX/CON/CHR：`-1,0,3,-1,-1,2`。复用 Class 修正与现有种族、性格合并。 |
| 八技能 | `dis,dev,sav,stl,srh,fos,thn,thb` 基础 `30,33,38,3,22,16,48,40`；成长 `10,11,10,0,0,0,12,17`，沿现有技能成长公式，不把成长数组直接当每级加值。 |
| 生存与经验 | life 99、base HP 4、exp 125、pets 35。复用职业生命/经验/宠物维持消费者。 |
| 施法属性与负重 | WIS；`max_wgt=400, weapon_pct=50, enc_wgt=800`，保留原版重量单位；caster options 默认 0，未允许普通 DEC_MANA。 |
| 出生物 | 小剑 1、软皮甲 1、速度药水随机 2–5。现有 ID 分别 `demo.item.small-sword`、`demo.item.soft-leather-armour`、`demo.item.swiftstep-tonic`；最后一项是既有改编物品，不新增近似药水身份。 |
| 被动 | 10 恐惧抗性；15 `clear_mind`；20 感知维持；25 `auto_id_sp=12`；30 混乱抗性；40 心灵感应。15 级同时影响休息恢复，25 级须有法力消费者。 |

出生合并差异必须保留记录：原版 `dungeon.c:6433` 先职业出生再种族出生，`py_birth.c:97,126` 出生物完全鉴定并尝试首个空槽；本项目 [initialization.rs](../crates/rfb-core/src/game/initialization.rs) 按种族、职业、性格、构筑合并，已有随机数量上界及装备处理。本项目出生知识当前为库存/装备外观已知，完全鉴定仅装备实例。第二步按既有约定验证合并、重复装备与库存，不为新职业重排所有种族的初始化；若要改变公共出生知识，另明确实际影响并验证。

熟练度使用原版 `s_info` 的 tier `0/1/2/3/4 → 0/4000/6000/7000/8000`。`skills.c::skills_on_birth` 对武器初始 0 执行 `min(2000,max)`，不能照抄成实际初始 0。下表用源 tier，未列例外取该组默认：

| 武器组 / tval−19 | 默认初始/上限 | 例外 sval |
| --- | --- | --- |
| 弓 / 0 | 0/1 | 2、12、13、23、24 上限 2；63 上限 4 |
| 挖掘 / 1 | 0/1 | 无 |
| 钝器 / 2 | 0/2 | 4 上限 1；21 初始 1/上限 4；40 上限 0；63 上限 4 |
| 长柄 / 3 | 0/2 | 无 |
| 剑 / 4 | 0/2 | 8 初始 1/上限 2；32 上限 4 |

`S:0` 徒手 0/4000、`S:1` 双持 0/4000、`S:2` 骑乘 0/2000；武器初始修正不套到这些技能。复用 [weapon_proficiency.rs](../crates/rfb-core/src/game/weapon_proficiency.rs) 的默认值、具体武器覆盖、种族/突变上限与训练增量。[导入审计](../crates/rfb-legacy-import/src/content.rs) 的 `audit_demo_weapon_proficiencies` 已有正确的初始 0 归一化，但职业列表和 `classes_checked` 目前固定六职业，第二步须纳入索引 9。只为实际导入的物品写覆盖，不把未导入武器扩成内容任务。

攻次是真实运行时缺口：[player_stats.rs](../crates/rfb-core/src/game/player_stats.rs) 当前以基础 1、物品 `attacks−1`、装备/状态增量合成，没有 `combat.c::calculate_base_blows` 的通用职业公式。心灵术士需要消费上述参数及原版力量表、敏捷插值（`combat.c:439–530`）；重武器、双手与最低 100/上限 500 按源规则处理。不可用固定 5 掩盖；也不可默默改掉其他六职业已有攻次。实现时明确新公式的适用范围。

物品感知的来源为 `dungeon.c` 的周期感知与 `_adj_pseudo_id`：第一轮慢速弱感知，第二轮中速强感知；混乱时跳过。实际判定为 `randint0(_adj_pseudo_id(N)/((L+10)^2+40)) == 0`，N 分别为 80000、20000；`_adj_pseudo_id` 先应用感知属性、知识美德，每 5 级再减半，35 级返回 0。easy-id 跳过概率并采用强感知，第一轮也在知识美德 ≥100 时变强。第一轮涉及背包、装备、箭袋和袋中物，第二轮背包/装备。现有 [item_knowledge.rs](../crates/rfb-core/src/game/item_knowledge.rs) 的托姆特地面远程强感知不等价，不能直接开成免费自动鉴定。

25 级自动鉴定沿 `autopick.c::autopick_auto_id` 核对：先可用装置，再可阅读的已知鉴定卷轴，再消耗 12 法力；该便捷路径不掷施法失败率、不另收行动能量。现有 [mogaminator.rs](../crates/rfb-core/src/game/mogaminator.rs) 已有装置/卷轴来源排序，缺职业法力来源。只在既有自动处理时机加入来源，检查未知物、法力不足、来源优先级和实际扣费；不改为遍历全地板免费认知。

## 3. 能力执行表

以下以 `src/mindcrafter.c` 各函数的 `SPELL_CAST` 为准。`L` 为等级，`dN` 为 1..N，整数除法向下取整，`B` 为 `to_d_spell`，`SP(x)` 为最终 `spell_power(x)`；先掷骰再 SP。基础费用/失败率在计划中已列，此处补实际动态行为，费用减免最后处理。

| 能力 / 函数 | 执行、范围与等级变化 |
| --- | --- |
| 神经爆破 `_neural_blast_spell` | `SP((3+(L−1)/4)d(3+L/15)+B)`；`d100 < 2L` 时射线，否则半径 0 PSI 球。严格小于不能改成 `2L%`。 |
| 预知 `_precognition_spell` | 初始探测普通怪；5 加陷阱/门；15 加隐形怪；20 测绘；25–39 临时 ESP `L+dL`（此持续时间不经 SP）；30 完整探测；45 全层照明并增加 Knowledge/Enlightenment 各 1。额外费用按最高分支为 `<20:0,20:1,25:3,30:4,45:9`，加基础 1；不是累计相加。40 已永久 ESP，不再附临时 ESP。 |
| 微级位移 `_minor_displacement_spell` | 45 前随机位移距离 10；45 起名称“任意门”，选目的格，范围 `L/2+10`，总基础费用 42。取消走既有目标取消路径。 |
| 宏级位移 `_major_displacement_spell` | 随机传送距离 `5L`，遵守现有反传送等边界。 |
| 支配 `_domination_spell` | 30 前定向 `GF_DOMINATION`、强度 `SP(L)`；30 起 `charm_monsters(SP(3L/2+15))`，视线范围 `GF_CHARM`。两种控制规则不等价。 |
| 念力粉碎 `_pulverise_spell` | `SP((8+(L−5)/4)d8+B)`；`L≤20` 半径 0，否则半径 `SP((L−20)/8+1)`；GF_TELEKINESIS。 |
| 精神测量 `_psychometry_spell` | 20 前 `psychometry()` 弱物品感觉，非 remote，支持原版物品选择范围；已经知道或无感觉也可完成施放并收费。20 起普通鉴定。不能直接复用托姆特 remote 强感知语义。 |
| 躯体装甲 `_character_armor_spell` | 一次 `SP(L+dL)` 同时用于石肤及抗性：15 酸、20 火、25 冷、30 电、35 毒。 |
| 精神波 `_mind_wave_spell` | 25 前自身周围半径 `2+L/10`，伤害 `SP(3L+B)`；25 起视线内 `SP(d[L*((L−5)/10+1)]+B)`。低级 `SPELL_INFO` 写成 1.5L，采用实际执行的 3L。 |
| 肾上腺素引导 `_adrenaline_spell` | 持续 `SP(15+d(3L/2))`；清震慑、英雄、加速。以施放前 `!FAST || !HERO` 决定是否在添加增益后治疗 L；两者原本都有则不治疗。spoiler 随机治疗说明不采用。 |
| 念力 `_telekinesis_spell` | `fetch(dir,15L,FALSE)`；沿现有取物重量、落点和定向规则，不附会为移动怪物。 |
| 精神吸取 `_psychic_drain_spell` | 半径 0 `GF_PSI_DRAIN`，`SP((L/2)d6+B)`；`fire_ball` 返回真才额外消耗 `d150` 能量。返回值由 `project` 的 notice 汇总，**不是实际回蓝成功**，免疫/可见性等必须进入测试。 |
| 精神之矛 `psycho_spear_spell` | `GF_PSY_SPEAR` 射线，`SP(d(3L)+3L+B)`，独立穿透无敌语义。 |
| 精神风暴 `_psycho_storm_spell` | 半径 4 `GF_PSI_STORM`，`SP(5L+10d10+B)`。 |
| 头脑清明 `spells_c.c::clear_mind_spell` | 15 级、WIS、费用 0、基础失败 30；有任何宠物则不能使用，回复 `2+L/30` 法力并封顶。属于 `_get_powers`，不使用 `_on_fail`。`dungeon.c:4993` 的休息直接调用 `cast_clear_mind()`，不掷手动能力失败率、不另加一个行动；现有休息回蓝尚未消费该职业标志。 |

动态名称、费用、失败率、目标及说明必须由同一 Rust 解析结果供投影和执行使用；`ClassAbilityDefinition` 已能按职业/等级推导能力，不增加存档中的“解锁集合”。

## 4. 施法、失败与资源顺序

[characters.rs](../crates/rfb-content/src/definitions/characters.rs) 当前要求 casting profile 有非空领域和正学习上限，Build 还强制第一领域；[player_abilities.rs](../crates/rfb-core/src/game/player_abilities.rs) 的出生法力依赖 casting profile。这是第一处必须改的边界：允许真正的无领域、零学习职业法力配置，保留书本职业约束，不塞一个虚假领域或给出不可用的学习点。

当前 Class 来源的能力投影读静态 activation，`can_cast` 与 [casting.rs](../crates/rfb-core/src/game/abilities/casting.rs) 的部分反魔法/狂暴限制只覆盖 Learned 来源；普通 Class 能力失败会发失败事件返回，没有职业失败回调。新增的 14 项心灵法术必须拥有对应限制和回调；“头脑清明”仍走独立 power 语义，不能把所有职业能力都套成心灵施法。

原版 `spells.c::calculate_cost/calculate_fail_rate` 与 `do_cmd_spell` 的约束：

1. 费用先含等级额外费用，再在适用 DEC_MANA 时取 `max(1,cost*3/4)`；不能把费用 0 的独立 power 强制改为 1。
2. 失败率基础为 `base−3*(L−required)+to_m_chance−3*(stat_adj−1)`，随后 heavy spell +20、easy spell −4、arcane mastery −3、Athena −2；先处理属性/caster 的失败下限及 Athena 下限调整，再加 `50*stun/100`、上限 95，最后 heavy +5/easy −1，钳制 0..100。现有 `class_ability_failure_chance` 与突变修正可复用数值来源，但顺序须与此核对，不能只叠一个最终百分数。
3. 原版先扣法力再掷失败；失败支付全部费用和行动，再调用 `_on_fail`。成功分支的取消返还费用且不行动。本项目先完成目标预检再掷失败，可保留当前交互约定，但必须显式测试取消不改资源、行动或 RNG；不要为模仿源交互重排所有既有施法。
4. `_on_fail` 用最终失败率 F，`d100 < floor(F/2)` 才反噬。这不是 F/2 百分比的近似掷骰。进入反噬后再掷一次 d100，按下表分支；不影响其他职业的 RNG 序列。

| 反噬 d100 | 执行与实际消费者 |
| --- | --- |
| 1–4 | `lose_all_info()`；不是删除背包或清空所有永久知识。 |
| 5–14 | 幻觉增加 `5+d10`。 |
| 15–44 | 混乱增加 `d8`。 |
| 45–89 | 震慑增加 `d8`。 |
| 90–100 | 以失控施法者身份在玩家处投射 GF_MANA，半径 `2+L/10`、伤害 `2L`，影响怪物、地形和物品，且可伤及玩家；随后法力再扣 `L*max(1,L/10)`，最低 0。 |

遗忘来源 `effects.c:6030–6080`：`never_forget` 阻止整次遗忘；否则 Knowledge/Enlightenment 各 −5，完全鉴定物品受保护，清除可遗忘物的感觉/尝试/普通已知等，并有重新感知路径；`auto_id` 可保护物品，但不保护当前楼层地图记忆。现有 [ty_curse.rs](../crates/rfb-core/src/game/item_curses/ty_curse.rs) 的清楼层记忆可复用，直接清 appraised/identified/affix 的部分没有完全鉴定保护，不能整体照搬。须以现有知识状态表达可区分部分；若缺状态，明确扩展保存/验证边界，不用全部清空近似。

失控风暴不复用“只伤敌人”的普通 self-area 攻击：原版 `spells1.c::project_p` 允许失控来源击中自己。必须进入现有伤害、物品/地形作用、死亡及击杀奖励消费者，按投射实际顺序结算；不能只扣玩家 HP 或只播放效果。

## 5. 精神 GF 与现有调用者

原版入口为 `gf.c:2302` PSI_STORM、`:2358` PSI、`:2455` PSI_DRAIN、`:2522` TELEKINESIS、`:2550` DOMINATION、`:3075` CHARM；共享免疫宏 `_BABBLE_HACK`、末尾震慑处理 `:4385`，以及 `xtra2.c::mon_damage_mod` 都属于调用链。以下是必须保留的区别，具体随机次序依此提交的分支实现。

| GF | 不能被普通 damage/control 代替的规则 |
| --- | --- |
| PSI | 怪物须能看见玩家；EMPTY_MIND 免疫；STUPID/WEIRD_MIND/ANIMAL 或等级豁免使伤害为三分之一。强大不死/恶魔在该抵抗分支有二分之一机会反噬，玩家仍可豁免；成功命中和反噬还各有随机状态分支。 |
| PSI_DRAIN | 同类无心智、抵抗与强大不死/恶魔反噬规则，但不能凭“同类”添加 PSI 独有的怪物视线前置。仅完全未抵抗且伤害正时回蓝 `5d(damage)/4`；反噬可使玩家损失 `5d(reduced_damage)/2` 法力并受伤，须遵守法力可吸取及已有防护条件。额外施法能量单独取 notice，不能绑定回蓝量。 |
| PSI_STORM | 无心智免疫；STUPID/WEIRD_MIND 减伤，动物不因此减伤；位移、震慑、混乱、恐惧、睡眠有各自判定，不照搬 PSI 的整段算法。 |
| TELEKINESIS | 四分之一机会传送距离 7，但不传送自己的坐骑；震慑按伤害生成，以唯一怪双倍等级参与豁免。它不是 mental GF，通用 SOUND/FORCE 与 NO_STUN 过滤仍适用；PSI 等 mental 的震慑绕过前两者，仍尊重 NO_STUN。 |
| DOMINATION | 非敌对目标不生效；怪物/强度掷骰豁免、强大不死/恶魔反噬；成功后非唯一/非任务怪且 power>29、`d100<power` 才能直接收宠，否则走随机震慑/混乱/恐惧。 |
| CHARM | 先加魅力及 Harmony/Individualism 修正，唯一/Nazgul/任务怪强度乘 18/25；唯一难度下限 25、守卫/任务怪 50。还检查 RES_ALL、NOPET、竞技场、任务保护、激怒；失败可能记 NOPET。成功对困难目标可能只变友好而非宠物，并更新美德。不能用旧 control 的 `power−10` 等级豁免替换。 |
| PSY_SPEAR | 普通伤害但对怪物走强制穿透无敌参数；对玩家走 `DAMAGE_FORCE`。仍有 RES_ALL 等源规则，不是全伤害免疫一律失效。 |

现状：[player_combat.rs](../crates/rfb-core/src/game/player_combat.rs) 主伤害入口对 Psi 仍走普通抗性/护甲链；念力已有部分位移/震慑 rider，但缺上述完整过滤。既有 [control.rs](../crates/rfb-core/src/game/abilities/control.rs) 是类别、强度与收宠的简化流程，不等于 DOMINATION 或 CHARM。精神反噬不能把这些“类型存在”当作完成证明。

玩家已有夺心魔种族能力、突变心灵爆破使用 Psi；已有念力风伤害调用者也会受共享 rider 改动影响。怪物侧 [monster_combat.rs](../crates/rfb-core/src/game/monster_combat.rs) 区分对玩家/敌对怪物的普通伤害路径，尚无上述完整 GF 目标规则。更具体地，正式 `abilityPrograms/psy-spear-*.json` 目前把精神之矛编码为 `beam-damage + damageType: psi`，已与普通 PSI 混在一起。新增语义必须能区分两者，并同步正式导入生成来源，不能让所有 Psi 都穿透无敌，也不能只修一个测试用怪物。

玩家入伤的 [player_stats.rs](../crates/rfb-core/src/game/player_stats.rs) 目前取状态 `incoming_damage_percent` 最小值，独立穿透语义不能只在玩家出伤处实现。实施须覆盖玩家→怪物、怪物→玩家与怪物→敌对怪物的实际相关调用，同时保留各 GF 的目标专属规则，不能把 `gf_affect_m` 的怪物心智豁免直接套给玩家。

## 6. 跨系统关联及明确限制

| 关联 | 主线现状与后续决定 |
| --- | --- |
| 冥想之石 | 原版 `a_info` 328、`artifact_name_zh.inc` 328，39/23、pval 2，激活恢复法力；本项目正式物品未检出。装备消费者现有，作为职业特例的最小神器接入项：按原版表/中文名/来源记录导入并消费 EASY_SPELL、DEC_MANA，普通装备不越过 caster options。该项尚未完成，不能宣称已支持。 |
| 旧城堡奖励 | [middle-earth.json](../packs/rfb-demo-original/worlds/middle-earth.json) 已有 `classOverrides`，当前仅战士专用池；无匹配职业落到现有默认神器。原版心灵术士池为 `eternity` 1/5、`palantir of westernesse` 4/5（`RANDOM27 % 5`）。原版神器索引 244/15，中文表词段“永恒之”/“西方之地的真知晶球”；正式包未检出对应物品。后续须导入这两项所需完整效果并添加职业 override，复用任务奖励机制；不能把默认奖励当已对齐，也不为本批重做其他职业奖励。 |
| 随机神器偏好 | 记录 priestly/20 参数；生成系统尚未在审计主线完成，暂不实现整套系统。若后续集成提供实际消费者，再做窄范围职业参数接线。 |
| 尚未开放的原版机制 | 源 GF 中的特殊政治家/特殊变身修正、未导入物品及未开放竞技场等条件按实际消费者范围处理；不为本职业预建无入口系统。现有唯一、任务、坐骑、宠物、状态、知识和入伤防护不能以此为由省略。 |
| 前端 | 使用现有“心智 → 心灵术士”叶子和 Class 能力面板，无领域子页；第四步才接入，最终验收前不计开放。 |

旧城堡与神器工作在实际实施前检查主线及其他方向交接状态；只协调确实重叠的文件/效果，不能以“共享文件”为审批点。缺权威中文显示名的任何新增子项记 unresolved，不自行翻译。

## 7. 后续实施切分与证据

第二步先处理无书法力配置及 Class 施法身份，接出生、技能/熟练度、真实攻次、负重、被动、休息和自动鉴定消费者；新增正式引用必须通过内容校验，不能写悬空能力引用。未完成的能力内容也不冒充可施放。第三步按表补动态能力、精神 GF、取消/失败资源时序及跨系统可达关联；第四、五步再完成正常入口和桌面验收。

测试优先复用当前调用者，新增最少的行为证据：

- 正式 Build 新生，种族/性格/职业合并、2–5 药水 RNG、装备知识差异；负重/攻次与熟练度实际变化。无需复制整张静态技能表为另一套测试矩阵。
- 无领域、零学习仍有正确法力；投影/执行的费用、失败率、目标一致；禁咒/反魔法、宠物与休息路径。重点等级 19/20、24/25、29/30、39/40、44/45，及各抗性边界。
- 选定 RNG 验证严格小于的射线概率、最终失败率触发反噬、各反噬状态及失控自伤死亡；取消不改资源/行动/RNG。遗忘分别覆盖完全鉴定、普通已知、地图及防护。
- 精神目标按实际分支选最小 fixture：无心智、奇异/动物、强大不死/恶魔、唯一/任务、自己的坐骑、无敌；精神吸取另验证 notice 与回蓝分离。复用夺心魔、突变、念力、魅惑和怪物精神之矛回归。
- 自动鉴定检查装置→卷轴→法力优先级与 12 法力支付；旧城堡检查职业池权重、强制奖励与已有神器生成状态；冥想之石检查特例与普通装备过滤。
- 一条真实能力/状态/知识或宠物链保存、加载、继续行动。按实际状态、RNG、出生或协议变化做契约验证；纯 contentHash 变化不升级状态哈希或全刷 fixture。

本步只新增审计文档、修订计划并检查差异及链接；没有修改内容包、Schema、协议、Rust 或 UI，没有运行游戏测试或构建。内容版本、lock、生成物和 Tauri standalone 验收随后续实际变更推进，执行[内容开发](content-development.md)与[验证约定](testing.md)。
