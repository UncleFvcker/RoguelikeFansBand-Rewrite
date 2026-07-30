# Contract v149：P98 组合恢复消耗品

日期：2026-07-30

Contract v149 接入 Restoring Food、Restoring Potion、Ambrosia 与 Life Potion 的组合恢复事务。协议保持 `1.123`，demo 内容包升至 `1.140.0`，save 容器保持 v1，state hash Schema 保持 `55`。active baseline 包含 454 条 exact fixtures、零 waiver；内置内容 hash 为 `cf977b882f1650f641035e1e12b22cca6430106a4992cceefd2e496060f51774`。

## 1. 内容效果

物品效果新增四种窄定义：`restore-all-attributes`、`restore-all-vitality { lifeForceAmount }`、`apply-restorative-feast { healingDice, healingSides }` 与 `apply-life-restoration { healingAmount, lifeForceAmount }`。它们只组合已有属性、经验、生命力、状态和治疗 mutation，不建立通用成长事务、任意状态列表或新的 sequence 框架。

demo 新增原创 Sixfold Provision、Deep Renewal Tonic、Sunlit Feast 与 Vitalis Elixir。legacy importer 映射 tval 80/sval 17–19、40 与 tval 75/sval 39、54；Life 原有的部分恢复序列替换为完整事务。`consumable-effect` 从 46 降至 41，尚未实现的食物营养仍单独计入 `food-nutrition=28`。

## 2. 事务与知识语义

Restoring Food 固定按 STR、INT、WIS、DEX、CON、CHA 恢复当前属性；Restoring Potion 在此基础上恢复当前经验至历史最高经验，并增加 150 生命力、封顶 1000。两者只有实际发生任一恢复时才 Aware，完全无变化仍消费、推进时间并保持 Tried-only。

Ambrosia 先把 Poison 剩余时间减少 `max(current / 5, 100)`，再掷 `15d15` 治疗，随后恢复六维属性和历史最高经验。Life 先恢复历史最高经验并把生命力补至 1000，再清除当前已建模的 Poison、Blindness、Confusion、Stun、Bleeding、Slow 与 Berserk，恢复六维属性，最后治疗 5000。Ambrosia 与 Life 合法使用即 Aware。

经验恢复继续复用等级重算和既有升级事件；属性恢复只在有变化时统一刷新一次 HP、职业资源上限和派生值。除 Ambrosia 的 15 个治疗骰外，四种效果不增加 RNG。

## 3. 事件与 Fixture

四种事务统一投影 `item.use-restoration` / `item.use-restoration-no-effect`，最终权威状态表达具体恢复结果，不在事件中重复状态和属性明细。

fixture 453 固定 Life 清除七种状态、恢复经验/生命力并治疗，随后用 Frailty 制造 Strength 损伤；Restoring Potion 恢复该属性并 Aware，最后无变化的 Restoring Food 只保留 Tried。最终 HP 为 48/48、经验为 25、等级为 3、生命力为 1000，效果 RNG 为零。

fixture 454 固定 Ambrosia 的 15 个治疗骰、Poison 500→380、Strength 恢复、经验 5→25 与等级 1→3；最终 HP 为 30/40，来源物品 Aware。两条 fixture 之外不增加 save/replay、Web 或 E2E 专项断言。

## 4. 导入与版本

固定原版源码导入保持 937 items、122 affixes、1260 abilities、4 ability books、1332 actors、68 races 和 54 classes。真实导入内容 hash 为 `54333ae2cda9df63ceaccc23794f54a66033897630afe44aa2f845fb217807ad`，编译文件 SHA-256 为 `0FA9BAB7854C042E641529E2927A0EC64C7DB93D3C6034FA4F541D7F4965CAA5`。

内容 Schema、中英 Fluent key 和 Web outcome 同步更新。内容 hash 改变，因此旧 fixture 只刷新 state hash 字段；协议 DTO 与 hash 输入结构未改变，协议和 state hash Schema 均不升级。
