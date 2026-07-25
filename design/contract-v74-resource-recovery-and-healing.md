# Contract v74：法术资源恢复与自身治疗

状态：协议 1.74 / contract-v74 active baseline；内容包 1.66.0；state hash Schema v33

## 范围

v74 在 v73 的 Mana、能力书、学习、失败率和目标施法闭环之上，补齐第一版资源恢复、可中断休息与非伤害能力。玩家可以用普通等待恢复少量 Mana，也可以发出一个确定性休息宏命令连续推进真实世界调度；第二本原创能力书则验证稳定自身目标与固定治疗效果。

本纵切仍只建立公共规则边界，不同时实现学习容量、熟练度、冷却、多种职业资源、怪物施法或旧 RFB 的完整法术领域。

## 内容模型

`ResourceDefinition` 新增两个带默认值的字段：

- `waitRecoveryAmount`：一次普通 `Wait` 完成后恢复的数量；
- `restRecoveryAmount`：一次安全休息回合完成后恢复的数量。

缺少字段的旧内容按 0 处理。恢复量是非负整数；恢复使用饱和加法并限制在资源上限，不会溢出或超过最大值。

`TargetMode` 新增稳定 `self`，JSON 序列化为 `{ "type": "self" }`。自身目标必须使用零射程并关闭视线要求。`AbilityEffectDefinition` 新增 `heal { amount }`；治疗量必须为正数，执行时复用既有 `HealResolutionDto`，实际生命不会超过最大生命。

demo 内容包升至 1.66.0，并新增或调整：

- `demo.resource.mana`：等待恢复 1，休息恢复 3；
- `demo.ability.mending-echo`：等级 1、消耗 4 Mana、基础失败率 30%、自身目标、固定治疗 6；
- `demo.ability-book.stillwater-notes`；
- `demo.item.stillwater-notes`，作为 Scholar 的第二本出生能力书；
- Mage 同时支持 Echo Primer 与 Stillwater Notes。

初始 Scholar 的 Intelligence index 为 15，因此 Mending Echo 的实际失败率为 `30 - 15 = 15%`。当前出生物品稳定分配使 Stillwater Notes 实例为 `generated.item.4`。

## 等待恢复

普通 `Wait` 仍是一个标准行动：先花费 100 能量并运行怪物、状态与 world tick 调度，玩家存活时再按稳定资源 ID 顺序应用 `waitRecoveryAmount`。只有实际增加的资源才产生 `resource.recovered` 事件与 `GameEventOutcomeDto.resource-recovery`；满资源或恢复量为 0 时不产生空恢复事件。

恢复本身不消费 RNG。调度器中的怪物行动、状态效果或其他既有规则仍可按原顺序消费 RNG；等待恢复不会创建旁路时间线。

## 确定性休息

协议新增：

```text
Rest { turns }
```

`turns` 的有效范围是 1–100。一次 `Rest` 是一个可回放的确定性宏命令，而不是前端循环发送多个 `Wait`：

1. 先拒绝无效回合数；
2. 再检查是否已经没有可恢复的缺损资源；
3. 再检查当前是否存在可见敌人；
4. 每个实际休息回合花费一次标准行动并运行完整调度器；
5. 调度后依次检查玩家死亡、生命下降和新出现的可见敌人；
6. 只有仍安全时才应用 `restRecoveryAmount`；
7. 资源已满则结束，否则达到请求上限时以 `turn-limit` 结束。

因此满资源检查优先于可见敌人：资源已满且敌人在场时仍返回 `full-resources`，且不推进世界。若一个回合中受到流血或怪物伤害，该回合已经真实推进，但不会获得该步恢复。

停止原因固定为：

- `invalid-turns`；
- `full-resources`；
- `enemy-visible`；
- `damaged`；
- `player-died`；
- `turn-limit`。

`full-resources` 与 `turn-limit` 投影 `rest.completed`，其余原因投影 `rest.interrupted`。`RestResolutionDto` 保存请求回合、实际完成回合、停止原因和按整次命令聚合的资源恢复结果。

一次休息命令只增加一次 revision 和命令序号；`turn` 增加实际完成回合数，但零回合结束时仍至少增加 1，以保存一次已处理命令的权威顺序。`worldTick` 只增加真实执行的调度时间，零回合结束保持不变。

