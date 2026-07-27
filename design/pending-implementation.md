# 待实现内容清单

状态：基于 contract-v1–v99、前端目标模式和系统路线书审计；每完成一个纵切后同步更新

本文件只记录已经在现有设计或原版对比中明确出现、但尚未实现的内容。长期设想仍保留在 [RFB 全系统梳理与重构实现路线](rfb-system-implementation-roadmap.md)，这里用于跟踪可以实际排入后续 contract 的缺口。

## 当前推进顺序

| 优先级 | 候选纵切 | 状态 | 边界 |
| --- | --- | --- | --- |
| P0 | 暂停任务管理 | 已由 contract-v61 完成 | 地表直接放弃、重接次数限制和保留进度的确定性重建 |
| P1 | 区域组合扩展 | 已由 contract-v62 完成 | 区域与 Vault、pit、动态群体、feature、分阶段地貌和多连接组合 |
| P2 | 树状地牢与共享守护者镜像 | 已由 contract-v63 完成 | 不同楼梯进入不同子层、多个程序化最终叶层和一次性共享征服 |
| P3 | Vault 多入口与连通拼接 | 已由 contract-v64 完成 | 1–8 个边界入口、模板/整层连通证明和确定性 BFS connector |
| P4 | 地牢实例身份与生命周期 | 已由 contract-v65 完成 | 实例序号、实例+floor 仓库键、实例级清理和 v64 存档迁移 |
| P5 | 动态探索树连接解析 | 已由 contract-v66 完成 | 加权楼梯候选、同层目标去重、解析目标持久化与旧存档固定目标回退 |
| P6 | 地牢入口守卫与可选进入条件 | 已由 contract-v67 完成 | 原版式可绕过软门槛、原创内容硬条件、原子拒绝和旧存档抑制迁移 |
| P7 | 胜利、退休与角色评分 | 已由 contract-v68 完成 | campaign victory dungeon、胜利/退休状态、确定性评分、存档迁移和 UI 结算 |
| P8 | 可配置实例生命周期 | 已由 contract-v69 完成 | `reset-on-surface`、`persistent`、`turn-ttl`、retained 存档字段、惰性 TTL 淘汰和实例级物品属性知识清理；普通地牢继续回地表即清空 |
| P9 | 运行时连通修复 | 明确不实现 | 地形破坏直接成为权威状态，不做自动重连、楼梯迁移或整层修复；玩家可使用挖掘能力自行恢复通路 |
| P10 | 角色成长基础 | 已由 contract-v70 完成 | 击杀经验、RFB 1–50 阈值、未胜利 50 级封顶、胜利后 100 级与 `18/820` 解锁、六维自然/有效属性、HP 序列、装备 modifier、属性点命令和存档迁移 |
| P11 | 角色创建与构筑基础 | 已由 contract-v71 完成 | Race/Class/Personality、技能集合、五个代表性初始构筑、出生装备、来源可解释的派生属性、技能成长和 v70 存档迁移 |
| P12 | 可观察技能检定 | 已由 contract-v72 完成 | device、saving-throw、stealth、perception 的权威消费、结构化事件、警戒存档和相同 seed 构筑对照 |
| P13 | 法术与能力书基础 | 已由 contract-v73 完成 | resource/ability/ability-book 内容根、Class casting profile、可保存 Mana/已学能力、学习/失败率/目标施法、Web 面板和旧存档迁移 |
| P14 | 法术恢复与多效果能力 | 已由 contract-v74 完成 | Mana 等待/休息恢复、真实调度与危险中断、稳定自身目标、Stillwater Notes 和固定治疗 |
| P15 | 能力熟练度与冷却 | 已由 contract-v75 完成 | RFB 五档熟练度、Mana 成本/失败率修正、成功/失败统计、独立/共享冷却、零 RNG 拒绝、存档迁移和 Web 展示 |
| P16 | 学习容量与遗忘 | 已由 contract-v76 完成 | 独立学习容量、等级/属性修正、容量投影、主动遗忘、零 RNG 拒绝、进度保留、重新学习与旧存档兼容 |
| P17 | RFB 式范围爆发伤害 | 已由 contract-v77 完成 | 定点穿透/方向停止、墙体遮挡、整数距离衰减、稳定由内向外顺序、逐 actor 抗性/击杀管线、零目标与无效目标 RNG 边界、Echo Burst 与 replay/save 基准 |
| P18 | 方向射线能力效果 | 已由 contract-v78 完成 | RFB `fire_beam()` 式方向射线，穿透 actor、墙体/边界截断、近到远顺序、共享一次基础伤害骰、空射/无效模式 RNG 边界、Echo Lance 与 replay/save 基准 |
| P19 | 锥形能力效果 | 已由 contract-v79 完成 | 复用目标验证与伤害管线，固定八向锥形 footprint、逐层展开、墙体遮挡、横向整数衰减、目标顺序和事件/RNG 语义；Echo Fan、replay/save 与八向几何基准已建立 |
| P20 | 定点延长射线 | 已由 contract-v80 完成 | RFB `project_hook()`/`PROJECT_THRU` 语义，支持 direction/position/entity，定点或实体目标后沿稳定整数斜率延长到最大射程；actor 穿透、墙体截断、共享伤害骰、无效目标零资源零 RNG；Echo Lance、replay/save 与 202 个 exact fixtures 已建立 |
| P21 | 首个位移能力 | 已由 contract-v81 完成 | Echo Step 内容驱动 teleport；仅 position 目标，落点需非当前格、在图内、可见、满足 line of effect、可行走且无存活 actor；无效落点零资源/零 RNG，成功精确移动并复用普通移动到达管线；协议 1.81、内容包 1.73.0、209 个 exact fixtures |
| P22 | 首个召唤能力 | 已由 contract-v82 完成 | Echo Companion 内容驱动 summon；稳定 actor ID、玩家阵营/所有者、数量/半径、确定性落位、玩家回合生命周期、空间不足原子回退、失败率资源语义、save/replay 与 Schema v35 |
| P23 | 首个侦测能力 | 已由 contract-v83 完成 | Echo Pulse/Echo Sight 内容驱动 detect；category/radius、FOV 与隐藏投影过滤、稳定顺序、持久/瞬时知识、空结果、非法目标/资源不足 RNG 边界、save/replay 与 Schema v36 |
| P24 | 首个地形改变能力 | 已由 contract-v84 完成 | Echo Delving/Echo Rampart 内容驱动 transform-terrain；来源/目标 terrain 集、position/FOV/line of effect、稳定原子提交、占用格与连接/边界保护、空结果、changed cells、save/replay 与 Schema v36 |
| P25 | 状态能力与多 effect 组合 | 已由 contract-v85 完成 | Echo Quickening/Echo Binding；状态添加/移除、2–8 个有序同目标 actor 效果、堆叠、抗性缩时、免疫、部分无效、目标死亡/无目标跳过、save/replay 与结构化 Web outcome |
| P26 | 首个怪物施法与能力选择 AI | 已由 contract-v86 完成 | Monster 百分比频率与加权能力集合、射程/墙体/友军 clean-shot、频率失败普通行动回退、伤害/状态/有序效果复用、逆频率自身行动冷却、save/replay 与 Schema v37 |
| P27 | 怪物施法效用与目标扩展 | 已由 contract-v87 完成 | HP/状态/距离有效权重、自身治疗/增益、范围/射线/锥形、保守 footprint 风险、敌对召唤、逐候选协议观察与 257 个 exact fixtures |
| P28 | 怪物目标选择与施法记忆 | 已由 contract-v88 完成 | 玩家阵营召唤物目标、敌我多目标评分与实际结算、保持距离/25% HP 撤退、smart caster 已观察抗性记忆、save/replay、Schema v38 与 265 个 exact fixtures |
| P29 | 友方召唤物行动与首版命令 | 已由 contract-v89 完成 | Follow/Attack/Keep Distance/Guard、零时间全局命令、能量调度、近战归属、2 格跨层跟随、save/replay 与 Schema v39 |
| P30 | 首个非 Mana 职业资源 | 已由 contract-v90 完成 | 多资源底子（ResourceDefinition 行为字段 + techniqueProfiles）、节奏/决斗家纵切、命中/击杀获得、闲置衰减、先天技法、save/replay、旧存档子集迁移、Schema v40 与 282 个 exact fixtures |
| P31 | 旧版内容导入管线 v1 | 已完成（纯工具，不动契约基线） | f_info/r_info 只读导入 .local 本地包（地形 180/188、怪物 1332/1396），缺口报告按缺失法术/效果/flag 计数，后续规则族按报告排期 |
| P32 | 多 blow → meleeRoutine 映射 | 已完成（纯工具） | 1124/1332 导入怪物获得完整多段近战 routine，逐 blow 伤害类型，107 条无骰副攻计入缺口 |
| P33 | 导入器法术映射 v1 | 已完成（纯工具） | SCARE/SLOW/HASTE/HEAL + 1_IN_N 频率 → monsterCasting，454 只导入怪物成为施法者，78 个共享生成能力 |
| P34 | 怪物位移法术族 | 已由 contract-v91 完成 | blink-self/teleport-self/teleport-target 三效果、rift-stalker 纵切、导入映射 455 实例（casting 怪物 553）、288 个 exact fixtures |
| P35 | 新状态族（混乱/致盲/麻痹） | 已由 contract-v92 完成 | 三个新状态种类 + 玩家侧效果（移动重定向/禁施法、FOV 压制、行动浪费）、gloom-weaver 纵切、导入映射 548 实例（casting 怪物 586）、299 个 exact fixtures |
| P36 | 怪物直伤弹族（bolt/ball）与伤害平坦加值 | 已由 contract-v93 完成 | 四伤害效果加 damageBonus、1d1+(F-1) 平坦恒等式、cinder-adept 纵切、导入映射 BO_/BA_ 622 实例（casting 怪物 696、共享能力 502）、303 个 exact fixtures。原候选 DETECT 族经源码核实为 `MST_POSSESSOR` 附身专用组（怪物不施放，共 522 个 token 实例含 MAPPING/BERSERK/BLESS 等），按不适用归档 |
| P37 | BR_ 吐息族 | 已由 contract-v94 完成 | breath-damage 效果（当前 HP×pct% 封顶 max、零伤害骰、锥形复用 v79）、ash-drake 纵切、fixtures 304-306（封顶/残血衰减/致死闭环）、306 个 exact fixtures。导入映射吐息 337 实例（casting 怪 696→765）+ FREQ_N 频率修复（297 实例消化）+ 附身组 522 实例重分类 notApplicable；施法表上限 32→64（旧版最大杂烩 34 技能悉数保留，导入器仍留 64 截断守卫） |
| P38 | S_ 召唤族 | 已由 contract-v95 完成 | summon-category 效果（标签+等级上限候选、数量骰、逐只有界抽 kind、落位/生命周期复用既有召唤）、mote-binder 纵切、fixtures 307-308、308 个 exact fixtures。导入器类型旗标→标签（undead/demon/dragon/animal）+ S_ 映射 493 实例（casting 怪 765→783、S_KIN 76 用固定召唤映射召唤同类）；源包文件预算 2048→4096 |
| P39 | 伤害类型扩展（RFB 原版元素表） | 已完成（纯枚举+导入器，无契约迁移） | 按 gf.h 原序新增 22 类（light/dark/confusion/nether/nexus/sound/shards/chaos/disenchant/time/mana/gravity/inertia/plasma/force/nuke/disintegrate/storm/holy-fire/hell-fire/ice/water），协议 1.96；导入器近似转正 + 异种元素/吐息全解锁 + blow 元素名直映——法术映射 3849（+778）、casting 怪 829、未映射 1379。见 [damage-type-roster-v1](damage-type-roster-v1.md) |
| P40 | 抗性档导入 | 已由 contract-v96 完成 | actor 内容层 resistances 字段（类型→档位，normal 缺省）、11 处生成路径盖章（存档保持权威）、slag-crawler kin 召唤纵切、fixtures 309-310（盖章召唤体电弹减半 / 显式免疫覆盖）、310 个 exact fixtures。导入器 RES_/IM_/HURT_ → resistant/immune/vulnerable：1023 只怪 3842 条条目 |
| P41 | 心灵族 | 已由 contract-v97 完成 | psi 伤害类型（协议 1.97）、MIND_BLAST/BRAIN_SMASH→Sequence[psi 伤害+骑手]（既有白名单零新机制）、PSY_SPEAR→首个导入 beam、mind-lasher 纵切、fixtures 311-312（同种子孪生：Normal 全额 vs psi 抗性减伤+缩时）、312 个 exact fixtures。导入 248 实例全数（casting 怪 831、映射累计 4097、未映射 1131） |
| P42 | 诅咒族与豁免门 | 已由 contract-v98 完成 | curse-damage 效果（豁免成功全免零后续 RNG、失败全额无护甲无抗性；难度=施法者等级；复用 v72 saving-throw 检定与 SavingThrowChecked 事件）+ curse 伤害类型（协议 1.98）、hex-chanter 纵切、fixtures 313-314（同设置双种子：成功 skip saved / 对抗失败 raw=final 全额）、314 个 exact fixtures。导入 CAUSE_1-4 240 实例全数（casting 怪 832、映射累计 4337、未映射 891） |
| P43 | 小型效果杂项包 | 已由 contract-v99 完成 | teleport-away（BanishTarget 专用计划、候选规划期收集+减半回退、复用 relocate 管线与 v91 位移事件族）、drain-resource（吸施法档资源池+施法者回血封顶）、amnesia（豁免门+清当前层 explored/revealedTerrain）、DISPEL→既有 remove-status haste；veil-warden 四式纵切、fixtures 315-318、318 个 exact fixtures。导入 264 实例全数（casting 怪 844、映射累计 4601、未映射 627） |
| P44 | k_info 物品导入 | 已完成（纯工具，见 [legacy-item-import-v2](legacy-item-import-v2.md)） | 545 条中 544 条落地：武器 meleeProfile、发射器按 sval 配对典型弹药（乐器类 5 件降级为壳）、护甲/首饰槽位+防御/六维修正、光源标签、消耗品/设备/魔典壳+行为缺口按类计数（consumable-effect 95/book 72/device 64 等）；产物过全量校验 |
| P45 | ego 词条 + 固定神器导入 | 已完成（纯工具，见 [legacy-item-import-v2](legacy-item-import-v2.md)） | e_info 160 条中 88 条成 affix（C: 上限取确定性顶格、六维含 DEC_ 负向按 pval 折算；72 条力量全在不可表达旗标按 ego-inexpressible 跳过入报告）；a_info 392 条神器全数落地（固定 pval/骰/修正，哨兵 pval 钳制 ±100，INSTA_ART 不计缺口，激活计 artifact-activation 180）；**同轮修正：普通戒指/护符为无属性通用壳**（属性只来自词条/神器，与原版生成模型一致）；产物 936 items + 88 affixes 过全量校验 |
| P46 | fake bow 修正（未配对发射器保槽） | 已完成（纯工具，见 [legacy-item-import-v2](legacy-item-import-v2.md)） | 未配对发射器（基础 5 + 神器 7）从"去槽壳"改为原版 `obj_is_fake_bow` 语义：保 launcher 槽、不带射击档、神器固定修正全保留（阿波罗竖琴取回六维 +5×3）；P: 射击加成随射击档舍弃；依据：契约物品规则均为单向、launcher 槽无射击档合法，运行时查无射击档仅拒绝开火 |
| P47 | 身体/槽位模板 | 已由 contract-v100 完成 | RaceDefinition.bodySlots 槽实例表（空=标准身体）+ 核心标准身体 13 槽（**ring-1/ring-2 双戒指、light 光源槽**，单实例 id=类型名故旧档零迁移）+ 装备按类型找空实例/满则顶替首实例（item.equip.swap）+ 存档权威（Schema v41）/旧档零 RNG 派生 + 快照 bodySlots（协议 1.100）+ 前端全槽位面板（空槽"空缺"、同类型序号）+ 导入器光源接 light 槽（帕蓝提尔等 8 件神器六维回收）。demo 新增共鸣指环（包 1.91.0），fixtures 319-320（双戒指/顶替），320 个 exact。记录差异：双持手与箭袋未纳入（待各自系统） |
| P48 | b_info+种族+性格导入（T1） | 已完成（纯工具，见 [legacy-character-import-v1](legacy-character-import-v1.md)） | 代码侧结构化提取（函数头识别+花括号配平+赋值行解析，rank 动态右值标记跳过）：**67/88 种族**（21 怪物种族 race-code-dynamic）、**20/21 性格**、113 身体模板全解析（缺口普查 any 76/quiver 13/capture-ball 4）；玩家种族绑 Standard 12 槽（刻意无 charm）；八技能花名册 1:1 映射+87 skillSets；钩子缺口 calc_bonuses 76/birth 27 为 T2/种族能力线提供覆盖数；霍比特抽查逐项一致 |
| P49 | 装备/内在旗标系统·防御面（T2 前半） | 已由 contract-v101 完成 | 装备/词条/种族三处统一 resistances/statusImmunities/modifiers.speed 声明表面；有效抗性确定性合并（immune 胜、正档遇 vulnerable 降档）、免疫查表跳过、装备速度进派生管线；协议 1.101（物品 DTO 知识门控暴露）、包 1.92.0（御火指环/疾行靴/镇静吊坠）、fixtures 321-323 共 323 exact、Schema 保持 v41。回灌收割：ego 105/160（+17）、神器 392/392、35 词条/33 种族/321 物品带防御表面，RES_*/IM_*/SPEED/FREE_ACT 清零。进攻面（SLAY_/BRAND_）留 T2 后半 |
| P50 | 进攻面旗标（T2 后半） | 已由 contract-v102 完成 | Item/Affix 统一 slays（11 类目标，slay/kill 两档）+ brands（酸电火冷毒）；玩家持武器近战按原版 tier 只放大武器骰、多项取最高、元素 immune 压制对应 brand，零额外 RNG；协议 1.102、包 1.93.0、Schema 保持 v41、fixtures 324-326 共 326 exact。回灌：ego 107/160、神器 392/392，12 词条/130 物品带 slay，5 词条/90 物品带基础 brand |
| P51 | 动态 affix 实例 + 装备被动属性 | 已由 contract-v103 完成 | Item/Affix 统一 equipmentBonuses/passives，Affix rollGroups 按深度过滤并加权抽取，物品实例保存 materialized rolledAffixes；旧档缺字段保持空且零 RNG。技能/额外攻击进入派生管线，regeneration 每 10 world ticks 恢复 1 HP；协议 1.103、包 1.94.0、Schema v42、fixtures 327-328 共 328 exact。真实导入 ego 128/160（+21），其余 32 个主要依赖反射、光环、诅咒、额外射击/威力与高级品牌 |
| P52 | 候选：职业壳 + m_info 施法档案导入（T3） | 下一候选 | 54 职业 stats/skills/casting 壳与领域可读性表；备选插队：设备效果系统（缺口 231+激活 193）、法术清尾（S_ 字形 177/SHRIEK/TRAPS） |

