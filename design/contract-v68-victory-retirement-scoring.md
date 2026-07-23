# Contract v68：胜利、退休与角色评分

状态：协议 1.68 / contract-v68 active baseline；内容包 1.60.0；state hash Schema v27

## 目标

这一纵切参考原版通过最终守护者死亡设置 `total_winner` 的结算边界，并把当前重写版尚未具备的经验、等级和金币评分替换为可复验的内容驱动评分。它只负责 campaign 的胜利、退休和分数冻结，不引入等级硬门槛，也不改变普通 dungeon 返回地表即清理实例的规则。

## Campaign 定义

世界可选声明 `campaign`：

- `victoryDungeonIds`：完成战役必须征服的 dungeon ID 列表；
- `dungeonConquestPoints`：每个已征服 dungeon 的分值；
- `taskCompletionPoints`：每个已完成任务的分值；
- `victoryBonus`：满足全部 victory dungeon 后的一次性奖励；
- `turnPenaltyInterval` 与 `turnPenaltyPoints`：每经过一个完整回合间隔扣分。

demo 将 Resonance 配置为唯一 victory dungeon。Echo 的最终守护者镜像仍会设置 Echo 的 `guardianDefeated` 并增加征服分，但不会触发 campaign 胜利。

## 状态与命令

`CampaignStateDto` 的状态为 `active`、`victorious` 或 `retired`。击败最后一个 victory dungeon 的最终守护者后，核心在同一命令结算末尾发布 `campaign-victorious`，保存 `victoryTurn`，并将状态改为 `victorious`。

`Retire` 是终止结算命令：只有状态为 `victorious` 且当前位于世界初始地表层时才成功。成功退休计算并保存 `finalScore`、`retiredTurn`，发布 `campaign-retired`，不运行怪物调度，但仍递增 command sequence、revision 和 turn 以保持回放确定性。条件不满足时发布 `campaign-retire-unavailable`，不改变世界实体、RNG 或能量。退休后任何进一步 dispatch 都返回 `CampaignEnded`。

## 评分

```text
base = conqueredDungeons * dungeonConquestPoints
     + completedTasks * taskCompletionPoints
     + (victorious ? victoryBonus : 0)
final = max(0, base - floor(turn / turnPenaltyInterval) * turnPenaltyPoints)
```

退休后分数从 `finalScore` 读取，不再随回合变化。当前 demo 参数为地牢 10000、任务 1000、胜利奖励 50000、每 100 回合扣 1 分。

## 存档、迁移与哈希

`SavePayloadV1.campaignState` 为可选字段。v67 及更早存档缺失该字段时按 `active` 载入；如果持久 dungeon 状态已经满足全部 victory dungeon，则确定性推导为 `victorious`，不推进 RNG。状态不变量拒绝未知 victory dungeon、非法回合顺序、退休时不在地表、分数与退休回合不一致等存档。

campaign 状态进入 state hash Schema v27，并同步写入完整快照与增量更新。旧 content hash `71d2f947fe2bb7b5e2190a12fdff12ba47ea9f7fc17b1eb26390b46d8abd092b` 允许迁移到内容包 1.60.0；当前 hash 为 `1614fadbf4cd1d3ee03fc011eac069de3a1b8c23ec65b6f09e210f20008dbc4c`。

## Contract 与 UI

contract-v68 active baseline 为 137 个 exact fixtures、0 waiver，覆盖 Echo 侧 dungeon 征服不触发 campaign、胜利前退休拒绝、退休分数冻结、退休后命令拒绝、存档回读以及旧存档状态推导。最终守护者死亡触发胜利的深层生成路径由核心确定性测试覆盖，并使用无伤 debug 抗性减少测试消耗。

Web 状态面板显示 campaign 状态、评分、征服地牢数和完成任务数；只有 victorious 且在 demo 地表时启用退休按钮。事件文本同时提供英文和简体中文 Fluent 资源。

## 后续边界

下一切片转入 P8：只为未来原创 dungeon 讨论可配置实例的持久化、TTL 和淘汰策略。demo 普通 dungeon 继续在返回地表时清空，下一次进入重新生成；不增加等级门槛、跨实例传送或并行实例 UI。
