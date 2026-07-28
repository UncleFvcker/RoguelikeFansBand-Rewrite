# 旧版玩家领域法术导入 v1（Death 第一至四册）

状态：已实现（P53–P57，导入优先级规划 T4 的 Death 四册）。P53 建立职业参数覆盖与首批纵切；P54–P57 以 contract-v104–v107 依次完成四册。当前项目已推进到协议 1.108、demo 内容包 1.99.0、demo content hash `4105aec18bdc40aced03bb503ec31e30385248545266d116b1d0088a374c04c8`、state hash Schema v47 和 368 条 active exact fixtures；P58 不改变 Death 法术语义。

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

## 4. Death 第三册映射

`k_info` 的 `tval=100/sval=2`（`[Black Channels]`）绑定第三本实体法书。P56 新增随机状态时长、状态派生加值/免疫、RandomChoice/NoOp、敌对固定召唤、永久武器 affix、Vampiric passive、重复 Drain Life、全可见目标伤害和 prorated 曲线，八个槽位全部映射：

| 原版槽位 | Ability | 当前行为 | 保留差异 |
| --- | --- | --- | --- |
| 16 Berserk | `death-berserk` | 随机时长、治疗、近战/技能加值、HP/防御修正和恐惧免疫 | 使用现有派生技能集合 |
| 17 Invoke Spirits | `death-invoke-spirits` | `1d100+floor(level/5)` 的 23 个阈值分支；19 个真实效果 | polymorph/line light/earthquake/destroy area 为明确 `NoOp` |
| 18 Dark Bolt | `death-dark-bolt` | dark bolt/beam，伤害骰按等级增长 | `to_d_spell` |
| 19 Battle Frenzy | `death-battle-frenzy` | Hero/Blessed/Haste 各自独立掷持续时间 | 使用现有派生属性词汇 |
| 20 Vampiric Branding | `death-vampiric-branding` | 当前武器永久添加 `vampiric` affix | 无附魔冲突/容量/费用系统 |
| 21 Vampirism True | `death-vampirism-true` | 三次 100 点 Drain Life，每击重新追踪并按实际伤害治疗 | `spell_power` |
| 22 Nether Wave | `death-nether-wave` | 全部可见活体共享 `1d(3*level)` nether 伤害掷骰 | 可见性使用当前 FOV |
| 23 Darkness Storm | `death-darkness-storm` | 半径 4 dark ball，使用原版形状的 prorated 等级曲线 | `to_d_spell` |

## 5. Death 第四册映射

`k_info` 的 `tval=100/sval=3`（`[Necronomicon]`）绑定第四本实体法书。P57 新增物品目标/鉴定、Death Ray、升级类别与敌友群体召唤、临时 Race、历史最高经验/生命力、邻域灭绝、穿墙和入伤比例，八个槽位全部映射：

| 原版槽位 | Ability | 当前行为 | 保留差异 |
| --- | --- | --- | --- |
| 24 Death Ray | `death-death-ray` | 仅活体；普通目标等级对抗，unique 先过 1/666 门 | 复用统一死亡事务，不建立独立即死抗性表 |
| 25 Raise Dead | `death-raise-dead` | 48 级切换 high-undead；敌友、群体与敌对 unique 按确定性概率结算 | 复用现有召唤物 AI，无宠物维护/忠诚 |
| 26 Esoteria | `death-esoteria` | 携带物品普通鉴定或 power 检定完整鉴定 | 无批量/自动选择 UI |
| 27 Vampiric Transformation | `death-vampiric-transformation` | 临时投影 Vampire Lord Race 的属性、技能与抗性 | 不改变身体槽或持久构筑 Race |
| 28 Restore Life | `death-restore-life` | 经验恢复到历史最高值，生命力恢复为 1000 | 生命力尚无其他消费/损伤入口 |
| 29 Mass Genocide | `death-mass-genocide` | 半径 20 nearby Genocide，unique 抵抗 | 不维护跨层种群记忆 |
| 30 Hellfire | `death-hellfire` | prorated nether ball，伤害与半径随等级增长 | 复用现有范围伤害与抗性 |
| 31 Wraithform | `death-wraithform` | 随机时长穿墙并承受 50% 入伤 | 到期不把墙内玩家传送或改图 |

## 6. 职业接入与容量近似

Death 可读的 15 个职业中，12 个静态档案生成运行时 casting profile：Mage、Priest、Ranger、Paladin、Warrior-Mage、High-Mage、Sorcerer、Monk、ForceTrainer、Red-Mage、Yellow-Mage、Gray-Mage。

刻意排除 Rogue（C 侧为 Dexterity 施法，当前枚举不支持）、Blood Mage（`CASTER_USE_HP`）和 Skillmaster（动态 caster_info）。Mana 容量暂按 `level + casting attribute index`，学习容量按 `min(32, 4 + level)`；负重、原版容量与学习公式都进入 `playerSpellBehaviorGaps`，不宣称精确复刻。

## 7. 固定基线结果

- 32 abilities、4 ability books、1 个 Mana resource、四本实体书绑定；
- 12 个运行时 casting profile、384 条职业参数覆盖与 384 条映射行；
- Death 效果缺口 480→96；
- 等级效果缩放与怪物状态 power 缺口清零；Malediction rider、随机抗性持续、Mana 容量、学习公式、caster encumbrance 各保留 12 条；
- Invoke Spirits 的 actor polymorph、line light、earthquake、destroy area 各保留 12 条行为缺口；
- 普通活体 legacy actor 获得 `living` 和通用尸体引用，`UNIQUE` 映射为 `unique`，Animate Dead 使用稳定 skeleton actor；
- 源文件数预算保持 32768，单文件 1 MiB、源包总计 16 MiB、编译产物 32 MiB 的独立守卫保持不变；
- P58 加入六种治疗药水后，本地包 content hash：`ed9534de7976be4668a8238deae3d207794d862e7a4ab41e888fde8c7e7b479c`。

## 8. 下一步

P58 已比较全领域与物品缺口并转入充能/治疗消耗品纵切；P59 已完成动态设备效果身份、容量和首批 staff/wand/rod；P60 已完成 rod/wand/staff 差异化自然恢复和资源/设备来源主动充能。P61 按实际覆盖收益继续选择设备、artifact/ego activation 或消耗品效果；Dexterity/HP/dynamic caster 仍应在相应资源与施法属性系统完成后接入，不能把这些职业强行改成 Mana 档案；Invoke Spirits 四项 `NoOp` 随对应通用系统逐项清零。