## contract-v103 明确遗留

- 首版 passive 中仅 regeneration 已有独立运行时消费；see-invisible、telepathy、levitation、hold-life、六维 sustain、blessed、easy-spell 与 device-power 已入内容/存档/DTO，分别等待视野、经验吸取、属性损伤、施法和设备系统接入；
- equipmentBonuses 中近战攻击次数与现有十类技能已经进入派生管线；infravision/light-radius 已保存和显示，但真实视觉半径消费仍待多视觉通道迭代；
- 旧存档缺 rolledAffixes 时刻意保持空，绝不按当前 rollGroups 补抽；内容作者修改候选只影响未来生成的物品；
- 剩余 32/160 ego 的主要系统缺口是反射、伤害光环、诅咒、额外射击/威力、高级品牌、随机抗性/维持配方与设备/法术威力。

## contract-v99 明确遗留

- amnesia 只清当前层地图记忆（原版 lose_all_info 忘全部楼层与物品感知；物品知识按长期约束保持存档级权威）；
- drain-resource 吸取施法档资源池（原版限 SP 且有 mana_loss_save 二段豁免；吸取量固定非随机）；无池玩家吸取 0；
- teleport-away 对玩家召唤物直接重定位（原版决斗家 30 级可对抗传送——反制机制未建立）；
- DISPEL 仅驱散加速（玩家增益现役唯一）；增益族扩展后驱散列表同步扩展；
- DARKNESS 85（房间光照状态未建立）、ANIM_DEAD 58（无尸体）、ANTI_MAGIC 47、TELE_LEVEL 25（跨层强制传送）、HAND_DOOM 30 继续留缺口。

