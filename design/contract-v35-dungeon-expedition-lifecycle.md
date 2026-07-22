# Contract v35：地牢探索实例生命周期

状态：协议 1.35 / contract-v35 active baseline

## 已完成边界

- 地表是持久区域；程序化深度 1 和深度 2 共同组成一次地牢探索实例。
- 探索实例内部上下楼继续使用 contract-v34 的 `storedFloors`，因此门、陷阱、怪物、物品和知识在同一次探索期间保持。
- 从深度 1 上楼返回地表时，核心在切层事务中清除全部程序化 `storedFloors`，并输出稳定 `floor.expedition-ended` 事件。
- 再次从地表进入会重新生成深度 1、重新消费 RNG，并重置地牢内全部地形、实体、掉落和发现知识。
- 载入旧存档时，如果当前位于地表，遗留的程序化楼层会被清除；如果当前仍在地牢中，则继续保留本次探索需要的楼层。

协议 1.35，内容包 1.29.0，content hash `830b8ededc0dadb5600436137da7edb41353f945a09a4325d05546e16e75c4a8`。save v1 与 state-hash Schema v15 不变，active baseline 共 73 个 exact fixtures。

## 与原版 RFB 对比

相同点：原版 `floors.c` 在地牢内部切层时通过 `CFM_SAVE_FLOORS` 保存可返回楼层；当 `dun_level` 回到零、真正抵达地表时会移除 `CFM_SAVE_FLOORS`，随后遍历并清除全部 saved floors。再次进入时生成新洞穴。

主动差异：重构以稳定 `FloorId` 集合表示一次探索实例，并在权威事务中直接清除内存/存档楼层，不使用原版临时文件和固定大小 `MAX_SAVED_FLOORS`。另外增加结构化探索结束事件，供 UI、任务和统计系统消费。

暂未实现：显式 `DungeonInstanceId`、多地牢并存、回忆/传送/死亡结束探索、可配置永久地牢，以及任务层自己的完成/失败清理策略。
