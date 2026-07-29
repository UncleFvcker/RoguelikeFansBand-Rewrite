# Contract v133：Slowness 药水

日期：2026-07-29

Contract v133 接入原版药水 tval 75 / sval 4 的 Slowness。协议保持 `1.121`，demo 内容包为 `1.124.0`，save 容器保持 v1，state hash Schema 保持 `54`。active baseline 包含 436 条 exact fixtures、零 waiver，内置内容 hash 为 `5ef19e0ecaf7328a7eb4ef3ff69ca066858ca0cc718c6b2db84b078e281f2404`。

## 1. 效果与事务

内容层增加 self-only 的窄静态消耗品效果 `apply-slowness { durationDice, durationSides, durationBonus }`。demo 与 legacy importer 都固定为 `1d25 + 15`；效果不能用于动态 activation、充能物品或设备检定，也不扩展 `AbilityEffectDefinition` 或通用物品状态 DSL。

合法使用先写入 Tried 并消费药水，再总是抽取一次持续时间。玩家免疫 Slow 时不写状态；否则通过既有 `apply_status` 以 `KeepStrongest` 合并 `rfb.status.slow`。首次新增状态才视为原版 `set_slow` 可察觉结果并升级为 Aware；已有 Slow 时，更长结果可以刷新持续时间，但仍返回 no-effect 事件并保持 Tried-only。更短/相等结果和免疫同样只记 Tried。错误目标沿用物品计划器，在消费、时间与 RNG 前拒绝。

物品动作继续支付既有 100 energy；首次减速后，调度器按新的速度等待玩家再次可操作，因此 fixture 中该动作推进 20 world ticks。状态在等待期间同步递减，不增加第二种时间或状态结算路径。

## 2. 协议、存档与界面

Slow 已存在于权威状态存档、快照和 state hash，本轮没有新增协议 DTO、命令、存档字段或 Schema 版本。内部 `ItemSlownessResolved` 事件投影为普通 `GameEventDto`：首次应用使用 `item.use-slowness-applied`，其余可用结果使用 `item.use-slowness-no-effect`；Web 只增加对应本地化展示分支。

## 3. 导入与契约

legacy importer 将 tval 75 / sval 4 映射为 `apply-slowness`。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 严格导入成功，`consumable-effect` 从 81 降至 80，`scroll-effect` 保持 15；真实包保持 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races，源码校验、编译与二进制回读 hash 均为 `d13e08a4feccd9717bac5eeab937f81266cad791e7ca53d8ca631abf88fe5764`。

fixture 436 固定首次应用的 `1d25 + 15`、Slow 状态、药水消费、Aware、一次效果 RNG 与存档回读。一个聚焦核心单测覆盖已有 Slow 的更长刷新、一次 RNG、消费和 Tried-only；导入器既有表测试增加 sval 4。没有增加 Web 测试、Tauri E2E、save 专项测试、免疫矩阵或通用状态测试。

## 4. 明确遗留

- 非生命、种族和装备提供的 Slow 免疫继续由既有 `player_status_immunities` 汇总，不增加药水专用免疫字段；
- 本轮不实现食物营养、其他增益/减益药水、物品损坏或通用状态序列；`consumable-effect` 剩余 80；
- 剩余 15 个 `scroll-effect` 仍等待 lighting、trap、glyph、loot 或物品重写等各自系统，不与本轮合并；
- 原版 virtues、自动鉴定、Devicemaster 特例和非物品入口继续显式保留。
