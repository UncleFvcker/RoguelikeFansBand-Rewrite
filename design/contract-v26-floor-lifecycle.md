# Contract v26：楼层生命周期与确定性程序化楼层

状态：协议 1.26 / contract-v26 active baseline

## 已完成边界

- 世界内容新增稳定 `initialFloorId` 和一个 `proceduralFloor` 定义。当前固定入口层为 `demo.floor.surface`，首个程序化层为 `demo.floor.echo-depth-1`。
- 协议新增 `TraverseStairs`；前端将 `<` 和 `>` 映射到同一权威命令。核心只在当前格具有相应 `stairs-down` 或 `stairs-up` 标签时切换，否则输出 `floor.transition-unavailable`，不隐式寻找楼梯。
- 首次下楼使用主 RNG 固定抽取四次房间起点，生成两个 6×5 房间和一条先横后纵的 L 形走廊，并在首个房间中心放置返回楼梯。同一种子、内容和命令序列产生相同地图与 state hash。
- 当前层离开时保存为显式 `FloorState`，包括尺寸、地形、玩家离层位置、怪物、地面物品、怪物携带物和探索记忆；玩家背包、装备、知识和其他全局物品继续随玩家移动。返回已访问楼层时恢复保存状态，不重新生成。
- `GameSnapshot` 与 `GameUpdate` 输出 `floorId`。成功切换输出 `floor.transition`，并以完整目标地图作为 changed cells，避免前端把不同楼层的同坐标格误当成局部更新。
- save v1 新增 `currentFloorId` 与 `storedFloors`。载入会验证楼层 ID、尺寸、地形引用、玩家位置、实例 ID、怪物携带引用和当前层/离层仓库互斥关系。

## 协议、内容、存档与基线

- 协议升至 1.26；内容包升至 1.21.0，content hash 为 `febe50b7a55a637a05d78135f14aa8f72fa457632ae8d705c002e92acf9e4fd9`。
- save schema 继续为 v1；旧文件缺失 `currentFloorId` 时迁移到世界 `initialFloorId`，缺失 `storedFloors` 时按空仓库载入。
- state hash 升至 Schema v14，显式纳入当前 `FloorId` 和按 ID 排序的离层 `FloorState`；探索记忆继续属于存档/UI 连续性，而不进入权威 state hash。
- 回放元数据同步要求 Schema v14，并覆盖楼层切换命令的确定性验证。
- contract-v26 从 v25 迁移 62 个 exact fixtures，并新增固定种子 27 的“移动到楼梯、首次生成程序化层、存档往返”场景，共 63 个。

## 与原版 RFB v1.3.0.7 对比

原版 `src/cmd2.c` 的 `do_cmd_go_up()` / `do_cmd_go_down()` 先验证当前地形的 `FF_LESS` / `FF_MORE`，再通过 `CFM_UP`、`CFM_DOWN`、`CFM_SAVE_FLOORS` 等 mode flag 请求离层。`src/floors.c` 的 `leave_floor()` 保存可返回的当前层并连接楼梯，`change_floor()` 优先 `load_floor()` 恢复已有层，否则调用 `generate_cave()` 创建新层。`saved_floor_type` 记录数值 floor ID、深度、访问时间和上下连接；正式存档会把最多 20 个临时楼层文件重新写入主存档。

相同点：

- 玩家必须站在方向匹配的楼梯上才会触发正常上下楼；
- 首次进入未访问楼层时生成地图，沿可返回连接再次进入时恢复已有楼层；
- 楼层地形、怪物和物品状态随离层保存，RNG 与存档共同维持连续游戏；
- 新层入口会建立反向楼梯，使上下层形成可往返连接。

主动差异：

- 重构使用内容定义的稳定字符串 `FloorId`、内存中的显式 `FloorState` 和按 ID 排序的仓库；原版使用短整数 ID、mode flag、固定 20 槽元数据和临时楼层文件，并会淘汰最久未访问层。
- 重构把当前层和离层仓库直接纳入跨平台 Schema v14 state hash、save v1 与 exact fixtures；原版没有对应的跨平台状态哈希或协议基线。
- 当前生成器只固定生成两个房间和一条 L 形走廊，严格消费四次布局 RNG；原版 `generate_cave()` 会按城镇、荒野、任务、竞技场和地牢分支调用完整 `level_gen()`，并可能因对象/怪物溢出重试生成。
- 原版恢复旧层后会让多数非宠物怪物回满生命、清除多种临时状态，并按离层时间补充随机怪物；重构当前精确恢复离层时的怪物状态，不模拟离层期间的生态演化。
- 当前只有固定入口层和一个程序化层，没有深度数值、竖井、随机落点、楼层传送、任务离层检查、宠物迁移、离层时间演化、旧层淘汰或不可返回模式。
- 当前程序化层尚不分配怪物、物品、门、陷阱、vault 和主题内容；fixtures 约束重构自身的确定性协议，不复现原版地图形状或随机序列。

本切片追求原版楼层生命周期的规则关系：楼梯触发、首次生成、可返回保存和恢复；不追求原版 C 数据布局、生成算法或 RNG 序列一致。

## 下一步

确定性房间内容分配已由 [contract-v27](contract-v27-procedural-room-content.md) 建立：程序化层声明深度、稳定房间来源、怪物候选和地面掉落表，并在返回时恢复同一批实例。下一纵切建立关闭门及开关门状态；陷阱、多深度连接、任务层和旧层淘汰策略继续拆成后续纵切。