## contract-v98 明确遗留

- 豁免公式为 v72 中性检定（技能 vs 3/4 难度阈值 + 5%/5% 自动窗口），非原版连续概率（sav×100/(100+rlev/2+dam/5)）；dam/5 难度加成未纳入；
- 装备诅咒副作用（curse_equipment）无对应系统；HAND_DOOM（当前 HP 百分比 + 豁免，30 实例）留缺口；
- 玩家召唤物作为诅咒目标时无豁免（无豁免技能概念），直接全额；
- 心灵族（MIND_BLAST dam/5、BRAIN_SMASH dam/3）的豁免门回补待排期——机制已就位，只差在 Sequence 骑手前插入豁免检定的形态设计。

## contract-v97 明确遗留

- 心灵族的原版豁免门（MIND_BLAST dam/5、BRAIN_SMASH dam/3 全免）未复刻：骑手恒定施加、psi 抗性缩时减伤代偿；未来 CAUSE 豁免迭代建立中性豁免检定后可回补；
- 吸蓝副作用（MIND_BLAST 50 / BRAIN_SMASH 100）归 DRAIN_MANA 资源吸取族；幻觉（chaos image）无对应状态，均留缺口；
- PSI/PSI_DRAIN/PSI_EGO_WHIP/PSI_STORM/DOMINATION/SUBJUGATION 等进阶心灵 GF 未出现在 r_info S: 行（多为玩家侧异能），暂不映射。

## contract-v96 明确遗留

- 玩家侧抗性来源（种族/职业/装备的内容层声明）未建立；玩家 resistances 仍只由既有运行时来源决定；
- 抗性档只在生成时盖章；临时抗性 buff、装备切换即时重算等动态来源等实际内容需要时接入；
- RES_TELE 238（传送抗性，等位移抗性机制）、RES_WALL 34（语义待核）、HURT_ROCK 29（削岩弱点）留缺口；
- 怪物抗性对玩家不可见（快照不暴露）；玩家侧的怪物回忆/知识系统仍未建立。

## contract-v95 明确遗留

- 类别召唤首版仅限 monsterCasting；玩家侧类别召唤等实际内容需要时开放；
- 落点沿用 v82 的自身半径规范序（原版优先玩家附近），召唤物时长以 10 000 玩家回合近似原版永久，均记为已知中性化差异；
- HI_ 系折算为同类别同等级上限（原版为更深怪物类选择）；S_ANGEL（无类型旗标）、S_HOUND/S_SPIDER 等字形子类与 S_SPECIAL/UNIQUE/CYBER/PANTHEON/AMBERITE 等特殊类共 177 实例留缺口——字形子类可考虑按 glyph 派生标签的后续小步；
- 尚无召唤物种群上限、繁殖与 unique 过滤；原版 S_UNIQUE/S_GUARDIAN 依赖 unique 生态。

## contract-v94 明确遗留

