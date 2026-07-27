# 旧版玩家领域法术导入 v1（Death 第一册）

状态：已实现（P53/P54，导入优先级规划 T4 第一册）。P53 建立职业参数覆盖与首批纵切；P54 以 contract-v104 完成八个槽位。当前协议 1.104、demo 内容包 1.95.0、demo content hash `c0708c7866d93bdbb6601d349300cd5ef5e95a7ebd754de60d62e27d6c4071c6`、state hash Schema v43 和 334 条 active exact fixtures。

## 1. 同一本法书的职业参数

原版 `m_info` 对同一法术按职业分别声明 `level/mana/fail/exp`，但物理法书与法术身份仍是共享的。复制成每职业一套 ability/book 会破坏物品语义，因此 `CastingProfileDefinition` 新增 `abilityOverrides`，只覆盖：

- `minimumLevel`
- `resourceCost`
- `baseFailurePercent`

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

## 3. 职业接入与容量近似

Death 可读的 15 个职业中，12 个静态档案生成运行时 casting profile：Mage、Priest、Ranger、Paladin、Warrior-Mage、High-Mage、Sorcerer、Monk、ForceTrainer、Red-Mage、Yellow-Mage、Gray-Mage。

刻意排除 Rogue（C 侧为 Dexterity 施法，当前枚举不支持）、Blood Mage（`CASTER_USE_HP`）和 Skillmaster（动态 caster_info）。Mana 容量暂按 `level + casting attribute index`，学习容量按 `min(32, 4 + level)`；负重、原版容量与学习公式都进入 `playerSpellBehaviorGaps`，不宣称精确复刻。

## 4. 固定基线结果

- 8 abilities、1 ability book、1 个 Mana resource、1 本实体书绑定；
- 12 个运行时 casting profile、96 条职业参数覆盖与 96 条映射行；
- Death 效果缺口 480→384，`itemBehaviorGaps.book-system` 72→71；
- 等级效果缩放与怪物状态 power 缺口清零；Malediction rider、随机抗性持续、Mana 容量、学习公式、caster encumbrance 各保留 12 条；
- 本地包：180 terrain / 1332 actors / 128 affixes / 936 items / 2 resources / 1236 abilities / 1 ability book / 8 skills / 141 skill sets / 67 races / 54 classes / 20 personalities；
- 本地源目录共 4157 文件、约 4.55 MiB；源文件数预算从 4096 提升到 32768，单文件 1 MiB、源包总计 16 MiB、编译产物 32 MiB 的独立守卫保持不变；
- 本地包 content hash：`6106efe2d864592c4ffd6d774d8f12b1ffb6ac1775fd9a47e5afc5147bbac7dd`。

## 5. 下一步

P55 优先盘点并推进 Death 第二册。Entropy Orb、Nether Bolt 与 Cloud Kill 可复用 v104 缩放和现有 area/bolt/beam，但活体限定、bolt-or-beam、自身中心 AoE、单体/全类灭绝、临时武器毒品牌、吸血治疗与尸体复活分别需要明确系统边界。Dexterity/HP/dynamic caster 仍应在相应资源与施法属性系统完成后接入。
