# Contract v67：地牢入口守卫与可选进入条件

状态：协议 1.67 / contract-v67 active baseline

Contract v67 将 RFB 原版“高等级地牢入口由怪物守住”的体验接入重写核心，同时保留用户直接挑战地牢的自由。入口守卫是战斗性的软门槛，不是不可绕过的等级检查；内容包还可以声明面向原创地牢的可选硬进入条件。

## 入口守卫

`DungeonDefinition.entranceGuardian` 可声明一个独立的 `instanceId`、怪物种类和地表位置。新游戏在生成世界实体时生成该怪物，并把它标记为 `GuardPosition` pack 行为：守卫不会主动追击或离开岗位，但玩家进入相邻格时仍按普通近战规则攻击。

守卫不占用楼梯本身，也不阻止玩家执行 `TraverseStairs`。因此玩家可以绕开守卫直接进入目标 dungeon；击败守卫只会记录该 dungeon 的入口已被清除。守卫死亡后不会在返回地表、再次进入或载入存档时重新生成，并发布 `dungeon.entrance-guardian-defeated` 事件。该事件是持久规则状态的一部分，但与 dungeon 最终守护者的征服状态独立。

## 可选硬条件

`DungeonDefinition.entryRequirements` 是按声明顺序稳定校验、并按 AND 组合的可选条件。当前支持：

- `task-status`：任务处于指定状态；
- `dungeon-conquered`：指定 dungeon 的最终守护者已被击败；
- `carried-item`：玩家库存或装备中拥有指定数量的物品种类。

demo 原版地牢不配置硬条件，玩家只需抵达入口即可尝试任何 dungeon。原创内容可以用这些条件表达钥匙、前置任务或征服顺序。条件在分配 dungeon instance ordinal、切换楼层和生成 RNG 之前检查；失败时返回进入拒绝，不改变当前楼层、存储楼层、实例序号或 RNG。

## 存档与迁移

`DungeonStateSaveDto` 增加可选 `entranceGuardianDefeated`。新存档显式写入布尔值。v66 及更早存档缺少该字段时，迁移器将已有入口守卫视为“已被抑制”，不回填实体，也不为了迁移额外消耗 RNG；这样旧存档不会在载入后凭空出现新的地表怪物。新旧状态均经过入口守卫实体与持久状态一致性校验。

入口守卫状态进入 state hash Schema v26；save 容器仍为 v1，协议版本升至 1.67。内容模型和协议绑定同步生成，`GuardPosition` 也作为稳定的 pack behavior DTO 保存。

## Demo 与验收

demo 内容包升至 1.59.0，content hash 为 `71d2f947fe2bb7b5e2190a12fdff12ba47ea9f7fc17b1eb26390b46d8abd092b`。Resonance dungeon 的入口位于地表 `(2,1)`，守卫为 `demo.actor.resonant-warden`；Echo 与 Resonance 均不配置硬进入条件。

contract-v67 从 v66 的 132 个 exact fixtures 迁移并增加入口守卫生成、可绕过软门槛和击败持久化三个场景，共 135 个 exact fixtures、0 waivers。核心测试还覆盖 `GuardPosition` 不移动、事件、存档回读、旧存档迁移，以及任务、征服和携带物三类条件的原子拒绝。

## 明确不包含

本切片不加入等级硬门槛、自动传送、同一 dungeon 的暂停实例选择、胜利/退休评分或新的实例生命周期策略。普通 dungeon 返回地表仍立即清理当前实例，下一次进入重新生成；入口守卫只是入口处的战斗压力，不能改变这条生命周期规则。