- 吐息首版仅限 monsterCasting（玩家目标规划层拒绝）；玩家侧吐息（龙形态等）等身体/变身系统再开放；
- 吐息锥形沿用 v79 横向整数衰减；原版吐息在锥内不衰减的差异记为已知近似；
- 异种元素吐息（BR_NETHER 51/DARK 50/CHAOS 36/NEXUS 35/SOUND 29/DISEN 27/DISI 27/LITE 26/SHARDS 24/CONF 26/TIME 22/FORCE 21/MANA 21/INERTIA 18/GRAVITY 17/STORM 16 等约 453 实例）与异种直伤（BO 135+BA 167）等待「伤害类型扩展」迭代：物理/五元素之外的伤害类型需要核心 DamageType、抗性档与存档扩展，硬折 physical 会让护甲错误参与减免；
- CAUSE_1–4 诅咒直伤（240 实例，固定骰 3d8/8d8/10d15/15d15）原版走豁免检定而非护甲/抗性，需先决定中性豁免机制归属；
- MIND_BLAST（63）/BRAIN_SMASH（123）为心灵伤害 + 状态骑手组合，伤害类型与豁免同上，组合形态可复用既有 Sequence 白名单；
- S_ 召唤族（670 实例）需要按类别/深度选择怪物的新召唤形态（现有 Summon 只支持固定 actor kind）；S_KIN（76）可率先映射为召唤施法者同类；
- TELE_OTHER（69，把玩家推远）、TELE_LEVEL（25，跨层）、DRAIN_MANA（83，吸资源）、DARKNESS（85，压制光照）、AMNESIA（64，清除知识）各需小型新效果形态，按覆盖数排期；
- 施法表上限已提至 64（当前旧版最大者 34 技能，零截断）；导入器保留 64 截断守卫，若未来映射族继续扩张再评估上限或按权重挑选保留集。

## contract-v90 明确遗留

- 玩家召唤物已执行 Follow/Attack/Keep Distance/Guard 全局命令，但尚无单体点名、召回、永久宠物、物品交互、法术施放或更复杂阵形；
- 选择层已按 HP、状态、距离、敌我目标数量和已观察抗性调整有效权重，但尚未按精确伤害期望、逃跑路径长度、协同法术或群体角色建立更完整评分；
- 怪物召唤物已是 hostile 并可执行普通 AI，但没有召唤命令、主人死亡联动、种群上限、unique 过滤或繁殖规则；
- 怪物位移、地形、侦测、反制和特殊投射效果仍未开放；
- 怪物首版不消费 Mana、学习、熟练度、失败率或玩家能力冷却；只使用百分比频率与按自身行动计数的逆频率冷却；
- smart caster 只学习当前六类伤害抗性；原版更广的反射、自由行动、传送抗性、遗忘/误导，以及反制、沉默、施法打断、领域协同和完整怪物法术表仍未建立；
- 多职业资源底子已建立并接入首个技法资源，但受击获得、持续吟唱逐回合扣费、姿态切换与资源联动（例如满值增益）尚未实现；
- 装备激活与设备共享能力继续后置。

contract-v89 已将玩家召唤物行动与全局命令接入协议 1.89、内容包 1.80.0、save v1 与 state hash Schema v39。命令零世界时间；移动和目标选择零 RNG；近战复用 actor routine 与玩家击杀归属。切层时仅 2 格内召唤物跟随，远处实体留层，Guard 锚点重置为到达位置。active baseline 为 272 个 exact fixtures、零 waiver。详细边界见 [contract-v89](contract-v89-friendly-summon-commands.md)。

## contract-v85 明确遗留

- sequence 首版只组合同一 actor 目标上的伤害、治疗和状态；多目标、terrain、召唤、侦测、位移等专用效果尚未进入组合器；
- 状态持续时间当前为固定整数并通过既有元素抗性确定性缩放；随机持续时间、独立 saving throw 和更复杂驱散优先级尚未建立；
- 仍缺 confusion、paralysis、blindness、sleep 等完整状态族及其对应行动规则；
- 怪物施法、HP/状态/距离/敌我/抗性能力选择、战术移动与友方召唤命令已由 contract-v86–v89 建立；装备激活、设备共享能力与多资源职业仍未实现；
- 原版完整法术书、法术顺序和按等级自动遗忘/记起模型尚未建立。

contract-v85 已将 Echo Quickening/Echo Binding 接入协议 1.85、内容包 1.77.0、save v1 与 state hash Schema v36。整次施法只支付一次资源并先抽一次失败率，子效果按声明顺序结算；前序击杀会把后续效果标为 `target-dead`，无 actor 命中标为 `no-target`，且不抽取被跳过的伤害骰。状态沿用既有 actor status 存档与 tick 管线，cold 抗性确定性缩短 slow，免疫返回零持续时间。该历史 baseline 为 242 个 exact fixtures、零 waiver。详细边界见 [contract-v85](contract-v85-ordered-status-effects.md)。

## contract-v84 明确遗留

- terrain 变换沿射线、锥形或任意图案传播，以及随机地震、塌方、液体流动和持续环境效果；
- 对 actor/物品造成伴随伤害、掩埋、推动或销毁的复合 terrain 效果；
- 玩家施法以外的挖掘设备、装备激活、怪物破墙/造墙和 AI 决策；
- 状态能力和首版多 effect 组合已由 contract-v85 完成；多资源职业仍未实现；
- 原版完整法术书、法术顺序和按等级自动遗忘/记起模型。

contract-v84 已将 Echo Delving/Echo Rampart 接入协议 1.84、内容包 1.76.0、save v1 与 state hash Schema v36。候选按 RFB 距离与坐标稳定排序，只处理当前 FOV 和合法来源 terrain，并跳过占用格、连接、入口标签和地图边界；候选在资源/RNG 前收集，成功后一次提交，失败不改 terrain，空结果仍正常施法。该历史 baseline 为 231 个 exact fixtures、零 waiver。详细边界见 [contract-v84](contract-v84-terrain-transform-ability.md)。

## contract-v83 明确遗留

- 完整地图、怪物、物品、楼梯和陷阱等更多侦测类别及对应知识菜单；
- 侦测范围的特殊穿墙、全层感知、持续 buff、黑暗/失明/反侦测修正；
- 地形改变已由 contract-v84 完成，状态能力和首版多 effect 组合已由 contract-v85 完成；多资源职业仍未实现；
- 怪物更完整的目标/知识 AI、装备激活与设备共享能力；
- 原版完整法术书、法术顺序和按等级自动遗忘/记起模型。

contract-v83 已将 Echo Pulse/Echo Sight 接入协议 1.83、内容包 1.75.0、save v1 与 state hash Schema v36。侦测只考虑当前地图、半径、玩家 FOV、隐藏投影与 category tag，按距离/坐标稳定排序；瞬时结果只进入事件，持久结果写入 `revealedTerrain`。空结果仍按正常施法消费资源和 RNG，非法目标与资源不足不推进 RNG。active baseline 为 221 个 exact fixtures、零 waiver。详细边界见 [contract-v83](contract-v83-detection-ability.md)。

## contract-v82 明确遗留

- 跟随、攻击、保持距离、守卫、玩家命令与附近跨层跟随已由 contract-v89 完成；仍缺单体命令、召回和永久宠物；
- 敌对/中立召唤、怪物召唤能力和能力选择 AI；
- 召唤物跨楼层、召回、永久宠物、繁殖、唯一性与复杂 pack/formation 组合；
- 侦测已由 contract-v83 完成，地形改变已由 contract-v84 完成，状态能力和首版多 effect 组合已由 contract-v85 完成；多资源职业仍未实现；
- 原版完整法术书、法术顺序和按等级自动遗忘/记起模型。

contract-v82 已将 Echo Companion 接入协议 1.82、内容包 1.74.0、save v1 与 state hash Schema v35。空间候选按距离/坐标稳定排序，玩家、actor 与地面物品都占用格；空间不足在 Mana、施法 RNG 与熟练度前原子拒绝。成功召唤保存 owner/source/lifetime，召唤物不参加敌对 AI或可见敌人判断，并按玩家回合到期移除。active baseline 为 213 个 exact fixtures、零 waiver。详细边界见 [contract-v82](contract-v82-summon-ability.md)。

