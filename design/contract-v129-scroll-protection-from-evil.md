# Contract v129：Protection from Evil 卷轴

日期：2026-07-29

Contract v129 接入原版卷轴 sval 37 的 Protection from Evil。协议保持 `1.119`，demo 内容包为 `1.120.0`，save 容器保持 v1，state hash Schema 保持 `53`。active baseline 包含 432 条 exact fixtures、零 waiver，内置内容 hash 为 `27ad6b88a3e4bdeb4f1464d2081f6f59e62cbbfbab14ed09e9b5bdfaf43ead24`。

## 1. 效果边界

内容层新增无参数、self-only 的 `protection-from-evil` 物品效果。合法阅读抽一次 `1d25`，以 `3 * player level + 1d25` ticks、Extend 方式施加 `rfb.status.protection-from-evil`。重复阅读继续消费并延长现有持续时间；成功施加即写入 Tried + Aware。

保护只介入怪物对玩家的近战 blow。每段攻击先走既有命中检定；命中后、伤害骰前，仅当怪物具有 `evil` tag 且玩家状态仍有效时按以下顺序结算：

1. 玩家 power 为 `player level + Wisdom save adjustment`，最低 1；Wisdom index 超出原版 0–37 表时钳制到最后一项；
2. 怪物 power 为 actor level，`unique` 增加 20%，最低 1；
3. 双方各抽 `1..power`，玩家点数不高于怪物时，怪物豁免且攻击继续；
4. 否则再抽 `one_in(3)`，命中该分支时攻击绕过保护，其余结果击退并跳过该 blow 的伤害骰与后续伤害结算。

非 evil、未命中和状态无效时不抽保护 RNG。状态不改变法术、远程或环境伤害，也不提前建立通用攻击拦截器。

## 2. 协议、存档与界面

状态复用既有 `StatusInstance`、存档 DTO、state hash 和 Web 状态列表，不新增玩家字段或协议类型，因此协议与 state hash Schema 均不升级。新增阅读与击退事件仍使用普通事件 envelope；Web 只增加事件格式化和中英文 Fluent 文案，没有新增 outcome DTO 或专门 E2E 场景。

## 3. 导入与契约

legacy importer 将 tval 70 / sval 37 映射为 `protection-from-evil`。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 严格导入成功，`scroll-effect` 从 19 降至 18；真实包保持 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races，源码校验、编译与二进制回读 hash 均为 `db78e5d8fe181d88943b024647afb94791c0e3f00adb25ab3271e18c67bde408`。

fixture 432 固定 50 级高 Wisdom 玩家阅读后的 158 tick 状态、一个行动周期后剩余 148 tick、evil 怪物击退、消费、知识和存档回读。一个窄核心组合单测覆盖两次阅读的 Extend 与 RNG 次数、非 evil 零 RNG，以及怪物豁免、`one_in(3)` 绕过和击退三条分支；导入器既有表测试增加 sval 37。

既有 431 条 fixture 只更新 `stateHash` 与 `saveRoundTripStateHash`。

## 4. 明确遗留

- 原版 `_scroll_power`、Devicemaster Scrolls 特例、怪物 lore 和非卷轴来源不在本轮；
- Protection from Evil 只拦截怪物对玩家的近战 blow，不覆盖法术、远程、环境伤害、反击或玩家阵营 actor；
- 状态使用当前统一 10-tick 行动推进，不另建原版独立回合计时器；
- 剩余 18 个 `scroll-effect` 继续按世界/地形、状态和物品/成长事务拆分。
