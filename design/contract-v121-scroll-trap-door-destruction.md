# Contract v121：相邻陷阱与门破坏卷轴

日期：2026-07-29

Contract v121 接入原版卷轴 sval 39 的 Trap/Door Destruction。协议 DTO 未变化，继续使用 `1.118`；demo 内容包为 `1.112.0`，save 容器保持 v1，state hash Schema 保持 `52`。active baseline 包含 424 条 exact fixtures、零 waiver，内置内容 hash 为 `3fd2b0a8b58531b89629aa2b50ef943a7a5687bdcb619991a26a3c81a7437bf7`。

## 1. 范围

物品效果只新增 self-only 的 `destroy-adjacent-traps-and-doors`。计划阶段按北、东北、东、东南、南、西南、西、西北扫描玩家周围八格：

- 陷阱直接替换为 `trap.disarmToTerrainId`；
- 带 `door` tag 且声明 `bashToTerrainId` 的地形直接替换为该目标；
- 没有直接 bash 目标的开启门、破损门和其他地形保持不变。

扫描不受 FOV、地形 revealed 状态、actor 或地面物品影响。执行时清除被替换位置的 `revealedTerrain` 记录，并通过既有 changed-cells 投影通知渲染层。没有新增通用地形效果 DSL、地形事务框架、`AbilityEffectDefinition` 分支或 Web 业务判断。

原版同一效果还会处理上锁或带陷阱的箱子；当前物品实例没有箱锁/箱陷阱状态，因此该分支明确留待箱子事务建立后再实现。

## 2. 事务、知识与 RNG

目标必须是自身；错误目标在消费、world tick 和 RNG 前拒绝。合法使用先冻结八邻域替换计划，再消费一张卷轴并推进一次玩家行动。

本效果不抽 RNG。即使八格内没有可替换地形，卷轴仍消费、推进时间并产生零效果事件；合法读取已经完整揭示效果，因此空用和实际破坏都会把物品知识标记为 Tried + Aware。

事件分为 `item.use-destroy-adjacent-traps-doors` 与零效果 code，均携带稳定 affected count。协议已有通用事件参数和 changed-cells 投影，故无需增加 DTO 或生成 bindings。

## 3. 导入与契约

legacy importer 以表式映射接入 tval 70 / sval 39。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 严格导入成功，`scroll-effect` 从 29 降至 28。

真实包包含 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races；源码校验、二进制编译与产物回读 hash 均为 `ad65fb2058f2a01b47ec73a616606d4550b5b807cb653d9410aafe0bfd49b6e2`。

新增 fixture 424 先在零有效邻格时使用一张卷轴，再移动到隐藏陷阱、秘密门和开启门旁使用第二张。它固定两次消费与 action tick、零 RNG 增量、affected count `0`→`2`、隐藏陷阱和秘密门替换、开启门与破损门保留、Tried + Aware 以及 save round trip。既有 423 条 fixture 只因内置 content hash 输入机械更新 state hash，其他 assertions 不变化。
