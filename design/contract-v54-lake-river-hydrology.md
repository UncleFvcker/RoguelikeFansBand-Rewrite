# Contract v54：湖泊与河流水文阶段

状态：已实现

## 原版依据与范围

FrogComposband 原版 `src/generate.c` 在墙体底图后调用 `gen_caverns_and_lakes()`，由 `src/rooms.c::build_lake()` 生成带深浅分层的湖泊；房间建立后，`src/streams.c::add_river()` 从边界向内部递归绘制深水中心与浅水边缘，再由 tunnel 阶段切开地貌。contract-v54 保留“基础地貌先行、结构保证连通”的阶段关系，但以内容引用、显式面积预算、规范候选顺序和可回放 RNG 取代全局概率、随机重试和隐式 dungeon flags。本版只建立 lake/river；streamer、destroyed 与 maze 留给后续契约。

## 内容契约

- `layout.lake` 引用一对 `deepTerrainId/shallowTerrainId`；`generationBudget.lakeAreaTiles` 与 `lakeDeepAreaTiles` 必须同时存在。湖泊总面积为 24 到楼层内部面积，深水至少 4 格，并保留至少 8 格浅水。
- `layout.river` 同样引用深浅 terrain；`generationBudget.riverAreaTiles` 必须同时存在。预算必须足够容纳任一内部边界起点到地图中心的最坏中心线，并且不得超过内部面积。
- 深水必须不可行走，浅水必须可行走；两者必须互不相同，也不得复用楼层 wall/floor、主题 floor 或 cavern terrain。
- 湖泊与河流同层存在时必须引用完全相同的深浅 terrain 对，对应原版只允许匹配湖泊类型的河流。
- 所有 hydrology 字段只允许与完整的 `layout` 房间预算共同出现；缺失定义、缺失预算、错误可行走性、不同材质对或越界面积均拒绝编译。

## 确定性生成顺序

1. 以 wall terrain 建立底图，按 v53 规则生成 cavern。
2. lake 从地图中心开始。四向连通前沿按 `y/x` 排序，多候选时消费一次有界抽取，直到精确达到 `lakeAreaTiles`；插入序列前 `lakeDeepAreaTiles` 格为连通深水核心，其余为浅水。
3. 生成并初次绘制房间。river 从四个内部边界之一开始，边界和边界坐标各按固定顺序抽取；中心线以单步四向移动连接湖心或地图中心，双轴均可推进时消费一次轴向抽取。
4. river 深水中心线完成后，从整条中心线的四向前沿按 `y/x` 排序扩展浅水，直到精确达到 `riverAreaTiles`。
5. 房间再次开凿，随后按 v53 规则连接 tunnel。房间、走廊与固定 terrain 可覆盖早期水文，所以面积预算定义的是该阶段的精确绘制量，不承诺最终地图仍可见的格数；这一优先级保证内容出生点可行走、主房间链始终连通，并自然形成渡口。
6. Vault、terrain feature、actor、loot 和守护者阶段保持既有顺序。

## Demo 与验证

- 新增不可行走的 `demo.terrain.resonance-water-deep` 和可行走的 `demo.terrain.resonance-water-shallow`。
- 深度 9 生成 70 格湖泊，其中 28 格深水；深度 10 生成 76 格湖泊、30 格深水和 52 格河流。
- 核心测试覆盖湖泊总面积、深水面积、四向连通、河流精确面积、中心命中、内部边界起点、完整楼层存档合法性和 v53 已生成楼层不回填。
- 内容测试覆盖预算配对、深浅可行走性和 lake/river 材质兼容；`contract-v54` 从 v53 迁移 106 个 exact fixtures，并新增深度 9 lake 与深度 10 lake/river 两个种子场景，共 108 个 exact fixtures，0 waiver。

## 版本与兼容

- 协议：1.54；
- 内容包：1.47.0；
- content hash：`e3c0d8653f86663c6bb7eb2cf99caf9d1ba5a259566560d7d70bb9592de2b1e9`；
- terrain：40；
- save 容器：v1；
- state-hash：Schema v19；
- active baseline：contract-v54，108 exact fixtures。

v53 content hash 已加入内置迁移白名单。旧存档继续保存生成后的普通 terrain/actor/item 与 RNG；已经生成的 v53 楼层不会补绘湖泊或河流，也不会额外推进 RNG。未知内容 hash 仍严格拒绝。
