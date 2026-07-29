# Contract v123：Mana 卷轴

日期：2026-07-29

Contract v123 接入原版卷轴 sval 61 的 Mana。协议 DTO 未变化，继续使用 `1.118`；demo 内容包为 `1.114.0`，save 容器保持 v1，state hash Schema 保持 `52`。active baseline 包含 426 条 exact fixtures、零 waiver，内置内容 hash 为 `db5233e09952166a195617182db8020cfacc457e2279d0ff403f16a941c49db2`。

## 1. 范围

Mana 卷轴复用 contract-v122 的 self-only `self-centered-elemental-blast`：以玩家为中心造成 1100 点 mana 基础伤害、半径 4，并按 `(base + distance) / (distance + 1)` 衰减。墙体继续阻断 line of effect，actor 伤害继续经过目标当前 Mana 抗性、死亡、经验和掉落管线。

效果只新增一个必填布尔字段 `backlashUsesResistance`。Fire/Ice 显式为 `true`，Mana 显式为 `false`。Mana 的玩家反噬固定为 `50+1d50` mana；它以 `ResistanceLevel::Normal` 进入既有玩家入伤管线，因此忽略玩家 Mana 抗性，但仍保留已有 incoming-damage 百分比。没有新增效果枚举、协议 DTO、存档字段、计划器或通用抗性穿透框架。

## 2. 事务、知识与 RNG

目标必须是自身；错误目标仍在消费、world tick 和 RNG 前拒绝。合法使用先记 Tried 并消费一张卷轴，再标记 Aware、结算中心范围 actor，最后抽取一次玩家反噬骰。actor 结算完成后才处理反噬；玩家因反噬死亡时继续使用已有 Death outcome。

即使没有 actor、actor 全部免疫或玩家有 Mana 免疫，卷轴仍消费、推进时间、变为 Aware，并保留一次反噬 RNG。`backlashUsesResistance` 只选择反噬进入 `resolve_damage` 时的抗性档，不绕过 incoming-damage 修正，也不改变 actor 侧抗性。

## 3. 导入与契约

legacy importer 以表式映射接入 tval 70 / sval 61。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 严格导入成功，`scroll-effect` 从 26 降至 25。

真实包包含 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races；源码校验、二进制编译与产物回读 hash 均为 `745204c6290b7cc64d5a5eda1783bb4212b43a74d932aa822799c46301fe03a5`。

新增 fixture 426 让玩家声明 Mana immune，并让距离 4 的 actor 声明 Mana resistant。一次使用固定 actor 原始伤害 220、抗性后伤害 110，玩家反噬以 Normal 抗性造成 75 点并致死，同时固定消费、Tried + Aware、一次反噬 RNG 和事件顺序；不增加 save round trip。既有 425 条 fixture 只因内置 content hash 输入更新 state hash，其他 assertions 零变化。

## 4. 明确遗留

- 原版 `_scroll_power(1100)` 会受设备 power 修正；当前静态卷轴固定基础数值；
- 原版 Devicemaster Scrolls 会完全跳过玩家反噬；当前没有该职业特例；
- 原版 `fire_ball` 的物品、地形和元素投射副作用尚未建立；本轮只结算 actor 与玩家反噬；
- 剩余 25 个 `scroll-effect` 继续按世界/地形、状态和物品/成长事务拆分。
