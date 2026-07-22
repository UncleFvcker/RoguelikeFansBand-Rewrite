# Contract v42：可重接任务

状态：协议 1.42 / contract-v42 active baseline

## 已完成边界

- 一次性任务内容新增 `retakeable`。未完成时从楼梯离开不会失败或关闭入口，而是把当前任务层保存在 `storedFloors`。
- 任务日志新增 `paused`：地表入口保持开放，累计进度继续显示；再次从同一入口进入后恢复为 `active`。
- 暂离和重入分别输出 `task.paused`、`task.resumed`。两者都经过普通楼层转换事务并进入回放事件序列。
- 重入恢复原任务层的玩家位置、幸存目标、掉落、地形知识和 `taskProgress`，不会重新生成或补满目标。
- 显式 `abandon-task` 仍会永久销毁任务层并关闭入口；达到目标后仍按 completed 路径关闭入口并发放奖励。
- 地表存档只允许保留声明为 retakeable 的一次性任务层；普通地牢离层仍按探索结束规则清除。

协议 1.42，内容包 1.36.0，content hash `7a65a77e6fec214a86be9ba7e6abbbebae14c7a68094b628f55d5960002e0b4f`，terrain 23。save v1 与 state-hash Schema v16 不变，active baseline 共 83 个 exact fixtures。

## 与原版 RFB 对比

相同点：原版 `quests_check_leave()` 对 `QF_RETAKE` 任务允许玩家确认后离开，`quests_on_leave()` 会把任务从 in-progress 退回 taken，而不是立即失败；之后玩家可以重新进入。两者都将暂离与最终失败区分开。

主动差异：重构的 `paused` 是权威可见状态，并原样保存整个任务层及累计进度。原版会调用 `_remove_questors()`，对 unique、普通目标和 saved floor 有专门兼容逻辑，重新进入不保证所有普通实体状态原样保留；部分随机任务还会额外询问是否故意失败。

暂未实现：任务接取确认、多个入口或楼层共享同一任务 ID、重新进入时重建普通目标、重接次数限制、超时与惩罚，以及在暂停状态直接选择最终放弃的地表 UI。
