# Contract v76：学习容量与主动遗忘

状态：协议 1.76 / contract-v76 active baseline；内容包 1.68.0；state hash Schema v34

## 目标

v76 在 v75 的能力熟练度、统计和冷却之上，补齐 RFB 原版“可学习数量”边界，并提供显式遗忘命令。法力容量仍是资源池属性，不与能力学习容量复用。

## 学习容量

Class 的 \`castingProfile\` 新增四个内容字段：

- \`baseLearningCapacity\`：等级 1 的基础槽位；
- \`learningCapacityPerLevel\`：每升一级增加的槽位；
- \`learningCapacityPerAttributeIndex\`：施法属性每个 RFB 属性桶增加的槽位；
- \`learningCapacityCap\`：最终槽位上限。

运行时使用确定性纯函数：

~~~text
capacity = min(
  learningCapacityCap,
  baseLearningCapacity
    + learningCapacityPerLevel × (level - 1)
    + learningCapacityPerAttributeIndex × castingAttributeIndex
)
~~~

已学能力数量不得超过容量。demo Mage 在等级 1 拥有 2 个槽位、3 个候选能力；等级与属性修正仍由内容决定。

\`PlayerDto.abilityLearning\` 暴露 \`learnedCount\`、\`capacity\` 和 \`remainingSlots\`。每个 \`AbilityDto\` 的 \`canStudy\` 在等级、书本和容量均满足时才为真。

## 遗忘

新增命令：

~~~json
{ "type": "forget-ability", "abilityId": "..." }
~~~

成功产生 \`ability.forgotten\`；能力不存在、职业不支持或尚未学习时产生结构化 \`ability.forget-unavailable\`。遗忘是普通行动，会推进一个标准世界回合，但不会抽能力或其他额外 RNG。

遗忘只从 \`learnedAbilityIds\` 移除能力。\`abilityProgress\` 中的熟练度、成功/失败次数和冷却保留；重新研习同一能力后立即恢复这些进度，符合原版“忘记法术不清除法术经验”的边界。主动遗忘不要求当前携带书本，重新学习仍需要匹配的能力书。

容量已满的学习拒绝在任何施法/技能 RNG 之前完成，不改变资源、进度或 RNG。重复遗忘、重复学习和容量拒绝都保持稳定的事件顺序。

## 存档与确定性

v75 已有的 \`learnedAbilityIds\` 与 \`abilityProgress\` 字段继续承载 v76 状态，因此 save 容器仍为 v1，state hash Schema 仍为 v34。载入缺少 \`abilityProgress\` 的旧存档时，按当前内容能力初值迁移；已学数量超过当前容量的存档原子拒绝。内容 hash 变化仍通过内置历史 hash 白名单迁移，不重建地图、不生成物品、不推进 RNG。

学习容量是内容与当前角色 progress 的派生投影，不单独写入存档；其规则变化由内容包 hash 锁定。

## 内容与基准

demo 内容包升至 1.68.0，新增 Harmonic Spark，并让 Echo Primer 包含两个候选能力。active baseline 位于 [\`tests/fixtures/contract-v76/scenarios\`](../tests/fixtures/contract-v76/scenarios)，共 186 个 exact fixtures、零 waiver，新增场景覆盖：

- 初始容量/剩余槽位投影；
- 满容量学习拒绝与零 RNG；
- 遗忘、替代学习、重新学习及进度保留；
- 缺少能力进度字段的旧存档兼容与 save round-trip。

## 明确不在 v76

- 自动按等级变化暂时遗忘/记起的完整 \`spell_order\` 模型；
- 随机学习、首次成功奖励；
- 多资源职业、范围/锥形/召唤/地形改变效果；
- 怪物施法与智能能力选择。
