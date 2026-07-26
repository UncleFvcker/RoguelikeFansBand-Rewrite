# Contract v89：友方召唤物行动与首版命令

状态：历史 baseline；当前 active baseline 见 [contract-v90](contract-v90-technique-resources.md)。协议版本为 1.89，demo 内容包继续使用 1.80.0，content hash 继续为 `29116f924e1ef4ddf6b0aa43f3b1b1bd0b4d28245ac086bce30d7a008e8e9e8e`。save 容器继续使用 v1；玩家的持久召唤指令是新增权威状态，state hash 升至 Schema v39。

## 1. 原版参考与本轮边界

FrogComposband 的宠物菜单提供全局跟随距离、主动搜敌和停留类命令，命令本身不消耗一次世界行动；切换楼层时，只让玩家附近且仍可行动的宠物跟随。本轮保留这种“全局零时间命令 + 附近宠物跨层”的形式，但使用新核心的强类型模式：

- `follow`：优先攻击相邻敌人，否则向主人靠近，已相邻时停留；
- `attack`：按 Chebyshev 距离、实体稳定 ID 选择最近敌人，攻击相邻目标，否则追击；没有敌人时回退跟随；
- `keep-distance`：以主人为中心维持 3 格距离，过近时远离、过远时靠近，在目标距离已经满足时才攻击相邻敌人；
- `guard`：记录下令时玩家所在格作为锚点，优先攻击相邻敌人，否则回到锚点 1 格内。

指令是玩家全局状态，不只作用于当前实体；后续生成的玩家召唤物立即服从当前模式。首版不包含单体点名、主人死亡自爆、永久宠物、繁殖、召回、种群上限、宠物拾取或法术施放。

## 2. 行动、目标与 RNG

玩家拥有的召唤物进入现有 actor 能量调度器，使用自身保存的速度和 `energyNeed`。所有非玩家所有者的存活 actor 都是敌对候选；移动候选只接受地图内、可行走、无存活 actor 占据的格。

移动、目标枚举和方向 tie-breaker 不抽 RNG。近战复用 actor 的 `meleeRoutine`、命中、伤害骰、护甲、抗性和死亡事务，因此命中/伤害照常推进 RNG。召唤物击杀按玩家所有权结算经验、任务、守护者、掉落和 campaign 结果，并发出独立的 `combat.summon-*` 事件。

`SetSummonCommand` 不推进 `turn`、`worldTick`、玩家/actor 能量、召唤生命周期、能力冷却或 RNG，但会推进 revision/command sequence。`guard` 保存当前玩家位置；其他模式不得残留 guard anchor。损坏的组合在载入时拒绝。

## 3. 楼层与生命周期

沿楼梯切层前，以切层时玩家位置为中心选取 Chebyshev 距离不超过 2 的存活玩家召唤物，按稳定实体 ID 携带到目标层，并在目标到达点半径 5 内按距离、坐标稳定落位。较远的召唤物保留在来源楼层。目标层空间不足时不会覆盖玩家、actor、物品或阻挡 terrain；仍保留的来源楼层会重新接纳未能跟随的召唤物。

`guard` 在成功切层后把锚点重置为目标层玩家到达位置，避免引用旧楼层坐标。普通地牢返回地表时仍按既有 `reset-on-surface` 生命周期清理实例；召唤物剩余回合继续按实际玩家世界回合递减。玩家死亡后不再接受命令或推进调度，首版不额外修改终局存档中的召唤实体。

## 4. 协议、存档与 Web

协议 1.89 增加：

- `SummonCommandModeDto`、`SummonCommandDto` 与 `SummonCommandResolutionDto`；
- `GameCommand::SetSummonCommand`；
- `PlayerDto.summonCommand` 与 `PlayerSaveDto.summonCommand`；
- `summon.command-changed`、`summon.followed-floor`、`summon.could-not-follow`；
- `combat.summon-miss`、`combat.summon-hit`、`combat.summon-slay`。

旧存档缺失指令时默认 `follow` 且无锚点，不推进 RNG。Web 增加四键召唤指令面板、当前模式/数量显示，以及跨层和召唤物近战的中英文消息。

## 5. 基线

contract-v89 从 v88 迁移全部历史场景，并新增 7 个 exact fixtures：

- Guard 命令零世界时间、结构化结果和 save round-trip；
- Follow、Attack、Keep Distance、Guard 的稳定移动；
- 召唤物近战击杀、玩家经验与掉落归属；
- 2 格内跨层跟随与远处召唤物留层。

active baseline 共 272 个 exact fixtures、零 waiver。回放另有独立用例验证零时间命令、守卫锚点、Schema v39 检查点和最终 state hash 精确一致。

## 6. 后续

P30 建议恢复角色资源纵切：先建立一个非 Mana 职业资源，证明“按行动获得、按能力消费、具备独立上限/恢复条件”的内容与存档边界，再扩展怒气、专注、鲜血等完整职业矩阵。宠物单体命令、永久同伴和更广生态在出现实际内容需求时另立 contract。
