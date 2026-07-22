# Contract v40：任务放弃与退出限制

状态：协议 1.40 / contract-v40 active baseline

## 已完成边界

- 一次性任务内容新增 `allowEarlyTaskExit`。允许时，未完成目标从楼梯退出仍沿用失败结算；禁止时，未完成目标的普通楼梯退出被权威核心拒绝。
- 协议新增 `abandon-task` 命令。它只在当前楼层是进行中的一次性任务层时有效，并显式绕过退出限制返回地表。
- 任务结果新增 `abandoned`，与 `failed`、`completed` 分离。内容声明独立的 abandoned 入口 terrain，确保销毁任务层后仍能从权威地表状态投影结果。
- 主动放弃销毁一次性任务层、关闭入口且不生成奖励，并输出 `task.abandoned`；前端任务日志为 active 任务提供“放弃任务”按钮。
- 讨伐任务禁止未完成时直接退出；收集任务继续允许直接退出并失败，覆盖两种内容策略。

协议 1.40，内容包 1.34.0，content hash `02df91742a4ad4daf3aebe88c397f0a70396e36f9afc293cd87bdc310715929b`，terrain 19。save v1 与 state-hash Schema v15 不变，active baseline 共 79 个 exact fixtures。

## 与原版 RFB 对比

相同点：原版 `quests_check_leave()` 会在未完成任务离开前警告，普通不可重接任务离开后由 `quests_on_leave()` 调用 `quest_fail()`；`QF_RETAKE` 任务则可以离开并回到 taken 状态。两者都把“离开是否允许”和“离开后的任务结果”作为任务规则，而不是楼梯 UI 自行决定。

主动差异：重构使用明确、可回放的 `abandon-task` 命令和内容布尔策略，不依赖同步确认对话框；主动放弃拥有独立 `abandoned` 状态。原版通常把确认后的离开记为失败，只有部分随机可重接任务在离开时额外询问是否故意失败。

暂未实现：确认弹窗、可重接任务和保留任务层、重新接取、超时失败、死亡/回忆/传送等非楼梯离开统一结算，以及失败或放弃后的惩罚回调。
