# Contract v80：RFB 式定点延长射线

状态：协议 1.80 / contract-v80 active baseline；内容包 1.72.0；state hash Schema v34

## 目标

v80 在 v78 的方向射线与 v79 的锥形之上补齐 RFB `project_hook()` 的 `PROJECT_THRU` 定点语义。玩家现在可以让射线锁定可见格子或实体；射线经过所选目标后沿同一斜率继续到内容射程，仍复用现有 Mana、失败率、熟练度、冷却、抗性、死亡、经验、掉落、任务和回放管线。

## 内容与目标模式

- `beam-damage` 不增加新的伤害字段；伤害骰、伤害类型和射程仍由既有内容定义；
- beam 能力可在 `TargetSpec.modes` 中声明 `direction`、`position` 和 `entity`，但不能声明 `self`；
- demo 的 `demo.ability.echo-lance` 继续使用 2d4 electricity、射程 6、4 Mana 和 25% 初始失败率，并把目标模式扩展为方向、格子与实体；
- `AbilityDto.targetSpec` 已足以把新目标入口投影给 Web；锁定格子时提交 position，格子上存在 actor 时优先提交稳定 entity ID。

## 延长射线路径

- 方向目标继续沿固定八向逐格推进，行为与 v78 完全一致；
- position/entity 目标必须存在、可见且位于内容射程内；路径先使用稳定整数直线离散到达所选目标，再保持同一误差累积与斜率继续推进，直到射程上限；
- 所选目标格包含在 footprint 中，目标之后的延长格可以超出当前视野，但仍受地图边界和权威地形约束；
- actor 不阻挡射线；墙体、不可行走格和边界截断路径，阻断格不进入 `affectedPositions`；
- 目标按路径近到远稳定结算，每次施法只投一次基础伤害骰，所有目标复用相同 raw damage。

该规则对应旧 RFB `project_hook()` 无条件加入 `PROJECT_THRU`、并在 `dir == 5` 时使用实际 target 坐标的行为；新实现保留规则语义，不复制旧 C 路径代码或全局目标状态。

## 资源、失败与确定性

目标模式、实体存在性、可见性和射程在扣 Mana、施法百分位骰、熟练度与伤害骰之前验证。自身目标、缺失实体、不可见或超距位置产生 `ability.target-unavailable`，不改变资源、能力进度或 RNG。有效的定点射线即使没有命中 actor，也仍按普通成功施法消费资源并投一次基础伤害骰。

## 存档、回放与基准

定点延长只改变当前内容的目标模式和命令执行路径，不增加 save 字段。资源、已学集合与 `abilityProgress` 继续使用 save v1，state hash 仍为 Schema v34。载入 v79 及更早存档时，已学 Echo Lance 保留原有进度并从当前内容取得新增目标模式；不会自动学习能力、补发书本、重建地图或推进 RNG。

active baseline 位于 [`tests/fixtures/contract-v80/scenarios`](../tests/fixtures/contract-v80/scenarios)，共 202 个 exact fixtures、零 waiver。新增场景覆盖：

- 斜向 position 目标前后 actor 的稳定路径与共享伤害；
- entity 目标之后继续延伸并命中后方 actor；
- 目标之后的墙体截断和墙后 actor 排除；
- 超距位置的零资源、零 RNG 拒绝；
- 既有自身模式拒绝、方向射线、save round-trip 和定点实体 replay。

完整协议、内容和兼容边界见 [核心协议 v1](protocol-v1.md)、[内容数据格式 v1](content-format-v1.md)、[确定性模拟](deterministic-simulation.md) 与 [新存档格式 v1](save-format-v1.md)。
