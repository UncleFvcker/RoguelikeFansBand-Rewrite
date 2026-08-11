# Contract v245: mutation M6-B

状态：已实现。协议 `1.166`，save 容器 `v1`，State Hash Schema `v83`，内容包
`1.237.0`，内容 hash
`65d3b8a64f8f32e83b38c1e9049b3d00cd41ecc19866ae6a4d24c02262204251`。

## 运行时边界

- Random Teleport 先进行 Nexus 抗性检定，再按原版固定命中点触发半径 40 的随机传送。
- Random Banish 以 1/9000 触发，复用现有可见怪物放逐事务，最大距离 100。
- Shadow Walk 以 1/12000 触发 `15 + randint0(21)` 的通用现实改变倒计时；再次触发会取消现有倒计时。
- 倒计时到零时只重生成普通、非任务、非固定地图的程序地下城当前层，并保留连接目标。城镇、固定任务层和连续荒野不改变；`wildernessSeed` 仍只在进入世界地图时推进。
- Fumbling 以 1/10000 触发，先造成 1d25 无修正物理伤害，再从可卸下的已装备近战武器中随机选择一件掉到玩家脚下。

所有周期检查继续只在本地地图运行，并按 `sourceIndex` 排序。世界地图零触发、零额外 RNG。

## 持久与投影边界

`PlayerSaveDto.realityChangeTicks` 和 `PlayerDto.realityChangeTicks` 的合法范围为
0..=35；它进入 State Hash Schema v83。重生成产生的新地形、实体、物品、金币和
探索状态继续复用既有 `FloorState`，没有第二套地图状态或荒野 seed 路径。

聚焦测试覆盖精确触发、传送/放逐复用、1d25 与装备掉落、倒计时 save/hash 往返、
程序地下城重生成，以及地表不改变且不推进 seed。
