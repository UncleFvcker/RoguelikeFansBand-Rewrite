# Contract v132：Spell 卷轴

日期：2026-07-29

Contract v132 接入原版卷轴 tval 70 / sval 43 的 Spell。协议保持 `1.121`，demo 内容包为 `1.123.0`，save 容器保持 v1，state hash Schema 升至 `54`。active baseline 包含 435 条 exact fixtures、零 waiver，内置内容 hash 为 `25d972db57c825d4e23f5a61532c00579f9467acbe10edf97f2c0600b00514f5`。

## 1. 效果与职业资格

内容层为 Class 增加默认 false 的 `usesSpellScrolls`，并增加无参数、仅供静态消耗品使用的 `increase-spell-learning-capacity`。效果固定增加 1，不开放 amount、概率、上限或职业覆盖字段，也不扩展 `AbilityEffectDefinition` 或通用成长管线。

学习总容量按 `min(既有等级/属性公式, learningCapacityCap) + bonusSpellLearningCapacity` 计算。符合资格的职业使用后将 bonus 饱和增加 1；不符合资格的职业仍消费卷轴、推进 10 ticks、写入 Tried + Aware，并产生明确的 no-effect 事件。两条路径都不抽效果 RNG。

## 2. 存档、协议与界面

`PlayerSaveDto` 增加默认 0、零值省略的 `bonusSpellLearningCapacity: u16`。旧存档缺字段迁移为 0；非零值要求当前 Class 的 `usesSpellScrolls` 为 true，否则载入时以具体无效存档错误拒绝。该字段进入权威存档与 state hash，因此 Schema 升至 54；save 容器版本不变。

快照继续使用既有 `AbilityLearningDto.capacity` 与 `remainingSlots`，没有增加前端 DTO、命令或专用操作流程。唯一新增事件 `item.use-spell-learning-capacity-increased` / `no-effect` 报告使用前后的总学习容量。

## 3. 导入与契约

legacy importer 将 tval 70 / sval 43 映射为 `increase-spell-learning-capacity`，并按固定原版 `class_uses_spell_scrolls()` 排除表写入 Class 资格。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 严格导入成功，`scroll-effect` 从 16 降至 15；真实包保持 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races，源码校验、编译与二进制回读 hash 均为 `6feceb4793b043f03c826cb242a9e182edf49ea2c708fffac31fa8f30daf589d`。

fixture 435 使用 Scholar 固定学习容量 2→3、卷轴消费、知识、单一事件、零效果 RNG 与存档回读。一个聚焦核心单测同时覆盖合格职业、无资格职业的已消费 no-effect 路径，以及无资格职业携带非零 bonus 的损坏存档拒绝；导入器表测试覆盖 sval 43 与代表性的允许/排除职业。

既有 434 条 fixture 只因内容 hash 和 state hash Schema 输入更新各类 state hash 断言。

## 4. 明确遗留

- 原版 `_scroll_power`、Devicemaster Scrolls 专精、virtues 与其他非卷轴入口尚未接入；
- 本轮只增加永久学习容量，不改变能力学习来源、书本可读性、领域限制、遗忘或熟练度；
- 资格属于 Class 定义，不从 casting profile、职业名称或当前可学习书本反推；
- 没有增加通用永久成长 effect、数值参数、公共上限策略或额外 debug 开关；
- 剩余 15 个 `scroll-effect` 继续按世界/地形、状态和物品/成长事务拆分。
