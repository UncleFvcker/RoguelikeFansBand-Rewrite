# Contract v137：Resistance 药水

状态：已实现（P86）

Contract v137 接入原版药水 tval 75 / sval 60 的 Resistance。协议保持 `1.121`，demo 内容包为 `1.128.0`，save 容器保持 v1，state hash Schema 保持 `54`。active baseline 包含 441 条 exact fixtures、零 waiver，内置内容 hash 为 `b33b104f3d7fd2153a66597b4f7685647020f3c9e3352366840dac326e650a57`。

## 1. 效果与事务

内容层增加 self-only 的窄静态消耗品效果 `apply-basic-resistance { durationDice, durationSides, durationBonus }`。demo 的原创公开物品 Prismatic Elixir 与 legacy importer 都固定为 `1d20 + 20`；效果不能用于动态 activation、充能物品或设备检定。本轮不把 Thermal 改写为参数化 resistance bundle，不增加通用物品状态 DSL，也不扩展 `AbilityEffectDefinition`。

合法使用先写入 Tried 并消费药水，再只抽取一次持续时间。核心以 KeepStrongest 应用单一 `rfb.status.basic-resistance`，该状态同时授予 Acid、Electricity、Fire、Cold 与 Poison 的 Resistant 档。重复使用只在新持续时间更长时替换剩余时间，不累加，也不缩短；P85 的 Thermal 状态保持独立，最终有效抗性继续由既有来源合并路径计算。

原版该药水在合法使用后直接设置 `device_noticed = TRUE`，因此本契约无条件把来源药水升级为 Aware，即使重复使用未改变状态。错误目标继续由既有物品计划器在消费、时间和 RNG 前拒绝，合法动作支付既有 100 energy。

## 2. 协议、存档与界面

状态实例的抗性授予、物品知识和内容 hash 已在既有权威状态中，本轮没有新增协议 DTO、命令、存档字段或 state-hash Schema。内部 `ItemBasicResistanceApplied` 事件只投影一个普通 `GameEventDto`：`item.use-basic-resistance` / `item-use-basic-resistance`，携带来源、显示键与本次掷出的持续时间；因为合法使用始终识别，不增加 no-effect 分支或 `noticed` 字段。

## 3. 导入与契约

legacy importer 将 tval 75 / sval 60 映射为 `apply-basic-resistance`。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 严格导入成功，`consumable-effect` 从 77 降至 76，`scroll-effect` 保持 15；真实包保持 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races，源码校验、编译与二进制回读 hash 均为 `430e28aaf60a043a344c02dc8d41185aaa0e33e0393da034fe0af9bbf0d785a2`。

fixture 441 使用同一堆叠中的两瓶药水：第一次掷出 40 ticks，支付首回合后剩 30；第二次掷出 29，KeepStrongest 保留 30，支付第二回合后最终剩 20。该 fixture 同时固定五抗、两次消费、Aware、两次效果 RNG、事件顺序和存档回读；导入器既有表测试增加 sval 60。没有增加核心、save、错误目标、内容校验、协议或 Tauri E2E 专项测试。

内容 hash 属于 state hash 输入，因此既有 440 条 fixture 只替换 `stateHash` / `saveRoundTripStateHash` 字段；去除这些字段后的完整 assertions 投影必须保持不变。

## 4. 明确遗留

- 本轮用单一 Basic Resistance 状态表达五种同步基础抗性，不提前建立五个独立计时器或跨来源共享 oppose 计时器；
- 原版 `_potion_power`、音乐/Kata/Wild Resistance 等尚未导入的临时抗性来源不在本轮；
- Thermal 保持独立双抗状态，不改造成通用 resistance 列表；
- Speed 的 Mauler 特例、Cure Poison 的部分毒素削减与临时毒抗、Blindness 的抗性保存继续独立处理；
- 剩余 76 个 `consumable-effect` 继续按独立事务分组；剩余 15 个 `scroll-effect` 不与本轮合并。
