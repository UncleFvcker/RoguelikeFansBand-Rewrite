# Contract v52：程序化特殊地形表与空间预算

## 目标

contract-v52 将额外陷阱、门和可挖掘障碍纳入独立内容表与楼层空间预算。候选按深度和权重选择，只能落在声明的房间或走廊空间；没有合法位置的候选稳定回退，所有成功位置同时排斥后续 terrain、actor 和 loot 重叠。

## 内容契约

- 新增独立 `TerrainFeatureTableDefinition` 根。表声明 1–8 次 `rolls`，条目包含 `terrainId`、`placement`、权重和深度范围。
- `placement: room` 只允许 trap 或具有 dig 转换的 terrain；`placement: corridor` 只允许具有 open 转换的门 terrain。权重必须非零，深度为 1–1000，条目键规范化且不可重复。
- 程序化楼层以 `terrainFeatureTableId` 引用表，并在既有 `generationBudget` 中成对声明 `featurePlacements`。预算范围为 1–8，且不能超过表的 `rolls`；引用楼层必须至少有一个深度合格候选。
- `featurePlacements` 只计算额外特殊地形。用于保证基础拓扑的固定连接门和固定教学陷阱不消耗该预算。

## 确定性生成顺序

1. 先完成主题选择、双房间/走廊开凿、固定门/楼梯/陷阱和 Vault 绘制。
2. 将上下楼梯、固定连接门、固定陷阱及完整 Vault 矩形加入保留位置。
3. 每个 feature placement 槽按深度合格条目权重选择；单候选不消费抽取。room 候选只枚举房间内当前主题 floor，corridor 候选只枚举房间外的当前主题 floor，位置均按地图行优先排序。
4. 所选条目没有合法位置时，仅从当前槽的候选池移除并继续加权回退，不消耗 placement；候选耗尽时停止剩余槽位。
5. 多个位置只消费一次有界抽取。成功后立即写入 terrain，并把位置加入 actor/loot 占位集合，后续槽位不能重叠。

生成后的 trap、门和 rubble 继续使用既有 terrain 交互、发现知识、存档和 state hash，不新增运行时状态字段。

## 原创场景与验证

- 新增 `demo.terrain-feature-table.resonance-hazards`：深度 3 起提供 room trap/rubble，深度 4–7 提供 corridor locked door，深度 6–10 提供 corridor secret door。
- 共鸣压力地牢深度 3 预算 2 个额外 feature，深度 4 预算 3 个，深度 5–10 预算 4 个；这些预算可与动态群体和空间 Vault 同层组合。
- `contract-v52` 从 v51 迁移 102 个 exact fixtures，新增深度 3 room feature 与深度 4 corridor door 两个场景；active baseline 共 104 个 exact fixtures，0 waiver。
- 单元测试覆盖表类型/权重/深度/引用/预算校验、深度过滤、权重可达性、主题 floor 候选、保留位置、精确预算、无重叠、空间失败回退、确定性和 v51 已生成楼层迁移。

## 版本

- 协议：1.52；
- 内容包：1.45.0；
- content hash：`1f8848e160b4ec51ca36acc512920946888fec20a36d7ac7b860bdb126aff79a`；
- save：v1，不增加字段；
- state hash：Schema v19，不增加字段；
- active baseline：contract-v52，104 exact fixtures。

## 明确延后

- 房间数量、尺寸、矩形以外形状、房间面积预算与走廊拓扑预算；
- trap/door 的分类型配额、密度曲线、相邻限制和全图连通性证明；
- pit、任意形状 formation、pack AI、召唤、繁殖与种群上限；
- Vault 多入口、大模板跨走廊拼接、分支、shaft 与独立到达点。
