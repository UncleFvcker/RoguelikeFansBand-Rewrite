# Contract v46：多深度最终层与持久守护者

状态：协议 1.46 / contract-v46 active baseline

## 已完成边界

- 地牢楼层新增稳定 `dungeonId`；内容编译器按 dungeon ID 验证从深度 1 入口到最终层的完整线性链，所有非最终层必须声明下一层，最终层必须禁止下行。
- 楼层新增显式 `finalFloor` 和 `guardian`。每座地牢必须恰有一个最终层与一个守护者，守护者实例 ID 和 actor kind 经过内容引用、角色、等级与全局实例唯一性验证。
- 原创回声地牢从两层扩展为三层；第 3 层是最终层，确定性生成唯一的“共振守卫”。
- 守护者死亡复用普通死亡、掉落和事件管线，并额外输出 `dungeon.guardian-defeated` 领域事件；奖励通过普通 loot table 生成。
- save v1 新增 `dungeonStates`，保存按 dungeon ID 排序的 `guardianDefeated`。击败状态不随探索实例结束而清除，因此重新生成三层地牢时守护者不会复活。
- 旧存档缺失 `dungeonStates` 时按守护者未击败迁移；当前存档中守护者实体与持久状态矛盾时拒绝载入。
- state-hash 升至 Schema v19，覆盖持久地牢状态。

协议 1.46，内容包 1.39.0，content hash `e03cb30ea8e1cd5821c14b54c4a038d30323cfc2cb6e0d6c483cbb006d70916f`，terrain 35，actor 8，loot table 4。save 容器仍为 v1，active baseline 共 91 个 exact fixtures。

## 与原版 RFB 对比

相同点：原版 `d_info` 用 `maxdepth` 和 `final_guardian` 描述地牢底层；`alloc_guardian()` 只在最大深度生成守护者，`_dungeon_boss_death()` 在守护者死亡时处理底层奖励。本实现同样只在最终深度生成守护者，并在普通死亡事实之后结算守护者状态和奖励。

主动差异：重构使用稳定 `dungeonId`、显式 `FloorId` 链、`finalFloor` 和守护者实例 ID，不依赖隐式整数深度、全局 `RF7_GUARDIAN` 标志或随机放置。守护者击败状态是可序列化、可哈希的独立地牢状态；奖励复用内容驱动 loot table，而不是在守护者死亡函数中硬编码属性提升、声望、神器和替代物逻辑。

暂未实现：入口守护者、分支/跳层/shaft 连接、随机楼梯、十层规模和深度 encounter 表；守护者的 unique 世界生态、神器奖励、声望与属性奖励；多座地牢、进入条件、胜利/退休和角色分数仍未建立。

## Fixtures

- contract-v46 从 v45 迁移 88 个 exact fixtures。
- 新增最终层生成守护者且无下行入口场景。
- 新增守护者死亡事件、掉落和存档回读场景。
- 新增结束探索、重新生成三层并确认守护者不复活场景。
- active baseline 共 91 个 fixtures，0 waiver。
