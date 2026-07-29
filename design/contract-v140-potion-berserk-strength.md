# Contract v140：Berserk Strength 药水

状态：已实现（P90）

Contract v140 接入原版药水 tval 75 / sval 33 的 Berserk Strength。协议保持 `1.121`，demo 内容包为 `1.131.0`，save 容器保持 v1，state hash Schema 保持 `54`。active baseline 包含 444 条 exact fixtures、零 waiver，内置内容 hash 为 `de5986a0133867854afb49f98e06a294528d9e4360bc88e7a0fa78d48fff8846`。

## 1. 效果与事务

固定原版 `191f48c3` 的 `src/devices.c` 先计算 `_potion_power(25 + randint1(25))`，再按顺序调用 `set_shero(p_ptr->shero + dur, FALSE)` 与 `hp_player(30)`；状态首次可察觉或实际治疗任一成立都会令物品可识别。当前纵切增加 self-only 的窄静态消耗品效果 `apply-berserk-strength { durationDice, durationSides, durationBonus }`，demo 的原创 Fury Draught 与 legacy importer 都使用 `1d25 + 25`。

合法使用先写入 Tried 并消费药水，再抽一次持续时间，以 Extend 应用 `rfb.status.berserk`。状态固定授予 max HP +30、defense -10、melee skill +12、melee damage `3 + player level / 5`、ranged skill -12、throwing -20、device -20、saving throw -30、stealth -7、search -15、perception -15、digging +30 与 Fear 免疫。状态事件写入后才复用既有物品治疗路径恢复 30 HP，因此首次应用会先扩大生命上限，再让治疗填充新增容量。

首次新增 Berserk 或治疗实际恢复 HP 任一成立都将来源药水升级为 Aware。已有 Berserk 且满血时仍抽取并延长状态，但只保留 Tried；已有 Berserk 但受伤时，延长本身不提供识别依据，实际治疗会令药水 Aware。所有合法分支支付既有 100 energy；错误目标继续由既有物品计划器在消费、时间和 RNG 前拒绝。

## 2. 协议、存档与界面

Berserk 状态、派生加值、状态免疫、存档和 state hash 均复用既有权威结构，本轮没有新增协议 DTO、命令、存档字段或 state-hash Schema。内部 `ItemBerserkStrengthResolved` 只投影普通 `GameEventDto`：首次新增为 `item.use-berserk-strength-applied` / `item-use-berserk-strength-applied`，重复延长为 `item.use-berserk-strength-no-new-effect` / `item-use-berserk-strength-no-new-effect`；随后沿用既有 `item.use-heal` 或 `item.use-no-effect`。Web 只增加对应展示分支和中英消息。

内容效果只能用于静态 consumable，不能用于动态 activation、充能物品或设备检定。本轮不扩展 item `sequence`，不增加通用 `apply-status`、职业覆盖表、药水倍率框架或第二套治疗事务。

## 3. 导入与契约

legacy importer 将 tval 75 / sval 33 映射为 `apply-berserk-strength`。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 严格导入成功，`consumable-effect` 从 73 降至 72，`scroll-effect` 保持 15；真实包保持 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races，源码校验、编译与二进制回读 hash 均为 `b143ba1a8198e280fbedfdb595088e9b572ef830731eed7ee101d6ce9f80ac0d`。

fixture 444 让满原始生命的一级 Vanguard 使用一瓶 Fury Draught，固定持续时间 49 ticks、行动后剩余 39 ticks、max HP 33→63、治疗 30 填满新上限、完整 Berserk 派生、一次效果 RNG、消费、Aware 以及状态先于治疗的事件顺序；该场景不做 save round-trip。一个表驱动核心测试只覆盖已有 Berserk 时的两条知识边界：受伤时因治疗变为 Aware，满血时仅延长并保持 Tried。导入器既有表测试增加 sval 33；没有增加 save、Schema 边界、错误目标、Web 单测或 Tauri E2E。

内容 hash 属于 state hash 输入，因此既有 443 条 fixture 只替换 `stateHash` / `saveRoundTripStateHash` 字段；去除这些字段后的完整 fixture 投影保持不变。

## 4. 明确遗留

- 原版 `_potion_power` 等待对应药水职业纵切，不建立通用药水倍率框架；
- Alchemist、Berserker 与 Beorning 的特殊状态/职业行为等待相应职业纵切，不向当前状态或内容 Schema 塞入无消费者参数；
- 原版状态计时的全局 10000-tick 上限等待统一时间状态边界，不在该药水 resolver 内加专用截断；
- 其他 72 个 `consumable-effect` 与 15 个 `scroll-effect` 继续按独立事务分组，不与本轮合并；
- contract-v139 后的 Blood importer 维护保持原有记录，不为填补阶段编号而重命名或改写历史。
