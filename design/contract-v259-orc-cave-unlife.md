# Contract v259: Orc Cave UNLIFE

状态：active

## 边界

本基线把原版 `GF_UNLIFE` 建模为独立生命力事务，不以 HP 伤害、暗影伤害或经验
吸取近似。Vampire 与 Ghoulking 的 `UNLIFE` 近战进入正式内容；非生命玩家与非生命
actor 免疫。

玩家被命中时，装备提供的每个 Hold Life 来源依次按怪物有效等级与玩家等级、魅力
进行原版豁免。豁免失败后：

- `CharacterProgress.lifeForce` 按掷骰结果降低并限制在 `0..=1000`；
- 最大生命按原版生命力公式降低，当前生命同比例缩放；
- 攻击者的 `powerPerMille` 增加相同数值；
- 怪物强度统一影响派生攻击、防御、护甲、近战效果与怪物能力伤害；
- 该强度进入 save 与 State Hash，离层和读档后不丢失。

怪物阵营之间命中生命目标时只削弱目标的 `powerPerMille`，最低为 100，不强化
攻击者。两条路径都不产生普通伤害事件。

原版在生命力耗尽时会从五个亡灵种族中随机改变玩家种族。当前正式内容只具备其中
一个种族，不能忠实执行该选择，因此本基线只饱和到 0；完整耗尽转化等待五个种族
及其规则全部进入正式内容，不用固定变成吸血鬼或直接死亡近似。

## 兼容与版本

- Protocol：1.171
- State Hash Schema：v86
- Contract：contract-v259
- 内容包：1.250.0
- save 容器：v1；旧开发存档不作为兼容边界

内容包包含 49 种地形、620 种 actor、255 种物品、272 个能力、152 项变异和
13 张 loot table；严格同步 573 条记录，内容 hash 为
`fb79eaccc3e80ef67093237baf089ba53beb53701e9ae91723298be8b538a94a`。

## 验收

- UNLIFE 不减少 HP，也不产生普通伤害结果。
- 成功吸取生命力后，最大生命和怪物强度按同一数值变化。
- 怪物强化影响近战与施法，并在 save/state hash 回环后保留。
- 非生命目标免疫；Hold Life 可在不改变双方状态的情况下豁免。
- Vampire、Ghoulking 的源索引、中文名、近战与施法仍由 RFB `master` Git 对象验证。
- `audit-demo-monsters` 为 203 imported、193 selected、5 blocked、28 excluded、
  1 guardian。
