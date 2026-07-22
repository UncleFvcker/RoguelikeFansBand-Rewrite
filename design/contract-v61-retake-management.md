# Contract v61：暂停任务管理与确定性重接

## 目标

contract-v42 已能完整保存并恢复可重接任务层，但暂停后的最终放弃只能重新进入任务层处理，也没有重接次数或重建策略。contract-v61 补齐地表管理入口，并把重接限制和任务层重建变成内容驱动的权威规则。

## 内容契约

- `retakeable` 任务可声明 `maxRetakes`，范围为 1–16；未声明表示不限制成功恢复次数。
- `retakeFloorPolicy` 默认为 `preserve-floor`，也可声明 `regenerate-floor`。非可重接任务不能声明这两个字段。
- 相同 `taskId` 的所有成员层必须使用相同的 `retakeable`、`maxRetakes` 和 `retakeFloorPolicy`。
- demo 的 echo bounty 使用 `maxRetakes: 1` 与 `regenerate-floor`；三阶段 echo chain 使用 `maxRetakes: 2` 与默认的 `preserve-floor`。

## 权威语义

- `retakesUsed` 只在 paused 任务成功重新进入成员层时递增；首次接取、暂离和被拒绝的入口尝试都不递增。
- 达到 `maxRetakes` 后，任务保持 paused，入口尝试返回 `floor.transition-unavailable`，不改变楼层、任务状态或 RNG。玩家仍可从地表最终放弃。
- `preserve-floor` 继续恢复完整 `FloorState`，不重新生成、不消费 RNG。
- `regenerate-floor` 在成功重入事务中丢弃该 task 的全部已保存成员层和对应实例知识，保留阶段、进度、玩家携带物及单调实例分配器，再使用当前权威 RNG 生成目标成员层。计数击杀只补当前阶段的剩余目标数量。
- 新命令 `abandon-paused-task { taskId }` 只在世界初始层且目标状态为 paused 时有效；它销毁该 task 的所有已保存成员层、关闭全部成员入口并转为 abandoned。无效请求产生 `task.abandon-unavailable`。

## 存档、协议与前端

- `TaskStateSaveDto.retakesUsed` 保存已使用次数，缺失时按 0 载入；非法超限状态被拒绝。
- `TaskStatusDto` 投影 `retakesUsed/maxRetakes`。任务日志显示有限重接计数，并为 paused 任务提供按 task ID 的放弃按钮。
- v60 存档载入时不会重建任务层或推进 RNG；当前内容的重建策略只在玩家之后显式重入时执行。
- 重接次数进入 state hash Schema v23；save 容器仍为 v1。

## 验证

- 内容测试覆盖零值/超限、非可重接声明和共享成员策略不一致。
- Core 测试覆盖地表放弃、剩余目标重建、进度保留、次数耗尽、非法存档和 v60 无 RNG 漂移迁移。
- active baseline 为 121 个 exact fixtures、0 waiver；新增地表放弃与“重建一次后拒绝”两个完整命令流。

## 版本

- 协议：1.61；
- 内容包：1.54.0；
- content hash：`56fc449617a4c05c12ff11716c14b4f5c680cada9ad86c6ece736b52fa904bc2`；
- save：v1；
- state hash：Schema v23；
- active baseline：contract-v61，121 exact fixtures。

## 明确延后

- 超时、失败惩罚、接取确认和脚本回调；
- 重接后重置任务进度、按目标类型选择性重建，以及玩家手动选择重建策略；
- 分支/并行阶段、同一阶段多目标、任务内部上下层连接和独立 quest 模块。
