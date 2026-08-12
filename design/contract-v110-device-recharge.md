# Contract v110：设备自然恢复与主动充能

> 历史合同。contract-v264 / Protocol 1.175 已删除本页的职业
> `deviceRechargeProfile`、`RechargeItem` 与 Artificer 界面入口；设备自然恢复、
> `UseItemForRecharge` 和供物品/卷轴复用的底层充能事务仍然有效。

## 范围

Contract v110 在动态 staff/wand/rod 实例之上建立确定性的自然能量恢复，并增加由职业资源或另一件设备供能的主动充能事务。协议为 `1.110`，demo 内容包为 `1.101.0`，state hash Schema 为 `49`，active baseline 包含 379 条 exact fixtures、零 waiver。内置内容 hash 为 `f2bf96ea4a980a6a9914ca80dff5527a5e04b2e36d25aa668b118e6562c9cad9`。

## 自然恢复

`ItemDefinition.deviceGeneration.recovery` 声明：

- `intervalTicks`：每隔多少 world tick 结算一次，合法范围为 1–10000；
- `energyPerMille`：每次按实例最大能量的千分比恢复，合法范围为 1–1000。

demo rod 使用 interval 1，wand/staff 使用 interval 10，三者均为 `energyPerMille: 10`。不足 1 点的结果进入实例级 `deviceRecoveryProgress` 余数，后续结算继续累积，不抽 RNG；达到最大能量时余数归零。首版只恢复玩家背包中的动态设备，地面、装备和怪物携带物不自动恢复。

地面、背包、装备和怪物携带四类 save DTO 均保存 0–999 的恢复余数。旧档缺字段按 0 载入；非恢复设备携带余数、余数越界或满能量仍携带非零余数均拒绝。

## 主动充能

`ClassDefinition.deviceRechargeProfile` 声明资源 ID、主宰属性、资源上限公式、充能 power 和设备来源损毁率。demo Artificer 使用新的 `demo.resource.resonance`，power 为 90，设备来源损毁率为 `1 in 3`。

协议增加：

```text
RechargeItem {
  targetItemId,
  source: resource | item { itemId }
}
```

目标必须是背包中未满的动态设备；资源来源必须有非零职业资源，设备来源必须是另一件有能量的背包设备。实际尝试量为充能 power、来源余量和目标缺口三者的最小值，来源在检定前扣除。合法命令消耗一个普通行动。

失败检定使用目标设备难度与职业 power 计算 `1 in N` 失败率。资源来源失败时目标能量归零；设备来源失败时目标保持原值。设备来源在每次合法尝试中独立承担内容声明的损毁率；带 `artifact` tag 的来源不会被销毁，但仍扣除供给能量。非法命令发布 `device.recharge-unavailable`，不推进 world tick、不抽 RNG，revision/turn 仍按现有零时间命令规则前进。

`PlayerDto.deviceRecharge` 投影当前职业的资源和 power；背包项目投影 `canReceiveRecharge` / `canSupplyRecharge`。Web 在目标设备行提供“充能”命令：勾选另一件可供能设备时使用物品来源，否则使用职业资源。结果通过结构化成功、失败和不可用事件进入中英文 Fluent 文本。

## Fixtures 与调试

- 374：同为 20 最大能量，50 world ticks 后 rod 恢复 10、wand 恢复 1，固定十倍速率差；
- 375：600/1000 的恢复余数完成存档回读且不抽 RNG；
- 376：无 recharge profile 的职业零时间、零 RNG 拒绝；
- 377：资源来源成功，目标 0→24，资源 25→1；
- 378：资源来源失败，已支付 19 点资源且目标 5→0；
- 379：设备来源成功，来源 5→0、目标 0→5，并完成存档回读。

Contract 前置条件增加 `debugRechargeAttemptsSucceed`、`debugRechargeAttemptsFail` 和 `debugRechargeSourcesSurvive`。它们只用于固定成功、失败和来源存活分支，不替代正式充能公式或 RNG 管线。373 条 v109 场景迁移到 Schema v49 后保持既有命令语义。

## Legacy 导入结果

原版通用 rod 壳写入 interval 1，wand/staff 壳写入 interval 10，均按最大能量的 1% 恢复。职业主动充能尚未从旧版职业代码自动生成；P60 只把通用内容字段和运行时事务建立为后续导入目标。现有 `device-effect` 61、`artifact-activation` 180、`ego-activation` 13 和 `consumable-effect` 89 缺口保持。重新生成的本地包严格编译通过：937 items、128 affixes、1260 abilities、4 ability books，content hash 为 `21b00c14f10f6feff7e87f0a37e7974c78ab683e4995190eae040a4c84601137`。

## 后续候选

P61 应重新按真实导入覆盖收益排序，继续扩展 `device-effect`、artifact/ego activation 或 consumable effects。强行使用、desperation、更多来源类型和复杂充能副作用必须作为独立规则进入契约，不能隐藏在 UI 或导入器特例中。
