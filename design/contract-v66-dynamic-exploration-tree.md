# Contract v66：动态楼梯目标与探索树

状态：协议 1.66 / contract-v66 active baseline

Contract v66 让一座仍在进行的 dungeon instance 在生成楼层时确定性解析动态楼梯目标。内容连接可声明多个加权候选；同一层的动态连接优先选择不同目标 floor，避免两座楼梯无意中合并到同一分支。解析结果写入运行时 `FloorConnectionState`，因此上下楼、存档回读和回放都使用同一棵实例级探索树。

## 连接模型

`ProceduralFloorConnectionDefinition` 保留原有 `targetFloorId`/`targetConnectionId` 固定目标，并可增加 `targetCandidates`。每个候选包含目标 floor、目标连接和正权重。候选经内容包校验，目标深度、连接类型、生命周期、地牢归属和地形方向必须合法。

楼层生成完成后才消费动态候选的 RNG，避免改变该楼层的房间、地貌、怪物和掉落布局。候选按稳定连接 ID 顺序无放回选择；候选耗尽时允许确定性回退到完整候选集。到达动态目标时，目标楼梯的返回连接会原子地写入实际出发连接，避免不同父分支共享模板时回到错误的楼梯。

## 持久化与兼容

`FloorConnectionSaveDto` 增加可选 `targetFloorId` 与 `targetConnectionId`。v65 及更早存档缺失这两个字段时，不重建楼层、不推进 RNG，运行时沿用内容中的固定目标；新生成楼层保存解析后的目标。解析目标进入 state hash Schema v25，协议升至 1.66，save 容器仍为 v1。

普通 dungeon 返回地表仍立即清理当前实例及其离层 floor；再次进入总是分配新的 `.instance.N` 并重新生成，因此 v66 不引入常驻的暂停 dungeon、实例选择 UI 或跨实例传送。

## Demo 与验收

demo echo depth 1 的两座普通楼梯各有 echo depth 2 与 mirror 的加权候选，并按无放回规则形成不同分支。contract-v66 在 v65 的 131 个 exact fixtures 基础上增加动态连接 fixture，共 132 个 exact fixtures、0 waivers。

核心测试覆盖：

- 同 seed 的解析目标、连接位置和 state hash 完全一致；
- 同层动态楼梯不会选择同一目标 floor（候选不足时才回退）；
- 动态目标写入 save 并在回读后保持一致；
- 缺失目标字段的旧存档使用固定内容目标，不推进 RNG；
- 目标 floor 的 arrival connection 被实际父连接原子修正。

## 明确不包含

同一 dungeon 的暂停实例并行访问、显式实例选择、TTL/自动淘汰、跨实例传送和运行时 Vault 破坏后的动态重连。这些与“返回地表即清空普通地牢”的生命周期规则冲突，暂不纳入当前协议。
