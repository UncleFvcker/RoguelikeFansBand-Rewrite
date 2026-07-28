# Contract v108：充能物品实例与首批治疗消耗品

## 范围

Contract v108 建立可重复使用设备的实例级充能、治疗骰物品效果和首批旧版固定治疗药水导入。协议为 `1.108`，demo 内容包为 `1.99.0`，state hash Schema 为 `47`，active baseline 包含 368 条 exact fixtures、零 waiver。内置内容 hash 为 `4105aec18bdc40aced03bb503ec31e30385248545266d116b1d0088a374c04c8`。

## 原版语义与当前边界

原版设备在使用前先检查剩余充能；无充能不消耗行动能量，也不进行设备失败检定。失败检定消耗行动但不扣充能；效果成功执行后才按 activation cost 扣除设备实例的 SP。staff/wand/rod 的 `k_info` 条目只是通用物品壳，实际激活效果与容量在生成时落到实例上。

本轮据此把充能建成 `ItemInstance` 权威状态，而不是背包显示缓存：

- 内容 `useAction.charges` 声明新实例的 `initial/maximum/cost`；
- 充能物品必须是 `maxStack: 1` 的 device，并必须声明设备检定难度；
- 当前值与容量随地面、背包、怪物携带、楼层缓存和 save/replay 一起移动；
- 成功检定扣充能但不消耗物品；失败保留充能；不足时不抽 RNG、不推进 world tick；
- `InventoryItemDto.usable` 在不足时为 false；精确充能只在物品种类已知时投影，未鉴定物品不会从 DTO 泄漏数值；
- 读档严格拒绝缺失、超容量、容量与当前内容不符或出现在非充能种类上的充能状态。

本轮只建立实例级容量/余量；staff/wand/rod 的动态效果身份、随机容量生成和充能恢复仍留给下一设备导入纵切。

## 物品效果与 demo

`ItemUseEffectDefinition` 保留固定 `heal`，新增确定性的 `heal-dice`。骰值从正式模拟 RNG 获取，事件继续复用既有 healing resolution，因而 requested/applied、鉴定边界和回放顺序不分叉。

demo 新增 `resonance mender`：2d4 治疗、3/3 初始充能、每次成本 1、设备难度 25。它以既有陌生装置外观进入未知识状态；可观察的成功治疗使种类变为 aware 并显示剩余充能。

## Legacy 导入结果

固定 commit 的六种治疗药水按原版 `tval=75/sval` 精选映射：

| sval | 原版效果 | 导入效果 |
| --- | --- | --- |
| 34 | Cure Light Wounds | 4d8 heal |
| 35 | Cure Serious Wounds | 8d8 heal |
| 36 | Cure Critical Wounds | 12d8 heal |
| 37 | Healing | 300 heal |
| 38 | *Healing* | 1000 heal |
| 39 | Life | 5000 heal |

真实 `consumable-effect` 缺口由 95 降至 89；`device-effect` 仍为 64，因为通用 staff/wand/rod 需要下一轮实例化效果生成，不能把某个固定激活错误写到种类壳上。重新生成的本地 legacy 包严格编译通过：1260 abilities、4 ability books、937 items、128 affixes，content hash 为 `ed9534de7976be4668a8238deae3d207794d862e7a4ab41e888fde8c7e7b479c`。

## Fixtures 与兼容性

- 366：Tinkerer 成功检定，2d4 固定为 4 点治疗，充能 `3 -> 2`，设备保留并成功回档；
- 367：Vanguard 失败检定，设备保留且充能不变；
- 368：0 充能设备拒绝使用，world tick 与 RNG 均不前进，`usable=false`，并成功回档。

365 条 v107 场景迁移到 Schema v47 后保持既有命令语义，新增三条场景形成 368 条 active exact fixtures。正式旧 demo hash `d8bdbdd4d4e85862a97229c279a874668b9b1d3ce9035aa6f17a11cff7b3af80` 已加入兼容列表；旧内容不包含 charged kind，因此历史 save 的既有物品继续以无充能状态零 RNG 迁移。

## 后续候选

P59 优先把动态设备效果身份与容量物化到实例，接入首批 staff/wand/rod 实际激活；之后可复用同一事务扩展充能恢复、artifact/ego activation 和剩余 89 个固定消耗品。卷轴、取消目标后是否扣费、设备强行使用与再充能公式必须分别对照原版后再进入通用表面。
