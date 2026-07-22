# Contract v51：动态 friends/escort 群体与 formation

## 目标

contract-v51 在楼层 actor 总预算之上建立第一版动态遭遇群体：encounter 候选可声明同类 friends、异类 escort 和空间 formation；群体数量与随从 actor 共享显式预算，并在空间不足时按固定顺序缩减或原子回退。

## 内容契约

- `EncounterEntryDefinition.group` 是可选群体定义。`friends` 声明与领袖同种类的 `minCount/maxCount`；`escort` 另含按权重和深度过滤的怪物候选；`formation` 支持 `cluster` 与 `ring`。
- 群体至少有一个最小随从，最多七个随从。friends/escort 计数范围必须有效；escort 候选必须是 monster、权重非零、深度范围有效，并覆盖群体所在 encounter 条目的全部深度。
- `generationBudget.groupPlacements/groupActorSlots` 必须成对出现，范围分别为 1–4 和 1–14。前者限制动态群体数，后者只计算 friend/escort 随从；领袖和随从仍同时计入楼层 `actorSlots`。
- 启用群体预算的 encounter 表必须同时有深度合格的 grouped 与 plain 候选，并为每个群体最小阵容及至少一个普通遭遇预留 actor。第一版群体预算不与 nest 或空间 Vault 预算混用。

## 确定性生成顺序

1. 先预留巢穴、Vault 固定成员和仍存活守护者，再从深度合格的 grouped 候选中按权重选择群体领袖。
2. 为剩余群体最小阵容和一个 plain 遭遇保留预算；friends 数量先抽取，escort 数量后抽取，escort 种类再按成员 ordinal 独立执行深度加权抽取。
3. formation 在指定房间中按领袖位置行优先、八个方向规范序枚举。`cluster` 使用相邻紧凑顺序，`ring` 将成员均匀分布在八邻域。
4. 多个合法 formation 候选只消费一次有界抽取。若空间不足，先将 escort 缩到最小值，再将 friends 缩到最小值；最小阵容仍无法整体放置时，整个 grouped 候选失败，不生成残缺群体。
5. 群体候选失败后按稳定顺序尝试其他 grouped 候选；群体阶段结束后，剩余 `actorSlots` 由 plain encounter 候选填充。

实例 ID 使用 `.encounter.N` 领袖、`.friend.N` 同类成员和 `.escort.N` 护卫成员。terrain、actor、RNG 和 content hash 继续直接进入既有存档与 state hash。

## 原创场景与验证

- 共鸣压力地牢深度 6 使用 ring：1–2 个同类 friends 与 1–2 个 frost/storm escort，在 7 actor、1 group、4 companion slots 内生成。
- 深度 7 使用 cluster：1–2 个同类 friends 与 2–3 个 venom/echo-hound escort，在 8 actor、1 group、5 companion slots 内生成。
- `contract-v51` 从 v50 迁移 100 个 exact fixtures，并新增 ring 与 cluster 两个场景；active baseline 共 102 个 exact fixtures，0 waiver。
- 单元测试覆盖内容范围/角色/预算校验、深度过滤、两种 formation、共享预算、空间压力缩减、原子回退、确定性和 v50 已生成楼层迁移。

## 版本

- 协议：1.51；
- 内容包：1.44.0；
- content hash：`de045e1652d6e484937743b84a98e5e77887f28340a6492e72e8c6e1f72326e6`；
- save：v1，不增加字段；
- state hash：Schema v19，不增加字段；
- active baseline：contract-v51，102 exact fixtures。

## 明确延后

- pit、任意半径/形状 formation、跨房间群体和 pack AI；
- 召唤、繁殖、种群上限、unique/守护者过滤和阵营关系；
- 群体预算与 nest/空间 Vault 同层组合，以及房间、陷阱、门和特殊地形预算；
- Vault 多入口、大模板连通性证明、跨走廊拼接、分支与独立到达点。
