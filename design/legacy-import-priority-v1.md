# 旧版内容导入优先级规划 v1

状态：规划基线（P47 身体/槽位模板落地后重排）。原则沿用导入线一贯打法：**先导壳 + 缺口报告量化 → 按覆盖数落地规则族 → 回灌重跑导入收割**。所有导入产物只进 `.local/packs/rfb-legacy`，仓库继续只含原创内容。

## 1. 已导入存量

f_info 地形 180 · r_info 怪物 1332（法术映射 4601 实例，结构性未映射 627）· k_info 基础物品 544/545 · e_info 词条 88/160（72 条 ego-inexpressible 等旗标系统）· a_info 神器 392/392。

## 2. 未导入内容盘点（按固定 commit）

| 内容族 | 源 | 规模 | RFB 表面就绪度 | 主要缺口 |
| --- | --- | --- | --- | --- |
| b_info 身体模板 | 数据 | 113 模板 | ✅ P47 `bodySlots` 直挂 | 无（纯解析）；挂靠对象是种族，须与种族同轮 |
| 种族 | 代码 races*.c 等 | 75（约 38 玩家 + 37 怪物种族） | 高：stats/life%/exp%/skills/bodySlots 全有 | 种族内在旗标（抗性/FREE_ACT/红外）、种族能力 powers、等级钩子 |
| 性格 | 代码 personality.c | ~20 | ✅ PersonalityDefinition | Lucky/Split 等特殊钩子记差异 |
| 职业 | 代码 classes.c + 54 个专属 .c | 54 | 中：stats/skills/castingProfile/techniqueProfiles | 每职业专属机制是大头；s_info 武器熟练无对应系统 |
| m_info 施法档案 | 数据 | 6703 行（职业×领域×法术 等级/耗魔/失败/经验） | 中：castingProfile + abilityBooks 就绪 | 领域→能力书映射；法术效果在代码侧 |
| 玩家领域法术 | 代码 do-spell 系 | 10+ 领域 × ~32 法术 | **效果词汇表大半就绪**（v73-v99 伤害/状态/召唤/位移/侦测全家族） | 逐法术映射（照搬怪物法术线方法论） |
| d_info 地牢 | 数据 | 44 条目 | 高：RFB 地牢/深度/守护者/生命周期（v46-v69） | 填充规则细节、与荒野联动 |
| v_info + vaults/rooms | 数据 | 65+ vault | ✅ RFB vaults（v47/v50/v64） | 符号表映射 |
| q_*.txt 任务 | 数据 | 85 个 | 中：task 系统（v36-v45） | 任务自带小地图布局、对话文案 |
| t_*.txt 城镇 + w_info 荒野 | 数据 | 10 城镇 + 荒野图 | ⚠️ RFB 世界模型原创、无荒野系统 | 大系统缺口，最后处理 |
| 装备/内在旗标 | 横切 | RES_/IGNORE_/SPEED/SLAY_/FREE_ACT/SEE_INVIS… | ❌ 引擎缺口 | **旗标系统**：词条 72 条 + 神器旗标主体 + 种族内在旗标共用同一语义空间 |
| 设备/消耗品效果 | 代码 | 行为缺口 231 + 激活 193 | ❌ 引擎缺口 | 设备效果系统 |

## 3. 关键依赖判断

1. **种族成为内容图枢纽**（P47 之后）：身体模板挂种族、内在旗标挂种族、怪物种族玩法挂种族。b_info 导入必须与种族导入同轮，否则模板无处落。
2. **职业不是身体的挂靠点**，其价值锚定在 m_info/玩家法术线——职业壳先于法术映射意义有限，排在种族之后。
3. **旗标系统是三处内容（词条/神器/种族）的共同解锁**。按"先导壳报缺口"的打法，先导种族让缺口报告把种族旗标计入覆盖数，再做旗标系统一次收割三处。
4. 玩家法术线的效果词汇经怪物法术线（P32-P43）已建成大半，映射成本主要在逐条核对，可按领域分批。
5. 设备效果系统独立于上述依赖链，可在任意间隙插入。

## 4. 优先级队列

| 梯队 | 迭代 | 内容 | 理由 |
| --- | --- | --- | --- |
| T1 | P48 ✅ | **b_info + 种族 + 性格导入**（已完成，见 [legacy-character-import-v1](legacy-character-import-v1.md)）：67/88 种族 + 20/21 性格 + 113 模板缺口普查；钩子缺口 calc_bonuses 76/birth 27 已量化 | P47 表面刚就绪；体量小；.local 包获得角色定义自洽性；缺口报告首次量化种族旗标覆盖数 |
| T2 | P49–P51 ✅ | **装备/内在旗标系统**：防御面见 [contract-v101](contract-v101-defensive-flags.md)，进攻面 slay/kill/五元素 brand 见 [contract-v102](contract-v102-offensive-flags.md)，动态实例与 passive 见 [contract-v103](contract-v103-dynamic-affixes.md) | 防御回灌 ego 105/160；进攻面 107/160；动态 roll、技能/能力旗标与首批原版配方回灌后 ego 128/160，剩余 32 个集中于反射/光环/诅咒/额外射击与高级品牌 |
| T3 | P52 ✅ | **职业壳 + m_info 施法档案导入**（见 [legacy-class-import-v1](legacy-class-import-v1.md)）：54 职业壳 + 54 skillSets；53 份 m_info 共 636 领域行/144 可读行/4608 逐法术参数；C caster_info 壳与领域可读性表；s_info 16640+156 条差异量化 | 已为玩家法术线铺好稳定挂靠点；中间档案不伪装成运行时能力 |
| T4 | P53+（Death 第一、二册 ✅） | **玩家领域法术逐册映射**（P53–P55 已完成 Death 前两册，见 [legacy-player-spell-import-v1](legacy-player-spell-import-v1.md)、[contract-v104](contract-v104-death-first-book.md) 与 [contract-v105](contract-v105-death-second-book.md)）∥ **设备/消耗品效果系统**（可并行插队） | `abilityOverrides`、等级缩放、职业 beam 档案、灭绝、品牌、吸血与尸体/复活已成为复用词汇。P56 候选为 Death 第三册逐槽盘点；设备纵切仍可按覆盖收益插队 |
| T5 | 后续 | **d_info 地牢 + v_info vault 导入** → q_* 任务 → 城镇/荒野 | 世界内容线；荒野/城镇需先补系统设计 |

## 5. 滚动修订

每轮导入后按缺口报告数字重排 T4+ 顺序。P53–P55 已完成 Death 前两册：12 个静态职业映射 16 个法术、192 行参数，Death 效果缺口 480→288；Rogue（敏捷）、Blood Mage（生命）和 Skillmaster（动态）保留未接。P55 分别建模了活体限定、bolt-or-beam、自身中心 AoE、灭绝、临时品牌、吸血和尸体/复活。P56 候选为 Death 第三册逐槽盘点；法术清尾（S_ 字形召唤 177/SHRIEK/TRAPS）与设备效果系统继续可按真实覆盖收益插队。本文件随每次重排更新。
