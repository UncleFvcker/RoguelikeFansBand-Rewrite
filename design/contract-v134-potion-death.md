# Contract v134：Death 药水

状态：已实现（P83）

Contract v134 接入原版药水 tval 75 / sval 23 的 Death。协议保持 `1.121`，demo 内容包为 `1.125.0`，save 容器保持 v1，state hash Schema 保持 `54`。active baseline 包含 437 条 exact fixtures、零 waiver，内置内容 hash 为 `1c6e2bf891c76796cca6eb53ea014caa03fb8bb1fa3a95b8df8fd81f942e8562`。

## 1. 效果与事务

内容层增加 self-only 的窄静态消耗品效果 `self-life-loss { amount }`，demo 的原创公开物品 Mortal Draught 与 legacy importer 都固定为 5000。效果不能用于动态 activation、充能物品或设备检定；本轮不增加伤害类型、伤害骰、穿透开关或通用伤害 DSL。

合法使用先写入 Tried 并消费药水，再直接从玩家当前 HP 扣除固定生命。该扣除复刻原版 `DAMAGE_LOSELIFE` 边界，不经过护甲、元素抗性或 `incomingDamagePercent`；不抽取效果 RNG。效果一经结算即 Aware。错误目标仍由既有物品计划器在消费、时间与 RNG 前拒绝。

若扣除后玩家死亡，调度器沿用既有死亡边界，不继续等待完整普通行动周期；本轮只由内部事件选择普通或致死文本，不增加新的死亡状态或第二套伤害事务。

## 2. 协议、存档与界面

生命值、死亡判定、物品知识和内容 hash 已在既有权威状态中，本轮没有新增协议 DTO、命令、存档字段或 state-hash Schema。内部 `ItemLifeLost` 投影为普通 `GameEventDto` 的 `item.use-life-loss` 或 `item.use-life-loss-death`；Web 只增加对应本地化展示分支。

## 3. 导入与契约

legacy importer 将 tval 75 / sval 23 映射为 `self-life-loss { amount: 5000 }`。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 严格导入成功，`consumable-effect` 从 80 降至 79，`scroll-effect` 保持 15；真实包保持 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races，源码校验、编译与二进制回读 hash 均为 `ab0e840f704f3c9a1e9de7ba5c6c2f0ab28ea6dc775a037a54104b1bb9970210`。

fixture 437 固定药水消费、Aware、5000 点生命损失、死亡、零效果 RNG 和致死事件，不重复保存回读断言。一个聚焦核心单测只覆盖 `incomingDamagePercent` 不削减 life loss 以及零效果 RNG；导入器既有表测试增加 sval 23。没有增加 Web、Tauri E2E、save、错误目标或动态 activation 专项测试。

## 4. 明确遗留

- 本轮只实现原版 Death 药水的固定生命损失，不提前实现 Ruination、Detonations、属性损失、伤害骰或其他药水副作用；
- `self-life-loss` 不是普通攻击伤害，不能由内容选择伤害类型、抗性、来源 actor 或绕过策略；
- 剩余 79 个 `consumable-effect` 继续按各自事务分组；剩余 15 个 `scroll-effect` 不与本轮合并。
