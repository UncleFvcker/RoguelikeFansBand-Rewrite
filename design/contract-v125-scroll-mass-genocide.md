# Contract v125：Mass Genocide 卷轴

日期：2026-07-29

Contract v125 接入原版卷轴 sval 45 的 Mass Genocide。协议 DTO 未变化，继续使用 `1.118`；demo 内容包为 `1.116.0`，save 容器保持 v1，state hash Schema 保持 `52`。active baseline 包含 428 条 exact fixtures、零 waiver，内置内容 hash 为 `39a7a79bdabafa301140266e7119735a0a0f16ef6a7071b8c5d06de6a53655a8`。

## 1. 范围

内容层新增 self-only 的 `mass-genocide` 物品效果，demo 固定 `power=300`、`radius=20`。候选是与玩家 RFB 距离不超过 20 的全部存活 actor，不要求玩家可见或存在 line of effect；候选实体 ID 排序后逐个结算，因此 actor 容器顺序不影响结果。

结算复用既有 Nearby Genocide 的单一候选处理函数。带 `unique` 或 `guardian` tag 的 actor 必定抵抗；其他目标按既有等级对抗决定移除或抵抗。成功目标从权威 actor 集直接移除，不产生 XP、掉落、尸体、任务进度、普通死亡事件或守护者击败/胜利事务。`guardian` 保护是必要的不变量边界：入口守卫不一定同时带 `unique`，直接移除会绕过 campaign 事务。

## 2. 事务、知识与 RNG

目标必须是自身；错误目标继续在消费、world tick 和 RNG 前拒绝。合法使用先记 Tried 并消费一张卷轴，再结算全部候选、标记 Aware，并发出一个聚合 `item.use-mass-genocide` 事件，事件记录移除数、抵抗数和总疲劳伤害。

每个候选都先抽一次 `1d3` 疲劳，包括必定抵抗的 unique/guardian；普通候选随后再抽一次 `[0,power)` 等级对抗。疲劳总和在候选结算后一次性扣除，沿用当前项目既有 Nearby Genocide 的 RNG 顺序。没有候选时仍消费、推进时间并变为 Aware，但不抽效果 RNG、疲劳为零。

## 3. 导入与契约

legacy importer 以表式映射接入 tval 70 / sval 45。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 严格导入成功，`scroll-effect` 从 24 降至 23。

真实包包含 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races；源码校验、二进制编译与产物回读 hash 均为 `aeba4b11bddc16259fd02558f666bdca774fe3f5dd7d347b35330cc6bc24436b`。

新增 fixture 428 在半径内放置一个普通 actor 和一个 guardian。一次使用固定普通目标移除、guardian 抵抗、疲劳 5、聚合事件 `removed=1/resisted=1`，总 `rngDrawCounter=6`，并锁定消费、Tried + Aware 与移除格变化。另有一个窄单测固定空候选时消费、Aware 和零新增 RNG。既有 427 条 fixture 只因内置 content hash 输入更新 state hash，其他 assertions 零变化。

## 4. 明确遗留

- 普通 Genocide 的 glyph 选择、骑乘目标和交互 UI 不在本轮；
- 原版 `NOGENO`、questor、骑乘和 virtues 依赖尚未建立的规则，不新增无消费者字段或兼容框架；
- 本轮不建立通用 actor-removal、死亡模式或 Genocide immunity 框架；
- 剩余 23 个 `scroll-effect` 继续按世界/地形、状态和物品/成长事务拆分。
