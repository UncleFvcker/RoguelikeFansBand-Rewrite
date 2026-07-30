# Contract v143：Restore Life Levels 药水

状态：已实现（P93）

Contract v143 接入原版药水 tval 75 / sval 41 的 Restore Life Levels 行为。协议保持 `1.121`，demo 内容包为 `1.134.0`，save 容器保持 v1，state hash Schema 保持 `54`。active baseline 包含 447 条 exact fixtures、零 waiver，内置内容 hash 为 `8b3bdb097563d99b6433a5746c07d395b406d5c8d86616540e0126cd6af72404`。

## 1. 效果与事务

固定原版 `191f48c3` 的 `src/devices.c` 先调用 `restore_level()`，再调用 `lp_player(150)`；前者把当前经验恢复到历史最高经验并重算等级，后者增加 150 生命力且封顶 1000。两项返回值以“或”组合，任一实际变化即为可察觉，整个效果不抽 RNG。

当前纵切增加 self-only 的窄静态消耗品效果 `restore-life-levels { lifeForceAmount }`。合法使用先写入 Tried 并消费药水，再把当前经验恢复到既有 `maximumExperience`，调用现有等级重算入口，最后以饱和加法恢复生命力并封顶 1000。经验或生命力任一变化才把来源药水升级为 Aware；两项都已满时保持 Tried-only。所有合法分支支付既有 100 energy；错误目标继续由既有物品计划器在消费、时间和 RNG 前拒绝。

## 2. 协议、界面与内容边界

历史最高经验、生命力、等级、HP、技能与资源上限均复用既有权威结构，本轮没有新增协议 DTO、命令、存档字段或 state-hash Schema。内部 `ItemRestoreLifeLevelsResolved` 只投影普通 `GameEventDto`：实际变化为 `item.use-restore-life-levels` / `item-use-restore-life-levels`，完全无变化为 `item.use-restore-life-levels-no-effect` / `item-use-restore-life-levels-no-effect`；事件只携带来源与显示键，不复制成长状态。Web 只增加对应展示分支和中英消息。

效果只能用于静态 consumable，不能用于动态 activation、充能物品或设备检定。demo 使用原创 Renewal Tonic；本轮不修改 Death `RestoreVitality`，不增加通用成长事务、经验吸取、生命力损伤、职业覆盖表或设备效果入口。

## 3. 导入与契约

legacy importer 将 tval 75 / sval 41 映射为 `restore-life-levels { lifeForceAmount: 150 }`。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 严格导入成功，解析诊断为零，`consumable-effect` 从 69 降至 68，`scroll-effect` 保持 15；真实包保持 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races，源码校验、编译与二进制回读 hash 均为 `c7d1868b4ed9452c9159b6870af80eb942bfca3350f76d42c2b540a90b710ed1`。

fixture 447 让 1 级 Vanguard 在经验 5、历史最高经验 25、生命力 900 时使用一瓶 Renewal Tonic，固定当前经验恢复到 25、等级升至 3、生命力封顶 1000、来源变为 Aware、药水消费、10 world ticks、零效果 RNG，以及两个升级事件先于物品恢复事件。一个表驱动核心测试分别覆盖仅经验变化、仅生命力变化和两项均无变化，固定 OR 识别语义和 Tried-only 边界；没有增加 save round-trip、Schema 负例、Web 单测或 Tauri E2E 断言。

旧正式 demo hash `48611b108dafc4b06836073ca6b5c6881779c653cbab569a7fdeaec82c1c707a` 已加入兼容列表。内容 hash 属于 state hash 输入，因此既有 446 条 fixture 只替换 `stateHash` / `saveRoundTripStateHash` 字段；去除这些字段后的完整 assertions 与 contract-v142 逐文件一致。

## 4. 明确遗留

- 原版 Possessor/Mimic 的形态经验上限等待对应形态系统，不在本效果中加入种族分支；
- Android 的经验模型、经验吸取来源和生命力损伤来源仍未建立，本轮只消费既有权威字段；
- 原版设备表中的 Restore Experience activation 等待设备效果纵切，不提前放宽静态 consumable 限制；
- 其他 68 个 `consumable-effect` 与 15 个 `scroll-effect` 继续按独立事务分组。
