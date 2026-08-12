# Contract v273：Virtue 基础状态与死亡领域接线

状态：已实现。

## 权威状态

- 以 RFB `master:src/virtue.c` 为准，保存 18 种 Virtue 中每角色 8 个唯一槽位；数值范围
  为 `-125..125`。
- 当前正式职业按原版初始化：Warrior 为 Valour、Honour；Archer 为 Nature、
  Temperance；High-Mage 为 Enlightenment、Enchantment、Knowledge。Human 增加
  Individualism，Vampire 增加 Unlife；Death realm 增加 Unlife。
- 重复类型先清空，再按原版 `1d29` 权重逐槽补齐。增减值依次执行 50、80、100 三层
  `one_in_(2)` 软上限，最后在 125 处硬截断。
- 8 个槽位进入 save、玩家只读协议投影、Web 状态面板和状态哈希；旧开发存档不提供
  兼容默认值。

## 死亡领域

- Poison Branding 与 Vampiric Branding 成功后增加 Enchantment 2。
- Vampiric Drain 成功吸取后、True Vampirism 施法时分别减少 Sacrifice 与 Vitality 1。
- Raise Dead 成功召唤后增加 Unlife 1。
- Invoke Spirits 在外层随机值完成 `spell_power` 后，按 Chance 的重复 `1d400` 规则修正；
  低于 26 的结果增加 Chance 1，最低档结果另增加 Unlife 1。
- 未增加通用递归效果 DSL，也未接入本提交范围外的通用施法成功/失败 Virtue 钩子。

## 协议与基准

- Protocol 1.181、State Hash Schema v90，save 容器 v1，内容包与 content hash 不变。
- `RandomChoice.roll` 改为有符号整数，以保存负 Chance 可能产生的权威 Invoke Spirits
  随机值。
- Virtue 初始化发生在出生金钱、食物和火把之前，改变新角色 RNG；公共玩家投影和状态
  哈希也新增 Virtue，因此 21 条 active exact fixture 全量刷新，基线提升为
  `contract-v273`，零 waiver。
