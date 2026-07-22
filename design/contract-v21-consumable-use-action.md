# Contract v21：消耗品 UseAction 与可观察鉴定

状态：协议 1.21 / contract-v21 active baseline

## 已建立边界

- 物品内容可声明 `useAction.effect`；首个效果为固定整数治疗，并复用核心既有 `EffectSpec::Heal` 与结构化 `EffectOutcome`，不在物品命令中另写治疗公式。
- `UseItem { itemId }` 只解析当前背包中的稳定实例 ID。合法使用在同一权威事务中消耗 1 件、执行效果、更新知识并支付标准行动成本；不可用或已经离开背包的实例不消耗物品，并输出结构化不可用事件。
- 合法使用总会把带外观名称的种类标记为 `tried`。只有实际治疗量大于零、玩家能够观察到效果时才标记为 `aware`；满血使用仍消耗 1 件，但继续显示未知外观名称。
- 原创发光碎片现在提供 4 点治疗。受伤后使用会显示真实名称并公开其既有投掷 profile；满血使用只变为 tried，隐藏属性仍不投影。
- HTML 背包只消费 `InventoryItemDto.usable` 决定“使用”按钮是否可用，不接收隐藏效果定义或治疗数值。使用事件携带核心决定的 `nameKey`，即使最后一件被消耗也不会由前端从 `kindId` 猜测名称。

## 协议、内容、存档与基线

- 协议升至 1.21，新增 `UseItem`、`InventoryItemDto.usable`、`HealingResolutionDto` 和 `GameEventOutcomeDto::Heal`。
- 原创内容包升至 1.16.0，新增 `ItemUseActionDefinition` 与首个 `heal` effect；1.15.0 content hash 加入显式内置存档迁移白名单。
- save schema 继续为 v1，没有新增字段。消耗复用既有物品实例/数量，鉴定复用 contract-v20 的 `itemKnowledge`。
- state hash Schema 继续为 v10：HP、物品数量和知识状态都已在该权威投影内；规则变化通过协议、内容 hash 和 contract 版本显式固定。
- contract-v21 从 v20 迁移 57 个 exact fixtures，并新增受伤使用治疗物品后成为 aware 的场景，共 58 个。

## 后续

Stage D 的实例级物品属性已由 [contract-v22](contract-v22-instance-affix-knowledge.md) 建立，质量鉴别与完整识别已由 [contract-v23](contract-v23-item-appraisal.md) 建立，确定性掉落已由 [contract-v24](contract-v24-deterministic-loot-generation.md) 建立，怪物携带物已由 [contract-v25](contract-v25-monster-carried-items.md) 建立。阶段 E 的楼层生命周期骨架已由 [contract-v26](contract-v26-floor-lifecycle.md) 建立。
