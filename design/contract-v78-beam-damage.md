# Contract v78：RFB 式方向射线伤害

状态：协议 1.78 / contract-v78 active baseline；内容包 1.70.0；state hash Schema v34

## 目标

v78 在 v77 范围爆发之上加入第一个 RFB 原版 `fire_beam()` 形状：方向型射线穿过生物，直到射程、地图边界或不可投射地形截断。它复用现有能力学习、Mana、失败率、熟练度、冷却、目标验证、抗性、死亡、经验、掉落和任务管线。

## 内容定义

`AbilityEffectDefinition` 新增 `beam-damage`：

- `damageDice`、`damageSides` 和 `damageType` 与普通伤害能力相同；
- 首版只允许 `TargetSpec.modes = ["direction"]`，射程为 1–64 且要求 line of effect；
- DTO 通过可选 `beamDamage: true` 投影形状；
- demo 新增 `demo.ability.echo-lance`：2d4 electricity、射程 6、4 Mana，收入 Echo Primer。

## RFB 射线语义

- 射线从玩家相邻格开始，逐格沿固定八向方向推进；
- actor 不会使射线停止；路径上的每个存活 actor 都按路径由近到远结算；
- 墙体、不可行走格、越界格截断射线；阻断格不进入 `affectedPositions`，但作为 `trace.impact` 保留；
- 每次射线只投一次基础伤害骰，所有目标使用相同的原始伤害值，不做范围距离衰减；
- 玩家自身不作为目标，当前版本不改变物品、地形或玩家伤害；
- `ProjectileTrace.traversed` 是射线 footprint，事件中的 `ability.beam-damage` 同时输出稳定目标数量和 footprint。

## 资源、失败与确定性

目标模式、方向和内容射程在扣 Mana、施法百分位骰、熟练度和伤害骰前验证。用位置/实体模式提交给方向型射线会产生 `ability.target-unavailable`，不改变资源、能力进度或 RNG。有效但撞墙或射程内没有 actor 的空射仍消耗资源、投一次基础伤害骰并产生 `targetCount = 0` 的成功 beam outcome。

事件顺序为普通 `ability.cast` 成功事件、`ability.beam-damage` 形状事件，随后按路径顺序产生既有 `ability.hit`/`ability.slay` 事件。每个目标继续独立经过抗性、死亡、经验、掉落和任务结算。

## 存档、回放与基准

射线形状和伤害参数来自内容定义；资源、已学集合、熟练度、统计和冷却沿用 save v1 / `abilityProgress`，因此 state hash 仍为 Schema v34。载入 v77 及更早存档时不自动学习 Echo Lance、不补发书本、不重建地图、不推进 RNG。

active baseline 位于 [`tests/fixtures/contract-v78/scenarios`](../tests/fixtures/contract-v78/scenarios)，共 194 个 exact fixtures、零 waiver。新增场景覆盖：

- 穿透多个 actor、单次基础伤害骰与近到远顺序；
- 墙体截断与墙后 actor 排除；
- 位置模式提交到方向射线的零资源/零 RNG 拒绝；
- 空射的资源扣除、单次伤害骰、footprint 与 save round-trip；
- replay 中的 Echo Lance study/cast。

完整协议和内容变更见 [核心协议 v1](protocol-v1.md)、[内容数据格式 v1](content-format-v1.md) 与 [新存档格式 v1](save-format-v1.md)。
