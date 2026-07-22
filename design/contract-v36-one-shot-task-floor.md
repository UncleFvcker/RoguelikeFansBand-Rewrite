# Contract v36：一次性任务层

状态：协议 1.36 / contract-v36 active baseline

## 已完成边界

- `ProceduralFloorDefinition.lifecycle` 支持 `dungeon` 和 `one-shot`；返回地表的楼层通过稳定 `entryTerrainId` 选择，不再依赖“第一座返回地表的楼层”。
- 地表新增 `demo.terrain.task-rift`。玩家站在入口上提交既有楼层切换命令后，首次生成独立 `demo.floor.echo-task-rift`。
- 任务层在进入期间与普通楼层一样可保存、回放和恢复；从其上楼梯退出时不触发普通地牢实例清理，而是只销毁该任务层。
- 退出后，地表入口按内容声明转换为 `demo.terrain.task-rift-closed`，输出 `floor.one-shot-closed`；再次提交切层命令只得到不可用事件。
- 完成状态不另设平行布尔字段，而由权威地表 terrain 转换保存并进入现有 state hash。

协议 1.36，内容包 1.30.0，content hash `738d40e03f4c4eaebb91d47c74ad7decd7c13ddd12cc41238d177408f66ea0cf`，terrain 13。save v1 与 state-hash Schema v15 不变，active baseline 共 74 个 exact fixtures。

## 与原版 RFB 对比

相同点：原版任务/竞技场等特殊区域可使用 `CFM_NO_RETURN`，离开时销毁当前 saved floor；任务入口和任务状态决定能否再次进入。任务层与普通地牢层使用不同的生命周期边界。

主动差异：重构把入口、目标楼层、生命周期和关闭后的 terrain 全部声明为稳定内容引用，并用地表 terrain 转换作为持久真值。原版任务状态由独立 quest 结构和大量特殊分支维护。

暂未实现：真正的目标完成/失败状态、奖励、任务日志、重接规则、固定任务地图、禁止提前退出，以及完成与放弃对应的不同入口结果。
