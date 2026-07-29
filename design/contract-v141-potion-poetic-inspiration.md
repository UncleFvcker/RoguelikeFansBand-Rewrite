# Contract v141：Poetic Inspiration 药水

状态：已实现（P91）

Contract v141 接入原版药水 tval 75 / sval 14 的 Mead of Poetry 行为。协议保持 `1.121`，demo 内容包为 `1.132.0`，save 容器保持 v1，state hash Schema 保持 `54`。active baseline 包含 445 条 exact fixtures、零 waiver，内置内容 hash 为 `6ecb079e1a1dd1e653e7c4d201f264d72e7c1db9bfe466f8d1ffa410cfee36e0`。

## 1. 效果与事务

固定原版 `191f48c3` 的 `src/devices.c` 每次抽取 `100 + 1d100`，再把结果加到既有 Poetry 计时；`src/files.c` 在状态激活期间提供 Wisdom +5 与 Charisma +5。当前纵切增加 self-only 的窄静态消耗品效果 `apply-poetic-inspiration { durationDice, durationSides, durationBonus }`，demo 的原创 Muse Tonic 与 legacy importer 都使用 `1d100 + 100`。

合法使用先写入 Tried 并消费药水，再抽持续时间，以 Extend 应用 `rfb.status.poetic-inspiration`。首次新增状态才将来源药水升级为 Aware；已有状态时仍抽取并累加，但延长本身保持 Tried-only。所有合法分支支付既有 100 energy；错误目标继续由既有物品计划器在消费、时间和 RNG 前拒绝。

状态的 Wisdom/Charisma 加值复用已有 `StatusDto.grantedModifiers`。核心有效属性计算补上对现有状态属性修正的消费位置；没有新增第二套属性管线、状态 DTO、存档字段或 state-hash 输入结构。

## 2. 协议、界面与内容边界

内部 `ItemPoeticInspirationResolved` 只投影普通 `GameEventDto`：首次新增为 `item.use-poetic-inspiration-applied` / `item-use-poetic-inspiration-applied`，重复延长为 `item.use-poetic-inspiration-no-new-effect` / `item-use-poetic-inspiration-no-new-effect`。两者都携带来源、显示键与本次骰出的持续时间；Web 只增加对应展示分支和中英消息。

内容效果只能用于静态 consumable，不能用于动态 activation、充能物品或设备检定。本轮不增加通用 item `apply-status`、药水 resolver、职业覆盖表、药水倍率框架或全局计时上限。

## 3. 导入与契约

legacy importer 将 tval 75 / sval 14 映射为 `apply-poetic-inspiration`。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 严格导入成功，`consumable-effect` 从 72 降至 71，`scroll-effect` 保持 15；真实包保持 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races，源码校验、编译与二进制回读 hash 均为 `53fd88e36019c7c40f177a00cc16a9bc019c51e3f31cb8c9b5b7036417a8fa89`。

fixture 445 连续使用两瓶 Muse Tonic，固定两次持续时间为 179、181 ticks，行动结算后最终剩余 340 ticks；同时固定首次 Applied/Aware、第二次 Extended/Tried-only、Wisdom 10→15、Charisma 23→28、两次消费、20 world ticks、两次效果 RNG 和事件顺序。导入器既有表测试增加 sval 14；没有增加 save round-trip、过期、错误目标、Schema 负例、Web 单测或 Tauri E2E。

内容 hash 属于 state hash 输入，因此既有 444 条 fixture 只替换所有 `stateHash` / `saveRoundTripStateHash` 字段；递归去除这些字段后的完整 assertions 与 contract-v140 逐文件一致。

## 4. 明确遗留

- 原版 `_potion_power` 与 Potion Devicemaster 特例等待对应职业纵切，不建立通用药水倍率框架；
- 原版状态计时的全局 10000-tick 上限等待统一时间状态边界，不在该 resolver 内加专用截断；
- Poetry 的额外职业、法术或 UI 表现等待各自纵切，不把无消费者字段塞入当前状态；
- 其他 71 个 `consumable-effect` 与 15 个 `scroll-effect` 继续按独立事务分组。
