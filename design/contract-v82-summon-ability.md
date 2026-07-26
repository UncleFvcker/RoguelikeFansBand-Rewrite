# Contract v82：首个召唤能力

状态：协议 1.82 / contract-v82 历史基线；内容包 1.74.0；state hash Schema v35

## 目标

v82 在既有能力书、Mana、熟练度、冷却和 actor 生成管线之上加入首个内容驱动的召唤效果。首版只证明“玩家施法 → 一次性生成多个有所有者的友方 actor → 按玩家回合存活 → 存档/回放一致”的最小闭环，不引入多资源、怪物施法 AI、繁殖或复杂宠物命令。

## 内容与校验

- `AbilityEffectDefinition` 新增 `summon`，声明 `actorKindId`、`count`、`radius` 和 `durationTurns`；
- 编译器要求召唤目标引用 `role = monster` 的 actor，数量和半径均为 1–8，生命周期为 1–10,000 回合；
- 首版目标固定为单一 `self`，射程为 0，且不要求 line of effect；
- DTO 通过 `AbilityDto.summon` 输出召唤规格；
- demo 新增 `demo.actor.echo-companion` 与 `demo.ability.echo-companion`。Echo Companion 消耗 6 Mana，基础失败率 20%，召唤 2 个伙伴，半径 2，配置生命周期 5 回合，并收入 Echo Primer。

## 落位、身份与阵营

施法成功后，核心在玩家周围的 Chebyshev 半径内收集候选格，并按距离、`y`、`x` 稳定排序。候选格必须在地图内、可行走且没有玩家、actor 或地面物品占用；空间不足时整次召唤原子拒绝，不扣 Mana、不抽施法 RNG、不增加能力熟练度，也不生成部分 actor。

每个召唤物使用稳定 ID：

```text
summon.<ability-id>.<command-seq>.<ordinal>
```

召唤物的 `faction` 为 `player`，并保存 `ownerId`、`sourceAbilityId` 与剩余回合。召唤物不参加敌对怪物 AI、不计入 `visibleHostileExists`，也不会主动攻击玩家；首版友方行为仅建立所有权与生命周期边界。召唤物不能同时拥有 pack identity。

## 资源、失败与生命周期

空间验证先于既有能力前置结算。空间足够时，能力沿统一流程扣除按熟练度派生的 Mana，并抽一次失败率百分位；失败仍消耗 Mana，但不生成 actor，能力失败计数照常更新。成功后发出 `ability.cast` 和 `ability.summon` 结构化事件。

生命周期按玩家完成的世界回合递减，而不是按内部 world tick 的每个调度步骤递减。召唤命令完成后进入该命令的首个回合结算，因此配置为 5 回合的 Echo Companion 在施法更新中显示 4 个剩余回合，并在随后四次玩家行动结算时发出 `summon.expired`、标记位置变化并加入 `removedEntities`。死亡仍沿用既有 actor/掉落事务；生命周期到期则直接移除，不生成掉落。

## 协议、存档与 state hash

协议 1.82 新增：

- `AbilitySummonSpecDto` 与 `AbilityDto.summon`；
- `AbilitySummonResolutionDto` 与 `GameEventOutcomeDto.ability-summon`；
- `EntityFactionDto`、`SummonDto` 与实体快照字段；
- `SummonSaveDto` 与 `ActorSaveDto.summon`。

召唤物的身份、阵营推导、位置、生命、状态和剩余生命周期进入 save v1 与 state hash Schema v35。没有召唤物的旧 v81 及更早存档通过缺省 `summon = null` 迁移，不自动学习 Echo Companion、不补发书本或 actor、不重建地图、不推进 RNG。载入时会校验所有者必须是当前玩家、源能力必须仍声明同一召唤目标、生命周期必须为正，并拒绝非法或与 pack 同时存在的召唤状态。

## Fixtures 与验收

历史 baseline 位于 [`tests/fixtures/contract-v82/scenarios`](../tests/fixtures/contract-v82/scenarios)，共 213 个 exact fixtures、零 waiver。新增场景覆盖：

- 两个召唤物的稳定 ID、排序落位、阵营/所有者和 save round-trip；
- 半径内空间不足时的原子回退及 Mana/RNG/熟练度不变；
- 玩家回合生命周期递减、到期事件和 `removedEntities`；
- 失败率失败时的 Mana 消耗、失败统计和无 actor 生成。

完整兼容边界见 [核心协议 v1](protocol-v1.md)、[内容数据格式 v1](content-format-v1.md)、[确定性模拟](deterministic-simulation.md) 与 [新存档格式 v1](save-format-v1.md)。

后续 contract-v83 已在同一公共效果管线上实现首个侦测能力；多资源、地形改变与怪物施法继续后置，友方命令、召唤物 AI 和持续效果组合仍待扩展。
