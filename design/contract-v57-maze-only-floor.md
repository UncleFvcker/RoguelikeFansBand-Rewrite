# Contract v57：Maze-only 专用楼层模式

状态：已实现

## 原版依据

FrogComposband `src/generate.c` 的 `DF1_MAZE` 是独立楼层生成分支：maze 建立后直接跳过普通房间循环，随后才继续楼梯和矿脉等公共阶段。contract-v55 只移植了迷宫地貌，仍会在同一张图上雕刻房间与走廊；contract-v57 修正这一结构差异，让 maze 真正替代 rooms。

## 内容边界

- `layout.mode` 新增默认兼容的 `rooms` 与显式 `maze-only`；`layout.rooms` 改为可选。
- `rooms` 模式必须声明房间几何和 `roomPlacements/roomAreaTiles`，且不再允许叠加 maze。
- `maze-only` 必须声明 maze 和严格匹配公式的 `mazeFloorTiles`，不得声明 rooms、cavern、lake、river、destroyed、pit、Vault、nest、动态 group 或 terrain feature；streamer 仍可在迷宫之后生成。
- demo 深度 9 改为 15×15、127 个可行走格的专用迷宫，保留 24 格 streamer、一个表驱动 encounter 和三个表驱动 loot placement。
- v56 的 5×5、25 actor pit 移到深度 10，与 lake、river、destroyed、streamer 和最终守护者共同保留；最终层 actor 总预算为 30。

## 生成与落位

- maze 仍按行优先格点、随机根节点和固定 north/east/south/west 候选执行 DFS。
- 从行优先最小通路格开始做 BFS，取稳定最远点作为入口；再从入口做 BFS，取稳定最远点作为下行锚点。同距离按 `y/x` 决胜。
- 上楼梯放在入口、下楼梯放在远端锚点；固定陷阱放在两锚点最短路径中点。完美迷宫不额外插入会切断唯一路径的门。
- encounter/loot 只从 maze 可行走集合落位，排除楼梯、陷阱和既有占位；候选按距入口从远到近、再按 `y/x` 排序。
- 生成结果中的 player、actor 和 ground item 必须全部位于同一连通可行走区域。

## 兼容性

- 协议：1.57；内容包：1.50.0；content hash：`d209d68a6a39af21eee8d1a951684be86e847ab570823c9c2604fa199e4571e1`。
- save v1 与 state hash Schema v19 不变。楼层仍只保存最终 terrain、actor、item 和 RNG。
- v56 content hash 已加入已知迁移集合；已生成楼层不会被重建为 maze-only，不移动既有 pit/loot，也不推进 RNG。
- Core 共 102 项测试；`contract-v57` 迁移 112 个 exact fixtures，并新增两个 maze-only seed，共 114 个 exact fixtures、0 waiver。

## 后续

下一纵切是多个楼梯、稳定连接 ID、独立到达点和 shaft；pack identity/AI 与同层多区域主题继续排在其后。
