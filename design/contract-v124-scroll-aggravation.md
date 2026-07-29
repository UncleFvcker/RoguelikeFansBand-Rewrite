# Contract v124：激怒怪物卷轴

日期：2026-07-29

Contract v124 接入原版卷轴 sval 1 的 Aggravate Monster。协议 DTO 未变化，继续使用 `1.118`；demo 内容包为 `1.115.0`，save 容器保持 v1，state hash Schema 保持 `52`。active baseline 包含 427 条 exact fixtures、零 waiver，内置内容 hash 为 `337e8599f02e53264b45ac1e899eb47b5ec6f4eeb6be0ae31b517c67ae6fb82b`。

## 1. 范围

内容层新增无参数、self-only 的 `aggravate-monsters` 物品效果。运行时复用当前权威视距 `8` 作为原版 `MAX_SIGHT` 的系统对应值：与玩家 RFB 距离小于 `16` 的存活 actor 清除 `rfb.status.sleep` 并设为警戒；距离不超过 `8`、与玩家存在几何 LOS 且不属于玩家阵营的 actor 延长 `100` ticks `rfb.status.haste`。

LOS 读取权威地形的 `blocksSight`，不经过玩家的显示可见性，因此 blindness 不会阻止加速判定。玩家控制的永久召唤物仍会被声音唤醒，但不会获得敌对加速。效果直接复用已有 sleep、alerted、haste 与 Extend 状态语义，没有新增 actor outcome DTO、逐目标事件、计划器快照、协议字段或存档字段。

## 2. 事务、知识与 RNG

目标必须是自身；错误目标继续在消费、world tick 和 RNG 前拒绝。合法使用先记 Tried 并消费一张卷轴，再结算唤醒与加速、标记 Aware，并只发出一个 `item.use-aggravate` 事件。即使当前没有 actor 或没有任何状态变化，卷轴仍正常消费、推进时间并变为 Aware。

效果本身没有随机检定、伤害骰或逐 actor RNG，也不改变 actor 遍历顺序。怪物随后是否行动继续由既有能量调度器决定，不属于卷轴效果 RNG。

## 3. 导入与契约

legacy importer 以表式映射接入 tval 70 / sval 1。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 严格导入成功，`scroll-effect` 从 25 降至 24。

真实包包含 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races；源码校验、二进制编译与产物回读 hash 均为 `3dd566a5705f3d7d9671a2fbabc03451802718024a1870b236af3d0088dd8ec7`。

新增 fixture 427 放置两个带 sleep、未警戒的 actor：一个在玩家 LOS 内，另一个位于近距墙后。一次使用固定两者均醒来并警戒，只有 LOS 内敌对目标获得 haste，同时固定消费、Tried + Aware、单一使用事件和零效果 RNG。fixture 使用既有玩家 haste 预条件缩短调度窗口，避免把随后怪物行动的移动与战斗 RNG 混入本效果契约。既有 426 条 fixture 只因内置 content hash 输入更新 state hash，其他 assertions 零变化。

## 4. 明确遗留

- 原版近距敌对怪物还可能获得 `MFLAG2_NOPET`；当前没有对应的驯服关系状态，不新增无消费者字段；
- 原版 `very_nice_summon_hack`、临时 nice 标记与骑乘 bonus 刷新依赖尚未建立的召唤/骑乘系统；
- 当前距离复用项目权威视距 8，不单独引入原版 `MAX_SIGHT=20` 的第二套感知常量；
- 剩余 24 个 `scroll-effect` 继续按世界/地形、状态和物品/成长事务拆分。
