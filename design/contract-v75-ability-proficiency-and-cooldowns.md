# Contract v75：能力熟练度与冷却

状态：协议 1.75 / contract-v75 active baseline；内容包 1.67.0；state hash Schema v34

## 范围

v75 在 v74 的 Mana、能力书、学习、失败率、等待/休息恢复和自身治疗之上，加入第一版持久能力进度。每个可由当前 Class casting profile 使用的能力都有独立的熟练度、上限、成功/失败统计和冷却剩余；这些字段属于权威游戏状态，进入 save、snapshot、回放和 state hash。

实现参考 RFB 原版的法术经验档位：

| 熟练度 | 等级 | 规则影响 |
| ---: | --- | --- |
| 0–899 | Unskilled | Mana 成本最高；无额外失败率修正 |
| 900–1199 | Beginner | 成本开始下降 |
| 1200–1399 | Skilled | 成本继续下降 |
| 1400–1599 | Expert | 成本达到低档，并降低 1 个百分点失败率 |
| 1600 及以上 | Master | 达到最低成本，并再降低 1 个百分点失败率 |

熟练度由内容定义的 `initial`、`cap`、`successGain` 和 `failureGain` 控制，运行时永远按能力自身上限截断；demo 两个能力均使用 `0/1600/128/0`。成功施法增加 `castCount` 与成功熟练度，失败检定增加 `failCount` 与失败熟练度。前置拒绝、资源不足和冷却拒绝不增加统计、不增加熟练度，也不抽失败率 RNG。

Mana 成本使用整数化的原版方向公式：

```text
ceil(baseCost × (3800 - min(proficiency, 1600)) / 2400)
```

因此 Resonant Bolt 的成本依次为 5、4、4、3、3；Mending Echo 依次为 7、6、6、5、5。失败率在 Expert 和 Master 档分别额外降低 1 个百分点，仍受 Class 的基础失败率和最低失败率边界约束。

## 冷却

能力可以声明可选 `cooldown.turns`，并可声明 `cooldown.groupId`。没有该字段的普通 RFB 风格能力保持零冷却；冷却是原创扩展入口，不会强行改变所有法术的节奏。成功施法后写入剩余回合；有组 ID 的能力共享同组最大剩余值，独立能力只更新自身。普通行动开始时按实际推进的世界回合递减，`Rest` 按实际完成回合递减。冷却拒绝发生在扣资源和失败率检定之前，因此资源与 RNG 均保持不变。

demo 的 Mending Echo 声明 2 回合冷却和 `demo.cooldown.mending` 组；Resonant Bolt 保持无冷却，用于对照普通法术路径。

## 协议、存档与回放

`AbilityDto` 输出基础/实际资源成本、熟练度、熟练等级、成功/失败次数和冷却字段；`AbilityCastResolutionDto` 输出施法前后对应值。`PlayerSaveDto.abilityProgress` 保存能力进度，`state hash Schema v34` 覆盖能力进度；save 容器仍为 v1。

载入旧 v73/v74 存档时，如果缺少 `abilityProgress`，运行时根据当前内容建立初值（熟练度 initial、统计为零、冷却为零），再恢复已有资源和已学能力。该迁移不补抽 RNG、不自动学习能力，也不改变已保存的资源值。重复 ID、能力上限不匹配、熟练度越界或冷却超出内容声明都会原子拒绝。

## 内容与基线

demo 内容包升至 1.67.0，content hash 为
`bcc23bf5834c37bf7fb0874bcb1dfc72c751efad36f76d94b07391100e976316`。旧 v74 hash
`9f61f6161b77c553fc9dfed8d2e550abca8794d1dc997fb2af3f953feb711cb0` 保留在内置迁移白名单。

active baseline 位于 [`tests/fixtures/contract-v75/scenarios`](../tests/fixtures/contract-v75/scenarios)，共有 182 个 exact fixtures、零 waiver。v74 的 174 个场景全部刷新到 Schema v34，并新增：

- 五档熟练度/成本和 Master 失败率边界；
- 成功增加熟练度与施法次数；
- 失败增加失败次数但不增加 demo 默认熟练度；
- 冷却拒绝的零资源/零 RNG 保证与等待恢复；
- 能力进度 save round-trip；
- 缺少 `abilityProgress` 的旧存档迁移。

contract-only 的 `debugClearEntities` 仍可用于采集无敌/无敌人场景；它不是正式协议或玩家运行时选项。

## 明确不在 v75 的范围

- 学习容量、随机学习、遗忘和首次成功奖励；
- 怒气、专注、鲜血等职业资源与资源互转；
- 范围、锥形、位移、召唤、侦测、地形改变和多效果组合；
- 怪物施法、能力选择 AI、完整领域/职业矩阵；
- 装备激活冷却与完整物品主动能力。
