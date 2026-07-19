# Contract v19：携带重量与拾取容量

状态：协议 1.19 / contract-v19 active baseline

## 已建立边界

- 玩家 actor 内容定义新增整数 `carryCapacityTenthsPound`；原创探索者容量为 100，即 10.0 磅。物品继续使用 contract-v18 的整数 `weightTenthsPound`。
- 当前携带重量统计背包与已装备物品，按 `单件重量 × 数量` 饱和求和；装备、卸下只改变位置，不改变携带总重。
- 拾取仍选择脚下稳定实例 ID 最小的物品堆。核心在任何堆叠合并或实例移动之前计算整堆重量；若 `current + pickup > capacity`，整堆拒绝，不部分拾取、不分配实例 ID，也不抽取 RNG。
- 超重拒绝仍是一次标准行动并推进调度，输出结构化 `item.pickup.over-capacity` 事件，包含物品、数量、当前重量、拾取重量和容量。
- 为兼容历史存档，载入时不因既有超重状态失败；超重角色可以丢弃、投掷或消耗物品，但在回到容量内之前不能继续拾取正重量物品。

## 协议、内容与基线

- 协议升至 1.19；`PlayerDto` 新增 `carriedWeightTenthsPound` 和 `carryCapacityTenthsPound`，HTML 背包标题显示权威总重/容量。
- 原创内容包升至 1.14.0；1.13.0 content hash 加入显式内置存档迁移白名单。
- save schema v1 / state hash schema v9 不变；容量来自锁定内容，携带重量由现有物品位置和数量重算。
- contract-v19 从 v18 迁移 55 个 exact fixtures，并新增整堆超重拒绝场景，共 56 个。

## 后续

Stage D 的下一切片已由 [contract-v20](contract-v20-item-knowledge.md) 建立种类级 `ItemKnowledge`、aware/tried 状态与未知名称投影；身体槽位、箭袋和负重惩罚在对应系统切片继续扩展。