## contract-v81 明确遗留

- 召唤、侦测和地形改变已由 contract-v82–v84 完成；
- 传送到不可见、不可行走或被 actor 占用格以外的复杂位移规则（随机传送、穿墙、跨层和群体传送）；
- 射线范围内物品破坏、地形变更或玩家伤害；
- 射线反射、穿透墙体例外；怪物对玩家召唤物的目标选择已由 contract-v88 完成；
- 多资源职业和怪物能力选择/施法 AI；
- 原版完整法术书、法术顺序和按等级自动遗忘/记起模型。

contract-v81 已将内容驱动的 Echo Step 接入协议 1.81、内容包 1.73.0、save v1 与 state hash Schema v34。position 落点的地图内、Chebyshev 射程、可见性、line of effect、可行走和 actor 占用验证在 Mana、施法 RNG 与熟练度前执行；成功传送复用普通移动的被动感知、陷阱触发和死亡处理。active baseline 为 209 个 exact fixtures、零 waiver。详细边界见 [contract-v81](contract-v81-teleport-ability.md)。

## contract-v80 明确遗留

- 位移、召唤、侦测和地形改变已由 contract-v81–v84 完成；
- 射线范围内物品破坏、地形变更或玩家伤害；
- 射线反射、穿透墙体例外和怪物多目标价值评分；
- 多资源职业和怪物能力选择/施法 AI；
- 原版完整法术书、法术顺序和按等级自动遗忘/记起模型。

RFB 式方向射线、actor 穿透、墙体截断、近到远顺序、共享单次伤害骰、空射/无效模式 RNG 边界和 Echo Lance 已由 contract-v78 建立；固定八向锥形已由 contract-v79 建立；定点/实体延长射线、稳定整数斜率、目标验证和延长后的墙体截断已进入协议 1.80、内容包 1.72.0、save v1 与 state hash Schema v34；active baseline 为 202 个 exact fixtures、零 waiver。详细边界见 [contract-v80](contract-v80-targeted-beam-extension.md)。

## contract-v77 明确遗留

- 锥形、定点延长射线、位移、召唤、侦测和地形改变已由 contract-v79–v84 完成；
- 范围内物品破坏、地形变更或玩家伤害；
- 多资源职业和怪物能力选择/施法 AI；
- 原版完整法术书、法术顺序和按等级自动遗忘/记起模型。

RFB 式范围爆发、目标停止策略、墙体遮挡、距离衰减、空爆/无效目标 RNG 边界和 Echo Burst 已进入协议 1.77、内容包 1.69.0、save v1 与 state hash Schema v34；active baseline 为 190 个 exact fixtures、零 waiver。详细边界见 [contract-v77](contract-v77-area-damage.md)。

## contract-v76 明确遗留

- 随机学习、首次成功奖励，以及原版按 `spell_order` 自动暂时遗忘/记起；
- 怒气、专注、鲜血等多种职业资源，资源互转与职业专属恢复条件；
- 范围、锥形、位移、召唤、侦测和地形改变已由 contract-v77–v84 完成；状态能力和首版多 effect 组合已由 contract-v85 完成；
- 装备负重、状态、环境与职业规则对失败率、恢复率和效果强度的完整修正；
- 怪物基础施法/效用已完成；仍缺智能学习和完整领域/职业矩阵；
- 饥饿、HP 自然恢复、旅行、自动探索和更高层的安全休息策略。

v76 的独立学习容量、能力容量投影、主动遗忘、重新学习进度保留、容量满零 RNG 拒绝和旧存档兼容已进入协议 1.76、内容包 1.68.0、save v1 与 state hash Schema v34；active baseline 为 186 个 exact fixtures、零 waiver。详细边界见 [contract-v76](contract-v76-learning-capacity-and-forgetting.md)。

## contract-v73 明确遗留

- Mana 等待/休息恢复、恢复中断、自身目标和固定治疗已由 contract-v74 完成；多种职业资源仍未实现；
- 学习容量、随机学习、遗忘、首次施放奖励、熟练度和冷却；
- 自身目标与治疗已由 contract-v74 完成；范围爆发、方向射线、锥形、定点/实体延长射线、位移、召唤、侦测和地形改变已由 contract-v77–v84 完成；状态能力和首版多 effect 组合已由 contract-v85 完成；
- 装备负重、状态、环境和职业规则对失败率的完整修正；
- 怪物基础施法/效用已完成；仍缺智能学习和完整领域/职业矩阵。

v73 的 Mana、能力书、学习、失败率、目标施法、结构化 ability-cast outcome、旧存档满资源/空已学迁移和 166 个 exact fixtures 已进入协议 1.73、save v1 与 state hash Schema v32。详细边界见 [contract-v73](contract-v73-ability-books.md)。

## contract-v72 明确遗留

- 技能练习/下降、属性损伤/恢复、职业专属资源和更复杂等级奖励；
- 失明、无光、混乱、幻觉、距离、噪声和环境亮度对技能检定的完整修正；
- 怪物警戒传播、睡眠深度、气味/flow、智能学习和完整潜行模式；
- 法力、能力书、学习、失败率、恢复、熟练度和冷却已由 contract-v73–v75 完成；玩家/怪物完整施法系统仍未实现；
- 完整原版 Race/Class/Personality 名单、创建 UI 和角色重建流程。

v72 的四类技能消费、结构化 check outcome、actor `alerted` 兼容恢复和 160 个 exact fixtures 已进入协议 1.72、save v1 与 state hash Schema v31。详细边界见 [contract-v72](contract-v72-observable-skill-checks.md)。

## contract-v71 明确遗留

- 完整原版 Race/Class/Personality 名单、职业选择界面和角色重建流程；
- 技能练习/下降、属性损伤/恢复、职业专属资源和更复杂的等级奖励；
- device、saving-throw、stealth、perception 的实际检定消费已由 contract-v72 完成；
- Mana、能力书、失败率、恢复、自身治疗、熟练度、冷却、学习容量和遗忘已由 contract-v73–v76 完成；随机学习、完整法术系统和职业专属资源仍未实现。

v71 的构筑身份、技能聚合、出生装备和 v70 缺字段迁移已进入协议 1.71、save v1 与 state hash Schema v30；v72 在其上补齐四类首轮技能消费。详细边界见 [contract-v71](contract-v71-rfb-character-builds.md)。

## contract-v70 明确遗留

- Race/Class/Personality 角色创建、种族/职业成长曲线和初始装备模板；
- 属性损伤、临时恢复、技能熟练、经验倍率和职业专属资源；
- Mana、能力书、失败率、恢复、自身治疗、熟练度和冷却已由 contract-v73–v75 完成；职业专属资源仍未实现；
- 属性点重置、自动分配策略和更复杂的等级奖励节点。

v70 的经验、HP 序列、自然属性、装备 modifier 和胜利解锁已进入协议 1.70、save v1 与 state hash Schema v29；v71 已在其上补齐角色来源层。详细边界见 [contract-v70](contract-v70-rfb-character-progression.md)。

## contract-v69 明确遗留

- 运行时地形破坏不触发自动重连；破坏结果直接写入地图，玩家可使用挖掘能力自行恢复通路；
- 非楼梯回忆、传送、死亡退出与实例生命周期的统一结算；
- 并行实例 UI 选择、跨实例传送和多 retained 实例并存仍不在当前规则内。

Archive Depths 是 `turn-ttl=3` 的 demo 验证包；Echo/Resonance 继续使用默认 reset。来源：[contract-v69](contract-v69-configurable-instance-lifecycle.md)。

## contract-v68 明确遗留

- 原创 dungeon 的可配置永久实例、TTL/淘汰与显式生命周期策略已由 contract-v69 完成；
- 运行时地形破坏不触发自动重连；破坏结果直接写入地图，玩家可使用挖掘能力自行恢复通路。

原版 demo dungeon 不使用硬进入条件；玩家仍可绕过入口守卫直接进入。Resonance 是 demo campaign 唯一 victory dungeon；Echo 征服只增加分数。胜利后必须在地表退休，退休冻结最终分数并结束 dispatch。普通 dungeon 返回地表即清理，下一次进入重新生成。

