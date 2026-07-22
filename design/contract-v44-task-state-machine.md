# Contract v44：权威任务状态机与领域事件订阅

状态：协议 1.44 / contract-v44 active baseline

## 已完成边界

- 核心新增按 `taskId` 排序的权威 `TaskState`，保存 status、current、required 和 active floor。任务日志不再从入口 terrain、当前楼层和 `storedFloors` 反向推导状态。
- save v1 新增可选 `taskStates`。新存档写入完整状态集合；旧存档缺失时从 `taskProgress`、入口 terrain、当前楼层、离层仓库和目标实例迁移。
- 旧 `taskProgress.floorId` / `taskProgress.taskId` 继续兼容读取，但新存档不再写入平行进度副本。
- state-hash 升至 Schema v17，并直接覆盖规范化 `TaskState`。
- 怪物死亡事务不再直接调用任务计数函数。dispatch 完成规则结算后，由任务事件消费器统一订阅玩家近战、投射、投掷、状态死亡和拾取事件。
- `collect-item`、`kill-actor` 和 `kill-actor-kind` 都通过同一事件消费边界更新任务进度。
- 进入、暂停、恢复、完成、失败和放弃通过显式状态转换更新 `TaskState`；terrain 只保留入口表现和通行结果。
- v36–v43 的全部行为保持兼容；新增收集目标 `1/1` active 状态的存档回环场景。

协议 1.44，内容包继续为 1.37.0，content hash `b37398cb9d005302c958a9e300d07a435e8631d6a5cd44ba63b0086069577c43`，terrain 27。save 容器仍为 v1，state-hash 升至 Schema v17，active baseline 共 86 个 exact fixtures。

## 与原版 RFB 对比

相同点：原版 quest 结构自身保存 status、goal_current、goal_count 等状态，`quests_on_kill_mon()` 等回调根据领域事实推进任务，而不是从地图入口外观反推任务结果。本实现同样建立独立任务状态并由击杀/拾取事件推进。

主动差异：重构使用排序稳定、可序列化的 `TaskState` 集合，并在每次权威 dispatch 后集中消费强类型领域事件。原版通过多个专用 `quests_on_*` 回调和全局 `_current` quest 修改任务结构，任务更新入口更分散。

暂未实现：任务状态机仍位于游戏聚合内部，尚未拆成独立 quest 模块；多阶段目标、任务接取来源、超时、脚本回调、失败惩罚和多任务追踪选择仍未建立。
