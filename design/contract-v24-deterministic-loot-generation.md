# Contract v24：确定性战利品生成

状态：协议 1.24 / contract-v24 active baseline

## 已完成边界

- 内容格式新增独立 `lootTables` 根。掉落表声明固定 roll 数、加权物品条目、品质权重和可选词条权重；编译器拒绝零权重、重复条目、悬空引用以及可能生成非法堆叠/词条实例的组合。
- 怪物可通过 `lootTableId` 引用掉落表。死亡结算构造稳定 `LootContext`，保存来源种类、掉落表和死亡格。
- 每个 roll 固定按“物品种类、品质、词条”顺序消耗三次 RNG。普通品质会忽略词条抽取结果，但仍消费该次 RNG，避免分支改变后续随机序列。
- 生成物使用既有 `generated.item.N` 分配器，质量与 affix 立即写入实例真值，并直接进入地面物品、存档、回放和 state hash。
- 近战、射击、投掷和持续状态四条怪物死亡路径都复用同一生成入口；死亡事件之后输出不泄露质量/词条的 `loot.drop` 事件。
- 原创内容包升至 1.19.0。余烬微光引用首个加权掉落表，可生成回声刃或回声护符，并为非普通结果抽取“谐振锋芒”。
- contract-v24 从 v23 迁移 60 个 exact fixtures，并新增固定种子 15 的完整场景：死亡掉落、拾取、鉴别、装备、词条发现和存档往返，共 61 个。

协议升至 1.24，用于拒绝旧客户端/回放按 1.23 解释新的掉落事件与 RNG 语义。save schema v1 和 state hash Schema v12 不变；生成结果、实例分配器、RNG 状态和内容 hash 已由现有投影完整覆盖。

## 下一步

怪物携带物与统一死亡掉落事务已由 [contract-v25](contract-v25-monster-carried-items.md) 建立：出生携带物保存同一实例，死亡时先放下真实携带物，再执行本切片的普通死亡生成。楼层生命周期骨架已由 [contract-v26](contract-v26-floor-lifecycle.md) 建立，深度与房间来源已由 [contract-v27](contract-v27-procedural-room-content.md) 接入 `LootContext`；区域主题和更完整的深度权重继续后补。