来源：[contract-v68](contract-v68-victory-retirement-scoring.md)。

## contract-v67 明确遗留

- 地牢征服后的胜利/退休结算和角色评分已由 contract-v68 完成；
- 原创 dungeon 的可配置永久实例、TTL/淘汰与显式实例选择；
- 运行时地形破坏不触发自动重连；破坏结果直接写入地图，玩家可使用挖掘能力自行恢复通路。

来源：[contract-v67](contract-v67-dungeon-entrance-guardians.md)。

## contract-v66 明确遗留

- 多 dungeon 的可选进入条件已由 contract-v67 完成，胜利/退休评分已由 contract-v68 完成；仍缺面向未来原创内容的可配置生命周期策略；
- 运行时地形破坏不触发自动重连；破坏结果直接写入地图，玩家可使用挖掘能力自行恢复通路。

同一 dungeon 的暂停实例 UI 选择、并行访问和跨实例传送不在当前规则内：普通 dungeon 返回地表即清理，下一次进入重新生成。

来源：[contract-v66](contract-v66-dynamic-exploration-tree.md)。

## contract-v65 明确遗留

- 动态楼梯目标与实例级探索树已由 contract-v66 完成；
- 可配置实例生命周期策略；多 dungeon 进入条件与胜利/退休评分已由 contract-v67/v68 完成；
- 运行时地形破坏不触发自动重连；破坏结果直接写入地图，玩家可使用挖掘能力自行恢复通路。

来源：[contract-v65](contract-v65-dungeon-instance-identity.md)。

## contract-v64 明确遗留

- 同一楼层模板生成多个运行时实例、显式 `DungeonInstanceId`、楼层淘汰和更一般的动态探索树；
- 多座地牢同时存在的探索实例和可配置重置策略；进入条件、胜利/退休与角色分数已由 contract-v67/v68 完成；
- 运行时地形破坏不触发自动重连；任意多边形/噪声区域连接和跨区域群体协作仍不在规则内。

来源：[contract-v64](contract-v64-multi-entry-vault-connectivity.md)。

## contract-v63 明确遗留

- Vault 多入口、大模板成功落位后的连通性证明与跨走廊拼接已由 contract-v64 完成；
- 同一楼层模板生成多个运行时实例、楼层淘汰和更一般的动态探索树；
- 多座地牢同时存在的探索实例与可配置重置策略；显式 `DungeonInstanceId`、进入条件、胜利/退休和角色分数已由 contract-v65/v67/v68 建立。

来源：[contract-v63](contract-v63-dungeon-tree-guardian-mirrors.md)。

## contract-v62 明确遗留

- 任意多边形/噪声区域边界、走廊区域归属、区域专属门和跨区域群体协作；
- 多个 pit、独立 nest 房间、任意 formation 模板、召唤、繁殖与种群上限；
- 树状地牢与不同楼梯进入不同子层已由 contract-v63 完成；Vault 多入口、大模板连通性证明和跨走廊拼接已由 contract-v64 完成。

来源：[contract-v62](contract-v62-regional-composition.md)。

## contract-v61 明确遗留

- 超时、失败惩罚、任务接取确认和脚本回调；
- 重接后重置进度、按目标类型选择性重建，以及运行时手动选择重建策略；
- 分支/并行阶段、单阶段多目标、任务内部上下层连接和独立 quest 模块。

来源：[contract-v61](contract-v61-retake-management.md)。

## contract-v60 明确遗留

- 区域与 Vault、pit、dynamic formation、terrain feature 和分阶段地貌的组合已由 contract-v62 完成；独立 nest 房间仍未实现；
- 任意多边形/噪声边界、走廊区域归属、区域专属门和跨区域群体协作；
- cavern/lake/river 等非房间空间的区域归属，以及更一般的多入口连通图。

来源：[contract-v60](contract-v60-regional-themes.md)。

## contract-v59 明确遗留

- 怪物开门/破门、远程攻击选择、逃跑、召唤、繁殖、种群上限和 unique 过滤；
- 任意半径/模板 formation、跨房间群体和跨阻断区域连通性修复；
- 更复杂的阵营关系、气味/flow、特殊感知和 pack 间战术协作。

来源：[contract-v59](contract-v59-pack-ai.md)。

## contract-v32 明确遗留

- 解除失败触发陷阱、重复解除命令和经验奖励；
- 箱子陷阱、随机陷阱类型、状态/传送/落层等复杂效果；
- 移动后邻近 terrain 的 perception 已由 contract-v72 建立；仍缺隐藏陷阱的被动发现、失明/无光/混乱/幻觉修正，以及怪物触发或规避陷阱；
- 一次性/耗尽陷阱、陷阱生成密度和多深度内容表。

来源：[contract-v32](contract-v32-hidden-traps-disarm.md)。

## contract-v33 明确遗留

- 镐、铲、重武器等装备提供的挖掘能力与物品描述；
- 自动重复挖掘、疲劳、声音、德行和挖掘秘密门时的偶发搜索；
- 树木、矿脉、玻璃、永久岩石等不同破坏规则与产物；
- 原版“怪物挡路时转为攻击”的兼容语义；当前核心与权威查询统一拒绝被占据目标。

来源：[contract-v33](contract-v33-diggable-terrain.md)。

## contract-v34 明确遗留

- 分支楼梯已由 contract-v63 完成；同层多个连接点、连接 ID 与到达点分别建模已由 contract-v58 完成；
- 随机楼梯位置、回忆/传送等非楼梯跨层入口；
- 深度相关 encounter/loot/theme 表已由 contract-v48 完成；树状分支、多个最终层和共享守护者镜像已由 contract-v63 完成；
- 旧 v33 已访问深度 1 不补下楼梯，因此不能从旧存档进入新深度 2；需要正式存档迁移策略。

来源：[contract-v34](contract-v34-multi-depth-floors.md)。

## contract-v35 明确遗留

- 多座地牢各自独立的活跃探索实例与同时存在规则；
- 地牢中途传送回城、回忆、死亡或任务退出时统一结束探索；
- 明确的 `DungeonInstanceId`，以及分支入口对应不同实例；稳定 `dungeonId` 与持久守护者状态已由 contract-v46 完成；
- 可配置的永久地牢或重置策略；当前所有程序化地牢从入口返回地表都会清除。

来源：[contract-v35](contract-v35-dungeon-expedition-lifecycle.md)。

## contract-v36 明确遗留

- 当前退出一次性任务层即关闭入口，尚未区分完成、失败和放弃；
- 护送和到达位置目标；有序多阶段任务已由 contract-v45 完成，击杀与收集目标已由 contract-v37/v39 建立；
- 任务奖励、任务日志、重新接取和可重复任务；
- 固定手工地图、任务专属生成器及禁止离开的任务规则。

来源：[contract-v36](contract-v36-one-shot-task-floor.md)。

## contract-v37 明确遗留

- 计数收集、护送和到达位置目标；有序多阶段任务已由 contract-v45 完成，单实例击杀已由 contract-v39 完成；
- 主动放弃与失败的区分、禁止提前退出已由 contract-v40 完成；尚缺超时失败；
- 任务奖励、任务日志、重新接取和可重复任务；
- 目标物被丢弃、销毁、投掷或带出后再处理的完整规则。

来源：[contract-v37](contract-v37-task-objective-resolution.md)。

## contract-v38 明确遗留

- 奖励选择、随机奖励、领取确认和容量不足处理；
- 多任务排序、任务详情与历史记录；目标进度数字已由 contract-v39 完成；
- 可重复任务、重新接取、奖励已领取状态；
- 奖励和任务状态改为独立领域状态，而不是完全由 terrain/物品派生。

来源：[contract-v38](contract-v38-task-reward-journal.md)。

## contract-v39 明确遗留

