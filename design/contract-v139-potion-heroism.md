# Contract v139：Heroism 药水

状态：已实现（P88）

Contract v139 接入原版药水 tval 75 / sval 32 的 Heroism。协议保持 `1.121`，demo 内容包为 `1.130.0`，save 容器保持 v1，state hash Schema 保持 `54`。active baseline 包含 443 条 exact fixtures、零 waiver，内置内容 hash 为 `99c41b9668586d97987cc18a459632c8f444d9c8dffbf1e6e024f2ce35a11091`。

## 1. 效果与事务

内容层增加 self-only 的窄静态消耗品效果 `apply-heroism { durationDice, durationSides, durationBonus }`。demo 的原创公开物品 Valor Tonic 与 legacy importer 都固定持续时间为 `1d25 + 25`；效果不能用于动态 activation、充能物品或设备检定。本轮不增加通用物品状态 DSL、状态抗性系统或职业覆盖表。

合法使用先写入 Tried 并消费药水，然后抽一次持续时间，以 Extend 应用 `rfb.status.hero`。该状态复用既有派生路径，授予 max HP +10、melee skill +12、ranged skill +12 与 Fear 免疫。首次新增状态才将来源药水升级为 Aware；已有 Hero 时仍抽取并延长持续时间，但不产生新的识别依据。两条分支都支付既有 100 energy；错误目标继续由既有物品计划器在消费、时间和 RNG 前拒绝。

原版 `_potion_power` 与 Alchemist 特例不属于当前可玩纵切，本轮不为它们增加核心硬编码或公共内容参数。Heroism 也不引入对 Blindness 等其他药水所需的概率状态抗性语义。

## 2. 协议、存档与界面

Hero 状态、派生加值、状态免疫、存档和 state hash 均复用既有权威结构，本轮没有新增协议 DTO、命令、存档字段或 state-hash Schema。内部 `ItemHeroismResolved` 只投影普通 `GameEventDto`：首次新增为 `item.use-heroism-applied` / `item-use-heroism-applied`，重复延长为 `item.use-heroism-no-new-effect` / `item-use-heroism-no-new-effect`；两者都携带来源、显示键与本次骰出的持续时间。Web 只增加对应展示分支和中英消息。

## 3. 导入与契约

legacy importer 将 tval 75 / sval 32 映射为 `apply-heroism`。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 严格导入成功，`consumable-effect` 从 75 降至 74，`scroll-effect` 保持 15；真实包保持 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races，源码校验、编译与二进制回读 hash 均为 `47b741de879cefd63ad79a6d9ea4643c1e37b4444c63b9b581a3598a620241cc`。

fixture 443 连续使用同一堆叠中的两瓶药水，分别掷出 50 和 36 ticks；支付两次行动时间后最终 Hero 剩余 66 ticks。该 fixture 固定 max HP 33→43、melee/ranged skill +12、Fear 免疫、两次消费、首次 Aware、两次效果 RNG、两个知识分支事件和存档回读；导入器既有表测试增加 sval 32。没有增加核心、save、错误目标、内容校验、协议、Web 或 Tauri E2E 专项测试。

内容 hash 属于 state hash 输入，因此既有 442 条 fixture 只替换 `stateHash` / `saveRoundTripStateHash` 字段；去除这些字段后的完整 assertions 投影必须保持不变。

## 4. 明确遗留

- 原版 `_potion_power` 与 Alchemist 特例等待对应职业纵切，不建立 Heroism 专用职业覆盖表；
- Berserk Strength、Sight、Blindness 和其他药水继续独立处理；
- Blindness 所需的概率状态抗性语义尚未建立，不能把现有 `RES_BLIND` 映射直接当作精确实现；
- 本轮不增加新的通用状态、状态抗性框架、物品状态 DSL 或协议表面；
- 剩余 74 个 `consumable-effect` 继续按独立事务分组；剩余 15 个 `scroll-effect` 不与本轮合并。
