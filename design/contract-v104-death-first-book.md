# Contract v104：玩家等级效果缩放与 Death 第一册

## 范围

Contract v104 在 P52/P53 的职业施法档案与逐法术参数覆盖之上，完成
Death 第一册 `[Stench of Death]` 的八个槽位。协议为 `1.104`，demo
内容包为 `1.95.0`，state hash Schema 为 `43`，active baseline 包含
334 条 exact fixtures，零 waiver。内置内容 hash 为
`c0708c7866d93bdbb6601d349300cd5ef5e95a7ebd754de60d62e27d6c4071c6`。

## 玩家等级缩放

Ability 可声明最多 32 条 `levelScaling`，每条引用顶层效果或 Sequence
中的一个效果，并选择以下七种标量之一：`damage-dice`、
`damage-bonus`、`radius`、`status-intensity`、`status-duration-ticks`、
`status-power`、`control-power`。

有效值统一为：

```text
base + floor(max(level - levelOffset, 0) * multiplier / divisor)
```

内容编译器验证 effect index、字段与效果类型匹配、除数非零以及 100 级
结果不越过对应字段上限。基础 ability 保持内容权威；Core 在玩家投影、
学习检查和施放前生成同一份 effective ability。缩放结果不单独入档，读档
后由内容包、职业覆盖和玩家等级确定性恢复。

## 新效果与权威状态

- Detect 增加 `actor` subject。它按 Chebyshev 半径和 actor tag 检测，
  不受视线阻挡，结果按距离、坐标和 entity ID 稳定排序；actor 检测必须是
  transient，不写地图记忆。
- ApplyStatus 增加可选 `power`。目标和 power 各掷一次有界骰，目标点数
  大于等于 power 点数时抵抗；不带 power 的既有状态保持原行为。
- `rfb.status.sleep` 会在怪物取得行动权并支付行动能量后跳过本次行动。
  任意大于零且未致死的伤害会立刻移除睡眠并发布唤醒事件。
- 状态实例可携带 `grantedResistances`。临时抗性参与玩家有效抗性合并，
  随状态计时和消失，并进入存档、协议与 state hash。
- Control 按目标 tag 判断资格。无资格或已受控目标不抽控制骰；成功后保存
  `controllerId`，解除目标所在 pack（控制 leader 时解散全 pack），阵营投影
  为 player，并复用玩家召唤物 AI。控制身份随存档和 state hash 保留。

## Death 第一册

demo 的 Echo Primer 现在包含完整八个 Death ability：

| 槽位 | Ability | v104 行为 |
| --- | --- | --- |
| 0 | Detect Unlife | 半径 8 检测 `nonliving` actor |
| 1 | Malediction | hell-fire `3 + floor((level-1)/5)`d4 |
| 2 | Detect Evil | 半径 8 检测 `evil` actor |
| 3 | Stinking Cloud | 半径 2 poison，总伤害 `10 + floor(level/2)` |
| 4 | Black Sleep | power `level*2`，成功后施加 sleep |
| 5 | Necromantic Resistance | 300 ticks 的 cold/poison resistant |
| 6 | Horrify | fear power `level*2`，stun 持续 `5 + floor(level/5)` |
| 7 | Enslave Undead | power `level*2`，控制 `undead` actor |

`gloom-weaver` 增加 `evil` tag，`resonant-warden` 增加 `undead`；导入器
把旧版 `UNDEAD`、`DEMON`、`NONLIVING` 统一派生为 `nonliving`。

## Legacy 导入结果

固定 commit 的真实导入生成 8 个玩家 ability、1 个 ability book、12 个
运行时 casting profile 和 96 条职业参数覆盖/映射行。Death 效果缺口从
480 降到 384；`player-level-effect-scaling` 与
`monster-status-power-resolution` 清零。每个静态职业仍保留一条
Malediction 随机 rider、随机抗性持续、施法负重、Mana 容量和学习容量
公式缺口。

本地包共 1236 个 abilities，content hash 为
`6106efe2d864592c4ffd6d774d8f12b1ffb6ac1775fd9a47e5afc5147bbac7dd`。
`.local/packs/rfb-legacy` 仍是忽略目录，不进入提交。

## Fixtures 与遗留

- 329：等级 11 投影锁住五个缩放法术的有效值；
- 330：非生命 actor 检测与稳定 entity ID；
- 331：Black Sleep 成功并跳过怪物行动；
- 332：睡眠目标受非致死近战伤害后立刻唤醒；
- 333：临时 cold/poison 抗性及 save round-trip；
- 334：不死控制、player faction、`controllerId` 与 save round-trip。

Malediction 的 1/5 随机 rider 仍未实现；Necromantic Resistance 暂用固定
300 ticks，未复刻原版 `20+1d20` 回合。施法强度、装备 spell power 与职业
负重公式继续保留为后续系统。原列为 P55 的 Death 第二册已由
[Contract v105](contract-v105-death-second-book.md) 完成：活体限定、
bolt-or-beam、自身中心 AoE、灭绝、临时品牌、吸血和尸体复活均按独立
系统边界实现，没有降级成无条件伤害。
