# Contract v77：RFB 式范围爆发伤害

状态：协议 1.77 / contract-v77 active baseline；内容包 1.69.0；state hash Schema v34

## 目标

v77 在 v76 的资源、能力书、熟练度、冷却、学习容量与遗忘之上，加入第一个完整的范围能力效果纵切。实现参考固定的 RFB 原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 的 `fire_ball()`/`project()` 语义，但只复用当前重构已经存在的目标、伤害、抗性、击杀、经验、掉落和任务管线。

## 内容定义

`AbilityEffectDefinition` 新增 `area-damage`：

- `damageDice`、`damageSides` 和 `damageType` 与普通伤害能力相同；
- `radius` 为 1–9 的正整数；
- 能力 DTO 通过可选 `areaRadius` 投影半径；
- demo 新增 `demo.ability.echo-burst`：2d4 electricity、半径 2、射程 6、5 Mana，收入 Echo Primer。

范围效果仍要求非自身目标、有效射程和视线；效果内容不写入 save，能力进度继续使用 v75/v76 的 `abilityProgress`。

## RFB 投射语义

- 格子/实体定点目标移除 `PROJECT_STOP`：投射轨迹穿过中途怪物，在最后可达格作为爆发中心；
- 方向目标保留 `PROJECT_STOP`：首个可达怪物成为停止点；
- 墙体或越界格截断投射，爆发传播使用逐格视线/可通行性证明；墙后的格子不在爆发 footprint；
- footprint 按 RFB `distance()` 的整数 Newton–Raphson 近似分层，按距离从近到远、同层按 `y,x` 稳定排序；
- 每次爆发只投一次基础伤害骰。距离 `d` 的最终原始伤害为：

```text
(baseDamage + d) / (d + 1)
```

每个受影响 actor 继续独立经过抗性、伤害、死亡、经验、掉落和任务结算。玩家自身不作为能力伤害目标，符合原版玩家发射爆发的 `project_p` 自伤排除边界。当前 v77 不改变物品或地形效果。

## 资源、失败与确定性

目标路径在扣 Mana、施法百分位骰、熟练度和伤害骰之前验证。无效目标产生 `ability.target-unavailable`，不消耗资源、不推进能力进度和 RNG。有效但无 actor 的空爆仍是成功施法：正常扣资源、投一次基础伤害骰并产生 `ability.area-damage`，`targetCount` 为 0。

成功范围施法先产生普通 `ability.cast` 成功事件，再产生带 `AbilityAreaDamageResolutionDto` 的 `ability.area-damage`，随后按稳定 footprint 顺序产生各目标的既有 `ability.hit`/`ability.slay` 事件。事件 trace 保留投射起点、落点和逐格路径。

## 存档、回放与基准

范围半径是内容定义，能力熟练度、统计、冷却、资源和已学集合沿用已有 save v1 字段；因此 state hash 仍为 Schema v34，未新增 save 字段。协议版本升至 1.77，内容包升至 1.69.0，新的 content hash 为：

```text
acecaf504ebc3affaf67fbd8400016d85a8f4fd6b70fb7de3f1626887e5c6d62
```

v76 hash 保留在内置历史兼容白名单。active baseline 位于 [`tests/fixtures/contract-v77/scenarios`](../tests/fixtures/contract-v77/scenarios)，共 190 个 exact fixtures、零 waiver；新增场景覆盖：

- 定点目标穿过中途 actor、中心/边缘距离衰减与顺序；
- 墙体遮挡与墙后 actor 排除；
- 无效目标的零 Mana/零 RNG 边界；
- 零目标成功爆发的资源扣除、单次伤害投骰与 save round-trip；
- replay 中的 Echo Burst study/cast 及确定性终态。

## 明确不在 v77

- 锥形、射线、召唤、位移、侦测和地形改变；
- 范围内物品破坏、地形变更或玩家伤害；
- 多资源职业和怪物能力选择/施法 AI；
- 原版完整法术书、法术顺序和按等级自动遗忘/记起模型。
