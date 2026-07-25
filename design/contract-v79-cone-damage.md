# Contract v79：RFB 式锥形能力伤害

状态：协议 1.79 / contract-v79 active baseline；内容包 1.71.0；state hash Schema v34

## 目标

v79 在 v78 方向射线之上加入首个 RFB 式固定八向锥形效果。锥形能力复用既有能力书、Mana、失败率、熟练度、冷却、抗性、死亡、经验、掉落、任务和回放管线；它只扩展空间 footprint 与目标排序，不引入新的存档字段或随机源。

## 内容定义

`AbilityEffectDefinition` 新增 `cone-damage`：

- `damageDice`、`damageSides` 与 `damageType` 声明一次基础伤害骰；
- `radius` 限定为 1–9；
- 首版只允许 `TargetSpec.modes = ["direction"]`，射程为 1–64 且要求 line of effect；
- DTO 通过可选 `coneRadius` 投影形状；
- demo 新增 `demo.ability.echo-fan`：2d4 electricity、射程 6、半径 2、6 Mana、25% 初始失败率，并收入 Echo Primer。

## 锥形几何

- 锥形从玩家相邻格开始，沿固定八向中心线逐层推进；
- 首层中心线宽度为 0，向末端线性展开到配置的 `radius`；整数取整保证相同 seed 下所有方向稳定；
- 候选格必须位于当前层的 Chebyshev 距离、前向半平面和锥区内，并通过从玩家到该格的 line of effect；
- actor 不阻挡中心线或侧向格；墙体、不可行走格、地图边界会截断中心线及其后续层，阻断格不进入 footprint；
- `affectedPositions` 按近到远、横向距离、`y`、`x` 排序；目标按同一顺序结算；
- 每次施法只投一次基础伤害骰。中心线目标使用原始伤害，侧向目标复用既有 `rfb_area_damage` 整数衰减；同层边缘因此伤害更低但顺序仍可观察；
- 玩家自身不作为目标，不改变物品、地形或玩家伤害。

## 资源、失败与确定性

目标模式、方向和内容射程在扣 Mana、施法百分位骰、熟练度和伤害骰前验证。位置、实体或自身目标提交给锥形能力会产生 `ability.target-unavailable`，不改变资源、能力进度或 RNG。有效但被墙截断、超出边界或没有 actor 的空锥仍消耗资源并投一次基础伤害骰，产生 `targetCount = 0` 的成功 cone outcome。

事件顺序为普通 `ability.cast` 成功事件、`ability.cone-damage` footprint 事件，随后按稳定顺序产生既有 `ability.hit`/`ability.slay` 事件。每个目标继续独立经过抗性、死亡、经验、掉落和任务结算。

## 存档、回放与基准

锥形半径、伤害参数和目标模式来自当前内容；资源、已学集合、熟练度、统计和冷却继续使用 save v1 / `abilityProgress`，因此 state hash 仍为 Schema v34。载入 v78 及更早存档时不自动学习 Echo Fan、不补发书本、不重建地图、不推进 RNG。锥形 footprint、阻断格、目标顺序和事件只存在于命令执行结果与回放中。

active baseline 位于 [`tests/fixtures/contract-v79/scenarios`](../tests/fixtures/contract-v79/scenarios)，共 198 个 exact fixtures、零 waiver。新增场景覆盖：

- 随深度展开的半径与中心/边缘整数衰减；
- 墙体截断以及墙后目标排除；
- 位置模式提交到方向锥形的零资源/零 RNG 拒绝；
- 无目标截断空锥的资源、单次伤害骰与 save round-trip；
- replay 中的 Echo Fan study/cast，以及核心八个方向的几何对称性。

完整协议和内容变更见 [核心协议 v1](protocol-v1.md)、[内容数据格式 v1](content-format-v1.md) 与 [新存档格式 v1](save-format-v1.md)。