- 按怪物种类累计多次击杀与持久计数已由 contract-v41 完成；尚缺跨楼层共享同一任务进度；
- 清空楼层、unique、随机目标、召唤物过滤和更通用的死亡订阅边界；
- 主动放弃与退出限制已由 contract-v40 完成，可重接暂离由 contract-v42 完成；尚缺超时失败和失败后的重新接取；
- 一个阶段内的多目标、分支阶段，以及独立 quest 模块；有序多阶段任务和持久任务状态已由 contract-v45/v44 完成。

来源：[contract-v39](contract-v39-kill-objective-progress.md)。

## contract-v40 明确遗留

- 可重接任务和保留任务层已由 contract-v42 完成；尚缺失败后的重新接取流程；
- 离开前确认对话框，以及失败/放弃后的惩罚或脚本回调；
- 死亡、回忆、传送和其他非楼梯离开方式统一进入任务结算；
- 超时失败，以及禁止离开但仍允许主动放弃之外的更多退出政策。

来源：[contract-v40](contract-v40-task-abandon-exit-policy.md)。

## contract-v41 明确遗留

- 多个入口楼层共享任务 ID 和计数状态已由 contract-v43 完成；尚缺任务内部的上下层连接；
- 可重接任务离开、重新进入后继续累计已由 contract-v42 完成，暂停时在地表主动最终放弃已由 contract-v61 完成；
- 召唤物、复制体、非玩家击杀和环境击杀的可配置计数规则；
- unique、随机任务、清空楼层和阶段内多目标；有序多阶段任务已由 contract-v45 完成。

来源：[contract-v41](contract-v41-counted-kill-progress.md)。

## contract-v42 明确遗留

- 独立于 floor ID 的任务 ID 和多个入口楼层共享任务已由 contract-v43 完成；
- 暂停状态下从地表直接最终放弃与重接次数限制已由 contract-v61 完成；尚缺超时；
- 保留完整楼层和“保留进度、重建成员层/剩余计数目标”已由 contract-v61 完成；尚缺重置进度、按目标类型选择性重建和玩家手动选择；
- 任务接取确认、失败惩罚和脚本回调。

来源：[contract-v42](contract-v42-retakeable-task.md)。

## contract-v43 明确遗留

- 同一任务内的直接上下层连接；并列入口之间的有序阶段已由 contract-v45 完成；
- 一个阶段内声明多个同时目标；跨成员楼层的收集、进入和击杀组合已由 contract-v45 完成；
- 独立任务名称、描述、接取来源和任务详情领域实体；
- 多任务并行追踪、排序、筛选和当前追踪目标。

来源：[contract-v43](contract-v43-shared-task-id.md)。

## contract-v44 明确遗留

- 分支、可选、并行阶段和一个阶段内的多个同时目标；有序单目标阶段已由 contract-v45 完成；
- 将任务状态机从游戏聚合继续拆分为独立 quest 模块；
- 任务接取来源、详情、脚本回调、超时与失败惩罚；
- 多任务追踪选择，以及非玩家/环境击杀的可配置计数来源。

来源：[contract-v44](contract-v44-task-state-machine.md)。

## contract-v45 明确遗留

- 分支、可选和并行阶段，以及一个阶段内的多个同时目标；
- 阶段级奖励、失败政策、脚本回调与阶段描述文本；
- 暂停状态下从地表主动放弃、重接次数限制和保留进度的成员楼层重建已由 contract-v61 完成；尚缺重置进度和选择性重建；
- 独立 quest 模块、任务接取来源与多任务追踪选择；
- 更通用的到达位置目标，以及环境/非玩家击杀的可配置计数来源。

来源：[contract-v45](contract-v45-ordered-task-stages.md)。

## contract-v46 明确遗留

- 单根树状分支、普通楼梯与 shaft 已由 contract-v58/v63 完成，跨走廊连接与 Vault 多入口已由 contract-v64 完成；仍缺运行时动态探索树；
- vault 内的深度 encounter、主题 terrain/loot 和固定群体已由 contract-v47 建立；楼层级表、多个 vault 加权选择和第一类巢穴已由 contract-v48 建立；十层规模、actor/loot 预算和深度区域主题已由 contract-v49 建立；旋转/镜像、自由落位、多 Vault 空间预算和失败回退已由 contract-v50 建立；动态 friends/escort formation 与群体预算已由 contract-v51 建立；多入口与大模板成功落位后的连通性证明已由 contract-v64 完成；
- 入口守护者、守护者 unique 世界生态，以及神器、声望和属性奖励；
- 多座地牢、进入条件、显式 `DungeonInstanceId`、胜利/退休和角色分数；
- 回忆、传送、死亡等非楼梯方式结束探索时的统一生命周期。

来源：[contract-v46](contract-v46-final-floor-guardian.md)。

## contract-v47 明确遗留

- 按深度和地牢主题加权选择多个 vault、无候选回退已由 contract-v48 建立；旋转、镜像、自由 wall 区落位、多 Vault 同层和生成失败回退已由 contract-v50 建立，多入口和跨走廊拼接已由 contract-v64 完成；
- 普通房间可引用的独立 encounter/loot/theme 表已由 contract-v48 建立，actor/loot 总预算与深度区域主题已由 contract-v49 建立，第一版 Vault 数量/面积预算已由 contract-v50 建立，额外 trap/door/rubble 表与 feature 预算已由 contract-v52 建立，房间数量/形状/面积预算与连通 cavern 基础地貌已由 contract-v53 建立，深浅 lake/river 水文阶段已由 contract-v54 建立，maze/destroyed/streamer 阶段已由 contract-v55 建立，完全替代房间的 maze-only 专用楼层已由 contract-v57 建立，同层房间区域与局部表已由 contract-v60 建立，区域与 Vault、pit、地貌、feature 和群体的组合已由 contract-v62 建立，多入口与跨走廊拼接已由 contract-v64 完成；
- 第一类同类巢穴已由 contract-v48 建立，动态 friends/escort、`cluster/ring` formation 和群体预算已由 contract-v51 建立，原版式独立复合 pit 与等级阵列已由 contract-v56 建立，持久 pack identity 与首版 AI 已由 contract-v59 建立；尚缺任意模板 formation、多个 pit、召唤、繁殖、种群上限、unique 过滤和更复杂 AI；
- vault 越级强敌/掉落、专属陷阱、神器、来源标签和探索奖励；
- 十层规模压力场景已由 contract-v49 建立，多 Vault 楼层已由 contract-v50 建立，更大模板成功落位后的连通性证明和多入口已由 contract-v64 完成。

来源：[contract-v47](contract-v47-themed-vault.md)。

## contract-v48 明确遗留

- 十层地牢、actor/loot 生成预算和深度区域主题已由 contract-v49 建立，多个 Vault 同层和第一版面积预算已由 contract-v50 建立，额外陷阱/门/障碍空间预算已由 contract-v52 建立，房间几何预算与 cavern 基底已由 contract-v53 建立，lake/river/maze/destroyed/streamer 已由 contract-v54–v55 建立；尚缺机器性能计时基线与更大地图压力场景；
- Vault 旋转、镜像、自由 wall 区落位和失败重试已由 contract-v50 建立；多入口与大模板成功落位后的连通性证明已由 contract-v64 完成；
- 动态 friends/escort、`cluster/ring` formation 与领袖/随从预算已由 contract-v51 建立，pit 专属表与等级阵列已由 contract-v56 建立，持久 pack identity 与首版 AI 已由 contract-v59 建立；尚缺独立 nest 房间、任意模板 formation、主题掉落和跨房间协作；
- unique/守护者过滤、召唤物与繁殖种群上限、越级强敌/掉落和神器来源标签；
- 树状分支已由 contract-v63 完成；跨走廊拼接和 Vault 多入口已由 contract-v64 完成，shaft、随机楼梯、同层多个连接点与显式到达点已由 contract-v58 建立。

来源：[contract-v48](contract-v48-floor-generation-tables.md)。

## contract-v49–v53 明确遗留

