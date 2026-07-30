# Contract v142：Stone Skin 药水

状态：已实现（P92）

Contract v142 接入原版药水 tval 75 / sval 69 的 Stone Skin 行为。协议保持 `1.121`，demo 内容包为 `1.133.0`，save 容器保持 v1，state hash Schema 保持 `54`。active baseline 包含 446 条 exact fixtures、零 waiver，内置内容 hash 为 `48611b108dafc4b06836073ca6b5c6881779c653cbab569a7fdeaec82c1c707a`。

## 1. 效果与事务

固定原版 `191f48c3` 的 `src/devices.c` 每次先计算 `_potion_power(20 + randint1(20))`，再调用 `set_shield`；`src/effects.c` 只在新值更长时替换既有计时，不累加，并且只有从无状态变为有状态时返回可察觉。当前纵切增加 self-only 的窄静态消耗品效果 `apply-stone-skin { durationDice, durationSides, durationBonus }`，demo 的原创 Granite Tonic 与 legacy importer 都使用 `1d20 + 20`。

合法使用先写入 Tried 并消费药水，再抽一次持续时间，以 KeepStrongest 应用 `rfb.status.stone-skin`。状态按饮用时玩家等级写入 `defense = 10 + 40 * level / 50`；既有状态的更长刷新只替换剩余时间，不累加。首次新增状态才将来源药水升级为 Aware；已有状态时即使刷新更长仍保持无新效果。所有合法分支支付既有 100 energy；错误目标继续由既有物品计划器在消费、时间和 RNG 前拒绝。

## 2. 协议、界面与内容边界

状态、防御派生、存档和 state hash 均复用既有权威结构，本轮没有新增协议 DTO、命令、存档字段或 state-hash Schema。内部 `ItemStoneSkinResolved` 只投影普通 `GameEventDto`：首次新增为 `item.use-stone-skin-applied` / `item-use-stone-skin-applied`，重复使用为 `item.use-stone-skin-no-new-effect` / `item-use-stone-skin-no-new-effect`；两者都携带来源、显示键与本次骰出的持续时间。Web 只增加对应展示分支和中英消息。

内容效果只能用于静态 consumable，不能用于动态 activation、充能物品或设备检定。本轮不增加通用 item `apply-status`、药水 resolver、职业覆盖表、药水倍率框架或第二套防御管线。

## 3. 导入与契约

legacy importer 将 tval 75 / sval 69 映射为 `apply-stone-skin`。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 严格导入成功，解析诊断为零，`consumable-effect` 从 70 降至 69，`scroll-effect` 保持 15；真实包保持 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races，源码校验、编译与二进制回读 hash 均为 `845faf23ab10df14f22dbf5c14481db63385e210011d548ee7bbd18ee5cb4136`。

fixture 446 让 25 级 Vanguard 连续使用两瓶 Granite Tonic，固定两次持续时间为 24、25 ticks。第一回合后既有计时降为 14，第二瓶刷新为 25，行动结算后最终剩余 15 ticks；同时固定首次 Applied/Aware、第二次 No New Effect、defense modifier +30、两次消费、20 world ticks、两次效果 RNG 和事件顺序。导入器既有表测试增加 sval 69；没有增加 save round-trip、过期、错误目标、Schema 负例、Web 单测或 Tauri E2E 断言。

内容 hash 属于 state hash 输入，因此既有 445 条 fixture 只替换 `stateHash` / `saveRoundTripStateHash` 字段；递归去除这些字段后的完整 assertions 与 contract-v141 逐文件一致。

## 4. 明确遗留

- 原版 `_potion_power` 与 Potion Devicemaster 特例等待对应职业纵切，不建立通用药水倍率框架；
- 原版防御加值会随有效等级重新计算；当前短持续时间状态固定饮用时数值，持续期间升级重算等待统一动态状态修正边界；
- Magic Defense、Kata Musou 与其他来源互斥等待对应状态/职业纵切，不在本效果中建立来源优先级表；
- 原版状态计时的全局 10000-tick 上限等待统一时间状态边界，不在该 resolver 内加专用截断；
- 其他 69 个 `consumable-effect` 与 15 个 `scroll-effect` 继续按独立事务分组。
