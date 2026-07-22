# Contract v38：任务奖励与任务日志

状态：协议 1.38 / contract-v38 active baseline

## 已完成边界

- 一次性任务可声明稳定 `taskReward`。成功退出时，核心在地表入口位置生成固定奖励物品并输出 `task.rewarded`；失败路径不生成奖励。
- 奖励实例使用内容声明的稳定 ID，进入普通地面物品、拾取、存档和回放事务，不建立任务专用奖励容器。
- 协议新增 `TaskStatusDto`，状态为 `available`、`active`、`completed` 或 `failed`。快照和每次更新都输出排序稳定的任务日志。
- 任务日志不复制一份平行可变状态：当前任务层决定 `active`，地表开放/成功/失败入口 terrain 决定其余状态。
- 前端新增任务日志面板并使用 Fluent 显示任务名称和状态。

协议 1.38，内容包 1.32.0，content hash `b44f98cea0cc7f125421faebf3085a23c79228be2573daca38acef63abcca6ea`，terrain 14。save v1 与 state-hash Schema v15 不变，active baseline 共 76 个 exact fixtures。

## 与原版 RFB 对比

相同点：原版任务完成后生成或发放奖励，并由任务结构保存当前状态；玩家可在任务界面查看任务是否进行中、完成或失败。

主动差异：重构首版奖励直接进入普通地面物品管线，任务日志由已有权威真值投影，避免 terrain、任务布尔值和 UI 状态三份数据漂移。原版拥有独立 quest 表、奖励领取流程和大量任务专用显示。

暂未实现：奖励选择与领取确认、容量拒绝、多任务详情和进度数字、历史日志、可重复任务，以及独立任务领域状态。
