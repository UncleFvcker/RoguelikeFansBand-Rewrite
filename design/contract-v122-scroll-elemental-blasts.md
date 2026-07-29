# Contract v122：火焰与寒冰卷轴

日期：2026-07-29

Contract v122 接入原版卷轴 sval 58/59 的 Fire 与 Ice。协议 DTO 未变化，继续使用 `1.118`；demo 内容包为 `1.113.0`，save 容器保持 v1，state hash Schema 保持 `52`。active baseline 包含 425 条 exact fixtures、零 waiver，内置内容 hash 为 `ab0bcb63b25c6729fd95d5fba97a4f618f7aca4589f3931a9ac149615d6062b5`。

## 1. 范围

物品层新增 self-only 的 `self-centered-elemental-blast`，只声明中心爆发伤害、伤害类型、半径和一次反噬骰。Fire 使用 666 点 fire、半径 4、`25+1d25` fire 反噬；Ice 使用 800 点 ice、半径 4、`30+1d30` cold 反噬。

actor 目标复用既有 RFB 范围格和稳定排序：墙阻断 line of effect，伤害按 `(base + distance) / (distance + 1)` 衰减，再经过目标当前抗性；非 physical 伤害不受护甲减免。目标死亡继续走既有死亡、经验和掉落管线。没有新增 `AbilityEffectDefinition` 分支、通用投射 DSL 或第二套物品计划器。

## 2. 事务、知识与 RNG

目标必须是自身；错误目标在消费、world tick 和 RNG 前拒绝。合法使用先记 Tried 并消费一张卷轴，再标记 Aware、冻结并结算当前中心范围 actor，最后抽取一次反噬骰。即使没有 actor、actor 全部免疫或反噬最终为零，卷轴仍消费、推进时间、变为 Aware，并保留这一次反噬 RNG。

actor 结算完成后才处理玩家反噬。反噬经过玩家当前对应元素抗性和既有 incoming-damage 百分比；致死时使用已有 Death outcome，不增加协议 DTO。结构化事件分别记录爆发目标数、命中/击杀和反噬，Web 只负责本地化已有事件参数。

## 3. 导入与契约

legacy importer 以表式映射接入 tval 70 / sval 58–59。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 严格导入成功，`scroll-effect` 从 28 降至 26。

真实包包含 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races；源码校验、二进制编译与产物回读 hash 均为 `54649044572c7ef0f36e7d078dc338680cab6489cfb29c3f723dbf5a7a5bc280`。

新增 fixture 425 先在墙的一侧使用 Fire，再移动两格使用 Ice。它固定距离 4 衰减、墙阻挡、actor fire immune、ice vulnerable、玩家 fire immune/cold resistant、睡眠唤醒、击杀/经验、两次反噬、Tried + Aware 和 save round trip。既有 424 条 fixture 只因内置 content hash 输入机械更新 state hash，其他 assertions 不变化。

## 4. 明确遗留

- 原版 `_scroll_power` 会受设备 power 修正；当前静态卷轴没有设备 power，先固定基础数值；
- 原版反噬前还有 Devicemaster Scrolls 免疫和 `res_save_default` 门；当前只使用已有玩家元素抗性与 incoming-damage 管线，不提前增加职业特例或第二套抗性检定；
- 原版 `fire_ball` 的物品、地形和元素投射副作用尚未建立；本轮只结算 actor 与玩家反噬；
- 剩余 26 个 `scroll-effect` 继续按世界/地形、状态和物品/成长事务拆分。
