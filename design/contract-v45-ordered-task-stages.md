# Contract v45：有序多阶段任务目标

状态：协议 1.45 / contract-v45 active baseline

## 已完成边界

- 内容格式新增 `taskStages`。一个共享任务由一个成员楼层声明稳定有序的阶段列表，每个阶段绑定目标 `floorId`。
- 阶段目标支持 `collect-item`、`enter-floor`、`kill-actor` 和 `kill-actor-kind`；旧 `taskObjective` 单目标格式继续兼容。
- 权威 `TaskState` 新增零基 `stageIndex`。`current/required` 始终表示当前阶段进度；完成非末阶段后按顺序切换到下一阶段并重置阶段计数。
- `TaskStatusDto` 投影一基 `stage/stages`，前端任务日志显示阶段编号和当前阶段进度。
- `TaskStateSaveDto` 保存阶段索引，save v1 可在任意阶段往返；state-hash 升至 Schema v18。
- 已知旧内置内容的 v44 存档缺少新增阶段任务时，该任务按 Available、第 1/3 阶段迁移；当前内容的任务状态集合缺项仍被拒绝。
- 地表任务入口根据当前阶段的目标楼层进行限制，不能从并列入口跳过阶段。可重接任务跨成员楼层继续使用 paused/resumed 状态。
- 原创示例任务完整覆盖“收集 → 进入指定成员楼层 → 按种类击杀两只目标 → 整组入口结算与奖励”。

协议 1.45，内容包 1.38.0，content hash `0e6cf15310644e7b3eb2f7acb0c18a8b1a7fb08739e981e7492d4079e61ab44a`，terrain 35。save 容器仍为 v1，state-hash 升至 Schema v18，active baseline 共 88 个 exact fixtures。

## 与原版 RFB 对比

相同点：原版 `quest_t` 保存 status、goal、goal_current 和 goal_count，通过 `quests_on_kill_mon()`、`quests_on_get_obj()` 与 `quests_on_leave()` 响应击杀、拾取和离层事实；`QF_RETAKE` 允许任务离开后回到可继续状态。重构版继续沿用“权威状态 + 领域事实回调 + 可重接”的规则结构。

主动差异：原版每个 `quest_t` 只有一个 `goal`，目标类型限于清层、击杀和寻找神器；任务顺序主要由城镇数据串联多个独立任务。重构版允许一个 task 内部声明强类型、有序、可跨成员楼层的多个阶段，并把当前阶段写入存档和 state hash。这是为原创内容建立的扩展，不是对原版数据结构的逐字段复刻。

暂未实现：分支、可选和并行阶段；一个阶段内的多个同时目标；阶段级奖励、失败政策与脚本回调；暂停状态下从地表主动放弃、重接次数限制和重新生成策略；独立 quest 模块与任务接取来源仍未建立。

## Fixtures

- contract-v45 从 v44 迁移 86 个 exact fixtures。
- 新增“收集后推进到进入楼层阶段并存档回读”。
- 新增“收集 → 进入楼层 → 计数击杀 → 完成与奖励”的完整阶段链。
- active baseline 共 88 个 fixtures，0 waiver。
