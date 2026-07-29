# Contract v131：Recharging 卷轴

日期：2026-07-29

Contract v131 接入原版卷轴 tval 70 / sval 22 的 Recharging。协议升级为 `1.121`，demo 内容包为 `1.122.0`，save 容器保持 v1，state hash Schema 保持 `53`。active baseline 包含 434 条 exact fixtures、零 waiver，内置内容 hash 为 `d486f818e41cea542ac951f6a92abca69e298d29f5139e6219ddd0c34836ad52`。

## 1. 效果与事务边界

内容层新增仅供静态消耗品使用的 `recharge-from-device { power }`；demo 与 legacy sval 22 均使用 power 100。它不能作为动态 activation，不能携带 charges 或 device check，也不扩展 `AbilityEffectDefinition`。

命令必须同时提供卷轴、来源设备和目标设备三个互异的背包实例 ID。来源必须是仍有能量的动态设备，目标必须是未充满的动态设备。任一实例缺失、不在背包、数量为零、身份重复或不满足设备条件时，在消费、world tick 和 RNG 前返回普通 `item.use-unavailable`。

合法事务按以下顺序执行：

1. 卷轴写入 Tried 并消费一件；
2. 来源设备执行固定 `one_in(3)` 损毁判定；未损毁或为 artifact 时扣除 `min(power, source energy, target missing)`，非 artifact 命中损毁时移除来源；
3. 目标复用 contract-v110 的设备充能失败公式和结构化事件；设备来源失败时目标能量保持不变；
4. 无论目标成功或失败，卷轴都写入 Aware。

artifact 来源只免于损毁，仍支付本次能量。来源损毁、来源扣能与目标失败均属于一次已提交事务，不因目标失败回滚。

## 2. 协议与界面

协议新增窄命令 `UseItemForRecharge { itemId, sourceItemId, targetItemId }`，`InventoryItemDto` 增加省略式 `requiresRechargeTargets`。没有把第二个物品输入塞入通用 `TargetSelection`，三个 ID 都是单次命令输入，不进入存档或 state hash。

Web 不增加新向导。玩家同时选中 Recharging 卷轴与一件 `canSupplyRecharge` 设备后，“使用”按钮复用既有物品目标对话框，并只列出第三件 `canReceiveRecharge` 设备。核心仍重新执行完整预检，界面过滤不承担权威规则。

## 3. 导入与契约

legacy importer 将 tval 70 / sval 22 映射为 `recharge-from-device { power: 100 }`。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 严格导入成功，`scroll-effect` 从 17 降至 16；真实包保持 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races，源码校验、编译与二进制回读 hash 均为 `3df0f3da5a5700ba42d0e6b40a1bcd630d298d1f808292f1da5e043dfb33084b`。

fixture 434 固定来源 5 点能量成功转入空目标、来源存活、卷轴消费、知识、事件和存档回读。一个窄核心单测覆盖来源缺失、来源与目标相同的零时间零 RNG 拒绝，以及强制目标失败时卷轴和来源已支付、目标保持不变；既有 contract-v110 测试继续覆盖设备来源损毁与 artifact 保护。

既有 433 条 fixture 只因协议和内容 hash 输入机械刷新其 state hash 字段。

## 4. 明确遗留

- 原版允许来源和目标来自地面；当前命令只接受背包实例，与现有 Web 物品事务边界一致；
- `_scroll_power`、Devicemaster Scrolls 专精和其他非卷轴入口尚未接入；
- 来源损毁率固定为原版 `one_in(3)`，不开放内容字段或新的 debug 开关；
- 没有建立通用多物品 target、通用设备能量转移 effect 或新的充能失败管线；
- 剩余 16 个 `scroll-effect` 继续按世界/地形、状态和物品/成长事务拆分。
