# Contract v105：Death 第二册与尸体/灭绝系统

## 范围

Contract v105 完成 Death 第二册 `[Sepulchral Ways]` 的八个槽位，并把所需规则拆成可复用的内容词汇，而不是把特殊法术近似成普通伤害。协议为 `1.105`，demo 内容包为 `1.96.0`，state hash Schema 为 `44`，active baseline 包含 343 条 exact fixtures、零 waiver。内置内容 hash 为 `26fdeb15063fa5ccc5a672cd8d2376f7ea66e7dc487fef6f1a4d5640a1050cf9`。

## 通用内容与协议表面

- `AreaDamage.targetCategory` 可把范围伤害限定为 `living` 等 actor 标签；不符合类别的目标不进入伤害结算。
- `BoltOrBeamDamage` 只进行一次确定性 beam 检定，bolt 命中首个合法目标，beam 沿完整投射线结算。
- `CastingProfileDefinition` 新增等级乘数、除数和常数三项 beam 几率参数；Mage 为 `level%`，High-Mage/Sorcerer 为 `level+10%`，其他已接入 Death 的静态职业按原版档案使用 `level/2%` 等公式。
- `DrainLife` 只作用于指定生命类别，并按目标实际损失生命治疗施法者，过量伤害不会产生额外治疗。
- `Genocide` 支持 `single` 与 `glyph`；强度沿用玩家等级缩放，唯一怪物稳定抵抗。
- `AnimateDead` 引用尸体 item kind 与生成 actor kind；`ActorDefinition.corpseItemKindId` 声明普通死亡留下的尸体。
- `ApplyStatus.grantedBrands` 让临时状态参与既有近战品牌倍率，并随状态过期、存档和 state hash 保持一致。
- `AbilityLevelScalingDefinition.maximum` 为缩放值提供内容上限；职业逐法术 override 可替换 `levelScaling`。缩放字段扩展到骰面、beam 几率与灭绝强度。

## Death 第二册

| 槽位 | Ability | v105 行为 |
| --- | --- | --- |
| 8 | Entropy Orb | 仅伤害 `living`；`3d6 + floor(3*level/2)`，半径 30 级前 2、之后封顶 3 |
| 9 | Nether Bolt | `(8 + floor(max(level-5, 0)/4))d8` nether；按职业施法档案选择 bolt 或 beam |
| 10 | Cloud Kill | 以施法者为中心，伤害 `60 + 2*level`，半径 `2 + floor(level/10)` |
| 11 | Genocide One | 单目标灭绝；唯一怪抵抗，疲劳骰面为 `(目标等级+1)/2` |
| 12 | Poison Branding | 500 ticks 临时 poison 品牌，不永久改写武器实例 |
| 13 | Vampiric Drain | 仅对活体生效；`1d(2*level) + 2*level`，按实际伤害治疗 |
| 14 | Animate Dead | 消耗半径内尸体，生成永久 `controllerId=player` 的受控亡灵 |
| 15 | Genocide | 按目标 glyph 灭绝，power 为 `3*level`；每个候选均造成 `1d4` 疲劳，包括抵抗者 |

Genocide 直接移除目标，不发布普通死亡事件，不授予经验、任务击杀或掉落，也不会生成尸体。普通伤害死亡继续走完整死亡事务；带 `corpseItemKindId` 的 actor 会生成不可堆叠尸体物品。Animate Dead 消耗尸体后创建正常 actor 实例，但不附加临时 summon 生命周期，因此受控身份和实体会随楼层状态与存档保持。

## Legacy 导入结果

固定 legacy commit 的真实导入现在生成两本 Death 物理法书、16 个玩家 abilities、12 个运行时 casting profiles 和 192 条逐职业参数覆盖/映射行。第二册允许职业 override 替换 Entropy Orb 的等级缩放，并从职业档案导入 Nether Bolt 的 beam 几率。普通活体 legacy actor 获得 `living` 与通用尸体引用，`UNIQUE` 映射为 `unique`；Animate Dead 使用 `rfb-legacy.actor.skeleton-human`。

Death 效果缺口由 384 降至 288。本地 legacy 内容包严格编译通过，content hash 为 `203378a37e05b2fa855037f86eb039d2ee68094ba04f982641e1bbc91001aa17`。

## Fixtures 与兼容性

- 335：30 级 Mage 的第二册投影、缩放值与 30% beam 几率；
- 336：Entropy Orb 只伤害活体，跳过同范围非生命目标；
- 337/338：相同队列分别锁住 Nether Bolt 的 beam 与 bolt 路径；
- 339：Cloud Kill 以玩家位置为中心并命中范围内多个目标；
- 340：临时 poison 品牌进入近战、状态计时和 save round-trip；
- 341：Vampiric Drain 只按目标实际损失的 7 HP 治疗；
- 342：glyph Genocide 移除普通同字形目标、保留唯一怪并结算全部疲劳；
- 343：普通死亡生成尸体，Animate Dead 消耗尸体并保存玩家控制身份。

旧正式 demo hash `c0708c7866d93bdbb6601d349300cd5ef5e95a7ebd754de60d62e27d6c4071c6` 保留在兼容列表。新增状态品牌、尸体 item 和受控亡灵都进入 save/state hash；缺失新字段的旧档按空状态迁移，不补抽 RNG。

## 后续候选

P56 可继续盘点 Death 第三册，优先按实际槽位聚类需要的新系统；设备与消耗品效果系统仍是并列高收益候选。Rogue 的 Dexterity 施法、Blood Mage 的 HP 施法和 Skillmaster 的动态档案继续等待对应通用资源/属性表面，不在 P55 中以 Mana 近似。
