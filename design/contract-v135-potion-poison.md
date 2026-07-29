# Contract v135：Poison 药水

状态：已实现（P84）

Contract v135 接入原版药水 tval 75 / sval 6 的 Poison。协议保持 `1.121`，demo 内容包为 `1.126.0`，save 容器保持 v1，state hash Schema 保持 `54`。active baseline 包含 439 条 exact fixtures、零 waiver，内置内容 hash 为 `497fbc6b137e9bc2d8162ad52b0253f4d655a37c58abe391be6bcdd94ef94d9e`。

## 1. 效果与事务

内容层增加 self-only 的窄静态消耗品效果 `apply-poison { durationDice, durationSides, durationBonus }`。demo 的原创公开物品 Venom Draught 与 legacy importer 都固定为 `1d15 + 9`，等价于原版 `randint0(15) + 10`；效果不能用于动态 activation、充能物品或设备检定。本轮不增加通用物品状态 DSL 或保存检定框架。

合法使用先写入 Tried 并消费药水，再无条件抽取一次 `bounded(55)`。该结果与玩家有效 Poison 抗性档的既有低抗性数值比较：Vulnerable/Normal 的阈值为 0，Resistant 为 50，Strong 为 65，Immune 为 100。Strong/Immune 虽必定抵抗，仍保留这一次 RNG；本轮不把 Poison 复制进 status-immunity 系统。

抵抗成功时不抽持续时间、不施加状态且保持 Tried-only。抵抗失败时才抽取 `1d15 + 9`，通过既有 `apply_status` 以 Extend 合并 `rfb.status.poison`，并将来源药水升级为 Aware。错误目标仍由既有物品计划器在消费、时间与 RNG 前拒绝。

物品动作继续支付既有 100 energy。Poison 的持续时间递减和每 tick 伤害完全复用现有调度器；本轮不增加第二条毒伤、时间或死亡管线。

## 2. 协议、存档与界面

Poison 状态、抗性、物品知识和内容 hash 已在既有权威状态中，本轮没有新增协议 DTO、命令、存档字段或 state-hash Schema。内部单一 `ItemPoisonResolved` 事件投影为普通 `GameEventDto`：应用成功使用 `item.use-poison-applied`，抵抗成功使用 `item.use-poison-resisted`；Web 只增加对应本地化展示分支。

## 3. 导入与契约

legacy importer 将 tval 75 / sval 6 映射为 `apply-poison`。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 严格导入成功，`consumable-effect` 从 79 降至 78，`scroll-effect` 保持 15；真实包保持 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races，源码校验、编译与二进制回读 hash 均为 `54244a2fd227878c7017bc8dfe2bd125c48f65cb093a198547bdcd891f1aef3c`。

fixture 438 固定 Normal 抗性失败、15 ticks 初始 Poison、药水消费、Aware、两次效果 RNG，以及普通行动等待中的 10 次既有 Poison tick；fixture 439 固定 Resistant 抗性成功、药水消费、Tried-only、无状态和一次效果 RNG。导入器既有表测试增加 sval 6；没有增加核心、save、错误目标、动态 activation 或 Tauri E2E 专项测试。

内容 hash 属于 state hash 输入，因此既有 437 条 fixture 只刷新 `stateHash` 与 `saveRoundTripStateHash`；去除这两个字段后的完整 assertions 投影保持不变。

## 4. 明确遗留

- 本轮只实现 Poison 药水的低抗性检定与 Poison 状态，不提前接入 Blindness、Confusion、Sleep、Speed、Ruination 或 Detonations；
- 原版抵抗成功时的逐装备 `equip_learn_resist` 等待具有单项属性粒度的物品知识模型；当前不会把整件装备错误标记为 identified；
- Poison 防护只读取现有 `DamageType::Poison` 抗性，不新增状态免疫字段、第二份抗性映射或通用保存检定；
- 剩余 78 个 `consumable-effect` 继续按各自事务分组；剩余 15 个 `scroll-effect` 不与本轮合并。
