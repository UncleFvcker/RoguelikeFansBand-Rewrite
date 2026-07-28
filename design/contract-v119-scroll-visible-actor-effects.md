# Contract v119：可见目标驱散与放逐卷轴

日期：2026-07-29

Contract v119 接入原版卷轴 sval 42/62：Dispel Undead 与 Banishment。协议 DTO 未变化，继续使用 `1.118`；demo 内容包为 `1.110.0`，save 容器保持 v1，state hash Schema 保持 `52`。active baseline 包含 422 条 exact fixtures、零 waiver，内置内容 hash 为 `a9fa7d716f4f5e13ba8f97cb9c72f1dfbb4ed84c83a284b3cde2219549fcb1dd`。

## 1. 范围

物品效果新增 self-only 的 `dispel-category { category, damage }` 和 `banish-visible { maximumDistance }`。两者只接入静态物品使用入口，共用 `ItemUsePlan::VisibleActors` 冻结当前存活、玩家可见且与玩家之间存在 line of effect 的 actor ID。

目标按当前权威实体顺序结算。计划阶段只收集 ID，不预抽伤害、抵抗或落点 RNG；执行阶段重新确认目标仍存活，避免前序死亡副作用使后续索引漂移。本轮不改 `AbilityEffectDefinition`，不增加通用 actor-effect DTO、trait 或效果 DSL，也不接入 Aggravate Monster。

## 2. Dispel Undead

demo 卷轴声明 `category: undead` 和固定 80 点伤害。结算边界为：

- 只影响快照中匹配 category 且没有 `resist-all` 的 actor；
- 伤害不经过普通元素抗性或物理护甲，以既有 `Damage` / `Death` outcome 和 `holy-fire` 类型记录；
- 至少影响一个目标才把物品知识提升为 Aware；
- 没有合格目标时仍消费卷轴并推进时间，只记录 Tried，且不抽 RNG。

死亡继续走统一 actor death、经验、任务和掉落事务。contract 场景使用零经验亡灵，避免无关的连续升级断言；死亡携带物掉落仍保留为真实副作用。

## 3. Banishment

最大距离固定为 150。抵抗按原版 `GF_AWAY_ALL` 顺序逐目标处理：

- guardian 无条件免疫且不抽抵抗 RNG；
- unique 只有同时具有 `resist-teleport` 才直接免疫；
- `resist-all` 也只有与 `resist-teleport` 同时存在时才直接免疫；
- 其余具有 `resist-teleport` 的普通 actor 以 `level > 1d100` 抵抗；没有该 tag 的 actor 不抽抵抗 RNG。

通过抵抗门后才收集该目标的合法落点。候选按 row-major 稳定枚举，先要求距离位于 75–150；没有候选时逐轮扩大最大距离、缩小最小距离，最大值封顶 200。每个实际位移只进行一次有界落点抽取，前一个目标移动后再为下一个目标重算占用。

通过抵抗门即视为效果已被观察；即使最终没有落点，卷轴也进入 Aware。全无目标或所有目标均抵抗时只进入 Tried；无目标路径零 RNG。抵抗与落点不得为所有目标预抽。

## 4. 导入与契约

legacy importer 将 sval 42/62 映射为上述两个效果，并把怪物 `RES_ALL` / `RES_TELE` 映射为 `resist-all` / `resist-teleport` actor tag。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 的 `scroll-effect` 从 34 降至 32。

真实包仍包含 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races；源码校验、编译和二进制回读 hash 均为 `eaf66414ab9d7eda4bac24957b4263e101250ac90b84a3f5cff9d0b9730e1bf7`。

新增 fixtures 421–422，分别固定驱散死亡事务与放逐成功/抵抗顺序。既有 420 条 fixture 只因内置 content hash 输入而机械更新 state hash，其他 assertions 未变化。事件继续复用已有 `Damage`、`Death` 和 `MonsterDisplacement` outcome，因此没有新增协议 DTO、TypeScript 分派或 E2E 场景。
