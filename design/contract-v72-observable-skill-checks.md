# Contract v72：可观察技能检定

状态：协议 1.72 / contract-v72 active baseline；内容包 1.64.0；state hash Schema v31

## 范围

v72 在 v71 的 Race/Class/Personality 与技能集合之上，让 `device`、`saving-throw`、`stealth` 和 `perception` 第一次进入权威规则消费。实现参考旧 RFB 固定基准 `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 的行为分工：装置有独立使用失败率，saving throw 可抵消敌对效果，stealth 降低怪物察觉概率，perception 在移动后进行被动发现。

本纵切只建立可观察、可存档、可回放的最小闭环，不把旧版全部环境修正、职业能力和法术系统一起引入。

## 内容模型

内容定义新增四个可选规则入口：

- terrain 的 `perceptionCheckDifficulty`：玩家成功移动后，对邻近且尚未揭示的该类 terrain 进行被动感知；
- trap 的 `savingThrowDifficulty`：陷阱触发后、伤害结算前进行 saving throw；
- item `useAction.deviceCheckDifficulty`：使用效果前进行装置检定；
- actor 的 `awareness { detectionDifficulty, detectionRange, startsAlerted }`：声明怪物的察觉难度、最大范围和出生警戒状态。

缺少这些字段的既有内容保持原行为：普通物品直接使用，普通陷阱直接结算，terrain 不触发被动感知，未声明 awareness 的怪物按已警戒恢复。

demo 内容包升至 1.64.0，并加入：

- `demo.item.resonance-stabilizer`：难度 60 的一次性治疗装置，具有独立未知外观；
- `demo.terrain.trap-resonance-ward`：难度 40 的 poison 陷阱豁免入口；
- `demo.terrain.echo-rune-hidden`：难度 24 的隐藏感知目标；
- `demo.actor.echo-listener`：难度 7、范围 8、出生未警戒的沉眠哨兵。

## 权威规则

### 装置

使用带 `deviceCheckDifficulty` 的物品时先把其种类标记为 tried，再进行 `device` 检定。失败会消耗回合但不消耗物品、不应用效果，也不会把未知种类提升为 aware；成功后才消费一件物品并进入既有 `UseAction`/effect/鉴定管线。

### Saving throw

玩家踏入带 `savingThrowDifficulty` 的陷阱时先揭示真实 terrain，再进行 `saving-throw` 检定。成功表示完整抵消本次陷阱效果；失败继续使用既有伤害类型、抗性和死亡规则结算。陷阱本身不会因为成功豁免自动消失。

### Perception

玩家只有在成功移动到新格后才执行被动感知。核心按稳定邻接方向顺序枚举尚未揭示、声明 `perceptionCheckDifficulty` 的 terrain；每个候选独立检定并发布事件，成功者写入权威 `revealedTerrain`，失败者继续只投影伪装 terrain。受阻移动、近战替代移动和没有候选的移动不会额外推进感知 RNG。

### Stealth 与警戒

未警戒怪物获得行动机会时，只有玩家位于 `detectionRange` 内且存在视线才进行对玩家 `stealth` 的察觉检定。玩家成功保持隐藏时怪物保持未警戒并放弃本次行动；失败时怪物永久进入警戒，并立即继续本次 AI 行动。玩家主动近战攻击目标会直接使其警戒。警戒状态属于 actor 权威状态，不是前端动画标志。

## 结构化检定结果

协议 1.72 新增 `CheckOutcomeDto`、`CheckResolutionDto` 和 `GameEventOutcomeDto.check`。四类新事件都携带：

- 稳定技能 ID；
- 最终 ability 与 difficulty；
- 百分位骰、可选对抗骰与阈值；
- `automatic-success`、`automatic-failure`、`success` 或 `failure` 结果。

检定继续复用统一确定性算法：先掷 `0..99`；`0..4` 自动成功，`5..9` 自动失败；其余情况下 ability 非正值失败，否则掷 `0..ability-1`，达到 `difficulty * 3 / 4` 即成功。所有骰都来自正式模拟 RNG，事件只投影已经完成的权威结算。

## 存档、hash 与兼容

`EntityDto.alerted` 暴露运行时警戒状态；`ActorSaveDto.alerted` 使用可选字段保存它。v71 及更早存档缺少该字段时按当前 actor 内容默认值恢复：声明 `startsAlerted=false` 的怪物保持沉眠，其他怪物保持历史上的已警戒行为。载入不补掷察觉骰，也不推进 RNG。

actor `alerted`、四类技能消费后的知识/地形/生命状态和完整 RNG 位置进入 state hash Schema v31。正式 save 容器仍为 v1；v71 内容 hash `1c94890a0f39d42a4b496a7222b8c9d191f24fe94b3c9d47d4a1eeea5364c5b4` 保留在迁移白名单中。

## 确定性覆盖

active baseline 位于 [`tests/fixtures/contract-v72/scenarios`](../tests/fixtures/contract-v72/scenarios)，共有 160 个 exact fixtures、零 waiver。新增八个场景使用同 seed 的高低构筑对照：

| 入口 | seed | Tinkerer | Vanguard | 难度 |
| --- | ---: | ---: | ---: | ---: |
| device | 0 | 69，成功并消费 | 16，失败且保留 | 60 |
| saving throw | 2 | 45，抵消陷阱 | 29，承受伤害 | 40 |
| perception | 1 | 25，揭示符文 | 4，保持隐藏 | 24 |
| stealth | 5 | 7，保持未被发现 | 1，怪物警戒并行动 | 7 |

核心专项测试同时锁定百分位/对抗骰、装置知识状态、陷阱伤害分流、terrain 真值投影、警戒移动，以及 `alerted=true` 的 save round-trip 和缺字段默认恢复。

## 明确不在 v72 的范围

- 技能练习、技能下降、使用次数熟练度和训练 UI；
- 失明、无光、混乱、幻觉、噪声、距离和环境亮度对这些检定的完整修正；
- 怪物间警戒传播、睡眠深度、气味/flow、智能学习和潜行模式；
- 法力、能力书、法术学习、施法失败率、冷却和完整职业能力系统；
- 完整原版 Race/Class/Personality 与内容规模。

下一纵切转入法术/能力书基础：先建立可保存的资源与法术身份、学习/可用性和一次可观察施法，再扩展职业矩阵与怪物施法。
