# Contract v41：数量击杀与持久进度

状态：协议 1.41 / contract-v41 active baseline

## 已完成边界

- 任务目标新增 `kill-actor-kind`，按稳定 actor kind 匹配目标，并通过 `required` 声明所需数量。
- 新增回声悬赏裂隙，固定生成两个同类目标；目标继续复用普通战斗、死亡、掉落和实体移除事务。
- 核心在统一死亡事务中更新以 floor ID 为键的权威任务计数，进度封顶于 `required`；实体只能被移除一次，因此不会重复计数。
- save v1 新增可选 `taskProgress`。旧存档缺失时按空进度载入，非法 floor ID、重复记录和超出 required 的数值被拒绝。
- `taskProgress` 进入 state-hash Schema v16。任务日志稳定显示 `0/2`、`1/2` 和完成后的 `2/2`。
- active baseline 新增 `1/2` 存档回环和 `2/2` 完成/奖励两个场景。

协议 1.41，内容包 1.35.0，content hash `9ff7c821379c543d13fc5ee690a84c71fa4267f210381781a54378040a876403`，terrain 23。save 容器仍为 v1，state-hash 升至 Schema v16，active baseline 共 81 个 exact fixtures。

## 与原版 RFB 对比

相同点：原版 `quests_on_kill_mon()` 按目标怪物种族匹配击杀，递增 `goal_current`，达到 `goal_count` 后完成任务，并用 `MFLAG2_COUNTED_KILLED` 防止同一怪物重复计数。本实现同样按怪物种类累计，并在达到要求数量后完成。

主动差异：重构把数量进度作为按稳定 floor ID 排序的显式存档状态，并由统一实体死亡事务更新；重复计数由实体生命周期保证。原版将计数直接保存在 quest 结构，并额外处理召唤物、unique、随机任务和复活限制。

暂未实现：跨多个楼层共享同一任务计数、可重接任务中的离开后继续累计、召唤物过滤、unique/随机任务、清空楼层目标，以及多种目标组成的多阶段任务。
