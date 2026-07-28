# Contract v114：卷轴传送、跨层与召回

状态：已实现

Contract v114 接入原版卷轴 sval 8/9/10/11/53：Phase Door、Teleport、Teleport Level、Word of Recall 与 Reset Recall。协议为 `1.114`，demo 内容包为 `1.105.0`，state hash Schema 为 `50`，active baseline 包含 398 条 exact fixtures、零 waiver。内置内容 hash 为 `36d07a047c3a9a331f051d4a0ebaa87070caef56408efb375e3b61e7e3fb1d86`。

## 1. 内容模型与原版映射

物品效果新增四种通用定义：

- `random-teleport { maximumDistance }`：距离限制为 1–200，Phase Door 使用 10，Teleport 使用 100；
- `teleport-level`：在当前地牢树的上/下方向目标间转移；
- `recall { delayDice, delaySides, delayBonus }`：Word of Recall 使用 `1d21 + 14`，即 15–35 个玩家行动周期；
- `reset-recall`：把当前地牢楼层设为稳定召回目的地。

四种效果都是 self-only。编译器验证目标模式、距离和延迟骰边界；固定 `useAction` 与动态设备 activation 使用同一效果定义和运行事务。demo 新增 Flicker、Farstep、Depthshift、Homeward 与 Recall Setting 五种原创卷轴。

## 2. 同层与跨层传送

随机传送先枚举最大距离内可行走、非当前格且没有存活 actor 占据的位置，按切比雪夫距离降序及坐标稳定排序，只保留最远的 `ceil(n / 2)` 个候选，再用一次正式 RNG 等概率选择。成功到达复用 `relocate_player`，因此被动感知、陷阱、伤害与死亡顺序和普通位移一致；无候选、错误目标或其他前置失败在消费、设备检定、RNG 和 world tick 前原子拒绝。

Teleport Level 先消耗一次正式 RNG 作 50% 上/下方向判定。所选方向没有目标时回退另一方向；同方向存在多个树连接时按稳定目标集合再等概率抽取。地牢层优先使用实例级已解析连接，保留成对楼梯的到达点和树状分支；连接缺失时才使用内容声明的 return/next 回退。地表只有在已有合法召回目的地且满足对应地牢进入条件时才能向下转移。

楼梯、Teleport Level 与 Recall 统一通过楼层转换和事件发布 helper，继续遵守共享守护者、跟随召唤物、任务楼层、到达点和实例生命周期规则。

## 3. 召回状态与实例生命周期

`RecallStateDto` 保存 `dungeonId`、`floorId` 和可选 `remainingTurns`。目的地使用内容稳定 ID，不保存易失的 dungeon instance ID：

- 首次进入地牢时建立目的地；进入同地牢更深或同深分支时自动更新，回到更浅层不会自动降级；进入另一地牢时切换到新地牢目的地；
- Reset Recall 只能在地牢内、且没有待触发召回时使用，并把目的地明确降为当前楼层；
- Word of Recall 启动正式延迟骰；测试 debug override 只替换最终等待值，正式骰仍消费；每个完成的玩家行动周期把倒计时减一；
- 待触发时再次使用 Word of Recall 会取消倒计时，但保留目的地；
- 地牢内触发时返回地表；地表触发时进入记录楼层。`reset-on-surface` 地牢返回地表后清除旧实例，从地表召回会创建新实例，因此不会保存或复活已清理的普通地牢实例。

前置无效时不消费物品、不抽 RNG、不推进世界时间。成功使用后来源种类进入 aware，并投影 `item.use-teleported`、`item.use-teleported-level`、`item.recall-started/cancelled/reset/triggered` 等结构化事件。

## 4. 存档、迁移与 fixtures

`PlayerDto` 与 `PlayerSaveDto` 都增加可选 `recall`。目的地必须引用同一 dungeon 的有效 dungeon floor；倒计时为 1–2000，且只能在地表或 dungeon floor 上待触发。v113 built-in 存档缺失 recall 时，在地牢内从当前楼层无 RNG 派生目的地；地表旧档保持无目的地。新增权威状态使 state hash 升至 Schema v50，save 容器仍为 v1。

fixtures 390–398 固定：

- 390–391：短距/长距随机传送的候选排序、正式 RNG 与到达结果；
- 392：地表没有召回目的地时，Teleport Level 和 Recall 均零消费、零 RNG、零 world tick 拒绝；
- 393：Teleport Level 在树状楼层中选择方向、分支并保持回读一致；
- 394：Recall 启动后再次使用会取消，目的地不丢失；
- 395：地牢召回地表并清除普通地牢旧实例；
- 396：从地表召回同一稳定楼层时创建新实例；
- 397：Reset Recall 把深层目的地降到当前浅层并可回读；
- 398：pending Recall 的稳定目的地与剩余 3 回合完成存档回读。

## 5. 导入结果与后续

真实导入包保持 937 items、128 affixes、1260 abilities 和 4 ability books；`scroll-effect` 从 52 降至 47。严格源校验、编译和二进制回读 hash 均为 `7d194979fdc047e93f60325f8d3d3b068d75a0f9e0b38eb5be0ecfd0ce77beba`。

剩余 47 条 `scroll-effect` 中，下一轮优先比较装备附魔/强化五条、召唤四条和解除/施加诅咒四条的通用系统收益。跨世界召回、多个可选召回槽、城镇服务、召回中断伤害和召回目的地 UI 不在本轮范围。
