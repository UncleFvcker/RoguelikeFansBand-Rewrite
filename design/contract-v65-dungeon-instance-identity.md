# Contract v65：地牢实例身份与生命周期

状态：协议 1.65 / contract-v65 active baseline

Contract v65 为运行时 dungeon floor 引入显式实例身份。实例属于一次从地表进入某座 dungeon 的探索历次；同一内容楼层模板在再次进入时不会复用旧实例号。

## 实例模型

Game 的当前层携带可选 currentDungeonInstanceId。地表与一次性任务层为 None；dungeon 层使用稳定 ID：

<dungeonId>.instance.<ordinal>

每座 dungeon 的 DungeonStateSaveDto.nextInstanceOrdinal 单调递增。首次进入分配 1，离开地表后再次进入分配 2；同一实例上下楼沿用原 ID。首版不提供同时选择同一模板的多个未结束实例，多 dungeon 并存访问和退休策略仍留给后续切片。

离层 FloorSaveDto 保存 dungeonInstanceId。运行时仓库键使用实例 ID 与 floor ID 的组合，避免不同实例覆盖同一模板楼层；一次性任务和地表继续使用原 floor ID 键。

## 生命周期

从地表进入 dungeon 时先分配实例序号，再按既有确定性生成流程生成根层。连接到子层时只在当前实例内查找或生成目标 floor。返回地表时只清理当前 dungeonInstanceId 的离层 floor、实体、地面物品和对应实例知识，不再调用全局 stored_floors.clear()；其他 dungeon 或任务层保留。

共享守护者征服状态仍属于 dungeon，而不是实例。击败一个镜像会移除所有已保存实例中的其他镜像，且后续实例生成不会重新出现守护者。

## 兼容性与哈希

协议 DTO 使用可选字段兼容 v64 及更早存档。缺失实例字段的 dungeon 当前层确定性迁移到 .instance.1；缺失的 nextInstanceOrdinal 从现有实例 ID 推导，不重建地形、不重抽 RNG。旧一次性任务层保持 None。

state hash 升至 Schema v24，纳入当前实例 ID、离层 floor 实例 ID 和每座 dungeon 的实例序号。协议 1.65、回放元数据和 bindings 同步升级；内容包仍为 1.57.0，已生成 floor 不回填实例字段以外的内容。

## Fixtures 与验收

active baseline 为 131 个 exact fixtures、0 waiver。新增：

- dungeon.instance.reentry：同一地表入口经历两次进入，覆盖实例序号 1/2 与回环存档；
- dungeon.instance.save-migration：生成 dungeon 根层并验证实例字段进入 save round-trip。

rfb-core 额外验证实例级仓库键、回地表只清理当前实例，以及 v64 缺失字段迁移。

## 明确不包含

同一 dungeon 多实例的 UI 选择、并行访问、多 dungeon 进入条件、胜利/退休评分、实例 TTL/淘汰策略、跨实例传送和动态探索树图。它们属于后续 contract。
