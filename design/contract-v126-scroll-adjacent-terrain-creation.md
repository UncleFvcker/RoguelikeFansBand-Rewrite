# Contract v126：相邻树木与石墙创建卷轴

日期：2026-07-29

Contract v126 接入原版卷轴 sval 48/49 的 Forest Creation 与 Wall Creation。协议 DTO 未变化，继续使用 `1.118`；demo 内容包为 `1.117.0`，save 容器保持 v1，state hash Schema 保持 `52`。active baseline 包含 429 条 exact fixtures、零 waiver，内置内容 hash 为 `7d344bf57cf11e303fbbd6b98f9792e572792e97a696e9a2c1987ba6f349a149`。

## 1. 效果边界

内容层新增 self-only 的 `create-adjacent-terrain { sourceTerrainIds, targetTerrainId }` 物品效果。运行时只按固定权威八方向扫描玩家邻格，不暴露 radius、shape、LOS、过滤模式或随机目标地形。当前 demo 的两种卷轴共用明确的普通地面 ID；Forest Creation 指向原创树地形，Wall Creation 复用既有墙地形。

候选格必须在 `sourceTerrainIds` 中，并且不能有存活 actor、地面物品或权威楼层连接。玩家格不在八邻格扫描中。地形连通性不会被证明、修复或回滚；地形创建本身就是权威状态，玩家仍可通过既有挖掘规则自行开路。

## 2. 事务、知识与 RNG

目标必须是自身，错误目标在消费、world tick 和 RNG 前拒绝。所有替换在消费前规划，消费后一次性提交；每个变更格写入目标地形、清除旧 terrain reveal 状态并进入既有 `changedCells`。一次使用只发出一个聚合事件。

至少改变一格时来源从 Tried 变为 Aware。没有合法格时仍消费卷轴并推进时间，但保持 Tried、产生零效果 RNG，并发出明确的 no-effect 事件。两种效果本身始终不抽 RNG。

## 3. 导入与契约

legacy importer 从已解析并实际导出的 `f_info` 条目中收集全部 `FF_FLOOR` 地形 ID，稳定排序后写入两种卷轴；目标分别解析同一导入包内的 TREE 与 GRANITE ID，不从 `walkable`/`blocksSight` 或新增 tag 推断。

固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 严格导入成功，`scroll-effect` 从 23 降至 21。真实包保持 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races；源码编译与二进制回读 hash 均为 `1eb1303a7476dcbce4209460a0af728019680112d55a767c03d2c39ade00bdad`。

fixture 429 在同一场景依次使用两种卷轴，固定显式源地形、非源地形、地面物品排除、聚合事件、消费、Aware、零效果 RNG 和回档。另一个窄核心单测固定空结果消费、10 ticks、Tried-only、零 RNG，并让一个合法源格分别受地面物品和权威楼层连接保护。既有 428 条 fixture 只更新 `stateHash` 与 `saveRoundTripStateHash`。

## 4. 明确遗留

- 本轮不建立通用地形 selector、投影 DSL 或物品版 `AbilityEffectDefinition`；
- Trap Creation、Rune、Light/Darkness、Destruction 和其他世界卷轴继续独立分组；
- 原版投影对物品的其他副作用、职业 scroll power 特例和额外地形刷新机制不在本轮；
- 剩余 21 个 `scroll-effect` 继续按世界/地形、状态和物品/成长事务拆分。
