# Contract v11：结构化伤害事件、派生属性与检定底座

状态：active contract baseline；阶段 B 收尾

## 1. 目标

contract-v10 已能正确结算物理、元素、状态和抗性伤害，但规则事件只向协议投影最终伤害字符串。contract-v11 保留完整 `DamageOutcome`，让本地化、动画、诊断、统计和未来知识发现共享同一份权威结算结果，不再从消息文本或最终 HP 反推；随后以内容驱动的怪物近战补齐酸、电、冷、毒的实际入口。

本 contract 不增加新的伤害类型枚举、角色属性或检定规则，也不改变存档、RNG、回合顺序和 state hash Schema。派生属性与检定接口先承接现有行为，后续状态规则只能通过这些公共入口修改能力或限制行动。

## 2. 结构化 outcome

伤害和死亡事件可携带 `GameEventOutcomeDto`：

```text
DamageResolutionDto {
  rawDamage,
  armorReduction,
  resistanceAdjustment,
  finalDamage,
  damageType,
  resistance
}
```

- `rawDamage`：进入物理护甲前的权威伤害；
- `armorReduction`：物理 AC 阶段阻止的伤害；
- `resistanceAdjustment`：正数表示抗性阻止的伤害，负数表示弱点增加的伤害；
- `finalDamage`：实际扣除的 HP；
- `damageType`：物理、酸、电、火、冷或毒；
- `resistance`：本次结算使用的弱点、普通、抗性、强抗或免疫等级。

非伤害事件不输出 `outcome`。伤害事件使用 `type: damage`；由同一次伤害触发的击杀或玩家死亡事件使用 `type: death` 并携带同一份 resolution。现有 `kind`、`messageKey` 和字符串 `args` 继续保留，供旧显示层和调试工具兼容读取。

## 3. 玩家可见投影

前端直接消费类型化 outcome：

- 普通结算继续显示最终伤害；
- 抗性显示伤害类型、最终伤害和阻止量；
- 弱点显示额外增加量；
- 免疫显示明确的免疫消息。

这些字段只描述已经发生且可观察的单次结算，不把未参与本次攻击的怪物隐藏抗性 profile 暴露给普通 UI。未来知识系统可以订阅实际结算事件形成 `KnowledgeDiscovery`，但不能直接读取完整真实抗性表。

## 4. 版本与基准

- 协议：`1.11`；
- active baseline：`contract-v11`；
- exact fixtures：47；
- 内容包：`1.7.0`；
- state hash Schema：继续为 v9；
- contract-v1 至 v10 原样保留为历史基准。

contract-v11 从 v10 迁移全部 39 个场景，并新增：

- 火焰近战遇到弱点，验证负 `resistanceAdjustment`；
- 火焰近战遇到强抗，验证稳定整数减伤；
- 腐蚀渗滴、风暴火花、霜息和毒孢的近战分别验证酸、电、冷、毒的内容定义、抗性结算与结构化 outcome；
- 眩晕将近战能力削减至零并令攻击失败；
- 恐惧检定失败会消耗行动、保留位置并产生明确事件。

原有普通、抗性、免疫、毒、流血和死亡场景现在同时断言类型化 damage/death outcome。

## 5. 派生属性与来源

`DerivedStatsPipeline` 已接管最大生命、攻击、防御、速度、近战能力和 AC。每个 `StatContribution` 记录：

- `sourceId`：实际提供 modifier 的角色定义、装备实例或状态 kind；
- `originId`：状态的施加来源，例如物品或攻击者实例；
- `layer`：基础、种族、职业、性格、装备、状态、姿态或环境；
- `priority` 与 `amount`：稳定应用顺序和整数变化量。

管线先按优先级、层、来源 ID 和 origin 排序，使用饱和整数加法，最后执行该属性的边界钳制。装备 modifier、加速和减速已经迁移到该接口；协议中的现有最终值保持不变。

## 6. 通用检定

`CheckContext` 记录检定种类、行动者、目标、带来源的 ability 与 difficulty；`CheckResult` 记录自动成功/失败或普通成功/失败、百分骰、可选对抗骰和最终阈值。玩家与怪物近战命中及恐惧行动限制统一走该接口；没有恐惧时，既有近战 RNG 抽取顺序保持不变。

## 7. 眩晕与恐惧

- `rfb.status.stun` 每级通过状态层贡献降低 10 点近战能力，最终值不低于 0；来源同时保留状态 kind 和施加者实例 ID。
- `rfb.status.fear` 只限制主动近战，不阻止等待、移动到空地或物品操作。攻击前以近战能力对抗每级 40 点行动难度；失败仍消耗正常行动成本并推进状态衰减，但不会移动或伤害目标。
- 恐惧阻止事件使用 `status-fear-blocked`，前端通过 Fluent 输出中英文消息。

## 8. 后续

contract-v12 建立武器 `AttackProfile`、玩家攻击次数和怪物多 blow；远程与投掷由后续 projectile contract 承接。