- Vault 旋转、镜像、自由 wall 区落位、多 Vault 预算竞争、重叠拒绝和稳定失败回退已由 contract-v50 建立；多入口、大模板成功落位后的连通性证明和跨走廊拼接已由 contract-v64 完成；
- 额外陷阱、门与可挖掘特殊地形表、room/corridor 放置、空间预算和失败回退已由 contract-v52 建立；房间数量/尺寸/rectangle-cross 形状/面积预算、连通 cavern 基地貌和跨房间内容分布已由 contract-v53 建立；深浅 lake/river 与结构连通保护已由 contract-v54 建立；maze/destroyed/streamer 与墙体限定回退已由 contract-v55 建立；maze-only、远距锚点和区域内容落位已由 contract-v57 建立；同层区域主题与走廊拼接带已由 contract-v60 建立，与区域组合已由 contract-v62 完成；尚缺 feature 分类型配额和相邻限制；
- friends/escort、`cluster/ring` formation、群体数量/随从预算、空间缩减和原子回退已由 contract-v51 建立，复合 pit、单入口、专属表和中心等级阵列已由 contract-v56 建立，持久 pack identity 与首版 AI 已由 contract-v59 建立；尚缺任意模板 formation、多个 pit、召唤、繁殖、种群上限、unique 过滤和更复杂 AI；
- 更一般的分支连接仍缺；Vault 跨走廊拼接已由 contract-v64 完成，shaft、随机楼梯、同层多个连接点与独立到达点已由 contract-v58 建立；
- 跨机器性能计时基线；当前十层 fixture 只锁定规模、状态和确定性。

来源：[contract-v49](contract-v49-budgeted-pressure-dungeon.md)、[contract-v50](contract-v50-spatial-vault-placement.md)、[contract-v51](contract-v51-dynamic-encounter-groups.md)、[contract-v52](contract-v52-terrain-feature-budgets.md)、[contract-v53](contract-v53-staged-cavern-layout.md)、[contract-v54](contract-v54-lake-river-hydrology.md)、[contract-v55](contract-v55-maze-destroyed-streamers.md)、[contract-v56](contract-v56-classic-monster-pit.md)、[contract-v57](contract-v57-maze-only-floor.md)、[contract-v58](contract-v58-floor-connections.md)、[contract-v59](contract-v59-pack-ai.md)。

## contract-v25–v29 明确遗留

### 怪物携带物与掉落

- 偷窃、缴械、怪物主动拾物和怪物使用物品；
- 多次掉落、区域主题掉落、unique 过滤和特殊怪物掉落规则；楼层 loot 表引用已由 contract-v48 建立，vault 专属 loot 已由 contract-v47 建立；
- 统一 `DeathOutcome` 订阅边界，以及经验、任务、统计等死亡消费者。

来源：[contract-v25](contract-v25-monster-carried-items.md)、[contract-v24](contract-v24-deterministic-loot-generation.md)。

### 楼层与生成

- 多深度连接与树状分支已由 contract-v34/v58/v63 完成；Vault 跨走廊拼接已由 contract-v64 完成，仍缺旧层淘汰、同一模板多实例和更一般的动态探索树；
- 动态朋友/护卫群体、`cluster/ring` formation 与群体预算已由 contract-v51 完成，额外陷阱/门/可挖掘障碍表与空间预算已由 contract-v52 完成，房间几何预算、连通 cavern 基底与跨房间内容分布已由 contract-v53 完成，深浅 lake/river 生成阶段已由 contract-v54 完成，maze/destroyed/streamer 已由 contract-v55 完成，原版式复合 pit 与等级阵列已由 contract-v56 完成，maze-only 专用楼层已由 contract-v57 完成，多楼梯/shaft/独立到达点已由 contract-v58 完成，持久 pack identity 与首版 AI 已由 contract-v59 完成，同层房间区域与局部表已由 contract-v60 完成，区域与现有特殊阶段组合已由 contract-v62 完成，树状地牢、多个最终叶层与共享守护者镜像已由 contract-v63 完成，Vault 多入口已由 contract-v64 完成。第一类固定主题 vault/group 已由 contract-v47 完成，多 Vault 加权选择与第一类巢穴已由 contract-v48 完成，actor/loot 总预算和十层压力链已由 contract-v49 完成，Vault 变换、自由落位、多模板面积预算与失败回退已由 contract-v50 完成；
- 陷阱、秘密门和其他可变地形进入生成管线。

来源：[contract-v26](contract-v26-floor-lifecycle.md)、[contract-v27](contract-v27-procedural-room-content.md)。

### 门与地形交互

- 原版 easy-open/自动选方向；权威可查询交互列表已由 [contract-v30](contract-v30-authoritative-terrain-interactions.md) 完成；
- 卡死门、玻璃门、更复杂的秘密门变体和门上的声音/经验反馈；
- 开锁受失明、无光、混乱、幻觉影响；
- 撞门成功后自动进入门洞、普通开启/破损随机分支；
- 撞门失败后的失衡/麻痹；
- 怪物挡门时是否显式转为近战，以及怪物自身开门/破门 AI。

来源：[contract-v28](contract-v28-door-terrain-state.md)、[contract-v29](contract-v29-locked-door-checks.md)。

### 搜索与地形知识

- 基础秘密门、主动搜索和知识安全投影已由 [contract-v31](contract-v31-secret-door-search.md) 完成；
- contract-v72 已建立成功移动后的邻近 perception 检定；仍缺搜索模式/命令重复、玩家自身格搜索和原版固定 3×3 RNG 扫描；
- 失明、无光、混乱、幻觉对搜索能力的修正；
- 隐藏陷阱和箱子陷阱发现。

## 更早纵切遗留

### 战斗、状态与效果

- 玩家 on-hit effect、暴击、品牌、克制、吸血等武器效果；
- 怪物 blow 的多 effect 列表、位移与中断；
- 失明、混乱、麻痹，以及这些状态对行动和检定的统一修正；
- Mana 等待/休息恢复已由 contract-v74 完成；HP 自然恢复、饥饿、环境伤害和更一般的世界级 tick 回调仍未实现；
- 抗性与感知进入更完整的多来源派生属性。

来源：[contract-v9](contract-v9-status-resistance-effects.md) 至 [contract-v13](contract-v13-monster-melee-routines.md)。

### 射击、投掷与目标选择

- 特殊返回弹药/武器、职业折损修正和职业射击修正；
- 药水投掷破裂与落点 effect；
- 投掷目标模式、鼠标点选、路径/范围预览和投射物动画；
- 自身 `TargetSpec` 已由 contract-v74 完成；范围、锥形等模式仍未实现。

来源：[contract-v14](contract-v14-projectile-foundation.md) 至 [contract-v18](contract-v18-thrown-attacks.md)、[前端目标模式 v1](frontend-targeting-v1.md)。

### 背包、装备、鉴定与物品

- 身体槽位扩展、箭袋、容器、槽位容量和负重分级惩罚；
- 鉴定卷轴、鉴定技能、诅咒知识、伪鉴定来源和逐项属性发现；
- unique、诅咒、固定神器、随机能力、随机神器和重铸；
- 消耗品的目标、状态 effect 与多 effect 组合。

来源：[contract-v19](contract-v19-inventory-capacity.md) 至 [contract-v24](contract-v24-deterministic-loot-generation.md)。

### 怪物 AI 与知识界面

- `AiIntent`、保持距离、逃跑、守卫、射击、能力选择、气味/flow 和特殊感知；
- 怪物开门、破门、拾物、偷窃和缴械决策；
- 怪物、物品、神器、ego、地形、地牢、宠物和统计知识菜单。

来源：[RFB 全系统梳理与重构实现路线](rfb-system-implementation-roadmap.md)。

## 维护规则

- 新纵切开始时，把目标列入“当前推进顺序”并标记进行中；完成后从待实现项中移除或标记由哪个 contract 完成。
- 每次原版对比发现主动差异时，必须判断它是永久设计差异还是未来缺口；未来缺口写入本文件。
- 不把“可能有用”的新功能直接加入清单；必须能追溯到既有规划、已完成 contract 的延后说明或原版对比。
