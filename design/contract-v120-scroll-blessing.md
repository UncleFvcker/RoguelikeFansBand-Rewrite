# Contract v120：祝福卷轴族

日期：2026-07-29

Contract v120 接入原版卷轴 sval 33–35：Blessing、Holy Chant 与 Holy Prayer。协议 DTO 未变化，继续使用 `1.118`；demo 内容包为 `1.111.0`，save 容器保持 v1，state hash Schema 保持 `52`。active baseline 包含 423 条 exact fixtures、零 waiver，内置内容 hash 为 `b62824da6e34e2f72a367f94b2e46e50e279ba6ac4df88bece81021a156e90ab`。

## 1. 范围

物品效果只新增 self-only 的 `bless { durationDice, durationSides, durationBonus }`。效果固定施加 `rfb.status.blessed`、intensity 1 和 `extend` 堆叠，并授予 defense +5、melee skill +10、ranged skill +10。

本轮复用 `ItemUsePlan::SelfTarget` 与已有状态结算 helper，不新增计划变体、通用状态 DSL、状态注册表或 debug 开关；也不修改 `AbilityEffectDefinition`。Protection from Evil、Vengeance、Monster Confusion、Understanding 和 Inventory Protection 继续留在后续独立纵切。

## 2. 事务与 RNG

计划阶段只验证自身目标，不抽 RNG。物品通过消费门后，执行阶段按 `durationBonus + durationDice d durationSides` 抽取持续时间并添加或延长祝福；正持续时间必定可观察，因此物品知识从 Tried 提升为 Aware。

三条原版基础公式分别映射为：

- Blessing：`6 + 1d12`；
- Holy Chant：`12 + 1d24`；
- Holy Prayer：`24 + 1d48`。

物品事件使用专用 `item-use-blessed` code，但 outcome 复用已有 `AbilityEffectsResolutionDto`，因此不增加协议 DTO。事件携带按物品知识投影的显示 key，Web 不直接以 content ID 泄漏未鉴定名称。

## 3. 导入与契约

legacy importer 以表式映射接入 sval 33/34/35。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 的导入零解析诊断，`scroll-effect` 从 32 降至 29。

真实包仍包含 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races；源码校验后的编译摘要 hash 为 `b008570c950fab4541286f1eccd86926f1c535cc0dea0770f038cca523b4e643`。

新增 fixture 423 连续使用两次 demo Benediction Scroll，固定两次持续时间抽取、`added`→`extended`、剩余持续时间、defense/melee/ranged 加值、两次消费、Tried→Aware 与 action tick 顺序。既有 422 条 fixture 只因内置 content hash 输入而机械更新 state hash，其他 assertions 未变化。
