# 旧版玩家领域法术导入 v1（Death 第一、二册）

状态：已实现（P53–P55，导入优先级规划 T4 前两册）。P53 建立职业参数覆盖与首批纵切；P54 以 contract-v104 完成第一册；P55 以 contract-v105 完成第二册。当前协议 1.105、demo 内容包 1.96.0、demo content hash `26fdeb15063fa5ccc5a672cd8d2376f7ea66e7dc487fef6f1a4d5640a1050cf9`、state hash Schema v44 和 343 条 active exact fixtures。

## 1. 同一本法书的职业参数

原版 `m_info` 对同一法术按职业分别声明 `level/mana/fail/exp`，但物理法书与法术身份仍是共享的。复制成每职业一套 ability/book 会破坏物品语义，因此 `CastingProfileDefinition` 新增 `abilityOverrides`，只覆盖：

- `minimumLevel`
- `resourceCost`
- `baseFailurePercent`
- `levelScaling`（仅在职业公式确有差异时替换）

内容编译器稳定排序并验证 abilityId 唯一、数值范围以及 ability 必须属于该 profile 引用的法书。旧内容缺字段默认为空且序列化省略，所以 demo hash 不变。Core 的能力投影、学习门槛、失败率、耗魔、施放、读档和权威状态校验统一生成 effective ability；覆盖值不单独入档，仍由“内容包 + 职业身份”确定。

## 2. Death 第一册映射

固定来源仍为 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 的 Git 对象。`k_info` 的 `tval=100/sval=0`（`[Stench of Death]`）改为不可堆叠实体书，并绑定 `rfb-legacy.ability-book.death-stench-of-death`。P54 增加七类玩家等级标量、actor Detect、status power、sleep、状态授予临时抗性和 Control 后，八个槽位全部映射：

| 原版槽位 | Ability | 当前行为 | 保留缺口 |
| --- | --- | --- | --- |
| 0 Detect Unlife | `death-detect-unlife` | 半径 8 检测 `nonliving` actor | 瞬时结果，不写怪物回忆 |
| 1 Malediction | `death-malediction` | hell-fire，`3+floor((level-1)/5)`d4 | `to_d_spell` 与 1/5 随机 rider |
| 2 Detect Evil | `death-detect-evil` | 半径 8 检测 `evil` actor | 瞬时结果，不写怪物回忆 |
| 3 Stinking Cloud | `death-stinking-cloud` | 半径 2 poison，总伤害 `10+floor(level/2)` | `to_d_spell` |
| 4 Black Sleep | `death-black-sleep` | power `level*2` 的 sleep | status power 使用中性双有界骰 |
| 5 Necromantic Resistance | `death-necromantic-resistance` | cold/poison resistant 状态 | 固定 300 ticks，未复刻 `20+1d20` |
| 6 Horrify | `death-horrify` | fear power `level*2` + stun `5+floor(level/5)` | spell power |
| 7 Enslave Undead | `death-enslave-undead` | 控制 `undead`，保存 controller identity | 无宠物维护/解除控制系统 |

## 3. Death 第二册映射

`k_info` 的 `tval=100/sval=1`（`[Sepulchral Ways]`）绑定第二本实体法书。P55 新增活体筛选、bolt-or-beam、职业 beam 档案、灭绝、临时品牌、吸血、尸体和复活词汇，八个槽位全部映射：

| 原版槽位 | Ability | 当前行为 | 保留差异 |
| --- | --- | --- | --- |
| 8 Entropy Orb | `death-entropy-orb` | 活体限定，`3d6+floor(3*level/2)`，半径封顶 3 | 部分职业 override 为 `5/4*level` |
| 9 Nether Bolt | `death-nether-bolt` | `(8+floor((level-5)/4))d8`；按职业概率 bolt/beam | `to_d_spell` |
| 10 Cloud Kill | `death-cloud-kill` | 自身中心，`60+2*level`，半径 `2+floor(level/10)` | 未建立持续云地形 |
| 11 Genocide One | `death-genocide-one` | 单目标直接移除，唯一怪抵抗，按目标等级疲劳 | 不进入普通死亡事务 |
| 12 Poison Branding | `death-poison-branding` | 500 ticks 临时 poison 品牌 | 按路线要求不永久修改武器 |
| 13 Vampiric Drain | `death-vampiric-drain` | 活体限定，`1d(2*level)+2*level`，按实际伤害治疗 | `spell_power` |
| 14 Animate Dead | `death-animate-dead` | 消耗尸体并生成永久玩家控制亡灵 | 无宠物维护/忠诚系统 |
| 15 Genocide | `death-genocide` | 同 glyph 灭绝，唯一怪抵抗，每候选 `1d4` 疲劳 | 不维护跨层种群记忆 |

## 4. 职业接入与容量近似

Death 可读的 15 个职业中，12 个静态档案生成运行时 casting profile：Mage、Priest、Ranger、Paladin、Warrior-Mage、High-Mage、Sorcerer、Monk、ForceTrainer、Red-Mage、Yellow-Mage、Gray-Mage。

刻意排除 Rogue（C 侧为 Dexterity 施法，当前枚举不支持）、Blood Mage（`CASTER_USE_HP`）和 Skillmaster（动态 caster_info）。Mana 容量暂按 `level + casting attribute index`，学习容量按 `min(32, 4 + level)`；负重、原版容量与学习公式都进入 `playerSpellBehaviorGaps`，不宣称精确复刻。

## 5. 固定基线结果

- 16 abilities、2 ability books、1 个 Mana resource、两本实体书绑定；
- 12 个运行时 casting profile、192 条职业参数覆盖与 192 条映射行；
- Death 效果缺口 480→288；
- 等级效果缩放与怪物状态 power 缺口清零；Malediction rider、随机抗性持续、Mana 容量、学习公式、caster encumbrance 各保留 12 条；
- 普通活体 legacy actor 获得 `living` 和通用尸体引用，`UNIQUE` 映射为 `unique`，Animate Dead 使用稳定 skeleton actor；
- 源文件数预算保持 32768，单文件 1 MiB、源包总计 16 MiB、编译产物 32 MiB 的独立守卫保持不变；
- 本地包 content hash：`203378a37e05b2fa855037f86eb039d2ee68094ba04f982641e1bbc91001aa17`。

## 6. 下一步

P56 可继续逐槽盘点 Death 第三册，并按真实效果聚类新增系统；设备/消耗品效果系统仍可按覆盖收益插队。Dexterity/HP/dynamic caster 仍应在相应资源与施法属性系统完成后接入，不能把这些职业强行改成 Mana 档案。
