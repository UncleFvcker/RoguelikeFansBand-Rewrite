# Contract v86：首个怪物施法与能力选择 AI

状态：历史 baseline。协议版本为 1.86，demo 内容包版本为 1.78.0，content hash 为 `be6b9b098c495ee3f2af6075ea5790d16eae7e8487c1fa310575c0dad8cba5bd`。save 容器继续使用 v1；怪物施法冷却新增权威 actor 状态，因此 state hash 升至 Schema v37。当前 active baseline 见 [Contract v88](contract-v88-monster-targets-tactics-memory.md)。

## 1. 原版参照

FrogComposband/RFB 的怪物行动先按种族施法频率做百分位检定；检定通过后，`monspell.c` 再过滤当前可用法术并按各法术概率选择。若未施法，怪物继续普通移动或近战。v86 保留这三个层次：

1. 施法频率决定本次是否尝试施法；
2. 射程、投影和友军阻挡决定候选是否可用；
3. 内容权重决定可用候选中的选择。

底层保存百分比而不是 `1 in N` 分母：`frequencyPercent: 50` 等价于 1 in 2，25 等价于 1 in 4，也允许 30% 等不能写成整数分母的频率。

## 2. 内容模型

Monster actor 可声明：

```json
{
  "monsterCasting": {
    "frequencyPercent": 50,
    "abilities": [
      { "abilityId": "demo.ability.resonant-bolt", "weight": 3 },
      { "abilityId": "demo.ability.echo-binding", "weight": 1 }
    ]
  }
}
```

编译器要求频率为 1–100、候选为 1–32 个稳定且不重复的能力 ID、权重为正数，并拒绝玩家 actor、缺失引用以及首版怪物执行器不支持的效果。首版复用直接 actor 目标的 `damage`、`apply-status`、`remove-status` 和有序 `sequence`；玩家专属 Mana、学习、熟练度、施法失败率与能力冷却字段不会被怪物消费。

demo 新增 Echo Cantor。它以 50% 频率在 Resonant Bolt（权重 3）和 Echo Binding（权重 1）之间选择。

## 3. 行动、选择与 RNG

已警戒且不处于施法冷却的怪物按以下顺序行动：

1. 固定抽取一次 1–100 频率骰；
2. 纯计算过滤超距、被墙阻挡或会穿过其他存活怪物的直接法术；
3. 频率通过且总可用权重大于零时，再抽取一次 1–总权重的选择骰；
4. 选中能力后占用整个怪物行动，并严格按声明顺序抽取效果所需 RNG；
5. 频率失败或没有可用候选时，不抽权重或效果 RNG，继续普通近战/移动。

候选与权重按内容声明顺序解释，不依赖哈希表顺序。直接敌对投影使用原版式 clean-shot 边界：墙体或路径上的另一存活怪物都会使该候选不可用。前序伤害击杀玩家后，后续效果记录 `target-dead` 且不抽取 RNG。

## 4. 怪物施法冷却

成功施法后，怪物获得按自身行动计数的冷却：

```text
cooldown = ceil(100 / frequencyPercent)
```

因此 50% 增加 2 次怪物行动冷却，25% 增加 4 次，30% 向上取整为 4 次。冷却中的怪物每获得一次自身行动便把剩余值减一，不做频率或权重检定，也不抽取施法 RNG；它仍可继续近战或移动。冷却归零后的下一次怪物行动重新进入正常施法频率检定。

## 5. 协议、存档与 Web

协议新增：

- `MonsterAbilityDecisionResolutionDto` 和 `monster.ability-decision`，暴露频率、频率骰、稳定可用候选、总权重、选择骰与最终能力；
- `MonsterAbilityCastResolutionDto` 和 `monster.ability-cast`，暴露施法者、目标、能力 ID 与逐效果结果；
- `ActorSaveDto.castingCooldownRemaining` 和 `EntityDto.castingCooldownRemaining`。

Web 使用本地化消息展示怪物的施法选择与结算。怪物冷却随当前或离层 actor 保存并参与 state hash；旧存档缺字段时迁移为零，不自动施法、不推进 RNG。回放继续只记录玩家命令，怪物频率、选择、伤害、状态和冷却都由确定性调度重演。

## 6. contract-v86

该历史 baseline 位于 [`tests/fixtures/contract-v86/scenarios`](../tests/fixtures/contract-v86/scenarios)，共 249 个 exact fixtures、零 waiver。新增 243–249 覆盖：

- 3:1 权重下的 Resonant Bolt 与 Echo Binding 两个确定性分支；
- 频率骰失败后继续普通行动；
- 50% 施法后的两次自身行动冷却及冷却期零施法 RNG；
- 盟友与墙体 clean-shot 阻挡；
- Echo Binding 前序击杀后的 `target-dead` 短路；
- 怪物状态、冷却、save round-trip 与 replay 终态哈希。

## 7. 下一步

P27 候选为怪物施法效用与目标扩展：在现有频率、可用性和权重层上增加自身增益、范围/锥形/射线、召唤、友军风险与基于当前状态的效用过滤；首版 P26 不伪装已经拥有这些智能规则。多资源职业继续后置。
