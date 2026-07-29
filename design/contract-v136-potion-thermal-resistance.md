# Contract v136：Thermal 药水

状态：已实现（P85）

Contract v136 接入原版药水 tval 75 / sval 30 的 Thermal。协议保持 `1.121`，demo 内容包为 `1.127.0`，save 容器保持 v1，state hash Schema 保持 `54`。active baseline 包含 440 条 exact fixtures、零 waiver，内置内容 hash 为 `3098d9de2051029b4509acc3b8973cec0b76679dcacfa6ace1244864bc3f363d`。

## 1. 效果与事务

内容层增加 self-only 的窄静态消耗品效果 `apply-thermal-resistance { durationDice, durationSides, durationBonus }`。demo 的原创公开物品 Temperate Tonic 与 legacy importer 都固定为 `1d10 + 10`；效果不能用于动态 activation、充能物品或设备检定。本轮不增加通用物品状态 DSL，也不扩展 `AbilityEffectDefinition`。

合法使用先写入 Tried 并消费药水，再只抽取一次持续时间。核心以 Extend 应用单一 `rfb.status.thermal-resistance`，该状态同时授予 Fire 与 Cold 的 Resistant 档；既有有效抗性合并、状态递减和存档投影完全复用当前权威路径。

只有状态首次 Added 才将来源药水升级为 Aware。状态已经存在时仍使用同一次骰值延长持续时间，但保持 Tried-only；不额外执行状态免疫检定。错误目标继续由既有物品计划器在消费、时间和 RNG 前拒绝，合法动作支付既有 100 energy。

## 2. 协议、存档与界面

状态实例的抗性授予、物品知识和内容 hash 已在既有权威状态中，本轮没有新增协议 DTO、命令、存档字段或 state-hash Schema。内部 `ItemThermalResistanceResolved` 事件投影为普通 `GameEventDto`：首次应用使用 `item.use-thermal-resistance-applied`，已有状态的延长使用 `item.use-thermal-resistance-no-effect`；Web 只增加对应本地化展示分支。

## 3. 导入与契约

legacy importer 将 tval 75 / sval 30 映射为 `apply-thermal-resistance`。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 严格导入成功，`consumable-effect` 从 78 降至 77，`scroll-effect` 保持 15；真实包保持 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races，源码校验、编译与二进制回读 hash 均为 `9832b1a0d8c31d49407adb4f4a9dd9982292dab35b1d50c8b187670fa825a370`。

fixture 440 固定首次使用、一次效果 RNG、同一 bundle 中的 Fire/Cold Resistant、消费、Aware、事件与回档。一个聚焦核心单测覆盖既有状态的 Extend、Tried-only 和单次 RNG；导入器既有表测试增加 sval 30。没有增加 save、错误目标、动态 activation、伤害减免或 Tauri E2E 专项测试。

内容 hash 属于 state hash 输入，因此既有 439 条 fixture 只替换 1397 个 `stateHash` / `saveRoundTripStateHash` 字段；去除这些字段后的完整 assertions 投影保持不变。

## 4. 明确遗留

- 本轮用单一 Thermal 状态表达当前可观察的双抗持续时间，不提前建立独立 Fire/Cold 临时计时器或跨来源共享 oppose 计时器；
- 原版 `_potion_power`、音乐/Kata/Wild Resistance 等尚未导入的临时抗性来源不在本轮；
- Speed 的 Mauler 特例、Heroism/Berserk Strength 的 Alchemist 特例，以及 Sight 的临时感知能力继续独立处理；
- Resistance Potion 的五抗 bundle、Cure Poison 的部分毒素削减和临时毒抗不与本轮合并；
- 剩余 77 个 `consumable-effect` 继续按独立事务分组；剩余 15 个 `scroll-effect` 不与本轮合并。
