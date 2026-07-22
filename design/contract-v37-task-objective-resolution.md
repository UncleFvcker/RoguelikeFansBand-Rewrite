# Contract v37：任务目标与完成/失败

状态：协议 1.37 / contract-v37 active baseline

## 已完成边界

- 一次性任务层可声明稳定的 `taskObjective.itemInstanceId/itemKindId`。生成任务层时，目标物固定出现在入口位置，可由普通拾取事务取得。
- 玩家携带目标物退出任务层时输出 `task.completed`；未携带目标物退出时输出 `task.failed`。两者随后都销毁一次性任务层。
- 成功把入口转换为 `completedEntryTerrainId`，失败转换为独立 `failedEntryTerrainId`。两种结果都由权威地表 terrain 保存并进入现有 state hash。
- 目标判定使用稳定实例 ID，而不是仅按物品种类计数，避免普通同类物品冒充任务目标。
- contract-v37 分别覆盖未取得目标的失败路径和拾取目标后的成功路径，并验证关闭入口与存档往返。

协议 1.37，内容包 1.31.0，content hash `c390fb30dcc041b266ee895e72441cf656dbacc470a24ba86bd8d7b948be994f`，terrain 14。save v1 与 state-hash Schema v15 不变，active baseline 共 75 个 exact fixtures。

## 与原版 RFB 对比

相同点：原版任务系统拥有明确目标和完成/失败状态；`quest_complete()` 与 `quest_fail()` 分别改变任务状态并输出反馈，离开特殊任务区域后根据结果处理入口与后续流程。

主动差异：重构首个目标只实现稳定实例物品的取得，并在退出事务中统一判定。原版支持击杀、数量、神器、随机任务及大量任务专用回调；重构先用内容定义、普通物品事务和 terrain 结果建立可回放主链。

暂未实现：击杀/计数/护送/多阶段目标、主动放弃、禁止退出、奖励和任务日志、重接/重复任务，以及目标物离开背包后的复杂状态。
