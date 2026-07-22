# Contract v39：击杀目标与任务进度

状态：协议 1.39 / contract-v39 active baseline

## 已完成边界

- 任务目标改为强类型 `collect-item` 与 `kill-actor`；两类目标分别声明稳定物品实例或 actor 实例。
- 新增一次性讨伐任务层。目标怪物以稳定实例 ID 生成在入口东侧，战斗、死亡移除、携带物和普通掉落完全复用既有实体管线。
- 权威任务日志新增 `current` / `required`。收集目标以目标实例的所有权判断进度；击杀目标以当前任务层中稳定 actor 实例是否仍存在判断进度。
- 退出任务层时用同一进度投影决定完成或失败；成功后关闭入口并在地表生成稳定奖励。
- 前端任务日志显示 `(current/required)`；旧 fixture 缺失字段时按 `0/1` 兼容读取。

协议 1.39，内容包 1.33.0，content hash `328600bfda30da20bd2efe7faac1f97eda03cccecb3ae0b36f4b683e74e5869e`，terrain 17。save v1 与 state-hash Schema v15 不变，active baseline 共 77 个 exact fixtures。

## 与原版 RFB 对比

相同点：原版 `quest.c::quests_on_kill_mon()` 也在目标怪物死亡时推进 `goal_current`，达到 `goal_count` 后调用 `quest_complete()`；两者都把击杀事实接到正常怪物死亡路径，并防止同一怪物重复计数。

主动差异：本纵切锁定一个稳定 actor 实例，进度由权威实体存在性投影，完成结算发生在玩家退出一次性任务层时。原版主要按怪物种族 `goal_idx` 累加数量，并可在最后一次击杀的位置立即完成任务；它还有 questor 标志、随机任务、unique 和多种任务回调。

暂未实现：同种怪物数量击杀、跨楼层累计、清空楼层、unique/随机任务、主动放弃、禁止提前退出、多阶段目标，以及独立持久任务领域状态。这些缺口已记入待实现清单。
