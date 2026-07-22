# Contract v55：迷宫、毁坏区与岩脉阶段

状态：已实现

## 原版依据与范围

FrogComposband 原版 `src/generate.c::cave_gen()` 将 `DF1_MAZE` 作为普通房间分支的替代结构，由 `src/rooms.c::build_maze_vault()` 对格点图执行随机深度优先遍历；普通楼层可在房间后调用 `src/streams.c::destroy_level()` 放置多个爆心，在隧道和楼梯后调用 `build_streamer()` 将普通花岗岩改写为石英或岩浆矿脉。contract-v55 保留 spanning-tree maze、多震中破坏和只改写岩壁的 streamer 语义，但把全局 flags、重试上限和隐式概率收敛为内容定义、显式预算与可回放候选顺序。

## 内容契约

- `layout.maze` 声明 9 格以上、可放入楼层内部的奇数 `width/height`。`generationBudget.mazeFloorTiles` 必须同时存在，并严格等于 `2 * ceil(width/2) * ceil(height/2) - 1`。
- `layout.destroyed.terrainId` 引用独立、不可行走且不同于 wall/floor、主题 floor、cavern 与水文 terrain 的废墟地形。`destructionCenters/destroyedAreaTiles` 必须成组出现；震中为 1–4 个，每个至少保留 8 格预算，总面积不超过内部面积的一半。
- `layout.streamers` 提供 1–4 个按 terrain ID 规范排序的加权候选。候选 terrain 必须不可行走、互不重复，并与 wall/floor、主题 floor、cavern、水文和 destroyed terrain 不同。
- `streamerPlacements/streamerAreaTiles` 必须与非空 streamer 候选同时出现；条数为 1–4，总面积至少为每条 4 格且不超过内部面积四分之一。
- 所有字段只允许出现在已有 room/layout 预算的程序化楼层；缺失配对、偶数或越界迷宫、错误迷宫面积、非法引用、重复候选和错误可行走性均拒绝编译。

## 确定性阶段顺序

1. wall 底图、cavern 和 lake 沿 v53/v54 规则生成。
2. maze 将居中的奇数矩形重置为 wall；从规范编号的格点中抽取根节点，以 `north/east/south/west` 顺序建立未访问邻居候选，多候选时消费一次抽取。深度优先 spanning tree 精确开凿声明数量的主题 floor，并保证全部 maze floor 四向连通。
3. 房间初次开凿后，destroyed 从中央安全带抽取互不重复的震中；多源四向前沿按 `y/x` 排序并扩展到精确 `destroyedAreaTiles`，全部绘制为独立废墟 terrain。
4. river 随后绘制；只要存在 destroyed 或 river，房间会再次开凿，再由 tunnel 建立主连通骨架。因此毁坏与水文面积是阶段绘制量，结构地板可覆盖它们。
5. streamer 对每条先按规范权重选择 terrain，再抽取中央普通岩壁起点和八向方向；沿射线收集一格邻域内的 wall 候选，按 `y/x` 排序后抽取并改写。候选不足时从全图剩余普通 wall 稳定回退；若楼层已无普通 wall，则在已绘制结果处稳定停止，不覆盖 floor、水、废墟、门或其他特殊 terrain。
6. 固定楼梯、门、陷阱、Vault、terrain feature、actor、loot 与守护者保持既有后续顺序。

## Demo 与验证

- 新增可挖掘的 `demo.terrain.resonance-vein` 与独立 `demo.terrain.resonance-ruin`。
- 深度 9 增加 15×15 maze（127 个通路格）及 2 条、24 格 streamer；深度 10 增加 2 个震中、48 格 destroyed 区及 2 条、24 格 streamer。
- 核心测试覆盖 maze 精确面积与四向连通、destroyed 精确面积与组件上限、streamer 只改写 wall、完整十层存档合法性及 v54 已生成楼层不回填。
- 内容测试覆盖 maze 预算公式、destroyed 配对和 streamer terrain 可行走性；`contract-v55` 从 v54 迁移 108 个 exact fixtures，并新增深度 9 maze/streamer 与深度 10 destroyed/streamer 场景，共 110 个 exact fixtures，0 waiver。

## 版本与兼容

- 协议：1.55；
- 内容包：1.48.0；
- content hash：`52c3db16ad5240ff83ba652b09ef70cccac991a586b593f84c11956a55539596`；
- terrain：42；
- save 容器：v1；
- state-hash：Schema v19；
- active baseline：contract-v55，110 exact fixtures。

v54 content hash 已加入内置迁移白名单。旧存档继续直接保存最终 terrain/actor/item 与 RNG；已生成的 v54 楼层不会补建 maze、destroyed 或 streamer，也不会额外推进 RNG。未知内容 hash 仍严格拒绝。
