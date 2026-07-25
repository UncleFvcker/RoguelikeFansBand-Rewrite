# Contract v81：首个短距位移能力

状态：协议 1.81 / contract-v81 active baseline；内容包 1.73.0；state hash Schema v34

## 目标

v81 在既有目标、资源、熟练度、冷却和调度公共管线之上加入首个内容驱动的精确短距位移。首版选择可见格到可见格的 `Echo Step`，不引入随机偏移、随机落点或额外的“传送误差”骰；能力成功后精确落在玩家提交的目标格。

## 内容与目标模式

- `AbilityEffectDefinition` 新增 `teleport`，该效果不携带伤害或额外存档参数；
- 编译器强制 teleport 只能使用单一 `position` 目标，射程为 1–64，并要求 `requiresLineOfEffect`；
- `AbilityDto.teleport` 向 Web 投影该效果；目标范围仍由既有 `targetSpec` 提供；
- demo 新增 `demo.ability.echo-step`（Echo Step / 回声步）：6 格射程、4 Mana、25% 初始失败率，收入 Echo Primer，熟练度沿用既有五档能力规则。

## 落点验证与移动

目标验证在扣 Mana、施法失败率骰、熟练度统计和冷却结算之前完成。目标格必须同时满足：

- 不是玩家当前格；
- 位于当前地图内，且 Chebyshev 距离不超过内容射程；
- 当前可见，并满足能力声明的 line of effect；
- 地形可行走；
- 没有存活 actor 占据。

任一条件失败都会产生既有 `ability.target-unavailable` 拒绝：不扣 Mana、不推进施法 RNG、不改变能力进度；命令仍按普通失败命令推进调度。有效目标即使未产生其他效果，也按正常施法流程处理失败率、资源、熟练度与冷却。

成功施法先产生普通 `ability.cast` 成功事件，再产生 `ability.teleport`/`AbilityTeleportResolutionDto`，其中明确记录 `from` 与 `to`。随后复用普通移动的统一到达管线：更新玩家位置、被动感知、陷阱触发和死亡处理；不额外投掷“落点安全”或“误传送”随机数。

## 存档、回放与基准

位移目标、路径和事件只存在于命令结果与回放中，不增加 save 字段；资源、已学能力、熟练度、统计和冷却继续使用 save v1 的既有字段，state hash 仍为 Schema v34。载入 v80 及更早存档时不会自动学习 Echo Step、补发 Echo Primer、重建地图或推进 RNG；legacy study-save 只按当前书本和能力定义执行显式学习。

active baseline 位于 [`tests/fixtures/contract-v81/scenarios`](../tests/fixtures/contract-v81/scenarios)，共 209 个 exact fixtures、零 waiver。新增场景覆盖：

- 精确传送与 save round-trip；
- 不可行走、line-of-effect 被阻断、占用、当前格、超距和错误目标模式的零资源/零 RNG 拒绝；
- 传送到陷阱格后复用普通移动的到达处理；
- 失败率施法仍按既有资源支付规则运行；
- legacy ability study-save 兼容与 replay 中的 Echo Step 学习/施放。

完整兼容边界见 [核心协议 v1](protocol-v1.md)、[内容数据格式 v1](content-format-v1.md)、[确定性模拟](deterministic-simulation.md) 与 [新存档格式 v1](save-format-v1.md)。
