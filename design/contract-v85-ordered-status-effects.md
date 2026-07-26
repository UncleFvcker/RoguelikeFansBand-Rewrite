# Contract v85：状态能力与有序多效果

状态：历史 baseline；当前 active baseline 已迁移到 [contract-v89](contract-v89-friendly-summon-commands.md)。协议版本为 1.85，demo 内容包版本为 1.77.0，content hash 为 `d056b65f8e2c61615e48badd8a6f02cd725007789535aa363448c8a0e8288bea`。save 容器继续使用 v1；actor statuses 原本已经进入 save 与 state hash，因此 state hash 继续使用 Schema v36。

## 1. 原版参照

FrogComposband/RFB 的主动能力经常在一次成功施法中按源码顺序调用多个效果：例如 `spells_c.c` 的复合增益依次调用 `set_hero()`、`set_blessed()`、`set_fast()`、`set_protevil()` 和恐惧清除；混乱光类法术也依次结算 slow、stun、confuse、fear 与 stasis。单体减速等目标状态则继续走 `project_hook()` 投影入口。v85 保留这种“先通过一次施法检定，再按声明顺序结算效果”的形式，但只引入当前核心已经拥有的状态、伤害和治疗原语。

## 2. 内容模型

`AbilityEffectDefinition` 新增：

- `apply-status`：声明 `statusKindId`、强度、持续 tick、`replace` / `extend` / `keep-strongest` 堆叠规则，以及可选的抗性类型；
- `remove-status`：按稳定状态 ID 移除状态；
- `sequence`：包含 2–8 个有序效果，不允许嵌套。

旧内容继续使用单个 `effect` 对象，不需要迁移。首版 sequence 只允许同一 actor 目标上的组合：

- `self`：heal、apply-status、remove-status；
- direction/position/entity：damage、apply-status、remove-status。

范围、射线、锥形、召唤、侦测、位移和地形改变暂不嵌入 sequence，避免一次里程碑同时改变多目标与世界状态事务边界。

demo 新增：

- Echo Quickening：先添加 haste，再移除 slow；
- Echo Binding：先造成 cold damage；目标存活时再添加受 cold 抗性影响的 slow。

## 3. 结算顺序与抗性

目标模式、射程、实体身份和 line of effect 在支付 Mana、失败率 RNG 与熟练度之前验证。通过前置验证后：

1. 支付一次能力资源；
2. 抽取一次整次施法失败率；
3. 失败时不执行任何子效果；
4. 成功时严格按内容数组顺序执行，每个伤害骰只在轮到该效果且目标仍存活时抽取；
5. 某一效果无效不会回滚前序效果，也不会阻止仍然有合法目标的后续效果；
6. 前序效果击杀目标后，剩余效果记录为 `target-dead`，不再抽取其 RNG；
7. 投影没有命中 actor 时，各效果记录为 `no-target`，伤害骰不会抽取。

带 `resistanceType` 的状态持续时间确定性缩放：

| 抗性 | 持续时间 |
| --- | --- |
| vulnerable | 150% |
| normal | 100% |
| resistant | 50% |
| strong | 35% |
| immune | 0，状态结果为 immune |

非免疫且缩放后不足 1 tick 时保留 1 tick。状态来源保存为能力 ID；重复状态继续复用已有 replace、extend 与 keep-strongest 规则，状态列表按 kind ID 稳定排序。

## 4. 协议与 Web

协议新增：

- `AbilityEffectSpecDto` 与 `AbilityDto.effects`，所有能力都投影为扁平有序效果列表；
- `AbilityStatusStackingDto`；
- `AbilityEffectResolutionDto`、`AbilityEffectsResolutionDto`；
- `AbilityStatusChangeDto` 与 `AbilityEffectSkipReasonDto`；
- `GameEventOutcomeDto.ability-effects`。

`ability.effects` 事件返回目标身份和逐效果结果；每项包含稳定 `effectIndex`。Web 能力列表标出多效果数量，并格式化组合事件；旧的单效果专用字段与事件继续保留。

## 5. 存档、回放和哈希

能力定义由 content hash 固定。运行时状态继续写入既有 `ActorSaveDto.statuses` / `StatusSaveDto`；状态、来源、强度和剩余 tick 原本已经进入 state hash，因此不新增 save 字段或 state hash Schema。旧 built-in content hash 迁移到 1.77.0 时不会自动学习新能力、补发书本、添加状态或推进 RNG。

回放记录仍只保存命令与确定性终态；新增 replay 覆盖 Echo Quickening 的添加/移除顺序及终态哈希。

## 6. contract-v85

该历史 baseline 位于 [`tests/fixtures/contract-v85/scenarios`](../tests/fixtures/contract-v85/scenarios)，共 242 个 exact fixtures、零 waiver。新增 232–242 覆盖：

- 自身状态添加后清除、移除不存在状态的部分无效；
- 重复施法的 extend；
- 目标伤害后状态添加；
- resistant 持续时间缩短与 immune；
- 前序击杀后的 `target-dead`；
- 无目标 `no-target`；
- 非法目标、失败率与资源不足的 Mana/RNG/熟练度边界；
- save round-trip 与结构化逐效果 outcome。

## 7. 下一步

P26 已由 [contract-v86](contract-v86-monster-casting-ai.md) 完成首个怪物施法与能力选择 AI；后续扩展自身增益、多目标法术、召唤、友军风险与状态驱动效用。
