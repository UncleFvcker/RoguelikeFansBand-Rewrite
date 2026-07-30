# Contract v144：Blindness 药水

状态：已实现（P94）

Contract v144 接入原版药水 tval 75 / sval 7 与食物 tval 80 / sval 1 的 Blindness 行为。协议保持 `1.121`，demo 内容包为 `1.135.0`，save 容器保持 v1，state hash Schema 保持 `54`。active baseline 包含 448 条 exact fixtures、零 waiver，内置内容 hash 为 `9f28bf79c8fc72bbcf97beec23da1c1fa0a10045b5c363defcb59e9a29457ed5`。

## 1. 效果与事务

固定原版 `191f48c3` 的 `src/devices.c` 在合法使用后先执行 Blindness 抗性判定；药水未抵抗时施加 `1d100+99`，食物未抵抗时施加 `1d25+24`。当前核心把 `RES_BLIND` 表达为既有 `rfb.status.blindness` 状态免疫，因此每次合法使用固定先抽一次 `bounded(55)`：普通阈值为 0，不会抵抗；拥有该免疫时阈值为 55，必定抵抗。抵抗成功不再抽持续时间 RNG。

当前纵切增加 self-only 的窄静态消耗品效果 `apply-blindness { durationDice, durationSides, durationBonus }`。合法使用先写入 Tried 并消费物品，未抵抗时才掷持续时间，并以 Extend 应用既有 Blindness。只有首次进入 Blindness 才把来源升级为 Aware；已有 Blindness 即使延长也保持 Tried-only。所有合法分支支付既有 100 energy；错误目标继续由既有物品计划器在消费、时间和 RNG 前拒绝。

## 2. 协议、界面与内容边界

Blindness 状态、FOV、状态免疫、存档与 state hash 均复用既有权威结构，本轮没有新增协议 DTO、命令、存档字段或 state-hash Schema。内部 `ItemBlindnessResolved` 只投影普通 `GameEventDto`：首次施加、已有状态延长和抵抗分别为 `item.use-blindness-applied`、`item.use-blindness-no-new-effect` 与 `item.use-blindness-resisted`；Web 只增加对应展示分支和中英消息。

效果只能用于静态 consumable，不能用于动态 activation、充能物品或设备检定。demo 使用原创 Veil Draught。本轮不增加通用物品状态 DSL、通用抗性 helper、debug 开关或 `AbilityEffectDefinition` 入口；食物只接入主动 Blindness 效果，营养事务仍由独立的 `food-nutrition` 缺口追踪。

## 3. 导入与契约

legacy importer 将 tval 75 / sval 7 映射为 `1d100+99`，将 tval 80 / sval 1 映射为 `1d25+24`。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 严格导入成功，解析诊断为零，`consumable-effect` 从 68 降至 66，`food-nutrition` 保持 28，`scroll-effect` 保持 15；真实包保持 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races，源码校验、编译与二进制回读 hash 均为 `47f5a78d899de6cee7339c97832e8cd2aef84049d1394ce42bf6dbcc644e8c39`。

fixture 448 让 Vanguard 使用一瓶 Veil Draught，固定一次抗性 RNG、一次持续时间 RNG、事件持续时间 116、推进 10 ticks 后 Blindness 剩余 106、来源变为 Aware，以及物品消费。一个核心测试以两种案例覆盖已有 Blindness 时继续抽持续时间并延长但保持 Tried-only，以及拥有 Blindness 免疫时只抽抗性 RNG、不产生状态；没有增加 save round-trip、Schema 负例、Web 单测或 Tauri E2E 断言。

旧正式 demo hash `8b3bdb097563d99b6433a5746c07d395b406d5c8d86616540e0126cd6af72404` 已加入兼容列表。内容 hash 属于 state hash 输入，因此既有 446 条受影响 fixture 只替换 `stateHash` / `saveRoundTripStateHash` 字段，其他 assertions 保持不变。

## 4. 明确遗留

- 食物营养、饥饿和进食事务仍由 28 条 `food-nutrition` 缺口独立追踪；
- 其余 66 个 `consumable-effect` 与 15 个 `scroll-effect` 继续按独立事务分组；
- 本轮只复用现有 Blindness 免疫语义，不建立可配置的物品状态抗性表或通用状态效果 DSL；
- 动态 activation 与玩家能力中的致盲效果等待各自真实导入需求，不提前放宽静态 consumable 限制。
