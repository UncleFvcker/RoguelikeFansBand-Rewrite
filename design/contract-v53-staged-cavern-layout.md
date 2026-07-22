# Contract v53：分阶段洞穴地貌与房间几何预算

## 原版生成管线参考

原版 `src/generate.c` 的 `cave_gen()` 并不是单一房间生成器。它先建立墙体底图，再调用 `gen_caverns_and_lakes()`，随后由 `generate_rooms()` 建立普通、分形洞穴、模板和 Vault 等房间；房间中心经过打乱后由普通或洞穴式 tunnel 连接，之后还可能叠加 river、streamer、destroyed level，最后才放置边界、楼梯、怪物、陷阱、物品和守护者。`src/rooms.c` 的 `build_cavern()` / `build_lake()` 使用高度图和连通区域，`src/streams.c` 则负责 river、矿脉和毁坏区。

contract-v53 不复制原版全局变量、随机重试或高度图实现，而是把这一思路收敛成可逐步扩展、可预算、可回放的确定性阶段：本版同时建立 cavern 基础地貌和多房间几何；lake、river、streamer、destroyed 与 maze 继续沿同一 layout 管线扩展。

## 内容契约

- 程序化楼层新增可选 `layout`。`layout.rooms` 声明 5–9 格的宽高范围，以及 1–2 个不可重复的加权 `rectangle/cross` 形状；`layout.cavern` 可引用一个独立、可行走且不同于楼层 wall/floor 的 terrain。
- `generationBudget.roomPlacements`、`roomAreaTiles` 与 `layout` 必须同时出现。房间数量范围为 2–6，房间面积预算不得超过楼层内部面积，也必须足够容纳每个槽的最小候选。
- `generationBudget.cavernAreaTiles` 与 `layout.cavern` 必须同时出现，范围为 16 到楼层内部面积。
- 形状权重范围为 1–1,000,000。十字房间面积为 `width + height - 1`，矩形房间面积为 `width × height`。
- 带 layout 的楼层不使用旧 `vaultId` 固定落位；独立空间 Vault 阶段仍可在后续版本与 layout 组合验证。

## 确定性生成顺序

1. 完成深度主题选择并以 wall terrain 填充地图。
2. 若声明 cavern，从地图中心开始扩展四向连通前沿，候选按 `y/x` 规范排序；每次从前沿抽取一个位置，直到精确达到 `cavernAreaTiles`。cavern 使用自己的 terrain ID。
3. 将内部地图稳定划分为网格槽：2–4 个房间使用两列，5–6 个房间使用三列，行数向上取整。每个槽按加权形状选择，再从按位置、尺寸规范枚举且符合剩余面积预算的候选中选择。单形状或单位置不消费对应 RNG。
4. 精确生成 `roomPlacements` 个互不重叠房间。前两个稳定命名为 `entry/remote`，其余为 `room.3` 起的顺序 ID；房间实际模板面积总和不得超过 `roomAreaTiles`。
5. 房间按槽顺序以 L 形走廊串联，并额外连接 `entry` 中心与 cavern 原点。房间和走廊使用当前主题 floor，可覆盖早期 cavern 地貌；因此 cavern 预算是基础地貌阶段的绘制预算，不是最终可见格数量。
6. 再执行固定门、楼梯、陷阱、Vault、terrain feature、actor 和 loot 阶段。terrain feature 的 room 判断使用真实矩形/十字模板，而非包围盒；独立 cavern terrain 不会被误判为 corridor。
7. 表驱动普通 encounter 和 floor loot 按 ordinal 在所有非入口房间之间轮转，避免新增房间只作为空装饰。

## 原创场景与验证

- 新增 `demo.terrain.resonance-cavern`。
- 共鸣压力地牢深度 9 使用 4 房间、96 房间面积和 56 cavern 基础面积；深度 10 使用 5 房间、112 房间面积和 64 cavern 基础面积。两层都按 `rectangle:cross = 3:2` 选择形状。
- 单元测试覆盖 layout/预算配对、尺寸、权重、terrain 可行走引用、精确房间数、面积上限、互不重叠、矩形/十字权重可达、cavern 精确面积与四向连通、遭遇/掉落容量、确定性和 v52 已生成楼层不回填。
- `contract-v53` 从 v52 迁移 104 个 exact fixtures，并新增两个不同种子的最终层 cavern/layout 场景；active baseline 共 106 个 exact fixtures，0 waiver。

## 版本

- 协议：1.53；
- 内容包：1.46.0；
- content hash：`11a28d24125572468148dce77f0082340ab82a3a7ef87637303578681b31c4e9`；
- save：v1，不增加字段；
- state hash：Schema v19，不增加字段；
- active baseline：contract-v53，106 exact fixtures。

## 明确延后

- 原版式 lake、river、streamer、destroyed level、maze 与完整分形高度图；
- tunnel 转向/穿墙/交叉口门预算、房间中心打乱和生成失败后的整层重试；
- pit、pack AI、同层多区域主题、多入口、大模板连通性和分支连接。
