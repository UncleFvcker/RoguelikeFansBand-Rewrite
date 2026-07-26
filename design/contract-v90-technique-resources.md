# Contract v90：多职业资源底子与首个技法资源

状态：历史 baseline；当前 active baseline 见 [contract-v91](contract-v91-monster-displacement.md)。协议版本为 1.90，demo 内容包升至 1.81.0（content hash 以 `content.lock.json` 为准）。save 容器继续使用 v1；技法资源池与先天能力熟练度进入既有权威结构，state hash 升至 Schema v40。

## 1. 原版参考与本轮边界

FrogComposband/Hengband 系除 HP/SP 外还有武士的集中度、练气士的气、吟游诗人的持续歌曲、狂怒法师的怒气、念力者的专注点等职业资源：按特定行为积累、随时间或条件衰减、独立于 Mana 的恢复策略。本轮建立可表达这一族机制的通用底子，并接入首个原创技法资源，但只提交原创内容与中性机制：

- `ResourceDefinition` 新增可选行为字段：`initialFillPercent`（初始填充百分比，缺省 100）、`meleeHitGainAmount`（玩家近战每次命中获得）、`meleeKillGainAmount`（玩家近战击杀额外获得）、`turnDecayAmount`（推进世界时间的玩家行动内该池未被触碰时行动后衰减）。Mana 不使用任何新字段，行为不变。
- `ClassDefinition` 新增可选 `techniqueProfiles` 数组。每个条目声明 `resourceId`、`governingAttribute`、`baseCapacity`/`capacityPerLevel`/`capacityPerAttributeIndex` 上限公式、`minimumFailurePercent` 与 `innateAbilityIds`。技法能力先天可用：不占学习容量、不需要书籍研读或随身携带、不可遗忘；等级下限、冷却与资源检查照常。
- 一个类可同时拥有 `castingProfile` 与多个 `techniqueProfiles`，各资源 ID 不得重复；技法能力的 `resourceId` 必须等于所属 profile 的资源。首版不包含持续吟唱型逐回合扣费、受击获得与姿态切换，后续资源按同一底子扩展。

## 2. 首个技法资源与内容

新增原创决斗家纵切：

- `demo.resource.tempo`（节奏）：`initialFillPercent` 0、近战命中 +2、近战击杀额外 +3、闲置衰减 1；`waitRecoveryAmount`/`restRecoveryAmount` 均为 0，等待与休息不恢复。
- `demo.class.duelist`（决斗家）：无 `castingProfile`；技法 profile 以敏捷为主宰属性，`baseCapacity` 8、每级 +1、每属性档 +1、最低失败率 2%，先天能力为弦月斩与涌动节奏；新建 `demo.skill-set.duelist`。
- `demo.ability.crescent-cut`（弦月斩）：方向目标、射程 1，消耗 4 节奏，基础失败率 12%，造成武器无关的固定骰伤害。
- `demo.ability.surging-tempo`（涌动节奏）：自身目标，消耗 6 节奏，基础失败率 15%，按 v85 有序效果施加 haste。
- `demo.build.duelist`：人类 + 决斗家 + 既有战斗人格。现有构筑、职业与 Mana 内容不变。

## 3. 行为、衰减与 RNG

- 获得：玩家近战每次命中后按定义增加对应池，击杀在命中之外追加；到达上限即封顶。每次实际获得发出 `resource.gained` 事件（`melee-hit`/`melee-kill` 来源）；封顶导致的零增长不发事件但仍视为“已触碰”。召唤物与怪物的近战不产生玩家资源。
- 衰减：推进世界时间的玩家行动结算后，若某池在本行动内未被获得、消费或恢复触碰，且 `turnDecayAmount` > 0 且当前值 > 0，则扣减至多该数值。衰减静默进行，不发事件、不抽 RNG。休息的每个完成回合按同一规则衰减。零世界时间命令（属性提升、召唤指令等）不触发衰减。
- 消费：技法能力复用既有 cast 管线；熟练度折减公式、冷却、失败率钳制与统计与 Mana 能力一致，失败率以所属 profile 的主宰属性与 `minimumFailurePercent` 计算。资源不足、冷却中、等级不足的拒绝路径不抽 RNG、不消费资源；被拒绝的施法回合仍推进世界时间，未被触碰的池照常衰减。
- 获得、衰减与上限计算全部不抽 RNG；命中/伤害照常走既有战斗 RNG。

## 4. 协议、存档与 Web

协议 1.90 增加：

- `ResourcePoolDto` 可选字段 `meleeHitGainAmount`、`meleeKillGainAmount`、`turnDecayAmount`（为 0 时省略），Mana 序列化不变；
- `AbilityDto.innate`（false 时省略）；
- `ResourceGainResolutionDto`、`ResourceGainSourceDto` 与 `GameEventOutcomeDto::ResourceGain`，事件 `resource.gained`。

存档沿用 `PlayerSaveDto` 既有字段：技法池写入 `resources`，先天能力熟练度写入 `abilityProgress`，`learnedAbilityIds` 仍只含研读所得。旧存档缺失新资源池时按 `initialFillPercent` 初始化、不抽 RNG；存档中的池校验放宽为子集匹配（未知 ID、上限不符或超上限仍拒绝）。无 `castingProfile` 的类仍不得携带 `learnedAbilityIds`。

Web 资源面板展示获得/衰减提示与节奏池，休息按钮只在存在可休息恢复的未满资源时可用，新增 `resource.gained` 与技法相关中英文案。

## 5. 契约场景

`contract-v89` 的 272 条场景全部迁移保留（内容包与 hash schema 变化仅体现为 stateHash 更新）。新增 273–282 共 10 条：决斗家初始状态与先天能力、命中获得、击杀获得与封顶、闲置衰减与等待不恢复、弦月斩消费与伤害、涌动节奏自身状态与消费回合不衰减、资源不足零 RNG 拒绝、仅节奏未满时休息立即以 full-resources 停止、存档往返、旧存档缺池迁移。active baseline 共 282 个 exact fixtures、零 waiver。
