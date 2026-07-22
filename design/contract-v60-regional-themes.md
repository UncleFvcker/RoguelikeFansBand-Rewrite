# Contract v60：同层多区域主题

## 目标

contract-v49 的“深度区域主题”仍是整层只选一个主题。contract-v60 参考 FrogComposband 以房间中心组织局部内容的方式，把同一楼层拆成多个稳定区域，让不同 terrain、怪物生态和掉落表在同层并存，同时保留走廊主链连通。

## 内容契约

- 新增独立 `regionTables` 根。每个 `RegionEntryDefinition` 声明稳定 `regionId`、`themeTableId/themeId`、局部 `encounterTableId/lootTableId`、整数权重和深度范围。
- 程序化楼层通过 `regionTableId` 启用区域生成，并在 `generationBudget.regionPlacements` 声明 2–4 个区域。区域数不得超过房间数或该深度的合格候选数；actor/loot 总预算都必须至少覆盖每个区域一个位置。
- 区域候选按权重无放回选择，因此同一候选不会在一层重复。显式 `themeId` 必须在所引主题表中完整覆盖候选深度；局部 encounter 必须只含普通候选且在楼层深度可用。
- 首版区域楼层不能同时声明楼层级 encounter/loot/theme、旧内联 spawn/Vault/nest/guardian、动态群体、terrain feature、maze-only、pit、cavern/lake/river/destroyed/streamer 或显式多连接。

## 生成语义

1. 先从深度合格候选按权重无放回选出区域，选择顺序即稳定锚点顺序。
2. 区域锚点沿房间序列均匀分布；其余房间按中心 Manhattan 距离归给最近锚点，同距按区域顺序决胜。房间是不可拆分的最小归属单元。
3. 每个房间使用所属区域的主题 terrain；连接房间的走廊使用楼层基础 terrain，作为不属于任何区域的拼接带。
4. actor/loot 总预算按区域顺序整除分配，余数给较早区域。每个位置只从该区域局部表抽取，并只能落在所属房间。
5. demo 的 `resonance-depth-2` 使用 `grotto:gallery = 3:1`，两区均被无放回选出，但入口房间归属随 seed 确定变化。

## 持久状态

- `FloorRegionSaveDto` 保存 region ID、theme ID、encounter/loot 表引用和规范排序的完整房间格集合；当前层使用 `SavePayloadV1.floorRegions`，离层使用 `FloorSaveDto.regions`。
- 载入拒绝非法或重复 ID、越界/重复/跨区域重叠格、与楼层 region table 不匹配的引用，以及区域楼层中越界的怪物或地面掉落。
- v59 及更早存档缺失区域字段时保持空列表，不重建已生成楼层，也不推进 RNG。
- 区域边界和局部表引用进入 state hash Schema v22；save 容器仍为 v1。

## 验证

- 内容测试覆盖根与 catalog 索引、深度耗尽、预算缺失/超额、引用错误和不允许的生成组合。
- Core 测试覆盖 3:1 权重的多 seed 确定性、边界互斥、房间完整归属、局部 terrain/actor/loot、不合法边界拒绝、存档回环和 v59 缺失字段兼容。
- active baseline 为 119 个 exact fixtures、0 waiver；新增两个 depth-2 seed 分别锁定 grotto 与 gallery 入口区域，并执行保存回环。

## 版本

- 协议：1.60；
- 内容包：1.53.0；
- content hash：`9789fcbbd8431ed745d8a0305cc81a54cc7e45ce79be86ed76e0227d66564a02`；
- save：v1；
- state hash：Schema v22；
- active baseline：contract-v60，119 exact fixtures。

## 明确延后

- 区域与 Vault、pit/nest、dynamic formation、terrain feature 和分阶段地貌的组合；
- 任意多边形、噪声或走廊分区，区域间专属门/边界和跨区域群体协作；
- cavern/lake/river 等非房间空间的区域归属，以及更一般的多入口连通图。
