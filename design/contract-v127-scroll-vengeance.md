# Contract v127：Vengeance 卷轴

日期：2026-07-29

Contract v127 接入原版卷轴 sval 50 的 Vengeance。协议 DTO 未变化，继续使用 `1.118`；demo 内容包为 `1.118.0`，save 容器保持 v1，state hash Schema 保持 `52`。active baseline 包含 430 条 exact fixtures、零 waiver，内置内容 hash 为 `c920d9f1b78d5f51a8ebb1097a54c1f74efe7b4a83eb469809b2c3e60d9717d3`。

## 1. 效果边界

内容层新增 self-only 的 `vengeance { durationDice, durationSides, durationBonus }` 物品效果。当前卷轴固定施加 `rfb.status.vengeance`，持续时间为 `25+1d25`，并使用 `KeepStrongest`；不暴露反伤比例、每次持续时间成本、伤害类型或触发来源过滤等当前内容不需要的配置。

Vengeance 只响应敌对怪物直接造成的玩家实际 HP 损失。怪物完整 melee blow routine 结束后合计一次，怪物完整 spell cast 结束后合计一次；零实际伤害和玩家死亡均不触发。反击伤害等于本次实际 HP 损失，不抽 RNG、不经过目标抗性，每次触发额外扣除 5 status ticks。

反击击杀复用既有 `resolve_actor_death`，因此经验、掉落、任务和 guardian 事务保持同一入口。触发时找不到对应来源 actor 属于核心不变量错误，不伪装成普通无效果结果。

## 2. 事务、知识与 RNG

目标必须是自身，错误目标在消费、world tick 和 RNG 前拒绝。合法使用消费卷轴后抽一次持续时间骰并无条件变为 Aware；效果本身没有无候选或无空间分支。

状态进入既有玩家 status 存档、回放和 state hash，不增加新的 save 字段。怪物近战和施法只在各自完整结算边界记录玩家 HP 前后差值，随后调用同一个窄反击 helper；没有建立通用伤害监听器、回调总线或 `AbilityEffectDefinition` 变体。

## 3. 导入与契约

legacy importer 将 tval 70 / sval 50 映射为 `vengeance { durationDice: 1, durationSides: 25, durationBonus: 25 }`。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 严格导入成功且零解析诊断，`scroll-effect` 从 21 降至 20。真实包保持 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races；源码校验、编译与二进制回读 hash 均为 `2178aea924ffe39476e2c89c668e13a98555b2f8a41d9315aa9630b32d0f4afc`。

fixture 430 固定 `25+1d25` 的持续时间骰、完整双 blow 仅按总实际伤害反击一次、5 tick 反击成本、卷轴消费、Aware、RNG 计数和存档回读。一个窄核心组合单测覆盖怪物 spell 反击和玩家死亡抑制；导入器既有卷轴表测试增加一个 sval 50 行。

既有 429 条 fixture 只更新 `stateHash` 与 `saveRoundTripStateHash`。

## 4. 明确遗留

- Protection from Evil、Monster Confusion、Understanding、Inventory Protection 和其他剩余卷轴继续独立分组；
- 玩家反伤目前只覆盖怪物 melee 与 monster ability 的直接 HP 损失，不建立环境、状态伤害、陷阱或玩家自身反噬的通用触发系统；
- 原版 `psion_backlash`、Eye for Eye 以外的职业/姿态来源、骑乘和投射物/地形副作用不在本轮；
- 剩余 20 个 `scroll-effect` 继续按世界/地形、状态和物品/成长事务拆分。
