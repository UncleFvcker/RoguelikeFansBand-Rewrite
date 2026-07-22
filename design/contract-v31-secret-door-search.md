# Contract v31：秘密门、搜索与地形知识

状态：协议 1.31 / contract-v31 active baseline

## 已完成边界

- `TerrainDefinition` 新增成对的 `concealedAsTerrainId` / `searchCheckDifficulty`。伪装地形必须存在、不能形成伪装链，并且与真实 terrain 具有相同的行走和阻光语义，避免未发现前后的碰撞或 LOS 泄漏。
- `ActorDefinition` 新增明确的 `searchSkill`；原创探索者声明 24。搜索能力进入 `DerivedStatsPipeline` 的 `SearchSkill`，不复用攻击、开锁或前端显示值。
- 原创内容新增 `demo.terrain.door-secret`：真实状态是一扇带锁、可开锁/破门的门，未发现时投影为 `demo.terrain.wall`，搜索难度为 8。程序化层首次生成该秘密门。
- 协议新增 `Search` 命令和 `CheckKind::SearchTerrain`。核心按北、东北、东、东南、南、西南、西、西北检查相邻、尚未发现的隐藏 terrain；每个候选复用结构化 `CheckContext` / `CheckResult`。
- 搜索失败输出 `terrain.search-empty`；成功按稳定位置顺序写入玩家地形知识、输出 `terrain.secret-discovered`，并把该格加入 `changedCells`。
- `CellDto` 使用玩家已知 terrain 投影。未发现的秘密门输出墙 ID，发现后才输出秘密门 ID；真实 terrain 继续用于移动、寻路、投射和 LOS。
- contract-v30 的 `terrainInteractions` 同样基于玩家已知 terrain。未发现时不会泄漏开门/破门动作；发现后立即出现相应动作。
- 当前层和每个 `FloorState` 保存稳定排序的 `revealedTerrain` 位置。发现知识随离层仓库和 save v1 往返；非法、重复、越界或指向非隐藏 terrain 的知识记录会被拒绝。
- 地形知识属于权威玩家状态并进入 state-hash Schema v15；普通 FOV `explored` 仍是显示记忆，不进入 hash。
- 秘密门一旦打开或撞破，terrain 转换为普通开启/破损门，同时移除不再需要的隐藏知识位置。之后关闭会得到普通未锁门，不重新变成秘密门。
- 前端使用大写 `S` 搜索，小写 `s` 继续保留给 WASD 向南移动；搜索结果通过 Fluent 输出中英文消息。

## 协议、内容、存档与基线

- 协议升至 1.31。
- 内容包升至 1.25.0，content hash 为 `e69258b4a303a38c10221f90d01c49628eb9ef737e97c7e777fe30070a025f81`，terrain 数量增至 9。
- save schema 继续为 v1，新增带默认值的当前层/离层 `revealedTerrain`；旧存档缺失时按空知识载入。
- state hash 升至 Schema v15，覆盖当前层和离层的地形发现知识。
- 已经访问的 v30 程序化层不会把普通锁门补成秘密门；尚未访问的层首次生成时使用 v31 内容。
- contract-v31 从 v30 迁移 67 个 exact fixtures，并新增“秘密门不泄漏、首次搜索失败、第二次发现、知识与交互列表持久化”场景，共 68 个。

## 与原版 RFB v1.3.0.7 对比

原版 `src/cmd2.c` 的 `do_cmd_search()` 消耗 100 能量并调用 `src/cmd1.c::search()`。`search()` 以 `skills.srh` 为基础，失明、无光、混乱或幻觉会把能力降为十分之一；随后遍历玩家周围 3×3 的每个格子，每格进行一次百分比抽取，并通过 `is_hidden_door()` / `disclose_grid()` 发现秘密门、隐藏陷阱或箱子陷阱。

相同点：

- 主动搜索是消耗完整行动的独立命令；
- 搜索使用独立角色能力，并检查玩家附近的格子；
- 失败保持隐藏状态，可以在后续回合重复尝试；
- 发现后地图表现和可交互能力立即变化，并随楼层保存；
- 搜索和发现都产生明确的玩家消息。

主动差异：

- 重构把真实 terrain 与玩家知识分离；原版通常通过 `mimic` / `disclose_grid()` 改写格子的公开 feature 状态。
- 重构普通快照和交互查询只输出玩家已知 terrain，并以 exact fixture 防止位置泄漏；原版没有独立协议投影层。
- 当前只对真实存在且尚未发现的相邻隐藏 terrain 执行结构化检定；原版会对包含玩家自身在内的 3×3 每个格子固定抽一次百分骰。因此两者 RNG 消耗模型主动不同。
- 当前未实现失明、无光、混乱和幻觉修正，也没有搜索模式、命令重复或被动搜索。
- 当前只发现秘密门；原版同一函数还发现隐藏陷阱和箱子陷阱。
- 重构发现知识进入 save、离层仓库、state hash 和 replay；原版没有对应跨平台 DTO/hash 边界。

本切片复刻的是“搜索能力对附近隐藏 feature 进行可重复发现，并改变玩家可知地图”的核心关系，同时先建立后续陷阱必须复用的知识安全边界。

## 下一步

下一纵切建立隐藏陷阱与解除：陷阱真实位置复用本切片的知识投影，主动搜索负责发现，踩入负责触发，解除命令复用结构化检定和 contract-v30 相邻交互查询。箱子陷阱、被动搜索和状态修正继续后补。
