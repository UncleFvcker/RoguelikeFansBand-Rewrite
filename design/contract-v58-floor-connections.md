# Contract v58：权威楼层连接与 shaft

状态：已实现

## 原版依据

FrogComposband `src/generate.c` 的 `alloc_stairs()` 会在同层分配多座楼梯，maze 楼层也会分别分配多座上、下楼梯；`src/cmd2.c` 的 shaft 命令按方向跨越两层；`src/floors.c` 则在楼梯格保存目标 floor ID。contract-v58 采用相同的“格子携带明确目标”结构，但使用稳定内容 ID 和平台无关的存档 DTO，不复制旧内存布局。

## 内容边界

- `ProceduralFloorDefinition` 新增可选 `entryConnectionId` 与 `connections`。
- 每个连接声明全局稳定 `id`、`kind = stairs|shaft`、`terrainId`、`targetFloorId` 和非地表目标所需的 `targetConnectionId`。
- 编译器要求连接 ID 唯一、terrain 可行走且标签方向匹配；普通楼梯深度差必须为 1，shaft 必须为 2；非地表连接必须严格双向互引，并保持相同 lifecycle 与 dungeon ID。
- 根楼层必须用 `entryConnectionId` 指向返回地表的上楼连接。地表继续通过 `entryTerrainId` 选择要进入的程序化楼层。
- demo 回声地牢每层提供两组独立普通楼梯；一层与三层之间新增双向 shaft。新增 `shaft-up`、`shaft-down` terrain。

## 生成与运行时

- 每层第一组主上、下楼连接继续使用既有入口/下楼锚点，保持旧主路径稳定。
- 其余连接按规范 ID 顺序从 Vault 绘制后的合法 floor 候选中使用种子 RNG 抽取；位置互不重复，并进入 feature、actor 和 loot 占位集合。同一 seed 可精确回放，不要求额外楼梯彼此最远。
- `TraverseStairs` 先用玩家位置查找权威 connection ID，再从内容解析目标 floor/connection；目标楼层首次生成或从仓库恢复后，玩家落在独立目标连接位置。
- 当前层保存 `floorConnections`，每个离层 `FloorSaveDto` 保存 `connections`；状态校验要求非空连接集合与当前内容定义完全一致，且每个坐标上的 terrain 与声明一致。

## 兼容性

- 协议：1.58；内容包：1.51.0；content hash：`ee07c276bbe568fafc1e1d6942e9d57d158bd250ed452b32c01c774d8521e96d`。
- save 容器仍为 v1；新增字段都有空列表默认值。state hash 升至 Schema v20，并纳入当前层与离层连接映射。
- v57 content hash 已加入迁移集合。旧已生成楼层没有连接列表时不重建 terrain、actor、item 或 RNG，继续按单一 `stairs-up/stairs-down` 标签走 legacy 回退。
- Core 共 106 项测试；`contract-v58` 迁移 114 个 exact fixtures，并新增两个随机连接布局 seed 与一个 shaft 跨层往返场景，共 117 个 exact fixtures、0 waiver。

## 后续

下一纵切是持久 pack identity 与首版 pack AI；同层多区域主题排在其后。Vault 多入口和更一般的跨走廊拼接仍是独立缺口。
