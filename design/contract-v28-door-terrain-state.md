# Contract v28：门地形状态与方向性交互

状态：协议 1.28 / contract-v28 active baseline

## 已完成边界

- `TerrainDefinition` 新增可选 `openToTerrainId` / `closeToTerrainId`。内容编译器要求转换单向且互为反向引用：关闭态必须不可行走并阻挡视线，开启态必须可行走且不阻挡视线。
- 原创内容新增 `demo.terrain.door-closed` 与 `demo.terrain.door-open`。程序化层声明 `closedDoorTerrainId`，生成器在两个房间之间的水平走廊中点放置一扇关闭门。
- 协议新增方向性 `OpenDoor { direction }` 与 `CloseDoor { direction }`。开门只接受相邻、具有 `openToTerrainId` 的地形；关门只接受相邻、具有 `closeToTerrainId` 且没有怪物或地面物品占用的地形。
- 成功交互原子替换一个权威 terrain ID，输出 `terrain.door-opened` / `terrain.door-closed` 并只标记该格为规则 dirty cell；不适用时输出对应 unavailable 事件。成功和失败命令当前都使用标准行动成本。
- 关闭门立即参与移动阻挡、怪物寻路、投射轨迹和 LOS/FOV；开启后同一格立即变为可行走、可透视。视野变化继续通过 `changedVisualCells` 独立输出。
- 门状态不增加平行运行时结构；当前 terrain ID 就是真值，因此活动层、离层 `FloorState`、save v1、回放和 state hash 已自然覆盖开关状态。
- 前端新增 `O → 方向` 开门、`C → 方向` 关门的两步输入，Esc 取消。纯 TypeScript 测试覆盖模式键和方向命令投影；同时补齐了 v26 楼层切换事件的本地化格式化。

## 协议、内容、存档与基线

- 协议升至 1.28；内容包升至 1.23.0，content hash 为 `f060f44c88033e8ef75478929a354d6b5b0bc5f933ca2772e79c3440940942e8`，terrain 数量增至 6。
- save schema 继续为 v1，state hash 继续为 Schema v14；两者已经保存/哈希完整 terrain ID 数组，无需重复增加 door-state 字段。
- v27 存档继续可读。已经生成的旧程序化层不会在走廊中补插门；尚未访问该层的旧存档会在首次生成时使用 v28 布局规则。
- contract-v28 从 v27 迁移 64 个 exact fixtures，并新增“下楼、走到门前、开门、穿过、关门、存档往返”场景，共 65 个。

## 与原版 RFB v1.3.0.7 对比

原版 `src/cmd2.c` 的 `do_cmd_open()` / `do_cmd_close()` 先取得方向，再检查相邻 feature、怪物和物品。普通门通过 `do_cmd_open_aux()` / `do_cmd_close_aux()` 调用 `cave_alter_feat(..., FF_OPEN/FF_CLOSE)`；`src/cave.c` 的 `feat_state()` 根据 feature state table 找到目标 feature，并由 `cave_set_feat()` 触发地图、视野和重绘更新。

相同点：

- 开门和关门都是面向相邻方向的独立行动；
- 门的开启/关闭由地形 feature 状态转换表示，而不是在玩家或 UI 上保存布尔值；
- 关闭门阻挡移动和视线，开启门允许通过并改变可见范围；
- 怪物占据目标格时不能普通关门，地面物品也可能阻止门闭合；
- 地形转换随楼层保存，返回后保持原状态。

主动差异：

- 重构使用稳定 terrain 字符串 ID 和显式互反引用；原版使用 feature 数组索引、`FF_OPEN`/`FF_CLOSE` action 和通用 `state[]` 转换表。
- 当前只有无锁、未损坏的普通门；原版还支持锁强度、开锁技能、失败重试、卡死门、破门、玻璃门、秘密门、箱子、经验、声音、easy-open 自动选方向和混乱下的特殊能量处理。
- 原版目标格有怪物时开/关命令可能转为近战攻击；重构当前返回 unavailable，不隐式改变命令类型。
- 重构所有成功/失败门命令统一消耗标准行动成本并输出强类型事件；原版部分“没有目标”路径可能不消耗完整回合。
- 重构门转换直接进入跨平台 save/state hash/replay/exact fixtures；原版没有对应协议层和状态哈希。

本切片复刻的是“方向命令驱动地形状态转换，并立即改变碰撞与视线”的核心关系，不复刻开锁、破门和自动交互的全部分支。

## 下一步

该纵切已由 [contract-v29](contract-v29-locked-door-checks.md) 完成：门可声明开锁/破门难度，玩家使用独立能力复用结构化 check，失败保持关闭，成功进入普通开启或破损 terrain。秘密门、搜索、陷阱、解除和挖掘继续拆分。