## 自身治疗能力

Stillwater Notes 的学习继续复用 `StudyAbility`，不会消耗书本或 RNG。Mending Echo 的施放继续遵循 v73 顺序：通过职业、等级、学习、书本和资源前置检查后先扣 4 Mana，再掷一次失败率骰。成功时稳定自身目标直接进入治疗管线，不打开前端准星；失败仍消耗 Mana。

seed 0 的首轮示例中，百分位 roll 为 32，高于 15% 失败率，因此施放成功：生命由 5 恢复到 11，Mana 由 21 降到 17。事件同时包含 ability-cast resolution 与应用量为 6 的 heal resolution。

Web 能力面板显示每种资源的当前值、上限、等待恢复量和休息恢复量；休息按钮提交 `Rest { turns: 100 }`。包含 `self` 的能力按钮直接提交稳定自身目标，其他能力继续使用既有目标模式。

## 存档、hash 与兼容

v74 不增加正式 save 字段。资源恢复速率属于内容定义；存档继续保存资源当前值/上限与已学能力 ID。休息请求、停止原因和聚合恢复结果属于命令/事件/回放，不写成额外持久状态。

载入 v73 内容 hash 的存档时：

- 保留既有资源值、已学能力和物品；
- 不补发 Stillwater Notes；
- 不自动学习 Mending Echo；
- 不推进 RNG 或 world tick。

恢复后的资源、生命、状态、`turn`、`worldTick`、命令序号与 RNG 位置进入 state hash Schema v33。正式 save 容器仍为 v1；v73 内容 hash `fa88458239f225a5033e5910c64ba30f8e1e4095fc82b1ebce6a5c914e05ad2d` 保留在迁移白名单中。

## 确定性覆盖

active baseline 位于 [`tests/fixtures/contract-v74/scenarios`](../tests/fixtures/contract-v74/scenarios)，共有 174 个 exact fixtures、零 waiver。v73 的 166 个场景全部迁移，并新增：

| Fixture | seed | 固定行为 |
| --- | ---: | --- |
| `resource.wait-recovery` | 0 | 一次等待真实推进到 world tick 10，Mana 由 10 恢复到 11 |
| `resource.rest-to-full` | 0 | 四个休息回合把 Mana 由 10 恢复到 21，revision 只增加 1 |
| `resource.rest-full-no-time` | 0 | 满资源立即结束，`turn = 1`、`worldTick = 0` |
| `resource.rest-enemy-interrupt` | 0 | 可见敌人使休息零回合中断，Mana 与 RNG 不变 |
| `resource.rest-damage-interrupt` | 0 | 流血先造成伤害并推进一个回合，随后中断且不恢复 Mana |
| `ability.study-healing` | 0 | 以 `generated.item.4` 学习 Mending Echo 并完成回读 |
| `ability.cast-healing` | 0 | roll 32 / failure 15%，生命 5→11、Mana 21→17 |
| `resource.rest-turn-limit` | 0 | 请求两回合后以 `turn-limit` 结束，Mana 10→16 |

为减少测试采集消耗，v74 fixtures 可使用 contract-only 的 `debugClearEntities` 前置项清除实体与携带物，并同步入口守卫状态。它不属于正式协议、命令或存档，也不会出现在玩家运行时。

内容、核心、回放、协议与 Web 专项测试同时锁定恢复字段默认值、无效自身目标、休息中断顺序、零时间边界、治疗效果和保存回读。

## 明确不在 v74 的范围

- 能力熟练度、首次成功奖励、练习增长、遗忘和学习容量；
- 每能力/每书本/共享组冷却，以及冷却 UI；
- 怒气、专注、鲜血等多种职业资源与资源互转；
- 范围、锥形、位移、召唤、侦测、地形改变和多效果组合；
- 怪物施法、能力选择 AI、智能学习和完整领域/职业矩阵；
- 饥饿、HP 自然恢复、旅店式安全休息与自动探索。

下一纵切建议推进 P15“能力熟练度与冷却”：先建立稳定、可保存、可回放的每能力状态，再让熟练度参与失败率或效果边界，并验证共享/独立冷却与零 RNG 拒绝。多资源和怪物施法继续后置。
