# Contract v111：有序恢复型消耗品效果

状态：已实现

Contract v111 把异常清除与职业资源恢复加入通用物品效果，并允许恢复型消耗品按声明顺序执行多个步骤。协议为 `1.111`，demo 内容包为 `1.102.0`，state hash Schema 保持 `49`，active baseline 包含 383 条 exact fixtures、零 waiver。内置内容 hash 为 `12c9160aec3bf8ebc6b7c92a785ad1ed8ad2dd23af674bd4bc6c445d2762d2e7`。

## 1. 内容模型

物品效果新增：

- `remove-status { statusKindId }`
- `restore-resource { resourceId, amount }`
- `restore-resource-dice { resourceId, dice, sides, bonus }`
- `restore-resource-full { resourceId }`
- `sequence { effects }`

`sequence` 只允许 2–8 个非嵌套、自目标恢复效果。编译器验证资源引用、非空状态 ID、骰值/固定值边界，并拒绝把 damage、detect 或另一条 sequence 放入恢复序列。动态设备 activation 可以复用单个恢复效果或恢复序列，但目标规格必须仍为 self。

## 2. 运行时语义

- 子效果严格按内容顺序结算并按相同顺序投影事件。
- 骰值资源恢复使用正式模拟 RNG；固定值和回满不抽 RNG。
- 恢复量钳制到资源池 maximum；成功恢复会触及该资源池，之后继续经过普通 world tick。
- 玩家没有对应资源池时，物品仍被消耗，恢复量为零，不创建资源池。
- 清除不存在的状态同样产生 no-effect 事件，不修改状态。
- 任一子效果产生可观察变化时，该种类立即变为 `aware`，本步和后续事件使用真实名称；整条序列均无变化时只标记 `tried`，事件继续使用外观名称。

事件使用 `item.use-status-removed` / `item.use-status-no-effect` 和 `item.use-resource-restored` / `item.use-resource-no-effect`。资源恢复沿用 `GameEventOutcomeDto::ResourceRecovery`，不增加显示缓存或新的存档字段。

## 3. 内容与导入

demo 新增 Clarity Draught（`3d6+3` Mana 后解除 confusion）与 Perfect Focus Elixir（Mana 回满后解除 berserk），两者共享未知药水外观。

legacy importer 接入四种状态恢复食物、Boldness、Vigor、Restore Mana 和 Clarity，并把六种既有治疗药水扩为原版可表达的有序异常清除序列。真实包保持 937 items、128 affixes、1260 abilities 和 4 ability books，严格编译 hash 为 `b6913ec229580a8decd6816fbebc4af6554bb55cd222fc7e11e9ceec1a353eac`；`consumable-effect` 从 89 降至 81。

## 4. 存档、回放与 fixtures

资源池、状态、物品数量和 `ItemKnowledge` 都复用现有 save v1 字段；旧档无需迁移，state hash Schema 保持 49。fixtures 380–383 固定：

- 骰值 Mana 恢复后解除 confusion，验证 3 次 RNG 与事件顺序；
- 回满 Mana 后解除 berserk，验证零额外 RNG；
- 缺少 Mana 池时消耗物品、保持未知且只标记 `tried`；
- 剩余堆叠、资源、状态和 `aware` 的存档回读。

## 5. 后续边界

属性恢复、经验恢复、食物营养、增益药水和卷轴专用效果仍未接入。真实 `device-effect` 61 经审计全部来自 tval 70/71 卷轴；三种通用 wand/staff/rod 壳已经由 Contract v109 接入首批 activation。P62 应先重分类并盘点卷轴的知识、传送、侦测、附魔等效果族，再按通用系统复用收益选择纵切。
