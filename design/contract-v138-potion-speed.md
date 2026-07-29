# Contract v138：Speed 药水

状态：已实现（P87）

Contract v138 接入原版药水 tval 75 / sval 29 的 Speed。协议保持 `1.121`，demo 内容包为 `1.129.0`，save 容器保持 v1，state hash Schema 保持 `54`。active baseline 包含 442 条 exact fixtures、零 waiver，内置内容 hash 为 `1b3c059fedbc14ad79a9549a8b0bd4496f22785355e2bb4ef1ce3a0f763c7e35`。

## 1. 效果与事务

内容层增加 self-only 的窄静态消耗品效果 `apply-speed { durationDice, durationSides, durationBonus }`。demo 的原创公开物品 Swiftstep Tonic 与 legacy importer 都固定初次持续时间为 `1d25 + 15`；效果不能用于动态 activation、充能物品或设备检定。本轮不增加通用物品状态 DSL、重复时长内容字段或职业覆盖表。

合法使用先写入 Tried 并消费药水。玩家没有 `rfb.status.haste` 时只抽一次持续时间，以 Extend 应用既有 Haste，并将来源药水升级为 Aware；已有 Haste 时不抽 RNG，只把剩余时间增加 5 ticks，也不产生新的识别依据。两条分支都支付既有 100 energy，速度派生、行动调度、状态递减和存档投影完全复用现有权威路径。错误目标继续由既有物品计划器在消费、时间和 RNG 前拒绝。

原版 Mauler 在重复使用时增加 10 ticks，本轮只接入普通职业的 `+5` 基础行为。固定原版导入包当前没有可选 build，因此不为未进入可玩纵切的职业特例增加核心硬编码或公共内容表；该差异在本契约中保持显式，不作为已完成的职业规则。

## 2. 协议、存档与界面

Haste 已存在于权威状态、派生速度、存档、快照和 state hash，本轮没有新增协议 DTO、命令、存档字段或 state-hash Schema。内部 `ItemSpeedResolved` 只投影一个普通 `GameEventDto`：`item.use-speed` / `item-use-speed`，携带来源、显示键与本次增加的持续时间；首次使用为骰值，重复使用为 5。Web 只增加一个展示分支和一组中英本地化消息。

## 3. 导入与契约

legacy importer 将 tval 75 / sval 29 映射为 `apply-speed`。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 严格导入成功，`consumable-effect` 从 76 降至 75，`scroll-effect` 保持 15；真实包保持 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races，源码校验、编译与二进制回读 hash 均为 `4b35c7d998cbb576b952384ce2c587a261a4dd28628dda451f04466e116a983f`。

fixture 442 使用同一堆叠中的两瓶药水：第一次掷出 40 ticks，第一次行动后剩 35；第二次零 RNG、增加 5，再支付行动时间后最终仍为 35。该 fixture 同时固定 Haste、速度 120、两次消费、Aware、单次效果 RNG、两个同类事件和存档回读；导入器既有表测试增加 sval 29。没有增加核心、save、错误目标、内容校验、协议、Web 或 Tauri E2E 专项测试。

内容 hash 属于 state hash 输入，因此既有 441 条 fixture 只替换 `stateHash` / `saveRoundTripStateHash` 字段；去除这些字段后的完整 assertions 投影必须保持不变。

## 4. 明确遗留

- 原版 `_potion_power` 与 Mauler 重复使用 `+10` 等待对应能力纵切，不建立 Speed Potion 专用职业覆盖表；
- Heroism、Berserk Strength、Sight 和其他带职业特例的药水继续独立处理；
- 本轮不增加新的 Haste 状态、速度层、调度分支、免疫或通用物品状态 DSL；
- 剩余 75 个 `consumable-effect` 继续按独立事务分组；剩余 15 个 `scroll-effect` 不与本轮合并。
