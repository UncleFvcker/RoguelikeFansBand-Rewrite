# Contract v43：独立任务 ID 与共享任务范围

状态：协议 1.43 / contract-v43 active baseline

## 已完成边界

- 一次性任务层新增可选 `taskId`。未声明时继续以 floor ID 作为兼容任务 ID；多个任务层可以声明同一 task ID。
- `TaskStatusDto` 新增 `taskId`。任务日志按 task ID 去重，仍保留代表性的 `floorId` 和名称键用于现有内容显示。
- save v1 的 `taskProgress` 改用 `taskId`；反序列化继续接受旧 `floorId`，并通过当前内容把旧 floor ID 迁移为规范 task ID。
- 回声悬赏新增第二个独立入口和任务层。两个楼层共享 `demo.task.echo-bounty`、`1/2` 进度和最终状态。
- 任一成员楼层中的目标死亡都会推进同一计数；暂停一个成员后可从另一个入口继续任务。
- 完成、失败或放弃共享任务时，核心统一销毁全部成员任务层、转换全部关联入口，并且只发放一次任务奖励。
- 内容编译器要求同一 task ID 的成员具有一致目标、required 和重接策略，并且整组恰好声明一个奖励。

协议 1.43，内容包 1.37.0，content hash `b37398cb9d005302c958a9e300d07a435e8631d6a5cd44ba63b0086069577c43`，terrain 27。save v1 与 state-hash Schema v16 不变，active baseline 共 85 个 exact fixtures。

## 与原版 RFB 对比

相同点：原版 quest 拥有独立于当前地牢层的 quest ID，`_current` 只引用当前任务，`goal_current/goal_count` 保存在 quest 本身而不是某个楼层对象中。本实现同样把任务身份和累计进度从单个 floor ID 中拆出。

主动差异：重构允许多个明确声明的入口/任务层组成一个任务组，并在完成时原子关闭全部入口。原版任务与城镇、地牢和层级的关联由 quest 表、生成回调和当前 quest 全局状态共同处理，并没有本实现这种内容声明式的成员入口集合。

暂未实现：同一任务内的上下层连接、阶段顺序、不同成员楼层使用不同目标、共享任务名称/描述实体、任务接取来源，以及多任务并行追踪和选择。
